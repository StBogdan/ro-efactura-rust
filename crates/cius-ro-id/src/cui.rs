//! Romanian Fiscal Identifier (`CUI` / `CIF`) parser and modulo-11 checksum verifier.
//!
//! Validates 2-to-10 digit Romanian Unique Registration Codes (*Cod Unic de Înregistrare* /
//! *Cod de Înregistrare Fiscală*), with or without the `RO` VAT registration prefix, using
//! the official Ministry of Finance weight vector `[7, 5, 3, 2, 1, 7, 5, 3, 2]`.

use alloc::format;
use alloc::string::String;
use core::fmt;
use core::str::FromStr;

use crate::{Locale, LocalizedFmt};

/// Official Ministry of Finance control weights for Romanian `CUI`/`CIF` check-digit calculation.
pub const CUI_WEIGHTS: [u32; 9] = [7, 5, 3, 2, 1, 7, 5, 3, 2];

/// Validation error returned when parsing a Romanian [`Cui`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
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
    /// Numeric part begins with `'0'`, which is not valid for Romanian `CUI`/`CIF` numbers.
    LeadingZero,
    /// Modulo-11 control digit mismatch.
    InvalidChecksum {
        /// Expected control digit (`0..=9`).
        expected: u8,
        /// Actual trailing control digit found in the input.
        actual: u8,
    },
}

impl LocalizedFmt for CuiError {
    fn fmt_locale(&self, f: &mut fmt::Formatter<'_>, locale: Locale) -> fmt::Result {
        match (self, locale) {
            (Self::Empty, Locale::En) => f.write_str("CUI/CIF cannot be empty."),
            (Self::Empty, Locale::Ro) => f.write_str("Codul CUI/CIF nu poate fi gol."),
            (Self::InvalidLength { len }, Locale::En) => {
                write!(
                    f,
                    "CUI/CIF must contain between 2 and 10 digits (found {len})."
                )
            }
            (Self::InvalidLength { len }, Locale::Ro) => write!(
                f,
                "Codul CUI/CIF trebuie să conțină între 2 și 10 cifre (găsit: {len})."
            ),
            (Self::InvalidCharacters, Locale::En) => f.write_str(
                "CUI/CIF may only contain an optional 'RO' prefix followed by ASCII digits.",
            ),
            (Self::InvalidCharacters, Locale::Ro) => f.write_str(
                "Codul CUI/CIF poate conține doar prefixul opțional 'RO' urmat exclusiv de cifre.",
            ),
            (Self::LeadingZero, Locale::En) => {
                f.write_str("CUI/CIF digits cannot start with zero.")
            }
            (Self::LeadingZero, Locale::Ro) => {
                f.write_str("Cifrele codului CUI/CIF nu pot începe cu zero.")
            }
            (Self::InvalidChecksum { expected, actual }, Locale::En) => write!(
                f,
                "Invalid CUI/CIF modulo-11 control digit (expected {expected}, found {actual})."
            ),
            (Self::InvalidChecksum { expected, actual }, Locale::Ro) => write!(
                f,
                "Cifra de control modulo-11 a CUI/CIF este invalidă (așteptat {expected}, găsit {actual})."
            ),
        }
    }
}

impl_localized_error!(CuiError);

/// Validated Romanian `CUI` / `CIF` fiscal identifier stored inline on the stack (`Copy`).
///
/// When `serde` is enabled, [`Cui`] serializes as its canonical string (`Display`) and validates
/// all structural and modulo-11 invariants during deserialization.
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Cui {
    digits: crate::AsciiDigits<10>,
    len: u8,
    has_ro_prefix: bool,
}

#[cfg(feature = "serde")]
impl_str_serde!(Cui, "a valid Romanian CUI/CIF fiscal identifier");

impl Cui {
    /// Parses and validates a Romanian `CUI`/`CIF` string (tolerating surrounding or internal whitespace).
    ///
    /// # Errors
    ///
    /// Returns [`CuiError`] if the input is empty, contains invalid characters, falls outside `2..=10` digits,
    /// starts with `'0'`, or fails the official modulo-11 control-digit check.
    pub fn parse(raw: &str) -> Result<Self, CuiError> {
        let mut chars = raw
            .chars()
            .take(crate::MAX_ID_SCAN_CHARS)
            .filter(|c| !c.is_whitespace());
        let Some(c0) = chars.next() else {
            return if raw.len() > crate::MAX_ID_SCAN_CHARS {
                Err(CuiError::InvalidLength { len: 0 })
            } else {
                Err(CuiError::Empty)
            };
        };
        let c1 = chars.next();

        let (has_ro_prefix, first_two): (bool, [Option<char>; 2]) = match (c0, c1) {
            ('R' | 'r', Some('O' | 'o')) => (true, [None, None]),
            (a, b) => (false, [Some(a), b]),
        };

        let mut digits = [0u8; 10];
        let mut count = 0usize;
        for ch in first_two.into_iter().flatten().chain(chars) {
            if !ch.is_ascii_digit() {
                return Err(CuiError::InvalidCharacters);
            }
            if count == digits.len() {
                return Err(CuiError::InvalidLength {
                    len: digits.len() + 1,
                });
            }
            digits[count] = ch as u8;
            count += 1;
        }

        if raw.len() > crate::MAX_ID_SCAN_CHARS {
            let reported = if count >= 2 { digits.len() + 1 } else { count };
            return Err(CuiError::InvalidLength { len: reported });
        }
        Self::from_validated_digits(digits, count, has_ro_prefix)
    }

