//! # `cius-ro` — Pure-Rust Romanian ANAF `RO e-Factura` (`CIUS-RO 1.0.1`) Validator
//!
//! Pure-Rust, zero-JVM, in-memory e-invoice validator for Romania's **ANAF `RO e-Factura`**
//! national specification (`CIUS-RO 1.0.1` / `ROeFactura v1.0.9`), combining four deterministic layers:
//!
//! 1. **ISO 19005-3 (`PDF/A-3`) & Hybrid `efactura.xml` Extraction** (`feature = "pdfa3"`):
//!    Inspects binary PDF headers, encryption prohibition, trailer `/ID`, stream filters, `GTS_PDFA1` ICC
//!    color profiles, embedded TrueType/OpenType font programs, unfiltered XMP `pdfaid:part=3`, and
//!    extracts `efactura.xml` / `factur-x.xml`.
//! 2. **Official W3C XML Schema (`XSD 1.1`) Grammar & XXE Defense** (`feature = "xsd"`):
//!    Validates against embedded OASIS UBL 2.1 (`Invoice`, `CreditNote`) and UN/CEFACT CII D16B schemas.
//! 3. **CEN EN 16931-1 Fixed-Point Base-10 Semantic Model** (`en16931` + `en16931-formats`):
//!    Exact `rust_decimal` monetary rounding, VAT breakdown math, and EN 16931 core business rules.
//! 4. **Complete ANAF `CIUS-RO 1.0.1` (`BR-RO-*`) & `cius-ro-id` Checksums**:
//!    All 122 official ANAF `ROeFactura v1.0.9` assertions (`BR-RO-001`..`210`, `BR-RO-DT*`, `BR-RO-A*`,
//!    `BR-RO-L*`, `BR-RO-Z2`, `BR-RO-050`) plus Romanian `CUI` modulo-11, `CNP` / `0000000000000`, `ONRC`,
//!    and `RO` IBAN (`ISO 7064 MOD 97-10`) checks, with bilingual English (`en`) and Romanian (`ro`) diagnostics.
//!
//! *Validator pur în Rust (fără JVM, complet în memorie) pentru facturile electronice **ANAF RO e-Factura**
//! (`CIUS-RO 1.0.1` / `ROeFactura v1.0.9`), incluzând modelul semantic EN 16931 în virgulă fixă baza 10,
//! toate regulile naționale `BR-RO-*`, validare XSD 1.1 (UBL 2.1 și CII D16B), extracția `efactura.xml`
//! din containere hibride PDF/A-3 și mesaje bilingve (română / engleză).*
//!
//! ## Example
//!
//! ```
//! use cius_ro::{validate_bytes, Locale, ValidateOptions};
//!
//! let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
//! <Invoice xmlns="urn:oasis:names:specification:ubl:schema:xsd:Invoice-2"
//!          xmlns:cac="urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2"
//!          xmlns:cbc="urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2">
//!   <cbc:CustomizationID>urn:cen.eu:en16931:2017#compliant#urn:efactura.mfinante.ro:CIUS-RO:1.0.1</cbc:CustomizationID>
//!   <cbc:ID>FCT-2026-001</cbc:ID>
//!   <cbc:IssueDate>2026-10-15</cbc:IssueDate>
//!   <cbc:DueDate>2026-11-15</cbc:DueDate>
//!   <cbc:InvoiceTypeCode>380</cbc:InvoiceTypeCode>
//!   <cbc:DocumentCurrencyCode>RON</cbc:DocumentCurrencyCode>
//!   <cac:AccountingSupplierParty><cac:Party>
//!     <cac:PostalAddress>
//!       <cbc:StreetName>Calea Dorobanti 42</cbc:StreetName>
//!       <cbc:CityName>SECTOR1</cbc:CityName>
//!       <cbc:CountrySubentity>RO-B</cbc:CountrySubentity>
//!       <cac:Country><cbc:IdentificationCode>RO</cbc:IdentificationCode></cac:Country>
//!     </cac:PostalAddress>
//!     <cac:PartyTaxScheme><cbc:CompanyID>RO45678918</cbc:CompanyID><cac:TaxScheme><cbc:ID>VAT</cbc:ID></cac:TaxScheme></cac:PartyTaxScheme>
//!     <cac:PartyLegalEntity><cbc:RegistrationName>Exemplu SRL</cbc:RegistrationName></cac:PartyLegalEntity>
//!   </cac:Party></cac:AccountingSupplierParty>
//!   <cac:AccountingCustomerParty><cac:Party>
//!     <cac:PostalAddress>
//!       <cbc:StreetName>Str. Memorandumului 10</cbc:StreetName>
//!       <cbc:CityName>Cluj-Napoca</cbc:CityName>
//!       <cbc:CountrySubentity>RO-CJ</cbc:CountrySubentity>
//!       <cac:Country><cbc:IdentificationCode>RO</cbc:IdentificationCode></cac:Country>
//!     </cac:PostalAddress>
//!     <cac:PartyLegalEntity><cbc:RegistrationName>Client SRL</cbc:RegistrationName><cbc:CompanyID>0000000000000</cbc:CompanyID></cac:PartyLegalEntity>
//!   </cac:Party></cac:AccountingCustomerParty>
//!   <cac:PaymentMeans>
//!     <cbc:PaymentMeansCode>58</cbc:PaymentMeansCode>
//!     <cac:PayeeFinancialAccount><cbc:ID>RO49AAAA1B31007593840000</cbc:ID></cac:PayeeFinancialAccount>
//!   </cac:PaymentMeans>
//!   <cac:TaxTotal>
//!     <cbc:TaxAmount currencyID="RON">190.00</cbc:TaxAmount>
//!     <cac:TaxSubtotal>
//!       <cbc:TaxableAmount currencyID="RON">1000.00</cbc:TaxableAmount>
//!       <cbc:TaxAmount currencyID="RON">190.00</cbc:TaxAmount>
//!       <cac:TaxCategory><cbc:ID>S</cbc:ID><cbc:Percent>19.00</cbc:Percent><cac:TaxScheme><cbc:ID>VAT</cbc:ID></cac:TaxScheme></cac:TaxCategory>
//!     </cac:TaxSubtotal>
//!   </cac:TaxTotal>
//!   <cac:LegalMonetaryTotal>
//!     <cbc:LineExtensionAmount currencyID="RON">1000.00</cbc:LineExtensionAmount>
//!     <cbc:TaxExclusiveAmount currencyID="RON">1000.00</cbc:TaxExclusiveAmount>
//!     <cbc:TaxInclusiveAmount currencyID="RON">1190.00</cbc:TaxInclusiveAmount>
//!     <cbc:PayableAmount currencyID="RON">1190.00</cbc:PayableAmount>
//!   </cac:LegalMonetaryTotal>
//!   <cac:InvoiceLine>
//!     <cbc:ID>1</cbc:ID>
//!     <cbc:InvoicedQuantity unitCode="C62">1</cbc:InvoicedQuantity>
//!     <cbc:LineExtensionAmount currencyID="RON">1000.00</cbc:LineExtensionAmount>
//!     <cac:Item>
//!       <cbc:Name>Servicii IT</cbc:Name>
//!       <cac:ClassifiedTaxCategory><cbc:ID>S</cbc:ID><cbc:Percent>19.00</cbc:Percent><cac:TaxScheme><cbc:ID>VAT</cbc:ID></cac:TaxScheme></cac:ClassifiedTaxCategory>
//!     </cac:Item>
//!     <cac:Price><cbc:PriceAmount currencyID="RON">1000.00</cbc:PriceAmount></cac:Price>
//!   </cac:InvoiceLine>
//! </Invoice>"#;
//!
//! let report = validate_bytes(xml, &ValidateOptions::with_locale(Locale::Ro));
//! assert!(report.is_valid, "Unexpected violations: {:?}", report.violations);
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod br_ro;
#[cfg(feature = "pdfa3")]
pub mod pdfa3;
pub mod rules;
pub mod semantic;
pub mod types;
#[cfg(feature = "xsd")]
pub mod xsd;

