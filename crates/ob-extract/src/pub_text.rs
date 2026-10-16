//! Phase 2b of the native .pub reader: the Quill text rescue.
//!
//! An independent implementation from documented format facts
//! (the public reverse-engineering record summarized in the
//! project's research notes; no libmspub code). The Publisher
//! 2002-family file (Contents magic byte 0x2C) keeps all of its
//! words in the Quill stream (`Quill/QuillSub/CONTENTS`), a
//! chunked sub-format: a directory of named chunks, `TEXT`
//! holding every story's characters concatenated as UTF-16LE,
//! `STRS` the story lengths, `SYID` the story ids, `FDPP` /
//! `FDPC` the paragraph and character-run property tables
//! (entries keyed by end offsets into TEXT), `FONT` the font
//! name table, and `STSH` the stylesheet defaults.
//!
//! This phase recovers the WORDS ONLY: one span per character
//! run, never collapsing a paragraph's mixed formatting, laid
//! out in a plain reading column on synthetic letter pages and
//! trusted as NeedsReview throughout (the app's Check this
//! badge). Positions and pictures are phase 2c. What the text
//! rescue cannot account for is counted, never dropped
//! silently: see LossReport.
//!
//! One correction to the research notes, from the POI sample
//! files: the STRS length array starts at byte 12 of the chunk
//! (a count, an offset word, and one further word precede it),
//! not byte 8. Under that reading the lengths sum exactly to
//! the TEXT character count, and every cumulative boundary
//! lands on an FDPP paragraph end, in every sample checked
//! (Publisher 2010-era samples with 2, 6, 20, and 44 stories).

use crate::cfb::Cfb;
use crate::{ExtractionDump, PageInfo, SourceKind, Span, SpanTrust, DUMP_SCHEMA_VERSION};
use std::collections::HashSet;

/// What the text rescue could not account for. Every story in
/// the file lands in exactly one of emitted / excluded /
/// unresolved, and unresolved entries carry their reasons.
#[derive(Debug, Clone, Default)]
pub struct LossReport {
    /// Stories listed in the file's story table (STRS).
    pub stories_total: usize,
    /// Stories whose words are in the dump.
    pub stories_emitted: usize,
    /// Stories intentionally skipped: empty, or nothing in
    /// them but spaces and paragraph marks.
    pub stories_excluded: usize,
    /// Stories that could not be read at all (their claimed
    /// characters run past the end of the text, or the story
    /// table itself is broken). Their text is NOT in the dump.
    pub stories_unresolved: usize,
    /// Stories whose words were emitted but whose SYID entry
    /// is missing or duplicated, so the join from a shape to
    /// its words cannot be resolved in phase 2c.
    pub stories_with_unresolved_ids: usize,
    /// SYID entries naming a story index that does not exist.
    pub ids_without_story: usize,
    /// Why each unresolved item is unresolved, in plain words.
    pub unresolved_reasons: Vec<String>,
    /// Characters carried into spans.
    pub characters_emitted: usize,
    /// Characters claimed by stories that could not be read,
    /// plus characters in TEXT no story claims.
    pub characters_unresolved: usize,
    /// Character runs emitted (before control-character splits).
    pub runs_emitted: usize,
    /// Runs whose font and size both came from stylesheet
    /// defaults rather than the run's own property record: the
    /// STSH path is partly empirical in the record, so these
    /// are counted rather than presented as certain.
    pub runs_styled_by_default: usize,
    /// Control characters mapped to a space or a break, or
    /// dropped, on their way into spans. Paragraph marks are
    /// structural and are not counted here.
    pub control_chars_mapped: usize,
    // Phase 2c (positions) accounting; all zero in a
    // text-only rescue.
    /// Shapes listed on real pages of the file, excluding
    /// deleted shapes, pure group containers, and
    /// decoration shapes (see `decoration_shapes_skipped`).
    pub shapes_total: usize,
    /// Of those, the shapes whose contents are on a page at
    /// the place the file gives them (words, table cells,
    /// word art, or a picture placeholder).
    pub shapes_placed: usize,
    /// Of those, the shapes whose contents could not be
    /// placed (no position, or their words never joined).
    /// Every one of them has its cause counted in
    /// `unplaced_shape_reasons`.
    pub shapes_unplaced: usize,
    /// Non-group shapes carrying nothing placeable at all:
    /// no story, no picture, no word art. Lines, rectangles,
    /// and empty frames are the page's furniture, not
    /// content that missed its place, so they are counted
    /// here and kept out of `shapes_total`/`shapes_unplaced`.
    pub decoration_shapes_skipped: usize,
    /// Why each unplaced content shape missed, counted by
    /// cause, in plain words, with example pages and shape
    /// numbers: "the drawing layer has no entry for it, so
    /// its place on the page is unknown (2): page 3 shape
    /// 87, page 5 shape 91".
    pub unplaced_shape_reasons: Vec<String>,
    /// Stories whose words were placed inside shape boxes
    /// on their pages (counted once per story, however many
    /// boxes the story flows through).
    pub stories_placed_in_boxes: usize,
    /// Shapes listed on pages but marked deleted in the
    /// drawing layer: skipped on purpose.
    pub shapes_deleted_skipped: usize,
    /// Alternate-shape chunks (type 0x20) set aside because
    /// the page lists their regular counterpart.
    pub alternate_shapes_excluded: usize,
    /// Shapes from applied master pages whose contents were
    /// emitted onto the page (badged Check this).
    pub master_shapes_emitted: usize,
    /// Table cells whose words were emitted as their own
    /// placed pieces.
    pub table_cells_emitted: usize,
    /// WordArt texts recovered from the drawing layer.
    pub wordart_emitted: usize,
    /// Pictures named in the file's image store.
    pub pictures_found: usize,
    /// Pictures whose image bytes came over. The dump
    /// schema carries no image bytes, so this stays zero
    /// until it can; the loss is reported, not hidden.
    pub pictures_carried: usize,
    /// Stories whose words were emitted but landed on the
    /// reading-order pages at the end because no shape
    /// claimed them (or their shape had no position).
    pub stories_unplaced: usize,
    /// Stories whose words ran past the bottom of their
    /// page: the words that fit stayed in the box, and the
    /// rest moved to the reading-order pages.
    pub stories_overflowed: usize,
    /// Structural fallbacks taken (a broken property table, an
    /// uncovered tail), in plain words.
    pub notes: Vec<String>,
}

/// The Publisher family byte from the root Contents stream
/// (0x2C = 2002 and later, 0x22 = the 2000/97 family), when
/// the file carries one.
pub fn contents_version(cfb: &Cfb) -> Option<u8> {
    let contents = cfb.read_stream_path(&["Contents"]).ok()?;
    if contents.len() >= 4 && contents[0] == 0xE8 && contents[1] == 0xAC {
        Some(contents[2])
    } else {
        None
    }
}

/// The whole rescue: a 0x2C-family .pub file (already opened
/// as a compound file) to a text dump plus its loss report.
pub fn extract_pub_text(
    cfb: &Cfb,
    source_name: &str,
    source_bytes: usize,
) -> Result<(ExtractionDump, LossReport), String> {
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
    extract_from_quill(&quill, source_name, source_bytes)
}

