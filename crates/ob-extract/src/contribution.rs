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
//!     all, never a guess; a framing weekday may wear the
//!     filler word "on" wherever it stands in the line,
//!     and a leading and a trailing framing weekday are
//!     each checked); "the Sunday after [date]" reads
//!     as that Sunday — its leading "on"/"the" garnishes
//!     compose, a trailing weekday on that shape
//!     describes the named date, and the named date is
//!     the shape's base, never the line's own date — and
//!     a Sunday named only in relative words ("next
//!     Sunday", or "the Sunday after next Sunday") is
//!     remembered in words for the review to read
//!     against the open bulletin, trailing weekday
//!     garnish and all
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
    /// The For Sunday line named a weekday that its own
    /// date does not fall on (round 3, RT3-D). Such a
    /// date does not read at all, so `for_date` is None;
    /// this record keeps what the line said, so the
    /// review can name the mistake instead of
    /// misdescribing the file as naming no Sunday.
    pub named_weekday_mismatch: Option<WeekdayMismatch>,
    /// The same kindness for the Keep-through line
    /// (round 4, R4-7): a wrong weekday there used to
    /// drop the date in silence, and the announcement
    /// simply ended a week early. `keep_through` is
    /// None when this record is set.
    pub keep_through_weekday_mismatch: Option<WeekdayMismatch>,
    /// The For Sunday line named a Sunday only in
    /// relative words: "next Sunday", "this Sunday",
    /// "on Sunday", or a bare "Sunday" (round 5,
    /// RT5-6). No date can be read from the words
    /// alone, so `for_date` is None; this keeps the
    /// sender's own words, trimmed, so the review can
    /// say the file DID name a Sunday, and read the
    /// words against the open bulletin's date, instead
    /// of claiming no Sunday was named.
    pub for_sunday_in_words: Option<String>,
    /// The For Sunday line named a Sunday in words
    /// AFTER another Sunday named in relative words:
    /// "the Sunday after next Sunday" (round 6,
    /// RT6-9). The shape is remembered as written;
    /// `for_date` stays None, but the verdict machinery
    /// resolves it against the open bulletin (the
    /// relative Sunday reads as that bulletin's date,
    /// as a bare relative Sunday does, and the named
    /// Sunday is the first Sunday strictly after it),
    /// and arms it like any named Sunday that differs.
    pub for_sunday_after_in_words: Option<String>,
}

/// A For Sunday line whose weekday disagrees with its
/// date: "Monday, October 25, 2026", when that date is
/// not a Monday. Naming no Sunday and naming an
/// impossible one are different facts, and only the
/// second is worth a word with the sender.
#[derive(Debug, Clone, PartialEq)]
pub struct WeekdayMismatch {
    /// The weekday word as written ("Monday").
    pub named: String,
    /// The date the line names, read without its weekday.
    pub date: SimpleDate,
}

impl WeekdayMismatch {
    /// The review's first line for such a part.
    pub fn sentence(&self) -> String {
        let actual = WEEKDAYS[(self.date.weekday() % 7) as usize];
        format!(
            "This part names {}, {}, but that date is a {}. \
             Check the date with the sender.",
            self.named,
            format_date_short(self.date),
            actual
        )
    }

    /// The same sentence for the Keep-through line
    /// (round 4, R4-7): the same shape, saying which
    /// line carried the mistake.
    pub fn keep_through_sentence(&self) -> String {
        let actual = WEEKDAYS[(self.date.weekday() % 7) as usize];
        format!(
            "This part says keep through {}, {}, but that date is a {}. \
             Check the date with the sender.",
            self.named,
            format_date_short(self.date),
            actual
        )
    }
}

/// A weekday word as an office writes one, full or
/// shortened, to its number (0 = Sunday .. 6 =
/// Saturday): the same table the date reader uses.
fn weekday_word(word: &str) -> Option<u32> {
    let w = word.trim_end_matches(['.', ',']).to_ascii_lowercase();
    Some(match w.as_str() {
        "sun" | "sunday" => 0,
        "mon" | "monday" => 1,
        "tue" | "tues" | "tuesday" => 2,
        "wed" | "wednesday" => 3,
        "thu" | "thur" | "thurs" | "thursday" => 4,
        "fri" | "friday" => 5,
        "sat" | "saturday" => 6,
        _ => return None,
    })
}

/// Tell "a Sunday was named with the wrong weekday"
/// apart from "no Sunday was named": when the For
/// Sunday line as written does not read, but its first
/// word is a weekday — or the filler word "on" stands
/// in front of one (round 6, R6-6) — and the rest
/// reads as a date that falls on a different weekday,
/// the line named an impossible day.
fn named_weekday_mismatch(raw: &Option<String>, today: SimpleDate) -> Option<WeekdayMismatch> {
    let text = raw.as_deref()?.trim();
    if text.is_empty() || parse_flexible_date(text, today).is_some() {
        return None;
    }
    let (first, rest) = text.split_once(|c: char| c.is_whitespace())?;
    // A framing weekday may wear the filler word "on"
    // in front of it ("on Monday, October 25, 2026",
    // round 6, R6-6): the weekday still stands directly
    // before the date, so it frames the date exactly as
    // the bare leading form does. Any other words
    // between the weekday and the date keep it ordinary
    // text (round 5, R5-D).
    let (named_word, rest) = if weekday_word(first).is_some() {
        (first, rest)
    } else if first.eq_ignore_ascii_case("on") {
        let (second, remainder) = rest.split_once(|c: char| c.is_whitespace())?;
        weekday_word(second)?;
        (second, remainder)
    } else {
        return None;
    };
    let named_number = weekday_word(named_word)?;
    let date = parse_flexible_date(rest.trim_start_matches(',').trim(), today)?;
    if date.weekday() == named_number {
        // The refusal had another cause; nothing about
        // a wrong weekday can honestly be said.
        return None;
    }
    Some(WeekdayMismatch {
        named: named_word.trim_end_matches([',', '.']).to_string(),
        date,
    })
}

