//! Shared violation accumulator (`StageAcc`) used across validation stages (`xsd`, `semantic`,
//! `br_ro`, and `pdfa3`).

use crate::types::{EngineId, EngineReport, EngineStatus, Locale, Severity, Violation};

/// Stack-allocated builder for a single [`Violation`] before recording it into [`StageAcc`].
pub(crate) struct StageFinding<'a> {
    severity: Severity,
    code: &'a str,
    sub_id: Option<&'a str>,
    term: Option<&'a str>,
    location: Option<String>,
    message: String,
    expected: Option<&'a str>,
    actual: Option<String>,
    hint: Option<String>,
}

impl<'a> StageFinding<'a> {
    /// Creates a finding with explicit `severity`, `code`, and localized `message`.
    pub(crate) fn new(severity: Severity, code: &'a str, message: impl Into<String>) -> Self {
        Self {
            severity,
            code,
            sub_id: None,
            term: None,
            location: None,
            message: message.into(),
            expected: None,
            actual: None,
            hint: None,
        }
    }

    /// Creates a [`Severity::Fatal`] finding.
    pub(crate) fn fatal(code: &'a str, message: impl Into<String>) -> Self {
        Self::new(Severity::Fatal, code, message)
    }

    /// Creates a [`Severity::Warning`] finding.
    pub(crate) fn warning(code: &'a str, message: impl Into<String>) -> Self {
        Self::new(Severity::Warning, code, message)
    }

    /// Attaches a Schematron sub-rule identifier (`Violation::rule_sub_id`).
    pub(crate) fn sub_id(mut self, sub_id: &'a str) -> Self {
        self.sub_id = Some(sub_id);
        self
    }

    /// Attaches a Business Term / Group or ISO clause (`Violation::business_term`).
    pub(crate) fn term(mut self, term: &'a str) -> Self {
        self.term = Some(term);
        self
    }

    /// Attaches an element path, XPath-like location, or line/column reference (`Violation::location`).
    pub(crate) fn location(mut self, location: impl Into<String>) -> Self {
        self.location = Some(location.into());
        self
    }

    /// Attaches an expected normative value (`Violation::expected`).
    pub(crate) fn expected(mut self, expected: &'a str) -> Self {
        self.expected = Some(expected);
        self
    }

    /// Attaches an optional expected normative value (`Violation::expected`).
    pub(crate) fn maybe_expected(mut self, expected: Option<&'a str>) -> Self {
        self.expected = expected;
        self
    }

    /// Attaches an observed value from the document (`Violation::actual`).
    pub(crate) fn actual(mut self, actual: impl Into<String>) -> Self {
        self.actual = Some(actual.into());
        self
    }

    /// Attaches an optional observed value (`Violation::actual`).
    pub(crate) fn maybe_actual(mut self, actual: Option<String>) -> Self {
        self.actual = actual;
        self
    }

    /// Attaches a localized remediation hint (`Violation::hint`).
    pub(crate) fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// Attaches an optional remediation hint (`Violation::hint`).
    pub(crate) fn maybe_hint(mut self, hint: Option<String>) -> Self {
        self.hint = hint;
        self
    }

    pub(crate) fn into_violation(self, engine: EngineId) -> Violation {
        Violation {
            code: self.code.to_owned(),
            rule_sub_id: self.sub_id.map(str::to_owned),
            engine,
            severity: self.severity,
            business_term: self.term.map(str::to_owned),
            message: self.message,
            location: self.location,
            expected: self.expected.map(str::to_owned),
            actual: self.actual,
            hint: self.hint,
        }
    }
}

/// Selects `en` or `ro` according to `locale`.
#[inline]
pub(crate) const fn tr<'a>(locale: Locale, en: &'a str, ro: &'a str) -> &'a str {
    locale.select(en, ro)
}

pub(crate) use cius_ro_id::tr_fmt;

/// Shared per-stage rule counter and [`Violation`] collector across `xsd`, `semantic`, `br_ro`,
/// and `pdfa3`.
pub(crate) struct StageAcc {
    engine: EngineId,
    name: &'static str,
    version: &'static str,
    locale: Locale,
    evaluated: usize,
    failed: usize,
    warnings: usize,
    violations: Vec<Violation>,
}

impl StageAcc {
    /// Creates a new accumulator for the given engine stage and output [`Locale`].
    pub(crate) const fn new(
        engine: EngineId,
        name: &'static str,
        version: &'static str,
        locale: Locale,
    ) -> Self {
        Self {
            engine,
            name,
            version,
            locale,
            evaluated: 0,
            failed: 0,
            warnings: 0,
            violations: Vec::new(),
        }
    }

