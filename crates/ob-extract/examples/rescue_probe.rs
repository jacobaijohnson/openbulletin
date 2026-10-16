//! Rescue probe: run a PDFium dump JSON through the draft
//! builder and report what the review screen would show.
//! Usage: rescue_probe <dump.json>
use ob_extract::draft::{document_from_dump, Badge};
use ob_extract::ExtractionDump;
use ob_model::ChurchProfile;

fn main() {
    let path = std::env::args().nth(1).expect("dump.json path");
    let text = std::fs::read_to_string(&path).expect("read dump");
    let dump = ExtractionDump::from_json(&text).expect("parse dump");
    let out = document_from_dump(&dump, ChurchProfile::default(), true, &[]);
    let (mut m, mut c, mut pic) = (0, 0, 0);
    for item in &out.items {
        match item.badge {
            Badge::Matched => m += 1,
            Badge::CheckThis => c += 1,
            Badge::PictureOnly => pic += 1,
        }
    }
    let frames: usize = out.doc.panels.iter().map(|p| p.frames.len()).sum();
    println!(
        "preset={} panels={} frames={} stories={} items: {m} matched, {c} check, {pic} picture",
        out.doc.sheet.preset,
        out.doc.panels.len(),
        frames,
        out.doc.stories.len()
    );
    for item in out.items.iter().take(8) {
        println!("  [{:?}] p{} {}", item.badge, item.panel + 1, item.label);
    }
}
