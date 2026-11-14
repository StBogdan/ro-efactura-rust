//! Pure-Rust Binary ISO 19005-3 (`PDF/A-3`) Inspector & `efactura.xml` Attachment Extractor.
//!
//! Works strictly in memory (native and `wasm32`) via `lopdf`, `ttf-parser`, `roxmltree`,
//! and `en16931_formats::zugferd`, supporting Romanian `efactura.xml` as well as `factur-x.xml`,
//! `zugferd-invoice.xml`, and `xrechnung.xml`.

use std::collections::HashSet;

use en16931_formats::zugferd::{self, Divergence};
use lopdf::{Dictionary, Document, Object, ObjectId, Stream};

use crate::types::{EngineId, EngineReport, EngineStatus, Locale, Severity, Violation};

pub(crate) const ENGINE_ID: EngineId = EngineId::Pdfa3Iso19005;
pub(crate) const ENGINE_NAME: &str = "ISO 19005-3 (PDF/A-3) & efactura.xml Inspector";
pub(crate) const ENGINE_VERSION: &str = "ISO 19005-3:2012";

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

struct CheckCounter {
    locale: Locale,
    evaluated: usize,
    failed: usize,
    warnings: usize,
    violations: Vec<Violation>,
}

impl CheckCounter {
    fn new(locale: Locale) -> Self {
        Self {
            locale,
            evaluated: 0,
            failed: 0,
            warnings: 0,
            violations: Vec::new(),
        }
    }

    fn pass(&mut self) {
        self.evaluated += 1;
    }

    #[allow(clippy::too_many_arguments)]
    fn push(
        &mut self,
        severity: Severity,
        code: &str,
        clause: &str,
        loc: impl Into<String>,
        msg_en: impl Into<String>,
        msg_ro: impl Into<String>,
        hint: Option<(&str, &str)>,
    ) {
        self.evaluated += 1;
        if severity == Severity::Fatal {
            self.failed += 1;
        } else {
            self.warnings += 1;
        }
        let (message, hint_msg) = match self.locale {
            Locale::En => (msg_en.into(), hint.map(|(en, _)| en.to_owned())),
            Locale::Ro => (msg_ro.into(), hint.map(|(_, ro)| ro.to_owned())),
        };
        self.violations.push(Violation {
            code: code.to_string(),
            rule_sub_id: None,
            engine: ENGINE_ID,
            severity,
            business_term: Some(clause.to_string()),
            message,
            location: Some(loc.into()),
            expected: None,
            actual: None,
            hint: hint_msg,
        });
    }

    fn fail(
        &mut self,
        code: &str,
        clause: &str,
        loc: impl Into<String>,
        en: impl Into<String>,
        ro: impl Into<String>,
    ) {
        self.push(Severity::Fatal, code, clause, loc, en, ro, None);
    }

    #[allow(clippy::too_many_arguments)]
    fn fail_hint(
        &mut self,
        code: &str,
        clause: &str,
        loc: impl Into<String>,
        en: impl Into<String>,
        ro: impl Into<String>,
        hint_en: &str,
        hint_ro: &str,
    ) {
        self.push(
            Severity::Fatal,
            code,
            clause,
            loc,
            en,
            ro,
            Some((hint_en, hint_ro)),
        );
    }

    fn warn(
        &mut self,
        code: &str,
        clause: &str,
        loc: impl Into<String>,
        en: impl Into<String>,
        ro: impl Into<String>,
    ) {
        self.push(Severity::Warning, code, clause, loc, en, ro, None);
    }

