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

#[derive(Clone, Copy)]
struct CountyMeta {
    county: County,
    iso: &'static str,
    short: &'static str,
    name_ro: &'static str,
    name_ascii: &'static str,
}

macro_rules! county_meta {
    ($variant:ident, $iso:literal, $short:literal, $ro:literal, $ascii:literal) => {
        CountyMeta {
            county: County::$variant,
            iso: $iso,
            short: $short,
            name_ro: $ro,
            name_ascii: $ascii,
        }
    };
}

const COUNTY_TABLE: [CountyMeta; 42] = [
    county_meta!(B, "RO-B", "B", "București", "Bucuresti"),
    county_meta!(AB, "RO-AB", "AB", "Alba", "Alba"),
    county_meta!(AR, "RO-AR", "AR", "Arad", "Arad"),
    county_meta!(AG, "RO-AG", "AG", "Argeș", "Arges"),
    county_meta!(BC, "RO-BC", "BC", "Bacău", "Bacau"),
    county_meta!(BH, "RO-BH", "BH", "Bihor", "Bihor"),
    county_meta!(BN, "RO-BN", "BN", "Bistrița-Năsăud", "Bistrita-Nasaud"),
    county_meta!(BT, "RO-BT", "BT", "Botoșani", "Botosani"),
    county_meta!(BV, "RO-BV", "BV", "Brașov", "Brasov"),
    county_meta!(BR, "RO-BR", "BR", "Brăila", "Braila"),
    county_meta!(BZ, "RO-BZ", "BZ", "Buzău", "Buzau"),
    county_meta!(CS, "RO-CS", "CS", "Caraș-Severin", "Caras-Severin"),
    county_meta!(CL, "RO-CL", "CL", "Călărași", "Calarasi"),
    county_meta!(CJ, "RO-CJ", "CJ", "Cluj", "Cluj"),
    county_meta!(CT, "RO-CT", "CT", "Constanța", "Constanta"),
    county_meta!(CV, "RO-CV", "CV", "Covasna", "Covasna"),
    county_meta!(DB, "RO-DB", "DB", "Dâmbovița", "Dambovita"),
    county_meta!(DJ, "RO-DJ", "DJ", "Dolj", "Dolj"),
    county_meta!(GL, "RO-GL", "GL", "Galați", "Galati"),
    county_meta!(GR, "RO-GR", "GR", "Giurgiu", "Giurgiu"),
    county_meta!(GJ, "RO-GJ", "GJ", "Gorj", "Gorj"),
    county_meta!(HR, "RO-HR", "HR", "Harghita", "Harghita"),
    county_meta!(HD, "RO-HD", "HD", "Hunedoara", "Hunedoara"),
    county_meta!(IL, "RO-IL", "IL", "Ialomița", "Ialomita"),
    county_meta!(IS, "RO-IS", "IS", "Iași", "Iasi"),
    county_meta!(IF, "RO-IF", "IF", "Ilfov", "Ilfov"),
    county_meta!(MM, "RO-MM", "MM", "Maramureș", "Maramures"),
    county_meta!(MH, "RO-MH", "MH", "Mehedinți", "Mehedinti"),
    county_meta!(MS, "RO-MS", "MS", "Mureș", "Mures"),
    county_meta!(NT, "RO-NT", "NT", "Neamț", "Neamt"),
    county_meta!(OT, "RO-OT", "OT", "Olt", "Olt"),
    county_meta!(PH, "RO-PH", "PH", "Prahova", "Prahova"),
    county_meta!(SM, "RO-SM", "SM", "Satu Mare", "Satu Mare"),
    county_meta!(SJ, "RO-SJ", "SJ", "Sălaj", "Salaj"),
    county_meta!(SB, "RO-SB", "SB", "Sibiu", "Sibiu"),
    county_meta!(SV, "RO-SV", "SV", "Suceava", "Suceava"),
    county_meta!(TR, "RO-TR", "TR", "Teleorman", "Teleorman"),
    county_meta!(TM, "RO-TM", "TM", "Timiș", "Timis"),
    county_meta!(TL, "RO-TL", "TL", "Tulcea", "Tulcea"),
    county_meta!(VS, "RO-VS", "VS", "Vaslui", "Vaslui"),
    county_meta!(VL, "RO-VL", "VL", "Vâlcea", "Valcea"),
    county_meta!(VN, "RO-VN", "VN", "Vrancea", "Vrancea"),
];

const fn const_str_eq(a: &str, b: &str) -> bool {
    let (ab, bb) = (a.as_bytes(), b.as_bytes());
    if ab.len() != bb.len() {
        return false;
    }
    let mut i = 0;
    while i < ab.len() {
        if ab[i] != bb[i] {
            return false;
        }
        i += 1;
    }
    true
}

