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
use std::collections::{BTreeMap, HashSet};
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

/// The most entries one container may hold (round 4,
/// RT-2). The byte caps count what the entries weigh;
/// this counts how many of them there are. A file of
/// 100,001 tiny assets sat under every byte cap and
/// still took minutes to open. A real bulletin or
/// sheet holds a handful of parts; 8,192 is far past
/// generous, and the count is the archive's raw entry
/// count, entries that share a name included (round 5,
/// RT5-5), read from its central directory before any
/// payload is read.
const MAX_ENTRIES: usize = 8_192;

/// The words for a container holding more parts than
/// the entry cap.
const TOO_MANY_PARTS: &str = "That file has too many parts inside to read safely.";

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

/// How many entries a container's central directory
/// lists, counting every record as written: two entries
/// that share a name count twice (round 5, RT5-5).
/// `ZipArchive::len` cannot answer this, because the
/// zip crate keeps entries in a map keyed by name, so
/// its count is the number of distinct names, and a
/// file of half a million entries all named the same
/// reported a count of two.
///
/// The count is WALKED, never read from the
/// directory's own declared totals (round 6, RT6-7):
/// the end record's count fields and the directory's
/// declared size can both lie, in either direction,
/// in the plain record and the zip64 record alike,
/// and a walk that trusted either let floods through.
/// The end record is located the way the zip reader
/// itself locates it, tolerance included: candidates
/// are tried latest first, a comment that ends before
/// the end of the file is accepted (trailing junk),
/// and the directory's real start is the first central
/// header at or after the declared offset, so bytes
/// prepended to the archive shift every offset and
/// the count still lands on the real directory. From
/// there the records are counted one by one, each
/// measured by its own name, extra, and comment
/// lengths, until the end structures begin; no
/// entry's payload is read, and the walk stops as
/// soon as the count passes `cap`, so a flood is
/// refused after reading only the first `cap + 1`
/// small directory records. None when the directory
/// cannot be walked: a damaged file is the archive
/// reader's error to report, in its own words.
fn raw_entry_count(path: &Path, cap: usize) -> Option<usize> {
    use std::io::{Read, Seek, SeekFrom};
    fn read_at(file: &mut std::fs::File, at: u64, buf: &mut [u8]) -> Option<()> {
        file.seek(SeekFrom::Start(at)).ok()?;
        file.read_exact(buf).ok()?;
        Some(())
    }
    /// The first place `sig` occurs in `[from, to)`,
    /// read in chunks so a directory of any size is
    /// scanned without holding it.
    fn find_signature(file: &mut std::fs::File, from: u64, to: u64, sig: &[u8]) -> Option<u64> {
        let mut carry: Vec<u8> = Vec::new();
        let mut pos = from;
        while pos < to {
            let want = ((to - pos).min(65_536)) as usize;
            let mut buf = vec![0u8; want];
            read_at(file, pos, &mut buf)?;
            let base = pos - carry.len() as u64;
            let mut hay = std::mem::take(&mut carry);
            hay.extend_from_slice(&buf);
            if let Some(found) = hay
                .windows(sig.len())
                .position(|w| w == sig)
                .map(|i| base + i as u64)
                .filter(|at| *at >= from && at + sig.len() as u64 <= to)
            {
                return Some(found);
            }
            let keep = (sig.len() - 1).min(hay.len());
            carry = hay[hay.len() - keep..].to_vec();
            pos += want as u64;
        }
        None
    }
    /// The end record's candidate positions, latest
    /// first: every "PK\x05\x06" whose comment length
    /// fits before the end of the file (the reader's
    /// relaxed rule: the comment may end early, which
    /// is what trailing junk means).
    fn end_record_candidates(file: &mut std::fs::File, file_len: u64) -> Vec<u64> {
        let mut out = Vec::new();
        let mut carry: Vec<u8> = Vec::new();
        let mut end = file_len;
        while end > 0 {
            let start = end.saturating_sub(1 << 20);
            let want = (end - start) as usize;
            let mut buf = vec![0u8; want];
            if read_at(file, start, &mut buf).is_none() {
                return out;
            }
            let mut hay = buf;
            hay.extend_from_slice(&carry);
            for pos in (0..hay.len().saturating_sub(21)).rev() {
                if !hay[pos..].starts_with(b"PK\x05\x06") {
                    continue;
                }
                let at = start + pos as u64;
                let comment_len = u64::from(u16::from_le_bytes([hay[pos + 20], hay[pos + 21]]));
                if at + 22 + comment_len <= file_len {
                    out.push(at);
                    if out.len() >= 16 {
                        return out;
                    }
                }
            }
            let keep = 21.min(hay.len());
            carry = hay[..keep].to_vec();
            end = start;
        }
        out
    }
    /// Count the directory records in front of one end
    /// record candidate: None when this candidate's
    /// directory cannot be walked, so the caller can
    /// try an earlier candidate.
    fn count_from_end_record(file: &mut std::fs::File, eocd_at: u64, cap: usize) -> Option<usize> {
        let mut eocd = [0u8; 22];
        read_at(file, eocd_at, &mut eocd)?;
        if !eocd.starts_with(b"PK\x05\x06") {
            return None;
        }
        let total16 = u16::from_le_bytes([eocd[10], eocd[11]]);
        let cd_offset32 = u32::from_le_bytes(eocd[16..20].try_into().ok()?);
        let mut dir_end = eocd_at;
        let mut declared_offset = u64::from(cd_offset32);
        let mut declared_total = u64::from(total16);
        // A zip64 locator in front of the record points
        // at the zip64 record, which carries the
        // directory's real offset when the plain fields
        // are too small to hold it (its total-entries
        // field sits at byte 32, its offset at 48). The
        // reader finds that record by searching forward
        // from the locator's declared offset; its real
        // position is where the directory ends.
        if (total16 == u16::MAX || cd_offset32 == u32::MAX) && eocd_at >= 20 {
            let locator_at = eocd_at - 20;
            let mut locator = [0u8; 20];
            if read_at(file, locator_at, &mut locator).is_some()
                && locator.starts_with(b"PK\x06\x07")
            {
                let record_declared = u64::from_le_bytes(locator[8..16].try_into().ok()?);
                if record_declared < locator_at {
                    if let Some(record_at) =
                        find_signature(file, record_declared, locator_at, b"PK\x06\x06")
                    {
                        let mut record = [0u8; 56];
                        if read_at(file, record_at, &mut record).is_some()
                            && record.starts_with(b"PK\x06\x06")
                        {
                            dir_end = record_at;
                            declared_total = u64::from_le_bytes(record[32..40].try_into().ok()?);
                            declared_offset = u64::from_le_bytes(record[48..56].try_into().ok()?);
                        }
                    }
                }
            }
        }
        // The directory's real start is the first
        // central header at or after the declared
        // offset: prepended bytes shift it forward, and
        // the declared totals (either field, either
        // record) are never consulted again.
        let Some(cd_start) = find_signature(file, declared_offset, dir_end, b"PK\x01\x02") else {
            // No directory records in front of the end
            // record: an honestly empty archive counts
            // zero; anything else is the reader's error
            // to report.
            return if declared_total == 0 { Some(0) } else { None };
        };
        // Walk the records: a fixed 46-byte header, then
        // the name, extra field, and comment its header
        // measures, until the end structures begin.
        let mut count = 0usize;
        let mut at = cd_start;
        while at < dir_end {
            if count > cap {
                return Some(count);
            }
            let mut header = [0u8; 46];
            read_at(file, at, &mut header)?;
            if !header.starts_with(b"PK\x01\x02") {
                return None;
            }
            let name_len = u64::from(u16::from_le_bytes([header[28], header[29]]));
            let extra_len = u64::from(u16::from_le_bytes([header[30], header[31]]));
            let comment_len = u64::from(u16::from_le_bytes([header[32], header[33]]));
            count += 1;
            at += 46 + name_len + extra_len + comment_len;
        }
        if at == dir_end {
            Some(count)
        } else {
            None
        }
    }

    let mut file = std::fs::File::open(path).ok()?;
    let file_len = file.metadata().ok()?.len();
    if file_len < 22 {
        return None;
    }
    for eocd_at in end_record_candidates(&mut file, file_len) {
        if let Some(count) = count_from_end_record(&mut file, eocd_at, cap) {
            return Some(count);
        }
    }
    None
}

