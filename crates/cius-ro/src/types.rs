//! Core public types for `cius-ro` validation options, localized diagnostics, and reports.

use core::fmt;

pub use cius_ro_id::{Locale, LocalizedError as LocalizedDisplay, LocalizedFmt, ParseLocaleError};

/// Official ANAF `RO e-Factura` (`CIUS-RO 1.0.1`) `BT-24` Specification Identifier URI.
pub const CIUS_RO_CUSTOMIZATION_ID: &str =
    "urn:cen.eu:en16931:2017#compliant#urn:efactura.mfinante.ro:CIUS-RO:1.0.1";

macro_rules! impl_str_enum {
    ($ty:ty, $doc:literal, { $($variant:pat => $label:literal),+ $(,)? }) => {
        impl $ty {
            #[doc = $doc]
            #[must_use]
            pub const fn as_str(self) -> &'static str {
                match self {
                    $($variant => $label,)+
                }
            }
        }

        impl fmt::Display for $ty {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl PartialEq<&str> for $ty {
            fn eq(&self, other: &&str) -> bool {
                self.as_str() == *other
            }
        }

        impl PartialEq<str> for $ty {
            fn eq(&self, other: &str) -> bool {
                self.as_str() == other
            }
        }
    };
}

/// Overall processing decision recommended for a validated document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
pub enum Recommendation {
    /// Document satisfies all configured normative checks (`"accept"`).
    Accept,
    /// Document failed one or more fatal checks, or warnings when `deny_warnings` is set (`"reject"`).
    Reject,
}

impl_str_enum!(Recommendation, "Lowercase string label (`\"accept\"` or `\"reject\"`).", {
    Self::Accept => "accept",
    Self::Reject => "reject",
});

/// Identifier of the validation stage that produced an [`EngineReport`] or [`Violation`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub enum EngineId {
    /// ISO 19005-3 (`PDF/A-3`) binary structure & `efactura.xml` inspector (`"pdfa3_iso19005"`).
    #[cfg_attr(feature = "serde", serde(rename = "pdfa3_iso19005"))]
    Pdfa3Iso19005,
    /// Official W3C XML Schema (`XSD 1.1`) grammar & XXE validator (`"xsd_schema"`).
    #[cfg_attr(feature = "serde", serde(rename = "xsd_schema"))]
    XsdSchema,
    /// CEN EN 16931-1 fixed-point base-10 semantic model (`"en16931_model"`).
    #[cfg_attr(feature = "serde", serde(rename = "en16931_model"))]
    En16931Model,
    /// ANAF `RO e-Factura` (`CIUS-RO 1.0.1`) national rules & `cius-ro-id` checksums (`"anaf_cius_ro"`).
    #[cfg_attr(feature = "serde", serde(rename = "anaf_cius_ro"))]
    AnafCiusRo,
    /// Pre-parse input gateway (`"gateway"`).
    #[cfg_attr(feature = "serde", serde(rename = "gateway"))]
    Gateway,
}

impl_str_enum!(
    EngineId,
    "Stable snake-case wire identifier (`\"pdfa3_iso19005\"`, `\"xsd_schema\"`, `\"en16931_model\"`, `\"anaf_cius_ro\"`, `\"gateway\"`).",
    {
        Self::Pdfa3Iso19005 => "pdfa3_iso19005",
        Self::XsdSchema => "xsd_schema",
        Self::En16931Model => "en16931_model",
        Self::AnafCiusRo => "anaf_cius_ro",
        Self::Gateway => "gateway",
    }
);

