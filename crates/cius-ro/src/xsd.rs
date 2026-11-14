//! Official W3C XML Schema (`XSD 1.1`) Grammar & XXE Validator (`uppsala`).
//!
//! Embeds OASIS UBL 2.1 (`Invoice`, `CreditNote`, 14 common schemas) and UN/CEFACT CII D16B
//! (`100pD16B`, 4 schemas) inside the crate binary via `include_str!`.
//!
//! - On native targets, the 18 embedded schema modules are materialized into an ephemeral
//!   temporary directory once during [`OnceLock`](std::sync::OnceLock) bootstrap (required by
//!   `uppsala` to resolve relative `<xs:import schemaLocation="..."/>` directives), compiled into
//!   reusable [`uppsala::XsdValidator`] state machines, and the temporary directory is deleted
//!   immediately before [`warm_xsd_validators`] returns.
//! - On `wasm32-unknown-unknown`, `uppsala::Parser` enforces XML 1.0 well-formedness, maximum
//!   element nesting depth (`64`), and strict `DOCTYPE`/`ENTITY` (`XXE`) prohibition in memory.

use core::fmt;

use uppsala::Parser;

use crate::stage_acc::{StageAcc, StageFinding};
use crate::types::{
    EngineId, EngineReport, Locale, ValidateOptions, Violation, XmlDecodeError, XmlSyntax,
};

pub(crate) const ENGINE_ID: EngineId = EngineId::XsdSchema;
pub(crate) const ENGINE_NAME: &str = "Official W3C XSD 1.1 Grammar Validator";
const DEFAULT_VERSION: &str = "W3C XSD 1.1";
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
const MAX_REPORTED_XSD_ERRORS: usize = 100;

/// Error returned if compiling the embedded UBL 2.1 / CII D16B XSD schemas fails at bootstrap.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum XsdInitError {
    /// Creating or writing to the temporary XSD bootstrap directory failed.
    BootstrapIo(String),
    /// Parsing an embedded XSD schema XML document failed.
    SchemaParse {
        /// Schema suite label (`"CII D16B"`, `"UBL Invoice 2.1"`, `"UBL CreditNote 2.1"`).
        schema: &'static str,
        /// Underlying parser message.
        detail: String,
    },
    /// Compiling an XSD schema and its `<xs:import>` dependencies failed.
    SchemaCompile {
        /// Schema suite label.
        schema: &'static str,
        /// Underlying compiler message.
        detail: String,
    },
}

impl fmt::Display for XsdInitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BootstrapIo(msg) => write!(f, "XSD bootstrap I/O error: {msg}"),
            Self::SchemaParse { schema, detail } => {
                write!(f, "failed parsing embedded {schema} XSD: {detail}")
            }
            Self::SchemaCompile { schema, detail } => {
                write!(
                    f,
                    "failed compiling embedded {schema} XSD validator: {detail}"
                )
            }
        }
    }
}

impl std::error::Error for XsdInitError {}

pub(crate) struct XsdOutcome {
    pub report: EngineReport,
    pub violations: Vec<Violation>,
}

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
mod native_compiled {
    use std::fs;
    use std::path::Path;
    use std::sync::OnceLock;
    use uppsala::XsdValidator;

    use super::XsdInitError;

    const CII_FILES: &[(&str, &str)] = &[
        (
            "CrossIndustryInvoice_100pD16B.xsd",
            include_str!("../schemas/xsd/cii_d16b/CrossIndustryInvoice_100pD16B.xsd"),
        ),
        (
            "CrossIndustryInvoice_QualifiedDataType_100pD16B.xsd",
            include_str!(
                "../schemas/xsd/cii_d16b/CrossIndustryInvoice_QualifiedDataType_100pD16B.xsd"
            ),
        ),
        (
            "CrossIndustryInvoice_ReusableAggregateBusinessInformationEntity_100pD16B.xsd",
            include_str!(
                "../schemas/xsd/cii_d16b/CrossIndustryInvoice_ReusableAggregateBusinessInformationEntity_100pD16B.xsd"
            ),
        ),
        (
            "CrossIndustryInvoice_UnqualifiedDataType_100pD16B.xsd",
            include_str!(
                "../schemas/xsd/cii_d16b/CrossIndustryInvoice_UnqualifiedDataType_100pD16B.xsd"
            ),
        ),
    ];

