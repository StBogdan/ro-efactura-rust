//! Pure-Rust evaluator for ANAF `RO e-Factura` (`CIUS-RO 1.0.1` / `ROeFactura v1.0.9`) rules
//! and `cius-ro-id` Romanian identifier checksums across OASIS UBL 2.1 (`Invoice`, `CreditNote`)
//! and UN/CEFACT CII D16B (`CrossIndustryInvoice`), operating on the shared `en16931::Invoice`
//! semantic model plus lexical XML facets (`BR-RO-DT*`, `BR-RO-Z2`).

use cius_ro_id::{
    is_anaf_b2c_placeholder, is_onrc_candidate, parse_cui_candidate, BucharestSector, Cnp, County,
    Onrc, RoIban,
};
use en16931::invoice::{Party, PaymentMeans, PostalAddress};
use en16931::{Date, Invoice};
use roxmltree::{Document, Node};

use crate::types::{
    EngineId, EngineReport, EngineStatus, Locale, Severity, ValidateOptions, Violation, XmlSyntax,
    CIUS_RO_CUSTOMIZATION_ID,
};

pub(crate) const ENGINE_ID: EngineId = EngineId::AnafCiusRo;
pub(crate) const ENGINE_NAME: &str = "ANAF RO e-Factura (CIUS-RO 1.0.1) & Checksum Engine";
pub(crate) const ENGINE_VERSION: &str = "CIUS-RO 1.0.1 (ROeFactura v1.0.9)";

const UBL_INVOICE_NS: &str = "urn:oasis:names:specification:ubl:schema:xsd:Invoice-2";
const UBL_CREDITNOTE_NS: &str = "urn:oasis:names:specification:ubl:schema:xsd:CreditNote-2";
const CII_NS: &str = "urn:un:unece:uncefact:data:standard:CrossIndustryInvoice:100";
const VAT_ACTIVE_CATEGORIES: &[&str] = &["S", "Z", "E", "AE", "K", "G", "L", "M"];

/// Outcome of evaluating ANAF `CIUS-RO 1.0.1` rules and `cius-ro-id` checksums on an XML document.
#[derive(Debug, Clone)]
pub struct BrRoOutcome {
    /// Engine stage summary (`anaf_cius_ro`).
    pub report: EngineReport,
    /// Detected XML syntax binding when root element is recognized.
    pub syntax: Option<XmlSyntax>,
    /// Extracted `BT-24` Specification Identifier (`CustomizationID`).
    pub customization_id: Option<String>,
    /// Normative and checksum findings emitted.
    pub violations: Vec<Violation>,
}

struct RuleAcc {
    locale: Locale,
    evaluated: usize,
    failed: usize,
    warnings: usize,
    violations: Vec<Violation>,
}

impl RuleAcc {
    fn new(locale: Locale) -> Self {
        Self {
            locale,
            evaluated: 0,
            failed: 0,
            warnings: 0,
            violations: Vec::new(),
        }
    }

    fn pass(&mut self) {
        self.evaluated += 1;
    }

    #[allow(clippy::too_many_arguments)]
    fn push(
        &mut self,
        severity: Severity,
        code: &str,
        sub_id: &str,
        term: &str,
        loc: impl Into<String>,
        en: String,
        ro: String,
        expected: Option<&str>,
        actual: Option<String>,
        hint: Option<(String, String)>,
    ) {
        self.evaluated += 1;
        if severity == Severity::Fatal {
            self.failed += 1;
        } else {
            self.warnings += 1;
        }
        let (message, hint_msg) = match self.locale {
            Locale::En => (en, hint.map(|(e, _)| e)),
            Locale::Ro => (ro, hint.map(|(_, r)| r)),
        };
        self.violations.push(Violation {
            code: code.to_string(),
            rule_sub_id: Some(sub_id.to_string()),
            engine: ENGINE_ID,
            severity,
            business_term: Some(term.to_string()),
            message,
            location: Some(loc.into()),
            expected: expected.map(str::to_owned),
            actual,
            hint: hint_msg,
        });
    }

    fn fatal(
        &mut self,
        code: &str,
        sub_id: &str,
        term: &str,
        loc: impl Into<String>,
        en: String,
        ro: String,
    ) {
        self.push(
            Severity::Fatal,
            code,
            sub_id,
            term,
            loc,
            en,
            ro,
            None,
            None,
            None,
        );
    }

    fn into_report(self) -> (EngineReport, Vec<Violation>) {
        let eval = self.evaluated.max(1);
        (
            EngineReport {
                id: ENGINE_ID,
                name: ENGINE_NAME.to_string(),
                version: ENGINE_VERSION.to_string(),
                status: if self.failed == 0 {
                    EngineStatus::Passed
                } else {
                    EngineStatus::Failed
                },
                rules_evaluated: eval,
                passed_count: eval.saturating_sub(self.failed),
                failed_count: self.failed,
                warning_count: self.warnings,
                skip_reason: None,
            },
            self.violations,
        )
    }
}

/// Decodes UTF-8 (with optional BOM) or UTF-16BE / UTF-16LE XML bytes into a `String`.
pub fn decode_xml_bytes(bytes: &[u8]) -> Result<std::borrow::Cow<'_, str>, crate::types::XmlDecodeError> {
    use std::borrow::Cow;
    use crate::types::XmlDecodeError;

    if let Some(rest) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        return std::str::from_utf8(rest)
            .map(Cow::Borrowed)
            .map_err(XmlDecodeError::InvalidUtf8AfterBom);
    }
    for (bom, is_be, encoding) in [
        (&[0xFE, 0xFF][..], true, "UTF-16BE"),
        (&[0xFF, 0xFE][..], false, "UTF-16LE"),
    ] {
        if let Some(rest) = bytes.strip_prefix(bom) {
            if rest.len() % 2 != 0 {
                return Err(XmlDecodeError::TruncatedUtf16 { encoding });
            }
            let units = rest.chunks_exact(2).map(|c| {
                if is_be {
                    u16::from_be_bytes([c[0], c[1]])
                } else {
                    u16::from_le_bytes([c[0], c[1]])
                }
            });
            let decoded: String = char::decode_utf16(units)
                .collect::<Result<String, _>>()
                .map_err(|_| XmlDecodeError::InvalidUtf16 { encoding })?;
            return Ok(Cow::Owned(decoded));
        }
    }
    std::str::from_utf8(bytes)
        .map(Cow::Borrowed)
        .map_err(XmlDecodeError::InvalidUtf8)
}

/// Inspects a parsed XML root element to identify its [`XmlSyntax`] and `BT-24` specification ID.
#[must_use]
pub fn detect_root_syntax_and_bt24(root: Node<'_, '_>) -> (Option<XmlSyntax>, Option<String>) {
    let ns = root.tag_name().namespace().unwrap_or("");
    let local = root.tag_name().name();
    let syntax = if ns == UBL_INVOICE_NS || (ns.is_empty() && local == "Invoice") {
        Some(XmlSyntax::UblInvoice)
    } else if ns == UBL_CREDITNOTE_NS || (ns.is_empty() && local == "CreditNote") {
        Some(XmlSyntax::UblCreditNote)
    } else if ns == CII_NS || local == "CrossIndustryInvoice" {
        Some(XmlSyntax::CiiD16B)
    } else {
        None
    };

    let bt24 = match syntax {
        Some(XmlSyntax::UblInvoice | XmlSyntax::UblCreditNote) => {
            child_text(root, "CustomizationID")
        }
        Some(XmlSyntax::CiiD16B) => child_elem(root, "ExchangedDocumentContext")
            .and_then(|c| child_elem(c, "GuidelineSpecifiedDocumentContextParameter"))
            .and_then(|g| child_text(g, "ID")),
        None => None,
    };
    (syntax, bt24)
}

