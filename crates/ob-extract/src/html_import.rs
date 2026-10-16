//! HTML import: a web page or newsletter export becomes a
//! draft bulletin (docs/planning/beyond-print.md section 3).
//!
//! The real-world case is a Mailchimp newsletter export: the
//! church already wrote the words once, in the newsletter
//! tool, and should not have to type them again. The page
//! comes through the one open door like every other rescue
//! and lands as a draft for the standard review.
//!
//! The honesty rules, same as the rest of the rescue family:
//!
//! - The words come over in reading order, with their bold
//!   and italic as runs. Headings map to the draft's heading
//!   styles by level; list items become body paragraphs
//!   starting with a bullet; a table row becomes one tabbed
//!   line, cells joined by tabs.
//! - Layout fidelity is NOT promised. Style attributes and
//!   CSS are ignored entirely; columns do not survive. The
//!   review says so.
//! - What a bulletin cannot carry is dropped WITH A COUNT,
//!   never silently: scripts, style blocks, forms, form
//!   fields, buttons, and iframes are counted by kind into
//!   one review item; links keep their words and lose their
//!   addresses, counted; pictures that live on the web are
//!   counted and NOT fetched.
//! - The page is measured before it is parsed (round 2,
//!   RT-3): a byte-level scan refuses pages built too
//!   deeply or of too many pieces in linear time, because
//!   the parser's own cost grows with depth. The parsed
//!   tree is then measured again, as before.
//! - Import makes NO network calls, ever. A picture is
//!   brought in only when its bytes are inside the page
//!   itself (a data: URI); those decode into the document's
//!   asset store exactly the way the G1a picture channel
//!   carries them (source: the page's file name, license
//!   unknown) and are placed as picture boxes where the
//!   page had them.
//!
//! Parsing is html5ever (the HTML parser from the Servo
//! project, MIT OR Apache-2.0) over its reference DOM. The
//! scraper crate was evaluated and not taken: its selector
//! machinery (the selectors and cssparser crates, MPL-2.0)
//! would add a copyleft family to the tree for an API this
//! walk does not need; the licenses are recorded in
//! docs/LEGAL.md.

use crate::draft::{Badge, DraftOutcome, ImportItem};
use base64::Engine as _;
use html5ever::tendril::TendrilSink as _;
use markup5ever_rcdom::{Handle, NodeData};
use ob_model::{
    default_styles, normalize_runs, AssetKind, ChurchProfile, ContentRole, Document, Frame,
    FrameKind, LicenseKind, Panel, Paragraph, PictureFit, PicturePlacement, Rect, RightsRecord,
    Run, SheetSpec, Size, Story, FORMAT_VERSION,
};

/// Does this file's head say it is a web page? The sniff for
/// the one open door, so a newsletter export whose name lost
/// its .html still routes here: a byte-order mark and leading
/// whitespace are allowed, an XML prolog is stepped over,
/// and then the head must open with `<!doctype html` or an
/// `<html` tag, case-insensitive.
pub fn sniff_html(bytes: &[u8]) -> bool {
    let mut s = bytes;
    if s.starts_with(&[0xEF, 0xBB, 0xBF]) {
        s = &s[3..];
    }
    s = skip_space(s);
    if s.starts_with(b"<?xml") {
        match find_subslice(s, b"?>") {
            Some(i) => s = skip_space(&s[i + 2..]),
            None => return false,
        }
    }
    let head: Vec<u8> = s.iter().take(64).map(|b| b.to_ascii_lowercase()).collect();
    if head.starts_with(b"<!doctype html") {
        return true;
    }
    if head.starts_with(b"<html") {
        return matches!(
            head.get(5),
            Some(b'>') | Some(b' ') | Some(b'\t') | Some(b'\n') | Some(b'\r')
        );
    }
    false
}