/// Functional grouping of a normative rule in [`crate::rules::RULES`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub enum RuleCategory {
    /// Core CIUS-RO document rules (`"core"`).
    Core,
    /// Party identifiers, addresses, ISO 3166-2:RO counties, and Bucharest sectors (`"party_address"`).
    PartyAddress,
    /// Lexical date format constraints (`"dates"`).
    Dates,
    /// Bounded group repetition limits (`"cardinality"`).
    Cardinality,
    /// Normalized character length limits (`"length"`).
    Length,
    /// Lexical monetary decimal precision rules (`"decimals"`).
    Decimals,
    /// Algorithmic Romanian fiscal/administrative checksums (`"checksum"`).
    Checksum,
    /// W3C XSD 1.1 grammar and ISO 19005-3 PDF/A-3 container rules (`"container_xsd"`).
    ContainerXsd,
}

impl_str_enum!(RuleCategory, "Snake-case category identifier (`\"core\"`, `\"party_address\"`, etc.).", {
    Self::Core => "core",
    Self::PartyAddress => "party_address",
    Self::Dates => "dates",
    Self::Cardinality => "cardinality",
    Self::Length => "length",
    Self::Decimals => "decimals",
    Self::Checksum => "checksum",
    Self::ContainerXsd => "container_xsd",
});

/// Error returned when raw XML bytes cannot be decoded as UTF-8 or UTF-16.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum XmlDecodeError {
    /// Byte stream begins with a UTF-8 BOM (`EF BB BF`) followed by invalid UTF-8 bytes.
    InvalidUtf8AfterBom(core::str::Utf8Error),
    /// Byte stream begins with a UTF-16 BOM (`FE FF` or `FF FE`) but has an odd byte length.
    TruncatedUtf16 {
        /// Detected UTF-16 byte-order label (`"UTF-16BE"` or `"UTF-16LE"`).
        encoding: &'static str,
    },
    /// Byte stream begins with a UTF-16 BOM (`FE FF` or `FF FE`) followed by unpaired surrogates.
    InvalidUtf16 {
        /// Detected UTF-16 byte-order label (`"UTF-16BE"` or `"UTF-16LE"`).
        encoding: &'static str,
    },
    /// Byte stream is not valid UTF-8.
    InvalidUtf8(core::str::Utf8Error),
}

impl LocalizedFmt for XmlDecodeError {
    fn fmt_locale(&self, f: &mut fmt::Formatter<'_>, locale: Locale) -> fmt::Result {
        match (self, locale) {
            (Self::InvalidUtf8AfterBom(err), Locale::En) => {
                write!(f, "Invalid UTF-8 byte sequence after BOM: {err}")
            }
            (Self::InvalidUtf8AfterBom(err), Locale::Ro) => {
                write!(f, "Secvență UTF-8 invalidă după marcajul BOM: {err}")
            }
            (Self::TruncatedUtf16 { encoding }, Locale::En) => {
                write!(
                    f,
                    "Truncated {encoding} XML payload (odd number of bytes after BOM)."
                )
            }
            (Self::TruncatedUtf16 { encoding }, Locale::Ro) => {
                write!(
                    f,
                    "Conținut XML {encoding} trunchiat (număr impar de octeți după BOM)."
                )
            }
            (Self::InvalidUtf16 { encoding }, Locale::En) => {
                write!(f, "Invalid {encoding} surrogate sequence in XML payload.")
            }
            (Self::InvalidUtf16 { encoding }, Locale::Ro) => {
                write!(f, "Secvență surogat {encoding} invalidă în documentul XML.")
            }
            (Self::InvalidUtf8(err), Locale::En) => {
                write!(f, "XML payload is not valid UTF-8: {err}")
            }
            (Self::InvalidUtf8(err), Locale::Ro) => {
                write!(f, "Documentul XML nu este codificat valid UTF-8: {err}")
            }
        }
    }
}

cius_ro_id::impl_localized_display!(XmlDecodeError);

impl core::error::Error for XmlDecodeError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::InvalidUtf8AfterBom(err) | Self::InvalidUtf8(err) => Some(err),
            Self::TruncatedUtf16 { .. } | Self::InvalidUtf16 { .. } => None,
        }
    }
}

