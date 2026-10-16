//! Turn an extraction dump into a draft bulletin (docs/EXTRACT.md,
//! the Opus import rule): a rescue produces a draft the admin
//! reviews, never a silent conversion. Text becomes real stories
//! and frames placed where the words were; pages with no readable
//! text become tracing-guide panels. Fonts are matched by name to
//! free fonts or metric twins, never extracted (docs/FONTS.md).

use crate::{ExtractionDump, SpanTrust};
use ob_model::*;
use std::collections::{BTreeMap, HashMap};

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

/// A page dimension from a dump that a document can hold:
/// finite, positive, and no larger than 10,000 points to a
/// side (about three and a half metres; no real page).
/// Anything else falls back to US Letter, so a broken dump
/// cannot put an impossible page into the document.
fn sane_page_dim(v: f32, fallback: f32) -> f32 {
    if v.is_finite() && v > 0.0 && v <= 10_000.0 {
        v
    } else {
        fallback
    }
}

/// Whether a span's stated geometry can live in a document
/// (audit Batch 3): every coordinate it states is finite and
/// within ten page lengths of the page, its size no bigger
/// than ten pages, its type size a size type can be. Dumps
/// are data read out of other programs' files; an infinite
/// or absurd coordinate used to flow straight through the
/// pt() cast into a frame (infinity saturates to the
/// largest size the model can state).
fn span_geometry_sane(sp: &crate::Span, pw: f32, ph: f32) -> bool {
    let coord = |v: Option<f32>, bound: f32| match v {
        None => true,
        Some(n) => n.is_finite() && n.abs() <= bound,
    };
    let extent = |v: Option<f32>, bound: f32| match v {
        None => true,
        Some(n) => n.is_finite() && n >= 0.0 && n <= bound,
    };
    coord(sp.x, 10.0 * pw)
        && coord(sp.y, 10.0 * ph)
        && extent(sp.w, 10.0 * pw)
        && extent(sp.h, 10.0 * ph)
        && match sp.font_size {
            None => true,
            Some(n) => n.is_finite() && n > 0.0 && n <= 1_000.0,
        }
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

/// One text fragment: x, width, text, font size, bold,
/// font name, italic, color, stated alignment, and the
/// span's inline runs (G1b; empty when the source states
/// one face for the whole span).
type Fragment = (
    f32,
    f32,
    String,
    f32,
    bool,
    Option<String>,
    bool,
    [u8; 3],
    Option<Align>,
    Vec<crate::SpanRun>,
);

struct Line {
    y: f32,
    h: f32,
    /// x, width, text, font size, bold, font name, italic,
    /// color, stated alignment, runs: one fragment.
    parts: Vec<Fragment>,
    /// Baseline wander inside the line: large on text set along
    /// a curve or in steps, which no reading order fully heals.
    spread: f32,
    /// Any fragment came from a recovery that cannot vouch for
    /// the words (SpanTrust::NeedsReview): the block is badged
    /// Check this no matter how well its font matched.
    review: bool,
    /// The thread this line's words belong to, when the
    /// source named one (T1: the .pub door stamps it). The
    /// first fragment of the line decides; fragments the
    /// .pub pen emits for one box all carry the same name.
    thread: Option<String>,
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
        for (x, w, text, size, _, _, _, _, _, _) in parts {
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
            prev_end = Some(match prev_end {
                Some(e) => e.max(x + w),
                None => x + w,
            });
        }
        out.trim().to_string()
    }
    /// This line's words as model runs (G1b), when every
    /// fragment carries runs that concatenate to its text:
    /// the same left-to-right walk `text()` makes (overlap
    /// skips honored, gap spaces earned the same way), each
    /// piece's flags made effective against its fragment's
    /// stated face. None when any fragment lacks runs or
    /// disagrees with its fragment's text.
    fn runs(&self) -> Option<Vec<ob_model::Run>> {
        self.runs_walk(true)
    }

    /// Runs built from the fragments themselves: one run
    /// per fragment, trimmed, in the fragment's own face.
    /// The fallback for lines whose spans carry no inline
    /// runs (PDF and .pub spans state a face per span, not
    /// per word): a bold or italic span inside the line
    /// still lands as a bold or italic run of the paragraph
    /// instead of vanishing into the block's one style.
    fn part_runs(&self) -> Vec<ob_model::Run> {
        self.runs_walk(false).unwrap_or_default()
    }

    /// The walk behind `runs` and `part_runs`. With
    /// `span_runs`, a fragment contributes its inline runs
    /// (and the walk fails, None, when a fragment has none
    /// or they disagree with its text); without, each
    /// fragment contributes one run in its own face.
    fn runs_walk(&self, span_runs: bool) -> Option<Vec<ob_model::Run>> {
        let mut parts: Vec<&Fragment> = self.parts.iter().collect();
        parts.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut out: Vec<ob_model::Run> = Vec::new();
        let mut prev_end: Option<f32> = None;
        for (x, w, text, size, bold, font, italic, _, _, runs) in parts {
            let mut frag: Vec<ob_model::Run> = if span_runs {
                if runs.is_empty() {
                    return None;
                }
                let concat: String = runs.iter().map(|r| r.text.as_str()).collect();
                if concat != *text {
                    return None;
                }
                runs.iter()
                    .map(|r| ob_model::Run {
                        text: r.text.clone(),
                        bold: Some(r.bold.unwrap_or(*bold)),
                        italic: Some(r.italic.unwrap_or(*italic)),
                    })
                    .collect()
            } else {
                // The fragment's own face, with the v1
                // name sniff for italic that Line::italic
                // applies line-wide.
                let fname = font.clone().unwrap_or_default().to_lowercase();
                let frag_italic = *italic || fname.contains("italic") || fname.contains("oblique");
                vec![ob_model::Run {
                    text: text.clone(),
                    bold: Some(*bold),
                    italic: Some(frag_italic),
                }]
            };
            if let Some(end) = prev_end {
                if *x < end - size * 0.1 {
                    continue; // the second drawing, as in text()
                }
                if x - end > (size * 0.06).max(0.15) && !out.is_empty() {
                    if let Some(last) = out.last_mut() {
                        last.text.push(' ');
                    }
                }
            }
            trim_runs(&mut frag);
            out.extend(frag);
            prev_end = Some(match prev_end {
                Some(e) => e.max(x + w),
                None => x + w,
            });
        }
        Some(ob_model::normalize_runs(out))
    }
    fn font(&self) -> Option<String> {
        self.parts.first().and_then(|p| p.5.clone())
    }
    fn bold(&self) -> bool {
        self.parts.iter().any(|p| p.4)
    }
    /// Italic: the span flag is authoritative (schema v2
    /// producers read the font's own flags); the old
    /// name sniff remains only as the v1 fallback, where
    /// no flag existed. The matched FAMILY still comes
    /// from the plain font name, so an italic line in
    /// Calibri matches Carlito and gets the italic face of
    /// it, never a name with "-Italic" glued on.
    fn italic(&self) -> bool {
        if self.parts.iter().any(|p| p.6) {
            return true;
        }
        let f = self.font().unwrap_or_default().to_lowercase();
        f.contains("italic") || f.contains("oblique")
    }
    /// This line's stated alignment: the one most of its
    /// parts state, earliest first on ties. None when no
    /// part states one.
    fn align(&self) -> Option<Align> {
        majority(self.parts.iter().filter_map(|p| p.8))
    }
    /// This line's color: the one most of its parts carry.
    fn color(&self) -> [u8; 3] {
        majority(self.parts.iter().map(|p| p.7)).unwrap_or([0, 0, 0])
    }
}

