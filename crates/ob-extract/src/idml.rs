//! IDML import: Adobe's supported way out of InDesign is the
//! IDML package (a zip of XML). This importer reads it into the
//! same extraction dump every importer targets, so the draft
//! builder and its review apply unchanged (docs/EXTRACT.md).
//! Words come over with their frames, sizes, and fonts; placed
//! pictures do not in this version, and the review says so.

use crate::{ExtractionDump, PageInfo, SourceKind, Span, SpanRun, SpanTrust, DUMP_SCHEMA_VERSION};
use quick_xml::events::Event;
use quick_xml::Reader;
use std::collections::HashMap;
use std::io::Read;

// ---------- a tiny XML tree over quick-xml ----------

#[derive(Debug, Default)]
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
        // Preorder, the recursive walk's exact order, on an
        // explicit stack: a hostile file nested ranges tens
        // of thousands deep and the recursion aborted the
        // whole app (audit Batch 1).
        let mut stack: Vec<&Node> = self.children.iter().rev().collect();
        while let Some(node) = stack.pop() {
            if node.tag == tag {
                out.push(node);
            }
            stack.extend(node.children.iter().rev());
        }
    }
    fn find_path<'a>(&'a self, path: &[&str]) -> Option<&'a Node> {
        let mut cur = self;
        for tag in path {
            cur = cur.children.iter().find(|c| c.tag == *tag)?;
        }
        Some(cur)
    }
}

/// XML nesting deeper than this is not a document, it is
/// a trap: the walks over the tree (and the tree's own
/// drop) recurse with its depth, and a file nested tens of
/// thousands of levels deep aborted the whole app before
/// this cap (audit Batch 1). Real IDML nests perhaps
/// twenty levels; this is generous headroom, and the cap
/// also bounds every per-range rescan by depth x nodes.
const MAX_XML_DEPTH: usize = 256;

/// The words marking the depth refusal, so story reading
/// can pass it up whole instead of treating the file as
/// damaged and quietly empty (audit Batch 1): a refused
/// story must refuse the import, never vanish from it.
const TOO_DEEP: &str = "nested too deeply to read safely";

fn parse_xml(text: &str) -> Result<Node, String> {
    let mut reader = Reader::from_str(text);
    reader.config_mut().trim_text(false);
    let mut stack: Vec<Node> = vec![Node::default()];
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                if stack.len() > MAX_XML_DEPTH {
                    return Err(format!("that IDML file's XML is {TOO_DEEP}"));
                }
                let mut node = Node {
                    tag: String::from_utf8_lossy(e.name().as_ref()).into_owned(),
                    ..Default::default()
                };
                for a in e.attributes().flatten() {
                    node.attrs.insert(
                        String::from_utf8_lossy(a.key.as_ref()).into_owned(),
                        a.unescape_value()
                            .map(|v| v.into_owned())
                            .unwrap_or_default(),
                    );
                }
                stack.push(node);
            }
            Ok(Event::Empty(e)) => {
                let mut node = Node {
                    tag: String::from_utf8_lossy(e.name().as_ref()).into_owned(),
                    ..Default::default()
                };
                for a in e.attributes().flatten() {
                    node.attrs.insert(
                        String::from_utf8_lossy(a.key.as_ref()).into_owned(),
                        a.unescape_value()
                            .map(|v| v.into_owned())
                            .unwrap_or_default(),
                    );
                }
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
            Err(e) => return Err(format!("that IDML file's XML is damaged: {e}")),
            _ => {}
        }
        buf.clear();
    }
    Ok(stack.into_iter().next().unwrap_or_default())
}

// ---------- IDML geometry helpers ----------

fn floats(s: &str) -> Vec<f32> {
    s.split_whitespace()
        .filter_map(|t| t.parse::<f32>().ok())
        .collect()
}

/// ItemTransform "a b c d tx ty": the translation is what
/// places the frame on the spread.
fn transform_xy(s: Option<&str>) -> (f32, f32) {
    match s {
        Some(t) => {
            let f = floats(t);
            if f.len() == 6 {
                (f[4], f[5])
            } else {
                (0.0, 0.0)
            }
        }
        None => (0.0, 0.0),
    }
}

/// GeometricBounds "top left bottom right".
fn geometric_bounds(s: Option<&str>) -> Option<(f32, f32, f32, f32)> {
    let f = floats(s?);
    if f.len() == 4 {
        Some((f[0], f[1], f[2], f[3]))
    } else {
        None
    }
}

// ---------- style table ----------

struct StyleDef {
    font: Option<String>,
    size: Option<f32>,
    align: Option<ob_model::Align>,
}

