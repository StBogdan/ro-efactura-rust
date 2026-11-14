//! ISO 19005-3 Clause 6.2.11: Font embedding and SFNT (`ttf-parser`) glyph table verification.

use std::collections::HashSet;

use lopdf::{Dictionary, Document, Object, ObjectId, Stream};

use super::{PdfBudget, PdfStreamDecodeError, deref_key, fail_clause_hint, obj_loc};
use crate::stage_acc::StageAcc;
use crate::types::Locale;

pub(super) fn check_embedded_fonts(doc: &Document, budget: &mut PdfBudget, acc: &mut StageAcc) {
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
            inspect_single_font(doc, dict, Some(obj_id), budget, acc);
        }
        if let Some((_, Object::Dictionary(res))) = deref_key(doc, dict, b"Resources")
            && let Some((_, Object::Dictionary(fonts))) = deref_key(doc, res, b"Font")
        {
            for (_, fref) in fonts {
                if let Ok((fid, Object::Dictionary(fdict))) = doc.dereference(fref)
                    && fid.is_none_or(|id| visited.insert(id))
                {
                    inspect_single_font(doc, fdict, fid, budget, acc);
                }
            }
        }
    }
    if visited.is_empty() {
        acc.pass();
    }
}

fn inspect_single_font(
    doc: &Document,
    font_dict: &Dictionary,
    obj_id: Option<ObjectId>,
    budget: &mut PdfBudget,
    acc: &mut StageAcc,
) {
    let subtype = font_dict
        .get(b"Subtype")
        .and_then(Object::as_name)
        .unwrap_or(b"");
    let base_font = font_dict
        .get(b"BaseFont")
        .and_then(Object::as_name)
        .map_or_else(
            |_| "<unnamed>".to_owned(),
            |b| String::from_utf8_lossy(b).into_owned(),
        );
    let fallback = format!("Font /{base_font}");
    let loc = obj_loc(obj_id, &fallback, &fallback);

    if subtype == b"Type3" {
        acc.pass();
    } else if subtype == b"Type0" {
        if let Some((_, Object::Array(descendants))) = deref_key(doc, font_dict, b"DescendantFonts")
        {
            for desc in descendants {
                if let Ok((did, Object::Dictionary(cid))) = doc.dereference(desc) {
                    verify_font_descriptor(doc, cid, &base_font, did.or(obj_id), budget, acc);
                }
            }
        } else {
            let msg = match acc.locale() {
                Locale::En => {
                    format!("Type0 composite font /{base_font} has no /DescendantFonts array.")
                }
                Locale::Ro => format!(
                    "Fontul compozit Type0 /{base_font} nu conține vectorul /DescendantFonts."
                ),
            };
            acc.fatal_clause(
                "PDFA3-6.2.11-TYPE0-DESCENDANT",
                "ISO 19005-3:6.2.11.4",
                loc,
                msg,
            );
        }
    } else {
        verify_font_descriptor(doc, font_dict, &base_font, obj_id, budget, acc);
    }
}

