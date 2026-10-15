//! Pub probe: open a .pub file's compound container, list
//! its entries, and read the document's streams by path.
//! Phase 2a of the native reader: the container layer.
//! Usage: pub_probe <file.pub>
use ob_extract::cfb::Cfb;

fn show(cfb: &Cfb, path: &[&str]) {
    match cfb.read_stream_path(path) {
        Ok(bytes) => {
            println!("{}: {} bytes", path.join("/"), bytes.len());
            if path == ["Contents"] {
                println!(
                    "Contents first 4 bytes: {:02x?}",
                    &bytes[0..4.min(bytes.len())]
                );
            }
        }
        Err(e) => println!("{}: not read ({e})", path.join("/")),
    }
}

fn main() {
    let path = std::env::args().nth(1).expect("file.pub path");
    let bytes = std::fs::read(&path).expect("read file");
    match Cfb::open(bytes) {
        Ok(cfb) => {
            for e in cfb.entries() {
                let kind = match e.object_type {
                    5 => "root",
                    1 => "storage",
                    2 => "stream",
                    _ => "?",
                };
                println!("{kind:8} {:40} {} bytes", e.name, e.size);
            }
            show(&cfb, &["Contents"]);
            show(&cfb, &["Quill", "QuillSub", "CONTENTS"]);
            show(&cfb, &["Escher", "EscherStm"]);
        }
        Err(e) => println!("ERROR: {e}"),
    }
}
