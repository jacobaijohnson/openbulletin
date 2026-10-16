//! Our own MusicXML reader: a light walk over the file that
//! learns what the bulletin needs to know (title, composer,
//! rights, part names) and counts the music structurally
//! (notes, measures, lyric syllables, fermatas), so every
//! import can check the engraving against the file itself
//! and no loss is ever silent (docs/research/music-spike.md,
//! kill criterion 2).
//!
//! This reader never changes a note. It does not transpose,
//! fix, or author; it reads, counts, and reports, which is
//! the whole of OpenBulletin's music job (Jacob's ruling).

use quick_xml::events::Event;
use quick_xml::Reader;
use std::io::Read as _;

/// The structural counts of one score, taken from the file
/// itself. The engraving is checked against these on every
/// import.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MusicCounts {
    /// Pitched notes, rests excluded, grace notes excluded
    /// (they are counted beside, not inside, this number).
    pub notes: u32,
    /// Grace notes (the small ornaments before a beat).
    pub grace_notes: u32,
    /// Rests.
    pub rests: u32,
    /// Measures in the fullest part (parts agree in an
    /// ordinary score; the sum would count the same music
    /// once per part, so it is never used).
    pub measures: u32,
    /// Lyric syllables: lyric elements that carry a word.
    pub syllables: u32,
    /// Fermatas (holds), wherever in a measure they sit.
    pub fermatas: u32,
}

impl MusicCounts {
    /// Every notehead the engraving should show: ordinary
    /// notes plus grace notes (each draws its own head).
    pub fn noteheads_expected(&self) -> u32 {
        self.notes + self.grace_notes
    }
}

/// What the reader learned about one music file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MusicInfo {
    /// movement-title, else work-title, else the title
    /// credit: the name the hymn goes by.
    pub title: Option<String>,
    /// The subtitle credit, when the file states one.
    pub subtitle: Option<String>,
    /// Creator lines from the file's identification, as
    /// (kind, name) pairs: ("composer", "..."), ("lyricist",
    /// "..."), ("arranger", "..."), and any other kind the
    /// file names, in file order.
    pub creators: Vec<(String, String)>,
    /// Credit blocks as (kind, words) pairs, the printed
    /// furniture of the first page. Used to fill creator
    /// kinds the identification leaves out.
    pub credits: Vec<(String, String)>,
    /// The file's rights line, verbatim, when it states one.
    pub rights: Option<String>,
    /// Part names in score order ("Soprano/Alto", ...).
    pub parts: Vec<String>,
    pub counts: MusicCounts,
}

impl MusicInfo {
    pub fn creator(&self, kind: &str) -> Option<&str> {
        self.creators
            .iter()
            .find(|(k, _)| k == kind)
            .map(|(_, v)| v.as_str())
            .or_else(|| {
                self.credits
                    .iter()
                    .find(|(k, _)| k == kind)
                    .map(|(_, v)| v.as_str())
            })
    }

    pub fn composer(&self) -> Option<&str> {
        self.creator("composer")
    }

    /// The credit line for under the music, composed from
    /// what the file states: its rights line verbatim when
    /// it states one; otherwise the creator lines it names
    /// ("Words: ... Music: ..."), the way hymnals print
    /// them. None when the file states nothing to print.
    pub fn credit_line(&self) -> Option<String> {
        if let Some(rights) = &self.rights {
            let rights = rights.trim();
            if !rights.is_empty() {
                return Some(rights.to_string());
            }
        }
        let mut parts: Vec<String> = Vec::new();
        for (kind, label) in [
            ("lyricist", "Words"),
            ("composer", "Music"),
            ("arranger", "Arranged"),
            ("poet", "Words"),
            ("translator", "Translated"),
        ] {
            if let Some(name) = self.creator(kind) {
                parts.push(format!("{label}: {name}"));
            }
        }
        if parts.is_empty() {
            None
        } else {
            Some(format!("{}.", parts.join(". ")))
        }
    }
}

