//! Preview and editor layout machinery: the shaped
//! lines, sheets, and pictures the canvas draws, and
//! the font rows they are drawn from.

use crate::*;

/// One row of the Fonts panel: a library font with everything the
/// admin needs (its own face for the preview, its name, and what its
/// license allows in plain words).
#[derive(Clone)]
pub(crate) struct FontRow {
    pub(crate) key: String,
    pub(crate) family: String,
    pub(crate) full_name: String,
    pub(crate) rights_words: String,
}

/// The shaped-lines cache: per-panel preview lines plus
/// the document's thread layouts (T1), all keyed by the
/// document snapshot, so the canvas, the Check step, and
/// the print preview all read the same layout of a story
/// through its boxes.
pub(crate) struct LinesCache {
    pub(crate) key: String,
    pub(crate) lines: std::collections::HashMap<usize, Vec<PreviewLine>>,
    pub(crate) threads: std::collections::HashMap<String, ob_text::threading::StoryThread>,
}

/// One sheet side as the printer will produce it: panel rectangles
/// in points, with the text lines shaped exactly as the PDF shapes
/// them (same shaper, same fonts), so the breaks shown are the
/// breaks printed.
#[derive(Clone)]
pub(crate) struct PreviewSheet {
    pub(crate) w: f32,
    pub(crate) h: f32,
    pub(crate) panels: Vec<PreviewPanel>,
}

/// One placed picture in a preview: its content key (the
/// asset fingerprint), the box it sits in, and the rect it
/// actually shows in, sheet points.
#[derive(Clone)]
pub(crate) struct PreviewPicture {
    pub(crate) asset: String,
    pub(crate) clip: egui::Rect,
    pub(crate) shown: egui::Rect,
    pub(crate) fit: ob_model::PictureFit,
}

#[derive(Clone)]
pub(crate) struct PreviewPanel {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) w: f32,
    pub(crate) h: f32,
    pub(crate) name: String,
    pub(crate) lines: Vec<PreviewLine>,
    /// Placed pictures, painted under the panel's words.
    pub(crate) pictures: Vec<PreviewPicture>,
}

#[derive(Clone)]
pub(crate) struct PreviewLine {
    pub(crate) text: String,
    pub(crate) size: f32,
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) w: f32,
    pub(crate) align: ob_model::Align,
    /// The face this line prints in, carried so the previews
    /// can draw it in that face too (UI audit round 2,
    /// batch 3): the style's font family name plus its
    /// bold and italic switches.
    pub(crate) font_family: String,
    pub(crate) bold: bool,
    pub(crate) italic: bool,
    /// Same-face segments of the line (G1b): a bold word
    /// inside the line draws in the bold face. `dx` is the
    /// segment's offset from the line's left edge, in points.
    /// Empty means "the whole line in the line's own face"
    /// (the editor's probe lines carry none).
    pub(crate) segs: Vec<PreviewSeg>,
    /// This line starts a spoken-only paragraph: the Pages
    /// canvas tags it "SPOKEN ONLY" above the line, so the
    /// state is visible on the page and never hidden. Only
    /// the canvas paints the tag, never the print previews:
    /// spoken-only words are not in any printed copy.
    pub(crate) spoken_tag: bool,
    /// This line starts a not-for-web paragraph (R-2b):
    /// the Pages canvas tags it "NOT FOR WEB" above the
    /// line, on the same terms as the spoken tag. The
    /// words DO print; only the web page leaves them out,
    /// and no printed copy carries the tag.
    pub(crate) web_tag: bool,
}

/// One same-face piece of a preview line (G1b).
#[derive(Clone)]
pub(crate) struct PreviewSeg {
    pub(crate) text: String,
    pub(crate) dx: f32,
    pub(crate) bold: bool,
    pub(crate) italic: bool,
}

/// Convert one chip-free display piece to its raw range,
/// through resolved space, and push it if it holds words.
pub(crate) fn push_raw_piece(
    map: &editmap::BufferMap,
    start: usize,
    end: usize,
    out: &mut Vec<std::ops::Range<usize>>,
) {
    if start >= end {
        return;
    }
    let rs = map.display_to_resolved(start);
    let re = map.display_to_resolved(end);
    let raws = map.resolved_to_raw(rs);
    let rawe = map.resolved_to_raw(re);
    if raws < rawe {
        out.push(raws..rawe);
    }
}

/// The effective face spans of a paragraph's RESOLVED runs:
/// each run's flags with the paragraph style folded in
/// (`run.bold.unwrap_or(style.bold)`), over the resolved
/// text. Shared by the preview builder and the editor.
pub(crate) fn effective_spans(
    runs: &[ob_model::Run],
    style: &ob_model::ParagraphStyle,
) -> Vec<ob_text::shaping::RunSpan> {
    let mut spans = Vec::new();
    let mut at = 0usize;
    for r in runs {
        spans.push(ob_text::shaping::RunSpan {
            range: at..at + r.text.len(),
            bold: r.bold.unwrap_or(style.bold),
            italic: r.italic.unwrap_or(style.italic),
        });
        at += r.text.len();
    }
    spans
}

