//! Caret-grade text layout for the canvas editor (E1).
//!
//! [`Shaper::wrap`] answers "which words land on which line, and
//! how wide is each line". Editing needs the next level down:
//! where each cluster of characters sits, where a caret at a
//! given byte offset is drawn, and which byte offset a click
//! lands on. This module builds that geometry from parley's
//! shaped output — never from the estimator in the parent
//! module — and, critically, its line breaks are computed by
//! the same two-phase algorithm as `wrap` (parley's lines, then
//! the strict word re-break of any over-wide line), so the
//! breaks a user edits against are the breaks the idle canvas
//! draws and the PDF prints.
//!
//! Coordinates are points (parley's space), relative to the
//! paragraph's top-left corner. Byte offsets are offsets into
//! the paragraph string the layout was built from.
//!
//! Why caret math is arithmetic over extracted clusters rather
//! than parley's own `Cursor` over the whole `Layout`: the
//! strict re-break can split a parley line after the fact, so
//! the display lines the editor works with are not always
//! parley's lines. The clusters themselves still come from
//! parley shaping of each display line's exact text.

use crate::shaping::{clip_spans, FontSpec, RunSpan, Shaper};
use ob_model::{Align, Mpt};
use std::ops::Range;

/// One shaped cluster (a grapheme-ish unit parley refuses to
/// split): its byte range in the paragraph text and its
/// position within its line.
#[derive(Debug, Clone, PartialEq)]
pub struct ClusterGeom {
    pub range: Range<usize>,
    /// X of the cluster's left edge, relative to the line's
    /// own left edge (before alignment offset).
    pub x: f32,
    pub width: f32,
    /// The effective face switches of the run this cluster
    /// belongs to (G1b): painters draw each maximal run of
    /// same-flag clusters in that face.
    pub bold: bool,
    pub italic: bool,
}

/// One display line of a laid-out paragraph.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutLine {
    /// Byte range of the line's text in the paragraph string
    /// (trailing wrap whitespace excluded, exactly as `wrap`
    /// trims its slices).
    pub range: Range<usize>,
    /// The line's advance as `wrap` reports it (used for the
    /// alignment offset, so an edited line sits where the idle
    /// canvas would put the same line).
    pub width: f32,
    /// Advance of the line's visible text (clusters summed);
    /// the caret at `range.end` sits at `x0 + text_advance`.
    pub text_advance: f32,
    /// X of the line's left edge within the paragraph, after
    /// alignment.
    pub x0: f32,
    pub clusters: Vec<ClusterGeom>,
}

/// A paragraph laid out for editing: display lines with
/// cluster geometry, caret mapping both ways.
#[derive(Debug, Clone, PartialEq)]
pub struct TextLayout {
    text: String,
    lines: Vec<LayoutLine>,
    line_height: f32,
}

impl TextLayout {
    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn lines(&self) -> &[LayoutLine] {
        &self.lines
    }

    pub fn line_height(&self) -> f32 {
        self.line_height
    }

    /// A copy holding only the first `n` lines, over the
    /// same text: the piece of a paragraph one box of a
    /// thread draws when the story continues in the next
    /// box (T1). Line ranges still index the whole text, so
    /// painters read the piece exactly as they read a full
    /// layout.
    pub fn first_lines(&self, n: usize) -> TextLayout {
        TextLayout {
            text: self.text.clone(),
            lines: self.lines.iter().take(n).cloned().collect(),
            line_height: self.line_height,
        }
    }

    /// Total height of the paragraph's lines, in points.
    pub fn height(&self) -> f32 {
        self.lines.len() as f32 * self.line_height
    }

