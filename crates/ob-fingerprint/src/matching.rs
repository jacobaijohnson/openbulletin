//! The tiers, the band table, the library index, and
//! the stored "Not duplicates" dismissals.
//!
//! The three worded tiers, and nothing else, ever reach
//! the user:
//!
//! * "Same file" — the SHA-256 of the bytes is equal.
//! * "Same content, different file" — a canonical hash
//!   is equal. This tier may group silently, because it
//!   is exact. It never deletes anything, at any tier.
//! * "Probably the same — please check" — a similarity
//!   band was reached. Always a proposal that asks the
//!   user. Never a bare percentage: every proposal
//!   carries a concrete reason (how many picture
//!   details differ, which word-groups are shared,
//!   whether the tune is in another key).
//!
//! Music has two extra honest labels inside the asking
//! tier, per the Tier-2 amendments: "Same notes,
//! different spelling" (MIDI-pitch equality, spelling
//! differs) and "Same melody, different harmony"
//! (melody equality, harmony differs). Only the full
//! spelling-sensitive canonical hash earns the silent
//! tier.
//!
//! The index lives in a library-side JSON file, beside
//! the Font Library, not in `.obd` documents: the file
//! itself learns nothing about where or how often it is
//! used, and no `.obd` schema change is needed. The
//! conventional home is `<base>/OpenBulletin/
//! fingerprints.json`; the app owns the path and hands
//! it to [`FingerprintIndex::load`] and `save`.
//!
//! A stale entry (its `fp_version` no longer current)
//! never matches anything: it is reported by
//! [`FingerprintIndex::stale_entries`] so the app can
//! recompute it from the original at import time.
//!
//! Short texts need a second step: the index stores
//! hashes and sets, never the words (an index is not
//! the record), so a short-text comparison needs the
//! source texts. [`FingerprintIndex::find_text_matches`]
//! returns those candidates separately in
//! `needs_texts`; the caller fetches the two source
//! texts, tokenises them, and calls
//! [`match_short_texts`].

use crate::image::{hamming, image_band, ImageBand, ImageFingerprint};
use crate::music::{music_decision, MusicCandidateIndex, MusicDecision, MusicFingerprint};
use crate::text::{short_text_scores, text_band, text_scores, TextDecision, TextFingerprint};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// The index file's own version. It bundles the entry
/// shapes below; the per-type `fp_version` strings
/// still gate every comparison independently.
pub const INDEX_VERSION: &str = "fp-index-v1";

/// Proposals for a text query: (match, entry name).
pub type TextProposals = Vec<(Match, String)>;

/// Short-text second-step candidates: (entry sha,
/// entry name) pairs whose source texts the caller
/// must fetch.
pub type ShortCandidates = Vec<(String, String)>;

/// Which tier a proposal sits in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchKind {
    /// Byte-identical. May group silently.
    SameFile,
    /// Canonical hash equal. May group silently.
    SameContent,
    /// Similarity or spelling/melody label. Asks.
    NeedsCheck,
}

/// One proposal to the user: a tier, a plain-words
/// title, and a concrete reason. Never a bare
/// percentage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Match {
    pub kind: MatchKind,
    pub title: String,
    pub reason: String,
}

impl Match {
    /// Only the exact tiers may group without asking.
    /// Nothing is ever deleted, at any tier.
    pub fn may_group_silently(&self) -> bool {
        matches!(self.kind, MatchKind::SameFile | MatchKind::SameContent)
    }
}

const PROBABLY_TITLE: &str = "Probably the same — please check";

