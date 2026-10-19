//! Threaded layout (T1): one story flowing through many
//! boxes, each line drawn exactly once.
//!
//! A story may be named by several frames (Publisher calls
//! them linked boxes; the .pub reader recovers the links,
//! and the Pages window can make them by hand). This module
//! is the ONE place that decides which lines land in which
//! box: it walks the story's paragraphs in order and the
//! thread's frames in document order, laying each paragraph
//! out at the width of the box it starts in and cutting the
//! line list at each box's bottom. A paragraph that runs
//! past a box continues in the next box, re-wrapped at that
//! box's width from the exact byte the previous box stopped
//! at. The canvas, the print previews, the PDF writer, and
//! the Check step all read these slices, so no two surfaces
//! can disagree about where a story's words are.
//!
//! The walk's rules, shared with the single-box paths this
//! replaces:
//! - Fields are resolved first; layout works on the words
//!   that print, in resolved space.
//! - A paragraph's line height is its style size x 1.3, and
//!   its `space_after` is spent in the box the paragraph
//!   ENDS in.
//! - A tabbed paragraph (an order-of-service line) is one
//!   dot-leader line: it moves to the next box whole rather
//!   than splitting. A leader line wider than its box still
//!   draws as one line but is charged two line heights, so
//!   a too-wide hymn line can never pass a fit check sight
//!   unseen (J14).
//! - Lines that fit no box are counted as leftover, never
//!   drawn; Check names them.

use crate::shaping::{clip_spans, FontSpec, RunSpan, Shaper};
use crate::text_layout::TextLayout;
use ob_model::{Document, Mpt, Paragraph, ParagraphStyle, Rect, Story};
use std::collections::HashMap;
use std::ops::Range;

/// Relative leading every surface uses (the style size
/// times this is one line's height).
pub const LEADING: f32 = 1.3;

/// One piece of one paragraph, as one box draws it: the
/// piece's own layout (of the piece's text when the
/// paragraph was cut, of the whole paragraph when it was
/// not), where the piece sits in the paragraph, and where
/// its first line's box top is in panel coordinates.
#[derive(Debug, Clone)]
pub struct PlacedPara {
    /// Paragraph index within the story.
    pub para: usize,
    /// Layout of this piece. Line ranges index
    /// `layout.text()`, the piece's text.
    pub layout: TextLayout,
    /// The piece's byte range in the paragraph's RESOLVED
    /// text (the whole paragraph when it was not cut).
    pub text_range: Range<usize>,
    /// Panel-space y of the first line's box top.
    pub top: Mpt,
    /// This paragraph's line height.
    pub line_h: Mpt,
    /// The paragraph's style (the fallback body style when
    /// the document names none, as the draw paths have
    /// always used).
    pub style: ParagraphStyle,
}

/// What one box of a thread draws.
#[derive(Debug, Clone)]
pub struct FrameSlice {
    /// Where this box's text begins, as (paragraph index,
    /// byte offset in that paragraph's resolved text): the
    /// first piece starts there, and a box the story never
    /// reaches begins at the walk's position when it was
    /// passed (the story's end, when everything fit
    /// earlier). "Make this box start its own text" cuts
    /// the story here.
    pub starts_at: (usize, usize),
    pub paras: Vec<PlacedPara>,
}

/// A story laid out through all its boxes.
#[derive(Debug, Clone)]
pub struct StoryThread {
    /// The thread's frames, (panel index, frame index) in
    /// document order, parallel to `slices`.
    pub frames: Vec<(usize, usize)>,
    pub slices: Vec<FrameSlice>,
    /// Lines no box could hold, in walk order. They are
    /// not drawn anywhere; Check reports the count.
    pub leftover_lines: usize,
}

impl StoryThread {
    /// The slice one frame draws, by its document position.
    pub fn slice_for(&self, panel: usize, frame: usize) -> Option<&FrameSlice> {
        self.frames
            .iter()
            .position(|&f| f == (panel, frame))
            .map(|i| &self.slices[i])
    }

