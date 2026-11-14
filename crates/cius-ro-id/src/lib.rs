//! Parsers, normalizers, and checksum verifiers for Romanian fiscal, legal, banking, and
//! administrative identifiers (`no_std` + `alloc`).
//!
//! Designed for **ANAF `RO e-Factura` (`CIUS-RO 1.0.1`)**, **SAF-T D406**, **e-Transport**, and
//! Romanian ERP/accounting integrations:
//!
//! - [`Cui`]: 2–10 digit *Cod Unic de Înregistrare* / *CIF* (`RO` VAT prefix optional), verified
//!   with the Ministry of Finance modulo-11 weight vector `[7, 5, 3, 2, 1, 7, 5, 3, 2]`.
//! - [`Cnp`] & [`CnpOrPlaceholder`]: 13-digit *Cod Numeric Personal* (sex/century, leap-year
//!   calendar date, county `01..=48 | 51 | 52 | 70`, modulo-11 check digit) plus ANAF's statutory
//!   `0000000000000` B2C placeholder (*OUG 69/2024*).
//! - [`Onrc`]: Trade Register (*Registrul Comerțului*) numbers in classic `J40/1234/2022` and
//!   Law 265/2022 `J2024001234005` (`[JFC]\d{13}`) formats.
//! - [`RoIban`]: 24-character Romanian IBAN (`RO` + 2 check digits + 4-letter bank code + 16-char
//!   account) verified via `ISO/IEC 7064 MOD 97-10`.
//! - [`County`] & [`BucharestSector`]: All 42 `ISO 3166-2:RO` subdivisions (`RO-B`, `RO-AB`..`RO-VN`)
//!   and `SECTOR1`..`SECTOR6` codes with fuzzy normalization helpers.
//!
//! ## Example
//!
//! ```
//! use cius_ro_id::{
//!     BucharestSector, CnpOrPlaceholder, County, Cui, Locale, Onrc, RoIban, inspect_identifier,
//! };
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
//!
//! let info = inspect_identifier("RO45678918", Locale::Ro);
//! assert!(info.is_valid);
//! ```

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

#[cfg(feature = "serde")]
macro_rules! impl_str_serde {
    ($ty:ty, $expecting:literal) => {
        impl serde::Serialize for $ty {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.collect_str(self)
            }
        }

        impl<'de> serde::Deserialize<'de> for $ty {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                struct StrVisitor;

                impl serde::de::Visitor<'_> for StrVisitor {
                    type Value = $ty;

                    fn expecting(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                        f.write_str($expecting)
                    }

                    fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
                        <$ty as core::str::FromStr>::from_str(v).map_err(E::custom)
                    }
                }

                deserializer.deserialize_str(StrVisitor)
            }
        }
    };
}

macro_rules! impl_localized_error {
    ($err:ty) => {
        impl $err {
            /// Returns a zero-allocation [`core::fmt::Display`] adapter formatted in `locale`.
            #[must_use]
            pub const fn display(
                &self,
                locale: $crate::Locale,
            ) -> $crate::LocalizedError<'_, Self> {
                $crate::LocalizedError::new(self, locale)
            }

            /// Returns a human-readable error message in the requested [`Locale`](crate::Locale).
            #[must_use]
            pub fn message(&self, locale: $crate::Locale) -> alloc::string::String {
                alloc::string::ToString::to_string(&self.display(locale))
            }
        }

        impl core::fmt::Display for $err {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                $crate::LocalizedFmt::fmt_locale(self, f, $crate::Locale::En)
            }
        }

        impl core::error::Error for $err {}
    };
}

pub mod cnp;
pub mod cui;
pub mod iban;
pub mod onrc;
pub mod subdivision;

pub use cnp::{
    ANAF_B2C_PLACEHOLDER, CNP_WEIGHTS, Cnp, CnpError, CnpOrPlaceholder, CnpSexCentury,
    compute_cnp_check_digit, is_anaf_b2c_placeholder,
};
pub use cui::{CUI_WEIGHTS, Cui, CuiError, compute_cui_check_digit, parse_cui_candidate};
pub use iban::{IbanError, RoIban, compute_ro_iban_check_digits};
pub use onrc::{Onrc, OnrcError, OnrcFormat, OnrcRegister, is_onrc_candidate};
pub use subdivision::{
    BUCHAREST_SECTOR_CODES, BucharestSector, County, ISO_3166_2_RO_CODES, SubdivisionError,
};

use alloc::format;
use alloc::string::{String, ToString};
use core::fmt;
use core::str::FromStr;

