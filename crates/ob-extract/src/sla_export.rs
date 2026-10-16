//! Scribus (.sla) export: an OpenBulletin document leaves
//! as a Scribus file, so a Scribus shop can take a bulletin
//! the other way (Jacob's import/export hub: out in every
//! format we take in, where anyone can write the format at
//! all). This closes the last cell of the matrix: Scribus
//! now goes both ways.
//!
//! The file is written in the Scribus 1.5/1.6 dialect our
//! own importer (sla.rs) reads, which was modeled on two
//! genuine Scribus files from the ScribusGenerator project
//! (Next_Record.sla, Scribus 1.5.6.1, and DynamicOutFile.sla,
//! 1.5.3): a SCRIBUSUTF8NEW root, a DOCUMENT carrying the
//! page size and default font, STYLE definitions, PAGE
//! elements at origins in document coordinates, and
//! PAGEOBJECT text frames (PTYPE 4) whose XPOS/YPOS are
//! document coordinates, each holding a StoryText of ITEXT
//! runs closed by para markers and a final trail.
//!
//! Format decisions, and what arbitrated them:
//! - Pages are laid side by side in one row, the first at
//!   (100, 20), each next page starting where the previous
//!   one ends. That is the genuine file's own arrangement
//!   (Next_Record.sla's second page begins exactly one page
//!   width after the first), and our importer places frames
//!   by subtracting each page's origin, so any consistent
//!   origins round-trip.
//! - Scribus has no bold or italic switch: the face name
//!   carries them ("Liberation Sans Bold"), exactly as our
//!   importer reads them back, so the exporter appends the
//!   style words to the family name. Since X1 that is done
//!   per run, not per paragraph: each run of a paragraph
//!   names its own effective face on its own ITEXT chunk,
//!   so bold and italic spans inside a paragraph survive.
//!   (Before X1 the runs flattened to the style's face.)
//! - Every ITEXT run carries its FONT and FONTSIZE
//!   explicitly, as the genuine file's differing runs do.
//!   Non-final paragraphs also name their paragraph style
//!   through the closing para's PARENT, and every document
//!   style is declared as a STYLE element, so a real Scribus
//!   sees named styles. The final paragraph's style rides
//!   on its runs alone: in the genuine file the closing
//!   trail is bare and names no style, so the exporter
//!   follows the file, not our importer's convenience.
//! - The root's Version is "1.5.6.1", the version of the
//!   genuine file this dialect was modeled on. The label
//!   names the dialect generation, nothing more.
//! - Alignment is written on the STYLE elements (Scribus
//!   ALIGN: 0 left, 1 center, 2 right). It cannot
//!   round-trip through our importer because the shared
//!   dump schema has no alignment field; the same limit is
//!   recorded for IDML export.

use ob_model::{Align, Document, FrameKind, ParagraphStyle};

/// What the export carried, and what it had to leave behind,
/// in counts and plain words. Nothing is dropped silently:
/// the app shows this to the user next to the saved file.
/// The fields mirror the IDML export report so both doors
/// account the same way.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct SlaExportReport {
    /// Pages written.
    pub pages: usize,
    /// Text frames written, each with its own StoryText.
    pub text_frames: usize,
    /// Paragraphs written.
    pub paragraphs: usize,
    /// Shape frames (rectangles, placeholders): Scribus
    /// export does not carry them in this version.
    pub shape_frames_left_behind: usize,
    /// Picture frames: Scribus export does not carry placed
    /// pictures in this version; the count is stated.
    pub picture_frames_left_behind: usize,
    /// Panels whose tracing-guide picture was not carried.
    /// Those pictures are editing guides that never print, so
    /// leaving them is correct, but the count is still stated.
    pub tracing_pictures_left_behind: usize,
    /// Text frames whose story id names no story in the
    /// document: written empty rather than guessed at.
    pub frames_with_missing_story: usize,
    /// Frames that share one story with an earlier frame.
    /// Scribus can link frames so one story flows through
    /// them; our importer reads a frame's story whole, so
    /// the export gives each frame its own copy of the story
    /// text and counts the repeats here.
    pub stories_repeated_for_threaded_frames: usize,
    /// Paragraph style ids no style in the document defines:
    /// the words still export, set in the default look.
    pub paragraphs_with_unknown_style: usize,
    /// Runs whose effective face the written face name
    /// cannot state exactly: the family name itself carries
    /// the word "Bold" or "Italic", and Scribus reads the
    /// name, so a run that wants that switch OFF in such a
    /// family reads back ON. The words are all written;
    /// the count is stated rather than the run silently
    /// flattened (X1).
    pub runs_with_unstatable_face: usize,
    /// Plain-words notes, one per kind of loss.
    pub notes: Vec<String>,
}

