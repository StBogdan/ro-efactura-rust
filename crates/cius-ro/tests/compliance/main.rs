//! Integration test suite for `cius-ro`, organized into focused domain modules:
//! - [`golden`]: compliant XML & PDF/A-3 corpus, `xmllint` XSD cross-check, faulty fixtures,
//!   rule catalog, bilingual (`en`/`ro`) & SVRL output, and multi-standard router mode.
//! - [`ubl_rules`]: UBL 2.1 `Invoice` & `CreditNote` `BR-RO-*` single-fault mutations, B2C `CNP` /
//!   OUG 69/2024 placeholder, subdivisions, dates, lengths, 2-decimal caps, and identifiers.
//! - [`cii_rules`]: UN/CEFACT CII D16B `BR-RO-*` single-fault mutations, notes, tax point dates,
//!   and foreign-seller `BR-RO-030` rules.
//! - [`pdfa3`]: ISO 19005-3 `PDF/A-3` hybrid extraction, tamper detection, `/Supplement`
//!   attachments, UTF-8 filenames, `/ObjStm` streams, and `XRef` bomb guards.
//! - [`security`]: XXE/DTD defense, deep comment padding, multi-byte UTF-8 boundary handling,
//!   BOM / UTF-16 decoding, max element depth, and strict XML namespace isolation.

mod cii_rules;
mod golden;
mod helpers;
#[cfg(feature = "pdfa3")]
mod pdfa3;
mod security;
mod ubl_rules;
