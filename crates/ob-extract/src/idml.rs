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

/// Back a cut text off to just before a construct it
/// ends inside (round 3, RT3-C; round 4, R4-11).
/// Two constructs can swallow the closing tags the
/// cut rescue appends as their own text: a CDATA
/// section and an XML comment. Inside either, nothing
/// is markup, so a story ending inside one parses to
/// nothing at all and even paragraphs finished long
/// before it are lost. Whichever construct opens
/// first owns the text until it closes: a "<![CDATA["
/// sitting inside a comment is comment text, not a
/// section opener (a story that mentioned one in a
/// comment used to lose everything from the comment
/// onward), and a "-->" inside a section is section
/// text, not a comment's end. Closed constructs
/// earlier in the text are left alone; only one the
/// text ends inside costs its opening, and everything
/// before it stands.
fn cdata_backoff(text: &str) -> &str {
    let mut from = 0;
    loop {
        let comment = text[from..].find("<!--").map(|o| from + o);
        let section = text[from..].find("<![CDATA[").map(|o| from + o);
        let (open, opener, closer) = match (comment, section) {
            (None, None) => return text,
            (Some(c), None) => (c, "<!--", "-->"),
            (None, Some(s)) => (s, "<![CDATA[", "]]>"),
            (Some(c), Some(s)) if c < s => (c, "<!--", "-->"),
            (Some(_), Some(s)) => (s, "<![CDATA[", "]]>"),
        };
        let body = open + opener.len();
        let Some(close) = text[body..].find(closer).map(|c| body + c) else {
            return &text[..open];
        };
        from = close + closer.len();
    }
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
/// The cut is first backed out of any CDATA section
/// or comment it landed inside (cdata_backoff): the
/// closing tags appended here would otherwise land
/// inside the construct and count for nothing.
fn close_off_cut(text: &str) -> String {
    let text = cdata_backoff(text);
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

/// One part's decompressed bytes off its raw spot,
/// read one byte past the per-part cap: two copies
/// are judged on the bytes the importer itself would
/// read (a part past the cap is cut at the cap, so
/// the comparison rides to the same point). None
/// when the payload cannot be read back at all.
fn raw_payload(bytes: &[u8], spot: &ob_model::zip_directory::CentralSpot) -> Option<Vec<u8>> {
    use std::io::Read as _;
    // The span is the shared checked computation
    // (round 36, F7): this copy's plain additions on
    // the spot's zip64-resolved offset panicked on an
    // offset near u64::MAX.
    let (data_start, data_end) = ob_model::zip_directory::payload_span(bytes, spot)?;
    let raw = &bytes[data_start..data_end];
    match spot.method {
        0 => Some(raw[..(raw.len() as u64).min(MAX_ENTRY + 1) as usize].to_vec()),
        8 => {
            let mut decoder = flate2::read::DeflateDecoder::new(raw);
            let mut out = Vec::new();
            decoder
                .by_ref()
                .take(MAX_ENTRY + 1)
                .read_to_end(&mut out)
                .ok()?;
            Some(out)
        }
        _ => None,
    }
}

/// The first XML part name the package carries more
/// than once whose copies' decompressed bytes
/// disagree (round 31, F5): the importer's map would
/// serve only the last copy, so which story a draft
/// was read from must never be the archive's quiet
/// choice. Exact byte-identical repeats pass — the
/// same words whichever copy is served — and a copy
/// whose bytes cannot be read back cannot be proven
/// identical, so it does not count as exact either.
fn disagreeing_duplicate_part(bytes: &[u8]) -> Option<String> {
    let spots = ob_model::zip_directory::central_spots(bytes);
    let mut checked: Vec<&str> = Vec::new();
    for (name, _) in &spots {
        if !name.ends_with(".xml") || checked.contains(&name.as_str()) {
            continue;
        }
        checked.push(name.as_str());
        let copies: Vec<&ob_model::zip_directory::CentralSpot> = spots
            .iter()
            .filter(|(n, _)| n == name)
            .map(|(_, spot)| spot)
            .collect();
        if copies.len() < 2 {
            continue;
        }
        let first_payload = raw_payload(bytes, copies[0]);
        if first_payload.is_none() {
            return Some(name.clone());
        }
        for spot in &copies[1..] {
            if raw_payload(bytes, spot) != first_payload {
                return Some(name.clone());
            }
        }
    }
    None
}

pub fn dump_from_idml(bytes: &[u8], source_name: &str) -> Result<ExtractionDump, String> {
    // One prefix-free view of the package for both the
    // reader and the duplicate scan below (round 33,
    // F3): a shifted package's junk is cut away by the
    // shared derivation, so neither consumer ever
    // traverses the window between the declared
    // directory offset and the real directory, where
    // signature bytes in a stored part's tail used to
    // poison both. The dump still counts the bytes as
    // they arrived, junk included.
    let source_bytes = bytes.len();
    let directory_derived = ob_model::zip_directory::central_directory(bytes).is_some();
    let view = ob_model::zip_directory::prefix_free(bytes);
    let bytes: &[u8] = &view;
    let cursor = std::io::Cursor::new(bytes);
    let mut zip = zip::ZipArchive::new(cursor)
        .map_err(|e| format!("that is not an IDML package (a zip of XML): {e}"))?;
    if zip.len() > MAX_ENTRIES {
        return Err("that IDML package has too many parts".into());
    }
    // Round 34, F2: the reader opened the package, so
    // its entries will be served — but no directory
    // could be derived for the duplicate scan below,
    // which would judge an empty directory while the
    // reader serves those entries (the steered hunt
    // used to land exactly here). Fail closed in this
    // door's reader-refusal words. A package the reader
    // cannot open refuses on its own above; a genuinely
    // empty package derives its empty directory and is
    // untouched.
    if !zip.is_empty() && !directory_derived {
        return Err(
            "that is not an IDML package (a zip of XML): its central directory cannot be derived"
                .into(),
        );
    }
    // A duplicated part whose copies disagree refuses
    // before any part is read (round 31, F5), in the
    // .obd door's words: the reader's map below has
    // already collapsed each name to its last copy, so
    // the count stands on the raw directory above.
    if let Some(name) = disagreeing_duplicate_part(bytes) {
        return Err(format!(
            "that file has two parts named {name}, so the file was not opened; \
             it may be damaged."
        ));
    }
    let mut files: HashMap<String, String> = HashMap::new();
    let mut loss_notes: Vec<String> = Vec::new();
    // Parts cut at the per-part cap, in archive order;
    // their loss notes are written once the walk shows
    // what the rescue kept (see the note loop below).
    let mut cut_parts: Vec<String> = Vec::new();
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
        // The read goes through ob-model's one capped
        // read (health audit E-9). This door's judgment
        // is Note: a part past its cap is cut to the
        // cap and named in `cut_parts` below, and its
        // loss is counted in the dump's loss notes.
        let read = ob_model::read_capped(&mut entry, MAX_ENTRY, ob_model::CapOverflow::Note)
            .map_err(|e| match e {
                ob_model::CappedReadError::Unreadable(e) => {
                    format!("cannot read a part of that IDML package: {e}")
                }
                // The Note judgment never refuses; this
                // arm only completes the match.
                ob_model::CappedReadError::Refused(words) => words,
            })?;
        // The total counts raw bytes as they arrived:
        // a noted part arrived as cap + 1 bytes, the
        // probe byte included, exactly as the inline
        // read this replaces counted it.
        total_read += read.bytes.len() as u64 + u64::from(read.overflowed);
        if total_read > MAX_TOTAL {
            return Err("that IDML file's insides are too large to read safely".into());
        }
        let text = if read.overflowed {
            // The loss note is written after the walk,
            // not here (round 3, RT3-C): only then is it
            // known whether the rescue kept any of this
            // part's words, and the note must say which.
            cut_parts.push(name.clone());
            // The cut can land inside a character; the
            // tail is already counted as lost, so a
            // lossy read of what is left is honest. The
            // prefix is then closed off (close_off_cut),
            // so everything complete before the cut is
            // still rescued instead of the whole part
            // reading as one unclosed, empty subtree.
            close_off_cut(&String::from_utf8_lossy(&read.bytes))
        } else {
            String::from_utf8(read.bytes).map_err(|_| {
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
    // Story parts the walk actually kept words from:
    // the honest half of the cut parts' loss notes.
    let mut rescued: std::collections::HashSet<String> = std::collections::HashSet::new();
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
            let story_name = format!("Stories/Story_{story_id}.xml");
            let Some(story_xml) = files.get(&story_name) else {
                continue;
            };
            let paras = story_paragraphs(story_xml)?;
            if paras.is_empty() {
                continue;
            }
            rescued.insert(story_name);
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
    // One loss note per cut part, worded by what the
    // rescue actually kept (round 3, RT3-C): the old
    // note always claimed a tail trim ("words past a
    // point were left out"), even when a cut landing
    // inside a CDATA section had cost the part every
    // word it held. A story part that yielded nothing
    // is counted whole; any other cut part, including
    // a story the walk kept words from, keeps the
    // tail-trim words, which are then true.
    for name in &cut_parts {
        if name.starts_with("Stories/") && !rescued.contains(name) {
            loss_notes.push(format!(
                "One part of that IDML file ({name}) was too large to read whole, \
                 and none of its words could be kept; the whole part was left out \
                 of this draft."
            ));
        } else {
            loss_notes.push(format!(
                "One part of that IDML file ({name}) was too large to read whole, \
                 so its words past a point were left out of this draft."
            ));
        }
    }
    Ok(ExtractionDump {
        schema_version: DUMP_SCHEMA_VERSION,
        source_kind: SourceKind::Idml,
        source_name: source_name.to_string(),
        source_bytes,
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

    fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        haystack.windows(needle.len()).position(|w| w == needle)
    }

    /// Rename one entry everywhere its name is written
    /// (the local header and the central directory),
    /// the two names the same length: the zip writer
    /// refuses a duplicate name outright, so a package
    /// carrying a repeated part is assembled by patch,
    /// the same helper shape as ob-format's tests.
    fn patch_entry_name(zip_bytes: &mut [u8], from_name: &str, to_name: &str) {
        assert_eq!(from_name.len(), to_name.len(), "the patch keeps lengths");
        let name = from_name.as_bytes();
        let mut patched = 0;
        let mut from = 0;
        while let Some(found) = find_bytes(&zip_bytes[from..], name) {
            let at = from + found;
            let guarded = (at >= 30 && &zip_bytes[at - 30..at - 26] == b"PK\x03\x04")
                || (at >= 46 && &zip_bytes[at - 46..at - 42] == b"PK\x01\x02");
            if guarded {
                zip_bytes[at..at + name.len()].copy_from_slice(to_name.as_bytes());
                patched += 1;
            }
            from = at + 1;
        }
        assert_eq!(patched, 2, "both records for {from_name} were patched");
    }

    fn idml_with_duplicate_story(story_a: &str, story_b: &str) -> Vec<u8> {
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
                ("Stories/Story_u9.xml", story_a),
                ("Stories/Story_u9.xmX", story_b),
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
        patch_entry_name(&mut buf, "Stories/Story_u9.xmX", "Stories/Story_u9.xml");
        buf
    }

    fn one_paragraph_story(words: &str) -> String {
        format!(
            r#"<?xml version="1.0"?><Story><ParagraphStyleRange AppliedParagraphStyle="ParagraphStyle/Body"><CharacterStyleRange FontStyle="Regular"><Content>{words}</Content></CharacterStyleRange></ParagraphStyleRange></Story>"#
        )
    }

    #[test]
    fn idml_duplicate_story_entries_with_different_text_error() {
        // Round 31, F5: two copies of one story part
        // carrying different words; the importer used
        // to keep the last copy quietly. It refuses
        // now, naming the duplicated part.
        let bytes = idml_with_duplicate_story(
            &one_paragraph_story("First story words"),
            &one_paragraph_story("Other story words"),
        );
        let err = dump_from_idml(&bytes, "dup.idml").expect_err("disagreeing duplicates refuse");
        assert!(
            err.contains("two parts named Stories/Story_u9.xml"),
            "{err}"
        );
        assert!(err.contains("may be damaged"), "{err}");
    }

    #[test]
    fn idml_exact_duplicate_story_entries_load() {
        // Round 31, F5, the other side: a byte-identical
        // story repeat loads — the same words whichever
        // copy is served.
        let story = one_paragraph_story("Words both copies carry");
        let bytes = idml_with_duplicate_story(&story, &story);
        let dump = dump_from_idml(&bytes, "same.idml").expect("exact repeats load");
        assert_eq!(dump.spans.len(), 1);
        assert_eq!(dump.spans[0].text, "Words both copies carry");
    }

    /// Prepend `junk` before a package's bytes without
    /// touching any declared offset: the self-extracting
    /// shape (round 32, F4), where every offset in the
    /// archive is declared relative to the archive's own
    /// start and the reader derives the shift itself.
    fn shifted(bytes: &[u8], junk: &[u8]) -> Vec<u8> {
        let mut out = junk.to_vec();
        out.extend_from_slice(bytes);
        out
    }

    #[test]
    fn idml_shifted_duplicate_story_entries_with_different_text_error() {
        // Round 32, F4 (review MINOR-1, witness
        // shifted_idml_dup.idml): the unshifted package
        // below refuses (the standing pin above), but
        // with 16 junk bytes prepended the scan walked
        // exactly the declared cd_offset — inside the
        // last entry's body — broke on its first check,
        // and returned no spots, so no duplicate check
        // happened and the importer served the last
        // copy ("Other story words"). The scan derives
        // the archive offset the way the reader does
        // now, and the shifted pair refuses in the same
        // words.
        let bytes = idml_with_duplicate_story(
            &one_paragraph_story("First story words"),
            &one_paragraph_story("Other story words"),
        );
        let bytes = shifted(&bytes, b"JUNKJUNKJUNKJUNK");
        let err = dump_from_idml(&bytes, "shifted.idml").expect_err("shifted duplicates refuse");
        assert!(
            err.contains("two parts named Stories/Story_u9.xml"),
            "{err}"
        );
        assert!(err.contains("may be damaged"), "{err}");
    }

    #[test]
    fn idml_shifted_package_without_duplicates_loads() {
        // Round 32, F4, the control beside the witness:
        // the same prepended shape carrying no duplicate
        // loads — the reader tolerates the shift, and so
        // does the importer; the pair above is refused
        // for its duplication, not for its shift.
        let bytes = shifted(&tiny_idml(), b"JUNKJUNKJUNKJUNK");
        let dump = dump_from_idml(&bytes, "shifted.idml").expect("a shifted package loads");
        assert_eq!(dump.spans.len(), 2);
        assert_eq!(dump.spans[0].text, "Welcome to worship this morning");
    }

    /// A Stored filler entry's payload in the round-33
    /// witness's shape: 600 bytes whose last 64 carry a
    /// fake central-directory signature, as a fake
    /// header whose name length is 0xFFFF, as a
    /// well-formed fake record naming `JUNK`, or not at
    /// all (the plain control).
    fn tail_filler(shape: &str) -> Vec<u8> {
        let mut payload = vec![b'A'; 600];
        if shape != "plain" {
            let at = 600 - 64;
            payload[at..at + 4].copy_from_slice(b"PK\x01\x02");
            if shape == "wellformed" {
                for byte in &mut payload[at + 4..at + 46] {
                    *byte = b'B';
                }
                payload[at + 28..at + 30].copy_from_slice(&4u16.to_le_bytes());
                payload[at + 30..at + 34].copy_from_slice(&0u32.to_le_bytes());
                payload[at + 46..at + 50].copy_from_slice(b"JUNK");
            } else {
                payload[at + 28..at + 30].copy_from_slice(&0xFFFFu16.to_le_bytes());
            }
        }
        payload
    }

    /// The duplicate-story package with a Stored
    /// filler entry LAST (its payload's tail is the
    /// window between the declared directory offset
    /// and the real directory once the package is
    /// shifted), then shifted by 64 junk bytes.
    fn idml_shifted_with_filler(stories: &[&str], filler: &[u8]) -> Vec<u8> {
        use std::io::Write;
        let designmap =
            r#"<?xml version="1.0"?><Document><Spread src="Spreads/Spread_u1.xml"/></Document>"#;
        let styles = r#"<?xml version="1.0"?><Root><ParagraphStyleGroup><ParagraphStyle Self="ParagraphStyle/Body" PointSize="11"><Properties><AppliedFont type="string">Calibri</AppliedFont></Properties></ParagraphStyle></ParagraphStyleGroup></Root>"#;
        let spread = r#"<?xml version="1.0"?><Spread><Page Self="u2" GeometricBounds="0 0 612 396" ItemTransform="1 0 0 1 0 0"/><TextFrame Self="u3" ParentStory="u9" ItemTransform="1 0 0 1 36 48"><Properties><PathGeometry><GeometryPathType><PathPointArray><PathPointType Anchor="0 0"/><PathPointType Anchor="324 0"/><PathPointType Anchor="324 200"/><PathPointType Anchor="0 200"/></PathPointArray></GeometryPathType></PathGeometry></Properties></TextFrame></Spread>"#;
        let mut buf = Vec::new();
        {
            let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
            let opts = zip::write::SimpleFileOptions::default();
            let stored = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            let mut parts: Vec<(&str, &[u8])> = vec![
                ("designmap.xml", designmap.as_bytes()),
                ("Resources/Styles.xml", styles.as_bytes()),
            ];
            if stories.len() > 1 {
                parts.push(("Stories/Story_u9.xml", stories[0].as_bytes()));
                parts.push(("Stories/Story_u9.xmX", stories[1].as_bytes()));
            } else {
                parts.push(("Stories/Story_u9.xml", stories[0].as_bytes()));
            }
            parts.push(("Spreads/Spread_u1.xml", spread.as_bytes()));
            for (name, body) in &parts {
                zip.start_file(*name, opts).unwrap();
                zip.write_all(body).unwrap();
            }
            zip.start_file("filler.bin", stored).unwrap();
            zip.write_all(filler).unwrap();
            zip.finish().unwrap();
        }
        if stories.len() > 1 {
            patch_entry_name(&mut buf, "Stories/Story_u9.xmX", "Stories/Story_u9.xml");
        }
        shifted(&buf, &[b'J'; 64])
    }

    #[test]
    fn r33c_shifted_packages_with_signature_in_the_stored_tail_are_judged_like_unshifted() {
        // Round 33, F3 (red team RT33-3): a shifted
        // package whose last entry is Stored and whose
        // payload carries PK\x01\x02 in the window
        // between the declared directory offset and the
        // real directory made ZipArchive::new itself
        // fail ("Could not find EOCD"), so the whole
        // package was refused before any duplicate
        // judgment. Both witnessed payload shapes (and
        // the plain shifted control) are judged exactly
        // as their unshifted twins now: disagreeing
        // duplicates refuse in the duplicate words, and
        // the same shifted shape without duplicates
        // loads. A genuinely corrupt package still
        // refuses at the reader.
        let story_a = one_paragraph_story("First story words");
        let story_b = one_paragraph_story("Other story words");
        for shape in ["plain", "fake", "wellformed"] {
            let filler = tail_filler(shape);
            let dup = idml_shifted_with_filler(&[&story_a, &story_b], &filler);
            let err = dump_from_idml(&dup, "witness.idml")
                .expect_err("shifted duplicates refuse, every shape");
            assert!(
                err.contains("two parts named Stories/Story_u9.xml"),
                "{shape}: {err}"
            );
            assert!(err.contains("may be damaged"), "{shape}: {err}");
            let single = idml_shifted_with_filler(&[&story_a], &filler);
            let dump = dump_from_idml(&single, "witness.idml")
                .expect("the shifted package without duplicates loads, every shape");
            assert_eq!(dump.spans.len(), 1, "{shape}");
            assert_eq!(dump.spans[0].text, "First story words", "{shape}");
            assert_eq!(dump.source_bytes, single.len(), "{shape}");
        }
        let mut corrupt = idml_shifted_with_filler(&[&story_a], &tail_filler("plain"));
        corrupt.truncate(corrupt.len() - 10);
        let err = dump_from_idml(&corrupt, "corrupt.idml").expect_err("a corrupt package refuses");
        assert!(err.contains("not an IDML package"), "{err}");
    }

    /// CRC-32 (IEEE) of a Stored entry's bytes, for the
    /// hand-assembled archives below: the readers check
    /// a stored entry's checksum as they read it.
    fn stored_crc(data: &[u8]) -> u32 {
        let mut crc: u32 = 0xFFFF_FFFF;
        for &byte in data {
            crc ^= u32::from(byte);
            for _ in 0..8 {
                crc = (crc >> 1) ^ (0xEDB8_8320 & 0u32.wrapping_sub(crc & 1));
            }
        }
        !crc
    }

    /// A zip64 archive assembled byte by byte, its
    /// zip64 record carrying an extensible data sector
    /// of `sector` bytes (size field 44 + sector, so
    /// the arithmetic derivation refuses it and the
    /// hunted fallback is the only path), all entries
    /// Stored, optionally shifted by `shift` junk
    /// bytes. The crafter's twin of the audit lanes'
    /// mkzip64 assembly (round 34).
    fn sector_idml(entries: &[(&[u8], &[u8])], sector: usize, shift: usize) -> Vec<u8> {
        let mut out: Vec<u8> = Vec::new();
        let mut centrals: Vec<u8> = Vec::new();
        for (name, data) in entries {
            let local_offset = out.len() as u32;
            let crc = stored_crc(data);
            out.extend_from_slice(b"PK\x03\x04");
            out.extend_from_slice(&20u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&crc.to_le_bytes());
            out.extend_from_slice(&(data.len() as u32).to_le_bytes());
            out.extend_from_slice(&(data.len() as u32).to_le_bytes());
            out.extend_from_slice(&(name.len() as u16).to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(name);
            out.extend_from_slice(data);

            centrals.extend_from_slice(b"PK\x01\x02");
            centrals.extend_from_slice(&45u16.to_le_bytes());
            centrals.extend_from_slice(&20u16.to_le_bytes());
            centrals.extend_from_slice(&0u16.to_le_bytes());
            centrals.extend_from_slice(&0u16.to_le_bytes());
            centrals.extend_from_slice(&0u16.to_le_bytes());
            centrals.extend_from_slice(&0u16.to_le_bytes());
            centrals.extend_from_slice(&crc.to_le_bytes());
            centrals.extend_from_slice(&(data.len() as u32).to_le_bytes());
            centrals.extend_from_slice(&(data.len() as u32).to_le_bytes());
            centrals.extend_from_slice(&(name.len() as u16).to_le_bytes());
            centrals.extend_from_slice(&0u16.to_le_bytes());
            centrals.extend_from_slice(&0u16.to_le_bytes());
            centrals.extend_from_slice(&0u16.to_le_bytes());
            centrals.extend_from_slice(&0u16.to_le_bytes());
            centrals.extend_from_slice(&0u32.to_le_bytes());
            centrals.extend_from_slice(&local_offset.to_le_bytes());
            centrals.extend_from_slice(name);
        }
        let cd_offset = out.len() as u64;
        let cd_size = centrals.len() as u64;
        out.extend_from_slice(&centrals);
        let record_at = out.len() as u64;
        out.extend_from_slice(b"PK\x06\x06");
        out.extend_from_slice(&((44 + sector) as u64).to_le_bytes());
        out.extend_from_slice(&45u16.to_le_bytes());
        out.extend_from_slice(&45u16.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&(entries.len() as u64).to_le_bytes());
        out.extend_from_slice(&(entries.len() as u64).to_le_bytes());
        out.extend_from_slice(&cd_size.to_le_bytes());
        out.extend_from_slice(&cd_offset.to_le_bytes());
        out.extend_from_slice(&vec![b'S'; sector]);
        out.extend_from_slice(b"PK\x06\x07");
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&record_at.to_le_bytes());
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(b"PK\x05\x06");
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&u16::MAX.to_le_bytes());
        out.extend_from_slice(&u16::MAX.to_le_bytes());
        out.extend_from_slice(&u32::MAX.to_le_bytes());
        out.extend_from_slice(&u32::MAX.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        if shift == 0 {
            out
        } else {
            shifted(&out, &vec![b'J'; shift])
        }
    }

    /// The last entry name in the red team's name-fake
    /// witnesses: see the twin helper in ob-model's
    /// zip_directory tests.
    fn fake_record_name() -> Vec<u8> {
        let mut name = vec![b'A'; 30];
        name.extend_from_slice(b"PK\x06\x06");
        name.extend_from_slice(&52u64.to_le_bytes());
        name.extend_from_slice(&45u16.to_le_bytes());
        name.extend_from_slice(&45u16.to_le_bytes());
        name.extend_from_slice(&0u32.to_le_bytes());
        name.extend_from_slice(&0u32.to_le_bytes());
        name.extend_from_slice(&2u64.to_le_bytes());
        name.extend_from_slice(&2u64.to_le_bytes());
        name.extend_from_slice(&257u64.to_le_bytes());
        name.extend_from_slice(&0x7F7F7F7Fu64.to_le_bytes());
        name
    }

    /// The IDML parts of the round-34 witnesses, in
    /// archive order: the package's real parts, the
    /// given stories, then the spread; the caller
    /// appends its tail entry (filler or fake name).
    fn witness_parts(stories: &[String]) -> Vec<(Vec<u8>, Vec<u8>)> {
        let designmap =
            r#"<?xml version="1.0"?><Document><Spread src="Spreads/Spread_u1.xml"/></Document>"#;
        let styles =
            r#"<?xml version="1.0"?><Root><ParagraphStyleGroup></ParagraphStyleGroup></Root>"#;
        let spread = r#"<?xml version="1.0"?><Spread><Page Self="u2" GeometricBounds="0 0 612 396" ItemTransform="1 0 0 1 0 0"/><TextFrame Self="u3" ParentStory="u9" ItemTransform="1 0 0 1 36 48"><Properties><PathGeometry><GeometryPathType><PathPointArray><PathPointType Anchor="0 0"/><PathPointType Anchor="324 0"/><PathPointType Anchor="324 200"/><PathPointType Anchor="0 200"/></PathPointArray></GeometryPathType></PathGeometry></Properties></TextFrame></Spread>"#;
        let mut parts: Vec<(Vec<u8>, Vec<u8>)> = vec![
            (b"designmap.xml".to_vec(), designmap.as_bytes().to_vec()),
            (b"Resources/Styles.xml".to_vec(), styles.as_bytes().to_vec()),
        ];
        for story in stories {
            parts.push((b"Stories/Story_u9.xml".to_vec(), story.as_bytes().to_vec()));
        }
        parts.push((
            b"Spreads/Spread_u1.xml".to_vec(),
            spread.as_bytes().to_vec(),
        ));
        parts
    }

    #[test]
    fn r34b_sector_shifted_packages_with_fakes_in_the_hunts_windows_are_judged_like_unshifted() {
        // Round 34, F2 (review MINOR-1 + red team
        // RT34-3): on a shifted zip64 archive with an
        // extensible data sector, the hunted fallback
        // validated nothing — a fake central signature
        // in the last entry's stored tail became the
        // directory the scans judged, and a fake zip64
        // record planted in the last record's name
        // steered the hunt to no directory at all, so
        // this package's disagreeing duplicate story
        // was SERVED ("Other story words here") while
        // its unshifted twin refused. The hunt validates
        // its record against the locator and its
        // directory by a walk now, and keeps searching
        // past fakes: every shape below is judged
        // exactly like its unshifted twin — the
        // duplicate refuses in the standing words, the
        // single story loads.
        let story_a = one_paragraph_story("First story words here");
        let story_b = one_paragraph_story("Other story words here");
        // (label, sector, tail entry) — the payload
        // shapes ride the review witness (sector 16,
        // the round-33 fillers in a Stored tail entry),
        // the name shape rides the red team's (sector
        // 8, the fake record standing in the last
        // record's name, which is the tail entry here).
        let fake_name = fake_record_name();
        let tails: Vec<(&str, usize, Vec<u8>, Vec<u8>)> = vec![
            (
                "payload-fake-header",
                16,
                b"filler.bin".to_vec(),
                tail_filler("fake"),
            ),
            (
                "payload-wellformed",
                16,
                b"filler.bin".to_vec(),
                tail_filler("wellformed"),
            ),
            (
                "payload-clean",
                16,
                b"filler.bin".to_vec(),
                tail_filler("plain"),
            ),
            ("name-fake", 8, fake_name.clone(), b"tail".to_vec()),
        ];
        for (label, sector, tail_name, tail_data) in &tails {
            for shift in [0usize, 64] {
                let mut parts = witness_parts(&[story_a.clone(), story_b.clone()]);
                parts.push((tail_name.clone(), tail_data.clone()));
                let entries: Vec<(&[u8], &[u8])> = parts
                    .iter()
                    .map(|(name, data)| (name.as_slice(), data.as_slice()))
                    .collect();
                let dup = sector_idml(&entries, *sector, shift);
                let err = dump_from_idml(&dup, "witness.idml")
                    .expect_err("the duplicate refuses, every shape");
                assert!(
                    err.contains("two parts named Stories/Story_u9.xml"),
                    "{label} shift={shift}: {err}"
                );
                assert!(
                    err.contains("may be damaged"),
                    "{label} shift={shift}: {err}"
                );

                let mut parts = witness_parts(std::slice::from_ref(&story_a));
                parts.push((tail_name.clone(), tail_data.clone()));
                let entries: Vec<(&[u8], &[u8])> = parts
                    .iter()
                    .map(|(name, data)| (name.as_slice(), data.as_slice()))
                    .collect();
                let single = sector_idml(&entries, *sector, shift);
                let dump = dump_from_idml(&single, "witness.idml")
                    .expect("the single story loads, every shape");
                assert_eq!(dump.spans.len(), 1, "{label} shift={shift}");
                assert_eq!(
                    dump.spans[0].text, "First story words here",
                    "{label} shift={shift}"
                );
            }
        }
    }

    /// The round-35 gap craft (red team RT35-1),
    /// assembled byte by byte in the crafter's shape:
    /// all entries Stored, a 512-byte gap between the
    /// last entry's data and the real directory, a
    /// central-directory digital-signature record after
    /// the real directory when `sig` is set (counted in
    /// the declared directory size), a fake directory
    /// planted at the head of the gap when `fake` is
    /// set (each duplicated name's copies replaced by
    /// the first copy's record, local offsets re-based
    /// by the gap, the last record's comment padding
    /// the fake walk to exactly the declared directory
    /// size), the end structures in plain or zip64
    /// dress, and the whole archive shifted by 4096
    /// junk bytes.
    fn gap_idml(entries: &[(&[u8], &[u8])], fake: bool, sig: bool, zip64: bool) -> Vec<u8> {
        const GAP: u64 = 512;
        let mut body: Vec<u8> = Vec::new();
        let mut locals: Vec<u32> = Vec::new();
        for (name, data) in entries {
            locals.push(body.len() as u32);
            let crc = stored_crc(data);
            body.extend_from_slice(b"PK\x03\x04");
            body.extend_from_slice(&20u16.to_le_bytes());
            body.extend_from_slice(&0u16.to_le_bytes());
            body.extend_from_slice(&0u16.to_le_bytes());
            body.extend_from_slice(&0u16.to_le_bytes());
            body.extend_from_slice(&0u16.to_le_bytes());
            body.extend_from_slice(&crc.to_le_bytes());
            body.extend_from_slice(&(data.len() as u32).to_le_bytes());
            body.extend_from_slice(&(data.len() as u32).to_le_bytes());
            body.extend_from_slice(&(name.len() as u16).to_le_bytes());
            body.extend_from_slice(&0u16.to_le_bytes());
            body.extend_from_slice(name);
            body.extend_from_slice(data);
        }
        let cd_offset = body.len() as u64 + GAP;
        let central = |name: &[u8], data: &[u8], local_offset: u32, comment: u16| {
            let crc = stored_crc(data);
            let mut rec: Vec<u8> = Vec::new();
            rec.extend_from_slice(b"PK\x01\x02");
            rec.extend_from_slice(&45u16.to_le_bytes());
            rec.extend_from_slice(&20u16.to_le_bytes());
            rec.extend_from_slice(&0u16.to_le_bytes());
            rec.extend_from_slice(&0u16.to_le_bytes());
            rec.extend_from_slice(&0u16.to_le_bytes());
            rec.extend_from_slice(&0u16.to_le_bytes());
            rec.extend_from_slice(&crc.to_le_bytes());
            rec.extend_from_slice(&(data.len() as u32).to_le_bytes());
            rec.extend_from_slice(&(data.len() as u32).to_le_bytes());
            rec.extend_from_slice(&(name.len() as u16).to_le_bytes());
            rec.extend_from_slice(&0u16.to_le_bytes());
            rec.extend_from_slice(&comment.to_le_bytes());
            rec.extend_from_slice(&0u16.to_le_bytes());
            rec.extend_from_slice(&0u16.to_le_bytes());
            rec.extend_from_slice(&0u32.to_le_bytes());
            rec.extend_from_slice(&local_offset.to_le_bytes());
            rec.extend_from_slice(name);
            rec.extend_from_slice(&vec![b'C'; usize::from(comment)]);
            rec
        };
        let mut centrals: Vec<u8> = Vec::new();
        for (i, (name, data)) in entries.iter().enumerate() {
            centrals.extend_from_slice(&central(name, data, locals[i], 0));
        }
        let sig_len: u64 = if sig { 22 } else { 0 };
        let cd_size = centrals.len() as u64 + sig_len;
        let mut fake_recs: Vec<u8> = Vec::new();
        for (i, (name, _)) in entries.iter().enumerate() {
            let first = entries
                .iter()
                .position(|(other, _)| other == name)
                .expect("the entry's own first copy stands");
            let comment = if i + 1 == entries.len() {
                u16::try_from(cd_size - fake_recs.len() as u64 - 46 - name.len() as u64)
                    .expect("the fake walk pads to the declared size")
            } else {
                0
            };
            fake_recs.extend_from_slice(&central(
                name,
                entries[first].1,
                locals[first] + GAP as u32,
                comment,
            ));
        }
        assert_eq!(fake_recs.len() as u64, cd_size);
        assert!(fake_recs.len() as u64 <= GAP);
        let mut gap = vec![b'G'; GAP as usize];
        if fake {
            gap[..fake_recs.len()].copy_from_slice(&fake_recs);
        }
        let mut out: Vec<u8> = Vec::new();
        out.extend_from_slice(&body);
        out.extend_from_slice(&gap);
        out.extend_from_slice(&centrals);
        if sig {
            out.extend_from_slice(b"PK\x05\x05");
            out.extend_from_slice(&16u16.to_le_bytes());
            out.extend_from_slice(b"SSSSSSSSSSSSSSSS");
        }
        if zip64 {
            let record_at = out.len() as u64;
            out.extend_from_slice(b"PK\x06\x06");
            out.extend_from_slice(&44u64.to_le_bytes());
            out.extend_from_slice(&45u16.to_le_bytes());
            out.extend_from_slice(&45u16.to_le_bytes());
            out.extend_from_slice(&0u32.to_le_bytes());
            out.extend_from_slice(&0u32.to_le_bytes());
            out.extend_from_slice(&(entries.len() as u64).to_le_bytes());
            out.extend_from_slice(&(entries.len() as u64).to_le_bytes());
            out.extend_from_slice(&cd_size.to_le_bytes());
            out.extend_from_slice(&cd_offset.to_le_bytes());
            out.extend_from_slice(b"PK\x06\x07");
            out.extend_from_slice(&0u32.to_le_bytes());
            out.extend_from_slice(&record_at.to_le_bytes());
            out.extend_from_slice(&1u32.to_le_bytes());
            out.extend_from_slice(b"PK\x05\x06");
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&u16::MAX.to_le_bytes());
            out.extend_from_slice(&u16::MAX.to_le_bytes());
            out.extend_from_slice(&u32::MAX.to_le_bytes());
            out.extend_from_slice(&u32::MAX.to_le_bytes());
        } else {
            out.extend_from_slice(b"PK\x05\x06");
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
            out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
            out.extend_from_slice(&(cd_size as u32).to_le_bytes());
            out.extend_from_slice(&(cd_offset as u32).to_le_bytes());
        }
        out.extend_from_slice(&0u16.to_le_bytes());
        shifted(&out, &vec![b'J'; 4096])
    }

    #[test]
    fn r35d_a_fake_directory_in_the_gap_does_not_steer_the_package() {
        // Round 35, F4 (red team RT35-1): on the gap
        // craft the signature record broke the
        // arithmetic derivation and the plain hunt
        // believed a fake directory planted in the gap
        // before the real one — its records replaced
        // the disagreeing duplicate's second copy with
        // the first copy's record, so the package
        // OPENED and served the first story with no
        // refusal. The real directory is judged now:
        // the duplicate refuses in the standing words,
        // in the craft and in all three controls —
        // gap-clean (gap and signature record, no
        // fake), no-signature (fake planted, the
        // arithmetic walk unbroken), and the zip64
        // twin (its hunt already carried the offset
        // anchor).
        let story_a = one_paragraph_story("First story words here");
        let story_b = one_paragraph_story("Other story words here");
        let shapes: [(&str, bool, bool, bool); 4] = [
            ("gapfake", true, true, false),
            ("gapclean", false, true, false),
            ("gapnosig", true, false, false),
            ("zip64gapfake", true, true, true),
        ];
        for (label, fake, sig, zip64) in shapes {
            let mut parts = witness_parts(&[story_a.clone(), story_b.clone()]);
            parts.push((b"filler.bin".to_vec(), vec![b'A'; 6000]));
            let entries: Vec<(&[u8], &[u8])> = parts
                .iter()
                .map(|(name, data)| (name.as_slice(), data.as_slice()))
                .collect();
            let bytes = gap_idml(&entries, fake, sig, zip64);
            let err = dump_from_idml(&bytes, "witness.idml")
                .expect_err("the disagreeing duplicate refuses, every shape");
            assert!(
                err.contains("two parts named Stories/Story_u9.xml"),
                "{label}: {err}"
            );
            assert!(err.contains("may be damaged"), "{label}: {err}");
        }
    }

    /// One plain central record for the round-36
    /// crafts: Stored, no comment, the given extra
    /// field riding after the name.
    fn plain_central(name: &[u8], data: &[u8], local_offset: u32, extra: &[u8]) -> Vec<u8> {
        let crc = stored_crc(data);
        let mut rec: Vec<u8> = Vec::new();
        rec.extend_from_slice(b"PK\x01\x02");
        rec.extend_from_slice(&45u16.to_le_bytes());
        rec.extend_from_slice(&20u16.to_le_bytes());
        rec.extend_from_slice(&0u16.to_le_bytes());
        rec.extend_from_slice(&0u16.to_le_bytes());
        rec.extend_from_slice(&0u16.to_le_bytes());
        rec.extend_from_slice(&0u16.to_le_bytes());
        rec.extend_from_slice(&crc.to_le_bytes());
        rec.extend_from_slice(&(data.len() as u32).to_le_bytes());
        rec.extend_from_slice(&(data.len() as u32).to_le_bytes());
        rec.extend_from_slice(&(name.len() as u16).to_le_bytes());
        rec.extend_from_slice(&(extra.len() as u16).to_le_bytes());
        rec.extend_from_slice(&0u16.to_le_bytes());
        rec.extend_from_slice(&0u16.to_le_bytes());
        rec.extend_from_slice(&0u16.to_le_bytes());
        rec.extend_from_slice(&0u32.to_le_bytes());
        rec.extend_from_slice(&local_offset.to_le_bytes());
        rec.extend_from_slice(name);
        rec.extend_from_slice(extra);
        rec
    }

    /// One plain end record with the given disk field
    /// and comment, the tail of the round-36 crafts.
    fn plain_eocd(total: u16, cd_size: u32, cd_offset: u32, disk: u16, comment: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"PK\x05\x06");
        out.extend_from_slice(&disk.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&total.to_le_bytes());
        out.extend_from_slice(&total.to_le_bytes());
        out.extend_from_slice(&cd_size.to_le_bytes());
        out.extend_from_slice(&cd_offset.to_le_bytes());
        out.extend_from_slice(&(comment.len() as u16).to_le_bytes());
        out.extend_from_slice(comment);
        out
    }

    /// The round-36 second-directory crafts (review
    /// R36-6 + red team RT36-5 + review R36-7's
    /// compound), assembled byte by byte over Stored
    /// entries: the real directory and end record,
    /// then per `shape` — "plain" nothing more;
    /// "appended-clean" / "appended-dirty" a second
    /// directory naming each part once (the first
    /// copies' records) with its own end record (disk
    /// field 0 or 1); "comment" that same second
    /// directory planted inside the real end record's
    /// declared comment; "steered" an appended
    /// directory whose extra record duplicates the
    /// first repeated name with a zip64 extra
    /// resolving its local offset to u64::MAX - 10.
    fn second_directory_idml(entries: &[(&[u8], &[u8])], shape: &str) -> Vec<u8> {
        let mut out: Vec<u8> = Vec::new();
        let mut locals: Vec<u32> = Vec::new();
        for (name, data) in entries {
            locals.push(out.len() as u32);
            let crc = stored_crc(data);
            out.extend_from_slice(b"PK\x03\x04");
            out.extend_from_slice(&20u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&crc.to_le_bytes());
            out.extend_from_slice(&(data.len() as u32).to_le_bytes());
            out.extend_from_slice(&(data.len() as u32).to_le_bytes());
            out.extend_from_slice(&(name.len() as u16).to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(name);
            out.extend_from_slice(data);
        }
        let cd_offset = out.len() as u32;
        let mut centrals: Vec<u8> = Vec::new();
        for (i, (name, data)) in entries.iter().enumerate() {
            centrals.extend_from_slice(&plain_central(name, data, locals[i], &[]));
        }
        // The second directory's records: the first
        // copy's record for every name, in archive
        // order.
        let mut fake_recs: Vec<u8> = Vec::new();
        let mut fake_total: u16 = 0;
        for (i, (name, data)) in entries.iter().enumerate() {
            if entries[..i].iter().any(|(other, _)| other == name) {
                continue;
            }
            fake_recs.extend_from_slice(&plain_central(name, data, locals[i], &[]));
            fake_total += 1;
        }
        out.extend_from_slice(&centrals);
        match shape {
            "plain" => {
                out.extend_from_slice(&plain_eocd(
                    entries.len() as u16,
                    centrals.len() as u32,
                    cd_offset,
                    0,
                    &[],
                ));
            }
            "comment" => {
                let planted_offset = cd_offset + centrals.len() as u32 + 22;
                let mut comment = fake_recs.clone();
                comment.extend_from_slice(&plain_eocd(
                    fake_total,
                    fake_recs.len() as u32,
                    planted_offset,
                    0,
                    &[],
                ));
                out.extend_from_slice(&plain_eocd(
                    entries.len() as u16,
                    centrals.len() as u32,
                    cd_offset,
                    0,
                    &comment,
                ));
            }
            _ => {
                let disk = if shape == "appended-dirty" { 1 } else { 0 };
                out.extend_from_slice(&plain_eocd(
                    entries.len() as u16,
                    centrals.len() as u32,
                    cd_offset,
                    0,
                    &[],
                ));
                let fake_offset = out.len() as u32;
                let mut appended = fake_recs.clone();
                let mut appended_total = fake_total;
                if shape == "steered" {
                    // The zpain3 record: the first
                    // repeated name again, its local
                    // field the zip64 sentinel and its
                    // zip64 extra resolving the offset
                    // to u64::MAX - 10.
                    let dup_at = entries
                        .iter()
                        .position(|(name, _)| entries.iter().filter(|(n, _)| n == name).count() > 1)
                        .expect("a repeated name stands");
                    let (name, _) = entries[dup_at];
                    let second = entries
                        .iter()
                        .enumerate()
                        .filter(|(i, (n, _))| *n == name && *i != dup_at)
                        .map(|(_, (_, data))| *data)
                        .next()
                        .expect("the repeated name's second copy stands");
                    let mut extra: Vec<u8> = Vec::new();
                    extra.extend_from_slice(&1u16.to_le_bytes());
                    extra.extend_from_slice(&8u16.to_le_bytes());
                    extra.extend_from_slice(&(u64::MAX - 10).to_le_bytes());
                    appended.extend_from_slice(&plain_central(name, second, u32::MAX, &extra));
                    appended_total += 1;
                }
                out.extend_from_slice(&appended);
                out.extend_from_slice(&plain_eocd(
                    appended_total,
                    appended.len() as u32,
                    fake_offset,
                    disk,
                    &[],
                ));
            }
        }
        out
    }

    /// The round-37 planted-directory crafts (red
    /// team RT37-2), assembled byte by byte over
    /// Stored entries in the round-36 crafter's shape:
    /// the real directory, and per `shape` — "plain"
    /// nothing more; "z1" a second directory naming
    /// each part once (the first copies' records)
    /// planted inside the real archive's zip64
    /// extensible data sector; "z2" that directory
    /// planted inside the last entry's payload (the
    /// filler, whose payload the caller makes large
    /// enough to carry it); "z3" / "z3b" the real
    /// archive standing behind a complete tiny prefix
    /// archive whose end record declares a comment of
    /// 64 bytes / the whole real archive, reaching
    /// into it — no comment bytes follow the prefix's
    /// end record; the real archive stands there.
    fn planted_directory_idml(entries: &[(&[u8], &[u8])], shape: &str) -> Vec<u8> {
        let mut body: Vec<u8> = Vec::new();
        let mut locals: Vec<u32> = Vec::new();
        for (name, data) in entries {
            locals.push(body.len() as u32);
            let crc = stored_crc(data);
            body.extend_from_slice(b"PK\x03\x04");
            body.extend_from_slice(&20u16.to_le_bytes());
            body.extend_from_slice(&0u16.to_le_bytes());
            body.extend_from_slice(&0u16.to_le_bytes());
            body.extend_from_slice(&0u16.to_le_bytes());
            body.extend_from_slice(&0u16.to_le_bytes());
            body.extend_from_slice(&crc.to_le_bytes());
            body.extend_from_slice(&(data.len() as u32).to_le_bytes());
            body.extend_from_slice(&(data.len() as u32).to_le_bytes());
            body.extend_from_slice(&(name.len() as u16).to_le_bytes());
            body.extend_from_slice(&0u16.to_le_bytes());
            body.extend_from_slice(name);
            body.extend_from_slice(data);
        }
        let cd_offset = body.len() as u32;
        let mut centrals: Vec<u8> = Vec::new();
        for (i, (name, data)) in entries.iter().enumerate() {
            centrals.extend_from_slice(&plain_central(name, data, locals[i], &[]));
        }
        let mut fake_recs: Vec<u8> = Vec::new();
        let mut fake_total: u16 = 0;
        for (i, (name, data)) in entries.iter().enumerate() {
            if entries[..i].iter().any(|(other, _)| other == name) {
                continue;
            }
            fake_recs.extend_from_slice(&plain_central(name, data, locals[i], &[]));
            fake_total += 1;
        }
        let fake_eocd =
            |cd_pos: usize| plain_eocd(fake_total, fake_recs.len() as u32, cd_pos as u32, 0, &[]);
        match shape {
            "z1" => {
                let mut out = body;
                out.extend_from_slice(&centrals);
                let record_at = out.len();
                let sector_len = 2048usize;
                let fake_cd_pos = record_at + 56 + 16;
                let mut sector = vec![b'S'; sector_len];
                sector[16..16 + fake_recs.len()].copy_from_slice(&fake_recs);
                let planted_eocd = fake_eocd(fake_cd_pos);
                let eocd_at = 16 + fake_recs.len();
                sector[eocd_at..eocd_at + 22].copy_from_slice(&planted_eocd);
                out.extend_from_slice(b"PK\x06\x06");
                out.extend_from_slice(&((44 + sector_len) as u64).to_le_bytes());
                out.extend_from_slice(&45u16.to_le_bytes());
                out.extend_from_slice(&45u16.to_le_bytes());
                out.extend_from_slice(&0u32.to_le_bytes());
                out.extend_from_slice(&0u32.to_le_bytes());
                out.extend_from_slice(&(entries.len() as u64).to_le_bytes());
                out.extend_from_slice(&(entries.len() as u64).to_le_bytes());
                out.extend_from_slice(&(centrals.len() as u64).to_le_bytes());
                out.extend_from_slice(&(cd_offset as u64).to_le_bytes());
                out.extend_from_slice(&sector);
                out.extend_from_slice(b"PK\x06\x07");
                out.extend_from_slice(&0u32.to_le_bytes());
                out.extend_from_slice(&(record_at as u64).to_le_bytes());
                out.extend_from_slice(&1u32.to_le_bytes());
                out.extend_from_slice(b"PK\x05\x06");
                out.extend_from_slice(&0u16.to_le_bytes());
                out.extend_from_slice(&0u16.to_le_bytes());
                out.extend_from_slice(&u16::MAX.to_le_bytes());
                out.extend_from_slice(&u16::MAX.to_le_bytes());
                out.extend_from_slice(&u32::MAX.to_le_bytes());
                out.extend_from_slice(&u32::MAX.to_le_bytes());
                out.extend_from_slice(&0u16.to_le_bytes());
                out
            }
            "z2" => {
                let last = entries.len() - 1;
                let payload_at = locals[last] as usize + 30 + entries[last].0.len();
                let fake_cd_pos = payload_at + 100;
                body[fake_cd_pos..fake_cd_pos + fake_recs.len()].copy_from_slice(&fake_recs);
                let planted_eocd = fake_eocd(fake_cd_pos);
                let eocd_at = fake_cd_pos + fake_recs.len();
                body[eocd_at..eocd_at + 22].copy_from_slice(&planted_eocd);
                // The carrier entry stays self-consistent:
                // its local and central checksums are the
                // checksum of its final, planted payload.
                let carrier = body[payload_at..payload_at + entries[last].1.len()].to_vec();
                let carrier_crc = stored_crc(&carrier);
                let local_crc_at = locals[last] as usize + 14;
                body[local_crc_at..local_crc_at + 4].copy_from_slice(&carrier_crc.to_le_bytes());
                let central_start = centrals.len() - 46 - entries[last].0.len();
                centrals[central_start + 16..central_start + 20]
                    .copy_from_slice(&carrier_crc.to_le_bytes());
                let mut out = body;
                out.extend_from_slice(&centrals);
                out.extend_from_slice(&plain_eocd(
                    entries.len() as u16,
                    centrals.len() as u32,
                    cd_offset,
                    0,
                    &[],
                ));
                out
            }
            "z3" | "z3b" => {
                let mut real = body;
                real.extend_from_slice(&centrals);
                real.extend_from_slice(&plain_eocd(
                    entries.len() as u16,
                    centrals.len() as u32,
                    cd_offset,
                    0,
                    &[],
                ));
                let tiny_name: &[u8] = b"tiny.txt";
                let tiny_data: &[u8] = b"tiny";
                let mut tiny: Vec<u8> = Vec::new();
                let crc = stored_crc(tiny_data);
                tiny.extend_from_slice(b"PK\x03\x04");
                tiny.extend_from_slice(&20u16.to_le_bytes());
                tiny.extend_from_slice(&0u16.to_le_bytes());
                tiny.extend_from_slice(&0u16.to_le_bytes());
                tiny.extend_from_slice(&0u16.to_le_bytes());
                tiny.extend_from_slice(&0u16.to_le_bytes());
                tiny.extend_from_slice(&crc.to_le_bytes());
                tiny.extend_from_slice(&(tiny_data.len() as u32).to_le_bytes());
                tiny.extend_from_slice(&(tiny_data.len() as u32).to_le_bytes());
                tiny.extend_from_slice(&(tiny_name.len() as u16).to_le_bytes());
                tiny.extend_from_slice(&0u16.to_le_bytes());
                tiny.extend_from_slice(tiny_name);
                tiny.extend_from_slice(tiny_data);
                let tiny_cd_offset = tiny.len() as u32;
                let tiny_central = plain_central(tiny_name, tiny_data, 0, &[]);
                tiny.extend_from_slice(&tiny_central);
                let comment_len: u16 = if shape == "z3b" {
                    u16::try_from(real.len().min(usize::from(u16::MAX)))
                        .expect("the declared comment fits its field")
                } else {
                    64
                };
                tiny.extend_from_slice(b"PK\x05\x06");
                tiny.extend_from_slice(&0u16.to_le_bytes());
                tiny.extend_from_slice(&0u16.to_le_bytes());
                tiny.extend_from_slice(&1u16.to_le_bytes());
                tiny.extend_from_slice(&1u16.to_le_bytes());
                tiny.extend_from_slice(&(tiny_central.len() as u32).to_le_bytes());
                tiny.extend_from_slice(&tiny_cd_offset.to_le_bytes());
                tiny.extend_from_slice(&comment_len.to_le_bytes());
                tiny.extend_from_slice(&real);
                tiny
            }
            "n1" => {
                // Round 38, F1: the fake directory
                // planted inside the last entry's
                // local-header extra field — the filler
                // local header gains a 4,000-byte extra
                // field carrying the fake records and
                // their end record; the filler's own
                // local offset is unmoved, and the real
                // directory stands 4,000 bytes later
                // than the plain shape's.
                let last = entries.len() - 1;
                let extra_start = locals[last] as usize + 30 + entries[last].0.len();
                let fake_cd_pos = extra_start + 100;
                let mut extra = vec![b'E'; 4000];
                extra[100..100 + fake_recs.len()].copy_from_slice(&fake_recs);
                let planted_eocd = fake_eocd(fake_cd_pos);
                extra[100 + fake_recs.len()..100 + fake_recs.len() + 22]
                    .copy_from_slice(&planted_eocd);
                let mut out = body[..extra_start].to_vec();
                let extra_len_at = locals[last] as usize + 28;
                out[extra_len_at..extra_len_at + 2].copy_from_slice(&4000u16.to_le_bytes());
                out.extend_from_slice(&extra);
                out.extend_from_slice(&body[extra_start..]);
                out.extend_from_slice(&centrals);
                out.extend_from_slice(&plain_eocd(
                    entries.len() as u16,
                    centrals.len() as u32,
                    cd_offset + 4000,
                    0,
                    &[],
                ));
                out
            }
            "n2b" => {
                // Round 38, F1: the fake directory
                // planted contiguously inside the last
                // record's central extra field.
                let last = entries.len() - 1;
                let name_len = entries[last].0.len();
                let record_start = centrals.len() - 46 - name_len;
                let fake_cd_pos = cd_offset as usize + record_start + 46 + name_len + 4 + 50;
                let mut blob = vec![b'Z'; fake_recs.len() + 400];
                blob[50..50 + fake_recs.len()].copy_from_slice(&fake_recs);
                let planted_eocd = fake_eocd(fake_cd_pos);
                blob[50 + fake_recs.len()..50 + fake_recs.len() + 22]
                    .copy_from_slice(&planted_eocd);
                let mut extra: Vec<u8> = Vec::new();
                extra.extend_from_slice(&0x9999u16.to_le_bytes());
                extra.extend_from_slice(&(blob.len() as u16).to_le_bytes());
                extra.extend_from_slice(&blob);
                let mut new_centrals = centrals.clone();
                let extra_len_at = record_start + 30;
                new_centrals[extra_len_at..extra_len_at + 2]
                    .copy_from_slice(&(extra.len() as u16).to_le_bytes());
                new_centrals.extend_from_slice(&extra);
                let mut out = body;
                out.extend_from_slice(&new_centrals);
                out.extend_from_slice(&plain_eocd(
                    entries.len() as u16,
                    new_centrals.len() as u32,
                    cd_offset,
                    0,
                    &[],
                ));
                out
            }
            "n3b" => {
                // Round 38, F1: the fake directory
                // planted contiguously inside the last
                // record's central comment field.
                let last = entries.len() - 1;
                let name_len = entries[last].0.len();
                let record_start = centrals.len() - 46 - name_len;
                let fake_cd_pos = cd_offset as usize + record_start + 46 + name_len + 50;
                let mut blob = vec![b'Z'; fake_recs.len() + 400];
                blob[50..50 + fake_recs.len()].copy_from_slice(&fake_recs);
                let planted_eocd = fake_eocd(fake_cd_pos);
                blob[50 + fake_recs.len()..50 + fake_recs.len() + 22]
                    .copy_from_slice(&planted_eocd);
                let mut new_centrals = centrals.clone();
                let comment_len_at = record_start + 32;
                new_centrals[comment_len_at..comment_len_at + 2]
                    .copy_from_slice(&(blob.len() as u16).to_le_bytes());
                new_centrals.extend_from_slice(&blob);
                let mut out = body;
                out.extend_from_slice(&new_centrals);
                out.extend_from_slice(&plain_eocd(
                    entries.len() as u16,
                    new_centrals.len() as u32,
                    cd_offset,
                    0,
                    &[],
                ));
                out
            }
            "n4" => {
                // Round 38, F1: the fake directory
                // planted inside a digital-signature
                // record's data, the record standing
                // between the real directory and its
                // end record; the declared directory
                // size counts the central records only,
                // the standing allowance's shape.
                let fake_cd_pos = cd_offset as usize + centrals.len() + 6 + 100;
                let mut sig_data = vec![b'S'; 4000];
                sig_data[100..100 + fake_recs.len()].copy_from_slice(&fake_recs);
                let planted_eocd = fake_eocd(fake_cd_pos);
                sig_data[100 + fake_recs.len()..100 + fake_recs.len() + 22]
                    .copy_from_slice(&planted_eocd);
                let mut out = body;
                out.extend_from_slice(&centrals);
                out.extend_from_slice(b"PK\x05\x05");
                out.extend_from_slice(&(sig_data.len() as u16).to_le_bytes());
                out.extend_from_slice(&sig_data);
                out.extend_from_slice(&plain_eocd(
                    entries.len() as u16,
                    centrals.len() as u32,
                    cd_offset,
                    0,
                    &[],
                ));
                out
            }
            _ => {
                let mut out = body;
                out.extend_from_slice(&centrals);
                out.extend_from_slice(&plain_eocd(
                    entries.len() as u16,
                    centrals.len() as u32,
                    cd_offset,
                    0,
                    &[],
                ));
                out
            }
        }
    }

    #[test]
    fn r37a_a_directory_planted_in_a_sector_a_payload_or_behind_a_prefix_comment_never_steers() {
        // Round 37, F8 (red team RT37-2): a second
        // self-consistent directory planted in a zip64
        // extensible data sector, in a stored entry's
        // payload, or behind a prefix archive whose
        // declared comment reaches into the real
        // archive steered the package the way round
        // 36's vectors did — the scan judged the planted
        // directory (each part once) while the reader
        // served the real one, and the second copy of
        // the disagreeing duplicate story was served in
        // silence. The real directory is judged in
        // every shape now: the duplicate refuses in the
        // standing words, exactly as the plain control
        // refuses, and the same shapes carrying one
        // story load it — the z1 single side is also
        // this door's sector-archive service control.
        let story_a = one_paragraph_story("First story words here");
        let story_b = one_paragraph_story("Other story words here");
        let filler = vec![b'A'; 2000];
        for shape in ["plain", "z1", "z2", "z3", "z3b"] {
            let mut parts = witness_parts(&[story_a.clone(), story_b.clone()]);
            parts.push((b"filler.bin".to_vec(), filler.clone()));
            let entries: Vec<(&[u8], &[u8])> = parts
                .iter()
                .map(|(name, data)| (name.as_slice(), data.as_slice()))
                .collect();
            let bytes = planted_directory_idml(&entries, shape);
            let err = dump_from_idml(&bytes, "witness.idml")
                .expect_err("the disagreeing duplicate refuses, every shape");
            assert!(
                err.contains("two parts named Stories/Story_u9.xml"),
                "{shape}: {err}"
            );
            assert!(err.contains("may be damaged"), "{shape}: {err}");

            let mut parts = witness_parts(std::slice::from_ref(&story_a));
            parts.push((b"filler.bin".to_vec(), filler.clone()));
            let entries: Vec<(&[u8], &[u8])> = parts
                .iter()
                .map(|(name, data)| (name.as_slice(), data.as_slice()))
                .collect();
            let bytes = planted_directory_idml(&entries, shape);
            let dump = dump_from_idml(&bytes, "witness.idml")
                .expect("the single story loads, every shape");
            assert_eq!(dump.spans.len(), 1, "{shape}");
            assert_eq!(dump.spans[0].text, "First story words here", "{shape}");
        }
    }

    #[test]
    fn r38a_a_directory_planted_in_a_local_extra_a_central_extra_a_central_comment_or_a_signature_record_never_steers(
    ) {
        // Round 38, F1 (red team RT38-1): a second
        // self-consistent directory planted in a local
        // header's extra field, in a central record's
        // extra or comment field, or in a
        // digital-signature record's data steered the
        // package the way round 37's vectors did — the
        // round-37 spans covered an entry's payload
        // only, so the scan judged the planted
        // directory (each part once) while the reader
        // served the real one, and the second copy of
        // the disagreeing duplicate story was served in
        // silence. Standing is judged against the whole
        // walked extent now: the real directory is
        // judged in every shape, the duplicate refuses
        // in the standing words exactly as the plain
        // control refuses, and the same shapes carrying
        // one story load it.
        let story_a = one_paragraph_story("First story words here");
        let story_b = one_paragraph_story("Other story words here");
        let filler = vec![b'A'; 2000];
        for shape in ["plain", "n1", "n2b", "n3b", "n4"] {
            let mut parts = witness_parts(&[story_a.clone(), story_b.clone()]);
            parts.push((b"filler.bin".to_vec(), filler.clone()));
            let entries: Vec<(&[u8], &[u8])> = parts
                .iter()
                .map(|(name, data)| (name.as_slice(), data.as_slice()))
                .collect();
            let bytes = planted_directory_idml(&entries, shape);
            let err = dump_from_idml(&bytes, "witness.idml")
                .expect_err("the disagreeing duplicate refuses, every shape");
            assert!(
                err.contains("two parts named Stories/Story_u9.xml"),
                "{shape}: {err}"
            );
            assert!(err.contains("may be damaged"), "{shape}: {err}");

            let mut parts = witness_parts(std::slice::from_ref(&story_a));
            parts.push((b"filler.bin".to_vec(), filler.clone()));
            let entries: Vec<(&[u8], &[u8])> = parts
                .iter()
                .map(|(name, data)| (name.as_slice(), data.as_slice()))
                .collect();
            let bytes = planted_directory_idml(&entries, shape);
            let dump = dump_from_idml(&bytes, "witness.idml")
                .expect("the single story loads, every shape");
            assert_eq!(dump.spans.len(), 1, "{shape}");
            assert_eq!(dump.spans[0].text, "First story words here", "{shape}");
        }
    }

    /// The round-39 crafts (red team RT39-1 / RT39-2,
    /// review R39-R2), assembled byte by byte over the
    /// caller's Stored entries in the planted-directory
    /// crafters' shape. Per `shape`:
    /// "gap" / "gap64" — a 5,000-byte unused gap
    /// between the last entry's payload and the real
    /// directory (plain / zip64 host) with a complete
    /// fake directory (the first copies' records plus
    /// its own end record) planted at the gap's head;
    /// "gapclean" — the gap with no plant, the control;
    /// "prefirst" — a 3,000-byte region before the
    /// first local header carrying the plant;
    /// "broken" / "broken64" — the last entry (the
    /// filler) gains a 4,000-byte local extra carrying
    /// the plant, and its central record declares a
    /// compressed size of 0xFFFFFFF0, so its payload
    /// span does not resolve (plain / zip64 host);
    /// "z64sig-clean" — a zip64 host whose 4,000-byte
    /// digital-signature record stands between the
    /// directory and the zip64 record, NOT counted in
    /// the declared directory size;
    /// "z64sig-plant" — the fake directory planted
    /// inside that signature record's data;
    /// "z64sig-counted" — the plant, with the
    /// signature record counted in the declared
    /// directory size;
    /// "z64sectorsig-clean" / "z64sectorsig-plant" —
    /// the uncounted-signature shapes on a sector host
    /// (a 2,048-byte extensible data sector, so the
    /// validating hunt is the only derivation).
    /// Zip64 hosts carry plain central records and a
    /// sentinel end record, as the sector crafter's do.
    fn r39a_archive_idml(entries: &[(&[u8], &[u8])], shape: &str) -> Vec<u8> {
        let zip64 = matches!(
            shape,
            "gap64"
                | "broken64"
                | "z64sig-clean"
                | "z64sig-plant"
                | "z64sig-counted"
                | "z64sectorsig-clean"
                | "z64sectorsig-plant"
        );
        let sector: usize = if shape.starts_with("z64sectorsig") {
            2048
        } else {
            0
        };
        let sig_shape = matches!(
            shape,
            "z64sig-clean"
                | "z64sig-plant"
                | "z64sig-counted"
                | "z64sectorsig-clean"
                | "z64sectorsig-plant"
        );
        let plant_in_sig = matches!(
            shape,
            "z64sig-plant" | "z64sig-counted" | "z64sectorsig-plant"
        );
        let prefix: usize = if shape == "prefirst" { 3000 } else { 0 };
        let gap: usize = if matches!(shape, "gap" | "gap64" | "gapclean") {
            5000
        } else {
            0
        };
        let broken = matches!(shape, "broken" | "broken64");
        // The pre-first-header region opens with the
        // archive's own signature bytes as filler, so
        // the file still presents as a zip at the
        // door's head check; the plant rides inside
        // the region either way.
        let mut body: Vec<u8> = Vec::new();
        if prefix > 0 {
            body.extend_from_slice(b"PK");
            body.resize(prefix, b'P');
        }
        let mut locals: Vec<u32> = Vec::new();
        let mut filler_extra_start = 0usize;
        for (i, (name, data)) in entries.iter().enumerate() {
            locals.push(body.len() as u32);
            let crc = stored_crc(data);
            let local_extra: usize = if broken && i + 1 == entries.len() {
                4000
            } else {
                0
            };
            body.extend_from_slice(b"PK\x03\x04");
            body.extend_from_slice(&20u16.to_le_bytes());
            body.extend_from_slice(&0u16.to_le_bytes());
            body.extend_from_slice(&0u16.to_le_bytes());
            body.extend_from_slice(&0u16.to_le_bytes());
            body.extend_from_slice(&0u16.to_le_bytes());
            body.extend_from_slice(&crc.to_le_bytes());
            body.extend_from_slice(&(data.len() as u32).to_le_bytes());
            body.extend_from_slice(&(data.len() as u32).to_le_bytes());
            body.extend_from_slice(&(name.len() as u16).to_le_bytes());
            body.extend_from_slice(&(local_extra as u16).to_le_bytes());
            body.extend_from_slice(name);
            if local_extra > 0 {
                filler_extra_start = body.len();
                body.extend_from_slice(&vec![b'E'; local_extra]);
            }
            body.extend_from_slice(data);
        }
        // The fake directory: the first copy's record
        // for every name, in archive order, plus its
        // own end record.
        let mut fake_recs: Vec<u8> = Vec::new();
        let mut fake_total: u16 = 0;
        for (i, (name, data)) in entries.iter().enumerate() {
            if entries[..i].iter().any(|(other, _)| other == name) {
                continue;
            }
            fake_recs.extend_from_slice(&plain_central(name, data, locals[i], &[]));
            fake_total += 1;
        }
        let fake_len = fake_recs.len();
        let plant = |host: &mut Vec<u8>, at: usize| {
            host[at..at + fake_len].copy_from_slice(&fake_recs);
            let eocd = plain_eocd(fake_total, fake_len as u32, at as u32, 0, &[]);
            host[at + fake_len..at + fake_len + 22].copy_from_slice(&eocd);
        };
        if shape == "prefirst" {
            plant(&mut body, 100);
        }
        if broken {
            plant(&mut body, filler_extra_start + 100);
        }
        if gap > 0 {
            let gap_start = body.len();
            body.extend_from_slice(&vec![b'G'; gap]);
            if shape != "gapclean" {
                plant(&mut body, gap_start + 100);
            }
        }
        let cd_offset = body.len() as u32;
        let mut centrals: Vec<u8> = Vec::new();
        for (i, (name, data)) in entries.iter().enumerate() {
            let mut rec = plain_central(name, data, locals[i], &[]);
            if broken && i + 1 == entries.len() {
                rec[20..24].copy_from_slice(&0xFFFF_FFF0u32.to_le_bytes());
            }
            centrals.extend_from_slice(&rec);
        }
        let mut out = body;
        out.extend_from_slice(&centrals);
        if sig_shape {
            let sig_start = out.len();
            let mut sig_data = vec![b'S'; 4000];
            if plant_in_sig {
                plant(&mut sig_data, 100);
                // The planted end record names its
                // directory's position in file
                // coordinates; the plant helper wrote
                // the signature-relative position, so
                // rebase its declared offset onto the
                // file.
                let eocd_at = 100 + fake_len;
                let declared =
                    u32::from_le_bytes(sig_data[eocd_at + 16..eocd_at + 20].try_into().unwrap());
                let rebased = declared + (sig_start + 6) as u32;
                sig_data[eocd_at + 16..eocd_at + 20].copy_from_slice(&rebased.to_le_bytes());
            }
            out.extend_from_slice(b"PK\x05\x05");
            out.extend_from_slice(&(sig_data.len() as u16).to_le_bytes());
            out.extend_from_slice(&sig_data);
        }
        if zip64 {
            let record_at = out.len() as u64;
            let cd_size = centrals.len() as u64 + if shape == "z64sig-counted" { 4006 } else { 0 };
            out.extend_from_slice(b"PK\x06\x06");
            out.extend_from_slice(&((44 + sector) as u64).to_le_bytes());
            out.extend_from_slice(&45u16.to_le_bytes());
            out.extend_from_slice(&45u16.to_le_bytes());
            out.extend_from_slice(&0u32.to_le_bytes());
            out.extend_from_slice(&0u32.to_le_bytes());
            out.extend_from_slice(&(entries.len() as u64).to_le_bytes());
            out.extend_from_slice(&(entries.len() as u64).to_le_bytes());
            out.extend_from_slice(&cd_size.to_le_bytes());
            out.extend_from_slice(&(cd_offset as u64).to_le_bytes());
            out.extend_from_slice(&vec![b'S'; sector]);
            out.extend_from_slice(b"PK\x06\x07");
            out.extend_from_slice(&0u32.to_le_bytes());
            out.extend_from_slice(&record_at.to_le_bytes());
            out.extend_from_slice(&1u32.to_le_bytes());
            out.extend_from_slice(b"PK\x05\x06");
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&u16::MAX.to_le_bytes());
            out.extend_from_slice(&u16::MAX.to_le_bytes());
            out.extend_from_slice(&u32::MAX.to_le_bytes());
            out.extend_from_slice(&u32::MAX.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
        } else {
            out.extend_from_slice(&plain_eocd(
                entries.len() as u16,
                centrals.len() as u32,
                cd_offset,
                0,
                &[],
            ));
        }
        out
    }

    #[test]
    fn r39a_a_directory_planted_in_the_gap_before_the_first_header_or_a_spanless_entrys_header_never_steers(
    ) {
        // Round 39, F1 (red team RT39-1 + review
        // R39-R2): a second self-consistent directory
        // planted in the unused gap before the central
        // directory, in the region before the first
        // local header, or in the local header of an
        // entry whose payload span does not resolve
        // steered the package the way round 38's
        // vectors did — those regions stood outside
        // the walked extent, so the scan judged the
        // planted directory (each part once) while the
        // reader served the real one, and the second
        // copy of the disagreeing duplicate story was
        // served in silence. The walked extent
        // accounts for every byte from the archive's
        // start to the end of its directory walk now,
        // and a local header stands in it from its own
        // fields whether or not its payload span
        // resolves: the real directory is judged in
        // every shape, on plain and zip64 hosts alike,
        // the duplicate refuses in the standing words
        // exactly as the gap-clean control refuses, and
        // the same shapes carrying one story load it.
        let story_a = one_paragraph_story("First story words here");
        let story_b = one_paragraph_story("Other story words here");
        let filler = vec![b'A'; 2000];
        for shape in ["gap", "gapclean", "gap64", "prefirst", "broken", "broken64"] {
            let mut parts = witness_parts(&[story_a.clone(), story_b.clone()]);
            parts.push((b"filler.bin".to_vec(), filler.clone()));
            let entries: Vec<(&[u8], &[u8])> = parts
                .iter()
                .map(|(name, data)| (name.as_slice(), data.as_slice()))
                .collect();
            let bytes = r39a_archive_idml(&entries, shape);
            let err = dump_from_idml(&bytes, "witness.idml")
                .expect_err("the disagreeing duplicate refuses, every shape");
            assert!(
                err.contains("two parts named Stories/Story_u9.xml"),
                "{shape}: {err}"
            );
            assert!(err.contains("may be damaged"), "{shape}: {err}");

            let mut parts = witness_parts(std::slice::from_ref(&story_a));
            parts.push((b"filler.bin".to_vec(), filler.clone()));
            let entries: Vec<(&[u8], &[u8])> = parts
                .iter()
                .map(|(name, data)| (name.as_slice(), data.as_slice()))
                .collect();
            let bytes = r39a_archive_idml(&entries, shape);
            let dump = dump_from_idml(&bytes, "witness.idml")
                .expect("the single story loads, every shape");
            assert_eq!(dump.spans.len(), 1, "{shape}");
            assert_eq!(dump.spans[0].text, "First story words here", "{shape}");
        }
    }

    #[test]
    fn r39a_a_zip64_archive_whose_signature_record_is_uncounted_derives_and_its_plant_never_steers()
    {
        // Round 39, F2 (red team RT39-2): a zip64
        // archive whose signature record is not
        // counted in the declared directory size
        // derived nothing at all — both derivations
        // demanded the directory end equal the zip64
        // record's position exactly — so every door
        // refused the clean craft, and a directory
        // planted inside that signature record was
        // the only standing candidate and steered the
        // package. The zip64 candidate derives under
        // the signature allowance now, on the
        // arithmetic path and the sector host's hunt
        // alike: the clean crafts derive and load,
        // the planted crafts judge the real directory
        // (the duplicate refuses in the standing
        // words), and the counted-signature variant
        // derives and refuses exactly as before. The
        // plain-host signature plant stands
        // subordinated as the r38a pin states (its
        // "n4" shape).
        let story_a = one_paragraph_story("First story words here");
        let story_b = one_paragraph_story("Other story words here");
        let filler = vec![b'A'; 2000];
        for shape in [
            "z64sig-clean",
            "z64sig-plant",
            "z64sig-counted",
            "z64sectorsig-clean",
            "z64sectorsig-plant",
        ] {
            let mut parts = witness_parts(&[story_a.clone(), story_b.clone()]);
            parts.push((b"filler.bin".to_vec(), filler.clone()));
            let entries: Vec<(&[u8], &[u8])> = parts
                .iter()
                .map(|(name, data)| (name.as_slice(), data.as_slice()))
                .collect();
            let bytes = r39a_archive_idml(&entries, shape);
            let err = dump_from_idml(&bytes, "witness.idml")
                .expect_err("the disagreeing duplicate refuses, every shape");
            assert!(
                err.contains("two parts named Stories/Story_u9.xml"),
                "{shape}: {err}"
            );
            assert!(err.contains("may be damaged"), "{shape}: {err}");

            let mut parts = witness_parts(std::slice::from_ref(&story_a));
            parts.push((b"filler.bin".to_vec(), filler.clone()));
            let entries: Vec<(&[u8], &[u8])> = parts
                .iter()
                .map(|(name, data)| (name.as_slice(), data.as_slice()))
                .collect();
            let bytes = r39a_archive_idml(&entries, shape);
            let dump = dump_from_idml(&bytes, "witness.idml")
                .expect("the single story loads, every shape");
            assert_eq!(dump.spans.len(), 1, "{shape}");
            assert_eq!(dump.spans[0].text, "First story words here", "{shape}");
        }
    }

    #[test]
    fn r36a_a_second_directory_never_steers_the_package() {
        // Round 36, F6 (review R36-6 + red team
        // RT36-5): a second self-consistent central
        // directory — appended past the real end
        // record, or planted inside the real end
        // record's own comment — judged the package
        // instead of the real directory, which records
        // the disagreeing duplicate story. The scan
        // judged the second directory (each part once,
        // no refusal) and the door proceeded into the
        // package or served whichever copy the second
        // directory named. The real directory is
        // judged in every shape now: the duplicate
        // refuses in the standing words, exactly as
        // the plain control refuses, and the same
        // shapes carrying one story load it.
        let story_a = one_paragraph_story("First story words here");
        let story_b = one_paragraph_story("Other story words here");
        for shape in ["plain", "appended-clean", "appended-dirty", "comment"] {
            let mut parts = witness_parts(&[story_a.clone(), story_b.clone()]);
            parts.push((b"filler.bin".to_vec(), vec![b'A'; 600]));
            let entries: Vec<(&[u8], &[u8])> = parts
                .iter()
                .map(|(name, data)| (name.as_slice(), data.as_slice()))
                .collect();
            let bytes = second_directory_idml(&entries, shape);
            let err = dump_from_idml(&bytes, "witness.idml")
                .expect_err("the disagreeing duplicate refuses, every shape");
            assert!(
                err.contains("two parts named Stories/Story_u9.xml"),
                "{shape}: {err}"
            );
            assert!(err.contains("may be damaged"), "{shape}: {err}");

            let mut parts = witness_parts(std::slice::from_ref(&story_a));
            parts.push((b"filler.bin".to_vec(), vec![b'A'; 600]));
            let entries: Vec<(&[u8], &[u8])> = parts
                .iter()
                .map(|(name, data)| (name.as_slice(), data.as_slice()))
                .collect();
            let bytes = second_directory_idml(&entries, shape);
            let dump = dump_from_idml(&bytes, "witness.idml")
                .expect("the single story loads, every shape");
            assert_eq!(dump.spans.len(), 1, "{shape}");
            assert_eq!(dump.spans[0].text, "First story words here", "{shape}");
        }
    }

    #[test]
    fn r36a_the_steered_appended_compound_refuses_instead_of_panicking() {
        // Round 36, F7 (review R36-7, witness zpain3):
        // the appended-directory compound whose
        // planted record resolves a local offset of
        // u64::MAX - 10 reached raw_payload's unchecked
        // `local + 30` and panicked (exit 101, "attempt
        // to add with overflow" at this door). With the
        // real directory judged (F6) the steered spot
        // never stands in the judged directory, and
        // raw_payload's arithmetic is checked besides —
        // the compound refuses in the standing
        // duplicate words, never a panic.
        let story_a = one_paragraph_story("First story words here");
        let story_b = one_paragraph_story("Other story words here");
        let mut parts = witness_parts(&[story_a, story_b]);
        parts.push((b"filler.bin".to_vec(), vec![b'A'; 600]));
        let entries: Vec<(&[u8], &[u8])> = parts
            .iter()
            .map(|(name, data)| (name.as_slice(), data.as_slice()))
            .collect();
        let bytes = second_directory_idml(&entries, "steered");
        let err = dump_from_idml(&bytes, "witness.idml").expect_err("the steered compound refuses");
        assert!(
            err.contains("two parts named Stories/Story_u9.xml"),
            "{err}"
        );
        assert!(err.contains("may be damaged"), "{err}");
    }

    #[test]
    fn r34b_a_package_the_reader_opens_with_no_derivable_directory_refuses() {
        // Round 34, F2, the fail-closed side: the
        // name-fake package above, with the real zip64
        // record's declared directory size corrupted
        // (ten bytes more than its directory holds), so
        // the record does not validate against its
        // locator and NO directory can be derived — yet
        // the reader opens the package anyway. Before
        // this batch the hunt was steered by the fake
        // record in the name to no directory at all and
        // the door served the second copy of the
        // disagreeing duplicate story in silence. The
        // door refuses in its reader-refusal words now
        // rather than scan an empty directory while its
        // reader serves the entries.
        let story_a = one_paragraph_story("First story words here");
        let story_b = one_paragraph_story("Other story words here");
        let fake_name = fake_record_name();
        let mut parts = witness_parts(&[story_a, story_b]);
        parts.push((fake_name, b"tail".to_vec()));
        let entries: Vec<(&[u8], &[u8])> = parts
            .iter()
            .map(|(name, data)| (name.as_slice(), data.as_slice()))
            .collect();
        let mut bytes = sector_idml(&entries, 8, 64);
        // The real zip64 record is the LAST PK\x06\x06
        // in the file (the fake stands earlier, in the
        // last record's name); its directory-size field
        // stands 40 bytes in.
        let record_at = bytes
            .windows(4)
            .rposition(|w| w == b"PK\x06\x06")
            .expect("the real zip64 record stands in the bytes");
        let cd_size = u64::from_le_bytes(bytes[record_at + 40..record_at + 48].try_into().unwrap());
        bytes[record_at + 40..record_at + 48].copy_from_slice(&(cd_size + 10).to_le_bytes());
        let err = dump_from_idml(&bytes, "underivable.idml")
            .expect_err("a reader-openable package with no derivable directory refuses");
        assert!(err.contains("not an IDML package"), "{err}");
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
    fn cdata_backoff_tracks_comments_and_sections_by_who_opened_first() {
        // Round 4, R4-11, at the scan itself: an opener
        // inside the other construct is that construct's
        // text, never an opener of its own.
        // A section opener mentioned inside a closed
        // comment: the text stands whole.
        let t = "<Story><!-- mentions <![CDATA[ here --><P>words</P></Story>";
        assert_eq!(cdata_backoff(t), t);
        // A comment opener and a comment's end inside a
        // closed section: section text; the text stands
        // whole.
        let t = "<Story><![CDATA[ <!-- not a comment --> ]]><P>words</P></Story>";
        assert_eq!(cdata_backoff(t), t);
        // A real section the text ends inside still
        // backs off to its opener, comment before it or
        // not.
        let t = "<Story><!-- ok --><P>words</P><![CDATA[ never closed";
        assert_eq!(cdata_backoff(t), "<Story><!-- ok --><P>words</P>");
        // A comment the text ends inside backs off the
        // same way, so the rescue's closing tags are not
        // swallowed as comment text.
        let t = "<Story><P>words</P><!-- never closed > more";
        assert_eq!(cdata_backoff(t), "<Story><P>words</P>");
        // A section opener inside an UNCLOSED comment
        // is still comment text: the back-off lands at
        // the comment, not at the marker inside it.
        let t = "<Story><P>words</P><!-- mentions <![CDATA[ and never ends";
        assert_eq!(cdata_backoff(t), "<Story><P>words</P>");
    }

    #[test]
    fn idml_a_cdata_opener_inside_a_comment_costs_nothing() {
        // Round 4, R4-11 / RT-4: the red team's shape.
        // A comment before the cut mentions a CDATA
        // opener as text and closes without any "]]>"
        // anywhere after it. The old backoff took the
        // mention for a real section that never closes
        // and dropped everything from the comment
        // onward, while the note still called it a tail
        // trim. The scan tracks comments now: the
        // comment owns its text, the paragraphs after
        // it are rescued, and the note keeps the
        // tail-trim words they earn.
        let range = |inner: &str| {
            format!(
                "<ParagraphStyleRange AppliedParagraphStyle=\"ParagraphStyle/Body\">\
                 <CharacterStyleRange FontStyle=\"Regular\">{inner}\
                 </CharacterStyleRange></ParagraphStyleRange>"
            )
        };
        let mut story = String::from("<?xml version=\"1.0\"?><Story>");
        story.push_str(&range("<Content>Words before the comment</Content>"));
        story.push_str("<!-- printer note: the <![CDATA[ marker stays text -->");
        story.push_str(&range("<Content>Words after the comment</Content>"));
        // One long trailing paragraph pushes the story
        // past the cap, so the cut lands well after the
        // comment.
        let mut filler = String::from("<Content>");
        filler.push_str(&"x".repeat(33 * 1024 * 1024));
        filler.push_str("</Content>");
        story.push_str(&range(&filler));
        story.push_str("</Story>");
        assert!(story.len() > 33 * 1024 * 1024, "the story passes the cap");
        let dump = dump_from_idml(&idml_with_story(&story), "trap.idml").expect("dump");
        assert!(
            dump.spans
                .iter()
                .any(|s| s.text.contains("Words before the comment")),
            "the paragraph before the comment is rescued"
        );
        assert!(
            dump.spans
                .iter()
                .any(|s| s.text.contains("Words after the comment")),
            "the paragraph after the comment is rescued"
        );
        assert_eq!(
            dump.loss_notes.len(),
            1,
            "the loss is counted once: {:?}",
            dump.loss_notes
        );
        assert!(
            dump.loss_notes[0].contains("Story_u9")
                && dump.loss_notes[0].contains("words past a point"),
            "a rescued part keeps the tail-trim words: {}",
            dump.loss_notes[0]
        );
    }

    #[test]
    fn idml_a_story_cut_inside_cdata_keeps_the_paragraphs_before_it() {
        // Round 3, RT3-C: the cut lands inside a CDATA
        // section. Nothing inside CDATA is markup, so
        // the closing tags the rescue appends used to be
        // swallowed as section text and the story parsed
        // to nothing, the finished paragraph before the
        // section included. The cut now backs off to
        // just before the section opens.
        let range = |inner: &str| {
            format!(
                "<ParagraphStyleRange AppliedParagraphStyle=\"ParagraphStyle/Body\">\
                 <CharacterStyleRange FontStyle=\"Regular\">{inner}\
                 </CharacterStyleRange></ParagraphStyleRange>"
            )
        };
        let mut story = String::from("<?xml version=\"1.0\"?><Story>");
        story.push_str(&range("<Content>Words before the flood</Content>"));
        let mut cdata = String::from("<Content><![CDATA[");
        cdata.push_str(&">".repeat(33 * 1024 * 1024));
        cdata.push_str("]]></Content>");
        story.push_str(&range(&cdata));
        story.push_str("</Story>");
        assert!(story.len() > 33 * 1024 * 1024, "the story passes the cap");
        let dump = dump_from_idml(&idml_with_story(&story), "cdata.idml").expect("dump");
        assert!(
            dump.spans
                .iter()
                .any(|s| s.text.contains("Words before the flood")),
            "the paragraph before the section is rescued"
        );
        assert!(
            !dump.spans.iter().any(|s| s.text.contains(">>>")),
            "no section text came over as words"
        );
        assert_eq!(
            dump.loss_notes.len(),
            1,
            "the loss is counted once: {:?}",
            dump.loss_notes
        );
        assert!(
            dump.loss_notes[0].contains("Story_u9")
                && dump.loss_notes[0].contains("words past a point"),
            "a rescued part keeps the tail-trim words: {}",
            dump.loss_notes[0]
        );
    }

    #[test]
    fn idml_a_story_cut_inside_its_first_cdata_is_counted_whole() {
        // The other half of RT3-C: the CDATA section
        // opens inside the story's FIRST paragraph, so
        // the back-off leaves that paragraph empty and
        // the rescue keeps nothing. The note must say
        // so, not claim a tail trim: the whole part was
        // left out, and it is counted.
        let mut story = String::from("<?xml version=\"1.0\"?><Story>");
        story.push_str(
            "<ParagraphStyleRange AppliedParagraphStyle=\"ParagraphStyle/Body\">\
             <CharacterStyleRange FontStyle=\"Regular\">\
             <Content><![CDATA[Words inside the section",
        );
        story.push_str(&">".repeat(33 * 1024 * 1024));
        story.push_str("]]></Content></CharacterStyleRange></ParagraphStyleRange></Story>");
        let dump = dump_from_idml(&idml_with_story(&story), "cdata-first.idml").expect("dump");
        assert!(
            dump.spans.is_empty(),
            "nothing of the story was kept: {} spans",
            dump.spans.len()
        );
        assert_eq!(
            dump.loss_notes.len(),
            1,
            "the loss is counted once: {:?}",
            dump.loss_notes
        );
        assert!(
            dump.loss_notes[0].contains("Story_u9")
                && dump.loss_notes[0].contains("none of its words could be kept"),
            "the note counts the whole part: {}",
            dump.loss_notes[0]
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

    /// The round-41 crafts (red team RT41-A +
    /// RT41-B), assembled byte by byte over the
    /// caller's Stored entries, in the fonts door's
    /// crafter shape: the four equal-set spellings in
    /// a 2,000-byte gap before the real directory
    /// ("eqorder" / "equncomp" / "eqextra" /
    /// "eqversion"), the "clean" control on the same
    /// layout, and the fake zip64 pair spellings in a
    /// 5,000-byte locator-to-end-record gap
    /// ("fakepair99" / "fakepair45").
    fn r41a_archive_idml(entries: &[(&[u8], &[u8])], shape: &str) -> Vec<u8> {
        let mut body: Vec<u8> = Vec::new();
        let mut locals: Vec<u32> = Vec::new();
        for (name, data) in entries {
            locals.push(body.len() as u32);
            let crc = stored_crc(data);
            body.extend_from_slice(b"PK\x03\x04");
            body.extend_from_slice(&20u16.to_le_bytes());
            body.extend_from_slice(&0u16.to_le_bytes());
            body.extend_from_slice(&0u16.to_le_bytes());
            body.extend_from_slice(&0u16.to_le_bytes());
            body.extend_from_slice(&0u16.to_le_bytes());
            body.extend_from_slice(&crc.to_le_bytes());
            body.extend_from_slice(&(data.len() as u32).to_le_bytes());
            body.extend_from_slice(&(data.len() as u32).to_le_bytes());
            body.extend_from_slice(&(name.len() as u16).to_le_bytes());
            body.extend_from_slice(&0u16.to_le_bytes());
            body.extend_from_slice(name);
            body.extend_from_slice(data);
        }
        let cd_offset = body.len() as u32;
        let mut centrals: Vec<u8> = Vec::new();
        for (i, (name, data)) in entries.iter().enumerate() {
            centrals.extend_from_slice(&plain_central(name, data, locals[i], &[]));
        }
        if shape.starts_with("fakepair") {
            let needed: u16 = if shape == "fakepair99" { 99 } else { 45 };
            let mut out = body;
            out.extend_from_slice(&centrals);
            let record_at = out.len() as u64;
            out.extend_from_slice(b"PK\x06\x06");
            out.extend_from_slice(&44u64.to_le_bytes());
            out.extend_from_slice(&45u16.to_le_bytes());
            out.extend_from_slice(&45u16.to_le_bytes());
            out.extend_from_slice(&0u32.to_le_bytes());
            out.extend_from_slice(&0u32.to_le_bytes());
            out.extend_from_slice(&(entries.len() as u64).to_le_bytes());
            out.extend_from_slice(&(entries.len() as u64).to_le_bytes());
            out.extend_from_slice(&(centrals.len() as u64).to_le_bytes());
            out.extend_from_slice(&(cd_offset as u64).to_le_bytes());
            out.extend_from_slice(b"PK\x06\x07");
            out.extend_from_slice(&0u32.to_le_bytes());
            out.extend_from_slice(&record_at.to_le_bytes());
            out.extend_from_slice(&1u32.to_le_bytes());
            let mut gap = vec![b'G'; 5000];
            let fake_at = (out.len() + 200) as u64;
            gap[200..204].copy_from_slice(b"PK\x06\x06");
            gap[204..212].copy_from_slice(&44u64.to_le_bytes());
            gap[212..214].copy_from_slice(&45u16.to_le_bytes());
            gap[214..216].copy_from_slice(&needed.to_le_bytes());
            gap[216..224].copy_from_slice(&0u64.to_le_bytes());
            gap[224..232].copy_from_slice(&(entries.len() as u64).to_le_bytes());
            gap[232..240].copy_from_slice(&(entries.len() as u64).to_le_bytes());
            gap[240..248].copy_from_slice(&(centrals.len() as u64).to_le_bytes());
            gap[248..256].copy_from_slice(&(cd_offset as u64).to_le_bytes());
            gap[256..260].copy_from_slice(b"PK\x06\x07");
            gap[260..264].copy_from_slice(&0u32.to_le_bytes());
            gap[264..272].copy_from_slice(&fake_at.to_le_bytes());
            gap[272..276].copy_from_slice(&1u32.to_le_bytes());
            out.extend_from_slice(&gap);
            out.extend_from_slice(b"PK\x05\x06");
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&u16::MAX.to_le_bytes());
            out.extend_from_slice(&u16::MAX.to_le_bytes());
            out.extend_from_slice(&u32::MAX.to_le_bytes());
            out.extend_from_slice(&u32::MAX.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            return out;
        }
        body.extend_from_slice(&vec![b'G'; 2000]);
        if shape != "clean" {
            let ut_extra: &[u8] = b"\x55\x54\x05\x00\x01\x00\x00\x00\x00";
            let order: Vec<usize> = if shape == "eqorder" {
                (0..entries.len()).rev().collect()
            } else {
                (0..entries.len()).collect()
            };
            let mut fake_recs: Vec<u8> = Vec::new();
            for i in order {
                let (name, data) = entries[i];
                let extra: &[u8] = if shape == "eqextra" { ut_extra } else { &[] };
                let mut rec = plain_central(name, data, locals[i], extra);
                if shape == "equncomp" {
                    rec[24..28].copy_from_slice(&1u32.to_le_bytes());
                }
                if shape == "eqversion" {
                    rec[4..6].copy_from_slice(&99u16.to_le_bytes());
                }
                fake_recs.extend_from_slice(&rec);
            }
            let plant_at = cd_offset as usize + 100;
            body[plant_at..plant_at + fake_recs.len()].copy_from_slice(&fake_recs);
            let eocd = plain_eocd(
                entries.len() as u16,
                fake_recs.len() as u32,
                plant_at as u32,
                0,
                &[],
            );
            let eocd_at = plant_at + fake_recs.len();
            body[eocd_at..eocd_at + 22].copy_from_slice(&eocd);
        }
        let real_cd = body.len() as u32;
        let mut out = body;
        out.extend_from_slice(&centrals);
        out.extend_from_slice(&plain_eocd(
            entries.len() as u16,
            centrals.len() as u32,
            real_cd,
            0,
            &[],
        ));
        out
    }

    #[test]
    fn r41a_an_equal_set_plant_or_a_fake_zip64_pair_never_steers_the_package() {
        // Round 41, F1 + F2 (red team RT41-A +
        // RT41-B): an equal-offset-set plant differing
        // from the real directory only in facts the
        // round-40 comparison never read was judged
        // as the package's directory, and a fake zip64
        // pair differing only in version-needed 99 was
        // spliced into the prefix-free view, where the
        // reader refused the good package outright.
        // The compared facts take the whole record in
        // walk order now, and the splice's pair choice
        // validates the version consistency the reader
        // judges: in every shape the real directory is
        // judged — the disagreeing duplicate refuses
        // in the standing words exactly as the clean
        // control refuses, and the same shapes
        // carrying one story load it.
        let story_a = one_paragraph_story("First story words here");
        let story_b = one_paragraph_story("Other story words here");
        let filler = vec![b'A'; 2000];
        for shape in [
            "clean",
            "eqorder",
            "equncomp",
            "eqextra",
            "eqversion",
            "fakepair99",
            "fakepair45",
        ] {
            let mut parts = witness_parts(&[story_a.clone(), story_b.clone()]);
            parts.push((b"filler.bin".to_vec(), filler.clone()));
            let entries: Vec<(&[u8], &[u8])> = parts
                .iter()
                .map(|(name, data)| (name.as_slice(), data.as_slice()))
                .collect();
            let bytes = r41a_archive_idml(&entries, shape);
            let err = dump_from_idml(&bytes, "witness.idml")
                .expect_err("the disagreeing duplicate refuses, every shape");
            assert!(
                err.contains("two parts named Stories/Story_u9.xml"),
                "{shape}: {err}"
            );

            let mut parts = witness_parts(std::slice::from_ref(&story_a));
            parts.push((b"filler.bin".to_vec(), filler.clone()));
            let entries: Vec<(&[u8], &[u8])> = parts
                .iter()
                .map(|(name, data)| (name.as_slice(), data.as_slice()))
                .collect();
            let bytes = r41a_archive_idml(&entries, shape);
            let dump = dump_from_idml(&bytes, "witness.idml")
                .expect("the single story loads, every shape");
            assert_eq!(dump.spans.len(), 1, "{shape}");
            assert_eq!(dump.spans[0].text, "First story words here", "{shape}");
        }
    }

    /// One Stored local header in the round-43
    /// crafts' shape, with real checksums, the flags
    /// stated outright, and the declared sizes zeroed
    /// where the descriptor form asks for it.
    fn r43d_local(name: &[u8], data: &[u8], flags: u16, zero_sizes: bool) -> Vec<u8> {
        let crc = stored_crc(data);
        let mut out: Vec<u8> = Vec::new();
        out.extend_from_slice(b"PK\x03\x04");
        out.extend_from_slice(&20u16.to_le_bytes());
        out.extend_from_slice(&flags.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&(if zero_sizes { 0 } else { crc }).to_le_bytes());
        let size = if zero_sizes { 0 } else { data.len() as u32 };
        out.extend_from_slice(&size.to_le_bytes());
        out.extend_from_slice(&size.to_le_bytes());
        out.extend_from_slice(&(name.len() as u16).to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(name);
        out.extend_from_slice(data);
        out
    }

    /// One central record in the round-43 crafts'
    /// shape, with the flags stated outright.
    fn r43d_central(name: &[u8], data: &[u8], local_offset: u32, flags: u16) -> Vec<u8> {
        let mut rec = plain_central(name, data, local_offset, &[]);
        if flags != 0 {
            rec[8..10].copy_from_slice(&flags.to_le_bytes());
        }
        rec
    }

    /// One end record in the round-43 crafts' shape:
    /// the disk number written to both disk fields, as
    /// the generator's disk craft writes it (1 and 1).
    fn r43d_eocd(total: u16, cd_size: u32, cd_offset: u32, disk: u16, comment: &[u8]) -> Vec<u8> {
        let mut out: Vec<u8> = Vec::new();
        out.extend_from_slice(b"PK\x05\x06");
        out.extend_from_slice(&disk.to_le_bytes());
        out.extend_from_slice(&disk.to_le_bytes());
        out.extend_from_slice(&total.to_le_bytes());
        out.extend_from_slice(&total.to_le_bytes());
        out.extend_from_slice(&cd_size.to_le_bytes());
        out.extend_from_slice(&cd_offset.to_le_bytes());
        out.extend_from_slice(&(comment.len() as u16).to_le_bytes());
        out.extend_from_slice(comment);
        out
    }

    /// The round-43 archive crafts (red team RT43-A1,
    /// generator `mk43.py`), assembled byte by byte
    /// over Stored entries: "clean" the real archive
    /// alone; "trailing" a complete second archive
    /// appended after the real end record whose
    /// records carry the real entries' names and the
    /// plant body's own local offsets, with the key
    /// entry's record describing the plant content;
    /// "comment" the plant directory in a gap before
    /// the real directory with a fake end record in
    /// the real end record's comment; "nameflip" the
    /// key entry's local name case-flipped; "crcbad"
    /// the key entry's local checksum falsified;
    /// "descriptor" the key entry in descriptor form;
    /// "disk1" both disk fields 1; "count1" the entry
    /// counts falsified to 1.
    fn r43d_archive(entries: &[(&[u8], &[u8])], key: usize, plant: &[u8], shape: &str) -> Vec<u8> {
        let mut body: Vec<u8> = Vec::new();
        let mut locals: Vec<u32> = Vec::new();
        for (i, (name, data)) in entries.iter().enumerate() {
            locals.push(body.len() as u32);
            if i == key && shape == "descriptor" {
                let mut local = r43d_local(name, data, 0x08, true);
                local.extend_from_slice(b"PK\x07\x08");
                local.extend_from_slice(&stored_crc(data).to_le_bytes());
                local.extend_from_slice(&(data.len() as u32).to_le_bytes());
                local.extend_from_slice(&(data.len() as u32).to_le_bytes());
                body.extend_from_slice(&local);
            } else if i == key && shape == "nameflip" {
                let flipped: Vec<u8> = name
                    .iter()
                    .map(|b| {
                        if b.is_ascii_lowercase() {
                            b.to_ascii_uppercase()
                        } else if b.is_ascii_uppercase() {
                            b.to_ascii_lowercase()
                        } else {
                            *b
                        }
                    })
                    .collect();
                body.extend_from_slice(&r43d_local(&flipped, data, 0, false));
            } else if i == key && shape == "crcbad" {
                let mut local = r43d_local(name, data, 0, false);
                let bad = stored_crc(data) ^ 0xFFFF_FFFF;
                local[14..18].copy_from_slice(&bad.to_le_bytes());
                body.extend_from_slice(&local);
            } else {
                body.extend_from_slice(&r43d_local(name, data, 0, false));
            }
        }
        let cd_start = body.len();
        let mut centrals: Vec<u8> = Vec::new();
        for (i, (name, data)) in entries.iter().enumerate() {
            let flags = if i == key && shape == "descriptor" {
                0x08
            } else {
                0
            };
            centrals.extend_from_slice(&r43d_central(name, data, locals[i], flags));
        }
        let total = entries.len() as u16;
        let mut pbody: Vec<u8> = Vec::new();
        let mut plocals: Vec<u32> = Vec::new();
        for (i, (name, data)) in entries.iter().enumerate() {
            plocals.push(pbody.len() as u32);
            let pdata = if i == key { plant } else { data };
            pbody.extend_from_slice(&r43d_local(name, pdata, 0, false));
        }
        let mut pcentrals: Vec<u8> = Vec::new();
        for (i, (name, data)) in entries.iter().enumerate() {
            let pdata = if i == key { plant } else { data };
            pcentrals.extend_from_slice(&r43d_central(name, pdata, plocals[i], 0));
        }
        let mut out = body;
        match shape {
            "trailing" => {
                out.extend_from_slice(&centrals);
                out.extend_from_slice(&r43d_eocd(
                    total,
                    centrals.len() as u32,
                    cd_start as u32,
                    0,
                    &[],
                ));
                let tcd = (out.len() + pbody.len()) as u32;
                out.extend_from_slice(&pbody);
                out.extend_from_slice(&pcentrals);
                out.extend_from_slice(&r43d_eocd(total, pcentrals.len() as u32, tcd, 0, &[]));
            }
            "comment" => {
                let mut gap = vec![b'G'; 5000];
                gap[200..200 + pcentrals.len()].copy_from_slice(&pcentrals);
                let fake = r43d_eocd(
                    total,
                    pcentrals.len() as u32,
                    (cd_start + 200) as u32,
                    0,
                    &[],
                );
                let mut comment = b"CC".to_vec();
                comment.extend_from_slice(&fake);
                comment.extend_from_slice(b"DDDDDDDDDDDDDDDDDDDD");
                out.extend_from_slice(&gap);
                out.extend_from_slice(&centrals);
                out.extend_from_slice(&r43d_eocd(
                    total,
                    centrals.len() as u32,
                    (cd_start + 5000) as u32,
                    0,
                    &comment,
                ));
            }
            "disk1" => {
                out.extend_from_slice(&centrals);
                out.extend_from_slice(&r43d_eocd(
                    total,
                    centrals.len() as u32,
                    cd_start as u32,
                    1,
                    &[],
                ));
            }
            "count1" => {
                out.extend_from_slice(&centrals);
                out.extend_from_slice(&r43d_eocd(
                    1,
                    centrals.len() as u32,
                    cd_start as u32,
                    0,
                    &[],
                ));
            }
            _ => {
                out.extend_from_slice(&centrals);
                out.extend_from_slice(&r43d_eocd(
                    total,
                    centrals.len() as u32,
                    cd_start as u32,
                    0,
                    &[],
                ));
            }
        }
        out
    }

    /// A MusicXML score in the generator's shape.
    fn r43d_score(title: &str) -> Vec<u8> {
        let pre = "<?xml version=\"1.0\"?><score-partwise version=\"3.1\"><work><work-title>";
        let post = "</work-title></work><part-list><score-part id=\"P1\"><part-name>Music</part-name></score-part></part-list><part id=\"P1\"><measure number=\"1\"><note><pitch><step>C</step><octave>4</octave></pitch><duration>4</duration></note></measure></part></score-partwise>";
        let mut out = pre.to_string();
        out.push_str(title);
        out.push_str(post);
        out.into_bytes()
    }

    /// One round-43 entry set: `(entries, key, plant)` —
    /// its owned `(name, data)` entries, the key entry's
    /// index, and the plant content.
    type R43dSet = (Vec<(Vec<u8>, Vec<u8>)>, usize, Vec<u8>);

    /// The four entry sets of the round-43 crafts:
    /// `(entries, key, plant)` with owned contents —
    /// the generator's names and duplicate shapes. The
    /// idml set is this door's own package shape (the
    /// witness parts, the key the first story); the
    /// idml, clip, and mxl plant contents are the real
    /// key contents' length; the font plant is not.
    fn r43d_entry_sets() -> Vec<R43dSet> {
        let filler = vec![b'A'; 600];
        let story_a = one_paragraph_story("First story words here");
        let story_b = one_paragraph_story("Secondcopy story words here");
        let mut idml_parts = witness_parts(&[story_a, story_b]);
        idml_parts.push((b"filler.bin".to_vec(), filler.clone()));
        vec![
            (
                idml_parts,
                2,
                one_paragraph_story("PLANT story words here").into_bytes(),
            ),
            (
                vec![
                    (b"MyFont.ttf".to_vec(), vec![b'R'; 1000]),
                    (b"MyFont.ttf".to_vec(), vec![b'B'; 900]),
                    (b"filler.bin".to_vec(), filler.clone()),
                ],
                0,
                vec![b'Q'; 800],
            ),
            (
                vec![
                    (b"pic.png".to_vec(), vec![b'P'; 69]),
                    (b"pic.png".to_vec(), vec![b'Q'; 69]),
                    (b"filler.bin".to_vec(), filler.clone()),
                ],
                0,
                vec![b'Q'; 69],
            ),
            (
                vec![
                    (
                        b"META-INF/container.xml".to_vec(),
                        b"<?xml version=\"1.0\"?><container><rootfiles><rootfile full-path=\"score.musicxml\"/></rootfiles></container>".to_vec(),
                    ),
                    (b"score.musicxml".to_vec(), r43d_score("FirstScore")),
                    (b"score.musicxml".to_vec(), r43d_score("SecondScore")),
                    (b"filler.bin".to_vec(), filler),
                ],
                1,
                r43d_score("PlantScore"),
            ),
        ]
    }

    /// This door's outcome on one archive, as a
    /// comparable string: the loaded spans' text, or
    /// the refusal's words.
    fn r43d_idml_outcome(bytes: &[u8]) -> String {
        match dump_from_idml(bytes, "witness.idml") {
            Ok(dump) => {
                let texts: Vec<&str> = dump.spans.iter().map(|span| span.text.as_str()).collect();
                format!("OK {}", texts.join("|"))
            }
            Err(err) => format!("ERR {err}"),
        }
    }

    /// One zip64 end-of-central-directory record in
    /// the round-44 crafts' shape: see the twin
    /// helper in ob-model's zip_directory tests.
    fn r44g_zip64_record(total: u64, cd_size: u64, cd_offset: u64) -> Vec<u8> {
        let mut out: Vec<u8> = Vec::new();
        out.extend_from_slice(b"PK\x06\x06");
        out.extend_from_slice(&44u64.to_le_bytes());
        out.extend_from_slice(&45u16.to_le_bytes());
        out.extend_from_slice(&45u16.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&total.to_le_bytes());
        out.extend_from_slice(&total.to_le_bytes());
        out.extend_from_slice(&cd_size.to_le_bytes());
        out.extend_from_slice(&cd_offset.to_le_bytes());
        out
    }

    /// One zip64 locator in the round-44 crafts'
    /// shape, naming the record at `record_at`.
    fn r44g_zip64_locator(record_at: u64) -> Vec<u8> {
        let mut out: Vec<u8> = Vec::new();
        out.extend_from_slice(b"PK\x06\x07");
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&record_at.to_le_bytes());
        out.extend_from_slice(&1u32.to_le_bytes());
        out
    }

    /// The saturated plain end record a zip64 writer
    /// leaves behind (round-44 crafts).
    fn r44g_saturated_eocd() -> Vec<u8> {
        let mut out: Vec<u8> = Vec::new();
        out.extend_from_slice(b"PK\x05\x06");
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&u16::MAX.to_le_bytes());
        out.extend_from_slice(&u16::MAX.to_le_bytes());
        out.extend_from_slice(&u32::MAX.to_le_bytes());
        out.extend_from_slice(&u32::MAX.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out
    }

    /// The round-44 zip64 crafts (red team RT44-A1,
    /// generator `mk44.py`), assembled byte by byte
    /// over the round-43 entry sets: "clean" the real
    /// archive alone, under a plain end record;
    /// "zip64real" the real archive under a zip64
    /// record carrying the TRUE directory, a locator
    /// naming that record, and a saturated plain end
    /// record; "wrongcd" the real locals, a
    /// 4,000-byte gap holding the plant directory at
    /// +300 (the plant body's own records, the key
    /// entry's record describing the plant content),
    /// then the real directory, then a zip64 record
    /// carrying the PLANT directory's offset, size,
    /// and count, a locator naming that record at its
    /// true position, and a saturated plain end
    /// record; "badlocator" the zip64-real shape with
    /// its locator pointing at 0.
    fn r44g_archive(entries: &[(&[u8], &[u8])], key: usize, plant: &[u8], shape: &str) -> Vec<u8> {
        let mut body: Vec<u8> = Vec::new();
        let mut locals: Vec<u32> = Vec::new();
        for (name, data) in entries.iter() {
            locals.push(body.len() as u32);
            body.extend_from_slice(&r43d_local(name, data, 0, false));
        }
        let cd_start = body.len();
        let mut centrals: Vec<u8> = Vec::new();
        for (i, (name, data)) in entries.iter().enumerate() {
            centrals.extend_from_slice(&r43d_central(name, data, locals[i], 0));
        }
        let mut pbody: Vec<u8> = Vec::new();
        let mut plocals: Vec<u32> = Vec::new();
        for (i, (name, data)) in entries.iter().enumerate() {
            plocals.push(pbody.len() as u32);
            let pdata = if i == key { plant } else { data };
            pbody.extend_from_slice(&r43d_local(name, pdata, 0, false));
        }
        let mut pcentrals: Vec<u8> = Vec::new();
        for (i, (name, data)) in entries.iter().enumerate() {
            let pdata = if i == key { plant } else { data };
            pcentrals.extend_from_slice(&r43d_central(name, pdata, plocals[i], 0));
        }
        let total = entries.len() as u64;
        let body_len = body.len();
        let mut out = body;
        match shape {
            "zip64real" => {
                let record_at = (body_len + centrals.len()) as u64;
                out.extend_from_slice(&centrals);
                out.extend_from_slice(&r44g_zip64_record(
                    total,
                    centrals.len() as u64,
                    cd_start as u64,
                ));
                out.extend_from_slice(&r44g_zip64_locator(record_at));
                out.extend_from_slice(&r44g_saturated_eocd());
            }
            "wrongcd" => {
                let mut gap = vec![b'H'; 4000];
                gap[300..300 + pcentrals.len()].copy_from_slice(&pcentrals);
                let record_at = (body_len + 4000 + centrals.len()) as u64;
                out.extend_from_slice(&gap);
                out.extend_from_slice(&centrals);
                out.extend_from_slice(&r44g_zip64_record(
                    total,
                    pcentrals.len() as u64,
                    (body_len + 300) as u64,
                ));
                out.extend_from_slice(&r44g_zip64_locator(record_at));
                out.extend_from_slice(&r44g_saturated_eocd());
            }
            "badlocator" => {
                out.extend_from_slice(&centrals);
                out.extend_from_slice(&r44g_zip64_record(
                    total,
                    centrals.len() as u64,
                    cd_start as u64,
                ));
                out.extend_from_slice(&r44g_zip64_locator(0));
                out.extend_from_slice(&r44g_saturated_eocd());
            }
            _ => {
                out.extend_from_slice(&centrals);
                out.extend_from_slice(&r43d_eocd(
                    entries.len() as u16,
                    centrals.len() as u32,
                    cd_start as u32,
                    0,
                    &[],
                ));
            }
        }
        out
    }

    #[test]
    fn r44g_zip64_wrongcd_and_controls_match_the_clean_control_at_the_idml_door() {
        // Round 44, F9 (red team RT44-A1): a zip64
        // EOCD whose CD offset points at a gap plant
        // directory was judged on all four entry sets.
        // This door's outcomes held on the audited
        // tree — the disagreeing duplicate story
        // refuses under either directory, since the
        // plant carries the real names — so this pin
        // stands as the door's control: with the
        // frame's directory judged, every outcome on
        // the wrongcd crafts matches the clean control
        // in all four entry sets, zip64-real is
        // clean-identical, and the bad-locator shape
        // refuses in the reader's standing
        // could-not-find words.
        for (set, (entries, key, plant)) in r43d_entry_sets().into_iter().enumerate() {
            let slices: Vec<(&[u8], &[u8])> = entries
                .iter()
                .map(|(name, data)| (name.as_slice(), data.as_slice()))
                .collect();
            let clean = r44g_archive(&slices, key, &plant, "clean");
            let control = r43d_idml_outcome(&clean);
            for shape in ["zip64real", "wrongcd"] {
                let bytes = r44g_archive(&slices, key, &plant, shape);
                let outcome = r43d_idml_outcome(&bytes);
                assert_eq!(outcome, control, "set {set} shape {shape}");
            }
            let bytes = r44g_archive(&slices, key, &plant, "badlocator");
            let outcome = r43d_idml_outcome(&bytes);
            assert!(
                outcome.contains("Could not find EOCD"),
                "set {set} badlocator: the uniform refusal: {outcome}"
            );
        }
    }

    #[test]
    fn r43d_trailing_and_held_archives_match_the_clean_control_at_the_idml_door() {
        // Round 43, F6 (red team RT43-A1): a complete
        // second archive appended after the first was
        // judged at its trailing directory for three of
        // the four entry sets. This door's outcomes held
        // on the audited tree — the disagreeing
        // duplicate story refuses under either directory,
        // since the plant carries the real names — so
        // this pin stands as the door's control: with
        // the first complete directory judged, every
        // outcome on the trailing crafts matches the
        // clean control in all four entry sets, the held
        // arms (the comment-embedded end record, the
        // local-header mutations, the disk fields set
        // to 1) are clean-identical, and the count
        // falsified to 1 refuses in the standing
        // underivable words.
        for (set, (entries, key, plant)) in r43d_entry_sets().into_iter().enumerate() {
            let slices: Vec<(&[u8], &[u8])> = entries
                .iter()
                .map(|(name, data)| (name.as_slice(), data.as_slice()))
                .collect();
            let clean = r43d_archive(&slices, key, &plant, "clean");
            let control = r43d_idml_outcome(&clean);
            for shape in [
                "trailing",
                "comment",
                "nameflip",
                "crcbad",
                "descriptor",
                "disk1",
            ] {
                let bytes = r43d_archive(&slices, key, &plant, shape);
                let outcome = r43d_idml_outcome(&bytes);
                assert_eq!(outcome, control, "set {set} shape {shape}");
            }
            let bytes = r43d_archive(&slices, key, &plant, "count1");
            let outcome = r43d_idml_outcome(&bytes);
            assert!(
                outcome.contains("central directory cannot be derived"),
                "set {set} count1: the uniform refusal: {outcome}"
            );
        }
    }
}
