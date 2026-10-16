//! Contributions (roles build R-1): the words other people
//! in the church send for the bulletin.
//!
//! A contribution is a small piece of markdown anyone can
//! type in a notes app or an email: two or three short
//! header lines at the top, then one heading per part.
//! Senders never run this program; the administrator's
//! Inbox and paste box read the shape here, and the review
//! slots each part into the bulletin. The sender's card
//! (one printable page) teaches exactly this shape and
//! nothing else.
//!
//! The header lines are:
//!   From: a name or a job (Pastor Lee, Music director)
//!   For Sunday: the date, in any form `parse_flexible_date`
//!     reads (a weekday that disagrees yields no date at
//!     all, never a guess)
//!   Keep-through: optional; the last Sunday an announcement
//!     should keep printing
//!
//! Part headings are markdown `##` lines, matched without
//! regard to case: "For the service", "Announcement" (or
//! "Announcements"), "Insert", "Sunday school", "Youth".
//! Words before the first heading are an announcement.
//! Part bodies go through the one markdown parser, so bold
//! and italic the sender typed survive as runs. Each typed
//! line is its own paragraph, the way the Announcements
//! step itself treats lines: a service part is a list of
//! lines, and email prose arrives hard-wrapped, so joining
//! lines together would mangle both.

use ob_model::markdown::markdown_to_paragraphs;
use ob_model::{default_styles, parse_flexible_date, Paragraph, SimpleDate};

/// Which part of the bulletin a piece of a contribution
/// belongs to, named the way the sender's card names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartKind {
    Service,
    Announcement,
    Insert,
    SundaySchool,
    Youth,
}

impl PartKind {
    /// The heading words, spelled as the sender's card
    /// spells them: a heading line is `##` plus these.
    pub fn heading(self) -> &'static str {
        match self {
            PartKind::Service => "For the service",
            PartKind::Announcement => "Announcement",
            PartKind::Insert => "Insert",
            PartKind::SundaySchool => "Sunday school",
            PartKind::Youth => "Youth",
        }
    }

    /// The part's name in plain sentences ("the Sunday
    /// school part").
    pub fn name(self) -> &'static str {
        match self {
            PartKind::Service => "service",
            PartKind::Announcement => "announcement",
            PartKind::Insert => "loose sheet",
            PartKind::SundaySchool => "Sunday school",
            PartKind::Youth => "youth",
        }
    }
}

/// One part of a contribution: its kind, and its words as
/// paragraphs (runs intact).
#[derive(Debug, Clone, PartialEq)]
pub struct Part {
    pub kind: PartKind,
    pub paragraphs: Vec<Paragraph>,
}

/// A whole contribution, parsed.
#[derive(Debug, Clone, PartialEq)]
pub struct Contribution {
    pub from: Option<String>,
    pub for_date: Option<SimpleDate>,
    pub keep_through: Option<SimpleDate>,
    pub parts: Vec<Part>,
}

/// The raw split of a contribution, before dates and
/// markdown are interpreted: header values as written, and
/// each part's body text as written, heading line removed.
/// The review's Edit mode works on this shape and re-parses.
#[derive(Debug, Clone, PartialEq)]
pub struct RawContribution {
    pub from: Option<String>,
    pub for_sunday: Option<String>,
    pub keep_through: Option<String>,
    pub parts: Vec<(PartKind, String)>,
}

/// The part kind a heading line names, if it names one.
/// Any `#` line whose words are not a part name is ordinary
/// body text (a heading inside a part), never a part break.
fn part_heading(line: &str) -> Option<PartKind> {
    let t = line.trim();
    if !t.starts_with('#') {
        return None;
    }
    let words = t
        .trim_start_matches('#')
        .trim()
        .trim_end_matches(':')
        .trim();
    let w = words.to_ascii_lowercase();
    Some(match w.as_str() {
        "for the service" => PartKind::Service,
        "announcement" | "announcements" => PartKind::Announcement,
        "insert" => PartKind::Insert,
        "sunday school" => PartKind::SundaySchool,
        "youth" => PartKind::Youth,
        _ => return None,
    })
}

