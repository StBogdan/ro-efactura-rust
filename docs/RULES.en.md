# ANAF RO e-Factura (`CIUS-RO 1.0.1`) Validation Rule Reference

> **Language / Limba:** **English** | [Română](RULES.ro.md)

This reference is generated directly from `cius_ro::rules::RULES` and covers all national ANAF `RO_CIUS` (`v1.0.9`) rules, `cius-ro-id` fiscal checksums, W3C XSD 1.1 grammar checks, and ISO 19005-3 (PDF/A-3) hybrid `efactura.xml` checks.

| Code | Severity | Business Terms | Title |
| :--- | :---: | :--- | :--- |
| [`BR-RO-001`](#br-ro-001) | `FATAL` | `BT-24` | CIUS-RO 1.0.1 Specification Identifier |
| [`BR-RO-010`](#br-ro-010) | `FATAL` | `BT-1` | Numeric Digit in Invoice Number |
| [`BR-RO-020`](#br-ro-020) | `FATAL` | `BT-3` | Allowed UNTDID 1001 Document Type Codes |
| [`BR-RO-030`](#br-ro-030) | `FATAL` | `BT-5, BT-6, BT-111` | Foreign Currency Requires RON VAT Accounting Currency |
| [`BR-RO-040`](#br-ro-040) | `FATAL` | `BT-8` | VAT Point Date Code Restriction |
| [`BR-RO-065`](#br-ro-065) | `FATAL` | `BT-31, BT-32, BT-63` | Seller Fiscal or VAT Identifier Required |
| [`BR-RO-080`](#br-ro-080) | `FATAL` | `BT-35, BT-50` | Seller and Buyer Address Line 1 Required |
| [`BR-RO-090`](#br-ro-090) | `FATAL` | `BT-37, BT-52` | Seller and Buyer City Required |
| [`BR-RO-100`](#br-ro-100) | `FATAL` | `BT-37, BT-39, BT-40, BT-52, BT-54, BT-55` | Bucharest (RO-B) City Must Use SECTOR1..SECTOR6 |
| [`BR-RO-110`](#br-ro-110) | `FATAL` | `BT-39, BT-40, BT-54, BT-55` | Romanian County Must Use ISO 3166-2:RO |
| [`BR-RO-120`](#br-ro-120) | `FATAL` | `BT-47, BT-48` | Buyer Legal Registration or VAT Identifier Required |
| [`BR-RO-140`](#br-ro-140) | `FATAL` | `BG-11, BG-12, BT-64` | Tax Representative Street Required |
| [`BR-RO-150`](#br-ro-150) | `FATAL` | `BG-11, BG-12, BT-66` | Tax Representative City Required |
| [`BR-RO-160`](#br-ro-160) | `FATAL` | `BT-66, BT-68, BT-69` | Tax Representative Bucharest Sector |
| [`BR-RO-170`](#br-ro-170) | `FATAL` | `BT-68, BT-69` | Tax Representative Romanian County Code |
| [`BR-RO-180`](#br-ro-180) | `FATAL` | `BG-15, BT-75` | Delivery Address Line 1 Required When BG-15 Exists |
| [`BR-RO-200`](#br-ro-200) | `FATAL` | `BG-15, BT-77, BT-79, BT-80` | Delivery City & Bucharest Sector Rules |
| [`BR-RO-210`](#br-ro-210) | `FATAL` | `BG-15, BT-79, BT-80` | Delivery Country Subdivision & ISO 3166-2:RO |
| [`BR-RO-DT001`](#br-ro-dt001) | `FATAL` | `BT-2` | IssueDate Strict YYYY-MM-DD Format |
| [`BR-RO-DT002`](#br-ro-dt002) | `FATAL` | `BT-7` | TaxPointDate Strict YYYY-MM-DD Format |
| [`BR-RO-DT003`](#br-ro-dt003) | `FATAL` | `BT-9` | DueDate Strict YYYY-MM-DD Format |
| [`BR-RO-DT003-CN`](#br-ro-dt003-cn) | `FATAL` | `BT-9` | PaymentDueDate Strict YYYY-MM-DD Format |
| [`BR-RO-DT004`](#br-ro-dt004) | `FATAL` | `BT-72` | ActualDeliveryDate Strict YYYY-MM-DD Format |
| [`BR-RO-DT005`](#br-ro-dt005) | `FATAL` | `BT-73, BT-134` | Period StartDate Strict YYYY-MM-DD Format |
| [`BR-RO-DT006`](#br-ro-dt006) | `FATAL` | `BT-74, BT-135` | Period EndDate Strict YYYY-MM-DD Format |
| [`BR-RO-A020`](#br-ro-a020) | `FATAL` | `BG-1` | Maximum 20 Invoice Notes |
| [`BR-RO-A050`](#br-ro-a050) | `FATAL` | `BG-24, BG-32` | Maximum 50 Supporting Documents or Item Attributes |
| [`BR-RO-A500`](#br-ro-a500) | `FATAL` | `BG-3` | Maximum 500 Preceding Invoice References |
| [`BR-RO-L020`](#br-ro-l020) | `FATAL` | `BT-38, BT-53, BT-67, BT-78` | Maximum 20 Characters for Postal Codes |
| [`BR-RO-L050`](#br-ro-l050) | `FATAL` | `BT-37, BT-52, BT-66, BT-77, BT-160` | Maximum 50 Characters for Cities & Item Attribute Names |
| [`BR-RO-L100`](#br-ro-l100) | `FATAL` | `BT-19, BT-36, BT-41..43, BT-51, BT-56..58, BT-65, BT-76, BT-82, BT-97, BT-104, BT-120, BT-123, BT-133, BT-139, BT-144, BT-153, BT-161..165` | Maximum 100 Characters for Item Names, Contacts, Address Lines 2/3 & Reasons |
| [`BR-RO-L140`](#br-ro-l140) | `FATAL` | `BT-83` | Maximum 140 Characters for Remittance Information |
| [`BR-RO-L150`](#br-ro-l150) | `FATAL` | `BT-35, BT-50, BT-64, BT-75` | Maximum 150 Characters for Street Address Line 1 |
| [`BR-RO-L200`](#br-ro-l200) | `FATAL` | `BT-1, BT-12..18, BT-25, BT-27, BT-28, BT-44, BT-45, BT-59, BT-62, BT-70, BT-85, BT-88, BT-122, BT-124, BT-125-2, BT-154` | Maximum 200 Characters for Party Names, Document IDs & Item Descriptions |
| [`BR-RO-L300`](#br-ro-l300) | `FATAL` | `BT-20, BT-22, BT-127` | Maximum 300 Characters for Payment Terms & Notes |
| [`BR-RO-L1000`](#br-ro-l1000) | `FATAL` | `BT-33` | Maximum 1000 Characters for Seller Additional Legal Information |
| [`BR-RO-Z2`](#br-ro-z2) | `FATAL` | `BT-92, BT-93, BT-99, BT-100, BT-106..117, BT-131, BT-136, BT-137, BT-141, BT-142` | Maximum 2 Fractional Decimals on Monetary Amounts |
| [`BR-27`](#br-27) | `FATAL` | `BT-146` | Item Net Price Must Not Be Negative |
| [`BR-RO-CUI`](#br-ro-cui) | `WARNING` | `BT-31, BT-32, BT-47, BT-48, BT-63` | Romanian CUI / CIF Modulo-11 Control Digit |
| [`BR-RO-CNP`](#br-ro-cnp) | `WARNING` | `BT-47, BT-48` | Romanian 13-Digit CNP Checksum & Calendar Validity |
| [`BR-RO-ONRC`](#br-ro-onrc) | `WARNING` | `BT-30, BT-47` | Romanian ONRC Trade Register Format |
| [`BR-RO-IBAN`](#br-ro-iban) | `WARNING` | `BT-84` | Romanian IBAN ISO 7064 MOD 97-10 Checksum |
| [`XSD-SCHEMA`](#xsd-schema) | `FATAL` | `UBL 2.1 / CII D16B XSD` | Official W3C XML Schema (XSD 1.1) Grammar & XXE Protection |
| [`PDFA3-ISO19005`](#pdfa3-iso19005) | `FATAL` | `ISO 19005-3 / efactura.xml` | Hybrid PDF/A-3 Container & Associated XML Attachment |

---

## `BR-RO-001` — CIUS-RO 1.0.1 Specification Identifier

- **Category:** `core`
- **Severity:** `FATAL`
- **Business Terms:** `BT-24`
- **Specification:** Specification identifier (BT-24) must equal 'urn:cen.eu:en16931:2017#compliant#urn:efactura.mfinante.ro:CIUS-RO:1.0.1'.
- **Remediation:** Set cbc:CustomizationID (or CII GuidelineSpecifiedDocumentContextParameter/ID) to the exact ANAF CIUS-RO 1.0.1 URN.

## `BR-RO-010` — Numeric Digit in Invoice Number

- **Category:** `core`
- **Severity:** `FATAL`
- **Business Terms:** `BT-1`
- **Specification:** Invoice number (BT-1) must include at least one numeric character (0-9).
- **Remediation:** Include a numeric sequence in cbc:ID (for example 'FCT-2026-0001').

## `BR-RO-020` — Allowed UNTDID 1001 Document Type Codes

- **Category:** `core`
- **Severity:** `FATAL`
- **Business Terms:** `BT-3`
- **Specification:** Invoice type code (BT-3) must be 380 (Invoice), 384 (Corrected invoice), 389 (Self-invoice), or 751 (Accounting info) for Invoice, and 381 for CreditNote.
- **Remediation:** Use cbc:InvoiceTypeCode in {380, 384, 389, 751} or <CreditNote> with cbc:CreditNoteTypeCode = 381.

## `BR-RO-030` — Foreign Currency Requires RON VAT Accounting Currency

- **Category:** `core`
- **Severity:** `FATAL`
- **Business Terms:** `BT-5, BT-6, BT-111`
- **Specification:** If Invoice currency code (BT-5) is other than RON, then VAT accounting currency code (BT-6) must be RON and total VAT in RON (BT-111) must be provided.
- **Remediation:** Add <cbc:TaxCurrencyCode>RON</cbc:TaxCurrencyCode> and a second <cac:TaxTotal> with currencyID='RON' converted at the BNR rate.

## `BR-RO-040` — VAT Point Date Code Restriction

- **Category:** `core`
- **Severity:** `FATAL`
- **Business Terms:** `BT-8`
- **Specification:** When present, Value added tax point date code (BT-8) must be one of 3 (Invoice issue date), 35 (Actual delivery date), or 432 (Paid to date) in UBL (or 5, 29, 72 in CII DueDateTypeCode).
- **Remediation:** Restrict cac:InvoicePeriod/cbc:DescriptionCode to '3', '35', or '432' (or CII ram:DueDateTypeCode to '5', '29', or '72').

## `BR-RO-065` — Seller Fiscal or VAT Identifier Required

- **Category:** `party_address`
- **Severity:** `FATAL`
- **Business Terms:** `BT-31, BT-32, BT-63`
- **Specification:** Seller tax registration identifier (BT-32), Seller VAT identifier (BT-31), or Seller tax representative VAT identifier (BT-63) must be present.
- **Remediation:** Populate AccountingSupplierParty/Party/PartyTaxScheme/CompanyID (with 'RO' prefix for VAT payers or TaxScheme 'FC' for non-VAT CIF).

## `BR-RO-080` — Seller and Buyer Address Line 1 Required

- **Category:** `party_address`
- **Severity:** `FATAL`
- **Business Terms:** `BT-35, BT-50`
- **Specification:** Both Seller address line 1 (BT-35) and Buyer address line 1 (BT-50) must be provided (sub-rules BR-RO-081, BR-RO-082).
- **Remediation:** Ensure PostalAddress/cbc:StreetName is non-empty for both Seller and Buyer.

## `BR-RO-090` — Seller and Buyer City Required

- **Category:** `party_address`
- **Severity:** `FATAL`
- **Business Terms:** `BT-37, BT-52`
- **Specification:** Both Seller city (BT-37) and Buyer city (BT-52) must be provided (sub-rules BR-RO-091, BR-RO-092).
- **Remediation:** Ensure PostalAddress/cbc:CityName is non-empty for both Seller and Buyer.

## `BR-RO-100` — Bucharest (RO-B) City Must Use SECTOR1..SECTOR6

- **Category:** `party_address`
- **Severity:** `FATAL`
- **Business Terms:** `BT-37, BT-39, BT-40, BT-52, BT-54, BT-55`
- **Specification:** When Country is RO and CountrySubentity is RO-B (Bucharest), CityName must be one of SECTOR1, SECTOR2, SECTOR3, SECTOR4, SECTOR5, SECTOR6 (BR-RO-100 Seller, BR-RO-101 Buyer).
- **Remediation:** Replace free-text 'București' or 'Sector 1' in cbc:CityName with exact uppercase 'SECTOR1'..'SECTOR6'.

## `BR-RO-110` — Romanian County Must Use ISO 3166-2:RO

- **Category:** `party_address`
- **Severity:** `FATAL`
- **Business Terms:** `BT-39, BT-40, BT-54, BT-55`
- **Specification:** When Seller or Buyer country code is RO, CountrySubentity must be coded using ISO 3166-2:RO (RO-B, RO-AB..RO-VN; BR-RO-110 Seller, BR-RO-111 Buyer).
- **Remediation:** Set cbc:CountrySubentity to a valid ISO 3166-2:RO code such as 'RO-B', 'RO-CJ', 'RO-TM', or 'RO-IS'.

## `BR-RO-120` — Buyer Legal Registration or VAT Identifier Required

- **Category:** `party_address`
- **Severity:** `FATAL`
- **Business Terms:** `BT-47, BT-48`
- **Specification:** Buyer legal registration identifier (BT-47) and/or Buyer VAT identifier (BT-48) must be present (for B2C consumers without disclosed CNP, use '0000000000000').
- **Remediation:** Provide Buyer PartyTaxScheme/CompanyID or PartyLegalEntity/CompanyID (CUI, ONRC, CNP, or 0000000000000 for B2C).

## `BR-RO-140` — Tax Representative Street Required

- **Category:** `party_address`
- **Severity:** `FATAL`
- **Business Terms:** `BG-11, BG-12, BT-64`
- **Specification:** If Seller has a Tax Representative party (BG-11), Tax representative address line 1 (BT-64) must be provided.
- **Remediation:** Populate TaxRepresentativeParty/PostalAddress/cbc:StreetName.

## `BR-RO-150` — Tax Representative City Required

- **Category:** `party_address`
- **Severity:** `FATAL`
- **Business Terms:** `BG-11, BG-12, BT-66`
- **Specification:** If Seller has a Tax Representative party (BG-11), Tax representative city (BT-66) must be provided.
- **Remediation:** Populate TaxRepresentativeParty/PostalAddress/cbc:CityName.

## `BR-RO-160` — Tax Representative Bucharest Sector

- **Category:** `party_address`
- **Severity:** `FATAL`
- **Business Terms:** `BT-66, BT-68, BT-69`
- **Specification:** When Tax representative country (BT-69) is RO and subdivision (BT-68) is RO-B, city (BT-66) must be SECTOR1..SECTOR6.
- **Remediation:** Set TaxRepresentativeParty/PostalAddress/cbc:CityName to SECTOR1..SECTOR6.

## `BR-RO-170` — Tax Representative Romanian County Code

- **Category:** `party_address`
- **Severity:** `FATAL`
- **Business Terms:** `BT-68, BT-69`
- **Specification:** When Tax representative country (BT-69) is RO, subdivision (BT-68) must be a valid ISO 3166-2:RO code.
- **Remediation:** Set TaxRepresentativeParty/PostalAddress/cbc:CountrySubentity to RO-B or RO-AB..RO-VN.

## `BR-RO-180` — Delivery Address Line 1 Required When BG-15 Exists

- **Category:** `party_address`
- **Severity:** `FATAL`
- **Business Terms:** `BG-15, BT-75`
- **Specification:** If Deliver to address (BG-15) is present, Deliver to address line 1 (BT-75) must be provided.
- **Remediation:** Populate Delivery/DeliveryLocation/Address/cbc:StreetName when Address is included.

## `BR-RO-200` — Delivery City & Bucharest Sector Rules

- **Category:** `party_address`
- **Severity:** `FATAL`
- **Business Terms:** `BG-15, BT-77, BT-79, BT-80`
- **Specification:** If Deliver to address (BG-15) exists, Deliver to city (BT-77) must be non-empty (BR-RO-201), and if country is RO with RO-B, must be SECTOR1..SECTOR6 (BR-RO-202).
- **Remediation:** Provide Delivery Address cbc:CityName (using SECTOR1..SECTOR6 when CountrySubentity is RO-B).

## `BR-RO-210` — Delivery Country Subdivision & ISO 3166-2:RO

- **Category:** `party_address`
- **Severity:** `FATAL`
- **Business Terms:** `BG-15, BT-79, BT-80`
- **Specification:** If Deliver to address (BG-15) exists, CountrySubentity (BT-79) must be provided (BR-RO-211) and, when country is RO, must use ISO 3166-2:RO (BR-RO-212).
- **Remediation:** Populate Delivery Address cbc:CountrySubentity (using RO-B or RO-AB..RO-VN when country is RO).

## `BR-RO-DT001` — IssueDate Strict YYYY-MM-DD Format

- **Category:** `dates`
- **Severity:** `FATAL`
- **Business Terms:** `BT-2`
- **Specification:** IssueDate (BT-2) must be exactly 10 characters long and a valid calendar date formatted YYYY-MM-DD (no timezone suffix).
- **Remediation:** Format cbc:IssueDate as YYYY-MM-DD (e.g. '2026-10-15').

## `BR-RO-DT002` — TaxPointDate Strict YYYY-MM-DD Format

- **Category:** `dates`
- **Severity:** `FATAL`
- **Business Terms:** `BT-7`
- **Specification:** TaxPointDate (BT-7) must be exactly 10 characters long and formatted YYYY-MM-DD.
- **Remediation:** Format cbc:TaxPointDate as YYYY-MM-DD.

## `BR-RO-DT003` — DueDate Strict YYYY-MM-DD Format

- **Category:** `dates`
- **Severity:** `FATAL`
- **Business Terms:** `BT-9`
- **Specification:** DueDate (BT-9) must be exactly 10 characters long and formatted YYYY-MM-DD.
- **Remediation:** Format cbc:DueDate as YYYY-MM-DD.

## `BR-RO-DT003-CN` — PaymentDueDate Strict YYYY-MM-DD Format

- **Category:** `dates`
- **Severity:** `FATAL`
- **Business Terms:** `BT-9`
- **Specification:** PaymentDueDate (BT-9 on CreditNote) must be exactly 10 characters long and formatted YYYY-MM-DD.
- **Remediation:** Format cbc:PaymentDueDate as YYYY-MM-DD.

## `BR-RO-DT004` — ActualDeliveryDate Strict YYYY-MM-DD Format

- **Category:** `dates`
- **Severity:** `FATAL`
- **Business Terms:** `BT-72`
- **Specification:** ActualDeliveryDate (BT-72) must be exactly 10 characters long and formatted YYYY-MM-DD.
- **Remediation:** Format cbc:ActualDeliveryDate as YYYY-MM-DD.

## `BR-RO-DT005` — Period StartDate Strict YYYY-MM-DD Format

- **Category:** `dates`
- **Severity:** `FATAL`
- **Business Terms:** `BT-73, BT-134`
- **Specification:** Invoice or line period StartDate (BT-73, BT-134) must be formatted YYYY-MM-DD (10 characters).
- **Remediation:** Format cbc:StartDate as YYYY-MM-DD.

## `BR-RO-DT006` — Period EndDate Strict YYYY-MM-DD Format

- **Category:** `dates`
- **Severity:** `FATAL`
- **Business Terms:** `BT-74, BT-135`
- **Specification:** Invoice or line period EndDate (BT-74, BT-135) must be formatted YYYY-MM-DD (10 characters).
- **Remediation:** Format cbc:EndDate as YYYY-MM-DD.

## `BR-RO-A020` — Maximum 20 Invoice Notes

- **Category:** `cardinality`
- **Severity:** `FATAL`
- **Business Terms:** `BG-1`
- **Specification:** An invoice may contain at most 20 occurrences of Invoice note (BG-1 / cbc:Note).
- **Remediation:** Consolidate header cbc:Note elements so no more than 20 appear on the document.

## `BR-RO-A050` — Maximum 50 Supporting Documents or Item Attributes

- **Category:** `cardinality`
- **Severity:** `FATAL`
- **Business Terms:** `BG-24, BG-32`
- **Specification:** At most 50 occurrences of Additional supporting documents (BG-24, BR-RO-A051) and at most 50 Item attributes (BG-32, BR-RO-A052) are allowed.
- **Remediation:** Keep AdditionalDocumentReference <= 50 and Item/AdditionalItemProperty <= 50.

## `BR-RO-A500` — Maximum 500 Preceding Invoice References

- **Category:** `cardinality`
- **Severity:** `FATAL`
- **Business Terms:** `BG-3`
- **Specification:** An invoice may contain at most 500 occurrences of Preceding invoice reference (BG-3).
- **Remediation:** Ensure BillingReference / InvoiceDocumentReference occurrences do not exceed 500.

## `BR-RO-L020` — Maximum 20 Characters for Postal Codes

- **Category:** `length`
- **Severity:** `FATAL`
- **Business Terms:** `BT-38, BT-53, BT-67, BT-78`
- **Specification:** Normalized character length of Seller, Buyer, Tax Representative, and Delivery post codes must not exceed 20 characters.
- **Remediation:** Trim cbc:PostalZone values to at most 20 characters.

## `BR-RO-L050` — Maximum 50 Characters for Cities & Item Attribute Names

- **Category:** `length`
- **Severity:** `FATAL`
- **Business Terms:** `BT-37, BT-52, BT-66, BT-77, BT-160`
- **Specification:** Normalized character length of city names (BT-37, BT-52, BT-66, BT-77) and item attribute names (BT-160) must not exceed 50 characters.
- **Remediation:** Shorten cbc:CityName and AdditionalItemProperty/cbc:Name to <= 50 characters.

## `BR-RO-L100` — Maximum 100 Characters for Item Names, Contacts, Address Lines 2/3 & Reasons

- **Category:** `length`
- **Severity:** `FATAL`
- **Business Terms:** `BT-19, BT-36, BT-41..43, BT-51, BT-56..58, BT-65, BT-76, BT-82, BT-97, BT-104, BT-120, BT-123, BT-133, BT-139, BT-144, BT-153, BT-161..165`
- **Specification:** Normalized character length of Item name (BT-153), address lines 2/3, contact fields, accounting cost, VAT exemption reason, and allowance/charge reasons must not exceed 100 characters.
- **Remediation:** Keep Item/cbc:Name <= 100 chars (put longer text in Item/cbc:Description up to 200 chars) and trim contact/reason fields to <= 100 chars.

## `BR-RO-L140` — Maximum 140 Characters for Remittance Information

- **Category:** `length`
- **Severity:** `FATAL`
- **Business Terms:** `BT-83`
- **Specification:** Normalized character length of Remittance information (BT-83 / PaymentMeans/cbc:PaymentID) must not exceed 140 characters.
- **Remediation:** Shorten PaymentMeans/cbc:PaymentID to at most 140 characters (SEPA unstructured remittance cap).

## `BR-RO-L150` — Maximum 150 Characters for Street Address Line 1

- **Category:** `length`
- **Severity:** `FATAL`
- **Business Terms:** `BT-35, BT-50, BT-64, BT-75`
- **Specification:** Normalized character length of StreetName (BT-35, BT-50, BT-64, BT-75) must not exceed 150 characters.
- **Remediation:** Split long addresses across StreetName (<= 150), AdditionalStreetName (<= 100), and AddressLine/Line (<= 100).

## `BR-RO-L200` — Maximum 200 Characters for Party Names, Document IDs & Item Descriptions

- **Category:** `length`
- **Severity:** `FATAL`
- **Business Terms:** `BT-1, BT-12..18, BT-25, BT-27, BT-28, BT-44, BT-45, BT-59, BT-62, BT-70, BT-85, BT-88, BT-122, BT-124, BT-125-2, BT-154`
- **Specification:** Normalized character length of Invoice number (BT-1), document references, party legal/trading names, account/card names, and Item description (BT-154) must not exceed 200 characters.
- **Remediation:** Ensure party names, document IDs, URIs, filenames, and Item/cbc:Description are <= 200 characters.

## `BR-RO-L300` — Maximum 300 Characters for Payment Terms & Notes

- **Category:** `length`
- **Severity:** `FATAL`
- **Business Terms:** `BT-20, BT-22, BT-127`
- **Specification:** Normalized character length of Payment terms (BT-20), Invoice note (BT-22), and Invoice line note (BT-127) must not exceed 300 characters.
- **Remediation:** Split long remarks into multiple cbc:Note elements (up to 20 notes, each <= 300 chars).

## `BR-RO-L1000` — Maximum 1000 Characters for Seller Additional Legal Information

- **Category:** `length`
- **Severity:** `FATAL`
- **Business Terms:** `BT-33`
- **Specification:** Normalized character length of Seller additional legal information (BT-33 / CompanyLegalForm) must not exceed 1000 characters.
- **Remediation:** Trim PartyLegalEntity/cbc:CompanyLegalForm (share capital, social headquarters) to <= 1000 characters.

## `BR-RO-Z2` — Maximum 2 Fractional Decimals on Monetary Amounts

- **Category:** `decimals`
- **Severity:** `FATAL`
- **Business Terms:** `BT-92, BT-93, BT-99, BT-100, BT-106..117, BT-131, BT-136, BT-137, BT-141, BT-142`
- **Specification:** Document totals, VAT totals/subtotals, line net amounts, and allowance/charge amounts/bases must have at most 2 digits after the decimal point (e.g. '100.00', not '100.000'; unit prices BT-146 may still use >2 decimals).
- **Remediation:** Round and format monetary amount elements to at most 2 decimal places (0, 1, or 2 fractional digits).

## `BR-27` — Item Net Price Must Not Be Negative

- **Category:** `core`
- **Severity:** `FATAL`
- **Business Terms:** `BT-146`
- **Specification:** The Item net price (BT-146 / cbc:PriceAmount) shall NOT be negative.
- **Remediation:** Use a non-negative unit price; represent discounts via AllowanceCharge or negative quantities / CreditNote.

## `BR-RO-CUI` — Romanian CUI / CIF Modulo-11 Control Digit

- **Category:** `checksum`
- **Severity:** `WARNING`
- **Business Terms:** `BT-31, BT-32, BT-47, BT-48, BT-63`
- **Specification:** Romanian fiscal identifier (2..10 digits, with or without 'RO' prefix) should satisfy the Ministry of Finance modulo-11 check-digit algorithm.
- **Remediation:** Verify the party's CUI/CIF against the ANAF / mfinante.ro taxpayer register.

## `BR-RO-CNP` — Romanian 13-Digit CNP Checksum & Calendar Validity

- **Category:** `checksum`
- **Severity:** `WARNING`
- **Business Terms:** `BT-47, BT-48`
- **Specification:** A 13-digit numeric identifier on a Romanian party must either equal '0000000000000' (ANAF OUG 69/2024 B2C placeholder) or pass 13-digit CNP date, county, and modulo-11 verification.
- **Remediation:** Check the 13-digit CNP for typos or use '0000000000000' when the natural-person buyer does not disclose their CNP.

## `BR-RO-ONRC` — Romanian ONRC Trade Register Format

- **Category:** `checksum`
- **Severity:** `WARNING`
- **Business Terms:** `BT-30, BT-47`
- **Specification:** Romanian Trade Register numbers (J/F/C...) should follow either classic 'J40/1234/2022' or Law 265/2022 'J2024001234005' structure.
- **Remediation:** Format ONRC legal registration IDs as J<county>/<seq>/<year> or J<YYYY><NNNNNN><JJ><C>.

## `BR-RO-IBAN` — Romanian IBAN ISO 7064 MOD 97-10 Checksum

- **Category:** `checksum`
- **Severity:** `WARNING`
- **Business Terms:** `BT-84`
- **Specification:** Payee bank account identifiers starting with 'RO' (BT-84) should be 24 characters long and pass ISO/IEC 7064 MOD 97-10 check-digit verification.
- **Remediation:** Check the 24-character Romanian IBAN in PayeeFinancialAccount/cbc:ID for missing characters or mistyped digits.

## `XSD-SCHEMA` — Official W3C XML Schema (XSD 1.1) Grammar & XXE Protection

- **Category:** `container_xsd`
- **Severity:** `FATAL`
- **Business Terms:** `UBL 2.1 / CII D16B XSD`
- **Specification:** XML invoice must be well-formed, free of DOCTYPE/ENTITY declarations, and conformant to OASIS UBL 2.1 Invoice/CreditNote or UN/CEFACT CII D16B XSD.
- **Remediation:** Ensure UBL/CII elements appear in strict XSD sequence order with valid namespaces and datatypes.

## `PDFA3-ISO19005` — Hybrid PDF/A-3 Container & Associated XML Attachment

- **Category:** `container_xsd`
- **Severity:** `FATAL`
- **Business Terms:** `ISO 19005-3 / efactura.xml`
- **Specification:** Hybrid PDF invoices must conform to ISO 19005-3 (unencrypted, trailer /ID, GTS_PDFA1 ICC profile, embedded fonts, unfiltered XMP pdfaid:part=3, /AF association) and carry efactura.xml / factur-x.xml.
- **Remediation:** Export the visual invoice as PDF/A-3b and attach efactura.xml in both /Names/EmbeddedFiles and Catalog /AF.