/// Evaluates all ANAF `CIUS-RO 1.0.1` (`BR-RO-*`) rules and `cius-ro-id` fiscal checksums on `xml_bytes`.
#[must_use]
pub fn evaluate_cius_ro_rules(xml_bytes: &[u8], opts: &ValidateOptions) -> BrRoOutcome {
    if !opts.run_cius_ro_rules && !opts.run_checksums {
        return skipped_outcome("Disabled by ValidateOptions", None, None);
    }
    let Ok(xml_str) = decode_xml_bytes(xml_bytes) else {
        return skipped_outcome("XML byte decoding failed", None, None);
    };
    let Ok(doc) = Document::parse(&xml_str) else {
        return skipped_outcome("XML is not well-formed", None, None);
    };
    evaluate_parsed(xml_bytes, &doc, None, opts)
}

pub(crate) fn skipped_for_unparsed(opts: &ValidateOptions) -> BrRoOutcome {
    let reason = if !opts.run_cius_ro_rules && !opts.run_checksums {
        "Disabled by ValidateOptions"
    } else {
        "XML is not well-formed or contains prohibited DTD"
    };
    skipped_outcome(reason, None, None)
}

pub(crate) fn evaluate_parsed(
    xml_bytes: &[u8],
    doc: &Document<'_>,
    pre_read: Option<Invoice>,
    opts: &ValidateOptions,
) -> BrRoOutcome {
    if !opts.run_cius_ro_rules && !opts.run_checksums {
        return skipped_outcome("Disabled by ValidateOptions", None, None);
    }
    let root = doc.root_element();
    let (syntax, customization_id) = detect_root_syntax_and_bt24(root);
    let Some(syntax_kind) = syntax else {
        return skipped_outcome("Unrecognized XML root element", None, customization_id);
    };

    let inv = match pre_read.or_else(|| en16931_formats::read(xml_bytes).ok().map(|r| r.invoice)) {
        Some(i) => i,
        None => {
            return skipped_outcome(
                "EN 16931 reader could not load document",
                syntax,
                customization_id,
            )
        }
    };

    let mut acc = RuleAcc::new(opts.locale);

    if opts.run_cius_ro_rules {
        let spec = customization_id
            .as_deref()
            .or(inv.specification_id.as_deref())
            .unwrap_or("");
        let low = spec.to_ascii_lowercase();
        let mentions_ro = low.contains("cius-ro") || low.contains("efactura.mfinante.ro");
        if spec == CIUS_RO_CUSTOMIZATION_ID {
            acc.pass();
        } else if opts.strict_cius_ro_customization || mentions_ro {
            acc.push(
                Severity::Fatal,
                "BR-RO-001",
                "BR-RO-001",
                "BT-24",
                "BT-24",
                format!("Specification identifier (BT-24) must equal '{CIUS_RO_CUSTOMIZATION_ID}' (found '{spec}')."),
                format!("Identificatorul specificației (BT-24) trebuie să corespundă exact valorii '{CIUS_RO_CUSTOMIZATION_ID}' (găsit: '{spec}')."),
                Some(CIUS_RO_CUSTOMIZATION_ID),
                Some(spec.to_string()),
                Some((
                    format!("Set CustomizationID (BT-24) to '{CIUS_RO_CUSTOMIZATION_ID}'."),
                    format!("Setați CustomizationID (BT-24) la '{CIUS_RO_CUSTOMIZATION_ID}'."),
                )),
            );
        } else {
            acc.push(
                Severity::Warning,
                "BR-RO-001",
                "BR-RO-001",
                "BT-24",
                "BT-24",
                format!("Domestic Romanian invoice does not declare ANAF CIUS-RO 1.0.1 CustomizationID '{CIUS_RO_CUSTOMIZATION_ID}'."),
                format!("Factura românească nu declară identificatorul ANAF CIUS-RO 1.0.1 '{CIUS_RO_CUSTOMIZATION_ID}'."),
                Some(CIUS_RO_CUSTOMIZATION_ID),
                Some(spec.to_string()),
                None,
            );
            let (report, violations) = acc.into_report();
            return BrRoOutcome {
                report,
                syntax,
                customization_id,
                violations,
            };
        }

        check_core_rules(&inv, root, syntax_kind, opts, &mut acc);
        check_dates_lexical(root, syntax_kind, &mut acc);
        check_cardinality(&inv, syntax_kind, &mut acc);
        check_lengths(&inv, syntax_kind, &mut acc);
        check_decimals_lexical(root, syntax_kind, &inv, &mut acc);
        check_parties_and_addresses(&inv, &mut acc);
    }

    if opts.run_checksums {
        check_all_checksums(&inv, &mut acc);
    }

    let (report, violations) = acc.into_report();
    BrRoOutcome {
        report,
        syntax,
        customization_id,
        violations,
    }
}

fn skipped_outcome(
    reason: &str,
    syntax: Option<XmlSyntax>,
    customization_id: Option<String>,
) -> BrRoOutcome {
    BrRoOutcome {
        report: EngineReport::skipped(ENGINE_ID, ENGINE_NAME, ENGINE_VERSION, reason),
        syntax,
        customization_id,
        violations: Vec::new(),
    }
}

fn check_core_rules(
    inv: &Invoice,
    root: Node<'_, '_>,
    syntax: XmlSyntax,
    opts: &ValidateOptions,
    acc: &mut RuleAcc,
) {
    // BR-RO-010: BT-1 must contain at least one ASCII digit
    let num = inv.number.as_deref().unwrap_or("");
    if num.bytes().any(|b| b.is_ascii_digit()) {
        acc.pass();
    } else {
        acc.fatal(
            "BR-RO-010",
            "BR-RO-010",
            "BT-1",
            "BT-1",
            format!("Invoice number (BT-1) '{num}' must contain at least one numeric digit (0-9)."),
            format!("Numărul facturii (BT-1) '{num}' trebuie să conțină cel puțin o cifră (0-9)."),
        );
    }

    // BR-RO-020: BT-3 allowed type codes
    let tc = inv.type_code.as_ref().map_or("", |c| c.as_str().trim());
    let tc_ok = match syntax {
        XmlSyntax::UblInvoice => matches!(tc, "380" | "384" | "389" | "751"),
        XmlSyntax::UblCreditNote => tc == "381",
        XmlSyntax::CiiD16B => matches!(tc, "380" | "381" | "384" | "389" | "751"),
    };
    if tc_ok {
        acc.pass();
    } else {
        acc.fatal(
            "BR-RO-020",
            "BR-RO-020",
            "BT-3",
            "BT-3",
            format!("Invoice type code (BT-3) '{tc}' is not allowed in CIUS-RO (permitted: 380, 384, 389, 751, or 381 for CreditNote)."),
            format!("Codul tipului de factură (BT-3) '{tc}' nu este permis în CIUS-RO (valori permise: 380, 384, 389, 751 sau 381 pentru CreditNote)."),
        );
    }

    // BR-RO-030: Foreign currency invoice from RO Seller must state RON VAT accounting currency (BT-6 & BT-111)
    let seller_ro = addr_country(&inv.seller.address) == "RO";
    let doc_ccy = inv.currency.as_ref().map_or("", |c| c.as_str().trim());
    if seller_ro && !doc_ccy.is_empty() && doc_ccy != "RON" {
        let tax_ccy_ron = inv
            .vat_accounting_currency
            .as_ref()
            .is_some_and(|c| c.as_str().trim() == "RON");
        let has_ron_vat =
            inv.totals.vat_total_accounting.is_some() || has_ron_tax_amount_in_xml(root, syntax);
        if tax_ccy_ron && has_ron_vat {
            acc.pass();
        } else {
            acc.fatal(
                "BR-RO-030",
                "BR-RO-030",
                "BT-5 / BT-6 / BT-111",
                "BT-6 / BT-111",
                format!("When Seller is in RO and DocumentCurrencyCode (BT-5) is '{doc_ccy}' (!= RON), TaxCurrencyCode (BT-6) must be 'RON' and VAT total in accounting currency (BT-111) must be provided."),
                format!("Dacă Vânzătorul este din RO și moneda facturii (BT-5) este '{doc_ccy}' (!= RON), moneda de contabilizare a TVA (BT-6) trebuie să fie 'RON' și valoarea totală a TVA în RON (BT-111) este obligatorie."),
            );
        }
    }

    // BR-RO-040: BT-8 VAT point date code in {3, 35, 432}
    if let Some(code) = inv
        .vat_point_date_code
        .as_ref()
        .map(|c| c.as_str().trim())
        .filter(|s| !s.is_empty())
    {
        check_bt8_code(code, "BT-8", acc);
    }
    if syntax == XmlSyntax::CiiD16B {
        for node in root
            .descendants()
            .filter(|n| n.is_element() && n.tag_name().name() == "TimeReferenceCode")
        {
            let code = norm_space(node.text().unwrap_or(""));
            if !code.is_empty() {
                check_bt8_code(&code, "ram:ApplicableTradeTax/ram:TimeReferenceCode", acc);
            }
        }
    }

    // BR-27 fallback when EN 16931 engine is disabled (no duplicate when run_en16931 is true)
    if !opts.run_en16931 {
        for line in &inv.lines {
            if line.price.net_price.into_decimal().is_sign_negative() {
                acc.fatal(
                    "BR-27",
                    "BR-27",
                    "BT-146",
                    "BG-25/BT-146",
                    "The Item net price (BT-146) shall NOT be negative.".to_string(),
                    "Prețul net al articolului (BT-146) NU poate fi negativ.".to_string(),
                );
            } else {
                acc.pass();
            }
        }
    }
}

