use cius_ro_id::{
    compute_cnp_check_digit, compute_cui_check_digit, compute_ro_iban_check_digits,
    inspect_identifier, is_anaf_b2c_placeholder, parse_cui_candidate, BucharestSector, Cnp,
    CnpError, CnpOrPlaceholder, County, Cui, CuiError, IbanError, Onrc, OnrcFormat, OnrcRegister,
    RoIban, ANAF_B2C_PLACEHOLDER, ISO_3166_2_RO_CODES,
};
use proptest::prelude::*;

#[test]
fn test_real_and_official_cui_vectors() {
    // Real Romanian public & sample CUIs (Ministry of Finance, ANAF, municipalities, stdnum.ro vectors)
    let valid_cuis = [
        "19",          // 2-digit minimum valid CUI (1*2 = 2 -> (20)%11 = 9)
        "RO 18547290", // stdnum.ro.cf canonical test vector
        "18547290",    // without RO prefix
        "RO4221306",   // Ministry of Finance (Ministerul Finanțelor: 4221306)
        "RO16059209",  // Valid 8-digit CUI (check digit 9)
        "RO4269282",   // Primăria Municipiului Timișoara
        "RO45678918",  // VeriFactura golden sample seller
        "RO98765438",  // VeriFactura golden sample buyer
        "38192043",    // Art. 310 small-business exempt FC tax scheme
        "RO31224458",  // Buyer in 03_small_business_exempt_ro.xml
        "RO33445560",  // Seller in 04_reverse_charge_ro.xml (rem == 10 -> check digit 0!)
        "ro 45678918", // lowercase + space
    ];

    for raw in valid_cuis {
        let parsed = Cui::parse(raw).unwrap_or_else(|e| panic!("Expected {raw} to be valid: {e}"));
        assert!(parsed.to_vat_id().starts_with("RO"));
        assert!(parsed.number() >= 19);
    }

    // Invalid CUI vectors
    assert!(matches!(
        Cui::parse("RO45678912"),
        Err(CuiError::InvalidChecksum {
            expected: 8,
            actual: 2
        })
    ));
    assert_eq!(Cui::parse(""), Err(CuiError::Empty));
    assert_eq!(Cui::parse("RO"), Err(CuiError::Empty));
    assert_eq!(Cui::parse("0123456"), Err(CuiError::LeadingZero));
    assert_eq!(Cui::parse("5"), Err(CuiError::InvalidLength { len: 1 }));
    assert_eq!(
        Cui::parse("12345678901"),
        Err(CuiError::InvalidLength { len: 11 })
    );
    assert_eq!(Cui::parse("RO12AB34"), Err(CuiError::InvalidCharacters));

    // Candidate discriminator ignores 13-digit CNP / B2C and ONRC
    assert_eq!(parse_cui_candidate(ANAF_B2C_PLACEHOLDER), None);
    assert_eq!(parse_cui_candidate("J40/1234/2022"), None);
    assert_eq!(parse_cui_candidate("RO-CJ"), None);
    assert_eq!(parse_cui_candidate("RO49AAAA1B31007593840000"), None);
    assert!(matches!(parse_cui_candidate("RO45678912"), Some(Err(_))));
}

