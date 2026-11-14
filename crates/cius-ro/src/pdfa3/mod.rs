//! Binary ISO 19005-3 (`PDF/A-3`) container inspector and `efactura.xml` attachment extractor.
//!
//! Operates in memory (native and `wasm32`) via `lopdf`, `ttf-parser`, `roxmltree`,
//! and `en16931_formats::zugferd`, supporting Romanian `efactura.xml` as well as `factur-x.xml`,
//! `zugferd-invoice.xml`, and `xrechnung.xml`.
//!
//! All stream decompressions and object-graph walks are bounded (`64 MiB` per stream, `128 MiB`
//! cumulative decompression budget per document, max `10,000` visited name-tree objects, max depth `32`)
//! to prevent zip/Flate decompression bombs and cyclic indirect-reference attacks.

mod colors;
mod extract;
mod fonts;
mod loader;
mod xmp;

use std::fmt;

use en16931_formats::zugferd;
use lopdf::{Dictionary, Document, Object, ObjectId, Stream};

pub use self::extract::extract_xml_from_pdf;
use self::loader::{PdfLoadError, load_bounded_document};
use crate::stage_acc::{StageAcc, StageFinding};
use crate::types::{
    EngineId, EngineReport, Locale, LocalizedDisplay, LocalizedFmt, Severity, Violation,
};

pub(crate) const ENGINE_ID: EngineId = EngineId::Pdfa3Iso19005;
pub(crate) const ENGINE_NAME: &str = "ISO 19005-3 (PDF/A-3) & efactura.xml Inspector";
pub(crate) const ENGINE_VERSION: &str = "ISO 19005-3:2012";

pub(super) const MAX_STREAM_BYTES: usize = zugferd::Limits::DEFAULT_MAX_STREAM_BYTES;
pub(super) const MAX_TOTAL_BYTES: usize = zugferd::Limits::DEFAULT_MAX_TOTAL_BYTES;
pub(super) const MAX_TREE_OBJECTS: usize = zugferd::Limits::DEFAULT_MAX_OBJECTS;
pub(super) const MAX_TREE_DEPTH: usize = 32;

/// Error returned by [`extract_xml_from_pdf`] when XML extraction from a PDF container fails.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PdfExtractError {
    /// Input bytes are not a readable PDF container or contain corrupt compressed streams.
    InvalidPdfStructure(String),
    /// A compressed stream or `/Names/EmbeddedFiles` tree exceeded decompression/traversal safety bounds.
    ResourceLimitExceeded(String),
    /// PDF parsed cleanly, but contains no embedded XML invoice (`efactura.xml`, `factur-x.xml`, etc.).
    NoEmbeddedXmlInvoice,
}

impl LocalizedFmt for PdfExtractError {
    fn fmt_locale(&self, f: &mut fmt::Formatter<'_>, locale: Locale) -> fmt::Result {
        match (self, locale) {
            (Self::InvalidPdfStructure(detail), Locale::En) => {
                write!(f, "Failed parsing PDF container: {detail}")
            }
            (Self::InvalidPdfStructure(detail), Locale::Ro) => {
                write!(f, "Containerul PDF nu a putut fi citit: {detail}")
            }
            (Self::ResourceLimitExceeded(detail), Locale::En) => write!(
                f,
                "PDF container exceeded decompression or object-graph safety limits: {detail}"
            ),
            (Self::ResourceLimitExceeded(detail), Locale::Ro) => write!(
                f,
                "Containerul PDF a depășit limitele de securitate la decomprimare: {detail}"
            ),
            (Self::NoEmbeddedXmlInvoice, Locale::En) => f.write_str(
                "No embedded XML e-invoice (efactura.xml / factur-x.xml) found in PDF container.",
            ),
            (Self::NoEmbeddedXmlInvoice, Locale::Ro) => f.write_str(
                "Nu s-a găsit nicio factură XML încorporată (efactura.xml / factur-x.xml) în containerul PDF.",
            ),
        }
    }
}

