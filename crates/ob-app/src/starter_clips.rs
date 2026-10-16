//! The bundled starter set of clip art (docs/research/
//! clipart-landscape.md section 3): single-color line and
//! silhouette SVGs for the church year, every one verified
//! CC0 at its source page, with the manifest that records
//! the verification shipped beside the files and read by
//! the app when the set installs into the Clipart Library.
//!
//! The files live in `assets/clipart/` next to this crate's
//! manifest, the same home pattern as the bundled fonts in
//! ob-text. Adding a piece means: verify it per the
//! landscape's checklist, add the file, add its manifest
//! line, and add one row to STARTER_FILES below.

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
/// with the files; the app installs pieces from
/// STARTER_FILES and the manifest rides along as data.
#[allow(dead_code)]
pub static STARTER_MANIFEST: &str = include_str!("../assets/clipart/manifest.json");