fn skip_space(mut s: &[u8]) -> &[u8] {
    while let Some((&b, rest)) = s.split_first() {
        if b == b' ' || b == b'\t' || b == b'\n' || b == b'\r' || b == 0x0B || b == 0x0C {
            s = rest;
        } else {
            break;
        }
    }
    s
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// What kind of page machinery was dropped, in plain words.
#[derive(Clone, Copy, PartialEq, Eq)]
enum DropKind {
    Script,
    StyleBlock,
    Form,
    Field,
    Button,
    Iframe,
}

impl DropKind {
    /// (article for one, noun for one, noun for many)
    fn words(self) -> (&'static str, &'static str, &'static str) {
        match self {
            DropKind::Script => ("a", "script", "scripts"),
            DropKind::StyleBlock => ("a", "style block", "style blocks"),
            DropKind::Form => ("a", "form", "forms"),
            DropKind::Field => ("a", "form field", "form fields"),
            DropKind::Button => ("a", "button", "buttons"),
            DropKind::Iframe => ("an", "iframe", "iframes"),
        }
    }
}

/// The walk's counts of everything that did not come over.
#[derive(Default)]
struct Counts {
    dropped: Vec<(DropKind, usize)>,
    links: usize,
    web_pictures: usize,
}

impl Counts {
    fn drop(&mut self, kind: DropKind) {
        match self.dropped.iter_mut().find(|(k, _)| *k == kind) {
            Some((_, n)) => *n += 1,
            None => self.dropped.push((kind, 1)),
        }
    }
    fn dropped_total(&self) -> usize {
        self.dropped.iter().map(|(_, n)| *n).sum()
    }
}

/// A picture whose bytes were inside the page: its bytes
/// and stated media type, in page order.
struct CarriedPicture {
    bytes: Vec<u8>,
    media_type: String,
}

/// One piece of the draft, in page order: a run of words
/// (one story's worth of paragraphs) or a carried picture
/// (an index into the walker's picture list).
enum Block {
    Words(Vec<Paragraph>),
    Picture(usize),
}

/// The paragraph being accumulated: the style it will wear
/// and its runs so far.
struct CurPara {
    style: String,
    /// h4 and deeper headings: body style, words bolded, so
    /// the structure still shows (the markdown rule).
    bold_all: bool,
    runs: Vec<Run>,
}

/// Inline formatting state as the walk descends.
#[derive(Clone, Copy, Default)]
struct Fmt {
    bold: u32,
    italic: u32,
}

struct Walker {
    blocks: Vec<Block>,
    cur: Option<CurPara>,
    /// The style the next paragraph to start will wear (set
    /// by headings, consumed when the paragraph starts).
    pending_style: Option<(String, bool)>,
    /// A prefix the next finished paragraph carries (a list
    /// item's bullet), whenever it finishes.
    pending_prefix: Option<String>,
    /// Pictures collected while walking a table cell: they
    /// join the block stream after the row they rode in on.
    cell_pictures: Vec<usize>,
    /// True while a table cell is being collected: pictures
    /// met there park in `cell_pictures` instead of joining
    /// the block stream on the spot.
    cell_pictures_open: bool,
    pictures: Vec<CarriedPicture>,
    counts: Counts,
    total_words: usize,
}

impl Walker {
    fn new() -> Self {
        Walker {
            blocks: Vec::new(),
            cur: None,
            pending_style: None,
            pending_prefix: None,
            cell_pictures: Vec::new(),
            cell_pictures_open: false,
            pictures: Vec::new(),
            counts: Counts::default(),
            total_words: 0,
        }
    }

    // --- paragraph plumbing ---------------------------------

    /// Start the current paragraph if words have arrived,
    /// wearing whatever style is pending (body by default).
    fn ensure_para(&mut self) {
        if self.cur.is_none() {
            let (style, bold_all) = self
                .pending_style
                .take()
                .unwrap_or_else(|| ("body".to_string(), false));
            self.cur = Some(CurPara {
                style,
                bold_all,
                runs: Vec::new(),
            });
        }
    }

    /// Append source text to the current paragraph, in the
    /// inline face in force: whitespace runs collapse to one
    /// space (a non-breaking space is a space), edges merge
    /// so no doubled space survives a tag boundary.
    fn push_text(&mut self, raw: &str, fmt: Fmt) {
        let mut collapsed = String::with_capacity(raw.len());
        let mut last_space = false;
        for c in raw.chars() {
            if c.is_whitespace() {
                if !last_space {
                    collapsed.push(' ');
                    last_space = true;
                }
            } else {
                collapsed.push(c);
                last_space = false;
            }
        }
        if collapsed.is_empty() {
            return;
        }
        self.ensure_para();
        let cur = self.cur.as_mut().expect("paragraph started");
        let bold = fmt.bold > 0;
        let italic = fmt.italic > 0;
        let mut text: &str = &collapsed;
        if let Some(last) = cur.runs.last() {
            if last.text.ends_with(' ') && text.starts_with(' ') {
                text = &text[1..];
            }
        } else if text.starts_with(' ') {
            text = text.trim_start_matches(' ');
        }
        if text.is_empty() {
            return;
        }
        match cur.runs.last_mut() {
            Some(last)
                if (last.bold == Some(true)) == bold && (last.italic == Some(true)) == italic =>
            {
                last.text.push_str(text);
            }
            _ => cur.runs.push(Run {
                text: text.to_string(),
                bold: if bold { Some(true) } else { None },
                italic: if italic { Some(true) } else { None },
            }),
        }
    }

    /// Finish the current paragraph, if it holds any words,
    /// and put it in the block stream.
    fn flush_para(&mut self) {
        let Some(mut cur) = self.cur.take() else {
            return;
        };
        // The paragraph's trailing edge is trimmed, as text is.
        while let Some(last) = cur.runs.last_mut() {
            let trimmed = last.text.trim_end().to_string();
            if trimmed == last.text && !trimmed.is_empty() {
                break;
            }
            last.text = trimmed;
            if last.text.is_empty() {
                cur.runs.pop();
            } else {
                break;
            }
        }
        if cur.bold_all {
            for r in cur.runs.iter_mut() {
                if r.bold.is_none() {
                    r.bold = Some(true);
                }
            }
        }
        let runs = normalize_runs(cur.runs);
        let text: String = runs.iter().map(|r| r.text.as_str()).collect();
        if text.trim().is_empty() {
            return;
        }
        self.total_words += text.split_whitespace().count();
        let mut runs = runs;
        if let Some(prefix) = self.pending_prefix.take() {
            runs.insert(
                0,
                Run {
                    text: prefix,
                    bold: None,
                    italic: None,
                },
            );
        }
        let para = Paragraph::from_runs(&cur.style, runs);
        match self.blocks.last_mut() {
            Some(Block::Words(paras)) => paras.push(para),
            _ => self.blocks.push(Block::Words(vec![para])),
        }
    }

    /// A picture block at this point of the walk.
    fn emit_picture(&mut self, index: usize) {
        self.flush_para();
        self.pending_prefix = None;
        self.blocks.push(Block::Picture(index));
    }

    // --- element pieces -------------------------------------

    /// An image element: its bytes come along only when they
    /// are inside the page (a data: URI). Anything addressed
    /// from elsewhere is counted, never fetched: import
    /// makes no network calls.
    fn handle_img(&mut self, src: Option<String>) {
        let Some(src) = src else { return };
        let src = src.trim();
        if src.is_empty() {
            return;
        }
        if src.len() >= 5 && src[..5].eq_ignore_ascii_case("data:") {
            match decode_data_uri(src) {
                Some((bytes, media_type)) if ob_image::probe(&bytes).is_ok() => {
                    let index = self.pictures.len();
                    self.pictures.push(CarriedPicture { bytes, media_type });
                    if self.cell_pictures_open {
                        self.cell_pictures.push(index);
                    } else {
                        self.emit_picture(index);
                    }
                }
                _ => self.counts.web_pictures += 1,
            }
        } else {
            self.counts.web_pictures += 1;
        }
    }

    /// Walk a node's children in order.
    fn walk_children(&mut self, node: &Handle, fmt: Fmt) {
        let children: Vec<Handle> = node.children.borrow().clone();
        for child in &children {
            self.walk_node(child, fmt);
        }
    }

    fn walk_node(&mut self, node: &Handle, fmt: Fmt) {
        match &node.data {
            NodeData::Text { contents } => {
                let text = contents.borrow();
                self.push_text(&text, fmt);
            }
            NodeData::Element { .. } => {
                self.walk_element(node, fmt);
            }
            _ => self.walk_children(node, fmt),
        }
    }

    fn walk_element(&mut self, node: &Handle, fmt: Fmt) {
        let NodeData::Element { name, attrs, .. } = &node.data else {
            return;
        };
        let tag: &str = &name.local;
        let attr = |want: &str| -> Option<String> {
            attrs
                .borrow()
                .iter()
                .find(|a| &*a.name.local == want)
                .map(|a| a.value.to_string())
        };
        match tag {
            // Dropped, counted, never walked into: a form's
            // own fields are part of the form, not separate
            // losses.
            "script" => self.counts.drop(DropKind::Script),
            "style" => self.counts.drop(DropKind::StyleBlock),
            "form" => self.counts.drop(DropKind::Form),
            "input" | "select" | "textarea" => self.counts.drop(DropKind::Field),
            "button" => self.counts.drop(DropKind::Button),
            "iframe" => self.counts.drop(DropKind::Iframe),
            // Page furniture the walk never enters: the head
            // when no body was found, and template contents
            // (inert by design; the parser also keeps them
            // outside the children this walk reads).
            "head" | "template" => {}
            "img" => self.handle_img(attr("src")),
            // The words on a link stay; the address does not.
            "a" => {
                if attr("href").is_some() {
                    self.counts.links += 1;
                }
                self.walk_children(node, fmt);
            }
            "strong" | "b" => {
                let mut fmt = fmt;
                fmt.bold += 1;
                self.walk_children(node, fmt);
            }
            "em" | "i" => {
                let mut fmt = fmt;
                fmt.italic += 1;
                self.walk_children(node, fmt);
            }
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                self.flush_para();
                self.pending_prefix = None;
                self.pending_style = Some(match tag {
                    "h1" => ("heading".to_string(), false),
                    "h2" | "h3" => ("subheading".to_string(), false),
                    _ => ("body".to_string(), true),
                });
                self.walk_children(node, fmt);
                self.flush_para();
                self.pending_style = None;
            }
            "br" => self.flush_para(),
            "hr" => {
                self.flush_para();
                self.pending_prefix = None;
            }
            "li" => {
                self.flush_para();
                self.pending_style = None;
                self.pending_prefix = Some("\u{2022} ".to_string());
                self.walk_children(node, fmt);
                self.flush_para();
                self.pending_prefix = None;
            }
            "table" => self.walk_table(node, fmt),
            "pre" => {
                self.flush_para();
                self.pending_prefix = None;
                self.pending_style = None;
                for line in text_content(node).lines() {
                    let line = line.trim_end();
                    if line.trim().is_empty() {
                        continue;
                    }
                    self.ensure_para();
                    if let Some(cur) = self.cur.as_mut() {
                        cur.runs.push(Run::plain(line));
                    }
                    self.flush_para();
                }
            }
            // Block containers: their words become paragraphs
            // of their own; anything inline directly inside
            // gathers into one paragraph per run of text.
            "p" | "div" | "section" | "article" | "header" | "footer" | "main" | "aside"
            | "blockquote" | "center" | "figure" | "figcaption" | "address" | "fieldset"
            | "details" | "summary" | "nav" | "hgroup" | "ul" | "ol" | "dl" | "dt" | "dd" => {
                self.flush_para();
                self.pending_style = None;
                self.walk_children(node, fmt);
                self.flush_para();
                self.pending_prefix = None;
            }
            // Everything else is inline and transparent:
            // span, font, u, small, sub, sup, code, and the
            // tags nobody planned for.
            _ => self.walk_children(node, fmt),
        }
    }

    /// A table whose cells hold only inline words: each
    /// row becomes one tabbed line, its cells' words joined
    /// by tabs (a two-column newsletter table becomes the
    /// tabbed lines a bulletin already speaks). A table
    /// whose cells hold structure (headings, lists, whole
    /// nested tables: the layout tables newsletter tools
    /// build) is walked as flow instead, so the blocks
    /// inside land as their own paragraphs in order and
    /// any content table inside still gets its tabbed
    /// lines. A caption becomes a paragraph ahead of the
    /// rows.
    fn walk_table(&mut self, node: &Handle, fmt: Fmt) {
        self.flush_para();
        self.pending_prefix = None;
        self.pending_style = None;
        let rows = table_rows(node);
        let tabbed = !rows.is_empty()
            && rows
                .iter()
                .all(|r| table_cells(r).iter().all(|(c, _)| cell_is_simple(c)));
        if !tabbed {
            self.walk_children(node, fmt);
            self.flush_para();
            return;
        }
        // The caption, when the table carries one.
        let children: Vec<Handle> = node.children.borrow().clone();
        for child in &children {
            if let NodeData::Element { name, .. } = &child.data {
                if &*name.local == "caption" {
                    let runs = self.collect_inline(child, fmt);
                    self.push_collected(runs);
                }
            }
        }
        for row in rows {
            let mut line_runs: Vec<Run> = Vec::new();
            let mut first_cell = true;
            for (cell, is_header) in table_cells(&row) {
                if !first_cell {
                    line_runs.push(Run::plain("\t"));
                }
                first_cell = false;
                let mut runs = self.collect_inline(&cell, fmt);
                if is_header {
                    for r in runs.iter_mut() {
                        if r.bold.is_none() {
                            r.bold = Some(true);
                        }
                    }
                }
                line_runs.extend(runs);
            }
            self.push_collected(line_runs);
            // Pictures that rode in this row's cells stand
            // right after the row's line.
            let rode: Vec<usize> = std::mem::take(&mut self.cell_pictures);
            for index in rode {
                self.emit_picture(index);
            }
        }
        // Any picture still parked (a caption's, or a row
        // with no line) stands after the table.
        let rode: Vec<usize> = std::mem::take(&mut self.cell_pictures);
        for index in rode {
            self.emit_picture(index);
        }
        self.flush_para();
    }

    /// Put already-collected runs down as one paragraph of
    /// body words (a table row's line, a caption).
    fn push_collected(&mut self, runs: Vec<Run>) {
        if runs.is_empty() {
            return;
        }
        self.flush_para();
        self.pending_style = None;
        self.cur = Some(CurPara {
            style: "body".to_string(),
            bold_all: false,
            runs,
        });
        self.flush_para();
    }

    /// Collect a subtree's words as runs without touching
    /// the paragraph in progress: block boundaries inside
    /// become spaces. Drops, links, and pictures inside are
    /// still counted and carried by the main walker's books
    /// (pictures are parked for the row code to place).
    fn collect_inline(&mut self, node: &Handle, fmt: Fmt) -> Vec<Run> {
        let mut acc: Vec<Run> = Vec::new();
        self.cell_pictures_open = true;
        self.collect_into(node, fmt, &mut acc);
        self.cell_pictures_open = false;
        // Trim the collected edges.
        while let Some(last) = acc.last_mut() {
            let trimmed = last.text.trim_end().to_string();
            if trimmed == last.text && !trimmed.is_empty() {
                break;
            }
            last.text = trimmed;
            if last.text.is_empty() {
                acc.pop();
            } else {
                break;
            }
        }
        while let Some(first) = acc.first_mut() {
            let trimmed = first.text.trim_start().to_string();
            if trimmed == first.text && !trimmed.is_empty() {
                break;
            }
            first.text = trimmed;
            if first.text.is_empty() {
                acc.remove(0);
            } else {
                break;
            }
        }
        normalize_runs(acc)
    }

    fn collect_into(&mut self, node: &Handle, fmt: Fmt, acc: &mut Vec<Run>) {
        match &node.data {
            NodeData::Text { contents } => {
                let text = contents.borrow();
                push_run_text(acc, &text, fmt);
            }
            NodeData::Element { name, attrs, .. } => {
                let tag: &str = &name.local;
                match tag {
                    "script" => self.counts.drop(DropKind::Script),
                    "style" => self.counts.drop(DropKind::StyleBlock),
                    "form" => self.counts.drop(DropKind::Form),
                    "input" | "select" | "textarea" => self.counts.drop(DropKind::Field),
                    "button" => self.counts.drop(DropKind::Button),
                    "iframe" => self.counts.drop(DropKind::Iframe),
                    "head" | "template" => {}
                    "img" => {
                        let src = attrs
                            .borrow()
                            .iter()
                            .find(|a| &*a.name.local == "src")
                            .map(|a| a.value.to_string());
                        self.handle_img(src);
                        push_run_text(acc, " ", fmt);
                    }
                    "a" => {
                        if attrs.borrow().iter().any(|a| &*a.name.local == "href") {
                            self.counts.links += 1;
                        }
                        self.collect_children_into(node, fmt, acc);
                    }
                    "strong" | "b" => {
                        let mut fmt = fmt;
                        fmt.bold += 1;
                        self.collect_children_into(node, fmt, acc);
                    }
                    "em" | "i" => {
                        let mut fmt = fmt;
                        fmt.italic += 1;
                        self.collect_children_into(node, fmt, acc);
                    }
                    "table" => {
                        // A nested table flattens into the
                        // cell: its rows' words, space-joined.
                        for row in table_rows(node) {
                            for (cell, _) in table_cells(&row) {
                                self.collect_into(&cell, fmt, acc);
                                push_run_text(acc, " ", fmt);
                            }
                        }
                    }
                    "br" => push_run_text(acc, " ", fmt),
                    // Block boundaries inside a cell are spaces.
                    "p" | "div" | "section" | "article" | "header" | "footer" | "main"
                    | "aside" | "blockquote" | "center" | "figure" | "figcaption" | "address"
                    | "fieldset" | "details" | "summary" | "nav" | "hgroup" | "ul" | "ol"
                    | "dl" | "dt" | "dd" | "li" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6"
                    | "tr" | "hr" => {
                        push_run_text(acc, " ", fmt);
                        self.collect_children_into(node, fmt, acc);
                        push_run_text(acc, " ", fmt);
                    }
                    _ => self.collect_children_into(node, fmt, acc),
                }
            }
            _ => self.collect_children_into(node, fmt, acc),
        }
    }

    fn collect_children_into(&mut self, node: &Handle, fmt: Fmt, acc: &mut Vec<Run>) {
        let children: Vec<Handle> = node.children.borrow().clone();
        for child in &children {
            self.collect_into(child, fmt, acc);
        }
    }
}

