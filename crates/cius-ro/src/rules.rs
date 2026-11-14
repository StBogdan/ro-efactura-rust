//! Normative rule catalog for ANAF `RO e-Factura` (`CIUS-RO 1.0.1` / `v1.0.9`), `cius-ro-id` checksums,
//! W3C XSD 1.1, and ISO 19005-3 (PDF/A-3) hybrid containers — with full English & Romanian metadata.

use core::fmt::Write as _;

use crate::types::{Locale, RuleCategory, Severity};

/// Static metadata entry describing a validation rule in both English and Romanian.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct RuleDescriptor {
    /// Canonical rule code (e.g. `"BR-RO-100"`).
    pub code: &'static str,
    /// Functional group (`Core`, `PartyAddress`, `Dates`, `Cardinality`, `Length`, `Decimals`, `Checksum`, `ContainerXsd`).
    pub category: RuleCategory,
    /// Default severity level.
    pub severity: Severity,
    /// Affected EN 16931 Business Terms / Groups.
    pub business_terms: &'static str,
    /// Short title in English.
    pub title_en: &'static str,
    /// Short title in Romanian.
    pub title_ro: &'static str,
    /// Normative description in English.
    pub description_en: &'static str,
    /// Normative description in Romanian.
    pub description_ro: &'static str,
    /// Actionable remediation guidance in English.
    pub remediation_en: &'static str,
    /// Actionable remediation guidance in Romanian.
    pub remediation_ro: &'static str,
}

impl RuleDescriptor {
    /// Returns the localized title for `locale`.
    #[must_use]
    pub const fn title(&self, locale: Locale) -> &'static str {
        match locale {
            Locale::En => self.title_en,
            Locale::Ro => self.title_ro,
        }
    }

    /// Returns the localized description for `locale`.
    #[must_use]
    pub const fn description(&self, locale: Locale) -> &'static str {
        match locale {
            Locale::En => self.description_en,
            Locale::Ro => self.description_ro,
        }
    }

    /// Returns the localized remediation hint for `locale`.
    #[must_use]
    pub const fn remediation(&self, locale: Locale) -> &'static str {
        match locale {
            Locale::En => self.remediation_en,
            Locale::Ro => self.remediation_ro,
        }
    }

    /// Projects this descriptor into a single-locale [`RuleView`] suitable for JSON/WASM output.
    #[must_use]
    pub const fn view(&self, locale: Locale) -> RuleView<'static> {
        RuleView {
            code: self.code,
            category: self.category,
            business_terms: self.business_terms,
            severity: self.severity,
            title: self.title(locale),
            description: self.description(locale),
            remediation: self.remediation(locale),
        }
    }
}

/// Single-locale view of a [`RuleDescriptor`] used by CLI and WebAssembly JSON responses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct RuleView<'a> {
    /// Canonical rule code (e.g. `"BR-RO-100"`).
    pub code: &'a str,
    /// Functional rule category.
    pub category: RuleCategory,
    /// Affected EN 16931 Business Terms / Groups.
    pub business_terms: &'a str,
    /// Default severity level.
    pub severity: Severity,
    /// Localized short title.
    pub title: &'a str,
    /// Localized normative description.
    pub description: &'a str,
    /// Localized actionable remediation guidance.
    pub remediation: &'a str,
}

