//! Core public types for `cius-ro` validation options, localized diagnostics, and reports.

use core::fmt;
use core::str::FromStr;

/// Official ANAF `RO e-Factura` (`CIUS-RO 1.0.1`) `BT-24` Specification Identifier URI.
pub const CIUS_RO_CUSTOMIZATION_ID: &str =
    "urn:cen.eu:en16931:2017#compliant#urn:efactura.mfinante.ro:CIUS-RO:1.0.1";

/// Output language for human-readable rule descriptions, error messages, and remediation hints.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum Locale {
    /// English (`en`).
    #[default]
    En,
    /// Romanian (`ro` — *Română*).
    Ro,
}

impl Locale {
    /// Returns `true` when this locale is [`Locale::Ro`].
    #[must_use]
    pub const fn is_ro(self) -> bool {
        matches!(self, Self::Ro)
    }

    /// Returns the two-letter BCP-47 language tag (`"en"` or `"ro"`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::Ro => "ro",
        }
    }
}

impl fmt::Display for Locale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Locale {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let low = s.trim().to_ascii_lowercase();
        if low == "en" || low.starts_with("en-") || low.starts_with("en_") {
            Ok(Self::En)
        } else if low == "ro" || low.starts_with("ro-") || low.starts_with("ro_") {
            Ok(Self::Ro)
        } else {
            Err(format!("unsupported locale '{s}'; expected 'en' or 'ro'"))
        }
    }
}

/// Severity of a validation finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "UPPERCASE"))]
pub enum Severity {
    /// Normative breach that causes invoice rejection (`FATAL`).
    Fatal,
    /// Advisory checksum or profile warning (`WARNING`).
    Warning,
    /// Informational notice (`INFO`).
    Info,
}

impl Severity {
    /// Uppercase string label (`"FATAL"`, `"WARNING"`, `"INFO"`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Fatal => "FATAL",
            Self::Warning => "WARNING",
            Self::Info => "INFO",
        }
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Execution status of an individual verification stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "UPPERCASE"))]
pub enum EngineStatus {
    /// Stage ran and produced zero fatal violations.
    Passed,
    /// Stage ran and emitted one or more fatal violations.
    Failed,
    /// Stage was disabled by options, feature flags, or platform target.
    Skipped,
}

/// Detected EN 16931 XML syntax binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum XmlSyntax {
    /// OASIS UBL 2.1 `<Invoice>`.
    UblInvoice,
    /// OASIS UBL 2.1 `<CreditNote>`.
    UblCreditNote,
    /// UN/CEFACT CrossIndustryInvoice D16B `<CrossIndustryInvoice>`.
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
pub enum ContainerKind {
    /// Standalone XML e-invoice (`UBL 2.1` or `CII D16B`).
    Xml,
    /// Hybrid PDF/A-3 document embedding an XML invoice (`efactura.xml`, `factur-x.xml`, etc.).
    PdfA3,
}

/// Extracted business identity fields (`BT-1`, `BT-2`, `BT-27`) when readable.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct DocumentIdentity {
    /// Invoice or credit note number (`BT-1`).
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub invoice_number: Option<String>,
    /// Document issue date (`BT-2`, `YYYY-MM-DD`).
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub issue_date: Option<String>,
    /// Seller legal or trading name (`BT-27` / `BT-28`).
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub seller_name: Option<String>,
}

/// Summary of one verification engine's execution on an invoice.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct EngineReport {
    /// Stable engine identifier (`"pdfa3_iso19005"`, `"xsd_schema"`, `"en16931_model"`, `"anaf_cius_ro"`).
    pub id: String,
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
    pub skip_reason: Option<String>,
}