/// Days in a month, leap years counted: the small
/// calendar helper the Sunday-after reading needs
/// (ob-model keeps its own copy private).
fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

/// A date `n` days later, crossing month and year
/// edges by the calendar above.
fn add_days(mut d: SimpleDate, mut n: u32) -> SimpleDate {
    while n > 0 {
        d.day += 1;
        if d.day > days_in_month(d.year, d.month) {
            d.day = 1;
            d.month += 1;
            if d.month > 12 {
                d.month = 1;
                d.year += 1;
            }
        }
        n -= 1;
    }
    d
}

/// The first Sunday STRICTLY after a date: a date
/// that is itself a Sunday steps a whole week.
fn first_sunday_after(named: SimpleDate) -> SimpleDate {
    let until_sunday = (7 - named.weekday()) % 7;
    let step = if until_sunday == 0 { 7 } else { until_sunday };
    add_days(named, step)
}

/// How a "Sunday after [date]" line reads: the Sunday
/// it names, or the weekday mistake it carries.
enum SundayAfter {
    Date(SimpleDate),
    Mismatch(WeekdayMismatch),
}

/// "The Sunday after October 25, 2026" (round 5,
/// RT5-6): a Sunday named by its place after a date
/// the line does give. The named date may be written
/// in any form `parse_flexible_date` reads, its own
/// leading weekday included. The line may ALSO carry
/// a trailing weekday in the office's way — "Sunday
/// after October 25, 2026 (Monday)" (round 6,
/// RT6-8) — and that weekday describes the date it
/// follows (the base date; it cannot describe the
/// answer, which is a Sunday by construction): an
/// agreeing one lets the line resolve, and a
/// disagreeing one records the mismatch against the
/// base date and yields no date, exactly as the plain
/// trailing form has since round 4. Leading "on" and
/// "the" garnishes compose with the shape (round 7,
/// R7-5), and the base date may carry its own leading
/// weekday: a disagreeing one records the mismatch
/// against the base and yields no date (round 7,
/// R7-4) — the base is never salvaged as the line's
/// own date.
fn sunday_after_outcome(text: &str, today: SimpleDate) -> Option<SundayAfter> {
    let mut rest = text.trim();
    // Opening garnishes are decoration, in either order
    // (round 7, R7-5): "on Sunday after ...", "on the
    // Sunday after ...", and the plain "the Sunday
    // after ..." all name the same shape.
    while let Some((first, remainder)) = rest.split_once(char::is_whitespace) {
        if first.eq_ignore_ascii_case("the") || first.eq_ignore_ascii_case("on") {
            rest = remainder.trim_start();
        } else {
            break;
        }
    }
    let (first, remainder) = rest.split_once(char::is_whitespace)?;
    if weekday_word(first) != Some(0) {
        return None;
    }
    let remainder = remainder.trim_start();
    let (second, remainder) = remainder.split_once(char::is_whitespace)?;
    if !second.eq_ignore_ascii_case("after") {
        return None;
    }
    let remainder = remainder.trim();
    if let Some(named) = parse_flexible_date(remainder, today) {
        return Some(SundayAfter::Date(first_sunday_after(named)));
    }
    // The base did not read whole. Its pieces may still
    // compose (round 7, R7-4/R7-5): an optional trailing
    // weekday describing the base, and an optional
    // leading weekday framing the base (wearing "on" or
    // not). Take the trailing weekday out first, then
    // read what is left; whichever framing weekday
    // disagrees with the base date is the mistake, and
    // the base date itself is never the line's date.
    let tokens: Vec<&str> = remainder.split_whitespace().collect();
    let last_word = tokens
        .iter()
        .rposition(|t| t.chars().any(|c| c.is_alphanumeric()));
    let mut trailing: Option<(String, u32)> = None;
    let mut core: Vec<&str> = tokens.clone();
    if let Some(i) = last_word {
        let word = tokens[i].trim_matches(['(', ')', '[', ']', ',', '.']);
        if let Some(named_number) = weekday_word(word) {
            let named = word.to_string();
            trailing = Some((named, named_number));
            core = tokens
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .map(|(_, t)| *t)
                .collect();
        }
    }
    if let Some(base) = parse_flexible_date(&core.join(" "), today) {
        return Some(match trailing {
            Some((named, named_number)) if base.weekday() != named_number => {
                SundayAfter::Mismatch(WeekdayMismatch { named, date: base })
            }
            _ => SundayAfter::Date(first_sunday_after(base)),
        });
    }
    // The base's own leading weekday (round 7, R7-4):
    // strip it — and a filler "on" in front of it —
    // and read the date it framed. The date that
    // remains is the base; a weekday that disagrees
    // with it records the mismatch and yields no date,
    // and no later reading may salvage the base date.
    let (lead_word, lead_rest): (&str, &[&str]) = match core.first() {
        Some(first) if weekday_word(first).is_some() => (*first, &core[1..]),
        Some(first) if first.eq_ignore_ascii_case("on") => {
            let second = core.get(1)?;
            weekday_word(second)?;
            (*second, &core[2..])
        }
        _ => return None,
    };
    let lead_number = weekday_word(lead_word)?;
    let base = parse_flexible_date(&lead_rest.join(" "), today)?;
    if base.weekday() != lead_number {
        let named = lead_word
            .trim_matches(['(', ')', '[', ']', ',', '.'])
            .to_string();
        return Some(SundayAfter::Mismatch(WeekdayMismatch { named, date: base }));
    }
    // The leading weekday agreed (it can only have
    // blocked the whole reading by wearing "on"); the
    // trailing weekday still has its say.
    Some(match trailing {
        Some((named, named_number)) if base.weekday() != named_number => {
            SundayAfter::Mismatch(WeekdayMismatch { named, date: base })
        }
        _ => SundayAfter::Date(first_sunday_after(base)),
    })
}

