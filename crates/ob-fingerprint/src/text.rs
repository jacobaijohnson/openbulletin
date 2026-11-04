//! Text fingerprints, version `text-fp-v1` (see
//! [`crate::TEXT_FP_VERSION`]).
//!
//! At church-library scale the unfashionable design is
//! the right one: store the FULL 3-word shingle set and
//! compute EXACT Jaccard and containment. MinHash is a
//! lossy compression of exactly that set, designed for
//! web-scale corpora; on a 30-word prayer its estimate
//! is statistically coarse (standard error
//! sqrt(J(1-J)/k), about 0.044 at k=128 even before the
//! small-document degeneracy, where a short document
//! can have fewer shingles than the signature length).
//! MinHash appears nowhere here, and if it ever does,
//! that formula is the error bar that must be stated.
//! SimHash (64-bit) is computed and stored, but it never
//! decides a match: at this scale exact set intersection
//! is cheaper than a filter miss, and a filter that can
//! miss a true duplicate is worse than a slower exact
//! pass. If libraries grow past roughly 100,000 blocks,
//! gate on SimHash distance <= 6 first, then re-score
//! exactly; until then SimHash is recorded, not used to
//! exclude.
//!
//! Normalisation (all of it is the semantics, so it is
//! pinned): Unicode NFC, Unicode lowercase, punctuation
//! unified to separators. Numbers, dates, and names are
//! KEPT: normalising them away would make different
//! weeks' announcements match, which is not a match.
//!
//! Winnowing (k=5 words, w=4) supplies shared-passage
//! positions for the review screen: any shared run of
//! at least k+w-1 = 8 tokens yields at least one shared
//! fingerprint, so "these announcements share this
//! paragraph" is locatable, not just scored.
//!
//! Short texts (under 15 tokens) never get shingle
//! scores: their shingle sets are too small for the
//! bands to mean anything. They fall back to direct
//! token-multiset comparison, and the diff is always
//! shown — a changed date, name, or one "not" changes
//! the meaning, not the score.
//!
//! Boilerplate: shingles present in more than 25% of
//! the library are furniture, not evidence. When a
//! document-frequency map is supplied, those shingles
//! are excluded from both sets before scoring (and
//! annotated, never silently dropped, in the reason).
//! When both filtered sets come out empty, there is
//! nothing distinctive shared: NoMatch, never a
//! vacuous empty-equals-empty score.

use sha2::{Digest, Sha256};
use std::collections::HashMap;
use unicode_normalization::UnicodeNormalization;

