//! Pub text probe: phase 2b of the native reader (the Quill
//! text rescue) against the POI sample files.
//!
//! For every .pub in the samples folder it reports the file's
//! family (0x2C files are parsed; 0x22 files are reported as
//! deferred, never parsed), the story accounting from the
//! loss report, and (where POI's expected-text file sits
//! beside the sample) how many of the expected non-empty
//! lines appear verbatim in the emitted text.
//!
//! POI's .txt files are another extractor's output, so they
//! are a cross-check, not ground truth. Matching: both sides
//! are whitespace-normalized (every whitespace run becomes
//! one space), and an expected line counts as found when it
//! appears in the stream of emitted span texts joined with
//! single spaces. That makes table cells that POI prints on
//! one printed line but the file stores as separate
//! paragraphs findable, while the words themselves must
//! match exactly and in order.
//!
//! Usage: pub_text_probe [samples-dir]
//! Default samples dir: ~/workspace/openbulletin-fixtures/pub/samples
use ob_extract::cfb::Cfb;
use ob_extract::pub_layout::extract_pub_positioned;
use ob_extract::pub_text::{contents_version, extract_pub_text};
use std::path::PathBuf;

fn normalize(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn main() {
    let dir = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let home = std::env::var("HOME").unwrap_or_default();
            PathBuf::from(format!(
                "{home}/workspace/openbulletin-fixtures/pub/samples"
            ))
        });
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("samples dir")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|x| x == "pub").unwrap_or(false))
        .collect();
    files.sort();
    for path in files {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(e) => {
                println!("{name}: cannot read ({e})");
                continue;
            }
        };
        let cfb = match Cfb::open(bytes.clone()) {
            Ok(c) => c,
            Err(e) => {
                println!("{name}: not a readable compound file ({e})");
                continue;
            }
        };
        match contents_version(&cfb) {
            Some(0x22) => {
                println!("{name}: family 0x22 (Publisher 2000 or earlier), deferred");
                continue;
            }
            Some(0x2C) => {}
            other => {
                println!("{name}: unrecognized Contents family {other:?}");
                continue;
            }
        }
        match extract_pub_text(&cfb, &name, bytes.len()) {
            Ok((dump, loss)) => {
                println!(
                    "{name}: family 0x2C, {} pages, {} spans, {} characters",
                    dump.pages.len(),
                    dump.spans.len(),
                    loss.characters_emitted
                );
                println!(
                    "    stories: {} total, {} emitted, {} excluded, {} unresolved, \
                     {} with unresolved ids; {} characters unresolved; \
                     {} runs ({} styled by STSH defaults only); \
                     {} control characters mapped",
                    loss.stories_total,
                    loss.stories_emitted,
                    loss.stories_excluded,
                    loss.stories_unresolved,
                    loss.stories_with_unresolved_ids,
                    loss.characters_unresolved,
                    loss.runs_emitted,
                    loss.runs_styled_by_default,
                    loss.control_chars_mapped
                );
                for reason in &loss.unresolved_reasons {
                    println!("    unresolved: {reason}");
                }
                for note in &loss.notes {
                    println!("    note: {note}");
                }
                // Phase 2c: the positioned rescue.
                match extract_pub_positioned(&cfb, &name, bytes.len()) {
                    Ok(x) => {
                        let l = &x.loss;
                        println!(
                            "    2c positioned: {}, {} pages, {} spans",
                            x.positioned,
                            x.dump.pages.len(),
                            x.dump.spans.len()
                        );
                        if x.positioned {
                            println!(
                                "    shapes: {} total, {} placed, {} unplaced, \
                                 {} deleted skipped, {} alternates excluded; \
                                 {} master shapes, {} table cells, {} wordart, \
                                 {} stories unplaced; pictures: {} found, {} carried",
                                l.shapes_total,
                                l.shapes_placed,
                                l.shapes_unplaced,
                                l.shapes_deleted_skipped,
                                l.alternate_shapes_excluded,
                                l.master_shapes_emitted,
                                l.table_cells_emitted,
                                l.wordart_emitted,
                                l.stories_unplaced,
                                l.pictures_found,
                                l.pictures_carried
                            );
                            // Sanity: placed text spans sit
                            // inside their page bounds.
                            let mut out = 0usize;
                            let mut out_bottom = 0usize;
                            let mut out_right = 0usize;
                            for sp in &x.dump.spans {
                                if sp.trust == ob_extract::SpanTrust::PictureOnly {
                                    continue;
                                }
                                let (Some(sx), Some(sy), Some(sw), Some(sh)) =
                                    (sp.x, sp.y, sp.w, sp.h)
                                else {
                                    continue;
                                };
                                let page = &x.dump.pages[sp.page as usize];
                                let right = sx + sw > page.width_pt + 1.0;
                                let bottom = sy + sh > page.height_pt + 1.0;
                                if sx < -1.0 || sy < -1.0 || right || bottom {
                                    out += 1;
                                    out_right += right as usize;
                                    out_bottom += bottom as usize;
                                }
                            }
                            println!(
                                "    spans outside page bounds: {out} (right {out_right}, bottom {out_bottom})"
                            );
                            for note in l.notes.iter().filter(|n| !loss.notes.contains(n)) {
                                println!("    2c note: {note}");
                            }
                        }
                    }
                    Err(e) => println!("    2c: failed ({e})"),
                }
                // The cross-check against POI's expected text.
                let txt_path = path.with_extension("txt");
                if let Ok(expected) = std::fs::read_to_string(&txt_path) {
                    let haystack = normalize(
                        &dump
                            .spans
                            .iter()
                            .map(|s| s.text.as_str())
                            .collect::<Vec<_>>()
                            .join(" "),
                    );
                    let lines: Vec<String> = expected
                        .lines()
                        .map(normalize)
                        .filter(|l| !l.is_empty())
                        .collect();
                    let found = lines
                        .iter()
                        .filter(|l| haystack.contains(l.as_str()))
                        .count();
                    let pct = if lines.is_empty() {
                        100.0
                    } else {
                        found as f64 * 100.0 / lines.len() as f64
                    };
                    println!(
                        "    POI cross-check: {} of {} expected lines found ({:.0}%)",
                        found,
                        lines.len(),
                        pct
                    );
                    for l in &lines {
                        if !haystack.contains(l.as_str()) {
                            println!("    missing: {l}");
                        }
                    }
                }
            }
            Err(e) => println!("{name}: extraction failed ({e})"),
        }
    }
}