/// Complete catalog of ANAF `CIUS-RO 1.0.1` (`ROeFactura v1.0.9`), fiscal checksum, XSD, and PDF/A-3 rules.
pub const RULES: &[RuleDescriptor] = &[
    RuleDescriptor {
        code: "BR-RO-001",
        category: RuleCategory::Core,
        severity: Severity::Fatal,
        business_terms: "BT-24",
        title_en: "CIUS-RO 1.0.1 Specification Identifier",
        title_ro: "Identificatorul specificației CIUS-RO 1.0.1",
        description_en: "Specification identifier (BT-24) must equal 'urn:cen.eu:en16931:2017#compliant#urn:efactura.mfinante.ro:CIUS-RO:1.0.1'.",
        description_ro: "Identificatorul specificației (BT-24) trebuie să corespundă exact valorii 'urn:cen.eu:en16931:2017#compliant#urn:efactura.mfinante.ro:CIUS-RO:1.0.1'.",
        remediation_en: "Set cbc:CustomizationID (or CII GuidelineSpecifiedDocumentContextParameter/ID) to the exact ANAF CIUS-RO 1.0.1 URN.",
        remediation_ro: "Setați elementul cbc:CustomizationID (sau ID-ul de profil CII) la URN-ul oficial ANAF CIUS-RO 1.0.1.",
    },
    RuleDescriptor {
        code: "BR-RO-010",
        category: RuleCategory::Core,
        severity: Severity::Fatal,
        business_terms: "BT-1",
        title_en: "Numeric Digit in Invoice Number",
        title_ro: "Caracter numeric obligatoriu în numărul facturii",
        description_en: "Invoice number (BT-1) must include at least one numeric character (0-9).",
        description_ro: "Numărul facturii (BT-1) trebuie să includă cel puțin un caracter numeric (0–9).",
        remediation_en: "Include a numeric sequence in cbc:ID (for example 'FCT-2026-0001').",
        remediation_ro: "Includeți cel puțin o cifră în numărul facturii cbc:ID (ex. 'FCT-2026-0001').",
    },
    RuleDescriptor {
        code: "BR-RO-020",
        category: RuleCategory::Core,
        severity: Severity::Fatal,
        business_terms: "BT-3",
        title_en: "Allowed UNTDID 1001 Document Type Codes",
        title_ro: "Coduri permise pentru tipul facturii (UNTDID 1001)",
        description_en: "Invoice type code (BT-3) must be 380 (Invoice), 384 (Corrected invoice), 389 (Self-invoice), or 751 (Accounting info) for Invoice, and 381 for CreditNote.",
        description_ro: "Codul tipului facturii (BT-3) trebuie să fie 380 (Factură), 384 (Factură corectată), 389 (Autofactură) sau 751 pentru Invoice, respectiv 381 (Notă de creditare) pentru CreditNote.",
        remediation_en: "Use cbc:InvoiceTypeCode in {380, 384, 389, 751} or <CreditNote> with cbc:CreditNoteTypeCode = 381.",
        remediation_ro: "Utilizați unul dintre codurile 380, 384, 389, 751 pentru Invoice sau documentul CreditNote cu codul 381.",
    },
    RuleDescriptor {
        code: "BR-RO-030",
        category: RuleCategory::Core,
        severity: Severity::Fatal,
        business_terms: "BT-5, BT-6, BT-111",
        title_en: "Foreign Currency Requires RON VAT Accounting Currency",
        title_ro: "Monedă străină — contabilizare TVA obligatorie în RON",
        description_en: "If Invoice currency code (BT-5) is other than RON, then VAT accounting currency code (BT-6) must be RON and total VAT in RON (BT-111) must be provided.",
        description_ro: "Dacă moneda facturii (BT-5) este alta decât RON, atunci moneda de contabilizare a TVA (BT-6) trebuie să fie RON, împreună cu valoarea TVA în RON (BT-111).",
        remediation_en: "Add <cbc:TaxCurrencyCode>RON</cbc:TaxCurrencyCode> and a second <cac:TaxTotal> with currencyID='RON' converted at the BNR rate.",
        remediation_ro: "Adăugați <cbc:TaxCurrencyCode>RON</cbc:TaxCurrencyCode> și al doilea grup <cac:TaxTotal> cu currencyID='RON' calculat la cursul BNR.",
    },
    RuleDescriptor {
        code: "BR-RO-040",
        category: RuleCategory::Core,
        severity: Severity::Fatal,
        business_terms: "BT-8",
        title_en: "VAT Point Date Code Restriction",
        title_ro: "Codul datei de exigibilitate a TVA (UNTDID 2005 / 2475)",
        description_en: "When present, Value added tax point date code (BT-8) must be one of 3 (Invoice issue date), 35 (Actual delivery date), or 432 (Paid to date) in UBL (or 5, 29, 72 in CII DueDateTypeCode).",
        description_ro: "Dacă este prezent, codul datei de exigibilitate a TVA (BT-8) trebuie să fie 3 (Data emiterii), 35 (Data livrării efective) sau 432 (Data încasării) în UBL (respectiv 5, 29 sau 72 în CII DueDateTypeCode).",
        remediation_en: "Restrict cac:InvoicePeriod/cbc:DescriptionCode to '3', '35', or '432' (or CII ram:DueDateTypeCode to '5', '29', or '72').",
        remediation_ro: "Limitați valoarea cac:InvoicePeriod/cbc:DescriptionCode la '3', '35' sau '432' (sau CII ram:DueDateTypeCode la '5', '29' sau '72').",
    },
    RuleDescriptor {
        code: "BR-RO-065",
        category: RuleCategory::PartyAddress,
        severity: Severity::Fatal,
        business_terms: "BT-31, BT-32, BT-63",
        title_en: "Seller Fiscal or VAT Identifier Required",
        title_ro: "Identificator fiscal sau de TVA al Vânzătorului obligatoriu",
        description_en: "Seller tax registration identifier (BT-32), Seller VAT identifier (BT-31), or Seller tax representative VAT identifier (BT-63) must be present.",
        description_ro: "Identificatorul de înregistrare fiscală a Vânzătorului (BT-32), identificatorul de TVA (BT-31) sau cel al reprezentantului fiscal (BT-63) trebuie înscris.",
        remediation_en: "Populate AccountingSupplierParty/Party/PartyTaxScheme/CompanyID (with 'RO' prefix for VAT payers or TaxScheme 'FC' for non-VAT CIF).",
        remediation_ro: "Completați PartyTaxScheme/CompanyID pentru Vânzător (cu prefix 'RO' pentru plătitori de TVA sau fără 'RO' cu TaxScheme 'FC' pentru neplătitori).",
    },
    RuleDescriptor {
        code: "BR-RO-080",
        category: RuleCategory::PartyAddress,
        severity: Severity::Fatal,
        business_terms: "BT-35, BT-50",
        title_en: "Seller and Buyer Address Line 1 Required",
        title_ro: "Adresa Vânzătorului și a Cumpărătorului (Linia 1) obligatorie",
        description_en: "Both Seller address line 1 (BT-35) and Buyer address line 1 (BT-50) must be provided (sub-rules BR-RO-081, BR-RO-082).",
        description_ro: "Atât Linia 1 din adresa Vânzătorului (BT-35), cât și Linia 1 din adresa Cumpărătorului (BT-50) sunt obligatorii (BR-RO-081, BR-RO-082).",
        remediation_en: "Ensure PostalAddress/cbc:StreetName is non-empty for both Seller and Buyer.",
        remediation_ro: "Completați elementul PostalAddress/cbc:StreetName atât pentru Vânzător, cât și pentru Cumpărător.",
    },
    RuleDescriptor {
        code: "BR-RO-090",
        category: RuleCategory::PartyAddress,
        severity: Severity::Fatal,
        business_terms: "BT-37, BT-52",
        title_en: "Seller and Buyer City Required",
        title_ro: "Localitatea Vânzătorului și a Cumpărătorului obligatorie",
        description_en: "Both Seller city (BT-37) and Buyer city (BT-52) must be provided (sub-rules BR-RO-091, BR-RO-092).",
        description_ro: "Atât Localitatea Vânzătorului (BT-37), cât și Localitatea Cumpărătorului (BT-52) trebuie furnizate (BR-RO-091, BR-RO-092).",
        remediation_en: "Ensure PostalAddress/cbc:CityName is non-empty for both Seller and Buyer.",
        remediation_ro: "Completați elementul PostalAddress/cbc:CityName atât pentru Vânzător, cât și pentru Cumpărător.",
    },
    RuleDescriptor {
        code: "BR-RO-100",
        category: RuleCategory::PartyAddress,
        severity: Severity::Fatal,
        business_terms: "BT-37, BT-39, BT-40, BT-52, BT-54, BT-55",
        title_en: "Bucharest (RO-B) City Must Use SECTOR1..SECTOR6",
        title_ro: "Codificare obligatorie SECTOR1..SECTOR6 pentru Municipiul București (RO-B)",
        description_en: "When Country is RO and CountrySubentity is RO-B (Bucharest), CityName must be one of SECTOR1, SECTOR2, SECTOR3, SECTOR4, SECTOR5, SECTOR6 (BR-RO-100 Seller, BR-RO-101 Buyer).",
        description_ro: "Dacă țara este RO și subdiviziunea este RO-B (Municipiul București), localitatea trebuie codificată folosind lista SECTOR-RO: SECTOR1..SECTOR6 (BR-RO-100 Vânzător, BR-RO-101 Cumpărător).",
        remediation_en: "Replace free-text 'București' or 'Sector 1' in cbc:CityName with exact uppercase 'SECTOR1'..'SECTOR6'.",
        remediation_ro: "Înlocuiți 'București' sau 'Sector 1' din cbc:CityName cu valoarea exactă 'SECTOR1'..'SECTOR6'.",
    },
    RuleDescriptor {
        code: "BR-RO-110",
        category: RuleCategory::PartyAddress,
        severity: Severity::Fatal,
        business_terms: "BT-39, BT-40, BT-54, BT-55",
        title_en: "Romanian County Must Use ISO 3166-2:RO",
        title_ro: "Subdiviziunea țării pentru România trebuie codificată ISO 3166-2:RO",
        description_en: "When Seller or Buyer country code is RO, CountrySubentity must be coded using ISO 3166-2:RO (RO-B, RO-AB..RO-VN; BR-RO-110 Seller, BR-RO-111 Buyer).",
        description_ro: "Dacă țara Vânzătorului sau Cumpărătorului este RO, subdiviziunea țării trebuie codificată conform ISO 3166-2:RO (RO-B, RO-AB..RO-VN; BR-RO-110 Vânzător, BR-RO-111 Cumpărător).",
        remediation_en: "Set cbc:CountrySubentity to a valid ISO 3166-2:RO code such as 'RO-B', 'RO-CJ', 'RO-TM', or 'RO-IS'.",
        remediation_ro: "Setați cbc:CountrySubentity la un cod ISO 3166-2:RO valid (ex. 'RO-B', 'RO-CJ', 'RO-TM', 'RO-IS').",
    },
    RuleDescriptor {
        code: "BR-RO-120",
        category: RuleCategory::PartyAddress,
        severity: Severity::Fatal,
        business_terms: "BT-47, BT-48",
        title_en: "Buyer Legal Registration or VAT Identifier Required",
        title_ro: "Identificatorul legal sau de TVA al Cumpărătorului obligatoriu",
        description_en: "Buyer legal registration identifier (BT-47) and/or Buyer VAT identifier (BT-48) must be present (for B2C consumers without disclosed CNP, use '0000000000000').",
        description_ro: "Identificatorul de înregistrare legală a Cumpărătorului (BT-47) și/sau identificatorul de TVA al Cumpărătorului (BT-48) trebuie înscris (pentru B2C fără CNP se utilizează '0000000000000').",
        remediation_en: "Provide Buyer PartyTaxScheme/CompanyID or PartyLegalEntity/CompanyID (CUI, ONRC, CNP, or 0000000000000 for B2C).",
        remediation_ro: "Completați PartyTaxScheme/CompanyID sau PartyLegalEntity/CompanyID la Cumpărător (CUI, ONRC, CNP sau 0000000000000 pentru B2C).",
    },
    RuleDescriptor {
        code: "BR-RO-140",
        category: RuleCategory::PartyAddress,
        severity: Severity::Fatal,
        business_terms: "BG-11, BG-12, BT-64",
        title_en: "Tax Representative Street Required",
        title_ro: "Linia 1 de adresă a reprezentantului fiscal obligatorie",
        description_en: "If Seller has a Tax Representative party (BG-11), Tax representative address line 1 (BT-64) must be provided.",
        description_ro: "Dacă Vânzătorul are un reprezentant fiscal (BG-11), adresa acestuia trebuie să conțină Linia 1 (BT-64).",
        remediation_en: "Populate TaxRepresentativeParty/PostalAddress/cbc:StreetName.",
        remediation_ro: "Completați TaxRepresentativeParty/PostalAddress/cbc:StreetName.",
    },
    RuleDescriptor {
        code: "BR-RO-150",
        category: RuleCategory::PartyAddress,
        severity: Severity::Fatal,
        business_terms: "BG-11, BG-12, BT-66",
        title_en: "Tax Representative City Required",
        title_ro: "Localitatea reprezentantului fiscal obligatorie",
        description_en: "If Seller has a Tax Representative party (BG-11), Tax representative city (BT-66) must be provided.",
        description_ro: "Dacă Vânzătorul are un reprezentant fiscal (BG-11), adresa acestuia trebuie să conțină Localitatea (BT-66).",
        remediation_en: "Populate TaxRepresentativeParty/PostalAddress/cbc:CityName.",
        remediation_ro: "Completați TaxRepresentativeParty/PostalAddress/cbc:CityName.",
    },
    RuleDescriptor {
        code: "BR-RO-160",
        category: RuleCategory::PartyAddress,
        severity: Severity::Fatal,
        business_terms: "BT-66, BT-68, BT-69",
        title_en: "Tax Representative Bucharest Sector",
        title_ro: "Sector București pentru reprezentantul fiscal (RO-B)",
        description_en: "When Tax representative country (BT-69) is RO and subdivision (BT-68) is RO-B, city (BT-66) must be SECTOR1..SECTOR6.",
        description_ro: "Dacă țara reprezentantului fiscal (BT-69) este RO și subdiviziunea (BT-68) este RO-B, localitatea (BT-66) trebuie să fie SECTOR1..SECTOR6.",
        remediation_en: "Set TaxRepresentativeParty/PostalAddress/cbc:CityName to SECTOR1..SECTOR6.",
        remediation_ro: "Setați localitatea reprezentantului fiscal la SECTOR1..SECTOR6.",
    },
    RuleDescriptor {
        code: "BR-RO-170",
        category: RuleCategory::PartyAddress,
        severity: Severity::Fatal,
        business_terms: "BT-68, BT-69",
        title_en: "Tax Representative Romanian County Code",
        title_ro: "Cod județ ISO 3166-2:RO pentru reprezentantul fiscal",
        description_en: "When Tax representative country (BT-69) is RO, subdivision (BT-68) must be a valid ISO 3166-2:RO code.",
        description_ro: "Dacă țara reprezentantului fiscal (BT-69) este RO, subdiviziunea (BT-68) trebuie codificată conform ISO 3166-2:RO.",
        remediation_en: "Set TaxRepresentativeParty/PostalAddress/cbc:CountrySubentity to RO-B or RO-AB..RO-VN.",
        remediation_ro: "Setați CountrySubentity pentru reprezentantul fiscal la un cod ISO 3166-2:RO valid.",
    },
    RuleDescriptor {
        code: "BR-RO-180",
        category: RuleCategory::PartyAddress,
        severity: Severity::Fatal,
        business_terms: "BG-15, BT-75",
        title_en: "Delivery Address Line 1 Required When BG-15 Exists",
        title_ro: "Linia 1 a adresei de livrare obligatorie când BG-15 există",
        description_en: "If Deliver to address (BG-15) is present, Deliver to address line 1 (BT-75) must be provided.",
        description_ro: "Dacă grupul Adresa de livrare (BG-15) există, trebuie furnizată Linia 1 a adresei de livrare (BT-75).",
        remediation_en: "Populate Delivery/DeliveryLocation/Address/cbc:StreetName when Address is included.",
        remediation_ro: "Completați cbc:StreetName în adresa de livrare (BG-15) sau eliminați elementul Address gol.",
    },
    RuleDescriptor {
        code: "BR-RO-200",
        category: RuleCategory::PartyAddress,
        severity: Severity::Fatal,
        business_terms: "BG-15, BT-77, BT-79, BT-80",
        title_en: "Delivery City & Bucharest Sector Rules",
        title_ro: "Localitatea de livrare și regula SECTOR1..6 pentru RO-B",
        description_en: "If Deliver to address (BG-15) exists, Deliver to city (BT-77) must be non-empty (BR-RO-201), and if country is RO with RO-B, must be SECTOR1..SECTOR6 (BR-RO-202).",
        description_ro: "Dacă Adresa de livrare (BG-15) există, Localitatea de livrare (BT-77) este obligatorie (BR-RO-201), iar pentru RO + RO-B trebuie codificată SECTOR1..SECTOR6 (BR-RO-202).",
        remediation_en: "Provide Delivery Address cbc:CityName (using SECTOR1..SECTOR6 when CountrySubentity is RO-B).",
        remediation_ro: "Completați cbc:CityName în adresa de livrare (folosind SECTOR1..SECTOR6 pentru RO-B).",
    },
    RuleDescriptor {
        code: "BR-RO-210",
        category: RuleCategory::PartyAddress,
        severity: Severity::Fatal,
        business_terms: "BG-15, BT-79, BT-80",
        title_en: "Delivery Country Subdivision & ISO 3166-2:RO",
        title_ro: "Subdiviziunea țării la livrare (BT-79 / ISO 3166-2:RO)",
        description_en: "If Deliver to address (BG-15) exists, CountrySubentity (BT-79) must be provided (BR-RO-211) and, when country is RO, must use ISO 3166-2:RO (BR-RO-212).",
        description_ro: "Dacă Adresa de livrare (BG-15) există, subdiviziunea țării (BT-79) este obligatorie (BR-RO-211), iar pentru țara RO trebuie codificată ISO 3166-2:RO (BR-RO-212).",
        remediation_en: "Populate Delivery Address cbc:CountrySubentity (using RO-B or RO-AB..RO-VN when country is RO).",
        remediation_ro: "Completați cbc:CountrySubentity în adresa de livrare (folosind RO-B sau RO-AB..RO-VN pentru România).",
    },
    RuleDescriptor {
        code: "BR-RO-DT001",
        category: RuleCategory::Dates,
        severity: Severity::Fatal,
        business_terms: "BT-2",
        title_en: "IssueDate Strict YYYY-MM-DD Format",
        title_ro: "Format strict YYYY-MM-DD pentru data emiterii (IssueDate)",
        description_en: "IssueDate (BT-2) must be exactly 10 characters long and a valid calendar date formatted YYYY-MM-DD (no timezone suffix).",
        description_ro: "Elementul IssueDate (BT-2) trebuie să aibă exact 10 caractere și să respecte formatul calendaristic YYYY-MM-DD (fără fus orar).",
        remediation_en: "Format cbc:IssueDate as YYYY-MM-DD (e.g. '2026-10-15').",
        remediation_ro: "Formatați cbc:IssueDate exact ca YYYY-MM-DD (ex. '2026-10-15').",
    },
    RuleDescriptor {
        code: "BR-RO-DT002",
        category: RuleCategory::Dates,
        severity: Severity::Fatal,
        business_terms: "BT-7",
        title_en: "TaxPointDate Strict YYYY-MM-DD Format",
        title_ro: "Format strict YYYY-MM-DD pentru TaxPointDate (BT-7)",
        description_en: "TaxPointDate (BT-7) must be exactly 10 characters long and formatted YYYY-MM-DD.",
        description_ro: "Data de exigibilitate a TVA TaxPointDate (BT-7) trebuie să respecte formatul YYYY-MM-DD (10 caractere).",
        remediation_en: "Format cbc:TaxPointDate as YYYY-MM-DD.",
        remediation_ro: "Formatați cbc:TaxPointDate ca YYYY-MM-DD.",
    },
    RuleDescriptor {
        code: "BR-RO-DT003",
        category: RuleCategory::Dates,
        severity: Severity::Fatal,
        business_terms: "BT-9",
        title_en: "DueDate Strict YYYY-MM-DD Format",
        title_ro: "Format strict YYYY-MM-DD pentru data scadenței (DueDate)",
        description_en: "DueDate (BT-9) must be exactly 10 characters long and formatted YYYY-MM-DD.",
        description_ro: "Data scadenței DueDate (BT-9) trebuie să respecte formatul YYYY-MM-DD (10 caractere).",
        remediation_en: "Format cbc:DueDate as YYYY-MM-DD.",
        remediation_ro: "Formatați cbc:DueDate ca YYYY-MM-DD.",
    },
    RuleDescriptor {
        code: "BR-RO-DT003-CN",
        category: RuleCategory::Dates,
        severity: Severity::Fatal,
        business_terms: "BT-9",
        title_en: "PaymentDueDate Strict YYYY-MM-DD Format",
        title_ro: "Format strict YYYY-MM-DD pentru PaymentDueDate (CreditNote)",
        description_en: "PaymentDueDate (BT-9 on CreditNote) must be exactly 10 characters long and formatted YYYY-MM-DD.",
        description_ro: "Elementul PaymentDueDate (BT-9 pe nota de creditare) trebuie să respecte formatul YYYY-MM-DD (10 caractere).",
        remediation_en: "Format cbc:PaymentDueDate as YYYY-MM-DD.",
        remediation_ro: "Formatați cbc:PaymentDueDate ca YYYY-MM-DD.",
    },
    RuleDescriptor {
        code: "BR-RO-DT004",
        category: RuleCategory::Dates,
        severity: Severity::Fatal,
        business_terms: "BT-72",
        title_en: "ActualDeliveryDate Strict YYYY-MM-DD Format",
        title_ro: "Format strict YYYY-MM-DD pentru data livrării efective",
        description_en: "ActualDeliveryDate (BT-72) must be exactly 10 characters long and formatted YYYY-MM-DD.",
        description_ro: "Data livrării efective ActualDeliveryDate (BT-72) trebuie să respecte formatul YYYY-MM-DD (10 caractere).",
        remediation_en: "Format cbc:ActualDeliveryDate as YYYY-MM-DD.",
        remediation_ro: "Formatați cbc:ActualDeliveryDate ca YYYY-MM-DD.",
    },
    RuleDescriptor {
        code: "BR-RO-DT005",
        category: RuleCategory::Dates,
        severity: Severity::Fatal,
        business_terms: "BT-73, BT-134",
        title_en: "Period StartDate Strict YYYY-MM-DD Format",
        title_ro: "Format strict YYYY-MM-DD pentru StartDate (BT-73, BT-134)",
        description_en: "Invoice or line period StartDate (BT-73, BT-134) must be formatted YYYY-MM-DD (10 characters).",
        description_ro: "Data de început a perioadei de facturare StartDate (BT-73, BT-134) trebuie să respecte formatul YYYY-MM-DD.",
        remediation_en: "Format cbc:StartDate as YYYY-MM-DD.",
        remediation_ro: "Formatați cbc:StartDate ca YYYY-MM-DD.",
    },
    RuleDescriptor {
        code: "BR-RO-DT006",
        category: RuleCategory::Dates,
        severity: Severity::Fatal,
        business_terms: "BT-74, BT-135",
        title_en: "Period EndDate Strict YYYY-MM-DD Format",
        title_ro: "Format strict YYYY-MM-DD pentru EndDate (BT-74, BT-135)",
        description_en: "Invoice or line period EndDate (BT-74, BT-135) must be formatted YYYY-MM-DD (10 characters).",
        description_ro: "Data de sfârșit a perioadei de facturare EndDate (BT-74, BT-135) trebuie să respecte formatul YYYY-MM-DD.",
        remediation_en: "Format cbc:EndDate as YYYY-MM-DD.",
        remediation_ro: "Formatați cbc:EndDate ca YYYY-MM-DD.",
    },
    RuleDescriptor {
        code: "BR-RO-A020",
        category: RuleCategory::Cardinality,
        severity: Severity::Fatal,
        business_terms: "BG-1",
        title_en: "Maximum 20 Invoice Notes",
        title_ro: "Maximum 20 de comentarii în factură (BG-1)",
        description_en: "An invoice may contain at most 20 occurrences of Invoice note (BG-1 / cbc:Note).",
        description_ro: "O factură poate conține cel mult 20 de apariții ale grupului Comentariu în factură (BG-1 / cbc:Note).",
        remediation_en: "Consolidate header cbc:Note elements so no more than 20 appear on the document.",
        remediation_ro: "Comasați elementele cbc:Note la nivel de document astfel încât numărul lor să nu depășească 20.",
    },
    RuleDescriptor {
        code: "BR-RO-A050",
        category: RuleCategory::Cardinality,
        severity: Severity::Fatal,
        business_terms: "BG-24, BG-32",
        title_en: "Maximum 50 Supporting Documents or Item Attributes",
        title_ro: "Maximum 50 de documente justificative (BG-24) sau atribute articol (BG-32)",
        description_en: "At most 50 occurrences of Additional supporting documents (BG-24, BR-RO-A051) and at most 50 Item attributes (BG-32, BR-RO-A052) are allowed.",
        description_ro: "Sunt permise maximum 50 de apariții ale grupului Documente justificative suplimentare (BG-24, BR-RO-A051) și maximum 50 de Atribute ale articolului (BG-32, BR-RO-A052).",
        remediation_en: "Keep AdditionalDocumentReference <= 50 and Item/AdditionalItemProperty <= 50.",
        remediation_ro: "Limitați numărul de elemente AdditionalDocumentReference și AdditionalItemProperty la cel mult 50.",
    },
    RuleDescriptor {
        code: "BR-RO-A500",
        category: RuleCategory::Cardinality,
        severity: Severity::Fatal,
        business_terms: "BG-3",
        title_en: "Maximum 500 Preceding Invoice References",
        title_ro: "Maximum 500 de referințe la facturi anterioare (BG-3)",
        description_en: "An invoice may contain at most 500 occurrences of Preceding invoice reference (BG-3).",
        description_ro: "O factură poate conține maximum 500 de apariții ale grupului Referință la o factură anterioară (BG-3).",
        remediation_en: "Ensure BillingReference / InvoiceDocumentReference occurrences do not exceed 500.",
        remediation_ro: "Limitați numărul referințelor la facturi anterioare (BG-3) la cel mult 500.",
    },
    RuleDescriptor {
        code: "BR-RO-L020",
        category: RuleCategory::Length,
        severity: Severity::Fatal,
        business_terms: "BT-38, BT-53, BT-67, BT-78",
        title_en: "Maximum 20 Characters for Postal Codes",
        title_ro: "Maximum 20 de caractere pentru codurile poștale",
        description_en: "Normalized character length of Seller, Buyer, Tax Representative, and Delivery post codes must not exceed 20 characters.",
        description_ro: "Lungimea maximă permisă pentru codul poștal (Vânzător, Cumpărător, Reprezentant fiscal, Livrare) este de 20 de caractere.",
        remediation_en: "Trim cbc:PostalZone values to at most 20 characters.",
        remediation_ro: "Limitați valoarea cbc:PostalZone la cel mult 20 de caractere.",
    },
    RuleDescriptor {
        code: "BR-RO-L050",
        category: RuleCategory::Length,
        severity: Severity::Fatal,
        business_terms: "BT-37, BT-52, BT-66, BT-77, BT-160",
        title_en: "Maximum 50 Characters for Cities & Item Attribute Names",
        title_ro: "Maximum 50 de caractere pentru localități și numele atributelor",
        description_en: "Normalized character length of city names (BT-37, BT-52, BT-66, BT-77) and item attribute names (BT-160) must not exceed 50 characters.",
        description_ro: "Numărul maxim permis de caractere pentru localitate (BT-37, BT-52, BT-66, BT-77) și numele atributului articolului (BT-160) este 50.",
        remediation_en: "Shorten cbc:CityName and AdditionalItemProperty/cbc:Name to <= 50 characters.",
        remediation_ro: "Limitați cbc:CityName și numele atributului articolului la cel mult 50 de caractere.",
    },
    RuleDescriptor {
        code: "BR-RO-L100",
        category: RuleCategory::Length,
        severity: Severity::Fatal,
        business_terms: "BT-19, BT-36, BT-41..43, BT-51, BT-56..58, BT-65, BT-76, BT-82, BT-97, BT-104, BT-120, BT-123, BT-133, BT-139, BT-144, BT-153, BT-161..165",
        title_en: "Maximum 100 Characters for Item Names, Contacts, Address Lines 2/3 & Reasons",
        title_ro: "Maximum 100 de caractere pentru numele articolului, contacte, liniile 2/3 de adresă și motive",
        description_en: "Normalized character length of Item name (BT-153), address lines 2/3, contact fields, accounting cost, VAT exemption reason, and allowance/charge reasons must not exceed 100 characters.",
        description_ro: "Numărul maxim permis de caractere pentru numele articolului (BT-153, mutați detaliile în BT-154), liniile 2/3 de adresă, datele de contact și motivele de scutire/deducere este 100.",
        remediation_en: "Keep Item/cbc:Name <= 100 chars (put longer text in Item/cbc:Description up to 200 chars) and trim contact/reason fields to <= 100 chars.",
        remediation_ro: "Păstrați Item/cbc:Name la <= 100 caractere (mutați detaliile în Item/cbc:Description până la 200 caractere) și limitați câmpurile de contact/motive la <= 100 caractere.",
    },
    RuleDescriptor {
        code: "BR-RO-L140",
        category: RuleCategory::Length,
        severity: Severity::Fatal,
        business_terms: "BT-83",
        title_en: "Maximum 140 Characters for Remittance Information",
        title_ro: "Maximum 140 de caractere pentru avizul de plată (BT-83)",
        description_en: "Normalized character length of Remittance information (BT-83 / PaymentMeans/cbc:PaymentID) must not exceed 140 characters.",
        description_ro: "Numărul maxim permis de caractere pentru Avizul de plată (BT-83 / cbc:PaymentID) este 140.",
        remediation_en: "Shorten PaymentMeans/cbc:PaymentID to at most 140 characters (SEPA unstructured remittance cap).",
        remediation_ro: "Limitați PaymentMeans/cbc:PaymentID la maximum 140 de caractere.",
    },
    RuleDescriptor {
        code: "BR-RO-L150",
        category: RuleCategory::Length,
        severity: Severity::Fatal,
        business_terms: "BT-35, BT-50, BT-64, BT-75",
        title_en: "Maximum 150 Characters for Street Address Line 1",
        title_ro: "Maximum 150 de caractere pentru Linia 1 de adresă",
        description_en: "Normalized character length of StreetName (BT-35, BT-50, BT-64, BT-75) must not exceed 150 characters.",
        description_ro: "Numărul maxim permis de caractere pentru Linia 1 a adresei (BT-35, BT-50, BT-64, BT-75) este 150.",
        remediation_en: "Split long addresses across StreetName (<= 150), AdditionalStreetName (<= 100), and AddressLine/Line (<= 100).",
        remediation_ro: "Împărțiți adresele lungi între StreetName (<= 150), AdditionalStreetName (<= 100) și AddressLine/Line (<= 100).",
    },
    RuleDescriptor {
        code: "BR-RO-L200",
        category: RuleCategory::Length,
        severity: Severity::Fatal,
        business_terms: "BT-1, BT-12..18, BT-25, BT-27, BT-28, BT-44, BT-45, BT-59, BT-62, BT-70, BT-85, BT-88, BT-122, BT-124, BT-125-2, BT-154",
        title_en: "Maximum 200 Characters for Party Names, Document IDs & Item Descriptions",
        title_ro: "Maximum 200 de caractere pentru denumiri părți, referințe documente și descrierea articolului",
        description_en: "Normalized character length of Invoice number (BT-1), document references, party legal/trading names, account/card names, and Item description (BT-154) must not exceed 200 characters.",
        description_ro: "Numărul maxim permis de caractere pentru numărul facturii (BT-1), referințele de documente, denumirile părților și descrierea articolului (BT-154) este 200.",
        remediation_en: "Ensure party names, document IDs, URIs, filenames, and Item/cbc:Description are <= 200 characters.",
        remediation_ro: "Limitați denumirile părților, referințele, URI-urile și descrierea articolului (BT-154) la maximum 200 de caractere.",
    },
    RuleDescriptor {
        code: "BR-RO-L300",
        category: RuleCategory::Length,
        severity: Severity::Fatal,
        business_terms: "BT-20, BT-22, BT-127",
        title_en: "Maximum 300 Characters for Payment Terms & Notes",
        title_ro: "Maximum 300 de caractere pentru termeni de plată și note",
        description_en: "Normalized character length of Payment terms (BT-20), Invoice note (BT-22), and Invoice line note (BT-127) must not exceed 300 characters.",
        description_ro: "Numărul maxim permis de caractere pentru Termeni de plată (BT-20), Comentariu în factură (BT-22) și Nota liniei (BT-127) este 300.",
        remediation_en: "Split long remarks into multiple cbc:Note elements (up to 20 notes, each <= 300 chars).",
        remediation_ro: "Împărțiți comentariile lungi în mai multe elemente cbc:Note (maximum 20 de note, fiecare <= 300 caractere).",
    },
    RuleDescriptor {
        code: "BR-RO-L1000",
        category: RuleCategory::Length,
        severity: Severity::Fatal,
        business_terms: "BT-33",
        title_en: "Maximum 1000 Characters for Seller Additional Legal Information",
        title_ro: "Maximum 1000 de caractere pentru informațiile juridice suplimentare ale Vânzătorului",
        description_en: "Normalized character length of Seller additional legal information (BT-33 / CompanyLegalForm) must not exceed 1000 characters.",
        description_ro: "Numărul maxim permis de caractere pentru Informații juridice suplimentare despre Vânzător (BT-33 / CompanyLegalForm) este 1000.",
        remediation_en: "Trim PartyLegalEntity/cbc:CompanyLegalForm (share capital, social headquarters) to <= 1000 characters.",
        remediation_ro: "Limitați PartyLegalEntity/cbc:CompanyLegalForm (capital social, mențiuni legale) la maximum 1000 de caractere.",
    },
    RuleDescriptor {
        code: "BR-RO-Z2",
        category: RuleCategory::Decimals,
        severity: Severity::Fatal,
        business_terms: "BT-92, BT-93, BT-99, BT-100, BT-106..117, BT-131, BT-136, BT-137, BT-141, BT-142",
        title_en: "Maximum 2 Fractional Decimals on Monetary Amounts",
        title_ro: "Maximum 2 zecimale pentru sumele monetare din factură",
        description_en: "Document totals, VAT totals/subtotals, line net amounts, and allowance/charge amounts/bases must have at most 2 digits after the decimal point (e.g. '100.00', not '100.000'; unit prices BT-146 may still use >2 decimals).",
        description_ro: "Totalurile facturii, sumele TVA, valoarea netă a liniei și deducerile/taxele suplimentare pot avea cel mult 2 zecimale (ex. '100.00', nu '100.000'; prețul unitar BT-146 poate avea mai mult de 2 zecimale).",
        remediation_en: "Round and format monetary amount elements to at most 2 decimal places (0, 1, or 2 fractional digits).",
        remediation_ro: "Rotunjiți și formatați sumele monetare la cel mult 2 zecimale în XML.",
    },
    RuleDescriptor {
        code: "BR-27",
        category: RuleCategory::Core,
        severity: Severity::Fatal,
        business_terms: "BT-146",
        title_en: "Item Net Price Must Not Be Negative",
        title_ro: "Prețul net al articolului (BT-146) nu poate fi negativ",
        description_en: "The Item net price (BT-146 / cbc:PriceAmount) shall NOT be negative.",
        description_ro: "Prețul net al articolului (BT-146 / cbc:PriceAmount) trebuie să NU fie negativ.",
        remediation_en: "Use a non-negative unit price; represent discounts via AllowanceCharge or negative quantities / CreditNote.",
        remediation_ro: "Utilizați un preț unitar >= 0; reprezentați reducerile prin AllowanceCharge, cantități negative sau CreditNote.",
    },
    RuleDescriptor {
        code: "BR-RO-CUI",
        category: RuleCategory::Checksum,
        severity: Severity::Warning,
        business_terms: "BT-31, BT-32, BT-47, BT-48, BT-63",
        title_en: "Romanian CUI / CIF Modulo-11 Control Digit",
        title_ro: "Verificarea cifrei de control modulo-11 pentru CUI / CIF",
        description_en: "Romanian fiscal identifier (2..10 digits, with or without 'RO' prefix) should satisfy the Ministry of Finance modulo-11 check-digit algorithm.",
        description_ro: "Codul de identificare fiscală românesc (2..10 cifre, cu sau fără prefix 'RO') trebuie să respecte algoritmul oficial modulo-11 al Ministerului Finanțelor.",
        remediation_en: "Verify the party's CUI/CIF against the ANAF / mfinante.ro taxpayer register.",
        remediation_ro: "Verificați codul CUI/CIF în registrul contribuabililor ANAF / mfinante.ro.",
    },
    RuleDescriptor {
        code: "BR-RO-CNP",
        category: RuleCategory::Checksum,
        severity: Severity::Warning,
        business_terms: "BT-47, BT-48",
        title_en: "Romanian 13-Digit CNP Checksum & Calendar Validity",
        title_ro: "Validarea CNP (13 cifre) sau substituent B2C 0000000000000",
        description_en: "A 13-digit numeric identifier on a Romanian party must either equal '0000000000000' (ANAF OUG 69/2024 B2C placeholder) or pass 13-digit CNP date, county, and modulo-11 verification.",
        description_ro: "Un identificator numeric de 13 cifre pentru o parte din România trebuie să fie fie '0000000000000' (substituent B2C cf. OUG 69/2024), fie un CNP valid calendaristic și modulo-11.",
        remediation_en: "Check the 13-digit CNP for typos or use '0000000000000' when the natural-person buyer does not disclose their CNP.",
        remediation_ro: "Verificați corectitudinea CNP-ului sau utilizați '0000000000000' pentru clienți persoane fizice (B2C) care nu comunică CNP-ul.",
    },
    RuleDescriptor {
        code: "BR-RO-ONRC",
        category: RuleCategory::Checksum,
        severity: Severity::Warning,
        business_terms: "BT-30, BT-47",
        title_en: "Romanian ONRC Trade Register Format",
        title_ro: "Formatul numărului din Registrul Comerțului (ONRC)",
        description_en: "Romanian Trade Register numbers (J/F/C...) should follow either classic 'J40/1234/2022' or Law 265/2022 'J2024001234005' structure.",
        description_ro: "Numărul de ordine din Registrul Comerțului trebuie să respecte formatul clasic 'J40/1234/2022' sau formatul electronic cf. Legii 265/2022 'J2024001234005'.",
        remediation_en: "Format ONRC legal registration IDs as J<county>/<seq>/<year> or J<YYYY><NNNNNN><JJ><C>.",
        remediation_ro: "Formatați numărul ONRC ca J<județ>/<nr>/<an> (ex. J40/1234/2022) sau conform Legii 265/2022.",
    },
    RuleDescriptor {
        code: "BR-RO-IBAN",
        category: RuleCategory::Checksum,
        severity: Severity::Warning,
        business_terms: "BT-84",
        title_en: "Romanian IBAN ISO 7064 MOD 97-10 Checksum",
        title_ro: "Verificarea IBAN românesc (24 caractere, ISO 7064 MOD 97-10)",
        description_en: "Payee bank account identifiers starting with 'RO' (BT-84) should be 24 characters long and pass ISO/IEC 7064 MOD 97-10 check-digit verification.",
        description_ro: "Conturile bancare IBAN care încep cu 'RO' (BT-84) trebuie să aibă 24 de caractere și să treacă verificarea cifrelor de control ISO/IEC 7064 MOD 97-10.",
        remediation_en: "Check the 24-character Romanian IBAN in PayeeFinancialAccount/cbc:ID for missing characters or mistyped digits.",
        remediation_ro: "Verificați codul IBAN de 24 de caractere înscris în PayeeFinancialAccount/cbc:ID.",
    },
    RuleDescriptor {
        code: "XSD-SCHEMA",
        category: RuleCategory::ContainerXsd,
        severity: Severity::Fatal,
        business_terms: "UBL 2.1 / CII D16B XSD",
        title_en: "Official W3C XML Schema (XSD 1.1) Grammar & XXE Protection",
        title_ro: "Validare schemă oficială W3C XSD 1.1 (UBL 2.1 / CII D16B) și protecție XXE",
        description_en: "XML invoice must be well-formed, free of DOCTYPE/ENTITY declarations, and conformant to OASIS UBL 2.1 Invoice/CreditNote or UN/CEFACT CII D16B XSD.",
        description_ro: "Documentul XML trebuie să fie bine format, fără declarații DOCTYPE/ENTITY și conform cu schemele XSD oficiale OASIS UBL 2.1 sau UN/CEFACT CII D16B.",
        remediation_en: "Ensure UBL/CII elements appear in strict XSD sequence order with valid namespaces and datatypes.",
        remediation_ro: "Respectați ordinea strictă a elementelor din schema XSD UBL 2.1 / CII D16B și tipurile de date aferente.",
    },
    RuleDescriptor {
        code: "PDFA3-ISO19005",
        category: RuleCategory::ContainerXsd,
        severity: Severity::Fatal,
        business_terms: "ISO 19005-3 / efactura.xml",
        title_en: "Hybrid PDF/A-3 Container & Associated XML Attachment",
        title_ro: "Container hibrid PDF/A-3 (ISO 19005-3) și atașament XML (efactura.xml)",
        description_en: "Hybrid PDF invoices must conform to ISO 19005-3 (unencrypted, trailer /ID, GTS_PDFA1 ICC profile, embedded fonts, unfiltered XMP pdfaid:part=3, /AF association) and carry efactura.xml / factur-x.xml.",
        description_ro: "Facturile PDF hibride trebuie să respecte ISO 19005-3 (fără criptare, /ID în trailer, profil ICC GTS_PDFA1, fonturi încorporate, XMP necomprimat pdfaid:part=3, asociere /AF) și să conțină atașamentul XML (efactura.xml / factur-x.xml).",
        remediation_en: "Export the visual invoice as PDF/A-3b and attach efactura.xml in both /Names/EmbeddedFiles and Catalog /AF.",
        remediation_ro: "Exportați factura vizuală ca PDF/A-3b și atașați efactura.xml atât în /Names/EmbeddedFiles, cât și în Catalog /AF.",
    },
];

