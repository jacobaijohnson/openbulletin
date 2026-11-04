//! Music fingerprints, version `music-fp-v1` (see
//! [`crate::MUSIC_FP_VERSION`]).
//!
//! Three hashes, three different promises — the Tier-2
//! cross-check's binding amendment, and the reason this
//! module never collapses them:
//!
//! * The full spelling-sensitive canonical hash: equality
//!   earns the silent grouping tier ("Same content,
//!   different file"). Only this hash earns it.
//! * The MIDI-pitch hash: equality gets its OWN label
//!   ("Same notes, different spelling") and ASKS. An
//!   enharmonic respelling (C-sharp written as D-flat)
//!   is the same notes, but the file says something
//!   different, so the user decides.
//! * The melody-only hash: equality only PROPOSES
//!   ("Same melody, different harmony"), never groups
//!   silently. A re-harmonised hymn is a candidate, not
//!   a duplicate.
//!
//! Similarity is scored separately for pitch and rhythm:
//! pitch over the melody's semitone-interval sequence
//! (transposition-invariant by construction), rhythm over
//! the melody's duration sequence. Bands (research §4.2,
//! literature starting points): very likely at pitch
//! similarity >= 0.95 with rhythm similarity >= 0.90;
//! possibly at pitch similarity >= 0.85 with incipit
//! containment >= 0.80. Every band is a proposal that
//! asks; the scores are distances with a reason, never a
//! bare percentage.
//!
//! Candidate gating is mandatory, not an optimisation:
//! a query is scored only against index entries that
//! share its incipit prefix (first 6 intervals) or an
//! interval 4-gram, and pass the length filter. The
//! length filter is provably lossless for the possibly
//! band: a pair with pitch similarity >= 0.85 has edit
//! distance <= 0.15 * max length, and the length
//! difference never exceeds the edit distance, so the
//! filter keeps every pair the band could accept. The
//! buckets are a heuristic and can miss a melody
//! rewritten so no 4-interval run survives and the
//! opening changed; that limitation is stated, and the
//! alternative — naive all-pairs edit distance at query
//! time — is never used.
//!
//! The melody is the first part's first voice, top note
//! per onset. For SATB exports that is the soprano line;
//! for a piano score it is the upper staff's top line;
//! for a melody-only file it is the melody. Rests stay
//! in the rhythm sequence and are skipped (not breaking)
//! in the interval chain.
//!
//! Audio fingerprinting is deliberately absent: it
//! identifies recordings, not tunes, and this library
//! holds notation, not audio.

use ob_music::{load_music, parse_canonical, CanonicalScore, NoteEvent};
use sha2::{Digest, Sha256};

/// A score's fingerprint: three hashes, the similarity
/// sequences, and the two human-readable incipits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MusicFingerprint {
    pub fp_version: &'static str,
    /// SHA-256 hex of the spelling-sensitive canonical
    /// serialisation. The only music hash that earns the
    /// silent grouping tier.
    pub canon_hash: String,
    /// SHA-256 hex of the MIDI-pitch serialisation.
    /// Own label, always asks.
    pub midi_hash: String,
    /// SHA-256 hex of the melody serialisation.
    /// Proposes only, never groups silently.
    pub melody_hash: String,
    /// Melody semitone intervals between consecutive
    /// pitched notes (rests skipped, chain unbroken).
    pub intervals: Vec<i16>,
    /// Melody onsets: (duration numerator, denominator,
    /// is rest).
    pub rhythm: Vec<(u32, u32, bool)>,
    /// Scale-degree incipit: first 15 pitched melody
    /// notes as movable-do degrees, joined with no
    /// separator (Hymnary/Oxford practice).
    pub incipit: String,
    /// Plaine & Easie body for the melody incipit (first
    /// 12 onsets): octave marks, rhythm digits, dots,
    /// accidentals, letters, rests, and bar lines. A
    /// display and interchange string, never a match key.
    pub pae: String,
}

/// Fingerprint one music file's bytes: MusicXML text or a
/// .mxl zip (both come through ob-music's loader, so the
/// container handling is shared, not re-implemented).
pub fn fingerprint_music(bytes: &[u8]) -> Result<MusicFingerprint, String> {
    let (xml, _) = load_music(bytes)?;
    let score = parse_canonical(&xml)?;
    Ok(fingerprint_score(&score))
}