pub use cius_ro_id;

pub use br_ro::{
    decode_xml_bytes, detect_root_syntax_and_bt24, evaluate_cius_ro_rules, BrRoOutcome,
};
#[cfg(feature = "pdfa3")]
pub use pdfa3::{extract_xml_from_pdf, inspect_pdfa3, ExtractedPdfXml, PdfInspectionOutcome};
pub use rules::{find_rule, render_rules_markdown, RuleDescriptor, RuleView, RULES};
pub use semantic::render_svrl_report;
pub use types::{
    ContainerKind, DocumentIdentity, EngineReport, EngineStatus, Locale, Severity, ValidateOptions,
    ValidationReport, Violation, XmlSyntax, CIUS_RO_CUSTOMIZATION_ID,
};
#[cfg(feature = "xsd")]
pub use xsd::warm_xsd_validators;

const PDFA_ENGINE_ID: &str = "pdfa3_iso19005";
const PDFA_ENGINE_NAME: &str = "ISO 19005-3 (PDF/A-3) & efactura.xml Inspector";
const PDFA_ENGINE_VER: &str = "ISO 19005-3:2012";

/// Auto-detects whether `bytes` is a standalone XML invoice or a hybrid PDF/A-3 document (`%PDF-`)
/// and runs all configured verification layers strictly in memory.
#[must_use]
pub fn validate_bytes(bytes: &[u8], opts: &ValidateOptions) -> ValidationReport {
    if bytes.is_empty() {
        let msg = match opts.locale {
            Locale::En => "Input payload is empty (0 bytes).",
            Locale::Ro => "Fișierul de intrare este gol (0 octeți).",
        };
        return reject_report(
            ContainerKind::Xml,
            opts,
            Vec::new(),
            vec![Violation {
                code: "INPUT-EMPTY".to_string(),
                rule_sub_id: None,
                engine: "gateway".to_string(),
                severity: Severity::Fatal,
                business_term: None,
                message: msg.to_string(),
                location: None,
                expected: Some("XML e-invoice or PDF/A-3 hybrid invoice".to_string()),
                actual: Some("0 bytes".to_string()),
                hint: None,
            }],
        );
    }

    if bytes[..bytes.len().min(1024)]
        .windows(5)
        .any(|w| w == b"%PDF-")
    {
        return validate_pdf_container(bytes, opts);
    }

    validate_xml_internal(bytes, ContainerKind::Xml, Vec::new(), Vec::new(), opts)
}

