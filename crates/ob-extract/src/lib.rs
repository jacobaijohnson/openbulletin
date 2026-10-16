//! Extraction dump: the one format every importer targets.
//!
//! A bulletin PDF (or later a .pub rescue or an IDML package) goes in;
//! a versioned JSON dump comes out: pages, their sizes, and the text
//! spans found on them, each marked with how much it can be trusted.
//! The Template Analyzer reads dumps, never raw foreign files, so the
//! risky parsing of stranger files lives in exactly one place (and, for
//! the real PDF engine, in a separate helper process: PDFium).
//!
//! This first implementation is scaffolding: page geometry plus the raw
//! text-show operations lopdf can decode. Positions and fonts stay
//! empty until the PDFium helper fills them; the schema already has
//! room, so the analyzer's side of the contract does not change.

pub mod analyzer;
pub mod cfb;
pub mod draft;
pub mod helper;
pub mod idml;
pub mod pub_bridge;
pub mod pub_layout;
pub mod pub_text;
pub mod runner;

use serde::{Deserialize, Serialize};

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
}

impl ExtractionDump {
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|e| e.to_string())
    }
    pub fn from_json(s: &str) -> Result<Self, String> {
        let dump: Self = serde_json::from_str(s)
            .map_err(|e| format!("the reading helper answered in a way we cannot read: {e}"))?;
        // The version is checked, not just carried
        // (audit M10, 2026-10-16): a helper answering
        // in a format from the future must be refused
        // in words, never read as if it were today's.
        // Versions 1 and 2 are understood; 1 parses
        // with the v2 fields defaulted.
        match dump.schema_version {
            1..=DUMP_SCHEMA_VERSION => Ok(dump),
            v if v > DUMP_SCHEMA_VERSION => Err(format!(
                "the reading helper answered in a newer format (version {v}) than this \
                 copy of OpenBulletin understands (version {DUMP_SCHEMA_VERSION} at most)"
            )),
            v => Err(format!(
                "the reading helper answered in a format this copy of OpenBulletin \
                 does not understand (version {v}); it reads versions 1 and \
                 {DUMP_SCHEMA_VERSION}"
            )),
        }
    }
}

/// Extract a dump from PDF bytes using lopdf (structural pass).
pub fn extract_pdf(bytes: &[u8], source_name: &str) -> Result<ExtractionDump, String> {
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
    fn garbage_is_an_error_not_a_panic() {
        assert!(extract_pdf(b"this is not a pdf", "x.pdf").is_err());
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
        assert!(err.contains("newer format (version 999)"), "{err}");
        // Version 0 was never a dump format either.
        let json0 = json.replace("999", "0");
        let err = ExtractionDump::from_json(&json0).expect_err("version 0 is not readable");
        assert!(err.contains("does not understand (version 0)"), "{err}");
    }
}

pub mod html_export;
pub mod html_import;
pub mod idml_export;
pub mod learn;
pub mod sla;
pub mod sla_export;
