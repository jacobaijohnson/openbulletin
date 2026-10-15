//! Probe: load PDFs with lopdf and report what the scaffold sees.
//! Usage: cargo run -p ob-extract --example probe -- <file.pdf> [...]
use std::process::exit;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("usage: probe <file.pdf> [...]");
        exit(2);
    }
    for path in &args {
        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(e) => {
                println!("{path}\n  READ ERROR: {e}");
                continue;
            }
        };
        match ob_extract::extract_pdf(&bytes, path) {
            Ok(dump) => {
                let text_chars: usize = dump.spans.iter().map(|s| s.text.len()).sum();
                let sizes: Vec<String> = dump
                    .pages
                    .iter()
                    .map(|p| format!("{}x{}", p.width_pt as i32, p.height_pt as i32))
                    .collect();
                println!(
                    "{path}\n  pages: {} [{}]  spans: {}  text chars: {}  picture-only pages: {}",
                    dump.pages.len(),
                    sizes.join(", "),
                    dump.spans.len(),
                    text_chars,
                    dump.pages.iter().filter(|p| p.picture_only).count()
                );
            }
            Err(e) => println!("{path}\n  EXTRACT ERROR: {e}"),
        }
    }
}
