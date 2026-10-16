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

use ob_model::{Asset, AssetKind, Document, Insert, RightsRecord};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
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

/// The words for a container whose actual insides pass
/// the total cap, whichever container it is.
const TOTAL_TOO_LARGE: &str = "That file's insides are too large to read safely.";

/// Read one zip entry's bytes through a cap, counting
/// ACTUAL bytes as they arrive (round 3, RT3-B). The
/// loaders used to cap by summing the sizes the file's
/// own directory declares, and a directory can lie: a
/// 612 KB file whose entries claimed 5 bytes expanded
/// to 600 MB in memory on open, because zip streams are
/// not bound by their declared size. The reading itself
/// is the count now, the IDML importer's discipline:
/// one byte past the cap is read, so an entry exactly
/// at the cap still counts as inside; anything more
/// refuses in plain words, and memory never holds more
/// than the cap plus that one byte for this entry.
fn read_entry_capped(
    entry: &mut zip::read::ZipFile,
    cap: u64,
    too_large: &str,
) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    entry
        .by_ref()
        .take(cap + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > cap {
        return Err(too_large.to_string());
    }
    Ok(bytes)
}

/// The text variant of `read_entry_capped`, for the
/// container's JSON entry.
fn read_entry_text_capped(
    entry: &mut zip::read::ZipFile,
    cap: u64,
    too_large: &str,
) -> Result<String, String> {
    let bytes = read_entry_capped(entry, cap, too_large)?;
    String::from_utf8(bytes).map_err(|_| "that file does not hold text we can read".to_string())
}

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
    doc.migrate_participants();
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
    // The total cap counts actual bytes as each entry is
    // read (round 3, RT3-B): the directory's declared
    // sizes are never summed, because a directory can
    // lie about them. `total` never passes the cap:
    // every read is capped at what is left of it.
    let mut total: u64 = 0;
    let mut doc: Document = {
        let mut entry = zip
            .by_name("document.json")
            .map_err(|_| "that file has no bulletin inside it (no document.json)".to_string())?;
        let text = read_entry_text_capped(
            &mut entry,
            MAX_JSON_BYTES,
            "that file is far larger than a bulletin ever is",
        )?;
        total += text.len() as u64;
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
        let bytes = read_entry_capped(&mut entry, MAX_TOTAL_UNCOMPRESSED - total, TOTAL_TOO_LARGE)?;
        total += bytes.len() as u64;
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

/// The root of an .obdi file: the sheet itself, plus the
/// asset records of the pictures it uses (their bytes
/// ride as `assets/<sha256>` entries, exactly as in a
/// bulletin file). The records travel so a sheet that
/// arrives from another church keeps the credit lines
/// its pictures are owed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct InsertPackage {
    insert: Insert,
    #[serde(default)]
    assets: Vec<Asset>,
}

/// Save one loose sheet as an .obdi file: the v1
/// container discipline under another name (roles build
/// R-3). Exactly the assets the sheet's frames use are
/// packed, from the host document's store; a referenced
/// picture whose bytes the host does not hold is left
/// out, and the sheet keeps saying that picture is
/// missing, as it did in the bulletin. The write is
/// atomic, like a bulletin's.
pub fn save_insert(insert: &Insert, host: &Document, path: &Path) -> Result<(), String> {
    let referenced = insert.referenced_assets();
    let package = InsertPackage {
        insert: insert.clone(),
        assets: host
            .assets
            .iter()
            .filter(|a| referenced.contains(&a.sha256))
            .cloned()
            .collect(),
    };
    let json = serde_json::to_string_pretty(&package).map_err(|e| e.to_string())?;
    let mut buf: Vec<u8> = Vec::new();
    {
        let cursor = std::io::Cursor::new(&mut buf);
        let mut zip = zip::ZipWriter::new(cursor);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        zip.start_file("insert.json", options)
            .map_err(|e| e.to_string())?;
        zip.write_all(json.as_bytes()).map_err(|e| e.to_string())?;
        for hash in &referenced {
            let Some(bytes) = host.asset_data(hash) else {
                continue;
            };
            zip.start_file(format!("assets/{hash}"), options)
                .map_err(|e| e.to_string())?;
            zip.write_all(bytes).map_err(|e| e.to_string())?;
        }
        zip.finish().map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension("obdi.tmp");
    std::fs::write(&tmp, buf).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())?;
    Ok(())
}