    /// A frame's place in the thread (0 is the first box).
    pub fn position(&self, panel: usize, frame: usize) -> Option<usize> {
        self.frames.iter().position(|&f| f == (panel, frame))
    }
}

/// The fallback style every draw path uses when a
/// paragraph names a style the document does not hold:
/// the same literals the canvas and the PDF writer have
/// always fallen back to.
fn style_or_body(doc: &Document, id: &str) -> ParagraphStyle {
    doc.style(id).cloned().unwrap_or_else(|| ParagraphStyle {
        id: "body".into(),
        name: "Body".into(),
        font_size: 11_000,
        bold: false,
        italic: false,
        align: ob_model::Align::Left,
        space_after: 4_000,
        color: ob_model::Rgb::BLACK,
        font_family: "Liberation Sans".into(),
        font_sha256: None,
    })
}

/// A paragraph made ready to lay out: resolved words, the
/// effective face spans over them (runs with the style
/// folded in), and the facts the walk needs.
struct PrepPara {
    style: ParagraphStyle,
    spec: FontSpec,
    resolved: String,
    spans: Vec<RunSpan>,
    tab: bool,
    line_h: Mpt,
}

fn prepare(doc: &Document, para: &Paragraph) -> PrepPara {
    let style = style_or_body(doc, &para.style);
    let resolved_runs = ob_model::resolve_runs(&para.runs, doc);
    let resolved: String = resolved_runs.iter().map(|r| r.text.as_str()).collect();
    let mut spans: Vec<RunSpan> = Vec::new();
    let mut at = 0usize;
    for r in &resolved_runs {
        spans.push(RunSpan {
            range: at..at + r.text.len(),
            bold: r.bold.unwrap_or(style.bold),
            italic: r.italic.unwrap_or(style.italic),
        });
        at += r.text.len();
    }
    PrepPara {
        spec: FontSpec {
            family: style.font_family.clone(),
            bold: style.bold,
            italic: style.italic,
        },
        tab: resolved.contains('\t'),
        line_h: style.font_size * 13 / 10,
        resolved,
        spans,
        style,
    }
}

/// Lay out one piece of a prepared paragraph: the text
/// from `start` to the paragraph's end, wrapped at
/// `width`. A tabbed paragraph ignores `start` (it is one
/// line, placed whole or not at all).
fn layout_piece(shaper: &mut Shaper, prep: &PrepPara, start: usize, width: Mpt) -> TextLayout {
    if prep.tab {
        let expanded =
            shaper.expand_order_line(&prep.resolved, width, &prep.spec, prep.style.font_size);
        let disp = crate::text_layout::leader_run_spans(&prep.resolved, &prep.spans, &expanded);
        return shaper.layout_display_line_runs(crate::text_layout::SpannedLayoutRequest {
            text: &expanded,
            spans: &disp,
            spec: &prep.spec,
            size: prep.style.font_size,
            leading_rel: LEADING,
            width,
            align: prep.style.align,
        });
    }
    let text = &prep.resolved[start..];
    let spans = clip_spans(&prep.spans, start..prep.resolved.len());
    shaper.layout_paragraph_runs(crate::text_layout::SpannedLayoutRequest {
        text,
        spans: &spans,
        spec: &prep.spec,
        size: prep.style.font_size,
        leading_rel: LEADING,
        width,
        align: prep.style.align,
    })
}

/// How many line heights a tabbed paragraph's one display
/// line costs at a width: two when the leader line measures
/// wider than the box (J14: a too-wide hymn line used to
/// pass every fit check sight unseen; it draws as one
/// line, but it is charged two line heights everywhere,
/// so the warning and the stacking agree).
fn tab_line_cost(shaper: &mut Shaper, prep: &PrepPara, width: Mpt) -> usize {
    let layout = layout_piece(shaper, prep, 0, width);
    let wide = layout
        .lines()
        .first()
        .map(|l| l.width > width as f32 / 1000.0)
        .unwrap_or(false);
    if wide {
        2
    } else {
        1
    }
}

