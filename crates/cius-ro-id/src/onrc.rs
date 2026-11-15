//! Romanian Trade Register (`ONRC` / *Registrul Comerțului*) number parser and normalizer.
//!
//! Supports both:
//! 1. **Classic format** (`J40/1234/2022`, `F12/89/2021`, `C08/5/2019`): `[JFC]` + county (`1..=40`, `51`, `52`)
//!    + `/` + sequence number + `/` + 4-digit year (`1990..=2100`).
//! 2. **Law 265/2022 electronic format** (rolled out July 2024, e.g. `J2024001234005`):
//!    `[JFC]` + 4-digit year (`YYYY`) + 6-digit national sequence (`NNNNNN`) + 2-digit county (`00..=40`, `51`, `52`) + 1 control digit (`[JFC]\d{13}`).

use core::fmt;
use core::str::FromStr;

use crate::{Locale, LocalizedFmt};

/// Trade Register section prefix (`J`, `F`, or `C`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum OnrcRegister {
    /// `J`: Legal entities / commercial companies (*persoane juridice*, SRL, SA, etc.).
    J,
    /// `F`: Authorized natural persons, individual & family enterprises (*PFA, ÎI, ÎF*).
    F,
    /// `C`: Cooperative societies (*societăți cooperative*).
    C,
}

impl OnrcRegister {
    /// Returns the uppercase ASCII letter (`'J'`, `'F'`, or `'C'`).
    #[must_use]
    pub const fn as_char(self) -> char {
        match self {
            Self::J => 'J',
            Self::F => 'F',
            Self::C => 'C',
        }
    }

    const fn from_ascii(b: u8) -> Option<Self> {
        match b.to_ascii_uppercase() {
            b'J' => Some(Self::J),
            b'F' => Some(Self::F),
            b'C' => Some(Self::C),
            _ => None,
        }
    }
}

/// Representation variant of a parsed [`Onrc`] Trade Register identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum OnrcFormat {
    /// Classic slash-delimited format (`J40/1234/2022`).
    Classic {
        /// County code (`1..=40`, `51`, or `52`).
        county: u8,
        /// Per-county annual registration sequence (`>= 1`).
        sequence: u32,
        /// Registration year (`1990..=2100`).
        year: u16,
    },
    /// Law 265/2022 nationwide electronic format (`J2024001234005`, `[JFC]\d{13}`).
    Law265 {
        /// Registration year (`2024..=2100` or historical migrated year `1990..=2100`).
        year: u16,
        /// Six-digit national sequence (`1..=999_999`).
        sequence: u32,
        /// Two-digit county marker (`00..=40`, `51`, `52`).
        county: u8,
        /// Trailing control digit (`0..=9`).
        check_digit: u8,
    },
}

/// Validation error returned when parsing a Romanian [`Onrc`] identifier.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum OnrcError {
    /// Input is empty.
    Empty,
    /// Does not begin with `'J'`, `'F'`, or `'C'`.
    InvalidRegisterPrefix,
    /// Structure matches neither `J<county>/<seq>/<year>` nor Law 265/2022 `[JFC]\d{13}`.
    InvalidFormat,
    /// County code is outside the valid `ONRC` county range (`1..=40`, `51`, `52`, or `00` in Law 265/2022).
    InvalidCounty(u8),
    /// Sequence number is zero or unparseable.
    InvalidSequence,
    /// Registration year is outside `1990..=2100`.
    InvalidYear(u16),
}

impl LocalizedFmt for OnrcError {
    fn fmt_locale(&self, f: &mut fmt::Formatter<'_>, locale: Locale) -> fmt::Result {
        match (self, locale) {
            (Self::Empty, Locale::En) => f.write_str("ONRC Trade Register number cannot be empty."),
            (Self::Empty, Locale::Ro) => f.write_str(
                "Numărul de ordine în Registrul Comerțului (ONRC) nu poate fi gol.",
            ),
            (Self::InvalidRegisterPrefix, Locale::En) => {
                f.write_str("ONRC number must start with 'J', 'F', or 'C'.")
            }
            (Self::InvalidRegisterPrefix, Locale::Ro) => {
                f.write_str("Numărul ONRC trebuie să înceapă cu litera 'J', 'F' sau 'C'.")
            }
            (Self::InvalidFormat, Locale::En) => f.write_str(
                "ONRC number must follow classic 'J40/1234/2022' or Law 265/2022 'J2024001234005' format.",
            ),
            (Self::InvalidFormat, Locale::Ro) => f.write_str(
                "Numărul ONRC trebuie să respecte formatul clasic 'J40/1234/2022' sau formatul Legii 265/2022 'J2024001234005'.",
            ),
            (Self::InvalidCounty(c), Locale::En) => {
                write!(f, "Invalid ONRC county code '{c:02}'.")
            }
            (Self::InvalidCounty(c), Locale::Ro) => {
                write!(f, "Indicativul de județ ONRC '{c:02}' este invalid.")
            }
            (Self::InvalidSequence, Locale::En) => {
                f.write_str("ONRC registration sequence must be positive.")
            }
            (Self::InvalidSequence, Locale::Ro) => {
                f.write_str("Numărul de ordine ONRC trebuie să fie un întreg pozitiv.")
            }
            (Self::InvalidYear(y), Locale::En) => {
                write!(f, "Invalid ONRC registration year '{y}' (expected 1990..=2100).")
            }
            (Self::InvalidYear(y), Locale::Ro) => write!(
                f,
                "Anul înmatriculării ONRC '{y}' este invalid (permis: 1990..=2100)."
            ),
        }
    }
}

