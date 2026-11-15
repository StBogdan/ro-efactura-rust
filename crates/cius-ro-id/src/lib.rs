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
//!   Law 265/2022 `J2024001234005` (`[JFC]\d{13}`) structural formats (Law 265/2022's trailing
//!   control digit has no published checksum formula and is parsed structurally as `0..=9`).
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

/// Fixed-capacity ASCII buffer upholding the UTF-8 invariant for [`Cui`] and [`Cnp`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct AsciiDigits<const N: usize>([u8; N]);

impl<const N: usize> AsciiDigits<N> {
    pub(crate) const fn new(bytes: [u8; N]) -> Self {
        let mut i = 0;
        while i < N {
            assert!(bytes[i].is_ascii());
            i += 1;
        }
        Self(bytes)
    }

    pub(crate) const fn bytes(&self) -> &[u8; N] {
        &self.0
    }

    pub(crate) fn as_str(&self) -> &str {
        core::str::from_utf8(&self.0).expect("AsciiDigits invariant: all bytes are ASCII")
    }

    pub(crate) fn prefix_str(&self, len: usize) -> &str {
        core::str::from_utf8(&self.0[..len]).expect("AsciiDigits invariant: all bytes are ASCII")
    }
}

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

#[doc(hidden)]
pub type __LocalizedString = alloc::string::String;

#[doc(hidden)]
#[must_use]
pub fn __format_localized<E: LocalizedFmt + ?Sized>(err: &E, locale: Locale) -> __LocalizedString {
    alloc::string::ToString::to_string(&LocalizedError::new(err, locale))
}

#[doc(hidden)]
#[must_use]
pub fn __format_args_to_string(args: core::fmt::Arguments<'_>) -> __LocalizedString {
    alloc::fmt::format(args)
}