/// Validates a standalone UTF-8 / UTF-16 XML e-invoice byte slice against XSD, EN 16931,
/// ANAF `CIUS-RO 1.0.1` (`BR-RO-*`), and `cius-ro-id` fiscal checksums.
#[must_use]
pub fn validate_xml(xml_bytes: &[u8], opts: &ValidateOptions) -> ValidationReport {
    let engines = vec![EngineReport::skipped(
        PDFA_ENGINE_ID,
        PDFA_ENGINE_NAME,
        PDFA_ENGINE_VER,
        "Input is standalone XML",
    )];
    validate_xml_internal(xml_bytes, ContainerKind::Xml, engines, Vec::new(), opts)
}

fn validate_pdf_container(bytes: &[u8], opts: &ValidateOptions) -> ValidationReport {
    #[cfg(feature = "pdfa3")]
    {
        let pdf_out = pdfa3::inspect_pdfa3(bytes, opts.run_pdfa, opts.locale);
        if let Some(ext) = pdf_out.extracted_xml {
            return validate_xml_internal(
                &ext.xml_bytes,
                ContainerKind::PdfA3,
                vec![pdf_out.report],
                pdf_out.violations,
                opts,
            );
        }
        reject_report(
            ContainerKind::PdfA3,
            opts,
            vec![pdf_out.report],
            pdf_out.violations,
        )
    }

    #[cfg(not(feature = "pdfa3"))]
    {
        let _ = bytes;
        let msg = match opts.locale {
            Locale::En => "Input is a PDF container, but crate feature 'pdfa3' is disabled.",
            Locale::Ro => {
                "Documentul este de tip PDF, dar funcționalitatea 'pdfa3' nu este activată."
            }
        };
        reject_report(
            ContainerKind::PdfA3,
            opts,
            vec![EngineReport::skipped(
                PDFA_ENGINE_ID,
                PDFA_ENGINE_NAME,
                PDFA_ENGINE_VER,
                "Feature 'pdfa3' disabled at compile time",
            )],
            vec![Violation {
                code: "PDFA3-FEATURE-DISABLED".to_string(),
                rule_sub_id: None,
                engine: PDFA_ENGINE_ID.to_string(),
                severity: Severity::Fatal,
                business_term: None,
                message: msg.to_string(),
                location: None,
                expected: None,
                actual: None,
                hint: None,
            }],
        )
    }
}