// ----------------------------------------------------------------
// Quill chunk directory

struct Chunk<'a> {
    name: String,
    /// The directory entry's chunk id: carried for phase
    /// 2c, which joins chunks (tables, cells) by id.
    #[allow(dead_code)]
    id: u16,
    bytes: &'a [u8],
    /// Offset of the chunk within the Quill stream: FDPP/FDPC
    /// end offsets are absolute Quill offsets, so TEXT's is
    /// needed to turn them into character positions.
    offset: usize,
}

/// Walk the linked directory blocks (the first is at 0x18).
/// Each block: 2 skipped bytes, a U16 chunk count, a U32
/// offset of the next block (0xFFFFFFFF ends the list).
/// Each entry is 24 bytes: U16 entry size, a 4-byte name,
/// a U16 chunk id, 4 skipped bytes, the name again, a U32
/// chunk offset, a U32 chunk length.
fn quill_chunks(quill: &[u8]) -> Vec<Chunk<'_>> {
    let mut out = Vec::new();
    let mut visited: HashSet<usize> = HashSet::new();
    let mut block_off = 0x18usize;
    loop {
        if block_off == 0xFFFF_FFFF || !visited.insert(block_off) || visited.len() > 128 {
            break;
        }
        let Some(head) = quill.get(block_off..block_off + 8) else {
            break;
        };
        let count = u16::from_le_bytes([head[2], head[3]]) as usize;
        let next = u32::from_le_bytes([head[4], head[5], head[6], head[7]]) as usize;
        let mut p = block_off + 8;
        for _ in 0..count {
            let Some(entry) = quill.get(p..p + 24) else {
                break;
            };
            let size = u16::from_le_bytes([entry[0], entry[1]]) as usize;
            if size < 24 {
                break; // an entry that cannot hold its fields ends the block
            }
            let name = String::from_utf8_lossy(&entry[2..6])
                .trim()
                .trim_matches('\0')
                .to_string();
            let id = u16::from_le_bytes([entry[6], entry[7]]);
            let off = u32::from_le_bytes([entry[16], entry[17], entry[18], entry[19]]) as usize;
            let len = u32::from_le_bytes([entry[20], entry[21], entry[22], entry[23]]) as usize;
            if let Some(bytes) = quill.get(off..off.saturating_add(len)) {
                out.push(Chunk {
                    name,
                    id,
                    bytes,
                    offset: off,
                });
            }
            p += size;
        }
        block_off = next;
    }
    out
}

fn chunks_named<'a>(chunks: &'a [Chunk<'a>], name: &str) -> Vec<&'a Chunk<'a>> {
    chunks.iter().filter(|c| c.name == name).collect()
}

// ----------------------------------------------------------------
// The (id, type) block grammar style records use, shared with
// the Contents stream: the type byte alone frames the payload.
// A block that cannot be framed stops the walk (the scoped
// strictness rule: never guess a length).

#[derive(Clone, Copy)]
struct Block<'a> {
    id: u8,
    /// Value bytes for fixed-size types; for variable-length
    /// (container) types, the content after the length word,
    /// where sub-blocks live.
    payload: &'a [u8],
    container: bool,
}

pub(crate) fn payload_fixed_len(ty: u8) -> Option<usize> {
    match ty {
        0x78 | 0x05 | 0x08 | 0x0A => Some(0),
        0x10 | 0x12 | 0x18 | 0x1A | 0x07 => Some(2),
        0x20 | 0x22 | 0x58 | 0x68 | 0x70 | 0xB8 => Some(4),
        0x28 => Some(8),
        0x38 => Some(16),
        0x48 => Some(24),
        _ => None,
    }
}

pub(crate) fn is_variable_len(ty: u8) -> bool {
    matches!(ty, 0xC0 | 0x80 | 0x82 | 0x88 | 0x8A | 0x90 | 0x98 | 0xA0)
}

fn walk_blocks(bytes: &[u8]) -> Vec<Block<'_>> {
    let mut out = Vec::new();
    let mut pos = 0usize;
    while pos + 2 <= bytes.len() {
        let (id, ty) = (bytes[pos], bytes[pos + 1]);
        if let Some(fixed) = payload_fixed_len(ty) {
            let Some(payload) = bytes.get(pos + 2..pos + 2 + fixed) else {
                break;
            };
            out.push(Block {
                id,
                payload,
                container: false,
            });
            pos += 2 + fixed;
        } else if is_variable_len(ty) {
            let Some(lenb) = bytes.get(pos + 2..pos + 6) else {
                break;
            };
            let total = u32::from_le_bytes([lenb[0], lenb[1], lenb[2], lenb[3]]) as usize;
            // The length counts itself but not the 2 header
            // bytes, and must cover at least its own 4 bytes.
            if total < 4 || pos + 2 + total > bytes.len() {
                break;
            }
            out.push(Block {
                id,
                payload: &bytes[pos + 6..pos + 2 + total],
                container: true,
            });
            pos += 2 + total;
        } else {
            break; // unknown type: never guess a length
        }
    }
    out
}

pub(crate) fn payload_int(payload: &[u8]) -> u64 {
    let mut v = 0u64;
    for (i, b) in payload.iter().take(8).enumerate() {
        v |= (*b as u64) << (8 * i);
    }
    v
}

/// One style record (FDPP/FDPC entry, or one STSH element):
/// a U32 total length that counts itself, then blocks. The
/// blocks region is bounded by the record and by the
/// enclosing limit the caller supplies.
fn style_blocks<'a>(chunk: &'a [u8], record_off: usize, limit: usize) -> Vec<Block<'a>> {
    let Some(lenb) = chunk.get(record_off..record_off + 4) else {
        return Vec::new();
    };
    let total = u32::from_le_bytes([lenb[0], lenb[1], lenb[2], lenb[3]]) as usize;
    if total < 4 {
        return Vec::new();
    }
    let end = record_off.saturating_add(total).min(limit).min(chunk.len());
    if record_off + 4 > end {
        return Vec::new();
    }
    walk_blocks(&chunk[record_off + 4..end])
}

// ----------------------------------------------------------------
// Character properties

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct CharProps {
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    /// Font size in points.
    pub size_pt: Option<f32>,
    pub font_index: Option<usize>,
}

impl CharProps {
    /// `over` wins wherever it states a property.
    fn merged(self, over: CharProps) -> CharProps {
        CharProps {
            bold: over.bold.or(self.bold),
            italic: over.italic.or(self.italic),
            size_pt: over.size_pt.or(self.size_pt),
            font_index: over.font_index.or(self.font_index),
        }
    }
}

/// The font index inside a font container (block id 0x24):
/// its first sub-block's value carries the index in the high
/// half of its first word (the low half is 0x1800 in every
/// sample seen; FONT has a handful of entries, so the low
/// half cannot be the index).
fn font_index_of(block: &Block) -> Option<usize> {
    if !block.container {
        return None;
    }
    let subs = walk_blocks(block.payload);
    let first = subs.first()?;
    if first.payload.len() >= 4 {
        Some((payload_int(&first.payload[0..4]) >> 16) as usize)
    } else if first.payload.len() == 2 {
        Some(payload_int(first.payload) as usize)
    } else {
        None
    }
}

