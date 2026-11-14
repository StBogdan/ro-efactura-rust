//! Electronic invoice validator for Romania's **ANAF `RO e-Factura`** specification
//! (`CIUS-RO 1.0.1` / `ROeFactura v1.0.9`).
//!
//! Runs four verification layers over standalone XML (`OASIS UBL 2.1` / `UN/CEFACT CII D16B`) or
//! hybrid `PDF/A-3` containers:
//!
//! 1. **ISO 19005-3 (`PDF/A-3`) & Hybrid `efactura.xml` Extraction** (`feature = "pdfa3"`):
//!    Checks PDF headers, encryption prohibition, trailer `/ID`, stream filters, `GTS_PDFA1` ICC
//!    color profiles, embedded TrueType/OpenType font programs, and XMP `pdfaid:part=3`, then
//!    extracts `efactura.xml` / `factur-x.xml` under bounded decompression budgets.
//! 2. **W3C XML Schema (`XSD 1.1`) Grammar & XXE Defense** (`feature = "xsd"`):
//!    Validates against bundled OASIS UBL 2.1 (`Invoice`, `CreditNote`) and UN/CEFACT CII D16B
//!    schemas (compiled once per process via an ephemeral temporary directory on native targets;
//!    on `wasm32-unknown-unknown`, XML well-formedness and `DOCTYPE`/`ENTITY` XXE gates run in
//!    memory and multi-file XSD compilation is skipped).
//! 3. **CEN EN 16931-1 Semantic Model** (`en16931` + `en16931-formats`):
//!    Base-10 fixed-point (`rust_decimal`) monetary rounding, VAT breakdown math, and EN 16931
//!    core business rules.
//! 4. **ANAF `CIUS-RO 1.0.1` (`BR-RO-*`) & `cius-ro-id` Checksums**:
//!    All 122 official ANAF `ROeFactura v1.0.9` assertions (`BR-RO-001`..`210`, `BR-RO-DT*`,
//!    `BR-RO-A*`, `BR-RO-L*`, `BR-RO-Z2`) plus Romanian `CUI` modulo-11, `CNP` / `0000000000000`,
//!    `ONRC`, and `RO` IBAN (`ISO 7064 MOD 97-10`) checks, with English (`en`) and Romanian (`ro`)
//!    diagnostics.
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

pub mod br_ro;
#[cfg(feature = "pdfa3")]
pub mod pdfa3;
pub mod rules;
pub mod semantic;
mod stage_acc;
pub mod types;
#[cfg(feature = "xsd")]
pub mod xsd;

pub use cius_ro_id;

pub use br_ro::{
    BrRoOutcome, decode_xml_bytes, detect_root_syntax_and_bt24, evaluate_cius_ro_rules,
};
#[cfg(feature = "pdfa3")]
pub use pdfa3::{ExtractedPdfXml, PdfInspectionOutcome, extract_xml_from_pdf, inspect_pdfa3};
pub use rules::{RULES, RuleDescriptor, RuleView, find_rule, render_rules_markdown};
#[cfg(feature = "svrl")]
pub use semantic::render_svrl_report;
pub use types::{
    CIUS_RO_CUSTOMIZATION_ID, ContainerKind, DocumentIdentity, EngineId, EngineReport,
    EngineStatus, Locale, LocalizedDisplay, ParseLocaleError, Recommendation, RuleCategory,
    Severity, ValidateOptions, ValidationReport, Violation, XmlDecodeError, XmlSyntax,
};
#[cfg(feature = "xsd")]
pub use xsd::warm_xsd_validators;

const PDFA_ENGINE_ID: EngineId = EngineId::Pdfa3Iso19005;
const PDFA_ENGINE_NAME: &str = "ISO 19005-3 (PDF/A-3) & efactura.xml Inspector";
const PDFA_ENGINE_VER: &str = "ISO 19005-3:2012";

/// Auto-detects whether `bytes` is a standalone XML invoice or a hybrid PDF/A-3 document (`%PDF-`)
/// and runs all configured verification layers on the byte slice (on native targets, embedded XSD
/// schemas are compiled once per process via an ephemeral temporary directory on first use).
#[must_use]
pub fn validate_bytes(bytes: &[u8], opts: &ValidateOptions) -> ValidationReport {
    if bytes.is_empty() {
        let (msg, expected, actual) = match opts.locale {
            Locale::En => (
                "Input payload is empty (0 bytes).",
                "XML e-invoice or PDF/A-3 hybrid invoice",
                "0 bytes",
            ),
            Locale::Ro => (
                "Fișierul de intrare este gol (0 octeți).",
                "Factură electronică XML sau container hibrid PDF/A-3",
                "0 octeți",
            ),
        };
        return reject_report(
            ContainerKind::Xml,
            opts,
            Vec::new(),
            vec![Violation {
                code: "INPUT-EMPTY".to_string(),
                rule_sub_id: None,
                engine: EngineId::Gateway,
                severity: Severity::Fatal,
                business_term: None,
                message: msg.to_string(),
                location: None,
                expected: Some(expected.to_string()),
                actual: Some(actual.to_string()),
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
                engine: PDFA_ENGINE_ID,
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

fn maybe_render_svrl(opts: &ValidateOptions, violations: &[Violation]) -> Option<String> {
    #[cfg(feature = "svrl")]
    {
        opts.include_svrl
            .then(|| semantic::render_svrl_report(violations))
    }
    #[cfg(not(feature = "svrl"))]
    {
        let _ = (opts, violations);
        None
    }
}

fn reject_report(
    container: ContainerKind,
    opts: &ValidateOptions,
    engines: Vec<EngineReport>,
    violations: Vec<Violation>,
) -> ValidationReport {
    let svrl_xml = maybe_render_svrl(opts, &violations);
    ValidationReport {
        is_valid: false,
        recommendation: Recommendation::Reject,
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
    let (mut syntax, mut customization_id) = parsed_doc.as_ref().map_or((None, None), |d| {
        br_ro::detect_root_syntax_and_bt24(d.root_element())
    });

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
            EngineId::XsdSchema,
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

    let ro_out = parsed_doc.as_ref().map_or_else(
        || br_ro::skipped_for_unparsed(opts),
        |doc| br_ro::evaluate_parsed(xml_bytes, doc, pre_invoice, opts),
    );
    syntax = syntax.or(ro_out.syntax);
    customization_id = customization_id.or(ro_out.customization_id);
    engines.push(ro_out.report);
    violations.extend(ro_out.violations);

    let has_fatal = violations.iter().any(|v| v.severity == Severity::Fatal);
    let has_warning = violations.iter().any(|v| v.severity == Severity::Warning);
    let is_valid = !(has_fatal || opts.deny_warnings && has_warning);
    let svrl_xml = maybe_render_svrl(opts, &violations);

    ValidationReport {
        is_valid,
        recommendation: if is_valid {
            Recommendation::Accept
        } else {
            Recommendation::Reject
        },
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

    #[doc = include_str!("../../../README.md")]
    struct WorkspaceReadmeEn;

    #[doc = include_str!("../../../README.ro.md")]
    struct WorkspaceReadmeRo;
}
