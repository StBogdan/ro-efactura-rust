//! WebAssembly (`wasm-bindgen`) bindings for validating Romanian **ANAF `RO e-Factura`**
//! (`CIUS-RO 1.0.1`) electronic invoices and fiscal identifiers.
//!
//! Exposes four JavaScript/TypeScript entry points with [`tsify`]-generated TypeScript definitions:
//! - [`validate_invoice`] (`validateInvoice` in JS/TS): validates standalone XML (`UBL 2.1` / `CII D16B`)
//!   and hybrid `PDF/A-3` (`efactura.xml`) byte buffers with a typed [`ValidateOptions`] object.
//! - [`extract_pdf_xml`] (`extractPdfXml` in JS/TS): extracts embedded `efactura.xml` / `factur-x.xml`
//!   from a PDF byte slice.
//! - [`validate_identifier`] (`validateIdentifier` in JS/TS): verifies Romanian `CUI`, `CNP`, `ONRC`,
//!   `RO` IBAN, `ISO 3166-2:RO` county codes, and Bucharest `SECTOR1`..`SECTOR6`.
//! - [`list_rules`] (`listRules` in JS/TS): returns the localized `CIUS-RO 1.0.1` rule catalog.

use tsify::{Ts, Tsify};
use wasm_bindgen::prelude::*;

use cius_ro::{
    Locale, RULES, RuleView, ValidateOptions, ValidationReport, extract_xml_from_pdf,
    validate_bytes,
};
use cius_ro_id::{IdentifierInspection, inspect_identifier};

/// Validates an XML (`UBL 2.1` / `CII D16B`) or hybrid `PDF/A-3` (`efactura.xml`) invoice `Uint8Array`
/// using an optional [`ValidateOptions`] configuration object and returns a typed JavaScript
/// [`ValidationReport`] (`{ isValid, recommendation, violations, ... }`).
///
/// Any omitted fields in `options` fall back to [`ValidateOptions::default()`].
///
/// # Errors
///
/// Returns [`JsError`] if `options` cannot be deserialized as [`ValidateOptions`] or if JS
/// value serialization fails.
#[wasm_bindgen(js_name = validateInvoice)]
pub fn validate_invoice(
    bytes: &[u8],
    options: Option<Ts<ValidateOptions>>,
) -> Result<Ts<ValidationReport>, JsError> {
    let opts = match options {
        Some(js_opts) => js_opts.to_rust()?,
        None => ValidateOptions::default(),
    };
    Ok(validate_bytes(bytes, &opts).into_ts()?)
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
/// Returns [`JsError`] if `locale` is not `"en"` or `"ro"`, or if JS value serialization fails.
#[wasm_bindgen(js_name = validateIdentifier)]
pub fn validate_identifier(
    input: &str,
    locale: Option<Ts<Locale>>,
) -> Result<Ts<IdentifierInspection>, JsError> {
    let loc = match locale {
        Some(js_loc) => js_loc.to_rust()?,
        None => Locale::En,
    };
    Ok(inspect_identifier(input, loc).into_ts()?)
}

/// Returns the complete localized `CIUS-RO 1.0.1` + `cius-ro-id` rule catalog as a JavaScript array.
///
/// # Errors
///
/// Returns [`JsError`] if `locale` is not `"en"` or `"ro"`, or if JS value serialization fails.
#[wasm_bindgen(js_name = listRules)]
pub fn list_rules(locale: Option<Ts<Locale>>) -> Result<Vec<Ts<RuleView<'static>>>, JsError> {
    let loc = match locale {
        Some(js_loc) => js_loc.to_rust()?,
        None => Locale::En,
    };
    let views = RULES
        .iter()
        .map(|r| r.view(loc).into_ts())
        .collect::<Result<Vec<_>, _>>()?;
    Ok(views)
}

#[cfg(test)]
mod tests {
    use cius_ro::{
        ContainerKind, DocumentIdentity, EngineId, EngineReport, EngineStatus, Recommendation,
        RuleCategory, Severity, Violation, XmlSyntax,
    };
    use cius_ro_id::IdentifierKind;

    use super::*;

    #[test]
    fn validate_options_serde_defaults_and_partial_overrides() {
        let empty: ValidateOptions = serde_json::from_str("{}").expect("empty options JSON");
        assert_eq!(empty, ValidateOptions::default());

        let partial: ValidateOptions = serde_json::from_str(
            r#"{
                "locale": "ro",
                "runPdfa": false,
                "strictCiusRoCustomization": false,
                "denyWarnings": true,
                "includeSvrl": true
            }"#,
        )
        .expect("partial options JSON");
        assert_eq!(partial.locale, Locale::Ro);
        assert!(!partial.run_pdfa);
        assert!(partial.run_xsd);
        assert!(partial.run_en16931);
        assert!(partial.run_cius_ro_rules);
        assert!(partial.run_checksums);
        assert!(!partial.strict_cius_ro_customization);
        assert!(partial.deny_warnings);
        assert!(partial.include_svrl);

        assert!(serde_json::from_str::<ValidateOptions>(r#"{"locale": "fr"}"#).is_err());
    }

    #[test]
    fn tsify_declarations_cover_all_dto_types_and_options_flags() {
        let opts_decl = ValidateOptions::DECL;
        assert!(opts_decl.contains("export interface ValidateOptions"));
        for field in [
            "locale?: Locale;",
            "runPdfa?: boolean;",
            "runXsd?: boolean;",
            "runEn16931?: boolean;",
            "runCiusRoRules?: boolean;",
            "runChecksums?: boolean;",
            "strictCiusRoCustomization?: boolean;",
            "denyWarnings?: boolean;",
            "includeSvrl?: boolean;",
        ] {
            assert!(
                opts_decl.contains(field),
                "ValidateOptions::DECL missing `{field}`:\n{opts_decl}"
            );
        }

        assert!(Locale::DECL.contains(r#"export type Locale = "en" | "ro";"#));
        assert!(Severity::DECL.contains(r#"export type Severity = "FATAL" | "WARNING" | "INFO";"#));
        assert!(
            Recommendation::DECL.contains(r#"export type Recommendation = "accept" | "reject";"#)
        );
        assert!(ContainerKind::DECL.contains(r#"export type ContainerKind = "XML" | "PDFA3";"#));
        assert!(XmlSyntax::DECL.contains("UblInvoice") && XmlSyntax::DECL.contains("CiiD16B"));
        assert!(EngineId::DECL.contains("anaf_cius_ro"));
        assert!(EngineStatus::DECL.contains("SKIPPED"));
        assert!(RuleCategory::DECL.contains("party_address"));
        assert!(IdentifierKind::DECL.contains("anaf_b2c_placeholder"));

        assert!(DocumentIdentity::DECL.contains("invoiceNumber?: string;"));
        assert!(EngineReport::DECL.contains("skipReason?: string;"));
        assert!(Violation::DECL.contains("ruleSubId?: string;"));
        assert!(ValidationReport::DECL.contains("svrlXml?: string;"));
        assert!(IdentifierInspection::DECL.contains("normalized?: string;"));
        assert!(RuleView::<'static>::DECL.contains("export interface RuleView"));
    }
}