    /// X (in paragraph coordinates) of the caret for a byte
    /// offset that falls in `line`.
    fn x_at(&self, line: &LayoutLine, offset: usize) -> f32 {
        if offset <= line.range.start {
            return line.x0;
        }
        if offset >= line.range.end {
            return line.x0 + line.text_advance;
        }
        for c in &line.clusters {
            if offset < c.range.start {
                // Between clusters: the previous cluster's end.
                return line.x0 + c.x;
            }
            if offset < c.range.end {
                let span = (c.range.end - c.range.start).max(1) as f32;
                let frac = (offset - c.range.start) as f32 / span;
                return line.x0 + c.x + frac * c.width;
            }
        }
        line.x0 + line.text_advance
    }

    /// The caret rectangle for a byte offset: (x, y of the
    /// line's top, height), in paragraph coordinates. An
    /// offset in the whitespace a wrap consumed between two
    /// lines belongs to the end of the earlier line.
    pub fn caret_rect(&self, byte_offset: usize) -> (f32, f32, f32) {
        if self.lines.is_empty() {
            return (0.0, 0.0, self.line_height);
        }
        let mut li = self
            .lines
            .iter()
            .position(|l| byte_offset <= l.range.end)
            .unwrap_or(self.lines.len() - 1);
        if byte_offset < self.lines[li].range.start && li > 0 {
            li -= 1;
            let line = &self.lines[li];
            return (
                line.x0 + line.text_advance,
                li as f32 * self.line_height,
                self.line_height,
            );
        }
        let line = &self.lines[li];
        (
            self.x_at(line, byte_offset),
            li as f32 * self.line_height,
            self.line_height,
        )
    }

    /// The byte offset a point (paragraph coordinates) lands
    /// on: nearest cluster boundary, line clamped vertically.
    pub fn byte_offset_at(&self, x: f32, y: f32) -> usize {
        if self.lines.is_empty() {
            return 0;
        }
        let li = if y <= 0.0 {
            0
        } else {
            (y / self.line_height) as usize
        }
        .min(self.lines.len() - 1);
        let line = &self.lines[li];
        let lx = x - line.x0;
        if lx <= 0.0 {
            return line.range.start;
        }
        for c in &line.clusters {
            if lx < c.x + c.width / 2.0 {
                return c.range.start;
            }
            if lx < c.x + c.width {
                return c.range.end;
            }
        }
        line.range.end
    }

    /// Highlight rectangles for a byte range: one (x, y, w, h)
    /// per line the range touches, in paragraph coordinates.
    pub fn selection_rects(&self, range: Range<usize>) -> Vec<(f32, f32, f32, f32)> {
        let mut out = Vec::new();
        if range.start >= range.end {
            return out;
        }
        for (li, line) in self.lines.iter().enumerate() {
            let s = range.start.max(line.range.start);
            let e = range.end.min(line.range.end);
            if s < e {
                let x1 = self.x_at(line, s);
                let x2 = self.x_at(line, e);
                out.push((x1, li as f32 * self.line_height, x2 - x1, self.line_height));
            }
        }
        out
    }

    /// The word (or whitespace run) at a byte offset, by
    /// whitespace scan: the unit a double-click selects.
    pub fn word_range_at(&self, byte_offset: usize) -> Range<usize> {
        let text = &self.text;
        if text.is_empty() {
            return 0..0;
        }
        let chars: Vec<(usize, char)> = text.char_indices().collect();
        // The character the offset points at (or the last one,
        // when the offset is the end of the text).
        let idx = if byte_offset >= text.len() {
            chars.len() - 1
        } else {
            chars
                .iter()
                .position(|&(s, c)| byte_offset < s + c.len_utf8())
                .unwrap_or(chars.len() - 1)
        };
        let (_, ch) = chars[idx];
        let want_word = !ch.is_whitespace();
        let mut start = chars[idx].0;
        let mut end = chars[idx].0 + ch.len_utf8();
        for &(s, c) in chars.iter().rev() {
            if s >= start {
                continue;
            }
            if (!c.is_whitespace()) == want_word {
                start = s;
            } else {
                break;
            }
        }
        for &(s, c) in chars.iter() {
            if s < end {
                continue;
            }
            if (!c.is_whitespace()) == want_word {
                end = s + c.len_utf8();
            } else {
                break;
            }
        }
        start..end
    }
}