    const UBL_MAINDOC_INVOICE: &str =
        include_str!("../schemas/xsd/ubl_2_1/maindoc/UBL-Invoice-2.1.xsd");
    const UBL_MAINDOC_CREDITNOTE: &str =
        include_str!("../schemas/xsd/ubl_2_1/maindoc/UBL-CreditNote-2.1.xsd");

    const UBL_COMMON_FILES: &[(&str, &str)] = &[
        (
            "CCTS_CCT_SchemaModule-2.1.xsd",
            include_str!("../schemas/xsd/ubl_2_1/common/CCTS_CCT_SchemaModule-2.1.xsd"),
        ),
        (
            "UBL-CommonAggregateComponents-2.1.xsd",
            include_str!("../schemas/xsd/ubl_2_1/common/UBL-CommonAggregateComponents-2.1.xsd"),
        ),
        (
            "UBL-CommonBasicComponents-2.1.xsd",
            include_str!("../schemas/xsd/ubl_2_1/common/UBL-CommonBasicComponents-2.1.xsd"),
        ),
        (
            "UBL-CommonExtensionComponents-2.1.xsd",
            include_str!("../schemas/xsd/ubl_2_1/common/UBL-CommonExtensionComponents-2.1.xsd"),
        ),
        (
            "UBL-CommonSignatureComponents-2.1.xsd",
            include_str!("../schemas/xsd/ubl_2_1/common/UBL-CommonSignatureComponents-2.1.xsd"),
        ),
        (
            "UBL-CoreComponentParameters-2.1.xsd",
            include_str!("../schemas/xsd/ubl_2_1/common/UBL-CoreComponentParameters-2.1.xsd"),
        ),
        (
            "UBL-ExtensionContentDataType-2.1.xsd",
            include_str!("../schemas/xsd/ubl_2_1/common/UBL-ExtensionContentDataType-2.1.xsd"),
        ),
        (
            "UBL-QualifiedDataTypes-2.1.xsd",
            include_str!("../schemas/xsd/ubl_2_1/common/UBL-QualifiedDataTypes-2.1.xsd"),
        ),
        (
            "UBL-SignatureAggregateComponents-2.1.xsd",
            include_str!("../schemas/xsd/ubl_2_1/common/UBL-SignatureAggregateComponents-2.1.xsd"),
        ),
        (
            "UBL-SignatureBasicComponents-2.1.xsd",
            include_str!("../schemas/xsd/ubl_2_1/common/UBL-SignatureBasicComponents-2.1.xsd"),
        ),
        (
            "UBL-UnqualifiedDataTypes-2.1.xsd",
            include_str!("../schemas/xsd/ubl_2_1/common/UBL-UnqualifiedDataTypes-2.1.xsd"),
        ),
        (
            "UBL-XAdESv132-2.1.xsd",
            include_str!("../schemas/xsd/ubl_2_1/common/UBL-XAdESv132-2.1.xsd"),
        ),
        (
            "UBL-XAdESv141-2.1.xsd",
            include_str!("../schemas/xsd/ubl_2_1/common/UBL-XAdESv141-2.1.xsd"),
        ),
        (
            "UBL-xmldsig-core-schema-2.1.xsd",
            include_str!("../schemas/xsd/ubl_2_1/common/UBL-xmldsig-core-schema-2.1.xsd"),
        ),
    ];

    pub(super) struct CompiledXsdSuites {
        pub cii_d16b: XsdValidator,
        pub ubl_invoice_2_1: XsdValidator,
        pub ubl_credit_note_2_1: XsdValidator,
    }

    static XSD_SUITES: OnceLock<Result<CompiledXsdSuites, XsdInitError>> = OnceLock::new();

