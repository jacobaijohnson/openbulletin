//! The draft vocabulary every importer shares: what a
//! rescue produces for the admin to review (see
//! ob-extract's draft builder and ob-music's draft
//! builder, which both hand back the same shape).
//! These types live in the model crate, beside
//! `hymns` / `markdown` / `rights` / `serving`, so a
//! crate that only needs to speak about a draft's
//! review list does not have to link an import stack
//! to do it (health audit HF-13).

use crate::Document;

/// How sure the rescue is about one piece of the draft, in the
/// words the review screen uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Badge {
    /// Words and a matching (or metric-twin) font.
    Matched,
    /// Words are here, but something needs the admin's eye.
    CheckThis,
    /// No readable words: the page rides along as a picture.
    PictureOnly,
}

/// One line of the review list.
#[derive(Debug, Clone)]
pub struct ImportItem {
    pub panel: usize,
    pub label: String,
    pub badge: Badge,
    pub detail: String,
}

/// What a draft builder hands back: the draft document
/// itself, and the review list that accounts for it.
pub struct DraftOutcome {
    pub doc: Document,
    pub items: Vec<ImportItem>,
}
