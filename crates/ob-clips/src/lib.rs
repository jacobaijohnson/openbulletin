//! The Clipart Library (docs/research/clipart-landscape.md
//! section 4): one app-managed folder, a sibling of the Font
//! Library, holding named collections of picture files.
//! Everything is copied in and never live-linked, exactly
//! like fonts: a bulletin made on the office PC and one made
//! on the NAS see the same library rules, and placing a clip
//! copies its bytes into the document's own asset store, so
//! the bulletin travels complete whatever happens to the
//! library afterward.
//!
//! Indexing is deliberately un-clever: the collection name,
//! the path inside the collection, and the words in each
//! file's name. No tagging engine; a rename is the tagging.

use ob_image::PictureFormat;
use ob_model::LicenseKind;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// The default library folder: beside the Font Library, in
/// the same `OpenBulletin` folder, on every platform shape.
pub fn default_library_dir() -> PathBuf {
    let fonts = ob_fonts::default_library_dir();
    match fonts.parent() {
        Some(base) => base.join("clipart"),
        None => PathBuf::from("clipart"),
    }
}

/// A WMF/EMF converter: (file name, metafile bytes) -> PNG
/// bytes. The app supplies one backed by a user-installed
/// LibreOffice when present; with none, metafiles are
/// counted and left out, never shown as blank thumbnails.
pub type MetafileConverter = dyn Fn(&str, &[u8]) -> Option<Vec<u8>>;

/// Run a converter program with a deadline that bounds
/// the PROCESS, not only the wait (round 2, R2-3): the
/// old shape waited on a channel for 30 s while the
/// LibreOffice child kept running behind it, and the
/// scratch folder was deleted under a live process.
/// Here the child is polled to the deadline and, past
/// it, killed and reaped. The shape is the shared
/// runner's in miniature; it lives in this crate rather
/// than borrowing ob-extract's runner because clips sits
/// below extract in the tree. Returns whether the child
/// finished in time and said success; a child that
/// cannot be started at all is a quiet false, which the
/// converter contract already reads as "not converted".
pub fn run_converter_with_deadline(
    cmd: &mut std::process::Command,
    timeout: std::time::Duration,
) -> bool {
    cmd.stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(_) => return false,
    };
    let deadline = std::time::Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status.success(),
            Ok(None) => {
                if std::time::Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return false;
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return false;
            }
        }
    }
}

/// What a collection says about itself: where it came from
/// and the rights the office asserts for its contents
/// (the collection-level default of clipart-landscape.md
/// section 5). Stored as `notes.json` inside the collection
/// folder; a collection without one says nothing, and its
/// pictures place with an Unknown license.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollectionNotes {
    /// Where the collection came from, in words.
    #[serde(default)]
    pub source: String,
    /// The license as the collection states it: "CC0",
    /// "Public domain", "CC BY 4.0", or empty for unstated.
    #[serde(default)]
    pub license: String,
    /// When the collection was added (ISO date).
    #[serde(default)]
    pub added: String,
    /// Who made the piece, when the record names a maker
    /// (a per-piece record in a provenance map; a whole
    /// collection rarely has one maker, so collection
    /// notes leave this empty).
    #[serde(default)]
    pub creator: String,
}

impl CollectionNotes {
    /// The license in the model's vocabulary, parsed from
    /// the stated words; anything unrecognized is Unknown,
    /// never guessed up.
    pub fn license_kind(&self) -> LicenseKind {
        let words = self.license.trim();
        if words.eq_ignore_ascii_case("cc0") {
            LicenseKind::Cc0
        } else if words.eq_ignore_ascii_case("public domain") || words.eq_ignore_ascii_case("pd") {
            LicenseKind::PublicDomain
        } else if let Some(v) = strip_ci(words, "cc by-sa ") {
            LicenseKind::CcBySa(v.to_string())
        } else if let Some(v) = strip_ci(words, "cc by ") {
            LicenseKind::CcBy(v.to_string())
        } else if words.eq_ignore_ascii_case("church owned") || words.eq_ignore_ascii_case("ours") {
            LicenseKind::ChurchOwned
        } else {
            LicenseKind::Unknown
        }
    }
}

fn strip_ci<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    if text.len() >= prefix.len() && text[..prefix.len()].eq_ignore_ascii_case(prefix) {
        Some(text[prefix.len()..].trim())
    } else {
        None
    }
}

/// One picture file in a collection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipItem {
    pub file_name: String,
    pub path: PathBuf,
    /// The format its name claims, when it claims one the
    /// door recognizes.
    pub format: Option<PictureFormat>,
}

/// One named collection on disk.
#[derive(Debug, Clone)]
pub struct Collection {
    pub name: String,
    pub path: PathBuf,
    pub notes: Option<CollectionNotes>,
    /// Per-piece records (file name -> notes), for a
    /// collection whose pieces carry different rights: the
    /// bundled starter set, whose manifest states each
    /// piece's own source page, author, and license.
    /// Stored as `provenance.json` inside the collection
    /// folder; a collection without one has an empty map
    /// and its pieces share the collection notes.
    pub pieces: std::collections::BTreeMap<String, CollectionNotes>,
    pub items: Vec<ClipItem>,
}

impl Collection {
    /// The record one piece carries: its own per-piece
    /// record when the collection keeps one, else the
    /// collection's notes.
    pub fn piece_notes(&self, file_name: &str) -> Option<&CollectionNotes> {
        self.pieces.get(file_name).or(self.notes.as_ref())
    }
}

impl Collection {
    /// The items whose name contains all the query's words
    /// (the whole search: filename words, nothing cleverer).
    pub fn matching(&self, query: &str) -> Vec<&ClipItem> {
        let words: Vec<String> = query
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| !w.is_empty())
            .map(|w| w.to_lowercase())
            .collect();
        self.items
            .iter()
            .filter(|item| {
                let name = item.file_name.to_lowercase();
                words.iter().all(|w| name.contains(w.as_str()))
            })
            .collect()
    }
}

/// How one candidate file in an "add a collection" flow is
/// treated, decided by its name alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Candidate {
    /// A picture this program decodes: copied in as-is.
    Picture(PictureFormat),
    /// An old Windows metafile (WMF/EMF): converted by the
    /// caller's bridge when one exists, else counted+skipped.
    Metafile,
    /// A picture the door recognizes but cannot decode
    /// (AVIF, HEIC): counted and left out, in plain words.
    ConvertFirst(PictureFormat),
    /// Not a picture at all.
    Other,
}

pub fn classify_name(name: &str) -> Candidate {
    let lower = name.to_lowercase();
    if lower.ends_with(".wmf") || lower.ends_with(".emf") {
        return Candidate::Metafile;
    }
    match ob_image::format_for_name(name) {
        Some(f) if f.decodable() => Candidate::Picture(f),
        Some(f) => Candidate::ConvertFirst(f),
        None => Candidate::Other,
    }
}

/// What an "add a collection" flow did, in counts the app
/// says out loud: nothing is silently dropped, ever.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AddReport {
    /// Pictures copied in unchanged.
    pub added: usize,
    /// Metafiles converted (to PNG) and copied in.
    pub converted: usize,
    /// Files that were not pictures.
    pub skipped_not_pictures: usize,
    /// Metafiles left out because no converter was available.
    pub skipped_metafiles: usize,
    /// AVIF/HEIC files left out (recognized, not decodable).
    pub skipped_convert_first: usize,
    /// Picture-named files too large to be clipart (> 64 MB),
    /// or whose bytes could not be read.
    pub skipped_unreadable: usize,
}

