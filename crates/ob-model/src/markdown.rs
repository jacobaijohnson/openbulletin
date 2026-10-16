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
    default_styles, normalize_runs, resolve_runs, ChurchProfile, ContentRole, Document, Frame,
    FrameKind, Panel, Paragraph, ParagraphStyle, Run, SheetSpec, Size, Story, FORMAT_VERSION,
};

/// Can this marker open a formatted span here? The marker
/// must sit at a word edge (start, or after whitespace or
/// punctuation) and be followed by a non-space character.
fn opener_ok(chars: &[char], at: usize, marker_len: usize) -> bool {
    let before_ok = at == 0 || !chars[at - 1].is_alphanumeric();
    let after = chars.get(at + marker_len);
    before_ok && matches!(after, Some(c) if !c.is_whitespace())
}

/// Find the closer for a span opened at `from`: the next
/// marker whose preceding character is not whitespace and
/// which ends at a word edge. The content between must be
/// non-empty.
fn find_closer(chars: &[char], from: usize, marker: char, marker_len: usize) -> Option<usize> {
    let mut j = from;
    while j + marker_len <= chars.len() {
        let is_marker = (0..marker_len).all(|k| chars[j + k] == marker);
        if is_marker && j > from {
            let before_ok = !chars[j - 1].is_whitespace();
            let after_at = j + marker_len;
            let after_ok = after_at >= chars.len() || !chars[after_at].is_alphanumeric();
            if before_ok && after_ok {
                return Some(j);
            }
        }
        j += 1;
    }
    None
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
            let Some(close) = find_closer(chars, content_start, marker, len) else {
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
/// resolves them.
pub fn to_markdown(doc: &Document) -> String {
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
                let resolved = resolve_runs(&para.runs, doc);
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
