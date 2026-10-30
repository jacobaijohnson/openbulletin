//! Extraction dump: the shared form the importers produce.
//!
//! A bulletin file goes in and a versioned JSON dump (schema v2)
//! comes out: pages, their sizes, and the text spans found on them,
//! each marked with how much it can be trusted, and carrying
//! position, size, and font facts where the source states them.
//! The Template Analyzer reads dumps, never raw foreign files, so
//! the risky parsing of stranger files lives in exactly one place.
//!
//! PDFs are extracted through the out-of-process PDFium helper
//! (see `helper`). The in-process importers that exist today are
//! IDML, Scribus, and the native Publisher reader; .pub files also
//! come through the LibreOffice bridge (`pub_bridge`), which
//! converts to PDF and then takes the PDF path, saved .obd files
//! turn back into dumps in `learn`, and HTML import builds its
//! draft directly (`html_import`). The lopdf reader that
//! scaffolded the first dump (`extract_pdf`) remains for the
//! crate's examples and tests only.

pub mod analyzer;
pub mod cfb;
pub mod contribution;
pub mod draft;
pub mod helper;
pub mod idml;
pub mod pub_bridge;
pub mod pub_layout;
pub mod pub_text;
// The process runner lives in its own crate (ob-runner,
// health audit HF-13); it is re-exported here under its
// old name so every existing `ob_extract::runner::` path
// in the workspace keeps resolving unchanged.
pub use ob_runner as runner;

use serde::{Deserialize, Serialize};

/// The open-door size limit: no file handed to an importer
/// may be larger than this, however it arrives. The CFB
/// reader has long declared 512 MB implausible for a
/// compound file (its own MAX_INPUT); the health audit
/// (HF-2) made the same judgment at every open door, so a
/// hostile multi-gigabyte file is refused from its
/// filesystem size before a single byte is read, and the
/// read that follows is counted in actual bytes besides.
pub const MAX_OPEN_DOOR_BYTES: u64 = 512 * 1024 * 1024;

/// How an open door can refuse (HF-12 first slice,
/// round 44 R44-I): the file is past the size limit, or
/// the file cannot be read at all. The typed core of the
/// door surface; its `Display` renders byte-for-byte the
/// plain words the doors have always spoken, so every
/// boundary that still speaks `String` hears the same
/// sentences it heard before the slice.
#[derive(Debug)]
pub enum OpenDoorError {
    /// The file is bigger than the open-door limit.
    TooLarge,
    /// The file could not be read; the reader's own
    /// error is carried whole.
    Unreadable(std::io::Error),
}

impl std::fmt::Display for OpenDoorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OpenDoorError::TooLarge => write!(
                f,
                "that file is too large to bring in: OpenBulletin can open files up to \
                 512 megabytes, and this one is bigger than that"
            ),
            OpenDoorError::Unreadable(e) => write!(f, "cannot read that file: {e}"),
        }
    }
}

impl std::error::Error for OpenDoorError {}

impl From<OpenDoorError> for String {
    fn from(err: OpenDoorError) -> String {
        err.to_string()
    }
}

/// How reading a dump back from JSON can refuse
/// (HF-12 second slice, round 46 R46-G, from_json
/// half): the text is not a dump at all, the dump
/// claims a format newer than this copy knows, or
/// the dump claims a format no copy has ever known.
/// The typed core of the from_json surface; its
/// `Display` renders byte-for-byte the plain words
/// from_json has always spoken, so every boundary
/// that still speaks `String` hears the same
/// sentences it heard before the slice.
#[derive(Debug)]
pub enum FromJsonError {
    /// The text does not parse as a dump; the
    /// parser's own error is carried whole.
    Unreadable(serde_json::Error),
    /// The dump's schema version is newer than
    /// `DUMP_SCHEMA_VERSION`.
    TooNew(u32),
    /// The dump's schema version is below 1 (no
    /// dump format has ever carried it).
    Unsupported(u32),
}

impl std::fmt::Display for FromJsonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FromJsonError::Unreadable(e) => {
                write!(
                    f,
                    "the reading helper answered in a way we cannot read: {e}"
                )
            }
            FromJsonError::TooNew(v) => write!(
                f,
                "the reading helper answered in a newer format (version {v}) than this \
                 copy of OpenBulletin understands (version {DUMP_SCHEMA_VERSION} at most)"
            ),
            FromJsonError::Unsupported(v) => write!(
                f,
                "the reading helper answered in a format this copy of OpenBulletin \
                 does not understand (version {v}); it reads versions 1 and \
                 {DUMP_SCHEMA_VERSION}"
            ),
        }
    }
}

impl std::error::Error for FromJsonError {}

impl From<FromJsonError> for String {
    fn from(err: FromJsonError) -> String {
        err.to_string()
    }
}