/// Match two pictures. `sha_a`/`sha_b` are the files'
/// SHA-256 hex; equal bytes are Layer 0 regardless of
/// fingerprint version. Bands need both hashes to
/// agree and the colour guard to pass (enforced inside
/// `image_band`).
pub fn match_image(
    a: &ImageFingerprint,
    b: &ImageFingerprint,
    sha_a: &str,
    sha_b: &str,
) -> Option<Match> {
    if sha_a == sha_b {
        return Some(Match {
            kind: MatchKind::SameFile,
            title: "Same file".to_string(),
            reason: "the file bytes are identical".to_string(),
        });
    }
    let ph = hamming(a.phash, b.phash);
    let dh = hamming(a.dhash, b.dhash);
    if image_band(a, b) == ImageBand::NoMatch {
        return None;
    }
    let mut reason = format!(
        "the picture hash differs in {ph} of 64 details and the gradient hash in {dh} of 64"
    );
    if a.low_information || b.low_information {
        reason.push_str(
            "; both pictures are simple drawings on a plain background, so look before you merge",
        );
    }
    Some(Match {
        kind: MatchKind::NeedsCheck,
        title: PROBABLY_TITLE.to_string(),
        reason,
    })
}

/// Match two tunes. Only the spelling-sensitive
/// canonical hash earns the silent tier; MIDI-pitch
/// equality and melody equality are asking labels.
pub fn match_music(a: &MusicFingerprint, b: &MusicFingerprint) -> Option<Match> {
    match music_decision(a, b) {
        MusicDecision::NoMatch => None,
        MusicDecision::SameContent => Some(Match {
            kind: MatchKind::SameContent,
            title: "Same content, different file".to_string(),
            reason: "the same notes, spelled the same way, in a different file".to_string(),
        }),
        MusicDecision::SameNotesDifferentSpelling => Some(Match {
            kind: MatchKind::NeedsCheck,
            title: "Same notes, different spelling — please check".to_string(),
            reason:
                "the same notes with different spelling (an enharmonic respelling, for example)"
                    .to_string(),
        }),
        MusicDecision::ProbablyMelody => Some(Match {
            kind: MatchKind::NeedsCheck,
            title: "Same melody, different harmony — please check".to_string(),
            reason: "the same melody with different harmony around it".to_string(),
        }),
        MusicDecision::ProbablyBand => {
            let reason = if a.intervals == b.intervals && a.rhythm == b.rhythm {
                "the same tune in another key".to_string()
            } else if a.intervals == b.intervals {
                "the same tune shape with a different rhythm".to_string()
            } else {
                "the melody and rhythm are both close to the same tune, and the opening notes are shared"
                    .to_string()
            };
            Some(Match {
                kind: MatchKind::NeedsCheck,
                title: PROBABLY_TITLE.to_string(),
                reason,
            })
        }
    }
}

/// Match two long texts (both at least 15 tokens).
/// Returns `None` for NoMatch AND for short texts: the
/// caller routes short texts through
/// [`match_short_texts`] with the source texts in hand.
/// `df` is the library document-frequency map; when
/// supplied, boilerplate shingles are excluded and the
/// reason says so.
pub fn match_text(
    a: &TextFingerprint,
    b: &TextFingerprint,
    df: Option<(&HashMap<u64, usize>, usize)>,
) -> Option<Match> {
    match text_band(a, b, df) {
        TextDecision::NoMatch | TextDecision::NeedsShortTexts => None,
        TextDecision::SameContent => Some(Match {
            kind: MatchKind::SameContent,
            title: "Same content, different file".to_string(),
            reason: "the same words, retyped with different spacing, capitals, or punctuation"
                .to_string(),
        }),
        TextDecision::VeryLikely | TextDecision::Possibly => {
            let s = text_scores(a, b, df);
            let mut reason = format!(
                "shares {} of {} word-groups with the other announcement",
                s.shared, s.union
            );
            if s.containment >= 0.85 {
                reason.push_str("; the shorter announcement is contained in the longer one");
            }
            if df.is_some() {
                reason.push_str("; standard phrases shared by many announcements are not counted");
            }
            Some(Match {
                kind: MatchKind::NeedsCheck,
                title: PROBABLY_TITLE.to_string(),
                reason,
            })
        }
    }
}