const MAX_CLIP_BYTES: usize = 64 * 1024 * 1024;

/// The most one folder intake will hold in total:
/// 512 MB of accepted files, the workspace's standing
/// batch cap (the zip door and the .obd loader hold the
/// same line). Files past it are counted and left out,
/// like files past the per-file cap above.
pub const MAX_CLIP_BATCH_BYTES: u64 = 512 * 1024 * 1024;

/// The CP437 characters for bytes 0x80..=0xFF, in byte
/// order: the zip reader's own table, copied so a name
/// the reader decodes as CP437 is keyed here by the
/// characters the reader resolves it to (round 32, F4).
const CP437_HIGH: &str = "\
    \u{00c7}\u{00fc}\u{00e9}\u{00e2}\u{00e4}\u{00e0}\u{00e5}\u{00e7}\
    \u{00ea}\u{00eb}\u{00e8}\u{00ef}\u{00ee}\u{00ec}\u{00c4}\u{00c5}\
    \u{00c9}\u{00e6}\u{00c6}\u{00f4}\u{00f6}\u{00f2}\u{00fb}\u{00f9}\
    \u{00ff}\u{00d6}\u{00dc}\u{00a2}\u{00a3}\u{00a5}\u{20a7}\u{0192}\
    \u{00e1}\u{00ed}\u{00f3}\u{00fa}\u{00f1}\u{00d1}\u{00aa}\u{00ba}\
    \u{00bf}\u{2310}\u{00ac}\u{00bd}\u{00bc}\u{00a1}\u{00ab}\u{00bb}\
    \u{2591}\u{2592}\u{2593}\u{2502}\u{2524}\u{2561}\u{2562}\u{2556}\
    \u{2555}\u{2563}\u{2551}\u{2557}\u{255d}\u{255c}\u{255b}\u{2510}\
    \u{2514}\u{2534}\u{252c}\u{251c}\u{2500}\u{253c}\u{255e}\u{255f}\
    \u{255a}\u{2554}\u{2569}\u{2566}\u{2560}\u{2550}\u{256c}\u{2567}\
    \u{2568}\u{2564}\u{2565}\u{2559}\u{2558}\u{2552}\u{2553}\u{256b}\
    \u{256a}\u{2518}\u{250c}\u{2588}\u{2584}\u{258c}\u{2590}\u{2580}\
    \u{03b1}\u{00df}\u{0393}\u{03c0}\u{03a3}\u{03c3}\u{00b5}\u{03c4}\
    \u{03a6}\u{0398}\u{03a9}\u{03b4}\u{221e}\u{03c6}\u{03b5}\u{2229}\
    \u{2261}\u{00b1}\u{2265}\u{2264}\u{2320}\u{2321}\u{00f7}\u{2248}\
    \u{00b0}\u{2219}\u{00b7}\u{221a}\u{207f}\u{00b2}\u{25a0}\u{00a0}";

/// Key one raw central-directory name the way the zip
/// reader resolves it (round 32, F4): with the entry's
/// UTF-8 flag set, the name is UTF-8 text; with it
/// clear, the reader decodes the bytes as CP437.
fn decode_entry_name(raw: &[u8], flags: u16) -> String {
    if flags & 0x0800 != 0 {
        return String::from_utf8_lossy(raw).into_owned();
    }
    raw.iter()
        .map(|&byte| {
            if byte < 0x80 {
                byte as char
            } else {
                CP437_HIGH
                    .chars()
                    .nth(usize::from(byte - 0x80))
                    .unwrap_or('\u{fffd}')
            }
        })
        .collect()
}

/// Where one central record's entry payload stands
/// and how it is packed, its sizes and local offset
/// resolved through the zip64 extra field wherever a
/// plain field carries the sentinel — the same spot
/// the sibling doors' duplicate walks read (round 31,
/// F1).
struct RawSpot {
    method: u16,
    comp_size: u64,
    crc32: u32,
    local_offset: u64,
}

/// Read one central record's spot out of its fixed
/// header and extra field (the zip64 values ride in
/// the order uncompressed, compressed, local offset,
/// disk, one per sentinel field).
fn raw_spot(header: &[u8], extra: &[u8]) -> RawSpot {
    let field = |at: usize| u32::from_le_bytes(header[at..at + 4].try_into().unwrap_or([0; 4]));
    let comp32 = field(20);
    let uncomp32 = field(24);
    let local32 = field(42);
    let disk16 = u16::from_le_bytes([header[34], header[35]]);
    let mut comp_size = u64::from(comp32);
    let mut local_offset = u64::from(local32);
    if comp32 == u32::MAX || uncomp32 == u32::MAX || local32 == u32::MAX || disk16 == u16::MAX {
        let mut at = 0usize;
        while at + 4 <= extra.len() {
            let id = u16::from_le_bytes([extra[at], extra[at + 1]]);
            let size = usize::from(u16::from_le_bytes([extra[at + 2], extra[at + 3]]));
            let data = &extra[at + 4..(at + 4 + size).min(extra.len())];
            if id == 0x0001 {
                let mut pos = 0usize;
                if uncomp32 == u32::MAX {
                    pos += 8;
                }
                if comp32 == u32::MAX && pos + 8 <= data.len() {
                    comp_size = u64::from_le_bytes(data[pos..pos + 8].try_into().unwrap_or([0; 8]));
                    pos += 8;
                }
                if local32 == u32::MAX && pos + 8 <= data.len() {
                    local_offset =
                        u64::from_le_bytes(data[pos..pos + 8].try_into().unwrap_or([0; 8]));
                }
                break;
            }
            at += 4 + size;
        }
    }
    RawSpot {
        method: u16::from_le_bytes([header[10], header[11]]),
        comp_size,
        crc32: field(16),
        local_offset,
    }
}

