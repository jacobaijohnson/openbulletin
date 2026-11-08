//! The keep-time dedupe decision.
//!
//! The caller loads the store and the fingerprint index
//! and hands both in; [`decide_keep`] mutates neither.
//! Storing is a separate step ([`store_snippet`]), so
//! the caller can ask the admin first. Nothing is ever
//! merged or deleted at any tier: exact tiers group
//! silently (no second copy) — that is dedupe, not a
//! merge.
//!
//! Decision algorithm, in order:
//!
//! 1. `sha256_hex(content)`. If the store already holds
//!    `id == sha` → `AlreadyKept`. No second copy, ever,
//!    on identical bytes.
//! 2. Compute the per-kind fingerprint (image:
//!    `ob_image::decode` then `fingerprint_image`;
//!    music: `fingerprint_music` — a parse failure is a
//!    loud error, never a silent wrong kind; text:
//!    `fingerprint_text`).
//! 3. Exact-content check against the STORE's own
//!    records: music `canon_hash` equal → `AlreadyKept`;
//!    text `norm_hash` equal → `AlreadyKept`. (Images
//!    have no canonical hash; their exact tier is the
//!    byte tier in step 1.) Stale store records never
//!    match here: a stale fingerprint is incomparable.
//! 4. Index consult. Exact-tier matches (SameFile /
//!    SameContent) need no action beyond steps 1–3: they
//!    mean the content is known to the library but was
//!    never kept, so the decision is `ReadyToStore`.
//!    NeedsCheck matches (stored dismissals already
//!    filtered) → `Ask`, the proposals carrying their
//!    concrete reasons, never a bare percentage. No
//!    matches → `ReadyToStore`.
//! 5. Short-text second step: for each short-text
//!    candidate whose sha is a snippet in THIS store,
//!    read its bytes from the bytes dir, tokenise both
//!    sides the way `ob-fingerprint/src/text.rs` does,
//!    and call `match_short_texts`. A proposal joins
//!    the Ask. A candidate whose sha is NOT in the store
//!    is index-only — the index holds no words, so it
//!    cannot be compared — and is skipped. That is the
//!    one documented limit of keep-time short-text
//!    dedupe.
//!
//! The index consult below mirrors
//! `FingerprintIndex::find_*_matches` exactly (same
//! dismissals filter, same stale refusal, same match
//! calls, same music candidate gate); the index methods
//! return names only, and a dismissal needs the entry
//! SHA, so the keep decision collects proposals and
//! entry SHAs together. The equivalence is pinned by
//! `keep_proposals_match_the_index_findings`.
//!
//! Ask resolution, all explicit:
//!
//! * Keep anyway → [`store_snippet`] with the decision's
//!   record.
//! * Not duplicates → [`dismiss_proposals`] for each
//!   proposal the admin rejects, save the index, then
//!   store. The pair is never re-proposed.
//! * Don't keep → store nothing, write nothing.
//!
//! A stored dismissal is honoured: with all matches
//! dismissed, the decision is `ReadyToStore` with no
//! Ask.

use crate::store::{SnippetFingerprint, SnippetKind, SnippetRecord, SnippetStore};
use ob_fingerprint::image::{fingerprint_image, ImageFingerprint};
use ob_fingerprint::matching::{
    match_image, match_music, match_short_texts, match_text, FingerprintIndex, ImageEntry, Match,
    MatchKind, MusicEntry, TextEntry,
};
use ob_fingerprint::music::{
    fingerprint_music, music_decision, MusicCandidateIndex, MusicDecision, MusicFingerprint,
};
use ob_fingerprint::text::{fingerprint_text, text_band, tokenise, TextDecision};
use ob_fingerprint::{sha256_hex, IMAGE_FP_VERSION, MUSIC_FP_VERSION, TEXT_FP_VERSION};
use ob_model::{Asset, AssetKind, RightsRecord};
use std::path::Path;

/// What the caller wants kept, with everything the
/// record needs. For text, `content` is the paragraph's
/// UTF-8 words.
#[derive(Debug, Clone)]
pub struct KeepInput {
    pub content: Vec<u8>,
    pub kind: SnippetKind,
    pub name: String,
    pub source: String,
    pub kept_at: String,
    pub asserted_by: String,
    /// IANA media type, e.g. "image/png". Ignored for
    /// text, which always records `text/plain`.
    pub media_type: String,
    pub rights: RightsRecord,
    pub spoken_only: bool,
    pub web_hidden: bool,
}

/// What keeping this content means.
#[derive(Debug, Clone)]
pub enum KeepDecision {
    /// Already in the store, under this id. Store
    /// nothing.
    AlreadyKept { id: String },
    /// No duplicate, no question: store the record.
    ReadyToStore { record: SnippetRecord },
    /// Near-duplicates need the admin's eye. The
    /// proposals carry their concrete reasons; the
    /// entry SHAs, in the same order as `proposals`,
    /// are the keys [`dismiss_proposals`] needs when
    /// the admin answers "Not duplicates". The record
    /// is what [`store_snippet`] stores on "Keep
    /// anyway" (after any dismissals).
    Ask {
        record: SnippetRecord,
        proposals: Vec<(Match, String)>,
        entry_shas: Vec<String>,
    },
}

/// Proposals with the index entry SHA behind each
/// proposal, in the same order.
type ProposalsWithShas = (Vec<(Match, String)>, Vec<String>);

/// What the text index consult returns: the proposals
/// with their entry SHAs, plus the short-text
/// second-step candidates (entry sha, entry name).
type TextFindings = (ProposalsWithShas, Vec<(String, String)>);

/// Fingerprint one snippet's content, as the store
/// record holds it. A music parse failure and a picture
/// decode failure are loud errors; a text that is not
/// valid UTF-8 is a loud error too.
fn fingerprint_content(
    kind: SnippetKind,
    content: &[u8],
    sha: &str,
    name: &str,
) -> Result<SnippetFingerprint, String> {
    match kind {
        SnippetKind::Picture => {
            let decoded = ob_image::decode(content)
                .map_err(|e| format!("could not decode the picture: {e}"))?;
            let fp = fingerprint_image(&decoded);
            Ok(SnippetFingerprint::Picture(ImageEntry::from_engine(
                sha, name, &fp,
            )))
        }
        SnippetKind::Music => {
            let fp = fingerprint_music(content)?;
            Ok(SnippetFingerprint::Music(MusicEntry::from_engine(
                sha, name, &fp,
            )))
        }
        SnippetKind::Text => {
            let text = String::from_utf8(content.to_vec())
                .map_err(|e| format!("the text snippet is not valid UTF-8: {e}"))?;
            let fp = fingerprint_text(&text);
            Ok(SnippetFingerprint::Text(TextEntry::from_engine(
                sha, name, &fp,
            )))
        }
    }
}

