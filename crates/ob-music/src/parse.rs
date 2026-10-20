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

/// One zip entry's decompressed cap, the same 32 MB the
/// IDML door puts on its parts. A score is text; a .mxl
/// whose entry inflates past this is a bomb, not a hymn
/// (audit Batch 1: the score and the container were read
/// with no cap at all, and a 4 MB file reached 4 GB in
/// memory).
const MAX_ENTRY_BYTES: u64 = 32 * 1024 * 1024;

/// XML nesting deeper than this is not a score, it is a
/// trap (health audit S-7): the sibling parsers (IDML,
/// Scribus, SVG) all stop at 256, and the walk below
/// piles every open element onto a stack, so a score
/// nested millions deep would pile without bound. Real
/// MusicXML nests perhaps thirty levels; this is
/// generous headroom.
const MAX_XML_DEPTH: usize = 256;

/// Read one zip entry's text under the cap, through
/// ob-model's one capped read (health audit E-9). This
/// door's judgment is Refuse: one byte past the cap is
/// read, so an entry exactly at the cap still counts
/// as inside; anything larger refuses in plain words.
fn read_entry_bytes(entry: &mut zip::read::ZipFile, what: &str) -> Result<Vec<u8>, String> {
    let too_large = format!("{what} is too large to read (past 32 MB of music text)");
    let read = ob_model::read_capped(
        entry,
        MAX_ENTRY_BYTES,
        ob_model::CapOverflow::Refuse(too_large),
    )
    .map_err(|e| match e {
        ob_model::CappedReadError::Refused(words) => words,
        ob_model::CappedReadError::Unreadable(e) => format!("{what} cannot be read ({e})"),
    })?;
    Ok(read.bytes)
}

fn read_entry_text(entry: &mut zip::read::ZipFile, what: &str) -> Result<String, String> {
    let bytes = read_entry_bytes(entry, what)?;
    String::from_utf8(bytes).map_err(|_| format!("{what} is not text we can read"))
}

/// The plain words for a zip whose directory names one
/// part twice with different bytes, in the .obd door's
/// shape (round 31, F5).
fn duplicate_name_refusal(name: &str) -> String {
    format!(
        "that file has two parts named {name}, so the file was not opened; \
         it may be damaged."
    )
}

/// Where one central record's entry payload stands
/// and how it is packed, its sizes and local offset
/// resolved through the zip64 extra field wherever a
/// plain field carries the sentinel — the same spot
/// ob-format's duplicate walk reads (round 31, F1).
struct RawSpot {
    method: u16,
    comp_size: u64,
    local_offset: u64,
}

/// Read one central record's spot out of its fixed
/// header and extra field (the zip64 values ride in
/// the order uncompressed, compressed, local offset,
/// disk, one per sentinel field).
fn raw_spot(header: &[u8], extra: &[u8]) -> RawSpot {
    let field = |at: usize| u32::from_le_bytes(header[at..at + 4].try_into().unwrap_or([0; 4]));
    let comp32 = field(20);
    let uncomp32 = field(24);
    let local32 = field(42);
    let disk16 = u16::from_le_bytes([header[34], header[35]]);
    let mut comp_size = u64::from(comp32);
    let mut local_offset = u64::from(local32);
    if comp32 == u32::MAX || uncomp32 == u32::MAX || local32 == u32::MAX || disk16 == u16::MAX {
        let mut at = 0usize;
        while at + 4 <= extra.len() {
            let id = u16::from_le_bytes([extra[at], extra[at + 1]]);
            let size = usize::from(u16::from_le_bytes([extra[at + 2], extra[at + 3]]));
            let data = &extra[at + 4..(at + 4 + size).min(extra.len())];
            if id == 0x0001 {
                let mut pos = 0usize;
                if uncomp32 == u32::MAX {
                    pos += 8;
                }
                if comp32 == u32::MAX && pos + 8 <= data.len() {
                    comp_size = u64::from_le_bytes(data[pos..pos + 8].try_into().unwrap_or([0; 8]));
                    pos += 8;
                }
                if local32 == u32::MAX && pos + 8 <= data.len() {
                    local_offset =
                        u64::from_le_bytes(data[pos..pos + 8].try_into().unwrap_or([0; 8]));
                }
                break;
            }
            at += 4 + size;
        }
    }
    RawSpot {
        method: u16::from_le_bytes([header[10], header[11]]),
        comp_size,
        local_offset,
    }
}

