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
    /// Said out loud when the family was too small to learn
    /// standing text from (fewer than three alike): the
    /// suggestion still arrives, but nothing in it is marked
    /// standing, and this sentence is why (round 2, R2-1).
    pub family_note: Option<String>,
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
/// `library_families` is the Font Library's family list at
/// learn time: learned styles keep the source names either
/// way, and the learned document's wanted-font records form
/// against it exactly as a single import's do.
pub fn analyze(
    dumps: &[ExtractionDump],
    profile: ChurchProfile,
    library_families: &[String],
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
    const NEIGHBOR: f32 = 0.25;
    let n = dumps.len();
    let pairs: Vec<(usize, usize, f32)> = (0..n)
        .flat_map(|i| ((i + 1)..n).map(move |j| (i, j)))
        .map(|(i, j)| {
            let inter = sets[i].intersection(&sets[j]).count();
            let smaller = sets[i].len().min(sets[j].len()).max(1);
            (i, j, inter as f32 / smaller as f32)
        })
        .collect();
    let mut ov = vec![vec![0.0f32; n]; n];
    for (i, j, v) in pairs {
        ov[i][j] = v;
        ov[j][i] = v;
    }
    // Seed: the dump with the most neighbors, ties broken by
    // the most total overlap, then by lowest index so the
    // choice never depends on iteration luck. Seeding from the
    // neighborhood (not from the single closest pair) is what
    // the method promises: a near-duplicate pair off to the
    // side (the same special bulletin picked twice) has a
    // neighborhood of one apiece and cannot outvote the real
    // family's larger neighborhood (round 2, R2-1).
    let mut seed = 0usize;
    let mut seed_key = (0usize, 0.0f32);
    for (i, row) in ov.iter().enumerate() {
        let neighbors = row
            .iter()
            .enumerate()
            .filter(|(j, v)| *j != i && **v >= NEIGHBOR)
            .count();
        let total: f32 = row.iter().sum();
        if (neighbors, total) > seed_key {
            seed_key = (neighbors, total);
            seed = i;
        }
    }
    // Grow one at a time: the outsider whose AVERAGE overlap
    // with the current family is highest joins, while that
    // average clears the neighbor threshold. Average linkage,
    // not any-single-link: a drifting chain (each month
    // resembling the next, the year-ago bulletin resembling
    // nothing current) can no longer pull its far tail in one
    // link at a time (round 2, R2-2).
    let mut best: Vec<usize> = vec![seed];
    loop {
        let mut pick: Option<usize> = None;
        let mut pick_avg = 0.0f32;
        let candidates = (0..n).filter(|j| !best.contains(j)).map(|j| {
            (
                j,
                best.iter().map(|&m| ov[m][j]).sum::<f32>() / best.len() as f32,
            )
        });
        for (j, avg) in candidates {
            if avg >= NEIGHBOR && (pick.is_none() || avg > pick_avg) {
                pick = Some(j);
                pick_avg = avg;
            }
        }
        match pick {
            Some(j) => best.push(j),
            None => break,
        }
    }
    best.sort_unstable();
    let family = &best;
    // Standing takes three alike, the same minimum the
    // learning door enforces on the pick itself: with a
    // family of two, the 80% rule below would demand both
    // copies agree, and a duplicated special bulletin's dated
    // text (present in both copies) would be canonized as
    // standing (round 2, R2-1). A smaller family still yields
    // a suggestion built from its fullest member, but nothing
    // in it is marked standing, and the reason is said in the
    // outcome's family note.
    let family_too_small = family.len() < 3;
    let family_note = if family_too_small {
        Some(
            "These bulletins do not include three alike enough to learn what \
             stays the same, so nothing was marked as standing: every piece \
             is set to change weekly."
                .to_string(),
        )
    } else {
        None
    };
    // Standing shingles: in at least 80% of the family.
    let need = ((family.len() as f32) * 0.8).ceil() as usize;
    let mut counts: HashMap<String, usize> = HashMap::new();
    for j in family {
        for sh in &sets[*j] {
            *counts.entry(sh.clone()).or_default() += 1;
        }
    }
    let standing: HashSet<String> = if family_too_small {
        HashSet::new()
    } else {
        counts
            .into_iter()
            .filter(|(_, c)| *c >= need.max(1))
            .map(|(sh, _)| sh)
            .collect()
    };
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
    let mut out = document_from_dump(&dumps[base_idx], profile, true, library_families);
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
        family_note,
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
        let out = analyze(&dumps, ChurchProfile::default(), &[]).expect("analyze");
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
        assert!(analyze(&dumps, ChurchProfile::default(), &[]).is_err());
    }

    /// A block of distinct words sharing a tag ("ab00",
    /// "ab01", ...). Every word is longer than one letter, so
    /// the single-letter joining leaves them alone, and two
    /// blocks with different tags share no word at all: the
    /// overlap between two dumps below is exactly the windows
    /// inside the blocks they share.
    fn block(tag: &str, count: usize) -> Vec<String> {
        (0..count).map(|i| format!("{tag}{i:02}")).collect()
    }

    /// One dump whose whole text is the given word blocks in
    /// order, as a single span on a single page.
    fn dump_words(name: &str, parts: Vec<Vec<String>>) -> ExtractionDump {
        let text = parts.concat().join(" ");
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
            spans: vec![Span {
                page: 0,
                text,
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
            }],
            images: Vec::new(),
            loss_notes: Vec::new(),
        }
    }

    #[test]
    fn a_duplicated_oddity_does_not_become_the_family() {
        // Three regular weeklies sharing one standing block,
        // plus the same special bulletin picked twice with one
        // word changed. The pair is the closest match in the
        // pile (the old seed), but its neighborhood is one
        // apiece against the regulars' two (R2-1).
        let standing = block("stand", 40);
        let special = block("easter", 40);
        let dated = block("dated", 16);
        let regular =
            |name: &str, own: &str| dump_words(name, vec![standing.clone(), block(own, 20)]);
        let mut changed = dated.clone();
        changed[3] = "changedword".to_string();
        let dumps = vec![
            regular("week1.pdf", "wkone"),
            regular("week2.pdf", "wktwo"),
            regular("week3.pdf", "wkthree"),
            dump_words("easter.pdf", vec![special.clone(), dated.clone()]),
            dump_words("easter-copy.pdf", vec![special, changed]),
        ];
        let out = analyze(&dumps, ChurchProfile::default(), &[]).expect("analyze");
        assert_eq!(out.family, 3, "the three regulars are the family");
        assert!(
            out.base_source.starts_with("week"),
            "the template is built from a regular: {}",
            out.base_source
        );
        assert!(out.family_note.is_none());
        assert!(
            out.items.iter().any(|i| i.standing),
            "the regulars' shared words stand"
        );
    }

    #[test]
    fn a_family_of_two_marks_nothing_standing() {
        // Two near-duplicates and one unrelated bulletin: the
        // best family has two members, and two is not enough
        // to learn what stays the same (R2-1). The suggestion
        // still arrives, with nothing standing and the reason
        // said in the family note.
        let shared = block("pair", 40);
        let dumps = vec![
            dump_words("pair-a.pdf", vec![shared.clone(), block("aonly", 12)]),
            dump_words("pair-b.pdf", vec![shared, block("bonly", 12)]),
            dump_words("loner.pdf", vec![block("alone", 52)]),
        ];
        let out = analyze(&dumps, ChurchProfile::default(), &[]).expect("analyze");
        assert_eq!(out.family, 2);
        let note = out.family_note.expect("the reason is said out loud");
        assert!(note.contains("three alike"), "{note}");
        assert!(
            out.items.iter().all(|i| !i.standing),
            "nothing is marked standing"
        );
        let roles: Vec<ContentRole> = out
            .doc
            .panels
            .iter()
            .flat_map(|p| p.frames.iter().map(|f| f.role))
            .collect();
        assert!(!roles.is_empty());
        assert!(
            roles.iter().all(|r| *r == ContentRole::Weekly),
            "every frame is weekly"
        );
    }

    #[test]
    fn a_drifting_chain_does_not_pull_its_tail_in() {
        // A year of drift: consecutive bulletins overlap
        // strongly (ab/bc share 60 of 100 windows, cd 50),
        // second neighbors weakly (ac/bd 15), the two ends not
        // at all. Single linkage chained all four into one
        // family; average linkage keeps the family at the
        // three that resemble the family as a whole (R2-2).
        let ab = block("ab", 67);
        let bc = block("bc", 67);
        let cd = block("cd", 57);
        let ac = block("ac", 22);
        let bd = block("bd", 22);
        let dumps = vec![
            dump_words("a.pdf", vec![ab.clone(), ac.clone(), block("fa", 18)]),
            dump_words("b.pdf", vec![ab, bc.clone(), bd.clone()]),
            dump_words("c.pdf", vec![bc, cd.clone(), ac]),
            dump_words("d.pdf", vec![cd, bd, block("fd", 28)]),
        ];
        let out = analyze(&dumps, ChurchProfile::default(), &[]).expect("analyze");
        assert_eq!(out.family, 3, "the tail stays out of the family");
        assert_ne!(out.base_source, "d.pdf");
        assert!(out.family_note.is_none());
    }
}
