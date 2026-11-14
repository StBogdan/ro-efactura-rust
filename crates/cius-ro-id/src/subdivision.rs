//! Romanian Administrative Subdivisions (`ISO 3166-2:RO` counties and Bucharest `SECTOR1`..`SECTOR6`).
//!
//! Enforces ANAF `CIUS-RO 1.0.1` rules `BR-RO-100`, `BR-RO-110`, `BR-RO-160`, `BR-RO-170`,
//! `BR-RO-200`, and `BR-RO-210`, while offering fuzzy lookup helpers (`"Cluj"` -> `RO-CJ`,
//! `"Sectorul 1"` -> `SECTOR1`) for actionable remediation hints.

use alloc::string::{String, ToString};
use core::fmt;
use core::str::FromStr;

use crate::{Locale, LocalizedFmt};

/// Error returned when parsing a strict [`County`] (`ISO 3166-2:RO`) or [`BucharestSector`] (`SECTOR-RO`) code.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum SubdivisionError {
    /// Value is not one of the 42 official `ISO 3166-2:RO` county codes (`RO-B`, `RO-AB`..`RO-VN`).
    InvalidCountyCode(String),
    /// Value is not one of the 6 official Bucharest `SECTOR-RO` district codes (`SECTOR1`..`SECTOR6`).
    InvalidBucharestSector(String),
}

impl LocalizedFmt for SubdivisionError {
    fn fmt_locale(&self, f: &mut fmt::Formatter<'_>, locale: Locale) -> fmt::Result {
        match (self, locale) {
            (Self::InvalidCountyCode(val), Locale::En) => {
                write!(f, "Invalid ISO 3166-2:RO county subdivision code '{val}'.")
            }
            (Self::InvalidCountyCode(val), Locale::Ro) => {
                write!(f, "Codul subdiviziunii ISO 3166-2:RO '{val}' este invalid.")
            }
            (Self::InvalidBucharestSector(val), Locale::En) => write!(
                f,
                "Invalid Bucharest sector code '{val}' (expected SECTOR1..SECTOR6)."
            ),
            (Self::InvalidBucharestSector(val), Locale::Ro) => write!(
                f,
                "Codul sectorului București '{val}' este invalid (permis: SECTOR1..SECTOR6)."
            ),
        }
    }
}

impl_localized_error!(SubdivisionError);

/// All 42 official `ISO 3166-2:RO` country subdivision codes accepted by ANAF (`$ISO-3166-RO-CODES`).
pub const ISO_3166_2_RO_CODES: [&str; 42] = [
    "RO-B", "RO-AB", "RO-AR", "RO-AG", "RO-BC", "RO-BH", "RO-BN", "RO-BT", "RO-BV", "RO-BR",
    "RO-BZ", "RO-CS", "RO-CL", "RO-CJ", "RO-CT", "RO-CV", "RO-DB", "RO-DJ", "RO-GL", "RO-GR",
    "RO-GJ", "RO-HR", "RO-HD", "RO-IL", "RO-IS", "RO-IF", "RO-MM", "RO-MH", "RO-MS", "RO-NT",
    "RO-OT", "RO-PH", "RO-SM", "RO-SJ", "RO-SB", "RO-SV", "RO-TR", "RO-TM", "RO-TL", "RO-VS",
    "RO-VL", "RO-VN",
];

/// All 6 official Bucharest district codes (`$SECTOR-RO-CODES`) required in `CityName` when `CountrySubentity = RO-B`.
pub const BUCHAREST_SECTOR_CODES: [&str; 6] = [
    "SECTOR1", "SECTOR2", "SECTOR3", "SECTOR4", "SECTOR5", "SECTOR6",
];

/// Official `ISO 3166-2:RO` administrative subdivision (Bucharest municipality + 41 counties).
///
/// When `serde` is enabled, [`County`] serializes as its normative `ISO 3166-2:RO` code
/// (`"RO-B"`, `"RO-CJ"`, etc.) and validates exact `ISO 3166-2:RO` strings on deserialization.
#[repr(usize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[expect(
    missing_docs,
    reason = "42 self-explanatory ISO 3166-2:RO county abbreviation variants"
)]
pub enum County {
    B,
    AB,
    AR,
    AG,
    BC,
    BH,
    BN,
    BT,
    BV,
    BR,
    BZ,
    CS,
    CL,
    CJ,
    CT,
    CV,
    DB,
    DJ,
    GL,
    GR,
    GJ,
    HR,
    HD,
    IL,
    IS,
    IF,
    MM,
    MH,
    MS,
    NT,
    OT,
    PH,
    SM,
    SJ,
    SB,
    SV,
    TR,
    TM,
    TL,
    VS,
    VL,
    VN,
}

#[cfg(feature = "serde")]
impl_str_serde!(
    County,
    "a valid ISO 3166-2:RO county code (e.g. 'RO-B' or 'RO-CJ')"
);