    pub(super) fn get_validators() -> Result<&'static CompiledXsdSuites, XsdInitError> {
        XSD_SUITES
            .get_or_init(compile_all_xsd_suites)
            .as_ref()
            .map_err(Clone::clone)
    }

    fn compile_one(
        xsd: &str,
        base: &Path,
        label: &'static str,
    ) -> Result<XsdValidator, XsdInitError> {
        let doc = uppsala::parse(xsd).map_err(|e| XsdInitError::SchemaParse {
            schema: label,
            detail: e.to_string(),
        })?;
        XsdValidator::from_schema_with_base_path(&doc, Some(base)).map_err(|e| {
            XsdInitError::SchemaCompile {
                schema: label,
                detail: e.to_string(),
            }
        })
    }

    fn compile_all_xsd_suites() -> Result<CompiledXsdSuites, XsdInitError> {
        let tmp = tempfile::tempdir().map_err(|e| XsdInitError::BootstrapIo(e.to_string()))?;
        let cii_dir = tmp.path().join("cii");
        let ubl_dir = tmp.path().join("ubl");
        let ubl_common_dir = ubl_dir.join("common");
        fs::create_dir_all(&cii_dir).map_err(|e| XsdInitError::BootstrapIo(e.to_string()))?;
        fs::create_dir_all(&ubl_common_dir)
            .map_err(|e| XsdInitError::BootstrapIo(e.to_string()))?;

        for (name, content) in CII_FILES {
            fs::write(cii_dir.join(name), content)
                .map_err(|e| XsdInitError::BootstrapIo(e.to_string()))?;
        }
        for (name, content) in UBL_COMMON_FILES {
            fs::write(ubl_common_dir.join(name), content)
                .map_err(|e| XsdInitError::BootstrapIo(e.to_string()))?;
        }

        let ubl_inv = UBL_MAINDOC_INVOICE.replace("../common/", "common/");
        let ubl_cn = UBL_MAINDOC_CREDITNOTE.replace("../common/", "common/");

        Ok(CompiledXsdSuites {
            cii_d16b: compile_one(CII_FILES[0].1, &cii_dir, "CII D16B")?,
            ubl_invoice_2_1: compile_one(&ubl_inv, &ubl_dir, "UBL Invoice 2.1")?,
            ubl_credit_note_2_1: compile_one(&ubl_cn, &ubl_dir, "UBL CreditNote 2.1")?,
        })
    }
}

/// Pre-compiles embedded OASIS UBL 2.1 & UN/CEFACT CII D16B XSD schemas so subsequent validations
/// pay zero initialization overhead.
///
/// # Errors
///
/// Returns [`XsdInitError`] if creating the ephemeral schema bootstrap directory or compiling any
/// embedded XSD suite fails.
pub fn warm_xsd_validators() -> Result<(), XsdInitError> {
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    {
        native_compiled::get_validators().map(|_| ())
    }
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    {
        Ok(())
    }
}