/// One header line's (key, value), when the line is one:
/// the key matched without regard to case, the value as
/// written, trimmed. All slicing is on the line as
/// written, at boundaries the key itself guarantees.
fn header_line(line: &str) -> Option<(&'static str, String)> {
    let t = line.trim();
    for key in ["from", "for sunday", "keep-through", "keep through"] {
        // `get` answers None when the cut would split a
        // character: such a line is not this header.
        let Some(head) = t.get(..key.len()) else {
            continue;
        };
        if !head.eq_ignore_ascii_case(key) {
            continue;
        }
        let rest = t[key.len()..].trim_start();
        if let Some(value) = rest.strip_prefix(':') {
            return Some((key, value.trim().to_string()));
        }
    }
    None
}

/// Split a contribution into its header values and its
/// parts' raw body texts, in file order. Header lines are
/// read from the words before the first part heading and
/// are not body text; words before the first heading that
/// are not header lines come back as an announcement part.
pub fn split_contribution(text: &str) -> RawContribution {
    let mut raw = RawContribution {
        from: None,
        for_sunday: None,
        keep_through: None,
        parts: Vec::new(),
    };
    // Sections in file order: the words before the first
    // heading (None), then one per part heading.
    let mut sections: Vec<(Option<PartKind>, Vec<String>)> = vec![(None, Vec::new())];
    for line in text.lines() {
        if let Some(kind) = part_heading(line) {
            sections.push((Some(kind), Vec::new()));
        } else {
            sections
                .last_mut()
                .expect("preamble")
                .1
                .push(line.to_string());
        }
    }
    let mut it = sections.into_iter();
    let (_, preamble) = it.next().expect("preamble");
    let mut preamble_words: Vec<String> = Vec::new();
    for line in &preamble {
        match header_line(line) {
            Some(("from", v)) => raw.from = Some(v),
            Some(("for sunday", v)) => raw.for_sunday = Some(v),
            Some((_, v)) => raw.keep_through = Some(v),
            None => preamble_words.push(line.clone()),
        }
    }
    let preamble_text = preamble_words.join("\n");
    if !preamble_text.trim().is_empty() {
        raw.parts.push((PartKind::Announcement, preamble_text));
    }
    for (kind, lines) in it {
        raw.parts
            .push((kind.expect("headed section"), lines.join("\n")));
    }
    raw
}

/// Read a contribution. A date that does not read (a wrong
/// weekday above all) yields None for that date, never a
/// guess. A contribution with no readable words anywhere
/// is refused in plain words, like every other importer.
pub fn parse_contribution(text: &str, today: SimpleDate) -> Result<Contribution, String> {
    let raw = split_contribution(text);
    let styles = default_styles();
    let mut parts: Vec<Part> = Vec::new();
    for (kind, body) in &raw.parts {
        // Line by line through the one parser (see the
        // module note): each typed line is a paragraph.
        let paragraphs: Vec<Paragraph> = body
            .lines()
            .flat_map(|line| markdown_to_paragraphs(line, &styles))
            .filter(|p| !p.text().trim().is_empty())
            .collect();
        if !paragraphs.is_empty() {
            parts.push(Part {
                kind: *kind,
                paragraphs,
            });
        }
    }
    if parts.is_empty() {
        return Err("This file has no words we can read.".to_string());
    }
    let date = |v: &Option<String>| v.as_deref().and_then(|s| parse_flexible_date(s, today));
    Ok(Contribution {
        from: raw.from.filter(|s| !s.trim().is_empty()),
        for_date: date(&raw.for_sunday),
        keep_through: date(&raw.keep_through),
        parts,
    })
}

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

const WEEKDAYS: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];

/// "October 25, 2026": the date without its weekday, for
/// sentences that already say the day ("the bulletin for
/// Sunday, ...").
pub fn format_date_short(d: SimpleDate) -> String {
    let month = MONTHS
        .get((d.month as usize).wrapping_sub(1))
        .copied()
        .unwrap_or("");
    format!("{month} {}, {}", d.day, d.year)
}