/// Severity of a validation finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "UPPERCASE"))]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
pub enum Severity {
    /// Normative breach that causes invoice rejection (`FATAL`).
    Fatal,
    /// Advisory checksum or profile warning (`WARNING`).
    Warning,
    /// Informational notice (`INFO`).
    Info,
}

impl_str_enum!(Severity, "Uppercase string label (`\"FATAL\"`, `\"WARNING\"`, `\"INFO\"`).", {
    Self::Fatal => "FATAL",
    Self::Warning => "WARNING",
    Self::Info => "INFO",
});

/// Execution status of an individual verification stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "UPPERCASE"))]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
pub enum EngineStatus {
    /// Stage ran and produced zero fatal violations.
    Passed,
    /// Stage ran and emitted one or more fatal violations.
    Failed,
    /// Stage was disabled by options, feature flags, or platform target.
    Skipped,
}

impl_str_enum!(EngineStatus, "Uppercase wire status label (`\"PASSED\"`, `\"FAILED\"`, `\"SKIPPED\"`).", {
    Self::Passed => "PASSED",
    Self::Failed => "FAILED",
    Self::Skipped => "SKIPPED",
});

/// Detected EN 16931 XML syntax binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub enum XmlSyntax {
    /// OASIS UBL 2.1 `<Invoice>`.
    UblInvoice,
    /// OASIS UBL 2.1 `<CreditNote>`.
    UblCreditNote,
    /// UN/CEFACT `CrossIndustryInvoice` D16B `<CrossIndustryInvoice>`.
    CiiD16B,
}

impl XmlSyntax {
    /// Human-readable specification label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::UblInvoice => "OASIS UBL 2.1 Invoice",
            Self::UblCreditNote => "OASIS UBL 2.1 CreditNote",
            Self::CiiD16B => "UN/CEFACT CII D16B",
        }
    }
}

impl fmt::Display for XmlSyntax {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// Container format supplied to the validator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "UPPERCASE"))]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
pub enum ContainerKind {
    /// Standalone XML e-invoice (`UBL 2.1` or `CII D16B`).
    Xml,
    /// Hybrid PDF/A-3 document embedding an XML invoice (`efactura.xml`, `factur-x.xml`, etc.).
    PdfA3,
}

impl_str_enum!(ContainerKind, "Uppercase container label (`\"XML\"` or `\"PDFA3\"`).", {
    Self::Xml => "XML",
    Self::PdfA3 => "PDFA3",
});

/// Extracted business identity fields (`BT-1`, `BT-2`, `BT-27`) when readable.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
pub struct DocumentIdentity {
    /// Invoice or credit note number (`BT-1`).
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub invoice_number: Option<String>,
    /// Document issue date (`BT-2`, `YYYY-MM-DD`).
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub issue_date: Option<String>,
    /// Seller legal or trading name (`BT-27` / `BT-28`).
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub seller_name: Option<String>,
}

/// Summary of one verification engine's execution on an invoice.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
pub struct EngineReport {
    /// Stage identifier (`"pdfa3_iso19005"`, `"xsd_schema"`, `"en16931_model"`, `"anaf_cius_ro"`).
    pub id: EngineId,
    /// Human-readable engine name.
    pub name: String,
    /// Specification version evaluated.
    pub version: String,
    /// Stage pass/fail/skip outcome.
    pub status: EngineStatus,
    /// Number of assertions or grammar checks evaluated.
    pub rules_evaluated: usize,
    /// Number of checks that passed without fatal errors.
    pub passed_count: usize,
    /// Number of fatal violations emitted by this stage.
    pub failed_count: usize,
    /// Number of advisory warnings emitted by this stage.
    pub warning_count: usize,
    /// Reason why the stage was skipped, when `status == EngineStatus::Skipped`.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub skip_reason: Option<String>,
}

impl EngineReport {
    pub(crate) fn skipped(
        id: EngineId,
        name: &str,
        version: &str,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            id,
            name: name.to_string(),
            version: version.to_string(),
            status: EngineStatus::Skipped,
            rules_evaluated: 0,
            passed_count: 0,
            failed_count: 0,
            warning_count: 0,
            skip_reason: Some(reason.into()),
        }
    }
}

