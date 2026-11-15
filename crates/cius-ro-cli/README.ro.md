# cius-ro-cli

[**English (`README.md`)**](https://github.com/StBogdan/ro-efactura-rust/blob/main/crates/cius-ro-cli/README.md) · [**Documentația principală**](https://github.com/StBogdan/ro-efactura-rust/blob/main/README.ro.md)

Utilitar în linie de comandă (`cius-ro`) pentru validarea facturilor electronice **ANAF `RO e-Factura`** (`CIUS-RO 1.0.1`) și a codurilor fiscale din România.

Verifică atât fișiere XML (`OASIS UBL 2.1` și `UN/CEFACT CII D16B`), cât și documente hibride `PDF/A-3` (`efactura.xml`) dintr-un singur executabil nativ — fără Java/JVM sau utilitare externe.

## Instalare

```bash
cargo install cius-ro-cli
```

## Exemple de utilizare

```bash
# Validare fișiere XML sau PDF/A-3 hibrid (sau `-` pentru citire din stdin)
cius-ro check factura.xml factura_hibrida.pdf --lang ro --deny-warnings

# Export rapoarte structurate (JSON, OASIS SVRL XML sau SARIF v2.1.0 pentru CI/CD)
cius-ro check factura.xml --lang ro --format json
cius-ro check factura.xml --lang ro --format svrl > raport.svrl.xml
cius-ro check factura.xml --lang ro --format sarif > rezultate.sarif

# Extragerea fișierului XML atașat (efactura.xml / factur-x.xml) dintr-un PDF/A-3
cius-ro extract factura_hibrida.pdf -o efactura.xml

# Verificarea unui CUI, CNP, nr. ONRC, IBAN RO, județ sau sector București
cius-ro id RO45678918 --lang ro
cius-ro id "Sector 1" --lang ro --json

# Consultarea regulilor CIUS-RO 1.0.1
cius-ro rules --lang ro
cius-ro rules BR-RO-100 --lang ro
cius-ro rules --markdown --lang ro > RULES.ro.md
```

## Coduri de ieșire

| Cod | Semnificație |
| :---: | :--- |
| `0` | Toate documentele sau codurile verificate sunt valide. |
| `1` | Au fost găsite erori de conformitate (sau codul regulii căutate nu există). |
| `2` | Eroare la citirea/scrierea fișierului sau argumente invalide în linia de comandă. |

## Licență

Distribuit sub licență dublă **MIT** sau **Apache-2.0**, la alegere.