impl EngineReport {
    pub(crate) fn skipped(id: &str, name: &str, version: &str, reason: impl Into<String>) -> Self {
        Self {
            id: id.to_string(),
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
pub struct Violation {
    /// Canonical specification rule code (e.g. `"BR-RO-100"`, `"BR-RO-110"`, `"BR-RO-Z2"`, `"BR-RO-CUI"`, `"BR-CO-15"`).
    pub code: String,
    /// Specific ANAF Schematron assertion sub-identifier when applicable (e.g. `"BR-RO-101"`, `"BR-DEC-RO-09"`, `"BR-RO-L1024"`).
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub rule_sub_id: Option<String>,
    /// Engine stage that produced this finding.
    pub engine: String,
    /// Severity level (`Fatal`, `Warning`, `Info`).
    pub severity: Severity,
    /// EN 16931 Business Term / Group or ISO clause (e.g. `"BT-37"`, `"BG-4"`, `"ISO 19005-3:6.8"`).
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub business_term: Option<String>,
    /// Localized diagnostic message (`Locale::En` or `Locale::Ro`).
    pub message: String,
    /// Element path, XPath-like location, or line/column reference.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub location: Option<String>,
    /// Expected normative value or constraint.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub expected: Option<String>,
    /// Observed value in the document.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub actual: Option<String>,
    /// Actionable remediation hint in the requested [`Locale`].
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub hint: Option<String>,
}

/// Configuration controlling how [`crate::validate_bytes`] and [`crate::validate_xml`] inspect a document.
#[derive(Debug, Clone)]
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
    /// Enforce `BR-RO-050` requiring `PaymentMeansCode` (`BT-81`) (`true` by default).
    pub require_payment_means: bool,
    /// Require `BT-24` to equal [`CIUS_RO_CUSTOMIZATION_ID`] as a `Fatal` rule (`true` by default).
    /// When `false` (multi-standard router mode), a domestic `RO`->`RO` invoice declaring another
    /// EN 16931 profile (e.g. Factur-X) receives a `Warning` `BR-RO-001` instead of a `Fatal` rejection.
    pub strict_cius_ro_customization: bool,
    /// Escalate `Warning` findings so any warning also marks `is_valid = false` (`false` by default).
    pub deny_warnings: bool,
    /// Generate OASIS Schematron Validation Report Language (`SVRL`) XML output (`false` by default).
    pub include_svrl: bool,
}

impl Default for ValidateOptions {
    fn default() -> Self {
        Self {
            locale: Locale::En,
            run_pdfa: true,
            run_xsd: true,
            run_en16931: true,
            run_cius_ro_rules: true,
            run_checksums: true,
            require_payment_means: true,
            strict_cius_ro_customization: true,
            deny_warnings: false,
            include_svrl: false,
        }
    }
}

impl ValidateOptions {
    /// Creates default validation options with the specified [`Locale`].
    #[must_use]
    pub fn with_locale(locale: Locale) -> Self {
        Self {
            locale,
            ..Self::default()
        }
    }
}

/// Complete verification report returned by [`crate::validate_bytes`] and [`crate::validate_xml`].
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct ValidationReport {
    /// `true` if zero `Fatal` violations occurred (and zero `Warning`s when `deny_warnings` is set).
    pub is_valid: bool,
    /// Overall processing recommendation (`"accept"` or `"reject"`).
    pub recommendation: String,
    /// Language used to format messages and hints.
    pub locale: Locale,
    /// Detected input container (`Xml` or `PdfA3`).
    pub container: ContainerKind,
    /// Detected XML syntax binding (`OASIS UBL 2.1 Invoice/CreditNote` or `UN/CEFACT CII D16B`).
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub detected_syntax: Option<XmlSyntax>,
    /// Raw `BT-24` Specification Identifier (`CustomizationID` / `GuidelineSpecifiedDocumentContextParameter/ID`).
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub customization_id: Option<String>,
    /// Extracted invoice identity (`BT-1`, `BT-2`, `BT-27`) when readable.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub identity: Option<DocumentIdentity>,
    /// Per-stage execution summaries.
    pub engines: Vec<EngineReport>,
    /// All normative and checksum findings in deterministic order.
    pub violations: Vec<Violation>,
    /// Optional SVRL XML report when `include_svrl = true`.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub svrl_xml: Option<String>,
}