/// Output language for human-readable error messages, rule descriptions, and remediation hints.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum Locale {
    /// English (`en`).
    #[default]
    En,
    /// Romanian (`ro` — *Română*).
    Ro,
}

impl Locale {
    /// Returns `true` when this locale is [`Locale::Ro`].
    #[must_use]
    pub const fn is_ro(self) -> bool {
        matches!(self, Self::Ro)
    }

    /// Returns the two-letter BCP-47 language tag (`"en"` or `"ro"`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::Ro => "ro",
        }
    }
}

impl fmt::Display for Locale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Error returned when parsing an unsupported [`Locale`] tag.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub struct ParseLocaleError {
    input: String,
}

impl ParseLocaleError {
    /// Returns the unrecognized locale string that failed to parse.
    #[must_use]
    pub fn input(&self) -> &str {
        &self.input
    }
}

impl fmt::Display for ParseLocaleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unsupported locale '{}'; expected 'en' or 'ro'",
            self.input
        )
    }
}

impl core::error::Error for ParseLocaleError {}

impl FromStr for Locale {
    type Err = ParseLocaleError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let trimmed = s.trim();
        if trimmed.eq_ignore_ascii_case("en")
            || trimmed
                .get(..3)
                .is_some_and(|p| p.eq_ignore_ascii_case("en-") || p.eq_ignore_ascii_case("en_"))
        {
            Ok(Self::En)
        } else if trimmed.eq_ignore_ascii_case("ro")
            || trimmed
                .get(..3)
                .is_some_and(|p| p.eq_ignore_ascii_case("ro-") || p.eq_ignore_ascii_case("ro_"))
        {
            Ok(Self::Ro)
        } else {
            Err(ParseLocaleError {
                input: s.to_string(),
            })
        }
    }
}

pub(crate) trait LocalizedFmt {
    fn fmt_locale(&self, f: &mut fmt::Formatter<'_>, locale: Locale) -> fmt::Result;
}

/// Zero-allocation [`fmt::Display`] adapter that formats an identifier error in a chosen [`Locale`].
#[derive(Debug, Clone, Copy)]
pub struct LocalizedError<'a, E: ?Sized> {
    err: &'a E,
    locale: Locale,
}

impl<'a, E: ?Sized> LocalizedError<'a, E> {
    pub(crate) const fn new(err: &'a E, locale: Locale) -> Self {
        Self { err, locale }
    }
}

impl<E: LocalizedFmt + ?Sized> fmt::Display for LocalizedError<'_, E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.err.fmt_locale(f, self.locale)
    }
}

/// Classification of a Romanian identifier inspected by [`inspect_identifier`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[non_exhaustive]
pub enum IdentifierKind {
    /// Romanian `CUI` / `CIF` fiscal identifier (`2..=10` digits, optional `RO` prefix).
    Cui,
    /// Romanian 13-digit Personal Numerical Code (`CNP`).
    Cnp,
    /// Statutory 13-zero ANAF B2C placeholder (`0000000000000` per OUG 69/2024).
    AnafB2cPlaceholder,
    /// Trade Register (`ONRC`) number (`J40/1234/2022` or Law 265/2022 `J2024001234005`).
    Onrc,
    /// 24-character Romanian bank account (`RO` IBAN).
    Iban,
    /// `ISO 3166-2:RO` administrative county subdivision (`RO-B`, `RO-AB`..`RO-VN`).
    County,
    /// Bucharest district code (`SECTOR1`..`SECTOR6`).
    BucharestSector,
    /// Input did not match any supported Romanian identifier pattern.
    Unknown,
}

impl IdentifierKind {
    /// Returns the `snake_case` wire identifier (`"cui"`, `"cnp"`, `"iban"`, etc.).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Cui => "cui",
            Self::Cnp => "cnp",
            Self::AnafB2cPlaceholder => "anaf_b2c_placeholder",
            Self::Onrc => "onrc",
            Self::Iban => "iban",
            Self::County => "county",
            Self::BucharestSector => "bucharest_sector",
            Self::Unknown => "unknown",
        }
    }
}

impl fmt::Display for IdentifierKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Diagnostic report returned by [`inspect_identifier`] for CLI and WebAssembly tools.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[non_exhaustive]
pub struct IdentifierInspection {
    /// Raw input string inspected.
    pub input: String,
    /// Detected identifier class.
    pub kind: IdentifierKind,
    /// Whether the identifier passed all structural and checksum validations.
    pub is_valid: bool,
    /// Canonical normalized representation when valid (or suggested normalization when fuzzy-matched).
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub normalized: Option<String>,
    /// Localized diagnostic summary in the requested [`Locale`].
    pub details: String,
}