/// How many lines a piece takes at a width: the leftover
/// count's ruler (the last box's width, as the walk uses).
fn count_lines(shaper: &mut Shaper, prep: &PrepPara, start: usize, width: Mpt) -> usize {
    if prep.tab {
        tab_line_cost(shaper, prep, width)
    } else {
        layout_piece(shaper, prep, start, width).lines().len()
    }
}

/// Lay one story out through its boxes. `frames` are the
/// thread's frames in document order (`Document::
/// frames_of_story` produces them); stale references are
/// dropped rather than panicking.
pub fn layout_story_thread(
    shaper: &mut Shaper,
    doc: &Document,
    story: &Story,
    frames: &[(usize, usize)],
) -> StoryThread {
    let mut refs: Vec<(usize, usize)> = Vec::new();
    let mut rects: Vec<Rect> = Vec::new();
    for &(pi, fi) in frames {
        if let Some(frame) = doc.panels.get(pi).and_then(|p| p.frames.get(fi)) {
            refs.push((pi, fi));
            rects.push(frame.rect);
        }
    }
    let mut slices: Vec<FrameSlice> = rects
        .iter()
        .map(|_| FrameSlice {
            starts_at: (0, 0),
            paras: Vec::new(),
        })
        .collect();
    if rects.is_empty() {
        return StoryThread {
            frames: refs,
            slices,
            leftover_lines: 0,
        };
    }
    let preps: Vec<PrepPara> = story.paragraphs.iter().map(|p| prepare(doc, p)).collect();
    let last_w = rects[rects.len() - 1].w;
    let mut leftover = 0usize;
    let mut entered = vec![false; rects.len()];
    entered[0] = true;
    let mut fi = 0usize;
    let mut y = rects[0].y;
    'paras: for (pi, prep) in preps.iter().enumerate() {
        let mut start = 0usize;
        loop {
            if fi >= rects.len() {
                leftover += count_lines(shaper, prep, start, last_w);
                for later in &preps[pi + 1..] {
                    leftover += count_lines(shaper, later, 0, last_w);
                }
                break 'paras;
            }
            let frame = rects[fi];
            let line_h = prep.line_h;
            if prep.tab {
                // One display line, placed whole or moved on
                // whole. A leader line wider than its box is
                // charged two line heights (see tab_line_cost).
                let cost = tab_line_cost(shaper, prep, frame.w) as Mpt * line_h;
                if y + cost <= frame.y + frame.h {
                    let layout = layout_piece(shaper, prep, 0, frame.w);
                    slices[fi].paras.push(PlacedPara {
                        para: pi,
                        layout,
                        text_range: 0..prep.resolved.len(),
                        top: y,
                        line_h,
                        style: prep.style.clone(),
                    });
                    y += cost + prep.style.space_after;
                    continue 'paras;
                }
                fi += 1;
                if fi < rects.len() {
                    y = rects[fi].y;
                    if !entered[fi] {
                        entered[fi] = true;
                        slices[fi].starts_at = (pi, 0);
                    }
                }
                continue;
            }
            let layout = layout_piece(shaper, prep, start, frame.w);
            let total = layout.lines().len();
            let mut fit = 0usize;
            let mut yy = y;
            while fit < total && yy + line_h <= frame.y + frame.h {
                yy += line_h;
                fit += 1;
            }
            if fit == total {
                slices[fi].paras.push(PlacedPara {
                    para: pi,
                    layout,
                    text_range: start..prep.resolved.len(),
                    top: y,
                    line_h,
                    style: prep.style.clone(),
                });
                y = yy + prep.style.space_after;
                continue 'paras;
            }
            if fit > 0 {
                let next_start = start + layout.lines()[fit].range.start;
                slices[fi].paras.push(PlacedPara {
                    para: pi,
                    layout: layout.first_lines(fit),
                    text_range: start..next_start,
                    top: y,
                    line_h,
                    style: prep.style.clone(),
                });
                start = next_start;
            }
            // This box can take no more of the paragraph
            // (full, or too short for one line): move on.
            fi += 1;
            if fi < rects.len() {
                y = rects[fi].y;
                if !entered[fi] {
                    entered[fi] = true;
                    slices[fi].starts_at = (pi, start);
                }
            }
        }
    }
    // Boxes the walk never reached begin at the story's
    // end: nothing is theirs yet.
    let end = (
        preps.len().saturating_sub(1),
        preps.last().map(|p| p.resolved.len()).unwrap_or(0),
    );
    for (i, was) in entered.iter().enumerate() {
        if !was {
            slices[i].starts_at = if preps.is_empty() { (0, 0) } else { end };
        }
    }
    StoryThread {
        frames: refs,
        slices,
        leftover_lines: leftover,
    }
}

