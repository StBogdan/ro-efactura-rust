//! # `cius-ro-id` — Romanian Fiscal, Legal, Banking & Geographic Identifiers
//!
//! Pure-Rust, `no_std`-compatible parsers, normalizers, and checksum verifiers
//! for Romanian national identifiers used across **ANAF RO e-Factura (`CIUS-RO 1.0.1`)**, **SAF-T D406**,
//! **e-Transport**, and Romanian ERP/accounting software.
//!
//! *Verificatoare și analizoare pure în Rust (compatibile `no_std`)
//! pentru identificatorii fiscali, juridici, bancari și administrativi din România:
//! `CUI`/`CIF`, `CNP` (+ substituentul B2C `0000000000000` cf. OUG 69/2024), `ONRC` (clasic și Legea 265/2022),
//! `IBAN` RO (ISO 7064 MOD 97-10), codurile de județ `ISO 3166-2:RO` și sectoarele Municipiului București.*
//!
//! ## Supported Identifiers / Identificatori suportați
//!
//! - [`Cui`]: 2–10 digit *Cod Unic de Înregistrare* / *Cod de Înregistrare Fiscală* (`RO` VAT prefix optional),
//!   verified with the official Ministry of Finance modulo-11 weight vector `[7, 5, 3, 2, 1, 7, 5, 3, 2]`.
//! - [`Cnp`] & [`CnpOrPlaceholder`]: 13-digit *Cod Numeric Personal* (sex/century, leap-year calendar date,
//!   county `01..=48 | 51 | 52 | 70`, modulo-11 check digit) plus ANAF's `0000000000000` B2C placeholder.
//! - [`Onrc`]: Trade Register (*Registrul Comerțului*) numbers in both classic `J40/1234/2022` and
//!   Law 265/2022 electronic `J2024001234005` (`[JFC]\d{13}`) formats.
//! - [`RoIban`]: 24-character Romanian IBAN (`RO` + 2 check digits + 4-letter bank code + 16-char account)
//!   verified via ISO/IEC 7064 MOD 97-10.
//! - [`County`] & [`BucharestSector`]: All 42 `ISO 3166-2:RO` subdivisions (`RO-B`, `RO-AB`..`RO-VN`)
//!   and `SECTOR1`..`SECTOR6` codes with fuzzy normalization helpers.
//!
//! ## Quick Start
//!
//! ```
//! use cius_ro_id::{BucharestSector, CnpOrPlaceholder, County, Cui, Onrc, RoIban};
//!
//! let cui: Cui = "RO45678918".parse().unwrap();
//! assert_eq!(cui.digits(), "45678918");
//!
//! let b2c = CnpOrPlaceholder::parse("0000000000000").unwrap();
//! assert!(b2c.is_placeholder());
//!
//! let onrc: Onrc = "J40/1234/2022".parse().unwrap();
//! assert_eq!(onrc.to_string(), "J40/1234/2022");
//!
//! let iban: RoIban = "RO49 AAAA 1B31 0075 9384 0000".parse().unwrap();
//! assert_eq!(iban.bank_code(), "AAAA");
//!
//! assert_eq!(County::lookup_fuzzy("Jud. Cluj"), Some(County::CJ));
//! assert_eq!(BucharestSector::normalize_fuzzy("Sectorul 1"), Some(BucharestSector::Sector1));
//! ```

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

extern crate alloc;

pub mod cnp;
pub mod cui;
pub mod iban;
pub mod onrc;
pub mod subdivision;

pub use cnp::{
    compute_cnp_check_digit, is_anaf_b2c_placeholder, Cnp, CnpError, CnpOrPlaceholder,
    CnpSexCentury, ANAF_B2C_PLACEHOLDER, CNP_WEIGHTS,
};
pub use cui::{compute_cui_check_digit, parse_cui_candidate, Cui, CuiError, CUI_WEIGHTS};
pub use iban::{compute_ro_iban_check_digits, IbanError, RoIban};
pub use onrc::{is_onrc_candidate, Onrc, OnrcError, OnrcFormat, OnrcRegister};
pub use subdivision::{
    BucharestSector, County, SubdivisionError, BUCHAREST_SECTOR_CODES, ISO_3166_2_RO_CODES,
};

use alloc::format;
use alloc::string::{String, ToString};

/// Diagnostic report returned by [`inspect_identifier`] for CLI and WebAssembly tools.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct IdentifierInspection {
    /// Raw input string inspected.
    pub input: String,
    /// Detected identifier kind (`"cui"`, `"cnp"`, `"anaf_b2c_placeholder"`, `"onrc"`, `"iban"`, `"county"`, `"bucharest_sector"`, or `"unknown"`).
    pub kind: String,
    /// Whether the identifier passed all structural and checksum validations.
    pub is_valid: bool,
    /// Canonical normalized representation when valid (or suggested normalization when fuzzy-matched).
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub normalized: Option<String>,
    /// Localized diagnostic summary in English (`ro = false`) or Romanian (`ro = true`).
    pub details: String,
}