/// Build the store record for a fresh fingerprint.
/// Asset kind is derived from the snippet kind; text
/// always records media type `text/plain`.
fn build_record(input: &KeepInput, sha: &str, fingerprint: SnippetFingerprint) -> SnippetRecord {
    let (asset_kind, media_type) = match input.kind {
        SnippetKind::Picture => (AssetKind::Picture, input.media_type.clone()),
        SnippetKind::Music => (AssetKind::Music, input.media_type.clone()),
        SnippetKind::Text => (AssetKind::Other, "text/plain".to_string()),
    };
    SnippetRecord {
        id: sha.to_string(),
        kind: input.kind,
        name: input.name.clone(),
        source: input.source.clone(),
        kept_at: input.kept_at.clone(),
        asserted_by: input.asserted_by.clone(),
        asset: Asset {
            sha256: sha.to_string(),
            kind: asset_kind,
            media_type,
            rights: input.rights.clone(),
        },
        fingerprint,
        spoken_only: input.spoken_only,
        web_hidden: input.web_hidden,
    }
}

/// Whether a record's fingerprint version is stale for
/// its kind. A stale snippet never matches for dedupe.
fn fingerprint_is_stale(kind: SnippetKind, fingerprint: &SnippetFingerprint) -> bool {
    let current = match kind {
        SnippetKind::Picture => IMAGE_FP_VERSION,
        SnippetKind::Music => MUSIC_FP_VERSION,
        SnippetKind::Text => TEXT_FP_VERSION,
    };
    fingerprint.fp_version() != current
}

/// The exact-content tier against the store's own
/// records (step 3): music by `canon_hash`, text by
/// `norm_hash`. Images have no canonical hash. Stale
/// records never match.
fn exact_store_match<'a>(
    store: &'a SnippetStore,
    kind: SnippetKind,
    fingerprint: &SnippetFingerprint,
) -> Option<&'a SnippetRecord> {
    match (kind, fingerprint) {
        (SnippetKind::Music, SnippetFingerprint::Music(entry)) => {
            let canon = entry.canon_hash.as_str();
            store.snippets.iter().find(|record| {
                !fingerprint_is_stale(record.kind, &record.fingerprint)
                    && matches!(&record.fingerprint,
                        SnippetFingerprint::Music(other) if other.canon_hash == canon)
            })
        }
        (SnippetKind::Text, SnippetFingerprint::Text(entry)) => {
            let norm = entry.norm_hash.as_str();
            store.snippets.iter().find(|record| {
                !fingerprint_is_stale(record.kind, &record.fingerprint)
                    && matches!(&record.fingerprint,
                        SnippetFingerprint::Text(other) if other.norm_hash == norm)
            })
        }
        _ => None,
    }
}

/// Image proposals with entry SHAs (see the module
/// docs: mirrors `find_image_matches`).
fn image_proposals(
    index: &FingerprintIndex,
    sha: &str,
    fp: &ImageFingerprint,
) -> ProposalsWithShas {
    let mut out = Vec::new();
    let mut shas = Vec::new();
    for entry in &index.images {
        if index.is_dismissed(sha, &entry.sha256) {
            continue;
        }
        let Some(engine) = entry.to_engine() else {
            continue;
        };
        if let Some(m) = match_image(fp, &engine, sha, &entry.sha256) {
            out.push((m, entry.name.clone()));
            shas.push(entry.sha256.clone());
        }
    }
    (out, shas)
}

/// Music proposals with entry SHAs (see the module
/// docs: mirrors `find_music_matches`, candidate gate
/// included; exact-hash tiers bypass the gate).
fn music_proposals(
    index: &FingerprintIndex,
    sha: &str,
    fp: &MusicFingerprint,
) -> ProposalsWithShas {
    let mut engines: Vec<MusicFingerprint> = Vec::new();
    let mut names: Vec<String> = Vec::new();
    let mut shas: Vec<String> = Vec::new();
    for entry in &index.music {
        if index.is_dismissed(sha, &entry.sha256) {
            continue;
        }
        if let Some(engine) = entry.to_engine() {
            engines.push(engine);
            names.push(entry.name.clone());
            shas.push(entry.sha256.clone());
        }
    }
    let mut out = Vec::new();
    let mut out_shas = Vec::new();
    let mut gated: Vec<bool> = vec![false; engines.len()];
    for (i, engine) in engines.iter().enumerate() {
        match music_decision(fp, engine) {
            MusicDecision::SameContent
            | MusicDecision::SameNotesDifferentSpelling
            | MusicDecision::ProbablyMelody => {
                if let Some(m) = match_music(fp, engine) {
                    out.push((m, names[i].clone()));
                    out_shas.push(shas[i].clone());
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
            out_shas.push(shas[i].clone());
        }
    }
    (out, out_shas)
}

/// Text proposals with entry SHAs, plus the short-text
/// second-step candidates (see the module docs: mirrors
/// `find_text_matches`).
fn text_proposals(
    index: &FingerprintIndex,
    sha: &str,
    fp: &ob_fingerprint::text::TextFingerprint,
) -> TextFindings {
    let (freq, total) = index.doc_freq();
    let df = Some((&freq, total));
    let mut out = Vec::new();
    let mut out_shas = Vec::new();
    let mut needs = Vec::new();
    for entry in &index.texts {
        if index.is_dismissed(sha, &entry.sha256) {
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
                    out_shas.push(entry.sha256.clone());
                }
            }
        }
    }
    ((out, out_shas), needs)
}

