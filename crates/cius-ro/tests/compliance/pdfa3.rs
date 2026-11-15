//! Hybrid ISO 19005-3 `PDF/A-3` XML extraction, tamper detection, `/Supplement` attachments,
//! UTF-8 filenames, `/ObjStm` compressed objects, `factur-x.xml`, and `XRef` bomb guards.

use cius_ro::{Locale, ValidateOptions, extract_xml_from_pdf, validate_bytes};
use lopdf::{Dictionary, Document, Object, Stream, dictionary};

use crate::helpers::{assert_has_code, load_fixture_bytes};

#[test]
fn pdfa3_extraction_and_tamper_mutations() {
    let valid_pdf = load_fixture_bytes("01_standard_b2b_ro_ubl.pdf");

    // 1. Direct XML extraction
    let ext = extract_xml_from_pdf(&valid_pdf).expect("extract embedded XML");
    let xml_str = String::from_utf8(ext.xml_bytes).expect("UTF-8 XML");
    assert!(xml_str.contains("EFACT-RO-2026-0042"));

    // 2. Corrupted PDF structure
    let corrupt_pdf = b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\ncorrupt-xref-payload";
    assert_has_code(corrupt_pdf, "PDFA3-6.1-PARSE");

    // 3. Inject /Encrypt into trailer -> PDFA3-6.1.4-ENCRYPT
    let mut doc = Document::load_mem(&valid_pdf).expect("parse valid PDF");
    doc.trailer
        .set("Encrypt", Object::Dictionary(Dictionary::new()));
    let mut encrypted_bytes = Vec::new();
    doc.save_to(&mut encrypted_bytes)
        .expect("serialize mutated PDF");
    assert_has_code(&encrypted_bytes, "PDFA3-6.1.4-ENCRYPT");

    // 4. Remove /AF from Document Catalog -> PDFA3-6.8-AF-MISSING
    let mut doc_no_af = Document::load_mem(&valid_pdf).expect("parse valid PDF");
    let root_id = doc_no_af
        .trailer
        .get(b"Root")
        .and_then(Object::as_reference)
        .expect("Catalog ref");
    if let Ok(Object::Dictionary(cat)) = doc_no_af.get_object_mut(root_id) {
        cat.remove(b"AF");
    }
    let mut no_af_bytes = Vec::new();
    doc_no_af
        .save_to(&mut no_af_bytes)
        .expect("serialize no-AF PDF");
    assert_has_code(&no_af_bytes, "PDFA3-6.8-AF-MISSING");
}

#[test]
fn pdfa3_supplement_attachment_af_membership_and_no_pdfa_diagnostics() {
    let base_pdf = load_fixture_bytes("01_standard_b2b_ro.pdf");

    // 1. Add a non-XML /Supplement attachment (attachment.txt, text/plain) to Catalog /AF:
    //    must PASS cleanly with deny_warnings=true (no false PDFA3-6.8-MIME-SUBTYPE warning)
    let mut doc_supp = Document::load_mem(&base_pdf).expect("parse base PDF");
    let txt_stream_id = doc_supp.add_object(Stream::new(
        dictionary! {
            "Type" => "EmbeddedFile",
            "Subtype" => "text/plain",
        },
        b"Anexa justificativa deviz lucrari".to_vec(),
    ));
    let supp_fs_id = doc_supp.add_object(dictionary! {
        "Type" => "Filespec",
        "F" => Object::string_literal("attachment.txt"),
        "UF" => Object::string_literal("attachment.txt"),
        "AFRelationship" => "Supplement",
        "EF" => dictionary! {
            "F" => txt_stream_id,
            "UF" => txt_stream_id,
        },
    });
    let root_id = doc_supp
        .trailer
        .get(b"Root")
        .and_then(Object::as_reference)
        .expect("Catalog ref");
    if let Ok(Object::Dictionary(cat)) = doc_supp.get_object_mut(root_id)
        && let Ok(Object::Array(af)) = cat.get_mut(b"AF")
    {
        af.push(Object::Reference(supp_fs_id));
    }
    let mut supp_pdf_bytes = Vec::new();
    doc_supp
        .save_to(&mut supp_pdf_bytes)
        .expect("save supplement PDF");
    let rep_supp = validate_bytes(
        &supp_pdf_bytes,
        &ValidateOptions::default().deny_warnings(true),
    );
    assert!(
        rep_supp.is_valid && rep_supp.violations.is_empty(),
        "Supplement text/plain attachment must not trigger false MIME warning: {:?}",
        rep_supp.violations
    );

    // 2. Remove efactura.xml from Catalog /AF while keeping attachment.txt in /AF:
    //    Catalog /AF is non-empty, yet efactura.xml is unassociated -> PDFA3-6.8-HYBRID-DIVERGENCE
    let mut doc_unassoc = Document::load_mem(&supp_pdf_bytes).expect("parse supplement PDF");
    let root_id2 = doc_unassoc
        .trailer
        .get(b"Root")
        .and_then(Object::as_reference)
        .expect("Catalog ref");
    if let Ok(Object::Dictionary(cat)) = doc_unassoc.get_object_mut(root_id2) {
        cat.set("AF", Object::Array(vec![Object::Reference(supp_fs_id)]));
    }
    let mut unassoc_bytes = Vec::new();
    doc_unassoc
        .save_to(&mut unassoc_bytes)
        .expect("save unassociated PDF");
    assert_has_code(&unassoc_bytes, "PDFA3-6.8-HYBRID-DIVERGENCE");

    // 3. PDF without any embedded XML validated with `.pdfa(false)` must emit a Fatal violation
    let mut doc_empty = Document::with_version("1.7");
    let pages_id = doc_empty.add_object(dictionary! {
        "Type" => "Pages",
        "Kids" => vec![],
        "Count" => 0,
    });
    let catalog_id = doc_empty.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    doc_empty.trailer.set("Root", catalog_id);
    let mut no_xml_pdf = Vec::new();
    doc_empty
        .save_to(&mut no_xml_pdf)
        .expect("save XML-less PDF");
    let rep_no_pdfa = validate_bytes(&no_xml_pdf, &ValidateOptions::default().pdfa(false));
    assert!(!rep_no_pdfa.is_valid);
    assert!(
        rep_no_pdfa
            .violations
            .iter()
            .any(|v| v.code == "PDFA3-6.8-NO-INVOICE-PAYLOAD"),
        "Expected PDFA3-6.8-NO-INVOICE-PAYLOAD when run_pdfa=false on XML-less PDF, got: {:?}",
        rep_no_pdfa.violations
    );
}