/// The CRC-32 (zip checksum, IEEE polynomial) table,
/// built at compile time.
const CRC_TABLE: [u32; 256] = {
    let mut table = [0u32; 256];
    let mut i = 0;
    while i < 256 {
        let mut c = i as u32;
        let mut k = 0;
        while k < 8 {
            c = if c & 1 != 0 {
                0xEDB8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
            k += 1;
        }
        table[i] = c;
        i += 1;
    }
    table
};

/// CRC-32 over `bytes`, the checksum the central
/// record declares for its entry: a raw-read copy is
/// judged against it the way the archive reader
/// judges the copies it serves (a declared SIZE is
/// never trusted — round 3, RT3-B — but the checksum
/// is the copy's own fingerprint).
fn crc32_ieee(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in bytes {
        let at = ((crc ^ u32::from(byte)) & 0xFF) as usize;
        crc = CRC_TABLE[at] ^ (crc >> 8);
    }
    !crc
}

/// The (name, spot) pairs of a zip's central directory,
/// walked off the raw records, in archive order and
/// WITH duplicates standing (round 32, F6): the zip
/// reader's own map is keyed by name, so a duplicated
/// name reaches a door that enumerates through the
/// reader as only its last copy. The directory is the
/// one the latest end record names, through its zip64
/// record where the plain fields are sentinels; its
/// first record stands at-or-after the declared offset
/// (junk prepended before the archive shifts every
/// declared offset, the reader's own derivation,
/// round 32, F4), and that archive offset applies to
/// the local offsets in the records. Empty when no
/// end record stands — the caller falls back to the
/// reader's enumeration.
fn central_spots(bytes: &[u8]) -> Vec<(String, RawSpot)> {
    let mut eocd: Option<usize> = None;
    if bytes.len() >= 22 {
        for at in (0..=bytes.len() - 22).rev() {
            if !bytes[at..].starts_with(b"PK\x05\x06") {
                continue;
            }
            let comment = usize::from(u16::from_le_bytes([bytes[at + 20], bytes[at + 21]]));
            if at + 22 + comment <= bytes.len() {
                eocd = Some(at);
                break;
            }
        }
    }
    let Some(eocd) = eocd else {
        return Vec::new();
    };
    let field = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap_or([0; 4]));
    let mut total = u64::from(u16::from_le_bytes([bytes[eocd + 10], bytes[eocd + 11]]));
    let mut cd_offset = u64::from(field(eocd + 16));
    if (total == u64::from(u16::MAX) || cd_offset == u64::from(u32::MAX)) && eocd >= 20 {
        let locator = &bytes[eocd - 20..eocd];
        if locator.starts_with(b"PK\x06\x07") {
            let declared = u64::from_le_bytes(locator[8..16].try_into().unwrap_or([0; 8]));
            let mut record_at = declared as usize;
            let mut found = None;
            while record_at + 4 <= eocd - 20 {
                if bytes[record_at..].starts_with(b"PK\x06\x06") {
                    found = Some(record_at);
                    break;
                }
                record_at += 1;
            }
            if let Some(record_at) = found {
                if record_at + 56 <= bytes.len() {
                    let record = &bytes[record_at..record_at + 56];
                    total = u64::from_le_bytes(record[32..40].try_into().unwrap_or([0; 8]));
                    cd_offset = u64::from_le_bytes(record[48..56].try_into().unwrap_or([0; 8]));
                }
            }
        }
    }
    if total == 0 {
        return Vec::new();
    }
    let mut at = cd_offset as usize;
    let mut cd_start = None;
    while at + 4 <= eocd {
        if bytes[at..].starts_with(b"PK\x01\x02") {
            cd_start = Some(at);
            break;
        }
        at += 1;
    }
    let Some(cd_start) = cd_start else {
        return Vec::new();
    };
    let archive_offset = (cd_start as u64).saturating_sub(cd_offset);
    let mut out: Vec<(String, RawSpot)> = Vec::new();
    let mut at = cd_start;
    for _ in 0..total {
        if at + 46 > bytes.len() || !bytes[at..].starts_with(b"PK\x01\x02") {
            break;
        }
        let header = &bytes[at..at + 46];
        let name_len = usize::from(u16::from_le_bytes([header[28], header[29]]));
        let extra_len = usize::from(u16::from_le_bytes([header[30], header[31]]));
        let comment_len = usize::from(u16::from_le_bytes([header[32], header[33]]));
        let name_start = at + 46;
        let name_end = name_start + name_len;
        let extra_end = name_end + extra_len;
        if extra_end > bytes.len() {
            break;
        }
        let flags = u16::from_le_bytes([header[8], header[9]]);
        let name = decode_entry_name(&bytes[name_start..name_end], flags);
        let mut spot = raw_spot(header, &bytes[name_end..extra_end]);
        spot.local_offset = spot.local_offset.saturating_add(archive_offset);
        out.push((name, spot));
        at = extra_end + comment_len;
    }
    out
}

/// One entry's decompressed bytes off its raw spot,
/// read one byte past `cap` (so a copy past the cap is
/// known to be past it). The caller judges: a result
/// longer than the cap, or whose checksum is not the
/// one its record declares, is not a readable copy.
/// None when the payload cannot be read back at all.
fn raw_payload(bytes: &[u8], spot: &RawSpot, cap: u64) -> Option<Vec<u8>> {
    use std::io::Read as _;
    let local = spot.local_offset as usize;
    if local + 30 > bytes.len() || !bytes[local..].starts_with(b"PK\x03\x04") {
        return None;
    }
    let name_len = usize::from(u16::from_le_bytes([bytes[local + 26], bytes[local + 27]]));
    let extra_len = usize::from(u16::from_le_bytes([bytes[local + 28], bytes[local + 29]]));
    let data_start = local + 30 + name_len + extra_len;
    let data_end = data_start.checked_add(spot.comp_size as usize)?;
    if data_end > bytes.len() {
        return None;
    }
    let raw = &bytes[data_start..data_end];
    match spot.method {
        0 => Some(raw[..(raw.len() as u64).min(cap + 1) as usize].to_vec()),
        8 => {
            let mut decoder = flate2::read::DeflateDecoder::new(raw);
            let mut out = Vec::new();
            decoder.by_ref().take(cap + 1).read_to_end(&mut out).ok()?;
            Some(out)
        }
        _ => None,
    }
}

pub struct ClipLibrary {
    dir: PathBuf,
}

impl ClipLibrary {
    /// Open (creating if needed) the library at `dir`.
    pub fn open(dir: PathBuf) -> Result<ClipLibrary, String> {
        std::fs::create_dir_all(&dir)
            .map_err(|e| format!("cannot create clip art library folder: {e}"))?;
        Ok(ClipLibrary { dir })
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Every collection in the library, by name. A
    /// collection is a subfolder; dot-folders (the thumbnail
    /// cache) are ours, not collections.
    pub fn collections(&self) -> Vec<Collection> {
        let mut out = Vec::new();
        let Ok(entries) = std::fs::read_dir(&self.dir) else {
            return out;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }
            out.push(self.read_collection(name, path));
        }
        out.sort_by_key(|a| a.name.to_lowercase());
        out
    }

    pub fn collection(&self, name: &str) -> Option<Collection> {
        self.collections().into_iter().find(|c| c.name == name)
    }

