//! Romanian IBAN (`RO` + 2 check digits + 4-char bank code + 16-char account, 24 chars total).
//!
//! Validates the National Bank of Romania (`BNR`) 24-character IBAN structure and `ISO/IEC 7064 MOD 97-10`
//! check digits used in `BT-84` (`cac:PayeeFinancialAccount/cbc:ID`).

use alloc::string::{String, ToString};
use core::fmt;
use core::str::FromStr;
use iban::{Iban, IbanLike};

use crate::{Locale, LocalizedFmt};

/// Validation error returned when parsing a Romanian [`RoIban`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum IbanError {
    /// Input is empty.
    Empty,
    /// Country code is not `"RO"`.
    NotRomanianCountryCode,
    /// Normalized length (without spaces or hyphens) is not 24 characters.
    InvalidLength {
        /// Observed character count.
        len: usize,
    },
    /// Contains non-ASCII-alphanumeric characters, or check digits at positions `2..4` are not digits.
    InvalidStructure,
    /// `ISO/IEC 7064 MOD 97-10` check-digit verification failed.
    InvalidChecksum {
        /// Expected two-digit check number (`02..=98`).
        expected: u8,
        /// Actual two-digit check number found in the IBAN.
        actual: u8,
    },
}

impl LocalizedFmt for IbanError {
    fn fmt_locale(&self, f: &mut fmt::Formatter<'_>, locale: Locale) -> fmt::Result {
        match (self, locale) {
            (Self::Empty, Locale::En) => f.write_str("IBAN cannot be empty."),
            (Self::Empty, Locale::Ro) => f.write_str("Codul IBAN nu poate fi gol."),
            (Self::NotRomanianCountryCode, Locale::En) => {
                f.write_str("Romanian IBAN must start with country code 'RO'.")
            }
            (Self::NotRomanianCountryCode, Locale::Ro) => {
                f.write_str("Codul IBAN românesc trebuie să înceapă cu 'RO'.")
            }
            (Self::InvalidLength { len }, Locale::En) => {
                write!(f, "Romanian IBAN must be exactly 24 characters long (found {len}).")
            }
            (Self::InvalidLength { len }, Locale::Ro) => write!(
                f,
                "Codul IBAN românesc trebuie să aibă exact 24 de caractere (găsit: {len})."
            ),
            (Self::InvalidStructure, Locale::En) => f.write_str(
                "Romanian IBAN must match RO + 2 digits + 4-char bank code + 16-char alphanumeric account.",
            ),
            (Self::InvalidStructure, Locale::Ro) => f.write_str(
                "Codul IBAN trebuie să respecte structura RO + 2 cifre control + 4 caractere bancă + 16 caractere cont.",
            ),
            (Self::InvalidChecksum { expected, actual }, Locale::En) => write!(
                f,
                "Romanian IBAN fails ISO 7064 MOD 97-10 checksum (expected RO{expected:02}..., found RO{actual:02}...)."
            ),
            (Self::InvalidChecksum { expected, actual }, Locale::Ro) => write!(
                f,
                "Suma de control ISO 7064 MOD 97-10 a IBAN-ului este invalidă (așteptat RO{expected:02}..., găsit RO{actual:02}...)."
            ),
        }
    }
}

impl_localized_error!(IbanError);

/// Validated 24-character Romanian IBAN (`ISO 13616` / `BNR`).
///
/// When `serde` is enabled, [`RoIban`] serializes as its 24-character uppercase electronic string
/// and enforces `"RO"` country-code, length, structure, and `MOD 97-10` invariants on deserialization.
///
/// # Examples
/// ```
/// use cius_ro_id::RoIban;
///
/// let iban: RoIban = "RO49 AAAA 1B31 0075 9384 0000".parse().unwrap();
/// assert_eq!(iban.as_str(), "RO49AAAA1B31007593840000");
/// assert_eq!(iban.check_digits(), 49);
/// assert_eq!(iban.bank_code(), "AAAA");
/// assert_eq!(iban.account_number(), "1B31007593840000");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RoIban(Iban);

#[cfg(feature = "serde")]
impl_str_serde!(
    RoIban,
    "a valid 24-character Romanian IBAN starting with 'RO'"
);