/// Append text to a free-standing run list, collapsing
/// whitespace and merging edges, the same discipline as
/// the paragraph walk.
fn push_run_text(acc: &mut Vec<Run>, raw: &str, fmt: Fmt) {
    let mut collapsed = String::with_capacity(raw.len());
    let mut last_space = false;
    for c in raw.chars() {
        if c.is_whitespace() {
            if !last_space {
                collapsed.push(' ');
                last_space = true;
            }
        } else {
            collapsed.push(c);
            last_space = false;
        }
    }
    if collapsed.is_empty() {
        return;
    }
    let bold = fmt.bold > 0;
    let italic = fmt.italic > 0;
    let mut text: &str = &collapsed;
    if let Some(last) = acc.last() {
        if last.text.ends_with(' ') && text.starts_with(' ') {
            text = &text[1..];
        }
    }
    if text.is_empty() {
        return;
    }
    match acc.last_mut() {
        Some(last)
            if (last.bold == Some(true)) == bold && (last.italic == Some(true)) == italic =>
        {
            last.text.push_str(text);
        }
        _ => acc.push(Run {
            text: text.to_string(),
            bold: if bold { Some(true) } else { None },
            italic: if italic { Some(true) } else { None },
        }),
    }
}

/// Every descendant text node of a subtree, concatenated
/// (a `pre` block's words, taken as they stand).
fn text_content(node: &Handle) -> String {
    let mut out = String::new();
    let mut stack: Vec<Handle> = vec![node.clone()];
    while let Some(n) = stack.pop() {
        match &n.data {
            NodeData::Text { contents } => out.push_str(&contents.borrow()),
            _ => {
                let children: Vec<Handle> = n.children.borrow().clone();
                for child in children.into_iter().rev() {
                    stack.push(child);
                }
            }
        }
    }
    out
}

