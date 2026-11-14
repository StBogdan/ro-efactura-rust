//! Command-line validator, PDF/A-3 extractor, and Romanian identifier inspector for
//! ANAF `RO e-Factura` (`CIUS-RO 1.0.1`).

use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};
use serde::Serialize;

use cius_ro::{
    Locale, RULES, RuleView, Severity, ValidateOptions, ValidationReport, extract_xml_from_pdf,
    find_rule, render_rules_markdown, render_svrl_report, validate_bytes,
};
use cius_ro_id::inspect_identifier;

/// Maximum input payload accepted from a file or `stdin` (`128 MiB`).
const MAX_CLI_INPUT_BYTES: u64 = 128 * 1024 * 1024;

#[derive(Parser, Debug)]
#[command(
    name = "cius-ro",
    version,
    about = "ANAF RO e-Factura (CIUS-RO 1.0.1) validator, PDF/A-3 extractor & Romanian fiscal ID inspector",
    long_about = "Validate standalone UBL 2.1 / UN-CEFACT CII D16B XML invoices and hybrid PDF/A-3 (efactura.xml) documents against W3C XSD 1.1, CEN EN 16931 fixed-point math, all 122 ANAF CIUS-RO 1.0.1 (BR-RO-*) assertions, and Romanian CUI / CNP / ONRC / IBAN checksums."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Validate one or more XML or hybrid PDF/A-3 invoice files (use `-` for stdin)
    Check {
        /// Paths to XML or PDF files, or `-` to read from standard input
        #[arg(required = true)]
        paths: Vec<PathBuf>,

        /// Diagnostic output language (`en` or `ro`)
        #[arg(short, long, default_value_t = Locale::En)]
        lang: Locale,

        /// Output format (`human`, `json`, `svrl`, or `sarif`)
        #[arg(short, long, value_enum, default_value_t = OutputFormat::Human)]
        format: OutputFormat,

        /// Skip W3C XSD 1.1 schema validation
        #[arg(long)]
        no_xsd: bool,

        /// Skip ISO 19005-3 PDF/A-3 structural conformance checks (still extracts XML from PDFs)
        #[arg(long)]
        no_pdfa: bool,

        /// Treat warnings (such as CUI / CNP / ONRC / IBAN checksum warnings) as validation failures
        #[arg(long)]
        deny_warnings: bool,
    },

    /// Extract embedded `efactura.xml` / `factur-x.xml` from a hybrid PDF/A-3 invoice
    Extract {
        /// Input PDF file path (or `-` for stdin)
        pdf: PathBuf,

        /// Optional output XML file path (writes to stdout if omitted)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Inspect and verify a Romanian fiscal or administrative identifier (CUI, CNP, ONRC, IBAN, County, Sector)
    Id {
        /// Identifier value to inspect (e.g. `RO45678918`, `J40/1234/2022`, `RO-CJ`, `SECTOR1`)
        value: String,

        /// Diagnostic output language (`en` or `ro`)
        #[arg(short, long, default_value_t = Locale::En)]
        lang: Locale,

        /// Emit machine-readable JSON
        #[arg(long)]
        json: bool,
    },

    /// Browse or look up `CIUS-RO 1.0.1` (`BR-RO-*`) and `cius-ro-id` validation rules
    Rules {
        /// Optional rule code or sub-ID to inspect (e.g. `BR-RO-110`, `BR-DEC-RO-05`, `BR-RO-CUI`)
        code: Option<String>,

        /// Documentation language (`en` or `ro`)
        #[arg(short, long, default_value_t = Locale::En)]
        lang: Locale,

        /// Render the complete Markdown reference table
        #[arg(long)]
        markdown: bool,

        /// Emit machine-readable JSON
        #[arg(long)]
        json: bool,
    },
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum OutputFormat {
    Human,
    Json,
    Svrl,
    Sarif,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MultiFileReport {
    path: String,
    #[serde(flatten)]
    report: ValidationReport,
}

fn main() -> ExitCode {
    match Cli::parse().command {
        Commands::Check {
            paths,
            lang,
            format,
            no_xsd,
            no_pdfa,
            deny_warnings,
        } => cmd_check(&paths, lang, format, !no_xsd, !no_pdfa, deny_warnings),
        Commands::Extract { pdf, output } => cmd_extract(&pdf, output.as_deref()),
        Commands::Id { value, lang, json } => cmd_id(&value, lang, json),
        Commands::Rules {
            code,
            lang,
            markdown,
            json,
        } => cmd_rules(code.as_deref(), lang, markdown, json),
    }
}

fn read_bounded<R: Read>(reader: R) -> io::Result<Vec<u8>> {
    let mut buf = Vec::new();
    reader.take(MAX_CLI_INPUT_BYTES + 1).read_to_end(&mut buf)?;
    if u64::try_from(buf.len()).is_ok_and(|len| len > MAX_CLI_INPUT_BYTES) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "input exceeds 128 MiB maximum size limit",
        ));
    }
    Ok(buf)
}

