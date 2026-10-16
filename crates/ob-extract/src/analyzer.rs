//! The Template Analyzer (docs/TEMPLATE-ANALYZER.md): given a
//! stack of old bulletins from one church, learn which words
//! stand still week to week and which change, and suggest a
//! template: standing text stays, weekly text becomes the slots
//! "Next Sunday" will clear. The method is the one the real
//! corpus taught us: rebuild lines, take 8-word shingles, count
//! a passage as standing when most of the family carries it,
//! and cluster first, because standing text moves pages.

use crate::draft::{document_from_dump, rebuilt_page_lines};
use crate::ExtractionDump;
use ob_model::{ChurchProfile, ContentRole, Document};
use std::collections::{HashMap, HashSet};

/// One line of the analyzer's review list.
#[derive(Debug, Clone)]
pub struct TemplateItem {
    pub label: String,
    pub standing: bool,
    pub detail: String,
}

pub struct TemplateOutcome {
    pub doc: Document,
    pub items: Vec<TemplateItem>,
    /// How many of the given bulletins formed the main family.
    pub family: usize,
    pub corpus: usize,
    /// The family member the template was built from.
    pub base_source: String,
}

fn norm(text: &str) -> String {
    let joined = join_single_letters(&norm_words(text));
    joined.join(" ")
}

/// The normalized word list: punctuation folded and stripped.
fn norm_words(text: &str) -> Vec<String> {
    // Lowercase, and strip punctuation from word edges: the
    // same sentence punctuated two ways (or carrying curly
    // quotes in one file and straight in another) must shingle
    // identically, or standing text looks weekly.
    text.replace(['\u{2019}', '\u{2018}'], "'")
        .replace(['\u{201C}', '\u{201D}'], "\"")
        .to_lowercase()
        .split_whitespace()
        .map(|w| {
            w.trim_matches(|c: char| !c.is_alphanumeric() && c != '\'')
                .to_string()
        })
        .filter(|w| !w.is_empty())
        .collect()
}

/// Producers sometimes shatter a word into single letters
/// ("o n" for "on"). Join runs of one-letter tokens back into
/// one token so shingles can match across files.
fn join_single_letters(words: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut run = String::new();
    for w in words {
        if w.chars().count() == 1 {
            run.push_str(w);
        } else {
            if !run.is_empty() {
                out.push(std::mem::take(&mut run));
            }
            out.push(w.clone());
        }
    }
    if !run.is_empty() {
        out.push(run);
    }
    out
}

/// 8-word shingles over a word sequence.
fn shingles(words: &[&str]) -> HashSet<String> {
    let mut out = HashSet::new();
    if words.len() < 8 {
        if !words.is_empty() {
            out.insert(words.join(" "));
        }
        return out;
    }
    for w in words.windows(8) {
        out.insert(w.join(" "));
    }
    out
}

fn doc_lines(dump: &ExtractionDump) -> Vec<String> {
    let mut lines = Vec::new();
    for page in &dump.pages {
        lines.extend(rebuilt_page_lines(dump, page.index));
    }
    lines
}

fn doc_shingle_set(dump: &ExtractionDump) -> HashSet<String> {
    let lines = doc_lines(dump);
    let text = norm(&lines.join(" "));
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut set = shingles(&words);
    // Each line also contributes its own shingles, so a short
    // heading ("Welcome to Worship") can be standing too: the
    // joined-text windows never contain it on its own.
    for line in &lines {
        let n = norm(line);
        let lw: Vec<&str> = n.split_whitespace().collect();
        set.extend(shingles(&lw));
    }
    set
}