/// Fingerprint an already-parsed canonical score.
pub fn fingerprint_score(score: &CanonicalScore) -> MusicFingerprint {
    let melody = melody_stream(score);
    MusicFingerprint {
        fp_version: crate::MUSIC_FP_VERSION,
        canon_hash: sha_hex(&serialise_score(score, true)),
        midi_hash: sha_hex(&serialise_score(score, false)),
        melody_hash: sha_hex(&serialise_events(&melody, true)),
        intervals: melody_intervals(&melody),
        rhythm: melody
            .iter()
            .map(|e| (e.dur_num, e.dur_den, e.rest))
            .collect(),
        incipit: scale_degree_incipit(score, &melody),
        pae: pae_incipit(&melody),
    }
}

/// The melody stream: first part, first voice, top note
/// per onset. A rest forms its own onset; chord notes
/// join the open onset and only the highest pitch
/// survives.
pub fn melody_stream(score: &CanonicalScore) -> Vec<NoteEvent> {
    let Some(first) = score.parts.first() else {
        return Vec::new();
    };
    let mut out: Vec<NoteEvent> = Vec::new();
    let mut open: Option<usize> = None;
    for ev in &first.events {
        // Melody events never carry a chord flag: whether
        // the top note arrived alone or as a chord tone
        // is not a melodic difference.
        let mut e = ev.clone();
        e.chord = false;
        if e.rest {
            out.push(e);
            open = None;
            continue;
        }
        if !ev.chord || open.is_none() {
            out.push(e);
            open = Some(out.len() - 1);
        } else if let Some(i) = open {
            if let (Some(top), Some(new)) = (out[i].midi, e.midi) {
                if new > top {
                    out[i] = e;
                }
            }
        }
    }
    out
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

fn alter_marks(alter: i8) -> &'static str {
    match alter {
        2 => "##",
        1 => "#",
        -1 => "b",
        -2 => "bb",
        _ => "",
    }
}

fn serialise_event_spelling(ev: &NoteEvent) -> String {
    if ev.rest {
        let base = format!("R:{}/{}", ev.dur_num, ev.dur_den);
        return base;
    }
    let chord = if ev.chord { "c" } else { "" };
    format!(
        "{}{}{}:{}/{}{}",
        ev.step,
        alter_marks(ev.alter),
        ev.octave,
        ev.dur_num,
        ev.dur_den,
        chord
    )
}

fn serialise_event_midi(ev: &NoteEvent) -> String {
    if ev.rest {
        return format!("R:{}/{}", ev.dur_num, ev.dur_den);
    }
    let chord = if ev.chord { "c" } else { "" };
    format!(
        "{}:{}/{}{}",
        ev.midi.unwrap_or(0),
        ev.dur_num,
        ev.dur_den,
        chord
    )
}

fn serialise_events(events: &[NoteEvent], spelling: bool) -> String {
    events
        .iter()
        .map(|e| {
            if spelling {
                serialise_event_spelling(e)
            } else {
                serialise_event_midi(e)
            }
        })
        .collect::<Vec<_>>()
        .join(";")
}

fn serialise_score(score: &CanonicalScore, spelling: bool) -> String {
    score
        .parts
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let body = serialise_events(&p.events, spelling);
            format!("P{i}V{}:{body}", p.voice)
        })
        .collect::<Vec<_>>()
        .join("|")
}

fn melody_intervals(melody: &[NoteEvent]) -> Vec<i16> {
    let pitched: Vec<i16> = melody.iter().filter_map(|e| e.midi).collect();
    pitched.windows(2).map(|w| w[1] - w[0]).collect()
}

/// The tonic pitch class from the opening key: fifths
/// around the circle (C=0, G=7, F=5, ...), or the first
/// melody note when the file states no key at all. A
/// minor key's tonic is the relative minor: three
/// semitones below its major namesake (fifths 0 minor
/// is A, not C).
fn tonic_pc(score: &CanonicalScore, melody: &[NoteEvent]) -> u8 {
    if score.fifths == 0 && score.mode.is_empty() {
        if let Some(m) = melody.iter().find_map(|e| e.midi) {
            return m.rem_euclid(12) as u8;
        }
    }
    let major = (score.fifths as i16 * 7).rem_euclid(12);
    if score.mode == "minor" {
        (major - 3).rem_euclid(12) as u8
    } else {
        major as u8
    }
}

fn degree_major(offset: u8) -> &'static str {
    match offset {
        0 => "1",
        1 => "#1",
        2 => "2",
        3 => "b3",
        4 => "3",
        5 => "4",
        6 => "#4",
        7 => "5",
        8 => "b6",
        9 => "6",
        10 => "b7",
        _ => "7",
    }
}

