//! The canonical note-event stream: what a MusicXML file
//! SOUNDS like, with everything re-export changes stripped
//! away.
//!
//! Re-export from different notation software rewrites
//! element order, whitespace, encoding tags, layout,
//! fonts, stem directions, beaming, and the `divisions`
//! scaling — while intending to preserve pitch and
//! relative duration. Any byte or raw-XML hash breaks.
//! This walk therefore parses, then canonicalises: one
//! event stream per (part, voice), in document order,
//! where each event is a pitch (spelling kept) or a rest
//! with a duration as a reduced fraction of a quarter
//! note. From that one stream, ob-fingerprint derives the
//! canonical hashes, the interval and rhythm sequences,
//! the scale-degree incipit, and the Plaine & Easie body.
//!
//! Rules, each pinned by a test:
//!
//! * Durations are `duration / divisions`, reduced by
//!   gcd. MusicXML defines `<duration>` as the SOUNDING
//!   length in divisions units, so dots and tuplet
//!   scaling are already inside it: applying them again
//!   would double-count. Dots are still captured, for
//!   the Plaine & Easie dots only; time-modification is
//!   not needed at all.
//! * Ties merge: a note with a tie-stop and the same
//!   pitch as an open tie-start adds its duration to the
//!   open event instead of starting a new one. A dotted
//!   quarter and a quarter tied to an eighth therefore
//!   canonicalise identically, which is the point.
//! * Chords group: a note carrying `<chord/>` sounds with
//!   the previous event (same onset); each event keeps
//!   its own duration and a `chord` flag, so the grouping
//!   is recoverable and deterministic.
//! * Grace notes are skipped: they are ornaments, and
//!   re-notation varies them freely. They change nothing
//!   in the stream.
//! * `<tie>` (the sound element) drives merging;
//!   `<tied>` (its notation mirror) is ignored, so a
//!   tie is never counted twice.
//! * Only `<tie>`-driven, `<duration>`-carrying,
//!   non-grace notes enter the stream. A pitched note
//!   without a duration, or a duration without a
//!   preceding `<divisions>`, fails the whole parse
//!   loudly: a wrong hash is worse than no hash, and the
//!   import itself still uses the full parser.
//! * `<forward>` and `<backup>` durations are ignored:
//!   they move the cursor between voices, and voices get
//!   their own streams, so cursor positions are not
//!   needed for ordering.
//! * The tonic for the scale-degree incipit comes from
//!   the FIRST `<key>` in the document; later key changes
//!   do not re-base it.
//!
//! This reader never changes a note. It does not
//! transpose, fix, or author; it reads and canonicalises,
//! which is the whole of OpenBulletin's music job
//! (Jacob's ruling).

use quick_xml::events::Event;
use quick_xml::Reader;

use crate::parse::MAX_XML_DEPTH;

/// One canonical note event: a pitch (spelling kept) or a
/// rest, with a sounding duration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteEvent {
    /// MIDI number, `None` for rests.
    pub midi: Option<i16>,
    /// Step letter 'A'..'G'; 'R' for rests.
    pub step: char,
    /// Semitone offset from the step (1 = sharp).
    pub alter: i8,
    /// Octave number; 0 for rests.
    pub octave: i8,
    /// Sounding duration as a reduced fraction of a
    /// quarter note.
    pub dur_num: u32,
    pub dur_den: u32,
    /// True for rests.
    pub rest: bool,
    /// True when this note sounds with the previous event
    /// (a `<chord/>` note).
    pub chord: bool,
    /// Measure index within the part, from 1. Carried for
    /// the Plaine & Easie bar lines; never hashed.
    pub measure: u32,
    /// Written note type ("quarter", "eighth", ...), "" if
    /// the file states none. Carried for the Plaine &
    /// Easie rhythm digits; never hashed.
    pub note_type: String,
    /// Dot count. Carried for the Plaine & Easie dots;
    /// the sounding effect is already in the duration.
    pub dots: u8,
    /// Stated accidental ("natural", "sharp", ...), "" if
    /// none. Carried for the Plaine & Easie natural; the
    /// sounding effect is already in `alter`.
    pub accidental: String,
}

