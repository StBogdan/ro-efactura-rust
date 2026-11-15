//! Single-pass bounded in-memory PDF container loader (`startxref`-chain `XRef` stream header
//! guards and in-place `/ObjStm` inflation accounting) shared by [`super::inspect_pdfa3`] and
//! [`super::extract_xml_from_pdf`].

use std::collections::{BTreeMap, HashSet, VecDeque};

use lopdf::xref::XrefEntry;
use lopdf::{
    DecompressError, Document, Error as LopdfError, LoadOptions, Object, ObjectId, ObjectStream,
};

use super::{MAX_STREAM_BYTES, PdfBudget};

/// Maximum indirect object count permitted by ISO 32000-1 Annex C (`8,388,607`).
const MAX_PDF_OBJECTS: i64 = 8_388_607;

/// Maximum `/Prev` / `/XRefStm` cross-reference revisions followed before aborting.
const MAX_XREF_REVISIONS: usize = 128;

pub(super) enum PdfLoadError {
    Parse(String),
    LimitExceeded(String),
}

/// Parses `pdf_bytes` into a [`lopdf::Document`] in a **single pass** while enforcing:
/// 1. Targeted `startxref` -> `/Prev` / `/XRefStm` `XRef` `/W` (`0..=8`) and `/Size` / `/Index`
///    (`0..=8_388_607`) pre-load bounds without scanning body streams.
/// 2. Single-pass `/ObjStm` deferral and in-place expansion under both per-stream (`64 MiB`) and
///    cumulative (`128 MiB`) decompression budgets.
pub(super) fn load_bounded_document(
    pdf_bytes: &[u8],
    budget: &mut PdfBudget,
) -> Result<Document, PdfLoadError> {
    check_xref_stream_bounds(pdf_bytes)?;

    let opts = LoadOptions {
        filter: Some(|id, object| {
            if let Ok(stream) = object.as_stream_mut()
                && stream.dict.has_type(b"ObjStm")
            {
                stream.dict.set("Type", "DeferredObjStm");
            }
            Some((id, Object::Null))
        }),
        max_decompressed_size: Some(MAX_STREAM_BYTES),
        ..LoadOptions::default()
    };
    let mut doc = Document::load_mem_with_options(pdf_bytes, opts).map_err(|err| match err {
        LopdfError::Decompress(DecompressError::MemoryLimitExceeded { .. }) => {
            PdfLoadError::LimitExceeded(format!(
                "PDF stream exceeded {MAX_STREAM_BYTES}-byte decompression limit"
            ))
        }
        other => PdfLoadError::Parse(other.to_string()),
    })?;
    expand_deferred_object_streams(&mut doc, budget)?;
    Ok(doc)
}

fn expand_deferred_object_streams(
    doc: &mut Document,
    budget: &mut PdfBudget,
) -> Result<(), PdfLoadError> {
    let deferred_ids: Vec<ObjectId> = doc
        .objects
        .iter()
        .filter_map(|(&id, obj)| {
            obj.as_stream()
                .ok()
                .filter(|s| s.dict.has_type(b"DeferredObjStm"))
                .map(|_| id)
        })
        .collect();
    if deferred_ids.is_empty() {
        return Ok(());
    }

    let compressed_containers: BTreeMap<u32, u32> = doc
        .reference_table
        .entries
        .iter()
        .filter_map(|(&obj_num, entry)| match entry {
            XrefEntry::Compressed { container, .. } => Some((obj_num, *container)),
            _ => None,
        })
        .collect();

    let mut unpacked: Vec<(ObjectId, Object)> = Vec::new();
    for container_id in deferred_ids {
        let Some(stream) = doc
            .objects
            .get_mut(&container_id)
            .and_then(|o| o.as_stream_mut().ok())
        else {
            continue;
        };
        stream.dict.set("Type", "ObjStm");

        let mut temp_stream = stream.clone();
        let cap = budget.stream_cap();
        let obj_stream =
            ObjectStream::new_with_limit(&mut temp_stream, Some(cap)).map_err(|err| match err {
                LopdfError::Decompress(DecompressError::MemoryLimitExceeded { .. }) => {
                    PdfLoadError::LimitExceeded(
                        "PDF /ObjStm object streams exceeded decompression safety budget"
                            .to_owned(),
                    )
                }
                other => PdfLoadError::Parse(format!("corrupt /ObjStm stream: {other}")),
            })?;
        budget.consume(temp_stream.content.len());

        for (obj_id, obj) in obj_stream.objects {
            if compressed_containers
                .get(&obj_id.0)
                .is_none_or(|&c| c == container_id.0)
            {
                unpacked.push((obj_id, obj));
            }
        }
    }

    for (obj_id, obj) in unpacked {
        doc.objects.entry(obj_id).or_insert(obj);
    }
    Ok(())
}

