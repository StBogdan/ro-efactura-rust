//! Romanian Personal Numerical Code (`CNP`) and ANAF `RO e-Factura` B2C placeholder (`0000000000000`).
//!
//! Implements full structural, calendar (including leap years), county/sector (`01..=48`, `51`, `52`, `70`),
//! sequence (`001..=999`), and modulo-11 (`[2, 7, 9, 1, 4, 6, 3, 5, 8, 2, 7, 9]`) verification for
//! 13-digit Romanian CNPs (*Cod Numeric Personal*), alongside explicit recognition of the statutory
//! 13-zero B2C placeholder (`"0000000000000"`) mandated by OUG 69/2024 when a natural-person buyer
//! does not provide their CNP.

use alloc::format;
use alloc::string::{String, ToString};
use core::fmt;
use core::str::FromStr;

/// Official 13-zero B2C buyer identifier (`BT-47` / `BT-48`) mandated by ANAF RO e-Factura (OUG 69/2024)
/// when a natural-person consumer does not disclose their Personal Numerical Code (`CNP`).
pub const ANAF_B2C_PLACEHOLDER: &str = "0000000000000";

/// Official modulo-11 control constant (`279146358279`) for Romanian 13-digit CNP numbers.
pub const CNP_WEIGHTS: [u32; 12] = [2, 7, 9, 1, 4, 6, 3, 5, 8, 2, 7, 9];

/// Returns `true` if `raw` (ignoring surrounding whitespace) equals [`ANAF_B2C_PLACEHOLDER`] (`"0000000000000"`).
#[must_use]
pub fn is_anaf_b2c_placeholder(raw: &str) -> bool {
    raw.trim() == ANAF_B2C_PLACEHOLDER
}

/// Sex and birth-century classification encoded by the first digit (`S`) of a Romanian [`Cnp`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CnpSexCentury {
    /// `1`: Male born 1900–1999.
    Male1900,
    /// `2`: Female born 1900–1999.
    Female1900,
    /// `3`: Male born 1800–1899.
    Male1800,
    /// `4`: Female born 1800–1899.
    Female1800,
    /// `5`: Male born 2000–2099.
    Male2000,
    /// `6`: Female born 2000–2099.
    Female2000,
    /// `7`: Foreign resident male in Romania.
    ResidentMale,
    /// `8`: Foreign resident female in Romania.
    ResidentFemale,
    /// `9`: Foreign non-resident person assigned a Romanian CNP.
    ForeignPerson,
}

impl CnpSexCentury {
    const fn from_digit(d: u8) -> Option<Self> {
        match d {
            1 => Some(Self::Male1900),
            2 => Some(Self::Female1900),
            3 => Some(Self::Male1800),
            4 => Some(Self::Female1800),
            5 => Some(Self::Male2000),
            6 => Some(Self::Female2000),
            7 => Some(Self::ResidentMale),
            8 => Some(Self::ResidentFemale),
            9 => Some(Self::ForeignPerson),
            _ => None,
        }
    }
}

/// Validation error returned when parsing a 13-digit [`Cnp`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[error("{}", self.message_localized(false))]
pub enum CnpError {
    /// Input is the ANAF 13-zero B2C placeholder (`0000000000000`), not a real person's CNP.
    /// Use [`CnpOrPlaceholder::parse`] when validating `RO e-Factura` B2C buyer identifiers.
    AnafB2cPlaceholder,
    /// Input does not contain exactly 13 characters after trimming whitespace.
    InvalidLength {
        /// Observed character length.
        len: usize,
    },
    /// Input contains non-ASCII-digit characters.
    InvalidCharacters,
    /// First digit (`S`) is `0` (outside `1..=9`).
    InvalidSexDigit(u8),
    /// Encoded birth date (`AA`, `LL`, `ZZ`) is not a valid calendar date.
    InvalidDate {
        /// Two-digit year (`00..=99`).
        yy: u8,
        /// Month (`00..=99`).
        month: u8,
        /// Day (`00..=99`).
        day: u8,
    },
    /// County/sector code (`JJ`) is not a valid Romanian CNP county code (`01..=48`, `51`, `52`, `70`).
    InvalidCountyCode(u8),
    /// Serial number (`NNN`) is `000` (must be `001..=999`).
    InvalidSequence,
    /// Modulo-11 control digit (`C`) mismatch.
    InvalidChecksum {
        /// Expected control digit (`0..=9`).
        expected: u8,
        /// Actual trailing control digit found in the input.
        actual: u8,
    },
}

