//! UBL 2.1 `Invoice` & `CreditNote` `BR-RO-*` single-fault mutations, B2C `CNP` / OUG 69/2024
//! placeholder, subdivisions, dates, lengths, 2-decimal caps, and fiscal checksum tests.

use std::fmt::Write as _;

use cius_ro::{Severity, ValidateOptions, validate_bytes};

use crate::helpers::{XmlFixture, assert_has_code};

#[test]
fn b2c_cnp_and_anaf_oug_69_2024_placeholder_compliance() {
    let base = XmlFixture::load("01_standard_b2b_ro_ubl.xml");

    // 1. OUG 69/2024 B2C 13-zero placeholder (no VAT PartyTaxScheme on natural-person buyer)
    let b2c_zeros = base
        .remove("AccountingCustomerParty/Party/PartyTaxScheme")
        .set_text(
            "AccountingCustomerParty/Party/PartyLegalEntity/CompanyID",
            "0000000000000",
        );

    let opts = ValidateOptions::new().deny_warnings(true);
    let rep_zeros = validate_bytes(b2c_zeros.as_bytes(), &opts);
    assert!(
        rep_zeros.is_valid && rep_zeros.violations.is_empty(),
        "OUG 69/2024 B2C 0000000000000 must pass cleanly: {:?}",
        rep_zeros.violations
    );

    // 2. Valid 13-digit Romanian CNP (1900101410011)
    let b2c_valid_cnp = b2c_zeros.clone().set_text(
        "AccountingCustomerParty/Party/PartyLegalEntity/CompanyID",
        "1900101410011",
    );
    let rep_valid_cnp = validate_bytes(b2c_valid_cnp.as_bytes(), &opts);
    assert!(
        rep_valid_cnp.is_valid && rep_valid_cnp.violations.is_empty(),
        "Valid 13-digit CNP must pass cleanly: {:?}",
        rep_valid_cnp.violations
    );

    // 3. Invalid 13-digit CNP check digit (1900101410019) -> BR-RO-CNP warning
    let b2c_bad_cnp = b2c_zeros.set_text(
        "AccountingCustomerParty/Party/PartyLegalEntity/CompanyID",
        "1900101410019",
    );
    let rep_bad_cnp_default = validate_bytes(b2c_bad_cnp.as_bytes(), &ValidateOptions::default());
    assert!(rep_bad_cnp_default.is_valid);
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
fn ubl_br_ro_core_and_party_mutations() {
    let ubl = XmlFixture::load("01_standard_b2b_ro_ubl.xml");
    let cn = XmlFixture::load("05_credit_note_ro.xml");

    assert_has_code(
        ubl.clone()
            .set_text(
                "CustomizationID",
                "urn:cen.eu:en16931:2017#compliant#urn:efactura.mfinante.ro:CIUS-RO:0.9.0",
            )
            .as_bytes(),
        "BR-RO-001",
    );
    assert_has_code(
        ubl.clone()
            .set_text("Invoice/ID", "EFACT-RO-ABCD")
            .as_bytes(),
        "BR-RO-010",
    );

    let rep_inv_type = validate_bytes(
        ubl.clone().set_text("InvoiceTypeCode", "383").as_bytes(),
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
        cn.clone().set_text("CreditNoteTypeCode", "380").as_bytes(),
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
        ubl.clone()
            .set_text("DocumentCurrencyCode", "EUR")
            .set_attr_all("currencyID", "RON", "EUR")
            .as_bytes(),
        "BR-RO-030",
    );
    assert_has_code(
        ubl.clone()
            .insert_before(
                "AccountingSupplierParty",
                "<cac:InvoicePeriod><cbc:DescriptionCode>99</cbc:DescriptionCode></cac:InvoicePeriod>",
            )
            .as_bytes(),
        "BR-RO-040",
    );

    // PaymentMeans (BG-16 / BT-81) is optional in EN 16931 and official ANAF RO16931-rules.sch
    let ubl_no_pm = ubl.clone().remove("PaymentMeans");
    let rep_no_pm = validate_bytes(
        ubl_no_pm.as_bytes(),
        &ValidateOptions::default().deny_warnings(true),
    );
    assert!(
        rep_no_pm.is_valid && rep_no_pm.violations.is_empty(),
        "UBL Invoice without PaymentMeans must be valid: {:?}",
        rep_no_pm.violations
    );

    let cn_no_pm = cn.remove("PaymentMeans");
    let rep_cn_no_pm = validate_bytes(
        cn_no_pm.as_bytes(),
        &ValidateOptions::default().deny_warnings(true),
    );
    assert!(
        rep_cn_no_pm.is_valid && rep_cn_no_pm.violations.is_empty(),
        "UBL CreditNote without PaymentMeans must be valid: {:?}",
        rep_cn_no_pm.violations
    );

    let ubl_no_seller_tax = ubl.remove("AccountingSupplierParty/Party/PartyTaxScheme");
    assert_has_code(ubl_no_seller_tax.as_bytes(), "BR-RO-065");
}

#[test]
fn ubl_br_ro_cardinality_and_party_address_mutations() {
    let ubl = XmlFixture::load("01_standard_b2b_ro_ubl.xml");

    let mut many_notes = String::new();
    for i in 0..21 {
        let _ = writeln!(many_notes, "<cbc:Note>Nota {i}</cbc:Note>");
    }
    assert_has_code(
        ubl.clone()
            .insert_before("DocumentCurrencyCode", &many_notes)
            .as_bytes(),
        "BR-RO-A020",
    );

    let mut many_docs = String::new();
    for i in 0..51 {
        let _ = writeln!(
            many_docs,
            "<cac:AdditionalDocumentReference><cbc:ID>DOC-{i}</cbc:ID></cac:AdditionalDocumentReference>"
        );
    }
    assert_has_code(
        ubl.clone()
            .insert_before("AccountingSupplierParty", &many_docs)
            .as_bytes(),
        "BR-RO-A050",
    );

    assert_has_code(
        ubl.clone()
            .remove("AccountingSupplierParty/Party/PostalAddress/StreetName")
            .as_bytes(),
        "BR-RO-080",
    );
    assert_has_code(
        ubl.remove("AccountingCustomerParty/Party/PostalAddress/CityName")
            .as_bytes(),
        "BR-RO-090",
    );
}

#[test]
fn ubl_br_ro_subdivision_mutations() {
    let ubl = XmlFixture::load("01_standard_b2b_ro_ubl.xml");

    let bad_county = ubl.clone().set_text(
        "AccountingCustomerParty/Party/PostalAddress/CountrySubentity",
        "Cluj",
    );
    let rep_county = validate_bytes(bad_county.as_bytes(), &ValidateOptions::default());
    let v_110 = rep_county
        .violations
        .iter()
        .find(|v| v.code == "BR-RO-110")
        .expect("BR-RO-110");
    assert!(v_110.hint.as_deref().is_some_and(|h| h.contains("RO-CJ")));

    let bad_sector = ubl.clone().set_text(
        "AccountingSupplierParty/Party/PostalAddress/CityName",
        "Sector 1",
    );
    let rep_sector = validate_bytes(bad_sector.as_bytes(), &ValidateOptions::default());
    let v_100 = rep_sector
        .violations
        .iter()
        .find(|v| v.code == "BR-RO-100")
        .expect("BR-RO-100");
    assert!(v_100.hint.as_deref().is_some_and(|h| h.contains("SECTOR1")));

    let no_buyer_ids = ubl
        .clone()
        .remove("AccountingCustomerParty/Party/PartyTaxScheme")
        .remove("AccountingCustomerParty/Party/PartyLegalEntity/CompanyID");
    assert_has_code(no_buyer_ids.as_bytes(), "BR-RO-120");

    let with_tax_rep_and_deliv = ubl.insert_before(
        "PaymentMeans",
        "<cac:TaxRepresentativeParty>\
             <cac:PartyName><cbc:Name>Rep Fiscal SRL</cbc:Name></cac:PartyName>\
             <cac:PostalAddress>\
               <cbc:StreetName>Str. Victoriei 1</cbc:StreetName>\
               <cbc:CityName>Bucuresti</cbc:CityName>\
               <cbc:CountrySubentity>RO-B</cbc:CountrySubentity>\
               <cac:Country><cbc:IdentificationCode>RO</cbc:IdentificationCode></cac:Country>\
             </cac:PostalAddress>\
             <cac:PartyTaxScheme><cbc:CompanyID>RO45678918</cbc:CompanyID><cac:TaxScheme><cbc:ID>VAT</cbc:ID></cac:TaxScheme></cac:PartyTaxScheme>\
           </cac:TaxRepresentativeParty>\
           <cac:Delivery>\
             <cac:DeliveryLocation>\
               <cac:Address>\
                 <cbc:StreetName>Bd. Unirii 10</cbc:StreetName>\
                 <cbc:CityName>Bucuresti</cbc:CityName>\
                 <cbc:CountrySubentity>RO-B</cbc:CountrySubentity>\
                 <cac:Country><cbc:IdentificationCode>RO</cbc:IdentificationCode></cac:Country>\
               </cac:Address>\
             </cac:DeliveryLocation>\
           </cac:Delivery>",
    );
    assert_has_code(with_tax_rep_and_deliv.as_bytes(), "BR-RO-160");
    assert_has_code(with_tax_rep_and_deliv.as_bytes(), "BR-RO-200");

    let bad_tax_rep_deliv_county = with_tax_rep_and_deliv
        .set_text(
            "TaxRepresentativeParty/PostalAddress/CountrySubentity",
            "RO-XX",
        )
        .set_text(
            "Delivery/DeliveryLocation/Address/CountrySubentity",
            "RO-XX",
        );
    assert_has_code(bad_tax_rep_deliv_county.as_bytes(), "BR-RO-170");
    assert_has_code(bad_tax_rep_deliv_county.as_bytes(), "BR-RO-210");
}

#[test]
fn ubl_br_ro_date_length_decimal_and_xsd_mutations() {
    let ubl = XmlFixture::load("01_standard_b2b_ro_ubl.xml");

    assert_has_code(
        ubl.clone().set_text("IssueDate", "2026-10-15Z").as_bytes(),
        "BR-RO-DT001",
    );
    assert_has_code(
        ubl.clone()
            .set_text("InvoiceLine/Item/Name", &"A".repeat(101))
            .as_bytes(),
        "BR-RO-L100",
    );
    assert_has_code(
        ubl.clone()
            .set_text("PaymentTerms/Note", &"P".repeat(301))
            .as_bytes(),
        "BR-RO-L300",
    );
    assert_has_code(
        ubl.clone()
            .set_text("LegalMonetaryTotal/LineExtensionAmount", "4500.000")
            .as_bytes(),
        "BR-RO-Z2",
    );

    let price_4_decimals = ubl
        .clone()
        .set_text("InvoiceLine/Price/PriceAmount", "4500.0000");
    let rep_price4 = validate_bytes(price_4_decimals.as_bytes(), &ValidateOptions::default());
    assert!(rep_price4.is_valid && rep_price4.violations.is_empty());

    let neg_price = ubl
        .clone()
        .set_text("InvoiceLine/Price/PriceAmount", "-4500.00");
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
        ubl.clone()
            .set_text(
                "AccountingSupplierParty/Party/PartyTaxScheme/CompanyID",
                "RO45678919",
            )
            .as_bytes(),
        "BR-RO-CUI",
    );
    assert_has_code(
        ubl.clone()
            .set_text(
                "AccountingSupplierParty/Party/PartyLegalEntity/CompanyID",
                "J99/1234/2022",
            )
            .as_bytes(),
        "BR-RO-ONRC",
    );
    assert_has_code(
        ubl.clone()
            .set_text("PayeeFinancialAccount/ID", "RO00AAAA1B31007593840000")
            .as_bytes(),
        "BR-RO-IBAN",
    );

    #[cfg(feature = "xsd")]
    {
        let bad_xsd_xml = ubl.insert_after(
            "Invoice/ID",
            "<cbc:IllegalNonExistentElement>1</cbc:IllegalNonExistentElement>",
        );
        assert_has_code(bad_xsd_xml.as_bytes(), "XSD-SCHEMA-001");
    }
}

