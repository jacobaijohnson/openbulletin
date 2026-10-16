//! ob-verovio-helper: the separate process the app spawns
//! to have music engraved. Reads one JSON request on
//! stdin (`{protocol, command, musicxml, width_mm,
//! staff_mm}`), writes one JSON response on stdout
//! (systems with their SVGs and measured staff sizes, or
//! a plain-words refusal). Exit 0 whenever a JSON answer
//! was given, including refusals; exit 2 when the request
//! itself could not be read.
//!
//! The engraver behind it (Verovio, LGPL-3.0) is found
//! and driven by ob-music's engine room; this binary is
//! deliberately thin, the same shape as ob-pdfium-helper.

use std::io::Read as _;

fn main() {
    let mut input = String::new();
    if std::io::stdin().read_to_string(&mut input).is_err() {
        eprintln!("ob-verovio-helper: cannot read the request");
        std::process::exit(2);
    }
    let req: ob_music::engine::EngineRequest = match serde_json::from_str(&input) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("ob-verovio-helper: the request is not readable JSON ({e})");
            std::process::exit(2);
        }
    };
    let resp = ob_music::engine::run(&req);
    match serde_json::to_string(&resp) {
        Ok(text) => println!("{text}"),
        Err(e) => {
            eprintln!("ob-verovio-helper: cannot answer ({e})");
            std::process::exit(2);
        }
    }
}