/// The effective flags in force at a byte offset: the span
/// covering it, else the last span that started before it,
/// else the plain face. Spans are sorted and non-overlapping
/// (callers build them from run lists).
fn flags_at(spans: &[RunSpan], offset: usize) -> (bool, bool) {
    let mut cur = (false, false);
    for s in spans {
        if offset >= s.range.start && offset < s.range.end {
            return (s.bold, s.italic);
        }
        if s.range.start <= offset {
            cur = (s.bold, s.italic);
        }
    }
    cur
}

/// Clusters of one exact line string, shaped standalone: the
/// line is built with no wrap width, so its single parley line
/// carries every cluster's range and advance. `spans` are
/// over the line text itself; each cluster takes the flags
/// of the span it starts in (a style boundary forces a run
/// break in parley, so a cluster never straddles two spans).
fn line_clusters(
    shaper: &mut Shaper,
    line_text: &str,
    spans: &[RunSpan],
    spec: &FontSpec,
    size: Mpt,
    leading_rel: f32,
) -> (Vec<ClusterGeom>, f32) {
    if line_text.is_empty() {
        return (Vec::new(), 0.0);
    }
    let layout = shaper.build_layout_spanned(line_text, spec, spans, size, leading_rel, None);
    let mut clusters = Vec::new();
    let mut advance = 0.0f32;
    if let Some(line) = layout.lines().next() {
        advance = line.metrics().advance;
        for run in line.runs() {
            for cluster in run.clusters() {
                let adv = cluster.advance();
                let range = cluster.text_range();
                let (bold, italic) = flags_at(spans, range.start);
                clusters.push(ClusterGeom {
                    range,
                    x: clusters
                        .last()
                        .map(|c: &ClusterGeom| c.x + c.width)
                        .unwrap_or(0.0),
                    width: adv,
                    bold,
                    italic,
                });
            }
        }
    }
    (clusters, advance)
}

/// Map the spans of a RESOLVED leader line ("Label\tPage 3")
/// onto its EXPANDED form ("Label ...... Page 3", the shape
/// `expand_order_line` builds): the left words keep their
/// spans, the right words keep theirs (shifted), and the
/// dots wear the face of the run that held the tab. If the
/// expanded text does not have the documented shape, one
/// span in the tab's face covers it: the words still print,
/// in one face, rather than not at all.
pub fn leader_run_spans(resolved: &str, spans: &[RunSpan], expanded: &str) -> Vec<RunSpan> {
    let Some(tab) = resolved.find('\t') else {
        return clip_spans(spans, 0..expanded.len().min(resolved.len()));
    };
    let (tab_bold, tab_italic) = flags_at(spans, tab);
    let left_vis = resolved[..tab].trim_end();
    let right_res = &resolved[tab + 1..];
    let right_vis = right_res.trim();
    let right_start_res = tab + 1 + (right_res.len() - right_res.trim_start().len());
    if !expanded.starts_with(left_vis)
        || right_vis.is_empty()
        || !expanded.ends_with(right_vis)
        || expanded.len() < left_vis.len() + right_vis.len()
    {
        return vec![RunSpan {
            range: 0..expanded.len(),
            bold: tab_bold,
            italic: tab_italic,
        }];
    }
    let right_start_exp = expanded.len() - right_vis.len();
    let mut out = clip_spans(spans, 0..left_vis.len());
    if right_start_exp > left_vis.len() {
        out.push(RunSpan {
            range: left_vis.len()..right_start_exp,
            bold: tab_bold,
            italic: tab_italic,
        });
    }
    let shift = right_start_exp as isize - right_start_res as isize;
    for s in clip_spans(spans, right_start_res..right_start_res + right_vis.len()) {
        let start = ((s.range.start + right_start_res) as isize).saturating_add(shift);
        let end = ((s.range.end + right_start_res) as isize).saturating_add(shift);
        out.push(RunSpan {
            range: start.max(0) as usize..end.max(0) as usize,
            bold: s.bold,
            italic: s.italic,
        });
    }
    out
}