/// Interpret a character style record's blocks. Bold is id
/// 0x02 or 0x37 present; italic id 0x03 or 0x38 (the record
/// documents both ids; the reference implementation applies
/// only the first of each pair, and the second pair's exact
/// role is unsettled, so presence of either sets the flag).
/// Size is id 0x0C, its value in EMU (12700 to a point).
/// A second size id, 0x39, is ignored: its purpose is
/// unknown in the record.
fn char_props(blocks: &[Block]) -> CharProps {
    let mut props = CharProps::default();
    for b in blocks {
        match b.id {
            0x02 | 0x37 => props.bold = Some(true),
            0x03 | 0x38 => props.italic = Some(true),
            0x0C => {
                let emu = payload_int(b.payload);
                if emu > 0 {
                    let pt = emu as f32 / 12_700.0;
                    if (1.0..=400.0).contains(&pt) {
                        props.size_pt = Some(pt);
                    }
                }
            }
            0x24 => {
                if let Some(idx) = font_index_of(b) {
                    props.font_index = Some(idx);
                }
            }
            _ => {}
        }
    }
    props
}

/// A paragraph record's default character-style index:
/// block id 0x19 in the paragraph style.
fn para_char_index(blocks: &[Block]) -> Option<usize> {
    blocks
        .iter()
        .find(|b| b.id == 0x19)
        .map(|b| payload_int(b.payload) as usize)
}

// ----------------------------------------------------------------
// The FONT chunk: a U32 unknown, a U32 font count, 12 skipped
// bytes and a 4-bytes-per-font table the record does not
// interpret, then per font a U16 name length in characters,
// the UTF-16LE name, and a trailing U32.

fn parse_fonts(chunk: &[u8]) -> Vec<String> {
    let mut names = Vec::new();
    if chunk.len() < 8 {
        return names;
    }
    let count = u32::from_le_bytes([chunk[4], chunk[5], chunk[6], chunk[7]]) as usize;
    let mut p = 8 + 12 + 4 * count.min(4096);
    for _ in 0..count {
        let Some(lenb) = chunk.get(p..p + 2) else {
            break;
        };
        let nchars = u16::from_le_bytes([lenb[0], lenb[1]]) as usize;
        let Some(nameb) = chunk.get(p + 2..p + 2 + nchars * 2) else {
            break;
        };
        let units: Vec<u16> = nameb
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        names.push(
            String::from_utf16_lossy(&units)
                .trim()
                .trim_matches('\0')
                .to_string(),
        );
        p += 2 + nchars * 2 + 4;
    }
    names
}

// ----------------------------------------------------------------
// STSH, the stylesheet: the record says the parser uses the
// SECOND STSH chunk in the file (the first is a stub in the
// samples). Layout: a U32 unknown, a U32 element count, 12
// skipped bytes, that many U32 offsets; each element sits at
// chunk offset 20 + offset, skips a U16, and then holds a
// style record. Elements alternate character style (even
// index) and paragraph style (odd index). How these defaults
// formally associate is marked empirical in the record
// (a FIXME in the reference), so values that arrive only
// through STSH are counted in the loss report.

struct StyleSheet {
    /// Character styles only: the even-indexed elements.
    /// The odd (paragraph) elements are walked for parity
    /// and otherwise unused in this phase.
    char_styles: Vec<CharProps>,
}

fn parse_stsh(chunks: &[&Chunk]) -> Option<StyleSheet> {
    let chunk = if chunks.len() >= 2 {
        chunks[1]
    } else {
        chunks.first()?
    };
    let bytes = chunk.bytes;
    if bytes.len() < 20 {
        return None;
    }
    let count = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as usize;
    if count > 1024 {
        return None;
    }
    let mut offsets = Vec::new();
    for i in 0..count {
        let p = 20 + 4 * i;
        let b = bytes.get(p..p + 4)?;
        offsets.push(u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize);
    }
    let mut sheet = StyleSheet {
        char_styles: Vec::new(),
    };
    for (i, off) in offsets.iter().enumerate() {
        let start = 20 + off;
        let end = offsets
            .get(i + 1)
            .map(|o| 20 + o)
            .unwrap_or(bytes.len())
            .min(bytes.len());
        // An element: a skipped U16, then a style record
        // whose length counts from the record's own position,
        // bounded by the element's end.
        let blocks = if start + 2 <= end {
            style_blocks(bytes, start + 2, end)
        } else {
            Vec::new()
        };
        if i % 2 == 0 {
            sheet.char_styles.push(char_props(&blocks));
        }
    }
    Some(sheet)
}

// ----------------------------------------------------------------
// FDPP / FDPC property tables

/// One partition entry: the character range it covers and
/// where its style record lives (chunk index within the
/// caller's chunk list for that name, and the offset of the
/// record within that chunk). A `None` record means the entry
/// carries no style of its own.
type RangedRecord = (usize, usize, Option<(usize, usize)>);

struct Partition {
    ranges: Vec<RangedRecord>,
}

/// Parse one FDPP or FDPC table across all its chunks, in
/// directory order: a U16 entry count, 6 skipped bytes, one
/// U32 end offset per entry (absolute within the Quill
/// stream), then one U16 style-record offset per entry
/// (within its own chunk). Returns None when the table is
/// unusable as a whole: an end offset outside TEXT, an odd
/// byte count, or ends running backwards. Scoped strictness:
/// the layer is abandoned, never guessed at.
fn parse_partition(chunks: &[&Chunk], text_off: usize, text_chars: usize) -> Option<Partition> {
    let mut ranges = Vec::new();
    let mut prev_char_end = 0usize;
    for (ci, chunk) in chunks.iter().enumerate() {
        let bytes = chunk.bytes;
        if bytes.len() < 8 {
            return None;
        }
        let count = u16::from_le_bytes([bytes[0], bytes[1]]) as usize;
        if 8 + 6 * count > bytes.len() {
            return None;
        }
        for i in 0..count {
            let ep = 8 + 4 * i;
            let end = u32::from_le_bytes([bytes[ep], bytes[ep + 1], bytes[ep + 2], bytes[ep + 3]])
                as usize;
            let op = 8 + 4 * count + 2 * i;
            let style_off = u16::from_le_bytes([bytes[op], bytes[op + 1]]) as usize;
            if end < text_off || (end - text_off) % 2 != 0 {
                return None;
            }
            let char_end = (end - text_off) / 2;
            if char_end > text_chars || char_end < prev_char_end {
                return None;
            }
            let record = if style_off > 0 && style_off + 4 <= bytes.len() {
                Some((ci, style_off))
            } else {
                None
            };
            ranges.push((prev_char_end, char_end, record));
            prev_char_end = char_end;
        }
    }
    if ranges.is_empty() {
        return None;
    }
    Some(Partition { ranges })
}

/// The blocks of one partition entry's style record.
fn record_blocks<'a>(chunks: &[&'a Chunk<'a>], record: &Option<(usize, usize)>) -> Vec<Block<'a>> {
    let Some((ci, off)) = record else {
        return Vec::new();
    };
    let chunk = chunks[*ci];
    style_blocks(chunk.bytes, *off, chunk.bytes.len())
}

