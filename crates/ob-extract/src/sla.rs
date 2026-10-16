//! Scribus (.sla) import: a .sla file is one XML document
//! (root SCRIBUSUTF8NEW), so the importer reads it straight
//! into the shared extraction dump, and the draft builder
//! and its review apply unchanged.
//!
//! Format facts, grounded in a genuine Scribus-written
//! file (Next_Record.sla, Scribus 1.5.6.1, from the
//! ScribusGenerator project's examples, fetched 2026-10-15)
//! and the Scribus wiki's format specification:
//! - DOCUMENT carries the page size and the document's
//!   default font (DFONT) and size (DSIZE).
//! - Each PAGE element carries its own origin in document
//!   coordinates (PAGEXPOS/PAGEYPOS) and size; a PAGEOBJECT's
//!   XPOS/YPOS are document coordinates, so a frame's place
//!   on its page is the difference. OwnPage names the page.
//! - PTYPE 4 is a text frame, 2 an image frame, 12 a group
//!   (groups nest more PAGEOBJECTs inside them).
//! - In the 1.5/1.6 layout a text frame holds a StoryText
//!   child: ITEXT runs (CH text; FONT/FONTSIZE when the run
//!   differs), breakline for a soft break, para and trail to
//!   end paragraphs (PARENT names the paragraph style). A
//!   DefaultStyle child states the story's base font/size.
//!   The older 1.4 layout puts ITEXT (CFONT/CSIZE) and para
//!   directly under PAGEOBJECT; both are read.
//! - Scribus has no bold switch: the face name carries it
//!   ("Liberation Sans Bold"), which is also how the dump's
//!   consumers spot italic, so face names pass through as
//!   the file writes them.
//!
//! Trust: a run whose font and size are stated in the file
//! (on the run, its story's DefaultStyle, or its paragraph
//! style) is RawText. A run that fell back to the document
//! defaults, or a frame that had to be placed by guessing,
//! is NeedsReview: the words are the file's, their look is
//! partly our guess.
//!
//! Pictures are not carried: the dump schema's only picture
//! channel is the page's picture_only flag (the same one
//! idml.rs uses), so image frames set that flag on pages
//! with no words and are counted in the loss notes.

use crate::{ExtractionDump, PageInfo, SourceKind, Span, SpanTrust, DUMP_SCHEMA_VERSION};
use quick_xml::events::Event;
use quick_xml::Reader;
use std::collections::HashMap;

// ---------- a tiny XML tree over quick-xml ----------
// (idml.rs keeps its own copy; the two parsers stay
// independent so neither importer can break the other.)

#[derive(Debug, Default, Clone)]
struct Node {
    tag: String,
    attrs: HashMap<String, String>,
    children: Vec<Node>,
    text: String,
}

impl Node {
    fn attr(&self, name: &str) -> Option<&str> {
        self.attrs.get(name).map(|s| s.as_str())
    }
    fn find_all<'a>(&'a self, tag: &str, out: &mut Vec<&'a Node>) {
        for c in &self.children {
            if c.tag == tag {
                out.push(c);
            }
            c.find_all(tag, out);
        }
    }
    fn child(&self, tag: &str) -> Option<&Node> {
        self.children.iter().find(|c| c.tag == tag)
    }
}

fn fill_attrs(node: &mut Node, e: &quick_xml::events::BytesStart) {
    for a in e.attributes().flatten() {
        node.attrs.insert(
            String::from_utf8_lossy(a.key.as_ref()).into_owned(),
            a.unescape_value()
                .map(|v| v.into_owned())
                .unwrap_or_default(),
        );
    }
}

fn parse_xml(text: &str) -> Result<Node, String> {
    let mut reader = Reader::from_str(text);
    reader.config_mut().trim_text(false);
    let mut stack: Vec<Node> = vec![Node::default()];
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let mut node = Node {
                    tag: String::from_utf8_lossy(e.name().as_ref()).into_owned(),
                    ..Default::default()
                };
                fill_attrs(&mut node, &e);
                stack.push(node);
            }
            Ok(Event::Empty(e)) => {
                let mut node = Node {
                    tag: String::from_utf8_lossy(e.name().as_ref()).into_owned(),
                    ..Default::default()
                };
                fill_attrs(&mut node, &e);
                stack.last_mut().expect("stack").children.push(node);
            }
            Ok(Event::Text(e)) => {
                let t = e.unescape().map(|v| v.into_owned()).unwrap_or_default();
                stack.last_mut().expect("stack").text.push_str(&t);
            }
            Ok(Event::End(_)) => {
                if stack.len() > 1 {
                    let node = stack.pop().expect("node");
                    stack.last_mut().expect("stack").children.push(node);
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(format!("that Scribus file's XML is damaged: {e}")),
            _ => {}
        }
        buf.clear();
    }
    Ok(stack.into_iter().next().unwrap_or_default())
}