/// An IDML Justification value as one of our three
/// alignments. The justified variants have no counterpart
/// in the model (there is no Justify), so they read as
/// Left; the values our own exporter writes (LeftAlign,
/// CenterAlign, RightAlign) round-trip exactly.
fn justification_align(value: Option<&str>) -> Option<ob_model::Align> {
    match value {
        Some("CenterAlign") => Some(ob_model::Align::Center),
        Some("RightAlign") => Some(ob_model::Align::Right),
        Some(_) => Some(ob_model::Align::Left),
        None => None,
    }
}

fn style_table(styles_xml: &str) -> HashMap<String, StyleDef> {
    let mut out = HashMap::new();
    if let Ok(root) = parse_xml(styles_xml) {
        let mut styles = Vec::new();
        root.find_all("ParagraphStyle", &mut styles);
        for st in styles {
            let Some(id) = st.attr("Self") else { continue };
            let font = st
                .find_path(&["Properties", "AppliedFont"])
                .map(|n| n.text.trim().to_string())
                .filter(|t| !t.is_empty());
            out.insert(
                id.to_string(),
                StyleDef {
                    font,
                    size: st.attr("PointSize").and_then(|v| v.parse().ok()),
                    align: justification_align(st.attr("Justification")),
                },
            );
        }
    }
    out
}

// ---------- stories ----------

struct ParaText {
    text: String,
    /// The paragraph's pieces, one per CharacterStyleRange
    /// segment (G1b): concatenating them gives `text`
    /// exactly (the joining space rides on the earlier
    /// piece). The paragraph's own bold/italic are its
    /// first piece's.
    runs: Vec<SpanRun>,
    font: Option<String>,
    size: Option<f32>,
    bold: bool,
    italic: bool,
    style_ref: Option<String>,
    align: Option<ob_model::Align>,
}

/// Close out one accumulated paragraph: runs become a
/// ParaText (skipped when they hold no words).
fn finish_paragraph(
    out: &mut Vec<ParaText>,
    runs: Vec<SpanRun>,
    font: Option<String>,
    size: Option<f32>,
    style_ref: &Option<String>,
    align: Option<ob_model::Align>,
) {
    let text: String = runs.iter().map(|r| r.text.as_str()).collect();
    if text.trim().is_empty() {
        return;
    }
    let (bold, italic) = runs
        .first()
        .map(|r| (r.bold.unwrap_or(false), r.italic.unwrap_or(false)))
        .unwrap_or((false, false));
    out.push(ParaText {
        text,
        runs,
        font,
        size,
        bold,
        italic,
        style_ref: style_ref.clone(),
        align,
    });
}

fn story_paragraphs(story_xml: &str) -> Result<Vec<ParaText>, String> {
    let mut out = Vec::new();
    let root = match parse_xml(story_xml) {
        Ok(root) => root,
        // A story nested too deeply refuses the whole
        // import (audit Batch 1): quietly returning no
        // words would hand the office an empty draft and
        // call it a rescue. Any other damage keeps the
        // long-standing behavior: that story reads empty.
        Err(e) if e.contains(TOO_DEEP) => return Err(e),
        Err(_) => return Ok(out),
    };
    let mut ranges = Vec::new();
    root.find_all("ParagraphStyleRange", &mut ranges);
    for range in ranges {
        let style_ref = range.attr("AppliedParagraphStyle").map(|s| s.to_string());
        let range_align = justification_align(range.attr("Justification"));
        let mut chars = Vec::new();
        range.find_all("CharacterStyleRange", &mut chars);
        if chars.is_empty() {
            chars.push(range);
        }
        // One paragraph accumulates across the range's
        // character ranges (G1b): each range's words become
        // runs of the paragraph they sit in, and only a Br
        // (or the range's end) closes the paragraph. The
        // paragraph's font and size are the first range's
        // that state them.
        let mut cur: Vec<SpanRun> = Vec::new();
        let mut cur_font: Option<String> = None;
        let mut cur_size: Option<f32> = None;
        for cr in chars {
            let font_style = cr.attr("FontStyle").unwrap_or("");
            let font = cr
                .find_path(&["Properties", "AppliedFont"])
                .map(|n| n.text.trim().to_string())
                .filter(|t| !t.is_empty());
            let size = cr.attr("PointSize").and_then(|v| v.parse().ok());
            let bold = font_style.to_lowercase().contains("bold");
            let italic = font_style.to_lowercase().contains("italic")
                || font_style.to_lowercase().contains("oblique");
            // Text: Content runs joined; a Br ends the paragraph.
            // Preorder on an explicit stack (audit Batch 1),
            // the recursive walk's exact order.
            let mut segments: Vec<String> = vec![String::new()];
            let mut walk_stack: Vec<&Node> = cr.children.iter().rev().collect();
            while let Some(node) = walk_stack.pop() {
                match node.tag.as_str() {
                    "Content" => {
                        segments
                            .last_mut()
                            .expect("segment")
                            .push_str(node.text.trim());
                    }
                    "Br" => segments.push(String::new()),
                    _ => walk_stack.extend(node.children.iter().rev()),
                }
            }
            for (si, seg) in segments.iter().enumerate() {
                if si > 0 {
                    finish_paragraph(
                        &mut out,
                        std::mem::take(&mut cur),
                        cur_font.take(),
                        cur_size.take(),
                        &style_ref,
                        range_align,
                    );
                }
                let text = seg.trim();
                if text.is_empty() {
                    continue;
                }
                if cur_font.is_none() {
                    cur_font = font.clone();
                }
                if cur_size.is_none() {
                    cur_size = size;
                }
                if let Some(last) = cur.last_mut() {
                    last.text.push(' ');
                }
                cur.push(SpanRun {
                    text: text.to_string(),
                    bold: Some(bold),
                    italic: Some(italic),
                });
            }
        }
        finish_paragraph(&mut out, cur, cur_font, cur_size, &style_ref, range_align);
    }
    Ok(out)
}