/// The metadata half of the door gate: refuse from the
/// filesystem's stated length, before any read. A file
/// whose metadata cannot be read passes here; the read
/// itself will speak for that case, and the counted read
/// below still bounds what arrives.
pub fn check_open_door_size(path: &std::path::Path) -> Result<(), OpenDoorError> {
    if let Ok(meta) = std::fs::metadata(path) {
        if meta.len() > MAX_OPEN_DOOR_BYTES {
            return Err(OpenDoorError::TooLarge);
        }
    }
    Ok(())
}

/// Read a whole file at an open door: the metadata gate
/// first, then a counted read (the limit plus one byte),
/// so a file that grows between the two, or whose
/// metadata understated it, is refused on the bytes that
/// actually arrived.
pub fn read_open_door_file(path: &std::path::Path) -> Result<Vec<u8>, OpenDoorError> {
    use std::io::Read as _;
    check_open_door_size(path)?;
    let mut file = std::fs::File::open(path).map_err(OpenDoorError::Unreadable)?;
    let mut bytes = Vec::new();
    file.by_ref()
        .take(MAX_OPEN_DOOR_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(OpenDoorError::Unreadable)?;
    if bytes.len() as u64 > MAX_OPEN_DOOR_BYTES {
        return Err(OpenDoorError::TooLarge);
    }
    Ok(bytes)
}

/// The same door for a text file: gated, counted in
/// actual bytes, and read as text.
pub fn read_open_door_text(path: &std::path::Path) -> Result<String, OpenDoorError> {
    use std::io::Read as _;
    check_open_door_size(path)?;
    let mut file = std::fs::File::open(path).map_err(OpenDoorError::Unreadable)?;
    let mut text = String::new();
    file.by_ref()
        .take(MAX_OPEN_DOOR_BYTES + 1)
        .read_to_string(&mut text)
        .map_err(OpenDoorError::Unreadable)?;
    if text.len() as u64 > MAX_OPEN_DOOR_BYTES {
        return Err(OpenDoorError::TooLarge);
    }
    Ok(text)
}

/// Bump when the dump shape changes incompatibly.
/// v2: spans carry alignment, italic, and fill color, and
/// the dump has an image channel (bytes in sidecar files,
/// see DumpImage). Readers accept v1 dumps unchanged: the
/// new span fields default (no alignment known, not italic,
/// black) and the image list defaults empty.
pub const DUMP_SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    Pdf,
    Pub,
    Idml,
    Obd,
    Sla,
}

/// How a span was recovered. The analyzer trusts these differently.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpanTrust {
    /// Decoded straight from the file's text operations.
    RawText,
    /// The page could not be read as text (scanned image, outlined
    /// fonts). The analyzer treats such pages as pictures to trace
    /// over, never as words it knows.
    PictureOnly,
    /// Decoded as text, but the recovery cannot vouch for the
    /// words' look or placement (the native .pub text rescue:
    /// words in reading order, without their places on the
    /// page). The draft badges every block built from these
    /// Check this.
    NeedsReview,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Span {
    pub page: u32,
    pub text: String,
    pub trust: SpanTrust,
    /// Position and size in points, when the extractor can supply them
    /// (the PDFium helper does; this scaffold does not).
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub w: Option<f32>,
    pub h: Option<f32>,
    pub font_size: Option<f32>,
    pub bold: Option<bool>,
    /// Font name as the source file names it (e.g. "ABCDEF+Calibri"
    /// for a subset embedded font). Importers report names; the font
    /// files themselves are never extracted (docs/FONTS.md).
    #[serde(default)]
    pub font: Option<String>,
    /// The line's alignment, when the extractor knows or can
    /// infer it (the PDFium helper infers it from line
    /// geometry against its block; see infer_alignments).
    /// None means nobody knows, and the draft builder falls
    /// back to Left without a badge change, as v1 drafts did.
    #[serde(default)]
    pub align: Option<ob_model::Align>,
    /// Italic face, from the source's own signals (PDFium
    /// font flags, IDML FontStyle, a Scribus face name), no
    /// longer smuggled through the font name. Defaults false
    /// for v1 dumps, whose names the draft still sniffs as a
    /// fallback.
    #[serde(default)]
    pub italic: bool,
    /// Text fill color as RGB. Defaults to black, which is
    /// also what the source usually says; the draft builder
    /// takes the block's consensus.
    #[serde(default)]
    pub color: [u8; 3],
    /// Inline formatting inside the span (G1b), when the
    /// source carries it run by run (IDML CharacterStyleRange
    /// today). Invariant: concatenating the run texts gives
    /// exactly `text`. Empty means the span states one face
    /// for all its words (its own bold/italic fields).
    #[serde(default)]
    pub runs: Vec<SpanRun>,
    /// Threading (T1): the .pub door stamps every span of a
    /// box that shares its text with other boxes with the
    /// thread's name; the draft builder then emits ONE story
    /// for all the thread's boxes instead of a copy per box.
    /// Absent on every other door and on older dumps.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thread: Option<String>,
}