/// A single normative or checksum finding emitted during validation.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
pub struct Violation {
    /// Canonical specification rule code (e.g. `"BR-RO-100"`, `"BR-RO-110"`, `"BR-RO-Z2"`, `"BR-RO-CUI"`, `"BR-CO-15"`).
    pub code: String,
    /// Specific ANAF Schematron assertion sub-identifier when applicable (e.g. `"BR-RO-101"`, `"BR-DEC-RO-09"`, `"BR-RO-L1024"`).
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub rule_sub_id: Option<String>,
    /// Engine stage that produced this finding.
    pub engine: EngineId,
    /// Severity level (`Fatal`, `Warning`, `Info`).
    pub severity: Severity,
    /// EN 16931 Business Term / Group or ISO clause (e.g. `"BT-37"`, `"BG-4"`, `"ISO 19005-3:6.8"`).
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub business_term: Option<String>,
    /// Localized diagnostic message (`Locale::En` or `Locale::Ro`).
    pub message: String,
    /// Element path, XPath-like location, or line/column reference.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub location: Option<String>,
    /// Expected normative value or constraint.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub expected: Option<String>,
    /// Observed value in the document.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub actual: Option<String>,
    /// Actionable remediation hint in the requested [`Locale`].
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub hint: Option<String>,
}

/// Configuration controlling how [`crate::validate_bytes`] and [`crate::validate_xml`] inspect a document.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(default, rename_all = "camelCase"))]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
#[expect(
    clippy::struct_excessive_bools,
    reason = "Independent toggles for each validation stage and policy flag"
)]
pub struct ValidateOptions {
    /// Output language for messages and hints (`Locale::En` by default).
    pub locale: Locale,
    /// Run ISO 19005-3 (PDF/A-3) container & attachment checks when input is a PDF (`true` by default).
    pub run_pdfa: bool,
    /// Run W3C XSD 1.1 grammar validation (`true` by default when `xsd` feature is enabled).
    pub run_xsd: bool,
    /// Run CEN EN 16931 fixed-point base-10 semantic validation (`true` by default).
    pub run_en16931: bool,
    /// Run ANAF `CIUS-RO 1.0.1` (`BR-RO-*`) national rules (`true` by default).
    pub run_cius_ro_rules: bool,
    /// Run `cius-ro-id` algorithmic checksum verifications (`CUI`, `CNP`, `ONRC`, `RO IBAN`) (`true` by default).
    pub run_checksums: bool,
    /// Require `BT-24` to equal [`CIUS_RO_CUSTOMIZATION_ID`] as a `Fatal` rule (`true` by default).
    /// When `false` (multi-standard router mode), a Romanian-context invoice declaring another
    /// EN 16931 profile (e.g. Factur-X) receives a `Warning` `BR-RO-001` instead of a `Fatal` rejection,
    /// while non-Romanian invoices omit `BR-RO-001` and skip `BR-RO-*` rules (checksums still run if enabled).
    pub strict_cius_ro_customization: bool,
    /// Escalate `Warning` findings so any warning also marks `is_valid = false` (`false` by default).
    pub deny_warnings: bool,
    /// Generate OASIS Schematron Validation Report Language (`SVRL`) XML output (`false` by default;
    /// requires crate feature `svrl`, enabled by default).
    pub include_svrl: bool,
}

impl Default for ValidateOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl ValidateOptions {
    /// Creates default validation options (`Locale::En`, all stages enabled, `deny_warnings = false`).
    #[must_use]
    pub const fn new() -> Self {
        Self {
            locale: Locale::En,
            run_pdfa: true,
            run_xsd: true,
            run_en16931: true,
            run_cius_ro_rules: true,
            run_checksums: true,
            strict_cius_ro_customization: true,
            deny_warnings: false,
            include_svrl: false,
        }
    }