/// True when these bytes look like a MusicXML file: either
/// the XML itself (a score-partwise root near the head) or
/// a zip whose container or contents say so. Used by the
/// one open door's sniff, where the file's name did not
/// tell. A .sla file also starts with XML; its root names
/// Scribus, never score-partwise, so the two never collide.
pub fn sniff_music(bytes: &[u8]) -> bool {
    if bytes.starts_with(b"PK") {
        return zip_score_text(bytes).is_ok();
    }
    looks_like_score(bytes)
}

fn looks_like_score(bytes: &[u8]) -> bool {
    let head = &bytes[..bytes.len().min(8192)];
    let Ok(text) = std::str::from_utf8(head) else {
        return false;
    };
    text.contains("<score-partwise")
}

/// Load a music file from its bytes: a plain `.musicxml`
/// or a `.mxl` (the same document in a zip) both come back
/// as (the score's XML text, what we learned from it).
/// Errors are plain words for the person at the door.
pub fn load_music(bytes: &[u8]) -> Result<(String, MusicInfo), String> {
    let xml = if bytes.starts_with(b"PK") {
        zip_score_text(bytes)?
    } else {
        let text = String::from_utf8(bytes.to_vec()).map_err(|_| {
            "that file is not music text we can read (it is not a MusicXML file)".to_string()
        })?;
        if !looks_like_score(bytes) {
            return Err(
                "that file does not look like a music file (its first page is not a score)"
                    .to_string(),
            );
        }
        text
    };
    let info = parse_musicxml(&xml)?;
    Ok((xml, info))
}

/// The score's XML out of a .mxl zip: the container's
/// rootfile when META-INF/container.xml names one, else
/// the first .musicxml/.xml entry that is one.
fn zip_score_text(bytes: &[u8]) -> Result<String, String> {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes))
        .map_err(|e| format!("that file is a zip we cannot open ({e})"))?;
    let mut names: Vec<String> = (0..zip.len())
        .filter_map(|i| zip.by_index(i).ok().map(|f| f.name().to_string()))
        .collect();
    let mut main: Option<String> = None;
    if let Ok(mut container) = zip.by_name("META-INF/container.xml") {
        let mut text = String::new();
        if container.read_to_string(&mut text).is_ok() {
            main = rootfile_path(&text).filter(|p| names.iter().any(|n| n == p));
        }
    }
    if main.is_none() {
        names.sort();
        main = names
            .iter()
            .find(|n| {
                let lower = n.to_ascii_lowercase();
                (lower.ends_with(".musicxml") || lower.ends_with(".xml"))
                    && !lower.starts_with("meta-inf/")
            })
            .cloned();
    }
    let Some(name) = main else {
        return Err(
            "that zip holds no music score (a .mxl file carries one .musicxml inside)".to_string(),
        );
    };
    let mut entry = zip
        .by_name(&name)
        .map_err(|e| format!("the score inside that zip cannot be read ({e})"))?;
    let mut text = String::new();
    entry
        .read_to_string(&mut text)
        .map_err(|e| format!("the score inside that zip cannot be read ({e})"))?;
    if !looks_like_score(text.as_bytes()) {
        return Err("the file inside that zip is not a score we can read".to_string());
    }
    Ok(text)
}

/// The first full-path attribute in a container.xml.
fn rootfile_path(container: &str) -> Option<String> {
    let key = "full-path=\"";
    let start = container.find(key)? + key.len();
    let end = container[start..].find('"')? + start;
    Some(container[start..end].to_string())
}

fn local(name: &[u8]) -> String {
    let s = String::from_utf8_lossy(name);
    s.rsplit(':').next().unwrap_or("").to_string()
}

fn attr(e: &quick_xml::events::BytesStart, key: &str) -> Option<String> {
    for a in e.attributes().flatten() {
        if local(a.key.as_ref()) == key {
            return Some(String::from_utf8_lossy(&a.value).into_owned());
        }
    }
    None
}

