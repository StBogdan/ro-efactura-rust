//! ISO 19005-3 Clause 6.8: Associated Files (`/AF`), `/Names/EmbeddedFiles` traversal, and
//! single-pass hybrid XML invoice (`efactura.xml` / `factur-x.xml`) extraction.

use std::collections::HashSet;

use en16931_formats::zugferd::{self, Divergence};
use lopdf::{Dictionary, Document, Object, ObjectId};

use super::loader::{PdfLoadError, load_bounded_document};
use super::{
    ExtractedPdfXml, MAX_TREE_DEPTH, MAX_TREE_OBJECTS, PdfBudget, PdfExtractError,
    PdfStreamDecodeError, deref_key, fail_clause_hint, obj_loc,
};
use crate::stage_acc::StageAcc;
use crate::types::Locale;

/// Priority order of standard hybrid e-invoice attachment filenames.
const PRIMARY_INVOICE_FILENAMES: &[&str] = &[
    "efactura.xml",
    "factur-x.xml",
    "zugferd-invoice.xml",
    "xrechnung.xml",
    "order-x.xml",
];

/// Extracts the embedded XML e-invoice (`efactura.xml`, `factur-x.xml`, `zugferd-invoice.xml`,
/// `xrechnung.xml`, or fallback `.xml` attachment) from raw PDF bytes in memory.
///
/// Parses the PDF container once under bounded decompression limits and inflates **only** the
/// single target XML invoice stream, skipping unrelated embedded attachments.
///
/// # Errors
///
/// Returns [`PdfExtractError`] if the PDF cannot be parsed, exceeds stream/object safety limits,
/// or carries no embedded XML e-invoice.
pub fn extract_xml_from_pdf(pdf_bytes: &[u8]) -> Result<ExtractedPdfXml, PdfExtractError> {
    let mut budget = PdfBudget::new();
    let doc = load_bounded_document(pdf_bytes, &mut budget).map_err(|err| match err {
        PdfLoadError::LimitExceeded(detail) => PdfExtractError::ResourceLimitExceeded(detail),
        PdfLoadError::Parse(detail) => PdfExtractError::InvalidPdfStructure(detail),
    })?;
    let catalog = doc
        .catalog()
        .map_err(|err| PdfExtractError::InvalidPdfStructure(err.to_string()))?;

    match extract_invoice_from_doc(&doc, catalog, &mut budget) {
        Ok(Some(found)) => Ok(found.payload),
        Ok(None) => Err(PdfExtractError::NoEmbeddedXmlInvoice),
        Err(PdfStreamDecodeError::LimitExceeded) => Err(PdfExtractError::ResourceLimitExceeded(
            "Embedded XML stream or /EmbeddedFiles name tree exceeded safety bounds".to_owned(),
        )),
        Err(PdfStreamDecodeError::Corrupt(reason)) => {
            Err(PdfExtractError::InvalidPdfStructure(reason))
        }
    }
}

fn has_xml_ext(name: &str) -> bool {
    name.get(name.len().wrapping_sub(4)..)
        .is_some_and(|ext| ext.eq_ignore_ascii_case(".xml"))
}

struct AfAssociationCtx<'a> {
    has_af: bool,
    af_ids: HashSet<ObjectId>,
    has_inline_af: bool,
    xmp_doc_filename: Option<&'a str>,
}

struct ExtractedDocInvoice {
    payload: ExtractedPdfXml,
    filespec_id: Option<ObjectId>,
    in_embedded_files: bool,
    has_relationship: bool,
}