/// Trim whitespace off the ends of a run list, as
/// `str::trim` would trim its concatenation.
fn trim_runs(runs: &mut [ob_model::Run]) {
    let full: String = runs.iter().map(|r| r.text.as_str()).collect();
    let start = full.len() - full.trim_start().len();
    let end = full.trim_end().len();
    if start >= end {
        for r in runs.iter_mut() {
            r.text.clear();
        }
        return;
    }
    let mut at = 0usize;
    for r in runs.iter_mut() {
        let len = r.text.len();
        let s = start.max(at);
        let e = end.min(at + len);
        r.text = if s < e {
            r.text[s - at..e - at].to_string()
        } else {
            String::new()
        };
        at += len;
    }
}

/// The most common value in an iterator, earliest first on
/// ties. None only for an empty iterator.
fn majority<T: Copy + PartialEq>(values: impl Iterator<Item = T>) -> Option<T> {
    let mut counts: Vec<(T, usize)> = Vec::new();
    for v in values {
        match counts.iter_mut().find(|(t, _)| *t == v) {
            Some((_, n)) => *n += 1,
            None => counts.push((v, 1)),
        }
    }
    counts.into_iter().max_by_key(|(_, n)| *n).map(|(t, _)| t)
}

/// A block's paragraph alignment: the alignment most of
/// its lines state. None when no line states one (the
/// draft falls back to Left, as before schema v2).
fn block_align(block: &[Line]) -> Option<Align> {
    majority(block.iter().filter_map(|l| l.align()))
}

/// A block's paragraph color: the color most of its lines
/// carry. Black only when the dump itself says black.
fn block_color(block: &[Line]) -> [u8; 3] {
    majority(block.iter().map(|l| l.color())).unwrap_or([0, 0, 0])
}