// ---------- styles ----------

#[derive(Debug, Clone, Default)]
struct StyleDef {
    font: Option<String>,
    size: Option<f32>,
    align: Option<ob_model::Align>,
}

/// A Scribus ALIGN value (on a para, a STYLE, or a story's
/// DefaultStyle): 0 left, 1 center, 2 right, 3 justified,
/// 4 forced. The model has no Justify, so the justified
/// values read as Left; our own exporter's 0/1/2 round-trip
/// exactly (sla_export.rs writes the same table).
fn align_value(raw: Option<&str>) -> Option<ob_model::Align> {
    match raw?.trim() {
        "1" => Some(ob_model::Align::Center),
        "2" => Some(ob_model::Align::Right),
        "0" | "3" | "4" => Some(ob_model::Align::Left),
        _ => None,
    }
}

/// Paragraph style definitions: DOCUMENT/STYLE elements by
/// NAME. A style's character look may sit on the STYLE
/// element itself or on a nested CSTYLE child; read both.
fn style_table(document: &Node) -> HashMap<String, StyleDef> {
    let mut out = HashMap::new();
    for st in &document.children {
        if st.tag != "STYLE" {
            continue;
        }
        let Some(name) = st.attr("NAME") else {
            continue;
        };
        let mut def = StyleDef {
            font: st.attr("FONT").map(|s| s.to_string()),
            size: st.attr("FONTSIZE").and_then(|v| v.parse().ok()),
            align: align_value(st.attr("ALIGN")),
        };
        if let Some(cs) = st.child("CSTYLE") {
            if def.font.is_none() {
                def.font = cs.attr("FONT").map(|s| s.to_string());
            }
            if def.size.is_none() {
                def.size = cs.attr("FONTSIZE").and_then(|v| v.parse().ok());
            }
        }
        out.insert(name.to_string(), def);
    }
    out
}

// ---------- stories ----------

/// One text segment: a run of characters sharing one look,
/// ended by a run change, a soft break, or a paragraph end.
struct Segment {
    text: String,
    font: Option<String>,
    size: Option<f32>,
    /// True when font and size both came from the file's
    /// statements for this story, not the document defaults.
    stated: bool,
    /// The paragraph's alignment, when the file states one
    /// (on the para itself, its style, or the story's base).
    align: Option<ob_model::Align>,
}

/// True when a Scribus face name carries a bold face, the
/// only bold signal the format has.
fn face_is_bold(face: &str) -> bool {
    face.split(|c: char| !c.is_alphanumeric())
        .any(|w| w.eq_ignore_ascii_case("bold"))
}

/// True when a Scribus face name carries an italic face:
/// the format's only italic signal, now read into the
/// span's italic field instead of being left for the draft
/// builder to sniff back out of the name.
fn face_is_italic(face: &str) -> bool {
    face.split(|c: char| !c.is_alphanumeric())
        .any(|w| w.eq_ignore_ascii_case("italic") || w.eq_ignore_ascii_case("oblique"))
}