/// Can this cell flatten into one cell of a tabbed line?
/// Only when nothing inside it carries structure of its
/// own: no headings, no lists, no nested table, no pre.
/// Inline wrappers and plain paragraphs flatten into the
/// cell's words with spaces, which is the shape content
/// tables actually take; a cell holding more than that is
/// layout, and its table is walked as flow instead.
fn cell_is_simple(cell: &Handle) -> bool {
    fn descend(node: &Handle) -> bool {
        let children: Vec<Handle> = node.children.borrow().clone();
        for child in &children {
            if let NodeData::Element { name, .. } = &child.data {
                match &*name.local {
                    "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "ul" | "ol" | "li" | "table"
                    | "pre" => return false,
                    _ => {}
                }
            }
            if !descend(child) {
                return false;
            }
        }
        true
    }
    descend(cell)
}

/// A table's rows, in order, descending through the
/// section wrappers (thead, tbody, tfoot) but never into
/// a nested table.
fn table_rows(table: &Handle) -> Vec<Handle> {
    fn descend(node: &Handle, out: &mut Vec<Handle>) {
        let children: Vec<Handle> = node.children.borrow().clone();
        for child in &children {
            if let NodeData::Element { name, .. } = &child.data {
                match &*name.local {
                    "tr" => {
                        out.push(child.clone());
                        continue;
                    }
                    "table" => continue,
                    _ => {}
                }
            }
            descend(child, out);
        }
    }
    let mut rows = Vec::new();
    descend(table, &mut rows);
    rows
}

/// A row's cells (td and th), in order, never descending
/// into a nested table's cells.
fn table_cells(row: &Handle) -> Vec<(Handle, bool)> {
    fn descend(node: &Handle, out: &mut Vec<(Handle, bool)>) {
        let children: Vec<Handle> = node.children.borrow().clone();
        for child in &children {
            if let NodeData::Element { name, .. } = &child.data {
                match &*name.local {
                    "td" => {
                        out.push((child.clone(), false));
                        continue;
                    }
                    "th" => {
                        out.push((child.clone(), true));
                        continue;
                    }
                    "table" => continue,
                    _ => {}
                }
            }
            descend(child, out);
        }
    }
    let mut cells = Vec::new();
    descend(row, &mut cells);
    cells
}

/// A data: URI's payload: the decoded bytes and the stated
/// media type (sniffed when the URI states none). None when
/// the URI is malformed or its payload does not decode.
fn decode_data_uri(src: &str) -> Option<(Vec<u8>, String)> {
    let comma = src.find(',')?;
    let head = &src[..comma];
    let data = &src[comma + 1..];
    let meta = head.strip_prefix("data:").unwrap_or(head);
    let is_base64 = meta
        .rsplit(';')
        .next()
        .map(|p| p.trim().eq_ignore_ascii_case("base64"))
        .unwrap_or(false);
    let stated = meta.split(';').next().unwrap_or("").trim().to_string();
    let bytes = if is_base64 {
        let cleaned: String = data.chars().filter(|c| !c.is_whitespace()).collect();
        base64::engine::general_purpose::STANDARD
            .decode(cleaned.as_bytes())
            .ok()?
    } else {
        percent_decode(data)
    };
    let media_type = if stated.starts_with("image/") {
        stated.to_ascii_lowercase()
    } else {
        media_type_of(&bytes)?
    };
    Some((bytes, media_type))
}

fn percent_decode(data: &str) -> Vec<u8> {
    let raw = data.as_bytes();
    let mut out = Vec::with_capacity(raw.len());
    let mut i = 0;
    while i < raw.len() {
        if raw[i] == b'%' && i + 2 < raw.len() {
            let hex = |b: u8| -> Option<u8> {
                match b {
                    b'0'..=b'9' => Some(b - b'0'),
                    b'a'..=b'f' => Some(b - b'a' + 10),
                    b'A'..=b'F' => Some(b - b'A' + 10),
                    _ => None,
                }
            };
            if let (Some(hi), Some(lo)) = (hex(raw[i + 1]), hex(raw[i + 2])) {
                out.push(hi * 16 + lo);
                i += 3;
                continue;
            }
        }
        out.push(raw[i]);
        i += 1;
    }
    out
}

/// The media type a picture's bytes sniff to, when its
/// data: URI stated none.
fn media_type_of(bytes: &[u8]) -> Option<String> {
    use ob_image::PictureFormat as F;
    let media = match ob_image::sniff(bytes) {
        F::Png => "image/png",
        F::Jpeg => "image/jpeg",
        F::Gif => "image/gif",
        F::WebP => "image/webp",
        F::Svg => "image/svg+xml",
        F::Bmp => "image/bmp",
        F::Tiff => "image/tiff",
        F::JpegXl => "image/jxl",
        F::Avif => "image/avif",
        F::Heic => "image/heic",
        F::Unknown => return None,
    };
    Some(media.to_string())
}

// --- the document ----------------------------------------

const PAGE_W: i64 = 612_000;
const PAGE_H: i64 = 792_000;
const MARGIN: i64 = 36_000;
const CONTENT_W: i64 = PAGE_W - 2 * MARGIN;
const CONTENT_H: i64 = PAGE_H - 2 * MARGIN;
const FRAME_GAP: i64 = 12_000;

/// A rough height for a run of paragraphs at the draft's
/// content width: measured in characters, as the markdown
/// draft's one box never had to be. A draft is for
/// arranging; the estimate only decides page breaks
/// between boxes.
fn estimate_height(paras: &[Paragraph], styles: &[ob_model::ParagraphStyle]) -> i64 {
    let mut total: i64 = 0;
    for para in paras {
        let style = styles.iter().find(|s| s.id == para.style);
        let size = style.map(|s| s.font_size).unwrap_or(11_000) as f64;
        let after = style.map(|s| s.space_after).unwrap_or(4_000);
        let chars = para.text().chars().count() as f64;
        let line_w = (size * 0.48).max(1.0);
        let lines = (chars * line_w / CONTENT_W as f64).ceil().max(1.0);
        total += (lines * size * 1.3) as i64 + after;
    }
    total.clamp(20_000, CONTENT_H)
}

/// The count phrase for a drops list: "a form" for one,
/// "3 forms" for many.
fn count_phrase(article: &str, one: &str, many: &str, n: usize) -> String {
    if n == 1 {
        format!("{article} {one}")
    } else {
        format!("{n} {many}")
    }
}

/// "a script", "a script and a form", "2 scripts, a form,
/// and 3 form fields": the drops list in one breath.
fn join_phrases(phrases: &[String]) -> String {
    match phrases.len() {
        0 => String::new(),
        1 => phrases[0].clone(),
        2 => format!("{} and {}", phrases[0], phrases[1]),
        _ => {
            let (last, rest) = phrases.split_last().expect("nonempty");
            format!("{}, and {}", rest.join(", "), last)
        }
    }
}

/// The deepest tree the walk below will enter. The walk
/// recurses with the tree, and a page nested thousands of
/// levels deep (boxes in boxes in boxes) overflowed the
/// stack and aborted the whole app (audit Batch 1, the
/// audit's one BLOCKER). Real newsletter exports nest
/// perhaps fifty levels; this is generous headroom.
/// (Dropping a refused tree is safe at any depth: rcdom's
/// own Drop drains children iteratively.)
const MAX_TREE_DEPTH: usize = 256;

/// A tree's depth, measured iteratively: the document
/// node is 0, its children 1, and so on down.
fn tree_depth(root: &Handle) -> usize {
    let mut deepest = 0usize;
    let mut stack: Vec<(Handle, usize)> = vec![(root.clone(), 0)];
    while let Some((node, depth)) = stack.pop() {
        deepest = deepest.max(depth);
        for child in node.children.borrow().iter() {
            stack.push((child.clone(), depth + 1));
        }
    }
    deepest
}

