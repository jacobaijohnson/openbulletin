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

/// One entry's decompressed bytes off its raw spot,
/// read one byte past `cap` (so a copy past the cap is
/// known to be past it). The caller judges: a result
/// longer than the cap, or whose checksum is not the
/// one its record declares, is not a readable copy.
/// None when the payload cannot be read back at all.
fn raw_payload(
    bytes: &[u8],
    spot: &ob_model::zip_directory::CentralSpot,
    cap: u64,
) -> Option<Vec<u8>> {
    use std::io::Read as _;
    // The span is the shared checked computation
    // (round 36, F7): this copy's plain additions on
    // the spot's zip64-resolved offset panicked on an
    // offset near u64::MAX.
    let (data_start, data_end) = ob_model::zip_directory::payload_span(bytes, spot)?;
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
        let directory_derived = ob_model::zip_directory::central_directory(bytes).is_some();
        // One prefix-free view of the pack for both the
        // reader and the raw-directory walk below
        // (round 33, F3): see the note at the IDML door.
        let view = ob_model::zip_directory::prefix_free(bytes);
        let bytes: &[u8] = &view;
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes))
            .map_err(|e| format!("that zip file cannot be read: {e}"))?;
        // Round 34, F2: the reader opened the pack, so
        // its entries can be served — but no directory
        // could be derived for the raw-directory walk
        // below, which would judge an empty directory
        // and leave the copies to the reader's
        // name-keyed enumeration, collapsing a
        // duplicated name to its last copy in silence.
        // Fail closed in this door's standing words. A
        // pack the reader cannot open refuses on its own
        // above; a genuinely empty pack derives its
        // empty directory and is untouched.
        if !archive.is_empty() && !directory_derived {
            return Err(
                "that zip file cannot be read: its central directory cannot be derived".to_string(),
            );
        }
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
        let spots = ob_model::zip_directory::central_spots(bytes);
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

    /// A Stored filler entry's payload in the round-33
    /// witness's shape: see the twin helper in
    /// ob-extract's tests.
    fn tail_filler(shape: &str) -> Vec<u8> {
        let mut payload = vec![b'A'; 600];
        if shape != "plain" {
            let at = 600 - 64;
            payload[at..at + 4].copy_from_slice(b"PK\x01\x02");
            if shape == "wellformed" {
                for byte in &mut payload[at + 4..at + 46] {
                    *byte = b'B';
                }
                payload[at + 28..at + 30].copy_from_slice(&4u16.to_le_bytes());
                payload[at + 30..at + 34].copy_from_slice(&0u32.to_le_bytes());
                payload[at + 46..at + 50].copy_from_slice(b"JUNK");
            } else {
                payload[at + 28..at + 30].copy_from_slice(&0xFFFFu16.to_le_bytes());
            }
        }
        payload
    }

    /// A pack whose last entry is the Stored filler
    /// (its payload's tail is the window between the
    /// declared directory offset and the real directory
    /// once the pack is shifted), shifted by 64 junk
    /// bytes.
    fn zip_shifted_with_filler(entries: &[(&str, &[u8])], filler: &[u8]) -> Vec<u8> {
        let mut buf = Vec::new();
        {
            let mut writer = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
            let opts = zip::write::SimpleFileOptions::default();
            let stored = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            use std::io::Write as _;
            for (name, body) in entries {
                writer.start_file(*name, opts).expect("start entry");
                writer.write_all(body).expect("write entry");
            }
            writer
                .start_file("filler.bin", stored)
                .expect("filler entry");
            writer.write_all(filler).expect("filler write");
            writer.finish().expect("finish");
        }
        let mut out = vec![b'J'; 64];
        out.extend_from_slice(&buf);
        out
    }

    #[test]
    fn r33c_shifted_packs_with_signature_in_the_stored_tail_land_like_unshifted() {
        // Round 33, F3 (red team RT33-3): the shifted
        // clip pack carrying the signature in its stored
        // tail was refused wholesale, where the plain
        // shifted pack landed both copies of the
        // duplicated name. Both witnessed payload shapes
        // (and the plain shifted control) land both
        // copies now, as pic.png and pic-2.png, and the
        // same shifted shape with one picture lands it.
        // A genuinely corrupt pack still refuses.
        let first_png = png_filled(255);
        let second_png = png_filled(0);
        for shape in ["plain", "fake", "wellformed"] {
            let filler = tail_filler(shape);
            let dir = temp_dir(&format!("r33c-{shape}"));
            let lib = ClipLibrary::open(dir.clone()).expect("open");
            let mut zip_bytes = zip_shifted_with_filler(
                &[("pic.png", &first_png), ("pic.pnX", &second_png)],
                &filler,
            );
            patch_entry_name(&mut zip_bytes, "pic.pnX", "pic.png");
            let report = lib
                .add_collection_from_zip("pack.zip", &zip_bytes, &CollectionNotes::default(), None)
                .expect("add zip");
            assert_eq!(report.added, 2, "{shape}: both copies land: {report:?}");
            let collection = lib.collection("pack").expect("collection");
            assert_eq!(collection.items.len(), 2, "{shape}");
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
                "{shape}"
            );
            assert_eq!(
                std::fs::read(&second.path).expect("read second"),
                second_png,
                "{shape}"
            );
            let _ = std::fs::remove_dir_all(&dir);

            let dir = temp_dir(&format!("r33c-single-{shape}"));
            let lib = ClipLibrary::open(dir.clone()).expect("open");
            let single = zip_shifted_with_filler(&[("pic.png", &first_png)], &filler);
            let report = lib
                .add_collection_from_zip("pack.zip", &single, &CollectionNotes::default(), None)
                .expect("add single zip");
            assert_eq!(report.added, 1, "{shape}: {report:?}");
            let _ = std::fs::remove_dir_all(&dir);
        }
        let mut corrupt =
            zip_shifted_with_filler(&[("pic.png", &first_png)], &tail_filler("plain"));
        corrupt.truncate(corrupt.len() - 10);
        let dir = temp_dir("r33c-corrupt");
        let lib = ClipLibrary::open(dir.clone()).expect("open");
        let err = lib
            .add_collection_from_zip("pack.zip", &corrupt, &CollectionNotes::default(), None)
            .expect_err("a corrupt pack refuses");
        assert!(err.contains("cannot be read"), "{err}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A zip64 clip pack assembled byte by byte, its
    /// zip64 record carrying an extensible data sector
    /// of `sector` bytes (size field 44 + sector, so
    /// the arithmetic derivation refuses it and the
    /// hunted fallback is the only path), all entries
    /// Stored, optionally shifted by `shift` junk
    /// bytes. The crafter's twin of the audit lanes'
    /// mkzip64 assembly (round 34).
    fn sector_pack(entries: &[(&[u8], &[u8])], sector: usize, shift: usize) -> Vec<u8> {
        let mut out: Vec<u8> = Vec::new();
        let mut centrals: Vec<u8> = Vec::new();
        for (name, data) in entries {
            let local_offset = out.len() as u32;
            let crc = crc32_ieee(data);
            out.extend_from_slice(b"PK\x03\x04");
            out.extend_from_slice(&20u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&crc.to_le_bytes());
            out.extend_from_slice(&(data.len() as u32).to_le_bytes());
            out.extend_from_slice(&(data.len() as u32).to_le_bytes());
            out.extend_from_slice(&(name.len() as u16).to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(name);
            out.extend_from_slice(data);

            centrals.extend_from_slice(b"PK\x01\x02");
            centrals.extend_from_slice(&45u16.to_le_bytes());
            centrals.extend_from_slice(&20u16.to_le_bytes());
            centrals.extend_from_slice(&0u16.to_le_bytes());
            centrals.extend_from_slice(&0u16.to_le_bytes());
            centrals.extend_from_slice(&0u16.to_le_bytes());
            centrals.extend_from_slice(&0u16.to_le_bytes());
            centrals.extend_from_slice(&crc.to_le_bytes());
            centrals.extend_from_slice(&(data.len() as u32).to_le_bytes());
            centrals.extend_from_slice(&(data.len() as u32).to_le_bytes());
            centrals.extend_from_slice(&(name.len() as u16).to_le_bytes());
            centrals.extend_from_slice(&0u16.to_le_bytes());
            centrals.extend_from_slice(&0u16.to_le_bytes());
            centrals.extend_from_slice(&0u16.to_le_bytes());
            centrals.extend_from_slice(&0u16.to_le_bytes());
            centrals.extend_from_slice(&0u32.to_le_bytes());
            centrals.extend_from_slice(&local_offset.to_le_bytes());
            centrals.extend_from_slice(name);
        }
        let cd_offset = out.len() as u64;
        let cd_size = centrals.len() as u64;
        out.extend_from_slice(&centrals);
        let record_at = out.len() as u64;
        out.extend_from_slice(b"PK\x06\x06");
        out.extend_from_slice(&((44 + sector) as u64).to_le_bytes());
        out.extend_from_slice(&45u16.to_le_bytes());
        out.extend_from_slice(&45u16.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&(entries.len() as u64).to_le_bytes());
        out.extend_from_slice(&(entries.len() as u64).to_le_bytes());
        out.extend_from_slice(&cd_size.to_le_bytes());
        out.extend_from_slice(&cd_offset.to_le_bytes());
        out.extend_from_slice(&vec![b'S'; sector]);
        out.extend_from_slice(b"PK\x06\x07");
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&record_at.to_le_bytes());
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(b"PK\x05\x06");
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&u16::MAX.to_le_bytes());
        out.extend_from_slice(&u16::MAX.to_le_bytes());
        out.extend_from_slice(&u32::MAX.to_le_bytes());
        out.extend_from_slice(&u32::MAX.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        if shift == 0 {
            out
        } else {
            let mut moved = vec![b'J'; shift];
            moved.extend_from_slice(&out);
            moved
        }
    }

    /// The last entry name in the red team's name-fake
    /// witnesses: see the twin helper in ob-model's
    /// zip_directory tests.
    fn fake_record_name() -> Vec<u8> {
        let mut name = vec![b'A'; 30];
        name.extend_from_slice(b"PK\x06\x06");
        name.extend_from_slice(&52u64.to_le_bytes());
        name.extend_from_slice(&45u16.to_le_bytes());
        name.extend_from_slice(&45u16.to_le_bytes());
        name.extend_from_slice(&0u32.to_le_bytes());
        name.extend_from_slice(&0u32.to_le_bytes());
        name.extend_from_slice(&2u64.to_le_bytes());
        name.extend_from_slice(&2u64.to_le_bytes());
        name.extend_from_slice(&257u64.to_le_bytes());
        name.extend_from_slice(&0x7F7F7F7Fu64.to_le_bytes());
        name
    }

    #[test]
    fn r34b_sector_shifted_packs_with_fakes_in_the_hunts_windows_land_both_copies() {
        // Round 34, F2 (red team RT34-3 + review
        // MINOR-1): on a shifted zip64 pack with an
        // extensible data sector, the unvalidated hunt
        // either took a fake central signature in the
        // last entry's stored tail as the directory or
        // was steered by a fake zip64 record in the last
        // record's name to no directory at all — the
        // scans judged a fake or empty directory, the
        // reader's enumeration collapsed the duplicated
        // name to its last copy, and ONE picture landed
        // where every control lands both. The hunt
        // validates and keeps searching now: both
        // copies land as pic.png and pic-2.png in every
        // shape, shifted exactly as unshifted, and the
        // same shapes with one picture land it.
        let first_png = png_filled(255);
        let second_png = png_filled(0);
        let fake_name = fake_record_name();
        let tails: Vec<(&str, usize, Vec<u8>, Vec<u8>)> = vec![
            (
                "payload-fake-header",
                16,
                b"filler.bin".to_vec(),
                tail_filler("fake"),
            ),
            (
                "payload-wellformed",
                16,
                b"filler.bin".to_vec(),
                tail_filler("wellformed"),
            ),
            (
                "payload-clean",
                16,
                b"filler.bin".to_vec(),
                tail_filler("plain"),
            ),
            ("name-fake", 8, fake_name.clone(), b"tail".to_vec()),
        ];
        for (label, sector, tail_name, tail_data) in &tails {
            for shift in [0usize, 64] {
                let entries: Vec<(&[u8], &[u8])> = vec![
                    (b"pic.png", &first_png),
                    (b"pic.png", &second_png),
                    (tail_name.as_slice(), tail_data.as_slice()),
                ];
                let pack = sector_pack(&entries, *sector, shift);
                let dir = temp_dir(&format!("r34b-{label}-{shift}"));
                let lib = ClipLibrary::open(dir.clone()).expect("open");
                let report = lib
                    .add_collection_from_zip("pack.zip", &pack, &CollectionNotes::default(), None)
                    .expect("add zip");
                assert_eq!(
                    report.added, 2,
                    "{label} shift={shift}: both copies land: {report:?}"
                );
                let collection = lib.collection("pack").expect("collection");
                assert_eq!(collection.items.len(), 2, "{label} shift={shift}");
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
                    "{label} shift={shift}"
                );
                assert_eq!(
                    std::fs::read(&second.path).expect("read second"),
                    second_png,
                    "{label} shift={shift}"
                );
                let _ = std::fs::remove_dir_all(&dir);

                let entries: Vec<(&[u8], &[u8])> = vec![
                    (b"pic.png", &first_png),
                    (tail_name.as_slice(), tail_data.as_slice()),
                ];
                let single = sector_pack(&entries, *sector, shift);
                let dir = temp_dir(&format!("r34b-single-{label}-{shift}"));
                let lib = ClipLibrary::open(dir.clone()).expect("open");
                let report = lib
                    .add_collection_from_zip("pack.zip", &single, &CollectionNotes::default(), None)
                    .expect("add single zip");
                assert_eq!(report.added, 1, "{label} shift={shift}: {report:?}");
                let _ = std::fs::remove_dir_all(&dir);
            }
        }
    }

    /// The round-35 gap craft (red team RT35-1),
    /// assembled byte by byte in the crafter's shape:
    /// see the twin helper in ob-extract's tests.
    fn gap_pack(entries: &[(&[u8], &[u8])], fake: bool, sig: bool, zip64: bool) -> Vec<u8> {
        const GAP: u64 = 512;
        let mut body: Vec<u8> = Vec::new();
        let mut locals: Vec<u32> = Vec::new();
        for (name, data) in entries {
            locals.push(body.len() as u32);
            let crc = crc32_ieee(data);
            body.extend_from_slice(b"PK\x03\x04");
            body.extend_from_slice(&20u16.to_le_bytes());
            body.extend_from_slice(&0u16.to_le_bytes());
            body.extend_from_slice(&0u16.to_le_bytes());
            body.extend_from_slice(&0u16.to_le_bytes());
            body.extend_from_slice(&0u16.to_le_bytes());
            body.extend_from_slice(&crc.to_le_bytes());
            body.extend_from_slice(&(data.len() as u32).to_le_bytes());
            body.extend_from_slice(&(data.len() as u32).to_le_bytes());
            body.extend_from_slice(&(name.len() as u16).to_le_bytes());
            body.extend_from_slice(&0u16.to_le_bytes());
            body.extend_from_slice(name);
            body.extend_from_slice(data);
        }
        let cd_offset = body.len() as u64 + GAP;
        let central = |name: &[u8], data: &[u8], local_offset: u32, comment: u16| {
            let crc = crc32_ieee(data);
            let mut rec: Vec<u8> = Vec::new();
            rec.extend_from_slice(b"PK\x01\x02");
            rec.extend_from_slice(&45u16.to_le_bytes());
            rec.extend_from_slice(&20u16.to_le_bytes());
            rec.extend_from_slice(&0u16.to_le_bytes());
            rec.extend_from_slice(&0u16.to_le_bytes());
            rec.extend_from_slice(&0u16.to_le_bytes());
            rec.extend_from_slice(&0u16.to_le_bytes());
            rec.extend_from_slice(&crc.to_le_bytes());
            rec.extend_from_slice(&(data.len() as u32).to_le_bytes());
            rec.extend_from_slice(&(data.len() as u32).to_le_bytes());
            rec.extend_from_slice(&(name.len() as u16).to_le_bytes());
            rec.extend_from_slice(&0u16.to_le_bytes());
            rec.extend_from_slice(&comment.to_le_bytes());
            rec.extend_from_slice(&0u16.to_le_bytes());
            rec.extend_from_slice(&0u16.to_le_bytes());
            rec.extend_from_slice(&0u32.to_le_bytes());
            rec.extend_from_slice(&local_offset.to_le_bytes());
            rec.extend_from_slice(name);
            rec.extend_from_slice(&vec![b'C'; usize::from(comment)]);
            rec
        };
        let mut centrals: Vec<u8> = Vec::new();
        for (i, (name, data)) in entries.iter().enumerate() {
            centrals.extend_from_slice(&central(name, data, locals[i], 0));
        }
        let sig_len: u64 = if sig { 22 } else { 0 };
        let cd_size = centrals.len() as u64 + sig_len;
        let mut fake_recs: Vec<u8> = Vec::new();
        for (i, (name, _)) in entries.iter().enumerate() {
            let first = entries
                .iter()
                .position(|(other, _)| other == name)
                .expect("the entry's own first copy stands");
            let comment = if i + 1 == entries.len() {
                u16::try_from(cd_size - fake_recs.len() as u64 - 46 - name.len() as u64)
                    .expect("the fake walk pads to the declared size")
            } else {
                0
            };
            fake_recs.extend_from_slice(&central(
                name,
                entries[first].1,
                locals[first] + GAP as u32,
                comment,
            ));
        }
        assert_eq!(fake_recs.len() as u64, cd_size);
        assert!(fake_recs.len() as u64 <= GAP);
        let mut gap = vec![b'G'; GAP as usize];
        if fake {
            gap[..fake_recs.len()].copy_from_slice(&fake_recs);
        }
        let mut out: Vec<u8> = Vec::new();
        out.extend_from_slice(&body);
        out.extend_from_slice(&gap);
        out.extend_from_slice(&centrals);
        if sig {
            out.extend_from_slice(b"PK\x05\x05");
            out.extend_from_slice(&16u16.to_le_bytes());
            out.extend_from_slice(b"SSSSSSSSSSSSSSSS");
        }
        if zip64 {
            let record_at = out.len() as u64;
            out.extend_from_slice(b"PK\x06\x06");
            out.extend_from_slice(&44u64.to_le_bytes());
            out.extend_from_slice(&45u16.to_le_bytes());
            out.extend_from_slice(&45u16.to_le_bytes());
            out.extend_from_slice(&0u32.to_le_bytes());
            out.extend_from_slice(&0u32.to_le_bytes());
            out.extend_from_slice(&(entries.len() as u64).to_le_bytes());
            out.extend_from_slice(&(entries.len() as u64).to_le_bytes());
            out.extend_from_slice(&cd_size.to_le_bytes());
            out.extend_from_slice(&cd_offset.to_le_bytes());
            out.extend_from_slice(b"PK\x06\x07");
            out.extend_from_slice(&0u32.to_le_bytes());
            out.extend_from_slice(&record_at.to_le_bytes());
            out.extend_from_slice(&1u32.to_le_bytes());
            out.extend_from_slice(b"PK\x05\x06");
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&u16::MAX.to_le_bytes());
            out.extend_from_slice(&u16::MAX.to_le_bytes());
            out.extend_from_slice(&u32::MAX.to_le_bytes());
            out.extend_from_slice(&u32::MAX.to_le_bytes());
        } else {
            out.extend_from_slice(b"PK\x05\x06");
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
            out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
            out.extend_from_slice(&(cd_size as u32).to_le_bytes());
            out.extend_from_slice(&(cd_offset as u32).to_le_bytes());
        }
        out.extend_from_slice(&0u16.to_le_bytes());
        let mut moved = vec![b'J'; 4096];
        moved.extend_from_slice(&out);
        moved
    }

    #[test]
    fn r35d_a_fake_directory_in_the_gap_does_not_steer_the_pack() {
        // Round 35, F4 (red team RT35-1): on the gap
        // craft the signature record broke the
        // arithmetic derivation and the plain hunt
        // believed a fake directory planted in the gap
        // before the real one — its records replaced
        // the duplicated name's second copy with the
        // first copy's record, so two items landed
        // both carrying the first picture's bytes. The
        // real directory is judged now: both copies
        // land, as pic.png and pic-2.png, each holding
        // its own bytes, in the craft and in all three
        // controls — gap-clean (gap and signature
        // record, no fake), no-signature (fake planted,
        // the arithmetic walk unbroken), and the zip64
        // twin (its hunt already carried the offset
        // anchor).
        let first_png = png_filled(255);
        let second_png = png_filled(0);
        let filler = vec![b'A'; 6000];
        let shapes: [(&str, bool, bool, bool); 4] = [
            ("gapfake", true, true, false),
            ("gapclean", false, true, false),
            ("gapnosig", true, false, false),
            ("zip64gapfake", true, true, true),
        ];
        for (label, fake, sig, zip64) in shapes {
            let entries: Vec<(&[u8], &[u8])> = vec![
                (b"pic.png", &first_png),
                (b"pic.png", &second_png),
                (b"filler.bin", &filler),
            ];
            let pack = gap_pack(&entries, fake, sig, zip64);
            let dir = temp_dir(&format!("r35d-{label}"));
            let lib = ClipLibrary::open(dir.clone()).expect("open");
            let report = lib
                .add_collection_from_zip("pack.zip", &pack, &CollectionNotes::default(), None)
                .expect("add zip");
            assert_eq!(report.added, 2, "{label}: both copies land: {report:?}");
            let collection = lib.collection("pack").expect("collection");
            assert_eq!(collection.items.len(), 2, "{label}");
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
                "{label}: the first item holds the first copy's bytes"
            );
            assert_eq!(
                std::fs::read(&second.path).expect("read second"),
                second_png,
                "{label}: the second item holds the second copy's bytes"
            );
            let _ = std::fs::remove_dir_all(&dir);
        }
    }

    /// One plain central record for the round-36
    /// crafts: Stored, no comment, the given extra
    /// field riding after the name.
    fn plain_central(name: &[u8], data: &[u8], local_offset: u32, extra: &[u8]) -> Vec<u8> {
        let crc = crc32_ieee(data);
        let mut rec: Vec<u8> = Vec::new();
        rec.extend_from_slice(b"PK\x01\x02");
        rec.extend_from_slice(&45u16.to_le_bytes());
        rec.extend_from_slice(&20u16.to_le_bytes());
        rec.extend_from_slice(&0u16.to_le_bytes());
        rec.extend_from_slice(&0u16.to_le_bytes());
        rec.extend_from_slice(&0u16.to_le_bytes());
        rec.extend_from_slice(&0u16.to_le_bytes());
        rec.extend_from_slice(&crc.to_le_bytes());
        rec.extend_from_slice(&(data.len() as u32).to_le_bytes());
        rec.extend_from_slice(&(data.len() as u32).to_le_bytes());
        rec.extend_from_slice(&(name.len() as u16).to_le_bytes());
        rec.extend_from_slice(&(extra.len() as u16).to_le_bytes());
        rec.extend_from_slice(&0u16.to_le_bytes());
        rec.extend_from_slice(&0u16.to_le_bytes());
        rec.extend_from_slice(&0u16.to_le_bytes());
        rec.extend_from_slice(&0u32.to_le_bytes());
        rec.extend_from_slice(&local_offset.to_le_bytes());
        rec.extend_from_slice(name);
        rec.extend_from_slice(extra);
        rec
    }

    /// One plain end record with the given disk field
    /// and comment, the tail of the round-36 crafts.
    fn plain_eocd(total: u16, cd_size: u32, cd_offset: u32, disk: u16, comment: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"PK\x05\x06");
        out.extend_from_slice(&disk.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&total.to_le_bytes());
        out.extend_from_slice(&total.to_le_bytes());
        out.extend_from_slice(&cd_size.to_le_bytes());
        out.extend_from_slice(&cd_offset.to_le_bytes());
        out.extend_from_slice(&(comment.len() as u16).to_le_bytes());
        out.extend_from_slice(comment);
        out
    }

    /// The round-36 second-directory crafts (review
    /// R36-6 + red team RT36-5 + review R36-7's
    /// compound), assembled byte by byte over Stored
    /// entries, in the IDML door's crafter shape:
    /// "plain" the real directory alone;
    /// "appended-clean" / "appended-dirty" a second
    /// directory naming each part once with its own
    /// end record (disk field 0 or 1); "comment" that
    /// second directory planted inside the real end
    /// record's declared comment; "steered" an
    /// appended directory whose extra record
    /// duplicates the first repeated name with a zip64
    /// extra resolving its local offset to
    /// u64::MAX - 10; "direct-steered" the real
    /// directory's own first record carrying that
    /// steered offset.
    fn second_directory_pack(entries: &[(&[u8], &[u8])], shape: &str) -> Vec<u8> {
        let mut out: Vec<u8> = Vec::new();
        let mut locals: Vec<u32> = Vec::new();
        for (name, data) in entries {
            locals.push(out.len() as u32);
            let crc = crc32_ieee(data);
            out.extend_from_slice(b"PK\x03\x04");
            out.extend_from_slice(&20u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&crc.to_le_bytes());
            out.extend_from_slice(&(data.len() as u32).to_le_bytes());
            out.extend_from_slice(&(data.len() as u32).to_le_bytes());
            out.extend_from_slice(&(name.len() as u16).to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(name);
            out.extend_from_slice(data);
        }
        let cd_offset = out.len() as u32;
        let steered_record = |entries: &[(&[u8], &[u8])]| -> Vec<u8> {
            let dup_at = entries
                .iter()
                .position(|(name, _)| entries.iter().filter(|(n, _)| n == name).count() > 1)
                .expect("a repeated name stands");
            let (name, _) = entries[dup_at];
            let second = entries
                .iter()
                .enumerate()
                .filter(|(i, (n, _))| *n == name && *i != dup_at)
                .map(|(_, (_, data))| *data)
                .next()
                .expect("the repeated name's second copy stands");
            let mut extra: Vec<u8> = Vec::new();
            extra.extend_from_slice(&1u16.to_le_bytes());
            extra.extend_from_slice(&8u16.to_le_bytes());
            extra.extend_from_slice(&(u64::MAX - 10).to_le_bytes());
            plain_central(name, second, u32::MAX, &extra)
        };
        let mut centrals: Vec<u8> = Vec::new();
        for (i, (name, data)) in entries.iter().enumerate() {
            if shape == "direct-steered" && i == 0 {
                let mut extra: Vec<u8> = Vec::new();
                extra.extend_from_slice(&1u16.to_le_bytes());
                extra.extend_from_slice(&8u16.to_le_bytes());
                extra.extend_from_slice(&(u64::MAX - 10).to_le_bytes());
                centrals.extend_from_slice(&plain_central(name, data, u32::MAX, &extra));
            } else {
                centrals.extend_from_slice(&plain_central(name, data, locals[i], &[]));
            }
        }
        let mut fake_recs: Vec<u8> = Vec::new();
        let mut fake_total: u16 = 0;
        for (i, (name, data)) in entries.iter().enumerate() {
            if entries[..i].iter().any(|(other, _)| other == name) {
                continue;
            }
            fake_recs.extend_from_slice(&plain_central(name, data, locals[i], &[]));
            fake_total += 1;
        }
        out.extend_from_slice(&centrals);
        match shape {
            "plain" | "direct-steered" => {
                out.extend_from_slice(&plain_eocd(
                    entries.len() as u16,
                    centrals.len() as u32,
                    cd_offset,
                    0,
                    &[],
                ));
            }
            "comment" => {
                let planted_offset = cd_offset + centrals.len() as u32 + 22;
                let mut comment = fake_recs.clone();
                comment.extend_from_slice(&plain_eocd(
                    fake_total,
                    fake_recs.len() as u32,
                    planted_offset,
                    0,
                    &[],
                ));
                out.extend_from_slice(&plain_eocd(
                    entries.len() as u16,
                    centrals.len() as u32,
                    cd_offset,
                    0,
                    &comment,
                ));
            }
            _ => {
                let disk = if shape == "appended-dirty" { 1 } else { 0 };
                out.extend_from_slice(&plain_eocd(
                    entries.len() as u16,
                    centrals.len() as u32,
                    cd_offset,
                    0,
                    &[],
                ));
                let fake_offset = out.len() as u32;
                let mut appended = fake_recs.clone();
                let mut appended_total = fake_total;
                if shape == "steered" {
                    appended.extend_from_slice(&steered_record(entries));
                    appended_total += 1;
                }
                out.extend_from_slice(&appended);
                out.extend_from_slice(&plain_eocd(
                    appended_total,
                    appended.len() as u32,
                    fake_offset,
                    disk,
                    &[],
                ));
            }
        }
        out
    }

    #[test]
    fn r36a_a_second_directory_never_steers_the_pack() {
        // Round 36, F6 (review R36-6 + red team
        // RT36-5): a second self-consistent central
        // directory — appended past the real end
        // record, or planted inside the real end
        // record's own comment — judged the pack
        // instead of the real directory, so the red
        // picture landed twice and the blue picture
        // never landed at all. The real directory is
        // judged in every shape now: both pictures
        // land, the first as pic.png and the second
        // as pic-2.png, each holding its own bytes,
        // exactly as the plain control lands them
        // (the r32 F6 pin above).
        let red = png_filled(255);
        let blue = png_filled(0);
        for shape in [
            "plain",
            "appended-clean",
            "appended-dirty",
            "comment",
            "steered",
        ] {
            let entries: Vec<(&[u8], &[u8])> = vec![
                (b"pic.png", &red),
                (b"pic.png", &blue),
                (b"filler.bin", b"tail"),
            ];
            let bytes = second_directory_pack(&entries, shape);
            let dir = temp_dir(&format!("r36a-{shape}"));
            let lib = ClipLibrary::open(dir.clone()).expect("open");
            let report = lib
                .add_collection_from_zip("pack.zip", &bytes, &CollectionNotes::default(), None)
                .expect("add zip");
            assert_eq!(report.added, 2, "{shape}: both copies land: {report:?}");
            let collection = lib.collection("pack").expect("collection");
            assert_eq!(collection.items.len(), 2, "{shape}");
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
                red,
                "{shape}: the first item holds the first copy's bytes"
            );
            assert_eq!(
                std::fs::read(&second.path).expect("read second"),
                blue,
                "{shape}: the second item holds the second copy's bytes"
            );
            let _ = std::fs::remove_dir_all(&dir);
        }
    }

    #[test]
    fn r36a_a_direct_over_range_spot_refuses_instead_of_panicking() {
        // Round 36, F7 (review R36-7): one entry whose
        // own central record resolves its local offset
        // to u64::MAX - 10 through a zip64 extra. This
        // door calls raw_payload for every spot, so
        // the unchecked `local + 30` arithmetic stood
        // directly on this shape; the checked
        // arithmetic refuses it in this door's standing
        // words ("cannot be read"), never a panic.
        let red = png_filled(255);
        let entries: Vec<(&[u8], &[u8])> = vec![(b"pic.png", &red)];
        let bytes = second_directory_pack(&entries, "direct-steered");
        let dir = temp_dir("r36a-direct-steered");
        let lib = ClipLibrary::open(dir.clone()).expect("open");
        let err = lib
            .add_collection_from_zip("pack.zip", &bytes, &CollectionNotes::default(), None)
            .expect_err("the over-range pack refuses");
        assert!(err.contains("cannot be read"), "{err}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn r34b_a_pack_the_reader_opens_with_no_derivable_directory_refuses() {
        // Round 34, F2, the fail-closed side: the
        // name-fake pack above, with the real zip64
        // record's declared directory size corrupted
        // (ten bytes more than its directory holds), so
        // the record does not validate against its
        // locator and NO directory can be derived — yet
        // the reader opens the pack anyway. Before this
        // batch the hunt was steered by the fake record
        // in the name to no directory at all and the
        // fallback to the reader's enumeration landed
        // only the last copy of the duplicated picture.
        // The door refuses the pack in its standing
        // words now.
        let first_png = png_filled(255);
        let second_png = png_filled(0);
        let fake_name = fake_record_name();
        let entries: Vec<(&[u8], &[u8])> = vec![
            (b"pic.png", &first_png),
            (b"pic.png", &second_png),
            (fake_name.as_slice(), b"tail"),
        ];
        let mut pack = sector_pack(&entries, 8, 64);
        // The real zip64 record is the LAST PK\x06\x06
        // in the file (the fake stands earlier, in the
        // last record's name); its directory-size field
        // stands 40 bytes in.
        let record_at = pack
            .windows(4)
            .rposition(|w| w == b"PK\x06\x06")
            .expect("the real zip64 record stands in the bytes");
        let cd_size = u64::from_le_bytes(pack[record_at + 40..record_at + 48].try_into().unwrap());
        pack[record_at + 40..record_at + 48].copy_from_slice(&(cd_size + 10).to_le_bytes());
        let dir = temp_dir("r34b-nodir");
        let lib = ClipLibrary::open(dir.clone()).expect("open");
        let err = lib
            .add_collection_from_zip("pack.zip", &pack, &CollectionNotes::default(), None)
            .expect_err("a reader-openable pack with no derivable directory refuses");
        assert!(err.contains("cannot be read"), "{err}");
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
