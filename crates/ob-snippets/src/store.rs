//! The snippet store: one record per kept snippet.
//!
//! The store lives library-side, beside the clip art
//! and the fonts, in the same `<base>/OpenBulletin/`
//! home as `fingerprints.json` (see
//! [`default_store_path`] and [`default_bytes_dir`]).
//! Two files make the store:
//!
//! * `snippets.json` — `{ "version": "snippets-v1",
//!   "snippets": [...] }`, one [`SnippetRecord`] per
//!   snippet. The record carries the fingerprint and
//!   the rights; the content bytes live beside it.
//! * `snippets/` — one file per snippet, named by its
//!   content SHA-256 hex (no extension). The store
//!   HOLDS the original, so a stale fingerprint can
//!   always be recomputed from the stored bytes.
//!
//! Load rule, mirroring `FingerprintIndex::load`: a
//! missing file is an empty store, never an error; a
//! damaged file is a loud error, never a silent empty
//! store (a silent empty would drop kept snippets).

use ob_fingerprint::matching::{ImageEntry, MusicEntry, TextEntry};
use ob_model::Asset;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// The store file's own version. It bundles the
/// [`SnippetRecord`] shape below.
pub const STORE_VERSION: &str = "snippets-v1";

/// What kind of snippet this is. The asset kind is
/// derived from it: Picture → `AssetKind::Picture`,
/// Music → `AssetKind::Music`, Text →
/// `AssetKind::Other` with media type `text/plain`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SnippetKind {
    Picture,
    Music,
    Text,
}

/// One snippet's fingerprint, reusing the fingerprint
/// index's per-kind entries. The enum is serde-tagged
/// on `kind`, so the JSON names the kind beside the
/// entry's own fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SnippetFingerprint {
    Picture(ImageEntry),
    Music(MusicEntry),
    Text(TextEntry),
}

impl SnippetFingerprint {
    /// The `fp_version` the entry was stamped with.
    pub fn fp_version(&self) -> &str {
        match self {
            SnippetFingerprint::Picture(entry) => &entry.fp_version,
            SnippetFingerprint::Music(entry) => &entry.fp_version,
            SnippetFingerprint::Text(entry) => &entry.fp_version,
        }
    }

    /// The content SHA-256 hex the entry is filed under.
    pub fn sha256(&self) -> &str {
        match self {
            SnippetFingerprint::Picture(entry) => &entry.sha256,
            SnippetFingerprint::Music(entry) => &entry.sha256,
            SnippetFingerprint::Text(entry) => &entry.sha256,
        }
    }
}

/// One kept snippet. Every field round-trips through
/// `snippets.json`, including the FULL rights record
/// (see [`ob_model::RightsRecord`]: `child_web_permission`
/// and `licensor_ref` included) and the text-only
/// `spoken_only` / `web_hidden` flags, so reuse
/// reproduces the same export behaviour the snippet
/// had when it was kept.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnippetRecord {
    /// SHA-256 hex of the content bytes (Layer-0
    /// identity). The bytes file is named by this id.
    pub id: String,
    pub kind: SnippetKind,
    /// The plain name the office recognises.
    pub name: String,
    /// The imported file name it was kept from.
    pub source: String,
    /// ISO date `YYYY-MM-DD`, supplied by the caller.
    pub kept_at: String,
    /// Who kept it; empty when the caller does not know
    /// (matches import-assigned records, which leave it
    /// default).
    #[serde(default)]
    pub asserted_by: String,
    /// `{ sha256, kind, media_type, rights }` with the
    /// FULL rights record.
    pub asset: Asset,
    /// The per-kind fingerprint entry.
    pub fingerprint: SnippetFingerprint,
    /// Text only in practice; default false, skipped
    /// when false. A kept paragraph keeps the flags it
    /// had, so reuse reproduces the same behaviour.
    #[serde(default, skip_serializing_if = "is_false")]
    pub spoken_only: bool,
    /// Text only in practice; default false, skipped
    /// when false.
    #[serde(default, skip_serializing_if = "is_false")]
    pub web_hidden: bool,
}