#[test]
fn pdfa3_utf8_multibyte_attachment_filenames_never_panic() {
    let base_pdf = load_fixture_bytes("01_standard_b2b_ro.pdf");

    // 1. Keep primary efactura.xml + add /Supplement with filename "șabc"
    //    (UTF-8 [0xC8, 0x99, b'a', b'b', b'c'], len=5 -> len-4=1 lands mid-codepoint)
    let mut doc_supp = Document::load_mem(&base_pdf).expect("parse base PDF");
    let utf16_sabc = vec![0xFE, 0xFF, 0x02, 0x19, 0x00, b'a', 0x00, b'b', 0x00, b'c'];
    let txt_stream_id = doc_supp.add_object(Stream::new(
        dictionary! {
            "Type" => "EmbeddedFile",
            "Subtype" => "text/plain",
        },
        b"Deviz anexa".to_vec(),
    ));
    let supp_fs_id = doc_supp.add_object(dictionary! {
        "Type" => "Filespec",
        "F" => Object::string_literal("anexa"),
        "UF" => Object::String(utf16_sabc.clone(), lopdf::StringFormat::Hexadecimal),
        "AFRelationship" => "Supplement",
        "EF" => dictionary! {
            "F" => txt_stream_id,
            "UF" => txt_stream_id,
        },
    });
    for obj in doc_supp.objects.values_mut() {
        if let Object::Dictionary(d) = obj
            && let Ok(Object::Array(names)) = d.get_mut(b"Names")
            && names.len() == 2
        {
            names.push(Object::String(
                utf16_sabc.clone(),
                lopdf::StringFormat::Hexadecimal,
            ));
            names.push(Object::Reference(supp_fs_id));
        }
    }
    let root_id = doc_supp
        .trailer
        .get(b"Root")
        .and_then(Object::as_reference)
        .expect("Catalog ref");
    if let Ok(Object::Dictionary(cat)) = doc_supp.get_object_mut(root_id)
        && let Ok(Object::Array(af)) = cat.get_mut(b"AF")
    {
        af.push(Object::Reference(supp_fs_id));
    }
    let mut supp_pdf_bytes = Vec::new();
    doc_supp
        .save_to(&mut supp_pdf_bytes)
        .expect("save supplement PDF");

    let rep_supp = validate_bytes(
        &supp_pdf_bytes,
        &ValidateOptions::default().deny_warnings(true),
    );
    assert!(
        rep_supp.is_valid && rep_supp.violations.is_empty(),
        "Multi-byte UTF-8 /Supplement filename 'șabc' must validate cleanly: {:?}",
        rep_supp.violations
    );

    // 2. Rename primary invoice UF to "factură.xml" alongside "șabc" so fallback .xml
    //    scanner also exercises has_xml_ext on both multi-byte filenames without panicking
    let mut doc_fallback = Document::load_mem(&supp_pdf_bytes).expect("parse supp PDF");
    let utf16_factura_xml: Vec<u8> = [0xFE, 0xFF]
        .into_iter()
        .chain("factură.xml".encode_utf16().flat_map(u16::to_be_bytes))
        .collect();
    for obj in doc_fallback.objects.values_mut() {
        if let Object::Dictionary(d) = obj
            && d.get(b"Type").and_then(Object::as_name).ok() == Some(b"Filespec")
            && d.get(b"F")
                .and_then(Object::as_str)
                .is_ok_and(|s| s == b"efactura.xml")
        {
            d.set("F", Object::string_literal("factura.xml"));
            d.set(
                "UF",
                Object::String(utf16_factura_xml.clone(), lopdf::StringFormat::Hexadecimal),
            );
        }
        if let Object::Dictionary(d) = obj
            && let Ok(Object::Array(names)) = d.get_mut(b"Names")
            && names.len() == 4
        {
            names[0] = Object::String(utf16_factura_xml.clone(), lopdf::StringFormat::Hexadecimal);
        }
    }
    let mut fallback_bytes = Vec::new();
    doc_fallback
        .save_to(&mut fallback_bytes)
        .expect("save fallback PDF");

    let extracted = extract_xml_from_pdf(&fallback_bytes).expect("extract XML without panicking");
    assert_eq!(extracted.filename, "factură.xml");
    assert!(
        std::str::from_utf8(&extracted.xml_bytes).is_ok_and(|s| s.contains("RO-2026-0042")),
        "Unexpected extracted XML: {:?}",
        String::from_utf8_lossy(&extracted.xml_bytes)
    );
    let rep_fb = validate_bytes(&fallback_bytes, &ValidateOptions::default());
    assert!(
        rep_fb
            .violations
            .iter()
            .any(|v| v.code == "PDFA3-6.8-NO-INVOICE-PAYLOAD")
    );
}