/// The CP437 characters for bytes 0x80..=0xFF, in byte
/// order: the zip reader's own table, copied so a name
/// the reader decodes as CP437 is keyed here by the
/// characters the reader resolves it to (round 32, F4).
const CP437_HIGH: &str = "\
    \u{00c7}\u{00fc}\u{00e9}\u{00e2}\u{00e4}\u{00e0}\u{00e5}\u{00e7}\
    \u{00ea}\u{00eb}\u{00e8}\u{00ef}\u{00ee}\u{00ec}\u{00c4}\u{00c5}\
    \u{00c9}\u{00e6}\u{00c6}\u{00f4}\u{00f6}\u{00f2}\u{00fb}\u{00f9}\
    \u{00ff}\u{00d6}\u{00dc}\u{00a2}\u{00a3}\u{00a5}\u{20a7}\u{0192}\
    \u{00e1}\u{00ed}\u{00f3}\u{00fa}\u{00f1}\u{00d1}\u{00aa}\u{00ba}\
    \u{00bf}\u{2310}\u{00ac}\u{00bd}\u{00bc}\u{00a1}\u{00ab}\u{00bb}\
    \u{2591}\u{2592}\u{2593}\u{2502}\u{2524}\u{2561}\u{2562}\u{2556}\
    \u{2555}\u{2563}\u{2551}\u{2557}\u{255d}\u{255c}\u{255b}\u{2510}\
    \u{2514}\u{2534}\u{252c}\u{251c}\u{2500}\u{253c}\u{255e}\u{255f}\
    \u{255a}\u{2554}\u{2569}\u{2566}\u{2560}\u{2550}\u{256c}\u{2567}\
    \u{2568}\u{2564}\u{2565}\u{2559}\u{2558}\u{2552}\u{2553}\u{256b}\
    \u{256a}\u{2518}\u{250c}\u{2588}\u{2584}\u{258c}\u{2590}\u{2580}\
    \u{03b1}\u{00df}\u{0393}\u{03c0}\u{03a3}\u{03c3}\u{00b5}\u{03c4}\
    \u{03a6}\u{0398}\u{03a9}\u{03b4}\u{221e}\u{03c6}\u{03b5}\u{2229}\
    \u{2261}\u{00b1}\u{2265}\u{2264}\u{2320}\u{2321}\u{00f7}\u{2248}\
    \u{00b0}\u{2219}\u{00b7}\u{221a}\u{207f}\u{00b2}\u{25a0}\u{00a0}";

/// Key one raw central-directory name the way the zip
/// reader resolves it (round 32, F4): with the entry's
/// UTF-8 flag set, the name is UTF-8 text; with it
/// clear, the reader decodes the bytes as CP437. The
/// duplicate scan keyed names by lossy UTF-8, so a
/// CP437-named pair keyed to a name the reader never
/// serves and the check passed over it.
fn decode_entry_name(raw: &[u8], flags: u16) -> String {
    if flags & 0x0800 != 0 {
        return String::from_utf8_lossy(raw).into_owned();
    }
    raw.iter()
        .map(|&byte| {
            if byte < 0x80 {
                byte as char
            } else {
                CP437_HIGH
                    .chars()
                    .nth(usize::from(byte - 0x80))
                    .unwrap_or('\u{fffd}')
            }
        })
        .collect()
}

