# cius-ro-wasm

[**English (`README.md`)**](./README.md) · [**Documentația principală**](https://github.com/StBogdan/ro-efactura-rust/blob/main/README.ro.md)

Pachet WebAssembly (`wasm-bindgen`) pentru [`cius-ro`](https://crates.io/crates/cius-ro) și [`cius-ro-id`](https://crates.io/crates/cius-ro-id).

Permite validarea facturilor electronice **ANAF `RO e-Factura`** (`CIUS-RO 1.0.1`), extragerea fișierului `efactura.xml` din documente hibride `PDF/A-3` și verificarea codurilor `CUI`, `CNP`, `ONRC` și `IBAN` direct în browser, Cloudflare Workers, Deno sau Node.js — fără a trimite datele facturilor către un server extern.

## Compilare

```bash
wasm-pack build crates/cius-ro-wasm --target web --release
```

## API JavaScript / TypeScript

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

const raport = validateInvoice(octetiFacturaUint8Array, "ro", true, false);
if (!raport.isValid) {
  console.error(raport.violations);
}

const verificare = validateIdentifier("RO45678918", "ro");
console.log(verificare.isValid, verificare.normalized);
```

## Licență

Distribuit sub licență dublă **MIT** sau **Apache-2.0**, la alegere.
