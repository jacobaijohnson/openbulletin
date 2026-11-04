//! Content fingerprints for OpenBulletin's library index.
//!
//! Three worded tiers, and nothing else, ever reach the user:
//!
//! * "Same file" — the SHA-256 of the file bytes is equal (Layer 0,
//!   already the app's identity; this crate does not own it).
//! * "Same content, different file" — a canonical-content hash is
//!   equal (Layer 1). Exact, no thresholds, no false positives
//!   beyond a canonicaliser bug.
//! * "Probably the same — please check" — a similarity distance
//!   under a stated band (Layer 2). Always a proposal that asks
//!   the user. Nothing is ever silently merged or deleted, at
//!   any layer, and a Layer-2 score is never shown as a bare
//!   percentage: it is a distance with a concrete reason.
//!
//! Fingerprints are an INDEX, never the record: the file itself
//! learns nothing about where or how often it is used. Every
//! fingerprint is stamped with an `fp_version` that bundles the
//! algorithm and every parameter; only equal versions are ever
//! compared, and a version bump recomputes from the originals.
//! Everything here is computed offline, at import, and persisted;
//! nothing is recomputed except on an `fp_version` change, and
//! nothing here touches the network.
//!
//! This module owns the image half. Music lives in `music`
//! (canonicaliser in ob-music, matching here); text lives in
//! `text`. The tiers, the band table, the index file, and the
//! stored "Not duplicates" dismissals live in `matching`.

pub mod image;

/// The image fingerprint version. It bundles every choice in
/// [`image::fingerprint_image`]: Rec.601 integer grayscale,
/// alpha composited over white, box-average downscale,
/// gradient bits for dHash, orthonormal DCT-II with
/// 6-decimal rounding for pHash, and the 4x4x4 colour
/// histogram. Change any one of those and this string
/// changes, because fingerprints from different parameters
/// are incomparable.
pub const IMAGE_FP_VERSION: &str = "img-fp-v1";
