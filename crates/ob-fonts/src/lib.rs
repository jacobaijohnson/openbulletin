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
        let info = inspect(bytes, suggested_name)?;
        for existing in self.scan() {
            if existing.sha256 == info.sha256 {
                return Ok(existing);
            }
        }
        let target = self.dir.join(suggested_name);
        std::fs::write(&target, bytes)
            .map_err(|e| format!("cannot save font into the library: {e}"))?;
        Ok(info)
    }

    /// The bytes of one library font, for handing to the text engine.
    pub fn read(&self, info: &FontInfo) -> Result<Vec<u8>, String> {
        std::fs::read(self.dir.join(&info.file_name))
            .map_err(|e| format!("cannot read font {}: {e}", info.file_name))
    }
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
        };
        pin_document_fonts(&mut doc, &lib);
        assert_eq!(
            doc.styles[0].font_sha256.as_deref(),
            Some(info.sha256.as_str())
        );
        std::fs::remove_dir_all(&dir).ok();
    }
}