/// Parse one MusicXML document (the `.musicxml` text).
/// Deliberately a walk, not a model: we count and label,
/// we do not interpret the music.
pub fn parse_musicxml(xml: &str) -> Result<MusicInfo, String> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);
    let mut info = MusicInfo::default();
    let mut stack: Vec<String> = Vec::new();
    let mut buf: Vec<u8> = Vec::new();
    // Text capture: the element whose text we are reading,
    // and the buffer it fills.
    let mut capture: Option<String> = None;
    let mut text = String::new();
    // Per-part measure counting.
    let mut in_part = false;
    let mut measures_here: u32 = 0;
    // Open note / lyric state.
    let mut note: Option<(bool, bool)> = None; // (is_rest, is_grace)
    let mut lyric_has_text = false;
    let mut in_lyric = false;
    // Credit assembly: the current credit's kind and words.
    let mut credit_kind: Option<String> = None;
    let mut credit_words: Vec<String> = Vec::new();
    let mut work_title: Option<String> = None;
    let mut movement_title: Option<String> = None;
    let mut saw_score = false;

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
                    "part" => {
                        in_part = true;
                        measures_here = 0;
                    }
                    "measure" if in_part => measures_here += 1,
                    "note" => note = Some((false, false)),
                    "rest" => {
                        if let Some(n) = &mut note {
                            n.0 = true;
                        }
                    }
                    "grace" => {
                        if let Some(n) = &mut note {
                            n.1 = true;
                        }
                    }
                    "fermata" => {
                        if stack.iter().any(|s| s == "measure") {
                            info.counts.fermatas += 1;
                        }
                    }
                    "lyric" => {
                        in_lyric = true;
                        lyric_has_text = false;
                    }
                    "text" if in_lyric && stack.last().map(|s| s.as_str()) == Some("lyric") => {
                        lyric_has_text = true;
                    }
                    "credit-words" => capture = Some("credit-words".into()),
                    "credit-type" => capture = Some("credit-type".into()),
                    "work-title" => capture = Some("work-title".into()),
                    "movement-title" => capture = Some("movement-title".into()),
                    "creator" => {
                        capture = Some(format!("creator:{}", attr(&e, "type").unwrap_or_default()))
                    }
                    "rights" => capture = Some("rights".into()),
                    "part-name" if stack.iter().any(|s| s == "score-part") => {
                        capture = Some("part-name".into())
                    }
                    _ => {}
                }
                if capture.is_some()
                    && matches!(
                        name.as_str(),
                        "credit-words"
                            | "credit-type"
                            | "work-title"
                            | "movement-title"
                            | "creator"
                            | "rights"
                            | "part-name"
                    )
                {
                    text.clear();
                }
                stack.push(name);
            }
            Ok(Event::Empty(e)) => {
                let name = local(e.name().as_ref());
                match name.as_str() {
                    "rest" => {
                        if let Some(n) = &mut note {
                            n.0 = true;
                        }
                    }
                    "grace" => {
                        if let Some(n) = &mut note {
                            n.1 = true;
                        }
                    }
                    "fermata" if stack.iter().any(|s| s == "measure") => {
                        info.counts.fermatas += 1;
                    }
                    _ => {}
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
                    "part" => {
                        in_part = false;
                        info.counts.measures = info.counts.measures.max(measures_here);
                    }
                    "note" => {
                        if let Some((is_rest, is_grace)) = note.take() {
                            if is_rest {
                                info.counts.rests += 1;
                            } else if is_grace {
                                info.counts.grace_notes += 1;
                            } else {
                                info.counts.notes += 1;
                            }
                        }
                    }
                    "lyric" => {
                        in_lyric = false;
                        if lyric_has_text {
                            info.counts.syllables += 1;
                        }
                    }
                    "credit" => {
                        if let Some(kind) = credit_kind.take() {
                            let words = credit_words.join(" ");
                            if !words.trim().is_empty() {
                                info.credits.push((kind, words.trim().to_string()));
                            }
                        }
                        credit_words.clear();
                    }
                    "credit-words" if capture.as_deref() == Some("credit-words") => {
                        let w = text.trim().to_string();
                        if !w.is_empty() {
                            credit_words.push(w);
                        }
                        capture = None;
                    }
                    "credit-type" if capture.as_deref() == Some("credit-type") => {
                        credit_kind = Some(text.trim().to_string());
                        capture = None;
                    }
                    "work-title" if capture.as_deref() == Some("work-title") => {
                        work_title = Some(text.trim().to_string());
                        capture = None;
                    }
                    "movement-title" if capture.as_deref() == Some("movement-title") => {
                        movement_title = Some(text.trim().to_string());
                        capture = None;
                    }
                    "rights" if capture.as_deref() == Some("rights") => {
                        let t = text.trim().to_string();
                        if !t.is_empty() {
                            info.rights = Some(t);
                        }
                        capture = None;
                    }
                    "part-name" if capture.as_deref() == Some("part-name") => {
                        let t = text.trim().to_string();
                        if !t.is_empty() {
                            info.parts.push(t);
                        }
                        capture = None;
                    }
                    _ => {
                        if let Some(c) = &capture {
                            if let Some(kind) = c.strip_prefix("creator:") {
                                if name == "creator" {
                                    let t = text.trim().to_string();
                                    if !t.is_empty() {
                                        let kind = if kind.is_empty() {
                                            "creator".to_string()
                                        } else {
                                            kind.to_string()
                                        };
                                        info.creators.push((kind, t));
                                    }
                                    capture = None;
                                }
                            }
                        }
                    }
                }
                stack.pop();
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(format!(
                    "that music file is not well-formed XML ({})",
                    plain_xml_error(&e.to_string())
                ));
            }
            _ => {}
        }
        buf.clear();
    }
    if !stack.is_empty() {
        // The streaming reader tolerates an abrupt end; a
        // score that stops mid-element is not well-formed,
        // and half a hymn must never import as a whole one.
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
    info.title = movement_title
        .filter(|t| !t.is_empty())
        .or(work_title)
        .or_else(|| info.creator("title").map(|s| s.to_string()));
    info.subtitle = info
        .credits
        .iter()
        .find(|(k, _)| k == "subtitle")
        .map(|(_, v)| v.clone());
    Ok(info)
}

