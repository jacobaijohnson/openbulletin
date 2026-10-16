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
    pub items: Vec<ClipItem>,
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
        let mut items = Vec::new();
        collect_items(&path, &path, &mut items);
        items.sort_by_key(|a| a.file_name.to_lowercase());
        Collection {
            name,
            path,
            notes,
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
                    report.added += 1;
                }
                Candidate::Metafile => match convert.and_then(|c| c(&base, bytes)) {
                    Some(png) => {
                        let stem = base.rsplit_once('.').map(|(s, _)| s).unwrap_or(&base);
                        let landed = unique_name(&target, &format!("{stem}.png"), &mut used_names);
                        std::fs::write(target.join(&landed), &png)
                            .map_err(|e| format!("cannot save {landed} into the library: {e}"))?;
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
        gather_files(folder, folder, &mut files, &mut unreadable);
        let mut report = self.add_collection(&name, &files, notes, convert)?;
        report.skipped_unreadable += unreadable;
        Ok(report)
    }

    /// Add a collection from a zip archive's bytes. Archive
    /// paths are flattened to bare file names (the font
    /// library's traversal rule: nothing may write outside
    /// the collection folder, ever), and the whole archive
    /// is refused when it decompresses past the cap.
    pub fn add_collection_from_zip(
        &self,
        zip_name: &str,
        bytes: &[u8],
        notes: &CollectionNotes,
        convert: Option<&MetafileConverter>,
    ) -> Result<AddReport, String> {
        use std::io::Read as _;
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
        let mut total: u64 = 0;
        let mut files: Vec<(String, Vec<u8>)> = Vec::new();
        for i in 0..archive.len() {
            let mut entry = archive
                .by_index(i)
                .map_err(|e| format!("that zip file cannot be read: {e}"))?;
            if entry.is_dir() {
                continue;
            }
            total += entry.size();
            if total > 512 * 1024 * 1024 {
                return Err(
                    "that zip holds more than 512 MB unzipped; a clipart pack is not that big"
                        .into(),
                );
            }
            let mut buf = Vec::new();
            entry
                .read_to_end(&mut buf)
                .map_err(|e| format!("a file in that zip cannot be read: {e}"))?;
            files.push((entry.name().to_string(), buf));
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
            if base == "notes.json" {
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

fn gather_files(root: &Path, dir: &Path, out: &mut Vec<(String, Vec<u8>)>, unreadable: &mut usize) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            gather_files(root, &path, out, unreadable);
        } else if path.is_file() {
            let rel = path
                .strip_prefix(root)
                .map(|p| p.to_string_lossy().replace('\\', "/"))
                .unwrap_or_else(|_| entry.file_name().to_string_lossy().to_string());
            match std::fs::read(&path) {
                Ok(bytes) => out.push((rel, bytes)),
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
