//! Phase 2c of the native .pub reader: positions.
//!
//! An independent implementation from documented format
//! facts (the public reverse-engineering record summarized
//! in the project's research notes, plus Microsoft's
//! published [MS-ODRAW] for the drawing layer's record
//! framing; no libmspub code). It joins the words phase 2b
//! recovered to the boxes Publisher put them in:
//!
//! - The root `Contents` stream (2002 family, magic 0x2C)
//!   is a grammar of (id, type) blocks. Its trailer
//!   directory assigns every page and shape a sequence
//!   number (seqNum); chunk references in the directory
//!   point at the page, shape, document, and table chunks.
//!   A shape chunk carries a text id that joins to a Quill
//!   story through the SYID table.
//! - The Escher stream (`Escher/EscherStm`) holds geometry:
//!   each shape instance names its seqNum in its Client
//!   Data record (property 0x6801), and its anchor record
//!   gives the box, in EMU (12700 to a point).
//!
//! Strictness, as settled in the Tier-2 synthesis: never
//! guess a length. An unknown block type inside a
//! length-carrying container abandons that container and
//! parsing resumes at its declared end; at an unbounded
//! level (the chunk directory) it aborts the layout layer
//! only, and the caller keeps the phase 2b text result.
//! Unknown (id, type) pairs are logged in the loss report
//! so the framing table grows from data.
//!
//! Every story is emitted exactly once: placed in its
//! shape's box, or, when no shape claims it or its shape
//! has no position, on reading-order pages at the end,
//! still badged Check this and counted as unplaced.

use crate::cfb::Cfb;
use crate::pub_text::{
    contents_version, is_variable_len, parse_quill, payload_fixed_len, payload_int,
    reading_column_dump, reading_column_paras, reading_column_spans, span_font, LossReport,
    QuillParse,
};
use crate::{ExtractionDump, PageInfo, SourceKind, Span, SpanTrust, DUMP_SCHEMA_VERSION};
use std::collections::{HashMap, HashSet};

const EMU_PER_PT: f64 = 12_700.0;
/// Publisher's default text inset when a shape's property
/// set does not state one (EMU, 0.04 inch).
const DEFAULT_INSET_EMU: i64 = 36_576;
/// Page seqNums the format reserves as dummies.
const DUMMY_PAGES: [u32; 4] = [0x10D, 0x110, 0x113, 0x117];

fn emu_pt(v: i64) -> f32 {
    (v as f64 / EMU_PER_PT) as f32
}

fn u32_at(b: &[u8], off: usize) -> Option<u32> {
    let s = b.get(off..off + 4)?;
    Some(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

/// The positioned rescue's result: the dump, its loss
/// report, and whether positions were read (false means the
/// dump is the phase 2b text-only result).
pub struct PubExtraction {
    pub dump: ExtractionDump,
    pub loss: LossReport,
    pub positioned: bool,
}

/// The whole native rescue: positioned when the layout can
/// be read, the phase 2b text result when it cannot. The
/// version gate is phase 2b's: only the 0x2C family.
pub fn extract_pub_positioned(
    cfb: &Cfb,
    source_name: &str,
    source_bytes: usize,
) -> Result<PubExtraction, String> {
    match contents_version(cfb) {
        Some(0x2C) => {}
        Some(0x22) => {
            return Err(
                "that Publisher file is from Publisher 2000 or earlier (the 0x22 \
                 family); the built-in reader covers Publisher 2002 and later"
                    .into(),
            )
        }
        _ => return Err("that file has no Publisher document stream we recognize".into()),
    }
    let quill = cfb
        .read_stream_path(&["Quill", "QuillSub", "CONTENTS"])
        .map_err(|e| format!("that file's text stream could not be read: {e}"))?;
    let (parse, loss) = parse_quill(&quill)?;
    match try_positioned(cfb, &parse, loss.clone(), source_name, source_bytes) {
        Some((dump, loss)) => Ok(PubExtraction {
            dump,
            loss,
            positioned: true,
        }),
        None => {
            let (dump, mut loss) = reading_column_dump(&parse, loss, source_name, source_bytes);
            loss.notes.push(
                "the page layout in that file could not be read, so the words \
                 are in reading order, without their places"
                    .into(),
            );
            Ok(PubExtraction {
                dump,
                loss,
                positioned: false,
            })
        }
    }
}

fn try_positioned(
    cfb: &Cfb,
    parse: &QuillParse,
    mut loss: LossReport,
    source_name: &str,
    source_bytes: usize,
) -> Option<(ExtractionDump, LossReport)> {
    let contents = cfb.read_stream_path(&["Contents"]).ok()?;
    let mut unknowns: Vec<(u8, u8)> = Vec::new();
    let layout = parse_contents(&contents, &mut loss.notes, &mut unknowns)?;
    if !unknowns.is_empty() {
        let mut kinds: Vec<(u8, u8)> = unknowns.clone();
        kinds.sort();
        kinds.dedup();
        let listed: Vec<String> = kinds
            .iter()
            .take(8)
            .map(|(id, ty)| format!("id {id:#04x} type {ty:#04x}"))
            .collect();
        loss.notes.push(format!(
            "the layout walk met {} block kind(s) whose size it cannot know \
             ({}); whatever sat inside those containers was skipped",
            kinds.len(),
            listed.join(", ")
        ));
    }
    let escher_bytes = cfb.read_stream_path(&["Escher", "EscherStm"]).ok()?;
    let escher = walk_escher(&escher_bytes);
    loss.pictures_found = escher.blips;
    let dump = compose(
        parse,
        &layout,
        &escher,
        &mut loss,
        source_name,
        source_bytes,
    );
    Some((dump, loss))
}

// ----------------------------------------------------------------
// The Contents stream: the scoped-strict block walk.

/// One Contents block. `payload` is the framed value: for
/// fixed-size types the value bytes, for variable-length
/// (container) types the content after the length word.
struct CBlock<'a> {
    id: u8,
    ty: u8,
    /// Offset of the block header within the walked slice.
    start: usize,
    payload: &'a [u8],
    container: bool,
}

/// Walk blocks until the slice ends or a block cannot be
/// framed. Returns the blocks read and whether the walk
/// ended cleanly; an unknown type byte is recorded and
/// stops the walk (never guess a length).
fn walk_scoped<'a>(bytes: &'a [u8], unknowns: &mut Vec<(u8, u8)>) -> (Vec<CBlock<'a>>, bool) {
    let mut out = Vec::new();
    let mut pos = 0usize;
    let mut clean = true;
    while pos + 2 <= bytes.len() {
        let (id, ty) = (bytes[pos], bytes[pos + 1]);
        if let Some(fixed) = payload_fixed_len(ty) {
            let Some(payload) = bytes.get(pos + 2..pos + 2 + fixed) else {
                clean = false;
                break;
            };
            out.push(CBlock {
                id,
                ty,
                start: pos,
                payload,
                container: false,
            });
            pos += 2 + fixed;
        } else if is_variable_len(ty) {
            let Some(total) = u32_at(bytes, pos + 2).map(|v| v as usize) else {
                clean = false;
                break;
            };
            if total < 4 || pos + 2 + total > bytes.len() {
                clean = false;
                break;
            }
            out.push(CBlock {
                id,
                ty,
                start: pos,
                payload: &bytes[pos + 6..pos + 2 + total],
                container: true,
            });
            pos += 2 + total;
        } else {
            unknowns.push((id, ty));
            clean = false;
            break;
        }
    }
    (out, clean)
}

// ----------------------------------------------------------------
// Contents layout: pages, shapes, tables.

#[derive(Clone, Copy)]
struct ChunkRef {
    seq: u32,
    ty: u8,
    offset: usize,
    parent: u32,
}

#[derive(Default)]
struct PageChunk {
    shapes: Vec<u32>,
    applied_master: Option<u32>,
    is_master: bool,
}

#[derive(Default, Clone, Copy)]
struct CellRect {
    start_row: usize,
    end_row: usize,
    start_col: usize,
    end_col: usize,
}

#[derive(Default)]
struct TableChunk {
    rows: usize,
    cols: usize,
    /// Column widths then row heights, EMU, from the
    /// row/column size array when the file carries it.
    col_widths: Vec<i64>,
    row_heights: Vec<i64>,
    cells: Vec<CellRect>,
}

#[derive(Default)]
struct ShapeChunk {
    ty: u8,
    parent: u32,
    text_id: Option<u32>,
    table: Option<TableChunk>,
}

struct ContentsLayout {
    page_w_pt: f32,
    page_h_pt: f32,
    page_order: Vec<u32>,
    pages: HashMap<u32, PageChunk>,
    shapes: HashMap<u32, ShapeChunk>,
}

