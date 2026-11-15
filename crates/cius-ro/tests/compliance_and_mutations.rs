//! Comprehensive golden-corpus, single-fault mutation, XSD/`xmllint`, PDF/A-3, SVRL,
//! and bilingual (`en` / `ro`) verification suite for `cius-ro`.

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(feature = "pdfa3")]
use cius_ro::extract_xml_from_pdf;
use cius_ro::{
    Locale, Recommendation, Severity, ValidateOptions, XmlSyntax, find_rule, render_rules_markdown,
    validate_bytes,
};
#[cfg(feature = "pdfa3")]
use lopdf::{Dictionary, Document, Object, Stream, dictionary};

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
    let xml_files = [
        "01_standard_b2b_ro.xml",
        "01_standard_b2b_ro_ubl.xml",
        "02_public_sector_ro.xml",
        "03_small_business_exempt_ro.xml",
        "04_reverse_charge_ro.xml",
        "05_credit_note_ro.xml",
        "06_cii_d16b_compliant_ro.xml",
    ];
    #[cfg(feature = "pdfa3")]
    let pdf_files = [
        "01_standard_b2b_ro.pdf",
        "01_standard_b2b_ro_ubl.pdf",
        "02_public_sector_ro.pdf",
        "03_small_business_exempt_ro.pdf",
        "04_reverse_charge_ro.pdf",
        "05_credit_note_ro.pdf",
    ];

    for locale in [Locale::En, Locale::Ro] {
        let opts = ValidateOptions::with_locale(locale)
            .deny_warnings(true)
            .include_svrl(true);

        #[cfg(feature = "pdfa3")]
        let all_files: Vec<&str> = xml_files.iter().chain(pdf_files.iter()).copied().collect();
        #[cfg(not(feature = "pdfa3"))]
        let all_files: Vec<&str> = xml_files.to_vec();

        for filename in all_files {
            let bytes = load_fixture_bytes(filename);
            let report = validate_bytes(&bytes, &opts);
            assert!(
                report.is_valid && report.violations.is_empty(),
                "Golden fixture '{filename}' failed under locale {locale:?}: {:?}",
                report.violations
            );
            assert_eq!(report.recommendation, Recommendation::Accept);
            assert_eq!(report.recommendation, "accept");

            #[cfg(feature = "svrl")]
            {
                let svrl = report.svrl_xml.as_deref().expect("SVRL generated");
                roxmltree::Document::parse(svrl).expect("SVRL XML must be well-formed");
                assert!(
                    !svrl.contains("failed-assert"),
                    "Compliant SVRL must have 0 failed-asserts"
                );
            }
            #[cfg(not(feature = "svrl"))]
            assert!(report.svrl_xml.is_none());
        }
    }

    #[cfg(not(feature = "pdfa3"))]
    {
        let pdf_bytes = load_fixture_bytes("01_standard_b2b_ro.pdf");
        assert_has_code(&pdf_bytes, "PDFA3-FEATURE-DISABLED");
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

    let opts = ValidateOptions::new().deny_warnings(true);
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
    assert!(
        rep_bad_cnp_default
            .violations
            .iter()
            .any(|v| v.code == "BR-RO-CNP" && v.severity == Severity::Warning)
    );
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
    assert!(
        rep_strict
            .violations
            .iter()
            .any(|v| v.code == "BR-RO-001" && v.severity == Severity::Fatal)
    );

    let rep_lenient = validate_bytes(
        &faulty_07,
        &ValidateOptions::new().strict_cius_ro_customization(false),
    );
    assert!(!rep_lenient.is_valid); // Still invalid due to EN 16931 math / missing fields
    assert!(
        rep_lenient
            .violations
            .iter()
            .any(|v| v.code == "BR-RO-001" && v.severity == Severity::Warning)
    );
}

