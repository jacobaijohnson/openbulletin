//! Markdown, both directions (G1b).
//!
//! In: a `.md` file's words become a draft bulletin through
//! the one open door, and text pasted into the canvas
//! editor with markdown markers in it lands as formatted
//! runs. Out: a bulletin's words as markdown, for a
//! website or newsletter, through the one export door.
//!
//! The dialect is deliberately small: `**bold**` /
//! `__bold__`, `*italic*` / `_italic_`, `#` / `##`
//! headings, `-` / `*` list items, blank-line paragraph
//! breaks. The honesty rules match the rest of the rescue
//! family: markers that do not pair up stay literal words,
//! markers inside a word (snake_case, 2*3) stay literal,
//! and nothing is ever guessed into formatting the writer
//! did not clearly mark.

use crate::{
    default_styles, normalize_runs, ChurchProfile, ContentRole, Document, Frame, FrameKind, Panel,
    Paragraph, ParagraphStyle, Run, SheetSpec, Size, Story, FORMAT_VERSION,
};

/// Can this marker open a formatted span here? The marker
/// must sit at a word edge (start, or after whitespace or
/// punctuation) and be followed by a non-space character.
fn opener_ok(chars: &[char], at: usize, marker_len: usize) -> bool {
    let before_ok = at == 0 || !chars[at - 1].is_alphanumeric();
    let after = chars.get(at + marker_len);
    before_ok && matches!(after, Some(c) if !c.is_whitespace())
}

/// Every position in the line where this marker could
/// close a span: the marker stands there, the character
/// before it is not whitespace, and it ends at a word
/// edge. Computed once per line (audit Batch 1): the
/// search used to rescan from every opener to the end of
/// the line, so a line of unmatched openers (`**a `
/// repeated, an ordinary broken-export shape) took time
/// squared in its length and effectively never finished.
fn closer_positions(chars: &[char], marker: char, marker_len: usize) -> Vec<usize> {
    let mut out = Vec::new();
    if chars.len() < marker_len {
        return out;
    }
    for j in 1..=chars.len() - marker_len {
        let is_marker = (0..marker_len).all(|k| chars[j + k] == marker);
        if !is_marker {
            continue;
        }
        let before_ok = !chars[j - 1].is_whitespace();
        let after_at = j + marker_len;
        let after_ok = after_at >= chars.len() || !chars[after_at].is_alphanumeric();
        if before_ok && after_ok {
            out.push(j);
        }
    }
    out
}

/// One line's closer lists, one per marker shape, so an
/// opener's search is a binary search, not a rescan. The
/// validity rule is exactly the one the rescan applied;
/// only the cost changed.
struct Closers {
    star_bold: Vec<usize>,
    star_italic: Vec<usize>,
    under_bold: Vec<usize>,
    under_italic: Vec<usize>,
}

impl Closers {
    fn new(chars: &[char]) -> Closers {
        Closers {
            star_bold: closer_positions(chars, '*', 2),
            star_italic: closer_positions(chars, '*', 1),
            under_bold: closer_positions(chars, '_', 2),
            under_italic: closer_positions(chars, '_', 1),
        }
    }

    /// The closer for a span whose content starts at
    /// `from`: the first valid closer strictly past it
    /// (the content between must be non-empty).
    fn find(&self, from: usize, marker: char, marker_len: usize) -> Option<usize> {
        let list = match (marker, marker_len) {
            ('*', 2) => &self.star_bold,
            ('*', _) => &self.star_italic,
            (_, 2) => &self.under_bold,
            _ => &self.under_italic,
        };
        let at = list.partition_point(|&j| j <= from);
        list.get(at).copied()
    }
}

/// Move the accumulated plain text into the run list.
fn push_plain(out: &mut Vec<Run>, plain: &mut String) {
    if !plain.is_empty() {
        let text = std::mem::take(plain);
        out.push(Run::plain(&text));
    }
}

