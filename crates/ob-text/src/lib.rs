//! Text: real shaping, layout, and threading for OpenBulletin.
//!
//! The engine is [`shaping::Shaper`], built on parley: real font
//! advances, real line breaks, real overflow, on registered fonts
//! only. [`text_layout`] turns shaped paragraphs into the display
//! lines the canvas and the PDF writer work from, and
//! [`threading`] flows stories through their frames and reports
//! what does not fit. Anything that decides whether text fits a
//! frame comes through here, never through an estimate.

pub mod shaping;
pub mod text_layout;
pub mod threading;

/// The path of one bundled font asset that ships in
/// this crate (a plain file name such as
/// "LiberationSans-Regular.ttf"), anchored at this
/// crate's own manifest folder (`assets/fonts/` under
/// it). This is the supported way for workspace tests
/// and tools in sibling crates to reach the bundled
/// test fonts (health audit m-8): callers go through
/// ob-text, the fonts' owner, instead of reaching into
/// its asset folder by a relative filesystem path that
/// only works while the workspace layout never moves.
pub fn bundled_asset_path(file_name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join("fonts")
        .join(file_name)
}