// ----------------------------------------------------------------
// The reading-column layout. This phase is text-only: spans
// are stacked in reading order on synthetic letter pages so
// the existing draft builder can group them into lines and
// blocks. Positions claim nothing about the original page.

const PAGE_W: f32 = 612.0;
const PAGE_H: f32 = 792.0;
const X0: f32 = 72.0;
const Y0: f32 = 72.0;
const COL_W: f32 = 468.0;
const BOTTOM: f32 = PAGE_H - 72.0;
const DEFAULT_SIZE: f32 = 11.0;

#[derive(Default)]
struct Layout {
    page: u32,
    x: f32,
    y: f32,
    /// Largest font size placed on the current line.
    line_max: f32,
    line_used: bool,
    pending_space: bool,
    prev_ended_space: bool,
}

impl Layout {
    fn new() -> Layout {
        Layout {
            page: 0,
            x: X0,
            y: Y0,
            line_max: DEFAULT_SIZE,
            line_used: false,
            pending_space: false,
            prev_ended_space: false,
        }
    }

    fn page_break(&mut self) {
        self.page += 1;
        self.y = Y0;
        self.x = X0;
        self.line_max = DEFAULT_SIZE;
        self.line_used = false;
        self.pending_space = false;
        self.prev_ended_space = false;
    }

    fn line_break(&mut self) {
        self.y += self.line_max.max(1.0) * 1.35;
        self.x = X0;
        self.line_max = DEFAULT_SIZE;
        self.line_used = false;
        self.pending_space = false;
        self.prev_ended_space = false;
    }

    /// A paragraph ends: its line step plus a visible gap, so
    /// the draft builder reads each paragraph as its own
    /// block. Room is checked when the next span is placed.
    fn paragraph_break(&mut self, size: f32) {
        self.y += self.line_max.max(1.0) * 1.35 + size * 0.9;
        self.x = X0;
        self.line_max = DEFAULT_SIZE;
        self.line_used = false;
        self.pending_space = false;
        self.prev_ended_space = false;
    }

    fn story_break(&mut self) {
        self.y += 6.0;
    }
}

/// The span font name for a run: the FONT table name for the
/// run's font index, with the draft builder's italic signal
/// (a "-Italic" suffix, the same convention the IDML
/// importer uses, since the dump schema carries bold but
/// not italic).
pub(crate) fn span_font(props: &CharProps, fonts: &[String]) -> Option<String> {
    let name = fonts.get(props.font_index?)?.clone();
    if props.italic == Some(true) && !name.to_lowercase().contains("italic") {
        Some(format!("{name}-Italic"))
    } else {
        Some(name)
    }
}

/// Emit one run piece (a run intersected with one story and
/// one paragraph), splitting at control characters: a
/// vertical tab is a line break within the paragraph, a form
/// feed is a page break, a tab becomes a space, the
/// paragraph mark and other control characters never reach
/// a span. (In the 97/98 notes the vertical tab is a
/// non-breaking space and the form feed is simply dropped;
/// for Quill text the break reading is the useful one, and
/// no control character is ever emitted raw.)
#[allow(clippy::too_many_arguments)]
fn emit_piece(
    layout: &mut Layout,
    spans: &mut Vec<Span>,
    text: &[char],
    props: &CharProps,
    fonts: &[String],
    loss: &mut LossReport,
) {
    let mut segment = String::new();
    for &c in text {
        match c {
            '\r' => { /* the paragraph mark: structural */ }
            '\n' => {
                if segment.is_empty() {
                    // The LF of a CRLF pair, or a stray one at
                    // a boundary: no words in it.
                    loss.control_chars_mapped += 1;
                } else {
                    flush_segment(layout, spans, &segment, props, fonts, loss);
                    segment.clear();
                    layout.line_break();
                    loss.control_chars_mapped += 1;
                }
            }
            '\u{b}' => {
                flush_segment(layout, spans, &segment, props, fonts, loss);
                segment.clear();
                layout.line_break();
                loss.control_chars_mapped += 1;
            }
            '\u{c}' => {
                flush_segment(layout, spans, &segment, props, fonts, loss);
                segment.clear();
                layout.page_break();
                loss.control_chars_mapped += 1;
            }
            '\t' => {
                segment.push(' ');
                loss.control_chars_mapped += 1;
            }
            c if c.is_control() => {
                loss.control_chars_mapped += 1;
            }
            c => segment.push(c),
        }
    }
    flush_segment(layout, spans, &segment, props, fonts, loss);
}

fn flush_segment(
    layout: &mut Layout,
    spans: &mut Vec<Span>,
    segment: &str,
    props: &CharProps,
    fonts: &[String],
    loss: &mut LossReport,
) {
    if segment.is_empty() {
        return;
    }
    if segment.trim().is_empty() {
        layout.pending_space = true;
        return;
    }
    let size = props.size_pt.unwrap_or(DEFAULT_SIZE);
    let nchars = segment.chars().count() as f32;
    let w = (nchars * size * 0.52).max(size * 0.5);
    if layout.line_used && layout.x + w > X0 + COL_W {
        layout.line_break();
    }
    if layout.y + size * 1.2 > BOTTOM {
        layout.page_break();
    }
    // A space at a run boundary must survive as a visible gap
    // in the placed spans: the draft builder joins fragments
    // by their real positions and only earns a space from a
    // real gap.
    let gap = if layout.line_used
        && (layout.pending_space || layout.prev_ended_space || segment.starts_with(' '))
    {
        size * 0.3
    } else {
        0.0
    };
    let x = layout.x + gap;
    spans.push(Span {
        page: layout.page,
        text: segment.to_string(),
        trust: SpanTrust::NeedsReview,
        x: Some(x),
        y: Some(layout.y),
        w: Some(w),
        h: Some(size * 1.2),
        font_size: props.size_pt,
        bold: Some(props.bold.unwrap_or(false)),
        font: span_font(props, fonts),
        align: None,
        italic: props.italic.unwrap_or(false),
        color: [0, 0, 0],
        runs: Vec::new(),
    });
    loss.characters_emitted += segment.chars().count();
    layout.x = x + w;
    layout.line_used = true;
    layout.line_max = layout.line_max.max(size);
    layout.pending_space = false;
    layout.prev_ended_space = segment.ends_with(' ');
}

/// Split a character range at paragraph marks: the fallback
/// paragraph partition when the FDPP table cannot be used.
fn split_at_marks(chars: &[char], start: usize, end: usize) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut p = start;
    for (j, &c) in chars[start..end].iter().enumerate() {
        if c == '\r' {
            out.push((p, start + j + 1));
            p = start + j + 1;
        }
    }
    if p < end {
        out.push((p, end));
    }
    out
}

/// The paragraph's base character properties: the STSH
/// character style its FDPP record names (block 0x19), else
/// the stylesheet's first character style, else nothing.
fn para_base(
    fdpp_chunks: &[&Chunk],
    record: &Option<(usize, usize)>,
    stsh: &Option<StyleSheet>,
) -> CharProps {
    let Some(sheet) = stsh else {
        return CharProps::default();
    };
    let named = record
        .as_ref()
        .and_then(|r| para_char_index(&record_blocks(fdpp_chunks, &Some(*r))));
    if let Some(i) = named {
        if let Some(props) = sheet.char_styles.get(i) {
            return *props;
        }
    }
    sheet.char_styles.first().copied().unwrap_or_default()
}