#[test]
fn pdfa3_single_stream_extraction_skips_unrelated_attachments_and_guards_xref() {
    let base_pdf = load_fixture_bytes("01_standard_b2b_ro.pdf");

    // 1. Attach a /Supplement stream with corrupt /FlateDecode bytes in /Names/EmbeddedFiles:
    //    `extract_xml_from_pdf` must inflate ONLY `efactura.xml` and ignore unrelated attachments
    let mut doc = Document::load_mem(&base_pdf).expect("parse base PDF");
    let bad_flate_id = doc.add_object(Stream::new(
        dictionary! {
            "Type" => "EmbeddedFile",
            "Subtype" => "application/octet-stream",
            "Filter" => "FlateDecode",
        },
        b"not-valid-zlib-stream-bytes".to_vec(),
    ));
    let supp_fs_id = doc.add_object(dictionary! {
        "Type" => "Filespec",
        "F" => Object::string_literal("corrupt_supplement.bin"),
        "UF" => Object::string_literal("corrupt_supplement.bin"),
        "AFRelationship" => "Supplement",
        "EF" => dictionary! { "F" => bad_flate_id, "UF" => bad_flate_id },
    });
    for obj in doc.objects.values_mut() {
        if let Object::Dictionary(d) = obj
            && let Ok(Object::Array(names)) = d.get_mut(b"Names")
            && names.len() == 2
        {
            names.push(Object::string_literal("corrupt_supplement.bin"));
            names.push(Object::Reference(supp_fs_id));
        }
    }
    let mut pdf_with_bad_supp = Vec::new();
    doc.save_to(&mut pdf_with_bad_supp)
        .expect("save mutated PDF");

    let ext = extract_xml_from_pdf(&pdf_with_bad_supp)
        .expect("extract_xml_from_pdf must skip inflating unrelated attachments");
    assert_eq!(ext.filename, "efactura.xml");
    assert!(std::str::from_utf8(&ext.xml_bytes).is_ok_and(|s| s.contains("RO-2026-0042")));

    // 2. Malicious XRef stream dictionaries must be rejected pre-load
    let bad_xref_pdf =
        b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n1 0 obj\n<< /Type /XRef /W [99 99 99] /Size 999999999 >>\nstream\nendstream\n%%EOF";
    assert!(extract_xml_from_pdf(bad_xref_pdf).is_err());
    assert_has_code(bad_xref_pdf, "PDFA3-6.1-PARSE");

    for xref_dict in [
        "<< /Type /XRef /Length 0 /W [999999999 99 99] /Size 999999999 >>",
        "<< /Type /XRef /Length 0 /W [0 0 0] /Size 999999999 >>",
    ] {
        let prefix = b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n";
        let xref_offset = prefix.len();
        let mut linked_bomb = prefix.to_vec();
        linked_bomb.extend_from_slice(
            format!(
                "1 0 obj\n{xref_dict}\nstream\nendstream\nendobj\nstartxref\n{xref_offset}\n%%EOF"
            )
            .as_bytes(),
        );
        assert!(extract_xml_from_pdf(&linked_bomb).is_err());
        assert_has_code(&linked_bomb, "PDFA3-6.1-PARSE");
    }

    // 3. Zero-allocation LocalizedDisplay matches .message() on both XmlDecodeError and PdfExtractError
    let xml_err = cius_ro::decode_xml_bytes(&[0xFF, 0xFE, 0x00]).unwrap_err();
    assert_eq!(
        xml_err.display(Locale::Ro).to_string(),
        xml_err.message(Locale::Ro)
    );
    let pdf_err = extract_xml_from_pdf(bad_xref_pdf).unwrap_err();
    assert_eq!(
        pdf_err.display(Locale::Ro).to_string(),
        pdf_err.message(Locale::Ro)
    );
}

