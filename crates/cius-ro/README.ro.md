# cius-ro

[**English (`README.md`)**](./README.md) · [**Catalog Reguli (RO)**](https://github.com/StBogdan/ro-efactura-rust/blob/main/docs/RULES.ro.md) · [**Rule Reference (EN)**](https://github.com/StBogdan/ro-efactura-rust/blob/main/docs/RULES.en.md)

Validator în Rust pentru facturile electronice **ANAF `RO e-Factura`** (**`CIUS-RO 1.0.1`** / `ROeFactura v1.0.9`).

`cius-ro` rulează direct în memorie, fără Java/JVM, fără biblioteci C externe și fără cod `unsafe` (`#![forbid(unsafe_code)]`). Primește fie un fișier XML (`UBL 2.1` sau `UN/CEFACT CII D16B`), fie un document hibrid `PDF/A-3` și returnează diagnostice detaliate în română sau engleză.

## Ce verifică

1. **Inspecție `PDF/A-3` (`ISO 19005-3`) și extragerea `efactura.xml`** (`feature = "pdfa3"`, activat implicit)
   Verifică structura PDF necriptată, identificatorul `/ID`, filtrele de compresie, profilul de culoare ICC `GTS_PDFA1`, fonturile încorporate și metadatele XMP `pdfaid:part=3`, apoi extrage fișierul XML atașat (`efactura.xml` / `factur-x.xml`).
2. **Validare schemă XML (`W3C XSD 1.1`) și protecție XXE** (`feature = "xsd"`, activat implicit)
   Respinge declarațiile `<!DOCTYPE` / `<!ENTITY` și validează documentul față de schemele oficiale incluse OASIS UBL 2.1 (`Invoice`, `CreditNote`) și UN/CEFACT CII D16B (`100pD16B`).
3. **Modelul european `EN 16931` cu aritmetică zecimală exactă**
   Verifică toate calculele pe linii, reduceri/taxe suplimentare, subtotale TVA și totaluri de plată folosind `en16931` (`rust_decimal`), fără erori de rotunjire în virgulă mobilă.
4. **Regulile naționale ANAF `CIUS-RO 1.0.1` și cifrele de control `cius-ro-id`**
   Aplică toate cele 122 de reguli `BR-RO-*` — date fără fus orar (`YYYY-MM-DD` / `YYYYMMDD`), TVA în `RON` pe facturile în valută, județe `ISO 3166-2:RO`, sectoarele `SECTOR1`..`SECTOR6` pentru București, lungimi maxime de câmpuri, limite de atașamente și maximum 2 zecimale pe sumele monetare (`BR-RO-Z2`) — împreună cu validarea `CUI`, `CNP` / `0000000000000`, `ONRC` și `IBAN RO`, plus export opțional în format OASIS `SVRL` XML.

## Utilizare

```toml
[dependencies]
cius-ro = "0.1"

# Variantă minimală doar pentru XML (fără schemele XSD încorporate și fără parser PDF/A-3):
# cius-ro = { version = "0.1", default-features = false }
```

```rust,no_run
use cius_ro::{validate_bytes, Locale, ValidateOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let octeti_factura = std::fs::read("factura.xml")?;
    let raport = validate_bytes(&octeti_factura, &ValidateOptions::with_locale(Locale::Ro));

    if raport.is_valid {
        println!("Factură conformă CIUS-RO 1.0.1!");
    } else {
        for eroare in &raport.violations {
            eprintln!("[{}] {}: {}", eroare.severity, eroare.code, eroare.message);
        }
    }
    Ok(())
}
```

## Licență

Distribuit sub licență dublă **MIT** sau **Apache-2.0**, la alegere.
