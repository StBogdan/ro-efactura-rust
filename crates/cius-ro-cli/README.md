# cius-ro-cli

[**Română (`README.ro.md`)**](https://github.com/StBogdan/ro-efactura-rust/blob/main/crates/cius-ro-cli/README.ro.md) · [**GitHub Repository**](https://github.com/StBogdan/ro-efactura-rust)

Command-line validator (`cius-ro`) for Romanian **ANAF `RO e-Factura`** (`CIUS-RO 1.0.1`) invoices and fiscal identifiers.

Validates standalone XML (`OASIS UBL 2.1` and `UN/CEFACT CII D16B`) as well as hybrid `PDF/A-3` files (`efactura.xml`) in a single self-contained binary — no Java runtime or external XML libraries needed.

## Install

```bash
cargo install cius-ro-cli
```

## Examples

```bash
# Validate XML or hybrid PDF/A-3 files (pass `-` to read from stdin)
cius-ro check invoice.xml hybrid_invoice.pdf --lang ro --deny-warnings

# Emit machine-readable reports (JSON, OASIS SVRL XML, or GitHub SARIF v2.1.0)
cius-ro check invoice.xml --format json
cius-ro check invoice.xml --format svrl > report.svrl.xml
cius-ro check invoice.xml --format sarif > results.sarif

# Extract the embedded efactura.xml / factur-x.xml from a PDF/A-3 document
cius-ro extract hybrid_invoice.pdf -o efactura.xml

# Verify a CUI, CNP, ONRC, RO IBAN, county code, or Bucharest sector
cius-ro id RO45678918 --lang ro
cius-ro id "Sector 1" --lang ro --json

# Browse or look up CIUS-RO 1.0.1 rules
cius-ro rules --lang ro
cius-ro rules BR-RO-100 --lang ro
cius-ro rules --markdown --lang ro > RULES.ro.md
```

## Exit Codes

| Code | Meaning |
| :---: | :--- |
| `0` | All checked files or identifiers are valid. |
| `1` | One or more validation rules failed (or an unknown rule code was queried). |
| `2` | File I/O error or invalid command-line arguments. |

## License

Dual-licensed under **MIT** or **Apache-2.0**, at your option.
