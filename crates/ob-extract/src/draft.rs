//! Turn an extraction dump into a draft bulletin (docs/EXTRACT.md,
//! the Opus import rule): a rescue produces a draft the admin
//! reviews, never a silent conversion. Text becomes real stories
//! and frames placed where the words were; pages with no readable
//! text become tracing-guide panels. Fonts are matched by name to
//! free fonts or metric twins, never extracted (docs/FONTS.md).

use crate::{ExtractionDump, SpanTrust};
use ob_model::*;
use std::collections::BTreeMap;

/// How sure the rescue is about one piece of the draft, in the
/// words the review screen uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Badge {
    /// Words and a matching (or metric-twin) font.
    Matched,
    /// Words are here, but something needs the admin's eye.
    CheckThis,
    /// No readable words: the page rides along as a picture.
    PictureOnly,
}

/// One line of the review list.
#[derive(Debug, Clone)]
pub struct ImportItem {
    pub panel: usize,
    pub label: String,
    pub badge: Badge,
    pub detail: String,
}

pub struct DraftOutcome {
    pub doc: Document,
    pub items: Vec<ImportItem>,
}

fn pt(v: f32) -> Mpt {
    (v * 1000.0).round() as Mpt
}

/// The free font a source font name maps to, when one is known:
/// the font itself when it is a free family we ship or stock,
/// else its metric twin (same letter widths, docs/FONTS.md).
pub fn font_match(source_name: &str) -> Option<String> {
    let clean = source_name
        .split('+')
        .next_back()
        .unwrap_or(source_name)
        .trim()
        .to_lowercase();
    let base = clean
        .split(['-', ','])
        .next()
        .unwrap_or("")
        .trim()
        .to_string();
    let table: &[(&str, &str)] = &[
        ("arial", "Liberation Sans"),
        ("helvetica", "Liberation Sans"),
        ("liberation sans", "Liberation Sans"),
        ("times new roman", "Liberation Serif"),
        ("timesnewroman", "Liberation Serif"),
        ("liberation serif", "Liberation Serif"),
        ("calibri", "Carlito"),
        ("carlito", "Carlito"),
        ("cambria", "Caladea"),
        ("caladea", "Caladea"),
        ("georgia", "Gelasio"),
        ("gelasio", "Gelasio"),
        ("lora", "Lora"),
        ("playfair display", "Playfair Display"),
        ("garamond", "EB Garamond"),
        ("book antiqua", "Gelasio"),
        ("bookman old style", "Bookman URW"),
        ("century schoolbook", "Century Schoolbook URW"),
    ];
    for (from, to) in table {
        if base == *from || clean.starts_with(from) {
            return Some(to.to_string());
        }
    }
    None
}

/// One text fragment: x, width, text, font size, bold, font.
type Fragment = (f32, f32, String, f32, bool, Option<String>);

struct Line {
    y: f32,
    h: f32,
    /// x, width, text, font size, bold, font name: one fragment.
    parts: Vec<Fragment>,
    /// Baseline wander inside the line: large on text set along
    /// a curve or in steps, which no reading order fully heals.
    spread: f32,
}

impl Line {
    fn x0(&self) -> f32 {
        self.parts.iter().map(|p| p.0).fold(f32::MAX, f32::min)
    }
    fn x1(&self) -> f32 {
        self.parts.iter().map(|p| p.0 + p.1).fold(0.0, f32::max)
    }
    fn size(&self) -> f32 {
        self.parts.iter().map(|p| p.3).fold(0.0, f32::max)
    }
    /// Join fragments left to right using their REAL widths
    /// (the rule the corpus consensus work proved: estimate
    /// nothing, trim each fragment, a space appears only when
    /// the physical gap earns one).
    fn text(&self) -> String {
        let mut parts: Vec<&Fragment> = self.parts.iter().collect();
        parts.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut out = String::new();
        let mut prev_end: Option<f32> = None;
        for (x, w, text, size, _, _) in parts {
            // Some producers draw the same words twice (a fill
            // pass and an outline pass, a point or two apart):
            // a fragment that starts deep inside the previous
            // one is the second drawing, not new words.
            if let Some(end) = prev_end {
                if *x < end - size * 0.1 {
                    continue;
                }
                if x - end > (size * 0.06).max(0.15) && !out.is_empty() {
                    out.push(' ');
                }
            }
            out.push_str(text.trim());
            prev_end = Some(match prev_end { Some(e) => e.max(x + w), None => x + w });
        }
        out.trim().to_string()
    }
    fn font(&self) -> Option<String> {
        self.parts.first().and_then(|p| p.5.clone())
    }
    fn bold(&self) -> bool {
        self.parts.iter().any(|p| p.4)
    }
}

