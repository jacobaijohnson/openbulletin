//! IDML export: an OpenBulletin document leaves as an IDML
//! package (a zip of XML), Adobe's published interchange
//! format, so an InDesign shop can take a bulletin the other
//! way (Jacob's import/export hub: out in every format we
//! take in, where anyone can write the format at all).
//!
//! Structure follows Adobe's published IDML layout, the same
//! parts our own importer (idml.rs) reads: a stored
//! `mimetype` entry first, META-INF/container.xml,
//! designmap.xml, Resources/Styles.xml, one Story part per
//! text frame, and one Spread part per page. The round-trip
//! test at the bottom proves the package against our own
//! importer; where the two disagreed, real IDML structure won
//! and the importer was the arbiter (see the test notes).

use ob_model::{Align, Document, FrameKind, ParagraphStyle};
use std::io::Write;

/// What the export carried, and what it had to leave behind,
/// in counts and plain words. Nothing is dropped silently:
/// the app shows this to the user next to the saved file.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct IdmlExportReport {
    /// Pages written as spreads.
    pub pages: usize,
    /// Text frames written, each with its own Story part.
    pub text_frames: usize,
    /// Paragraphs written.
    pub paragraphs: usize,
    /// Shape frames (rectangles, placeholders): IDML export
    /// does not carry them in this version.
    pub shape_frames_left_behind: usize,
    /// Picture frames: IDML export does not carry placed
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
    /// IDML threads one story through linked frames; our
    /// importer reads a story whole per frame, so the export
    /// gives each frame its own copy of the story text and
    /// counts the repeats here.
    pub stories_repeated_for_threaded_frames: usize,
    /// Paragraph style ids no style in the document defines:
    /// the words still export, set in the default look.
    pub paragraphs_with_unknown_style: usize,
    /// Plain-words notes, one per kind of loss.
    pub notes: Vec<String>,
}

fn esc_text(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn esc_attr(s: &str) -> String {
    esc_text(s).replace('"', "&quot;")
}

/// Milli-points to the plain decimal points IDML writes.
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

/// The IDML Self id for one of our paragraph styles. Real
/// IDML namespaces style ids as "ParagraphStyle/<name>".
fn style_self(style: &ParagraphStyle) -> String {
    format!("ParagraphStyle/{}", style.id)
}

fn justification(align: Align) -> &'static str {
    match align {
        Align::Left => "LeftAlign",
        Align::Center => "CenterAlign",
        Align::Right => "RightAlign",
    }
}

fn font_style_name(style: &ParagraphStyle) -> &'static str {
    match (style.bold, style.italic) {
        (true, true) => "Bold Italic",
        (true, false) => "Bold",
        (false, true) => "Italic",
        (false, false) => "Regular",
    }
}

const PKG_NS: &str = "http://xmlns.adobe.com/AdobeInDesign/IDML/1.0/packaging";
const XML_HEAD: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n";

fn container_xml() -> String {
    format!(
        "{XML_HEAD}<container version=\"1.0\" xmlns=\"urn:oasis:names:tc:opendocument:xmlns:container\">\n\
         <rootfiles><rootfile full-path=\"designmap.xml\" media-type=\"text/xml\"/></rootfiles>\n\
         </container>\n"
    )
}

fn styles_xml(doc: &Document) -> String {
    let mut out = String::new();
    out.push_str(XML_HEAD);
    out.push_str(&format!(
        "<idPkg:Styles xmlns:idPkg=\"{PKG_NS}\" DOMVersion=\"8.0\">\n<RootParagraphStyleGroup Self=\"uStylesRoot\">\n"
    ));
    for st in &doc.styles {
        out.push_str(&format!(
            "<ParagraphStyle Self=\"{}\" Name=\"{}\" PointSize=\"{}\" SpaceAfter=\"{}\" Justification=\"{}\">\
             <Properties><AppliedFont type=\"string\">{}</AppliedFont></Properties></ParagraphStyle>\n",
            esc_attr(&style_self(st)),
            esc_attr(&st.name),
            pt(st.font_size),
            pt(st.space_after),
            justification(st.align),
            esc_text(&st.font_family),
        ));
    }
    out.push_str("</RootParagraphStyleGroup>\n</idPkg:Styles>\n");
    out
}