/// Decide whether to keep `input`'s content. The store
/// and the index are read, never mutated; the bytes dir
/// is read only for the short-text second step. Errors
/// are loud: an undecodable picture, an unparsable
/// tune, non-UTF-8 text, or a stored snippet whose
/// bytes cannot be read for the short-text comparison.
pub fn decide_keep(
    input: &KeepInput,
    store: &SnippetStore,
    index: &FingerprintIndex,
    bytes_dir: &Path,
) -> Result<KeepDecision, String> {
    // 1. Byte tier: identical bytes are already kept.
    let sha = sha256_hex(&input.content);
    if store.contains(&sha) {
        return Ok(KeepDecision::AlreadyKept { id: sha });
    }
    // 2. Per-kind fingerprint, then the record it would
    // become if stored.
    let fingerprint = fingerprint_content(input.kind, &input.content, &sha, &input.name)?;
    let record = build_record(input, &sha, fingerprint.clone());
    // 3. Exact-content tier against the store's own
    // records.
    if let Some(existing) = exact_store_match(store, input.kind, &fingerprint) {
        return Ok(KeepDecision::AlreadyKept {
            id: existing.id.clone(),
        });
    }
    // 4. Index consult (exact tiers need no action;
    // NeedsCheck asks). The engine fingerprint comes
    // back out of the fresh entry, which is always
    // current-version.
    let (mut proposals, mut entry_shas) = match &fingerprint {
        SnippetFingerprint::Picture(entry) => {
            let fp = entry.to_engine().expect("a fresh entry is current");
            image_proposals(index, &sha, &fp)
        }
        SnippetFingerprint::Music(entry) => {
            let fp = entry.to_engine().expect("a fresh entry is current");
            music_proposals(index, &sha, &fp)
        }
        SnippetFingerprint::Text(entry) => {
            let fp = entry.to_engine().expect("a fresh entry is current");
            let ((found, found_shas), needs) = text_proposals(index, &sha, &fp);
            let mut proposals = found;
            let mut shas = found_shas;
            // 5. Short-text second step, from the
            // store's own bytes.
            let text = String::from_utf8(input.content.clone())
                .map_err(|e| format!("the text snippet is not valid UTF-8: {e}"))?;
            let new_tokens = tokenise(&text);
            for (entry_sha, entry_name) in needs {
                let Some(stored) = store.get(&entry_sha) else {
                    // Index-only: the index holds no
                    // words, so it cannot be compared.
                    continue;
                };
                if fingerprint_is_stale(stored.kind, &stored.fingerprint) {
                    continue;
                }
                let stored_bytes = std::fs::read(bytes_dir.join(&entry_sha)).map_err(|e| {
                    format!("could not read the stored bytes for snippet {entry_sha}: {e}")
                })?;
                let stored_text = String::from_utf8(stored_bytes).map_err(|e| {
                    format!("the stored bytes for snippet {entry_sha} are not valid UTF-8: {e}")
                })?;
                if let Some(m) = match_short_texts(&new_tokens, &tokenise(&stored_text)) {
                    proposals.push((m, entry_name));
                    shas.push(entry_sha);
                }
            }
            (proposals, shas)
        }
    };
    // Only NeedsCheck proposals ask.
    let mut ask = Vec::new();
    let mut ask_shas = Vec::new();
    for ((proposal, name), entry_sha) in proposals.drain(..).zip(entry_shas.drain(..)) {
        if proposal.kind == MatchKind::NeedsCheck {
            ask.push((proposal, name));
            ask_shas.push(entry_sha);
        }
    }
    if ask.is_empty() {
        Ok(KeepDecision::ReadyToStore { record })
    } else {
        Ok(KeepDecision::Ask {
            record,
            proposals: ask,
            entry_shas: ask_shas,
        })
    }
}

/// Store a decided snippet: write the bytes file
/// (skipped when the file already exists), push the
/// record, save the store JSON; add the fingerprint
/// entry to the index when absent, save the index.
/// Returns the snippet id. Call after the decision is
/// `ReadyToStore`, or after the admin answers an Ask
/// (with any dismissals recorded first).
#[allow(clippy::too_many_arguments)]
pub fn store_snippet(
    store: &mut SnippetStore,
    index: &mut FingerprintIndex,
    record: SnippetRecord,
    content: &[u8],
    bytes_dir: &Path,
    store_path: &Path,
    index_path: &Path,
) -> Result<String, String> {
    std::fs::create_dir_all(bytes_dir)
        .map_err(|e| format!("could not create the snippet bytes folder: {e}"))?;
    let bytes_path = bytes_dir.join(&record.id);
    if !bytes_path.exists() {
        std::fs::write(&bytes_path, content)
            .map_err(|e| format!("could not write the snippet bytes: {e}"))?;
    }
    if !store.contains(&record.id) {
        store.snippets.push(record.clone());
    }
    store.save(store_path)?;
    match &record.fingerprint {
        SnippetFingerprint::Picture(entry) => {
            if !index.images.iter().any(|e| e.sha256 == entry.sha256) {
                index.images.push(entry.clone());
            }
        }
        SnippetFingerprint::Music(entry) => {
            if !index.music.iter().any(|e| e.sha256 == entry.sha256) {
                index.music.push(entry.clone());
            }
        }
        SnippetFingerprint::Text(entry) => {
            if !index.texts.iter().any(|e| e.sha256 == entry.sha256) {
                index.texts.push(entry.clone());
            }
        }
    }
    index.save(index_path)?;
    Ok(record.id.clone())
}

/// Record the admin's "Not duplicates" answer for the
/// given proposals: each entry SHA is dismissed against
/// the new content's SHA. The caller saves the index
/// afterwards (or stores, which saves it).
pub fn dismiss_proposals(index: &mut FingerprintIndex, sha: &str, entry_shas: &[String]) {
    for entry_sha in entry_shas {
        index.dismiss(sha, entry_sha);
    }
}

/// Records whose `fp_version` is stale for their kind.
/// A stale snippet never matches for dedupe; it is
/// listed here so the app can recompute it from the
/// stored original.
pub fn stale_snippets(store: &SnippetStore) -> Vec<&SnippetRecord> {
    store
        .snippets
        .iter()
        .filter(|record| fingerprint_is_stale(record.kind, &record.fingerprint))
        .collect()
}