/// Load an .obdi file: the sheet, the asset records it
/// carried, and the picture bytes it packed, by hash.
/// The .obd discipline throughout: the total is capped
/// by counting actual bytes as entries are read (the
/// directory's declared sizes are never trusted —
/// round 3, RT3-B), and every asset entry's bytes are
/// checked against the fingerprint in its name before
/// anything is trusted; a mismatch refuses in plain
/// words.
/// What a sheet file carries out: the sheet itself,
/// the records of the pictures it uses, and their
/// bytes keyed by fingerprint.
pub type LoadedInsert = (Insert, Vec<Asset>, BTreeMap<String, Vec<u8>>);

pub fn load_insert(path: &Path) -> Result<LoadedInsert, String> {
    let head = read_head(path)?;
    if head != ZIP_MAGIC {
        return Err("that file is not a loose sheet file (.obdi)".to_string());
    }
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut zip =
        zip::ZipArchive::new(file).map_err(|e| format!("that file's container is damaged: {e}"))?;
    // Actual bytes, counted as they are read — the same
    // discipline as the bulletin loader (round 3, RT3-B).
    let mut total: u64 = 0;
    let mut package: InsertPackage = {
        let mut entry = zip
            .by_name("insert.json")
            .map_err(|_| "that file has no loose sheet inside it (no insert.json)".to_string())?;
        let text = read_entry_text_capped(
            &mut entry,
            MAX_JSON_BYTES,
            "that file is far larger than a loose sheet ever is",
        )?;
        total += text.len() as u64;
        serde_json::from_str(&text).map_err(|e| e.to_string())?
    };
    let mut bytes_by_hash: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| e.to_string())?;
        let name = entry.name().to_string();
        let Some(hash) = name.strip_prefix("assets/") else {
            continue;
        };
        let hash = hash.to_string();
        let bytes = read_entry_capped(&mut entry, MAX_TOTAL_UNCOMPRESSED - total, TOTAL_TOO_LARGE)?;
        total += bytes.len() as u64;
        if ob_model::sha256_hex(&bytes) != hash {
            let short: String = hash.chars().take(12).collect();
            return Err(format!(
                "a picture inside that file does not match its fingerprint ({short}) \
                 so the file was not opened; it may be damaged."
            ));
        }
        bytes_by_hash.insert(hash, bytes);
    }
    // Bytes whose record did not travel still belong to
    // the sheet; give them an honest record rather than
    // dropping them, the same rule as a bulletin file.
    for hash in bytes_by_hash.keys() {
        if !package.assets.iter().any(|a| &a.sha256 == hash) {
            package.assets.push(Asset {
                sha256: hash.clone(),
                kind: AssetKind::Other,
                media_type: "application/octet-stream".to_string(),
                rights: RightsRecord {
                    source: "inside the sheet file".to_string(),
                    ..Default::default()
                },
            });
        }
    }
    // Geometry from a file gets the same clamping the
    // bulletin loader promises: a hostile .obdi must not
    // plant numbers the writer cannot print.
    Ok((package.insert.sanitized(), package.assets, bytes_by_hash))
}

/// One sidecar picture's size cap. A tracing picture is a
/// page render; tens of megabytes is generous, and a
/// "picture" bigger than this is not one (audit Batch 1).
const MAX_SIDECAR_BYTES: u64 = 64 * 1024 * 1024;

/// The media type a picture's own first bytes state, or
/// None when the bytes are none of the four: a text file
/// renamed .png is not a picture and is never gathered
/// in (audit Batch 1).
fn picture_media_type(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        Some("image/png")
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("image/jpeg")
    } else if bytes.starts_with(b"GIF8") {
        Some("image/gif")
    } else if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some("image/webp")
    } else {
        None
    }
}