#[derive(Default)]
struct XrefDictFrame {
    array_depth: usize,
    w: Option<Vec<i64>>,
    index: Option<Vec<i64>>,
    size: Option<i64>,
    prev: Option<i64>,
    xref_stm: Option<i64>,
}

impl XrefDictFrame {
    fn validate(&self) -> Result<(), PdfLoadError> {
        let (Some(w), Some(size)) = (&self.w, self.size) else {
            return Ok(());
        };
        if w.iter().any(|&n| !(0..=8).contains(&n)) {
            return Err(PdfLoadError::Parse(format!(
                "cross-reference stream declares invalid /W widths {w:?} (each must be 0..=8)"
            )));
        }
        let default_index = [0, size];
        let index_slice = self.index.as_deref().unwrap_or(&default_index);
        let counts: Vec<i64> = index_slice
            .chunks(2)
            .filter_map(|c| c.get(1).copied())
            .collect();
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
    let header_window = &pdf[..pdf.len().min(1024)];
    let Some(hdr_pos) = header_window.windows(5).position(|w| w == b"%PDF-") else {
        return Err(PdfLoadError::Parse("missing %PDF- header".to_owned()));
    };
    let buf = &pdf[hdr_pos..];
    let Some(initial_offset) = find_startxref_offset(buf) else {
        return Ok(());
    };

    let mut queue = VecDeque::from([initial_offset]);
    let mut visited = HashSet::new();
    while let Some(offset) = queue.pop_front() {
        if offset >= buf.len() || !visited.insert(offset) {
            continue;
        }
        if visited.len() > MAX_XREF_REVISIONS {
            return Err(PdfLoadError::LimitExceeded(format!(
                "PDF XRef /Prev chain exceeded {MAX_XREF_REVISIONS} revisions"
            )));
        }
        if let Some(dict_pos) = locate_xref_or_trailer_dict(buf, offset) {
            let frame = parse_xref_or_trailer_dict(buf, dict_pos)?;
            for link in [frame.prev, frame.xref_stm].into_iter().flatten() {
                if let Ok(next_off) = usize::try_from(link) {
                    queue.push_back(next_off);
                }
            }
        }
    }
    Ok(())
}

fn find_startxref_offset(buf: &[u8]) -> Option<usize> {
    let tail_start = buf.len().saturating_sub(512);
    let eof_pos = rfind_bytes(buf, b"%%EOF", tail_start).filter(|&p| p > 25)?;
    let kw_pos = rfind_bytes(&buf[..eof_pos], b"startxref", eof_pos.saturating_sub(64))?;
    let mut i = kw_pos + b"startxref".len();
    let raw_offset = parse_int(buf, &mut i)?;
    usize::try_from(raw_offset).ok()
}

fn rfind_bytes(buf: &[u8], pattern: &[u8], start_pos: usize) -> Option<usize> {
    buf.get(start_pos..)?
        .windows(pattern.len())
        .rposition(|w| w == pattern)
        .map(|p| start_pos + p)
}

fn locate_xref_or_trailer_dict(buf: &[u8], offset: usize) -> Option<usize> {
    let pos = skip_ws_and_comments(buf, offset);
    if buf.get(pos..)?.starts_with(b"xref") && buf.get(pos + 4).is_none_or(|&b| is_pdf_delim(b)) {
        return skip_to_trailer_dict(buf, pos + 4);
    }

    let mut i = pos;
    let d0 = i;
    while i < buf.len() && buf[i].is_ascii_digit() {
        i += 1;
    }
    if i == d0 {
        return None;
    }
    i = skip_ws_and_comments(buf, i);
    let d1 = i;
    while i < buf.len() && buf[i].is_ascii_digit() {
        i += 1;
    }
    if i == d1 {
        return None;
    }
    i = skip_ws_and_comments(buf, i);
    if !buf.get(i..)?.starts_with(b"obj") || !buf.get(i + 3).is_none_or(|&b| is_pdf_delim(b)) {
        return None;
    }
    let dict_pos = skip_ws_and_comments(buf, i + 3);
    buf.get(dict_pos..)?.starts_with(b"<<").then_some(dict_pos)
}

fn skip_to_trailer_dict(buf: &[u8], mut i: usize) -> Option<usize> {
    while i < buf.len() {
        i = skip_ws_and_comments(buf, i);
        if buf.get(i..).is_some_and(|s| s.starts_with(b"trailer"))
            && buf.get(i + 7).is_none_or(|&b| is_pdf_delim(b))
        {
            let dict_pos = skip_ws_and_comments(buf, i + 7);
            return buf
                .get(dict_pos..)
                .is_some_and(|s| s.starts_with(b"<<"))
                .then_some(dict_pos);
        }
        if buf.get(i) == Some(&b'<') {
            return None;
        }
        while i < buf.len() && !matches!(buf[i], b'\r' | b'\n') {
            i += 1;
        }
    }
    None
}

fn parse_xref_or_trailer_dict(
    buf: &[u8],
    dict_start: usize,
) -> Result<XrefDictFrame, PdfLoadError> {
    let mut stack = vec![XrefDictFrame::default()];
    let mut i = dict_start + 2;
    while i < buf.len() {
        match buf[i] {
            b'%' => i = skip_comment(buf, i),
            b'(' => i = skip_literal_string(buf, i),
            b'<' if buf.get(i + 1) == Some(&b'<') => {
                stack.push(XrefDictFrame::default());
                i += 2;
            }
            b'<' => {
                while i < buf.len() && buf[i] != b'>' {
                    i += 1;
                }
                i += usize::from(i < buf.len());
            }
            b'>' if buf.get(i + 1) == Some(&b'>') => {
                if let Some(frame) = stack.pop() {
                    frame.validate()?;
                    if stack.is_empty() {
                        return Ok(frame);
                    }
                }
                i += 2;
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
            b'/' => i = scan_dict_key(buf, i + 1, stack.last_mut()),
            _ => i += 1,
        }
    }
    Ok(stack.pop().unwrap_or_default())
}

const fn is_pdf_delim(b: u8) -> bool {
    b.is_ascii_whitespace()
        || matches!(
            b,
            b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%' | 0
        )
}

fn skip_ws_and_comments(pdf: &[u8], mut i: usize) -> usize {
    loop {
        while i < pdf.len() && pdf[i].is_ascii_whitespace() {
            i += 1;
        }
        if pdf.get(i) == Some(&b'%') {
            i = skip_comment(pdf, i);
        } else {
            return i;
        }
    }
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

fn scan_dict_key(pdf: &[u8], start: usize, frame: Option<&mut XrefDictFrame>) -> usize {
    let mut i = start;
    while i < pdf.len() && !is_pdf_delim(pdf[i]) {
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
            b"Prev" => d.prev = parse_int(pdf, &mut i),
            b"XRefStm" => d.xref_stm = parse_int(pdf, &mut i),
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
                .and_then(|h| core::str::from_utf8(h).ok())
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
    let mut k = skip_ws_and_comments(pdf, *i);
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
    let text = core::str::from_utf8(&pdf[start..k]).ok()?;
    *i = k;
    Some(text.parse::<i64>().unwrap_or(if text.starts_with('-') {
        i64::MIN
    } else {
        i64::MAX
    }))
}

fn parse_int_array(pdf: &[u8], i: &mut usize) -> Option<Vec<i64>> {
    let mut k = skip_ws_and_comments(pdf, *i);
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
        k = skip_ws_and_comments(pdf, k);
        if pdf.get(k) == Some(&b']') {
            *i = k + 1;
            return Some(out);
        }
        return None;
    }
}