/// The pre-parse budgets (round 2, RT-3). The tree
/// check above measures the parsed tree honestly, but
/// html5ever's cost grows quadratically with nesting
/// depth, so a page nested tens of thousands deep was
/// refused only after half a minute of parsing — on
/// the import thread, the same lost work as the abort
/// it replaced, at a slower clock. The scan below runs
/// FIRST, on the raw bytes, in linear time: a tolerant
/// tokenizer that counts tags and tracks naive nesting
/// depth. It is conservative by design (real pages
/// nest perhaps fifty levels, and the optional-close
/// rules below keep ordinary sloppy HTML from piling
/// into false depth), and the tree check still
/// governs afterwards: tree construction can deepen a
/// page past its source's apparent nesting, and that
/// deepening is caught there, as before.
const MAX_NAIVE_DEPTH: usize = 512;
const MAX_TAG_COUNT: usize = 500_000;

/// The refusal both pre-parse budgets speak.
const TOO_BIG_PAGE: &str = "This page is built too deeply (or has too many pieces) \
    to read safely, so it was not brought in.";

/// Elements that never hold anything: counted as tags
/// but never pushed onto the nesting stack.
const VOID_ELEMENTS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
];

/// Elements whose contents are not markup: everything
/// up to the matching close tag is text, skipped whole
/// (a script full of tag-shaped strings must not pile
/// into the counts).
const RAW_TEXT_ELEMENTS: &[&str] = &["script", "style", "textarea", "title"];

/// A block-level open that closes an open paragraph
/// (the HTML rule). Without it, a page of ordinary
/// unclosed paragraphs would stack into false depth.
fn is_block_open(name: &str) -> bool {
    matches!(
        name,
        "address"
            | "article"
            | "aside"
            | "blockquote"
            | "details"
            | "div"
            | "dl"
            | "dd"
            | "dt"
            | "fieldset"
            | "figcaption"
            | "figure"
            | "footer"
            | "form"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "header"
            | "hgroup"
            | "hr"
            | "li"
            | "main"
            | "menu"
            | "nav"
            | "ol"
            | "p"
            | "pre"
            | "section"
            | "table"
            | "ul"
    )
}

/// Does opening `new` implicitly close an open `top`?
/// The optional-close rules of ordinary HTML, so a
/// sloppy-but-shallow page (unclosed paragraphs and
/// list items are normal out there) does not
/// accumulate depth it does not have.
fn implied_close(top: &str, new: &str) -> bool {
    match top {
        "p" => is_block_open(new),
        "li" => new == "li",
        "dt" | "dd" => matches!(new, "dt" | "dd"),
        "td" | "th" => matches!(new, "td" | "th" | "tr" | "thead" | "tbody" | "tfoot"),
        "tr" => matches!(new, "tr" | "thead" | "tbody" | "tfoot"),
        "thead" | "tbody" | "tfoot" => matches!(new, "thead" | "tbody" | "tfoot"),
        "option" => matches!(new, "option" | "optgroup"),
        "optgroup" => new == "optgroup",
        "head" => new == "body",
        _ => false,
    }
}

/// The tag name at `from` (letters, digits, and the
/// punctuation tag names wear), lowercased, with the
/// offset just past it.
fn scan_tag_name(bytes: &[u8], from: usize) -> (String, usize) {
    let mut j = from;
    while j < bytes.len()
        && (bytes[j].is_ascii_alphanumeric() || matches!(bytes[j], b'-' | b':' | b'_'))
    {
        j += 1;
    }
    (
        String::from_utf8_lossy(&bytes[from..j]).to_ascii_lowercase(),
        j,
    )
}

/// The offset just past a tag's closing '>', honoring
/// quoted attribute values (a '>' inside quotes ends
/// nothing). None when the tag never ends.
fn scan_tag_end(bytes: &[u8], from: usize) -> Option<usize> {
    let mut quote: Option<u8> = None;
    let mut j = from;
    while j < bytes.len() {
        let b = bytes[j];
        match quote {
            Some(q) => {
                if b == q {
                    quote = None;
                }
            }
            None => match b {
                b'"' | b'\'' => quote = Some(b),
                b'>' => return Some(j + 1),
                _ => {}
            },
        }
        j += 1;
    }
    None
}

/// Does the tag ending at `end` (just past its '>')
/// close itself: a '/' as its last meaningful byte
/// before the '>'?
fn tag_is_self_closing(bytes: &[u8], end: usize) -> bool {
    let mut j = end - 1; // the '>' itself
    while j > 0 {
        j -= 1;
        match bytes[j] {
            b' ' | b'\t' | b'\r' | b'\n' => continue,
            b => return b == b'/',
        }
    }
    false
}

/// The offset of the close tag for a raw-text element
/// (`</script` and kin, any case, followed by a tag
/// delimiter), at or after `from`.
fn find_close_tag(bytes: &[u8], from: usize, name: &str) -> Option<usize> {
    let name = name.as_bytes();
    let mut i = from;
    while i + 2 + name.len() <= bytes.len() {
        let off = bytes[i..].iter().position(|&b| b == b'<')?;
        let p = i + off;
        if p + 2 + name.len() > bytes.len() {
            return None;
        }
        if bytes[p + 1] == b'/'
            && bytes[p + 2..p + 2 + name.len()]
                .iter()
                .zip(name.iter())
                .all(|(a, b)| a.eq_ignore_ascii_case(b))
        {
            match bytes.get(p + 2 + name.len()) {
                None => return Some(p),
                Some(b) if b.is_ascii_whitespace() || *b == b'>' || *b == b'/' => {
                    return Some(p);
                }
                _ => {}
            }
        }
        i = p + 1;
    }
    None
}

/// Measure a page before the parser sees it: refuse
/// when naive nesting runs past MAX_NAIVE_DEPTH or
/// the tag count past MAX_TAG_COUNT. Anything the
/// scan cannot make sense of is simply not counted;
/// the parser and the tree check judge the page
/// itself, as they always have.
fn pre_parse_budget(bytes: &[u8]) -> Result<(), String> {
    let mut stack: Vec<String> = Vec::new();
    let mut tags = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        let Some(off) = bytes[i..].iter().position(|&b| b == b'<') else {
            break;
        };
        let p = i + off;
        let rest = &bytes[p..];
        if rest.starts_with(b"<!--") {
            // A comment: skip to its end; a comment that
            // never ends makes the rest of the page one.
            match find_subslice(&bytes[p + 4..], b"-->") {
                Some(end) => i = p + 4 + end + 3,
                None => break,
            }
            continue;
        }
        if rest.starts_with(b"<!") || rest.starts_with(b"<?") {
            // Doctype, declaration, processing
            // instruction: skip the tag whole.
            match scan_tag_end(bytes, p + 2) {
                Some(end) => i = end,
                None => break,
            }
            continue;
        }
        if rest.starts_with(b"</") {
            // A close tag: pop through its element when
            // it is open (misnesting pops what sits above
            // it too, the tolerant reading); a close for
            // something never opened is ignored.
            let (name, after_name) = scan_tag_name(bytes, p + 2);
            tags += 1;
            if tags > MAX_TAG_COUNT {
                return Err(TOO_BIG_PAGE.to_string());
            }
            if let Some(pos) = stack.iter().rposition(|t| t == &name) {
                stack.truncate(pos);
            }
            match scan_tag_end(bytes, after_name) {
                Some(end) => i = end,
                None => break,
            }
            continue;
        }
        if rest.get(1).is_some_and(|b| b.is_ascii_alphabetic()) {
            let (name, after_name) = scan_tag_name(bytes, p + 1);
            tags += 1;
            if tags > MAX_TAG_COUNT {
                return Err(TOO_BIG_PAGE.to_string());
            }
            let Some(end) = scan_tag_end(bytes, after_name) else {
                break;
            };
            let self_closing = tag_is_self_closing(bytes, end);
            i = end;
            if VOID_ELEMENTS.contains(&name.as_str()) || self_closing {
                continue;
            }
            while let Some(top) = stack.last() {
                if implied_close(top, &name) {
                    stack.pop();
                } else {
                    break;
                }
            }
            let is_raw_text = RAW_TEXT_ELEMENTS.contains(&name.as_str());
            stack.push(name);
            if stack.len() > MAX_NAIVE_DEPTH {
                return Err(TOO_BIG_PAGE.to_string());
            }
            if is_raw_text {
                // The contents are not markup: jump to
                // the matching close tag and let the
                // loop count it and pop like any close.
                match find_close_tag(bytes, i, stack.last().expect("just pushed")) {
                    Some(pos) => i = pos,
                    None => break,
                }
            }
            continue;
        }
        // A '<' that opens nothing (words like "a < b"):
        // text; step past it.
        i = p + 1;
    }
    Ok(())
}

