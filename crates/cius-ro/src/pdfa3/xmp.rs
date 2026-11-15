//! ISO 19005-3 Clauses 6.6.2 & 6.6.4: XMP `/Metadata` stream, `pdfaid` identification, and
//! Factur-X / `ZUGFeRD` extension schema declarations.

use lopdf::{Dictionary, Document, Object};

use super::{PdfBudget, PdfStreamDecodeError, deref_key, obj_loc};
use crate::stage_acc::{StageAcc, tr_fmt};

const PDFAID_NS: &str = "http://www.aiim.org/pdfa/ns/id/";
const PDFA_EXT_NS: &str = "http://www.aiim.org/pdfa/ns/extension/";

struct ParsedXmpProps {
    part: Option<String>,
    conf: Option<String>,
    doc_fname: Option<String>,
    has_fx: bool,
    has_ext: bool,
}

pub(super) fn check_xmp_metadata(
    doc: &Document,
    catalog: &Dictionary,
    budget: &mut PdfBudget,
    acc: &mut StageAcc,
) -> Option<String> {
    let (loc, raw_xmp) = decode_xmp_stream(doc, catalog, budget, acc)?;
    let Ok(xmp_str) = core::str::from_utf8(&raw_xmp) else {
        acc.fail_clause(
            "PDFA3-6.6.2-METADATA-UTF8",
            "ISO 19005-3:6.6.2.1",
            loc,
            "Catalog /Metadata XMP stream is not valid UTF-8.",
            "Fluxul XMP /Metadata nu este codificat valid UTF-8.",
        );
        return None;
    };
    let Ok(xml) = roxmltree::Document::parse(xmp_str.trim_matches('\0').trim()) else {
        acc.fail_clause(
            "PDFA3-6.6.2-METADATA-XML",
            "ISO 19005-3:6.6.2.1",
            loc,
            "Catalog /Metadata XMP packet is not well-formed XML.",
            "Pachetul XMP /Metadata nu este XML bine format.",
        );
        return None;
    };

    let props = scan_xmp_properties(&xml);

    match props.part.as_deref() {
        Some("3") => acc.pass(),
        Some(other) => {
            let msg = tr_fmt!(
                acc,
                "XMP pdfaid:part is '{other}', expected '3' for PDF/A-3 hybrid e-invoices.",
                "Proprietatea XMP pdfaid:part este '{other}', dar pentru PDF/A-3 trebuie să fie '3'."
            );
            acc.fatal_clause("PDFA3-6.6.4-PDFAID-PART", "ISO 19005-3:6.6.4", &loc, msg);
        }
        None => acc.fail_clause(
            "PDFA3-6.6.4-PDFAID-MISSING",
            "ISO 19005-3:6.6.4",
            &loc,
            "XMP metadata lacks pdfaid:part='3' identification property.",
            "Metadatele XMP nu conțin proprietatea pdfaid:part='3'.",
        ),
    }

    match props.conf.as_deref() {
        Some("A" | "B" | "U") => acc.pass(),
        Some(other) => {
            let msg = tr_fmt!(
                acc,
                "XMP pdfaid:conformance '{other}' is invalid; must be 'A', 'B', or 'U'.",
                "Valoarea XMP pdfaid:conformance '{other}' este invalidă; permis: 'A', 'B' sau 'U'."
            );
            acc.fatal_clause(
                "PDFA3-6.6.4-PDFAID-CONFORMANCE",
                "ISO 19005-3:6.6.4",
                &loc,
                msg,
            );
        }
        None => acc.fail_clause(
            "PDFA3-6.6.4-PDFAID-CONFORMANCE-MISSING",
            "ISO 19005-3:6.6.4",
            &loc,
            "XMP metadata lacks pdfaid:conformance ('A', 'B', or 'U').",
            "Metadatele XMP nu conțin pdfaid:conformance ('A', 'B' sau 'U').",
        ),
    }

    if props.has_fx && !props.has_ext {
        acc.fail_clause(
            "PDFA3-6.6.2.3-EXTENSION-SCHEMA",
            "ISO 19005-3:6.6.2.3",
            loc,
            "XMP contains custom Factur-X/ZUGFeRD properties without pdfaExtension:schemas.",
            "Metadatele XMP conțin proprietăți Factur-X/ZUGFeRD fără declarația pdfaExtension:schemas.",
        );
    } else {
        acc.pass();
    }
    props.doc_fname
}

fn decode_xmp_stream(
    doc: &Document,
    catalog: &Dictionary,
    budget: &mut PdfBudget,
    acc: &mut StageAcc,
) -> Option<(String, Vec<u8>)> {
    let Some((meta_id, Object::Stream(meta_stream))) = deref_key(doc, catalog, b"Metadata") else {
        acc.fail_clause(
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
        acc.fail_clause(
            "PDFA3-6.6.2-METADATA-FILTERED",
            "ISO 19005-3:6.6.2.1",
            &loc,
            "Catalog /Metadata XMP stream is compressed (/Filter present); ISO 19005-3 requires unfiltered XMP.",
            "Fluxul XMP /Metadata este comprimat (/Filter prezent); ISO 19005-3 cere XMP necomprimat.",
        );
    } else {
        acc.pass();
    }

    match budget.decode_stream(meta_stream) {
        Ok(b) => Some((loc, b)),
        Err(PdfStreamDecodeError::LimitExceeded) => {
            acc.fail_clause(
                "PDFA3-6.1-RESOURCE-LIMIT",
                "ISO 19005-3:6.6.2.1",
                loc,
                "Catalog /Metadata XMP stream exceeded decompression safety budget.",
                "Fluxul XMP /Metadata a depășit limita de memorie la decomprimare.",
            );
            None
        }
        Err(PdfStreamDecodeError::Corrupt(_)) => {
            acc.fail_clause(
                "PDFA3-6.6.2-METADATA-UNREADABLE",
                "ISO 19005-3:6.6.2.1",
                loc,
                "Catalog /Metadata XMP stream could not be decoded.",
                "Fluxul XMP /Metadata nu a putut fi decodat.",
            );
            None
        }
    }
}

fn scan_xmp_properties(xml: &roxmltree::Document<'_>) -> ParsedXmpProps {
    let (mut part, mut conf, mut doc_fname) = (None, None, None);
    let (mut has_fx, mut has_ext) = (false, false);

    for node in xml.descendants().filter(roxmltree::Node::is_element) {
        let ns = node.tag_name().namespace().unwrap_or("");
        let name = node.tag_name().name();
        let text = || {
            node.text()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
        };

        if ns == PDFAID_NS && name == "part" {
            part = node.text().map(|s| s.trim().to_owned());
        } else if ns == PDFAID_NS && name == "conformance" {
            conf = node.text().map(|s| s.trim().to_owned());
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
                part = Some(val.to_owned());
            } else if ans == PDFAID_NS && attr.name() == "conformance" {
                conf = Some(val.to_owned());
            } else if ans.contains("factur-x") || ans.contains("zugferd") {
                has_fx = true;
                if attr.name() == "DocumentFileName" && !val.is_empty() && doc_fname.is_none() {
                    doc_fname = Some(val.to_owned());
                }
            }
        }
    }

    ParsedXmpProps {
        part,
        conf,
        doc_fname,
        has_fx,
        has_ext,
    }
}
