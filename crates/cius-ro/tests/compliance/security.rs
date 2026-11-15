//! XML security & boundary tests: XXE/DTD rejection, deep comment padding, multi-byte UTF-8
//! boundaries, BOM/UTF-16 decoding, max element depth, namespace isolation, and standalone
//! malformed/unrecognized XML gates.

use std::fmt::Write as _;

use cius_ro::{
    EngineStatus, Recommendation, Severity, ValidateOptions, evaluate_cius_ro_rules,
    validate_bytes, validate_xml,
};

use crate::helpers::{XmlFixture, assert_has_code};

#[test]
fn utf8_multibyte_boundary_and_deep_xxe_safety() {
    let base = XmlFixture::load("01_standard_b2b_ro_ubl.xml");

    // 1. Construct a >22 KiB compliant UBL invoice rich in Romanian multi-byte UTF-8 characters
    //    (ă, â, î, ș, ț, €) where a multi-byte sequence explicitly straddles byte index 16_384.
    let diacritic_chunk = "ăâîșțĂÂÎȘȚ€";
    let note_body = diacritic_chunk.repeat(24);
    let mut large_xml = String::new();
    for pad in 0..150 {
        let prefix_pad = "X".repeat(pad % 30);
        let extra_ascii = "Y".repeat(pad);
        let mut notes = format!("<cbc:Note>{prefix_pad}{note_body}</cbc:Note>\n");
        let _ = writeln!(notes, "<cbc:Note>{extra_ascii}</cbc:Note>");
        for _ in 2..20 {
            let _ = writeln!(notes, "<cbc:Note>{note_body}</cbc:Note>");
        }
        let mut add_docs = String::new();
        for idx in 0..25 {
            let _ = writeln!(
                add_docs,
                "<cac:AdditionalDocumentReference><cbc:ID>ANEXA-{idx}</cbc:ID><cbc:DocumentDescription>{diacritic_chunk} anexă justificativă {idx}</cbc:DocumentDescription></cac:AdditionalDocumentReference>"
            );
        }
        let candidate = base
            .clone()
            .insert_before("DocumentCurrencyCode", &notes)
            .insert_before("AccountingSupplierParty", &add_docs);
        assert!(candidate.as_str().len() > 20_000);
        if !candidate.as_str().is_char_boundary(16_384) {
            large_xml = candidate.as_str().to_owned();
            break;
        }
    }
    assert!(
        !large_xml.is_empty() && !large_xml.is_char_boundary(16_384),
        "Must test an XML payload with a multi-byte UTF-8 char straddling byte 16_384"
    );
    let rep_large = validate_bytes(
        large_xml.as_bytes(),
        &ValidateOptions::default().deny_warnings(true),
    );
    assert!(
        rep_large.is_valid && rep_large.violations.is_empty(),
        "Large >20 KiB multi-byte UTF-8 invoice must pass without panic or violations: {:?}",
        rep_large.violations
    );

    // 2. Immediate & deep (>18 KiB comment padding) <!DOCTYPE / <!ENTITY must both be rejected
    let xxe_xml = base.clone().replace_once(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>",
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE Invoice [ <!ENTITY xxe \"boom\"> ]>",
    );
    #[cfg(feature = "xsd")]
    assert_has_code(xxe_xml.as_bytes(), "XSD-XXE-PROHIBITED");
    #[cfg(not(feature = "xsd"))]
    assert_has_code(xxe_xml.as_bytes(), "EN16931-UNREADABLE");

    let deep_comment = format!("<!-- {} -->\n", "padding-comment-".repeat(1200));
    assert!(deep_comment.len() > 18_000);
    let deep_xxe = base.replace_once(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>",
        &format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n{deep_comment}<!DOCTYPE Invoice [ <!ENTITY xxe \"boom\"> ]>"
        ),
    );
    #[cfg(feature = "xsd")]
    assert_has_code(deep_xxe.as_bytes(), "XSD-XXE-PROHIBITED");
    #[cfg(not(feature = "xsd"))]
    assert_has_code(deep_xxe.as_bytes(), "EN16931-UNREADABLE");
}

#[test]
fn xml_bom_utf8_and_utf16_decoding() {
    let base = XmlFixture::load("01_standard_b2b_ro_ubl.xml");

    // 1. UTF-8 with BOM (\xEF\xBB\xBF)
    let mut utf8_bom = vec![0xEF, 0xBB, 0xBF];
    utf8_bom.extend_from_slice(base.as_bytes());
    let rep_utf8_bom = validate_bytes(&utf8_bom, &ValidateOptions::default());
    assert!(
        rep_utf8_bom.is_valid && rep_utf8_bom.violations.is_empty(),
        "UTF-8 BOM XML must pass cleanly: {:?}",
        rep_utf8_bom.violations
    );

    // 2. UTF-16LE with BOM (\xFF\xFE) and UTF-16BE with BOM (\xFE\xFF)
    let u16_src = base.replace_once("encoding=\"UTF-8\"", "encoding=\"UTF-16\"");
    let mut little_endian_u16 = vec![0xFF, 0xFE];
    for unit in u16_src.as_str().encode_utf16() {
        little_endian_u16.extend_from_slice(&unit.to_le_bytes());
    }
    let rep_little = validate_bytes(&little_endian_u16, &ValidateOptions::default());
    assert!(
        rep_little.is_valid && rep_little.violations.is_empty(),
        "UTF-16LE BOM XML must pass cleanly: {:?}",
        rep_little.violations
    );

    let mut big_endian_u16 = vec![0xFE, 0xFF];
    for unit in u16_src.as_str().encode_utf16() {
        big_endian_u16.extend_from_slice(&unit.to_be_bytes());
    }
    let rep_big = validate_bytes(&big_endian_u16, &ValidateOptions::default());
    assert!(
        rep_big.is_valid && rep_big.violations.is_empty(),
        "UTF-16BE BOM XML must pass cleanly: {:?}",
        rep_big.violations
    );
}

#[cfg(feature = "xsd")]
#[test]
fn xsd_malformed_xml_with_xxe_keywords_emits_wellformedness_not_xxe() {
    for benign_malformed in [
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
         <Invoice xmlns=\"urn:oasis:names:specification:ubl:schema:xsd:Invoice-2\">\
         forbid_dtd and forbid_entities unclosed",
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
         <!-- <!DOCTYPE Invoice [ <!ENTITY xxe \"boom\"> ]> -->\
         <Invoice attr=\"<!DOCTYPE fake>\"><![CDATA[<!ENTITY fake \"x\">]]>unclosed",
    ] {
        let rep = validate_bytes(benign_malformed.as_bytes(), &ValidateOptions::default());
        assert!(!rep.is_valid);
        assert!(
            rep.violations
                .iter()
                .any(|v| v.code == "XSD-WELLFORMEDNESS"),
            "Expected XSD-WELLFORMEDNESS on unclosed XML, got: {:?}",
            rep.violations
        );
        assert!(
            !rep.violations
                .iter()
                .any(|v| v.code == "XSD-XXE-PROHIBITED"),
            "Comment/CDATA/attribute DTD text must not emit XSD-XXE-PROHIBITED: {:?}",
            rep.violations
        );
    }

    // Actual <!ENTITY ...> markup declaration must still emit XSD-XXE-PROHIBITED
    let rogue_entity = b"<?xml version=\"1.0\"?><Invoice><!ENTITY xxe \"boom\"></Invoice>";
    assert_has_code(rogue_entity, "XSD-XXE-PROHIBITED");
}

#[test]
fn xml_strict_root_and_child_namespace_enforcement() {
    // 1. Un-namespaced <Invoice> must NOT be treated as UBL 2.1
    let no_ns_invoice = b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
        <Invoice><CustomizationID>urn:cen.eu:en16931:2017#compliant#urn:efactura.mfinante.ro:CIUS-RO:1.0.1</CustomizationID></Invoice>";
    let rep_no_ns = validate_bytes(no_ns_invoice, &ValidateOptions::default());
    assert!(!rep_no_ns.is_valid);
    assert_eq!(rep_no_ns.detected_syntax, None);
    #[cfg(feature = "xsd")]
    assert!(
        rep_no_ns
            .violations
            .iter()
            .any(|v| v.code == "XSD-UNKNOWN-ROOT")
    );

    // 2. Foreign-namespace child elements must not shadow UBL cbc:/cac: elements in br_ro's XML inspector
    let foreign_children = XmlFixture::load("01_standard_b2b_ro_ubl.xml")
        .insert_before(
            "CustomizationID",
            "<ext:CustomizationID xmlns:ext=\"urn:example:ext\">urn:wrong:customization</ext:CustomizationID>",
        )
        .insert_before(
            "IssueDate",
            "<ext:IssueDate xmlns:ext=\"urn:example:ext\">not-a-valid-date</ext:IssueDate>",
        )
        .insert_before(
            "LegalMonetaryTotal/LineExtensionAmount",
            "<ext:PayableAmount xmlns:ext=\"urn:example:ext\">5355.999</ext:PayableAmount>",
        );
    let rep_foreign = validate_bytes(
        foreign_children.as_bytes(),
        &ValidateOptions::default()
            .xsd(false)
            .en16931(false)
            .deny_warnings(true),
    );
    assert!(
        rep_foreign.is_valid && rep_foreign.violations.is_empty(),
        "Foreign-namespace elements must not shadow UBL cbc:/cac: elements: {:?}",
        rep_foreign.violations
    );
}

#[cfg(feature = "xsd")]
#[test]
fn xsd_deeply_nested_elements_exceeding_max_depth_rejected_pre_uppsala() {
    let mut deep_xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>");
    for _ in 0..70 {
        deep_xml.push_str("<n>");
    }
    for _ in 0..70 {
        deep_xml.push_str("</n>");
    }
    assert_has_code(deep_xml.as_bytes(), "XSD-WELLFORMEDNESS");
}

#[test]
fn standalone_evaluate_cius_ro_rules_reports_fatal_on_malformed_or_unrecognized_xml() {
    let opts = ValidateOptions::default();

    // 1. Truncated UTF-16 BOM -> Failed with BR-RO-XML-ENCODING
    let bad_utf16 = evaluate_cius_ro_rules(&[0xFF, 0xFE, 0x3C], &opts);
    assert_eq!(bad_utf16.report.status, EngineStatus::Failed);
    assert!(
        bad_utf16
            .violations
            .iter()
            .any(|v| v.code == "BR-RO-XML-ENCODING" && v.severity == Severity::Fatal)
    );

    // 2. Malformed XML -> Failed with BR-RO-XML-SYNTAX
    let bad_syntax = evaluate_cius_ro_rules(b"<Invoice><unclosed", &opts);
    assert_eq!(bad_syntax.report.status, EngineStatus::Failed);
    assert!(
        bad_syntax
            .violations
            .iter()
            .any(|v| v.code == "BR-RO-XML-SYNTAX" && v.severity == Severity::Fatal)
    );

    // 3. DTD / XXE -> Failed with BR-RO-XXE-PROHIBITED
    let xxe = evaluate_cius_ro_rules(b"<!DOCTYPE foo [<!ELEMENT foo ANY>]><foo/>", &opts);
    assert_eq!(xxe.report.status, EngineStatus::Failed);
    assert!(
        xxe.violations
            .iter()
            .any(|v| v.code == "BR-RO-XXE-PROHIBITED" && v.severity == Severity::Fatal)
    );

    // 4. Unknown root element -> Failed with BR-RO-UNKNOWN-ROOT
    let unknown_root = evaluate_cius_ro_rules(b"<Order xmlns=\"urn:example\"/>", &opts);
    assert_eq!(unknown_root.report.status, EngineStatus::Failed);
    assert!(
        unknown_root
            .violations
            .iter()
            .any(|v| v.code == "BR-RO-UNKNOWN-ROOT" && v.severity == Severity::Fatal)
    );

    // 5. Recognized UBL Invoice root missing mandatory CIUS-RO fields -> Failed with BR-RO-001
    let empty_ubl = evaluate_cius_ro_rules(
        b"<Invoice xmlns=\"urn:oasis:names:specification:ubl:schema:xsd:Invoice-2\"/>",
        &opts,
    );
    assert_eq!(empty_ubl.report.status, EngineStatus::Failed);
    assert!(
        empty_ubl
            .violations
            .iter()
            .any(|v| v.code == "BR-RO-001" && v.severity == Severity::Fatal)
    );

    // 6. Explicitly disabling both CIUS-RO rules and checksums still yields Skipped
    let disabled = evaluate_cius_ro_rules(
        b"<Invoice><unclosed",
        &ValidateOptions::default()
            .cius_ro_rules(false)
            .checksums(false),
    );
    assert_eq!(disabled.report.status, EngineStatus::Skipped);
    assert_eq!(disabled.violations, Vec::new());

    // 7. validate_xml with xsd(false).en16931(false) must still reject malformed/unknown XML
    let only_ro_opts = ValidateOptions::default().xsd(false).en16931(false);
    let rep_bad = validate_xml(b"<Invoice><unclosed", &only_ro_opts);
    assert!(!rep_bad.is_valid);
    assert_eq!(rep_bad.recommendation, Recommendation::Reject);
    let rep_root = validate_xml(b"<UnknownRoot/>", &only_ro_opts);
    assert!(!rep_root.is_valid);
    assert_eq!(rep_root.recommendation, Recommendation::Reject);
}