fn is_false(value: &bool) -> bool {
    !*value
}

/// The library-side snippet store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnippetStore {
    pub version: String,
    #[serde(default)]
    pub snippets: Vec<SnippetRecord>,
}

impl Default for SnippetStore {
    fn default() -> Self {
        SnippetStore {
            version: STORE_VERSION.to_string(),
            snippets: Vec::new(),
        }
    }
}

impl SnippetStore {
    /// Load the store. A missing file is a fresh
    /// library, not an error. An unreadable file IS an
    /// error: silently starting empty would drop kept
    /// snippets.
    pub fn load(path: &Path) -> Result<SnippetStore, String> {
        match std::fs::read_to_string(path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(SnippetStore::default()),
            Err(e) => Err(format!("could not read the snippet store: {e}")),
            Ok(text) => serde_json::from_str(&text)
                .map_err(|e| format!("the snippet store is damaged: {e}")),
        }
    }

    /// Save the store as pretty JSON, creating parent
    /// directories.
    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("could not create the snippet store folder: {e}"))?;
        }
        let text = serde_json::to_string_pretty(self)
            .map_err(|e| format!("could not write the snippet store: {e}"))?;
        std::fs::write(path, text).map_err(|e| format!("could not write the snippet store: {e}"))
    }

    /// The record for a content SHA-256 hex, if kept.
    pub fn get(&self, id: &str) -> Option<&SnippetRecord> {
        self.snippets.iter().find(|record| record.id == id)
    }

    /// Whether the store already holds this content SHA.
    pub fn contains(&self, id: &str) -> bool {
        self.get(id).is_some()
    }
}

/// The default store file:
/// `<base>/OpenBulletin/snippets.json`, where `<base>`
/// is the parent of `ob_fonts::default_library_dir()`
/// — exactly as `ob_clips::default_library_dir()` is
/// computed.
pub fn default_store_path() -> PathBuf {
    let fonts = ob_fonts::default_library_dir();
    match fonts.parent() {
        Some(base) => base.join("snippets.json"),
        None => PathBuf::from("snippets.json"),
    }
}