fn read_input(path: &Path) -> io::Result<Vec<u8>> {
    if path == Path::new("-") {
        return read_bounded(io::stdin().lock());
    }
    let meta = fs::metadata(path)?;
    if meta.is_file() && meta.len() > MAX_CLI_INPUT_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "file exceeds 128 MiB maximum size limit",
        ));
    }
    read_bounded(File::open(path)?)
}

fn print_json<T: Serialize>(value: &T) -> Result<(), ExitCode> {
    match serde_json::to_string_pretty(value) {
        Ok(s) => {
            println!("{s}");
            Ok(())
        }
        Err(err) => {
            eprintln!("cius-ro: JSON serialization error: {err}");
            Err(ExitCode::from(2))
        }
    }
}

fn exit_status(ok: bool) -> ExitCode {
    if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn cmd_check(
    paths: &[PathBuf],
    locale: Locale,
    format: OutputFormat,
    run_xsd: bool,
    run_pdfa: bool,
    deny_warnings: bool,
) -> ExitCode {
    let opts = ValidateOptions::with_locale(locale)
        .xsd(run_xsd)
        .pdfa(run_pdfa)
        .deny_warnings(deny_warnings)
        .include_svrl(format == OutputFormat::Svrl);

    let mut results = Vec::with_capacity(paths.len());
    for p in paths {
        let display = p.display().to_string();
        let bytes = match read_input(p) {
            Ok(b) => b,
            Err(err) => {
                eprintln!("cius-ro: failed to read '{display}': {err}");
                return ExitCode::from(2);
            }
        };
        results.push((display, validate_bytes(&bytes, &opts)));
    }

    let all_valid = results.iter().all(|(_, r)| r.is_valid);

    match format {
        OutputFormat::Human => {
            for (idx, (path, rep)) in results.iter().enumerate() {
                if idx > 0 {
                    println!();
                }
                print_human_report(path, rep, locale);
            }
        }
        OutputFormat::Json => {
            let res = if results.len() == 1 {
                print_json(&results[0].1)
            } else {
                let multi: Vec<MultiFileReport> = results
                    .into_iter()
                    .map(|(path, report)| MultiFileReport { path, report })
                    .collect();
                print_json(&multi)
            };
            if let Err(code) = res {
                return code;
            }
        }
        OutputFormat::Svrl => {
            for (_, rep) in &results {
                if let Some(svrl) = &rep.svrl_xml {
                    print!("{svrl}");
                } else {
                    print!("{}", render_svrl_report(&rep.violations));
                }
            }
        }
        OutputFormat::Sarif => {
            if let Err(code) = print_json(&build_sarif(&results, locale)) {
                return code;
            }
        }
    }

    exit_status(all_valid)
}

fn print_human_report(path: &str, rep: &ValidationReport, locale: Locale) {
    let badge = match (rep.is_valid, locale) {
        (true, Locale::En) => "PASS",
        (true, Locale::Ro) => "VALID",
        (false, Locale::En) => "FAIL",
        (false, Locale::Ro) => "RESPINS",
    };
    let syntax = rep.detected_syntax.map_or("-", cius_ro::XmlSyntax::label);
    let inv_num = rep
        .identity
        .as_ref()
        .and_then(|i| i.invoice_number.as_deref())
        .unwrap_or("-");

    println!(
        "[{badge}] {path} (container={}, syntax={syntax}, id={inv_num})",
        rep.container
    );

    for eng in &rep.engines {
        println!(
            "  - {}: {} (evaluated={}, failed={}, warnings={})",
            eng.id, eng.status, eng.rules_evaluated, eng.failed_count, eng.warning_count
        );
    }

    for v in &rep.violations {
        let sev = match v.severity {
            Severity::Fatal => "FATAL",
            Severity::Warning => "WARN",
            Severity::Info => "INFO",
        };
        println!(
            "  [{sev}] {} @ {}: {}",
            v.code,
            v.location.as_deref().unwrap_or("/"),
            v.message
        );
        if let Some(hint) = &v.hint {
            let lbl = if locale.is_ro() { "Sugestie" } else { "Hint" };
            println!("         {lbl}: {hint}");
        }
    }
}

fn build_sarif(results: &[(String, ValidationReport)], locale: Locale) -> serde_json::Value {
    let mut seen_rule_ids = BTreeSet::new();
    let mut sarif_rules = Vec::new();
    for (_, rep) in results {
        for v in &rep.violations {
            if seen_rule_ids.insert(v.code.clone())
                && let Some(desc) = find_rule(&v.code)
            {
                sarif_rules.push(serde_json::json!({
                    "id": v.code,
                    "shortDescription": { "text": desc.title(locale) },
                    "fullDescription": { "text": desc.description(locale) },
                    "help": { "text": desc.remediation(locale) },
                }));
            }
        }
    }

    let sarif_results: Vec<_> = results
        .iter()
        .flat_map(|(path, rep)| {
            rep.violations.iter().map(move |v| {
                let level = match v.severity {
                    Severity::Fatal => "error",
                    Severity::Warning => "warning",
                    Severity::Info => "note",
                };
                serde_json::json!({
                    "ruleId": v.code,
                    "level": level,
                    "message": { "text": v.message },
                    "locations": [{
                        "physicalLocation": { "artifactLocation": { "uri": path } },
                        "logicalLocations": v.location.as_ref().map(|loc| vec![serde_json::json!({
                            "fullyQualifiedName": loc
                        })]).unwrap_or_default()
                    }]
                })
            })
        })
        .collect();

    serde_json::json!({
        "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
        "version": "2.1.0",
        "runs": [{
            "tool": {
                "driver": {
                    "name": "cius-ro",
                    "version": env!("CARGO_PKG_VERSION"),
                    "informationUri": "https://github.com/StBogdan/ro-efactura-rust",
                    "rules": sarif_rules
                }
            },
            "results": sarif_results
        }]
    })
}

fn cmd_extract(pdf: &Path, output: Option<&Path>) -> ExitCode {
    let bytes = match read_input(pdf) {
        Ok(b) => b,
        Err(err) => {
            eprintln!("cius-ro: failed to read '{}': {err}", pdf.display());
            return ExitCode::from(2);
        }
    };
    let extracted = match extract_xml_from_pdf(&bytes) {
        Ok(ext) => ext,
        Err(err) => {
            eprintln!("cius-ro: extraction failed: {err}");
            return ExitCode::from(1);
        }
    };

    if let Some(out_path) = output {
        if let Err(err) = fs::write(out_path, &extracted.xml_bytes) {
            eprintln!("cius-ro: failed to write '{}': {err}", out_path.display());
            return ExitCode::from(2);
        }
        eprintln!(
            "Extracted '{}' ({} bytes) -> {}",
            extracted.filename,
            extracted.xml_bytes.len(),
            out_path.display()
        );
    } else if let Err(err) = io::stdout().lock().write_all(&extracted.xml_bytes) {
        eprintln!("cius-ro: stdout write error: {err}");
        return ExitCode::from(2);
    }
    ExitCode::SUCCESS
}

fn cmd_id(value: &str, locale: Locale, json: bool) -> ExitCode {
    let res = inspect_identifier(value, locale);
    if json {
        if let Err(code) = print_json(&res) {
            return code;
        }
    } else {
        let status = if res.is_valid { "VALID" } else { "INVALID" };
        println!("[{status}] kind={} input={}", res.kind, res.input);
        if let Some(norm) = &res.normalized {
            println!("  normalized: {norm}");
        }
        println!("  {}", res.details);
    }
    exit_status(res.is_valid)
}

fn cmd_rules(code: Option<&str>, locale: Locale, markdown: bool, json: bool) -> ExitCode {
    if let Some(q) = code {
        let Some(rule) = find_rule(q) else {
            if locale.is_ro() {
                eprintln!("Cod regulă CIUS-RO necunoscut: '{q}'");
            } else {
                eprintln!("Unknown CIUS-RO rule code: '{q}'");
            }
            return ExitCode::from(1);
        };
        if json {
            if let Err(code) = print_json(&rule.view(locale)) {
                return code;
            }
        } else {
            println!(
                "{} ({}, category={}) [{}]",
                rule.code, rule.severity, rule.category, rule.business_terms
            );
            println!("  {}", rule.title(locale));
            println!("  {}", rule.description(locale));
            let rem_lbl = if locale.is_ro() {
                "Remediere"
            } else {
                "Remediation"
            };
            println!("  {rem_lbl}: {}", rule.remediation(locale));
        }
        return ExitCode::SUCCESS;
    }

    if json {
        let views: Vec<RuleView<'static>> = RULES.iter().map(|r| r.view(locale)).collect();
        return print_json(&views).map_or_else(|c| c, |()| ExitCode::SUCCESS);
    }

    if markdown {
        print!("{}", render_rules_markdown(locale));
        return ExitCode::SUCCESS;
    }

    for rule in RULES {
        println!(
            "{:<14} {:<8} {:<24} {}",
            rule.code,
            rule.severity.as_str(),
            rule.business_terms,
            rule.title(locale)
        );
    }
    ExitCode::SUCCESS
}