pub(super) fn check_associated_files_and_extract(
    pdf_bytes: &[u8],
    doc: &Document,
    catalog: &Dictionary,
    xmp_doc_filename: Option<&str>,
    budget: &mut PdfBudget,
    acc: &mut StageAcc,
) -> Option<ExtractedPdfXml> {
    let mut af_ids: HashSet<ObjectId> = HashSet::new();
    let mut has_inline_af = false;
    let has_af = match deref_key(doc, catalog, b"AF") {
        Some((_, Object::Array(af_arr))) if !af_arr.is_empty() => {
            acc.pass();
            for item in af_arr {
                match item {
                    Object::Reference(id) => {
                        af_ids.insert(*id);
                    }
                    Object::Dictionary(_) => {
                        has_inline_af = true;
                    }
                    _ => {}
                }
            }
            check_filespec_mime_and_relationship(doc, af_arr, acc);
            true
        }
        _ => {
            let msg = acc
                .tr(
                    "Document Catalog is missing a non-empty /AF (Associated Files) array required by ISO 19005-3 Clause 6.8.",
                    "Catalogul PDF nu conține vectorul /AF (Associated Files) cerut de ISO 19005-3 Clauza 6.8.",
                )
                .to_owned();
            fail_clause_hint(
                acc,
                "PDFA3-6.8-AF-MISSING",
                "ISO 19005-3:6.8",
                "Catalog /AF",
                msg,
                "Reference the efactura.xml Filespec dictionary from both Catalog /AF and /Names/EmbeddedFiles.",
                "Referențiați dicționarul Filespec pentru efactura.xml atât în Catalog /AF, cât și în /Names/EmbeddedFiles.",
            );
            false
        }
    };

    let ctx = AfAssociationCtx {
        has_af,
        af_ids,
        has_inline_af,
        xmp_doc_filename,
    };
    inspect_extracted_invoice(pdf_bytes, doc, catalog, &ctx, budget, acc)
}

fn inspect_extracted_invoice(
    pdf_bytes: &[u8],
    doc: &Document,
    catalog: &Dictionary,
    ctx: &AfAssociationCtx<'_>,
    budget: &mut PdfBudget,
    acc: &mut StageAcc,
) -> Option<ExtractedPdfXml> {
    let found = match extract_invoice_from_doc(doc, catalog, budget) {
        Ok(Some(f)) => f,
        Ok(None) => {
            emit_missing_invoice_payload(None, None, acc);
            return None;
        }
        Err(PdfStreamDecodeError::LimitExceeded) => {
            acc.fail_clause(
                "PDFA3-6.1-RESOURCE-LIMIT",
                "ISO 19005-3:6.8",
                "Catalog /Names/EmbeddedFiles",
                "Embedded XML stream or name tree exceeded decompression/traversal safety bounds.",
                "Fluxul XML încorporat sau arborele /EmbeddedFiles a depășit limitele de securitate.",
            );
            return None;
        }
        Err(PdfStreamDecodeError::Corrupt(reason)) => {
            let msg = match acc.locale() {
                Locale::En => format!("Embedded XML stream failed to decode: {reason}"),
                Locale::Ro => format!("Fluxul XML încorporat nu a putut fi decodat: {reason}"),
            };
            acc.fatal_clause(
                "PDFA3-6.8-NO-INVOICE-PAYLOAD",
                "ISO 19005-3:6.8",
                "Catalog /Names/EmbeddedFiles",
                msg,
            );
            return None;
        }
    };

    let fname = &found.payload.filename;
    if fname.eq_ignore_ascii_case("efactura.xml") {
        let is_associated = !ctx.has_af
            || found
                .filespec_id
                .map_or(ctx.has_inline_af, |id| ctx.af_ids.contains(&id));
        verify_efactura_association(
            fname,
            is_associated,
            found.in_embedded_files,
            found.has_relationship,
            ctx.xmp_doc_filename,
            acc,
        );
        return Some(found.payload);
    }

    if zugferd::FILENAMES
        .iter()
        .any(|f| fname.eq_ignore_ascii_case(f))
    {
        match zugferd::extract(pdf_bytes, &zugferd::Limits::default()) {
            Ok(extracted) => record_zugferd_divergences(&extracted, acc),
            Err(err) => emit_missing_invoice_payload(Some(fname), Some(&err), acc),
        }
        return Some(found.payload);
    }

    emit_missing_invoice_payload(Some(fname), None, acc);
    Some(found.payload)
}