/// One voice's stream: all events of one (part, voice)
/// pair, in document order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartStream {
    pub part_id: String,
    pub part_name: String,
    pub voice: String,
    pub events: Vec<NoteEvent>,
}

/// A parsed score: its voice streams plus the opening key
/// (the incipit's tonic).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalScore {
    pub parts: Vec<PartStream>,
    /// Fifths of the first `<key>` in the document, 0
    /// when the file states no key.
    pub fifths: i8,
    /// Mode of the first `<key>` ("major", "minor", ...),
    /// "" when the file states none.
    pub mode: String,
}

/// A spelled pitch, the key of an open tie.
type PitchKey = (i16, char, i8, i8);

/// Open ties per voice stream: stream key -> (pitch, event index).
type PendingTies = Vec<(String, Vec<(PitchKey, usize)>)>;

fn local(name: &[u8]) -> String {
    let s = String::from_utf8_lossy(name).into_owned();
    s.rsplit(':').next().unwrap_or("").to_string()
}

fn attr(e: &quick_xml::events::BytesStart, key: &str) -> Option<String> {
    e.attributes().find_map(|a| {
        a.ok().and_then(|a| {
            if a.key.as_ref() == key.as_bytes() {
                a.unescape_value().ok().map(|v| v.into_owned())
            } else {
                None
            }
        })
    })
}

fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a.max(1)
}

fn reduce(num: u64, den: u64) -> Result<(u32, u32), String> {
    let g = gcd(num, den);
    let num = num / g;
    let den = den / g;
    if num > u32::MAX as u64 || den > u32::MAX as u64 {
        return Err("that music file has a note duration too large to read".to_string());
    }
    Ok((num as u32, den as u32))
}

/// The MIDI number of a spelled pitch.
pub fn midi_of(step: char, alter: i8, octave: i8) -> i16 {
    let base = match step {
        'C' => 0,
        'D' => 2,
        'E' => 4,
        'F' => 5,
        'G' => 7,
        'A' => 9,
        'B' => 11,
        _ => 0,
    };
    (octave as i16 + 1) * 12 + base + alter as i16
}