/// Keep an XML parser's complaint short enough for a
/// sentence in the review window.
fn plain_xml_error(raw: &str) -> String {
    let raw = raw.trim();
    if raw.len() > 90 {
        format!("{}...", &raw[..90])
    } else {
        raw.to_string()
    }
}

/// A tiny score written for the crate's tests (two
/// measures, one rest, one grace note, one fermata, three
/// lyric syllables, a rights line): public-domain-shaped,
/// original, and small enough to check by eye.
#[cfg(test)]
pub(crate) const TINY_SCORE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<score-partwise version="4.0">
  <work><work-title>Work Title Should Lose</work-title></work>
  <movement-title>A Tiny Test Hymn</movement-title>
  <identification>
    <creator type="composer">Test Composer</creator>
    <creator type="lyricist">Test Poet</creator>
    <rights>Public domain in the file's own words</rights>
  </identification>
  <part-list>
    <score-part id="P1"><part-name>Voice</part-name></score-part>
  </part-list>
  <part id="P1">
    <measure number="1">
      <note><pitch><step>C</step><octave>4</octave></pitch><duration>2</duration>
        <lyric><syllabic>single</syllabic><text>Come</text></lyric></note>
      <note><pitch><step>D</step><octave>4</octave></pitch><duration>2</duration>
        <lyric><syllabic>begin</syllabic><text>let</text></lyric></note>
      <note><rest/><duration>2</duration></note>
      <note><grace/><pitch><step>E</step><octave>4</octave></pitch></note>
    </measure>
    <measure number="2">
      <note><pitch><step>F</step><octave>4</octave></pitch><duration>4</duration>
        <notations><fermata/></notations>
        <lyric><syllabic>end</syllabic><text>sing</text></lyric></note>
      <note><pitch><step>G</step><octave>4</octave></pitch><duration>4</duration>
        <lyric><extend/></lyric></note>
    </measure>
  </part>