#[test]
fn xsd_boolean_1_credit_note_and_foreign_vat_edges() {
    let ubl = XmlFixture::load("01_standard_b2b_ro_ubl.xml");
    let cn = XmlFixture::load("05_credit_note_ro.xml");
    let eur = XmlFixture::load("04_reverse_charge_ro.xml");

    // 1. XSD xs:boolean `<cbc:ChargeIndicator>1</cbc:ChargeIndicator>` with 3-decimal Amount
    //    must be classified as a Charge (BR-DEC-RO-05), not an Allowance (BR-DEC-RO-01).
    let charge_bool_1 = ubl.insert_before(
        "TaxTotal",
        "<cac:AllowanceCharge>\
             <cbc:ChargeIndicator>1</cbc:ChargeIndicator>\
             <cbc:AllowanceChargeReason>Taxă transport</cbc:AllowanceChargeReason>\
             <cbc:Amount currencyID=\"RON\">10.125</cbc:Amount>\
             <cac:TaxCategory><cbc:ID>S</cbc:ID><cbc:Percent>19.00</cbc:Percent><cac:TaxScheme><cbc:ID>VAT</cbc:ID></cac:TaxScheme></cac:TaxCategory>\
           </cac:AllowanceCharge>",
    );
    let rep_charge1 = validate_bytes(charge_bool_1.as_bytes(), &ValidateOptions::default());
    assert!(
        rep_charge1
            .violations
            .iter()
            .any(|v| v.code == "BR-RO-Z2" && v.rule_sub_id.as_deref() == Some("BR-DEC-RO-05")),
        "Expected BR-DEC-RO-05 for ChargeIndicator=1, got: {:?}",
        rep_charge1.violations
    );

    // 2. CreditNote (381) PaymentDueDate with timezone -> BR-RO-DT003-CN (sub-ID BR-RO-DT003_CN)
    let cn_bad_due = cn
        .clone()
        .set_text("PaymentMeans/PaymentDueDate", "2026-11-24Z");
    let rep_cn_due = validate_bytes(cn_bad_due.as_bytes(), &ValidateOptions::default());
    assert!(
        rep_cn_due.violations.iter().any(|v| {
            v.code == "BR-RO-DT003-CN" && v.rule_sub_id.as_deref() == Some("BR-RO-DT003_CN")
        }),
        "Expected BR-RO-DT003-CN on CreditNote, got: {:?}",
        rep_cn_due.violations
    );

    // 3. CreditNote (381) 3-decimal CreditNoteLine/LineExtensionAmount
    let cn_bad_line_dec = cn.set_text("CreditNoteLine/LineExtensionAmount", "600.000");
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
    let eur_bad_ron_vat =
        eur.set_text_where_attr("Invoice/TaxTotal/TaxAmount", "currencyID", "RON", "0.000");
    let rep_eur_vat = validate_bytes(eur_bad_ron_vat.as_bytes(), &ValidateOptions::default());
    assert!(
        rep_eur_vat
            .violations
            .iter()
            .any(|v| v.code == "BR-RO-Z2" && v.rule_sub_id.as_deref() == Some("BR-DEC-RO-15")),
        "Expected BR-DEC-RO-15 on accounting-currency TaxAmount, got: {:?}",
        rep_eur_vat.violations
    );
}

