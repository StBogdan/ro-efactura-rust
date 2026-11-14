//! Bounded in-memory PDF container loader (`XRef` stream header guards and `/ObjStm` inflation
//! accounting) shared by [`super::inspect_pdfa3`] and [`super::extract_xml_from_pdf`].

use lopdf::{Document, LoadOptions, Object, ObjectId};

use super::{MAX_STREAM_BYTES, PdfBudget, PdfStreamDecodeError};

/// Maximum indirect object count permitted by ISO 32000-1 Annex C (`8,388,607`).
const MAX_PDF_OBJECTS: i64 = 8_388_607;

pub(super) enum PdfLoadError {
    Parse(String),
    LimitExceeded(String),
}

/// Parses `pdf_bytes` into a [`lopdf::Document`] while enforcing:
/// 1. `XRef` `/W` field-width (`0..=8`) and `/Size` / `/Index` (`0..=8_388_607`) pre-load bounds.
/// 2. Cumulative `/ObjStm` stream decompression accounting against `budget`.
/// 3. Per-stream `max_decompressed_size` (`64 MiB`) during object-stream expansion.
pub(super) fn load_bounded_document(
    pdf_bytes: &[u8],
    budget: &mut PdfBudget,
) -> Result<Document, PdfLoadError> {
    check_xref_stream_bounds(pdf_bytes)?;

    let objstm_doc = load_with_filter(pdf_bytes, Some(defer_object_streams))?;
    for object in objstm_doc.objects.values() {
        if let Ok(stream) = object.as_stream() {
            budget.measure_stream(stream).map_err(|err| match err {
                PdfStreamDecodeError::LimitExceeded => PdfLoadError::LimitExceeded(
                    "PDF /ObjStm object streams exceeded decompression safety budget".to_owned(),
                ),
                PdfStreamDecodeError::Corrupt(reason) => {
                    PdfLoadError::Parse(format!("corrupt /ObjStm stream: {reason}"))
                }
            })?;
        }
    }

    load_with_filter(pdf_bytes, None)
}

fn defer_object_streams(id: ObjectId, object: &mut Object) -> Option<(ObjectId, Object)> {
    let stream = object.as_stream_mut().ok()?;
    if !stream.dict.has_type(b"ObjStm") {
        return None;
    }
    stream.dict.set("Type", "DeferredObjStm");
    Some((id, Object::Null))
}

type ObjectFilter = fn(ObjectId, &mut Object) -> Option<(ObjectId, Object)>;

fn load_with_filter(
    pdf_bytes: &[u8],
    filter: Option<ObjectFilter>,
) -> Result<Document, PdfLoadError> {
    let opts = LoadOptions {
        filter,
        max_decompressed_size: Some(MAX_STREAM_BYTES),
        ..LoadOptions::default()
    };
    Document::load_mem_with_options(pdf_bytes, opts).map_err(|err| match err {
        lopdf::Error::Decompress(lopdf::DecompressError::MemoryLimitExceeded { .. }) => {
            PdfLoadError::LimitExceeded(format!(
                "PDF stream exceeded {MAX_STREAM_BYTES}-byte decompression limit"
            ))
        }
        other => PdfLoadError::Parse(other.to_string()),
    })
}

#[derive(Default)]
struct XrefDictFrame {
    array_depth: usize,
    w: Option<Vec<i64>>,
    index: Option<Vec<i64>>,
    size: Option<i64>,
}

impl XrefDictFrame {
    fn validate(self) -> Result<(), PdfLoadError> {
        let (Some(w), Some(size)) = (self.w, self.size) else {
            return Ok(());
        };
        if w.iter().any(|&n| !(0..=8).contains(&n)) {
            return Err(PdfLoadError::Parse(format!(
                "cross-reference stream declares invalid /W widths {w:?} (each must be 0..=8)"
            )));
        }
        let index = self.index.unwrap_or_else(|| vec![0, size]);
        let counts: Vec<i64> = index.chunks(2).filter_map(|c| c.get(1).copied()).collect();
        if counts.iter().any(|&n| !(0..=MAX_PDF_OBJECTS).contains(&n))
            || !(0..=MAX_PDF_OBJECTS).contains(&size)
        {
            return Err(PdfLoadError::Parse(format!(
                "cross-reference stream declares out-of-range /Size {size} or /Index counts {counts:?}"
            )));
        }
        if w.iter().all(|&n| n == 0) && counts.iter().any(|&n| n > 0) {
            return Err(PdfLoadError::Parse(
                "cross-reference stream declares zero-width entries".to_owned(),
            ));
        }
        Ok(())
    }
}