const COUNTY_TABLE: [(County, &str, &str, &str, &str); 42] = [
    (County::B, "RO-B", "B", "București", "Bucuresti"),
    (County::AB, "RO-AB", "AB", "Alba", "Alba"),
    (County::AR, "RO-AR", "AR", "Arad", "Arad"),
    (County::AG, "RO-AG", "AG", "Argeș", "Arges"),
    (County::BC, "RO-BC", "BC", "Bacău", "Bacau"),
    (County::BH, "RO-BH", "BH", "Bihor", "Bihor"),
    (
        County::BN,
        "RO-BN",
        "BN",
        "Bistrița-Năsăud",
        "Bistrita-Nasaud",
    ),
    (County::BT, "RO-BT", "BT", "Botoșani", "Botosani"),
    (County::BV, "RO-BV", "BV", "Brașov", "Brasov"),
    (County::BR, "RO-BR", "BR", "Brăila", "Braila"),
    (County::BZ, "RO-BZ", "BZ", "Buzău", "Buzau"),
    (County::CS, "RO-CS", "CS", "Caraș-Severin", "Caras-Severin"),
    (County::CL, "RO-CL", "CL", "Călărași", "Calarasi"),
    (County::CJ, "RO-CJ", "CJ", "Cluj", "Cluj"),
    (County::CT, "RO-CT", "CT", "Constanța", "Constanta"),
    (County::CV, "RO-CV", "CV", "Covasna", "Covasna"),
    (County::DB, "RO-DB", "DB", "Dâmbovița", "Dambovita"),
    (County::DJ, "RO-DJ", "DJ", "Dolj", "Dolj"),
    (County::GL, "RO-GL", "GL", "Galați", "Galati"),
    (County::GR, "RO-GR", "GR", "Giurgiu", "Giurgiu"),
    (County::GJ, "RO-GJ", "GJ", "Gorj", "Gorj"),
    (County::HR, "RO-HR", "HR", "Harghita", "Harghita"),
    (County::HD, "RO-HD", "HD", "Hunedoara", "Hunedoara"),
    (County::IL, "RO-IL", "IL", "Ialomița", "Ialomita"),
    (County::IS, "RO-IS", "IS", "Iași", "Iasi"),
    (County::IF, "RO-IF", "IF", "Ilfov", "Ilfov"),
    (County::MM, "RO-MM", "MM", "Maramureș", "Maramures"),
    (County::MH, "RO-MH", "MH", "Mehedinți", "Mehedinti"),
    (County::MS, "RO-MS", "MS", "Mureș", "Mures"),
    (County::NT, "RO-NT", "NT", "Neamț", "Neamt"),
    (County::OT, "RO-OT", "OT", "Olt", "Olt"),
    (County::PH, "RO-PH", "PH", "Prahova", "Prahova"),
    (County::SM, "RO-SM", "SM", "Satu Mare", "Satu Mare"),
    (County::SJ, "RO-SJ", "SJ", "Sălaj", "Salaj"),
    (County::SB, "RO-SB", "SB", "Sibiu", "Sibiu"),
    (County::SV, "RO-SV", "SV", "Suceava", "Suceava"),
    (County::TR, "RO-TR", "TR", "Teleorman", "Teleorman"),
    (County::TM, "RO-TM", "TM", "Timiș", "Timis"),
    (County::TL, "RO-TL", "TL", "Tulcea", "Tulcea"),
    (County::VS, "RO-VS", "VS", "Vaslui", "Vaslui"),
    (County::VL, "RO-VL", "VL", "Vâlcea", "Valcea"),
    (County::VN, "RO-VN", "VN", "Vrancea", "Vrancea"),
];

impl County {
    /// Returns all 42 [`County`] variants in `ISO 3166-2:RO` order.
    pub fn all() -> impl Iterator<Item = Self> {
        COUNTY_TABLE.iter().map(|(c, _, _, _, _)| *c)
    }