/// Analyze a corpus (3 or more dumps from one church). The
/// largest family of look-alike bulletins drives the template;
/// the base is the family member carrying the most text.
pub fn analyze(
    dumps: &[ExtractionDump],
    profile: ChurchProfile,
) -> Result<TemplateOutcome, String> {
    if dumps.len() < 3 {
        return Err("three or more old bulletins are needed to see what stands still".into());
    }
    let sets: Vec<HashSet<String>> = dumps.iter().map(doc_shingle_set).collect();
    // Family: the dump whose neighborhood is largest, where two
    // bulletins are neighbors when their shared shingles cover
    // at least a quarter of the smaller one's set (the overlap
    // coefficient: fairer than Jaccard when lengths differ, and
    // the shape the corpus clusters actually took).
    let overlap = |i: usize, j: usize| -> f32 {
        let inter = sets[i].intersection(&sets[j]).count();
        let smaller = sets[i].len().min(sets[j].len()).max(1);
        inter as f32 / smaller as f32
    };
    // Seed with the closest pair, then grow: any bulletin that
    // overlaps a current member by a quarter or more joins.
    // (Single-linkage at a high threshold; it is the shape the
    // corpus families actually took.)
    let (mut a, mut b, mut best_pair) = (0usize, 0usize, -1.0f32);
    for i in 0..dumps.len() {
        for j in (i + 1)..dumps.len() {
            let ov = overlap(i, j);
            if ov > best_pair {
                best_pair = ov;
                a = i;
                b = j;
            }
        }
    }
    let mut best: Vec<usize> = vec![a, b];
    if dumps.len() == 1 {
        best = vec![0];
    }
    loop {
        let mut grew = false;
        for j in 0..dumps.len() {
            if !best.contains(&j) && best.iter().any(|&m| overlap(m, j) >= 0.25) {
                best.push(j);
                grew = true;
            }
        }
        if !grew {
            break;
        }
    }
    best.sort_unstable();
    let family = &best;
    // Standing shingles: in at least 80% of the family.
    let need = ((family.len() as f32) * 0.8).ceil() as usize;
    let mut counts: HashMap<String, usize> = HashMap::new();
    for j in family {
        for sh in &sets[*j] {
            *counts.entry(sh.clone()).or_default() += 1;
        }
    }
    let standing: HashSet<String> = counts
        .into_iter()
        .filter(|(_, c)| *c >= need.max(1))
        .map(|(sh, _)| sh)
        .collect();
    let standing_lines: HashSet<String> = {
        let mut out = HashSet::new();
        for j in family {
            for line in doc_lines(&dumps[*j]) {
                let n = norm(&line);
                let words: Vec<&str> = n.split_whitespace().collect();
                if !words.is_empty() && shingles(&words).iter().all(|sh| standing.contains(sh)) {
                    out.insert(n);
                }
            }
        }
        out
    };
    // Base: the family member with the most rebuilt text.
    let base_idx = *family
        .iter()
        .max_by_key(|j| doc_lines(&dumps[**j]).join(" ").len())
        .unwrap_or(&family[0]);
    let mut out = document_from_dump(&dumps[base_idx], profile, true);
    // Classify each story: standing when most of its paragraphs
    // are standing lines (or carry a standing shingle).
    let mut items = Vec::new();
    let mut role_by_story: HashMap<String, ContentRole> = HashMap::new();
    for story in &out.doc.stories {
        let mut stand = 0usize;
        for para in &story.paragraphs {
            let n = norm(&para.text());
            let words: Vec<&str> = n.split_whitespace().collect();
            let para_standing = standing_lines.contains(&n)
                || shingles(&words).iter().any(|sh| standing.contains(sh));
            if para_standing {
                stand += 1;
            }
        }
        let is_standing = !story.paragraphs.is_empty() && stand * 2 >= story.paragraphs.len();
        role_by_story.insert(
            story.id.clone(),
            if is_standing {
                ContentRole::Standing
            } else {
                ContentRole::Weekly
            },
        );
        let label: String = story
            .paragraphs
            .first()
            .map(|p| p.text().chars().take(44).collect())
            .unwrap_or_default();
        items.push(TemplateItem {
            label,
            standing: is_standing,
            detail: if is_standing {
                "Appears in nearly every bulletin: it stays put each week.".into()
            } else {
                "Changes from week to week: Next Sunday clears it for the new words.".into()
            },
        });
    }
    for panel in &mut out.doc.panels {
        for frame in &mut panel.frames {
            if let Some(sid) = &frame.story_id {
                if let Some(role) = role_by_story.get(sid) {
                    frame.role = *role;
                }
            }
        }
    }
    Ok(TemplateOutcome {
        doc: out.doc,
        items,
        family: family.len(),
        corpus: dumps.len(),
        base_source: dumps[base_idx].source_name.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PageInfo, SourceKind, Span, SpanTrust};

    fn dump_with(name: &str, weekly: &str) -> ExtractionDump {
        let mut spans = Vec::new();
        let standing =
            "the lord is my shepherd i shall not want he makes me lie down in green pastures";
        spans.push(Span {
            page: 0,
            text: standing.into(),
            trust: SpanTrust::RawText,
            x: Some(40.0),
            y: Some(40.0),
            w: Some(300.0),
            h: Some(14.0),
            font_size: Some(11.0),
            bold: Some(false),
            font: Some("Calibri".into()),
            align: None,
            italic: false,
            color: [0, 0, 0],
            runs: Vec::new(),
            thread: None,
        });
        spans.push(Span {
            page: 0,
            text: weekly.into(),
            trust: SpanTrust::RawText,
            x: Some(40.0),
            y: Some(120.0),
            w: Some(300.0),
            h: Some(14.0),
            font_size: Some(11.0),
            bold: Some(false),
            font: Some("Calibri".into()),
            align: None,
            italic: false,
            color: [0, 0, 0],
            runs: Vec::new(),
            thread: None,
        });
        ExtractionDump {
            schema_version: 1,
            source_kind: SourceKind::Pdf,
            source_name: name.into(),
            source_bytes: 10,
            extractor: "test".into(),
            pages: vec![PageInfo {
                index: 0,
                width_pt: 612.0,
                height_pt: 792.0,
                picture_only: false,
            }],
            spans,
            images: Vec::new(),
            loss_notes: Vec::new(),
        }
    }

    #[test]
    fn analyzer_separates_standing_from_weekly() {
        let dumps = vec![
            dump_with("a.pdf", "potluck this friday at six bring a dish"),
            dump_with("b.pdf", "choir rehearsal moved to thursday evening"),
            dump_with("c.pdf", "youth group car wash on saturday morning"),
        ];
        let out = analyze(&dumps, ChurchProfile::default()).expect("analyze");
        assert_eq!(out.family, 3);
        assert!(out.items.iter().any(|i| i.standing), "the psalm stands");
        assert!(out.items.iter().any(|i| !i.standing), "the notice changes");
        // The standing story's frame carries the Standing role.
        let roles: Vec<ContentRole> = out
            .doc
            .panels
            .iter()
            .flat_map(|p| p.frames.iter().map(|f| f.role))
            .collect();
        assert!(roles.contains(&ContentRole::Standing));
        assert!(roles.contains(&ContentRole::Weekly));
    }

    #[test]
    fn analyzer_needs_three() {
        let dumps = vec![dump_with("a.pdf", "x"), dump_with("b.pdf", "y")];
        assert!(analyze(&dumps, ChurchProfile::default()).is_err());
    }
}