/// A For Sunday line that names a Sunday in words
/// after a Sunday named in relative words: "the
/// Sunday after next Sunday", "the Sunday after this
/// Sunday" (round 6, RT6-9). Returns the line as
/// written (trimmed, closing punctuation off) when
/// that is the line's whole shape, so the review can
/// repeat the sender's own words; the base must be a
/// relative Sunday — an absolute base is
/// `sunday_after_outcome`'s shape, and anything more
/// is not this shape either.
fn sunday_after_relative_phrase(raw: &Option<String>) -> Option<String> {
    let text = raw.as_deref()?.trim();
    let words: Vec<String> = text
        .split_whitespace()
        .map(|w| {
            w.trim_matches(['(', ')', '[', ']', '.', ',', ';'])
                .to_ascii_lowercase()
        })
        .collect();
    // Leading garnishes compose in order (round 7,
    // R7-5): an optional "on" and/or "the" in front of
    // the shape changes nothing about the shape.
    let mut core: &[String] = &words[..];
    while matches!(core.first().map(String::as_str), Some("on" | "the")) {
        core = &core[1..];
    }
    // A trailing weekday garnish (round 7, R7-5) is
    // part of the sender's words, remembered with
    // them; without it the same shape must still read.
    let shape = |slice: &[String]| -> bool {
        let base: &[String] = match slice {
            [day, after, rest @ ..] if day.as_str() == "sunday" && after.as_str() == "after" => {
                rest
            }
            _ => return false,
        };
        // The relative base may itself wear a "the".
        let base: &[String] = match base.first().map(String::as_str) {
            Some("the") => &base[1..],
            _ => base,
        };
        match base {
            [which, base_day] => {
                (which.as_str() == "next" || which.as_str() == "this")
                    && base_day.as_str() == "sunday"
            }
            _ => false,
        }
    };
    let names_shape = shape(core)
        || (core.len() > 1
            && weekday_word(core.last().map(String::as_str).unwrap_or("")).is_some()
            && shape(&core[..core.len() - 1]));
    if names_shape {
        Some(text.trim_end_matches(['.', ',']).to_string())
    } else {
        None
    }
}

/// A For Sunday line that names its day only in
/// relative words (round 5, RT5-6): "next Sunday",
/// "this Sunday", "on Sunday", or a bare "Sunday".
/// Returns the line as written (trimmed, closing
/// punctuation off) when that is ALL the line says, so
/// the review can repeat the sender's own words; a
/// line saying anything more is not this shape.
fn relative_sunday_phrase(raw: &Option<String>) -> Option<String> {
    let text = raw.as_deref()?.trim();
    let words: Vec<String> = text
        .split_whitespace()
        .map(|w| {
            w.trim_matches(['(', ')', '[', ']', '.', ',', ';'])
                .to_ascii_lowercase()
        })
        .collect();
    // Leading garnishes compose in order (round 7,
    // R7-5): an optional "on" and/or "the".
    let mut core: &[String] = &words[..];
    while matches!(core.first().map(String::as_str), Some("on" | "the")) {
        core = &core[1..];
    }
    let shape = |slice: &[String]| -> bool {
        match slice {
            [day] => day.as_str() == "sunday",
            [which, day] => {
                (which.as_str() == "next" || which.as_str() == "this") && day.as_str() == "sunday"
            }
            _ => false,
        }
    };
    // A trailing weekday garnish (round 7, R7-5) is
    // remembered with the sender's words, as on the
    // Sunday-after shapes; it does not unname the
    // Sunday the words name.
    let names_sunday = shape(core)
        || (core.len() > 1
            && weekday_word(core.last().map(String::as_str).unwrap_or("")).is_some()
            && shape(&core[..core.len() - 1]));
    if names_sunday {
        Some(text.trim_end_matches(['.', ',']).to_string())
    } else {
        None
    }
}

/// A weekday wearing "on" directly in front of a
/// date frames that date wherever it stands in the
/// line (round 7, R7-3), and a leading framing weekday
/// and a trailing one on the same date are each
/// checked (round 7, R7-7). Returns the read date, or
/// the first disagreeing framing's mismatch, when a
/// framing weekday stands in one of those places and
/// the date it frames reads; a weekday with no "on"
/// away from the line's start frames nothing here and
/// stays ordinary text (round 5, R5-D).
fn framed_date_outcome(
    text: &str,
    today: SimpleDate,
) -> Option<(Option<SimpleDate>, Option<WeekdayMismatch>)> {
    fn clean(t: &str) -> &str {
        t.trim_matches(['(', ')', '[', ']', ',', '.'])
    }
    let tokens: Vec<&str> = text.split_whitespace().collect();
    let last = tokens
        .iter()
        .rposition(|t| t.chars().any(|c| c.is_alphanumeric()))?;
    let trailing_word = clean(tokens[last]);
    let trailing = weekday_word(trailing_word).map(|n| (trailing_word, n));
    let core: Vec<&str> = match trailing {
        Some(_) => tokens
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != last)
            .map(|(_, t)| *t)
            .collect(),
        None => tokens.clone(),
    };
    let mut lead: Option<(&str, u32, usize)> = None;
    if core.len() >= 2 && clean(core[0]).eq_ignore_ascii_case("on") {
        if let Some(n) = weekday_word(clean(core[1])) {
            lead = Some((clean(core[1]), n, 1));
        }
    }
    if lead.is_none() && !core.is_empty() {
        if let Some(n) = weekday_word(clean(core[0])) {
            lead = Some((clean(core[0]), n, 0));
        }
    }
    if lead.is_none() {
        for i in 1..core.len() {
            if clean(core[i - 1]).eq_ignore_ascii_case("on") {
                if let Some(n) = weekday_word(clean(core[i])) {
                    lead = Some((clean(core[i]), n, i));
                    break;
                }
            }
        }
    }
    let (lead_word, lead_number, lead_idx) = lead?;
    let date = parse_flexible_date(&core[lead_idx + 1..].join(" "), today)?;
    if date.weekday() != lead_number {
        let named = lead_word.to_string();
        return Some((None, Some(WeekdayMismatch { named, date })));
    }
    if let Some((trailing_word, trailing_number)) = trailing {
        if date.weekday() != trailing_number {
            let named = trailing_word.to_string();
            return Some((None, Some(WeekdayMismatch { named, date })));
        }
    }
    Some((Some(date), None))
}