/// The default content-bytes folder:
/// `<base>/OpenBulletin/snippets/`, beside the store
/// file.
pub fn default_bytes_dir() -> PathBuf {
    let fonts = ob_fonts::default_library_dir();
    match fonts.parent() {
        Some(base) => base.join("snippets"),
        None => PathBuf::from("snippets"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ob_model::{AssetKind, LicenseKind, RightsRecord};

    fn rights_with_children(permission: Option<bool>) -> RightsRecord {
        RightsRecord {
            source: "old-bulletin.pdf".to_string(),
            license: LicenseKind::CcBy("4.0".to_string()),
            title: "Cross".to_string(),
            creator: "Jane Doe".to_string(),
            copyright_line: "© 2019 Jane Doe".to_string(),
            credit_text: "Cross by Jane Doe".to_string(),
            licensor_ref: "CCLI #123456".to_string(),
            asserted_by: "office".to_string(),
            asserted_at: "2026-11-07".to_string(),
            children_present: Some(true),
            child_web_permission: permission,
            ..RightsRecord::default()
        }
    }

    fn picture_record() -> SnippetRecord {
        SnippetRecord {
            id: "aa".repeat(32),
            kind: SnippetKind::Picture,
            name: "cross.png".to_string(),
            source: "old-bulletin.pdf".to_string(),
            kept_at: "2026-11-07".to_string(),
            asserted_by: "office".to_string(),
            asset: Asset {
                sha256: "aa".repeat(32),
                kind: AssetKind::Picture,
                media_type: "image/png".to_string(),
                rights: rights_with_children(Some(false)),
            },
            fingerprint: SnippetFingerprint::Picture(ImageEntry {
                sha256: "aa".repeat(32),
                name: "cross.png".to_string(),
                fp_version: ob_fingerprint::IMAGE_FP_VERSION.to_string(),
                phash: 0,
                dhash: 0,
                hist: vec![0u32; 64],
                low_information: false,
            }),
            spoken_only: false,
            web_hidden: false,
        }
    }

    fn text_record() -> SnippetRecord {
        SnippetRecord {
            id: "bb".repeat(32),
            kind: SnippetKind::Text,
            name: "Welcome".to_string(),
            source: "old-bulletin.pdf".to_string(),
            kept_at: "2026-11-07".to_string(),
            asserted_by: String::new(),
            asset: Asset {
                sha256: "bb".repeat(32),
                kind: AssetKind::Other,
                media_type: "text/plain".to_string(),
                rights: rights_with_children(Some(true)),
            },
            fingerprint: SnippetFingerprint::Text(TextEntry {
                sha256: "bb".repeat(32),
                name: "Welcome".to_string(),
                fp_version: ob_fingerprint::TEXT_FP_VERSION.to_string(),
                norm_hash: "cc".repeat(32),
                shingles: vec![1, 2, 3],
                winnow: vec![(1, 0)],
                simhash: 7,
                token_count: 20,
            }),
            spoken_only: true,
            web_hidden: true,
        }
    }

    #[test]
    fn round_trip_preserves_every_field() {
        let dir = std::env::temp_dir().join("ob-snippets-store-test");
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("snippets.json");
        let mut store = SnippetStore::default();
        assert_eq!(store.version, STORE_VERSION);
        store.snippets.push(picture_record());
        store.snippets.push(text_record());
        let before = serde_json::to_string_pretty(&store).expect("serialise");
        store.save(&path).expect("save works");
        let loaded = SnippetStore::load(&path).expect("load works");
        let after = serde_json::to_string_pretty(&loaded).expect("serialise");
        assert_eq!(before, after, "save/load is byte-identical JSON");
        let picture = &loaded.snippets[0];
        assert_eq!(picture.asset.rights.child_web_permission, Some(false));
        assert_eq!(picture.asset.rights.licensor_ref, "CCLI #123456");
        assert_eq!(picture.asset.rights.children_present, Some(true));
        assert_eq!(
            picture.asset.rights.license,
            LicenseKind::CcBy("4.0".to_string())
        );
        assert!(!picture.spoken_only);
        assert!(!picture.web_hidden);
        let text = &loaded.snippets[1];
        assert_eq!(text.asset.rights.child_web_permission, Some(true));
        assert_eq!(text.asset.rights.licensor_ref, "CCLI #123456");
        assert!(text.spoken_only);
        assert!(text.web_hidden);
        assert_eq!(text.asserted_by, "");
        // The false flags are skipped in the JSON, the
        // true flags are present.
        assert!(!before.contains("spoken_only\": false"));
        assert!(before.contains("spoken_only\": true"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_file_is_empty_and_damaged_is_loud() {
        let dir = std::env::temp_dir().join("ob-snippets-missing-test");
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("snippets.json");
        let fresh = SnippetStore::load(&path).expect("missing is empty");
        assert!(fresh.snippets.is_empty());
        assert_eq!(fresh.version, STORE_VERSION);
        std::fs::create_dir_all(&dir).expect("mkdir");
        std::fs::write(&path, "{not json").expect("write damaged");
        let err = SnippetStore::load(&path).expect_err("damaged is loud");
        assert!(err.contains("damaged"), "error is {err}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn path_helpers_end_in_library_home() {
        let store_path = default_store_path();
        let tail = PathBuf::from("OpenBulletin").join("snippets.json");
        assert!(store_path.ends_with(tail), "path is {store_path:?}");
        let bytes_dir = default_bytes_dir();
        let tail = PathBuf::from("OpenBulletin").join("snippets");
        assert!(bytes_dir.ends_with(tail), "path is {bytes_dir:?}");
    }
}
