//! Official W3C XML Schema (`XSD 1.1`) Grammar & XXE Validator (`uppsala`).
//!
//! Embeds OASIS UBL 2.1 (`Invoice`, `CreditNote`, 14 common schemas) and UN/CEFACT CII D16B
//! (`100pD16B`, 4 schemas) inside the crate binary via `include_str!`.

use uppsala::Parser;

use crate::types::{
    EngineId, EngineReport, EngineStatus, Locale, Severity, ValidateOptions, Violation, XmlSyntax,
};

pub(crate) const ENGINE_ID: EngineId = EngineId::XsdSchema;
pub(crate) const ENGINE_NAME: &str = "Official W3C XSD 1.1 Grammar Validator";

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

    const CII_FILES: &[(&str, &str)] = &[
        (
            "CrossIndustryInvoice_100pD16B.xsd",
            include_str!("../schemas/xsd/cii_d16b/CrossIndustryInvoice_100pD16B.xsd"),
        ),
        (
            "CrossIndustryInvoice_QualifiedDataType_100pD16B.xsd",
            include_str!("../schemas/xsd/cii_d16b/CrossIndustryInvoice_QualifiedDataType_100pD16B.xsd"),
        ),
        (
            "CrossIndustryInvoice_ReusableAggregateBusinessInformationEntity_100pD16B.xsd",
            include_str!("../schemas/xsd/cii_d16b/CrossIndustryInvoice_ReusableAggregateBusinessInformationEntity_100pD16B.xsd"),
        ),
        (
            "CrossIndustryInvoice_UnqualifiedDataType_100pD16B.xsd",
            include_str!("../schemas/xsd/cii_d16b/CrossIndustryInvoice_UnqualifiedDataType_100pD16B.xsd"),
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

    static XSD_SUITES: OnceLock<Result<CompiledXsdSuites, String>> = OnceLock::new();

    pub(super) fn get_validators() -> Result<&'static CompiledXsdSuites, String> {
        XSD_SUITES
            .get_or_init(compile_all_xsd_suites)
            .as_ref()
            .map_err(Clone::clone)
    }

    fn compile_one(xsd: &str, base: &Path, label: &str) -> Result<XsdValidator, String> {
        let doc = uppsala::parse(xsd).map_err(|e| format!("failed parsing {label} XSD: {e}"))?;
        XsdValidator::from_schema_with_base_path(&doc, Some(base))
            .map_err(|e| format!("failed compiling {label} XSD validator: {e}"))
    }

    fn compile_all_xsd_suites() -> Result<CompiledXsdSuites, String> {
        let tmp = tempfile::tempdir().map_err(|e| format!("failed creating XSD temp dir: {e}"))?;
        let cii_dir = tmp.path().join("cii");
        let ubl_dir = tmp.path().join("ubl");
        let ubl_common_dir = ubl_dir.join("common");
        fs::create_dir_all(&cii_dir).map_err(|e| e.to_string())?;
        fs::create_dir_all(&ubl_common_dir).map_err(|e| e.to_string())?;

        for (name, content) in CII_FILES {
            fs::write(cii_dir.join(name), content).map_err(|e| e.to_string())?;
        }
        for (name, content) in UBL_COMMON_FILES {
            fs::write(ubl_common_dir.join(name), content).map_err(|e| e.to_string())?;
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
pub fn warm_xsd_validators() -> Result<(), String> {
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
    xml_res: Result<&str, &String>,
    syntax: Option<XmlSyntax>,
    opts: &ValidateOptions,
) -> XsdOutcome {
    if !opts.run_xsd {
        return XsdOutcome {
            report: EngineReport::skipped(
                ENGINE_ID,
                ENGINE_NAME,
                "W3C XSD 1.1",
                "Disabled by ValidateOptions",
            ),
            violations: Vec::new(),
        };
    }

    let xml = match xml_res {
        Ok(s) => s,
        Err(enc_err) => {
            return fatal(
                "XSD-ENCODING-001",
                "XML 1.0 Encoding",
                (*enc_err).clone(),
                Some("Byte offset 0"),
                None,
                "W3C XSD 1.1",
            );
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
            let err_str = err.to_string();
            if err_str.contains("forbid_dtd") || err_str.contains("forbid_entities") {
                let (msg, hint) = match opts.locale {
                    Locale::En => (
                        "XML DOCTYPE and ENTITY declarations are strictly forbidden in EN 16931 / RO e-Factura documents.",
                        "Remove all <!DOCTYPE ...> and <!ENTITY ...> declarations.",
                    ),
                    Locale::Ro => (
                        "Declarațiile XML DOCTYPE și ENTITY sunt strict interzise în facturile electronice EN 16931 / RO e-Factura.",
                        "Eliminați toate declarațiile <!DOCTYPE ...> și <!ENTITY ...> din documentul XML.",
                    ),
                };
                return fatal(
                    "XSD-XXE-PROHIBITED",
                    "XML 1.0 DTD / XXE Prohibition",
                    msg.to_string(),
                    Some("XML Prologue"),
                    Some(hint),
                    "W3C XSD 1.1",
                );
            }
            let hint = match opts.locale {
                Locale::En => "Fix XML syntax, closing tags, and namespace declarations.",
                Locale::Ro => {
                    "Corectați sintaxa XML, etichetele de închidere și declarațiile de namespace."
                }
            };
            return fatal(
                "XSD-WELLFORMEDNESS",
                "W3C XML 1.0",
                err_str,
                None,
                Some(hint),
                "W3C XSD 1.1",
            );
        }
    };

    let Some(syntax_kind) = syntax else {
        let msg = match opts.locale {
            Locale::En => "Root XML element is neither OASIS UBL 2.1 Invoice/CreditNote nor UN/CEFACT CrossIndustryInvoice.",
            Locale::Ro => "Elementul rădăcină XML nu este nici OASIS UBL 2.1 Invoice/CreditNote, nici UN/CEFACT CrossIndustryInvoice.",
        };
        return fatal(
            "XSD-UNKNOWN-ROOT",
            "EN 16931 Syntax Binding",
            msg.to_string(),
            Some("/*"),
            None,
            "UBL 2.1 / CII D16B",
        );
    };

    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    {
        let _ = doc;
        return XsdOutcome {
            report: EngineReport::skipped(
                ENGINE_ID,
                ENGINE_NAME,
                syntax_kind.label(),
                "Browser WASM target skips multi-file XSD fs bootstrap (XML well-formedness, XXE defense, and EN 16931 syntax order/duplicate gates passed)",
            ),
            violations: Vec::new(),
        };
    }

    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    {
        let suites = match native_compiled::get_validators() {
            Ok(s) => s,
            Err(err) => {
                return fatal(
                    "XSD-ENGINE-INIT",
                    "XSD Bootstrap",
                    err,
                    None,
                    None,
                    syntax_kind.label(),
                );
            }
        };

        let (validator, spec_label) = match syntax_kind {
            XmlSyntax::CiiD16B => (&suites.cii_d16b, "UN/CEFACT CII D16B (100pD16B)"),
            XmlSyntax::UblInvoice => (&suites.ubl_invoice_2_1, "OASIS UBL 2.1 Invoice"),
            XmlSyntax::UblCreditNote => (&suites.ubl_credit_note_2_1, "OASIS UBL 2.1 CreditNote"),
        };

        let xsd_errors = validator.validate(&doc);
        let element_checks = doc
            .descendants(doc.root())
            .into_iter()
            .filter(|&id| doc.element(id).is_some())
            .count()
            .max(1);
        let hint_str = match opts.locale {
            Locale::En => "Verify UBL/CII element sequence order, namespace prefixes, and XSD datatype facets.",
            Locale::Ro => "Verificați ordinea elementelor UBL/CII din schema XSD, prefixele de namespace și tipurile de date.",
        };

        let violations = xsd_errors
            .iter()
            .take(100)
            .enumerate()
            .map(|(idx, err)| {
                let loc = match (err.line, err.column) {
                    (Some(l), Some(c)) => Some(format!("line {l}, column {c}")),
                    (Some(l), None) => Some(format!("line {l}")),
                    _ => None,
                };
                Violation {
                    code: format!("XSD-SCHEMA-{:03}", idx + 1),
                    rule_sub_id: None,
                    engine: ENGINE_ID,
                    severity: Severity::Fatal,
                    business_term: Some(spec_label.to_string()),
                    message: err.message.clone(),
                    location: loc,
                    expected: None,
                    actual: None,
                    hint: Some(hint_str.to_string()),
                }
            })
            .collect();

        let failed_count = xsd_errors.len();
        let rules_evaluated = element_checks.max(failed_count);
        XsdOutcome {
            report: EngineReport {
                id: ENGINE_ID,
                name: ENGINE_NAME.to_string(),
                version: spec_label.to_string(),
                status: if failed_count == 0 {
                    EngineStatus::Passed
                } else {
                    EngineStatus::Failed
                },
                rules_evaluated,
                passed_count: rules_evaluated.saturating_sub(failed_count),
                failed_count,
                warning_count: 0,
                skip_reason: None,
            },
            violations,
        }
    }
}

fn fatal(
    code: &str,
    term: &str,
    message: String,
    location: Option<&str>,
    hint: Option<&str>,
    version: &str,
) -> XsdOutcome {
    XsdOutcome {
        report: EngineReport {
            id: ENGINE_ID,
            name: ENGINE_NAME.to_string(),
            version: version.to_string(),
            status: EngineStatus::Failed,
            rules_evaluated: 1,
            passed_count: 0,
            failed_count: 1,
            warning_count: 0,
            skip_reason: None,
        },
        violations: vec![Violation {
            code: code.to_string(),
            rule_sub_id: None,
            engine: ENGINE_ID,
            severity: Severity::Fatal,
            business_term: Some(term.to_string()),
            message,
            location: location.map(str::to_owned),
            expected: None,
            actual: None,
            hint: hint.map(str::to_owned),
        }],
    }
}