/// One formatted piece of a span (G1b): text plus the face
/// switches the source states for it (None = the source
/// states nothing; the draft builder reads those as the
/// span's own face).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpanRun {
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bold: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub italic: Option<bool>,
}

/// One picture found in the source, carried beside the dump:
/// the JSON names the file, the bytes live in that file next
/// to wherever the dump was written (the helper's output
/// folder). Geometry is in points, top-left origin, the
/// dump's convention for spans.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DumpImage {
    pub id: String,
    pub page: u32,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    /// File name (not a path) holding the PNG bytes.
    pub file: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageInfo {
    pub index: u32,
    pub width_pt: f32,
    pub height_pt: f32,
    /// A page the extractor could not read as text at all.
    pub picture_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionDump {
    pub schema_version: u32,
    pub source_kind: SourceKind,
    pub source_name: String,
    pub source_bytes: usize,
    pub extractor: String,
    pub pages: Vec<PageInfo>,
    pub spans: Vec<Span>,
    /// Pictures the extractor pulled out of the source
    /// (schema v2). Absent in v1 dumps.
    #[serde(default)]
    pub images: Vec<DumpImage>,
    /// Losses the extractor counted while reading the
    /// source, each one sentence in plain words, for
    /// the draft builder to surface in its review
    /// (audit round 2, RT-5: words that vanish must be
    /// counted where the user can see them). Absent in
    /// dumps from before the field existed.
    #[serde(default)]
    pub loss_notes: Vec<String>,
}

/// The loss sentence for text spans addressed to pages
/// a dump does not list (round 2, R2-8), worded once so
/// every door says it the same way.
pub(crate) fn unlisted_page_spans_note(dropped: usize) -> String {
    if dropped == 1 {
        "1 piece of text pointed at a page the reading did not list, so it was left out."
            .to_string()
    } else {
        format!(
            "{dropped} pieces of text pointed at pages the reading did not list, \
             so they were left out."
        )
    }
}

impl ExtractionDump {
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|e| e.to_string())
    }

    /// Drop the spans addressed to pages this dump does
    /// not list, counting them into `loss_notes` (round
    /// 2, R2-8). The draft builder walks the listed pages,
    /// so such spans would vanish without a word; here
    /// they are removed where the loss is counted, in the
    /// channel the draft review already reads aloud.
    /// Picture-only markers carry no words and are not
    /// counted as text losses.
    fn note_unlisted_page_spans(&mut self) {
        let listed: std::collections::HashSet<u32> = self.pages.iter().map(|p| p.index).collect();
        let before = self.spans.len();
        self.spans
            .retain(|s| s.trust == SpanTrust::PictureOnly || listed.contains(&s.page));
        let dropped = before - self.spans.len();
        if dropped > 0 {
            self.loss_notes.push(unlisted_page_spans_note(dropped));
        }
    }

    pub fn from_json(s: &str) -> Result<Self, FromJsonError> {
        let mut dump: Self = serde_json::from_str(s).map_err(FromJsonError::Unreadable)?;
        // The version is checked, not just carried
        // (audit M10, 2026-10-16): a helper answering
        // in a format from the future must be refused
        // in words, never read as if it were today's.
        // Versions 1 and 2 are understood; 1 parses
        // with the v2 fields defaulted.
        match dump.schema_version {
            1..=DUMP_SCHEMA_VERSION => {
                // Spans addressed to pages the dump does
                // not list are counted out here, where
                // every helper answer passes (round 2,
                // R2-8).
                dump.note_unlisted_page_spans();
                Ok(dump)
            }
            v if v > DUMP_SCHEMA_VERSION => Err(FromJsonError::TooNew(v)),
            v => Err(FromJsonError::Unsupported(v)),
        }
    }
}