fn degree_minor(offset: u8) -> &'static str {
    match offset {
        0 => "1",
        1 => "#1",
        2 => "2",
        3 => "3",
        4 => "#3",
        5 => "4",
        6 => "#4",
        7 => "5",
        8 => "6",
        9 => "#6",
        10 => "7",
        _ => "#7",
    }
}

/// First 15 pitched melody notes as movable-do scale
/// degrees. Minor tunes are re-based so their tonic
/// scale also starts at 1 (Hymnary practice).
pub fn scale_degree_incipit(score: &CanonicalScore, melody: &[NoteEvent]) -> String {
    let tonic = tonic_pc(score, melody);
    let minor = score.mode == "minor";
    let mut out = String::new();
    for ev in melody.iter().filter(|e| !e.rest).take(15) {
        if let Some(m) = ev.midi {
            let offset = m.rem_euclid(12) as u8;
            let rel = (offset + 12 - tonic) % 12;
            let degree = if minor {
                degree_minor(rel)
            } else {
                degree_major(rel)
            };
            out.push_str(degree);
        }
    }
    out
}

fn pae_rhythm_digit(ev: &NoteEvent) -> Option<&'static str> {
    match ev.note_type.as_str() {
        "long" => Some("0"),
        "breve" => Some("9"),
        "whole" => Some("1"),
        "half" => Some("2"),
        "quarter" => Some("4"),
        "eighth" => Some("8"),
        "16th" => Some("6"),
        "32nd" => Some("3"),
        "64th" => Some("5"),
        "128th" => Some("7"),
        _ => match (ev.dur_num, ev.dur_den) {
            (4, 1) => Some("1"),
            (2, 1) => Some("2"),
            (1, 1) => Some("4"),
            (1, 2) => Some("8"),
            (1, 4) => Some("6"),
            (1, 8) => Some("3"),
            (1, 16) => Some("5"),
            (1, 32) => Some("7"),
            _ => None,
        },
    }
}

fn pae_octave_mark(octave: i8) -> String {
    if octave >= 4 {
        "'".repeat((octave - 3) as usize)
    } else {
        ",".repeat((4 - octave) as usize)
    }
}

/// Plaine & Easie body for the first 12 melody onsets
/// (IAML §4): octave marks (apostrophes at and above
/// middle C, commas below), rhythm digits (stated when
/// they change), dots, accidentals (x sharp, b flat, n
/// natural when stated), uppercase letters, rests as
/// rhythm + '-', and '/' bar lines. Clef, key, and time
/// headers are not emitted: this is the note body for
/// display and interchange, and consumers prepend
/// headers.
pub fn pae_incipit(melody: &[NoteEvent]) -> String {
    let mut out = String::new();
    let mut last_octave: Option<i8> = None;
    let mut last_rhythm: Option<String> = None;
    let mut last_measure: Option<u32> = None;
    for ev in melody.iter().take(12) {
        if let Some(m) = last_measure {
            if ev.measure != m {
                out.push('/');
            }
        }
        last_measure = Some(ev.measure);
        let digit = pae_rhythm_digit(ev).unwrap_or("").to_string();
        let dots = ".".repeat(ev.dots as usize);
        let rhythm = format!("{digit}{dots}");
        if !ev.rest {
            let octave = pae_octave_mark(ev.octave);
            if last_octave != Some(ev.octave) {
                out.push_str(&octave);
                last_octave = Some(ev.octave);
            }
        }
        if last_rhythm.as_deref() != Some(rhythm.as_str()) {
            out.push_str(&rhythm);
            last_rhythm = Some(rhythm);
        }
        if ev.rest {
            out.push('-');
        } else {
            let acc = match ev.alter {
                2 => "xx",
                1 => "x",
                -1 => "b",
                -2 => "bb",
                0 if ev.accidental == "natural" => "n",
                _ => "",
            };
            out.push_str(acc);
            out.push(ev.step);
        }
    }
    out
}

