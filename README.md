# ro-efactura

[**Română (`README.ro.md`)**](./README.ro.md) · [**Rule Reference (EN)**](./docs/RULES.en.md) · [**Catalog Reguli (RO)**](./docs/RULES.ro.md)

Validate Romanian electronic invoices (**ANAF `RO e-Factura`** / **`CIUS-RO 1.0.1`**) and fiscal identifiers in pure Rust — with no JVM, no external XML tools, and exact base-10 decimal math.

Built for Romania's national B2B, B2G, and B2C e-invoicing system (*OUG 120/2021*, *Law 296/2023*, and *OUG 69/2024*, aligned with ANAF `ROeFactura v1.0.9`), and open-sourced from the validation engine behind [**VeriFactura**](https://verifactura.co).

## Crates

| Crate | Description | `no_std` | `wasm32` |
| :--- | :--- | :---: | :---: |
| [`cius-ro-id`](./crates/cius-ro-id) | Parse and verify Romanian CUI/CIF, CNP, ONRC, IBAN, counties, and Bucharest sectors (`no_std` compatible). | Yes | Yes |
| [`cius-ro`](./crates/cius-ro) | Full `CIUS-RO 1.0.1` validator for standalone XML (`UBL 2.1`, `CII D16B`) and hybrid `PDF/A-3` invoices. | — | Yes¹ |
| [`cius-ro-cli`](./crates/cius-ro-cli) | Command-line tool (`cius-ro`) for CI/CD checks, ERP pre-flight validation, and PDF XML extraction. | — | — |
| [`cius-ro-wasm`](./crates/cius-ro-wasm) | WebAssembly bindings for running the validator in browsers, Edge Workers, Deno, and Node.js. | — | Yes¹ |

¹ *On `wasm32-unknown-unknown`, XML well-formedness, `<!DOCTYPE` / `<!ENTITY` XXE blocking, `EN 16931`, `BR-RO-*`, `cius-ro-id`, and `PDF/A-3` checks run in memory; multi-file XSD compilation (which uses a one-time ephemeral temporary directory on native targets) is marked `SKIPPED`.*

## Highlights

- **Self-contained & fast:** Written in 100% safe Rust (`#![forbid(unsafe_code)]`, Edition 2024) with bundled XSD schemas, zero-heap-allocation `Copy` identifier types (`Cui`, `Cnp`, `CnpOrPlaceholder`, `Onrc`), and bounded PDF stream decompression. No Java runtime, Saxon-HE, or `xmllint` installation required.
- **Exact financial arithmetic:** Line totals, allowances/charges, VAT breakdowns (`BG-23`), and payable amounts (`BG-22`) are checked in base-10 fixed-point (`rust_decimal`), avoiding floating-point rounding drift.
- **Full ANAF `ROeFactura v1.0.9` rule set:** Covers all 122 official `BR-RO-*` rules — including strict timezone-free dates (`YYYY-MM-DD`), RON tax-currency rules on foreign-currency invoices, 2-decimal monetary caps (`BR-RO-Z2`), field length limits, and attachment cardinality caps.
- **Bilingual diagnostics (`en` & `ro`) with fix hints:** Every error and warning is available in English and Romanian, with automatic suggestions for common mistakes (for example, `"Cluj"` -> `"RO-CJ"` or `"Sector 1"` -> `"SECTOR1"`).

## Quick Start

### 1. Validate an invoice in Rust (`cius-ro`)

```toml
[dependencies]
cius-ro = "0.1"
```

```rust,no_run
use cius_ro::{validate_bytes, Locale, ValidateOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Accepts standalone XML (UBL 2.1 / CII D16B) or a hybrid PDF/A-3 file
    let bytes = std::fs::read("invoice.xml")?;
    let opts = ValidateOptions::with_locale(Locale::Ro).deny_warnings(true);
    let report = validate_bytes(&bytes, &opts);

    if report.is_valid {
        println!("Valid CIUS-RO invoice ({:?})", report.detected_syntax);
    } else {
        for v in &report.violations {
            eprintln!("[{}] {} — {}", v.severity, v.code, v.message);
            if let Some(hint) = &v.hint {
                eprintln!("  Hint: {hint}");
            }
        }
    }
    Ok(())
}
```

### 2. Verify Romanian identifiers (`cius-ro-id`)

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

### 3. Command-line tool (`cius-ro-cli`)

```bash
cargo install cius-ro-cli

# Validate standalone XML or hybrid PDF/A-3 files
cius-ro check invoice.xml hybrid_invoice.pdf --lang ro --deny-warnings

# Export structured reports (JSON, OASIS SVRL XML, or GitHub SARIF v2.1.0)
cius-ro check invoice.xml --format json
cius-ro check invoice.xml --format svrl > report.svrl.xml
cius-ro check invoice.xml --format sarif > results.sarif

# Extract embedded efactura.xml / factur-x.xml from a PDF/A-3 invoice
cius-ro extract hybrid_invoice.pdf -o efactura.xml

# Check a CUI, CNP, ONRC, RO IBAN, county, or Bucharest sector
cius-ro id RO45678918 --lang ro
cius-ro id "Sector 1" --lang ro

# Look up a rule by code
cius-ro rules BR-RO-110 --lang ro
```

### 4. WebAssembly (`cius-ro-wasm`)

```bash
wasm-pack build crates/cius-ro-wasm --target web --release
```

```js
import init, { validateInvoice, validateIdentifier } from "./pkg/cius_ro_wasm.js";

await init();

const report = validateInvoice(fileUint8Array, "ro", true, false);
console.log(report.isValid, report.violations);

const idCheck = validateIdentifier("RO45678918", "ro");
console.log(idCheck.isValid, idCheck.normalized);
```

## How Validation Works

`cius-ro` runs inputs through four validation stages:

```text
Input bytes (&[u8]: XML or %PDF-)
  │
  ├─► 1. PDF/A-3 Container Check (ISO 19005-3, feature = "pdfa3")
  │     Verifies unencrypted structure, ICC color profile, embedded fonts,
  │     XMP pdfaid:part=3 metadata, and extracts efactura.xml / factur-x.xml.
  │
  ├─► 2. XML Schema & XXE Gate (W3C XSD 1.1, feature = "xsd")
  │     Rejects <!DOCTYPE / <!ENTITY declarations and validates against bundled
  │     OASIS UBL 2.1 (Invoice, CreditNote) and UN/CEFACT CII D16B schemas.
  │
  ├─► 3. CEN EN 16931 Semantic Model (en16931 + rust_decimal)
  │     Checks base-10 line math, allowances/charges, VAT subtotals, and totals.
  │
  └─► 4. ANAF CIUS-RO 1.0.1 Rules & Fiscal Checksums
        Enforces all 122 BR-RO-* rules (CustomizationID, digit in invoice ID,
        RON VAT on foreign-currency invoices, ISO 3166-2:RO counties, SECTOR1..6,
        date formats, length/cardinality/2-decimal caps) plus CUI, CNP, ONRC, and IBAN.
```

For the full list of rules and remediation steps, see [**`docs/RULES.en.md`**](./docs/RULES.en.md) (English) and [**`docs/RULES.ro.md`**](./docs/RULES.ro.md) (Romanian).

## License

Dual-licensed under either of:

- **MIT License** ([`LICENSE-MIT`](./LICENSE-MIT))
- **Apache License, Version 2.0** ([`LICENSE-APACHE`](./LICENSE-APACHE))

at your option.