impl PdfExtractError {
    /// Returns a zero-allocation [`fmt::Display`] adapter formatted in `locale`.
    #[must_use]
    pub const fn display(&self, locale: Locale) -> LocalizedDisplay<'_, Self> {
        LocalizedDisplay::new(self, locale)
    }

    /// Formats a localized error message in `locale`.
    #[must_use]
    pub fn message(&self, locale: Locale) -> String {
        self.display(locale).to_string()
    }
}

impl fmt::Display for PdfExtractError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_locale(f, Locale::En)
    }
}

impl std::error::Error for PdfExtractError {}

/// Extracted XML invoice payload from a hybrid PDF/A-3 container.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct ExtractedPdfXml {
    /// Embedded attachment filename (e.g. `"efactura.xml"` or `"factur-x.xml"`).
    pub filename: String,
    /// Raw decompressed XML bytes.
    pub xml_bytes: Vec<u8>,
}

/// Outcome of inspecting a hybrid PDF/A-3 container.
#[derive(Debug, Clone)]
pub struct PdfInspectionOutcome {
    /// Stage execution summary (`pdfa3_iso19005`).
    pub report: EngineReport,
    /// Structural, ICC, font, XMP, and `/AF` violations.
    pub violations: Vec<Violation>,
    /// Extracted XML invoice payload when found.
    pub extracted_xml: Option<ExtractedPdfXml>,
}

pub(super) struct PdfBudget {
    remaining_total: usize,
}

impl PdfBudget {
    pub(super) const fn new() -> Self {
        Self {
            remaining_total: MAX_TOTAL_BYTES,
        }
    }

    pub(super) fn decode_stream(
        &mut self,
        stream: &Stream,
    ) -> Result<Vec<u8>, PdfStreamDecodeError> {
        let cap = MAX_STREAM_BYTES.min(self.remaining_total);
        if stream.dict.get(b"Filter").is_err() {
            if stream.content.len() > cap {
                return Err(PdfStreamDecodeError::LimitExceeded);
            }
            self.remaining_total = self.remaining_total.saturating_sub(stream.content.len());
            return Ok(stream.content.clone());
        }
        match stream.decompressed_content_with_limit(cap) {
            Ok(bytes) if bytes.len() <= cap => {
                self.remaining_total = self.remaining_total.saturating_sub(bytes.len());
                Ok(bytes)
            }
            Ok(_)
            | Err(lopdf::Error::Decompress(lopdf::DecompressError::MemoryLimitExceeded {
                ..
            })) => Err(PdfStreamDecodeError::LimitExceeded),
            Err(err) => Err(PdfStreamDecodeError::Corrupt(err.to_string())),
        }
    }

    pub(super) fn measure_stream(&mut self, stream: &Stream) -> Result<(), PdfStreamDecodeError> {
        self.decode_stream(stream).map(drop)
    }
}

pub(super) enum PdfStreamDecodeError {
    LimitExceeded,
    Corrupt(String),
}

