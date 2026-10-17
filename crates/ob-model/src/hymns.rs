//! Hymns as parts, not as one typed line (roles build
//! R-2a). A hymn line in the order of service names three
//! things: which book, which number, which song. When the
//! three ride in one string, the administrator retypes all
//! of them every week and every error (a wrong number, a
//! dropped title) is born in that retyping. Here the parts
//! are parsed out of the words as written, composed back
//! into one canonical line, and remembered as number and
//! title pairs the church itself confirmed, so Check can
//! hold this week's words up against the church's own
//! memory. There is no bundled hymn database: the memory
//! starts empty and learns only from this church's
//! bulletins.

use crate::{Document, SimpleDate};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// One hymn, in parts. Any part may be missing: a number
/// alone is how most bulletins print, a title alone is how
/// most people speak.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HymnEntry {
    pub hymnal: Option<String>,
    pub number: Option<String>,
    pub title: Option<String>,
}

/// Is this order-of-service label a hymn line's label? The
/// one test every hymn feature shares: the word "hymn"
/// anywhere in the label, in any case ("Opening Hymn",
/// "Hymn of the Day", "Closing hymn").
pub fn is_hymn_label(label: &str) -> bool {
    label.to_lowercase().contains("hymn")
}

/// The first word of `s`, and what follows it.
fn first_word(s: &str) -> (&str, &str) {
    let s = s.trim_start();
    match s.find(char::is_whitespace) {
        Some(i) => (&s[..i], s[i..].trim_start()),
        None => (s, ""),
    }
}

/// Is this word a hymn number as written? Digits, with an
/// optional leading "#" and a trailing comma or period the
/// typing carried ("432", "#432", "432,").
fn number_word(word: &str) -> Option<String> {
    let w = word
        .trim()
        .trim_start_matches('#')
        .trim_end_matches([',', '.']);
    if !w.is_empty() && w.chars().all(|c| c.is_ascii_digit()) {
        Some(w.to_string())
    } else {
        None
    }
}

/// Take a title as written and return the title itself:
/// the separators a typist puts between a number and its
/// title (a comma, a dash, a dot leader dot) come off, and
/// one pair of surrounding quotes comes off. Words inside
/// quotes are kept exactly as written.
fn clean_title(s: &str) -> Option<String> {
    let t = s.trim().trim_start_matches([
        ' ', ',', '-', '\u{2013}', '\u{2014}', ':', ';', '\u{00b7}', '.',
    ]);
    let t = t.trim();
    if t.is_empty() {
        return None;
    }
    let pairs = [
        ('\'', '\''),
        ('"', '"'),
        ('\u{2018}', '\u{2019}'),
        ('\u{201c}', '\u{201d}'),
        ('\u{2019}', '\u{2019}'),
    ];
    let mut core = t;
    for (open, close) in pairs {
        if t.starts_with(open) {
            if let Some(end) = t[open.len_utf8()..].find(close) {
                core = &t[open.len_utf8()..open.len_utf8() + end];
            } else if t.len() > open.len_utf8() && t.ends_with(close) {
                core = &t[open.len_utf8()..t.len() - close.len_utf8()];
            }
            break;
        }
    }
    let core = core.trim().trim_end_matches([',', ';']).trim();
    if core.is_empty() {
        None
    } else {
        Some(core.to_string())
    }
}

/// Read a hymn line's value as its parts. The shapes a
/// church actually writes: "ELW 432", "LBW 165", "432",
/// "Hymn 432", each with a title after the number when one
/// is written (in quotes, after a dash or a comma, or as
/// bare words), and a title standing alone ("Amazing
/// Grace" is a title with no number, never a hymnal).
pub fn parse_hymn_value(text: &str) -> HymnEntry {
    let t = text.trim();
    let mut entry = HymnEntry::default();
    if t.is_empty() {
        return entry;
    }
    let mut rest = t;
    let (w1, r1) = first_word(rest);
    // Words that introduce a number without naming a book.
    let noise = ["hymn", "hymnal", "no", "number"];
    if noise
        .iter()
        .any(|n| w1.trim_end_matches('.').eq_ignore_ascii_case(n))
    {
        rest = r1;
    } else if w1.chars().all(|c| c.is_alphabetic())
        && w1.chars().count() >= 2
        && w1.chars().count() <= 5
    {
        // A short letters-only word followed by a number is
        // a hymnal's name (ELW, LBW, LSB); a word followed
        // by more words is the start of a title.
        let (w2, _) = first_word(r1);
        if number_word(w2).is_some() {
            entry.hymnal = Some(w1.trim_end_matches([':', ',']).to_string());
            rest = r1;
        }
    }
    let (wn, rn) = first_word(rest);
    if let Some(n) = number_word(wn) {
        entry.number = Some(n);
        rest = rn;
    }
    entry.title = clean_title(rest);
    entry
}

