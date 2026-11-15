//! ISO 19005-3 Clauses 6.2.2 & 6.2.3: `/OutputIntents` and `GTS_PDFA1` ICC profile inspection.

use lopdf::{Dictionary, Document, Object, Stream};

use super::{PdfBudget, PdfStreamDecodeError, deref_key, fail_clause_hint, obj_loc};
use crate::stage_acc::{StageAcc, tr_fmt};

pub(super) fn check_output_intents(
    doc: &Document,
    catalog: &Dictionary,
    budget: &mut PdfBudget,
    acc: &mut StageAcc,
) {
    let Some((_, oi_obj)) = deref_key(doc, catalog, b"OutputIntents") else {
        let msg = acc
            .tr(
                "Document Catalog has no /OutputIntents array; PDF/A-3 requires a GTS_PDFA1 output intent with an embedded ICC profile.",
                "Catalogul PDF nu conține /OutputIntents; PDF/A-3 necesită un profil de culoare ICC GTS_PDFA1 încorporat.",
            )
            .to_owned();
        fail_clause_hint(
            acc,
            "PDFA3-6.2.2-OUTPUTINTENT-MISSING",
            "ISO 19005-3:6.2.2",
            "Catalog /OutputIntents",
            msg,
            "Embed an sRGB or CMYK ICC profile stream under /OutputIntents with /S /GTS_PDFA1.",
            "Încorporați un profil ICC sRGB sau CMYK în /OutputIntents cu /S /GTS_PDFA1.",
        );
        return;
    };
    let Object::Array(intents) = oi_obj else {
        acc.fail_clause(
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
        acc.pass();
        let loc = obj_loc(ref_id, "/OutputIntent", "Catalog /OutputIntents");
        match deref_key(doc, intent_dict, b"DestOutputProfile") {
            Some((icc_id, Object::Stream(icc_stream))) => {
                validate_icc_stream(
                    icc_stream,
                    &obj_loc(icc_id, "/DestOutputProfile", &loc),
                    budget,
                    acc,
                );
            }
            _ => acc.fail_clause(
                "PDFA3-6.2.3-ICC-MISSING",
                "ISO 19005-3:6.2.3",
                loc,
                "GTS_PDFA1 OutputIntent dictionary lacks an embedded /DestOutputProfile ICC stream.",
                "Dicționarul OutputIntent GTS_PDFA1 nu conține fluxul ICC /DestOutputProfile.",
            ),
        }
    }
    if !found_gts {
        acc.fail_clause(
            "PDFA3-6.2.2-GTS-PDFA1-MISSING",
            "ISO 19005-3:6.2.2",
            "Catalog /OutputIntents",
            "Catalog /OutputIntents array contains no entry with subtype /S /GTS_PDFA1.",
            "Vectorul /OutputIntents nu conține nicio intrare cu subtipul /S /GTS_PDFA1.",
        );
    }
}

fn validate_icc_stream(stream: &Stream, loc: &str, budget: &mut PdfBudget, acc: &mut StageAcc) {
    let bytes = match budget.decode_stream(stream) {
        Ok(b) => b,
        Err(PdfStreamDecodeError::LimitExceeded) => {
            acc.fail_clause(
                "PDFA3-6.1-RESOURCE-LIMIT",
                "ISO 19005-3:6.2.3",
                loc,
                "Embedded /DestOutputProfile ICC stream exceeded decompression safety budget.",
                "Fluxul ICC /DestOutputProfile a depășit limita de memorie la decomprimare.",
            );
            return;
        }
        Err(PdfStreamDecodeError::Corrupt(_)) => {
            acc.fail_clause(
                "PDFA3-6.2.3-ICC-DECODE",
                "ISO 19005-3:6.2.3",
                loc,
                "Embedded /DestOutputProfile ICC stream failed to decompress.",
                "Fluxul ICC /DestOutputProfile nu a putut fi decomprimat.",
            );
            return;
        }
    };
    if bytes.len() < 128 || &bytes[36..40] != b"acsp" {
        acc.fail_clause(
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
        let msg = tr_fmt!(
            acc,
            "ICC profile version {major}.x exceeds maximum ICC v4 allowed by ISO 19005-3.",
            "Versiunea profilului ICC {major}.x depășește versiunea maximă v4 permisă de ISO 19005-3."
        );
        acc.fatal_clause("PDFA3-6.2.3-ICC-VERSION", "ISO 19005-3:6.2.3", loc, msg);
        return;
    }
    let expected_n: i64 = match (&bytes[12..16], &bytes[16..20]) {
        (b"scnr" | b"mntr" | b"prtr" | b"spac", b"GRAY") => 1,
        (b"scnr" | b"mntr" | b"prtr" | b"spac", b"RGB ") => 3,
        (b"scnr" | b"mntr" | b"prtr" | b"spac", b"CMYK") => 4,
        _ => {
            acc.fail_clause(
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
        Ok(n) if n == expected_n => acc.pass(),
        Ok(n) => {
            let msg = tr_fmt!(
                acc,
                "/DestOutputProfile declares /N {n}, expected /N {expected_n}.",
                "/DestOutputProfile declară /N {n}, dar spațiul de culoare impune /N {expected_n}."
            );
            acc.fatal_clause("PDFA3-6.2.3-ICC-COMPONENTS", "ISO 19005-3:6.2.3", loc, msg);
        }
        Err(_) => {
            let msg = tr_fmt!(
                acc,
                "/DestOutputProfile is missing required /N {expected_n} color component count.",
                "/DestOutputProfile nu conține numărul obligatoriu de componente /N {expected_n}."
            );
            acc.fatal_clause("PDFA3-6.2.3-ICC-N-MISSING", "ISO 19005-3:6.2.3", loc, msg);
        }
    }
}