pub(crate) fn evaluate_xsd(
    xml_res: Result<&str, &XmlDecodeError>,
    dtd_detected: bool,
    syntax: Option<XmlSyntax>,
    opts: &ValidateOptions,
) -> XsdOutcome {
    if !opts.run_xsd {
        return XsdOutcome {
            report: EngineReport::skipped(
                ENGINE_ID,
                ENGINE_NAME,
                DEFAULT_VERSION,
                "Disabled by ValidateOptions",
            ),
            violations: Vec::new(),
        };
    }

    let mut acc = StageAcc::new(ENGINE_ID, ENGINE_NAME, DEFAULT_VERSION, opts.locale);

    let xml = match xml_res {
        Ok(s) => s,
        Err(enc_err) => {
            let msg = enc_err.message(acc.locale());
            acc.record(
                StageFinding::fatal("XSD-ENCODING-001", msg)
                    .term("XML 1.0 Encoding")
                    .location("Byte offset 0"),
            );
            return finish(acc);
        }
    };

    let parser = Parser::new()
        .with_max_depth(64)
        .with_max_entity_expansion(0)
        .with_forbid_dtd(true)
        .with_forbid_entities(true);
    let doc = match parser.parse(xml) {
        Ok(d) => d,
        Err(err) => {
            if dtd_detected || contains_dtd_or_entity_decl(xml) {
                let msg = acc
                    .tr(
                        "XML DOCTYPE and ENTITY declarations are strictly forbidden in EN 16931 / RO e-Factura documents.",
                        "Declarațiile XML DOCTYPE și ENTITY sunt strict interzise în facturile electronice EN 16931 / RO e-Factura.",
                    )
                    .to_owned();
                let hint = acc
                    .tr(
                        "Remove all <!DOCTYPE ...> and <!ENTITY ...> declarations.",
                        "Eliminați toate declarațiile <!DOCTYPE ...> și <!ENTITY ...> din documentul XML.",
                    )
                    .to_owned();
                acc.record(
                    StageFinding::fatal("XSD-XXE-PROHIBITED", msg)
                        .term("XML 1.0 DTD / XXE Prohibition")
                        .location("XML Prologue")
                        .hint(hint),
                );
                return finish(acc);
            }
            let err_str = err.to_string();
            let msg = match acc.locale() {
                Locale::En => format!("XML document is not well-formed: {err_str}"),
                Locale::Ro => format!("Documentul XML nu este bine format: {err_str}"),
            };
            let hint = acc
                .tr(
                    "Fix XML syntax, closing tags, and namespace declarations.",
                    "Corectați sintaxa XML, etichetele de închidere și declarațiile de namespace.",
                )
                .to_owned();
            acc.record(
                StageFinding::fatal("XSD-WELLFORMEDNESS", msg)
                    .term("W3C XML 1.0")
                    .hint(hint),
            );
            return finish(acc);
        }
    };

    let Some(syntax_kind) = syntax else {
        acc.set_version("UBL 2.1 / CII D16B");
        let msg = acc
            .tr(
                "Root XML element is neither OASIS UBL 2.1 Invoice/CreditNote nor UN/CEFACT CrossIndustryInvoice.",
                "Elementul rădăcină XML nu este nici OASIS UBL 2.1 Invoice/CreditNote, nici UN/CEFACT CrossIndustryInvoice.",
            )
            .to_owned();
        acc.record(
            StageFinding::fatal("XSD-UNKNOWN-ROOT", msg)
                .term("EN 16931 Syntax Binding")
                .location("/*"),
        );
        return finish(acc);
    };

    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    {
        let _ = doc;
        XsdOutcome {
            report: EngineReport::skipped(
                ENGINE_ID,
                ENGINE_NAME,
                syntax_kind.label(),
                "Browser WASM target skips multi-file XSD fs bootstrap (XML well-formedness, XXE defense, and EN 16931 syntax order/duplicate gates passed)",
            ),
            violations: Vec::new(),
        }
    }

    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    {
        evaluate_native_xsd(&doc, syntax_kind, acc)
    }
}

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
fn evaluate_native_xsd(
    doc: &uppsala::Document<'_>,
    syntax_kind: XmlSyntax,
    mut acc: StageAcc,
) -> XsdOutcome {
    let suites = match native_compiled::get_validators() {
        Ok(s) => s,
        Err(err) => {
            acc.set_version(syntax_kind.label());
            acc.record(
                StageFinding::fatal("XSD-ENGINE-INIT", err.to_string()).term("XSD Bootstrap"),
            );
            return finish(acc);
        }
    };

    let (validator, spec_label) = match syntax_kind {
        XmlSyntax::CiiD16B => (&suites.cii_d16b, "UN/CEFACT CII D16B (100pD16B)"),
        XmlSyntax::UblInvoice => (&suites.ubl_invoice_2_1, "OASIS UBL 2.1 Invoice"),
        XmlSyntax::UblCreditNote => (&suites.ubl_credit_note_2_1, "OASIS UBL 2.1 CreditNote"),
    };
    acc.set_version(spec_label);

    let xsd_errors = validator.validate(doc);
    let element_checks = doc
        .descendants(doc.root())
        .into_iter()
        .filter(|&id| doc.element(id).is_some())
        .count()
        .max(1);
    let hint_str = acc.tr(
        "Verify UBL/CII element sequence order, namespace prefixes, and XSD datatype facets.",
        "Verificați ordinea elementelor UBL/CII din schema XSD, prefixele de namespace și tipurile de date.",
    );

    for (idx, err) in xsd_errors.iter().take(MAX_REPORTED_XSD_ERRORS).enumerate() {
        let loc = match (err.line, err.column) {
            (Some(l), Some(c)) => Some(format!("line {l}, column {c}")),
            (Some(l), None) => Some(format!("line {l}")),
            _ => None,
        };
        let code = format!("XSD-SCHEMA-{:03}", idx + 1);
        let mut finding = StageFinding::fatal(&code, err.message.clone())
            .term(spec_label)
            .hint(hint_str);
        if let Some(l) = loc {
            finding = finding.location(l);
        }
        acc.record(finding);
    }

    if xsd_errors.len() > MAX_REPORTED_XSD_ERRORS {
        let omitted = xsd_errors.len() - MAX_REPORTED_XSD_ERRORS;
        acc.add_unreported_failures(omitted);
        let msg = match acc.locale() {
            Locale::En => format!(
                "{omitted} additional XSD schema error(s) omitted (output capped at {MAX_REPORTED_XSD_ERRORS} findings)."
            ),
            Locale::Ro => format!(
                "Încă {omitted} erori de schemă XSD au fost omise (afișarea este limitată la primele {MAX_REPORTED_XSD_ERRORS} erori)."
            ),
        };
        acc.record_uncounted(
            StageFinding::new(crate::types::Severity::Info, "XSD-SCHEMA-TRUNCATED", msg)
                .term(spec_label)
                .actual(xsd_errors.len().to_string()),
        );
    }

    acc.ensure_min_evaluated(element_checks);
    finish(acc)
}