/// Re-fingerprint every stale snippet from its stored
/// bytes and update the record (fp_version +
/// fingerprint). Returns the recomputed ids. A missing
/// bytes file is a loud error, never a silent skip.
/// The caller saves the store afterwards.
pub fn recompute_stale(store: &mut SnippetStore, bytes_dir: &Path) -> Result<Vec<String>, String> {
    let stale_ids: Vec<String> = stale_snippets(store)
        .iter()
        .map(|record| record.id.clone())
        .collect();
    let mut done = Vec::new();
    for id in stale_ids {
        let pos = store
            .snippets
            .iter()
            .position(|record| record.id == id)
            .expect("the stale list came from this store");
        let kind = store.snippets[pos].kind;
        let name = store.snippets[pos].name.clone();
        let bytes = std::fs::read(bytes_dir.join(&id))
            .map_err(|e| format!("could not read the stored bytes for snippet {id}: {e}"))?;
        let fresh = fingerprint_content(kind, &bytes, &id, &name)?;
        store.snippets[pos].fingerprint = fresh;
        done.push(id);
    }
    Ok(done)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{default_bytes_dir, default_store_path, SnippetStore, STORE_VERSION};
    use ob_fingerprint::matching::MatchKind;
    use std::path::PathBuf;

    fn test_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(name);
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn paths(dir: &Path) -> (PathBuf, PathBuf, PathBuf) {
        (
            dir.join("snippets.json"),
            dir.join("snippets"),
            dir.join("fingerprints.json"),
        )
    }

    fn input(kind: SnippetKind, content: Vec<u8>, name: &str) -> KeepInput {
        let media_type = match kind {
            SnippetKind::Picture => "image/png".to_string(),
            SnippetKind::Music => "application/vnd.recordare.musicxml+xml".to_string(),
            SnippetKind::Text => "text/plain".to_string(),
        };
        KeepInput {
            content,
            kind,
            name: name.to_string(),
            source: "old-bulletin.pdf".to_string(),
            kept_at: "2026-11-07".to_string(),
            asserted_by: "office".to_string(),
            media_type,
            rights: RightsRecord::default(),
            spoken_only: false,
            web_hidden: false,
        }
    }

    /// A PNG built from a pixel function, so the tests
    /// need no fixture files.
    fn png_bytes(w: u32, h: u32, pixel: impl Fn(u32, u32) -> (u8, u8, u8)) -> Vec<u8> {
        use image::{ImageBuffer, Rgb};
        let img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_fn(w, h, |x, y| {
            let (r, g, b) = pixel(x, y);
            Rgb([r, g, b])
        });
        let mut bytes = Vec::new();
        image::DynamicImage::ImageRgb8(img)
            .write_to(
                &mut std::io::Cursor::new(&mut bytes),
                image::ImageFormat::Png,
            )
            .expect("PNG encode works");
        bytes
    }

    /// A busy 32x32 grey picture: plenty of hash signal,
    /// so a small change stays inside the bands.
    fn busy_picture() -> Vec<u8> {
        png_bytes(32, 32, |x, y| {
            let v = ((x * 7 + y * 13 + x * y * 3) % 256) as u8;
            (v, v, v)
        })
    }

    /// The same picture with a 6x6 block brightened: a
    /// near-duplicate, not the same bytes.
    fn busy_picture_near() -> Vec<u8> {
        png_bytes(32, 32, |x, y| {
            let mut v = ((x * 7 + y * 13 + x * y * 3) % 256) as u8;
            if (10..16).contains(&x) && (10..16).contains(&y) {
                v = v.saturating_add(40);
            }
            (v, v, v)
        })
    }

    fn music_shell(notes: &str, key: &str) -> String {
        format!(
            r#"<?xml version="1.0"?><score-partwise>
            <part-list><score-part id="P1"><part-name>V</part-name></score-part></part-list>
            <part id="P1"><measure><attributes><divisions>4</divisions>{key}</attributes>
            {notes}
            </measure></part></score-partwise>"#
        )
    }

    fn music_note(step: &str, alter: &str, octave: u8, dur: u32, ntype: &str) -> String {
        let alter_xml = if alter.is_empty() {
            String::new()
        } else {
            format!("<alter>{alter}</alter>")
        };
        format!(
            "<note><pitch><step>{step}</step>{alter_xml}<octave>{octave}</octave></pitch>\
             <duration>{dur}</duration><type>{ntype}</type></note>"
        )
    }

    fn scale_c_xml() -> String {
        let mut notes = String::new();
        for (s, o) in [
            ("C", 4),
            ("D", 4),
            ("E", 4),
            ("F", 4),
            ("G", 4),
            ("A", 4),
            ("B", 4),
        ] {
            notes.push_str(&music_note(s, "", o, 4, "quarter"));
        }
        music_shell(&notes, "<key><fifths>0</fifths><mode>major</mode></key>")
    }

    /// The same scale with divisions rescaled 4 -> 480:
    /// different bytes, same canonical content.
    fn scale_c_rescaled_xml() -> String {
        let mut notes = String::new();
        for (s, o) in [
            ("C", 4),
            ("D", 4),
            ("E", 4),
            ("F", 4),
            ("G", 4),
            ("A", 4),
            ("B", 4),
        ] {
            notes.push_str(&format!(
                "<note><pitch><step>{s}</step><octave>{o}</octave></pitch>\
                 <duration>480</duration><type>quarter</type></note>"
            ));
        }
        format!(
            r#"<?xml version="1.0"?><score-partwise>
            <part-list><score-part id="P1"><part-name>V</part-name></score-part></part-list>
            <part id="P1"><measure><attributes><divisions>480</divisions></attributes>
            {notes}
            </measure></part></score-partwise>"#
        )
    }

    /// The scale transposed to D major: same tune shape
    /// in another key (a NeedsCheck band, not exact).
    fn scale_d_xml() -> String {
        let mut notes = String::new();
        for (s, al, o) in [
            ("D", "", 4),
            ("E", "", 4),
            ("F", "1", 4),
            ("G", "", 4),
            ("A", "", 4),
            ("B", "", 4),
            ("C", "1", 5),
        ] {
            notes.push_str(&music_note(s, al, o, 4, "quarter"));
        }
        music_shell(&notes, "<key><fifths>2</fifths><mode>major</mode></key>")
    }

    fn decide(
        kind: SnippetKind,
        content: Vec<u8>,
        name: &str,
        store: &SnippetStore,
        index: &FingerprintIndex,
        bytes_dir: &Path,
    ) -> KeepDecision {
        decide_keep(&input(kind, content, name), store, index, bytes_dir).expect("decide works")
    }

    fn store_decision(
        store: &mut SnippetStore,
        index: &mut FingerprintIndex,
        decision: KeepDecision,
        content: &[u8],
        bytes_dir: &Path,
        store_path: &Path,
        index_path: &Path,
    ) -> String {
        let record = match decision {
            KeepDecision::ReadyToStore { record } => record,
            KeepDecision::Ask { record, .. } => record,
            KeepDecision::AlreadyKept { .. } => panic!("nothing to store"),
        };
        store_snippet(
            store, index, record, content, bytes_dir, store_path, index_path,
        )
        .expect("store works")
    }

    #[test]
    fn same_bytes_twice_keeps_one_record() {
        let dir = test_dir("ob-snippets-same-bytes-test");
        let (store_path, bytes_dir, index_path) = paths(&dir);
        let mut store = SnippetStore::default();
        let mut index = FingerprintIndex::default();
        let content = busy_picture();
        let first = decide(
            SnippetKind::Picture,
            content.clone(),
            "busy.png",
            &store,
            &index,
            &bytes_dir,
        );
        assert!(matches!(first, KeepDecision::ReadyToStore { .. }));
        let id = store_decision(
            &mut store,
            &mut index,
            first,
            &content,
            &bytes_dir,
            &store_path,
            &index_path,
        );
        assert_eq!(store.snippets.len(), 1);
        let bytes_file = bytes_dir.join(&id);
        let written = std::fs::read(&bytes_file).expect("bytes written");
        assert_eq!(written, content);
        // The second keep of the same bytes is already
        // kept, and the bytes file is untouched.
        let second = decide(
            SnippetKind::Picture,
            content.clone(),
            "busy-again.png",
            &store,
            &index,
            &bytes_dir,
        );
        assert!(matches!(second, KeepDecision::AlreadyKept { id: ref kept } if *kept == id));
        assert_eq!(store.snippets.len(), 1);
        assert_eq!(
            std::fs::read(&bytes_file).expect("bytes still there"),
            content
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn same_music_under_another_name_is_already_kept() {
        let dir = test_dir("ob-snippets-same-music-test");
        let (store_path, bytes_dir, index_path) = paths(&dir);
        let mut store = SnippetStore::default();
        let mut index = FingerprintIndex::default();
        let first_xml = scale_c_xml();
        let first = decide(
            SnippetKind::Music,
            first_xml.into_bytes(),
            "scale.musicxml",
            &store,
            &index,
            &bytes_dir,
        );
        assert!(matches!(first, KeepDecision::ReadyToStore { .. }));
        let content = scale_c_xml().into_bytes();
        store_decision(
            &mut store,
            &mut index,
            first,
            &content,
            &bytes_dir,
            &store_path,
            &index_path,
        );
        // Same notes, rescaled divisions: different
        // bytes, same canonical hash.
        let rescaled = scale_c_rescaled_xml().into_bytes();
        assert_ne!(rescaled, content);
        let second = decide(
            SnippetKind::Music,
            rescaled,
            "scale-copy.musicxml",
            &store,
            &index,
            &bytes_dir,
        );
        assert!(
            matches!(second, KeepDecision::AlreadyKept { .. }),
            "rescaled same tune must group on canon_hash, got {second:?}"
        );
        assert_eq!(store.snippets.len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn retyped_text_is_already_kept() {
        let dir = test_dir("ob-snippets-retyped-text-test");
        let (store_path, bytes_dir, index_path) = paths(&dir);
        let mut store = SnippetStore::default();
        let mut index = FingerprintIndex::default();
        let first_text = "Lord, in your mercy hear our prayer";
        let first = decide(
            SnippetKind::Text,
            first_text.as_bytes().to_vec(),
            "prayer",
            &store,
            &index,
            &bytes_dir,
        );
        assert!(matches!(first, KeepDecision::ReadyToStore { .. }));
        let content = first_text.as_bytes().to_vec();
        store_decision(
            &mut store,
            &mut index,
            first,
            &content,
            &bytes_dir,
            &store_path,
            &index_path,
        );
        // Retyped: different spacing and capitals, same
        // normalised words.
        let retyped = "LORD,  in Your Mercy — hear our prayer!";
        assert_ne!(retyped.as_bytes(), content.as_slice());
        let second = decide(
            SnippetKind::Text,
            retyped.as_bytes().to_vec(),
            "prayer-copy",
            &store,
            &index,
            &bytes_dir,
        );
        assert!(
            matches!(second, KeepDecision::AlreadyKept { .. }),
            "retyped text must group on norm_hash, got {second:?}"
        );
        assert_eq!(store.snippets.len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn near_duplicate_picture_asks_and_dismissal_is_honoured() {
        let dir = test_dir("ob-snippets-near-picture-test");
        let (store_path, bytes_dir, index_path) = paths(&dir);
        let mut store = SnippetStore::default();
        let mut index = FingerprintIndex::default();
        let base = busy_picture();
        let near = busy_picture_near();
        assert_ne!(base, near);
        // Scenario A: keep-anyway stores.
        let asked = decide(
            SnippetKind::Picture,
            near.clone(),
            "busy-near.png",
            &store,
            &index,
            &bytes_dir,
        );
        // Nothing is in the index yet, so no Ask: store
        // the base first, then the near one asks.
        assert!(matches!(asked, KeepDecision::ReadyToStore { .. }));
        let base_decision = decide(
            SnippetKind::Picture,
            base.clone(),
            "busy.png",
            &store,
            &index,
            &bytes_dir,
        );
        store_decision(
            &mut store,
            &mut index,
            base_decision,
            &base,
            &bytes_dir,
            &store_path,
            &index_path,
        );
        let asked = decide(
            SnippetKind::Picture,
            near.clone(),
            "busy-near.png",
            &store,
            &index,
            &bytes_dir,
        );
        let (record, proposals, entry_shas) = match asked {
            KeepDecision::Ask {
                record,
                proposals,
                entry_shas,
            } => (record, proposals, entry_shas),
            other => panic!("near-duplicate must Ask, got {other:?}"),
        };
        assert_eq!(proposals.len(), 1);
        assert_eq!(proposals[0].0.kind, MatchKind::NeedsCheck);
        assert!(proposals[0].0.reason.contains("of 64"));
        assert_eq!(proposals[0].1, "busy.png");
        assert_eq!(entry_shas.len(), 1);
        // Keep-anyway stores.
        let id = store_snippet(
            &mut store,
            &mut index,
            record,
            &near,
            &bytes_dir,
            &store_path,
            &index_path,
        )
        .expect("keep-anyway stores");
        assert_eq!(store.snippets.len(), 2);
        assert_eq!(id, sha256_hex(&near));
        // Scenario B, fresh store and index: dismiss,
        // then the same pair is never re-asked.
        let dir_b = test_dir("ob-snippets-dismiss-test");
        let (store_path_b, bytes_dir_b, index_path_b) = paths(&dir_b);
        let mut store_b = SnippetStore::default();
        let mut index_b = FingerprintIndex::default();
        let base_decision = decide(
            SnippetKind::Picture,
            base.clone(),
            "busy.png",
            &store_b,
            &index_b,
            &bytes_dir_b,
        );
        store_decision(
            &mut store_b,
            &mut index_b,
            base_decision,
            &base,
            &bytes_dir_b,
            &store_path_b,
            &index_path_b,
        );
        let asked_b = decide(
            SnippetKind::Picture,
            near.clone(),
            "busy-near.png",
            &store_b,
            &index_b,
            &bytes_dir_b,
        );
        let (record_b, _proposals_b, entry_shas_b) = match asked_b {
            KeepDecision::Ask {
                record,
                proposals,
                entry_shas,
            } => (record, proposals, entry_shas),
            other => panic!("near-duplicate must Ask, got {other:?}"),
        };
        let sha_near = sha256_hex(&near);
        dismiss_proposals(&mut index_b, &sha_near, &entry_shas_b);
        index_b.save(&index_path_b).expect("save index");
        assert!(index_b.is_dismissed(&sha_near, &entry_shas_b[0]));
        // A second keep of the same pair, before
        // storing, decides ReadyToStore with no Ask.
        let again = decide(
            SnippetKind::Picture,
            near.clone(),
            "busy-near.png",
            &store_b,
            &index_b,
            &bytes_dir_b,
        );
        assert!(
            matches!(again, KeepDecision::ReadyToStore { .. }),
            "dismissed pair must not re-ask, got {again:?}"
        );
        store_snippet(
            &mut store_b,
            &mut index_b,
            record_b,
            &near,
            &bytes_dir_b,
            &store_path_b,
            &index_path_b,
        )
        .expect("store after dismissal");
        assert_eq!(store_b.snippets.len(), 2);
        // The index itself never re-proposes the
        // dismissed pair: the only match left is the
        // snippet's own entry, as Same file.
        let fp = fingerprint_content(SnippetKind::Picture, &near, &sha_near, "busy-near.png")
            .expect("fingerprint");
        let SnippetFingerprint::Picture(entry) = fp else {
            panic!("picture entry");
        };
        let engine = entry.to_engine().expect("current");
        let reproposed = index_b.find_image_matches(&sha_near, &engine);
        assert_eq!(reproposed.len(), 1);
        assert_eq!(reproposed[0].0.kind, MatchKind::SameFile);
        assert_eq!(reproposed[0].1, "busy-near.png");
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&dir_b);
    }

    #[test]
    fn stored_dismissal_means_no_ask() {
        let dir = test_dir("ob-snippets-predismissed-test");
        let (store_path, bytes_dir, index_path) = paths(&dir);
        let mut store = SnippetStore::default();
        let mut index = FingerprintIndex::default();
        let base = busy_picture();
        let near = busy_picture_near();
        let base_decision = decide(
            SnippetKind::Picture,
            base.clone(),
            "busy.png",
            &store,
            &index,
            &bytes_dir,
        );
        store_decision(
            &mut store,
            &mut index,
            base_decision,
            &base,
            &bytes_dir,
            &store_path,
            &index_path,
        );
        // Dismiss the pair up front, in the index.
        let sha_near = sha256_hex(&near);
        let sha_base = sha256_hex(&base);
        index.dismiss(&sha_near, &sha_base);
        index.save(&index_path).expect("save index");
        let decision = decide(
            SnippetKind::Picture,
            near,
            "busy-near.png",
            &store,
            &index,
            &bytes_dir,
        );
        assert!(
            matches!(decision, KeepDecision::ReadyToStore { .. }),
            "stored dismissal must mean no Ask, got {decision:?}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn short_texts_ask_with_the_diff() {
        let dir = test_dir("ob-snippets-short-text-test");
        let (store_path, bytes_dir, index_path) = paths(&dir);
        let mut store = SnippetStore::default();
        let mut index = FingerprintIndex::default();
        let first_text = "we pray for the sick";
        let first = decide(
            SnippetKind::Text,
            first_text.as_bytes().to_vec(),
            "prayers",
            &store,
            &index,
            &bytes_dir,
        );
        assert!(matches!(first, KeepDecision::ReadyToStore { .. }));
        let content = first_text.as_bytes().to_vec();
        store_decision(
            &mut store,
            &mut index,
            first,
            &content,
            &bytes_dir,
            &store_path,
            &index_path,
        );
        // Two 5-word paragraphs differing by one word.
        let second_text = "we pray for the ill";
        let asked = decide(
            SnippetKind::Text,
            second_text.as_bytes().to_vec(),
            "prayers-ill",
            &store,
            &index,
            &bytes_dir,
        );
        let (_record, proposals, _shas) = match asked {
            KeepDecision::Ask {
                record,
                proposals,
                entry_shas,
            } => (record, proposals, entry_shas),
            other => panic!("short near-duplicate must Ask, got {other:?}"),
        };
        assert_eq!(proposals.len(), 1);
        assert_eq!(proposals[0].0.kind, MatchKind::NeedsCheck);
        assert!(
            proposals[0].0.reason.contains("sick"),
            "reason: {}",
            proposals[0].0.reason
        );
        assert!(
            proposals[0].0.reason.contains("ill"),
            "reason: {}",
            proposals[0].0.reason
        );
        // Identical short texts, same bytes: already kept.
        let again = decide(
            SnippetKind::Text,
            first_text.as_bytes().to_vec(),
            "prayers",
            &store,
            &index,
            &bytes_dir,
        );
        assert!(matches!(again, KeepDecision::AlreadyKept { .. }));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn short_text_index_only_candidate_is_skipped() {
        let dir = test_dir("ob-snippets-short-index-only-test");
        let (_store_path, bytes_dir, index_path) = paths(&dir);
        let store = SnippetStore::default();
        let mut index = FingerprintIndex::default();
        // The index knows a short text that was never
        // kept as a snippet: the index holds no words,
        // so the second step cannot compare it.
        let known = ob_fingerprint::text::fingerprint_text("we pray for the sick");
        index.texts.push(TextEntry::from_engine(
            &sha256_hex(b"we pray for the sick"),
            "prayers",
            &known,
        ));
        index.save(&index_path).expect("save index");
        let decision = decide(
            SnippetKind::Text,
            b"we pray for the ill".to_vec(),
            "prayers-ill",
            &store,
            &index,
            &bytes_dir,
        );
        assert!(
            matches!(decision, KeepDecision::ReadyToStore { .. }),
            "index-only short candidate must be skipped, got {decision:?}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn stale_record_is_listed_never_matches_and_recomputes() {
        let dir = test_dir("ob-snippets-stale-test");
        let (store_path, bytes_dir, index_path) = paths(&dir);
        let mut store = SnippetStore::default();
        let mut index = FingerprintIndex::default();
        let text = "Lord, in your mercy hear our prayer for all who are sick and in need today";
        let first = decide(
            SnippetKind::Text,
            text.as_bytes().to_vec(),
            "prayer-long",
            &store,
            &index,
            &bytes_dir,
        );
        assert!(matches!(first, KeepDecision::ReadyToStore { .. }));
        let content = text.as_bytes().to_vec();
        store_decision(
            &mut store,
            &mut index,
            first,
            &content,
            &bytes_dir,
            &store_path,
            &index_path,
        );
        // Hand-stamp an old fp_version on the record.
        let SnippetFingerprint::Text(entry) = &mut store.snippets[0].fingerprint else {
            panic!("text entry");
        };
        entry.fp_version = "text-fp-v0".to_string();
        store.save(&store_path).expect("save store");
        assert_eq!(stale_snippets(&store).len(), 1);
        assert_eq!(stale_snippets(&store)[0].id, sha256_hex(&content));
        // The stale record never matches for dedupe: the
        // same words retyped decide ReadyToStore, not
        // AlreadyKept.
        let retyped =
            "LORD, in Your Mercy — hear our prayer for all who are sick and in need today!";
        let decision = decide(
            SnippetKind::Text,
            retyped.as_bytes().to_vec(),
            "prayer-long-copy",
            &store,
            &index,
            &bytes_dir,
        );
        assert!(
            matches!(decision, KeepDecision::ReadyToStore { .. }),
            "stale record must never match, got {decision:?}"
        );
        // Recompute from the stored bytes.
        let done = recompute_stale(&mut store, &bytes_dir).expect("recompute works");
        assert_eq!(done.len(), 1);
        assert!(stale_snippets(&store).is_empty());
        assert_eq!(
            store.snippets[0].fingerprint.fp_version(),
            ob_fingerprint::TEXT_FP_VERSION
        );
        store.save(&store_path).expect("save store");
        // Now the retyped text groups again.
        let decision = decide(
            SnippetKind::Text,
            retyped.as_bytes().to_vec(),
            "prayer-long-copy",
            &store,
            &index,
            &bytes_dir,
        );
        assert!(matches!(decision, KeepDecision::AlreadyKept { .. }));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn recompute_stale_missing_bytes_is_a_loud_error() {
        let dir = test_dir("ob-snippets-stale-missing-test");
        let (store_path, bytes_dir, index_path) = paths(&dir);
        let mut store = SnippetStore::default();
        let mut index = FingerprintIndex::default();
        let text = "Lord, in your mercy hear our prayer for all who are sick and in need today";
        let first = decide(
            SnippetKind::Text,
            text.as_bytes().to_vec(),
            "prayer-long",
            &store,
            &index,
            &bytes_dir,
        );
        let content = text.as_bytes().to_vec();
        store_decision(
            &mut store,
            &mut index,
            first,
            &content,
            &bytes_dir,
            &store_path,
            &index_path,
        );
        let SnippetFingerprint::Text(entry) = &mut store.snippets[0].fingerprint else {
            panic!("text entry");
        };
        entry.fp_version = "text-fp-v0".to_string();
        std::fs::remove_file(bytes_dir.join(&store.snippets[0].id)).expect("remove bytes");
        let err = recompute_stale(&mut store, &bytes_dir).expect_err("missing bytes is loud");
        assert!(err.contains("could not read"), "error is {err}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn store_writes_the_index_entry_back() {
        let dir = test_dir("ob-snippets-index-writeback-test");
        let (store_path, bytes_dir, index_path) = paths(&dir);
        let mut store = SnippetStore::default();
        let mut index = FingerprintIndex::default();
        let content = busy_picture();
        let first = decide(
            SnippetKind::Picture,
            content.clone(),
            "busy.png",
            &store,
            &index,
            &bytes_dir,
        );
        let id = store_decision(
            &mut store,
            &mut index,
            first,
            &content,
            &bytes_dir,
            &store_path,
            &index_path,
        );
        let reloaded = FingerprintIndex::load(&index_path).expect("fresh load of the index");
        assert_eq!(reloaded.images.len(), 1);
        assert_eq!(reloaded.images[0].sha256, id);
        assert_eq!(reloaded.images[0].name, "busy.png");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn loud_errors_on_undecodable_content() {
        let dir = test_dir("ob-snippets-loud-errors-test");
        let (_store_path, bytes_dir, _index_path) = paths(&dir);
        let store = SnippetStore::default();
        let index = FingerprintIndex::default();
        let bad_picture = decide_keep(
            &input(SnippetKind::Picture, b"not a picture".to_vec(), "bad.png"),
            &store,
            &index,
            &bytes_dir,
        );
        assert!(bad_picture.is_err(), "bad picture must be loud");
        let bad_music = decide_keep(
            &input(SnippetKind::Music, b"not musicxml".to_vec(), "bad.musicxml"),
            &store,
            &index,
            &bytes_dir,
        );
        assert!(bad_music.is_err(), "bad music must be loud");
        let bad_text = decide_keep(
            &input(SnippetKind::Text, vec![0xff, 0xfe, 0xfd], "bad.txt"),
            &store,
            &index,
            &bytes_dir,
        );
        assert!(bad_text.is_err(), "non-UTF-8 text must be loud");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn keep_proposals_match_the_index_findings() {
        // The keep decision's proposal collectors mirror
        // the index's find_* methods exactly; pin the
        // equivalence on one case per kind. Each case
        // gets a fresh index: the text boilerplate guard
        // reads the whole index, so a one-text index
        // would call every shared shingle boilerplate.
        // Image: a near-duplicate pair.
        let mut index = FingerprintIndex::default();
        let base = busy_picture();
        let near = busy_picture_near();
        let sha_base = sha256_hex(&base);
        let sha_near = sha256_hex(&near);
        let base_fp = {
            let fp = fingerprint_content(SnippetKind::Picture, &base, &sha_base, "busy.png")
                .expect("fingerprint");
            let SnippetFingerprint::Picture(entry) = fp else {
                panic!("picture entry");
            };
            entry
        };
        index.images.push(base_fp.clone());
        let near_fp = {
            let decoded = ob_image::decode(&near).expect("decode");
            fingerprint_image(&decoded)
        };
        let (mine, mine_shas) = image_proposals(&index, &sha_near, &near_fp);
        let theirs = index.find_image_matches(&sha_near, &near_fp);
        assert_eq!(mine, theirs);
        assert_eq!(mine_shas, vec![sha_base.clone()]);
        // Music: the transposed scale proposes.
        let mut index = FingerprintIndex::default();
        let scale = scale_c_xml();
        let transposed = scale_d_xml();
        let sha_scale = sha256_hex(scale.as_bytes());
        let sha_transposed = sha256_hex(transposed.as_bytes());
        let scale_fp = {
            let fp = fingerprint_content(SnippetKind::Music, scale.as_bytes(), &sha_scale, "scale")
                .expect("fingerprint");
            let SnippetFingerprint::Music(entry) = fp else {
                panic!("music entry");
            };
            entry
        };
        index.music.push(scale_fp.clone());
        let transposed_fp = fingerprint_music(transposed.as_bytes()).expect("fingerprint");
        let (mine, mine_shas) = music_proposals(&index, &sha_transposed, &transposed_fp);
        let theirs = index.find_music_matches(&sha_transposed, &transposed_fp);
        assert_eq!(mine, theirs);
        assert_eq!(mine_shas, vec![sha_scale.clone()]);
        assert_eq!(mine.len(), 1);
        assert_eq!(mine[0].0.kind, MatchKind::NeedsCheck);
        // Text, long: a shared run proposes; the
        // short-text candidates stay empty. Four
        // unrelated long texts keep the shared shingles
        // out of the boilerplate guard.
        let mut index = FingerprintIndex::default();
        let long_a =
            "lord in your mercy hear our prayer for all who are sick and in need today alpha beta";
        let long_b =
            "lord in your mercy hear our prayer for all who are sick and in need today one two";
        let sha_a = sha256_hex(long_a.as_bytes());
        let sha_b = sha256_hex(long_b.as_bytes());
        let fp_a = ob_fingerprint::text::fingerprint_text(long_a);
        index
            .texts
            .push(TextEntry::from_engine(&sha_a, "prayer-a", &fp_a));
        for (i, filler) in [
            "zebra quilt xenon yellow violet umbra tango sierra romeo quebec papa oscar november mike lima",
            "anchor bridge canyon delta ember falcon garden harbour island jungle kernel lantern meadow nickel oscar",
            "october pumpkin quarter river summit timber umbrella valley willow xenon yellow zephyr alpine boreal cedar",
            "copper desert emerald forest granite harbour indigo jasper kelp lagoon marble nickel opal pearl quartz",
        ]
        .iter()
        .enumerate()
        {
            let fp = ob_fingerprint::text::fingerprint_text(filler);
            let filler_sha = sha256_hex(filler.as_bytes());
            index
                .texts
                .push(TextEntry::from_engine(&filler_sha, &format!("filler-{i}"), &fp));
        }
        let fp_b = ob_fingerprint::text::fingerprint_text(long_b);
        let ((mine, mine_shas), mine_needs) = text_proposals(&index, &sha_b, &fp_b);
        let (theirs, theirs_needs) = index.find_text_matches(&sha_b, &fp_b);
        assert_eq!(mine, theirs);
        assert_eq!(mine_shas, vec![sha_a.clone()]);
        assert_eq!(mine_needs, theirs_needs);
        assert!(mine_needs.is_empty());
        assert_eq!(mine.len(), 1);
        assert_eq!(mine[0].0.kind, MatchKind::NeedsCheck);
        // Text, short: both sides route to needs.
        let mut index = FingerprintIndex::default();
        let short_a = "we pray for the sick";
        let short_b = "we pray for the ill";
        let sha_sa = sha256_hex(short_a.as_bytes());
        let sha_sb = sha256_hex(short_b.as_bytes());
        let fp_sa = ob_fingerprint::text::fingerprint_text(short_a);
        index
            .texts
            .push(TextEntry::from_engine(&sha_sa, "short-a", &fp_sa));
        let fp_sb = ob_fingerprint::text::fingerprint_text(short_b);
        let ((mine, _mine_shas), mine_needs) = text_proposals(&index, &sha_sb, &fp_sb);
        let (theirs, theirs_needs) = index.find_text_matches(&sha_sb, &fp_sb);
        assert!(mine.is_empty());
        assert_eq!(mine, theirs);
        assert_eq!(mine_needs, theirs_needs);
        assert_eq!(mine_needs.len(), 1);
        assert_eq!(mine_needs[0].0, sha_sa);
    }

    #[test]
    fn path_helpers_end_in_library_home() {
        let store_path = default_store_path();
        let tail = PathBuf::from("OpenBulletin").join("snippets.json");
        assert!(store_path.ends_with(tail), "path is {store_path:?}");
        let bytes_dir = default_bytes_dir();
        let tail = PathBuf::from("OpenBulletin").join("snippets");
        assert!(bytes_dir.ends_with(tail), "path is {bytes_dir:?}");
        assert_eq!(STORE_VERSION, "snippets-v1");
    }
}
