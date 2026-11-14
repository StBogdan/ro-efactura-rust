# Catalogul Regulilor de Validare ANAF RO e-Factura (`CIUS-RO 1.0.1`)

> **Limba / Language:** [English](RULES.en.md) | **Română**

Acest ghid este generat automat din catalogul `cius_ro::rules::RULES` și acoperă toate regulile naționale ANAF `RO_CIUS` (`v1.0.9`), verificările algoritmice `cius-ro-id` (CUI, CNP, ONRC, IBAN), schema oficială W3C XSD 1.1 și inspecția containerelor hibride PDF/A-3 (`efactura.xml`).

| Cod regulă | Severitate | Termeni de business | Titlu |
| :--- | :---: | :--- | :--- |
| [`BR-RO-001`](#br-ro-001) | `FATAL` | `BT-24` | Identificatorul specificației CIUS-RO 1.0.1 |
| [`BR-RO-010`](#br-ro-010) | `FATAL` | `BT-1` | Caracter numeric obligatoriu în numărul facturii |
| [`BR-RO-020`](#br-ro-020) | `FATAL` | `BT-3` | Coduri permise pentru tipul facturii (UNTDID 1001) |
| [`BR-RO-030`](#br-ro-030) | `FATAL` | `BT-5, BT-6, BT-111` | Monedă străină — contabilizare TVA obligatorie în RON |
| [`BR-RO-040`](#br-ro-040) | `FATAL` | `BT-8` | Codul datei de exigibilitate a TVA (UNTDID 2005 / 2475) |
| [`BR-RO-065`](#br-ro-065) | `FATAL` | `BT-31, BT-32, BT-63` | Identificator fiscal sau de TVA al Vânzătorului obligatoriu |
| [`BR-RO-080`](#br-ro-080) | `FATAL` | `BT-35, BT-50` | Adresa Vânzătorului și a Cumpărătorului (Linia 1) obligatorie |
| [`BR-RO-090`](#br-ro-090) | `FATAL` | `BT-37, BT-52` | Localitatea Vânzătorului și a Cumpărătorului obligatorie |
| [`BR-RO-100`](#br-ro-100) | `FATAL` | `BT-37, BT-39, BT-40, BT-52, BT-54, BT-55` | Codificare obligatorie SECTOR1..SECTOR6 pentru Municipiul București (RO-B) |
| [`BR-RO-110`](#br-ro-110) | `FATAL` | `BT-39, BT-40, BT-54, BT-55` | Subdiviziunea țării pentru România trebuie codificată ISO 3166-2:RO |
| [`BR-RO-120`](#br-ro-120) | `FATAL` | `BT-47, BT-48` | Identificatorul legal sau de TVA al Cumpărătorului obligatoriu |
| [`BR-RO-140`](#br-ro-140) | `FATAL` | `BG-11, BG-12, BT-64` | Linia 1 de adresă a reprezentantului fiscal obligatorie |
| [`BR-RO-150`](#br-ro-150) | `FATAL` | `BG-11, BG-12, BT-66` | Localitatea reprezentantului fiscal obligatorie |
| [`BR-RO-160`](#br-ro-160) | `FATAL` | `BT-66, BT-68, BT-69` | Sector București pentru reprezentantul fiscal (RO-B) |
| [`BR-RO-170`](#br-ro-170) | `FATAL` | `BT-68, BT-69` | Cod județ ISO 3166-2:RO pentru reprezentantul fiscal |
| [`BR-RO-180`](#br-ro-180) | `FATAL` | `BG-15, BT-75` | Linia 1 a adresei de livrare obligatorie când BG-15 există |
| [`BR-RO-200`](#br-ro-200) | `FATAL` | `BG-15, BT-77, BT-79, BT-80` | Localitatea de livrare și regula SECTOR1..6 pentru RO-B |
| [`BR-RO-210`](#br-ro-210) | `FATAL` | `BG-15, BT-79, BT-80` | Subdiviziunea țării la livrare (BT-79 / ISO 3166-2:RO) |
| [`BR-RO-DT001`](#br-ro-dt001) | `FATAL` | `BT-2` | Format strict YYYY-MM-DD pentru data emiterii (IssueDate) |
| [`BR-RO-DT002`](#br-ro-dt002) | `FATAL` | `BT-7` | Format strict YYYY-MM-DD pentru TaxPointDate (BT-7) |
| [`BR-RO-DT003`](#br-ro-dt003) | `FATAL` | `BT-9` | Format strict YYYY-MM-DD pentru data scadenței (DueDate) |
| [`BR-RO-DT003-CN`](#br-ro-dt003-cn) | `FATAL` | `BT-9` | Format strict YYYY-MM-DD pentru PaymentDueDate (CreditNote) |
| [`BR-RO-DT004`](#br-ro-dt004) | `FATAL` | `BT-72` | Format strict YYYY-MM-DD pentru data livrării efective |
| [`BR-RO-DT005`](#br-ro-dt005) | `FATAL` | `BT-73, BT-134` | Format strict YYYY-MM-DD pentru StartDate (BT-73, BT-134) |
| [`BR-RO-DT006`](#br-ro-dt006) | `FATAL` | `BT-74, BT-135` | Format strict YYYY-MM-DD pentru EndDate (BT-74, BT-135) |
| [`BR-RO-A020`](#br-ro-a020) | `FATAL` | `BG-1` | Maximum 20 de comentarii în factură (BG-1) |
| [`BR-RO-A050`](#br-ro-a050) | `FATAL` | `BG-24, BG-32` | Maximum 50 de documente justificative (BG-24) sau atribute articol (BG-32) |
| [`BR-RO-A500`](#br-ro-a500) | `FATAL` | `BG-3` | Maximum 500 de referințe la facturi anterioare (BG-3) |
| [`BR-RO-L020`](#br-ro-l020) | `FATAL` | `BT-38, BT-53, BT-67, BT-78` | Maximum 20 de caractere pentru codurile poștale |
| [`BR-RO-L050`](#br-ro-l050) | `FATAL` | `BT-37, BT-52, BT-66, BT-77, BT-160` | Maximum 50 de caractere pentru localități și numele atributelor |
| [`BR-RO-L100`](#br-ro-l100) | `FATAL` | `BT-19, BT-36, BT-41..43, BT-51, BT-56..58, BT-65, BT-76, BT-82, BT-97, BT-104, BT-120, BT-123, BT-133, BT-139, BT-144, BT-153, BT-161..165` | Maximum 100 de caractere pentru numele articolului, contacte, liniile 2/3 de adresă și motive |
| [`BR-RO-L140`](#br-ro-l140) | `FATAL` | `BT-83` | Maximum 140 de caractere pentru avizul de plată (BT-83) |
| [`BR-RO-L150`](#br-ro-l150) | `FATAL` | `BT-35, BT-50, BT-64, BT-75` | Maximum 150 de caractere pentru Linia 1 de adresă |
| [`BR-RO-L200`](#br-ro-l200) | `FATAL` | `BT-1, BT-12..18, BT-25, BT-27, BT-28, BT-44, BT-45, BT-59, BT-62, BT-70, BT-85, BT-88, BT-122, BT-124, BT-125-2, BT-154` | Maximum 200 de caractere pentru denumiri părți, referințe documente și descrierea articolului |
| [`BR-RO-L300`](#br-ro-l300) | `FATAL` | `BT-20, BT-22, BT-127` | Maximum 300 de caractere pentru termeni de plată și note |
| [`BR-RO-L1000`](#br-ro-l1000) | `FATAL` | `BT-33` | Maximum 1000 de caractere pentru informațiile juridice suplimentare ale Vânzătorului |
| [`BR-RO-Z2`](#br-ro-z2) | `FATAL` | `BT-92, BT-93, BT-99, BT-100, BT-106..117, BT-131, BT-136, BT-137, BT-141, BT-142` | Maximum 2 zecimale pentru sumele monetare din factură |
| [`BR-27`](#br-27) | `FATAL` | `BT-146` | Prețul net al articolului (BT-146) nu poate fi negativ |
| [`BR-RO-CUI`](#br-ro-cui) | `WARNING` | `BT-31, BT-32, BT-47, BT-48, BT-63` | Verificarea cifrei de control modulo-11 pentru CUI / CIF |
| [`BR-RO-CNP`](#br-ro-cnp) | `WARNING` | `BT-47, BT-48` | Validarea CNP (13 cifre) sau substituent B2C 0000000000000 |
| [`BR-RO-ONRC`](#br-ro-onrc) | `WARNING` | `BT-30, BT-47` | Formatul numărului din Registrul Comerțului (ONRC) |
| [`BR-RO-IBAN`](#br-ro-iban) | `WARNING` | `BT-84` | Verificarea IBAN românesc (24 caractere, ISO 7064 MOD 97-10) |
| [`XSD-SCHEMA`](#xsd-schema) | `FATAL` | `UBL 2.1 / CII D16B XSD` | Validare schemă oficială W3C XSD 1.1 (UBL 2.1 / CII D16B) și protecție XXE |
| [`PDFA3-ISO19005`](#pdfa3-iso19005) | `FATAL` | `ISO 19005-3 / efactura.xml` | Container hibrid PDF/A-3 (ISO 19005-3) și atașament XML (efactura.xml) |

---

## `BR-RO-001` — Identificatorul specificației CIUS-RO 1.0.1

- **Categorie:** `core`
- **Severitate:** `FATAL`
- **Termeni EN 16931:** `BT-24`
- **Descriere normativă:** Identificatorul specificației (BT-24) trebuie să corespundă exact valorii 'urn:cen.eu:en16931:2017#compliant#urn:efactura.mfinante.ro:CIUS-RO:1.0.1'.
- **Recomandare de remediere:** Setați elementul cbc:CustomizationID (sau ID-ul de profil CII) la URN-ul oficial ANAF CIUS-RO 1.0.1.

## `BR-RO-010` — Caracter numeric obligatoriu în numărul facturii

- **Categorie:** `core`
- **Severitate:** `FATAL`
- **Termeni EN 16931:** `BT-1`
- **Descriere normativă:** Numărul facturii (BT-1) trebuie să includă cel puțin un caracter numeric (0–9).
- **Recomandare de remediere:** Includeți cel puțin o cifră în numărul facturii cbc:ID (ex. 'FCT-2026-0001').

## `BR-RO-020` — Coduri permise pentru tipul facturii (UNTDID 1001)

- **Categorie:** `core`
- **Severitate:** `FATAL`
- **Termeni EN 16931:** `BT-3`
- **Descriere normativă:** Codul tipului facturii (BT-3) trebuie să fie 380 (Factură), 384 (Factură corectată), 389 (Autofactură) sau 751 pentru Invoice, respectiv 381 (Notă de creditare) pentru CreditNote.
- **Recomandare de remediere:** Utilizați unul dintre codurile 380, 384, 389, 751 pentru Invoice sau documentul CreditNote cu codul 381.

## `BR-RO-030` — Monedă străină — contabilizare TVA obligatorie în RON

- **Categorie:** `core`
- **Severitate:** `FATAL`
- **Termeni EN 16931:** `BT-5, BT-6, BT-111`
- **Descriere normativă:** Dacă moneda facturii (BT-5) este alta decât RON, atunci moneda de contabilizare a TVA (BT-6) trebuie să fie RON, împreună cu valoarea TVA în RON (BT-111).
- **Recomandare de remediere:** Adăugați <cbc:TaxCurrencyCode>RON</cbc:TaxCurrencyCode> și al doilea grup <cac:TaxTotal> cu currencyID='RON' calculat la cursul BNR.

## `BR-RO-040` — Codul datei de exigibilitate a TVA (UNTDID 2005 / 2475)

- **Categorie:** `core`
- **Severitate:** `FATAL`
- **Termeni EN 16931:** `BT-8`
- **Descriere normativă:** Dacă este prezent, codul datei de exigibilitate a TVA (BT-8) trebuie să fie 3 (Data emiterii), 35 (Data livrării efective) sau 432 (Data încasării) în UBL (respectiv 5, 29 sau 72 în CII DueDateTypeCode).
- **Recomandare de remediere:** Limitați valoarea cac:InvoicePeriod/cbc:DescriptionCode la '3', '35' sau '432' (sau CII ram:DueDateTypeCode la '5', '29' sau '72').

## `BR-RO-065` — Identificator fiscal sau de TVA al Vânzătorului obligatoriu

- **Categorie:** `party_address`
- **Severitate:** `FATAL`
- **Termeni EN 16931:** `BT-31, BT-32, BT-63`
- **Descriere normativă:** Identificatorul de înregistrare fiscală a Vânzătorului (BT-32), identificatorul de TVA (BT-31) sau cel al reprezentantului fiscal (BT-63) trebuie înscris.
- **Recomandare de remediere:** Completați PartyTaxScheme/CompanyID pentru Vânzător (cu prefix 'RO' pentru plătitori de TVA sau fără 'RO' cu TaxScheme 'FC' pentru neplătitori).

## `BR-RO-080` — Adresa Vânzătorului și a Cumpărătorului (Linia 1) obligatorie

- **Categorie:** `party_address`
- **Severitate:** `FATAL`
- **Termeni EN 16931:** `BT-35, BT-50`
- **Descriere normativă:** Atât Linia 1 din adresa Vânzătorului (BT-35), cât și Linia 1 din adresa Cumpărătorului (BT-50) sunt obligatorii (BR-RO-081, BR-RO-082).
- **Recomandare de remediere:** Completați elementul PostalAddress/cbc:StreetName atât pentru Vânzător, cât și pentru Cumpărător.

## `BR-RO-090` — Localitatea Vânzătorului și a Cumpărătorului obligatorie

- **Categorie:** `party_address`
- **Severitate:** `FATAL`
- **Termeni EN 16931:** `BT-37, BT-52`
- **Descriere normativă:** Atât Localitatea Vânzătorului (BT-37), cât și Localitatea Cumpărătorului (BT-52) trebuie furnizate (BR-RO-091, BR-RO-092).
- **Recomandare de remediere:** Completați elementul PostalAddress/cbc:CityName atât pentru Vânzător, cât și pentru Cumpărător.

## `BR-RO-100` — Codificare obligatorie SECTOR1..SECTOR6 pentru Municipiul București (RO-B)

- **Categorie:** `party_address`
- **Severitate:** `FATAL`
- **Termeni EN 16931:** `BT-37, BT-39, BT-40, BT-52, BT-54, BT-55`
- **Descriere normativă:** Dacă țara este RO și subdiviziunea este RO-B (Municipiul București), localitatea trebuie codificată folosind lista SECTOR-RO: SECTOR1..SECTOR6 (BR-RO-100 Vânzător, BR-RO-101 Cumpărător).
- **Recomandare de remediere:** Înlocuiți 'București' sau 'Sector 1' din cbc:CityName cu valoarea exactă 'SECTOR1'..'SECTOR6'.

## `BR-RO-110` — Subdiviziunea țării pentru România trebuie codificată ISO 3166-2:RO

- **Categorie:** `party_address`
- **Severitate:** `FATAL`
- **Termeni EN 16931:** `BT-39, BT-40, BT-54, BT-55`
- **Descriere normativă:** Dacă țara Vânzătorului sau Cumpărătorului este RO, subdiviziunea țării trebuie codificată conform ISO 3166-2:RO (RO-B, RO-AB..RO-VN; BR-RO-110 Vânzător, BR-RO-111 Cumpărător).
- **Recomandare de remediere:** Setați cbc:CountrySubentity la un cod ISO 3166-2:RO valid (ex. 'RO-B', 'RO-CJ', 'RO-TM', 'RO-IS').

## `BR-RO-120` — Identificatorul legal sau de TVA al Cumpărătorului obligatoriu

- **Categorie:** `party_address`
- **Severitate:** `FATAL`
- **Termeni EN 16931:** `BT-47, BT-48`
- **Descriere normativă:** Identificatorul de înregistrare legală a Cumpărătorului (BT-47) și/sau identificatorul de TVA al Cumpărătorului (BT-48) trebuie înscris (pentru B2C fără CNP se utilizează '0000000000000').
- **Recomandare de remediere:** Completați PartyTaxScheme/CompanyID sau PartyLegalEntity/CompanyID la Cumpărător (CUI, ONRC, CNP sau 0000000000000 pentru B2C).

## `BR-RO-140` — Linia 1 de adresă a reprezentantului fiscal obligatorie

- **Categorie:** `party_address`
- **Severitate:** `FATAL`
- **Termeni EN 16931:** `BG-11, BG-12, BT-64`
- **Descriere normativă:** Dacă Vânzătorul are un reprezentant fiscal (BG-11), adresa acestuia trebuie să conțină Linia 1 (BT-64).
- **Recomandare de remediere:** Completați TaxRepresentativeParty/PostalAddress/cbc:StreetName.

## `BR-RO-150` — Localitatea reprezentantului fiscal obligatorie

- **Categorie:** `party_address`
- **Severitate:** `FATAL`
- **Termeni EN 16931:** `BG-11, BG-12, BT-66`
- **Descriere normativă:** Dacă Vânzătorul are un reprezentant fiscal (BG-11), adresa acestuia trebuie să conțină Localitatea (BT-66).
- **Recomandare de remediere:** Completați TaxRepresentativeParty/PostalAddress/cbc:CityName.

## `BR-RO-160` — Sector București pentru reprezentantul fiscal (RO-B)

- **Categorie:** `party_address`
- **Severitate:** `FATAL`
- **Termeni EN 16931:** `BT-66, BT-68, BT-69`
- **Descriere normativă:** Dacă țara reprezentantului fiscal (BT-69) este RO și subdiviziunea (BT-68) este RO-B, localitatea (BT-66) trebuie să fie SECTOR1..SECTOR6.
- **Recomandare de remediere:** Setați localitatea reprezentantului fiscal la SECTOR1..SECTOR6.

## `BR-RO-170` — Cod județ ISO 3166-2:RO pentru reprezentantul fiscal

- **Categorie:** `party_address`
- **Severitate:** `FATAL`
- **Termeni EN 16931:** `BT-68, BT-69`
- **Descriere normativă:** Dacă țara reprezentantului fiscal (BT-69) este RO, subdiviziunea (BT-68) trebuie codificată conform ISO 3166-2:RO.
- **Recomandare de remediere:** Setați CountrySubentity pentru reprezentantul fiscal la un cod ISO 3166-2:RO valid.

## `BR-RO-180` — Linia 1 a adresei de livrare obligatorie când BG-15 există

- **Categorie:** `party_address`
- **Severitate:** `FATAL`
- **Termeni EN 16931:** `BG-15, BT-75`
- **Descriere normativă:** Dacă grupul Adresa de livrare (BG-15) există, trebuie furnizată Linia 1 a adresei de livrare (BT-75).
- **Recomandare de remediere:** Completați cbc:StreetName în adresa de livrare (BG-15) sau eliminați elementul Address gol.

## `BR-RO-200` — Localitatea de livrare și regula SECTOR1..6 pentru RO-B

- **Categorie:** `party_address`
- **Severitate:** `FATAL`
- **Termeni EN 16931:** `BG-15, BT-77, BT-79, BT-80`
- **Descriere normativă:** Dacă Adresa de livrare (BG-15) există, Localitatea de livrare (BT-77) este obligatorie (BR-RO-201), iar pentru RO + RO-B trebuie codificată SECTOR1..SECTOR6 (BR-RO-202).
- **Recomandare de remediere:** Completați cbc:CityName în adresa de livrare (folosind SECTOR1..SECTOR6 pentru RO-B).

## `BR-RO-210` — Subdiviziunea țării la livrare (BT-79 / ISO 3166-2:RO)

- **Categorie:** `party_address`
- **Severitate:** `FATAL`
- **Termeni EN 16931:** `BG-15, BT-79, BT-80`
- **Descriere normativă:** Dacă Adresa de livrare (BG-15) există, subdiviziunea țării (BT-79) este obligatorie (BR-RO-211), iar pentru țara RO trebuie codificată ISO 3166-2:RO (BR-RO-212).
- **Recomandare de remediere:** Completați cbc:CountrySubentity în adresa de livrare (folosind RO-B sau RO-AB..RO-VN pentru România).

## `BR-RO-DT001` — Format strict YYYY-MM-DD pentru data emiterii (IssueDate)

- **Categorie:** `dates`
- **Severitate:** `FATAL`
- **Termeni EN 16931:** `BT-2`
- **Descriere normativă:** Elementul IssueDate (BT-2) trebuie să aibă exact 10 caractere și să respecte formatul calendaristic YYYY-MM-DD (fără fus orar).
- **Recomandare de remediere:** Formatați cbc:IssueDate exact ca YYYY-MM-DD (ex. '2026-10-15').

## `BR-RO-DT002` — Format strict YYYY-MM-DD pentru TaxPointDate (BT-7)

- **Categorie:** `dates`
- **Severitate:** `FATAL`
- **Termeni EN 16931:** `BT-7`
- **Descriere normativă:** Data de exigibilitate a TVA TaxPointDate (BT-7) trebuie să respecte formatul YYYY-MM-DD (10 caractere).
- **Recomandare de remediere:** Formatați cbc:TaxPointDate ca YYYY-MM-DD.

## `BR-RO-DT003` — Format strict YYYY-MM-DD pentru data scadenței (DueDate)

- **Categorie:** `dates`
- **Severitate:** `FATAL`
- **Termeni EN 16931:** `BT-9`
- **Descriere normativă:** Data scadenței DueDate (BT-9) trebuie să respecte formatul YYYY-MM-DD (10 caractere).
- **Recomandare de remediere:** Formatați cbc:DueDate ca YYYY-MM-DD.

## `BR-RO-DT003-CN` — Format strict YYYY-MM-DD pentru PaymentDueDate (CreditNote)

- **Categorie:** `dates`
- **Severitate:** `FATAL`
- **Termeni EN 16931:** `BT-9`
- **Descriere normativă:** Elementul PaymentDueDate (BT-9 pe nota de creditare) trebuie să respecte formatul YYYY-MM-DD (10 caractere).
- **Recomandare de remediere:** Formatați cbc:PaymentDueDate ca YYYY-MM-DD.

## `BR-RO-DT004` — Format strict YYYY-MM-DD pentru data livrării efective

- **Categorie:** `dates`
- **Severitate:** `FATAL`
- **Termeni EN 16931:** `BT-72`
- **Descriere normativă:** Data livrării efective ActualDeliveryDate (BT-72) trebuie să respecte formatul YYYY-MM-DD (10 caractere).
- **Recomandare de remediere:** Formatați cbc:ActualDeliveryDate ca YYYY-MM-DD.

## `BR-RO-DT005` — Format strict YYYY-MM-DD pentru StartDate (BT-73, BT-134)

- **Categorie:** `dates`
- **Severitate:** `FATAL`
- **Termeni EN 16931:** `BT-73, BT-134`
- **Descriere normativă:** Data de început a perioadei de facturare StartDate (BT-73, BT-134) trebuie să respecte formatul YYYY-MM-DD.
- **Recomandare de remediere:** Formatați cbc:StartDate ca YYYY-MM-DD.

## `BR-RO-DT006` — Format strict YYYY-MM-DD pentru EndDate (BT-74, BT-135)

- **Categorie:** `dates`
- **Severitate:** `FATAL`
- **Termeni EN 16931:** `BT-74, BT-135`
- **Descriere normativă:** Data de sfârșit a perioadei de facturare EndDate (BT-74, BT-135) trebuie să respecte formatul YYYY-MM-DD.
- **Recomandare de remediere:** Formatați cbc:EndDate ca YYYY-MM-DD.

## `BR-RO-A020` — Maximum 20 de comentarii în factură (BG-1)

- **Categorie:** `cardinality`
- **Severitate:** `FATAL`
- **Termeni EN 16931:** `BG-1`
- **Descriere normativă:** O factură poate conține cel mult 20 de apariții ale grupului Comentariu în factură (BG-1 / cbc:Note).
- **Recomandare de remediere:** Comasați elementele cbc:Note la nivel de document astfel încât numărul lor să nu depășească 20.

## `BR-RO-A050` — Maximum 50 de documente justificative (BG-24) sau atribute articol (BG-32)

- **Categorie:** `cardinality`
- **Severitate:** `FATAL`
- **Termeni EN 16931:** `BG-24, BG-32`
- **Descriere normativă:** Sunt permise maximum 50 de apariții ale grupului Documente justificative suplimentare (BG-24, BR-RO-A051) și maximum 50 de Atribute ale articolului (BG-32, BR-RO-A052).
- **Recomandare de remediere:** Limitați numărul de elemente AdditionalDocumentReference și AdditionalItemProperty la cel mult 50.

## `BR-RO-A500` — Maximum 500 de referințe la facturi anterioare (BG-3)

- **Categorie:** `cardinality`
- **Severitate:** `FATAL`
- **Termeni EN 16931:** `BG-3`
- **Descriere normativă:** O factură poate conține maximum 500 de apariții ale grupului Referință la o factură anterioară (BG-3).
- **Recomandare de remediere:** Limitați numărul referințelor la facturi anterioare (BG-3) la cel mult 500.

## `BR-RO-L020` — Maximum 20 de caractere pentru codurile poștale

- **Categorie:** `length`
- **Severitate:** `FATAL`
- **Termeni EN 16931:** `BT-38, BT-53, BT-67, BT-78`
- **Descriere normativă:** Lungimea maximă permisă pentru codul poștal (Vânzător, Cumpărător, Reprezentant fiscal, Livrare) este de 20 de caractere.
- **Recomandare de remediere:** Limitați valoarea cbc:PostalZone la cel mult 20 de caractere.

## `BR-RO-L050` — Maximum 50 de caractere pentru localități și numele atributelor

- **Categorie:** `length`
- **Severitate:** `FATAL`
- **Termeni EN 16931:** `BT-37, BT-52, BT-66, BT-77, BT-160`
- **Descriere normativă:** Numărul maxim permis de caractere pentru localitate (BT-37, BT-52, BT-66, BT-77) și numele atributului articolului (BT-160) este 50.
- **Recomandare de remediere:** Limitați cbc:CityName și numele atributului articolului la cel mult 50 de caractere.

## `BR-RO-L100` — Maximum 100 de caractere pentru numele articolului, contacte, liniile 2/3 de adresă și motive

- **Categorie:** `length`
- **Severitate:** `FATAL`
- **Termeni EN 16931:** `BT-19, BT-36, BT-41..43, BT-51, BT-56..58, BT-65, BT-76, BT-82, BT-97, BT-104, BT-120, BT-123, BT-133, BT-139, BT-144, BT-153, BT-161..165`
- **Descriere normativă:** Numărul maxim permis de caractere pentru numele articolului (BT-153, mutați detaliile în BT-154), liniile 2/3 de adresă, datele de contact și motivele de scutire/deducere este 100.
- **Recomandare de remediere:** Păstrați Item/cbc:Name la <= 100 caractere (mutați detaliile în Item/cbc:Description până la 200 caractere) și limitați câmpurile de contact/motive la <= 100 caractere.

## `BR-RO-L140` — Maximum 140 de caractere pentru avizul de plată (BT-83)

- **Categorie:** `length`
- **Severitate:** `FATAL`
- **Termeni EN 16931:** `BT-83`
- **Descriere normativă:** Numărul maxim permis de caractere pentru Avizul de plată (BT-83 / cbc:PaymentID) este 140.
- **Recomandare de remediere:** Limitați PaymentMeans/cbc:PaymentID la maximum 140 de caractere.

## `BR-RO-L150` — Maximum 150 de caractere pentru Linia 1 de adresă

- **Categorie:** `length`
- **Severitate:** `FATAL`
- **Termeni EN 16931:** `BT-35, BT-50, BT-64, BT-75`
- **Descriere normativă:** Numărul maxim permis de caractere pentru Linia 1 a adresei (BT-35, BT-50, BT-64, BT-75) este 150.
- **Recomandare de remediere:** Împărțiți adresele lungi între StreetName (<= 150), AdditionalStreetName (<= 100) și AddressLine/Line (<= 100).

## `BR-RO-L200` — Maximum 200 de caractere pentru denumiri părți, referințe documente și descrierea articolului

- **Categorie:** `length`
- **Severitate:** `FATAL`
- **Termeni EN 16931:** `BT-1, BT-12..18, BT-25, BT-27, BT-28, BT-44, BT-45, BT-59, BT-62, BT-70, BT-85, BT-88, BT-122, BT-124, BT-125-2, BT-154`
- **Descriere normativă:** Numărul maxim permis de caractere pentru numărul facturii (BT-1), referințele de documente, denumirile părților și descrierea articolului (BT-154) este 200.
- **Recomandare de remediere:** Limitați denumirile părților, referințele, URI-urile și descrierea articolului (BT-154) la maximum 200 de caractere.

## `BR-RO-L300` — Maximum 300 de caractere pentru termeni de plată și note

- **Categorie:** `length`
- **Severitate:** `FATAL`
- **Termeni EN 16931:** `BT-20, BT-22, BT-127`
- **Descriere normativă:** Numărul maxim permis de caractere pentru Termeni de plată (BT-20), Comentariu în factură (BT-22) și Nota liniei (BT-127) este 300.
- **Recomandare de remediere:** Împărțiți comentariile lungi în mai multe elemente cbc:Note (maximum 20 de note, fiecare <= 300 caractere).

## `BR-RO-L1000` — Maximum 1000 de caractere pentru informațiile juridice suplimentare ale Vânzătorului

- **Categorie:** `length`
- **Severitate:** `FATAL`
- **Termeni EN 16931:** `BT-33`
- **Descriere normativă:** Numărul maxim permis de caractere pentru Informații juridice suplimentare despre Vânzător (BT-33 / CompanyLegalForm) este 1000.
- **Recomandare de remediere:** Limitați PartyLegalEntity/cbc:CompanyLegalForm (capital social, mențiuni legale) la maximum 1000 de caractere.

## `BR-RO-Z2` — Maximum 2 zecimale pentru sumele monetare din factură

- **Categorie:** `decimals`
- **Severitate:** `FATAL`
- **Termeni EN 16931:** `BT-92, BT-93, BT-99, BT-100, BT-106..117, BT-131, BT-136, BT-137, BT-141, BT-142`
- **Descriere normativă:** Totalurile facturii, sumele TVA, valoarea netă a liniei și deducerile/taxele suplimentare pot avea cel mult 2 zecimale (ex. '100.00', nu '100.000'; prețul unitar BT-146 poate avea mai mult de 2 zecimale).
- **Recomandare de remediere:** Rotunjiți și formatați sumele monetare la cel mult 2 zecimale în XML.

## `BR-27` — Prețul net al articolului (BT-146) nu poate fi negativ

- **Categorie:** `core`
- **Severitate:** `FATAL`
- **Termeni EN 16931:** `BT-146`
- **Descriere normativă:** Prețul net al articolului (BT-146 / cbc:PriceAmount) trebuie să NU fie negativ.
- **Recomandare de remediere:** Utilizați un preț unitar >= 0; reprezentați reducerile prin AllowanceCharge, cantități negative sau CreditNote.

## `BR-RO-CUI` — Verificarea cifrei de control modulo-11 pentru CUI / CIF

- **Categorie:** `checksum`
- **Severitate:** `WARNING`
- **Termeni EN 16931:** `BT-31, BT-32, BT-47, BT-48, BT-63`
- **Descriere normativă:** Codul de identificare fiscală românesc (2..10 cifre, cu sau fără prefix 'RO') trebuie să respecte algoritmul oficial modulo-11 al Ministerului Finanțelor.
- **Recomandare de remediere:** Verificați codul CUI/CIF în registrul contribuabililor ANAF / mfinante.ro.

## `BR-RO-CNP` — Validarea CNP (13 cifre) sau substituent B2C 0000000000000

- **Categorie:** `checksum`
- **Severitate:** `WARNING`
- **Termeni EN 16931:** `BT-47, BT-48`
- **Descriere normativă:** Un identificator numeric de 13 cifre pentru o parte din România trebuie să fie fie '0000000000000' (substituent B2C cf. OUG 69/2024), fie un CNP valid calendaristic și modulo-11.
- **Recomandare de remediere:** Verificați corectitudinea CNP-ului sau utilizați '0000000000000' pentru clienți persoane fizice (B2C) care nu comunică CNP-ul.

## `BR-RO-ONRC` — Formatul numărului din Registrul Comerțului (ONRC)

- **Categorie:** `checksum`
- **Severitate:** `WARNING`
- **Termeni EN 16931:** `BT-30, BT-47`
- **Descriere normativă:** Numărul de ordine din Registrul Comerțului trebuie să respecte formatul clasic 'J40/1234/2022' sau formatul electronic cf. Legii 265/2022 'J2024001234005'.
- **Recomandare de remediere:** Formatați numărul ONRC ca J<județ>/<nr>/<an> (ex. J40/1234/2022) sau conform Legii 265/2022.

## `BR-RO-IBAN` — Verificarea IBAN românesc (24 caractere, ISO 7064 MOD 97-10)

- **Categorie:** `checksum`
- **Severitate:** `WARNING`
- **Termeni EN 16931:** `BT-84`
- **Descriere normativă:** Conturile bancare IBAN care încep cu 'RO' (BT-84) trebuie să aibă 24 de caractere și să treacă verificarea cifrelor de control ISO/IEC 7064 MOD 97-10.
- **Recomandare de remediere:** Verificați codul IBAN de 24 de caractere înscris în PayeeFinancialAccount/cbc:ID.

## `XSD-SCHEMA` — Validare schemă oficială W3C XSD 1.1 (UBL 2.1 / CII D16B) și protecție XXE

- **Categorie:** `container_xsd`
- **Severitate:** `FATAL`
- **Termeni EN 16931:** `UBL 2.1 / CII D16B XSD`
- **Descriere normativă:** Documentul XML trebuie să fie bine format, fără declarații DOCTYPE/ENTITY și conform cu schemele XSD oficiale OASIS UBL 2.1 sau UN/CEFACT CII D16B.
- **Recomandare de remediere:** Respectați ordinea strictă a elementelor din schema XSD UBL 2.1 / CII D16B și tipurile de date aferente.

## `PDFA3-ISO19005` — Container hibrid PDF/A-3 (ISO 19005-3) și atașament XML (efactura.xml)

- **Categorie:** `container_xsd`
- **Severitate:** `FATAL`
- **Termeni EN 16931:** `ISO 19005-3 / efactura.xml`
- **Descriere normativă:** Facturile PDF hibride trebuie să respecte ISO 19005-3 (fără criptare, /ID în trailer, profil ICC GTS_PDFA1, fonturi încorporate, XMP necomprimat pdfaid:part=3, asociere /AF) și să conțină atașamentul XML (efactura.xml / factur-x.xml).
- **Recomandare de remediere:** Exportați factura vizuală ca PDF/A-3b și atașați efactura.xml atât în /Names/EmbeddedFiles, cât și în Catalog /AF.