/// One Story part: the paragraphs of one frame's story, each
/// a ParagraphStyleRange whose single CharacterStyleRange
/// carries the run's look explicitly (as real IDML does when
/// formatting is applied directly), so the story reads the
/// same with or without the style table.
fn story_xml(doc: &Document, story: &ob_model::Story, report: &mut IdmlExportReport) -> String {
    let mut out = String::new();
    out.push_str(XML_HEAD);
    out.push_str(&format!(
        "<idPkg:Story xmlns:idPkg=\"{PKG_NS}\" DOMVersion=\"8.0\">\n<Story Self=\"{}\" UserText=\"true\">\n",
        esc_attr(&story.id)
    ));
    for para in &story.paragraphs {
        let style = doc.style(&para.style);
        let (self_id, font, size, font_style, justify) = match style {
            Some(st) => (
                style_self(st),
                st.font_family.clone(),
                st.font_size,
                font_style_name(st),
                justification(st.align),
            ),
            None => {
                report.paragraphs_with_unknown_style += 1;
                (
                    "ParagraphStyle/$ID/[No paragraph style]".to_string(),
                    "Liberation Sans".to_string(),
                    11_000,
                    "Regular",
                    "LeftAlign",
                )
            }
        };
        out.push_str(&format!(
            "<ParagraphStyleRange AppliedParagraphStyle=\"{}\" Justification=\"{}\">\
             <CharacterStyleRange AppliedCharacterStyle=\"$ID/[No character style]\" FontStyle=\"{}\" PointSize=\"{}\">\
             <Properties><AppliedFont type=\"string\">{}</AppliedFont></Properties>\
             <Content>{}</Content></CharacterStyleRange></ParagraphStyleRange>\n",
            esc_attr(&self_id),
            justify,
            font_style,
            pt(size),
            esc_text(&font),
            esc_text(&para.text()),
        ));
        report.paragraphs += 1;
    }
    out.push_str("</Story>\n</idPkg:Story>\n");
    out
}

/// One Spread part per page: the page at the spread origin
/// with the bounds our exporter and importer both read as
/// the page rectangle, frames in page coordinates.
///
/// Arbiter note: real InDesign files park a page inside a
/// larger spread coordinate system with a translating
/// ItemTransform on the Page. Our importer reads the Page
/// GeometricBounds as the page rectangle and places frames
/// in spread coordinates, so this exporter emits one page
/// per spread with bounds "0 0 H W" and an identity page
/// transform. That is valid, minimal IDML structure; it is
/// the one place our own reader's contract, not a sample
/// InDesign file, decided the exact numbers.
fn spread_xml(doc: &Document, panel_index: usize) -> String {
    let panel = &doc.panels[panel_index];
    let mut out = String::new();
    out.push_str(XML_HEAD);
    out.push_str(&format!(
        "<idPkg:Spread xmlns:idPkg=\"{PKG_NS}\" DOMVersion=\"8.0\">\n<Spread Self=\"uSpread{}\" \
         AllowMasterPageItems=\"true\" PageCount=\"1\" BindingLocation=\"0\" \
         GeometricBounds=\"0 0 {} {}\" ItemTransform=\"1 0 0 1 0 0\">\n",
        panel_index + 1,
        pt(panel.size.h),
        pt(panel.size.w),
    ));
    out.push_str(&format!(
        "<Page Self=\"uPage{}\" GeometricBounds=\"0 0 {} {}\" ItemTransform=\"1 0 0 1 0 0\"/>\n",
        panel_index + 1,
        pt(panel.size.h),
        pt(panel.size.w),
    ));
    for frame in &panel.frames {
        if frame.kind != FrameKind::Text {
            continue;
        }
        if frame.story_id.is_none() {
            continue;
        }
        // The Story part is named after the frame, one story
        // per frame (see the report's threading note).
        let story_part_id = format!("uStory{}", frame.id);
        out.push_str(&format!(
            "<TextFrame Self=\"uFrame{}\" ParentStory=\"{}\" ItemTransform=\"1 0 0 1 {} {}\" \
             PreviousTextFrame=\"n\" NextTextFrame=\"n\">\
             <Properties><PathGeometry><GeometryPathType PathOpen=\"false\"><PathPointArray>\
             <PathPointType Anchor=\"0 0\" LeftDirection=\"0 0\" RightDirection=\"0 0\"/>\
             <PathPointType Anchor=\"{} 0\" LeftDirection=\"{} 0\" RightDirection=\"{} 0\"/>\
             <PathPointType Anchor=\"{} {}\" LeftDirection=\"{} {}\" RightDirection=\"{} {}\"/>\
             <PathPointType Anchor=\"0 {}\" LeftDirection=\"0 {}\" RightDirection=\"0 {}\"/>\
             </PathPointArray></GeometryPathType></PathGeometry></Properties></TextFrame>\n",
            esc_attr(&frame.id),
            esc_attr(&story_part_id),
            pt(frame.rect.x),
            pt(frame.rect.y),
            pt(frame.rect.w),
            pt(frame.rect.w),
            pt(frame.rect.w),
            pt(frame.rect.w),
            pt(frame.rect.h),
            pt(frame.rect.w),
            pt(frame.rect.h),
            pt(frame.rect.w),
            pt(frame.rect.h),
            pt(frame.rect.h),
            pt(frame.rect.h),
            pt(frame.rect.h),
        ));
    }
    out.push_str("</Spread>\n</idPkg:Spread>\n");
    out
}

