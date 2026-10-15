//! Corpus consensus pass (Template Analyzer, v2: word shingles).
//!
//! v1 matched whole extracted lines exactly and found almost nothing:
//! producers chunk text differently, so the same sentence rarely
//! arrives as the same "line". Shingles (runs of 8 words) survive
//! chunk boundaries. A passage counts as standing when its shingles
//! appear in at least 80% of the bulletins.
//!
//! Prints aggregate counts only. Example passages go to the JSON
//! report beside the corpus, never into the repo (real names).
//!
//! Usage: cargo run -p ob-extract --example consensus -- <dir> <report.json>
use std::collections::{BTreeMap, HashMap, HashSet};
use std::process::exit;

const SHINGLE: usize = 8;

fn words(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(|w| {
            w.trim_matches(|c: char| !c.is_alphanumeric())
                .to_lowercase()
        })
        .filter(|w| !w.is_empty())
        .collect()
}

fn shingles(ws: &[String]) -> HashSet<Vec<String>> {
    if ws.len() < SHINGLE {
        return HashSet::new();
    }
    ws.windows(SHINGLE).map(|w| w.to_vec()).collect()
}

/// Fraction of a word stream covered by a set of standing shingles.
fn coverage(ws: &[String], standing: &HashSet<Vec<String>>) -> f32 {
    if ws.len() < SHINGLE {
        return 0.0;
    }
    let mut covered = vec![false; ws.len()];
    for (i, w) in ws.windows(SHINGLE).enumerate() {
        if standing.contains(w) {
            for c in covered.iter_mut().skip(i).take(SHINGLE) {
                *c = true;
            }
        }
    }
    covered.iter().filter(|c| **c).count() as f32 / ws.len() as f32
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() != 2 {
        eprintln!("usage: consensus <dir> <report.json>");
        exit(2);
    }
    let mut paths: Vec<_> = std::fs::read_dir(&args[0])
        .expect("read dir")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|x| x == "pdf").unwrap_or(false))
        .collect();
    paths.sort();
    let n = paths.len();
    let cut = ((n as f32) * 0.8).ceil() as usize;
    println!("corpus: {n} bulletins, standing cut: present in >= {cut}");

    // per bulletin: whole-doc words, per-page words
    let mut docs: Vec<Vec<String>> = Vec::new();
    let mut pages: Vec<BTreeMap<u32, Vec<String>>> = Vec::new();
    for path in &paths {
        let bytes = std::fs::read(path).expect("read pdf");
        let dump = ob_extract::extract_pdf(&bytes, &path.display().to_string()).expect("extract");
        let mut doc_words = Vec::new();
        let mut page_words: BTreeMap<u32, Vec<String>> = BTreeMap::new();
        for span in dump.spans.iter().filter(|s| s.trust == ob_extract::SpanTrust::RawText) {
            let ws = words(&span.text);
            page_words.entry(span.page).or_default().extend(ws.iter().cloned());
            doc_words.extend(ws);
        }
        docs.push(doc_words);
        pages.push(page_words);
    }

    // Document-level standing shingles
    let mut doc_count: HashMap<Vec<String>, usize> = HashMap::new();
    for d in &docs {
        for sh in shingles(d) {
            *doc_count.entry(sh).or_default() += 1;
        }
    }
    let standing_doc: HashSet<Vec<String>> = doc_count
        .iter()
        .filter(|(_, c)| **c >= cut)
        .map(|(sh, _)| sh.clone())
        .collect();
    println!("standing passages (8-word runs) anywhere in the booklet: {}", standing_doc.len());
    let covs: Vec<f32> = docs.iter().map(|d| coverage(d, &standing_doc)).collect();
    let mut sorted = covs.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    println!(
        "share of each bulletin made of standing text: min {:.0}%, median {:.0}%, max {:.0}%",
        sorted[0] * 100.0,
        sorted[sorted.len() / 2] * 100.0,
        sorted[sorted.len() - 1] * 100.0
    );

    // Per-page standing shingles and coverage (median across bulletins)
    let page_nums: HashSet<u32> = pages.iter().flat_map(|p| p.keys().copied()).collect();
    let mut report = serde_json::json!({ "bulletins": n, "standing_passages": standing_doc.len() });
    let mut examples: Vec<String> = standing_doc
        .iter()
        .take(40)
        .map(|sh| sh.join(" "))
        .collect();
    examples.sort();
    report["standing_examples_private"] = serde_json::json!(examples);
    for page in 0..*page_nums.iter().max().unwrap_or(&0) {
        let mut page_count: HashMap<Vec<String>, usize> = HashMap::new();
        for p in &pages {
            if let Some(ws) = p.get(&page) {
                for sh in shingles(ws) {
                    *page_count.entry(sh).or_default() += 1;
                }
            }
        }
        let standing_page: HashSet<Vec<String>> = page_count
            .iter()
            .filter(|(_, c)| **c >= cut)
            .map(|(sh, _)| sh.clone())
            .collect();
        let mut pcovs: Vec<f32> = pages
            .iter()
            .filter_map(|p| p.get(&page))
            .map(|ws| coverage(ws, &standing_page))
            .collect();
        pcovs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let median = if pcovs.is_empty() { 0.0 } else { pcovs[pcovs.len() / 2] };
        println!(
            "page {:>2}: standing passages {:>3} | median standing share {:.0}%",
            page + 1,
            standing_page.len(),
            median * 100.0
        );
        report["pages"][page.to_string()] = serde_json::json!({
            "standing_passages": standing_page.len(),
            "median_standing_share": median,
        });
    }
    std::fs::write(&args[1], serde_json::to_string_pretty(&report).unwrap()).expect("write report");
    println!("report written to {}", args[1]);
}
