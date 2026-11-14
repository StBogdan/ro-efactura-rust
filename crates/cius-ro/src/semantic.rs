//! Fixed-point base-10 CEN EN 16931 semantic model evaluation (`en16931` + `en16931-formats`)
//! and OASIS Schematron Validation Report Language (`SVRL`) XML serializer.

use std::borrow::Cow;

use en16931::{profiles, Severity as EnSeverity};
use en16931_formats::{report::Report, Policy};

use crate::types::{
    DocumentIdentity, EngineReport, EngineStatus, Locale, Severity, ValidateOptions, Violation,
    CIUS_RO_CUSTOMIZATION_ID,
};

pub(crate) const ENGINE_ID: &str = "en16931_model";
pub(crate) const ENGINE_NAME: &str = "CEN EN 16931 Fixed-Point Semantic Model";
pub(crate) const ENGINE_VERSION: &str = "EN 16931-1:2017+A1:2019 (v1.3.17)";

pub(crate) struct En16931Outcome {
    pub report: EngineReport,
    pub identity: Option<DocumentIdentity>,
    pub invoice: Option<en16931::Invoice>,
    pub violations: Vec<Violation>,
}

pub(crate) fn evaluate_en16931(
    xml_bytes: &[u8],
    customization_id: Option<&str>,
    opts: &ValidateOptions,
) -> En16931Outcome {
    if !opts.run_en16931 {
        return En16931Outcome {
            report: EngineReport::skipped(
                ENGINE_ID,
                ENGINE_NAME,
                ENGINE_VERSION,
                "Disabled by ValidateOptions",
            ),
            identity: None,
            invoice: None,
            violations: Vec::new(),
        };
    }

    let is_cius_ro =
        customization_id.is_some_and(|s| s.trim().eq_ignore_ascii_case(CIUS_RO_CUSTOMIZATION_ID));

    let mut policy = Policy::default();
    if !is_cius_ro {
        policy.require = Some(&profiles::EN16931);
    }

    let received = en16931_formats::receive(xml_bytes, &policy);
    let id = &received.report_declared.identity;
    let identity =
        (id.seller.is_some() || id.number.is_some() || id.issue_date.is_some()).then(|| {
            DocumentIdentity {
                invoice_number: id.number.clone(),
                issue_date: id.issue_date.clone(),
                seller_name: id.seller.clone(),
            }
        });

    let mut violations = Vec::new();
    let (mut rules_evaluated, mut failed_count, mut warning_count) = (0usize, 0usize, 0usize);

    collect_en_report(
        &received.report_declared,
        &mut violations,
        &mut rules_evaluated,
        &mut failed_count,
        &mut warning_count,
        false,
    );
    if let Some(req_rep) = &received.report_required {
        collect_en_report(
            req_rep,
            &mut violations,
            &mut rules_evaluated,
            &mut failed_count,
            &mut warning_count,
            true,
        );
    }

    let bt24_err = match &received.declared {
        Some(profiles::Declaration::Unknown(opt_id)) if !is_cius_ro && !opts.run_cius_ro_rules => {
            let msg = match (opt_id, opts.locale) {
                (Some(v), Locale::En) => format!(
                    "Specification identifier (BT-24) '{v}' is not a recognized EN 16931 / CIUS-RO URI."
                ),
                (Some(v), Locale::Ro) => format!(
                    "Identificatorul specificației (BT-24) '{v}' nu este un profil EN 16931 / CIUS-RO recunoscut."
                ),
                (None, Locale::En) => "Specification identifier (BT-24) is missing.".to_string(),
                (None, Locale::Ro) => "Identificatorul specificației (BT-24) lipsește.".to_string(),
            };
            Some(("EN16931-BT24-PROFILE", msg, opt_id.clone()))
        }
        Some(profiles::Declaration::WrongProcess(v)) => {
            let msg = match opts.locale {
                Locale::En => {
                    format!("Specification identifier (BT-24) '{v}' belongs to a non-invoice process.")
                }
                Locale::Ro => format!(
                    "Identificatorul specificației (BT-24) '{v}' aparține unui alt proces decât facturarea."
                ),
            };
            Some(("EN16931-BT24-PROCESS", msg, Some(v.clone())))
        }
        _ => None,
    };

    if let Some((code, message, actual)) = bt24_err {
        rules_evaluated += 1;
        failed_count += 1;
        violations.push(Violation {
            code: code.to_string(),
            rule_sub_id: None,
            engine: ENGINE_ID.to_string(),
            severity: Severity::Fatal,
            business_term: Some("BT-24".to_string()),
            message,
            location: Some("BT-24".to_string()),
            expected: Some(CIUS_RO_CUSTOMIZATION_ID.to_string()),
            actual,
            hint: None,
        });
    }

    let eval = rules_evaluated.max(1);
    En16931Outcome {
        report: EngineReport {
            id: ENGINE_ID.to_string(),
            name: ENGINE_NAME.to_string(),
            version: ENGINE_VERSION.to_string(),
            status: if failed_count == 0 {
                EngineStatus::Passed
            } else {
                EngineStatus::Failed
            },
            rules_evaluated: eval,
            passed_count: eval.saturating_sub(failed_count),
            failed_count,
            warning_count,
            skip_reason: None,
        },
        identity,
        invoice: received.invoice,
        violations,
    }
}