#[doc(hidden)]
#[macro_export]
macro_rules! impl_localized_display {
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

            /// Returns a human-readable diagnostic message in the requested [`Locale`]($crate::Locale).
            #[must_use]
            pub fn message(&self, locale: $crate::Locale) -> $crate::__LocalizedString {
                $crate::__format_localized(self, locale)
            }
        }

        impl ::core::fmt::Display for $err {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                $crate::LocalizedFmt::fmt_locale(self, f, $crate::Locale::En)
            }
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! impl_localized_error {
    ($err:ty) => {
        $crate::impl_localized_display!($err);

        impl ::core::error::Error for $err {}
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! tr_fmt {
    ($locale:expr, $en:literal, $ro:literal $(, $($arg:tt)*)?) => {
        match ($locale).__as_locale() {
            $crate::Locale::En => {
                $crate::__format_args_to_string(::core::format_args!($en $(, $($arg)*)?))
            }
            $crate::Locale::Ro => {
                $crate::__format_args_to_string(::core::format_args!($ro $(, $($arg)*)?))
            }
        }
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

use alloc::string::{String, ToString};
use core::fmt;
use core::str::FromStr;

/// Output language for human-readable error messages, rule descriptions, and remediation hints.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
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

    /// Selects `en` when `self` is [`Locale::En`] or `ro` when `self` is [`Locale::Ro`].
    #[must_use]
    pub const fn select<'a>(self, en: &'a str, ro: &'a str) -> &'a str {
        match self {
            Self::En => en,
            Self::Ro => ro,
        }
    }

    #[doc(hidden)]
    #[must_use]
    pub const fn __as_locale(&self) -> Self {
        *self
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

/// Formats a diagnostic or error value in a chosen [`Locale`] without intermediate allocation.
pub trait LocalizedFmt {
    /// Writes the localized message for `locale` into `f`.
    ///
    /// # Errors
    ///
    /// Propagates any [`fmt::Error`] returned by `f`.
    fn fmt_locale(&self, f: &mut fmt::Formatter<'_>, locale: Locale) -> fmt::Result;
}

/// Zero-allocation [`fmt::Display`] adapter that formats a [`LocalizedFmt`] error or diagnostic
/// in a chosen [`Locale`].
#[derive(Debug, Clone, Copy)]
pub struct LocalizedError<'a, E: ?Sized> {
    err: &'a E,
    locale: Locale,
}

impl<'a, E: ?Sized> LocalizedError<'a, E> {
    /// Wraps `err` so its [`fmt::Display`] implementation formats in `locale`.
    #[must_use]
    pub const fn new(err: &'a E, locale: Locale) -> Self {
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
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
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
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
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
    #[cfg_attr(feature = "tsify", tsify(optional))]
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

    let details = locale
        .select(
            "Unrecognized format (not a valid Romanian CUI/CIF, CNP, ONRC, RO IBAN, or ISO 3166-2:RO code).",
            "Format nerecunoscut ca CUI/CIF, CNP, ONRC, IBAN RO sau subdiviziune ISO 3166-2:RO.",
        )
        .to_string();
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
            let (code, name) = (c.code(), c.name_ro());
            let details = tr_fmt!(
                locale,
                "Valid ISO 3166-2:RO county code: {code} ({name}).",
                "Cod județ ISO 3166-2:RO valid: {code} ({name})."
            );
            return Some(build_inspection(
                raw,
                IdentifierKind::County,
                true,
                Some(code.to_string()),
                details,
            ));
        }

        if let Some(sec) = BucharestSector::from_code(&upper) {
            let code = sec.code();
            let details = tr_fmt!(
                locale,
                "Valid ANAF Bucharest sector code: {code}.",
                "Cod sector București ANAF valid: {code}."
            );
            return Some(build_inspection(
                raw,
                IdentifierKind::BucharestSector,
                true,
                Some(code.to_string()),
                details,
            ));
        }
    }

    BucharestSector::normalize_fuzzy(trimmed).map(|sec| {
        let code = sec.code();
        let details = tr_fmt!(
            locale,
            "Non-canonical Bucharest sector notation; CIUS-RO requires '{code}'.",
            "Denumire necanonică pentru sector București; în CIUS-RO utilizați '{code}'."
        );
        build_inspection(
            raw,
            IdentifierKind::BucharestSector,
            false,
            Some(code.to_string()),
            details,
        )
    })
}

fn try_inspect_banking_or_registry(
    raw: &str,
    trimmed: &str,
    locale: Locale,
) -> Option<IdentifierInspection> {
    if iban::looks_like_ro_iban_candidate(trimmed) {
        return Some(match RoIban::parse(trimmed) {
            Ok(iban) => {
                let (bank, fmt_iban) = (iban.bank_code(), iban.formatted());
                let details = tr_fmt!(
                    locale,
                    "Valid Romanian IBAN (bank: {bank}, formatted: {fmt_iban}).",
                    "IBAN românesc valid (bancă: {bank}, formatat: {fmt_iban})."
                );
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
            let details = tr_fmt!(
                locale,
                "Valid ONRC Trade Register number: {onrc}.",
                "Număr ONRC valid: {onrc}."
            );
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
        let details = locale
            .select(
                "Valid ANAF B2C placeholder (13 zeros per OUG 69/2024 when consumer CNP is not disclosed).",
                "Substituent B2C ANAF valid (13 zerouri cf. OUG 69/2024 pentru persoane fizice fără CNP declarat).",
            )
            .to_string();
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
                let (mm, dd, jj) = (cnp.birth_month(), cnp.birth_day(), cnp.county_code());
                let details = tr_fmt!(
                    locale,
                    "Valid 13-digit Romanian CNP (month {mm:02}, day {dd:02}, county {jj:02}).",
                    "CNP valid (luna {mm:02}, ziua {dd:02}, cod județ {jj:02})."
                );
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
                let digits = cui.digits();
                let details = tr_fmt!(
                    locale,
                    "Valid Romanian CUI/CIF (digits: {digits}, VAT ID: {vat}).",
                    "CUI/CIF valid (cifre: {digits}, cod TVA: {vat})."
                );
                build_inspection(raw, IdentifierKind::Cui, true, Some(vat), details)
            }
            Err(e) => build_inspection(raw, IdentifierKind::Cui, false, None, e.message(locale)),
        });
    }

    County::lookup_fuzzy(trimmed).map(|c| {
        let (code, name) = (c.code(), c.name_ro());
        let details = tr_fmt!(
            locale,
            "Unencoded county name; CIUS-RO requires ISO 3166-2:RO code '{code}' ({name}).",
            "Denumire de județ necodificată ISO; în CIUS-RO utilizați '{code}' ({name})."
        );
        build_inspection(
            raw,
            IdentifierKind::County,
            false,
            Some(code.to_string()),
            details,
        )
    })
}

#[cfg(all(doctest, feature = "std"))]
mod readme_doctests {
    #[doc = include_str!("../README.md")]
    struct ReadmeEn;

    #[doc = include_str!("../README.ro.md")]
    struct ReadmeRo;
}
