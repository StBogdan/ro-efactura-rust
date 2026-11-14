//! Romanian Fiscal Identifier (`CUI` / `CIF`) parser and modulo-11 checksum verifier.
//!
//! Validates 2-to-10 digit Romanian Unique Registration Codes (*Cod Unic de Înregistrare* /
//! *Cod de Înregistrare Fiscală*), with or without the `RO` VAT registration prefix, using
//! the official Ministry of Finance weight vector `[7, 5, 3, 2, 1, 7, 5, 3, 2]`.

use alloc::format;
use alloc::string::{String, ToString};
use core::fmt;
use core::str::FromStr;

/// Official Ministry of Finance control weights for Romanian CUI/CIF check-digit calculation.
pub const CUI_WEIGHTS: [u32; 9] = [7, 5, 3, 2, 1, 7, 5, 3, 2];

/// Validation error returned when parsing a Romanian [`Cui`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[error("{}", self.message_localized(false))]
pub enum CuiError {
    /// Input string is empty or whitespace-only (or `"RO"` with no digits).
    Empty,
    /// Digit body (excluding `RO` prefix) is outside the valid `2..=10` digit length range.
    InvalidLength {
        /// Observed digit count.
        len: usize,
    },
    /// Input contains non-ASCII-digit characters after removing any leading `RO` prefix.
    InvalidCharacters,
    /// Numeric part begins with `'0'`, which is not valid for Romanian CUI/CIF numbers.
    LeadingZero,
    /// Modulo-11 control digit mismatch.
    InvalidChecksum {
        /// Expected control digit (`0..=9`).
        expected: u8,
        /// Actual trailing control digit found in the input.
        actual: u8,
    },
}

impl CuiError {
    /// Returns a human-readable error message in English (`en`) or Romanian (`ro`).
    #[must_use]
    pub fn message_localized(&self, ro: bool) -> String {
        match (self, ro) {
            (Self::Empty, false) => "CUI/CIF cannot be empty.".to_string(),
            (Self::Empty, true) => "Codul CUI/CIF nu poate fi gol.".to_string(),
            (Self::InvalidLength { len }, false) => {
                format!("CUI/CIF must contain between 2 and 10 digits (found {len}).")
            }
            (Self::InvalidLength { len }, true) => {
                format!("Codul CUI/CIF trebuie să conțină între 2 și 10 cifre (găsit: {len}).")
            }
            (Self::InvalidCharacters, false) => {
                "CUI/CIF may only contain an optional 'RO' prefix followed by ASCII digits."
                    .to_string()
            }
            (Self::InvalidCharacters, true) => {
                "Codul CUI/CIF poate conține doar prefixul opțional 'RO' urmat exclusiv de cifre."
                    .to_string()
            }
            (Self::LeadingZero, false) => "CUI/CIF digits cannot start with zero.".to_string(),
            (Self::LeadingZero, true) => "Cifrele codului CUI/CIF nu pot începe cu zero.".to_string(),
            (Self::InvalidChecksum { expected, actual }, false) => format!(
                "Invalid CUI/CIF modulo-11 control digit (expected {expected}, found {actual})."
            ),
            (Self::InvalidChecksum { expected, actual }, true) => format!(
                "Cifra de control modulo-11 a CUI/CIF este invalidă (așteptat {expected}, găsit {actual})."
            ),
        }
    }
}

/// Validated Romanian `CUI` / `CIF` fiscal identifier.
///
/// # Examples
/// ```
/// use cius_ro_id::Cui;
///
/// let cui: Cui = "RO45678918".parse().unwrap();
/// assert!(cui.has_ro_prefix());
/// assert_eq!(cui.digits(), "45678918");
/// assert_eq!(cui.number(), 45_678_918);
/// assert_eq!(cui.check_digit(), 8);
/// assert_eq!(cui.to_vat_id(), "RO45678918");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Cui {
    digits: String,
    has_ro_prefix: bool,
}