/// Auto-detects and validates any supported Romanian identifier string (`CUI`, `CNP`, `0000000000000`,
/// `ONRC`, `RO IBAN`, `ISO 3166-2:RO` county, or Bucharest `SECTOR1`..`SECTOR6`).
#[must_use]
pub fn inspect_identifier(raw: &str, ro: bool) -> IdentifierInspection {
    let trimmed = raw.trim();
    let compact_upper: String = trimmed
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .map(|c| c.to_ascii_uppercase())
        .collect();
    let build = |kind: &str, is_valid: bool, normalized: Option<String>, details: String| {
        IdentifierInspection {
            input: raw.to_string(),
            kind: kind.to_string(),
            is_valid,
            normalized,
            details,
        }
    };
    let loc = |en: String, ro_msg: String| if ro { ro_msg } else { en };

    if let Some(c) = County::from_code(&trimmed.to_ascii_uppercase()) {
        return build(
            "county",
            true,
            Some(c.code().to_string()),
            loc(
                format!(
                    "Valid ISO 3166-2:RO county code: {} ({}).",
                    c.code(),
                    c.name_ro()
                ),
                format!(
                    "Cod județ ISO 3166-2:RO valid: {} ({}).",
                    c.code(),
                    c.name_ro()
                ),
            ),
        );
    }

    if let Some(sec) = BucharestSector::from_code(&trimmed.to_ascii_uppercase()) {
        return build(
            "bucharest_sector",
            true,
            Some(sec.code().to_string()),
            loc(
                format!("Valid ANAF Bucharest sector code: {}.", sec.code()),
                format!("Cod sector București ANAF valid: {}.", sec.code()),
            ),
        );
    }
    if let Some(sec) = BucharestSector::normalize_fuzzy(trimmed) {
        return build(
            "bucharest_sector",
            false,
            Some(sec.code().to_string()),
            loc(
                format!(
                    "Non-canonical Bucharest sector notation; CIUS-RO requires '{}'.",
                    sec.code()
                ),
                format!(
                    "Denumire necanonică pentru sector București; în CIUS-RO utilizați '{}'.",
                    sec.code()
                ),
            ),
        );
    }

    if compact_upper.starts_with("RO")
        && compact_upper.len() >= 14
        && compact_upper[2..].bytes().any(|b| b.is_ascii_alphabetic())
    {
        return match RoIban::parse(trimmed) {
            Ok(iban) => build(
                "iban",
                true,
                Some(iban.as_str().to_string()),
                loc(
                    format!(
                        "Valid Romanian IBAN (bank: {}, formatted: {}).",
                        iban.bank_code(),
                        iban.formatted()
                    ),
                    format!(
                        "IBAN românesc valid (bancă: {}, formatat: {}).",
                        iban.bank_code(),
                        iban.formatted()
                    ),
                ),
            ),
            Err(e) => build("iban", false, None, e.message_localized(ro)),
        };
    }

    if is_onrc_candidate(trimmed) {
        return match Onrc::parse(trimmed) {
            Ok(onrc) => build(
                "onrc",
                true,
                Some(onrc.to_string()),
                loc(
                    format!("Valid ONRC Trade Register number: {onrc}."),
                    format!("Număr ONRC valid: {onrc}."),
                ),
            ),
            Err(e) => build("onrc", false, None, e.message_localized(ro)),
        };
    }

    if is_anaf_b2c_placeholder(trimmed) {
        return build(
            "anaf_b2c_placeholder",
            true,
            Some(ANAF_B2C_PLACEHOLDER.to_string()),
            loc(
                "Valid ANAF B2C placeholder (13 zeros per OUG 69/2024 when consumer CNP is not disclosed).".to_string(),
                "Substituent B2C ANAF valid (13 zerouri cf. OUG 69/2024 pentru persoane fizice fără CNP declarat).".to_string(),
            ),
        );
    }

    if trimmed.len() == 13 && trimmed.bytes().all(|b| b.is_ascii_digit()) {
        return match Cnp::parse(trimmed) {
            Ok(cnp) => build(
                "cnp",
                true,
                Some(cnp.to_string()),
                loc(
                    format!(
                        "Valid 13-digit Romanian CNP (month {:02}, day {:02}, county {:02}).",
                        cnp.birth_month(),
                        cnp.birth_day(),
                        cnp.county_code()
                    ),
                    format!(
                        "CNP valid (luna {:02}, ziua {:02}, cod județ {:02}).",
                        cnp.birth_month(),
                        cnp.birth_day(),
                        cnp.county_code()
                    ),
                ),
            ),
            Err(e) => build("cnp", false, None, e.message_localized(ro)),
        };
    }

    if let Some(res) = parse_cui_candidate(trimmed) {
        return match res {
            Ok(cui) => build(
                "cui",
                true,
                Some(cui.to_vat_id()),
                loc(
                    format!(
                        "Valid Romanian CUI/CIF (digits: {}, VAT ID: {}).",
                        cui.digits(),
                        cui.to_vat_id()
                    ),
                    format!(
                        "CUI/CIF valid (cifre: {}, cod TVA: {}).",
                        cui.digits(),
                        cui.to_vat_id()
                    ),
                ),
            ),
            Err(e) => build("cui", false, None, e.message_localized(ro)),
        };
    }

    if let Some(c) = County::lookup_fuzzy(trimmed) {
        return build(
            "county",
            false,
            Some(c.code().to_string()),
            loc(
                format!(
                    "Unencoded county name; CIUS-RO requires ISO 3166-2:RO code '{}' ({}).",
                    c.code(),
                    c.name_ro()
                ),
                format!(
                    "Denumire de județ necodificată ISO; în CIUS-RO utilizați '{}' ({}).",
                    c.code(),
                    c.name_ro()
                ),
            ),
        );
    }

    build(
        "unknown",
        false,
        None,
        loc(
            "Unrecognized format (not a valid Romanian CUI/CIF, CNP, ONRC, RO IBAN, or ISO 3166-2:RO code).".to_string(),
            "Format nerecunoscut ca CUI/CIF, CNP, ONRC, IBAN RO sau subdiviziune ISO 3166-2:RO.".to_string(),
        ),
    )
}

#[cfg(all(doctest, feature = "std"))]
mod readme_doctests {
    #[doc = include_str!("../README.md")]
    struct ReadmeEn;

    #[doc = include_str!("../README.ro.md")]
    struct ReadmeRo;
}