/// The (name, spot) pairs of the container's central
/// directory, walked off the raw records: the zip
/// reader's own map is keyed by name, so a duplicated
/// name reaches it as only its last copy and the
/// copies can only be counted here, before that map
/// is trusted (round 31, F5). The directory is the
/// one the latest end record names, through its
/// zip64 record where the plain fields are sentinels.
/// Empty when no end record stands — the archive
/// reader reports that damage in its own words.
fn central_spots(bytes: &[u8]) -> Vec<(String, RawSpot)> {
    let mut eocd: Option<usize> = None;
    if bytes.len() >= 22 {
        for at in (0..=bytes.len() - 22).rev() {
            if !bytes[at..].starts_with(b"PK\x05\x06") {
                continue;
            }
            let comment = usize::from(u16::from_le_bytes([bytes[at + 20], bytes[at + 21]]));
            if at + 22 + comment <= bytes.len() {
                eocd = Some(at);
                break;
            }
        }
    }
    let Some(eocd) = eocd else {
        return Vec::new();
    };
    let field = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap_or([0; 4]));
    let mut total = u64::from(u16::from_le_bytes([bytes[eocd + 10], bytes[eocd + 11]]));
    let mut cd_offset = u64::from(field(eocd + 16));
    if (total == u64::from(u16::MAX) || cd_offset == u64::from(u32::MAX)) && eocd >= 20 {
        let locator = &bytes[eocd - 20..eocd];
        if locator.starts_with(b"PK\x06\x07") {
            // The locator's record offset is declared
            // relative to the archive's own start, like
            // every offset here: the record itself
            // stands at-or-after it, shifted by whatever
            // junk precedes the archive, and the reader
            // finds it by searching forward (round 32,
            // F4). The search stops at the locator.
            let declared = u64::from_le_bytes(locator[8..16].try_into().unwrap_or([0; 8]));
            let mut record_at = declared as usize;
            let mut found = None;
            while record_at + 4 <= eocd - 20 {
                if bytes[record_at..].starts_with(b"PK\x06\x06") {
                    found = Some(record_at);
                    break;
                }
                record_at += 1;
            }
            if let Some(record_at) = found {
                if record_at + 56 <= bytes.len() {
                    let record = &bytes[record_at..record_at + 56];
                    total = u64::from_le_bytes(record[32..40].try_into().unwrap_or([0; 8]));
                    cd_offset = u64::from_le_bytes(record[48..56].try_into().unwrap_or([0; 8]));
                }
            }
        }
    }
    if total == 0 {
        return Vec::new();
    }
    // The directory's first record stands at-or-after
    // the declared offset, the reader's own derivation
    // (round 32, F4): junk prepended before the archive
    // shifts every declared offset by the same amount,
    // and a scan that starts exactly at the declared
    // offset starts inside the last entry's body,
    // breaks on its first check, and counts nothing —
    // no duplicate check happened at all. The distance
    // from the declared offset to the first record that
    // stands is the archive offset, and it applies to
    // the local offsets in the records just as the
    // reader applies it.
    let mut at = cd_offset as usize;
    let mut cd_start = None;
    while at + 4 <= eocd {
        if bytes[at..].starts_with(b"PK\x01\x02") {
            cd_start = Some(at);
            break;
        }
        at += 1;
    }
    let Some(cd_start) = cd_start else {
        return Vec::new();
    };
    let archive_offset = (cd_start as u64).saturating_sub(cd_offset);
    let mut out: Vec<(String, RawSpot)> = Vec::new();
    let mut at = cd_start;
    for _ in 0..total {
        if at + 46 > bytes.len() || !bytes[at..].starts_with(b"PK\x01\x02") {
            break;
        }
        let header = &bytes[at..at + 46];
        let name_len = usize::from(u16::from_le_bytes([header[28], header[29]]));
        let extra_len = usize::from(u16::from_le_bytes([header[30], header[31]]));
        let comment_len = usize::from(u16::from_le_bytes([header[32], header[33]]));
        let name_start = at + 46;
        let name_end = name_start + name_len;
        let extra_end = name_end + extra_len;
        if extra_end > bytes.len() {
            break;
        }
        let flags = u16::from_le_bytes([header[8], header[9]]);
        let name = decode_entry_name(&bytes[name_start..name_end], flags);
        let mut spot = raw_spot(header, &bytes[name_end..extra_end]);
        spot.local_offset = spot.local_offset.saturating_add(archive_offset);
        out.push((name, spot));
        at = extra_end + comment_len;
    }
    out
}