/// Parse score-partwise MusicXML text into canonical
/// streams. Accepts the same files `parse_musicxml`
/// accepts; rejects timewise files with the same plain
/// words.
pub fn parse_canonical(xml: &str) -> Result<CanonicalScore, String> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);
    let mut stack: Vec<String> = Vec::new();
    let mut buf: Vec<u8> = Vec::new();
    let mut capture: Option<String> = None;
    let mut text = String::new();
    let mut saw_score = false;

    let mut part_names: Vec<(String, String)> = Vec::new();
    let mut score_part_id = String::new();
    let mut parts: Vec<PartStream> = Vec::new();
    let mut current_part = String::new();
    let mut in_part = false;
    let mut measure: u32 = 0;
    let mut divisions: Option<u32> = None;
    let mut key_count: u32 = 0;
    let mut fifths: i8 = 0;
    let mut mode = String::new();

    // Open-note state, reset on every <note>.
    let mut in_note = false;
    let mut is_rest = false;
    let mut is_grace = false;
    let mut is_chord = false;
    let mut dots: u8 = 0;
    let mut tie_start = false;
    let mut tie_stop = false;
    let mut step: Option<char> = None;
    let mut alter: i8 = 0;
    let mut octave: Option<i8> = None;
    let mut duration: Option<u32> = None;
    let mut voice = String::new();
    let mut note_type = String::new();
    let mut accidental = String::new();
    // Open ties per voice stream, keyed by spelled pitch:
    // (midi, step, alter, octave) -> event index in that
    // stream. Chords tie note by note, so one slot per
    // voice would lose all but one chord tone.
    let mut pending: PendingTies = Vec::new();

    let stream_index = |parts: &mut Vec<PartStream>, part_id: &str, voice: &str| -> usize {
        if let Some(i) = parts
            .iter()
            .position(|p| p.part_id == part_id && p.voice == voice)
        {
            return i;
        }
        parts.push(PartStream {
            part_id: part_id.to_string(),
            part_name: String::new(),
            voice: voice.to_string(),
            events: Vec::new(),
        });
        parts.len() - 1
    };

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = local(e.name().as_ref());
                match name.as_str() {
                    "score-partwise" => saw_score = true,
                    "score-timewise" => {
                        return Err("that is a timewise MusicXML file. OpenBulletin \
                                    reads the partwise kind (the kind MuseScore, \
                                    Dorico, Sibelius, and Finale export); export it \
                                    again as partwise and it will come in."
                            .to_string());
                    }
                    "score-part" => {
                        score_part_id = attr(&e, "id").unwrap_or_default();
                    }
                    "part" => {
                        in_part = true;
                        current_part = attr(&e, "id").unwrap_or_default();
                        measure = 0;
                        divisions = None;
                    }
                    "measure" if in_part => measure += 1,
                    "note" => {
                        in_note = true;
                        is_rest = false;
                        is_grace = false;
                        is_chord = false;
                        dots = 0;
                        tie_start = false;
                        tie_stop = false;
                        step = None;
                        alter = 0;
                        octave = None;
                        duration = None;
                        voice = String::new();
                        note_type = String::new();
                        accidental = String::new();
                    }
                    "rest" if in_note => is_rest = true,
                    "grace" if in_note => is_grace = true,
                    "chord" if in_note => is_chord = true,
                    "dot" if in_note => dots = dots.saturating_add(1),
                    "tie" if in_note => match attr(&e, "type").as_deref() {
                        Some("start") => tie_start = true,
                        Some("stop") => tie_stop = true,
                        _ => {}
                    },
                    "part-name" if stack.iter().any(|s| s == "score-part") => {
                        capture = Some("part-name".into());
                    }
                    "key" => key_count += 1,
                    "divisions" | "fifths" | "mode" => {
                        capture = Some(name.clone());
                    }
                    "step" | "alter" | "octave" | "duration" | "voice" | "type" | "accidental"
                        if in_note =>
                    {
                        capture = Some(name.clone());
                    }
                    _ => {}
                }
                if capture.is_some() {
                    text.clear();
                }
                stack.push(name);
                if stack.len() > MAX_XML_DEPTH {
                    return Err(
                        "that music file's XML is nested too deeply to read safely".to_string()
                    );
                }
            }
            Ok(Event::Empty(e)) => {
                let name = local(e.name().as_ref());
                if in_note {
                    match name.as_str() {
                        "rest" => is_rest = true,
                        "grace" => is_grace = true,
                        "chord" => is_chord = true,
                        "dot" => dots = dots.saturating_add(1),
                        "tie" => match attr(&e, "type").as_deref() {
                            Some("start") => tie_start = true,
                            Some("stop") => tie_stop = true,
                            _ => {}
                        },
                        _ => {}
                    }
                }
            }
            Ok(Event::Text(e)) => {
                if capture.is_some() {
                    if let Ok(t) = e.unescape() {
                        text.push_str(&t);
                    }
                }
            }
            Ok(Event::End(e)) => {
                let name = local(e.name().as_ref());
                match name.as_str() {
                    "part" => in_part = false,
                    "part-name" if capture.as_deref() == Some("part-name") => {
                        let t = text.trim().to_string();
                        if !t.is_empty() {
                            part_names.push((score_part_id.clone(), t));
                        }
                        capture = None;
                    }
                    "divisions" if capture.as_deref() == Some("divisions") => {
                        if let Ok(v) = text.trim().parse::<u32>() {
                            if v > 0 {
                                divisions = Some(v);
                            }
                        }
                        capture = None;
                    }
                    "fifths" if capture.as_deref() == Some("fifths") => {
                        if key_count == 1 {
                            if let Ok(v) = text.trim().parse::<i8>() {
                                fifths = v;
                            }
                        }
                        capture = None;
                    }
                    "mode" if capture.as_deref() == Some("mode") => {
                        if key_count == 1 {
                            mode = text.trim().to_string();
                        }
                        capture = None;
                    }
                    "step" | "alter" | "octave" | "duration" | "voice" | "type" | "accidental" => {
                        let want = capture.clone().unwrap_or_default();
                        if want == name {
                            let t = text.trim().to_string();
                            match name.as_str() {
                                "step" => {
                                    step = t.chars().next().filter(|c| ('A'..='G').contains(c));
                                }
                                "alter" => {
                                    if let Ok(v) = t.parse::<f32>() {
                                        alter = v.round() as i8;
                                    }
                                }
                                "octave" => octave = t.parse::<i8>().ok(),
                                "duration" => duration = t.parse::<u32>().ok(),
                                "voice" => voice = t,
                                "type" => note_type = t,
                                "accidental" => accidental = t,
                                _ => {}
                            }
                            capture = None;
                        }
                    }
                    "note" => {
                        in_note = false;
                        if is_grace {
                            // Ornaments never enter the stream.
                        } else if is_rest {
                            // A rest breaks any open tie in
                            // this voice: a tie cannot sound
                            // across a rest, so a later
                            // tie-stop with the same pitch
                            // is dangling, not a merge.
                            let stream_key = format!("{}\u{1f}{}", current_part, voice);
                            pending.retain(|(k, _)| *k != stream_key);
                            let div = divisions.ok_or_else(|| {
                                "that music file states a rest before any divisions, \
                                 so its durations cannot be read"
                                    .to_string()
                            })?;
                            let dur = duration.ok_or_else(|| {
                                "that music file has a rest with no duration, \
                                 so its rhythm cannot be read"
                                    .to_string()
                            })?;
                            let (num, den) = reduce(dur as u64, div as u64)?;
                            let idx = stream_index(&mut parts, &current_part, &voice);
                            parts[idx].events.push(NoteEvent {
                                midi: None,
                                step: 'R',
                                alter: 0,
                                octave: 0,
                                dur_num: num,
                                dur_den: den,
                                rest: true,
                                chord: is_chord,
                                measure,
                                note_type: note_type.clone(),
                                dots,
                                accidental: accidental.clone(),
                            });
                        } else {
                            let st = step.ok_or_else(|| {
                                "that music file has a note with no pitch, \
                                 so its notes cannot be read"
                                    .to_string()
                            })?;
                            let oc = octave.ok_or_else(|| {
                                "that music file has a note with no octave, \
                                 so its notes cannot be read"
                                    .to_string()
                            })?;
                            let div = divisions.ok_or_else(|| {
                                "that music file states a note before any divisions, \
                                 so its durations cannot be read"
                                    .to_string()
                            })?;
                            let dur = duration.ok_or_else(|| {
                                "that music file has a note with no duration, \
                                 so its rhythm cannot be read"
                                    .to_string()
                            })?;
                            // Sounding duration, already
                            // dot- and tuplet-scaled by
                            // the exporter: reduce only.
                            let (num, den) = reduce(dur as u64, div as u64)?;
                            let midi = midi_of(st, alter, oc);
                            let key = (midi, st, alter, oc);
                            let idx = stream_index(&mut parts, &current_part, &voice);
                            let stream_key = format!("{}\u{1f}{}", current_part, voice);
                            let pslot = pending
                                .iter()
                                .position(|(k, _)| *k == stream_key)
                                .unwrap_or_else(|| {
                                    pending.push((stream_key.clone(), Vec::new()));
                                    pending.len() - 1
                                });
                            if tie_stop {
                                if let Some(pos) =
                                    pending[pslot].1.iter().position(|(k, _)| *k == key)
                                {
                                    let ev_idx = pending[pslot].1[pos].1;
                                    let ev = &mut parts[idx].events[ev_idx];
                                    let (n, d) = reduce(
                                        ev.dur_num as u64 * den as u64
                                            + num as u64 * ev.dur_den as u64,
                                        ev.dur_den as u64 * den as u64,
                                    )?;
                                    ev.dur_num = n;
                                    ev.dur_den = d;
                                    if !tie_start {
                                        pending[pslot].1.remove(pos);
                                    }
                                    // A merged tie continuation adds no
                                    // new event, whatever its chord flag.
                                } else {
                                    // A dangling tie-stop names no open
                                    // tie: the note still sounds, so it
                                    // enters the stream as an ordinary
                                    // note rather than failing the file.
                                    let ev_idx = parts[idx].events.len();
                                    parts[idx].events.push(NoteEvent {
                                        midi: Some(midi),
                                        step: st,
                                        alter,
                                        octave: oc,
                                        dur_num: num,
                                        dur_den: den,
                                        rest: false,
                                        chord: is_chord,
                                        measure,
                                        note_type: note_type.clone(),
                                        dots,
                                        accidental: accidental.clone(),
                                    });
                                    if tie_start {
                                        pending[pslot].1.push((key, ev_idx));
                                    }
                                }
                            } else {
                                let ev_idx = parts[idx].events.len();
                                parts[idx].events.push(NoteEvent {
                                    midi: Some(midi),
                                    step: st,
                                    alter,
                                    octave: oc,
                                    dur_num: num,
                                    dur_den: den,
                                    rest: false,
                                    chord: is_chord,
                                    measure,
                                    note_type: note_type.clone(),
                                    dots,
                                    accidental: accidental.clone(),
                                });
                                if tie_start {
                                    pending[pslot].1.push((key, ev_idx));
                                }
                            }
                        }
                    }
                    _ => {}
                }
                stack.pop();
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(format!("that music file is not well-formed XML ({e})"));
            }
            _ => {}
        }
        buf.clear();
    }
    if !stack.is_empty() {
        return Err(
            "that music file is not well-formed XML (it ends in the middle of the score)"
                .to_string(),
        );
    }
    if !saw_score {
        return Err(
            "that file has no score in it (a MusicXML file carries one score-partwise score)"
                .to_string(),
        );
    }
    for part in &mut parts {
        if let Some((_, name)) = part_names.iter().find(|(id, _)| *id == part.part_id) {
            part.part_name = name.clone();
        }
    }
    Ok(CanonicalScore {
        parts,
        fifths,
        mode,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCORE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<score-partwise>
  <part-list>
    <score-part id="P1"><part-name>Soprano</part-name></score-part>
    <score-part id="P2"><part-name>Alto</part-name></score-part>
  </part-list>
  <part id="P1">
    <measure number="1">
      <attributes><divisions>4</divisions><key><fifths>0</fifths><mode>major</mode></key></attributes>
      <note><pitch><step>C</step><octave>4</octave></pitch><duration>4</duration><type>quarter</type></note>
      <note><pitch><step>D</step><octave>4</octave></pitch><duration>4</duration><type>quarter</type></note>
      <note><chord/><pitch><step>E</step><octave>4</octave></pitch><duration>4</duration><type>quarter</type></note>
      <note><pitch><step>F</step><octave>4</octave></pitch><duration>6</duration><dot/><type>quarter</type><tie type="start"/></note>
      <note><pitch><step>F</step><octave>4</octave></pitch><duration>2</duration><type>eighth</type><tie type="stop"/></note>
      <note><grace/><pitch><step>G</step><octave>4</octave></pitch><type>eighth</type></note>
      <note><rest/><duration>4</duration><type>quarter</type></note>
    </measure>
  </part>
  <part id="P2">
    <measure number="1">
      <attributes><divisions>2</divisions></attributes>
      <note><pitch><step>C</step><alter>-1</alter><octave>3</octave></pitch><duration>4</duration><type>half</type></note>
    </measure>
  </part>
</score-partwise>"#;

    #[test]
    fn canonical_stream_rules() {
        let score = parse_canonical(SCORE).unwrap();
        assert_eq!(score.fifths, 0);
        assert_eq!(score.mode, "major");
        assert_eq!(score.parts.len(), 2);
        let soprano = &score.parts[0];
        assert_eq!(soprano.part_id, "P1");
        assert_eq!(soprano.part_name, "Soprano");
        // C quarter, D quarter, E chord-quarter, F dotted-quarter
        // tied to an eighth (merged to a half), rest quarter.
        // The grace G never enters.
        assert_eq!(soprano.events.len(), 5);
        let first = &soprano.events[0];
        assert_eq!(first.step, 'C');
        assert_eq!(first.midi, Some(60));
        assert_eq!((first.dur_num, first.dur_den), (1, 1));
        assert!(!first.chord);
        let chord = &soprano.events[2];
        assert!(chord.chord);
        assert_eq!(chord.step, 'E');
        let tied = &soprano.events[3];
        assert_eq!(tied.step, 'F');
        assert_eq!((tied.dur_num, tied.dur_den), (2, 1));
        let rest = &soprano.events[4];
        assert!(rest.rest);
        assert_eq!(rest.midi, None);
        // The alto part states divisions 2, so its half note
        // is also 2 quarters: same sounding duration, and
        // its B-flat spelling is kept, not collapsed.
        let alto = &score.parts[1];
        assert_eq!(alto.events.len(), 1);
        assert_eq!(alto.events[0].step, 'C');
        assert_eq!(alto.events[0].alter, -1);
        assert_eq!(alto.events[0].midi, Some(47));
        assert_eq!((alto.events[0].dur_num, alto.events[0].dur_den), (2, 1));
    }

    #[test]
    fn dotted_quarter_equals_tied_quarter_plus_eighth() {
        let dotted = r#"<?xml version="1.0"?><score-partwise>
          <part-list><score-part id="P1"><part-name>V</part-name></score-part></part-list>
          <part id="P1"><measure><attributes><divisions>4</divisions></attributes>
          <note><pitch><step>A</step><octave>4</octave></pitch><duration>6</duration><dot/><type>quarter</type></note>
          </measure></part></score-partwise>"#;
        let tied = r#"<?xml version="1.0"?><score-partwise>
          <part-list><score-part id="P1"><part-name>V</part-name></score-part></part-list>
          <part id="P1"><measure><attributes><divisions>4</divisions></attributes>
          <note><pitch><step>A</step><octave>4</octave></pitch><duration>4</duration><type>quarter</type><tie type="start"/></note>
          <note><pitch><step>A</step><octave>4</octave></pitch><duration>2</duration><type>eighth</type><tie type="stop"/></note>
          </measure></part></score-partwise>"#;
        let a = parse_canonical(dotted).unwrap();
        let b = parse_canonical(tied).unwrap();
        // Identical canonically: same pitch and sounding
        // duration. They differ only in display fields
        // (the dotted note states one dot; the tied pair
        // states none), which are never hashed.
        let proj = |e: &NoteEvent| {
            (
                e.midi, e.step, e.alter, e.octave, e.dur_num, e.dur_den, e.rest, e.chord,
            )
        };
        let av: Vec<_> = a.parts[0].events.iter().map(proj).collect();
        let bv: Vec<_> = b.parts[0].events.iter().map(proj).collect();
        assert_eq!(av, bv);
    }

    #[test]
    fn divisions_rescaling_is_invisible() {
        let four = r#"<?xml version="1.0"?><score-partwise>
          <part-list><score-part id="P1"><part-name>V</part-name></score-part></part-list>
          <part id="P1"><measure><attributes><divisions>4</divisions></attributes>
          <note><pitch><step>C</step><octave>4</octave></pitch><duration>4</duration><type>quarter</type></note>
          </measure></part></score-partwise>"#;
        let four_eighty = r#"<?xml version="1.0"?><score-partwise>
          <part-list><score-part id="P1"><part-name>V</part-name></score-part></part-list>
          <part id="P1"><measure><attributes><divisions>480</divisions></attributes>
          <note><pitch><step>C</step><octave>4</octave></pitch><duration>480</duration><type>quarter</type></note>
          </measure></part></score-partwise>"#;
        let a = parse_canonical(four).unwrap();
        let b = parse_canonical(four_eighty).unwrap();
        assert_eq!(a.parts[0].events, b.parts[0].events);
    }

    #[test]
    fn malformed_notes_fail_loudly() {
        let no_duration = r#"<?xml version="1.0"?><score-partwise>
          <part-list><score-part id="P1"><part-name>V</part-name></score-part></part-list>
          <part id="P1"><measure><attributes><divisions>4</divisions></attributes>
          <note><pitch><step>C</step><octave>4</octave></pitch><type>quarter</type></note>
          </measure></part></score-partwise>"#;
        assert!(parse_canonical(no_duration).is_err());
        let timewise = r#"<?xml version="1.0"?><score-timewise></score-timewise>"#;
        assert!(parse_canonical(timewise).is_err());
    }
}
