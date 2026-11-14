# cius-ro-wasm

[**Română (`README.ro.md`)**](./README.ro.md) · [**GitHub Repository**](https://github.com/StBogdan/ro-efactura-rust)

WebAssembly bindings (`wasm-bindgen`) for [`cius-ro`](https://crates.io/crates/cius-ro) and [`cius-ro-id`](https://crates.io/crates/cius-ro-id).

Validate **ANAF `RO e-Factura`** (`CIUS-RO 1.0.1`) invoices, extract `efactura.xml` from hybrid `PDF/A-3` files, and check Romanian `CUI`, `CNP`, `ONRC`, and `IBAN` identifiers directly in the browser, Edge Workers, Deno, or Node.js — without uploading sensitive invoice data to a server.

## Build

```bash
wasm-pack build crates/cius-ro-wasm --target web --release
```

## JavaScript / TypeScript API

Generated `pkg/cius_ro_wasm.d.ts` bundles ship strict TypeScript interfaces (`ValidationReport`, `Violation`, `EngineReport`, `IdentifierInspection`, `RuleView`, `Locale`, `Severity`, etc.).

- `validateInvoice(bytes: Uint8Array, locale?: "en" | "ro", denyWarnings?: boolean, includeSvrl?: boolean): ValidationReport`
- `extractPdfXml(bytes: Uint8Array): Uint8Array`
- `validateIdentifier(input: string, locale?: "en" | "ro"): IdentifierInspection`
- `listRules(locale?: "en" | "ro"): RuleView[]`

> **Note:** On `wasm32-unknown-unknown`, XML well-formedness, `<!DOCTYPE` / `<!ENTITY` XXE blocking, CEN EN 16931 element-order and fixed-point decimal math, all 122 ANAF `BR-RO-*` assertions, `cius-ro-id` checksums, and ISO 19005-3 `PDF/A-3` container checks run in memory; multi-file filesystem XSD compilation is marked `"SKIPPED"`.

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
