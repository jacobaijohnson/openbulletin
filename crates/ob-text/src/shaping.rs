//! Real text shaping on parley (HarfBuzz-class shaping + line breaking).
//!
//! Anything that decides whether text fits a frame (preflight, import
//! honesty labels, the PDF writer) must come through here: real advances,
//! real line breaks, real overflow.
//!
//! Units: parley works in float "pixels"; we treat 1 unit = 1 point and
//! convert to milli-points at the boundary.

use ob_model::Mpt;
use parley::{
    FontContext, FontFamily, FontStyle, FontWeight, LayoutContext, LineHeight, StyleProperty,
};

/// Which font to shape with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontSpec {
    pub family: String,
    pub bold: bool,
    pub italic: bool,
}

impl FontSpec {
    pub fn new(family: impl Into<String>) -> Self {
        Self {
            family: family.into(),
            bold: false,
            italic: false,
        }
    }
    pub fn bold(mut self) -> Self {
        self.bold = true;
        self
    }
}

/// One styled span over a paragraph's text: a byte range
/// plus the EFFECTIVE face switches for it (the caller has
/// already folded the paragraph style in:
/// `run.bold.unwrap_or(style.bold)`). Spans are how inline
/// formatting (G1b) reaches the shaper: one parley layout
/// for the whole paragraph, with the weight and style
/// pushed per range, so line breaking stays parley-true
/// across run boundaries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunSpan {
    pub range: std::ops::Range<usize>,
    pub bold: bool,
    pub italic: bool,
}

/// The spans of `spans` that intersect `range`, clipped to
/// it and rebased so `range.start` becomes 0: the spans of
/// one display line, or of one trial string in the strict
/// re-break.
pub fn clip_spans(spans: &[RunSpan], range: std::ops::Range<usize>) -> Vec<RunSpan> {
    let mut out = Vec::new();
    for s in spans {
        let start = s.range.start.max(range.start);
        let end = s.range.end.min(range.end);
        if start < end {
            out.push(RunSpan {
                range: start - range.start..end - range.start,
                bold: s.bold,
                italic: s.italic,
            });
        }
    }
    out
}

/// One shaped line of text.
#[derive(Debug, Clone, PartialEq)]
pub struct ShapedLine {
    pub text: String,
    pub width: Mpt,
}

/// Result of laying a story out inside a frame.
#[derive(Debug, Clone, PartialEq)]
pub struct FrameFit {
    pub lines: Vec<ShapedLine>,
    /// Height the lines need, in milli-points (line count x leading).
    pub needed_height: Mpt,
    /// True when every line fits the width and the total height fits.
    pub fits: bool,
    /// Lines beyond the frame height (0 when it fits vertically).
    pub overflow_lines: usize,
    /// True when some single line is wider than the frame (unbreakable
    /// word): the classic silent-overflow trap.
    pub has_wide_line: bool,
}

/// A Shaper pre-loaded with the bundled fonts (Liberation Sans and
/// Serif, regular and bold). Liberation is metric-compatible with Arial
/// and very close to the Helvetica the PDF writer draws, so shaped line
/// breaks and fit checks track the printed page closely.
/// The bundled starter fonts as (file name, bytes): the eight
/// Liberation faces. The app seeds the Font Library with these on
/// first run so the library folder is the one source of fonts on
/// every machine shape (docs/FONTS.md).
pub fn bundled_font_files() -> Vec<(&'static str, &'static [u8])> {
    vec![
        (
            "LiberationSans-Regular.ttf",
            include_bytes!("../assets/fonts/LiberationSans-Regular.ttf"),
        ),
        (
            "LiberationSans-Bold.ttf",
            include_bytes!("../assets/fonts/LiberationSans-Bold.ttf"),
        ),
        (
            "LiberationSerif-Regular.ttf",
            include_bytes!("../assets/fonts/LiberationSerif-Regular.ttf"),
        ),
        (
            "LiberationSerif-Bold.ttf",
            include_bytes!("../assets/fonts/LiberationSerif-Bold.ttf"),
        ),
        (
            "LiberationSans-Italic.ttf",
            include_bytes!("../assets/fonts/LiberationSans-Italic.ttf"),
        ),
        (
            "LiberationSans-BoldItalic.ttf",
            include_bytes!("../assets/fonts/LiberationSans-BoldItalic.ttf"),
        ),
        (
            "LiberationSerif-Italic.ttf",
            include_bytes!("../assets/fonts/LiberationSerif-Italic.ttf"),
        ),
        (
            "LiberationSerif-BoldItalic.ttf",
            include_bytes!("../assets/fonts/LiberationSerif-BoldItalic.ttf"),
        ),
    ]
}