/// Inspects a PDF container against ISO 19005-3 (PDF/A-3) clauses and extracts its embedded XML invoice.
#[must_use]
pub fn inspect_pdfa3(bytes: &[u8], run_pdfa_checks: bool, locale: Locale) -> PdfInspectionOutcome {
    if !run_pdfa_checks {
        return inspect_pdfa3_disabled(bytes, locale);
    }

    let mut acc = StageAcc::new(ENGINE_ID, ENGINE_NAME, ENGINE_VERSION, locale);
    check_header_and_binary_comment(bytes, &mut acc);

    let mut budget = PdfBudget::new();
    let Some(doc) = load_bounded_for_inspection(bytes, &mut budget, &mut acc) else {
        let (report, violations) = acc.into_report();
        return PdfInspectionOutcome {
            report,
            violations,
            extracted_xml: None,
        };
    };

    if doc.trailer.has(b"Encrypt") || doc.encryption_state.is_some() {
        let msg = acc
            .tr(
                "Trailer dictionary contains an /Encrypt entry; encryption is forbidden in PDF/A-3.",
                "Dicționarul Trailer conține intrarea /Encrypt; criptarea este interzisă în PDF/A-3.",
            )
            .to_owned();
        fail_clause_hint(
            &mut acc,
            "PDFA3-6.1.4-ENCRYPT",
            "ISO 19005-3:6.1.4",
            "Trailer /Encrypt",
            msg,
            "Export the PDF without password or permissions encryption.",
            "Exportați documentul PDF fără parolă sau restricții de criptare.",
        );
    } else {
        acc.pass();
    }

    check_trailer_id(&doc, &mut acc);
    check_streams_and_filters(&doc, &mut acc);

    let catalog = match doc.catalog() {
        Ok(c) => {
            acc.pass();
            c
        }
        Err(err) => {
            let msg = match acc.locale() {
                Locale::En => format!("PDF Document Catalog (/Root) is missing or invalid: {err}"),
                Locale::Ro => {
                    format!("Catalogul documentului PDF (/Root) lipsește sau este invalid: {err}")
                }
            };
            acc.fatal_clause("PDFA3-6.1-CATALOG", "ISO 19005-3:6.1", "Trailer /Root", msg);
            let (report, violations) = acc.into_report();
            return PdfInspectionOutcome {
                report,
                violations,
                extracted_xml: None,
            };
        }
    };

    colors::check_output_intents(&doc, catalog, &mut budget, &mut acc);
    fonts::check_embedded_fonts(&doc, &mut budget, &mut acc);
    let xmp_doc_filename = xmp::check_xmp_metadata(&doc, catalog, &mut budget, &mut acc);
    let extracted_xml = extract::check_associated_files_and_extract(
        bytes,
        &doc,
        catalog,
        xmp_doc_filename.as_deref(),
        &mut budget,
        &mut acc,
    );

    let (report, violations) = acc.into_report();
    PdfInspectionOutcome {
        report,
        violations,
        extracted_xml,
    }
}

fn inspect_pdfa3_disabled(bytes: &[u8], locale: Locale) -> PdfInspectionOutcome {
    match extract_xml_from_pdf(bytes) {
        Ok(ext) => PdfInspectionOutcome {
            report: EngineReport::skipped(
                ENGINE_ID,
                ENGINE_NAME,
                ENGINE_VERSION,
                "Disabled by ValidateOptions",
            ),
            violations: Vec::new(),
            extracted_xml: Some(ext),
        },
        Err(err) => {
            let code = match &err {
                PdfExtractError::InvalidPdfStructure(_) => "PDFA3-6.1-PARSE",
                PdfExtractError::ResourceLimitExceeded(_) => "PDFA3-6.1-RESOURCE-LIMIT",
                PdfExtractError::NoEmbeddedXmlInvoice => "PDFA3-6.8-NO-INVOICE-PAYLOAD",
            };
            PdfInspectionOutcome {
                report: EngineReport::skipped(
                    ENGINE_ID,
                    ENGINE_NAME,
                    ENGINE_VERSION,
                    "Disabled by ValidateOptions",
                ),
                violations: vec![Violation {
                    code: code.to_owned(),
                    rule_sub_id: None,
                    engine: ENGINE_ID,
                    severity: Severity::Fatal,
                    business_term: Some("ISO 19005-3:6.8".to_owned()),
                    message: err.message(locale),
                    location: Some("Catalog /Names/EmbeddedFiles".to_owned()),
                    expected: None,
                    actual: None,
                    hint: None,
                }],
                extracted_xml: None,
            }
        }
    }
}

fn load_bounded_for_inspection(
    bytes: &[u8],
    budget: &mut PdfBudget,
    acc: &mut StageAcc,
) -> Option<Document> {
    match load_bounded_document(bytes, budget) {
        Ok(doc) => {
            acc.pass();
            Some(doc)
        }
        Err(PdfLoadError::LimitExceeded(detail)) => {
            let msg = match acc.locale() {
                Locale::En => format!(
                    "PDF container exceeded decompression or object-graph safety bounds: {detail}"
                ),
                Locale::Ro => format!(
                    "Containerul PDF a depășit limitele de securitate la decomprimare sau parcurgere: {detail}"
                ),
            };
            acc.fatal_clause(
                "PDFA3-6.1-RESOURCE-LIMIT",
                "ISO 19005-3:6.1",
                "PDF Streams / ObjStm",
                msg,
            );
            None
        }
        Err(PdfLoadError::Parse(detail)) => {
            let msg = match acc.locale() {
                Locale::En => {
                    format!("PDF cross-reference table or object syntax is unreadable: {detail}")
                }
                Locale::Ro => {
                    format!("Tabela XRef sau sintaxa obiectelor PDF nu poate fi citită: {detail}")
                }
            };
            fail_clause_hint(
                acc,
                "PDFA3-6.1-PARSE",
                "ISO 19005-3:6.1",
                "PDF Trailer / XRef",
                msg,
                "Verify the file is a non-corrupted, unencrypted PDF document.",
                "Verificați că fișierul este un document PDF necorupt și necriptat.",
            );
            None
        }
    }
}