/// Close off a part that was cut at the per-part cap,
/// so the parser can read everything in it that was
/// complete before the cut (round 2, RT-5): drop the
/// partial tail past the last whole tag, then close
/// every element still open, innermost first. Without
/// this, a cut story's still-open ancestors never
/// land in the parsed tree and even its finished
/// paragraphs read as nothing. The importer's loss
/// note already says the words past the point are
/// gone; this is how the words before it survive.
fn close_off_cut(text: &str) -> String {
    let Some(at) = text.rfind('>') else {
        return String::new();
    };
    let mut out = text[..=at].to_string();
    let mut reader = Reader::from_str(&out);
    let mut buf = Vec::new();
    let mut open: Vec<String> = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                open.push(String::from_utf8_lossy(e.name().as_ref()).into_owned());
            }
            Ok(Event::End(_)) => {
                open.pop();
            }
            Ok(Event::Eof) => break,
            // Damage before the cut is the parser's to
            // judge; the repair just stops tracking.
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    for name in open.iter().rev() {
        out.push_str("</");
        out.push_str(name);
        out.push('>');
    }
    out
}

// ---------- the importer ----------

const MAX_ENTRIES: usize = 512;
const MAX_ENTRY: u64 = 32 * 1024 * 1024;

/// The most of an IDML package we will hold at once,
/// across all its parts together (round 2, RT-4): the
/// per-part cap alone let 512 parts of 32 MB add up
/// toward 16 GB, refused in the end only by the
/// machine running out of memory. 512 MB is the
/// project's standing total cap, enforced while
/// reading; a real package is a few megabytes.
const MAX_TOTAL: u64 = 512 * 1024 * 1024;

