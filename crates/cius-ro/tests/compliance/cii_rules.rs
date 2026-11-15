//! UN/CEFACT CII D16B `BR-RO-*` single-fault mutations, notes, tax point dates,
//! and foreign-seller `BR-RO-030` compliance tests.

use std::fmt::Write as _;

use cius_ro::{ValidateOptions, validate_bytes};

use crate::helpers::{XmlFixture, assert_has_code};

#[test]
fn cii_d16b_br_ro_single_fault_mutations() {
    let cii = XmlFixture::load("06_cii_d16b_compliant_ro.xml");

    // BR-RO-010: CII ID without digit
    assert_has_code(
        cii.clone()
            .set_text("ExchangedDocument/ID", "CII-RO-ABCD")
            .as_bytes(),
        "BR-RO-010",
    );

    // BR-RO-020: disallowed TypeCode
    assert_has_code(
        cii.clone()
            .set_text("ExchangedDocument/TypeCode", "383")
            .as_bytes(),
        "BR-RO-020",
    );

    // BR-RO-030: foreign currency EUR without TaxCurrencyCode="RON"
    assert_has_code(
        cii.clone()
            .set_text("InvoiceCurrencyCode", "EUR")
            .set_attr_all("currencyID", "RON", "EUR")
            .as_bytes(),
        "BR-RO-030",
    );

    // BR-RO-040: ApplicableTradeTax/DueDateTypeCode allowed UNTDID 2475 {5, 29, 72}; disallowed 99
    for allowed_code in ["5", "29", "72"] {
        let cii_due_ok = cii.clone().insert_after(
            "ApplicableHeaderTradeSettlement/ApplicableTradeTax/CategoryCode",
            &format!("<ram:DueDateTypeCode>{allowed_code}</ram:DueDateTypeCode>"),
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
    let cii_due_bad = cii.clone().insert_after(
        "ApplicableHeaderTradeSettlement/ApplicableTradeTax/CategoryCode",
        "<ram:DueDateTypeCode>99</ram:DueDateTypeCode>",
    );
    assert_has_code(cii_due_bad.as_bytes(), "BR-RO-040");

    // PaymentMeans is optional in EN 16931 / CIUS-RO CII D16B
    let cii_no_pm = cii.clone().remove("SpecifiedTradeSettlementPaymentMeans");
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
        cii.clone()
            .set_text("SellerTradeParty/PostalTradeAddress/CityName", "Bucuresti")
            .as_bytes(),
        "BR-RO-100",
    );
    assert_has_code(
        cii.clone()
            .set_text(
                "BuyerTradeParty/PostalTradeAddress/CountrySubDivisionName",
                "Cluj",
            )
            .as_bytes(),
        "BR-RO-110",
    );

    // BR-RO-DT001: invalid 8-digit calendar date (month 13) and hyphenated date in CII
    assert_has_code(
        cii.clone()
            .set_text("ExchangedDocument/IssueDateTime/DateTimeString", "20261315")
            .as_bytes(),
        "BR-RO-DT001",
    );
    assert_has_code(
        cii.clone()
            .set_text(
                "ExchangedDocument/IssueDateTime/DateTimeString",
                "2026-10-15",
            )
            .as_bytes(),
        "BR-RO-DT001",
    );

    // BR-RO-Z2 on CII GrandTotalAmount (3 decimals)
    assert_has_code(
        cii.set_text("GrandTotalAmount", "2380.000").as_bytes(),
        "BR-RO-Z2",
    );
}

#[test]
fn cii_d16b_notes_length_tax_rep_and_namespace_strictness() {
    let cii = XmlFixture::load("06_cii_d16b_compliant_ro.xml");

    let mut many_cii_notes = String::new();
    for i in 0..21 {
        let _ = writeln!(
            many_cii_notes,
            "<ram:IncludedNote><ram:Content>Nota {i}</ram:Content></ram:IncludedNote>"
        );
    }
    assert_has_code(
        cii.clone()
            .append_child("ExchangedDocument", &many_cii_notes)
            .as_bytes(),
        "BR-RO-A020",
    );

    assert_has_code(
        cii.clone()
            .set_text("SpecifiedTradeProduct/Name", &"N".repeat(101))
            .as_bytes(),
        "BR-RO-L100",
    );

    let cii_with_tax_rep = cii.append_child(
        "ApplicableHeaderTradeAgreement",
        "<ram:SellerTaxRepresentativeTradeParty>\
             <ram:Name>Reprezentant Fiscal SRL</ram:Name>\
             <ram:PostalTradeAddress>\
                 <ram:LineOne>Calea Victoriei 10</ram:LineOne>\
                 <ram:CityName>Bucuresti</ram:CityName>\
                 <ram:CountryID>RO</ram:CountryID>\
                 <ram:CountrySubDivisionName>RO-B</ram:CountrySubDivisionName>\
             </ram:PostalTradeAddress>\
             <ram:SpecifiedTaxRegistration><ram:ID schemeID=\"VA\">RO45678918</ram:ID></ram:SpecifiedTaxRegistration>\
         </ram:SellerTaxRepresentativeTradeParty>",
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
fn anaf_schematron_v109_foreign_seller_br_ro_030_and_cii_tax_point_date() {
    let ubl = XmlFixture::load("01_standard_b2b_ro_ubl.xml");
    let cii = XmlFixture::load("06_cii_d16b_compliant_ro.xml");

    // 1. BR-RO-030 applies regardless of Seller country (IT Seller invoicing in EUR without TaxCurrencyCode=RON)
    let foreign_seller_eur = ubl
        .set_text("DocumentCurrencyCode", "EUR")
        .set_attr_all("currencyID", "RON", "EUR")
        .set_text(
            "AccountingSupplierParty/Party/PostalAddress/CityName",
            "Milano",
        )
        .remove("AccountingSupplierParty/Party/PostalAddress/CountrySubentity")
        .set_text(
            "AccountingSupplierParty/Party/PostalAddress/Country/IdentificationCode",
            "IT",
        )
        .set_text(
            "AccountingSupplierParty/Party/PartyTaxScheme/CompanyID",
            "IT12345678901",
        )
        .set_text(
            "AccountingSupplierParty/Party/PartyLegalEntity/CompanyID",
            "MI-123456",
        );
    assert_has_code(foreign_seller_eur.as_bytes(), "BR-RO-030");

    // 2. CII D16B TaxPointDate uses udt:DateString format="102" (YYYYMMDD)
    let cii_tax_point_ok = cii.clone().insert_after(
        "ApplicableHeaderTradeSettlement/ApplicableTradeTax/CategoryCode",
        "<ram:TaxPointDate><udt:DateString format=\"102\">20261015</udt:DateString></ram:TaxPointDate>",
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

    let cii_tax_point_bad = cii.clone().insert_after(
        "ApplicableHeaderTradeSettlement/ApplicableTradeTax/CategoryCode",
        "<ram:TaxPointDate><udt:DateString format=\"102\">2026-10-15</udt:DateString></ram:TaxPointDate>",
    );
    assert_has_code(cii_tax_point_bad.as_bytes(), "BR-RO-DT002");

    let cii_missing_format_attr = cii.replace_element(
        "ExchangedDocument/IssueDateTime/DateTimeString",
        "<udt:DateTimeString>20261015</udt:DateTimeString>",
    );
    assert_has_code(cii_missing_format_attr.as_bytes(), "BR-RO-DT001");
}