fn check_bt8_code(code: &str, loc: &str, acc: &mut RuleAcc) {
    if matches!(code, "3" | "35" | "432") {
        acc.pass();
    } else {
        acc.fatal(
            "BR-RO-040",
            "BR-RO-040",
            "BT-8",
            loc,
            format!("Value added tax point date code (BT-8) '{code}' must be one of 3, 35, or 432."),
            format!("Codul datei exigibilității TVA (BT-8) '{code}' trebuie să fie unul dintre: 3, 35 sau 432."),
        );
    }
}

fn has_ron_tax_amount_in_xml(root: Node<'_, '_>, syntax: XmlSyntax) -> bool {
    let tag = if syntax == XmlSyntax::CiiD16B {
        "TaxTotalAmount"
    } else {
        "TaxAmount"
    };
    root.descendants().any(|n| {
        n.is_element()
            && n.tag_name().name() == tag
            && n.attribute("currencyID").is_some_and(|c| c.trim() == "RON")
    })
}

fn check_dates_lexical(root: Node<'_, '_>, syntax: XmlSyntax, acc: &mut RuleAcc) {
    if matches!(syntax, XmlSyntax::UblInvoice | XmlSyntax::UblCreditNote) {
        for node in root.descendants().filter(Node::is_element) {
            let meta = match node.tag_name().name() {
                "IssueDate" => Some(("BR-RO-DT001", "BR-RO-DT001", "BT-2")),
                "TaxPointDate" => Some(("BR-RO-DT002", "BR-RO-DT002", "BT-7")),
                "DueDate" => Some(("BR-RO-DT003", "BR-RO-DT003", "BT-9")),
                "PaymentDueDate" => Some(("BR-RO-DT003-CN", "BR-RO-DT003_CN", "BT-9")),
                "ActualDeliveryDate" => Some(("BR-RO-DT004", "BR-RO-DT004", "BT-72")),
                "StartDate" => Some(("BR-RO-DT005", "BR-RO-DT005", "BT-73 / BT-134")),
                "EndDate" => Some(("BR-RO-DT006", "BR-RO-DT006", "BT-74 / BT-135")),
                _ => None,
            };
            if let Some((code, sub, bt)) = meta {
                let raw = node.text().unwrap_or("").trim();
                if raw.len() == 10 && Date::parse(raw).is_ok() {
                    acc.pass();
                } else {
                    acc.fatal(
                        code,
                        sub,
                        bt,
                        format!("cbc:{}", node.tag_name().name()),
                        format!("Date ({bt}) '{raw}' must strictly follow YYYY-MM-DD without timezone offset."),
                        format!("Data ({bt}) '{raw}' trebuie să respecte strict formatul AAAA-LL-ZZ fără fus orar."),
                    );
                }
            }
        }
    } else {
        for dt in root
            .descendants()
            .filter(|n| n.is_element() && n.tag_name().name() == "DateTimeString")
        {
            let p1 = dt.parent().map(|n| n.tag_name().name()).unwrap_or("");
            let p2 = dt
                .parent()
                .and_then(|p| p.parent())
                .map(|n| n.tag_name().name())
                .unwrap_or("");
            let meta = match (p2, p1) {
                (_, "IssueDateTime") => Some(("BR-RO-DT001", "BT-2")),
                ("ActualDeliverySupplyChainEvent", "OccurrenceDateTime") => {
                    Some(("BR-RO-DT004", "BT-72"))
                }
                ("TaxPointDate" | "ActualTaxPointDate", _)
                | (_, "TaxPointDate" | "ActualTaxPointDate") => Some(("BR-RO-DT002", "BT-7")),
                ("SpecifiedTradePaymentTerms", "DueDateDateTime") => Some(("BR-RO-DT003", "BT-9")),
                ("BillingSpecifiedPeriod", "StartDateTime") => {
                    Some(("BR-RO-DT005", "BT-73 / BT-134"))
                }
                ("BillingSpecifiedPeriod", "EndDateTime") => {
                    Some(("BR-RO-DT006", "BT-74 / BT-135"))
                }
                _ => None,
            };
            if let Some((code, bt)) = meta {
                let fmt = dt.attribute("format").unwrap_or("102").trim();
                let raw = dt.text().unwrap_or("").trim();
                let ok = fmt == "102"
                    && raw.len() == 8
                    && raw.bytes().all(|b| b.is_ascii_digit())
                    && Date::parse(&format!("{}-{}-{}", &raw[0..4], &raw[4..6], &raw[6..8]))
                        .is_ok();
                if ok {
                    acc.pass();
                } else {
                    acc.fatal(
                        code,
                        code,
                        bt,
                        format!("ram:{p1}/udt:DateTimeString"),
                        format!("CII Date ({bt}) '{raw}' (format='{fmt}') must use format='102' and valid YYYYMMDD."),
                        format!("Data CII ({bt}) '{raw}' (format='{fmt}') trebuie să aibă format='102' și o dată validă AAAALLZZ."),
                    );
                }
            }
        }
    }
}

fn check_cardinality(inv: &Invoice, syntax: XmlSyntax, acc: &mut RuleAcc) {
    let check_cap = |count: usize,
                     max: usize,
                     code: &str,
                     sub: &str,
                     bg: &str,
                     en: &str,
                     ro: &str,
                     acc: &mut RuleAcc| {
        if count <= max {
            acc.pass();
        } else {
            acc.fatal(
                code,
                sub,
                bg,
                bg,
                format!("Maximum {max} {en} ({bg}) are allowed in CIUS-RO (found {count})."),
                format!(
                    "În CIUS-RO sunt permise maximum {max} elemente {ro} ({bg}) (găsit: {count})."
                ),
            );
        }
    };

    check_cap(
        inv.notes.len(),
        20,
        "BR-RO-A020",
        "BR-RO-A020",
        "BG-1",
        "Invoice notes",
        "Note în factură",
        acc,
    );
    let doc_refs = inv.attachments.len()
        + usize::from(
            matches!(syntax, XmlSyntax::UblInvoice | XmlSyntax::UblCreditNote)
                && inv.object_identifier.is_some(),
        );
    check_cap(
        doc_refs,
        50,
        "BR-RO-A050",
        "BR-RO-A051",
        "BG-24",
        "Supporting documents",
        "Documente justificative",
        acc,
    );
    for line in &inv.lines {
        check_cap(
            line.item.attributes.len(),
            50,
            "BR-RO-A050",
            "BR-RO-A052",
            "BG-32",
            "Item attributes",
            "Atribute ale articolului",
            acc,
        );
    }
    check_cap(
        inv.preceding_invoices.len(),
        500,
        "BR-RO-A500",
        "BR-RO-A500",
        "BG-3",
        "Preceding invoice references",
        "Referințe la facturi anterioare",
        acc,
    );
}