impl_localized_error!(OnrcError);

/// Validated Romanian Trade Register (`ONRC`) identifier stored inline on the stack (`Copy`).
///
/// When `serde` is enabled, [`Onrc`] serializes as its normalized canonical string (`"J40/1234/2022"`
/// or `"J2024001234005"`) and validates all structural invariants on deserialization.
///
/// # Examples
/// ```
/// use cius_ro_id::{Onrc, OnrcRegister};
///
/// let classic: Onrc = "J40/1234/2022".parse().unwrap();
/// assert_eq!(classic.register(), OnrcRegister::J);
/// assert_eq!(classic.to_string(), "J40/1234/2022");
/// assert_eq!(classic.year(), 2022);
///
/// let modern: Onrc = "J2024001234005".parse().unwrap();
/// assert_eq!(modern.register(), OnrcRegister::J);
/// assert_eq!(modern.year(), 2024);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Onrc {
    register: OnrcRegister,
    format: OnrcFormat,
}

#[cfg(feature = "serde")]
impl_str_serde!(Onrc, "a valid Romanian ONRC Trade Register number");

impl Onrc {
    /// Parses and validates a classic or Law 265/2022 `ONRC` number.
    ///
    /// # Errors
    ///
    /// Returns [`OnrcError`] if the input is empty, lacks a `'J'`/`'F'`/`'C'` prefix, has an invalid
    /// structure, or specifies an out-of-range county, sequence, or registration year.
    pub fn parse(raw: &str) -> Result<Self, OnrcError> {
        let mut buf = [0u8; 32];
        let mut len = 0usize;
        for ch in raw
            .chars()
            .take(crate::MAX_ID_SCAN_CHARS)
            .filter(|c| !c.is_whitespace())
        {
            let Ok(b) = u8::try_from(ch) else {
                return Err(if len == 0 {
                    OnrcError::InvalidRegisterPrefix
                } else {
                    OnrcError::InvalidFormat
                });
            };
            if len == 0 && OnrcRegister::from_ascii(b).is_none() {
                return Err(OnrcError::InvalidRegisterPrefix);
            }
            if len >= buf.len() {
                return Err(OnrcError::InvalidFormat);
            }
            buf[len] = b;
            len += 1;
        }
        if raw.len() > crate::MAX_ID_SCAN_CHARS {
            return Err(OnrcError::InvalidFormat);
        }
        if len == 0 {
            return Err(OnrcError::Empty);
        }

        let register = OnrcRegister::from_ascii(buf[0]).ok_or(OnrcError::InvalidRegisterPrefix)?;
        let rest = core::str::from_utf8(&buf[1..len]).map_err(|_| OnrcError::InvalidFormat)?;

        if rest.contains('/') {
            let mut segments = rest.split('/');
            let (Some(county_str), Some(seq_str), Some(third_str), None) = (
                segments.next(),
                segments.next(),
                segments.next(),
                segments.next(),
            ) else {
                return Err(OnrcError::InvalidFormat);
            };
            let year_str = parse_classic_year_or_date(third_str)?;
            if !(1..=2).contains(&county_str.len())
                || !county_str.bytes().all(|b| b.is_ascii_digit())
                || !(1..=8).contains(&seq_str.len())
                || !seq_str.bytes().all(|b| b.is_ascii_digit())
            {
                return Err(OnrcError::InvalidFormat);
            }

            let county: u8 = county_str.parse().map_err(|_| OnrcError::InvalidFormat)?;
            if !matches!(county, 1..=40 | 51 | 52) {
                return Err(OnrcError::InvalidCounty(county));
            }
            let sequence: u32 = seq_str
                .parse()
                .ok()
                .filter(|&n| n > 0)
                .ok_or(OnrcError::InvalidSequence)?;
            let year: u16 = year_str.parse().map_err(|_| OnrcError::InvalidFormat)?;
            if !(1990..=2100).contains(&year) {
                return Err(OnrcError::InvalidYear(year));
            }

            return Ok(Self {
                register,
                format: OnrcFormat::Classic {
                    county,
                    sequence,
                    year,
                },
            });
        }

        if rest.len() == 13 && rest.bytes().all(|b| b.is_ascii_digit()) {
            let year: u16 = rest[0..4].parse().map_err(|_| OnrcError::InvalidFormat)?;
            if !(1990..=2100).contains(&year) {
                return Err(OnrcError::InvalidYear(year));
            }
            let sequence: u32 = rest[4..10]
                .parse()
                .ok()
                .filter(|&n| n > 0)
                .ok_or(OnrcError::InvalidSequence)?;
            let county: u8 = rest[10..12].parse().map_err(|_| OnrcError::InvalidFormat)?;
            if !matches!(county, 0..=40 | 51 | 52) {
                return Err(OnrcError::InvalidCounty(county));
            }
            return Ok(Self {
                register,
                format: OnrcFormat::Law265 {
                    year,
                    sequence,
                    county,
                    check_digit: rest.as_bytes()[12] - b'0',
                },
            });
        }

        Err(OnrcError::InvalidFormat)
    }

