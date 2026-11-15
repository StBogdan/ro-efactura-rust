# cius-ro-id

[**Română (`README.ro.md`)**](https://github.com/StBogdan/ro-efactura-rust/blob/main/crates/cius-ro-id/README.ro.md) · [**GitHub Repository**](https://github.com/StBogdan/ro-efactura-rust)

Parse, normalize, and verify Romanian fiscal, banking, and administrative identifiers in Rust.

`cius-ro-id` supports `no_std` (with `alloc`) and forbids `unsafe` code (`#![forbid(unsafe_code)]`). It is designed for **ANAF `RO e-Factura`** (`CIUS-RO 1.0.1`), **SAF-T D406**, and Romanian ERP, accounting, and fintech applications.

## Supported Identifiers

- **`Cui`** — Fiscal Identification Code (*Cod Unic de Înregistrare* / *CIF*): `Copy` stack value (`2`–`10` digits with optional `RO` VAT prefix), verified with the Ministry of Finance modulo-11 control key (`753217532`).
- **`Cnp` & `CnpOrPlaceholder`** — Personal Numeric Code (*Cod Numeric Personal*): `Copy` stack types for 13-digit CNPs (sex/century `1`–`9`, calendar birth date, county code, and modulo-11 check digit `279146358279`) and ANAF's statutory `0000000000000` B2C placeholder (*OUG 69/2024*).
- **`Onrc`** — Trade Register Number (*Registrul Comerțului*): `Copy` stack representation supporting classic `J40/1234/2022` numbers (including 2003–2008 *Law 359/2004* `DD.MM.YYYY` certificates) and the 14-character *Law 265/2022* format (`J2024001234005`).
- **`RoIban`** — 24-character Romanian bank account (`ROkk BBBB ...`), verified with `ISO/IEC 7064 MOD 97-10`.
- **`County`** — All 42 `ISO 3166-2:RO` subdivision codes (`RO-AB`..`RO-VN` and `RO-B`), with diacritic-insensitive fuzzy lookup (`"Timiș"` -> `County::TM`).
- **`BucharestSector`** — ANAF `SECTOR-RO` codes (`SECTOR1`..`SECTOR6`), with fuzzy normalization (`"Sectorul 1"` -> `BucharestSector::Sector1`).
- **`inspect_identifier`** — Auto-detects any of the identifiers above and returns structured diagnostics in [`Locale::En`] or [`Locale::Ro`].

## Usage

```toml
[dependencies]
cius-ro-id = "0.1"
# Optional: features = ["serde"], or default-features = false for no_std + alloc
```

```rust
use cius_ro_id::{BucharestSector, Cnp, County, Cui, Onrc, RoIban};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cui: Cui = "RO 45678918".parse()?;
    assert_eq!(cui.digits(), "45678918");
    assert_eq!(cui.to_vat_id(), "RO45678918");

    let cnp: Cnp = "1900101410011".parse()?;
    let onrc: Onrc = "J40/1234/2022".parse()?;
    let iban: RoIban = "RO49 AAAA 1B31 0075 9384 0000".parse()?;
    let _ = (cnp, onrc, iban);

    assert_eq!(County::lookup_fuzzy("Timiș"), Some(County::TM));
    assert_eq!(
        BucharestSector::normalize_fuzzy("Sector 3"),
        Some(BucharestSector::Sector3)
    );
    Ok(())
}
```

## License

Dual-licensed under **MIT** or **Apache-2.0**, at your option.