#[test]
fn pdfa3_single_pass_objstm_and_facturx_hybrid_association() {
    let base_pdf = load_fixture_bytes("01_standard_b2b_ro.pdf");

    // 1. Single-pass /ObjStm + /XRef stream container (PDF 1.5+ modern save)
    let mut modern_doc = Document::load_mem(&base_pdf).expect("parse base PDF for modern save");
    let mut modern_pdf = Vec::new();
    modern_doc
        .save_modern(&mut modern_pdf)
        .expect("save PDF with /ObjStm and /XRef streams");
    let modern_ext = extract_xml_from_pdf(&modern_pdf).expect("extract from /ObjStm PDF");
    assert_eq!(modern_ext.filename, "efactura.xml");
    let modern_rep = validate_bytes(&modern_pdf, &ValidateOptions::default().deny_warnings(true));
    assert!(
        modern_rep.is_valid && modern_rep.violations.is_empty(),
        "Single-pass /ObjStm PDF/A-3 validation must pass cleanly: {:?}",
        modern_rep.violations
    );

    // 2. Hybrid factur-x.xml attachment association & XMP fx:DocumentFileName divergence check
    let mut fx_doc = Document::load_mem(&base_pdf).expect("parse base PDF for factur-x.xml");
    for obj in fx_doc.objects.values_mut() {
        match obj {
            Object::Dictionary(d) => {
                if d.get(b"F")
                    .and_then(Object::as_str)
                    .is_ok_and(|s| s == b"efactura.xml")
                {
                    d.set("F", Object::string_literal("factur-x.xml"));
                    d.set("UF", Object::string_literal("factur-x.xml"));
                }
                if let Ok(Object::Array(names)) = d.get_mut(b"Names")
                    && names
                        .first()
                        .is_some_and(|o| o.as_str().is_ok_and(|s| s == b"efactura.xml"))
                {
                    names[0] = Object::string_literal("factur-x.xml");
                }
            }
            Object::Stream(s) => {
                if s.dict
                    .get(b"Subtype")
                    .and_then(Object::as_name)
                    .is_ok_and(|n| n == b"XML")
                    && let Ok(xmp_str) = std::str::from_utf8(&s.content)
                {
                    s.set_content(xmp_str.replace("efactura.xml", "factur-x.xml").into_bytes());
                }
            }
            _ => {}
        }
    }
    let mut fx_pdf = Vec::new();
    fx_doc.save_to(&mut fx_pdf).expect("save factur-x.xml PDF");
    let fx_rep = validate_bytes(&fx_pdf, &ValidateOptions::default().deny_warnings(true));
    assert!(
        fx_rep.is_valid && fx_rep.violations.is_empty(),
        "Compliant factur-x.xml hybrid PDF/A-3 must pass without zugferd re-parse: {:?}",
        fx_rep.violations
    );

    // Mismatched XMP fx:DocumentFileName ('efactura.xml') vs attachment ('factur-x.xml') -> PDFA3-6.8-HYBRID-DIVERGENCE
    for obj in fx_doc.objects.values_mut() {
        if let Object::Stream(s) = obj
            && s.dict
                .get(b"Subtype")
                .and_then(Object::as_name)
                .is_ok_and(|n| n == b"XML")
            && let Ok(xmp_str) = std::str::from_utf8(&s.content)
        {
            s.set_content(xmp_str.replace("factur-x.xml", "efactura.xml").into_bytes());
        }
    }
    let mut fx_mismatch_pdf = Vec::new();
    fx_doc
        .save_to(&mut fx_mismatch_pdf)
        .expect("save mismatched factur-x PDF");
    assert_has_code(&fx_mismatch_pdf, "PDFA3-6.8-HYBRID-DIVERGENCE");
}