fn reject_report(
    container: ContainerKind,
    opts: &ValidateOptions,
    engines: Vec<EngineReport>,
    violations: Vec<Violation>,
) -> ValidationReport {
    let svrl_xml = opts
        .include_svrl
        .then(|| semantic::render_svrl_report(&violations));
    ValidationReport {
        is_valid: false,
        recommendation: "reject".to_string(),
        locale: opts.locale,
        container,
        detected_syntax: None,
        customization_id: None,
        identity: None,
        engines,
        violations,
        svrl_xml,
    }
}

fn validate_xml_internal(
    xml_bytes: &[u8],
    container: ContainerKind,
    mut engines: Vec<EngineReport>,
    mut violations: Vec<Violation>,
    opts: &ValidateOptions,
) -> ValidationReport {
    if container == ContainerKind::Xml && engines.is_empty() {
        engines.push(EngineReport::skipped(
            PDFA_ENGINE_ID,
            PDFA_ENGINE_NAME,
            PDFA_ENGINE_VER,
            "Input is standalone XML",
        ));
    }

    let decoded = br_ro::decode_xml_bytes(xml_bytes);
    let parsed_doc = decoded
        .as_deref()
        .ok()
        .and_then(|s| roxmltree::Document::parse(s).ok());
    let (mut syntax, mut customization_id) = parsed_doc
        .as_ref()
        .map(|d| br_ro::detect_root_syntax_and_bt24(d.root_element()))
        .unwrap_or((None, None));

    #[cfg(feature = "xsd")]
    {
        let xsd_out = xsd::evaluate_xsd(decoded.as_deref(), syntax, opts);
        engines.push(xsd_out.report);
        violations.extend(xsd_out.violations);
    }
    #[cfg(not(feature = "xsd"))]
    {
        let _ = &decoded;
        engines.push(EngineReport::skipped(
            "xsd_schema",
            "Official W3C XSD 1.1 Grammar Validator",
            "W3C XSD 1.1",
            "Feature 'xsd' disabled at compile time",
        ));
    }

    let en_out = semantic::evaluate_en16931(xml_bytes, customization_id.as_deref(), opts);
    let identity = en_out.identity;
    let pre_invoice = en_out.invoice;
    engines.push(en_out.report);
    violations.extend(en_out.violations);

    let ro_out = match parsed_doc.as_ref() {
        Some(doc) => br_ro::evaluate_parsed(xml_bytes, doc, pre_invoice, opts),
        None => br_ro::skipped_for_unparsed(opts),
    };
    syntax = syntax.or(ro_out.syntax);
    customization_id = customization_id.or(ro_out.customization_id);
    engines.push(ro_out.report);
    violations.extend(ro_out.violations);

    let has_fatal = violations.iter().any(|v| v.severity == Severity::Fatal);
    let has_warning = violations.iter().any(|v| v.severity == Severity::Warning);
    let is_valid = !has_fatal && !(opts.deny_warnings && has_warning);
    let svrl_xml = opts
        .include_svrl
        .then(|| semantic::render_svrl_report(&violations));

    ValidationReport {
        is_valid,
        recommendation: if is_valid { "accept" } else { "reject" }.to_string(),
        locale: opts.locale,
        container,
        detected_syntax: syntax,
        customization_id,
        identity,
        engines,
        violations,
        svrl_xml,
    }
}

#[cfg(doctest)]
mod readme_doctests {
    #[doc = include_str!("../README.md")]
    struct ReadmeEn;

    #[doc = include_str!("../README.ro.md")]
    struct ReadmeRo;
}