#[test]
fn bt1_and_bt19_length_caps_and_whitespace_normalization() {
    let ubl = XmlFixture::load("01_standard_b2b_ro_ubl.xml");

    // BT-1 (Invoice number): 200 chars passes, 201 chars triggers BR-RO-L200 (sub-ID BR-RO-L155)
    let id_200 = format!("RO2026{}", "1".repeat(194));
    assert_eq!(id_200.len(), 200);
    let rep_id_ok = validate_bytes(
        ubl.clone().set_text("Invoice/ID", &id_200).as_bytes(),
        &ValidateOptions::default(),
    );
    assert!(rep_id_ok.is_valid, "{:?}", rep_id_ok.violations);

    let id_201 = format!("RO2026{}", "1".repeat(195));
    assert_eq!(id_201.len(), 201);
    let rep_id_bad = validate_bytes(
        ubl.clone().set_text("Invoice/ID", &id_201).as_bytes(),
        &ValidateOptions::default(),
    );
    assert!(
        rep_id_bad
            .violations
            .iter()
            .any(|v| v.code == "BR-RO-L200" && v.rule_sub_id.as_deref() == Some("BR-RO-L155")),
        "Expected BR-RO-L200 / BR-RO-L155 for 201-char BT-1, got: {:?}",
        rep_id_bad.violations
    );

    // BT-19 (AccountingCost): 100 chars passes, 101 chars triggers BR-RO-L100 (sub-ID BR-RO-L1001)
    let acct_100 = "C".repeat(100);
    let with_acct_100 = ubl.clone().insert_after(
        "DocumentCurrencyCode",
        &format!("<cbc:AccountingCost>{acct_100}</cbc:AccountingCost>"),
    );
    let rep_acct_ok = validate_bytes(with_acct_100.as_bytes(), &ValidateOptions::default());
    assert!(rep_acct_ok.is_valid, "{:?}", rep_acct_ok.violations);

    let acct_101 = "C".repeat(101);
    let with_acct_101 = ubl.clone().insert_after(
        "DocumentCurrencyCode",
        &format!("<cbc:AccountingCost>{acct_101}</cbc:AccountingCost>"),
    );
    let rep_acct_bad = validate_bytes(with_acct_101.as_bytes(), &ValidateOptions::default());
    assert!(
        rep_acct_bad
            .violations
            .iter()
            .any(|v| v.code == "BR-RO-L100" && v.rule_sub_id.as_deref() == Some("BR-RO-L1001")),
        "Expected BR-RO-L100 / BR-RO-L1001 for 101-char BT-19, got: {:?}",
        rep_acct_bad.violations
    );

    // XPath normalize-space(): 95-char string padded with 20 leading/trailing spaces -> PASS
    let padded_acct = format!("          {}          ", "D".repeat(95));
    let with_padded = ubl.insert_after(
        "DocumentCurrencyCode",
        &format!("<cbc:AccountingCost>{padded_acct}</cbc:AccountingCost>"),
    );
    let rep_padded = validate_bytes(with_padded.as_bytes(), &ValidateOptions::default());
    assert!(rep_padded.is_valid, "{:?}", rep_padded.violations);
}

