//! WebAssembly (`wasm-bindgen`) bindings for validating Romanian **ANAF `RO e-Factura`**
//! (`CIUS-RO 1.0.1`) electronic invoices and fiscal identifiers.
//!
//! Exposes four JavaScript/TypeScript entry points:
//! - [`validate_invoice`] (`validateInvoice` in JS/TS): validates standalone XML (`UBL 2.1` / `CII D16B`)
//!   and hybrid `PDF/A-3` (`efactura.xml`) byte buffers in browsers, Edge Workers, Deno, or Node.js.
//! - [`extract_pdf_xml`] (`extractPdfXml` in JS/TS): extracts embedded `efactura.xml` / `factur-x.xml`
//!   from a PDF byte slice.
//! - [`validate_identifier`] (`validateIdentifier` in JS/TS): verifies Romanian `CUI`, `CNP`, `ONRC`,
//!   `RO` IBAN, `ISO 3166-2:RO` county codes, and Bucharest `SECTOR1`..`SECTOR6`.
//! - [`list_rules`] (`listRules` in JS/TS): returns the localized `CIUS-RO 1.0.1` rule catalog.

use serde::Serialize;
use serde_wasm_bindgen::Serializer;
use wasm_bindgen::prelude::*;

use cius_ro::{Locale, RULES, RuleView, ValidateOptions, extract_xml_from_pdf, validate_bytes};
use cius_ro_id::inspect_identifier;

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &'static str = include_str!("types.d.ts");

#[wasm_bindgen]
extern "C" {
    /// Typed JavaScript `ValidationReport` object returned by [`validate_invoice`].
    #[wasm_bindgen(typescript_type = "ValidationReport")]
    pub type JsValidationReport;

    /// Typed JavaScript `IdentifierInspection` object returned by [`validate_identifier`].
    #[wasm_bindgen(typescript_type = "IdentifierInspection")]
    pub type JsIdentifierInspection;

    /// Typed JavaScript `RuleView[]` array returned by [`list_rules`].
    #[wasm_bindgen(typescript_type = "RuleView[]")]
    pub type JsRuleViewArray;
}

fn parse_locale_opt(locale: Option<&str>) -> Locale {
    locale
        .and_then(|s| s.parse::<Locale>().ok())
        .unwrap_or(Locale::En)
}

fn to_js_object<T: Serialize>(value: &T) -> Result<JsValue, JsError> {
    value
        .serialize(&Serializer::json_compatible())
        .map_err(|e| JsError::new(&e.to_string()))
}

/// Validates an XML (`UBL 2.1` / `CII D16B`) or hybrid `PDF/A-3` (`efactura.xml`) invoice `Uint8Array`
/// and returns a typed JavaScript `ValidationReport` object (`{ isValid, recommendation, violations, ... }`).
///
/// # Errors
///
/// Returns [`JsError`] only if JS value serialization fails.
#[wasm_bindgen(js_name = validateInvoice)]
#[expect(
    clippy::needless_pass_by_value,
    reason = "wasm-bindgen ABI requires owned Option<String> for optional JS string arguments"
)]
pub fn validate_invoice(
    bytes: &[u8],
    locale: Option<String>,
    deny_warnings: Option<bool>,
    include_svrl: Option<bool>,
) -> Result<JsValidationReport, JsError> {
    let opts = ValidateOptions::with_locale(parse_locale_opt(locale.as_deref()))
        .deny_warnings(deny_warnings.unwrap_or(false))
        .include_svrl(include_svrl.unwrap_or(false));
    to_js_object(&validate_bytes(bytes, &opts)).map(JsCast::unchecked_into)
}

/// Extracts the embedded `efactura.xml` / `factur-x.xml` byte payload (`Uint8Array`) from a
/// hybrid `PDF/A-3` invoice buffer.
///
/// # Errors
///
/// Returns [`JsError`] if the PDF cannot be parsed, exceeds stream limits, or carries no XML invoice.
#[wasm_bindgen(js_name = extractPdfXml)]
pub fn extract_pdf_xml(bytes: &[u8]) -> Result<Vec<u8>, JsError> {
    extract_xml_from_pdf(bytes)
        .map(|ext| ext.xml_bytes)
        .map_err(|err| JsError::new(&err.to_string()))
}

/// Validates and normalizes a Romanian identifier (`CUI`, `CNP`, `0000000000000`, `ONRC`,
/// `RO IBAN`, `ISO 3166-2:RO` county, or Bucharest `SECTOR1`..`SECTOR6`).
///
/// # Errors
///
/// Returns [`JsError`] only if JS value serialization fails.
#[wasm_bindgen(js_name = validateIdentifier)]
#[expect(
    clippy::needless_pass_by_value,
    reason = "wasm-bindgen ABI requires owned Option<String> for optional JS string arguments"
)]
pub fn validate_identifier(
    input: &str,
    locale: Option<String>,
) -> Result<JsIdentifierInspection, JsError> {
    let report = inspect_identifier(input, parse_locale_opt(locale.as_deref()));
    to_js_object(&report).map(JsCast::unchecked_into)
}

