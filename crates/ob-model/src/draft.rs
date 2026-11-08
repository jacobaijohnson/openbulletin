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

/// One line of the review list. `keep` is what the
/// line offers to keep in the library, if anything:
/// one tick = one snippet = one paragraph, one
/// picture, or one song. Empty means the line offers
/// no tick. The keep rows are additive, under their
/// review line; the line's label, detail, badge, and
/// order are unchanged by them.
#[derive(Debug, Clone)]
pub struct ImportItem {
    pub panel: usize,
    pub label: String,
    pub badge: Badge,
    pub detail: String,
    pub keep: Vec<Keepable>,
}

/// What one review line offers to keep in the library, if anything.
/// One tick = one snippet = one paragraph, one picture, or one song.
#[derive(Debug, Clone)]
pub struct Keepable {
    pub kind: KeepKind,
    /// The name the picker will show, unless the admin renames it at
    /// keep time. Defaults: text → its first line; picture → the
    /// picture's file name where one exists, else "Picture {n}";
    /// song → its title, else the source file name; page picture →
    /// "Page {n} picture".
    pub default_name: String,
}

#[derive(Debug, Clone)]
pub enum KeepKind {
    /// A picture already in the draft document's asset store.
    Picture { sha: String },
    /// The music file itself, in the draft document's asset store.
    Music { sha: String },
    /// One paragraph of a story in the draft document.
    Text { story_id: String, paragraph: usize },
    /// A picture-only page's tracing render, read from the import
    /// workdir (`assets/page-NN.png`) at keep time.
    PagePicture { page: usize },
}

/// A paragraph's first line, for keep names: the text
/// up to its first line break, trimmed. A paragraph
/// with no words names itself "Untitled paragraph".
pub fn first_line(text: &str) -> String {
    let first = text.lines().next().unwrap_or("").trim();
    if first.is_empty() {
        "Untitled paragraph".to_string()
    } else {
        first.to_string()
    }
}

/// What a draft builder hands back: the draft document
/// itself, and the review list that accounts for it.
pub struct DraftOutcome {
    pub doc: Document,
    pub items: Vec<ImportItem>,
}
