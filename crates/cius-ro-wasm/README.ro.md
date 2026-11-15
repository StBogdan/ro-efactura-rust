# cius-ro-wasm

[**English (`README.md`)**](https://github.com/StBogdan/ro-efactura-rust/blob/main/crates/cius-ro-wasm/README.md) · [**Documentația principală**](https://github.com/StBogdan/ro-efactura-rust/blob/main/README.ro.md)

Pachet WebAssembly (`wasm-bindgen`) pentru [`cius-ro`](https://crates.io/crates/cius-ro) și [`cius-ro-id`](https://crates.io/crates/cius-ro-id).

Validează facturi electronice **ANAF `RO e-Factura`** (`CIUS-RO 1.0.1`), extrage `efactura.xml` din documente hibride `PDF/A-3` și verifică coduri `CUI`, `CNP`, `ONRC` și `IBAN` direct în browser, Cloudflare Workers, Deno sau Node.js — fără a trimite facturile către un server.

## Compilare

```bash
wasm-pack build crates/cius-ro-wasm --target web --release
```

## API JavaScript / TypeScript

Fișierele generate `pkg/cius_ro_wasm.d.ts` includ definiții TypeScript complete generate prin `tsify` (`ValidateOptions`, `ValidationReport`, `Violation`, `EngineReport`, `IdentifierInspection`, `RuleView`, `Locale`, `Severity` etc.).

- `validateInvoice(bytes: Uint8Array, options?: ValidateOptions | null): ValidationReport`
- `extractPdfXml(bytes: Uint8Array): Uint8Array`
- `validateIdentifier(input: string, locale?: Locale | null): IdentifierInspection`
- `listRules(locale?: Locale | null): RuleView[]`

Toate câmpurile din [`ValidateOptions`](https://docs.rs/cius-ro/latest/cius_ro/struct.ValidateOptions.html) (`locale`, `runPdfa`, `runXsd`, `runEn16931`, `runCiusRoRules`, `runChecksums`, `strictCiusRoCustomization`, `denyWarnings`, `includeSvrl`) sunt opționale și folosesc valorile implicite atunci când sunt omise.

> **Notă:** Pe `wasm32-unknown-unknown`, validarea sintaxei XML, protecția XXE (`<!DOCTYPE` / `<!ENTITY`), modelul CEN EN 16931, toate cele 122 de reguli ANAF `BR-RO-*`, verificările `cius-ro-id` și verificarea containerului `PDF/A-3` rulează direct în memorie; validarea XSD multi-fișier pe disc este marcată `"SKIPPED"`.

```js
import init, {
  validateInvoice,
  extractPdfXml,
  validateIdentifier,
  listRules,
} from "./pkg/cius_ro_wasm.js";

await init();

const raport = validateInvoice(octetiFacturaUint8Array, {
  locale: "ro",
  denyWarnings: true,
  includeSvrl: false,
});
if (!raport.isValid) {
  console.error(raport.violations);
}

const verificare = validateIdentifier("RO45678918", "ro");
console.log(verificare.isValid, verificare.normalized);
```

## Licență

Distribuit sub licență dublă **MIT** sau **Apache-2.0**, la alegere.