/// "Sunday, October 25, 2026": the date in full, the way
/// the steps speak of a service day.
pub fn format_date_long(d: SimpleDate) -> String {
    let weekday = WEEKDAYS[(d.weekday() % 7) as usize];
    format!("{weekday}, {}", format_date_short(d))
}

/// A contribution's Sunday against the open bulletin's:
/// the review's first line and its Accept button both
/// follow from this verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeekVerdict {
    /// The words name the bulletin's own Sunday.
    SameSunday,
    /// The words name no Sunday (or the bulletin names
    /// none): nothing to disagree about.
    NoSundayNamed,
    /// The words name a different Sunday than the open
    /// bulletin. Accept must arm: words never land in the
    /// wrong week silently.
    DifferentSunday(SimpleDate),
}

/// The wrong-week guard's verdict (roles synthesis: a part
/// can be held for its own Sunday, never lands silently).
pub fn week_verdict(c: &Contribution, bulletin: Option<SimpleDate>) -> WeekVerdict {
    match (c.for_date, bulletin) {
        (Some(said), Some(open)) if said != open => WeekVerdict::DifferentSunday(said),
        (Some(_), Some(_)) => WeekVerdict::SameSunday,
        _ => WeekVerdict::NoSundayNamed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn today() -> SimpleDate {
        SimpleDate {
            year: 2026,
            month: 10,
            day: 16,
        }
    }

    fn oct(day: u32) -> SimpleDate {
        SimpleDate {
            year: 2026,
            month: 10,
            day,
        }
    }

    const FULL: &str = "\
From: Pastor Lee
For Sunday: October 25, 2026
Keep-through: November 8, 2026

## For the service
Sermon: **The Good Shepherd**
Hymn\t432

## Announcement
Harvest supper is Saturday at 5 PM.

## Insert
Prayer list for this week.

## Sunday school
Classes meet at 9 AM.

## Youth
Youth group meets Wednesday at 6 PM.
";

    #[test]
    fn full_header_and_all_five_parts() {
        let c = parse_contribution(FULL, today()).expect("parses");
        assert_eq!(c.from.as_deref(), Some("Pastor Lee"));
        assert_eq!(c.for_date, Some(oct(25)));
        assert_eq!(
            c.keep_through,
            Some(SimpleDate {
                year: 2026,
                month: 11,
                day: 8
            })
        );
        let kinds: Vec<PartKind> = c.parts.iter().map(|p| p.kind).collect();
        assert_eq!(
            kinds,
            vec![
                PartKind::Service,
                PartKind::Announcement,
                PartKind::Insert,
                PartKind::SundaySchool,
                PartKind::Youth,
            ]
        );
        assert_eq!(c.parts[0].paragraphs[0].text(), "Sermon: The Good Shepherd");
        assert_eq!(
            c.parts[1].paragraphs[0].text(),
            "Harvest supper is Saturday at 5 PM."
        );
    }

    #[test]
    fn missing_header_still_parses() {
        let c = parse_contribution("## Announcement\n\nThe bake sale is Friday.\n", today())
            .expect("parses");
        assert_eq!(c.from, None);
        assert_eq!(c.for_date, None);
        assert_eq!(c.keep_through, None);
        assert_eq!(c.parts.len(), 1);
        assert_eq!(c.parts[0].kind, PartKind::Announcement);
    }

    #[test]
    fn words_before_the_first_heading_are_an_announcement() {
        let text =
            "From: Sue\n\nChoir rehearsal moved to Thursday.\n\n## Youth\n\nPizza night Friday.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.parts.len(), 2);
        assert_eq!(c.parts[0].kind, PartKind::Announcement);
        assert_eq!(
            c.parts[0].paragraphs[0].text(),
            "Choir rehearsal moved to Thursday."
        );
        assert_eq!(c.parts[1].kind, PartKind::Youth);
    }

    #[test]
    fn headings_match_without_regard_to_case_or_plural() {
        let text = "## ANNOUNCEMENTS\n\nOne.\n\n## sunday school\n\nTwo.\n";
        let c = parse_contribution(text, today()).expect("parses");
        let kinds: Vec<PartKind> = c.parts.iter().map(|p| p.kind).collect();
        assert_eq!(kinds, vec![PartKind::Announcement, PartKind::SundaySchool]);
    }

    #[test]
    fn flexible_dates_read_and_a_wrong_weekday_yields_no_date() {
        // 2026-10-25 is a Sunday; saying Monday makes the
        // date unreadable rather than guessed.
        let wrong = "From: Sue\nFor Sunday: Monday, October 25, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(wrong, today()).expect("parses");
        assert_eq!(c.for_date, None);
        let right = "From: Sue\nFor Sunday: Sun, Oct 25, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(right, today()).expect("parses");
        assert_eq!(c.for_date, Some(oct(25)));
        let numeric = "For Sunday: 10/25/26\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(numeric, today()).expect("parses");
        assert_eq!(c.for_date, Some(oct(25)));
    }

    #[test]
    fn runs_the_sender_typed_survive() {
        let text = "## Announcement\n\nThe **harvest supper** is *Saturday* at 5.\n";
        let c = parse_contribution(text, today()).expect("parses");
        let runs = &c.parts[0].paragraphs[0].runs;
        assert!(
            runs.iter()
                .any(|r| r.text.contains("harvest supper") && r.bold == Some(true)),
            "bold run survives: {runs:?}"
        );
        assert!(
            runs.iter()
                .any(|r| r.text.contains("Saturday") && r.italic == Some(true)),
            "italic run survives: {runs:?}"
        );
    }

    #[test]
    fn no_readable_words_is_a_plain_refusal() {
        assert_eq!(
            parse_contribution("From: Sue\nFor Sunday: October 25, 2026\n", today()),
            Err("This file has no words we can read.".to_string())
        );
        assert_eq!(
            parse_contribution("  \n\n", today()),
            Err("This file has no words we can read.".to_string())
        );
        assert_eq!(
            parse_contribution("## Announcement\n\n## Youth\n", today()),
            Err("This file has no words we can read.".to_string())
        );
    }

    #[test]
    fn header_values_keep_their_letters_exactly() {
        // A name whose letters change byte length when
        // lowercased must still come through whole: the
        // header reader slices the line as written.
        let text = "From: İpek Yılmaz\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.from.as_deref(), Some("İpek Yılmaz"));
        // And a line that merely starts alike is not a
        // header: its words stay an announcement.
        let text = "Fromage is a fine word.\n\n## Youth\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.from, None);
        assert_eq!(c.parts[0].kind, PartKind::Announcement);
        assert_eq!(c.parts[0].paragraphs[0].text(), "Fromage is a fine word.");
    }

    #[test]
    fn week_verdicts() {
        let mut c = parse_contribution(FULL, today()).expect("parses");
        assert_eq!(
            week_verdict(&c, Some(oct(18))),
            WeekVerdict::DifferentSunday(oct(25))
        );
        assert_eq!(week_verdict(&c, Some(oct(25))), WeekVerdict::SameSunday);
        assert_eq!(week_verdict(&c, None), WeekVerdict::NoSundayNamed);
        c.for_date = None;
        assert_eq!(week_verdict(&c, Some(oct(18))), WeekVerdict::NoSundayNamed);
    }

    #[test]
    fn date_formats_speak_plainly() {
        assert_eq!(format_date_long(oct(25)), "Sunday, October 25, 2026");
        assert_eq!(format_date_short(oct(25)), "October 25, 2026");
    }

    #[test]
    fn split_keeps_the_words_as_written_for_editing() {
        let raw = split_contribution(FULL);
        assert_eq!(raw.from.as_deref(), Some("Pastor Lee"));
        assert_eq!(raw.for_sunday.as_deref(), Some("October 25, 2026"));
        assert_eq!(raw.keep_through.as_deref(), Some("November 8, 2026"));
        assert_eq!(raw.parts.len(), 5);
        assert_eq!(raw.parts[0].0, PartKind::Service);
        assert!(raw.parts[0].1.contains("Sermon: **The Good Shepherd**"));
    }
}