    /// Returns the register section (`J`, `F`, or `C`).
    #[must_use]
    pub const fn register(&self) -> OnrcRegister {
        self.register
    }

    /// Returns the parsed structural representation (`Classic` or `Law265`).
    #[must_use]
    pub const fn format(&self) -> &OnrcFormat {
        &self.format
    }

    /// Returns the 4-digit registration year (`1990..=2100`).
    #[must_use]
    pub const fn year(&self) -> u16 {
        match self.format {
            OnrcFormat::Classic { year, .. } | OnrcFormat::Law265 { year, .. } => year,
        }
    }

    /// Returns the registration sequence number (`>= 1`).
    #[must_use]
    pub const fn sequence(&self) -> u32 {
        match self.format {
            OnrcFormat::Classic { sequence, .. } | OnrcFormat::Law265 { sequence, .. } => sequence,
        }
    }

    /// Returns the numeric `ONRC` county code (`1..=40`, `51`, `52`, or `0` for nationwide Law 265/2022).
    #[must_use]
    pub const fn county_code(&self) -> u8 {
        match self.format {
            OnrcFormat::Classic { county, .. } | OnrcFormat::Law265 { county, .. } => county,
        }
    }
}

impl fmt::Display for Onrc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let p = self.register.as_char();
        match self.format {
            OnrcFormat::Classic {
                county,
                sequence,
                year,
            } => write!(f, "{p}{county:02}/{sequence}/{year}"),
            OnrcFormat::Law265 {
                year,
                sequence,
                county,
                check_digit,
            } => write!(f, "{p}{year:04}{sequence:06}{county:02}{check_digit}"),
        }
    }
}

impl FromStr for Onrc {
    type Err = OnrcError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

/// Returns `true` if `raw` structurally looks like an `ONRC` Trade Register attempt
/// (starts with `J`, `F`, or `C` followed by a digit or `'/'`, ignoring whitespace).
#[must_use]
pub fn is_onrc_candidate(raw: &str) -> bool {
    let mut non_ws = raw
        .chars()
        .take(crate::MAX_ID_SCAN_CHARS)
        .filter(|c| !c.is_whitespace());
    let (Some(c0), Some(c1)) = (non_ws.next(), non_ws.next()) else {
        return false;
    };
    u8::try_from(c0).ok().is_some_and(|b| {
        OnrcRegister::from_ascii(b).is_some() && (c1.is_ascii_digit() || c1 == '/')
    })
}

fn parse_classic_year_or_date(segment: &str) -> Result<&str, OnrcError> {
    if segment.len() == 4 && segment.bytes().all(|b| b.is_ascii_digit()) {
        return Ok(segment);
    }
    if segment.len() == 10 {
        let b = segment.as_bytes();
        let sep_ok = (b[2] == b'.' && b[5] == b'.') || (b[2] == b'-' && b[5] == b'-');
        if sep_ok
            && b[0..2].iter().all(u8::is_ascii_digit)
            && b[3..5].iter().all(u8::is_ascii_digit)
            && b[6..10].iter().all(u8::is_ascii_digit)
        {
            let day = (b[0] - b'0') * 10 + (b[1] - b'0');
            let month = (b[3] - b'0') * 10 + (b[4] - b'0');
            if (1..=31).contains(&day) && (1..=12).contains(&month) {
                return Ok(&segment[6..10]);
            }
        }
    }
    Err(OnrcError::InvalidFormat)
}