pub fn dump_from_idml(bytes: &[u8], source_name: &str) -> Result<ExtractionDump, String> {
    let cursor = std::io::Cursor::new(bytes);
    let mut zip = zip::ZipArchive::new(cursor)
        .map_err(|e| format!("that is not an IDML package (a zip of XML): {e}"))?;
    if zip.len() > MAX_ENTRIES {
        return Err("that IDML package has too many parts".into());
    }
    let mut files: HashMap<String, String> = HashMap::new();
    let mut loss_notes: Vec<String> = Vec::new();
    let mut total_read: u64 = 0;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| e.to_string())?;
        let name = entry.name().to_string();
        if !name.ends_with(".xml") {
            continue;
        }
        // Read one byte past the per-part cap, so a part
        // that runs over it is KNOWN to run over it
        // (round 2, RT-5): the words past the cap used
        // to vanish in silence when the cut text was
        // read as if it were whole. What survives the
        // cut is still rescued; the loss is counted in
        // the dump's loss notes and surfaces in the
        // draft's review.
        let mut raw = Vec::new();
        entry
            .by_ref()
            .take(MAX_ENTRY + 1)
            .read_to_end(&mut raw)
            .map_err(|e| format!("cannot read a part of that IDML package: {e}"))?;
        total_read += raw.len() as u64;
        if total_read > MAX_TOTAL {
            return Err("that IDML file's insides are too large to read safely".into());
        }
        let text = if raw.len() as u64 > MAX_ENTRY {
            loss_notes.push(format!(
                "One part of that IDML file ({name}) was too large to read whole, \
                 so its words past a point were left out of this draft."
            ));
            raw.truncate(MAX_ENTRY as usize);
            // The cut can land inside a character; the
            // tail is already counted as lost, so a
            // lossy read of what is left is honest. The
            // prefix is then closed off (close_off_cut),
            // so everything complete before the cut is
            // still rescued instead of the whole part
            // reading as one unclosed, empty subtree.
            close_off_cut(&String::from_utf8_lossy(&raw))
        } else {
            String::from_utf8(raw).map_err(|_| {
                "cannot read a part of that IDML package: it is not text we can read".to_string()
            })?
        };
        files.insert(name, text);
    }
    let designmap = files
        .get("designmap.xml")
        .ok_or("that IDML package has no design map; it may be damaged")?;
    let dm = parse_xml(designmap)?;
    // Collect part paths from any attribute pointing at them.
    let mut spread_srcs: Vec<String> = Vec::new();
    fn collect(node: &Node, spread_srcs: &mut Vec<String>) {
        for v in node.attrs.values() {
            if v.starts_with("Spreads/") && v.ends_with(".xml") && !spread_srcs.contains(v) {
                spread_srcs.push(v.clone());
            }
        }
        for c in &node.children {
            collect(c, spread_srcs);
        }
    }
    collect(&dm, &mut spread_srcs);
    if spread_srcs.is_empty() {
        return Err("that IDML package lists no pages".into());
    }
    let styles = files
        .get("Resources/Styles.xml")
        .map(|x| style_table(x))
        .unwrap_or_default();

    let mut pages: Vec<PageInfo> = Vec::new();
    let mut spans: Vec<Span> = Vec::new();
    for src in &spread_srcs {
        let Some(spread_xml) = files.get(src) else {
            continue;
        };
        let spread = parse_xml(spread_xml)?;
        // Pages first: their bounds define the panels.
        let mut page_nodes = Vec::new();
        spread.find_all("Page", &mut page_nodes);
        struct PageRect {
            top: f32,
            left: f32,
            w: f32,
            h: f32,
            index: u32,
        }
        let mut rects: Vec<PageRect> = Vec::new();
        for pg in &page_nodes {
            if let Some((top, left, bottom, right)) = geometric_bounds(pg.attr("GeometricBounds")) {
                let index = pages.len() as u32;
                pages.push(PageInfo {
                    index,
                    width_pt: right - left,
                    height_pt: bottom - top,
                    picture_only: false,
                });
                rects.push(PageRect {
                    top,
                    left,
                    w: right - left,
                    h: bottom - top,
                    index,
                });
            }
        }
        // Text frames: find the page each belongs to by center.
        let mut frames = Vec::new();
        spread.find_all("TextFrame", &mut frames);
        for frame in frames {
            let Some(story_id) = frame.attr("ParentStory") else {
                continue;
            };
            let (tx, ty) = transform_xy(frame.attr("ItemTransform"));
            // Frame size from its path points.
            let mut pts = Vec::new();
            frame.find_all("PathPointType", &mut pts);
            let (mut minx, mut miny, mut maxx, mut maxy) = (0f32, 0f32, 0f32, 0f32);
            for pt in &pts {
                if let Some(a) = pt.attr("Anchor") {
                    let f = floats(a);
                    if f.len() == 2 {
                        minx = minx.min(f[0]);
                        miny = miny.min(f[1]);
                        maxx = maxx.max(f[0]);
                        maxy = maxy.max(f[1]);
                    }
                }
            }
            let fx = tx + minx;
            let fy = ty + miny;
            let fw = (maxx - minx).max(10.0);
            let fh = (maxy - miny).max(10.0);
            let cx = fx + fw / 2.0;
            let cy = fy + fh / 2.0;
            let Some(rect) = rects
                .iter()
                .find(|r| cx >= r.left && cx <= r.left + r.w && cy >= r.top && cy <= r.top + r.h)
            else {
                continue;
            };
            let Some(story_xml) = files.get(&format!("Stories/Story_{story_id}.xml")) else {
                continue;
            };
            let paras = story_paragraphs(story_xml)?;
            if paras.is_empty() {
                continue;
            }
            // One span per paragraph, stacked down the frame.
            let slice = fh / paras.len() as f32;
            for (k, para) in paras.iter().enumerate() {
                let mut font = para.font.clone();
                let mut size = para.size;
                let mut align = para.align;
                if let Some(sid) = &para.style_ref {
                    if let Some(def) = styles.get(sid) {
                        if font.is_none() {
                            font = def.font.clone();
                        }
                        if size.is_none() {
                            size = def.size;
                        }
                        if align.is_none() {
                            align = def.align;
                        }
                    }
                }
                spans.push(Span {
                    page: rect.index,
                    text: para.text.clone(),
                    trust: SpanTrust::RawText,
                    x: Some(fx - rect.left),
                    y: Some(fy - rect.top + slice * k as f32),
                    w: Some(fw),
                    h: Some(slice),
                    font_size: Some(size.unwrap_or(11.0)),
                    bold: Some(para.bold),
                    font,
                    // Schema v2: italic rides its own field,
                    // no longer smuggled through the font name
                    // as a "-Italic" suffix.
                    align,
                    italic: para.italic,
                    color: [0, 0, 0],
                    runs: para.runs.clone(),
                    thread: None,
                });
            }
        }
        // A page with images but no words reads as picture-only.
        let mut images = Vec::new();
        spread.find_all("Image", &mut images);
        if !images.is_empty() {
            for rect in &rects {
                if !spans.iter().any(|s| s.page == rect.index) {
                    pages[rect.index as usize].picture_only = true;
                }
            }
        }
    }
    Ok(ExtractionDump {
        schema_version: DUMP_SCHEMA_VERSION,
        source_kind: SourceKind::Idml,
        source_name: source_name.to_string(),
        source_bytes: bytes.len(),
        extractor: "openbulletin-idml".into(),
        pages,
        spans,
        images: Vec::new(),
        loss_notes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny_idml() -> Vec<u8> {
        use std::io::Write;
        let mut buf = Vec::new();
        {
            let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
            let opts = zip::write::SimpleFileOptions::default();
            let parts: &[(&str, &str)] = &[
                (
                    "designmap.xml",
                    r#"<?xml version="1.0"?><Document><Spread src="Spreads/Spread_u1.xml"/></Document>"#,
                ),
                (
                    "Resources/Styles.xml",
                    r#"<?xml version="1.0"?><Root><ParagraphStyleGroup><ParagraphStyle Self="ParagraphStyle/Body" PointSize="11"><Properties><AppliedFont type="string">Calibri</AppliedFont></Properties></ParagraphStyle></ParagraphStyleGroup></Root>"#,
                ),
                (
                    "Stories/Story_u9.xml",
                    r#"<?xml version="1.0"?><Story><ParagraphStyleRange AppliedParagraphStyle="ParagraphStyle/Body"><CharacterStyleRange FontStyle="Regular"><Content>Welcome to worship this morning</Content></CharacterStyleRange></ParagraphStyleRange><ParagraphStyleRange AppliedParagraphStyle="ParagraphStyle/Body"><CharacterStyleRange FontStyle="Bold"><Content>Announcements follow</Content></CharacterStyleRange></ParagraphStyleRange></Story>"#,
                ),
                (
                    "Spreads/Spread_u1.xml",
                    r#"<?xml version="1.0"?><Spread><Page Self="u2" GeometricBounds="0 0 612 396" ItemTransform="1 0 0 1 0 0"/><TextFrame Self="u3" ParentStory="u9" ItemTransform="1 0 0 1 36 48"><Properties><PathGeometry><GeometryPathType><PathPointArray><PathPointType Anchor="0 0"/><PathPointType Anchor="324 0"/><PathPointType Anchor="324 200"/><PathPointType Anchor="0 200"/></PathPointArray></GeometryPathType></PathGeometry></Properties></TextFrame></Spread>"#,
                ),
            ];
            for (name, body) in parts {
                zip.start_file(*name, opts).unwrap();
                zip.write_all(body.as_bytes()).unwrap();
            }
            zip.finish().unwrap();
        }
        buf
    }

    fn idml_with_story(story: &str) -> Vec<u8> {
        use std::io::Write;
        let mut buf = Vec::new();
        {
            let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
            let opts = zip::write::SimpleFileOptions::default();
            let parts: &[(&str, &str)] = &[
                (
                    "designmap.xml",
                    r#"<?xml version="1.0"?><Document><Spread src="Spreads/Spread_u1.xml"/></Document>"#,
                ),
                (
                    "Resources/Styles.xml",
                    r#"<?xml version="1.0"?><Root><ParagraphStyleGroup><ParagraphStyle Self="ParagraphStyle/Body" PointSize="11"><Properties><AppliedFont type="string">Calibri</AppliedFont></Properties></ParagraphStyle></ParagraphStyleGroup></Root>"#,
                ),
                ("Stories/Story_u9.xml", story),
                (
                    "Spreads/Spread_u1.xml",
                    r#"<?xml version="1.0"?><Spread><Page Self="u2" GeometricBounds="0 0 612 396" ItemTransform="1 0 0 1 0 0"/><TextFrame Self="u3" ParentStory="u9" ItemTransform="1 0 0 1 36 48"><Properties><PathGeometry><GeometryPathType><PathPointArray><PathPointType Anchor="0 0"/><PathPointType Anchor="324 0"/><PathPointType Anchor="324 200"/><PathPointType Anchor="0 200"/></PathPointArray></GeometryPathType></PathGeometry></Properties></TextFrame></Spread>"#,
                ),
            ];
            for (name, body) in parts {
                zip.start_file(*name, opts).unwrap();
                zip.write_all(body.as_bytes()).unwrap();
            }
            zip.finish().unwrap();
        }
        buf
    }

    #[test]
    fn idml_character_ranges_become_span_runs() {
        // One paragraph, three character ranges: the bold
        // word in the middle becomes a run of ONE span whose
        // text is the whole sentence (G1b).
        let story = r#"<?xml version="1.0"?><Story><ParagraphStyleRange AppliedParagraphStyle="ParagraphStyle/Body"><CharacterStyleRange FontStyle="Regular"><Content>Sing</Content></CharacterStyleRange><CharacterStyleRange FontStyle="Bold"><Content>hymn</Content></CharacterStyleRange><CharacterStyleRange FontStyle="Regular"><Content>432</Content></CharacterStyleRange></ParagraphStyleRange></Story>"#;
        let dump = dump_from_idml(&idml_with_story(story), "runs.idml").expect("dump");
        assert_eq!(dump.spans.len(), 1, "one paragraph, one span");
        let span = &dump.spans[0];
        assert_eq!(span.text, "Sing hymn 432");
        let concat: String = span.runs.iter().map(|r| r.text.as_str()).collect();
        assert_eq!(concat, span.text, "the runs concatenate to the span text");
        assert_eq!(span.runs.len(), 3);
        assert_eq!(
            span.runs[1].text.trim(),
            "hymn",
            "the middle run is the bold word (its joining space rides on it)"
        );
        assert_eq!(span.runs[1].bold, Some(true));
        assert_eq!(span.runs[0].bold, Some(false));
        // And the draft paragraph carries the run through.
        let out =
            crate::draft::document_from_dump(&dump, ob_model::ChurchProfile::default(), false, &[]);
        let para = out
            .doc
            .stories
            .iter()
            .flat_map(|st| &st.paragraphs)
            .find(|p| p.text().contains("hymn"))
            .expect("the paragraph exists");
        assert!(
            para.runs
                .iter()
                .any(|r| r.text.contains("hymn") && r.bold == Some(true)),
            "the draft paragraph keeps the bold word: {:?}",
            para.runs
        );
    }

    #[test]
    fn idml_dump_carries_words_frames_and_fonts() {
        let dump = dump_from_idml(&tiny_idml(), "old.idml").expect("dump");
        assert_eq!(dump.pages.len(), 1);
        assert_eq!(
            (dump.pages[0].width_pt, dump.pages[0].height_pt),
            (396.0, 612.0)
        );
        assert_eq!(dump.spans.len(), 2);
        let first = &dump.spans[0];
        assert_eq!(first.text, "Welcome to worship this morning");
        assert_eq!(first.x, Some(36.0));
        assert_eq!(first.font.as_deref(), Some("Calibri"));
        assert_eq!(dump.spans[1].bold, Some(true));
        // And the draft builder turns it into an editable draft.
        let out =
            crate::draft::document_from_dump(&dump, ob_model::ChurchProfile::default(), false, &[]);
        assert_eq!(out.doc.panels.len(), 1);
        assert_eq!(out.doc.panels[0].frames.len(), 1);
        assert!(out.doc.stories.iter().any(|st| st
            .paragraphs
            .iter()
            .any(|p| p.text().contains("Announcements"))));
    }

    #[test]
    fn idml_rejects_non_packages() {
        assert!(dump_from_idml(b"not a zip", "x.idml").is_err());
    }

    #[test]
    fn idml_justification_and_italic_ride_the_spans() {
        // The round trip our own exporter depends on: it
        // writes Justification="CenterAlign"; the importer
        // must read it back as Align::Center. Italic rides
        // its own field, not a "-Italic" font-name suffix.
        use std::io::Write;
        let mut buf = Vec::new();
        {
            let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
            let opts = zip::write::SimpleFileOptions::default();
            let parts: &[(&str, &str)] = &[
                (
                    "designmap.xml",
                    r#"<?xml version="1.0"?><Document><Spread src="Spreads/Spread_u1.xml"/></Document>"#,
                ),
                (
                    "Resources/Styles.xml",
                    r#"<?xml version="1.0"?><Root><ParagraphStyleGroup><ParagraphStyle Self="ParagraphStyle/Body" PointSize="11"><Properties><AppliedFont type="string">Calibri</AppliedFont></Properties></ParagraphStyle></ParagraphStyleGroup></Root>"#,
                ),
                (
                    "Stories/Story_u9.xml",
                    r#"<?xml version="1.0"?><Story><ParagraphStyleRange AppliedParagraphStyle="ParagraphStyle/Body" Justification="CenterAlign"><CharacterStyleRange FontStyle="Italic"><Content>A mighty fortress</Content></CharacterStyleRange></ParagraphStyleRange></Story>"#,
                ),
                (
                    "Spreads/Spread_u1.xml",
                    r#"<?xml version="1.0"?><Spread><Page Self="u2" GeometricBounds="0 0 612 396" ItemTransform="1 0 0 1 0 0"/><TextFrame Self="u3" ParentStory="u9" ItemTransform="1 0 0 1 36 48"><Properties><PathGeometry><GeometryPathType><PathPointArray><PathPointType Anchor="0 0"/><PathPointType Anchor="324 0"/><PathPointType Anchor="324 200"/><PathPointType Anchor="0 200"/></PathPointArray></GeometryPathType></PathGeometry></Properties></TextFrame></Spread>"#,
                ),
            ];
            for (name, body) in parts {
                zip.start_file(*name, opts).unwrap();
                zip.write_all(body.as_bytes()).unwrap();
            }
            zip.finish().unwrap();
        }
        let dump = dump_from_idml(&buf, "centered.idml").expect("dump");
        assert_eq!(dump.spans.len(), 1);
        assert_eq!(dump.spans[0].align, Some(ob_model::Align::Center));
        assert!(dump.spans[0].italic);
        assert_eq!(dump.spans[0].font.as_deref(), Some("Calibri"));
    }

    fn idml_from_parts(parts: &[(&str, Vec<u8>)]) -> Vec<u8> {
        use std::io::Write;
        let mut buf = Vec::new();
        {
            let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
            let opts = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);
            for (name, body) in parts {
                zip.start_file(*name, opts).unwrap();
                zip.write_all(body).unwrap();
            }
            zip.finish().unwrap();
        }
        buf
    }

    #[test]
    fn idml_parts_past_the_total_cap_refuse_in_plain_words() {
        // Round 2, RT-4: many parts, each under the
        // per-part cap, whose decompressed sum passes
        // the 512 MB total cap (the lane's shape: a
        // 6.5 MB package inflating toward 6.4 GB was
        // refused only by the allocator). The parts are
        // one repeated byte, so the package stays tiny
        // on disk while its insides are huge; the
        // refusal must come from the budget, in plain
        // words, while reading.
        let filler = vec![b'a'; 31 * 1024 * 1024];
        let names: Vec<String> = (0..17).map(|k| format!("Stories/Filler_{k}.xml")).collect();
        let mut parts: Vec<(&str, Vec<u8>)> = vec![
            (
                "designmap.xml",
                br#"<?xml version="1.0"?><Document><Spread src="Spreads/Spread_u1.xml"/></Document>"#
                    .to_vec(),
            ),
            (
                "Spreads/Spread_u1.xml",
                br#"<?xml version="1.0"?><Spread><Page Self="u2" GeometricBounds="0 0 612 396" ItemTransform="1 0 0 1 0 0"/></Spread>"#
                    .to_vec(),
            ),
        ];
        // 17 parts of 31 MB: 527 MB of insides.
        for name in &names {
            parts.push((name.as_str(), filler.clone()));
        }
        let bytes = idml_from_parts(&parts);
        assert!(
            bytes.len() < 32 * 1024 * 1024,
            "the package itself stays small: {}",
            bytes.len()
        );
        let err = dump_from_idml(&bytes, "bomb.idml").expect_err("refused by the total cap");
        assert!(
            err.contains("insides are too large to read safely"),
            "{err}"
        );
    }

    #[test]
    fn idml_a_part_past_its_cap_is_counted_not_silent() {
        // Round 2, RT-5: a story that runs past the
        // 32 MB per-part cap used to be cut mid-stream
        // and read as if whole; the words past the cap
        // vanished with no count. Now the words before
        // the cap are still rescued, the loss is counted
        // in the dump's loss notes, and the draft's
        // review carries it where the admin can see it.
        // (The padding is many complete ranges, so where
        // the cut lands only whole ranges are lost.)
        let para = |words: &str| {
            format!(
                "<ParagraphStyleRange AppliedParagraphStyle=\"ParagraphStyle/Body\">\
                 <CharacterStyleRange FontStyle=\"Regular\"><Content>{words}</Content>\
                 </CharacterStyleRange></ParagraphStyleRange>"
            )
        };
        let filler_para = format!(
            "<ParagraphStyleRange AppliedParagraphStyle=\"ParagraphStyle/Body\">\
             <CharacterStyleRange FontStyle=\"Regular\"><Content>{}</Content>\
             </CharacterStyleRange></ParagraphStyleRange>",
            "x".repeat(4_000)
        );
        let mut story = String::from("<?xml version=\"1.0\"?><Story>");
        story.push_str(&para("Words before the flood"));
        for _ in 0..8_500 {
            story.push_str(&filler_para);
        }
        story.push_str(&para("ZZZAFTERCAP words"));
        story.push_str("</Story>");
        assert!(story.len() > 33 * 1024 * 1024, "the story passes the cap");
        let dump = dump_from_idml(&idml_with_story(&story), "long.idml").expect("dump");
        assert!(
            dump.spans
                .iter()
                .any(|s| s.text.contains("Words before the flood")),
            "the words before the cap are rescued"
        );
        assert!(
            !dump.spans.iter().any(|s| s.text.contains("ZZZAFTERCAP")),
            "the words past the cap did not come over"
        );
        assert_eq!(
            dump.loss_notes.len(),
            1,
            "the loss is counted: {:?}",
            dump.loss_notes
        );
        assert!(
            dump.loss_notes[0].contains("Story_u9"),
            "the note names the part: {}",
            dump.loss_notes[0]
        );
        let out =
            crate::draft::document_from_dump(&dump, ob_model::ChurchProfile::default(), false, &[]);
        assert!(
            out.items.iter().any(|i| {
                i.badge == crate::draft::Badge::CheckThis
                    && i.label == "Part of the old file was left out"
                    && i.detail.contains("Story_u9")
            }),
            "the review carries the counted loss: {:?}",
            out.items.iter().map(|i| &i.label).collect::<Vec<_>>()
        );
    }

    #[test]
    fn idml_a_story_cut_inside_a_comment_is_counted_too() {
        // The lane's exact shape: the story passes the
        // cap inside one long comment, the real
        // paragraph after it never read. The paragraph
        // closed before the comment is still rescued
        // (the cut is closed off), the loss is counted,
        // and the words past the cap stay out.
        let mut story = String::from("<?xml version=\"1.0\"?><Story>");
        story.push_str(
            "<ParagraphStyleRange AppliedParagraphStyle=\"ParagraphStyle/Body\">\
             <CharacterStyleRange FontStyle=\"Regular\"><Content>Words before the flood</Content>\
             </CharacterStyleRange></ParagraphStyleRange>",
        );
        story.push_str("<!--");
        story.push_str(&"x".repeat(33 * 1024 * 1024));
        story.push_str("-->");
        story.push_str(
            "<ParagraphStyleRange AppliedParagraphStyle=\"ParagraphStyle/Body\">\
             <CharacterStyleRange FontStyle=\"Regular\"><Content>ZZZAFTERCAP words</Content>\
             </CharacterStyleRange></ParagraphStyleRange>",
        );
        story.push_str("</Story>");
        let dump = dump_from_idml(&idml_with_story(&story), "comment.idml").expect("dump");
        assert!(
            dump.spans
                .iter()
                .any(|s| s.text.contains("Words before the flood")),
            "the paragraph closed before the comment is rescued"
        );
        assert!(
            !dump.spans.iter().any(|s| s.text.contains("ZZZAFTERCAP")),
            "the words past the cap did not come over"
        );
        assert_eq!(
            dump.loss_notes.len(),
            1,
            "the loss is counted: {:?}",
            dump.loss_notes
        );
    }

    #[test]
    fn idml_deeply_nested_ranges_refuse_instead_of_aborting() {
        // Audit Batch 1: character ranges nested thousands
        // deep used to overflow the stack and kill the whole
        // app. The hostile shape is built from string pieces
        // here (never a checked-in hostile file); the import
        // must refuse it whole, in plain words.
        let mut story = String::from(
            r#"<?xml version="1.0"?><Story><ParagraphStyleRange AppliedParagraphStyle="ParagraphStyle/Body">"#,
        );
        for _ in 0..2_000 {
            story.push_str("<CharacterStyleRange>");
        }
        story.push_str("<Content>deep words</Content>");
        for _ in 0..2_000 {
            story.push_str("</CharacterStyleRange>");
        }
        story.push_str("</ParagraphStyleRange></Story>");
        let err = dump_from_idml(&idml_with_story(&story), "deep.idml").expect_err("refused");
        assert!(err.contains("nested too deeply"), "{err}");
    }
}