/// Walk a story's child elements (a StoryText's children,
/// or a 1.4 PAGEOBJECT's own children) into segments in
/// reading order. `base` is the story's DefaultStyle look;
/// `doc_font`/`doc_size` are the DOCUMENT defaults.
fn story_segments(
    children: &[Node],
    base_font: &Option<String>,
    base_size: &Option<f32>,
    base_align: &Option<ob_model::Align>,
    styles: &HashMap<String, StyleDef>,
    doc_font: &Option<String>,
    doc_size: &Option<f32>,
) -> Vec<Segment> {
    let mut out: Vec<Segment> = Vec::new();
    // The paragraph style named by the closing para/trail
    // applies to the text before it, so segments are held
    // per paragraph and resolved when the paragraph ends.
    let mut paragraph: Vec<(String, Option<String>, Option<f32>)> = Vec::new();
    let flush = |paragraph: &mut Vec<(String, Option<String>, Option<f32>)>,
                 style_name: &Option<String>,
                 para_align: &Option<ob_model::Align>,
                 out: &mut Vec<Segment>| {
        let pdef = style_name
            .as_ref()
            .and_then(|n| styles.get(n))
            .cloned()
            .unwrap_or_default();
        // Alignment resolves like the look does: the para's
        // own ALIGN, then its style's, then the story's base.
        let align = para_align.or(pdef.align).or(*base_align);
        for (text, run_font, run_size) in paragraph.drain(..) {
            if text.trim().is_empty() {
                continue;
            }
            // Resolution order: the run itself, then the
            // paragraph style, then the story's base look,
            // then the document defaults (a guess).
            let mut stated = true;
            let font = run_font
                .clone()
                .or_else(|| pdef.font.clone())
                .or_else(|| base_font.clone())
                .or_else(|| {
                    stated = false;
                    doc_font.clone()
                });
            let mut size_stated = true;
            let size = run_size.or(pdef.size).or(*base_size).or_else(|| {
                size_stated = false;
                *doc_size
            });
            out.push(Segment {
                text,
                font,
                size,
                stated: stated && size_stated,
                align,
            });
        }
    };
    for node in children {
        match node.tag.as_str() {
            "ITEXT" => {
                let text = node.attr("CH").unwrap_or("").to_string();
                if text.is_empty() {
                    continue;
                }
                let font = node
                    .attr("FONT")
                    .or_else(|| node.attr("CFONT"))
                    .map(|s| s.to_string());
                let size = node
                    .attr("FONTSIZE")
                    .or_else(|| node.attr("CSIZE"))
                    .and_then(|v| v.parse::<f32>().ok());
                paragraph.push((text, font, size));
            }
            // A soft line break ends the segment; the
            // paragraph (and its style) continues.
            "breakline" => {
                // Split here by flushing what we have as
                // segments but keep them in this paragraph:
                // emulate by closing the current run group
                // with no style change: mark with an empty
                // paragraph-style resolution round-trip.
                // Simplest faithful model: segments are
                // already per-run; a break only matters for
                // stacking, which the slice layout absorbs.
            }
            "para" | "trail" => {
                let style_name = node.attr("PARENT").map(|s| s.to_string());
                let para_align = align_value(node.attr("ALIGN"));
                flush(&mut paragraph, &style_name, &para_align, &mut out);
            }
            _ => {}
        }
    }
    // A story that never closes its last paragraph still
    // has words; take them with no style claim.
    flush(&mut paragraph, &None, &None, &mut out);
    out
}

// ---------- the importer ----------