/// Gather a panel's tracing picture into the asset store
/// when its reference is a sidecar path that resolves beside
/// the bulletin file (a v0 file, or a v1 file written by an
/// early draft of the format). A reference already naming a
/// stored asset stays as it is; a path that does not resolve
/// stays a path too, so the app keeps saying the picture is
/// missing, exactly as it always has.
///
/// The gathering is fenced (audit Batch 1): the sidecar
/// must resolve INSIDE the bulletin's own folder tree,
/// must be no larger than a page picture ever is, and its
/// bytes must actually be a picture. Before the fence, a
/// file that named "../secret.png" (or an absolute path to
/// anything at all) had that file read into the asset
/// store on load, silently, ready to travel with the next
/// save to every recipient.
fn absorb_sidecar_pictures(doc: &mut Document, obd_path: &Path) {
    let base = obd_path.parent().unwrap_or_else(|| Path::new("."));
    let Ok(base) = std::fs::canonicalize(base) else {
        return;
    };
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
        // Canonicalizing settles "..", "." and symlinks in
        // one step: what remains must sit inside the
        // bulletin's own folder tree.
        let Ok(candidate) = std::fs::canonicalize(base.join(reference)) else {
            continue;
        };
        if !candidate.starts_with(&base) {
            continue;
        }
        let Ok(meta) = std::fs::metadata(&candidate) else {
            continue;
        };
        if !meta.is_file() || meta.len() > MAX_SIDECAR_BYTES {
            continue;
        }
        let Ok(bytes) = std::fs::read(&candidate) else {
            continue;
        };
        if bytes.is_empty() || bytes.len() as u64 > MAX_SIDECAR_BYTES {
            continue;
        }
        let Some(media_type) = picture_media_type(&bytes) else {
            continue;
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
            wanted_fonts: Vec::new(),
            participants: Default::default(),
            inserts: Default::default(),
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
    fn a_file_written_before_the_participants_map_loads_with_its_names() {
        // R-2b: an old file keeps its serving names in
        // `fields` under their slugs and has no
        // `participants` key at all. Loading folds them
        // into the map, and the tokens resolve.
        let mut doc = test_doc();
        doc.fields
            .insert("readers".to_string(), "Ana Ruiz".to_string());
        doc.fields
            .insert("ushers".to_string(), "Tom and Bea".to_string());
        let mut json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&doc).expect("json")).expect("value");
        json.as_object_mut().expect("object").remove("participants");
        let dir = scratch("parts");
        let path = dir.join("old.obd");
        std::fs::write(&path, json.to_string()).expect("write");
        let loaded = load(&path).expect("loads");
        assert_eq!(loaded.participant_value("readers"), "Ana Ruiz");
        assert_eq!(
            ob_model::resolve_fields("Readers: {Sunday.readers}", &loaded),
            "Readers: Ana Ruiz"
        );
        assert!(
            !loaded.fields.contains_key("readers"),
            "the fold moved the name out of fields"
        );
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
        // A real PNG's first bytes (audit Batch 1 requires
        // them: text bytes wearing a .png name no longer
        // count as a picture).
        const PNG_BYTES: &[u8] = b"\x89PNG\r\n\x1a\npage-one-render";
        std::fs::write(dir.join("assets/page-01.png"), PNG_BYTES).unwrap();
        let path = dir.join("old.obd");
        ob_format_save_helper(&path, &doc);
        let loaded = load(&path).expect("v0 loads");
        let hash = ob_model::sha256_hex(PNG_BYTES);
        assert_eq!(
            loaded.panels[0].background_image.as_deref(),
            Some(hash.as_str()),
            "the panel now names the asset, not the path"
        );
        assert_eq!(loaded.asset_data(&hash), Some(PNG_BYTES));
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
        assert_eq!(back.asset_data(&hash), Some(PNG_BYTES));
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&moved_dir);
    }

    #[test]
    fn sidecars_outside_the_folder_or_not_pictures_are_never_gathered() {
        // Audit Batch 1: a hostile "old bulletin" must not
        // be able to pull files off the computer into
        // itself. Three shapes: a ".." walk to a canary
        // outside the folder, an absolute path to the same
        // canary, and text bytes wearing a picture's name
        // inside the folder. None may be gathered.
        const PNG_BYTES: &[u8] = b"\x89PNG\r\n\x1a\ncanary";
        let root = scratch("fence");
        let sub = root.join("sub");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(root.join("secret.png"), PNG_BYTES).unwrap();
        std::fs::write(sub.join("fake.png"), b"donor list, page one").unwrap();

        let write_doc = |reference: String| -> std::path::PathBuf {
            let mut doc = test_doc();
            doc.format_version = 0;
            doc.panels[0].background_image = Some(reference);
            let path = sub.join("old.obd");
            ob_format_save_helper(&path, &doc);
            path
        };

        // The ".." walk out of the folder.
        let path = write_doc("../secret.png".to_string());
        let loaded = load(&path).expect("loads");
        assert_eq!(
            loaded.panels[0].background_image.as_deref(),
            Some("../secret.png"),
            "the reference stays a path; nothing was gathered"
        );
        assert!(loaded.asset_bytes.is_empty());

        // The absolute path to the same canary.
        let absolute = root.join("secret.png").to_string_lossy().into_owned();
        let path = write_doc(absolute.clone());
        let loaded = load(&path).expect("loads");
        assert_eq!(
            loaded.panels[0].background_image.as_deref(),
            Some(absolute.as_str())
        );
        assert!(loaded.asset_bytes.is_empty());

        // Text bytes with a picture's name, inside the folder.
        let path = write_doc("fake.png".to_string());
        let loaded = load(&path).expect("loads");
        assert_eq!(
            loaded.panels[0].background_image.as_deref(),
            Some("fake.png")
        );
        assert!(loaded.asset_bytes.is_empty());

        // And the honest case still gathers: a real picture
        // sitting beside the bulletin under a plain name.
        std::fs::write(sub.join("page.png"), PNG_BYTES).unwrap();
        let path = write_doc("page.png".to_string());
        let loaded = load(&path).expect("loads");
        let hash = ob_model::sha256_hex(PNG_BYTES);
        assert_eq!(
            loaded.panels[0].background_image.as_deref(),
            Some(hash.as_str())
        );
        assert_eq!(loaded.asset_data(&hash), Some(PNG_BYTES));
        let _ = std::fs::remove_dir_all(&root);
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

    fn sheet_with_picture(host: &mut Document) -> (Insert, String) {
        let hash = host.add_asset(
            b"sheet-picture-bytes",
            AssetKind::Picture,
            "image/png",
            RightsRecord {
                source: "the church camera".into(),
                license: LicenseKind::ChurchOwned,
                ..Default::default()
            },
        );
        let ins = Insert {
            id: "sheet-1".into(),
            name: "Prayer list".into(),
            life: ob_model::InsertLife::ThisWeekOnly,
            keep_through: None,
            panels: vec![Panel {
                background_image: None,
                id: "sheet-1-p1".into(),
                name: "Prayer list".into(),
                size: Size::new(396_000, 612_000),
                frames: vec![Frame {
                    id: "sheet-1-f1".into(),
                    name: "Photo".into(),
                    kind: FrameKind::Picture,
                    rect: Rect::new(0, 0, 100_000, 100_000),
                    story_id: None,
                    role: ContentRole::Standing,
                    picture: Some(PicturePlacement::new(hash.clone())),
                    music: None,
                }],
                guides: vec![],
            }],
            stories: vec![Story {
                id: "sheet-1-s1".into(),
                paragraphs: vec![Paragraph::new("body", "Pray for us")],
                keep_through: None,
            }],
            styles: default_styles(),
            sheet: ob_model::InsertSheet {
                preset: "half-letter".into(),
                copies: 60,
                paper_note: "blue paper".into(),
            },
        };
        (ins, hash)
    }

    #[test]
    fn an_obdi_sheet_round_trips_with_its_picture() {
        let dir = scratch("obdi");
        let mut host = test_doc();
        let (ins, hash) = sheet_with_picture(&mut host);
        // A picture the sheet does not use must not travel.
        host.add_asset(
            b"unrelated-bytes",
            AssetKind::Picture,
            "image/png",
            RightsRecord::default(),
        );
        let path = dir.join("Prayer list.obdi");
        save_insert(&ins, &host, &path).unwrap();
        let head = std::fs::read(&path).unwrap();
        assert_eq!(&head[..4], b"PK\x03\x04", "an .obdi is the v1 container");
        let (back, assets, bytes) = load_insert(&path).unwrap();
        assert_eq!(back, ins);
        assert_eq!(
            bytes.get(&hash).map(|b| b.as_slice()),
            Some(b"sheet-picture-bytes".as_slice())
        );
        assert_eq!(bytes.len(), 1, "only the used picture travels");
        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].sha256, hash);
        assert_eq!(assets[0].rights.source, "the church camera");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_obdi_with_swapped_picture_bytes_is_refused() {
        let dir = scratch("obdi-tamper");
        let mut host = test_doc();
        let (ins, hash) = sheet_with_picture(&mut host);
        let path = dir.join("sheet.obdi");
        save_insert(&ins, &host, &path).unwrap();
        // Rewrite the zip with the asset entry's bytes
        // swapped: the entry still answers to the old hash.
        let file = std::fs::File::open(&path).unwrap();
        let mut zip = zip::ZipArchive::new(file).unwrap();
        let mut insert_json = String::new();
        zip.by_name("insert.json")
            .unwrap()
            .read_to_string(&mut insert_json)
            .unwrap();
        drop(zip);
        let bad = dir.join("bad.obdi");
        {
            let cursor = std::io::Cursor::new(Vec::new());
            let mut zw = zip::ZipWriter::new(cursor);
            let opts = zip::write::SimpleFileOptions::default();
            zw.start_file("insert.json", opts).unwrap();
            zw.write_all(insert_json.as_bytes()).unwrap();
            zw.start_file(format!("assets/{hash}"), opts).unwrap();
            zw.write_all(b"swapped-bytes").unwrap();
            let cursor = zw.finish().unwrap();
            std::fs::write(&bad, cursor.into_inner()).unwrap();
        }
        let err = load_insert(&bad).expect_err("tampered bytes refuse");
        assert!(err.contains("does not match its fingerprint"), "err: {err}");
        // And a file that is not a container at all is
        // refused in plain words, not misparsed.
        let not_zip = dir.join("plain.obdi");
        std::fs::write(&not_zip, b"just some text").unwrap();
        let err = load_insert(&not_zip).expect_err("not a sheet file");
        assert!(err.contains("not a loose sheet"), "err: {err}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_bulletin_written_before_loose_sheets_loads_with_none() {
        // The inserts key is simply absent from an old
        // file; loading must come back with no sheets.
        let mut doc = test_doc();
        doc.inserts = vec![];
        let mut json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&doc).expect("json")).expect("value");
        json.as_object_mut().expect("object").remove("inserts");
        let dir = scratch("noinserts");
        let path = dir.join("old.obd");
        std::fs::write(&path, json.to_string()).expect("write");
        let loaded = load(&path).expect("loads");
        assert!(loaded.inserts.is_empty());
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

    /// Find a byte pattern in a slice (fixture plumbing).
    fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        haystack.windows(needle.len()).position(|w| w == needle)
    }

    /// Rewrite the declared uncompressed size of one
    /// entry, in both its local header and its central
    /// directory record, leaving the real bytes and the
    /// honest checksum untouched: the red team's lie
    /// (round 3, RT3-B). The declared field sits at a
    /// fixed distance ahead of the entry's name in each
    /// record, 12 bytes in a local header and 22 in a
    /// central one, and the record's signature says
    /// which record a name belongs to.
    fn patch_declared_size(zip_bytes: &mut [u8], entry_name: &str, declared: u32) {
        let name = entry_name.as_bytes();
        let mut patched = 0;
        let mut from = 0;
        while let Some(found) = find_bytes(&zip_bytes[from..], name) {
            let at = from + found;
            if at >= 30 && &zip_bytes[at - 30..at - 26] == b"PK\x03\x04" {
                zip_bytes[at - 12..at - 8].copy_from_slice(&declared.to_le_bytes());
                patched += 1;
            } else if at >= 46 && &zip_bytes[at - 46..at - 42] == b"PK\x01\x02" {
                zip_bytes[at - 22..at - 18].copy_from_slice(&declared.to_le_bytes());
                patched += 1;
            }
            from = at + 1;
        }
        assert_eq!(patched, 2, "both records for {entry_name} were patched");
    }

    #[test]
    fn capped_reads_count_actual_bytes_not_declared_ones() {
        // One 100-byte entry in a fresh in-memory zip
        // each time. A cap of 100 reads it whole; a cap
        // of 50 refuses it, and patching the declared
        // size down to 5 changes neither outcome,
        // because the count follows the bytes that
        // actually arrive.
        let honest = || {
            let mut buf = Vec::new();
            {
                let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
                zip.start_file(
                    "entry.bin",
                    zip::write::SimpleFileOptions::default()
                        .compression_method(zip::CompressionMethod::Deflated),
                )
                .expect("start");
                zip.write_all(&[7u8; 100]).expect("write");
                zip.finish().expect("finish");
            }
            buf
        };
        let mut lying = honest();
        patch_declared_size(&mut lying, "entry.bin", 5);
        for bytes in [honest(), lying] {
            let mut zip =
                zip::ZipArchive::new(std::io::Cursor::new(&bytes)).expect("archive opens");
            let mut entry = zip.by_index(0).expect("entry");
            assert_eq!(
                read_entry_capped(&mut entry, 100, "too large")
                    .expect("at the cap reads")
                    .len(),
                100
            );
            drop(entry);
            let mut entry = zip.by_index(0).expect("entry again");
            let err =
                read_entry_capped(&mut entry, 50, "too large").expect_err("past the cap refuses");
            assert_eq!(err, "too large");
        }
    }

    #[test]
    fn a_bulletin_whose_directory_lies_about_its_size_is_refused() {
        // The red team's .obd bomb: one asset of zeroes
        // a byte past the whole-container cap, deflated
        // down to a small file, with its declared size
        // patched down to 5 bytes. The old loader summed
        // the declarations, saw a tiny file, and read
        // the whole thing into memory. The loader now
        // counts actual bytes and refuses.
        let zeros = vec![0u8; (MAX_TOTAL_UNCOMPRESSED + 1) as usize];
        let mut doc = test_doc();
        let hash = doc.add_asset(
            &zeros,
            AssetKind::Picture,
            "image/png",
            RightsRecord::default(),
        );
        let dir = scratch("bomb-obd");
        let honest_path = dir.join("honest.obd");
        save(&doc, &honest_path).expect("save");
        assert!(load(&honest_path).is_err(), "even honest, it is too big");
        let mut bytes = std::fs::read(&honest_path).expect("read");
        let entry_name = format!("assets/{hash}");
        patch_declared_size(&mut bytes, &entry_name, 5);
        let lying_path = dir.join("lying.obd");
        std::fs::write(&lying_path, &bytes).expect("write lying copy");
        let err = load(&lying_path).expect_err("the lie no longer opens it");
        assert_eq!(err, TOTAL_TOO_LARGE);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_loose_sheet_whose_directory_lies_about_its_size_is_refused() {
        // The same bomb through the .obdi door.
        let zeros = vec![0u8; (MAX_TOTAL_UNCOMPRESSED + 1) as usize];
        let mut host = test_doc();
        let hash = host.add_asset(
            &zeros,
            AssetKind::Picture,
            "image/png",
            RightsRecord::default(),
        );
        let insert = Insert {
            id: "sheet-big".into(),
            name: "Big art".into(),
            life: ob_model::InsertLife::ThisWeekOnly,
            keep_through: None,
            panels: vec![Panel {
                background_image: None,
                id: "sheet-big-p1".into(),
                name: "Big art".into(),
                size: Size::new(396_000, 612_000),
                frames: vec![Frame {
                    id: "sheet-big-f1".into(),
                    name: "Photo".into(),
                    kind: FrameKind::Picture,
                    rect: Rect::new(0, 0, 100_000, 100_000),
                    story_id: None,
                    role: ContentRole::Standing,
                    picture: Some(PicturePlacement::new(hash.clone())),
                    music: None,
                }],
                guides: vec![],
            }],
            stories: vec![],
            styles: default_styles(),
            sheet: ob_model::InsertSheet {
                preset: "half-letter".into(),
                copies: 1,
                paper_note: String::new(),
            },
        };
        let dir = scratch("bomb-obdi");
        let honest_path = dir.join("honest.obdi");
        save_insert(&insert, &host, &honest_path).expect("save");
        let mut bytes = std::fs::read(&honest_path).expect("read");
        let entry_name = format!("assets/{hash}");
        patch_declared_size(&mut bytes, &entry_name, 5);
        let lying_path = dir.join("lying.obdi");
        std::fs::write(&lying_path, &bytes).expect("write lying copy");
        let err = load_insert(&lying_path).expect_err("the lie no longer opens it");
        assert_eq!(err, TOTAL_TOO_LARGE);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
