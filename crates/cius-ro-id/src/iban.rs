//! Romanian IBAN (`RO` + 2 check digits + 4-char bank code + 16-char account, 24 chars total).
//!
//! Validates the National Bank of Romania (`BNR`) 24-character IBAN structure and ISO/IEC 7064 MOD 97-10
//! check digits used in `BT-84` (`cac:PayeeFinancialAccount/cbc:ID`).

use alloc::format;
use alloc::string::{String, ToString};
use core::fmt;
use core::str::FromStr;
use iban::{Iban, IbanLike};

/// Validation error returned when parsing a Romanian [`RoIban`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[error("{}", self.message_localized(false))]
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
    /// ISO/IEC 7064 MOD 97-10 check-digit verification failed.
    InvalidChecksum {
        /// Expected two-digit check number (`02..=98`).
        expected: u8,
        /// Actual two-digit check number found in the IBAN.
        actual: u8,
    },
}

impl IbanError {
    /// Returns a human-readable error message in English (`false`) or Romanian (`true`).
    #[must_use]
    pub fn message_localized(&self, ro: bool) -> String {
        match (self, ro) {
            (Self::Empty, false) => "IBAN cannot be empty.".to_string(),
            (Self::Empty, true) => "Codul IBAN nu poate fi gol.".to_string(),
            (Self::NotRomanianCountryCode, false) => {
                "Romanian IBAN must start with country code 'RO'.".to_string()
            }
            (Self::NotRomanianCountryCode, true) => {
                "Codul IBAN românesc trebuie să înceapă cu 'RO'.".to_string()
            }
            (Self::InvalidLength { len }, false) => {
                format!("Romanian IBAN must be exactly 24 characters long (found {len}).")
            }
            (Self::InvalidLength { len }, true) => {
                format!("Codul IBAN românesc trebuie să aibă exact 24 de caractere (găsit: {len}).")
            }
            (Self::InvalidStructure, false) => {
                "Romanian IBAN must match RO + 2 digits + 4-char bank code + 16-char alphanumeric account.".to_string()
            }
            (Self::InvalidStructure, true) => {
                "Codul IBAN trebuie să respecte structura RO + 2 cifre control + 4 caractere bancă + 16 caractere cont.".to_string()
            }
            (Self::InvalidChecksum { expected, actual }, false) => format!(
                "Romanian IBAN fails ISO 7064 MOD 97-10 checksum (expected RO{expected:02}..., found RO{actual:02}...)."
            ),
            (Self::InvalidChecksum { expected, actual }, true) => format!(
                "Suma de control ISO 7064 MOD 97-10 a IBAN-ului este invalidă (așteptat RO{expected:02}..., găsit RO{actual:02}...)."
            ),
        }
    }
}

/// Validated 24-character Romanian IBAN (`ISO 13616` / `BNR`).
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct RoIban(Iban);

impl RoIban {
    /// Parses and validates a Romanian IBAN (ignoring spaces and hyphens).
    pub fn parse(raw: &str) -> Result<Self, IbanError> {
        let compact: String = raw
            .chars()
            .filter(|c| !c.is_whitespace() && *c != '-')
            .map(|c| c.to_ascii_uppercase())
            .collect();

        if compact.is_empty() {
            return Err(IbanError::Empty);
        }
        if !compact.starts_with("RO") {
            return Err(IbanError::NotRomanianCountryCode);
        }
        if compact.len() != 24 {
            return Err(IbanError::InvalidLength { len: compact.len() });
        }

        let bytes = compact.as_bytes();
        if !bytes[2..4].iter().all(u8::is_ascii_digit)
            || !bytes[4..8].iter().all(u8::is_ascii_alphabetic)
            || !bytes[8..24].iter().all(u8::is_ascii_alphanumeric)
        {
            return Err(IbanError::InvalidStructure);
        }

        compact.parse::<Iban>().map(Self).map_err(|_| {
            let actual = (bytes[2] - b'0') * 10 + (bytes[3] - b'0');
            let expected =
                compute_ro_iban_check_digits(&compact[4..8], &compact[8..24]).unwrap_or(0);
            IbanError::InvalidChecksum { expected, actual }
        })
    }

    /// Returns the normalized 24-character uppercase IBAN without spaces.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.electronic_str()
    }

    /// Returns the 2-digit ISO 7064 check number (`02..=98`).
    #[must_use]
    pub fn check_digits(&self) -> u8 {
        self.0.check_digits()
    }

    /// Returns the 4-letter bank identifier prefix (positions `4..8`, e.g. `"BTRL"`, `"RNCB"`, `"TREZ"`).
    #[must_use]
    pub fn bank_code(&self) -> &str {
        self.0.bank_identifier().unwrap_or_default()
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

/// Computes the two-digit ISO/IEC 7064 MOD 97-10 check digits (`02..=98`) for a Romanian IBAN
/// given a 4-letter `bank_code` and a 16-character alphanumeric `account`.
pub fn compute_ro_iban_check_digits(bank_code: &str, account: &str) -> Result<u8, IbanError> {
    if bank_code.len() != 4
        || !bank_code.bytes().all(|b| b.is_ascii_alphabetic())
        || account.len() != 16
        || !account.bytes().all(|b| b.is_ascii_alphanumeric())
    {
        return Err(IbanError::InvalidStructure);
    }

    let rem = bank_code
        .bytes()
        .chain(account.bytes())
        .chain(*b"RO00")
        .fold(0u64, |r, b| {
            if b.is_ascii_digit() {
                (r * 10 + u64::from(b - b'0')) % 97
            } else {
                (r * 100 + u64::from(b.to_ascii_uppercase() - b'A' + 10)) % 97
            }
        });
    Ok((98 - rem) as u8)
}