// ----------------------------------------------------------------
// The parsed Quill stream, shared by the text-only rescue
// (this module) and the positioned reader (pub_layout).
// Parsing accounts for every story exactly once; a layout
// (the reading column below, or real boxes in phase 2c)
// then turns the parsed runs into spans.

/// One character run inside a story paragraph, with the
/// character properties that apply to it (the paragraph's
/// stylesheet base merged with the run's own record).
#[derive(Clone)]
pub(crate) struct RunParse {
    pub props: CharProps,
    /// Character range in TEXT.
    pub start: usize,
    pub end: usize,
    /// The run's font index resolved to a name in the FONT
    /// table: the positioned reader trusts a placed run by
    /// this (deliverable rule in the 2c plan).
    pub font_resolved: bool,
}

/// One paragraph of a story: its runs in order, and the
/// character range it covers in TEXT (marks included).
#[derive(Clone)]
pub(crate) struct ParaParse {
    pub runs: Vec<RunParse>,
    /// Largest effective font size among the runs.
    pub max_size: f32,
    /// End of the paragraph's character range in TEXT
    /// (the table reader measures cell ends against it).
    pub char_end: usize,
}

/// One story of the file, after the parse's accounting.
pub(crate) struct StoryParse {
    pub index: usize,
    /// The story's SYID id, when the file gives it exactly
    /// one: the join a shape's text id uses in phase 2c.
    pub syid: Option<u32>,
    /// Whether the story's words are emitted (not excluded
    /// as blank, not unresolved).
    pub emitted: bool,
    /// Start of the story's character range in TEXT (the
    /// table reader measures cell ends against it).
    pub char_start: usize,
    /// Paragraphs with their runs; only for emitted
    /// stories.
    pub paragraphs: Vec<ParaParse>,
}

/// The whole Quill stream, parsed once.
pub(crate) struct QuillParse {
    pub chars: Vec<char>,
    pub fonts: Vec<String>,
    pub stories: Vec<StoryParse>,
    /// Table cell text ends by story index, from the TCD
    /// chunks (a TCD chunk's directory id is the story
    /// index it describes): character ends within the
    /// story, converted from the stored byte ends (halved;
    /// the record compensates all but the last end by one
    /// character for the cell's terminating mark).
    pub table_cell_ends: Vec<Option<Vec<usize>>>,
}