    fn from_validated_digits(
        digits: [u8; 10],
        count: usize,
        has_ro_prefix: bool,
    ) -> Result<Self, CuiError> {
        if count == 0 {
            return Err(CuiError::Empty);
        }
        let Some(len) = u8::try_from(count).ok().filter(|n| (2..=10).contains(n)) else {
            return Err(CuiError::InvalidLength { len: count });
        };
        if digits[0] == b'0' {
            return Err(CuiError::LeadingZero);
        }

        let actual = digits[count - 1] - b'0';
        let expected = compute_cui_check_digit_unchecked(&digits[..count - 1]);
        if actual != expected {
            return Err(CuiError::InvalidChecksum { expected, actual });
        }

        Ok(Self {
            digits: crate::AsciiDigits::new(digits),
            len,
            has_ro_prefix,
        })
    }

    /// Returns the validated numeric digits without any `RO` prefix (e.g. `"45678918"`).
    #[must_use]
    pub fn digits(&self) -> &str {
        self.digits.prefix_str(usize::from(self.len))
    }

    /// Returns the numeric value of the `CUI`/`CIF` as `u64`.
    #[must_use]
    pub const fn number(&self) -> u64 {
        let bytes = self.digits.bytes();
        let mut idx = 0usize;
        let end = self.len as usize;
        let mut val = 0u64;
        while idx < end {
            val = val * 10 + (bytes[idx] - b'0') as u64;
            idx += 1;
        }
        val
    }

    /// Returns `true` if the parsed input carried the `RO` VAT registration prefix.
    #[must_use]
    pub const fn has_ro_prefix(&self) -> bool {
        self.has_ro_prefix
    }

    /// Returns the trailing modulo-11 check digit (`0..=9`).
    #[must_use]
    pub const fn check_digit(&self) -> u8 {
        self.digits.bytes()[(self.len - 1) as usize] - b'0'
    }

    /// Formats the identifier with the `RO` VAT prefix (e.g. `"RO45678918"`).
    #[must_use]
    pub fn to_vat_id(&self) -> String {
        format!("RO{}", self.digits())
    }
}

impl fmt::Display for Cui {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.has_ro_prefix {
            write!(f, "RO{}", self.digits())
        } else {
            f.write_str(self.digits())
        }
    }
}

impl FromStr for Cui {
    type Err = CuiError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

/// Computes the official modulo-11 control digit (`0..=9`) for a `1..=9` digit `CUI` body (without check digit).
///
/// Formula: `rem = (sum(digit_i * weight_i) * 10) % 11`; if `rem == 10`, control digit is `0`, else `rem`.
///
/// # Errors
///
/// Returns [`CuiError`] if `body_digits` is empty, contains non-digits, starts with `'0'`, or is not `1..=9` digits long.
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

    Ok(compute_cui_check_digit_unchecked(body_digits.as_bytes()))
}

fn compute_cui_check_digit_unchecked(body_digits: &[u8]) -> u8 {
    let offset = CUI_WEIGHTS.len() - body_digits.len();
    let sum: u32 = body_digits
        .iter()
        .zip(&CUI_WEIGHTS[offset..])
        .map(|(&b, &w)| u32::from(b - b'0') * w)
        .sum();
    let rem = (sum * 10) % 11;
    if rem == 10 { 0 } else { rem as u8 }
}

/// Inspects `raw` as a potential `CUI`/`CIF` candidate inside an e-invoice field that might
/// legitimately hold another identifier class (such as a 13-digit `CNP`, `0000000000000` B2C placeholder,
/// `RO` IBAN, or `J40/1234/2022` `ONRC` Trade Register number).
#[must_use]
pub fn parse_cui_candidate(raw: &str) -> Option<Result<Cui, CuiError>> {
    let mut buf = [0u8; 25];
    let mut len = 0usize;
    let mut has_ro = false;
    for ch in raw
        .chars()
        .take(crate::MAX_ID_SCAN_CHARS)
        .filter(|c| !c.is_whitespace())
    {
        if !ch.is_ascii() {
            return None;
        }
        let b = ch as u8;
        if len == 2 {
            has_ro = buf[0].eq_ignore_ascii_case(&b'R') && buf[1].eq_ignore_ascii_case(&b'O');
            if has_ro && b == b'-' {
                return None;
            }
        }
        if len >= 2 && !has_ro && (!b.is_ascii_digit() || len == 10) {
            return None;
        }
        if len == buf.len() {
            break;
        }
        buf[len] = b;
        len += 1;
    }
    let is_ro =
        len >= 2 && buf[0].eq_ignore_ascii_case(&b'R') && buf[1].eq_ignore_ascii_case(&b'O');
    if len < 2 || (raw.len() > crate::MAX_ID_SCAN_CHARS && !is_ro) {
        return None;
    }

    if is_ro {
        if len == 24 && !buf[2..24].iter().all(u8::is_ascii_digit) {
            return None;
        }
        let digit_slice = &buf[2..len];
        if !digit_slice.iter().all(u8::is_ascii_digit) {
            return Some(Err(CuiError::InvalidCharacters));
        }
        if raw.len() > crate::MAX_ID_SCAN_CHARS || digit_slice.len() > 10 {
            return Some(Err(CuiError::InvalidLength {
                len: digit_slice.len(),
            }));
        }
        let mut digits = [0u8; 10];
        digits[..digit_slice.len()].copy_from_slice(digit_slice);
        return Some(Cui::from_validated_digits(digits, digit_slice.len(), true));
    }

    if (2..=10).contains(&len) && buf[..len].iter().all(u8::is_ascii_digit) {
        let mut digits = [0u8; 10];
        digits[..len].copy_from_slice(&buf[..len]);
        return Some(Cui::from_validated_digits(digits, len, false));
    }
    None
}