/// Read one of a contribution's two date lines: the
/// date when the line reads, and the mismatch record
/// when the line named a weekday its date does not
/// fall on. A weekday word speaks about the date only
/// when it directly frames it (round 4, R4-7; round
/// 5, R5-D): first, as the Sunday line has always
/// allowed, or as the line's last word after the date,
/// in the way an office writes "October 25, 2026
/// (Monday)". A weekday word anywhere else is
/// ordinary text: it records no mismatch, and a date
/// it stands directly in front of still reads. A date
/// that does not read yields None for the date, never
/// a guess.
fn date_line(
    raw: &Option<String>,
    today: SimpleDate,
) -> (Option<SimpleDate>, Option<WeekdayMismatch>) {
    let Some(text) = raw.as_deref().map(str::trim).filter(|s| !s.is_empty()) else {
        return (None, None);
    };
    if let Some(d) = parse_flexible_date(text, today) {
        return (Some(d), None);
    }
    match sunday_after_outcome(text, today) {
        Some(SundayAfter::Date(d)) => return (Some(d), None),
        Some(SundayAfter::Mismatch(m)) => return (None, Some(m)),
        None => {}
    }
    if let Some(m) = named_weekday_mismatch(raw, today) {
        return (None, Some(m));
    }
    // Framing composed (round 7, R7-3/R7-7): a weekday
    // wearing "on" anywhere in the line, and a leading
    // weekday standing with a trailing one, are checked
    // before the plainer readings below get their turn.
    if let Some(outcome) = framed_date_outcome(text, today) {
        return outcome;
    }
    let tokens: Vec<&str> = text.split_whitespace().collect();
    // A weekday written as the line's last word, right
    // after the date (punctuation may wrap it, as in
    // "(Monday)"): take the word out and read what is
    // left. If what is left reads as the date the word
    // names, the line was fine all along; if it reads
    // as a different day, the word was the mistake, and
    // the record says so.
    let last_word = tokens
        .iter()
        .rposition(|t| t.chars().any(|c| c.is_alphanumeric()));
    if let Some(i) = last_word {
        let word = tokens[i].trim_matches(['(', ')', '[', ']', ',', '.']);
        if let Some(named_number) = weekday_word(word) {
            let rest: Vec<&str> = tokens
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .map(|(_, t)| *t)
                .collect();
            if let Some(date) = parse_flexible_date(&rest.join(" "), today) {
                if date.weekday() == named_number {
                    return (Some(date), None);
                }
                return (
                    None,
                    Some(WeekdayMismatch {
                        named: word.to_string(),
                        date,
                    }),
                );
            }
        }
    }
    // A weekday word standing directly in front of a
    // date that ends the line, with other words before
    // it ("bring a friend Friday, October 25, 2026"),
    // is ordinary text about those other words, not a
    // claim about the date: the date reads, and no
    // mismatch is recorded. (A weekday in the line's
    // first position is the claim case, handled above
    // by `named_weekday_mismatch`.)
    for i in 1..tokens.len() {
        let word = tokens[i].trim_matches(['(', ')', '[', ']', ',', '.']);
        if weekday_word(word).is_none() {
            continue;
        }
        if let Some(date) = parse_flexible_date(&tokens[i + 1..].join(" "), today) {
            return (Some(date), None);
        }
    }
    (None, None)
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
    let (for_date, named_weekday_mismatch) = date_line(&raw.for_sunday, today);
    let (keep_through, keep_through_weekday_mismatch) = date_line(&raw.keep_through, today);
    // A Sunday named only in relative words leaves no
    // date and no mismatch, but it is not silence: the
    // words are remembered as written (round 5, RT5-6).
    let for_sunday_in_words = if for_date.is_none() && named_weekday_mismatch.is_none() {
        relative_sunday_phrase(&raw.for_sunday)
    } else {
        None
    };
    // A Sunday named in words after a relative Sunday
    // (round 6, RT6-9) is remembered the same way, in
    // its own field: the verdict resolves it against
    // the open bulletin, and with no bulletin the
    // review's sentence can still admit the file named
    // a Sunday in words after another Sunday.
    let for_sunday_after_in_words = if for_date.is_none()
        && named_weekday_mismatch.is_none()
        && for_sunday_in_words.is_none()
    {
        sunday_after_relative_phrase(&raw.for_sunday)
    } else {
        None
    };
    Ok(Contribution {
        from: raw.from.filter(|s| !s.trim().is_empty()),
        for_date,
        keep_through,
        parts,
        named_weekday_mismatch,
        keep_through_weekday_mismatch,
        for_sunday_in_words,
        for_sunday_after_in_words,
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
    /// The words name a Sunday only in relative words
    /// ("next Sunday"); read against the open bulletin,
    /// they name its Sunday, and the review says that is
    /// what it did (round 5, RT5-6).
    SundayInWords(SimpleDate),
}

/// The wrong-week guard's verdict (roles synthesis: a part
/// can be held for its own Sunday, never lands silently).
pub fn week_verdict(c: &Contribution, bulletin: Option<SimpleDate>) -> WeekVerdict {
    match (c.for_date, bulletin) {
        (Some(said), Some(open)) if said != open => WeekVerdict::DifferentSunday(said),
        (Some(_), Some(_)) => WeekVerdict::SameSunday,
        // Round 6 (RT6-9): a Sunday named in words after
        // a relative Sunday resolves against the open
        // bulletin — the relative Sunday reads as the
        // bulletin's date, as a bare relative Sunday
        // does, and the named Sunday is the first Sunday
        // strictly after it, so it always differs from
        // the open bulletin and arms like any named
        // Sunday that does.
        (None, Some(open)) if c.for_sunday_after_in_words.is_some() => {
            WeekVerdict::DifferentSunday(first_sunday_after(open))
        }
        (None, Some(open)) if c.for_sunday_in_words.is_some() => WeekVerdict::SundayInWords(open),
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
    fn a_wrong_weekday_is_named_not_mistaken_for_no_sunday() {
        // Round 3 (RT3-D): 2026-10-25 is a Sunday. A line
        // calling it Monday names an impossible day, and
        // the parse says so, instead of the review having
        // to call the file dateless.
        let wrong = "From: Sue\nFor Sunday: Monday, October 25, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(wrong, today()).expect("parses");
        assert_eq!(c.for_date, None, "the date itself still does not read");
        let m = c
            .named_weekday_mismatch
            .expect("the wrong weekday is recorded");
        assert_eq!(m.named, "Monday");
        assert_eq!(m.date, oct(25));
        assert_eq!(
            m.sentence(),
            "This part names Monday, October 25, 2026, but that date is a \
             Sunday. Check the date with the sender."
        );
        // A correct weekday records no mismatch, and a
        // line with no date in it records none either.
        let right = "From: Sue\nFor Sunday: Sunday, October 25, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(right, today()).expect("parses");
        assert_eq!(c.for_date, Some(oct(25)));
        assert_eq!(c.named_weekday_mismatch, None);
        let none = parse_contribution("## Announcement\n\nHi.\n", today()).expect("parses");
        assert_eq!(none.named_weekday_mismatch, None);
        let vague = "For Sunday: soon\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(vague, today()).expect("parses");
        assert_eq!(c.for_date, None);
        assert_eq!(
            c.named_weekday_mismatch, None,
            "an unreadable line is not a wrong-weekday line"
        );
    }

    #[test]
    fn a_wrong_weekday_on_the_keep_through_line_is_named_too() {
        // Round 4 (R4-7 / RT-5d): 2026-11-30 is a
        // Monday. A keep-through line calling it Friday
        // used to drop the date in silence, and the
        // announcement ended a week early; now the parse
        // records the mistake and applies no date.
        let wrong =
            "From: Sue\nKeep-through: Friday, November 30, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(wrong, today()).expect("parses");
        assert_eq!(c.keep_through, None, "the date does not apply");
        let m = c
            .keep_through_weekday_mismatch
            .expect("the wrong weekday is recorded");
        assert_eq!(m.named, "Friday");
        assert_eq!(
            m.date,
            SimpleDate {
                year: 2026,
                month: 11,
                day: 30
            }
        );
        assert_eq!(
            m.keep_through_sentence(),
            "This part says keep through Friday, November 30, 2026, but that date is a \
             Monday. Check the date with the sender."
        );
        // A correct keep-through weekday reads as before.
        let right =
            "From: Sue\nKeep-through: Monday, November 30, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(right, today()).expect("parses");
        assert_eq!(
            c.keep_through,
            Some(SimpleDate {
                year: 2026,
                month: 11,
                day: 30
            })
        );
        assert_eq!(c.keep_through_weekday_mismatch, None);
    }

    #[test]
    fn a_weekday_written_after_the_date_is_read_and_checked() {
        // Round 4 (R4-7): the weekday does not have to
        // come first. "October 25, 2026 (Monday)" names
        // it last; 2026-10-25 is a Sunday, so the word
        // is the mistake. Written correctly, in parens
        // or after a comma, the line parses normally.
        let wrong = "From: Sue\nFor Sunday: October 25, 2026 (Monday)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(wrong, today()).expect("parses");
        assert_eq!(c.for_date, None);
        let m = c
            .named_weekday_mismatch
            .expect("the trailing wrong weekday is recorded");
        assert_eq!(m.named, "Monday");
        assert_eq!(m.date, oct(25));
        assert_eq!(
            m.sentence(),
            "This part names Monday, October 25, 2026, but that date is a \
             Sunday. Check the date with the sender."
        );
        let comma = "From: Sue\nFor Sunday: October 25, 2026, Monday\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(comma, today()).expect("parses");
        assert_eq!(c.for_date, None);
        assert!(c.named_weekday_mismatch.is_some());
        let right = "From: Sue\nFor Sunday: October 25, 2026 (Sunday)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(right, today()).expect("parses");
        assert_eq!(c.for_date, Some(oct(25)));
        assert_eq!(c.named_weekday_mismatch, None);
    }

    #[test]
    fn the_sunday_after_a_named_date_reads_as_that_sunday() {
        // Round 5 (RT5-6): "the Sunday after October 25,
        // 2026" named no date the parser could use.
        // October 25, 2026 is itself a Sunday, and
        // "after" is strict, so the words name the NEXT
        // Sunday, November 1. A named weekday date in
        // the middle of the week (Wednesday, October
        // 21) names the Sunday of that same week, and
        // the named date may carry its own weekday or
        // arrive in numeric form.
        let text =
            "From: Sue\nFor Sunday: the Sunday after October 25, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(
            c.for_date,
            Some(SimpleDate {
                year: 2026,
                month: 11,
                day: 1
            })
        );
        assert_eq!(c.named_weekday_mismatch, None);
        assert_eq!(c.for_sunday_in_words, None);
        let text = "For Sunday: Sunday after Sunday, October 25, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(
            c.for_date,
            Some(SimpleDate {
                year: 2026,
                month: 11,
                day: 1
            }),
            "a named date that IS a Sunday steps a whole week"
        );
        let text = "For Sunday: Sunday after October 21, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(oct(25)));
        let text = "For Sunday: Sunday after 10/21/26\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(oct(25)));
        // The verdict machinery treats the read date
        // like any other: it arms against a different
        // open Sunday and matches its own.
        let text = "For Sunday: the Sunday after October 21, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(week_verdict(&c, Some(oct(25))), WeekVerdict::SameSunday);
        assert_eq!(
            week_verdict(&c, Some(oct(18))),
            WeekVerdict::DifferentSunday(oct(25))
        );
    }

    #[test]
    fn an_on_before_a_framing_weekday_records_its_mismatch() {
        // Round 6 (R6-6): "on Monday, October 25, 2026"
        // used to drop its weekday silently and read
        // the date clean, where the bare leading form
        // ("Monday, October 25, 2026") records the
        // mismatch. The filler word "on" in front of a
        // framing weekday changes nothing: October 25,
        // 2026 is a Sunday. Both date lines, the shared
        // helper.
        let text = "From: Sue\nFor Sunday: on Monday, October 25, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, None, "the date itself still does not read");
        let m = c
            .named_weekday_mismatch
            .expect("the wrong weekday behind \"on\" is recorded");
        assert_eq!(m.named, "Monday");
        assert_eq!(m.date, oct(25));
        assert_eq!(
            m.sentence(),
            "This part names Monday, October 25, 2026, but that date is a \
             Sunday. Check the date with the sender."
        );
        let text =
            "From: Sue\nKeep-through: on Monday, October 25, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.keep_through, None);
        let m = c
            .keep_through_weekday_mismatch
            .expect("the keep-through line records it too");
        assert_eq!(m.named, "Monday");
        assert_eq!(m.date, oct(25));
        // An agreeing weekday behind "on" reads clean.
        let text = "From: Sue\nFor Sunday: on Sunday, October 25, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(oct(25)));
        assert_eq!(c.named_weekday_mismatch, None);
    }

    #[test]
    fn the_sunday_after_composes_with_a_trailing_weekday() {
        // Round 6 (RT6-8): a trailing weekday on a
        // "Sunday after [date]" line describes the base
        // date it follows. An agreeing one resolves
        // (Sunday after October 25, 2026 is November 1);
        // a disagreeing one records the mismatch against
        // the base date and yields no date, on both
        // lines.
        let nov1 = SimpleDate {
            year: 2026,
            month: 11,
            day: 1,
        };
        let text = "From: Sue\nFor Sunday: Sunday after October 25, 2026 (Sunday)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(nov1));
        assert_eq!(c.named_weekday_mismatch, None);
        let text = "From: Sue\nFor Sunday: Sunday after October 25, 2026 (Monday)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(
            c.for_date, None,
            "a disagreeing trailing weekday yields no date"
        );
        let m = c
            .named_weekday_mismatch
            .expect("the trailing weekday's mistake is recorded");
        assert_eq!(m.named, "Monday");
        assert_eq!(m.date, oct(25), "the weekday describes the base date");
        assert_eq!(
            m.sentence(),
            "This part names Monday, October 25, 2026, but that date is a \
             Sunday. Check the date with the sender."
        );
        let text = "From: Sue\nKeep-through: the Sunday after October 25, 2026 (Monday)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.keep_through, None);
        let m = c
            .keep_through_weekday_mismatch
            .expect("the keep-through line records it too");
        assert_eq!(m.named, "Monday");
        assert_eq!(m.date, oct(25));
        let text = "From: Sue\nKeep-through: the Sunday after October 25, 2026 (Sunday)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.keep_through, Some(nov1));
        assert_eq!(c.keep_through_weekday_mismatch, None);
    }

    #[test]
    fn a_sunday_after_a_relative_sunday_is_remembered_and_resolved_by_the_verdict() {
        // Round 6 (RT6-9): "the Sunday after next
        // Sunday" used to fall into NoSundayNamed with
        // the file misdescribed as naming no Sunday.
        // The shape is remembered in the sender's words;
        // the verdict resolves it against the open
        // bulletin (the relative Sunday reads as the
        // bulletin's date, then the first Sunday
        // strictly after it) and arms it like any named
        // Sunday that differs. October 25, 2026 is a
        // Sunday; the Sunday after it is November 1.
        let nov1 = SimpleDate {
            year: 2026,
            month: 11,
            day: 1,
        };
        for line in [
            "the Sunday after next Sunday",
            "the Sunday after this Sunday",
            "Sunday after next Sunday",
        ] {
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, None, "{line}: no date in the words alone");
            assert_eq!(c.named_weekday_mismatch, None, "{line}");
            assert_eq!(c.for_sunday_in_words, None, "{line}");
            assert_eq!(
                c.for_sunday_after_in_words.as_deref(),
                Some(line),
                "{line}: the shape is remembered as written"
            );
            assert_eq!(
                week_verdict(&c, Some(oct(25))),
                WeekVerdict::DifferentSunday(nov1),
                "{line}: resolved against the open bulletin, and armed"
            );
            assert_eq!(
                week_verdict(&c, Some(oct(18))),
                WeekVerdict::DifferentSunday(oct(25)),
                "{line}: the Sunday after October 18's reading is October 25"
            );
            assert_eq!(
                week_verdict(&c, None),
                WeekVerdict::NoSundayNamed,
                "{line}: no bulletin date to read against"
            );
        }
        // A bare relative Sunday keeps its round-5
        // behavior exactly: remembered in the other
        // field, read as the open bulletin's Sunday.
        let text = "For Sunday: next Sunday\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_sunday_after_in_words, None);
        assert_eq!(c.for_sunday_in_words.as_deref(), Some("next Sunday"));
        assert_eq!(
            week_verdict(&c, Some(oct(25))),
            WeekVerdict::SundayInWords(oct(25))
        );
    }

    #[test]
    fn a_sunday_named_only_in_words_is_remembered_and_read_against_the_bulletin() {
        // Round 5 (RT5-6): "next Sunday" used to read as
        // no Sunday named, and the review misdescribed
        // the file. The words are remembered as written;
        // with a bulletin open, the verdict reads them
        // as that bulletin's Sunday; with none, the
        // verdict stays NoSundayNamed but the words are
        // still on the parse for the review's sentence.
        for line in ["next Sunday", "this Sunday", "on Sunday", "Sunday"] {
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, None, "{line}: no date in the words alone");
            assert_eq!(c.named_weekday_mismatch, None, "{line}");
            assert_eq!(
                c.for_sunday_in_words.as_deref(),
                Some(line),
                "{line}: the sender's words are remembered"
            );
            assert_eq!(
                week_verdict(&c, Some(oct(25))),
                WeekVerdict::SundayInWords(oct(25)),
                "{line}: read as the open bulletin's Sunday"
            );
            assert_eq!(
                week_verdict(&c, None),
                WeekVerdict::NoSundayNamed,
                "{line}: no bulletin date to read against"
            );
        }
        // Words beyond the relative Sunday are not this
        // shape, and an absolute date leaves no marker.
        let text = "For Sunday: next Sunday morning\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_sunday_in_words, None);
        let text = "For Sunday: October 25, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(oct(25)));
        assert_eq!(c.for_sunday_in_words, None);
    }

    #[test]
    fn a_weekday_word_elsewhere_in_the_line_is_ordinary_text() {
        // Round 5 (R5-D, review M4): the mismatch
        // sentence speaks only when the weekday word
        // directly frames the date. In "bring a friend
        // Friday, October 25, 2026" the Friday belongs
        // to the friend, not the date: the date reads
        // and no mismatch is recorded, on either date
        // line. October 25, 2026 is a Sunday.
        let text =
            "For Sunday: bring a friend Friday, October 25, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(oct(25)));
        assert_eq!(c.named_weekday_mismatch, None);
        let text =
            "Keep-through: bring a friend Friday, October 25, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.keep_through, Some(oct(25)));
        assert_eq!(c.keep_through_weekday_mismatch, None);
        // A weekday word buried inside the date's own
        // tokens frames nothing either: no mismatch is
        // recorded (and the line still yields no date).
        let text = "For Sunday: October Monday 25, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, None);
        assert_eq!(c.named_weekday_mismatch, None);
        // The genuinely adjacent cases still speak, on
        // both lines: a leading weekday is a claim about
        // the date, and so is a trailing one.
        let text = "For Sunday: Friday, October 25, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, None);
        let m = c.named_weekday_mismatch.expect("the leading Friday speaks");
        assert_eq!(m.named, "Friday");
        assert_eq!(m.date, oct(25));
        let text = "Keep-through: Friday, October 25, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.keep_through, None);
        let m = c
            .keep_through_weekday_mismatch
            .expect("the leading Friday speaks on this line too");
        assert_eq!(m.named, "Friday");
        assert_eq!(m.date, oct(25));
        let text = "Keep-through: October 25, 2026 (Monday)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.keep_through, None);
        let m = c
            .keep_through_weekday_mismatch
            .expect("the trailing Monday speaks on this line too");
        assert_eq!(m.named, "Monday");
        assert_eq!(m.date, oct(25));
    }

    #[test]
    fn r7c_an_on_weekday_mid_sentence_records_its_mismatch() {
        // Round 7 (R7-3 / RT7-3): "use on Monday,
        // October 25, 2026" used to land the date clean
        // — no mismatch, SameSunday unarmed against the
        // October 25 bulletin — because the "on" framing
        // was only recognized at the line's start.
        // October 25, 2026 is a Sunday; Monday, October
        // 25 does not exist. A weekday wearing "on"
        // frames the date wherever it stands, on both
        // lines.
        let text =
            "From: Sue\nFor Sunday: use on Monday, October 25, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, None, "the impossible day does not read");
        assert_eq!(
            week_verdict(&c, Some(oct(25))),
            WeekVerdict::NoSundayNamed,
            "no longer an unarmed SameSunday against October 25"
        );
        let m = c
            .named_weekday_mismatch
            .expect("the mid-sentence \"on\" weekday is recorded");
        assert_eq!(m.named, "Monday");
        assert_eq!(m.date, oct(25));
        let text =
            "From: Sue\nKeep-through: use on Monday, October 25, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.keep_through, None);
        let m = c
            .keep_through_weekday_mismatch
            .expect("the keep-through line records it too");
        assert_eq!(m.named, "Monday");
        assert_eq!(m.date, oct(25));
        // An agreeing mid-sentence "on" weekday reads
        // clean, exactly as the leading form does.
        let text = "For Sunday: use on Sunday, October 25, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(oct(25)));
        assert_eq!(c.named_weekday_mismatch, None);
        // Standing, by the round-7 disposition: red
        // team's D5b ("please note Monday, ...", no
        // "on") stays under the R5-D ordinary-text rule
        // — the weekday belongs to the sentence's other
        // words, the date reads, and no mismatch is
        // recorded. Recorded for round 8 to re-probe.
        let text = "For Sunday: please note Monday, October 25, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(oct(25)), "D5b stands as shipped");
        assert_eq!(c.named_weekday_mismatch, None, "D5b stands as shipped");
    }

    #[test]
    fn r7c_a_sunday_after_base_with_a_wrong_weekday_yields_no_date() {
        // Round 7 (R7-4 / RT7-4): "Sunday after
        // Tuesday, October 26, 2026" (October 26 is a
        // Monday) used to yield for_date = October 26
        // itself, no mismatch: the base's wrong weekday
        // refused the Sunday-after reading, and the
        // mid-sentence rule then salvaged the bare base
        // date. A date inside a "Sunday after ..." phrase
        // is the base, never the line's own date, on
        // both lines.
        for line in [
            "Sunday after Tuesday, October 26, 2026",
            "the Sunday after Tuesday, October 26, 2026",
        ] {
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, None, "{line}: the base is not the date");
            let m = c
                .named_weekday_mismatch
                .expect("the base's wrong weekday is recorded");
            assert_eq!(m.named, "Tuesday", "{line}");
            assert_eq!(m.date, oct(26), "{line}: the base date");
        }
        let text =
            "Keep-through: Sunday after Tuesday, October 26, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.keep_through, None);
        let m = c
            .keep_through_weekday_mismatch
            .expect("the keep-through line records it too");
        assert_eq!(m.named, "Tuesday");
        assert_eq!(m.date, oct(26));
    }

    #[test]
    fn r7c_the_sunday_after_reader_composes_its_garnishes() {
        // Round 7 (R7-5 / review M3, RT7-7 D9): "on
        // Sunday after October 25, 2026" and "on the
        // Sunday after ..." used to fall to a silent
        // NoSundayNamed under the false sentence "These
        // words do not say which Sunday they are for".
        // The date is fully determined: November 1,
        // 2026, and it arms like any named Sunday.
        let nov1 = SimpleDate {
            year: 2026,
            month: 11,
            day: 1,
        };
        for line in [
            "on Sunday after October 25, 2026",
            "on the Sunday after October 25, 2026",
        ] {
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, Some(nov1), "{line}");
            assert_eq!(c.named_weekday_mismatch, None, "{line}");
            assert_eq!(c.for_sunday_in_words, None, "{line}");
            assert_eq!(c.for_sunday_after_in_words, None, "{line}");
            assert_eq!(
                week_verdict(&c, Some(nov1)),
                WeekVerdict::SameSunday,
                "{line}"
            );
            assert_eq!(
                week_verdict(&c, Some(oct(25))),
                WeekVerdict::DifferentSunday(nov1),
                "{line}: armed against a different open Sunday"
            );
        }
        let text = "Keep-through: on the Sunday after October 25, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.keep_through, Some(nov1));
        assert_eq!(c.keep_through_weekday_mismatch, None);
        // The trailing weekday composes with the
        // leading garnishes too, against the base
        // (RT6-8's rule): agreeing resolves, disagreeing
        // records the mismatch against the base.
        let text =
            "For Sunday: on Sunday after October 25, 2026 (Sunday)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(nov1));
        let text =
            "For Sunday: on Sunday after October 25, 2026 (Monday)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, None);
        let m = c
            .named_weekday_mismatch
            .expect("the trailing weekday's mistake is recorded");
        assert_eq!(m.named, "Monday");
        assert_eq!(m.date, oct(25), "against the base date");
    }

    #[test]
    fn r7c_relative_shapes_keep_their_words_with_a_trailing_weekday() {
        // Round 7 (R7-5 / RT7-6): "the Sunday after
        // next Sunday (Monday)" and "next Sunday
        // (Monday)" used to die on the garnish — silent
        // NoSundayNamed, the words remembered nowhere.
        // The shapes keep their in-words memory WITH
        // the garnish attached, and the verdicts read
        // exactly as the ungarnished shapes' do.
        let nov1 = SimpleDate {
            year: 2026,
            month: 11,
            day: 1,
        };
        for line in [
            "the Sunday after next Sunday (Monday)",
            "on the Sunday after next Sunday",
            "the Sunday after the next Sunday",
        ] {
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, None, "{line}: no date in the words alone");
            assert_eq!(c.named_weekday_mismatch, None, "{line}");
            assert_eq!(c.for_sunday_in_words, None, "{line}");
            assert_eq!(
                c.for_sunday_after_in_words.as_deref(),
                Some(line),
                "{line}: remembered as written, garnish attached"
            );
            assert_eq!(
                week_verdict(&c, Some(oct(25))),
                WeekVerdict::DifferentSunday(nov1),
                "{line}: resolved against the open bulletin, and armed"
            );
            assert_eq!(
                week_verdict(&c, None),
                WeekVerdict::NoSundayNamed,
                "{line}: no bulletin date to read against"
            );
        }
        let text = "For Sunday: next Sunday (Monday)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, None);
        assert_eq!(c.named_weekday_mismatch, None);
        assert_eq!(
            c.for_sunday_in_words.as_deref(),
            Some("next Sunday (Monday)"),
            "the bare relative shape remembers the garnish too"
        );
        assert_eq!(c.for_sunday_after_in_words, None);
        assert_eq!(
            week_verdict(&c, Some(oct(25))),
            WeekVerdict::SundayInWords(oct(25))
        );
        assert_eq!(week_verdict(&c, None), WeekVerdict::NoSundayNamed);
    }

    #[test]
    fn r7c_leading_and_trailing_weekdays_are_each_checked() {
        // Round 7 (R7-7 / RT7-7): combined, a leading
        // and a trailing weekday used to silence each
        // other — no date and no mismatch, though each
        // single shape records its own. Each framing is
        // checked; a disagreeing one records its
        // mismatch even when no date results, on both
        // lines. October 25, 2026 is a Sunday.
        let text = "For Sunday: on Monday, October 25, 2026 (Sunday)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, None);
        let m = c
            .named_weekday_mismatch
            .expect("the disagreeing leading weekday is recorded");
        assert_eq!(m.named, "Monday");
        assert_eq!(m.date, oct(25));
        let text = "For Sunday: on Sunday, October 25, 2026 (Monday)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, None);
        let m = c
            .named_weekday_mismatch
            .expect("the disagreeing trailing weekday is recorded");
        assert_eq!(m.named, "Monday");
        assert_eq!(m.date, oct(25));
        let text = "Keep-through: on Monday, October 25, 2026 (Sunday)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.keep_through, None);
        let m = c
            .keep_through_weekday_mismatch
            .expect("the keep-through line records it too");
        assert_eq!(m.named, "Monday");
        assert_eq!(m.date, oct(25));
        // Both framings agreeing reads the date clean.
        let text = "For Sunday: on Sunday, October 25, 2026 (Sunday)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(oct(25)));
        assert_eq!(c.named_weekday_mismatch, None);
    }

    #[test]
    fn r7c_agreeing_controls_still_stand() {
        // The controls the round-7 probes resolved
        // correctly stay resolved (red team RT7-4's
        // agreeing base, RT7-6's ungarnished relative
        // shapes, and the R5-D ordinary-text rule).
        let nov1 = SimpleDate {
            year: 2026,
            month: 11,
            day: 1,
        };
        let text = "For Sunday: Sunday after October 25, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(nov1));
        assert_eq!(c.named_weekday_mismatch, None);
        let text = "For Sunday: Sunday after Monday, October 26, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(
            c.for_date,
            Some(nov1),
            "an agreeing base weekday still resolves to November 1"
        );
        assert_eq!(c.named_weekday_mismatch, None);
        let text =
            "For Sunday: bring a friend Friday, October 25, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(oct(25)), "ordinary text reads clean");
        assert_eq!(c.named_weekday_mismatch, None);
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
