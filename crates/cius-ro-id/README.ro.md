# cius-ro-id

[**English (`README.md`)**](./README.md) · [**Documentația principală**](https://github.com/StBogdan/ro-efactura-rust/blob/main/README.ro.md)

Bibliotecă în Rust pentru parsarea, normalizarea și validarea codurilor fiscale, bancare și administrative din România.

`cius-ro-id` este compatibilă `no_std` (cu `alloc`) și nu conține cod `unsafe` (`#![forbid(unsafe_code)]`). Este gândită pentru integrări **ANAF `RO e-Factura`** (`CIUS-RO 1.0.1`), **SAF-T D406**, aplicații de facturare, ERP și fintech.

## Ce validează

- **`Cui`** — *Cod Unic de Înregistrare* (`CUI` / `CIF`): tip `Copy` pe stivă (`2`–`10` cifre, cu sau fără prefixul de TVA `RO`), verificat cu cheia oficială modulo-11 a Ministerului Finanțelor (`753217532`).
- **`Cnp` și `CnpOrPlaceholder`** — *Cod Numeric Personal*: tipuri `Copy` fără alocare pe heap pentru CNP-uri de 13 cifre (sex/secol `1`–`9`, data nașterii, județ și cifra de control modulo-11 `279146358279`) și codul ANAF `0000000000000` pentru consumatori persoane fizice B2C (*OUG 69/2024*).
- **`Onrc`** — *Număr de ordine în Registrul Comerțului*: tip `Copy` pe stivă ce recunoaște formatul clasic `J40/1234/2022` (inclusiv certificatele 2003–2008 cu dată `ZZ.LL.AAAA`) și formatul nou de 14 caractere introdus prin *Legea 265/2022* (`J2024001234005`).
- **`RoIban`** — Cont bancar IBAN românesc de 24 de caractere (`ROkk BBBB ...`), verificat conform standardului `ISO/IEC 7064 MOD 97-10`.
- **`County`** — Cele 42 de coduri de județ `ISO 3166-2:RO` (`RO-AB`..`RO-VN` și `RO-B`), cu recunoaștere tolerantă la diacritice și abrevieri (`"Jud. Timiș"` -> `County::TM`).
- **`BucharestSector`** — Codurile ANAF `SECTOR1`..`SECTOR6` pentru Municipiul București, cu normalizare automată (`"Sectorul 1"` -> `BucharestSector::Sector1`).
- **`inspect_identifier`** — Detectează automat tipul identificatorului și returnează diagnostice structurate în [`Locale::Ro`] sau [`Locale::En`].

## Utilizare

```toml
[dependencies]
cius-ro-id = "0.1"
# Opțional: features = ["serde"] sau default-features = false pentru no_std + alloc
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

    assert_eq!(County::lookup_fuzzy("Cluj"), Some(County::CJ));
    assert_eq!(
        BucharestSector::normalize_fuzzy("Sectorul 1"),
        Some(BucharestSector::Sector1)
    );
    Ok(())
}
```

## Licență

Distribuit sub licență dublă **MIT** sau **Apache-2.0**, la alegere.