/// Parse one line's inline markers into runs. `bold` and
/// `italic` say which markers are live at this level (bold
/// content re-parses for italic; italic content does not
/// nest further).
fn parse_inline(chars: &[char], bold: bool, italic: bool) -> Vec<Run> {
    let closers = Closers::new(chars);
    let mut out: Vec<Run> = Vec::new();
    let mut plain = String::new();
    let mut i = 0usize;
    while i < chars.len() {
        // Try the two-character markers first.
        let mut consumed = false;
        for (marker, len, is_bold) in [
            ('*', 2usize, true),
            ('_', 2, true),
            ('*', 1, false),
            ('_', 1, false),
        ] {
            if (is_bold && !bold) || (!is_bold && !italic) {
                continue;
            }
            let is_marker = (0..len).all(|k| chars.get(i + k) == Some(&marker));
            if !is_marker || !opener_ok(chars, i, len) {
                continue;
            }
            let content_start = i + len;
            let Some(close) = closers.find(content_start, marker, len) else {
                continue;
            };
            push_plain(&mut out, &mut plain);
            let inner = &chars[content_start..close];
            if is_bold {
                // Inside bold, italic markers still speak.
                for mut r in parse_inline(inner, false, true) {
                    r.bold = Some(true);
                    out.push(r);
                }
            } else {
                out.push(Run {
                    text: inner.iter().collect(),
                    bold: None,
                    italic: Some(true),
                });
            }
            i = close + len;
            consumed = true;
            break;
        }
        if !consumed {
            plain.push(chars[i]);
            i += 1;
        }
    }
    push_plain(&mut out, &mut plain);
    normalize_runs(out)
}

/// One line's inline markdown as runs.
pub fn inline_runs(line: &str) -> Vec<Run> {
    let chars: Vec<char> = line.chars().collect();
    parse_inline(&chars, true, true)
}

/// The style id for a heading of `level` (1 or 2) in a
/// style list: a style whose id or name is the heading
/// word, else None (the caller falls back to body + bold).
fn heading_style(styles: &[ParagraphStyle], level: usize) -> Option<String> {
    let word = if level == 1 { "heading" } else { "subheading" };
    styles
        .iter()
        .find(|s| s.id == word || s.name.eq_ignore_ascii_case(word))
        .map(|s| s.id.clone())
}

fn style_id(styles: &[ParagraphStyle], id: &str) -> String {
    if styles.iter().any(|s| s.id == id) {
        id.to_string()
    } else {
        styles
            .first()
            .map(|s| s.id.clone())
            .unwrap_or_else(|| "body".to_string())
    }
}