fn esc_text(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn esc_attr(s: &str) -> String {
    // Character references for the whitespace XML attribute
    // normalization would otherwise flatten, so story text
    // survives exactly.
    esc_text(s)
        .replace('"', "&quot;")
        .replace('\t', "&#9;")
        .replace('\n', "&#10;")
        .replace('\r', "&#13;")
}

/// Milli-points to the plain decimal points Scribus writes.
fn pt(mpt: i64) -> String {
    let v = mpt as f64 / 1000.0;
    if (v - v.round()).abs() < 0.000_5 {
        format!("{}", v.round() as i64)
    } else {
        format!("{v:.3}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

/// A point value already in points (page origins, document
/// coordinates), formatted the same way.
fn ptf(v: f64) -> String {
    if (v - v.round()).abs() < 0.000_5 {
        format!("{}", v.round() as i64)
    } else {
        format!("{v:.3}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

fn has_word(face: &str, word: &str) -> bool {
    face.split(|c: char| !c.is_alphanumeric())
        .any(|w| w.eq_ignore_ascii_case(word))
}

/// The Scribus face name for one effective look: the
/// family plus the style words, since the format has no
/// bold/italic switch. Words the family already carries
/// are not repeated.
fn face_name_for(family: &str, bold: bool, italic: bool) -> String {
    let mut face = family.to_string();
    if bold && !has_word(&face, "bold") {
        face.push_str(" Bold");
    }
    if italic && !has_word(&face, "italic") {
        face.push_str(" Italic");
    }
    face
}

/// The Scribus face name for a paragraph style.
fn face_name(style: &ParagraphStyle) -> String {
    face_name_for(&style.font_family, style.bold, style.italic)
}

/// What our importer reads back from a face name (sla.rs's
/// face_is_bold / face_is_italic, mirrored so the exporter
/// can tell when a name cannot state a run's switches
/// exactly: a family that itself carries the word reads
/// that way no matter what the run wants).
fn face_reads_bold(face: &str) -> bool {
    has_word(face, "bold")
}

fn face_reads_italic(face: &str) -> bool {
    has_word(face, "italic") || has_word(face, "oblique")
}

/// Scribus paragraph alignment numbers.
fn align_num(align: Align) -> u8 {
    match align {
        Align::Left => 0,
        Align::Center => 1,
        Align::Right => 2,
    }
}

/// The name a paragraph style goes by in the file: the
/// style's own name, or its id when the name is blank.
fn style_label(style: &ParagraphStyle) -> String {
    if style.name.trim().is_empty() {
        style.id.clone()
    } else {
        style.name.clone()
    }
}

/// The look a paragraph exports with: its style's face and
/// size, or the document default for an unknown style id
/// (counted in the report).
struct ParaLook {
    face: String,
    size_pt: String,
    /// The paragraph style's file name, when the style is
    /// known; named by the closing para marker.
    label: Option<String>,
    /// The bare family the face was built from, and the
    /// style's own bold/italic switches: a run's face is
    /// this family plus the words for the run's effective
    /// switches (X1). For an unknown style id these are the
    /// default look's, the same source `face` falls back to.
    family: String,
    bold: bool,
    italic: bool,
}

/// Export the document as a Scribus .sla file. Returns the
/// file bytes and the report of what was not carried.
pub fn export_sla(doc: &Document) -> Result<(Vec<u8>, SlaExportReport), String> {
    // Spoken-only paragraphs never publish (the one rule,
    // ob_model::Document::without_spoken_only); the note
    // names what was left out, like every other count.
    let (filtered, spoken_left_out) = doc.without_spoken_only();
    let doc = &filtered;
    let mut report = SlaExportReport {
        pages: doc.panels.len(),
        ..Default::default()
    };
    if spoken_left_out > 0 {
        report.notes.push(
            ob_model::spoken_only_left_out_sentence(spoken_left_out)
                .trim()
                .trim_end_matches('.')
                .to_string(),
        );
    }

    // The document default look: the first style's, or the
    // genuine file's own defaults when the document defines
    // no styles at all.
    let (default_face, default_size_mpt) = match doc.styles.first() {
        Some(st) => (face_name(st), st.font_size),
        None => ("Liberation Serif".to_string(), 12_000),
    };
    // The default look as a family and switches too, for
    // the runs of a paragraph whose style is unknown.
    let (default_family, default_bold, default_italic) = match doc.styles.first() {
        Some(st) => (st.font_family.clone(), st.bold, st.italic),
        None => ("Liberation Serif".to_string(), false, false),
    };
    let look_of = |doc: &Document, style_id: &str, report: &mut SlaExportReport| -> ParaLook {
        match doc.style(style_id) {
            Some(st) => ParaLook {
                face: face_name(st),
                size_pt: pt(st.font_size),
                label: Some(style_label(st)),
                family: st.font_family.clone(),
                bold: st.bold,
                italic: st.italic,
            },
            None => {
                report.paragraphs_with_unknown_style += 1;
                ParaLook {
                    face: default_face.clone(),
                    size_pt: pt(default_size_mpt),
                    label: None,
                    family: default_family.clone(),
                    bold: default_bold,
                    italic: default_italic,
                }
            }
        }
    };

    // Page origins in document coordinates: one row, first
    // page at (100, 20), each next page where the last ended.
    let mut origins: Vec<(f64, f64)> = Vec::new();
    let mut cursor_x = 100.0f64;
    for panel in &doc.panels {
        origins.push((cursor_x, 20.0));
        cursor_x += panel.size.w as f64 / 1000.0;
    }
    let (doc_w, doc_h) = doc
        .panels
        .first()
        .map(|p| (pt(p.size.w), pt(p.size.h)))
        .unwrap_or_else(|| ("612".to_string(), "792".to_string()));

    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str("<SCRIBUSUTF8NEW Version=\"1.5.6.1\">\n");
    out.push_str(&format!(
        "<DOCUMENT ANZPAGES=\"{}\" PAGEWIDTH=\"{}\" PAGEHEIGHT=\"{}\" DFONT=\"{}\" DSIZE=\"{}\">\n",
        doc.panels.len(),
        doc_w,
        doc_h,
        esc_attr(&default_face),
        pt(default_size_mpt),
    ));

    // One STYLE per document style, its look on the CSTYLE
    // child, as the genuine file's Heading style carries it.
    for st in &doc.styles {
        out.push_str(&format!(
            "<STYLE NAME=\"{}\" ALIGN=\"{}\"><CSTYLE FONT=\"{}\" FONTSIZE=\"{}\"/></STYLE>\n",
            esc_attr(&style_label(st)),
            align_num(st.align),
            esc_attr(&face_name(st)),
            pt(st.font_size),
        ));
    }

    // Pages first, then the objects that sit on them.
    for (i, panel) in doc.panels.iter().enumerate() {
        let (ox, oy) = origins[i];
        out.push_str(&format!(
            "<PAGE PAGEXPOS=\"{}\" PAGEYPOS=\"{}\" PAGEWIDTH=\"{}\" PAGEHEIGHT=\"{}\" NUM=\"{}\" MNAM=\"Normal\"/>\n",
            ptf(ox),
            ptf(oy),
            pt(panel.size.w),
            pt(panel.size.h),
            i,
        ));
    }

    let mut stories_seen: Vec<String> = Vec::new();
    for (i, panel) in doc.panels.iter().enumerate() {
        let (ox, oy) = origins[i];
        for frame in &panel.frames {
            match frame.kind {
                FrameKind::Shape => report.shape_frames_left_behind += 1,
                FrameKind::Picture => report.picture_frames_left_behind += 1,
                FrameKind::Text => {
                    let Some(story_id) = &frame.story_id else {
                        // A text frame with no story is an
                        // empty box; there are no words to
                        // carry. The IDML export treats it
                        // the same way.
                        continue;
                    };
                    report.text_frames += 1;
                    let x = ox + frame.rect.x as f64 / 1000.0;
                    let y = oy + frame.rect.y as f64 / 1000.0;
                    out.push_str(&format!(
                        "<PAGEOBJECT XPOS=\"{}\" YPOS=\"{}\" OwnPage=\"{}\" PTYPE=\"4\" WIDTH=\"{}\" HEIGHT=\"{}\" ANNAME=\"{}\">\n<StoryText>\n",
                        ptf(x),
                        ptf(y),
                        i,
                        pt(frame.rect.w),
                        pt(frame.rect.h),
                        esc_attr(&frame.name),
                    ));
                    match doc.story(story_id) {
                        Some(story) => {
                            if stories_seen.contains(story_id) {
                                report.stories_repeated_for_threaded_frames += 1;
                            } else {
                                stories_seen.push(story_id.clone());
                            }
                            let paras = &story.paragraphs;
                            // Resolve every paragraph's look
                            // once, up front, so an unknown
                            // style is counted exactly once and
                            // the story's DefaultStyle can carry
                            // the first paragraph's look.
                            let looks: Vec<ParaLook> = paras
                                .iter()
                                .map(|p| look_of(doc, &p.style, &mut report))
                                .collect();
                            if let Some(first) = looks.first() {
                                out.push_str(&format!(
                                    "<DefaultStyle FONT=\"{}\" FONTSIZE=\"{}\"/>\n",
                                    esc_attr(&first.face),
                                    first.size_pt,
                                ));
                            }
                            for (j, (para, look)) in paras.iter().zip(&looks).enumerate() {
                                report.paragraphs += 1;
                                // One ITEXT per run (X1): each
                                // run names its own effective
                                // face, the only way Scribus
                                // states bold or italic, and
                                // the way our importer reads
                                // runs back. A paragraph of
                                // plain runs normalizes to one
                                // run and writes the single
                                // chunk it always wrote; empty
                                // runs carry no words and are
                                // skipped, as an empty
                                // paragraph always was.
                                for run in ob_model::normalize_runs(para.runs.clone()) {
                                    if run.text.is_empty() {
                                        continue;
                                    }
                                    let eff_bold = run.bold.unwrap_or(look.bold);
                                    let eff_italic = run.italic.unwrap_or(look.italic);
                                    let run_face =
                                        face_name_for(&look.family, eff_bold, eff_italic);
                                    // A family that itself
                                    // carries the word (an
                                    // "Acme Bold") reads that
                                    // way whatever the run
                                    // wants: the switch cannot
                                    // be stated exactly, and
                                    // is counted, never
                                    // silently flattened.
                                    if face_reads_bold(&run_face) != eff_bold
                                        || face_reads_italic(&run_face) != eff_italic
                                    {
                                        report.runs_with_unstatable_face += 1;
                                    }
                                    out.push_str(&format!(
                                        "<ITEXT FONT=\"{}\" FONTSIZE=\"{}\" CH=\"{}\"/>\n",
                                        esc_attr(&run_face),
                                        look.size_pt,
                                        esc_attr(&run.text),
                                    ));
                                }
                                if j + 1 < paras.len() {
                                    match &look.label {
                                        Some(label) => out.push_str(&format!(
                                            "<para PARENT=\"{}\"/>\n",
                                            esc_attr(label)
                                        )),
                                        None => out.push_str("<para/>\n"),
                                    }
                                }
                            }
                            // The story closes with a bare
                            // trail, as the genuine file ends
                            // its stories.
                            out.push_str("<trail/>\n");
                        }
                        None => {
                            report.frames_with_missing_story += 1;
                        }
                    }
                    out.push_str("</StoryText>\n</PAGEOBJECT>\n");
                }
            }
        }
        if panel.background_image.is_some() {
            report.tracing_pictures_left_behind += 1;
        }
    }

    out.push_str("</DOCUMENT>\n</SCRIBUSUTF8NEW>\n");

    // Plain-words notes for every kind of loss.
    if report.shape_frames_left_behind > 0 {
        report.notes.push(format!(
            "{} shape frame(s) (plain rectangles and placeholders) were not carried into the Scribus file.",
            report.shape_frames_left_behind
        ));
    }
    if report.picture_frames_left_behind > 0 {
        report.notes.push(format!(
            "{} picture box(es) were not carried into the Scribus file; the pictures are in the .obd and the print copies.",
            report.picture_frames_left_behind
        ));
    }
    if report.tracing_pictures_left_behind > 0 {
        report.notes.push(format!(
            "{} page(s) had a tracing-guide picture underneath; those guides never print and were not carried.",
            report.tracing_pictures_left_behind
        ));
    }
    if report.frames_with_missing_story > 0 {
        report.notes.push(format!(
            "{} text frame(s) named a story that is not in the document; they were written empty.",
            report.frames_with_missing_story
        ));
    }
    if report.stories_repeated_for_threaded_frames > 0 {
        report.notes.push(format!(
            "{} frame(s) share a story with an earlier frame; the Scribus file gives each frame its own copy of the words instead of linking the frames.",
            report.stories_repeated_for_threaded_frames
        ));
    }
    if report.paragraphs_with_unknown_style > 0 {
        report.notes.push(format!(
            "{} paragraph(s) use a style this document does not define; their words were written in the default look.",
            report.paragraphs_with_unknown_style
        ));
    }
    if report.runs_with_unstatable_face > 0 {
        report.notes.push(format!(
            "{} run(s) of words could not keep their exact bold or italic: their font's name itself carries that word, and Scribus reads the name.",
            report.runs_with_unstatable_face
        ));
    }
    Ok((out.into_bytes(), report))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ob_model::{
        ChurchProfile, ContentRole, Document, Frame, Panel, Paragraph, ParagraphStyle, Rect,
        SheetSpec, Size, Story, FORMAT_VERSION,
    };
    use std::collections::BTreeMap;

    fn style(
        id: &str,
        family: &str,
        size: i64,
        bold: bool,
        italic: bool,
        align: Align,
    ) -> ParagraphStyle {
        ParagraphStyle {
            id: id.into(),
            name: id.into(),
            font_size: size,
            bold,
            italic,
            align,
            space_after: 4_000,
            color: ob_model::Rgb::BLACK,
            font_family: family.into(),
            font_sha256: None,
        }
    }

    /// Two pages, two text frames, one shape frame, mixed
    /// styles: the round-trip fixture (the same document
    /// shape the IDML export test proves against).
    fn fixture_doc() -> Document {
        let styles = vec![
            style(
                "heading",
                "Liberation Sans",
                18_000,
                true,
                false,
                Align::Left,
            ),
            style("body", "Calibri", 11_000, false, false, Align::Left),
            style(
                "scripture",
                "Liberation Serif",
                11_000,
                false,
                true,
                Align::Left,
            ),
            style(
                "centered",
                "Liberation Sans",
                12_000,
                false,
                false,
                Align::Center,
            ),
        ];
        let stories = vec![
            Story {
                keep_through: None,
                id: "s-cover".into(),
                paragraphs: vec![
                    Paragraph::new("heading", "Welcome to worship"),
                    Paragraph::new("body", "Grace be with you this morning"),
                    Paragraph::new("scripture", "Be still, and know that I am God"),
                ],
            },
            Story {
                keep_through: None,
                id: "s-inside".into(),
                paragraphs: vec![
                    Paragraph::new("body", "Announcements for the week"),
                    Paragraph::new("centered", "All are welcome at the table"),
                ],
            },
        ];
        let panels = vec![
            Panel {
                id: "p1".into(),
                name: "Cover".into(),
                size: Size::new(396_000, 612_000),
                frames: vec![
                    Frame {
                        id: "f-cover".into(),
                        name: "Cover text".into(),
                        kind: FrameKind::Text,
                        rect: Rect::new(36_000, 48_000, 324_000, 200_000),
                        story_id: Some("s-cover".into()),
                        role: ContentRole::Standing,
                        picture: None,
                        music: None,
                    },
                    Frame {
                        id: "f-art".into(),
                        name: "Cover art block".into(),
                        kind: FrameKind::Shape,
                        rect: Rect::new(36_000, 400_000, 324_000, 120_000),
                        story_id: None,
                        role: ContentRole::Standing,
                        picture: None,
                        music: None,
                    },
                ],
                guides: Vec::new(),
                background_image: None,
            },
            Panel {
                id: "p2".into(),
                name: "Inside".into(),
                size: Size::new(612_000, 396_000),
                frames: vec![Frame {
                    id: "f-inside".into(),
                    name: "Inside text".into(),
                    kind: FrameKind::Text,
                    rect: Rect::new(24_000, 24_000, 400_000, 100_000),
                    story_id: Some("s-inside".into()),
                    role: ContentRole::Weekly,
                    picture: None,
                    music: None,
                }],
                guides: Vec::new(),
                background_image: None,
            },
        ];
        Document {
            format_version: FORMAT_VERSION,
            title: "Round trip".into(),
            profile: ChurchProfile::default(),
            fields: BTreeMap::new(),
            styles,
            stories,
            panels,
            sheet: SheetSpec {
                preset: "letter-bifold".into(),
                size: Size::new(792_000, 612_000),
            },
            no_announcements: false,
            assets: Vec::new(),
            asset_bytes: Default::default(),
            occasion_time: None,
            wanted_fonts: Vec::new(),
        }
    }

    #[test]
    fn sla_round_trip_keeps_words_styles_and_frames() {
        let doc = fixture_doc();
        let (bytes, report) = export_sla(&doc).expect("export");
        assert_eq!(report.pages, 2);
        assert_eq!(report.text_frames, 2);
        assert_eq!(report.paragraphs, 5);
        assert_eq!(report.shape_frames_left_behind, 1);
        assert!(report.notes.iter().any(|n| n.contains("shape frame")));

        let text = std::str::from_utf8(&bytes).expect("sla is text");
        assert!(text.starts_with("<?xml"), "an XML file: {text:.60}");
        assert!(text.contains("<SCRIBUSUTF8NEW"), "Scribus root");
        // Alignment rides on the style, as Scribus writes it,
        // even though the dump cannot carry it back.
        assert!(
            text.contains("ALIGN=\"1\""),
            "the centered style keeps its alignment in the file"
        );

        let dump = crate::sla::dump_from_sla(&bytes, "roundtrip.sla").expect("re-import");
        assert_eq!(dump.source_kind, crate::SourceKind::Sla);
        assert_eq!(dump.pages.len(), 2);
        assert!((dump.pages[0].width_pt - 396.0).abs() < 1.0);
        assert!((dump.pages[0].height_pt - 612.0).abs() < 1.0);
        assert!((dump.pages[1].width_pt - 612.0).abs() < 1.0);
        assert!((dump.pages[1].height_pt - 396.0).abs() < 1.0);

        let span = |text: &str| {
            dump.spans
                .iter()
                .find(|s| s.text == text)
                .unwrap_or_else(|| panic!("span {text:?} survived; got {:?}", dump.spans))
        };
        // Words survive exactly, with their looks as the
        // dump states them. Bold and italic ride in the
        // face name, the only place Scribus puts them.
        let heading = span("Welcome to worship");
        assert_eq!(heading.page, 0);
        assert_eq!(heading.font.as_deref(), Some("Liberation Sans Bold"));
        assert_eq!(heading.font_size, Some(18.0));
        assert_eq!(heading.bold, Some(true));

        let body = span("Grace be with you this morning");
        assert_eq!(body.font.as_deref(), Some("Calibri"));
        assert_eq!(body.font_size, Some(11.0));
        assert_eq!(body.bold, Some(false));

        let scripture = span("Be still, and know that I am God");
        assert_eq!(scripture.font.as_deref(), Some("Liberation Serif Italic"));
        assert_eq!(scripture.font_size, Some(11.0));
        assert_eq!(scripture.bold, Some(false));

        let centered = span("All are welcome at the table");
        assert_eq!(centered.page, 1);
        assert_eq!(centered.font.as_deref(), Some("Liberation Sans"));
        assert_eq!(centered.font_size, Some(12.0));

        // Frame rectangles survive within a point: the cover
        // frame's spans share its x and width, start at its
        // top, and tile to its bottom.
        let cover: Vec<_> = dump.spans.iter().filter(|s| s.page == 0).collect();
        assert_eq!(cover.len(), 3);
        for s in &cover {
            assert!((s.x.unwrap() - 36.0).abs() < 1.0, "x: {:?}", s.x);
            assert!((s.w.unwrap() - 324.0).abs() < 1.0, "w: {:?}", s.w);
        }
        assert!((cover[0].y.unwrap() - 48.0).abs() < 1.0);
        let bottom = cover[2].y.unwrap() + cover[2].h.unwrap();
        assert!((bottom - 248.0).abs() < 1.0, "bottom: {bottom}");
        // The inside frame on page 2.
        let inside = span("Announcements for the week");
        assert!((inside.x.unwrap() - 24.0).abs() < 1.0);
        assert!((inside.y.unwrap() - 24.0).abs() < 1.0);
        assert!((inside.w.unwrap() - 400.0).abs() < 1.0);
    }

    #[test]
    fn sla_export_escapes_text() {
        let mut doc = fixture_doc();
        doc.stories[0].paragraphs[0].set_text("Fish & loaves <for> \"all\"");
        let (bytes, _) = export_sla(&doc).expect("export");
        let dump = crate::sla::dump_from_sla(&bytes, "esc.sla").expect("re-import");
        assert!(dump
            .spans
            .iter()
            .any(|s| s.text == "Fish & loaves <for> \"all\""));
    }

    /// One frame whose story carries inline runs: a bold
    /// word and an italic phrase in a body paragraph, and a
    /// scripture paragraph (an italic style) with a bold
    /// run, which only the combined face name states.
    fn runs_doc() -> Document {
        let styles = vec![
            style("body", "Calibri", 11_000, false, false, Align::Left),
            style(
                "scripture",
                "Liberation Serif",
                11_000,
                false,
                true,
                Align::Left,
            ),
        ];
        let stories = vec![Story {
            keep_through: None,
            id: "s-runs".into(),
            paragraphs: vec![
                Paragraph::from_runs(
                    "body",
                    vec![
                        ob_model::Run::plain("Sing "),
                        ob_model::Run {
                            text: "hymn".into(),
                            bold: Some(true),
                            italic: None,
                        },
                        ob_model::Run::plain(" 432 with "),
                        ob_model::Run {
                            text: "gladness".into(),
                            bold: None,
                            italic: Some(true),
                        },
                        ob_model::Run::plain(" today"),
                    ],
                ),
                Paragraph::from_runs(
                    "scripture",
                    vec![
                        ob_model::Run::plain("Be still"),
                        ob_model::Run {
                            text: " and know".into(),
                            bold: Some(true),
                            italic: None,
                        },
                    ],
                ),
            ],
        }];
        let panels = vec![Panel {
            id: "p1".into(),
            name: "Page".into(),
            size: Size::new(396_000, 612_000),
            frames: vec![Frame {
                id: "f-runs".into(),
                name: "Words".into(),
                kind: FrameKind::Text,
                rect: Rect::new(36_000, 48_000, 324_000, 300_000),
                story_id: Some("s-runs".into()),
                role: ContentRole::Weekly,
                picture: None,
                music: None,
            }],
            guides: Vec::new(),
            background_image: None,
        }];
        Document {
            format_version: FORMAT_VERSION,
            title: "Runs".into(),
            profile: ChurchProfile::default(),
            fields: BTreeMap::new(),
            styles,
            stories,
            panels,
            sheet: SheetSpec {
                preset: "letter-bifold".into(),
                size: Size::new(792_000, 612_000),
            },
            no_announcements: false,
            assets: Vec::new(),
            asset_bytes: Default::default(),
            occasion_time: None,
            wanted_fonts: Vec::new(),
        }
    }

    #[test]
    fn sla_export_carries_runs_and_they_round_trip() {
        let doc = runs_doc();
        let (bytes, report) = export_sla(&doc).expect("export");
        assert_eq!(report.paragraphs, 2);
        assert_eq!(report.runs_with_unstatable_face, 0);

        let text = std::str::from_utf8(&bytes).expect("sla is text");
        assert!(
            text.contains("FONT=\"Calibri Bold\""),
            "the bold word names its own face"
        );
        assert!(
            text.contains("FONT=\"Liberation Serif Bold Italic\""),
            "the bold run in the italic style names the combined face"
        );
        assert_eq!(text.matches("<ITEXT").count(), 7, "one chunk per run");

        // The importer states one span per chunk, in order:
        // the sequence of (trimmed words, bold, italic) is
        // the source paragraphs' run sequence exactly.
        let dump = crate::sla::dump_from_sla(&bytes, "runs.sla").expect("re-import");
        let shape: Vec<(String, Option<bool>, bool)> = dump
            .spans
            .iter()
            .map(|s| (s.text.trim().to_string(), s.bold, s.italic))
            .collect();
        assert_eq!(
            shape,
            vec![
                ("Sing".to_string(), Some(false), false),
                ("hymn".to_string(), Some(true), false),
                ("432 with".to_string(), Some(false), false),
                ("gladness".to_string(), Some(false), true),
                ("today".to_string(), Some(false), false),
                ("Be still".to_string(), Some(false), true),
                ("and know".to_string(), Some(true), true),
            ],
            "every run's boundaries and face survive the round trip"
        );

        // The draft paragraph holds the same faces as runs
        // (effective against its style), not as new styles.
        let out = crate::draft::document_from_dump(&dump, ChurchProfile::default(), false, &[]);
        let para = out
            .doc
            .stories
            .iter()
            .flat_map(|st| &st.paragraphs)
            .find(|p| p.text().contains("hymn"))
            .expect("the paragraph exists");
        let pstyle = out.doc.style(&para.style).expect("the paragraph's style");
        let eff = |needle: &str| {
            para.runs.iter().find(|r| r.text.contains(needle)).map(|r| {
                (
                    r.bold.unwrap_or(pstyle.bold),
                    r.italic.unwrap_or(pstyle.italic),
                )
            })
        };
        assert_eq!(eff("hymn"), Some((true, false)), "the bold word");
        assert_eq!(eff("gladness"), Some((false, true)), "the italic phrase");
        assert_eq!(eff("and know"), Some((true, true)), "bold within italic");
    }

    #[test]
    fn sla_export_unformatted_paragraphs_keep_one_chunk_each() {
        // No churn for documents without runs: one ITEXT per
        // paragraph, its FONT the paragraph style's face,
        // exactly as before X1.
        let doc = fixture_doc();
        let (bytes, report) = export_sla(&doc).expect("export");
        let text = std::str::from_utf8(&bytes).expect("sla is text");
        let fonts: Vec<&str> = text
            .split("<ITEXT FONT=\"")
            .skip(1)
            .map(|rest| rest.split('"').next().expect("the FONT value"))
            .collect();
        assert_eq!(fonts.len(), report.paragraphs);
        assert_eq!(
            fonts,
            vec![
                "Liberation Sans Bold",
                "Calibri",
                "Liberation Serif Italic",
                "Calibri",
                "Liberation Sans",
            ]
        );
    }

    #[test]
    fn sla_export_counts_runs_a_bold_named_family_cannot_state() {
        // The family name itself says Bold; the style does
        // not. Any run in it reads back bold, so a run that
        // wants bold OFF cannot be stated exactly. The words
        // still export, and the count says so (X1, item 3).
        let mut doc = runs_doc();
        doc.styles = vec![style(
            "acme",
            "Acme Bold",
            11_000,
            false,
            false,
            Align::Left,
        )];
        doc.stories[0].paragraphs = vec![Paragraph::from_runs(
            "acme",
            vec![
                ob_model::Run::plain("Plain words"),
                ob_model::Run {
                    text: " spoken soft".into(),
                    bold: None,
                    italic: Some(true),
                },
                ob_model::Run {
                    text: " sung loud".into(),
                    bold: Some(true),
                    italic: None,
                },
            ],
        )];
        let (bytes, report) = export_sla(&doc).expect("export");
        assert_eq!(
            report.runs_with_unstatable_face, 2,
            "the plain and italic runs read back bold; the bold run states exactly"
        );
        assert!(report
            .notes
            .iter()
            .any(|n| n.contains("font's name itself carries that word")));
        let text = std::str::from_utf8(&bytes).expect("sla is text");
        for words in ["Plain words", "spoken soft", "sung loud"] {
            assert!(text.contains(words), "the words are all written: {words}");
        }
    }
}
