//! Evaluator for ANAF `RO e-Factura` (`CIUS-RO 1.0.1` / `ROeFactura v1.0.9`) national rules
//! and `cius-ro-id` Romanian identifier checksums across OASIS UBL 2.1 (`Invoice`, `CreditNote`)
//! and UN/CEFACT CII D16B (`CrossIndustryInvoice`), operating on the shared `en16931::Invoice`
//! semantic model plus lexical XML facets (`BR-RO-DT*`, `BR-RO-Z2`).

mod checksums;
mod core;
mod dates;
mod decimals;
mod lengths;
mod parties;

use std::borrow::Cow;

use en16931::Invoice;
use en16931::invoice::PostalAddress;
use roxmltree::{Document, Node};

use crate::stage_acc::{StageAcc, StageFinding, tr_fmt};
use crate::types::{
    CIUS_RO_CUSTOMIZATION_ID, EngineId, EngineReport, ValidateOptions, Violation, XmlDecodeError,
    XmlSyntax,
};

pub(crate) const ENGINE_ID: EngineId = EngineId::AnafCiusRo;
pub(crate) const ENGINE_NAME: &str = "ANAF RO e-Factura (CIUS-RO 1.0.1) & Checksum Engine";
pub(crate) const ENGINE_VERSION: &str = "CIUS-RO 1.0.1 (ROeFactura v1.0.9)";

pub(super) const UBL_INVOICE_NS: &str = "urn:oasis:names:specification:ubl:schema:xsd:Invoice-2";
pub(super) const UBL_CREDITNOTE_NS: &str =
    "urn:oasis:names:specification:ubl:schema:xsd:CreditNote-2";
pub(super) const UBL_CBC_NS: &str =
    "urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2";
pub(super) const UBL_CAC_NS: &str =
    "urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2";
pub(super) const CII_NS: &str = "urn:un:unece:uncefact:data:standard:CrossIndustryInvoice:100";
pub(super) const CII_RAM_NS: &str =
    "urn:un:unece:uncefact:data:standard:ReusableAggregateBusinessInformationEntity:100";
pub(super) const CII_UDT_NS: &str = "urn:un:unece:uncefact:data:standard:UnqualifiedDataType:100";

/// Outcome of evaluating ANAF `CIUS-RO 1.0.1` rules and `cius-ro-id` checksums on an XML document.
#[derive(Debug, Clone)]
#[non_exhaustive]
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

/// Decodes UTF-8 (with optional BOM) or UTF-16BE / UTF-16LE XML bytes into a borrowed or owned string.
///
/// UTF-8 inputs (with or without a `EF BB BF` BOM) borrow directly from `bytes` with zero heap
/// allocation (`Cow::Borrowed`). UTF-16BE / UTF-16LE inputs are transcoded into `Cow::Owned`.
///
/// # Errors
///
/// Returns [`XmlDecodeError`] if `bytes` contains invalid UTF-8 or malformed/truncated UTF-16.
pub fn decode_xml_bytes(bytes: &[u8]) -> Result<Cow<'_, str>, XmlDecodeError> {
    if let Some(rest) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        return ::core::str::from_utf8(rest)
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
            let units = rest.as_chunks::<2>().0.iter().copied().map(|pair| {
                if is_be {
                    u16::from_be_bytes(pair)
                } else {
                    u16::from_le_bytes(pair)
                }
            });
            let decoded: String = char::decode_utf16(units)
                .collect::<Result<String, _>>()
                .map_err(|_| XmlDecodeError::InvalidUtf16 { encoding })?;
            return Ok(Cow::Owned(decoded));
        }
    }
    ::core::str::from_utf8(bytes)
        .map(Cow::Borrowed)
        .map_err(XmlDecodeError::InvalidUtf8)
}

