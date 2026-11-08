//! Reusable snippets: the library-side snippet store
//! plus the keep-time dedupe decision.
//!
//! When an old bulletin is imported, the pieces worth
//! keeping — a picture, a song, a well-written
//! announcement — can stay in the church's own library
//! with their credit attached, instead of living only
//! inside that one bulletin. Nothing is kept unless the
//! admin says so, nothing is ever merged or deleted,
//! and a snippet never phones home: the library lives
//! on the office computer, beside the clip art and the
//! fonts.
//!
//! This crate is the library side only: the store
//! ([`store`]) and the keep decision ([`keep`]). No app
//! UI, no CLI, no picker here. The store holds one
//! [`store::SnippetRecord`] per snippet plus the content
//! bytes, content-addressed by SHA-256; the keep
//! decision consults the store and the fingerprint
//! index, and never mutates either — storing is a
//! separate step, so the caller can ask the admin
//! first.

pub mod keep;
pub mod store;

pub use keep::{decide_keep, dismiss_proposals, recompute_stale, stale_snippets, store_snippet};
pub use keep::{KeepDecision, KeepInput};
pub use store::{
    default_bytes_dir, default_store_path, SnippetFingerprint, SnippetKind, SnippetRecord,
    SnippetStore, STORE_VERSION,
};