impl CnpError {
    /// Returns a human-readable error message in English (`false`) or Romanian (`true`).
    #[must_use]
    pub fn message_localized(&self, ro: bool) -> String {
        match (self, ro) {
            (Self::AnafB2cPlaceholder, false) => {
                "Input '0000000000000' is the ANAF B2C placeholder, not an individual's CNP."
                    .to_string()
            }
            (Self::AnafB2cPlaceholder, true) => {
                "Valoarea '0000000000000' este substituentul B2C ANAF, nu un CNP individual."
                    .to_string()
            }
            (Self::InvalidLength { len }, false) => {
                format!("CNP must contain exactly 13 digits (found {len}).")
            }
            (Self::InvalidLength { len }, true) => {
                format!("Codul CNP trebuie să conțină exact 13 cifre (găsit: {len}).")
            }
            (Self::InvalidCharacters, false) => "CNP must contain only ASCII digits.".to_string(),
            (Self::InvalidCharacters, true) => {
                "Codul CNP trebuie să conțină exclusiv cifre.".to_string()
            }
            (Self::InvalidSexDigit(s), false) => {
                format!("Invalid CNP sex/century digit '{s}' (must be 1..=9).")
            }
            (Self::InvalidSexDigit(s), true) => {
                format!("Cifra de sex/secol '{s}' din CNP este invalidă (permis: 1..=9).")
            }
            (Self::InvalidDate { yy, month, day }, false) => format!(
                "Invalid calendar birth date in CNP (YY={yy:02}, MM={month:02}, DD={day:02})."
            ),
            (Self::InvalidDate { yy, month, day }, true) => format!(
                "Data nașterii din CNP este invalidă calendaristic (AA={yy:02}, LL={month:02}, ZZ={day:02})."
            ),
            (Self::InvalidCountyCode(jj), false) => {
                format!("Invalid CNP county/sector code '{jj:02}'.")
            }
            (Self::InvalidCountyCode(jj), true) => {
                format!("Codul de județ/sector '{jj:02}' din CNP este invalid.")
            }
            (Self::InvalidSequence, false) => {
                "CNP serial sequence (digits 10..12) cannot be '000'.".to_string()
            }
            (Self::InvalidSequence, true) => {
                "Numărul secvențial NNN din CNP (cifrele 10..12) nu poate fi '000'.".to_string()
            }
            (Self::InvalidChecksum { expected, actual }, false) => format!(
                "Invalid CNP modulo-11 control digit (expected {expected}, found {actual})."
            ),
            (Self::InvalidChecksum { expected, actual }, true) => format!(
                "Cifra de control a CNP este invalidă (așteptat {expected}, găsit {actual})."
            ),
        }
    }
}

/// Validated 13-digit Romanian Personal Numerical Code (`CNP`).
///
/// # Examples
/// ```
/// use cius_ro_id::{Cnp, CnpOrPlaceholder};
///
/// let cnp: Cnp = "1900101410011".parse().unwrap();
/// assert_eq!(cnp.birth_year(), Some(1990));
/// assert_eq!(cnp.birth_month(), 1);
/// assert_eq!(cnp.birth_day(), 1);
/// assert_eq!(cnp.county_code(), 41);
///
/// let b2c = CnpOrPlaceholder::parse("0000000000000").unwrap();
/// assert!(b2c.is_placeholder());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Cnp {
    digits: String,
}

impl Cnp {
    /// Parses and validates a 13-digit Romanian CNP.
    ///
    /// Rejects `"0000000000000"` with [`CnpError::AnafB2cPlaceholder`]; use [`CnpOrPlaceholder::parse`]
    /// when accepting ANAF `RO e-Factura` B2C buyer identifiers.
    pub fn parse(raw: &str) -> Result<Self, CnpError> {
        let trimmed = raw.trim();
        if trimmed == ANAF_B2C_PLACEHOLDER {
            return Err(CnpError::AnafB2cPlaceholder);
        }
        if !trimmed.bytes().all(|b| b.is_ascii_digit()) {
            return Err(CnpError::InvalidCharacters);
        }
        if trimmed.len() != 13 {
            return Err(CnpError::InvalidLength { len: trimmed.len() });
        }

        let b = trimmed.as_bytes();
        let s = b[0] - b'0';
        let Some(sex_century) = CnpSexCentury::from_digit(s) else {
            return Err(CnpError::InvalidSexDigit(s));
        };

        let yy = (b[1] - b'0') * 10 + (b[2] - b'0');
        let month = (b[3] - b'0') * 10 + (b[4] - b'0');
        let day = (b[5] - b'0') * 10 + (b[6] - b'0');

        if !is_valid_cnp_date(sex_century, yy, month, day) {
            return Err(CnpError::InvalidDate { yy, month, day });
        }

        let jj = (b[7] - b'0') * 10 + (b[8] - b'0');
        if !matches!(jj, 1..=48 | 51 | 52 | 70) {
            return Err(CnpError::InvalidCountyCode(jj));
        }

        if &trimmed[9..12] == "000" {
            return Err(CnpError::InvalidSequence);
        }

        let expected = compute_cnp_check_digit(&trimmed[..12])?;
        let actual = b[12] - b'0';
        if actual != expected {
            return Err(CnpError::InvalidChecksum { expected, actual });
        }

        Ok(Self {
            digits: trimmed.to_string(),
        })
    }