/// The bundled starter fonts as inspected facts (family,
/// face flags, rights, hash), for the consumers that
/// enumerate faces without shaping: the document font pin
/// shares the PDF writer's candidate rule, whose set is
/// these faces plus the Font Library's
/// (ob_fonts::candidate_faces, round 4 R4-9).
pub fn bundled_font_infos() -> Vec<ob_fonts::FontInfo> {
    bundled_font_files()
        .iter()
        .filter_map(|(name, bytes)| ob_fonts::inspect(bytes, name).ok())
        .collect()
}

pub fn bundled_shaper() -> Shaper {
    let mut s = Shaper::new();
    s.register_font(include_bytes!("../assets/fonts/LiberationSans-Regular.ttf").to_vec());
    s.register_font(include_bytes!("../assets/fonts/LiberationSans-Bold.ttf").to_vec());
    s.register_font(include_bytes!("../assets/fonts/LiberationSerif-Regular.ttf").to_vec());
    s.register_font(include_bytes!("../assets/fonts/LiberationSerif-Bold.ttf").to_vec());
    s.register_font(include_bytes!("../assets/fonts/LiberationSans-Italic.ttf").to_vec());
    s.register_font(include_bytes!("../assets/fonts/LiberationSans-BoldItalic.ttf").to_vec());
    s.register_font(include_bytes!("../assets/fonts/LiberationSerif-Italic.ttf").to_vec());
    s.register_font(include_bytes!("../assets/fonts/LiberationSerif-BoldItalic.ttf").to_vec());
    s
}

/// A reusable shaping context. Register bundled or app fonts once, then
/// shape many strings.
pub struct Shaper {
    fonts: FontContext,
    layout: LayoutContext<()>,
    /// Requested family -> the family the resolution chain
    /// answered with, against the registered families at
    /// answer time. Registering a font clears the cache:
    /// the new face may be the real font a document was
    /// waiting for (docs/FONTS.md, wanted fonts).
    resolved: std::collections::HashMap<String, String>,
}

impl Default for Shaper {
    fn default() -> Self {
        Self::new()
    }
}

impl Shaper {
    /// Shaper backed by system fonts.
    pub fn new() -> Self {
        Self {
            fonts: FontContext::new(),
            layout: LayoutContext::new(),
            resolved: std::collections::HashMap::new(),
        }
    }

    /// Register font bytes (TTF/OTF) so `FontSpec::family` can name them.
    /// Used for bundled fonts and for deterministic tests.
    pub fn register_font(&mut self, data: Vec<u8>) {
        self.fonts
            .collection
            .register_fonts(fontique::Blob::new(std::sync::Arc::new(data)), None);
        self.resolved.clear();
    }

    /// The family a request for `family` shapes with: the
    /// shared resolution chain (ob_fonts::resolve) walked
    /// against the families registered in this shaper right
    /// now. Every layout built here resolves its spec's
    /// family through this, so a document naming a font
    /// the library lacked shapes in the stand-in, and
    /// shapes in the real font the moment it is registered,
    /// with no edit to the document.
    pub fn resolved_family(&mut self, family: &str) -> String {
        if let Some(hit) = self.resolved.get(family) {
            return hit.clone();
        }
        let available: Vec<String> = self
            .fonts
            .collection
            .family_names()
            .map(|s| s.to_string())
            .collect();
        let answer = ob_fonts::resolve::resolve_family(family, &available).family;
        self.resolved.insert(family.to_string(), answer.clone());
        answer
    }

    pub(crate) fn build_layout(
        &mut self,
        text: &str,
        spec: &FontSpec,
        size: Mpt,
        leading_rel: f32,
        max_width: Option<Mpt>,
    ) -> parley::Layout<()> {
        self.build_layout_spanned(text, spec, &[], size, leading_rel, max_width)
    }

