//! # `cius-ro-wasm` — WebAssembly Bindings for Romanian ANAF `RO e-Factura` (`CIUS-RO 1.0.1`)
//!
//! Pure-Rust WebAssembly (`wasm32-unknown-unknown` / `wasm-bindgen`) bindings exposing:
//! - [`validate_invoice`] (`validateInvoice` in JS/TS): validates standalone XML (`UBL 2.1` / `CII D16B`)
//!   and hybrid `PDF/A-3` (`efactura.xml`) byte buffers entirely in the browser, Edge Worker, or Node.js.
//! - [`extract_pdf_xml`] (`extractPdfXml` in JS/TS): extracts embedded `efactura.xml` / `factur-x.xml`
//!   from a PDF byte slice.
//! - [`validate_identifier`] (`validateIdentifier` in JS/TS): verifies Romanian `CUI`, `CNP`, `ONRC`,
//!   `RO` IBAN, `ISO 3166-2:RO` county codes, and Bucharest `SECTOR1`..`SECTOR6`.
//! - [`list_rules`] (`listRules` in JS/TS): returns the localized `CIUS-RO 1.0.1` rule catalog.
//!
//! *Binding-uri WebAssembly (`wasm-bindgen`) pentru validarea facturilor electronice **ANAF RO e-Factura**
//! (`CIUS-RO 1.0.1`), extracția `efactura.xml` din PDF/A-3 și verificarea identificatorilor fiscali românești
//! direct în browser, Cloudflare Workers, Deno sau Node.js.*

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use serde::Serialize;
use serde_wasm_bindgen::Serializer;
use wasm_bindgen::prelude::*;

use cius_ro::{extract_xml_from_pdf, validate_bytes, Locale, RuleView, ValidateOptions, RULES};
use cius_ro_id::inspect_identifier;

fn parse_locale_opt(locale: Option<String>) -> Locale {
    locale
        .as_deref()
        .and_then(|s| s.parse::<Locale>().ok())
        .unwrap_or(Locale::En)
}

fn to_js_object<T: Serialize>(value: &T) -> Result<JsValue, JsError> {
    value
        .serialize(&Serializer::json_compatible())
        .map_err(|e| JsError::new(&e.to_string()))
}

/// Validates an XML (`UBL 2.1` / `CII D16B`) or hybrid `PDF/A-3` (`efactura.xml`) invoice `Uint8Array`
/// and returns a plain JavaScript `ValidationReport` object (`{ isValid, recommendation, violations, ... }`).
#[wasm_bindgen(js_name = validateInvoice)]
pub fn validate_invoice(
    bytes: &[u8],
    locale: Option<String>,
    deny_warnings: Option<bool>,
    include_svrl: Option<bool>,
) -> Result<JsValue, JsError> {
    let opts = ValidateOptions {
        locale: parse_locale_opt(locale),
        deny_warnings: deny_warnings.unwrap_or(false),
        include_svrl: include_svrl.unwrap_or(false),
        ..ValidateOptions::default()
    };
    to_js_object(&validate_bytes(bytes, &opts))
}

/// Extracts the embedded `efactura.xml` / `factur-x.xml` byte payload (`Uint8Array`) from a
/// hybrid `PDF/A-3` invoice buffer.
#[wasm_bindgen(js_name = extractPdfXml)]
pub fn extract_pdf_xml(bytes: &[u8]) -> Result<Vec<u8>, JsError> {
    extract_xml_from_pdf(bytes)
        .map(|ext| ext.xml_bytes)
        .map_err(|msg| JsError::new(&msg))
}

/// Validates and normalizes a Romanian identifier (`CUI`, `CNP`, `0000000000000`, `ONRC`,
/// `RO IBAN`, `ISO 3166-2:RO` county, or Bucharest `SECTOR1`..`SECTOR6`).
#[wasm_bindgen(js_name = validateIdentifier)]
pub fn validate_identifier(input: &str, locale: Option<String>) -> Result<JsValue, JsError> {
    to_js_object(&inspect_identifier(input, parse_locale_opt(locale).is_ro()))
}

/// Returns the complete localized `CIUS-RO 1.0.1` + `cius-ro-id` rule catalog as a JavaScript array.
#[wasm_bindgen(js_name = listRules)]
pub fn list_rules(locale: Option<String>) -> Result<JsValue, JsError> {
    let loc = parse_locale_opt(locale);
    let views: Vec<RuleView<'static>> = RULES.iter().map(|r| r.view(loc)).collect();
    to_js_object(&views)
}