impl Cui {
    /// Parses and validates a Romanian CUI/CIF string (tolerating surrounding or internal ASCII whitespace).
    pub fn parse(raw: &str) -> Result<Self, CuiError> {
        let cleaned: String = raw.chars().filter(|c| !c.is_whitespace()).collect();
        if cleaned.is_empty() {
            return Err(CuiError::Empty);
        }

        let (has_ro_prefix, rest) = cleaned
            .get(..2)
            .filter(|p| p.eq_ignore_ascii_case("RO"))
            .map_or((false, cleaned.as_str()), |_| (true, &cleaned[2..]));

        if rest.is_empty() {
            return Err(CuiError::Empty);
        }
        if !rest.bytes().all(|b| b.is_ascii_digit()) {
            return Err(CuiError::InvalidCharacters);
        }
        if !(2..=10).contains(&rest.len()) {
            return Err(CuiError::InvalidLength { len: rest.len() });
        }
        if rest.starts_with('0') {
            return Err(CuiError::LeadingZero);
        }

        let actual = rest.as_bytes()[rest.len() - 1] - b'0';
        let expected = compute_cui_check_digit(&rest[..rest.len() - 1])?;
        if actual != expected {
            return Err(CuiError::InvalidChecksum { expected, actual });
        }

        Ok(Self {
            digits: rest.to_string(),
            has_ro_prefix,
        })
    }

    /// Returns the validated numeric digits without any `RO` prefix (e.g. `"45678918"`).
    #[must_use]
    pub fn digits(&self) -> &str {
        &self.digits
    }

    /// Returns the numeric value of the CUI/CIF as `u64`.
    #[must_use]
    pub fn number(&self) -> u64 {
        self.digits.parse::<u64>().unwrap_or(0)
    }

    /// Returns `true` if the parsed input carried the `RO` VAT registration prefix.
    #[must_use]
    pub const fn has_ro_prefix(&self) -> bool {
        self.has_ro_prefix
    }

    /// Returns the trailing modulo-11 check digit (`0..=9`).
    #[must_use]
    pub fn check_digit(&self) -> u8 {
        self.digits.as_bytes()[self.digits.len() - 1] - b'0'
    }

    /// Formats the identifier with the `RO` VAT prefix (e.g. `"RO45678918"`).
    #[must_use]
    pub fn to_vat_id(&self) -> String {
        format!("RO{}", self.digits)
    }
}

impl fmt::Display for Cui {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.has_ro_prefix {
            write!(f, "RO{}", self.digits)
        } else {
            f.write_str(&self.digits)
        }
    }
}

impl FromStr for Cui {
    type Err = CuiError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

/// Computes the official modulo-11 control digit (`0..=9`) for a `1..=9` digit CUI body (without check digit).
///
/// Formula: `rem = (sum(digit_i * weight_i) * 10) % 11`; if `rem == 10`, control digit is `0`, else `rem`.
pub fn compute_cui_check_digit(body_digits: &str) -> Result<u8, CuiError> {
    if body_digits.is_empty() {
        return Err(CuiError::Empty);
    }
    if !body_digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(CuiError::InvalidCharacters);
    }
    if !(1..=9).contains(&body_digits.len()) {
        return Err(CuiError::InvalidLength {
            len: body_digits.len() + 1,
        });
    }
    if body_digits.starts_with('0') {
        return Err(CuiError::LeadingZero);
    }

    let offset = CUI_WEIGHTS.len() - body_digits.len();
    let sum: u32 = body_digits
        .bytes()
        .zip(&CUI_WEIGHTS[offset..])
        .map(|(b, w)| u32::from(b - b'0') * w)
        .sum();
    let rem = (sum * 10) % 11;
    Ok(if rem == 10 { 0 } else { rem as u8 })
}

/// Inspects `raw` as a potential CUI/CIF candidate inside an e-invoice field that might
/// legitimately hold another identifier class (such as a 13-digit CNP, `0000000000000` B2C placeholder,
/// or `J40/1234/2022` ONRC Trade Register number).
#[must_use]
pub fn parse_cui_candidate(raw: &str) -> Option<Result<Cui, CuiError>> {
    let cleaned: String = raw.chars().filter(|c| !c.is_whitespace()).collect();
    if cleaned.is_empty() {
        return None;
    }
    if let Some(rest) = cleaned
        .get(..2)
        .filter(|p| p.eq_ignore_ascii_case("RO"))
        .map(|_| &cleaned[2..])
    {
        if rest.starts_with('-')
            || (cleaned.len() == 24 && !rest.bytes().all(|b| b.is_ascii_digit()))
        {
            return None;
        }
        return Some(Cui::parse(&cleaned));
    }
    ((2..=10).contains(&cleaned.len()) && cleaned.bytes().all(|b| b.is_ascii_digit()))
        .then(|| Cui::parse(&cleaned))
}