/// Split text into tokens: NFC, lowercase, punctuation
/// unified to separators. Numbers, dates, and names
/// survive as tokens.
pub fn tokenise(text: &str) -> Vec<String> {
    let normalised: String = text.nfc().collect();
    let mut tokens = Vec::new();
    let mut current = String::new();
    for c in normalised.to_lowercase().chars() {
        if c.is_alphanumeric() {
            current.push(c);
        } else if !current.is_empty() {
            tokens.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

/// The normalised text: tokens joined by single spaces.
/// Its SHA-256 is the Layer-1 exact tier for text.
pub fn normalised_text(text: &str) -> String {
    tokenise(text).join(" ")
}

fn sha_hex(text: &str) -> String {
    let mut h = Sha256::new();
    h.update(text.as_bytes());
    let sum = h.finalize();
    let mut out = String::with_capacity(64);
    for b in sum {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

/// A stable 64-bit hash of one shingle: SHA-256, first 8
/// bytes, big-endian. Never Rust's DefaultHasher, whose
/// algorithm is explicitly not guaranteed stable across
/// releases — a fingerprint format must survive
/// toolchain upgrades.
fn hash_shingle(shingle: &str) -> u64 {
    let mut h = Sha256::new();
    h.update(shingle.as_bytes());
    let sum = h.finalize();
    let mut out: u64 = 0;
    for b in sum.iter().take(8) {
        out = (out << 8) | *b as u64;
    }
    out
}

/// The sorted, deduplicated 3-word shingle set.
pub fn shingle_set(tokens: &[String]) -> Vec<u64> {
    let mut set: Vec<u64> = tokens
        .windows(3)
        .map(|w| hash_shingle(&w.join("\u{1f}")))
        .collect();
    set.sort_unstable();
    set.dedup();
    set
}

/// Exact intersection size of two sorted sets.
fn intersection_size(a: &[u64], b: &[u64]) -> usize {
    let mut i = 0;
    let mut j = 0;
    let mut n = 0;
    while i < a.len() && j < b.len() {
        match a[i].cmp(&b[j]) {
            std::cmp::Ordering::Equal => {
                n += 1;
                i += 1;
                j += 1;
            }
            std::cmp::Ordering::Less => i += 1,
            std::cmp::Ordering::Greater => j += 1,
        }
    }
    n
}

/// Exact Jaccard and containment of two sorted shingle
/// sets, with the counts the reason strings need.
#[derive(Debug, Clone, Copy)]
pub struct SetScores {
    pub jaccard: f64,
    pub containment: f64,
    pub shared: usize,
    pub union: usize,
    pub smaller: usize,
}

pub fn set_scores(a: &[u64], b: &[u64]) -> SetScores {
    if a.is_empty() && b.is_empty() {
        // Never a vacuous match: see the module docs.
        // Callers treat empty filtered sets as NoMatch.
        return SetScores {
            jaccard: 0.0,
            containment: 0.0,
            shared: 0,
            union: 0,
            smaller: 0,
        };
    }
    let shared = intersection_size(a, b);
    let union = a.len() + b.len() - shared;
    let smaller = a.len().min(b.len());
    SetScores {
        jaccard: if union == 0 {
            0.0
        } else {
            shared as f64 / union as f64
        },
        containment: if smaller == 0 {
            0.0
        } else {
            shared as f64 / smaller as f64
        },
        shared,
        union,
        smaller,
    }
}

/// Winnowing fingerprints: (hash, gram start index) for
/// each selected 5-gram. Window w=4, minimum hash wins,
/// ties go to the rightmost gram. Consecutive windows
/// selecting the same gram yield one entry.
pub fn winnow(tokens: &[String]) -> Vec<(u64, usize)> {
    let grams: Vec<u64> = tokens
        .windows(5)
        .map(|w| hash_shingle(&w.join("\u{1f}")))
        .collect();
    if grams.is_empty() {
        return Vec::new();
    }
    let mut out: Vec<(u64, usize)> = Vec::new();
    if grams.len() < 4 {
        let mut best = grams[0];
        let mut best_pos = 0;
        for (i, h) in grams.iter().enumerate() {
            if *h <= best {
                best = *h;
                best_pos = i;
            }
        }
        out.push((best, best_pos));
        return out;
    }
    for start in 0..=(grams.len() - 4) {
        let window = &grams[start..start + 4];
        let mut best = window[0];
        let mut best_pos = start;
        for (i, h) in window.iter().enumerate() {
            if *h <= best {
                best = *h;
                best_pos = start + i;
            }
        }
        let entry = (best, best_pos);
        if out.last() != Some(&entry) {
            out.push(entry);
        }
    }
    out
}

/// Shared winnow positions: (position in a, position in
/// b) for each hash present in both, first occurrence
/// each. The review screen maps positions back to token
/// ranges in the source texts.
pub fn shared_passages(a: &[(u64, usize)], b: &[(u64, usize)]) -> Vec<(usize, usize)> {
    let mut by_hash: HashMap<u64, usize> = HashMap::new();
    for (h, pos) in b {
        by_hash.entry(*h).or_insert(*pos);
    }
    let mut out = Vec::new();
    for (h, pos) in a {
        if let Some(bpos) = by_hash.get(h) {
            out.push((*pos, *bpos));
        }
    }
    out
}

/// 64-bit SimHash over token features: per bit, +1 when
/// the token hash sets it, -1 otherwise; final bit is 1
/// when the sum is positive. Recorded for a future
/// candidate gate; never decides a match alone.
pub fn simhash(tokens: &[String]) -> u64 {
    let mut sums = [0i32; 64];
    for token in tokens {
        let h = hash_shingle(token);
        for (b, sum) in sums.iter_mut().enumerate() {
            if (h >> b) & 1 == 1 {
                *sum += 1;
            } else {
                *sum -= 1;
            }
        }
    }
    let mut out: u64 = 0;
    for (b, sum) in sums.iter().enumerate() {
        if *sum > 0 {
            out |= 1 << b;
        }
    }
    out
}

/// A text block's fingerprint: the exact Layer-1 hash,
/// the full shingle set, winnowing positions, the
/// SimHash record, and the token count that routes
/// short texts to the fallback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextFingerprint {
    pub fp_version: &'static str,
    pub norm_hash: String,
    pub shingles: Vec<u64>,
    pub winnow: Vec<(u64, usize)>,
    pub simhash: u64,
    pub token_count: usize,
}

/// Fingerprint one text block.
pub fn fingerprint_text(text: &str) -> TextFingerprint {
    let tokens = tokenise(text);
    TextFingerprint {
        fp_version: crate::TEXT_FP_VERSION,
        norm_hash: sha_hex(&tokens.join(" ")),
        shingles: shingle_set(&tokens),
        winnow: winnow(&tokens),
        simhash: simhash(&tokens),
        token_count: tokens.len(),
    }
}

/// Texts under this many tokens never get shingle
/// scores; they go through the token fallback.
pub const SHORT_TOKEN_LIMIT: usize = 15;

/// Multiset Jaccard of two token lists, with the
/// multiset differences the review screen shows.
pub fn short_text_scores(a: &[String], b: &[String]) -> (f64, Vec<String>, Vec<String>) {
    let mut counts_b: HashMap<&str, usize> = HashMap::new();
    for t in b {
        *counts_b.entry(t.as_str()).or_insert(0) += 1;
    }
    let mut counts_a: HashMap<&str, usize> = HashMap::new();
    for t in a {
        *counts_a.entry(t.as_str()).or_insert(0) += 1;
    }
    let mut shared = 0usize;
    let mut only_a: Vec<String> = Vec::new();
    for t in a {
        if let Some(n) = counts_b.get_mut(t.as_str()) {
            if *n > 0 {
                *n -= 1;
                shared += 1;
                continue;
            }
        }
        only_a.push(t.clone());
    }
    let mut only_b: Vec<String> = Vec::new();
    for t in b {
        if let Some(n) = counts_a.get_mut(t.as_str()) {
            if *n > 0 {
                *n -= 1;
                continue;
            }
        }
        only_b.push(t.clone());
    }
    let union = a.len() + b.len() - shared;
    let jaccard = if union == 0 {
        0.0
    } else {
        shared as f64 / union as f64
    };
    (jaccard, only_a, only_b)
}

/// What two text fingerprints mean together, before
/// any band is applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextDecision {
    /// Normalised texts equal.
    SameContent,
    /// Shingle band reached.
    VeryLikely,
    /// Shingle band reached.
    Possibly,
    /// The caller must supply the source texts: at
    /// least one side is under [`SHORT_TOKEN_LIMIT`]
    /// tokens, and shingle scores would not mean
    /// anything there.
    NeedsShortTexts,
    /// No tier, no band.
    NoMatch,
}

/// Score two text fingerprints. `df` is the library
/// document-frequency map (shingle -> documents
/// containing it) with the total document count: when
/// supplied, shingles in more than 25% of documents
/// are excluded from both sets before scoring.
pub fn text_band(
    a: &TextFingerprint,
    b: &TextFingerprint,
    df: Option<(&HashMap<u64, usize>, usize)>,
) -> TextDecision {
    if a.fp_version != b.fp_version {
        return TextDecision::NoMatch;
    }
    if a.norm_hash == b.norm_hash {
        return TextDecision::SameContent;
    }
    if a.token_count < SHORT_TOKEN_LIMIT || b.token_count < SHORT_TOKEN_LIMIT {
        return TextDecision::NeedsShortTexts;
    }
    let scores = text_scores(a, b, df);
    if scores.jaccard >= 0.90 {
        TextDecision::VeryLikely
    } else if scores.jaccard >= 0.70 || scores.containment >= 0.85 {
        TextDecision::Possibly
    } else {
        TextDecision::NoMatch
    }
}

/// The scores behind [`text_band`], with boilerplate
/// exclusion applied when a frequency map is supplied.
/// Empty filtered sets score NoMatch, never a vacuous
/// match.
pub fn text_scores(
    a: &TextFingerprint,
    b: &TextFingerprint,
    df: Option<(&HashMap<u64, usize>, usize)>,
) -> SetScores {
    match df {
        None => set_scores(&a.shingles, &b.shingles),
        Some((freq, total)) => {
            let kept = |set: &[u64]| {
                set.iter()
                    .copied()
                    .filter(|s| freq.get(s).copied().unwrap_or(0) * 4 <= total)
                    .collect::<Vec<u64>>()
            };
            let fa = kept(&a.shingles);
            let fb = kept(&b.shingles);
            if fa.is_empty() || fb.is_empty() {
                return SetScores {
                    jaccard: 0.0,
                    containment: 0.0,
                    shared: 0,
                    union: 0,
                    smaller: 0,
                };
            }
            set_scores(&fa, &fb)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens_of(text: &str) -> Vec<String> {
        tokenise(text)
    }

    #[test]
    fn normalisation_unifies_case_punctuation_and_nfc() {
        assert_eq!(
            tokens_of("Lord, in your mercy—hear our prayer!"),
            vec!["lord", "in", "your", "mercy", "hear", "our", "prayer"]
        );
        assert_eq!(tokens_of("St. Mary"), tokens_of("St Mary"));
        // Numbers, dates, and names survive: they are
        // meaning, not formatting.
        assert_eq!(
            tokens_of("Psalm 23, Dec 25"),
            vec!["psalm", "23", "dec", "25"]
        );
        // Composed vs decomposed e-acute normalise alike.
        assert_eq!(
            fingerprint_text("café").norm_hash,
            fingerprint_text("cafe\u{301}").norm_hash
        );
    }

    #[test]
    fn shingle_jaccard_and_containment_are_exact() {
        let base: Vec<String> = (0..20).map(|i| format!("w{i}")).collect();
        let mut changed = base.clone();
        changed[10] = "other".to_string();
        let a = fingerprint_text(&base.join(" "));
        let b = fingerprint_text(&changed.join(" "));
        // 18 shingles; one changed word touches 3 of
        // them: intersection 15, union 21.
        let s = text_scores(&a, &b, None);
        assert_eq!(s.shared, 15);
        assert_eq!(s.union, 21);
        assert!((s.jaccard - 15.0 / 21.0).abs() < 1e-9);
        assert!((s.containment - 15.0 / 18.0).abs() < 1e-9);
        // A short text inside a long one: containment 1,
        // Jaccard low.
        let short = fingerprint_text(&base[5..10].join(" "));
        let s2 = text_scores(&short, &a, None);
        assert_eq!(s2.containment, 1.0);
        assert!(s2.jaccard < 0.3);
    }

    #[test]
    fn winnowing_finds_a_shared_passage() {
        let a =
            fingerprint_text("alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu");
        let b = fingerprint_text(
            "one two three alpha beta gamma delta epsilon zeta eta theta iota kappa nine ten",
        );
        // The shared run is 10 tokens (>= k+w-1 = 8), so
        // at least one winnow fingerprint must be shared.
        assert!(!shared_passages(&a.winnow, &b.winnow).is_empty());
    }

    #[test]
    fn simhash_is_deterministic_and_zero_for_identical() {
        let a = fingerprint_text("lord hear our prayer for the sick");
        let b = fingerprint_text("lord hear our prayer for the sick");
        assert_eq!(a.simhash, b.simhash);
        assert_eq!(a.simhash ^ b.simhash, 0);
    }

    #[test]
    fn short_texts_route_to_the_token_fallback() {
        let a = fingerprint_text("we pray for the sick");
        let b = fingerprint_text("we pray for the ill");
        assert_eq!(text_band(&a, &b, None), TextDecision::NeedsShortTexts);
        let ta = tokens_of("we pray for the sick");
        let tb = tokens_of("we pray for the ill");
        let (j, only_a, only_b) = short_text_scores(&ta, &tb);
        assert!((j - 4.0 / 6.0).abs() < 1e-9);
        assert_eq!(only_a, vec!["sick".to_string()]);
        assert_eq!(only_b, vec!["ill".to_string()]);
    }

    #[test]
    fn boilerplate_exclusion_empties_to_no_match() {
        // A 16-token shared run: 14 shared shingles of
        // 16 each side, Jaccard 14/18, so Possibly.
        let a2 = fingerprint_text(
            "lord in your mercy hear our prayer for all who are sick and in need today alpha beta",
        );
        let b2 = fingerprint_text(
            "lord in your mercy hear our prayer for all who are sick and in need today one two",
        );
        assert_eq!(text_band(&a2, &b2, None), TextDecision::Possibly);
        // Pretend every shingle in the two texts is
        // boilerplate: document frequency 1 or 2 of 2
        // documents, both over 25%, so all are excluded.
        let mut freq: HashMap<u64, usize> = HashMap::new();
        for s in a2.shingles.iter().chain(b2.shingles.iter()) {
            *freq.entry(*s).or_insert(0) += 1;
        }
        let s = text_scores(&a2, &b2, Some((&freq, 2)));
        assert_eq!(s.shared, 0);
        assert_eq!(text_band(&a2, &b2, Some((&freq, 2))), TextDecision::NoMatch);
    }
}
