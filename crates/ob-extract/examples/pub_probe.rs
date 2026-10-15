//! Pub probe: open a .pub file's compound container and list
//! its streams. Phase 1 of the native reader.
//! Usage: pub_probe <file.pub>
use ob_extract::cfb::Cfb;

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
            if let Ok(contents) = cfb.read_stream("Contents") {
                println!("Contents stream: {} bytes", contents.len());
                println!("first 4 bytes: {:02x?}", &contents[0..4.min(contents.len())]);
            }
        }
        Err(e) => println!("ERROR: {e}"),
    }
}
