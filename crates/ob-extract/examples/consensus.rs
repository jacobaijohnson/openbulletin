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

/// Rebuild a page's true lines from positioned spans: group spans
/// into rows by vertical overlap, order each row left to right, and
/// join with a space only where the horizontal gap looks like a word
/// space. Tiny gaps mean the producer split a word mid-way; joining
/// without a space heals the split. Returns None when the dump has
/// no positions (older scaffold dumps).
fn rebuilt_lines(dump: &ob_extract::ExtractionDump, page: u32) -> Option<Vec<String>> {
    let mut spans: Vec<&ob_extract::Span> = dump
        .spans
        .iter()
        .filter(|s| s.page == page && s.x.is_some() && s.y.is_some())
        .collect();
    if spans.is_empty() {
        return None;
    }
    spans.sort_by(|a, b| {
        a.y.unwrap()
            .partial_cmp(&b.y.unwrap())
            .unwrap()
            .then(a.x.unwrap().partial_cmp(&b.x.unwrap()).unwrap())
    });
    let mut rows: Vec<Vec<&ob_extract::Span>> = Vec::new();
    for sp in spans {
        let y = sp.y.unwrap();
        let h = sp.h.unwrap_or(10.0).max(1.0);
        match rows.last_mut() {
            Some(row) if (y - row[0].y.unwrap()).abs() <= 0.5 * h => row.push(sp),
            _ => rows.push(vec![sp]),
        }
    }
    let mut lines = Vec::new();
    for mut row in rows {
        row.sort_by(|a, b| a.x.unwrap().partial_cmp(&b.x.unwrap()).unwrap());
        let mut line = String::new();
        let mut prev_end: Option<f32> = None;
        for sp in row {
            let x = sp.x.unwrap();
            if let Some(pe) = prev_end {
                let gap = x - pe;
                let size = sp.font_size.unwrap_or(11.0);
                if gap > 0.22 * size {
                    line.push(' ');
                }
            }
            line.push_str(sp.text.trim());
            prev_end = Some(x + sp.w.unwrap_or(0.0));
        }
        if !line.trim().is_empty() {
            lines.push(line);
        }
    }
    Some(lines)
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
        .filter(|p| {
            p.extension()
                .map(|x| x == "pdf" || x == "json")
                .unwrap_or(false)
        })
        .collect();
    paths.sort();
    let from_dumps = paths
        .first()
        .map(|p| p.extension().map(|x| x == "json").unwrap_or(false))
        .unwrap_or(false);
    let n = paths.len();
    let cut = ((n as f32) * 0.8).ceil() as usize;
    println!("corpus: {n} bulletins, standing cut: present in >= {cut}");

    // per bulletin: whole-doc words, per-page words
    let mut docs: Vec<Vec<String>> = Vec::new();
    let mut pages: Vec<BTreeMap<u32, Vec<String>>> = Vec::new();
    for path in &paths {
        let dump = if from_dumps {
            let json = std::fs::read_to_string(path).expect("read dump");
            serde_json::from_str(&json).expect("parse dump")
        } else {
            let bytes = std::fs::read(path).expect("read pdf");
            ob_extract::extract_pdf(&bytes, &path.display().to_string()).expect("extract")
        };
        let mut doc_words = Vec::new();
        let mut page_words: BTreeMap<u32, Vec<String>> = BTreeMap::new();
        let page_nums: Vec<u32> = dump.pages.iter().map(|pg| pg.index).collect();
        for page in page_nums {
            if let Some(lines) = rebuilt_lines(&dump, page) {
                let mut ws = Vec::new();
                for line in &lines {
                    ws.extend(words(line));
                }
                page_words
                    .entry(page)
                    .or_default()
                    .extend(ws.iter().cloned());
                doc_words.extend(ws);
            } else {
                for span in dump
                    .spans
                    .iter()
                    .filter(|sp| sp.page == page && sp.trust == ob_extract::SpanTrust::RawText)
                {
                    let ws = words(&span.text);
                    page_words
                        .entry(page)
                        .or_default()
                        .extend(ws.iter().cloned());
                    doc_words.extend(ws);
                }
            }
        }
        docs.push(doc_words);
        pages.push(page_words);
    }

    // --- Automatic clustering -------------------------------------
    // The analyzer must find a church's families on its own, without
    // reading filenames. Similarity = Jaccard over 8-word shingle
    // sets; clusters are connected components above a threshold.
    let doc_sets: Vec<HashSet<Vec<String>>> = docs.iter().map(|d| shingles(d)).collect();
    let names: Vec<String> = paths
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
        .collect();
    // Clustering v2 (evidence: docs/EXTRACT.md). A single global bar
    // was the wrong shape: the main family chains at 0.14+, while a
    // real subfamily sat at 0.11. Rules, in order:
    //   1. Mutual closest siblings merge (each names the other).
    //   2. A bulletin joins its closest sibling's cluster when that
    //      similarity clears a low floor: it reaches for that family
    //      over all others.
    //   3. Two clusters merge only at the strict bar.
    // Remaining pairs above a hint floor are reported as "looks
    // related" for the human review step, never auto-merged.
    let sim = |i: usize, j: usize| -> f32 {
        let inter = doc_sets[i].intersection(&doc_sets[j]).count() as f32;
        let union = doc_sets[i].union(&doc_sets[j]).count() as f32;
        if union > 0.0 {
            inter / union
        } else {
            0.0
        }
    };
    let best: Vec<(f32, usize)> = (0..n)
        .map(|i| {
            (0..n).filter(|&j| j != i).map(|j| (sim(i, j), j)).fold(
                (0.0f32, usize::MAX),
                |acc, x| if x.0 > acc.0 { x } else { acc },
            )
        })
        .collect();
    let mut parent: Vec<usize> = (0..n).collect();
    fn find(parent: &mut [usize], mut x: usize) -> usize {
        while parent[x] != x {
            parent[x] = parent[parent[x]];
            x = parent[x];
        }
        x
    }
    fn union(parent: &mut [usize], a: usize, b: usize) {
        let (ra, rb) = (find(parent, a), find(parent, b));
        if ra != rb {
            parent[ra] = rb;
        }
    }
    // rule 1: mutual closest siblings
    for i in 0..n {
        let j = best[i].1;
        if j != usize::MAX && best[j].1 == i && best[i].0 >= 0.08 {
            union(&mut parent, i, j);
        }
    }
    // rule 2: attach to closest sibling's cluster (descending strength)
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| best[b].0.partial_cmp(&best[a].0).unwrap());
    for &i in &order {
        let j = best[i].1;
        if j != usize::MAX && best[i].0 >= 0.10 {
            union(&mut parent, i, j);
        }
    }
    // rule 3: cluster-cluster merge at the strict bar
    loop {
        let mut merged = false;
        for i in 0..n {
            for j in (i + 1)..n {
                if find(&mut parent, i) != find(&mut parent, j) && sim(i, j) >= 0.12 {
                    union(&mut parent, i, j);
                    merged = true;
                }
            }
        }
        if !merged {
            break;
        }
    }
    // relatedness hints between final clusters
    let mut clusters: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for i in 0..n {
        let r = find(&mut parent, i);
        clusters.entry(r).or_default().push(i);
    }
    let roots: Vec<usize> = clusters.keys().copied().collect();
    for a in 0..roots.len() {
        for b in (a + 1)..roots.len() {
            let cross = clusters[&roots[a]]
                .iter()
                .flat_map(|&i| clusters[&roots[b]].iter().map(move |&j| sim(i, j)))
                .fold(0.0f32, f32::max);
            if cross >= 0.08 {
                println!(
                    "hint: clusters {} and {} look related (best pair {:.2})",
                    a + 1,
                    b + 1,
                    cross
                );
            }
        }
    }
    println!("closest sibling per bulletin (Jaccard):");
    for i in 0..n {
        println!(
            "    {:.2}  {}  ~  {}",
            best[i].0, names[i], names[best[i].1]
        );
    }
    println!("automatic clusters (mutual-sibling rules):");
    let mut cluster_report = serde_json::json!({});
    for (k, members) in clusters.values().enumerate() {
        // consensus inside the cluster
        let ccut = ((members.len() as f32) * 0.8).ceil() as usize;
        let mut ccount: HashMap<Vec<String>, usize> = HashMap::new();
        for &m in members {
            for sh in &doc_sets[m] {
                *ccount.entry(sh.clone()).or_default() += 1;
            }
        }
        let cstanding: HashSet<Vec<String>> = ccount
            .iter()
            .filter(|(_, c)| **c >= ccut)
            .map(|(sh, _)| sh.clone())
            .collect();
        let mut cshare: Vec<f32> = members
            .iter()
            .map(|&m| coverage(&docs[m], &cstanding))
            .collect();
        cshare.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let median = cshare[cshare.len() / 2];
        println!(
            "cluster {}: {} bulletins | standing passages {} | median standing share {:.0}%",
            k + 1,
            members.len(),
            cstanding.len(),
            median * 100.0
        );
        for &m in members {
            println!("    {}", names[m]);
        }
        cluster_report[format!("cluster_{}", k + 1)] = serde_json::json!({
            "members": members.iter().map(|&m| names[m].clone()).collect::<Vec<_>>(),
            "standing_passages": cstanding.len(),
            "median_standing_share": median,
        });
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
    println!(
        "standing passages (8-word runs) anywhere in the booklet: {}",
        standing_doc.len()
    );
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
    report["clusters"] = cluster_report;
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
        let median = if pcovs.is_empty() {
            0.0
        } else {
            pcovs[pcovs.len() / 2]
        };
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