fn record_zugferd_divergences(extracted: &zugferd::Extracted, acc: &mut StageAcc) {
    if extracted.divergence.is_empty() {
        acc.pass();
        return;
    }
    let loc = format!("EmbeddedFile ({})", extracted.filename);
    for div in &extracted.divergence {
        let msg = format_divergence(div, acc.locale());
        match div {
            Divergence::Profile { .. }
            | Divergence::Filename { .. }
            | Divergence::Relationship { .. }
            | Divergence::NoXmp
            | Divergence::NotAssociated
            | Divergence::NotInEmbeddedFiles
            | Divergence::NoRelationship
            | Divergence::NotPdfA3 { .. } => {
                acc.fatal_clause("PDFA3-6.8-HYBRID-DIVERGENCE", "ISO 19005-3:6.8", &loc, msg);
            }
            _ => {
                acc.warn_clause("PDFA3-6.8-HYBRID-NOTICE", "ISO 19005-3:6.8", &loc, msg);
            }
        }
    }
}

fn verify_efactura_association(
    filename: &str,
    is_associated: bool,
    in_embedded_files: bool,
    has_relationship: bool,
    xmp_doc_filename: Option<&str>,
    acc: &mut StageAcc,
) {
    let loc = format!("EmbeddedFile ({filename})");
    let mut clean = true;

    if !is_associated {
        clean = false;
        let msg = match acc.locale() {
            Locale::En => format!(
                "Embedded invoice '{filename}' is not referenced in the Document Catalog /AF array."
            ),
            Locale::Ro => format!(
                "Factura încorporată '{filename}' nu este referențiată în vectorul /AF din Catalogul PDF."
            ),
        };
        acc.fatal_clause("PDFA3-6.8-HYBRID-DIVERGENCE", "ISO 19005-3:6.8", &loc, msg);
    }

    if !in_embedded_files {
        clean = false;
        let msg = match acc.locale() {
            Locale::En => format!(
                "Embedded invoice '{filename}' is not reachable via Catalog /Names/EmbeddedFiles."
            ),
            Locale::Ro => format!(
                "Factura încorporată '{filename}' nu este accesibilă prin arborele /Names/EmbeddedFiles din Catalogul PDF."
            ),
        };
        acc.fatal_clause("PDFA3-6.8-HYBRID-DIVERGENCE", "ISO 19005-3:6.8", &loc, msg);
    }

    if !has_relationship {
        clean = false;
        let msg = match acc.locale() {
            Locale::En => {
                format!("Filespec dictionary for '{filename}' carries no /AFRelationship entry.")
            }
            Locale::Ro => format!(
                "Dicționarul Filespec pentru '{filename}' nu conține intrarea /AFRelationship."
            ),
        };
        acc.fatal_clause("PDFA3-6.8-HYBRID-DIVERGENCE", "ISO 19005-3:6.8", &loc, msg);
    }

    if let Some(xmp_f) = xmp_doc_filename.filter(|x| !x.eq_ignore_ascii_case(filename)) {
        clean = false;
        let msg = match acc.locale() {
            Locale::En => format!(
                "XMP fx:DocumentFileName declares '{xmp_f}', but associated invoice attachment is '{filename}'."
            ),
            Locale::Ro => format!(
                "Metadatele XMP fx:DocumentFileName declară '{xmp_f}', dar fișierul atașat este '{filename}'."
            ),
        };
        acc.fatal_clause("PDFA3-6.8-HYBRID-DIVERGENCE", "ISO 19005-3:6.8", &loc, msg);
    }

    if clean {
        acc.pass();
    }
}