/// Match two short texts from their tokens. The diff is
/// always part of the reason: a changed name, date, or
/// one "not" changes the meaning, not just the score.
///
/// Short texts use a containment rule, not the
/// long-text Jaccard bands: a single changed word in a
/// five-word prayer line is a Jaccard of 0.67, below
/// the long-text possibly band, but it is exactly the
/// pair the user needs to see. So a short pair proposes
/// when at least 80% of the shorter text's words are
/// shared, and the diff is shown so the user judges in
/// one glance. A dismissed pair is never re-proposed,
/// so the cost of a liberal short band is one glance,
/// not repeated noise.
pub fn match_short_texts(a: &[String], b: &[String]) -> Option<Match> {
    let min_len = a.len().min(b.len());
    if min_len == 0 {
        return None;
    }
    let (_j, only_a, only_b) = short_text_scores(a, b);
    // Multiset accounting: only_a holds exactly the
    // a-tokens unmatched in b, so shared = a.len() -
    // only_a.len().
    let shared = a.len() - only_a.len();
    if (shared as f64) / (min_len as f64) < 0.80 {
        return None;
    }
    let mut reason = format!(
        "the words differ only here — only in the first: {}; only in the second: {}",
        only_a.join(", "),
        only_b.join(", ")
    );
    if only_a.is_empty() && only_b.is_empty() {
        reason = "the words are the same".to_string();
    }
    Some(Match {
        kind: MatchKind::NeedsCheck,
        title: PROBABLY_TITLE.to_string(),
        reason,
    })
}

/// One image entry in the index. Owned strings, so the
/// file round-trips; `to_engine` refuses stale versions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageEntry {
    pub sha256: String,
    pub name: String,
    pub fp_version: String,
    pub phash: u64,
    pub dhash: u64,
    /// 64 bins, index `(r/64)*16 + (g/64)*4 + (b/64)`.
    /// A Vec, not an array, because serde's array impls
    /// stop at 32; `to_engine` refuses any other length.
    pub hist: Vec<u32>,
    pub low_information: bool,
}

impl ImageEntry {
    pub fn from_engine(sha256: &str, name: &str, fp: &ImageFingerprint) -> ImageEntry {
        ImageEntry {
            sha256: sha256.to_string(),
            name: name.to_string(),
            fp_version: fp.fp_version.to_string(),
            phash: fp.phash,
            dhash: fp.dhash,
            hist: fp.hist.to_vec(),
            low_information: fp.low_information,
        }
    }

    pub fn to_engine(&self) -> Option<ImageFingerprint> {
        if self.fp_version != crate::IMAGE_FP_VERSION || self.hist.len() != 64 {
            return None;
        }
        let mut hist = [0u32; 64];
        hist.copy_from_slice(&self.hist);
        Some(ImageFingerprint {
            fp_version: crate::IMAGE_FP_VERSION,
            phash: self.phash,
            dhash: self.dhash,
            hist,
            width: 0,
            height: 0,
            low_information: self.low_information,
        })
    }
}

/// One music entry in the index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusicEntry {
    pub sha256: String,
    pub name: String,
    pub fp_version: String,
    pub canon_hash: String,
    pub midi_hash: String,
    pub melody_hash: String,
    pub intervals: Vec<i16>,
    pub rhythm: Vec<(u32, u32, bool)>,
    pub incipit: String,
    pub pae: String,
}

impl MusicEntry {
    pub fn from_engine(sha256: &str, name: &str, fp: &MusicFingerprint) -> MusicEntry {
        MusicEntry {
            sha256: sha256.to_string(),
            name: name.to_string(),
            fp_version: fp.fp_version.to_string(),
            canon_hash: fp.canon_hash.clone(),
            midi_hash: fp.midi_hash.clone(),
            melody_hash: fp.melody_hash.clone(),
            intervals: fp.intervals.clone(),
            rhythm: fp.rhythm.clone(),
            incipit: fp.incipit.clone(),
            pae: fp.pae.clone(),
        }
    }

