//! Analyzer probe: real dumps in, template summary out.
//! Usage: analyzer_probe <dump.json> <dump.json> <dump.json> [...]
use ob_extract::analyzer::analyze;
use ob_extract::ExtractionDump;
use ob_model::ChurchProfile;

fn main() {
    let dumps: Vec<ExtractionDump> = std::env::args()
        .skip(1)
        .map(|p| {
            let text = std::fs::read_to_string(&p).expect("read dump");
            ExtractionDump::from_json(&text).expect("parse dump")
        })
        .collect();
    let out = analyze(&dumps, ChurchProfile::default()).expect("analyze");
    let standing = out.items.iter().filter(|i| i.standing).count();
    println!(
        "corpus={} family={} panels={} stories={} standing={} weekly={}",
        out.corpus,
        out.family,
        out.doc.panels.len(),
        out.doc.stories.len(),
        standing,
        out.items.len() - standing
    );
    for item in out.items.iter().take(6) {
        println!("  [{}] {}", if item.standing { "stands" } else { "weekly" }, item.label);
    }
}