/// Looks up a [`RuleDescriptor`] by canonical code or ANAF Schematron sub-ID / container clause
/// (case-insensitive, e.g. `"BR-RO-100"`, `"BR-RO-101"`, `"BR-DEC-RO-05"`, `"BR-RO-L155"`,
/// `"XSD-SCHEMA-001"`, or `"PDFA3-6.1.4-ENCRYPT"`).
#[must_use]
pub fn find_rule(code: &str) -> Option<&'static RuleDescriptor> {
    let target = code.trim();
    if target.is_empty() || target.len() > 64 {
        return None;
    }

    if let Some(exact) = RULES.iter().find(|r| r.code.eq_ignore_ascii_case(target)) {
        return Some(exact);
    }

    let mut upper_buf = [0u8; 64];
    for (dst, &src) in upper_buf.iter_mut().zip(target.as_bytes()) {
        if !src.is_ascii() {
            return None;
        }
        *dst = src.to_ascii_uppercase();
    }
    let upper = core::str::from_utf8(&upper_buf[..target.len()]).ok()?;

    let canonical = match upper {
        "BR-RO-DT003_CN" => "BR-RO-DT003-CN",
        "BR-RO-020_1" | "BR-RO-020_2" => "BR-RO-020",
        "BR-RO-081" | "BR-RO-082" => "BR-RO-080",
        "BR-RO-091" | "BR-RO-092" => "BR-RO-090",
        "BR-RO-101" => "BR-RO-100",
        "BR-RO-111" => "BR-RO-110",
        "BR-RO-201" | "BR-RO-202" => "BR-RO-200",
        "BR-RO-211" | "BR-RO-212" => "BR-RO-210",
        "BR-RO-A051" | "BR-RO-A052" => "BR-RO-A050",
        "BR-RO-L151" | "BR-RO-L152" | "BR-RO-L153" | "BR-RO-L154" => "BR-RO-L150",
        "BR-RO-L155" | "BR-RO-L156" => "BR-RO-L200",
        "BR-RO-L301" | "BR-RO-L302" | "BR-RO-L303" => "BR-RO-L300",
        s if s.starts_with("BR-DEC-RO-") => "BR-RO-Z2",
        s if s.starts_with("BR-RO-L020") => "BR-RO-L020",
        s if s.starts_with("BR-RO-L050") => "BR-RO-L050",
        s if s.starts_with("BR-RO-L10") => "BR-RO-L100",
        s if s.starts_with("BR-RO-L030") || s.starts_with("BR-RO-L2") => "BR-RO-L200",
        s if s.starts_with("PDFA3-") => "PDFA3-ISO19005",
        s if s.starts_with("XSD-") => "XSD-SCHEMA",
        _ => return None,
    };

    RULES.iter().find(|r| r.code == canonical)
}