fn check_lengths(inv: &Invoice, syntax: XmlSyntax, acc: &mut RuleAcc) {
    let mut chk = |val: Option<&str>, max: usize, code: &str, sub: &str, bt: &str| {
        if let Some(raw) = val {
            let len = norm_space(raw).chars().count();
            if len <= max {
                acc.pass();
            } else {
                acc.fatal(
                    code,
                    sub,
                    bt,
                    bt,
                    format!("Field {bt} exceeds maximum normalized length of {max} characters (found {len})."),
                    format!("Câmpul {bt} depășește lungimea maximă permisă de {max} caractere (găsit: {len})."),
                );
            }
        }
    };

    chk(
        inv.accounting_reference.as_deref(),
        200,
        "BR-RO-L200",
        "BR-RO-L155",
        "BT-19",
    );
    chk(
        inv.buyer_reference.as_deref(),
        100,
        "BR-RO-L100",
        "BR-RO-L1001",
        "BT-10",
    );
    chk(
        inv.project_reference.as_ref().map(|r| r.as_str()),
        200,
        "BR-RO-L200",
        "BR-RO-L0302",
        "BT-11",
    );
    chk(
        inv.contract_reference.as_ref().map(|r| r.as_str()),
        200,
        "BR-RO-L200",
        "BR-RO-L0303",
        "BT-12",
    );
    chk(
        inv.purchase_order_reference.as_ref().map(|r| r.as_str()),
        200,
        "BR-RO-L200",
        "BR-RO-L0304",
        "BT-13",
    );
    chk(
        inv.sales_order_reference.as_ref().map(|r| r.as_str()),
        200,
        "BR-RO-L200",
        "BR-RO-L0305",
        "BT-14",
    );
    chk(
        inv.despatch_advice_reference.as_ref().map(|r| r.as_str()),
        200,
        "BR-RO-L200",
        "BR-RO-L0306",
        "BT-16",
    );
    chk(
        inv.receiving_advice_reference.as_ref().map(|r| r.as_str()),
        200,
        "BR-RO-L200",
        "BR-RO-L0307",
        "BT-15",
    );
    chk(
        inv.tender_reference.as_ref().map(|r| r.as_str()),
        200,
        "BR-RO-L200",
        "BR-RO-L0308",
        "BT-17",
    );
    chk(
        inv.object_identifier.as_ref().map(|r| r.content()),
        200,
        "BR-RO-L200",
        "BR-RO-L0308",
        "BT-18",
    );
    chk(
        inv.payment_terms.as_deref(),
        300,
        "BR-RO-L300",
        "BR-RO-L302",
        "BT-20",
    );

    for n in &inv.notes {
        let reconstructed;
        let s = match (&n.subject_code, &n.note, syntax) {
            (Some(sc), Some(body), XmlSyntax::UblInvoice | XmlSyntax::UblCreditNote) => {
                reconstructed = format!("#{}#{body}", sc.as_str());
                Some(reconstructed.as_str())
            }
            (_, body, _) => body.as_deref(),
        };
        chk(s, 300, "BR-RO-L300", "BR-RO-L301", "BT-22");
    }

    // Seller (BG-4 / BG-5 / BG-6)
    chk(
        inv.seller.address.line1.as_deref(),
        150,
        "BR-RO-L150",
        "BR-RO-L151",
        "BT-35",
    );
    chk(
        inv.seller.address.line2.as_deref(),
        100,
        "BR-RO-L100",
        "BR-RO-L1002",
        "BT-36",
    );
    chk(
        inv.seller.address.line3.as_deref(),
        100,
        "BR-RO-L100",
        "BR-RO-L1003",
        "BT-162",
    );
    chk(
        inv.seller.address.city.as_deref(),
        50,
        "BR-RO-L050",
        "BR-RO-L0501",
        "BT-37",
    );
    chk(
        inv.seller.address.post_code.as_deref(),
        20,
        "BR-RO-L020",
        "BR-RO-L0201",
        "BT-38",
    );
    chk(
        inv.seller.contact.name.as_deref(),
        100,
        "BR-RO-L100",
        "BR-RO-L1004",
        "BT-41",
    );
    chk(
        inv.seller.contact.phone.as_deref(),
        100,
        "BR-RO-L100",
        "BR-RO-L1005",
        "BT-42",
    );
    chk(
        inv.seller.contact.email.as_deref(),
        100,
        "BR-RO-L100",
        "BR-RO-L1006",
        "BT-43",
    );
    chk(
        inv.seller.trading_name.as_deref(),
        200,
        "BR-RO-L200",
        "BR-RO-L202",
        "BT-28",
    );
    chk(
        inv.seller.name.as_deref(),
        200,
        "BR-RO-L200",
        "BR-RO-L201",
        "BT-27",
    );
    chk(
        inv.seller.additional_legal_information.as_deref(),
        1000,
        "BR-RO-L1000",
        "BR-RO-L1000",
        "BT-33",
    );

    // Buyer (BG-7 / BG-8 / BG-9)
    chk(
        inv.buyer.address.line1.as_deref(),
        150,
        "BR-RO-L150",
        "BR-RO-L152",
        "BT-50",
    );
    chk(
        inv.buyer.address.line2.as_deref(),
        100,
        "BR-RO-L100",
        "BR-RO-L1007",
        "BT-51",
    );
    chk(
        inv.buyer.address.line3.as_deref(),
        100,
        "BR-RO-L100",
        "BR-RO-L1008",
        "BT-163",
    );
    chk(
        inv.buyer.address.city.as_deref(),
        50,
        "BR-RO-L050",
        "BR-RO-L0502",
        "BT-52",
    );
    chk(
        inv.buyer.address.post_code.as_deref(),
        20,
        "BR-RO-L020",
        "BR-RO-L0202",
        "BT-53",
    );
    chk(
        inv.buyer.contact.name.as_deref(),
        100,
        "BR-RO-L100",
        "BR-RO-L1009",
        "BT-56",
    );
    chk(
        inv.buyer.contact.phone.as_deref(),
        100,
        "BR-RO-L100",
        "BR-RO-L1010",
        "BT-57",
    );
    chk(
        inv.buyer.contact.email.as_deref(),
        100,
        "BR-RO-L100",
        "BR-RO-L1011",
        "BT-58",
    );
    chk(
        inv.buyer.trading_name.as_deref(),
        200,
        "BR-RO-L200",
        "BR-RO-L204",
        "BT-45",
    );
    chk(
        inv.buyer.name.as_deref(),
        200,
        "BR-RO-L200",
        "BR-RO-L203",
        "BT-44",
    );

    // Payee (BG-10), TaxRepresentative (BG-11/12), Delivery (BG-13/15)
    if let Some(p) = &inv.payee {
        chk(p.name.as_deref(), 200, "BR-RO-L200", "BR-RO-L205", "BT-59");
    }
    if let Some(tr) = &inv.tax_representative {
        chk(tr.name.as_deref(), 200, "BR-RO-L200", "BR-RO-L206", "BT-62");
        chk(
            tr.address.line1.as_deref(),
            150,
            "BR-RO-L150",
            "BR-RO-L153",
            "BT-64",
        );
        chk(
            tr.address.line2.as_deref(),
            100,
            "BR-RO-L100",
            "BR-RO-L1012",
            "BT-65",
        );
        chk(
            tr.address.line3.as_deref(),
            100,
            "BR-RO-L100",
            "BR-RO-L1013",
            "BT-164",
        );
        chk(
            tr.address.city.as_deref(),
            50,
            "BR-RO-L050",
            "BR-RO-L0503",
            "BT-66",
        );
        chk(
            tr.address.post_code.as_deref(),
            20,
            "BR-RO-L020",
            "BR-RO-L0203",
            "BT-67",
        );
    }
    if let Some(del) = &inv.delivery {
        chk(
            del.party_name.as_deref(),
            200,
            "BR-RO-L200",
            "BR-RO-L207",
            "BT-70",
        );
        if let Some(a) = &del.address {
            chk(a.line1.as_deref(), 150, "BR-RO-L150", "BR-RO-L154", "BT-75");
            chk(
                a.line2.as_deref(),
                100,
                "BR-RO-L100",
                "BR-RO-L1014",
                "BT-76",
            );
            chk(
                a.line3.as_deref(),
                100,
                "BR-RO-L100",
                "BR-RO-L1015",
                "BT-165",
            );
            chk(a.city.as_deref(), 50, "BR-RO-L050", "BR-RO-L0504", "BT-77");
            chk(
                a.post_code.as_deref(),
                20,
                "BR-RO-L020",
                "BR-RO-L0204",
                "BT-78",
            );
        }
    }

    // Payment (BG-16..18)
    if let Some(pay) = &inv.payment {
        chk(
            pay.means_text.as_deref(),
            100,
            "BR-RO-L100",
            "BR-RO-L1016",
            "BT-82",
        );
        chk(
            pay.remittance_information.as_deref(),
            140,
            "BR-RO-L140",
            "BR-RO-L140",
            "BT-83",
        );
        match &pay.means {
            Some(PaymentMeans::CreditTransfer(cts)) => {
                for ct in cts {
                    chk(
                        ct.account_name.as_deref(),
                        200,
                        "BR-RO-L200",
                        "BR-RO-L208",
                        "BT-85",
                    );
                }
            }
            Some(PaymentMeans::Card(card)) => {
                chk(
                    card.holder_name.as_deref(),
                    200,
                    "BR-RO-L200",
                    "BR-RO-L209",
                    "BT-88",
                );
            }
            _ => {}
        }
    }

    // Allowances, Charges, VAT Breakdown, Supporting Documents, Preceding Invoices
    for a in &inv.allowances {
        chk(
            a.reason.as_deref(),
            100,
            "BR-RO-L100",
            "BR-RO-L1017",
            "BT-97",
        );
    }
    for c in &inv.charges {
        chk(
            c.reason.as_deref(),
            100,
            "BR-RO-L100",
            "BR-RO-L1018",
            "BT-104",
        );
    }
    for vb in &inv.vat_breakdown {
        chk(
            vb.exemption_reason.as_deref(),
            100,
            "BR-RO-L100",
            "BR-RO-L1019",
            "BT-120",
        );
    }
    for att in &inv.attachments {
        chk(
            Some(att.reference.as_str()),
            200,
            "BR-RO-L200",
            "BR-RO-L0308",
            "BT-122",
        );
        chk(
            att.description.as_deref(),
            100,
            "BR-RO-L100",
            "BR-RO-L1020",
            "BT-123",
        );
        chk(
            att.uri.as_deref(),
            200,
            "BR-RO-L200",
            "BR-RO-L210",
            "BT-124",
        );
        if let Some(bin) = &att.attachment {
            chk(
                Some(bin.filename()),
                200,
                "BR-RO-L200",
                "BR-RO-L211",
                "BT-125-2",
            );
        }
    }
    for pi in &inv.preceding_invoices {
        chk(
            Some(pi.reference.as_str()),
            200,
            "BR-RO-L200",
            "BR-RO-L156",
            "BT-25",
        );
    }

    // Invoice Lines (BG-25..32)
    for line in &inv.lines {
        chk(
            line.accounting_reference.as_deref(),
            100,
            "BR-RO-L100",
            "BR-RO-L1021",
            "BT-133",
        );
        chk(
            line.note.as_deref(),
            300,
            "BR-RO-L300",
            "BR-RO-L303",
            "BT-127",
        );
        for a in &line.allowances {
            chk(
                a.reason.as_deref(),
                100,
                "BR-RO-L100",
                "BR-RO-L1022",
                "BT-139",
            );
        }
        for c in &line.charges {
            chk(
                c.reason.as_deref(),
                100,
                "BR-RO-L100",
                "BR-RO-L1023",
                "BT-144",
            );
        }
        chk(
            line.item.name.as_deref(),
            100,
            "BR-RO-L100",
            "BR-RO-L1024",
            "BT-153",
        );
        chk(
            line.item.description.as_deref(),
            200,
            "BR-RO-L200",
            "BR-RO-L212",
            "BT-154",
        );
        for attr in &line.item.attributes {
            chk(
                attr.name.as_deref(),
                50,
                "BR-RO-L050",
                "BR-RO-L0505",
                "BT-160",
            );
            chk(
                attr.value.as_deref(),
                100,
                "BR-RO-L100",
                "BR-RO-L1025",
                "BT-161",
            );
        }
    }
}

