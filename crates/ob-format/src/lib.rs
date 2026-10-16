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
    // A bulletin file is small. Refuse absurd sizes before
    // reading them into memory (closing audit).
    const MAX_OBD_BYTES: u64 = 64 * 1024 * 1024;
    if let Ok(md) = std::fs::metadata(path) {
        if md.len() > MAX_OBD_BYTES {
            return Err("that file is far larger than a bulletin ever is".to_string());
        }
    }
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let doc: Document = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    if doc.format_version > ob_model::FORMAT_VERSION {
        return Err(format!(
            "document format {} is newer than this app understands ({})",
            doc.format_version,
            ob_model::FORMAT_VERSION
        ));
    }
    Ok(doc.sanitized())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ob_model::*;

    #[test]
    fn extreme_values_are_clamped_on_load() {
        let dir = std::env::temp_dir().join(format!("ob-format-x-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("dir");
        let path = dir.join("hostile.obd");
        let mut doc = Document {
            format_version: FORMAT_VERSION,
            title: "t".into(),
            profile: ChurchProfile::default(),
            fields: Default::default(),
            styles: vec![ParagraphStyle {
                id: "body".into(),
                name: "Body".into(),
                font_family: "Liberation Sans".into(),
                font_sha256: None,
                font_size: i64::MAX,
                bold: false,
                italic: false,
                align: Align::Left,
                space_after: i64::MAX,
                color: Rgb::BLACK,
            }],
            stories: vec![],
            panels: vec![],
            sheet: SheetSpec {
                preset: "letter-portrait".into(),
                size: Size::new(612_000, 792_000),
            },
            no_announcements: false,
        };
        ob_format_save_helper(&path, &doc);
        let loaded = load(&path).expect("loads");
        assert_eq!(loaded.styles[0].font_size, 288_000);
        assert_eq!(loaded.styles[0].space_after, 144_000);
        doc.format_version = FORMAT_VERSION + 1;
        ob_format_save_helper(&path, &doc);
        assert!(load(&path).is_err(), "newer versions are refused");
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn ob_format_save_helper(path: &std::path::Path, doc: &Document) {
        std::fs::write(path, serde_json::to_string(doc).expect("json")).expect("write");
    }

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
                keep_through: None,
            }],
            panels: vec![Panel {
                background_image: None,
                id: "p".into(),
                name: "P".into(),
                size: Size::new(1, 2),
                frames: vec![],
                guides: vec![],
            }],
            sheet: SheetSpec {
                preset: "letter-portrait".into(),
                size: Size::new(1, 2),
            },
            no_announcements: false,
        };
        let path = std::env::temp_dir().join("ob-roundtrip-test.obd");
        save(&doc, &path).unwrap();
        let back = load(&path).unwrap();
        assert_eq!(doc, back);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn files_written_before_keep_through_and_no_announcements_load() {
        // A v0 file from before the keep-through date and the
        // no-announcements mark existed: neither field is in the
        // JSON, and both must come back as their defaults (no
        // keep-through, announcements not marked none).
        let json = r#"{
            "format_version": 0,
            "title": "Old file",
            "profile": {"name": "Grace", "address": "", "service_times": "",
                        "website": "", "giving_url": "", "pastor": ""},
            "fields": {},
            "styles": [],
            "stories": [{"id": "s", "paragraphs": [{"style": "body", "text": "Hello"}]}],
            "panels": [],
            "sheet": {"preset": "letter-portrait", "size": {"w": 1, "h": 2}}
        }"#;
        let dir = std::env::temp_dir().join(format!("ob-format-old-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("dir");
        let path = dir.join("old.obd");
        std::fs::write(&path, json).expect("write");
        let doc = load(&path).expect("old file loads");
        assert_eq!(doc.stories[0].keep_through, None);
        assert!(!doc.no_announcements);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
