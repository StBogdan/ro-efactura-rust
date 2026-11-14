//! Comprehensive golden-corpus, single-fault mutation, XSD/`xmllint`, PDF/A-3, SVRL,
//! and bilingual (`en` / `ro`) verification suite for `cius-ro`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use cius_ro::{
    extract_xml_from_pdf, render_rules_markdown, validate_bytes, Locale, Severity, ValidateOptions,
    XmlSyntax,
};
use lopdf::{Dictionary, Document, Object};

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn schemas_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("schemas/xsd")
}

#[test]
fn rules_catalog_markdown_matches_generated_docs() {
    let generated_en = render_rules_markdown(Locale::En);
    let generated_ro = render_rules_markdown(Locale::Ro);
    assert!(generated_en.contains("BR-RO-001") && generated_ro.contains("BR-RO-001"));

    if let Some(root) = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
    {
        let docs_dir = root.join("docs");
        if docs_dir.join("RULES.en.md").exists() && docs_dir.join("RULES.ro.md").exists() {
            let en_on_disk =
                fs::read_to_string(docs_dir.join("RULES.en.md")).expect("docs/RULES.en.md");
            let ro_on_disk =
                fs::read_to_string(docs_dir.join("RULES.ro.md")).expect("docs/RULES.ro.md");
            assert_eq!(en_on_disk, generated_en);
            assert_eq!(ro_on_disk, generated_ro);
        }
    }
}

fn load_fixture_bytes(name: &str) -> Vec<u8> {
    fs::read(fixtures_dir().join(name)).unwrap_or_else(|e| panic!("missing fixture {name}: {e}"))
}

fn load_fixture_str(name: &str) -> String {
    String::from_utf8(load_fixture_bytes(name)).expect("UTF-8 XML fixture")
}

fn assert_has_code(xml_or_pdf: &[u8], expected_code: &str) {
    let report = validate_bytes(xml_or_pdf, &ValidateOptions::default());
    assert!(
        report.violations.iter().any(|v| v.code == expected_code),
        "Expected violation code '{expected_code}', got: {:?}",
        report
            .violations
            .iter()
            .map(|v| (&v.code, &v.rule_sub_id, v.severity, &v.message))
            .collect::<Vec<_>>()
    );
}

#[test]
fn golden_compliant_corpus_passes_with_zero_violations_and_warnings() {
    let compliant_files = [
        "01_standard_b2b_ro.xml",
        "01_standard_b2b_ro.pdf",
        "01_standard_b2b_ro_ubl.xml",
        "01_standard_b2b_ro_ubl.pdf",
        "02_public_sector_ro.xml",
        "02_public_sector_ro.pdf",
        "03_small_business_exempt_ro.xml",
        "03_small_business_exempt_ro.pdf",
        "04_reverse_charge_ro.xml",
        "04_reverse_charge_ro.pdf",
        "05_credit_note_ro.xml",
        "05_credit_note_ro.pdf",
        "06_cii_d16b_compliant_ro.xml",
    ];

    for locale in [Locale::En, Locale::Ro] {
        let opts = ValidateOptions {
            locale,
            deny_warnings: true,
            include_svrl: true,
            ..ValidateOptions::default()
        };

        for filename in compliant_files {
            let bytes = load_fixture_bytes(filename);
            let report = validate_bytes(&bytes, &opts);
            assert!(
                report.is_valid && report.violations.is_empty(),
                "Golden fixture '{filename}' failed under locale {locale:?}: {:?}",
                report.violations
            );
            assert_eq!(report.recommendation, "accept");
            let svrl = report.svrl_xml.as_deref().expect("SVRL generated");
            roxmltree::Document::parse(svrl).expect("SVRL XML must be well-formed");
            assert!(
                !svrl.contains("failed-assert"),
                "Compliant SVRL must have 0 failed-asserts"
            );
        }
    }
}

#[test]
fn b2c_cnp_and_anaf_oug_69_2024_placeholder_compliance() {
    let base = load_fixture_str("01_standard_b2b_ro_ubl.xml");

    // 1. OUG 69/2024 B2C 13-zero placeholder (no VAT PartyTaxScheme on natural-person buyer)
    let b2c_zeros = base
        .replace(
            "<cac:PartyTaxScheme>\n        <cbc:CompanyID>RO98765438</cbc:CompanyID>\n        <cac:TaxScheme><cbc:ID>VAT</cbc:ID></cac:TaxScheme>\n      </cac:PartyTaxScheme>\n      ",
            "",
        )
        .replace("J12/890/2020", "0000000000000");

    let opts = ValidateOptions {
        deny_warnings: true,
        ..ValidateOptions::default()
    };
    let rep_zeros = validate_bytes(b2c_zeros.as_bytes(), &opts);
    assert!(
        rep_zeros.is_valid && rep_zeros.violations.is_empty(),
        "OUG 69/2024 B2C 0000000000000 must pass cleanly: {:?}",
        rep_zeros.violations
    );

    // 2. Valid 13-digit Romanian CNP (1900101410011)
    let b2c_valid_cnp = b2c_zeros.replace("0000000000000", "1900101410011");
    let rep_valid_cnp = validate_bytes(b2c_valid_cnp.as_bytes(), &opts);
    assert!(
        rep_valid_cnp.is_valid && rep_valid_cnp.violations.is_empty(),
        "Valid 13-digit CNP must pass cleanly: {:?}",
        rep_valid_cnp.violations
    );

    // 3. Invalid 13-digit CNP check digit (1900101410019) -> BR-RO-CNP warning
    let b2c_bad_cnp = b2c_zeros.replace("0000000000000", "1900101410019");
    let rep_bad_cnp_default = validate_bytes(b2c_bad_cnp.as_bytes(), &ValidateOptions::default());
    assert!(rep_bad_cnp_default.is_valid); // Warning by default
    assert!(rep_bad_cnp_default
        .violations
        .iter()
        .any(|v| v.code == "BR-RO-CNP" && v.severity == Severity::Warning));
    let rep_bad_cnp_strict = validate_bytes(b2c_bad_cnp.as_bytes(), &opts);
    assert!(!rep_bad_cnp_strict.is_valid);
}