fn check_decimals_lexical(root: Node<'_, '_>, syntax: XmlSyntax, inv: &Invoice, acc: &mut RuleAcc) {
    let mut chk = |node: Option<Node<'_, '_>>, sub: &str, bt: &str, loc: &str| {
        if let Some(el) = node {
            let raw = el.text().unwrap_or("").trim();
            if !raw.is_empty() {
                if count_decimals(raw) <= 2 {
                    acc.pass();
                } else {
                    acc.fatal(
                        "BR-RO-Z2",
                        sub,
                        bt,
                        loc,
                        format!("Monetary amount ({bt}) '{raw}' exceeds maximum of 2 decimal digits allowed by CIUS-RO ({sub})."),
                        format!("Suma monetară ({bt}) '{raw}' depășește numărul maxim de 2 zecimale permis de CIUS-RO ({sub})."),
                    );
                }
            }
        }
    };

    let tax_ccy = inv
        .vat_accounting_currency
        .as_ref()
        .map(|c| c.as_str().trim());
    let doc_ccy = inv.currency.as_ref().map(|c| c.as_str().trim());

    if matches!(syntax, XmlSyntax::UblInvoice | XmlSyntax::UblCreditNote) {
        let is_cn = syntax == XmlSyntax::UblCreditNote;
        for ac in children_named(root, "AllowanceCharge") {
            let is_charge = is_xsd_true(child_text(ac, "ChargeIndicator").as_deref());
            chk(
                child_elem(ac, "Amount"),
                if is_charge {
                    "BR-DEC-RO-05"
                } else {
                    "BR-DEC-RO-01"
                },
                if is_charge { "BT-99" } else { "BT-92" },
                "cac:AllowanceCharge/cbc:Amount",
            );
            chk(
                child_elem(ac, "BaseAmount"),
                if is_charge {
                    "BR-DEC-RO-06"
                } else {
                    "BR-DEC-RO-02"
                },
                if is_charge { "BT-100" } else { "BT-93" },
                "cac:AllowanceCharge/cbc:BaseAmount",
            );
        }
        for tt in children_named(root, "TaxTotal") {
            if let Some(ta) = child_elem(tt, "TaxAmount") {
                let ccy = ta.attribute("currencyID").map(str::trim);
                let is_acct = ccy.is_some() && ccy == tax_ccy && tax_ccy != doc_ccy;
                chk(
                    Some(ta),
                    if is_acct {
                        "BR-DEC-RO-15"
                    } else {
                        "BR-DEC-RO-13"
                    },
                    if is_acct { "BT-111" } else { "BT-110" },
                    "cac:TaxTotal/cbc:TaxAmount",
                );
            }
            for sub in children_named(tt, "TaxSubtotal") {
                chk(
                    child_elem(sub, "TaxableAmount"),
                    "BR-DEC-RO-1009",
                    "BT-116",
                    "cac:TaxSubtotal/cbc:TaxableAmount",
                );
                chk(
                    child_elem(sub, "TaxAmount"),
                    "BR-DEC-RO-1010",
                    "BT-117",
                    "cac:TaxSubtotal/cbc:TaxAmount",
                );
            }
        }
        if let Some(lmt) = child_elem(root, "LegalMonetaryTotal") {
            for (tag, sub, bt) in [
                ("LineExtensionAmount", "BR-DEC-RO-09", "BT-106"),
                ("AllowanceTotalAmount", "BR-DEC-RO-10", "BT-107"),
                ("ChargeTotalAmount", "BR-DEC-RO-11", "BT-108"),
                ("TaxExclusiveAmount", "BR-DEC-RO-12", "BT-109"),
                ("TaxInclusiveAmount", "BR-DEC-RO-14", "BT-112"),
                ("PrepaidAmount", "BR-DEC-RO-16", "BT-113"),
                ("PayableRoundingAmount", "BR-DEC-RO-17", "BT-114"),
                ("PayableAmount", "BR-DEC-RO-18", "BT-115"),
            ] {
                chk(
                    child_elem(lmt, tag),
                    sub,
                    bt,
                    &format!("cac:LegalMonetaryTotal/cbc:{tag}"),
                );
            }
        }
        let line_tag = if is_cn {
            "CreditNoteLine"
        } else {
            "InvoiceLine"
        };
        let line_loc = format!("cac:{line_tag}/cbc:LineExtensionAmount");
        let ac_amt_loc = format!("cac:{line_tag}/cac:AllowanceCharge/cbc:Amount");
        let ac_base_loc = format!("cac:{line_tag}/cac:AllowanceCharge/cbc:BaseAmount");
        for line in children_named(root, line_tag) {
            chk(
                child_elem(line, "LineExtensionAmount"),
                "BR-DEC-RO-23",
                "BT-131",
                &line_loc,
            );
            for ac in children_named(line, "AllowanceCharge") {
                let is_charge = is_xsd_true(child_text(ac, "ChargeIndicator").as_deref());
                chk(
                    child_elem(ac, "Amount"),
                    if is_charge {
                        "BR-DEC-RO-27"
                    } else {
                        "BR-DEC-RO-24"
                    },
                    if is_charge { "BT-141" } else { "BT-136" },
                    &ac_amt_loc,
                );
                chk(
                    child_elem(ac, "BaseAmount"),
                    if is_charge {
                        "BR-DEC-RO-28"
                    } else {
                        "BR-DEC-RO-25"
                    },
                    if is_charge { "BT-142" } else { "BT-137" },
                    &ac_base_loc,
                );
            }
        }
    } else if let Some(tx) = child_elem(root, "SupplyChainTradeTransaction") {
        if let Some(hs) = child_elem(tx, "ApplicableHeaderTradeSettlement") {
            for ac in children_named(hs, "SpecifiedTradeAllowanceCharge") {
                let ind =
                    child_elem(ac, "ChargeIndicator").and_then(|ci| child_text(ci, "Indicator"));
                let is_charge = is_xsd_true(ind.as_deref());
                chk(
                    child_elem(ac, "ActualAmount"),
                    if is_charge {
                        "BR-DEC-RO-05"
                    } else {
                        "BR-DEC-RO-01"
                    },
                    if is_charge { "BT-99" } else { "BT-92" },
                    "ram:SpecifiedTradeAllowanceCharge/ram:ActualAmount",
                );
                chk(
                    child_elem(ac, "BasisAmount"),
                    if is_charge {
                        "BR-DEC-RO-06"
                    } else {
                        "BR-DEC-RO-02"
                    },
                    if is_charge { "BT-100" } else { "BT-93" },
                    "ram:SpecifiedTradeAllowanceCharge/ram:BasisAmount",
                );
            }
            for tax in children_named(hs, "ApplicableTradeTax") {
                chk(
                    child_elem(tax, "BasisAmount"),
                    "BR-DEC-RO-1009",
                    "BT-116",
                    "ram:ApplicableTradeTax/ram:BasisAmount",
                );
                chk(
                    child_elem(tax, "CalculatedAmount"),
                    "BR-DEC-RO-1010",
                    "BT-117",
                    "ram:ApplicableTradeTax/ram:CalculatedAmount",
                );
            }
            if let Some(ms) = child_elem(hs, "SpecifiedTradeSettlementHeaderMonetarySummation") {
                for (tag, sub, bt) in [
                    ("LineTotalAmount", "BR-DEC-RO-09", "BT-106"),
                    ("AllowanceTotalAmount", "BR-DEC-RO-10", "BT-107"),
                    ("ChargeTotalAmount", "BR-DEC-RO-11", "BT-108"),
                    ("TaxBasisTotalAmount", "BR-DEC-RO-12", "BT-109"),
                    ("GrandTotalAmount", "BR-DEC-RO-14", "BT-112"),
                    ("TotalPrepaidAmount", "BR-DEC-RO-16", "BT-113"),
                    ("RoundingAmount", "BR-DEC-RO-17", "BT-114"),
                    ("DuePayableAmount", "BR-DEC-RO-18", "BT-115"),
                ] {
                    chk(
                        child_elem(ms, tag),
                        sub,
                        bt,
                        &format!("ram:SpecifiedTradeSettlementHeaderMonetarySummation/ram:{tag}"),
                    );
                }
                for tta in children_named(ms, "TaxTotalAmount") {
                    let ccy = tta.attribute("currencyID").map(str::trim);
                    let is_acct = ccy.is_some() && ccy == tax_ccy && tax_ccy != doc_ccy;
                    chk(
                        Some(tta),
                        if is_acct {
                            "BR-DEC-RO-15"
                        } else {
                            "BR-DEC-RO-13"
                        },
                        if is_acct { "BT-111" } else { "BT-110" },
                        "ram:SpecifiedTradeSettlementHeaderMonetarySummation/ram:TaxTotalAmount",
                    );
                }
            }
        }
        for item in children_named(tx, "IncludedSupplyChainTradeLineItem") {
            if let Some(ls) = child_elem(item, "SpecifiedLineTradeSettlement") {
                if let Some(lms) = child_elem(ls, "SpecifiedTradeSettlementLineMonetarySummation") {
                    chk(
                        child_elem(lms, "LineTotalAmount"),
                        "BR-DEC-RO-23",
                        "BT-131",
                        "ram:SpecifiedTradeSettlementLineMonetarySummation/ram:LineTotalAmount",
                    );
                }
                for ac in children_named(ls, "SpecifiedTradeAllowanceCharge") {
                    let ind = child_elem(ac, "ChargeIndicator")
                        .and_then(|ci| child_text(ci, "Indicator"));
                    let is_charge = is_xsd_true(ind.as_deref());
                    chk(child_elem(ac, "ActualAmount"), if is_charge { "BR-DEC-RO-27" } else { "BR-DEC-RO-24" }, if is_charge { "BT-141" } else { "BT-136" }, "ram:SpecifiedLineTradeSettlement/ram:SpecifiedTradeAllowanceCharge/ram:ActualAmount");
                    chk(child_elem(ac, "BasisAmount"), if is_charge { "BR-DEC-RO-28" } else { "BR-DEC-RO-25" }, if is_charge { "BT-142" } else { "BT-137" }, "ram:SpecifiedLineTradeSettlement/ram:SpecifiedTradeAllowanceCharge/ram:BasisAmount");
                }
            }
        }
    }
}