#[test]
fn ubl_br_ro_core_and_party_mutations() {
    let ubl = load_fixture_str("01_standard_b2b_ro_ubl.xml");
    let cn = load_fixture_str("05_credit_note_ro.xml");

    assert_has_code(
        ubl.replace("CIUS-RO:1.0.1", "CIUS-RO:0.9.0").as_bytes(),
        "BR-RO-001",
    );
    assert_has_code(
        ubl.replace("EFACT-RO-2026-0042", "EFACT-RO-ABCD")
            .as_bytes(),
        "BR-RO-010",
    );
    let rep_inv_type = validate_bytes(
        ubl.replace(
            "<cbc:InvoiceTypeCode>380</cbc:InvoiceTypeCode>",
            "<cbc:InvoiceTypeCode>383</cbc:InvoiceTypeCode>",
        )
        .as_bytes(),
        &ValidateOptions::default(),
    );
    assert!(
        rep_inv_type
            .violations
            .iter()
            .any(|v| v.code == "BR-RO-020" && v.rule_sub_id.as_deref() == Some("BR-RO-020_1")),
        "Expected BR-RO-020 / BR-RO-020_1 on UBL Invoice, got: {:?}",
        rep_inv_type.violations
    );

    let rep_cn_type = validate_bytes(
        cn.replace(
            "<cbc:CreditNoteTypeCode>381</cbc:CreditNoteTypeCode>",
            "<cbc:CreditNoteTypeCode>380</cbc:CreditNoteTypeCode>",
        )
        .as_bytes(),
        &ValidateOptions::default(),
    );
    assert!(
        rep_cn_type
            .violations
            .iter()
            .any(|v| v.code == "BR-RO-020" && v.rule_sub_id.as_deref() == Some("BR-RO-020_2")),
        "Expected BR-RO-020 / BR-RO-020_2 on UBL CreditNote, got: {:?}",
        rep_cn_type.violations
    );

    assert_has_code(
        ubl.replace(
            "<cbc:DocumentCurrencyCode>RON</cbc:DocumentCurrencyCode>",
            "<cbc:DocumentCurrencyCode>EUR</cbc:DocumentCurrencyCode>",
        )
        .replace("currencyID=\"RON\"", "currencyID=\"EUR\"")
        .as_bytes(),
        "BR-RO-030",
    );
    assert_has_code(
        ubl.replace(
            "  <cac:AccountingSupplierParty>",
            "  <cac:InvoicePeriod><cbc:DescriptionCode>99</cbc:DescriptionCode></cac:InvoicePeriod>\n  <cac:AccountingSupplierParty>",
        )
        .as_bytes(),
        "BR-RO-040",
    );

    // PaymentMeans (BG-16 / BT-81) is optional in EN 16931 and official ANAF RO16931-rules.sch
    let ubl_no_pm = ubl.replace(
        "  <cac:PaymentMeans>\n    <cbc:PaymentMeansCode>58</cbc:PaymentMeansCode>\n    <cac:PayeeFinancialAccount>\n      <cbc:ID>RO49AAAA1B31007593840000</cbc:ID>\n      <cac:FinancialInstitutionBranch><cbc:ID>BTRLRO22</cbc:ID></cac:FinancialInstitutionBranch>\n    </cac:PayeeFinancialAccount>\n  </cac:PaymentMeans>\n",
        "",
    );
    let rep_no_pm = validate_bytes(
        ubl_no_pm.as_bytes(),
        &ValidateOptions::default().deny_warnings(true),
    );
    assert!(
        rep_no_pm.is_valid && rep_no_pm.violations.is_empty(),
        "UBL Invoice without PaymentMeans must be valid: {:?}",
        rep_no_pm.violations
    );

    let cn_no_pm = cn.replace(
        "  <cac:PaymentMeans>\n    <cbc:PaymentMeansCode>58</cbc:PaymentMeansCode>\n    <cbc:PaymentDueDate>2026-11-24</cbc:PaymentDueDate>\n    <cac:PayeeFinancialAccount>\n      <cbc:ID>RO49AAAA1B31007593840000</cbc:ID>\n      <cac:FinancialInstitutionBranch><cbc:ID>BTRLRO22</cbc:ID></cac:FinancialInstitutionBranch>\n    </cac:PayeeFinancialAccount>\n  </cac:PaymentMeans>\n",
        "",
    );
    let rep_cn_no_pm = validate_bytes(
        cn_no_pm.as_bytes(),
        &ValidateOptions::default().deny_warnings(true),
    );
    assert!(
        rep_cn_no_pm.is_valid && rep_cn_no_pm.violations.is_empty(),
        "UBL CreditNote without PaymentMeans must be valid: {:?}",
        rep_cn_no_pm.violations
    );

    let ubl_no_seller_tax = ubl.replace(
        "      <cac:PartyTaxScheme>\n        <cbc:CompanyID>RO45678918</cbc:CompanyID>\n        <cac:TaxScheme><cbc:ID>VAT</cbc:ID></cac:TaxScheme>\n      </cac:PartyTaxScheme>\n",
        "",
    );
    assert_has_code(ubl_no_seller_tax.as_bytes(), "BR-RO-065");
}

#[test]
fn ubl_br_ro_cardinality_and_party_address_mutations() {
    let ubl = load_fixture_str("01_standard_b2b_ro_ubl.xml");

    let mut many_notes = String::new();
    for i in 0..21 {
        let _ = writeln!(many_notes, "  <cbc:Note>Nota {i}</cbc:Note>");
    }
    assert_has_code(
        ubl.replace(
            "<cbc:DocumentCurrencyCode>RON</cbc:DocumentCurrencyCode>",
            &format!("{many_notes}  <cbc:DocumentCurrencyCode>RON</cbc:DocumentCurrencyCode>"),
        )
        .as_bytes(),
        "BR-RO-A020",
    );

    let mut many_docs = String::new();
    for i in 0..51 {
        let _ = writeln!(
            many_docs,
            "  <cac:AdditionalDocumentReference><cbc:ID>DOC-{i}</cbc:ID></cac:AdditionalDocumentReference>"
        );
    }
    assert_has_code(
        ubl.replace(
            "  <cac:AccountingSupplierParty>",
            &format!("{many_docs}  <cac:AccountingSupplierParty>"),
        )
        .as_bytes(),
        "BR-RO-A050",
    );

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
}

#[test]
fn ubl_br_ro_subdivision_mutations() {
    let ubl = load_fixture_str("01_standard_b2b_ro_ubl.xml");

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
    assert!(v_110.hint.as_deref().is_some_and(|h| h.contains("RO-CJ")));

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
    assert!(v_100.hint.as_deref().is_some_and(|h| h.contains("SECTOR1")));

    let no_buyer_ids = ubl
        .replace(
            "      <cac:PartyTaxScheme>\n        <cbc:CompanyID>RO98765438</cbc:CompanyID>\n        <cac:TaxScheme><cbc:ID>VAT</cbc:ID></cac:TaxScheme>\n      </cac:PartyTaxScheme>\n",
            "",
        )
        .replace("        <cbc:CompanyID>J12/890/2020</cbc:CompanyID>\n", "");
    assert_has_code(no_buyer_ids.as_bytes(), "BR-RO-120");

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

    let bad_tax_rep_deliv_county = with_tax_rep_and_deliv.replace(
        "<cbc:CountrySubentity>RO-B</cbc:CountrySubentity>",
        "<cbc:CountrySubentity>RO-XX</cbc:CountrySubentity>",
    );
    assert_has_code(bad_tax_rep_deliv_county.as_bytes(), "BR-RO-170");
    assert_has_code(bad_tax_rep_deliv_county.as_bytes(), "BR-RO-210");
}