/// Lay out every story that sits in at least one frame,
/// keyed by story id: the document's whole threading in
/// one pass, so a caller shaping many panels shapes each
/// story once.
pub fn layout_document_threads(
    shaper: &mut Shaper,
    doc: &Document,
) -> HashMap<String, StoryThread> {
    let mut out = HashMap::new();
    for story in &doc.stories {
        let frames = doc.frames_of_story(&story.id);
        if frames.is_empty() {
            continue;
        }
        let thread = layout_story_thread(shaper, doc, story, &frames);
        out.insert(story.id.clone(), thread);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shaping::bundled_shaper;
    use ob_model::{
        ChurchProfile, ContentRole, Frame, FrameKind, Panel, Paragraph, SheetSpec, Size,
    };

    /// A document with one panel holding the given frames
    /// and one story ("s") with the given paragraphs.
    fn doc_with(frames: Vec<(i64, i64, i64, i64)>, texts: Vec<&str>) -> (Document, Story) {
        let story = Story {
            id: "s".into(),
            paragraphs: texts.iter().map(|t| Paragraph::new("body", t)).collect(),
            keep_through: None,
        };
        let frames = frames
            .iter()
            .enumerate()
            .map(|(i, &(x, y, w, h))| Frame {
                id: format!("f{i}"),
                name: format!("Box {i}"),
                kind: FrameKind::Text,
                rect: Rect::new(x, y, w, h),
                story_id: Some("s".into()),
                role: ContentRole::Weekly,
                picture: None,
                music: None,
            })
            .collect();
        let doc = Document {
            format_version: ob_model::FORMAT_VERSION,
            title: "t".into(),
            profile: ChurchProfile::default(),
            fields: Default::default(),
            styles: ob_model::default_styles(),
            stories: vec![story.clone()],
            panels: vec![Panel {
                id: "p".into(),
                name: "Page 1".into(),
                size: Size::new(612_000, 792_000),
                frames,
                guides: Vec::new(),
                background_image: None,
            }],
            sheet: SheetSpec {
                preset: "letter-portrait".into(),
                size: Size::new(612_000, 792_000),
            },
            no_announcements: false,
            assets: Vec::new(),
            asset_bytes: Default::default(),
            occasion_time: None,
            wanted_fonts: Vec::new(),
            participants: Default::default(),
            inserts: Default::default(),
            applied_contributions: Vec::new(),
        };
        (doc, story)
    }

    const LONG: &str = "Welcome to worship this morning. We are glad you are here, \
        glad for the singing, glad for the readings, glad for the quiet, \
        and glad for the coffee after. Bring a friend next week.";

    /// Every line the thread draws, in order, as (piece
    /// text) per line.
    fn drawn_lines(thread: &StoryThread) -> Vec<String> {
        let mut out = Vec::new();
        for slice in &thread.slices {
            for piece in &slice.paras {
                for line in piece.layout.lines() {
                    out.push(piece.layout.text()[line.range.clone()].to_string());
                }
            }
        }
        out
    }

    #[test]
    fn lines_partition_exactly_across_two_boxes() {
        // Two narrow boxes, one long paragraph: the lines
        // in box 1 and box 2 together are the lines the
        // paragraph makes in one tall box, in order, with
        // nothing drawn twice and nothing skipped.
        let (doc, story) = doc_with(
            vec![(0, 0, 180_000, 60_000), (0, 100_000, 180_000, 400_000)],
            vec![LONG],
        );
        let mut shaper = bundled_shaper();
        let frames = doc.frames_of_story("s");
        let thread = layout_story_thread(&mut shaper, &doc, &story, &frames);
        assert_eq!(thread.leftover_lines, 0);
        let two = drawn_lines(&thread);
        assert!(two.len() > 3, "the text takes several lines: {two:?}");
        // Box 1 is short: exactly the lines that fit its
        // height (60pt / 14.3pt per line = 4).
        assert_eq!(thread.slices[0].paras.len(), 1);
        assert_eq!(thread.slices[0].paras[0].layout.lines().len(), 4);
        assert!(!thread.slices[1].paras[0].layout.lines().is_empty());
        // One tall box: the same lines, the same order.
        let (doc1, story1) = doc_with(vec![(0, 0, 180_000, 600_000)], vec![LONG]);
        let frames1 = doc1.frames_of_story("s");
        let one = layout_story_thread(&mut shaper, &doc1, &story1, &frames1);
        assert_eq!(drawn_lines(&one), two, "the partition is the line list");
        // Text ranges tile the paragraph with no overlap.
        let ranges: Vec<Range<usize>> = thread
            .slices
            .iter()
            .flat_map(|s| s.paras.iter().map(|p| p.text_range.clone()))
            .collect();
        assert_eq!(ranges[0].start, 0);
        for w in ranges.windows(2) {
            assert_eq!(w[0].end, w[1].start, "ranges are contiguous");
        }
        assert_eq!(ranges.last().unwrap().end, LONG.len());
        // Box 2 begins where box 1 stopped.
        assert_eq!(thread.slices[1].starts_at, (0, ranges[0].end));
    }

    #[test]
    fn one_box_behaves_as_the_plain_layout_always_did() {
        // A story in a single box: the thread's lines are
        // the lines a direct layout of the story makes,
        // stacked with the same arithmetic (line height,
        // then space_after) the draw paths used before
        // threading existed.
        let texts = vec![
            "Grace Lutheran Church",
            "Welcome to worship this morning, friends and neighbors all.",
            "Hymn of Praise\t432",
        ];
        let (doc, story) = doc_with(vec![(36_000, 36_000, 300_000, 500_000)], texts.clone());
        let mut shaper = bundled_shaper();
        let thread = layout_story_thread(&mut shaper, &doc, &story, &doc.frames_of_story("s"));
        assert_eq!(thread.leftover_lines, 0);
        assert_eq!(thread.slices.len(), 1);
        let slice = &thread.slices[0];
        assert_eq!(slice.paras.len(), 3);
        // Paragraph 2 starts after paragraph 1's lines and
        // its space_after: 1 line x 14_300 + 4_000.
        assert_eq!(slice.paras[0].top, 36_000);
        assert_eq!(slice.paras[1].top, 36_000 + 14_300 + 4_000);
        // Paragraph 3 (the tabbed line) likewise, after
        // paragraph 2 wraps at the box width.
        let p2_lines = slice.paras[1].layout.lines().len();
        assert_eq!(
            slice.paras[2].top,
            slice.paras[1].top + p2_lines as Mpt * 14_300 + 4_000
        );
        // The words drawn are the story's words, line by
        // line what wrap makes of them. (Body style:
        // Liberation Sans 11pt, neither bold nor italic.)
        let spec = crate::shaping::FontSpec {
            family: "Liberation Sans".into(),
            bold: false,
            italic: false,
        };
        let wrapped = shaper.wrap(texts[1], 300_000, &spec, 11_000, 1.3);
        let drawn: Vec<String> = slice.paras[1]
            .layout
            .lines()
            .iter()
            .map(|l| slice.paras[1].layout.text()[l.range.clone()].to_string())
            .collect();
        assert_eq!(
            drawn,
            wrapped.iter().map(|l| l.text.clone()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn a_story_taller_than_its_boxes_reports_the_leftover() {
        // Two short boxes and a long story: some lines fit
        // nowhere. They are counted, and only counted.
        let (doc, story) = doc_with(
            vec![(0, 0, 180_000, 30_000), (0, 100_000, 180_000, 30_000)],
            vec![LONG],
        );
        let mut shaper = bundled_shaper();
        let thread = layout_story_thread(&mut shaper, &doc, &story, &doc.frames_of_story("s"));
        let drawn = drawn_lines(&thread).len();
        let (doc1, story1) = doc_with(vec![(0, 0, 180_000, 600_000)], vec![LONG]);
        let one = layout_story_thread(&mut shaper, &doc1, &story1, &doc1.frames_of_story("s"));
        let total = drawn_lines(&one).len();
        assert!(thread.leftover_lines > 0, "some lines fit nowhere");
        assert_eq!(
            drawn + thread.leftover_lines,
            total,
            "drawn plus leftover is every line"
        );
        // Each short box holds 2 lines (30pt / 14.3).
        assert_eq!(drawn, 4);
    }

    #[test]
    fn a_too_wide_leader_line_costs_two_line_heights() {
        // J14: a hymn line wider than its box draws as one
        // leader line but is charged two line heights, so
        // the paragraph after it stacks two lines down, and
        // a box too short for the charge reports the
        // overflow instead of passing the line sight unseen.
        let wide = "Hymn of Praise and Thanksgiving for All the Saints\t1234";
        let (doc, story) = doc_with(vec![(0, 0, 72_000, 400_000)], vec![wide, "After the hymn."]);
        let mut shaper = bundled_shaper();
        let thread = layout_story_thread(&mut shaper, &doc, &story, &doc.frames_of_story("s"));
        assert_eq!(thread.leftover_lines, 0);
        let slice = &thread.slices[0];
        assert_eq!(
            slice.paras[0].layout.lines().len(),
            1,
            "still one drawn line"
        );
        assert_eq!(
            slice.paras[1].top,
            2 * 14_300 + 4_000,
            "the next paragraph stacks below the two-height charge"
        );
        // A box with room for one line height only: the
        // leader line fits nowhere and counts as leftover.
        let (doc2, story2) = doc_with(vec![(0, 0, 72_000, 15_000)], vec![wide]);
        let thread2 = layout_story_thread(&mut shaper, &doc2, &story2, &doc2.frames_of_story("s"));
        assert_eq!(thread2.leftover_lines, 2);
        assert!(thread2.slices[0].paras.is_empty());
    }

    #[test]
    fn paragraphs_flow_in_order_and_space_after_stays_with_the_ender() {
        // Box 1 fits paragraph 1 exactly; paragraph 2
        // starts in box 2, at its top.
        let (doc, story) = doc_with(
            vec![(0, 0, 300_000, 15_000), (0, 100_000, 300_000, 300_000)],
            vec!["Short first.", "Second paragraph follows here."],
        );
        let mut shaper = bundled_shaper();
        let thread = layout_story_thread(&mut shaper, &doc, &story, &doc.frames_of_story("s"));
        assert_eq!(thread.leftover_lines, 0);
        assert_eq!(thread.slices[0].paras.len(), 1);
        assert_eq!(thread.slices[0].paras[0].para, 0);
        assert_eq!(thread.slices[1].paras.len(), 1);
        assert_eq!(thread.slices[1].paras[0].para, 1);
        assert_eq!(thread.slices[1].paras[0].top, 100_000);
        assert_eq!(thread.slices[1].starts_at, (1, 0));
    }
}