/// Parse the Quill stream once: the story accounting and
/// the run/paragraph/style resolution every layout shares.
/// The returned loss report carries the story-level books;
/// a layout adds its character counters on top.
pub(crate) fn parse_quill(quill: &[u8]) -> Result<(QuillParse, LossReport), String> {
    let chunks = quill_chunks(quill);
    let mut loss = LossReport::default();

    // TEXT: every story's characters, UTF-16LE, concatenated.
    let text_chunks = chunks_named(&chunks, "TEXT");
    let Some(text_chunk) = text_chunks.first() else {
        return Err("that file's text stream has no TEXT chunk".into());
    };
    if text_chunks.len() > 1 {
        loss.notes
            .push("the file carries more than one TEXT chunk; only the first was read".into());
    }
    let text_off = text_chunk.offset;
    let units: Vec<u16> = text_chunk
        .bytes
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    let chars: Vec<char> = char::decode_utf16(units)
        .map(|r| r.unwrap_or(char::REPLACEMENT_CHARACTER))
        .collect();
    let text_chars = chars.len();

    let fonts = chunks_named(&chunks, "FONT")
        .first()
        .map(|c| parse_fonts(c.bytes))
        .unwrap_or_default();
    let stsh = parse_stsh(&chunks_named(&chunks, "STSH"));

    // STRS: the story lengths. A count, an offset word, and
    // one further word precede the U32 character lengths
    // (see the module note). Stories are consecutive ranges
    // of TEXT in table order.
    let mut stories: Vec<(usize, usize)> = Vec::new();
    let strs_ok = chunks_named(&chunks, "STRS").first().and_then(|c| {
        let b = c.bytes;
        if b.len() < 12 {
            return None;
        }
        let count = u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize;
        if count > 100_000 || 12 + 4 * count > b.len() {
            return None;
        }
        let mut out = Vec::new();
        let mut at = 0usize;
        for i in 0..count {
            let p = 12 + 4 * i;
            let len = u32::from_le_bytes([b[p], b[p + 1], b[p + 2], b[p + 3]]) as usize;
            out.push((at, at + len));
            at += len;
        }
        Some(out)
    });
    match strs_ok {
        Some(s) => stories = s,
        None => {
            loss.notes.push(
                "the story table (STRS) could not be read; the whole text was \
                 taken as one story"
                    .into(),
            );
            stories.push((0, text_chars));
        }
    }
    loss.stories_total = stories.len();

    // SYID: the story ids, parallel to STRS.
    let syid_ids: Vec<u32> = chunks_named(&chunks, "SYID")
        .first()
        .and_then(|c| {
            let b = c.bytes;
            if b.len() < 8 {
                return None;
            }
            let count = u32::from_le_bytes([b[4], b[5], b[6], b[7]]) as usize;
            if count > 100_000 || 8 + 4 * count > b.len() {
                return None;
            }
            Some(
                (0..count)
                    .map(|i| {
                        let p = 8 + 4 * i;
                        u32::from_le_bytes([b[p], b[p + 1], b[p + 2], b[p + 3]])
                    })
                    .collect(),
            )
        })
        .unwrap_or_default();
    if syid_ids.is_empty() && !stories.is_empty() {
        loss.stories_with_unresolved_ids = stories.len();
        loss.unresolved_reasons.push(
            "the file has no SYID table, so no story's words can be tied to a \
             shape in a later step"
                .into(),
        );
    }
    if syid_ids.len() > stories.len() {
        loss.ids_without_story = syid_ids.len() - stories.len();
        loss.unresolved_reasons.push(format!(
            "{} SYID entries name stories that do not exist",
            loss.ids_without_story
        ));
    }

    // FDPP: the paragraph partition, or marks alone.
    let fdpp_chunks = chunks_named(&chunks, "FDPP");
    let mut paragraphs: Vec<RangedRecord> =
        match parse_partition(&fdpp_chunks, text_off, text_chars) {
            Some(part) => part.ranges,
            None => {
                if !fdpp_chunks.is_empty() {
                    loss.notes.push(
                        "the paragraph table (FDPP) could not be read; paragraphs \
                         were split at paragraph marks"
                            .into(),
                    );
                }
                split_at_marks(&chars, 0, text_chars)
                    .iter()
                    .map(|&(a, b)| (a, b, None))
                    .collect()
            }
        };
    let para_end = paragraphs.last().map(|p| p.1).unwrap_or(0);
    if para_end < text_chars {
        loss.notes.push(
            "the paragraph table ends before the text does; the tail was split \
             at paragraph marks"
                .into(),
        );
        for (a, b) in split_at_marks(&chars, para_end, text_chars) {
            paragraphs.push((a, b, None));
        }
    }

    // FDPC: the character-run partition, or paragraphs.
    let fdpc_chunks = chunks_named(&chunks, "FDPC");
    let mut runs: Vec<RangedRecord> = match parse_partition(&fdpc_chunks, text_off, text_chars) {
        Some(part) => part.ranges,
        None => {
            if !fdpc_chunks.is_empty() {
                loss.notes.push(
                    "the character-run table (FDPC) could not be read; each \
                         paragraph was taken as one run"
                        .into(),
                );
            }
            paragraphs.iter().map(|(a, b, _)| (*a, *b, None)).collect()
        }
    };
    let run_end = runs.last().map(|r| r.1).unwrap_or(0);
    if run_end < text_chars {
        loss.notes.push(
            "the character-run table ends before the text does; the tail took \
             the paragraph defaults"
                .into(),
        );
        for (a, b, _) in &paragraphs {
            let (a, b) = ((*a).max(run_end), *b);
            if a < b {
                runs.push((a, b, None));
            }
        }
    }

    // TCD: table cell text ends. A TCD chunk's directory id
    // is the index of the story it describes. The chunk
    // opens with a U32 that is one less than the number of
    // ends; the ends themselves are U32 byte offsets within
    // the story's text, starting at byte 12 of the chunk.
    let mut table_cell_ends: Vec<Option<Vec<usize>>> = vec![None; stories.len()];
    for chunk in chunks_named(&chunks, "TCD") {
        let idx = chunk.id as usize;
        if idx >= stories.len() || table_cell_ends[idx].is_some() {
            continue;
        }
        let b = chunk.bytes;
        if b.len() < 12 {
            continue;
        }
        let n = u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize + 1;
        if n > 4096 || 12 + 4 * n > b.len() {
            continue;
        }
        let story_len = stories[idx].1 - stories[idx].0;
        let mut ends = Vec::with_capacity(n);
        for i in 0..n {
            let p = 12 + 4 * i;
            let raw = u32::from_le_bytes([b[p], b[p + 1], b[p + 2], b[p + 3]]) as usize;
            // All but the last stored end excludes the
            // cell's terminating mark: one character back.
            let bytes = if i + 1 == n { raw } else { raw + 2 };
            ends.push((bytes / 2).min(story_len));
        }
        table_cell_ends[idx] = Some(ends);
    }

    // Account for every story, and resolve the runs of the
    // emitted ones.
    let mut story_parses: Vec<StoryParse> = Vec::new();
    let mut excluded_chars = 0usize;
    let mut covered_end = 0usize;
    // Input characters covered by resolved runs (they come
    // out of a layout as span text, breaks, or mapped
    // controls).
    let mut consumed_chars = 0usize;
    for (si, &(s0, s1)) in stories.iter().enumerate() {
        let mut parse = StoryParse {
            index: si,
            syid: None,
            emitted: false,
            char_start: s0,
            paragraphs: Vec::new(),
        };
        // The SYID join for this story.
        if !syid_ids.is_empty() {
            match syid_ids.get(si) {
                None => {
                    loss.stories_with_unresolved_ids += 1;
                    loss.unresolved_reasons.push(format!(
                        "story {} has no SYID entry, so its words cannot be \
                         tied to a shape in a later step",
                        si + 1
                    ));
                }
                Some(id) if syid_ids.iter().filter(|x| *x == id).count() > 1 => {
                    loss.stories_with_unresolved_ids += 1;
                    loss.unresolved_reasons.push(format!(
                        "story {} shares SYID {} with another story, so its \
                         words cannot be tied to one shape in a later step",
                        si + 1,
                        id
                    ));
                }
                Some(id) => parse.syid = Some(*id),
            }
        }
        if s1 > text_chars {
            loss.stories_unresolved += 1;
            loss.unresolved_reasons.push(format!(
                "story {} could not be read: it claims characters {}..{} but \
                 the text holds {} characters",
                si + 1,
                s0,
                s1,
                text_chars
            ));
            story_parses.push(parse);
            continue;
        }
        covered_end = covered_end.max(s1);
        let story_chars = &chars[s0..s1];
        if story_chars
            .iter()
            .all(|c| c.is_whitespace() || c.is_control())
        {
            loss.stories_excluded += 1;
            excluded_chars += s1 - s0;
            story_parses.push(parse);
            continue;
        }
        loss.stories_emitted += 1;
        parse.emitted = true;
        for (p0, p1, prec) in &paragraphs {
            let (a, b) = ((*p0).max(s0), (*p1).min(s1));
            if a >= b {
                continue;
            }
            let base = para_base(&fdpp_chunks, prec, &stsh);
            let mut para = ParaParse {
                runs: Vec::new(),
                max_size: DEFAULT_SIZE,
                char_end: b,
            };
            for (r0, r1, rrec) in &runs {
                let (ra, rb) = ((*r0).max(a), (*r1).min(b));
                if ra >= rb {
                    continue;
                }
                let run_props = char_props(&record_blocks(&fdpc_chunks, rrec));
                if run_props.font_index.is_none() && run_props.size_pt.is_none() {
                    loss.runs_styled_by_default += 1;
                }
                loss.runs_emitted += 1;
                consumed_chars += rb - ra;
                let eff = base.merged(run_props);
                para.max_size = para.max_size.max(eff.size_pt.unwrap_or(DEFAULT_SIZE));
                para.runs.push(RunParse {
                    props: eff,
                    start: ra,
                    end: rb,
                    font_resolved: eff
                        .font_index
                        .map(|i| fonts.get(i).is_some())
                        .unwrap_or(false),
                });
            }
            parse.paragraphs.push(para);
        }
        story_parses.push(parse);
    }
    if covered_end < text_chars {
        loss.unresolved_reasons.push(format!(
            "{} characters in the text belong to no story and were left out",
            text_chars - covered_end
        ));
    }
    // The books balance by construction: every character in
    // TEXT is covered by a resolved run of an emitted story,
    // sits in an excluded story, or is unresolved (claimed
    // by a story that could not be read, or by no story).
    loss.characters_unresolved = text_chars - consumed_chars - excluded_chars;

    Ok((
        QuillParse {
            chars,
            fonts,
            stories: story_parses,
            table_cell_ends,
        },
        loss,
    ))
}

/// Lay the given stories (by index, in the order given) out
/// in the plain reading column starting at `first_page`:
/// the text-only placement. Returns the spans and how many
/// pages they occupy from `first_page`.
pub(crate) fn reading_column_spans(
    parse: &QuillParse,
    indices: &[usize],
    first_page: u32,
    loss: &mut LossReport,
) -> (Vec<Span>, u32) {
    let mut layout = Layout::new();
    layout.page = first_page;
    let mut spans: Vec<Span> = Vec::new();
    for &i in indices {
        let Some(story) = parse.stories.get(i) else {
            continue;
        };
        if !story.emitted {
            continue;
        }
        for para in &story.paragraphs {
            for run in &para.runs {
                emit_piece(
                    &mut layout,
                    &mut spans,
                    &parse.chars[run.start..run.end],
                    &run.props,
                    &parse.fonts,
                    loss,
                );
            }
            layout.paragraph_break(para.max_size);
        }
        layout.story_break();
    }
    let pages = layout.page - first_page + 1;
    (spans, pages)
}