fn align_offset(align: Align, frame_w: f32, line_w: f32) -> f32 {
    match align {
        Align::Left => 0.0,
        Align::Center => (frame_w - line_w) / 2.0,
        Align::Right => frame_w - line_w,
    }
}

/// One spanned layout request (health audit m-6): the
/// text, its effective face spans, the face it is set in,
/// its size and leading, the width it must fit, and its
/// alignment. The two `*_runs` layout entry points take
/// exactly this set, so it travels as one object instead
/// of seven positional arguments at every call site.
pub struct SpannedLayoutRequest<'a> {
    pub text: &'a str,
    pub spans: &'a [RunSpan],
    pub spec: &'a FontSpec,
    pub size: Mpt,
    pub leading_rel: f32,
    pub width: Mpt,
    pub align: Align,
}

impl Shaper {
    /// Lay a paragraph out for editing: display lines whose
    /// breaks are exactly `wrap`'s breaks (same parley pass,
    /// same strict re-break), each with cluster geometry.
    pub fn layout_paragraph(
        &mut self,
        text: &str,
        spec: &FontSpec,
        size: Mpt,
        leading_rel: f32,
        max_width: Mpt,
        align: Align,
    ) -> TextLayout {
        let spans = [RunSpan {
            range: 0..text.len(),
            bold: spec.bold,
            italic: spec.italic,
        }];
        self.layout_paragraph_runs(SpannedLayoutRequest {
            text,
            spans: &spans,
            spec,
            size,
            leading_rel,
            width: max_width,
            align,
        })
    }