fn check_header_and_binary_comment(bytes: &[u8], acc: &mut StageAcc) {
    let header_window = &bytes[..bytes.len().min(1024)];
    let Some(pos) = header_window.windows(5).position(|w| w == b"%PDF-") else {
        acc.fail_clause(
            "PDFA3-6.1.2-HEADER",
            "ISO 19005-3:6.1.2",
            "Byte offset 0",
            "File does not begin with a valid %PDF-x.y header.",
            "Fișierul nu începe cu un antet valid %PDF-x.y.",
        );
        return;
    };
    if pos > 0 {
        let msg = match acc.locale() {
            Locale::En => format!(
                "%PDF- header is preceded by {pos} extraneous byte(s); ISO 19005-3 requires byte offset 0."
            ),
            Locale::Ro => format!(
                "Antetul %PDF- este precedat de {pos} octeți suplimentari; ISO 19005-3 impune poziția 0."
            ),
        };
        acc.warn_clause(
            "PDFA3-6.1.2-LEADING-BYTES",
            "ISO 19005-3:6.1.2",
            format!("Byte offset {pos}"),
            msg,
        );
    } else {
        acc.pass();
    }

    let after = &bytes[pos..bytes.len().min(pos + 64)];
    if let Some(nl) = after.iter().position(|&b| b == b'\n' || b == b'\r') {
        let rest = after[nl..]
            .strip_prefix(b"\r\n")
            .or_else(|| after[nl..].strip_prefix(b"\n"))
            .or_else(|| after[nl..].strip_prefix(b"\r"))
            .unwrap_or(&after[nl..]);
        if let Some(body_rest) = rest.strip_prefix(b"%") {
            let end = body_rest
                .iter()
                .position(|&b| b == b'\r' || b == b'\n')
                .unwrap_or(body_rest.len());
            let high_bits = body_rest[..end].iter().filter(|&&b| b >= 0x80).count();
            if high_bits >= 4 {
                acc.pass();
            } else {
                let msg = match acc.locale() {
                    Locale::En => format!(
                        "PDF header binary comment contains {high_bits} high-bit (>0x7F) byte(s); ISO 19005-3 requires at least 4."
                    ),
                    Locale::Ro => format!(
                        "Comentariul binar din antetul PDF conține {high_bits} octeți >0x7F; ISO 19005-3 cere cel puțin 4."
                    ),
                };
                acc.warn_clause(
                    "PDFA3-6.1.2-BINARY-COMMENT",
                    "ISO 19005-3:6.1.2",
                    "Header line 2",
                    msg,
                );
            }
        } else {
            let msg = acc
                .tr(
                    "Missing binary marker comment line immediately after %PDF- header (ISO 19005-3 Clause 6.1.2).",
                    "Lipsește linia de comentariu binar imediat după antetul %PDF- (ISO 19005-3 Clauza 6.1.2).",
                )
                .to_owned();
            acc.warn_clause(
                "PDFA3-6.1.2-BINARY-COMMENT",
                "ISO 19005-3:6.1.2",
                "Header line 2",
                msg,
            );
        }
    }
}