/// Levenshtein edit similarity over token slices: 1
/// minus distance over the longer length. Two empty
/// sequences score 1.0; an empty against a non-empty
/// scores 0.0.
pub fn edit_similarity<T: PartialEq>(a: &[T], b: &[T]) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0usize; b.len() + 1];
    for (i, x) in a.iter().enumerate() {
        cur[0] = i + 1;
        for (j, y) in b.iter().enumerate() {
            let cost = if x == y { 0 } else { 1 };
            cur[j + 1] = (prev[j + 1] + 1).min(cur[j] + 1).min(prev[j] + cost);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    let dist = prev[b.len()];
    let max = a.len().max(b.len());
    1.0 - dist as f64 / max as f64
}

/// Multiset containment of two incipits: the shared
/// degree count over the shorter incipit. Two empty
/// incipits score 1.0 (both melodies empty); an empty
/// against a non-empty scores 0.0.
pub fn incipit_containment(a: &str, b: &str) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let mut counts_b: std::collections::HashMap<char, usize> = std::collections::HashMap::new();
    for c in b.chars() {
        *counts_b.entry(c).or_insert(0) += 1;
    }
    let mut shared = 0usize;
    for c in a.chars() {
        if let Some(n) = counts_b.get_mut(&c) {
            if *n > 0 {
                *n -= 1;
                shared += 1;
            }
        }
    }
    let min = a.chars().count().min(b.chars().count());
    shared as f64 / min as f64
}

/// The similarity scores between two fingerprints of the
/// same version, before any band is applied.
#[derive(Debug, Clone, Copy)]
pub struct MusicScores {
    pub pitch_sim: f64,
    pub rhythm_sim: f64,
    pub incipit_containment: f64,
}

pub fn music_scores(a: &MusicFingerprint, b: &MusicFingerprint) -> MusicScores {
    MusicScores {
        pitch_sim: edit_similarity(&a.intervals, &b.intervals),
        rhythm_sim: edit_similarity(&a.rhythm, &b.rhythm),
        incipit_containment: incipit_containment(&a.incipit, &b.incipit),
    }
}

/// What two fingerprints mean together. `SameContent`
/// is the only music decision that may group silently;
/// every other decision asks the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MusicDecision {
    /// Spelling-sensitive canonical hashes equal.
    SameContent,
    /// MIDI-pitch hashes equal: same notes, different
    /// spelling. Own label, always asks.
    SameNotesDifferentSpelling,
    /// Melody hashes equal: same melody, different
    /// harmony. Proposes only.
    ProbablyMelody,
    /// Similarity band reached.
    ProbablyBand,
    /// No tier, no band.
    NoMatch,
}

/// Decide two fingerprints of the same version.
/// Different versions never match: incomparable.
/// Similarity needs a melody on both sides: two empty
/// melodies never match on similarity (they can still
/// match on canonical hash when truly identical).
pub fn music_decision(a: &MusicFingerprint, b: &MusicFingerprint) -> MusicDecision {
    if a.fp_version != b.fp_version {
        return MusicDecision::NoMatch;
    }
    if a.canon_hash == b.canon_hash {
        return MusicDecision::SameContent;
    }
    if a.midi_hash == b.midi_hash {
        return MusicDecision::SameNotesDifferentSpelling;
    }
    if a.melody_hash == b.melody_hash {
        return MusicDecision::ProbablyMelody;
    }
    if a.intervals.is_empty() || b.intervals.is_empty() {
        return MusicDecision::NoMatch;
    }
    let s = music_scores(a, b);
    if s.pitch_sim >= 0.95 && s.rhythm_sim >= 0.90 {
        return MusicDecision::ProbablyBand;
    }
    if s.pitch_sim >= 0.85 && s.incipit_containment >= 0.80 {
        return MusicDecision::ProbablyBand;
    }
    MusicDecision::NoMatch
}

/// The candidate gate: an index over fingerprints that
/// answers "which entries could this query match?" by
/// incipit-prefix and interval 4-gram buckets plus the
/// provably lossless length filter. Scoring runs only
/// over the returned candidates, never over the whole
/// library.
pub struct MusicCandidateIndex {
    items: Vec<MusicFingerprint>,
    by_prefix: std::collections::HashMap<Vec<i16>, Vec<usize>>,
    by_gram: std::collections::HashMap<[i16; 4], Vec<usize>>,
}

impl MusicCandidateIndex {
    pub fn new(items: Vec<MusicFingerprint>) -> MusicCandidateIndex {
        let mut by_prefix = std::collections::HashMap::new();
        let mut by_gram = std::collections::HashMap::new();
        for (i, fp) in items.iter().enumerate() {
            let prefix: Vec<i16> = fp.intervals.iter().take(6).copied().collect();
            by_prefix.entry(prefix).or_insert_with(Vec::new).push(i);
            for gram in fp.intervals.windows(4) {
                let key = [gram[0], gram[1], gram[2], gram[3]];
                by_gram.entry(key).or_insert_with(Vec::new).push(i);
            }
        }
        MusicCandidateIndex {
            items,
            by_prefix,
            by_gram,
        }
    }