/// Refuse a container whose raw entry count passes the
/// cap, in the cap's own words. A count that cannot be
/// walked leaves the file to the archive reader, whose
/// distinct-name count still applies there.
fn check_entry_cap(path: &Path) -> Result<(), String> {
    match raw_entry_count(path, MAX_ENTRIES) {
        Some(n) if n > MAX_ENTRIES => Err(TOO_MANY_PARTS.to_string()),
        _ => Ok(()),
    }
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
    // Records that share a hash fold into one before
    // anything reads them (round 5, RT5-1): a reader
    // takes the first record it finds for a picture,
    // so a weak duplicate standing first would
    // silently shadow the fuller record.
    ob_model::fold_duplicate_asset_records(&mut doc.assets);
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
    // The raw entry count is checked before the archive
    // is opened at all, so a flood of same-name entries
    // is refused from the directory alone (round 5,
    // RT5-5).
    check_entry_cap(path)?;
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut zip =
        zip::ZipArchive::new(file).map_err(|e| format!("that file's container is damaged: {e}"))?;
    // The entry cap counts the archive's parts before
    // any payload is read (round 4, RT-2).
    if zip.len() > MAX_ENTRIES {
        return Err(TOO_MANY_PARTS.to_string());
    }
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
    // The hashes the document already holds a record
    // for, kept as a set (round 4, RT-2): asking the
    // growing list once per asset made opening cost
    // rise with the square of the asset count.
    let mut recorded: HashSet<String> = doc.assets.iter().map(|a| a.sha256.clone()).collect();
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
        if recorded.insert(hash.clone()) {
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
    // The same raw entry count as the bulletin door,
    // checked before the archive is opened (round 5,
    // RT5-5).
    check_entry_cap(path)?;
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut zip =
        zip::ZipArchive::new(file).map_err(|e| format!("that file's container is damaged: {e}"))?;
    // The entry cap counts the archive's parts before
    // any payload is read (round 4, RT-2).
    if zip.len() > MAX_ENTRIES {
        return Err(TOO_MANY_PARTS.to_string());
    }
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
    // The membership question is asked of a set, not by
    // scanning the records once per picture (round 4,
    // RT-2: the same squared cost the bulletin loader
    // had).
    let mut recorded: HashSet<String> = package.assets.iter().map(|a| a.sha256.clone()).collect();
    for hash in bytes_by_hash.keys() {
        if recorded.insert(hash.clone()) {
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
    // Records that share a hash fold into one before
    // the attach reads them, the same rule as a
    // bulletin file (round 5, RT5-1).
    ob_model::fold_duplicate_asset_records(&mut package.assets);
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
            applied_contributions: Vec::new(),
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
    fn the_applied_parts_record_survives_a_save_and_an_old_file_loads_without_one() {
        // Round 4 (R4-4): the record rides in the file,
        // so a bulletin saved by hand and reopened still
        // knows which parts were applied to it. A file
        // written before the field existed has no key
        // and loads with an empty record.
        let mut doc = test_doc();
        doc.applied_contributions = vec!["abcd1234".to_string(), "ef567890".to_string()];
        let dir = scratch("applied");
        let path = dir.join("applied.obd");
        save(&doc, &path).unwrap();
        let back = load(&path).unwrap();
        assert_eq!(back.applied_contributions, doc.applied_contributions);
        let mut json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&doc).expect("json")).expect("value");
        json.as_object_mut()
            .expect("object")
            .remove("applied_contributions");
        let old_path = dir.join("old.obd");
        std::fs::write(&old_path, serde_json::to_string(&json).expect("json")).expect("write");
        let old = load(&old_path).unwrap();
        assert!(old.applied_contributions.is_empty());
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

    #[test]
    fn r5_duplicate_asset_records_fold_on_load() {
        // Round 5 (RT5-1): document.json carries two
        // records for one picture: a weak one and the
        // record noting the children and the CC BY
        // license. Readers take the first record they
        // find, so a weak record standing first
        // disarmed the children note and dropped the
        // license. The load folds the pair into one
        // composed record, in either order.
        let bytes: &[u8] = b"picture bytes whose fingerprint both records share";
        let hash = ob_model::sha256_hex(bytes);
        let weak = RightsRecord {
            source: "a plainer copy".into(),
            ..Default::default()
        };
        let noting = RightsRecord {
            title: "Harvest supper".into(),
            license: LicenseKind::CcBy("4.0".into()),
            children_present: Some(true),
            ..Default::default()
        };
        let asset = |rights: RightsRecord| Asset {
            sha256: hash.clone(),
            kind: AssetKind::Picture,
            media_type: "image/png".into(),
            rights,
        };
        let dir = scratch("dup-records");
        for (tag, records) in [
            (
                "weak-first",
                vec![asset(weak.clone()), asset(noting.clone())],
            ),
            (
                "noting-first",
                vec![asset(noting.clone()), asset(weak.clone())],
            ),
        ] {
            let mut doc = test_doc();
            doc.assets = records;
            doc.asset_bytes.insert(hash.clone(), bytes.to_vec());
            let file = dir.join(format!("{tag}.obd"));
            save(&doc, &file).expect("the file saves");
            let loaded = load(&file).expect("the file loads");
            assert_eq!(loaded.assets.len(), 1, "{tag}: one record after the fold");
            let rights = &loaded.assets[0].rights;
            assert_eq!(rights.children_present, Some(true), "{tag}");
            assert_eq!(rights.license, LicenseKind::CcBy("4.0".into()), "{tag}");
            assert_eq!(rights.title, "Harvest supper", "{tag}");
            assert_eq!(rights.source, "a plainer copy", "{tag}");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn r6_credit_first_records_fold_on_load_without_a_permission() {
        // Round 6 (R6-1): an .obd carrying the credit
        // record first and the children record second
        // used to fold into a record noting children
        // WITH a permission. Both orders now fold to a
        // record that notes the children and no
        // permission.
        let bytes: &[u8] = b"picture bytes the credit-first pair shares";
        let hash = ob_model::sha256_hex(bytes);
        let credit = RightsRecord {
            credit_text: "Photo by Lee".into(),
            ..Default::default()
        };
        let children = RightsRecord {
            children_present: Some(true),
            ..Default::default()
        };
        let asset = |rights: RightsRecord| Asset {
            sha256: hash.clone(),
            kind: AssetKind::Picture,
            media_type: "image/png".into(),
            rights,
        };
        let dir = scratch("dup-r6-fold");
        for (tag, records) in [
            (
                "credit-first",
                vec![asset(credit.clone()), asset(children.clone())],
            ),
            (
                "children-first",
                vec![asset(children.clone()), asset(credit.clone())],
            ),
        ] {
            let mut doc = test_doc();
            doc.assets = records;
            doc.asset_bytes.insert(hash.clone(), bytes.to_vec());
            let file = dir.join(format!("{tag}.obd"));
            save(&doc, &file).expect("the file saves");
            let loaded = load(&file).expect("the file loads");
            assert_eq!(loaded.assets.len(), 1, "{tag}: one record after the fold");
            let rights = &loaded.assets[0].rights;
            assert_eq!(rights.children_present, Some(true), "{tag}");
            assert_eq!(rights.credit_text, "", "{tag}");
            assert_eq!(rights.license, LicenseKind::Unknown, "{tag}");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn r5_duplicate_records_fold_in_a_sheet_file_too() {
        // Round 5 (RT5-1): the same fold in an .obdi.
        // The host holds two records for the sheet's
        // picture; both travel, and the load composes
        // them into the one record the attach reads.
        // FLAGGED, round 6 (R6-A): this test's pair is a
        // ChurchOwned record (no children note) standing
        // first and a children-noted record second, and
        // it pinned the ChurchOwned license standing in
        // the folded record. That composition is exactly
        // the manufactured permission RT6-1 demonstrated:
        // no single record carried the license together
        // with the children note. The license assertion
        // below therefore changed from ChurchOwned to
        // Unknown in R6-A; every other assertion (one
        // folded record, source, title, children note)
        // stands unchanged.
        let dir = scratch("obdi-dup");
        let mut host = test_doc();
        let (ins, hash) = sheet_with_picture(&mut host);
        host.assets.push(Asset {
            sha256: hash.clone(),
            kind: AssetKind::Picture,
            media_type: "image/png".into(),
            rights: RightsRecord {
                title: "Prayer sheet photo".into(),
                children_present: Some(true),
                ..Default::default()
            },
        });
        let path = dir.join("Prayer list.obdi");
        save_insert(&ins, &host, &path).unwrap();
        let (_back, assets, _bytes) = load_insert(&path).unwrap();
        assert_eq!(assets.len(), 1, "the pair folds into one record");
        assert_eq!(assets[0].rights.source, "the church camera");
        assert_eq!(assets[0].rights.license, LicenseKind::Unknown);
        assert_eq!(assets[0].rights.title, "Prayer sheet photo");
        assert_eq!(assets[0].rights.children_present, Some(true));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn r6_duplicate_records_fold_in_a_sheet_file_too() {
        // Round 6 (R6-1): the credit-first pair in an
        // .obdi, in both orders. The folded record notes
        // the children and no permission, whichever
        // record stood first in the file.
        let bytes: &[u8] = b"sheet picture bytes whose fingerprint both records share";
        let hash = ob_model::sha256_hex(bytes);
        let credit = RightsRecord {
            credit_text: "Photo by Lee".into(),
            ..Default::default()
        };
        let children = RightsRecord {
            children_present: Some(true),
            ..Default::default()
        };
        let asset = |rights: RightsRecord| Asset {
            sha256: hash.clone(),
            kind: AssetKind::Picture,
            media_type: "image/png".into(),
            rights,
        };
        let dir = scratch("obdi-r6-fold");
        for (tag, records) in [
            (
                "credit-first",
                vec![asset(credit.clone()), asset(children.clone())],
            ),
            (
                "children-first",
                vec![asset(children.clone()), asset(credit.clone())],
            ),
        ] {
            let mut host = test_doc();
            let (ins, _sheet_hash) = sheet_with_picture(&mut host);
            host.assets = records;
            host.asset_bytes.insert(hash.clone(), bytes.to_vec());
            // Point the sheet's frame at this picture, so
            // both records travel in the file.
            let mut ins = ins;
            ins.panels[0].frames[0].picture = Some(PicturePlacement::new(hash.clone()));
            let path = dir.join(format!("{tag}.obdi"));
            save_insert(&ins, &host, &path).unwrap();
            let (_back, assets, _bytes) = load_insert(&path).unwrap();
            assert_eq!(assets.len(), 1, "{tag}: one record after the fold");
            let rights = &assets[0].rights;
            assert_eq!(rights.children_present, Some(true), "{tag}");
            assert_eq!(rights.credit_text, "", "{tag}");
        }
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

    /// A zip holding one JSON entry under `json_name`
    /// plus `fillers` one-byte stored entries behind
    /// it: the shape the entry cap counts (round 4,
    /// RT-2). Nothing in it is large; only numerous.
    fn zip_with_fillers(json_name: &str, json: &str, fillers: usize) -> Vec<u8> {
        let stored = || {
            zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored)
        };
        let mut buf = Vec::new();
        {
            let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
            zip.start_file(json_name, stored()).expect("json entry");
            zip.write_all(json.as_bytes()).expect("json bytes");
            for i in 0..fillers {
                zip.start_file(format!("filler/{i}"), stored())
                    .expect("filler entry");
                zip.write_all(b"x").expect("filler bytes");
            }
            zip.finish().expect("finish");
        }
        buf
    }

    #[test]
    fn a_bulletin_with_too_many_parts_is_refused_before_any_is_read() {
        // Round 4, RT-2: a file of 100,001 tiny assets
        // sat under every byte cap and still took
        // minutes to open, because nothing counted its
        // parts. The count now comes from the archive's
        // entry list, before any payload is read: one
        // part past the cap refuses the whole file, and
        // a file AT the cap still opens.
        let json = serde_json::to_string(&test_doc()).expect("json");
        let dir = scratch("many-parts-obd");
        let over = dir.join("over.obd");
        std::fs::write(&over, zip_with_fillers("document.json", &json, MAX_ENTRIES))
            .expect("write over");
        let err = load(&over).expect_err("one part past the cap refuses");
        assert_eq!(err, TOO_MANY_PARTS);
        let at = dir.join("at.obd");
        std::fs::write(
            &at,
            zip_with_fillers("document.json", &json, MAX_ENTRIES - 1),
        )
        .expect("write at");
        load(&at).expect("a file at the cap still opens");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_loose_sheet_with_too_many_parts_is_refused_before_any_is_read() {
        // The same count at the sheet door (round 4,
        // RT-2).
        let package = InsertPackage {
            insert: Insert {
                id: "sheet-many".into(),
                name: "Many parts".into(),
                life: ob_model::InsertLife::ThisWeekOnly,
                keep_through: None,
                panels: vec![],
                stories: vec![],
                styles: default_styles(),
                sheet: ob_model::InsertSheet {
                    preset: "half-letter".into(),
                    copies: 1,
                    paper_note: String::new(),
                },
            },
            assets: vec![],
        };
        let json = serde_json::to_string(&package).expect("json");
        let dir = scratch("many-parts-obdi");
        let over = dir.join("over.obdi");
        std::fs::write(&over, zip_with_fillers("insert.json", &json, MAX_ENTRIES))
            .expect("write over");
        let err = load_insert(&over).expect_err("one part past the cap refuses");
        assert_eq!(err, TOO_MANY_PARTS);
        let at = dir.join("at.obdi");
        std::fs::write(&at, zip_with_fillers("insert.json", &json, MAX_ENTRIES - 1))
            .expect("write at");
        load_insert(&at).expect("a file at the cap still opens");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Rewrite every filler name in a zip built by
    /// `zip_with_fillers`-style code to one shared name,
    /// in a single pass: the digits after "dup/" become
    /// "shared" in the local header and the central
    /// directory alike (the header guards keep payload
    /// bytes from being mistaken for a name).
    fn patch_all_filler_names(zip_bytes: &mut [u8]) {
        let mut patched = 0;
        let mut from = 0;
        while let Some(found) = find_bytes(&zip_bytes[from..], b"dup/") {
            let at = from + found;
            let guarded = (at >= 30 && &zip_bytes[at - 30..at - 26] == b"PK\x03\x04")
                || (at >= 46 && &zip_bytes[at - 46..at - 42] == b"PK\x01\x02");
            if guarded {
                zip_bytes[at + 4..at + 10].copy_from_slice(b"shared");
                patched += 1;
            }
            from = at + 1;
        }
        assert!(patched >= 2, "the fillers' names were patched");
    }

    /// A zip holding one JSON entry plus `fillers`
    /// one-byte stored entries that ALL share one name:
    /// the shape that walked past the old cap, which
    /// counted distinct names (round 5, RT5-5). The
    /// writer refuses a duplicate name outright, so the
    /// fillers are written under stand-in names of the
    /// same length and patched to the shared one.
    fn zip_with_same_name_fillers(json_name: &str, json: &str, fillers: usize) -> Vec<u8> {
        let stored = || {
            zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored)
        };
        let mut buf = Vec::new();
        {
            let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
            zip.start_file(json_name, stored()).expect("json entry");
            zip.write_all(json.as_bytes()).expect("json bytes");
            for i in 0..fillers {
                zip.start_file(format!("dup/{i:06}"), stored())
                    .expect("filler entry");
                zip.write_all(b"x").expect("filler bytes");
            }
            zip.finish().expect("finish");
        }
        patch_all_filler_names(&mut buf);
        buf
    }

    #[test]
    fn same_name_entries_count_against_the_cap_in_a_bulletin() {
        // Round 5, RT5-5: the cap counts entries as
        // written, not distinct names. A file whose
        // 8,192 fillers all share one name holds 8,193
        // entries and is refused in the cap's own words,
        // while the same shape one entry under the cap
        // (8,192 raw entries, only two distinct names)
        // still opens.
        let json = serde_json::to_string(&test_doc()).expect("json");
        let dir = scratch("dup-parts-obd");
        let over = dir.join("over.obd");
        std::fs::write(
            &over,
            zip_with_same_name_fillers("document.json", &json, MAX_ENTRIES),
        )
        .expect("write over");
        let err = load(&over).expect_err("same-name entries past the cap refuse");
        assert_eq!(err, TOO_MANY_PARTS);
        let at = dir.join("at.obd");
        std::fs::write(
            &at,
            zip_with_same_name_fillers("document.json", &json, MAX_ENTRIES - 1),
        )
        .expect("write at");
        load(&at).expect("a file at the raw cap still opens");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn same_name_entries_count_against_the_cap_in_a_loose_sheet() {
        // The same raw count at the sheet door (round 5,
        // RT5-5).
        let package = InsertPackage {
            insert: Insert {
                id: "sheet-dup".into(),
                name: "Same names".into(),
                life: ob_model::InsertLife::ThisWeekOnly,
                keep_through: None,
                panels: vec![],
                stories: vec![],
                styles: default_styles(),
                sheet: ob_model::InsertSheet {
                    preset: "half-letter".into(),
                    copies: 1,
                    paper_note: String::new(),
                },
            },
            assets: vec![],
        };
        let json = serde_json::to_string(&package).expect("json");
        let dir = scratch("dup-parts-obdi");
        let over = dir.join("over.obdi");
        std::fs::write(
            &over,
            zip_with_same_name_fillers("insert.json", &json, MAX_ENTRIES),
        )
        .expect("write over");
        let err = load_insert(&over).expect_err("same-name entries past the cap refuse");
        assert_eq!(err, TOO_MANY_PARTS);
        let at = dir.join("at.obdi");
        std::fs::write(
            &at,
            zip_with_same_name_fillers("insert.json", &json, MAX_ENTRIES - 1),
        )
        .expect("write at");
        load_insert(&at).expect("a sheet at the raw cap still opens");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The end-of-directory record's position in a zip
    /// built by the writer helpers: the last PK\x05\x06
    /// whose comment lands on the end of the file.
    fn find_eocd(zip_bytes: &[u8]) -> usize {
        let mut found = None;
        let mut from = 0;
        while let Some(hit) = find_bytes(&zip_bytes[from..], b"PK\x05\x06") {
            let at = from + hit;
            if at + 22 <= zip_bytes.len() {
                let comment = u16::from_le_bytes([zip_bytes[at + 20], zip_bytes[at + 21]]) as usize;
                if at + 22 + comment == zip_bytes.len() {
                    found = Some(at);
                }
            }
            from = at + 1;
        }
        found.expect("an end record")
    }

    /// Splice a zip64 end record and locator in front
    /// of a plain zip's end record, the plain fields
    /// turned to their sentinels: the shape the round-6
    /// red team's zip64 fixtures took. The zip64
    /// record's own total-count and directory-size
    /// fields are the caller's to set — honest, or
    /// lying in either direction.
    fn splice_zip64(zip_bytes: &[u8], total: u64, cd_size: u64) -> Vec<u8> {
        let eocd = find_eocd(zip_bytes);
        let cd_offset = u64::from(u32::from_le_bytes(
            zip_bytes[eocd + 16..eocd + 20]
                .try_into()
                .expect("offset field"),
        ));
        let mut out = zip_bytes[..eocd].to_vec();
        let mut record = Vec::new();
        record.extend_from_slice(b"PK\x06\x06");
        record.extend_from_slice(&44u64.to_le_bytes());
        record.extend_from_slice(&45u16.to_le_bytes());
        record.extend_from_slice(&45u16.to_le_bytes());
        record.extend_from_slice(&0u32.to_le_bytes());
        record.extend_from_slice(&0u32.to_le_bytes());
        record.extend_from_slice(&total.to_le_bytes());
        record.extend_from_slice(&total.to_le_bytes());
        record.extend_from_slice(&cd_size.to_le_bytes());
        record.extend_from_slice(&cd_offset.to_le_bytes());
        assert_eq!(record.len(), 56);
        out.extend_from_slice(&record);
        let mut locator = Vec::new();
        locator.extend_from_slice(b"PK\x06\x07");
        locator.extend_from_slice(&0u32.to_le_bytes());
        locator.extend_from_slice(&(eocd as u64).to_le_bytes());
        locator.extend_from_slice(&1u32.to_le_bytes());
        assert_eq!(locator.len(), 20);
        out.extend_from_slice(&locator);
        let mut tail = zip_bytes[eocd..].to_vec();
        tail[8..10].copy_from_slice(&u16::MAX.to_le_bytes());
        tail[10..12].copy_from_slice(&u16::MAX.to_le_bytes());
        tail[12..16].copy_from_slice(&u32::MAX.to_le_bytes());
        tail[16..20].copy_from_slice(&u32::MAX.to_le_bytes());
        out.extend_from_slice(&tail);
        out
    }

    /// One loose-sheet package's JSON, for the lying
    /// directory tests at the sheet door.
    fn lying_sheet_json() -> String {
        let package = InsertPackage {
            insert: Insert {
                id: "sheet-lying".into(),
                name: "Lying directory".into(),
                life: ob_model::InsertLife::ThisWeekOnly,
                keep_through: None,
                panels: vec![],
                stories: vec![],
                styles: default_styles(),
                sheet: ob_model::InsertSheet {
                    preset: "half-letter".into(),
                    copies: 1,
                    paper_note: String::new(),
                },
            },
            assets: vec![],
        };
        serde_json::to_string(&package).expect("json")
    }

    #[test]
    fn a_lying_directory_size_or_count_does_not_hide_entries_past_the_cap() {
        // Round 6 (RT6-7): the count walk used to follow
        // the directory's declared SIZE, so patching it
        // down to the first record let 9,001 entries
        // load. The count is walked from the records
        // themselves now: declared fields are ignored in
        // either direction — a size shrunk to one record
        // or a count field claiming two both still count
        // 9,001, and both loaders refuse.
        let doc_json = serde_json::to_string(&test_doc()).expect("json");
        let sheet_json = lying_sheet_json();
        for (json_name, json, ext) in [
            ("document.json", doc_json.as_str(), "obd"),
            ("insert.json", sheet_json.as_str(), "obdi"),
        ] {
            let dir = scratch(&format!("lying-dir-{ext}"));
            let honest = zip_with_same_name_fillers(json_name, json, 9_000);
            let eocd = find_eocd(&honest);
            // The size lie: the directory claims to cover
            // only its first record.
            let first_record = 46 + json_name.len() as u32;
            let mut size_lie = honest.clone();
            size_lie[eocd + 12..eocd + 16].copy_from_slice(&first_record.to_le_bytes());
            // The count lie: both count fields claim two.
            let mut count_lie = honest;
            count_lie[eocd + 8..eocd + 10].copy_from_slice(&2u16.to_le_bytes());
            count_lie[eocd + 10..eocd + 12].copy_from_slice(&2u16.to_le_bytes());
            for (tag, bytes) in [("size", size_lie), ("count", count_lie)] {
                let path = dir.join(format!("{tag}-lie.{ext}"));
                std::fs::write(&path, &bytes).expect("write lying copy");
                assert_eq!(
                    raw_entry_count(&path, MAX_ENTRIES),
                    Some(MAX_ENTRIES + 1),
                    "{ext} {tag} lie: the walk counts records until it passes \
                     the cap; the declared fields play no part"
                );
                let err = if ext == "obd" {
                    load(&path).expect_err("the flood refuses at the bulletin door")
                } else {
                    load_insert(&path).expect_err("the flood refuses at the sheet door")
                };
                assert_eq!(err, TOO_MANY_PARTS, "{ext} {tag} lie");
            }
            let _ = std::fs::remove_dir_all(&dir);
        }
    }

    #[test]
    fn a_lying_zip64_directory_is_counted_by_its_records() {
        // Round 6 (RT6-7): the same lies in a spliced
        // zip64 end record — its size field shrunk, or
        // its count field claiming two — fooled the old
        // walk the same way. Both loaders refuse: the
        // walk counts the 9,001 records in front of the
        // zip64 record and never consults its totals.
        let doc_json = serde_json::to_string(&test_doc()).expect("json");
        let sheet_json = lying_sheet_json();
        for (json_name, json, ext) in [
            ("document.json", doc_json.as_str(), "obd"),
            ("insert.json", sheet_json.as_str(), "obdi"),
        ] {
            let dir = scratch(&format!("lying-zip64-{ext}"));
            let honest = zip_with_same_name_fillers(json_name, json, 9_000);
            let eocd = find_eocd(&honest);
            let honest_size = u64::from(u32::from_le_bytes(
                honest[eocd + 12..eocd + 16].try_into().expect("size field"),
            ));
            let first_record = 46 + json_name.len() as u64;
            for (tag, bytes) in [
                ("size", splice_zip64(&honest, 9_001, first_record)),
                ("count", splice_zip64(&honest, 2, honest_size)),
            ] {
                let path = dir.join(format!("{tag}-lie.{ext}"));
                std::fs::write(&path, &bytes).expect("write lying copy");
                assert_eq!(
                    raw_entry_count(&path, MAX_ENTRIES),
                    Some(MAX_ENTRIES + 1),
                    "{ext} zip64 {tag} lie: the walk counts records until it \
                     passes the cap; the zip64 totals play no part"
                );
                let err = if ext == "obd" {
                    load(&path).expect_err("the flood refuses at the bulletin door")
                } else {
                    load_insert(&path).expect_err("the flood refuses at the sheet door")
                };
                assert_eq!(err, TOO_MANY_PARTS, "{ext} zip64 {tag} lie");
            }
            let _ = std::fs::remove_dir_all(&dir);
        }
    }

    #[test]
    fn a_declared_huge_size_with_few_real_entries_still_loads() {
        // Round 6 (RT6-7), the other direction: a
        // directory size claiming far more than the file
        // holds, with two real entries in it. The
        // declared size is ignored, the two records are
        // counted, and the file loads — plain and zip64.
        let doc_json = serde_json::to_string(&test_doc()).expect("json");
        let sheet_json = lying_sheet_json();
        for (json_name, json, ext) in [
            ("document.json", doc_json.as_str(), "obd"),
            ("insert.json", sheet_json.as_str(), "obdi"),
        ] {
            let dir = scratch(&format!("huge-size-{ext}"));
            let small = zip_with_fillers(json_name, json, 1);
            let eocd = find_eocd(&small);
            let mut plain = small.clone();
            plain[eocd + 12..eocd + 16].copy_from_slice(&0xFFFF_FFF0u32.to_le_bytes());
            let zip64 = splice_zip64(&small, 2, 0xFFFF_FFFF_FFFF);
            for (tag, bytes) in [("plain", plain), ("zip64", zip64)] {
                let path = dir.join(format!("{tag}-huge.{ext}"));
                std::fs::write(&path, &bytes).expect("write copy");
                assert_eq!(
                    raw_entry_count(&path, MAX_ENTRIES),
                    Some(2),
                    "{ext} {tag}: two records, whatever the size claims"
                );
                if ext == "obd" {
                    load(&path).expect("a huge declared size with few entries loads");
                } else {
                    load_insert(&path).expect("a huge declared size with few entries loads");
                }
            }
            let _ = std::fs::remove_dir_all(&dir);
        }
    }

    #[test]
    fn junk_around_the_archive_shifts_the_count_but_not_the_answer() {
        // Round 6 (R6-10 in the review lane, RT6-7's
        // tolerance half): bytes prepended to an archive
        // shift every offset the directory declares, and
        // bytes after the end record push it off the end
        // of the file. The count walk now locates the
        // end record and the directory the reader's way,
        // so the count is exact under either kind of
        // junk. What the LOADERS then do with a junked
        // file is pinned as it actually stands (this
        // test IS the pin): trailing junk loads, and a
        // prepended file is refused at the loaders' head
        // check — while the cap check itself counts the
        // shifted flood and refuses it.
        let doc_json = serde_json::to_string(&test_doc()).expect("json");
        let junk: Vec<u8> = b"self-extracting stub bytes, no signatures here ".repeat(13);
        let dir = scratch("junk-around");
        let small = zip_with_fillers("document.json", &doc_json, 1);
        // Prepended: the WALK counts the shifted archive
        // exactly (2), but the loaders never reach the
        // archive: both dispatch on the file's first
        // four bytes, which are junk here, not the zip
        // signature. That is what the shipped reader
        // does with a prepended file today (the round-6
        // probe's Z3 recorded the same refusal), and
        // this test pins it rather than wishing it
        // otherwise.
        let mut prepended = junk.clone();
        prepended.extend_from_slice(&small);
        let prepended_path = dir.join("prepended.obd");
        std::fs::write(&prepended_path, &prepended).expect("write prepended");
        assert_eq!(raw_entry_count(&prepended_path, MAX_ENTRIES), Some(2));
        let err = load(&prepended_path).expect_err("a prepended file is refused at the door");
        assert!(err.contains("UTF-8"), "the v0 words, as probed: {err}");
        // Trailing junk is different: the head is a real
        // signature, the reader tolerates the end record
        // sitting off the end of the file, and the file
        // loads.
        let mut trailed = small;
        trailed.extend_from_slice(&junk);
        let trailed_path = dir.join("trailed.obd");
        std::fs::write(&trailed_path, &trailed).expect("write trailed");
        assert_eq!(raw_entry_count(&trailed_path, MAX_ENTRIES), Some(2));
        load(&trailed_path).expect("the reader loads a trailed archive, and so do we");
        // Over the cap, with the same junk in front: the
        // count itself passes the cap (the cap check
        // refuses it in the cap's own words), and the
        // loaders refuse the file as well — at their
        // head check, before the archive is opened.
        let sheet_json = lying_sheet_json();
        for (json_name, json, ext) in [
            ("document.json", doc_json.as_str(), "obd"),
            ("insert.json", sheet_json.as_str(), "obdi"),
        ] {
            let big = zip_with_same_name_fillers(json_name, json, 9_000);
            let mut shifted = junk.clone();
            shifted.extend_from_slice(&big);
            let path = dir.join(format!("shifted-over.{ext}"));
            std::fs::write(&path, &shifted).expect("write shifted flood");
            assert_eq!(
                raw_entry_count(&path, MAX_ENTRIES),
                Some(MAX_ENTRIES + 1),
                "{ext}: the walk stops just past the cap, junk or not"
            );
            assert_eq!(
                check_entry_cap(&path),
                Err(TOO_MANY_PARTS.to_string()),
                "{ext}: the cap itself refuses the shifted flood"
            );
            if ext == "obd" {
                load(&path).expect_err("a shifted flood is refused at the bulletin door");
            } else {
                let err =
                    load_insert(&path).expect_err("a shifted flood is refused at the sheet door");
                assert_eq!(err, "that file is not a loose sheet file (.obdi)");
            }
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Rename one entry everywhere its name is
    /// written (the local header and the central
    /// directory), the two names the same length: the
    /// writer refuses a duplicate name outright, so a
    /// file that carries a repeated entry is assembled
    /// by patch, like the lying-size files above.
    fn patch_entry_name(zip_bytes: &mut [u8], from_name: &str, to_name: &str) {
        assert_eq!(from_name.len(), to_name.len(), "the patch keeps lengths");
        let name = from_name.as_bytes();
        let mut patched = 0;
        let mut from = 0;
        while let Some(found) = find_bytes(&zip_bytes[from..], name) {
            let at = from + found;
            let guarded = (at >= 30 && &zip_bytes[at - 30..at - 26] == b"PK\x03\x04")
                || (at >= 46 && &zip_bytes[at - 46..at - 42] == b"PK\x01\x02");
            if guarded {
                zip_bytes[at..at + name.len()].copy_from_slice(to_name.as_bytes());
                patched += 1;
            }
            from = at + 1;
        }
        assert_eq!(patched, 2, "both records for {from_name} were patched");
    }

    #[test]
    fn many_assets_load_with_one_record_each_even_when_entries_repeat() {
        // Round 4, RT-2's other half: the dedupe
        // question is asked of a set now, not of the
        // growing record list. This hand-built file
        // carries 300 pictures and NO records for them
        // in its document.json, and every picture's
        // entry rides twice (the repeats patched under
        // their real names, per the helper above): the
        // loader writes the record for the first entry
        // it meets, the repeat adds nothing, and the
        // bytes land once.
        let mut doc = test_doc();
        doc.assets.clear();
        let mut buf = Vec::new();
        let mut renames: Vec<(String, String)> = Vec::new();
        {
            let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            zip.start_file("document.json", options)
                .expect("json entry");
            zip.write_all(serde_json::to_string(&doc).expect("json").as_bytes())
                .expect("json bytes");
            for i in 0..300 {
                let bytes = format!("picture number {i}").into_bytes();
                let hash = ob_model::sha256_hex(&bytes);
                let name = format!("assets/{hash}");
                zip.start_file(&name, options).expect("asset entry");
                zip.write_all(&bytes).expect("asset bytes");
                let stand_in = format!("zzzz{i:067}");
                assert_eq!(stand_in.len(), name.len());
                zip.start_file(&stand_in, options).expect("repeat entry");
                zip.write_all(&bytes).expect("repeat bytes");
                renames.push((stand_in, name));
            }
            zip.finish().expect("finish");
        }
        for (from_name, to_name) in &renames {
            patch_entry_name(&mut buf, from_name, to_name);
        }
        let dir = scratch("many-assets");
        let path = dir.join("many.obd");
        std::fs::write(&path, &buf).expect("write");
        let loaded = load(&path).expect("loads");
        assert_eq!(loaded.assets.len(), 300, "one record per picture");
        assert_eq!(
            loaded.asset_bytes.len(),
            300,
            "one set of bytes per picture"
        );
        let bytes = "picture number 7".as_bytes().to_vec();
        let hash = ob_model::sha256_hex(&bytes);
        assert_eq!(loaded.asset_bytes.get(&hash), Some(&bytes));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