fn check_trailer_id(doc: &Document, acc: &mut StageAcc) {
    match deref_key(doc, &doc.trailer, b"ID") {
        Some((_, Object::Array(arr))) if arr.len() == 2 => {
            if arr
                .iter()
                .all(|i| matches!(i, Object::String(b, _) if !b.is_empty()))
            {
                acc.pass();
            } else {
                acc.fail_clause(
                    "PDFA3-6.1.3-ID-EMPTY",
                    "ISO 19005-3:6.1.3",
                    "Trailer /ID",
                    "Trailer /ID array must contain two non-empty byte strings.",
                    "Vectorul /ID din Trailer trebuie să conțină două șiruri de octeți nevide.",
                );
            }
        }
        _ => acc.fail_clause(
            "PDFA3-6.1.3-ID-MISSING",
            "ISO 19005-3:6.1.3",
            "Trailer /ID",
            "Trailer dictionary is missing a two-element /ID file identifier array required by ISO 19005-3 Clause 6.1.3.",
            "Dicționarul Trailer nu conține vectorul /ID cu două elemente cerut de ISO 19005-3 Clauza 6.1.3.",
        ),
    }
}

fn check_streams_and_filters(doc: &Document, acc: &mut StageAcc) {
    let (mut bad_filter, mut external) = (false, false);
    for (&(n, g), obj) in &doc.objects {
        let Object::Stream(stream) = obj else {
            continue;
        };
        let loc = format!("Object {n} {g} R");
        if [b"F".as_slice(), b"FFilter", b"FDecodeParms"]
            .iter()
            .any(|k| stream.dict.has(k))
        {
            external = true;
            acc.fail_clause(
                "PDFA3-6.1.7-EXTERNAL-STREAM",
                "ISO 19005-3:6.1.7",
                &loc,
                "Stream dictionary references external file content (/F, /FFilter, or /FDecodeParms), prohibited in PDF/A-3.",
                "Dicționarul fluxului face referire la un fișier extern (/F, /FFilter, /FDecodeParms), interzis în PDF/A-3.",
            );
        }
        for filter in extract_filter_names(doc, &stream.dict) {
            if filter == b"LZWDecode" || filter == b"LZW" {
                bad_filter = true;
                let msg = acc
                    .tr(
                        "Stream uses the LZWDecode filter, which is forbidden in PDF/A-3.",
                        "Fluxul utilizează filtrul LZWDecode, interzis în PDF/A-3.",
                    )
                    .to_owned();
                fail_clause_hint(
                    acc,
                    "PDFA3-6.1.8-LZW",
                    "ISO 19005-3:6.1.8",
                    &loc,
                    msg,
                    "Use FlateDecode instead of LZWDecode.",
                    "Utilizați FlateDecode în loc de LZWDecode.",
                );
            } else if filter == b"Crypt" {
                bad_filter = true;
                acc.fail_clause(
                    "PDFA3-6.1.8-CRYPT",
                    "ISO 19005-3:6.1.8",
                    &loc,
                    "Stream uses the Crypt filter, which is forbidden in PDF/A-3.",
                    "Fluxul utilizează filtrul Crypt, interzis în PDF/A-3.",
                );
            }
        }
    }
    if !external {
        acc.pass();
    }
    if !bad_filter {
        acc.pass();
    }
}

fn extract_filter_names<'a>(doc: &'a Document, dict: &'a Dictionary) -> Vec<&'a [u8]> {
    match deref_key(doc, dict, b"Filter").map(|(_, o)| o) {
        Some(Object::Name(n)) => vec![n.as_slice()],
        Some(Object::Array(arr)) => arr
            .iter()
            .filter_map(|i| match doc.dereference(i).ok()?.1 {
                Object::Name(n) => Some(n.as_slice()),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

pub(super) fn fail_clause_hint(
    acc: &mut StageAcc,
    code: &str,
    clause: &str,
    loc: impl Into<String>,
    message: impl Into<String>,
    hint_en: &str,
    hint_ro: &str,
) {
    let hint = acc.tr(hint_en, hint_ro).to_owned();
    acc.record(
        StageFinding::fatal(code, message)
            .term(clause)
            .location(loc)
            .hint(hint),
    );
}

pub(super) fn deref_key<'a>(
    doc: &'a Document,
    dict: &'a Dictionary,
    key: &[u8],
) -> Option<(Option<ObjectId>, &'a Object)> {
    dict.get(key).and_then(|o| doc.dereference(o)).ok()
}

pub(super) fn obj_loc(id: Option<ObjectId>, role: &str, fallback: &str) -> String {
    id.map_or_else(
        || fallback.to_owned(),
        |(n, g)| format!("Object {n} {g} R ({role})"),
    )
}