fn collect_en_report(
    report: &Report,
    violations: &mut Vec<Violation>,
    rules_evaluated: &mut usize,
    failed_count: &mut usize,
    warning_count: &mut usize,
    dedup: bool,
) {
    for entry in report.entries() {
        let sev = match entry.level() {
            EnSeverity::Fatal => Severity::Fatal,
            EnSeverity::Warning => Severity::Warning,
            EnSeverity::Info => continue,
        };
        let code = entry
            .rule
            .clone()
            .unwrap_or_else(|| format!("EN16931-{}", entry.kind.as_str().to_ascii_uppercase()));
        if dedup
            && violations
                .iter()
                .any(|v| v.code == code && v.location.as_deref() == Some(entry.path.as_str()))
        {
            continue;
        }
        *rules_evaluated += 1;
        if sev == Severity::Fatal {
            *failed_count += 1;
        } else {
            *warning_count += 1;
        }
        violations.push(Violation {
            code,
            rule_sub_id: None,
            engine: ENGINE_ID.to_string(),
            severity: sev,
            business_term: entry.term.clone(),
            message: entry.message.clone(),
            location: Some(entry.path.clone()),
            expected: None,
            actual: entry.value.clone(),
            hint: entry.hint.clone(),
        });
    }

    if let Some(sem_rep) = &report.semantic.report {
        if !dedup {
            *rules_evaluated += sem_rep.rules_checked();
        }
        for finding in sem_rep.findings() {
            let loc = finding.path.to_string();
            if dedup
                && violations
                    .iter()
                    .any(|v| v.code == finding.rule && v.location.as_deref() == Some(loc.as_str()))
            {
                continue;
            }
            let sev = match finding.severity {
                EnSeverity::Fatal => {
                    *failed_count += 1;
                    Severity::Fatal
                }
                EnSeverity::Warning => {
                    *warning_count += 1;
                    Severity::Warning
                }
                EnSeverity::Info => Severity::Info,
            };
            violations.push(Violation {
                code: finding.rule.clone(),
                rule_sub_id: None,
                engine: ENGINE_ID.to_string(),
                severity: sev,
                business_term: Some(loc.clone()),
                message: finding.message.clone(),
                location: Some(loc),
                expected: finding.detail.as_ref().map(|d| d.expected.clone()),
                actual: finding.detail.as_ref().map(|d| d.actual.clone()),
                hint: finding.hint.clone(),
            });
        }
    }
}

/// Serializes all validation findings (`EN 16931`, `CIUS-RO`, `XSD`, `PDF/A-3`, and `cius-ro-id` checksums)
/// into a well-formed OASIS Schematron Validation Report Language (`SVRL`) XML document via `en16931::svrl`.
#[must_use]
pub fn render_svrl_report(violations: &[Violation]) -> String {
    let mut rep = en16931::report::Report::unchecked();
    rep.valid = violations.iter().all(|v| v.severity != Severity::Fatal);
    rep.profile = Some("ANAF RO e-Factura CIUS-RO 1.0.1".to_owned());
    rep.findings = violations
        .iter()
        .map(|v| en16931::report::Entry {
            rule: v.rule_sub_id.as_deref().unwrap_or(&v.code).to_owned(),
            severity: Cow::Borrowed(match v.severity {
                Severity::Fatal => "fatal",
                Severity::Warning => "warning",
                Severity::Info => "information",
            }),
            source: Cow::Owned(v.engine.clone()),
            location: v.location.as_deref().unwrap_or("/").to_owned(),
            text: format!("[{}]-{}", v.code, v.message),
            expected: v.expected.clone(),
            actual: v.actual.clone(),
            hint: v.hint.clone(),
        })
        .collect();
    en16931::svrl::to_svrl(&rep)
}
