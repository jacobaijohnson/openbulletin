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
    let mut rest = text;
    loop {
        if shape(rest) {
            return true;
        }
        match strip_trailing_parenthesized(rest) {
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
/// The dates standing inside a parenthesized
/// group's words, every occurrence in written order,
/// with where each one stands: the line's own prefix
/// reader tried at each word, the same reader the
/// second-date scan uses; the
/// token index its reading starts at, and how many
/// tokens it took. The words standing before a date
/// inside its group decide whether that date may
/// contribute at all (round 11, R11-4/R11-8).
fn dates_inside_group_positional(
    inner: &str,
    today: SimpleDate,
) -> Vec<(SimpleDate, usize, usize)> {
    let tokens: Vec<&str> = inner.split_whitespace().collect();
    let mut out: Vec<(SimpleDate, usize, usize)> = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        match parse_date_prefix(&tokens[i..], today) {
            Some((d, used)) => {
                // Every occurrence stands on its own
                // (round 12, R12-2): a date restated
                // ungoverned after its governed first
                // standing — "(not November 1, November
                // 1)" — is read by its second standing,
                // so occurrences are never deduped away
                // by value here; the group analysis
                // qualifies each one and the outcome
                // dedupes by date.
                out.push((d, i, used));
                i += used.max(1);
            }
            None => i += 1,
        }
    }
    out
}

/// One group token with its wrapping punctuation off,
/// lowercased: the shape the governor and weekday
/// checks read.
fn group_token(raw: &str) -> String {
    raw.trim_matches([
        '(', ')', '[', ']', ',', '.', ':', ';', '-', '\u{2013}', '\u{2014}',
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
    let trimmed = raw.trim_end_matches([')', ']', ' ']);
    trimmed.ends_with(',')
        || trimmed.ends_with(';')
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
            if unspent_trailing_governor(clause, clause_follows_a_date) {
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
            break;
        }
    }
    start
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
fn governed_before(window: &[String]) -> bool {
    for i in 0..window.len() {
        for len in 1..=3usize {
            let Some(phrase) = window.get(i..i + len) else {
                break;
            };
            if !phrase_governs(window, i, len) {
                continue;
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
fn phrase_governs(slice: &[String], i: usize, len: usize) -> bool {
    let Some(phrase) = slice.get(i..i + len) else {
        return false;
    };
    if !is_governor_phrase(phrase) {
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
fn unspent_trailing_governor(clause: &[String], clause_follows_a_date: bool) -> bool {
    for i in 0..clause.len() {
        for len in 1..=3usize {
            let Some(phrase) = clause.get(i..i + len) else {
                break;
            };
            if !phrase_governs(clause, i, len) {
                continue;
            }
            // Round 15 (R15-1): a displacement verb
            // standing directly after an auxiliary
            // ("it was moved") is that clause's own
            // predicate, already spent on its
            // subject — the held "(it was moved,
            // November 1)" shape names its date.
            if len == 1
                && is_displacement_verb(&clause[i])
                && i > 0
                && matches!(
                    clause[i - 1].as_str(),
                    "was" | "is" | "were" | "be" | "been"
                )
            {
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
            let governs = is_cancellation_phrase(phrase)
                || (next_start.is_none() && phrase_governs(after, i, len));
            if !governs {
                continue;
            }
            // The clause the phrase stands in, and
            // every clause between the date and it,
            // judged by the one scope rule.
            let pos = seg_start + i;
            let clause_idx = clause_starts.iter().rposition(|s| *s <= pos).unwrap_or(0);
            let mut in_scope = true;
            for k in 0..clause_idx {
                let c_start = clause_starts[k];
                let c_end = clause_starts.get(k + 1).copied().unwrap_or(seg_end);
                let clause = &tokens[c_start..c_end];
                if !is_cancellation_only_clause(clause) && !is_filler_clause(clause) {
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

/// What one parenthesized group offers (round 11,
/// R11-4/R11-6/R11-8): at most one contributed date,
/// and — when the group offers two or more dates
/// instead — the offered dates as named alternatives,
/// which the account names on the unarmed second-date
/// note and which never stand as the group's date.
struct GroupAnalysis {
    contributed: Option<SimpleDate>,
    alternatives: Vec<SimpleDate>,
}

fn analyze_group(inner: &str, today: SimpleDate) -> GroupAnalysis {
    // Round 12 (R12-7b): the content reading sees
    // through balanced inner parentheses — the inner
    // content is the group's content for date-reading
    // purposes, so "((November 1))" and "(note
    // (November 1))" read the date a flat group
    // reads, and a nested governor ("(note (not
    // November 1))") still governs it.
    let unwrapped = inner.replace(['(', ')'], " ");
    if let Some(d) = parse_flexible_date(unwrapped.trim(), today) {
        let analysis = GroupAnalysis {
            contributed: Some(d),
            alternatives: Vec::new(),
        };
        return analysis;
    }
    let raw_tokens: Vec<&str> = unwrapped.split_whitespace().collect();
    let tokens: Vec<String> = raw_tokens.iter().map(|t| group_token(t)).collect();
    let positional = dates_inside_group_positional(&unwrapped, today);
    let mut qualified: Vec<(SimpleDate, usize, usize)> = Vec::new();
    // Round 15 (R15-6): dates disqualified ONLY by
    // a trailing phrase standing after the group's
    // last date — governed, but the line did name
    // them, and beside a date that stands they are
    // named on the second-date note like any other
    // distinct date the line named, never silently
    // dropped: "(November 1, November 8, drop it)"
    // asserts November 1 and names November 8.
    let mut trailing_named: Vec<SimpleDate> = Vec::new();
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
        let before_disqualified =
            governed_before(window) || split_phrasal || contradicted_before(&tokens, start, d);
        let after_disqualified = governed_after(&raw_tokens, &tokens, start + used, next_start);
        if before_disqualified || after_disqualified {
            if after_disqualified
                && !before_disqualified
                && next_start.is_none()
                && !trailing_named.contains(&d)
            {
                trailing_named.push(d);
            }
            continue;
        }
        qualified.push(*entry);
    }
    let partial = partial_or_alternative(&tokens, &qualified);
    let mut distinct: Vec<SimpleDate> = Vec::new();
    for (d, _, _) in &qualified {
        if !distinct.contains(d) {
            distinct.push(*d);
        }
    }
    if distinct.len() == 1 && partial.is_none() {
        let mut alternatives: Vec<SimpleDate> = Vec::new();
        for d in &trailing_named {
            if *d != distinct[0] && !alternatives.contains(d) {
                alternatives.push(*d);
            }
        }
        let analysis = GroupAnalysis {
            contributed: Some(distinct[0]),
            alternatives,
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
            if !alternatives.contains(d) {
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
fn group_date(inner: &str, today: SimpleDate) -> Option<SimpleDate> {
    analyze_group(inner, today).contributed
}

/// The dates a trailing group offers as alternatives
/// instead of contributing one (round 11, R11-6).
fn group_alternatives(inner: &str, today: SimpleDate) -> Vec<SimpleDate> {
    analyze_group(inner, today).alternatives
}

fn trailing_parenthesized_date(text: &str, today: SimpleDate) -> Option<SimpleDate> {
    let mut rest = text;
    while let Some((before, inner)) = split_last_trailing_group(rest) {
        if let Some(d) = group_date(inner, today) {
            return Some(d);
        }
        rest = before;
    }
    None
}

/// Every date a trailing parenthesized group names,
/// in the order the groups stand in the line (round
/// 9, R9-10): groups are walked from the end like
/// `trailing_parenthesized_date`, a group that does
/// not read as a date is garnish and is skipped, and
/// the collected dates are put back in written order.
fn trailing_parenthesized_dates(text: &str, today: SimpleDate) -> Vec<SimpleDate> {
    let mut found: Vec<SimpleDate> = Vec::new();
    let mut rest = text;
    while let Some((before, inner)) = split_last_trailing_group(rest) {
        if let Some(d) = group_date(inner, today) {
            found.push(d);
        }
        rest = before;
    }
    found.reverse();
    found
}

/// Every alternative a trailing parenthesized group
/// offers, in written order (round 11, R11-6): a
/// group that offers two or more dates contributes
/// none of them, and these are named on the line's
/// second-date note instead.
fn trailing_group_alternatives(text: &str, today: SimpleDate) -> Vec<SimpleDate> {
    let mut by_group: Vec<Vec<SimpleDate>> = Vec::new();
    let mut rest = text;
    while let Some((before, inner)) = split_last_trailing_group(rest) {
        by_group.push(group_alternatives(inner, today));
        rest = before;
    }
    let mut found: Vec<SimpleDate> = Vec::new();
    for group in by_group.into_iter().rev() {
        for d in group {
            if !found.contains(&d) {
                found.push(d);
            }
        }
    }
    found
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
fn sunday_after_relative_phrase(raw: &Option<String>) -> Option<String> {
    let text = raw.as_deref()?.trim();
    if relative_shape_matches(text, sunday_after_relative_shape) {
        Some(text.trim_end_matches(['.', ',']).to_string())
    } else {
        None
    }
}

/// Whether `text` is exactly the Sunday-after-a-
/// relative-Sunday shape, trailing weekday garnish
/// and all.
fn sunday_after_relative_shape(text: &str) -> bool {
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
    names_shape
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
    if relative_shape_matches(text, relative_sunday_shape) {
        Some(text.trim_end_matches(['.', ',']).to_string())
    } else {
        None
    }
}

/// Whether `text` is exactly a bare relative Sunday,
/// trailing weekday garnish and all.
fn relative_sunday_shape(text: &str) -> bool {
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
    names_sunday
}

/// Read a date standing at the head of `tokens`,
/// letting the line's remainder follow it (round 8,
/// R8-4): the longest opening run of words that reads
/// as a date, and how many words it took. Words after
/// the date are the remainder, not a parse failure.
fn parse_date_prefix(tokens: &[&str], today: SimpleDate) -> Option<(SimpleDate, usize)> {
    (1..=tokens.len())
        .rev()
        .find_map(|end| parse_flexible_date(&tokens[..end].join(" "), today).map(|d| (d, end)))
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
        if let Some(d) = group_date(inner, today) {
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
    fn clean(t: &str) -> String {
        t.trim_matches(['(', ')', '[', ']', ',', '.', ';'])
            .to_ascii_lowercase()
    }
    let core = strip_all_trailing_parenthesized(text.trim());
    let tokens: Vec<&str> = core.split_whitespace().collect();
    let cleaned: Vec<String> = tokens.iter().map(|t| clean(t)).collect();
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
    fn clean(t: &str) -> String {
        t.trim_matches(['(', ')', '[', ']', ',', '.', ';'])
            .to_ascii_lowercase()
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
    fn clean(t: &str) -> String {
        t.trim_matches(['(', ')', '[', ']', ',', '.', ';'])
            .to_ascii_lowercase()
    }
    let tokens: Vec<&str> = core.split_whitespace().collect();
    let cleaned: Vec<String> = tokens.iter().map(|t| clean(t)).collect();
    // Every signal in the line, in written order:
    // a date, and whether it stood framed (an
    // absolute date as written) or relative (a
    // phrase, carried here by its resolution).
    struct Signal {
        date: SimpleDate,
        framed: bool,
        // A pure relative phrase's resolution
        // against `anchor`, when this signal is
        // one; None for a framed signal and for
        // a Sunday-after phrase over a framed
        // base (its date does not turn on any
        // anchor). Used only when the line
        // proves mixed, after the scan.
        anchored: Option<SimpleDate>,
    }
    let mut signals: Vec<Signal> = Vec::new();
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
                signals.push(Signal {
                    date,
                    framed: false,
                    anchored: Some(anchored),
                });
                last_end = i + used;
                last_framed_sunday = false;
                i += used;
                continue;
            }
            if let Some((base, end)) = sunday_after_phrase_run(&tokens, &cleaned, i, today) {
                let date = first_sunday_after(base);
                signals.push(Signal {
                    date,
                    framed: false,
                    anchored: None,
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
            signals.push(Signal {
                date,
                framed: false,
                anchored: Some(anchored),
            });
            last_end = i + 2;
            last_framed_sunday = false;
            i += 2;
            continue;
        }
        match parse_date_prefix(&tokens[i..], today) {
            Some((d, used)) => {
                signals.push(Signal {
                    date: d,
                    framed: true,
                    anchored: None,
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
                if cleaned[i] == "sunday" && !(i == last_end && last_framed_sunday) {
                    let date = relative_sunday_date(false, today);
                    let anchored = relative_sunday_date(false, anchor);
                    signals.push(Signal {
                        date,
                        framed: false,
                        anchored: Some(anchored),
                    });
                    last_end = i + 1;
                    last_framed_sunday = false;
                }
                i += 1;
            }
        }
    }
    if signals.len() < 2 {
        return None;
    }
    // Round 14 (R14-6): only a truly mixed line —
    // a framed signal standing beside a pure
    // relative signal — re-resolves its relative
    // signals against the anchor. Every other
    // shape keeps the `today` resolutions the
    // scan computed.
    let is_mixed = signals.iter().any(|s| s.framed) && signals.iter().any(|s| s.anchored.is_some());
    if is_mixed {
        for s in &mut signals {
            if let Some(anchored) = s.anchored {
                s.date = anchored;
            }
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
    let groups = trailing_parenthesized_dates(text, today);
    // Round 11 (R11-6): the alternatives a trailing
    // group offers instead of contributing are named
    // on the second-date note, beside whatever the
    // line itself reads.
    let alternatives = trailing_group_alternatives(text, today);
    if let Some(read) = date {
        let parenthesized_mismatch = mismatch_from_groups(&groups, read, false);
        let mut second_dates = second_dates_in_line(text, today, read);
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
    // Round 10 (R10-10b): a group standing between a
    // month-day and its year does not poison the plain
    // date; the group is compared as garnish.
    if let Some(reading) = mid_date_group_reading(text, today, &groups, &alternatives) {
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
        let parenthesized_mismatch = mismatch_from_groups(&groups, read, true);
        let mut second_dates: Vec<SimpleDate> = Vec::new();
        for d in &alternatives {
            if *d != read && !second_dates.contains(d) {
                second_dates.push(*d);
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
            // Round 12 (R12-6): the same first-date
            // primacy on the de-grouped core, with
            // the trailing groups still compared as
            // garnish against the date it reads.
            if let Some(mut reading) = plain_multi_date_reading(core, today, anchor, &alternatives)
            {
                let read = reading.date.expect("the multi-date reading reads");
                reading.parenthesized_mismatch = mismatch_from_groups(&groups, read, false);
                return reading;
            }
            DateLineReading {
                date: None,
                weekday_mismatch: core_mismatch,
                parenthesized_mismatch: None,
                second_dates: alternatives,
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
/// anchors ONLY the mixed-signal relative resolutions
/// in `plain_multi_date_reading`. A file date that is
/// missing, not a real calendar date, or after `today`
/// falls back to `today`: the anchor never stands in
/// the future beyond today. Every other reading in
/// the parse — framed, relative-only, and the standing
/// in-words memory — is anchored exactly as before.
pub fn parse_contribution_anchored(
    text: &str,
    today: SimpleDate,
    file_date: Option<SimpleDate>,
) -> Result<Contribution, String> {
    let anchor = match file_date {
        Some(d) if d.is_valid() && d <= today => d,
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
            let groups = trailing_parenthesized_dates(kt_text, today);
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
                .and_then(|t| trailing_parenthesized_date(t, today))
        } else {
            None
        };
    // Round 9 (R9-10): every trailing parenthesized
    // date group on the relative phrase is remembered,
    // not only the last, so the verdict compares each.
    let for_sunday_parenthesized_dates =
        if for_sunday_in_words.is_some() || for_sunday_after_in_words.is_some() {
            raw.for_sunday
                .as_deref()
                .map(|t| trailing_parenthesized_dates(t, today))
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
        // The same date restated ungoverned after its
        // negation is read by its second standing.
        let text =
            "For Sunday: next Sunday (not November 1, November 1)\n\n## Announcement\n\nHi.\n";
        let c = parse_contribution(text, today()).expect("parses");
        assert_eq!(c.for_sunday_parenthesized_date, Some(nov(1)));
        assert_eq!(
            week_verdict(&c, Some(oct(25))),
            WeekVerdict::DifferentSunday(nov(1))
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
}