/// Returns the complete localized `CIUS-RO 1.0.1` + `cius-ro-id` rule catalog as a JavaScript array.
///
/// # Errors
///
/// Returns [`JsError`] only if JS value serialization fails.
#[wasm_bindgen(js_name = listRules)]
#[expect(
    clippy::needless_pass_by_value,
    reason = "wasm-bindgen ABI requires owned Option<String> for optional JS string arguments"
)]
pub fn list_rules(locale: Option<String>) -> Result<JsRuleViewArray, JsError> {
    let loc = parse_locale_opt(locale.as_deref());
    let views: Vec<RuleView<'static>> = RULES.iter().map(|r| r.view(loc)).collect();
    to_js_object(&views).map(JsCast::unchecked_into)
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use cius_ro::{
        ContainerKind, DocumentIdentity, EngineId, EngineReport, EngineStatus, Recommendation,
        RuleCategory, Severity, ValidationReport, Violation, XmlSyntax,
    };
    use cius_ro_id::IdentifierKind;

    use super::*;

    struct TsSchema {
        unions: BTreeMap<String, BTreeSet<String>>,
        interfaces: BTreeMap<String, BTreeMap<String, bool>>,
    }

    fn parse_ts_schema(ts: &str) -> TsSchema {
        let mut unions: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        let mut interfaces: BTreeMap<String, BTreeMap<String, bool>> = BTreeMap::new();

        for chunk in ts.split("export ") {
            let chunk = chunk.trim();
            if let Some(rest) = chunk.strip_prefix("type ") {
                let (name, rhs) = rest.split_once('=').expect("type `=`");
                let rhs = rhs.split(';').next().expect("type `;`");
                let lits: BTreeSet<String> = rhs
                    .split('|')
                    .map(|part| part.trim().trim_matches('"').to_owned())
                    .filter(|s| !s.is_empty())
                    .collect();
                unions.insert(name.trim().to_owned(), lits);
            } else if let Some(rest) = chunk.strip_prefix("interface ") {
                let (name, body_rest) = rest.split_once('{').expect("interface `{`");
                let body = body_rest.split('}').next().expect("interface `}`");
                let mut fields = BTreeMap::new();
                for line in body.lines().map(str::trim).filter(|l| !l.is_empty()) {
                    let (raw_key, _) = line.split_once(':').expect("field `:`");
                    let raw_key = raw_key.trim();
                    let (key, optional) = match raw_key.strip_suffix('?') {
                        Some(k) => (k, true),
                        None => (raw_key, false),
                    };
                    fields.insert(key.to_owned(), optional);
                }
                interfaces.insert(name.trim().to_owned(), fields);
            }
        }
        TsSchema { unions, interfaces }
    }

    fn serialized_strings<T: Serialize>(items: &[T]) -> BTreeSet<String> {
        items
            .iter()
            .map(|v| {
                serde_json::to_value(v)
                    .expect("serialize enum")
                    .as_str()
                    .expect("string enum")
                    .to_owned()
            })
            .collect()
    }

    fn assert_interface_parity<T: Serialize>(
        interfaces: &BTreeMap<String, BTreeMap<String, bool>>,
        name: &str,
        full_sample: &T,
        minimal_sample: Option<&T>,
    ) {
        let ts_fields = interfaces
            .get(name)
            .unwrap_or_else(|| panic!("missing TS interface {name}"));
        let full_obj = serde_json::to_value(full_sample)
            .expect("serialize full sample")
            .as_object()
            .expect("JSON object")
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>();
        let expected_all: BTreeSet<String> = ts_fields.keys().cloned().collect();
        assert_eq!(full_obj, expected_all, "field mismatch on interface {name}");

        if let Some(min_sample) = minimal_sample {
            let min_obj = serde_json::to_value(min_sample)
                .expect("serialize minimal sample")
                .as_object()
                .expect("JSON object")
                .keys()
                .cloned()
                .collect::<BTreeSet<_>>();
            let expected_req: BTreeSet<String> = ts_fields
                .iter()
                .filter(|&(_, &opt)| !opt)
                .map(|(k, _)| k.clone())
                .collect();
            assert_eq!(
                min_obj, expected_req,
                "required/optional mismatch on interface {name}"
            );
        }
    }

    #[test]
    fn locale_parsing_defaults_and_bcp47_tags() {
        assert_eq!(parse_locale_opt(None), Locale::En);
        assert_eq!(parse_locale_opt(Some("")), Locale::En);
        assert_eq!(parse_locale_opt(Some("fr")), Locale::En);
        assert_eq!(parse_locale_opt(Some("ro")), Locale::Ro);
        assert_eq!(parse_locale_opt(Some("ro-RO")), Locale::Ro);
        assert_eq!(parse_locale_opt(Some("en-US")), Locale::En);
    }

    #[test]
    fn typescript_unions_match_serde_enums() {
        let unions = parse_ts_schema(include_str!("types.d.ts")).unions;
        assert_eq!(unions.len(), 9);
        assert_eq!(
            unions["Locale"],
            serialized_strings(&[Locale::En, Locale::Ro])
        );
        assert_eq!(
            unions["Severity"],
            serialized_strings(&[Severity::Fatal, Severity::Warning, Severity::Info])
        );
        assert_eq!(
            unions["Recommendation"],
            serialized_strings(&[Recommendation::Accept, Recommendation::Reject])
        );
        assert_eq!(
            unions["ContainerKind"],
            serialized_strings(&[ContainerKind::Xml, ContainerKind::PdfA3])
        );
        assert_eq!(
            unions["XmlSyntax"],
            serialized_strings(&[
                XmlSyntax::UblInvoice,
                XmlSyntax::UblCreditNote,
                XmlSyntax::CiiD16B,
            ])
        );
        assert_eq!(
            unions["EngineId"],
            serialized_strings(&[
                EngineId::Pdfa3Iso19005,
                EngineId::XsdSchema,
                EngineId::En16931Model,
                EngineId::AnafCiusRo,
                EngineId::Gateway,
            ])
        );
        assert_eq!(
            unions["EngineStatus"],
            serialized_strings(&[
                EngineStatus::Passed,
                EngineStatus::Failed,
                EngineStatus::Skipped,
            ])
        );
        assert_eq!(
            unions["RuleCategory"],
            serialized_strings(&[
                RuleCategory::Core,
                RuleCategory::PartyAddress,
                RuleCategory::Dates,
                RuleCategory::Cardinality,
                RuleCategory::Length,
                RuleCategory::Decimals,
                RuleCategory::Checksum,
                RuleCategory::ContainerXsd,
            ])
        );
        assert_eq!(
            unions["IdentifierKind"],
            serialized_strings(&[
                IdentifierKind::Cui,
                IdentifierKind::Cnp,
                IdentifierKind::AnafB2cPlaceholder,
                IdentifierKind::Onrc,
                IdentifierKind::Iban,
                IdentifierKind::County,
                IdentifierKind::BucharestSector,
                IdentifierKind::Unknown,
            ])
        );
    }

    #[test]
    fn typescript_interfaces_match_serde_structs() {
        let interfaces = parse_ts_schema(include_str!("types.d.ts")).interfaces;
        assert_eq!(interfaces.len(), 6);

        let full_id = DocumentIdentity {
            invoice_number: Some("F-1".into()),
            issue_date: Some("2026-10-15".into()),
            seller_name: Some("SRL".into()),
        };
        assert_interface_parity(
            &interfaces,
            "DocumentIdentity",
            &full_id,
            Some(&DocumentIdentity::default()),
        );

        let full_eng = EngineReport {
            id: EngineId::AnafCiusRo,
            name: "n".into(),
            version: "v".into(),
            status: EngineStatus::Skipped,
            rules_evaluated: 0,
            passed_count: 0,
            failed_count: 0,
            warning_count: 0,
            skip_reason: Some("r".into()),
        };
        let min_eng = EngineReport {
            skip_reason: None,
            ..full_eng.clone()
        };
        assert_interface_parity(&interfaces, "EngineReport", &full_eng, Some(&min_eng));

        let full_v = Violation {
            code: "BR-RO-001".into(),
            rule_sub_id: Some("SUB".into()),
            engine: EngineId::AnafCiusRo,
            severity: Severity::Fatal,
            business_term: Some("BT-24".into()),
            message: "m".into(),
            location: Some("/Invoice".into()),
            expected: Some("e".into()),
            actual: Some("a".into()),
            hint: Some("h".into()),
        };
        let min_v = Violation {
            rule_sub_id: None,
            business_term: None,
            location: None,
            expected: None,
            actual: None,
            hint: None,
            ..full_v.clone()
        };
        assert_interface_parity(&interfaces, "Violation", &full_v, Some(&min_v));

        let full_rep = ValidationReport {
            is_valid: true,
            recommendation: Recommendation::Accept,
            locale: Locale::Ro,
            container: ContainerKind::Xml,
            detected_syntax: Some(XmlSyntax::UblInvoice),
            customization_id: Some("urn:...".into()),
            identity: Some(full_id),
            engines: vec![full_eng],
            violations: vec![full_v],
            svrl_xml: Some("<svrl/>".into()),
        };
        let min_rep = validate_bytes(b"", &ValidateOptions::with_locale(Locale::Ro));
        assert_interface_parity(&interfaces, "ValidationReport", &full_rep, Some(&min_rep));

        let valid_cui = inspect_identifier("RO45678918", Locale::Ro);
        let unknown_id = inspect_identifier("???", Locale::En);
        assert_interface_parity(
            &interfaces,
            "IdentifierInspection",
            &valid_cui,
            Some(&unknown_id),
        );
        assert_interface_parity(&interfaces, "RuleView", &RULES[0].view(Locale::En), None);
    }
}