/// The one shape a hymn value takes in the bulletin:
/// "<Hymnal> <number> · <title>", each missing part
/// simply left out. A title alone prints as the title.
pub fn compose_hymn_value(entry: &HymnEntry) -> String {
    let clean = |o: &Option<String>| {
        o.as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    };
    let hymnal = clean(&entry.hymnal);
    let number = clean(&entry.number);
    let title = clean(&entry.title);
    let head = [hymnal, number]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" ");
    match (head.is_empty(), title) {
        (true, None) => String::new(),
        (true, Some(title)) => title,
        (false, None) => head,
        (false, Some(title)) => format!("{head} \u{00b7} {title}"),
    }
}

/// The memory's key for one pair: the hymnal's name
/// without regard to case (or no hymnal at all), and the
/// number as written.
pub fn hymn_key(hymnal: Option<&str>, number: &str) -> String {
    format!(
        "{}|{}",
        hymnal.unwrap_or("").trim().to_lowercase(),
        number.trim()
    )
}

/// One remembered pair: the title the church printed for
/// this number, and the Sunday it was last confirmed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HymnPair {
    pub title: String,
    pub last_seen: SimpleDate,
}

/// The church's own hymn memory: number and title pairs
/// learned from the church's confirmed bulletins, nothing
/// else. The app keeps it in a small file beside its
/// settings and feeds it every saved bulletin. A learned
/// template teaches nothing: it carries no occasion date
/// of its own (it spans many Sundays), so its pairs
/// arrive when the template is saved as a real week
/// (round 3, R3-12).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HymnMemory {
    pub pairs: BTreeMap<String, HymnPair>,
}

impl HymnMemory {
    /// The pair remembered for this hymnal and number, if
    /// the church has ever confirmed one.
    pub fn lookup(&self, hymnal: Option<&str>, number: &str) -> Option<&HymnPair> {
        self.pairs.get(&hymn_key(hymnal, number))
    }

    /// Forget one remembered pair outright (round 3,
    /// R3-6): Check's disagreement finding offers this
    /// when the office says the memory is the wrong one.
    /// The pair is not barred from returning: the next
    /// saved bulletin carrying it teaches it again, with
    /// that bulletin's date. Answers true when a pair
    /// was actually there to forget.
    pub fn forget(&mut self, hymnal: Option<&str>, number: &str) -> bool {
        self.forget_key(&hymn_key(hymnal, number))
    }

    /// Forget by the memory's own key (the shape the
    /// finding carries): see `forget`.
    pub fn forget_key(&mut self, key: &str) -> bool {
        self.pairs.remove(key).is_some()
    }

    /// Learn one entry, when it carries both a number and
    /// a title. Confirmation order is save order, and the
    /// last confirmation for a Sunday wins it: a newer
    /// Sunday wins a disagreement, and on the SAME
    /// Sunday a different title is a correction, because
    /// re-saving the same bulletin with the title fixed
    /// is how a typo gets repaired (round 3, R3-6). An
    /// older Sunday never overwrites. Answers true when
    /// the memory changed, so the caller knows to save
    /// it; confirming the same pair for the same Sunday
    /// again changes nothing.
    pub fn learn(&mut self, entry: &HymnEntry, date: SimpleDate) -> bool {
        let (Some(number), Some(title)) = (entry.number.as_deref(), entry.title.as_deref()) else {
            return false;
        };
        let number = number.trim();
        let title = title.trim();
        if number.is_empty() || title.is_empty() {
            return false;
        }
        let key = hymn_key(entry.hymnal.as_deref(), number);
        match self.pairs.get(&key) {
            Some(old) if old.last_seen > date => false,
            Some(old) if old.last_seen == date && old.title == title => false,
            _ => {
                self.pairs.insert(
                    key,
                    HymnPair {
                        title: title.to_string(),
                        last_seen: date,
                    },
                );
                true
            }
        }
    }