    /// Returns the 13-digit string representation.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.digits
    }

    /// Returns the sex and birth-century classification (`S`, digit 1).
    #[must_use]
    pub fn sex_century(&self) -> CnpSexCentury {
        CnpSexCentury::from_digit(self.digits.as_bytes()[0] - b'0')
            .expect("validated Cnp always has S in 1..=9")
    }

    /// Returns the exact 4-digit birth year for `S` in `1..=6`, or `None` when `S` is `7..=9`
    /// (foreign residents/non-residents whose century is not uniquely fixed by `S`).
    #[must_use]
    pub fn birth_year(&self) -> Option<u16> {
        let yy =
            u16::from((self.digits.as_bytes()[1] - b'0') * 10 + (self.digits.as_bytes()[2] - b'0'));
        match self.sex_century() {
            CnpSexCentury::Male1800 | CnpSexCentury::Female1800 => Some(1800 + yy),
            CnpSexCentury::Male1900 | CnpSexCentury::Female1900 => Some(1900 + yy),
            CnpSexCentury::Male2000 | CnpSexCentury::Female2000 => Some(2000 + yy),
            _ => None,
        }
    }

    /// Returns the birth month (`1..=12`).
    #[must_use]
    pub fn birth_month(&self) -> u8 {
        (self.digits.as_bytes()[3] - b'0') * 10 + (self.digits.as_bytes()[4] - b'0')
    }

    /// Returns the birth day of month (`1..=31`).
    #[must_use]
    pub fn birth_day(&self) -> u8 {
        (self.digits.as_bytes()[5] - b'0') * 10 + (self.digits.as_bytes()[6] - b'0')
    }

    /// Returns the two-digit county/sector code (`01..=48`, `51`, `52`, or `70`).
    #[must_use]
    pub fn county_code(&self) -> u8 {
        (self.digits.as_bytes()[7] - b'0') * 10 + (self.digits.as_bytes()[8] - b'0')
    }

    /// Returns the trailing modulo-11 control digit (`0..=9`).
    #[must_use]
    pub fn check_digit(&self) -> u8 {
        self.digits.as_bytes()[12] - b'0'
    }
}

impl fmt::Display for Cnp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.digits)
    }
}

impl FromStr for Cnp {
    type Err = CnpError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

/// Either a validated 13-digit [`Cnp`] or the ANAF B2C placeholder (`0000000000000`) under OUG 69/2024.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CnpOrPlaceholder {
    /// A real, checksum-verified 13-digit Personal Numerical Code.
    Cnp(Cnp),
    /// The statutory 13-zero ANAF `RO e-Factura` B2C placeholder (`"0000000000000"`).
    AnafB2cPlaceholder,
}

impl CnpOrPlaceholder {
    /// Parses either a valid 13-digit [`Cnp`] or `"0000000000000"`.
    pub fn parse(raw: &str) -> Result<Self, CnpError> {
        if is_anaf_b2c_placeholder(raw) {
            Ok(Self::AnafB2cPlaceholder)
        } else {
            Cnp::parse(raw).map(Self::Cnp)
        }
    }

    /// Returns `true` if this value is [`Self::AnafB2cPlaceholder`].
    #[must_use]
    pub const fn is_placeholder(&self) -> bool {
        matches!(self, Self::AnafB2cPlaceholder)
    }
}

/// Computes the official modulo-11 control digit for the first 12 digits of a Romanian CNP.
///
/// Formula: `rem = sum(digit_i * weight_i) % 11`; if `rem == 10`, control digit is `1`, else `rem`.
pub fn compute_cnp_check_digit(first_twelve: &str) -> Result<u8, CnpError> {
    if !first_twelve.bytes().all(|b| b.is_ascii_digit()) {
        return Err(CnpError::InvalidCharacters);
    }
    if first_twelve.len() != 12 {
        return Err(CnpError::InvalidLength {
            len: first_twelve.len() + 1,
        });
    }
    let sum: u32 = first_twelve
        .bytes()
        .zip(CNP_WEIGHTS)
        .map(|(b, w)| u32::from(b - b'0') * w)
        .sum();
    let rem = sum % 11;
    Ok(if rem == 10 { 1 } else { rem as u8 })
}

fn is_valid_cnp_date(sex_century: CnpSexCentury, yy: u8, month: u8, day: u8) -> bool {
    let base: i32 = match sex_century {
        CnpSexCentury::Male1800 | CnpSexCentury::Female1800 => 1800,
        CnpSexCentury::Male1900 | CnpSexCentury::Female1900 => 1900,
        // For S = 5..=6 (20xx) and S = 7..=9 (century unspecified; 2000 permits Feb 29 for YY=00)
        _ => 2000,
    };
    time::Month::try_from(month)
        .ok()
        .and_then(|m| time::Date::from_calendar_date(base + i32::from(yy), m, day).ok())
        .is_some()
}
