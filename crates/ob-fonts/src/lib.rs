//! The OpenBulletin Font Library (docs/FONTS.md).
//!
//! One app-managed folder of font files, identical on a desktop and
//! in the Docker/NAS shape, because the layout engine only ever uses
//! fonts handed to it from here: the same bulletin must lay out the
//! same way on every machine. System fonts are copied in, never
//! linked live.
//!
//! Every font carries its embedding permission (the OpenType fsType
//! bits) translated into plain words, and a SHA-256 of its bytes:
//! documents name fonts by family + hash, so a bulletin that travels
//! also says exactly which font file it was laid out with.

pub mod catalog;

use serde::Serialize;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// What the font's own permission bits say about embedding it in
/// files that travel (an .obd package or a shared PDF).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum EmbeddingRights {
    /// May be embedded and installed by the receiver.
    Installable,
    /// May be embedded; receiver may edit with it temporarily.
    Editable,
    /// May be embedded for looking and printing only.
    PreviewAndPrint,
    /// May not be embedded at all.
    Restricted,
    /// The font carries no readable permission bits.
    Unknown,
}

impl EmbeddingRights {
    /// Plain-words explanation for the admin, per the ELI5 rule.
    pub fn plain_words(&self) -> &'static str {
        match self {
            EmbeddingRights::Installable => {
                "Free to share: this font may travel inside bulletin files, and whoever receives one may keep and use the font."
            }
            EmbeddingRights::Editable => {
                "May travel inside bulletin files so others can open and edit them, but it is not theirs to keep."
            }
            EmbeddingRights::PreviewAndPrint => {
                "May be used to show and print this bulletin, but the font itself must not travel inside shared files."
            }
            EmbeddingRights::Restricted => {
                "This font must not be embedded or shared. You can use it on this machine only."
            }
            EmbeddingRights::Unknown => {
                "This font does not say what sharing it allows. Treat it as this machine only, unless you know its license."
            }
        }
    }

    /// May this font's bytes ride inside an .obd package?
    pub fn may_package(&self) -> bool {
        matches!(
            self,
            EmbeddingRights::Installable | EmbeddingRights::Editable
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FontInfo {
    /// Family name, e.g. "Liberation Sans".
    pub family: String,
    /// Full face name, e.g. "Liberation Sans Bold".
    pub full_name: String,
    pub bold: bool,
    pub italic: bool,
    pub rights: EmbeddingRights,
    /// True when the font forbids subsetting before embedding.
    pub no_subsetting: bool,
    /// File name inside the library folder.
    pub file_name: String,
    /// Hex SHA-256 of the font file bytes.
    pub sha256: String,
    pub bytes: usize,
}

/// Read the facts out of one font file's bytes.
pub fn inspect(bytes: &[u8], file_name: &str) -> Result<FontInfo, String> {
    let face = ttf_parser::Face::parse(bytes, 0)
        .map_err(|e| format!("not a readable font file: {e:?}"))?;
    let name = |id: u16| -> Option<String> {
        face.names()
            .into_iter()
            .find(|n| n.name_id == id && n.is_unicode())
            .and_then(|n| n.to_string())
    };
    let family = name(ttf_parser::name_id::TYPOGRAPHIC_FAMILY)
        .or_else(|| name(ttf_parser::name_id::FAMILY))
        .ok_or("the font does not carry a family name")?;
    let full_name = name(ttf_parser::name_id::FULL_NAME).unwrap_or_else(|| family.clone());
    let (rights, no_subsetting) = match face.tables().os2 {
        Some(os2) => {
            use ttf_parser::Permissions::*;
            let rights = match os2.permissions() {
                Some(Installable) => EmbeddingRights::Installable,
                Some(Editable) => EmbeddingRights::Editable,
                Some(PreviewAndPrint) => EmbeddingRights::PreviewAndPrint,
                Some(Restricted) => EmbeddingRights::Restricted,
                None => EmbeddingRights::Unknown,
            };
            (rights, !os2.is_subsetting_allowed())
        }
        None => (EmbeddingRights::Unknown, false),
    };
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    Ok(FontInfo {
        family,
        full_name,
        bold: face.is_bold(),
        italic: face.is_italic(),
        rights,
        no_subsetting,
        file_name: file_name.to_string(),
        sha256: format!("{:x}", hasher.finalize()),
        bytes: bytes.len(),
    })
}

/// The default library folder: next to the app settings, in an
/// `OpenBulletin/fonts` folder, on every platform shape.
pub fn default_library_dir() -> PathBuf {
    let base = if let Ok(appdata) = std::env::var("APPDATA") {
        PathBuf::from(appdata)
    } else if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
        PathBuf::from(xdg)
    } else if let Ok(home) = std::env::var("HOME") {
        let mac = PathBuf::from(&home).join("Library/Application Support");
        if mac.exists() {
            mac
        } else {
            PathBuf::from(home).join(".local/share")
        }
    } else {
        PathBuf::from(".")
    };
    base.join("OpenBulletin").join("fonts")
}

/// Reduce any suggested or archived name to a bare file name:
/// split on both slash kinds (a zip made on Windows can carry
/// backslash paths), drop anything empty or dotty.
pub fn sanitize_file_name(name: &str) -> String {
    let base = name
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("")
        .trim()
        .trim_start_matches('.')
        .to_string();
    if base.is_empty() {
        "font.ttf".to_string()
    } else {
        base
    }
}

pub struct FontLibrary {
    dir: PathBuf,
}

impl FontLibrary {
    /// Open (creating if needed) the library at `dir`.
    pub fn open(dir: PathBuf) -> Result<FontLibrary, String> {
        std::fs::create_dir_all(&dir)
            .map_err(|e| format!("cannot create font library folder: {e}"))?;
        Ok(FontLibrary { dir })
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Every readable font in the folder, sorted by family then face.
    pub fn scan(&self) -> Vec<FontInfo> {
        let mut out = Vec::new();
        let Ok(entries) = std::fs::read_dir(&self.dir) else {
            return out;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let ext = path
                .extension()
                .map(|e| e.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            if ext != "ttf" && ext != "otf" {
                continue;
            }
            if let Ok(bytes) = std::fs::read(&path) {
                let file_name = entry.file_name().to_string_lossy().to_string();
                if let Ok(info) = inspect(&bytes, &file_name) {
                    out.push(info);
                }
            }
        }
        out.sort_by(|a, b| a.family.cmp(&b.family).then(a.full_name.cmp(&b.full_name)));
        out
    }

    /// Copy a font into the library. Identical bytes already present
    /// are returned as-is (no duplicates).
    pub fn add_bytes(&self, bytes: &[u8], suggested_name: &str) -> Result<FontInfo, String> {
        let mut known: std::collections::HashSet<String> =
            self.scan().iter().map(|f| f.sha256.clone()).collect();
        self.add_bytes_known(bytes, suggested_name, &mut known)
    }

    /// add_bytes with the library's known hashes already collected:
    /// importing a whole zip used to re-scan (re-read, re-parse,
    /// re-hash) the entire library for every font, which made big
    /// packs quadratic (closing audit). One scan per batch now.
    /// Names are sanitized to a bare file name on every platform:
    /// an archive entry like ..\..\evil.ttf must never write
    /// outside the library (closing audit, Windows traversal).
    fn add_bytes_known(
        &self,
        bytes: &[u8],
        suggested_name: &str,
        known: &mut std::collections::HashSet<String>,
    ) -> Result<FontInfo, String> {
        const MAX_FONT_BYTES: usize = 64 * 1024 * 1024;
        if bytes.len() > MAX_FONT_BYTES {
            return Err("that font file is larger than 64 MB; real fonts are not".into());
        }
        let clean = sanitize_file_name(suggested_name);
        let mut info = inspect(bytes, &clean)?;
        if known.contains(&info.sha256) {
            // Return the existing entry's facts when we can find them.
            if let Some(existing) = self.scan().into_iter().find(|f| f.sha256 == info.sha256) {
                return Ok(existing);
            }
            return Ok(info);
        }
        // A DIFFERENT font already owns this file name: never
        // overwrite it (a bulletin pinned to the old font would
        // silently change face). Write under a hashed name.
        let target = self.dir.join(&clean);
        let final_name = if target.exists() {
            let short: String = info.sha256.chars().take(8).collect();
            let hashed = format!("{}-{short}", clean);
            info.file_name = hashed.clone();
            hashed
        } else {
            clean
        };
        std::fs::write(self.dir.join(&final_name), bytes)
            .map_err(|e| format!("cannot save font into the library: {e}"))?;
        known.insert(info.sha256.clone());
        Ok(info)
    }

    /// Add a font from a file on disk (a download, a USB stick, a
    /// system font being copied in). Same door as add_bytes.
    pub fn add_path(&self, path: &std::path::Path) -> Result<FontInfo, String> {
        let bytes =
            std::fs::read(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("font.ttf");
        self.add_bytes(&bytes, name)
    }

    /// Add every font inside a zip archive (how font sites ship
    /// packs). License text files in the archive are copied into
    /// the library folder beside the fonts. Returns one result per
    /// font file found; a bad file never sinks the rest.
    pub fn add_zip_bytes(&self, bytes: &[u8]) -> Vec<Result<FontInfo, String>> {
        // Caps against hostile or broken archives (closing
        // audit): a small zip must not decompress to gigabytes,
        // and a pack means dozens of fonts, not thousands.
        const MAX_ENTRIES: usize = 512;
        const MAX_ENTRY_BYTES: u64 = 64 * 1024 * 1024;
        const MAX_TOTAL_BYTES: u64 = 256 * 1024 * 1024;
        let mut results = Vec::new();
        let cursor = std::io::Cursor::new(bytes.to_vec());
        let Ok(mut archive) = zip::ZipArchive::new(cursor) else {
            return vec![Err("that file is not a readable zip archive".to_string())];
        };
        let mut known: std::collections::HashSet<String> =
            self.scan().iter().map(|f| f.sha256.clone()).collect();
        let mut total: u64 = 0;
        for i in 0..archive.len().min(MAX_ENTRIES) {
            let Ok(mut entry) = archive.by_index(i) else {
                continue;
            };
            let name = entry.name().to_string();
            let lower = name.to_lowercase();
            let file_name = sanitize_file_name(&name);
            let mut content = Vec::new();
            let mut limited = std::io::Read::take(&mut entry, MAX_ENTRY_BYTES + 1);
            let read = std::io::Read::read_to_end(&mut limited, &mut content);
            if read.is_err() || content.len() as u64 > MAX_ENTRY_BYTES {
                continue;
            }
            total += content.len() as u64;
            if total > MAX_TOTAL_BYTES {
                results.push(Err(
                    "that archive holds more than 256 MB of files; stopping there".to_string(),
                ));
                break;
            }
            if lower.ends_with(".ttf") || lower.ends_with(".otf") {
                results.push(self.add_bytes_known(&content, &file_name, &mut known));
            } else if (lower.ends_with(".txt") || lower.ends_with(".md"))
                && (lower.contains("license") || lower.contains("ofl") || lower.contains("copying"))
            {
                let _ = std::fs::write(self.dir.join(&file_name), &content);
            }
        }
        if results.is_empty() {
            results.push(Err(
                "no font files (.ttf or .otf) inside that zip".to_string()
            ));
        }
        results
    }

    /// The library folder itself (for cheap state fingerprints).
    pub fn dir_path(&self) -> &std::path::Path {
        &self.dir
    }

    /// The bytes of one library font, for handing to the text engine.
    pub fn read(&self, info: &FontInfo) -> Result<Vec<u8>, String> {
        std::fs::read(self.dir.join(&info.file_name))
            .map_err(|e| format!("cannot read font {}: {e}", info.file_name))
    }
}

/// One font found on this computer, outside the library: its facts
/// and where the file lives. The panel offers these with an
/// "add a copy" button; adding copies the file into the library
/// (copy-in, never a live link: docs/FONTS.md).
pub struct SystemFont {
    pub info: FontInfo,
    pub path: std::path::PathBuf,
}

/// The folders this operating system keeps fonts in.
fn system_font_dirs() -> Vec<std::path::PathBuf> {
    let mut dirs: Vec<std::path::PathBuf> = Vec::new();
    let home = std::env::var_os("HOME").map(std::path::PathBuf::from);
    if cfg!(target_os = "windows") {
        let windir = std::env::var_os("WINDIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::path::PathBuf::from(r"C:\Windows"));
        dirs.push(windir.join("Fonts"));
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            dirs.push(
                std::path::PathBuf::from(local)
                    .join("Microsoft")
                    .join("Windows")
                    .join("Fonts"),
            );
        }
    } else if cfg!(target_os = "macos") {
        dirs.push(std::path::PathBuf::from("/System/Library/Fonts"));
        dirs.push(std::path::PathBuf::from("/Library/Fonts"));
        if let Some(h) = &home {
            dirs.push(h.join("Library").join("Fonts"));
        }
    } else {
        dirs.push(std::path::PathBuf::from("/usr/share/fonts"));
        dirs.push(std::path::PathBuf::from("/usr/local/share/fonts"));
        if let Some(h) = &home {
            dirs.push(h.join(".local").join("share").join("fonts"));
            dirs.push(h.join(".fonts"));
        }
    }
    dirs
}

/// Every font file on this computer (recursive, .ttf and .otf),
/// deduplicated by hash. Font collections (.ttc) are skipped in
/// this version: a collection holds many faces and needs
/// face-by-face handling.
pub fn system_fonts() -> Vec<SystemFont> {
    fn walk(
        dir: &std::path::Path,
        out: &mut Vec<SystemFont>,
        seen: &mut std::collections::HashSet<String>,
    ) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out, seen);
                continue;
            }
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            if ext != "ttf" && ext != "otf" {
                continue;
            }
            let Ok(bytes) = std::fs::read(&path) else {
                continue;
            };
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("font")
                .to_string();
            if let Ok(info) = inspect(&bytes, &name) {
                if seen.insert(info.sha256.clone()) {
                    out.push(SystemFont { info, path });
                }
            }
        }
    }
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for dir in system_font_dirs() {
        walk(&dir, &mut out, &mut seen);
    }
    out.sort_by(|a, b| {
        a.info
            .family
            .cmp(&b.info.family)
            .then(a.info.bold.cmp(&b.info.bold))
            .then(a.info.italic.cmp(&b.info.italic))
    });
    out
}