fn check_parties_and_addresses(inv: &Invoice, acc: &mut RuleAcc) {
    let uses_active_vat = inv.lines.is_empty()
        || inv
            .allowances
            .iter()
            .any(|a| VAT_ACTIVE_CATEGORIES.contains(&a.vat.category.as_str()))
        || inv
            .charges
            .iter()
            .any(|c| VAT_ACTIVE_CATEGORIES.contains(&c.vat.category.as_str()))
        || inv
            .vat_breakdown
            .iter()
            .any(|v| VAT_ACTIVE_CATEGORIES.contains(&v.category.as_str()))
        || inv
            .lines
            .iter()
            .any(|l| VAT_ACTIVE_CATEGORIES.contains(&l.vat.category.as_str()));

    if uses_active_vat {
        let has_seller_tax = non_empty(inv.seller.vat_identifier.as_deref())
            || non_empty(inv.seller.tax_registration.as_deref())
            || inv
                .tax_representative
                .as_ref()
                .is_some_and(|tr| non_empty(tr.vat_identifier.as_deref()));
        if has_seller_tax {
            acc.pass();
        } else {
            acc.fatal(
                "BR-RO-065",
                "BR-RO-065",
                "BT-31 / BT-32 / BT-63",
                "BG-4",
                "Seller tax registration identifier (BT-32), Seller VAT identifier (BT-31), or Seller tax representative VAT identifier (BT-63) shall be present.".to_string(),
                "Identificatorul de înregistrare fiscală a Vânzătorului (BT-32) și/sau Identificatorul de TVA al Vânzătorului (BT-31) și/sau al reprezentantului fiscal (BT-63) trebuie să fie înscris.".to_string(),
            );
        }

        let has_buyer_id = non_empty(inv.buyer.vat_identifier.as_deref())
            || non_empty(inv.buyer.tax_registration.as_deref())
            || inv
                .buyer
                .legal_registration
                .as_ref()
                .is_some_and(|id| !id.is_blank());
        if has_buyer_id {
            acc.pass();
        } else {
            acc.fatal(
                "BR-RO-120",
                "BR-RO-120",
                "BT-47 / BT-48",
                "BG-7",
                "Buyer legal registration identifier (BT-47) and/or Buyer VAT identifier (BT-48) shall be present.".to_string(),
                "Identificatorul de înregistrare legală a Cumpărătorului (BT-47) și/sau Identificatorul de TVA al Cumpărătorului (BT-48) trebuie să fie înscris.".to_string(),
            );
        }
    }

    // Seller & Buyer mandatory street (BR-RO-080) & city (BR-RO-090) + RO county/sector (BR-RO-100/110)
    check_required_field(
        inv.seller.address.line1.as_deref(),
        "BR-RO-080",
        "BR-RO-081",
        "BT-35",
        "BG-5",
        "Seller address line 1 (BT-35)",
        "Linia 1 a adresei Vânzătorului (BT-35)",
        acc,
    );
    check_required_field(
        inv.seller.address.city.as_deref(),
        "BR-RO-090",
        "BR-RO-091",
        "BT-37",
        "BG-5",
        "Seller city (BT-37)",
        "Localitatea Vânzătorului (BT-37)",
        acc,
    );
    check_ro_county_and_sector(
        &inv.seller.address,
        "Seller",
        "Vânzătorului",
        "BR-RO-110",
        "BR-RO-110",
        "BR-RO-100",
        "BR-RO-100",
        "BT-39",
        "BT-37",
        "BG-4",
        acc,
    );

    check_required_field(
        inv.buyer.address.line1.as_deref(),
        "BR-RO-080",
        "BR-RO-082",
        "BT-50",
        "BG-8",
        "Buyer address line 1 (BT-50)",
        "Linia 1 a adresei Cumpărătorului (BT-50)",
        acc,
    );
    check_required_field(
        inv.buyer.address.city.as_deref(),
        "BR-RO-090",
        "BR-RO-092",
        "BT-52",
        "BG-8",
        "Buyer city (BT-52)",
        "Localitatea Cumpărătorului (BT-52)",
        acc,
    );
    check_ro_county_and_sector(
        &inv.buyer.address,
        "Buyer",
        "Cumpărătorului",
        "BR-RO-110",
        "BR-RO-111",
        "BR-RO-100",
        "BR-RO-101",
        "BT-54",
        "BT-52",
        "BG-7",
        acc,
    );

    // Tax Representative (BG-11 / BG-12)
    if let Some(tr) = &inv.tax_representative {
        check_required_field(
            tr.address.line1.as_deref(),
            "BR-RO-140",
            "BR-RO-140",
            "BT-64",
            "BG-12",
            "Tax representative address line 1 (BT-64)",
            "Linia 1 a adresei reprezentantului fiscal (BT-64)",
            acc,
        );
        check_required_field(
            tr.address.city.as_deref(),
            "BR-RO-150",
            "BR-RO-150",
            "BT-66",
            "BG-12",
            "Tax representative city (BT-66)",
            "Localitatea reprezentantului fiscal (BT-66)",
            acc,
        );
        check_ro_county_and_sector(
            &tr.address,
            "Tax representative",
            "Reprezentantului fiscal",
            "BR-RO-170",
            "BR-RO-170",
            "BR-RO-160",
            "BR-RO-160",
            "BT-68",
            "BT-66",
            "BG-12",
            acc,
        );
    }

    // Delivery Address (BG-15)
    if let Some(a) = inv.delivery.as_ref().and_then(|d| d.address.as_ref()) {
        check_required_field(
            a.line1.as_deref(),
            "BR-RO-180",
            "BR-RO-180",
            "BT-75",
            "BG-15",
            "Deliver to address line 1 (BT-75)",
            "Adresa de livrare — Linia 1 (BT-75)",
            acc,
        );
        check_required_field(
            a.city.as_deref(),
            "BR-RO-200",
            "BR-RO-201",
            "BT-77",
            "BG-15",
            "Deliver to city (BT-77)",
            "Localitatea de livrare (BT-77)",
            acc,
        );
        check_required_field(
            a.subdivision.as_deref(),
            "BR-RO-210",
            "BR-RO-211",
            "BT-79",
            "BG-15",
            "Deliver to country subdivision (BT-79)",
            "Subdiviziunea țării de livrare (BT-79)",
            acc,
        );
        check_ro_county_and_sector(
            a,
            "Delivery",
            "de livrare",
            "BR-RO-210",
            "BR-RO-212",
            "BR-RO-200",
            "BR-RO-202",
            "BT-79",
            "BT-77",
            "BG-15",
            acc,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn check_required_field(
    val: Option<&str>,
    code: &str,
    sub: &str,
    bt: &str,
    loc: &str,
    label_en: &str,
    label_ro: &str,
    acc: &mut RuleAcc,
) {
    if non_empty(val) {
        acc.pass();
    } else {
        acc.fatal(
            code,
            sub,
            bt,
            loc,
            format!("{label_en} must be provided."),
            format!("{label_ro} trebuie furnizată."),
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn check_ro_county_and_sector(
    addr: &PostalAddress,
    role_en: &str,
    role_ro_gen: &str,
    county_code: &str,
    county_sub: &str,
    sector_code: &str,
    sector_sub: &str,
    bt_sub: &str,
    bt_city: &str,
    loc: &str,
    acc: &mut RuleAcc,
) {
    if addr_country(addr) != "RO" {
        return;
    }
    let sub = addr.subdivision.as_deref().map_or("", str::trim);
    if County::from_code(sub).is_some() {
        acc.pass();
    } else {
        let hint = match County::lookup_fuzzy(sub) {
            Some(c) => (
                format!("Did you mean '{}' ({})?", c.code(), c.name_ro()),
                format!("Înlocuiți valoarea cu '{}' ({}).", c.code(), c.name_ro()),
            ),
            None => (
                "Use an ISO 3166-2:RO code such as 'RO-B', 'RO-CJ', 'RO-TM'.".to_string(),
                "Utilizați un cod ISO 3166-2:RO valid (ex. 'RO-B', 'RO-CJ', 'RO-TM').".to_string(),
            ),
        };
        acc.push(
            Severity::Fatal,
            county_code,
            county_sub,
            bt_sub,
            loc,
            format!("When {role_en} country code is RO, country subdivision ({bt_sub}) '{sub}' must be coded using ISO 3166-2:RO (e.g. 'RO-B', 'RO-AB'..'RO-VN')."),
            format!("Dacă țara {role_ro_gen} este RO, subdiviziunea țării ({bt_sub}) '{sub}' trebuie codificată conform listei ISO 3166-2:RO (ex. 'RO-B', 'RO-AB'..'RO-VN')."),
            Some("RO-B or RO-AB..RO-VN"),
            Some(sub.to_string()),
            Some(hint),
        );
    }

    if sub == "RO-B" {
        let city = addr.city.as_deref().map_or("", str::trim);
        if BucharestSector::from_code(city).is_some() {
            acc.pass();
        } else {
            let hint = match BucharestSector::normalize_fuzzy(city) {
                Some(s) => (
                    format!("Replace '{city}' with exact uppercase '{}'.", s.code()),
                    format!("Înlocuiți '{city}' cu valoarea exactă '{}'.", s.code()),
                ),
                None => (
                    "For Bucharest (RO-B), set CityName to one of SECTOR1, SECTOR2, SECTOR3, SECTOR4, SECTOR5, SECTOR6.".to_string(),
                    "Pentru Municipiul București (RO-B), setați CityName la SECTOR1, SECTOR2, SECTOR3, SECTOR4, SECTOR5 sau SECTOR6.".to_string(),
                ),
            };
            acc.push(
                Severity::Fatal,
                sector_code,
                sector_sub,
                bt_city,
                loc,
                format!("When {role_en} country is RO and subdivision is RO-B (Bucharest), city ({bt_city}) must be coded as SECTOR1..SECTOR6 (found '{city}')."),
                format!("Dacă țara {role_ro_gen} este RO și subdiviziunea este RO-B (București), localitatea ({bt_city}) trebuie codificată ca SECTOR1..SECTOR6 (găsit: '{city}')."),
                Some("SECTOR1..SECTOR6"),
                Some(city.to_string()),
                Some(hint),
            );
        }
    }
}

fn check_all_checksums(inv: &Invoice, acc: &mut RuleAcc) {
    check_party_checksums(&inv.seller, "Seller", "Vânzător", "BG-4", acc);
    check_party_checksums(&inv.buyer, "Buyer", "Cumpărător", "BG-7", acc);

    if let Some(tr_vat) = inv
        .tax_representative
        .as_ref()
        .and_then(|t| t.vat_identifier.as_deref())
    {
        if let Some(res) = parse_cui_candidate(tr_vat) {
            match res {
                Ok(_) => acc.pass(),
                Err(e) => acc.push(
                    Severity::Warning,
                    "BR-RO-CUI",
                    "BR-RO-CUI",
                    "BT-63",
                    "BG-11/BT-63",
                    format!("Tax representative fiscal identifier '{tr_vat}' fails CUI validation: {}", e.display(cius_ro_id::Locale::En)),
                    format!("Identificatorul fiscal al reprezentantului fiscal '{tr_vat}' este invalid: {}", e.display(cius_ro_id::Locale::Ro)),
                    Some("Valid modulo-11 Romanian CUI/CIF"),
                    Some(tr_vat.to_string()),
                    None,
                ),
            }
        }
    }

    if let Some(PaymentMeans::CreditTransfer(cts)) =
        inv.payment.as_ref().and_then(|p| p.means.as_ref())
    {
        for ct in cts {
            if let Some(raw) = ct.account_identifier.as_deref() {
                let compact: String = raw.chars().filter(|c| !c.is_whitespace()).collect();
                if compact
                    .get(..2)
                    .is_some_and(|p| p.eq_ignore_ascii_case("RO"))
                {
                    match RoIban::parse(raw) {
                        Ok(_) => acc.pass(),
                        Err(e) => acc.push(
                            Severity::Warning,
                            "BR-RO-IBAN",
                            "BR-RO-IBAN",
                            "BT-84",
                            "BG-17/BT-84",
                            format!(
                                "Payee Romanian IBAN '{raw}' (BT-84) is invalid: {}",
                                e.display(cius_ro_id::Locale::En)
                            ),
                            format!(
                                "Codul IBAN românesc '{raw}' (BT-84) este invalid: {}",
                                e.display(cius_ro_id::Locale::Ro)
                            ),
                            Some("Valid 24-character RO IBAN (ISO 7064 MOD 97-10)"),
                            Some(raw.to_string()),
                            None,
                        ),
                    }
                }
            }
        }
    }
}

fn check_party_checksums(party: &Party, role_en: &str, role_ro: &str, bg: &str, acc: &mut RuleAcc) {
    let tax_ids: Vec<&str> = [
        party.vat_identifier.as_deref(),
        party.tax_registration.as_deref(),
    ]
    .into_iter()
    .flatten()
    .collect();
    let legal_id = party.legal_registration.as_ref().map(|i| i.content());
    let is_ro = addr_country(&party.address) == "RO"
        || tax_ids
            .iter()
            .any(|id| id.get(..2).is_some_and(|p| p.eq_ignore_ascii_case("RO")));
    if !is_ro {
        return;
    }

    for &cand in tax_ids.iter().chain(legal_id.iter()) {
        if let Some(res) = parse_cui_candidate(cand) {
            match res {
                Ok(_) => acc.pass(),
                Err(e) => acc.push(
                    Severity::Warning,
                    "BR-RO-CUI",
                    "BR-RO-CUI",
                    &format!("{bg} (Romanian CUI)"),
                    bg,
                    format!("Romanian {role_en} fiscal identifier '{cand}' fails modulo-11 verification: {}", e.display(cius_ro_id::Locale::En)),
                    format!("Identificatorul fiscal al {role_ro}ului '{cand}' a eșuat verificarea cifrei de control modulo-11: {}", e.display(cius_ro_id::Locale::Ro)),
                    Some("Valid modulo-11 Romanian CUI/CIF"),
                    Some(cand.to_string()),
                    Some((
                        "Verify the CUI/CIF against the ANAF / Ministry of Finance register.".to_string(),
                        "Verificați codul CUI/CIF în registrul contribuabililor ANAF / Ministerul Finanțelor.".to_string(),
                    )),
                ),
            }
            break;
        }
    }

    for &cand in tax_ids.iter().chain(legal_id.iter()) {
        let t = cand.trim();
        if t.len() == 13 && t.bytes().all(|b| b.is_ascii_digit()) {
            if is_anaf_b2c_placeholder(t) {
                acc.pass();
            } else {
                match Cnp::parse(t) {
                    Ok(_) => acc.pass(),
                    Err(e) => acc.push(
                        Severity::Warning,
                        "BR-RO-CNP",
                        "BR-RO-CNP",
                        &format!("{bg} (Romanian CNP)"),
                        bg,
                        format!("Romanian {role_en} 13-digit identifier '{t}' fails CNP verification: {}", e.display(cius_ro_id::Locale::En)),
                        format!("Identificatorul de 13 cifre al {role_ro}ului '{t}' nu este un CNP valid: {}", e.display(cius_ro_id::Locale::Ro)),
                        Some("Valid 13-digit CNP or 0000000000000 (B2C)"),
                        Some(t.to_string()),
                        Some((
                            "Use '0000000000000' for B2C consumers who do not provide a CNP.".to_string(),
                            "Utilizați '0000000000000' pentru persoane fizice (B2C) care nu comunică CNP-ul.".to_string(),
                        )),
                    ),
                }
            }
        }
    }

    if let Some(legal) = legal_id.filter(|l| is_onrc_candidate(l)) {
        match Onrc::parse(legal) {
            Ok(_) => acc.pass(),
            Err(e) => acc.push(
                Severity::Warning,
                "BR-RO-ONRC",
                "BR-RO-ONRC",
                &format!("{bg} (ONRC)"),
                bg,
                format!("Romanian {role_en} Trade Register number '{legal}' is malformed: {}", e.display(cius_ro_id::Locale::En)),
                format!("Numărul din Registrul Comerțului (ONRC) '{legal}' al {role_ro}ului este invalid: {}", e.display(cius_ro_id::Locale::Ro)),
                Some("J40/1234/2022 or J2024001234005"),
                Some(legal.to_string()),
                None,
            ),
        }
    }
}

fn addr_country(addr: &PostalAddress) -> &str {
    addr.country.as_ref().map_or("", |c| c.as_str().trim())
}

fn non_empty(s: Option<&str>) -> bool {
    s.is_some_and(|v| !v.trim().is_empty())
}

fn is_xsd_true(val: Option<&str>) -> bool {
    matches!(val.map(str::trim), Some("true" | "1"))
}

fn norm_space(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn count_decimals(raw: &str) -> usize {
    raw.split_once('.').map_or(0, |(_, frac)| {
        frac.bytes().take_while(u8::is_ascii_digit).count()
    })
}

fn child_elem<'a, 'input>(parent: Node<'a, 'input>, local: &str) -> Option<Node<'a, 'input>> {
    parent
        .children()
        .find(|n| n.is_element() && n.tag_name().name() == local)
}

fn children_named<'a, 'input>(
    parent: Node<'a, 'input>,
    local: &'a str,
) -> impl Iterator<Item = Node<'a, 'input>> + 'a {
    parent
        .children()
        .filter(move |n| n.is_element() && n.tag_name().name() == local)
}

fn child_text(parent: Node<'_, '_>, local: &str) -> Option<String> {
    child_elem(parent, local)
        .and_then(|n| n.text())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}
