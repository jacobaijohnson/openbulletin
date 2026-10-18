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
//!     filler word "on" — and an optional "the" with it —
//!     wherever it stands in the line, every framing
//!     candidate is tried until one frames a date, words
//!     after the framed date are the line's remainder,
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
//!     garnish and all, with any trailing parenthesized
//!     absolute date remembered beside the words for
//!     the verdict to compare once a bulletin anchors it
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
    /// The absolute date a relative Sunday phrase
    /// carried in a trailing parenthesis (round 8,
    /// carried shape): "the Sunday after next Sunday
    /// (October 25)", "next Sunday (October 25, 2026)".
    /// The phrase is remembered in words as before;
    /// this is the date its parenthesis names, when
    /// the parenthesis reads as a date at all — one
    /// that does not (a weekday garnish, "(Monday)")
    /// is garnish, not memory, and leaves this None.
    /// The verdict compares the date with the relative
    /// resolution once an open bulletin anchors it.
    pub for_sunday_parenthesized_date: Option<SimpleDate>,
    /// Every trailing parenthesized date group on a
    /// relative Sunday phrase, in the order written
    /// (round 9, R9-10): stacked groups are all
    /// remembered, not only the last, so a disagreeing
    /// earlier group cannot be laundered by an agreeing
    /// later one. `for_sunday_parenthesized_date` above
    /// stays the last group, as it has been.
    pub for_sunday_parenthesized_dates: Vec<SimpleDate>,
    /// A trailing parenthesized date on an absolute For
    /// Sunday line that disagrees with the date the
    /// line itself read (round 9, R9-12): the
    /// parenthesis composes as garnish — agreeing
    /// stands clean, disagreeing is recorded here, and
    /// it never kills the plain reading.
    pub for_sunday_parenthesized_mismatch: Option<ParenthesizedDateMismatch>,
    /// The same record for the Keep-through line
    /// (round 9, R9-9/R9-12): a parenthesized date that
    /// disagrees with the keep-through date the line
    /// read, or with the parenthesized date that stands
    /// when stacked groups on that line disagree.
    pub keep_through_parenthesized_mismatch: Option<ParenthesizedDateMismatch>,
    /// A second parseable absolute date standing in the
    /// For Sunday line after the date that was read
    /// (round 9, R9-13): the read date still governs,
    /// and the review notes the other date so the
    /// office can check which one was meant.
    pub for_sunday_second_date: Option<SimpleDate>,
    /// The same note for the Keep-through line.
    pub keep_through_second_date: Option<SimpleDate>,
    /// Every distinct second date standing in the For
    /// Sunday line, in written order (round 10, R10-4 /
    /// R10-6): the scan reads the whole line, before
    /// the read date as well as after it, and the
    /// review's note names each one. `for_sunday_second_date`
    /// above stays the first of these.
    pub for_sunday_second_dates: Vec<SimpleDate>,
    /// The same list for the Keep-through line.
    pub keep_through_second_dates: Vec<SimpleDate>,
}

/// A date line whose trailing parenthesized date
/// disagrees with the date the line itself read
/// (round 9, R9-12): "October 25, 2026 (November 1)".
/// The line's own date governs; this record keeps both
/// dates so the review can name the disagreement
/// instead of dropping the parenthesis in silence.
#[derive(Debug, Clone, PartialEq)]
pub struct ParenthesizedDateMismatch {
    /// The first disagreeing date a parenthesis names,
    /// in written order.
    pub named: SimpleDate,
    /// Every other distinct disagreeing date a
    /// parenthesis names, in written order (round 10,
    /// R10-6): stacked groups are all named in the
    /// review's sentence, not only the first.
    pub also_named: Vec<SimpleDate>,
    /// The date the line itself read (on a Sunday-after
    /// line, the resolution it names).
    pub read: SimpleDate,
    /// The read date itself came from a parenthesized
    /// group (round 10, R10-6): the standing date on a
    /// Keep-through line whose own words yielded no
    /// date, or a line that is only parenthesized
    /// groups. The sentences then say the dates in
    /// parentheses disagree with each other, instead of
    /// framing a plain reading against a parenthesis
    /// that never stood.
    pub read_from_parentheses: bool,
}

impl ParenthesizedDateMismatch {
    /// Every distinct disagreeing date, in written
    /// order: `named` first, then `also_named`.
    fn named_dates(&self) -> Vec<SimpleDate> {
        let mut out: Vec<SimpleDate> = vec![self.named];
        for d in &self.also_named {
            if !out.contains(d) {
                out.push(*d);
            }
        }
        out
    }

    /// The review's line for the For Sunday shape.
    pub fn sentence(&self) -> String {
        if self.read_from_parentheses {
            let mut all = self.named_dates();
            if !all.contains(&self.read) {
                all.push(self.read);
            }
            let listed = all
                .iter()
                .map(|d| format_date_short(*d))
                .collect::<Vec<String>>()
                .join(" and ");
            let read = format_date_short(self.read);
            return format!(
                "The dates in parentheses do not agree with each other: {listed}. \
                 This part is read as naming {read}. \
                 Check the date with the sender."
            );
        }
        if self.also_named.is_empty() {
            let read = format_date_short(self.read);
            let named = format_date_short(self.named);
            return format!(
                "This part names {read}, but the date in parentheses is {named}. \
                 Check the date with the sender."
            );
        }
        let read = format_date_short(self.read);
        let listed = self
            .named_dates()
            .iter()
            .map(|d| format_date_short(*d))
            .collect::<Vec<String>>()
            .join(" and ");
        format!(
            "This part names {read}, but the dates in parentheses are {listed}. \
             Check the date with the sender."
        )
    }

    /// The same sentence for the Keep-through line.
    pub fn keep_through_sentence(&self) -> String {
        if self.read_from_parentheses {
            let mut all = self.named_dates();
            if !all.contains(&self.read) {
                all.push(self.read);
            }
            let listed = all
                .iter()
                .map(|d| format_date_short(*d))
                .collect::<Vec<String>>()
                .join(" and ");
            let read = format_date_short(self.read);
            return format!(
                "The dates in parentheses do not agree with each other: {listed}. \
                 This part is read as keep through {read}. \
                 Check the date with the sender."
            );
        }
        if self.also_named.is_empty() {
            let read = format_date_short(self.read);
            let named = format_date_short(self.named);
            return format!(
                "This part says keep through {read}, but the date in parentheses is {named}. \
                 Check the date with the sender."
            );
        }
        let read = format_date_short(self.read);
        let listed = self
            .named_dates()
            .iter()
            .map(|d| format_date_short(*d))
            .collect::<Vec<String>>()
            .join(" and ");
        format!(
            "This part says keep through {read}, but the dates in parentheses are {listed}. \
             Check the date with the sender."
        )
    }
}

/// The mismatch record for a set of parenthesized
/// groups standing against a read date (round 10,
/// R10-6): every distinct group date that disagrees
/// with the read date is kept, in written order; no
/// record stands when every group agrees.
fn mismatch_from_groups(
    groups: &[SimpleDate],
    read: SimpleDate,
    read_from_parentheses: bool,
) -> Option<ParenthesizedDateMismatch> {
    let mut disagreeing: Vec<SimpleDate> = Vec::new();
    for g in groups {
        if *g != read && !disagreeing.contains(g) {
            disagreeing.push(*g);
        }
    }
    let mut it = disagreeing.into_iter();
    let named = it.next()?;
    let also_named: Vec<SimpleDate> = it.collect();
    Some(ParenthesizedDateMismatch {
        named,
        also_named,
        read,
        read_from_parentheses,
    })
}

/// The review's note when a For Sunday line named a
/// second absolute date after the one that was read
/// (round 9, R9-13): the read date governs, and the
/// note changes no verdict.
pub fn second_date_sentence(second: SimpleDate) -> String {
    second_dates_sentence(&[second])
}

/// The note when a For Sunday line named second dates
/// (round 10, R10-6): every distinct one is named.
pub fn second_dates_sentence(seconds: &[SimpleDate]) -> String {
    if seconds.len() == 1 {
        let second = format_date_long(seconds[0]);
        return format!("This file named another date too: {second}. Check which date was meant.");
    }
    let listed = seconds
        .iter()
        .map(|d| format_date_long(*d))
        .collect::<Vec<String>>()
        .join(" and ");
    format!("This file named other dates too: {listed}. Check which date was meant.")
}

/// The same note for the Keep-through line.
pub fn keep_through_second_date_sentence(second: SimpleDate) -> String {
    keep_through_second_dates_sentence(&[second])
}

/// The Keep-through note naming every distinct second
/// date (round 10, R10-6).
pub fn keep_through_second_dates_sentence(seconds: &[SimpleDate]) -> String {
    if seconds.len() == 1 {
        let second = format_date_long(seconds[0]);
        return format!(
            "The Keep-through line named another date too: {second}. \
             Check which date was meant."
        );
    }
    let listed = seconds
        .iter()
        .map(|d| format_date_long(*d))
        .collect::<Vec<String>>()
        .join(" and ");
    format!(
        "The Keep-through line named other dates too: {listed}. \
         Check which date was meant."
    )
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
    // Round 23 (R23-3): a weekday wearing a boundary
    // ending (colon, semicolon, slash — the one
    // boundary classification) still names its weekday,
    // as it already did wearing a comma or period.
    let w = word
        .trim_end_matches(['.', ',', ':', ';', '/'])
        .to_ascii_lowercase();
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
    // the bare leading form does. The garnish stack
    // composes (round 8, R8-6): an optional "the" may
    // stand between "on" and the weekday, or in front
    // of the weekday at the line's start, as the
    // Sunday-after reader has long allowed. Any other
    // words between the weekday and the date keep it
    // ordinary text (round 5, R5-D).
    let (named_word, rest) = if weekday_word(first).is_some() {
        (first, rest)
    } else if first.eq_ignore_ascii_case("on") || first.eq_ignore_ascii_case("the") {
        let (second, remainder) = rest.split_once(|c: char| c.is_whitespace())?;
        if weekday_word(second).is_some() {
            (second, remainder)
        } else if second.eq_ignore_ascii_case("the") {
            let (third, tail) = remainder.split_once(|c: char| c.is_whitespace())?;
            weekday_word(third)?;
            (third, tail)
        } else {
            return None;
        }
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
    let tokens: Vec<&str> = text.split_whitespace().collect();
    // The shape may stand anywhere in the line (round
    // 8, R8-4): words before it are the line's lead-in
    // ("words for Sunday after ..."), and its opening
    // garnishes are just lead-in words to walk past —
    // "on Sunday after ...", "on the Sunday after ...",
    // and the plain "the Sunday after ..." all name the
    // same shape (round 7, R7-5). Each "Sunday after"
    // candidate is tried in order until one reads.
    let mut start = 0;
    while start + 1 < tokens.len() {
        let first = tokens[start].trim_matches(['(', ')', '[', ']', ',', '.']);
        let second = tokens[start + 1].trim_matches(['(', ')', '[', ']', ',', '.']);
        if weekday_word(first) == Some(0) && second.eq_ignore_ascii_case("after") {
            if let Some(outcome) = sunday_after_base(&tokens[start + 2..], today) {
                return Some(outcome);
            }
        }
        start += 1;
    }
    None
}

/// Read the base date of a "Sunday after ..." shape:
/// the words after "after", read as the shape's base.
/// Words after the base date are the line's remainder,
/// not a parse failure (round 8, R8-4).
fn sunday_after_base(tokens: &[&str], today: SimpleDate) -> Option<SundayAfter> {
    // The base's pieces may compose (round 7,
    // R7-4/R7-5): an optional trailing weekday
    // describing the base, and an optional leading
    // weekday framing the base (wearing "on" or not).
    // Take the trailing weekday out first, then read
    // what is left; whichever framing weekday disagrees
    // with the base date is the mistake, and the base
    // date itself is never the line's date.
    let last_word = tokens
        .iter()
        .rposition(|t| t.chars().any(|c| c.is_alphanumeric()));
    let mut trailing: Option<(String, u32)> = None;
    let mut core: Vec<&str> = tokens.to_vec();
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
    if let Some((base, used)) = parse_date_prefix(&core, today) {
        // The trailing weekday describes the base only
        // when it stands directly after it; words
        // between the base and the trailing word make
        // that word part of the remainder.
        return Some(match trailing {
            Some((named, named_number)) if used == core.len() && base.weekday() != named_number => {
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
    let (base, used) = parse_date_prefix(lead_rest, today)?;
    if base.weekday() != lead_number {
        let named = lead_word
            .trim_matches(['(', ')', '[', ']', ',', '.'])
            .to_string();
        return Some(SundayAfter::Mismatch(WeekdayMismatch { named, date: base }));
    }
    // The leading weekday agreed (it can only have
    // blocked the whole reading by wearing "on"); the
    // trailing weekday still has its say, when it
    // stands directly after the base.
    Some(match trailing {
        Some((named, named_number))
            if used == lead_rest.len() && base.weekday() != named_number =>
        {
            SundayAfter::Mismatch(WeekdayMismatch { named, date: base })
        }
        _ => SundayAfter::Date(first_sunday_after(base)),
    })
}

/// The last trailing "(...)" group of `text`, split
/// as (the words before it, the group's own words).
/// The group's opening parenthesis is found by
/// balancing, so a nested group ("(note (November
/// 1))") is one trailing group, not a defect that
/// poisons the line it trails (round 11, R11-6).
fn split_last_trailing_group(text: &str) -> Option<(&str, &str)> {
    let text = text.trim_end_matches(['.', ',']);
    if !text.ends_with(')') {
        return None;
    }
    let bytes = text.as_bytes();
    let mut depth = 0usize;
    let mut open = None;
    for i in (0..bytes.len()).rev() {
        match bytes[i] {
            b')' => depth += 1,
            b'(' => {
                depth -= 1;
                if depth == 0 {
                    open = Some(i);
                    break;
                }
            }
            _ => {}
        }
    }
    let open = open?;
    Some((text[..open].trim_end(), &text[open + 1..text.len() - 1]))
}

/// `text` without a trailing "(...)" group, when it
/// ends with one: the words before the group.
fn strip_trailing_parenthesized(text: &str) -> Option<&str> {
    split_last_trailing_group(text).map(|(before, _)| before)
}

/// Whether `text` names a relative-Sunday shape,
/// allowing trailing parenthesized groups (round 8,
/// carried shape) to come off one by one until the
/// shape stands alone: a "(October 25)" after a
/// relative Sunday is garnish to the shape — the date
/// it names is remembered separately — the way a
/// trailing weekday garnish already is.
fn relative_shape_matches(text: &str, shape: fn(&str) -> bool) -> bool {
    // Round 23 (R23-3): the ONE boundary classification
    // at this site too — colon, semicolon, comma, dash
    // (incl. en/em), slash, and period are boundaries
    // exactly as the comma is, at BOTH ends of the
    // remainder: a separator standing where a group was
    // taken off, standing between two trailing groups,
    // or standing at a shape's own boundary ("next
    // Sunday: (November 8)", "(November 1) / next
    // Sunday", "next Sunday (November 8); (November 1)")
    // is separator punctuation, not words of the shape,
    // and the in-words memory survives it exactly as it
    // survives the space and comma twins.
    const BOUNDARY_TRIM: [char; 10] = [
        ' ', '\t', ';', ',', ':', '.', '/', '-', '\u{2013}', '\u{2014}',
    ];
    let mut rest = text;
    loop {
        rest = rest.trim_start_matches(BOUNDARY_TRIM);
        rest = rest.trim_end_matches(BOUNDARY_TRIM);
        // Round 24 (R24-4): a standalone boundary
        // token at either end of the remainder ("*",
        // "•", "|", "~", "_", an emoji — the token
        // rule, not the character table) is separator
        // punctuation standing at the shape's
        // boundary, exactly as the tabled characters
        // are: "next Sunday * (November 8)" and
        // "(November 8) * next Sunday" remember
        // their Sundays as their comma twins do.
        rest = strip_leading_boundary_tokens(rest);
        rest = strip_trailing_boundary_tokens(rest);
        rest = rest.trim_start_matches(BOUNDARY_TRIM);
        rest = rest.trim_end_matches(BOUNDARY_TRIM);
        if shape(rest) {
            return true;
        }
        // Round 21 (R21-1): a LEADING group is garnish
        // to the shape exactly as a trailing one is —
        // "(November 8) next Sunday" names its Sunday
        // in words the way "next Sunday (November 8)"
        // does, the group's date remembered separately.
        match strip_trailing_parenthesized(rest).or_else(|| strip_leading_parenthesized(rest)) {
            Some(stripped) => rest = stripped,
            None => return false,
        }
    }
}

/// The date a trailing parenthesized group names, if
/// one does (round 8, carried shape): groups are
/// taken from the end, and the first that reads as a
/// date is the remembered one. A group that does not
/// read as a date at all — a weekday garnish like
/// "(Monday)" — is garnish, not memory.
/// The signals standing inside a parenthesized
/// group's words, every occurrence in written order,
/// with where each one stands: `dates_inside_group_signals`
/// below is that scan (round 11, R11-4/R11-8: the
/// words standing before a date inside its group
/// decide whether that date may contribute at all).
/// One signal standing inside a parenthesized
/// group (round 18, R18-6): a framed date as written,
/// or a relative phrase carried by its resolution —
/// the same signal set a plain line carries, in
/// written order, each with the token span it stands
/// in. `anchored` is a pure relative phrase's
/// resolution against the parse anchor, when this
/// signal is one; the group analysis applies it to
/// every relative signal (round 19, R19-3).
struct GroupSignal {
    date: SimpleDate,
    start: usize,
    used: usize,
    anchored: Option<SimpleDate>,
}

/// The signals standing inside a parenthesized
/// group's words, every occurrence in written order
/// (round 12, R12-2's occurrence rule, extended
/// round 18, R18-6): the scan is the plain line's
/// signal scan — a Sunday-after phrase consumes its
/// run first, then "next Sunday" / "this Sunday",
/// then a framed date, then a bare "Sunday" the
/// framed reader did not take. Every occurrence
/// stands on its own: occurrences are never deduped
/// away by value here; the group analysis qualifies
/// each one and the outcome dedupes by date. One
/// guard is the group's own: a bare "Sunday"
/// standing directly in front of a framed date is
/// that date's framing weekday, not a relative
/// signal — inside a group there is no earlier
/// framed reader to have claimed it, the way a
/// plain line's readers claim it before the plain
/// scan runs.
fn dates_inside_group_signals(
    raw_tokens: &[&str],
    tokens: &[String],
    today: SimpleDate,
    anchor: SimpleDate,
) -> Vec<GroupSignal> {
    let mut out: Vec<GroupSignal> = Vec::new();
    let mut last_end = 0usize;
    let mut last_framed_sunday = false;
    let mut i = 0;
    while i < tokens.len() {
        if tokens[i] == "sunday" && tokens.get(i + 1).map(String::as_str) == Some("after") {
            let (which_at, used) = if tokens.get(i + 2).map(String::as_str) == Some("the") {
                (i + 3, 5)
            } else {
                (i + 2, 4)
            };
            let which = tokens.get(which_at).map(String::as_str);
            let base_day = tokens.get(which_at + 1).map(String::as_str);
            if (which == Some("next") || which == Some("this")) && base_day == Some("sunday") {
                let base = relative_sunday_date(which == Some("next"), today);
                let date = first_sunday_after(base);
                let anchored_base = relative_sunday_date(which == Some("next"), anchor);
                let anchored = first_sunday_after(anchored_base);
                out.push(GroupSignal {
                    date,
                    start: i,
                    used,
                    anchored: Some(anchored),
                });
                last_end = i + used;
                last_framed_sunday = false;
                i += used;
                continue;
            }
            if let Some((base, end)) = sunday_after_phrase_run(raw_tokens, tokens, i, today) {
                let date = first_sunday_after(base);
                let used = end.max(i + 1) - i;
                out.push(GroupSignal {
                    date,
                    start: i,
                    used,
                    anchored: None,
                });
                last_end = i + used;
                last_framed_sunday = false;
                i += used;
                continue;
            }
        }
        if (tokens[i] == "next" || tokens[i] == "this")
            && tokens.get(i + 1).map(String::as_str) == Some("sunday")
        {
            let date = relative_sunday_date(tokens[i] == "next", today);
            let anchored = relative_sunday_date(tokens[i] == "next", anchor);
            out.push(GroupSignal {
                date,
                start: i,
                used: 2,
                anchored: Some(anchored),
            });
            last_end = i + 2;
            last_framed_sunday = false;
            i += 2;
            continue;
        }
        match parse_date_prefix(&raw_tokens[i..], today) {
            Some((d, used)) => {
                out.push(GroupSignal {
                    date: d,
                    start: i,
                    used: used.max(1),
                    anchored: None,
                });
                last_end = i + used.max(1);
                last_framed_sunday = d.weekday() == 0;
                i += used.max(1);
            }
            None => {
                // A group that is a bare weekday word
                // alone — "(Sunday)" — is a weekday
                // garnish group, the shape the r9c
                // stacked-parentheses pin carries
                // ("next Sunday (October 25) (Sunday)"):
                // it contributes nothing, as "(Monday)"
                // never has. The explicit relative
                // phrases "(next Sunday)" / "(this
                // Sunday)" are signals (above), not
                // garnish.
                if tokens[i] == "sunday"
                    && tokens.len() > 1
                    && !(i == last_end && last_framed_sunday)
                    && parse_date_prefix(&raw_tokens[i + 1..], today).is_none()
                    // Round 20 (R20-6): a weekday word
                    // followed by a noun-like word is
                    // that noun phrase's modifier, not
                    // a relative signal — "printed
                    // Sunday bulletins" names no
                    // Sunday at all, and the scan must
                    // not invent one (October 18) for
                    // the note. Round 23 (R23-6): the
                    // verb-form adjective shape joins
                    // it ("Sunday taken seats") — the
                    // one reading in `weekday_modifier_at`.
                    && !weekday_modifier_at(tokens, i)
                    // Round 22 (R22-5): a weekday
                    // consumed wholly as a subject
                    // PP's object ("the choir on
                    // Sunday was singing") spawns no
                    // signal either — the auxiliary
                    // or verb after it is not
                    // noun-like, so the modifier rule
                    // above never reached this shape.
                    && !weekday_in_subject_pp(tokens, i)
                    // Round 24 (R24-5): a weekday
                    // standing as a PP object inside
                    // a target claim's object
                    // continuation ("baked beans
                    // from Kent on Sunday to
                    // November 8") is consumed wholly
                    // the same way — it is a modifier
                    // of the object, and no phantom
                    // October 18 spawns from it.
                    && !weekday_in_object_continuation_pp(raw_tokens, tokens, i, today)
                {
                    let date = relative_sunday_date(false, today);
                    let anchored = relative_sunday_date(false, anchor);
                    out.push(GroupSignal {
                        date,
                        start: i,
                        used: 1,
                        anchored: Some(anchored),
                    });
                    last_end = i + 1;
                    last_framed_sunday = false;
                }
                i += 1;
            }
        }
    }
    out
}

/// Whether a token contains no letters and no
/// digits at all (round 24, R24-4): the boundary
/// classification is a TOKEN rule now, not a
/// character table — a standalone token of this
/// shape ("*", "•", "|", "~", "_", an emoji, and
/// every tabled separator of the earlier rounds
/// alike) is a boundary token at every site that
/// uses the boundary classification.
fn token_has_no_letters_or_digits(raw: &str) -> bool {
    !raw.chars().any(|c| c.is_alphanumeric())
}

/// Whether a token is one bracketed unit — "[x]",
/// "{x}" (round 24, R24-4): such a token counts as
/// a boundary/garnish token in group cleaning and
/// in date spans. Brackets are NOT group
/// delimiters: this says nothing about the text a
/// bracket pair spans, only about a single token
/// the pair wraps whole.
fn is_bracketed_token(raw: &str) -> bool {
    let t = raw.trim_end_matches([',', '.', ':', ';']);
    (t.starts_with('[') && t.ends_with(']')) || (t.starts_with('{') && t.ends_with('}'))
}

/// `text` with a standalone boundary token at its
/// head taken off, repeatedly (round 24, R24-4):
/// text-level sites (relative-shape memory, the
/// composition segment weekday check) strip such a
/// token standing where punctuation was taken off
/// exactly as they strip the tabled characters.
fn strip_leading_boundary_tokens(text: &str) -> &str {
    let mut rest = text.trim_start();
    loop {
        let Some(first) = rest.split_whitespace().next() else {
            return rest;
        };
        if rest.len() <= first.len() || !token_has_no_letters_or_digits(first) {
            return rest;
        }
        rest = rest[first.len()..].trim_start();
    }
}

/// `text` with a standalone boundary token at its
/// tail taken off, repeatedly (round 24, R24-4):
/// the tail twin of `strip_leading_boundary_tokens`.
fn strip_trailing_boundary_tokens(text: &str) -> &str {
    let mut rest = text.trim_end();
    loop {
        let Some(last) = rest.split_whitespace().next_back() else {
            return rest;
        };
        if rest.len() <= last.len() || !token_has_no_letters_or_digits(last) {
            return rest;
        }
        rest = rest[..rest.len() - last.len()].trim_end();
    }
}

/// One group token with its wrapping punctuation off,
/// lowercased: the shape the governor and weekday
/// checks read.
fn group_token(raw: &str) -> String {
    // Round 23 (R23-3): ONE boundary classification —
    // colon, semicolon, comma, dash (incl. en/em),
    // slash, and period are clause/group boundaries,
    // and this shared cleaning strips them all from a
    // token's edges. A slash INSIDE a token is not an
    // edge: a numeric date ("11/1") stays token-internal
    // and reads whole.
    // Round 24 (R24-4): the classification is a token
    // rule — a standalone token containing no letters
    // and no digits cleans to nothing, whatever
    // characters it is built from ("*", "•", "|",
    // "~", "_", an emoji token), and a wholly
    // bracketed token ("[x]", "{x}") cleans to
    // nothing too: it is boundary/garnish here, not
    // a content word standing beside a date.
    if is_bracketed_token(raw) || token_has_no_letters_or_digits(raw) {
        return String::new();
    }
    raw.trim_matches([
        '(', ')', '[', ']', '{', '}', ',', '.', ':', ';', '-', '\u{2013}', '\u{2014}', '/',
    ])
    .to_ascii_lowercase()
}

/// Whether a raw group token ends a clause: a token
/// wearing a trailing comma or semicolon closes the
/// words it stands in, so a governor on the far side
/// of it does not reach across it. A dash closes a
/// clause the same way (round 13, R13-2): a
/// standalone "-" / "--" token, or a token wearing a
/// trailing en/em dash or hyphen.
fn clause_break_after(raw: &str) -> bool {
    // Round 23 (R23-3): the ONE boundary classification
    // — a token wearing a trailing colon, semicolon,
    // comma, dash (incl. en/em), slash, or period closes
    // its clause exactly as the comma has since round 12,
    // and a standalone boundary token ("/", ")", a dash)
    // is a boundary standing alone: a closing paren
    // between a governor and its date is a boundary of
    // the same kind. A slash inside a token ("11/1") is
    // token-internal and never reaches this test as an
    // ending.
    // Round 24 (R24-4): a standalone token containing
    // no letters and no digits is a boundary token
    // standing alone, whatever characters build it
    // ("*", "•", "|", "~", "_", an emoji), and a
    // token closing a bracketed span (ending "]" or
    // "}") carries that span's boundary at its tail,
    // as the comma twin of "[November 1, cancelled]
    // [November 8]" requires.
    let raw_trimmed = raw.trim();
    if !raw_trimmed.is_empty() && token_has_no_letters_or_digits(raw_trimmed) {
        return true;
    }
    if raw_trimmed.ends_with(']') || raw_trimmed.ends_with('}') {
        return true;
    }
    if raw.trim() == ")" || raw.trim() == "/" {
        return true;
    }
    let trimmed = raw.trim_end_matches([')', ']', ' ']);
    trimmed.ends_with(',')
        || trimmed.ends_with(';')
        || trimmed.ends_with(':')
        || trimmed.ends_with('.')
        || trimmed.ends_with('/')
        || trimmed.ends_with('\u{2013}')
        || trimmed.ends_with('\u{2014}')
        || trimmed == "-"
        || trimmed == "--"
        || (trimmed.len() > 1 && trimmed.ends_with('-'))
}

/// Whether a clause carries no content of its
/// own (round 14, RT14-4; round 15, RT15-5): every
/// word is date-referring filler — "the", "a",
/// "an", "date", "it", "note", "notes", and now
/// "of" — or the clause is a genitive fragment of
/// the date phrase itself: a clause headed by
/// "date of" ("the date of the service") names
/// the date by naming the occasion the date
/// belongs to; it asserts no event of its own, so
/// it is filler for the reach-back like any pure
/// filler clause. A clause of nothing but break
/// punctuation (a standalone dash) is likewise
/// empty. Any other word is content and stops
/// the reach-back.
fn is_filler_clause(clause: &[String]) -> bool {
    if clause.iter().all(|w| w.is_empty()) {
        return true;
    }
    if clause.iter().all(|w| {
        matches!(
            w.as_str(),
            "the" | "a" | "an" | "date" | "it" | "note" | "notes" | "of"
        )
    }) {
        return true;
    }
    let body: &[String] = match clause.first().map(String::as_str) {
        Some("the" | "a" | "an") => &clause[1..],
        _ => clause,
    };
    body.len() >= 3
        && body[0] == "date"
        && body[1] == "of"
        && body[2..]
            .iter()
            .all(|w| !direct_single_governor(w) && weekday_word(w).is_none())
}

/// Whether a clause is a cancellation and
/// nothing else (round 15, R15-3): it holds a
/// cancellation phrase, and every word outside
/// that phrase is date-referring filler ("the",
/// "a", "an", "date", "it", "that", "note",
/// "notes", "of") — "postponed", "drop it",
/// "scratch that", "cancelled, the date" read
/// as clauses. Such a clause standing after a
/// date is spent on that date, and a run of
/// them is spent together.
fn is_cancellation_only_clause(clause: &[String]) -> bool {
    for i in 0..clause.len() {
        for len in 1..=3usize {
            let Some(phrase) = clause.get(i..i + len) else {
                break;
            };
            if !is_cancellation_phrase(phrase) {
                continue;
            }
            // Round 16 (RT16-6): a determiner-headed
            // -s word is a noun, not a cancellation —
            // the clause is not cancellation-only.
            if len == 1 && determiner_headed_s_form(clause, i, &phrase[0]) {
                continue;
            }
            let rest_filler = clause
                .iter()
                .enumerate()
                .filter(|(j, _)| *j < i || *j >= i + len)
                .all(|(_, w)| {
                    matches!(
                        w.as_str(),
                        "the" | "a" | "an" | "date" | "it" | "that" | "note" | "notes" | "of"
                    )
                });
            if rest_filler {
                return true;
            }
        }
    }
    false
}

/// Whether a clause is a negation / prospective
/// phrase and nothing else (round 16, R16-2): it
/// holds a governor phrase that is not a
/// cancellation phrase — "not", "no longer",
/// "ignore", "never", a contraction — and every
/// word outside that phrase is date-referring
/// filler, the same set `is_cancellation_only_clause`
/// allows. Such a clause carries no date of its
/// own, so standing between two dates it belongs to
/// the run that spends on the first date, exactly
/// as a cancellation clause does. (A prospective
/// clause carrying its own date — "use November 8"
/// — is the second date's own clause: the date
/// stands inside it, so it never reaches this test
/// as a between-dates clause.)
fn is_prospective_only_clause(clause: &[String]) -> bool {
    for i in 0..clause.len() {
        for len in 1..=3usize {
            let Some(phrase) = clause.get(i..i + len) else {
                break;
            };
            if !is_governor_phrase(phrase) || is_cancellation_phrase(phrase) {
                continue;
            }
            let rest_filler = clause
                .iter()
                .enumerate()
                .filter(|(j, _)| *j < i || *j >= i + len)
                .all(|(_, w)| {
                    matches!(
                        w.as_str(),
                        "the" | "a" | "an" | "date" | "it" | "that" | "note" | "notes" | "of"
                    )
                });
            if rest_filler {
                return true;
            }
        }
    }
    false
}

/// Whether a clause belongs to one of the governor
/// families or is filler (round 16, R16-2): the
/// clause scope a run between dates is judged by.
fn is_governor_family_clause(clause: &[String]) -> bool {
    is_cancellation_only_clause(clause)
        || is_prospective_only_clause(clause)
        || is_filler_clause(clause)
}

/// The governor window standing before one date
/// occurrence inside its group (round 12, R12-1 /
/// R12-2): the tokens after the previous date
/// occurrence's end and after the last clause break
/// before this date, up to the date itself. A
/// governor reaches only the one date it governs —
/// the first date standing in its own clause after
/// it. Words in an earlier clause, or spent on an
/// earlier date, govern nothing here: "(notes from
/// the pastor, November 1)" and "(choir was singing,
/// November 1)" name November 1 in a fresh clause,
/// and "(not November 1, November 8)" asserts
/// November 8 on its own terms. Round 13 (R13-2):
/// the break erases only a spent governor — an
/// unspent governor standing as its clause's
/// trailing element reaches the first date of the
/// next clause (see the body).
fn governor_window_start(
    raw_tokens: &[&str],
    tokens: &[String],
    prev_end: usize,
    date_start: usize,
    has_prev_date: bool,
) -> usize {
    let lo = prev_end.min(date_start);
    let mut start = lo;
    let mut last_break: Option<usize> = None;
    for j in lo..date_start {
        if raw_tokens.get(j).map(|t| clause_break_after(t)) == Some(true) {
            start = j + 1;
            last_break = Some(j);
        }
    }
    // Round 13 (R13-2): a clause break erases only a
    // SPENT governor. When the clause standing
    // before the last break ends in an unspent
    // governor — nothing between it and the break
    // but date-referring filler — that governor
    // still governs the first date of the next
    // clause, so the window reaches back to that
    // clause's start. A governor followed by other
    // words in its own clause ("notes from the
    // pastor, …") reaches nothing across the break:
    // the garnish pin stands. Semicolon and dash
    // breaks behave like comma breaks here, as
    // `clause_break_after` already classes them.
    if last_break.is_some() {
        // The clause breaks in the window, in order;
        // the clauses they close are examined from
        // the nearest back (round 14, RT14-4 below).
        let breaks: Vec<usize> = (lo..date_start)
            .filter(|&j| raw_tokens.get(j).map(|t| clause_break_after(t)) == Some(true))
            .collect();
        let mut idx = breaks.len();
        while idx > 0 {
            idx -= 1;
            let b = breaks[idx];
            let clause_start = if idx == 0 { lo } else { breaks[idx - 1] + 1 };
            // A standalone dash break token carries
            // no word of the clause it closes.
            let mut clause_end = b + 1;
            if matches!(tokens.get(b).map(String::as_str), Some("" | "-" | "--")) {
                clause_end = b;
            }
            if clause_start >= clause_end {
                break;
            }
            let clause = &tokens[clause_start..clause_end];
            let clause_follows_a_date = has_prev_date && clause_start == lo;
            // Round 15 (R15-3): a clause that is a
            // cancellation and nothing else, standing
            // in a window that follows a date, is
            // spent on that date — and so is a whole
            // RUN of them: "(November 1, postponed,
            // drop it, November 8)" spends both
            // phrases on November 1, and November 8
            // asserts. The walk simply never extends
            // the window into such a clause; filler
            // clauses between the run's phrases are
            // crossed by the filler rule below, so a
            // run wearing one ("cancelled, the date,
            // drop it") is spent together too.
            if has_prev_date && is_cancellation_only_clause(clause) {
                continue;
            }
            // Round 16 (R16-2): a negation / prospective
            // clause with no date of its own, standing
            // between dates, spends on the first date
            // together with the cancellation clauses —
            // "(November 1, not, drop it, November 8)"
            // spends "not" and "drop it" on November 1
            // and asserts November 8. Round 15's walk
            // spent only cancellation clauses, so the
            // prospective word read as an unspent
            // governor of the second date and the
            // replacement died unnamed.
            if has_prev_date && is_prospective_only_clause(clause) {
                continue;
            }
            // Round 20 (R20-5): a fronted adverb
            // stack standing in its own clause
            // directly before this one is judged by
            // the same list and the same two-word
            // cap as every other position: an
            // unlisted member, or a stack that with
            // this clause's own crossed adverbs
            // exceeds the cap, leaves this clause's
            // predicates unspent.
            let fronted_invalid = if idx >= 1 {
                let (p_start, p_end) = clause_span(&breaks, lo, idx - 1, tokens);
                match adverb_stack_clause(&tokens[p_start..p_end]) {
                    Some((count, all_listed)) => {
                        let in_clause = clause.iter().filter(|w| is_tolerated_adverb(w)).count();
                        !all_listed || count + in_clause > 2
                    }
                    None => false,
                }
            } else {
                false
            };
            if unspent_trailing_governor_fronted(clause, clause_follows_a_date, fronted_invalid) {
                start = clause_start;
                break;
            }
            // Round 14 (RT14-4): an intervening clause
            // of nothing but date-referring filler
            // ("the", "a", "an", "date", "it", "note",
            // "notes") is transparent to the
            // reach-back — "(ignore, the date,
            // November 1)" and "(ignore, notes,
            // November 1)" disqualify like the held
            // "(ignore the date, November 1)". (Such a
            // clause holds no date token: its words
            // are the filler set itself.) A clause
            // with real content stops the reach-back,
            // exactly as before. Round 15 (RT15-5):
            // "of" joins the filler set, and a
            // genitive fragment of the date phrase
            // ("the date of the service") counts as
            // filler too — see `is_filler_clause`.
            if is_filler_clause(clause) {
                continue;
            }
            // Round 16 (R16-7): a filler clause ending
            // in "of" leaves a genitive open, and a
            // following determiner-headed noun clause
            // completes it — "(ignore, the date of,
            // the service, November 1)" disqualifies
            // like the unsplit shape. Cross the
            // completing clause and judge the "of"
            // clause itself on the next pass.
            if idx >= 1 {
                let (p_start, p_end) = clause_span(&breaks, lo, idx - 1, tokens);
                if p_start < p_end && genitive_completion(clause, &tokens[p_start..p_end]) {
                    continue;
                }
            }
            break;
        }
    }
    start
}

/// The (start, end) token span of the clause closed
/// by break `k` in a governor window, in the walk's
/// own terms (round 16, R16-7): a standalone dash
/// break token carries no word of its clause.
fn clause_span(breaks: &[usize], lo: usize, k: usize, tokens: &[String]) -> (usize, usize) {
    let b = breaks[k];
    let start = if k == 0 { lo } else { breaks[k - 1] + 1 };
    let mut end = b + 1;
    if matches!(tokens.get(b).map(String::as_str), Some("" | "-" | "--")) {
        end = b;
    }
    (start, end)
}

/// Whether `clause` completes a genitive the clause
/// before it left open (round 16, R16-7): the prior
/// clause is filler ending in "of" ("the date of"),
/// and this clause is a determiner-headed noun
/// fragment — a determiner plus nouns only, no date
/// token, no verb-form, no governor — that completes
/// it ("the service"). The reach-back crosses such a
/// clause as it crosses the filler itself. A clause
/// with real content (a verb, a date, or more than
/// the fragment) still stops the reach-back.
fn genitive_completion(clause: &[String], prev: &[String]) -> bool {
    if clause.len() < 2 || !matches!(clause[0].as_str(), "the" | "a" | "an") {
        return false;
    }
    // Round 17 (R17-6): the "no verb-form" test is
    // the determiner-aware form test, the same one
    // the rest of the batch's determiner rule uses —
    // an -s stem word headed by this clause's own
    // determiner is a noun phrase's head ("the
    // drops", "the shifts"), not a verb-form, so it
    // does not stop the crossing. A word that is a
    // verb-form without that heading (a bare -ed
    // form, "the moved service") still stops it.
    let nouns_only = clause[1..].iter().enumerate().all(|(off, w)| {
        let headed_noun = determiner_headed_s_form(clause, off + 1, w);
        (!is_cancellation_verb(w) || headed_noun)
            && (!direct_single_governor(w) || headed_noun)
            && weekday_word(w).is_none()
    });
    if !nouns_only {
        return false;
    }
    prev.last().map(String::as_str) == Some("of") && is_filler_clause(prev)
}

/// Whether the words in a date occurrence's governor
/// window disqualify that date (round 11, R11-4;
/// families completed round 12, R12-1): a negation
/// or cancellation ("not", "ignore", "never",
/// "skip", "except", "rather than", "instead of",
/// "cancelled/canceled", bare "no", "isn't" and its
/// family, "never mind", "no longer",
/// "don't/didn't use") or an origin/past marker
/// ("from", "originally", "previously", "formerly",
/// "used to be", "was", "old date") standing in the
/// window means the group does not assert that
/// date — it denies it, cancels it, or names it as
/// the superseded one. Round 13 (R13-1) completes
/// the classes structurally: any "n't" contraction
/// (plus wont / cannot) negates, the cancellation
/// verbs cancel / scratch / drop / postpone /
/// remove and the pair "hold off" govern, and the
/// label "wrong:" (the token "wrong", its colon
/// stripped by `group_token`) disowns its date.
/// "Was" and "no" are common
/// words: they govern only here, in the governing
/// clause the window defines, never anywhere in
/// prose at large. "Moved to" and "correction:"
/// carry no marker and keep contributing.
/// The start of a passive "for" construction
/// standing at the end of a governor window, if one
/// does (round 17, R17-4): the window ends in "for",
/// the word before it is a participle — a governor
/// participle in any form, or any -ed participle
/// ("scheduled", the canonical occasion verb) — and
/// scanning back from the participle crosses at
/// least one auxiliary, up to two tolerated
/// adverbs allowed (round 18, R18-10), exactly as
/// the spent-predicate scan crosses them. In "(it was moved for November 1)"
/// the predicate is spent on its subject and "for"
/// names the date as the occasion, the way the
/// comma twin "(it was moved, November 1)" always
/// has. The bare same-clause parity stands apart:
/// with no "for" there is no construction here, and
/// "(it was moved November 1)" governs as pinned.
/// Common finite verb stems beyond the governor
/// families (round 18, R18-8): the verbs an office
/// clause is built from — "choir sang", "the ushers
/// walked" — whose forms mark a new finite clause.
/// The governor families (cancel / move / …) are
/// tested by their own stem rules wherever this
/// test is used; this list covers the rest of the
/// ordinary clause verbs, regular and irregular
/// alike. It is a closed list, like the adverb
/// list: a verb outside it is not seen as one.
const FINITE_VERB_STEMS: [&str; 50] = [
    "sing", "walk", "talk", "speak", "write", "read", "play", "work", "stand", "sit", "run",
    "meet", "hold", "take", "give", "make", "say", "tell", "come", "leave", "keep", "begin",
    "break", "choose", "fall", "feel", "find", "grow", "hear", "know", "lose", "pay", "see",
    "send", "spend", "teach", "think", "wear", "win", "build", "catch", "draw", "fight", "lead",
    "mean", "ring",
    // Round 19 (R19-5): the go family and the
    // rehearses / gathers / plans stems — "go"
    // stood in neither list ("went" alone was in
    // the irregulars), and "rehearse" / "gather" /
    // "plan" were absent entirely, so the reach
    // crossed those second clauses and named the
    // from-date.
    "go", "plan", "rehearse", "gather",
];

/// Irregular finite forms the stem rules cannot
/// build (round 18, R18-8): "sang" is a form of
/// "sing" the way "forgot" is a form of "forget",
/// and it is the witness word of the second-clause
/// rule — "(moved November 1, choir sang to
/// November 8)".
const IRREGULAR_FINITE_FORMS: [&str; 59] = [
    // Round 19 (R19-5): "gone" joins "went" — the
    // go family's participle, which the stem rules
    // cannot build from "go".
    // Round 22 (R22-4): "taken" joins on the same
    // ground — the take family's participle, which
    // the suffix rules cannot build from "take";
    // without it "then was taken" had no complement
    // the auxiliary scan could see.
    "sang", "sung", "went", "gone", "came", "took", "taken", "gave", "made", "said", "told", "met",
    "held", "ran", "sat", "stood", "spoke", "wrote", "ate", "drank", "brought", "bought", "sold",
    "left", "kept", "began", "broke", "chose", "fell", "felt", "found", "got", "grew", "heard",
    "knew", "lost", "paid", "saw", "sent", "spent", "taught", "thought", "wore", "won", "built",
    "caught", "drew", "fought", "led", "meant", "rang", "rose", "shook", "slept", "struck", "swam",
    "tore", "threw", "woke",
];

/// The past-tense-only irregular forms (round 22,
/// R22-6): the members of `IRREGULAR_FINITE_FORMS`
/// whose participle is a different word (sing /
/// sang / sung, ring / rang / rung, take / took /
/// taken, begin / began / begun, and their kin —
/// "went" and "came" included, their participles
/// being "gone" and "come"). A past-only form has
/// no adjective use in any position, so the
/// determiner shield that spans a noun phrase for
/// a participle form (see `verb_form_is_finite`)
/// never reaches it: "sang hymns" is a clause
/// standing alone and a clause behind a subject
/// ("the choir sang hymns"), never a noun phrase.
/// The remaining irregulars are participles or
/// past/participle-shared forms (sung, gone, left,
/// held, made, met, …), which do carry the
/// adjective use "the big sung hymns" wears.
const IRREGULAR_PAST_ONLY_FORMS: [&str; 25] = [
    "sang", "went", "came", "took", "gave", "spoke", "wrote", "ate", "drank", "ran", "began",
    "broke", "chose", "fell", "grew", "knew", "saw", "drew", "rang", "rose", "shook", "swam",
    "tore", "threw", "woke",
];

/// Whether `word` is a finite verb form beyond the
/// governor families (round 18, R18-8): a form of
/// one of the common clause-verb stems, an irregular
/// finite form, or a regular -ed form. The -ed test
/// is the blunt edge: a word the office uses as a
/// noun or adjective can end in "ed" ("indeed" is
/// the standing exception), and callers that scan a
/// noun phrase temper it with the determiner rule
/// in `segment_has_finite_verb`.
fn is_finite_verb_form(word: &str) -> bool {
    if word == "indeed" {
        return false;
    }
    if IRREGULAR_FINITE_FORMS.contains(&word) {
        return true;
    }
    if FINITE_VERB_STEMS
        .iter()
        .any(|stem| is_verb_form(word, stem))
    {
        return true;
    }
    word.len() > 3 && word.ends_with("ed")
}

/// The generic -ed ambiguity (round 19, R19-4):
/// a word whose only claim to being a finite
/// verb is the generic -ed suffix — not an
/// irregular form and not a form of any listed
/// stem — is exactly the ambiguous case: the
/// same spelling
/// is an adjective in "baked beans" and a noun in
/// "the printed bulletins" office prose. Round 20
/// (R20-4): the tempering that distinction fed —
/// generic -ed forms only — is replaced by the
/// one verb test for every verb form (see
/// `verb_form_is_finite` below).
/// Whether a noun-like subject stands before the
/// verb-form word at `off` in its segment (round
/// 20, R20-4): the subject is a noun-like word
/// that is not itself a verb form, and nothing
/// but tolerated adverbs and auxiliaries stand
/// between it and the verb form — "choir folded",
/// "team played", "their team played", "the choir
/// sang" all qualify. A segment opening with a
/// conjunction continues the target phrase's
/// object list (the R18-4 reading: "…, and the
/// dinner to …"), so its words are that list's
/// own material and no subject stands in it: "and
/// planned dinner", "and fresh baked beans" name
/// things, they do not do anything.
///
/// Round 21 (R21-6): the subject is a RUN of
/// determiner / modifier / noun-like words ending
/// at the verb form, not exactly one noun-like
/// word — the one-word test crossed every real
/// clause whose subject wore a modifier or a
/// title ("the big choir sang", "choir members
/// sang", "the youth choir rang", "Pastor Lee
/// sang") and named a false from-date. The walk
/// back from the verb form crosses the words that
/// can stand in a subject phrase or between it
/// and its verb — determiners and possessives,
/// noun-like words (the head and its modifiers,
/// verb-form modifiers among them: "visiting
/// choir", "planning committee"), tolerated
/// adverbs, and auxiliaries — and the subject
/// stands when the walk crosses the phrase's
/// head: a noun-like word that is not itself a
/// verb form. Two noun-like words before the verb
/// form are no longer by themselves a noun phrase
/// the verb form postposes; the postposed reading
/// is judged in `verb_form_is_finite` itself.
/// The prepositions that can head a phrase inside
/// a subject run (round 22, R22-5): the locative /
/// origin / temporal family a subject wears — "the
/// team from Kent", "the choir in the hall", "the
/// choir on Sunday". "to" and "for" are not here:
/// they head the target and occasion constructions,
/// never a subject's own phrase.
fn is_subject_preposition(word: &str) -> bool {
    matches!(
        word,
        "from"
            | "of"
            | "on"
            | "in"
            | "at"
            | "by"
            | "with"
            | "before"
            | "after"
            | "since"
            | "during"
            | "between"
            | "among"
            | "within"
            | "upon"
            | "into"
            | "over"
            | "under"
            | "against"
            | "across"
            | "behind"
            | "beyond"
            | "beside"
            | "near"
            | "through"
            | "without"
            | "toward"
            | "towards"
            | "onto"
            | "per"
            | "via"
            | "until"
            | "around"
            | "about"
    )
}

fn segment_subject_before(segment: &[String], off: usize) -> bool {
    if matches!(
        segment.first().map(String::as_str),
        Some("and" | "or" | "but")
    ) {
        return false;
    }
    let mut head = false;
    let mut i = off;
    // Round 22 (R22-5): the subject run crosses a
    // prepositional phrase standing inside it —
    // determiner / noun head + preposition + its
    // own noun-like run counts as the subject, and
    // the phrase's words are consumed wholly by
    // it: "the team from Kent sang", "the choir on
    // Sunday played". The walk crosses a
    // preposition only after crossing the noun-like
    // run that is its object (a weekday object
    // included — "on Sunday" is the phrase's own
    // material, spawning nothing of its own), and
    // a preposition with no object run before it in
    // the walk still ends the run, as every
    // preposition did before this round. Subject
    // runs without PPs behave exactly as R21-B
    // pinned.
    let mut pp_object_crossed = false;
    while i > 0 {
        let w = segment[i - 1].as_str();
        if matches!(w, "the" | "a" | "an") || POSSESSIVE_DETERMINERS.contains(&w) {
            i -= 1;
        } else if noun_like_word(w) {
            if !is_finite_verb_form(w) {
                head = true;
            }
            pp_object_crossed = true;
            i -= 1;
        } else if weekday_word(w).is_some()
            && ((i >= 2 && is_subject_preposition(segment[i - 2].as_str()))
                // Round 23 (R23-5): the PP's weekday
                // object may wear its own determiner
                // — "the choir on the Sunday sang":
                // the determiner stands between the
                // preposition and the weekday, and
                // the run crosses it the same way.
                || (i >= 3
                    && matches!(segment[i - 2].as_str(), "the" | "a" | "an")
                    && is_subject_preposition(segment[i - 3].as_str()))
                // Round 24 (R24-7): the PP's object
                // may be a relative phrase — "the
                // choir from next Sunday sang": the
                // relative word stands between the
                // preposition (or its determiner) and
                // the weekday, and the run crosses
                // the phrase the same way.
                || (i >= 3
                    && matches!(segment[i - 2].as_str(), "next" | "this")
                    && is_subject_preposition(segment[i - 3].as_str()))
                || (i >= 4
                    && matches!(segment[i - 2].as_str(), "next" | "this")
                    && matches!(segment[i - 3].as_str(), "the" | "a" | "an")
                    && is_subject_preposition(segment[i - 4].as_str())))
        {
            pp_object_crossed = true;
            i -= 1;
        } else if is_subject_preposition(w) && pp_object_crossed {
            pp_object_crossed = false;
            i -= 1;
        } else if is_tolerated_adverb(w) || AUXILIARY_WORDS.contains(&w) {
            i -= 1;
        } else {
            break;
        }
    }
    head
}

/// Whether the weekday word at `i` in `tokens`
/// heads a weekday-modifier phrase (round 20,
/// R20-6; round 21, R21-7; round 23, R23-6): a
/// weekday word followed by a noun-like word is
/// that noun phrase's modifier ("Sunday
/// bulletins", "Sunday school"), and — R23-6 — a
/// weekday word followed by a VERB-FORM word and
/// then a noun-like word is the same phrase with
/// the verb-form word read as its adjective
/// ("Sunday left luggage", "Sunday sung hymns",
/// "Sunday taken seats"). Before this round the
/// second shape spawned the phantom October 18:
/// `noun_like_word` excludes the irregular verb
/// forms, so "left" / "sung" / "taken" could not
/// be the adjective the scans needed, and the
/// weekday fell through to the bare-Sunday
/// relative signal. One reading on every surface
/// that judges a weekday phrase: the group and
/// plain signal scans, the claim reach's
/// between-words test, and the lone-framed fence.
fn weekday_modifier_at(tokens: &[String], i: usize) -> bool {
    weekday_word(&tokens[i]).is_some()
        && (tokens.get(i + 1).is_some_and(|next| noun_like_word(next))
            || (tokens
                .get(i + 1)
                .is_some_and(|next| is_finite_verb_form(next))
                && tokens.get(i + 2).is_some_and(|next| noun_like_word(next))))
}

/// Whether the weekday word at `i` in `tokens` is
/// consumed wholly as the object of a prepositional
/// phrase inside a subject run (round 22, R22-5):
/// a subject head stands before the preposition
/// ("the choir on Sunday …") and the clause's own
/// verb stands after the weekday. Such a weekday
/// spawns no relative signal, asserts no phantom
/// date, and is named nowhere — before this round
/// "the choir on Sunday was singing" minted an
/// October 18 signal inside the subject phrase,
/// because the modifier rule reaches only a
/// following noun-like word and an auxiliary or an
/// irregular verb is not one. A weekday phrase
/// with no subject head before it, or no clause
/// verb after it, keeps its standing readings.
fn weekday_in_subject_pp(tokens: &[String], i: usize) -> bool {
    // Round 23 (R23-5): the weekday object may
    // wear its own determiner — "the choir on the
    // Sunday sang" — so the preposition may stand
    // one word further back, behind that
    // determiner. A determiner inside the PP does
    // not block consumption.
    let prep = if i >= 1 && is_subject_preposition(tokens[i - 1].as_str()) {
        i - 1
    } else if i >= 2
        && matches!(tokens[i - 1].as_str(), "the" | "a" | "an")
        && is_subject_preposition(tokens[i - 2].as_str())
    {
        i - 2
    } else {
        return false;
    };
    if !subject_head_before(tokens, prep) {
        return false;
    }
    // The phrase's object may wear its own nouns
    // after the weekday ("on Sunday morning sang"),
    // and a subject may wear a STACK of PPs, so the
    // clause's verb may stand past a further PP
    // ("on Sunday from Kent sang"); the shared
    // forward walk crosses them all.
    verb_after_pp_objects(tokens, i + 1)
}

/// Whether a subject head stands before the
/// preposition at `pos` in `tokens` (round 23,
/// R23-5): the backward walk crosses determiners,
/// noun-like subject words, and — where a stack
/// stands before this preposition — an earlier
/// PP's object run and its preposition, exactly
/// as `segment_subject_before`'s forward-facing
/// twin crosses them walking back from a verb.
/// The head is a noun-like word that is not
/// itself a verb form.
fn subject_head_before(tokens: &[String], pos: usize) -> bool {
    let mut head = false;
    let mut object_crossed = false;
    let mut j = pos;
    while j > 0 {
        let w = tokens[j - 1].as_str();
        if matches!(w, "the" | "a" | "an") || POSSESSIVE_DETERMINERS.contains(&w) {
            j -= 1;
        } else if noun_like_word(w) {
            if !is_finite_verb_form(w) {
                head = true;
            }
            object_crossed = true;
            j -= 1;
        } else if weekday_word(w).is_some()
            && ((j >= 2 && is_subject_preposition(tokens[j - 2].as_str()))
                || (j >= 3
                    && matches!(tokens[j - 2].as_str(), "the" | "a" | "an")
                    && is_subject_preposition(tokens[j - 3].as_str()))
                // Round 24 (R24-7): a relative phrase
                // standing as an earlier PP's object
                // in the stack ("the choir from next
                // Sunday on November 1 sang") crosses
                // the same way a weekday object does.
                || (j >= 3
                    && matches!(tokens[j - 2].as_str(), "next" | "this")
                    && is_subject_preposition(tokens[j - 3].as_str()))
                || (j >= 4
                    && matches!(tokens[j - 2].as_str(), "next" | "this")
                    && matches!(tokens[j - 3].as_str(), "the" | "a" | "an")
                    && is_subject_preposition(tokens[j - 4].as_str())))
        {
            object_crossed = true;
            j -= 1;
        } else if is_subject_preposition(w) && object_crossed {
            object_crossed = false;
            j -= 1;
        } else if is_tolerated_adverb(w) || AUXILIARY_WORDS.contains(&w) {
            j -= 1;
        } else {
            break;
        }
    }
    head
}

/// Whether the clause's verb stands at or after
/// `start` in `tokens`, crossing the object
/// material of a subject's PP STACK (round 23,
/// R23-5): from `start`, cross noun-like object
/// continuations; where a subject preposition
/// stands, cross it and its own object — a
/// determiner-wearing noun run, a weekday object
/// (its own determiner included), or a framed
/// date's word run, which is noun-like word by
/// word — and repeat, any number of PPs in any
/// preposition order. The walk ends at the first
/// word that is none of that material: the verb
/// stands there, or nowhere.
fn verb_after_pp_objects(tokens: &[String], start: usize) -> bool {
    let mut k = start;
    loop {
        while tokens
            .get(k)
            .is_some_and(|w| noun_like_word(w) && !is_finite_verb_form(w))
        {
            k += 1;
        }
        if !tokens.get(k).is_some_and(|w| is_subject_preposition(w)) {
            break;
        }
        // Cross this PP: the preposition, any
        // determiners its object wears, and the
        // object itself.
        let mut m = k + 1;
        while tokens.get(m).is_some_and(|w| {
            matches!(w.as_str(), "the" | "a" | "an") || POSSESSIVE_DETERMINERS.contains(&w.as_str())
        }) {
            m += 1;
        }
        // Round 24 (R24-7): the PP's object may be
        // a relative phrase ("on next Sunday") —
        // cross both of its words as one object.
        if matches!(
            tokens.get(m).map(String::as_str),
            Some("next") | Some("this")
        ) && tokens.get(m + 1).is_some_and(|w| weekday_word(w).is_some())
        {
            k = m + 2;
            continue;
        }
        if tokens.get(m).is_some_and(|w| weekday_word(w).is_some()) {
            k = m + 1;
            continue;
        }
        let object_start = m;
        while tokens
            .get(m)
            .is_some_and(|w| noun_like_word(w) && !is_finite_verb_form(w))
        {
            m += 1;
        }
        if m == object_start {
            return false;
        }
        k = m;
    }
    match tokens.get(k) {
        Some(w) => {
            (is_finite_verb_form(w) && verb_form_is_finite(tokens, k))
                || auxiliary_heads_clause(tokens, k)
        }
        None => false,
    }
}

/// Whether the framed date standing at
/// `start..start + used` in `tokens` is consumed
/// wholly as the object of a prepositional phrase
/// inside a subject run (round 23, R23-5, review
/// F9 + red team RT23-10): a subject preposition
/// stands directly before the date, a subject head
/// stands before the preposition, and the clause's
/// own verb stands after the date — "the team
/// after November 1 sang", "the choir on November
/// 15 sang", after ANY preposition, exactly as
/// after "from". Such a date is the subject
/// phrase's own material: it asserts nothing of
/// its own and competes with no other date; where
/// a composition names the dates beside its
/// assertion, it is named like the "from" twin's
/// date is. A date whose PP is not a subject PP —
/// no head before it, or no clause verb after it —
/// keeps its standing readings.
fn date_in_subject_pp(tokens: &[String], start: usize, used: usize) -> bool {
    if start == 0 || !is_subject_preposition(tokens[start - 1].as_str()) {
        return false;
    }
    if !subject_head_before(tokens, start - 1) {
        return false;
    }
    verb_after_pp_objects(tokens, start + used)
}

/// Whether the relative phrase standing at `start`
/// in `tokens` ("next Sunday" / "this Sunday") is
/// consumed wholly as the object of a prepositional
/// phrase inside a subject run (round 24, R24-7):
/// the relative-phrase twin of `date_in_subject_pp`
/// — a subject preposition stands directly before
/// the phrase (a determiner may stand between the
/// preposition and the phrase), a subject head
/// stands before the preposition, and the clause's
/// own verb stands after the phrase: "the choir
/// from next Sunday sang", "the choir before this
/// Sunday was singing". Such a phrase is the
/// subject phrase's own material: it asserts
/// nothing of its own and takes no primacy over the
/// clause's date; where a composition names the
/// dates beside its assertion, its resolution is
/// named exactly as the framed twin's PP object is
/// named. Before this round the phrase kept its
/// signal unconsumed — it was named beside a clean
/// target in the finite forms, took first-signal
/// primacy over the framed date after "on" /
/// "after", and demoted the target to alternatives
/// behind an auxiliary. A relative phrase standing
/// anywhere else — no subject head before its
/// preposition, or no clause verb after it — keeps
/// its standing readings.
fn relative_in_subject_pp(tokens: &[String], start: usize) -> bool {
    if !matches!(
        tokens.get(start).map(String::as_str),
        Some("next") | Some("this")
    ) || !tokens
        .get(start + 1)
        .is_some_and(|w| weekday_word(w).is_some())
    {
        return false;
    }
    let prep = if start >= 1 && is_subject_preposition(tokens[start - 1].as_str()) {
        start - 1
    } else if start >= 2
        && matches!(tokens[start - 1].as_str(), "the" | "a" | "an")
        && is_subject_preposition(tokens[start - 2].as_str())
    {
        start - 2
    } else {
        return false;
    };
    if !subject_head_before(tokens, prep) {
        return false;
    }
    verb_after_pp_objects(tokens, start + 2)
}

/// Whether the weekday word at `i` in `tokens` is
/// consumed wholly as the object of a prepositional
/// phrase inside a target claim's OBJECT
/// continuation (round 24, R24-5): a subject
/// preposition stands directly before the weekday
/// (a determiner may stand between them), walking
/// BACK from that preposition across the object
/// list — noun runs, verb-form adjectives heading
/// them, determiners, conjunctions, an intervening
/// date's own words, and the earlier PPs of the
/// stack — reaches the claim's target phrase
/// without crossing an auxiliary or a finite verb,
/// and walking FORWARD from the weekday across the
/// rest of the PP stack reaches the claim's "to" /
/// "until" and the framed date it names: "(moved
/// November 1, and baked beans from Kent on Sunday
/// to November 8)", and its swapped twin "… baked
/// beans on Sunday from Kent to …". Such a weekday
/// is a modifier of the object the claim moves; it
/// spawns no relative signal, asserts no phantom
/// October 18, and is named nowhere — exactly as a
/// subject PP's weekday is consumed (R22-5/R23-5).
/// The standing "from" governance of the line is
/// untouched: the target date in these witnesses
/// reads nothing because "from" governs it, as the
/// held twin without the weekday PP does. Two
/// boundaries: a weekday PP with no target phrase
/// behind it is no claim's object continuation and
/// keeps its standing readings; and a weekday PP
/// standing in a SECOND clause — its own subject
/// and finite verb between the claim's phrase and
/// the preposition ("beans were served on Sunday")
/// — is that clause's own phrase, not a PP inside
/// the continuation, and keeps its signal.
fn weekday_in_object_continuation_pp(
    raw_tokens: &[&str],
    tokens: &[String],
    i: usize,
    today: SimpleDate,
) -> bool {
    let prep = if i >= 1 && is_subject_preposition(tokens[i - 1].as_str()) {
        i - 1
    } else if i >= 2
        && matches!(tokens[i - 1].as_str(), "the" | "a" | "an")
        && is_subject_preposition(tokens[i - 2].as_str())
    {
        i - 2
    } else {
        return false;
    };
    // Backward: reach the claim's target phrase
    // across the object continuation's own material.
    let mut j = prep;
    let mut object_crossed = false;
    let reached_target = loop {
        if j == 0 {
            break false;
        }
        if is_target_phrase(&tokens[j - 1..j]) || (j >= 2 && is_target_phrase(&tokens[j - 2..j])) {
            break true;
        }
        let w = tokens[j - 1].as_str();
        if matches!(w, "the" | "a" | "an" | "and" | "or") || POSSESSIVE_DETERMINERS.contains(&w) {
            j -= 1;
        } else if is_subject_preposition(w) {
            if !object_crossed {
                break false;
            }
            object_crossed = false;
            j -= 1;
        } else if weekday_word(w).is_some() {
            // An earlier PP's weekday object in the
            // stack, its preposition (or its
            // determiner, or its relative word)
            // standing just before it.
            let object_ok = (j >= 2 && is_subject_preposition(tokens[j - 2].as_str()))
                || (j >= 3
                    && matches!(tokens[j - 2].as_str(), "the" | "a" | "an" | "next" | "this")
                    && is_subject_preposition(tokens[j - 3].as_str()));
            if !object_ok {
                break false;
            }
            object_crossed = true;
            j -= 1;
        } else if (matches!(w, "next" | "this")
            && tokens
                .get(j)
                .is_some_and(|next| weekday_word(next).is_some()))
            || (noun_like_word(w) && !is_finite_verb_form(w))
        {
            // The relative word of an earlier PP's
            // relative-phrase object, or a noun-like
            // object word: the continuation's own
            // material either way.
            object_crossed = true;
            j -= 1;
        } else if is_finite_verb_form(w) {
            // A verb-form word the one verb test
            // reads as finite is a second clause's
            // verb — the continuation ends there.
            // A verb-form adjective heading the
            // object run ("baked beans") is the
            // continuation's own material. The verb
            // test judges the word in its own
            // segment — the words after the nearest
            // intervening date, exactly as the claim
            // reach's after-date judgment resets its
            // segment there (R18-8): an intervening
            // date's own words are not the verb
            // form's subject.
            let mut seg_start = 0usize;
            let mut s = 0usize;
            while s < j - 1 {
                match parse_date_prefix(&raw_tokens[s..], today) {
                    Some((_, used)) if s + used.max(1) < j => {
                        seg_start = s + used.max(1);
                        s += used.max(1);
                    }
                    _ => s += 1,
                }
            }
            if verb_form_is_finite(&tokens[seg_start..], j - 1 - seg_start) {
                break false;
            }
            object_crossed = true;
            j -= 1;
        } else {
            break false;
        }
    };
    if !reached_target {
        return false;
    }
    // Forward: cross the rest of the PP stack to
    // the claim's "to" / "until" and its framed date.
    let mut k = i + 1;
    loop {
        while tokens
            .get(k)
            .is_some_and(|w| noun_like_word(w) && !is_finite_verb_form(w))
        {
            k += 1;
        }
        match tokens.get(k).map(String::as_str) {
            Some("to") | Some("until") => {
                let mut m = k + 1;
                while tokens
                    .get(m)
                    .is_some_and(|w| matches!(w.as_str(), "the" | "a" | "an"))
                {
                    m += 1;
                }
                return raw_tokens
                    .get(m..)
                    .is_some_and(|rest| parse_date_prefix(rest, today).is_some());
            }
            Some(w) if is_subject_preposition(w) => {
                let mut m = k + 1;
                while tokens.get(m).is_some_and(|w| {
                    matches!(w.as_str(), "the" | "a" | "an")
                        || POSSESSIVE_DETERMINERS.contains(&w.as_str())
                }) {
                    m += 1;
                }
                if matches!(
                    tokens.get(m).map(String::as_str),
                    Some("next") | Some("this")
                ) && tokens.get(m + 1).is_some_and(|w| weekday_word(w).is_some())
                {
                    k = m + 2;
                    continue;
                }
                if tokens.get(m).is_some_and(|w| weekday_word(w).is_some()) {
                    k = m + 1;
                    continue;
                }
                if let Some((_, used)) = raw_tokens
                    .get(m..)
                    .and_then(|rest| parse_date_prefix(rest, today))
                {
                    k = m + used.max(1);
                    continue;
                }
                let object_start = m;
                while tokens
                    .get(m)
                    .is_some_and(|w| noun_like_word(w) && !is_finite_verb_form(w))
                {
                    m += 1;
                }
                if m == object_start {
                    return false;
                }
                k = m;
            }
            _ => return false,
        }
    }
}

/// Whether `word` can head the noun phrase a
/// verb form would modify standing directly
/// before it (round 21, R21-6): a noun-like word
/// that is not itself adverbial — an -ly word
/// after a verb form is that verb's adverb
/// ("played beautifully"), never its noun.
fn noun_phrase_word(word: &str) -> bool {
    noun_like_word(word) && !adverbial_word(word)
}

/// The ONE verb test for every verb-form word in
/// a segment (round 20, R20-4; round 21, R21-6,
/// the verb test v2): a verb-form word — a
/// generic -ed form, a listed stem form, an
/// irregular form, an -ing form — is a finite
/// verb when a subject stands before it in its
/// segment (the run rule of
/// `segment_subject_before`: "the big choir sang",
/// "choir members sang", "Pastor Lee sang"), or
/// when no subject stands before it but the word
/// cannot head a noun phrase there: an irregular
/// past form ("sang hymns" — an irregular form
/// has no adjective use the test can read, the
/// distinction between "sang" / "sung" and
/// "baked" that the spelling itself carries), or
/// a form followed by a determiner or an adverb
/// ("folded the bulletins", "played
/// beautifully"), or a bare form with no noun
/// phrase around it at all ("then played"). It
/// reads as a modifier only when a bare noun-like
/// word follows it directly with no subject
/// before it ("baked beans", "planned dinner",
/// "planning meeting"), or when it stands
/// postposed at its segment's end after the noun
/// it modifies ("dinner rolls, baked"). A verb
/// form standing directly after a determiner or
/// a possessive is that noun phrase's own
/// material, as it has always been ("the baked
/// beans").
fn verb_form_is_finite(segment: &[String], off: usize) -> bool {
    if off > 0 {
        let prev = segment[off - 1].as_str();
        // Round 23 (R23-8): the immediate shield is
        // scoped by the past-only split exactly as
        // the spanned shield below is — a past-only
        // form ("sang", "rang", "took") has no
        // adjective use in ANY position, the
        // determiner directly before it
        // notwithstanding, so "the sang hymns" reads
        // as a clause exactly as "the big sang
        // hymns" already did. Participle and regular
        // forms keep the shield ("the taken seats",
        // "the baked beans").
        if (matches!(prev, "the" | "a" | "an") || POSSESSIVE_DETERMINERS.contains(&prev))
            && !IRREGULAR_PAST_ONLY_FORMS.contains(&segment[off].as_str())
        {
            return false;
        }
    }
    // The noun phrase it would head may itself
    // wear a weekday modifier (round 20, R20-6's
    // reading): "printed Sunday bulletins" heads
    // the same noun phrase "printed bulletins"
    // does.
    let heads_noun_phrase = segment
        .get(off + 1)
        .is_some_and(|next| noun_phrase_word(next))
        || (segment
            .get(off + 1)
            .is_some_and(|next| weekday_word(next).is_some())
            && segment
                .get(off + 2)
                .is_some_and(|next| noun_phrase_word(next)));
    // Round 22 (R22-6): the determiner shield
    // spans its noun phrase — an irregular
    // PARTICIPLE form heading a noun phrase is
    // shielded by a determiner (or possessive)
    // standing ANYWHERE before it in the segment,
    // the R19-6 active-"for" rule applied to the
    // reach's shield: "the big left luggage" and
    // "the big sung hymns" are noun phrases exactly
    // as "the left luggage" is, the intervening
    // adjective being the phrase's own material.
    // Before this round the shield covered only
    // the immediately preceding word, and the
    // irregular branch below returned finite
    // before the heads-a-noun-phrase test was ever
    // reached, so one stacked adjective killed the
    // claim. The span is scoped to the forms with
    // a participle use — the branch whose
    // unconditional return was the defect. A
    // past-tense-only irregular ("sang", "rang",
    // "took") has no adjective use in any
    // position: "the team from Kent sang hymns"
    // stays a clause by the subject rule below,
    // its subject's determiner notwithstanding,
    // exactly as a regular form with a subject
    // head before it ("the choir folded
    // bulletins") does. An unshielded participle
    // ("sung hymns", no determiner phrase before
    // it) still ends the reach by the branch
    // below, and a verb form that heads no noun
    // phrase ("the choir sang") is judged by the
    // subject rule as always.
    if heads_noun_phrase
        && IRREGULAR_FINITE_FORMS.contains(&segment[off].as_str())
        && !IRREGULAR_PAST_ONLY_FORMS.contains(&segment[off].as_str())
        && segment[..off].iter().any(|prev| {
            matches!(prev.as_str(), "the" | "a" | "an")
                || POSSESSIVE_DETERMINERS.contains(&prev.as_str())
        })
    {
        return false;
    }
    if segment_subject_before(segment, off) {
        return true;
    }
    // Round 23 (R23-6): a verb-form word standing
    // directly after a weekday word and heading a
    // noun phrase is the weekday-modifier phrase's
    // adjective, not a finite verb — "Sunday left
    // luggage", "Sunday taken seats" in an object
    // continuation are the phrase's own material,
    // and the irregular branch below must not make
    // a clause of them. The one exception is a
    // weekday standing as a subject PP's object:
    // in "the choir on Sunday sang hymns" the word
    // after the weekday is the clause's verb, and
    // it stays finite by the subject rule and this
    // branch alike (the preposition — or its
    // determiner — stands directly before the
    // weekday there).
    if heads_noun_phrase
        && off > 0
        && weekday_word(&segment[off - 1]).is_some()
        && !(off >= 2 && is_subject_preposition(segment[off - 2].as_str()))
        && !(off >= 3
            && matches!(segment[off - 2].as_str(), "the" | "a" | "an")
            && is_subject_preposition(segment[off - 3].as_str()))
    {
        return false;
    }
    if IRREGULAR_FINITE_FORMS.contains(&segment[off].as_str()) {
        return true;
    }
    if heads_noun_phrase {
        return false;
    }
    if off + 1 == segment.len() && segment[..off].iter().any(|w| noun_phrase_word(w)) {
        return false;
    }
    true
}

/// Whether `word` can stand as a noun-like word in
/// its segment (round 19, R19-4): a content word —
/// not a determiner, conjunction, preposition,
/// possessive, auxiliary, weekday, governor, or
/// tolerated adverb, and not an irregular finite
/// form. This is the word class that can head the
/// noun phrase an -ed adjective modifies, and the
/// word class that can stand as a clause's subject.
fn noun_like_word(word: &str) -> bool {
    !word.is_empty()
        && !matches!(
            word,
            "the"
                | "a"
                | "an"
                | "and"
                | "or"
                | "but"
                | "then"
                | "to"
                | "for"
                | "from"
                | "of"
                | "on"
                | "in"
                | "at"
                | "by"
        )
        && !AUXILIARY_WORDS.contains(&word)
        && !POSSESSIVE_DETERMINERS.contains(&word)
        && weekday_word(word).is_none()
        && !direct_single_governor(word)
        && !is_tolerated_adverb(word)
        && !IRREGULAR_FINITE_FORMS.contains(&word)
}

/// Whether a run of words holds a finite verb form
/// (round 18, R18-8): the test `target_claim`'s
/// post-date reach and the active-"for" object scan
/// share. A verb form standing directly after a
/// determiner or a possessive is that noun phrase's
/// own material, not its verb — a finite verb never
/// stands immediately after a determiner in a
/// clause; it stands after the subject's head noun.
/// So "the meeting" and "the baked beans" hold no
/// verb, while "choir sang" and "the choir sang"
/// do (the verb follows the head noun, not the
/// determiner).
///
/// Round 19 (R19-4) tempered only generic -ed
/// forms this way; round 20 (R20-4) makes the
/// test one for every verb form (see
/// `verb_form_is_finite`): listed stems,
/// irregulars, and -ing forms in the adjective
/// slot ("planned dinner", "planning meeting")
/// read as modifiers exactly as "baked beans"
/// does, a real clause ("choir folded
/// bulletins") ends the reach exactly as "choir
/// sang" does, and a postposed form ("dinner
/// rolls, baked") continues it. Round 21 (R21-6)
/// states the test as v2 (see
/// `verb_form_is_finite`): the subject is a run
/// of words, an irregular form is finite wherever
/// it stands, and an auxiliary heading its own
/// clause ("choir was singing") is a finite verb
/// too (see `auxiliary_heads_clause`).
/// Whether the auxiliary at `off` in its segment
/// heads a finite clause of its own (round 21,
/// R21-6): a subject stands before it in its
/// segment (the run rule of
/// `segment_subject_before`) and an ordinary
/// verb form stands after it as its complement,
/// across any tolerated adverbs — "choir was
/// singing", "choir had sung". Such an auxiliary
/// is itself the clause's finite verb: a
/// progressive or perfect second clause is a
/// clause, and its auxiliary is not a bare
/// marker word standing in front of the date the
/// clause names.
/// The complement verb form of the auxiliary at
/// `off`, if one stands there (round 21, R21-6;
/// round 22, R22-4): the verb form the auxiliary
/// carries, across the adverbs that may stand
/// between them. The claim's own passive
/// constructions keep their standing readings —
/// this is clause DETECTION only, not the shared
/// adverb walk's spending rule: the tolerated
/// adverbs cross as they always have, and ONE
/// further adverb of any kind (an unlisted -ly
/// adverb, "not", "never") may stand in the slot
/// too — "choir was loudly singing" is a finite
/// clause exactly as "choir was happily singing"
/// is. Two unlisted adverbs, or any other word,
/// end the scan. The complement must be an
/// ordinary verb form, not a governor participle:
/// an auxiliary whose participle is itself a
/// governor ("it was moved", "it was cancelled")
/// is the governor machinery's own construction,
/// spent or unspent by the spent-predicate rule —
/// the auxiliary keeps its standing there and
/// this rule does not reach it.
///
/// Round 23 (R23-4): the auxiliary's slot holds
/// TWO words, not one — each a negation or an
/// adverb, listed or not. The two tolerances
/// R22-4 pinned separately compose: a negation
/// AND an adverb ("was not loudly played", "was
/// not ever played", "was really not played",
/// "was not yet played"), and two adverbs ("was
/// softly gently singing", "was loudly
/// beautifully played"), read as one auxiliary
/// complement exactly as either tolerance alone
/// did. "ever" and "yet" are the slot's non--ly
/// adverbs the witnesses wear; a third slot word
/// ends the scan, as any other word does.
fn aux_complement(segment: &[String], off: usize) -> Option<usize> {
    let mut j = off + 1;
    let mut unlisted_used = 0usize;
    while let Some(w) = segment.get(j) {
        let w = w.as_str();
        if is_tolerated_adverb(w) {
            j += 1;
        } else if adverbial_word(w) || matches!(w, "not" | "never" | "ever" | "yet") {
            if unlisted_used >= 2 {
                break;
            }
            unlisted_used += 1;
            j += 1;
        } else {
            break;
        }
    }
    match segment.get(j) {
        Some(w) if is_finite_verb_form(w) && !is_single_governor(w) => Some(j),
        _ => None,
    }
}

fn auxiliary_heads_clause(segment: &[String], off: usize) -> bool {
    if !AUXILIARY_WORDS.contains(&segment[off].as_str()) {
        return false;
    }
    if !segment_subject_before(segment, off) {
        return false;
    }
    aux_complement(segment, off).is_some()
}

/// Whether the auxiliary at `off` heads a finite
/// clause with NO subject before it in its
/// segment (round 22, R22-4): a subjectless
/// auxiliary + participle construction — "then
/// was played", "then was singing" — is a second
/// clause whose subject the reader supplies from
/// the claim's own clause, exactly as its "were"
/// twin has always read. Before this round bare
/// "was" alone of the auxiliaries escaped the
/// reading: it is itself a `direct_single_governor`
/// (the R12 past-marker family), so the claim
/// died on the between-words fence AND the target
/// date stood governed by the "was" of its own
/// clause — total silence. A bare "was" with no
/// participle after it ("(was November 1)") keeps
/// governing as the past marker it is: this rule
/// needs the complement the clause is built from.
fn auxiliary_subjectless_clause(segment: &[String], off: usize) -> bool {
    if !AUXILIARY_WORDS.contains(&segment[off].as_str()) {
        return false;
    }
    if segment_subject_before(segment, off) {
        return false;
    }
    aux_complement(segment, off).is_some()
}

fn segment_has_finite_verb(segment: &[String]) -> bool {
    segment.iter().enumerate().any(|(off, w)| {
        (is_finite_verb_form(w) && verb_form_is_finite(segment, off))
            || auxiliary_heads_clause(segment, off)
            // Round 22 (R22-4): a subjectless
            // auxiliary + participle construction
            // is a finite clause for the reach's
            // test too ("then was played hymns" —
            // the participle reads as a modifier
            // standing alone, but the construction
            // is a clause).
            || auxiliary_subjectless_clause(segment, off)
    })
}

/// Whether an active-"for" object phrase holds a
/// finite verb form (round 19, R19-6): the shared
/// segment test, with the determiner shield widened
/// for this scan only — a determiner (or possessive)
/// standing ANYWHERE before the verb-form word in
/// the object protects it, because the word is the
/// object phrase's head and the words between the
/// determiner and the head are the phrase's own
/// modifiers: "the big meeting", "the church
/// meeting", "the Johnson meeting", "a big meeting"
/// hold no verb, exactly as bare "the meeting" does.
/// The claim reach keeps the immediate-determiner
/// rule (see `segment_has_finite_verb`): there, "the
/// choir sang" is a clause, not a noun phrase.
fn active_for_object_has_finite_verb(object: &[String]) -> bool {
    object.iter().enumerate().any(|(off, w)| {
        if !is_finite_verb_form(w) {
            return false;
        }
        if object[..off].iter().any(|prev| {
            matches!(prev.as_str(), "the" | "a" | "an")
                || POSSESSIVE_DETERMINERS.contains(&prev.as_str())
        }) {
            return false;
        }
        // The rest of the shared test, judged on the
        // word's own segment position: the one verb
        // test of round 20 (R20-4) — a verb form is
        // finite only with a noun-like subject
        // standing before it in the object — which
        // subsumes the generic -ed adjective and
        // subject rules of R19-4.
        verb_form_is_finite(object, off)
    })
}

/// The backward walk over the passive-"for"
/// construction's own complement — the slot between
/// its auxiliary and its participle (round 24,
/// R24-6): the walk the passive-for scan uses, and
/// no other scan. Detection (`aux_complement`)
/// crosses an unlisted adverb in this slot on its
/// own — one unlisted adverb, or two, spend exactly
/// as a listed pair does — while the shared
/// spending walk (`aux_adverb_walk_bounded`)
/// crossed an unlisted adverb only beside a
/// negation, so "(it was softly gently moved for
/// November 1)" detected as a clause on the claim
/// surface and spent nothing on its own surface,
/// and "(it was loudly moved for November 1)" spent
/// nothing either. This walk aligns the passive-for
/// complement with detection: the slot holds up to
/// TWO slot words — a tolerated adverb, an
/// adverbial word of any kind, "ever" / "yet", or a
/// negation (a negation counts as one slot word,
/// as the R23-4 rule states) — in the post- and
/// pre-auxiliary positions alike, and the fronted
/// stack standing before the subject is judged by
/// `fronted_stack_ok` under the same cap, exactly
/// as the shared walk judges it. The governor
/// scans keep the shared walk and its caps
/// unchanged: they guard a governor's reach across
/// dates, not a local complement inside one
/// construction, and a three-word slot still does
/// not spend there.
fn passive_for_slot_walk(tokens: &[String], start: usize) -> Option<(usize, usize)> {
    let mut j = start;
    let mut aux_seen = false;
    let mut adverbs_used = 0usize;
    while j > 0 {
        let prev = tokens[j - 1].as_str();
        if AUXILIARY_WORDS.contains(&prev) {
            aux_seen = true;
            j -= 1;
        } else if (matches!(prev, "not" | "never") && aux_beyond_slot(tokens, j - 1))
            || is_tolerated_adverb(prev)
            || adverbial_word(prev)
            || matches!(prev, "ever" | "yet")
        {
            // One slot word, of whichever kind: a
            // negation (its auxiliary beyond it, as
            // the shared walk requires), a tolerated
            // adverb, an adverbial word of any kind,
            // or "ever" / "yet" — each counts once
            // toward the two-word cap.
            if adverbs_used >= 2 {
                return None;
            }
            adverbs_used += 1;
            j -= 1;
        } else {
            break;
        }
    }
    if !aux_seen {
        return None;
    }
    if j > 0 && !fronted_stack_ok(tokens, j, adverbs_used) {
        return None;
    }
    Some((j, adverbs_used))
}

fn passive_for_zone_start(before: &[String], window: &[String]) -> Option<usize> {
    if window.len() < 3 || window.last().map(String::as_str) != Some("for") {
        return None;
    }
    let for_pos = window.len() - 1;
    // The participle phrase ends just before "for":
    // a phrasal pair ("put back"), or one word.
    let participle_start = if for_pos >= 2
        && is_phrasal_governor_pair(&window[for_pos - 2], &window[for_pos - 1])
    {
        for_pos - 2
    } else {
        let word = window[for_pos - 1].as_str();
        let is_participle = is_cancellation_verb(word) || word.ends_with("ed") || word == "held";
        if !is_participle {
            return None;
        }
        for_pos - 1
    };
    // Round 18 (R18-10): the tolerance is up to TWO
    // consecutive listed adverbs — "(it was just
    // recently moved for November 1)" spends like
    // its one-adverb twins. Three stacked still
    // break the scan, as any unlisted word does.
    // Round 20 (R20-5): the walk is the one shared
    // walk (`aux_adverb_walk`) — the same list and
    // the same cap in the post-auxiliary,
    // pre-auxiliary, and fronted positions.
    // Round 24 (R24-6): the passive-for scan's own
    // complement walk is `passive_for_slot_walk` —
    // it crosses adverbs as detection does (an
    // unlisted adverb needs no negation beside it),
    // under the same two-slot-word cap; the shared
    // walk and the governor scans it serves are
    // unchanged.
    let (j, adverbs_used) = passive_for_slot_walk(window, participle_start)?;
    // A fronted stack standing in its own clause
    // before the window (the window starts after
    // its clause break) is judged by the same rule:
    // the adverbial run at the end of `before`
    // must be listed throughout and fit the cap
    // together with the walk's own adverbs.
    let mut run = 0usize;
    let mut run_all_listed = true;
    for w in before.iter().rev() {
        if adverbial_word(w) {
            run += 1;
            if !is_tolerated_adverb(w) {
                run_all_listed = false;
            }
        } else {
            break;
        }
    }
    if run > 0 && (!run_all_listed || run + adverbs_used > 2) {
        return None;
    }
    Some(j)
}

/// The start of an ACTIVE "for" construction
/// standing at the end of a governor window, if one
/// does (round 18, R18-9): the window ends in "for",
/// and a displacement phrase — a displacement verb
/// in any form, or a "put back" / "push back" pair,
/// the same family the passive rule is scoped by —
/// stands before the "for" with nothing between it
/// and the "for" but the phrase's own object: object
/// words only, no second verb, no governor, no
/// auxiliary, no weekday. "(we moved it for
/// November 1)" names November 1 as the occasion in
/// the active voice, as "(it was moved for
/// November 1)" does in the passive: the verb is
/// spent on its object, and "for" names the date.
/// The scope is the displacement family alone, as
/// the disposition states — an active cancellation
/// ("we cancelled it for November 1") keeps
/// governing its date, and the bare parity "(we
/// moved it November 1)" stands: with no "for"
/// there is no construction here. Round 19
/// (R19-6): the object's no-second-verb test is
/// `active_for_object_has_finite_verb`, whose
/// determiner shield reaches across the object's
/// modifiers.
fn active_for_zone_start(window: &[String]) -> Option<usize> {
    if window.len() < 2 || window.last().map(String::as_str) != Some("for") {
        return None;
    }
    let for_pos = window.len() - 1;
    // The nearest displacement phrase ending at or
    // before the "for": try the phrase that ends
    // closest to the "for" first, so an object that
    // happens to hold a verb form of its own ("we
    // moved the move for …") is read by the nearest
    // phrase, and a phrase too far back — behind a
    // second verb — never reaches across it.
    for end in (1..=for_pos).rev() {
        for len in 1..=2usize {
            if end < len {
                continue;
            }
            let phrase = &window[end - len..end];
            if !is_displacement_phrase(phrase) {
                continue;
            }
            let object = &window[end..for_pos];
            let object_ok = !active_for_object_has_finite_verb(object)
                && object.iter().all(|w| {
                    !w.is_empty()
                        && !direct_single_governor(w)
                        && !is_cancellation_verb(w)
                        && !AUXILIARY_WORDS.contains(&w.as_str())
                        && weekday_word(w).is_none()
                        && w.as_str() != "to"
                        && w.as_str() != "for"
                });
            if object_ok {
                return Some(end - len);
            }
        }
    }
    None
}

fn governed_before(before: &[String], window: &[String]) -> bool {
    // Round 17 (R17-4): governor phrases standing
    // inside a passive "for" construction at the
    // window's end — the auxiliary and the participle
    // themselves — are the construction, not
    // governors of the date the "for" names. A
    // governor standing before the construction
    // still governs.
    // Round 18 (R18-9): the active "for" construction
    // is a zone the same way — the displacement verb
    // and its object are the construction, not
    // governors of the occasion date. The zone that
    // starts earlier covers both.
    let mut for_zone_start = passive_for_zone_start(before, window);
    if let Some(active_start) = active_for_zone_start(window) {
        for_zone_start = Some(match for_zone_start {
            Some(passive_start) => passive_start.min(active_start),
            None => active_start,
        });
    }
    for i in 0..window.len() {
        for len in 1..=3usize {
            let Some(phrase) = window.get(i..i + len) else {
                break;
            };
            if !phrase_governs(window, i, len) {
                continue;
            }
            if let Some(zone_start) = for_zone_start {
                if i >= zone_start {
                    continue;
                }
            }
            // Round 15 (R15-1): a displacement
            // phrase standing directly before a
            // "to" governs the date it moves FROM;
            // the date the "to" names is its
            // target and asserts — "(rescheduled
            // to November 8)" and "(moved to
            // November 8)" name November 8, the
            // held "(moved to ...)" shape, and in
            // "(November 1, moved to November 8)"
            // the phrase governs November 1
            // through `governed_after` instead.
            if window.get(i + len).map(String::as_str) == Some("to")
                && is_displacement_phrase(phrase)
            {
                continue;
            }
            return true;
        }
    }
    false
}

/// The possessive determiners (round 14, R14-5).
const POSSESSIVE_DETERMINERS: [&str; 7] = ["his", "her", "their", "my", "our", "your", "its"];

/// Whether the phrase standing at `i` in `slice`
/// governs, in its context (round 11–14): the
/// families of `is_governor_phrase`, with one
/// contextual narrowing — round 14 (R14-5), a bare
/// "wont" standing immediately after a possessive
/// determiner is the noun "wont" (accustomed), not
/// the unpunctuated "won't", so "(his wont,
/// November 1)" names its date. The contraction
/// "won't" itself is unaffected.
/// Whether `word`, standing at `i` in `slice`, is an
/// -s form of a cancellation stem heading a
/// determiner-headed noun phrase (round 16, RT16-6):
/// the word itself is an -s inflection (not the bare
/// stem), and walking back over noun-like words —
/// words that are not verb forms, governors,
/// auxiliaries, or weekdays — reaches a determiner
/// ("the", "a", "an", "these", "those") within three
/// words. "The drops" and "the cough drops" are noun
/// phrases: a finite verb cannot stand as the head of
/// one, so the -s word does not govern in that slot,
/// exactly as the possessive-preceded "wont" (round
/// 14, R14-5) is the noun after its determiner. Bare
/// -s forms ("choir cancels", "ticket drops" as an
/// event label) keep governing: no determiner heads
/// them.
fn determiner_headed_s_form(slice: &[String], i: usize, word: &str) -> bool {
    let is_s_form = CANCELLATION_STEMS.iter().any(|stem| {
        word.strip_suffix('s') == Some(*stem) || word.strip_suffix("es") == Some(*stem)
    });
    if !is_s_form || i == 0 {
        return false;
    }
    let mut j = i;
    let mut steps = 0;
    while j > 0 && steps < 3 {
        let prev = slice[j - 1].as_str();
        if matches!(prev, "the" | "a" | "an" | "these" | "those") {
            return true;
        }
        // Only a noun-like word may stand between the
        // determiner and its head; anything else ends
        // the phrase.
        if direct_single_governor(prev)
            || is_cancellation_verb(prev)
            || AUXILIARY_WORDS.contains(&prev)
            || weekday_word(prev).is_some()
        {
            return false;
        }
        j -= 1;
        steps += 1;
    }
    false
}

/// Whether the "from" at `i` in `slice` stands
/// inside a subject prepositional phrase (round
/// 22, R22-5): a subject head run before it ("the
/// team from …"), its own noun-like object run
/// after it ("… Kent"), and the clause's finite
/// verb after that ("… sang"). Such a "from"
/// governs nothing — it is the subject phrase's
/// own preposition, not the origin marker of the
/// "(from November 1)" family — but its clause
/// stands in the target date's governor window,
/// and before this round the window's scan could
/// not tell a subject's "from Kent" from a
/// governing "from", so "the team from Kent sang"
/// silenced both dates for every verb form. A
/// "from" with no subject head before it, or no
/// clause verb after its object, governs as it
/// always has.
fn from_stands_in_subject_pp(slice: &[String], i: usize) -> bool {
    if !subject_head_before(slice, i) {
        return false;
    }
    // The object itself: determiners, then a
    // weekday object or a noun-like run (a framed
    // date's words included).
    let mut k = i + 1;
    while slice.get(k).is_some_and(|w| {
        matches!(w.as_str(), "the" | "a" | "an") || POSSESSIVE_DETERMINERS.contains(&w.as_str())
    }) {
        k += 1;
    }
    let object_start = k;
    if slice.get(k).is_some_and(|w| weekday_word(w).is_some()) {
        k += 1;
    } else {
        while slice
            .get(k)
            .is_some_and(|w| noun_like_word(w) && !is_finite_verb_form(w))
        {
            k += 1;
        }
    }
    if k == object_start {
        return false;
    }
    // Round 23 (R23-5): the clause's verb may
    // stand past a SECOND prepositional phrase —
    // "the team from Kent on Sunday sang" — the
    // shared forward walk crosses the whole stack
    // before judging the verb.
    verb_after_pp_objects(slice, k)
}

/// Whether the negation at `i` in `slice` stands
/// in an auxiliary's adverb slot (round 22, R22-4;
/// round 23, R23-4): scanning back from the
/// negation crosses only slot words — tolerated
/// adverbs, adverbs of any kind, "ever" / "yet",
/// and further negations — and reaches an
/// auxiliary whose complement (the `aux_complement`
/// scan) stands AFTER the negation. R22-4's rule
/// reached only a negation standing directly
/// after its auxiliary ("was not played"); the
/// two-word slot puts the negation in either
/// position ("was really not played", "was loudly
/// never played", "was ever not played"), and in
/// every position the negation is spent on its
/// verb, not on the date its clause names. A
/// negation with no such construction around it
/// ("was not November 8", "(not November 8)")
/// governs as always.
fn negation_in_aux_slot(slice: &[String], i: usize) -> bool {
    let mut j = i;
    while j > 0 {
        let w = slice[j - 1].as_str();
        if AUXILIARY_WORDS.contains(&w) {
            return match aux_complement(slice, j - 1) {
                Some(comp) => comp > i,
                None => false,
            };
        }
        if is_tolerated_adverb(w)
            || adverbial_word(w)
            || matches!(w, "not" | "never" | "ever" | "yet")
        {
            j -= 1;
        } else {
            return false;
        }
    }
    false
}

fn phrase_governs(slice: &[String], i: usize, len: usize) -> bool {
    let Some(phrase) = slice.get(i..i + len) else {
        return false;
    };
    if !is_governor_phrase(phrase) {
        return false;
    }
    // Round 22 (R22-5): a "from" standing inside a
    // subject prepositional phrase is that phrase's
    // own word, not an origin governor (see
    // `from_stands_in_subject_pp`).
    if len == 1 && phrase[0] == "from" && from_stands_in_subject_pp(slice, i) {
        return false;
    }
    // Round 16 (RT16-6): a determiner-headed -s word
    // is a noun phrase's head, not a verb.
    if len == 1 && determiner_headed_s_form(slice, i, &phrase[0]) {
        return false;
    }
    // Round 21 (R21-6): an auxiliary standing as
    // the finite verb of its own clause — a
    // subject before it in the slice and a verb
    // form after it as its complement ("choir was
    // singing") — is that clause's verb, not the
    // bare past marker: the "was" of "choir was
    // singing to November 8" does not govern the
    // date its own clause names, exactly as the
    // "was" of "(choir was singing, November 1)"
    // never governed across its clause break. A
    // bare "was" with no clause around it ("(was
    // November 1)") keeps governing as the past
    // marker it is, and a governor participle
    // standing after the auxiliary ("it was
    // moved") governs by its own phrase below.
    // Round 22 (R22-4): the same holds for a
    // SUBJECTLESS auxiliary + participle
    // construction standing in the window — "then
    // was played to November 8" names November 8
    // in a second clause the reader completes,
    // and its "was" does not govern the date its
    // own clause names either; November 8 stands
    // on its own standing there.
    if len == 1
        && phrase[0] == "was"
        && (auxiliary_heads_clause(slice, i) || auxiliary_subjectless_clause(slice, i))
    {
        return false;
    }
    // Round 22 (R22-4): a negation standing in an
    // auxiliary's own adverb slot — "then was not
    // played", "then was never played" — negates
    // the clause's verb, not the date the clause
    // names, and does not govern past it. Round 23
    // (R23-4): the slot holds two words, so the
    // negation may stand in either position in it
    // (see `negation_in_aux_slot`).
    if len == 1 && matches!(phrase[0].as_str(), "not" | "never") && negation_in_aux_slot(slice, i) {
        return false;
    }
    if len == 1
        && (phrase[0] == "wont" || phrase[0].starts_with("wont-"))
        && i > 0
        && POSSESSIVE_DETERMINERS.contains(&slice[i - 1].as_str())
    {
        return false;
    }
    true
}

/// The cancellation verb stems (round 14, R14-1):
/// the verbs whose inflections govern by stem, the
/// way round 13's contraction class governs by
/// suffix. Noun derivations are not forms of the
/// verb: "cancellation" and "postponement" govern
/// nothing (pinned in the r14a tests).
const CANCELLATION_STEMS: [&str; 13] = [
    "cancel",
    "scratch",
    "drop",
    "postpone",
    "remove",
    "delay",
    "omit",
    "disregard",
    "forget",
    // Round 15 (R15-1): the displacement verbs
    // govern like the cancellation verbs — a date
    // a sender moved, shifted, bumped, or
    // rescheduled is precisely the date that is
    // not it, and with no new date named there is
    // nothing to lose by silencing it. Where a
    // new date IS named through "to", the target
    // asserts (see `governed_before`).
    "move",
    "shift",
    "bump",
    "reschedule",
];

/// The displacement verb stems (round 15,
/// R15-1): the subset of the cancellation stems
/// that name a move to another date rather than
/// an end. A displacement phrase standing before
/// a "to" governs the date it moves FROM, never
/// the target the "to" names.
const DISPLACEMENT_STEMS: [&str; 4] = ["move", "shift", "bump", "reschedule"];

/// Whether `word` is a form of one of the
/// displacement verbs (round 15, R15-1).
fn is_displacement_verb(word: &str) -> bool {
    DISPLACEMENT_STEMS
        .iter()
        .any(|stem| is_verb_form(word, stem))
}

/// Whether a two-word phrase is one of the
/// phrasal governors whose first word inflects
/// (round 15, R15-1): "call off" in any of
/// call's forms (called off, calling off, calls
/// off), "put back" and "push back" in any of
/// their forms (pushed back, putting back), the
/// older "put off" in put's forms, and "forget
/// about" in any of forget's forms, the
/// irregular "forgot about" included. A bare
/// "call", "push", or "put" with no second word
/// governs nothing, as a bare "put" never has.
fn is_phrasal_governor_pair(a: &str, b: &str) -> bool {
    (is_verb_form(a, "call") && b == "off")
        || (is_verb_form(a, "put") && matches!(b, "off" | "back"))
        || (is_verb_form(a, "push") && b == "back")
        || (is_verb_form(a, "forget") && b == "about")
}

/// Whether one run of words is a displacement
/// phrase (round 15, R15-1): a displacement verb
/// on its own, or a "put back" / "push back"
/// pair. These are the phrases a "to" after them
/// turns into a from/to shape.
fn is_displacement_phrase(phrase: &[String]) -> bool {
    let words: Vec<&str> = phrase.iter().map(String::as_str).collect();
    match words.as_slice() {
        [w] => is_displacement_verb(w),
        [a, b] => (is_verb_form(a, "put") || is_verb_form(a, "push")) && *b == "back",
        _ => false,
    }
}

/// A stem with a doubled final consonant undoubled:
/// "dropp" -> "drop", "cancell" -> "cancel".
/// Anything else is returned unchanged.
fn undoubled(stem: &str) -> &str {
    let bytes = stem.as_bytes();
    if bytes.len() >= 2 && bytes[bytes.len() - 1] == bytes[bytes.len() - 2] {
        &stem[..stem.len() - 1]
    } else {
        stem
    }
}

/// Whether `word` is an inflection of the verb
/// `stem`: the base form, an -ed form, an -ing
/// form, or an -s form, with the spelling adjustments English
/// makes — a final "e" drops before -ing
/// (postponing, removing) and takes a bare "d" for
/// the past (postponed, removed), and a final
/// consonant may double (dropping, omitting, and
/// "cancel"'s British cancelling / cancelled).
/// Round 15 (R15-2): the -s form (cancels,
/// postpones, scratches — the last with the -es
/// spelling a sibilant-final stem takes) is a form
/// of the verb like any other, and "forget" has an
/// irregular past the suffix rules cannot build:
/// "forgot" and "forgotten" are its forms too.
fn is_verb_form(word: &str, stem: &str) -> bool {
    if word == stem {
        return true;
    }
    if stem == "forget" && matches!(word, "forgot" | "forgotten") {
        return true;
    }
    if let Some(base) = word.strip_suffix('s') {
        if base == stem {
            return true;
        }
    }
    if let Some(base) = word.strip_suffix("es") {
        if base == stem {
            return true;
        }
    }
    if let Some(base) = word.strip_suffix("ing") {
        if base == stem || undoubled(base) == stem {
            return true;
        }
        if stem.ends_with('e') && &stem[..stem.len() - 1] == base {
            return true;
        }
    }
    if let Some(base) = word.strip_suffix("ed") {
        if base == stem || undoubled(base) == stem {
            return true;
        }
    }
    if stem.ends_with('e') && word.strip_suffix('d') == Some(stem) {
        return true;
    }
    false
}

/// Whether `word` is a form of one of the
/// cancellation verbs (round 14, R14-1).
fn is_cancellation_verb(word: &str) -> bool {
    CANCELLATION_STEMS
        .iter()
        .any(|stem| is_verb_form(word, stem))
}

/// Whether one token is a governor on its own
/// (round 11–14): the single words of the families,
/// stated once. Round 14 (R14-4): a hyphenated
/// compound governs when a part of it governs or
/// its parts form a governor phrase — "do-not-use",
/// "no-longer", "hold-off" — the hyphens split for
/// this test only; the token is read whole
/// everywhere else.
fn is_single_governor(word: &str) -> bool {
    if direct_single_governor(word) {
        return true;
    }
    if !word.contains('-') {
        return false;
    }
    let parts: Vec<String> = word
        .split('-')
        .filter(|p| !p.is_empty())
        .map(str::to_string)
        .collect();
    if parts.len() < 2 {
        return false;
    }
    for i in 0..parts.len() {
        for len in 1..=3usize {
            let Some(phrase) = parts.get(i..i + len) else {
                break;
            };
            if is_governor_phrase(phrase) {
                return true;
            }
        }
    }
    false
}

/// Whether one token, read whole, is a governor on
/// its own (round 11–14): the contraction class,
/// the negation and origin singles, "nope" (round
/// 14, R14-3), and the cancellation verbs by stem
/// (round 14, R14-1).
fn direct_single_governor(w: &str) -> bool {
    w == "cannot"
        || w == "wont"
        || w.ends_with("n't")
        || w.ends_with("n\u{2019}t")
        || is_cancellation_verb(w)
        || matches!(
            w,
            "not"
                | "ignore"
                | "never"
                | "originally"
                | "from"
                | "skip"
                | "except"
                | "previously"
                | "formerly"
                | "was"
                | "no"
                | "nope"
                | "isnt"
                | "arent"
                | "wasnt"
                | "werent"
                | "wrong"
        )
}

/// Whether one run of words is a governor phrase
/// (round 11–13): the single words, the pairs, and
/// the triples the families recognize, stated once
/// so the window check and the clause-trailing
/// check (round 13, R13-2) read the same families.
/// Round 13 (R13-1): negation gains the contraction
/// class AS A CLASS — any token ending in "n't"
/// (either apostrophe), plus the unpunctuated
/// "wont" and "cannot", is a negation governor, the
/// rule a reader applies rather than another word
/// list; cancellation gains the verbs cancel /
/// scratch / drop / postpone / remove and the pair
/// "hold off"; the label word "wrong" (its colon
/// stripped by `group_token`) disowns its date.
fn is_governor_phrase(phrase: &[String]) -> bool {
    let words: Vec<&str> = phrase.iter().map(String::as_str).collect();
    match words.as_slice() {
        [w] => is_single_governor(w),
        [a, b] => {
            is_phrasal_governor_pair(a, b)
                || matches!(
                    (*a, *b),
                    ("no", "longer")
                        | ("don't", "use")
                        | ("didn't", "use")
                        | ("dont", "use")
                        | ("didnt", "use")
                        | ("rather", "than")
                        | ("instead", "of")
                        | ("never", "mind")
                        | ("old", "date")
                        | ("hold", "off")
                        | ("put", "off")
                        | ("forget", "about")
                )
        }
        [a, b, c] => {
            matches!((*a, *b, *c), ("used", "to", "be") | ("do", "not", "use"))
        }
        _ => false,
    }
}

/// Whether the parts of a hyphenated token, read
/// as consecutive phrases, contain a cancellation
/// phrase (round 14, R14-2/R14-4): "hold-off"
/// standing after a date cancels it.
fn hyphenated_cancellation(parts: &[String]) -> bool {
    for i in 0..parts.len() {
        for len in 1..=2usize {
            let Some(phrase) = parts.get(i..i + len) else {
                break;
            };
            if is_cancellation_phrase(phrase) {
                return true;
            }
        }
    }
    false
}

/// Whether one run of words is a cancellation
/// phrase (round 14, R14-2): a cancellation verb in
/// any of its stem forms, the phrasal pairs "hold
/// off" / "put off", "never mind", "forget about",
/// and a cancellation verb carrying its object
/// ("drop it", "scratch that", "cancel it"). These
/// are the forms that attach to a date they FOLLOW,
/// the way "cancelled" always has — the trailing
/// position draws from the same family definition
/// as the before position, so "(remove November 1)"
/// and "(November 1, removed)" disqualify alike.
fn is_cancellation_phrase(phrase: &[String]) -> bool {
    let words: Vec<&str> = phrase.iter().map(String::as_str).collect();
    match words.as_slice() {
        [w] => {
            is_cancellation_verb(w)
                || (w.contains('-') && {
                    let parts: Vec<String> = w
                        .split('-')
                        .filter(|p| !p.is_empty())
                        .map(str::to_string)
                        .collect();
                    parts.len() >= 2 && hyphenated_cancellation(&parts)
                })
        }
        [a, b] => {
            is_phrasal_governor_pair(a, b)
                || matches!(
                    (*a, *b),
                    ("hold", "off") | ("put", "off") | ("never", "mind") | ("forget", "about")
                )
                || (is_cancellation_verb(a) && matches!(*b, "it" | "that"))
        }
        _ => false,
    }
}

/// The auxiliary words (round 16, R16-3): auxiliaries
/// as a class, not a five-word list — the be-forms,
/// the have-forms, and the get-forms. A governor
/// participle standing as the predicate of one of
/// these is spent on its subject.
const AUXILIARY_WORDS: [&str; 15] = [
    "am", "is", "are", "was", "were", "be", "been", "being", //
    "has", "had", "have", //
    "get", "gets", "got", "getting",
];

/// Whether a word may stand as an adverb of
/// tolerance between an auxiliary and its participle
/// (round 16, R16-3): one of the common short adverbs
/// — "it was just moved" is the witness shape.
/// Round 17 (R17-7): the tolerance set is a CLOSED
/// list — the short adverbs plus the attested -ly
/// adverbs named here. There is no -ly catch-all:
/// the catch-all admitted nouns and names standing
/// in the adverb slot ("it was family moved", "it
/// was kelly moved" spent like "recently"), and any
/// non-tolerated intervening word governs as it
/// always has outside this slot.
fn is_tolerated_adverb(word: &str) -> bool {
    matches!(
        word,
        "just"
            | "already"
            | "still"
            | "also"
            | "even"
            | "now"
            | "soon"
            | "often"
            | "recently"
            | "previously"
            | "originally"
            | "finally"
            | "officially"
            | "tentatively"
            | "temporarily"
            | "currently"
            | "newly"
            | "jointly"
            // Round 18 (R18-10): then, briefly,
            // shortly, formally, unanimously, and
            // happily join the list — ordinary office
            // adverbs in exactly this slot, whose
            // absence silenced the occasion date.
            // Round 19 (R19-7): very, plain, and
            // simply join — the ordinary intensifiers
            // of "just plain moved" and "very recently
            // moved", whose absence broke a stack in
            // every scan.
            // Round 20 (R20-5): really, actually,
            // truly, quickly, definitely, probably,
            // and carefully join — the next tier of
            // ordinary office adverbs, whose absence
            // silenced even a target claim ("it was
            // really moved to November 8" read
            // nothing). The list is 34 words in all.
            | "then"
            | "briefly"
            | "shortly"
            | "formally"
            | "unanimously"
            | "happily"
            | "very"
            | "plain"
            | "simply"
            | "really"
            | "actually"
            | "truly"
            | "quickly"
            | "definitely"
            | "probably"
            | "carefully"
    )
}

/// Whether `word` is an adverbial word without
/// being on the tolerated list (round 20, R20-5):
/// an -ly word, or "yesterday". Used only to
/// recognise a fronted adverb STACK standing in
/// its own clause — a stack is a run of adverbial
/// words, listed or not; whether it tolerates is
/// judged on the list itself.
fn adverbial_word(word: &str) -> bool {
    is_tolerated_adverb(word) || word == "yesterday" || word.ends_with("ly")
}

/// A run of words as a fronted adverb stack, if
/// it is one (round 20, R20-5): non-empty, and
/// every word adverbial. Returns the stack's word
/// count and whether every word is on the
/// tolerated list.
fn adverb_stack_clause(words: &[String]) -> Option<(usize, bool)> {
    if words.is_empty() || !words.iter().all(|w| adverbial_word(w)) {
        return None;
    }
    let all_listed = words.iter().all(|w| is_tolerated_adverb(w));
    Some((words.len(), all_listed))
}

/// Whether the fronted adverb stack standing in
/// `tokens[..j]`, if one stands there, is
/// tolerated (round 20, R20-5): the stack is the
/// run of adverbial words in the adverb slot —
/// directly before the subject, or in the
/// pre-auxiliary slot the walk stopped at — and
/// it tolerates exactly when every word of it is
/// on the tolerated list and it plus the
/// `adverbs_used` already crossed by the walk
/// fits the one two-word cap. An unlisted
/// adverbial word in the slot, or a third listed
/// one, is not tolerated.
fn fronted_stack_ok(tokens: &[String], j: usize, adverbs_used: usize) -> bool {
    // The auxiliary chain may wear an auxiliary the
    // walk did not cross — a modal standing before
    // a crossed auxiliary ("it will be postponed"):
    // those are the construction's own words, not a
    // fronted stack.
    let mut end = j;
    while end > 0 && AUXILIARY_WORDS.contains(&tokens[end - 1].as_str()) {
        end -= 1;
    }
    if end == 0 {
        return true;
    }
    // Where the stack stands, when one does: if the
    // run's last word is adverbial, the walk stopped
    // at the stack itself — the pre-auxiliary slot
    // ("it yesterday was moved": the subject stands
    // before the run). Otherwise the run's last word
    // is the subject (wearing any determiners or
    // possessives directly before it), and the stack
    // is the adverbial run directly before the
    // subject — the detached and in-clause fronted
    // positions. Words before that run are earlier
    // material (a governor clause, another clause's
    // words), not this construction's stack.
    let stack_end = if adverbial_word(&tokens[end - 1]) {
        end
    } else {
        let mut subj = end - 1;
        while subj > 0
            && (matches!(tokens[subj - 1].as_str(), "the" | "a" | "an")
                || POSSESSIVE_DETERMINERS.contains(&tokens[subj - 1].as_str()))
        {
            subj -= 1;
        }
        subj
    };
    let mut stack_start = stack_end;
    while stack_start > 0 && adverbial_word(&tokens[stack_start - 1]) {
        stack_start -= 1;
    }
    let mut total = adverbs_used;
    for w in &tokens[stack_start..stack_end] {
        if is_tolerated_adverb(w) {
            total += 1;
            if total > 2 {
                return false;
            }
        } else {
            return false;
        }
    }
    true
}

/// The backward auxiliary / adverb walk the three
/// adverb scans share (round 20, R20-5): starting
/// just before `start`, cross auxiliaries and
/// tolerated adverbs in either order — the
/// post-auxiliary stack ("it was just moved") and
/// the pre-auxiliary stack ("it just was moved")
/// are one walk — under ONE budget of two
/// tolerated adverbs for the whole walk, and then
/// judge the fronted stack standing before the
/// subject by the same list and the same cap
/// (`fronted_stack_ok`). Returns the index the
/// walk stopped at and the adverbs it crossed, or
/// None when no auxiliary stands in the walk, a
/// third tolerated adverb stands in it, or the
/// fronted stack is not tolerated. Before round
/// 20 each scan walked on its own, the fronted
/// position was never judged at all, and a stack
/// split across the auxiliary spent past the cap.
fn aux_adverb_walk(tokens: &[String], start: usize) -> Option<(usize, usize)> {
    aux_adverb_walk_bounded(tokens, start, 0, true)
}

/// `aux_adverb_walk` with the fronted stack judged
/// only within `tokens[bound..]`: a caller whose
/// run carries earlier material that is not this
/// construction's clause (an earlier date's span,
/// an earlier clause) bounds the fronted judgment
/// to the construction's own clause, exactly as
/// the unbounded walk's callers' runs are already
/// clause-scoped.
/// Whether an auxiliary stands just beyond the
/// slot words ending at `pos` (round 23, R23-4):
/// scanning back from `pos` crosses only slot
/// words (tolerated adverbs, adverbs of any kind,
/// "ever" / "yet", negations) and reaches an
/// auxiliary. The spending walk crosses a
/// negation only on this condition: a negation
/// in an AUXILIARY's slot is spent on its verb,
/// but a fronted negation with no auxiliary
/// beyond it ("not moved to November 8") is a
/// governor standing before its phrase, and the
/// walk must not pull it into the claim zone.
fn aux_beyond_slot(tokens: &[String], pos: usize) -> bool {
    let mut k = pos;
    while k > 0 {
        let w = tokens[k - 1].as_str();
        if AUXILIARY_WORDS.contains(&w) {
            return true;
        }
        if is_tolerated_adverb(w)
            || adverbial_word(w)
            || matches!(w, "not" | "never" | "ever" | "yet")
        {
            k -= 1;
        } else {
            return false;
        }
    }
    false
}

fn aux_adverb_walk_bounded(
    tokens: &[String],
    start: usize,
    bound: usize,
    require_aux: bool,
) -> Option<(usize, usize)> {
    let mut j = start;
    let mut aux_seen = false;
    let mut adverbs_used = 0usize;
    // Round 23 (R23-4): a negation in the slot is
    // spent on its verb in every scan, this walk
    // included — "(it was not moved for November
    // 1)" asserts its occasion exactly as "(it was
    // moved for November 1)" does. The negation
    // counts toward the walk's two-word cap like
    // any slot word, so a three-word slot (a
    // negation and two adverbs, in any order) does
    // not spend, as a three-adverb slot never has.
    // An UNLISTED adverb crosses only beside a
    // negation — the negation already crossed
    // nearer the verb, or the negation standing
    // just past it — the passive constructions'
    // analogue of R22-4's single-unlisted slot:
    // "not loudly", "loudly never", and "not ever"
    // spend; an unlisted adverb alone ("loudly
    // moved") and two unlisted adverbs still do
    // not, the tolerated list governing which
    // adverbs spend as it always has.
    let mut negation_seen = false;
    while j > 0 {
        let prev = tokens[j - 1].as_str();
        if AUXILIARY_WORDS.contains(&prev) {
            aux_seen = true;
            j -= 1;
        } else if matches!(prev, "not" | "never") && aux_beyond_slot(tokens, j - 1) {
            if adverbs_used >= 2 {
                return None;
            }
            adverbs_used += 1;
            negation_seen = true;
            j -= 1;
        } else if is_tolerated_adverb(prev)
            || ((adverbial_word(prev) || matches!(prev, "ever" | "yet"))
                && (negation_seen || (j >= 2 && matches!(tokens[j - 2].as_str(), "not" | "never"))))
        {
            if adverbs_used >= 2 {
                return None;
            }
            adverbs_used += 1;
            j -= 1;
        } else {
            break;
        }
    }
    if require_aux && !aux_seen {
        return None;
    }
    if j > bound && !fronted_stack_ok(&tokens[bound..], j - bound, adverbs_used) {
        return None;
    }
    Some((j, adverbs_used))
}

/// Whether the governor phrase standing at `i` in
/// `clause` is a SPENT predicate (round 16, R16-3):
/// a cancellation phrase — displacement and
/// cancellation alike, single and phrasal — standing
/// as the predicate of an auxiliary construction:
/// scanning back from the phrase crosses at most two
/// tolerated adverbs (round 18, R18-10) and then at
/// least one auxiliary
/// word ("it was moved", "they are moved", "it is
/// being moved", "it has moved", "it got moved", "it
/// was just moved", "it was put back"). Such a phrase
/// is spent on its subject whatever the verb, the
/// held "(it was moved, November 1)" garnish reading
/// generalized: the following date is the group's
/// occasion date. Round 15's rule was this shape
/// scoped to single displacement stems after five
/// listed auxiliaries; the boundary was arbitrary,
/// as the round-16 review states.
fn is_spent_predicate(clause: &[String], i: usize, len: usize) -> bool {
    is_spent_predicate_fronted(clause, i, len, false)
}

/// `is_spent_predicate` with the fronted-stack
/// judgment of round 20 (R20-5): the backward walk
/// is the one shared walk (`aux_adverb_walk`) —
/// the same tolerated list and the same two-word
/// cap in the post-auxiliary, pre-auxiliary, and
/// fronted positions. `fronted_invalid` carries
/// the judgment for a fronted stack standing in
/// its OWN clause before this one, which the
/// clause slice itself cannot see: the caller
/// that sliced the clause (the governor window
/// walk) judges that stack and reports it here —
/// an invalid fronted stack leaves the predicate
/// unspent, exactly as an invalid in-clause stack
/// does.
fn is_spent_predicate_fronted(
    clause: &[String],
    i: usize,
    len: usize,
    fronted_invalid: bool,
) -> bool {
    let Some(phrase) = clause.get(i..i + len) else {
        return false;
    };
    if !is_cancellation_phrase(phrase) {
        return false;
    }
    if fronted_invalid {
        return false;
    }
    aux_adverb_walk(clause, i).is_some()
}

/// Whether a clause ending at a clause break holds
/// an UNSPENT governor as its trailing element
/// (round 13, R13-2): a governor phrase after which
/// nothing stands in the clause but date-referring
/// filler ("the", "date", "it"). Such a governor
/// has governed no date yet, so the break does not
/// erase it. A cancellation standing in the clause
/// directly after a date is the exception: it was
/// spent on that date by `governed_after`
/// ("(November 1, cancelled, November 8)" still
/// asserts November 8), so it carries no further.
/// `unspent_trailing_governor` with a fronted
/// stack standing in its own clause before this
/// one already judged invalid (round 20, R20-5;
/// see `is_spent_predicate_fronted`): a predicate
/// the fronted stack leaves unspent is an unspent
/// governor of this clause like any other.
fn unspent_trailing_governor_fronted(
    clause: &[String],
    clause_follows_a_date: bool,
    fronted_invalid: bool,
) -> bool {
    for i in 0..clause.len() {
        for len in 1..=3usize {
            let Some(phrase) = clause.get(i..i + len) else {
                break;
            };
            if !phrase_governs(clause, i, len) {
                continue;
            }
            // Round 15 (R15-1), generalized round 16
            // (R16-3): a governor participle standing
            // as an auxiliary's predicate ("it was
            // moved", "it was cancelled", "it was put
            // back") is that clause's own predicate,
            // already spent on its subject — the held
            // "(it was moved, November 1)" shape names
            // its date, and so do its synonyms.
            if is_spent_predicate_fronted(clause, i, len, fronted_invalid) {
                continue;
            }
            // Round 14 (R14-2): the spent exception
            // covers the whole cancellation family,
            // not just cancelled/canceled — a trailing
            // "postponed" or "drop it" standing after
            // a date was spent on that date by
            // `governed_after`, the same as the
            // participles always were.
            if clause_follows_a_date && is_cancellation_phrase(phrase) {
                continue;
            }
            let suffix = &clause[i + len..];
            if suffix
                .iter()
                .all(|w| matches!(w.as_str(), "the" | "date" | "it"))
            {
                return true;
            }
        }
    }
    false
}

/// Whether a governor standing AFTER a date inside
/// its group disqualifies that date (round 12,
/// R12-7a; parity completed round 14, R14-2): a
/// cancellation phrase in the words between this
/// date and the next one — or the group's end —
/// cancels the date it follows, and a forward
/// governor ("not", "no longer") standing after the
/// group's LAST date, with no later date left for
/// it to govern, falls back on the date it follows:
/// "(November 1, cancelled)", "(November 1, drop
/// it)", and "(November 1, not)" contribute
/// nothing. The held case stands apart by the same
/// rule: in "(November 1, not October 25)" the
/// trailing "not" has a date after it to govern, so
/// November 1 is untouched.
///
/// Round 15 (R15-6): the trailing reach is
/// clause-scoped — a phrase governs the date it
/// follows only when every clause between that
/// date and the phrase is itself a cancellation
/// or filler clause. A content clause breaks the
/// reach: "(November 1, choir practice, ignore)"
/// now contributes November 1, because the
/// "ignore" stands behind words that are neither.
fn governed_after(
    raw_tokens: &[&str],
    tokens: &[String],
    date_end: usize,
    next_start: Option<usize>,
) -> bool {
    let seg_start = date_end.min(tokens.len());
    let seg_end = next_start.unwrap_or(tokens.len()).min(tokens.len());
    if seg_start >= seg_end {
        return false;
    }
    // The clause starts inside the segment: a new
    // clause opens after every clause break.
    let mut clause_starts: Vec<usize> = vec![seg_start];
    for j in seg_start + 1..seg_end {
        if raw_tokens.get(j - 1).map(|t| clause_break_after(t)) == Some(true) {
            clause_starts.push(j);
        }
    }
    let after = &tokens[seg_start..seg_end];
    for i in 0..after.len() {
        for len in 1..=3usize {
            let Some(phrase) = after.get(i..i + len) else {
                break;
            };
            // Round 14 (R14-2): a cancellation phrase
            // standing after a date cancels the date
            // it follows — the trailing position draws
            // from the same family as the before
            // position, not the cancelled/canceled
            // pair alone. Round 15 (R15-6): subject
            // to the clause scope stated above.
            // The rest of the family is prospective:
            // standing between two dates it governs
            // the later one (judged by that date's own
            // window), and only falls back on the date
            // it follows when no later date is left —
            // the standing "not" / "no longer" rule,
            // now read off the one family definition.
            // The clause the phrase stands in, judged
            // with the scope rule below.
            let pos = seg_start + i;
            let clause_idx = clause_starts.iter().rposition(|s| *s <= pos).unwrap_or(0);
            let own_start = clause_starts[clause_idx];
            let own_end = clause_starts
                .get(clause_idx + 1)
                .copied()
                .unwrap_or(seg_end);
            let own_clause = &tokens[own_start..own_end];
            // Round 16 (RT16-6): a determiner-headed
            // -s word is a noun phrase's head in the
            // trailing position too — "(November 1,
            // the drops)" names its date.
            // Round 16 (R16-2): a prospective phrase
            // standing as its OWN complete clause
            // between two dates — the clause ends at
            // a clause break inside the segment —
            // spends on the date it follows, together
            // with the run's cancellation clauses:
            // "(November 1, not, drop it, November 8)"
            // governs November 1 by the run. A
            // prospective word standing in the next
            // date's own clause ("(November 1, not
            // November 8)") keeps the standing rule:
            // it governs the later date, judged by
            // that date's window, and this date is
            // untouched.
            let own_clause_complete = own_end > own_start
                && raw_tokens.get(own_end - 1).map(|t| clause_break_after(t)) == Some(true);
            let prospective_governs = next_start.is_some()
                && phrase_governs(after, i, len)
                && !is_cancellation_phrase(phrase)
                && own_clause_complete
                && is_prospective_only_clause(own_clause);
            let governs = (is_cancellation_phrase(phrase)
                && !(len == 1 && determiner_headed_s_form(after, i, &phrase[0])))
                || (next_start.is_none() && phrase_governs(after, i, len))
                || prospective_governs;
            if !governs {
                continue;
            }
            // Round 19 (R19-1): a cancellation phrase
            // standing in a clause of event content
            // that runs directly into the NEXT date —
            // no clause break closes the clause before
            // that date stands — is the next date's own
            // clause governor, judged by that date's
            // window, not a governor of the date it
            // follows: in "(early service November 1,
            // late service cancelled November 1)" the
            // "cancelled" cancels the late service's
            // November 1, and the early service's date
            // is untouched by it. The standing
            // between-dates rule is untouched where it
            // has always applied: the phrase's clause
            // is closed by a break before the next
            // date, or the clause is a governor/filler
            // clause of its own ("(November 1,
            // cancelled November 8)" still governs
            // November 1 here as before).
            if next_start.is_some()
                && is_cancellation_phrase(phrase)
                && !own_clause_complete
                && !is_governor_family_clause(own_clause)
            {
                continue;
            }
            // Every clause between the date and the
            // phrase's clause, judged by the one scope
            // rule — round 16 (R16-2): the governor
            // families compose, so a run mixing
            // cancellation, prospective, and filler
            // clauses is in scope as a whole.
            let mut in_scope = true;
            for k in 0..clause_idx {
                let c_start = clause_starts[k];
                let c_end = clause_starts.get(k + 1).copied().unwrap_or(seg_end);
                let clause = &tokens[c_start..c_end];
                if !is_governor_family_clause(clause) {
                    in_scope = false;
                    break;
                }
            }
            if in_scope {
                return true;
            }
        }
    }
    false
}

/// Whether a weekday word standing directly before a
/// date inside its group contradicts that date
/// (round 11, R11-8): the same wrong-weekday-yields-
/// no-date rule the line readers obey, applied
/// inside groups — "(Monday November 1)" names an
/// impossible day, since November 1, 2026 is a
/// Sunday, and contributes nothing.
fn contradicted_before(tokens: &[String], date_start: usize, date: SimpleDate) -> bool {
    if date_start == 0 {
        return false;
    }
    match weekday_word(&tokens[date_start - 1]) {
        Some(named) => named != date.weekday(),
        None => false,
    }
}

/// The partial date an "or" tail offers inside a
/// group (round 11, R11-6): "(November 1 or the 8th)"
/// offers two dates, not one — the tail names only a
/// day, so it takes the month and year of the full
/// date standing before the "or".
fn partial_or_alternative(
    tokens: &[String],
    qualified: &[(SimpleDate, usize, usize)],
) -> Option<SimpleDate> {
    for (i, w) in tokens.iter().enumerate() {
        if w != "or" {
            continue;
        }
        let base = qualified
            .iter()
            .rev()
            .find(|(_, start, _)| *start < i)
            .map(|(d, _, _)| *d)?;
        let mut j = i + 1;
        if tokens.get(j).map(String::as_str) == Some("the") {
            j += 1;
        }
        let day_token = tokens.get(j)?;
        let mut digits: &str = day_token.as_str();
        for suffix in ["st", "nd", "rd", "th"] {
            if let Some(stripped) = digits.strip_suffix(suffix) {
                digits = stripped;
                break;
            }
        }
        let digits: String = digits.to_string();
        if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        let day: u32 = digits.parse().ok()?;
        let candidate = SimpleDate {
            year: base.year,
            month: base.month,
            day,
        };
        if candidate.is_valid() && candidate != base {
            return Some(candidate);
        }
    }
    None
}

/// A target-rule governor phrase (round 16, R16-1):
/// a displacement phrase, or a form of "postpone" —
/// the phrases whose whole meaning is a move to
/// another date.
fn is_target_phrase(phrase: &[String]) -> bool {
    is_displacement_phrase(phrase) || (phrase.len() == 1 && is_verb_form(&phrase[0], "postpone"))
}

/// The group-level target claim standing for the
/// date occurrence `idx` (round 16, R16-1): the
/// group contains a target phrase and a "to" /
/// "until" naming THIS date as the phrase's target —
/// the date is the first date standing after that
/// "to" / "until", the words between it and the date
/// are nothing but determiners, and the words between
/// the phrase and the "to" are an object noun phrase,
/// "back", or the like: no governor, cancellation
/// verb, auxiliary, or weekday stands among them.
/// Round 17 (R17-5): the reach from the phrase to
/// its "to" is bounded by the phrase's CLAUSE, not
/// by a word count — the round-16 four-word fence
/// lost a five-word object ("moved the November
/// potluck planning meeting to November 8")
/// silently. A clause break standing among the
/// in-between words ends the reach (a comma inside
/// or closing an intervening DATE is that date's
/// own punctuation, not a clause break in the
/// reach: the
/// second "to" of a correction — "moved to
/// November 8, then to November 15" — still claims
/// its date, round 17 R17-1). Round 17 (R17-3): a
/// "from" standing directly before a date among the
/// in-between words is part of the construction —
/// "(moved from November 1 to November 8)" claims
/// its target, and November 1 is governed and named
/// as the from-date like every other claimed
/// group's other date. Returns the claim zone: the
/// token the zone starts at — the phrase, extended
/// back over its auxiliaries and one tolerated
/// adverb, so the passive's "was" is part of the
/// construction, not a past marker governing the
/// target — and the "to" / "until" token.
struct TargetClaim {
    zone_start: usize,
    to_pos: usize,
}

fn target_claim(
    raw_tokens: &[&str],
    tokens: &[String],
    positional: &[(SimpleDate, usize, usize)],
    idx: usize,
) -> Option<TargetClaim> {
    let start = positional[idx].1;
    let mut to_pos = start;
    while to_pos > 0 {
        to_pos -= 1;
        if !matches!(tokens[to_pos].as_str(), "to" | "until") {
            continue;
        }
        // Between the "to" and the date: determiners
        // only ("moved to the November 8").
        if !tokens[to_pos + 1..start]
            .iter()
            .all(|w| matches!(w.as_str(), "the" | "a" | "an" | ""))
        {
            continue;
        }
        // A date standing between this "to" and the
        // claimed date means the "to" names THAT date.
        if positional.iter().any(|(_, s, _)| *s > to_pos && *s < start) {
            continue;
        }
        // The nearest target phrase ending at or
        // before the "to".
        let mut found: Option<(usize, usize)> = None;
        for ps in 0..to_pos {
            for len in 1..=2usize {
                let Some(phrase) = tokens.get(ps..ps + len) else {
                    break;
                };
                if ps + len > to_pos {
                    break;
                }
                if is_target_phrase(phrase) {
                    found = Some((ps, ps + len));
                }
            }
        }
        let Some((ps, pe)) = found else {
            continue;
        };
        // Between the phrase and the "to": an object
        // noun phrase, "back", or the like — no word
        // with a governing or verbal life of its own,
        // and the reach stays inside the phrase's
        // clause (round 17, R17-5 above). A "from"
        // directly before an intervening date is the
        // construction's own word (round 17, R17-3).
        let between = &tokens[pe..to_pos];
        // A comma standing inside an intervening
        // date's own span — after its day, before
        // its year, or closing the date — is that
        // date's punctuation, not a clause break in
        // the reach.
        // Round 18 (R18-4): a comma standing directly
        // BEFORE an intervening date is the object's
        // list comma in front of that date — "(moved
        // the meeting, November 1, 2026, and the
        // dinner to November 8)" writes its object as
        // a list whose middle item is the from-date.
        // The intervening date joins the claim's
        // governed set as the from-date; it never
        // breaks the reach by standing there, and
        // neither does the list comma that introduces
        // it. Its comma-after-date twin has always
        // claimed this way.
        let clause_bounded = (pe..to_pos).all(|j| {
            let in_date_span = positional
                .iter()
                .any(|(_, s, used)| j >= *s && j < s + used);
            let before_intervening_date = positional.iter().any(|(_, s, _)| *s == j + 1 && *s > pe);
            // Round 20 (R20-4): a comma standing on a
            // noun-like word directly before a verb-form
            // word the one verb test reads as a postposed
            // modifier ("dinner rolls, baked to …") is
            // that modifier phrase's own punctuation, not
            // a clause break in the reach — the postposed
            // adjective continues the object continuation
            // it postposes. A comma before a verb form the
            // test reads as finite (a real clause's verb)
            // keeps its standing clause-break force.
            // Round 21 (R21-10): the exemption is fenced
            // to that pinned postposed shape — the verb
            // form the comma's continuation contains,
            // reached by crossing only the noun-like
            // words of its own run ("beans, choir baked":
            // the continuation's verb form is "baked",
            // judged in its segment exactly as the
            // directly-postposed form is), and exempt
            // only when the one verb test reads THAT
            // form as a postposed modifier in its
            // segment. Before this fence, a two-noun
            // segment behind the comma ("beans," opening
            // a segment whose second noun heads a real
            // clause) left the comma standing as a break
            // the pinned shape's own logic should have
            // crossed the same way — the claim died, the
            // group offered both dates as alternatives,
            // and the plain path filed the vacated date.
            let postposed_modifier_break = noun_like_word(&tokens[j]) && {
                let mut k = j + 1;
                while k < to_pos
                    && tokens
                        .get(k)
                        .is_some_and(|w| noun_like_word(w) && !is_finite_verb_form(w))
                {
                    k += 1;
                }
                match tokens.get(k) {
                    Some(next) if k < to_pos && is_finite_verb_form(next) => {
                        let seg_start = positional
                            .iter()
                            .filter(|(_, s, used)| s + used <= k)
                            .map(|(_, s, used)| s + used)
                            .max()
                            .unwrap_or(pe);
                        let seg = &tokens[seg_start..to_pos];
                        !verb_form_is_finite(seg, k - seg_start)
                    }
                    _ => false,
                }
            };
            !clause_break_after(raw_tokens[j])
                || in_date_span
                || before_intervening_date
                || postposed_modifier_break
        });
        let between_ok = clause_bounded
            && between.iter().enumerate().all(|(off, w)| {
                let j = pe + off;
                let from_before_date =
                    w.as_str() == "from" && positional.iter().any(|(_, s, _)| *s == j + 1);
                // Round 18 (R18-3): a word standing
                // inside an intervening SIGNAL's own
                // span — an earlier target in the same
                // claim chain, framed or relative — is
                // that target's own word, not a clause
                // word of this reach. A framed target's
                // words already passed this test (no
                // month or day word is a weekday word);
                // a relative target's "Sunday" is one,
                // so a chain whose earlier target was
                // written relatively ("moved to next
                // Sunday, then to November 15") lost
                // every later claim and the superseded
                // relative target asserted. The span
                // exemption covers the weekday test
                // only: governor, cancellation, and
                // auxiliary words still end the reach
                // wherever they stand.
                let in_signal_span = positional
                    .iter()
                    .any(|(_, s, used)| j >= *s && j < s + used);
                // Round 20 (R20-6): a weekday word
                // followed by a noun-like word is a
                // noun phrase's modifier ("printed
                // Sunday bulletins"), not a clause
                // word of the reach — the same
                // reading the group signal scan
                // takes, so the reach crosses the
                // noun phrase it modifies.
                // Round 23 (R23-6): the one
                // weekday-modifier reading, verb-form
                // adjective shape included ("Sunday
                // taken seats" in an object
                // continuation is the phrase's own
                // material, and the claim crosses it).
                let weekday_modifier = weekday_modifier_at(tokens, j);
                from_before_date
                    || (!direct_single_governor(w)
                        && !is_cancellation_verb(w)
                        && !AUXILIARY_WORDS.contains(&w.as_str())
                        && (in_signal_span || weekday_word(w).is_none() || weekday_modifier))
            });
        if !between_ok {
            continue;
        }
        // Round 18 (R18-8): the reach's continuation
        // AFTER an intervening date is judged on its
        // own. The reach continues only into the
        // target construction itself — the "to"
        // directly ("moved November 1, to November
        // 8"), its lead-ins ("then to", "and to"),
        // or an object continuation of the phrase's
        // own object ("…, and the dinner to
        // November 8", the R18-4 witness: a
        // conjunction or determiner carrying the
        // object list on). A new finite clause
        // standing there — a subject and its finite
        // verb, "choir sang" — is clause content,
        // not object continuation, and it ends the
        // reach: "(moved November 1, choir sang to
        // November 8)" claims nothing for November
        // 8. The distinguisher is the finite verb:
        // object continuations name things, a second
        // clause does something. Only the words
        // after the intervening date's own span are
        // judged — the object BEFORE the date is the
        // phrase's object however it is written, and
        // a verb-repeated target ("moved November 1,
        // moved to November 8") re-anchors the phrase
        // itself, so no intervening date stands after
        // the phrase that claims.
        let clause_content_after_date = positional.iter().any(|(_, s, used)| {
            *s >= pe && s + used <= to_pos && segment_has_finite_verb(&tokens[s + used..to_pos])
        });
        if clause_content_after_date {
            continue;
        }
        // The claim zone extends back from the phrase
        // over its auxiliaries and up to two tolerated
        // adverbs (round 18, R18-10): "it was moved
        // to …" claims its target whole.
        // Round 20 (R20-5): the extension is the one
        // shared walk (`aux_adverb_walk_bounded`) —
        // the same list and the same two-word cap in
        // the post-auxiliary, pre-auxiliary, and
        // fronted positions, the fronted judgment
        // bounded to the phrase's own clause (after
        // the last clause break and after any earlier
        // date's span). A fronted stack standing in
        // its own clause before that bound is judged
        // by the same rule, as in the other scans.
        let bound = {
            let mut b = 0usize;
            for (_, s, used) in positional.iter() {
                if s + used <= ps {
                    b = b.max(s + used);
                }
            }
            for (j, raw) in raw_tokens.iter().enumerate().take(ps) {
                if clause_break_after(raw) {
                    b = b.max(j + 1);
                }
            }
            b
        };
        let Some((walk_start, adverbs_used)) = aux_adverb_walk_bounded(tokens, ps, bound, false)
        else {
            continue;
        };
        if bound > 0 {
            let mut pcs = 0usize;
            for (j, raw) in raw_tokens.iter().enumerate().take(bound - 1) {
                if clause_break_after(raw) {
                    pcs = j + 1;
                }
            }
            if let Some((count, all_listed)) = adverb_stack_clause(&tokens[pcs..bound]) {
                if !all_listed || count + adverbs_used > 2 {
                    continue;
                }
            }
            // Round 21 (R21-9): a fronted stack
            // standing in its own clause before an
            // intervening date ("just really truly,
            // November 1, moved to …") is judged by
            // the same rule — the clause directly
            // before the bound's own fragment is the
            // stack's clause, exactly as the
            // governor-window walk judges the clause
            // directly before its window's clause.
            // The fragment check above sees only the
            // date's own fragment and missed it.
            if pcs > 0 {
                let mut ppcs = 0usize;
                for (j, raw) in raw_tokens.iter().enumerate().take(pcs - 1) {
                    if clause_break_after(raw) {
                        ppcs = j + 1;
                    }
                }
                if let Some((count, all_listed)) = adverb_stack_clause(&tokens[ppcs..pcs]) {
                    if !all_listed || count + adverbs_used > 2 {
                        continue;
                    }
                }
            }
        }
        let zone_start = walk_start.max(bound);
        return Some(TargetClaim { zone_start, to_pos });
    }
    None
}

/// Whether the words in a target-claimed date's
/// governor window disqualify it (round 16, R16-1):
/// `governed_before`, except that a governor phrase
/// standing inside the claim zone — the target
/// phrase itself, its auxiliaries, its object — is
/// part of the target construction and does not
/// govern the target. A governor standing before
/// the zone ("not moved to …", "never moved to …")
/// still governs.
fn governed_before_target(window: &[String], window_start: usize, claim: &TargetClaim) -> bool {
    for i in 0..window.len() {
        for len in 1..=3usize {
            let Some(phrase) = window.get(i..i + len) else {
                break;
            };
            if !phrase_governs(window, i, len) {
                continue;
            }
            let abs_start = window_start + i;
            if abs_start >= claim.zone_start && abs_start <= claim.to_pos {
                continue;
            }
            // Round 15 (R15-1): the standing adjacency
            // skip, kept for phrases the claim zone
            // does not cover.
            if window.get(i + len).map(String::as_str) == Some("to")
                && is_displacement_phrase(phrase)
            {
                continue;
            }
            return true;
        }
    }
    false
}

/// Whether one word may stand in a bare restatement
/// of a date (round 19, R19-1): the date's framing
/// words (a weekday word, "on", "the"), the
/// date-referring filler the governor machinery
/// already knows ("date", "it", "that", "note",
/// "notes", "of"), the connectives and prepositions
/// that only join a date to its clause ("and", "or",
/// "but", "then", "for", "to", "from", "at", "by",
/// "in"), an auxiliary, or a tolerated adverb. Any
/// other word — a label, a subject, an object, an
/// event verb — is event content of the occurrence's
/// own. ("for" here is only a joining word for this
/// test; the for-composition's own fences in
/// R18-5/R18-9 are judged elsewhere and unchanged.)
///
/// Round 21 (R21-8): the inventory is completed by
/// its general rule — FUNCTION WORDS, words that
/// name no event, are bare; content labels are not.
/// The classes that join on that rule: conjunction
/// and discourse heads ("so", "well", "however"),
/// the adverb "only" standing as a region word
/// (an adverb outside the tolerated list is
/// content only when it modifies an event; "only"
/// before a date modifies nothing), the
/// preposition family ("until", "before", "after",
/// "through", "circa", "about", "around", "near"),
/// and the possessive pronouns ("mine", "ours",
/// "yours", "hers", "theirs" — the determiner
/// forms were already here). The label heads the
/// R20-3 adjudication named (reminder, update,
/// correction, attention, see, btw, fyi, and
/// "thanks" in head position) stay content: they
/// head an announcement the office wrote, and a
/// restatement wearing one asserts. "too" and
/// trailing "thanks" are bare in the trailing
/// slot only (see `occurrence_is_bare`).
///
/// Round 22 (R22-8): the rule is class-based —
/// function words are bare, content labels are
/// not. The classes that join: the remaining
/// prepositions ("with", "during", "since",
/// "within", "without", "upon", "unto", "into",
/// "over", "under", "between", "among", "against",
/// "per", "via", "onto", "toward", "towards",
/// "across", "behind", "beyond", "beside",
/// "despite", "amid", and "as" — so the two-word
/// units "as per" and "as of" read bare word by
/// word, "of" being already here); the
/// determiner / quantifier class ("each", "every",
/// "either", "neither", "both", "all", "any",
/// "some", "another", "other", "same", "such");
/// possessive + "own" ("his own November 1" — the
/// possessives were already here, "own" joins);
/// the conjunction / discourse tail ("lest",
/// "albeit", "unless", "whereas", "hence", "thus",
/// "moreover", "furthermore", "nonetheless",
/// "nevertheless", "instead", "meanwhile",
/// "otherwise", "therefore", "anyway", "rather",
/// "quite", "indeed"); and "too", which is bare
/// in BOTH slots now (head position included —
/// "(too November 1 too)" reads nothing).
/// Trailing "thanks" stays bare in the trailing
/// slot only, and "thanks" in head position stays
/// content, per the R20-3 adjudication.
///
/// Round 23 (R23-1/R23-2): the CLASSES are the
/// rule; the lists below are their current
/// membership, not a closed set. A newly found
/// function word is a list gap — graded MAJOR
/// when the word is ordinary office vocabulary,
/// as every word in this round's classes is. The
/// classes that join: the deictics ("yesterday",
/// "today", "tomorrow", "here", "there",
/// "together", "alone", and the preposition "re"
/// — temporal/locative deictic adverbs and their
/// kin, the same class as the tolerated "again" /
/// "now" / "then", naming no event); the lanes'
/// enumerated remainders of the classes R22-8
/// named — prepositions ("past", "off", "besides",
/// "plus", "minus", "concerning", "regarding",
/// "barring", "following", "including", "atop",
/// "amidst", "amongst", "thru", "versus", "vs"),
/// quantifier / determiners ("much", "many",
/// "few", "several", "enough", "most", "more",
/// "less", "least", "none", "little", "what",
/// "which", "whose", "whatever", "whichever",
/// "whoever"), connective adverbs ("accordingly",
/// "consequently", "subsequently", "next",
/// "later", "earlier", "afterwards", "afterward",
/// "beforehand", "simultaneously", "eventually",
/// "initially"), and adverbs ("once", "twice",
/// "always", "ever", "perhaps", "maybe",
/// "almost", "nearly", "pretty", "fairly",
/// "basically", "merely"). The lanes' ungraded
/// pronouns / modals / interjections do NOT join.
///
/// Round 24 (R24-1): the subordinator class joins
/// the inventory — "when", "whenever", "while",
/// "whilst", "although", "though", "because",
/// "if", "yet", "provided", "supposing",
/// "wherever", "where", "why", "how", "whether"
/// — the commonest members of the conjunction
/// class the inventory already held in its rare
/// members ("lest", "albeit", "whereas",
/// "unless"), in head and trailing slots alike;
/// and "than" joins the connective class in the
/// same batch (review M1: "(more than November 1)"
/// restates bare through it). The mirror held for
/// every word: a content label standing with one
/// ("(our choir November 1)") still asserts. The
/// ungraded pronoun / modal / interjection sets
/// the red team recorded-not-graded stay
/// untouched.
fn bare_filler_word(w: &str) -> bool {
    weekday_word(w).is_some()
        || AUXILIARY_WORDS.contains(&w)
        || POSSESSIVE_DETERMINERS.contains(&w)
        || is_tolerated_adverb(w)
        || matches!(
            w,
            "the"
                | "a"
                | "an"
                | "on"
                | "for"
                | "of"
                | "date"
                | "it"
                | "that"
                // Round 20 (R20-3): the demonstratives
                // join "that", which was already here —
                // a restatement wearing only a
                // demonstrative ("these November 1")
                // carries no event content, exactly as
                // "the November 1" does not.
                | "this"
                | "these"
                | "those"
                | "note"
                | "notes"
                | "and"
                | "or"
                | "but"
                | "then"
                | "to"
                | "from"
                | "at"
                | "by"
                | "in"
                // Round 20 (R20-3): the discourse
                // garnish list is CLOSED — please, yes,
                // again, ps, and no others. These words
                // frame a restatement ("please
                // November 1", "ps November 1") without
                // naming any event; a word not on this
                // list (a label like "reminder", any
                // other discourse word) is still event
                // content for this test.
                | "please"
                | "yes"
                | "again"
                | "ps"
                // Round 21 (R21-8): the function-word
                // classes the general rule above
                // names — conjunction / discourse
                // heads, the adverb "only", the
                // preposition family, and the
                // possessive pronouns.
                | "so"
                | "well"
                | "however"
                | "only"
                | "until"
                | "before"
                | "after"
                | "through"
                | "circa"
                | "about"
                | "around"
                | "near"
                | "mine"
                | "ours"
                | "yours"
                | "hers"
                | "theirs"
                // Round 22 (R22-8): the remaining
                // prepositions, the determiner /
                // quantifier class, possessive
                // "own", the conjunction / discourse
                // tail, and "too" in both slots —
                // the classes named in the doc
                // comment above.
                | "with"
                | "during"
                | "since"
                | "within"
                | "without"
                | "upon"
                | "unto"
                | "into"
                | "over"
                | "under"
                | "between"
                | "among"
                | "against"
                | "per"
                | "via"
                | "onto"
                | "toward"
                | "towards"
                | "across"
                | "behind"
                | "beyond"
                | "beside"
                | "despite"
                | "amid"
                | "as"
                | "each"
                | "every"
                | "either"
                | "neither"
                | "both"
                | "all"
                | "any"
                | "some"
                | "another"
                | "other"
                | "same"
                | "such"
                | "own"
                | "lest"
                | "albeit"
                | "unless"
                | "whereas"
                | "hence"
                | "thus"
                | "moreover"
                | "furthermore"
                | "nonetheless"
                | "nevertheless"
                | "instead"
                | "meanwhile"
                | "otherwise"
                | "therefore"
                | "anyway"
                | "rather"
                | "quite"
                | "indeed"
                | "too"
                // Round 23 (R23-1): the deictic class —
                // temporal and locative deictics and
                // the preposition "re" (see the doc
                // comment above).
                | "yesterday"
                | "today"
                | "tomorrow"
                | "here"
                | "there"
                | "together"
                | "alone"
                | "re"
                // Round 23 (R23-2): the preposition
                // remainder.
                | "past"
                | "off"
                | "besides"
                | "plus"
                | "minus"
                | "concerning"
                | "regarding"
                | "barring"
                | "following"
                | "including"
                | "atop"
                | "amidst"
                | "amongst"
                | "thru"
                | "versus"
                | "vs"
                // Round 23 (R23-2): the quantifier /
                // determiner remainder.
                | "much"
                | "many"
                | "few"
                | "several"
                | "enough"
                | "most"
                | "more"
                | "less"
                | "least"
                | "none"
                | "little"
                | "what"
                | "which"
                | "whose"
                | "whatever"
                | "whichever"
                | "whoever"
                // Round 23 (R23-2): the connective-
                // adverb remainder.
                | "accordingly"
                | "consequently"
                | "subsequently"
                | "next"
                | "later"
                | "earlier"
                | "afterwards"
                | "afterward"
                | "beforehand"
                | "simultaneously"
                | "eventually"
                | "initially"
                // Round 23 (R23-2): the adverb
                // remainder.
                | "once"
                | "twice"
                | "always"
                | "ever"
                | "perhaps"
                | "maybe"
                | "almost"
                | "nearly"
                | "pretty"
                | "fairly"
                | "basically"
                | "merely"
                // Round 24 (R24-1): the subordinator
                // class (see the doc comment above),
                // and "than" with the connectives.
                | "when"
                | "whenever"
                | "while"
                | "whilst"
                | "although"
                | "though"
                | "because"
                | "if"
                | "yet"
                | "provided"
                | "supposing"
                | "wherever"
                | "where"
                | "why"
                | "how"
                | "whether"
                | "than"
        )
}

/// Whether one side of an occurrence's region
/// carries a passive predicate construction
/// (round 20, R20-2): a cancellation/displacement
/// participle standing as an auxiliary's predicate
/// anywhere in the run — the `is_spent_predicate`
/// test, applied to every phrase in the run. A
/// lone auxiliary ("was November 1") and a lone
/// participle with no auxiliary ("November 1,
/// cancelled") are each no construction and do not
/// fire here.
fn region_has_passive_predicate(words: &[String]) -> bool {
    (0..words.len()).any(|i| {
        (1..=3usize).any(|len| words.get(i..i + len).is_some() && is_spent_predicate(words, i, len))
    })
}

/// Whether the AFTER side of an occurrence's region
/// carries a passive predicate of the occurrence's
/// OWN (round 21, R21-5): an occurrence's regions end
/// at the neighbouring occurrences, so a passive
/// "for" construction standing in the after-region
/// whose occasion is the NEXT occurrence — the run
/// is the next date's lead-in, "it was moved for"
/// and then the next date stands — credits that next
/// occurrence, not this one. Such a lead-in does not
/// make this occurrence non-bare: "November 1, 2026,
/// cancelled, November 1, 2026, it was moved for
/// November 8, 2026" — the restated November 1 stays
/// a bare restatement, bound by the cancellation,
/// and November 8 asserts by its own construction.
/// A predicate that completes inside the region (the
/// comma form, "it was moved, November 8") is the
/// occurrence's own, exactly as before.
fn after_region_has_own_passive_predicate(words: &[String], has_next: bool) -> bool {
    for i in 0..words.len() {
        for len in 1..=3usize {
            if words.get(i..i + len).is_none() || !is_spent_predicate(words, i, len) {
                continue;
            }
            if has_next {
                // The construction runs on into the
                // next occurrence: everything after
                // the participle phrase is joining
                // words ending in the "for" that
                // names the next date as its occasion.
                let suffix = &words[i + len..];
                let lead_in = !suffix.is_empty()
                    && suffix.last().map(String::as_str) == Some("for")
                    && suffix
                        .iter()
                        .all(|w| w.is_empty() || w.as_str() == "for" || bare_filler_word(w));
                if lead_in {
                    continue;
                }
            }
            return true;
        }
    }
    false
}

/// Round 24 (R24-2): the PHRASE units of the bare
/// test. A region is bare when its words partition
/// into inventory words and these listed phrase
/// units, matched as units — the closed list as
/// shipped is exactly: "care of", "in care of",
/// "care of the", "back together", "back together
/// again", "gone tomorrow", "gone by tomorrow",
/// "per se", "if and when", "when and if",
/// "as and when", "yet again", "together at last".
/// Their members are not individually function
/// words ("care" is a noun, "gone" a verb form,
/// "last" and "se" name nothing bare), so no word
/// list reaches them; a unit's member standing
/// alone keeps its current reading — "(care
/// November 1)" and "(gone November 1)" are not
/// bare, and assert as they did.
const BARE_PHRASE_UNITS: &[&[&str]] = &[
    &["care", "of"],
    &["in", "care", "of"],
    &["care", "of", "the"],
    &["back", "together"],
    &["back", "together", "again"],
    &["gone", "tomorrow"],
    &["gone", "by", "tomorrow"],
    &["per", "se"],
    &["if", "and", "when"],
    &["when", "and", "if"],
    &["as", "and", "when"],
    &["yet", "again"],
    &["together", "at", "last"],
];

/// The token span of a listed phrase unit standing
/// at `i` in `words`, if one does (round 24,
/// R24-2): the unit's words must stand consecutively
/// as a whole phrase (empty cleaned tokens — a
/// boundary token the shared cleaning reduced to
/// nothing — may stand between them, as they may
/// stand anywhere in a region). The longest unit
/// matching at `i` wins, so "care of the" is taken
/// as one unit, not as "care of" plus the article.
fn bare_phrase_unit_span(words: &[String], i: usize) -> Option<usize> {
    let mut best: Option<usize> = None;
    for unit in BARE_PHRASE_UNITS {
        let mut pos = i;
        let mut matched = true;
        for want in unit.iter() {
            while words.get(pos).is_some_and(|w| w.is_empty()) {
                pos += 1;
            }
            if words.get(pos).map(String::as_str) != Some(*want) {
                matched = false;
                break;
            }
            pos += 1;
        }
        if matched {
            let span = pos - i;
            if best.map_or(true, |b| span > b) {
                best = Some(span);
            }
        }
    }
    best
}

/// Whether the occurrence at `idx` in `positional`
/// is a BARE restatement of its date (round 19,
/// R19-1): the occurrence's own region of the line —
/// the words standing after the previous occurrence
/// and before it, and the words standing after it up
/// to the next occurrence (or the end) — carries,
/// besides the date span itself, nothing but framing
/// words, filler words, and governor phrases. An
/// occurrence heading its own event clause — a
/// label, subject, or object beyond the framing,
/// before the date ("early service November 1",
/// "choir November 1") or after it ("November 1,
/// choir") — is not bare, wherever it stands.
fn occurrence_is_bare(
    tokens: &[String],
    positional: &[(SimpleDate, usize, usize)],
    idx: usize,
) -> bool {
    let (_, start, used) = positional[idx];
    let region_start = if idx == 0 {
        0
    } else {
        positional[idx - 1].1 + positional[idx - 1].2
    };
    let region_end = positional
        .get(idx + 1)
        .map(|(_, s, _)| *s)
        .unwrap_or(tokens.len());
    // Round 20 (R20-2): a passive predicate
    // construction standing in the region — an
    // auxiliary with a governor participle as its
    // predicate, "it was moved", "it was
    // rescheduled", "it was cancelled" — is NOT
    // bare, however the region's words classify one
    // by one. Such a construction heads its own
    // event clause (round 19, R19-1's rule), and the
    // spent-predicate machinery already asserts its
    // date standing alone; the bare test must not
    // consume the auxiliary and the participle as
    // governor-phrase words and suppress that
    // clause's date under a sibling's governance.
    // The two sides of the date are judged
    // separately: a predicate cannot span the date
    // itself, so an auxiliary before the date and a
    // governor after it ("was November 1,
    // cancelled") form no construction here.
    let before_words = &tokens[region_start..start.min(tokens.len())];
    let after = (start + used).min(tokens.len());
    let after_words = &tokens[after..region_end.min(tokens.len())];
    // Round 21 (R21-5): the after side is judged by
    // the bounded rule — a passive "for" lead-in to
    // the next occurrence is that occurrence's
    // construction, not this one's (see
    // `after_region_has_own_passive_predicate`).
    let has_next = idx + 1 < positional.len();
    if region_has_passive_predicate(before_words)
        || after_region_has_own_passive_predicate(after_words, has_next)
    {
        return false;
    }
    let mut words: Vec<String> = Vec::new();
    words.extend_from_slice(before_words);
    // Round 21 (R21-8): the trailing discourse
    // word "thanks" is bare in the TRAILING slot
    // only — the words standing after the date.
    // In head position "thanks" is a content
    // label by the R20-3 adjudication and a
    // restatement wearing it asserts, as
    // "(thanks November 1)" always has. Round 22
    // (R22-8): "too" is bare in both slots (it
    // joins `bare_filler_word` above), so only
    // "thanks" still needs this trailing rule.
    let after_start = words.len();
    words.extend_from_slice(after_words);
    let mut i = 0;
    while i < words.len() {
        // Round 24 (R24-2): a listed phrase unit
        // standing at this word is consumed as a
        // unit, before any member is judged alone —
        // "per" is bare word by word but "se" is
        // not, and "together" / "at" are bare but
        // "last" is not, so a member-by-member pass
        // strands both units on their last word.
        if let Some(span) = bare_phrase_unit_span(&words, i) {
            i += span;
            continue;
        }
        if words[i].is_empty()
            || bare_filler_word(&words[i])
            || (i >= after_start && matches!(words[i].as_str(), "too" | "thanks"))
        {
            i += 1;
            continue;
        }
        // A governor phrase standing in the region —
        // the families `is_governor_phrase` knows,
        // longest first — is the governance itself,
        // not event content: "cancelled, November 1"
        // restates its date bare.
        let mut consumed = 0usize;
        for len in (1..=3usize).rev() {
            if let Some(phrase) = words.get(i..i + len) {
                if is_governor_phrase(phrase) {
                    consumed = len;
                    break;
                }
            }
        }
        if consumed > 0 {
            i += consumed;
            continue;
        }
        return false;
    }
    true
}

/// What one parenthesized group offers (round 11,
/// R11-4/R11-6/R11-8): at most one contributed date,
/// and — when the group offers two or more dates
/// instead — the offered dates as named alternatives,
/// which the account names on the unarmed second-date
/// note and which never stand as the group's date.
struct GroupAnalysis {
    contributed: Option<SimpleDate>,
    alternatives: Vec<SimpleDate>,
    // The date values this group governs directly,
    // in one of its own occurrences (round 19,
    // R19-2): the line-level pass over a line's
    // trailing groups reads these beside each
    // group's contribution, so a bare restatement
    // standing in a sibling group stays governed.
    governed_values: Vec<SimpleDate>,
    // Whether the contributed date stands only on
    // bare restatements of it (round 19, R19-1/R19-2):
    // every contributing occurrence is bare and
    // none is a target claim's last target. A
    // contribution headed by its own event clause,
    // or asserted by a claim, is never suppressed
    // by governance standing in another group.
    contributed_bare: bool,
}

fn analyze_group(inner: &str, today: SimpleDate, anchor: SimpleDate) -> GroupAnalysis {
    // Round 12 (R12-7b): the content reading sees
    // through balanced inner parentheses — the inner
    // content is the group's content for date-reading
    // purposes, so "((November 1))" and "(note
    // (November 1))" read the date a flat group
    // reads, and a nested governor ("(note (not
    // November 1))") still governs it.
    let unwrapped = inner.replace(['(', ')'], " ");
    if let Some(d) = parse_flexible_date(unwrapped.trim(), today) {
        // The group is the date and its framing
        // alone — a bare restatement by construction
        // (round 19, R19-2), governing nothing.
        let analysis = GroupAnalysis {
            contributed: Some(d),
            alternatives: Vec::new(),
            governed_values: Vec::new(),
            contributed_bare: true,
        };
        return analysis;
    }
    let raw_tokens: Vec<&str> = unwrapped.split_whitespace().collect();
    let tokens: Vec<String> = raw_tokens.iter().map(|t| group_token(t)).collect();
    // Round 18 (R18-6): the group's signals are the
    // plain line's signal set — framed dates and
    // relative phrases alike, in written order — so
    // "(November 1, cancelled, next Sunday)" judges
    // its relative replacement exactly as its plain
    // twin does, where the relative used to not be a
    // signal in a group at all and the group read
    // nothing. Anchor rule (round 19, R19-3): a
    // group resolves ALL its relative signals
    // against the parse anchor, relative-only groups
    // exactly as mixed ones — "(next Sunday)" in a
    // file dated October 16, scanned October 23,
    // contributes October 25, its mixed twin's
    // reading. Round 18's mixed-only rule mirrored
    // plain lines, but a relative-only PLAIN line
    // asserts no hard date while a relative-only
    // group contributes one, so the asymmetry
    // resolved a late file's group a week late. A
    // Sunday-after phrase over a framed base carries
    // no anchor resolution (`anchored` is None) and
    // keeps its framed-base reading, as before.
    let signals = dates_inside_group_signals(&raw_tokens, &tokens, today, anchor);
    let positional: Vec<(SimpleDate, usize, usize)> = signals
        .iter()
        .map(|s| (s.anchored.unwrap_or(s.date), s.start, s.used))
        .collect();
    let mut qualified: Vec<(SimpleDate, usize, usize)> = Vec::new();
    // The qualified occurrences' own indexes, in
    // written order (round 18, R18-5): the
    // for-composition below judges the qualified set
    // by occurrence, not only by date value.
    let mut qualified_idx: Vec<usize> = Vec::new();
    // Round 15 (R15-6): dates disqualified ONLY by
    // a trailing phrase standing after the group's
    // last date — governed, but the line did name
    // them, and beside a date that stands they are
    // named on the second-date note like any other
    // distinct date the line named, never silently
    // dropped: "(November 1, November 8, drop it)"
    // asserts November 1 and names November 8.
    // Round 16 (R16-5): the naming extends to a date
    // governed BETWEEN dates, whatever family
    // governed it — a phrase standing between it and
    // the next date, in either position, or the
    // target rule of R16-1: "(November 1, usher
    // shift, November 8)" names November 1 beside
    // November 8's assertion, instead of dropping it
    // unnamed. A group-initial before-governor
    // ("(not November 1, November 8)") keeps the
    // standing silence: its governor stands before
    // every date, not between dates.
    let mut trailing_named: Vec<SimpleDate> = Vec::new();
    // Round 16 (R16-1): the target claim standing
    // for each occurrence, computed once.
    let claims: Vec<Option<TargetClaim>> = (0..positional.len())
        .map(|idx| target_claim(&raw_tokens, &tokens, &positional, idx))
        .collect();
    let any_target = claims.iter().any(|c| c.is_some());
    // Round 20 (R20-7): the base dates this group's
    // own Sunday-after phrases consume, read with
    // the standing phrase-run scan — a consumed
    // base is part of its phrase and is never
    // named as another date the group offered, on
    // any naming channel (round 19, R19-8's
    // exclusion, which held only on the plain-core
    // merge before this).
    let consumed_bases: Vec<SimpleDate> = {
        let mut bases: Vec<SimpleDate> = Vec::new();
        let mut i = 0;
        while i < tokens.len() {
            if let Some((base, end)) = sunday_after_phrase_run(&raw_tokens, &tokens, i, today) {
                if !bases.contains(&base) {
                    bases.push(base);
                }
                i = end.max(i + 1);
                continue;
            }
            i += 1;
        }
        bases
    };
    // Round 18 (R18-2): the per-occurrence judgments
    // are collected first and applied in a second
    // pass, because governance attaches to the DATE
    // VALUE, not the occurrence (see below). Each
    // entry: (before, after, target, superseded).
    let mut judgments: Vec<(bool, bool, bool, bool)> = Vec::with_capacity(positional.len());
    // Round 18 (R18-5/R18-9): each occurrence's
    // "for" standing, judged from its own window —
    // whether the date is a for-occasion (a passive
    // or active "for" zone ends its window), and
    // whether it stands as a prospective "use" date
    // ("use November 8" is the window's last word
    // before the date). The for-composition below
    // reads these beside the qualified set.
    let mut for_occasions: Vec<bool> = Vec::with_capacity(positional.len());
    let mut use_prospectives: Vec<bool> = Vec::with_capacity(positional.len());
    for (idx, entry) in positional.iter().enumerate() {
        let (d, start, used) = *entry;
        // Round 12 (R12-2): the governor window for
        // this occurrence starts after the previous
        // occurrence's end — a governor is spent on
        // the one date it governs, and a later date
        // in the group is judged on its own window.
        let prev_end = if idx == 0 {
            0
        } else {
            positional[idx - 1].1 + positional[idx - 1].2
        };
        let window_start = governor_window_start(&raw_tokens, &tokens, prev_end, start, idx > 0);
        let window = &tokens[window_start..start.min(tokens.len())];
        for_occasions.push(
            passive_for_zone_start(&tokens[..window_start], window).is_some()
                || active_for_zone_start(window).is_some(),
        );
        use_prospectives.push(window.last().map(String::as_str) == Some("use"));
        let next_start = positional.get(idx + 1).map(|(_, s, _)| *s);
        // Round 14 (R14-1): the split phrasal form —
        // "put" standing last in the window and "off"
        // standing first after the date is the pair
        // "put off" wrapped around its date:
        // "(put November 1 off)" governs. Round 15
        // (R15-1): the same wrap for every phrasal
        // pair whose first word inflects — "(called
        // November 1 off)", "(put November 1
        // back)", "(push November 1 back)".
        let split_first = window.last().map(String::as_str);
        let split_second = tokens.get(start + used).map(String::as_str);
        let split_phrasal = match (split_first, split_second) {
            (Some(first), Some(second)) => is_phrasal_governor_pair(first, second),
            _ => false,
        };
        // Round 16 (R16-1): a target-claimed date is
        // judged by the claim — the construction's
        // own words do not govern it — and any OTHER
        // distinct date in a group whose target is
        // claimed is the phrase's from-date: governed
        // by the phrase and named, wherever it stands
        // ("(it was moved to November 8, November 1)").
        let claim = &claims[idx];
        let before_disqualified = match claim {
            Some(claim) => {
                governed_before_target(window, window_start, claim)
                    || split_phrasal
                    || contradicted_before(&tokens, start, d)
            }
            None => {
                governed_before(&tokens[..window_start], window)
                    || split_phrasal
                    || contradicted_before(&tokens, start, d)
            }
        };
        // Round 23 (R23-5): a framed date consumed
        // as a subject PP's object is that phrase's
        // own material — it does not qualify as an
        // occurrence of its own, exactly as the
        // "from" twin's date is governed in its
        // window and named beside the assertion.
        let before_disqualified = before_disqualified
            || date_in_subject_pp(&tokens, start, used)
            // Round 24 (R24-7): a relative phrase
            // consumed as a subject PP's object is
            // disqualified the same way — it does not
            // qualify as an occurrence of its own, and
            // the naming below names its resolution
            // beside the assertion exactly as it
            // names the framed twin's PP object.
            || relative_in_subject_pp(&tokens, start);
        let after_disqualified = governed_after(&raw_tokens, &tokens, start + used, next_start);
        let target_disqualified = claim.is_none()
            && any_target
            && claims
                .iter()
                .enumerate()
                .any(|(j, c)| c.is_some() && positional[j].0 != d);
        // Round 17 (R17-1): the LAST target in a
        // claim run wins. An earlier claimed target
        // superseded by a later distinct claimed
        // target is governed like the from-date —
        // "(November 1, moved to November 8, then to
        // November 15)" asserts November 15, with
        // November 1 and November 8 both named —
        // instead of the run collapsing to no
        // assertion at all.
        let superseded_target = claim.is_some()
            && claims
                .iter()
                .enumerate()
                .any(|(j, c)| j > idx && c.is_some() && positional[j].0 != d);
        judgments.push((
            before_disqualified,
            after_disqualified,
            target_disqualified,
            superseded_target,
        ));
    }
    // Round 18 (R18-2): governance by date value, the
    // group surface of the plain rule in
    // `plain_composition`. A date governed in ANY
    // occurrence (a governor before or after that
    // occurrence, the split wrap, a contradicting
    // weekday) does not assert from another
    // occurrence: "(November 1, cancelled, November
    // 1)" reads nothing, where the restated second
    // standing used to assert the cancelled date on
    // its own clean window. The one override is a
    // target claim naming the date as the LAST
    // target: "(meeting November 1, cancelled, moved
    // to November 1)" asserts November 1 by the
    // claim, the governed first standing named by
    // its own judgment. A by-date disqualification
    // standing after the group's first occurrence is
    // named like a between-dates governance; the
    // governed occurrence's own naming rules above
    // are unchanged.
    // Round 19 (R19-1): the by-date step binds only
    // a BARE restatement (see `occurrence_is_bare`).
    // An occurrence heading its own event clause
    // asserts or is governed by its own occurrence's
    // judgment, independently of the same value
    // elsewhere in the group: "(early service
    // November 1, late service cancelled November 1)"
    // asserts November 1 for the early service — the
    // governed occurrence is the late service's, and
    // the early service's occurrence carries its own
    // event content. Round 18's witnesses are all
    // bare restatements and stay silent.
    let governed_values: Vec<SimpleDate> = positional
        .iter()
        .zip(judgments.iter())
        .filter(|(_, (before, after, _, _))| *before || *after)
        .map(|(entry, _)| entry.0)
        .collect();
    for (idx, entry) in positional.iter().enumerate() {
        let (d, _, _) = *entry;
        let (before_disqualified, after_disqualified, target_disqualified, superseded_target) =
            judgments[idx];
        let is_last_target = claims[idx].is_some() && !superseded_target;
        // Round 20 (R20-10): a Sunday-after phrase
        // occurrence is never bound by value — an
        // ungoverned phrase salvages by its own
        // resolution whatever its base's form,
        // exactly as its framed-base twin does (the
        // framed twin's resolution differs from its
        // governed base, so it never needed the
        // exemption; a relative base's resolution
        // can coincide with the governed restated
        // value). A phrase governed in its own
        // occurrence still dies by that judgment
        // above, untouched.
        let is_sunday_after_phrase = tokens.get(positional[idx].1).map(String::as_str)
            == Some("sunday")
            && tokens.get(positional[idx].1 + 1).map(String::as_str) == Some("after");
        let by_date = !(before_disqualified || after_disqualified)
            && governed_values.contains(&d)
            && !is_last_target
            && !is_sunday_after_phrase
            && occurrence_is_bare(&tokens, &positional, idx);
        if before_disqualified
            || after_disqualified
            || target_disqualified
            || superseded_target
            || by_date
        {
            let next_start = positional.get(idx + 1).map(|(_, s, _)| *s);
            let named = target_disqualified
                || superseded_target
                || (after_disqualified && next_start.is_some())
                || (after_disqualified && !before_disqualified && next_start.is_none())
                || (before_disqualified && idx > 0 && next_start.is_some())
                || (by_date && idx > 0);
            if named && !trailing_named.contains(&d) {
                trailing_named.push(d);
            }
            continue;
        }
        qualified.push(*entry);
        qualified_idx.push(idx);
    }
    // Round 19 (R19-2): whether a date the group is
    // about to contribute stands only on bare
    // restatements of it — every qualified occurrence
    // of the value is bare, and none is a target
    // claim's last target (a claim asserts by its
    // own judgment and is never a restatement). The
    // line-level pass over trailing groups reads
    // this beside the sibling groups' governed
    // values.
    let contributed_bare = |d: SimpleDate| -> bool {
        let idxs: Vec<usize> = qualified_idx
            .iter()
            .copied()
            .filter(|i| positional[*i].0 == d)
            .collect();
        !idxs.is_empty()
            && idxs.iter().all(|i| {
                occurrence_is_bare(&tokens, &positional, *i)
                    && !(claims[*i].is_some() && !judgments[*i].3)
            })
    };
    let partial = partial_or_alternative(&tokens, &qualified);
    let mut distinct: Vec<SimpleDate> = Vec::new();
    for (d, _, _) in &qualified {
        if !distinct.contains(d) {
            distinct.push(*d);
        }
    }
    // Round 18 (R18-5): a for-date enters the group's
    // composition as a signal like any other, instead
    // of collapsing the group to alternatives the
    // moment a second date stands beside it. Two
    // readings, both fenced to groups no target claim
    // already orders (a "then to …" chain reads by
    // its claim, as it always has):
    // - A prospective second date asserts over the
    //   for-date: "(it was moved for November 1, use
    //   November 8)" asserts November 8, the
    //   for-date named — "use" is the office's own
    //   assertion word, and the for-date is the
    //   occasion it replaces.
    // - Otherwise the for-date reads by primacy:
    //   "(it was scheduled for November 1 and
    //   November 8)" asserts November 1 with
    //   November 8 noted, in written order either
    //   way — the for construction names its
    //   occasion first, and the standing two-date
    //   rule (neither asserts, both offered) yields
    //   to it, as it yields to a claim.
    if distinct.len() >= 2 && !any_target && !qualified_idx.is_empty() {
        let use_asserts = qualified_idx
            .iter()
            .rev()
            .copied()
            .find(|i| use_prospectives[*i])
            .filter(|ui| {
                qualified_idx
                    .iter()
                    .any(|fi| *fi < *ui && for_occasions[*fi])
            });
        let for_outcome = match use_asserts {
            Some(ui) => Some(positional[ui].0),
            None if for_occasions[qualified_idx[0]] => Some(distinct[0]),
            None => None,
        };
        if let Some(contributed) = for_outcome {
            let mut alternatives: Vec<SimpleDate> = Vec::new();
            for d in &distinct {
                if *d != contributed && !alternatives.contains(d) {
                    alternatives.push(*d);
                }
            }
            for d in &trailing_named {
                if *d != contributed && !consumed_bases.contains(d) && !alternatives.contains(d) {
                    alternatives.push(*d);
                }
            }
            if let Some(p) = partial {
                if p != contributed && !alternatives.contains(&p) {
                    alternatives.push(p);
                }
            }
            let bare = contributed_bare(contributed);
            return GroupAnalysis {
                contributed: Some(contributed),
                alternatives,
                governed_values: governed_values.clone(),
                contributed_bare: bare,
            };
        }
    }
    if distinct.len() == 1 && partial.is_none() {
        let mut alternatives: Vec<SimpleDate> = Vec::new();
        for d in &trailing_named {
            if *d != distinct[0] && !consumed_bases.contains(d) && !alternatives.contains(d) {
                alternatives.push(*d);
            }
        }
        let bare = contributed_bare(distinct[0]);
        let analysis = GroupAnalysis {
            contributed: Some(distinct[0]),
            alternatives,
            governed_values: governed_values.clone(),
            contributed_bare: bare,
        };
        return analysis;
    }
    let mut alternatives: Vec<SimpleDate> = Vec::new();
    for (d, _, _) in &qualified {
        if !alternatives.contains(d) {
            alternatives.push(*d);
        }
    }
    // A trailing-governed date is named beside the
    // offered alternatives too, when the group
    // offers more than one date and asserts none —
    // but a lone governed date (distinct is empty)
    // keeps the standing silence: "(November 1,
    // cancelled)" names nothing anywhere.
    if distinct.len() >= 2 {
        for d in &trailing_named {
            if !consumed_bases.contains(d) && !alternatives.contains(d) {
                alternatives.push(*d);
            }
        }
    }
    if let Some(p) = partial {
        if !alternatives.contains(&p) {
            alternatives.push(p);
        }
    }
    GroupAnalysis {
        contributed: None,
        alternatives,
        governed_values: governed_values.clone(),
        contributed_bare: false,
    }
}

/// The date one parenthesized group contributes
/// (round 10, R10-5): a group that is itself a bare
/// date contributes that date, as it always has; a
/// group with words around a date contributes the
/// date it contains when it contains exactly one —
/// "correction: November 1", "use November 1 instead"
/// — and a group containing no date, or several,
/// contributes nothing.
///
/// Round 11 (R11-4/R11-8): a contained date governed
/// by a negation or an origin marker standing before
/// it, or contradicted by a weekday word standing
/// directly before it, does not contribute — the
/// analysis in `analyze_group` is the one rule, and
/// every consumer of group contributions reads it
/// through here.
///
/// Round 12 (R12-1/R12-2/R12-7): the governor
/// families are completed (skip / except / rather
/// than / instead of / cancelled / bare no / the
/// isn't family / never mind; previously / formerly
/// / used to be / was / old date), a governor scopes
/// to the one date in its own clause, a trailing
/// governor with no later date to govern
/// disqualifies the date it follows, and the
/// content reading sees through inner parentheses.
///
/// Round 13 (R13-1/R13-2): the families are
/// completed structurally — negation is the
/// "n't" contraction class (plus wont / cannot),
/// cancellation gains cancel / scratch / drop /
/// postpone / remove / hold off, labels gain
/// "wrong:" — and a clause break erases only a
/// spent governor: an unspent governor standing
/// as its clause's trailing element still governs
/// the first date of the next clause.
///
/// Round 14 (R14-1/R14-2/R14-3/R14-4/R14-5,
/// RT14-4): the governor machinery is completed —
/// the verb set gains put off (split form
/// included), delay, omit, disregard, forget, and
/// the cancellation verbs match by stem; the
/// trailing position draws from the same family
/// as the before position; "nope" joins the
/// negation family; hyphenated compounds split
/// for the governor test only; a possessive-
/// preceded "wont" is the noun and governs
/// nothing; and the unspent-governor reach-back
/// crosses consecutive filler-only clauses.
fn group_date(inner: &str, today: SimpleDate, anchor: SimpleDate) -> Option<SimpleDate> {
    analyze_group(inner, today, anchor).contributed
}

/// A line's trailing parenthesized groups, in
/// written order, each with its analysis, under the
/// cross-group governance rule (round 19, R19-2):
/// governance by date value crosses a group boundary
/// exactly as it crosses a clause inside one group,
/// and binds exactly the same occurrences — a group
/// whose contribution stands only on BARE
/// restatements of a value that a SIBLING group on
/// the line governs directly contributes nothing
/// ("(November 1, cancelled) (November 1)" reads
/// nothing, in either group order, as the one-group
/// shapes do). A group heading its own event clause
/// asserts independently on either side of the
/// boundary ("(potluck November 1 cancelled)
/// (service November 1)" asserts November 1 for the
/// service — the potluck's cancellation governs the
/// potluck's occurrence, in its own group), and a
/// contribution a target claim asserts is never a
/// restatement (see `GroupAnalysis::contributed_bare`).
/// The line's plain core is not a sibling group
/// in THIS pass: a trailing group is judged on its
/// own beside the core here, and the core/group
/// composition (round 20, R20-1) applies the
/// core's governance to these analyses at its own
/// call sites (`governed_core_group_reading`,
/// `trailing_dates_after_core_governance`).
fn trailing_group_analyses(
    text: &str,
    today: SimpleDate,
    anchor: SimpleDate,
) -> Vec<(String, GroupAnalysis)> {
    let mut inners: Vec<String> = Vec::new();
    let mut rest = text;
    while let Some((before, inner)) = split_last_trailing_group(rest) {
        inners.push(inner.to_string());
        rest = before;
    }
    inners.reverse();
    let mut analyses: Vec<GroupAnalysis> = inners
        .iter()
        .map(|inner| analyze_group(inner, today, anchor))
        .collect();
    apply_cross_group_governance(&mut analyses);
    inners.into_iter().zip(analyses).collect()
}

/// The cross-group governance pass (round 19,
/// R19-2), shared by every group list a line is read
/// through: a group whose contribution stands only
/// on bare restatements of a value a SIBLING group
/// governs directly contributes nothing.
fn apply_cross_group_governance(analyses: &mut [GroupAnalysis]) {
    for i in 0..analyses.len() {
        let Some(d) = analyses[i].contributed else {
            continue;
        };
        if !analyses[i].contributed_bare {
            continue;
        }
        let governed_elsewhere = analyses
            .iter()
            .enumerate()
            .any(|(j, a)| j != i && a.governed_values.contains(&d));
        if governed_elsewhere {
            analyses[i].contributed = None;
        }
    }
}

/// Every parenthesized group standing in `text`, in
/// written order, each with its analysis, under the
/// same cross-group governance rule as the trailing
/// groups (round 21, R21-1/R21-3): the whole-line
/// composition reads a line's groups wherever they
/// stand — leading, trailing, or between plain
/// segments — so this is `trailing_group_analyses`
/// over the balanced spans instead of only the
/// trailing run.
fn all_group_analyses(
    text: &str,
    today: SimpleDate,
    anchor: SimpleDate,
) -> Vec<(String, GroupAnalysis)> {
    let inners: Vec<String> = balanced_spans(text)
        .iter()
        .map(|(start, end)| text[start + 1..end - 1].to_string())
        .collect();
    let mut analyses: Vec<GroupAnalysis> = inners
        .iter()
        .map(|inner| analyze_group(inner, today, anchor))
        .collect();
    apply_cross_group_governance(&mut analyses);
    inners.into_iter().zip(analyses).collect()
}

/// Every date a group standing anywhere in `text`
/// contributes, in written order: the whole-line
/// form of `trailing_parenthesized_dates`, for the
/// readers that remember a line's parenthesized
/// dates beside a relative phrase wherever the
/// groups stand (round 21, R21-1).
fn all_parenthesized_dates(text: &str, today: SimpleDate, anchor: SimpleDate) -> Vec<SimpleDate> {
    all_group_analyses(text, today, anchor)
        .iter()
        .filter_map(|(_, a)| a.contributed)
        .collect()
}

/// The last date a group standing anywhere in
/// `text` contributes, if one does: the whole-line
/// form of `trailing_parenthesized_date`.
fn all_parenthesized_date(text: &str, today: SimpleDate, anchor: SimpleDate) -> Option<SimpleDate> {
    all_group_analyses(text, today, anchor)
        .iter()
        .rev()
        .find_map(|(_, a)| a.contributed)
}

fn trailing_parenthesized_date(
    text: &str,
    today: SimpleDate,
    anchor: SimpleDate,
) -> Option<SimpleDate> {
    trailing_group_analyses(text, today, anchor)
        .iter()
        .rev()
        .find_map(|(_, a)| a.contributed)
}

/// Every date a trailing parenthesized group names,
/// in the order the groups stand in the line (round
/// 9, R9-10): groups are walked from the end like
/// `trailing_parenthesized_date`, a group that does
/// not read as a date is garnish and is skipped, and
/// the collected dates are put back in written order.
fn trailing_parenthesized_dates(
    text: &str,
    today: SimpleDate,
    anchor: SimpleDate,
) -> Vec<SimpleDate> {
    trailing_group_analyses(text, today, anchor)
        .iter()
        .filter_map(|(_, a)| a.contributed)
        .collect()
}

/// Every alternative a trailing parenthesized group
/// offers, in written order (round 11, R11-6): a
/// group that offers two or more dates contributes
/// none of them, and these are named on the line's
/// second-date note instead.
fn trailing_group_alternatives(
    text: &str,
    today: SimpleDate,
    anchor: SimpleDate,
) -> Vec<SimpleDate> {
    let mut found: Vec<SimpleDate> = Vec::new();
    for (_, a) in trailing_group_analyses(text, today, anchor) {
        for d in a.alternatives {
            if !found.contains(&d) {
                found.push(d);
            }
        }
    }
    found
}

/// Whether a group carries an actual target claim
/// for `read` (round 19, R19-9): either a target
/// claim judged inside the group itself — a target
/// phrase in the group claiming `read` through its
/// own "to" / "until", by the standing `target_claim`
/// test — or the chain's continuation link standing
/// in the group: a "to" / "until" before the `read`
/// occurrence with nothing between it and the date
/// but determiners and the link's own lead-ins
/// ("then", "and"), and no other date standing
/// between ("(then to November 15)", "(to November
/// 15)"). A bare "to" token anywhere in the group
/// is not a claim: "(November 15, note to self)" and
/// "(November 15, back to church)" wear their "to"
/// after the date, in garnish, and claim nothing.
fn group_has_target_claim(
    inner: &str,
    read: SimpleDate,
    today: SimpleDate,
    anchor: SimpleDate,
) -> bool {
    let unwrapped = inner.replace(['(', ')'], " ");
    let raw_tokens: Vec<&str> = unwrapped.split_whitespace().collect();
    let tokens: Vec<String> = raw_tokens.iter().map(|t| group_token(t)).collect();
    let signals = dates_inside_group_signals(&raw_tokens, &tokens, today, anchor);
    let positional: Vec<(SimpleDate, usize, usize)> = signals
        .iter()
        .map(|s| (s.anchored.unwrap_or(s.date), s.start, s.used))
        .collect();
    for (idx, entry) in positional.iter().enumerate() {
        if entry.0 != read {
            continue;
        }
        if target_claim(&raw_tokens, &tokens, &positional, idx).is_some() {
            return true;
        }
        let start = entry.1;
        for to_pos in 0..start {
            if !matches!(tokens[to_pos].as_str(), "to" | "until") {
                continue;
            }
            if positional.iter().any(|(_, s, _)| *s > to_pos && *s < start) {
                continue;
            }
            if !tokens[to_pos + 1..start]
                .iter()
                .all(|w| matches!(w.as_str(), "the" | "a" | "an" | "then" | "and" | ""))
            {
                continue;
            }
            // Round 20 (R20-8): the link's "to" must
            // belong to a target construction — a
            // displacement/postpone phrase in the
            // group governs the link (a target phrase
            // standing before the "to" with nothing
            // between it and the "to" but its object
            // words: no governor, cancellation,
            // auxiliary, or weekday), or the link
            // directly continues one (the fragment
            // before the "to" is nothing but the
            // link's own lead-ins: "(then to
            // November 15)", "(to November 15)").
            // A garnish fragment wearing its own
            // content word before the "to" — "(note
            // to November 15)", "(back to November
            // 15)", "(return to November 15)" — is
            // prose, not a chain link, even when the
            // group carries a claim elsewhere.
            let phrase_governs_link = (0..to_pos).any(|ps| {
                (1..=2usize).any(|len| {
                    let Some(phrase) = tokens.get(ps..ps + len) else {
                        return false;
                    };
                    if ps + len > to_pos || !is_target_phrase(phrase) {
                        return false;
                    }
                    tokens[ps + len..to_pos].iter().all(|w| {
                        !direct_single_governor(w)
                            && !is_cancellation_verb(w)
                            && !AUXILIARY_WORDS.contains(&w.as_str())
                            && weekday_word(w).is_none()
                    })
                })
            });
            let directly_continues = tokens[..to_pos]
                .iter()
                .all(|w| matches!(w.as_str(), "the" | "a" | "an" | "then" | "and" | ""));
            if phrase_governs_link || directly_continues {
                return true;
            }
        }
    }
    false
}

/// The superseded targets of a target chain split
/// across trailing groups (round 18, R18-11): when
/// two or more trailing groups contribute dates and
/// the groups between them carry a target
/// construction — a target phrase with its "to" /
/// "until" in one group, a target claim in the
/// last (round 19, R19-9) — the chain reads across
/// the group boundary exactly as it reads inside
/// one group,
/// and every earlier contributing group's date that
/// differs from `read` is a superseded target: it
/// is named on the line's note, and it is not a
/// disagreeing garnish date, so it leaves the
/// parenthesized-mismatch comparison. A stack of
/// bare date groups carries no target construction
/// and keeps the standing mismatch reading; only
/// the chain shape is named here.
fn chain_superseded_dates(
    text: &str,
    today: SimpleDate,
    anchor: SimpleDate,
    read: SimpleDate,
) -> Vec<SimpleDate> {
    // The chain's groups are the line's trailing
    // groups under the same cross-group pass as
    // every other reader (round 19, R19-2): a chain
    // link asserts by its claim, so the pass leaves
    // chain shapes exactly as they stood.
    let entries: Vec<(String, Option<SimpleDate>)> = trailing_group_analyses(text, today, anchor)
        .into_iter()
        .map(|(inner, a)| (inner, a.contributed))
        .collect();
    chain_superseded_from_entries(&entries, read, today, anchor)
}

/// The chain test over a list of (group words,
/// contributed date) entries in written order —
/// `chain_superseded_dates` over a line's trailing
/// groups, and the whole-line composition (round 21)
/// over a line's groups wherever they stand. The
/// shape judged is the one stated there: a target
/// phrase with its "to" / "until" in some
/// contributing group, and an actual target claim
/// for `read` in the last contributing group.
fn chain_superseded_from_entries(
    entries: &[(String, Option<SimpleDate>)],
    read: SimpleDate,
    today: SimpleDate,
    anchor: SimpleDate,
) -> Vec<SimpleDate> {
    let contributing: Vec<usize> = entries
        .iter()
        .enumerate()
        .filter(|(_, (_, d))| d.is_some())
        .map(|(i, _)| i)
        .collect();
    if contributing.len() < 2 {
        return Vec::new();
    }
    let last = *contributing.last().expect("two or more stand");
    if entries[last].1 != Some(read) {
        return Vec::new();
    }
    // The chain's shape, read across the groups: a
    // target phrase stands in some contributing
    // group with its "to" / "until" after it, and
    // the last contributing group carries an actual
    // target claim for the read date — the phrase
    // may stand in the first group while the last
    // group is only its continuation, "(moved to
    // November 8) (then to November 15)", which is
    // the witness shape. Round 19 (R19-9): the last
    // group's part used to ride on any "to" /
    // "until" token standing anywhere in it, so a
    // bare date group wearing garnish prose that
    // happened to contain "to" — "(November 15, note
    // to self)", "(November 15, back to church)" —
    // misclassified as a chain link and suppressed
    // a real garnish mismatch; the test now requires
    // the claim itself (see `group_has_target_claim`).
    let group_tokens: Vec<Vec<String>> = entries
        .iter()
        .map(|(inner, _)| inner.split_whitespace().map(group_token).collect())
        .collect();
    let phrase_with_to = |toks: &[String]| {
        (0..toks.len()).any(|ps| {
            (1..=2usize).any(|len| {
                let Some(phrase) = toks.get(ps..ps + len) else {
                    return false;
                };
                is_target_phrase(phrase)
                    && toks[ps + len..]
                        .iter()
                        .any(|w| w.as_str() == "to" || w.as_str() == "until")
            })
        })
    };
    let any_phrase = contributing
        .iter()
        .any(|i| phrase_with_to(&group_tokens[*i]));
    let last_claims = group_has_target_claim(&entries[last].0, read, today, anchor);
    if !any_phrase || !last_claims {
        return Vec::new();
    }
    let mut superseded: Vec<SimpleDate> = Vec::new();
    for i in &contributing[..contributing.len() - 1] {
        if let Some(d) = entries[*i].1 {
            if d != read && !superseded.contains(&d) {
                superseded.push(d);
            }
        }
    }
    superseded
}

/// `text` with every trailing parenthesized group
/// taken off, date groups and garnish alike: the core
/// a plain reading stands on once the groups have
/// been remembered separately.
fn strip_all_trailing_parenthesized(text: &str) -> &str {
    let mut rest = text;
    while let Some(stripped) = strip_trailing_parenthesized(rest) {
        rest = stripped;
    }
    rest
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
fn sunday_after_relative_phrase(raw: &Option<String>, today: SimpleDate) -> Option<String> {
    let text = raw.as_deref()?.trim();
    if !relative_shape_matches(text, sunday_after_relative_shape) {
        return None;
    }
    // Round 21 (R21-1), bounded by the R10-10c
    // boundary: a leading group that merely restates
    // the Sunday the phrase itself names stays
    // unread, as a group standing before the phrase
    // always has; a leading group naming a DIFFERENT
    // Sunday is a segment asserting another date, and
    // the phrase is remembered with it, the group
    // named on the parenthesized record.
    if text.trim_start().starts_with('(') {
        let resolution = relative_phrase_resolution(text, today, true);
        if !leading_group_disagrees(text, today, resolution) {
            return None;
        }
    }
    Some(text.trim_end_matches(['.', ',']).to_string())
}

/// The Sunday a relative phrase in `text` resolves
/// to at parse time (round 21, R21-1): the groups at
/// either end taken off, the remaining shape's
/// "next" / "this" base resolved against `today`,
/// and — for a Sunday-after phrase — the first
/// Sunday after that base.
fn relative_phrase_resolution(text: &str, today: SimpleDate, sunday_after: bool) -> SimpleDate {
    let mut rest = text;
    while let Some(stripped) =
        strip_trailing_parenthesized(rest).or_else(|| strip_leading_parenthesized(rest))
    {
        rest = stripped;
    }
    let next = rest
        .split_whitespace()
        .map(group_token)
        .any(|w| w == "next");
    let base = relative_sunday_date(next, today);
    if sunday_after {
        first_sunday_after(base)
    } else {
        base
    }
}

/// Whether a parenthesized group standing at the
/// very start of `text` contributes a date other
/// than `resolution` (round 21, R21-1): every leading
/// group is read in written order, and the first
/// disagreement answers. A group that contributes
/// nothing, or restates the resolution, does not
/// disagree.
fn leading_group_disagrees(text: &str, today: SimpleDate, resolution: SimpleDate) -> bool {
    let mut rest = text.trim_start();
    while rest.starts_with('(') {
        let Some((start, end)) = balanced_spans(rest).into_iter().next() else {
            return false;
        };
        if start != 0 {
            return false;
        }
        let inner = &rest[start + 1..end - 1];
        if let Some(d) = group_date(inner, today, today) {
            if d != resolution {
                return true;
            }
        }
        rest = rest[end..].trim_start();
    }
    false
}

/// Whether `text` is exactly the Sunday-after-a-
/// relative-Sunday shape, trailing weekday garnish
/// and all.
fn sunday_after_relative_shape(text: &str) -> bool {
    let words: Vec<String> = text
        .split_whitespace()
        // Round 23 (R23-3): the one shared cleaning
        // (`group_token`), as everywhere else — a
        // separator token standing alone (":" / "/"
        // / a dash where a group was taken off, or
        // wearing a word) cleans to an empty word or
        // a clean word, and is no word of the shape.
        .map(group_token)
        .filter(|w| !w.is_empty())
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
    names_shape
}

/// A For Sunday line that names its day only in
/// relative words (round 5, RT5-6): "next Sunday",
/// "this Sunday", "on Sunday", or a bare "Sunday".
/// Returns the line as written (trimmed, closing
/// punctuation off) when that is ALL the line says, so
/// the review can repeat the sender's own words; a
/// line saying anything more is not this shape.
fn relative_sunday_phrase(raw: &Option<String>, today: SimpleDate) -> Option<String> {
    let text = raw.as_deref()?.trim();
    if !relative_shape_matches(text, relative_sunday_shape) {
        return None;
    }
    // Round 21 (R21-1), bounded by the R10-10c
    // boundary, as in `sunday_after_relative_phrase`:
    // a leading group restating the phrase's own
    // Sunday stays unread; a leading group naming a
    // different Sunday is remembered with the phrase.
    if text.trim_start().starts_with('(') {
        let resolution = relative_phrase_resolution(text, today, false);
        if !leading_group_disagrees(text, today, resolution) {
            return None;
        }
    }
    Some(text.trim_end_matches(['.', ',']).to_string())
}

/// Whether `text` is exactly a bare relative Sunday,
/// trailing weekday garnish and all.
fn relative_sunday_shape(text: &str) -> bool {
    let words: Vec<String> = text
        .split_whitespace()
        // Round 23 (R23-3): the one shared cleaning
        // (`group_token`), as in
        // `sunday_after_relative_shape` — a separator
        // token standing alone cleans to an empty word
        // and is no word of the shape.
        .map(group_token)
        .filter(|w| !w.is_empty())
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
    names_sunday
}

/// Read a date standing at the head of `tokens`,
/// letting the line's remainder follow it (round 8,
/// R8-4): the longest opening run of words that reads
/// as a date, and how many words it took. Words after
/// the date are the remainder, not a parse failure.
fn parse_date_prefix(tokens: &[&str], today: SimpleDate) -> Option<(SimpleDate, usize)> {
    // Round 23 (R23-3): a date run may not START on a
    // boundary token — a token the shared cleaning
    // (`group_token`) reduces to nothing is boundary
    // punctuation standing alone (a comma left by a
    // closing paren, a colon, a slash, a dash), never
    // the first word of a date. The flexible reader
    // strips commas globally, so ", November 8" used
    // to read as a date run STARTING on the comma:
    // the boundary token was absorbed into the date's
    // own span, the governor before it stood inside
    // the span's window, and a nested group closing
    // between a governor and its date ("((November 1,
    // cancelled), November 8)") silenced the target
    // its comma twin asserts. A boundary is a boundary
    // wherever it stands; the scan steps over it and
    // starts the run on the date's own first word.
    if tokens.first().is_some_and(|t| group_token(t).is_empty()) {
        return None;
    }
    // Round 22 (R22-2): a sender's semicolon (or colon)
    // attached to a date token — "2026;" — is a clause
    // break standing after the date, never part of the
    // date itself: the token is read without it, exactly
    // as `group_token` cleans it for the governor scans.
    // Without this, the year fell outside the date's own
    // span, the stray year token broke the governor
    // scope behind it, and a cancelled date restated
    // after its semicolon resurrected — a sender's
    // semicolon must be indistinguishable from the
    // composition's internal group marker, which the
    // date readers never absorb into a date run.
    let normalized: Vec<String> = tokens
        .iter()
        .map(|t| {
            // A token of nothing but the punctuation
            // (a standalone ";" — the composition's own
            // group marker) keeps its shape: it is a
            // segment boundary no date run may start
            // on or cross, never an empty word.
            // Round 23 (R23-3): the same boundary
            // punctuation ATTACHED at a token's head —
            // a separator written with no space after
            // a group, "(please):November 1, 2026,
            // cancelled", or the composition's own ";"
            // marker glued to the core's first word —
            // is boundary, not part of the word: the
            // token is read without it, exactly as at
            // its tail.
            // Round 24 (R24-4): a bracket at a date
            // token's edge is boundary of the same
            // kind in a date span — "[November 8,
            // 2026]" reads its date on the date's own
            // merits (governance, framing, the fence)
            // instead of the bracket poisoning the
            // parse. Brackets are still not group
            // delimiters: a lone "[November 8]" gains
            // no group reading and stays unread, the
            // recorded limitation.
            let trimmed = t
                .trim_start_matches([';', ':', '[', '{'])
                .trim_end_matches([';', ':', ']', '}']);
            if trimmed.is_empty() {
                t.to_string()
            } else {
                trimmed.to_string()
            }
        })
        .collect();
    let refs: Vec<&str> = normalized.iter().map(String::as_str).collect();
    (1..=refs.len())
        .rev()
        .find_map(|end| parse_flexible_date(&refs[..end].join(" "), today).map(|d| (d, end)))
}

/// Whether the weekday at `i` in `core` wears the
/// framing garnish stack (round 8, R8-6): an optional
/// "on" and an optional "the" in front of it — the
/// stack the Sunday-after reader strips — or stands
/// at the line's start bare, as the leading form
/// always has. A weekday wearing neither, away from
/// the line's start, frames nothing and stays
/// ordinary text (round 5, R5-D).
fn weekday_is_framed(core: &[&str], i: usize) -> bool {
    fn clean(t: &str) -> &str {
        t.trim_matches(['(', ')', '[', ']', ',', '.'])
    }
    if i == 0 {
        return true;
    }
    if clean(core[i - 1]).eq_ignore_ascii_case("on") {
        return true;
    }
    if clean(core[i - 1]).eq_ignore_ascii_case("the") {
        // "on the Monday", or the line's own opening
        // "the" standing in front of the weekday.
        return i == 1 || clean(core[i - 2]).eq_ignore_ascii_case("on");
    }
    false
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
///
/// Round 8 (R8-4/R8-5/R8-6): the framing needs the
/// weekday directly in front of the date — what
/// follows the date is the line's remainder, not a
/// parse failure — the reader scans EVERY framing
/// candidate in the line until one frames a date (a
/// candidate whose following words do not parse as a
/// date is not a framing, and scanning continues),
/// and the weekday may wear the garnish stack, an
/// optional "on" and an optional "the". The trailing
/// weekday frames the same date only when it stands
/// directly after it; words between the date and a
/// trailing weekday word make that word part of the
/// remainder, not a second framing.
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
    for i in 0..core.len() {
        let Some(lead_number) = weekday_word(clean(core[i])) else {
            continue;
        };
        if !weekday_is_framed(&core, i) {
            continue;
        }
        let Some((date, used)) = parse_date_prefix(&core[i + 1..], today) else {
            continue;
        };
        if date.weekday() != lead_number {
            let named = clean(core[i]).to_string();
            return Some((None, Some(WeekdayMismatch { named, date })));
        }
        if i + 1 + used == core.len() {
            if let Some((trailing_word, trailing_number)) = trailing {
                if date.weekday() != trailing_number {
                    let named = trailing_word.to_string();
                    return Some((None, Some(WeekdayMismatch { named, date })));
                }
            }
        }
        return Some((Some(date), None));
    }
    None
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
fn date_line_inner(
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
    // by `named_weekday_mismatch`.) Defense in depth
    // (round 8, R8-5): a weekday wearing the framing
    // garnish stack was the framing check's to judge,
    // so this fallback never returns the date it
    // stands in front of clean.
    for i in 1..tokens.len() {
        let word = tokens[i].trim_matches(['(', ')', '[', ']', ',', '.']);
        if weekday_word(word).is_none() {
            continue;
        }
        if weekday_is_framed(&tokens, i) {
            continue;
        }
        if let Some(date) = parse_flexible_date(&tokens[i + 1..].join(" "), today) {
            return (Some(date), None);
        }
    }
    (None, None)
}

/// What one date line reads, all records together
/// (round 9, R9-C): the date, the weekday mistake when
/// one is named, the parenthesized garnish's
/// disagreement when one stands, and a second absolute
/// date named later in the line when one stands.
struct DateLineReading {
    date: Option<SimpleDate>,
    weekday_mismatch: Option<WeekdayMismatch>,
    parenthesized_mismatch: Option<ParenthesizedDateMismatch>,
    second_dates: Vec<SimpleDate>,
}

/// The parenthesized groups standing in `text`, as
/// (start, end) byte spans, end past the closing
/// parenthesis: flat groups only, in written order.
fn parenthesized_spans(text: &str) -> Vec<(usize, usize)> {
    let mut out: Vec<(usize, usize)> = Vec::new();
    let mut from = 0;
    while let Some(rel_open) = text[from..].find('(') {
        let open = from + rel_open;
        let Some(rel_close) = text[open + 1..].find(')') else {
            break;
        };
        let close = open + 1 + rel_close;
        let inner = &text[open + 1..close];
        if !inner.contains('(') {
            out.push((open, close + 1));
        }
        from = close + 1;
    }
    out
}

/// The top-level parenthesized spans of `text`, as
/// (start, end) byte spans, end past the closing
/// parenthesis, in written order (round 21, R21-1):
/// found by balancing, so a nested group is one span
/// ("(note (November 1))"), and a group standing
/// anywhere in the line — leading, between plain
/// segments, trailing — is seen. An unbalanced
/// opening parenthesis forms no span.
fn balanced_spans(text: &str) -> Vec<(usize, usize)> {
    let mut out: Vec<(usize, usize)> = Vec::new();
    let bytes = text.as_bytes();
    let mut depth = 0usize;
    let mut open = 0usize;
    for (i, b) in bytes.iter().enumerate() {
        match b {
            b'(' => {
                if depth == 0 {
                    open = i;
                }
                depth += 1;
            }
            b')' if depth == 1 => {
                depth = 0;
                out.push((open, i + 1));
            }
            b')' => {
                depth = depth.saturating_sub(1);
            }
            _ => {}
        }
    }
    out
}

/// `text` without its leading "(...)" group, when it
/// begins with one (round 21, R21-1): the words after
/// the group. Found by balancing, like
/// `split_last_trailing_group` at the other end.
fn strip_leading_parenthesized(text: &str) -> Option<&str> {
    let text = text.trim_start();
    if !text.starts_with('(') {
        return None;
    }
    let (start, end) = balanced_spans(text).into_iter().next()?;
    if start != 0 {
        return None;
    }
    Some(text[end..].trim_start())
}

/// A parenthesized group standing between a month-day
/// and its year does not poison the plain date (round
/// 10, R10-10b): "October 25 (November 1), 2026" reads
/// October 25, 2026, with the November 1 group
/// compared as garnish. The shape is pinned by what
/// follows the group — a comma and a year — and by the
/// line reading as a date once the group is taken out;
/// a group anywhere else is not tried here.
fn mid_date_group_reading(
    text: &str,
    today: SimpleDate,
    anchor: SimpleDate,
    trailing_groups: &[SimpleDate],
    trailing_alternatives: &[SimpleDate],
) -> Option<DateLineReading> {
    for (start, end) in parenthesized_spans(text) {
        let after = text[end..].trim_start();
        let Some(after_comma) = after.strip_prefix(',') else {
            continue;
        };
        let Some(year_token) = after_comma.split_whitespace().next() else {
            continue;
        };
        let year_clean = year_token.trim_matches(['.', ',']);
        if !(year_clean.len() == 2 || year_clean.len() == 4)
            || !year_clean.chars().all(|c| c.is_ascii_digit())
        {
            continue;
        }
        let mut candidate = String::with_capacity(text.len());
        candidate.push_str(&text[..start]);
        candidate.push_str(&text[end..]);
        // Round 11 (R11-6/R11-7 review shape): a
        // trailing group standing after the year is
        // garnish the composition already holds; the
        // de-grouped candidate is read on its core,
        // trailing groups off, so a mid-date group and
        // a trailing group together no longer defeat
        // each other and read as nothing.
        let candidate_core = strip_all_trailing_parenthesized(candidate.trim()).to_string();
        let candidate_raw = Some(candidate_core.clone());
        let (cand_date, _) = date_line_inner(&candidate_raw, today);
        let Some(read) = cand_date else {
            continue;
        };
        let inner = &text[start + 1..end - 1];
        let mut all_groups: Vec<SimpleDate> = Vec::new();
        if let Some(d) = group_date(inner, today, anchor) {
            all_groups.push(d);
        }
        for g in trailing_groups {
            if !all_groups.contains(g) {
                all_groups.push(*g);
            }
        }
        let parenthesized_mismatch = mismatch_from_groups(&all_groups, read, false);
        let mut second_dates = second_dates_in_line(&candidate_core, today, read);
        for d in trailing_alternatives {
            if *d != read && !second_dates.contains(d) {
                second_dates.push(*d);
            }
        }
        return Some(DateLineReading {
            date: Some(read),
            weekday_mismatch: None,
            parenthesized_mismatch,
            second_dates,
        });
    }
    None
}

/// Every distinct second parseable absolute date
/// standing in a date line (round 9, R9-13; round 10,
/// R10-4/R10-6): the scan reads the WHOLE line, before
/// the read date as well as after it. The first
/// date-shaped run in the line that is the read date
/// itself — or, on a Sunday-after line, the base the
/// read date resolves from — is the anchor and is
/// never a second date; every other distinct date the
/// line names, on either side of the anchor, is a
/// second date, in written order. Trailing
/// parenthesized groups are garnish, remembered on
/// their own records, and are not scanned here.
///
/// Round 11 (R11-7): the anchor is the read date's
/// OWN occurrence — the first run that is the read
/// date itself. A distinct date in the six days
/// before the read date shares its Sunday but is not
/// its proxy: it is a second date. On a Sunday-after
/// line the one other anchor is the resolution's
/// base — the date run the "Sunday after" phrase
/// itself consumes — and the base is never flagged
/// as its own second date.
///
/// Round 12 (R12-9a): the phrase consumes its run
/// wherever it stands in the line, not only where
/// the line's own reading found it. A phrase run's
/// value is its RESOLUTION: a second phrase whose
/// resolution differs from the read date is noted
/// with that resolution, exactly as the first
/// phrase's resolution is the read date itself. And
/// a plain run equal to the standing base — the base
/// the read date resolves from — is a restatement of
/// the base, never "another date", wherever in the
/// line it stands.
fn second_dates_in_line(text: &str, today: SimpleDate, read: SimpleDate) -> Vec<SimpleDate> {
    // Round 18 (R18-1): the second-date scan cleans
    // with the one shared cleaning too (`group_token`),
    // so a "Sunday after" run wearing a colon or a
    // dash is the same run here as in a group.
    let core = strip_all_trailing_parenthesized(text.trim());
    let tokens: Vec<&str> = core.split_whitespace().collect();
    let cleaned: Vec<String> = tokens.iter().map(|t| group_token(t)).collect();
    // Every run in the line, in written order: a
    // plain date run (base None), or a Sunday-after
    // phrase run whose date is its resolution and
    // whose base it consumed.
    struct LineRun {
        date: SimpleDate,
        base: Option<SimpleDate>,
    }
    let mut runs: Vec<LineRun> = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        if let Some((base, end)) = sunday_after_phrase_run(&tokens, &cleaned, i, today) {
            runs.push(LineRun {
                date: first_sunday_after(base),
                base: Some(base),
            });
            i = end.max(i + 1);
            continue;
        }
        match parse_date_prefix(&tokens[i..], today) {
            Some((d, used)) => {
                runs.push(LineRun {
                    date: d,
                    base: None,
                });
                i += used.max(1);
            }
            None => i += 1,
        }
    }
    // The standing base: the base of the first
    // phrase whose resolution is the read date.
    let standing_base = runs
        .iter()
        .find(|r| r.base.is_some() && r.date == read)
        .and_then(|r| r.base);
    let mut own_anchored = false;
    let mut out: Vec<SimpleDate> = Vec::new();
    for run in runs {
        if run.base.is_some() {
            // A phrase resolving to the read date is
            // the anchor (or a restatement of it);
            // any other phrase is noted by its
            // resolution, its base consumed.
            if run.date != read && !out.contains(&run.date) {
                out.push(run.date);
            }
            continue;
        }
        if Some(run.date) == standing_base {
            continue;
        }
        if !own_anchored && run.date == read {
            own_anchored = true;
            continue;
        }
        if run.date != read && !out.contains(&run.date) {
            out.push(run.date);
        }
    }
    out
}

/// The Sunday-after phrase standing at token `i` of
/// a line, if one does (round 12, R12-9a): "Sunday
/// after" followed by a readable base date, the base
/// wearing its own leading weekday (behind an
/// optional "on") as the line reader allows.
/// Returns the base and the token index just past
/// the phrase.
fn sunday_after_phrase_run(
    tokens: &[&str],
    cleaned: &[String],
    i: usize,
    today: SimpleDate,
) -> Option<(SimpleDate, usize)> {
    if weekday_word(&cleaned[i]) != Some(0)
        || cleaned.get(i + 1).map(String::as_str) != Some("after")
    {
        return None;
    }
    // Round 18 (R18-1): the one shared cleaning.
    fn clean(t: &str) -> String {
        group_token(t)
    }
    let rest = &tokens[i + 2..];
    if let Some((base, used)) = parse_date_prefix(rest, today) {
        return Some((base, i + 2 + used));
    }
    // The base's own leading weekday, wearing an
    // optional "on": strip it and read what it framed.
    let offset = if !rest.is_empty() && weekday_word(&clean(rest[0])).is_some() {
        1
    } else if rest.len() > 1
        && clean(rest[0]).eq_ignore_ascii_case("on")
        && weekday_word(&clean(rest[1])).is_some()
    {
        2
    } else {
        return None;
    };
    let (base, used) = parse_date_prefix(&rest[offset..], today)?;
    Some((base, i + 2 + offset + used))
}

/// The Sunday a bare relative phrase names,
/// resolved against `today` — the only anchor a
/// plain line carries at parse time (round 13,
/// R13-5): "this Sunday" and a bare "Sunday" name
/// the coming Sunday (today itself when today is
/// one), and "next Sunday" names the Sunday after
/// it — the reading under which "this" and "next"
/// name different days, as the sender's words do.
fn relative_sunday_date(next: bool, today: SimpleDate) -> SimpleDate {
    let until_sunday = (7 - today.weekday()) % 7;
    let coming = add_days(today, until_sunday);
    if next {
        add_days(coming, 7)
    } else {
        coming
    }
}

/// The governor composition on a plain line's
/// signals (round 16, R16-4; round 17, R17-2: framed
/// and relative signals alike, each judged by the
/// words around its own token span): each signal is
/// judged exactly as a date occurrence inside a
/// group — its governor window (with the round-16
/// run spending and target claim), its trailing
/// governors, the split phrasal wrap, a contradicting
/// weekday — so the words between two dates on a
/// plain header line compose the way the same words
/// compose inside parentheses. Returns None when no
/// signal is governed (the standing first-signal
/// primacy then reads the line). When some signal
/// is governed, the first UNGOVERNED signal reads
/// and every other distinct date is named on the
/// second-date note; when every signal is governed
/// the line reads no date, the group's silence.
fn plain_composition(
    raw_tokens: &[&str],
    tokens: &[String],
    signals: &[(SimpleDate, usize, usize)],
    alternatives: &[SimpleDate],
) -> Option<DateLineReading> {
    let positional: Vec<(SimpleDate, usize, usize)> = signals
        .iter()
        .map(|(d, start, end)| (*d, *start, end - start))
        .collect();
    let claims: Vec<Option<TargetClaim>> = (0..positional.len())
        .map(|idx| target_claim(raw_tokens, tokens, &positional, idx))
        .collect();
    let any_target = claims.iter().any(|c| c.is_some());
    // Round 18 (R18-2): the per-occurrence judgments
    // are kept apart — direct governance (a governor
    // before or after the occurrence, the split
    // phrasal wrap, a contradicting weekday) versus
    // the claim machinery — because governance now
    // attaches to the DATE VALUE, not the occurrence
    // (see the by-date pass after the loop).
    let mut direct_governed: Vec<bool> = Vec::new();
    let mut claim_governed: Vec<bool> = Vec::new();
    let mut last_target: Vec<bool> = Vec::new();
    // Round 18 (R18-5): each signal's "for" standing,
    // as in `analyze_group` — a for-occasion window
    // and a prospective "use" window.
    let mut for_occasions: Vec<bool> = Vec::new();
    let mut use_prospectives: Vec<bool> = Vec::new();
    for (idx, entry) in positional.iter().enumerate() {
        let (d, start, used) = *entry;
        let prev_end = if idx == 0 {
            0
        } else {
            positional[idx - 1].1 + positional[idx - 1].2
        };
        let window_start = governor_window_start(raw_tokens, tokens, prev_end, start, idx > 0);
        let window = &tokens[window_start..start.min(tokens.len())];
        for_occasions.push(
            passive_for_zone_start(&tokens[..window_start], window).is_some()
                || active_for_zone_start(window).is_some(),
        );
        use_prospectives.push(window.last().map(String::as_str) == Some("use"));
        let next_start = positional.get(idx + 1).map(|(_, s, _)| *s);
        let split_first = window.last().map(String::as_str);
        let split_second = tokens.get(start + used).map(String::as_str);
        let split_phrasal = match (split_first, split_second) {
            (Some(first), Some(second)) => is_phrasal_governor_pair(first, second),
            _ => false,
        };
        let claim = &claims[idx];
        let before = match claim {
            Some(claim) => governed_before_target(window, window_start, claim),
            None => governed_before(&tokens[..window_start], window),
        };
        // Round 23 (R23-5): a framed date consumed
        // as a subject PP's object is governed for
        // this composition in the same way as in a
        // group — it asserts nothing of its own and
        // is named beside the date that stands.
        let before_disqualified = before
            || split_phrasal
            || contradicted_before(tokens, start, d)
            || date_in_subject_pp(tokens, start, used)
            // Round 24 (R24-7): the relative-phrase
            // twin — a relative phrase consumed as a
            // subject PP's object asserts nothing of
            // its own on the plain surface either, so
            // "the choir on next Sunday sang November
            // 8, 2026" reads November 8 with October
            // 25 named, not the other way round.
            || relative_in_subject_pp(tokens, start);
        let after_disqualified = governed_after(raw_tokens, tokens, start + used, next_start);
        let target_disqualified = claim.is_none()
            && any_target
            && claims
                .iter()
                .enumerate()
                .any(|(j, c)| c.is_some() && positional[j].0 != d);
        // Round 17 (R17-1): the last target wins on
        // the plain surface too — an earlier claimed
        // target superseded by a later distinct one
        // yields and is named, so the plain twin of
        // the collapsed group no longer asserts the
        // superseded date.
        let superseded_target = claim.is_some()
            && claims
                .iter()
                .enumerate()
                .any(|(j, c)| j > idx && c.is_some() && positional[j].0 != d);
        direct_governed.push(before_disqualified || after_disqualified);
        claim_governed.push(target_disqualified || superseded_target);
        last_target.push(claim.is_some() && !superseded_target);
    }
    // Round 18 (R18-2): governance by date value. A
    // date governed in ANY occurrence does not assert
    // from another occurrence — a cancelled date
    // restated in either form, on either side of its
    // governor, stays governed ("November 1, 2026,
    // cancelled, November 1, 2026" and, at a today
    // where "next Sunday" IS November 1, "November 1,
    // 2026, cancelled, next Sunday" both read
    // nothing). The one override: an occurrence a
    // target claim names as the LAST target asserts
    // by the claim, as a claimed target always has.
    // Restated UNGOVERNED signals are untouched —
    // with no directly governed occurrence the set
    // below is empty, this pass changes nothing, and
    // the R13-C first-signal reading stands.
    // Round 19 (R19-1): the by-date step binds only
    // a BARE restatement, exactly as in groups (see
    // `occurrence_is_bare`) — an occurrence heading
    // its own event clause on a plain line ("early
    // service November 1, 2026, late service
    // cancelled November 1, 2026") asserts by its
    // own judgment, and the plain twins of the
    // round-18 bare witnesses stay silent.
    let governed_values: Vec<SimpleDate> = positional
        .iter()
        .zip(direct_governed.iter())
        .filter(|(_, g)| **g)
        .map(|(entry, _)| entry.0)
        .collect();
    let mut governed: Vec<bool> = Vec::with_capacity(positional.len());
    for (idx, entry) in positional.iter().enumerate() {
        // Round 20 (R20-10): a Sunday-after phrase
        // signal is never bound by value, as in
        // groups (see `analyze_group`) — an
        // ungoverned phrase salvages by its own
        // resolution whatever its base's form; a
        // phrase governed in its own span still
        // dies by that judgment.
        let is_sunday_after_phrase = tokens.get(entry.1).map(String::as_str) == Some("sunday")
            && tokens.get(entry.1 + 1).map(String::as_str) == Some("after");
        let by_date = !direct_governed[idx]
            && governed_values.contains(&entry.0)
            && !last_target[idx]
            && !is_sunday_after_phrase
            && occurrence_is_bare(tokens, &positional, idx);
        governed.push(direct_governed[idx] || claim_governed[idx] || by_date);
    }
    // Round 18 (R18-5): the prospective half of the
    // for-composition, on the plain surface too — a
    // "use" date standing after a for-occasion
    // asserts, with the for-date named: "it was
    // moved for November 1, 2026, use November 8,
    // 2026" asserts November 8. (The primacy half
    // needs no code here: an ungoverned plain line
    // already reads its first signal, and the
    // for-date stands first in its witness.) The
    // check runs before the no-governance return
    // below, because neither the for-date nor the
    // use date is governed — that is the shape's
    // whole point. A target claim anywhere on the
    // line fences it out, as in groups.
    if !any_target {
        let use_asserts = (0..positional.len()).rev().find(|i| {
            use_prospectives[*i]
                && !governed[*i]
                && (0..*i).any(|fi| for_occasions[fi] && !governed[fi])
        });
        if let Some(ui) = use_asserts {
            let read = positional[ui].0;
            let mut second_dates: Vec<SimpleDate> = Vec::new();
            for (d, _, _) in &positional {
                if *d != read && !second_dates.contains(d) {
                    second_dates.push(*d);
                }
            }
            for d in alternatives {
                if *d != read && !second_dates.contains(d) {
                    second_dates.push(*d);
                }
            }
            return Some(DateLineReading {
                date: Some(read),
                weekday_mismatch: None,
                parenthesized_mismatch: None,
                second_dates,
            });
        }
    }
    if governed.iter().all(|g| !*g) {
        return None;
    }
    let mut qualified: Vec<SimpleDate> = Vec::new();
    for (idx, (d, _, _)) in positional.iter().enumerate() {
        if !governed[idx] && !qualified.contains(d) {
            qualified.push(*d);
        }
    }
    if qualified.is_empty() {
        // Every signal governed: the group's
        // silence, on the plain surface too.
        let mut second_dates: Vec<SimpleDate> = Vec::new();
        for d in alternatives {
            if !second_dates.contains(d) {
                second_dates.push(*d);
            }
        }
        return Some(DateLineReading {
            date: None,
            weekday_mismatch: None,
            parenthesized_mismatch: None,
            second_dates,
        });
    }
    let read = qualified[0];
    let mut second_dates: Vec<SimpleDate> = Vec::new();
    for (d, _, _) in &positional {
        if *d != read && !second_dates.contains(d) {
            second_dates.push(*d);
        }
    }
    for d in alternatives {
        if *d != read && !second_dates.contains(d) {
            second_dates.push(*d);
        }
    }
    Some(DateLineReading {
        date: Some(read),
        weekday_mismatch: None,
        parenthesized_mismatch: None,
        second_dates,
    })
}

/// One signal in a plain line, in written order
/// (round 13, R13-5; extracted round 18, R18-7 so
/// the salvaged-line naming in `date_line` scans a
/// line's signals with the very scan the plain
/// reading uses): a date, and whether it stood
/// framed (an absolute date as written) or relative
/// (a phrase, carried here by its resolution).
struct PlainSignal {
    date: SimpleDate,
    framed: bool,
    // A pure relative phrase's resolution against
    // `anchor`, when this signal is one; None for a
    // framed signal and for a Sunday-after phrase
    // over a framed base (its date does not turn on
    // any anchor). Used only when the line proves
    // mixed, after the scan.
    anchored: Option<SimpleDate>,
    // The token span the signal stands in (round
    // 16, R16-4): the governor composition judges a
    // signal by the words around its own span,
    // exactly as a group's occurrence is judged.
    start: usize,
    end: usize,
}

/// Every signal in a plain line's tokens, in
/// written order, with the round-14 anchor rule
/// applied: only a truly mixed line — a framed
/// signal standing beside a pure relative signal —
/// re-resolves its relative signals against
/// `anchor`; every other shape keeps the `today`
/// resolutions the scan computes.
fn scan_plain_signals(
    tokens: &[&str],
    cleaned: &[String],
    today: SimpleDate,
    anchor: SimpleDate,
) -> Vec<PlainSignal> {
    let mut signals: Vec<PlainSignal> = Vec::new();
    // The token index just past the last signal,
    // and whether that signal was a framed Sunday:
    // a bare "Sunday" standing directly after a
    // framed Sunday date is that date's trailing
    // weekday garnish, not a second signal.
    let mut last_end = 0usize;
    let mut last_framed_sunday = false;
    let mut i = 0;
    while i < tokens.len() {
        // A Sunday-after phrase consumes its run
        // first, as the second-date scan has it:
        // its base is never a separate signal.
        if cleaned[i] == "sunday" && cleaned.get(i + 1).map(String::as_str) == Some("after") {
            // "Sunday after next/this Sunday", the
            // relative base wearing an optional
            // "the": the resolution is the first
            // Sunday strictly after the relative
            // base's own resolution.
            let (which_at, used) = if cleaned.get(i + 2).map(String::as_str) == Some("the") {
                (i + 3, 5)
            } else {
                (i + 2, 4)
            };
            let which = cleaned.get(which_at).map(String::as_str);
            let base_day = cleaned.get(which_at + 1).map(String::as_str);
            if (which == Some("next") || which == Some("this")) && base_day == Some("sunday") {
                let base = relative_sunday_date(which == Some("next"), today);
                let date = first_sunday_after(base);
                let anchored_base = relative_sunday_date(which == Some("next"), anchor);
                let anchored = first_sunday_after(anchored_base);
                signals.push(PlainSignal {
                    date,
                    framed: false,
                    anchored: Some(anchored),
                    start: i,
                    end: i + used,
                });
                last_end = i + used;
                last_framed_sunday = false;
                i += used;
                continue;
            }
            if let Some((base, end)) = sunday_after_phrase_run(tokens, cleaned, i, today) {
                let date = first_sunday_after(base);
                signals.push(PlainSignal {
                    date,
                    framed: false,
                    anchored: None,
                    start: i,
                    end: end.max(i + 1),
                });
                last_end = end.max(i + 1);
                last_framed_sunday = false;
                i = end.max(i + 1);
                continue;
            }
        }
        // A bare relative phrase: "next Sunday" /
        // "this Sunday" (a framed date cannot stand
        // at "next"/"this", so no order is lost).
        if (cleaned[i] == "next" || cleaned[i] == "this")
            && cleaned.get(i + 1).map(String::as_str) == Some("sunday")
        {
            let date = relative_sunday_date(cleaned[i] == "next", today);
            let anchored = relative_sunday_date(cleaned[i] == "next", anchor);
            signals.push(PlainSignal {
                date,
                framed: false,
                anchored: Some(anchored),
                start: i,
                end: i + 2,
            });
            last_end = i + 2;
            last_framed_sunday = false;
            i += 2;
            continue;
        }
        match parse_date_prefix(&tokens[i..], today) {
            Some((d, used)) => {
                signals.push(PlainSignal {
                    date: d,
                    framed: true,
                    anchored: None,
                    start: i,
                    end: i + used.max(1),
                });
                last_end = i + used.max(1);
                last_framed_sunday = d.weekday() == 0;
                i += used.max(1);
            }
            None => {
                // A bare "Sunday" the framed reader
                // did not take (it framed no date
                // here) is a relative signal naming
                // the coming Sunday — unless it
                // stands directly after a framed
                // Sunday date, whose trailing
                // weekday garnish it is.
                // Round 21 (R21-7): the round-20
                // weekday-modifier rule (R20-6)
                // applies on the plain surface too —
                // a weekday word followed by a
                // noun-like word is that noun
                // phrase's modifier, never a relative
                // signal ("Sunday school, November 8,
                // 2026" files for November 8, not for
                // a phantom October 18), exactly as
                // in `dates_inside_group_signals`.
                if cleaned[i] == "sunday"
                    && !(i == last_end && last_framed_sunday)
                    // Round 23 (R23-6): the one
                    // weekday-modifier reading,
                    // verb-form adjective shape
                    // included ("Sunday left
                    // luggage"), as in groups.
                    && !weekday_modifier_at(cleaned, i)
                    // Round 22 (R22-5): a weekday
                    // consumed wholly as a subject
                    // PP's object ("the choir on
                    // Sunday was singing") spawns no
                    // signal on the plain surface
                    // either.
                    && !weekday_in_subject_pp(cleaned, i)
                    // Round 24 (R24-5): the object-
                    // continuation twin — a weekday
                    // PP inside a claim's object
                    // continuation is consumed on the
                    // plain surface too.
                    && !weekday_in_object_continuation_pp(tokens, cleaned, i, today)
                {
                    let date = relative_sunday_date(false, today);
                    let anchored = relative_sunday_date(false, anchor);
                    signals.push(PlainSignal {
                        date,
                        framed: false,
                        anchored: Some(anchored),
                        start: i,
                        end: i + 1,
                    });
                    last_end = i + 1;
                    last_framed_sunday = false;
                }
                i += 1;
            }
        }
    }
    // Round 14 (R14-6): the anchor adjustment,
    // as stated above.
    let is_mixed = signals.iter().any(|s| s.framed) && signals.iter().any(|s| s.anchored.is_some());
    if is_mixed {
        for s in &mut signals {
            if let Some(anchored) = s.anchored {
                s.date = anchored;
            }
        }
    }
    signals
}

/// The closed GARNISH-noun set of the lone-framed
/// fence (round 23, R23-7): the head nouns of the
/// pinned weekday-modifier shapes — printed matter
/// and the standing time-of-day / school garnish of
/// the r21b/r22a pins ("Sunday bulletins", "Sunday
/// school", "Sunday morning", and their
/// determiner- and modifier-wearing twins). The
/// set is closed as listed: a bare weekday + noun
/// phrase whose head stands outside it is content,
/// whatever the noun, and the fence does not fire.
///
/// Round 24 (R24-3): the set completes with the
/// printed-matter kin the lanes found missing in
/// the safe direction — "bulletin" (the singular
/// of its first member), "papers", "newsletter",
/// "leaflet", "sheets" — so "Sunday bulletin,
/// November 1, 2026" and "Sunday newsletter,
/// November 8, 2026" assert as their plural and
/// bulletins twins do. The rest of the kin stay
/// outside it as listed ("paper", "newsletters",
/// "leaflets", "sheet" are not members), and the
/// failure direction for an unlisted noun stays
/// silence.
const GARNISH_NOUNS: [&str; 8] = [
    "bulletins",
    "school",
    "morning",
    "bulletin",
    "papers",
    "newsletter",
    "leaflet",
    "sheets",
];

/// Whether the lone-framed fence stands for a plain
/// line (round 21, R21-7; fenced round 22, R22-7;
/// inverted round 23, R23-7): the line's only signal
/// is the framed date at `start..end`, and beside
/// that date the line carries nothing but a
/// weekday-modifier phrase — a weekday word standing
/// with noun-like words, the framed date's garnish
/// ("Sunday bulletins", "printed Monday bulletins",
/// "Sunday school"). Every clause of the line (split
/// where a clause break stands) is judged: a clause
/// holding no words beside the date is garnish-free;
/// a clause holding words must hold a weekday-
/// modifier phrase and nothing but noun-like words
/// (and determiners) beside it.
///
/// Round 22's fence asked the clause to hold no
/// CONTENT head, judged against a ~70-word list —
/// and the list's failure direction was assertion:
/// every event noun the list missed ("Sunday
/// picnic", "Sunday brunch", "Sunday festival")
/// asserted its date, while a listed noun ("Sunday
/// rehearsal") stayed silent. Round 23 inverts the
/// test: the fence fires only when the phrase is
/// garnish by the closed `GARNISH_NOUNS` set — a
/// bare weekday + noun phrase fires only when its
/// head noun is on the set, and any other noun,
/// event or not, listed or not, blocks the fence.
/// The failure direction for an unlisted noun is
/// now silence, the safe direction, matching the
/// held rule for lone content lines. The
/// content-head list has left the fence entirely.
///
/// The verb-form adjective shape (R23-6) is judged
/// as the phrase it is: weekday + verb-form word +
/// noun-like word ("Sunday left luggage", "Sunday
/// baked beans") is a modifier phrase whose
/// adjective carries it, exactly as the signal
/// scans read it — its pinned shapes assert, as
/// they did before the inversion (the review's
/// "Sunday baked beans" twin among them).
fn lone_framed_fence(tokens: &[&str], cleaned: &[String], start: usize, end: usize) -> bool {
    let mut clauses: Vec<(usize, usize)> = Vec::new();
    let mut clause_start = 0usize;
    for (j, raw) in tokens.iter().enumerate() {
        if clause_break_after(raw) {
            clauses.push((clause_start, j + 1));
            clause_start = j + 1;
        }
    }
    if clause_start < tokens.len() {
        clauses.push((clause_start, tokens.len()));
    }
    // Whether the word at `k` stands outside the
    // framed date's own span: the adjective shape's
    // head noun must be a real word of the clause,
    // not the date itself ("Monday meeting" is a
    // bare weekday + noun phrase whose head is
    // "meeting" — the date after it is no head).
    let outside_date = |k: usize| k < start || k >= end;
    let mut has_phrase = false;
    for (cs, ce) in clauses {
        let mut clause_has_phrase = false;
        let mut clause_has_words = false;
        for i in cs..ce {
            if i >= start && i < end {
                continue;
            }
            let w = &cleaned[i];
            if w.is_empty() {
                continue;
            }
            clause_has_words = true;
            if weekday_word(w).is_some() && i + 1 < ce && outside_date(i + 1) {
                // The verb-form adjective shape
                // first: weekday + verb-form word +
                // noun-like head outside the date
                // span ("Sunday left luggage").
                if cleaned
                    .get(i + 1)
                    .is_some_and(|next| is_finite_verb_form(next))
                    && cleaned.get(i + 2).is_some_and(|next| noun_like_word(next))
                    && i + 2 < ce
                    && outside_date(i + 2)
                {
                    clause_has_phrase = true;
                    continue;
                }
                // The bare shape: weekday + noun
                // phrase — garnish only when the
                // phrase's HEAD is on the closed set
                // ("Sunday picnic" and "Sunday
                // rehearsal" alike read nothing now).
                // Round 24 (R24-3): the head is the
                // phrase's FINAL noun, not the word
                // after the weekday — a stacked
                // phrase passes only when its final
                // noun is garnish, so "Sunday morning
                // rehearsal", "Sunday school picnic",
                // "Sunday morning service", and
                // "Sunday morning meeting" read
                // nothing (their heads stand outside
                // the set), while "Sunday morning"
                // (head "morning"), "Sunday school
                // bulletin" (head "bulletin"), and
                // "Sunday morning bulletins" (head
                // "bulletins") assert. The verb-form
                // adjective shape above keeps its own
                // pass by phrase shape (the standing
                // R23-6 adjudication): it is not
                // routed through this garnish-head
                // test.
                if cleaned.get(i + 1).is_some_and(|next| noun_like_word(next)) {
                    let mut head = i + 1;
                    let mut k = i + 2;
                    while k < ce && outside_date(k) && noun_like_word(&cleaned[k]) {
                        head = k;
                        k += 1;
                    }
                    if !GARNISH_NOUNS.contains(&cleaned[head].as_str()) {
                        return false;
                    }
                    clause_has_phrase = true;
                    continue;
                }
            }
            // The verb-form adjective word of a
            // phrase starting at the word before it
            // ("left" in "Sunday left luggage"):
            // the phrase's own material.
            if i > cs
                && weekday_word(&cleaned[i - 1]).is_some()
                && is_finite_verb_form(w)
                && cleaned.get(i + 1).is_some_and(|next| noun_like_word(next))
                && outside_date(i + 1)
            {
                continue;
            }
            // Round 24 (R24-8): a possessive
            // determiner standing before the weekday
            // phrase ("our Sunday bulletins", "my
            // Sunday bulletins", "their Sunday
            // bulletins") is the phrase's own material
            // exactly as a plain determiner is — it
            // does not block the fence, and the phrase
            // is still judged by its head noun above.
            if noun_like_word(w)
                || matches!(w.as_str(), "the" | "a" | "an")
                || POSSESSIVE_DETERMINERS.contains(&w.as_str())
            {
                continue;
            }
            return false;
        }
        if clause_has_words && !clause_has_phrase {
            return false;
        }
        has_phrase = has_phrase || clause_has_phrase;
    }
    has_phrase
}

/// A plain line naming two or more framed dates
/// and reading in no other way (round 12, R12-6):
/// "October 25, 2026 and November 1, 2026" used to
/// read as naming no Sunday at all. First-date
/// primacy — the standing rule inside groups — is
/// the line's rule too: the first framed date is
/// the line's date, and the remaining DISTINCT
/// dates join the second-date note. The same date
/// written twice is two runs of one date: it reads
/// as that date plainly, with nothing noted. A line
/// naming fewer than two date runs is not this
/// shape and keeps its standing readings.
///
/// Round 13 (R13-5): the reading covers MIXED
/// signals — a framed date and a relative phrase
/// ("next Sunday", "this Sunday", a bare "Sunday",
/// "the Sunday after next Sunday", or a Sunday-
/// after phrase, whose resolution the second-date
/// scan already computes) in either order. The
/// FIRST signal is the line's date, the standing
/// primacy: a first framed date reads as itself,
/// and a first relative phrase reads as its
/// resolution against `today`. Every other
/// distinct signal date joins the second-date
/// note — a relative phrase's resolution named as
/// a second date, or a framed date named beside
/// the resolution. The same signal twice — the
/// framed date restated, a relative phrase whose
/// resolution IS the framed date, or the same
/// relative phrase twice — reads plainly, with
/// nothing noted, as the framed twin already
/// does. A line whose only signals are relative
/// phrases naming DIFFERENT Sundays is not this
/// shape: a bare relative line keeps its standing
/// in-words reading, anchored on the bulletin.
///
/// Round 14 (R14-6): a MIXED line's relative
/// phrases resolve against `anchor` — the
/// contribution file's own date, when the parse
/// carries one, else `today` (the caller has
/// already clamped it: never after `today`, and
/// `today` itself when no usable file date
/// stands). The anchor applies ONLY here and
/// ONLY when the line is truly mixed (at least
/// one framed signal and at least one pure
/// relative signal): framed signals still read
/// as written against `today`, a Sunday-after
/// phrase over a framed base still resolves off
/// that base, and a line with no framed signal
/// keeps its standing relative-only reading
/// against `today`. When anchor and today agree
/// the reading is identical to the round-13 one.
fn plain_multi_date_reading(
    core: &str,
    today: SimpleDate,
    anchor: SimpleDate,
    alternatives: &[SimpleDate],
) -> Option<DateLineReading> {
    // Round 18 (R18-1): ONE token cleaning on every
    // surface — the plain path cleans its tokens with
    // `group_token`, exactly as the group path does.
    // The plain cleaner used to leave ":" on its
    // tokens, so a governor wearing a colon
    // ("cancelled: November 1, 2026, next Sunday")
    // governed inside parentheses and was invisible
    // on a plain line, and the cancelled date armed.
    // The group set also strips "-" and the en/em
    // dashes, which the plain set left on.
    let tokens: Vec<&str> = core.split_whitespace().collect();
    let cleaned: Vec<String> = tokens.iter().map(|t| group_token(t)).collect();
    let signals = scan_plain_signals(&tokens, &cleaned, today, anchor);
    if signals.len() < 2 {
        // Round 21 (R21-7): a lone FRAMED signal
        // whose line carries a weekday-modifier
        // phrase ("Sunday school, November 8,
        // 2026", "printed Sunday bulletins,
        // November 8, 2026") still reads — the
        // modifier phrase is the framed date's own
        // garnish (see `scan_plain_signals`: a
        // weekday word followed by a noun-like
        // word is a modifier, never a relative
        // signal), and with the phantom signal
        // gone the framed date is the line's only
        // signal and stands by the same primacy,
        // the composition below judging its
        // governance first as it does for two.
        // A lone framed signal under any other
        // words keeps its standing readings (a
        // content-headed line alone reads
        // nothing), and a lone relative signal
        // keeps the in-words readings.
        let lone_framed = signals.len() == 1 && signals[0].framed;
        // Round 22 (R22-7): the fence fires only when the
        // weekday-modifier phrase is the line's only
        // content beside the framed date — no clause of
        // the line may carry other content: "Monday
        // meeting, service November 8, 2026", "service
        // November 8, 2026, Monday meeting", and
        // "Tuesday choir, November 1, 2026" all read
        // nothing, where the round-21 fence's any-
        // weekday-anywhere scan asserted them. Round 23
        // (R23-7): the phrase must be garnish by the
        // closed set in `lone_framed_fence` — the
        // content-head list that judged this in round
        // 22 has left the fence.
        let modifier_phrase =
            lone_framed && lone_framed_fence(&tokens, &cleaned, signals[0].start, signals[0].end);
        if !(lone_framed && modifier_phrase) {
            return None;
        }
    }
    let framed_count = signals.iter().filter(|s| s.framed).count();
    if framed_count == 0 {
        // No framed signal: only the restated
        // relative phrase — every signal resolving
        // to one date — reads, and it reads
        // plainly. Differing relative phrases keep
        // the standing readings.
        let read = signals[0].date;
        if signals.iter().any(|s| s.date != read) {
            return None;
        }
        let mut second_dates: Vec<SimpleDate> = Vec::new();
        for d in alternatives {
            if *d != read && !second_dates.contains(d) {
                second_dates.push(*d);
            }
        }
        return Some(DateLineReading {
            date: Some(read),
            weekday_mismatch: None,
            parenthesized_mismatch: None,
            second_dates,
        });
    }
    // Round 16 (R16-4): a line whose signals are all
    // framed composes its governors like a group —
    // "November 1, 2026, postponed, drop it,
    // November 8, 2026" asserts November 8, November
    // 1 governed and named. Round 17 (R17-2): the
    // composition reaches MIXED lines too — a
    // relative phrase DOES have a token span (the
    // phrase's own words), so every signal, framed
    // or relative, is judged by the words around its
    // span: "November 1, 2026, cancelled, next
    // Sunday" asserts October 25 (the relative's
    // resolution, resolved exactly as round 13
    // resolved it, the anchor rules unchanged) with
    // November 1 governed and named. The composition
    // returns None when no signal is governed, so a
    // mixed line with no governor phrase, run, or
    // target claim keeps the standing first-signal
    // reading below byte-for-byte, and single-date
    // plain lines never reach this reading at all.
    if framed_count > 0 {
        let spans: Vec<(SimpleDate, usize, usize)> =
            signals.iter().map(|s| (s.date, s.start, s.end)).collect();
        if let Some(reading) = plain_composition(&tokens, &cleaned, &spans, alternatives) {
            return Some(reading);
        }
    }
    let read = signals[0].date;
    let mut second_dates: Vec<SimpleDate> = Vec::new();
    for s in &signals[1..] {
        if s.date != read && !second_dates.contains(&s.date) {
            second_dates.push(s.date);
        }
    }
    for d in alternatives {
        if *d != read && !second_dates.contains(d) {
            second_dates.push(*d);
        }
    }
    Some(DateLineReading {
        date: Some(read),
        weekday_mismatch: None,
        parenthesized_mismatch: None,
        second_dates,
    })
}

/// The governor composition's reading of a plain
/// line's core, alone (round 18, R18-7): the same
/// scan and the same composition
/// `plain_multi_date_reading` uses, but WITHOUT the
/// first-signal fallback — None when no signal is
/// governed, so a caller can tell a composition
/// reading from a primacy reading and take only the
/// composition's naming.
fn plain_composition_reading(
    core: &str,
    today: SimpleDate,
    anchor: SimpleDate,
    alternatives: &[SimpleDate],
) -> Option<DateLineReading> {
    let tokens: Vec<&str> = core.split_whitespace().collect();
    let cleaned: Vec<String> = tokens.iter().map(|t| group_token(t)).collect();
    let signals = scan_plain_signals(&tokens, &cleaned, today, anchor);
    if signals.len() < 2 || !signals.iter().any(|s| s.framed) {
        return None;
    }
    let spans: Vec<(SimpleDate, usize, usize)> =
        signals.iter().map(|s| (s.date, s.start, s.end)).collect();
    plain_composition(&tokens, &cleaned, &spans, alternatives)
}

/// The date values a plain core governs when its
/// composition governs EVERY signal it carries
/// (round 20, R20-1): the distinct signal dates of
/// a core whose `plain_composition` reading is the
/// every-signal-governed silence. A core that
/// asserts, or governs nothing, or carries no
/// signal, governs no value for this purpose — an
/// asserting core composes with its groups through
/// the standing garnish comparison, not through
/// this rule. The composition itself is the judge:
/// a single governed date ("November 1, 2026,
/// cancelled") and a multi-signal all-governed
/// core are read by the same scan the plain
/// reading uses, so no second governor test
/// stands beside it.
fn every_governed_core_values(
    core: &str,
    today: SimpleDate,
    anchor: SimpleDate,
) -> Vec<SimpleDate> {
    let tokens: Vec<&str> = core.split_whitespace().collect();
    let cleaned: Vec<String> = tokens.iter().map(|t| group_token(t)).collect();
    let mut signals = scan_plain_signals(&tokens, &cleaned, today, anchor);
    if signals.is_empty() {
        return Vec::new();
    }
    // Round 21 (R21-4): every value this composition
    // binds resolves its relative signals at the parse
    // anchor — the same anchor the groups use since
    // R19-3. The plain scan re-resolves relatives at
    // the anchor only on a MIXED line, so a governed
    // core whose value is a relative phrase alone
    // ("next Sunday, cancelled") kept its scan-day
    // resolution (November 1 on an October 23 scan of
    // an October 16 file) and never met the group
    // restating its anchored value (October 25): the
    // group asserted the cancelled date and the note
    // named a Sunday nobody wrote. Here, where the
    // values are read off for governance binding,
    // the anchor applies to every relative signal.
    for s in &mut signals {
        if let Some(anchored) = s.anchored {
            s.date = anchored;
        }
    }
    let spans: Vec<(SimpleDate, usize, usize)> =
        signals.iter().map(|s| (s.date, s.start, s.end)).collect();
    let no_alternatives: &[SimpleDate] = &[];
    match plain_composition(&tokens, &cleaned, &spans, no_alternatives) {
        Some(reading) if reading.date.is_none() => {
            let mut values: Vec<SimpleDate> = Vec::new();
            for s in &signals {
                if !values.contains(&s.date) {
                    values.push(s.date);
                }
            }
            values
        }
        _ => Vec::new(),
    }
}

/// One composition for a governed plain core and
/// its trailing groups (round 20, R20-1): the
/// reading `date_line` takes when the core's own
/// composition reads no date because every core
/// signal is governed, and trailing groups stand
/// beside it. Before this, the two paths split on
/// exactly this shape: the For Sunday reading
/// returned the core's silence and dropped the
/// groups unnamed, while the Keep-through
/// fallback asserted the last group blind — in
/// the base witness asserting the very date the
/// core cancelled.
///
/// The composition is the paren-only composition
/// with the core standing where a sibling group
/// stands in the cross-group rule (round 19,
/// R19-2): the core's governed values bind a
/// trailing group's BARE restatement of the same
/// value — governance crosses the core/group
/// boundary exactly as it crosses a group
/// boundary, and binds exactly the same
/// occurrences (a labelled group, a claim's
/// target, and a group governed in itself are
/// judged as they always were). A surviving
/// group then stands by the trailing-group
/// primacy the paren-only path uses: the last
/// surviving group is the line's date, earlier
/// surviving groups disagree with it on the
/// mismatch record, a chain's superseded targets
/// and a two-group relative loser are named on
/// the note as the paren-only path names them,
/// and the core's governed values are named on
/// the note beside the read date, as the plain
/// composition names its governed dates. When
/// no group survives, the line reads nothing and
/// names only the groups' alternatives, as a
/// no-reading line does. None when the core is
/// not an every-governed core, or no trailing
/// group stands at all: every other shape keeps
/// its standing reading.
fn governed_core_group_reading(
    text: &str,
    core: &str,
    today: SimpleDate,
    anchor: SimpleDate,
    alternatives: &[SimpleDate],
    core_mismatch: Option<WeekdayMismatch>,
) -> Option<DateLineReading> {
    let governed_values = every_governed_core_values(core, today, anchor);
    if governed_values.is_empty() {
        return None;
    }
    let analyses = trailing_group_analyses(text, today, anchor);
    if analyses.is_empty() {
        return None;
    }
    // The core's governance applied to the
    // groups: a bare contribution of a governed
    // value is suppressed; every other
    // contribution stands as the cross-group pass
    // left it.
    let surviving: Vec<SimpleDate> = analyses
        .iter()
        .filter_map(|(_, a)| {
            a.contributed
                .filter(|d| !(a.contributed_bare && governed_values.contains(d)))
        })
        .collect();
    let Some(read) = surviving.last().copied() else {
        let mut second_dates: Vec<SimpleDate> = Vec::new();
        for d in alternatives {
            if !second_dates.contains(d) {
                second_dates.push(*d);
            }
        }
        return Some(DateLineReading {
            date: None,
            weekday_mismatch: core_mismatch,
            parenthesized_mismatch: None,
            second_dates,
        });
    };
    // A split chain's superseded targets are
    // named on the note, not compared as garnish,
    // as on the paren-only path — counting only
    // targets that survived the core's governance.
    let superseded: Vec<SimpleDate> = chain_superseded_dates(text, today, anchor, read)
        .into_iter()
        .filter(|d| surviving.contains(d))
        .collect();
    let remaining: Vec<SimpleDate> = surviving
        .iter()
        .copied()
        .filter(|d| !superseded.contains(d))
        .collect();
    let parenthesized_mismatch = mismatch_from_groups(&remaining, read, true);
    let mut second_dates: Vec<SimpleDate> = Vec::new();
    for d in &governed_values {
        if *d != read && !second_dates.contains(d) {
            second_dates.push(*d);
        }
    }
    for d in alternatives {
        if *d != read && !second_dates.contains(d) {
            second_dates.push(*d);
        }
    }
    for d in &superseded {
        if *d != read && !second_dates.contains(d) {
            second_dates.push(*d);
        }
    }
    // Round 19 (R19-10)'s loser naming, generalized
    // to multi-group readings by round 20 (R20-9),
    // as the paren-only path applies it.
    if surviving.len() >= 2
        && analyses
            .iter()
            .any(|(inner, _)| group_inner_is_relative(inner))
    {
        for d in &surviving {
            if *d != read && !second_dates.contains(d) {
                second_dates.push(*d);
            }
        }
    }
    Some(DateLineReading {
        date: Some(read),
        weekday_mismatch: None,
        parenthesized_mismatch,
        second_dates,
    })
}

/// What a date line's plain core — the line with
/// every parenthesized group taken out, wherever
/// the groups stood — contributes to the whole-line
/// composition (round 21, R21-1/R21-2/R21-3).
enum CoreProfile {
    /// The core asserts a date: by the inner readers
    /// (a framed or Sunday-after core), by the plain
    /// composition / first-signal primacy, or — a
    /// single framed signal headed by its own words —
    /// as a content-headed core ("service November 1,
    /// 2026", "choir November 1, 2026"), whose date
    /// round 20's composition never reached because
    /// no plain reader asserts a lone content-headed
    /// signal. `single_signal` marks that last family:
    /// it is the family the composition extends to,
    /// beside groups, for the first time.
    Assert {
        read: SimpleDate,
        second_dates: Vec<SimpleDate>,
        single_signal: bool,
    },
    /// The core asserts nothing and holds values the
    /// composition binds by: an every-governed core's
    /// governed values, or a for-occasion core's
    /// occasion date ("it was scheduled for November
    /// 1, 2026", "we moved the meeting for November 1,
    /// 2026") — the occasion date is the core's value
    /// for governance binding, while the core alone
    /// keeps reading nothing, as it always has.
    Governed {
        values: Vec<SimpleDate>,
        for_occasion: bool,
    },
}

/// A plain core's composition profile, read with the
/// standing readers in their standing order — the
/// inner readers, the plain multi-date reading, the
/// every-governed values — and, for a lone framed
/// signal none of them asserts or governs, the
/// signal's own standing: a for-occasion (its window
/// ends in a passive or active "for" construction)
/// contributes its occasion date as a governed
/// value; any other lone framed signal is a
/// content-headed core and asserts its date. A
/// weekday mismatch on the core, a relative-only
/// core, and a core no reader reaches have no
/// profile: the whole-line composition does not
/// touch them, and their standing readings stand.
fn core_composition_profile(
    core: &str,
    today: SimpleDate,
    anchor: SimpleDate,
) -> Option<CoreProfile> {
    let core_raw = Some(core.to_string());
    let (inner_date, inner_mismatch) = date_line_inner(&core_raw, today);
    if inner_mismatch.is_some() {
        return None;
    }
    if let Some(read) = inner_date {
        let mut second_dates = second_dates_in_line(core, today, read);
        // Round 18 (R18-7)'s merge, as in `date_line`:
        // the plain composition's naming joins the
        // note when it reads the same date.
        if let Some(plain) = plain_composition_reading(core, today, anchor, &[]) {
            if plain.date == Some(read) {
                let consumed_bases = sunday_after_consumed_bases(core, today);
                for d in &plain.second_dates {
                    if *d != read && !consumed_bases.contains(d) && !second_dates.contains(d) {
                        second_dates.push(*d);
                    }
                }
            }
        }
        let profile = CoreProfile::Assert {
            read,
            second_dates,
            single_signal: false,
        };
        return Some(profile);
    }
    if let Some(reading) = plain_multi_date_reading(core, today, anchor, &[]) {
        if let Some(read) = reading.date {
            let profile = CoreProfile::Assert {
                read,
                second_dates: reading.second_dates,
                single_signal: false,
            };
            return Some(profile);
        }
        // An every-signal-governed silence: its
        // governed values are read off below.
    }
    let governed_values = every_governed_core_values(core, today, anchor);
    if !governed_values.is_empty() {
        let profile = CoreProfile::Governed {
            values: governed_values,
            for_occasion: false,
        };
        return Some(profile);
    }
    let tokens: Vec<&str> = core.split_whitespace().collect();
    let cleaned: Vec<String> = tokens.iter().map(|t| group_token(t)).collect();
    let signals = scan_plain_signals(&tokens, &cleaned, today, anchor);
    if signals.len() != 1 || !signals[0].framed {
        return None;
    }
    let signal = &signals[0];
    let window_start = governor_window_start(&tokens, &cleaned, 0, signal.start, false);
    let window = &cleaned[window_start..signal.start.min(cleaned.len())];
    let for_occasion = passive_for_zone_start(&cleaned[..window_start], window).is_some()
        || active_for_zone_start(window).is_some();
    if for_occasion {
        let profile = CoreProfile::Governed {
            values: vec![signal.date],
            for_occasion: true,
        };
        return Some(profile);
    }
    let profile = CoreProfile::Assert {
        read: signal.date,
        second_dates: Vec::new(),
        single_signal: true,
    };
    Some(profile)
}

/// The whole-line composition (round 21, R21-1 /
/// R21-2 / R21-3): ONE composition for every segment
/// of a date line, in written order — the R20-1
/// composition, which covered a governed plain core
/// and its TRAILING groups, extended to the whole
/// line. The line's plain core is the line with
/// every parenthesized group taken out, wherever
/// the groups stood, and every group on the line —
/// leading, trailing, or standing between plain
/// segments — composes with that core by the same
/// rules the trailing groups compose by:
///
/// - Beside an ASSERTING core, every group is
///   garnish: a group that disagrees with the read
///   date is named on the mismatch record, a group
///   that agrees is silent — leading groups exactly
///   as trailing ones ("(November 8) November 15,
///   2026" asserts November 15 and names November 8;
///   "(November 15) November 15, 2026" asserts
///   November 15, silently). A plain segment after
///   the last group is part of the core, so a
///   post-group date is judged by the plain
///   composition itself, never by a first-signal
///   reading that displaces the groups.
/// - Beside a GOVERNED core (an every-governed core,
///   or a for-occasion core by its occasion date),
///   the core's values bind a group's BARE
///   restatement of the same value, wherever the
///   group stands; a surviving group asserts by the
///   paren-only primacy (the last surviving group
///   stands, earlier survivors named on the mismatch
///   record), and the core's values are named on the
///   note beside the read date. When no group
///   survives, the line reads nothing.
///
/// The composition is entered only for the shapes
/// the standing readings do not already compose: a
/// group standing anywhere but the trailing run, or
/// a lone content-headed / for-occasion core beside
/// its trailing groups. Every other shape — the
/// paren-only line, the relative in-words line, the
/// mid-date group, a framed or governed core beside
/// trailing groups alone — keeps its standing
/// reading byte for byte, and this returns None.
fn whole_line_composition(
    text: &str,
    today: SimpleDate,
    anchor: SimpleDate,
) -> Option<DateLineReading> {
    let spans = balanced_spans(text);
    if spans.is_empty() {
        return None;
    }
    // The mid-date group ("October 25 (November 1),
    // 2026") is the mid-date reading's shape, not a
    // segment boundary: a group followed by a comma
    // and a year belongs to the date it stands in.
    for (_, end) in &spans {
        let after = text[*end..].trim_start();
        if let Some(after_comma) = after.strip_prefix(',') {
            let year_token = after_comma.split_whitespace().next().unwrap_or("");
            let year_clean = year_token.trim_matches(['.', ',']);
            if (year_clean.len() == 2 || year_clean.len() == 4)
                && year_clean.chars().all(|c| c.is_ascii_digit())
            {
                return None;
            }
        }
    }
    // The trailing run: the groups the standing
    // readings already compose with the core.
    let mut trailing_run = 0usize;
    {
        let mut rest = text;
        while let Some(stripped) = strip_trailing_parenthesized(rest) {
            trailing_run += 1;
            rest = stripped;
        }
    }
    let has_non_trailing_group = spans.len() > trailing_run;
    // The core: the line with every group taken out.
    // A group leaves a clause break behind it — a
    // semicolon token, which the date readers treat
    // as a break and never absorb into a date run
    // the way a comma token is absorbed: the group
    // is a segment boundary, so a governor standing
    // before it ("cancelled (November 8) November 8,
    // 2026") is spent where it stood and does not
    // reach across the group to govern the plain
    // segment after it — that segment is judged on
    // its own words, as a segment.
    let mut core = String::with_capacity(text.len());
    let mut at = 0usize;
    for (start, end) in &spans {
        core.push_str(&text[at..*start]);
        core.push(';');
        at = *end;
    }
    core.push_str(&text[at..]);
    let core = core.trim();
    let profile = match core_composition_profile(core, today, anchor) {
        Some(profile) => profile,
        None => {
            // Round 22 (R22-1): a groups-only line whose
            // groups are separated by a semicolon, a dash,
            // a comma, or plain words — even a filler word
            // like "note" — composes exactly as the same
            // groups standing adjacent do. The de-grouped
            // core carries no signal of its own (no date,
            // no relative phrase), so there is no core
            // profile; the line is the paren-only
            // composition over every group on the line,
            // in written order, and the Keep-through
            // fallback (which reads this composition
            // through `trailing_dates_after_core_governance`)
            // and the For Sunday reading agree on it.
            let core_tokens: Vec<&str> = core.split_whitespace().collect();
            let core_cleaned: Vec<String> = core_tokens.iter().map(|t| group_token(t)).collect();
            let core_signals = scan_plain_signals(&core_tokens, &core_cleaned, today, anchor);
            if !core_signals.is_empty() {
                return None;
            }
            let entries = all_group_analyses(text, today, anchor);
            let mut alternatives: Vec<SimpleDate> = Vec::new();
            for (_, a) in &entries {
                for d in &a.alternatives {
                    if !alternatives.contains(d) {
                        alternatives.push(*d);
                    }
                }
            }
            let contributed: Vec<SimpleDate> =
                entries.iter().filter_map(|(_, a)| a.contributed).collect();
            let Some(read) = contributed.last().copied() else {
                return Some(DateLineReading {
                    date: None,
                    weekday_mismatch: None,
                    parenthesized_mismatch: None,
                    second_dates: alternatives,
                });
            };
            let superseded = chain_superseded_dates(text, today, anchor, read);
            let remaining: Vec<SimpleDate> = contributed
                .iter()
                .copied()
                .filter(|d| !superseded.contains(d))
                .collect();
            let parenthesized_mismatch = mismatch_from_groups(&remaining, read, true);
            let mut second_dates: Vec<SimpleDate> = Vec::new();
            for d in &alternatives {
                if *d != read && !second_dates.contains(d) {
                    second_dates.push(*d);
                }
            }
            for d in &superseded {
                if *d != read && !second_dates.contains(d) {
                    second_dates.push(*d);
                }
            }
            if contributed.len() >= 2
                && entries
                    .iter()
                    .any(|(inner, _)| group_inner_is_relative(inner))
            {
                for d in &contributed {
                    if *d != read && !second_dates.contains(d) {
                        second_dates.push(*d);
                    }
                }
            }
            return Some(DateLineReading {
                date: Some(read),
                weekday_mismatch: None,
                parenthesized_mismatch,
                second_dates,
            });
        }
    };
    // Enter only for the shapes named above: a group
    // outside the trailing run, or the lone-signal
    // families (a content-headed core asserting, a
    // for-occasion core governing) beside any group.
    let extends_core = match &profile {
        CoreProfile::Assert { single_signal, .. } => *single_signal,
        CoreProfile::Governed { for_occasion, .. } => *for_occasion,
    };
    if !has_non_trailing_group && !extends_core {
        return None;
    }
    let entries = all_group_analyses(text, today, anchor);
    let alternatives: Vec<SimpleDate> = {
        let mut found: Vec<SimpleDate> = Vec::new();
        for (_, a) in &entries {
            for d in &a.alternatives {
                if !found.contains(d) {
                    found.push(*d);
                }
            }
        }
        found
    };
    let entry_dates: Vec<(String, Option<SimpleDate>)> = entries
        .iter()
        .map(|(inner, a)| (inner.clone(), a.contributed))
        .collect();
    match profile {
        CoreProfile::Assert {
            read, second_dates, ..
        } => {
            let superseded = chain_superseded_from_entries(&entry_dates, read, today, anchor);
            let group_dates: Vec<SimpleDate> =
                entries.iter().filter_map(|(_, a)| a.contributed).collect();
            let remaining: Vec<SimpleDate> = group_dates
                .iter()
                .copied()
                .filter(|d| !superseded.contains(d))
                .collect();
            let parenthesized_mismatch = mismatch_from_groups(&remaining, read, false);
            let mut named: Vec<SimpleDate> = second_dates;
            for d in &superseded {
                if *d != read && !named.contains(d) {
                    named.push(*d);
                }
            }
            for d in &alternatives {
                if *d != read && !named.contains(d) {
                    named.push(*d);
                }
            }
            Some(DateLineReading {
                date: Some(read),
                weekday_mismatch: None,
                parenthesized_mismatch,
                second_dates: named,
            })
        }
        CoreProfile::Governed { values, .. } => {
            let surviving: Vec<SimpleDate> = entries
                .iter()
                .filter_map(|(_, a)| {
                    a.contributed
                        .filter(|d| !(a.contributed_bare && values.contains(d)))
                })
                .collect();
            let Some(read) = surviving.last().copied() else {
                let mut named: Vec<SimpleDate> = Vec::new();
                for d in &alternatives {
                    if !named.contains(d) {
                        named.push(*d);
                    }
                }
                return Some(DateLineReading {
                    date: None,
                    weekday_mismatch: None,
                    parenthesized_mismatch: None,
                    second_dates: named,
                });
            };
            let superseded: Vec<SimpleDate> =
                chain_superseded_from_entries(&entry_dates, read, today, anchor)
                    .into_iter()
                    .filter(|d| surviving.contains(d))
                    .collect();
            let remaining: Vec<SimpleDate> = surviving
                .iter()
                .copied()
                .filter(|d| !superseded.contains(d))
                .collect();
            let parenthesized_mismatch = mismatch_from_groups(&remaining, read, true);
            let mut named: Vec<SimpleDate> = Vec::new();
            for d in &values {
                if *d != read && !named.contains(d) {
                    named.push(*d);
                }
            }
            for d in &alternatives {
                if *d != read && !named.contains(d) {
                    named.push(*d);
                }
            }
            for d in &superseded {
                if *d != read && !named.contains(d) {
                    named.push(*d);
                }
            }
            // Round 19 (R19-10)'s loser naming, as in
            // `governed_core_group_reading`.
            if surviving.len() >= 2
                && entries
                    .iter()
                    .any(|(inner, _)| group_inner_is_relative(inner))
            {
                for d in &surviving {
                    if *d != read && !named.contains(d) {
                        named.push(*d);
                    }
                }
            }
            Some(DateLineReading {
                date: Some(read),
                weekday_mismatch: None,
                parenthesized_mismatch,
                second_dates: named,
            })
        }
    }
}

/// The trailing groups' contributed dates after
/// the core's governance has bound its bare
/// restatements (round 20, R20-1): the list the
/// Keep-through fallback (round 9, R9-9) reads,
/// so a fallback beside a governed core asserts
/// exactly what the unified composition asserts —
/// a surviving group, never a date the core
/// governed and the group only restated bare.
/// Beside any other core the list is the standing
/// trailing-group list unchanged.
///
/// Round 21 (R21-1/R21-2): the fallback reads the
/// WHOLE line through the same composition — a line
/// the whole-line composition reads contributes
/// exactly its read date (or nothing, when the
/// composition bound every group: a for-occasion
/// core's bare restatement group must not become
/// the keep-through date the For Sunday line
/// refused), and beside a core the composition does
/// not reach (a relative in-words core) the group
/// list is every group on the line, leading groups
/// included, against the core the groups leave
/// behind — "(November 8) next Sunday" keeps
/// through November 8 exactly as its trailing twin
/// does.
fn trailing_dates_after_core_governance(
    text: &str,
    today: SimpleDate,
    anchor: SimpleDate,
) -> Vec<SimpleDate> {
    if let Some(reading) = whole_line_composition(text, today, anchor) {
        return reading.date.into_iter().collect();
    }
    let spans = balanced_spans(text);
    let mut core = String::with_capacity(text.len());
    let mut at = 0usize;
    for (start, end) in &spans {
        core.push_str(&text[at..*start]);
        core.push(';');
        at = *end;
    }
    core.push_str(&text[at..]);
    let governed_values = every_governed_core_values(core.trim(), today, anchor);
    if governed_values.is_empty() {
        // A leading group joins the fallback only
        // beside a relative core it disagrees with
        // (the R21-1 shape above): beside any other
        // core a group standing before the words is
        // not the fallback's to read — the R10-10c
        // boundary — and the list is the standing
        // trailing-group list.
        let core_words = core.replace(';', " ");
        let core_trim = core_words.trim();
        let after_shape = sunday_after_relative_shape(core_trim);
        if after_shape || relative_sunday_shape(core_trim) {
            let resolution = relative_phrase_resolution(core_trim, today, after_shape);
            if leading_group_disagrees(text, today, resolution) {
                return all_parenthesized_dates(text, today, anchor);
            }
        }
        return trailing_parenthesized_dates(text, today, anchor);
    }
    all_group_analyses(text, today, anchor)
        .iter()
        .filter_map(|(_, a)| {
            a.contributed
                .filter(|d| !(a.contributed_bare && governed_values.contains(d)))
        })
        .collect()
}

/// The base dates a line's Sunday-after phrases
/// consume (round 19, R19-8): every "Sunday after
/// <base>" run standing in the line's core, read
/// with the second-date scan's own run scan
/// (`sunday_after_phrase_run`), so a phrase's base
/// is known wherever the phrase stands in the line.
/// A consumed base is part of its phrase — the
/// phrase's resolution is the date the line names —
/// and is never another date beside it.
fn sunday_after_consumed_bases(text: &str, today: SimpleDate) -> Vec<SimpleDate> {
    let core = strip_all_trailing_parenthesized(text.trim());
    let tokens: Vec<&str> = core.split_whitespace().collect();
    let cleaned: Vec<String> = tokens.iter().map(|t| group_token(t)).collect();
    let mut bases: Vec<SimpleDate> = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        if let Some((base, end)) = sunday_after_phrase_run(&tokens, &cleaned, i, today) {
            if !bases.contains(&base) {
                bases.push(base);
            }
            i = end.max(i + 1);
            continue;
        }
        i += 1;
    }
    bases
}

/// Whether a parenthesized group's own words carry
/// a relative Sunday phrase (round 19, R19-10): a
/// "next Sunday" / "this Sunday" phrase, or a
/// "Sunday after" phrase — the group signals a
/// relative Sunday, not only a framed date.
fn group_inner_is_relative(inner: &str) -> bool {
    let tokens: Vec<String> = inner.split_whitespace().map(group_token).collect();
    tokens.windows(2).any(|w| {
        ((w[0] == "next" || w[0] == "this") && w[1] == "sunday")
            || (w[0] == "sunday" && w[1] == "after")
    })
}

/// The weekday mismatch a plain segment of the
/// whole-line composition carries, if one does
/// (round 22, R22-3): the line's de-grouped core —
/// every group taken out, a semicolon standing where
/// each group stood, as `whole_line_composition`
/// builds it — is read segment by segment with the
/// standing inner reader, and the first segment
/// whose framed date disagrees with its weekday
/// answers. A segment standing at the start of the
/// core (directly after a leading group) frames its
/// date there exactly as the same words frame it at
/// the start of a line, and a weekday wearing "on"
/// frames wherever it stands, so no segment position
/// exempts a framed occurrence from the check. A
/// line with no group has no composition segments
/// and is never judged here.
fn composition_segment_weekday_mismatch(text: &str, today: SimpleDate) -> Option<WeekdayMismatch> {
    let spans = balanced_spans(text);
    if spans.is_empty() {
        return None;
    }
    let mut core = String::with_capacity(text.len());
    let mut at = 0usize;
    for (start, end) in &spans {
        core.push_str(&text[at..*start]);
        core.push(';');
        at = *end;
    }
    core.push_str(&text[at..]);
    for segment in core.split(';') {
        // Round 23 (R23-3): the separator standing where
        // the group was taken off — comma, colon, dash
        // (incl. en/em), period, slash, or the word
        // "and" — is a boundary exactly as the space and
        // semicolon already are: it is stripped from the
        // segment's head before the segment is judged, so
        // "(November 8), Monday, November 15, 2026" (and
        // its colon / dash / period / slash / "and"
        // twins) records the same mismatch and asserts
        // nothing, exactly as the space twin does. A
        // segment carrying no word of its own is skipped.
        const SEGMENT_BOUNDARY: [char; 10] = [
            ' ', '\t', ';', ',', ':', '.', '/', '-', '\u{2013}', '\u{2014}',
        ];
        let mut segment = segment.trim_matches(SEGMENT_BOUNDARY);
        // Round 24 (R24-4): a standalone boundary
        // token at the segment's head ("*", "•", "|",
        // "~", "_", an emoji — any token with no
        // letters and no digits) is stripped the same
        // way, before and after the "and" twin, so
        // "(November 1) * Monday November 15, 2026"
        // records its mismatch as its comma twin
        // does.
        segment = strip_leading_boundary_tokens(segment);
        segment = segment.trim_matches(SEGMENT_BOUNDARY);
        if let Some(rest) = segment
            .strip_prefix("and ")
            .or_else(|| segment.strip_prefix("And "))
            .or_else(|| segment.strip_prefix("AND "))
        {
            segment = rest.trim_matches(SEGMENT_BOUNDARY);
            segment = strip_leading_boundary_tokens(segment);
            segment = segment.trim_matches(SEGMENT_BOUNDARY);
        }
        if segment.is_empty() || !segment.chars().any(|c| c.is_alphanumeric()) {
            continue;
        }
        let segment_raw = Some(segment.to_string());
        let (_, mismatch) = date_line_inner(&segment_raw, today);
        if mismatch.is_some() {
            return mismatch;
        }
    }
    None
}

/// Read one date line with its round-9 records: the
/// plain readings are `date_line_inner`'s, and the
/// trailing parenthesized dates compose around them.
///
/// - A line the inner reader reads keeps its date; a
///   trailing parenthesized date that disagrees with
///   it records the garnish mismatch (on a
///   Sunday-after line the comparison is with the
///   resolution, the date the line names).
/// - A plain date line the inner reader cannot read
///   at all — "October 25, 2026 (November 1)", whose
///   parenthesis used to poison the whole reading —
///   is re-read on its core, trailing parenthesized
///   groups off, and the same comparison runs. The
///   parenthesis never silently kills the plain
///   reading.
/// - A line that is only parenthesized groups reads
///   the last group's date (round 10, R10-10a), and a
///   group standing between a month-day and its year
///   is compared as garnish without poisoning the
///   plain date (R10-10b).
/// - Every distinct second absolute date in the
///   line, before the read date or after it, is
///   noted, never substituted (round 10, R10-4).
/// - A plain line naming two or more framed dates
///   and reading in no other way reads its first
///   date, the rest noted (round 12, R12-6);
///   NoSundayNamed is reserved for lines naming no
///   date.
fn date_line(raw: &Option<String>, today: SimpleDate, anchor: SimpleDate) -> DateLineReading {
    let (date, weekday_mismatch) = date_line_inner(raw, today);
    let none_reading = DateLineReading {
        date: None,
        weekday_mismatch: None,
        parenthesized_mismatch: None,
        second_dates: Vec::new(),
    };
    let Some(text) = raw.as_deref().map(str::trim).filter(|s| !s.is_empty()) else {
        return none_reading;
    };
    let groups = trailing_parenthesized_dates(text, today, anchor);
    // Round 11 (R11-6): the alternatives a trailing
    // group offers instead of contributing are named
    // on the second-date note, beside whatever the
    // line itself reads.
    let alternatives = trailing_group_alternatives(text, today, anchor);
    // Round 22 (R22-3): a wrong-weekday framed date
    // standing in a plain segment of the composition
    // speaks first, exactly as it does without the
    // group: "(November 8) Monday, November 15, 2026"
    // yields no date and records the mismatch, where
    // the line used to assert the date because the
    // inner reader treated the weekday as ordinary
    // text standing after the group. The check runs
    // before every composed reading below; a line
    // whose own inner reader already recorded a
    // mismatch records the same one here (its first
    // segment is the line's own opening words).
    if let Some(mismatch) = composition_segment_weekday_mismatch(text, today) {
        return DateLineReading {
            date: None,
            weekday_mismatch: Some(mismatch),
            parenthesized_mismatch: None,
            second_dates: Vec::new(),
        };
    }
    if let Some(read) = date {
        // Round 18 (R18-11): chain links among the
        // trailing groups are named as superseded
        // targets, not compared as disagreeing
        // garnish, when the line's own date is the
        // chain's last target.
        let superseded = chain_superseded_dates(text, today, anchor, read);
        let remaining: Vec<SimpleDate> = groups
            .iter()
            .copied()
            .filter(|d| !superseded.contains(d))
            .collect();
        let parenthesized_mismatch = mismatch_from_groups(&remaining, read, false);
        let mut second_dates = second_dates_in_line(text, today, read);
        for d in &superseded {
            if *d != read && !second_dates.contains(d) {
                second_dates.push(*d);
            }
        }
        // Round 18 (R18-7): a line the inner readers
        // salvage never used to reach the governor
        // composition, so a governed RELATIVE signal
        // standing before the salvaged date was never
        // named — "cancelled, next Sunday, November 1,
        // 2026" asserted November 1 with October 25
        // nowhere. When the plain composition on the
        // line's core reads the SAME date the inner
        // readers read, its naming joins the note:
        // governed signals are named by their resolved
        // date, as framed ones are. Only the
        // COMPOSITION's naming is taken (never the
        // first-signal fallback's, whose second dates
        // the standing second-date scan already
        // judges — a Sunday-after line's restated
        // base stays a restatement, not "another
        // date"), and a composition reading any other
        // date (or none) changes nothing here — the
        // salvaged reading stands exactly as it stood.
        let core = strip_all_trailing_parenthesized(text);
        if !core.trim().is_empty() {
            if let Some(plain) = plain_composition_reading(core, today, anchor, &alternatives) {
                if plain.date == Some(read) {
                    // Round 19 (R19-8): the merge never
                    // names a Sunday-after phrase's
                    // consumed base. The composition's
                    // signal scan splits a restated base
                    // into its own signal, so "the Sunday
                    // after November 1, 2026, November 1,
                    // 2026, cancelled" named November 1 —
                    // the phrase's own base, restated and
                    // governed — beside the November 8 the
                    // phrase resolves to. The standing
                    // second-date scan already excludes a
                    // consumed base; the merge does too.
                    // The salvaged reading itself (what
                    // asserts) is unchanged.
                    let consumed_bases = sunday_after_consumed_bases(core, today);
                    for d in &plain.second_dates {
                        if *d != read && !consumed_bases.contains(d) && !second_dates.contains(d) {
                            second_dates.push(*d);
                        }
                    }
                }
            }
        }
        for d in &alternatives {
            if *d != read && !second_dates.contains(d) {
                second_dates.push(*d);
            }
        }
        return DateLineReading {
            date: Some(read),
            weekday_mismatch,
            parenthesized_mismatch,
            second_dates,
        };
    }
    if weekday_mismatch.is_some() {
        return DateLineReading {
            date: None,
            weekday_mismatch,
            parenthesized_mismatch: None,
            second_dates: Vec::new(),
        };
    }
    // Round 21 (R21-1/R21-2/R21-3): the whole-line
    // composition — a group standing anywhere but
    // the trailing run, or a content-headed /
    // for-occasion core beside its groups — is read
    // here, after the standing weekday-mismatch
    // return (a wrong weekday speaks first, as it
    // always has) and before the standing readings
    // below, which the composition leaves untouched
    // for every other shape (it returns None there).
    if let Some(reading) = whole_line_composition(text, today, anchor) {
        return reading;
    }
    // Round 10 (R10-10b): a group standing between a
    // month-day and its year does not poison the plain
    // date; the group is compared as garnish.
    if let Some(reading) = mid_date_group_reading(text, today, anchor, &groups, &alternatives) {
        return reading;
    }
    // Round 11 (R11-6): the core re-read below no
    // longer waits for a group to have contributed.
    // A group that contributes nothing — several
    // dates, a negated date, no date, nested groups —
    // behaves as if absent: the line's own reading
    // proceeds on its core either way.
    let core = strip_all_trailing_parenthesized(text);
    if core.trim().is_empty() {
        if groups.is_empty() {
            // Only non-contributing groups stand: no
            // date governs, and the alternatives they
            // offered are named on the note.
            return DateLineReading {
                date: None,
                weekday_mismatch: None,
                parenthesized_mismatch: None,
                second_dates: alternatives,
            };
        }
        // Round 10 (R10-10a): a line that is only
        // parenthesized groups reads its date exactly
        // as the Keep-through line has since round 9 —
        // the last group stands, and an earlier group
        // naming a different date is recorded against
        // it, its source stated as the parentheses.
        // Round 15 (R15-6): a date the standing group
        // governed only by its trailing phrase is
        // named on the second-date note, like the
        // alternatives on every other path.
        let read = *groups.last().expect("a group stands");
        // Round 18 (R18-11): a target chain split
        // across the groups names its superseded
        // targets on the note, as the single-group
        // chain does, and those targets are chain
        // links, not disagreeing garnish — they leave
        // the mismatch comparison.
        let superseded = chain_superseded_dates(text, today, anchor, read);
        let remaining: Vec<SimpleDate> = groups
            .iter()
            .copied()
            .filter(|d| !superseded.contains(d))
            .collect();
        let parenthesized_mismatch = mismatch_from_groups(&remaining, read, true);
        let mut second_dates: Vec<SimpleDate> = Vec::new();
        for d in &alternatives {
            if *d != read && !second_dates.contains(d) {
                second_dates.push(*d);
            }
        }
        for d in &superseded {
            if *d != read && !second_dates.contains(d) {
                second_dates.push(*d);
            }
        }
        // Round 19 (R19-10): a two-group reading
        // mixing a framed group and a relative group
        // names its loser on the note, as a plain
        // mixed line names its other signal —
        // "(November 8) (next Sunday)" asserts
        // October 25 (the last group stands) AND
        // names November 8. Before this, the loser
        // stood only on the mismatch record; the
        // note itself carried nothing, on either
        // line's mirror.
        // Round 20 (R20-9): the rule generalizes —
        // in a multi-group reading with at least one
        // relative group, the losers are named on
        // the note the same way ("(November 8)
        // (next Sunday) (November 15)" names
        // November 8 and October 25).
        // The bare framed stack is not this shape:
        // its loser keeps the standing mismatch-only
        // naming the r18b chain pin has.
        if groups.len() >= 2
            && trailing_group_analyses(text, today, anchor)
                .iter()
                .any(|(inner, _)| group_inner_is_relative(inner))
        {
            for d in &groups {
                if *d != read && !second_dates.contains(d) {
                    second_dates.push(*d);
                }
            }
        }
        return DateLineReading {
            date: Some(read),
            weekday_mismatch: None,
            parenthesized_mismatch,
            second_dates,
        };
    }
    if core == text {
        // Round 12 (R12-6): a plain line naming two
        // framed dates reads its first, the rest
        // noted — it does not read as naming none.
        if let Some(reading) = plain_multi_date_reading(core, today, anchor, &alternatives) {
            return reading;
        }
        return DateLineReading {
            date: None,
            weekday_mismatch,
            parenthesized_mismatch: None,
            second_dates: alternatives,
        };
    }
    let core_raw = Some(core.to_string());
    let (core_date, core_mismatch) = date_line_inner(&core_raw, today);
    match core_date {
        Some(read) => {
            let parenthesized_mismatch = mismatch_from_groups(&groups, read, false);
            let mut second_dates = second_dates_in_line(core, today, read);
            for d in &alternatives {
                if *d != read && !second_dates.contains(d) {
                    second_dates.push(*d);
                }
            }
            DateLineReading {
                date: Some(read),
                weekday_mismatch: None,
                parenthesized_mismatch,
                second_dates,
            }
        }
        None => {
            // Round 20 (R20-1): one composition for
            // an every-governed core and its trailing
            // groups — the core's governed values
            // bind bare trailing restatements, and a
            // surviving group asserts here instead
            // of dropping unnamed (see
            // `governed_core_group_reading`).
            if let Some(reading) = governed_core_group_reading(
                text,
                core,
                today,
                anchor,
                &alternatives,
                core_mismatch.clone(),
            ) {
                return reading;
            }
            // Round 12 (R12-6): the same first-date
            // primacy on the de-grouped core, with
            // the trailing groups still compared as
            // garnish against the date it reads.
            if let Some(mut reading) = plain_multi_date_reading(core, today, anchor, &alternatives)
            {
                // Round 16 (R16-4): the composition can
                // read no date at all (every signal
                // governed); then there is no read date
                // for the garnish to disagree with.
                if let Some(read) = reading.date {
                    reading.parenthesized_mismatch = mismatch_from_groups(&groups, read, false);
                }
                return reading;
            }
            // Round 18 (R18-11): the relative path —
            // the core is a relative phrase and the
            // groups carry the chain — names a split
            // chain's superseded targets on the note
            // too, against the last group's date,
            // which the verdict machinery reads as the
            // phrase's parenthesized date.
            let mut second_dates = alternatives.clone();
            if let Some(chain_read) = trailing_parenthesized_date(text, today, anchor) {
                for d in chain_superseded_dates(text, today, anchor, chain_read) {
                    if d != chain_read && !second_dates.contains(&d) {
                        second_dates.push(d);
                    }
                }
            }
            DateLineReading {
                date: None,
                weekday_mismatch: core_mismatch,
                parenthesized_mismatch: None,
                second_dates,
            }
        }
    }
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

/// The part kind a set of heading words names exactly,
/// if it names one.
fn part_kind_for_words(w: &str) -> Option<PartKind> {
    Some(match w {
        "for the service" => PartKind::Service,
        "announcement" | "announcements" => PartKind::Announcement,
        "insert" => PartKind::Insert,
        "sunday school" => PartKind::SundaySchool,
        "youth" => PartKind::Youth,
        _ => return None,
    })
}

/// The part kind a heading line names, if it names one.
/// Any `#` line whose words are not a part name is ordinary
/// body text (a heading inside a part), never a part break.
/// Round 9 (R9-8): part headings match document headings
/// symmetrically — the same rule the bulletin side applies
/// (`heading_words_match` in ob-app). A part heading whose
/// words are a heading's words plus a parenthesized suffix
/// ("## Sunday school (Room 2)") is THAT heading's part:
/// it plans under the base heading, and the suffixed
/// heading itself is never written to the bulletin.
/// Trailing punctuation — a final period — does not
/// defeat the match either; a longer heading ("Sunday
/// school teachers") still does not match.
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
    let core = w.trim_end_matches('.');
    if let Some(kind) = part_kind_for_words(core) {
        return Some(kind);
    }
    for want in [
        "for the service",
        "announcement",
        "announcements",
        "insert",
        "sunday school",
        "youth",
    ] {
        let prefix = format!("{want} (");
        if core.starts_with(&prefix) && core.ends_with(')') {
            return part_kind_for_words(want);
        }
    }
    None
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
    parse_contribution_anchored(text, today, None)
}

/// Read a contribution whose file's own date is known
/// (round 14, R14-6): `file_date` — the contribution
/// file's modification date, when the caller has one —
/// anchors the mixed-signal relative resolutions
/// in `plain_multi_date_reading` and, since round 19
/// (R19-3), every relative signal inside a
/// parenthesized group (see `analyze_group`). A file
/// date that is missing, not a real calendar date, or
/// after `today` falls back to `today`: the anchor
/// never stands in the future beyond today. Round 15
/// (R15-4) adds a plausibility bound on the other
/// side: a file date more than 28 days before
/// `today` also falls back to `today`. Every other
/// reading in the parse — framed, relative-only
/// plain lines, and the standing in-words memory —
/// is anchored exactly as before.
pub fn parse_contribution_anchored(
    text: &str,
    today: SimpleDate,
    file_date: Option<SimpleDate>,
) -> Result<Contribution, String> {
    // The 28-day plausibility bound (round 15,
    // R15-4): four weeks covers the late-file harm
    // this anchor exists for (a file one to two
    // weeks late); beyond it, a preserved timestamp
    // from a zip/rsync/sync copy keeping an old
    // mtime is more likely than a month-old
    // contribution, and anchoring at such a date
    // resolves relatives into a week the sender
    // never meant — a worse read than today's.
    let anchor = match file_date {
        Some(d) if d.is_valid() && d <= today && add_days(d, 28) >= today => d,
        _ => today,
    };
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
    let for_reading = date_line(&raw.for_sunday, today, anchor);
    let for_date = for_reading.date;
    let named_weekday_mismatch = for_reading.weekday_mismatch;
    let kt_reading = date_line(&raw.keep_through, today, anchor);
    let mut keep_through = kt_reading.date;
    let keep_through_weekday_mismatch = kt_reading.weekday_mismatch;
    let mut keep_through_parenthesized_mismatch = kt_reading.parenthesized_mismatch;
    // Round 9 (R9-9): on a Keep-through line whose own
    // words yield no date, a trailing parenthesized
    // absolute date IS the keep-through date — relative
    // phrases on this line stay unread by design (no
    // anchor machinery, recorded since round 6), but
    // the sender's one absolute date is not lost. The
    // last group stands, as the For Sunday memory has
    // always taken it; an earlier group naming a
    // different date records the keep-through mismatch.
    if keep_through.is_none()
        && keep_through_weekday_mismatch.is_none()
        && keep_through_parenthesized_mismatch.is_none()
    {
        if let Some(kt_text) = raw.keep_through.as_deref() {
            // Round 20 (R20-1): the fallback reads
            // the groups through the same core/group
            // composition as the For Sunday line — a
            // bare restatement of a value the core
            // governs never becomes the keep-through
            // date, and a surviving group is the one
            // the composition stands on.
            let groups = trailing_dates_after_core_governance(kt_text, today, anchor);
            if let Some(chosen) = groups.last().copied() {
                keep_through = Some(chosen);
                // Round 10 (R10-6): every distinct
                // disagreeing group is recorded, and
                // the record says the kept date came
                // from a parenthesized group.
                keep_through_parenthesized_mismatch = mismatch_from_groups(&groups, chosen, true);
            }
        }
    }
    // A Sunday named only in relative words leaves no
    // date and no mismatch, but it is not silence: the
    // words are remembered as written (round 5, RT5-6).
    let for_sunday_in_words = if for_date.is_none() && named_weekday_mismatch.is_none() {
        relative_sunday_phrase(&raw.for_sunday, today)
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
        sunday_after_relative_phrase(&raw.for_sunday, today)
    } else {
        None
    };
    // A trailing parenthesized absolute date on a
    // relative phrase is remembered with the phrase
    // (round 8, carried shape): the verdict compares
    // it with the relative resolution once a bulletin
    // anchors it. A parenthesis that does not read as
    // a date is garnish and leaves no memory.
    let for_sunday_parenthesized_date =
        if for_sunday_in_words.is_some() || for_sunday_after_in_words.is_some() {
            raw.for_sunday
                .as_deref()
                .and_then(|t| all_parenthesized_date(t, today, anchor))
        } else {
            None
        };
    // Round 9 (R9-10): every trailing parenthesized
    // date group on the relative phrase is remembered,
    // not only the last, so the verdict compares each.
    // Round 21 (R21-1): the memory covers the line's
    // groups wherever they stand — a leading group
    // beside the phrase is remembered exactly as a
    // trailing one is.
    let for_sunday_parenthesized_dates =
        if for_sunday_in_words.is_some() || for_sunday_after_in_words.is_some() {
            raw.for_sunday
                .as_deref()
                .map(|t| all_parenthesized_dates(t, today, anchor))
                .unwrap_or_default()
        } else {
            Vec::new()
        };
    let for_sunday_second_dates = for_reading.second_dates.clone();
    let keep_through_second_dates = kt_reading.second_dates.clone();
    let for_sunday_second_date = for_reading.second_dates.first().copied();
    let keep_through_second_date = kt_reading.second_dates.first().copied();
    Ok(Contribution {
        from: raw.from.filter(|s| !s.trim().is_empty()),
        for_date,
        keep_through,
        parts,
        named_weekday_mismatch,
        keep_through_weekday_mismatch,
        for_sunday_in_words,
        for_sunday_after_in_words,
        for_sunday_parenthesized_date,
        for_sunday_parenthesized_dates,
        for_sunday_parenthesized_mismatch: for_reading.parenthesized_mismatch,
        keep_through_parenthesized_mismatch,
        for_sunday_second_date,
        keep_through_second_date,
        for_sunday_second_dates,
        keep_through_second_dates,
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

/// The distinct parenthesized dates a relative Sunday
/// phrase carried, in written order (round 9, R9-10).
/// Hand-built contributions that set only the round-8
/// single field still read through it.
fn distinct_parenthesized_dates(c: &Contribution) -> Vec<SimpleDate> {
    let mut out: Vec<SimpleDate> = Vec::new();
    for d in &c.for_sunday_parenthesized_dates {
        if !out.contains(d) {
            out.push(*d);
        }
    }
    if out.is_empty() {
        if let Some(d) = c.for_sunday_parenthesized_date {
            out.push(d);
        }
    }
    out
}

impl Contribution {
    /// The review's first line when a Sunday-after
    /// phrase's parenthesized date IS the open
    /// bulletin's date (round 9, R9-11): the verdict is
    /// SameSunday on the sender's explicit date, and
    /// the sentence notes in plain words that the words
    /// also named a relative Sunday that would be the
    /// resolution date. None when that is not the shape.
    pub fn parenthesized_same_sunday_sentence(&self, bulletin: SimpleDate) -> Option<String> {
        self.for_sunday_after_in_words.as_ref()?;
        let dates = distinct_parenthesized_dates(self);
        if dates == [bulletin] {
            let said = format_date_long(bulletin);
            let resolved = format_date_long(first_sunday_after(bulletin));
            return Some(format!(
                "These words say they are for {said}, the date written in \
                 parentheses. The words also name a Sunday in relative words \
                 that would be {resolved}."
            ));
        }
        // Round 10 (R10-7): stacked groups whose first
        // disagreement with the resolution IS the open
        // bulletin's date read SameSunday on that
        // bulletin-equal parenthesis — a DifferentSunday
        // verdict never carries the bulletin's own
        // date — and the note names every group, the
        // disagreeing garnish included.
        let resolved_date = first_sunday_after(bulletin);
        let first_disagreeing = dates.iter().find(|d| **d != resolved_date);
        if first_disagreeing != Some(&bulletin) {
            return None;
        }
        let said = format_date_long(bulletin);
        let resolved = format_date_long(resolved_date);
        let listed = dates
            .iter()
            .map(|d| format_date_long(*d))
            .collect::<Vec<String>>()
            .join(" and ");
        Some(format!(
            "These words say they are for {said}, the date written in \
             parentheses. The words also name a Sunday in relative words \
             that would be {resolved}. The dates in parentheses do not \
             agree with each other: {listed}."
        ))
    }

    /// The review's first line when a Sunday-after
    /// phrase's parenthesized date (or dates) disagree
    /// with the relative resolution without matching
    /// the open bulletin (round 9, R9-11/R9-10): the
    /// armed sentence names the resolution and the
    /// parenthesized date — never one date on both
    /// sides. None when that is not the shape; the
    /// ordinary DifferentSunday sentence then stands.
    pub fn parenthesized_different_sentence(&self, bulletin: SimpleDate) -> Option<String> {
        if self.for_sunday_after_in_words.is_none() {
            // Round 10 (R10-6): the bare relative shape
            // gains the listing sentence — with more
            // than one distinct parenthesized date, the
            // armed line names every group and the
            // bulletin, not only the first disagreeing
            // group. A single group keeps the ordinary
            // armed sentence.
            self.for_sunday_in_words.as_ref()?;
            let dates = distinct_parenthesized_dates(self);
            if dates.len() < 2 || dates.iter().all(|d| *d == bulletin) {
                return None;
            }
            let open = format_date_long(bulletin);
            let listed = dates
                .iter()
                .map(|d| format_date_long(*d))
                .collect::<Vec<String>>()
                .join(" and ");
            return Some(format!(
                "These words name a Sunday in relative words that would be \
                 {open}. The dates in parentheses do not agree with \
                 each other: {listed}. The bulletin open is for {open}."
            ));
        }
        let dates = distinct_parenthesized_dates(self);
        if dates.is_empty() {
            return None;
        }
        let resolved_date = first_sunday_after(bulletin);
        let resolved = format_date_long(resolved_date);
        let open = format_date_long(bulletin);
        if dates.len() > 1 {
            if dates.iter().all(|d| *d == resolved_date) {
                return None;
            }
            let listed = dates
                .iter()
                .map(|d| format_date_long(*d))
                .collect::<Vec<String>>()
                .join(" and ");
            return Some(format!(
                "These words name a Sunday in relative words that would be \
                 {resolved}. The dates in parentheses do not agree with \
                 each other: {listed}. The bulletin open is for {open}."
            ));
        }
        let said_date = dates[0];
        if said_date == resolved_date || said_date == bulletin {
            return None;
        }
        let said = format_date_long(said_date);
        Some(format!(
            "These words name a Sunday in relative words that would be \
             {resolved}. The date in parentheses is {said}. \
             The bulletin open is for {open}."
        ))
    }
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
        //
        // Round 8 (carried shape): a trailing
        // parenthesized absolute date is compared with
        // that resolution. Agreement reads exactly as
        // the relative verdict does; disagreement names
        // the sender's own date instead, and arms the
        // same way — the review's sentence names it
        // beside the open bulletin's date.
        (None, Some(open)) if c.for_sunday_after_in_words.is_some() => {
            let resolved = first_sunday_after(open);
            let dates = distinct_parenthesized_dates(c);
            if dates.is_empty() {
                return WeekVerdict::DifferentSunday(resolved);
            }
            if dates.len() == 1 {
                let said = dates[0];
                // Round 9 (R9-11): the parenthesized
                // date is the sender's explicit date and
                // the stronger signal. When it equals
                // the open bulletin's date the verdict is
                // SameSunday, unarmed, and the review
                // notes the relative words beside it.
                if said == open {
                    return WeekVerdict::SameSunday;
                }
                if said == resolved {
                    return WeekVerdict::DifferentSunday(resolved);
                }
                return WeekVerdict::DifferentSunday(said);
            }
            // Round 9 (R9-10): stacked groups are each
            // compared; any disagreement with the
            // resolution arms and names the disagreeing
            // date. Round 10 (R10-7): a DifferentSunday
            // verdict never carries the open bulletin's
            // own date — where the first disagreeing
            // group IS the bulletin's date, the outcome
            // is the SameSunday-with-note composition
            // (the bulletin-equal parenthesis is
            // agreement evidence, as in R9-11), and the
            // note names the disagreeing garnish.
            match dates.iter().find(|d| **d != resolved) {
                Some(said) if *said == open => WeekVerdict::SameSunday,
                Some(said) => WeekVerdict::DifferentSunday(*said),
                None => WeekVerdict::DifferentSunday(resolved),
            }
        }
        // The same comparison for a bare relative
        // Sunday: its resolution is the open bulletin
        // itself, so an agreeing parenthesis reads as
        // the in-words verdict does, and a disagreeing
        // one names its date and arms. Round 9 (R9-10):
        // every stacked group is compared — all
        // agreeing reads as today, any disagreement
        // arms and names the disagreeing date.
        (None, Some(open)) if c.for_sunday_in_words.is_some() => {
            let dates = distinct_parenthesized_dates(c);
            if dates.iter().all(|d| *d == open) {
                WeekVerdict::SundayInWords(open)
            } else {
                let said = dates
                    .iter()
                    .find(|d| **d != open)
                    .expect("a disagreeing group stands");
                WeekVerdict::DifferentSunday(*said)
            }
        }
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

    fn sep(day: u32) -> SimpleDate {
        SimpleDate {
            year: 2026,
            month: 9,
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
    fn a_part_heading_with_a_parenthesized_suffix_is_that_headings_part() {
        // Round 9 (R9-8, red team RT9-8 verbatim): a
        // sender writes "## Sunday school (Room 2)" —
        // the very words the bulletin prints for that
        // section. The part is the Sunday school part,
        // not a plain announcement, and its heading line
        // is not swallowed: the part carries its lines
        // under the base heading. The same holds as a
        // second part, after an announcement.
        let text =
            "## Sunday school (Room 2)\n\nClasses meet at 9 AM.\n\nBring a friend to class.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.parts.len(), 1);
        assert_eq!(c.parts[0].kind, PartKind::SundaySchool);
        let lines: Vec<String> = c.parts[0].paragraphs.iter().map(|p| p.text()).collect();
        assert_eq!(
            lines,
            vec![
                "Classes meet at 9 AM.".to_string(),
                "Bring a friend to class.".to_string()
            ]
        );
        let text = "## Announcement\n\nThe bake sale is Friday.\n\n## Sunday school (Room 2)\n\nClasses meet at 9 AM.\n";
        let c = parse_contribution(text, today()).expect("parses");
        let kinds: Vec<PartKind> = c.parts.iter().map(|p| p.kind).collect();
        assert_eq!(kinds, vec![PartKind::Announcement, PartKind::SundaySchool]);
        // A longer heading is not a suffixed one: it
        // stays ordinary body text, never a part break.
        let text = "## Sunday school teachers\n\nOne.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.parts.len(), 1);
        assert_eq!(c.parts[0].kind, PartKind::Announcement);
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

    #[test]
    fn r8c_a_framed_date_reads_past_words_after_the_date() {
        // Round 8 (R8-4 / RT8-3): "use on Monday,
        // October 25, 2026 for the picnic" recorded
        // nothing — the framed reader parsed everything
        // after the framing weekday as one date, and any
        // word after the date killed the reading. The
        // framing needs the weekday directly in front of
        // the date; what follows the date is the line's
        // remainder. October 25, 2026 is a Sunday.
        for line in [
            "use on Monday, October 25, 2026 for the picnic",
            "please use on Monday, October 25, 2026, thank you",
            "on Monday, October 25, 2026 we celebrate",
        ] {
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, None, "{line}: the impossible day does not read");
            assert_eq!(
                week_verdict(&c, Some(oct(25))),
                WeekVerdict::NoSundayNamed,
                "{line}: no date reads, and the mismatch record speaks instead"
            );
            let m = c
                .named_weekday_mismatch
                .expect("the mismatch is recorded wherever the date stands");
            assert_eq!(m.named, "Monday", "{line}");
            assert_eq!(m.date, oct(25), "{line}");
            let text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, None, "{line}: the mirror drops it too");
            let m = c
                .keep_through_weekday_mismatch
                .expect("the keep-through mirror records it");
            assert_eq!(m.named, "Monday", "{line}");
            assert_eq!(m.date, oct(25), "{line}");
        }
        // The agreeing control (RT8-3's E2) died the
        // same way — the shape failed on the trailing
        // words, not the weekday. It now yields its
        // date by the same rules as the line-ending
        // form, on both lines.
        let text = "For Sunday: use on Sunday, October 25, 2026 for the picnic\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(oct(25)));
        assert_eq!(c.named_weekday_mismatch, None);
        assert_eq!(week_verdict(&c, Some(oct(25))), WeekVerdict::SameSunday);
        let text = "Keep-through: use on Sunday, October 25, 2026 for the picnic\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.keep_through, Some(oct(25)));
        assert_eq!(c.keep_through_weekday_mismatch, None);
        // The Sunday-after variant (RT8-3's E5) lost a
        // determined date to one trailing word the same
        // way: the Sunday after October 25, 2026 is
        // November 1.
        let nov1 = SimpleDate {
            year: 2026,
            month: 11,
            day: 1,
        };
        let text = "For Sunday: words for Sunday after October 25, 2026 please\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(nov1));
        assert_eq!(c.named_weekday_mismatch, None);
        let text = "Keep-through: words for Sunday after October 25, 2026 please\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.keep_through, Some(nov1));
        assert_eq!(c.keep_through_weekday_mismatch, None);
    }

    #[test]
    fn r8c_a_second_on_framing_is_judged_not_laundered() {
        // Round 8 (R8-5 / RT8-4): the framed reader
        // tried only the FIRST "on"+weekday candidate;
        // when it failed, the ordinary-text fallback
        // returned the second framed date clean —
        // SameSunday, unarmed, no mismatch. The reader
        // now scans every candidate until one frames a
        // date, and the fallback never returns a date
        // whose weekday wears the framing stack.
        // October 25, 2026 is a Sunday.
        let text = "For Sunday: meet on Friday, then use on Monday, October 25, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(
            c.for_date, None,
            "no longer a clean read of an impossible day"
        );
        assert_eq!(
            week_verdict(&c, Some(oct(25))),
            WeekVerdict::NoSundayNamed,
            "not the unarmed SameSunday the red team probed"
        );
        let m = c
            .named_weekday_mismatch
            .expect("the second framing is judged, not laundered");
        assert_eq!(m.named, "Monday");
        assert_eq!(m.date, oct(25));
        let text = "Keep-through: meet on Friday, then use on Monday, October 25, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.keep_through, None);
        let m = c
            .keep_through_weekday_mismatch
            .expect("the keep-through mirror records it too");
        assert_eq!(m.named, "Monday");
        assert_eq!(m.date, oct(25));
        // The shadow control (round 8 review, D4): a
        // candidate whose following words do not parse
        // as a date is not a framing — scanning
        // continues, and the real date reads clean.
        let text = "For Sunday: on Monday we prayed, service on Sunday, October 25, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(oct(25)), "D4 still reads clean");
        assert_eq!(c.named_weekday_mismatch, None, "D4 still records nothing");
        assert_eq!(week_verdict(&c, Some(oct(25))), WeekVerdict::SameSunday);
        let text = "Keep-through: on Monday we prayed, service on Sunday, October 25, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.keep_through, Some(oct(25)));
        assert_eq!(c.keep_through_weekday_mismatch, None);
    }

    #[test]
    fn r8c_on_the_weekday_frames_with_its_garnish_stack() {
        // Round 8 (R8-6 / RT8-5): the plain-date
        // readers required the weekday immediately
        // after "on", so "on the Monday, October 25,
        // 2026" read clean — SameSunday, unarmed —
        // though the Sunday-after reader strips
        // "on"/"the" garnishes in a loop. The framing
        // readers accept the same garnish stack,
        // wherever the framing stands. October 25, 2026
        // is a Sunday.
        let text = "For Sunday: on the Monday, October 25, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, None, "no longer an unarmed SameSunday");
        assert_eq!(
            week_verdict(&c, Some(oct(25))),
            WeekVerdict::NoSundayNamed,
            "not the unarmed SameSunday the red team probed"
        );
        let m = c
            .named_weekday_mismatch
            .expect("the garnished framing is judged");
        assert_eq!(m.named, "Monday");
        assert_eq!(m.date, oct(25));
        let text = "Keep-through: on the Monday, October 25, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.keep_through, None);
        let m = c
            .keep_through_weekday_mismatch
            .expect("the keep-through mirror records it too");
        assert_eq!(m.named, "Monday");
        assert_eq!(m.date, oct(25));
        // Wherever the framing stands in the line.
        let text = "For Sunday: use on the Monday, October 25, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, None);
        let m = c
            .named_weekday_mismatch
            .expect("mid-line, the stack still frames");
        assert_eq!(m.named, "Monday");
        assert_eq!(m.date, oct(25));
        // An agreeing weekday behind the stack reads
        // clean, exactly as "on Sunday" does.
        let text = "For Sunday: on the Sunday, October 25, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(oct(25)));
        assert_eq!(c.named_weekday_mismatch, None);
        assert_eq!(week_verdict(&c, Some(oct(25))), WeekVerdict::SameSunday);
    }

    #[test]
    fn r8c_a_parenthesized_date_on_a_relative_sunday_is_remembered_and_compared() {
        // Round 8 (carried shape, open two rounds /
        // RT8-10): "the Sunday after next Sunday
        // (October 25)" named no Sunday anywhere — the
        // trailing parenthesized absolute date was
        // remembered nowhere. It is now remembered with
        // the in-words phrase, and the verdict compares
        // it with the relative resolution once an open
        // bulletin anchors it. October 25, 2026 is a
        // Sunday; the Sunday after October 18 is
        // October 25, and after October 25, November 1.
        let nov8 = SimpleDate {
            year: 2026,
            month: 11,
            day: 8,
        };
        for line in [
            "the Sunday after next Sunday (October 25)",
            "the Sunday after next Sunday (October 25, 2026)",
        ] {
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, None, "{line}: no date in the words alone");
            assert_eq!(c.named_weekday_mismatch, None, "{line}");
            assert_eq!(
                c.for_sunday_after_in_words.as_deref(),
                Some(line),
                "{line}: the phrase is remembered as written"
            );
            assert_eq!(
                c.for_sunday_parenthesized_date,
                Some(oct(25)),
                "{line}: the parenthesized date is remembered with it"
            );
            // Anchored and agreeing (resolution October
            // 25 against the October 18 bulletin): reads
            // as the relative verdict does today.
            assert_eq!(
                week_verdict(&c, Some(oct(18))),
                WeekVerdict::DifferentSunday(oct(25)),
                "{line}: resolution and parenthesis agree"
            );
            // Anchored against October 25 (the
            // resolution is November 1): CHANGED IN
            // ROUND 9 (R9-11) — this assertion used to
            // expect DifferentSunday(October 25), armed,
            // with the review naming October 25 on both
            // sides. The parenthesized date is the
            // sender's explicit date and the stronger
            // signal: when it equals the open bulletin's
            // date the verdict is SameSunday, unarmed,
            // and the review notes the relative words
            // beside it (pinned in
            // `r9c_a_parenthesized_date_that_matches_the_bulletin_reads_same_sunday`).
            assert_eq!(
                week_verdict(&c, Some(oct(25))),
                WeekVerdict::SameSunday,
                "{line}: the parenthesized date is the bulletin's own Sunday"
            );
            // No anchor: the review's in-words sentence
            // carries the sender's phrase as now.
            assert_eq!(
                week_verdict(&c, None),
                WeekVerdict::NoSundayNamed,
                "{line}: no bulletin date to read against"
            );
        }
        // A parenthesis naming a third Sunday: the
        // verdict names it, not the resolution.
        let text =
            "For Sunday: the Sunday after next Sunday (November 8)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_sunday_parenthesized_date, Some(nov8));
        assert_eq!(
            week_verdict(&c, Some(oct(25))),
            WeekVerdict::DifferentSunday(nov8),
            "the resolution is November 1; the sender's date is named instead"
        );
        // The bare relative Sunday carries the same
        // memory: agreement reads as the in-words
        // verdict does today, disagreement names the
        // sender's date and arms like any differing
        // named Sunday.
        let text = "For Sunday: next Sunday (October 25)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(
            c.for_sunday_in_words.as_deref(),
            Some("next Sunday (October 25)")
        );
        assert_eq!(c.for_sunday_after_in_words, None);
        assert_eq!(c.for_sunday_parenthesized_date, Some(oct(25)));
        assert_eq!(
            week_verdict(&c, Some(oct(25))),
            WeekVerdict::SundayInWords(oct(25)),
            "agreeing: read as the open bulletin's Sunday, as today"
        );
        assert_eq!(
            week_verdict(&c, Some(oct(18))),
            WeekVerdict::DifferentSunday(oct(25)),
            "disagreeing: the named date, armed"
        );
        assert_eq!(week_verdict(&c, None), WeekVerdict::NoSundayNamed);
        // A parenthesis that does not read as a date at
        // all is garnish, not memory: "(Monday)" stands
        // beside the phrase as it always has (round 6,
        // RT6-8), and an unreadable date remembers
        // nothing either.
        let text = "For Sunday: next Sunday (Monday)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(
            c.for_sunday_in_words.as_deref(),
            Some("next Sunday (Monday)")
        );
        assert_eq!(c.for_sunday_parenthesized_date, None);
        assert_eq!(
            week_verdict(&c, Some(oct(25))),
            WeekVerdict::SundayInWords(oct(25))
        );
        let text = "For Sunday: next Sunday (October 32)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(
            c.for_sunday_in_words.as_deref(),
            Some("next Sunday (October 32)")
        );
        assert_eq!(c.for_sunday_parenthesized_date, None);
        assert_eq!(
            week_verdict(&c, Some(oct(25))),
            WeekVerdict::SundayInWords(oct(25))
        );
    }

    fn nov(day: u32) -> SimpleDate {
        SimpleDate {
            year: 2026,
            month: 11,
            day,
        }
    }

    #[test]
    fn r9c_keep_through_reads_a_trailing_parenthesized_date() {
        // Round 9 (R9-9, review R9-R4 + red team RT9-9
        // verbatim): "Keep through the Sunday after next
        // Sunday (November 8)" read nothing at all — the
        // sender's one absolute date was lost in silence.
        // The parenthesized date now stands as the
        // keep-through date when the line's own words
        // yield none. October 25, 2026 is a Sunday;
        // November 1 and November 8 are the Sundays
        // after it.
        let text =
            "Keep-through: the Sunday after next Sunday (November 8)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.keep_through, Some(nov(8)));
        assert_eq!(c.keep_through_parenthesized_mismatch, None);
        assert_eq!(c.keep_through_weekday_mismatch, None);
        let text = "Keep-through: the Sunday after next Sunday (November 8, 2026)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.keep_through, Some(nov(8)));
        // The review lane's second shape: a bare relative
        // phrase whose parenthesis alone is complete.
        let text = "Keep-through: next Sunday (November 1, 2026)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.keep_through, Some(nov(1)));
        assert_eq!(c.keep_through_parenthesized_mismatch, None);
        // Relative phrases on Keep-through otherwise stay
        // unread BY DESIGN (no anchor machinery,
        // recorded since round 6): no parenthesis, no
        // date, and nothing recorded.
        let text = "Keep-through: the Sunday after next Sunday\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.keep_through, None);
        assert_eq!(c.keep_through_parenthesized_mismatch, None);
        let text = "Keep-through: next Sunday\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.keep_through, None);
        // When the line yields a date AND a parenthesized
        // date, they compose as garnish: agreeing stands
        // clean, disagreeing records the keep-through
        // mismatch and the line's own date governs.
        let text = "Keep-through: November 8, 2026 (November 8)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.keep_through, Some(nov(8)));
        assert_eq!(c.keep_through_parenthesized_mismatch, None);
        let text = "Keep-through: November 8, 2026 (November 1)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.keep_through, Some(nov(8)));
        let m = c
            .keep_through_parenthesized_mismatch
            .expect("the disagreeing garnish is recorded");
        assert_eq!(m.read, nov(8));
        assert_eq!(m.named, nov(1));
        assert_eq!(
            m.keep_through_sentence(),
            "This part says keep through November 8, 2026, but the date in \
             parentheses is November 1, 2026. Check the date with the sender."
        );
        // Stacked groups on a relative Keep-through line
        // that disagree with each other: the last stands
        // and the earlier disagreement is recorded, not
        // laundered.
        let text =
            "Keep-through: next Sunday (November 1) (November 8)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.keep_through, Some(nov(8)));
        let m = c
            .keep_through_parenthesized_mismatch
            .expect("the stacked disagreement is recorded");
        assert_eq!(m.read, nov(8));
        assert_eq!(m.named, nov(1));
    }

    #[test]
    fn r9c_stacked_parentheses_are_all_compared() {
        // Round 9 (R9-10, red team RT9-10 verbatim):
        // stacked parentheses compared only the LAST
        // group, so a disagreeing earlier group was
        // laundered when the last agreed. Against the
        // October 25 bulletin, the bare shape
        // "(November 1) (October 25)" read clean.
        let text = "For Sunday: next Sunday (November 1) (October 25)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_sunday_parenthesized_dates, vec![nov(1), oct(25)]);
        assert_eq!(
            c.for_sunday_parenthesized_date,
            Some(oct(25)),
            "the round-8 single field stays the last group"
        );
        assert_eq!(
            week_verdict(&c, Some(oct(25))),
            WeekVerdict::DifferentSunday(nov(1)),
            "the earlier disagreeing group arms and is named"
        );
        // Both groups agreeing reads clean, exactly as a
        // single agreeing group does today.
        let text =
            "For Sunday: next Sunday (October 25) (October 25, 2026)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_sunday_parenthesized_dates, vec![oct(25), oct(25)]);
        assert_eq!(
            week_verdict(&c, Some(oct(25))),
            WeekVerdict::SundayInWords(oct(25))
        );
        // The Sunday-after mirror (RT9-10): "(October 25)
        // (November 1)" — the last group is the resolution
        // (November 1) and read clean, while the earlier
        // group disagreed with it.
        // CHANGED in round 10 (R10-7, flagged): this shape
        // used to expect DifferentSunday(October 25) —
        // a verdict carrying the open bulletin's own
        // date, the contract wart review MINOR-2 named.
        // The verdict is now SameSunday on the
        // bulletin-equal parenthesis, and the note (the
        // same-Sunday sentence, not the armed one) names
        // the resolution and both groups.
        let text = "For Sunday: the Sunday after next Sunday (October 25) (November 1)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_sunday_parenthesized_dates, vec![oct(25), nov(1)]);
        assert_eq!(
            week_verdict(&c, Some(oct(25))),
            WeekVerdict::SameSunday,
            "a DifferentSunday verdict never carries the bulletin's date"
        );
        let sentence = c
            .parenthesized_same_sunday_sentence(oct(25))
            .expect("the SameSunday note names the disagreeing garnish");
        assert_eq!(
            sentence,
            "These words say they are for Sunday, October 25, 2026, the date \
             written in parentheses. The words also name a Sunday in relative \
             words that would be Sunday, November 1, 2026. The dates in \
             parentheses do not agree with each other: Sunday, October 25, \
             2026 and Sunday, November 1, 2026."
        );
        // Both groups agreeing with the resolution reads
        // exactly as the single agreeing group does.
        let text = "For Sunday: the Sunday after next Sunday (November 1) (November 1, 2026)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_sunday_parenthesized_dates, vec![nov(1), nov(1)]);
        assert_eq!(
            week_verdict(&c, Some(oct(25))),
            WeekVerdict::DifferentSunday(nov(1))
        );
        assert_eq!(c.parenthesized_different_sentence(oct(25)), None);
        // Non-date groups are skipped in both orders, as
        // the round-8 probes held: the date group is the
        // only memory.
        for line in [
            "next Sunday (Room 2) (October 25)",
            "next Sunday (October 25) (Room 2)",
            "next Sunday (October 25) (Sunday)",
            "next Sunday (Sunday) (October 25)",
        ] {
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_sunday_parenthesized_dates, vec![oct(25)], "{line}");
            assert_eq!(
                week_verdict(&c, Some(oct(25))),
                WeekVerdict::SundayInWords(oct(25)),
                "{line}"
            );
        }
    }

    #[test]
    fn r9c_a_parenthesized_date_that_matches_the_bulletin_reads_same_sunday() {
        // Round 9 (R9-11, red team RT9-11 verbatim):
        // bulletin October 25; "the Sunday after next
        // Sunday (October 25)". The relative resolution
        // is November 1 and the parenthesis disagrees
        // with it, but the parenthesis IS the bulletin's
        // date: the old armed sentence named October 25
        // as both the file's Sunday and the bulletin's.
        // The parenthesized date is the sender's explicit
        // date and the stronger signal, so the verdict is
        // SameSunday, unarmed, and the review notes the
        // relative words in plain words.
        let text =
            "For Sunday: the Sunday after next Sunday (October 25)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(week_verdict(&c, Some(oct(25))), WeekVerdict::SameSunday);
        let sentence = c
            .parenthesized_same_sunday_sentence(oct(25))
            .expect("the note stands beside the SameSunday verdict");
        assert_eq!(
            sentence,
            "These words say they are for Sunday, October 25, 2026, the date \
             written in parentheses. The words also name a Sunday in relative \
             words that would be Sunday, November 1, 2026."
        );
        assert_eq!(c.parenthesized_different_sentence(oct(25)), None);
        // The other branch: the parenthesis matches
        // neither the resolution (November 1) nor the
        // bulletin (October 25). It still arms, and the
        // armed sentence names the resolution and the
        // parenthesized date — never one date on both
        // sides.
        let text =
            "For Sunday: the Sunday after next Sunday (November 8)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(
            week_verdict(&c, Some(oct(25))),
            WeekVerdict::DifferentSunday(nov(8))
        );
        let sentence = c
            .parenthesized_different_sentence(oct(25))
            .expect("the armed sentence names both dates");
        assert_eq!(
            sentence,
            "These words name a Sunday in relative words that would be \
             Sunday, November 1, 2026. The date in parentheses is Sunday, \
             November 8, 2026. The bulletin open is for Sunday, October 25, 2026."
        );
        assert_eq!(c.parenthesized_same_sunday_sentence(oct(25)), None);
        // The agreeing-with-resolution shape keeps the
        // ordinary armed sentence: no special sentence.
        let text =
            "For Sunday: the Sunday after next Sunday (November 1)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(
            week_verdict(&c, Some(oct(25))),
            WeekVerdict::DifferentSunday(nov(1))
        );
        assert_eq!(c.parenthesized_different_sentence(oct(25)), None);
        assert_eq!(c.parenthesized_same_sunday_sentence(oct(25)), None);
    }

    #[test]
    fn r9c_absolute_lines_compose_a_parenthesized_date_as_garnish() {
        // Round 9 (R9-12, red team RT9-12 verbatim): on
        // absolute lines a parenthesized date was never
        // examined — a plain date with one read as NO
        // date at all, and a Sunday-after line's
        // contradictory parenthesis was invisible.
        // The plain shape must yield its date.
        let text = "For Sunday: October 25, 2026 (November 1)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(oct(25)), "the plain date reads");
        assert_eq!(
            week_verdict(&c, Some(oct(25))),
            WeekVerdict::SameSunday,
            "the line's own date governs"
        );
        let m = c
            .for_sunday_parenthesized_mismatch
            .expect("the disagreeing parenthesis is recorded");
        assert_eq!(m.read, oct(25));
        assert_eq!(m.named, nov(1));
        assert_eq!(
            m.sentence(),
            "This part names October 25, 2026, but the date in parentheses is \
             November 1, 2026. Check the date with the sender."
        );
        // Agreeing stands clean.
        let text = "For Sunday: October 25, 2026 (October 25)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(oct(25)));
        assert_eq!(c.for_sunday_parenthesized_mismatch, None);
        assert_eq!(week_verdict(&c, Some(oct(25))), WeekVerdict::SameSunday);
        // The Keep-through mirror of the plain shape.
        let text = "Keep-through: October 25, 2026 (November 1)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.keep_through, Some(oct(25)));
        let m = c
            .keep_through_parenthesized_mismatch
            .expect("the keep-through mirror records it");
        assert_eq!(m.read, oct(25));
        assert_eq!(m.named, nov(1));
        // A Sunday-after line's parenthesis is compared
        // with the resolution (November 1): the
        // contradictory one is recorded, the agreeing
        // one stands clean — on both lines, where the
        // machinery is shared.
        let text =
            "For Sunday: Sunday after October 25, 2026 (November 8)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(nov(1)));
        assert_eq!(
            week_verdict(&c, Some(nov(1))),
            WeekVerdict::SameSunday,
            "against its own resolution the line is clean but for the garnish"
        );
        let m = c
            .for_sunday_parenthesized_mismatch
            .expect("the contradictory parenthesis is no longer invisible");
        assert_eq!(m.read, nov(1));
        assert_eq!(m.named, nov(8));
        let text =
            "For Sunday: Sunday after October 25, 2026 (November 1)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(nov(1)));
        assert_eq!(c.for_sunday_parenthesized_mismatch, None);
        let text =
            "Keep-through: Sunday after October 25, 2026 (November 8)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.keep_through, Some(nov(1)));
        let m = c
            .keep_through_parenthesized_mismatch
            .expect("the keep-through mirror records it too");
        assert_eq!(m.read, nov(1));
        assert_eq!(m.named, nov(8));
        let text =
            "Keep-through: Sunday after October 25, 2026 (November 1)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.keep_through, Some(nov(1)));
        assert_eq!(c.keep_through_parenthesized_mismatch, None);
        // The trailing weekday parenthesis control
        // stands as it always has: "(Monday)" is a
        // weekday garnish, judged as one, not a date.
        let text = "For Sunday: October 25, 2026 (Monday)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, None);
        assert!(c.named_weekday_mismatch.is_some());
        assert_eq!(c.for_sunday_parenthesized_mismatch, None);
    }

    #[test]
    fn r9c_a_second_date_in_a_date_line_is_noted_not_substituted() {
        // Round 9 (R9-13, red team RT9-13 verbatim): the
        // line concludes SameSunday on the framed date,
        // and the sender's operative second date was
        // invisible to the review. It is now noted in
        // plain words, without changing which date
        // governs.
        let line = "on Sunday, October 25, 2026 was when the fair was supposed to be, use November 1 instead";
        let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(c.for_date, Some(oct(25)), "the framed date still governs");
        assert_eq!(week_verdict(&c, Some(oct(25))), WeekVerdict::SameSunday);
        assert_eq!(c.for_sunday_second_date, Some(nov(1)));
        assert_eq!(
            second_date_sentence(nov(1)),
            "This file named another date too: Sunday, November 1, 2026. \
             Check which date was meant."
        );
        // The Keep-through mirror, run verbatim as the
        // red team ran it.
        let text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(c.keep_through, Some(oct(25)));
        assert_eq!(c.keep_through_second_date, Some(nov(1)));
        assert_eq!(
            keep_through_second_date_sentence(nov(1)),
            "The Keep-through line named another date too: Sunday, \
             November 1, 2026. Check which date was meant."
        );
        // Single-date lines note nothing: their review
        // sentences are unchanged.
        let text = "For Sunday: October 25, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_sunday_second_date, None);
        // A Sunday-after line's base is not a second
        // date: it is the date the read date resolves
        // from.
        let text = "For Sunday: Sunday after October 25, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(nov(1)));
        assert_eq!(c.for_sunday_second_date, None);
        // A parenthesized garnish is remembered on its
        // own record, not double-counted as a second
        // date.
        let text = "For Sunday: October 25, 2026 (November 1)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(oct(25)));
        assert_eq!(c.for_sunday_second_date, None);
        // The same date named twice is not another date.
        let text = "For Sunday: on Sunday, October 25, 2026, the same Sunday, October 25, 2026 as ever\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(oct(25)));
        assert_eq!(c.for_sunday_second_date, None);
    }

    #[test]
    fn r10b_a_second_date_before_the_read_date_is_noted() {
        // Round 10 (R10-4, red team RT10-D1 verbatim):
        // "Use November 1, not Sunday, October 25, 2026"
        // read October 25 with the November 1 standing
        // BEFORE it invisible — the second-date scan only
        // looked after the read date. The scan now reads
        // the whole line: the before-the-date correction
        // earns the same note, and the same arming, as
        // the after-the-date case pinned in round 9
        // (the read date governs; the verdict stays
        // SameSunday, unarmed, beside the note).
        let text =
            "For Sunday: Use November 1, not Sunday, October 25, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(oct(25)), "the framed date still governs");
        assert_eq!(
            week_verdict(&c, Some(oct(25))),
            WeekVerdict::SameSunday,
            "armed exactly as the after-the-date case: not at all"
        );
        assert_eq!(c.for_sunday_second_date, Some(nov(1)));
        assert_eq!(c.for_sunday_second_dates, vec![nov(1)]);
        assert_eq!(
            second_dates_sentence(&c.for_sunday_second_dates),
            "This file named another date too: Sunday, November 1, 2026. \
             Check which date was meant."
        );
        // The Keep-through mirror.
        let text =
            "Keep-through: Use November 1, not Sunday, October 25, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.keep_through, Some(oct(25)));
        assert_eq!(c.keep_through_second_date, Some(nov(1)));
        assert_eq!(c.keep_through_second_dates, vec![nov(1)]);
        // Its twin is indistinguishable to the machinery,
        // as the red team recorded: the framed date is
        // followed and the other date is noted either way.
        let text = "For Sunday: The fair moved from November 1 to Sunday, October 25, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(oct(25)));
        assert_eq!(c.for_sunday_second_dates, vec![nov(1)]);
    }

    #[test]
    fn r10b_a_wordy_trailing_group_contributes_its_date() {
        // Round 10 (R10-5, red team RT10-D2 verbatim): a
        // trailing group with words in it used to poison
        // the whole line — no date read at all. A group
        // containing exactly one absolute date now
        // contributes it as a parenthesized date for the
        // existing composition.
        for line in [
            "October 25, 2026 (correction: November 1)",
            "October 25, 2026 (use November 1 instead)",
        ] {
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, Some(oct(25)), "{line}: the line date reads");
            assert_eq!(
                week_verdict(&c, Some(oct(25))),
                WeekVerdict::SameSunday,
                "{line}"
            );
            let m = c
                .for_sunday_parenthesized_mismatch
                .expect("the contained date is named as a parenthesized mismatch");
            assert_eq!(m.read, oct(25), "{line}");
            assert_eq!(m.named, nov(1), "{line}");
            assert_eq!(
                m.sentence(),
                "This part names October 25, 2026, but the date in parentheses is \
                 November 1, 2026. Check the date with the sender.",
                "{line}: named exactly as a bare-date group is"
            );
            assert_eq!(
                c.for_sunday_second_date, None,
                "{line}: the garnish is not double-counted as a second date"
            );
        }
        // The Keep-through mirror of the correction shape.
        let text =
            "Keep-through: October 25, 2026 (correction: November 1)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.keep_through, Some(oct(25)));
        let m = c
            .keep_through_parenthesized_mismatch
            .expect("the keep-through mirror records it too");
        assert_eq!(m.named, nov(1));
        // FLAGGED PIN CHANGE (round 11, R11-6): both
        // blocks below used to expect NoSundayNamed —
        // the non-contributing group poisoned the plain
        // reading it trailed, which is the defect
        // RT11-6 names. A group that contributes nothing
        // now behaves as if absent: the plain date
        // reads and stands, and a two-date group's
        // dates are named as alternatives on the
        // second-date note. The groups themselves still
        // contribute nothing, as they always have.
        let text = "For Sunday: October 25, 2026 (Room 2)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(
            c.for_date,
            Some(oct(25)),
            "the no-date group no longer poisons the line"
        );
        assert_eq!(c.for_sunday_parenthesized_mismatch, None);
        assert_eq!(c.for_sunday_second_dates, Vec::<SimpleDate>::new());
        assert_eq!(week_verdict(&c, Some(oct(25))), WeekVerdict::SameSunday);
        // A trailing group with two dates in it
        // contributes nothing either — but the dates
        // are named as alternatives, not laundered.
        let text =
            "For Sunday: October 25, 2026 (November 1 and November 8)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(
            c.for_date,
            Some(oct(25)),
            "the multi-date group no longer poisons the line"
        );
        assert_eq!(c.for_sunday_parenthesized_mismatch, None);
        assert_eq!(c.for_sunday_second_dates, vec![nov(1), nov(8)]);
        assert_eq!(week_verdict(&c, Some(oct(25))), WeekVerdict::SameSunday);
    }

    #[test]
    fn r10b_every_disagreeing_date_is_named_on_every_shape() {
        // Round 10 (R10-6, review MINOR-1 + red team
        // RT10-D3): outside the Sunday-after shape only
        // one disagreeing date was ever named, and the
        // Keep-through stacked sentence misdescribed
        // where the kept date came from.
        // Keep-through relative with three groups: the
        // last stands, and BOTH earlier disagreeing
        // groups are named; the sentence says the kept
        // date came from the parentheses.
        let text =
            "Keep-through: next Sunday (November 1) (November 8) (November 15)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.keep_through, Some(nov(15)));
        let m = c
            .keep_through_parenthesized_mismatch
            .expect("the stacked disagreement is recorded");
        assert_eq!(m.read, nov(15));
        assert_eq!(m.named, nov(1));
        assert_eq!(m.also_named, vec![nov(8)], "November 8 is named too");
        assert_eq!(
            m.keep_through_sentence(),
            "The dates in parentheses do not agree with each other: \
             November 1, 2026 and November 8, 2026 and November 15, 2026. \
             This part is read as keep through November 15, 2026. \
             Check the date with the sender."
        );
        // The review lane's MINOR-1 witness: both dates
        // were in parentheses, and the sentence now says
        // so instead of framing one as a plain reading.
        let text =
            "Keep-through: next Sunday (November 1) (November 8)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.keep_through, Some(nov(8)));
        let m = c
            .keep_through_parenthesized_mismatch
            .expect("the stacked disagreement is recorded");
        assert_eq!(
            m.keep_through_sentence(),
            "The dates in parentheses do not agree with each other: \
             November 1, 2026 and November 8, 2026. \
             This part is read as keep through November 8, 2026. \
             Check the date with the sender."
        );
        // Absolute Keep-through lines with two groups,
        // both orders: every disagreeing group is named.
        let text =
            "Keep-through: November 8, 2026 (November 1) (November 15)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.keep_through, Some(nov(8)));
        let m = c
            .keep_through_parenthesized_mismatch
            .expect("both disagreeing groups are recorded");
        assert_eq!(m.named, nov(1));
        assert_eq!(m.also_named, vec![nov(15)]);
        assert_eq!(
            m.keep_through_sentence(),
            "This part says keep through November 8, 2026, but the dates in \
             parentheses are November 1, 2026 and November 15, 2026. \
             Check the date with the sender."
        );
        let text =
            "Keep-through: November 8, 2026 (November 15) (November 1)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        let m = c
            .keep_through_parenthesized_mismatch
            .expect("both disagreeing groups are recorded, in written order");
        assert_eq!(m.named, nov(15));
        assert_eq!(m.also_named, vec![nov(1)]);
        // Absolute For Sunday line with two groups.
        let text =
            "For Sunday: October 25, 2026 (November 1) (November 8)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(oct(25)));
        let m = c
            .for_sunday_parenthesized_mismatch
            .expect("both disagreeing groups are recorded");
        assert_eq!(m.named, nov(1));
        assert_eq!(m.also_named, vec![nov(8)]);
        assert_eq!(
            m.sentence(),
            "This part names October 25, 2026, but the dates in parentheses are \
             November 1, 2026 and November 8, 2026. \
             Check the date with the sender."
        );
        // A middle group that agrees with the standing
        // date does not hide the disagreeing groups
        // around it: every distinct disagreeing date is
        // still named.
        let text = "For Sunday: October 25, 2026 (November 1) (October 25) (November 8)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        let m = c
            .for_sunday_parenthesized_mismatch
            .expect("the disagreeing groups are recorded");
        assert_eq!(m.named, nov(1));
        assert_eq!(m.also_named, vec![nov(8)]);
        // The bare relative shape gains the listing
        // sentence: every group is named, and the
        // bulletin is named.
        let text =
            "For Sunday: next Sunday (November 8) (November 15) (October 25)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(
            week_verdict(&c, Some(oct(25))),
            WeekVerdict::DifferentSunday(nov(8))
        );
        let sentence = c
            .parenthesized_different_sentence(oct(25))
            .expect("the bare shape gains the listing sentence");
        assert_eq!(
            sentence,
            "These words name a Sunday in relative words that would be \
             Sunday, October 25, 2026. The dates in parentheses do not agree \
             with each other: Sunday, November 8, 2026 and Sunday, November 15, \
             2026 and Sunday, October 25, 2026. The bulletin open is for \
             Sunday, October 25, 2026."
        );
        // Two second dates: the note names both.
        let line = "on Sunday, October 25, 2026 was when the fair was supposed to be, use November 1 instead, or November 8 instead";
        let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(c.for_date, Some(oct(25)));
        assert_eq!(c.for_sunday_second_date, Some(nov(1)));
        assert_eq!(c.for_sunday_second_dates, vec![nov(1), nov(8)]);
        assert_eq!(
            second_dates_sentence(&c.for_sunday_second_dates),
            "This file named other dates too: Sunday, November 1, 2026 and \
             Sunday, November 8, 2026. Check which date was meant."
        );
        assert_eq!(
            week_verdict(&c, Some(oct(25))),
            WeekVerdict::SameSunday,
            "the note still changes no verdict"
        );
    }

    #[test]
    fn r10b_a_different_sunday_verdict_never_carries_the_bulletins_date() {
        // Round 10 (R10-7, review MINOR-2): under
        // stacked-parenthesis composition week_verdict
        // could return DifferentSunday(d) where d IS
        // the open bulletin's date. The contract is
        // pinned over the stacked shapes wholesale:
        // for every witness, DifferentSunday(d) implies
        // d != the bulletin's date.
        // The review lane's witness reads SameSunday
        // with a note naming the disagreeing garnish.
        let text = "For Sunday: the Sunday after next Sunday (October 25) (November 8)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(week_verdict(&c, Some(oct(25))), WeekVerdict::SameSunday);
        let sentence = c
            .parenthesized_same_sunday_sentence(oct(25))
            .expect("the SameSunday note stands");
        assert!(
            sentence.contains("Sunday, November 8, 2026"),
            "the disagreeing garnish is named in the note: {sentence}"
        );
        // The wholesale pin over the stacked shapes,
        // both group orders and both bulletins.
        let shapes: [(&str, SimpleDate); 10] = [
            (
                "the Sunday after next Sunday (October 25) (November 8)",
                oct(25),
            ),
            (
                "the Sunday after next Sunday (October 25) (November 1)",
                oct(25),
            ),
            (
                "the Sunday after next Sunday (November 8) (October 25)",
                oct(25),
            ),
            (
                "the Sunday after next Sunday (November 1) (November 8)",
                oct(25),
            ),
            ("the Sunday after next Sunday (October 25)", oct(25)),
            ("the Sunday after next Sunday (November 8)", oct(25)),
            (
                "the Sunday after next Sunday (November 1) (November 8)",
                nov(1),
            ),
            ("next Sunday (November 1) (October 25)", oct(25)),
            ("next Sunday (October 25) (November 1)", oct(25)),
            ("next Sunday (November 8) (November 15)", oct(25)),
        ];
        for (line, bulletin) in shapes {
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            if let WeekVerdict::DifferentSunday(d) = week_verdict(&c, Some(bulletin)) {
                assert_ne!(
                    d,
                    bulletin,
                    "{line} against {}: DifferentSunday never carries the bulletin's date",
                    format_date_long(bulletin)
                );
            }
        }
    }

    #[test]
    fn r10b_paren_only_for_sunday_parity_and_a_mid_date_group() {
        // Round 10 (R10-10, red team RT10-D4): a
        // paren-only For Sunday line read NoSundayNamed
        // against every bulletin while the identical
        // Keep-through line read its date fine.
        let text = "For Sunday: (October 25)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(oct(25)), "the paren-only date reads");
        assert_eq!(c.for_sunday_parenthesized_mismatch, None);
        assert_eq!(
            week_verdict(&c, Some(oct(25))),
            WeekVerdict::SameSunday,
            "composed by the existing rules, as Keep-through is"
        );
        assert_eq!(
            week_verdict(&c, Some(nov(1))),
            WeekVerdict::DifferentSunday(oct(25))
        );
        let text = "Keep-through: (October 25)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.keep_through, Some(oct(25)), "the parity shape itself");
        assert_eq!(c.keep_through_parenthesized_mismatch, None);
        // Stacked paren-only groups: the last stands and
        // the earlier disagreement is recorded, its
        // source stated as the parentheses.
        let text = "For Sunday: (October 25) (November 1)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(nov(1)));
        let m = c
            .for_sunday_parenthesized_mismatch
            .expect("the stacked paren-only disagreement is recorded");
        assert_eq!(m.read, nov(1));
        assert_eq!(m.named, oct(25));
        assert_eq!(
            m.sentence(),
            "The dates in parentheses do not agree with each other: \
             October 25, 2026 and November 1, 2026. \
             This part is read as naming November 1, 2026. \
             Check the date with the sender."
        );
        // A group standing between a month-day and its
        // year does not poison the plain date: it reads
        // October 25, 2026 with the November 1 group
        // compared as garnish, on both lines.
        let text = "For Sunday: October 25 (November 1), 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(oct(25)));
        assert_eq!(week_verdict(&c, Some(oct(25))), WeekVerdict::SameSunday);
        let m = c
            .for_sunday_parenthesized_mismatch
            .expect("the mid-date group is compared as garnish");
        assert_eq!(m.read, oct(25));
        assert_eq!(m.named, nov(1));
        assert_eq!(
            c.for_sunday_second_date, None,
            "the garnish is not double-counted as a second date"
        );
        let text = "Keep-through: October 25 (November 1), 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.keep_through, Some(oct(25)));
        let m = c
            .keep_through_parenthesized_mismatch
            .expect("the keep-through mirror compares it too");
        assert_eq!(m.named, nov(1));
        // The accepted boundary stands (R10-10c): a
        // group followed by other words, or standing
        // before the phrase, stays unread.
        let text = "For Sunday: next Sunday (October 25) please\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, None);
        assert_eq!(week_verdict(&c, Some(oct(25))), WeekVerdict::NoSundayNamed);
        let text = "For Sunday: (October 25) next Sunday\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, None);
        assert_eq!(week_verdict(&c, Some(oct(25))), WeekVerdict::NoSundayNamed);
    }

    #[test]
    fn r11c_negated_and_origin_groups_contribute_nothing() {
        // Round 11 (R11-4, red team RT11-5 verbatim): a
        // contained date governed by a negation or an
        // origin marker standing before it inside its
        // group does not contribute, on every consuming
        // path. November 1, 2026 is a Sunday.
        for group in [
            "(not November 1)",
            "(ignore November 1)",
            "(moved from November 1)",
            "(from November 1)",
            "(never November 1)",
            "(no longer November 1)",
            "(don't use November 1)",
            "(didn't use November 1)",
            "(originally November 1)",
        ] {
            let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            let expected_words = format!("next Sunday {group}");
            assert_eq!(
                c.for_sunday_in_words.as_deref(),
                Some(expected_words.as_str()),
                "{group}: the relative words still stand"
            );
            assert_eq!(c.for_sunday_parenthesized_date, None, "{group}");
            assert_eq!(
                c.for_sunday_parenthesized_dates,
                Vec::<SimpleDate>::new(),
                "{group}: the governed date is not remembered"
            );
            assert_eq!(
                week_verdict(&c, Some(oct(25))),
                WeekVerdict::SundayInWords(oct(25)),
                "{group}: no arming on the governed date"
            );
            // Paren-only lines: the governed date never
            // becomes the operative date.
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, None, "{group}: paren-only");
            assert_eq!(c.for_sunday_parenthesized_mismatch, None, "{group}");
            assert_eq!(
                week_verdict(&c, Some(nov(1))),
                WeekVerdict::NoSundayNamed,
                "{group}: never SameSunday on a negated date"
            );
            // Keep-through lines: trailing governed
            // groups and the fallback alike set nothing.
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, None, "{group}: keep-through paren-only");
            let text = format!("Keep-through: choir anthem {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, None, "{group}: keep-through fallback");
            assert_eq!(c.keep_through_parenthesized_mismatch, None, "{group}");
        }
        // The review lane's origin witness through the
        // Keep-through fallback (R11-6 in their numbering).
        let text =
            "Keep-through: choir anthem (from November 1 service)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(
            c.keep_through, None,
            "an origin mention sets no keep-through"
        );
        // The Sunday-after relative shape: the sentence
        // never states a negated date as the sender's.
        let text =
            "For Sunday: the Sunday after next Sunday (not November 1)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(
            week_verdict(&c, Some(nov(1))),
            WeekVerdict::DifferentSunday(nov(8)),
            "resolved against November 1, not SameSunday on the negated group"
        );
        assert_eq!(c.parenthesized_same_sunday_sentence(nov(1)), None);
        // Absolute composition: the plain date stands,
        // no mismatch is recorded on the governed date.
        let text = "For Sunday: October 25, 2026 (not November 1)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(oct(25)));
        assert_eq!(c.for_sunday_parenthesized_mismatch, None);
        assert_eq!(week_verdict(&c, Some(oct(25))), WeekVerdict::SameSunday);
        let text = "Keep-through: November 8, 2026 (ignore November 1)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.keep_through, Some(nov(8)));
        assert_eq!(c.keep_through_parenthesized_mismatch, None);
        // Held shapes, unchanged: "(moved to ...)" is
        // an assertion and arms; "(correction: ...)"
        // contributes; a group restating the line's own
        // date records no mismatch.
        let text = "For Sunday: next Sunday (moved to November 1)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_sunday_parenthesized_date, Some(nov(1)));
        assert_eq!(
            week_verdict(&c, Some(oct(25))),
            WeekVerdict::DifferentSunday(nov(1))
        );
        let text =
            "For Sunday: October 25, 2026 (correction: November 1)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        let m = c
            .for_sunday_parenthesized_mismatch
            .expect("the correction shape still contributes");
        assert_eq!(m.named, nov(1));
        let text = "For Sunday: October 25, 2026 (service on October 25, 2026)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(oct(25)));
        assert_eq!(c.for_sunday_parenthesized_mismatch, None);
        assert_eq!(week_verdict(&c, Some(oct(25))), WeekVerdict::SameSunday);
    }

    #[test]
    fn r11c_non_contributing_groups_name_alternatives_and_never_poison() {
        // Round 11 (R11-6, red team RT11-6 + review
        // R11-7): a group that contributes nothing never
        // takes the line's reading with it, and offered
        // alternatives are named on the second-date
        // note, not laundered into a clean reading.
        let text =
            "For Sunday: October 25, 2026 (November 1 or November 8)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(oct(25)), "the plain date stands");
        assert_eq!(week_verdict(&c, Some(oct(25))), WeekVerdict::SameSunday);
        assert_eq!(c.for_sunday_parenthesized_mismatch, None);
        assert_eq!(c.for_sunday_second_dates, vec![nov(1), nov(8)]);
        assert_eq!(
            second_dates_sentence(&c.for_sunday_second_dates),
            "This file named other dates too: Sunday, November 1, 2026 and \
             Sunday, November 8, 2026. Check which date was meant."
        );
        // The relative mirror: the words resolve, the
        // alternatives are named beside the resolution,
        // and no group feeds the verdict.
        let text = "For Sunday: next Sunday (November 1 or November 8)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(
            c.for_sunday_in_words.as_deref(),
            Some("next Sunday (November 1 or November 8)")
        );
        assert_eq!(c.for_sunday_parenthesized_dates, Vec::<SimpleDate>::new());
        assert_eq!(
            week_verdict(&c, Some(oct(25))),
            WeekVerdict::SundayInWords(oct(25)),
            "resolved on the words, not armed on an alternative"
        );
        assert_eq!(
            c.for_sunday_second_dates,
            vec![nov(1), nov(8)],
            "the alternatives are named, not laundered into a clean acceptance"
        );
        // Nested groups kill neither reading.
        // FLAGGED PIN CHANGE (round 12, R12-7b): the
        // relative block below used to expect
        // SundayInWords — the nested group's date was
        // invisible inside its inner parentheses,
        // which is the defect RT12-5 names. The group
        // content reading now sees through balanced
        // inner parentheses, exactly as the flat
        // "(note: November 1)" control has always
        // contributed, so November 1 is the group's
        // date: it is compared as garnish on the
        // absolute line and remembered — and armed —
        // on the relative one.
        let text = "For Sunday: October 25, 2026 (note (November 1))\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(oct(25)));
        assert_eq!(week_verdict(&c, Some(oct(25))), WeekVerdict::SameSunday);
        let m = c
            .for_sunday_parenthesized_mismatch
            .expect("the nested group's date is compared as garnish");
        assert_eq!(m.named, nov(1));
        let text = "For Sunday: next Sunday (note (November 1))\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(
            c.for_sunday_in_words.as_deref(),
            Some("next Sunday (note (November 1))")
        );
        assert_eq!(c.for_sunday_parenthesized_date, Some(nov(1)));
        assert_eq!(
            week_verdict(&c, Some(oct(25))),
            WeekVerdict::DifferentSunday(nov(1))
        );
        // "Or the 8th" is an offered alternative, not a
        // promotion of November 1 alone to a certainty.
        let text = "For Sunday: next Sunday (November 1 or the 8th)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_sunday_parenthesized_dates, Vec::<SimpleDate>::new());
        assert_eq!(
            week_verdict(&c, Some(oct(25))),
            WeekVerdict::SundayInWords(oct(25)),
            "November 1 alone no longer arms"
        );
        assert_eq!(c.for_sunday_second_dates, vec![nov(1), nov(8)]);
        let text =
            "For Sunday: October 25, 2026 (November 1 or the 8th)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(oct(25)));
        assert_eq!(c.for_sunday_second_dates, vec![nov(1), nov(8)]);
        // A mid-date group plus a trailing group
        // together (review R11-7): the plain date reads,
        // and both groups are compared as garnish.
        let text =
            "For Sunday: October 25 (November 1), 2026 (November 8)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(oct(25)));
        assert_eq!(week_verdict(&c, Some(oct(25))), WeekVerdict::SameSunday);
        let m = c
            .for_sunday_parenthesized_mismatch
            .expect("both groups are named as garnish");
        assert_eq!(m.named, nov(1));
        assert_eq!(m.also_named, vec![nov(8)]);
    }

    #[test]
    fn r11c_the_second_date_anchor_is_the_read_dates_own_occurrence() {
        // Round 11 (R11-7, review R11-3 + red team
        // RT11-3): "October 20, then Sunday, October 25,
        // 2026" notes October 20 — a date in the six
        // days before the read date is a second date,
        // not the read date's anchor by proxy.
        let text =
            "For Sunday: October 20, then Sunday, October 25, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(oct(25)));
        assert_eq!(c.for_sunday_second_dates, vec![oct(20)]);
        assert_eq!(
            second_dates_sentence(&c.for_sunday_second_dates),
            "This file named another date too: Tuesday, October 20, 2026. \
             Check which date was meant."
        );
        let text = "For Sunday: October 20, 2026 planning day; service is Sunday, October 25, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(oct(25)));
        assert_eq!(c.for_sunday_second_dates, vec![oct(20)]);
        // The Keep-through mirror.
        let text = "Keep-through: October 20, 2026 planning day; service is Sunday, October 25, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.keep_through, Some(oct(25)));
        assert_eq!(c.keep_through_second_dates, vec![oct(20)]);
        // On a Sunday-after line the resolution's base
        // is never flagged as its own second date...
        let text = "For Sunday: the Sunday after October 20, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(oct(25)));
        assert_eq!(c.for_sunday_second_dates, Vec::<SimpleDate>::new());
        // ...and a stray date standing before the phrase
        // is the date noted, not the base.
        let text = "For Sunday: October 22, 2026 social; the Sunday after October 20, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(oct(25)));
        assert_eq!(c.for_sunday_second_dates, vec![oct(22)]);
    }

    #[test]
    fn r11c_a_contradicting_weekday_inside_a_wordy_group_contributes_nothing() {
        // Round 11 (R11-8, review R11-4 + red team
        // RT11-6 tail): November 1, 2026 is a Sunday, so
        // a group naming it a Monday contributes
        // nothing — the weekday law inside groups.
        let text = "For Sunday: October 25, 2026 (Monday November 1)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(oct(25)));
        assert_eq!(c.for_sunday_parenthesized_mismatch, None);
        assert_eq!(week_verdict(&c, Some(oct(25))), WeekVerdict::SameSunday);
        let text =
            "For Sunday: October 25, 2026 (correction: Monday, November 1)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(oct(25)));
        assert_eq!(c.for_sunday_parenthesized_mismatch, None);
        // A paren-only line governed by such a group
        // reads no date at all.
        for line in ["(Monday November 1)", "(correction: Monday, November 1)"] {
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, None, "{line}");
            assert_eq!(c.for_sunday_parenthesized_mismatch, None, "{line}");
            assert_eq!(
                week_verdict(&c, Some(nov(1))),
                WeekVerdict::NoSundayNamed,
                "{line}: no governing on a weekday-contradicted date"
            );
        }
        // An agreeing weekday inside the group leaves
        // the contribution standing.
        let text =
            "For Sunday: October 25, 2026 (correction: Sunday, November 1)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        let m = c
            .for_sunday_parenthesized_mismatch
            .expect("an agreeing weekday still contributes");
        assert_eq!(m.named, nov(1));
    }

    #[test]
    fn r12a_governor_families_completed_on_every_path() {
        // Round 12 (R12-1, red team RT12-1 + review
        // R12-3): the R11 governor list missed the
        // ordinary words an office actually writes,
        // and every miss reproduced the full R11-4
        // harm — the disowned date armed a verdict,
        // blessed SameSunday, or set keep-through.
        // The families are completed: negation /
        // cancellation (skip, except, rather than,
        // instead of, cancelled/canceled, bare no,
        // isn't family, never mind) and origin / past
        // (previously, formerly, used to be, was, old
        // date), on top of the R11 set. Each phrasing
        // is pinned on every consuming path.
        for phrase in [
            "skip",
            "except",
            "rather than",
            "instead of",
            "cancelled",
            "canceled:",
            "no",
            "isn't",
            "aren't",
            "wasn't",
            "weren't",
            "never mind",
            "previously",
            "formerly",
            "used to be",
            "was",
            "old date:",
        ] {
            let group = format!("({phrase} November 1)");
            // Relative path: the governed date is not
            // remembered, and nothing arms on it.
            let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            let expected_words = format!("next Sunday {group}");
            assert_eq!(
                c.for_sunday_in_words.as_deref(),
                Some(expected_words.as_str()),
                "{group}: the relative words still stand"
            );
            assert_eq!(c.for_sunday_parenthesized_date, None, "{group}");
            assert_eq!(
                c.for_sunday_parenthesized_dates,
                Vec::<SimpleDate>::new(),
                "{group}: the governed date is not remembered"
            );
            assert_eq!(
                week_verdict(&c, Some(oct(25))),
                WeekVerdict::SundayInWords(oct(25)),
                "{group}: no arming on the governed date"
            );
            // Paren-only path: the governed date never
            // becomes the operative date.
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, None, "{group}: paren-only");
            assert_eq!(c.for_sunday_parenthesized_mismatch, None, "{group}");
            assert_eq!(
                week_verdict(&c, Some(nov(1))),
                WeekVerdict::NoSundayNamed,
                "{group}: never SameSunday on a governed date"
            );
            // Keep-through paths: trailing governed
            // groups and the fallback alike set
            // nothing.
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, None, "{group}: keep-through paren-only");
            let text = format!("Keep-through: choir anthem {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, None, "{group}: keep-through fallback");
            assert_eq!(c.keep_through_parenthesized_mismatch, None, "{group}");
            // Absolute composition: the plain date
            // stands, and no mismatch is recorded on
            // the governed date.
            let text = format!("For Sunday: October 25, 2026 {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, Some(oct(25)), "{group}");
            assert_eq!(c.for_sunday_parenthesized_mismatch, None, "{group}");
            assert_eq!(
                week_verdict(&c, Some(oct(25))),
                WeekVerdict::SameSunday,
                "{group}"
            );
        }
        // A governed date trailing a line's own date
        // names no mismatch either (RT12-1's trailing
        // form, with a completed-family word).
        let text = "For Sunday: November 1, 2026 (skip November 8)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(nov(1)));
        assert_eq!(c.for_sunday_parenthesized_mismatch, None);
        assert_eq!(week_verdict(&c, Some(nov(1))), WeekVerdict::SameSunday);
        // Held care with the common words: "was" and
        // "no" govern only in the governing clause.
        // A group merely carrying them in an earlier
        // clause still contributes its date — the
        // governor was spent on that clause's own
        // words, not on the date in the fresh clause
        // after the comma.
        for group in [
            "(choir was singing, November 1)",
            "(no rehearsal, November 1)",
            "(it was moved, November 1)",
        ] {
            let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date,
                Some(nov(1)),
                "{group}: a non-governing \"was\"/\"no\" does not disqualify"
            );
        }
    }

    #[test]
    fn r13a_contraction_class_and_wrong_label_on_every_path() {
        // Round 13 (R13-1, review R13-1 + red team
        // RT13-1): the governor families missed the
        // contraction class, and "(won't be November
        // 1)" / "(wrong: November 1)" reproduced the
        // full R11-4 harm on every consuming path.
        // Negation is now a class: any token ending
        // in "n't", plus unpunctuated "wont" and
        // "cannot", governs — the rule a reader
        // applies, not another word list.
        for phrase in [
            "won't be",
            "wont be",
            "cannot be",
            "can't be",
            "shouldn't be",
            "couldn't be",
        ] {
            let group = format!("({phrase} November 1)");
            let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date, None,
                "{group}: the contraction class governs on the relative path"
            );
            assert_eq!(
                c.for_sunday_parenthesized_dates,
                Vec::<SimpleDate>::new(),
                "{group}: the governed date is not remembered"
            );
            assert_eq!(
                week_verdict(&c, Some(oct(25))),
                WeekVerdict::SundayInWords(oct(25)),
                "{group}: no arming on the governed date"
            );
            assert_eq!(
                week_verdict(&c, Some(nov(1))),
                WeekVerdict::SundayInWords(nov(1)),
                "{group}: the words resolve against the bulletin, \
                 never DifferentSunday on the governed date"
            );
        }
        // Held controls: the spelled-out forms the
        // bare "not" token has always caught still
        // govern.
        for phrase in ["will not be", "must not be", "is not"] {
            let group = format!("({phrase} November 1)");
            let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date, None,
                "{group}: the spelled-out negation still governs"
            );
        }
        // "won't" and the "wrong:" label are pinned
        // on every consuming path, the way the R12
        // families are.
        for phrase in ["won't be", "wrong:"] {
            let group = format!("({phrase} November 1)");
            let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            let expected_words = format!("next Sunday {group}");
            assert_eq!(
                c.for_sunday_in_words.as_deref(),
                Some(expected_words.as_str()),
                "{group}: the relative words still stand"
            );
            assert_eq!(c.for_sunday_parenthesized_date, None, "{group}");
            assert_eq!(
                week_verdict(&c, Some(oct(25))),
                WeekVerdict::SundayInWords(oct(25)),
                "{group}: no arming on the governed date"
            );
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, None, "{group}: paren-only");
            assert_eq!(c.for_sunday_parenthesized_mismatch, None, "{group}");
            assert_eq!(
                week_verdict(&c, Some(nov(1))),
                WeekVerdict::NoSundayNamed,
                "{group}: never SameSunday on a governed date"
            );
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, None, "{group}: keep-through paren-only");
            let text = format!("Keep-through: choir anthem {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, None, "{group}: keep-through fallback");
            assert_eq!(c.keep_through_parenthesized_mismatch, None, "{group}");
            let text = format!("For Sunday: October 25, 2026 {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, Some(oct(25)), "{group}");
            assert_eq!(c.for_sunday_parenthesized_mismatch, None, "{group}");
            assert_eq!(
                week_verdict(&c, Some(oct(25))),
                WeekVerdict::SameSunday,
                "{group}"
            );
        }
    }

    #[test]
    fn r13a_cancellation_verbs_disqualify() {
        // Round 13 (R13-1, red team RT13-1): the
        // cancellation family held only the past
        // forms cancelled/canceled; the verbs an
        // office actually writes — cancel, scratch,
        // drop, postpone, remove, hold off — governed
        // nothing, and every miss set the dropped
        // date as keep-through or armed it.
        for phrase in [
            "cancel", "scratch", "drop", "postpone", "remove", "hold off",
        ] {
            let group = format!("({phrase} November 1)");
            let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date, None,
                "{group}: the cancellation verb governs on the relative path"
            );
            assert_eq!(
                week_verdict(&c, Some(oct(25))),
                WeekVerdict::SundayInWords(oct(25)),
                "{group}: no arming on the cancelled date"
            );
        }
        // Cancel and scratch are also pinned on the
        // Keep-through paths, trailing and fallback.
        for phrase in ["cancel", "scratch"] {
            let group = format!("({phrase} November 1)");
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, None, "{group}: keep-through paren-only");
            let text = format!("Keep-through: choir anthem {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, None, "{group}: keep-through fallback");
            assert_eq!(c.keep_through_parenthesized_mismatch, None, "{group}");
        }
    }

    #[test]
    fn r13a_unspent_governor_survives_clause_break() {
        // Round 13 (R13-2, review R13-2): a clause
        // break erased a governor that had governed
        // nothing yet — "(ignore, November 1)" and
        // its family contributed in full. A break now
        // erases only a spent governor: an unspent
        // governor standing as its clause's trailing
        // element (nothing after it but date-referring
        // filler) still governs the first date of the
        // next clause.
        for group in [
            "(ignore, November 1)",
            "(from, November 1)",
            "(skip, November 1)",
            "(please ignore, November 1)",
            "(do not use, November 1)",
            "(ignore the date, November 1)",
            "(ignore; November 1)",
            "(ignore - November 1)",
            "(ignore \u{2014} November 1)",
        ] {
            let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date, None,
                "{group}: the unspent governor survives the break"
            );
            assert_eq!(
                week_verdict(&c, Some(oct(25))),
                WeekVerdict::SundayInWords(oct(25)),
                "{group}: no arming on the governed date"
            );
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, None, "{group}: paren-only");
            assert_eq!(
                week_verdict(&c, Some(nov(1))),
                WeekVerdict::NoSundayNamed,
                "{group}: never SameSunday on a governed date"
            );
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, None, "{group}: keep-through");
        }
        // The garnish pin stands, on comma, semicolon
        // and dash alike: a governor followed by other
        // words in its own clause reaches nothing
        // across the break, and the fresh-clause date
        // is named, not silently dropped.
        for group in [
            "(notes from the pastor, November 1)",
            "(notes from the pastor; November 1)",
            "(notes from the pastor - November 1)",
        ] {
            let text = format!("For Sunday: October 25, 2026 {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, Some(oct(25)), "{group}");
            let m = c
                .for_sunday_parenthesized_mismatch
                .expect("the fresh-clause date is named, not silently dropped");
            assert_eq!(m.named, nov(1), "{group}");
        }
        // Held care cases stand with original
        // expectations: a non-governing clause before
        // the break contributes its date, a group
        // naming no full date reads nothing, and the
        // between-two-dates pins are untouched — a
        // governor spent on one date (forward, or as
        // the trailing cancellation the date it
        // follows claims) governs nothing after it.
        let text =
            "For Sunday: next Sunday (choir was singing, November 1)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_sunday_parenthesized_date, Some(nov(1)));
        let text = "For Sunday: next Sunday (choir from November)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_sunday_parenthesized_date, None);
        assert_eq!(c.for_sunday_parenthesized_dates, Vec::<SimpleDate>::new());
        let text =
            "For Sunday: next Sunday (ignore November 1, use November 8)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_sunday_parenthesized_date, Some(nov(8)));
        let text = "For Sunday: (November 1, cancelled, November 8)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(
            c.for_date,
            Some(nov(8)),
            "a spent cancellation governs nothing in the fresh clause"
        );
    }

    #[test]
    fn r14a_wider_verb_set_governs_before_the_date() {
        // Round 14 (R14-1a, review R14-1 + red team
        // RT14-2): the before-position verb set the
        // R13-A builder flagged as untaken — put off,
        // delay, omit, disregard, forget (about) —
        // contributed at full strength on every path.
        // The set is completed, the split form
        // included. Each phrasing is pinned on the
        // relative, paren-only, and Keep-through
        // paths, the way the lanes probed them.
        for phrase in [
            "put off",
            "delay",
            "delayed",
            "omit",
            "disregard",
            "forget",
            "forget about",
        ] {
            let group = format!("({phrase} November 1)");
            let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date, None,
                "{group}: governs on the relative path"
            );
            assert_eq!(
                c.for_sunday_parenthesized_dates,
                Vec::<SimpleDate>::new(),
                "{group}: the governed date is not remembered"
            );
            assert_eq!(
                week_verdict(&c, Some(oct(25))),
                WeekVerdict::SundayInWords(oct(25)),
                "{group}: no arming on the governed date"
            );
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, None, "{group}: paren-only");
            assert_eq!(
                week_verdict(&c, Some(nov(1))),
                WeekVerdict::NoSundayNamed,
                "{group}: never SameSunday on a governed date"
            );
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, None, "{group}: keep-through paren-only");
            let text = format!("Keep-through: choir anthem {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, None, "{group}: keep-through fallback");
        }
        // The split form: "put" before the date,
        // "off" after it, one phrasal governor
        // wrapped around its date.
        let group = "(put November 1 off)";
        let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(
            c.for_sunday_parenthesized_date, None,
            "{group}: the split form governs on the relative path"
        );
        let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(c.for_date, None, "{group}: paren-only");
        let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(c.keep_through, None, "{group}: keep-through");
        // Held care: a bare "put" with no "off" is
        // not the phrasal governor — the date stands.
        let text = "For Sunday: (put November 1)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(
            c.for_date,
            Some(nov(1)),
            "a bare \"put\" governs nothing without its \"off\""
        );
    }

    #[test]
    fn r14a_cancellation_verbs_govern_by_stem() {
        // Round 14 (R14-1b, red team RT14-2): the
        // attested verbs' own morphology governed
        // nothing — the list matched whole tokens and
        // no tense class existed beside the
        // contraction class. The cancellation verbs
        // now match by stem: base, -ed, and -ing
        // forms, doubled-consonant and e-drop
        // spellings included.
        for phrase in [
            "cancelling",  // doubled consonant
            "canceling",   // e-drop class spelling of cancel
            "postponing",  // e-drop
            "postponed",   // e-final past
            "removing",    // e-drop
            "dropping",    // doubled consonant
            "dropped",     // doubled consonant past
            "scratching",  // plain -ing
            "delaying",    // plain -ing
            "omitting",    // doubled consonant
            "disregarded", // plain -ed
            "forgetting",  // doubled consonant
        ] {
            let group = format!("({phrase} November 1)");
            let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date, None,
                "{group}: the stem form governs on the relative path"
            );
            assert_eq!(
                week_verdict(&c, Some(oct(25))),
                WeekVerdict::SundayInWords(oct(25)),
                "{group}: no arming on the governed date"
            );
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, None, "{group}: paren-only");
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, None, "{group}: keep-through");
        }
        // Overreach discipline, pinned: the stem rule
        // matches verb inflections only. Noun
        // derivations and near-miss words govern
        // nothing, and the dates they stand before
        // are named.
        for group in [
            "(cancellation policy, November 1)",
            "(postponement notice, November 1)",
            "(November notes, November 1)",
        ] {
            let text = format!("For Sunday: October 25, 2026 {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, Some(oct(25)), "{group}");
            let m = c
                .for_sunday_parenthesized_mismatch
                .expect("the near-miss clause names its date");
            assert_eq!(m.named, nov(1), "{group}: the date is named, not governed");
        }
    }

    #[test]
    fn r14a_trailing_position_draws_from_the_same_family() {
        // Round 14 (R14-2, review R14-3 + red team
        // RT14-1): `governed_after` was never widened
        // — after a date, only cancelled/canceled
        // governed, so the trailing forms of the
        // round's own verbs blessed the dates they
        // cancelled. The trailing position now draws
        // from the same family function as the before
        // position. Before/after symmetry pins: each
        // tail below disqualifies, and its
        // before-position twin disqualifies too.
        for (before, after) in [
            ("drop November 1", "November 1, drop it"),
            ("scratch November 1", "November 1, scratch that"),
            ("postpone November 1", "November 1, postponed"),
            ("remove November 1", "November 1, removed"),
            ("put off November 1", "November 1, put off"),
            ("hold off November 1", "November 1, hold off"),
            ("cancel November 1", "November 1, cancel it"),
            ("delay November 1", "November 1, delayed"),
            ("omit November 1", "November 1, omit it"),
            ("forget November 1", "November 1, forget it"),
            ("ignore November 1", "November 1, ignore"),
            ("dropping November 1", "November 1, dropping"),
            ("cancel November 1", "November 1 - scratch that"),
        ] {
            let group = format!("({after})");
            let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date, None,
                "{group}: the trailing form governs on the relative path"
            );
            assert_eq!(
                week_verdict(&c, Some(oct(25))),
                WeekVerdict::SundayInWords(oct(25)),
                "{group}: no arming on the cancelled date"
            );
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, None, "{group}: paren-only");
            assert_eq!(
                week_verdict(&c, Some(nov(1))),
                WeekVerdict::NoSundayNamed,
                "{group}: never SameSunday on a cancelled date"
            );
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, None, "{group}: keep-through");
            let group = format!("({before})");
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_date, None,
                "{group}: the before-position twin disqualifies too"
            );
        }
        // A governor at the end of a nested
        // parenthetical inside the group still falls
        // back on the date it follows (RT14-1).
        let text = "For Sunday: (November 1 (ignore))\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(
            c.for_date, None,
            "the nested trailing governor disqualifies"
        );
        // Held lines stand: a trailing cancellation
        // is spent on the date it follows and governs
        // nothing after it — the fresh date asserts,
        // for the widened family as for cancelled.
        for group in [
            "(November 1, cancelled, November 8)",
            "(November 1, postponed, November 8)",
            "(November 1, drop it, November 8)",
        ] {
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_date,
                Some(nov(8)),
                "{group}: the spent trailing governor governs nothing after it"
            );
        }
        // And the prospective fallback stays scoped:
        // a "not" with a later date to govern leaves
        // the date it follows untouched.
        let text = "For Sunday: (November 1, not November 8)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(
            c.for_date,
            Some(nov(1)),
            "the trailing \"not\" governs the later date, not this one"
        );
    }

    #[test]
    fn r14a_nope_hyphenated_compounds_and_possessed_wont() {
        // Round 14 (R14-3): "nope" joins the negation
        // family in the before position.
        let group = "(nope November 1)";
        let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(
            c.for_sunday_parenthesized_date, None,
            "{group}: \"nope\" governs on the relative path"
        );
        let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(c.for_date, None, "{group}: paren-only");
        let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(c.keep_through, None, "{group}: keep-through");
        // Its trailing twin governs too, under the
        // last-date fallback (review R14-3's probe).
        let text = "For Sunday: (November 1, nope)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, None, "the trailing \"nope\" disqualifies");
        // Round 14 (R14-4): a hyphenated compound
        // containing a governor stem or negation
        // governs — the hyphens split for the
        // governor test only.
        for group in [
            "(do-not-use November 1)",
            "(no-longer November 1)",
            "(hold-off November 1)",
            "(never-mind November 1)",
            "(scratch-that November 1)",
            "(don't-use November 1)",
        ] {
            let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date, None,
                "{group}: the hyphenated compound governs on the relative path"
            );
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, None, "{group}: paren-only");
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, None, "{group}: keep-through");
        }
        // The hyphenated trailing twin cancels too.
        let text = "For Sunday: (November 1, hold-off)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, None, "the hyphenated trailing compound cancels");
        // Held care: a hyphenated compound with no
        // governor part governs nothing.
        let text = "For Sunday: (follow-up November 1)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(
            c.for_date,
            Some(nov(1)),
            "a hyphenated compound with no governor part names its date"
        );
        // Round 14 (R14-5): "wont" does not govern
        // when immediately preceded by a possessive
        // determiner — "(his wont, November 1)" now
        // NAMES its date, on every path the lanes
        // showed it suppressed.
        for group in [
            "(his wont, November 1)",
            "(his wont November 1)",
            "(her wont, November 1)",
            "(by his wont, November 1)",
            "(as is his wont, November 1)",
        ] {
            let text = format!("For Sunday: October 25, 2026 {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, Some(oct(25)), "{group}");
            let m = c
                .for_sunday_parenthesized_mismatch
                .expect("the possessed \"wont\" clause names its date");
            assert_eq!(m.named, nov(1), "{group}: the date is named, not governed");
        }
        let text = "For Sunday: next Sunday (his wont, November 1)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(
            c.for_sunday_parenthesized_date,
            Some(nov(1)),
            "the possessed \"wont\" governs nothing on the relative path"
        );
        let text = "Keep-through: (his wont, November 1)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(
            c.keep_through,
            Some(nov(1)),
            "the possessed \"wont\" governs nothing on the keep-through path"
        );
        // The unpunctuated "won't" still governs where
        // no possessive precedes it, and the
        // contraction itself is unaffected by a
        // possessive-shaped neighbour.
        let text = "For Sunday: (wont be November 1)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, None, "the bare \"wont\" still governs");
        let text = "For Sunday: (his won't be November 1)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, None, "the contraction \"won't\" is unaffected");
    }

    #[test]
    fn r14a_reach_back_crosses_filler_only_clauses() {
        // Round 14 (RT14-4): the unspent-governor
        // reach-back was exactly one clause — an
        // extra comma defeated the held witness
        // "(ignore the date, November 1)". The
        // reach-back now crosses consecutive
        // intervening clauses while each contains no
        // date token and consists solely of filler
        // words (the, a, an, date, it, note, notes).
        for group in [
            "(ignore, the date, November 1)",
            "(ignore, notes, November 1)",
            "(ignore, the, November 1)",
            "(ignore, a note, the date, November 1)",
        ] {
            let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date, None,
                "{group}: the reach-back crosses the filler clauses"
            );
            assert_eq!(
                week_verdict(&c, Some(oct(25))),
                WeekVerdict::SundayInWords(oct(25)),
                "{group}: no arming on the governed date"
            );
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, None, "{group}: paren-only");
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, None, "{group}: keep-through");
        }
        // Content-clause control: an intervening
        // clause with real content still stops the
        // reach-back, and the date contributes.
        for group in [
            "(ignore, choir notes, November 1)",
            "(ignore, notes from the pastor, November 1)",
        ] {
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_date,
                Some(nov(1)),
                "{group}: a content clause stops the reach-back"
            );
        }
        // Held companions from the same family: two
        // unspent governors in two clauses still
        // govern, and an unspent governor spends on
        // the first date only.
        let text = "For Sunday: (ignore, skip, November 1)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, None, "two unspent governors still govern");
        let text = "For Sunday: (ignore, November 1, November 8)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(
            c.for_date,
            Some(nov(8)),
            "the unspent governor spends on the first date only"
        );
    }

    #[test]
    fn r15a_remaining_verbs_govern_before_and_trailing() {
        // Round 15 (R15-1, review R15-1 + red team
        // RT15-1): the verb family round 14 recorded
        // as not taken — called off / call off, put
        // back, push back, reschedule, move, shift,
        // bump, in their stem forms — contributed at
        // full strength before and trailing, on
        // every path. They govern now.
        for phrase in [
            "called off",
            "call off",
            "calling off",
            "calls off",
            "put back",
            "push back",
            "pushed back",
            "pushing back",
            "reschedule",
            "rescheduled",
            "rescheduling",
            "move",
            "moved",
            "moving",
            "shift",
            "shifted",
            "bump",
            "bumped",
        ] {
            let group = format!("({phrase} November 1)");
            let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date, None,
                "{group}: governs on the relative path"
            );
            assert_eq!(
                week_verdict(&c, Some(oct(25))),
                WeekVerdict::SundayInWords(oct(25)),
                "{group}: no arming on the governed date"
            );
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, None, "{group}: paren-only");
            assert_eq!(
                week_verdict(&c, Some(nov(1))),
                WeekVerdict::NoSundayNamed,
                "{group}: never SameSunday on a governed date"
            );
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, None, "{group}: keep-through");
            let text = format!("Keep-through: choir anthem {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, None, "{group}: keep-through fallback");
        }
        for phrase in [
            "called off",
            "call off",
            "put back",
            "pushed back",
            "push back",
            "rescheduled",
            "reschedule",
            "reschedule it",
            "moved",
            "move",
            "shifted",
            "bumped",
        ] {
            let group = format!("(November 1, {phrase})");
            let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date, None,
                "{group}: the trailing form governs on the relative path"
            );
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, None, "{group}: paren-only");
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, None, "{group}: keep-through");
        }
        // The split forms: the pair wrapped around
        // its date governs, as "put ... off" has
        // since round 14.
        for group in [
            "(called November 1 off)",
            "(put November 1 back)",
            "(push November 1 back)",
        ] {
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, None, "{group}: the split form governs");
        }
        // A spent trailing phrase of the new family
        // governs nothing after it — the fresh date
        // asserts, as with cancelled.
        for group in [
            "(November 1, called off, November 8)",
            "(November 1, rescheduled, November 8)",
            "(November 1, moved, November 8)",
        ] {
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_date,
                Some(nov(8)),
                "{group}: the spent trailing governor governs nothing after it"
            );
        }
        // The "to"-target shapes assert their
        // target: the phrase governs the FROM date
        // when one stands before it, and with no
        // from-date the to-date is the line's date.
        for group in [
            "(rescheduled to November 8)",
            "(moved to November 8)",
            "(move November 1 to November 8)",
            "(November 1, moved to November 8)",
            "(reschedule November 1 to November 8)",
            "(bump November 1 to November 8)",
        ] {
            let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date,
                Some(nov(8)),
                "{group}: the target asserts on the relative path"
            );
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, Some(nov(8)), "{group}: paren-only");
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(8)), "{group}: keep-through");
        }
    }

    #[test]
    fn r15a_s_forms_and_forgots_irregular_past_govern() {
        // Round 15 (R15-2, review R15-2 + red team
        // RT15-2): the stem rule missed its own
        // fourth form — every -s form — and forget's
        // irregular past. Both govern now, subject
        // shapes included.
        for phrase in [
            "cancels",
            "postpones",
            "removes",
            "delays",
            "drops",
            "scratches",
            "omits",
            "disregards",
            "forgets",
            "forgot",
            "forgotten",
            "forgot about",
            "choir cancels",
            "Sue postpones",
            "it removes",
            "choir drops",
            "we forgot",
        ] {
            let group = format!("({phrase} November 1)");
            let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date, None,
                "{group}: governs on the relative path"
            );
            assert_eq!(
                week_verdict(&c, Some(oct(25))),
                WeekVerdict::SundayInWords(oct(25)),
                "{group}: no arming on the governed date"
            );
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, None, "{group}: paren-only");
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, None, "{group}: keep-through");
        }
        for group in ["(November 1, cancels)", "(November 1, forgot)"] {
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, None, "{group}: the trailing form governs");
        }
        // The two-date correction shapes read their
        // asserted date again, on its own terms.
        for group in [
            "(cancels November 1, use November 8)",
            "(forgot November 1, use November 8)",
        ] {
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_date,
                Some(nov(8)),
                "{group}: the asserted date is read"
            );
        }
    }

    #[test]
    fn r15a_stacked_cancellations_spend_the_whole_run() {
        // Round 15 (R15-3, red team RT15-3): the
        // spent-trailing exception covered only the
        // clause immediately after a date, so a
        // second cancellation phrase between the
        // dates read unspent and governed the
        // replacement — both dates vanished. The
        // exception now covers a RUN of consecutive
        // cancellation-only clauses: the whole run
        // spends on the first date, the second
        // asserts. Every lane construction is a
        // witness, the "scratch that" variant
        // included.
        for group in [
            "(November 1, cancelled, cancelled, November 8)",
            "(November 1, postponed, postponed, November 8)",
            "(November 1, cancelled, drop it, November 8)",
            "(November 1, drop it, drop it, November 8)",
            "(November 1, postponed, drop it, November 8)",
            "(November 1, postponed, drop it, the date, November 8)",
            "(November 1, cancelled, the date, drop it, November 8)",
            "(November 1, postponed, scratch that, November 8)",
        ] {
            let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date,
                Some(nov(8)),
                "{group}: the replacement asserts on the relative path"
            );
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, Some(nov(8)), "{group}: paren-only");
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(8)), "{group}: keep-through");
        }
    }

    #[test]
    fn r15a_of_joins_the_filler_reach_back() {
        // Round 15 (RT15-5): the filler reach-back
        // stopped at "of" — a filler clause carrying
        // it read as content and armed the governed
        // date. "of" is filler now, and a genitive
        // fragment of the date phrase ("the date of
        // the service") counts as filler too: it
        // names the date's occasion, not content.
        for group in [
            "(ignore, the date of, November 1)",
            "(ignore, the date of the service, November 1)",
            "(ignore, the date of November 1)",
        ] {
            let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date, None,
                "{group}: the reach-back crosses it"
            );
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, None, "{group}: paren-only");
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, None, "{group}: keep-through");
        }
        // Content-clause control stands: a clause
        // with real content of its own still stops
        // the reach-back.
        let text = "For Sunday: (ignore, the date, choir, November 1)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(
            c.for_date,
            Some(nov(1)),
            "a content clause still stops the reach-back"
        );
    }

    #[test]
    fn r15a_trailing_scope_and_the_named_governed_date() {
        // Round 15 (R15-6, red team RT15-6): one
        // clause-scope rule for the trailing reach.
        // (a) A trailing phrase governs the date it
        // follows only when every clause between
        // the date and the phrase is itself a
        // cancellation or filler clause — a content
        // clause breaks the reach, and the date
        // contributes.
        let group = "(November 1, choir practice, ignore)";
        let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(
            c.for_sunday_parenthesized_date,
            Some(nov(1)),
            "{group}: the content clause breaks the trailing reach"
        );
        let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(c.for_date, Some(nov(1)), "{group}: paren-only");
        let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(c.keep_through, Some(nov(1)), "{group}: keep-through");
        // The in-scope twins still govern: the
        // phrase in the date's own clause, and the
        // phrase behind nothing but filler.
        for group in ["(November 1, ignore)", "(November 1, the date, ignore)"] {
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, None, "{group}: still governs");
        }
        // (b) A trailing phrase governs the date it
        // follows, and that date is still NAMED on
        // the second-date note — governed, but
        // named like any distinct date the line
        // named, never silently dropped.
        for (group, read, named) in [
            ("(November 1, November 8, drop it)", nov(1), nov(8)),
            ("(November 1, November 8, cancelled)", nov(1), nov(8)),
            (
                "(November 8, November 1, postponed, drop it)",
                nov(8),
                nov(1),
            ),
        ] {
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, Some(read), "{group}: the first date stands");
            assert_eq!(
                c.for_sunday_second_dates,
                vec![named],
                "{group}: the governed date is named on the note"
            );
            let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date,
                Some(read),
                "{group}: relative path"
            );
            assert_eq!(
                c.for_sunday_second_dates,
                vec![named],
                "{group}: named on the relative path too"
            );
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(read), "{group}: keep-through");
            assert_eq!(
                c.keep_through_second_dates,
                vec![named],
                "{group}: named on the keep-through note"
            );
        }
    }

    #[test]
    fn r16a_target_rule_reads_the_named_target() {
        // Round 16 (R16-1, review R16-1 + red team
        // RT16-1/2/3): the to-target rule fired only
        // for the telegraphic "(moved to November 8)".
        // The target rule is group-level now: a
        // displacement or postpone phrase and a
        // "to" / "until" + date later in the group —
        // across auxiliaries, an object noun phrase,
        // or "back" — reads that date as the target.
        for group in [
            "(it was moved to November 8)",
            "(the service was moved to November 8)",
            "(the potluck was moved to November 8)",
            "(service was rescheduled to November 8)",
            "(was moved to November 8)",
            "(moved the potluck to November 8)",
            "(moved the meeting to November 8)",
            "(moved it to November 8)",
            "(rescheduled the concert to November 8)",
            "(moved back to November 8)",
            "(moved forward to November 8)",
            "(postponed to November 8)",
            "(postponed until November 8)",
            "(moved to November 8)",
        ] {
            let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date,
                Some(nov(8)),
                "{group}: the target asserts on the relative path"
            );
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, Some(nov(8)), "{group}: paren-only");
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(8)), "{group}: keep-through");
        }
        // With an earlier date in the group, the
        // phrase governs that date and the target
        // asserts; the governed date is named on the
        // second-date note (R16-1 / R16-5).
        for group in [
            "(November 1, postponed to November 8)",
            "(November 1, postponed until November 8)",
            "(it was moved to November 8, November 1)",
        ] {
            let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date,
                Some(nov(8)),
                "{group}: the target asserts on the relative path"
            );
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, Some(nov(8)), "{group}: paren-only");
            assert_eq!(
                c.for_sunday_second_dates,
                vec![nov(1)],
                "{group}: the governed date is named"
            );
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(8)), "{group}: keep-through");
            assert_eq!(
                c.keep_through_second_dates,
                vec![nov(1)],
                "{group}: named on the keep-through note"
            );
        }
        // The claim's scope, stated as pins: a
        // governor standing before the claim zone
        // still governs the target, and a phrase
        // outside the displacement/postpone family
        // ("delayed") claims no target.
        for group in ["(not moved to November 8)", "(delayed until November 8)"] {
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, None, "{group}: no target is claimed");
        }
    }

    #[test]
    fn r16a_prospective_clauses_spend_with_the_run() {
        // Round 16 (R16-2, review Q4): a prospective
        // clause inside a between-dates run broke
        // run-spending — the replacement died unnamed,
        // or both dates died in silence. Clauses
        // between the dates that carry no date of
        // their own and belong to the governor
        // families now spend together on the first
        // date; the second asserts, the first is named.
        for group in [
            "(November 1, not, drop it, November 8)",
            "(November 1, postponed, not, November 8)",
            "(November 1, drop it, not, November 8)",
            "(November 1, ignore, drop it, November 8)",
        ] {
            let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date,
                Some(nov(8)),
                "{group}: the replacement asserts on the relative path"
            );
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, Some(nov(8)), "{group}: paren-only");
            assert_eq!(
                c.for_sunday_second_dates,
                vec![nov(1)],
                "{group}: the spent first date is named"
            );
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(8)), "{group}: keep-through");
            assert_eq!(
                c.keep_through_second_dates,
                vec![nov(1)],
                "{group}: named on the keep-through note"
            );
        }
        // Held: a prospective word in the second
        // date's own clause still governs that date —
        // "(November 1, postponed, not November 8)"
        // names nothing anywhere, the defensible
        // silence — and a prospective clause carrying
        // its own date is that date's own clause.
        let text =
            "For Sunday: (November 1, postponed, not November 8)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, None, "the negated replacement governs");
        let text =
            "For Sunday: (November 1, postponed, use November 8)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(
            c.for_date,
            Some(nov(8)),
            "a prospective clause carrying its own date asserts it"
        );
        // Held, defensible silence: a run AFTER the
        // second date governs it and names it, and a
        // trailing run over both dates silences both.
        let text =
            "For Sunday: (November 1, cancelled, November 8, drop it)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, None, "both dates governed on the page");
        assert_eq!(
            c.for_sunday_second_dates,
            Vec::<SimpleDate>::new(),
            "a lone governed outcome keeps the standing silence"
        );
    }

    #[test]
    fn r16a_auxiliary_class_spends_every_participle() {
        // Round 16 (R16-3, red team RT16-4 + review
        // Q1): the auxiliary-spent rule was a five-word
        // list scoped to single displacement stems.
        // Auxiliaries are a class now (be / have / get
        // forms, one adverb of tolerance), and the
        // spent rule covers every governor participle,
        // cancellation and displacement, single and
        // phrasal: the predicate is spent on its
        // subject, and the following date is the
        // group's occasion date.
        for (group, want) in [
            ("(it was cancelled, November 1)", nov(1)),
            ("(it was canceled, November 1)", nov(1)),
            ("(they are moved, November 1)", nov(1)),
            ("(it is being moved, November 1)", nov(1)),
            ("(it was being moved, November 1)", nov(1)),
            ("(it has moved, November 1)", nov(1)),
            ("(it got moved, November 1)", nov(1)),
            ("(it gets moved, November 1)", nov(1)),
            ("(it was just moved, November 1)", nov(1)),
            ("(it was put back, November 1)", nov(1)),
            ("(it was pushed back, November 1)", nov(1)),
            ("(it was called off, November 1)", nov(1)),
            ("(it has been cancelled, November 1)", nov(1)),
            ("(it was postponed, November 8)", nov(8)),
            ("(it was put back, November 8)", nov(8)),
            ("(it was moved, November 1)", nov(1)),
            ("(she was shifted, November 1)", nov(1)),
            ("(the meeting got moved, November 1)", nov(1)),
            ("(it will be postponed, November 8)", nov(8)),
            ("(it was recently rescheduled, November 1)", nov(1)),
        ] {
            let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date,
                Some(want),
                "{group}: the occasion date contributes on the relative path"
            );
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, Some(want), "{group}: paren-only");
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(want), "{group}: keep-through");
        }
        // Held parity: without the clause break the
        // spent rule does not apply, for both families
        // alike — the predicate governs its date.
        for group in ["(it was moved November 1)", "(it was cancelled November 1)"] {
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, None, "{group}: no clause break, no spending");
        }
    }

    #[test]
    fn r16a_plain_line_two_date_composition() {
        // Round 16 (R16-4, red team RT16-5): the
        // clause/run composition did not reach plain
        // header lines — the same words that assert
        // the replacement inside parentheses ARMED
        // the vacated first date on a plain line. One
        // composition, both surfaces.
        let text = "For Sunday: November 1, 2026, postponed, drop it, November 8, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(nov(8)), "the replacement asserts");
        assert_eq!(
            c.for_sunday_second_dates,
            vec![nov(1)],
            "the governed first date is named"
        );
        let text = "Keep-through: November 1, 2026, postponed, drop it, November 8, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.keep_through, Some(nov(8)), "keep-through mirror");
        assert_eq!(c.keep_through_second_dates, vec![nov(1)]);
        // The single-phrase and mirror shapes compose
        // the same way.
        let text =
            "For Sunday: November 1, 2026, postponed, November 8, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(nov(8)));
        assert_eq!(c.for_sunday_second_dates, vec![nov(1)]);
        let text =
            "For Sunday: November 8, 2026, postponed, November 1, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(
            c.for_date,
            Some(nov(1)),
            "the mirror asserts its replacement"
        );
        assert_eq!(c.for_sunday_second_dates, vec![nov(8)]);
        // The year-less shape with its "for".
        let text = "For Sunday: November 1, postponed, drop it, for November 8\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(nov(8)));
        assert_eq!(c.for_sunday_second_dates, vec![nov(1)]);
        // Both dates governed on the page reads as
        // the group's silence, not as the first date.
        let text = "For Sunday: November 1, 2026, cancelled, November 8, 2026, drop it\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, None, "both dates governed on the page");
        // Single-date plain lines are unchanged: a
        // trailing cancellation still governs.
        let text = "For Sunday: November 1, 2026, cancelled\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, None, "single-date behavior unchanged");
    }

    #[test]
    fn r16a_governed_between_dates_is_named() {
        // Round 16 (R16-5, review Q2): displacement
        // noun labels suppress per the accepted fence,
        // but between dates the suppressed date dropped
        // UNNAMED — the R15-6a cost class. A date
        // governed between dates is now named on the
        // second-date note whatever family governed it.
        for group in [
            "(November 1, usher shift, November 8)",
            "(November 1, parking bump, November 8)",
            "(November 1, cancelled, November 8)",
        ] {
            let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date,
                Some(nov(8)),
                "{group}: the second date asserts on the relative path"
            );
            assert_eq!(
                c.for_sunday_second_dates,
                vec![nov(1)],
                "{group}: the governed date is named on the relative path"
            );
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, Some(nov(8)), "{group}: paren-only");
            assert_eq!(
                c.for_sunday_second_dates,
                vec![nov(1)],
                "{group}: named, not dropped in silence"
            );
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(8)), "{group}: keep-through");
            assert_eq!(
                c.keep_through_second_dates,
                vec![nov(1)],
                "{group}: named on the keep-through note"
            );
        }
    }

    #[test]
    fn r16a_genitive_reach_back_crosses_the_completing_clause() {
        // Round 16 (R16-7, review R16-6): the genitive
        // filler rule was clause-local, so splitting
        // "the date of / the service" at its own comma
        // armed the date under an explicit "ignore".
        // When a filler clause ends in "of", the
        // reach-back also crosses the determiner-headed
        // clause that completes the genitive.
        for group in [
            "(ignore, the date of, the service, November 1)",
            "(ignore, the date of, the choir picnic, November 1)",
        ] {
            let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date, None,
                "{group}: the reach-back crosses it on the relative path"
            );
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, None, "{group}: paren-only");
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, None, "{group}: keep-through");
        }
        // A following clause with real content — a
        // verb-form among its words — still stops
        // the reach-back: the clause is no longer a
        // determiner-headed noun fragment.
        let text = "For Sunday: (ignore, the date of, the moved service, November 1)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(
            c.for_date,
            Some(nov(1)),
            "a verb-form in the following clause stops the reach-back"
        );
    }

    #[test]
    fn r16a_determiner_headed_s_nouns_name_their_dates() {
        // Round 16 (RT16-6): determiner-headed -s noun
        // phrases suppressed like verbs — "(the drops,
        // November 1)" lost its date. An -s stem word
        // heading a determiner-headed noun phrase does
        // not govern in its -s form; bare -s forms
        // keep governing (the accepted verb-identity
        // class stands).
        for group in [
            "(the drops, November 1)",
            "(the cough drops, November 1)",
            "(the moves, November 1)",
            "(the shifts, November 1)",
            "(the bumps, November 1)",
            "(the delays, November 1)",
        ] {
            let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date,
                Some(nov(1)),
                "{group}: the noun phrase names its date on the relative path"
            );
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, Some(nov(1)), "{group}: paren-only");
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(1)), "{group}: keep-through");
        }
        // The trailing position reads the same way.
        let text = "For Sunday: (November 1, the drops)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(nov(1)), "trailing determiner-headed noun");
        // Bare -s forms keep governing.
        for group in ["(ticket drops, November 1)", "(choir cancels November 1)"] {
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, None, "{group}: the bare -s form still governs");
        }
    }

    #[test]
    fn r17a_last_target_wins() {
        // Round 17 (R17-1, red team RT17-1): a
        // from-date plus two targets collapsed — the
        // group asserted nothing and its plain-line
        // twin asserted the SUPERSEDED first target.
        // The target rule takes the LAST target in
        // the claim run; earlier targets are governed
        // and named like the from-date, on both
        // surfaces. Verb-repeated forms already read
        // this way and stand as the held twins.
        for group in [
            "(November 1, moved to November 8, then to November 15)",
            "(November 1, postponed to November 8, then to November 15)",
            "(November 1, moved to November 8, to November 15)",
            "(November 1, moved to November 8, moved to November 15)",
            "(November 1, moved to November 8, then moved to November 15)",
        ] {
            let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date,
                Some(nov(15)),
                "{group}: the last target asserts on the relative path"
            );
            assert_eq!(
                c.for_sunday_second_dates,
                vec![nov(1), nov(8)],
                "{group}: the from-date and the superseded target are named"
            );
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, Some(nov(15)), "{group}: paren-only");
            assert_eq!(
                c.for_sunday_second_dates,
                vec![nov(1), nov(8)],
                "{group}: named on the paren-only note"
            );
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(15)), "{group}: keep-through");
            assert_eq!(
                c.keep_through_second_dates,
                vec![nov(1), nov(8)],
                "{group}: named on the keep-through note"
            );
        }
        // The plain-line twin reads the same way:
        // the last target, the rest named.
        let text = "For Sunday: November 1, 2026, moved to November 8, 2026, then to November 15, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(
            c.for_date,
            Some(nov(15)),
            "the plain twin asserts the last target, not the superseded one"
        );
        assert_eq!(c.for_sunday_second_dates, vec![nov(1), nov(8)]);
        let text = "Keep-through: November 1, 2026, moved to November 8, 2026, then to November 15, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.keep_through, Some(nov(15)), "keep-through mirror");
        assert_eq!(c.keep_through_second_dates, vec![nov(1), nov(8)]);
    }

    #[test]
    fn r17a_mixed_plain_line_composition() {
        // Round 17 (R17-2, red team RT17-2 + review
        // Q4): plain composition stopped at all-framed
        // lines, so a mixed line's governed framed
        // date ARMED at full strength while its
        // relative replacement was merely noted. The
        // composition reaches mixed lines now: the
        // governed signals yield and are named, the
        // first ungoverned signal asserts, and
        // relatives resolve exactly as round 13
        // resolved them — today is Friday, October
        // 16, 2026, so "next Sunday" is October 25.
        for line in [
            "November 1, 2026, cancelled, next Sunday",
            "November 1, 2026, postponed, next Sunday",
            "November 1, 2026, moved to next Sunday",
            "November 1, 2026, postponed, drop it, next Sunday",
        ] {
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_date,
                Some(oct(25)),
                "{line}: the replacement asserts on For Sunday"
            );
            assert_eq!(
                c.for_sunday_second_dates,
                vec![nov(1)],
                "{line}: the governed date is named"
            );
            let text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.keep_through,
                Some(oct(25)),
                "{line}: the replacement asserts on Keep-through"
            );
            assert_eq!(
                c.keep_through_second_dates,
                vec![nov(1)],
                "{line}: named on the keep-through note"
            );
        }
    }

    #[test]
    fn r17a_from_date_is_named() {
        // Round 17 (R17-3, red team RT17-4): in the
        // "from … to" shape the governed from-date
        // was never named — the group asserted its
        // target with an empty note. The shape claims
        // its target now, so the from-date is governed
        // and named like any claimed group's other
        // date (the R16-5 naming principle).
        let group = "(moved from November 1 to November 8)";
        let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(
            c.for_sunday_parenthesized_date,
            Some(nov(8)),
            "{group}: the target asserts on the relative path"
        );
        assert_eq!(
            c.for_sunday_second_dates,
            vec![nov(1)],
            "{group}: the from-date is named on the relative path"
        );
        let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(c.for_date, Some(nov(8)), "{group}: paren-only");
        assert_eq!(
            c.for_sunday_second_dates,
            vec![nov(1)],
            "{group}: the from-date is named, not dropped in silence"
        );
        let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(c.keep_through, Some(nov(8)), "{group}: keep-through");
        assert_eq!(
            c.keep_through_second_dates,
            vec![nov(1)],
            "{group}: named on the keep-through note"
        );
    }

    #[test]
    fn r17a_passive_for_names_the_occasion() {
        // Round 17 (R17-4, red team RT17-5): "was +
        // participle + for + date" silenced the
        // occasion date it stated, while the comma
        // forms and present-tense forms named it.
        // After a passive participle, "for" + date
        // names the date as the occasion.
        for group in [
            "(it was moved for November 1)",
            "(it was rescheduled for November 1)",
            "(it was scheduled for November 1)",
        ] {
            let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date,
                Some(nov(1)),
                "{group}: the occasion date contributes on the relative path"
            );
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, Some(nov(1)), "{group}: paren-only");
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(1)), "{group}: keep-through");
        }
        // Held parity: "for" is the distinguisher —
        // the bare same-clause form still governs.
        let text = "For Sunday: (it was moved November 1)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(
            c.for_date, None,
            "the pinned bare same-clause parity stands unchanged"
        );
    }

    #[test]
    fn r17a_claim_zone_is_clause_bounded() {
        // Round 17 (R17-5, review Q1): the claim
        // zone reached four words past its phrase, so
        // a five-word object lost its target silently
        // on every path. The zone is clause-bounded
        // now, not word-counted: the object noun
        // phrase may run to the end of the phrase's
        // clause. No pre-existing pin asserted the
        // four-word boundary as a reading — the
        // four-word shapes in the r16a set read
        // exactly as they read before, through the
        // same rule with the count removed.
        for group in [
            "(moved the November potluck planning meeting to November 8)",
            "(postponed the November potluck planning meeting until November 8)",
        ] {
            let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date,
                Some(nov(8)),
                "{group}: the target asserts on the relative path"
            );
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, Some(nov(8)), "{group}: paren-only");
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(8)), "{group}: keep-through");
        }
    }

    #[test]
    fn r17a_genitive_crossing_uses_the_determiner_test() {
        // Round 17 (R17-6, review sweep): the
        // genitive crossing's "no verb-form" test
        // called the raw verb test, so a determiner-
        // headed -s noun head failed it and the date
        // armed under an explicit "ignore", while the
        // "the service" control disqualified. The
        // completing-clause test uses the
        // determiner-aware form test now.
        for group in [
            "(ignore, the date of, the drops, November 1)",
            "(ignore, the date of, the shifts, November 1)",
            "(ignore, the date of, the service, November 1)",
            "(ignore, the date of, the classes, November 1)",
        ] {
            let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date, None,
                "{group}: the crossing completes on the relative path"
            );
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, None, "{group}: paren-only");
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, None, "{group}: keep-through");
        }
        // Held: a completing clause whose head is a
        // bare verb-form still stops the crossing.
        let text = "For Sunday: (ignore, the date of, the moved service, November 1)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(
            c.for_date,
            Some(nov(1)),
            "a verb-form head still stops the crossing"
        );
    }

    #[test]
    fn r17a_adverb_tolerance_is_a_closed_list() {
        // Round 17 (R17-7, review sweep): the one-
        // adverb tolerance's -ly catch-all admitted
        // nouns and names in the adverb slot, so
        // "(it was family moved, November 1)" spent
        // like "recently". The set is closed now:
        // a non-tolerated intervening word does not
        // spend — the predicate governs its date, as
        // any non-tolerated word does.
        for group in [
            "(it was family moved, November 1)",
            "(it was kelly moved, November 1)",
            "(it was family cancelled, November 1)",
            "(it was household moved, November 1)",
        ] {
            let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date, None,
                "{group}: no spending on the relative path"
            );
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, None, "{group}: paren-only");
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, None, "{group}: keep-through");
        }
        // The attested adverbs on the closed list
        // still spend.
        for group in [
            "(it was recently moved, November 1)",
            "(it was just moved, November 1)",
            "(it was finally moved, November 1)",
            "(it was originally moved, November 1)",
        ] {
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_date,
                Some(nov(1)),
                "{group}: a listed adverb still spends"
            );
        }
    }

    #[test]
    fn r18a_one_token_cleaning() {
        // Round 18 (R18-1, review Q2): the plain-line
        // cleaner did not strip ":", while the group
        // tokenizer did, so a governor wearing a colon
        // governed inside parentheses and was invisible
        // on a plain line — "cancelled: November 1,
        // 2026, next Sunday" ARMED the cancelled
        // November 1. One cleaning now: the plain path
        // cleans with `group_token`, so the colon form
        // reads exactly as its colon-free twin, on
        // both date lines.
        for line in [
            "cancelled: November 1, 2026, next Sunday",
            "cancelled November 1, 2026, next Sunday",
        ] {
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_date,
                Some(oct(25)),
                "{line}: the replacement asserts on For Sunday"
            );
            assert_eq!(
                c.for_sunday_second_dates,
                vec![nov(1)],
                "{line}: the governed date is named"
            );
            let text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(oct(25)), "{line}: Keep-through");
            assert_eq!(c.keep_through_second_dates, vec![nov(1)], "{line}");
        }
        // The two-framed twin: the colon form asserts
        // the standing date, like the colon-free one.
        for line in [
            "cancelled: November 1, 2026, November 8, 2026",
            "cancelled November 1, 2026, November 8, 2026",
        ] {
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, Some(nov(8)), "{line}");
            assert_eq!(c.for_sunday_second_dates, vec![nov(1)], "{line}");
        }
        // The group twin always read this way; it
        // stands as the control.
        let group = "(cancelled: November 1, November 8)";
        let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(c.for_date, Some(nov(8)), "{group}: paren-only control");
    }

    #[test]
    fn r18a_governance_by_date() {
        // Round 18 (R18-2, red team RT18-2): governance
        // attached to OCCURRENCES, so a cancelled date
        // restated anywhere on the line — in either
        // form, on either side of the governor —
        // resurrected and asserted. Governance now
        // attaches to the date VALUE: a date governed
        // in any occurrence does not assert from
        // another occurrence, unless a target claim
        // names it as the last target.
        for line in [
            "November 1, 2026, cancelled, November 1, 2026",
            "October 25, 2026, cancelled, next Sunday",
            "next Sunday, cancelled, October 25, 2026",
        ] {
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_date, None,
                "{line}: the restated governed date asserts nothing"
            );
            assert_eq!(
                c.for_sunday_second_dates,
                Vec::<SimpleDate>::new(),
                "{line}: and is named nowhere"
            );
            let text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, None, "{line}: Keep-through");
        }
        // At a today where "next Sunday" IS the
        // governed date (Friday, October 23, 2026, so
        // next Sunday is November 1), the relative
        // restatement asserts nothing either, in
        // either order — and the anchored copies read
        // the same (today October 30, file dated
        // October 23: the anchor resolves the relative
        // to November 1 there too).
        let late = oct(23);
        for line in [
            "November 1, 2026, cancelled, next Sunday",
            "next Sunday, cancelled, November 1, 2026",
        ] {
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, late).expect("parses");
            assert_eq!(c.for_date, None, "{line}: at October 23");
            let c = parse_contribution_anchored(&text, oct(30), Some(late)).expect("parses");
            assert_eq!(c.for_date, None, "{line}: anchored at October 23");
        }
        // The control stands: a DIFFERENT replacement
        // date beside the governed one still asserts,
        // with the governed date named (the r17a pin's
        // shape, restated here as this rule's fence).
        let text =
            "For Sunday: November 1, 2026, cancelled, next Sunday\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(oct(25)));
        assert_eq!(c.for_sunday_second_dates, vec![nov(1)]);
        // Groups judge by the same value rule, on
        // every path.
        for group in [
            "(November 1, cancelled, November 1)",
            "(October 25, cancelled, October 25)",
        ] {
            let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date, None,
                "{group}: relative path"
            );
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, None, "{group}: paren-only");
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, None, "{group}: keep-through");
        }
        // The target override: a governed date restated
        // as the LAST target of a claim asserts by the
        // claim, on both surfaces.
        for group in [
            "(meeting November 1, cancelled, moved to November 1)",
            "(November 1, moved to November 1)",
        ] {
            let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date,
                Some(nov(1)),
                "{group}: the claim asserts on the relative path"
            );
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, Some(nov(1)), "{group}: paren-only");
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(1)), "{group}: keep-through");
        }
        let text = "For Sunday: November 1, 2026, cancelled, moved to November 1, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(
            c.for_date,
            Some(nov(1)),
            "the plain target override asserts by the claim"
        );
    }

    #[test]
    fn r18a_target_ranking_by_written_order() {
        // Round 18 (R18-3, red team RT18-1): last-target
        // ranking ignored RELATIVE targets on plain
        // lines — a relative target anywhere but last
        // asserted, and every later framed target was
        // demoted to the note. Ranking is by written
        // order across framed and relative targets
        // alike: the last target asserts, the earlier
        // targets and the from-date are named, matching
        // the all-framed twin.
        for line in [
            "November 1, 2026, moved to next Sunday, then to November 15, 2026",
            "November 1, 2026, postponed, next Sunday, then to November 15, 2026",
        ] {
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_date,
                Some(nov(15)),
                "{line}: the last target asserts on For Sunday"
            );
            assert_eq!(
                c.for_sunday_second_dates,
                vec![nov(1), oct(25)],
                "{line}: the from-date and the resolved relative target are named"
            );
            let text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(15)), "{line}: Keep-through");
            assert_eq!(c.keep_through_second_dates, vec![nov(1), oct(25)], "{line}");
        }
        // The lane's other witnesses: a different
        // from-date, and a three-target chain ending
        // in a framed date past the relative.
        let text = "For Sunday: November 8, 2026, moved to next Sunday, then to November 15, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(nov(15)));
        assert_eq!(c.for_sunday_second_dates, vec![nov(8), oct(25)]);
        let text = "For Sunday: November 1, 2026, moved to next Sunday, then to November 15, 2026, then to November 22, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(nov(22)));
        assert_eq!(c.for_sunday_second_dates, vec![nov(1), oct(25), nov(15)]);
        // The group twin reads identically now that
        // groups carry relative signals (R18-6).
        let group = "(November 1, moved to next Sunday, then to November 15)";
        let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(
            c.for_sunday_parenthesized_date,
            Some(nov(15)),
            "{group}: relative path"
        );
        assert_eq!(c.for_sunday_second_dates, vec![nov(1), oct(25)]);
        let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(c.for_date, Some(nov(15)), "{group}: paren-only");
        assert_eq!(c.for_sunday_second_dates, vec![nov(1), oct(25)]);
        let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(c.keep_through, Some(nov(15)), "{group}: keep-through");
        assert_eq!(c.keep_through_second_dates, vec![nov(1), oct(25)]);
    }

    #[test]
    fn r18a_groups_gain_relative_signals() {
        // Round 18 (R18-6, review Q2): a relative phrase
        // inside a group was not a signal at all —
        // "(November 1, cancelled, next Sunday)" read
        // nothing while its plain twin asserted
        // October 25. Groups now resolve relative
        // phrases and judge them as signals in written
        // order alongside framed ones, under the same
        // per-occurrence rules (governors, claims, and
        // R18-2's governance by date).
        for (group, want, named) in [
            ("(November 1, cancelled, next Sunday)", oct(25), nov(1)),
            ("(November 1, cancelled, this Sunday)", oct(18), nov(1)),
        ] {
            let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date,
                Some(want),
                "{group}: relative path"
            );
            assert_eq!(c.for_sunday_second_dates, vec![named], "{group}");
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, Some(want), "{group}: paren-only");
            assert_eq!(c.for_sunday_second_dates, vec![named], "{group}");
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(want), "{group}: keep-through");
            assert_eq!(c.keep_through_second_dates, vec![named], "{group}");
        }
        // A governed relative alone in its group
        // contributes nothing, as a governed framed
        // date does.
        let group = "(cancelled, next Sunday)";
        let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(c.for_date, None, "{group}: the governed relative is silent");
        // Relative-only groups on a date line
        // contribute their resolution.
        let text = "For Sunday: (next Sunday)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(oct(25)), "(next Sunday) contributes");
        assert_eq!(c.for_sunday_in_words, None, "and is not an in-words line");
        let text = "Keep-through: (next Sunday)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.keep_through, Some(oct(25)), "keep-through mirror");
        let text = "For Sunday: (this Sunday)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(oct(18)), "(this Sunday) contributes");
        let text = "For Sunday: next Sunday (next Sunday)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(
            c.for_sunday_parenthesized_date,
            Some(oct(25)),
            "a relative-only group trailing a relative line is remembered by its resolution"
        );
        // The anchor reaches a mixed group exactly as
        // it reaches a mixed plain line: parsed at
        // October 23 for a file dated October 16, the
        // group's "next Sunday" resolves to October 25
        // and asserts, with November 1 governed and
        // named; unanchored at October 23 the relative
        // resolves to November 1 itself, the governed
        // value, and R18-2 silences the group.
        let group = "(November 1, cancelled, next Sunday)";
        let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution_anchored(&text, oct(23), Some(today())).expect("parses");
        assert_eq!(c.for_date, Some(oct(25)), "{group}: anchored mixed group");
        assert_eq!(c.for_sunday_second_dates, vec![nov(1)]);
        let c = parse_contribution(&text, oct(23)).expect("parses");
        assert_eq!(c.for_date, None, "{group}: unanchored at October 23");
        // A governor-free mixed group follows the
        // standing two-date group rule, as two framed
        // dates do: it contributes neither and offers
        // both as alternatives.
        let text = "For Sunday: (November 1, next Sunday)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, None, "two ungoverned signals: no assertion");
        assert_eq!(
            c.for_sunday_second_dates,
            vec![nov(1), oct(25)],
            "both are offered, in written order"
        );
    }

    #[test]
    fn r18a_governed_relative_is_named() {
        // Round 18 (R18-7, review Q2): lines an earlier
        // reader salvaged never reached the governor
        // composition, so a governed relative signal
        // standing before the salvaged date was never
        // named — "cancelled, next Sunday, November 1,
        // 2026" asserted November 1 with October 25
        // nowhere. A governed relative is now named on
        // the second-date note by its resolved date,
        // as a governed framed date is.
        for line in [
            "cancelled, next Sunday, November 1, 2026",
            "drop next Sunday, November 1, 2026",
        ] {
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_date,
                Some(nov(1)),
                "{line}: the standing date asserts on For Sunday"
            );
            assert_eq!(
                c.for_sunday_second_dates,
                vec![oct(25)],
                "{line}: the governed relative is named by its resolution"
            );
            let text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(1)), "{line}: Keep-through");
            assert_eq!(
                c.keep_through_second_dates,
                vec![oct(25)],
                "{line}: named on the keep-through note"
            );
        }
    }

    #[test]
    fn r19a_governance_binds_bare_restatements_only() {
        // Round 19 (R19-1, review Q1): governance by
        // date value (R18-2) overreached within a line
        // — two EVENTS sharing one date value, one of
        // them governed, read nothing at all, unnamed,
        // because the value rule had no notion of
        // which event an occurrence belonged to. The
        // by-date step now binds only a BARE
        // restatement: an occurrence whose own region
        // carries the date, its framing, filler, and
        // governor phrases and no event content of its
        // own. An occurrence heading its own event
        // clause asserts by its own judgment.
        for group in [
            "(early service November 1, late service cancelled November 1)",
            "(early service cancelled November 1, late service November 1)",
            "(choir party November 1, cancelled, service November 1)",
            "(choir November 1, late service cancelled November 1)",
            "(early service cancelled November 1, November 1, choir)",
        ] {
            let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date,
                Some(nov(1)),
                "{group}: relative path asserts the standing event's date"
            );
            // The governed twin is the SAME value as
            // the asserted date, so the note carries
            // nothing: there is no distinct second
            // date to name.
            assert_eq!(
                c.for_sunday_second_dates,
                Vec::<SimpleDate>::new(),
                "{group}: the note carries nothing on the relative path"
            );
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, Some(nov(1)), "{group}: paren-only");
            assert_eq!(
                c.for_sunday_second_dates,
                Vec::<SimpleDate>::new(),
                "{group}: the note carries nothing on the paren-only path"
            );
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(1)), "{group}: keep-through");
            assert_eq!(
                c.keep_through_second_dates,
                Vec::<SimpleDate>::new(),
                "{group}: the note carries nothing on the keep-through path"
            );
        }
        // The plain twins assert the same way.
        for line in [
            "early service November 1, 2026, late service cancelled November 1, 2026",
            "choir party November 1, 2026, cancelled, service November 1, 2026",
            "service November 1, 2026, choir party November 1, 2026, cancelled",
            "November 1, 2026, cancelled, the service November 1, 2026",
            "November 1, 2026, cancelled, November 1, 2026, choir",
        ] {
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_date,
                Some(nov(1)),
                "{line}: the content occurrence asserts on the plain surface"
            );
            assert_eq!(
                c.for_sunday_second_dates,
                Vec::<SimpleDate>::new(),
                "{line}: and the note carries nothing"
            );
            let text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(1)), "{line}: Keep-through");
        }
        // The bare family stays silent, framing words
        // and all: round 18's witnesses, the moved
        // r12a pin's shape, and a restatement wearing
        // only "on".
        for group in [
            "(November 1, cancelled, November 1)",
            "(not November 1, November 1)",
            "(November 1, November 1, cancelled)",
        ] {
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_date, None,
                "{group}: a bare restatement stays governed"
            );
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, None, "{group}: keep-through");
        }
        let text =
            "For Sunday: November 1, 2026, cancelled, on November 1, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(
            c.for_date, None,
            "a restatement wearing only the framing word \"on\" is bare and stays governed"
        );
    }

    #[test]
    fn r19a_cross_group_governance_scope() {
        // Round 19 (R19-2, red team RT19-1, as
        // adjudicated): governance by date value
        // stopped at the parenthesis in round 18 — a
        // date governed in one group asserted from a
        // sibling group on the same line. The rule is
        // now the same across the boundary as inside
        // one group: a BARE restatement in another
        // group of a value governed anywhere among
        // the line's groups stays governed, in either
        // group order, framing words and all.
        for groups in [
            "(November 1, cancelled) (November 1)",
            "(cancelled, November 1) (November 1)",
            "(November 1) (November 1, cancelled)",
            "(November 1, cancelled) (on November 1)",
            "(November 1, cancelled) (the November 1)",
        ] {
            let text = format!("For Sunday: next Sunday {groups}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date, None,
                "{groups}: relative path reads nothing"
            );
            assert_eq!(
                c.for_sunday_parenthesized_dates,
                Vec::<SimpleDate>::new(),
                "{groups}: no group date is remembered on the relative path"
            );
            let text = format!("For Sunday: {groups}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, None, "{groups}: paren-only reads nothing");
            assert_eq!(
                c.for_sunday_second_dates,
                Vec::<SimpleDate>::new(),
                "{groups}: and names nothing"
            );
            let text = format!("Keep-through: {groups}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, None, "{groups}: keep-through reads nothing");
        }
        // Content groups assert independently, on
        // either side of the boundary: the potluck
        // was cancelled, the service was not, and
        // November 1 asserting for the service is the
        // correct reading (the coordinator's
        // adjudication of RT19-1). A target claim in
        // its own group asserts its target likewise —
        // cross-group governance never eats a claim's
        // target.
        for groups in [
            "(potluck November 1 cancelled) (service November 1)",
            "(November 1, cancelled) (service November 1)",
            "(choir, November 1, cancelled) (service November 1)",
            "(service November 1) (potluck November 1 cancelled)",
            "(November 1, cancelled) (moved to November 1)",
        ] {
            let text = format!("For Sunday: next Sunday {groups}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date,
                Some(nov(1)),
                "{groups}: relative path"
            );
            let text = format!("For Sunday: {groups}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, Some(nov(1)), "{groups}: paren-only");
            let text = format!("Keep-through: {groups}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(1)), "{groups}: keep-through");
        }
        // A governed value does not leak onto other
        // dates: the clean sibling still asserts.
        let text = "For Sunday: (November 1, cancelled) (November 8)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(nov(8)), "the clean group asserts");
        // A suppressed bare group leaves the clean
        // group standing before it untouched.
        let text = "For Sunday: (November 8) (November 1, cancelled) (November 1)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(
            c.for_date,
            Some(nov(8)),
            "the bare third group stays governed; November 8 stands"
        );
        // Two content groups asserting different
        // dates: the line asserts per its standing
        // primacy for paren-only lines — the last
        // group stands — and the other asserted date
        // is named on the parenthesized-mismatch
        // record, not dropped unnamed.
        let groups = "(November 1, cancelled) (service November 8) (service November 1)";
        let text = format!("For Sunday: {groups}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(
            c.for_date,
            Some(nov(1)),
            "{groups}: the last content group stands"
        );
        let mismatch = c
            .for_sunday_parenthesized_mismatch
            .expect("the other asserted date is named");
        assert_eq!(mismatch.named, nov(8), "{groups}: November 8 is named");
        assert_eq!(mismatch.read, nov(1), "{groups}");
        let text = format!("For Sunday: next Sunday {groups}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(
            c.for_sunday_parenthesized_date,
            Some(nov(1)),
            "{groups}: relative path"
        );
        assert_eq!(
            c.for_sunday_parenthesized_dates,
            vec![nov(8), nov(1)],
            "{groups}: both asserted group dates are remembered on the relative path"
        );
        let text = format!("Keep-through: {groups}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(c.keep_through, Some(nov(1)), "{groups}: keep-through");
        let mismatch = c
            .keep_through_parenthesized_mismatch
            .expect("keep-through names the other asserted date too");
        assert_eq!(mismatch.named, nov(8), "{groups}");
    }

    #[test]
    fn r20a_core_group_composition() {
        // Round 20 (R20-1, review F1 = red team
        // RT20-1): a governed plain core beside any
        // contributing trailing group split by path
        // — For Sunday returned the core's governed
        // silence and dropped the group unnamed,
        // while the Keep-through fallback asserted
        // the last group blind, in the base witness
        // asserting the very date the core
        // cancelled. One composition now stands for
        // core + trailing groups on both paths: the
        // core's governed values bind BARE trailing
        // restatements of the same value across the
        // core/group boundary, and a surviving group
        // asserts by the trailing-group primacy the
        // paren-only path uses. Every witness below
        // is read on both lines and the two readings
        // must agree field for field.
        // (line, read date, second dates, mismatch named)
        type CoreGroupCase<'a> = (
            &'a str,
            Option<SimpleDate>,
            &'a [SimpleDate],
            Option<SimpleDate>,
        );
        let cases: [CoreGroupCase<'_>; 14] = [
            // The base witness: the trailing
            // restatement is bare and bound by the
            // core's cancellation — nothing, on
            // both paths.
            ("November 1, 2026, cancelled (November 1)", None, &[], None),
            ("November 1, 2026, postponed (November 1)", None, &[], None),
            ("not November 1, 2026 (November 1)", None, &[], None),
            // A trailing group whose value the core
            // does not govern asserts, the governed
            // core value named on the note.
            (
                "November 1, 2026, cancelled (November 8)",
                Some(nov(8)),
                &[nov(1)],
                None,
            ),
            (
                "November 1, 2026, cancelled (next Sunday)",
                Some(oct(25)),
                &[nov(1)],
                None,
            ),
            (
                "November 1, 2026, cancelled (this Sunday)",
                Some(oct(18)),
                &[nov(1)],
                None,
            ),
            (
                "November 1, 2026, cancelled (the Sunday after November 1)",
                Some(nov(8)),
                &[nov(1)],
                None,
            ),
            // A governed core date does not poison a
            // clean group date it never governed.
            (
                "November 8, 2026, cancelled (November 1)",
                Some(nov(1)),
                &[nov(8)],
                None,
            ),
            // A labelled group asserts its governed
            // value (round 19, R19-1's rule), and a
            // claim group claims its target.
            (
                "November 1, 2026, cancelled (service November 1)",
                Some(nov(1)),
                &[],
                None,
            ),
            (
                "November 1, 2026, cancelled (moved to November 1)",
                Some(nov(1)),
                &[],
                None,
            ),
            (
                "November 1, 2026, cancelled (moved to November 8)",
                Some(nov(8)),
                &[nov(1)],
                None,
            ),
            // A relative restatement of the core's
            // governed relative value is bound too.
            ("next Sunday, cancelled (October 25)", None, &[], None),
            // Two trailing groups: the bare governed
            // restatement is suppressed and the clean
            // group stands, in either group order.
            (
                "November 1, 2026, cancelled (November 8) (November 1)",
                Some(nov(8)),
                &[nov(1)],
                None,
            ),
            (
                "November 1, 2026, cancelled (November 1) (November 8)",
                Some(nov(8)),
                &[nov(1)],
                None,
            ),
        ];
        for (line, want, want_second, want_mismatch) in cases {
            let for_text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let f = parse_contribution(&for_text, today()).expect("parses");
            assert_eq!(f.for_date, want, "{line}: For Sunday");
            assert_eq!(
                f.for_sunday_second_dates,
                want_second.to_vec(),
                "{line}: For Sunday names the governed value"
            );
            let for_mismatch = f.for_sunday_parenthesized_mismatch.map(|m| m.named);
            assert_eq!(for_mismatch, want_mismatch, "{line}: For Sunday mismatch");
            let keep_text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
            let k = parse_contribution(&keep_text, today()).expect("parses");
            assert_eq!(k.keep_through, want, "{line}: Keep-through agrees");
            assert_eq!(
                k.keep_through_second_dates,
                want_second.to_vec(),
                "{line}: Keep-through names the same"
            );
            let keep_mismatch = k.keep_through_parenthesized_mismatch.map(|m| m.named);
            assert_eq!(
                keep_mismatch, want_mismatch,
                "{line}: Keep-through mismatch agrees"
            );
        }
        // Two surviving groups follow the paren-only
        // primacy: the last group stands, the earlier
        // surviving group is named on the mismatch
        // record, the governed core value on the note.
        let line = "November 1, 2026, cancelled (November 8) (November 15)";
        let for_text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
        let f = parse_contribution(&for_text, today()).expect("parses");
        assert_eq!(f.for_date, Some(nov(15)), "{line}: the last group stands");
        assert_eq!(f.for_sunday_second_dates, vec![nov(1)], "{line}");
        let mismatch = f
            .for_sunday_parenthesized_mismatch
            .expect("the earlier surviving group is named");
        assert_eq!(mismatch.named, nov(8), "{line}");
        assert_eq!(mismatch.read, nov(15), "{line}");
        assert!(mismatch.read_from_parentheses, "{line}");
        let keep_text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
        let k = parse_contribution(&keep_text, today()).expect("parses");
        assert_eq!(k.keep_through, Some(nov(15)), "{line}: keep-through");
        assert_eq!(k.keep_through_second_dates, vec![nov(1)], "{line}");
        let mismatch = k
            .keep_through_parenthesized_mismatch
            .expect("keep-through names the earlier group too");
        assert_eq!(mismatch.named, nov(8), "{line}");
        // Held bounds: the trailing group itself
        // governed reads nothing on both paths; a
        // core that asserts is unchanged, its groups
        // still garnish against its date.
        let line = "November 1, 2026, cancelled (November 1, cancelled)";
        let for_text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
        let f = parse_contribution(&for_text, today()).expect("parses");
        assert_eq!(f.for_date, None, "{line}: the group governs itself");
        assert_eq!(f.for_sunday_parenthesized_mismatch, None, "{line}");
        let keep_text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
        let k = parse_contribution(&keep_text, today()).expect("parses");
        assert_eq!(k.keep_through, None, "{line}: keep-through");
        let line = "November 8, 2026 (November 1, cancelled)";
        let for_text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
        let f = parse_contribution(&for_text, today()).expect("parses");
        assert_eq!(
            f.for_date,
            Some(nov(8)),
            "{line}: the asserting core stands"
        );
        assert_eq!(f.for_sunday_parenthesized_mismatch, None, "{line}");
        let keep_text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
        let k = parse_contribution(&keep_text, today()).expect("parses");
        assert_eq!(k.keep_through, Some(nov(8)), "{line}: keep-through");
        // The salvaging core composes as it always
        // did: November 8 asserted, November 1
        // second-named, the group naming November 15
        // on the mismatch record, both paths.
        let line = "November 1, 2026, cancelled, November 8, 2026 (November 15)";
        let for_text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
        let f = parse_contribution(&for_text, today()).expect("parses");
        assert_eq!(f.for_date, Some(nov(8)), "{line}");
        assert_eq!(f.for_sunday_second_dates, vec![nov(1)], "{line}");
        let mismatch = f
            .for_sunday_parenthesized_mismatch
            .expect("the group disagrees with the core's date");
        assert_eq!(mismatch.named, nov(15), "{line}");
        let keep_text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
        let k = parse_contribution(&keep_text, today()).expect("parses");
        assert_eq!(k.keep_through, Some(nov(8)), "{line}: keep-through");
        assert_eq!(k.keep_through_second_dates, vec![nov(1)], "{line}");
        let mismatch = k
            .keep_through_parenthesized_mismatch
            .expect("keep-through names the group too");
        assert_eq!(mismatch.named, nov(15), "{line}");
    }

    #[test]
    fn r21a_leading_groups_compose() {
        // Round 21 (R21-1, red team RT21-1): a group
        // standing BEFORE the plain core composed with
        // nothing at all — the line read nothing on
        // both paths, silently, even when the group
        // agreed with the core, even beside an
        // asserting core. Leading groups now join the
        // one composition in written order, by the
        // trailing rules mirrored: beside an asserting
        // core a leading group is garnish, named on
        // the mismatch when it differs and silent when
        // it agrees; beside a governed core a surviving
        // leading group asserts as a surviving trailing
        // group does, the governed value named on the
        // note. Every witness is read on both lines and
        // the two readings must agree field for field.
        // (line, read date, second dates, mismatch named)
        type LeadCase<'a> = (
            &'a str,
            Option<SimpleDate>,
            &'a [SimpleDate],
            Option<SimpleDate>,
        );
        let cases: [LeadCase<'_>; 8] = [
            // The agreeing group is silent; the core
            // asserts, on both paths.
            ("(November 15) November 15, 2026", Some(nov(15)), &[], None),
            (
                "(November 15, 2026) November 15, 2026",
                Some(nov(15)),
                &[],
                None,
            ),
            // A disagreeing leading group is garnish
            // named on the mismatch, exactly as a
            // trailing one is.
            (
                "(November 8) November 15, 2026",
                Some(nov(15)),
                &[],
                Some(nov(8)),
            ),
            // Beside a governed core the surviving
            // leading group asserts, the governed core
            // value named on the note.
            (
                "(November 8) November 1, 2026, cancelled",
                Some(nov(8)),
                &[nov(1)],
                None,
            ),
            // A bare leading restatement of the
            // governed value binds: nothing reads.
            ("(November 1) November 1, 2026, cancelled", None, &[], None),
            // Two leading groups compose in order: the
            // bare governed restatement binds and the
            // surviving group asserts.
            (
                "(November 8) (November 1) November 1, 2026, cancelled",
                Some(nov(8)),
                &[nov(1)],
                None,
            ),
            // A leading group beside a governed core
            // AND a trailing group: written-order
            // primacy among the survivors — the last
            // stands, the earlier is named on the
            // mismatch record, the governed value on
            // the note.
            (
                "(November 8) November 1, 2026, cancelled (November 15)",
                Some(nov(15)),
                &[nov(1)],
                Some(nov(8)),
            ),
            // A content-headed core reads with its
            // leading group composed the same way.
            (
                "(November 8) service November 15, 2026",
                Some(nov(15)),
                &[],
                Some(nov(8)),
            ),
        ];
        for (line, want, want_second, want_mismatch) in cases {
            let for_text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let f = parse_contribution(&for_text, today()).expect("parses");
            assert_eq!(f.for_date, want, "{line}: For Sunday");
            assert_eq!(
                f.for_sunday_second_dates,
                want_second.to_vec(),
                "{line}: For Sunday second dates"
            );
            let for_mismatch = f.for_sunday_parenthesized_mismatch.map(|m| m.named);
            assert_eq!(for_mismatch, want_mismatch, "{line}: For Sunday mismatch");
            let keep_text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
            let k = parse_contribution(&keep_text, today()).expect("parses");
            assert_eq!(k.keep_through, want, "{line}: Keep-through agrees");
            assert_eq!(
                k.keep_through_second_dates,
                want_second.to_vec(),
                "{line}: Keep-through second dates agree"
            );
            let keep_mismatch = k.keep_through_parenthesized_mismatch.map(|m| m.named);
            assert_eq!(
                keep_mismatch, want_mismatch,
                "{line}: Keep-through mismatch agrees"
            );
        }
        // Two leading groups beside an asserting core
        // are both named on the mismatch record, in
        // written order.
        let line = "(November 8) (November 22) November 15, 2026";
        let for_text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
        let f = parse_contribution(&for_text, today()).expect("parses");
        assert_eq!(f.for_date, Some(nov(15)), "{line}");
        let mismatch = f
            .for_sunday_parenthesized_mismatch
            .expect("both leading groups are named");
        assert_eq!(mismatch.named, nov(8), "{line}");
        assert_eq!(mismatch.also_named, vec![nov(22)], "{line}");
        assert!(!mismatch.read_from_parentheses, "{line}");
        let keep_text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
        let k = parse_contribution(&keep_text, today()).expect("parses");
        assert_eq!(k.keep_through, Some(nov(15)), "{line}: keep-through");
        let mismatch = k
            .keep_through_parenthesized_mismatch
            .expect("keep-through names both too");
        assert_eq!(mismatch.named, nov(8), "{line}");
        assert_eq!(mismatch.also_named, vec![nov(22)], "{line}");
        // The relative core: "(November 8) next
        // Sunday" reads as its trailing twin does —
        // the For Sunday line remembers the words with
        // the group's date on the parenthesized
        // record (the phrase resolves to October 25),
        // and the Keep-through line reads the group.
        // The standing relative-line asymmetry is the
        // shape of both twins, not a split this round
        // closes.
        let line = "(November 8) next Sunday";
        let for_text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
        let f = parse_contribution(&for_text, today()).expect("parses");
        assert_eq!(f.for_date, None, "{line}: the words carry the Sunday");
        assert_eq!(f.for_sunday_in_words.as_deref(), Some(line), "{line}");
        assert_eq!(f.for_sunday_parenthesized_date, Some(nov(8)), "{line}");
        assert_eq!(f.for_sunday_parenthesized_dates, vec![nov(8)], "{line}");
        assert_eq!(
            week_verdict(&f, Some(oct(25))),
            WeekVerdict::DifferentSunday(nov(8)),
            "{line}: the named group date is compared with the resolution"
        );
        let keep_text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
        let k = parse_contribution(&keep_text, today()).expect("parses");
        assert_eq!(k.keep_through, Some(nov(8)), "{line}: keep-through");
        // Held boundary (round 10, R10-10c): a leading
        // group that merely restates the phrase's own
        // Sunday stays unread on both paths — "(October
        // 25) next Sunday" names no Sunday anywhere.
        let line = "(October 25) next Sunday";
        let for_text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
        let f = parse_contribution(&for_text, today()).expect("parses");
        assert_eq!(f.for_date, None, "{line}");
        assert_eq!(f.for_sunday_in_words, None, "{line}: the boundary stands");
        assert_eq!(f.for_sunday_parenthesized_date, None, "{line}");
        assert_eq!(
            week_verdict(&f, Some(oct(25))),
            WeekVerdict::NoSundayNamed,
            "{line}"
        );
        let keep_text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
        let k = parse_contribution(&keep_text, today()).expect("parses");
        assert_eq!(k.keep_through, None, "{line}: keep-through boundary too");
    }

    #[test]
    fn r21a_content_and_for_occasion_cores_compose() {
        // Round 21 (R21-2, red team RT21-2): the R20-1
        // composition keyed on the governed plain
        // core, so a content-headed core ("service
        // November 1, 2026") and the for-occasion
        // cores ("it was scheduled for November 1,
        // 2026", "it was moved for …", "we moved the
        // meeting for …") beside a trailing group
        // still split — For Sunday dropped the whole
        // line silently while Keep-through asserted
        // the last group blind (402 of the lanes' 421
        // census combinations per core). The
        // composition now keys on the core's VALUE,
        // whatever head the core wears: a
        // content-headed core asserts its date and its
        // groups are garnish; a for-occasion core
        // contributes its occasion date as the core
        // value for governance binding — a bare
        // trailing restatement of it binds and reads
        // nothing extra, a different group asserts by
        // the survivor rules with the occasion named
        // on the note, a labelled restatement stands
        // by its own clause — identically on both
        // paths. The for-core alone keeps reading
        // nothing, as it always has.
        // (line, read date, second dates, mismatch named)
        type CoreCase<'a> = (
            &'a str,
            Option<SimpleDate>,
            &'a [SimpleDate],
            Option<SimpleDate>,
        );
        let cases: [CoreCase<'_>; 13] = [
            // Content-headed cores assert; the group
            // is garnish named on the mismatch.
            (
                "service November 1, 2026 (November 8)",
                Some(nov(1)),
                &[],
                Some(nov(8)),
            ),
            (
                "choir November 1, 2026 (November 8)",
                Some(nov(1)),
                &[],
                Some(nov(8)),
            ),
            (
                "service November 1, 2026 (next Sunday)",
                Some(nov(1)),
                &[],
                Some(oct(25)),
            ),
            // An agreeing group beside a content core
            // is silent.
            (
                "service November 1, 2026 (November 1)",
                Some(nov(1)),
                &[],
                None,
            ),
            // A bare trailing restatement of the
            // occasion date binds on both paths: the
            // line reads nothing extra — nothing.
            (
                "it was scheduled for November 1, 2026 (November 1)",
                None,
                &[],
                None,
            ),
            (
                "it was moved for November 1, 2026 (November 1)",
                None,
                &[],
                None,
            ),
            (
                "we moved the meeting for November 1, 2026 (November 1)",
                None,
                &[],
                None,
            ),
            // A different group asserts, the occasion
            // named on the note, on both paths.
            (
                "it was scheduled for November 1, 2026 (November 8)",
                Some(nov(8)),
                &[nov(1)],
                None,
            ),
            (
                "it was moved for November 1, 2026 (November 8)",
                Some(nov(8)),
                &[nov(1)],
                None,
            ),
            (
                "we moved the meeting for November 1, 2026 (next Sunday)",
                Some(oct(25)),
                &[nov(1)],
                None,
            ),
            (
                "it was scheduled for November 1, 2026 (service November 8)",
                Some(nov(8)),
                &[nov(1)],
                None,
            ),
            // A labelled restatement of the occasion
            // is no restatement: it stands by its own
            // clause, as R19-1 requires everywhere.
            (
                "it was scheduled for November 1, 2026 (service November 1)",
                Some(nov(1)),
                &[],
                None,
            ),
            // The for-core alone keeps reading
            // nothing (standing), on both paths.
            ("it was scheduled for November 1, 2026", None, &[], None),
        ];
        for (line, want, want_second, want_mismatch) in cases {
            let for_text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let f = parse_contribution(&for_text, today()).expect("parses");
            assert_eq!(f.for_date, want, "{line}: For Sunday");
            assert_eq!(
                f.for_sunday_second_dates,
                want_second.to_vec(),
                "{line}: For Sunday second dates"
            );
            let for_mismatch = f.for_sunday_parenthesized_mismatch.map(|m| m.named);
            assert_eq!(for_mismatch, want_mismatch, "{line}: For Sunday mismatch");
            let keep_text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
            let k = parse_contribution(&keep_text, today()).expect("parses");
            assert_eq!(k.keep_through, want, "{line}: Keep-through agrees");
            assert_eq!(
                k.keep_through_second_dates,
                want_second.to_vec(),
                "{line}: Keep-through second dates agree"
            );
            let keep_mismatch = k.keep_through_parenthesized_mismatch.map(|m| m.named);
            assert_eq!(
                keep_mismatch, want_mismatch,
                "{line}: Keep-through mismatch agrees"
            );
        }
        // The lanes' census, in miniature (the full
        // 20-core × 421-combination census was re-run
        // against this build as a probe: every family
        // below at zero divergences; the bare relative
        // cores remain the standing asymmetry, out of
        // scope): for each core family — governed
        // framed, content-headed, for-occasion ×3,
        // claim, Sunday-after, salvaged — crossed with
        // a representative group set (none, bare same,
        // bare different, labelled, relative, claim),
        // the For Sunday and Keep-through readings
        // agree field for field.
        let cores = [
            "November 1, 2026, cancelled",
            "service November 1, 2026",
            "choir November 1, 2026",
            "it was scheduled for November 1, 2026",
            "it was moved for November 1, 2026",
            "we moved the meeting for November 1, 2026",
            "November 1, 2026, moved to November 8, 2026",
            "the Sunday after November 1, 2026, cancelled",
            "November 1, 2026, cancelled, November 8, 2026",
        ];
        let groups = [
            "",
            "(November 1)",
            "(November 8)",
            "(service November 1)",
            "(next Sunday)",
            "(moved to November 8)",
        ];
        for core in cores {
            for group in groups {
                let line = if group.is_empty() {
                    core.to_string()
                } else {
                    format!("{core} {group}")
                };
                let for_text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
                let f = parse_contribution(&for_text, today()).expect("parses");
                let keep_text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
                let k = parse_contribution(&keep_text, today()).expect("parses");
                assert_eq!(f.for_date, k.keep_through, "{line}: dates agree");
                assert_eq!(
                    f.for_sunday_second_dates, k.keep_through_second_dates,
                    "{line}: second dates agree"
                );
                let fm = f
                    .for_sunday_parenthesized_mismatch
                    .map(|m| (m.named, m.also_named, m.read, m.read_from_parentheses));
                let km = k
                    .keep_through_parenthesized_mismatch
                    .map(|m| (m.named, m.also_named, m.read, m.read_from_parentheses));
                assert_eq!(fm, km, "{line}: mismatch records agree");
            }
        }
    }

    #[test]
    fn r21a_post_group_plain_segments_compose() {
        // Round 21 (R21-3, red team RT21-3): plain
        // words standing after the last trailing group
        // escaped the composition entirely — the first
        // plain signal read the line and the governed
        // core date resurrected on both paths, the
        // surviving group named nowhere. A post-group
        // plain segment is a segment of the
        // composition now: the line's core is read
        // with every group taken out (a group leaves a
        // clause break behind it, so its neighbours
        // are judged as segments), governance by date
        // value binds a bare post-group restatement, a
        // surviving post-group date stands by the
        // composition, and a surviving group is never
        // displaced silently.
        // (line, read date, second dates, mismatch named)
        type PostCase<'a> = (
            &'a str,
            Option<SimpleDate>,
            &'a [SimpleDate],
            Option<SimpleDate>,
        );
        let cases: [PostCase<'_>; 7] = [
            // The bare post-group restatement of the
            // governed value binds: the surviving
            // group asserts, the governed value named.
            (
                "November 1, 2026, cancelled (November 8) November 1, 2026",
                Some(nov(8)),
                &[nov(1)],
                None,
            ),
            // The no-year twin reads the same.
            (
                "November 1, 2026, cancelled (November 8) November 1",
                Some(nov(8)),
                &[nov(1)],
                None,
            ),
            // The post-group date restates the
            // surviving group's own value: it stands,
            // the governed value named, no mismatch.
            (
                "November 1, 2026, cancelled (November 8) November 8, 2026",
                Some(nov(8)),
                &[nov(1)],
                None,
            ),
            // Two surviving segments: written-order
            // primacy — the later survivor (the
            // post-group plain date) stands, the
            // earlier survivor (the group) is named on
            // the mismatch record, the governed value
            // on the note.
            (
                "November 1, 2026, cancelled (November 8) November 15, 2026",
                Some(nov(15)),
                &[nov(1)],
                Some(nov(8)),
            ),
            // The two-group twin: both groups stand
            // between the governed core and the bound
            // restatement — the last group asserts,
            // the earlier is named on the mismatch.
            (
                "November 1, 2026, cancelled (November 8) (November 15) November 1, 2026",
                Some(nov(15)),
                &[nov(1)],
                Some(nov(8)),
            ),
            // The relative twin: the post-group framed
            // date restates the governed relative's
            // resolution and binds — the group asserts,
            // the resolution named on the note.
            (
                "next Sunday, cancelled (November 8) October 25, 2026",
                Some(nov(8)),
                &[oct(25)],
                None,
            ),
            // Control, unchanged: the restatement
            // inside a group stays governed.
            (
                "November 1, 2026, cancelled (November 8) (November 1)",
                Some(nov(8)),
                &[nov(1)],
                None,
            ),
        ];
        for (line, want, want_second, want_mismatch) in cases {
            let for_text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let f = parse_contribution(&for_text, today()).expect("parses");
            assert_eq!(f.for_date, want, "{line}: For Sunday");
            assert_eq!(
                f.for_sunday_second_dates,
                want_second.to_vec(),
                "{line}: For Sunday second dates"
            );
            let for_mismatch = f.for_sunday_parenthesized_mismatch.map(|m| m.named);
            assert_eq!(for_mismatch, want_mismatch, "{line}: For Sunday mismatch");
            let keep_text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
            let k = parse_contribution(&keep_text, today()).expect("parses");
            assert_eq!(k.keep_through, want, "{line}: Keep-through agrees");
            assert_eq!(
                k.keep_through_second_dates,
                want_second.to_vec(),
                "{line}: Keep-through second dates agree"
            );
            let keep_mismatch = k.keep_through_parenthesized_mismatch.map(|m| m.named);
            assert_eq!(
                keep_mismatch, want_mismatch,
                "{line}: Keep-through mismatch agrees"
            );
        }
        // Control, unchanged: with no group at all the
        // governed restatement stays governed.
        let line = "November 1, 2026, cancelled, November 1, 2026";
        let for_text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
        let f = parse_contribution(&for_text, today()).expect("parses");
        assert_eq!(f.for_date, None, "{line}");
        let keep_text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
        let k = parse_contribution(&keep_text, today()).expect("parses");
        assert_eq!(k.keep_through, None, "{line}: keep-through");
    }

    #[test]
    fn r21a_anchor_parity_for_governed_values() {
        // Round 21 (R21-4, review F1): a relative-
        // governed core read its governed value at
        // scan-day, not at the parse anchor — "next
        // Sunday, cancelled (October 25)" filed
        // October 16 and scanned October 23 asserted
        // the very date the core cancelled and named a
        // phantom November 1 (the scan-day resolution)
        // on the note. Every value the composition
        // binds now resolves its relative signals at
        // the anchor, the anchor the groups have used
        // since R19-3.
        let scan_day = SimpleDate {
            year: 2026,
            month: 10,
            day: 23,
        };
        let file_day = oct(16);
        // (line, read date, second dates)
        type AnchorCase<'a> = (&'a str, Option<SimpleDate>, &'a [SimpleDate]);
        let cases: [AnchorCase<'_>; 3] = [
            // October 25 is the governed value at the
            // anchor: its bare restatement binds, and
            // no phantom November 1 is named anywhere.
            ("next Sunday, cancelled (October 25)", None, &[]),
            ("next Sunday, cancelled (next Sunday)", None, &[]),
            // The surviving group asserts, and the
            // governed value it names is the anchored
            // October 25.
            (
                "next Sunday, cancelled (October 25) (November 8)",
                Some(nov(8)),
                &[oct(25)],
            ),
        ];
        for (line, want, want_second) in cases {
            let for_text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let f =
                parse_contribution_anchored(&for_text, scan_day, Some(file_day)).expect("parses");
            assert_eq!(f.for_date, want, "{line}: For Sunday at the anchor");
            assert_eq!(
                f.for_sunday_second_dates,
                want_second.to_vec(),
                "{line}: For Sunday names the anchored value"
            );
            assert_eq!(f.for_sunday_parenthesized_mismatch, None, "{line}");
            let keep_text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
            let k =
                parse_contribution_anchored(&keep_text, scan_day, Some(file_day)).expect("parses");
            assert_eq!(k.keep_through, want, "{line}: Keep-through agrees");
            assert_eq!(
                k.keep_through_second_dates,
                want_second.to_vec(),
                "{line}: Keep-through names the same"
            );
            assert_eq!(k.keep_through_parenthesized_mismatch, None, "{line}");
        }
    }

    #[test]
    fn r21a_occurrence_regions_end_at_neighbours() {
        // Round 21 (R21-5, review F5): the bare test's
        // after-side sweep credited a bare restatement
        // with the NEXT date's passive-for clause —
        // "November 1, 2026, cancelled, November 1,
        // 2026, it was moved for November 8, 2026"
        // asserted the cancelled November 1, the
        // moved-for construction minus its date being
        // read as the restatement's own predicate. An
        // occurrence's regions end at the neighbouring
        // occurrences now, on every surface that
        // judges bareness: a passive "for" lead-in in
        // the after-region credits the next occurrence,
        // not this one, so the restatement stays bare
        // and bound, and November 8 asserts by its own
        // construction with November 1 named.
        let line =
            "November 1, 2026, cancelled, November 1, 2026, it was moved for November 8, 2026";
        let for_text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
        let f = parse_contribution(&for_text, today()).expect("parses");
        assert_eq!(f.for_date, Some(nov(8)), "{line}: For Sunday");
        assert_eq!(f.for_sunday_second_dates, vec![nov(1)], "{line}");
        let keep_text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
        let k = parse_contribution(&keep_text, today()).expect("parses");
        assert_eq!(k.keep_through, Some(nov(8)), "{line}: Keep-through agrees");
        assert_eq!(k.keep_through_second_dates, vec![nov(1)], "{line}");
        // The group surface judges bareness by the
        // same bound: inside one group the restatement
        // stays governed and the moved-for occasion
        // contributes, where the group used to fence
        // both dates into alternatives.
        let line = "(November 1, cancelled, November 1, it was moved for November 8)";
        let for_text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
        let f = parse_contribution(&for_text, today()).expect("parses");
        assert_eq!(f.for_date, Some(nov(8)), "{line}: paren-only For Sunday");
        assert_eq!(f.for_sunday_second_dates, vec![nov(1)], "{line}");
        let keep_text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
        let k = parse_contribution(&keep_text, today()).expect("parses");
        assert_eq!(k.keep_through, Some(nov(8)), "{line}: keep-through");
        assert_eq!(k.keep_through_second_dates, vec![nov(1)], "{line}");
        // Held (round 20, R20-2): a predicate standing
        // in an occurrence's OWN region still makes it
        // non-bare — the comma twin, whose trailing
        // clause is clause-complete, and the
        // sibling-cancellation shapes pinned in
        // `r20a_passive_predicate_is_not_bare` — and
        // those readings stand unchanged.
        let line = "November 1, 2026, cancelled, November 1, 2026, it was moved, November 8, 2026";
        let for_text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
        let f = parse_contribution(&for_text, today()).expect("parses");
        assert_eq!(f.for_date, Some(nov(8)), "{line}: the comma twin stands");
        assert_eq!(f.for_sunday_second_dates, vec![nov(1)], "{line}");
        let keep_text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
        let k = parse_contribution(&keep_text, today()).expect("parses");
        assert_eq!(k.keep_through, Some(nov(8)), "{line}: keep-through");
    }

    #[test]
    fn r20a_passive_predicate_is_not_bare() {
        // Round 20 (R20-2, review F2): the bare test
        // consumed an auxiliary + governor-participle
        // predicate word by word — "it was moved",
        // "it was rescheduled", "it was cancelled"
        // are all filler, auxiliaries, and governor
        // phrases — so a passive construction's
        // occasion date read as a bare restatement
        // and was suppressed by any sibling
        // governance, while the same construction
        // with a non-governor participle ("it was
        // scheduled for …", "it was held, …")
        // asserted, as the spent / passive-for rules
        // require. A region carrying a passive
        // predicate construction is not bare: the
        // construction heads its own event clause.
        for groups in [
            "(November 1, cancelled) (it was moved for November 1)",
            "(November 1, cancelled) (it was moved, November 1)",
            "(November 1, cancelled) (it was rescheduled for November 1)",
            "(November 1, moved) (it was cancelled, November 1)",
            "(it was moved for November 1) (November 1, cancelled)",
            "(November 1, cancelled, it was moved for November 1)",
        ] {
            let text = format!("For Sunday: next Sunday {groups}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date,
                Some(nov(1)),
                "{groups}: relative path asserts the passive clause's date"
            );
            let text = format!("For Sunday: {groups}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, Some(nov(1)), "{groups}: paren-only");
            let text = format!("Keep-through: {groups}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(1)), "{groups}: keep-through");
        }
        // The plain twins, in both orders.
        for line in [
            "November 1, 2026, cancelled, it was moved for November 1, 2026",
            "it was moved for November 1, 2026, November 1, 2026, cancelled",
        ] {
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_date,
                Some(nov(1)),
                "{line}: the plain passive clause asserts"
            );
            let text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(1)), "{line}: keep-through");
        }
        // Held: a lone "for" fragment stays bare and
        // bound; the constructions that always
        // asserted keep asserting; "use" stays
        // non-bare by the stated judgment call.
        let groups = "(November 1, cancelled) (for November 1)";
        let text = format!("For Sunday: {groups}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(c.for_date, None, "{groups}: a lone for fragment stays bare");
        let text = format!("Keep-through: {groups}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(c.keep_through, None, "{groups}: keep-through");
        for groups in [
            "(November 1, cancelled) (it was scheduled for November 1)",
            "(November 1, cancelled) (it was held, November 1)",
            "(November 1, cancelled) (we moved the meeting for November 1)",
            "(November 1, cancelled) (use November 1)",
        ] {
            let text = format!("For Sunday: {groups}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, Some(nov(1)), "{groups}: paren-only asserts");
            let text = format!("Keep-through: {groups}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(1)), "{groups}: keep-through");
        }
    }

    #[test]
    fn r20a_bare_function_words() {
        // Round 20 (R20-3, review F3 + red team
        // RT20-2): the bare inventory admitted the
        // articles but missed the other function
        // words — possessive determiners,
        // demonstratives, and discourse garnish — so
        // a restatement wearing only one of those
        // words counted as event content and its
        // governed date resurrected on every path.
        // Those classes are filler now. The garnish
        // list is closed: please, yes, again, ps.
        for word in [
            "his", "her", "their", "my", "our", "your", "its", "this", "that", "these", "those",
            "please", "yes", "again", "ps",
        ] {
            let groups = format!("(November 1, cancelled) ({word} November 1)");
            let text = format!("For Sunday: next Sunday {groups}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date, None,
                "{groups}: relative path stays governed"
            );
            let text = format!("For Sunday: {groups}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, None, "{groups}: paren-only stays governed");
            let text = format!("Keep-through: {groups}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.keep_through, None,
                "{groups}: keep-through stays governed"
            );
            // The plain twin reads nothing either.
            let line = format!("November 1, 2026, cancelled, {word} November 1, 2026");
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, None, "{line}: the plain twin stays governed");
            let text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, None, "{line}: keep-through");
        }
        // Held: a label still asserts (round 19,
        // R19-1) — function words bind, content
        // labels assert. That is the pinned
        // boundary of this fix.
        for groups in [
            "(November 1, cancelled) (choir November 1)",
            "(November 1, cancelled) (meeting November 1)",
            "(November 1, cancelled) (November 1 reminder)",
        ] {
            let text = format!("For Sunday: {groups}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_date,
                Some(nov(1)),
                "{groups}: a label asserts beside a sibling cancellation"
            );
            let text = format!("Keep-through: {groups}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(1)), "{groups}: keep-through");
        }
    }

    #[test]
    fn r20b_one_verb_test_for_every_verb_form() {
        // Round 20 (R20-4, review F4 + red team
        // RT20-4/RT20-5): the reach's verb test was
        // two half-rules — R19-4 tempered only
        // generic -ed forms, so listed-stem,
        // irregular, and -ing adjectives heading the
        // object continuation killed the claim
        // identically, a generic -ed verb with a
        // subject and an object read as an adjective
        // (the reach crossed a real clause and named
        // a false from-date), and a postposed -ed
        // after its noun killed the claim. The test
        // is now ONE for every verb-form word: a
        // verb-form word counts as a finite verb
        // only when a noun-like subject stands
        // before it in its segment; otherwise, with
        // a noun-like word after it or standing
        // postposed after its noun, it reads as a
        // modifier and the reach continues.
        // The adjective family claims the target on
        // every path, the from-date named.
        for group in [
            "(moved the meeting, November 1, 2026, and planned dinner to November 8)",
            "(moved the meeting, November 1, 2026, and gathered supplies to November 8)",
            // Round 21 (R21-6): "sung hymns" and
            // "left luggage" stood in this list
            // until the verb test v2 — both are
            // irregular past forms, which cannot
            // head a noun phrase under the v2 rule,
            // so they read as clauses now and are
            // pinned with the clause family below.
            // ("sung hymns" is the move the round-21
            // synthesis pre-authorizes; "left
            // luggage" is the same rule on the same
            // list — "left" is an irregular form of
            // "leave" exactly as "sung" is of
            // "sing".)
            "(moved the meeting, November 1, 2026, and read books to November 8)",
            "(moved the meeting, November 1, 2026, and planning meeting to November 8)",
            "(moved the meeting, November 1, 2026, and meeting notes to November 8)",
            "(moved the meeting, November 1, 2026, and singing lessons to November 8)",
            "(moved the meeting, November 1, 2026, and gathering supplies to November 8)",
            // The stacked generic controls of the
            // review probes claim the same way.
            "(moved the meeting, November 1, 2026, and fresh baked beans to November 8)",
            "(moved the meeting, November 1, 2026, and sliced baked beans to November 8)",
            "(moved the meeting, November 1, 2026, and baked canned goods to November 8)",
            "(moved the meeting, November 1, 2026, and beans baked canned goods to November 8)",
        ] {
            let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date,
                Some(nov(8)),
                "{group}: the target asserts on the relative path"
            );
            assert_eq!(
                c.for_sunday_second_dates,
                vec![nov(1)],
                "{group}: November 1 is named as the from-date"
            );
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, Some(nov(8)), "{group}: paren-only");
            assert_eq!(c.for_sunday_second_dates, vec![nov(1)], "{group}");
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(8)), "{group}: keep-through");
            assert_eq!(c.keep_through_second_dates, vec![nov(1)], "{group}");
        }
        // The plain twins assert the target instead
        // of filing for the from-date.
        for line in [
            "moved the meeting, November 1, 2026, and planned dinner to November 8, 2026",
            "moved the meeting, November 1, 2026, and planning meeting to November 8, 2026",
        ] {
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_date,
                Some(nov(8)),
                "{line}: the plain twin asserts the target"
            );
            assert_eq!(c.for_sunday_second_dates, vec![nov(1)], "{line}");
            let text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(8)), "{line}: keep-through");
            assert_eq!(c.keep_through_second_dates, vec![nov(1)], "{line}");
        }
        // A real clause ends the reach (RT20-4):
        // subject + generic -ed verb + object, like
        // "choir sang hymns" — November 8 keeps its
        // own standing and November 1 is NOT named
        // as a from-date.
        for group in [
            "(moved November 1, choir folded bulletins to November 8)",
            "(moved November 1, team grilled chicken to November 8)",
            "(moved November 1, ushers stacked chairs to November 8)",
        ] {
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_date,
                Some(nov(8)),
                "{group}: November 8 stands on its own"
            );
            assert_eq!(
                c.for_sunday_second_dates,
                Vec::<SimpleDate>::new(),
                "{group}: the reach ended, so November 1 is not named as a from-date"
            );
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(8)), "{group}: keep-through");
            assert_eq!(
                c.keep_through_second_dates,
                Vec::<SimpleDate>::new(),
                "{group}: keep-through note is empty"
            );
        }
        // Round 21 (R21-6, the verb test v2): the
        // two irregular entries moved out of the
        // adjective family above — an irregular
        // past form cannot head a noun phrase, so
        // "sung hymns" and "left luggage" are
        // subjectless clauses and the reach ends.
        // In this "moved the meeting" form the
        // from-date is not governed ("moved" is
        // spent on its object), so with no claim
        // to order the pair the group offers both
        // dates as alternatives instead of
        // claiming the target; in the "(moved
        // November 1, …)" form the same clause
        // leaves November 8 standing alone (see
        // `r21b_verb_test_v2`).
        for group in [
            "(moved the meeting, November 1, 2026, and sung hymns to November 8)",
            "(moved the meeting, November 1, 2026, and left luggage to November 8)",
        ] {
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_date, None,
                "{group}: the reach ended, so the target is not claimed"
            );
            assert_eq!(
                c.for_sunday_second_dates,
                vec![nov(1), nov(8)],
                "{group}: both dates are offered as alternatives"
            );
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, None, "{group}: keep-through");
            assert_eq!(
                c.keep_through_second_dates,
                vec![nov(1), nov(8)],
                "{group}: keep-through offers both dates"
            );
        }
        // A postposed adjective continues the reach
        // (RT20-5): the target is claimed, the
        // from-date named. The comma variant of the
        // stacked family — "beans, baked canned
        // goods", the same postposed construction
        // wearing its comma — claims like its
        // comma-less twin; before this round it
        // collapsed through the reach's
        // clause-boundary rule.
        for group in [
            "(moved the meeting, November 1, 2026, and dinner rolls, baked to November 8)",
            "(moved the meeting, November 1, 2026, and beans, baked canned goods to November 8)",
        ] {
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_date,
                Some(nov(8)),
                "{group}: the postposed adjective continues the reach"
            );
            assert_eq!(
                c.for_sunday_second_dates,
                vec![nov(1)],
                "{group}: November 1 is named as the from-date"
            );
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(8)), "{group}: keep-through");
            assert_eq!(c.keep_through_second_dates, vec![nov(1)], "{group}");
        }
    }

    #[test]
    fn r20b_adverb_tolerance_unified() {
        // Round 20 (R20-5, review F5 + red team
        // RT20-7/RT20-8): adverb tolerance was three
        // different rules — the list omitted the
        // ordinary tier (really, actually, truly,
        // quickly, definitely, probably, carefully),
        // a single one silenced even a target claim,
        // and fronted/pre-auxiliary stacks consulted
        // neither the list nor the two-word cap. The
        // seven words join the list (34 in all), and
        // all three scans (target claim, passive
        // spent, passive-for) apply the SAME list
        // and the SAME two-word cap in BOTH
        // positions: a listed stack of up to two
        // before the auxiliary spends exactly as
        // after it; an unlisted adverb in either
        // position does not spend; a three-word
        // stack does not spend in either position.
        // Post-auxiliary, the spent scan: each new
        // word spends alone, and a listed pair does.
        for group in [
            "(it was really moved, November 1)",
            "(it was actually moved, November 1)",
            "(it was truly moved, November 1)",
            "(it was quickly moved, November 1)",
            "(it was definitely moved, November 1)",
            "(it was probably moved, November 1)",
            "(it was carefully moved, November 1)",
            "(it was just really moved, November 1)",
        ] {
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_date,
                Some(nov(1)),
                "{group}: the new word spends in the spent scan"
            );
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(1)), "{group}: keep-through");
        }
        // The passive-for scan reads them the same
        // way, and the target claim claims through
        // them.
        for group in [
            "(it was really moved for November 1)",
            "(it was just really moved for November 1)",
        ] {
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_date,
                Some(nov(1)),
                "{group}: the new word spends in the passive-for scan"
            );
        }
        let group = "(it was really moved to November 8)";
        let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(
            c.for_date,
            Some(nov(8)),
            "{group}: the claim claims its target through the new word"
        );
        // Both fronted positions spend, in all three
        // scans: pre-auxiliary (between subject and
        // auxiliary) and detached (before the
        // clause).
        for (group, want) in [
            ("(it really was moved, November 1)", nov(1)),
            ("(really, it was moved, November 1)", nov(1)),
            ("(it really was moved for November 1)", nov(1)),
            ("(really, it was moved for November 1)", nov(1)),
            ("(it really was moved to November 8)", nov(8)),
            ("(really, it was moved to November 8)", nov(8)),
        ] {
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_date,
                Some(want),
                "{group}: the fronted stack spends as the post-auxiliary stack does"
            );
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(want), "{group}: keep-through");
        }
        // Three stacked do not spend in either
        // position, in any scan — the fronted three
        // (RT20-8: three words that are NOW listed,
        // so the cap, not the list, is what stops
        // them), the pre-auxiliary three, and the
        // post-auxiliary three.
        for group in [
            // Spent scan.
            "(really truly actually, it was moved, November 1)",
            "(just very plain, it was moved, November 1)",
            "(it just very plain was moved, November 1)",
            "(it was just very plain moved, November 1)",
            // Passive-for scan.
            "(really truly actually, it was moved for November 1)",
            "(it just very plain was moved for November 1)",
            // Target-claim scan.
            "(really truly actually, it was moved to November 8)",
            "(it was just very plain moved to November 8)",
        ] {
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_date, None,
                "{group}: a three-word stack does not spend in either position"
            );
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, None, "{group}: keep-through");
        }
        // The pre-auxiliary three in a claim group
        // leaves the from-date standing on its own:
        // the claim dies, November 8 is governed by
        // the unspent construction, November 1
        // asserts alone and names nothing.
        let group = "(November 1, just very plain was moved to November 8)";
        let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(c.for_date, Some(nov(1)), "{group}: the claim dies");
        assert_eq!(
            c.for_sunday_second_dates,
            Vec::<SimpleDate>::new(),
            "{group}: nothing is named"
        );
        // An unlisted adverb does not spend in
        // either position, in any scan.
        for group in [
            "(it was yesterday moved, November 1)",
            "(it yesterday was moved, November 1)",
            "(yesterday, it was moved, November 1)",
            "(yesterday, it was moved for November 1)",
            "(yesterday, it was moved to November 8)",
        ] {
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_date, None,
                "{group}: an unlisted adverb does not spend in either position"
            );
        }
    }

    #[test]
    fn r20b_weekday_modifier_in_a_group_is_not_a_signal() {
        // Round 20 (R20-6, red team RT20-6): a bare
        // weekday standing as a noun modifier inside
        // a group — "printed Sunday bulletins" —
        // spawned a relative signal and named a date
        // nobody wrote (October 18, this Sunday).
        // A weekday word followed by a noun-like
        // word inside a group is a modifier, not a
        // relative signal: the group asserts its
        // target and names its from-date, and names
        // NO October 18.
        let group =
            "(moved the meeting, November 1, 2026, and printed Sunday bulletins to November 8)";
        let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(
            c.for_sunday_parenthesized_date,
            Some(nov(8)),
            "{group}: the target asserts on the relative path"
        );
        assert_eq!(
            c.for_sunday_second_dates,
            vec![nov(1)],
            "{group}: only the from-date is named — no October 18"
        );
        let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(c.for_date, Some(nov(8)), "{group}: paren-only");
        assert_eq!(c.for_sunday_second_dates, vec![nov(1)], "{group}");
        let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(c.keep_through, Some(nov(8)), "{group}: keep-through");
        assert_eq!(c.keep_through_second_dates, vec![nov(1)], "{group}");
        // The modifier phrase alone names no date
        // at all.
        let text = "For Sunday: (Sunday bulletins)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, None, "a noun phrase names no Sunday");
        assert_eq!(
            c.for_sunday_second_dates,
            Vec::<SimpleDate>::new(),
            "and names nothing on the note"
        );
        // Held: the true relative phrases in groups
        // keep contributing as pinned.
        for (group, want) in [
            ("(next Sunday)", oct(25)),
            ("(this Sunday)", oct(18)),
            ("(the Sunday after November 1)", nov(8)),
        ] {
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_date,
                Some(want),
                "{group}: the relative phrase stands"
            );
        }
    }

    #[test]
    fn r20b_consumed_base_excluded_in_group_alternatives() {
        // Round 20 (R20-7, review F6): R19-8's
        // consumed-base exclusion held only on the
        // plain-core merge; inside a group the
        // restated base was still named through the
        // group's alternatives. The exclusion now
        // applies wherever second dates are named
        // from a Sunday-after phrase: the group
        // below asserts its resolution and names
        // NOTHING for its consumed base, on every
        // path. The salvaged/asserted reading is
        // unchanged.
        let group = "(the Sunday after November 1, November 1, cancelled)";
        let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(
            c.for_date,
            Some(nov(8)),
            "{group}: the resolution asserts, as it did"
        );
        assert_eq!(
            c.for_sunday_second_dates,
            Vec::<SimpleDate>::new(),
            "{group}: the consumed base is not named on the group path"
        );
        let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(c.keep_through, Some(nov(8)), "{group}: keep-through");
        assert_eq!(
            c.keep_through_second_dates,
            Vec::<SimpleDate>::new(),
            "{group}: the base is not named on the keep-through note either"
        );
        let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(
            c.for_sunday_second_dates,
            Vec::<SimpleDate>::new(),
            "{group}: the relative path names no consumed base either"
        );
    }

    #[test]
    fn r20b_chain_link_must_belong_to_a_target_construction() {
        // Round 20 (R20-8, review F7): R19-9's
        // continuation link admitted any pre-date
        // "to" in the last group, including garnish
        // prose — "note to November 15" wore the
        // chain's shape, so the earlier group was
        // named as a superseded target and the real
        // garnish mismatch was suppressed. The link
        // now counts only when its "to" belongs to
        // a target construction: a displacement /
        // postpone phrase in the group governs the
        // link, or the link directly continues one.
        // Garnish "to" in the last group never
        // classifies a chain.
        for groups in [
            "(moved to November 8) (back to November 15)",
            "(moved to November 8) (note to November 15)",
            "(moved to November 8) (return to November 15)",
        ] {
            let text = format!("For Sunday: {groups}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_date,
                Some(nov(15)),
                "{groups}: the last group still asserts"
            );
            assert_eq!(
                c.for_sunday_second_dates,
                Vec::<SimpleDate>::new(),
                "{groups}: November 8 is garnish, not a superseded target"
            );
            let mismatch = c
                .for_sunday_parenthesized_mismatch
                .expect("the garnish mismatch stands");
            assert_eq!(mismatch.named, nov(8), "{groups}");
            assert_eq!(mismatch.read, nov(15), "{groups}");
            let text = format!("Keep-through: {groups}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(15)), "{groups}: keep-through");
            assert_eq!(
                c.keep_through_second_dates,
                Vec::<SimpleDate>::new(),
                "{groups}: keep-through note is empty"
            );
        }
        // Held: the true chains still name their
        // superseded targets (the r19b witnesses,
        // re-pinned here on the link forms the new
        // test admits: a bare continuation link, and
        // a phrase governing its own link).
        for groups in [
            "(moved to November 8) (then to November 15)",
            "(moved to November 8) (to November 15)",
            "(moved to November 8) (moved to November 15)",
        ] {
            let text = format!("For Sunday: {groups}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_date,
                Some(nov(15)),
                "{groups}: the last target asserts"
            );
            assert_eq!(
                c.for_sunday_second_dates,
                vec![nov(8)],
                "{groups}: the superseded target is named"
            );
            assert_eq!(
                c.for_sunday_parenthesized_mismatch, None,
                "{groups}: a chain link is not a garnish disagreement"
            );
        }
    }

    #[test]
    fn r20b_multi_group_losers_are_named() {
        // Round 20 (R20-9, review F8): R19-10's
        // loser naming stopped at exactly two
        // groups; a three-group mixed stack's losers
        // stood on the mismatch record only. The
        // two-group rule generalizes: in a
        // multi-group reading with at least one
        // relative group, losers are named on the
        // note as the two-group rule names them.
        let groups = "(November 8) (next Sunday) (November 15)";
        let text = format!("For Sunday: {groups}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(
            c.for_date,
            Some(nov(15)),
            "{groups}: the last group still stands"
        );
        assert_eq!(
            c.for_sunday_second_dates,
            vec![nov(8), oct(25)],
            "{groups}: both losers are named on the note"
        );
        let mismatch = c
            .for_sunday_parenthesized_mismatch
            .expect("the mismatch record stands as it did");
        assert_eq!(mismatch.named, nov(8), "{groups}");
        assert_eq!(mismatch.also_named, vec![oct(25)], "{groups}");
        assert_eq!(mismatch.read, nov(15), "{groups}");
        let text = format!("Keep-through: {groups}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(c.keep_through, Some(nov(15)), "{groups}: keep-through");
        assert_eq!(
            c.keep_through_second_dates,
            vec![nov(8), oct(25)],
            "{groups}: the losers are named on the keep-through note"
        );
        // Held: the bare framed stack keeps
        // mismatch-only naming (the r18b pin) — no
        // relative group stands in it.
        let text =
            "For Sunday: (November 1) (November 8) (November 15)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(nov(15)), "the bare stack stands");
        assert_eq!(
            c.for_sunday_second_dates,
            Vec::<SimpleDate>::new(),
            "the bare stack names nothing on the note"
        );
        let mismatch = c
            .for_sunday_parenthesized_mismatch
            .expect("the bare stack keeps its garnish mismatch");
        assert_eq!(mismatch.named, nov(1));
        assert_eq!(mismatch.also_named, vec![nov(8)]);
    }

    #[test]
    fn r20b_sunday_after_relative_base_salvages() {
        // Round 20 (R20-10, review F9): a
        // Sunday-after phrase over a RELATIVE base
        // died with its restated governed value —
        // its resolution is the same value as the
        // governed restatement, so governance by
        // value took the resolution with it — while
        // the framed-base twin salvaged. Parity: an
        // ungoverned Sunday-after phrase salvages
        // by its own resolution whatever its base's
        // form; a governed phrase still dies.
        for (line, want) in [
            (
                "the Sunday after next Sunday, November 1, 2026, cancelled",
                nov(1),
            ),
            (
                "the Sunday after this Sunday, October 25, 2026, cancelled",
                oct(25),
            ),
        ] {
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_date,
                Some(want),
                "{line}: the phrase salvages by its own resolution"
            );
            assert_eq!(
                c.for_sunday_second_dates,
                Vec::<SimpleDate>::new(),
                "{line}: the restated base is not named"
            );
            let text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(want), "{line}: keep-through");
            assert_eq!(
                c.keep_through_second_dates,
                Vec::<SimpleDate>::new(),
                "{line}: keep-through note is empty"
            );
        }
        // The group twin salvages the same way.
        let group = "(the Sunday after next Sunday, November 1, cancelled)";
        let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(
            c.for_date,
            Some(nov(1)),
            "{group}: the group phrase salvages by its resolution"
        );
        assert_eq!(
            c.for_sunday_second_dates,
            Vec::<SimpleDate>::new(),
            "{group}: nothing is named"
        );
        // A governed phrase still dies: the governor
        // standing between the phrase and its
        // restatement governs the phrase in its own
        // span, and the bare restatement is bound by
        // that governance.
        let line = "the Sunday after next Sunday, cancelled, November 1, 2026";
        let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(c.for_date, None, "{line}: a governed phrase dies");
        assert_eq!(
            c.for_sunday_second_dates,
            Vec::<SimpleDate>::new(),
            "{line}: and names nothing"
        );
    }

    #[test]
    fn r19a_groups_anchor_all_relatives() {
        // Round 19 (R19-3, review Q2): a relative-only
        // group resolved at scan-day `today` under an
        // anchored parse, while its mixed twin
        // resolved at the file's date — "(next
        // Sunday)" in a file dated October 16, scanned
        // October 23, contributed November 1, a Sunday
        // the sender never meant. Groups now resolve
        // every relative signal at the parse anchor,
        // relative-only exactly as mixed.
        let late = oct(23);
        let file_day = Some(today());
        for (group, want) in [
            ("(next Sunday)", oct(25)),
            ("(choir picnic next Sunday)", oct(25)),
            ("(this Sunday)", oct(18)),
            ("(the Sunday after next Sunday)", nov(1)),
        ] {
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution_anchored(&text, late, file_day).expect("parses");
            assert_eq!(
                c.for_date,
                Some(want),
                "{group}: the anchored relative-only group reads at the file's date"
            );
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution_anchored(&text, late, file_day).expect("parses");
            assert_eq!(c.keep_through, Some(want), "{group}: keep-through");
            let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution_anchored(&text, late, file_day).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date,
                Some(want),
                "{group}: relative path"
            );
        }
        // The unanchored controls keep the scan-day
        // reading the lanes probed.
        let text = "For Sunday: (next Sunday)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, late).expect("parses");
        assert_eq!(
            c.for_date,
            Some(nov(1)),
            "unanchored at October 23, next Sunday is November 1"
        );
        // The mixed twin agrees with its relative-only
        // sibling at the anchor, as it did before.
        let text = "For Sunday: (November 1, cancelled, next Sunday)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution_anchored(text, late, file_day).expect("parses");
        assert_eq!(c.for_date, Some(oct(25)), "the mixed twin at the anchor");
        assert_eq!(c.for_sunday_second_dates, vec![nov(1)]);
        // Held shapes stand: a lone "(Sunday)" group
        // is weekday garnish and contributes nothing,
        // anchored or not; a Sunday-after phrase over
        // a framed base keeps its framed-base
        // resolution; and at October 30, where "this
        // Sunday" IS November 1, the governed merge
        // holds on the group surface.
        let text = "For Sunday: (Sunday)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution_anchored(text, late, file_day).expect("parses");
        assert_eq!(c.for_date, None, "(Sunday) alone stays garnish, anchored");
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, None, "(Sunday) alone stays garnish");
        let text = "For Sunday: (the Sunday after November 1)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution_anchored(text, late, file_day).expect("parses");
        assert_eq!(
            c.for_date,
            Some(nov(8)),
            "a framed-base Sunday-after group keeps its framed-base resolution"
        );
        let text = "For Sunday: (November 1, cancelled, this Sunday)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, oct(30)).expect("parses");
        assert_eq!(
            c.for_date, None,
            "at October 30 this Sunday is the governed November 1 and merges"
        );
        // The 28-day bound holds for groups as for
        // plain lines: a file dated September 24
        // (29 days before the scan) falls back to the
        // unanchored reading.
        let text = "For Sunday: (next Sunday)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution_anchored(text, late, Some(sep(24))).expect("parses");
        assert_eq!(
            c.for_date,
            Some(nov(1)),
            "a file date beyond the bound falls back to today for groups too"
        );
        let c = parse_contribution_anchored(text, late, Some(sep(25))).expect("parses");
        assert_eq!(
            c.for_date,
            Some(oct(4)),
            "a file date exactly 28 days before anchors the group: next Sunday is October 4"
        );
    }

    #[test]
    fn r19b_ed_adjectives_in_the_reach_claim() {
        // Round 19 (R19-4, review Q3 + red team
        // RT19-2): the claim reach's finite-verb test
        // shielded only the word directly after a
        // determiner, so a determiner-less -ed
        // adjective heading the object continuation
        // killed the claim — R18-4's own witness
        // shape collapsed to alternatives, and its
        // plain twin filed for the from-date. The
        // test, in one sentence: an -ed word whose
        // only verb claim is the generic -ed suffix
        // is an adjective when a noun-like word
        // stands immediately after it, and a finite
        // reading of such a word requires a subject
        // standing before it in its segment.
        for group in [
            "(moved the meeting, November 1, 2026, and baked beans to November 8)",
            "(moved the meeting, November 1, 2026, and canned goods to November 8)",
            "(moved the meeting, November 1, 2026, and sliced bread to November 8)",
            "(moved the meeting, November 1, 2026, and printed bulletins to November 8)",
            // The determiner twin, held from round 18.
            "(moved the meeting, November 1, 2026, and the baked beans to November 8)",
        ] {
            let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date,
                Some(nov(8)),
                "{group}: the target asserts on the relative path"
            );
            assert_eq!(
                c.for_sunday_second_dates,
                vec![nov(1)],
                "{group}: November 1 is named as the from-date"
            );
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, Some(nov(8)), "{group}: paren-only");
            assert_eq!(c.for_sunday_second_dates, vec![nov(1)], "{group}");
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(8)), "{group}: keep-through");
            assert_eq!(c.keep_through_second_dates, vec![nov(1)], "{group}");
        }
        // The simple continuation claims the same
        // way, and the plain twin asserts the target
        // instead of filing for the from-date.
        let group = "(moved November 1, and baked beans to November 8)";
        let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(c.for_date, Some(nov(8)), "{group}: paren-only");
        assert_eq!(
            c.for_sunday_second_dates,
            vec![nov(1)],
            "{group}: the claim names its from-date"
        );
        let line = "moved the meeting, November 1, 2026, and baked beans to November 8, 2026";
        let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(
            c.for_date,
            Some(nov(8)),
            "{line}: the plain twin asserts the target, not the from-date"
        );
        assert_eq!(c.for_sunday_second_dates, vec![nov(1)], "{line}");
        let text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(c.keep_through, Some(nov(8)), "{line}: keep-through");
        assert_eq!(c.keep_through_second_dates, vec![nov(1)], "{line}");
        // Held: a real second clause still ends the
        // reach — "sang" through the irregular list,
        // and an -ed verb with its own subject
        // standing before it ("team played").
        for group in [
            "(moved November 1, choir sang to November 8)",
            "(moved November 1, team played to November 8)",
            "(moved November 1, choir practiced to November 8)",
        ] {
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_date,
                Some(nov(8)),
                "{group}: November 8 stands on its own"
            );
            assert_eq!(
                c.for_sunday_second_dates,
                Vec::<SimpleDate>::new(),
                "{group}: the reach ended, so November 1 is not named as a from-date"
            );
        }
    }

    #[test]
    fn r19b_verb_list_gaps_close() {
        // Round 19 (R19-5, review Q3 + red team
        // RT19-3): the finite-verb closed list missed
        // the go family and the rehearses / gathers /
        // plans stems, so the claim reach crossed
        // those second clauses and named the from-date
        // (the R18-8 witness behavior with a different
        // verb). They join the test, and the second
        // clauses end the reach like "sang" does: the
        // claim dies, and November 8 keeps its own
        // standing per the R18-8 reading.
        for group in [
            "(moved November 1, choir goes to November 8)",
            "(moved November 1, team plans to November 8)",
            "(moved November 1, choir rehearses to November 8)",
            "(moved November 1, congregation gathers to November 8)",
            "(moved November 1, choir went to November 8)",
        ] {
            let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date,
                Some(nov(8)),
                "{group}: November 8 stands on its own, on the relative path"
            );
            assert_eq!(
                c.for_sunday_second_dates,
                Vec::<SimpleDate>::new(),
                "{group}: the claim died, so November 1 is not named"
            );
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, Some(nov(8)), "{group}: paren-only");
            assert_eq!(
                c.for_sunday_second_dates,
                Vec::<SimpleDate>::new(),
                "{group}: paren-only note is empty"
            );
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(8)), "{group}: keep-through");
            assert_eq!(
                c.keep_through_second_dates,
                Vec::<SimpleDate>::new(),
                "{group}: keep-through note is empty"
            );
        }
    }

    #[test]
    fn r19b_active_for_determiner_protects_a_modified_head() {
        // Round 19 (R19-6, review Q4 + red team
        // RT19-4): the active-for object scan shielded
        // only the word directly after the determiner,
        // so any modifier between the determiner and
        // the head noun "meeting" (meet's -ing form)
        // exposed it as a second verb and the occasion
        // date fell silent. In this scan a determiner
        // standing anywhere before the object phrase's
        // head protects it.
        for group in [
            "(we moved the big meeting for November 1)",
            "(we moved the church meeting for November 1)",
            "(we moved the Johnson meeting for November 1)",
            "(we moved a big meeting for November 1)",
            "(she moved the Johnson meeting for November 1)",
            // The bare heads, held from round 18.
            "(we moved the meeting for November 1)",
            "(we moved the files for November 1)",
            "(we moved the Johnson files for November 1)",
        ] {
            let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date,
                Some(nov(1)),
                "{group}: the occasion contributes on the relative path"
            );
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, Some(nov(1)), "{group}: paren-only");
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(1)), "{group}: keep-through");
        }
        // The fences stand: bare active parity
        // governs, and an active cancellation with
        // "for" stays outside the displacement family.
        for group in [
            "(we moved it November 1)",
            "(we moved the meeting November 1)",
            "(we cancelled it for November 1)",
            "(we cancelled the meeting for November 1)",
        ] {
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, None, "{group}: outside the active-for scope");
        }
    }

    #[test]
    fn r19b_adverb_list_completed_and_both_positions_tolerate() {
        // Round 19 (R19-7, red team RT19-5/RT19-6):
        // the closed tolerated-adverb list omitted
        // the ordinary intensifiers very, plain, and
        // simply, so a stack wearing one silenced in
        // every scan, while the same stack standing
        // before the auxiliary spent. The three words
        // join the list (27 words in all), and one rule
        // governs both positions: a listed stack of
        // up to two standing before the auxiliary
        // tolerates exactly as after it — in all three
        // scans that cross the slot (the spent-
        // predicate scan, the passive-for scan, and
        // the target claim's zone scan).
        // Post-auxiliary stacks now spend in the
        // spent-predicate scan, on every path.
        for group in [
            "(it was just plain moved, November 1)",
            "(it was recently plain moved, November 1)",
            "(it was just simply moved, November 1)",
            "(it was very recently moved, November 1)",
            "(it has been just plain moved, November 1)",
        ] {
            let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date,
                Some(nov(1)),
                "{group}: the stack spends on the relative path"
            );
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, Some(nov(1)), "{group}: paren-only");
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(1)), "{group}: keep-through");
        }
        // The passive-for scan reads the new words
        // the same way.
        for group in [
            "(it was just plain moved for November 1)",
            "(it was very recently moved for November 1)",
            "(it was just simply moved for November 1)",
        ] {
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_date,
                Some(nov(1)),
                "{group}: the stack spends in the passive-for scan"
            );
        }
        // The target claim's zone scan: the stack
        // between the auxiliary and the participle no
        // longer leaves "moved" outside its own claim
        // zone to govern the target.
        let group = "(it was just plain moved to November 8)";
        let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(
            c.for_date,
            Some(nov(8)),
            "{group}: the claim claims its target"
        );
        // Both positions, in all three scans: a
        // fronted stack (standing before the whole
        // clause, or between the subject and the
        // auxiliary) reads exactly as the
        // post-auxiliary stack does.
        for group in [
            // Spent-predicate scan.
            "(just plain, it was moved, November 1)",
            "(it just plain was moved, November 1)",
            "(very recently, it was moved, November 1)",
            // Passive-for scan.
            "(just plain, it was moved for November 1)",
            "(it just plain was moved for November 1)",
            // Target-claim zone scan.
            "(just plain, it was moved to November 8)",
            "(it just plain was moved to November 8)",
        ] {
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            let want = if group.contains("November 8") {
                nov(8)
            } else {
                nov(1)
            };
            assert_eq!(
                c.for_date,
                Some(want),
                "{group}: the fronted stack spends as the post-auxiliary stack does"
            );
        }
        // Held lines stand: an unlisted word in the
        // post-auxiliary slot still does not spend,
        // and three stacked listed adverbs still
        // break the tolerance there.
        for group in [
            "(it was family moved, November 1)",
            "(it was still just plain moved, November 1)",
        ] {
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_date, None,
                "{group}: past the tolerance, the participle governs"
            );
        }
    }

    #[test]
    fn r19b_naming_excludes_consumed_bases() {
        // Round 19 (R19-8, review sweep): the R18-7
        // composition naming merge leaked a Sunday-
        // after phrase's consumed base: the
        // composition's signal scan splits the
        // restated base into its own signal, so "the
        // Sunday after November 1, 2026, November 1,
        // 2026, cancelled" named November 1 beside
        // the November 8 it asserts — the base the
        // standing second-date scan deliberately
        // never names. The merge now excludes a
        // relative phrase's consumed base, as the
        // standing scan does. What asserts is
        // unchanged; only the leaked name goes.
        let line = "the Sunday after November 1, 2026, November 1, 2026, cancelled";
        let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(
            c.for_date,
            Some(nov(8)),
            "{line}: the salvaged reading itself is unchanged"
        );
        assert_eq!(
            c.for_sunday_second_dates,
            Vec::<SimpleDate>::new(),
            "{line}: the consumed base is not named"
        );
        let text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(c.keep_through, Some(nov(8)), "{line}: keep-through");
        assert_eq!(
            c.keep_through_second_dates,
            Vec::<SimpleDate>::new(),
            "{line}: the base is not named on the keep-through note either"
        );
        // The control without the governor names
        // nothing either, as it never has.
        let line = "the Sunday after November 1, 2026, November 1, 2026";
        let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(c.for_date, Some(nov(8)), "{line}");
        assert_eq!(
            c.for_sunday_second_dates,
            Vec::<SimpleDate>::new(),
            "{line}: a restated base is a restatement"
        );
        // The merge still names what it exists to
        // name: a governed relative signal standing
        // before a salvaged date (the R18-7 witness)
        // is no phrase's base, and keeps its name.
        let line = "cancelled, next Sunday, November 1, 2026";
        let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(c.for_date, Some(nov(1)), "{line}");
        assert_eq!(
            c.for_sunday_second_dates,
            vec![oct(25)],
            "{line}: the governed relative is still named by its resolution"
        );
    }

    #[test]
    fn r19b_chain_test_requires_a_claim() {
        // Round 19 (R19-9, review sweep): R18-11's
        // chain-vs-stack test rode on any "to"/"until"
        // token in the last contributing group, so a
        // bare date group wearing garnish prose that
        // happened to contain "to" misclassified as a
        // chain link — November 8 was reclassified
        // from disagreeing garnish to superseded
        // target, and the real mismatch was
        // suppressed. The test now requires an actual
        // target claim in the last group.
        for groups in [
            "(moved to November 8) (November 15, note to self)",
            "(moved to November 8) (November 15, back to church)",
        ] {
            let text = format!("For Sunday: {groups}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_date,
                Some(nov(15)),
                "{groups}: the last group still asserts"
            );
            assert_eq!(
                c.for_sunday_second_dates,
                Vec::<SimpleDate>::new(),
                "{groups}: November 8 is garnish, not a superseded target"
            );
            let mismatch = c
                .for_sunday_parenthesized_mismatch
                .expect("the garnish mismatch is no longer suppressed");
            assert_eq!(mismatch.named, nov(8), "{groups}");
            assert_eq!(mismatch.read, nov(15), "{groups}");
            let text = format!("For Sunday: next Sunday {groups}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date,
                Some(nov(15)),
                "{groups}: relative path asserts the last group"
            );
            assert_eq!(
                c.for_sunday_second_dates,
                Vec::<SimpleDate>::new(),
                "{groups}: relative path names no chain link"
            );
            let text = format!("Keep-through: {groups}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(15)), "{groups}: keep-through");
            assert_eq!(
                c.keep_through_second_dates,
                Vec::<SimpleDate>::new(),
                "{groups}: keep-through note is empty"
            );
            let mismatch = c
                .keep_through_parenthesized_mismatch
                .expect("keep-through keeps the garnish mismatch too");
            assert_eq!(mismatch.named, nov(8), "{groups}");
        }
        // The true chain family still names its
        // superseded targets, in every link form.
        for groups in [
            "(moved to November 8) (then to November 15)",
            "(moved to November 8) (to November 15)",
            "(moved to November 8) (until November 15)",
            "(moved to November 8) (moved to November 15)",
        ] {
            let text = format!("For Sunday: {groups}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_date,
                Some(nov(15)),
                "{groups}: the last target asserts"
            );
            assert_eq!(
                c.for_sunday_second_dates,
                vec![nov(8)],
                "{groups}: the superseded target is named"
            );
            assert_eq!(
                c.for_sunday_parenthesized_mismatch, None,
                "{groups}: a chain link is not a garnish disagreement"
            );
        }
    }

    #[test]
    fn r19b_two_group_loser_is_named() {
        // Round 19 (R19-10, red team RT19-7): a
        // two-group reading mixing a framed group and
        // a relative group resolved by last-group-wins
        // and left the loser off the note — "(November
        // 8) (next Sunday)" asserted October 25 with
        // November 8 standing only on the mismatch
        // record; the note carried nothing, on either
        // line. The loser of a two-group reading
        // is now named on the note, as the other signal
        // of a plain mixed line is.
        let groups = "(November 8) (next Sunday)";
        let text = format!("For Sunday: {groups}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(
            c.for_date,
            Some(oct(25)),
            "{groups}: the last group still stands"
        );
        assert_eq!(
            c.for_sunday_second_dates,
            vec![nov(8)],
            "{groups}: the loser is named on the note"
        );
        let mismatch = c
            .for_sunday_parenthesized_mismatch
            .expect("the mismatch record stands as it did");
        assert_eq!(mismatch.named, nov(8), "{groups}");
        assert_eq!(mismatch.read, oct(25), "{groups}");
        let text = format!("Keep-through: {groups}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(c.keep_through, Some(oct(25)), "{groups}: keep-through");
        assert_eq!(
            c.keep_through_second_dates,
            vec![nov(8)],
            "{groups}: the loser is named on the keep-through note"
        );
        // The reversed order reads alike: the framed
        // group stands, the relative loser is named.
        let groups = "(next Sunday) (November 8)";
        let text = format!("For Sunday: {groups}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(c.for_date, Some(nov(8)), "{groups}: the last group stands");
        assert_eq!(
            c.for_sunday_second_dates,
            vec![oct(25)],
            "{groups}: the relative loser is named by its resolution"
        );
        // Held: the bare framed stack keeps its
        // standing reading — the loser named on the
        // mismatch record alone, nothing on the note
        // (the r18b chain pin's control).
        let text = "For Sunday: (November 1) (November 8)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(nov(8)), "the bare stack stands");
        assert_eq!(
            c.for_sunday_second_dates,
            Vec::<SimpleDate>::new(),
            "the bare stack names nothing on the note"
        );
        let mismatch = c
            .for_sunday_parenthesized_mismatch
            .expect("the bare stack keeps its garnish mismatch");
        assert_eq!(mismatch.named, nov(1));
    }

    #[test]
    fn r18b_intervening_date_is_a_from_date() {
        // Round 18 (R18-4, review Q3): a list comma
        // in the object collapsed the from/to group —
        // "(moved the meeting, November 1, 2026, and
        // the dinner to November 8)" offered both
        // dates as alternatives and asserted nothing,
        // where its comma-after-date twin asserted
        // November 8 with November 1 named. An
        // intervening date inside the claim reach is
        // a from-date (governed + named); it never
        // breaks the reach by standing there, and
        // neither does the list comma in front of it.
        for group in [
            "(moved the meeting, November 1, 2026, and the dinner to November 8)",
            "(moved the meeting November 1, 2026, and the dinner to November 8)",
        ] {
            let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date,
                Some(nov(8)),
                "{group}: the target asserts on the relative path"
            );
            assert_eq!(
                c.for_sunday_second_dates,
                vec![nov(1)],
                "{group}: the intervening date is named as the from-date"
            );
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, Some(nov(8)), "{group}: paren-only");
            assert_eq!(c.for_sunday_second_dates, vec![nov(1)], "{group}");
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(8)), "{group}: keep-through");
            assert_eq!(c.keep_through_second_dates, vec![nov(1)], "{group}");
        }
        // The plain-line twin reads the same way.
        let line = "moved the meeting, November 1, 2026, and the dinner to November 8, 2026";
        let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(
            c.for_date,
            Some(nov(8)),
            "{line}: the plain twin asserts the target"
        );
        assert_eq!(c.for_sunday_second_dates, vec![nov(1)], "{line}");
        let text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(c.keep_through, Some(nov(8)), "{line}: keep-through");
        assert_eq!(c.keep_through_second_dates, vec![nov(1)], "{line}");
    }

    #[test]
    fn r18b_reach_ends_at_a_second_clause() {
        // Round 18 (R18-8, review Q3): the date-span
        // exemption let the claim reach cross a real
        // second clause — "(moved November 1, choir
        // sang to November 8)" CLAIMED November 8
        // across "choir sang" and named November 1
        // as its from-date. The rule, stated plainly:
        // after an intervening date's span, the reach
        // continues only into the target construction
        // itself ("to", "then to", "and to") or an
        // object continuation of the phrase's object
        // ("and the dinner to …"); a new finite verb
        // clause — a subject and its finite verb —
        // ends it. The distinguisher is the finite
        // verb: object continuations name things, a
        // second clause does something.
        for group in [
            "(moved November 1, choir sang to November 8)",
            "(moved November 1, choir practiced to November 8)",
        ] {
            let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date,
                Some(nov(8)),
                "{group}: November 8 stands on its own, unclaimed"
            );
            assert_eq!(
                c.for_sunday_second_dates,
                Vec::<SimpleDate>::new(),
                "{group}: nothing is claimed, so November 1 is not named as a from-date"
            );
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, Some(nov(8)), "{group}: paren-only");
            assert_eq!(
                c.for_sunday_second_dates,
                Vec::<SimpleDate>::new(),
                "{group}: paren-only note is empty"
            );
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(8)), "{group}: keep-through");
            assert_eq!(
                c.keep_through_second_dates,
                Vec::<SimpleDate>::new(),
                "{group}: keep-through note is empty"
            );
        }
        // November 8's own standing is the standing
        // of any lone ungoverned group date: "(choir
        // sang to November 8)" has always asserted
        // it, and still does.
        let text = "For Sunday: (choir sang to November 8)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(
            c.for_date,
            Some(nov(8)),
            "the second clause's own date stands"
        );
        // The continuations that DO claim: the "to"
        // directly, its lead-ins, and an object
        // continuation — each names November 1 as
        // the from-date.
        for group in [
            "(moved November 1, to November 8)",
            "(moved November 1, then to November 8)",
            "(moved November 1, and to November 8)",
            "(moved November 1, the dinner to November 8)",
            "(moved November 1, and the dinner to November 8)",
        ] {
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_date,
                Some(nov(8)),
                "{group}: the reach claims its target"
            );
            assert_eq!(
                c.for_sunday_second_dates,
                vec![nov(1)],
                "{group}: the from-date is named"
            );
        }
    }

    #[test]
    fn r18b_for_date_enters_composition() {
        // Round 18 (R18-5, red team RT18-3): the
        // passive-"for" occasion reading collapsed
        // when any second date stood in the group —
        // both dates merely noted, neither filed.
        // The for-date enters the composition as a
        // signal like any other: two for-side dates
        // read by primacy with the other noted, and a
        // prospective second date ("use November 8")
        // asserts with the for-date named.
        for (group, want, named) in [
            (
                "(it was scheduled for November 1 and November 8)",
                nov(1),
                nov(8),
            ),
            (
                "(it was scheduled for November 8 and November 1)",
                nov(8),
                nov(1),
            ),
            ("(it was moved for November 1, November 8)", nov(1), nov(8)),
            (
                "(it was moved for November 1, then November 8)",
                nov(1),
                nov(8),
            ),
        ] {
            let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date,
                Some(want),
                "{group}: primacy asserts on the relative path"
            );
            assert_eq!(
                c.for_sunday_second_dates,
                vec![named],
                "{group}: the other date is noted"
            );
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, Some(want), "{group}: paren-only");
            assert_eq!(c.for_sunday_second_dates, vec![named], "{group}");
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(want), "{group}: keep-through");
            assert_eq!(c.keep_through_second_dates, vec![named], "{group}");
        }
        // The prospective form asserts its own date,
        // on the group paths and the plain twin.
        let group = "(it was moved for November 1, use November 8)";
        let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(
            c.for_sunday_parenthesized_date,
            Some(nov(8)),
            "{group}: the use date asserts on the relative path"
        );
        assert_eq!(
            c.for_sunday_second_dates,
            vec![nov(1)],
            "{group}: the for-date is named"
        );
        let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(c.for_date, Some(nov(8)), "{group}: paren-only");
        assert_eq!(c.for_sunday_second_dates, vec![nov(1)], "{group}");
        let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(c.keep_through, Some(nov(8)), "{group}: keep-through");
        assert_eq!(c.keep_through_second_dates, vec![nov(1)], "{group}");
        let line = "it was moved for November 1, 2026, use November 8, 2026";
        let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(
            c.for_date,
            Some(nov(8)),
            "{line}: the plain twin asserts the use date"
        );
        assert_eq!(c.for_sunday_second_dates, vec![nov(1)], "{line}");
        // Held lines: a "then to" chain still reads
        // by its claim, the lone for-date still
        // contributes, and the bare parity still
        // governs.
        let text = "For Sunday: (it was moved for November 1, then to November 8)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(
            c.for_date,
            Some(nov(8)),
            "the claim still orders a then-to chain"
        );
        assert_eq!(c.for_sunday_second_dates, vec![nov(1)]);
        let text = "For Sunday: (it was moved November 1)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(
            c.for_date, None,
            "the pinned bare same-clause parity stands"
        );
    }

    #[test]
    fn r18b_active_for_names_the_occasion() {
        // Round 18 (R18-9, red team RT18-4): active
        // "for" never named the occasion — "(we moved
        // it for November 1)" was silent, while its
        // passive twin named the date. A displacement
        // verb + "for" + date names the occasion in
        // the active voice too, scoped to the
        // displacement family as the passive rule is
        // scoped to participles.
        for group in [
            "(we moved it for November 1)",
            "(moved it for November 1)",
            "(she moved it for November 1)",
            "(we shifted it for November 1)",
            "(we bumped it for November 1)",
            "(we rescheduled it for November 1)",
            "(we moved the meeting for November 1)",
        ] {
            let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date,
                Some(nov(1)),
                "{group}: the occasion contributes on the relative path"
            );
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, Some(nov(1)), "{group}: paren-only");
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(1)), "{group}: keep-through");
        }
        // A second date beside an active for-date
        // composes by the same primacy as the passive
        // form (R18-5): the for-date asserts, the
        // other is named — where RT18-4 found the
        // for-date dropped off the note entirely.
        let text =
            "For Sunday: (we moved it for November 1, November 8)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(
            c.for_date,
            Some(nov(1)),
            "the active for-date reads by primacy"
        );
        assert_eq!(c.for_sunday_second_dates, vec![nov(8)]);
        // Scope fences: the bare active parity still
        // governs ("for" is the distinguisher), and
        // an active cancellation with "for" is
        // outside the displacement family.
        for group in [
            "(we moved it November 1)",
            "(we cancelled it for November 1)",
        ] {
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, None, "{group}: outside the active-for scope");
        }
    }

    #[test]
    fn r18b_adverb_tolerance_two_and_the_completed_list() {
        // Round 18 (R18-10, red team RT18-5/RT18-6 +
        // review sweep): the tolerance was exactly
        // one adverb and the closed list omitted
        // common forms, so "(it was just recently
        // moved, November 1)" silenced while its
        // has-been twin spent, and "then" / "briefly"
        // broke the slot outright. Tolerance is up
        // to TWO consecutive listed adverbs now, and
        // the list gains then, briefly, shortly,
        // formally, unanimously, happily (24 words).
        let group = "(it was just recently moved, November 1)";
        let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(
            c.for_sunday_parenthesized_date,
            Some(nov(1)),
            "{group}: two stacked adverbs spend on the relative path"
        );
        let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(c.for_date, Some(nov(1)), "{group}: paren-only");
        let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(c.keep_through, Some(nov(1)), "{group}: keep-through");
        for group in [
            "(it was recently just moved, November 1)",
            "(it was just recently cancelled, November 1)",
            "(it has been just recently moved, November 1)",
            "(it was just recently moved for November 1)",
        ] {
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_date,
                Some(nov(1)),
                "{group}: the stack spends in every scan"
            );
        }
        // The completed list: each new word spends in
        // the slot, and the for-zone scan inherits it.
        for group in [
            "(it was then moved, November 1)",
            "(it was briefly moved, November 1)",
            "(it was shortly moved, November 1)",
            "(it was formally moved, November 1)",
            "(it was unanimously moved, November 1)",
            "(it was happily moved, November 1)",
            "(it was briefly moved for November 1)",
            "(it was then moved for November 1)",
        ] {
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, Some(nov(1)), "{group}: a listed adverb spends");
        }
        // The cap is pinned: three stacked listed
        // adverbs still break the tolerance, and an
        // unlisted word still does not spend.
        for group in [
            "(it was still just recently moved, November 1)",
            "(it was just recently finally moved, November 1)",
            "(it was family moved, November 1)",
        ] {
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_date, None,
                "{group}: past the cap, the participle governs"
            );
        }
    }

    #[test]
    fn r18b_split_chain_names_superseded_targets() {
        // Round 18 (R18-11, red team RT18-7): a chain
        // split across two groups asserted correctly
        // but dropped the superseded target unnamed —
        // "(moved to November 8) (then to November
        // 15)" named nothing, where the single-group
        // twin names November 8. Superseded targets
        // are named on the line's note across group
        // boundaries, and they are chain links, not
        // disagreeing garnish: no parenthesized
        // mismatch stands against them.
        let groups = "(moved to November 8) (then to November 15)";
        let text = format!("For Sunday: next Sunday {groups}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(
            c.for_sunday_parenthesized_date,
            Some(nov(15)),
            "{groups}: the last target asserts on the relative path"
        );
        assert_eq!(
            c.for_sunday_second_dates,
            vec![nov(8)],
            "{groups}: the superseded target is named on the relative path"
        );
        let text = format!("For Sunday: {groups}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(c.for_date, Some(nov(15)), "{groups}: paren-only");
        assert_eq!(c.for_sunday_second_dates, vec![nov(8)], "{groups}");
        assert_eq!(
            c.for_sunday_parenthesized_mismatch, None,
            "{groups}: a chain link is not a garnish disagreement"
        );
        let text = format!("Keep-through: {groups}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(c.keep_through, Some(nov(15)), "{groups}: keep-through");
        assert_eq!(c.keep_through_second_dates, vec![nov(8)], "{groups}");
        // The from-date group split: both earlier
        // dates are named, in written order.
        let groups = "(November 1, moved to November 8) (then to November 15)";
        let text = format!("For Sunday: next Sunday {groups}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(c.for_sunday_parenthesized_date, Some(nov(15)), "{groups}");
        assert_eq!(
            c.for_sunday_second_dates,
            vec![nov(1), nov(8)],
            "{groups}: the from-date and the superseded target are both named"
        );
        let text = format!("For Sunday: {groups}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(c.for_date, Some(nov(15)), "{groups}: paren-only");
        assert_eq!(c.for_sunday_second_dates, vec![nov(1), nov(8)], "{groups}");
        assert_eq!(c.for_sunday_parenthesized_mismatch, None, "{groups}");
        let text = format!("Keep-through: {groups}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(c.keep_through, Some(nov(15)), "{groups}: keep-through");
        assert_eq!(
            c.keep_through_second_dates,
            vec![nov(1), nov(8)],
            "{groups}"
        );
        // A three-link split names both superseded
        // targets.
        let text = "For Sunday: (moved to November 8) (then to November 15) (then to November 22)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(nov(22)), "the last link asserts");
        assert_eq!(
            c.for_sunday_second_dates,
            vec![nov(8), nov(15)],
            "both superseded targets are named"
        );
        // Held: the single-group twin reads as it
        // always has, and a bare stack of date groups
        // keeps the garnish-mismatch reading — no
        // target construction, no chain naming.
        let text =
            "For Sunday: (moved to November 8, then to November 15)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(nov(15)), "the single-group twin stands");
        assert_eq!(c.for_sunday_second_dates, vec![nov(8)]);
        let text = "For Sunday: (November 1) (November 8)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(
            c.for_date,
            Some(nov(8)),
            "the bare stack still reads its last group"
        );
        assert_eq!(
            c.for_sunday_second_dates,
            Vec::<SimpleDate>::new(),
            "the bare stack names nothing on the note"
        );
        let mismatch = c
            .for_sunday_parenthesized_mismatch
            .expect("the bare stack keeps its garnish mismatch");
        assert_eq!(
            mismatch.named,
            nov(1),
            "the earlier group is the disagreement"
        );
        assert_eq!(mismatch.read, nov(8));
    }

    #[test]
    fn r12a_a_governor_scopes_to_the_one_date_it_governs() {
        // Round 12 (R12-2, red team RT12-2 + review
        // R12-4): a governor used to scope over the
        // whole rest of its group, so the asserted
        // date after it vanished — not contributed,
        // not an alternative, not a second date,
        // nowhere. A governor now scopes to the one
        // date it governs; a date asserted later in
        // the group is read on its own terms, exactly
        // as if the governed date were absent.
        for group in [
            "(ignore November 1, use November 8)",
            "(not November 1, November 8)",
            "(moved from November 1 to November 8)",
            "(from November 1 to November 8)",
            "(not November 1; use November 8)",
            "(correction: not November 1, November 8)",
        ] {
            let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date,
                Some(nov(8)),
                "{group}: the asserted date is read"
            );
            assert_eq!(
                c.for_sunday_parenthesized_dates,
                vec![nov(8)],
                "{group}: the governed date is not remembered beside it"
            );
            assert_eq!(
                week_verdict(&c, Some(oct(25))),
                WeekVerdict::DifferentSunday(nov(8)),
                "{group}: the asserted date arms on its own terms"
            );
            assert_eq!(
                week_verdict(&c, Some(nov(1))),
                WeekVerdict::DifferentSunday(nov(8)),
                "{group}: never a reading on the governed date"
            );
        }
        // Paren-only and Keep-through paths read the
        // asserted date the same way.
        let text = "For Sunday: (ignore November 1, use November 8)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(nov(8)));
        assert_eq!(
            week_verdict(&c, Some(nov(8))),
            WeekVerdict::SameSunday,
            "the asserted date is the operative date, on its own terms"
        );
        let text = "For Sunday: (moved from November 1 to November 8)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(nov(8)), "moved-from reads its target");
        let text =
            "Keep-through: (moved from November 1 to November 8)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.keep_through, Some(nov(8)));
        // "(moved from X to Y)" in the canonical
        // reschedule shape: X is the origin, Y the
        // assertion.
        let text =
            "For Sunday: next Sunday (moved from October 25 to November 1)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_sunday_parenthesized_date, Some(nov(1)));
        assert_eq!(
            week_verdict(&c, Some(oct(25))),
            WeekVerdict::DifferentSunday(nov(1))
        );
        // PIN MOVED, round 18 (R18-2): this block used
        // to pin the opposite reading — "the same date
        // restated ungoverned after its negation is
        // read by its second standing" (round 12,
        // R12-2). Governance now attaches to the date
        // VALUE, not the occurrence: a date governed in
        // any occurrence does not assert from another
        // occurrence, so "(not November 1, November 1)"
        // reads nothing, exactly as the red team's
        // restated-cancellation witness does. The
        // group contributes no date, and the line
        // stands on its relative words alone.
        let text =
            "For Sunday: next Sunday (not November 1, November 1)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_sunday_parenthesized_date, None);
        assert_eq!(
            week_verdict(&c, Some(oct(25))),
            WeekVerdict::SundayInWords(oct(25))
        );
        // The "or" form of the same correction: the
        // ungoverned offer stands.
        let text =
            "For Sunday: next Sunday (not November 1 or November 8)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_sunday_parenthesized_date, Some(nov(8)));
        // A governor whose clause holds no date
        // governs nothing in the next clause
        // (RT12-2's over-broad edge): "from" governs
        // "notes", and the November 1 in the fresh
        // clause is named, as garnish, not suppressed
        // into silence.
        let text =
            "For Sunday: October 25, 2026 (notes from the pastor, November 1)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(oct(25)));
        let m = c
            .for_sunday_parenthesized_mismatch
            .expect("the fresh-clause date is named, not silently dropped");
        assert_eq!(m.named, nov(1));
        // Held, from round 11's probe controls: a
        // governor with a word of its own to govern
        // inside its clause still governs the date
        // after it in that clause.
        let text = "For Sunday: next Sunday (ignore it: November 1)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_sunday_parenthesized_date, None);
        let text = "For Sunday: next Sunday (not anymore November 1)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_sunday_parenthesized_date, None);
    }

    #[test]
    fn r12a_two_framed_dates_on_a_plain_line_read_the_first() {
        // Round 12 (R12-6, red team RT12-4): a plain
        // For Sunday line naming two dates read
        // NoSundayNamed, and the review's first line
        // said the words do not say which Sunday they
        // are for — false, and Accept stood unarmed.
        // First-date primacy: the first framed date
        // is the line's date, the remaining distinct
        // dates join the second-date note, and the
        // verdict arms like any named Sunday.
        for line in [
            "October 25, 2026 and November 1, 2026",
            "October 25, 2026 / November 1, 2026",
            "October 25, 2026 (Room 2) and November 1, 2026",
        ] {
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, Some(oct(25)), "{line}: the first date reads");
            assert_eq!(
                c.for_sunday_second_dates,
                vec![nov(1)],
                "{line}: the second date is noted"
            );
            assert_eq!(
                second_dates_sentence(&c.for_sunday_second_dates),
                "This file named another date too: Sunday, November 1, 2026. \
                 Check which date was meant.",
                "{line}"
            );
            assert_eq!(
                week_verdict(&c, Some(oct(25))),
                WeekVerdict::SameSunday,
                "{line}"
            );
            assert_eq!(
                week_verdict(&c, Some(nov(1))),
                WeekVerdict::DifferentSunday(oct(25)),
                "{line}: armed — words meant for November 1 no longer land silently"
            );
        }
        // The Keep-through mirror shares the rule.
        let text =
            "Keep-through: October 25, 2026 and November 1, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.keep_through, Some(oct(25)));
        assert_eq!(c.keep_through_second_dates, vec![nov(1)]);
        // Written order decides, not the calendar.
        let text = "For Sunday: November 1, 2026 and October 25, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(nov(1)));
        assert_eq!(c.for_sunday_second_dates, vec![oct(25)]);
        // The same date written twice reads as that
        // date plainly, with nothing noted.
        let text = "For Sunday: October 25, 2026 and October 25, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(oct(25)));
        assert_eq!(c.for_sunday_second_dates, Vec::<SimpleDate>::new());
        assert_eq!(week_verdict(&c, Some(oct(25))), WeekVerdict::SameSunday);
        // NoSundayNamed stays reserved for lines
        // naming no date — and for a line naming
        // only one date in passing words, whose
        // standing readings are unchanged.
        let text = "For Sunday: soon\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, None);
        assert_eq!(week_verdict(&c, Some(oct(25))), WeekVerdict::NoSundayNamed);
        let text = "For Sunday: October 25, 2026 please\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, None, "a single passing date is not this shape");
    }

    #[test]
    fn r13c_mixed_plain_signals_read_the_first_and_note_the_other() {
        // Round 13 (R13-5, red team RT13-4): "October
        // 25, 2026 and next Sunday" read NoSundayNamed
        // with nothing noted — the R12-6 plain
        // multi-date reading covered framed + framed
        // only. The mixed reading stands now: the
        // FIRST signal is the line's date, the other
        // is noted through the second-date machinery.
        // Today is Friday, October 16, 2026, so
        // "this Sunday" resolves to October 18 and
        // "next Sunday" to October 25 — the Sundays
        // the lane fixtures name.
        // Framed first: the framed date reads, and
        // the relative phrase's resolution is noted
        // as a second date.
        let text = "For Sunday: November 1, 2026 and next Sunday\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(nov(1)), "the first signal reads");
        assert_eq!(
            c.for_sunday_second_dates,
            vec![oct(25)],
            "next Sunday resolves to October 25 against today, and is noted"
        );
        assert_eq!(
            second_dates_sentence(&c.for_sunday_second_dates),
            "This file named another date too: Sunday, October 25, 2026. \
             Check which date was meant."
        );
        assert_eq!(week_verdict(&c, Some(nov(1))), WeekVerdict::SameSunday);
        assert_eq!(
            week_verdict(&c, Some(oct(25))),
            WeekVerdict::DifferentSunday(nov(1)),
            "armed — words meant for November 1 no longer land silently"
        );
        let text = "For Sunday: October 25, 2026 and this Sunday\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(oct(25)));
        assert_eq!(
            c.for_sunday_second_dates,
            vec![oct(18)],
            "this Sunday resolves to October 18, the coming Sunday"
        );
        // Relative first: the resolution reads, and
        // the framed date is named beside it as the
        // second date.
        let text = "For Sunday: next Sunday and November 1, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(
            c.for_date,
            Some(oct(25)),
            "the first signal — the relative phrase — reads by its resolution"
        );
        assert_eq!(c.for_sunday_second_dates, vec![nov(1)]);
        assert_eq!(week_verdict(&c, Some(oct(25))), WeekVerdict::SameSunday);
        assert_eq!(
            week_verdict(&c, Some(nov(1))),
            WeekVerdict::DifferentSunday(oct(25))
        );
        let text = "For Sunday: this Sunday and November 1, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(oct(18)));
        assert_eq!(c.for_sunday_second_dates, vec![nov(1)]);
        // A Sunday-after relative phrase is a
        // relative signal too, in either order: its
        // resolution (the Sunday after next Sunday,
        // November 1) is the signal's date.
        let text =
            "For Sunday: October 25, 2026 and the Sunday after next Sunday\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(oct(25)));
        assert_eq!(c.for_sunday_second_dates, vec![nov(1)]);
        let text =
            "For Sunday: the Sunday after next Sunday and October 25, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(nov(1)));
        assert_eq!(c.for_sunday_second_dates, vec![oct(25)]);
        // The same signal twice reads plainly, as
        // the framed twin already does: a relative
        // phrase whose resolution IS the framed date
        // restates it, in either order, and the same
        // relative phrase twice is one signal.
        for line in [
            "October 25, 2026 and next Sunday",
            "next Sunday and October 25, 2026",
            "October 25, 2026 and next Sunday and next Sunday",
            "next Sunday and next Sunday",
        ] {
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, Some(oct(25)), "{line}: reads plainly");
            assert_eq!(
                c.for_sunday_second_dates,
                Vec::<SimpleDate>::new(),
                "{line}: nothing is noted — the signals agree"
            );
            assert_eq!(
                week_verdict(&c, Some(oct(25))),
                WeekVerdict::SameSunday,
                "{line}"
            );
        }
        // The Keep-through mirror shares the rule.
        let text = "Keep-through: November 1, 2026 and next Sunday\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.keep_through, Some(nov(1)));
        assert_eq!(c.keep_through_second_dates, vec![oct(25)]);
        let text = "Keep-through: next Sunday and November 1, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.keep_through, Some(oct(25)));
        assert_eq!(c.keep_through_second_dates, vec![nov(1)]);
        // Held lines stand: a bare relative line
        // keeps its in-words reading (no date is read
        // from the words alone), and differing
        // relative phrases with no framed signal are
        // not this shape either.
        let text = "For Sunday: next Sunday\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, None);
        assert_eq!(c.for_sunday_in_words.as_deref(), Some("next Sunday"));
        let text = "For Sunday: this Sunday and next Sunday\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, None);
        assert_eq!(week_verdict(&c, Some(oct(25))), WeekVerdict::NoSundayNamed);
    }

    #[test]
    fn r14b_mixed_relatives_anchor_at_the_file_date() {
        // Round 14 (R14-6, review R14-6 + red team
        // RT14-5): a mixed line's relative phrases
        // used to resolve against parse day alone.
        // Parsed a week late — today Friday, October
        // 23, 2026, for a file dated Friday, October
        // 16 — "next Sunday and October 25, 2026"
        // read for November 1, the sender's own
        // restated explicit date demoted to the
        // second-date note. Anchored at the file's
        // date, the relative phrase resolves as it
        // did when written: against October 16,
        // "next Sunday" IS October 25, the signals
        // agree, and the line reads plainly.
        let late = oct(23);
        let file_day = oct(16);
        let text = "For Sunday: next Sunday and October 25, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, late).expect("parses");
        assert_eq!(
            c.for_date,
            Some(nov(1)),
            "the unanchored control: a week late, the relative phrase wins"
        );
        assert_eq!(c.for_sunday_second_dates, vec![oct(25)]);
        let c = parse_contribution_anchored(text, late, Some(file_day)).expect("parses");
        assert_eq!(
            c.for_date,
            Some(oct(25)),
            "anchored: the sender's restated date wins"
        );
        assert_eq!(
            c.for_sunday_second_dates,
            Vec::<SimpleDate>::new(),
            "the signals agree at the file's date — nothing is noted"
        );
        assert_eq!(week_verdict(&c, Some(oct(25))), WeekVerdict::SameSunday);
        // The reversed order likewise: anchored, the
        // framed date reads and the relative phrase
        // restates it, so the spurious November 1
        // note the late parse manufactures is gone.
        let text = "For Sunday: October 25, 2026 and next Sunday\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, late).expect("parses");
        assert_eq!(c.for_date, Some(oct(25)));
        assert_eq!(
            c.for_sunday_second_dates,
            vec![nov(1)],
            "the unanchored control: the late resolution is noted"
        );
        let c = parse_contribution_anchored(text, late, Some(file_day)).expect("parses");
        assert_eq!(c.for_date, Some(oct(25)));
        assert_eq!(c.for_sunday_second_dates, Vec::<SimpleDate>::new());
        // Distinct signals stay distinct at the
        // anchor: the relative phrase reads for
        // October 25 with November 1 noted, in
        // either order, on both date lines.
        let text = "For Sunday: next Sunday and November 1, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution_anchored(text, late, Some(file_day)).expect("parses");
        assert_eq!(c.for_date, Some(oct(25)));
        assert_eq!(c.for_sunday_second_dates, vec![nov(1)]);
        let text = "For Sunday: November 1, 2026 and next Sunday\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution_anchored(text, late, Some(file_day)).expect("parses");
        assert_eq!(c.for_date, Some(nov(1)));
        assert_eq!(c.for_sunday_second_dates, vec![oct(25)]);
        let text = "Keep-through: next Sunday and November 1, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution_anchored(text, late, Some(file_day)).expect("parses");
        assert_eq!(c.keep_through, Some(oct(25)));
        assert_eq!(c.keep_through_second_dates, vec![nov(1)]);
        // The Sunday-after witness (RT14-5's second
        // line): anchored, its resolution is the
        // explicit November 1 it stands beside;
        // unanchored a week late it read November 8
        // over that date.
        let text =
            "For Sunday: the Sunday after next Sunday and November 1, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, late).expect("parses");
        assert_eq!(c.for_date, Some(nov(8)), "the unanchored control");
        assert_eq!(c.for_sunday_second_dates, vec![nov(1)]);
        let c = parse_contribution_anchored(text, late, Some(file_day)).expect("parses");
        assert_eq!(c.for_date, Some(nov(1)));
        assert_eq!(c.for_sunday_second_dates, Vec::<SimpleDate>::new());
        // A file date in the future, a file date
        // that is no calendar date, and no file
        // date at all all fall back to today: the
        // anchor never stands in the future.
        let text = "For Sunday: next Sunday and October 25, 2026\n\n## Announcement\n\nHi.\n";
        for bad in [
            Some(oct(30)),
            Some(SimpleDate {
                year: 2026,
                month: 2,
                day: 31,
            }),
            None,
        ] {
            let c = parse_contribution_anchored(text, late, bad).expect("parses");
            assert_eq!(
                c.for_date,
                Some(nov(1)),
                "{bad:?}: falls back to the today reading"
            );
            assert_eq!(c.for_sunday_second_dates, vec![oct(25)], "{bad:?}");
        }
        // Held lines: the anchor touches mixed
        // lines only. A relative-only line keeps
        // its standing reading against today — the
        // restated relative phrase resolves to
        // November 1, the coming "next Sunday" at
        // parse day — a bare relative line still
        // reads in words alone, and differing
        // relatives with no framed signal still
        // name no Sunday.
        let text = "For Sunday: next Sunday and next Sunday\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution_anchored(text, late, Some(file_day)).expect("parses");
        assert_eq!(
            c.for_date,
            Some(nov(1)),
            "relative-only readings are unchanged by the anchor"
        );
        let text = "For Sunday: next Sunday\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution_anchored(text, late, Some(file_day)).expect("parses");
        assert_eq!(c.for_date, None);
        assert_eq!(c.for_sunday_in_words.as_deref(), Some("next Sunday"));
        let text = "For Sunday: this Sunday and next Sunday\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution_anchored(text, late, Some(file_day)).expect("parses");
        assert_eq!(c.for_date, None);
        assert_eq!(week_verdict(&c, Some(oct(25))), WeekVerdict::NoSundayNamed);
        // A framed-only line is unchanged too.
        let text = "For Sunday: October 25, 2026 and November 1, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution_anchored(text, late, Some(file_day)).expect("parses");
        assert_eq!(c.for_date, Some(oct(25)));
        assert_eq!(c.for_sunday_second_dates, vec![nov(1)]);
    }

    #[test]
    fn r15b_file_date_plausibility_bound() {
        // Round 15 (R15-4): a preserved timestamp
        // (a zip/rsync/sync copy keeping an old
        // mtime) can make the file-date anchor WORSE
        // than today — the review lane's witness
        // family, scanned Friday, October 23, 2026,
        // read an unnamed Sunday in the past under
        // old-September anchors. The bound: the file
        // date anchors only when it is at most 28
        // days before today; older falls back to
        // today, exactly like missing/invalid.
        let late = oct(23);
        let text = "For Sunday: next Sunday and October 25, 2026\n\n## Announcement\n\nHi.\n";
        let plain = parse_contribution(text, late).expect("parses");
        assert_eq!(plain.for_date, Some(nov(1)), "the unanchored control");
        assert_eq!(plain.for_sunday_second_dates, vec![oct(25)]);
        // Exactly 28 days before today — Friday,
        // September 25, 2026 — anchors: at that
        // date "next Sunday" is October 4, and the
        // sender's explicit October 25 is noted.
        let c = parse_contribution_anchored(text, late, Some(sep(25))).expect("parses");
        assert_eq!(
            c.for_date,
            Some(oct(4)),
            "a file date exactly 28 days before today anchors"
        );
        assert_eq!(c.for_sunday_second_dates, vec![oct(25)]);
        // 29 days before today — September 24 —
        // falls back: the parse is the unanchored
        // parse, field for field.
        let c = parse_contribution_anchored(text, late, Some(sep(24))).expect("parses");
        assert_eq!(
            c, plain,
            "a file date 29 days before today falls back to today"
        );
        assert_eq!(c.for_date, Some(nov(1)));
        // The witness family itself: an
        // old-September file date (September 20,
        // 33 days before the October scan) reads
        // exactly as the unanchored parse does.
        let c = parse_contribution_anchored(text, late, Some(sep(20))).expect("parses");
        assert_eq!(c, plain, "the old-September witness falls back to today");
        // The Keep-through mirror shares the bound
        // through the same anchor resolution.
        let kt_text = "Keep-through: next Sunday and November 1, 2026\n\n## Announcement\n\nHi.\n";
        let kt_plain = parse_contribution(kt_text, late).expect("parses");
        assert_eq!(kt_plain.keep_through, Some(nov(1)));
        assert_eq!(kt_plain.keep_through_second_dates, Vec::<SimpleDate>::new());
        let c = parse_contribution_anchored(kt_text, late, Some(sep(25))).expect("parses");
        assert_eq!(
            c.keep_through,
            Some(oct(4)),
            "28 days before anchors here too"
        );
        assert_eq!(c.keep_through_second_dates, vec![nov(1)]);
        let c = parse_contribution_anchored(kt_text, late, Some(sep(24))).expect("parses");
        assert_eq!(c, kt_plain, "29 days before falls back here too");
    }

    #[test]
    fn r14b_same_week_file_date_is_identical() {
        // Round 14 (R14-6): when the file's date IS
        // today — the normal same-week case — the
        // anchored parse is the standing parse,
        // field for field, across the round-13
        // mixed-signal family and its held lines.
        for line in [
            "November 1, 2026 and next Sunday",
            "October 25, 2026 and this Sunday",
            "next Sunday and November 1, 2026",
            "this Sunday and November 1, 2026",
            "October 25, 2026 and the Sunday after next Sunday",
            "the Sunday after next Sunday and October 25, 2026",
            "October 25, 2026 and next Sunday",
            "next Sunday and October 25, 2026",
            "next Sunday and next Sunday",
            "October 25, 2026 and November 1, 2026",
            "next Sunday",
            "this Sunday and next Sunday",
        ] {
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let plain = parse_contribution(&text, today()).expect("parses");
            let anchored =
                parse_contribution_anchored(&text, today(), Some(today())).expect("parses");
            assert_eq!(plain, anchored, "{line}: mtime == today changes nothing");
        }
        let text = "Keep-through: next Sunday and November 1, 2026\n\n## Announcement\n\nHi.\n";
        let plain = parse_contribution(text, today()).expect("parses");
        let anchored = parse_contribution_anchored(text, today(), Some(today())).expect("parses");
        assert_eq!(plain, anchored, "the Keep-through mirror too");
    }

    #[test]
    fn r15b_different_day_same_week_file_date_is_identical() {
        // Round 15 (R15-5): the r14b pin above
        // exercises only anchor == today, where
        // identity holds by construction. A
        // DIFFERENT-day same-week anchor — file
        // dates Monday, October 12 through
        // Thursday, October 15, 2026, against
        // today Friday, October 16 — parses
        // field-for-field identically too, over
        // the same round-13 family, because
        // relative resolutions are constant
        // within a relative week (the review
        // lane's probe verified 0/14 diffs).
        for file_day in [oct(12), oct(13), oct(14), oct(15)] {
            for line in [
                "November 1, 2026 and next Sunday",
                "October 25, 2026 and this Sunday",
                "next Sunday and November 1, 2026",
                "this Sunday and November 1, 2026",
                "October 25, 2026 and the Sunday after next Sunday",
                "the Sunday after next Sunday and October 25, 2026",
                "October 25, 2026 and next Sunday",
                "next Sunday and October 25, 2026",
                "next Sunday and next Sunday",
                "October 25, 2026 and November 1, 2026",
                "next Sunday",
                "this Sunday and next Sunday",
            ] {
                let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
                let plain = parse_contribution(&text, today()).expect("parses");
                let anchored =
                    parse_contribution_anchored(&text, today(), Some(file_day)).expect("parses");
                assert_eq!(
                    plain, anchored,
                    "{line}: a same-week file date ({file_day:?}) changes nothing"
                );
            }
            let text = "Keep-through: next Sunday and November 1, 2026\n\n## Announcement\n\nHi.\n";
            let plain = parse_contribution(text, today()).expect("parses");
            let anchored =
                parse_contribution_anchored(text, today(), Some(file_day)).expect("parses");
            assert_eq!(
                plain, anchored,
                "the Keep-through mirror too ({file_day:?})"
            );
        }
    }

    #[test]
    fn r12a_trailing_governors_and_nested_groups() {
        // Round 12 (R12-7, review R12-5 + red team
        // RT12-5): (a) a trailing governor word in the
        // same group clause, with no later date to
        // govern, disqualifies the date it follows;
        // (b) the group content reading sees through
        // balanced inner parentheses.
        for group in [
            "(November 1, cancelled)",
            "(November 1, canceled)",
            "(November 1, no longer)",
            "(November 1, not)",
        ] {
            let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date, None,
                "{group}: the trailing governor disqualifies its date"
            );
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, None, "{group}: paren-only");
            assert_eq!(
                week_verdict(&c, Some(nov(1))),
                WeekVerdict::NoSundayNamed,
                "{group}: never SameSunday on a cancelled date"
            );
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, None, "{group}: keep-through");
        }
        // A cancellation spent, the date after it in
        // a fresh clause is read on its own terms.
        let text = "For Sunday: (November 1, cancelled, November 8)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(nov(8)));
        // The held case stands exactly: the trailing
        // "not" governs the date after it, and the
        // date before it is untouched.
        let text =
            "For Sunday: next Sunday (November 1, not October 25)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_sunday_parenthesized_date, Some(nov(1)));
        assert_eq!(
            week_verdict(&c, Some(oct(25))),
            WeekVerdict::DifferentSunday(nov(1))
        );
        let text = "For Sunday: (November 1, not October 25)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(nov(1)), "paren-only reads November 1");
        // (b) Nested groups: the inner content is the
        // group's content for date-reading purposes.
        let text = "For Sunday: ((November 1))\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(nov(1)), "((November 1)) reads its date");
        assert_eq!(c.for_sunday_parenthesized_mismatch, None);
        assert_eq!(week_verdict(&c, Some(nov(1))), WeekVerdict::SameSunday);
        assert_eq!(
            week_verdict(&c, Some(oct(25))),
            WeekVerdict::DifferentSunday(nov(1))
        );
        let text = "Keep-through: ((November 1))\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.keep_through, Some(nov(1)));
        let text = "For Sunday: next Sunday ((November 1))\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_sunday_parenthesized_date, Some(nov(1)));
        // A nested governor still governs through
        // the inner parentheses.
        let text = "For Sunday: next Sunday (note (not November 1))\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_sunday_parenthesized_date, None);
        let text = "For Sunday: next Sunday ((not) November 1)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_sunday_parenthesized_date, None);
    }

    #[test]
    fn r12a_sunday_after_phrases_consume_their_runs_wherever_they_stand() {
        // Round 12 (R12-9a, red team RT12-6): a
        // second "Sunday after" phrase's resolution
        // was never noted — the scan treated its
        // base as the read date's own restatement —
        // and a base restated in plain words WAS
        // noted as "another date", the same date
        // called both base and another date on one
        // line. The phrase now consumes its run
        // wherever it stands, and its resolution is
        // noted like the first phrase's.
        let text = "For Sunday: Sunday after October 18, 2026 and Sunday after October 25, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(oct(25)), "the first phrase reads");
        assert_eq!(
            c.for_sunday_second_dates,
            vec![nov(1)],
            "the second phrase's resolution (the Sunday after October 25) is noted"
        );
        assert_eq!(
            second_dates_sentence(&c.for_sunday_second_dates),
            "This file named another date too: Sunday, November 1, 2026. \
             Check which date was meant."
        );
        assert_eq!(week_verdict(&c, Some(oct(25))), WeekVerdict::SameSunday);
        // A run equal to the standing base is a
        // restatement, never another date: the base
        // (October 20) is not noted, and neither is
        // the read date (October 25) beside it.
        let text = "For Sunday: Sunday after October 20, 2026; October 20, 2026 minutes, service October 25, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(oct(25)));
        assert_eq!(
            c.for_sunday_second_dates,
            Vec::<SimpleDate>::new(),
            "the restated base is not \"another date\""
        );
        // The same phrase twice: one resolution,
        // standing as the read date, and its base is
        // consumed both times — nothing is noted.
        let text = "For Sunday: Sunday after October 25, 2026 and Sunday after October 25, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(nov(1)));
        assert_eq!(c.for_sunday_second_dates, Vec::<SimpleDate>::new());
        // Held controls from round 11 stand: two
        // runs after a phrase note exactly the
        // further date, and a base standing after a
        // plainly read date is not flagged.
        let text = "For Sunday: Sunday after October 18, 2026, then October 25, 2026 and November 1, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(oct(25)));
        assert_eq!(c.for_sunday_second_dates, vec![nov(1)]);
        let text =
            "For Sunday: November 1, 2026, the Sunday after October 25, 2026\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, Some(nov(1)));
        assert_eq!(c.for_sunday_second_dates, Vec::<SimpleDate>::new());
    }

    #[test]
    fn r21b_verb_test_v2_subject_is_a_run() {
        // Round 21 (R21-6, review F3 + red team
        // RT21-7): the one verb test's subject was
        // exactly ONE noun-like word, so any real
        // clause with a multi-word subject — a
        // modifier, a compound head, a title, a
        // subject wearing a phrase — was crossed
        // and November 1 was named as a false
        // from-date. Under the verb test v2 the
        // subject is a RUN of determiner / modifier
        // / noun-like words ending at the verb:
        // every shape below ends the reach, on
        // every path — November 8 stands alone and
        // November 1 is NOT named.
        for group in [
            "(moved November 1, the big choir sang to November 8)",
            "(moved November 1, big choir sang to November 8)",
            "(moved November 1, choir members sang to November 8)",
            "(moved November 1, the choir members sang to November 8)",
            "(moved November 1, visiting choir sang to November 8)",
            "(moved November 1, the youth choir sang to November 8)",
            "(moved November 1, the handbell choir rang to November 8)",
            "(moved November 1, handbell choir rang to November 8)",
            "(moved November 1, Pastor Lee sang to November 8)",
            "(moved November 1, Mr Johnson played to November 8)",
            "(moved November 1, Dr Lee sang to November 8)",
            "(moved November 1, choir 2 sang to November 8)",
            "(moved November 1, the choir in the hall sang to November 8)",
            // The comma variant (F3): the comma
            // after the subject is a clause break,
            // not postposed-modifier punctuation —
            // "sang" is a real clause's verb.
            "(moved November 1, the choir members, sang to November 8)",
        ] {
            let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date,
                Some(nov(8)),
                "{group}: November 8 stands alone on the relative path"
            );
            assert_eq!(
                c.for_sunday_second_dates,
                Vec::<SimpleDate>::new(),
                "{group}: November 1 is not named as a from-date"
            );
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, Some(nov(8)), "{group}: paren-only");
            assert_eq!(
                c.for_sunday_second_dates,
                Vec::<SimpleDate>::new(),
                "{group}: paren-only note is empty"
            );
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(8)), "{group}: keep-through");
            assert_eq!(
                c.keep_through_second_dates,
                Vec::<SimpleDate>::new(),
                "{group}: keep-through note is empty"
            );
        }
        // The plain twin: November 8 asserts; the
        // plain composition names the governed
        // November 1 on its note, as it names
        // every governed date beside an asserting
        // one (the group paths above name nothing,
        // by the standing group naming rule).
        let line = "moved November 1, 2026, the big choir sang to November 8, 2026";
        let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(c.for_date, Some(nov(8)), "{line}: the plain twin asserts");
        assert_eq!(c.for_sunday_second_dates, vec![nov(1)], "{line}");
        let text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(c.keep_through, Some(nov(8)), "{line}: keep-through");
        assert_eq!(c.keep_through_second_dates, vec![nov(1)], "{line}");
    }

    #[test]
    fn r21b_verb_test_v2_subjectless_clauses_and_auxiliaries() {
        // Round 21 (R21-6, review F4 + red team
        // RT21-6): subjectless verb forms with
        // clause shapes claimed and named the
        // from-date, and a past-progressive second
        // clause silenced both dates. Under v2 a
        // verb-form word with no subject before it
        // is still finite when it cannot head a
        // noun phrase there — an irregular past
        // form, a form followed by a determiner or
        // an adverb, a bare form with no noun
        // phrase around it — and an auxiliary with
        // a subject before it in its segment is
        // itself a finite verb. Every shape below
        // ends the reach: November 8 stands alone,
        // November 1 is NOT named, on every path.
        for group in [
            // Irregular past forms, with and
            // without an object.
            "(moved November 1, sang hymns to November 8)",
            "(moved November 1, then sang to November 8)",
            // A determiner object, an adverb
            // object, a bare regular verb.
            "(moved November 1, folded the bulletins to November 8)",
            "(moved November 1, played beautifully to November 8)",
            "(moved November 1, then played to November 8)",
            "(moved November 1, and played to November 8)",
            // The past-progressive second clause:
            // "was" heads its own clause and is
            // not a bare past marker governing
            // the date its clause names.
            "(moved November 1, choir was singing to November 8)",
            "(moved November 1, choir was singing hymns to November 8)",
            "(moved November 1, choir was playing to November 8)",
            "(moved November 1, choir was rehearsing to November 8)",
        ] {
            let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date,
                Some(nov(8)),
                "{group}: November 8 stands alone on the relative path"
            );
            assert_eq!(
                c.for_sunday_second_dates,
                Vec::<SimpleDate>::new(),
                "{group}: November 1 is not named as a from-date"
            );
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, Some(nov(8)), "{group}: paren-only");
            assert_eq!(
                c.for_sunday_second_dates,
                Vec::<SimpleDate>::new(),
                "{group}: paren-only note is empty"
            );
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(8)), "{group}: keep-through");
            assert_eq!(
                c.keep_through_second_dates,
                Vec::<SimpleDate>::new(),
                "{group}: keep-through note is empty"
            );
        }
        // The plain twin of the progressive: the
        // line asserts November 8 instead of
        // reading nothing on both paths.
        let line = "moved November 1, 2026, choir was singing to November 8, 2026";
        let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(c.for_date, Some(nov(8)), "{line}: the plain twin asserts");
        assert_eq!(c.for_sunday_second_dates, vec![nov(1)], "{line}");
        let text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(c.keep_through, Some(nov(8)), "{line}: keep-through");
        assert_eq!(c.keep_through_second_dates, vec![nov(1)], "{line}");
        // Held: the modifier side of the v2 line —
        // a REGULAR form followed directly by a
        // bare noun-like word, with no subject
        // before it, heads that noun phrase and
        // the reach continues: the claim stands
        // and November 1 is named. "played hymns"
        // is the "baked beans" shape with a listed
        // stem; the irregular twin "sang hymns"
        // above is the clause, and the spelling
        // is the distinction the test can see.
        for group in [
            "(moved November 1, baked beans to November 8)",
            "(moved November 1, planned dinner to November 8)",
            "(moved November 1, planning meeting to November 8)",
            "(moved November 1, played hymns to November 8)",
        ] {
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_date,
                Some(nov(8)),
                "{group}: the target asserts on the paren-only path"
            );
            assert_eq!(
                c.for_sunday_second_dates,
                vec![nov(1)],
                "{group}: November 1 is named as the from-date"
            );
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(8)), "{group}: keep-through");
            assert_eq!(
                c.keep_through_second_dates,
                vec![nov(1)],
                "{group}: keep-through names the from-date"
            );
        }
    }

    #[test]
    fn r21b_plain_weekday_modifier_is_not_a_signal() {
        // Round 21 (R21-7, review F2 = red team
        // RT21-4): the R20-6 weekday-modifier rule
        // was scoped to groups, so on a plain line
        // "Sunday school, November 8, 2026" filed
        // for a phantom October 18 — the bare
        // "Sunday" spawned a this-Sunday relative
        // signal that outranked the framed date
        // after it. The rule now applies in
        // `scan_plain_signals` too: a weekday word
        // followed by a noun-like word is a
        // modifier, never a relative signal, and
        // the framed date the phrase garnishes
        // asserts by the plain primacy.
        for (line, want) in [
            ("Sunday school, November 8, 2026", nov(8)),
            ("Sunday bulletins, November 1, 2026", nov(1)),
            ("the Sunday bulletins, November 1, 2026", nov(1)),
            ("printed Sunday bulletins, November 8, 2026", nov(8)),
            ("Sunday morning, November 8, 2026", nov(8)),
            // The weekday rule is the general
            // R20-6 rule, not a Sunday special
            // case.
            ("printed Monday bulletins, November 8, 2026", nov(8)),
        ] {
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_date,
                Some(want),
                "{line}: the framed date asserts, no phantom Sunday"
            );
            assert_eq!(
                c.for_sunday_second_dates,
                Vec::<SimpleDate>::new(),
                "{line}: no October 18 is named"
            );
            let text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(want), "{line}: keep-through");
            assert_eq!(
                c.keep_through_second_dates,
                Vec::<SimpleDate>::new(),
                "{line}: keep-through names no phantom"
            );
        }
        // The naming half: where a claim or a
        // first framed date saves the asserted
        // date, the phantom is not named either.
        let line =
            "moved the meeting, November 1, 2026, and printed Sunday bulletins to November 8, 2026";
        let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(c.for_date, Some(nov(8)), "{line}: the target asserts");
        assert_eq!(
            c.for_sunday_second_dates,
            vec![nov(1)],
            "{line}: November 1 is named, October 18 is not"
        );
        let text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(c.keep_through, Some(nov(8)), "{line}: keep-through");
        assert_eq!(c.keep_through_second_dates, vec![nov(1)], "{line}");
        let line = "November 1, 2026, printed Sunday bulletins, November 8, 2026";
        let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(
            c.for_date,
            Some(nov(1)),
            "{line}: the first framed date stands"
        );
        assert_eq!(
            c.for_sunday_second_dates,
            vec![nov(8)],
            "{line}: November 8 is named, October 18 is not"
        );
        // Held: the framing control and the true
        // relative phrases keep their standing
        // readings on plain lines.
        for (line, want) in [
            ("Sunday, November 8, 2026", nov(8)),
            ("next Sunday, November 8, 2026", nov(8)),
            ("this Sunday, November 8, 2026", nov(8)),
            (
                "Sunday morning service, November 1, 2026, November 8, 2026",
                nov(1),
            ),
        ] {
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, Some(want), "{line}: standing reading");
        }
        // Held: the modifier fence did not widen
        // the plain readings — a lone framed date
        // under ordinary content words still reads
        // nothing, as a content-headed line alone
        // always has.
        let line = "choir practice, November 8, 2026";
        let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(c.for_date, None, "{line}: no weekday phrase, no reading");
    }

    #[test]
    fn r21b_bare_inventory_function_words() {
        // Round 21 (R21-8, red team RT21-5 +
        // review F7): the bare inventory still
        // missed the function-word classes —
        // conjunction / discourse heads, trailing
        // discourse words, the adverb "only", the
        // preposition family, and the possessive
        // pronouns — so a governed date restated
        // behind any of them resurrected on every
        // path. The inventory's rule is general
        // (see `bare_filler_word`): function words
        // — words that name no event — are bare;
        // content labels are not. Every witness
        // below reads nothing on every path.
        for word in [
            "so", "well", "however", "only", "until", "before", "after", "through", "circa",
            "about", "around", "near", "mine", "ours", "yours", "hers", "theirs",
        ] {
            let groups = format!("(November 1, cancelled) ({word} November 1)");
            let text = format!("For Sunday: next Sunday {groups}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date, None,
                "{groups}: relative path stays governed"
            );
            let text = format!("For Sunday: {groups}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, None, "{groups}: paren-only stays governed");
            let text = format!("Keep-through: {groups}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.keep_through, None,
                "{groups}: keep-through stays governed"
            );
        }
        // A listed word behind a discourse head
        // binds too ("so" poisoned "please").
        let groups = "(November 1, cancelled) (so please November 1)";
        let text = format!("For Sunday: {groups}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(c.for_date, None, "{groups}: paren-only stays governed");
        // The trailing slot: "too" and "thanks"
        // standing after the date are bare there.
        for word in ["too", "thanks"] {
            let groups = format!("(November 1, cancelled) (November 1 {word})");
            let text = format!("For Sunday: next Sunday {groups}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date, None,
                "{groups}: relative path stays governed"
            );
            let text = format!("For Sunday: {groups}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, None, "{groups}: paren-only stays governed");
            let text = format!("Keep-through: {groups}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.keep_through, None,
                "{groups}: keep-through stays governed"
            );
        }
        // The plain twins read nothing either.
        for line in [
            "November 1, 2026, cancelled, so November 1, 2026",
            "November 1, 2026, cancelled, November 1, 2026 too",
        ] {
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, None, "{line}: the plain twin stays governed");
            let text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, None, "{line}: keep-through");
        }
        // Held (the R20-3 adjudication, not
        // moved): the label heads stay content and
        // assert — "thanks" in HEAD position among
        // them — and a possessive-headed event
        // asserts as it always has.
        for groups in [
            "(November 1, cancelled) (reminder November 1)",
            "(November 1, cancelled) (thanks November 1)",
            "(November 1, cancelled) (our choir November 1)",
        ] {
            let text = format!("For Sunday: {groups}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_date,
                Some(nov(1)),
                "{groups}: a label asserts beside a sibling cancellation"
            );
            let text = format!("Keep-through: {groups}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(1)), "{groups}: keep-through");
        }
    }

    #[test]
    fn r21b_claim_fronted_stack_before_an_intervening_date() {
        // Round 21 (R21-9, review F6): on the
        // claim scan, a fronted adverb stack
        // standing in its own clause before an
        // intervening date escaped the unified
        // cap — the claim's extra fronted check
        // examined only the fragment immediately
        // before the bound (the from-date's own
        // fragment) and never the stack's clause
        // one break earlier. The shared walk now
        // applies there too: same list, same
        // two-word cap, fronted position. A
        // fronted three-stack does not spend; the
        // claim dies and the pair stands as
        // alternatives, as an unclaimed pair with
        // no governed date does.
        for group in [
            "(just really truly, November 1, moved to November 8)",
            // An unlisted fronted adverb does not
            // spend either, in this position as in
            // every other.
            "(yesterday, November 1, moved to November 8)",
        ] {
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_date, None,
                "{group}: the fronted stack does not spend the claim"
            );
            assert_eq!(
                c.for_sunday_second_dates,
                vec![nov(1), nov(8)],
                "{group}: both dates are offered as alternatives"
            );
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, None, "{group}: keep-through");
            assert_eq!(
                c.keep_through_second_dates,
                vec![nov(1), nov(8)],
                "{group}: keep-through offers both dates"
            );
        }
        // A fronted listed one- or two-stack in
        // the same position spends as it does in
        // every other position: the claim stands.
        for group in [
            "(just, November 1, moved to November 8)",
            "(just truly, November 1, moved to November 8)",
        ] {
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_date,
                Some(nov(8)),
                "{group}: the fronted stack spends, the target asserts"
            );
            assert_eq!(
                c.for_sunday_second_dates,
                vec![nov(1)],
                "{group}: November 1 is named as the from-date"
            );
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(8)), "{group}: keep-through");
            assert_eq!(
                c.keep_through_second_dates,
                vec![nov(1)],
                "{group}: keep-through names the from-date"
            );
        }
    }

    #[test]
    fn r21b_postposed_exemption_fenced_to_its_shape() {
        // Round 21 (R21-10, red team RT21-8): the
        // postposed-modifier clause-break
        // exemption plus a two-noun segment let
        // the plain path file the vacated date —
        // "(moved the meeting, November 1, 2026,
        // and beans, choir baked to November 8)"
        // offered both dates as alternatives in
        // the group and filed November 1 (the
        // date the meeting moved FROM) on the
        // plain path. The exemption is now
        // fenced to the pinned postposed shape:
        // it judges the verb form the comma's
        // continuation contains — reached across
        // the noun-like words of its own run —
        // and exempts the comma only when the one
        // verb test reads that form as a postposed
        // modifier in its segment, exactly as for
        // the directly postposed form. The
        // witness files its target date on every
        // path, the vacated date named as the
        // claim's from-date.
        let group = "(moved the meeting, November 1, 2026, and beans, choir baked to November 8)";
        let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(
            c.for_sunday_parenthesized_date,
            Some(nov(8)),
            "{group}: the target asserts on the relative path"
        );
        assert_eq!(
            c.for_sunday_second_dates,
            vec![nov(1)],
            "{group}: November 1 is named as the from-date"
        );
        let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(c.for_date, Some(nov(8)), "{group}: paren-only");
        assert_eq!(c.for_sunday_second_dates, vec![nov(1)], "{group}");
        let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(c.keep_through, Some(nov(8)), "{group}: keep-through");
        assert_eq!(c.keep_through_second_dates, vec![nov(1)], "{group}");
        // The plain twin files the target too.
        let line =
            "moved the meeting, November 1, 2026, and beans, choir baked to November 8, 2026";
        let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(
            c.for_date,
            Some(nov(8)),
            "{line}: the plain twin files the target date, not the vacated one"
        );
        assert_eq!(c.for_sunday_second_dates, vec![nov(1)], "{line}");
        let text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(c.keep_through, Some(nov(8)), "{line}: keep-through");
        assert_eq!(c.keep_through_second_dates, vec![nov(1)], "{line}");
        // The same shape in miniature, where
        // November 1 is governed by the bare
        // "moved": the object-list continuation
        // claims by the same fence.
        let group = "(moved November 1, and beans, choir baked to November 8)";
        let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(c.for_date, Some(nov(8)), "{group}: the target asserts");
        assert_eq!(
            c.for_sunday_second_dates,
            vec![nov(1)],
            "{group}: November 1 is named as the from-date"
        );
        // The fence's other side: without the
        // object-list conjunction the segment is
        // judged as the clause it is — "baked"
        // has its subject run ("beans, choir")
        // and is finite under the v2 test, the
        // comma keeps its clause-break force, and
        // the reach ends (RT21-8's F43 bound).
        let group = "(moved November 1, beans, choir baked to November 8)";
        let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(
            c.for_date,
            Some(nov(8)),
            "{group}: November 8 stands on its own"
        );
        assert_eq!(
            c.for_sunday_second_dates,
            Vec::<SimpleDate>::new(),
            "{group}: the reach ended, so November 1 is not named as a from-date"
        );
    }

    #[test]
    fn r22a_separators_between_groups_compose() {
        // Round 22 (R22-1, red team RT22-1): a groups-only
        // line whose groups are separated by ";", "-", or
        // plain words read nothing on the For Sunday line
        // while Keep-through asserted the last group —
        // the de-grouped core (";", "note", ", cancelled,")
        // was non-empty, so the paren-only reading was
        // never taken. Separators are separators: the
        // groups compose exactly as adjacent groups do,
        // on both paths, the earlier group named on the
        // parenthesized record as the paren-only
        // composition names it.
        for (line, want) in [
            ("(November 8); (November 15)", nov(15)),
            ("(November 8) - (November 15)", nov(15)),
            ("(November 8) note (November 15)", nov(15)),
            ("(November 1); (next Sunday)", oct(25)),
            ("(November 1) - (next Sunday)", oct(25)),
            ("(November 1) note (November 8)", nov(8)),
            ("(November 1) cancelled (service November 1)", nov(1)),
            (
                "(service November 1), cancelled, (service November 1)",
                nov(1),
            ),
        ] {
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, Some(want), "{line}: For Sunday");
            let text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(want), "{line}: Keep-through agrees");
        }
        // Field-for-field the paren-only composition:
        // the last group stands, the earlier group is
        // named on the mismatch record, and a relative
        // loser is named on the note.
        let line = "(November 8); (November 15)";
        let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        let mismatch = c
            .for_sunday_parenthesized_mismatch
            .expect("the earlier group is named");
        assert_eq!(mismatch.named, nov(8), "{line}: November 8 named");
        assert_eq!(mismatch.read, nov(15), "{line}: November 15 stands");
        let line = "(November 1); (next Sunday)";
        let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(
            c.for_sunday_second_dates,
            vec![nov(1)],
            "{line}: the framed loser is named on the note"
        );
        let text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(
            c.keep_through_second_dates,
            vec![nov(1)],
            "{line}: Keep-through names it too"
        );
    }

    #[test]
    fn r22a_separator_after_a_leading_group_keeps_relative_memory() {
        // Round 22 (R22-1, red team RT22-2): a comma or
        // semicolon after a leading group destroyed the
        // For line's relative memory entirely — the shape
        // readers matched the phrase only when the group
        // was followed by a bare space, so the verdict
        // could not arm. The punctuated lines keep the
        // in-words reading their space controls have.
        for line in ["(November 1); next Sunday", "(November 1), next Sunday"] {
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, None, "{line}: no hard date");
            assert_eq!(
                c.for_sunday_in_words.as_deref(),
                Some(line),
                "{line}: the sender's words are remembered as written"
            );
            assert_eq!(
                c.for_sunday_parenthesized_date,
                Some(nov(1)),
                "{line}: the group is remembered with the phrase"
            );
            assert_eq!(
                week_verdict(&c, Some(oct(25))),
                WeekVerdict::DifferentSunday(nov(1)),
                "{line}: the verdict arms against the group, as the space twin's does"
            );
            let text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.keep_through,
                Some(nov(1)),
                "{line}: Keep-through asserts the group, as the space twin's does"
            );
        }
        for line in [
            "(November 1); this Sunday",
            "(November 1); (November 8) next Sunday",
        ] {
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_in_words.as_deref(),
                Some(line),
                "{line}: remembered in words"
            );
        }
        let line = "(November 1); (November 8) next Sunday";
        let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(
            c.for_sunday_parenthesized_dates,
            vec![nov(1), nov(8)],
            "{line}: every group on the line is remembered"
        );
        let text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(
            c.keep_through,
            Some(nov(8)),
            "{line}: the last group stands"
        );
        // A leading relative group before a plain date
        // keeps the reading its space control has.
        for line in [
            "(next Sunday), November 1, 2026",
            "(next Sunday); November 1, 2026",
        ] {
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, Some(nov(1)), "{line}: For Sunday");
            let text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(1)), "{line}: Keep-through agrees");
        }
    }

    #[test]
    fn r22a_a_senders_semicolon_is_a_clause_break() {
        // Round 22 (R22-2, review F3 = red team RT22-3):
        // a sender's semicolon severed a date from its
        // postposed governor on the composition paths —
        // attached to the year ("2026;") it also dropped
        // the year out of the date's own span, and the
        // stray year token broke the governor scope
        // behind it. A semicolon is a clause break
        // everywhere governance is judged, attached or
        // spaced, indistinguishable from the composition's
        // internal group marker: every witness reads as
        // its comma twin reads, on both paths.
        for (line, want, want_second) in [
            (
                "November 1, 2026; cancelled, November 8, 2026",
                Some(nov(8)),
                vec![nov(1)],
            ),
            (
                "November 1, 2026; postponed, November 8, 2026",
                Some(nov(8)),
                vec![nov(1)],
            ),
            (
                "November 1, 2026; cancelled (November 8)",
                Some(nov(8)),
                vec![nov(1)],
            ),
            (
                "(November 8) November 1, 2026; cancelled",
                Some(nov(8)),
                vec![nov(1)],
            ),
            (
                "November 1, 2026, cancelled (November 8) November 15, 2026; cancelled",
                Some(nov(8)),
                vec![nov(1), nov(15)],
            ),
            (
                "November 1, 2026; cancelled, November 1, 2026",
                None,
                Vec::new(),
            ),
            ("November 1, 2026; not November 1, 2026", None, Vec::new()),
            ("November 1, 2026; cancelled", None, Vec::new()),
            (
                "November 1, 2026 ; cancelled, November 1, 2026",
                None,
                Vec::new(),
            ),
            ("(November 1; cancelled, November 1)", None, Vec::new()),
        ] {
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, want, "{line}: For Sunday");
            assert_eq!(
                c.for_sunday_second_dates, want_second,
                "{line}: For Sunday second dates"
            );
            let text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, want, "{line}: Keep-through agrees");
            assert_eq!(
                c.keep_through_second_dates, want_second,
                "{line}: Keep-through second dates agree"
            );
        }
    }

    #[test]
    fn r22a_wrong_weekday_behind_a_leading_group() {
        // Round 22 (R22-3, red team RT22-4): a framed
        // date whose weekday is wrong yields no date —
        // except behind a leading group, where it
        // asserted on both paths with no mismatch
        // record, because the de-grouped core's inner
        // reader treated the weekday as ordinary text
        // after the group. The check applies to every
        // framed occurrence however the line is
        // composed: no segment position exempts it.
        for (line, named, date) in [
            ("(November 8) Monday, November 15, 2026", "Monday", nov(15)),
            ("(November 15) Monday, November 8, 2026", "Monday", nov(8)),
            (
                "(November 8) Tuesday, November 15, 2026",
                "Tuesday",
                nov(15),
            ),
            (
                "(November 8) Monday, November 15, 2026 (November 22)",
                "Monday",
                nov(15),
            ),
            (
                "(November 8) on Monday, November 15, 2026",
                "Monday",
                nov(15),
            ),
        ] {
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, None, "{line}: no date on the For line");
            let mismatch = c
                .named_weekday_mismatch
                .as_ref()
                .expect("the mismatch is recorded on the For line");
            assert_eq!(mismatch.named, named, "{line}: the weekday is named");
            assert_eq!(mismatch.date, date, "{line}: the date it framed");
            assert_eq!(
                week_verdict(&c, Some(oct(25))),
                WeekVerdict::NoSundayNamed,
                "{line}: the verdict stays unarmed"
            );
            let text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, None, "{line}: no date on the Keep line");
            let mismatch = c
                .keep_through_weekday_mismatch
                .as_ref()
                .expect("the mismatch is recorded on the Keep line");
            assert_eq!(mismatch.named, named, "{line}: Keep names the weekday");
            assert_eq!(mismatch.date, date, "{line}: Keep names the date");
        }
        // Held (R21-A): correct-weekday framed dates
        // behind leading groups assert as they always
        // have, on both paths.
        for (line, want) in [
            ("(November 8) Sunday, November 15, 2026", nov(15)),
            ("(November 15) Sunday, November 8, 2026", nov(8)),
        ] {
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, Some(want), "{line}: For Sunday asserts");
            assert_eq!(c.named_weekday_mismatch, None, "{line}: no mismatch");
            let text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(want), "{line}: Keep-through asserts");
        }
    }

    #[test]
    fn r22a_lone_framed_fence_needs_a_content_free_line() {
        // Round 22 (R22-7, review F5): the R21-B fence
        // (a lone framed date asserting beside a
        // weekday-modifier phrase) triggered on ANY
        // weekday+noun window anywhere in the line, so
        // lone content-headed lines the batch held to
        // silence asserted. The fence now requires the
        // weekday-modifier phrase to be the line's only
        // content beside the framed date — no
        // content-headed segment (service/choir/meeting
        // heads and their kin) anywhere on the line.
        for line in [
            "Monday meeting, service November 8, 2026",
            "service November 8, 2026, Monday meeting",
            "Tuesday choir, November 1, 2026",
            "choir November 1, 2026, Monday meeting",
            "early service November 1, 2026, Monday meeting",
            "Wednesday dinner, November 8, 2026",
            "Monday meeting, November 8, 2026",
            "November 8, 2026, Monday meeting",
            "Monday meeting, Tuesday choir, November 8, 2026",
            "Monday meeting, Sunday school, November 8, 2026",
            "Sunday school, Monday meeting, November 8, 2026",
        ] {
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, None, "{line}: For Sunday reads nothing");
            let text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, None, "{line}: Keep-through reads nothing");
        }
        // Held: the intended Sunday shapes keep
        // asserting, and governed lines under the
        // fence keep their governed readings.
        for (line, want) in [
            ("Sunday bulletins, November 1, 2026", Some(nov(1))),
            ("Sunday school, November 8, 2026", Some(nov(8))),
            ("printed Sunday bulletins, November 8, 2026", Some(nov(8))),
            ("printed Monday bulletins, November 8, 2026", Some(nov(8))),
            ("the Sunday bulletins, November 1, 2026", Some(nov(1))),
            ("Sunday morning, November 8, 2026", Some(nov(8))),
            ("Sunday bulletins, November 1, 2026, cancelled", None),
            ("Monday meeting, November 1, 2026, cancelled", None),
            ("Monday meeting November 8, 2026, cancelled", None),
            ("service November 8, 2026", None),
        ] {
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, want, "{line}: For Sunday");
            let text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, want, "{line}: Keep-through agrees");
        }
    }

    #[test]
    fn r22b_subjectless_auxiliary_is_a_second_clause() {
        // Round 22 (R22-4, review F1 + red team
        // RT22-5/RT22-9): "(moved November 1, then
        // was played to November 8)" was total
        // silence — `target_claim`'s between-words
        // fence killed the claim AND bare "was" is
        // itself a direct_single_governor, so
        // November 8 was governed rather than
        // unclaimed. Every other auxiliary, and
        // "was" with a subject, already landed on
        // November 8 alone. In the claim reach a
        // subjectless auxiliary + participle
        // construction is a second clause: the
        // claim dies there, November 8 stands on
        // its own standing (asserts alone, nothing
        // named), and the auxiliary does not govern
        // it — "was" joins the family. An auxiliary
        // followed by at most one adverb of any
        // kind (listed or not) and then a
        // participle / -ing form, with a subject
        // before the auxiliary, is likewise a
        // finite clause for the reach's test.
        for group in [
            // The subjectless "was" family (F1).
            "(moved November 1, then was played to November 8)",
            "(moved November 1, was then played to November 8)",
            "(moved November 1, and was played to November 8)",
            "(moved November 1, was played to November 8)",
            "(moved November 1, then was sung to November 8)",
            "(moved November 1, then was sang to November 8)",
            "(moved November 1, then was held to November 8)",
            "(moved November 1, then was taken to November 8)",
            "(moved November 1, then was beautifully played to November 8)",
            "(moved November 1, then was played beautifully to November 8)",
            "(moved November 1, then quickly was played to November 8)",
            "(moved November 1, then was quickly played to November 8)",
            "(moved November 1, then was never played to November 8)",
            "(moved November 1, then was not played to November 8)",
            "(moved November 1, then was played hymns to November 8)",
            "(moved November 1, then was singing to November 8)",
            "(moved November 1, was singing to November 8)",
            // The standing escapes keep landing.
            "(moved November 1, then were played to November 8)",
            "(moved November 1, it was played to November 8)",
            "(moved November 1, choir was played to November 8)",
            // The unlisted-adverb progressive
            // (RT22-5): one adverb of any kind
            // between auxiliary and participle.
            "(moved November 1, choir was loudly singing to November 8)",
            "(moved November 1, choir was beautifully singing to November 8)",
            "(moved November 1, choir was joyfully singing to November 8)",
            "(moved November 1, choir was loudly playing to November 8)",
            "(moved November 1, choir was loudly sung to November 8)",
            "(moved November 1, choir was not singing to November 8)",
        ] {
            let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date,
                Some(nov(8)),
                "{group}: November 8 stands alone on the relative path"
            );
            assert_eq!(
                c.for_sunday_second_dates,
                Vec::<SimpleDate>::new(),
                "{group}: nothing is named"
            );
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, Some(nov(8)), "{group}: paren-only");
            assert_eq!(
                c.for_sunday_second_dates,
                Vec::<SimpleDate>::new(),
                "{group}: paren-only note is empty"
            );
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(8)), "{group}: keep-through");
            assert_eq!(
                c.keep_through_second_dates,
                Vec::<SimpleDate>::new(),
                "{group}: keep-through note is empty"
            );
        }
        // The plain twins assert November 8; the
        // plain composition names the governed
        // November 1 on its note, as it names every
        // governed date beside an asserting one.
        for line in [
            "moved November 1, 2026, then was played to November 8, 2026",
            "moved November 1, 2026, choir was loudly singing to November 8, 2026",
        ] {
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, Some(nov(8)), "{line}: the plain twin asserts");
            assert_eq!(c.for_sunday_second_dates, vec![nov(1)], "{line}");
            let text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(8)), "{line}: keep-through");
            assert_eq!(c.keep_through_second_dates, vec![nov(1)], "{line}");
        }
        // Held: the claim's own passive
        // constructions keep their standing
        // readings — the repeated governor
        // participle still re-anchors and claims
        // with naming, the cancellation still
        // silences, and a bare "was" with no
        // participle after it still governs as
        // the past marker it is.
        let group = "(moved November 1, then was moved to November 8)";
        let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(
            c.for_date,
            Some(nov(8)),
            "{group}: the governor participle still claims"
        );
        assert_eq!(
            c.for_sunday_second_dates,
            vec![nov(1)],
            "{group}: November 1 is named"
        );
        for group in [
            "(moved November 1, then was cancelled to November 8)",
            "(was November 1)",
            "(November 1, cancelled) (was November 1)",
            "(was not November 1)",
        ] {
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, None, "{group}: paren-only reads nothing");
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, None, "{group}: keep-through reads nothing");
        }
    }

    #[test]
    fn r22b_subject_prepositional_phrases() {
        // Round 22 (R22-5, red team RT22-6/RT22-7):
        // a subject run crosses a prepositional
        // phrase — determiner / noun head +
        // preposition + its own noun-like run
        // counts as the subject, and the phrase's
        // words are consumed wholly by it. "The
        // team from Kent …" silenced both dates
        // for all 23 verb forms (the subject's
        // "from" read as an origin governor of the
        // target), and "the choir on Sunday …"
        // failed three ways by verb class (a
        // phantom October 18 asserting over the
        // was-forms, alternatives naming the
        // phantom for the irregular and auxiliary
        // forms, a false from-date naming for the
        // regular -ed forms). Every shape below
        // ends the reach: November 8 stands alone,
        // November 1 is NOT named, and no October
        // 18 is asserted or named anywhere.
        for group in [
            // The from-Kent family (RT22-6): plain,
            // auxiliary, perfect, and object forms.
            "(moved November 1, the team from Kent sang to November 8)",
            "(moved November 1, the team from Kent played to November 8)",
            "(moved November 1, the team from Kent met to November 8)",
            "(moved November 1, the team from Kent spoke to November 8)",
            "(moved November 1, the team from Kent walked to November 8)",
            "(moved November 1, the team from Kent rehearsed to November 8)",
            "(moved November 1, the team from Kent folded bulletins to November 8)",
            "(moved November 1, the team from Kent stacked chairs to November 8)",
            "(moved November 1, the team from Kent played hymns to November 8)",
            "(moved November 1, the team from Kent sang hymns to November 8)",
            "(moved November 1, the team from Kent rang bells to November 8)",
            "(moved November 1, the team from Kent was singing to November 8)",
            "(moved November 1, the team from Kent were singing to November 8)",
            "(moved November 1, the team from Kent is singing to November 8)",
            "(moved November 1, the team from Kent had sung to November 8)",
            "(moved November 1, the team from Kent was loudly singing to November 8)",
            "(moved November 1, Pastor Lee from Kent played to November 8)",
            // The on-Sunday family (RT22-7): the
            // PP's weekday spawns no signal for
            // was-forms, plain forms, and -ed forms
            // alike.
            "(moved November 1, the choir on Sunday was singing to November 8)",
            "(moved November 1, the choir on Sunday was playing to November 8)",
            "(moved November 1, the choir on Sunday was singing hymns to November 8)",
            "(moved November 1, the choir on Sunday was loudly singing to November 8)",
            "(moved November 1, the choir on Sunday sang to November 8)",
            "(moved November 1, the choir on Sunday sang hymns to November 8)",
            "(moved November 1, the choir on Sunday met to November 8)",
            "(moved November 1, the choir on Sunday spoke to November 8)",
            "(moved November 1, the choir on Sunday had sung to November 8)",
            "(moved November 1, the choir on Sunday is singing to November 8)",
            "(moved November 1, the choir on Sunday were singing to November 8)",
            "(moved November 1, the choir on Sunday played to November 8)",
            "(moved November 1, the choir on Sunday rehearsed to November 8)",
            "(moved November 1, the choir on Sunday walked to November 8)",
            "(moved November 1, the choir on Sunday folded bulletins to November 8)",
            "(moved November 1, the choir on Sunday played hymns to November 8)",
            // The standing PP controls keep landing.
            "(moved November 1, the choir in Kent sang to November 8)",
            "(moved November 1, the choir before lunch played to November 8)",
            "(moved November 1, the choir after lunch played to November 8)",
            "(moved November 1, the choir since lunch played to November 8)",
            "(moved November 1, the choir in the hall sang to November 8)",
        ] {
            let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date,
                Some(nov(8)),
                "{group}: November 8 stands alone on the relative path"
            );
            assert_eq!(
                c.for_sunday_second_dates,
                Vec::<SimpleDate>::new(),
                "{group}: nothing is named — no phantom October 18"
            );
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, Some(nov(8)), "{group}: paren-only");
            assert_eq!(
                c.for_sunday_second_dates,
                Vec::<SimpleDate>::new(),
                "{group}: paren-only note is empty"
            );
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(8)), "{group}: keep-through");
            assert_eq!(
                c.keep_through_second_dates,
                Vec::<SimpleDate>::new(),
                "{group}: keep-through note is empty"
            );
        }
        // The plain twins assert November 8 with
        // no phantom: the plain composition names
        // the governed November 1 on its note, as
        // it names every governed date beside an
        // asserting one.
        for line in [
            "moved November 1, 2026, the team from Kent sang to November 8, 2026",
            "moved November 1, 2026, the choir on Sunday was singing to November 8, 2026",
            "moved November 1, 2026, the choir on Sunday sang to November 8, 2026",
            "moved November 1, 2026, the choir on Sunday played to November 8, 2026",
        ] {
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, Some(nov(8)), "{line}: the plain twin asserts");
            assert_eq!(c.for_sunday_second_dates, vec![nov(1)], "{line}");
            let text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(8)), "{line}: keep-through");
            assert_eq!(c.keep_through_second_dates, vec![nov(1)], "{line}");
        }
        // Held: a "from" with no subject phrase
        // around it still governs as the origin
        // marker it is.
        let text = "For Sunday: (from November 1)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_date, None, "a bare origin group reads nothing");
    }

    #[test]
    fn r22b_determiner_shield_spans_its_noun_phrase() {
        // Round 22 (R22-6, review F2): the
        // determiner shield stopped one word early
        // — "(moved November 1, and the big left
        // luggage to November 8)" and "the big
        // sung hymns" killed the claim where the
        // regular twins claim, because the shield
        // covered only the word immediately before
        // the verb form and the irregular branch
        // returned finite before the
        // heads-a-noun-phrase test was reached.
        // Within its noun phrase, a participle-form
        // adjective is now shielded by a determiner
        // standing anywhere before it (the R19-6
        // active-"for" rule applied to the reach's
        // shield): every shape below claims
        // November 8 with November 1 named, like
        // the regular twins.
        for group in [
            "(moved November 1, and the big left luggage to November 8)",
            "(moved November 1, the big left luggage to November 8)",
            "(moved November 1, and my big left luggage to November 8)",
            "(moved November 1, and the big sung hymns to November 8)",
            "(moved November 1, the big sung hymns to November 8)",
            "(moved November 1, and a big sung hymn to November 8)",
            "(moved November 1, and the sung hymns to November 8)",
            // The regular twins, unchanged.
            "(moved November 1, and the big baked beans to November 8)",
            "(moved November 1, and the big singing choir to November 8)",
            "(moved November 1, and the left luggage to November 8)",
            "(moved November 1, the left luggage to November 8)",
        ] {
            let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date,
                Some(nov(8)),
                "{group}: the target asserts on the relative path"
            );
            assert_eq!(
                c.for_sunday_second_dates,
                vec![nov(1)],
                "{group}: November 1 is named as the from-date"
            );
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, Some(nov(8)), "{group}: paren-only");
            assert_eq!(c.for_sunday_second_dates, vec![nov(1)], "{group}");
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(8)), "{group}: keep-through");
            assert_eq!(c.keep_through_second_dates, vec![nov(1)], "{group}");
        }
        // Held: unshielded irregulars still end
        // the reach per verb test v2, and a
        // past-tense-only form has no adjective
        // use to shield — a clause subject's own
        // determiner does not shield its verb, in
        // the subject's phrase or any other
        // position: November 8 stands alone,
        // November 1 not named.
        for group in [
            "(moved November 1, sang hymns to November 8)",
            "(moved November 1, sung hymns to November 8)",
            "(moved November 1, left luggage to November 8)",
            "(moved November 1, the choir sang hymns to November 8)",
            "(moved November 1, the choir sang to November 8)",
            "(moved November 1, the big choir sang to November 8)",
            "(moved November 1, the choir folded bulletins to November 8)",
        ] {
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, Some(nov(8)), "{group}: November 8 stands alone");
            assert_eq!(
                c.for_sunday_second_dates,
                Vec::<SimpleDate>::new(),
                "{group}: November 1 is not named as a from-date"
            );
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(8)), "{group}: keep-through");
            assert_eq!(
                c.keep_through_second_dates,
                Vec::<SimpleDate>::new(),
                "{group}: keep-through note is empty"
            );
        }
        // Held (the v2 move of R21-B, unchanged):
        // the subjectless irregulars in the
        // "moved the meeting" form still land on
        // alternatives — the reach ends and the
        // group offers both dates.
        for group in [
            "(moved the meeting, November 1, 2026, and sung hymns to November 8)",
            "(moved the meeting, November 1, 2026, and left luggage to November 8)",
        ] {
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, None, "{group}: the target is not claimed");
            assert_eq!(
                c.for_sunday_second_dates,
                vec![nov(1), nov(8)],
                "{group}: both dates are offered as alternatives"
            );
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, None, "{group}: keep-through");
            assert_eq!(
                c.keep_through_second_dates,
                vec![nov(1), nov(8)],
                "{group}: keep-through offers both dates"
            );
        }
    }

    #[test]
    fn r22b_bare_inventory_function_classes_round_three() {
        // Round 22 (R22-8, review F4/F6 + red team
        // RT22-8): the bare inventory's classes
        // still omitted whole function-word classes
        // — the remaining prepositions (led by
        // "with"), the determiner / quantifier
        // class, possessive + "own", the review
        // tail, and head-position "too" — so a
        // governed date restated behind any of them
        // resurrected on every path (280 fuzz
        // resurrections, all through "(with
        // November 1)"). The classes join the
        // inventory; they are closed as listed in
        // `bare_filler_word` and the rule is
        // class-based: function words are bare,
        // content labels are not. Every witness
        // below reads nothing on every path.
        for word in [
            // The remaining prepositions.
            "with",
            "during",
            "since",
            "within",
            "without",
            "upon",
            "unto",
            "into",
            "over",
            "under",
            "between",
            "among",
            "against",
            "per",
            "via",
            "onto",
            "toward",
            "towards",
            "across",
            "behind",
            "beyond",
            "beside",
            "despite",
            "amid",
            "as",
            // The determiner / quantifier class.
            "each",
            "every",
            "either",
            "neither",
            "both",
            "all",
            "any",
            "some",
            "another",
            "other",
            "same",
            "such",
            // The review tail (conjunction /
            // discourse), and "too" in head
            // position.
            "lest",
            "albeit",
            "unless",
            "whereas",
            "hence",
            "thus",
            "moreover",
            "furthermore",
            "nonetheless",
            "nevertheless",
            "instead",
            "meanwhile",
            "otherwise",
            "therefore",
            "anyway",
            "rather",
            "quite",
            "indeed",
            "too",
        ] {
            let groups = format!("(November 1, cancelled) ({word} November 1)");
            let text = format!("For Sunday: next Sunday {groups}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date, None,
                "{groups}: relative path stays governed"
            );
            let text = format!("For Sunday: {groups}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, None, "{groups}: paren-only stays governed");
            let text = format!("Keep-through: {groups}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.keep_through, None,
                "{groups}: keep-through stays governed"
            );
        }
        // The same classes in the trailing slot,
        // the two-word units, possessive + "own",
        // and stacks of bare words.
        for groups in [
            "(November 1, cancelled) (November 1 with)",
            "(November 1, cancelled) (November 1 per)",
            "(November 1, cancelled) (November 1 same)",
            "(November 1, cancelled) (November 1 own)",
            "(November 1, cancelled) (November 1 very own)",
            "(November 1, cancelled) (as per November 1)",
            "(November 1, cancelled) (as of November 1)",
            "(November 1, cancelled) (my own November 1)",
            "(November 1, cancelled) (his own November 1)",
            "(November 1, cancelled) (our own November 1)",
            "(November 1, cancelled) (the same November 1)",
            "(November 1, cancelled) (that same November 1)",
            "(November 1, cancelled) (each and every November 1)",
            "(November 1, cancelled) (with the November 1)",
            "(November 1, cancelled) (too November 1 too)",
            "(November 1, cancelled) (November 1, cancelled (with November 1))",
        ] {
            let text = format!("For Sunday: next Sunday {groups}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date, None,
                "{groups}: relative path stays governed"
            );
            let text = format!("For Sunday: {groups}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, None, "{groups}: paren-only stays governed");
            let text = format!("Keep-through: {groups}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.keep_through, None,
                "{groups}: keep-through stays governed"
            );
        }
        // The composition fuzz's shape and the
        // plain twins read nothing either.
        for line in [
            "November 1, 2026, cancelled (with November 1)",
            "November 1, 2026, cancelled, as per November 1, 2026",
            "November 1, 2026, cancelled, my own November 1, 2026",
            "November 1, 2026, cancelled, too November 1, 2026",
        ] {
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, None, "{line}: the plain twin stays governed");
            let text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, None, "{line}: keep-through stays governed");
        }
        // Held (the standing adjudications, not
        // moved): "thanks" in head position stays
        // content, content labels assert, and a
        // possessive- or quantifier-headed event
        // asserts — the new classes eat no real
        // event.
        for groups in [
            "(November 1, cancelled) (thanks November 1)",
            "(November 1, cancelled) (reminder November 1)",
            "(November 1, cancelled) (our choir November 1)",
            "(November 1, cancelled) (our own choir November 1)",
            "(November 1, cancelled) (each choir November 1)",
            "(November 1, cancelled) (with choir November 1)",
        ] {
            let text = format!("For Sunday: {groups}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_date,
                Some(nov(1)),
                "{groups}: content asserts beside a sibling cancellation"
            );
            let text = format!("Keep-through: {groups}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(1)), "{groups}: keep-through");
        }
    }

    #[test]
    fn r23a_bare_inventory_deictics_and_remainders() {
        // Round 23 (R23-1/R23-2, review F1 + red team
        // RT23-6/RT23-7): the deictic class and the
        // lanes' enumerated remainders of the classes
        // R22-8 named were still outside the bare
        // inventory, so a governed date restated behind
        // any of them resurrected on every path. The
        // classes are the rule; the lists in
        // `bare_filler_word` are their current
        // membership. Every witness below reads
        // nothing on every path; the lanes' ungraded
        // pronouns / modals / interjections do NOT
        // join and are not pinned here.
        for word in [
            // R23-1: the deictic class.
            "yesterday",
            "today",
            "tomorrow",
            "here",
            "there",
            "together",
            "alone",
            "re",
            // R23-2: the preposition remainder.
            "past",
            "off",
            "besides",
            "plus",
            "minus",
            "concerning",
            "regarding",
            "barring",
            "following",
            "including",
            "atop",
            "amidst",
            "amongst",
            "thru",
            "versus",
            "vs",
            // R23-2: the quantifier / determiner
            // remainder.
            "much",
            "many",
            "few",
            "several",
            "enough",
            "most",
            "more",
            "less",
            "least",
            "none",
            "little",
            "what",
            "which",
            "whose",
            "whatever",
            "whichever",
            "whoever",
            // R23-2: the connective-adverb remainder.
            "accordingly",
            "consequently",
            "subsequently",
            "next",
            "later",
            "earlier",
            "afterwards",
            "afterward",
            "beforehand",
            "simultaneously",
            "eventually",
            "initially",
            // R23-2: the adverb remainder.
            "once",
            "twice",
            "always",
            "ever",
            "perhaps",
            "maybe",
            "almost",
            "nearly",
            "pretty",
            "fairly",
            "basically",
            "merely",
        ] {
            let groups = format!("(November 1, cancelled) ({word} November 1)");
            let text = format!("For Sunday: next Sunday {groups}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date, None,
                "{groups}: relative path stays governed"
            );
            let text = format!("For Sunday: {groups}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, None, "{groups}: paren-only stays governed");
            let text = format!("Keep-through: {groups}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.keep_through, None,
                "{groups}: keep-through stays governed"
            );
            // The plain twin resurrects nowhere either.
            let line = format!("November 1, 2026, cancelled, {word} November 1, 2026");
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, None, "{line}: plain twin stays governed");
            let text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, None, "{line}: keep-through stays governed");
        }
        // The trailing slot, for every deictic and
        // for the remainders' probed trailing members.
        for word in [
            "yesterday",
            "today",
            "tomorrow",
            "here",
            "there",
            "together",
            "alone",
            "re",
            "past",
            "off",
            "besides",
            "much",
            "many",
            "few",
            "several",
            "most",
            "none",
            "later",
            "earlier",
            "afterwards",
        ] {
            let groups = format!("(November 1, cancelled) (November 1 {word})");
            let text = format!("For Sunday: next Sunday {groups}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date, None,
                "{groups}: relative path stays governed"
            );
            let text = format!("For Sunday: {groups}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, None, "{groups}: paren-only stays governed");
            let text = format!("Keep-through: {groups}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.keep_through, None,
                "{groups}: keep-through stays governed"
            );
            let line = format!("November 1, 2026, cancelled, November 1, 2026 {word}");
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_date, None,
                "{line}: plain trailing twin stays governed"
            );
        }
        // Stacks, the typed "re:" form, the "in re"
        // unit, and a restatement wearing its own
        // governor (the deictic is no governor).
        for groups in [
            "(November 1, cancelled) (here today November 1)",
            "(November 1, cancelled) (November 1 here today)",
            "(November 1, cancelled) (the November 1 today)",
            "(November 1, cancelled) (re: November 1)",
            "(November 1, cancelled) (in re November 1)",
            "(November 1, cancelled) (today November 1, cancelled)",
        ] {
            let text = format!("For Sunday: next Sunday {groups}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date, None,
                "{groups}: relative path stays governed"
            );
            let text = format!("For Sunday: {groups}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, None, "{groups}: paren-only stays governed");
            let text = format!("Keep-through: {groups}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.keep_through, None,
                "{groups}: keep-through stays governed"
            );
        }
        // Held (the mirror, R23-1/R23-2): the widened
        // inventory eats no real event — a member of
        // each new class standing before a content
        // label still asserts, "thanks" in head
        // position stays content, and a
        // possessive-headed event still asserts.
        for groups in [
            "(November 1, cancelled) (yesterday choir November 1)",
            "(November 1, cancelled) (past choir November 1)",
            "(November 1, cancelled) (several service November 1)",
            "(November 1, cancelled) (which choir November 1)",
            "(November 1, cancelled) (later service November 1)",
            "(November 1, cancelled) (once choir November 1)",
            "(November 1, cancelled) (thanks November 1)",
            "(November 1, cancelled) (our choir November 1)",
        ] {
            let text = format!("For Sunday: {groups}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_date,
                Some(nov(1)),
                "{groups}: content asserts beside a sibling cancellation"
            );
            let text = format!("Keep-through: {groups}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(1)), "{groups}: keep-through");
        }
    }

    #[test]
    fn r23a_one_boundary_classification() {
        // Round 23 (R23-3, review F2/F3/F4/F5 + red
        // team RT23-1/RT23-2/RT23-3): boundary
        // classification was per-site, so colon,
        // semicolon, comma, dash (incl. en/em), slash,
        // and period behaved differently at different
        // sites. ONE classification now: every one of
        // them is a clause/group boundary exactly as
        // the comma is, at every site that judges
        // governor scope, relative-shape memory,
        // weekday checks, group composition, and
        // postposed composition — and a closing paren
        // between a governor and its date is a
        // boundary of the same kind. Every witness
        // below reads as its comma/space twin.
        //
        // (a) The wrong-weekday date behind a leading
        // group asserts NEVER, whatever separator
        // follows the group; the mismatch is recorded
        // as the space twin records it, on both paths.
        for sep in [
            " ",
            ", ",
            ": ",
            " - ",
            "- ",
            " \u{2013} ",
            " \u{2014} ",
            ". ",
            " / ",
            " and ",
            "; ",
        ] {
            let line = format!("(November 8){sep}Monday, November 15, 2026");
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_date, None,
                "{line}: the contradicted date never asserts"
            );
            let mismatch = c
                .named_weekday_mismatch
                .clone()
                .expect("the mismatch is recorded");
            assert_eq!(mismatch.named, "Monday", "{line}");
            assert_eq!(mismatch.date, nov(15), "{line}");
            let text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, None, "{line}: keep-through asserts nothing");
            let mismatch = c
                .keep_through_weekday_mismatch
                .clone()
                .expect("the keep-through mismatch is recorded");
            assert_eq!(mismatch.date, nov(15), "{line}");
            // With a trailing group the contradiction
            // is still recorded and nothing asserts.
            let line = format!("(November 8){sep}Monday, November 15, 2026 (November 22)");
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, None, "{line}: nothing asserts");
            assert!(
                c.named_weekday_mismatch.is_some(),
                "{line}: the mismatch is recorded beside a trailing group"
            );
        }
        // (b) A colon after the governor word is a
        // clause break standing after the governor:
        // the governor is spent on the first date and
        // the replacement asserts, first date named —
        // the comma twin's reading, on both surfaces.
        for line in [
            "November 1, 2026, cancelled: November 8, 2026",
            "November 1, 2026, cancelled : November 8, 2026",
            "November 1, 2026, cancelled: service November 8, 2026",
            "November 1, 2026, postponed: service November 8, 2026",
            "November 1, 2026: cancelled: November 8, 2026",
            "(November 1, cancelled: November 8)",
            "(November 1, cancelled: service November 8)",
        ] {
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, Some(nov(8)), "{line}: November 8 asserts");
            assert_eq!(
                c.for_sunday_second_dates,
                vec![nov(1)],
                "{line}: November 1 is named governed"
            );
            let text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(8)), "{line}: keep-through");
            assert_eq!(
                c.keep_through_second_dates,
                vec![nov(1)],
                "{line}: keep-through names November 1"
            );
        }
        // (d) A slash after a date composes postposed
        // exactly as the comma does.
        for line in [
            "November 1, 2026, cancelled / November 8, 2026",
            "November 1, 2026, moved / November 8, 2026",
            "(November 1, cancelled / November 8)",
            "(November 1, moved / November 8)",
            "November 1, 2026, cancelled / November 8, 2026, November 15, 2026",
        ] {
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, Some(nov(8)), "{line}: November 8 asserts");
            assert!(
                c.for_sunday_second_dates.contains(&nov(1)),
                "{line}: November 1 is named governed"
            );
            let text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(8)), "{line}: keep-through");
        }
        // The slash same-date restatement reads
        // nothing, as every other separator twin does.
        for line in [
            "November 1, 2026 / cancelled, November 1, 2026",
            "November 1, 2026, cancelled: November 1, 2026",
        ] {
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, None, "{line}: the governed date stays governed");
            let text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, None, "{line}: keep-through");
        }
        // (c) The relative-shape memory survives a
        // colon at its boundaries, a dash or slash
        // before its group, and a semicolon or dash
        // between trailing groups — the RT22-2
        // witnesses with the new separators.
        for (line, remembered) in [
            ("next Sunday: (November 8)", nov(8)),
            ("next Sunday \u{2014} (November 8)", nov(8)),
            ("next Sunday / (November 8)", nov(8)),
            ("(November 1): next Sunday", nov(1)),
            ("(November 1) / next Sunday", nov(1)),
            ("(November 1) next Sunday", nov(1)),
            ("next Sunday (November 1, cancelled); (November 8)", nov(8)),
            (
                "next Sunday (November 1, cancelled) \u{2014} (November 8)",
                nov(8),
            ),
            ("next Sunday; (November 8); (November 1)", nov(1)),
            ("next Sunday: (November 8): (November 1)", nov(1)),
            ("next Sunday, (November 8), (November 1)", nov(1)),
        ] {
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert!(
                c.for_sunday_in_words.is_some(),
                "{line}: the Sunday is remembered in words"
            );
            assert_eq!(
                c.for_sunday_parenthesized_date,
                Some(remembered),
                "{line}: the parenthesized date is remembered"
            );
        }
        // (e) The nested-paren witnesses read as
        // their comma twins: a closing paren between
        // a governor and its date is a boundary, and
        // November 8 asserts with November 1 named.
        for line in [
            "((November 1, cancelled), November 8)",
            "(November 1, (cancelled), November 8)",
            "(((November 1, cancelled), November 8))",
            "(November 1, cancelled, (November 8))",
        ] {
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, Some(nov(8)), "{line}: November 8 asserts");
            assert_eq!(
                c.for_sunday_second_dates,
                vec![nov(1)],
                "{line}: November 1 is named governed"
            );
            let text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(8)), "{line}: keep-through");
            assert_eq!(
                c.keep_through_second_dates,
                vec![nov(1)],
                "{line}: keep-through names November 1"
            );
        }
        // The working twin whose governor stands
        // BEFORE its date asserts November 8 and
        // names nothing — the standing silence for a
        // group-initial before-governor (round 16),
        // exactly as its flat comma twin reads.
        let line = "((cancelled, November 1), November 8)";
        let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(c.for_date, Some(nov(8)), "{line}: November 8 asserts");
        assert_eq!(
            c.for_sunday_second_dates,
            Vec::<SimpleDate>::new(),
            "{line}: nothing is named, as the flat twin names nothing"
        );
        let text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(c.keep_through, Some(nov(8)), "{line}: keep-through");
        // A separator attached with no space, after a
        // group or before a governed core, is the
        // same boundary (the comp23-D attached forms).
        for line in [
            "(please): November 1, 2026, cancelled",
            "(yesterday November 1) (please November 1);November 1, 2026, postponed",
            "(November 1 thanks) (please);November 1, 2026, cancelled",
        ] {
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, None, "{line}: the governed core stays governed");
            let text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, None, "{line}: keep-through");
        }
        // Held: a colon after a DATE already read as
        // a clause break (R22-2) and stays; a lone
        // colon-governed date reads nothing; a
        // numeric-date slash stays token-internal.
        for (line, want) in [
            (
                "November 1, 2026: cancelled, November 8, 2026",
                Some(nov(8)),
            ),
            ("cancelled: November 8, 2026", None),
            ("note: November 8, 2026", None),
            ("11/8/2026", Some(nov(8))),
            ("11/1/2026, cancelled, 11/8/2026", Some(nov(8))),
        ] {
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, want, "{line}");
            let text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, want, "{line}: keep-through");
        }
    }

    #[test]
    fn r23b_aux_slot_holds_two_words() {
        // Round 23 (R23-4, review F6/F7 = red team
        // RT23-4): the auxiliary's slot holds two
        // words — a negation AND an adverb, or two
        // adverbs — in detection and in spending
        // alike. R22-4's two tolerances composed
        // silenced both dates, and the spending walk
        // never learned the negation rule at all.
        // Every detection witness below reads
        // November 8 alone, nothing named, on every
        // path: the auxiliary construction is one
        // clause, and its verb ends the reach.
        for group in [
            "(moved November 1, then was not loudly played to November 8)",
            "(moved November 1, then was never loudly played to November 8)",
            "(moved November 1, then was not ever played to November 8)",
            "(moved November 1, choir was not ever singing to November 8)",
            "(moved November 1, then was ever not played to November 8)",
            "(moved November 1, then was really not played to November 8)",
            "(moved November 1, then was not yet played to November 8)",
            "(moved November 1, then was loudly never played to November 8)",
            "(moved November 1, choir was softly gently singing to November 8)",
            "(moved November 1, then was beautifully loudly played to November 8)",
            "(moved November 1, then was loudly beautifully played to November 8)",
        ] {
            let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date,
                Some(nov(8)),
                "{group}: November 8 stands alone on the relative path"
            );
            assert_eq!(
                c.for_sunday_second_dates,
                Vec::<SimpleDate>::new(),
                "{group}: nothing is named"
            );
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, Some(nov(8)), "{group}: paren-only");
            assert_eq!(
                c.for_sunday_second_dates,
                Vec::<SimpleDate>::new(),
                "{group}: paren-only note is empty"
            );
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(8)), "{group}: keep-through");
            assert_eq!(
                c.keep_through_second_dates,
                Vec::<SimpleDate>::new(),
                "{group}: keep-through note is empty"
            );
        }
        // The spending scans learn the negation
        // rule: a negation in the slot is spent on
        // its verb in every scan, so the passive-for
        // occasion asserts — alone, or beside its
        // trailing date — and the claim's own
        // passive claims through the negation.
        for group in [
            "(it was not moved for November 1)",
            "(it was never moved for November 1)",
            "(it was not ever moved for November 1)",
            "(it was never really moved for November 1)",
            "(it was not really moved for November 1)",
            "(it was not loudly moved for November 1)",
            "(it was loudly never moved for November 1)",
            // Round 24 (R24-6): an unlisted adverb
            // alone in the passive-for complement
            // spends now — the passive-for scan
            // crosses adverbs as detection does, so
            // this shape joins the asserting list
            // (it stood in the held list below until
            // this round; see the note there).
            "(it was loudly moved for November 1)",
            "(it was not moved, November 1)",
            "(it was never moved, November 1)",
        ] {
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_date,
                Some(nov(1)),
                "{group}: the November 1 occasion asserts, spent through the slot"
            );
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(1)), "{group}: keep-through");
        }
        for group in [
            "(moved November 1, it was not moved to November 8)",
            "(moved November 1, it was never moved to November 8)",
        ] {
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_date,
                Some(nov(8)),
                "{group}: the claim claims its target through the negation"
            );
            assert_eq!(
                c.for_sunday_second_dates,
                vec![nov(1)],
                "{group}: November 1 is named as the from-date"
            );
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(8)), "{group}: keep-through");
            assert_eq!(c.keep_through_second_dates, vec![nov(1)], "{group}");
        }
        // Held: three-word slots do not spend (the
        // spending cap is two, negation included);
        // the claim's own passives keep their
        // standing readings. Round 24 (R24-6) moved
        // one shape out of this list: "(it was loudly
        // moved for November 1)" asserts now, because
        // the passive-for scan's complement crosses
        // adverbs as detection does — an unlisted
        // adverb alone crosses there, as it always
        // has in `aux_complement`. The governor
        // scans keep the R17-7 closed list and this
        // cap unchanged: the comma twin "(it was
        // loudly moved, November 1)" still reads
        // nothing by the spent-predicate scan.
        let group = "(it was not really truly moved for November 1)";
        {
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_date, None,
                "{group}: no spending, the date is governed"
            );
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, None, "{group}: keep-through");
        }
        let group = "(moved November 1, then was moved to November 8)";
        let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(c.for_date, Some(nov(8)), "{group}: still claims");
        assert_eq!(c.for_sunday_second_dates, vec![nov(1)], "{group}");
        let group = "(moved November 1, then was cancelled to November 8)";
        let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(c.for_date, None, "{group}: still silences");
    }

    #[test]
    fn r23b_subject_pp_stacks_and_date_objects() {
        // Round 23 (R23-5, review F8/F9 = red team
        // RT23-5/RT23-10): the subject run crosses a
        // PP stack of any length and any preposition
        // order; a determiner inside the PP blocks
        // nothing; and a framed date standing as a
        // subject-PP object after ANY preposition is
        // consumed by the subject exactly as after
        // "from". Every stack witness below ends the
        // reach: November 8 alone, nothing named, no
        // phantom from the weekday PP in either
        // stack order, on every path.
        for group in [
            "(moved November 1, the team from Kent on Sunday sang to November 8)",
            "(moved November 1, the team from Kent on Sunday played to November 8)",
            "(moved November 1, the team from Kent on Sunday was singing to November 8)",
            "(moved November 1, the team from Kent in the hall sang to November 8)",
            "(moved November 1, the team from Kent at noon sang to November 8)",
            "(moved November 1, the big team from Kent on Sunday sang to November 8)",
            "(moved November 1, the choir on Sunday from Kent sang to November 8)",
            "(moved November 1, the choir on Sunday in the hall sang to November 8)",
            "(moved November 1, the choir on Sunday in Kent sang to November 8)",
            "(moved November 1, the choir on the Sunday sang to November 8)",
            "(moved November 1, the choir on the Sunday played to November 8)",
            "(moved November 1, the choir on the Sunday was singing to November 8)",
        ] {
            let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date,
                Some(nov(8)),
                "{group}: November 8 stands alone on the relative path"
            );
            assert_eq!(
                c.for_sunday_second_dates,
                Vec::<SimpleDate>::new(),
                "{group}: nothing is named — no phantom October 18"
            );
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, Some(nov(8)), "{group}: paren-only");
            assert_eq!(
                c.for_sunday_second_dates,
                Vec::<SimpleDate>::new(),
                "{group}: paren-only note is empty"
            );
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(8)), "{group}: keep-through");
            assert_eq!(
                c.keep_through_second_dates,
                Vec::<SimpleDate>::new(),
                "{group}: keep-through note is empty"
            );
        }
        // The plain twins of the stacks assert
        // November 8 with the governed November 1
        // named, as the plain composition names
        // every governed date beside an asserting
        // one — and no phantom.
        for line in [
            "moved November 1, 2026, the team from Kent on Sunday sang to November 8, 2026",
            "moved November 1, 2026, the choir on Sunday from Kent sang to November 8, 2026",
        ] {
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, Some(nov(8)), "{line}: the plain twin asserts");
            assert_eq!(c.for_sunday_second_dates, vec![nov(1)], "{line}");
            let text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(8)), "{line}: keep-through");
            assert_eq!(c.keep_through_second_dates, vec![nov(1)], "{line}");
        }
        // A framed date as a subject-PP object,
        // after any preposition: the target asserts
        // (not alternatives), and the PP's date is
        // named beside it exactly as the "from"
        // twin's date is.
        for (group, named) in [
            (
                "(moved November 1, the team from November 15 sang to November 8)",
                nov(15),
            ),
            (
                "(moved November 1, the choir on November 15 sang to November 8)",
                nov(15),
            ),
            (
                "(moved November 1, the team by November 15 sang to November 8)",
                nov(15),
            ),
            (
                "(moved November 1, the team before November 15 sang to November 8)",
                nov(15),
            ),
            (
                "(moved November 1, the team after November 1 sang to November 8)",
                nov(1),
            ),
            (
                "(moved November 1, the choir on November 1 sang to November 8)",
                nov(1),
            ),
        ] {
            let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date,
                Some(nov(8)),
                "{group}: the target asserts on the relative path, not alternatives"
            );
            assert_eq!(
                c.for_sunday_second_dates,
                vec![named],
                "{group}: the subject PP's date is named, as after from"
            );
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, Some(nov(8)), "{group}: paren-only");
            assert_eq!(c.for_sunday_second_dates, vec![named], "{group}");
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(8)), "{group}: keep-through");
            assert_eq!(c.keep_through_second_dates, vec![named], "{group}");
        }
        // The plain twins: the PP's date no longer
        // asserts itself or demotes the clause's
        // date — November 8 asserts, the PP date is
        // noted, exactly as the "from" twin reads.
        for line in [
            "the choir on November 15 sang November 8, 2026",
            "the team from November 15 sang November 8, 2026",
            "the choir on November 1 was singing November 8, 2026",
        ] {
            let want_named = if line.contains("November 15") {
                nov(15)
            } else {
                nov(1)
            };
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, Some(nov(8)), "{line}: November 8 asserts");
            assert_eq!(c.for_sunday_second_dates, vec![want_named], "{line}");
            let text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(8)), "{line}: keep-through");
            assert_eq!(c.keep_through_second_dates, vec![want_named], "{line}");
        }
        // Held: the object-continuation PP is not a
        // subject PP — "(moved November 1, baked
        // beans from Kent to November 8)" still
        // reads nothing by "from" governance, and
        // the single-PP pins of R22-5 stand (the
        // r22b test above).
        let group = "(moved November 1, baked beans from Kent to November 8)";
        let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(c.for_date, None, "{group}: the object PP still governs");
        let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(c.keep_through, None, "{group}: keep-through");
    }

    #[test]
    fn r23b_weekday_verb_form_modifier_phrase() {
        // Round 23 (R23-6, review F10): weekday +
        // verb-form word + noun-like word is a
        // modifier phrase on both surfaces — the
        // verb-form word is the phrase's adjective,
        // no phantom October 18 is asserted, offered,
        // or named, and the framed date asserts.
        for (line, want) in [
            ("Sunday left luggage, November 8, 2026", nov(8)),
            ("Sunday sung hymns, November 1, 2026", nov(1)),
            ("Sunday taken seats, November 8, 2026", nov(8)),
            ("Sunday gone seats, November 8, 2026", nov(8)),
            ("Sunday met choir, November 8, 2026", nov(8)),
            ("Monday left luggage, November 8, 2026", nov(8)),
            ("Sunday singing choir, November 8, 2026", nov(8)),
            // The regular -ed twin, standing.
            ("Sunday baked beans, November 8, 2026", nov(8)),
        ] {
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_date,
                Some(want),
                "{line}: the framed date asserts, no phantom"
            );
            assert_eq!(
                c.for_sunday_second_dates,
                Vec::<SimpleDate>::new(),
                "{line}: no October 18 is named"
            );
            let text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(want), "{line}: keep-through");
            assert_eq!(
                c.keep_through_second_dates,
                Vec::<SimpleDate>::new(),
                "{line}: keep-through names no phantom"
            );
        }
        // The group twin: the phrase stands in an
        // object continuation, the claim crosses it,
        // November 8 claims with November 1 named —
        // and no October 18 anywhere.
        let group = "(moved the meeting, November 1, 2026, and Sunday taken seats to November 8)";
        let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(
            c.for_sunday_parenthesized_date,
            Some(nov(8)),
            "{group}: the target claims on the relative path"
        );
        assert_eq!(
            c.for_sunday_second_dates,
            vec![nov(1)],
            "{group}: November 1 is named, October 18 is not"
        );
        let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(c.for_date, Some(nov(8)), "{group}: paren-only claims");
        assert_eq!(c.for_sunday_second_dates, vec![nov(1)], "{group}");
        let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(c.keep_through, Some(nov(8)), "{group}: keep-through");
        assert_eq!(c.keep_through_second_dates, vec![nov(1)], "{group}");
        // Held: the bare-noun modifier pins
        // (R20-6/R21-7), the true relative phrases,
        // and the subject-PP weekday consumption
        // (R22-5) all stand — the r20b/r21b/r22b
        // tests above; one of each re-checked here.
        for (line, want) in [
            ("Sunday bulletins, November 1, 2026", Some(nov(1))),
            ("Sunday school, November 8, 2026", Some(nov(8))),
            ("next Sunday, November 8, 2026", Some(nov(8))),
            ("Sunday, November 8, 2026", Some(nov(8))),
        ] {
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, want, "{line}: standing reading");
        }
    }

    #[test]
    fn r23b_lone_framed_fence_fires_only_for_garnish() {
        // Round 23 (R23-7, review F11): the
        // lone-framed fence is inverted — it fires
        // only when the line's content beside the
        // framed date is a weekday-modifier phrase
        // whose head noun is on the closed
        // GARNISH-noun set (bulletins, school,
        // morning, as shipped in `GARNISH_NOUNS`).
        // Every other noun — event or not, on the
        // old content-head list or off it — blocks
        // the fence, and the line reads nothing:
        // the failure direction is silence now.
        for line in [
            "Sunday picnic, November 8, 2026",
            "Sunday brunch, November 8, 2026",
            "Sunday festival, November 8, 2026",
            "Sunday banquet, November 8, 2026",
            "Monday picnic, November 8, 2026",
            "Sunday picnics, November 8, 2026",
            // On the old list, silent then and now —
            // now by the garnish rule, not by list
            // membership.
            "Sunday rehearsal, November 1, 2026",
            "Sunday rehearsal, November 8, 2026",
            "Sunday service, November 8, 2026",
        ] {
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, None, "{line}: For Sunday reads nothing");
            assert_eq!(
                c.for_sunday_second_dates,
                Vec::<SimpleDate>::new(),
                "{line}: nothing is named"
            );
            let text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, None, "{line}: Keep-through reads nothing");
        }
        // The pinned garnish shapes keep asserting,
        // and a governed line under a garnish phrase
        // keeps its governed reading.
        for (line, want) in [
            ("Sunday bulletins, November 1, 2026", Some(nov(1))),
            ("Sunday school, November 8, 2026", Some(nov(8))),
            ("Sunday morning, November 8, 2026", Some(nov(8))),
            ("printed Sunday bulletins, November 8, 2026", Some(nov(8))),
            ("printed Monday bulletins, November 8, 2026", Some(nov(8))),
            ("the Sunday bulletins, November 1, 2026", Some(nov(1))),
            ("Sunday picnic, November 1, 2026, cancelled", None),
            ("Sunday bulletins, November 1, 2026, cancelled", None),
        ] {
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, want, "{line}: For Sunday");
            let text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, want, "{line}: Keep-through agrees");
        }
        // Held: an off-garnish head in its own
        // clause still closes the fence for the
        // whole line, in either clause order.
        for line in [
            "Sunday bulletins, picnic November 8, 2026",
            "picnic November 8, 2026, Sunday bulletins",
        ] {
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, None, "{line}: the fence stays closed");
            let text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, None, "{line}: keep-through");
        }
    }

    #[test]
    fn r23b_past_only_split_scopes_both_shields() {
        // Round 23 (R23-8, review M1 = red team
        // RT23-8): the past-only split scopes BOTH
        // determiner shields — "the sang hymns" in
        // the reach reads as a clause (November 8
        // alone, November 1 unnamed), exactly as
        // "the big sang hymns" does, because a
        // past-only form has no adjective use in
        // any position. Participle forms keep their
        // shield under both.
        for group in [
            "(moved November 1, and the sang hymns to November 8)",
            "(moved November 1, the sang hymns to November 8)",
            "(moved November 1, and the big sang hymns to November 8)",
            "(moved November 1, and the took seats to November 8)",
        ] {
            let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date,
                Some(nov(8)),
                "{group}: November 8 stands alone on the relative path"
            );
            assert_eq!(
                c.for_sunday_second_dates,
                Vec::<SimpleDate>::new(),
                "{group}: November 1 is not named — the form read as a clause verb"
            );
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, Some(nov(8)), "{group}: paren-only");
            assert_eq!(
                c.for_sunday_second_dates,
                Vec::<SimpleDate>::new(),
                "{group}: paren-only note is empty"
            );
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(8)), "{group}: keep-through");
            assert_eq!(
                c.keep_through_second_dates,
                Vec::<SimpleDate>::new(),
                "{group}: keep-through note is empty"
            );
        }
        // Held: participle forms keep their shield
        // under both the immediate and the spanned
        // shield, and claim as before (R22-6's
        // spanned-shield pins stand in the r22b
        // test above).
        for group in [
            "(moved November 1, and the taken seats to November 8)",
            "(moved November 1, and the big taken seats to November 8)",
            "(moved November 1, and the baked beans to November 8)",
            "(moved November 1, and the left luggage to November 8)",
        ] {
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_date,
                Some(nov(8)),
                "{group}: the shielded phrase still claims"
            );
            assert_eq!(
                c.for_sunday_second_dates,
                vec![nov(1)],
                "{group}: November 1 is named as the from-date"
            );
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(8)), "{group}: keep-through");
            assert_eq!(c.keep_through_second_dates, vec![nov(1)], "{group}");
        }
    }

    #[test]
    fn r24a_bare_inventory_subordinator_class() {
        // Round 24 (R24-1, review F1 = red team
        // RT24-1): the subordinator class joins the
        // bare inventory, and "than" joins the
        // connective class. Every witness below reads
        // nothing on every path; the mirror held (a
        // content label standing with the word still
        // asserts).
        for word in [
            "when",
            "whenever",
            "while",
            "whilst",
            "although",
            "though",
            "because",
            "if",
            "yet",
            "provided",
            "supposing",
            "wherever",
            "where",
            "why",
            "how",
            "whether",
            "than",
        ] {
            for groups in [
                format!("(November 1, cancelled) ({word} November 1)"),
                format!("(November 1, cancelled) (November 1 {word})"),
            ] {
                let text = format!("For Sunday: next Sunday {groups}\n\n## Announcement\n\nHi.\n");
                let c = parse_contribution(&text, today()).expect("parses");
                assert_eq!(
                    c.for_sunday_parenthesized_date, None,
                    "{groups}: relative path stays governed"
                );
                let text = format!("For Sunday: {groups}\n\n## Announcement\n\nHi.\n");
                let c = parse_contribution(&text, today()).expect("parses");
                assert_eq!(c.for_date, None, "{groups}: paren-only stays governed");
                let text = format!("Keep-through: {groups}\n\n## Announcement\n\nHi.\n");
                let c = parse_contribution(&text, today()).expect("parses");
                assert_eq!(
                    c.keep_through, None,
                    "{groups}: keep-through stays governed"
                );
            }
            // The plain twins, head and trailing.
            for line in [
                format!("November 1, 2026, cancelled, {word} November 1, 2026"),
                format!("November 1, 2026, cancelled, November 1, 2026 {word}"),
            ] {
                let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
                let c = parse_contribution(&text, today()).expect("parses");
                assert_eq!(c.for_date, None, "{line}: plain twin stays governed");
                let text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
                let c = parse_contribution(&text, today()).expect("parses");
                assert_eq!(c.keep_through, None, "{line}: keep-through stays governed");
            }
            // Held (the mirror): a content label
            // standing with the word still asserts.
            let groups = format!("(November 1, cancelled) ({word} choir November 1)");
            let text = format!("For Sunday: {groups}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_date,
                Some(nov(1)),
                "{groups}: content asserts beside a sibling cancellation"
            );
            let text = format!("Keep-through: {groups}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(1)), "{groups}: keep-through");
        }
        // Phrasal witnesses through the new members.
        for groups in [
            "(November 1, cancelled) (even if November 1)",
            "(November 1, cancelled) (as if November 1)",
            "(November 1, cancelled) (more than November 1)",
            "(November 1, cancelled) (less than November 1)",
        ] {
            let text = format!("For Sunday: {groups}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, None, "{groups}: paren-only stays governed");
            let text = format!("Keep-through: {groups}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.keep_through, None,
                "{groups}: keep-through stays governed"
            );
        }
        // Held: the ungraded pronoun / modal /
        // interjection sets the red team
        // recorded-not-graded stay untouched — a
        // restatement wearing only one still asserts.
        for groups in [
            "(November 1, cancelled) (he November 1)",
            "(November 1, cancelled) (will November 1)",
            "(November 1, cancelled) (oh November 1)",
        ] {
            let text = format!("For Sunday: {groups}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_date,
                Some(nov(1)),
                "{groups}: the ungraded sets stay untouched"
            );
        }
    }

    #[test]
    fn r24a_bare_phrase_units() {
        // Round 24 (R24-2, red team RT24-2 + review
        // M1): phrase units in the bare test — a
        // region is bare when its words partition
        // into inventory words and the listed phrase
        // units, matched as units. The closed list as
        // shipped: "care of", "in care of", "care of
        // the", "back together", "back together
        // again", "gone tomorrow", "gone by tomorrow",
        // "per se", "if and when", "when and if",
        // "as and when", "yet again", "together at
        // last". Every unit witness reads nothing on
        // every path, head and trailing alike.
        for unit in [
            "care of",
            "in care of",
            "care of the",
            "back together",
            "back together again",
            "gone tomorrow",
            "gone by tomorrow",
            "per se",
            "if and when",
            "when and if",
            "as and when",
            "yet again",
            "together at last",
        ] {
            for groups in [
                format!("(November 1, cancelled) ({unit} November 1)"),
                format!("(November 1, cancelled) (November 1 {unit})"),
            ] {
                let text = format!("For Sunday: next Sunday {groups}\n\n## Announcement\n\nHi.\n");
                let c = parse_contribution(&text, today()).expect("parses");
                assert_eq!(
                    c.for_sunday_parenthesized_date, None,
                    "{groups}: relative path stays governed"
                );
                let text = format!("For Sunday: {groups}\n\n## Announcement\n\nHi.\n");
                let c = parse_contribution(&text, today()).expect("parses");
                assert_eq!(c.for_date, None, "{groups}: paren-only stays governed");
                let text = format!("Keep-through: {groups}\n\n## Announcement\n\nHi.\n");
                let c = parse_contribution(&text, today()).expect("parses");
                assert_eq!(
                    c.keep_through, None,
                    "{groups}: keep-through stays governed"
                );
            }
        }
        // The plain twins of the lead units.
        for line in [
            "November 1, 2026, cancelled, care of November 1, 2026",
            "November 1, 2026, cancelled, November 1, 2026 per se",
            "November 1, 2026, cancelled (back together November 1)",
        ] {
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, None, "{line}: plain twin stays governed");
            let text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, None, "{line}: keep-through stays governed");
        }
        // Held: a unit's member standing alone keeps
        // its current reading — "care" and "gone"
        // alone are not bare, and assert as they did.
        for groups in [
            "(November 1, cancelled) (care November 1)",
            "(November 1, cancelled) (gone November 1)",
        ] {
            let text = format!("For Sunday: {groups}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_date,
                Some(nov(1)),
                "{groups}: a lone member still asserts"
            );
            let text = format!("Keep-through: {groups}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(1)), "{groups}: keep-through");
        }
    }

    #[test]
    fn r24a_fence_judges_the_phrase_head() {
        // Round 24 (R24-3, review F2 + M2 = red team
        // RT24-6 part): the lone-framed fence judges
        // the weekday phrase's HEAD — its final noun —
        // against the garnish set, and the set gains
        // the printed-matter kin (bulletin, papers,
        // newsletter, leaflet, sheets).
        for line in [
            "Sunday morning rehearsal, November 8, 2026",
            "Sunday school picnic, November 8, 2026",
            "Sunday morning service, November 8, 2026",
            "Sunday morning picnic, November 8, 2026",
            "Sunday bulletins rehearsal, November 8, 2026",
            "Sunday morning meeting, November 8, 2026",
            "Sunday school rehearsal, November 1, 2026",
            "Sunday morning choir, November 8, 2026",
            "Monday morning meeting, November 15, 2026",
            // Held: "Sunday service" reads nothing —
            // service is an event head, not garnish.
            "Sunday service, November 8, 2026",
        ] {
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, None, "{line}: For Sunday reads nothing");
            assert_eq!(
                c.for_sunday_second_dates,
                Vec::<SimpleDate>::new(),
                "{line}: nothing is named"
            );
            let text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, None, "{line}: Keep-through reads nothing");
        }
        for (line, want) in [
            // The pinned garnish heads: the final noun
            // is garnish.
            ("Sunday morning, November 8, 2026", nov(8)),
            ("Sunday school, November 8, 2026", nov(8)),
            ("Sunday bulletins, November 8, 2026", nov(8)),
            ("Sunday school bulletin, November 8, 2026", nov(8)),
            ("Sunday morning bulletins, November 8, 2026", nov(8)),
            ("Sunday morning bulletin, November 8, 2026", nov(8)),
            // The printed-matter kin as shipped.
            ("Sunday bulletin, November 1, 2026", nov(1)),
            ("Sunday bulletin, November 8, 2026", nov(8)),
            ("Sunday papers, November 8, 2026", nov(8)),
            ("Sunday newsletter, November 8, 2026", nov(8)),
            ("Sunday newsletter, November 1, 2026", nov(1)),
            ("Sunday leaflet, November 8, 2026", nov(8)),
            ("Sunday sheets, November 8, 2026", nov(8)),
            // Held pins: "Monday bulletins" asserts,
            // and the verb-form adjective phrase
            // passes by phrase shape (R23-6), not by
            // the garnish set.
            ("Monday bulletins, November 8, 2026", nov(8)),
            ("Sunday left luggage, November 8, 2026", nov(8)),
        ] {
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, Some(want), "{line}: For Sunday asserts");
            let text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(want), "{line}: Keep-through asserts");
        }
    }

    #[test]
    fn r24a_boundary_token_rule() {
        // Round 24 (R24-4, review F4/F5/F6 + M5 part
        // = red team RT24-3): the boundary rule is a
        // token rule — any standalone token containing
        // no letters and no digits is a boundary token
        // at every site (governor scope, weekday
        // check, relative-shape memory, group
        // composition, postposed composition), and
        // bracketed tokens count as boundary/garnish
        // tokens in group cleaning and in date spans.
        //
        // (F4) The postposed composition: a separator
        // between a postposed governor and its target
        // reads as its comma twin.
        for sep in ["*", "|", "~", "_", "\u{2022}", "\u{1f389}"] {
            let line = format!("November 1, 2026, cancelled {sep} November 8, 2026");
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, Some(nov(8)), "{line}: November 8 asserts");
            assert_eq!(
                c.for_sunday_second_dates,
                vec![nov(1)],
                "{line}: November 1 is named governed"
            );
            let text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(8)), "{line}: keep-through");
            assert_eq!(c.keep_through_second_dates, vec![nov(1)], "{line}");
            // Inside a group too.
            let groups = format!("(November 1, cancelled {sep} November 8)");
            let text = format!("For Sunday: {groups}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, Some(nov(8)), "{groups}: November 8 asserts");
            assert_eq!(c.for_sunday_second_dates, vec![nov(1)], "{groups}");
            // The same-date restatement reads nothing,
            // in both separator positions.
            for line in [
                format!("November 1, 2026, cancelled {sep} November 1, 2026"),
                format!("November 1, 2026 {sep} cancelled, November 1, 2026"),
            ] {
                let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
                let c = parse_contribution(&text, today()).expect("parses");
                assert_eq!(c.for_date, None, "{line}: the governed date stays governed");
                let text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
                let c = parse_contribution(&text, today()).expect("parses");
                assert_eq!(c.keep_through, None, "{line}: keep-through");
            }
        }
        // (F5) The wrong-weekday bypass stays closed
        // behind a leading group and any of these
        // separators: the mismatch is recorded.
        for sep in ["*", "\u{2022}", "|", "~", "_", "\u{1f389}"] {
            let line = format!("(November 1) {sep} Monday November 15, 2026");
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_date, None,
                "{line}: the contradicted date never asserts"
            );
            let mismatch = c
                .named_weekday_mismatch
                .clone()
                .expect("the mismatch is recorded");
            assert_eq!(mismatch.named, "Monday", "{line}");
            assert_eq!(mismatch.date, nov(15), "{line}");
            let text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, None, "{line}: keep-through asserts nothing");
            let mismatch = c
                .keep_through_weekday_mismatch
                .clone()
                .expect("the keep-through mismatch is recorded");
            assert_eq!(mismatch.date, nov(15), "{line}");
        }
        // (F6) Relative-shape memory keeps the
        // in-words reading across these tokens.
        for sep in ["*", "|", "\u{2022}", "~", "\u{1f389}"] {
            for (line, remembered) in [
                (format!("next Sunday {sep} (November 8)"), nov(8)),
                (format!("(November 8) {sep} next Sunday"), nov(8)),
                (
                    format!("next Sunday (November 1) {sep} (November 8)"),
                    nov(8),
                ),
            ] {
                let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
                let c = parse_contribution(&text, today()).expect("parses");
                assert!(
                    c.for_sunday_in_words.is_some(),
                    "{line}: the Sunday is remembered in words"
                );
                assert_eq!(
                    c.for_sunday_parenthesized_date,
                    Some(remembered),
                    "{line}: the parenthesized date is remembered"
                );
            }
            let line = format!("(November 8) {sep} next Sunday");
            let text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.keep_through,
                Some(nov(8)),
                "{line}: keep-through reads the remembered date"
            );
        }
        // (RT24-3) Boundary and bracketed tokens
        // standing inside a group beside its date are
        // garnish: the group restates bare and the
        // sibling governance binds it.
        for shape in [
            "(November 1 *)",
            "(* November 1)",
            "(November 1 |)",
            "(| November 1)",
            "(November 1 \u{2022})",
            "(\u{2022} November 1)",
            "(November 1 [x])",
            "(November 1 {x})",
        ] {
            let groups = format!("(November 1, cancelled) {shape}");
            let text = format!("For Sunday: next Sunday {groups}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date, None,
                "{groups}: relative path stays governed"
            );
            let text = format!("For Sunday: {groups}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, None, "{groups}: paren-only stays governed");
            let text = format!("Keep-through: {groups}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.keep_through, None,
                "{groups}: keep-through stays governed"
            );
        }
        // (M5) A "[" before a date does not poison its
        // parse: the date reads or not on its own
        // merits.
        for (line, want, named) in [
            (
                "[November 8, 2026], November 15, 2026",
                Some(nov(8)),
                vec![nov(15)],
            ),
            (
                "November 15, 2026, [November 8, 2026]",
                Some(nov(15)),
                vec![nov(8)],
            ),
            (
                "{November 8, 2026}, November 15, 2026",
                Some(nov(8)),
                vec![nov(15)],
            ),
            ("Sunday bulletins, [November 8, 2026]", Some(nov(8)), vec![]),
            ("cancelled [November 8, 2026]", None, vec![]),
            (
                "[November 1, cancelled] [November 8]",
                Some(nov(8)),
                vec![nov(1)],
            ),
        ] {
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, want, "{line}: For Sunday");
            assert_eq!(c.for_sunday_second_dates, named, "{line}: second dates");
            let text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, want, "{line}: Keep-through");
            assert_eq!(
                c.keep_through_second_dates, named,
                "{line}: keep second dates"
            );
        }
        // Held: brackets are NOT group delimiters — a
        // bracketed date alone stays unread, the
        // recorded limitation.
        for line in ["[November 8]", "{November 8}"] {
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_date, None,
                "{line}: a bracketed date alone stays unread"
            );
            let text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, None, "{line}: keep-through stays unread");
        }
    }

    #[test]
    fn r24b_weekday_pp_inside_object_continuation() {
        // Round 24 (R24-5, review F3): a weekday
        // standing as a PP object inside a target
        // claim's OBJECT continuation is consumed
        // wholly — no signal, no naming — exactly as
        // in a subject PP. Before this round the
        // "on Sunday" PP spawned a relative signal
        // inside the continuation: in one order the
        // line claimed November 8 naming a phantom
        // October 18, and in the swapped order the
        // phantom was the group's date outright.
        // Both orders now read nothing and name
        // nothing, by the standing "from" governance
        // — the held twin without the weekday PP,
        // re-checked below, reads the same.
        for group in [
            "(moved November 1, and baked beans from Kent on Sunday to November 8)",
            "(moved November 1, and baked beans on Sunday from Kent to November 8)",
            "(moved November 1, and the baked beans from Kent on Sunday to November 8)",
            // Held: the object-continuation PP with
            // no weekday in it — "from" governance
            // silences the line, as pinned in R23-B.
            "(moved November 1, baked beans from Kent to November 8)",
        ] {
            let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date, None,
                "{group}: nothing asserts on the relative path"
            );
            assert_eq!(
                c.for_sunday_second_dates,
                Vec::<SimpleDate>::new(),
                "{group}: nothing is named — no phantom October 18"
            );
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, None, "{group}: paren-only reads nothing");
            assert_eq!(
                c.for_sunday_second_dates,
                Vec::<SimpleDate>::new(),
                "{group}: paren-only note is empty"
            );
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, None, "{group}: keep-through reads nothing");
            assert_eq!(
                c.keep_through_second_dates,
                Vec::<SimpleDate>::new(),
                "{group}: keep-through note is empty"
            );
        }
        // The plain twins of both orders read
        // nothing and name nothing the same way.
        for line in [
            "moved November 1, 2026, and baked beans from Kent on Sunday to November 8, 2026",
            "moved November 1, 2026, and baked beans on Sunday from Kent to November 8, 2026",
        ] {
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, None, "{line}: the plain twin reads nothing");
            assert_eq!(
                c.for_sunday_second_dates,
                Vec::<SimpleDate>::new(),
                "{line}: no phantom is named"
            );
            let text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, None, "{line}: keep-through reads nothing");
            assert_eq!(
                c.keep_through_second_dates,
                Vec::<SimpleDate>::new(),
                "{line}: keep-through names nothing"
            );
        }
        // The continuation without the list
        // conjunction is the same continuation: the
        // weekday is consumed, the ungoverned target
        // asserts, and November 1 stays governed.
        let group = "(moved November 1, baked beans on Sunday to November 8)";
        let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(c.for_date, Some(nov(8)), "{group}: the target asserts");
        assert_eq!(
            c.for_sunday_second_dates,
            Vec::<SimpleDate>::new(),
            "{group}: no phantom is named"
        );
        // The full-date object list ("moved the
        // meeting, November 1, 2026, and …") reads
        // as its standing twins do — the no-weekday
        // twin and the "on Sunday morning" twin both
        // read November 1 alone: the phantom used to
        // collapse this shape into alternatives
        // naming all three dates, and it now asserts
        // its occasion cleanly, naming nothing.
        let group =
            "(moved the meeting, November 1, 2026, and baked beans from Kent on Sunday to November 8)";
        let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(
            c.for_sunday_parenthesized_date,
            Some(nov(1)),
            "{group}: the occasion asserts on the relative path, as its twins do"
        );
        assert_eq!(
            c.for_sunday_second_dates,
            Vec::<SimpleDate>::new(),
            "{group}: nothing is named"
        );
        let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(c.for_date, Some(nov(1)), "{group}: paren-only");
        assert_eq!(
            c.for_sunday_second_dates,
            Vec::<SimpleDate>::new(),
            "{group}: paren-only note is empty"
        );
        let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(c.keep_through, Some(nov(1)), "{group}: keep-through");
        assert_eq!(
            c.keep_through_second_dates,
            Vec::<SimpleDate>::new(),
            "{group}: keep-through note is empty"
        );
        // The boundary this batch implements: a
        // weekday PP standing in a SECOND clause —
        // its own subject and finite verb between
        // the claim's phrase and the preposition —
        // is a clause, not a PP inside the
        // continuation, so its weekday keeps its
        // signal; and a weekday PP with no target
        // phrase behind it is no claim's object
        // continuation. Both shapes keep their
        // standing readings: nothing asserts, and
        // both dates stand as alternatives.
        for group in [
            "(moved November 1, beans were served on Sunday to November 8)",
            "(moved November 1, choir sang on Sunday to November 8)",
            "(baked beans on Sunday to November 8)",
        ] {
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_date, None,
                "{group}: outside the continuation, nothing asserts"
            );
            assert_eq!(
                c.for_sunday_second_dates,
                vec![oct(18), nov(8)],
                "{group}: the weekday's signal stands, both dates offered"
            );
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.keep_through, None,
                "{group}: keep-through asserts nothing"
            );
            assert_eq!(
                c.keep_through_second_dates,
                vec![oct(18), nov(8)],
                "{group}: keep-through offers both dates"
            );
        }
        // Held re-checks: the subject-PP weekday
        // consumption (R22-5/R23-5) is unchanged, and
        // the "Sunday morning" modifier shape inside
        // the continuation still claims its target
        // with its from-date named, as before.
        let group = "(moved November 1, the choir on Sunday sang to November 8)";
        let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(
            c.for_date,
            Some(nov(8)),
            "{group}: the subject PP still ends the reach"
        );
        assert_eq!(
            c.for_sunday_second_dates,
            Vec::<SimpleDate>::new(),
            "{group}: nothing is named"
        );
        let group = "(moved November 1, and baked beans on Sunday morning to November 8)";
        let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(
            c.for_date,
            Some(nov(8)),
            "{group}: the modifier shape claims"
        );
        assert_eq!(
            c.for_sunday_second_dates,
            vec![nov(1)],
            "{group}: November 1 is named, no phantom"
        );
    }

    #[test]
    fn r24b_passive_for_complement_crosses_adverbs_as_detection_does() {
        // Round 24 (R24-6, review M3 = red team
        // RT24-5): detection and spending disagreed
        // on the passive-for complement — detection
        // crosses two unlisted adverbs in the
        // auxiliary's slot ("choir was softly gently
        // singing" is one clause), while the
        // passive-for scan crossed an unlisted adverb
        // only beside a negation, so the claim's own
        // passive "(it was softly gently moved for
        // November 1)" asserted nothing. The
        // passive-for scan's complement now crosses
        // adverbs as detection does, under the same
        // two-slot-word cap (a negation counts as one
        // slot word): the occasions assert.
        for group in [
            "(it was softly gently moved for November 1)",
            "(it was gently loudly moved for November 1)",
            "(it was loudly moved for November 1)",
            "(it was softly moved for November 1)",
            "(it was not softly moved for November 1)",
        ] {
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_date,
                Some(nov(1)),
                "{group}: the November 1 occasion asserts, spent through the slot"
            );
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(1)), "{group}: keep-through");
        }
        // The detection twin of the witness slot, as
        // pinned in R23-B: the clause is detected and
        // the target reads alone.
        let group = "(moved November 1, the choir was softly gently singing to November 8)";
        let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(
            c.for_date,
            Some(nov(8)),
            "{group}: the clause reads its target"
        );
        assert_eq!(
            c.for_sunday_second_dates,
            Vec::<SimpleDate>::new(),
            "{group}: nothing is named"
        );
        // Held: three-word slots still do not spend
        // on the passive-for surface either (three
        // adverbs, or a negation carrying two more
        // slot words), the GOVERNOR scans' caps are
        // unchanged — the comma twins are judged by
        // the spent-predicate scan and read nothing
        // as they always have — and the claim scan's
        // own passive keeps the shared walk: a
        // three-word slot in a claim's passive still
        // kills the claim.
        for group in [
            "(it was softly gently quietly moved for November 1)",
            "(it was not really ever moved for November 1)",
            "(it was not really truly moved for November 1)",
            "(it was just very plain moved for November 1)",
            "(it was softly gently moved, November 1)",
            "(it was loudly moved, November 1)",
            "(moved November 1, it was not really ever moved to November 8)",
        ] {
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_date, None,
                "{group}: no spending, the date is governed"
            );
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, None, "{group}: keep-through");
        }
    }

    #[test]
    fn r24b_relative_phrase_as_subject_pp_object() {
        // Round 24 (R24-7, review M4 = red team
        // RT24-4): a relative phrase standing as a
        // subject-PP object is consumed by the
        // subject exactly as a framed date object is
        // (R23-5) — the target asserts (not
        // alternatives), and the PP object's
        // resolution is named exactly as the framed
        // twin's PP object is named. Before this
        // round the finite forms named the phantom
        // beside the target anyway, but the
        // auxiliary forms demoted the target to
        // alternatives; the whole family now reads
        // one way.
        for (group, named) in [
            (
                "(moved November 1, the choir from next Sunday sang to November 8)",
                oct(25),
            ),
            (
                "(moved November 1, the choir from this Sunday sang to November 8)",
                oct(18),
            ),
            (
                "(moved November 1, the choir on next Sunday sang to November 8)",
                oct(25),
            ),
            (
                "(moved November 1, the choir after next Sunday sang to November 8)",
                oct(25),
            ),
            (
                "(moved November 1, the choir from next Sunday in the hall sang to November 8)",
                oct(25),
            ),
            (
                "(moved November 1, the choir before this Sunday sang to November 8)",
                oct(18),
            ),
            (
                "(moved November 1, the choir before this Sunday was singing to November 8)",
                oct(18),
            ),
            (
                "(moved November 1, the choir before this Sunday had sung to November 8)",
                oct(18),
            ),
            (
                "(moved November 1, the choir before this Sunday was played to November 8)",
                oct(18),
            ),
        ] {
            let text = format!("For Sunday: next Sunday {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(
                c.for_sunday_parenthesized_date,
                Some(nov(8)),
                "{group}: the target asserts on the relative path, not alternatives"
            );
            assert_eq!(
                c.for_sunday_second_dates,
                vec![named],
                "{group}: the PP object's resolution is named, as the framed twin's is"
            );
            let text = format!("For Sunday: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, Some(nov(8)), "{group}: paren-only");
            assert_eq!(c.for_sunday_second_dates, vec![named], "{group}");
            let text = format!("Keep-through: {group}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(8)), "{group}: keep-through");
            assert_eq!(c.keep_through_second_dates, vec![named], "{group}");
        }
        // The plain twins: the relative PP object no
        // longer takes first-signal primacy over the
        // clause's framed date after "on" / "after"
        // — November 8 asserts and the resolution is
        // named, exactly as after "from".
        for (line, named) in [
            ("the choir from next Sunday sang November 8, 2026", oct(25)),
            ("the choir on next Sunday sang November 8, 2026", oct(25)),
            ("the choir after next Sunday sang November 8, 2026", oct(25)),
            (
                "the choir before this Sunday was singing to November 8",
                oct(18),
            ),
        ] {
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, Some(nov(8)), "{line}: November 8 asserts");
            assert_eq!(c.for_sunday_second_dates, vec![named], "{line}");
            let text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(8)), "{line}: keep-through");
            assert_eq!(c.keep_through_second_dates, vec![named], "{line}");
        }
        // Held: the framed twin's pin stands (the
        // PP's date is named beside the assertion);
        // the bare-weekday subject PP still consumes
        // to silence; and a relative phrase with no
        // subject PP around it — no subject head
        // before a preposition, no clause verb after
        // it — keeps its standing primacy reading.
        let line = "the team after November 1 sang November 8, 2026";
        let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(c.for_date, Some(nov(8)), "{line}: the framed twin stands");
        assert_eq!(c.for_sunday_second_dates, vec![nov(1)], "{line}");
        let line = "the choir from Sunday sang November 8, 2026";
        let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(
            c.for_date, None,
            "{line}: the bare weekday is consumed, the lone line stays silent"
        );
        let line = "next Sunday sang November 8, 2026";
        let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
        let c = parse_contribution(&text, today()).expect("parses");
        assert_eq!(
            c.for_date,
            Some(oct(25)),
            "{line}: no subject PP — the relative keeps its primacy"
        );
        assert_eq!(c.for_sunday_second_dates, vec![nov(8)], "{line}");
    }

    #[test]
    fn r24b_fence_possessive_determiner() {
        // Round 24 (R24-8, red team RT24-6 part): a
        // possessive determiner before the weekday
        // phrase does not block the lone-framed
        // fence — "our Sunday bulletins" asserts its
        // framed date like the bare shape, and the
        // phrase's head is judged exactly as before
        // (R24-3): the possessive stands before the
        // phrase, it is not part of the head test.
        for line in [
            "our Sunday bulletins, November 8, 2026",
            "my Sunday bulletins, November 8, 2026",
            "their Sunday bulletins, November 8, 2026",
            "his Sunday bulletins, November 8, 2026",
            "her Sunday bulletins, November 8, 2026",
            "your Sunday bulletins, November 8, 2026",
            "its Sunday bulletins, November 8, 2026",
            "our Sunday school, November 8, 2026",
            "our Sunday morning, November 8, 2026",
        ] {
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, Some(nov(8)), "{line}: For Sunday asserts");
            assert_eq!(
                c.for_sunday_second_dates,
                Vec::<SimpleDate>::new(),
                "{line}: nothing is named"
            );
            let text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, Some(nov(8)), "{line}: Keep-through asserts");
        }
        // Held: the head test is unchanged under a
        // possessive — an off-garnish head still
        // silences the line — and conjoined garnish
        // stays silent (accepted, recorded in the
        // round-24 synthesis; not changed here).
        for line in [
            "our Sunday picnic, November 8, 2026",
            "Sunday bulletins and Sunday school, November 8, 2026",
        ] {
            let text = format!("For Sunday: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.for_date, None, "{line}: For Sunday reads nothing");
            let text = format!("Keep-through: {line}\n\n## Announcement\n\nHi.\n");
            let c = parse_contribution(&text, today()).expect("parses");
            assert_eq!(c.keep_through, None, "{line}: Keep-through reads nothing");
        }
    }
}
