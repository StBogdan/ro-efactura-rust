# cius-ro-wasm

[**Română (`README.ro.md`)**](./README.ro.md) · [**GitHub Repository**](https://github.com/StBogdan/ro-efactura-rust)

WebAssembly bindings (`wasm-bindgen`) for [`cius-ro`](https://crates.io/crates/cius-ro) and [`cius-ro-id`](https://crates.io/crates/cius-ro-id).

Validate **ANAF `RO e-Factura`** (`CIUS-RO 1.0.1`) invoices, extract `efactura.xml` from hybrid `PDF/A-3` files, and check Romanian `CUI`, `CNP`, `ONRC`, and `IBAN` identifiers directly in the browser, Edge Workers, Deno, or Node.js — without uploading sensitive invoice data to a server.

## Build

```bash
wasm-pack build crates/cius-ro-wasm --target web --release
```

## JavaScript / TypeScript API

- `validateInvoice(bytes: Uint8Array, locale?: "en" | "ro", denyWarnings?: boolean, includeSvrl?: boolean): ValidationReport`
- `extractPdfXml(bytes: Uint8Array): Uint8Array`
- `validateIdentifier(input: string, locale?: "en" | "ro"): IdentifierInspection`
- `listRules(locale?: "en" | "ro"): RuleView[]`

```js
import init, {
  validateInvoice,
  extractPdfXml,
  validateIdentifier,
  listRules,
} from "./pkg/cius_ro_wasm.js";

await init();

const report = validateInvoice(invoiceUint8Array, "ro", true, false);
if (!report.isValid) {
  console.error(report.violations);
}

const check = validateIdentifier("RO45678918", "ro");
console.log(check.isValid, check.normalized);
```

## License

Dual-licensed under **MIT** or **Apache-2.0**, at your option.