#[test]
fn lexical_dates_scoped_and_secondary_party_cui_checked() {
    let ubl = XmlFixture::load("01_standard_b2b_ro_ubl.xml");

    // 1. Optional non-CIUS-RO OrderReference/IssueDate must NOT falsely trigger BR-RO-DT001 when XSD is off
    let with_order_issue_date = ubl.clone().insert_before(
        "AccountingSupplierParty",
        "<cac:OrderReference><cbc:ID>PO-99</cbc:ID><cbc:IssueDate>2026-10-15+02:00</cbc:IssueDate></cac:OrderReference>",
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
    let secondary_bad_cui = ubl.clone().insert_after(
        "AccountingSupplierParty/Party/PartyTaxScheme",
        "<cac:PartyTaxScheme><cbc:CompanyID>45678919</cbc:CompanyID><cac:TaxScheme><cbc:ID>CIF</cbc:ID></cac:TaxScheme></cac:PartyTaxScheme>",
    );
    let rep_sec_cui = validate_bytes(secondary_bad_cui.as_bytes(), &ValidateOptions::default());
    assert!(
        rep_sec_cui.violations.iter().any(|v| v.code == "BR-RO-CUI"),
        "Expected secondary invalid CUI on BT-32 to emit BR-RO-CUI, got: {:?}",
        rep_sec_cui.violations
    );

    // 3. Duplicate identical invalid CUI in BT-31 and BT-32 emits BR-RO-CUI only once
    let dup_bad_cui = ubl
        .set_text(
            "AccountingSupplierParty/Party/PartyTaxScheme/CompanyID",
            "RO45678919",
        )
        .insert_after(
            "AccountingSupplierParty/Party/PartyTaxScheme",
            "<cac:PartyTaxScheme><cbc:CompanyID>RO45678919</cbc:CompanyID><cac:TaxScheme><cbc:ID>CIF</cbc:ID></cac:TaxScheme></cac:PartyTaxScheme>",
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
fn anaf_schematron_v109_length_caps_and_sub_ids() {
    let ubl = XmlFixture::load("01_standard_b2b_ro_ubl.xml");

    // 1. BT-10 (150 chars) and BT-11 (250 chars) have NO length cap in ANAF RO16931-rules.sch
    let bt10_bt11_long = ubl
        .clone()
        .set_text("BuyerReference", &"B".repeat(150))
        .insert_after(
            "BuyerReference",
            &format!(
                "<cac:ProjectReference><cbc:ID>{}</cbc:ID></cac:ProjectReference>",
                "P".repeat(250)
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
    let r = "R".repeat(201);
    let doc_refs_over = ubl
        .insert_before(
            "DocumentCurrencyCode",
            &format!("<cbc:Note>{}</cbc:Note>", "N".repeat(301)),
        )
        .insert_before(
            "AccountingSupplierParty",
            &format!(
                "<cac:OrderReference><cbc:ID>{r}</cbc:ID><cbc:SalesOrderID>{r}</cbc:SalesOrderID></cac:OrderReference>\
                 <cac:DespatchDocumentReference><cbc:ID>{r}</cbc:ID></cac:DespatchDocumentReference>\
                 <cac:ReceiptDocumentReference><cbc:ID>{r}</cbc:ID></cac:ReceiptDocumentReference>\
                 <cac:OriginatorDocumentReference><cbc:ID>{r}</cbc:ID></cac:OriginatorDocumentReference>\
                 <cac:ContractDocumentReference><cbc:ID>{r}</cbc:ID></cac:ContractDocumentReference>"
            ),
        )
        .set_text("PaymentTerms/Note", &"T".repeat(301));
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
fn lexical_decimal_and_non_xpath_whitespace_edge_cases() {
    let ubl = XmlFixture::load("01_standard_b2b_ro_ubl.xml");

    for bad_amount in ["5355.12abc", "5355.1e-5"] {
        let mutated = ubl
            .clone()
            .set_text("LegalMonetaryTotal/PayableAmount", bad_amount);
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
    let with_nbsp = ubl.insert_after(
        "DocumentCurrencyCode",
        &format!("<cbc:AccountingCost>{nbsp_101}</cbc:AccountingCost>"),
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