    /// Learn from one whole document: every hymn line
    /// (a tabbed line whose label names a hymn, the order
    /// of service's own shape) that carries both a number
    /// and a title. Answers true when the memory changed.
    pub fn learn_from_document(&mut self, doc: &Document, date: SimpleDate) -> bool {
        let mut changed = false;
        for story in &doc.stories {
            for para in &story.paragraphs {
                let text = para.text();
                if !text.contains('\t') {
                    continue;
                }
                let label = text.split('\t').next().unwrap_or("").trim();
                if !is_hymn_label(label) {
                    continue;
                }
                let value = text.rsplit('\t').next().unwrap_or("").trim();
                let entry = parse_hymn_value(value);
                changed |= self.learn(&entry, date);
            }
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(hymnal: Option<&str>, number: Option<&str>, title: Option<&str>) -> HymnEntry {
        HymnEntry {
            hymnal: hymnal.map(str::to_string),
            number: number.map(str::to_string),
            title: title.map(str::to_string),
        }
    }

    #[test]
    fn parses_the_shapes_churches_write() {
        assert_eq!(
            parse_hymn_value("ELW 432"),
            entry(Some("ELW"), Some("432"), None)
        );
        assert_eq!(
            parse_hymn_value("LBW 165"),
            entry(Some("LBW"), Some("165"), None)
        );
        assert_eq!(parse_hymn_value("432"), entry(None, Some("432"), None));
        assert_eq!(parse_hymn_value("Hymn 432"), entry(None, Some("432"), None));
        assert_eq!(parse_hymn_value(""), entry(None, None, None));
        assert_eq!(
            parse_hymn_value("  ELW   432  "),
            entry(Some("ELW"), Some("432"), None)
        );
    }

    #[test]
    fn parses_titles_in_every_separator_dress() {
        let want = entry(Some("ELW"), Some("432"), Some("O Come, All Ye Faithful"));
        assert_eq!(parse_hymn_value("ELW 432 'O Come, All Ye Faithful'"), want);
        assert_eq!(
            parse_hymn_value("ELW 432 \"O Come, All Ye Faithful\""),
            want
        );
        assert_eq!(
            parse_hymn_value("ELW 432 \u{2013} O Come, All Ye Faithful"),
            want
        );
        assert_eq!(parse_hymn_value("ELW 432, O Come, All Ye Faithful"), want);
        assert_eq!(parse_hymn_value("ELW 432 O Come, All Ye Faithful"), want);
        assert_eq!(
            parse_hymn_value("432 \u{00b7} Amazing Grace"),
            entry(None, Some("432"), Some("Amazing Grace"))
        );
        assert_eq!(
            parse_hymn_value("432 - Amazing Grace"),
            entry(None, Some("432"), Some("Amazing Grace"))
        );
    }

    #[test]
    fn a_title_alone_is_a_title() {
        assert_eq!(
            parse_hymn_value("Amazing Grace"),
            entry(None, None, Some("Amazing Grace"))
        );
        assert_eq!(
            parse_hymn_value("'Amazing Grace'"),
            entry(None, None, Some("Amazing Grace"))
        );
        // A long first word is not a hymnal, even followed
        // by words that end in digits.
        assert_eq!(
            parse_hymn_value("Praise to the Lord, the Almighty"),
            entry(None, None, Some("Praise to the Lord, the Almighty"))
        );
    }

    #[test]
    fn compose_omits_missing_parts_gracefully() {
        assert_eq!(
            compose_hymn_value(&entry(Some("ELW"), Some("432"), Some("Amazing Grace"))),
            "ELW 432 \u{00b7} Amazing Grace"
        );
        assert_eq!(
            compose_hymn_value(&entry(Some("ELW"), Some("432"), None)),
            "ELW 432"
        );
        assert_eq!(compose_hymn_value(&entry(None, Some("432"), None)), "432");
        assert_eq!(
            compose_hymn_value(&entry(None, None, Some("Amazing Grace"))),
            "Amazing Grace"
        );
        assert_eq!(compose_hymn_value(&entry(None, None, None)), "");
    }

    #[test]
    fn compose_and_parse_round_trip() {
        for e in [
            entry(Some("ELW"), Some("432"), Some("O Come, All Ye Faithful")),
            entry(Some("LBW"), Some("165"), None),
            entry(None, Some("432"), Some("Amazing Grace")),
            entry(None, Some("432"), None),
            entry(None, None, Some("Amazing Grace")),
        ] {
            assert_eq!(parse_hymn_value(&compose_hymn_value(&e)), e, "{e:?}");
        }
        // A messier written form composes to the canonical
        // shape, and the canonical shape parses back.
        let parsed = parse_hymn_value("ELW 432, 'Amazing Grace'");
        assert_eq!(
            compose_hymn_value(&parsed),
            "ELW 432 \u{00b7} Amazing Grace"
        );
    }

    #[test]
    fn hymn_labels_are_named_by_the_word() {
        assert!(is_hymn_label("Opening Hymn"));
        assert!(is_hymn_label("Hymn of the Day"));
        assert!(is_hymn_label("closing hymn"));
        assert!(!is_hymn_label("Anthem"));
        assert!(!is_hymn_label("Prelude"));
        assert!(!is_hymn_label(""));
    }

    fn d(year: i32, month: u32, day: u32) -> SimpleDate {
        SimpleDate { year, month, day }
    }

    #[test]
    fn memory_learns_pairs_and_the_newer_sunday_wins() {
        let mut mem = HymnMemory::default();
        let e = entry(Some("ELW"), Some("432"), Some("O Come, All Ye Faithful"));
        assert!(mem.learn(&e, d(2026, 10, 4)));
        let pair = mem.lookup(Some("ELW"), "432").expect("remembered");
        assert_eq!(pair.title, "O Come, All Ye Faithful");
        assert_eq!(pair.last_seen, d(2026, 10, 4));
        // The hymnal's case does not matter.
        assert!(mem.lookup(Some("elw"), "432").is_some());
        // An older Sunday never overwrites.
        let older = entry(Some("ELW"), Some("432"), Some("A Different Song"));
        assert!(!mem.learn(&older, d(2026, 9, 27)));
        assert_eq!(
            mem.lookup(Some("ELW"), "432").unwrap().title,
            "O Come, All Ye Faithful"
        );
        // The same Sunday: the last confirmation wins.
        // Re-saving the same bulletin with the title
        // corrected is how a typo gets repaired; the
        // first confirmation keeping the seat made a
        // wrong pair unfixable (round 3, R3-6).
        assert!(mem.learn(&older, d(2026, 10, 4)));
        assert_eq!(
            mem.lookup(Some("ELW"), "432").unwrap().title,
            "A Different Song"
        );
        assert_eq!(
            mem.lookup(Some("ELW"), "432").unwrap().last_seen,
            d(2026, 10, 4)
        );
        // Confirming the same pair again changes nothing.
        assert!(!mem.learn(&older, d(2026, 10, 4)));
        // An older Sunday still never overwrites.
        assert!(!mem.learn(&e, d(2026, 9, 27)));
        // A newer Sunday wins.
        assert!(mem.learn(&older, d(2026, 10, 11)));
        assert_eq!(
            mem.lookup(Some("ELW"), "432").unwrap().title,
            "A Different Song"
        );
        assert_eq!(
            mem.lookup(Some("ELW"), "432").unwrap().last_seen,
            d(2026, 10, 11)
        );
        // Number-only and title-only entries teach nothing.
        assert!(!mem.learn(&entry(Some("ELW"), Some("500"), None), d(2026, 10, 11)));
        assert!(!mem.learn(&entry(None, None, Some("Orphan Title")), d(2026, 10, 11)));
        assert!(mem.lookup(Some("ELW"), "500").is_none());
    }

    #[test]
    fn memory_learns_from_a_documents_hymn_lines() {
        use crate::{ChurchProfile, Paragraph, SheetSpec, Size, Story, FORMAT_VERSION};
        use std::collections::BTreeMap;
        let doc = Document {
            format_version: FORMAT_VERSION,
            title: "t".to_string(),
            profile: ChurchProfile::default(),
            fields: BTreeMap::new(),
            styles: Vec::new(),
            stories: vec![Story {
                id: "order".to_string(),
                keep_through: None,
                paragraphs: vec![
                    Paragraph::plain(
                        "order-line",
                        "Hymn of Praise\tELW 432 \u{00b7} O Come, All Ye Faithful",
                    ),
                    Paragraph::plain("order-line", "Hymn of Response\t365"),
                    Paragraph::plain("order-line", "Prelude\t"),
                    Paragraph::plain("body", "An announcement with a\ttab and no hymn."),
                ],
            }],
            panels: Vec::new(),
            sheet: SheetSpec {
                preset: "letter-portrait".to_string(),
                size: Size::new(0, 0),
            },
            no_announcements: false,
            assets: Vec::new(),
            asset_bytes: BTreeMap::new(),
            occasion_time: None,
            wanted_fonts: Vec::new(),
            participants: BTreeMap::new(),
            inserts: Vec::new(),
            applied_contributions: Vec::new(),
        };
        let mut mem = HymnMemory::default();
        assert!(mem.learn_from_document(&doc, d(2026, 10, 18)));
        // Only the line with both a number and a title taught.
        let pair = mem.lookup(Some("ELW"), "432").expect("remembered");
        assert_eq!(pair.title, "O Come, All Ye Faithful");
        assert_eq!(mem.pairs.len(), 1);
        // The same document again changes nothing.
        assert!(!mem.learn_from_document(&doc, d(2026, 10, 18)));
    }

    #[test]
    fn a_typo_saved_then_corrected_on_the_same_sunday_heals() {
        // Round 3, R3-6: next Sunday's bulletin is drafted
        // early with a wrong title and saved; the office
        // spots it, fixes the same file, and saves again.
        // The correction must displace the typo, or Check
        // flags the correct bulletin forever.
        let mut mem = HymnMemory::default();
        let typo = entry(Some("ELW"), Some("432"), Some("O Come, All Ye Faithful"));
        let fixed = entry(
            Some("ELW"),
            Some("432"),
            Some("Joyful, Joyful We Adore Thee"),
        );
        assert!(mem.learn(&typo, d(2026, 10, 25)));
        assert!(mem.learn(&fixed, d(2026, 10, 25)));
        let pair = mem.lookup(Some("ELW"), "432").expect("remembered");
        assert_eq!(pair.title, "Joyful, Joyful We Adore Thee");
        assert_eq!(pair.last_seen, d(2026, 10, 25));
    }

    #[test]
    fn forgetting_a_pair_removes_it_and_it_can_be_learned_again() {
        // Round 3, R3-6: the office can tell Check to
        // forget a pairing outright; the removal is what
        // the persisted file carries, and nothing bars
        // the pair from being taught again by a later
        // save.
        let mut mem = HymnMemory::default();
        let e = entry(Some("ELW"), Some("432"), Some("O Come, All Ye Faithful"));
        assert!(mem.learn(&e, d(2026, 10, 4)));
        assert!(
            mem.forget(Some("elw"), "432"),
            "the hymnal's case does not matter"
        );
        assert!(mem.lookup(Some("ELW"), "432").is_none());
        assert!(!mem.forget(Some("ELW"), "432"), "nothing left to forget");
        assert!(!mem.forget_key("elw|999"));
        // The forgotten state is what a round trip keeps.
        let text = serde_json::to_string(&mem).expect("serializes");
        let back: HymnMemory = serde_json::from_str(&text).expect("parses");
        assert_eq!(back, mem);
        // And the pair can be learned again, at its new
        // confirmation date.
        assert!(mem.learn(&e, d(2026, 10, 11)));
        assert_eq!(
            mem.lookup(Some("ELW"), "432").unwrap().last_seen,
            d(2026, 10, 11)
        );
    }

    #[test]
    fn memory_survives_a_json_round_trip() {
        let mut mem = HymnMemory::default();
        mem.learn(
            &entry(Some("ELW"), Some("432"), Some("O Come, All Ye Faithful")),
            d(2026, 10, 18),
        );
        mem.learn(
            &entry(None, Some("88"), Some("A Title With No Book")),
            d(2026, 10, 18),
        );
        let text = serde_json::to_string(&mem).expect("serializes");
        let back: HymnMemory = serde_json::from_str(&text).expect("parses");
        assert_eq!(back, mem);
        assert_eq!(hymn_key(None, "88"), "|88");
    }
}