#[test]
fn ubl_br_ro_date_length_decimal_and_xsd_mutations() {
    let ubl = load_fixture_str("01_standard_b2b_ro_ubl.xml");

    assert_has_code(
        ubl.replace(
            "<cbc:IssueDate>2026-10-15</cbc:IssueDate>",
            "<cbc:IssueDate>2026-10-15Z</cbc:IssueDate>",
        )
        .as_bytes(),
        "BR-RO-DT001",
    );

    assert_has_code(
        ubl.replace(
            "Servicii de consultanță IT și integrare ANAF RO e-Factura (CIUS-RO)",
            &"A".repeat(101),
        )
        .as_bytes(),
        "BR-RO-L100",
    );
    assert_has_code(
        ubl.replace(
            "Plata în termen de 30 de zile de la data emiterii facturii",
            &"P".repeat(301),
        )
        .as_bytes(),
        "BR-RO-L300",
    );

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
    assert!(rep_price4.is_valid && rep_price4.violations.is_empty());

    let neg_price = ubl.replace(
        "<cbc:PriceAmount currencyID=\"RON\">4500.00</cbc:PriceAmount>",
        "<cbc:PriceAmount currencyID=\"RON\">-4500.00</cbc:PriceAmount>",
    );
    let rep_neg = validate_bytes(neg_price.as_bytes(), &ValidateOptions::default());
    assert_eq!(
        rep_neg
            .violations
            .iter()
            .filter(|v| v.code == "BR-27")
            .count(),
        1
    );

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

    let xxe_xml = ubl.replacen(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>",
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE Invoice [ <!ENTITY xxe \"boom\"> ]>",
        1,
    );
    #[cfg(feature = "xsd")]
    assert_has_code(xxe_xml.as_bytes(), "XSD-XXE-PROHIBITED");
    #[cfg(not(feature = "xsd"))]
    assert_has_code(xxe_xml.as_bytes(), "EN16931-UNREADABLE");

    #[cfg(feature = "xsd")]
    {
        let bad_xsd_xml = ubl.replace(
            "<cbc:ID>EFACT-RO-2026-0042</cbc:ID>",
            "<cbc:ID>EFACT-RO-2026-0042</cbc:ID>\n  <cbc:IllegalNonExistentElement>1</cbc:IllegalNonExistentElement>",
        );
        assert_has_code(bad_xsd_xml.as_bytes(), "XSD-SCHEMA-001");
    }
}

#[cfg(feature = "pdfa3")]
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
        &ValidateOptions::with_locale(Locale::En).include_svrl(true),
    );
    let rep_ro = validate_bytes(
        &faulty,
        &ValidateOptions::with_locale(Locale::Ro).include_svrl(true),
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

    #[cfg(feature = "svrl")]
    {
        let svrl = rep_ro.svrl_xml.as_deref().expect("SVRL present");
        let parsed = roxmltree::Document::parse(svrl).expect("well-formed SVRL");
        let failed_asserts = parsed
            .descendants()
            .filter(|n| n.is_element() && n.tag_name().name() == "failed-assert")
            .count();
        assert_eq!(failed_asserts, rep_ro.violations.len());
    }
    #[cfg(not(feature = "svrl"))]
    assert!(rep_ro.svrl_xml.is_none());
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
        let _ = writeln!(notes, "  <cbc:Note>{extra_ascii}</cbc:Note>");
        for _ in 2..20 {
            let _ = writeln!(notes, "  <cbc:Note>{note_body}</cbc:Note>");
        }
        let mut add_docs = String::new();
        for idx in 0..25 {
            let _ = writeln!(
                add_docs,
                "  <cac:AdditionalDocumentReference><cbc:ID>ANEXA-{idx}</cbc:ID><cbc:DocumentDescription>{diacritic_chunk} anexă justificativă {idx}</cbc:DocumentDescription></cac:AdditionalDocumentReference>"
            );
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
        &ValidateOptions::default().deny_warnings(true),
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
    #[cfg(feature = "xsd")]
    assert_has_code(deep_xxe.as_bytes(), "XSD-XXE-PROHIBITED");
    #[cfg(not(feature = "xsd"))]
    assert_has_code(deep_xxe.as_bytes(), "EN16931-UNREADABLE");

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
    let mut little_endian_u16 = vec![0xFF, 0xFE];
    for unit in u16_src.encode_utf16() {
        little_endian_u16.extend_from_slice(&unit.to_le_bytes());
    }
    let rep_little = validate_bytes(&little_endian_u16, &ValidateOptions::default());
    assert!(
        rep_little.is_valid && rep_little.violations.is_empty(),
        "UTF-16LE BOM XML must pass cleanly: {:?}",
        rep_little.violations
    );

    let mut big_endian_u16 = vec![0xFE, 0xFF];
    for unit in u16_src.encode_utf16() {
        big_endian_u16.extend_from_slice(&unit.to_be_bytes());
    }
    let rep_big = validate_bytes(&big_endian_u16, &ValidateOptions::default());
    assert!(
        rep_big.is_valid && rep_big.violations.is_empty(),
        "UTF-16BE BOM XML must pass cleanly: {:?}",
        rep_big.violations
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

    // BR-RO-040: ApplicableTradeTax/DueDateTypeCode allowed UNTDID 2475 {5, 29, 72}; disallowed 99
    for allowed_code in ["5", "29", "72"] {
        let cii_due_ok = cii.replace(
            "<ram:CategoryCode>S</ram:CategoryCode>\n                <ram:RateApplicablePercent>19.00</ram:RateApplicablePercent>",
            &format!(
                "<ram:CategoryCode>S</ram:CategoryCode>\n                <ram:DueDateTypeCode>{allowed_code}</ram:DueDateTypeCode>\n                <ram:RateApplicablePercent>19.00</ram:RateApplicablePercent>"
            ),
        );
        let rep_due_ok = validate_bytes(
            cii_due_ok.as_bytes(),
            &ValidateOptions::default().deny_warnings(true),
        );
        assert!(
            rep_due_ok.is_valid && rep_due_ok.violations.is_empty(),
            "CII DueDateTypeCode={allowed_code} must pass cleanly: {:?}",
            rep_due_ok.violations
        );
    }
    let cii_due_bad = cii.replace(
        "<ram:CategoryCode>S</ram:CategoryCode>\n                <ram:RateApplicablePercent>19.00</ram:RateApplicablePercent>",
        "<ram:CategoryCode>S</ram:CategoryCode>\n                <ram:DueDateTypeCode>99</ram:DueDateTypeCode>\n                <ram:RateApplicablePercent>19.00</ram:RateApplicablePercent>",
    );
    assert_has_code(cii_due_bad.as_bytes(), "BR-RO-040");

    // PaymentMeans is optional in EN 16931 / CIUS-RO CII D16B
    let cii_no_pm = cii.replace(
        "            <ram:SpecifiedTradeSettlementPaymentMeans>\n                <ram:TypeCode>58</ram:TypeCode>\n                <ram:PayeePartyCreditorFinancialAccount>\n                    <ram:IBANID>RO49AAAA1B31007593840000</ram:IBANID>\n                </ram:PayeePartyCreditorFinancialAccount>\n            </ram:SpecifiedTradeSettlementPaymentMeans>\n",
        "",
    );
    let rep_cii_no_pm = validate_bytes(
        cii_no_pm.as_bytes(),
        &ValidateOptions::default().deny_warnings(true),
    );
    assert!(
        rep_cii_no_pm.is_valid && rep_cii_no_pm.violations.is_empty(),
        "CII without SpecifiedTradeSettlementPaymentMeans must be valid: {:?}",
        rep_cii_no_pm.violations
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
}

#[test]
fn cii_d16b_notes_length_tax_rep_and_namespace_strictness() {
    let cii = load_fixture_str("06_cii_d16b_compliant_ro.xml");
    let mut many_cii_notes = String::new();
    for i in 0..21 {
        let _ = writeln!(
            many_cii_notes,
            "        <ram:IncludedNote><ram:Content>Nota {i}</ram:Content></ram:IncludedNote>"
        );
    }
    assert_has_code(
        cii.replace(
            "    </rsm:ExchangedDocument>",
            &format!("{many_cii_notes}    </rsm:ExchangedDocument>"),
        )
        .as_bytes(),
        "BR-RO-A020",
    );

    assert_has_code(
        cii.replace(
            "Licență software și mentenanță RO e-Factura",
            &"N".repeat(101),
        )
        .as_bytes(),
        "BR-RO-L100",
    );

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

    // Root tag with wrong namespace must NOT be classified as CII D16B or UBL 2.1
    let wrong_ns_cii =
        b"<?xml version=\"1.0\" encoding=\"UTF-8\"?><CrossIndustryInvoice xmlns=\"http://example.com/wrong\"/>";
    let rep_wrong = validate_bytes(wrong_ns_cii, &ValidateOptions::default());
    assert!(!rep_wrong.is_valid);
    assert_eq!(rep_wrong.detected_syntax, None);
    #[cfg(feature = "xsd")]
    assert!(
        rep_wrong
            .violations
            .iter()
            .any(|v| v.code == "XSD-UNKNOWN-ROOT")
    );
    #[cfg(not(feature = "xsd"))]
    assert!(
        rep_wrong
            .violations
            .iter()
            .any(|v| v.code == "EN16931-UNREADABLE" || v.code == "BR-RO-UNKNOWN-ROOT")
    );
}

#[test]
fn bt1_and_bt19_length_caps_and_whitespace_normalization() {
    let ubl = load_fixture_str("01_standard_b2b_ro_ubl.xml");

    // BT-1 (Invoice number): 200 chars passes, 201 chars triggers BR-RO-L200 (sub-ID BR-RO-L155)
    let id_200 = format!("RO2026{}", "1".repeat(194));
    assert_eq!(id_200.len(), 200);
    let rep_id_ok = validate_bytes(
        ubl.replace("EFACT-RO-2026-0042", &id_200).as_bytes(),
        &ValidateOptions::default(),
    );
    assert!(rep_id_ok.is_valid, "{:?}", rep_id_ok.violations);

    let id_201 = format!("RO2026{}", "1".repeat(195));
    assert_eq!(id_201.len(), 201);
    let rep_id_bad = validate_bytes(
        ubl.replace("EFACT-RO-2026-0042", &id_201).as_bytes(),
        &ValidateOptions::default(),
    );
    assert!(
        rep_id_bad
            .violations
            .iter()
            .any(|v| { v.code == "BR-RO-L200" && v.rule_sub_id.as_deref() == Some("BR-RO-L155") }),
        "Expected BR-RO-L200 / BR-RO-L155 for 201-char BT-1, got: {:?}",
        rep_id_bad.violations
    );

    // BT-19 (AccountingCost): 100 chars passes, 101 chars triggers BR-RO-L100 (sub-ID BR-RO-L1001)
    let acct_100 = "C".repeat(100);
    let with_acct_100 = ubl.replace(
        "  <cbc:DocumentCurrencyCode>RON</cbc:DocumentCurrencyCode>",
        &format!(
            "  <cbc:DocumentCurrencyCode>RON</cbc:DocumentCurrencyCode>\n  <cbc:AccountingCost>{acct_100}</cbc:AccountingCost>"
        ),
    );
    let rep_acct_ok = validate_bytes(with_acct_100.as_bytes(), &ValidateOptions::default());
    assert!(rep_acct_ok.is_valid, "{:?}", rep_acct_ok.violations);

    let acct_101 = "C".repeat(101);
    let with_acct_101 = ubl.replace(
        "  <cbc:DocumentCurrencyCode>RON</cbc:DocumentCurrencyCode>",
        &format!(
            "  <cbc:DocumentCurrencyCode>RON</cbc:DocumentCurrencyCode>\n  <cbc:AccountingCost>{acct_101}</cbc:AccountingCost>"
        ),
    );
    let rep_acct_bad = validate_bytes(with_acct_101.as_bytes(), &ValidateOptions::default());
    assert!(
        rep_acct_bad
            .violations
            .iter()
            .any(|v| { v.code == "BR-RO-L100" && v.rule_sub_id.as_deref() == Some("BR-RO-L1001") }),
        "Expected BR-RO-L100 / BR-RO-L1001 for 101-char BT-19, got: {:?}",
        rep_acct_bad.violations
    );

    // XPath normalize-space(): 95-char string padded with 20 leading/trailing spaces -> 95 normalized chars -> PASS
    let padded_acct = format!("          {}          ", "D".repeat(95));
    let with_padded = ubl.replace(
        "  <cbc:DocumentCurrencyCode>RON</cbc:DocumentCurrencyCode>",
        &format!(
            "  <cbc:DocumentCurrencyCode>RON</cbc:DocumentCurrencyCode>\n  <cbc:AccountingCost>{padded_acct}</cbc:AccountingCost>"
        ),
    );
    let rep_padded = validate_bytes(with_padded.as_bytes(), &ValidateOptions::default());
    assert!(rep_padded.is_valid, "{:?}", rep_padded.violations);
}

#[test]
fn lexical_dates_scoped_and_secondary_party_cui_checked() {
    let ubl = load_fixture_str("01_standard_b2b_ro_ubl.xml");

    // 1. Optional non-CIUS-RO OrderReference/IssueDate must NOT falsely trigger BR-RO-DT001 when XSD is off
    let with_order_issue_date = ubl.replace(
        "  <cac:AccountingSupplierParty>",
        "  <cac:OrderReference><cbc:ID>PO-99</cbc:ID><cbc:IssueDate>2026-10-15+02:00</cbc:IssueDate></cac:OrderReference>\n  <cac:AccountingSupplierParty>",
    );
    let rep_scoped = validate_bytes(
        with_order_issue_date.as_bytes(),
        &ValidateOptions::default().xsd(false),
    );
    assert!(
        !rep_scoped
            .violations
            .iter()
            .any(|v| v.code == "BR-RO-DT001"),
        "OrderReference/IssueDate must not falsely trigger BR-RO-DT001: {:?}",
        rep_scoped.violations
    );

    // 2. Valid primary Seller VAT CUI (BT-31: RO45678918) + invalid secondary TaxRegistration CUI (BT-32: 45678919)
    //    must emit BR-RO-CUI warning instead of stopping after BT-31!
    let secondary_bad_cui = ubl.replace(
        "      <cac:PartyTaxScheme>\n        <cbc:CompanyID>RO45678918</cbc:CompanyID>\n        <cac:TaxScheme><cbc:ID>VAT</cbc:ID></cac:TaxScheme>\n      </cac:PartyTaxScheme>",
        "      <cac:PartyTaxScheme>\n        <cbc:CompanyID>RO45678918</cbc:CompanyID>\n        <cac:TaxScheme><cbc:ID>VAT</cbc:ID></cac:TaxScheme>\n      </cac:PartyTaxScheme>\n      <cac:PartyTaxScheme>\n        <cbc:CompanyID>45678919</cbc:CompanyID>\n        <cac:TaxScheme><cbc:ID>CIF</cbc:ID></cac:TaxScheme>\n      </cac:PartyTaxScheme>",
    );
    let rep_sec_cui = validate_bytes(secondary_bad_cui.as_bytes(), &ValidateOptions::default());
    assert!(
        rep_sec_cui.violations.iter().any(|v| v.code == "BR-RO-CUI"),
        "Expected secondary invalid CUI on BT-32 to emit BR-RO-CUI, got: {:?}",
        rep_sec_cui.violations
    );

    // 3. Duplicate identical invalid CUI in BT-31 and BT-32 emits BR-RO-CUI only once
    let dup_bad_cui = ubl.replace(
        "      <cac:PartyTaxScheme>\n        <cbc:CompanyID>RO45678918</cbc:CompanyID>\n        <cac:TaxScheme><cbc:ID>VAT</cbc:ID></cac:TaxScheme>\n      </cac:PartyTaxScheme>",
        "      <cac:PartyTaxScheme>\n        <cbc:CompanyID>RO45678919</cbc:CompanyID>\n        <cac:TaxScheme><cbc:ID>VAT</cbc:ID></cac:TaxScheme>\n      </cac:PartyTaxScheme>\n      <cac:PartyTaxScheme>\n        <cbc:CompanyID>RO45678919</cbc:CompanyID>\n        <cac:TaxScheme><cbc:ID>CIF</cbc:ID></cac:TaxScheme>\n      </cac:PartyTaxScheme>",
    );
    let rep_dup_cui = validate_bytes(dup_bad_cui.as_bytes(), &ValidateOptions::default());
    assert_eq!(
        rep_dup_cui
            .violations
            .iter()
            .filter(|v| v.code == "BR-RO-CUI")
            .count(),
        1
    );
}

#[test]
fn rules_catalog_sub_id_resolution() {
    let cases = [
        ("BR-RO-020_1", "BR-RO-020"),
        ("BR-RO-020_2", "BR-RO-020"),
        ("BR-RO-101", "BR-RO-100"),
        ("br-ro-111", "BR-RO-110"),
        ("BR-RO-A051", "BR-RO-A050"),
        ("BR-DEC-RO-05", "BR-RO-Z2"),
        ("BR-RO-L155", "BR-RO-L200"),
        ("BR-RO-L0302", "BR-RO-L200"),
        ("BR-RO-L0307", "BR-RO-L200"),
        ("BR-RO-L301", "BR-RO-L300"),
        ("BR-RO-L302", "BR-RO-L300"),
        ("BR-RO-L1001", "BR-RO-L100"),
        ("BR-RO-L1024", "BR-RO-L100"),
        ("BR-RO-DT003_CN", "BR-RO-DT003-CN"),
        ("XSD-SCHEMA-001", "XSD-SCHEMA"),
        ("PDFA3-6.1.4-ENCRYPT", "PDFA3-ISO19005"),
    ];
    for (sub_id, expected_canonical) in cases {
        let found = find_rule(sub_id).unwrap_or_else(|| panic!("find_rule failed for {sub_id}"));
        assert_eq!(found.code, expected_canonical, "sub_id {sub_id}");
    }
    assert!(
        find_rule("BR-RO-050").is_none(),
        "Hallucinated BR-RO-050 must not exist in RULES catalog"
    );
}

#[cfg(feature = "pdfa3")]
#[test]
fn pdfa3_supplement_attachment_af_membership_and_no_pdfa_diagnostics() {
    // 01_standard_b2b_ro.pdf embeds `efactura.xml`
    let base_pdf = load_fixture_bytes("01_standard_b2b_ro.pdf");

    // 1. Add a non-XML /Supplement attachment (attachment.txt, text/plain) to Catalog /AF:
    //    must PASS cleanly with deny_warnings=true (no false PDFA3-6.8-MIME-SUBTYPE warning!)
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
    //    Catalog /AF is non-empty, yet efactura.xml is unassociated -> must emit PDFA3-6.8-HYBRID-DIVERGENCE!
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
    //    instead of returning `is_valid: false` with empty `violations`!
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

#[cfg(feature = "pdfa3")]
#[test]
fn pdfa3_utf8_multibyte_attachment_filenames_never_panic() {
    let base_pdf = load_fixture_bytes("01_standard_b2b_ro.pdf");

    // 1. Keep primary efactura.xml + add /Supplement with filename "șabc"
    //    (UTF-8 [0xC8, 0x99, b'a', b'b', b'c'], len=5 -> len-4=1 lands mid-codepoint!)
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
    let ubl = load_fixture_str("01_standard_b2b_ro_ubl.xml");
    let foreign_children = ubl
        .replace(
            "  <cbc:CustomizationID>",
            "  <ext:CustomizationID xmlns:ext=\"urn:example:ext\">urn:wrong:customization</ext:CustomizationID>\n  <cbc:CustomizationID>",
        )
        .replace(
            "  <cbc:IssueDate>2026-10-15</cbc:IssueDate>",
            "  <ext:IssueDate xmlns:ext=\"urn:example:ext\">not-a-valid-date</ext:IssueDate>\n  <cbc:IssueDate>2026-10-15</cbc:IssueDate>",
        )
        .replace(
            "  <cac:LegalMonetaryTotal>",
            "  <cac:LegalMonetaryTotal>\n    <ext:PayableAmount xmlns:ext=\"urn:example:ext\">5355.999</ext:PayableAmount>",
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

#[test]
fn multi_standard_router_mode_non_ro_clean_and_ro_checksums_run() {
    let ubl = load_fixture_str("01_standard_b2b_ro_ubl.xml");

    // 1. Non-Romanian EN 16931 invoice (DE seller -> FR buyer, EUR) in router mode:
    //    must pass with ZERO warnings even when deny_warnings=true!
    let de_fr_invoice = ubl
        .replace(
            "urn:cen.eu:en16931:2017#compliant#urn:efactura.mfinante.ro:CIUS-RO:1.0.1",
            "urn:cen.eu:en16931:2017",
        )
        .replace(
            "<cbc:DocumentCurrencyCode>RON</cbc:DocumentCurrencyCode>",
            "<cbc:DocumentCurrencyCode>EUR</cbc:DocumentCurrencyCode>",
        )
        .replace("currencyID=\"RON\"", "currencyID=\"EUR\"")
        .replace(
            "<cbc:CityName>SECTOR1</cbc:CityName>",
            "<cbc:CityName>Berlin</cbc:CityName>",
        )
        .replace(
            "<cbc:CountrySubentity>RO-B</cbc:CountrySubentity>\n        ",
            "",
        )
        .replace(
            "<cbc:IdentificationCode>RO</cbc:IdentificationCode>",
            "<cbc:IdentificationCode>DE</cbc:IdentificationCode>",
        )
        .replace("RO45678918", "DE123456789")
        .replace("J40/1234/2022", "HRB12345")
        .replace(
            "<cbc:CountrySubentity>RO-CJ</cbc:CountrySubentity>\n        ",
            "",
        )
        .replace("RO98765438", "DE987654321")
        .replace("J12/890/2020", "HRB67890")
        .replace("RO49AAAA1B31007593840000", "DE89370400440532013000");

    let router_opts = ValidateOptions::default()
        .strict_cius_ro_customization(false)
        .deny_warnings(true);
    let rep_de = validate_bytes(de_fr_invoice.as_bytes(), &router_opts);
    assert!(
        rep_de.is_valid && rep_de.violations.is_empty(),
        "Non-Romanian EN 16931 invoice must not emit BR-RO-001 warning in router mode: {:?}",
        rep_de.violations
    );
    let anaf_eng = rep_de
        .engines
        .iter()
        .find(|e| e.id == "anaf_cius_ro")
        .expect("anaf_cius_ro engine report");
    assert_eq!(anaf_eng.rules_evaluated, 0);
    assert_eq!(anaf_eng.passed_count, 0);

    // 2. Romanian party EN 16931 invoice without CIUS-RO CustomizationID and with invalid Seller CUI:
    //    must emit BOTH Warning BR-RO-001 AND Warning BR-RO-CUI!
    let ro_non_cius_bad_cui = ubl
        .replace(
            "urn:cen.eu:en16931:2017#compliant#urn:efactura.mfinante.ro:CIUS-RO:1.0.1",
            "urn:cen.eu:en16931:2017",
        )
        .replace("RO45678918", "RO45678919");
    let rep_ro_router = validate_bytes(
        ro_non_cius_bad_cui.as_bytes(),
        &ValidateOptions::default().strict_cius_ro_customization(false),
    );
    assert!(rep_ro_router.is_valid);
    assert!(
        rep_ro_router
            .violations
            .iter()
            .any(|v| v.code == "BR-RO-001" && v.severity == Severity::Warning),
        "Expected BR-RO-001 warning on Romanian invoice without CIUS-RO BT-24: {:?}",
        rep_ro_router.violations
    );
    assert!(
        rep_ro_router
            .violations
            .iter()
            .any(|v| v.code == "BR-RO-CUI" && v.severity == Severity::Warning),
        "Expected BR-RO-CUI checksum warning to run even when BT-24 lacks CIUS-RO: {:?}",
        rep_ro_router.violations
    );
}

#[test]
fn anaf_schematron_v109_length_caps_and_sub_ids() {
    let ubl = load_fixture_str("01_standard_b2b_ro_ubl.xml");

    // 1. BT-10 (150 chars) and BT-11 (250 chars) have NO length cap in ANAF RO16931-rules.sch
    let bt10_bt11_long = ubl.replace(
        "  <cbc:BuyerReference>CONTR-RO-2026-88</cbc:BuyerReference>",
        &format!(
            "  <cbc:BuyerReference>{}</cbc:BuyerReference>\n  <cac:ProjectReference><cbc:ID>{}</cbc:ID></cac:ProjectReference>",
            "B".repeat(150),
            "P".repeat(250),
        ),
    );
    let rep_bt10_11 = validate_bytes(
        bt10_bt11_long.as_bytes(),
        &ValidateOptions::default().deny_warnings(true),
    );
    assert!(
        rep_bt10_11.is_valid && rep_bt10_11.violations.is_empty(),
        "BT-10 (150 chars) and BT-11 (250 chars) must pass cleanly: {:?}",
        rep_bt10_11.violations
    );

    // 2. Official ANAF sub-IDs on BT-12..BT-17, BT-20, and BT-22
    let doc_refs_over = ubl
        .replace(
            "  <cbc:DocumentCurrencyCode>RON</cbc:DocumentCurrencyCode>",
            &format!(
                "  <cbc:Note>{}</cbc:Note>\n  <cbc:DocumentCurrencyCode>RON</cbc:DocumentCurrencyCode>",
                "N".repeat(301)
            ),
        )
        .replace(
            "  <cac:AccountingSupplierParty>",
            &format!(
                "  <cac:OrderReference><cbc:ID>{r}</cbc:ID><cbc:SalesOrderID>{r}</cbc:SalesOrderID></cac:OrderReference>\n\
                 <cac:DespatchDocumentReference><cbc:ID>{r}</cbc:ID></cac:DespatchDocumentReference>\n\
                 <cac:ReceiptDocumentReference><cbc:ID>{r}</cbc:ID></cac:ReceiptDocumentReference>\n\
                 <cac:OriginatorDocumentReference><cbc:ID>{r}</cbc:ID></cac:OriginatorDocumentReference>\n\
                 <cac:ContractDocumentReference><cbc:ID>{r}</cbc:ID></cac:ContractDocumentReference>\n\
                 <cac:AccountingSupplierParty>",
                r = "R".repeat(201)
            ),
        )
        .replace(
            "Plata în termen de 30 de zile de la data emiterii facturii",
            &"T".repeat(301),
        );
    let rep_sub_ids = validate_bytes(doc_refs_over.as_bytes(), &ValidateOptions::default());
    for (expected_code, expected_sub) in [
        ("BR-RO-L200", "BR-RO-L0302"),
        ("BR-RO-L200", "BR-RO-L0303"),
        ("BR-RO-L200", "BR-RO-L0304"),
        ("BR-RO-L200", "BR-RO-L0305"),
        ("BR-RO-L200", "BR-RO-L0306"),
        ("BR-RO-L200", "BR-RO-L0307"),
        ("BR-RO-L300", "BR-RO-L301"),
        ("BR-RO-L300", "BR-RO-L302"),
    ] {
        assert!(
            rep_sub_ids.violations.iter().any(|v| {
                v.code == expected_code && v.rule_sub_id.as_deref() == Some(expected_sub)
            }),
            "Expected {expected_code} / {expected_sub}, got: {:?}",
            rep_sub_ids.violations
        );
    }
}

#[test]
fn anaf_schematron_v109_foreign_seller_br_ro_030_and_cii_tax_point_date() {
    let ubl = load_fixture_str("01_standard_b2b_ro_ubl.xml");
    let cii = load_fixture_str("06_cii_d16b_compliant_ro.xml");

    // 1. BR-RO-030 applies regardless of Seller country (e.g. IT Seller invoicing in EUR without TaxCurrencyCode=RON)
    let foreign_seller_eur = ubl
        .replace(
            "<cbc:DocumentCurrencyCode>RON</cbc:DocumentCurrencyCode>",
            "<cbc:DocumentCurrencyCode>EUR</cbc:DocumentCurrencyCode>",
        )
        .replace("currencyID=\"RON\"", "currencyID=\"EUR\"")
        .replace(
            "<cbc:CityName>SECTOR1</cbc:CityName>",
            "<cbc:CityName>Milano</cbc:CityName>",
        )
        .replace(
            "<cbc:CountrySubentity>RO-B</cbc:CountrySubentity>\n        ",
            "",
        )
        .replacen(
            "<cbc:IdentificationCode>RO</cbc:IdentificationCode>",
            "<cbc:IdentificationCode>IT</cbc:IdentificationCode>",
            1,
        )
        .replace("RO45678918", "IT12345678901")
        .replace("J40/1234/2022", "MI-123456");
    assert_has_code(foreign_seller_eur.as_bytes(), "BR-RO-030");

    // 4. CII D16B TaxPointDate uses udt:DateString format="102" (YYYYMMDD)
    let cii_tax_point_ok = cii.replace(
        "<ram:CategoryCode>S</ram:CategoryCode>\n                <ram:RateApplicablePercent>19.00</ram:RateApplicablePercent>",
        "<ram:CategoryCode>S</ram:CategoryCode>\n                <ram:TaxPointDate><udt:DateString format=\"102\">20261015</udt:DateString></ram:TaxPointDate>\n                <ram:RateApplicablePercent>19.00</ram:RateApplicablePercent>",
    );
    let rep_tp_ok = validate_bytes(
        cii_tax_point_ok.as_bytes(),
        &ValidateOptions::default().deny_warnings(true),
    );
    assert!(
        rep_tp_ok.is_valid && rep_tp_ok.violations.is_empty(),
        "Valid CII TaxPointDate/DateString[@format='102'] must pass cleanly: {:?}",
        rep_tp_ok.violations
    );

    let cii_tax_point_bad = cii.replace(
        "<ram:CategoryCode>S</ram:CategoryCode>\n                <ram:RateApplicablePercent>19.00</ram:RateApplicablePercent>",
        "<ram:CategoryCode>S</ram:CategoryCode>\n                <ram:TaxPointDate><udt:DateString format=\"102\">2026-10-15</udt:DateString></ram:TaxPointDate>\n                <ram:RateApplicablePercent>19.00</ram:RateApplicablePercent>",
    );
    assert_has_code(cii_tax_point_bad.as_bytes(), "BR-RO-DT002");

    let cii_missing_format_attr = cii.replace(
        "<udt:DateTimeString format=\"102\">20261015</udt:DateTimeString>",
        "<udt:DateTimeString>20261015</udt:DateTimeString>",
    );
    assert_has_code(cii_missing_format_attr.as_bytes(), "BR-RO-DT001");
}

#[test]
fn lexical_decimal_and_non_xpath_whitespace_edge_cases() {
    let ubl = load_fixture_str("01_standard_b2b_ro_ubl.xml");

    for bad_amount in ["5355.12abc", "5355.1e-5"] {
        let mutated = ubl.replace(
            "<cbc:PayableAmount currencyID=\"RON\">5355.00</cbc:PayableAmount>",
            &format!("<cbc:PayableAmount currencyID=\"RON\">{bad_amount}</cbc:PayableAmount>"),
        );
        let rep = validate_bytes(
            mutated.as_bytes(),
            &ValidateOptions::default().xsd(false).en16931(false),
        );
        assert!(
            rep.violations.iter().any(|v| v.code == "BR-RO-Z2"),
            "Expected BR-RO-Z2 on malformed amount '{bad_amount}' without XSD, got: {:?}",
            rep.violations
        );
    }

    // U+00A0 (NBSP) is NOT W3C XPath whitespace, so 50 + 1 + 50 = 101 normalized chars -> BR-RO-L1001
    let nbsp_101 = format!("{}\u{00A0}{}", "A".repeat(50), "B".repeat(50));
    let with_nbsp = ubl.replace(
        "  <cbc:DocumentCurrencyCode>RON</cbc:DocumentCurrencyCode>",
        &format!(
            "  <cbc:DocumentCurrencyCode>RON</cbc:DocumentCurrencyCode>\n  <cbc:AccountingCost>{nbsp_101}</cbc:AccountingCost>"
        ),
    );
    let rep_nbsp = validate_bytes(with_nbsp.as_bytes(), &ValidateOptions::default());
    assert!(
        rep_nbsp
            .violations
            .iter()
            .any(|v| v.code == "BR-RO-L100" && v.rule_sub_id.as_deref() == Some("BR-RO-L1001")),
        "NBSP must not be collapsed as XPath whitespace: {:?}",
        rep_nbsp.violations
    );
}

#[test]
fn art_310_small_business_fixture_uses_vat_category_o() {
    let xml = load_fixture_str("03_small_business_exempt_ro.xml");
    assert!(xml.contains("<cbc:ID>O</cbc:ID>"));
    assert!(xml.contains("<cbc:TaxExemptionReasonCode>VATEX-EU-O</cbc:TaxExemptionReasonCode>"));
    assert!(!xml.contains("<cbc:ID>E</cbc:ID>"));
    assert!(!xml.contains("<cbc:Percent>"));

    let rep_xml = validate_bytes(
        xml.as_bytes(),
        &ValidateOptions::default().deny_warnings(true),
    );
    assert!(rep_xml.is_valid && rep_xml.violations.is_empty());

    #[cfg(feature = "pdfa3")]
    {
        let pdf = load_fixture_bytes("03_small_business_exempt_ro.pdf");
        let extracted = String::from_utf8(
            extract_xml_from_pdf(&pdf)
                .expect("extract 03 PDF XML")
                .xml_bytes,
        )
        .expect("UTF-8");
        assert_eq!(extracted, xml);
    }
}

#[cfg(feature = "pdfa3")]
#[test]
fn pdfa3_single_stream_extraction_skips_unrelated_attachments_and_guards_xref() {
    let base_pdf = load_fixture_bytes("01_standard_b2b_ro.pdf");

    // 1. Attach a /Supplement stream with corrupt /FlateDecode bytes in /Names/EmbeddedFiles:
    //    `extract_xml_from_pdf` must inflate ONLY `efactura.xml` and ignore unrelated attachments!
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

    // 2. Malicious XRef stream dictionaries (both missing startxref AND real startxref-linked
    //    /W [99 99 99] and zero-width /W [0 0 0] /Size 999999999 bombs) must be rejected pre-load!
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

#[cfg(feature = "pdfa3")]
#[test]
fn pdfa3_single_pass_objstm_and_facturx_hybrid_association() {
    let base_pdf = load_fixture_bytes("01_standard_b2b_ro.pdf");

    // 1. Single-pass /ObjStm + /XRef stream container (PDF 1.5+ modern save) must unpack
    //    compressed dictionaries cleanly and pass full PDF/A-3 + CIUS-RO validation
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
    use cius_ro::{EngineStatus, evaluate_cius_ro_rules, validate_xml};

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
