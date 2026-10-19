//! The bundled starter set of clip art (docs/research/
//! clipart-landscape.md section 3): single-color line and
//! silhouette SVGs for the church year, every one verified
//! at its source page, with the manifest that records the
//! verification shipped beside the files. When the set
//! installs into the Clipart Library, the app parses the
//! manifest and records each piece with its own row's
//! facts (source page, author, license, and the manifest's
//! retrieval date as the added date), so the mixed
//! public-domain / CC0 set keeps its per-piece provenance
//! in the library instead of flattening into one note.
//!
//! The files live in `assets/clipart/` next to this crate's
//! manifest, the same home pattern as the bundled fonts in
//! ob-text. Adding a piece means: verify it per the
//! landscape's checklist, add the file, add its manifest
//! line, and add one row to STARTER_FILES below. A test
//! below pins the pairing: a piece without exactly one
//! manifest row fails the suite, so a new piece cannot
//! slip in without its provenance.

/// (file name, contents) for every starter piece. The
/// manifest beside the files is the per-piece record
/// (source page, author, license, our edits).
pub static STARTER_FILES: &[(&str, &str)] = &[
    ("cross.svg", include_str!("../assets/clipart/cross.svg")),
    ("dove.svg", include_str!("../assets/clipart/dove.svg")),
    ("candle.svg", include_str!("../assets/clipart/candle.svg")),
    ("star.svg", include_str!("../assets/clipart/star.svg")),
    ("fish.svg", include_str!("../assets/clipart/fish.svg")),
    ("chalice.svg", include_str!("../assets/clipart/chalice.svg")),
    ("church.svg", include_str!("../assets/clipart/church.svg")),
    ("angel.svg", include_str!("../assets/clipart/angel.svg")),
    ("anchor.svg", include_str!("../assets/clipart/anchor.svg")),
    ("crown.svg", include_str!("../assets/clipart/crown.svg")),
    (
        "butterfly.svg",
        include_str!("../assets/clipart/butterfly.svg"),
    ),
    (
        "open-book.svg",
        include_str!("../assets/clipart/open-book.svg"),
    ),
];

/// The shipped manifest: one record per piece (file,
/// title, source page, author, license, retrieval date,
/// our edits). It is the set's provenance record, kept
/// with the files; at install it is parsed and each
/// piece's row becomes that piece's record in the
/// library (see `install_into`).
pub static STARTER_MANIFEST: &str = include_str!("../assets/clipart/manifest.json");

/// One piece's row in the shipped manifest.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct ManifestPiece {
    pub file: String,
    pub author: String,
    pub license: String,
    pub source_url: String,
}

/// The shipped manifest, parsed: the retrieval date the
/// whole set shares, and one row per piece.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct StarterManifest {
    pub retrieved: String,
    pub items: Vec<ManifestPiece>,
}

impl StarterManifest {
    /// The row for one piece, by file name.
    pub fn row(&self, file: &str) -> Option<&ManifestPiece> {
        self.items.iter().find(|p| p.file == file)
    }
}

/// Parse the shipped manifest. A manifest that does not
/// parse yields None, and the install then records the
/// set with the manifest named as its source in words
/// rather than per-piece rows (the tests below pin the
/// shipped manifest's shape, so this is a fallback, not
/// the expected path).
pub fn starter_manifest() -> Option<StarterManifest> {
    serde_json::from_str(STARTER_MANIFEST).ok()
}

/// The collection-level record for the starter set. The
/// license stays unstated here on purpose: the set mixes
/// public-domain and CC0 pieces, and each piece's own
/// record (below) states its license; one collection-wide
/// word would flatten that.
pub fn collection_notes(manifest: Option<&StarterManifest>) -> ob_clips::CollectionNotes {
    let added = manifest.map(|m| m.retrieved.clone()).unwrap_or_default();
    ob_clips::CollectionNotes {
        source: "OpenBulletin starter set (bundled with the program). Each piece's own \
                 record — its source page, author, and license — is read from the \
                 manifest shipped with the program and kept with the piece."
            .into(),
        license: String::new(),
        added,
        creator: String::new(),
    }
}

/// One piece's library record, from its manifest row: its
/// own license (a public-domain piece is recorded as
/// public domain, never as CC0), its author, its source
/// page, and the manifest's retrieval date as the added
/// date. A piece whose row is somehow missing still gets
/// a record, with the manifest named as its source in
/// words; the pin in the tests is what guards the shipped
/// set against ever taking this path.
pub fn piece_notes(manifest: Option<&StarterManifest>, file: &str) -> ob_clips::CollectionNotes {
    let added = manifest.map(|m| m.retrieved.clone()).unwrap_or_default();
    match manifest.and_then(|m| m.row(file)) {
        Some(row) => ob_clips::CollectionNotes {
            source: row.source_url.clone(),
            license: row.license.clone(),
            added,
            creator: row.author.clone(),
        },
        None => ob_clips::CollectionNotes {
            source: "The starter set manifest shipped with OpenBulletin".into(),
            license: String::new(),
            added,
            creator: String::new(),
        },
    }
}