    fn read_collection(&self, name: String, path: PathBuf) -> Collection {
        let notes = std::fs::read_to_string(path.join("notes.json"))
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok());
        let pieces = std::fs::read_to_string(path.join("provenance.json"))
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();
        let mut items = Vec::new();
        collect_items(&path, &path, &mut items);
        items.sort_by_key(|a| a.file_name.to_lowercase());
        Collection {
            name,
            path,
            notes,
            pieces,
            items,
        }
    }

    /// The on-disk thumbnail cache path for one library
    /// file: beside the library, keyed by the file's path,
    /// length, and modified time, so a replaced file gets a
    /// fresh thumb and the cache is always safe to delete.
    pub fn thumb_path(&self, item_path: &Path) -> Option<PathBuf> {
        let meta = std::fs::metadata(item_path).ok()?;
        let mtime = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let key = format!("{}|{}|{}", item_path.display(), meta.len(), mtime);
        let hash = ob_model::sha256_hex(key.as_bytes());
        Some(self.dir.join(".thumbs").join(format!("{hash}.png")))
    }

    /// Add a collection from (file name, bytes) pairs: the
    /// one funnel for the folder door, the zip door, and the
    /// bundled starter set. `convert` is the caller's
    /// metafile bridge (bytes and the file's name in, PNG
    /// bytes out); None leaves metafiles out, counted.
    pub fn add_collection(
        &self,
        name: &str,
        files: &[(String, Vec<u8>)],
        notes: &CollectionNotes,
        convert: Option<&MetafileConverter>,
    ) -> Result<AddReport, String> {
        self.add_collection_with_piece_notes(
            name,
            files,
            notes,
            &std::collections::BTreeMap::new(),
            convert,
        )
    }

    /// The same funnel, for a collection whose pieces
    /// carry their own records: `piece_notes` maps a
    /// piece's file name to the record that piece states
    /// (its own source, author, license, and added date),
    /// and the map is written beside the pieces as
    /// `provenance.json`, so the per-piece facts install
    /// with the set instead of flattening into the one
    /// collection note. Pieces named in the map but not
    /// among the files are not written; a piece with no
    /// entry in the map falls back to the collection
    /// notes when its record is read.
    pub fn add_collection_with_piece_notes(
        &self,
        name: &str,
        files: &[(String, Vec<u8>)],
        notes: &CollectionNotes,
        piece_notes: &std::collections::BTreeMap<String, CollectionNotes>,
        convert: Option<&MetafileConverter>,
    ) -> Result<AddReport, String> {
        let clean = ob_fonts::sanitize_file_name(name);
        let clean = if clean.is_empty() || clean == "font.ttf" {
            "Collection".to_string()
        } else {
            clean.trim_end_matches(['.', ' ']).to_string()
        };
        let target = self.dir.join(&clean);
        std::fs::create_dir_all(&target)
            .map_err(|e| format!("cannot create the collection folder: {e}"))?;
        let mut report = AddReport::default();
        let mut used_names: Vec<String> = Vec::new();
        // The records of the pieces that actually land,
        // keyed by the name each piece lands under: a
        // piece's record is paired with its landed name
        // here, while the landing is known, so it follows
        // the piece through dedup renaming (two pieces
        // sharing a basename land under different names,
        // each with its own record) and through metafile
        // conversion (a converted piece's record is kept
        // under its converted name).
        let mut landed_records: std::collections::BTreeMap<String, CollectionNotes> =
            std::collections::BTreeMap::new();
        for (file_name, bytes) in files {
            let base = ob_fonts::sanitize_file_name(file_name);
            match classify_name(&base) {
                Candidate::Picture(_) => {
                    if bytes.len() > MAX_CLIP_BYTES {
                        report.skipped_unreadable += 1;
                        continue;
                    }
                    let landed = unique_name(&target, &base, &mut used_names);
                    std::fs::write(target.join(&landed), bytes)
                        .map_err(|e| format!("cannot save {landed} into the library: {e}"))?;
                    if let Some(record) = piece_notes.get(file_name) {
                        landed_records.insert(landed.clone(), record.clone());
                    }
                    report.added += 1;
                }
                Candidate::Metafile => match convert.and_then(|c| c(&base, bytes)) {
                    Some(png) => {
                        let stem = base.rsplit_once('.').map(|(s, _)| s).unwrap_or(&base);
                        let landed = unique_name(&target, &format!("{stem}.png"), &mut used_names);
                        std::fs::write(target.join(&landed), &png)
                            .map_err(|e| format!("cannot save {landed} into the library: {e}"))?;
                        if let Some(record) = piece_notes.get(file_name) {
                            landed_records.insert(landed.clone(), record.clone());
                        }
                        report.converted += 1;
                    }
                    None => report.skipped_metafiles += 1,
                },
                Candidate::ConvertFirst(_) => report.skipped_convert_first += 1,
                Candidate::Other => report.skipped_not_pictures += 1,
            }
        }
        if !notes.source.is_empty() || !notes.license.is_empty() {
            let text = serde_json::to_string_pretty(notes)
                .map_err(|e| format!("cannot write the collection's notes: {e}"))?;
            std::fs::write(target.join("notes.json"), text)
                .map_err(|e| format!("cannot write the collection's notes: {e}"))?;
        }
        if !piece_notes.is_empty() {
            // Only the records of pieces that actually
            // came through the funnel are written, keyed
            // by the name the piece lands under (the
            // `landed_records` map built in the loop
            // above, where each landing was paired with
            // the record of the piece that landed there).
            if !landed_records.is_empty() {
                let text = serde_json::to_string_pretty(&landed_records)
                    .map_err(|e| format!("cannot write the pieces' records: {e}"))?;
                std::fs::write(target.join("provenance.json"), text)
                    .map_err(|e| format!("cannot write the pieces' records: {e}"))?;
            }
        }
        Ok(report)
    }

    /// Add a collection from a folder on disk: every file
    /// under it, recursively, through the same funnel.
    pub fn add_collection_from_folder(
        &self,
        folder: &Path,
        notes: &CollectionNotes,
        convert: Option<&MetafileConverter>,
    ) -> Result<AddReport, String> {
        let name = folder
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "Collection".to_string());
        let mut files: Vec<(String, Vec<u8>)> = Vec::new();
        let mut unreadable = 0usize;
        let mut total: u64 = 0;
        gather_files(folder, folder, &mut files, &mut unreadable, &mut total);
        let mut report = self.add_collection(&name, &files, notes, convert)?;
        report.skipped_unreadable += unreadable;
        Ok(report)
    }

    /// Add a collection from a zip archive's bytes. Archive
    /// paths are flattened to bare file names (the font
    /// library's traversal rule: nothing may write outside
    /// the collection folder, ever), and the whole archive
    /// is refused when it decompresses past the cap. The
    /// cap counts actual bytes as each entry is read
    /// (round 3, RT3-B): the sizes the archive's own
    /// directory declares are never summed, because a
    /// directory can lie about them, claiming a few bytes
    /// for an entry that expands to hundreds of
    /// megabytes.
    pub fn add_collection_from_zip(
        &self,
        zip_name: &str,
        bytes: &[u8],
        notes: &CollectionNotes,
        convert: Option<&MetafileConverter>,
    ) -> Result<AddReport, String> {
        let name = zip_name
            .rsplit(['/', '\\'])
            .next()
            .unwrap_or(zip_name)
            .trim_end_matches(['.', ' '])
            .to_string();
        let name = name
            .strip_suffix(".zip")
            .or_else(|| name.strip_suffix(".ZIP"))
            .unwrap_or(&name)
            .to_string();
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes))
            .map_err(|e| format!("that zip file cannot be read: {e}"))?;
        // Entry-count cap (health audit S-6), the IDML and
        // font-pack readers' 512: a clipart pack is dozens
        // of pictures, not thousands of entries, and the
        // count is known from the archive's own directory
        // before any entry is read.
        const MAX_ENTRIES: usize = 512;
        // Actual bytes, counted as they arrive; `total`
        // never passes the cap, so each read is capped at
        // what is left of it and memory stays bounded no
        // matter what the entries expand to.
        const MAX_TOTAL_UNCOMPRESSED: u64 = 512 * 1024 * 1024;
        // The pack's entries, in archive order, collected
        // off the raw central directory (central_spots)
        // so EVERY copy of a duplicated name arrives
        // (round 32, F6): the reader's name-keyed map
        // collapses a duplicated name to its last copy,
        // so two pic.png entries with different bytes
        // used to land as one item, the last copy,
        // although add_collection's unique_name dedup
        // exists to land both. Fed the whole list in
        // archive order, the copies land as pic.png and
        // pic-2.png, the first copy first. Only when no
        // raw directory stands does the collection fall
        // back to the reader's own enumeration.
        let spots = central_spots(bytes);
        let mut files: Vec<(String, Vec<u8>)> = Vec::new();
        if !spots.is_empty() {
            if spots.len() > MAX_ENTRIES {
                return Err(format!(
                    "that zip holds {} files; a clipart pack may carry at most {}",
                    spots.len(),
                    MAX_ENTRIES
                ));
            }
            let mut total: u64 = 0;
            for (name, spot) in &spots {
                if name.ends_with('/') || name.ends_with('\\') {
                    continue;
                }
                let remaining = MAX_TOTAL_UNCOMPRESSED - total;
                let content = raw_payload(bytes, spot, remaining).ok_or_else(|| {
                    "a file in that zip cannot be read: its bytes could not be read back"
                        .to_string()
                })?;
                if content.len() as u64 > remaining {
                    return Err(
                        "that zip holds more than 512 MB unzipped; a clipart pack is not that big"
                            .to_string(),
                    );
                }
                if crc32_ieee(&content) != spot.crc32 {
                    return Err(
                        "a file in that zip cannot be read: its bytes could not be read back"
                            .to_string(),
                    );
                }
                total += content.len() as u64;
                files.push((name.clone(), content));
            }
        } else {
            if archive.len() > MAX_ENTRIES {
                return Err(format!(
                    "that zip holds {} files; a clipart pack may carry at most {}",
                    archive.len(),
                    MAX_ENTRIES
                ));
            }
            let mut total: u64 = 0;
            for i in 0..archive.len() {
                let mut entry = archive
                    .by_index(i)
                    .map_err(|e| format!("that zip file cannot be read: {e}"))?;
                if entry.is_dir() {
                    continue;
                }
                // The read goes through ob-model's one
                // capped read (health audit E-9), capped at
                // what is left of the shared total. This
                // door's judgment is Refuse, in the words
                // below.
                let read = ob_model::read_capped(
                    &mut entry,
                    MAX_TOTAL_UNCOMPRESSED - total,
                    ob_model::CapOverflow::Refuse(
                        "that zip holds more than 512 MB unzipped; a clipart pack is not that big"
                            .to_string(),
                    ),
                )
                .map_err(|e| match e {
                    ob_model::CappedReadError::Refused(words) => words,
                    ob_model::CappedReadError::Unreadable(e) => {
                        format!("a file in that zip cannot be read: {e}")
                    }
                })?;
                total += read.bytes.len() as u64;
                files.push((entry.name().to_string(), read.bytes));
            }
        }
        self.add_collection(&name, &files, notes, convert)
    }
}

