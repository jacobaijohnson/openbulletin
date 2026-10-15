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

use serde::{Deserialize, Serialize};

/// Bump when the dump shape changes incompatibly.
pub const DUMP_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    Pdf,
    Pub,
    Idml,
    Obd,
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
}

impl ExtractionDump {
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|e| e.to_string())
    }
    pub fn from_json(s: &str) -> Result<Self, String> {
        serde_json::from_str(s).map_err(|e| e.to_string())
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
    })
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
        ob_pdf::export_reader_pdf(&doc)
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
}