    /// Creates default validation options with the specified [`Locale`].
    #[must_use]
    pub const fn with_locale(locale: Locale) -> Self {
        Self::new().locale(locale)
    }

    /// Sets the diagnostic output [`Locale`].
    #[must_use]
    pub const fn locale(mut self, locale: Locale) -> Self {
        self.locale = locale;
        self
    }

    /// Enables or disables ISO 19005-3 (`PDF/A-3`) structural container checks.
    #[must_use]
    pub const fn pdfa(mut self, enabled: bool) -> Self {
        self.run_pdfa = enabled;
        self
    }

    /// Enables or disables W3C XSD 1.1 grammar validation.
    #[must_use]
    pub const fn xsd(mut self, enabled: bool) -> Self {
        self.run_xsd = enabled;
        self
    }

    /// Enables or disables CEN EN 16931 semantic model evaluation.
    #[must_use]
    pub const fn en16931(mut self, enabled: bool) -> Self {
        self.run_en16931 = enabled;
        self
    }

    /// Enables or disables ANAF `CIUS-RO 1.0.1` (`BR-RO-*`) national rules.
    #[must_use]
    pub const fn cius_ro_rules(mut self, enabled: bool) -> Self {
        self.run_cius_ro_rules = enabled;
        self
    }

    /// Enables or disables `cius-ro-id` fiscal checksum verifications.
    #[must_use]
    pub const fn checksums(mut self, enabled: bool) -> Self {
        self.run_checksums = enabled;
        self
    }

    /// Configures whether a non-`CIUS-RO` `BT-24` identifier is treated as `Fatal` (`true`) or `Warning` (`false`).
    #[must_use]
    pub const fn strict_cius_ro_customization(mut self, enabled: bool) -> Self {
        self.strict_cius_ro_customization = enabled;
        self
    }

    /// Escalates advisory warnings so any warning marks `is_valid = false`.
    #[must_use]
    pub const fn deny_warnings(mut self, enabled: bool) -> Self {
        self.deny_warnings = enabled;
        self
    }

    /// Requests generation of an OASIS SVRL XML report in [`ValidationReport::svrl_xml`].
    #[must_use]
    pub const fn include_svrl(mut self, enabled: bool) -> Self {
        self.include_svrl = enabled;
        self
    }
}

/// Complete verification report returned by [`crate::validate_bytes`] and [`crate::validate_xml`].
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
pub struct ValidationReport {
    /// `true` if zero `Fatal` violations occurred (and zero `Warning`s when `deny_warnings` is set).
    pub is_valid: bool,
    /// Overall processing recommendation (`Recommendation::Accept` or `Recommendation::Reject`).
    pub recommendation: Recommendation,
    /// Language used to format messages and hints.
    pub locale: Locale,
    /// Detected input container (`Xml` or `PdfA3`).
    pub container: ContainerKind,
    /// Detected XML syntax binding (`OASIS UBL 2.1 Invoice/CreditNote` or `UN/CEFACT CII D16B`).
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub detected_syntax: Option<XmlSyntax>,
    /// Raw `BT-24` Specification Identifier (`CustomizationID` / `GuidelineSpecifiedDocumentContextParameter/ID`).
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub customization_id: Option<String>,
    /// Extracted invoice identity (`BT-1`, `BT-2`, `BT-27`) when readable.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub identity: Option<DocumentIdentity>,
    /// Per-stage execution summaries.
    pub engines: Vec<EngineReport>,
    /// All normative and checksum findings in deterministic order.
    pub violations: Vec<Violation>,
    /// Optional SVRL XML report when `include_svrl = true` and crate feature `svrl` is enabled.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub svrl_xml: Option<String>,
}
