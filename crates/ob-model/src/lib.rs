//! OpenBulletin document model.
//!
//! Design rules (see docs/planning/SYNTHESIS.md):
//! - Geometry is integer milli-points (1/1000 pt) so saves are stable and diffs are clean.
//! - The model knows nothing about windows, files, or printers.
//! - A document is a set of *reader panels* (what the congregation reads, in reading
//!   order). Sheets and folds belong to layout/imposition, never to the reader model.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// One milli-point: 1/1000 of a PostScript point.
pub type Mpt = i64;
pub const MPT_PER_INCH: Mpt = 72_000;

/// Convert inches (e.g. 5.5) to milli-points.
pub fn inch(v: f64) -> Mpt {
    (v * MPT_PER_INCH as f64).round() as Mpt
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Size {
    pub w: Mpt,
    pub h: Mpt,
}
impl Size {
    pub fn new(w: Mpt, h: Mpt) -> Self {
        Self { w, h }
    }
}

/// Rectangle in panel space: origin top-left, y grows downward (screen convention).
/// The PDF exporter converts to PDF's bottom-up space in exactly one place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rect {
    pub x: Mpt,
    pub y: Mpt,
    pub w: Mpt,
    pub h: Mpt,
}
impl Rect {
    pub fn new(x: Mpt, y: Mpt, w: Mpt, h: Mpt) -> Self {
        Self { x, y, w, h }
    }
    /// Shrink by a uniform margin.
    pub fn inset(&self, m: Mpt) -> Rect {
        Rect::new(self.x + m, self.y + m, self.w - 2 * m, self.h - 2 * m)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}
impl Rgb {
    pub const BLACK: Rgb = Rgb { r: 0, g: 0, b: 0 };
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Align {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParagraphStyle {
    pub id: String,
    pub name: String,
    pub font_size: Mpt,
    pub bold: bool,
    pub align: Align,
    pub space_after: Mpt,
    pub color: Rgb,
    /// Font family this style is set in, by name (docs/FONTS.md).
    /// The Font Library resolves the name to actual font bytes; the
    /// layout engine only ever shapes with library fonts, so the same
    /// document lays out identically on every machine.
    #[serde(default = "default_font_family")]
    pub font_family: String,
    /// SHA-256 of the exact font file this style was laid out with,
    /// when known. Pins the metrics even if a same-named font changes.
    #[serde(default)]
    pub font_sha256: Option<String>,
}

fn default_font_family() -> String {
    "Liberation Sans".to_string()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Paragraph {
    pub style: String,
    pub text: String,
}
impl Paragraph {
    pub fn new(style: &str, text: &str) -> Self {
        Self {
            style: style.to_string(),
            text: text.to_string(),
        }
    }
}

/// A story is text that belongs together. In later phases one story threads
/// across several frames; in v0.1 each frame holds at most one story.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Story {
    pub id: String,
    pub paragraphs: Vec<Paragraph>,
}

/// Whether content survives the "Next Sunday" command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContentRole {
    /// Standing content: service times, liturgy skeleton, notes lines.
    Standing,
    /// Weekly content: announcements and other text replaced each week.
    Weekly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FrameKind {
    Text,
    /// A simple rectangle/placeholder (cover art block, QR placeholder).
    Shape,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Frame {
    pub id: String,
    pub name: String,
    pub kind: FrameKind,
    pub rect: Rect,
    pub story_id: Option<String>,
    pub role: ContentRole,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Panel {
    pub id: String,
    pub name: String,
    pub size: Size,
    pub frames: Vec<Frame>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ChurchProfile {
    pub name: String,
    pub address: String,
    pub service_times: String,
    pub website: String,
    pub giving_url: String,
    pub pastor: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SheetSpec {
    /// Preset id, e.g. "letter-bifold". Imposition dispatches on this.
    pub preset: String,
    pub size: Size,
}

pub const FORMAT_VERSION: u32 = 0;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Document {
    pub format_version: u32,
    pub title: String,
    pub profile: ChurchProfile,
    /// Field values: sunday_date (ISO), sunday_date_long, sunday_name, reading.
    pub fields: BTreeMap<String, String>,
    pub styles: Vec<ParagraphStyle>,
    pub stories: Vec<Story>,
    pub panels: Vec<Panel>,
    pub sheet: SheetSpec,
}

impl Document {
    pub fn style(&self, id: &str) -> Option<&ParagraphStyle> {
        self.styles.iter().find(|s| s.id == id)
    }
    pub fn story(&self, id: &str) -> Option<&Story> {
        self.stories.iter().find(|s| s.id == id)
    }
    pub fn story_mut(&mut self, id: &str) -> Option<&mut Story> {
        self.stories.iter_mut().find(|s| s.id == id)
    }
}

/// Resolve `{...}` field tokens in text against the profile and field map.
pub fn resolve_fields(text: &str, doc: &Document) -> String {
    let p = &doc.profile;
    let get = |k: &str| doc.fields.get(k).cloned().unwrap_or_default();
    text.replace("{Church.name}", &p.name)
        .replace("{Church.address}", &p.address)
        .replace("{Church.times}", &p.service_times)
        .replace("{Church.website}", &p.website)
        .replace("{Church.giving}", &p.giving_url)
        .replace("{Church.pastor}", &p.pastor)
        .replace("{Sunday.date}", &get("sunday_date_long"))
        .replace("{Sunday.name}", &get("sunday_name"))
        .replace("{Sunday.reading}", &get("reading"))
}

/// The house style set every template starts from. Large-print leaning:
/// body text is 11pt minimum, order-of-service lines get their own style
/// because they carry tab-leader hymn numbers.
pub fn default_styles() -> Vec<ParagraphStyle> {
    let st =
        |id: &str, name: &str, size: Mpt, bold: bool, align: Align, after: Mpt| ParagraphStyle {
            id: id.to_string(),
            name: name.to_string(),
            font_size: size,
            bold,
            align,
            space_after: after,
            color: Rgb::BLACK,
            font_family: default_font_family(),
            font_sha256: None,
        };
    vec![
        st(
            "church-name",
            "Church Name",
            26_000,
            true,
            Align::Center,
            6_000,
        ),
        st("heading", "Heading", 18_000, true, Align::Left, 6_000),
        st("subheading", "Subheading", 13_000, true, Align::Left, 4_000),
        st("body", "Body", 11_000, false, Align::Left, 4_000),
        st(
            "body-center",
            "Body Centered",
            11_000,
            false,
            Align::Center,
            4_000,
        ),
        st(
            "order-line",
            "Order of Service Line",
            11_000,
            false,
            Align::Left,
            3_000,
        ),
        st(
            "notes-line",
            "Sermon Notes Line",
            11_000,
            false,
            Align::Left,
            10_000,
        ),
        st("caption", "Caption", 9_500, false, Align::Center, 3_000),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inch_math() {
        assert_eq!(inch(1.0), 72_000);
        assert_eq!(inch(5.5), 396_000);
        assert_eq!(inch(8.5), 612_000);
    }

    #[test]
    fn fields_resolve() {
        let mut fields = BTreeMap::new();
        fields.insert(
            "sunday_name".to_string(),
            "Twentieth Sunday after Pentecost".to_string(),
        );
        fields.insert(
            "sunday_date_long".to_string(),
            "October 18, 2026".to_string(),
        );
        let doc = Document {
            format_version: FORMAT_VERSION,
            title: "t".into(),
            profile: ChurchProfile {
                name: "Grace Lutheran".into(),
                ..Default::default()
            },
            fields,
            styles: vec![],
            stories: vec![],
            panels: vec![],
            sheet: SheetSpec {
                preset: "letter-portrait".into(),
                size: Size::new(0, 0),
            },
        };
        assert_eq!(
            resolve_fields("{Church.name} - {Sunday.name}, {Sunday.date}", &doc),
            "Grace Lutheran - Twentieth Sunday after Pentecost, October 18, 2026"
        );
    }
}