    /// Strict ANAF `ISO 3166-2:RO` lookup (exact uppercase match after trimming, e.g. `"RO-CJ"`, `"RO-B"`).
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        let trimmed = code.trim();
        COUNTY_TABLE
            .iter()
            .find(|(_, iso, _, _, _)| *iso == trimmed)
            .map(|(c, _, _, _, _)| *c)
    }

    /// Fuzzy county resolution from free-text inputs (`"CLUJ"`, `"Jud. Cluj"`, `"CJ"`, `"ro-cj"`, `"București"`),
    /// useful for generating remediation hints when `BR-RO-110` fails.
    #[must_use]
    pub fn lookup_fuzzy(input: &str) -> Option<Self> {
        let folded = fold_ascii_upper(input);
        let stripped = ["JUDETUL", "JUD.", "JUD", "MUNICIPIUL", "MUN."]
            .iter()
            .find_map(|p| folded.strip_prefix(p))
            .unwrap_or(&folded)
            .trim_matches(|c: char| !c.is_ascii_alphanumeric())
            .trim();

        if stripped == "BUCURESTI" || stripped == "BUCHAREST" {
            return Some(Self::B);
        }

        let compact: String = stripped
            .chars()
            .filter(char::is_ascii_alphanumeric)
            .collect();
        COUNTY_TABLE
            .iter()
            .find_map(|&(county, iso, short, _, ascii_name)| {
                let norm_name = ascii_name
                    .chars()
                    .filter(char::is_ascii_alphanumeric)
                    .map(|c| c.to_ascii_uppercase());
                (stripped == iso || stripped == short || compact.chars().eq(norm_name))
                    .then_some(county)
            })
    }

    /// Returns the normative `ISO 3166-2:RO` code (e.g. `"RO-CJ"`, `"RO-B"`).
    #[must_use]
    pub const fn code(self) -> &'static str {
        COUNTY_TABLE[self as usize].1
    }

    /// Returns the 1-or-2 letter county abbreviation without `RO-` (e.g. `"CJ"`, `"B"`).
    #[must_use]
    pub const fn short_code(self) -> &'static str {
        COUNTY_TABLE[self as usize].2
    }

    /// Returns the Romanian county name with proper diacritics (e.g. `"Brașov"`, `"București"`).
    #[must_use]
    pub const fn name_ro(self) -> &'static str {
        COUNTY_TABLE[self as usize].3
    }

    /// Returns the ASCII-transliterated county name (e.g. `"Brasov"`, `"Bucuresti"`).
    #[must_use]
    pub const fn name_ascii(self) -> &'static str {
        COUNTY_TABLE[self as usize].4
    }
}

impl fmt::Display for County {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

impl FromStr for County {
    type Err = SubdivisionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_code(s).ok_or_else(|| SubdivisionError::InvalidCountyCode(s.trim().to_string()))
    }
}

/// Official ANAF `SECTOR-RO` Bucharest district code (`SECTOR1`..`SECTOR6`) for `BT-37` / `BT-52` / `BT-66` / `BT-77`.
///
/// When `serde` is enabled, [`BucharestSector`] serializes as `"SECTOR1"`..`"SECTOR6"` and validates
/// exact `SECTOR1`..`SECTOR6` codes on deserialization.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[expect(
    missing_docs,
    reason = "6 self-explanatory Bucharest sector enum variants"
)]
pub enum BucharestSector {
    Sector1 = 1,
    Sector2 = 2,
    Sector3 = 3,
    Sector4 = 4,
    Sector5 = 5,
    Sector6 = 6,
}

#[cfg(feature = "serde")]
impl_str_serde!(
    BucharestSector,
    "a valid Bucharest sector code ('SECTOR1'..'SECTOR6')"
);

impl BucharestSector {
    /// Strict ANAF lookup (`"SECTOR1"`..`"SECTOR6"`).
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        let rest = code.trim().strip_prefix("SECTOR")?;
        (rest.len() == 1).then(|| Self::from_number(rest.as_bytes()[0].wrapping_sub(b'0')))?
    }

    /// Fuzzy normalization from common human-written variants (`"Sector 1"`, `"Sectorul 1"`,
    /// `"București, Sector 3"`, `"Sect. 2"`, `"S6"`) into the canonical ANAF `SECTOR1`..`SECTOR6` token.
    #[must_use]
    pub fn normalize_fuzzy(input: &str) -> Option<Self> {
        let compact: String = fold_ascii_upper(input)
            .chars()
            .filter(char::is_ascii_alphanumeric)
            .collect();
        let s = compact.strip_prefix("BUCURESTI").unwrap_or(&compact);
        let tail = ["SECTORUL", "SECTOR", "SECT", "S"]
            .iter()
            .find_map(|p| s.strip_prefix(p))?;
        (tail.len() == 1).then(|| Self::from_number(tail.as_bytes()[0].wrapping_sub(b'0')))?
    }

    /// Constructs a [`BucharestSector`] from sector number `1..=6`.
    #[must_use]
    pub const fn from_number(n: u8) -> Option<Self> {
        match n {
            1 => Some(Self::Sector1),
            2 => Some(Self::Sector2),
            3 => Some(Self::Sector3),
            4 => Some(Self::Sector4),
            5 => Some(Self::Sector5),
            6 => Some(Self::Sector6),
            _ => None,
        }
    }

    /// Returns the sector number (`1..=6`).
    #[must_use]
    pub const fn number(self) -> u8 {
        self as u8
    }

    /// Returns the normative ANAF `SECTOR-RO` string (`"SECTOR1"`..`"SECTOR6"`).
    #[must_use]
    pub const fn code(self) -> &'static str {
        BUCHAREST_SECTOR_CODES[(self as usize) - 1]
    }
}

impl fmt::Display for BucharestSector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

impl FromStr for BucharestSector {
    type Err = SubdivisionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_code(s)
            .ok_or_else(|| SubdivisionError::InvalidBucharestSector(s.trim().to_string()))
    }
}

fn fold_ascii_upper(s: &str) -> String {
    let trimmed = s.trim();
    if trimmed.len() > 256 {
        return String::new();
    }
    deunicode::deunicode(trimmed).to_ascii_uppercase()
}