fn emit_missing_invoice_payload(
    found_name: Option<&str>,
    zugferd_err: Option<&zugferd::Error>,
    acc: &mut StageAcc,
) {
    let msg = match (found_name, zugferd_err, acc.locale()) {
        (_, Some(err), Locale::En) => format!("Hybrid PDF invoice extraction failed: {err}"),
        (_, Some(err), Locale::Ro) => {
            format!("Extragerea facturii XML din PDF-ul hibrid a eșuat: {err}")
        }
        (Some(name), None, Locale::En) => format!(
            "Embedded XML file '{name}' does not use a recognized hybrid invoice filename (efactura.xml, factur-x.xml, zugferd-invoice.xml, or xrechnung.xml)."
        ),
        (Some(name), None, Locale::Ro) => format!(
            "Fișierul XML atașat '{name}' nu are un nume standard de factură hibridă (efactura.xml, factur-x.xml, zugferd-invoice.xml sau xrechnung.xml)."
        ),
        (None, None, Locale::En) => {
            "PDF container carries no embedded XML invoice attachment.".to_owned()
        }
        (None, None, Locale::Ro) => {
            "Containerul PDF nu conține nicio factură electronică XML încorporată.".to_owned()
        }
    };
    fail_clause_hint(
        acc,
        "PDFA3-6.8-NO-INVOICE-PAYLOAD",
        "ISO 19005-3:6.8",
        "Catalog /Names/EmbeddedFiles",
        msg,
        "Attach the XML invoice as efactura.xml, factur-x.xml, zugferd-invoice.xml, or xrechnung.xml.",
        "Atașați factura XML cu numele efactura.xml, factur-x.xml, zugferd-invoice.xml sau xrechnung.xml.",
    );
}

fn format_divergence(div: &Divergence, locale: Locale) -> String {
    if locale == Locale::En {
        return div.to_string();
    }
    match div {
        Divergence::Profile { xmp, payload } => format!(
            "Metadatele XMP ale PDF-ului declară profilul '{xmp}', dar identificatorul BT-24 din XML este '{payload}'."
        ),
        Divergence::Filename { xmp, actual } => format!(
            "Metadatele XMP declară numele fișierului atașat '{xmp}', dar atașamentul găsit este '{actual}'."
        ),
        Divergence::Relationship { found, profile } => format!(
            "Factura este atașată cu /AFRelationship /{found}, incompatibil cu profilul {profile:?} (utilizați /Alternative sau /Source)."
        ),
        Divergence::NoXmp => {
            "Documentul PDF nu conține metadate XMP pentru factura electronică încorporată."
                .to_owned()
        }
        Divergence::NotAssociated => {
            "Factura XML nu este referențiată în vectorul /AF din Catalogul PDF (nu este declarată ca fișier asociat PDF/A-3)."
                .to_owned()
        }
        Divergence::NotInEmbeddedFiles => {
            "Factura XML nu este accesibilă prin arborele /Names/EmbeddedFiles din Catalogul PDF."
                .to_owned()
        }
        Divergence::NoRelationship => {
            "Dicționarul Filespec al facturii XML nu conține intrarea obligatorie /AFRelationship."
                .to_owned()
        }
        Divergence::NotPdfA3 { part: Some(p) } => format!(
            "Documentul PDF declară conformitate PDF/A-{p}; facturile hibride necesită PDF/A-3 (pdfaid:part=3)."
        ),
        Divergence::NotPdfA3 { part: None } => {
            "Documentul PDF nu declară conformitatea PDF/A-3 în metadatele XMP.".to_owned()
        }
        _ => div.to_string(),
    }
}

struct EmbeddedCandidate<'a> {
    filename: String,
    filespec_id: Option<ObjectId>,
    filespec: &'a Dictionary,
    in_embedded_files: bool,
}