/// Extract a dump from PDF bytes using lopdf (structural pass).
///
/// This is the in-process lopdf path, used by the crate's
/// probes (`examples/probe.rs`) and tests. It is NOT the
/// production PDF door: production reads PDFs through the
/// PDFium helper (`helper::dump_pdf`), which runs the
/// native parser in its own process under the runner's
/// deadline and output caps. Because this path parses in
/// process, it refuses a byte slice longer than the
/// standing open-door limit at entry, in plain words,
/// before lopdf sees a byte.
pub fn extract_pdf(bytes: &[u8], source_name: &str) -> Result<ExtractionDump, String> {
    if bytes.len() as u64 > MAX_OPEN_DOOR_BYTES {
        // R44-I: this entry site rides the door's typed
        // TooLarge kind through the From boundary;
        // extract_pdf's own signature stays on String
        // for this slice (the narrower diff).
        return Err(String::from(OpenDoorError::TooLarge));
    }
    let doc = lopdf::Document::load_mem(bytes).map_err(|e| format!("not a readable PDF: {e}"))?;
    let mut pages = Vec::new();
    let mut spans = Vec::new();
    // lopdf's page map is page number (1-based) -> object id.
    for (page_num, page_id) in doc.get_pages() {
        let idx = page_num - 1;
        let (w, h) = page_size(&doc, &page_id).unwrap_or((0.0, 0.0));
        let text = doc
            .extract_text(&[page_num])
            .unwrap_or_default()
            .trim()
            .to_string();
        let picture_only = text.is_empty();
        pages.push(PageInfo {
            index: idx,
            width_pt: w,
            height_pt: h,
            picture_only,
        });
        if picture_only {
            spans.push(Span {
                page: idx,
                text: String::new(),
                trust: SpanTrust::PictureOnly,
                x: None,
                y: None,
                w: None,
                h: None,
                font_size: None,
                bold: None,
                font: None,
                align: None,
                italic: false,
                color: [0, 0, 0],
                runs: Vec::new(),
                thread: None,
            });
        } else {
            for line in text.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                spans.push(Span {
                    page: idx,
                    text: line.to_string(),
                    trust: SpanTrust::RawText,
                    x: None,
                    y: None,
                    w: None,
                    h: None,
                    font_size: None,
                    bold: None,
                    font: None,
                    align: None,
                    italic: false,
                    color: [0, 0, 0],
                    runs: Vec::new(),
                    thread: None,
                });
            }
        }
    }
    Ok(ExtractionDump {
        schema_version: DUMP_SCHEMA_VERSION,
        source_kind: SourceKind::Pdf,
        source_name: source_name.to_string(),
        source_bytes: bytes.len(),
        extractor: "ob-extract lopdf scaffold 0.1".to_string(),
        pages,
        spans,
        images: Vec::new(),
        loss_notes: Vec::new(),
    })
}

/// Infer line alignment from geometry, for dumps whose
/// extractor cannot state it (the PDFium helper's text
/// segments carry no justification). Lines are grouped and
/// blocked with the same tolerances the draft builder uses,
/// so the two never disagree about what a line is. For each
/// line, the gaps between the line's edges and its block's
/// edges decide: both gaps real and near-equal is Center, a
/// flush right edge with a real left gap is Right, anything
/// else is Left. A line that IS its block's widest line says
/// nothing (both gaps are zero by construction) and keeps
/// the alignment it had; spans that already state an
/// alignment are never overridden.
pub fn infer_alignments(dump: &mut ExtractionDump) {
    use ob_model::Align;
    // Page index -> span indices, in reading order.
    let mut order: Vec<usize> = (0..dump.spans.len())
        .filter(|&i| dump.spans[i].x.is_some() && dump.spans[i].y.is_some())
        .collect();
    order.sort_by(|&a, &b| {
        let (sa, sb) = (&dump.spans[a], &dump.spans[b]);
        sa.page
            .cmp(&sb.page)
            .then(sa.y.unwrap_or(0.0).total_cmp(&sb.y.unwrap_or(0.0)))
            .then(sa.x.unwrap_or(0.0).total_cmp(&sb.x.unwrap_or(0.0)))
    });
    // Group into lines: (page, line's span indices).
    let mut lines: Vec<Vec<usize>> = Vec::new();
    for &i in &order {
        let sp = &dump.spans[i];
        let y = sp.y.unwrap_or(0.0);
        let size = sp.font_size.unwrap_or(11.0);
        let joins = lines.last().and_then(|l: &Vec<usize>| {
            let first = &dump.spans[l[0]];
            if first.page != sp.page {
                return None;
            }
            let tol = (first.font_size.unwrap_or(11.0).max(size) * 0.4).max(2.0);
            Some((y - first.y.unwrap_or(0.0)).abs() <= tol)
        });
        if joins == Some(true) {
            lines.last_mut().expect("line").push(i);
        } else {
            lines.push(vec![i]);
        }
    }
    // Group lines into blocks: a vertical gap larger than
    // three quarters of the line above starts a new block.
    let mut blocks: Vec<Vec<usize>> = Vec::new(); // line indices
    for (li, line) in lines.iter().enumerate() {
        let split = blocks.last().map(|b: &Vec<usize>| {
            let prev = &lines[*b.last().expect("block line")];
            let (py, ph) = line_geom(&dump.spans, prev);
            let (ly, _) = line_geom(&dump.spans, line);
            ly - (py + ph) > ph * 0.75
        });
        if split == Some(true) || blocks.is_empty() {
            blocks.push(vec![li]);
        } else {
            blocks.last_mut().expect("block").push(li);
        }
    }
    // Decide per line against its block's edges.
    let mut decided: Vec<(usize, Align)> = Vec::new();
    for block in &blocks {
        let mut bx0 = f32::MAX;
        let mut bx1 = f32::MIN;
        for &li in block {
            let (x0, x1) = line_edges(&dump.spans, &lines[li]);
            bx0 = bx0.min(x0);
            bx1 = bx1.max(x1);
        }
        for &li in block {
            let (x0, x1) = line_edges(&dump.spans, &lines[li]);
            let left_gap = x0 - bx0;
            let right_gap = bx1 - x1;
            // Real: wider than a rounding error (2pt) and
            // wider than a hair of the block's own width.
            let real = |g: f32| g > 2.0 && g > (bx1 - bx0) * 0.02;
            let align = if real(left_gap) && real(right_gap) {
                if (left_gap - right_gap).abs() <= (left_gap + right_gap) * 0.25 {
                    Align::Center
                } else {
                    Align::Left
                }
            } else if real(left_gap) && right_gap <= 2.0 {
                Align::Right
            } else {
                Align::Left
            };
            for &si in &lines[li] {
                decided.push((si, align));
            }
        }
    }
    for (si, align) in decided {
        if dump.spans[si].align.is_none() {
            dump.spans[si].align = Some(align);
        }
    }
}

