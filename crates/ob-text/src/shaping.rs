//! Real text shaping on parley (HarfBuzz-class shaping + line breaking).
//!
//! The estimator in the parent module exists for quick, fontless checks.
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
pub fn bundled_shaper() -> Shaper {
    let mut s = Shaper::new();
    s.register_font(include_bytes!("../assets/fonts/LiberationSans-Regular.ttf").to_vec());
    s.register_font(include_bytes!("../assets/fonts/LiberationSans-Bold.ttf").to_vec());
    s.register_font(include_bytes!("../assets/fonts/LiberationSerif-Regular.ttf").to_vec());
    s.register_font(include_bytes!("../assets/fonts/LiberationSerif-Bold.ttf").to_vec());
    s
}

/// A reusable shaping context. Register bundled or app fonts once, then
/// shape many strings.
pub struct Shaper {
    fonts: FontContext,
    layout: LayoutContext<()>,
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
        }
    }

    /// Register font bytes (TTF/OTF) so `FontSpec::family` can name them.
    /// Used for bundled fonts and for deterministic tests.
    pub fn register_font(&mut self, data: Vec<u8>) {
        self.fonts
            .collection
            .register_fonts(fontique::Blob::new(std::sync::Arc::new(data)), None);
    }

    fn build_layout(
        &mut self,
        text: &str,
        spec: &FontSpec,
        size: Mpt,
        leading_rel: f32,
        max_width: Option<Mpt>,
    ) -> parley::Layout<()> {
        let size_px = size as f32 / 1000.0;
        let mut builder = self.layout.ranged_builder(&mut self.fonts, text, 1.0, true);
        builder.push_default(StyleProperty::FontFamily(FontFamily::from(
            spec.family.as_str(),
        )));
        builder.push_default(StyleProperty::FontSize(size_px));
        builder.push_default(StyleProperty::FontWeight(if spec.bold {
            FontWeight::BOLD
        } else {
            FontWeight::NORMAL
        }));
        if spec.italic {
            builder.push_default(StyleProperty::FontStyle(FontStyle::Italic));
        }
        builder.push_default(StyleProperty::LineHeight(LineHeight::FontSizeRelative(
            leading_rel,
        )));
        let mut layout = builder.build(text);
        layout.break_all_lines(max_width.map(|w| w as f32 / 1000.0));
        layout
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