fn extract_invoice_from_doc(
    doc: &Document,
    catalog: &Dictionary,
    budget: &mut PdfBudget,
) -> Result<Option<ExtractedDocInvoice>, PdfStreamDecodeError> {
    let mut candidates: Vec<EmbeddedCandidate<'_>> = Vec::new();
    let mut seen_ids: HashSet<ObjectId> = HashSet::new();

    if let Some((_, Object::Dictionary(names_dict))) = deref_key(doc, catalog, b"Names")
        && let Some((root_id, Object::Dictionary(ef_tree))) =
            deref_key(doc, names_dict, b"EmbeddedFiles")
    {
        let mut path_ids: HashSet<ObjectId> = HashSet::new();
        if let Some(id) = root_id {
            path_ids.insert(id);
        }
        let mut visited_count = 1usize;
        collect_name_tree_filespecs(
            doc,
            ef_tree,
            0,
            &mut path_ids,
            &mut visited_count,
            &mut seen_ids,
            &mut candidates,
        )?;
    }

    if let Some((_, Object::Array(af_arr))) = deref_key(doc, catalog, b"AF") {
        for item in af_arr {
            let Ok((fs_id, Object::Dictionary(fs_dict))) = doc.dereference(item) else {
                continue;
            };
            if fs_id.is_some_and(|id| !seen_ids.insert(id)) {
                continue;
            }
            if let Some(filename) = filespec_filename(doc, fs_dict) {
                candidates.push(EmbeddedCandidate {
                    filename,
                    filespec_id: fs_id,
                    filespec: fs_dict,
                    in_embedded_files: false,
                });
            }
        }
    }

    let chosen = PRIMARY_INVOICE_FILENAMES
        .iter()
        .find_map(|target| {
            candidates
                .iter()
                .find(|c| c.filename.eq_ignore_ascii_case(target))
        })
        .or_else(|| candidates.iter().find(|c| has_xml_ext(&c.filename)));

    let Some(cand) = chosen else {
        return Ok(None);
    };

    let Some((_, Object::Dictionary(ef_dict))) = deref_key(doc, cand.filespec, b"EF") else {
        return Ok(None);
    };
    let ef_stream_obj = deref_key(doc, ef_dict, b"UF").or_else(|| deref_key(doc, ef_dict, b"F"));
    let Some((_, Object::Stream(stream))) = ef_stream_obj else {
        return Ok(None);
    };

    let xml_bytes = budget.decode_stream(stream)?;
    let has_relationship = cand
        .filespec
        .get(b"AFRelationship")
        .and_then(Object::as_name)
        .is_ok();

    Ok(Some(ExtractedDocInvoice {
        payload: ExtractedPdfXml {
            filename: cand.filename.clone(),
            xml_bytes,
        },
        filespec_id: cand.filespec_id,
        in_embedded_files: cand.in_embedded_files,
        has_relationship,
    }))
}

fn collect_name_tree_filespecs<'a>(
    doc: &'a Document,
    node: &'a Dictionary,
    depth: usize,
    path_ids: &mut HashSet<ObjectId>,
    visited_count: &mut usize,
    seen_ids: &mut HashSet<ObjectId>,
    out: &mut Vec<EmbeddedCandidate<'a>>,
) -> Result<(), PdfStreamDecodeError> {
    if depth > MAX_TREE_DEPTH || *visited_count > MAX_TREE_OBJECTS {
        return Err(PdfStreamDecodeError::LimitExceeded);
    }

    if let Some((_, Object::Array(names_arr))) = deref_key(doc, node, b"Names") {
        for pair in names_arr.chunks(2) {
            let Some(val_obj) = pair.get(1) else {
                continue;
            };
            *visited_count += 1;
            if *visited_count > MAX_TREE_OBJECTS {
                return Err(PdfStreamDecodeError::LimitExceeded);
            }
            let Ok((fs_id, Object::Dictionary(fs_dict))) = doc.dereference(val_obj) else {
                continue;
            };
            if let Some(id) = fs_id {
                seen_ids.insert(id);
            }
            let filename = filespec_filename(doc, fs_dict)
                .or_else(|| match pair.first() {
                    Some(Object::String(bytes, _)) => Some(decode_pdf_string(bytes)),
                    _ => None,
                })
                .unwrap_or_default();
            if !filename.is_empty() {
                out.push(EmbeddedCandidate {
                    filename,
                    filespec_id: fs_id,
                    filespec: fs_dict,
                    in_embedded_files: true,
                });
            }
        }
    }

    if let Some((_, Object::Array(kids_arr))) = deref_key(doc, node, b"Kids") {
        for kid_ref in kids_arr {
            *visited_count += 1;
            if *visited_count > MAX_TREE_OBJECTS {
                return Err(PdfStreamDecodeError::LimitExceeded);
            }
            let Ok((kid_id, Object::Dictionary(kid_dict))) = doc.dereference(kid_ref) else {
                continue;
            };
            if let Some(id) = kid_id
                && !path_ids.insert(id)
            {
                return Err(PdfStreamDecodeError::LimitExceeded);
            }
            collect_name_tree_filespecs(
                doc,
                kid_dict,
                depth + 1,
                path_ids,
                visited_count,
                seen_ids,
                out,
            )?;
            if let Some(id) = kid_id {
                path_ids.remove(&id);
            }
        }
    }

    Ok(())
}