/// Auto-detects and validates any supported Romanian identifier string (`CUI`, `CNP`, `0000000000000`,
/// `ONRC`, `RO IBAN`, `ISO 3166-2:RO` county, or Bucharest `SECTOR1`..`SECTOR6`).
#[must_use]
pub fn inspect_identifier(raw: &str, locale: Locale) -> IdentifierInspection {
    let trimmed = raw.trim();
    if let Some(res) = try_inspect_exact_or_fuzzy_subdivision(raw, trimmed, locale) {
        return res;
    }
    if let Some(res) = try_inspect_banking_or_registry(raw, trimmed, locale) {
        return res;
    }
    if let Some(res) = try_inspect_fiscal_or_fuzzy_county(raw, trimmed, locale) {
        return res;
    }

    let details = match locale {
        Locale::En => {
            "Unrecognized format (not a valid Romanian CUI/CIF, CNP, ONRC, RO IBAN, or ISO 3166-2:RO code)."
                .to_string()
        }
        Locale::Ro => {
            "Format nerecunoscut ca CUI/CIF, CNP, ONRC, IBAN RO sau subdiviziune ISO 3166-2:RO."
                .to_string()
        }
    };
    IdentifierInspection {
        input: raw.to_string(),
        kind: IdentifierKind::Unknown,
        is_valid: false,
        normalized: None,
        details,
    }
}

fn build_inspection(
    raw: &str,
    kind: IdentifierKind,
    is_valid: bool,
    normalized: Option<String>,
    details: String,
) -> IdentifierInspection {
    IdentifierInspection {
        input: raw.to_string(),
        kind,
        is_valid,
        normalized,
        details,
    }
}

fn try_inspect_exact_or_fuzzy_subdivision(
    raw: &str,
    trimmed: &str,
    locale: Locale,
) -> Option<IdentifierInspection> {
    if trimmed.len() <= 16 {
        let upper = trimmed.to_ascii_uppercase();
        if let Some(c) = County::from_code(&upper) {
            let details = match locale {
                Locale::En => format!(
                    "Valid ISO 3166-2:RO county code: {} ({}).",
                    c.code(),
                    c.name_ro()
                ),
                Locale::Ro => format!(
                    "Cod județ ISO 3166-2:RO valid: {} ({}).",
                    c.code(),
                    c.name_ro()
                ),
            };
            return Some(build_inspection(
                raw,
                IdentifierKind::County,
                true,
                Some(c.code().to_string()),
                details,
            ));
        }

        if let Some(sec) = BucharestSector::from_code(&upper) {
            let details = match locale {
                Locale::En => format!("Valid ANAF Bucharest sector code: {}.", sec.code()),
                Locale::Ro => format!("Cod sector București ANAF valid: {}.", sec.code()),
            };
            return Some(build_inspection(
                raw,
                IdentifierKind::BucharestSector,
                true,
                Some(sec.code().to_string()),
                details,
            ));
        }
    }

    BucharestSector::normalize_fuzzy(trimmed).map(|sec| {
        let details = match locale {
            Locale::En => format!(
                "Non-canonical Bucharest sector notation; CIUS-RO requires '{}'.",
                sec.code()
            ),
            Locale::Ro => format!(
                "Denumire necanonică pentru sector București; în CIUS-RO utilizați '{}'.",
                sec.code()
            ),
        };
        build_inspection(
            raw,
            IdentifierKind::BucharestSector,
            false,
            Some(sec.code().to_string()),
            details,
        )
    })
}

fn try_inspect_banking_or_registry(
    raw: &str,
    trimmed: &str,
    locale: Locale,
) -> Option<IdentifierInspection> {
    if looks_like_ro_iban_candidate(trimmed) {
        return Some(match RoIban::parse(trimmed) {
            Ok(iban) => {
                let details = match locale {
                    Locale::En => format!(
                        "Valid Romanian IBAN (bank: {}, formatted: {}).",
                        iban.bank_code(),
                        iban.formatted()
                    ),
                    Locale::Ro => format!(
                        "IBAN românesc valid (bancă: {}, formatat: {}).",
                        iban.bank_code(),
                        iban.formatted()
                    ),
                };
                build_inspection(
                    raw,
                    IdentifierKind::Iban,
                    true,
                    Some(iban.as_str().to_string()),
                    details,
                )
            }
            Err(e) => build_inspection(raw, IdentifierKind::Iban, false, None, e.message(locale)),
        });
    }

    is_onrc_candidate(trimmed).then(|| match Onrc::parse(trimmed) {
        Ok(onrc) => {
            let details = match locale {
                Locale::En => format!("Valid ONRC Trade Register number: {onrc}."),
                Locale::Ro => format!("Număr ONRC valid: {onrc}."),
            };
            build_inspection(
                raw,
                IdentifierKind::Onrc,
                true,
                Some(onrc.to_string()),
                details,
            )
        }
        Err(e) => build_inspection(raw, IdentifierKind::Onrc, false, None, e.message(locale)),
    })
}

