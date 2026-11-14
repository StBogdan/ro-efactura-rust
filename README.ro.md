# ro-efactura

[**English (`README.md`)**](./README.md) · [**Catalog Reguli (RO)**](./docs/RULES.ro.md) · [**Rule Reference (EN)**](./docs/RULES.en.md)

Bibliotecă și utilitar CLI în Rust pentru validarea facturilor electronice **ANAF `RO e-Factura`** (**`CIUS-RO 1.0.1`**) și a identificatorilor fiscali din România — fără Java/JVM, fără utilitare externe și cu calcule zecimale exacte.

Construită conform regulilor naționale B2B, B2G și B2C (*OUG 120/2021*, *Legea 296/2023* și *OUG 69/2024*, aliniat la specificația tehnică ANAF `ROeFactura v1.0.9`) și publicată open-source din motorul de validare [**VeriFactura**](https://verifactura.co).

## Pachete disponibile

| Crate | Descriere | `no_std` | `wasm32` |
| :--- | :--- | :---: | :---: |
| [`cius-ro-id`](./crates/cius-ro-id/README.ro.md) | Parsare și validare pentru CUI/CIF, CNP, nr. Reg. Com. (ONRC), IBAN RO, județe și sectoare București (compatibil `no_std`). | Da | Da |
| [`cius-ro`](./crates/cius-ro/README.ro.md) | Validator complet `CIUS-RO 1.0.1` pentru fișiere XML (`UBL 2.1`, `CII D16B`) și PDF-uri hibride `PDF/A-3`. | — | Da¹ |
| [`cius-ro-cli`](./crates/cius-ro-cli/README.ro.md) | Utilitar în linie de comandă (`cius-ro`) pentru verificări înainte de încărcarea în SPV, CI/CD și extragerea XML din PDF. | — | — |
| [`cius-ro-wasm`](./crates/cius-ro-wasm/README.ro.md) | Pachet WebAssembly pentru rularea validatorului direct în browser, Cloudflare Workers, Deno sau Node.js. | — | Da¹ |

¹ *Pe `wasm32-unknown-unknown`, sintaxa XML, protecția XXE (`<!DOCTYPE` / `<!ENTITY`), modelul `EN 16931`, regulile `BR-RO-*`, `cius-ro-id` și inspecția `PDF/A-3` rulează în memorie; compilarea schemelor XSD multi-fișier (care folosește un director temporar efemer la inițializare pe platforme native) este marcată `SKIPPED`.*

## De ce `ro-efactura`?

- **Fără Java / JVM sau utilitare externe:** Scris integral în Rust fără cod `unsafe` (`#![forbid(unsafe_code)]`, Edition 2024), cu schemele XSD incluse în binarul compilat, tipuri `Copy` fără alocare pe heap pentru identificatori (`Cui`, `Cnp`, `CnpOrPlaceholder`, `Onrc`) și decomprimare PDF limitată împotriva arhivelor supradimensionate. Nu necesită Saxon-HE sau `xmllint`.
- **Calcule financiare exacte în baza 10:** Liniile facturii, reducerile/taxele suplimentare, defalcarea TVA (`BG-23`) și totalurile de plată (`BG-22`) sunt verificate cu `rust_decimal`, evitând diferențele de rotunjire la nivel de ban specifice numerelor `f64`.
- **Toate regulile oficiale ANAF `ROeFactura v1.0.9`:** Acoperă cele 122 de reguli `BR-RO-*` — inclusiv datele calendaristice fără fus orar (`YYYY-MM-DD`), TVA-ul exprimat în `RON` pentru facturile în valută, plafonul de 2 zecimale pe sumele monetare (`BR-RO-Z2`), lungimile maxime de câmpuri și numărul maxim de atașamente.
- **Mesaje clare în română și engleză, cu sugestii de corectare:** Semnalează greșelile frecvente înainte de transmiterea în SPV ANAF și propune automat forma corectă (de exemplu, `'RO-CJ'` în loc de `'Cluj'` sau `'SECTOR1'` în loc de `'Sectorul 1'`).

## Început rapid

### 1. Validarea unei facturi în Rust (`cius-ro`)

```toml
[dependencies]
cius-ro = "0.1"
```

```rust,no_run
use cius_ro::{validate_bytes, Locale, ValidateOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Acceptă direct atât fișiere XML (UBL 2.1 / CII D16B), cât și PDF/A-3 hibrid
    let octeti = std::fs::read("factura.xml")?;
    let optiuni = ValidateOptions::with_locale(Locale::Ro).deny_warnings(true);
    let raport = validate_bytes(&octeti, &optiuni);

    if raport.is_valid {
        println!("Factură validă CIUS-RO 1.0.1 ({:?})", raport.detected_syntax);
    } else {
        for eroare in &raport.violations {
            eprintln!("[{}] {} — {}", eroare.severity, eroare.code, eroare.message);
            if let Some(sugestie) = &eroare.hint {
                eprintln!("  Sugestie: {sugestie}");
            }
        }
    }
    Ok(())
}
```

### 2. Verificarea codurilor fiscale și administrative (`cius-ro-id`)

```toml
[dependencies]
cius-ro-id = "0.1"
```

```rust
use cius_ro_id::{BucharestSector, Cnp, County, Cui, Onrc, RoIban};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cui: Cui = "RO 45678918".parse()?;
    assert_eq!(cui.digits(), "45678918");
    assert_eq!(cui.to_vat_id(), "RO45678918");

    let onrc: Onrc = "J40/1234/2022".parse()?;
    let iban: RoIban = "RO49 AAAA 1B31 0075 9384 0000".parse()?;
    let cnp: Cnp = "1900101410011".parse()?;
    let _ = (onrc, iban, cnp);

    assert_eq!(County::lookup_fuzzy("Cluj"), Some(County::CJ));
    assert_eq!(
        BucharestSector::normalize_fuzzy("Sectorul 1"),
        Some(BucharestSector::Sector1)
    );
    Ok(())
}
```

### 3. Linia de comandă (`cius-ro-cli`)

```bash
cargo install cius-ro-cli

# Validare XML sau PDF/A-3 hibrid cu mesaje în limba română
cius-ro check factura.xml factura_hibrida.pdf --lang ro --deny-warnings

# Export rapoarte în format JSON, OASIS SVRL XML sau SARIF v2.1.0 (pentru CI/CD)
cius-ro check factura.xml --lang ro --format json
cius-ro check factura.xml --lang ro --format svrl > raport.svrl.xml
cius-ro check factura.xml --lang ro --format sarif > rezultate.sarif

# Extragerea fișierului XML atașat (efactura.xml / factur-x.xml) dintr-un PDF/A-3
cius-ro extract factura_hibrida.pdf -o efactura.xml

# Verificarea rapidă a unui CUI, CNP, nr. ONRC, IBAN RO, județ sau sector
cius-ro id RO45678918 --lang ro
cius-ro id "Sector 1" --lang ro

# Consultarea explicațiilor pentru o regulă ANAF
cius-ro rules BR-RO-110 --lang ro
```

### 4. WebAssembly în browser sau Node.js (`cius-ro-wasm`)

```bash
wasm-pack build crates/cius-ro-wasm --target web --release
```

```js
import init, { validateInvoice, validateIdentifier } from "./pkg/cius_ro_wasm.js";

await init();

const raport = validateInvoice(fisierUint8Array, "ro", true, false);
console.log(raport.isValid, raport.violations);

const verificare = validateIdentifier("RO45678918", "ro");
console.log(verificare.isValid, verificare.normalized);
```

## Etapele de validare

`cius-ro` parcurge 4 etape de validare:

```text
Octeți de intrare (&[u8]: XML sau %PDF-)
  │
  ├─► 1. Inspecție container PDF/A-3 (ISO 19005-3, feature = "pdfa3")
  │     Verifică absența criptării, profilul de culoare ICC, fonturile încorporate,
  │     metadatele XMP pdfaid:part=3 și extrage atașamentul efactura.xml / factur-x.xml.
  │
  ├─► 2. Validare schemă XML și protecție XXE (W3C XSD 1.1, feature = "xsd")
  │     Blochează declarațiile <!DOCTYPE / <!ENTITY și validează structura XML față de
  │     schemele oficiale OASIS UBL 2.1 (Invoice, CreditNote) și UN/CEFACT CII D16B.
  │
  ├─► 3. Modelul semantic CEN EN 16931 (en16931 + rust_decimal)
  │     Verifică exactitatea calculelor pe linii, reduceri/taxe, subtotale TVA și totaluri.
  │
  └─► 4. Regulile ANAF CIUS-RO 1.0.1 și cifrele de control fiscale
        Aplică cele 122 de reguli BR-RO-* (CustomizationID, cifră în numărul facturii,
        TVA în RON pentru facturi în valută, județe ISO 3166-2:RO, SECTOR1..6,
        formate de dată, limite de caractere și 2 zecimale) + CUI, CNP, ONRC și IBAN.
```

Lista completă a regulilor și pașii de remediere sunt disponibile în [**Catalogul Regulilor în Limba Română (`docs/RULES.ro.md`)**](./docs/RULES.ro.md) și în [**English Rule Reference (`docs/RULES.en.md`)**](./docs/RULES.en.md).

## Licență

Distribuit sub licență dublă, la alegere:

- **MIT License** ([`LICENSE-MIT`](./LICENSE-MIT))
- **Apache License, Version 2.0** ([`LICENSE-APACHE`](./LICENSE-APACHE))