fn filespec_filename(doc: &Document, fs_dict: &Dictionary) -> Option<String> {
    for key in [b"UF".as_slice(), b"F"] {
        if let Some((_, Object::String(bytes, _))) = deref_key(doc, fs_dict, key) {
            let s = decode_pdf_string(bytes);
            if !s.is_empty() {
                return Some(s);
            }
        }
    }
    None
}

fn decode_pdf_string(bytes: &[u8]) -> String {
    if let Some(rest) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        let units: Vec<u16> = rest
            .chunks_exact(2)
            .map(|c| u16::from_be_bytes([c[0], c[1]]))
            .collect();
        return String::from_utf16_lossy(&units);
    }
    if let Some(rest) = bytes.strip_prefix(&[0xFF, 0xFE]) {
        let units: Vec<u16> = rest
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        return String::from_utf16_lossy(&units);
    }
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_owned(),
        Err(_) => bytes.iter().map(|&b| char::from(b)).collect(),
    }
}

fn check_filespec_mime_and_relationship(doc: &Document, af_arr: &[Object], acc: &mut StageAcc) {
    for item in af_arr {
        let Ok((fs_id, Object::Dictionary(fs_dict))) = doc.dereference(item) else {
            continue;
        };
        let loc = obj_loc(fs_id, "/Filespec", "Catalog /AF Filespec");
        let rel_name = fs_dict.get(b"AFRelationship").and_then(Object::as_name);
        match rel_name {
            Ok(b"Alternative" | b"Data" | b"Source" | b"Supplement" | b"Unspecified") => {
                acc.pass();
            }
            Ok(other) => {
                let s = String::from_utf8_lossy(other).into_owned();
                let msg = match acc.locale() {
                    Locale::En => {
                        format!("Filespec /AFRelationship '/{s}' is not valid in ISO 19005-3.")
                    }
                    Locale::Ro => format!(
                        "Valoarea /AFRelationship '/{s}' din Filespec nu este permisă de ISO 19005-3."
                    ),
                };
                acc.fatal_clause(
                    "PDFA3-6.8-AFRELATIONSHIP-INVALID",
                    "ISO 19005-3:6.8",
                    &loc,
                    msg,
                );
            }
            Err(_) => acc.fail_clause(
                "PDFA3-6.8-AFRELATIONSHIP-MISSING",
                "ISO 19005-3:6.8",
                &loc,
                "Associated file specification dictionary lacks a required /AFRelationship entry.",
                "Dicționarul Filespec nu conține intrarea obligatorie /AFRelationship.",
            ),
        }

        let is_non_xml_supplement = matches!(rel_name, Ok(b"Supplement" | b"Unspecified"))
            && filespec_filename(doc, fs_dict).is_some_and(|fname| !has_xml_ext(&fname));

        if let Some((_, Object::Dictionary(ef_dict))) = deref_key(doc, fs_dict, b"EF") {
            let ef = deref_key(doc, ef_dict, b"UF").or_else(|| deref_key(doc, ef_dict, b"F"));
            if let Some((ef_id, Object::Stream(ef_stream))) = ef {
                let ef_loc = obj_loc(ef_id, "/EmbeddedFile", &loc);
                match ef_stream.dict.get(b"Subtype").and_then(Object::as_name) {
                    Ok(b"text/xml" | b"application/xml") => acc.pass(),
                    Ok(other) if !other.is_empty() && is_non_xml_supplement => acc.pass(),
                    Ok(other) => {
                        let s = String::from_utf8_lossy(other).into_owned();
                        let msg = match acc.locale() {
                            Locale::En => {
                                format!("Embedded file stream /Subtype is '{s}'; recommended 'text/xml'.")
                            }
                            Locale::Ro => {
                                format!("Subtipul MIME /Subtype este '{s}'; se recomandă 'text/xml'.")
                            }
                        };
                        acc.warn_clause("PDFA3-6.8-MIME-SUBTYPE", "ISO 19005-3:6.8", ef_loc, msg);
                    }
                    Err(_) => acc.fail_clause(
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