</score-partwise>"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiny_score_counts_and_credits() {
        let info = parse_musicxml(TINY_SCORE).expect("parse");
        assert_eq!(info.title.as_deref(), Some("A Tiny Test Hymn"));
        assert_eq!(info.composer(), Some("Test Composer"));
        assert_eq!(info.creator("lyricist"), Some("Test Poet"));
        assert_eq!(
            info.rights.as_deref(),
            Some("Public domain in the file's own words")
        );
        assert_eq!(info.parts, vec!["Voice".to_string()]);
        // 4 pitched notes (C, D, F, G), the grace E apart,
        // the one rest apart, 2 measures, 3 worded syllables
        // (the extend-only lyric sings no new word), 1 fermata.
        assert_eq!(info.counts.notes, 4);
        assert_eq!(info.counts.grace_notes, 1);
        assert_eq!(info.counts.rests, 1);
        assert_eq!(info.counts.measures, 2);
        assert_eq!(info.counts.syllables, 3);
        assert_eq!(info.counts.fermatas, 1);
        // The rights line wins as the credit line, verbatim.
        assert_eq!(
            info.credit_line().as_deref(),
            Some("Public domain in the file's own words")
        );
    }

    #[test]
    fn credit_line_composes_from_creators_without_rights() {
        let info = MusicInfo {
            creators: vec![
                ("composer".into(), "Jane Tunesmith".into()),
                ("lyricist".into(), "John Wordsworth".into()),
            ],
            ..Default::default()
        };
        assert_eq!(
            info.credit_line().as_deref(),
            Some("Words: John Wordsworth. Music: Jane Tunesmith.")
        );
    }

    #[test]
    fn malformed_and_scoreless_files_refuse_in_plain_words() {
        assert!(parse_musicxml("<score-partwise><part>").is_err());
        let err = parse_musicxml("<html><body/></html>").unwrap_err();
        assert!(err.contains("no score"), "{err}");
        let err =
            parse_musicxml("<score-timewise><measure number=\"1\"/></score-timewise>").unwrap_err();
        assert!(err.contains("timewise"), "{err}");
        let err = load_music(b"definitely not music").unwrap_err();
        assert!(err.contains("does not look like a music file"), "{err}");
    }

    #[test]
    fn mxl_unzips_through_its_container() {
        let mut buf = Vec::new();
        {
            let mut zw = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
            let opts = zip::write::SimpleFileOptions::default();
            zw.start_file("META-INF/container.xml", opts).unwrap();
            use std::io::Write as _;
            zw.write_all(
                br#"<?xml version="1.0"?><container><rootfiles><rootfile full-path="score.musicxml"/></rootfiles></container>"#,
            )
            .unwrap();
            zw.start_file("score.musicxml", opts).unwrap();
            zw.write_all(TINY_SCORE.as_bytes()).unwrap();
            zw.finish().unwrap();
        }
        let (xml, info) = load_music(&buf).expect("load mxl");
        assert!(xml.contains("score-partwise"));
        assert_eq!(info.counts.notes, 4);
        assert!(sniff_music(&buf));
        assert!(sniff_music(TINY_SCORE.as_bytes()));
        assert!(!sniff_music(b"<SCRIBUSUTF8NEW/>"));
    }

    /// The Bach fixture from the M0 spike: the real corpus.
    /// Skips cleanly when the fixtures are not on this
    /// machine (they never enter the repo).
    #[test]
    fn bach_fixture_counts_match_the_spike() {
        let path = concat!(
            env!("HOME"),
            "/workspace/openbulletin-fixtures/music-spike/ein-feste-burg.musicxml"
        );
        let Ok(bytes) = std::fs::read(path) else {
            eprintln!("spike fixture absent; skipping");
            return;
        };
        let (_xml, info) = load_music(&bytes).expect("load fixture");
        assert_eq!(info.counts.notes, 237);
        assert_eq!(info.counts.grace_notes, 0);
        assert_eq!(info.counts.rests, 0);
        assert_eq!(info.counts.measures, 14);
        assert_eq!(info.counts.syllables, 58);
        assert_eq!(info.counts.fermatas, 13);
        assert_eq!(info.composer(), Some("Johann Sebastian Bach"));
        assert_eq!(
            info.title.as_deref(),
            Some("Ein' feste Burg ist unser Gott")
        );
        assert_eq!(info.parts.len(), 2);
        assert_eq!(
            info.credit_line().as_deref(),
            Some("Words: Martin Luther (1483-1546). Music: Johann Sebastian Bach.")
        );
    }
}