/// Export the document as an IDML package. Returns the
/// package bytes and the report of what was not carried.
pub fn export_idml(doc: &Document) -> Result<(Vec<u8>, IdmlExportReport), String> {
    let mut report = IdmlExportReport {
        pages: doc.panels.len(),
        ..Default::default()
    };

    // Story parts first: one per text frame, so the design
    // map and spreads can name them.
    let mut story_parts: Vec<(String, String)> = Vec::new(); // (file name, xml)
    let mut stories_seen: Vec<String> = Vec::new();
    for panel in &doc.panels {
        for frame in &panel.frames {
            match frame.kind {
                FrameKind::Shape => report.shape_frames_left_behind += 1,
                FrameKind::Picture => report.picture_frames_left_behind += 1,
                FrameKind::Text => {
                    let Some(story_id) = &frame.story_id else {
                        continue;
                    };
                    report.text_frames += 1;
                    let part_id = format!("uStory{}", frame.id);
                    match doc.story(story_id) {
                        Some(story) => {
                            if stories_seen.contains(story_id) {
                                report.stories_repeated_for_threaded_frames += 1;
                            } else {
                                stories_seen.push(story_id.clone());
                            }
                            // The story's own id is replaced by the
                            // per-frame part id so ParentStory names
                            // the part unambiguously.
                            let mut renamed = story.clone();
                            renamed.id = part_id.clone();
                            let xml = story_xml(doc, &renamed, &mut report);
                            story_parts.push((format!("Stories/Story_{part_id}.xml"), xml));
                        }
                        None => {
                            report.frames_with_missing_story += 1;
                            let empty = ob_model::Story {
                                id: part_id.clone(),
                                paragraphs: Vec::new(),
                                keep_through: None,
                            };
                            let xml = story_xml(doc, &empty, &mut report);
                            story_parts.push((format!("Stories/Story_{part_id}.xml"), xml));
                        }
                    }
                }
            }
        }
        if panel.background_image.is_some() {
            report.tracing_pictures_left_behind += 1;
        }
    }

    // The design map names every part.
    let mut designmap = String::new();
    designmap.push_str(XML_HEAD);
    designmap.push_str(&format!(
        "<Document xmlns:idPkg=\"{PKG_NS}\" DOMVersion=\"8.0\" Self=\"d\" ZeroPoint=\"0 0\">\n"
    ));
    designmap.push_str("<idPkg:Styles src=\"Resources/Styles.xml\"/>\n");
    for (name, _) in &story_parts {
        designmap.push_str(&format!("<idPkg:Story src=\"{name}\"/>\n"));
    }
    for i in 0..doc.panels.len() {
        designmap.push_str(&format!(
            "<idPkg:Spread src=\"Spreads/Spread_uSpread{}.xml\"/>\n",
            i + 1
        ));
    }
    designmap.push_str("</Document>\n");

    // Assemble the zip: mimetype first and stored, per the
    // package rules, then the XML parts deflated.
    let mut buf = Vec::new();
    {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
        let stored = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        let deflated = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        zip.start_file("mimetype", stored)
            .map_err(|e| format!("cannot write the IDML package: {e}"))?;
        zip.write_all(b"application/vnd.adobe.indesign-idml-package")
            .map_err(|e| format!("cannot write the IDML package: {e}"))?;
        let parts: Vec<(String, String)> =
            std::iter::once(("META-INF/container.xml".to_string(), container_xml()))
                .chain(std::iter::once(("designmap.xml".to_string(), designmap)))
                .chain(std::iter::once((
                    "Resources/Styles.xml".to_string(),
                    styles_xml(doc),
                )))
                .chain(story_parts)
                .chain((0..doc.panels.len()).map(|i| {
                    (
                        format!("Spreads/Spread_uSpread{}.xml", i + 1),
                        spread_xml(doc, i),
                    )
                }))
                .collect();
        for (name, body) in &parts {
            zip.start_file(name, deflated)
                .map_err(|e| format!("cannot write the IDML package: {e}"))?;
            zip.write_all(body.as_bytes())
                .map_err(|e| format!("cannot write the IDML package: {e}"))?;
        }
        zip.finish()
            .map_err(|e| format!("cannot finish the IDML package: {e}"))?;
    }

    // Plain-words notes for every kind of loss.
    if report.shape_frames_left_behind > 0 {
        report.notes.push(format!(
            "{} shape frame(s) (plain rectangles and placeholders) were not carried into the IDML file.",
            report.shape_frames_left_behind
        ));
    }
    if report.picture_frames_left_behind > 0 {
        report.notes.push(format!(
            "{} picture box(es) were not carried into the IDML file; the pictures are in the .obd and the print copies.",
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
            "{} frame(s) share a story with an earlier frame; the IDML file gives each frame its own copy of the words instead of threading them.",
            report.stories_repeated_for_threaded_frames
        ));
    }
    if report.paragraphs_with_unknown_style > 0 {
        report.notes.push(format!(
            "{} paragraph(s) use a style this document does not define; their words were written in the default look.",
            report.paragraphs_with_unknown_style
        ));
    }
    Ok((buf, report))
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

    /// Two pages, three text frames, one shape frame, mixed
    /// styles: the round-trip fixture.
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
                    },
                    Frame {
                        id: "f-art".into(),
                        name: "Cover art block".into(),
                        kind: FrameKind::Shape,
                        rect: Rect::new(36_000, 400_000, 324_000, 120_000),
                        story_id: None,
                        role: ContentRole::Standing,
                        picture: None,
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
        }
    }

    #[test]
    fn idml_round_trip_keeps_words_styles_and_frames() {
        let doc = fixture_doc();
        let (bytes, report) = export_idml(&doc).expect("export");
        assert_eq!(report.pages, 2);
        assert_eq!(report.text_frames, 2);
        assert_eq!(report.paragraphs, 5);
        assert_eq!(report.shape_frames_left_behind, 1);
        assert!(report.notes.iter().any(|n| n.contains("shape frame")));

        // The package opens with the stored mimetype entry.
        let zip = zip::ZipArchive::new(std::io::Cursor::new(&bytes)).expect("package is a zip");
        assert_eq!(zip.name_for_index(0), Some("mimetype"));

        let dump = crate::idml::dump_from_idml(&bytes, "roundtrip.idml").expect("re-import");
        assert_eq!(dump.source_kind, crate::SourceKind::Idml);
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
        // Words survive, with their styles as the dump states them.
        let heading = span("Welcome to worship");
        assert_eq!(heading.page, 0);
        assert_eq!(heading.font.as_deref(), Some("Liberation Sans"));
        assert_eq!(heading.font_size, Some(18.0));
        assert_eq!(heading.bold, Some(true));

        let body = span("Grace be with you this morning");
        assert_eq!(body.font.as_deref(), Some("Calibri"));
        assert_eq!(body.font_size, Some(11.0));
        assert_eq!(body.bold, Some(false));

        // Italic travels as the span's own field (schema
        // v2); the font name stays the plain family name.
        let scripture = span("Be still, and know that I am God");
        assert_eq!(scripture.font.as_deref(), Some("Liberation Serif"));
        assert!(scripture.italic);
        assert_eq!(scripture.font_size, Some(11.0));

        let centered = span("All are welcome at the table");
        assert_eq!(centered.page, 1);
        assert_eq!(centered.font_size, Some(12.0));

        // Frame rectangles survive within a point: the cover
        // frame's spans share its x and width, start at its
        // top, and end at its bottom.
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
    fn idml_export_escapes_text() {
        let mut doc = fixture_doc();
        doc.stories[0].paragraphs[0].set_text("Fish & loaves <for> \"all\"");
        let (bytes, _) = export_idml(&doc).expect("export");
        let dump = crate::idml::dump_from_idml(&bytes, "esc.idml").expect("re-import");
        assert!(dump
            .spans
            .iter()
            .any(|s| s.text == "Fish & loaves <for> \"all\""));
    }
}