fn try_inspect_fiscal_or_fuzzy_county(
    raw: &str,
    trimmed: &str,
    locale: Locale,
) -> Option<IdentifierInspection> {
    if is_anaf_b2c_placeholder(trimmed) {
        let details = match locale {
            Locale::En => {
                "Valid ANAF B2C placeholder (13 zeros per OUG 69/2024 when consumer CNP is not disclosed)."
                    .to_string()
            }
            Locale::Ro => {
                "Substituent B2C ANAF valid (13 zerouri cf. OUG 69/2024 pentru persoane fizice fără CNP declarat)."
                    .to_string()
            }
        };
        return Some(build_inspection(
            raw,
            IdentifierKind::AnafB2cPlaceholder,
            true,
            Some(ANAF_B2C_PLACEHOLDER.to_string()),
            details,
        ));
    }

    if trimmed.len() == 13 && trimmed.bytes().all(|b| b.is_ascii_digit()) {
        return Some(match Cnp::parse(trimmed) {
            Ok(cnp) => {
                let details = match locale {
                    Locale::En => format!(
                        "Valid 13-digit Romanian CNP (month {:02}, day {:02}, county {:02}).",
                        cnp.birth_month(),
                        cnp.birth_day(),
                        cnp.county_code()
                    ),
                    Locale::Ro => format!(
                        "CNP valid (luna {:02}, ziua {:02}, cod județ {:02}).",
                        cnp.birth_month(),
                        cnp.birth_day(),
                        cnp.county_code()
                    ),
                };
                build_inspection(
                    raw,
                    IdentifierKind::Cnp,
                    true,
                    Some(cnp.to_string()),
                    details,
                )
            }
            Err(e) => build_inspection(raw, IdentifierKind::Cnp, false, None, e.message(locale)),
        });
    }

    if let Some(res) = parse_cui_candidate(trimmed) {
        return Some(match res {
            Ok(cui) => {
                let vat = cui.to_vat_id();
                let details = match locale {
                    Locale::En => format!(
                        "Valid Romanian CUI/CIF (digits: {}, VAT ID: {vat}).",
                        cui.digits()
                    ),
                    Locale::Ro => {
                        format!("CUI/CIF valid (cifre: {}, cod TVA: {vat}).", cui.digits())
                    }
                };
                build_inspection(raw, IdentifierKind::Cui, true, Some(vat), details)
            }
            Err(e) => build_inspection(raw, IdentifierKind::Cui, false, None, e.message(locale)),
        });
    }

    County::lookup_fuzzy(trimmed).map(|c| {
        let details = match locale {
            Locale::En => format!(
                "Unencoded county name; CIUS-RO requires ISO 3166-2:RO code '{}' ({}).",
                c.code(),
                c.name_ro()
            ),
            Locale::Ro => format!(
                "Denumire de județ necodificată ISO; în CIUS-RO utilizați '{}' ({}).",
                c.code(),
                c.name_ro()
            ),
        };
        build_inspection(
            raw,
            IdentifierKind::County,
            false,
            Some(c.code().to_string()),
            details,
        )
    })
}

fn looks_like_ro_iban_candidate(trimmed: &str) -> bool {
    let mut iter = trimmed.chars().filter(|c| !c.is_whitespace() && *c != '-');
    let (Some(c0), Some(c1)) = (iter.next(), iter.next()) else {
        return false;
    };
    if !c0.eq_ignore_ascii_case(&'R') || !c1.eq_ignore_ascii_case(&'O') {
        return false;
    }
    let mut rest_count = 0usize;
    let mut has_alpha = false;
    for ch in iter {
        rest_count += 1;
        if ch.is_ascii_alphabetic() {
            has_alpha = true;
        }
        if rest_count >= 12 && has_alpha {
            return true;
        }
    }
    false
}

#[cfg(all(doctest, feature = "std"))]
mod readme_doctests {
    #[doc = include_str!("../README.md")]
    struct ReadmeEn;

    #[doc = include_str!("../README.ro.md")]
    struct ReadmeRo;
}