/// One entry's decompressed bytes off its raw spot,
/// read one byte past this door's cap: two copies are
/// judged on the bytes the door itself would read,
/// and a score past the cap is the capped read's
/// refusal to report, exactly as before. None when
/// the payload cannot be read back at all.
fn raw_payload(bytes: &[u8], spot: &RawSpot) -> Option<Vec<u8>> {
    use std::io::Read as _;
    let local = spot.local_offset as usize;
    if local + 30 > bytes.len() || !bytes[local..].starts_with(b"PK\x03\x04") {
        return None;
    }
    let name_len = usize::from(u16::from_le_bytes([bytes[local + 26], bytes[local + 27]]));
    let extra_len = usize::from(u16::from_le_bytes([bytes[local + 28], bytes[local + 29]]));
    let data_start = local + 30 + name_len + extra_len;
    let data_end = data_start.checked_add(spot.comp_size as usize)?;
    if data_end > bytes.len() {
        return None;
    }
    let raw = &bytes[data_start..data_end];
    match spot.method {
        0 => Some(raw[..(raw.len() as u64).min(MAX_ENTRY_BYTES + 1) as usize].to_vec()),
        8 => {
            let mut decoder = flate2::read::DeflateDecoder::new(raw);
            let mut out = Vec::new();
            decoder
                .by_ref()
                .take(MAX_ENTRY_BYTES + 1)
                .read_to_end(&mut out)
                .ok()?;
            Some(out)
        }
        _ => None,
    }
}

/// Answer the name when the container holds more than
/// one copy of `name` and the copies' decompressed
/// bytes disagree (round 31, F5): the reader's map
/// would serve only the LAST copy, so which container
/// or score a file was read from must never be the
/// archive's quiet choice. Exact byte-identical
/// repeats pass — whichever copy is served, the same
/// bytes are served — and a copy whose bytes cannot
/// be read back cannot be proven identical, so it
/// does not count as exact either. A name held once
/// is never payload-read here, so single-copy
/// behavior is untouched.
fn disagreeing_duplicate(bytes: &[u8], spots: &[(String, RawSpot)], name: &str) -> Option<String> {
    let copies: Vec<&RawSpot> = spots
        .iter()
        .filter(|(n, _)| n == name)
        .map(|(_, spot)| spot)
        .collect();
    if copies.len() < 2 {
        return None;
    }
    let first_payload = raw_payload(bytes, copies[0]);
    if first_payload.is_none() {
        return Some(name.to_string());
    }
    for spot in &copies[1..] {
        if raw_payload(bytes, spot) != first_payload {
            return Some(name.to_string());
        }
    }
    None
}

