//! File format v0: a single JSON document with a manifest header.
//!
//! JSON on purpose: diffable, scriptable, survivable, and emailable. The v1
//! plan (docs/FORMAT.md) wraps this exact document JSON in a zip container
//! with content-addressed assets; readers must accept every older version.

use ob_model::Document;
use std::path::Path;

pub fn save(doc: &Document, path: &Path) -> Result<(), String> {
    let json = serde_json::to_string_pretty(doc).map_err(|e| e.to_string())?;
    // Atomic save: write to a sibling temp file, then rename over the target.
    // A bulletin lost on Saturday night is a trust-ending event.
    let tmp = path.with_extension("obd.tmp");
    std::fs::write(&tmp, json).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn load(path: &Path) -> Result<Document, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let doc: Document = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    if doc.format_version > ob_model::FORMAT_VERSION {
        return Err(format!(
            "document format {} is newer than this app understands ({})",
            doc.format_version,
            ob_model::FORMAT_VERSION
        ));
    }
    Ok(doc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ob_model::*;

    #[test]
    fn roundtrip() {
        let doc = Document {
            format_version: FORMAT_VERSION,
            title: "Roundtrip".into(),
            profile: ChurchProfile {
                name: "Grace".into(),
                ..Default::default()
            },
            fields: Default::default(),
            styles: default_styles(),
            stories: vec![Story {
                id: "s".into(),
                paragraphs: vec![Paragraph::new("body", "Hello")],
            }],
            panels: vec![Panel {
                background_image: None,
                id: "p".into(),
                name: "P".into(),
                size: Size::new(1, 2),
                frames: vec![],
            }],
            sheet: SheetSpec {
                preset: "letter-portrait".into(),
                size: Size::new(1, 2),
            },
        };
        let path = std::env::temp_dir().join("ob-roundtrip-test.obd");
        save(&doc, &path).unwrap();
        let back = load(&path).unwrap();
        assert_eq!(doc, back);
        let _ = std::fs::remove_file(path);
    }
}