/// Markdown text as paragraphs against a style list:
/// headings take the heading styles when they exist (else
/// body with the words bolded), list markers are stripped,
/// soft-wrapped lines join with a space, blank lines break
/// paragraphs.
pub fn markdown_to_paragraphs(md: &str, styles: &[ParagraphStyle]) -> Vec<Paragraph> {
    let body_id = style_id(styles, "body");
    let mut out: Vec<Paragraph> = Vec::new();
    let mut cur: Option<(String, Vec<Run>)> = None;
    let flush = |cur: &mut Option<(String, Vec<Run>)>, out: &mut Vec<Paragraph>| {
        if let Some((style, runs)) = cur.take() {
            out.push(Paragraph::from_runs(&style, runs));
        }
    };
    for raw_line in md.lines() {
        let line = raw_line.trim_end();
        let trimmed = line.trim();
        if trimmed.is_empty() {
            flush(&mut cur, &mut out);
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("## ") {
            flush(&mut cur, &mut out);
            push_heading(&mut out, rest, 2, styles, &body_id);
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("# ") {
            flush(&mut cur, &mut out);
            push_heading(&mut out, rest, 1, styles, &body_id);
            continue;
        }
        let item = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "));
        if let Some(rest) = item {
            flush(&mut cur, &mut out);
            out.push(Paragraph::from_runs(&body_id, inline_runs(rest)));
            continue;
        }
        // A body line: start a paragraph or soft-join the
        // one in progress.
        let runs = inline_runs(trimmed);
        match &mut cur {
            None => cur = Some((body_id.clone(), runs)),
            Some((_, acc)) => {
                if let Some(last) = acc.last_mut() {
                    last.text.push(' ');
                }
                acc.extend(runs);
            }
        }
    }
    flush(&mut cur, &mut out);
    if out.is_empty() {
        out.push(Paragraph::plain(&body_id, ""));
    }
    out
}

fn push_heading(
    out: &mut Vec<Paragraph>,
    text: &str,
    level: usize,
    styles: &[ParagraphStyle],
    body_id: &str,
) {
    let runs = inline_runs(text);
    match heading_style(styles, level) {
        Some(id) => out.push(Paragraph::from_runs(&id, runs)),
        None => {
            // No heading style to wear: body, with the words
            // bolded so the page still shows the structure.
            let runs = runs
                .into_iter()
                .map(|mut r| {
                    if r.bold.is_none() {
                        r.bold = Some(true);
                    }
                    r
                })
                .collect();
            out.push(Paragraph::from_runs(body_id, runs));
        }
    }
}

/// A document from markdown text: one letter page, one box,
/// the words as one story. The shape every other rescue
/// produces, so the standard review flow takes it from
/// here.
pub fn document_from_markdown(text: &str, profile: ChurchProfile, source_name: &str) -> Document {
    let styles = default_styles();
    let paragraphs = markdown_to_paragraphs(text, &styles);
    Document {
        format_version: FORMAT_VERSION,
        title: source_name.to_string(),
        profile,
        fields: Default::default(),
        styles,
        stories: vec![Story {
            id: "notes".into(),
            paragraphs,
            keep_through: None,
        }],
        panels: vec![Panel {
            background_image: None,
            id: "page-1".into(),
            name: "Page 1".into(),
            size: Size::new(612_000, 792_000),
            frames: vec![Frame {
                id: "notes-frame".into(),
                name: "Notes".into(),
                kind: FrameKind::Text,
                rect: crate::Rect {
                    x: 36_000,
                    y: 36_000,
                    w: 540_000,
                    h: 720_000,
                },
                story_id: Some("notes".into()),
                role: ContentRole::Weekly,
                picture: None,
                music: None,
            }],
            guides: Vec::new(),
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
    }
}

/// A run's markdown: bold wraps in `**`, italic in `*`,
/// both in `***`. The words themselves are written as they
/// are (a literal asterisk in the text is not escaped; the
/// export is for reading and re-use, not a round-trip
/// format).
fn run_markdown(run: &Run) -> String {
    match (run.bold == Some(true), run.italic == Some(true)) {
        (true, true) => format!("***{}***", run.text),
        (true, false) => format!("**{}**", run.text),
        (false, true) => format!("*{}*", run.text),
        (false, false) => run.text.clone(),
    }
}

/// The document as markdown, the reading copy's sibling:
/// panels in order, each story once, headings from the
/// style names, field words resolved as the reading copy
/// resolves them. Spoken-only paragraphs never publish, so
/// they are not in here (the one rule; the count of them
/// rides with [`to_markdown_report`]).
pub fn to_markdown(doc: &Document) -> String {
    to_markdown_report(doc).0
}

/// [`to_markdown`] with the spoken-only count, so a door
/// can say how many names it left out.
pub fn to_markdown_report(doc: &Document) -> (String, usize) {
    let (filtered, spoken_left_out) = doc.without_spoken_only();
    (to_markdown_filtered(&filtered), spoken_left_out)
}

fn to_markdown_filtered(doc: &Document) -> String {
    // One field table for the whole document (see FieldTable).
    let table = crate::FieldTable::for_document(doc);
    let mut out = String::new();
    let mut seen: Vec<&str> = Vec::new();
    for panel in &doc.panels {
        for frame in &panel.frames {
            let Some(sid) = &frame.story_id else { continue };
            if seen.contains(&sid.as_str()) {
                continue;
            }
            seen.push(sid.as_str());
            let Some(story) = doc.story(sid) else {
                continue;
            };
            for para in &story.paragraphs {
                let resolved = table.resolve_runs(&para.runs);
                let text: String = resolved.iter().map(run_markdown).collect();
                let style = doc.style(&para.style);
                let name = style.map(|s| s.name.as_str()).unwrap_or("");
                let id = style.map(|s| s.id.as_str()).unwrap_or("");
                if name.eq_ignore_ascii_case("heading") || id == "heading" {
                    out.push_str("# ");
                } else if name.eq_ignore_ascii_case("subheading") || id == "subheading" {
                    out.push_str("## ");
                }
                out.push_str(text.trim());
                out.push_str("\n\n");
            }
        }
    }
    out
}

#[test]
fn web_hidden_paragraphs_stay_in_markdown() {
    // R-2b: not-for-web is a web-page rule only. Markdown
    // is a publication of the room's copy, like print: the
    // paragraph rides along.
    let mut hidden = Paragraph::new("body", "Timmy and June, front row");
    hidden.web_hidden = true;
    let doc = crate::Document {
        format_version: crate::FORMAT_VERSION,
        title: "Test".to_string(),
        profile: crate::ChurchProfile::default(),
        fields: Default::default(),
        styles: crate::default_styles(),
        stories: vec![crate::Story {
            id: "notes".to_string(),
            keep_through: None,
            paragraphs: vec![Paragraph::new("body", "For the food bank crew"), hidden],
        }],
        panels: vec![crate::Panel {
            id: "p1".to_string(),
            name: "Panel".to_string(),
            size: crate::Size::new(396_000, 612_000),
            frames: vec![crate::Frame {
                id: "f1".to_string(),
                name: "Notes".to_string(),
                kind: crate::FrameKind::Text,
                rect: crate::Rect::new(0, 0, 300_000, 500_000),
                story_id: Some("notes".to_string()),
                role: crate::ContentRole::Weekly,
                picture: None,
                music: None,
            }],
            background_image: None,
            guides: Vec::new(),
        }],
        sheet: crate::SheetSpec {
            preset: "letter-bifold".to_string(),
            size: crate::Size::new(792_000, 612_000),
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
    let text = to_markdown(&doc);
    assert!(
        text.contains("Timmy and June, front row"),
        "markdown keeps not-for-web words: {text}"
    );
}

#[test]
fn spoken_only_paragraphs_stay_out_of_markdown() {
    let mut spoken = Paragraph::new("body", "For Wilhelmina Quietname");
    spoken.spoken_only = true;
    let doc = crate::Document {
        format_version: crate::FORMAT_VERSION,
        title: "Test".to_string(),
        profile: crate::ChurchProfile::default(),
        fields: Default::default(),
        styles: crate::default_styles(),
        stories: vec![crate::Story {
            id: "notes".to_string(),
            keep_through: None,
            paragraphs: vec![Paragraph::new("body", "For the food bank crew"), spoken],
        }],
        panels: vec![crate::Panel {
            id: "p1".to_string(),
            name: "Panel".to_string(),
            size: crate::Size::new(396_000, 612_000),
            frames: vec![crate::Frame {
                id: "f1".to_string(),
                name: "Notes".to_string(),
                kind: crate::FrameKind::Text,
                rect: crate::Rect::new(0, 0, 300_000, 500_000),
                story_id: Some("notes".to_string()),
                role: crate::ContentRole::Weekly,
                picture: None,
                music: None,
            }],
            background_image: None,
            guides: Vec::new(),
        }],
        sheet: crate::SheetSpec {
            preset: "letter-bifold".to_string(),
            size: crate::Size::new(792_000, 612_000),
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
    let (md, left_out) = to_markdown_report(&doc);
    assert_eq!(left_out, 1);
    assert!(
        md.contains("food bank crew"),
        "the printable line stays: {md}"
    );
    assert!(
        !md.contains("Wilhelmina"),
        "the spoken name stays out: {md}"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inline_bold_and_italic_become_runs() {
        let runs = inline_runs("Sing **hymn** 432 and *psalm* 23");
        assert_eq!(
            runs,
            vec![
                Run::plain("Sing "),
                Run {
                    text: "hymn".into(),
                    bold: Some(true),
                    italic: None,
                },
                Run::plain(" 432 and "),
                Run {
                    text: "psalm".into(),
                    bold: None,
                    italic: Some(true),
                },
                Run::plain(" 23"),
            ]
        );
        // Underscore markers speak too.
        let runs = inline_runs("__hymn__ and _psalm_");
        assert_eq!(runs[0].bold, Some(true));
        assert_eq!(runs[0].text, "hymn");
        assert!(runs
            .iter()
            .any(|r| r.text == "psalm" && r.italic == Some(true)));
    }

    #[test]
    fn unbalanced_and_intraword_markers_stay_literal() {
        assert_eq!(
            inline_runs("Sing **hymn 432"),
            vec![Run::plain("Sing **hymn 432")]
        );
        assert_eq!(
            inline_runs("snake_case_word"),
            vec![Run::plain("snake_case_word")]
        );
        assert_eq!(
            inline_runs("2*3*4 people"),
            vec![Run::plain("2*3*4 people")]
        );
        assert_eq!(
            inline_runs("a * lone star"),
            vec![Run::plain("a * lone star")]
        );
    }

    #[test]
    fn unmatched_openers_by_the_thousand_still_finish() {
        // Audit Batch 1: `**a ` repeated never finished
        // (each opener rescanned the rest of the line).
        // 50,000 repeats is the scale that made the old
        // code effectively hang; it must now parse at once.
        // No closer ever qualifies (every later marker
        // follows a space), so the words come back plain.
        let line = "**a ".repeat(50_000);
        let runs = inline_runs(&line);
        assert_eq!(runs, vec![Run::plain(&line)]);
        let paras = markdown_to_paragraphs(&line, &default_styles());
        assert_eq!(paras.len(), 1);
        assert_eq!(paras[0].text(), line.trim());
    }

    #[test]
    fn italic_nests_inside_bold() {
        let runs = inline_runs("**Sing *loudly* now**");
        assert!(runs.iter().all(|r| r.bold == Some(true)));
        assert!(runs
            .iter()
            .any(|r| r.text == "loudly" && r.italic == Some(true)));
        let text: String = runs.iter().map(|r| r.text.as_str()).collect();
        assert_eq!(text, "Sing loudly now");
    }

    #[test]
    fn blocks_map_headings_lists_and_wraps() {
        let styles = default_styles();
        let md = "# Welcome\n\n## News\n\nFirst line\nsecond line\n\n- One\n- Two **bold**\n";
        let paras = markdown_to_paragraphs(md, &styles);
        assert_eq!(paras.len(), 5);
        assert_eq!(paras[0].style, "heading");
        assert_eq!(paras[0].text(), "Welcome");
        assert_eq!(paras[1].style, "subheading");
        assert_eq!(paras[2].text(), "First line second line");
        assert_eq!(paras[3].text(), "One");
        assert_eq!(paras[4].text(), "Two bold");
        assert!(paras[4]
            .runs
            .iter()
            .any(|r| r.text == "bold" && r.bold == Some(true)));
    }

    #[test]
    fn heading_without_a_heading_style_falls_back_to_bold_body() {
        let styles: Vec<ParagraphStyle> = default_styles()
            .into_iter()
            .filter(|s| s.id == "body")
            .collect();
        let paras = markdown_to_paragraphs("# Welcome\n", &styles);
        assert_eq!(paras[0].style, "body");
        assert!(paras[0].runs.iter().all(|r| r.bold == Some(true)));
    }

    #[test]
    fn markdown_document_has_the_rescue_shape() {
        let doc = document_from_markdown(
            "# Title\n\nSome **words** here.\n",
            ChurchProfile::default(),
            "notes.md",
        );
        assert_eq!(doc.title, "notes.md");
        assert_eq!(doc.panels.len(), 1);
        assert_eq!(doc.stories.len(), 1);
        assert_eq!(doc.stories[0].paragraphs.len(), 2);
        assert_eq!(doc.panels[0].frames[0].story_id.as_deref(), Some("notes"));
    }

    #[test]
    fn export_writes_headings_runs_and_resolved_fields() {
        let mut doc = document_from_markdown(
            "# Welcome\n\nBody text.\n",
            ChurchProfile::default(),
            "x.md",
        );
        doc.profile.name = "Grace Lutheran".into();
        doc.stories[0].paragraphs.push(Paragraph::from_runs(
            "body",
            vec![
                Run::plain("Sing "),
                Run {
                    text: "hymn".into(),
                    bold: Some(true),
                    italic: None,
                },
                Run::plain(" at {Church.name}"),
            ],
        ));
        let md = to_markdown(&doc);
        assert!(md.starts_with("# Welcome"), "{md}");
        assert!(md.contains("Sing **hymn** at Grace Lutheran"), "{md}");
    }
}