/// Build the draft bulletin from a web page's bytes.
/// Errors only when the page holds no words we can read.
pub fn document_from_html(
    bytes: &[u8],
    source_name: &str,
    profile: ChurchProfile,
) -> Result<DraftOutcome, String> {
    // Round 2, RT-3: measure before parsing (see
    // pre_parse_budget); the tree check below still
    // runs on what the parser built.
    pre_parse_budget(bytes)?;
    let dom = html5ever::parse_document(markup5ever_rcdom::RcDom::default(), Default::default())
        .from_utf8()
        .read_from(&mut &bytes[..])
        .unwrap_or_default();
    // Audit Batch 1: measure before walking, and refuse a
    // too-deep page whole, in plain words. Nothing partial
    // becomes a draft.
    if tree_depth(&dom.document) > MAX_TREE_DEPTH {
        return Err(
            "This page has boxes nested too deeply to read safely, so it was not brought in."
                .to_string(),
        );
    }
    let mut walker = Walker::new();
    // Walk the body when the page has one; otherwise the
    // whole tree (the head is skipped by the walk itself).
    let start = find_first(&dom.document, "body").unwrap_or_else(|| dom.document.clone());
    walker.walk_children(&start, Fmt::default());
    walker.flush_para();
    if walker.total_words == 0 {
        return Err("This page has no words we can read.".to_string());
    }

    let styles = default_styles();
    let mut doc = Document {
        format_version: FORMAT_VERSION,
        title: source_name.to_string(),
        profile,
        fields: Default::default(),
        styles: styles.clone(),
        stories: Vec::new(),
        panels: Vec::new(),
        sheet: SheetSpec {
            preset: "letter-portrait".into(),
            size: Size::new(PAGE_W, PAGE_H),
        },
        no_announcements: false,
        assets: Vec::new(),
        asset_bytes: Default::default(),
        occasion_time: None,
    };

    // Lay the blocks down the pages: word runs become story
    // boxes, pictures become picture boxes at the size the
    // picture itself asks for, centered in the column.
    let mut panels: Vec<Panel> = Vec::new();
    let mut frames: Vec<Frame> = Vec::new();
    let mut cursor = MARGIN;
    let mut story_no = 0usize;
    let mut frame_no = 0usize;
    let mut first_words_panel = 0usize;
    let mut first_picture_panel: Option<usize> = None;
    let mut paragraph_total = 0usize;
    let new_panel = |panels: &mut Vec<Panel>, frames: &mut Vec<Frame>| {
        let n = panels.len() + 1;
        panels.push(Panel {
            id: format!("page-{n}"),
            name: format!("Page {n}"),
            size: Size::new(PAGE_W, PAGE_H),
            frames: std::mem::take(frames),
            background_image: None,
            guides: Vec::new(),
        });
    };
    for block in &walker.blocks {
        let (w, h, is_words) = match block {
            Block::Words(paras) => {
                paragraph_total += paras.len();
                (CONTENT_W, estimate_height(paras, &styles), true)
            }
            Block::Picture(index) => {
                let pic = &walker.pictures[*index];
                let (pw, ph) = ob_image::probe(&pic.bytes)
                    .map(|p| (p.width as i64 * 1000, p.height as i64 * 1000))
                    .unwrap_or((300_000, 225_000));
                let mut w = pw.clamp(1, CONTENT_W);
                let mut h = (ph * w / pw.max(1)).max(1);
                if h > CONTENT_H / 2 {
                    w = w * (CONTENT_H / 2) / h;
                    h = CONTENT_H / 2;
                }
                (w, h, false)
            }
        };
        if cursor + h > MARGIN + CONTENT_H && !frames.is_empty() {
            new_panel(&mut panels, &mut frames);
            cursor = MARGIN;
        }
        frame_no += 1;
        let x = if is_words {
            MARGIN
        } else {
            MARGIN + (CONTENT_W - w) / 2
        };
        match block {
            Block::Words(paras) => {
                if doc.stories.is_empty() {
                    first_words_panel = panels.len();
                }
                story_no += 1;
                let sid = format!("web-words-{story_no}");
                doc.stories.push(Story {
                    id: sid.clone(),
                    paragraphs: paras.clone(),
                    keep_through: None,
                });
                frames.push(Frame {
                    id: format!("web-frame-{frame_no}"),
                    name: "Words from the page".into(),
                    kind: FrameKind::Text,
                    rect: Rect { x, y: cursor, w, h },
                    story_id: Some(sid),
                    role: ContentRole::Weekly,
                    picture: None,
                    music: None,
                });
            }
            Block::Picture(index) => {
                if first_picture_panel.is_none() {
                    first_picture_panel = Some(panels.len());
                }
                let pic = &walker.pictures[*index];
                let hash = doc.add_asset(
                    &pic.bytes,
                    AssetKind::Picture,
                    &pic.media_type,
                    RightsRecord {
                        source: source_name.to_string(),
                        license: LicenseKind::Unknown,
                        ..Default::default()
                    },
                );
                let mut placement = PicturePlacement::new(hash);
                placement.fit = PictureFit::ShowWhole;
                frames.push(Frame {
                    id: format!("web-frame-{frame_no}"),
                    name: "Picture from the page".into(),
                    kind: FrameKind::Picture,
                    rect: Rect { x, y: cursor, w, h },
                    story_id: None,
                    role: ContentRole::Weekly,
                    picture: Some(placement),
                    music: None,
                });
            }
        }
        cursor += h + FRAME_GAP;
    }
    new_panel(&mut panels, &mut frames);
    let page_total = panels.len();
    doc.panels = panels;

    // The review list: the words first, then what came
    // with them and what did not, each counted in plain
    // words.
    let mut items: Vec<ImportItem> = Vec::new();
    let first_text: String = doc
        .stories
        .first()
        .and_then(|s| s.paragraphs.first())
        .map(|p| p.text().chars().take(38).collect())
        .unwrap_or_default();
    items.push(ImportItem {
        panel: first_words_panel,
        label: first_text,
        badge: Badge::CheckThis,
        detail: format!(
            "{} paragraph{} of words from {}, on {} page{}. The page's own layout did not \
             come over; the words are in reading order, ready to arrange.",
            paragraph_total,
            if paragraph_total == 1 { "" } else { "s" },
            source_name,
            page_total,
            if page_total == 1 { "" } else { "s" },
        ),
    });
    if !walker.pictures.is_empty() {
        let n = walker.pictures.len();
        items.push(ImportItem {
            panel: first_picture_panel.unwrap_or(0),
            label: "Pictures from the page".into(),
            badge: Badge::Matched,
            detail: format!(
                "{} picture{} were inside the page itself and came with it, placed where \
                 the page had {}.",
                n,
                if n == 1 { "" } else { "s" },
                if n == 1 { "it" } else { "them" },
            ),
        });
    }
    if walker.counts.web_pictures > 0 {
        let n = walker.counts.web_pictures;
        items.push(ImportItem {
            panel: 0,
            label: "Pictures that stayed on the web".into(),
            badge: Badge::CheckThis,
            detail: if n == 1 {
                "1 picture sits on the web and was not brought in. The words around it \
                 are here."
                    .into()
            } else {
                format!(
                    "{n} pictures sit on the web and were not brought in. The words \
                     around them are here."
                )
            },
        });
    }
    if walker.counts.links > 0 {
        let n = walker.counts.links;
        items.push(ImportItem {
            panel: 0,
            label: "Web addresses".into(),
            badge: Badge::Matched,
            detail: if n == 1 {
                "1 web address was left out; the words it was on are here.".into()
            } else {
                format!("{n} web addresses were left out; the words they were on are here.")
            },
        });
    }
    if walker.counts.dropped_total() > 0 {
        let order = [
            DropKind::Script,
            DropKind::StyleBlock,
            DropKind::Form,
            DropKind::Field,
            DropKind::Button,
            DropKind::Iframe,
        ];
        let phrases: Vec<String> = order
            .iter()
            .filter_map(|kind| {
                walker
                    .counts
                    .dropped
                    .iter()
                    .find(|(k, _)| k == kind)
                    .map(|(_, n)| {
                        let (article, one, many) = kind.words();
                        count_phrase(article, one, many, *n)
                    })
            })
            .collect();
        items.push(ImportItem {
            panel: 0,
            label: "Page machinery left out".into(),
            badge: Badge::Matched,
            detail: format!(
                "The page had {}; those do not come into a bulletin and were left out.",
                join_phrases(&phrases)
            ),
        });
    }
    Ok(DraftOutcome { doc, items })
}