/// Group a shaped line's clusters into same-face segments
/// (G1b), in drawing order; each segment starts at its first
/// cluster's position (the line's alignment offset folded
/// in). Clusters in a line are adjacent by construction, so
/// equal-flag neighbors simply extend the open segment.
pub(crate) fn line_segments(
    line: &ob_text::text_layout::LayoutLine,
    text: &str,
) -> Vec<PreviewSeg> {
    let mut segs: Vec<PreviewSeg> = Vec::new();
    for c in &line.clusters {
        let seg_text = &text[c.range.clone()];
        match segs.last_mut() {
            Some(last) if last.bold == c.bold && last.italic == c.italic => {
                last.text.push_str(seg_text);
            }
            _ => segs.push(PreviewSeg {
                text: seg_text.to_string(),
                dx: line.x0 + c.x,
                bold: c.bold,
                italic: c.italic,
            }),
        }
    }
    segs
}

/// One paragraph of the shaped canvas editor (E1): its
/// parley layout in DISPLAY space plus where its slice
/// sits inside the entered frame, stacked with the same
/// arithmetic the thread layout stacks the idle preview.
pub(crate) struct EditorPara {
    pub(crate) layout: ob_text::text_layout::TextLayout,
    /// Top of the first DRAWN line box, frame-local
    /// milli-points.
    pub(crate) top: ob_model::Mpt,
    /// Line height in milli-points (style size x 1.3).
    pub(crate) line_h: ob_model::Mpt,
    /// Lines of this paragraph that sit in EARLIER boxes of
    /// the thread (T1): the session shows the slice starting
    /// at the entered box, so these lines are not drawn
    /// here. Always 0 for a story in one box.
    pub(crate) skip: usize,
    /// Lines drawn in the entered box, starting at `skip`.
    /// Lines before and after exist for caret math but are
    /// not drawn: the same clipping the idle canvas applies,
    /// extended across the thread.
    pub(crate) drawn: usize,
    pub(crate) size: f32,
    pub(crate) font_family: String,
    pub(crate) bold: bool,
    pub(crate) italic: bool,
    pub(crate) align: ob_model::Align,
}

/// The shaped editor's geometry for one buffer state: the
/// raw/resolved/display map plus one layout per paragraph.
pub(crate) struct EditorLayout {
    pub(crate) key: String,
    pub(crate) map: editmap::BufferMap,
    pub(crate) paras: Vec<EditorPara>,
    /// The display offset where the entered box's slice of
    /// the story begins (T1): the first drawn line's start.
    /// None when no line is drawn in the entered box.
    pub(crate) slice_start: Option<usize>,
}

/// The egui family key for the library face a preview line
/// prints in. Every library file registers as its own egui
/// family (keyed by its hash), so the face is chosen among
/// the rows of the line's family by the face's full name:
/// the exact face when the library holds it (Bold, Italic,
/// Bold Italic), else the family's regular face. A family
/// the library does not hold paints in the resolution
/// chain's stand-in (docs/FONTS.md, wanted fonts), the
/// same face shaping and print use, so the screen never
/// shows a different font than the paper gets. None when
/// no face answers at all; the caller then falls back to
/// the interface font.
pub(crate) fn preview_face_key(
    rows: &[FontRow],
    family: &str,
    bold: bool,
    italic: bool,
) -> Option<String> {
    if let Some(key) = face_key_in_rows(rows, family, bold, italic) {
        return Some(key);
    }
    let mut available: Vec<String> = Vec::new();
    for row in rows {
        if !available.contains(&row.family) {
            available.push(row.family.clone());
        }
    }
    let resolved = ob_fonts::resolve::resolve_family(family, &available).family;
    if resolved != family {
        return face_key_in_rows(rows, &resolved, bold, italic);
    }
    None
}

/// The face-key lookup itself: among the rows of one
/// family, the exact face by full name, else the regular
/// face, else any face of the family.
pub(crate) fn face_key_in_rows(
    rows: &[FontRow],
    family: &str,
    bold: bool,
    italic: bool,
) -> Option<String> {
    let mut regular: Option<&FontRow> = None;
    let mut any: Option<&FontRow> = None;
    for row in rows {
        if !row.family.eq_ignore_ascii_case(family) {
            continue;
        }
        any = any.or(Some(row));
        let name = row.full_name.to_lowercase();
        let has_bold = name.contains("bold");
        let has_italic = name.contains("italic") || name.contains("oblique");
        if has_bold == bold && has_italic == italic {
            return Some(row.key.clone());
        }
        if !has_bold && !has_italic {
            regular = regular.or(Some(row));
        }
    }
    regular.or(any).map(|row| row.key.clone())
}