/// The thread a whole block belongs to (T1): the name when
/// every line carries the same one. Mixed or unnamed lines
/// mean no thread: the block stands alone, as blocks from
/// every other door always have.
fn block_thread(block: &[Line]) -> Option<String> {
    let first = block.first()?.thread.clone()?;
    if block
        .iter()
        .all(|l| l.thread.as_deref() == Some(first.as_str()))
    {
        Some(first)
    } else {
        None
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
            sp.italic,
            sp.color,
            sp.align,
            sp.runs.clone(),
        );
        if joins == Some(true) {
            let line = lines.last_mut().expect("line");
            line.spread = line.spread.max((y - line.y).abs());
            line.review |= sp.trust == SpanTrust::NeedsReview;
            line.parts.push(part);
        } else {
            lines.push(Line {
                y,
                h: sp.h.unwrap_or(size * 1.2),
                parts: vec![part],
                spread: 0.0,
                review: sp.trust == SpanTrust::NeedsReview,
                thread: sp.thread.clone(),
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
    build_draft(dump, profile, with_backgrounds, &HashMap::new())
}

/// Build the draft, carrying the dump's page pictures into
/// the document's asset store. `image_bytes` maps each
/// `DumpImage.file` name to its bytes (the caller read the
/// helper's picture files); pictures whose bytes are not
/// supplied stay out, exactly as with `document_from_dump`.
/// Each carried picture is an asset with an honest rights
/// record (source: the imported file, license unknown),
/// and each page with pictures gets one review item saying
/// they arrived. Placing them on the page is the picture
/// tools' job, not the rescue's.
pub fn document_from_dump_with_assets(
    dump: &ExtractionDump,
    profile: ChurchProfile,
    with_backgrounds: bool,
    image_bytes: &HashMap<String, Vec<u8>>,
) -> DraftOutcome {
    build_draft(dump, profile, with_backgrounds, image_bytes)
}

fn build_draft(
    dump: &ExtractionDump,
    profile: ChurchProfile,
    with_backgrounds: bool,
    image_bytes: &HashMap<String, Vec<u8>>,
) -> DraftOutcome {
    let mut items: Vec<ImportItem> = Vec::new();
    // Losses the extractor itself counted while reading
    // (round 2, RT-5: a part too large to read whole, and
    // its kin) belong to no page, so they ride at the head
    // of the list on the first panel, in the extractor's
    // own words.
    for note in &dump.loss_notes {
        items.push(ImportItem {
            panel: 0,
            label: "Part of the old file was left out".to_string(),
            badge: Badge::CheckThis,
            detail: note.clone(),
        });
    }
    // Spans addressed to pages the dump does not list
    // never reach a panel below (round 2, R2-8). Dumps
    // that arrived through from_json already dropped and
    // counted them into loss_notes above; a dump built
    // any other way gets the same loss counted here, in
    // the same shape, so its words never vanish silently.
    {
        let listed: std::collections::HashSet<u32> = dump.pages.iter().map(|p| p.index).collect();
        let orphans = dump
            .spans
            .iter()
            .filter(|s| s.trust != SpanTrust::PictureOnly && !listed.contains(&s.page))
            .count();
        if orphans > 0 {
            items.push(ImportItem {
                panel: 0,
                label: "Part of the old file was left out".to_string(),
                badge: Badge::CheckThis,
                detail: crate::unlisted_page_spans_note(orphans),
            });
        }
    }
    let mut styles: Vec<ParagraphStyle> = default_styles();
    let mut style_ids: BTreeMap<String, String> = BTreeMap::new();
    let mut stories: Vec<Story> = Vec::new();
    let mut panels: Vec<Panel> = Vec::new();
    // Threading (T1): thread name -> the story the thread's
    // first block created, kept across pages, so every box
    // of a thread names ONE story and the words live in the
    // document once.
    let mut thread_stories: HashMap<String, String> = HashMap::new();

    // Preset: the booklet rule the corpus taught us; otherwise
    // one page per sheet, which always prints safely.
    let first = dump.pages.first();
    let (w_pt, h_pt) = first
        .map(|p| {
            (
                sane_page_dim(p.width_pt, 612.0),
                sane_page_dim(p.height_pt, 792.0),
            )
        })
        .unwrap_or((612.0, 792.0));
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
        ("saddle-booklet".to_string(), Size::new(792_000, 612_000))
    } else {
        ("letter-portrait".to_string(), Size::new(pt(w_pt), pt(h_pt)))
    };

    for page in &dump.pages {
        let pi = page.index as usize;
        let spans: Vec<&crate::Span> = dump
            .spans
            .iter()
            .filter(|s| {
                s.page == page.index
                    && matches!(s.trust, SpanTrust::RawText | SpanTrust::NeedsReview)
            })
            .collect();
        // Geometry guard (audit Batch 3): words whose stated
        // place no page can hold are left out here, at the
        // one door into the document, and counted in the
        // review like every other loss — never carried into
        // a frame as an infinite or absurd position.
        let pw = sane_page_dim(page.width_pt, 612.0);
        let ph = sane_page_dim(page.height_pt, 792.0);
        let mut kept: Vec<&crate::Span> = Vec::with_capacity(spans.len());
        let mut dropped = 0usize;
        for sp in spans {
            if span_geometry_sane(sp, pw, ph) {
                kept.push(sp);
            } else {
                dropped += 1;
            }
        }
        let spans = kept;
        if dropped > 0 {
            items.push(ImportItem {
                panel: pi,
                label: format!("Words left out on page {}", page.index + 1),
                badge: Badge::CheckThis,
                detail: format!(
                    "{dropped} piece{} of words on page {} sat at a place no page can hold in the old file, so they were left out of this draft. The old page sits underneath as a picture to check against.",
                    if dropped == 1 { "" } else { "s" },
                    page.index + 1
                ),
            });
        }
        let mut panel = Panel {
            id: format!("page-{}", page.index + 1),
            name: format!("Page {}", page.index + 1),
            size: Size::new(pt(pw), pt(ph)),
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
            // Paragraphs accumulate as RUNS (G1b): a line whose
            // spans carry inline formatting keeps it, with the
            // flags still effective (the block style is chosen
            // below; flags are relativized against it when the
            // story is written). A line without runs becomes
            // plain runs with the LINE's own face, so a line
            // more italic than its block lands as an italic
            // run of the block's paragraph, not a new style.
            let maxw = block.iter().map(|l| l.x1() - l.x0()).fold(0.0, f32::max);
            let mut paras: Vec<Vec<ob_model::Run>> = Vec::new();
            for line in block {
                let t = line.text();
                if t.is_empty() {
                    continue;
                }
                let line_runs = line.runs().unwrap_or_else(|| line.part_runs());
                let wrapped = line.x1() - line.x0() > maxw * 0.85;
                if wrapped {
                    if let Some(last) = paras.last_mut() {
                        if !last.is_empty() {
                            if let Some(tail) = last.last_mut() {
                                tail.text.push(' ');
                            }
                            last.extend(line_runs);
                            continue;
                        }
                    }
                }
                paras.push(line_runs);
            }
            if paras.is_empty() {
                continue;
            }
            // One style per look on demand. Alignment and
            // color come from the block's spans (schema v2),
            // no longer hardcoded: the alignment most of
            // the block's lines state, the color most of
            // them carry; a dump that states neither (v1)
            // lands on Left and black exactly as before.
            let rep = &block[0];
            let src_font = rep.font().unwrap_or_default();
            let italic = rep.italic();
            let size_pt = rep.size();
            let mapped = font_match(&src_font);
            let family = mapped
                .clone()
                .unwrap_or_else(|| "Liberation Sans".to_string());
            let block_align_v = block_align(block);
            let block_color_v = block_color(block);
            let key = format!(
                "{family}|{size_pt:.0}|{}|{italic}|{block_align_v:?}|{},{},{}",
                rep.bold(),
                block_color_v[0],
                block_color_v[1],
                block_color_v[2]
            );
            let style_id = style_ids.entry(key).or_insert_with(|| {
                let id = format!("import-{}", styles.len());
                styles.push(ParagraphStyle {
                    id: id.clone(),
                    name: format!("{family} {:.0}pt", size_pt),
                    font_size: pt(size_pt),
                    bold: rep.bold(),
                    italic,
                    align: block_align_v.unwrap_or(Align::Left),
                    space_after: 4_000,
                    color: Rgb::new(block_color_v[0], block_color_v[1], block_color_v[2]),
                    font_family: family.clone(),
                    font_sha256: None,
                });
                id
            });
            // Relativize the runs against the block style
            // (G1b): a flag the style already carries becomes
            // None (inherit), a differing one stays explicit.
            let style = styles
                .iter()
                .find(|s| s.id == *style_id)
                .expect("the style was just made");
            let (sb, si) = (style.bold, style.italic);
            let relativize = |runs: &Vec<ob_model::Run>| -> Vec<ob_model::Run> {
                runs.iter()
                    .map(|r| ob_model::Run {
                        text: r.text.clone(),
                        bold: match r.bold {
                            Some(f) if f == sb => None,
                            other => other,
                        },
                        italic: match r.italic {
                            Some(f) if f == si => None,
                            other => other,
                        },
                    })
                    .collect()
            };
            let new_paras: Vec<Paragraph> = paras
                .iter()
                .map(|runs| Paragraph::from_runs(style_id, relativize(runs)))
                .collect();
            // A block in a thread joins the thread's story:
            // the first block of the thread creates it (under
            // its own usual id), later blocks add their words
            // to it in reading order, and every block's frame
            // names the same story (T1).
            let sid = match block_thread(block) {
                Some(name) => match thread_stories.get(&name) {
                    Some(existing) => existing.clone(),
                    None => {
                        let sid = format!("rescue-{pi}-{bi}");
                        thread_stories.insert(name, sid.clone());
                        sid
                    }
                },
                None => format!("rescue-{pi}-{bi}"),
            };
            match stories.iter_mut().find(|s| s.id == sid) {
                Some(existing) => existing.paragraphs.extend(new_paras),
                None => stories.push(Story {
                    keep_through: None,
                    id: sid.clone(),
                    paragraphs: new_paras,
                }),
            }
            let x0 = block.iter().map(|l| l.x0()).fold(f32::MAX, f32::min);
            let y0 = block.iter().map(|l| l.y).fold(f32::MAX, f32::min);
            let x1 = block.iter().map(|l| l.x1()).fold(0.0, f32::max);
            let y1 = block.iter().map(|l| l.y + l.h).fold(0.0, f32::max);
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
                picture: None,
                music: None,
            });
            // Words set along a curve or in steps in the old
            // file cannot be read back in a certain order: flag
            // the block even when its font matched. Words from
            // a recovery that cannot vouch for them (the .pub
            // text rescue) are flagged the same way.
            let jittery = block.iter().any(|l| l.spread > l.size() * 0.6);
            let unvouched = block.iter().any(|l| l.review);
            let first_text: String = paras[0].iter().map(|r| r.text.as_str()).collect();
            let first_line: String = first_text.chars().take(38).collect();
            items.push(ImportItem {
                panel: pi,
                label: first_line,
                badge: if mapped.is_some() && !jittery && !unvouched {
                    Badge::Matched
                } else {
                    Badge::CheckThis
                },
                detail: if unvouched {
                    format!("Words from page {}; they came over as words only, without their places on the page, so check them against the original file.", page.index + 1)
                } else if jittery {
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

    let mut doc = Document {
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
        no_announcements: false,
        assets: Vec::new(),
        asset_bytes: Default::default(),
        occasion_time: None,
    };
    // The picture channel: bytes the caller supplied ride
    // into the asset store, one review item per page that
    // had pictures, in panel order.
    let mut picture_items: Vec<ImportItem> = Vec::new();
    for img in &dump.images {
        let Some(bytes) = image_bytes.get(&img.file) else {
            continue;
        };
        doc.add_asset(
            bytes,
            AssetKind::Picture,
            "image/png",
            RightsRecord {
                source: dump.source_name.clone(),
                ..Default::default()
            },
        );
        let panel = img.page as usize;
        if !picture_items.iter().any(|i| i.panel == panel) {
            picture_items.push(ImportItem {
                panel,
                label: format!("Pictures from page {}", img.page + 1),
                badge: Badge::CheckThis,
                detail: "Pictures from the old file were brought into this bulletin. They are kept with it; placing them on the page comes with the picture tools.".into(),
            });
        }
    }
    if !picture_items.is_empty() {
        items.extend(picture_items);
        // Stable: the review list stays in page order, with
        // a page's words ahead of its pictures.
        items.sort_by_key(|i| i.panel);
    }
    DraftOutcome { doc, items }
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
            align: None,
            italic: false,
            color: [0, 0, 0],
            runs: Vec::new(),
            thread: None,
        }
    }

    #[test]
    fn spans_for_unlisted_pages_surface_as_a_counted_loss() {
        // Round 2, R2-8, at the draft door: a dump built
        // in code (never read through from_json, so the
        // spans are still present) must still count its
        // unlisted-page words in the review, in the same
        // shape as an extractor loss note, and the words
        // themselves must not appear in the draft.
        let dump = ExtractionDump {
            schema_version: 2,
            source_kind: SourceKind::Pdf,
            source_name: "odd.pdf".into(),
            source_bytes: 10,
            extractor: "test".into(),
            pages: vec![PageInfo {
                index: 0,
                width_pt: 612.0,
                height_pt: 792.0,
                picture_only: false,
            }],
            spans: vec![
                span(0, "Welcome to worship", 60.0, 90.0, 11.0, "Calibri", false),
                span(
                    4,
                    "Words addressed to no listed page",
                    60.0,
                    130.0,
                    11.0,
                    "Calibri",
                    false,
                ),
            ],
            images: Vec::new(),
            loss_notes: Vec::new(),
        };
        let out = document_from_dump(&dump, ChurchProfile::default(), false);
        let losses: Vec<_> = out
            .items
            .iter()
            .filter(|i| i.label == "Part of the old file was left out")
            .collect();
        assert_eq!(losses.len(), 1, "one counted loss: {:?}", out.items);
        assert_eq!(losses[0].badge, Badge::CheckThis);
        assert!(
            losses[0]
                .detail
                .contains("1 piece of text pointed at a page"),
            "the loss is counted in plain words: {}",
            losses[0].detail
        );
        let all_text: String = out
            .doc
            .stories
            .iter()
            .flat_map(|s| s.paragraphs.iter().map(|p| p.text()))
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            !all_text.contains("no listed page"),
            "the orphan words are not in the draft: {all_text}"
        );
        assert!(all_text.contains("Welcome to worship"));
    }

    #[test]
    fn a_line_more_italic_than_its_block_becomes_an_italic_run() {
        // G1b: the block takes its style from its first line
        // (regular); the second line, set italic in the old
        // file, must not force a second style or lose its
        // face: it lands as an italic run of its paragraph.
        let mut italic_span = span(0, "A mighty fortress", 60.0, 104.0, 11.0, "Calibri", false);
        italic_span.italic = true;
        italic_span.w = Some(40.0); // a short line: its own paragraph, not a wrap join
        let dump = ExtractionDump {
            schema_version: 2,
            source_kind: SourceKind::Pdf,
            source_name: "old.pdf".into(),
            source_bytes: 10,
            extractor: "test".into(),
            pages: vec![PageInfo {
                index: 0,
                width_pt: 612.0,
                height_pt: 792.0,
                picture_only: false,
            }],
            spans: vec![
                span(0, "Welcome to worship", 60.0, 90.0, 11.0, "Calibri", false),
                italic_span,
            ],
            images: Vec::new(),
            loss_notes: Vec::new(),
        };
        let out = document_from_dump(&dump, ChurchProfile::default(), false);
        let story = &out.doc.stories[0];
        assert_eq!(story.paragraphs.len(), 2);
        let first = &story.paragraphs[0];
        assert!(
            first.runs.iter().all(|r| r.italic.is_none()),
            "the regular line inherits the regular style: {:?}",
            first.runs
        );
        let second = &story.paragraphs[1];
        assert_eq!(second.text(), "A mighty fortress");
        assert!(
            second
                .runs
                .iter()
                .all(|r| r.italic == Some(true) && !r.text.is_empty()),
            "the italic line is an italic run, not a style: {:?}",
            second.runs
        );
        assert_eq!(
            first.style, second.style,
            "both paragraphs share the block's one style"
        );
    }

    #[test]
    fn impossible_geometry_is_left_out_and_counted() {
        // Audit Batch 3: a dump is data read out of another
        // program's file. Spans at an infinite, NaN, or absurd
        // position used to flow into frames; now they are
        // left out at the draft door and counted in the
        // review, and no impossible number reaches the
        // document.
        let mut inf_span = span(0, "Words at infinity", 60.0, 200.0, 11.0, "Calibri", false);
        inf_span.x = Some(f32::INFINITY);
        let mut absurd_span = span(
            0,
            "Words beyond the page",
            60.0,
            300.0,
            11.0,
            "Calibri",
            false,
        );
        absurd_span.x = Some(1.0e9);
        let mut nan_span = span(
            0,
            "Words with no place",
            60.0,
            400.0,
            11.0,
            "Calibri",
            false,
        );
        nan_span.y = Some(f32::NAN);
        let dump = ExtractionDump {
            schema_version: 2,
            source_kind: SourceKind::Pdf,
            source_name: "old.pdf".into(),
            source_bytes: 10,
            extractor: "test".into(),
            pages: vec![PageInfo {
                index: 0,
                width_pt: 612.0,
                height_pt: 792.0,
                picture_only: false,
            }],
            spans: vec![
                span(0, "Welcome to worship", 60.0, 90.0, 11.0, "Calibri", false),
                inf_span,
                absurd_span,
                nan_span,
            ],
            images: Vec::new(),
            loss_notes: Vec::new(),
        };
        let out = document_from_dump(&dump, ChurchProfile::default(), false);
        // The good words arrived; the impossible ones did not.
        let all_text: String = out
            .doc
            .stories
            .iter()
            .flat_map(|s| s.paragraphs.iter().map(|p| p.text()))
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            all_text.contains("Welcome to worship"),
            "good words kept: {all_text}"
        );
        assert!(
            !all_text.contains("infinity"),
            "inf words left out: {all_text}"
        );
        assert!(
            !all_text.contains("beyond the page"),
            "absurd words left out: {all_text}"
        );
        assert!(
            !all_text.contains("no place"),
            "NaN words left out: {all_text}"
        );
        // No frame carries an impossible number (ten page
        // lengths is the guard's own bound, in mpt).
        let bound = 10 * 792_000;
        for panel in &out.doc.panels {
            assert!(panel.size.w.abs() <= bound && panel.size.h.abs() <= bound);
            for frame in &panel.frames {
                assert!(
                    frame.rect.x.abs() <= bound,
                    "frame x sane: {}",
                    frame.rect.x
                );
                assert!(
                    frame.rect.y.abs() <= bound,
                    "frame y sane: {}",
                    frame.rect.y
                );
                assert!(frame.rect.w >= 0 && frame.rect.w <= bound);
                assert!(frame.rect.h >= 0 && frame.rect.h <= bound);
            }
        }
        // The loss is counted in the review, in plain words.
        let left_out: Vec<_> = out
            .items
            .iter()
            .filter(|i| i.label == "Words left out on page 1")
            .collect();
        assert_eq!(left_out.len(), 1, "one counted loss item: {:?}", out.items);
        assert_eq!(left_out[0].badge, Badge::CheckThis);
        assert!(
            left_out[0].detail.contains("3 pieces"),
            "the count is stated: {}",
            left_out[0].detail
        );
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
                PageInfo {
                    index: 0,
                    width_pt: 396.0,
                    height_pt: 612.0,
                    picture_only: false,
                },
                PageInfo {
                    index: 1,
                    width_pt: 396.0,
                    height_pt: 612.0,
                    picture_only: false,
                },
                PageInfo {
                    index: 2,
                    width_pt: 396.0,
                    height_pt: 612.0,
                    picture_only: false,
                },
                PageInfo {
                    index: 3,
                    width_pt: 396.0,
                    height_pt: 612.0,
                    picture_only: true,
                },
            ],
            spans: vec![
                span(
                    0,
                    "Grace Lutheran",
                    60.0,
                    40.0,
                    20.0,
                    "ABCDEF+Arial-Bold",
                    true,
                ),
                span(0, "Welcome to worship", 60.0, 90.0, 11.0, "Calibri", false),
                span(0, "this morning and", 60.0, 104.0, 11.0, "Calibri", false),
                span(0, "bring a friend", 60.0, 118.0, 11.0, "Calibri", false),
                span(1, "Mystery words", 60.0, 40.0, 11.0, "GillSansMT", false),
            ],
            images: Vec::new(),
            loss_notes: Vec::new(),
        };
        let out = document_from_dump(&dump, ChurchProfile::default(), true);
        assert_eq!(out.doc.sheet.preset, "saddle-booklet");
        assert_eq!(out.doc.panels.len(), 4);
        // Page 1: a heading block and a body block.
        assert_eq!(out.doc.panels[0].frames.len(), 2);
        // Arial maps to its twin; Calibri to Carlito; GillSans is unknown.
        assert!(out
            .items
            .iter()
            .any(|i| i.badge == Badge::Matched && i.label.contains("Grace")));
        assert!(out
            .items
            .iter()
            .any(|i| i.badge == Badge::CheckThis && i.label.contains("Mystery")));
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
    fn unvouched_words_are_check_this_even_with_a_matched_font() {
        // The .pub text rescue: words without places. Arial
        // would normally earn a Matched badge; NeedsReview
        // words are Check this regardless.
        let mut s = span(0, "Grace Lutheran", 60.0, 40.0, 20.0, "Arial", true);
        s.trust = SpanTrust::NeedsReview;
        let dump = ExtractionDump {
            schema_version: 1,
            source_kind: SourceKind::Pub,
            source_name: "old.pub".into(),
            source_bytes: 10,
            extractor: "test".into(),
            pages: vec![PageInfo {
                index: 0,
                width_pt: 612.0,
                height_pt: 792.0,
                picture_only: false,
            }],
            spans: vec![s],
            images: Vec::new(),
            loss_notes: Vec::new(),
        };
        let out = document_from_dump(&dump, ChurchProfile::default(), false);
        assert_eq!(out.items.len(), 1);
        assert_eq!(out.items[0].badge, Badge::CheckThis);
        assert!(out.items[0].detail.contains("without their places"));
        // And the words themselves still land in a story.
        assert!(out
            .doc
            .stories
            .iter()
            .any(|st| st.paragraphs.iter().any(|p| p.text() == "Grace Lutheran")));
    }

    #[test]
    fn v2_spans_carry_alignment_color_and_italic_into_the_draft_style() {
        // A centered, red, italic two-line block: the draft
        // style must state Center, the red, and italic, and
        // the family must come from the plain font name
        // (Calibri -> Carlito), not a name with the italic
        // glued on.
        let mut s1 = span(0, "A Mighty Fortress", 120.0, 40.0, 14.0, "Calibri", false);
        s1.align = Some(Align::Center);
        s1.italic = true;
        s1.color = [170, 30, 20];
        let mut s2 = span(0, "is our God", 140.0, 58.0, 14.0, "Calibri", false);
        s2.align = Some(Align::Center);
        s2.italic = true;
        s2.color = [170, 30, 20];
        let dump = ExtractionDump {
            schema_version: 2,
            source_kind: SourceKind::Pdf,
            source_name: "hymn.pdf".into(),
            source_bytes: 10,
            extractor: "test".into(),
            pages: vec![PageInfo {
                index: 0,
                width_pt: 612.0,
                height_pt: 792.0,
                picture_only: false,
            }],
            spans: vec![s1, s2],
            images: Vec::new(),
            loss_notes: Vec::new(),
        };
        let out = document_from_dump(&dump, ChurchProfile::default(), false);
        let story = &out.doc.stories[0];
        let style = out
            .doc
            .style(&story.paragraphs[0].style)
            .expect("style exists");
        assert_eq!(style.align, Align::Center);
        assert_eq!(style.color, Rgb::new(170, 30, 20));
        assert!(style.italic);
        assert_eq!(style.font_family, "Carlito");
    }

    #[test]
    fn carried_pictures_become_assets_with_a_review_item() {
        let mut dump_s = span(0, "Welcome", 60.0, 40.0, 14.0, "Calibri", false);
        dump_s.page = 0;
        let png = vec![0x89, b'P', b'N', b'G', 1, 2, 3];
        let bytes: HashMap<String, Vec<u8>> = [("p1-img1.png".to_string(), png.clone())]
            .into_iter()
            .collect();
        let dump = ExtractionDump {
            schema_version: 2,
            source_kind: SourceKind::Pdf,
            source_name: "old.pdf".into(),
            source_bytes: 10,
            extractor: "test".into(),
            pages: vec![PageInfo {
                index: 0,
                width_pt: 612.0,
                height_pt: 792.0,
                picture_only: false,
            }],
            spans: vec![dump_s],
            images: vec![crate::DumpImage {
                id: "p1-img1".into(),
                page: 0,
                x: 40.0,
                y: 100.0,
                w: 200.0,
                h: 150.0,
                file: "p1-img1.png".into(),
            }],
            loss_notes: Vec::new(),
        };
        let out = document_from_dump_with_assets(&dump, ChurchProfile::default(), false, &bytes);
        assert_eq!(out.doc.assets.len(), 1);
        assert_eq!(out.doc.assets[0].kind, AssetKind::Picture);
        assert_eq!(out.doc.assets[0].rights.source, "old.pdf");
        assert_eq!(out.doc.assets[0].rights.license, LicenseKind::Unknown);
        let hash = out.doc.assets[0].sha256.clone();
        assert_eq!(out.doc.asset_data(&hash), Some(png.as_slice()));
        assert!(out
            .items
            .iter()
            .any(|i| i.label == "Pictures from page 1" && i.badge == Badge::CheckThis));
        // The plain builder (analyzer path) carries nothing.
        let plain = document_from_dump(&dump, ChurchProfile::default(), false);
        assert!(plain.doc.assets.is_empty());
    }

    #[test]
    fn font_twins_are_matched() {
        assert_eq!(
            font_match("ABCDEF+ArialMT").as_deref(),
            Some("Liberation Sans")
        );
        assert_eq!(
            font_match("TimesNewRomanPSMT").as_deref(),
            Some("Liberation Serif")
        );
        assert_eq!(font_match("Calibri").as_deref(), Some("Carlito"));
        assert_eq!(font_match("GillSansMT"), None);
    }

    #[test]
    fn threaded_blocks_share_one_story() {
        // T1: two blocks whose spans carry the same thread
        // name (the .pub door's linked boxes) become ONE
        // story named by both blocks' frames, its words in
        // reading order and exactly once. A third block with
        // no thread stands alone, as blocks always have.
        let mut first = span(0, "Alpha one", 60.0, 90.0, 11.0, "Calibri", false);
        first.thread = Some("pub-thread-7".into());
        let mut second = span(0, "Bravo two", 60.0, 300.0, 11.0, "Calibri", false);
        second.thread = Some("pub-thread-7".into());
        let lone = span(0, "Standing alone", 60.0, 500.0, 11.0, "Calibri", false);
        let dump = ExtractionDump {
            schema_version: 2,
            source_kind: SourceKind::Pub,
            source_name: "old.pub".into(),
            source_bytes: 10,
            extractor: "test".into(),
            pages: vec![PageInfo {
                index: 0,
                width_pt: 612.0,
                height_pt: 792.0,
                picture_only: false,
            }],
            spans: vec![first, second, lone],
            images: Vec::new(),
            loss_notes: Vec::new(),
        };
        let out = document_from_dump(&dump, ChurchProfile::default(), false);
        let doc = &out.doc;
        assert_eq!(doc.stories.len(), 2, "thread story + lone story");
        let shared = doc
            .stories
            .iter()
            .find(|s| s.paragraphs.iter().any(|p| p.text().contains("Alpha")))
            .expect("the thread's story");
        let texts: Vec<String> = shared.paragraphs.iter().map(|p| p.text()).collect();
        assert_eq!(
            texts,
            vec!["Alpha one", "Bravo two"],
            "words once, in order"
        );
        let refs = doc.frames_of_story(&shared.id);
        assert_eq!(refs.len(), 2, "both boxes name the shared story");
        // The lone block kept its own story and frame.
        let lone_story = doc
            .stories
            .iter()
            .find(|s| s.id != shared.id)
            .expect("the lone story");
        assert!(lone_story.paragraphs[0].text().contains("Standing alone"));
        assert_eq!(doc.frames_of_story(&lone_story.id).len(), 1);
    }
}