fn check_xref_stream_bounds(pdf: &[u8]) -> Result<(), PdfLoadError> {
    let mut stack: Vec<XrefDictFrame> = Vec::new();
    let mut i = 0;
    while i < pdf.len() {
        match pdf[i] {
            b'%' => i = skip_comment(pdf, i),
            b'(' => i = skip_literal_string(pdf, i),
            b'<' if pdf.get(i + 1) == Some(&b'<') => {
                stack.push(XrefDictFrame::default());
                i += 2;
            }
            b'<' => {
                while i < pdf.len() && pdf[i] != b'>' {
                    i += 1;
                }
                i += usize::from(i < pdf.len());
            }
            b'>' if pdf.get(i + 1) == Some(&b'>') => {
                if let Some(frame) = stack.pop() {
                    frame.validate()?;
                }
                i = skip_optional_stream_body(pdf, i + 2);
            }
            b'[' => {
                if let Some(d) = stack.last_mut() {
                    d.array_depth += 1;
                }
                i += 1;
            }
            b']' => {
                if let Some(d) = stack.last_mut() {
                    d.array_depth = d.array_depth.saturating_sub(1);
                }
                i += 1;
            }
            b'/' => i = scan_dict_key(pdf, i + 1, stack.last_mut()),
            _ => i += 1,
        }
    }
    Ok(())
}

fn skip_comment(pdf: &[u8], mut i: usize) -> usize {
    while i < pdf.len() && !matches!(pdf[i], b'\r' | b'\n') {
        i += 1;
    }
    i
}

fn skip_literal_string(pdf: &[u8], mut i: usize) -> usize {
    let mut depth = 0usize;
    while i < pdf.len() {
        match pdf[i] {
            b'\\' => i += 1,
            b'(' => depth += 1,
            b')' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return i + 1;
                }
            }
            _ => {}
        }
        i += 1;
    }
    i
}

fn skip_optional_stream_body(pdf: &[u8], after_dict: usize) -> usize {
    let mut j = after_dict;
    while j < pdf.len() && pdf[j].is_ascii_whitespace() {
        j += 1;
    }
    if pdf[j..].starts_with(b"stream") {
        match pdf[j..].windows(9).position(|w| w == b"endstream") {
            Some(end) => j + end + 9,
            None => pdf.len(),
        }
    } else {
        after_dict
    }
}

fn scan_dict_key(pdf: &[u8], start: usize, frame: Option<&mut XrefDictFrame>) -> usize {
    let is_delim = |b: u8| b.is_ascii_whitespace() || b"()<>[]{}/%".contains(&b) || b == 0;
    let mut i = start;
    while i < pdf.len() && !is_delim(pdf[i]) {
        i += 1;
    }
    if let Some(d) = frame
        && d.array_depth == 0
    {
        let key = decode_pdf_name(&pdf[start..i]);
        match key.as_slice() {
            b"W" => d.w = parse_int_array(pdf, &mut i),
            b"Index" => d.index = parse_int_array(pdf, &mut i),
            b"Size" => d.size = parse_int(pdf, &mut i),
            _ => {}
        }
    }
    i
}

fn decode_pdf_name(raw: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(raw.len());
    let mut k = 0;
    while k < raw.len() {
        if raw[k] == b'#'
            && let Some(byte) = raw
                .get(k + 1..k + 3)
                .and_then(|h| std::str::from_utf8(h).ok())
                .and_then(|h| u8::from_str_radix(h, 16).ok())
        {
            out.push(byte);
            k += 3;
        } else {
            out.push(raw[k]);
            k += 1;
        }
    }
    out
}

fn parse_int(pdf: &[u8], i: &mut usize) -> Option<i64> {
    let mut k = *i;
    while k < pdf.len() && pdf[k].is_ascii_whitespace() {
        k += 1;
    }
    let start = k;
    if k < pdf.len() && matches!(pdf[k], b'+' | b'-') {
        k += 1;
    }
    let digits = k;
    while k < pdf.len() && pdf[k].is_ascii_digit() {
        k += 1;
    }
    if k == digits {
        return None;
    }
    let text = std::str::from_utf8(&pdf[start..k]).ok()?;
    *i = k;
    Some(text.parse::<i64>().unwrap_or(if text.starts_with('-') {
        i64::MIN
    } else {
        i64::MAX
    }))
}

fn parse_int_array(pdf: &[u8], i: &mut usize) -> Option<Vec<i64>> {
    let mut k = *i;
    while k < pdf.len() && pdf[k].is_ascii_whitespace() {
        k += 1;
    }
    if pdf.get(k) != Some(&b'[') {
        return None;
    }
    k += 1;
    let mut out = Vec::new();
    loop {
        if let Some(n) = parse_int(pdf, &mut k) {
            out.push(n);
            continue;
        }
        while k < pdf.len() && pdf[k].is_ascii_whitespace() {
            k += 1;
        }
        if pdf.get(k) == Some(&b']') {
            *i = k + 1;
            return Some(out);
        }
        return None;
    }
}