#[test]
fn test_cnp_and_anaf_b2c_placeholder_vectors() {
    assert!(is_anaf_b2c_placeholder("0000000000000"));
    assert!(is_anaf_b2c_placeholder(" 0000000000000\n"));
    assert_eq!(
        Cnp::parse("0000000000000"),
        Err(CnpError::AnafB2cPlaceholder)
    );
    assert_eq!(
        CnpOrPlaceholder::parse("0000000000000"),
        Ok(CnpOrPlaceholder::AnafB2cPlaceholder)
    );

    // stdnum.ro.cnp classic vector: 1630615123457
    let cnp1 = Cnp::parse("1630615123457").expect("stdnum vector must pass");
    assert_eq!(cnp1.birth_year(), Some(1963));
    assert_eq!(cnp1.birth_month(), 6);
    assert_eq!(cnp1.birth_day(), 15);
    assert_eq!(cnp1.county_code(), 12);

    // Leap year 2024-02-29 in nationwide county code 70 (`524022970001x`)
    let chk_leap = compute_cnp_check_digit("524022970001").unwrap();
    let leap_cnp_str = format!("524022970001{chk_leap}");
    let leap_cnp = Cnp::parse(&leap_cnp_str).expect("2024-02-29 county 70 must pass");
    assert_eq!(leap_cnp.birth_year(), Some(2024));
    assert_eq!(leap_cnp.county_code(), 70);

    // Non-leap year 1900-02-29 (`300022940001x`) must fail calendar validation!
    let chk_1900 = compute_cnp_check_digit("100022940001").unwrap();
    let bad_1900 = format!("100022940001{chk_1900}");
    assert!(matches!(
        Cnp::parse(&bad_1900),
        Err(CnpError::InvalidDate { .. })
    ));

    // Invalid county 49, sequence 000, sex digit 0
    assert!(matches!(
        Cnp::parse("1900101490010"),
        Err(CnpError::InvalidCountyCode(49))
    ));
    assert!(matches!(
        Cnp::parse("1900101410000"),
        Err(CnpError::InvalidSequence)
    ));
    assert!(matches!(
        Cnp::parse("0900101410010"),
        Err(CnpError::InvalidSexDigit(0))
    ));
}

#[test]
fn test_onrc_classic_and_law265_formats() {
    let classic = Onrc::parse("J40/1234/2022").unwrap();
    assert_eq!(classic.register(), OnrcRegister::J);
    assert_eq!(
        classic.format(),
        &OnrcFormat::Classic {
            county: 40,
            sequence: 1234,
            year: 2022
        }
    );
    assert_eq!(classic.to_string(), "J40/1234/2022");

    // Leading-zero county normalization: J8/512/2023 -> J08/512/2023
    let bv = Onrc::parse("J8/512/2023").unwrap();
    assert_eq!(bv.to_string(), "J08/512/2023");

    // PFA (F) & Cooperative (C) in Călărași (51) and Giurgiu (52)
    assert_eq!(
        Onrc::parse("F51/99/2021").unwrap().register(),
        OnrcRegister::F
    );
    assert_eq!(
        Onrc::parse("C52/7/2020").unwrap().register(),
        OnrcRegister::C
    );

    // Law 265/2022 14-char format (J + YYYY + NNNNNN + JJ + C)
    let modern = Onrc::parse("J2024001234005").unwrap();
    assert_eq!(modern.register(), OnrcRegister::J);
    assert_eq!(modern.to_string(), "J2024001234005");

    // Classic 2003–2008 DD.MM.YYYY certificate format (`J40/1234/15.03.2005`) + whitespace candidate
    let cert_date = Onrc::parse("J 40 / 1234 / 15.03.2005").unwrap();
    assert_eq!(cert_date.to_string(), "J40/1234/2005");
    assert!(cius_ro_id::is_onrc_candidate("J 40/1234/2005"));

    // Invalid county 45 in classic ONRC
    assert!(Onrc::parse("J45/100/2022").is_err());
    assert!(Onrc::parse("X40/100/2022").is_err());
    assert!(Onrc::parse("J40/1234/35.13.2005").is_err());
}

#[test]
fn test_ro_iban_vectors() {
    let iban = RoIban::parse("RO49 AAAA 1B31 0075 9384 0000").unwrap();
    assert_eq!(iban.as_str(), "RO49AAAA1B31007593840000");
    assert_eq!(iban.check_digits(), 49);
    assert_eq!(iban.bank_code(), "AAAA");
    assert_eq!(iban.account_number(), "1B31007593840000");
    assert_eq!(iban.formatted(), "RO49 AAAA 1B31 0075 9384 0000");

    assert!(matches!(
        RoIban::parse("RO00AAAA1B31007593840000"),
        Err(IbanError::InvalidChecksum {
            expected: 49,
            actual: 0
        })
    ));
    assert_eq!(
        RoIban::parse("DE89370400440532013000"),
        Err(IbanError::NotRomanianCountryCode)
    );
}