/// List a collection's files, recursively, naming each by
/// its path inside the collection so subfolders give the
/// picker its search words.
fn collect_items(root: &Path, dir: &Path, out: &mut Vec<ClipItem>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_items(root, &path, out);
        } else if path.is_file() {
            let base = entry.file_name().to_string_lossy().to_string();
            if base == "notes.json" || base == "provenance.json" {
                continue;
            }
            let rel = path
                .strip_prefix(root)
                .map(|p| p.to_string_lossy().replace('\\', "/"))
                .unwrap_or(base);
            out.push(ClipItem {
                file_name: rel,
                format: ob_image::format_for_name(&entry.file_name().to_string_lossy()),
                path,
            });
        }
    }
}

fn gather_files(
    root: &Path,
    dir: &Path,
    out: &mut Vec<(String, Vec<u8>)>,
    unreadable: &mut usize,
    total: &mut u64,
) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        // The entry's own file type, never followed
        // (health audit S-8): a symlink is not gathered
        // and a symlinked folder is not descended, so a
        // handed-over folder cannot pull outside content
        // into the library across a fence the user never
        // opened. Skipped symlinks are counted on the
        // same surface as unreadable files: the intake
        // report's skipped_unreadable count, which
        // add_collection_from_folder folds in.
        let Ok(file_type) = entry.file_type() else {
            *unreadable += 1;
            continue;
        };
        let path = entry.path();
        if file_type.is_symlink() {
            *unreadable += 1;
            continue;
        }
        if file_type.is_dir() {
            gather_files(root, &path, out, unreadable, total);
        } else if file_type.is_file() {
            // Size before read: the per-file cap is
            // judged on the file's metadata length, so an
            // oversize file is counted and left out
            // without its bytes ever being read whole.
            let Ok(meta) = entry.metadata() else {
                *unreadable += 1;
                continue;
            };
            if meta.len() > MAX_CLIP_BYTES as u64 {
                *unreadable += 1;
                continue;
            }
            // The batch total: intake stops accepting
            // past 512 MB of accepted bytes; files past
            // the line are counted and left out, like
            // the other skips on this door.
            if *total + meta.len() > MAX_CLIP_BATCH_BYTES {
                *unreadable += 1;
                continue;
            }
            let rel = path
                .strip_prefix(root)
                .map(|p| p.to_string_lossy().replace('\\', "/"))
                .unwrap_or_else(|_| entry.file_name().to_string_lossy().to_string());
            match std::fs::read(&path) {
                Ok(bytes) => {
                    *total += bytes.len() as u64;
                    out.push((rel, bytes));
                }
                Err(_) => *unreadable += 1,
            }
        }
    }
}