/// The first element with this tag in the tree, if any.
fn find_first(node: &Handle, tag: &str) -> Option<Handle> {
    if let NodeData::Element { name, .. } = &node.data {
        if &*name.local == tag {
            return Some(node.clone());
        }
    }
    for child in node.children.borrow().iter() {
        if let Some(found) = find_first(child, tag) {
            return Some(found);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use ob_model::{AssetKind, LicenseKind};

    /// A Mailchimp-flavored newsletter export: nested
    /// layout tables with inline styles everywhere, a
    /// heading, bold and italic runs, a list, a link, a
    /// two-column schedule table, one picture addressed
    /// from the web, one picture carried inside the page,
    /// and the machinery (script, style block, form, stray
    /// field and button) that never comes over.
    const NEWSLETTER: &str = r##"<!DOCTYPE html PUBLIC "-//W3C//DTD XHTML 1.0 Transitional//EN" "http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd">
<html xmlns="http://www.w3.org/1999/xhtml">
<head>
<meta http-equiv="Content-Type" content="text/html; charset=UTF-8">
<title>Grace Notes</title>
</head>
<body style="margin: 0; padding: 0;">
<center>
<table border="0" cellpadding="0" cellspacing="0" width="600" style="background-color: #ffffff;">
  <tr>
    <td align="center" valign="top">
      <table border="0" cellpadding="0" cellspacing="0" width="100%">
        <tr>
          <td style="font-family: Arial, sans-serif; font-size: 14px; color: #333333;">
            <h2 style="margin: 0;">News for the Week</h2>
            <p>Join us for the <strong>harvest supper</strong> on Saturday, and bring a <em>friend</em> along.</p>
            <p>Read more <a href="https://example.org/supper">on the website</a> before you come.</p>
            <ul>
              <li>Set up tables at 4 pm</li>
              <li>Bring a salad or dessert</li>
            </ul>
            <table border="0" cellpadding="4" cellspacing="0">
              <tr><td>Sunday worship</td><td>10:00 AM</td></tr>
              <tr><td>Bible study</td><td>Wednesday 6:30 PM</td></tr>
            </table>
            <p><img src="https://example.org/banner.jpg" alt="Church picnic" width="600"></p>
            <p><img src="data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==" alt="Dot"></p>
          </td>
        </tr>
      </table>
    </td>
  </tr>
</table>
</center>
<style type="text/css">p { margin: 0; }</style>
<script type="text/javascript">var tracker = 1;</script>
<form action="https://example.org/subscribe" method="post">
  <input type="email" name="EMAIL" value="">
</form>
<p><input type="text" name="stray"> <button type="button">Click me</button></p>
</body>
</html>"##;

    fn all_paragraphs(doc: &Document) -> Vec<&Paragraph> {
        doc.stories
            .iter()
            .flat_map(|s| s.paragraphs.iter())
            .collect()
    }

    #[test]
    fn newsletter_words_arrive_in_order_with_their_faces() {
        let out = document_from_html(
            NEWSLETTER.as_bytes(),
            "newsletter.html",
            ChurchProfile::default(),
        )
        .expect("the newsletter imports");
        let paras = all_paragraphs(&out.doc);
        let texts: Vec<String> = paras.iter().map(|p| p.text()).collect();
        assert_eq!(
            texts,
            vec![
                "News for the Week",
                "Join us for the harvest supper on Saturday, and bring a friend along.",
                "Read more on the website before you come.",
                "\u{2022} Set up tables at 4 pm",
                "\u{2022} Bring a salad or dessert",
                "Sunday worship\t10:00 AM",
                "Bible study\tWednesday 6:30 PM",
            ],
            "the words, in reading order, table rows as tabbed lines"
        );
        // The heading wears the draft's subheading style.
        assert_eq!(paras[0].style, "subheading");
        // The runs kept their faces.
        let supper = paras[1];
        assert!(
            supper
                .runs
                .iter()
                .any(|r| r.text == "harvest supper" && r.bold == Some(true)),
            "the bold words are a bold run: {:?}",
            supper.runs
        );
        assert!(
            supper
                .runs
                .iter()
                .any(|r| r.text == "friend" && r.italic == Some(true)),
            "the italic word is an italic run: {:?}",
            supper.runs
        );
        // The whole draft is one page of letter portrait.
        assert_eq!(out.doc.panels.len(), 1);
        assert_eq!(out.doc.sheet.preset, "letter-portrait");
    }

    #[test]
    fn carried_picture_lands_as_an_asset_and_a_frame() {
        let out = document_from_html(
            NEWSLETTER.as_bytes(),
            "newsletter.html",
            ChurchProfile::default(),
        )
        .expect("the newsletter imports");
        let doc = &out.doc;
        assert_eq!(doc.assets.len(), 1, "one picture was inside the page");
        let asset = &doc.assets[0];
        assert_eq!(asset.kind, AssetKind::Picture);
        assert_eq!(asset.media_type, "image/png");
        assert_eq!(asset.rights.source, "newsletter.html");
        assert_eq!(asset.rights.license, LicenseKind::Unknown);
        assert!(
            doc.asset_data(&asset.sha256).is_some(),
            "its bytes came with it"
        );
        let frame = doc
            .panels
            .iter()
            .flat_map(|p| p.frames.iter())
            .find(|f| f.picture.is_some())
            .expect("a picture frame was placed");
        assert_eq!(
            frame.picture.as_ref().map(|p| p.asset.as_str()),
            Some(asset.sha256.as_str()),
            "the frame names the carried asset"
        );
        // The review says the picture came, as a Good item.
        assert!(out.items.iter().any(|i| {
            i.badge == Badge::Matched
                && i.detail.contains("1 picture")
                && i.detail.contains("came with it")
        }));
    }

    #[test]
    fn drops_links_and_web_pictures_are_counted_in_plain_words() {
        let out = document_from_html(
            NEWSLETTER.as_bytes(),
            "newsletter.html",
            ChurchProfile::default(),
        )
        .expect("the newsletter imports");
        let details: Vec<&str> = out.items.iter().map(|i| i.detail.as_str()).collect();
        // The words item leads, and names the shape honestly.
        assert_eq!(out.items[0].badge, Badge::CheckThis);
        assert!(
            details[0].contains("7 paragraphs of words from newsletter.html"),
            "words item: {}",
            details[0]
        );
        // One item counts the machinery, by kind.
        let drops = out
            .items
            .iter()
            .find(|i| i.label == "Page machinery left out")
            .expect("a drops item");
        assert_eq!(
            drops.detail,
            "The page had a script, a style block, a form, a form field, and a button; \
             those do not come into a bulletin and were left out."
        );
        // The link's words stayed (asserted above); its
        // address is counted here.
        let links = out
            .items
            .iter()
            .find(|i| i.label == "Web addresses")
            .expect("a links item");
        assert_eq!(
            links.detail,
            "1 web address was left out; the words it was on are here."
        );
        // The web picture was not fetched; the count says so.
        let webpics = out
            .items
            .iter()
            .find(|i| i.label == "Pictures that stayed on the web")
            .expect("a web pictures item");
        assert_eq!(webpics.badge, Badge::CheckThis);
        assert_eq!(
            webpics.detail,
            "1 picture sits on the web and was not brought in. The words around it are here."
        );
    }

    #[test]
    fn a_page_with_no_words_refuses_in_plain_words() {
        let html = "<html><body><script>var x = 1;</script>\
                    <img src=\"https://example.org/only.jpg\"></body></html>";
        let err = match document_from_html(html.as_bytes(), "empty.html", ChurchProfile::default())
        {
            Ok(_) => panic!("no words, no draft"),
            Err(e) => e,
        };
        assert_eq!(err, "This page has no words we can read.");
    }

    #[test]
    fn deep_headings_become_bold_body_and_nbsp_is_a_space() {
        let html = "<html><body><h4>Small&nbsp;print heading</h4>\
                    <p>Words&nbsp;follow here.</p></body></html>";
        let out = document_from_html(html.as_bytes(), "page.html", ChurchProfile::default())
            .expect("imports");
        let paras = all_paragraphs(&out.doc);
        assert_eq!(paras[0].style, "body");
        assert_eq!(paras[0].text(), "Small print heading");
        assert!(
            paras[0].runs.iter().all(|r| r.bold == Some(true)),
            "an h4's words are bolded body: {:?}",
            paras[0].runs
        );
        assert_eq!(paras[1].text(), "Words follow here.");
    }

    #[test]
    fn a_percent_encoded_svg_data_picture_is_carried() {
        let html = "<html><body><p>Before the art.</p>\
                    <p><img src=\"data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 10 10'%3E%3Crect width='10' height='10'/%3E%3C/svg%3E\"></p>\
                    </body></html>";
        let out = document_from_html(html.as_bytes(), "art.html", ChurchProfile::default())
            .expect("imports");
        assert_eq!(out.doc.assets.len(), 1);
        assert_eq!(out.doc.assets[0].media_type, "image/svg+xml");
        assert!(out
            .doc
            .panels
            .iter()
            .flat_map(|p| p.frames.iter())
            .any(|f| f.picture.is_some()));
    }

    #[test]
    fn deeply_nested_pages_refuse_instead_of_aborting() {
        // Audit Batch 1 (the audit's BLOCKER): pages nested
        // thousands of divs deep used to overflow the stack
        // and kill the whole app, unsaved work with it. The
        // hostile shapes are built from string pieces here;
        // the import must refuse them whole, in plain words.
        // Since round 2 (RT-3) the refusal comes from the
        // pre-parse scan, in its words, before the parser
        // spends quadratic time getting to the tree check.
        let mut divs = String::from("<html><body>");
        for _ in 0..10_000 {
            divs.push_str("<div>");
        }
        divs.push_str("words at the bottom");
        for _ in 0..10_000 {
            divs.push_str("</div>");
        }
        divs.push_str("</body></html>");
        let err = match document_from_html(divs.as_bytes(), "deep.html", ChurchProfile::default()) {
            Ok(_) => panic!("the deep page must be refused"),
            Err(e) => e,
        };
        assert!(err.contains("built too deeply"), "{err}");

        // The same attack wearing inline tags instead.
        let mut spans = String::from("<html><body><p>");
        for _ in 0..10_000 {
            spans.push_str("<span>");
        }
        spans.push_str("words at the bottom");
        for _ in 0..10_000 {
            spans.push_str("</span>");
        }
        spans.push_str("</p></body></html>");
        let err = match document_from_html(spans.as_bytes(), "deep2.html", ChurchProfile::default())
        {
            Ok(_) => panic!("the deep page must be refused"),
            Err(e) => e,
        };
        assert!(err.contains("built too deeply"), "{err}");

        // Nesting a real page would plausibly use (a
        // hundred levels of layout boxes) still comes over.
        let mut ok = String::from("<html><body>");
        for _ in 0..100 {
            ok.push_str("<div>");
        }
        ok.push_str("<p>Words from a deep page</p>");
        for _ in 0..100 {
            ok.push_str("</div>");
        }
        ok.push_str("</body></html>");
        let out = document_from_html(ok.as_bytes(), "ok.html", ChurchProfile::default())
            .expect("a hundred levels is a page, not a trap");
        assert!(all_paragraphs(&out.doc)
            .iter()
            .any(|p| p.text().contains("Words from a deep page")));
    }

    #[test]
    fn very_deep_pages_refuse_before_the_parser_starts() {
        // Round 2, RT-3: at 25,000 nested divs the old
        // order (parse, then measure the tree) produced
        // no verdict within a minute, the parse being
        // quadratic in depth. The pre-parse scan is
        // linear, so this test finishing quickly IS the
        // evidence; the refusal speaks the scan's words.
        let mut divs = String::from("<html><body>");
        for _ in 0..25_000 {
            divs.push_str("<div>");
        }
        divs.push_str("words at the bottom");
        for _ in 0..25_000 {
            divs.push_str("</div>");
        }
        divs.push_str("</body></html>");
        let err = match document_from_html(divs.as_bytes(), "deep.html", ChurchProfile::default()) {
            Ok(_) => panic!("the deep page must be refused"),
            Err(e) => e,
        };
        assert_eq!(
            err,
            "This page is built too deeply (or has too many pieces) to read \
             safely, so it was not brought in."
        );
    }

    #[test]
    fn a_page_of_too_many_pieces_refuses_before_the_parser_starts() {
        // The other pre-parse budget: half a million
        // tags, none of them deep. Void elements keep
        // the nesting at zero, so only the count can
        // refuse this page.
        let mut page = String::from("<html><body><p>Words first.</p>");
        for _ in 0..500_001 {
            page.push_str("<br>");
        }
        page.push_str("</body></html>");
        let err = match document_from_html(page.as_bytes(), "many.html", ChurchProfile::default()) {
            Ok(_) => panic!("the many-pieced page must be refused"),
            Err(e) => e,
        };
        assert!(err.contains("too many pieces"), "{err}");
    }

    #[test]
    fn sloppy_but_shallow_pages_import_like_tidy_ones() {
        // The pre-parse scan's conservative half (round
        // 2, RT-3): unclosed <p> and <li> are normal
        // HTML. They must neither pile into false depth
        // (600 unclosed paragraphs would pass a naive
        // 512 budget if each stacked on the last) nor
        // change what comes over: the sloppy page and
        // its tidy twin import the same words.
        let tidy = "<html><body>\
            <p>First words here</p>\
            <p>Second words here</p>\
            <ul><li>Alpha item</li><li>Beta item</li><li>Gamma item</li></ul>\
            <p>Last words here</p></body></html>";
        let sloppy = "<html><body>\
            <p>First words here\
            <p>Second words here\
            <ul><li>Alpha item<li>Beta item<li>Gamma item</ul>\
            <p>Last words here</body></html>";
        let texts = |html: &str| -> Vec<String> {
            let out = document_from_html(html.as_bytes(), "page.html", ChurchProfile::default())
                .expect("the page imports");
            all_paragraphs(&out.doc).iter().map(|p| p.text()).collect()
        };
        assert_eq!(texts(sloppy), texts(tidy));

        let mut pile = String::from("<html><body>");
        for n in 0..600 {
            pile.push_str(&format!("<p>Paragraph number {n}"));
        }
        pile.push_str("<ul>");
        for n in 0..600 {
            pile.push_str(&format!("<li>Item number {n}"));
        }
        pile.push_str("</ul></body></html>");
        let out = document_from_html(pile.as_bytes(), "pile.html", ChurchProfile::default())
            .expect("unclosed paragraphs and items are a page, not a trap");
        assert!(all_paragraphs(&out.doc).len() > 600);
    }

    #[test]
    fn the_sniff_reads_a_pages_head() {
        assert!(sniff_html(b"<!DOCTYPE html><html><body>x</body></html>"));
        assert!(sniff_html(b"  \n<HTML LANG=\"en\">"));
        assert!(sniff_html(b"\xEF\xBB\xBF<html><body>x</body></html>"));
        assert!(sniff_html(
            b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE html><html></html>"
        ));
        assert!(!sniff_html(b"%PDF-1.7 not a page"));
        assert!(!sniff_html(b"Plain notes about <html> tags"));
        assert!(!sniff_html(b"<htmlx>not a tag</htmlx>"));
        assert!(!sniff_html(b""));
    }
}