    /// `layout_paragraph` for a paragraph whose face switches
    /// change mid-text (G1b): `spans` carry the EFFECTIVE
    /// flags over `text`. One parley layout for the whole
    /// paragraph, so the breaks are the breaks the same text
    /// would get unformatted except where the wider faces
    /// genuinely move a word; the strict re-break measures
    /// with the spans in force.
    pub fn layout_paragraph_runs(&mut self, req: SpannedLayoutRequest<'_>) -> TextLayout {
        let SpannedLayoutRequest {
            text,
            spans,
            spec,
            size,
            leading_rel,
            width: max_width,
            align,
        } = req;
        let line_height = size as f32 / 1000.0 * leading_rel;
        let frame_w = max_width as f32 / 1000.0;
        if text.is_empty() {
            return TextLayout {
                text: String::new(),
                lines: vec![LayoutLine {
                    range: 0..0,
                    width: 0.0,
                    text_advance: 0.0,
                    x0: align_offset(align, frame_w, 0.0),
                    clusters: Vec::new(),
                }],
                line_height,
            };
        }
        // Phase 1, parley's lines, exactly as wrap() reads them.
        let layout =
            self.build_layout_spanned(text, spec, spans, size, leading_rel, Some(max_width));
        let mut raw_lines: Vec<(Range<usize>, f32)> = Vec::new();
        for line in layout.lines() {
            let range = line.text_range();
            let slice = text.get(range.clone()).unwrap_or("").trim_end();
            raw_lines.push((
                range.start..range.start + slice.len(),
                line.metrics().advance,
            ));
        }
        if raw_lines.is_empty() {
            raw_lines.push((0..0, 0.0));
        }
        // Phase 2, the strict re-break, mirroring wrap() word
        // for word: an over-wide line that contains spaces is
        // re-broken greedily by shaped word widths. Each piece
        // is a contiguous slice of the line (words joined by
        // single spaces reproduce the slice exactly).
        let mut display: Vec<(Range<usize>, f32, bool)> = Vec::new(); // (range, width pt, rebroken?)
        for (range, width_pt) in raw_lines {
            let slice = &text[range.clone()];
            let width_mpt = (width_pt * 1000.0).round() as Mpt;
            if width_mpt <= max_width || !slice.contains(' ') {
                display.push((range, width_pt, false));
                continue;
            }
            // Word spans within the slice, split on single
            // spaces (empty spans for repeated spaces, as
            // wrap's split(' ') sees them).
            let mut words: Vec<Range<usize>> = Vec::new();
            let mut wstart = 0usize;
            for (i, b) in slice.bytes().enumerate() {
                if b == b' ' {
                    words.push(wstart..i);
                    wstart = i + 1;
                }
            }
            words.push(wstart..slice.len());
            let measured = |shaper: &mut Shaper, local: Range<usize>| -> Mpt {
                let global = range.start + local.start..range.start + local.end;
                let local_spans = clip_spans(spans, global);
                shaper.measure_spanned(&slice[local], &local_spans, spec, size)
            };
            let mut cur: Option<Range<usize>> = None;
            for w in words {
                let trial = match &cur {
                    None => w.clone(),
                    Some(c) => c.start..w.end,
                };
                if cur.is_some() && measured(self, trial.clone()) > max_width {
                    let c = cur.take().expect("cur checked");
                    let pw = measured(self, c.clone()) as f32 / 1000.0;
                    display.push((range.start + c.start..range.start + c.end, pw, true));
                    cur = Some(w);
                } else {
                    cur = Some(trial);
                }
            }
            if let Some(c) = cur {
                let pw = measured(self, c.clone()) as f32 / 1000.0;
                display.push((range.start + c.start..range.start + c.end, pw, true));
            }
        }
        // Phase 3: cluster geometry per display line, shaped
        // from the line's exact text.
        let mut lines = Vec::new();
        for (range, width_pt, _rebroken) in display {
            let line_text = &text[range.clone()];
            let line_spans = clip_spans(spans, range.clone());
            let (mut clusters, text_advance) =
                line_clusters(self, line_text, &line_spans, spec, size, leading_rel);
            for c in &mut clusters {
                c.range = c.range.start + range.start..c.range.end + range.start;
            }
            lines.push(LayoutLine {
                range,
                width: width_pt,
                text_advance,
                x0: align_offset(align, frame_w, width_pt),
                clusters,
            });
        }
        TextLayout {
            text: text.to_string(),
            lines,
            line_height,
        }
    }

    /// Lay out one pre-broken line (a dot-leader line produced
    /// by `expand_order_line`): no wrapping, one display line
    /// with full cluster geometry.
    pub fn layout_display_line(
        &mut self,
        text: &str,
        spec: &FontSpec,
        size: Mpt,
        leading_rel: f32,
        frame_width: Mpt,
        align: Align,
    ) -> TextLayout {
        let spans = [RunSpan {
            range: 0..text.len(),
            bold: spec.bold,
            italic: spec.italic,
        }];
        self.layout_display_line_runs(SpannedLayoutRequest {
            text,
            spans: &spans,
            spec,
            size,
            leading_rel,
            width: frame_width,
            align,
        })
    }