fn finish(acc: StageAcc) -> XsdOutcome {
    let (report, violations) = acc.into_report();
    XsdOutcome { report, violations }
}

/// Scans XML bytes outside comments, CDATA sections, processing instructions, and quoted
/// tag literals for `<!DOCTYPE` or `<!ENTITY` markup declarations without relying on upstream
/// parser `Display` strings or re-running `roxmltree::Document::parse`.
fn contains_dtd_or_entity_decl(xml: &str) -> bool {
    let bytes = xml.as_bytes();
    let mut i = 0;
    let mut in_tag = false;
    let mut quote: Option<u8> = None;
    while i < bytes.len() {
        if let Some(q) = quote {
            if bytes[i] == q {
                quote = None;
            }
            i += 1;
            continue;
        }
        if in_tag {
            match bytes[i] {
                b'"' | b'\'' => quote = Some(bytes[i]),
                b'>' => in_tag = false,
                b'<' => {
                    in_tag = false;
                    continue;
                }
                _ => {}
            }
            i += 1;
            continue;
        }
        if bytes[i] != b'<' {
            i += 1;
            continue;
        }
        let rest = &bytes[i..];
        if rest.starts_with(b"<!--") {
            let Some(end) = rest[4..].windows(3).position(|w| w == b"-->") else {
                return false;
            };
            i += 4 + end + 3;
        } else if rest.starts_with(b"<![CDATA[") {
            let Some(end) = rest[9..].windows(3).position(|w| w == b"]]>") else {
                return false;
            };
            i += 9 + end + 3;
        } else if rest.starts_with(b"<?") {
            let Some(end) = rest[2..].windows(2).position(|w| w == b"?>") else {
                return false;
            };
            i += 2 + end + 2;
        } else if has_decl_keyword(rest, b"<!DOCTYPE") || has_decl_keyword(rest, b"<!ENTITY") {
            return true;
        } else {
            in_tag = true;
            i += 1;
        }
    }
    false
}

fn has_decl_keyword(rest: &[u8], kw: &[u8]) -> bool {
    rest.get(..kw.len())
        .is_some_and(|p| p.eq_ignore_ascii_case(kw))
        && rest
            .get(kw.len())
            .is_none_or(|b| b.is_ascii_whitespace() || matches!(b, b'>' | b'['))
}
