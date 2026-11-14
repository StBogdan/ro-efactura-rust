# cius-ro

[**Română (`README.ro.md`)**](./README.ro.md) · [**Rule Reference (EN)**](https://github.com/StBogdan/ro-efactura-rust/blob/main/docs/RULES.en.md) · [**Catalog Reguli (RO)**](https://github.com/StBogdan/ro-efactura-rust/blob/main/docs/RULES.ro.md)

Validate Romanian electronic invoices (**ANAF `RO e-Factura`** / **`CIUS-RO 1.0.1`**) in pure Rust.

`cius-ro` validates byte slices without a JVM, external XML libraries, or `unsafe` code (`#![forbid(unsafe_code)]`). Pass raw XML (`UBL 2.1` or `UN/CEFACT CII D16B`) or a hybrid `PDF/A-3` document, and get structured validation results in English or Romanian.

## What It Checks

1. **Hybrid `PDF/A-3` (`ISO 19005-3`) & XML Extraction** (`feature = "pdfa3"`, on by default)
   Checks unencrypted PDF structure, `/ID`, stream filters, `GTS_PDFA1` ICC color profile, embedded font glyph programs, and XMP `pdfaid:part=3` metadata under bounded decompression limits (`64 MiB` per stream, `128 MiB` total), then extracts the associated `efactura.xml` / `factur-x.xml` payload.
2. **XML Schema (`W3C XSD 1.1`) & XXE Protection** (`feature = "xsd"`, on by default)
   Blocks `<!DOCTYPE` and `<!ENTITY` declarations and validates against bundled official OASIS UBL 2.1 (`Invoice`, `CreditNote`) and UN/CEFACT CII D16B (`100pD16B`) schemas (compiled once per process via an ephemeral temporary directory on native targets; browser `wasm32-unknown-unknown` runs well-formedness and XXE gates in memory and skips multi-file XSD compilation).
3. **CEN `EN 16931` Decimal Math**
   Evaluates the European core invoice model using base-10 fixed-point arithmetic (`en16931` + `rust_decimal`) so penny-level VAT and total checks never suffer from floating-point drift.
4. **ANAF `CIUS-RO 1.0.1` (`ROeFactura v1.0.9`) & Fiscal Checksums**
   Enforces all 122 official `BR-RO-*` rules — including strict `YYYY-MM-DD` / `YYYYMMDD` dates, `RON` tax currency on foreign-currency invoices, `ISO 3166-2:RO` county codes, Bucharest `SECTOR1`..`SECTOR6`, XPath `normalize-space()` character length limits, attachment caps, and 2-decimal monetary caps (`BR-RO-Z2`) — alongside `cius-ro-id` checksums (`CUI`, `CNP` / `0000000000000`, `ONRC`, `RO IBAN`) and optional OASIS `SVRL` XML output (`feature = "svrl"`, on by default).

## Usage

```toml
[dependencies]
cius-ro = "0.1"

# Default features: ["xsd", "pdfa3", "serde", "svrl"]
# Minimal XML-only mode (skips embedded XSD schemas, PDF/A-3 parser, serde, and SVRL):
# cius-ro = { version = "0.1", default-features = false }
```

```rust,no_run
use cius_ro::{validate_bytes, Locale, ValidateOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let invoice_bytes = std::fs::read("invoice.xml")?;
    let opts = ValidateOptions::with_locale(Locale::Ro).deny_warnings(true);
    let report = validate_bytes(&invoice_bytes, &opts);

    if report.is_valid {
        println!("Compliant CIUS-RO 1.0.1 invoice!");
    } else {
        for v in &report.violations {
            eprintln!("[{}] {}: {}", v.severity, v.code, v.message);
        }
    }
    Ok(())
}
```

## License

Dual-licensed under **MIT** or **Apache-2.0**, at your option.
