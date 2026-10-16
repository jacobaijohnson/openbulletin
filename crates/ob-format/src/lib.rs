//! The .obd file format (docs/FORMAT.md).
//!
//! v1: a zip container holding `document.json` (the same
//! document JSON v0 wrote bare) plus one entry per asset,
//! `assets/<sha256>`, so the pictures a bulletin carries
//! travel inside the file and arrive identically on the next
//! volunteer's machine.
//!
//! v0: a single JSON document. Readers accept every older
//! version and migrate in memory: the pre-occasions field
//! keys are renamed, and a panel's tracing picture that
//! names a sidecar file beside the .obd is read into the
//! asset store. Saving always writes v1.
//!
//! Saves are atomic (temp file + rename) in both versions.

use ob_model::{AssetKind, Document, RightsRecord};
use std::io::{Read, Write};
use std::path::Path;

/// document.json stays small; refuse absurd sizes before
/// reading them into memory (closing audit).
const MAX_JSON_BYTES: u64 = 64 * 1024 * 1024;
/// Everything inside a v1 file, uncompressed, stays under
/// this. A bulletin with its pictures is megabytes, not
/// hundreds of megabytes.
const MAX_TOTAL_UNCOMPRESSED: u64 = 512 * 1024 * 1024;

/// The first four bytes of a zip container ("PK" + 0x03 0x04).
const ZIP_MAGIC: [u8; 4] = [b'P', b'K', 0x03, 0x04];

pub fn save(doc: &Document, path: &Path) -> Result<(), String> {
    let mut doc = doc.clone();
    // Save what the document will say it is: the current
    // format version, whichever version it was loaded from.
    doc.format_version = ob_model::FORMAT_VERSION;
    let json = serde_json::to_string_pretty(&doc).map_err(|e| e.to_string())?;
    // Zip into memory, then the atomic write below.
    let mut buf: Vec<u8> = Vec::new();
    {
        let cursor = std::io::Cursor::new(&mut buf);
        let mut zip = zip::ZipWriter::new(cursor);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        zip.start_file("document.json", options)
            .map_err(|e| e.to_string())?;
        zip.write_all(json.as_bytes()).map_err(|e| e.to_string())?;
        // Asset bytes ride under their hash. Every byte set
        // in the map is written, whether or not its metadata
        // row survived an edit: bytes without a row get one
        // on load, never a silent drop here.
        let mut hashes: Vec<&String> = doc.asset_bytes.keys().collect();
        hashes.sort();
        for hash in hashes {
            zip.start_file(format!("assets/{hash}"), options)
                .map_err(|e| e.to_string())?;
            zip.write_all(&doc.asset_bytes[hash])
                .map_err(|e| e.to_string())?;
        }
        zip.finish().map_err(|e| e.to_string())?;
    }
    // Atomic save: write to a sibling temp file, then rename over the target.
    // A bulletin lost on Saturday night is a trust-ending event.
    let tmp = path.with_extension("obd.tmp");
    std::fs::write(&tmp, buf).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn load(path: &Path) -> Result<Document, String> {
    let head = read_head(path)?;
    let mut doc = if head == ZIP_MAGIC {
        load_v1(path)?
    } else {
        load_v0_json(path)?
    };
    doc.migrate_occasion_keys();
    absorb_sidecar_pictures(&mut doc, path);
    Ok(doc.sanitized())
}

/// The first bytes of the file: enough to tell a zip (v1)
/// from bare JSON (v0) by the container, never the name.
fn read_head(path: &Path) -> Result<Vec<u8>, String> {
    let mut f = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut head = vec![0u8; 4];
    let n = f.read(&mut head).map_err(|e| e.to_string())?;
    head.truncate(n);
    Ok(head)
}

fn check_version(doc: &Document) -> Result<(), String> {
    if doc.format_version > ob_model::FORMAT_VERSION {
        return Err(format!(
            "document format {} is newer than this app understands ({})",
            doc.format_version,
            ob_model::FORMAT_VERSION
        ));
    }
    Ok(())
}

/// A v0 file: bare JSON, size-guarded as it always was.
fn load_v0_json(path: &Path) -> Result<Document, String> {
    if let Ok(md) = std::fs::metadata(path) {
        if md.len() > MAX_JSON_BYTES {
            return Err("that file is far larger than a bulletin ever is".to_string());
        }
    }
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let doc: Document = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    check_version(&doc)?;
    Ok(doc)
}

/// A v1 file: document.json plus its asset entries. Every
/// asset entry's bytes are checked against the hash in its
/// name before anything is trusted: a picture that does not
/// match its fingerprint is refused in plain words, never
/// silently used.
fn load_v1(path: &Path) -> Result<Document, String> {
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut zip =
        zip::ZipArchive::new(file).map_err(|e| format!("that file's container is damaged: {e}"))?;
    // The total cap is checked from the directory before a
    // single entry is decompressed.
    let mut total: u64 = 0;
    for i in 0..zip.len() {
        let entry = zip.by_index(i).map_err(|e| e.to_string())?;
        total = total.saturating_add(entry.size());
        if total > MAX_TOTAL_UNCOMPRESSED {
            return Err(
                "what is inside that file adds up to far more than a bulletin ever holds; \
                 it was not opened."
                    .to_string(),
            );
        }
    }
    let mut doc: Document = {
        let mut entry = zip
            .by_name("document.json")
            .map_err(|_| "that file has no bulletin inside it (no document.json)".to_string())?;
        if entry.size() > MAX_JSON_BYTES {
            return Err("that file is far larger than a bulletin ever is".to_string());
        }
        let mut text = String::new();
        entry.read_to_string(&mut text).map_err(|e| e.to_string())?;
        serde_json::from_str(&text).map_err(|e| e.to_string())?
    };
    check_version(&doc)?;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| e.to_string())?;
        let name = entry.name().to_string();
        let Some(hash) = name.strip_prefix("assets/") else {
            continue;
        };
        let hash = hash.to_string();
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).map_err(|e| e.to_string())?;
        if ob_model::sha256_hex(&bytes) != hash {
            let short: String = hash.chars().take(12).collect();
            return Err(format!(
                "a picture inside that file does not match its fingerprint ({short}) \
                 so the file was not opened; it may be damaged."
            ));
        }
        if !doc.assets.iter().any(|a| a.sha256 == hash) {
            // Bytes whose metadata row is missing still
            // belong to the bulletin; give them an honest
            // record rather than dropping them.
            doc.assets.push(ob_model::Asset {
                sha256: hash.clone(),
                kind: AssetKind::Other,
                media_type: "application/octet-stream".to_string(),
                rights: RightsRecord {
                    source: "inside the bulletin file".to_string(),
                    ..Default::default()
                },
            });
        }
        doc.asset_bytes.insert(hash, bytes);
    }
    Ok(doc)
}