/// The name a file lands under in a collection: its own,
/// unless that name is taken in this add or already on disk,
/// when a short hash of its bytes keeps the two apart
/// (the font library's rule: never overwrite silently).
fn unique_name(dir: &Path, base: &str, used: &mut Vec<String>) -> String {
    let taken = |n: &str| used.iter().any(|u| u == n) || dir.join(n).exists();
    if !taken(base) {
        used.push(base.to_string());
        return base.to_string();
    }
    let stem = base.rsplit_once('.').map(|(s, _)| s).unwrap_or(base);
    let ext = base.rsplit_once('.').map(|(_, e)| e).unwrap_or("");
    let mut n = 2;
    loop {
        let candidate = if ext.is_empty() {
            format!("{stem}-{n}")
        } else {
            format!("{stem}-{n}.{ext}")
        };
        if !taken(&candidate) {
            used.push(candidate.clone());
            return candidate;
        }
        n += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn a_sleeping_converter_is_killed_at_the_deadline() {
        // Round 2, R2-3: the deadline bounds the process.
        // The fake converter sleeps, then writes a marker
        // only a process that lived past the deadline
        // could write. The run must come back at the
        // deadline, and the marker must never appear.
        use std::os::unix::fs::PermissionsExt;
        let dir = temp_dir("deadline");
        std::fs::create_dir_all(&dir).expect("dir");
        let marker = dir.join("lived-too-long");
        let script = dir.join("fake-converter.sh");
        std::fs::write(
            &script,
            format!("#!/bin/sh\nsleep 2\ntouch \"{}\"\n", marker.display()),
        )
        .expect("script");
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).expect("chmod");
        let mut cmd = std::process::Command::new(&script);
        let started = std::time::Instant::now();
        let ok = run_converter_with_deadline(&mut cmd, std::time::Duration::from_millis(300));
        let elapsed = started.elapsed();
        assert!(!ok, "a killed converter is not a success");
        assert!(
            elapsed < std::time::Duration::from_millis(1500),
            "the call returned at the deadline, not at the script's pace: {elapsed:?}"
        );
        // The script's own 2 s sleep would have ended by
        // now if anything of it were still alive.
        std::thread::sleep(std::time::Duration::from_millis(2200));
        assert!(
            !marker.exists(),
            "the converter was killed, not left running"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// TEST SUPPORT ONLY (round 10, R10-9): run a
    /// converter script this test itself just wrote,
    /// retrying a small bounded number of times
    /// until it reports success. A just-written
    /// script can refuse to start for a moment
    /// (ETXTBSY, the write-to-exec race), and the
    /// production runner folds that spawn failure
    /// into its plain `false`, so for a script whose
    /// own words are "exit 0" a `false` inside the
    /// window can only be the fixture's timing, not
    /// the converter's answer. The retry lives HERE,
    /// in the test harness, and nowhere else:
    /// `run_converter_with_deadline` gains no retry,
    /// so a real converter that cannot start on a
    /// user's machine still reports failure at once.
    #[cfg(unix)]
    fn run_fresh_converter(script: &Path, timeout: std::time::Duration) -> bool {
        for _ in 0..6 {
            let mut cmd = std::process::Command::new(script);
            if run_converter_with_deadline(&mut cmd, timeout) {
                return true;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        false
    }

    #[cfg(unix)]
    #[test]
    fn a_prompt_converter_reports_its_own_result() {
        use std::os::unix::fs::PermissionsExt;
        let dir = temp_dir("prompt");
        std::fs::create_dir_all(&dir).expect("dir");
        for (tag, code, want) in [("good", 0, true), ("bad", 3, false)] {
            let script = dir.join(format!("conv-{tag}.sh"));
            std::fs::write(&script, format!("#!/bin/sh\nexit {code}\n")).expect("script");
            std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755))
                .expect("chmod");
            // The exit-0 script's success is read
            // through the harness retry above (R10-9);
            // the exit-3 script's failure is its own
            // answer on the first attempt either way.
            let ok = if want {
                run_fresh_converter(&script, std::time::Duration::from_secs(5))
            } else {
                let mut cmd = std::process::Command::new(&script);
                run_converter_with_deadline(&mut cmd, std::time::Duration::from_secs(5))
            };
            assert_eq!(ok, want, "exit {code}");
        }
        // A program that does not exist is a quiet false.
        let mut cmd = std::process::Command::new(dir.join("no-such-program"));
        assert!(!run_converter_with_deadline(
            &mut cmd,
            std::time::Duration::from_secs(1)
        ));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn the_fresh_converter_retry_is_bounded() {
        // Round 10 (R10-9): the harness retry's
        // behavior on values a test can state
        // directly (the busy window itself is a race
        // and cannot be scheduled): a just-written
        // exit-0 script answers true through the
        // retry helper, and a program that is not
        // there answers false — the bounded attempts
        // end and the failure stands.
        use std::os::unix::fs::PermissionsExt;
        let dir = temp_dir("retry");
        std::fs::create_dir_all(&dir).expect("dir");
        let script = dir.join("conv-ok.sh");
        std::fs::write(&script, "#!/bin/sh\nexit 0\n").expect("script");
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).expect("chmod");
        assert!(run_fresh_converter(
            &script,
            std::time::Duration::from_secs(5)
        ));
        assert!(!run_fresh_converter(
            &dir.join("no-such-program"),
            std::time::Duration::from_secs(1)
        ));
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn temp_dir(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("ob-clips-test-{}-{}", std::process::id(), tag));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn tiny_png() -> Vec<u8> {
        let d = ob_image::Decoded {
            width: 4,
            height: 4,
            rgba: vec![255; 4 * 4 * 4],
            has_alpha: false,
        };
        ob_image::encode_png(&d).expect("png")
    }

    #[test]
    fn add_counts_every_kind_of_file() {
        let dir = temp_dir("counts");
        let lib = ClipLibrary::open(dir.clone()).expect("open");
        let notes = CollectionNotes {
            source: "a folder from the office".into(),
            license: "CC0".into(),
            added: "2026-10-16".into(),
            creator: String::new(),
        };
        let files = vec![
            ("cross.png".to_string(), tiny_png()),
            ("dove.svg".to_string(), br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><path d="M0 0h10v10z"/></svg>"#.to_vec()),
            ("old.wmf".to_string(), b"placeholder metafile".to_vec()),
            ("phone.avif".to_string(), b"\x00\x00\x00\x18ftypavif".to_vec()),
            ("readme.txt".to_string(), b"hello".to_vec()),
        ];
        // No converter: the metafile is counted and left out.
        let report = lib
            .add_collection("Office art", &files, &notes, None)
            .expect("add");
        assert_eq!(report.added, 2);
        assert_eq!(report.converted, 0);
        assert_eq!(report.skipped_metafiles, 1);
        assert_eq!(report.skipped_convert_first, 1);
        assert_eq!(report.skipped_not_pictures, 1);
        let collections = lib.collections();
        assert_eq!(collections.len(), 1);
        assert_eq!(collections[0].name, "Office art");
        assert_eq!(collections[0].items.len(), 2);
        assert_eq!(
            collections[0].notes.as_ref().map(|n| n.license_kind()),
            Some(LicenseKind::Cc0)
        );
        // Search is filename words.
        assert_eq!(collections[0].matching("cross").len(), 1);
        assert_eq!(collections[0].matching("dove").len(), 1);
        assert_eq!(collections[0].matching("star").len(), 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_converter_turns_metafiles_into_pngs() {
        let dir = temp_dir("convert");
        let lib = ClipLibrary::open(dir.clone()).expect("open");
        let convert = |_name: &str, _bytes: &[u8]| Some(tiny_png());
        let files = vec![("legacy.wmf".to_string(), b"wmf".to_vec())];
        let report = lib
            .add_collection(
                "Old pack",
                &files,
                &CollectionNotes::default(),
                Some(&convert),
            )
            .expect("add");
        assert_eq!(report.converted, 1);
        let collection = lib.collection("Old pack").expect("collection");
        assert_eq!(collection.items.len(), 1);
        assert_eq!(collection.items[0].file_name, "legacy.png");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn zip_past_the_entry_count_cap_is_refused() {
        // S-6 pin: 513 tiny entries against the 512 cap.
        // The refusal comes from the archive's own
        // directory count, before any entry is read.
        let dir = temp_dir("zip-count");
        let lib = ClipLibrary::open(dir.clone()).expect("open");
        let zip_bytes = {
            let mut buf = Vec::new();
            {
                let mut writer = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
                let opts = zip::write::SimpleFileOptions::default();
                use std::io::Write as _;
                for i in 0..513 {
                    writer
                        .start_file(format!("f{i}.png"), opts)
                        .expect("start entry");
                    writer.write_all(&tiny_png()).expect("write entry");
                }
                writer.finish().expect("finish");
            }
            buf
        };
        let err = lib
            .add_collection_from_zip("pack.zip", &zip_bytes, &CollectionNotes::default(), None)
            .expect_err("too many entries refused");
        assert!(err.contains("512"), "{err}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn zip_paths_flatten_and_never_escape() {
        let dir = temp_dir("zip");
        let lib = ClipLibrary::open(dir.clone()).expect("open");
        let zip_bytes = {
            let mut buf = Vec::new();
            {
                let mut writer = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
                let opts = zip::write::SimpleFileOptions::default();
                writer.start_file("../../evil.png", opts).expect("start");
                use std::io::Write as _;
                writer.write_all(&tiny_png()).expect("write");
                writer.start_file("nested/dove.png", opts).expect("start 2");
                writer.write_all(&tiny_png()).expect("write 2");
                writer.finish().expect("finish");
            }
            buf
        };
        let report = lib
            .add_collection_from_zip("pack.zip", &zip_bytes, &CollectionNotes::default(), None)
            .expect("add zip");
        assert_eq!(report.added, 2);
        let collection = lib.collection("pack").expect("collection");
        // Both landed inside the collection, flattened, with
        // apart-keeping names (identical bytes dedupe by
        // name only: two files, two entries).
        assert!(collection
            .items
            .iter()
            .all(|i| i.path.starts_with(&collection.path)));
        assert!(!dir.parent().expect("parent").join("evil.png").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        haystack.windows(needle.len()).position(|w| w == needle)
    }

    /// Rename one entry everywhere its name is written
    /// (the local header and the central directory),
    /// the two names the same length: the zip writer
    /// refuses a duplicate name outright, so a pack
    /// carrying a repeated entry is assembled by
    /// patch, the same helper shape as the sibling
    /// doors' tests.
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

    fn png_filled(byte: u8) -> Vec<u8> {
        let d = ob_image::Decoded {
            width: 4,
            height: 4,
            rgba: vec![byte; 4 * 4 * 4],
            has_alpha: false,
        };
        ob_image::encode_png(&d).expect("png")
    }

    #[test]
    fn zip_with_two_pictures_under_one_name_lands_both() {
        // Round 32, F6 (red team RT32-4, witness
        // c01-dup-pic.zip): two pic.png entries with
        // different bytes — the reader's name-keyed map
        // collapsed the pair to the last copy, so the
        // pack landed as ONE item although
        // add_collection's unique_name dedup exists to
        // land both. Every copy arrives now: the first
        // lands as pic.png, the second as pic-2.png,
        // each holding its own bytes.
        let dir = temp_dir("zip-dup-pic");
        let lib = ClipLibrary::open(dir.clone()).expect("open");
        let first_png = png_filled(255);
        let second_png = png_filled(0);
        assert_ne!(first_png, second_png, "the copies really differ");
        let mut zip_bytes = Vec::new();
        {
            let mut writer = zip::ZipWriter::new(std::io::Cursor::new(&mut zip_bytes));
            let opts = zip::write::SimpleFileOptions::default();
            writer.start_file("pic.png", opts).expect("start");
            use std::io::Write as _;
            writer.write_all(&first_png).expect("write first");
            writer.start_file("pic.pnX", opts).expect("start 2");
            writer.write_all(&second_png).expect("write second");
            writer.finish().expect("finish");
        }
        patch_entry_name(&mut zip_bytes, "pic.pnX", "pic.png");
        let report = lib
            .add_collection_from_zip("pack.zip", &zip_bytes, &CollectionNotes::default(), None)
            .expect("add zip");
        assert_eq!(report.added, 2, "both copies land: {report:?}");
        let collection = lib.collection("pack").expect("collection");
        assert_eq!(collection.items.len(), 2);
        let first = collection
            .items
            .iter()
            .find(|i| i.file_name == "pic.png")
            .expect("the first copy keeps the name");
        let second = collection
            .items
            .iter()
            .find(|i| i.file_name == "pic-2.png")
            .expect("the second copy lands apart");
        assert_eq!(
            std::fs::read(&first.path).expect("read first"),
            first_png,
            "the first item holds the first copy's bytes"
        );
        assert_eq!(
            std::fs::read(&second.path).expect("read second"),
            second_png,
            "the second item holds the second copy's bytes"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn zip_declared_sizes_no_longer_decide_the_cap() {
        // Round 3, RT3-B: the intake used to sum the
        // sizes the archive's directory declares toward
        // its 512 MB cap. This pack's two small pictures
        // declare 400 MB each, so the old sum refused a
        // pack that really holds a few hundred bytes.
        // The count now follows the bytes that actually
        // arrive, and the pack loads.
        let dir = temp_dir("zip-lying");
        let lib = ClipLibrary::open(dir.clone()).expect("open");
        let mut zip_bytes = {
            let mut buf = Vec::new();
            {
                let mut writer = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
                let opts = zip::write::SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Deflated);
                writer.start_file("dove.png", opts).expect("start");
                use std::io::Write as _;
                writer.write_all(&tiny_png()).expect("write");
                writer.start_file("cross.png", opts).expect("start 2");
                writer.write_all(&tiny_png()).expect("write 2");
                writer.finish().expect("finish");
            }
            buf
        };
        for name in ["dove.png", "cross.png"] {
            let needle = name.as_bytes();
            let mut patched = 0;
            let mut from = 0;
            while let Some(found) = zip_bytes[from..]
                .windows(needle.len())
                .position(|w| w == needle)
            {
                let at = from + found;
                if at >= 30 && &zip_bytes[at - 30..at - 26] == b"PK\x03\x04" {
                    zip_bytes[at - 12..at - 8].copy_from_slice(&400_000_000u32.to_le_bytes());
                    patched += 1;
                } else if at >= 46 && &zip_bytes[at - 46..at - 42] == b"PK\x01\x02" {
                    zip_bytes[at - 22..at - 18].copy_from_slice(&400_000_000u32.to_le_bytes());
                    patched += 1;
                }
                from = at + 1;
            }
            assert_eq!(patched, 2, "both records for {name} were patched");
        }
        let report = lib
            .add_collection_from_zip(
                "honest-pack.zip",
                &zip_bytes,
                &CollectionNotes::default(),
                None,
            )
            .expect("small pack with lying declarations loads");
        assert_eq!(report.added, 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn piece_notes_install_with_the_pieces_and_read_back() {
        let dir = temp_dir("piece-notes");
        let lib = ClipLibrary::open(dir.clone()).expect("open");
        let notes = CollectionNotes {
            source: "a mixed set".into(),
            license: String::new(),
            added: "2026-10-16".into(),
            creator: String::new(),
        };
        let cross = CollectionNotes {
            source: "https://example.org/cross".into(),
            license: "Public domain".into(),
            added: "2026-10-16".into(),
            creator: "Boris23".into(),
        };
        let candle = CollectionNotes {
            source: "https://example.org/candle".into(),
            license: "CC0".into(),
            added: "2026-10-16".into(),
            creator: "Quincy Morgan".into(),
        };
        let mut pieces = std::collections::BTreeMap::new();
        pieces.insert("cross.svg".to_string(), cross.clone());
        pieces.insert("candle.svg".to_string(), candle.clone());
        // A record for a piece that is not installed is
        // not written.
        pieces.insert(
            "ghost.svg".to_string(),
            CollectionNotes {
                source: "nowhere".into(),
                ..Default::default()
            },
        );
        let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><path d="M0 0h10v10z"/></svg>"#.to_vec();
        let files = vec![
            ("cross.svg".to_string(), svg.clone()),
            ("candle.svg".to_string(), svg),
            ("star.svg".to_string(), tiny_png()),
        ];
        let report = lib
            .add_collection_with_piece_notes("Mixed set", &files, &notes, &pieces, None)
            .expect("add");
        assert_eq!(report.added, 3);
        let collections = lib.collections();
        assert_eq!(collections.len(), 1);
        let set = &collections[0];
        // The record files are not pieces.
        assert_eq!(set.items.len(), 3);
        assert_eq!(set.piece_notes("cross.svg"), Some(&cross));
        assert_eq!(set.piece_notes("candle.svg"), Some(&candle));
        assert_eq!(
            set.piece_notes("cross.svg").map(|n| n.license_kind()),
            Some(LicenseKind::PublicDomain)
        );
        // A piece with no record of its own falls back to
        // the collection's notes.
        assert_eq!(set.piece_notes("star.svg"), set.notes.as_ref());
        assert!(!set.pieces.contains_key("ghost.svg"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn duplicate_basename_pieces_each_carry_their_own_record() {
        // F4(b) pin (round 30, RT30-2 witness): two
        // pieces share the basename dup.png but arrive
        // under different paths with different records.
        // They land as dup.png and dup-2.png, and each
        // landed piece must carry its OWN record — the
        // first piece's record is not overwritten by the
        // second's, and the second piece is not left
        // recordless.
        let dir = temp_dir("dup-basename");
        let lib = ClipLibrary::open(dir.clone()).expect("open");
        let alice = CollectionNotes {
            source: "alice-page".into(),
            license: "CC0".into(),
            added: "2026-10-16".into(),
            creator: "Alice".into(),
        };
        let bob = CollectionNotes {
            source: "bob-page".into(),
            license: "Public Domain".into(),
            added: "2026-10-16".into(),
            creator: "Bob".into(),
        };
        let mut pieces = std::collections::BTreeMap::new();
        pieces.insert("a/dup.png".to_string(), alice.clone());
        pieces.insert("b/dup.png".to_string(), bob.clone());
        let files = vec![
            ("a/dup.png".to_string(), tiny_png()),
            ("b/dup.png".to_string(), tiny_png()),
        ];
        let report = lib
            .add_collection_with_piece_notes(
                "Dup pack",
                &files,
                &CollectionNotes::default(),
                &pieces,
                None,
            )
            .expect("add");
        assert_eq!(report.added, 2);
        let set = lib.collection("Dup pack").expect("collection");
        assert_eq!(set.items.len(), 2);
        let first = set.piece_notes("dup.png").expect("first piece's record");
        assert_eq!(first.creator, "Alice");
        assert_eq!(first.license, "CC0");
        assert_eq!(first.source, "alice-page");
        assert_eq!(first, &alice);
        let second = set.piece_notes("dup-2.png").expect("second piece's record");
        assert_eq!(second.creator, "Bob");
        assert_eq!(second.license, "Public Domain");
        assert_eq!(second.source, "bob-page");
        assert_eq!(second, &bob);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_converted_pieces_record_is_kept_under_its_converted_name() {
        // F4(b) pin (round 30, RT30-2 witness): a piece
        // converted from a metafile (art.wmf lands as
        // art.png) keeps its record, keyed by the
        // converted name it lands under, not the
        // pre-conversion name.
        let dir = temp_dir("converted-record");
        let lib = ClipLibrary::open(dir.clone()).expect("open");
        let record = CollectionNotes {
            source: "art-page".into(),
            license: "CC0".into(),
            added: "2026-10-16".into(),
            creator: "Carol".into(),
        };
        let mut pieces = std::collections::BTreeMap::new();
        pieces.insert("art.wmf".to_string(), record.clone());
        let convert = |_name: &str, _bytes: &[u8]| Some(tiny_png());
        let files = vec![("art.wmf".to_string(), b"wmf".to_vec())];
        let report = lib
            .add_collection_with_piece_notes(
                "Old art",
                &files,
                &CollectionNotes::default(),
                &pieces,
                Some(&convert),
            )
            .expect("add");
        assert_eq!(report.converted, 1);
        let set = lib.collection("Old art").expect("collection");
        assert_eq!(set.items.len(), 1);
        assert_eq!(set.items[0].file_name, "art.png");
        assert_eq!(set.piece_notes("art.png"), Some(&record));
        assert!(!set.pieces.contains_key("art.wmf"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn a_symlinked_picture_is_not_gathered_and_a_symlinked_folder_is_not_descended() {
        // S-8 pin: a handed-over folder carrying symlinks
        // must not pull the linked content into the
        // library. The real picture lands; the symlink to
        // it is counted as skipped, and the symlinked
        // subfolder's picture never arrives at all.
        let dir = temp_dir("symlink");
        let outside = temp_dir("symlink-outside");
        std::fs::create_dir_all(&outside).expect("outside dir");
        std::fs::write(outside.join("secret.png"), tiny_png()).expect("outside picture");
        let folder = dir.join("handed-over");
        std::fs::create_dir_all(&folder).expect("folder");
        std::fs::write(folder.join("real.png"), tiny_png()).expect("real picture");
        std::os::unix::fs::symlink(folder.join("real.png"), folder.join("link.png"))
            .expect("file symlink");
        std::os::unix::fs::symlink(&outside, folder.join("linked-folder")).expect("folder symlink");
        let lib = ClipLibrary::open(dir.join("library")).expect("open");
        let report = lib
            .add_collection_from_folder(&folder, &CollectionNotes::default(), None)
            .expect("add");
        assert_eq!(report.added, 1, "only the real picture lands: {report:?}");
        assert!(
            report.skipped_unreadable >= 1,
            "the symlink is counted as skipped: {report:?}"
        );
        let collection = lib.collection("handed-over").expect("collection");
        assert_eq!(collection.items.len(), 1);
        assert_eq!(collection.items[0].file_name, "real.png");
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&outside);
    }

    #[test]
    fn an_oversize_file_is_left_out_from_its_size_alone() {
        // S-8 pin (size before read): a sparse file whose
        // stated length is past the 64 MB per-file cap is
        // counted and left out; the small file beside it
        // still lands.
        let dir = temp_dir("oversize");
        let folder = dir.join("handed-over");
        std::fs::create_dir_all(&folder).expect("folder");
        std::fs::write(folder.join("small.png"), tiny_png()).expect("small picture");
        let big = std::fs::File::create(folder.join("big.png")).expect("big file");
        big.set_len(65 * 1024 * 1024).expect("sparse length");
        drop(big);
        let mut files: Vec<(String, Vec<u8>)> = Vec::new();
        let mut unreadable = 0usize;
        let mut total: u64 = 0;
        gather_files(&folder, &folder, &mut files, &mut unreadable, &mut total);
        assert_eq!(files.len(), 1, "only the small file is gathered: {files:?}");
        assert_eq!(files[0].0, "small.png");
        assert_eq!(unreadable, 1, "the oversize file is counted as skipped");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn folder_intake_stops_accepting_past_the_batch_cap() {
        // S-8 pin (batch total): sparse files of 60 MB
        // each (each under the per-file cap) sum past
        // 512 MB; intake accepts up to the cap and counts
        // the rest as skipped, accepting nothing past it.
        let dir = temp_dir("batch-cap");
        let folder = dir.join("handed-over");
        std::fs::create_dir_all(&folder).expect("folder");
        for i in 0..10 {
            let f = std::fs::File::create(folder.join(format!("f{i}.png"))).expect("file");
            f.set_len(60 * 1024 * 1024).expect("sparse length");
            drop(f);
        }
        let mut files: Vec<(String, Vec<u8>)> = Vec::new();
        let mut unreadable = 0usize;
        let mut total: u64 = 0;
        gather_files(&folder, &folder, &mut files, &mut unreadable, &mut total);
        assert_eq!(files.len(), 8, "eight 60 MB files fit under 512 MB");
        assert!(
            total <= MAX_CLIP_BATCH_BYTES,
            "accepted total stays at the cap: {total}"
        );
        assert_eq!(total, 8 * 60 * 1024 * 1024);
        assert_eq!(
            unreadable, 2,
            "the two files past the cap are counted as skipped"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn license_words_parse_conservatively() {
        let n = |license: &str| {
            CollectionNotes {
                license: license.into(),
                ..Default::default()
            }
            .license_kind()
        };
        assert_eq!(n("CC0"), LicenseKind::Cc0);
        assert_eq!(n("Public domain"), LicenseKind::PublicDomain);
        assert_eq!(n("CC BY 4.0"), LicenseKind::CcBy("4.0".into()));
        assert_eq!(n("CC BY-SA 3.0"), LicenseKind::CcBySa("3.0".into()));
        assert_eq!(n("free-ish, probably"), LicenseKind::Unknown);
        assert_eq!(n(""), LicenseKind::Unknown);
    }
}