/// Record, on each of the document's styles, the SHA-256 of the
/// exact library font file that style will be laid out with
/// (docs/FONTS.md). Run before saving or exporting, so the file
/// remembers which font bytes its layout depends on. Styles whose
/// family is not in the library keep whatever pin they had.
pub fn pin_document_fonts(doc: &mut ob_model::Document, lib: &FontLibrary) {
    let fonts = lib.scan();
    for style in &mut doc.styles {
        let face = fonts
            .iter()
            .filter(|f| f.family == style.font_family)
            .find(|f| f.bold == style.bold && !f.italic)
            .or_else(|| fonts.iter().find(|f| f.family == style.font_family));
        if let Some(face) = face {
            style.font_sha256 = Some(face.sha256.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bundled(name: &str) -> Vec<u8> {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../crates/ob-text/assets/fonts")
            .join(name);
        std::fs::read(&path).expect("bundled font present")
    }

    #[test]
    fn bundled_liberation_faces_inspect_cleanly() {
        let regular = inspect(
            &bundled("LiberationSans-Regular.ttf"),
            "LiberationSans-Regular.ttf",
        )
        .expect("inspect sans regular");
        assert_eq!(regular.family, "Liberation Sans");
        assert!(!regular.bold);
        let bold = inspect(
            &bundled("LiberationSans-Bold.ttf"),
            "LiberationSans-Bold.ttf",
        )
        .expect("inspect sans bold");
        assert!(bold.bold);
        let serif = inspect(
            &bundled("LiberationSerif-Regular.ttf"),
            "LiberationSerif-Regular.ttf",
        )
        .expect("inspect serif");
        assert_eq!(serif.family, "Liberation Serif");
        // Liberation ships with installable embedding (fsType 0).
        assert_eq!(regular.rights, EmbeddingRights::Installable);
        assert!(regular.rights.may_package());
        assert!(!regular.rights.plain_words().is_empty());
    }

    fn temp_dir(tag: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("ob-fonts-test-{tag}-{}", std::process::id()))
    }

    #[test]
    fn names_are_sanitized_on_every_platform() {
        assert_eq!(sanitize_file_name("../../evil.ttf"), "evil.ttf");
        assert_eq!(sanitize_file_name("..\\..\\evil.ttf"), "evil.ttf");
        assert_eq!(sanitize_file_name("a/b\\c.ttf"), "c.ttf");
        assert_eq!(sanitize_file_name(".."), "font.ttf");
        assert_eq!(
            sanitize_file_name("Carlito-Regular.ttf"),
            "Carlito-Regular.ttf"
        );
    }

    #[test]
    fn zip_traversal_names_stay_in_the_library() {
        let dir = temp_dir("traversal");
        let lib = FontLibrary::open(dir.clone()).expect("open");
        let font = std::fs::read("../ob-text/assets/fonts/LiberationSans-Regular.ttf")
            .expect("bundled font");
        // Build a zip whose entry name tries to escape.
        let mut zip_bytes = Vec::new();
        {
            let cursor = std::io::Cursor::new(&mut zip_bytes);
            let mut zw = zip::ZipWriter::new(cursor);
            let opts = zip::write::SimpleFileOptions::default();
            zw.start_file("..\\..\\escaped.ttf", opts).expect("start");
            std::io::Write::write_all(&mut zw, &font).expect("write font");
            zw.finish().expect("finish zip");
        }
        let results = lib.add_zip_bytes(&zip_bytes);
        assert!(
            results.iter().any(|r| r.is_ok()),
            "the font itself is fine: {results:?}"
        );
        assert!(
            !dir.parent().expect("parent").join("escaped.ttf").exists()
                && !std::path::Path::new("escaped.ttf").exists(),
            "nothing may be written outside the library"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_second_font_never_overwrites_a_namesake() {
        let dir = temp_dir("collision");
        let lib = FontLibrary::open(dir.clone()).expect("open");
        let a =
            std::fs::read("../ob-text/assets/fonts/LiberationSans-Regular.ttf").expect("font a");
        let b =
            std::fs::read("../ob-text/assets/fonts/LiberationSerif-Regular.ttf").expect("font b");
        let first = lib.add_bytes(&a, "Same.ttf").expect("add a");
        let second = lib.add_bytes(&b, "Same.ttf").expect("add b");
        assert_ne!(first.file_name, second.file_name);
        // Both still read back as themselves.
        assert_eq!(lib.read(&first).expect("read a"), a);
        assert_eq!(lib.read(&second).expect("read b"), b);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn restricted_fonts_may_not_be_packaged() {
        // Patch the fsType of a real font to Restricted (0x0002)
        // in its OS/2 table and confirm the rights gate closes.
        let mut bytes = std::fs::read("../ob-text/assets/fonts/LiberationSans-Regular.ttf")
            .expect("bundled font");
        let num_tables = u16::from_be_bytes([bytes[4], bytes[5]]) as usize;
        let mut patched = false;
        for i in 0..num_tables {
            let rec = 12 + i * 16;
            if &bytes[rec..rec + 4] == b"OS/2" {
                let off = u32::from_be_bytes([
                    bytes[rec + 8],
                    bytes[rec + 9],
                    bytes[rec + 10],
                    bytes[rec + 11],
                ]) as usize;
                // fsType is the uint16 at OS/2 + 8.
                bytes[off + 8] = 0x00;
                bytes[off + 9] = 0x02;
                patched = true;
            }
        }
        assert!(patched, "the test font has an OS/2 table");
        let info = inspect(&bytes, "Restricted.ttf").expect("inspect");
        assert!(
            !info.rights.may_package(),
            "restricted fonts must not package"
        );
        assert!(
            info.rights.plain_words().to_lowercase().contains("cannot")
                || info.rights.plain_words().to_lowercase().contains("not"),
            "the refusal speaks plainly: {}",
            info.rights.plain_words()
        );
    }

    #[test]
    fn library_add_scan_dedup_round_trip() {
        let dir = std::env::temp_dir().join(format!("ob-fonts-test-{}", std::process::id()));
        let lib = FontLibrary::open(dir.clone()).expect("open library");
        let bytes = bundled("LiberationSans-Regular.ttf");
        let info = lib
            .add_bytes(&bytes, "LiberationSans-Regular.ttf")
            .expect("add font");
        let again = lib
            .add_bytes(&bytes, "LiberationSans-Regular.ttf")
            .expect("add same font again");
        assert_eq!(info.sha256, again.sha256);
        let scanned = lib.scan();
        assert_eq!(
            scanned.len(),
            1,
            "duplicate bytes must not duplicate the font"
        );
        assert_eq!(lib.read(&scanned[0]).expect("read back").len(), info.bytes);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn garbage_is_refused() {
        assert!(inspect(b"this is not a font", "junk.ttf").is_err());
    }

    #[test]
    fn zip_import_takes_fonts_and_license() {
        use std::io::Write as _;
        let mut buf = Vec::new();
        {
            let mut zw = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
            let opts = zip::write::SimpleFileOptions::default();
            zw.start_file("Lato-Regular.ttf", opts).expect("zip entry");
            zw.write_all(&bundled("LiberationSans-Regular.ttf"))
                .expect("zip write");
            zw.start_file("Lato-Bold.ttf", opts).expect("zip entry");
            zw.write_all(&bundled("LiberationSans-Bold.ttf"))
                .expect("zip write");
            zw.start_file("OFL.txt", opts).expect("zip entry");
            zw.write_all(b"SIL Open Font License").expect("zip write");
            zw.finish().expect("zip finish");
        }
        let dir = std::env::temp_dir().join(format!("ob-fonts-zip-{}", std::process::id()));
        let lib = FontLibrary::open(dir.clone()).expect("open library");
        let results = lib.add_zip_bytes(&buf);
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.is_ok()));
        assert!(dir.join("OFL.txt").exists(), "license text kept");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn system_scan_never_errors() {
        // Whatever this machine has (possibly nothing), the scan
        // returns parsed facts. (It used to also assert each path
        // exists; a font can vanish mid-scan on a live system,
        // which made the test environment-flaky — closing audit.)
        for found in system_fonts() {
            assert!(!found.info.family.is_empty());
        }
    }

    #[test]
    fn pinning_records_the_library_files_hash() {
        let dir = std::env::temp_dir().join(format!("ob-fonts-pin-{}", std::process::id()));
        let lib = FontLibrary::open(dir.clone()).expect("open library");
        let info = lib
            .add_bytes(
                &bundled("LiberationSans-Regular.ttf"),
                "LiberationSans-Regular.ttf",
            )
            .expect("add font");
        let mut doc = ob_model::Document {
            format_version: ob_model::FORMAT_VERSION,
            title: "Test".into(),
            profile: Default::default(),
            fields: Default::default(),
            styles: vec![ob_model::ParagraphStyle {
                id: "body".into(),
                name: "Body".into(),
                font_size: 11_000,
                bold: false,
                italic: false,
                align: ob_model::Align::Left,
                space_after: 4_000,
                color: ob_model::Rgb::BLACK,
                font_family: "Liberation Sans".into(),
                font_sha256: None,
            }],
            stories: Vec::new(),
            panels: Vec::new(),
            sheet: ob_model::SheetSpec {
                preset: "letter-bifold".into(),
                size: ob_model::Size {
                    w: 612_000,
                    h: 792_000,
                },
            },
            no_announcements: false,
        };
        pin_document_fonts(&mut doc, &lib);
        assert_eq!(
            doc.styles[0].font_sha256.as_deref(),
            Some(info.sha256.as_str())
        );
        std::fs::remove_dir_all(&dir).ok();
    }
}