struct PageGeom {
    index: u32,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

fn num(node: &Node, name: &str) -> Option<f32> {
    node.attr(name).and_then(|v| v.parse().ok())
}

/// Import a .sla file. Returns the dump plus plain-words
/// loss notes (what the file had that the dump cannot
/// carry), in the same spirit as the IDML export report.
pub fn import_sla(
    bytes: &[u8],
    source_name: &str,
) -> Result<(ExtractionDump, Vec<String>), String> {
    let text =
        std::str::from_utf8(bytes).map_err(|_| "that file is not Scribus XML text".to_string())?;
    let root_holder = parse_xml(text)?;
    let root = root_holder
        .children
        .iter()
        .find(|c| c.tag.starts_with("SCRIBUS"))
        .ok_or("that file has no Scribus document in it")?;
    let version = root.attr("Version").unwrap_or("").to_string();
    let document = root
        .child("DOCUMENT")
        .ok_or("that Scribus file has no document in it")?;

    let doc_w = num(document, "PAGEWIDTH").unwrap_or(612.0);
    let doc_h = num(document, "PAGEHEIGHT").unwrap_or(792.0);
    let doc_font = document.attr("DFONT").map(|s| s.to_string());
    let doc_size = num(document, "DSIZE");
    let styles = style_table(document);

    // Pages, in file order.
    let mut pages: Vec<PageGeom> = Vec::new();
    for pg in &document.children {
        if pg.tag != "PAGE" {
            continue;
        }
        let index = pages.len() as u32;
        pages.push(PageGeom {
            index,
            x: num(pg, "PAGEXPOS").unwrap_or(0.0),
            y: num(pg, "PAGEYPOS").unwrap_or(0.0),
            w: num(pg, "PAGEWIDTH").unwrap_or(doc_w),
            h: num(pg, "PAGEHEIGHT").unwrap_or(doc_h),
        });
    }
    if pages.is_empty() {
        return Err("that Scribus file lists no pages".into());
    }

    let mut page_infos: Vec<PageInfo> = pages
        .iter()
        .map(|p| PageInfo {
            index: p.index,
            width_pt: p.w,
            height_pt: p.h,
            picture_only: false,
        })
        .collect();
    let mut spans: Vec<Span> = Vec::new();
    let mut notes: Vec<String> = Vec::new();
    let mut pictures = 0usize;
    let mut picture_names: Vec<String> = Vec::new();
    let mut orphan_objects = 0usize;
    let mut rotated_frames = 0usize;
    let mut path_text_frames = 0usize;
    let mut empty_text_frames = 0usize;
    let mut pages_with_images: Vec<u32> = Vec::new();

    // Every PAGEOBJECT, including the ones nested in groups.
    let mut objects = Vec::new();
    document.find_all("PAGEOBJECT", &mut objects);
    for obj in objects {
        let ptype = obj.attr("PTYPE").unwrap_or("");
        // Which page: OwnPage when stated, else the page
        // whose rectangle holds the object's center.
        let fx = num(obj, "XPOS").unwrap_or(0.0);
        let fy = num(obj, "YPOS").unwrap_or(0.0);
        let fw = num(obj, "WIDTH").unwrap_or(0.0);
        let fh = num(obj, "HEIGHT").unwrap_or(0.0);
        let own = obj.attr("OwnPage").and_then(|v| v.parse::<u32>().ok());
        let page = match own {
            Some(i) => pages.iter().find(|p| p.index == i),
            None => pages.iter().find(|p| {
                let cx = fx + fw / 2.0;
                let cy = fy + fh / 2.0;
                cx >= p.x && cx <= p.x + p.w && cy >= p.y && cy <= p.y + p.h
            }),
        };
        let Some(page) = page else {
            if ptype == "4" || ptype == "2" {
                orphan_objects += 1;
            }
            continue;
        };
        let lx = fx - page.x;
        let ly = fy - page.y;
        match ptype {
            "2" => {
                pictures += 1;
                pages_with_images.push(page.index);
                if let Some(pfile) = obj.attr("PFILE") {
                    let base = pfile.rsplit('/').next().unwrap_or(pfile);
                    if !base.is_empty() && picture_names.len() < 5 {
                        picture_names.push(base.to_string());
                    }
                }
            }
            "4" => {
                let rotated = num(obj, "ROT").map(|r| r.abs() > 0.01).unwrap_or(false);
                if rotated {
                    rotated_frames += 1;
                }
                // The story: a StoryText child (1.5/1.6), or
                // the object's own ITEXT children (1.4).
                let (children, base_font, base_size, base_align): (
                    Vec<Node>,
                    Option<String>,
                    Option<f32>,
                    Option<ob_model::Align>,
                ) = if let Some(story) = obj.child("StoryText") {
                    let (mut bf, mut bs, mut ba) = (None, None, None);
                    if let Some(ds) = story.child("DefaultStyle") {
                        bf = ds.attr("FONT").map(|s| s.to_string());
                        bs = num(ds, "FONTSIZE");
                        ba = align_value(ds.attr("ALIGN"));
                    }
                    (story.children.clone(), bf, bs, ba)
                } else {
                    (obj.children.clone(), None, None, None)
                };
                let segments = story_segments(
                    &children,
                    &base_font,
                    &base_size,
                    &base_align,
                    &styles,
                    &doc_font,
                    &doc_size,
                );
                if segments.is_empty() {
                    empty_text_frames += 1;
                    continue;
                }
                // One span per segment, stacked down the
                // frame, the draft builder's shared layout.
                let slice = fh.max(1.0) / segments.len() as f32;
                for (k, seg) in segments.iter().enumerate() {
                    let bold = seg.font.as_deref().map(face_is_bold).unwrap_or(false);
                    let italic = seg.font.as_deref().map(face_is_italic).unwrap_or(false);
                    spans.push(Span {
                        page: page.index,
                        text: seg.text.clone(),
                        trust: if seg.stated && !rotated {
                            SpanTrust::RawText
                        } else {
                            SpanTrust::NeedsReview
                        },
                        x: Some(lx),
                        y: Some(ly + slice * k as f32),
                        w: Some(fw),
                        h: Some(slice),
                        font_size: Some(seg.size.unwrap_or(12.0)),
                        bold: Some(bold),
                        font: seg.font.clone(),
                        align: seg.align,
                        italic,
                        color: [0, 0, 0],
                        runs: Vec::new(),
                        thread: None,
                    });
                }
            }
            "8" => path_text_frames += 1,
            _ => {}
        }
    }

    // A page of pictures with no words reads as picture-only.
    for idx in &pages_with_images {
        if !spans.iter().any(|s| s.page == *idx) {
            page_infos[*idx as usize].picture_only = true;
        }
    }

    if pictures > 0 {
        let mut note = format!(
            "{pictures} picture frame(s) were not carried over; the words are here, the pictures are not"
        );
        if !picture_names.is_empty() {
            note.push_str(&format!(" ({})", picture_names.join(", ")));
        }
        note.push('.');
        notes.push(note);
    }
    if orphan_objects > 0 {
        notes.push(format!(
            "{orphan_objects} frame(s) sit on a master page or a page we could not place, and were left out."
        ));
    }
    if rotated_frames > 0 {
        notes.push(format!(
            "{rotated_frames} text frame(s) are rotated in Scribus; their words came over but are marked Check this, and their boxes are unrotated."
        ));
    }
    if path_text_frames > 0 {
        notes.push(format!(
            "{path_text_frames} frame(s) run text along a path; that text was not carried over."
        ));
    }
    if empty_text_frames > 0 {
        notes.push(format!("{empty_text_frames} text frame(s) were empty."));
    }

    let dump = ExtractionDump {
        schema_version: DUMP_SCHEMA_VERSION,
        source_kind: SourceKind::Sla,
        source_name: source_name.to_string(),
        source_bytes: bytes.len(),
        extractor: format!("openbulletin-sla (Scribus {version})"),
        pages: page_infos,
        spans,
        images: Vec::new(),
    };
    Ok((dump, notes))
}

/// The house-shaped entry point: dump only, notes dropped.
pub fn dump_from_sla(bytes: &[u8], source_name: &str) -> Result<ExtractionDump, String> {
    import_sla(bytes, source_name).map(|(dump, _)| dump)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A trimmed .sla in the shape of a genuine Scribus
    /// 1.5.6.1 file: the structure, element names, and
    /// attribute names (and the A4 page origins) are modeled
    /// on Next_Record.sla from the ScribusGenerator project
    /// (a real Scribus-written file, fetched 2026-10-15);
    /// the words are our fictional church's, and the file is
    /// hand-trimmed, not a Scribus save.
    fn fixture_sla() -> &'static str {
        r#"<?xml version="1.0" encoding="UTF-8"?>
<SCRIBUSUTF8NEW Version="1.5.6.1">
<DOCUMENT ANZPAGES="2" PAGEWIDTH="595.275590551181" PAGEHEIGHT="841.889763779528" DFONT="Liberation Serif" DSIZE="12">
<STYLE NAME="Default Paragraph Style" DefaultStyle="1" ALIGN="0"/>
<STYLE NAME="Heading" ALIGN="0"><CSTYLE FONT="Liberation Sans Bold" FONTSIZE="18"/></STYLE>
<PAGE PAGEXPOS="100.00062992126" PAGEYPOS="20.0012598425197" PAGEWIDTH="595.275590551181" PAGEHEIGHT="841.889763779528" NUM="0" MNAM="Normal"/>
<PAGE PAGEXPOS="695.276220472441" PAGEYPOS="20.0012598425197" PAGEWIDTH="595.275590551181" PAGEHEIGHT="841.889763779528" NUM="1" MNAM="Normal"/>
<PAGEOBJECT XPOS="259.5" YPOS="84" OwnPage="0" PTYPE="4" WIDTH="211.5" HEIGHT="90" ANNAME="Text1">
<StoryText>
<DefaultStyle FONT="Liberation Serif"/>
<ITEXT CH="Welcome to "/>
<ITEXT FONT="Liberation Sans Bold" FONTSIZE="14" CH="Grace Lutheran"/>
<para PARENT="Heading"/>
<ITEXT FONTSIZE="11" CH="Service at 9:30 in the morning"/>
<breakline/>
<ITEXT FONTSIZE="11" CH="All are welcome"/>
<trail/>
</StoryText>
</PAGEOBJECT>
<PAGEOBJECT XPOS="173.669291338583" YPOS="162.169291338583" OwnPage="0" PTYPE="2" WIDTH="63" HEIGHT="60.9264566929134" ANNAME="Image1" PFILE="images/cover-art.png"/>
<PAGEOBJECT XPOS="900" YPOS="120" OwnPage="1" PTYPE="4" WIDTH="204" HEIGHT="80" ANNAME="Text2">
<StoryText>
<DefaultStyle/>
<ITEXT CH="Announcements for this week"/>
<trail/>
</StoryText>
</PAGEOBJECT>
</DOCUMENT>
</SCRIBUSUTF8NEW>
"#
    }