    /// `build_layout` with per-range face switches: the
    /// base spec supplies the family and the default weight
    /// and style; each span pushes its own effective weight
    /// and style over its range. Ranges the spans do not
    /// cover keep the base face. This is the one layout the
    /// whole runs pipeline shares, so a paragraph breaks
    /// identically whether it carries runs or not.
    pub(crate) fn build_layout_spanned(
        &mut self,
        text: &str,
        spec: &FontSpec,
        spans: &[RunSpan],
        size: Mpt,
        leading_rel: f32,
        max_width: Option<Mpt>,
    ) -> parley::Layout<()> {
        let size_px = size as f32 / 1000.0;
        // The resolution chain, at the one place every
        // layout is built: the spec names the family the
        // document asks for, and the words shape in the
        // family the chain answers with (real font, twin,
        // lookalike, or house default).
        let family = self.resolved_family(&spec.family);
        let mut builder = self.layout.ranged_builder(&mut self.fonts, text, 1.0, true);
        builder.push_default(StyleProperty::FontFamily(FontFamily::from(family.as_str())));
        builder.push_default(StyleProperty::FontSize(size_px));
        builder.push_default(StyleProperty::FontWeight(if spec.bold {
            FontWeight::BOLD
        } else {
            FontWeight::NORMAL
        }));
        builder.push_default(StyleProperty::FontStyle(if spec.italic {
            FontStyle::Italic
        } else {
            FontStyle::Normal
        }));
        for span in spans {
            if span.range.start >= span.range.end || span.range.end > text.len() {
                continue;
            }
            builder.push(
                StyleProperty::FontWeight(if span.bold {
                    FontWeight::BOLD
                } else {
                    FontWeight::NORMAL
                }),
                span.range.clone(),
            );
            builder.push(
                StyleProperty::FontStyle(if span.italic {
                    FontStyle::Italic
                } else {
                    FontStyle::Normal
                }),
                span.range.clone(),
            );
        }
        builder.push_default(StyleProperty::LineHeight(LineHeight::FontSizeRelative(
            leading_rel,
        )));
        let mut layout = builder.build(text);
        layout.break_all_lines(max_width.map(|w| w as f32 / 1000.0));
        layout
    }

    /// Shaped width of a string whose face switches change
    /// mid-string: the spans are over `text` itself.
    pub fn measure_spanned(
        &mut self,
        text: &str,
        spans: &[RunSpan],
        spec: &FontSpec,
        size: Mpt,
    ) -> Mpt {
        if text.is_empty() {
            return 0;
        }
        let layout = self.build_layout_spanned(text, spec, spans, size, 1.0, None);
        let mut max = 0.0f32;
        for line in layout.lines() {
            max = max.max(line.metrics().advance);
        }
        (max * 1000.0).round() as Mpt
    }

    /// Shaped width of a single-line string, in milli-points.
    pub fn measure(&mut self, text: &str, spec: &FontSpec, size: Mpt) -> Mpt {
        if text.is_empty() {
            return 0;
        }
        let layout = self.build_layout(text, spec, size, 1.0, None);
        let mut max = 0.0f32;
        for line in layout.lines() {
            max = max.max(line.metrics().advance);
        }
        (max * 1000.0).round() as Mpt
    }

    /// Break a paragraph into lines that fit `max_width` (real shaping,
    /// real line breaking). Parley's optimizer occasionally accepts a
    /// line a hair over the width on tight lines; a strict greedy re-break
    /// of any over-wide line (using shaped word widths) restores the
    /// width guarantee. Only a single unbreakable word can still exceed
    /// the width, and callers see that in the returned widths.
    pub fn wrap(
        &mut self,
        text: &str,
        max_width: Mpt,
        spec: &FontSpec,
        size: Mpt,
        leading_rel: f32,
    ) -> Vec<ShapedLine> {
        if text.is_empty() {
            return vec![ShapedLine {
                text: String::new(),
                width: 0,
            }];
        }
        let layout = self.build_layout(text, spec, size, leading_rel, Some(max_width));
        let mut out = Vec::new();
        for line in layout.lines() {
            let range = line.text_range();
            let slice = text.get(range).unwrap_or("").trim_end().to_string();
            out.push(ShapedLine {
                text: slice,
                width: (line.metrics().advance * 1000.0).round() as Mpt,
            });
        }
        if out.is_empty() {
            out.push(ShapedLine {
                text: String::new(),
                width: 0,
            });
            return out;
        }
        // Strict pass: re-break any line that came back over-wide.
        let mut strict = Vec::new();
        for line in out {
            if line.width <= max_width || !line.text.contains(' ') {
                strict.push(line);
                continue;
            }
            let mut cur = String::new();
            for word in line.text.split(' ') {
                let trial = if cur.is_empty() {
                    word.to_string()
                } else {
                    format!("{cur} {word}")
                };
                if !cur.is_empty() && self.measure(&trial, spec, size) > max_width {
                    let w = self.measure(&cur, spec, size);
                    strict.push(ShapedLine {
                        text: cur,
                        width: w,
                    });
                    cur = word.to_string();
                } else {
                    cur = trial;
                }
            }
            if !cur.is_empty() {
                let w = self.measure(&cur, spec, size);
                strict.push(ShapedLine {
                    text: cur,
                    width: w,
                });
            }
        }
        strict
    }