    /// `layout_display_line` with effective face spans over
    /// the (already expanded) line text.
    pub fn layout_display_line_runs(&mut self, req: SpannedLayoutRequest<'_>) -> TextLayout {
        let SpannedLayoutRequest {
            text,
            spans,
            spec,
            size,
            leading_rel,
            width: frame_width,
            align,
        } = req;
        let line_height = size as f32 / 1000.0 * leading_rel;
        let frame_w = frame_width as f32 / 1000.0;
        let (clusters, advance) = line_clusters(self, text, spans, spec, size, leading_rel);
        TextLayout {
            text: text.to_string(),
            lines: vec![LayoutLine {
                range: 0..text.len(),
                width: advance,
                text_advance: advance,
                x0: align_offset(align, frame_w, advance),
                clusters,
            }],
            line_height,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shaping::bundled_shaper;

    fn spec() -> FontSpec {
        FontSpec::new("Liberation Sans")
    }

    #[test]
    fn caret_and_hit_test_round_trip_on_two_lines() {
        let mut s = bundled_shaper();
        let text = "Welcome to Grace Lutheran Church. We are glad you are here with us this morning for worship.";
        let layout = s.layout_paragraph(text, &spec(), 11_000, 1.3, 180_000, Align::Left);
        assert!(
            layout.lines().len() >= 2,
            "expected 2+ lines, got {}",
            layout.lines().len()
        );
        // Breaks agree with wrap(), line for line.
        let wrapped = s.wrap(text, 180_000, &spec(), 11_000, 1.3);
        assert_eq!(layout.lines().len(), wrapped.len());
        for (ll, wl) in layout.lines().iter().zip(wrapped.iter()) {
            assert_eq!(&text[ll.range.clone()], wl.text);
        }
        // Caret at the very start and very end.
        let lh = layout.line_height();
        let (x, y, h) = layout.caret_rect(0);
        assert!((x - 0.0).abs() < 0.01 && (y - 0.0).abs() < 0.01 && (h - lh).abs() < 0.01);
        let last = layout.lines().len() - 1;
        let (_, y, _) = layout.caret_rect(text.len());
        assert!(
            (y - last as f32 * lh).abs() < 0.01,
            "end caret should sit on the last line"
        );
        // Round-trip: caret position -> hit test returns the
        // same offset, for cluster boundaries across both lines.
        let mut probes: Vec<usize> = vec![0, text.len()];
        for line in layout.lines() {
            probes.push(line.range.start);
            probes.push(line.range.end);
            for c in &line.clusters {
                probes.push(c.range.start);
            }
        }
        for off in probes {
            let (cx, cy, ch) = layout.caret_rect(off);
            let back = layout.byte_offset_at(cx, cy + ch / 2.0);
            assert_eq!(back, off, "round trip failed for offset {off}");
        }
    }

    #[test]
    fn dot_leader_line_has_full_geometry() {
        let mut s = bundled_shaper();
        let expanded = s.expand_order_line("Hymn of Praise\t432", 300_000, &spec(), 11_000);
        assert!(expanded.contains("..."));
        let layout = s.layout_display_line(&expanded, &spec(), 11_000, 1.3, 300_000, Align::Left);
        assert_eq!(layout.lines().len(), 1);
        let line = &layout.lines()[0];
        assert!(!line.clusters.is_empty(), "leader line must have clusters");
        // The clusters tile the whole expanded string.
        assert_eq!(line.clusters.first().unwrap().range.start, 0);
        assert_eq!(line.clusters.last().unwrap().range.end, expanded.len());
        // The right-hand number is where the geometry says it
        // is: hit-testing near the end finds its offset.
        let num_start = expanded.find("432").expect("number present");
        let (cx, cy, ch) = layout.caret_rect(num_start);
        assert!(cx > 200.0, "number sits far right, caret x was {cx}");
        assert_eq!(layout.byte_offset_at(cx, cy + ch / 2.0), num_start);
        // And the same line through layout_paragraph (the wide
        // width leaves it on one line) agrees.
        let lp = s.layout_paragraph(&expanded, &spec(), 11_000, 1.3, 300_000, Align::Left);
        assert_eq!(lp.lines().len(), 1);
        assert_eq!(lp.lines()[0].clusters.len(), line.clusters.len());
    }

    #[test]
    fn selection_rects_span_lines_and_word_scan_works() {
        let mut s = bundled_shaper();
        let text = "The quick brown fox jumps over the lazy dog again and again and again today.";
        let layout = s.layout_paragraph(text, &spec(), 11_000, 1.3, 150_000, Align::Left);
        assert!(layout.lines().len() >= 2);
        let full = layout.selection_rects(0..text.len());
        assert_eq!(full.len(), layout.lines().len());
        assert!(layout.selection_rects(5..5).is_empty());
        let w = layout.word_range_at(text.find("brown").unwrap() + 2);
        assert_eq!(&text[w], "brown");
    }

    #[test]
    fn centered_lines_carry_the_alignment_offset() {
        let mut s = bundled_shaper();
        let layout = s.layout_paragraph("Grace", &spec(), 11_000, 1.3, 300_000, Align::Center);
        let line = &layout.lines()[0];
        assert!(line.x0 > 100.0, "centered short line starts mid-frame");
        let (x, _, _) = layout.caret_rect(0);
        assert!((x - line.x0).abs() < 0.01);
        assert_eq!(layout.byte_offset_at(0.0, 1.0), 0);
    }

    #[test]
    fn runs_break_like_their_plain_concatenation() {
        // One layout for the whole paragraph: a bold word
        // inside a sentence takes the same line breaks the
        // plain sentence takes (at a width with slack), and
        // its clusters carry the bold flag.
        let mut s = bundled_shaper();
        let text = "Sing hymn 432 with glad and generous hearts today";
        let hymn = text.find("hymn").expect("word");
        let spans = vec![
            RunSpan {
                range: 0..hymn,
                bold: false,
                italic: false,
            },
            RunSpan {
                range: hymn..hymn + 4,
                bold: true,
                italic: false,
            },
            RunSpan {
                range: hymn + 4..text.len(),
                bold: false,
                italic: false,
            },
        ];
        let plain = s.layout_paragraph(text, &spec(), 12_000, 1.3, 170_000, Align::Left);
        let with_runs = s.layout_paragraph_runs(SpannedLayoutRequest {
            text,
            spans: &spans,
            spec: &spec(),
            size: 12_000,
            leading_rel: 1.3,
            width: 170_000,
            align: Align::Left,
        });
        let plain_ranges: Vec<Range<usize>> =
            plain.lines().iter().map(|l| l.range.clone()).collect();
        let run_ranges: Vec<Range<usize>> =
            with_runs.lines().iter().map(|l| l.range.clone()).collect();
        assert_eq!(plain_ranges, run_ranges, "run boundaries move no breaks");
        let mut saw_bold = false;
        let mut saw_plain = false;
        for line in with_runs.lines() {
            for c in &line.clusters {
                if c.range.start >= hymn && c.range.end <= hymn + 4 {
                    assert!(c.bold, "the hymn clusters are bold");
                    saw_bold = true;
                } else {
                    assert!(!c.bold, "the rest stays regular");
                    saw_plain = true;
                }
            }
        }
        assert!(saw_bold && saw_plain);
    }

    #[test]
    fn leader_spans_put_the_dots_in_the_tabs_face() {
        let resolved = "Hymn of Praise\t432";
        let tab = resolved.find('\t').expect("tab");
        let spans = vec![
            RunSpan {
                range: 0..tab,
                bold: false,
                italic: false,
            },
            RunSpan {
                range: tab..resolved.len(),
                bold: false,
                italic: true,
            },
        ];
        let expanded = "Hymn of Praise ......... 432";
        let out = leader_run_spans(resolved, &spans, expanded);
        // The left words are plain...
        assert!(out.iter().any(|s| s.range == (0..14) && !s.italic));
        // ...the dots wear the tab's (italic) face...
        let dots = out
            .iter()
            .find(|s| s.range.start == 14 && s.range.end < expanded.len())
            .expect("a dots span");
        assert!(dots.italic, "dots take the tab run's face");
        // ...and the number keeps its own italic span, shifted.
        assert!(out
            .iter()
            .any(|s| s.range == (expanded.len() - 3..expanded.len()) && s.italic));
    }
}