/// Group a page's spans into visual lines (top to bottom).
fn lines_of(spans: &[&crate::Span]) -> Vec<Line> {
    let mut spans: Vec<&crate::Span> = spans
        .iter()
        .filter(|s| s.x.is_some() && s.y.is_some() && !s.text.trim().is_empty())
        .copied()
        .collect();
    spans.sort_by(|a, b| {
        a.y.unwrap_or(0.0)
            .total_cmp(&b.y.unwrap_or(0.0))
            .then(a.x.unwrap_or(0.0).total_cmp(&b.x.unwrap_or(0.0)))
    });
    let mut lines: Vec<Line> = Vec::new();
    for sp in spans {
        let y = sp.y.unwrap_or(0.0);
        let size = sp.font_size.unwrap_or(11.0);
        let joins = lines.last().map(|l: &Line| {
            let tol = (l.size() * 0.4).max(2.0);
            (y - l.y).abs() <= tol
        });
        let part = (
            sp.x.unwrap_or(0.0),
            sp.w.unwrap_or(size * 2.0),
            sp.text.replace(['\r', '\n'], ""),
            size,
            sp.bold.unwrap_or(false),
            sp.font.clone(),
        );
        if joins == Some(true) {
            let line = lines.last_mut().expect("line");
            line.spread = line.spread.max((y - line.y).abs());
            line.parts.push(part);
        } else {
            lines.push(Line {
                y,
                h: sp.h.unwrap_or(size * 1.2),
                parts: vec![part],
                spread: 0.0,
            });
        }
    }
    lines
}

/// One page's words, rebuilt into reading lines (the analyzer
/// reads these, never raw foreign files).
pub fn rebuilt_page_lines(dump: &ExtractionDump, page: u32) -> Vec<String> {
    let spans: Vec<&crate::Span> = dump
        .spans
        .iter()
        .filter(|s| s.page == page && s.trust == SpanTrust::RawText)
        .collect();
    lines_of(&spans)
        .iter()
        .map(|l| l.text())
        .filter(|t| !t.is_empty())
        .collect()
}