/// Gather a panel's tracing picture into the asset store
/// when its reference is a sidecar path that resolves beside
/// the bulletin file (a v0 file, or a v1 file written by an
/// early draft of the format). A reference already naming a
/// stored asset stays as it is; a path that does not resolve
/// stays a path too, so the app keeps saying the picture is
/// missing, exactly as it always has.
fn absorb_sidecar_pictures(doc: &mut Document, obd_path: &Path) {
    let base = obd_path.parent().unwrap_or_else(|| Path::new("."));
    // First pass (read-only): which panels carry a resolvable
    // sidecar, and what its bytes and type are.
    let mut pending: Vec<(usize, Vec<u8>, &'static str, String)> = Vec::new();
    for (i, panel) in doc.panels.iter().enumerate() {
        let Some(reference) = &panel.background_image else {
            continue;
        };
        if doc.asset_bytes.contains_key(reference) {
            continue;
        }
        let candidate = base.join(reference);
        let Ok(bytes) = std::fs::read(&candidate) else {
            continue;
        };
        if bytes.is_empty() {
            continue;
        }
        let media_type = match candidate
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase()
            .as_str()
        {
            "png" => "image/png",
            "jpg" | "jpeg" => "image/jpeg",
            "gif" => "image/gif",
            "webp" => "image/webp",
            _ => "application/octet-stream",
        };
        pending.push((i, bytes, media_type, reference.clone()));
    }
    // Second pass: store the bytes and point the panel at
    // the asset.
    for (i, bytes, media_type, source) in pending {
        let hash = doc.add_asset(
            &bytes,
            AssetKind::TracingPicture,
            media_type,
            RightsRecord {
                source,
                ..Default::default()
            },
        );
        doc.panels[i].background_image = Some(hash);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ob_model::*;

    fn scratch(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("ob-format-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("dir");
        dir
    }

    fn test_doc() -> Document {
        Document {
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
            assets: Vec::new(),
            asset_bytes: Default::default(),
            occasion_time: None,
        }
    }

    #[test]
    fn extreme_values_are_clamped_on_load() {
        let dir = scratch("x");
        let path = dir.join("hostile.obd");
        let mut doc = test_doc();
        doc.styles = vec![ParagraphStyle {
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
        }];
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
        let doc = test_doc();
        let dir = scratch("rt");
        let path = dir.join("roundtrip.obd");
        save(&doc, &path).unwrap();
        // What landed on disk is the v1 container.
        let head = std::fs::read(&path).unwrap();
        assert_eq!(&head[..4], b"PK\x03\x04", "v1 files are zips");
        let back = load(&path).unwrap();
        assert_eq!(doc, back);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn runs_survive_the_v1_round_trip_in_the_runs_shape() {
        let mut doc = test_doc();
        doc.stories[0].paragraphs = vec![Paragraph::from_runs(
            "body",
            vec![
                Run::plain("Sing "),
                Run {
                    text: "hymn".into(),
                    bold: Some(true),
                    italic: None,
                },
                Run::plain(" 432"),
            ],
        )];
        let dir = scratch("runs");
        let path = dir.join("runs.obd");
        save(&doc, &path).unwrap();
        // The document.json inside the zip speaks runs.
        let file = std::fs::File::open(&path).unwrap();
        let mut zip = zip::ZipArchive::new(file).unwrap();
        let mut json = String::new();
        zip.by_name("document.json")
            .unwrap()
            .read_to_string(&mut json)
            .unwrap();
        assert!(json.contains("\"runs\""), "runs written: {json}");
        let back = load(&path).unwrap();
        assert_eq!(back.stories[0].paragraphs, doc.stories[0].paragraphs);
        assert_eq!(back.stories[0].paragraphs[0].text(), "Sing hymn 432");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn v0_file_with_text_shape_paragraphs_loads_with_words_intact() {
        // A G1a-era (and earlier) file on disk: bare JSON,
        // paragraphs stored as { style, text }.
        let mut doc = test_doc();
        doc.stories[0].paragraphs = vec![Paragraph::new("body", "Welcome to worship")];
        let json = serde_json::to_string(&doc).expect("json");
        assert!(
            json.contains("\"runs\""),
            "the writer today speaks runs even for this doc"
        );
        // Rewrite the story's paragraph in the old shape, as
        // an old file on disk would hold it.
        let old_json = json.replace(
            r#""paragraphs":[{"style":"body","runs":[{"text":"Welcome to worship"}]}]"#,
            r#""paragraphs":[{"style":"body","text":"Welcome to worship"}]"#,
        );
        assert_ne!(old_json, json, "the old shape was substituted");
        let dir = scratch("v0runs");
        let path = dir.join("old.obd");
        std::fs::write(&path, old_json).unwrap();
        let back = load(&path).unwrap();
        assert_eq!(back.stories[0].paragraphs[0].text(), "Welcome to worship");
        assert_eq!(
            back.stories[0].paragraphs[0].runs,
            vec![Run::plain("Welcome to worship")]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn v1_carries_asset_bytes_by_hash_and_v0_resaves_as_v1() {
        let dir = scratch("v1");
        // A v0 file on disk: bare JSON, an asset recorded in
        // its table-less past (v0 JSON could not hold bytes;
        // the record alone is written here by hand).
        let mut doc = test_doc();
        let hash = doc.add_asset(
            b"fake-png-bytes",
            AssetKind::Picture,
            "image/png",
            RightsRecord {
                source: "the church camera".into(),
                license: LicenseKind::ChurchOwned,
                ..Default::default()
            },
        );
        let path = dir.join("week.obd");
        save(&doc, &path).unwrap();
        // The zip holds document.json and the asset entry.
        let file = std::fs::File::open(&path).unwrap();
        let mut zip = zip::ZipArchive::new(file).unwrap();
        assert!(zip.by_name("document.json").is_ok());
        let bytes = {
            let mut entry = zip.by_name(&format!("assets/{hash}")).expect("asset entry");
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes).unwrap();
            bytes
        };
        assert_eq!(bytes, b"fake-png-bytes");
        drop(zip);
        // And it loads back whole: bytes by hash, rights intact.
        let back = load(&path).unwrap();
        assert_eq!(back.asset_data(&hash), Some(b"fake-png-bytes".as_slice()));
        assert_eq!(back.assets.len(), 1);
        assert_eq!(back.assets[0].rights.source, "the church camera");
        assert_eq!(doc, back);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn v0_sidecar_picture_is_gathered_in_and_travels() {
        let dir = scratch("sidecar");
        // A v0 file in the old shape: JSON naming a sidecar
        // picture, the sidecar sitting beside it.
        let mut doc = test_doc();
        doc.format_version = 0;
        doc.panels[0].background_image = Some("assets/page-01.png".into());
        std::fs::create_dir_all(dir.join("assets")).unwrap();
        std::fs::write(dir.join("assets/page-01.png"), b"page-one-render").unwrap();
        let path = dir.join("old.obd");
        ob_format_save_helper(&path, &doc);
        let loaded = load(&path).expect("v0 loads");
        let hash = ob_model::sha256_hex(b"page-one-render");
        assert_eq!(
            loaded.panels[0].background_image.as_deref(),
            Some(hash.as_str()),
            "the panel now names the asset, not the path"
        );
        assert_eq!(
            loaded.asset_data(&hash),
            Some(b"page-one-render".as_slice())
        );
        assert_eq!(loaded.assets[0].kind, AssetKind::TracingPicture);
        // Re-saved (always v1) and moved ALONE to a new home:
        // the picture is still attached, no sidecar needed.
        let moved_dir = scratch("moved");
        let moved = moved_dir.join("week.obd");
        save(&loaded, &moved).unwrap();
        let back = load(&moved).expect("moved file loads");
        assert_eq!(
            back.panels[0].background_image.as_deref(),
            Some(hash.as_str())
        );
        assert_eq!(back.asset_data(&hash), Some(b"page-one-render".as_slice()));
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&moved_dir);
    }

    #[test]
    fn v0_missing_sidecar_keeps_its_path_and_its_honest_note() {
        let dir = scratch("missing");
        let mut doc = test_doc();
        doc.format_version = 0;
        doc.panels[0].background_image = Some("assets/page-01.png".into());
        let path = dir.join("old.obd");
        ob_format_save_helper(&path, &doc);
        let loaded = load(&path).expect("loads without the sidecar");
        // Nothing was invented: the reference stays the path
        // it was, so the app still reports the picture
        // missing instead of pretending it was gathered.
        assert_eq!(
            loaded.panels[0].background_image.as_deref(),
            Some("assets/page-01.png")
        );
        assert!(loaded.asset_bytes.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn tampered_asset_bytes_are_refused() {
        let dir = scratch("tamper");
        let mut doc = test_doc();
        doc.add_asset(
            b"real-bytes",
            AssetKind::Picture,
            "image/png",
            RightsRecord::default(),
        );
        let path = dir.join("week.obd");
        save(&doc, &path).unwrap();
        // Rewrite the zip with the asset entry's bytes
        // swapped: the entry still answers to the old hash.
        let file = std::fs::File::open(&path).unwrap();
        let mut zip = zip::ZipArchive::new(file).unwrap();
        let mut doc_json = String::new();
        zip.by_name("document.json")
            .unwrap()
            .read_to_string(&mut doc_json)
            .unwrap();
        let hash = ob_model::sha256_hex(b"real-bytes");
        drop(zip);
        let bad = dir.join("bad.obd");
        {
            let cursor = std::io::Cursor::new(Vec::new());
            let mut zw = zip::ZipWriter::new(cursor);
            let opts = zip::write::SimpleFileOptions::default();
            zw.start_file("document.json", opts).unwrap();
            zw.write_all(doc_json.as_bytes()).unwrap();
            zw.start_file(format!("assets/{hash}"), opts).unwrap();
            zw.write_all(b"swapped-bytes").unwrap();
            let cursor = zw.finish().unwrap();
            std::fs::write(&bad, cursor.into_inner()).unwrap();
        }
        let err = load(&bad).expect_err("tampered bytes refuse");
        assert!(err.contains("does not match its fingerprint"), "err: {err}");
        let _ = std::fs::remove_dir_all(&dir);
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
            "fields": {"sunday_date": "2026-10-18", "sunday_name": "Pentecost"},
            "styles": [],
            "stories": [{"id": "s", "paragraphs": [{"style": "body", "text": "Hello"}]}],
            "panels": [],
            "sheet": {"preset": "letter-portrait", "size": {"w": 1, "h": 2}}
        }"#;
        let dir = scratch("old");
        let path = dir.join("old.obd");
        std::fs::write(&path, json).expect("write");
        let doc = load(&path).expect("old file loads");
        assert_eq!(doc.stories[0].keep_through, None);
        assert!(!doc.no_announcements);
        // The occasion keys were renamed on the way in.
        assert_eq!(doc.fields["occasion_date"], "2026-10-18");
        assert_eq!(doc.fields["occasion_name"], "Pentecost");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