/// The reading-column placement for an arbitrary list of
/// paragraphs (phase 2c uses it for the part of a story
/// that ran past the bottom of its page). Returns the
/// spans and the number of pages used.
pub(crate) fn reading_column_paras(
    parse: &QuillParse,
    paras: &[ParaParse],
    first_page: u32,
    loss: &mut LossReport,
) -> (Vec<Span>, u32) {
    let mut layout = Layout::new();
    layout.page = first_page;
    let mut spans: Vec<Span> = Vec::new();
    for para in paras {
        for run in &para.runs {
            emit_piece(
                &mut layout,
                &mut spans,
                &parse.chars[run.start..run.end],
                &run.props,
                &parse.fonts,
                loss,
            );
        }
        layout.paragraph_break(para.max_size);
    }
    let pages = layout.page - first_page + 1;
    (spans, pages)
}

/// The text-only dump: every emitted story in the reading
/// column on synthetic letter pages.
pub(crate) fn reading_column_dump(
    parse: &QuillParse,
    mut loss: LossReport,
    source_name: &str,
    source_bytes: usize,
) -> (ExtractionDump, LossReport) {
    let indices: Vec<usize> = (0..parse.stories.len()).collect();
    let (spans, npages) = reading_column_spans(parse, &indices, 0, &mut loss);
    let pages: Vec<PageInfo> = (0..npages)
        .map(|i| PageInfo {
            index: i,
            width_pt: PAGE_W,
            height_pt: PAGE_H,
            picture_only: false,
        })
        .collect();
    let dump = ExtractionDump {
        schema_version: DUMP_SCHEMA_VERSION,
        source_kind: SourceKind::Pub,
        source_name: source_name.to_string(),
        source_bytes,
        extractor: "openbulletin-pub-text 0.1".to_string(),
        pages,
        spans,
        images: Vec::new(),
    };
    (dump, loss)
}