    /// A dot-leader line ("Hymn of Praise ...... 432") sized with real
    /// shaped widths, so the right-hand number lands on the frame edge.
    pub fn leader_line(
        &mut self,
        left: &str,
        right: &str,
        width: Mpt,
        spec: &FontSpec,
        size: Mpt,
    ) -> String {
        if right.is_empty() {
            return left.to_string();
        }
        let dot_w = self.measure(".", spec, size).max(1);
        let used = self.measure(left, spec, size) + self.measure(right, spec, size) + 2 * dot_w;
        let dots = ((width - used) / dot_w).max(2) as usize;
        format!("{left} {} {right}", ".".repeat(dots))
    }

    /// Expand the tab in an order-of-service line into a dot-leader line.
    pub fn expand_order_line(
        &mut self,
        text: &str,
        width: Mpt,
        spec: &FontSpec,
        size: Mpt,
    ) -> String {
        match text.split_once('\t') {
            Some((l, r)) => self.leader_line(l.trim_end(), r.trim(), width, spec, size),
            None => text.to_string(),
        }
    }

    /// Lay a story out in a frame and report whether it fits, loudly.
    pub fn frame_fit(
        &mut self,
        text: &str,
        frame_w: Mpt,
        frame_h: Mpt,
        spec: &FontSpec,
        size: Mpt,
        leading_rel: f32,
    ) -> FrameFit {
        let lines = self.wrap(text, frame_w, spec, size, leading_rel);
        let line_h = (size as f32 * leading_rel).round() as Mpt;
        let needed_height = line_h * lines.len() as Mpt;
        let has_wide_line = lines.iter().any(|l| l.width > frame_w);
        let overflow_lines = if needed_height > frame_h {
            let fits_count = (frame_h / line_h.max(1)) as usize;
            lines.len().saturating_sub(fits_count)
        } else {
            0
        };
        FrameFit {
            fits: needed_height <= frame_h && !has_wide_line,
            lines,
            needed_height,
            overflow_lines,
            has_wide_line,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shaper() -> Shaper {
        let mut s = Shaper::new();
        s.register_font(include_bytes!("../assets/fonts/LiberationSans-Regular.ttf").to_vec());
        s.register_font(include_bytes!("../assets/fonts/LiberationSans-Bold.ttf").to_vec());
        s
    }

    /// The bundled set is complete: eight faces, all parseable.
    /// A copy of a bundled face whose family-name records
    /// (name IDs 1 and 16) say `new` instead of `old`: the
    /// install-later fixture. A church's real Gill Sans MT
    /// is a commercial file we cannot ship in a test; a
    /// free face WEARING the name is exactly what the
    /// library holds the day the admin adds the real one,
    /// as far as every name-based lookup can tell.
    fn renamed_family(bytes: &[u8], old: &str, new: &str) -> Vec<u8> {
        let mut out = bytes.to_vec();
        let num_tables = u16::from_be_bytes([out[4], out[5]]) as usize;
        let mut name_off = 0usize;
        for i in 0..num_tables {
            let rec = 12 + i * 16;
            if &out[rec..rec + 4] == b"name" {
                name_off =
                    u32::from_be_bytes([out[rec + 8], out[rec + 9], out[rec + 10], out[rec + 11]])
                        as usize;
            }
        }
        assert!(name_off > 0, "the font has a name table");
        let count = u16::from_be_bytes([out[name_off + 2], out[name_off + 3]]) as usize;
        let storage = u16::from_be_bytes([out[name_off + 4], out[name_off + 5]]) as usize;
        let mut patched: Vec<usize> = Vec::new();
        for i in 0..count {
            let rec = name_off + 6 + i * 12;
            let platform = u16::from_be_bytes([out[rec], out[rec + 1]]);
            let name_id = u16::from_be_bytes([out[rec + 6], out[rec + 7]]);
            if name_id != 1 && name_id != 16 {
                continue;
            }
            let len = u16::from_be_bytes([out[rec + 8], out[rec + 9]]) as usize;
            let off = u16::from_be_bytes([out[rec + 10], out[rec + 11]]) as usize;
            let abs = name_off + storage + off;
            let raw = &bytes[abs..abs + len];
            let text: String = if platform == 1 {
                raw.iter().map(|&b| b as char).collect()
            } else {
                let units: Vec<u16> = raw
                    .chunks(2)
                    .map(|c| u16::from_be_bytes([c[0], c[1]]))
                    .collect();
                String::from_utf16_lossy(&units)
            };
            if text != old {
                continue;
            }
            let enc_len = if platform == 1 {
                new.len()
            } else {
                new.len() * 2
            };
            if !patched.contains(&abs) {
                let encoded: Vec<u8> = if platform == 1 {
                    new.bytes().collect()
                } else {
                    new.encode_utf16().flat_map(|u| u.to_be_bytes()).collect()
                };
                assert!(encoded.len() <= len, "the new name must fit the record");
                out[abs..abs + encoded.len()].copy_from_slice(&encoded);
                for b in &mut out[abs + encoded.len()..abs + len] {
                    *b = 0;
                }
                patched.push(abs);
            }
            out[rec + 8..rec + 10].copy_from_slice(&(enc_len as u16).to_be_bytes());
        }
        assert!(!patched.is_empty(), "a family record was patched");
        out
    }

    #[test]
    fn the_chain_decides_what_a_missing_family_shapes_in() {
        // Only Liberation registered: Gill Sans MT falls
        // to the house default (its lookalike is absent).
        let mut s = shaper();
        assert_eq!(s.resolved_family("Gill Sans MT"), "Liberation Sans");
        // A face named Cabin arrives: the lookalike tier.
        s.register_font(renamed_family(
            include_bytes!("../assets/fonts/LiberationSerif-Regular.ttf"),
            "Liberation Serif",
            "Cabin",
        ));
        assert_eq!(s.resolved_family("Gill Sans MT"), "Cabin");
        // The real name arrives: exact, and the shaped
        // widths are the Gill-named face's own (here the
        // serif bytes wearing the name), not the stand-in's.
        s.register_font(renamed_family(
            include_bytes!("../assets/fonts/LiberationSerif-Regular.ttf"),
            "Liberation Serif",
            "Gill Sans MT",
        ));
        assert_eq!(s.resolved_family("Gill Sans MT"), "Gill Sans MT");
        let text = "Hymn of the Day";
        let gill = s.measure(text, &FontSpec::new("Gill Sans MT"), 11_000);
        let face = ttf_parser::Face::parse(
            include_bytes!("../assets/fonts/LiberationSerif-Regular.ttf"),
            0,
        )
        .unwrap();
        let upem = face.units_per_em() as i64;
        let want: i64 = text
            .chars()
            .map(|c| {
                let gid = face.glyph_index(c).map(|g| g.0).unwrap_or(0);
                face.glyph_hor_advance(ttf_parser::GlyphId(gid))
                    .unwrap_or(0) as i64
            })
            .sum::<i64>()
            * 11_000
            / upem;
        assert!(
            (gill - want).abs() <= 30,
            "shaped {gill} should be the Gill-named face's widths {want}"
        );
    }

    #[test]
    fn bundled_set_has_all_eight_faces() {
        let files = bundled_font_files();
        assert_eq!(files.len(), 8);
        for (name, bytes) in &files {
            assert!(
                ttf_parser::Face::parse(bytes, 0).is_ok(),
                "{name} should parse"
            );
        }
    }

    /// Italic shaping uses the real italic face: shape a string in
    /// the italic face only and compare against that face's hmtx.
    #[test]
    fn italic_spec_shapes_with_the_italic_face() {
        let mut shaper = Shaper::new();
        shaper.register_font(include_bytes!("../assets/fonts/LiberationSans-Italic.ttf").to_vec());
        let text = "The Lord be with you";
        let spec = FontSpec {
            family: "Liberation Sans".into(),
            bold: false,
            italic: true,
        };
        let shaped = shaper.measure(text, &spec, 11_000);
        let face = ttf_parser::Face::parse(
            include_bytes!("../assets/fonts/LiberationSans-Italic.ttf"),
            0,
        )
        .expect("italic face parses");
        let upem = face.units_per_em() as i64;
        let mut want: i64 = 0;
        for ch in text.chars() {
            let gid = face.glyph_index(ch).expect("glyph");
            want += face.glyph_hor_advance(gid).expect("advance") as i64;
        }
        let want = (want * 11_000) / upem;
        assert!(
            (shaped - want).abs() <= 30,
            "italic shaped {shaped} vs hmtx {want}"
        );
    }

    /// Independent oracle: width straight from the font's hmtx table
    /// (no shaping, no kerning) via ttf-parser.
    fn hmtx_width(text: &str, size: Mpt, bold: bool) -> Mpt {
        let data: &[u8] = if bold {
            include_bytes!("../assets/fonts/LiberationSans-Bold.ttf")
        } else {
            include_bytes!("../assets/fonts/LiberationSans-Regular.ttf")
        };
        let face = ttf_parser::Face::parse(data, 0).unwrap();
        let upem = face.units_per_em() as i64;
        let sum: i64 = text
            .chars()
            .map(|c| {
                let gid = face.glyph_index(c).map(|g| g.0).unwrap_or(0);
                face.glyph_hor_advance(ttf_parser::GlyphId(gid))
                    .unwrap_or(0) as i64
            })
            .sum();
        (sum * size) / upem
    }

    #[test]
    fn measure_matches_font_table_within_kerning() {
        let mut s = shaper();
        let spec = FontSpec::new("Liberation Sans");
        let text = "Hymn of the Day";
        let shaped = s.measure(text, &spec, 11_000);
        let raw = hmtx_width(text, 11_000, false);
        // Shaping may tighten (kerning/ligatures) but never exceed the raw
        // advance sum by more than a hair, and must land within 8% of it.
        assert!(shaped <= raw + raw / 100, "shaped {shaped} vs raw {raw}");
        assert!(shaped >= raw - raw / 12, "shaped {shaped} vs raw {raw}");
        assert!(shaped > 50_000, "sanity: 15 chars at 11pt is wide");
    }

    #[test]
    fn bold_is_wider_than_regular() {
        let mut s = shaper();
        let text = "Order of Service";
        let reg = s.measure(text, &FontSpec::new("Liberation Sans"), 11_000);
        let bold = s.measure(text, &FontSpec::new("Liberation Sans").bold(), 11_000);
        assert!(bold > reg, "bold {bold} should exceed regular {reg}");
    }

    #[test]
    fn wrap_lines_fit_width() {
        let mut s = shaper();
        let spec = FontSpec::new("Liberation Sans");
        let text = "Welcome to Grace Lutheran Church. We are glad you are here with us this morning for worship and fellowship.";
        let lines = s.wrap(text, 180_000, &spec, 11_000, 1.25);
        assert!(
            lines.len() >= 3,
            "expected several lines, got {}",
            lines.len()
        );
        for l in &lines {
            assert!(
                l.width <= 180_000,
                "line too wide: {} ({})",
                l.text,
                l.width
            );
        }
        let rejoined: String = lines
            .iter()
            .map(|l| l.text.clone())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(rejoined.contains("fellowship"));
    }

    #[test]
    fn frame_fit_detects_overflow_and_wide_words() {
        let mut s = shaper();
        let spec = FontSpec::new("Liberation Sans");
        // Panel-interior width of a bi-fold panel (5.5in - margins).
        let w = 320_000;
        let short = "Hymn of the Day: A Mighty Fortress";
        let fit = s.frame_fit(short, w, 60_000, &spec, 11_000, 1.25);
        assert!(fit.fits, "short text should fit: {:?}", fit);

        let long = "Announcements: The fall cleanup day is Saturday at nine. Bring gloves, rakes, and a friend. Lunch is provided for all volunteers in the fellowship hall after we finish the yard work and window washing.";
        let over = s.frame_fit(long, w, 40_000, &spec, 11_000, 1.25);
        assert!(!over.fits);
        assert!(over.overflow_lines > 0, "must report overflow lines");

        let wide = s.frame_fit(
            "Antidisestablishmentarianism",
            60_000,
            100_000,
            &spec,
            11_000,
            1.25,
        );
        assert!(wide.has_wide_line, "unbreakable word must be flagged");
        assert!(!wide.fits);
    }
}
