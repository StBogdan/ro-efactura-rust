//! Golden corpus, `xmllint` XSD cross-verification, faulty sample fixtures, rule catalog,
//! bilingual (`en` / `ro`) & SVRL output, and multi-standard router mode tests.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(feature = "pdfa3")]
use cius_ro::extract_xml_from_pdf;
use cius_ro::{
    Locale, Recommendation, Severity, ValidateOptions, XmlSyntax, find_rule, render_rules_markdown,
    validate_bytes,
};

#[cfg(not(feature = "pdfa3"))]
use crate::helpers::assert_has_code;
use crate::helpers::{XmlFixture, fixtures_dir, load_fixture_bytes, load_fixture_str, schemas_dir};

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
    assert!(!rep_lenient.is_valid);
    assert!(
        rep_lenient
            .violations
            .iter()
            .any(|v| v.code == "BR-RO-001" && v.severity == Severity::Warning)
    );
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
fn multi_standard_router_mode_non_ro_clean_and_ro_checksums_run() {
    let ubl = XmlFixture::load("01_standard_b2b_ro_ubl.xml");

    // 1. Non-Romanian EN 16931 invoice (DE seller -> DE buyer, EUR) in router mode:
    //    must pass with ZERO warnings even when deny_warnings=true.
    let de_invoice = ubl
        .clone()
        .set_text("CustomizationID", "urn:cen.eu:en16931:2017")
        .set_text("DocumentCurrencyCode", "EUR")
        .set_attr_all("currencyID", "RON", "EUR")
        .set_text(
            "AccountingSupplierParty/Party/PostalAddress/CityName",
            "Berlin",
        )
        .remove("AccountingSupplierParty/Party/PostalAddress/CountrySubentity")
        .set_text(
            "AccountingSupplierParty/Party/PostalAddress/Country/IdentificationCode",
            "DE",
        )
        .set_text(
            "AccountingSupplierParty/Party/PartyTaxScheme/CompanyID",
            "DE123456789",
        )
        .set_text(
            "AccountingSupplierParty/Party/PartyLegalEntity/CompanyID",
            "HRB12345",
        )
        .remove("AccountingCustomerParty/Party/PostalAddress/CountrySubentity")
        .set_text(
            "AccountingCustomerParty/Party/PostalAddress/Country/IdentificationCode",
            "DE",
        )
        .set_text(
            "AccountingCustomerParty/Party/PartyTaxScheme/CompanyID",
            "DE987654321",
        )
        .set_text(
            "AccountingCustomerParty/Party/PartyLegalEntity/CompanyID",
            "HRB67890",
        )
        .set_text("PayeeFinancialAccount/ID", "DE89370400440532013000");

    let router_opts = ValidateOptions::default()
        .strict_cius_ro_customization(false)
        .deny_warnings(true);
    let rep_de = validate_bytes(de_invoice.as_bytes(), &router_opts);
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
    //    must emit BOTH Warning BR-RO-001 AND Warning BR-RO-CUI.
    let ro_non_cius_bad_cui = ubl
        .set_text("CustomizationID", "urn:cen.eu:en16931:2017")
        .set_text(
            "AccountingSupplierParty/Party/PartyTaxScheme/CompanyID",
            "RO45678919",
        );
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