/// The named families a set of font definitions binds.
/// Painting may only name families in this set: egui
/// panics on a FontId whose family the ACTIVE definitions
/// do not bind ("is not bound to any fonts"), and new
/// definitions take a frame to become active, so a freshly
/// registered library family is unbound for exactly one
/// frame (round 2, NOTE 9: the first frame after
/// refresh_fonts painted preview lines in library faces the
/// still-default definitions had never heard of, and the
/// app died at startup).
pub fn bound_name_families(defs: &egui::FontDefinitions) -> std::collections::HashSet<String> {
    defs.families
        .keys()
        .filter_map(|f| match f {
            egui::FontFamily::Name(name) => Some(name.to_string()),
            _ => None,
        })
        .collect()
}

/// The font a preview line is drawn with: its own library
/// face when the library holds it AND that face's family is
/// bound in the definitions active this frame; otherwise
/// the interface font. The bound check is the NOTE 9 rule:
/// never hand the painter a family name the active
/// definitions do not bind.
pub(crate) fn line_font_id(
    rows: &[FontRow],
    bound: &std::collections::HashSet<String>,
    line: &PreviewLine,
    size: f32,
) -> egui::FontId {
    match preview_face_key(rows, &line.font_family, line.bold, line.italic) {
        Some(key) if bound.contains(&key) => {
            egui::FontId::new(size, egui::FontFamily::Name(key.into()))
        }
        _ => egui::FontId::proportional(size),
    }
}

/// Paint one preview line through `place` (a line-local x in
/// points to the screen point at the line's baseline). With
/// segments (G1b), each same-face piece draws in its own
/// face at its own offset, the offsets already carrying the
/// alignment; without them, the whole line draws in the
/// line's face with the alignment anchor, as before.
pub(crate) fn paint_line_segments(
    painter: &egui::Painter,
    rows: &[FontRow],
    bound: &std::collections::HashSet<String>,
    line: &PreviewLine,
    scale: f32,
    place: impl Fn(f32) -> egui::Pos2,
) {
    let size = (line.size * scale).max(4.0);
    if line.segs.is_empty() {
        let anchor = match line.align {
            ob_model::Align::Left => egui::Align2::LEFT_BOTTOM,
            ob_model::Align::Center => egui::Align2::CENTER_BOTTOM,
            ob_model::Align::Right => egui::Align2::RIGHT_BOTTOM,
        };
        let lx = match line.align {
            ob_model::Align::Left => line.x,
            ob_model::Align::Center => line.x + line.w / 2.0,
            ob_model::Align::Right => line.x + line.w,
        };
        painter.text(
            place(lx),
            anchor,
            &line.text,
            line_font_id(rows, bound, line, size),
            egui::Color32::BLACK,
        );
        return;
    }
    for seg in &line.segs {
        let probe = PreviewLine {
            text: seg.text.clone(),
            size: line.size,
            x: line.x,
            y: line.y,
            w: line.w,
            align: line.align,
            font_family: line.font_family.clone(),
            bold: seg.bold,
            italic: seg.italic,
            segs: Vec::new(),
            spoken_tag: false,
            web_tag: false,
        };
        painter.text(
            place(line.x + seg.dx),
            egui::Align2::LEFT_BOTTOM,
            &seg.text,
            line_font_id(rows, bound, &probe, size),
            egui::Color32::BLACK,
        );
    }
}

/// The font definitions and the Fonts panel rows for the
/// whole library, built together so the two can never
/// disagree: every row's key is a family the definitions
/// bind (round 2, NOTE 9's registration rule).
pub(crate) fn library_font_setup(
    lib: &ob_fonts::FontLibrary,
) -> (egui::FontDefinitions, Vec<FontRow>) {
    let mut defs = egui::FontDefinitions::default();
    let mut rows = Vec::new();
    for info in lib.scan() {
        let Ok(bytes) = lib.read(&info) else { continue };
        let key = format!("lib-{}", &info.sha256[..12]);
        defs.font_data
            .insert(key.clone(), egui::FontData::from_owned(bytes));
        defs.families.insert(
            egui::FontFamily::Name(key.clone().into()),
            vec![key.clone()],
        );
        rows.push(FontRow {
            key,
            family: info.family.clone(),
            full_name: info.full_name.clone(),
            rights_words: info.rights.plain_words().to_string(),
        });
    }
    (defs, rows)
}

/// A message back from a catalog worker thread (search or install).
pub(crate) enum CatalogMsg {
    Hits(Vec<ob_fonts::catalog::CatalogEntry>),
    Installed(Vec<String>),
    PackDone {
        installed: Vec<String>,
        failed: Vec<(String, String)>,
    },
    Failed(String),
}