fn verify_font_descriptor(
    doc: &Document,
    font_dict: &Dictionary,
    base_font: &str,
    obj_id: Option<ObjectId>,
    budget: &mut PdfBudget,
    acc: &mut StageAcc,
) {
    let fallback = format!("Font /{base_font}");
    let loc = obj_loc(obj_id, &fallback, &fallback);
    let Some((_, Object::Dictionary(fd))) = deref_key(doc, font_dict, b"FontDescriptor") else {
        let msg = match acc.locale() {
            Locale::En => format!(
                "Font /{base_font} has no /FontDescriptor and its glyph program is not embedded; ISO 19005-3 requires embedding every used font."
            ),
            Locale::Ro => format!(
                "Fontul /{base_font} nu are /FontDescriptor și nu este încorporat; ISO 19005-3 impune încorporarea tuturor fonturilor utilizate."
            ),
        };
        fail_clause_hint(
            acc,
            "PDFA3-6.2.11-FONT-NOT-EMBEDDED",
            "ISO 19005-3:6.2.11.4",
            loc,
            msg,
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
        let msg = match acc.locale() {
            Locale::En => format!(
                "FontDescriptor for /{base_font} contains none of /FontFile, /FontFile2, or /FontFile3."
            ),
            Locale::Ro => format!(
                "FontDescriptor pentru /{base_font} nu conține /FontFile, /FontFile2 sau /FontFile3."
            ),
        };
        acc.fatal_clause(
            "PDFA3-6.2.11-FONTFILE-MISSING",
            "ISO 19005-3:6.2.11.4",
            loc,
            msg,
        );
        return;
    };

    let sloc = obj_loc(sid, &fallback, &loc);
    verify_embedded_font_stream(key, stream, base_font, sloc, budget, acc);
}

fn verify_embedded_font_stream(
    key: &[u8],
    stream: &Stream,
    base_font: &str,
    sloc: String,
    budget: &mut PdfBudget,
    acc: &mut StageAcc,
) {
    let font_bytes = match budget.decode_stream(stream) {
        Ok(b) => b,
        Err(PdfStreamDecodeError::LimitExceeded) => {
            let msg = match acc.locale() {
                Locale::En => format!(
                    "Embedded font stream for /{base_font} exceeded decompression safety budget."
                ),
                Locale::Ro => format!(
                    "Fluxul fontului încorporat /{base_font} a depășit limita de memorie la decomprimare."
                ),
            };
            acc.fatal_clause(
                "PDFA3-6.1-RESOURCE-LIMIT",
                "ISO 19005-3:6.2.11.4",
                sloc,
                msg,
            );
            return;
        }
        Err(PdfStreamDecodeError::Corrupt(_)) => {
            let msg = match acc.locale() {
                Locale::En => {
                    format!("Embedded font stream for /{base_font} could not be decompressed.")
                }
                Locale::Ro => {
                    format!("Fluxul fontului încorporat /{base_font} nu a putut fi decomprimat.")
                }
            };
            acc.fatal_clause(
                "PDFA3-6.2.11-FONTFILE-DECODE",
                "ISO 19005-3:6.2.11.4",
                sloc,
                msg,
            );
            return;
        }
    };
    if font_bytes.is_empty() {
        let msg = match acc.locale() {
            Locale::En => format!("Embedded font stream for /{base_font} is empty (0 bytes)."),
            Locale::Ro => format!("Fluxul fontului încorporat /{base_font} este gol (0 octeți)."),
        };
        acc.fatal_clause(
            "PDFA3-6.2.11-FONTFILE-EMPTY",
            "ISO 19005-3:6.2.11.4",
            sloc,
            msg,
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
            Ok(f) if f.number_of_glyphs() > 0 => acc.pass(),
            Ok(_) => {
                let msg = match acc.locale() {
                    Locale::En => {
                        format!("Embedded TrueType/OpenType font /{base_font} contains 0 glyphs.")
                    }
                    Locale::Ro => {
                        format!("Fontul TrueType/OpenType /{base_font} conține 0 glife.")
                    }
                };
                acc.fatal_clause(
                    "PDFA3-6.2.11-FONT-ZERO-GLYPHS",
                    "ISO 19005-3:6.2.11.4",
                    sloc,
                    msg,
                );
            }
            Err(err) => {
                let msg = match acc.locale() {
                    Locale::En => format!(
                        "Embedded TrueType/OpenType font /{base_font} failed SFNT parsing: {err}"
                    ),
                    Locale::Ro => {
                        format!("Fontul TrueType/OpenType /{base_font} este corupt SFNT: {err}")
                    }
                };
                acc.fatal_clause(
                    "PDFA3-6.2.11-FONT-CORRUPT",
                    "ISO 19005-3:6.2.11.4",
                    sloc,
                    msg,
                );
            }
        }
    } else {
        acc.pass();
    }
}