impl RoIban {
    /// Parses and validates a Romanian IBAN (ignoring spaces and hyphens).
    ///
    /// # Errors
    ///
    /// Returns [`IbanError`] if the input is empty, does not begin with `"RO"`, is not 24 characters long,
    /// violates the `BNR` structure, or fails `ISO 7064 MOD 97-10` check-digit verification.
    pub fn parse(raw: &str) -> Result<Self, IbanError> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(IbanError::Empty);
        }
        let mut buf = [0u8; 24];
        let mut len = 0usize;
        let mut all_ascii = true;

        for ch in trimmed.chars().filter(|c| !c.is_whitespace() && *c != '-') {
            if (len == 0 && !matches!(ch, 'R' | 'r')) || (len == 1 && !matches!(ch, 'O' | 'o')) {
                return Err(IbanError::NotRomanianCountryCode);
            }
            if len == buf.len() {
                return Err(IbanError::InvalidLength { len: buf.len() + 1 });
            }
            if ch.is_ascii() {
                buf[len] = ch.to_ascii_uppercase() as u8;
            } else {
                all_ascii = false;
            }
            len += 1;
        }

        if len == 0 {
            return Err(IbanError::Empty);
        }
        if len < 2 {
            return Err(IbanError::NotRomanianCountryCode);
        }
        if len != buf.len() {
            return Err(IbanError::InvalidLength { len });
        }
        if !all_ascii
            || !buf[2..4].iter().all(u8::is_ascii_digit)
            || !buf[4..8].iter().all(u8::is_ascii_alphabetic)
            || !buf[8..24].iter().all(u8::is_ascii_alphanumeric)
        {
            return Err(IbanError::InvalidStructure);
        }

        let compact = core::str::from_utf8(&buf).map_err(|_| IbanError::InvalidStructure)?;
        compact.parse::<Iban>().map(Self).map_err(|_| {
            let actual = (buf[2] - b'0') * 10 + (buf[3] - b'0');
            let expected = compute_ro_iban_check_digits_unchecked(&buf[4..8], &buf[8..24]);
            IbanError::InvalidChecksum { expected, actual }
        })
    }

    /// Returns the normalized 24-character uppercase IBAN without spaces.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.electronic_str()
    }

    /// Returns the 2-digit `ISO 7064` check number (`02..=98`).
    #[must_use]
    pub fn check_digits(&self) -> u8 {
        self.0.check_digits()
    }

    /// Returns the 4-letter bank identifier prefix (positions `4..8`, e.g. `"BTRL"`, `"RNCB"`, `"TREZ"`).
    #[must_use]
    pub fn bank_code(&self) -> &str {
        &self.0.bban()[..4]
    }

    /// Returns the 16-character alphanumeric account identifier (positions `8..24`).
    #[must_use]
    pub fn account_number(&self) -> &str {
        &self.0.bban()[4..]
    }

    /// Formats the IBAN in human-readable 4-character groups (`"RO49 AAAA 1B31 0075 9384 0000"`).
    #[must_use]
    pub fn formatted(&self) -> String {
        self.0.to_string()
    }
}

impl fmt::Display for RoIban {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for RoIban {
    type Err = IbanError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

/// Computes the two-digit `ISO/IEC 7064 MOD 97-10` check digits (`02..=98`) for a Romanian IBAN
/// given a 4-letter `bank_code` and a 16-character alphanumeric `account`.
///
/// # Errors
///
/// Returns [`IbanError::InvalidStructure`] if `bank_code` is not 4 ASCII letters or `account` is not 16 ASCII alphanumeric characters.
pub fn compute_ro_iban_check_digits(bank_code: &str, account: &str) -> Result<u8, IbanError> {
    if bank_code.len() != 4
        || !bank_code.bytes().all(|b| b.is_ascii_alphabetic())
        || account.len() != 16
        || !account.bytes().all(|b| b.is_ascii_alphanumeric())
    {
        return Err(IbanError::InvalidStructure);
    }

    Ok(compute_ro_iban_check_digits_unchecked(
        bank_code.as_bytes(),
        account.as_bytes(),
    ))
}

fn compute_ro_iban_check_digits_unchecked(bank_code: &[u8], account: &[u8]) -> u8 {
    let rem = bank_code
        .iter()
        .chain(account.iter())
        .chain(b"RO00".iter())
        .fold(0u64, |r, &b| {
            if b.is_ascii_digit() {
                (r * 10 + u64::from(b - b'0')) % 97
            } else {
                (r * 100 + u64::from(b.to_ascii_uppercase() - b'A' + 10)) % 97
            }
        });
    u8::try_from(98 - rem).expect("ISO 7064 MOD 97-10 check digits are in 2..=98")
}

pub(crate) fn looks_like_ro_iban_candidate(raw: &str) -> bool {
    let mut iter = raw
        .trim()
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-');
    let (Some(c0), Some(c1)) = (iter.next(), iter.next()) else {
        return false;
    };
    if !c0.eq_ignore_ascii_case(&'R') || !c1.eq_ignore_ascii_case(&'O') {
        return false;
    }
    let mut rest_count = 0usize;
    let mut has_alpha = false;
    for ch in iter.take(23) {
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