    fn into_report(self) -> (EngineReport, Vec<Violation>) {
        (
            EngineReport {
                id: ENGINE_ID,
                name: ENGINE_NAME.to_string(),
                version: ENGINE_VERSION.to_string(),
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

/// Extracts the embedded XML e-invoice (`efactura.xml`, `factur-x.xml`, `zugferd-invoice.xml`,
/// `xrechnung.xml`, or fallback `.xml` attachment) from raw PDF bytes in memory.
pub fn extract_xml_from_pdf(pdf_bytes: &[u8]) -> Result<ExtractedPdfXml, String> {
    let limits = zugferd::Limits::default();
    if let Ok(ext) = zugferd::extract(pdf_bytes, &limits) {
        return Ok(ExtractedPdfXml {
            filename: ext.filename,
            xml_bytes: ext.xml,
        });
    }
    let files = zugferd::embedded_files(pdf_bytes, &limits)
        .map_err(|e| format!("Failed parsing PDF container: {e}"))?;
    find_efactura_or_xml(files).ok_or_else(|| {
        "No embedded XML e-invoice (efactura.xml / factur-x.xml) found in PDF container."
            .to_string()
    })
}

fn find_efactura_or_xml(
    mut files: std::collections::BTreeMap<String, Vec<u8>>,
) -> Option<ExtractedPdfXml> {
    let key = files
        .keys()
        .find(|k| k.eq_ignore_ascii_case("efactura.xml"))
        .or_else(|| {
            files
                .keys()
                .find(|k| k.to_ascii_lowercase().ends_with(".xml"))
        })?
        .clone();
    let xml_bytes = files.remove(&key)?;
    Some(ExtractedPdfXml {
        filename: key,
        xml_bytes,
    })
}

/// Inspects a PDF container against ISO 19005-3 (PDF/A-3) clauses and extracts its embedded XML invoice.
#[must_use]
pub fn inspect_pdfa3(bytes: &[u8], run_pdfa_checks: bool, locale: Locale) -> PdfInspectionOutcome {
    if !run_pdfa_checks {
        return PdfInspectionOutcome {
            report: EngineReport::skipped(
                ENGINE_ID,
                ENGINE_NAME,
                ENGINE_VERSION,
                "Disabled by ValidateOptions",
            ),
            violations: Vec::new(),
            extracted_xml: extract_xml_from_pdf(bytes).ok(),
        };
    }

    let limits = zugferd::Limits::default();
    let zugferd_res = zugferd::extract(bytes, &limits);
    let mut checks = CheckCounter::new(locale);
    check_header_and_binary_comment(bytes, &mut checks);

    let doc = match Document::load_mem(bytes) {
        Ok(d) => {
            checks.pass();
            d
        }
        Err(err) => {
            checks.fail_hint(
                "PDFA3-6.1-PARSE",
                "ISO 19005-3:6.1",
                "PDF Trailer / XRef",
                format!("PDF cross-reference table or object syntax is unreadable: {err}"),
                format!("Tabela XRef sau sintaxa obiectelor PDF nu poate fi citită: {err}"),
                "Verify the file is a non-corrupted, unencrypted PDF document.",
                "Verificați că fișierul este un document PDF necorupt și necriptat.",
            );
            let (report, violations) = checks.into_report();
            return PdfInspectionOutcome {
                report,
                violations,
                extracted_xml: None,
            };
        }
    };

    if doc.trailer.has(b"Encrypt") || doc.encryption_state.is_some() {
        checks.fail_hint(
            "PDFA3-6.1.4-ENCRYPT",
            "ISO 19005-3:6.1.4",
            "Trailer /Encrypt",
            "Trailer dictionary contains an /Encrypt entry; encryption is forbidden in PDF/A-3.",
            "Dicționarul Trailer conține intrarea /Encrypt; criptarea este interzisă în PDF/A-3.",
            "Export the PDF without password or permissions encryption.",
            "Exportați documentul PDF fără parolă sau restricții de criptare.",
        );
    } else {
        checks.pass();
    }

    check_trailer_id(&doc, &mut checks);
    check_streams_and_filters(&doc, &mut checks);

    let catalog = match doc.catalog() {
        Ok(c) => {
            checks.pass();
            c
        }
        Err(err) => {
            checks.fail(
                "PDFA3-6.1-CATALOG",
                "ISO 19005-3:6.1",
                "Trailer /Root",
                format!("PDF Document Catalog (/Root) is missing or invalid: {err}"),
                format!("Catalogul documentului PDF (/Root) lipsește sau este invalid: {err}"),
            );
            let (report, violations) = checks.into_report();
            return PdfInspectionOutcome {
                report,
                violations,
                extracted_xml: None,
            };
        }
    };

    check_output_intents(&doc, catalog, &mut checks);
    check_embedded_fonts(&doc, &mut checks);
    let xmp_doc_filename = check_xmp_metadata(&doc, catalog, &mut checks);
    let extracted_xml = check_associated_files_and_extract(
        bytes,
        &doc,
        catalog,
        &zugferd_res,
        xmp_doc_filename.as_deref(),
        &limits,
        &mut checks,
    );

    let (report, violations) = checks.into_report();
    PdfInspectionOutcome {
        report,
        violations,
        extracted_xml,
    }
}

fn check_header_and_binary_comment(bytes: &[u8], checks: &mut CheckCounter) {
    let header_window = &bytes[..bytes.len().min(1024)];
    let Some(pos) = header_window.windows(5).position(|w| w == b"%PDF-") else {
        checks.fail(
            "PDFA3-6.1.2-HEADER",
            "ISO 19005-3:6.1.2",
            "Byte offset 0",
            "File does not begin with a valid %PDF-x.y header.",
            "Fișierul nu începe cu un antet valid %PDF-x.y.",
        );
        return;
    };
    if pos > 0 {
        checks.warn(
            "PDFA3-6.1.2-LEADING-BYTES",
            "ISO 19005-3:6.1.2",
            format!("Byte offset {pos}"),
            format!("%PDF- header is preceded by {pos} extraneous byte(s); ISO 19005-3 requires byte offset 0."),
            format!("Antetul %PDF- este precedat de {pos} octeți suplimentari; ISO 19005-3 impune poziția 0."),
        );
    } else {
        checks.pass();
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
                checks.pass();
            } else {
                checks.warn(
                    "PDFA3-6.1.2-BINARY-COMMENT",
                    "ISO 19005-3:6.1.2",
                    "Header line 2",
                    format!("PDF header binary comment contains {high_bits} high-bit (>0x7F) byte(s); ISO 19005-3 requires at least 4."),
                    format!("Comentariul binar din antetul PDF conține {high_bits} octeți >0x7F; ISO 19005-3 cere cel puțin 4."),
                );
            }
        } else {
            checks.warn(
                "PDFA3-6.1.2-BINARY-COMMENT",
                "ISO 19005-3:6.1.2",
                "Header line 2",
                "Missing binary marker comment line immediately after %PDF- header (ISO 19005-3 Clause 6.1.2).",
                "Lipsește linia de comentariu binar imediat după antetul %PDF- (ISO 19005-3 Clauza 6.1.2).",
            );
        }
    }
}

fn check_trailer_id(doc: &Document, checks: &mut CheckCounter) {
    match deref_key(doc, &doc.trailer, b"ID") {
        Some((_, Object::Array(arr))) if arr.len() == 2 => {
            if arr
                .iter()
                .all(|i| matches!(i, Object::String(b, _) if !b.is_empty()))
            {
                checks.pass();
            } else {
                checks.fail(
                    "PDFA3-6.1.3-ID-EMPTY",
                    "ISO 19005-3:6.1.3",
                    "Trailer /ID",
                    "Trailer /ID array must contain two non-empty byte strings.",
                    "Vectorul /ID din Trailer trebuie să conțină două șiruri de octeți nevide.",
                );
            }
        }
        _ => checks.fail(
            "PDFA3-6.1.3-ID-MISSING",
            "ISO 19005-3:6.1.3",
            "Trailer /ID",
            "Trailer dictionary is missing a two-element /ID file identifier array required by ISO 19005-3 Clause 6.1.3.",
            "Dicționarul Trailer nu conține vectorul /ID cu două elemente cerut de ISO 19005-3 Clauza 6.1.3.",
        ),
    }
}

fn check_streams_and_filters(doc: &Document, checks: &mut CheckCounter) {
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
            checks.fail(
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
                checks.fail_hint(
                    "PDFA3-6.1.8-LZW",
                    "ISO 19005-3:6.1.8",
                    &loc,
                    "Stream uses the LZWDecode filter, which is forbidden in PDF/A-3.",
                    "Fluxul utilizează filtrul LZWDecode, interzis în PDF/A-3.",
                    "Use FlateDecode instead of LZWDecode.",
                    "Utilizați FlateDecode în loc de LZWDecode.",
                );
            } else if filter == b"Crypt" {
                bad_filter = true;
                checks.fail(
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
        checks.pass();
    }
    if !bad_filter {
        checks.pass();
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

fn check_output_intents(doc: &Document, catalog: &Dictionary, checks: &mut CheckCounter) {
    let Some((_, oi_obj)) = deref_key(doc, catalog, b"OutputIntents") else {
        checks.fail_hint(
            "PDFA3-6.2.2-OUTPUTINTENT-MISSING",
            "ISO 19005-3:6.2.2",
            "Catalog /OutputIntents",
            "Document Catalog has no /OutputIntents array; PDF/A-3 requires a GTS_PDFA1 output intent with an embedded ICC profile.",
            "Catalogul PDF nu conține /OutputIntents; PDF/A-3 necesită un profil de culoare ICC GTS_PDFA1 încorporat.",
            "Embed an sRGB or CMYK ICC profile stream under /OutputIntents with /S /GTS_PDFA1.",
            "Încorporați un profil ICC sRGB sau CMYK în /OutputIntents cu /S /GTS_PDFA1.",
        );
        return;
    };
    let Object::Array(intents) = oi_obj else {
        checks.fail(
            "PDFA3-6.2.2-OUTPUTINTENT-TYPE",
            "ISO 19005-3:6.2.2",
            "Catalog /OutputIntents",
            "Catalog /OutputIntents must be a PDF Array.",
            "Intrarea /OutputIntents din Catalog trebuie să fie de tip Array PDF.",
        );
        return;
    };

    let mut found_gts = false;
    for item in intents {
        let Ok((ref_id, Object::Dictionary(intent_dict))) = doc.dereference(item) else {
            continue;
        };
        if intent_dict
            .get(b"S")
            .and_then(Object::as_name)
            .unwrap_or(b"")
            != b"GTS_PDFA1"
        {
            continue;
        }
        found_gts = true;
        checks.pass();
        let loc = obj_loc(ref_id, "/OutputIntent", "Catalog /OutputIntents");
        match deref_key(doc, intent_dict, b"DestOutputProfile") {
            Some((icc_id, Object::Stream(icc_stream))) => {
                validate_icc_stream(icc_stream, &obj_loc(icc_id, "/DestOutputProfile", &loc), checks);
            }
            _ => checks.fail(
                "PDFA3-6.2.3-ICC-MISSING",
                "ISO 19005-3:6.2.3",
                loc,
                "GTS_PDFA1 OutputIntent dictionary lacks an embedded /DestOutputProfile ICC stream.",
                "Dicționarul OutputIntent GTS_PDFA1 nu conține fluxul ICC /DestOutputProfile.",
            ),
        }
    }
    if !found_gts {
        checks.fail(
            "PDFA3-6.2.2-GTS-PDFA1-MISSING",
            "ISO 19005-3:6.2.2",
            "Catalog /OutputIntents",
            "Catalog /OutputIntents array contains no entry with subtype /S /GTS_PDFA1.",
            "Vectorul /OutputIntents nu conține nicio intrare cu subtipul /S /GTS_PDFA1.",
        );
    }
}

fn validate_icc_stream(stream: &Stream, loc: &str, checks: &mut CheckCounter) {
    let Ok(bytes) = decode_stream_bytes(stream) else {
        checks.fail(
            "PDFA3-6.2.3-ICC-DECODE",
            "ISO 19005-3:6.2.3",
            loc,
            "Embedded /DestOutputProfile ICC stream failed to decompress.",
            "Fluxul ICC /DestOutputProfile nu a putut fi decomprimat.",
        );
        return;
    };
    if bytes.len() < 128 || &bytes[36..40] != b"acsp" {
        checks.fail(
            "PDFA3-6.2.3-ICC-SIGNATURE",
            "ISO 19005-3:6.2.3",
            loc,
            "Embedded /DestOutputProfile is not a valid ICC profile (missing 'acsp' signature).",
            "Profilul /DestOutputProfile nu este un profil ICC valid (lipsește semnătura 'acsp').",
        );
        return;
    }
    let major = bytes[8];
    if major > 4 {
        checks.fail(
            "PDFA3-6.2.3-ICC-VERSION",
            "ISO 19005-3:6.2.3",
            loc,
            format!("ICC profile version {major}.x exceeds maximum ICC v4 allowed by ISO 19005-3."),
            format!("Versiunea profilului ICC {major}.x depășește versiunea maximă v4 permisă de ISO 19005-3."),
        );
        return;
    }
    let expected_n: i64 = match (&bytes[12..16], &bytes[16..20]) {
        (b"scnr" | b"mntr" | b"prtr" | b"spac", b"GRAY") => 1,
        (b"scnr" | b"mntr" | b"prtr" | b"spac", b"RGB ") => 3,
        (b"scnr" | b"mntr" | b"prtr" | b"spac", b"CMYK") => 4,
        _ => {
            checks.fail(
                "PDFA3-6.2.3-ICC-COLORSPACE",
                "ISO 19005-3:6.2.3",
                loc,
                "ICC profile device class or color space is not valid for GTS_PDFA1.",
                "Clasa dispozitivului sau spațiul de culoare ICC nu este valid pentru GTS_PDFA1.",
            );
            return;
        }
    };
    match stream.dict.get(b"N").and_then(Object::as_i64) {
        Ok(n) if n == expected_n => checks.pass(),
        Ok(n) => checks.fail(
            "PDFA3-6.2.3-ICC-COMPONENTS",
            "ISO 19005-3:6.2.3",
            loc,
            format!("/DestOutputProfile declares /N {n}, expected /N {expected_n}."),
            format!(
                "/DestOutputProfile declară /N {n}, dar spațiul de culoare impune /N {expected_n}."
            ),
        ),
        Err(_) => checks.fail(
            "PDFA3-6.2.3-ICC-N-MISSING",
            "ISO 19005-3:6.2.3",
            loc,
            format!(
                "/DestOutputProfile is missing required /N {expected_n} color component count."
            ),
            format!(
                "/DestOutputProfile nu conține numărul obligatoriu de componente /N {expected_n}."
            ),
        ),
    }
}

fn check_embedded_fonts(doc: &Document, checks: &mut CheckCounter) {
    let mut visited: HashSet<ObjectId> = HashSet::new();
    for (&obj_id, obj) in &doc.objects {
        let Object::Dictionary(dict) = obj else {
            continue;
        };
        if dict
            .get(b"Type")
            .and_then(Object::as_name)
            .is_ok_and(|t| t == b"Font")
            && visited.insert(obj_id)
        {
            inspect_single_font(doc, dict, Some(obj_id), checks);
        }
        if let Some((_, Object::Dictionary(res))) = deref_key(doc, dict, b"Resources") {
            if let Some((_, Object::Dictionary(fonts))) = deref_key(doc, res, b"Font") {
                for (_, fref) in fonts.iter() {
                    if let Ok((fid, Object::Dictionary(fdict))) = doc.dereference(fref) {
                        if fid.is_none_or(|id| visited.insert(id)) {
                            inspect_single_font(doc, fdict, fid, checks);
                        }
                    }
                }
            }
        }
    }
    if visited.is_empty() {
        checks.pass();
    }
}

fn inspect_single_font(
    doc: &Document,
    font_dict: &Dictionary,
    obj_id: Option<ObjectId>,
    checks: &mut CheckCounter,
) {
    let subtype = font_dict
        .get(b"Subtype")
        .and_then(Object::as_name)
        .unwrap_or(b"");
    let base_font = font_dict
        .get(b"BaseFont")
        .and_then(Object::as_name)
        .map(|b| String::from_utf8_lossy(b).into_owned())
        .unwrap_or_else(|_| "<unnamed>".to_string());
    let fallback = format!("Font /{base_font}");
    let loc = obj_loc(obj_id, &fallback, &fallback);

    if subtype == b"Type3" {
        checks.pass();
    } else if subtype == b"Type0" {
        if let Some((_, Object::Array(descendants))) = deref_key(doc, font_dict, b"DescendantFonts")
        {
            for desc in descendants {
                if let Ok((did, Object::Dictionary(cid))) = doc.dereference(desc) {
                    verify_font_descriptor(doc, cid, &base_font, did.or(obj_id), checks);
                }
            }
        } else {
            checks.fail(
                "PDFA3-6.2.11-TYPE0-DESCENDANT",
                "ISO 19005-3:6.2.11.4",
                loc,
                format!("Type0 composite font /{base_font} has no /DescendantFonts array."),
                format!("Fontul compozit Type0 /{base_font} nu conține vectorul /DescendantFonts."),
            );
        }
    } else {
        verify_font_descriptor(doc, font_dict, &base_font, obj_id, checks);
    }
}

fn verify_font_descriptor(
    doc: &Document,
    font_dict: &Dictionary,
    base_font: &str,
    obj_id: Option<ObjectId>,
    checks: &mut CheckCounter,
) {
    let fallback = format!("Font /{base_font}");
    let loc = obj_loc(obj_id, &fallback, &fallback);
    let Some((_, Object::Dictionary(fd))) = deref_key(doc, font_dict, b"FontDescriptor") else {
        checks.fail_hint(
            "PDFA3-6.2.11-FONT-NOT-EMBEDDED",
            "ISO 19005-3:6.2.11.4",
            loc,
            format!("Font /{base_font} has no /FontDescriptor and its glyph program is not embedded; ISO 19005-3 requires embedding every used font."),
            format!("Fontul /{base_font} nu are /FontDescriptor și nu este încorporat; ISO 19005-3 impune încorporarea tuturor fonturilor utilizate."),
            "Embed a TrueType/OpenType font subset instead of unembedded Standard 14 Type1 fonts.",
            "Încorporați un subset de font TrueType/OpenType în locul fonturilor Standard 14 neîncorporate.",
        );
        return;
    };

    let entry = [b"FontFile".as_slice(), b"FontFile2", b"FontFile3"]
        .into_iter()
        .find_map(|k| match deref_key(doc, fd, k)? {
            (sid, Object::Stream(s)) => Some((k, sid, s)),
            _ => None,
        });
    let Some((key, sid, stream)) = entry else {
        checks.fail(
            "PDFA3-6.2.11-FONTFILE-MISSING",
            "ISO 19005-3:6.2.11.4",
            loc,
            format!("FontDescriptor for /{base_font} contains none of /FontFile, /FontFile2, or /FontFile3."),
            format!("FontDescriptor pentru /{base_font} nu conține /FontFile, /FontFile2 sau /FontFile3."),
        );
        return;
    };

    let sloc = obj_loc(sid, &fallback, &loc);
    let Ok(font_bytes) = decode_stream_bytes(stream) else {
        checks.fail(
            "PDFA3-6.2.11-FONTFILE-DECODE",
            "ISO 19005-3:6.2.11.4",
            sloc,
            format!("Embedded font stream for /{base_font} could not be decompressed."),
            format!("Fluxul fontului încorporat /{base_font} nu a putut fi decomprimat."),
        );
        return;
    };
    if font_bytes.is_empty() {
        checks.fail(
            "PDFA3-6.2.11-FONTFILE-EMPTY",
            "ISO 19005-3:6.2.11.4",
            sloc,
            format!("Embedded font stream for /{base_font} is empty (0 bytes)."),
            format!("Fluxul fontului încorporat /{base_font} este gol (0 octeți)."),
        );
        return;
    }

    let is_otf = key == b"FontFile2"
        || (key == b"FontFile3"
            && stream
                .dict
                .get(b"Subtype")
                .and_then(Object::as_name)
                .is_ok_and(|s| s == b"OpenType"));
    if is_otf {
        match ttf_parser::Face::parse(&font_bytes, 0) {
            Ok(f) if f.number_of_glyphs() > 0 => checks.pass(),
            Ok(_) => checks.fail(
                "PDFA3-6.2.11-FONT-ZERO-GLYPHS",
                "ISO 19005-3:6.2.11.4",
                sloc,
                format!("Embedded TrueType/OpenType font /{base_font} contains 0 glyphs."),
                format!("Fontul TrueType/OpenType /{base_font} conține 0 glife."),
            ),
            Err(err) => checks.fail(
                "PDFA3-6.2.11-FONT-CORRUPT",
                "ISO 19005-3:6.2.11.4",
                sloc,
                format!("Embedded TrueType/OpenType font /{base_font} failed SFNT parsing: {err}"),
                format!("Fontul TrueType/OpenType /{base_font} este corupt SFNT: {err}"),
            ),
        }
    } else {
        checks.pass();
    }
}

fn check_xmp_metadata(
    doc: &Document,
    catalog: &Dictionary,
    checks: &mut CheckCounter,
) -> Option<String> {
    let Some((meta_id, Object::Stream(meta_stream))) = deref_key(doc, catalog, b"Metadata") else {
        checks.fail(
            "PDFA3-6.6.2-METADATA-MISSING",
            "ISO 19005-3:6.6.2.1",
            "Catalog /Metadata",
            "Document Catalog lacks an XMP /Metadata stream required by ISO 19005-3.",
            "Catalogul PDF nu conține fluxul XMP /Metadata cerut de ISO 19005-3.",
        );
        return None;
    };
    let loc = obj_loc(meta_id, "/Metadata", "Catalog /Metadata");

    if meta_stream.dict.has(b"Filter") {
        checks.fail(
            "PDFA3-6.6.2-METADATA-FILTERED",
            "ISO 19005-3:6.6.2.1",
            &loc,
            "Catalog /Metadata XMP stream is compressed (/Filter present); ISO 19005-3 requires unfiltered XMP.",
            "Fluxul XMP /Metadata este comprimat (/Filter prezent); ISO 19005-3 cere XMP necomprimat.",
        );
    } else {
        checks.pass();
    }

    let Ok(raw_xmp) = decode_stream_bytes(meta_stream) else {
        checks.fail(
            "PDFA3-6.6.2-METADATA-UNREADABLE",
            "ISO 19005-3:6.6.2.1",
            loc,
            "Catalog /Metadata XMP stream could not be decoded.",
            "Fluxul XMP /Metadata nu a putut fi decodat.",
        );
        return None;
    };
    let Ok(xmp_str) = std::str::from_utf8(&raw_xmp) else {
        checks.fail(
            "PDFA3-6.6.2-METADATA-UTF8",
            "ISO 19005-3:6.6.2.1",
            loc,
            "Catalog /Metadata XMP stream is not valid UTF-8.",
            "Fluxul XMP /Metadata nu este codificat valid UTF-8.",
        );
        return None;
    };
    let Ok(xml) = roxmltree::Document::parse(xmp_str.trim_matches('\0').trim()) else {
        checks.fail(
            "PDFA3-6.6.2-METADATA-XML",
            "ISO 19005-3:6.6.2.1",
            loc,
            "Catalog /Metadata XMP packet is not well-formed XML.",
            "Pachetul XMP /Metadata nu este XML bine format.",
        );
        return None;
    };

    const PDFAID_NS: &str = "http://www.aiim.org/pdfa/ns/id/";
    const PDFA_EXT_NS: &str = "http://www.aiim.org/pdfa/ns/extension/";

    let (mut part, mut conf, mut doc_fname) = (None, None, None);
    let (mut has_fx, mut has_ext) = (false, false);

    for node in xml.descendants().filter(|n| n.is_element()) {
        let ns = node.tag_name().namespace().unwrap_or("");
        let name = node.tag_name().name();
        let text = || {
            node.text()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
        };

        if ns == PDFAID_NS && name == "part" {
            part = node.text().map(|s| s.trim().to_string());
        } else if ns == PDFAID_NS && name == "conformance" {
            conf = node.text().map(|s| s.trim().to_string());
        } else if ns == PDFA_EXT_NS && name == "schemas" {
            has_ext = true;
        } else if ns.contains("factur-x") || ns.contains("zugferd") {
            has_fx = true;
            if name == "DocumentFileName" && doc_fname.is_none() {
                doc_fname = text();
            }
        }
        for attr in node.attributes() {
            let ans = attr.namespace().unwrap_or("");
            let val = attr.value().trim();
            if ans == PDFAID_NS && attr.name() == "part" {
                part = Some(val.to_string());
            } else if ans == PDFAID_NS && attr.name() == "conformance" {
                conf = Some(val.to_string());
            } else if ans.contains("factur-x") || ans.contains("zugferd") {
                has_fx = true;
                if attr.name() == "DocumentFileName" && !val.is_empty() && doc_fname.is_none() {
                    doc_fname = Some(val.to_string());
                }
            }
        }
    }

    match part.as_deref() {
        Some("3") => checks.pass(),
        Some(other) => checks.fail(
            "PDFA3-6.6.4-PDFAID-PART",
            "ISO 19005-3:6.6.4",
            &loc,
            format!("XMP pdfaid:part is '{other}', expected '3' for PDF/A-3 hybrid e-invoices."),
            format!("Proprietatea XMP pdfaid:part este '{other}', dar pentru PDF/A-3 trebuie să fie '3'."),
        ),
        None => checks.fail(
            "PDFA3-6.6.4-PDFAID-MISSING",
            "ISO 19005-3:6.6.4",
            &loc,
            "XMP metadata lacks pdfaid:part='3' identification property.",
            "Metadatele XMP nu conțin proprietatea pdfaid:part='3'.",
        ),
    }

    match conf.as_deref() {
        Some("A" | "B" | "U") => checks.pass(),
        Some(other) => checks.fail(
            "PDFA3-6.6.4-PDFAID-CONFORMANCE",
            "ISO 19005-3:6.6.4",
            &loc,
            format!("XMP pdfaid:conformance '{other}' is invalid; must be 'A', 'B', or 'U'."),
            format!("Valoarea XMP pdfaid:conformance '{other}' este invalidă; permis: 'A', 'B' sau 'U'."),
        ),
        None => checks.fail(
            "PDFA3-6.6.4-PDFAID-CONFORMANCE-MISSING",
            "ISO 19005-3:6.6.4",
            &loc,
            "XMP metadata lacks pdfaid:conformance ('A', 'B', or 'U').",
            "Metadatele XMP nu conțin pdfaid:conformance ('A', 'B' sau 'U').",
        ),
    }

    if has_fx && !has_ext {
        checks.fail(
            "PDFA3-6.6.2.3-EXTENSION-SCHEMA",
            "ISO 19005-3:6.6.2.3",
            loc,
            "XMP contains custom Factur-X/ZUGFeRD properties without pdfaExtension:schemas.",
            "Metadatele XMP conțin proprietăți Factur-X/ZUGFeRD fără declarația pdfaExtension:schemas.",
        );
    } else {
        checks.pass();
    }
    doc_fname
}

fn check_associated_files_and_extract(
    raw_pdf: &[u8],
    doc: &Document,
    catalog: &Dictionary,
    zugferd_res: &Result<zugferd::Extracted, zugferd::Error>,
    xmp_doc_filename: Option<&str>,
    limits: &zugferd::Limits,
    checks: &mut CheckCounter,
) -> Option<ExtractedPdfXml> {
    let has_af = match deref_key(doc, catalog, b"AF") {
        Some((_, Object::Array(af_arr))) if !af_arr.is_empty() => {
            checks.pass();
            check_filespec_mime_and_relationship(doc, af_arr, checks);
            true
        }
        _ => {
            checks.fail_hint(
                "PDFA3-6.8-AF-MISSING",
                "ISO 19005-3:6.8",
                "Catalog /AF",
                "Document Catalog is missing a non-empty /AF (Associated Files) array required by ISO 19005-3 Clause 6.8.",
                "Catalogul PDF nu conține vectorul /AF (Associated Files) cerut de ISO 19005-3 Clauza 6.8.",
                "Reference the efactura.xml Filespec dictionary from both Catalog /AF and /Names/EmbeddedFiles.",
                "Referențiați dicționarul Filespec pentru efactura.xml atât în Catalog /AF, cât și în /Names/EmbeddedFiles.",
            );
            false
        }
    };

    match zugferd_res {
        Ok(extracted) => {
            if extracted.divergence.is_empty() {
                checks.pass();
            } else {
                let loc = format!("EmbeddedFile ({})", extracted.filename);
                for div in &extracted.divergence {
                    let msg = div.to_string();
                    match div {
                        Divergence::Profile { .. }
                        | Divergence::Filename { .. }
                        | Divergence::Relationship { .. }
                        | Divergence::NoXmp
                        | Divergence::NotAssociated
                        | Divergence::NotInEmbeddedFiles
                        | Divergence::NoRelationship
                        | Divergence::NotPdfA3 { .. } => {
                            checks.fail(
                                "PDFA3-6.8-HYBRID-DIVERGENCE",
                                "ISO 19005-3:6.8",
                                &loc,
                                &msg,
                                &msg,
                            );
                        }
                        _ => {
                            checks.warn(
                                "PDFA3-6.8-HYBRID-NOTICE",
                                "ISO 19005-3:6.8",
                                &loc,
                                &msg,
                                &msg,
                            );
                        }
                    }
                }
            }
            Some(ExtractedPdfXml {
                filename: extracted.filename.clone(),
                xml_bytes: extracted.xml.clone(),
            })
        }
        Err(err) => {
            let embedded = zugferd::embedded_files(raw_pdf, limits)
                .ok()
                .and_then(find_efactura_or_xml);
            if has_af {
                if let Some(ext) = &embedded {
                    if ext.filename.eq_ignore_ascii_case("efactura.xml") {
                        if let Some(xmp_f) =
                            xmp_doc_filename.filter(|x| !x.eq_ignore_ascii_case(&ext.filename))
                        {
                            checks.fail(
                                "PDFA3-6.8-HYBRID-DIVERGENCE",
                                "ISO 19005-3:6.8",
                                format!("EmbeddedFile ({})", ext.filename),
                                format!("XMP fx:DocumentFileName declares '{xmp_f}', but associated invoice attachment is '{}'.", ext.filename),
                                format!("Metadatele XMP fx:DocumentFileName declară '{xmp_f}', dar fișierul atașat este '{}'.", ext.filename),
                            );
                        } else {
                            checks.pass();
                        }
                        return embedded;
                    }
                }
            }
            checks.fail_hint(
                "PDFA3-6.8-NO-INVOICE-PAYLOAD",
                "ISO 19005-3:6.8",
                "Catalog /Names/EmbeddedFiles",
                format!("Hybrid PDF invoice extraction failed: {err}"),
                format!("Extragerea facturii XML din PDF-ul hibrid a eșuat: {err}"),
                "Attach the XML invoice as efactura.xml, factur-x.xml, zugferd-invoice.xml, or xrechnung.xml.",
                "Atașați factura XML cu numele efactura.xml, factur-x.xml, zugferd-invoice.xml sau xrechnung.xml.",
            );
            embedded
        }
    }
}

fn check_filespec_mime_and_relationship(
    doc: &Document,
    af_arr: &[Object],
    checks: &mut CheckCounter,
) {
    for item in af_arr {
        let Ok((fs_id, Object::Dictionary(fs_dict))) = doc.dereference(item) else {
            continue;
        };
        let loc = obj_loc(fs_id, "/Filespec", "Catalog /AF Filespec");
        match fs_dict.get(b"AFRelationship").and_then(Object::as_name) {
            Ok(b"Alternative" | b"Data" | b"Source" | b"Supplement" | b"Unspecified") => {
                checks.pass();
            }
            Ok(other) => {
                let s = String::from_utf8_lossy(other);
                checks.fail(
                    "PDFA3-6.8-AFRELATIONSHIP-INVALID",
                    "ISO 19005-3:6.8",
                    &loc,
                    format!("Filespec /AFRelationship '/{s}' is not valid in ISO 19005-3."),
                    format!("Valoarea /AFRelationship '/{s}' din Filespec nu este permisă de ISO 19005-3."),
                );
            }
            Err(_) => checks.fail(
                "PDFA3-6.8-AFRELATIONSHIP-MISSING",
                "ISO 19005-3:6.8",
                &loc,
                "Associated file specification dictionary lacks a required /AFRelationship entry.",
                "Dicționarul Filespec nu conține intrarea obligatorie /AFRelationship.",
            ),
        }

        if let Some((_, Object::Dictionary(ef_dict))) = deref_key(doc, fs_dict, b"EF") {
            let ef = deref_key(doc, ef_dict, b"F").or_else(|| deref_key(doc, ef_dict, b"UF"));
            if let Some((ef_id, Object::Stream(ef_stream))) = ef {
                let ef_loc = obj_loc(ef_id, "/EmbeddedFile", &loc);
                match ef_stream.dict.get(b"Subtype").and_then(Object::as_name) {
                    Ok(b"text/xml" | b"application/xml") => checks.pass(),
                    Ok(other) => {
                        let s = String::from_utf8_lossy(other);
                        checks.warn(
                            "PDFA3-6.8-MIME-SUBTYPE",
                            "ISO 19005-3:6.8",
                            ef_loc,
                            format!("Embedded file stream /Subtype is '{s}'; recommended 'text/xml'."),
                            format!("Subtipul MIME /Subtype este '{s}'; se recomandă 'text/xml'."),
                        );
                    }
                    Err(_) => checks.fail(
                        "PDFA3-6.8-MIME-MISSING",
                        "ISO 19005-3:6.8",
                        ef_loc,
                        "EmbeddedFile stream dictionary lacks required /Subtype MIME entry (e.g. /Subtype /text#2Fxml).",
                        "Dicționarul EmbeddedFile nu conține intrarea obligatorie /Subtype MIME (ex. /Subtype /text#2Fxml).",
                    ),
                }
            }
        }
    }
}

fn deref_key<'a>(
    doc: &'a Document,
    dict: &'a Dictionary,
    key: &[u8],
) -> Option<(Option<ObjectId>, &'a Object)> {
    dict.get(key).and_then(|o| doc.dereference(o)).ok()
}

fn obj_loc(id: Option<ObjectId>, role: &str, fallback: &str) -> String {
    id.map_or_else(
        || fallback.to_string(),
        |(n, g)| format!("Object {n} {g} R ({role})"),
    )
}

fn decode_stream_bytes(stream: &Stream) -> Result<Vec<u8>, String> {
    if !stream.dict.has(b"Filter") {
        return Ok(stream.content.clone());
    }
    stream
        .decompressed_content()
        .map_err(|e| format!("stream decompression error: {e}"))
}