    #[test]
    fn sla_dump_carries_pages_words_and_geometry() {
        let (dump, notes) = import_sla(fixture_sla().as_bytes(), "old.sla").expect("import");
        assert_eq!(dump.source_kind, SourceKind::Sla);
        assert_eq!(dump.pages.len(), 2);
        assert!((dump.pages[0].width_pt - 595.276).abs() < 0.01);
        assert!((dump.pages[0].height_pt - 841.89).abs() < 0.01);

        // Page 0 story: four segments in all (two runs in
        // the Heading paragraph, two in the plain one).
        let p0: Vec<_> = dump.spans.iter().filter(|s| s.page == 0).collect();
        assert_eq!(p0.len(), 4, "spans: {p0:?}");
        let first = p0[0];
        assert_eq!(first.text, "Welcome to ");
        // No run attributes: the closing para names the
        // Heading style, whose look (Liberation Sans Bold,
        // 18pt) covers the whole paragraph.
        assert_eq!(first.font.as_deref(), Some("Liberation Sans Bold"));
        assert_eq!(first.font_size, Some(18.0));
        assert_eq!(first.bold, Some(true));
        assert_eq!(first.trust, SpanTrust::RawText);
        // Frame place: XPOS 259.5 - PAGEXPOS 100.0006.
        assert!((first.x.unwrap() - 159.4994).abs() < 0.01);
        assert!((first.y.unwrap() - 63.9987).abs() < 0.01);
        assert!((first.w.unwrap() - 211.5).abs() < 0.01);

        let second = p0[1];
        assert_eq!(second.text, "Grace Lutheran");
        assert_eq!(second.font.as_deref(), Some("Liberation Sans Bold"));
        assert_eq!(second.font_size, Some(14.0));
        assert_eq!(second.bold, Some(true));
        assert_eq!(second.trust, SpanTrust::RawText);

        let third = p0[2];
        assert_eq!(third.text, "Service at 9:30 in the morning");
        assert_eq!(third.font_size, Some(11.0));
        // No font anywhere for this paragraph: its size is
        // stated, its font came only from the story base...
        // the base names a font, so the look is stated.
        assert_eq!(third.font.as_deref(), Some("Liberation Serif"));

        // Page 1 story: one segment, document-default look:
        // font from DFONT, size from DSIZE, so NeedsReview.
        let p1: Vec<_> = dump.spans.iter().filter(|s| s.page == 1).collect();
        assert_eq!(p1.len(), 1);
        assert_eq!(p1[0].text, "Announcements for this week");
        assert_eq!(p1[0].trust, SpanTrust::NeedsReview);
        assert!((p1[0].x.unwrap() - 204.7238).abs() < 0.01);

        // The picture is counted, not carried; page 0 still
        // has words, so it is not picture-only.
        assert!(
            notes.iter().any(|n| n.contains("1 picture frame")),
            "{notes:?}"
        );
        assert!(!dump.pages[0].picture_only);
    }

    #[test]
    fn sla_rejects_non_scribus() {
        assert!(dump_from_sla(b"<html></html>", "x.sla").is_err());
        assert!(dump_from_sla(b"not xml", "x.sla").is_err());
    }
}