/// Install the starter set into `lib` as the "Starter set"
/// collection, each piece carrying its manifest row as its
/// library record.
pub fn install_into(lib: &ob_clips::ClipLibrary) -> Result<ob_clips::AddReport, String> {
    let manifest = starter_manifest();
    let files: Vec<(String, Vec<u8>)> = STARTER_FILES
        .iter()
        .map(|(name, text)| (name.to_string(), text.as_bytes().to_vec()))
        .collect();
    let mut pieces = std::collections::BTreeMap::new();
    for (name, _) in STARTER_FILES {
        pieces.insert(name.to_string(), piece_notes(manifest.as_ref(), name));
    }
    lib.add_collection_with_piece_notes(
        "Starter set",
        &files,
        &collection_notes(manifest.as_ref()),
        &pieces,
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_library(tag: &str) -> (std::path::PathBuf, ob_clips::ClipLibrary) {
        let dir = std::env::temp_dir().join(format!(
            "ob-app-starter-test-{}-{}",
            std::process::id(),
            tag
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let lib = ob_clips::ClipLibrary::open(dir.clone()).expect("open library");
        (dir, lib)
    }

    #[test]
    fn every_starter_file_has_exactly_one_manifest_row() {
        let manifest = starter_manifest().expect("the shipped manifest parses");
        assert_eq!(
            manifest.items.len(),
            STARTER_FILES.len(),
            "one manifest row per starter piece, no more, no fewer"
        );
        for (name, _) in STARTER_FILES {
            let rows = manifest.items.iter().filter(|p| p.file == *name).count();
            assert_eq!(rows, 1, "{name} has exactly one manifest row");
        }
        assert!(
            !manifest.retrieved.is_empty(),
            "the manifest states its retrieval date"
        );
    }

    #[test]
    fn starter_set_installs_all_pieces_with_their_own_records() {
        let (dir, lib) = temp_library("install");
        let manifest = starter_manifest().expect("the shipped manifest parses");
        let report = install_into(&lib).expect("install");
        assert_eq!(report.added, STARTER_FILES.len());
        let set = lib.collection("Starter set").expect("the starter set");
        assert_eq!(set.items.len(), STARTER_FILES.len());
        // The collection's added date is the manifest's
        // retrieval date.
        assert_eq!(
            set.notes.as_ref().map(|n| n.added.as_str()),
            Some(manifest.retrieved.as_str())
        );
        // A named public-domain piece and a named CC0
        // piece, each checked against the values its own
        // manifest row states.
        for name in ["cross.svg", "candle.svg"] {
            let row = manifest.row(name).expect("row");
            let record = set.piece_notes(name).expect("the piece's record");
            assert_eq!(record.license, row.license, "{name} license");
            assert_eq!(record.creator, row.author, "{name} author");
            assert_eq!(record.source, row.source_url, "{name} source page");
            assert_eq!(record.added, manifest.retrieved, "{name} added date");
        }
        // The cross's row is a public-domain row, and its
        // record parses back to the model's Public Domain,
        // not CC0; the candle's row is CC0 and parses to Cc0.
        let cross_row = manifest.row("cross.svg").expect("row");
        assert_eq!(cross_row.license, "Public domain");
        assert_eq!(
            set.piece_notes("cross.svg").map(|n| n.license_kind()),
            Some(ob_model::LicenseKind::PublicDomain)
        );
        let candle_row = manifest.row("candle.svg").expect("row");
        assert_eq!(candle_row.license, "CC0");
        assert_eq!(
            set.piece_notes("candle.svg").map(|n| n.license_kind()),
            Some(ob_model::LicenseKind::Cc0)
        );
        // Every installed piece carries a record of its
        // own, none left on the collection fallback alone.
        for item in &set.items {
            assert!(
                set.pieces.contains_key(&item.file_name),
                "{} carries its own record",
                item.file_name
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_piece_without_a_manifest_row_still_gets_a_record() {
        let manifest = starter_manifest().expect("the shipped manifest parses");
        let record = piece_notes(Some(&manifest), "no-such-piece.svg");
        assert!(record.source.contains("manifest"), "{record:?}");
        assert_eq!(record.added, manifest.retrieved);
        assert!(record.license.is_empty());
    }
}