/// The byte range blocks live in for one chunk: the chunk
/// body starts with its own U32 length; blocks occupy the
/// body after it. The extent (next chunk's offset, or the
/// trailer's start for the last chunk) caps the declared
/// length, per the cross-check the plan requires.
fn chunk_region<'a>(
    contents: &'a [u8],
    offset: usize,
    extent_end: usize,
    notes: &mut Vec<String>,
    seq: u32,
) -> Option<&'a [u8]> {
    let slice = contents.get(offset..extent_end)?;
    if slice.len() < 4 {
        return None;
    }
    let declared = u32_at(slice, 0)? as usize;
    if declared > slice.len() {
        notes.push(format!(
            "chunk {seq} declares more bytes than the file gives it; read what fits"
        ));
    }
    let bound = declared.max(4).min(slice.len());
    Some(&slice[4..bound])
}

fn parse_contents(
    contents: &[u8],
    notes: &mut Vec<String>,
    unknowns: &mut Vec<(u8, u8)>,
) -> Option<ContentsLayout> {
    if contents.len() < 0x1E {
        return None;
    }
    let trailer_off = u32_at(contents, 0x1A)? as usize;
    let trailer_len = u32_at(contents, trailer_off)? as usize;
    if trailer_len < 4 {
        return None;
    }
    let trailer_end = trailer_off.saturating_add(trailer_len).min(contents.len());
    let trailer = contents.get(trailer_off + 4..trailer_end)?;
    let (tblocks, _) = walk_scoped(trailer, unknowns);
    let directory = tblocks.iter().find(|b| b.ty == 0x90)?;
    let (dblocks, clean) = walk_scoped(directory.payload, unknowns);
    if !clean {
        notes.push(
            "the file's chunk directory holds a block whose size cannot be \
             known; the layout was not read"
                .into(),
        );
        return None;
    }
    // Directory ordinals are the seqNums; chunk references
    // are the 0x88 blocks among them.
    let mut refs: Vec<ChunkRef> = Vec::new();
    for (i, b) in dblocks.iter().enumerate() {
        if b.ty != 0x88 {
            continue;
        }
        let (subs, _) = walk_scoped(b.payload, unknowns);
        let (mut ty, mut off, mut parent) = (None, None, 0u32);
        for sb in &subs {
            match sb.id {
                0x02 => ty = Some(payload_int(sb.payload) as u32),
                0x04 => off = Some(payload_int(sb.payload) as usize),
                0x05 => parent = payload_int(sb.payload) as u32,
                _ => {}
            }
        }
        if let (Some(t), Some(o)) = (ty, off) {
            refs.push(ChunkRef {
                seq: i as u32,
                ty: t as u8,
                offset: o,
                parent,
            });
        }
    }
    if refs.is_empty() {
        notes.push("the file's chunk directory names no chunks; the layout was not read".into());
        return None;
    }
    let mut offsets: Vec<usize> = refs.iter().map(|r| r.offset).collect();
    offsets.sort_unstable();
    offsets.dedup();
    let extent_end = |offset: usize| -> usize {
        offsets
            .iter()
            .copied()
            .find(|o| *o > offset)
            .unwrap_or(trailer_off)
            .min(contents.len())
    };
    let region_of = |r: &ChunkRef, notes: &mut Vec<String>| {
        chunk_region(contents, r.offset, extent_end(r.offset), notes, r.seq)
    };

    // The document chunk: page size and the page list.
    let doc_ref = refs.iter().find(|r| r.ty == 0x44)?;
    let region = region_of(doc_ref, notes)?;
    let (blocks, _) = walk_scoped(region, unknowns);
    let (mut page_w, mut page_h) = (0i64, 0i64);
    let mut page_order: Vec<u32> = Vec::new();
    for b in &blocks {
        if b.id == 0x12 && b.container {
            let (subs, _) = walk_scoped(b.payload, unknowns);
            for sb in &subs {
                match sb.id {
                    0x01 => page_w = payload_int(sb.payload) as u32 as i32 as i64,
                    0x02 => page_h = payload_int(sb.payload) as u32 as i32 as i64,
                    _ => {}
                }
            }
        } else if b.id == 0x02 && b.container {
            let (subs, _) = walk_scoped(b.payload, unknowns);
            for sb in &subs {
                if sb.id == 0x00 {
                    page_order.push(payload_int(sb.payload) as u32);
                }
            }
        }
    }
    if page_w <= 0 || page_h <= 0 || page_order.is_empty() {
        notes.push(
            "the document chunk gave no page size or page list; the layout was not read".into(),
        );
        return None;
    }

    // Table cells chunks first: tables point at them.
    let mut cells_by_seq: HashMap<u32, Vec<CellRect>> = HashMap::new();
    for r in refs.iter().filter(|r| r.ty == 0x63) {
        let Some(region) = region_of(r, notes) else {
            continue;
        };
        let (blocks, _) = walk_scoped(region, unknowns);
        let mut cells = Vec::new();
        for b in &blocks {
            if b.id == 0x02 && b.container {
                let (entries, _) = walk_scoped(b.payload, unknowns);
                for e in &entries {
                    if e.id == 0x00 && e.container {
                        let (fields, _) = walk_scoped(e.payload, unknowns);
                        let mut cell = CellRect::default();
                        for f in &fields {
                            match f.id {
                                0x01 => cell.start_row = payload_int(f.payload) as usize,
                                0x02 => cell.end_row = payload_int(f.payload) as usize,
                                0x03 => cell.start_col = payload_int(f.payload) as usize,
                                0x04 => cell.end_col = payload_int(f.payload) as usize,
                                _ => {}
                            }
                        }
                        cells.push(cell);
                    }
                }
            }
        }
        cells_by_seq.insert(r.seq, cells);
    }

    // Page chunks.
    let mut pages: HashMap<u32, PageChunk> = HashMap::new();
    for r in refs.iter().filter(|r| r.ty == 0x43) {
        let Some(region) = region_of(r, notes) else {
            continue;
        };
        let (blocks, _) = walk_scoped(region, unknowns);
        let mut page = PageChunk::default();
        for b in &blocks {
            match b.id {
                0x02 if b.container => {
                    let (subs, _) = walk_scoped(b.payload, unknowns);
                    for sb in &subs {
                        if sb.ty == 0x70 {
                            page.shapes.push(payload_int(sb.payload) as u32);
                        }
                    }
                }
                0x0E => {
                    if b.payload.iter().any(|x| *x != 0) {
                        page.is_master = true;
                    }
                }
                0x0D => page.applied_master = Some(payload_int(b.payload) as u32),
                _ => {}
            }
        }
        pages.insert(r.seq, page);
    }

    // Shape chunks (and groups, logos, tables).
    let mut shapes: HashMap<u32, ShapeChunk> = HashMap::new();
    for r in refs
        .iter()
        .filter(|r| matches!(r.ty, 0x01 | 0x20 | 0x30 | 0x31 | 0x10))
    {
        let Some(region) = region_of(r, notes) else {
            continue;
        };
        let (blocks, _) = walk_scoped(region, unknowns);
        let mut shape = ShapeChunk {
            ty: r.ty,
            parent: r.parent,
            ..Default::default()
        };
        let mut table = TableChunk::default();
        let mut cells_seq: Option<u32> = None;
        let mut is_table = r.ty == 0x10;
        for b in &blocks {
            match b.id {
                0x27 => shape.text_id = Some(payload_int(b.payload) as u32),
                0x66 if is_table => table.rows = payload_int(b.payload) as usize,
                0x67 if is_table => table.cols = payload_int(b.payload) as usize,
                0x6B if is_table => cells_seq = Some(payload_int(b.payload) as u32),
                0x6D if is_table && b.container => {
                    // The size array lives at this block's
                    // own offset in the stream (its length
                    // word position): walk it there.
                    let array_at = r.offset + 4 + b.start + 2;
                    if let Some(sizes) = rowcol_sizes(contents, array_at, unknowns) {
                        let nc = table.cols;
                        let nr = table.rows;
                        for v in sizes {
                            if table.col_widths.len() < nc {
                                table.col_widths.push(v);
                            } else if table.row_heights.len() < nr {
                                table.row_heights.push(v);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        if r.ty != 0x10 {
            is_table = false;
        }
        if is_table {
            if let Some(cs) = cells_seq {
                table.cells = cells_by_seq.get(&cs).cloned().unwrap_or_default();
            }
            shape.table = Some(table);
        }
        shapes.insert(r.seq, shape);
    }

    Some(ContentsLayout {
        page_w_pt: emu_pt(page_w),
        page_h_pt: emu_pt(page_h),
        page_order,
        pages,
        shapes,
    })
}

/// The table row/column size array at a stream offset: a
/// U32 length, then entries (id 0x00) whose sub-blocks
/// (id 0x02) carry the sizes in EMU, columns first.
fn rowcol_sizes(
    contents: &[u8],
    array_at: usize,
    unknowns: &mut Vec<(u8, u8)>,
) -> Option<Vec<i64>> {
    let total = u32_at(contents, array_at)? as usize;
    if total < 4 {
        return None;
    }
    let end = array_at.saturating_add(total).min(contents.len());
    let region = contents.get(array_at + 4..end)?;
    let (entries, _) = walk_scoped(region, unknowns);
    let mut sizes = Vec::new();
    for e in &entries {
        if e.id == 0x00 && e.container {
            let (subs, _) = walk_scoped(e.payload, unknowns);
            for sb in &subs {
                if sb.id == 0x02 {
                    sizes.push(payload_int(sb.payload) as u32 as i32 as i64);
                }
            }
        }
    }
    Some(sizes)
}

// ----------------------------------------------------------------
// The Escher stream ([MS-ODRAW] record framing; the
// Publisher-specific payloads from the research notes).

/// One drawing record: header (version/instance, type,
/// content length) and its content bytes.
struct Rec<'a> {
    ver_inst: u16,
    ty: u16,
    content: &'a [u8],
}

/// The records at one level of the stream, in order. The
/// walk stops at the first record it cannot frame. Drawing
/// group and drawing containers carry 4 extra bytes after
/// their contents that their length does not count.
fn records(bytes: &[u8]) -> Vec<Rec<'_>> {
    let mut out = Vec::new();
    let mut pos = 0usize;
    while pos + 8 <= bytes.len() {
        let ver_inst = u16::from_le_bytes([bytes[pos], bytes[pos + 1]]);
        let ty = u16::from_le_bytes([bytes[pos + 2], bytes[pos + 3]]);
        let len = u32_at(bytes, pos + 4).unwrap_or(0) as usize;
        let cstart = pos + 8;
        let cend = cstart.saturating_add(len).min(bytes.len());
        out.push(Rec {
            ver_inst,
            ty,
            content: &bytes[cstart..cend],
        });
        let tail = if ty == 0xF000 || ty == 0xF002 { 4 } else { 0 };
        let next = cstart.saturating_add(len).saturating_add(tail);
        if next <= pos || cend < cstart + len {
            break; // framed past the end: nothing sane follows
        }
        pos = next;
    }
    out
}

/// (U16 id, U32 value) pairs from `skip` bytes into a
/// record's content: the shape of Client Data, Client
/// Anchor, and the FSP body (after its shape id).
fn pairs(content: &[u8], skip: usize) -> HashMap<u16, u32> {
    let mut out = HashMap::new();
    let mut pos = skip;
    while pos + 6 <= content.len() {
        let id = u16::from_le_bytes([content[pos], content[pos + 1]]);
        let value = u32_at(content, pos + 2).unwrap_or(0);
        out.insert(id, value);
        pos += 6;
    }
    out
}

/// A property set (FOPT and its tertiary sibling): scalar
/// entries first, then the complex payloads in entry order.
#[derive(Default)]
struct Fopt {
    scalars: HashMap<u16, u32>,
    complex: HashMap<u16, Vec<u8>>,
}

impl Fopt {
    /// A scalar by property id. Stored keys carry the
    /// complex and blip-id bits, so an exact hit wins and
    /// a masked-id hit is accepted after.
    fn scalar(&self, pid: u16) -> Option<u32> {
        if let Some(v) = self.scalars.get(&pid) {
            return Some(*v);
        }
        self.scalars
            .iter()
            .find(|(k, _)| **k & 0x3FFF == pid & 0x3FFF)
            .map(|(_, v)| *v)
    }

    fn complex(&self, pid: u16) -> Option<&[u8]> {
        if let Some(v) = self.complex.get(&pid) {
            return Some(v);
        }
        self.complex
            .iter()
            .find(|(k, _)| **k & 0x3FFF == pid & 0x3FFF)
            .map(|(_, v)| v.as_slice())
    }
}

fn parse_fopt(rec: &Rec) -> Fopt {
    let mut fopt = Fopt::default();
    let n = (rec.ver_inst >> 4) as usize;
    let mut complex_ids: Vec<u16> = Vec::new();
    let mut pos = 0usize;
    for _ in 0..n {
        let Some(entry) = rec.content.get(pos..pos + 6) else {
            break;
        };
        let raw = u16::from_le_bytes([entry[0], entry[1]]);
        let value = u32::from_le_bytes([entry[2], entry[3], entry[4], entry[5]]);
        fopt.scalars.insert(raw, value);
        if raw & 0x8000 != 0 {
            complex_ids.push(raw);
        }
        pos += 6;
    }
    for raw in complex_ids {
        let len = fopt.scalars.get(&raw).copied().unwrap_or(0) as usize;
        let end = pos.saturating_add(len).min(rec.content.len());
        if pos <= end {
            fopt.complex.insert(raw, rec.content[pos..end].to_vec());
        }
        pos = end;
    }
    fopt
}

/// Decode a WordArt geometry-text payload. Two shapes are
/// seen in the wild: the bare UTF-16LE string, and the
/// array form (a U16 count, 2 skipped bytes, a U16 entry
/// size of 2, then the characters) the generic complex
/// reader in the record assumes. Take whichever decodes
/// to more letters.
fn wordart_text(payload: &[u8]) -> Option<String> {
    let clean = |bytes: &[u8]| -> String {
        let units: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        String::from_utf16_lossy(&units)
            .trim_matches('\0')
            .trim()
            .to_string()
    };
    let letters = |s: &str| s.chars().filter(|c| c.is_alphanumeric()).count();
    let direct = clean(payload);
    let mut headed = String::new();
    if payload.len() >= 6 {
        let n = u16::from_le_bytes([payload[0], payload[1]]) as usize;
        let esz = u16::from_le_bytes([payload[4], payload[5]]) as usize;
        if esz == 2 && 6 + 2 * n <= payload.len() {
            headed = clean(&payload[6..6 + 2 * n]);
        }
    }
    let best = if letters(&headed) > letters(&direct) {
        headed
    } else {
        direct
    };
    if letters(&best) > 0 {
        Some(best)
    } else {
        None
    }
}

/// An EMU rectangle: left, top, right, bottom.
type EmuRect = (i64, i64, i64, i64);

fn arrange(r: EmuRect) -> EmuRect {
    let (mut xs, mut ys, mut xe, mut ye) = r;
    if xs > xe {
        std::mem::swap(&mut xs, &mut xe);
    }
    if ys > ye {
        std::mem::swap(&mut ys, &mut ye);
    }
    (xs, ys, xe, ye)
}

struct EscherShape {
    /// The anchor box in EMU, when an anchor was found.
    /// Escher coordinates are relative to the CENTER of
    /// the page (the reference's Coordinate type adds half
    /// the page size back); composition translates.
    rect: Option<EmuRect>,
    deleted: bool,
    /// Rotation in degrees, normalized to [0, 360).
    rotation: f32,
    flip_h: bool,
    flip_v: bool,
    /// Text insets (left, top, right, bottom) in points.
    insets_pt: (f32, f32, f32, f32),
    columns: u32,
    picture_id: Option<u32>,
    wordart: Option<String>,
}

#[derive(Default)]
struct EscherDoc {
    by_seq: HashMap<u32, EscherShape>,
    /// Image store entries (FBSE records in the blip
    /// store): the file's picture count.
    blips: usize,
}

const MAX_GROUP_DEPTH: usize = 100;

fn walk_escher(bytes: &[u8]) -> EscherDoc {
    let mut doc = EscherDoc::default();
    walk_level(bytes, (0, 0, 0, 0), (0, 0, 0, 0), &mut doc, 0);
    doc
}

fn walk_level(bytes: &[u8], coord: EmuRect, abs: EmuRect, doc: &mut EscherDoc, depth: usize) {
    if depth > MAX_GROUP_DEPTH {
        return;
    }
    for rec in records(bytes) {
        match rec.ty {
            0xF001 => {
                doc.blips += records(rec.content)
                    .iter()
                    .filter(|r| r.ty == 0xF007)
                    .count();
            }
            // Drawing group / drawing containers: their
            // children live at the same level of the walk.
            0xF000 | 0xF002 => walk_level(rec.content, coord, abs, doc, depth + 1),
            0xF003 => walk_group(rec.content, coord, abs, doc, depth + 1),
            0xF004 => {
                let (mut c, mut a) = (coord, abs);
                walk_shape(rec.content, &mut c, &mut a, doc);
            }
            _ => {}
        }
    }
}

/// A shape-group container: its first shape is the group
/// leader, whose FSPGR sets the child coordinate system
/// and whose anchor sets the group's absolute rectangle
/// for the shapes that follow it in the group.
fn walk_group(bytes: &[u8], coord: EmuRect, abs: EmuRect, doc: &mut EscherDoc, depth: usize) {
    if depth > MAX_GROUP_DEPTH {
        return;
    }
    let (mut coord, mut abs) = (coord, abs);
    for rec in records(bytes) {
        match rec.ty {
            0xF001 => {
                doc.blips += records(rec.content)
                    .iter()
                    .filter(|r| r.ty == 0xF007)
                    .count();
            }
            0xF003 => walk_group(rec.content, coord, abs, doc, depth + 1),
            0xF004 => walk_shape(rec.content, &mut coord, &mut abs, doc),
            _ => {}
        }
    }
}

fn walk_shape(content: &[u8], coord: &mut EmuRect, abs: &mut EmuRect, doc: &mut EscherDoc) {
    let mut fspgr: Option<&Rec> = None;
    let mut fsp: Option<&Rec> = None;
    let mut fopt: Option<&Rec> = None;
    let mut tertiary: Option<&Rec> = None;
    let mut cdata: Option<&Rec> = None;
    let mut anchor: Option<&Rec> = None;
    let recs = records(content);
    for rec in &recs {
        match rec.ty {
            0xF009 => fspgr = Some(rec),
            0xF00A => fsp = Some(rec),
            0xF00B => fopt = Some(rec),
            0xF122 => tertiary = Some(rec),
            0xF011 => cdata = Some(rec),
            0xF00F | 0xF010 if anchor.is_none() => anchor = Some(rec),
            _ => {}
        }
    }
    // The group leader's child coordinate system governs
    // the shapes that follow it in its group (and is set
    // even when the leader itself joins to nothing).
    let snapshot_coord = *coord;
    let mut defines_relative = false;
    if let Some(g) = fspgr {
        if g.content.len() >= 16 {
            let v = |i: usize| u32_at(g.content, i * 4).unwrap_or(0) as i32 as i64;
            *coord = arrange((v(0), v(1), v(2), v(3)));
            defines_relative = true;
        }
    }
    let flags = fsp.and_then(|f| u32_at(f.content, 4)).unwrap_or(0);
    let group_leader = flags & 0x01 != 0;
    let _ = group_leader;
    // The sole join to Contents: the Client Data seqNum.
    let Some(seq) = cdata.and_then(|c| pairs(c.content, 4).get(&0x6801).copied()) else {
        return;
    };
    let fo = fopt.map(parse_fopt);
    let rotation = fo
        .as_ref()
        .and_then(|f| f.scalar(0x0004))
        .map(|v| {
            let deg = (v as i32) as f64 / 65_536.0;
            ((deg % 360.0) + 360.0) % 360.0
        })
        .unwrap_or(0.0) as f32;
    let inset = |f: &Fopt, pid: u16| -> f32 {
        emu_pt(
            f.scalar(pid)
                .map(|v| v as i32 as i64)
                .unwrap_or(DEFAULT_INSET_EMU),
        )
    };
    let insets_pt = match &fo {
        Some(f) => (
            inset(f, 0x0081),
            inset(f, 0x0082),
            inset(f, 0x0083),
            inset(f, 0x0084),
        ),
        None => (
            emu_pt(DEFAULT_INSET_EMU),
            emu_pt(DEFAULT_INSET_EMU),
            emu_pt(DEFAULT_INSET_EMU),
            emu_pt(DEFAULT_INSET_EMU),
        ),
    };
    let columns = tertiary
        .map(parse_fopt)
        .and_then(|t| t.scalar(0x008C))
        .or_else(|| fo.as_ref().and_then(|f| f.scalar(0x008C)))
        .unwrap_or(1);
    let picture_id = fo.as_ref().and_then(|f| f.scalar(0x4104));
    let wordart = fo
        .as_ref()
        .and_then(|f| f.complex(0x00C0))
        .and_then(wordart_text);
    // The anchor: a client anchor names the box outright
    // (page coordinates, EMU); a child anchor names it in
    // the parent group's child coordinate system, rescaled
    // through the group's absolute rectangle.
    let mut absolute: Option<EmuRect> = None;
    if let Some(a) = anchor {
        if a.ty == 0xF010 {
            let p = pairs(a.content, 4);
            let get = |id: u16| p.get(&id).copied().unwrap_or(0) as i32 as i64;
            absolute = Some((get(0x2001), get(0x2002), get(0x2003), get(0x2004)));
        } else if a.content.len() >= 16 {
            let v = |i: usize| u32_at(a.content, i * 4).unwrap_or(0) as i32 as i64;
            let cs = snapshot_coord;
            let ga = *abs;
            let csw = if cs.2 - cs.0 == 0 { 1 } else { cs.2 - cs.0 };
            let csh = if cs.3 - cs.1 == 0 { 1 } else { cs.3 - cs.1 };
            let gw = (ga.2 - ga.0) as f64;
            let gh = (ga.3 - ga.1) as f64;
            let sx = |x: i64| ga.0 + (((x - cs.0) as f64 * gw / csw as f64).round() as i64);
            let sy = |y: i64| ga.1 + (((y - cs.1) as f64 * gh / csh as f64).round() as i64);
            absolute = Some((sx(v(0)), sy(v(1)), sx(v(2)), sy(v(3))));
        }
    }
    let absolute = absolute.map(|r| {
        // A quarter-turned shape's stored box is swapped
        // around its center (the record's own convention).
        let r = if (45.0..135.0).contains(&rotation) || (225.0..315.0).contains(&rotation) {
            let w = r.2 - r.0;
            let h = r.3 - r.1;
            let cx = r.0 + w / 2;
            let cy = r.1 + h / 2;
            (cx - h / 2, cy - w / 2, cx + h / 2, cy + w / 2)
        } else {
            r
        };
        arrange(r)
    });
    if let (Some(r), true) = (absolute, defines_relative) {
        *abs = r;
    }
    doc.by_seq.insert(
        seq,
        EscherShape {
            rect: absolute,
            deleted: flags & 0x08 != 0,
            rotation,
            flip_h: flags & 0x40 != 0,
            flip_v: flags & 0x80 != 0,
            insets_pt,
            columns,
            picture_id,
            wordart,
        },
    );
}

// ----------------------------------------------------------------
// Composition: parsed stories into their shapes' boxes.

struct Compose<'a> {
    parse: &'a QuillParse,
    layout: &'a ContentsLayout,
    escher: &'a EscherDoc,
    by_syid: HashMap<u32, usize>,
    placed_stories: HashSet<usize>,
    /// Paragraph tails that ran past the bottom of their
    /// page, in the order their stories were placed; they
    /// land on the reading-order pages at the end.
    remainders: Vec<Vec<crate::pub_text::ParaParse>>,
    remainder_stories: HashSet<usize>,
    spans: Vec<Span>,
    loss: LossReport,
}

/// The pen for placing one story's words inside a box:
/// the reading column's discipline, bounded by the box.
struct RectPen {
    x: f32,
    y: f32,
    line_max: f32,
    line_used: bool,
    pending_space: bool,
    prev_ended_space: bool,
}

impl<'a> Compose<'a> {
    /// Place the given paragraphs of one story in a box
    /// (already inset). `review` forces every span to
    /// NeedsReview; otherwise a run is trusted (RawText)
    /// exactly when its font resolved in the FONT table.
    /// Place paragraphs into a box. Words may run past
    /// the box's own bottom (a rescue keeps the words near
    /// their home), but never past the bottom of the page:
    /// the first paragraph that would cross it, and every
    /// paragraph after, are reported back by index so the
    /// caller can move them to the reading-order pages.
    fn place_paras(
        &mut self,
        paras: &[crate::pub_text::ParaParse],
        rect: (f32, f32, f32, f32),
        page: u32,
        review: bool,
    ) -> Option<usize> {
        let (rx, ry, rw, _) = rect;
        let right = rx + rw;
        let page_bottom = self.layout.page_h_pt - 1.0;
        let mut pen = RectPen {
            x: rx,
            y: ry,
            line_max: 11.0,
            line_used: false,
            pending_space: false,
            prev_ended_space: false,
        };
        for (i, para) in paras.iter().enumerate() {
            let span_mark = self.spans.len();
            let emitted_mark = self.loss.characters_emitted;
            for run in &para.runs {
                let trust = if review || !run.font_resolved {
                    SpanTrust::NeedsReview
                } else {
                    SpanTrust::RawText
                };
                self.place_piece(
                    &mut pen,
                    rx,
                    right,
                    &self.parse.chars[run.start..run.end],
                    &run.props,
                    trust,
                    page,
                );
            }
            // Paragraph step: its line height plus a gap,
            // as in the reading column.
            pen.y += pen.line_max.max(1.0) * 1.35 + para.max_size * 0.9;
            pen.x = rx;
            pen.line_max = 11.0;
            pen.line_used = false;
            pen.pending_space = false;
            pen.prev_ended_space = false;
            let crossed = self.spans[span_mark..]
                .iter()
                .any(|sp| sp.y.unwrap_or(0.0) + sp.h.unwrap_or(0.0) > page_bottom);
            if crossed {
                self.spans.truncate(span_mark);
                self.loss.characters_emitted = emitted_mark;
                return Some(i);
            }
        }
        None
    }

    /// One character run into the box, splitting at control
    /// characters exactly as the text rescue does: a
    /// vertical tab or newline breaks the line, a form
    /// feed breaks the line too (a box has no next page),
    /// a tab becomes a space, the paragraph mark and other
    /// controls never reach a span.
    #[allow(clippy::too_many_arguments)]
    fn place_piece(
        &mut self,
        pen: &mut RectPen,
        rx: f32,
        right: f32,
        text: &[char],
        props: &crate::pub_text::CharProps,
        trust: SpanTrust,
        page: u32,
    ) {
        let mut segment = String::new();
        for &c in text {
            match c {
                '\r' => { /* the paragraph mark: structural */ }
                '\n' | '\u{b}' | '\u{c}' => {
                    self.place_flush(pen, rx, right, &segment, props, trust, page);
                    segment.clear();
                    self.rect_line_break(pen, rx);
                    self.loss.control_chars_mapped += 1;
                }
                '\t' => {
                    segment.push(' ');
                    self.loss.control_chars_mapped += 1;
                }
                c if c.is_control() => {
                    self.loss.control_chars_mapped += 1;
                }
                c => segment.push(c),
            }
        }
        self.place_flush(pen, rx, right, &segment, props, trust, page);
    }

    fn rect_line_break(&self, pen: &mut RectPen, rx: f32) {
        pen.y += pen.line_max.max(1.0) * 1.35;
        pen.x = rx;
        pen.line_max = 11.0;
        pen.line_used = false;
        pen.pending_space = false;
        pen.prev_ended_space = false;
    }

    #[allow(clippy::too_many_arguments)]
    fn place_flush(
        &mut self,
        pen: &mut RectPen,
        rx: f32,
        right: f32,
        segment: &str,
        props: &crate::pub_text::CharProps,
        trust: SpanTrust,
        page: u32,
    ) {
        if segment.is_empty() {
            return;
        }
        if segment.trim().is_empty() {
            pen.pending_space = true;
            return;
        }
        let size = props.size_pt.unwrap_or(11.0);
        let est = |nchars: f32| (nchars * size * 0.52).max(size * 0.5);
        let seg_w = est(segment.chars().count() as f32);
        // A segment that fits the line it lands on keeps
        // the run's own span (the draft builder joins runs
        // by their positions, and a space at a boundary is
        // earned by a real gap, as in the reading column).
        let gap0 = if pen.line_used
            && (pen.pending_space || pen.prev_ended_space || segment.starts_with(' '))
        {
            size * 0.3
        } else {
            0.0
        };
        if pen.x + gap0 + seg_w <= right {
            let x = pen.x + gap0;
            self.push_span(page, segment, trust, x, pen.y, seg_w, size, props);
            pen.x = x + seg_w;
            pen.line_used = true;
            pen.line_max = pen.line_max.max(size);
            pen.pending_space = false;
            pen.prev_ended_space = segment.ends_with(' ');
            return;
        }
        // A segment wider than the rest of its line flows
        // word by word down the box, so no span claims a
        // width the page cannot hold. (A single word wider
        // than the box is clamped to the box; its text is
        // whole, only its claimed width is the box's.)
        let box_w = right - rx;
        let mut first_word = true;
        for word in segment.split(' ') {
            if word.is_empty() {
                pen.pending_space = true;
                continue;
            }
            let word_w = est(word.chars().count() as f32);
            let gap = if pen.line_used && (pen.pending_space || !first_word) {
                size * 0.3
            } else {
                0.0
            };
            if pen.line_used && pen.x + gap + word_w > right {
                self.rect_line_break(pen, rx);
            }
            let gap = if pen.line_used && (pen.pending_space || !first_word) {
                size * 0.3
            } else {
                0.0
            };
            let x = pen.x + gap;
            if gap > 0.0 {
                self.loss.characters_emitted += 1; // the earned space
            }
            self.push_span(page, word, trust, x, pen.y, word_w.min(box_w), size, props);
            pen.x = x + word_w;
            pen.line_used = true;
            pen.line_max = pen.line_max.max(size);
            pen.pending_space = false;
            first_word = false;
        }
        pen.prev_ended_space = segment.ends_with(' ');
    }

    #[allow(clippy::too_many_arguments)]
    fn push_span(
        &mut self,
        page: u32,
        text: &str,
        trust: SpanTrust,
        x: f32,
        y: f32,
        w: f32,
        size: f32,
        props: &crate::pub_text::CharProps,
    ) {
        self.spans.push(Span {
            page,
            text: text.to_string(),
            trust,
            x: Some(x),
            y: Some(y),
            w: Some(w),
            h: Some(size * 1.2),
            font_size: props.size_pt,
            bold: Some(props.bold.unwrap_or(false)),
            font: span_font(props, &self.parse.fonts),
        });
        self.loss.characters_emitted += text.chars().count();
    }

    /// A shape's box in page points: the Escher anchor is
    /// relative to the page's center, so half the page
    /// size is added back.
    fn rect_pt(&self, e: &EscherShape) -> Option<(f32, f32, f32, f32)> {
        e.rect.map(|r| {
            (
                emu_pt(r.0) + self.layout.page_w_pt / 2.0,
                emu_pt(r.1) + self.layout.page_h_pt / 2.0,
                emu_pt(r.2 - r.0),
                emu_pt(r.3 - r.1),
            )
        })
    }

    /// Why a placed shape's words still need review beyond
    /// their fonts: rotation, flips, multiple text
    /// columns, or a box that runs off the page.
    fn shape_needs_review(&self, e: &EscherShape, rect: (f32, f32, f32, f32)) -> bool {
        let (x, y, w, h) = rect;
        e.rotation > 0.01
            || e.flip_h
            || e.flip_v
            || e.columns > 1
            || x < -1.0
            || y < -1.0
            || x + w > self.layout.page_w_pt + 1.0
            || y + h > self.layout.page_h_pt + 1.0
    }

    /// Emit one listed shape onto a page. `master` marks
    /// shapes arriving through an applied master page:
    /// always Check this, counted separately.
    fn emit_shape(&mut self, page: u32, seq: u32, master: bool, paired_alts: &HashSet<u32>) {
        let esch = self.escher.by_seq.get(&seq);
        if let Some(e) = esch {
            if e.deleted {
                if !master {
                    self.loss.shapes_deleted_skipped += 1;
                }
                return;
            }
        }
        if paired_alts.contains(&seq) {
            if !master {
                self.loss.alternate_shapes_excluded += 1;
            }
            return;
        }
        let Some(chunk) = self.layout.shapes.get(&seq) else {
            if !master {
                self.loss.shapes_total += 1;
                self.loss.shapes_unplaced += 1;
            }
            return;
        };
        if chunk.ty == 0x10 && chunk.table.is_some() {
            let emitted = self.emit_table(page, chunk, esch);
            if master {
                if emitted {
                    self.loss.master_shapes_emitted += 1;
                }
            } else {
                self.loss.shapes_total += 1;
                if emitted {
                    self.loss.shapes_placed += 1;
                } else {
                    self.loss.shapes_unplaced += 1;
                }
            }
            return;
        }
        let rect = esch.and_then(|e| self.rect_pt(e));
        let mut emitted = false;
        // WordArt: the text lives in the drawing layer.
        if let (Some(e), Some(r)) = (esch, rect) {
            if let Some(wa) = &e.wordart {
                self.spans.push(Span {
                    page,
                    text: wa.clone(),
                    trust: SpanTrust::NeedsReview,
                    x: Some(r.0),
                    y: Some(r.1),
                    w: Some(r.2),
                    h: Some(r.3),
                    font_size: None,
                    bold: None,
                    font: None,
                });
                self.loss.wordart_emitted += 1;
                emitted = true;
            }
        }
        // Quill text joined through the shape's text id.
        if let Some(tid) = chunk.text_id {
            if let Some(&si) = self.by_syid.get(&tid) {
                if !self.parse.stories[si].emitted {
                    emitted = true; // a blank story: nothing lost
                } else if self.placed_stories.contains(&si) || self.remainder_stories.contains(&si)
                {
                    emitted = true; // already placed by its first shape
                } else if let (Some(e), Some(r)) = (esch, rect) {
                    let review = master || self.shape_needs_review(e, r);
                    let (il, it, ir, ib) = e.insets_pt;
                    let text_rect = (
                        r.0 + il,
                        r.1 + it,
                        (r.2 - il - ir).max(1.0),
                        (r.3 - it - ib).max(1.0),
                    );
                    let parse: &QuillParse = self.parse;
                    match self.place_paras(&parse.stories[si].paragraphs, text_rect, page, review) {
                        None => {
                            self.placed_stories.insert(si);
                        }
                        Some(first_unplaced) => {
                            // The words that fit stayed in
                            // the box; the rest of the story
                            // moves to the reading-order
                            // pages, counted, never dropped.
                            self.remainders
                                .push(parse.stories[si].paragraphs[first_unplaced..].to_vec());
                            self.remainder_stories.insert(si);
                            self.loss.stories_overflowed += 1;
                        }
                    }
                    emitted = true;
                }
            }
        }
        // A picture: its bytes cannot ride the dump, so its
        // place is marked and the loss is counted, never
        // dressed up as a carried picture.
        if let (Some(e), Some(r)) = (esch, rect) {
            if e.picture_id.is_some() {
                self.spans.push(Span {
                    page,
                    text: String::new(),
                    trust: SpanTrust::PictureOnly,
                    x: Some(r.0),
                    y: Some(r.1),
                    w: Some(r.2),
                    h: Some(r.3),
                    font_size: None,
                    bold: None,
                    font: None,
                });
                emitted = true;
            }
        }
        if master {
            if emitted {
                self.loss.master_shapes_emitted += 1;
            }
            return;
        }
        // Pure group containers carry no content of their
        // own; they are structure, not shapes to account.
        let is_group = matches!(chunk.ty, 0x30 | 0x31);
        let has_content = chunk.text_id.is_some()
            || esch
                .map(|e| e.picture_id.is_some() || e.wordart.is_some())
                .unwrap_or(false);
        if is_group && !has_content {
            return;
        }
        self.loss.shapes_total += 1;
        if emitted {
            self.loss.shapes_placed += 1;
        } else {
            self.loss.shapes_unplaced += 1;
        }
    }

    /// A table: its story's paragraphs split into cells by
    /// the Quill TCD ends, each cell placed by the table's
    /// row/column size arrays when the file carries them,
    /// else stacked in the table's box. Always Check this.
    /// Returns whether anything was emitted.
    fn emit_table(&mut self, page: u32, chunk: &ShapeChunk, esch: Option<&EscherShape>) -> bool {
        let Some(table) = &chunk.table else {
            return false;
        };
        let Some(&si) = chunk.text_id.and_then(|tid| self.by_syid.get(&tid)) else {
            return false;
        };
        if !self.parse.stories[si].emitted {
            return true; // a blank table: nothing lost
        }
        let Some(rect) = esch.and_then(|e| self.rect_pt(e)) else {
            return false; // no position: the story overflows whole
        };
        if self.placed_stories.contains(&si) {
            return true;
        }
        // Cells in reading order; the file's cell records
        // when present, else the plain grid.
        let mut cells: Vec<CellRect> = table.cells.clone();
        if cells.is_empty() && table.rows > 0 && table.cols > 0 {
            for r in 0..table.rows {
                for c in 0..table.cols {
                    cells.push(CellRect {
                        start_row: r,
                        end_row: r,
                        start_col: c,
                        end_col: c,
                    });
                }
            }
        }
        if cells.is_empty() {
            cells.push(CellRect::default());
        }
        cells.sort_by_key(|c| (c.start_row, c.start_col));
        // Assign the story's paragraphs to cells: by the
        // TCD ends when the file gives them, else one
        // paragraph per cell in order.
        let story = &self.parse.stories[si];
        let ends = self.parse.table_cell_ends.get(si).and_then(|e| e.as_ref());
        let ncells = cells.len();
        let mut cell_paras: Vec<Vec<usize>> = vec![Vec::new(); ncells];
        match ends {
            Some(ends) => {
                let mut ci = 0usize;
                for (pi, para) in story.paragraphs.iter().enumerate() {
                    cell_paras[ci].push(pi);
                    let rel_end = para.char_end - story.char_start;
                    if ci + 1 < ncells && ends.get(ci).map(|e| rel_end >= *e).unwrap_or(false) {
                        ci += 1;
                    }
                }
            }
            None => {
                for (pi, _) in story.paragraphs.iter().enumerate() {
                    cell_paras[pi.min(ncells - 1)].push(pi);
                }
            }
        }
        // Cell boxes.
        let sized = table.cols > 0
            && table.col_widths.len() == table.cols
            && table.rows > 0
            && table.row_heights.len() == table.rows;
        let band_h = rect.3 / ncells as f32;
        let mut emitted_any = false;
        for (ci, cell) in cells.iter().enumerate() {
            if cell_paras[ci].is_empty() {
                continue;
            }
            let cell_rect = if sized {
                let x = rect.0
                    + table.col_widths[..cell.start_col.min(table.cols)]
                        .iter()
                        .map(|v| emu_pt(*v))
                        .sum::<f32>();
                let w: f32 = table.col_widths
                    [cell.start_col.min(table.cols)..=(cell.end_col.min(table.cols - 1))]
                    .iter()
                    .map(|v| emu_pt(*v))
                    .sum();
                let y = rect.1
                    + table.row_heights[..cell.start_row.min(table.rows)]
                        .iter()
                        .map(|v| emu_pt(*v))
                        .sum::<f32>();
                let h: f32 = table.row_heights
                    [cell.start_row.min(table.rows)..=(cell.end_row.min(table.rows - 1))]
                    .iter()
                    .map(|v| emu_pt(*v))
                    .sum();
                (x, y, w.max(1.0), h.max(1.0))
            } else {
                (rect.0, rect.1 + band_h * ci as f32, rect.2, band_h.max(1.0))
            };
            let paras: Vec<crate::pub_text::ParaParse> = cell_paras[ci]
                .iter()
                .map(|&pi| self.parse.stories[si].paragraphs[pi].clone())
                .collect();
            let before = self.spans.len();
            self.place_paras(&paras, cell_rect, page, true);
            if self.spans.len() > before {
                self.loss.table_cells_emitted += 1;
                emitted_any = true;
            }
        }
        if emitted_any {
            self.placed_stories.insert(si);
        }
        emitted_any
    }
}

fn compose(
    parse: &QuillParse,
    layout: &ContentsLayout,
    escher: &EscherDoc,
    loss: &mut LossReport,
    source_name: &str,
    source_bytes: usize,
) -> ExtractionDump {
    let by_syid: HashMap<u32, usize> = parse
        .stories
        .iter()
        .filter_map(|s| s.syid.map(|id| (id, s.index)))
        .collect();
    let mut cx = Compose {
        parse,
        layout,
        escher,
        by_syid,
        placed_stories: HashSet::new(),
        remainders: Vec::new(),
        remainder_stories: HashSet::new(),
        spans: Vec::new(),
        loss: std::mem::take(loss),
    };
    let mut pages: Vec<PageInfo> = Vec::new();
    for &page_seq in &layout.page_order {
        if DUMMY_PAGES.contains(&page_seq) {
            continue;
        }
        let Some(page_chunk) = layout.pages.get(&page_seq) else {
            cx.loss.notes.push(format!(
                "the page list names page chunk {page_seq}, but the file holds \
                 no chunk for it; the page was kept blank"
            ));
            pages.push(PageInfo {
                index: pages.len() as u32,
                width_pt: layout.page_w_pt,
                height_pt: layout.page_h_pt,
                picture_only: false,
            });
            continue;
        };
        if page_chunk.is_master {
            continue; // master pages serve other pages; they are not pages
        }
        let page_index = pages.len() as u32;
        pages.push(PageInfo {
            index: page_index,
            width_pt: layout.page_w_pt,
            height_pt: layout.page_h_pt,
            picture_only: false,
        });
        // Alternate-shape pairing on this page: a listed
        // alternate (0x20) whose parent also lists a
        // regular shape (0x01) is the same object twice;
        // the regular one wins.
        let listed: HashSet<u32> = page_chunk.shapes.iter().copied().collect();
        let regular_parents: HashSet<u32> = listed
            .iter()
            .filter_map(|s| layout.shapes.get(s))
            .filter(|c| c.ty == 0x01)
            .map(|c| c.parent)
            .collect();
        let paired_alts: HashSet<u32> = listed
            .iter()
            .copied()
            .filter(|s| {
                layout
                    .shapes
                    .get(s)
                    .map(|c| c.ty == 0x20 && regular_parents.contains(&c.parent))
                    .unwrap_or(false)
            })
            .collect();
        // The applied master's shapes sit underneath.
        if let Some(mseq) = page_chunk.applied_master {
            if let Some(master) = layout.pages.get(&mseq) {
                let master_shapes = master.shapes.clone();
                for seq in master_shapes {
                    cx.emit_shape(page_index, seq, true, &paired_alts);
                }
            }
        }
        let own_shapes = page_chunk.shapes.clone();
        for seq in own_shapes {
            cx.emit_shape(page_index, seq, false, &paired_alts);
        }
    }
    // Stories no shape claimed (or whose shape had no
    // position) keep their words on reading-order pages at
    // the end, counted, never dropped.
    let unplaced: Vec<usize> = parse
        .stories
        .iter()
        .filter(|s| {
            s.emitted
                && !cx.placed_stories.contains(&s.index)
                && !cx.remainder_stories.contains(&s.index)
        })
        .map(|s| s.index)
        .collect();
    if !unplaced.is_empty() {
        cx.loss.stories_unplaced = unplaced.len();
        let first = pages.len() as u32;
        let (spans, npages) = reading_column_spans(parse, &unplaced, first, &mut cx.loss);
        for i in 0..npages {
            pages.push(PageInfo {
                index: first + i,
                width_pt: layout.page_w_pt,
                height_pt: layout.page_h_pt,
                picture_only: false,
            });
        }
        cx.spans.extend(spans);
    }
    // Paragraph tails that ran past the bottom of their
    // page follow on the same reading-order pages.
    if !cx.remainders.is_empty() {
        let paras: Vec<crate::pub_text::ParaParse> = cx
            .remainders
            .iter()
            .flat_map(|r| r.iter().cloned())
            .collect();
        let first = pages.len() as u32;
        let (spans, npages) = reading_column_paras(parse, &paras, first, &mut cx.loss);
        for i in 0..npages {
            pages.push(PageInfo {
                index: first + i,
                width_pt: layout.page_w_pt,
                height_pt: layout.page_h_pt,
                picture_only: false,
            });
        }
        cx.spans.extend(spans);
        cx.loss.notes.push(format!(
            "{} {} had more words than fit on the page; the words that fit              stayed in their box and the rest are on the last pages in              reading order",
            cx.loss.stories_overflowed,
            if cx.loss.stories_overflowed == 1 {
                "story"
            } else {
                "stories"
            }
        ));
    }
    *loss = std::mem::take(&mut cx.loss);
    let mut spans = std::mem::take(&mut cx.spans);
    // Span widths are estimates (character count times a
    // per-point factor), and a long unbroken run can claim
    // more width than its page holds. Clamp the claim to
    // the page: the text is whole, only the estimate is
    // cut down to what the page can hold.
    for sp in &mut spans {
        if let (Some(x), Some(w)) = (sp.x, sp.w) {
            sp.w = Some(w.min((layout.page_w_pt - x).max(1.0)));
        }
        if let (Some(y), Some(h)) = (sp.y, sp.h) {
            sp.h = Some(h.min((layout.page_h_pt - y).max(1.0)));
        }
    }
    ExtractionDump {
        schema_version: DUMP_SCHEMA_VERSION,
        source_kind: SourceKind::Pub,
        source_name: source_name.to_string(),
        source_bytes,
        extractor: "openbulletin-pub 0.2".to_string(),
        pages,
        spans,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cfb::Cfb;

    // ---- Synthetic fixtures --------------------------------
    //
    // A whole Publisher file in miniature: a CFB container
    // (v3, 512-byte sectors, every stream a regular stream
    // of exactly 4096 bytes) holding a Contents stream with
    // one document, one page, and one shape chunk; a Quill
    // stream with one story whose SYID the shape names; and
    // an Escher stream with one shape instance anchored at
    // a known box. Page: US Letter, 612 x 792 pt. The box:
    // (72, 72) pt, 200 x 100 pt, stored center-relative.

    const PAGE_W_EMU: i64 = 612 * 12_700;
    const PAGE_H_EMU: i64 = 792 * 12_700;
    const BOX_XS: i64 = 72 * 12_700 - PAGE_W_EMU / 2;
    const BOX_YS: i64 = 72 * 12_700 - PAGE_H_EMU / 2;
    const BOX_XE: i64 = BOX_XS + 200 * 12_700;
    const BOX_YE: i64 = BOX_YS + 100 * 12_700;

    fn blk_int(id: u8, ty: u8, val: u32) -> Vec<u8> {
        let mut v = vec![id, ty];
        v.extend_from_slice(&val.to_le_bytes());
        v
    }

    fn blk_box(id: u8, ty: u8, subs: &[u8]) -> Vec<u8> {
        let mut v = vec![id, ty];
        v.extend_from_slice(&((subs.len() + 4) as u32).to_le_bytes());
        v.extend_from_slice(subs);
        v
    }

    fn chunk(blocks: &[u8]) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(&((blocks.len() + 4) as u32).to_le_bytes());
        v.extend_from_slice(blocks);
        v
    }

    fn chunk_ref(ty: u32, off: usize, parent: u32) -> Vec<u8> {
        let mut subs = Vec::new();
        subs.extend_from_slice(&blk_int(0x02, 0x20, ty));
        subs.extend_from_slice(&blk_int(0x04, 0x20, off as u32));
        subs.extend_from_slice(&blk_int(0x05, 0x20, parent));
        blk_box(0x00, 0x88, &subs)
    }

    /// The Contents stream. `poison` puts a block of
    /// unknown type (0x99) inside the document chunk's
    /// size container, ahead of the size blocks: per the
    /// scoped-strictness rule the container is abandoned,
    /// the page size is never read, and the layout layer
    /// must give up (the caller keeps the words).
    fn contents_stream(poison: bool) -> Vec<u8> {
        let mut out = vec![0u8; 0x30];
        out[0] = 0xE8;
        out[1] = 0xAC;
        out[2] = 0x2C;
        // Document chunk at 0x30 (directory ordinal 0).
        let mut size_subs = Vec::new();
        if poison {
            size_subs.extend_from_slice(&[0x55, 0x99]);
        }
        size_subs.extend_from_slice(&blk_int(0x01, 0x20, PAGE_W_EMU as u32));
        size_subs.extend_from_slice(&blk_int(0x02, 0x20, PAGE_H_EMU as u32));
        let mut doc_blocks = Vec::new();
        doc_blocks.extend_from_slice(&blk_box(0x12, 0x88, &size_subs));
        doc_blocks.extend_from_slice(&blk_box(0x02, 0x88, &blk_int(0x00, 0x20, 1)));
        let doc = chunk(&doc_blocks);
        out.extend_from_slice(&doc);
        // Page chunk (ordinal 1): lists shape 2.
        let page_off = out.len();
        let page = chunk(&blk_box(0x02, 0x88, &blk_int(0x00, 0x70, 2)));
        out.extend_from_slice(&page);
        // Shape chunk (ordinal 2): text id 7.
        let shape_off = out.len();
        out.extend_from_slice(&chunk(&blk_int(0x27, 0x20, 7)));
        // Trailer: total length, then the directory block.
        let trailer_off = out.len();
        out[0x1A..0x1E].copy_from_slice(&(trailer_off as u32).to_le_bytes());
        let mut dir = Vec::new();
        dir.extend_from_slice(&chunk_ref(0x44, 0x30, 0));
        dir.extend_from_slice(&chunk_ref(0x43, page_off, 0));
        dir.extend_from_slice(&chunk_ref(0x01, shape_off, 0));
        let dir_block = blk_box(0x00, 0x90, &dir);
        out.extend_from_slice(&((dir_block.len() + 4) as u32).to_le_bytes());
        out.extend_from_slice(&dir_block);
        // Pad to a full regular stream with dummy blocks.
        while out.len() + 2 <= 4096 {
            out.extend_from_slice(&[0x00, 0x78]);
        }
        out.resize(4096, 0);
        out
    }

    /// The Quill stream: one story, "Hello placed world",
    /// SYID 7. Same directory shape the text rescue's own
    /// fixtures use.
    fn quill_stream() -> Vec<u8> {
        let text = "Hello placed world\r";
        let text_bytes: Vec<u8> = text.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
        let mut strs = Vec::new();
        strs.extend_from_slice(&1u32.to_le_bytes());
        strs.extend_from_slice(&8u32.to_le_bytes());
        strs.extend_from_slice(&255u32.to_le_bytes());
        strs.extend_from_slice(&19u32.to_le_bytes());
        let mut syid = Vec::new();
        syid.extend_from_slice(&3u32.to_le_bytes());
        syid.extend_from_slice(&1u32.to_le_bytes());
        syid.extend_from_slice(&7u32.to_le_bytes());
        let chunks: Vec<(&str, Vec<u8>)> =
            vec![("TEXT", text_bytes), ("STRS", strs), ("SYID", syid)];
        let text_off = 0x18 + 8 + 24 * chunks.len();
        let mut out = vec![0u8; 0x18];
        out.extend_from_slice(&[0, 0]);
        out.extend_from_slice(&(chunks.len() as u16).to_le_bytes());
        out.extend_from_slice(&0xFFFF_FFFFu32.to_le_bytes());
        let mut at = text_off;
        let mut offsets = Vec::new();
        for (_, b) in &chunks {
            offsets.push(at);
            at += b.len();
        }
        for (i, (name, b)) in chunks.iter().enumerate() {
            out.extend_from_slice(&24u16.to_le_bytes());
            let mut nm = [b' '; 4];
            nm[..name.len()].copy_from_slice(name.as_bytes());
            out.extend_from_slice(&nm);
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&0x0100_0000u32.to_le_bytes());
            out.extend_from_slice(&nm);
            out.extend_from_slice(&(offsets[i] as u32).to_le_bytes());
            out.extend_from_slice(&(b.len() as u32).to_le_bytes());
        }
        for (_, b) in &chunks {
            out.extend_from_slice(b);
        }
        out.resize(4096, 0);
        out
    }

    fn escher_record(ty: u16, ver_inst: u16, content: &[u8]) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(&ver_inst.to_le_bytes());
        v.extend_from_slice(&ty.to_le_bytes());
        v.extend_from_slice(&(content.len() as u32).to_le_bytes());
        v.extend_from_slice(content);
        v
    }

    /// The Escher stream: one drawing holding one shape
    /// instance; its client anchor is the known box and its
    /// client data names directory ordinal 2 (the shape
    /// chunk). Drawing containers carry 4 tail bytes their
    /// length does not count.
    fn escher_stream() -> Vec<u8> {
        let mut fsp = Vec::new();
        fsp.extend_from_slice(&0x1001u32.to_le_bytes()); // shape id
        fsp.extend_from_slice(&0u32.to_le_bytes()); // flags: not deleted
        let mut anchor = Vec::new();
        anchor.extend_from_slice(&28u32.to_le_bytes());
        for (id, val) in [
            (0x2001u16, BOX_XS),
            (0x2002, BOX_YS),
            (0x2003, BOX_XE),
            (0x2004, BOX_YE),
        ] {
            anchor.extend_from_slice(&id.to_le_bytes());
            anchor.extend_from_slice(&(val as u32).to_le_bytes());
        }
        let mut cdata = Vec::new();
        cdata.extend_from_slice(&10u32.to_le_bytes());
        cdata.extend_from_slice(&0x6801u16.to_le_bytes());
        cdata.extend_from_slice(&2u32.to_le_bytes());
        let mut sp = Vec::new();
        sp.extend_from_slice(&escher_record(0xF00A, 0x0010, &fsp));
        sp.extend_from_slice(&escher_record(0xF010, 0x0000, &anchor));
        sp.extend_from_slice(&escher_record(0xF011, 0x0000, &cdata));
        let sp_record = escher_record(0xF004, 0x000F, &sp);
        let mut dg = escher_record(0xF002, 0x000F, &sp_record);
        dg.extend_from_slice(&[0, 0, 0, 0]); // the container tail
        dg.resize(4096, 0);
        dg
    }

    /// A minimal CFB v3 container: FAT at sector 0,
    /// directory at sectors 1-2, the three streams as
    /// regular 8-sector chains at 3, 11, and 19.
    fn build_cfb(contents: Vec<u8>, quill: Vec<u8>, escher: Vec<u8>) -> Vec<u8> {
        const END: u32 = 0xFFFF_FFFE;
        const FREE: u32 = 0xFFFF_FFFF;
        const NO_STREAM: u32 = 0xFFFF_FFFF;
        let sector = 512usize;
        let mut file = vec![0u8; sector * 28];
        // Header.
        file[0..8].copy_from_slice(&[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]);
        file[24..26].copy_from_slice(&62u16.to_le_bytes());
        file[26..28].copy_from_slice(&3u16.to_le_bytes());
        file[28..30].copy_from_slice(&0xFFFEu16.to_le_bytes());
        file[30..32].copy_from_slice(&9u16.to_le_bytes());
        file[32..34].copy_from_slice(&6u16.to_le_bytes());
        file[44..48].copy_from_slice(&1u32.to_le_bytes()); // one FAT sector
        file[48..52].copy_from_slice(&1u32.to_le_bytes()); // directory at 1
        file[56..60].copy_from_slice(&4096u32.to_le_bytes());
        file[60..64].copy_from_slice(&END.to_le_bytes()); // no miniFAT
        file[64..68].copy_from_slice(&0u32.to_le_bytes());
        file[68..72].copy_from_slice(&END.to_le_bytes()); // no DIFAT chain
        file[76..80].copy_from_slice(&0u32.to_le_bytes()); // DIFAT[0] = sector 0
        for i in 1..109 {
            file[76 + i * 4..80 + i * 4].copy_from_slice(&FREE.to_le_bytes());
        }
        // FAT: 0 FATSECT; 1 -> 2 -> END (directory);
        // 3..=10, 11..=18, 19..=26 the stream chains.
        let mut fat = [FREE; 128];
        fat[0] = 0xFFFF_FFFD;
        fat[1] = 2;
        fat[2] = END;
        for base in [3usize, 11, 19] {
            for i in 0..8 {
                fat[base + i] = if i == 7 { END } else { (base + i + 1) as u32 };
            }
        }
        for (i, v) in fat.iter().enumerate() {
            file[sector + i * 4..sector + i * 4 + 4].copy_from_slice(&v.to_le_bytes());
        }
        // Directory entries: 0 root, 1 Contents, 2 Quill,
        // 3 QuillSub, 4 Escher, 5 CONTENTS, 6 EscherStm.
        let entry = |file: &mut Vec<u8>,
                     idx: usize,
                     name: &str,
                     ty: u8,
                     right: u32,
                     child: u32,
                     start: u32,
                     size: u64| {
            let sec = 1 + idx / 4;
            let base = (sec + 1) * sector + (idx % 4) * 128;
            let units: Vec<u16> = name.encode_utf16().collect();
            for (i, u) in units.iter().enumerate() {
                file[base + i * 2..base + i * 2 + 2].copy_from_slice(&u.to_le_bytes());
            }
            file[base + 64..base + 66]
                .copy_from_slice(&(((units.len() + 1) * 2) as u16).to_le_bytes());
            file[base + 66] = ty;
            file[base + 68..base + 72].copy_from_slice(&NO_STREAM.to_le_bytes());
            file[base + 72..base + 76].copy_from_slice(&right.to_le_bytes());
            file[base + 76..base + 80].copy_from_slice(&child.to_le_bytes());
            file[base + 116..base + 120].copy_from_slice(&start.to_le_bytes());
            file[base + 120..base + 128].copy_from_slice(&size.to_le_bytes());
        };
        entry(&mut file, 0, "Root Entry", 5, NO_STREAM, 1, END, 0);
        entry(&mut file, 1, "Contents", 2, 2, NO_STREAM, 3, 4096);
        entry(&mut file, 2, "Quill", 1, 4, 3, 0, 0);
        entry(&mut file, 3, "QuillSub", 1, NO_STREAM, 5, 0, 0);
        entry(&mut file, 4, "Escher", 1, NO_STREAM, 6, 0, 0);
        entry(&mut file, 5, "CONTENTS", 2, NO_STREAM, NO_STREAM, 11, 4096);
        entry(&mut file, 6, "EscherStm", 2, NO_STREAM, NO_STREAM, 19, 4096);
        // Stream bodies.
        file[4 * sector..4 * sector + 4096].copy_from_slice(&contents);
        file[12 * sector..12 * sector + 4096].copy_from_slice(&quill);
        file[20 * sector..20 * sector + 4096].copy_from_slice(&escher);
        file
    }

    #[test]
    fn story_lands_inside_its_anchor_box() {
        let bytes = build_cfb(contents_stream(false), quill_stream(), escher_stream());
        let cfb = Cfb::open(bytes).expect("cfb opens");
        let x = extract_pub_positioned(&cfb, "test.pub", 1000).expect("extract");
        assert!(
            x.positioned,
            "the layout layer read the file: {:?}",
            x.loss.notes
        );
        assert_eq!(x.loss.shapes_total, 1);
        assert_eq!(x.loss.shapes_placed, 1);
        assert_eq!(x.loss.shapes_unplaced, 0);
        assert_eq!(x.loss.stories_unplaced, 0);
        assert_eq!(x.dump.pages.len(), 1);
        assert_eq!(x.dump.pages[0].width_pt, 612.0);
        assert_eq!(x.dump.pages[0].height_pt, 792.0);
        let texts: Vec<&str> = x.dump.spans.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(texts.join(" "), "Hello placed world");
        // Every span sits inside the anchor box (72, 72,
        // 200 x 100 pt), and the first span starts at the
        // box origin plus the default text inset
        // (36576 EMU, about 2.88 pt): the center-relative
        // EMU anchor, converted.
        for sp in &x.dump.spans {
            let (sx, sy) = (sp.x.unwrap(), sp.y.unwrap());
            assert!(sx >= 72.0 && sy >= 72.0, "span at {sx}, {sy}");
            assert!(sx + sp.w.unwrap() <= 272.0 + 0.01);
            assert!(sy + sp.h.unwrap() <= 172.0 + 0.01);
        }
        let first = &x.dump.spans[0];
        assert!(
            (first.x.unwrap() - 74.88).abs() < 0.5,
            "x {}",
            first.x.unwrap()
        );
        assert!(
            (first.y.unwrap() - 74.88).abs() < 0.5,
            "y {}",
            first.y.unwrap()
        );
    }

    #[test]
    fn unknown_block_inside_a_container_degrades_to_text_only() {
        let bytes = build_cfb(contents_stream(true), quill_stream(), escher_stream());
        let cfb = Cfb::open(bytes).expect("cfb opens");
        // The extraction must NOT fail: the words still
        // come over, in reading order, with the fallback
        // stated in the loss notes.
        let x = extract_pub_positioned(&cfb, "test.pub", 1000).expect("extract keeps the words");
        assert!(!x.positioned);
        let texts: Vec<&str> = x.dump.spans.iter().map(|s| s.text.as_str()).collect();
        assert!(texts.join(" ").contains("Hello placed world"), "{texts:?}");
        assert!(
            x.loss.notes.iter().any(|n| n.contains("reading order")),
            "{:?}",
            x.loss.notes
        );
    }
}