/// Build the draft. `with_backgrounds` names each picture page's
/// tracing image the way the renderers write it
/// (assets/page-NN.png), so the caller only renders the pages.
pub fn document_from_dump(
    dump: &ExtractionDump,
    profile: ChurchProfile,
    with_backgrounds: bool,
) -> DraftOutcome {
    let mut items: Vec<ImportItem> = Vec::new();
    let mut styles: Vec<ParagraphStyle> = default_styles();
    let mut style_ids: BTreeMap<String, String> = BTreeMap::new();
    let mut stories: Vec<Story> = Vec::new();
    let mut panels: Vec<Panel> = Vec::new();

    // Preset: the booklet rule the corpus taught us; otherwise
    // one page per sheet, which always prints safely.
    let first = dump.pages.first();
    let (w_pt, h_pt) = first.map(|p| (p.width_pt, p.height_pt)).unwrap_or((612.0, 792.0));
    let all_same = dump
        .pages
        .iter()
        .all(|p| (p.width_pt - w_pt).abs() < 2.0 && (p.height_pt - h_pt).abs() < 2.0);
    let booklet = all_same
        && dump.pages.len() % 4 == 0
        && !dump.pages.is_empty()
        && (w_pt - 396.0).abs() < 4.0
        && (h_pt - 612.0).abs() < 4.0;
    let (preset, sheet) = if booklet {
        (
            "saddle-booklet".to_string(),
            Size::new(792_000, 612_000),
        )
    } else {
        (
            "letter-portrait".to_string(),
            Size::new(pt(w_pt), pt(h_pt)),
        )
    };

    for page in &dump.pages {
        let pi = page.index as usize;
        let spans: Vec<&crate::Span> = dump
            .spans
            .iter()
            .filter(|s| s.page == page.index && s.trust == SpanTrust::RawText)
            .collect();
        let mut panel = Panel {
            id: format!("page-{}", page.index + 1),
            name: format!("Page {}", page.index + 1),
            size: Size::new(pt(page.width_pt), pt(page.height_pt)),
            frames: Vec::new(),
            background_image: None,
            guides: Vec::new(),
        };
        // Every panel keeps its old page as a tracing picture
        // (the review promise: check the rebuild against the
        // original, on the page). Panels drop it as they are
        // cleaned up; it never prints.
        if with_backgrounds {
            panel.background_image = Some(format!("assets/page-{:02}.png", page.index + 1));
        }
        if spans.is_empty() || page.picture_only {
            items.push(ImportItem {
                panel: pi,
                label: format!("Page {} is a picture", page.index + 1),
                badge: Badge::PictureOnly,
                detail:
                    "No readable words here. The page sits under the new one as a tracing guide."
                        .into(),
            });
            panels.push(panel);
            continue;
        }
        // Blocks: lines separated by a real gap start a new block.
        let lines = lines_of(&spans);
        let mut blocks: Vec<Vec<Line>> = Vec::new();
        for line in lines {
            let split = blocks.last().map(|b: &Vec<Line>| {
                let prev = b.last().expect("block line");
                line.y - (prev.y + prev.h) > prev.h * 0.75
            });
            if split == Some(true) || blocks.is_empty() {
                blocks.push(vec![line]);
            } else {
                blocks.last_mut().expect("block").push(line);
            }
        }
        for (bi, block) in blocks.iter().enumerate() {
            // Paragraphs: a line that nearly fills the block's
            // width wrapped, so it joins the next line.
            let maxw = block.iter().map(|l| l.x1() - l.x0()).fold(0.0, f32::max);
            let mut paras: Vec<String> = Vec::new();
            for line in block {
                let t = line.text();
                if t.is_empty() {
                    continue;
                }
                let wrapped = line.x1() - line.x0() > maxw * 0.85;
                if wrapped {
                    if let Some(last) = paras.last_mut() {
                        if !last.is_empty() {
                            last.push(' ');
                            last.push_str(&t);
                            continue;
                        }
                    }
                }
                paras.push(t);
            }
            if paras.is_empty() {
                continue;
            }
            // One style per (font, size, bold, italic) on demand.
            let rep = &block[0];
            let src_font = rep.font().unwrap_or_default();
            let italic = src_font.to_lowercase().contains("italic")
                || src_font.to_lowercase().contains("oblique");
            let size_pt = rep.size();
            let mapped = font_match(&src_font);
            let family = mapped
                .clone()
                .unwrap_or_else(|| "Liberation Sans".to_string());
            let key = format!("{family}|{size_pt:.0}|{}|{italic}", rep.bold());
            let style_id = style_ids.entry(key).or_insert_with(|| {
                let id = format!("import-{}", styles.len());
                styles.push(ParagraphStyle {
                    id: id.clone(),
                    name: format!("{family} {:.0}pt", size_pt),
                    font_size: pt(size_pt),
                    bold: rep.bold(),
                    italic,
                    align: Align::Left,
                    space_after: 4_000,
                    color: Rgb::BLACK,
                    font_family: family.clone(),
                    font_sha256: None,
                });
                id
            });
            let sid = format!("rescue-{pi}-{bi}");
            stories.push(Story {
                id: sid.clone(),
                paragraphs: paras
                    .iter()
                    .map(|t| Paragraph {
                        style: style_id.clone(),
                        text: t.clone(),
                    })
                    .collect(),
            });
            let x0 = block.iter().map(|l| l.x0()).fold(f32::MAX, f32::min);
            let y0 = block.iter().map(|l| l.y).fold(f32::MAX, f32::min);
            let x1 = block.iter().map(|l| l.x1()).fold(0.0, f32::max);
            let y1 = block
                .iter()
                .map(|l| l.y + l.h)
                .fold(0.0, f32::max);
            panel.frames.push(Frame {
                id: format!("rescue-frame-{pi}-{bi}"),
                name: format!("Words from page {}", page.index + 1),
                kind: FrameKind::Text,
                rect: Rect {
                    x: pt(x0),
                    y: pt(y0),
                    w: pt(x1 - x0).max(10_000),
                    h: pt((y1 - y0) * 1.1).max(10_000),
                },
                story_id: Some(sid),
                role: ContentRole::Weekly,
            });
            // Words set along a curve or in steps in the old
            // file cannot be read back in a certain order: flag
            // the block even when its font matched.
            let jittery = block
                .iter()
                .any(|l| l.spread > l.size() * 0.6);
            let first_line: String = paras[0].chars().take(38).collect();
            items.push(ImportItem {
                panel: pi,
                label: first_line,
                badge: if mapped.is_some() && !jittery {
                    Badge::Matched
                } else {
                    Badge::CheckThis
                },
                detail: if jittery {
                    format!("Words from page {}; they were set along a curve or in steps in the old file, so their order may be off. The old page sits underneath as a picture to check against.", page.index + 1)
                } else if mapped.is_some() {
                    format!("Words from page {}, in a font we have or a twin of {src_font}.", page.index + 1)
                } else if src_font.is_empty() {
                    format!("Words from page {}; the file did not name their font, so the look may differ.", page.index + 1)
                } else {
                    format!("Words from page {}; we do not have {src_font}, so the look may differ until a font is added.", page.index + 1)
                },
            });
        }
        panels.push(panel);
    }

    DraftOutcome {
        doc: Document {
            format_version: FORMAT_VERSION,
            title: dump.source_name.clone(),
            profile,
            fields: Default::default(),
            styles,
            stories,
            panels,
            sheet: SheetSpec {
                preset,
                size: sheet,
            },
        },
        items,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PageInfo, SourceKind, Span};

    fn span(page: u32, text: &str, x: f32, y: f32, size: f32, font: &str, bold: bool) -> Span {
        Span {
            page,
            text: text.into(),
            trust: SpanTrust::RawText,
            x: Some(x),
            y: Some(y),
            w: Some(100.0),
            h: Some(size * 1.2),
            font_size: Some(size),
            bold: Some(bold),
            font: Some(font.into()),
        }
    }

    #[test]
    fn rescue_builds_frames_styles_and_badges() {
        let dump = ExtractionDump {
            schema_version: 1,
            source_kind: SourceKind::Pdf,
            source_name: "old.pdf".into(),
            source_bytes: 10,
            extractor: "test".into(),
            pages: vec![
                PageInfo { index: 0, width_pt: 396.0, height_pt: 612.0, picture_only: false },
                PageInfo { index: 1, width_pt: 396.0, height_pt: 612.0, picture_only: false },
                PageInfo { index: 2, width_pt: 396.0, height_pt: 612.0, picture_only: false },
                PageInfo { index: 3, width_pt: 396.0, height_pt: 612.0, picture_only: true },
            ],
            spans: vec![
                span(0, "Grace Lutheran", 60.0, 40.0, 20.0, "ABCDEF+Arial-Bold", true),
                span(0, "Welcome to worship", 60.0, 90.0, 11.0, "Calibri", false),
                span(0, "this morning and", 60.0, 104.0, 11.0, "Calibri", false),
                span(0, "bring a friend", 60.0, 118.0, 11.0, "Calibri", false),
                span(1, "Mystery words", 60.0, 40.0, 11.0, "GillSansMT", false),
            ],
        };
        let out = document_from_dump(&dump, ChurchProfile::default(), true);
        assert_eq!(out.doc.sheet.preset, "saddle-booklet");
        assert_eq!(out.doc.panels.len(), 4);
        // Page 1: a heading block and a body block.
        assert_eq!(out.doc.panels[0].frames.len(), 2);
        // Arial maps to its twin; Calibri to Carlito; GillSans is unknown.
        assert!(out.items.iter().any(|i| i.badge == Badge::Matched && i.label.contains("Grace")));
        assert!(out.items.iter().any(|i| i.badge == Badge::CheckThis && i.label.contains("Mystery")));
        assert!(out.items.iter().any(|i| i.badge == Badge::PictureOnly));
        assert_eq!(
            out.doc.panels[3].background_image.as_deref(),
            Some("assets/page-04.png")
        );
        // The Arial style names the twin family.
        assert!(out
            .doc
            .styles
            .iter()
            .any(|s| s.font_family == "Liberation Sans" && s.font_size == 20_000));
    }

    #[test]
    fn font_twins_are_matched() {
        assert_eq!(font_match("ABCDEF+ArialMT").as_deref(), Some("Liberation Sans"));
        assert_eq!(font_match("TimesNewRomanPSMT").as_deref(), Some("Liberation Serif"));
        assert_eq!(font_match("Calibri").as_deref(), Some("Carlito"));
        assert_eq!(font_match("GillSansMT"), None);
    }
}