    /// Candidate indices for a query fingerprint. The
    /// length filter keeps only entries whose interval
    /// length could reach the possibly band: |la - lb|
    /// <= 0.15 * max(la, lb), in integer math
    /// (20 * diff <= 3 * max). Any pair the band could
    /// accept passes this filter; the buckets then
    /// narrow further.
    pub fn candidates(&self, query: &MusicFingerprint) -> Vec<usize> {
        let mut out: Vec<usize> = Vec::new();
        let mut seen = vec![false; self.items.len()];
        let consider = |out: &mut Vec<usize>, seen: &mut [bool], i: usize| {
            if seen[i] {
                return;
            }
            let other = &self.items[i];
            let la = query.intervals.len();
            let lb = other.intervals.len();
            let max = la.max(lb);
            let diff = la.abs_diff(lb);
            if max > 0 && 20 * diff <= 3 * max {
                seen[i] = true;
                out.push(i);
            }
        };
        let prefix: Vec<i16> = query.intervals.iter().take(6).copied().collect();
        if let Some(ids) = self.by_prefix.get(&prefix) {
            for id in ids.clone() {
                consider(&mut out, &mut seen, id);
            }
        }
        for gram in query.intervals.windows(4) {
            let key = [gram[0], gram[1], gram[2], gram[3]];
            if let Some(ids) = self.by_gram.get(&key) {
                for id in ids.clone() {
                    consider(&mut out, &mut seen, id);
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shell(notes: &str, key: &str) -> String {
        format!(
            r#"<?xml version="1.0"?><score-partwise>
            <part-list><score-part id="P1"><part-name>V</part-name></score-part></part-list>
            <part id="P1"><measure><attributes><divisions>4</divisions>{key}</attributes>
            {notes}
            </measure></part></score-partwise>"#
        )
    }

    fn note(step: &str, alter: &str, octave: u8, dur: u32, ntype: &str) -> String {
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

    fn fp(xml: &str) -> MusicFingerprint {
        let score = parse_canonical(xml).unwrap();
        fingerprint_score(&score)
    }

    fn scale_c() -> String {
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
            notes.push_str(&note(s, "", o, 4, "quarter"));
        }
        shell(&notes, "<key><fifths>0</fifths><mode>major</mode></key>")
    }

    #[test]
    fn divisions_rescaling_keeps_the_canonical_hash() {
        let a = fp(&scale_c());
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
        let xml = format!(
            r#"<?xml version="1.0"?><score-partwise>
            <part-list><score-part id="P1"><part-name>V</part-name></score-part></part-list>
            <part id="P1"><measure><attributes><divisions>480</divisions></attributes>
            {notes}
            </measure></part></score-partwise>"#
        );
        let b = fp(&xml);
        assert_eq!(a.canon_hash, b.canon_hash);
        assert_eq!(music_decision(&a, &b), MusicDecision::SameContent);
    }

    #[test]
    fn transposition_keeps_intervals_but_not_hashes() {
        let a = fp(&scale_c());
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
            notes.push_str(&note(s, al, o, 4, "quarter"));
        }
        let b = fp(&shell(
            &notes,
            "<key><fifths>2</fifths><mode>major</mode></key>",
        ));
        assert_ne!(a.canon_hash, b.canon_hash);
        assert_ne!(a.midi_hash, b.midi_hash);
        assert_eq!(a.intervals, b.intervals);
        assert_eq!(a.rhythm, b.rhythm);
        assert_eq!(music_decision(&a, &b), MusicDecision::ProbablyBand);
    }

    #[test]
    fn enharmonic_respelling_gets_its_own_label() {
        let a = fp(&shell(&note("C", "1", 4, 4, "quarter"), ""));
        let b = fp(&shell(&note("D", "-1", 4, 4, "quarter"), ""));
        assert_ne!(a.canon_hash, b.canon_hash);
        assert_eq!(a.midi_hash, b.midi_hash);
        assert_eq!(
            music_decision(&a, &b),
            MusicDecision::SameNotesDifferentSpelling
        );
    }

    #[test]
    fn melody_hash_proposes_but_never_groups() {
        // Same soprano line, one with an added alto part.
        let a = fp(&scale_c());
        let xml = format!(
            r#"<?xml version="1.0"?><score-partwise>
            <part-list>
              <score-part id="P1"><part-name>Soprano</part-name></score-part>
              <score-part id="P2"><part-name>Alto</part-name></score-part>
            </part-list>
            <part id="P1"><measure><attributes><divisions>4</divisions></attributes>
              {}
            </measure></part>
            <part id="P2"><measure><attributes><divisions>4</divisions></attributes>
              {}
            </measure></part></score-partwise>"#,
            [
                ("C", 4),
                ("D", 4),
                ("E", 4),
                ("F", 4),
                ("G", 4),
                ("A", 4),
                ("B", 4)
            ]
            .iter()
            .map(|(s, o)| note(s, "", *o, 4, "quarter"))
            .collect::<Vec<_>>()
            .join(""),
            note("C", "", 3, 28, "quarter")
        );
        let b = fp(&xml);
        assert_ne!(a.canon_hash, b.canon_hash);
        assert_eq!(a.melody_hash, b.melody_hash);
        assert_eq!(music_decision(&a, &b), MusicDecision::ProbablyMelody);
    }

    #[test]
    fn incipit_degrees_major_and_minor() {
        let major = fp(&scale_c());
        assert_eq!(major.incipit, "1234567");
        let mut notes = String::new();
        for (s, o) in [("A", 3), ("B", 3), ("C", 4)] {
            notes.push_str(&note(s, "", o, 4, "quarter"));
        }
        let minor = fp(&shell(
            &notes,
            "<key><fifths>0</fifths><mode>minor</mode></key>",
        ));
        assert_eq!(minor.incipit, "123");
    }

    #[test]
    fn pae_body_states_octave_and_rhythm_on_change() {
        let mut notes = String::new();
        notes.push_str(&note("C", "", 4, 4, "quarter"));
        notes.push_str(&note("D", "", 4, 4, "quarter"));
        notes.push_str(&note("E", "", 4, 8, "half"));
        notes.push_str("<note><rest/><duration>4</duration><type>quarter</type></note>");
        notes.push_str(&note("F", "1", 4, 4, "quarter"));
        let f = fp(&shell(&notes, ""));
        // The rest states rhythm 4, so the following quarter
        // correctly omits it: rhythm digits are stated when
        // they change, and a rest states one too.
        assert_eq!(f.pae, "'4CD2E4-xF");
    }

    #[test]
    fn edit_similarity_bounds() {
        assert_eq!(edit_similarity(&[1, 2, 3], &[1, 2, 3]), 1.0);
        let sim = edit_similarity(&[1, 2, 3], &[1, 9, 3]);
        assert!((sim - 2.0 / 3.0).abs() < 1e-9, "one change in three: {sim}");
        assert_eq!(edit_similarity::<i16>(&[], &[]), 1.0);
        assert_eq!(edit_similarity(&[1], &[]), 0.0);
    }

    #[test]
    fn candidate_gate_finds_the_transposition_and_skips_the_stranger() {
        let a = fp(&scale_c());
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
            notes.push_str(&note(s, al, o, 4, "quarter"));
        }
        let transposed = fp(&shell(&notes, ""));
        // An unrelated short figure: few intervals, far length.
        let stranger = fp(&shell(
            &format!(
                "{}{}",
                note("C", "", 5, 4, "quarter"),
                note("C", "", 5, 4, "quarter")
            ),
            "",
        ));
        let index = MusicCandidateIndex::new(vec![a.clone(), stranger.clone()]);
        let found = index.candidates(&transposed);
        assert!(found.contains(&0), "transposed tune must be a candidate");
        assert!(
            !found.contains(&1),
            "unrelated figure must not be a candidate"
        );
        // The length filter alone never excludes a possibly-band
        // pair: a 20-interval tune against a 17-interval tune
        // passes (20*3 <= 3*20), a 20 against a 10 does not.
        let long = MusicFingerprint {
            intervals: vec![2; 20],
            ..a.clone()
        };
        let near = MusicFingerprint {
            intervals: vec![2; 17],
            ..a.clone()
        };
        let far = MusicFingerprint {
            intervals: vec![2; 10],
            ..a.clone()
        };
        let index2 = MusicCandidateIndex::new(vec![near, far]);
        let found2 = index2.candidates(&long);
        assert!(found2.contains(&0));
        assert!(!found2.contains(&1));
    }
}