#[test]
fn test_iso_3166_2_ro_and_bucharest_sectors() {
    assert_eq!(County::all().count(), 42);
    for code in ISO_3166_2_RO_CODES {
        let c = County::from_code(code).unwrap_or_else(|| panic!("Missing {code}"));
        assert_eq!(c.code(), code);
    }

    assert_eq!(County::lookup_fuzzy("CLUJ"), Some(County::CJ));
    assert_eq!(County::lookup_fuzzy("Jud. Brașov"), Some(County::BV));
    assert_eq!(
        County::lookup_fuzzy("Municipiul București"),
        Some(County::B)
    );
    assert_eq!(County::lookup_fuzzy("Timis"), Some(County::TM));

    assert_eq!(
        BucharestSector::from_code("SECTOR1"),
        Some(BucharestSector::Sector1)
    );
    assert_eq!(BucharestSector::from_code("Sector 1"), None);
    assert_eq!(
        BucharestSector::normalize_fuzzy("Sector 1"),
        Some(BucharestSector::Sector1)
    );
    assert_eq!(
        BucharestSector::normalize_fuzzy("București, Sectorul 3"),
        Some(BucharestSector::Sector3)
    );

    // FromStr returns std::error::Error-compatible SubdivisionError
    let parsed_cj: Result<County, Box<dyn std::error::Error>> =
        "RO-CJ".parse::<County>().map_err(Into::into);
    assert_eq!(parsed_cj.unwrap(), County::CJ);
    assert!("CLUJ".parse::<County>().is_err());
    assert!("Sector 1".parse::<BucharestSector>().is_err());

    // Unified inspector localized check
    let rep_ro = inspect_identifier("RO45678918", true);
    assert!(rep_ro.is_valid);
    assert_eq!(rep_ro.kind, "cui");
    assert!(rep_ro.details.contains("valid"));
}

proptest! {
    #[test]
    fn prop_cui_roundtrip_and_single_digit_corruption(body in "[1-9][0-9]{0,8}", prefix in any::<bool>()) {
        let check = compute_cui_check_digit(&body).unwrap();
        let full = if prefix {
            format!("RO{body}{check}")
        } else {
            format!("{body}{check}")
        };
        let parsed = Cui::parse(&full).unwrap();
        prop_assert_eq!(parsed.check_digit(), check);
        prop_assert_eq!(parsed.has_ro_prefix(), prefix);

        // Perturbing the trailing check digit MUST always fail checksum verification
        let wrong_check = (check + 1) % 10;
        let corrupted = format!("RO{body}{wrong_check}");
        let is_bad_checksum = matches!(
            Cui::parse(&corrupted),
            Err(CuiError::InvalidChecksum { .. })
        );
        prop_assert!(is_bad_checksum);
    }

    #[test]
    fn prop_iban_roundtrip_and_corruption(
        bank in "[A-Z]{4}",
        acct in "[A-Z0-9]{16}",
    ) {
        let chk = compute_ro_iban_check_digits(&bank, &acct).unwrap();
        let iban_str = format!("RO{chk:02}{bank}{acct}");
        let parsed = RoIban::parse(&iban_str).unwrap();
        prop_assert_eq!(parsed.check_digits(), chk);
        prop_assert_eq!(parsed.bank_code(), bank.as_str());
        prop_assert_eq!(parsed.account_number(), acct.as_str());

        let bad_chk = if chk == 98 { 2 } else { chk + 1 };
        let bad_iban = format!("RO{bad_chk:02}{bank}{acct}");
        prop_assert!(RoIban::parse(&bad_iban).is_err());
    }
}