/// Renders the complete rule catalog as a GitHub-Flavored Markdown reference document in `locale`.
#[must_use]
pub fn render_rules_markdown(locale: Locale) -> String {
    let (h1, banner, intro, cols, l_cat, l_sev, l_bt, l_spec, l_rem) = match locale {
        Locale::En => (
            "# ANAF RO e-Factura (`CIUS-RO 1.0.1`) Validation Rule Reference\n\n",
            "> **Language / Limba:** **English** | [Română](RULES.ro.md)\n\n",
            "This reference is generated directly from `cius_ro::rules::RULES` and covers all national ANAF `RO_CIUS` (`v1.0.9`) rules, `cius-ro-id` fiscal checksums, W3C XSD 1.1 grammar checks, and ISO 19005-3 (PDF/A-3) hybrid `efactura.xml` checks.\n\n",
            "| Code | Severity | Business Terms | Title |\n",
            "Category",
            "Severity",
            "Business Terms",
            "Specification",
            "Remediation",
        ),
        Locale::Ro => (
            "# Catalogul Regulilor de Validare ANAF RO e-Factura (`CIUS-RO 1.0.1`)\n\n",
            "> **Limba / Language:** [English](RULES.en.md) | **Română**\n\n",
            "Acest ghid este generat automat din catalogul `cius_ro::rules::RULES` și acoperă toate regulile naționale ANAF `RO_CIUS` (`v1.0.9`), verificările algoritmice `cius-ro-id` (CUI, CNP, ONRC, IBAN), schema oficială W3C XSD 1.1 și inspecția containerelor hibride PDF/A-3 (`efactura.xml`).\n\n",
            "| Cod regulă | Severitate | Termeni de business | Titlu |\n",
            "Categorie",
            "Severitate",
            "Termeni EN 16931",
            "Descriere normativă",
            "Recomandare de remediere",
        ),
    };
    let mut out = String::with_capacity(16_384);
    out.push_str(h1);
    out.push_str(banner);
    out.push_str(intro);
    out.push_str(cols);
    out.push_str("| :--- | :---: | :--- | :--- |\n");
    for r in RULES {
        let anchor = r.code.to_ascii_lowercase();
        let _ = writeln!(
            out,
            "| [`{}`](#{anchor}) | `{}` | `{}` | {} |",
            r.code,
            r.severity,
            r.business_terms,
            r.title(locale)
        );
    }
    out.push_str("\n---\n\n");
    for r in RULES {
        let _ = write!(
            out,
            "## `{}` — {}\n\n- **{l_cat}:** `{}`\n- **{l_sev}:** `{}`\n- **{l_bt}:** `{}`\n- **{l_spec}:** {}\n- **{l_rem}:** {}\n\n",
            r.code,
            r.title(locale),
            r.category,
            r.severity,
            r.business_terms,
            r.description(locale),
            r.remediation(locale),
        );
    }
    out
}