/// The score's XML out of a .mxl zip: the container's
/// rootfile when META-INF/container.xml names one, else
/// the first .musicxml/.xml entry that is one.
fn zip_score_text(bytes: &[u8]) -> Result<String, String> {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes))
        .map_err(|e| format!("that file is a zip we cannot open ({e})"))?;
    // Entry-count cap (health audit S-6), the IDML and
    // font-pack readers' 512: a .mxl carries a container
    // and a score, not thousands of entries, and the
    // count is known from the archive's own directory
    // before any entry is read.
    const MAX_ENTRIES: usize = 512;
    if zip.len() > MAX_ENTRIES {
        return Err(format!(
            "that zip holds {} files; a music file may carry at most {}",
            zip.len(),
            MAX_ENTRIES
        ));
    }
    let mut names: Vec<String> = (0..zip.len())
        .filter_map(|i| zip.by_index(i).ok().map(|f| f.name().to_string()))
        .collect();
    // A duplicated container whose copies disagree
    // would redirect the score choice to its last
    // copy; refuse before any copy is read (round 31,
    // F5). The copies are counted off the raw central
    // directory (central_spots): the archive's own map
    // has already collapsed them to the last one.
    let spots = central_spots(bytes);
    if let Some(dup) = disagreeing_duplicate(bytes, &spots, "META-INF/container.xml") {
        return Err(duplicate_name_refusal(&dup));
    }
    let mut main: Option<String> = None;
    if let Ok(mut container) = zip.by_name("META-INF/container.xml") {
        // A container that cannot be read (or is past the
        // cap) simply does not name the score; the entry
        // scan below decides, and the score's own read is
        // capped all the same.
        if let Ok(text) = read_entry_text(&mut container, "the list inside that zip") {
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
    // The same discipline for the score the door
    // resolved: duplicated copies that disagree refuse
    // instead of serving the last (round 31, F5).
    if let Some(dup) = disagreeing_duplicate(bytes, &spots, &name) {
        return Err(duplicate_name_refusal(&dup));
    }
    let mut entry = zip
        .by_name(&name)
        .map_err(|e| format!("the score inside that zip cannot be read ({e})"))?;
    let text = read_entry_text(&mut entry, "the score inside that zip")?;
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
                if stack.len() > MAX_XML_DEPTH {
                    return Err(
                        "that music file's XML is nested too deeply to read safely".to_string()
                    );
                }
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

    fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        haystack.windows(needle.len()).position(|w| w == needle)
    }

    /// Rename one entry everywhere its name is written
    /// (the local header and the central directory),
    /// the two names the same length: the zip writer
    /// refuses a duplicate name outright, so a file
    /// carrying a repeated entry is assembled by patch,
    /// the same helper shape as ob-format's tests.
    fn patch_entry_name(zip_bytes: &mut [u8], from_name: &str, to_name: &str) {
        assert_eq!(from_name.len(), to_name.len(), "the patch keeps lengths");
        let name = from_name.as_bytes();
        let mut patched = 0;
        let mut from = 0;
        while let Some(found) = find_bytes(&zip_bytes[from..], name) {
            let at = from + found;
            let guarded = (at >= 30 && &zip_bytes[at - 30..at - 26] == b"PK\x03\x04")
                || (at >= 46 && &zip_bytes[at - 46..at - 42] == b"PK\x01\x02");
            if guarded {
                zip_bytes[at..at + name.len()].copy_from_slice(to_name.as_bytes());
                patched += 1;
            }
            from = at + 1;
        }
        assert_eq!(patched, 2, "both records for {from_name} were patched");
    }

    fn mxl_zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
        use std::io::Write as _;
        let mut buf = Vec::new();
        {
            let mut zw = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
            let opts = zip::write::SimpleFileOptions::default();
            for (name, body) in entries {
                zw.start_file(*name, opts).unwrap();
                zw.write_all(body).unwrap();
            }
            zw.finish().unwrap();
        }
        buf
    }

    const CONTAINER_FOR_SCORE: &[u8] = br#"<?xml version="1.0"?><container><rootfiles><rootfile full-path="score.musicxml"/></rootfiles></container>"#;

    #[test]
    fn duplicate_score_entries_with_different_bodies_error() {
        // Round 31, F5: two score.musicxml entries with
        // different scores inside; the door used to
        // serve the last copy silently. It refuses now,
        // naming the duplicated part.
        let score_a = TINY_SCORE.replace("A Tiny Test Hymn", "First Test Hymn!");
        let score_b = TINY_SCORE.replace("A Tiny Test Hymn", "Other Test Hymn!");
        assert_ne!(score_a, score_b);
        let mut buf = mxl_zip(&[
            ("META-INF/container.xml", CONTAINER_FOR_SCORE),
            ("score.musicxml", score_a.as_bytes()),
            ("score.musicxmX", score_b.as_bytes()),
        ]);
        patch_entry_name(&mut buf, "score.musicxmX", "score.musicxml");
        let err = load_music(&buf).expect_err("disagreeing duplicate scores refuse");
        assert!(err.contains("two parts named score.musicxml"), "{err}");
        assert!(err.contains("may be damaged"), "{err}");
    }

    #[test]
    fn duplicate_container_entries_naming_different_scores_error() {
        // Round 31, F5: two container.xml entries naming
        // different score files; the last container used
        // to redirect the score choice quietly.
        let container_a: &[u8] = br#"<?xml version="1.0"?><container><rootfiles><rootfile full-path="a.musicxml"/></rootfiles></container>"#;
        let container_b: &[u8] = br#"<?xml version="1.0"?><container><rootfiles><rootfile full-path="b.musicxml"/></rootfiles></container>"#;
        let score_a = TINY_SCORE.replace("A Tiny Test Hymn", "First Test Hymn!");
        let score_b = TINY_SCORE.replace("A Tiny Test Hymn", "Other Test Hymn!");
        let mut buf = mxl_zip(&[
            ("META-INF/container.xml", container_a),
            ("META-INF/container.xmX", container_b),
            ("a.musicxml", score_a.as_bytes()),
            ("b.musicxml", score_b.as_bytes()),
        ]);
        patch_entry_name(&mut buf, "META-INF/container.xmX", "META-INF/container.xml");
        let err = load_music(&buf).expect_err("disagreeing duplicate containers refuse");
        assert!(
            err.contains("two parts named META-INF/container.xml"),
            "{err}"
        );
    }

    #[test]
    fn exact_duplicate_score_and_container_entries_load() {
        // Round 31, F5, the other side: byte-identical
        // repeats of both resolved names load — the same
        // bytes whichever copy is served.
        let mut buf = mxl_zip(&[
            ("META-INF/container.xml", CONTAINER_FOR_SCORE),
            ("META-INF/container.xmX", CONTAINER_FOR_SCORE),
            ("score.musicxml", TINY_SCORE.as_bytes()),
            ("score.musicxmX", TINY_SCORE.as_bytes()),
        ]);
        patch_entry_name(&mut buf, "META-INF/container.xmX", "META-INF/container.xml");
        patch_entry_name(&mut buf, "score.musicxmX", "score.musicxml");
        let (_xml, info) = load_music(&buf).expect("exact repeats load");
        assert_eq!(info.title.as_deref(), Some("A Tiny Test Hymn"));
        assert_eq!(info.counts.notes, 4);
    }

    /// Patch ONE byte of one entry's name everywhere
    /// the name is written (the local header and the
    /// central directory), at `at_in_name` in the
    /// name: the byte-level twin of patch_entry_name,
    /// for names no Rust string can hold (a raw CP437
    /// byte, round 32, F4).
    fn patch_name_byte(zip_bytes: &mut [u8], name: &str, at_in_name: usize, byte: u8) {
        let name_bytes = name.as_bytes();
        let mut patched = 0;
        let mut from = 0;
        while let Some(found) = find_bytes(&zip_bytes[from..], name_bytes) {
            let at = from + found;
            let guarded = (at >= 30 && &zip_bytes[at - 30..at - 26] == b"PK\x03\x04")
                || (at >= 46 && &zip_bytes[at - 46..at - 42] == b"PK\x01\x02");
            if guarded {
                zip_bytes[at + at_in_name] = byte;
                patched += 1;
            }
            from = at + 1;
        }
        assert_eq!(patched, 2, "both records for {name} were patched");
    }

    const CONTAINER_FOR_CP437_SCORE: &str = "<?xml version=\"1.0\"?><container><rootfiles><rootfile full-path=\"sc\u{00fc}re.musicxml\"/></rootfiles></container>";

    #[test]
    fn duplicate_cp437_named_score_entries_with_different_bodies_error() {
        // Round 32, F4 (review MINOR-1, witness
        // cp437_dup.mxl): two differing scores under a
        // raw name carrying byte 0x81 with the UTF-8
        // flag clear. The reader resolves that name as
        // CP437 ("scüre.musicxml", the name the
        // container states); the duplicate scan keyed
        // names by lossy UTF-8, so its key never equaled
        // the resolved name, the check passed over the
        // pair, and by_name served the last copy. The
        // scan keys names the way the reader resolves
        // them now, and the pair refuses.
        let score_a = TINY_SCORE.replace("A Tiny Test Hymn", "First Test Hymn!");
        let score_b = TINY_SCORE.replace("A Tiny Test Hymn", "Other Test Hymn!");
        let mut buf = mxl_zip(&[
            (
                "META-INF/container.xml",
                CONTAINER_FOR_CP437_SCORE.as_bytes(),
            ),
            ("scAre.musicxml", score_a.as_bytes()),
            ("scBre.musicxml", score_b.as_bytes()),
        ]);
        patch_name_byte(&mut buf, "scAre.musicxml", 2, 0x81);
        patch_name_byte(&mut buf, "scBre.musicxml", 2, 0x81);
        let err = load_music(&buf).expect_err("disagreeing CP437 duplicates refuse");
        assert!(
            err.contains("two parts named sc\u{00fc}re.musicxml"),
            "{err}"
        );
        assert!(err.contains("may be damaged"), "{err}");
    }

    #[test]
    fn a_single_cp437_named_score_loads() {
        // Round 32, F4, the control beside the witness:
        // one CP437-named score on its own loads — the
        // door genuinely reads such names, so the pair
        // above is refused for its duplication, not for
        // its encoding.
        let score = TINY_SCORE.replace("A Tiny Test Hymn", "First Test Hymn!");
        let mut buf = mxl_zip(&[
            (
                "META-INF/container.xml",
                CONTAINER_FOR_CP437_SCORE.as_bytes(),
            ),
            ("scAre.musicxml", score.as_bytes()),
        ]);
        patch_name_byte(&mut buf, "scAre.musicxml", 2, 0x81);
        let (_xml, info) = load_music(&buf).expect("a single CP437 score loads");
        assert_eq!(info.title.as_deref(), Some("First Test Hymn!"));
    }

    #[test]
    fn mxl_score_entry_past_the_cap_refuses_in_plain_words() {
        // Audit Batch 1: the score entry was read with no
        // cap, so a small .mxl could inflate to gigabytes
        // in memory. This entry is a hair past 32 MB of
        // almost-nothing, deflated to very little on disk.
        let mut buf = Vec::new();
        {
            let mut zw = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
            let opts = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);
            zw.start_file("score.musicxml", opts).unwrap();
            use std::io::Write as _;
            let filler = "a".repeat(33 * 1024 * 1024);
            zw.write_all(format!("<score-partwise>{filler}</score-partwise>").as_bytes())
                .unwrap();
            zw.finish().unwrap();
        }
        assert!(
            buf.len() < 1024 * 1024,
            "the bomb is small on disk: {} bytes",
            buf.len()
        );
        let err = load_music(&buf).expect_err("refused");
        assert!(err.contains("too large"), "{err}");
        assert!(
            !sniff_music(&buf),
            "the sniff refuses it too, without reading it whole"
        );
    }

    #[test]
    fn mxl_past_the_entry_count_cap_is_refused() {
        // S-6 pin: 513 tiny entries against the 512 cap,
        // refused from the archive's own directory count.
        let mut buf = Vec::new();
        {
            let mut zw = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
            let opts = zip::write::SimpleFileOptions::default();
            use std::io::Write as _;
            for i in 0..513 {
                zw.start_file(format!("f{i}.txt"), opts).unwrap();
                zw.write_all(b"x").unwrap();
            }
            zw.finish().unwrap();
        }
        let err = load_music(&buf).expect_err("too many entries refused");
        assert!(err.contains("512"), "{err}");
    }

    #[test]
    fn score_nested_past_the_depth_budget_refuses_in_plain_words() {
        // S-7 pin: a score nested deeper than the sibling
        // parsers' 256 refuses; the audit's probe nested
        // 3,000,000 deep and parsed Ok.
        let deep = format!(
            "<score-partwise>{}</score-partwise>",
            "<a>".repeat(300) + &"</a>".repeat(300)
        );
        let err = parse_musicxml(&deep).expect_err("too deep refused");
        assert!(err.contains("nested too deeply"), "{err}");
        // Control: nesting within the budget still parses.
        let within = format!(
            "<score-partwise>{}</score-partwise>",
            "<a>".repeat(200) + &"</a>".repeat(200)
        );
        assert!(parse_musicxml(&within).is_ok());
    }

    /// The Bach fixture from the M0 spike: the real corpus.
    /// Skips cleanly when the fixtures are not on this
    /// machine (they never enter the repo).
    #[test]
    fn bach_fixture_counts_match_the_spike() {
        // Runtime home lookup, not compile-time env!:
        // HOME is not set on Windows runners (USERPROFILE
        // is), so the old form did not compile there. With
        // neither variable set the path is empty and the
        // read below fails, skipping exactly as an absent
        // fixture does.
        let path = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(|home| {
                std::path::PathBuf::from(home)
                    .join("workspace/openbulletin-fixtures/music-spike/ein-feste-burg.musicxml")
            })
            .unwrap_or_default();
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