const _: () = {
    let mut i = 0;
    while i < COUNTY_TABLE.len() {
        assert!(COUNTY_TABLE[i].county as usize == i);
        assert!(const_str_eq(ISO_3166_2_RO_CODES[i], COUNTY_TABLE[i].iso));
        i += 1;
    }
    assert!(BUCHAREST_SECTOR_CODES.len() == 6);
    assert!(BucharestSector::Sector1 as usize == 1);
    assert!(BucharestSector::Sector2 as usize == 2);
    assert!(BucharestSector::Sector3 as usize == 3);
    assert!(BucharestSector::Sector4 as usize == 4);
    assert!(BucharestSector::Sector5 as usize == 5);
    assert!(BucharestSector::Sector6 as usize == 6);
};

impl County {
    /// Returns all 42 [`County`] variants in `ISO 3166-2:RO` order.
    pub fn all() -> impl Iterator<Item = Self> {
        COUNTY_TABLE.iter().map(|meta| meta.county)
    }

    /// Strict ANAF `ISO 3166-2:RO` lookup (exact uppercase match after trimming, e.g. `"RO-CJ"`, `"RO-B"`).
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        let trimmed = code.trim();
        COUNTY_TABLE
            .iter()
            .find(|meta| meta.iso == trimmed)
            .map(|meta| meta.county)
    }

    /// Fuzzy county resolution from free-text inputs (`"CLUJ"`, `"Jud. Cluj"`, `"CJ"`, `"ro-cj"`, `"București"`),
    /// useful for generating remediation hints when `BR-RO-110` fails.
    #[must_use]
    pub fn lookup_fuzzy(input: &str) -> Option<Self> {
        let folded = fold_ascii_upper(input)?;
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
        COUNTY_TABLE.iter().find_map(|meta| {
            let norm_name = meta
                .name_ascii
                .chars()
                .filter(char::is_ascii_alphanumeric)
                .map(|c| c.to_ascii_uppercase());
            (stripped == meta.iso || stripped == meta.short || compact.chars().eq(norm_name))
                .then_some(meta.county)
        })
    }

    /// Returns the normative `ISO 3166-2:RO` code (e.g. `"RO-CJ"`, `"RO-B"`).
    #[must_use]
    pub const fn code(self) -> &'static str {
        COUNTY_TABLE[self as usize].iso
    }

    /// Returns the 1-or-2 letter county abbreviation without `RO-` (e.g. `"CJ"`, `"B"`).
    #[must_use]
    pub const fn short_code(self) -> &'static str {
        COUNTY_TABLE[self as usize].short
    }

    /// Returns the Romanian county name with proper diacritics (e.g. `"Brașov"`, `"București"`).
    #[must_use]
    pub const fn name_ro(self) -> &'static str {
        COUNTY_TABLE[self as usize].name_ro
    }

    /// Returns the ASCII-transliterated county name (e.g. `"Brasov"`, `"Bucuresti"`).
    #[must_use]
    pub const fn name_ascii(self) -> &'static str {
        COUNTY_TABLE[self as usize].name_ascii
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
        Self::from_code(s).ok_or_else(|| SubdivisionError::InvalidCountyCode(bounded_err_token(s)))
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
        match code.trim() {
            "SECTOR1" => Some(Self::Sector1),
            "SECTOR2" => Some(Self::Sector2),
            "SECTOR3" => Some(Self::Sector3),
            "SECTOR4" => Some(Self::Sector4),
            "SECTOR5" => Some(Self::Sector5),
            "SECTOR6" => Some(Self::Sector6),
            _ => None,
        }
    }

    /// Fuzzy normalization from common human-written variants (`"Sector 1"`, `"Sectorul 1"`,
    /// `"București, Sector 3"`, `"Sect. 2"`, `"S6"`) into the canonical ANAF `SECTOR1`..`SECTOR6` token.
    #[must_use]
    pub fn normalize_fuzzy(input: &str) -> Option<Self> {
        let folded = fold_ascii_upper(input)?;
        let compact: String = folded.chars().filter(char::is_ascii_alphanumeric).collect();
        let s = compact.strip_prefix("BUCURESTI").unwrap_or(&compact);
        let tail = ["SECTORUL", "SECTOR", "SECT", "S"]
            .iter()
            .find_map(|p| s.strip_prefix(p))?;
        match tail {
            "1" => Some(Self::Sector1),
            "2" => Some(Self::Sector2),
            "3" => Some(Self::Sector3),
            "4" => Some(Self::Sector4),
            "5" => Some(Self::Sector5),
            "6" => Some(Self::Sector6),
            _ => None,
        }
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
            .ok_or_else(|| SubdivisionError::InvalidBucharestSector(bounded_err_token(s)))
    }
}

const MAX_FUZZY_SUBDIVISION_BYTES: usize = 128;

fn bounded_err_token(s: &str) -> String {
    let trimmed = s.trim();
    let end = trimmed
        .char_indices()
        .nth(64)
        .map_or(trimmed.len(), |(idx, _)| idx);
    trimmed[..end].to_string()
}

fn fold_ascii_upper(s: &str) -> Option<String> {
    let trimmed = s.trim();
    if trimmed.is_empty() || trimmed.len() > MAX_FUZZY_SUBDIVISION_BYTES {
        return None;
    }
    Some(deunicode::deunicode(trimmed).to_ascii_uppercase())
}