    pub fn to_engine(&self) -> Option<MusicFingerprint> {
        if self.fp_version != crate::MUSIC_FP_VERSION {
            return None;
        }
        Some(MusicFingerprint {
            fp_version: crate::MUSIC_FP_VERSION,
            canon_hash: self.canon_hash.clone(),
            midi_hash: self.midi_hash.clone(),
            melody_hash: self.melody_hash.clone(),
            intervals: self.intervals.clone(),
            rhythm: self.rhythm.clone(),
            incipit: self.incipit.clone(),
            pae: self.pae.clone(),
        })
    }
}

/// One text entry in the index. The words themselves
/// are never stored: an index is not the record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextEntry {
    pub sha256: String,
    pub name: String,
    pub fp_version: String,
    pub norm_hash: String,
    pub shingles: Vec<u64>,
    pub winnow: Vec<(u64, usize)>,
    pub simhash: u64,
    pub token_count: usize,
}

impl TextEntry {
    pub fn from_engine(sha256: &str, name: &str, fp: &TextFingerprint) -> TextEntry {
        TextEntry {
            sha256: sha256.to_string(),
            name: name.to_string(),
            fp_version: fp.fp_version.to_string(),
            norm_hash: fp.norm_hash.clone(),
            shingles: fp.shingles.clone(),
            winnow: fp.winnow.clone(),
            simhash: fp.simhash,
            token_count: fp.token_count,
        }
    }

    pub fn to_engine(&self) -> Option<TextFingerprint> {
        if self.fp_version != crate::TEXT_FP_VERSION {
            return None;
        }
        Some(TextFingerprint {
            fp_version: crate::TEXT_FP_VERSION,
            norm_hash: self.norm_hash.clone(),
            shingles: self.shingles.clone(),
            winnow: self.winnow.clone(),
            simhash: self.simhash,
            token_count: self.token_count,
        })
    }
}

/// The library-side fingerprint index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FingerprintIndex {
    pub version: String,
    pub images: Vec<ImageEntry>,
    pub music: Vec<MusicEntry>,
    pub texts: Vec<TextEntry>,
    /// Stored dismissals: sorted (smaller, larger) SHA
    /// pairs. A dismissed pair is never re-proposed.
    pub dismissals: Vec<(String, String)>,
}

impl Default for FingerprintIndex {
    fn default() -> Self {
        FingerprintIndex {
            version: INDEX_VERSION.to_string(),
            images: Vec::new(),
            music: Vec::new(),
            texts: Vec::new(),
            dismissals: Vec::new(),
        }
    }
}

impl FingerprintIndex {
    /// Load the index. A missing file is a fresh
    /// library, not an error. An unreadable file IS an
    /// error: silently starting empty would drop
    /// dismissals and re-propose rejected pairs.
    pub fn load(path: &Path) -> Result<FingerprintIndex, String> {
        match std::fs::read_to_string(path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(FingerprintIndex::default()),
            Err(e) => Err(format!("could not read the fingerprint index: {e}")),
            Ok(text) => serde_json::from_str(&text)
                .map_err(|e| format!("the fingerprint index is damaged: {e}")),
        }
    }