    /// Returns the target diagnostic [`Locale`] for cold-path message formatting.
    #[inline]
    pub(crate) const fn locale(&self) -> Locale {
        self.locale
    }

    #[inline]
    pub(crate) const fn __as_locale(&self) -> Locale {
        self.locale
    }

    /// Selects `en` or `ro` according to [`StageAcc::locale`].
    #[inline]
    pub(crate) const fn tr<'a>(&self, en: &'a str, ro: &'a str) -> &'a str {
        self.locale.select(en, ro)
    }

    /// Updates the reported specification/schema version label when resolved mid-stage.
    #[cfg(feature = "xsd")]
    pub(crate) fn set_version(&mut self, version: &'static str) {
        self.version = version;
    }

    /// Records a passing rule evaluation.
    #[inline]
    pub(crate) fn pass(&mut self) {
        self.evaluated += 1;
    }

    /// Adds a batch of pre-counted rule evaluations (e.g. upstream `en16931` semantic rules).
    pub(crate) fn add_evaluated(&mut self, count: usize) {
        self.evaluated += count;
    }

    /// Ensures `rules_evaluated` is at least `floor` (e.g. XSD element count).
    #[cfg(all(
        feature = "xsd",
        not(all(target_arch = "wasm32", target_os = "unknown"))
    ))]
    pub(crate) fn ensure_min_evaluated(&mut self, floor: usize) {
        self.evaluated = self.evaluated.max(floor);
    }

    /// Accounts for truncated fatal findings whose individual [`Violation`]s were capped.
    #[cfg(all(
        feature = "xsd",
        not(all(target_arch = "wasm32", target_os = "unknown"))
    ))]
    pub(crate) fn add_unreported_failures(&mut self, count: usize) {
        self.failed += count;
        self.evaluated += count;
    }

    /// Read-only view of accumulated violations (used for cross-pass deduplication).
    pub(crate) fn violations(&self) -> &[Violation] {
        &self.violations
    }

    /// Records a [`StageFinding`] and increments `evaluated` plus `failed`/`warnings`.
    pub(crate) fn record(&mut self, finding: StageFinding<'_>) {
        self.evaluated += 1;
        self.record_uncounted(finding);
    }

    /// Records a [`StageFinding`] updating `failed`/`warnings` when `evaluated` was already counted.
    pub(crate) fn record_uncounted(&mut self, finding: StageFinding<'_>) {
        match finding.severity {
            Severity::Fatal => self.failed += 1,
            Severity::Warning => self.warnings += 1,
            Severity::Info => {}
        }
        self.violations.push(finding.into_violation(self.engine));
    }

    /// Shorthand for recording a [`Severity::Fatal`] rule with a sub-ID, business term, and location.
    pub(crate) fn fatal_sub(
        &mut self,
        code: &str,
        sub_id: &str,
        term: &str,
        loc: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.record(
            StageFinding::fatal(code, message)
                .sub_id(sub_id)
                .term(term)
                .location(loc),
        );
    }

    /// Shorthand for recording a static bilingual [`Severity::Fatal`] ISO clause check.
    #[cfg(feature = "pdfa3")]
    pub(crate) fn fail_clause(
        &mut self,
        code: &str,
        clause: &str,
        loc: impl Into<String>,
        en: &str,
        ro: &str,
    ) {
        let msg = self.tr(en, ro).to_owned();
        self.fatal_clause(code, clause, loc, msg);
    }

    /// Shorthand for recording a formatted [`Severity::Fatal`] ISO clause check.
    #[cfg(feature = "pdfa3")]
    pub(crate) fn fatal_clause(
        &mut self,
        code: &str,
        clause: &str,
        loc: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.record(
            StageFinding::fatal(code, message)
                .term(clause)
                .location(loc),
        );
    }

    /// Shorthand for recording a formatted [`Severity::Warning`] ISO clause check.
    #[cfg(feature = "pdfa3")]
    pub(crate) fn warn_clause(
        &mut self,
        code: &str,
        clause: &str,
        loc: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.record(
            StageFinding::warning(code, message)
                .term(clause)
                .location(loc),
        );
    }

    /// Finalizes the stage into its [`EngineReport`] and collected [`Violation`] list.
    pub(crate) fn into_report(self) -> (EngineReport, Vec<Violation>) {
        (
            EngineReport {
                id: self.engine,
                name: self.name.to_owned(),
                version: self.version.to_owned(),
                status: if self.failed == 0 {
                    EngineStatus::Passed
                } else {
                    EngineStatus::Failed
                },
                rules_evaluated: self.evaluated,
                passed_count: self.evaluated.saturating_sub(self.failed),
                failed_count: self.failed,
                warning_count: self.warnings,
                skip_reason: None,
            },
            self.violations,
        )
    }
}