/// A line's (y, height) from its spans.
fn line_geom(spans: &[Span], line: &[usize]) -> (f32, f32) {
    let y = line
        .iter()
        .map(|&i| spans[i].y.unwrap_or(0.0))
        .fold(f32::MAX, f32::min);
    let h = line
        .iter()
        .map(|&i| {
            spans[i]
                .h
                .unwrap_or(spans[i].font_size.unwrap_or(11.0) * 1.2)
        })
        .fold(0.0, f32::max);
    (y, h)
}

/// A line's (left, right) edges from its spans.
fn line_edges(spans: &[Span], line: &[usize]) -> (f32, f32) {
    let x0 = line
        .iter()
        .map(|&i| spans[i].x.unwrap_or(0.0))
        .fold(f32::MAX, f32::min);
    let x1 = line
        .iter()
        .map(|&i| spans[i].x.unwrap_or(0.0) + spans[i].w.unwrap_or(0.0))
        .fold(f32::MIN, f32::max);
    (x0, x1)
}

fn page_size(doc: &lopdf::Document, page_id: &lopdf::ObjectId) -> Option<(f32, f32)> {
    let page = doc.get_object(*page_id).ok()?.as_dict().ok()?;
    let mb = page.get(b"MediaBox").ok()?.as_array().ok()?;
    let nums: Vec<f32> = mb
        .iter()
        .filter_map(|o| {
            o.as_float()
                .ok()
                .or_else(|| o.as_i64().ok().map(|i| i as f32))
        })
        .collect();
    if nums.len() == 4 {
        Some((nums[2] - nums[0], nums[3] - nums[1]))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A sparse file whose stated length is one byte past
    /// the door limit: set_len allocates nothing, so the
    /// pin is instant, and the refusal must come from the
    /// metadata gate alone (reading it whole would also
    /// "work" — the point is that the door never tries).
    fn sparse_oversize_file(tag: &str) -> std::path::PathBuf {
        let path =
            std::env::temp_dir().join(format!("ob-door-gate-{tag}-{}.bin", std::process::id()));
        let file = std::fs::File::create(&path).expect("create sparse file");
        file.set_len(MAX_OPEN_DOOR_BYTES + 1)
            .expect("state an oversize length");
        drop(file);
        path
    }

    #[test]
    fn open_door_gate_refuses_an_oversize_file_before_any_read() {
        // R44-I restatement (flagged): this pin asserted
        // on the door functions' error strings; the doors
        // now return the typed OpenDoorError, so it
        // asserts the TooLarge kind and the same words
        // through the Display rendering. No other part of
        // the pin moved.
        let path = sparse_oversize_file("gate");
        let err = check_open_door_size(&path).expect_err("the gate refuses");
        assert!(matches!(err, OpenDoorError::TooLarge), "{err}");
        let words = err.to_string();
        assert!(words.contains("too large"), "{words}");
        assert!(words.contains("512 megabytes"), "{words}");
        let err = read_open_door_file(&path).expect_err("the counted read refuses at the gate");
        assert!(matches!(err, OpenDoorError::TooLarge), "{err}");
        let words = err.to_string();
        assert!(words.contains("too large"), "{words}");
        let err = read_open_door_text(&path).expect_err("the text door refuses at the gate");
        assert!(matches!(err, OpenDoorError::TooLarge), "{err}");
        let words = err.to_string();
        assert!(words.contains("too large"), "{words}");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn r44i_open_door_error_renders_the_pre_slice_words_byte_for_byte() {
        // (a) Both kinds render byte-for-byte the
        // sentences the base tree's code produced: the
        // TooLarge literal below is taken verbatim from
        // the base tree's open_door_too_large(), and the
        // Unreadable shape is its `cannot read that file:
        // {e}` format with a fixed reader error.
        assert_eq!(
            OpenDoorError::TooLarge.to_string(),
            "that file is too large to bring in: OpenBulletin can open files up to \
             512 megabytes, and this one is bigger than that"
        );
        let err = OpenDoorError::Unreadable(std::io::Error::other("disk hiccup"));
        assert_eq!(err.to_string(), "cannot read that file: disk hiccup");
    }

    #[test]
    fn r44i_door_functions_return_the_typed_kinds_at_their_gates() {
        // (b) TooLarge at the size gate, Unreadable on a
        // missing file. This pin names the typed surface,
        // so it cannot compile on the base tree.
        let path = sparse_oversize_file("r44i-kinds");
        let err = check_open_door_size(&path).expect_err("the gate refuses");
        assert!(matches!(err, OpenDoorError::TooLarge), "{err}");
        let err = read_open_door_file(&path).expect_err("the counted read refuses at the gate");
        assert!(matches!(err, OpenDoorError::TooLarge), "{err}");
        let err = read_open_door_text(&path).expect_err("the text door refuses at the gate");
        assert!(matches!(err, OpenDoorError::TooLarge), "{err}");
        let _ = std::fs::remove_file(&path);
        let missing =
            std::env::temp_dir().join(format!("ob-door-r44i-missing-{}.bin", std::process::id()));
        let err = read_open_door_file(&missing).expect_err("a missing file cannot be read");
        assert!(matches!(err, OpenDoorError::Unreadable(_)), "{err}");
        let err = read_open_door_text(&missing).expect_err("a missing file cannot be read");
        assert!(matches!(err, OpenDoorError::Unreadable(_)), "{err}");
    }

    #[test]
    fn r44i_from_open_door_error_for_string_renders_the_same_words() {
        // (c) The boundary conversion the standing String
        // channels rely on renders the same words as the
        // Display. This pin names the typed surface, so
        // it cannot compile on the base tree.
        let words = String::from(OpenDoorError::TooLarge);
        assert_eq!(
            words,
            "that file is too large to bring in: OpenBulletin can open files up to \
             512 megabytes, and this one is bigger than that"
        );
        let words = String::from(OpenDoorError::Unreadable(std::io::Error::other(
            "disk hiccup",
        )));
        assert_eq!(words, "cannot read that file: disk hiccup");
    }

    #[test]
    fn open_door_reads_an_ordinary_file_through() {
        let path =
            std::env::temp_dir().join(format!("ob-door-gate-small-{}.txt", std::process::id()));
        std::fs::write(&path, b"a small bulletin").expect("write small file");
        assert!(check_open_door_size(&path).is_ok());
        assert_eq!(
            read_open_door_file(&path).expect("read small file"),
            b"a small bulletin"
        );
        assert_eq!(
            read_open_door_text(&path).expect("read small text"),
            "a small bulletin"
        );
        let _ = std::fs::remove_file(&path);
    }

    fn sample_pdf() -> Vec<u8> {
        let start = chrono::NaiveDate::from_ymd_opt(2026, 10, 18).unwrap();
        let doc = ob_layout::build_template(
            ob_layout::Preset::LetterBifold,
            ob_model::ChurchProfile {
                name: "Grace Lutheran Church".into(),
                pastor: "Pastor Lee".into(),
                ..Default::default()
            },
            start,
            "Twentieth Sunday after Pentecost",
        );
        ob_pdf::export_reader_pdf(&doc, ob_pdf::ExportPurpose::ForEmailOrWeb)
    }

    #[test]
    fn dump_of_own_reader_pdf() {
        let pdf = sample_pdf();
        let dump = extract_pdf(&pdf, "sample.pdf").expect("extract");
        assert_eq!(dump.schema_version, DUMP_SCHEMA_VERSION);
        assert_eq!(dump.pages.len(), 4, "four panels as reader pages");
        // Panels are 5.5 x 8.5 in = 396 x 612 pt.
        assert!((dump.pages[0].width_pt - 396.0).abs() < 0.5);
        assert!((dump.pages[0].height_pt - 612.0).abs() < 0.5);
        assert!(
            dump.spans.iter().any(|s| s.text.contains("Grace Lutheran")),
            "cover text recovered: {:?}",
            dump.spans.iter().map(|s| &s.text).collect::<Vec<_>>()
        );
        // JSON round trip keeps the schema honest.
        let json = dump.to_json().expect("json");
        let back = ExtractionDump::from_json(&json).expect("parse");
        assert_eq!(back.pages.len(), 4);
    }

    #[test]
    fn spans_for_unlisted_pages_are_counted_not_vanished() {
        // Round 2, R2-8: words addressed to a page the
        // dump does not list used to pass the door and
        // then vanish in the draft builder, which walks
        // only the listed pages. Reading the dump now
        // drops them here and counts them into the loss
        // notes the review reads aloud.
        let span = |page: u32, text: &str, trust: SpanTrust| Span {
            page,
            text: text.into(),
            trust,
            x: None,
            y: None,
            w: None,
            h: None,
            font_size: None,
            bold: None,
            font: None,
            align: None,
            italic: false,
            color: [0, 0, 0],
            runs: Vec::new(),
            thread: None,
        };
        let dump = ExtractionDump {
            schema_version: DUMP_SCHEMA_VERSION,
            source_kind: SourceKind::Pdf,
            source_name: "odd.pdf".into(),
            source_bytes: 10,
            extractor: "test".into(),
            pages: vec![PageInfo {
                index: 0,
                width_pt: 612.0,
                height_pt: 792.0,
                picture_only: false,
            }],
            spans: vec![
                span(0, "welcome to worship", SpanTrust::RawText),
                span(7, "words for a page that is not listed", SpanTrust::RawText),
                span(9, "more words for no listed page", SpanTrust::NeedsReview),
                span(9, "", SpanTrust::PictureOnly),
            ],
            images: Vec::new(),
            loss_notes: Vec::new(),
        };
        let back = ExtractionDump::from_json(&dump.to_json().expect("json")).expect("parse");
        assert_eq!(
            back.spans.len(),
            2,
            "the listed page's words and the wordless picture marker stay"
        );
        assert!(back
            .spans
            .iter()
            .all(|s| s.page == 0 || s.trust == SpanTrust::PictureOnly));
        assert_eq!(back.loss_notes.len(), 1, "one counted loss");
        assert!(
            back.loss_notes[0].contains("2 pieces of text pointed at pages"),
            "the loss is counted in plain words: {}",
            back.loss_notes[0]
        );
    }

    #[test]
    fn garbage_is_an_error_not_a_panic() {
        assert!(extract_pdf(b"this is not a pdf", "x.pdf").is_err());
    }

    #[test]
    fn extract_pdf_refuses_bytes_past_the_open_door_limit() {
        // S-11: the in-process lopdf path is capped like
        // every other door, at entry, in plain words,
        // before any parsing happens.
        let bytes = vec![0u8; (MAX_OPEN_DOOR_BYTES + 1) as usize];
        let err = extract_pdf(&bytes, "big.pdf").expect_err("over the door limit");
        assert!(err.contains("too large"), "{err}");
    }

    fn geom_span(text: &str, x: f32, y: f32, w: f32) -> Span {
        Span {
            page: 0,
            text: text.into(),
            trust: SpanTrust::RawText,
            x: Some(x),
            y: Some(y),
            w: Some(w),
            h: Some(13.0),
            font_size: Some(11.0),
            bold: Some(false),
            font: None,
            align: None,
            italic: false,
            color: [0, 0, 0],
            runs: Vec::new(),
            thread: None,
        }
    }

    fn geom_dump(spans: Vec<Span>) -> ExtractionDump {
        ExtractionDump {
            schema_version: DUMP_SCHEMA_VERSION,
            source_kind: SourceKind::Pdf,
            source_name: "t.pdf".into(),
            source_bytes: 1,
            extractor: "test".into(),
            pages: vec![PageInfo {
                index: 0,
                width_pt: 396.0,
                height_pt: 612.0,
                picture_only: false,
            }],
            spans,
            images: Vec::new(),
            loss_notes: Vec::new(),
        }
    }

    #[test]
    fn alignment_is_inferred_from_line_geometry() {
        // One block, three lines: a wide line sets the block
        // edges (36..360), a centered line floats in the
        // middle, a right-flush line hugs the right edge.
        let mut dump = geom_dump(vec![
            geom_span("A wide line of body text here", 36.0, 40.0, 324.0),
            geom_span("Centered title", 141.0, 56.0, 114.0),
            geom_span("Page 4", 324.0, 72.0, 36.0),
        ]);
        infer_alignments(&mut dump);
        assert_eq!(dump.spans[1].align, Some(ob_model::Align::Center));
        assert_eq!(dump.spans[2].align, Some(ob_model::Align::Right));
        // The widest line itself says Left (its gaps are 0 by
        // construction, so it proves nothing either way).
        assert_eq!(dump.spans[0].align, Some(ob_model::Align::Left));
        // A stated alignment is never overridden.
        let mut dump = geom_dump(vec![
            geom_span("A wide line of body text here", 36.0, 40.0, 324.0),
            geom_span("Centered title", 141.0, 56.0, 114.0),
        ]);
        dump.spans[1].align = Some(ob_model::Align::Right);
        infer_alignments(&mut dump);
        assert_eq!(dump.spans[1].align, Some(ob_model::Align::Right));
    }

    #[test]
    fn a_v1_dump_still_reads() {
        // Schema v1 JSON: no align, no italic, no color, no
        // images. It parses with defaults (the old fixtures
        // and old helper binaries keep working).
        let json = r#"{
            "schema_version": 1,
            "source_kind": "pdf",
            "source_name": "old.pdf",
            "source_bytes": 10,
            "extractor": "old helper",
            "pages": [{"index": 0, "width_pt": 396.0, "height_pt": 612.0, "picture_only": false}],
            "spans": [{"page": 0, "text": "Hello", "trust": "raw_text",
                       "x": 40.0, "y": 40.0, "w": 60.0, "h": 13.0,
                       "font_size": 11.0, "bold": false, "font": "Calibri"}]
        }"#;
        let dump = ExtractionDump::from_json(json).expect("v1 parses");
        assert_eq!(dump.schema_version, 1);
        assert_eq!(dump.spans[0].align, None);
        assert!(!dump.spans[0].italic);
        assert_eq!(dump.spans[0].color, [0, 0, 0]);
        assert!(dump.images.is_empty());
    }

    #[test]
    fn a_dump_from_the_future_is_refused_in_words() {
        // The red team's live repro (audit M10): a fake
        // helper answering schema_version 999 parsed
        // exactly like a real dump. Now the version is
        // compared, and a newer one is refused plainly.
        let json = r#"{
            "schema_version": 999,
            "source_kind": "pdf",
            "source_name": "future.pdf",
            "source_bytes": 10,
            "extractor": "helper from the future",
            "pages": [],
            "spans": []
        }"#;
        let err = ExtractionDump::from_json(json).expect_err("version 999 is not readable");
        // R46-G restatement (flagged): this pin asserted
        // on from_json's error strings; from_json now
        // returns the typed FromJsonError, so it asserts
        // the same words through the Display rendering.
        // No other part of the pin moved.
        assert!(
            err.to_string().contains("newer format (version 999)"),
            "{err}"
        );
        // Version 0 was never a dump format either.
        let json0 = json.replace("999", "0");
        let err = ExtractionDump::from_json(&json0).expect_err("version 0 is not readable");
        assert!(
            err.to_string().contains("does not understand (version 0)"),
            "{err}"
        );
    }

    #[test]
    fn r46g_from_json_version_refusals_are_typed_and_render_byte_for_byte() {
        // R46-G (HF-12 slice 2, from_json half): the
        // version refusals are typed kinds, and their
        // Display renders byte-for-byte the sentences
        // the base tree's from_json produced (the
        // literals below are taken verbatim from the
        // base tree's format strings, with
        // DUMP_SCHEMA_VERSION = 2 spelled out).
        let json = r#"{
            "schema_version": 999,
            "source_kind": "pdf",
            "source_name": "future.pdf",
            "source_bytes": 10,
            "extractor": "helper from the future",
            "pages": [],
            "spans": []
        }"#;
        let err = ExtractionDump::from_json(json).expect_err("version 999 is not readable");
        assert!(matches!(err, FromJsonError::TooNew(999)), "{err}");
        assert_eq!(
            err.to_string(),
            "the reading helper answered in a newer format (version 999) than this \
             copy of OpenBulletin understands (version 2 at most)"
        );
        let json0 = json.replace("999", "0");
        let err = ExtractionDump::from_json(&json0).expect_err("version 0 is not readable");
        assert!(matches!(err, FromJsonError::Unsupported(0)), "{err}");
        assert_eq!(
            err.to_string(),
            "the reading helper answered in a format this copy of OpenBulletin \
             does not understand (version 0); it reads versions 1 and 2"
        );
    }

    #[test]
    fn r46g_from_json_unreadable_renders_the_pre_slice_words_byte_for_byte() {
        // The serde-parse arm keeps its format string
        // character for character; the input below
        // fails serde with a fixed, deterministic
        // message, so the whole sentence is pinned,
        // not just its prefix.
        let err = ExtractionDump::from_json("this is not json").expect_err("garbage is not a dump");
        assert!(matches!(err, FromJsonError::Unreadable(_)), "{err}");
        assert_eq!(
            err.to_string(),
            "the reading helper answered in a way we cannot read: \
             expected ident at line 1 column 2"
        );
    }

    #[test]
    fn r46g_from_from_json_error_for_string_renders_the_same_words() {
        // (c, mirroring R44-I): the boundary conversion
        // the standing String channels rely on renders
        // the same words as the Display.
        let words = String::from(FromJsonError::TooNew(999));
        assert_eq!(
            words,
            "the reading helper answered in a newer format (version 999) than this \
             copy of OpenBulletin understands (version 2 at most)"
        );
        let words = String::from(FromJsonError::Unsupported(0));
        assert_eq!(
            words,
            "the reading helper answered in a format this copy of OpenBulletin \
             does not understand (version 0); it reads versions 1 and 2"
        );
    }
}

pub mod html_export;
pub mod html_import;
pub mod idml_export;
pub mod learn;
pub mod sla;
pub mod sla_export;