    /// Save the index, creating parent directories.
    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("could not create the index folder: {e}"))?;
        }
        let text = serde_json::to_string_pretty(self)
            .map_err(|e| format!("could not write the fingerprint index: {e}"))?;
        std::fs::write(path, text)
            .map_err(|e| format!("could not write the fingerprint index: {e}"))
    }

    /// Remember that a pair is not duplicates. The pair
    /// is stored sorted, so dismissal order does not
    /// matter, and duplicates are ignored.
    pub fn dismiss(&mut self, a: &str, b: &str) {
        let pair = if a <= b {
            (a.to_string(), b.to_string())
        } else {
            (b.to_string(), a.to_string())
        };
        if !self.dismissals.contains(&pair) {
            self.dismissals.push(pair);
        }
    }

    pub fn is_dismissed(&self, a: &str, b: &str) -> bool {
        let pair = if a <= b {
            (a.to_string(), b.to_string())
        } else {
            (b.to_string(), a.to_string())
        };
        self.dismissals.contains(&pair)
    }

    /// Document frequencies for the boilerplate guard:
    /// shingle -> documents containing it, and the
    /// total text-document count.
    pub fn doc_freq(&self) -> (HashMap<u64, usize>, usize) {
        let mut freq: HashMap<u64, usize> = HashMap::new();
        for entry in &self.texts {
            for s in &entry.shingles {
                *freq.entry(*s).or_insert(0) += 1;
            }
        }
        (freq, self.texts.len())
    }

    /// Entries whose fingerprint version is no longer
    /// current, by (kind, sha, name). The app
    /// recomputes these from the originals.
    pub fn stale_entries(&self) -> Vec<(&str, &str, &str)> {
        let mut out = Vec::new();
        for e in &self.images {
            if e.fp_version != crate::IMAGE_FP_VERSION {
                out.push(("image", e.sha256.as_str(), e.name.as_str()));
            }
        }
        for e in &self.music {
            if e.fp_version != crate::MUSIC_FP_VERSION {
                out.push(("music", e.sha256.as_str(), e.name.as_str()));
            }
        }
        for e in &self.texts {
            if e.fp_version != crate::TEXT_FP_VERSION {
                out.push(("text", e.sha256.as_str(), e.name.as_str()));
            }
        }
        out
    }

    /// Image proposals for a new picture, dismissals
    /// already filtered out. (Match, entry name) pairs.
    pub fn find_image_matches(&self, sha: &str, fp: &ImageFingerprint) -> Vec<(Match, String)> {
        let mut out = Vec::new();
        for entry in &self.images {
            if self.is_dismissed(sha, &entry.sha256) {
                continue;
            }
            let Some(engine) = entry.to_engine() else {
                continue;
            };
            if let Some(m) = match_image(fp, &engine, sha, &entry.sha256) {
                out.push((m, entry.name.clone()));
            }
        }
        out
    }

    /// Music proposals for a new tune. Scoring runs
    /// only over the candidate gate's buckets, never
    /// all-pairs over the library.
    pub fn find_music_matches(&self, sha: &str, fp: &MusicFingerprint) -> Vec<(Match, String)> {
        let mut engines: Vec<MusicFingerprint> = Vec::new();
        let mut names: Vec<String> = Vec::new();
        for entry in &self.music {
            if self.is_dismissed(sha, &entry.sha256) {
                continue;
            }
            if let Some(engine) = entry.to_engine() {
                engines.push(engine);
                names.push(entry.name.clone());
            }
        }
        // Exact-hash tiers bypass the gate: a gate miss
        // must never hide an exact match.
        let mut out = Vec::new();
        let mut gated: Vec<bool> = vec![false; engines.len()];
        for (i, engine) in engines.iter().enumerate() {
            match music_decision(fp, engine) {
                MusicDecision::SameContent
                | MusicDecision::SameNotesDifferentSpelling
                | MusicDecision::ProbablyMelody => {
                    if let Some(m) = match_music(fp, engine) {
                        out.push((m, names[i].clone()));
                        gated[i] = true;
                    }
                }
                _ => {}
            }
        }
        let gate = MusicCandidateIndex::new(engines.clone());
        for i in gate.candidates(fp) {
            if gated[i] {
                continue;
            }
            if let Some(m) = match_music(fp, &engines[i]) {
                out.push((m, names[i].clone()));
            }
        }
        out
    }

    /// Text proposals for a new block, plus the
    /// candidates that need the short-text second step:
    /// (entry sha, entry name) pairs where at least one
    /// side is under 15 tokens. The caller fetches
    /// those source texts and calls
    /// [`match_short_texts`].
    pub fn find_text_matches(
        &self,
        sha: &str,
        fp: &TextFingerprint,
    ) -> (TextProposals, ShortCandidates) {
        let (freq, total) = self.doc_freq();
        let df = Some((&freq, total));
        let mut out = Vec::new();
        let mut needs = Vec::new();
        for entry in &self.texts {
            if self.is_dismissed(sha, &entry.sha256) {
                continue;
            }
            let Some(engine) = entry.to_engine() else {
                continue;
            };
            match text_band(fp, &engine, df) {
                TextDecision::NeedsShortTexts => {
                    needs.push((entry.sha256.clone(), entry.name.clone()));
                }
                TextDecision::NoMatch => {}
                _ => {
                    if let Some(m) = match_text(fp, &engine, df) {
                        out.push((m, entry.name.clone()));
                    }
                }
            }
        }
        (out, needs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::fingerprint_text;

    fn image_fp(ph: u64, dh: u64) -> ImageFingerprint {
        ImageFingerprint {
            fp_version: crate::IMAGE_FP_VERSION,
            phash: ph,
            dhash: dh,
            hist: [0u32; 64],
            width: 0,
            height: 0,
            low_information: false,
        }
    }

    fn music_fp(
        canon: &str,
        midi: &str,
        melody: &str,
        intervals: Vec<i16>,
        rhythm: Vec<(u32, u32, bool)>,
    ) -> MusicFingerprint {
        MusicFingerprint {
            fp_version: crate::MUSIC_FP_VERSION,
            canon_hash: canon.to_string(),
            midi_hash: midi.to_string(),
            melody_hash: melody.to_string(),
            intervals,
            rhythm,
            incipit: String::new(),
            pae: String::new(),
        }
    }

    #[test]
    fn same_bytes_are_same_file_and_group_silently() {
        let a = image_fp(0, 0);
        let b = image_fp(0, 0);
        let m = match_image(&a, &b, "abc", "abc").expect("same bytes match");
        assert_eq!(m.kind, MatchKind::SameFile);
        assert_eq!(m.title, "Same file");
        assert!(m.may_group_silently());
    }

    #[test]
    fn close_hashes_propose_and_colour_veto_blocks() {
        let a = image_fp(0, 0);
        let close = image_fp(0b111, 0b101);
        let m = match_image(&a, &close, "aaa", "bbb").expect("close hashes propose");
        assert_eq!(m.kind, MatchKind::NeedsCheck);
        assert!(!m.may_group_silently());
        assert!(m.reason.contains("3 of 64"));
        // Same hashes but recoloured: histogram veto.
        let mut recoloured = image_fp(0, 0);
        recoloured.hist[63] = 1000;
        let mut base_hist = [0u32; 64];
        base_hist[0] = 1000;
        let mut base = image_fp(0, 0);
        base.hist = base_hist;
        assert_eq!(match_image(&base, &recoloured, "aaa", "bbb"), None);
        // Version mismatch never matches.
        let mut stale = image_fp(0, 0);
        stale.fp_version = "img-fp-v0";
        assert_eq!(match_image(&a, &stale, "aaa", "bbb"), None);
    }

    #[test]
    fn music_tiers_honour_the_amendments() {
        let rhythm = vec![(1, 1, false), (1, 1, false)];
        let canon = music_fp("c1", "m1", "mel1", vec![2, 2], rhythm.clone());
        let same = music_fp("c1", "m1", "mel1", vec![2, 2], rhythm.clone());
        let m = match_music(&canon, &same).expect("canon equal groups");
        assert_eq!(m.kind, MatchKind::SameContent);
        assert!(m.may_group_silently());
        // MIDI equal, spelling differs: asks, own label.
        let respelled = music_fp("c2", "m1", "mel1", vec![2, 2], rhythm.clone());
        let m = match_music(&canon, &respelled).expect("midi equal asks");
        assert_eq!(m.kind, MatchKind::NeedsCheck);
        assert!(m.title.contains("different spelling"));
        assert!(!m.may_group_silently());
        // Melody equal, harmony differs: asks, own label.
        let reharmonised = music_fp("c3", "m2", "mel1", vec![2, 2], rhythm.clone());
        let m = match_music(&canon, &reharmonised).expect("melody equal asks");
        assert!(m.title.contains("different harmony"));
        // Transposition: intervals and rhythm equal,
        // hashes differ: the reason says so.
        let transposed = music_fp("c4", "m3", "mel2", vec![2, 2], rhythm.clone());
        let m = match_music(&canon, &transposed).expect("transposed proposes");
        assert_eq!(m.reason, "the same tune in another key");
    }

    #[test]
    fn text_match_states_shared_word_groups() {
        let a = fingerprint_text(
            "lord in your mercy hear our prayer for all who are sick and in need today alpha beta",
        );
        let b = fingerprint_text(
            "lord in your mercy hear our prayer for all who are sick and in need today one two",
        );
        let m = match_text(&a, &b, None).expect("shared run proposes");
        assert_eq!(m.kind, MatchKind::NeedsCheck);
        assert!(m.reason.contains("word-groups"));
        // Retyped identical words: silent tier.
        let c = fingerprint_text("LORD, in Your Mercy — hear our prayer!");
        let d = fingerprint_text("lord in your mercy hear our prayer");
        let m = match_text(&c, &d, None).expect("retyped identical groups");
        assert_eq!(m.kind, MatchKind::SameContent);
        assert!(m.may_group_silently());
    }

    #[test]
    fn index_round_trips_and_dismissals_filter() {
        let dir = std::env::temp_dir().join("ob-fp-index-test");
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("fingerprints.json");
        let mut index = FingerprintIndex::default();
        let fp = image_fp(0, 0);
        index
            .images
            .push(ImageEntry::from_engine("sha1", "cross.png", &fp));
        index.dismiss("sha1", "sha2");
        index.save(&path).expect("save works");
        let loaded = FingerprintIndex::load(&path).expect("load works");
        assert!(loaded.is_dismissed("sha2", "sha1"));
        assert_eq!(loaded.images.len(), 1);
        // A dismissed pair is never re-proposed.
        assert!(loaded.find_image_matches("sha2", &fp).is_empty());
        // A missing file is a fresh index, not an error.
        let fresh = FingerprintIndex::load(&dir.join("missing.json")).expect("fresh index");
        assert!(fresh.images.is_empty());
        // A damaged file is a loud error, never a silent
        // empty index (that would drop dismissals).
        std::fs::write(&path, "{not json").expect("write damaged");
        assert!(FingerprintIndex::load(&path).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn stale_entries_never_match_but_are_listed() {
        let mut index = FingerprintIndex::default();
        let mut entry = ImageEntry::from_engine("sha1", "old.png", &image_fp(0, 0));
        entry.fp_version = "img-fp-v0".to_string();
        index.images.push(entry);
        assert_eq!(index.stale_entries().len(), 1);
        assert!(index.find_image_matches("sha2", &image_fp(0, 0)).is_empty());
    }

    #[test]
    fn short_text_query_routes_to_needs_texts() {
        let mut index = FingerprintIndex::default();
        let stored = fingerprint_text("we pray for the sick");
        index
            .texts
            .push(TextEntry::from_engine("sha1", "prayers", &stored));
        let query = fingerprint_text("we pray for the ill");
        let (matches, needs) = index.find_text_matches("sha2", &query);
        assert!(matches.is_empty());
        assert_eq!(needs.len(), 1);
        // The second step, with source texts in hand,
        // shows the diff.
        let a = crate::text::tokenise("we pray for the sick");
        let b = crate::text::tokenise("we pray for the ill");
        let m = match_short_texts(&a, &b).expect("short pair proposes");
        assert!(m.reason.contains("sick"));
        assert!(m.reason.contains("ill"));
    }
}