/// Inspects a parsed XML root element to identify its [`XmlSyntax`] and `BT-24` specification ID.
#[must_use]
pub fn detect_root_syntax_and_bt24(root: Node<'_, '_>) -> (Option<XmlSyntax>, Option<String>) {
    let ns = root.tag_name().namespace().unwrap_or("");
    let local = root.tag_name().name();
    let syntax = match (local, ns) {
        ("Invoice", UBL_INVOICE_NS) => Some(XmlSyntax::UblInvoice),
        ("CreditNote", UBL_CREDITNOTE_NS) => Some(XmlSyntax::UblCreditNote),
        ("CrossIndustryInvoice", CII_NS) => Some(XmlSyntax::CiiD16B),
        _ => None,
    };

    let bt24 = match syntax {
        Some(XmlSyntax::UblInvoice | XmlSyntax::UblCreditNote) => {
            child_text(root, UBL_CBC_NS, "CustomizationID")
        }
        Some(XmlSyntax::CiiD16B) => child_elem(root, CII_NS, "ExchangedDocumentContext")
            .and_then(|c| child_elem(c, CII_RAM_NS, "GuidelineSpecifiedDocumentContextParameter"))
            .and_then(|g| child_text(g, CII_RAM_NS, "ID")),
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
    let decoded = decode_xml_bytes(xml_bytes);
    let roxml_res = decoded.as_deref().ok().map(Document::parse);
    if let Some(Ok(doc)) = &roxml_res {
        let (syntax, customization_id) = detect_root_syntax_and_bt24(doc.root_element());
        evaluate_parsed(xml_bytes, doc, syntax, customization_id, None, true, opts)
    } else {
        failed_unparsed(
            decoded.as_deref(),
            roxml_res.as_ref().map(Result::as_ref),
            opts,
        )
    }
}

pub(crate) fn skipped_for_unparsed(opts: &ValidateOptions) -> BrRoOutcome {
    let reason = if !opts.run_cius_ro_rules && !opts.run_checksums {
        "Disabled by ValidateOptions"
    } else {
        "XML is not well-formed or contains prohibited DTD"
    };
    skipped_outcome(reason, None, None)
}

pub(crate) fn failed_unparsed(
    xml_res: Result<&str, &XmlDecodeError>,
    roxml_res: Option<Result<&Document<'_>, &roxmltree::Error>>,
    opts: &ValidateOptions,
) -> BrRoOutcome {
    let mut acc = StageAcc::new(ENGINE_ID, ENGINE_NAME, ENGINE_VERSION, opts.locale);
    match xml_res {
        Err(err) => {
            acc.record(
                StageFinding::fatal("BR-RO-XML-ENCODING", err.message(acc.locale()))
                    .term("W3C XML 1.0")
                    .location("Byte offset 0"),
            );
        }
        Ok(_) => match roxml_res {
            Some(Err(roxmltree::Error::DtdDetected)) => {
                let msg = acc
                    .tr(
                        "XML DOCTYPE / ENTITY declarations are prohibited (XXE defense).",
                        "Declarațiile XML DOCTYPE / ENTITY sunt interzise (protecție XXE).",
                    )
                    .to_owned();
                acc.record(
                    StageFinding::fatal("BR-RO-XXE-PROHIBITED", msg)
                        .term("W3C XML 1.0 / OWASP XXE")
                        .location("XML Prologue"),
                );
            }
            Some(Err(err)) => {
                let msg = tr_fmt!(
                    acc,
                    "XML document is not well-formed: {err}",
                    "Documentul XML nu este bine format: {err}"
                );
                acc.record(StageFinding::fatal("BR-RO-XML-SYNTAX", msg).term("W3C XML 1.0"));
            }
            _ => {
                let msg = acc
                    .tr(
                        "XML document could not be parsed.",
                        "Documentul XML nu a putut fi analizat.",
                    )
                    .to_owned();
                acc.record(StageFinding::fatal("BR-RO-XML-SYNTAX", msg).term("W3C XML 1.0"));
            }
        },
    }
    finish_outcome(acc, None, None)
}

pub(crate) fn evaluate_parsed(
    xml_bytes: &[u8],
    doc: &Document<'_>,
    syntax: Option<XmlSyntax>,
    customization_id: Option<String>,
    pre_read: Option<Invoice>,
    report_unparseable: bool,
    opts: &ValidateOptions,
) -> BrRoOutcome {
    if !opts.run_cius_ro_rules && !opts.run_checksums {
        return skipped_outcome("Disabled by ValidateOptions", syntax, customization_id);
    }
    let root = doc.root_element();
    let Some(syntax_kind) = syntax else {
        if !report_unparseable {
            return skipped_outcome("Unrecognized XML root element", None, customization_id);
        }
        let mut acc = StageAcc::new(ENGINE_ID, ENGINE_NAME, ENGINE_VERSION, opts.locale);
        let root_name = root.tag_name().name();
        let msg = tr_fmt!(
            acc,
            "Root XML element '<{root_name}>' is neither OASIS UBL 2.1 Invoice/CreditNote nor UN/CEFACT CII D16B CrossIndustryInvoice.",
            "Elementul rădăcină XML '<{root_name}>' nu este o factură OASIS UBL 2.1 Invoice/CreditNote sau UN/CEFACT CII D16B CrossIndustryInvoice."
        );
        acc.record(
            StageFinding::fatal("BR-RO-UNKNOWN-ROOT", msg)
                .term("XML Root Element")
                .location(root_name),
        );
        return finish_outcome(acc, None, customization_id);
    };

    let inv = match pre_read {
        Some(inv) => inv,
        None => match en16931_formats::read(xml_bytes) {
            Ok(read_out) => read_out.invoice,
            Err(err) if report_unparseable => {
                let mut acc = StageAcc::new(ENGINE_ID, ENGINE_NAME, ENGINE_VERSION, opts.locale);
                let msg = tr_fmt!(
                    acc,
                    "EN 16931 invoice model could not be extracted from XML: {err}",
                    "Modelul semantic EN 16931 nu a putut fi extras din documentul XML: {err}"
                );
                acc.record(
                    StageFinding::fatal("BR-RO-XML-UNREADABLE", msg).term(syntax_kind.label()),
                );
                return finish_outcome(acc, syntax, customization_id);
            }
            Err(_) => {
                return skipped_outcome(
                    "EN 16931 reader could not load document",
                    syntax,
                    customization_id,
                );
            }
        },
    };

    let mut acc = StageAcc::new(ENGINE_ID, ENGINE_NAME, ENGINE_VERSION, opts.locale);

    if opts.run_cius_ro_rules
        && should_evaluate_national_rules(&inv, customization_id.as_deref(), opts, &mut acc)
    {
        self::core::check_core_rules(&inv, root, syntax_kind, opts, &mut acc);
        dates::check_dates_lexical(root, syntax_kind, &mut acc);
        self::core::check_cardinality(&inv, syntax_kind, &mut acc);
        lengths::check_lengths(&inv, syntax_kind, &mut acc);
        decimals::check_decimals_lexical(root, syntax_kind, &inv, &mut acc);
        parties::check_parties_and_addresses(&inv, &mut acc);
    }

    if opts.run_checksums {
        checksums::check_all_checksums(&inv, &mut acc);
    }

    finish_outcome(acc, syntax, customization_id)
}

fn should_evaluate_national_rules(
    inv: &Invoice,
    customization_id: Option<&str>,
    opts: &ValidateOptions,
    acc: &mut StageAcc,
) -> bool {
    let spec = customization_id
        .or(inv.specification_id.as_deref())
        .unwrap_or("");
    if spec == CIUS_RO_CUSTOMIZATION_ID {
        acc.pass();
        return true;
    }

    let mentions_ro = contains_ascii_case_insensitive(spec, "cius-ro")
        || contains_ascii_case_insensitive(spec, "efactura.mfinante.ro");
    if opts.strict_cius_ro_customization || mentions_ro {
        let msg = tr_fmt!(
            acc,
            "Specification identifier (BT-24) must equal '{CIUS_RO_CUSTOMIZATION_ID}' (found '{spec}').",
            "Identificatorul specificației (BT-24) trebuie să corespundă exact valorii '{CIUS_RO_CUSTOMIZATION_ID}' (găsit: '{spec}')."
        );
        let hint = tr_fmt!(
            acc,
            "Set CustomizationID (BT-24) to '{CIUS_RO_CUSTOMIZATION_ID}'.",
            "Setați CustomizationID (BT-24) la '{CIUS_RO_CUSTOMIZATION_ID}'."
        );
        acc.record(
            StageFinding::fatal("BR-RO-001", msg)
                .sub_id("BR-RO-001")
                .term("BT-24")
                .location("BT-24")
                .expected(CIUS_RO_CUSTOMIZATION_ID)
                .actual(spec.to_owned())
                .hint(hint),
        );
        return true;
    }

    if checksums::has_romanian_context(inv) {
        let msg = tr_fmt!(
            acc,
            "Romanian-context invoice does not declare ANAF CIUS-RO 1.0.1 CustomizationID '{CIUS_RO_CUSTOMIZATION_ID}'.",
            "Factura cu context românesc nu declară identificatorul ANAF CIUS-RO 1.0.1 '{CIUS_RO_CUSTOMIZATION_ID}'."
        );
        acc.record(
            StageFinding::warning("BR-RO-001", msg)
                .sub_id("BR-RO-001")
                .term("BT-24")
                .location("BT-24")
                .expected(CIUS_RO_CUSTOMIZATION_ID)
                .actual(spec.to_owned()),
        );
    }
    false
}

fn finish_outcome(
    acc: StageAcc,
    syntax: Option<XmlSyntax>,
    customization_id: Option<String>,
) -> BrRoOutcome {
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

fn contains_ascii_case_insensitive(haystack: &str, needle: &str) -> bool {
    let n = needle.as_bytes();
    haystack
        .as_bytes()
        .windows(n.len())
        .any(|w| w.eq_ignore_ascii_case(n))
}

pub(super) fn addr_country(addr: &PostalAddress) -> &str {
    addr.country.as_ref().map_or("", |c| c.as_str().trim())
}

pub(super) fn non_empty(s: Option<&str>) -> bool {
    s.is_some_and(|v| !v.trim().is_empty())
}

pub(super) fn is_xsd_true(val: Option<&str>) -> bool {
    matches!(val.map(str::trim), Some("true" | "1"))
}

pub(super) fn child_elem<'a, 'input>(
    parent: Node<'a, 'input>,
    ns: &str,
    local: &str,
) -> Option<Node<'a, 'input>> {
    parent.children().find(|n| {
        n.is_element() && n.tag_name().namespace() == Some(ns) && n.tag_name().name() == local
    })
}

pub(super) fn children_named<'a, 'input>(
    parent: Node<'a, 'input>,
    ns: &'a str,
    local: &'a str,
) -> impl Iterator<Item = Node<'a, 'input>> + 'a {
    parent.children().filter(move |n| {
        n.is_element() && n.tag_name().namespace() == Some(ns) && n.tag_name().name() == local
    })
}

pub(super) fn child_text(parent: Node<'_, '_>, ns: &str, local: &str) -> Option<String> {
    child_elem(parent, ns, local)
        .and_then(|n| n.text())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}
