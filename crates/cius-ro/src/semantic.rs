//! Fixed-point base-10 CEN EN 16931 semantic model evaluation (`en16931` + `en16931-formats`)
//! and OASIS Schematron Validation Report Language (`SVRL`) XML serializer.

#[cfg(feature = "svrl")]
use std::borrow::Cow;
use std::collections::HashSet;

use en16931::{Severity as EnSeverity, profiles};
use en16931_formats::{Policy, report::Report};

use crate::stage_acc::{StageAcc, StageFinding, tr, tr_fmt};
use crate::types::{
    CIUS_RO_CUSTOMIZATION_ID, DocumentIdentity, EngineId, EngineReport, Locale, Severity,
    ValidateOptions, Violation,
};

pub(crate) const ENGINE_ID: EngineId = EngineId::En16931Model;
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

    let mut acc = StageAcc::new(ENGINE_ID, ENGINE_NAME, ENGINE_VERSION, opts.locale);
    collect_en_report(&received.report_declared, &mut acc, None);
    if let Some(req_rep) = &received.report_required {
        let mut seen: HashSet<(String, Option<String>)> = acc
            .violations()
            .iter()
            .map(|v| (v.code.clone(), v.location.clone()))
            .collect();
        collect_en_report(req_rep, &mut acc, Some(&mut seen));
    }

    if let Some((code, message, actual)) = check_bt24_declaration(
        received.declared.as_ref(),
        is_cius_ro,
        opts.run_cius_ro_rules,
        acc.locale(),
    ) {
        acc.record(
            StageFinding::fatal(code, message)
                .term("BT-24")
                .location("BT-24")
                .expected(CIUS_RO_CUSTOMIZATION_ID)
                .maybe_actual(actual),
        );
    }

    let (report, violations) = acc.into_report();
    En16931Outcome {
        report,
        identity,
        invoice: received.invoice,
        violations,
    }
}

fn check_bt24_declaration(
    declared: Option<&profiles::Declaration>,
    is_cius_ro: bool,
    run_cius_ro_rules: bool,
    locale: Locale,
) -> Option<(&'static str, String, Option<String>)> {
    match declared {
        Some(profiles::Declaration::Unknown(opt_id)) if !is_cius_ro && !run_cius_ro_rules => {
            let msg = opt_id.as_deref().map_or_else(
                || {
                    tr(
                        locale,
                        "Specification identifier (BT-24) is missing.",
                        "Identificatorul specificației (BT-24) lipsește.",
                    )
                    .to_owned()
                },
                |v| {
                    tr_fmt!(
                        locale,
                        "Specification identifier (BT-24) '{v}' is not a recognized EN 16931 / CIUS-RO URI.",
                        "Identificatorul specificației (BT-24) '{v}' nu este un profil EN 16931 / CIUS-RO recunoscut."
                    )
                },
            );
            Some(("EN16931-BT24-PROFILE", msg, opt_id.clone()))
        }
        Some(profiles::Declaration::WrongProcess(v)) => {
            let msg = tr_fmt!(
                locale,
                "Specification identifier (BT-24) '{v}' belongs to a non-invoice process.",
                "Identificatorul specificației (BT-24) '{v}' aparține unui alt proces decât facturarea."
            );
            Some(("EN16931-BT24-PROCESS", msg, Some(v.clone())))
        }
        _ => None,
    }
}

fn collect_en_report(
    report: &Report,
    acc: &mut StageAcc,
    mut seen: Option<&mut HashSet<(String, Option<String>)>>,
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
        if let Some(s) = seen.as_deref_mut()
            && !s.insert((code.clone(), Some(entry.path.clone())))
        {
            continue;
        }
        let mut finding = StageFinding::new(sev, &code, entry.message.clone())
            .location(entry.path.clone())
            .maybe_actual(entry.value.clone())
            .maybe_hint(entry.hint.clone());
        if let Some(term) = entry.term.as_deref() {
            finding = finding.term(term);
        }
        acc.record(finding);
    }

    if let Some(sem_rep) = &report.semantic.report {
        if seen.is_none() {
            acc.add_evaluated(sem_rep.rules_checked());
        }
        for finding in sem_rep.findings() {
            let loc = finding.path.to_string();
            if let Some(s) = seen.as_deref_mut()
                && !s.insert((finding.rule.clone(), Some(loc.clone())))
            {
                continue;
            }
            let sev = match finding.severity {
                EnSeverity::Fatal => Severity::Fatal,
                EnSeverity::Warning => Severity::Warning,
                EnSeverity::Info => Severity::Info,
            };
            let expected = finding.detail.as_ref().map(|d| d.expected.as_str());
            let actual = finding.detail.as_ref().map(|d| d.actual.clone());
            acc.record_uncounted(
                StageFinding::new(sev, &finding.rule, finding.message.clone())
                    .term(&loc)
                    .location(loc.clone())
                    .maybe_expected(expected)
                    .maybe_actual(actual)
                    .maybe_hint(finding.hint.clone()),
            );
        }
    }
}

/// Serializes all validation findings (`EN 16931`, `CIUS-RO`, `XSD`, `PDF/A-3`, and `cius-ro-id` checksums)
/// into a well-formed OASIS Schematron Validation Report Language (`SVRL`) XML document via `en16931::svrl`.
#[cfg(feature = "svrl")]
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
            source: Cow::Borrowed(v.engine.as_str()),
            location: v.location.as_deref().unwrap_or("/").to_owned(),
            text: format!("[{}]-{}", v.code, v.message),
            expected: v.expected.clone(),
            actual: v.actual.clone(),
            hint: v.hint.clone(),
        })
        .collect();
    en16931::svrl::to_svrl(&rep)
}