#[test]
fn xmllint_official_schema_cross_verification() {
    let xmllint = Path::new("/usr/bin/xmllint");
    if !xmllint.exists() {
        return;
    }

    let ubl_inv_xsd = schemas_dir().join("ubl_2_1/maindoc/UBL-Invoice-2.1.xsd");
    let ubl_cn_xsd = schemas_dir().join("ubl_2_1/maindoc/UBL-CreditNote-2.1.xsd");
    let cii_xsd = schemas_dir().join("cii_d16b/CrossIndustryInvoice_100pD16B.xsd");

    let cases = [
        ("01_standard_b2b_ro.xml", &ubl_inv_xsd),
        ("01_standard_b2b_ro_ubl.xml", &ubl_inv_xsd),
        ("02_public_sector_ro.xml", &ubl_inv_xsd),
        ("03_small_business_exempt_ro.xml", &ubl_inv_xsd),
        ("04_reverse_charge_ro.xml", &ubl_inv_xsd),
        ("05_credit_note_ro.xml", &ubl_cn_xsd),
        ("06_cii_d16b_compliant_ro.xml", &cii_xsd),
    ];

    for (xml_name, xsd_path) in cases {
        let xml_path = fixtures_dir().join(xml_name);
        let out = Command::new(xmllint)
            .arg("--noout")
            .arg("--schema")
            .arg(xsd_path)
            .arg(&xml_path)
            .output()
            .expect("failed spawning xmllint");
        assert!(
            out.status.success(),
            "xmllint rejected golden fixture {xml_name}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

#[test]
fn faulty_samples_emit_exact_expected_codes() {
    // 1. faulty_08_ro_invalid_sector_cui.xml
    let faulty_08 = load_fixture_bytes("faulty_08_ro_invalid_sector_cui.xml");
    let rep_08 = validate_bytes(&faulty_08, &ValidateOptions::default());
    assert!(!rep_08.is_valid);
    for expected in ["BR-RO-010", "BR-RO-100", "BR-RO-110", "BR-RO-CUI"] {
        assert!(
            rep_08.violations.iter().any(|v| v.code == expected),
            "Missing expected code {expected} in faulty_08: {:?}",
            rep_08.violations
        );
    }

    // 2. 07_invalid_invoice_ro.xml (non-CIUS-RO CII D16B with RO parties + broken math)
    let faulty_07 = load_fixture_bytes("07_invalid_invoice_ro.xml");
    let rep_strict = validate_bytes(&faulty_07, &ValidateOptions::default());
    assert!(!rep_strict.is_valid);
    assert_eq!(rep_strict.detected_syntax, Some(XmlSyntax::CiiD16B));
    assert!(rep_strict
        .violations
        .iter()
        .any(|v| v.code == "BR-RO-001" && v.severity == Severity::Fatal));

    let rep_lenient = validate_bytes(
        &faulty_07,
        &ValidateOptions {
            strict_cius_ro_customization: false,
            ..ValidateOptions::default()
        },
    );
    assert!(!rep_lenient.is_valid); // Still invalid due to EN 16931 math / missing fields
    assert!(rep_lenient
        .violations
        .iter()
        .any(|v| v.code == "BR-RO-001" && v.severity == Severity::Warning));
}

#[test]
fn ubl_br_ro_single_fault_mutations() {
    let ubl = load_fixture_str("01_standard_b2b_ro_ubl.xml");
    let cn = load_fixture_str("05_credit_note_ro.xml");

    // BR-RO-001: wrong CustomizationID
    assert_has_code(
        ubl.replace("CIUS-RO:1.0.1", "CIUS-RO:0.9.0").as_bytes(),
        "BR-RO-001",
    );

    // BR-RO-010: invoice number without any digit
    assert_has_code(
        ubl.replace("EFACT-RO-2026-0042", "EFACT-RO-ABCD")
            .as_bytes(),
        "BR-RO-010",
    );

    // BR-RO-020: disallowed InvoiceTypeCode on UBL Invoice (383) and CreditNote (380)
    assert_has_code(
        ubl.replace(
            "<cbc:InvoiceTypeCode>380</cbc:InvoiceTypeCode>",
            "<cbc:InvoiceTypeCode>383</cbc:InvoiceTypeCode>",
        )
        .as_bytes(),
        "BR-RO-020",
    );
    assert_has_code(
        cn.replace(
            "<cbc:CreditNoteTypeCode>381</cbc:CreditNoteTypeCode>",
            "<cbc:CreditNoteTypeCode>380</cbc:CreditNoteTypeCode>",
        )
        .as_bytes(),
        "BR-RO-020",
    );

    // BR-RO-030: foreign DocumentCurrencyCode="EUR" without TaxCurrencyCode="RON" for RO Seller
    assert_has_code(
        ubl.replace(
            "<cbc:DocumentCurrencyCode>RON</cbc:DocumentCurrencyCode>",
            "<cbc:DocumentCurrencyCode>EUR</cbc:DocumentCurrencyCode>",
        )
        .replace("currencyID=\"RON\"", "currencyID=\"EUR\"")
        .as_bytes(),
        "BR-RO-030",
    );

    // BR-RO-040: InvoicePeriod/DescriptionCode (BT-8) outside {3, 35, 432}
    assert_has_code(
        ubl.replace(
            "  <cac:AccountingSupplierParty>",
            "  <cac:InvoicePeriod><cbc:DescriptionCode>99</cbc:DescriptionCode></cac:InvoicePeriod>\n  <cac:AccountingSupplierParty>",
        )
        .as_bytes(),
        "BR-RO-040",
    );

    // BR-RO-050: missing mandatory PaymentMeans / PaymentMeansCode (BT-81)
    let ubl_no_pm = ubl.replace(
        "  <cac:PaymentMeans>\n    <cbc:PaymentMeansCode>58</cbc:PaymentMeansCode>\n    <cac:PayeeFinancialAccount>\n      <cbc:ID>RO49AAAA1B31007593840000</cbc:ID>\n      <cac:FinancialInstitutionBranch><cbc:ID>BTRLRO22</cbc:ID></cac:FinancialInstitutionBranch>\n    </cac:PayeeFinancialAccount>\n  </cac:PaymentMeans>\n",
        "",
    );
    assert_has_code(ubl_no_pm.as_bytes(), "BR-RO-050");

    // BR-RO-065: Seller missing tax registration & VAT identifier when active VAT category is used
    let ubl_no_seller_tax = ubl.replace(
        "      <cac:PartyTaxScheme>\n        <cbc:CompanyID>RO45678918</cbc:CompanyID>\n        <cac:TaxScheme><cbc:ID>VAT</cbc:ID></cac:TaxScheme>\n      </cac:PartyTaxScheme>\n",
        "",
    );
    assert_has_code(ubl_no_seller_tax.as_bytes(), "BR-RO-065");

    // BR-RO-A020: > 20 Notes
    let many_notes: String = (0..21)
        .map(|i| format!("  <cbc:Note>Nota {i}</cbc:Note>\n"))
        .collect();
    let ubl_21_notes = ubl.replace(
        "<cbc:DocumentCurrencyCode>RON</cbc:DocumentCurrencyCode>",
        &format!("{many_notes}  <cbc:DocumentCurrencyCode>RON</cbc:DocumentCurrencyCode>"),
    );
    assert_has_code(ubl_21_notes.as_bytes(), "BR-RO-A020");

    // BR-RO-A050 (sub-ID BR-RO-A051): > 50 AdditionalDocumentReference elements
    let many_docs: String = (0..51)
        .map(|i| {
            format!(
                "  <cac:AdditionalDocumentReference><cbc:ID>DOC-{i}</cbc:ID></cac:AdditionalDocumentReference>\n"
            )
        })
        .collect();
    let ubl_51_docs = ubl.replace(
        "  <cac:AccountingSupplierParty>",
        &format!("{many_docs}  <cac:AccountingSupplierParty>"),
    );
    assert_has_code(ubl_51_docs.as_bytes(), "BR-RO-A050");

    // BR-RO-080 & BR-RO-090: missing Seller street / Buyer city
    assert_has_code(
        ubl.replace("<cbc:StreetName>Calea Dorobanți 42</cbc:StreetName>", "")
            .as_bytes(),
        "BR-RO-080",
    );
    assert_has_code(
        ubl.replace("<cbc:CityName>Cluj-Napoca</cbc:CityName>", "")
            .as_bytes(),
        "BR-RO-090",
    );

    // BR-RO-110 with fuzzy County normalization hint ("Cluj" -> "RO-CJ")
    let bad_county = ubl.replace(
        "<cbc:CountrySubentity>RO-CJ</cbc:CountrySubentity>",
        "<cbc:CountrySubentity>Cluj</cbc:CountrySubentity>",
    );
    let rep_county = validate_bytes(bad_county.as_bytes(), &ValidateOptions::default());
    let v_110 = rep_county
        .violations
        .iter()
        .find(|v| v.code == "BR-RO-110")
        .expect("BR-RO-110");
    assert!(
        v_110.hint.as_deref().is_some_and(|h| h.contains("RO-CJ")),
        "Expected fuzzy hint suggesting RO-CJ, got: {:?}",
        v_110.hint
    );

    // BR-RO-100 with fuzzy Bucharest Sector normalization hint ("Sector 1" -> "SECTOR1")
    let bad_sector = ubl.replace(
        "<cbc:CityName>SECTOR1</cbc:CityName>",
        "<cbc:CityName>Sector 1</cbc:CityName>",
    );
    let rep_sector = validate_bytes(bad_sector.as_bytes(), &ValidateOptions::default());
    let v_100 = rep_sector
        .violations
        .iter()
        .find(|v| v.code == "BR-RO-100")
        .expect("BR-RO-100");
    assert!(
        v_100.hint.as_deref().is_some_and(|h| h.contains("SECTOR1")),
        "Expected fuzzy hint suggesting SECTOR1, got: {:?}",
        v_100.hint
    );

    // BR-RO-120: Buyer missing both VAT ID and Legal Registration ID
    let no_buyer_ids = ubl
        .replace(
            "      <cac:PartyTaxScheme>\n        <cbc:CompanyID>RO98765438</cbc:CompanyID>\n        <cac:TaxScheme><cbc:ID>VAT</cbc:ID></cac:TaxScheme>\n      </cac:PartyTaxScheme>\n",
            "",
        )
        .replace("        <cbc:CompanyID>J12/890/2020</cbc:CompanyID>\n", "");
    assert_has_code(no_buyer_ids.as_bytes(), "BR-RO-120");

    // BR-RO-160 & BR-RO-200: TaxRepresentativeParty and Delivery in RO-B without SECTOR1..SECTOR6
    let with_tax_rep_and_deliv = ubl.replace(
        "  <cac:PaymentMeans>",
        "  <cac:TaxRepresentativeParty>\n\
             <cac:PartyName><cbc:Name>Rep Fiscal SRL</cbc:Name></cac:PartyName>\n\
             <cac:PostalAddress>\n\
               <cbc:StreetName>Str. Victoriei 1</cbc:StreetName>\n\
               <cbc:CityName>Bucuresti</cbc:CityName>\n\
               <cbc:CountrySubentity>RO-B</cbc:CountrySubentity>\n\
               <cac:Country><cbc:IdentificationCode>RO</cbc:IdentificationCode></cac:Country>\n\
             </cac:PostalAddress>\n\
             <cac:PartyTaxScheme><cbc:CompanyID>RO45678918</cbc:CompanyID><cac:TaxScheme><cbc:ID>VAT</cbc:ID></cac:TaxScheme></cac:PartyTaxScheme>\n\
           </cac:TaxRepresentativeParty>\n\
           <cac:Delivery>\n\
             <cac:DeliveryLocation>\n\
               <cac:Address>\n\
                 <cbc:StreetName>Bd. Unirii 10</cbc:StreetName>\n\
                 <cbc:CityName>Bucuresti</cbc:CityName>\n\
                 <cbc:CountrySubentity>RO-B</cbc:CountrySubentity>\n\
                 <cac:Country><cbc:IdentificationCode>RO</cbc:IdentificationCode></cac:Country>\n\
               </cac:Address>\n\
             </cac:DeliveryLocation>\n\
           </cac:Delivery>\n  <cac:PaymentMeans>",
    );
    assert_has_code(with_tax_rep_and_deliv.as_bytes(), "BR-RO-160");
    assert_has_code(with_tax_rep_and_deliv.as_bytes(), "BR-RO-200");

    // BR-RO-170 & BR-RO-210: TaxRepresentativeParty and Delivery with invalid RO county codes
    let bad_tax_rep_deliv_county = with_tax_rep_and_deliv.replace(
        "<cbc:CountrySubentity>RO-B</cbc:CountrySubentity>",
        "<cbc:CountrySubentity>RO-XX</cbc:CountrySubentity>",
    );
    assert_has_code(bad_tax_rep_deliv_county.as_bytes(), "BR-RO-170");
    assert_has_code(bad_tax_rep_deliv_county.as_bytes(), "BR-RO-210");

    // BR-RO-DT001: timezone suffix on IssueDate (valid XSD xs:date, strictly prohibited by ANAF!)
    assert_has_code(
        ubl.replace(
            "<cbc:IssueDate>2026-10-15</cbc:IssueDate>",
            "<cbc:IssueDate>2026-10-15Z</cbc:IssueDate>",
        )
        .as_bytes(),
        "BR-RO-DT001",
    );

    // BR-RO-L100 & BR-RO-L300: length caps
    let long_item_name = "A".repeat(101);
    assert_has_code(
        ubl.replace(
            "Servicii de consultanță IT și integrare ANAF RO e-Factura (CIUS-RO)",
            &long_item_name,
        )
        .as_bytes(),
        "BR-RO-L100",
    );
    let long_pt_note = "P".repeat(301);
    assert_has_code(
        ubl.replace(
            "Plata în termen de 30 de zile de la data emiterii facturii",
            &long_pt_note,
        )
        .as_bytes(),
        "BR-RO-L300",
    );

    // BR-RO-Z2: 3 decimals on monetary total fails, while 4 decimals on PriceAmount (BT-146) passes!
    assert_has_code(
        ubl.replace(
            "<cbc:LineExtensionAmount currencyID=\"RON\">4500.00</cbc:LineExtensionAmount>",
            "<cbc:LineExtensionAmount currencyID=\"RON\">4500.000</cbc:LineExtensionAmount>",
        )
        .as_bytes(),
        "BR-RO-Z2",
    );
    let price_4_decimals = ubl.replace(
        "<cbc:PriceAmount currencyID=\"RON\">4500.00</cbc:PriceAmount>",
        "<cbc:PriceAmount currencyID=\"RON\">4500.0000</cbc:PriceAmount>",
    );
    let rep_price4 = validate_bytes(price_4_decimals.as_bytes(), &ValidateOptions::default());
    assert!(
        rep_price4.is_valid && rep_price4.violations.is_empty(),
        "BT-146 PriceAmount with 4 decimals must pass BR-RO-Z2: {:?}",
        rep_price4.violations
    );

    // BR-27: negative PriceAmount emitted once (deduplicated across EN 16931 & CIUS-RO)
    let neg_price = ubl.replace(
        "<cbc:PriceAmount currencyID=\"RON\">4500.00</cbc:PriceAmount>",
        "<cbc:PriceAmount currencyID=\"RON\">-4500.00</cbc:PriceAmount>",
    );
    let rep_neg = validate_bytes(neg_price.as_bytes(), &ValidateOptions::default());
    let br27_count = rep_neg
        .violations
        .iter()
        .filter(|v| v.code == "BR-27")
        .count();
    assert_eq!(br27_count, 1, "BR-27 must appear exactly once");

    // cius-ro-id checksum warnings: CUI, ONRC, IBAN
    assert_has_code(
        ubl.replace("RO45678918", "RO45678919").as_bytes(),
        "BR-RO-CUI",
    );
    assert_has_code(
        ubl.replace("J40/1234/2022", "J99/1234/2022").as_bytes(),
        "BR-RO-ONRC",
    );
    assert_has_code(
        ubl.replace("RO49AAAA1B31007593840000", "RO00AAAA1B31007593840000")
            .as_bytes(),
        "BR-RO-IBAN",
    );

    // XXE & W3C XSD 1.1 breaches
    let xxe_xml = ubl.replacen(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>",
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE Invoice [ <!ENTITY xxe \"boom\"> ]>",
        1,
    );
    assert_has_code(xxe_xml.as_bytes(), "XSD-XXE-PROHIBITED");

    let bad_xsd_xml = ubl.replace(
        "<cbc:ID>EFACT-RO-2026-0042</cbc:ID>",
        "<cbc:ID>EFACT-RO-2026-0042</cbc:ID>\n  <cbc:IllegalNonExistentElement>1</cbc:IllegalNonExistentElement>",
    );
    assert_has_code(bad_xsd_xml.as_bytes(), "XSD-SCHEMA-001");
}

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
fn bilingual_ro_vs_en_and_svrl_report_generation() {
    let faulty = load_fixture_bytes("faulty_08_ro_invalid_sector_cui.xml");

    let rep_en = validate_bytes(
        &faulty,
        &ValidateOptions {
            locale: Locale::En,
            include_svrl: true,
            ..ValidateOptions::default()
        },
    );
    let rep_ro = validate_bytes(
        &faulty,
        &ValidateOptions {
            locale: Locale::Ro,
            include_svrl: true,
            ..ValidateOptions::default()
        },
    );

    let v100_en = rep_en
        .violations
        .iter()
        .find(|v| v.code == "BR-RO-100")
        .expect("EN BR-RO-100");
    let v100_ro = rep_ro
        .violations
        .iter()
        .find(|v| v.code == "BR-RO-100")
        .expect("RO BR-RO-100");

    assert_ne!(v100_en.message, v100_ro.message);
    assert!(v100_en.message.contains("SECTOR1"));
    assert!(v100_ro.message.contains("SECTOR1"));
    assert!(v100_ro.message.contains("RO-B"));

    let svrl = rep_ro.svrl_xml.as_deref().expect("SVRL present");
    let parsed = roxmltree::Document::parse(svrl).expect("well-formed SVRL");
    let failed_asserts = parsed
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "failed-assert")
        .count();
    assert_eq!(failed_asserts, rep_ro.violations.len());
}

#[test]
fn utf8_multibyte_boundary_bom_and_deep_xxe_safety() {
    let base = load_fixture_str("01_standard_b2b_ro_ubl.xml");

    // 1. Construct a >22 KiB compliant UBL invoice rich in Romanian multi-byte UTF-8 characters
    //    (ă, â, î, ș, ț, €) where a multi-byte sequence explicitly straddles byte index 16_384.
    let diacritic_chunk = "ăâîșțĂÂÎȘȚ€"; // 23 bytes, 11 chars (<= 300 chars per Note)
    let note_body = diacritic_chunk.repeat(24); // 264 chars, 552 bytes per Note
    let mut large_xml = String::new();
    for pad in 0..150 {
        let prefix_pad = "X".repeat(pad % 30);
        let extra_ascii = "Y".repeat(pad);
        let mut notes = format!("  <cbc:Note>{prefix_pad}{note_body}</cbc:Note>\n");
        notes.push_str(&format!("  <cbc:Note>{extra_ascii}</cbc:Note>\n"));
        for _ in 2..20 {
            notes.push_str(&format!("  <cbc:Note>{note_body}</cbc:Note>\n"));
        }
        let mut add_docs = String::new();
        for idx in 0..25 {
            add_docs.push_str(&format!(
                "  <cac:AdditionalDocumentReference><cbc:ID>ANEXA-{idx}</cbc:ID><cbc:DocumentDescription>{diacritic_chunk} anexă justificativă {idx}</cbc:DocumentDescription></cac:AdditionalDocumentReference>\n"
            ));
        }
        let candidate = base
            .replace(
                "  <cbc:DocumentCurrencyCode>RON</cbc:DocumentCurrencyCode>",
                &format!("{notes}  <cbc:DocumentCurrencyCode>RON</cbc:DocumentCurrencyCode>"),
            )
            .replace(
                "  <cac:AccountingSupplierParty>",
                &format!("{add_docs}  <cac:AccountingSupplierParty>"),
            );
        assert!(candidate.len() > 20_000);
        if !candidate.is_char_boundary(16_384) {
            large_xml = candidate;
            break;
        }
    }
    assert!(
        !large_xml.is_empty() && !large_xml.is_char_boundary(16_384),
        "Must test an XML payload with a multi-byte UTF-8 char straddling byte 16_384"
    );
    let rep_large = validate_bytes(
        large_xml.as_bytes(),
        &ValidateOptions {
            deny_warnings: true,
            ..ValidateOptions::default()
        },
    );
    assert!(
        rep_large.is_valid && rep_large.violations.is_empty(),
        "Large >20 KiB multi-byte UTF-8 invoice must pass without panic or violations: {:?}",
        rep_large.violations
    );

    // 2. Deep <!DOCTYPE / <!ENTITY placed AFTER >18 KiB of XML comment padding must still be caught!
    let deep_comment = format!("<!-- {} -->\n", "padding-comment-".repeat(1200));
    assert!(deep_comment.len() > 18_000);
    let deep_xxe = base.replacen(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>",
        &format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n{deep_comment}<!DOCTYPE Invoice [ <!ENTITY xxe \"boom\"> ]>"
        ),
        1,
    );
    assert_has_code(deep_xxe.as_bytes(), "XSD-XXE-PROHIBITED");

    // 3. UTF-8 with BOM (\xEF\xBB\xBF)
    let mut utf8_bom = vec![0xEF, 0xBB, 0xBF];
    utf8_bom.extend_from_slice(base.as_bytes());
    let rep_utf8_bom = validate_bytes(&utf8_bom, &ValidateOptions::default());
    assert!(
        rep_utf8_bom.is_valid && rep_utf8_bom.violations.is_empty(),
        "UTF-8 BOM XML must pass cleanly: {:?}",
        rep_utf8_bom.violations
    );

    // 4. UTF-16LE with BOM (\xFF\xFE) and UTF-16BE with BOM (\xFE\xFF)
    let u16_src = base.replace("encoding=\"UTF-8\"", "encoding=\"UTF-16\"");
    let mut utf16le = vec![0xFF, 0xFE];
    for unit in u16_src.encode_utf16() {
        utf16le.extend_from_slice(&unit.to_le_bytes());
    }
    let rep_utf16le = validate_bytes(&utf16le, &ValidateOptions::default());
    assert!(
        rep_utf16le.is_valid && rep_utf16le.violations.is_empty(),
        "UTF-16LE BOM XML must pass cleanly: {:?}",
        rep_utf16le.violations
    );

    let mut utf16be = vec![0xFE, 0xFF];
    for unit in u16_src.encode_utf16() {
        utf16be.extend_from_slice(&unit.to_be_bytes());
    }
    let rep_utf16be = validate_bytes(&utf16be, &ValidateOptions::default());
    assert!(
        rep_utf16be.is_valid && rep_utf16be.violations.is_empty(),
        "UTF-16BE BOM XML must pass cleanly: {:?}",
        rep_utf16be.violations
    );
}

#[test]
fn xsd_boolean_1_credit_note_and_foreign_vat_edges() {
    let ubl = load_fixture_str("01_standard_b2b_ro_ubl.xml");
    let cn = load_fixture_str("05_credit_note_ro.xml");
    let eur = load_fixture_str("04_reverse_charge_ro.xml");

    // 1. XSD xs:boolean `<cbc:ChargeIndicator>1</cbc:ChargeIndicator>` with 3-decimal Amount
    //    must be classified as a Charge (BR-DEC-RO-05), not an Allowance (BR-DEC-RO-01).
    let charge_bool_1 = ubl.replace(
        "  <cac:TaxTotal>",
        "  <cac:AllowanceCharge>\n\
             <cbc:ChargeIndicator>1</cbc:ChargeIndicator>\n\
             <cbc:AllowanceChargeReason>Taxă transport</cbc:AllowanceChargeReason>\n\
             <cbc:Amount currencyID=\"RON\">10.125</cbc:Amount>\n\
             <cac:TaxCategory><cbc:ID>S</cbc:ID><cbc:Percent>19.00</cbc:Percent><cac:TaxScheme><cbc:ID>VAT</cbc:ID></cac:TaxScheme></cac:TaxCategory>\n\
           </cac:AllowanceCharge>\n  <cac:TaxTotal>",
    );
    let rep_charge1 = validate_bytes(charge_bool_1.as_bytes(), &ValidateOptions::default());
    assert!(
        rep_charge1
            .violations
            .iter()
            .any(|v| { v.code == "BR-RO-Z2" && v.rule_sub_id.as_deref() == Some("BR-DEC-RO-05") }),
        "Expected BR-DEC-RO-05 for ChargeIndicator=1, got: {:?}",
        rep_charge1.violations
    );

    // 2. CreditNote (381) PaymentDueDate with timezone -> BR-RO-DT003-CN (sub-ID BR-RO-DT003_CN)
    let cn_bad_due = cn.replace(
        "<cbc:PaymentDueDate>2026-11-24</cbc:PaymentDueDate>",
        "<cbc:PaymentDueDate>2026-11-24Z</cbc:PaymentDueDate>",
    );
    let rep_cn_due = validate_bytes(cn_bad_due.as_bytes(), &ValidateOptions::default());
    assert!(
        rep_cn_due.violations.iter().any(|v| {
            v.code == "BR-RO-DT003-CN" && v.rule_sub_id.as_deref() == Some("BR-RO-DT003_CN")
        }),
        "Expected BR-RO-DT003-CN on CreditNote, got: {:?}",
        rep_cn_due.violations
    );

    // 3. CreditNote (381) 3-decimal CreditNoteLine/LineExtensionAmount -> cac:CreditNoteLine/cbc:LineExtensionAmount
    let cn_bad_line_dec = cn.replace(
        "  <cac:CreditNoteLine>\n    <cbc:ID>1</cbc:ID>\n    <cbc:CreditedQuantity unitCode=\"C62\">1</cbc:CreditedQuantity>\n    <cbc:LineExtensionAmount currencyID=\"RON\">600.00</cbc:LineExtensionAmount>",
        "  <cac:CreditNoteLine>\n    <cbc:ID>1</cbc:ID>\n    <cbc:CreditedQuantity unitCode=\"C62\">1</cbc:CreditedQuantity>\n    <cbc:LineExtensionAmount currencyID=\"RON\">600.000</cbc:LineExtensionAmount>",
    );
    let rep_cn_dec = validate_bytes(cn_bad_line_dec.as_bytes(), &ValidateOptions::default());
    assert!(
        rep_cn_dec.violations.iter().any(|v| {
            v.code == "BR-RO-Z2"
                && v.rule_sub_id.as_deref() == Some("BR-DEC-RO-23")
                && v.location.as_deref() == Some("cac:CreditNoteLine/cbc:LineExtensionAmount")
        }),
        "Expected BR-DEC-RO-23 on cac:CreditNoteLine/cbc:LineExtensionAmount, got: {:?}",
        rep_cn_dec.violations
    );

    // 4. Foreign-currency EUR invoice with 3-decimal RON accounting VAT -> BR-DEC-RO-15
    let eur_bad_ron_vat = eur.replace(
        "<cbc:TaxAmount currencyID=\"RON\">0.00</cbc:TaxAmount>",
        "<cbc:TaxAmount currencyID=\"RON\">0.000</cbc:TaxAmount>",
    );
    let rep_eur_vat = validate_bytes(eur_bad_ron_vat.as_bytes(), &ValidateOptions::default());
    assert!(
        rep_eur_vat
            .violations
            .iter()
            .any(|v| { v.code == "BR-RO-Z2" && v.rule_sub_id.as_deref() == Some("BR-DEC-RO-15") }),
        "Expected BR-DEC-RO-15 on accounting-currency TaxAmount, got: {:?}",
        rep_eur_vat.violations
    );
}

#[test]
fn cii_d16b_br_ro_single_fault_mutations() {
    let cii = load_fixture_str("06_cii_d16b_compliant_ro.xml");

    // BR-RO-010: CII ID without digit
    assert_has_code(
        cii.replace("CII-RO-2026-0001", "CII-RO-ABCD").as_bytes(),
        "BR-RO-010",
    );

    // BR-RO-020: disallowed TypeCode
    assert_has_code(
        cii.replace(
            "<ram:TypeCode>380</ram:TypeCode>",
            "<ram:TypeCode>383</ram:TypeCode>",
        )
        .as_bytes(),
        "BR-RO-020",
    );

    // BR-RO-030: foreign currency EUR without TaxCurrencyCode="RON"
    assert_has_code(
        cii.replace(
            "<ram:InvoiceCurrencyCode>RON</ram:InvoiceCurrencyCode>",
            "<ram:InvoiceCurrencyCode>EUR</ram:InvoiceCurrencyCode>",
        )
        .replace("currencyID=\"RON\"", "currencyID=\"EUR\"")
        .as_bytes(),
        "BR-RO-030",
    );

    // BR-RO-040: ApplicableTradeTax/TimeReferenceCode outside {3, 35, 432}
    assert_has_code(
        cii.replace(
            "<ram:CategoryCode>S</ram:CategoryCode>\n                <ram:RateApplicablePercent>19.00</ram:RateApplicablePercent>",
            "<ram:CategoryCode>S</ram:CategoryCode>\n                <ram:TimeReferenceCode>99</ram:TimeReferenceCode>\n                <ram:RateApplicablePercent>19.00</ram:RateApplicablePercent>",
        )
        .as_bytes(),
        "BR-RO-040",
    );

    // BR-RO-050: missing PaymentMeans
    assert_has_code(
        cii.replace(
            "            <ram:SpecifiedTradeSettlementPaymentMeans>\n                <ram:TypeCode>58</ram:TypeCode>\n                <ram:PayeePartyCreditorFinancialAccount>\n                    <ram:IBANID>RO49AAAA1B31007593840000</ram:IBANID>\n                </ram:PayeePartyCreditorFinancialAccount>\n            </ram:SpecifiedTradeSettlementPaymentMeans>\n",
            "",
        )
        .as_bytes(),
        "BR-RO-050",
    );

    // BR-RO-100 & BR-RO-110 on CII PostalTradeAddress
    assert_has_code(
        cii.replace(
            "<ram:CityName>SECTOR1</ram:CityName>",
            "<ram:CityName>Bucuresti</ram:CityName>",
        )
        .as_bytes(),
        "BR-RO-100",
    );
    assert_has_code(
        cii.replace(
            "<ram:CountrySubDivisionName>RO-CJ</ram:CountrySubDivisionName>",
            "<ram:CountrySubDivisionName>Cluj</ram:CountrySubDivisionName>",
        )
        .as_bytes(),
        "BR-RO-110",
    );

    // BR-RO-DT001: invalid 8-digit calendar date (month 13) and hyphenated date in CII
    assert_has_code(
        cii.replace(
            "<udt:DateTimeString format=\"102\">20261015</udt:DateTimeString>",
            "<udt:DateTimeString format=\"102\">20261315</udt:DateTimeString>",
        )
        .as_bytes(),
        "BR-RO-DT001",
    );
    assert_has_code(
        cii.replace(
            "<udt:DateTimeString format=\"102\">20261015</udt:DateTimeString>",
            "<udt:DateTimeString format=\"102\">2026-10-15</udt:DateTimeString>",
        )
        .as_bytes(),
        "BR-RO-DT001",
    );

    // BR-RO-Z2 on CII GrandTotalAmount (3 decimals)
    assert_has_code(
        cii.replace(
            "<ram:GrandTotalAmount>2380.00</ram:GrandTotalAmount>",
            "<ram:GrandTotalAmount>2380.000</ram:GrandTotalAmount>",
        )
        .as_bytes(),
        "BR-RO-Z2",
    );

    // BR-RO-A020 on CII (> 20 IncludedNote elements)
    let many_cii_notes: String = (0..21)
        .map(|i| format!("        <ram:IncludedNote><ram:Content>Nota {i}</ram:Content></ram:IncludedNote>\n"))
        .collect();
    assert_has_code(
        cii.replace(
            "    </rsm:ExchangedDocument>",
            &format!("{many_cii_notes}    </rsm:ExchangedDocument>"),
        )
        .as_bytes(),
        "BR-RO-A020",
    );

    // BR-RO-L100 on CII SpecifiedTradeProduct/Name (> 100 chars)
    assert_has_code(
        cii.replace(
            "Licență software și mentenanță RO e-Factura",
            &"N".repeat(101),
        )
        .as_bytes(),
        "BR-RO-L100",
    );

    // BR-RO-160 on CII SellerTaxRepresentativeTradeParty in RO-B without SECTOR1..SECTOR6
    let cii_with_tax_rep = cii.replace(
        "        </ram:ApplicableHeaderTradeAgreement>",
        "            <ram:SellerTaxRepresentativeTradeParty>\n\
                 <ram:Name>Reprezentant Fiscal SRL</ram:Name>\n\
                 <ram:PostalTradeAddress>\n\
                     <ram:LineOne>Calea Victoriei 10</ram:LineOne>\n\
                     <ram:CityName>Bucuresti</ram:CityName>\n\
                     <ram:CountryID>RO</ram:CountryID>\n\
                     <ram:CountrySubDivisionName>RO-B</ram:CountrySubDivisionName>\n\
                 </ram:PostalTradeAddress>\n\
                 <ram:SpecifiedTaxRegistration><ram:ID schemeID=\"VA\">RO45678918</ram:ID></ram:SpecifiedTaxRegistration>\n\
             </ram:SellerTaxRepresentativeTradeParty>\n        </ram:ApplicableHeaderTradeAgreement>",
    );
    assert_has_code(cii_with_tax_rep.as_bytes(), "BR-RO-160");
}