/// Extract the words from Quill stream bytes: a text dump
/// (one span per character run, reading order, synthetic
/// letter pages) plus the loss report.
pub fn extract_from_quill(
    quill: &[u8],
    source_name: &str,
    source_bytes: usize,
) -> Result<(ExtractionDump, LossReport), String> {
    let (parse, loss) = parse_quill(quill)?;
    Ok(reading_column_dump(&parse, loss, source_name, source_bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Assemble a synthetic Quill stream: a directory block
    /// at 0x18, then the chunks in the order given. Returns
    /// the bytes and TEXT's offset within them (TEXT is
    /// always the first chunk in these fixtures, so FDPP /
    /// FDPC end offsets can be computed before assembly).
    fn assemble(chunks: Vec<(&str, u16, Vec<u8>)>) -> (Vec<u8>, usize) {
        let n = chunks.len();
        let text_off = 0x18 + 8 + 24 * n;
        let mut out = vec![0u8; 0x18];
        out.extend_from_slice(&[0, 0]);
        out.extend_from_slice(&(n as u16).to_le_bytes());
        out.extend_from_slice(&0xFFFF_FFFFu32.to_le_bytes());
        let mut at = text_off;
        let mut offsets = Vec::new();
        for (_, _, b) in &chunks {
            offsets.push(at);
            at += b.len();
        }
        for (i, (name, id, b)) in chunks.iter().enumerate() {
            out.extend_from_slice(&24u16.to_le_bytes());
            let mut nm = [b' '; 4];
            nm[..name.len()].copy_from_slice(name.as_bytes());
            out.extend_from_slice(&nm);
            out.extend_from_slice(&id.to_le_bytes());
            out.extend_from_slice(&0x0100_0000u32.to_le_bytes());
            out.extend_from_slice(&nm);
            out.extend_from_slice(&(offsets[i] as u32).to_le_bytes());
            out.extend_from_slice(&(b.len() as u32).to_le_bytes());
        }
        for (_, _, b) in &chunks {
            out.extend_from_slice(b);
        }
        (out, text_off)
    }

    fn text_payload(s: &str) -> Vec<u8> {
        s.encode_utf16().flat_map(|u| u.to_le_bytes()).collect()
    }

    fn strs_payload(lens: &[u32]) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(&(lens.len() as u32).to_le_bytes());
        v.extend_from_slice(&8u32.to_le_bytes());
        v.extend_from_slice(&255u32.to_le_bytes());
        for l in lens {
            v.extend_from_slice(&l.to_le_bytes());
        }
        v
    }

    fn syid_payload(ids: &[u32]) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(&3u32.to_le_bytes());
        v.extend_from_slice(&(ids.len() as u32).to_le_bytes());
        for id in ids {
            v.extend_from_slice(&id.to_le_bytes());
        }
        v
    }

    fn font_payload(names: &[&str]) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(&0u32.to_le_bytes());
        v.extend_from_slice(&(names.len() as u32).to_le_bytes());
        v.extend_from_slice(&[0u8; 12]);
        v.extend_from_slice(&vec![0u8; 4 * names.len()]);
        for name in names {
            let units: Vec<u16> = name.encode_utf16().collect();
            v.extend_from_slice(&(units.len() as u16).to_le_bytes());
            for u in units {
                v.extend_from_slice(&u.to_le_bytes());
            }
            v.extend_from_slice(&0u32.to_le_bytes());
        }
        v
    }

    /// The stub STSH a real file carries first: no elements.
    fn stsh_stub() -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(&0u32.to_le_bytes());
        v.extend_from_slice(&0u32.to_le_bytes());
        v.extend_from_slice(&[0u8; 12]);
        v
    }

    /// The real stylesheet: character style 0 is 10pt in
    /// font 0; paragraph style 0 names character style 0.
    fn stsh_real() -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(&420u32.to_le_bytes());
        v.extend_from_slice(&2u32.to_le_bytes());
        v.extend_from_slice(&[0u8; 12]);
        v.extend_from_slice(&8u32.to_le_bytes()); // element 0 at 20 + 8
        v.extend_from_slice(&36u32.to_le_bytes()); // element 1 at 20 + 36
                                                   // Element 0 (character style): U16 skip, record.
        v.extend_from_slice(&0u16.to_le_bytes());
        let mut blocks = Vec::new();
        blocks.extend_from_slice(&[0x0C, 0x22]);
        blocks.extend_from_slice(&127_000u32.to_le_bytes()); // 10pt in EMU
        blocks.extend_from_slice(&[0x24, 0x8A]);
        blocks.extend_from_slice(&14u32.to_le_bytes()); // container total
        blocks.extend_from_slice(&[0x00, 0x88]);
        blocks.extend_from_slice(&8u32.to_le_bytes());
        blocks.extend_from_slice(&0x1800u32.to_le_bytes()); // font index 0
        v.extend_from_slice(&((4 + blocks.len()) as u32).to_le_bytes());
        v.extend_from_slice(&blocks);
        // Element 1 (paragraph style): default char style 0.
        v.extend_from_slice(&0u16.to_le_bytes());
        v.extend_from_slice(&8u32.to_le_bytes());
        v.extend_from_slice(&[0x19, 0x12, 0, 0]);
        v
    }

    fn empty_record() -> Vec<u8> {
        vec![4, 0, 0, 0]
    }

    fn bold_record() -> Vec<u8> {
        let mut v = Vec::new();
        let mut blocks = Vec::new();
        blocks.extend_from_slice(&[0x02, 0x0A]); // bold flag
        blocks.extend_from_slice(&[0x0C, 0x22]);
        blocks.extend_from_slice(&254_000u32.to_le_bytes()); // 20pt in EMU
        v.extend_from_slice(&((4 + blocks.len()) as u32).to_le_bytes());
        v.extend_from_slice(&blocks);
        v
    }

    /// An FDPP/FDPC chunk: entries are (end in characters,
    /// style record bytes). End offsets in the chunk are
    /// absolute byte offsets within the Quill stream.
    fn fdp_payload(entries: &[(usize, Vec<u8>)], text_off: usize) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(&(entries.len() as u16).to_le_bytes());
        v.extend_from_slice(&[0u8; 6]);
        for (end, _) in entries {
            v.extend_from_slice(&((text_off + 2 * end) as u32).to_le_bytes());
        }
        let records_at = 8 + 6 * entries.len();
        let mut at = records_at;
        let mut offsets = Vec::new();
        for (_, rec) in entries {
            offsets.push(at);
            at += rec.len();
        }
        for off in offsets {
            v.extend_from_slice(&(off as u16).to_le_bytes());
        }
        for (_, rec) in entries {
            v.extend_from_slice(rec);
        }
        v
    }

    /// The standard fixture: one story, two paragraphs, the
    /// first holding a bold run in its middle.
    /// Text: "Hello bold world\rSecond para\r" (29 chars).
    fn standard_quill(strs: Vec<u8>, syid: Vec<u8>) -> Vec<u8> {
        let text = "Hello bold world\rSecond para\r";
        let n = 8; // TEXT STSH STSH FDPP FDPC SYID FONT STRS
        let text_off = 0x18 + 8 + 24 * n;
        let fdpp = fdp_payload(&[(17, empty_record()), (29, empty_record())], text_off);
        let fdpc = fdp_payload(
            &[
                (6, empty_record()),
                (10, bold_record()),
                (17, empty_record()),
                (29, empty_record()),
            ],
            text_off,
        );
        let (bytes, _) = assemble(vec![
            ("TEXT", 0, text_payload(text)),
            ("STSH", 0, stsh_stub()),
            ("STSH", 1, stsh_real()),
            ("FDPP", 0, fdpp),
            ("FDPC", 0, fdpc),
            ("SYID", 0, syid),
            ("FONT", 0, font_payload(&["Times New Roman", "Arial"])),
            ("STRS", 0, strs),
        ]);
        bytes
    }

    #[test]
    fn runs_keep_their_own_formatting() {
        let quill = standard_quill(strs_payload(&[29]), syid_payload(&[7]));
        let (dump, loss) = extract_from_quill(&quill, "test.pub", 1000).expect("extract");
        let texts: Vec<&str> = dump.spans.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(texts, ["Hello ", "bold", " world", "Second para"]);
        let bolds: Vec<bool> = dump.spans.iter().map(|s| s.bold.unwrap()).collect();
        assert_eq!(bolds, [false, true, false, false]);
        // The plain runs take size and font from the STSH
        // default (10pt, font 0); the bold run states its own
        // size (20pt) and inherits the font.
        assert_eq!(dump.spans[0].font_size, Some(10.0));
        assert_eq!(dump.spans[0].font.as_deref(), Some("Times New Roman"));
        assert_eq!(dump.spans[1].font_size, Some(20.0));
        assert_eq!(dump.spans[1].font.as_deref(), Some("Times New Roman"));
        assert!(dump.spans.iter().all(|s| s.trust == SpanTrust::NeedsReview));
        // The second paragraph starts lower on the page.
        assert!(dump.spans[3].y.unwrap() > dump.spans[0].y.unwrap());
        assert_eq!(dump.pages.len(), 1);
        assert_eq!(dump.source_kind, SourceKind::Pub);
        // Loss books balance.
        assert_eq!(loss.stories_total, 1);
        assert_eq!(loss.stories_emitted, 1);
        assert_eq!(loss.stories_excluded, 0);
        assert_eq!(loss.stories_unresolved, 0);
        assert_eq!(loss.stories_with_unresolved_ids, 0);
        assert_eq!(loss.runs_emitted, 4);
        assert_eq!(loss.runs_styled_by_default, 3);
        assert_eq!(loss.characters_emitted, 27);
        assert_eq!(loss.characters_unresolved, 0);
    }

    #[test]
    fn unresolvable_story_is_counted_not_dropped_quietly() {
        // Story 2 claims 50 characters the text does not hold.
        let quill = standard_quill(strs_payload(&[29, 50]), syid_payload(&[7, 8]));
        let (dump, loss) = extract_from_quill(&quill, "test.pub", 1000).expect("extract");
        assert_eq!(loss.stories_total, 2);
        assert_eq!(loss.stories_emitted, 1);
        assert_eq!(loss.stories_unresolved, 1);
        assert!(
            loss.unresolved_reasons
                .iter()
                .any(|r| r.contains("story 2")),
            "{:?}",
            loss.unresolved_reasons
        );
        // Only story 1's words are in the dump.
        assert_eq!(dump.spans.len(), 4);
    }

    #[test]
    fn shared_syid_is_reported_but_words_still_come_over() {
        let text = "aa\rbb\r";
        let (quill, _) = assemble(vec![
            ("TEXT", 0, text_payload(text)),
            ("SYID", 0, syid_payload(&[5, 5])),
            ("STRS", 0, strs_payload(&[3, 3])),
        ]);
        let (dump, loss) = extract_from_quill(&quill, "test.pub", 100).expect("extract");
        assert_eq!(loss.stories_emitted, 2);
        assert_eq!(loss.stories_with_unresolved_ids, 2);
        let texts: Vec<&str> = dump.spans.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(texts, ["aa", "bb"]);
    }

    #[test]
    fn control_characters_never_reach_spans() {
        let text = "one\ttwo\u{b}three\r";
        let (quill, _) = assemble(vec![
            ("TEXT", 0, text_payload(text)),
            ("SYID", 0, syid_payload(&[1])),
            ("STRS", 0, strs_payload(&[14])),
        ]);
        let (dump, loss) = extract_from_quill(&quill, "test.pub", 100).expect("extract");
        let texts: Vec<&str> = dump.spans.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(texts, ["one two", "three"]);
        assert!(dump
            .spans
            .iter()
            .all(|s| !s.text.chars().any(|c| c.is_control())));
        // The tab and the vertical tab were mapped.
        assert_eq!(loss.control_chars_mapped, 2);
        assert_eq!(loss.stories_emitted, 1);
    }

    #[test]
    fn garbage_quill_is_an_error_not_a_panic() {
        assert!(extract_from_quill(b"not a quill stream", "x.pub", 10).is_err());
        assert!(extract_from_quill(&[], "x.pub", 0).is_err());
    }
}
