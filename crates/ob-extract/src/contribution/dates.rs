//! Date and header sentences: the For Sunday and
//! Keep-through lines, their weekday and parenthesized
//! disagreements, Sunday-after shapes, and the
//! sentences the review speaks about them.

use super::composition::*;
use super::grammar::*;
use super::segments::*;
use super::*;

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
pub(super) fn mismatch_from_groups(
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
    second_dates_sentence_with_lead(seconds, "This file named")
}

/// The same note for the Keep-through line.
pub fn keep_through_second_date_sentence(second: SimpleDate) -> String {
    keep_through_second_dates_sentence(&[second])
}

/// The Keep-through note naming every distinct second
/// date (round 10, R10-6).
pub fn keep_through_second_dates_sentence(seconds: &[SimpleDate]) -> String {
    second_dates_sentence_with_lead(seconds, "The Keep-through line named")
}

/// The one second-dates note (health audit E-8): the
/// For Sunday and Keep-through sentences are the same
/// judgment — name the other dates the line carried,
/// and say to check which was meant — differing only
/// in the lead that says which line carried them.
pub(super) fn second_dates_sentence_with_lead(seconds: &[SimpleDate], lead: &str) -> String {
    if seconds.len() == 1 {
        let second = format_date_long(seconds[0]);
        return format!("{lead} another date too: {second}. Check which date was meant.");
    }
    let listed = seconds
        .iter()
        .map(|d| format_date_long(*d))
        .collect::<Vec<String>>()
        .join(" and ");
    format!("{lead} other dates too: {listed}. Check which date was meant.")
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
        self.mismatch_sentence("This part names")
    }

    /// The same sentence for the Keep-through line
    /// (round 4, R4-7): the same shape, saying which
    /// line carried the mistake.
    pub fn keep_through_sentence(&self) -> String {
        self.mismatch_sentence("This part says keep through")
    }

    /// The one weekday-mismatch sentence (health audit
    /// E-8): the For Sunday and Keep-through forms are
    /// the same judgment, differing only in the lead
    /// that says how the line carried the mistake.
    fn mismatch_sentence(&self, lead: &str) -> String {
        let actual = WEEKDAY_NAMES[(self.date.weekday() % 7) as usize];
        let short = format_date_short(self.date);
        format!(
            "{lead} {}, {short}, but that date is a {actual}. \
             Check the date with the sender.",
            self.named
        )
    }
}

/// A weekday word as an office writes one, full or
/// shortened, to its number (0 = Sunday .. 6 =
/// Saturday): the same sun..sat alias table the date
/// reader states as `weekday_number` in ob-model —
/// twins, recorded on both sides (health audit E-6).
/// The CLEANING differs on purpose: this reader first
/// strips format characters and trims five boundary
/// endings ('.', ',', ':', ';', '/'), because its
/// tokens arrive raw from a sender's text and rounds
/// 23/25 gave boundary-wearing and Cf-glued weekday
/// words a reading here, while ob-model's twin trims
/// only a trailing '.' — the flexible-date reader's
/// narrower contract (refuse what it does not plainly
/// read, on tokens whose commas are already spaces).
/// Unifying the cleaning either way would move a
/// reading, so the tables stay twins and the
/// cleanings stay each reader's own.
pub(super) fn weekday_word(word: &str) -> Option<u32> {
    // Round 23 (R23-3): a weekday wearing a boundary
    // ending (colon, semicolon, slash — the one
    // boundary classification) still names its weekday,
    // as it already did wearing a comma or period.
    // Round 25 (R25-A, M6): format characters are
    // stripped here too, as at every cleaning site.
    let cleaned = strip_format_chars(word);
    // Round 40 (R40-G, F8): the trailing trim gains
    // the ellipsis U+2026 — a leading weekday word
    // wearing one ("Monday… November 1, 2026")
    // names its weekday here, as one wearing a
    // boundary ending already does, so
    // `named_weekday_mismatch` records the mismatch
    // the clean bare form records instead of the
    // line dying with no record.
    let w = cleaned
        .trim_end_matches(['.', ',', ':', ';', '/', '\u{2026}'])
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
pub(super) fn named_weekday_mismatch(
    raw: &Option<String>,
    today: SimpleDate,
) -> Option<WeekdayMismatch> {
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
        // Round 40 (R40-G, F8): the recorded name
        // sheds the ellipsis with the comma and
        // period — the record names the word the
        // sender wrote, not its garnish dress, as
        // the clean form's record names it.
        named: named_word
            .trim_end_matches([',', '.', '\u{2026}'])
            .to_string(),
        date,
    })
}

/// Days in a month, leap years counted: the small
/// calendar helper the Sunday-after reading needs
/// (ob-model keeps its own copy private).
pub(super) fn days_in_month(year: i32, month: u32) -> u32 {
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
pub(super) fn add_days(mut d: SimpleDate, mut n: u32) -> SimpleDate {
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
pub(super) fn first_sunday_after(named: SimpleDate) -> SimpleDate {
    let until_sunday = (7 - named.weekday()) % 7;
    let step = if until_sunday == 0 { 7 } else { until_sunday };
    add_days(named, step)
}

/// How a "Sunday after [date]" line reads: the Sunday
/// it names, or the weekday mistake it carries.
pub(super) enum SundayAfter {
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
pub(super) fn sunday_after_outcome(text: &str, today: SimpleDate) -> Option<SundayAfter> {
    // Round 27 (R27-A, F3): a token that is
    // nothing but format characters is nothing on
    // this reader too — it stands neither in the
    // shape nor between its words ("the Sunday
    // \u{200b} after November 1, 2026" reads its
    // base exactly as the clean line does).
    let tokens: Vec<&str> = text
        .split_whitespace()
        .filter(|t| !cf_only_token(t))
        .collect();
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
        // Round 35 (R35-A, F1 = review R35-R1): both
        // trim sets gain the brace pair — a braced
        // weekday ("{Sunday} after ...") names its
        // weekday here as its paren and bracket twins
        // do; the pair rides on both words of the
        // shape, as the other two pairs already do.
        // Round 36 (R36-F, F12 = review R36-10): both
        // trim sets gain the sentence marks — "!", "?",
        // and the quotes, with ":" and ";" stated
        // outright — so the shape word wearing one
        // ("Sunday! after ...", "\"Sunday\" after ...")
        // names its weekday here as the ":" / ";"
        // forms already did through `weekday_word`'s
        // trailing trim. Between the two trims the
        // marks were judged unevenly: two sentence
        // marks read the phrase and the other three
        // silenced the whole line.
        // Round 37 (R37-E, F3): both trim sets gain
        // the curly quote pairs — a shape word
        // wearing curly quotes ("“Sunday” after
        // ...", "Sunday “after” ...") names its
        // word here as the straight-quoted twins
        // already do, so the phrase forms wherever
        // the straight-quoted phrase forms.
        let first = tokens[start].trim_matches([
            '(', ')', '[', ']', '{', '}', ',', '.', ':', ';', '!', '?', '"', '\'', '\u{2018}',
            '\u{2019}', '\u{201c}', '\u{201d}',
        ]);
        let second = tokens[start + 1].trim_matches([
            '(', ')', '[', ']', '{', '}', ',', '.', ':', ';', '!', '?', '"', '\'', '\u{2018}',
            '\u{2019}', '\u{201c}', '\u{201d}',
        ]);
        // Round 29 (R29-A, F8 = review MINOR-1): the
        // phrase's two words are judged unglued on
        // this whole-line reader too — "the *Sunday
        // *after November 1, 2026" is the Sunday-after
        // phrase, exactly as its spaced twin is, and
        // glue on either word alone loses nothing.
        // The keep-plain whole-line relative readers
        // (`relative_sunday_shape`,
        // `sunday_after_relative_shape`) are a
        // different shape family and are untouched.
        let first = unglued_word(first);
        let second = unglued_word(second);
        if weekday_word(&first) == Some(0) && second.eq_ignore_ascii_case("after") {
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
pub(super) fn sunday_after_base(tokens: &[&str], today: SimpleDate) -> Option<SundayAfter> {
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
    // Round 37 (R37-G): the trailing weekday's raw
    // token, kept when it closes the line — the
    // group-sequence walk below judges the sequence
    // standing after the base in written order, and
    // a lone trailing weekday taken out here stands
    // last in that sequence when companions precede
    // it ("(notes)(Monday)").
    let mut trailing_token: Option<&str> = None;
    let mut core: Vec<&str> = tokens.to_vec();
    if let Some(i) = last_word {
        // Round 34 (R34-C, F4 = RT34-5): the trailing
        // word's trim set carries the same sentence
        // marks as the date line's (`clean` and
        // `date_line_inner`), so a base garnish
        // wearing ";", ":", "!", "?", or a quote is
        // judged here too.
        // Round 35 (R35-A, F1): the set gains the
        // brace pair, so a braced base garnish is
        // judged here as its paren and bracket twins
        // are.
        // Round 35 (R35-C, F7): the set gains the
        // apostrophe and the typographic closing
        // quote, as the date line's trailing sets do —
        // a base garnish wearing either is judged here
        // too.
        // Round 37 (R37-E, F2): the set gains the
        // rest of the curly family — the opening
        // quotes and the single closing quote join
        // the double closing quote — so a base
        // garnish wearing a full curly pair
        // ("... “Monday”") is judged here as its
        // straight-quoted twin is; the pair was
        // only half trimmed before, and the garnish
        // passed as ordinary text.
        // Round 40 (R40-G, F8): the set gains the
        // ellipsis U+2026, as the plain line's
        // trailing sets do — a base garnish wearing
        // one ("Sunday after November 1, 2026
        // Monday…") is judged here as one wearing
        // ";" already is, instead of the line
        // serving its resolution with no record.
        // Round 41 (R41-E, F6): the base garnish is
        // read through the rest of the R40-G
        // sequence-tail set — "~", "*", "#", "-",
        // "—" (U+2014) — shed at the word's tail
        // only, between the two standing trims, as
        // at the plain line's trailing slot, so a
        // base garnish wearing one ("Sunday after
        // November 1, 2026 Monday~") is judged here
        // as one wearing the ellipsis already is,
        // instead of serving the resolution with
        // the record lost.
        // Round 42 (R42-F, F9 = red team RT42-S4):
        // the tail set gains the en dash (U+2013),
        // the set's own hyphen's codepoint sibling,
        // judged at this slot as "-" and "—" are —
        // a base garnish wearing one takes its
        // twins' reading here, as at the plain
        // line's trailing slot.
        let word = tokens[i]
            .trim_matches([
                '(', ')', '[', ']', '{', '}', ',', '.', ';', ':', '!', '?', '"', '\'', '\u{201d}',
                '\u{201c}', '\u{2018}', '\u{2019}', '\u{2026}', '\u{ff01}', '\u{ff1f}', '\u{ff0c}',
                '\u{ff0e}', '\u{ff1a}', '\u{ff1b}',
            ])
            .trim_end_matches([
                '~', '*', '#', '-', '\u{2014}', '\u{2013}', '\u{2010}', '\u{2011}', '\u{2212}',
                '\u{ff0d}', '\u{ff0a}', '\u{ff5e}', '\u{ff03}', '\u{2dc}', '\u{00ad}',
            ])
            .trim_matches([
                '(', ')', '[', ']', '{', '}', ',', '.', ';', ':', '!', '?', '"', '\'', '\u{201d}',
                '\u{201c}', '\u{2018}', '\u{2019}', '\u{2026}', '\u{ff01}', '\u{ff1f}', '\u{ff0c}',
                '\u{ff0e}', '\u{ff1a}', '\u{ff1b}',
            ]);
        // Round 39 (R39-E, F7): the base's trailing
        // garnish word is read through
        // `cleaned_weekday`, so its interior sentence
        // marks are shed as at the plain line's
        // garnish slots (the bare path's whole scope
        // since round 40, R40-F) — "Sunday after
        // November 1, 2026 (Mon!day)" is judged here
        // as its bare twin is.
        if let Some((named, named_number)) = cleaned_weekday(word) {
            trailing = Some((named, named_number));
            // Round 41 (R41-E, F6): a spaced suffix
            // token of the sequence-tail set standing
            // after the garnish ("Sunday after
            // November 1, 2026 Monday ~") is the
            // garnish's dress, as the glued suffix
            // is — when every token after the garnish
            // is such a token, the garnish closes the
            // line and those tokens leave the core
            // with it, so the base reads whole and
            // the record stands instead of the line
            // serving its resolution in silence.
            let dressed_close = tokens[i + 1..].iter().all(|t| suffix_tail_token(t));
            if dressed_close {
                trailing_token = Some(tokens[i]);
            }
            core = tokens
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i && !(dressed_close && *j > i))
                .map(|(_, t)| *t)
                .collect();
        }
    }
    if let Some((base, used)) = parse_date_prefix(&core, today) {
        // Round 37 (R37-G): a group SEQUENCE standing
        // directly after the base is judged before
        // the lone trailing word below — the first
        // disagreeing weekday group in the sequence
        // records the lone form's mismatch.
        if let Some(m) = base_trailing_sequence_mismatch(&core, used, trailing_token, base, today) {
            return Some(SundayAfter::Mismatch(m));
        }
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
    // Round 36 (R36-B, F2): the base's leading
    // weekday may stand wrapped — a bracket pair,
    // a brace pair, a paren pair, doubled or mixed —
    // and this reader judged it through
    // `weekday_word` on the raw token, which never
    // unwraps a pair, so a wrapped framing weekday
    // was not seen at all, the phrase reading
    // failed, and the line fell to the generic
    // reading that adopts the base itself. The
    // weekday is asked for as the phrase scan asks
    // (round 35, R35-H): through the shared
    // cleaning, which reads a paren pair through
    // its edges, and through the raw token's
    // bracketed word for the pairs the cleaning
    // reads as garnish.
    let lead_weekday =
        |t: &str| weekday_word(&group_token(t)).or_else(|| bracketed_weekday_word(t));
    let (lead_word, lead_rest): (&str, &[&str]) = match core.first() {
        Some(first) if lead_weekday(first).is_some() => (*first, &core[1..]),
        Some(first) if first.eq_ignore_ascii_case("on") => {
            let second = core.get(1)?;
            lead_weekday(second)?;
            (*second, &core[2..])
        }
        _ => return None,
    };
    let lead_number = lead_weekday(lead_word)?;
    let (base, used) = parse_date_prefix(lead_rest, today)?;
    if base.weekday() != lead_number {
        // Round 35 (R35-A, F1): the named word's
        // trim set gains the brace pair, as at the
        // weekday readers above.
        // Round 37 (R37-F, F11 = review R37-R8): the
        // named string is the cleaned weekday — the
        // wrapper and sentence-mark residue the old
        // trim could not reach (a closing bracket
        // standing behind a trailing mark, "Monday]!",
        // or behind a trailing typographic quote,
        // "Monday]”") is shed, so the record the
        // Check surface quotes names the weekday the
        // sender wrote. A word wearing only a quote
        // pair keeps its quotes: that verbatim naming
        // is the standing twin behavior (R37-E), not
        // residue (see `base_leading_named`).
        let named = base_leading_named(lead_word);
        return Some(SundayAfter::Mismatch(WeekdayMismatch { named, date: base }));
    }
    // The leading weekday agreed (it can only have
    // blocked the whole reading by wearing "on"); the
    // trailing garnish still has its say, when it
    // stands directly after the base — a group
    // sequence first (round 37, R37-G), then the
    // lone trailing weekday.
    let consumed = core.len() - lead_rest.len() + used;
    if let Some(m) = base_trailing_sequence_mismatch(&core, consumed, trailing_token, base, today) {
        return Some(SundayAfter::Mismatch(m));
    }
    Some(match trailing {
        Some((named, named_number))
            if used == lead_rest.len() && base.weekday() != named_number =>
        {
            SundayAfter::Mismatch(WeekdayMismatch { named, date: base })
        }
        _ => SundayAfter::Date(first_sunday_after(base)),
    })
}

/// The weekday mismatch a group sequence standing
/// directly after the Sunday-after base date names,
/// when one does (round 37, R37-G, late batch on
/// the R35-H precedent, from the R37-D writer's
/// witnessed flag): the lone trailing garnish is
/// judged above — "Sunday after November 1, 2026
/// (Monday)" refuses with WeekdayMismatch(Monday@
/// 2026-11-01) — but the judgment asked only for
/// the line's last word, so a weekday group
/// standing with any companion group beside it, in
/// any spacing and either position, was served in
/// silence ("Sunday after November 1, 2026
/// (Monday)(notes)" and its mirror "(notes)
/// (Monday)" both resolved to 2026-11-08). This is
/// a third slot beside the plain line's leading
/// and trailing slots, which R37-D's
/// `group_sequence_weekday_mismatch` walks; the
/// walk here is the same step inside the base's
/// own machinery. `core` is the base's token list
/// with the lone trailing weekday (if any) taken
/// out, `consumed` is how much of it the base date
/// itself consumed, and `trailing_token` is that
/// taken-out weekday's raw token when it closed
/// the line — it stands last in written order, so
/// it closes the sequence the walk reads. The
/// garnish text is the tail of the base's last
/// token from its first group opener (a garnish
/// glued against the year rides inside that
/// token), the tokens standing after the base,
/// and that trailing token. A sequence is two or
/// more top-level groups covering that text
/// whole — sentence punctuation and a standalone
/// separator token after the last group aside, as
/// the standing trailing readers peel them —
/// separated only by whitespace, commas, and the
/// sentence separators ";" "." ":", the
/// separations the lone forms allow (round 38,
/// R38-B, F2 — the lone garnish is judged wearing
/// any of those marks, "Sunday after November 1,
/// 2026 (Monday);" included, and its suffix peel
/// below carries the typographic closing quote
/// U+2019 the lone readers' trim sets carry);
/// round 39 (R39-C, F4) completes the boundary
/// sets at this slot as at the plain slots: a
/// group's suffix marks "!", "?", and U+201D
/// stand in the gaps (arm (a)), head separators
/// standing before the garnish are taken off, and
/// a single group standing behind those head
/// separators is walked (arm (c)), as at the
/// plain trailing slot. Every weekday group in it is judged on its
/// contents against the BASE date, and the first
/// disagreeing one in written order records the
/// lone form's mismatch. An agreeing (or
/// weekday-free) sequence names nothing here, so
/// the standing readings consume it exactly as
/// they consume the lone agreeing garnish.
/// Companion contents keep their standing
/// judgments at this slot throughout: the walk
/// adopts nothing and governs nothing — a
/// governing or date-carrying companion standing
/// after the base is garnish here, as it is when
/// it stands alone — and words standing after the
/// sequence are the line's remainder, which ends
/// the sequence, so the R5-D bare-word arm is
/// untouched by construction. Round 42 (R42-B,
/// F5 arm (b), adjudication 4): ordinary bare
/// words standing BEFORE the sequence no longer
/// end it when the sequence is a governor/weekday
/// company — the company behind a word takes the
/// composed reading, as at the direct slot (see
/// the arm in the body); every non-company shape
/// behind a word keeps the R5-D reading.
fn base_trailing_sequence_mismatch(
    core: &[&str],
    consumed: usize,
    trailing_token: Option<&str>,
    base: SimpleDate,
    today: SimpleDate,
) -> Option<WeekdayMismatch> {
    let mut garnish = String::new();
    if let Some(last) = consumed.checked_sub(1).and_then(|i| core.get(i)) {
        if let Some(open) = last.find(['(', '[', '{']) {
            garnish.push_str(&last[open..]);
        }
    }
    for token in &core[consumed.min(core.len())..] {
        if !garnish.is_empty() {
            garnish.push(' ');
        }
        garnish.push_str(token);
    }
    if let Some(token) = trailing_token {
        if !garnish.is_empty() {
            garnish.push(' ');
        }
        garnish.push_str(token);
    }
    // Round 43 (R43-C, F5 MINOR = review F43-3): a
    // quoted member standing in the garnish — a
    // whole group wearing quotes instead of parens
    // ("Sunday after November 1, 2026 \"cancelled\"
    // (Monday)", "…(cancelled)\"Monday\"") — is
    // re-dressed as its bracket twin
    // (`quoted_members_as_parens`, composition.rs)
    // before the walk below: this walk never
    // re-dressed one, so the company collapsed and
    // the resolution served in silence where the
    // paren twin refuses with the record. The
    // garnish is this walk's own string, so the
    // re-dress is local to it; a quoted DATE group
    // is untouched by the consultation and keeps
    // its standing reading.
    // Round 44 (R44-C, F5 MAJOR as adjudicated):
    // the consultation gains the INERT-including
    // re-dress (see `quoted_members_as_parens`),
    // so a quoted inert member standing in the
    // garnish counts toward this walk's sequence
    // as its bracket twin does — under the same
    // anchor gate as the trailing sequence walk:
    // the inert-including reading is taken only
    // where a member re-dress already completes a
    // sequence here (its span count strictly
    // grows to two or more) or a member of the
    // completed sequence stands in the garnish in
    // brackets as written, never from whole cloth
    // (a garnish standing in quotes alone,
    // `"Monday" "notes"`, keeps its standing
    // reading). The I11/I12 parity stands: a
    // governor in the garnish is garnish at this
    // slot in every dress, so the completed
    // sequence still names no weekday record for
    // the governor-only company and the resolution
    // serves identically.
    // Round 44 (R44-E, F4 MINOR = red team
    // RT44-C3): a company standing in the garnish
    // in fullwidth dress pairs — "（cancelled）
    // （Monday）", and the mixed dress "(cancelled)
    // （Monday）" — is no bracket span to the walk
    // below, so the company collapsed and the
    // resolution served in silence where the
    // ASCII twin refuses with the record. The
    // garnish is re-dressed first
    // (`fullwidth_groups_as_ascii`, composition.rs)
    // — it is this walk's own string, so the
    // re-dress is local to it, and the quoted
    // consultation below then reads the garnish
    // with its groups standing as written.
    let mut garnish = match fullwidth_groups_as_ascii(&garnish) {
        Some(redressed) => redressed,
        None => garnish,
    };
    let member_redressed = quoted_members_as_parens(&garnish, today, false);
    let full_redressed = quoted_members_as_parens(&garnish, today, true);
    if let Some(full) = full_redressed {
        let member_completed = member_redressed.as_ref().is_some_and(|member| {
            let member_spans = balanced_spans(member).len();
            member_spans >= 2 && member_spans > balanced_spans(&garnish).len()
        });
        let anchored = {
            let bracket_inners: Vec<&str> = balanced_spans(&garnish)
                .iter()
                .map(|&(s, e)| garnish[s + 1..e - 1].trim())
                .collect();
            balanced_spans(&full)
                .iter()
                .any(|&(s, e)| bracket_inners.contains(&full[s + 1..e - 1].trim()))
        };
        if member_completed || anchored {
            garnish = full;
        } else if let Some(member) = member_redressed {
            garnish = member;
        }
    }
    // Round 42 (R42-F, F9 = red team RT42-S4): an
    // en dash (U+2013) standing at the garnish's
    // own tail was taken off before the walk
    // below, and the walk's answer stood only when
    // the sequence's FINAL group was a weekday
    // group (the gate below, past the gap walk).
    // Round 44 (R44-A, F1 = review F44-1): that
    // gate is retired — the peel set below gains
    // the mark itself, so the walk answers for the
    // en dash exactly as for its twins, and a
    // base-trailing sequence closing on an inert
    // group with a weekday member standing in it
    // ("Sunday after November 1, 2026
    // (Monday)(notes)–") refuses with its record
    // instead of serving the resolution silently.
    let mut rest = garnish.as_str();
    loop {
        // Round 38 (R38-B, F2): the peel set gains
        // U+2019, as at the plain trailing walk —
        // the lone base garnish wearing it ("Sunday
        // after November 1, 2026 (Monday)’") is
        // judged, and its pair U+201D was already
        // here.
        // Round 40 (R40-G, F8): the peel set gains
        // the ellipsis U+2026, as at the plain
        // trailing walk — a base sequence closing
        // the line in front of one ("Sunday after
        // November 1, 2026 (Monday)(notes)…") is the
        // sequence it names, so its weekday groups
        // are judged against the base.
        // Round 41 (R41-E, F6): the peel set gains
        // the rest of the R40-G sequence-tail set —
        // "~", "*", "#", "-", "—" (U+2014) — as at
        // the plain trailing walk, and a spaced
        // suffix token of the set is taken with the
        // standing separator tokens below.
        // Round 44 (R44-B, F2 MAJOR = red team
        // RT44-C1): the peel set gains the fullwidth
        // quote closers U+FF02 and U+FF07 — a
        // base-trailing sequence closing on one
        // ("(cancelled)(Monday)＂") was no sequence
        // and served the resolution with its record
        // lost, where the straight closers standing
        // in this set are judged. The fullwidth
        // separators do not join this peel: the
        // suffix seat is R44-D's, not this batch's.
        // Round 44 (R44-D, F3 + F7): that suffix
        // seat is taken now, as at the plain
        // trailing walks — the peel set gains
        // U+FF5E, U+FF03, U+FF0E, U+FF0C, U+FF1A,
        // U+FF1B, and U+02DC, each beside its twin
        // standing in this set; the spaced-token
        // predicate below gains only the "~" / "#"
        // family (U+FF5E, U+FF03, U+02DC).
        let peeled = rest.trim_end().trim_end_matches([
            '.', ',', ';', ':', '!', '?', '"', '\'', '\u{201d}', '\u{2019}', '\u{2026}', '~', '*',
            '#', '-', '\u{2014}', '\u{2013}', '\u{2010}', '\u{2011}', '\u{2212}', '\u{ff0d}',
            '\u{ff0a}', '\u{ff01}', '\u{ff1f}', '\u{ff02}', '\u{ff07}', '\u{ff5e}', '\u{ff03}',
            '\u{ff0e}', '\u{ff0c}', '\u{ff1a}', '\u{ff1b}', '\u{2dc}', '\u{00ad}',
        ]);
        if peeled.len() != rest.len() {
            rest = peeled;
            continue;
        }
        match rest.rsplit_once(char::is_whitespace) {
            Some((head, token))
                if !token.is_empty()
                    && token.chars().all(|c| {
                        matches!(
                            c,
                            '*' | '|'
                                | '~'
                                | '#'
                                | '-'
                                | '\u{2014}'
                                | '\u{2013}'
                                | '\u{2010}'
                                | '\u{2011}'
                                | '\u{2212}'
                                | '\u{ff0d}'
                                | '\u{ff0a}'
                                | '\u{ff5e}'
                                | '\u{ff03}'
                                | '\u{2dc}'
                                | '\u{00ad}'
                        )
                    }) =>
            {
                rest = head;
            }
            _ => break,
        }
    }
    let rest = rest.trim_end();
    // Round 39 (R39-C, F4, arm (c)): head
    // separators standing before the garnish are
    // the walk's head dress, as at the plain
    // slots — "Sunday after November 1, 2026
    // ;(Monday)(notes)" is the unseparated twin's
    // sequence standing behind a ";". Before this
    // the first span had to stand at the garnish's
    // first byte, so the separator ended the walk.
    let head_trimmed = rest.trim_start_matches([' ', '\t', ',', ';', '.', ':']);
    let head_had_separator = rest[..rest.len() - head_trimmed.len()]
        .chars()
        .any(|c| matches!(c, ',' | ';' | '.' | ':'));
    let rest = head_trimmed;
    let spans = balanced_spans(rest);
    if spans.len() < 2 && !(spans.len() == 1 && head_had_separator) {
        return None;
    }
    if spans[spans.len() - 1].1 != rest.len() {
        return None;
    }
    if spans[0].0 != 0 {
        // Round 42 (R42-B, F5 arm (b), round-42
        // synthesis adjudication 4): the company
        // standing behind ordinary bare words
        // between the base and the sequence — "Sunday
        // after November 1, 2026 notes
        // (cancelled)(Monday)" — is the direct-slot
        // company one word further in, and it takes
        // the composed reading the direct slot takes
        // (refused, with the weekday record) instead
        // of serving the resolution in silence. The
        // R41-B controls pin that named the
        // behind-a-word serve is restated for exactly
        // this arm, flagged there and in the batch
        // record; its scope was the direct slot.
        // The arm is the COMPANY's: the words before
        // the first span must be ordinary bare words
        // (each token cleaning to one alphabetic
        // word), the sequence must hold two or more
        // groups, and among them must stand both a
        // governor group (a governor phrase at its
        // contents' own head, as the company walk
        // judges it) and a weekday group. A weekday
        // sequence with no governor companion, and
        // every other shape standing behind a word,
        // keeps the standing R5-D adjacency reading
        // at this slot, untouched.
        let prefix = &rest[..spans[0].0];
        let words_are_bare = !prefix.trim().is_empty()
            && prefix.split_whitespace().all(|t| {
                let w = group_token(t);
                !w.is_empty() && w.chars().all(|c| c.is_alphabetic())
            });
        let company = spans.len() >= 2
            && spans
                .iter()
                .any(|&(s, e)| governor_group_at_head(&rest[s + 1..e - 1]))
            && spans
                .iter()
                .any(|&(s, e)| garnish_group_weekday(&rest[s + 1..e - 1]).is_some());
        if !words_are_bare || !company {
            return None;
        }
    }
    // Round 42 (R42-C, F6 MAJOR = red team
    // RT42-S1): the inter-group seat at this
    // base-trailing sequence takes the witnessed
    // suffix set ("~", "*", "#", "-", "—" U+2014)
    // and the ellipsis (U+2026) in its gaps, as the
    // line-final seat at this same slot already
    // does through the tail peels above (R41-E) —
    // scoped to the COMPANY's sequences, judged
    // exactly as the behind-a-word arm above judges
    // company-ness (a governor phrase at some
    // span's contents' own head, and a weekday
    // group among the spans). A non-company
    // sequence wearing the set between its groups
    // keeps the standing reading below, unjudged
    // here; and no quote mark joins THIS set —
    // the straight closer standing between a
    // company's groups at this slot is a different
    // seat (the garnish walk's, R42-B deviation
    // (c)) and is not claimed here. Round 43
    // (R43-C, F4): that seat is claimed now — the
    // straight closers join the FIRST gap set
    // below (this walk is the garnish walk
    // deviation (c) named), at every sequence this
    // walk judges, the weekday-only sequences
    // included; the company set below carries them
    // too, as it carries their curly twins.
    let is_company = spans
        .iter()
        .any(|&(s, e)| governor_group_at_head(&rest[s + 1..e - 1]))
        && spans
            .iter()
            .any(|&(s, e)| garnish_group_weekday(&rest[s + 1..e - 1]).is_some());
    for pair in spans.windows(2) {
        let gap = &rest[pair[0].1..pair[1].0];
        // Round 38 (R38-B, F2): a gap may carry the
        // sentence separators ";" "." ":" and a
        // U+2019 standing as the preceding group's
        // suffix, as at the plain slots' walks.
        // Round 39 (R39-C, F4, arm (a)): a gap may
        // also carry "!", "?", and U+201D standing
        // as the preceding group's suffix, the three
        // marks the lone forms judge a group wearing.
        // Round 43 (R43-C, F4 MAJOR = red team
        // RT43-B1): a gap may also carry the
        // straight quote closers '"' and '\''
        // standing as the preceding group's suffix —
        // at this slot the straight forms ended the
        // walk where their curly twins U+201D /
        // U+2019 are judged: the named company
        // "(cancelled)\"(Monday)" served the
        // resolution with its record lost, the
        // reversed and three-group companies served
        // in silence, and the weekday-only sequence
        // "(notes)\"(Monday)" served in silence
        // where its clean twin refuses with the
        // record — the seat R42-B deviation (c)
        // named and left.
        // Round 44 (R44-B, F2 MAJOR = red team
        // RT44-C1): the first gap set gains the
        // fullwidth separators (U+FF0C, U+FF1B,
        // U+FF1A, U+FF0E, U+3001, U+3002) and the
        // fullwidth closers (U+FF02, U+FF07) — the
        // named company "(cancelled)，(Monday)" and
        // the weekday-only sequence served in
        // silence through them, where their ASCII
        // and straight twins refuse with the record.
        if gap.chars().all(|c| {
            matches!(
                c,
                ',' | ';'
                    | '.'
                    | ':'
                    | '!'
                    | '?'
                    | '"'
                    | '\''
                    | '\u{201d}'
                    | '\u{2019}'
                    | '\u{ff0c}'
                    | '\u{ff1b}'
                    | '\u{ff1a}'
                    | '\u{ff0e}'
                    | '\u{3001}'
                    | '\u{3002}'
                    | '\u{ff02}'
                    | '\u{ff07}'
            ) || c.is_whitespace()
        }) {
            continue;
        }
        // Round 43 (R43-A, F1): the company gap set
        // gains the en dash (U+2013) — the
        // base-trailing company wearing it between
        // its groups ("(cancelled)–(Monday)"),
        // glued or spaced, served the resolution
        // silently where both twins refuse with the
        // record against the base. The widening
        // stays company-scoped exactly as R42-C
        // scoped it: a non-company sequence wearing
        // the mark between its groups keeps its
        // standing reading.
        // Round 44 (R44-B, F2 MAJOR = red team
        // RT44-C1): the company gap set gains the
        // fullwidth separators (U+FF0C, U+FF1B,
        // U+FF1A, U+FF0E, U+3001, U+3002) and the
        // fullwidth closers (U+FF02, U+FF07), as the
        // first gap set above gains them — the
        // company-scope of this tier stands as
        // R42-C scoped it.
        if !is_company
            || !gap.chars().all(|c| {
                matches!(
                    c,
                    ',' | ';'
                        | '.'
                        | ':'
                        | '!'
                        | '?'
                        | '"'
                        | '\''
                        | '\u{201d}'
                        | '\u{2019}'
                        | '\u{2026}'
                        | '~'
                        | '*'
                        | '#'
                        | '-'
                        | '\u{2014}'
                        | '\u{2013}'
                        | '\u{2010}'
                        | '\u{2011}'
                        | '\u{2212}'
                        | '\u{ff0d}'
                        | '\u{ff0a}'
                        | '\u{ff01}'
                        | '\u{ff1f}'
                        | '\u{ff0c}'
                        | '\u{ff1b}'
                        | '\u{ff1a}'
                        | '\u{ff0e}'
                        | '\u{3001}'
                        | '\u{3002}'
                        | '\u{ff02}'
                        | '\u{ff07}'
                        | '\u{ff5e}'
                        | '\u{ff03}'
                        | '\u{2dc}'
                        | '\u{00ad}'
                ) || c.is_whitespace()
            })
        {
            return None;
        }
    }
    // Round 42 (R42-F, F9) stood an en-dash gate
    // here — a garnish the mark closed was this
    // walk's only when its final group was a
    // weekday group. Round 44 (R44-A) retires it
    // with the gate at the walk's head: the mark
    // stands in the peels above, so the walk's
    // answer stands for it as for every twin.
    for (start, end) in &spans {
        let inner = &rest[start + 1..end - 1];
        if let Some((named, number)) = garnish_group_weekday(inner) {
            if base.weekday() != number {
                return Some(WeekdayMismatch { named, date: base });
            }
        }
    }
    None
}

/// The name the Sunday-after base's leading-weekday
/// mismatch record carries (round 37, R37-F, F11 =
/// review R37-R8): the sender's weekday word with its
/// wrapper and sentence-mark residue shed, in
/// whichever order the wrapper and the marks stand —
/// "[Monday]!" names "Monday", as "(Monday)!",
/// "{Monday}?", and the bare "Monday!" do, and a
/// trailing typographic quote standing behind a
/// wrapper ("[Monday]”") sheds with the wrapper. The
/// one form that keeps its marks is a word wearing
/// only a quote pair — straight or curly, bare or
/// standing inside a shed wrapper ("\"Monday\"",
/// "“Monday”", "(“Monday”)"): at this site the
/// record names the sender's quoted word verbatim,
/// exactly as the straight twin's record names its
/// own (the standing reading R37-E pinned stated
/// outright) — the quotes are the word's own dress,
/// not residue standing against it.
fn base_leading_named(lead_word: &str) -> String {
    // The wrapper and sentence marks shed first,
    // quotes aside: what remains decides the form.
    let shed = lead_word.trim_matches(['(', ')', '[', ']', '{', '}', ',', '.', ';', ':', '!', '?']);
    // A quote pair standing at the shed word's own
    // edges, around the weekday itself: the quoted
    // word, named verbatim.
    let inner = shed.trim_matches(['"', '\'', '\u{201c}', '\u{201d}', '\u{2018}', '\u{2019}']);
    if inner.len() < shed.len() && weekday_word(inner).is_some() {
        return shed.to_string();
    }
    // Every other form names the cleaned weekday:
    // the full edge trim, quotes and typographic
    // quotes included, reaches the wrapper marks
    // standing behind them.
    lead_word
        .trim_matches([
            '(', ')', '[', ']', '{', '}', ',', '.', ';', ':', '!', '?', '"', '\'', '\u{2018}',
            '\u{2019}', '\u{201c}', '\u{201d}',
        ])
        .to_string()
}

/// The opener byte index of the group that ends
/// `text` (which must end with a closer), found by
/// balancing: a nested group ("(note (November
/// 1))") is one trailing group, not a defect that
/// poisons the line it trails (round 11, R11-6).
/// Round 34 (R34-E, F3): brackets and braces are
/// group delimiters alongside parentheses — a
/// bracketed group is judged as its paren twin
/// (the adjudicated reading) — so the balance
/// walk is a stack over all three pairs: a closer
/// closes the opener it matches, and a closer
/// matching nothing on the stack closes nothing.
/// On parenthesis-only text the stack is the
/// depth count it replaces, step for step.
fn trailing_group_open(text: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut stack: Vec<u8> = Vec::new();
    for i in (0..bytes.len()).rev() {
        match bytes[i] {
            b')' | b']' | b'}' => stack.push(bytes[i]),
            b'(' | b'[' | b'{' => {
                let want = match bytes[i] {
                    b'(' => b')',
                    b'[' => b']',
                    _ => b'}',
                };
                if stack.last() == Some(&want) {
                    stack.pop();
                    if stack.is_empty() {
                        return Some(i);
                    }
                }
            }
            _ => {}
        }
    }
    None
}

/// A line's token-edge facts, computed once
/// (round 40, R40-B, F1): the byte position of
/// every whitespace character in the line and the
/// alphanumeric population of every byte range,
/// as a prefix count over its characters — the
/// table pattern R39-G built inside
/// `relative_shape_matches`, stated once for
/// every strip loop the date readers run.
///
/// The loops this serves strip one trailing
/// group at a time, and each step used to
/// re-derive its edge facts by scanning: the
/// nothing-token peel inside the trailing-group
/// split walked back from the remainder's end to
/// the first alphanumeric or whitespace, and the
/// boundary-token strips in the relative-shape
/// loop scanned in from their edge the same way.
/// On a line holding a letter those scans stop
/// within a token; on a line of empty groups
/// — one letterless glued token, no whitespace
/// anywhere — every scan ran the whole
/// remaining line, one scan per group, across
/// several whole-line passes: a 15,405-byte line
/// of "()"×7,680 did not finish in 90 s. The
/// facts those scans were after are properties
/// of the line, not of the step, so they are
/// computed once here and each step reads them
/// off the table: the peel asks for the last
/// whitespace standing in its range and whether
/// any alphanumeric stands in the token after
/// it; the boundary strips ask the same two
/// questions at their edge. The judgments
/// themselves are the standing ones — the peel
/// still judges its token by `nothing_token`,
/// the split still finds its group by the
/// balance walk — only the search for the
/// token's edge is tabled, so the strip loops'
/// total work is linear in the line.
pub(super) struct LineScan<'a> {
    text: &'a str,
    alnum_prefix: Vec<u32>,
    ws_starts: Vec<usize>,
}

impl<'a> LineScan<'a> {
    pub(super) fn new(text: &'a str) -> Self {
        let mut alnum_prefix = vec![0u32; text.len() + 1];
        let mut ws_starts: Vec<usize> = Vec::new();
        let mut count = 0u32;
        for (i, c) in text.char_indices() {
            alnum_prefix[i] = count;
            if c.is_alphanumeric() {
                count += 1;
            }
            if c.is_whitespace() {
                ws_starts.push(i);
            }
            alnum_prefix[i + c.len_utf8()] = count;
        }
        LineScan {
            text,
            alnum_prefix,
            ws_starts,
        }
    }

    /// The alphanumeric characters standing in
    /// `text[a..b]` (both char boundaries).
    fn alnum_between(&self, a: usize, b: usize) -> u32 {
        self.alnum_prefix[b] - self.alnum_prefix[a]
    }

    /// The first whitespace character standing in
    /// `text[at..hi]`, as its byte position.
    fn first_whitespace(&self, at: usize, hi: usize) -> Option<usize> {
        let i = self.ws_starts.partition_point(|&w| w < at);
        self.ws_starts.get(i).copied().filter(|&w| w < hi)
    }

    /// The last whitespace character standing in
    /// `text[lo..hi]`, as (byte position, byte
    /// length).
    fn last_whitespace(&self, lo: usize, hi: usize) -> Option<(usize, usize)> {
        let past = self.ws_starts.partition_point(|&w| w < hi);
        if past == 0 {
            return None;
        }
        let w = self.ws_starts[past - 1];
        if w < lo {
            return None;
        }
        let c = self.text[w..].chars().next()?;
        Some((w, c.len_utf8()))
    }

    /// The last trailing group of the slice
    /// `text[lo..hi]`, split as (the words before
    /// it, the group's own words) — returned as
    /// absolute byte positions: the end of the
    /// words before the group, and the group's
    /// inner range. This is the split the strip
    /// loops have always walked (the last trailing
    /// group of a line, round 8, carried shape;
    /// its opening delimiter found by balancing,
    /// R11-6 / R34-E — see `trailing_group_open`).
    /// Round 28 (R28-A, F2 = review MINOR-2): a
    /// nothing-token trailing the group in the raw
    /// text ("(moved … to November 8)
    /// \u{200b}", "… ) *") is nothing after
    /// it — the group still ends the line, so
    /// trailing whitespace-separated nothings come
    /// off before the closing-delimiter test, and
    /// the parts are parts of the text with those
    /// nothings (and the trailing '.' / ','
    /// standing over them) removed. Round 39
    /// (R39-G, F9/F11) replaced the peel's
    /// whole-text split search with a bounded
    /// reverse scan that stops at the first
    /// alphanumeric (the token holding it is never
    /// a nothing) or the first whitespace (the
    /// token after it is judged by `nothing_token`,
    /// exactly as before); round 40 (R40-B, F1)
    /// tables that scan — its two stopping facts
    /// are the table's two questions, and a slice
    /// holding no whitespace at all is one token,
    /// which the standing scan leaves standing
    /// unjudged, exactly as the table's
    /// no-whitespace answer leaves it.
    pub(super) fn split_last(&self, lo: usize, hi: usize) -> Option<(usize, usize, usize)> {
        let text = self.text;
        // Round 42 (R42-D, F10 MINOR = red team
        // RT42-D1): the end trims gain the witnessed
        // suffix set ("~", "*", "#", "-", "—"
        // U+2014, the ellipsis U+2026, and "!") — a
        // trailing group wearing one ("next Sunday
        // (November 8, 2026)~") ended its slice in
        // the mark, so the group was no trailing
        // group to this split at all: the relative
        // shape's strip loop could not take it off
        // (the alone memory was lost in both
        // dresses) and the trailing-group analyses
        // never saw it (the Keep route's
        // keep-through was lost, for "-" / "—"
        // as for the rest). The trims are the
        // standing '.' / ',' trims widened to the
        // set the trailing peels judge a group
        // wearing; `end` still lands just past the
        // closer, so the split's own facts are
        // unchanged for every slice that already
        // split.
        // Round 43 (R43-A, F1): the end trims gain
        // the en dash (U+2013) — a trailing-alone
        // group wearing it ("next Sunday (November
        // 8, 2026)–") ended its slice in the mark, so
        // the group was no trailing group to this
        // split: the For-route memory survived
        // through the relative shape's own boundary
        // trims, but the Keep route's keep-through,
        // which the fallback reads off this split,
        // was lost, where both twins keep the
        // group's date. Both trim sites in this
        // function take the mark, as they took the
        // R42-D set.
        // Round 43 (R43-B, F2 + F3): both trim sites
        // gain the sibling class whose twins stand
        // in this set — the dash siblings (U+2010,
        // U+2011, U+2212, U+FF0D), the fullwidth
        // asterisk (U+FF0A) and "!" (U+FF01), and
        // U+00AD. (U+FF1F does not join here: its
        // twin "?" stands in no trim of this set.)
        // Round 44 (R44-D, F6 MINOR = red team
        // RT44-C4): both trim sites gain "?" itself,
        // beside "!" — "?" was admitted beside "!"
        // in the suffix peels in R39-C and the two
        // are twin-identical at every other arm, so
        // the trailing-alone group wearing it
        // ("next Sunday (November 8, 2026)?") lost
        // the For-route memory and the Keep-through
        // where "!" keeps both. U+FF1F joins with
        // it: its R43-B exclusion rested expressly
        // on its twin standing in no trim of this
        // set, and that premise is retired here.
        // Both sites also gain the R44-D suffix
        // siblings whose twins stand in this set —
        // U+FF5E (of "~"), U+FF03 (of "#"), U+FF0E
        // (of "."), U+FF0C (of ",") — and U+02DC
        // beside "~" (F7). U+FF1A / U+FF1B do not
        // join: their twins ":" / ";" stand in no
        // trim of this set, the same seat logic
        // that held U+FF1F out until now.
        let mut end = lo
            + text[lo..hi]
                .trim_end_matches([
                    '.', ',', '~', '*', '#', '-', '\u{2014}', '\u{2026}', '!', '?', '\u{2013}',
                    '\u{2010}', '\u{2011}', '\u{2212}', '\u{ff0d}', '\u{ff0a}', '\u{ff01}',
                    '\u{ff1f}', '\u{ff5e}', '\u{ff03}', '\u{ff0e}', '\u{ff0c}', '\u{2dc}',
                    '\u{00ad}',
                ])
                .len();
        loop {
            let trimmed = lo + text[lo..end].trim_end().len();
            let peel_at = match self.last_whitespace(lo, trimmed) {
                Some((w, wlen)) if self.alnum_between(w + wlen, trimmed) == 0 => {
                    let last = &text[w + wlen..trimmed];
                    nothing_token(last).then_some(w)
                }
                _ => None,
            };
            match peel_at {
                Some(w) => {
                    end = lo
                        + text[lo..w]
                            .trim_end_matches([
                                '.', ',', '~', '*', '#', '-', '\u{2014}', '\u{2026}', '!', '?',
                                '\u{2013}', '\u{2010}', '\u{2011}', '\u{2212}', '\u{ff0d}',
                                '\u{ff0a}', '\u{ff01}', '\u{ff1f}', '\u{ff5e}', '\u{ff03}',
                                '\u{ff0e}', '\u{ff0c}', '\u{2dc}', '\u{00ad}',
                            ])
                            .len();
                }
                None => {
                    end = trimmed;
                    break;
                }
            }
        }
        let slice = &text[lo..end];
        if !slice.ends_with([')', ']', '}']) {
            return None;
        }
        let open = trailing_group_open(slice)?;
        let before = lo + slice[..open].trim_end().len();
        Some((before, lo + open + 1, end - 1))
    }

    /// The slice `text[lo..hi]` with a standalone
    /// boundary token at its head taken off,
    /// repeatedly: the new start. The tabled form
    /// of `strip_leading_boundary_tokens` (round
    /// 24, R24-4), judged on a slice of a scanned
    /// line: the standing scan reads the first
    /// token only as far as the judgment reads
    /// — an alphanumeric standing before any
    /// whitespace makes the token no boundary
    /// token, and a whitespace standing first
    /// ends a token with no alphanumeric in it,
    /// which comes off. The table answers the
    /// same pair of questions per token: the
    /// first whitespace at or after the head, and
    /// whether any alphanumeric stands before it.
    pub(super) fn strip_leading_boundary(&self, lo: usize, hi: usize) -> usize {
        let text = self.text;
        let mut start = hi - text[lo..hi].trim_start().len();
        loop {
            let Some(w) = self.first_whitespace(start, hi) else {
                return start;
            };
            if self.alnum_between(start, w) > 0 {
                return start;
            }
            start = hi - text[w..hi].trim_start().len();
        }
    }

    /// The trailing twin of
    /// `strip_leading_boundary`: the slice
    /// `text[lo..hi]` with a standalone boundary
    /// token at its tail taken off, repeatedly
    /// (round 24, R24-4) — the new end. The
    /// standing scan (`strip_trailing_boundary_
    /// tokens`, whose place this takes) read the
    /// last token from the end the same way: an
    /// alphanumeric or a whitespace ending the
    /// scan, the token after the last whitespace
    /// coming off exactly when it holds no
    /// alphanumeric.
    pub(super) fn strip_trailing_boundary(&self, lo: usize, hi: usize) -> usize {
        let text = self.text;
        let mut end = lo + text[lo..hi].trim_end().len();
        loop {
            let Some((w, wlen)) = self.last_whitespace(lo, end) else {
                return end;
            };
            if self.alnum_between(w + wlen, end) > 0 {
                return end;
            }
            end = lo + text[lo..w].trim_end().len();
        }
    }

    /// The successive trailing-group splits of
    /// the whole line, last group first: the
    /// sequence `split_last` yields when it is
    /// called on its own before-part until it
    /// answers None — every trailing group of
    /// the line, each as (the words before it,
    /// its inner text).
    pub(super) fn trailing_splits(&self) -> Vec<(&'a str, &'a str)> {
        let mut splits: Vec<(&'a str, &'a str)> = Vec::new();
        let mut hi = self.text.len();
        while let Some((before, inner_lo, inner_hi)) = self.split_last(0, hi) {
            splits.push((&self.text[..before], &self.text[inner_lo..inner_hi]));
            hi = before;
        }
        splits
    }
}

/// Whether `text` names a relative-Sunday shape,
/// allowing trailing parenthesized groups (round 8,
/// carried shape) to come off one by one until the
/// shape stands alone: a "(October 25)" after a
/// relative Sunday is garnish to the shape — the date
/// it names is remembered separately — the way a
/// trailing weekday garnish already is.
pub(super) fn relative_shape_matches(text: &str, shape: fn(&[String]) -> bool) -> bool {
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
    // Round 44 (R44-D, F3 MINOR = red team
    // RT44-C2): the trim gains the fullwidth
    // siblings of its four sentence members —
    // U+FF1B (of ";"), U+FF0C (of ","), U+FF1A
    // (of ":"), U+FF0E (of ".") — because the
    // trailing-alone arm is witnessed reading
    // through this trim for the twins: "next
    // Sunday (November 8, 2026):" keeps its
    // in-words memory here, while the fullwidth
    // form lost it. The dash siblings remain out
    // (R43-B deviation 4(a) stands: no witnessed
    // arm reads one through this trim, and
    // U+FF5E / U+FF03 / U+02DC do not join either
    // — their twins "~" / "#" are not members of
    // this classification; their alone arms are
    // judged through `split_last`'s trims, where
    // their twins stand).
    const BOUNDARY_TRIM: [char; 14] = [
        ' ', '\t', ';', ',', ':', '.', '/', '-', '\u{2013}', '\u{2014}', '\u{ff1b}', '\u{ff0c}',
        '\u{ff1a}', '\u{ff0e}',
    ];
    // Round 31 (R31-C, F4): the shape is judged on the
    // remainder's cleaned words, and the loop below
    // used to re-clean EVERY word of the remainder for
    // every group it stripped — thousands of repeated
    // groups on a relative-Sunday line cost quadratic
    // time. The line's tokens are now cleaned once, up
    // front; each remainder is a byte range of the one
    // text, so its words are the pre-cleaned tokens
    // inside the range, with at most the two edge
    // tokens re-cleaned from the part of them the
    // range still holds (a group taken off mid-token
    // leaves a partial token, exactly the token the
    // remainder's own cleaning would read). Both
    // shapes this loop serves match only a bounded
    // pattern: past the leading "on"/"the" run, at
    // most six words (the Sunday-after shape with its
    // trailing weekday garnish is the longest). A
    // remainder holding more than six words past that
    // run cannot match either shape, so its full word
    // list is never assembled; a remainder within the
    // bound is assembled whole and judged by the
    // shape itself, word for word as before.
    let mut toks: Vec<(usize, usize, String)> = Vec::new();
    let mut tok_start: Option<usize> = None;
    for (i, c) in text.char_indices() {
        if c.is_whitespace() {
            if let Some(s) = tok_start.take() {
                toks.push((s, i, group_token(&text[s..i])));
            }
        } else if tok_start.is_none() {
            tok_start = Some(i);
        }
    }
    if let Some(s) = tok_start {
        toks.push((s, text.len(), group_token(&text[s..])));
    }
    let ne: Vec<usize> = toks
        .iter()
        .enumerate()
        .filter(|(_, t)| !t.2.is_empty())
        .map(|(i, _)| i)
        .collect();
    // Round 39 (R39-G, F9): the alphanumeric
    // population of every byte range of `text`, as
    // a prefix count over its characters — built
    // once here so the word assembly below can
    // settle a clipped head or tail part's cleaning
    // from its size class instead of re-cleaning
    // the part whole (see `relative_shape_words`).
    // Round 40 (R40-B, F1): the count is the
    // line scan's now (`LineScan`), which also
    // tables the line's whitespace; the strip
    // loop below reads its boundary strips and
    // its trailing split off the same scan, so a
    // letterless line pays no per-group edge scan
    // here either.
    let scan = LineScan::new(text);
    let alnum_prefix: &[u32] = &scan.alnum_prefix;
    // Round 40 (R40-B, F1): the remainder is
    // tracked as a byte range of the one line, so
    // the loop's edge steps read the line scan
    // instead of re-scanning the remainder: the
    // boundary-token strips and the trailing split
    // are the scan's (`strip_leading_boundary`,
    // `strip_trailing_boundary`, `split_last`),
    // each the standing judgment judged off the
    // precomputed facts. The boundary trims and
    // the leading split keep their standing forms.
    let mut lo = 0usize;
    let mut hi = text.len();
    loop {
        lo = hi - text[lo..hi].trim_start_matches(BOUNDARY_TRIM).len();
        hi = lo + text[lo..hi].trim_end_matches(BOUNDARY_TRIM).len();
        // Round 24 (R24-4): a standalone boundary
        // token at either end of the remainder ("*",
        // "•", "|", "~", "_", an emoji — the token
        // rule, not the character table) is separator
        // punctuation standing at the shape's
        // boundary, exactly as the tabled characters
        // are: "next Sunday * (November 8)" and
        // "(November 8) * next Sunday" remember
        // their Sundays as their comma twins do.
        lo = scan.strip_leading_boundary(lo, hi);
        hi = scan.strip_trailing_boundary(lo, hi);
        lo = hi - text[lo..hi].trim_start_matches(BOUNDARY_TRIM).len();
        hi = lo + text[lo..hi].trim_end_matches(BOUNDARY_TRIM).len();
        if relative_shape_words(text, &toks, &ne, alnum_prefix, lo, hi, shape) {
            return true;
        }
        // Round 21 (R21-1): a LEADING group is garnish
        // to the shape exactly as a trailing one is —
        // "(November 8) next Sunday" names its Sunday
        // in words the way "next Sunday (November 8)"
        // does, the group's date remembered separately.
        match scan.split_last(lo, hi) {
            Some((before, _, _)) => hi = before,
            None => match strip_leading_parenthesized(&text[lo..hi]) {
                Some(stripped) => {
                    lo = stripped.as_ptr() as usize - text.as_ptr() as usize;
                }
                None => {
                    return false;
                }
            },
        }
    }
}

/// One remainder's shape verdict in
/// `relative_shape_matches` (round 31, R31-C): the
/// remainder is `text[lo..hi]`; its cleaned word
/// list is read off the line's pre-cleaned tokens
/// (see there), and the shape is consulted only
/// when the words past the leading "on"/"the" run
/// number six or fewer — the bound both shapes
/// fit — in which case the whole list is assembled
/// and the shape judges it exactly as it judged
/// the remainder's text before.
fn relative_shape_words(
    text: &str,
    toks: &[(usize, usize, String)],
    ne: &[usize],
    alnum_prefix: &[u32],
    lo: usize,
    hi: usize,
    shape: fn(&[String]) -> bool,
) -> bool {
    // Round 39 (R39-G, F9): a cut part's cleaning,
    // settled by the part's letter-and-digit count
    // where the count alone decides it. The strip
    // loop above calls here once per group it peels,
    // and on a line whose groups are glued into one
    // line-long token every call re-cleaned the
    // whole clipped remainder — the cleaning walks
    // the part end to end — so k groups paid k
    // whole-line cleanings. The count decides two
    // classes exactly:
    //
    // - No letter or digit: `group_token` answers
    //   the empty word for every such token — the
    //   round-24 rule for the plain path, and the
    //   bracketed path's governor exception reads a
    //   cleaned inner word, which no letterless part
    //   can supply (its inner word is letterless
    //   too, and no governor or cancellation phrase
    //   is letterless).
    // - More than forty letters or digits, and not
    //   a wholly bracketed token: the cleaning is
    //   the part's own word with its wrapping off —
    //   every letter and digit of the part survives
    //   `clean_group_word` in order, so the word is
    //   longer than forty characters. This assembly
    //   reads a head or tail word only for whether
    //   it is empty, whether it is "on" or "the",
    //   and through the two shapes, whose every
    //   comparison is against a literal of at most
    //   six letters or a weekday name of at most
    //   nine — a stand-in word of forty-one letters
    //   answers all of those exactly as the true
    //   word does. A bracketed part is cleaned
    //   exactly as before: its governor exception
    //   can turn on any window of the inner word,
    //   so no size class settles it.
    let clean_part = |a: usize, b: usize| -> String {
        let letters = alnum_prefix[b] - alnum_prefix[a];
        if letters == 0 {
            return String::new();
        }
        if letters > 40 && !is_bracketed_token(&text[a..b]) {
            return "x".repeat(41);
        }
        group_token(&text[a..b])
    };
    // The tokens the range holds: the first whose
    // end passes `lo`, through the last whose start
    // precedes `hi`. A token cut by either edge is
    // read from the part of it inside the range.
    let t_first = toks.partition_point(|t| t.1 <= lo);
    let t_past = toks.partition_point(|t| t.0 < hi);
    if t_first >= t_past {
        return shape(&[]);
    }
    let t_last = t_past - 1;
    let head_cut = toks[t_first].0 < lo;
    // Round 32 (R32-B, F2): the tail edge is
    // re-cleaned whenever the range cuts the token
    // there, a one-token remainder included. R31-C
    // gated the tail re-clean on a second token
    // standing in the range, so the bare "Sunday"
    // remainder left when a glued trailing group
    // came off ("Sunday(November 8, 2026)") was
    // judged on the token's full pre-cleaning
    // ("sunday(november") and read nothing, where
    // the pre-R31-C reader judged the remainder's
    // own text. When the range's one token is cut
    // at BOTH edges the head re-clean already holds
    // exactly the in-range part, so no tail word
    // stands beside it.
    let tail_cut = toks[t_last].1 > hi && !(t_last == t_first && head_cut);
    let head: Option<String> = head_cut.then(|| clean_part(lo, toks[t_first].1.min(hi)));
    let tail: Option<String> = tail_cut.then(|| clean_part(toks[t_last].0, hi));
    let first_in = if head_cut { t_first + 1 } else { t_first };
    // The whole tokens between the cut edges, as
    // the index range [first_in, past_in): a token
    // cut at either edge contributes its re-cleaned
    // part, never its pre-cleaning, and a one-token
    // remainder has no whole token at all.
    let past_in = if toks[t_last].1 > hi {
        t_last
    } else {
        t_last + 1
    };
    let (ne_lo, ne_hi) = if first_in < past_in {
        (
            ne.partition_point(|&ti| ti < first_in),
            ne.partition_point(|&ti| ti < past_in),
        )
    } else {
        let p = ne.partition_point(|&ti| ti < first_in);
        (p, p)
    };
    let head_on = head.as_ref().is_some_and(|w| !w.is_empty());
    let tail_on = tail.as_ref().is_some_and(|w| !w.is_empty());
    let total = head_on as usize + (ne_hi - ne_lo) + tail_on as usize;
    // The leading "on"/"the" run, walked over the
    // pre-cleaned words: the head word first (when
    // the range cut its token and the part still
    // cleans to a word), then the whole tokens.
    let mut run = 0usize;
    let mut pos = ne_lo;
    if head_on {
        let h = head.as_deref().unwrap_or("");
        if h == "on" || h == "the" {
            run += 1;
        }
        // Otherwise the head word is the first core
        // word: the run is empty, and the walk below
        // is skipped by its gate.
    }
    if run > 0 || !head_on {
        while pos < ne_hi {
            let w = toks[ne[pos]].2.as_str();
            if w == "on" || w == "the" {
                run += 1;
                pos += 1;
            } else {
                break;
            }
        }
    }
    if total - run > 6 {
        return false;
    }
    let mut words: Vec<String> = Vec::with_capacity(total);
    if head_on {
        words.push(head.clone().unwrap_or_default());
    }
    for p in ne_lo..ne_hi {
        words.push(toks[ne[p]].2.clone());
    }
    if tail_on {
        words.push(tail.clone().unwrap_or_default());
    }
    shape(&words)
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
pub(super) struct GroupSignal {
    pub(super) date: SimpleDate,
    pub(super) start: usize,
    pub(super) used: usize,
    pub(super) anchored: Option<SimpleDate>,
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
pub(super) fn dates_inside_group_signals(
    raw_tokens: &[&str],
    tokens: &[String],
    today: SimpleDate,
    anchor: SimpleDate,
) -> Vec<GroupSignal> {
    let mut out: Vec<GroupSignal> = Vec::new();
    // Round 31 (R31-C, F4): the next-nonempty
    // answers for this scan, read off one table
    // (see `next_nonempty_table`).
    let nonempty_next = next_nonempty_table(tokens);
    let mut last_end = 0usize;
    let mut last_framed_sunday = false;
    let mut i = 0;
    while i < tokens.len() {
        // Round 27 (R27-A, F3): the "after" and the
        // base phrase's words are the next words
        // that are something — a token that cleans
        // to nothing inside the shape ("Sunday
        // \u{200b} after next Sunday") leaves the
        // shape standing, and the span it reports
        // covers the nothing it crossed.
        let after_at = nonempty_next[i + 1];
        // Round 36 (R36-B, F3): the shape word may
        // stand wholly bracketed — the shared
        // cleaning reads a bracketed token as
        // garnish, so its cleaned form here is
        // nothing, where the paren twin's cleaning
        // gives the word itself, and a gate on the
        // cleaned token alone never opens for it:
        // the phrase run (which itself asks the raw
        // token, round 35 R35-H) was unreachable
        // inside a group, the phrase never formed,
        // and its base stood as a plain signal to be
        // recorded against the resolution. The gate
        // asks the raw token for its bracketed word,
        // as the phrase scan now does.
        let shape_sunday =
            tokens[i] == "sunday" || bracketed_weekday_word(raw_tokens[i]) == Some(0);
        if shape_sunday && after_at.is_some_and(|a| tokens[a].as_str() == "after") {
            let after_at = after_at.unwrap_or(i + 1);
            let first_at = nonempty_next[after_at + 1];
            let (which_at, base_at) = match first_at {
                Some(f) if tokens[f].as_str() == "the" => (
                    nonempty_next[f + 1],
                    nonempty_next[f + 1].and_then(|w| nonempty_next[w + 1]),
                ),
                other => (other, other.and_then(|w| nonempty_next[w + 1])),
            };
            let used = base_at.map(|b| b - i + 1).unwrap_or(0);
            let which = which_at.and_then(|w| tokens.get(w)).map(String::as_str);
            let base_day = base_at.and_then(|b| tokens.get(b)).map(String::as_str);
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
        // Round 27 (R27-A, F3): the phrase's Sunday
        // is the next word that is something after
        // the relative word ("next \u{200b}
        // Sunday" is the phrase, its span covering
        // the nothing between its words).
        let rel_sunday = if tokens[i] == "next" || tokens[i] == "this" {
            nonempty_next[i + 1].filter(|&j| tokens[j].as_str() == "sunday")
        } else {
            None
        };
        if let Some(rel_i) = rel_sunday {
            // Round 25 (R25-B, F5b = review R25-2):
            // a relative phrase standing as a PP
            // object inside a target claim's object
            // continuation is consumed exactly as
            // the bare weekday is (R24-5): it spawns
            // no signal here.
            if weekday_in_object_continuation_pp(raw_tokens, tokens, rel_i, today) {
                last_end = rel_i + 1;
                last_framed_sunday = false;
                i = rel_i + 1;
                continue;
            }
            let date = relative_sunday_date(tokens[i] == "next", today);
            let anchored = relative_sunday_date(tokens[i] == "next", anchor);
            out.push(GroupSignal {
                date,
                start: i,
                used: rel_i - i + 1,
                anchored: Some(anchored),
            });
            last_end = rel_i + 1;
            last_framed_sunday = false;
            i = rel_i + 1;
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
pub(super) fn token_has_no_letters_or_digits(raw: &str) -> bool {
    !raw.chars().any(|c| c.is_alphanumeric())
}

/// Whether a token is one bracketed unit — "[x]",
/// "{x}" (round 24, R24-4): such a token counts as
/// a boundary/garnish token in group cleaning and
/// in date spans. This test itself says nothing
/// about the text a bracket pair spans, only about
/// a single token the pair wraps whole; the span
/// the pair delimits is a group in the group
/// machinery since round 34 (R34-E, F3), judged as
/// its paren twin. Round 36 (R36-E, F5): the
/// trailing trim also takes the sentence marks
/// `! ? ' "` — a wrapped word wearing one of them
/// ("[cancelled]!") is still the one bracketed
/// unit, exactly as its paren twin "(cancelled)!"
/// is read through the paren unwrap.
pub(super) fn is_bracketed_token(raw: &str) -> bool {
    // Round 37 (R37-E, F4): the trailing trim gains
    // the typographic closing quotes — a wrapped
    // word wearing one ("[cancelled]”") is still
    // the one bracketed unit, exactly as one
    // wearing the straight closing quote is.
    let t = raw.trim_end_matches([
        ',', '.', ':', ';', '!', '?', '\'', '"', '\u{201d}', '\u{2019}',
    ]);
    (t.starts_with('[') && t.ends_with(']')) || (t.starts_with('{') && t.ends_with('}'))
}

/// The weekday a wholly bracketed token names, if
/// it names one (round 35, R35-H): the shared
/// cleaning (`group_token`) reads a bracketed
/// token as garnish and gives nothing for it, where
/// the paren twin's cleaning gives the word itself
/// ("(Sunday)" cleans through its edges to
/// "sunday"). A reader asking for the word itself —
/// the Sunday-after phrase scan asking after its
/// shape word and its base's leading weekday — asks
/// the raw token here. Only the pairs
/// `is_bracketed_token` names unwrap ([] and {});
/// a doubled pair unwraps layer by layer, as the
/// paren twin's edge trim reads "((Sunday))".
pub(super) fn bracketed_weekday_word(raw: &str) -> Option<u32> {
    let mut token: String = raw.to_string();
    while is_bracketed_token(&token) {
        // Round 37 (R37-E, F4): the extraction trim
        // gains the typographic closing quotes, as
        // `is_bracketed_token`'s own trim does — the
        // closing bracket of "[Monday]”" is reached
        // past the quote, so the wrapped weekday
        // stands.
        let trimmed = token
            .trim_end_matches([',', '.', ':', ';', '\u{201d}', '\u{2019}'])
            .to_string();
        token = trimmed[1..trimmed.len() - 1].to_string();
        if let Some(number) = weekday_word(&group_token(&token)) {
            return Some(number);
        }
    }
    None
}

/// `text` with a standalone boundary token at its
/// head taken off, repeatedly (round 24, R24-4):
/// text-level sites (relative-shape memory, the
/// composition segment weekday check) strip such a
/// token standing where punctuation was taken off
/// exactly as they strip the tabled characters.
pub(super) fn strip_leading_boundary_tokens(text: &str) -> &str {
    let mut rest = text.trim_start();
    loop {
        // Round 39 (R39-G, F9): the first token is
        // scanned only as far as the judgment reads:
        // an alphanumeric standing before any
        // whitespace makes the token no boundary
        // token (the standing test is the token's
        // own `token_has_no_letters_or_digits`, a
        // scan for exactly that character class),
        // and a whitespace standing first ends the
        // token with no alphanumeric in it. The scan
        // this replaces materialized the whole first
        // token on every call — on a line whose
        // tokens are glued group runs the first token
        // is the whole line, and the relative-shape
        // loop calls here once per group it strips.
        let mut token_end: Option<usize> = None;
        let mut has_alnum = false;
        for (i, c) in rest.char_indices() {
            if c.is_whitespace() {
                token_end = Some(i);
                break;
            }
            if c.is_alphanumeric() {
                has_alnum = true;
                break;
            }
        }
        if has_alnum {
            return rest;
        }
        let Some(end) = token_end else {
            return rest;
        };
        rest = rest[end..].trim_start();
    }
}

/// Whether `c` is a Unicode format character
/// (general category Cf): the invisible clipboard
/// characters — zero-width spaces and joiners,
/// the soft hyphen, bidi controls, the byte-order
/// mark, and their kin across the planes.
/// Round 25 (R25-A, M6 = review R25-10): token
/// cleaning strips these everywhere tokens are
/// cleaned — the ONE cleaning rule, extended by
/// character class — so "cancelled" carrying a
/// zero-width space governs again.
pub(super) fn is_format_char(c: char) -> bool {
    matches!(
        c,
        '\u{00ad}'
            | '\u{0600}'..='\u{0605}'
            | '\u{061c}'
            | '\u{06dd}'
            | '\u{070f}'
            | '\u{0890}'..='\u{0891}'
            | '\u{08e2}'
            | '\u{180e}'
            | '\u{200b}'..='\u{200f}'
            | '\u{202a}'..='\u{202e}'
            | '\u{2060}'..='\u{2064}'
            | '\u{2066}'..='\u{206f}'
            | '\u{feff}'
            | '\u{fff9}'..='\u{fffb}'
            | '\u{110bd}'
            | '\u{110cd}'
            | '\u{13430}'..='\u{1343f}'
            | '\u{1bca0}'..='\u{1bca3}'
            | '\u{1d173}'..='\u{1d182}'
            | '\u{e0001}'
            | '\u{e0020}'..='\u{e007f}'
    )
}

/// `text` with every format character (category
/// Cf) removed: the cleaning step M6 adds at every
/// site tokens are cleaned.
pub(super) fn strip_format_chars(text: &str) -> String {
    if text.chars().any(is_format_char) {
        text.chars().filter(|c| !is_format_char(*c)).collect()
    } else {
        text.to_string()
    }
}

/// Whether a raw token is nothing but format
/// characters and the punctuation they rode in on
/// (round 27, R27-A, F3): stripping the format
/// characters leaves no letter and no digit, and
/// at least one format character was there to
/// strip. Such a token cleans to nothing AND is
/// nothing — it is not the boundary punctuation a
/// standalone "*" or "|" is (those carry no format
/// character and keep their standing boundary
/// readings wherever those stand).
pub(super) fn cf_only_token(raw: &str) -> bool {
    let stripped = strip_format_chars(raw);
    stripped.len() < raw.len() && !stripped.chars().any(|c| c.is_alphanumeric())
}

/// Whether a raw token is a NOTHING token (round
/// 28, R28-A, F2 = review MAJOR-1/MINOR-3): a token
/// that is nothing but format characters
/// (`cf_only_token`), a token that is nothing but
/// this format's own separators standing alone
/// ("*", "|", "**", "*|"), or a separator glued to
/// nothing ("|\u{200b}", "*,"). A nothing token is
/// nothing at every site that reads tokens —
/// adjacency, walk-backs, slice bounds, date spans,
/// and raw-text extraction alike — exactly as a
/// Cf-only token has been since round 27. Other
/// boundary tokens (a standalone "•", "~", an
/// emoji, a dash) are NOT nothings — they keep
/// their standing boundary readings wherever those
/// stand, and a standalone ";" keeps its
/// segment-boundary shape inside a date span (see
/// `parse_date_prefix`).
///
/// The class is stated once, in ob-model, and this
/// function delegates to that statement (round 33,
/// R33-D, F5): `parse_flexible_date` tokenizes by
/// ob-model's `nothing_token`, so every reader here
/// and the flexible reader there judge the same
/// fragments by the same function, by construction.
/// Until this round this crate kept its own copy,
/// identical but coupled by nothing; the weekday
/// fallback's suffix skip in `date_line_inner`
/// (composition.rs) counts pieces by this predicate
/// and is sound only while the count agrees with
/// the reader's own tokenization — the premise that
/// site now names. The classification of the
/// fragment family both readers meet is pinned in
/// `r33d_nothing_token_is_the_flexible_readers_predicate`.
pub(super) fn nothing_token(raw: &str) -> bool {
    ob_model::nothing_token(raw)
}

/// Whether a token is nothing but R40-G sequence-tail
/// suffix marks ("~", "*", "#", "-", "—" U+2014): a
/// spaced suffix standing after a judged trailing word
/// or group is that word's dress, not line content
/// (round 41, R41-E, F6 — the spaced form " ~" disarmed
/// the trailing judgments as the glued form did). A
/// standalone "~" is NOT a nothing token (see
/// `nothing_token`) and keeps its standing boundary
/// readings everywhere else; this predicate is read
/// only by the trailing-slot arms that judge the word
/// the suffix follows.
/// Round 42 (R42-F, F9 = red team RT42-S4): the set
/// gains the en dash (U+2013), the set's own hyphen's
/// codepoint sibling, judged wherever "-" and "—"
/// are judged — a spaced en dash standing after a
/// trailing garnish is that garnish's dress here, as
/// the two judged dashes are.
/// Round 43 (R43-B, F2 + F3): the set gains the
/// sibling class at the set level — the dash
/// siblings (U+2010, U+2011, U+2212, U+FF0D), the
/// fullwidth asterisk (U+FF0A), and U+00AD — so a
/// spaced sibling token standing after a trailing
/// garnish is that garnish's dress here, as its
/// twin's token is.
pub(super) fn suffix_tail_token(raw: &str) -> bool {
    !raw.is_empty()
        && raw.chars().all(|c| {
            matches!(
                c,
                '~' | '*'
                    | '#'
                    | '-'
                    | '\u{2014}'
                    | '\u{2013}'
                    | '\u{2010}'
                    | '\u{2011}'
                    | '\u{2212}'
                    | '\u{ff0d}'
                    | '\u{ff0a}'
                    | '\u{ff5e}'
                    | '\u{ff03}'
                    | '\u{2dc}'
                    | '\u{00ad}'
            )
        })
}

/// One group token with its wrapping punctuation off,
/// lowercased: the shape the governor and weekday
/// checks read.
pub(super) fn group_token(raw: &str) -> String {
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
    // Round 35 (R35-B, F2): the bracketed rule has
    // one exception — a governing word governs
    // however it is wrapped. A wholly bracketed or
    // braced token whose word the governor checks
    // themselves recognize (a single governor, by
    // `is_single_governor` — the cancellation stems
    // included — or a one-word cancellation phrase,
    // by `is_cancellation_phrase`) cleans to that
    // word, exactly as its paren twin cleans through
    // the edge trim in `clean_group_word`:
    // "[cancelled]" and "{postponed}" are words
    // standing beside a
    // date, not garnish. Garnish words ("[choir]")
    // keep the round-24 rule untouched.
    if is_bracketed_token(raw) {
        // Round 36 (R36-E, F5): the same trailing
        // trim `is_bracketed_token` judges by, so the
        // inner word of "[cancelled]!" is "cancelled"
        // itself, not "cancelled]".
        // Round 37 (R37-E, F4): the trim gains the
        // typographic closing quotes with it, so the
        // inner word of "[cancelled]”" is "cancelled"
        // itself too.
        let trimmed = raw.trim_end_matches([
            ',', '.', ':', ';', '!', '?', '\'', '"', '\u{201d}', '\u{2019}',
        ]);
        let inner = &trimmed[1..trimmed.len() - 1];
        let word = clean_group_word(inner);
        if is_single_governor(&word) || is_cancellation_phrase(std::slice::from_ref(&word)) {
            return word;
        }
        return String::new();
    }
    if token_has_no_letters_or_digits(raw) {
        return String::new();
    }
    clean_group_word(raw)
}

/// One token's word with its wrapping punctuation
/// off, lowercased: format characters stripped
/// (round 25, R25-A, M6 — one glued inside a word,
/// "cancel\u{200b}led", cannot kill the word), then
/// the edge trim over every wrapping pair and
/// boundary mark (round 23, R23-3). Round 36
/// (R36-E, F5): the edge trim also takes the
/// sentence marks `! ? ' "` — a word wearing one
/// ("cancelled!", "off]!") cleans to the word, as
/// the bare line's own reading of "cancelled!"
/// already governed; an apostrophe INSIDE a word
/// ("don't") is not an edge and is untouched.
fn clean_group_word(text: &str) -> String {
    let stripped = strip_format_chars(text);
    // Round 37 (R37-E, F2/F3/F4): the edge trim
    // learns the curly quote pairs — U+201C/U+201D
    // and U+2018/U+2019 stand at a word's edges
    // exactly as the straight quotes do, so a
    // curly-quoted word ("“cancelled”",
    // "‘Sunday’") cleans to the word itself, at
    // parity with its straight-quoted twin.
    // Round 44 (R44-B, F2 MAJOR = red team
    // RT44-C1): the edge trim gains the fullwidth
    // separators (U+FF0C, U+FF1B, U+FF1A, U+FF0E,
    // U+3001, U+3002) — a word wearing one at its
    // edge is the word it names, as one wearing
    // "," / ";" / ":" / "." already is (the shared
    // word cleaning, as in `clean`) — and the
    // fullwidth quotes (U+FF02, U+FF07), which a
    // word wearing at its edges sheds as it sheds
    // the straight and curly pairs.
    stripped
        .trim_matches([
            '(', ')', '[', ']', '{', '}', ',', '.', ':', ';', '-', '\u{2013}', '\u{2014}', '/',
            '!', '?', '\'', '"', '\u{2018}', '\u{2019}', '\u{201c}', '\u{201d}', '\u{ff0c}',
            '\u{ff1b}', '\u{ff1a}', '\u{ff0e}', '\u{3001}', '\u{3002}', '\u{ff02}', '\u{ff07}',
        ])
        .to_ascii_lowercase()
}

/// The cleaned tokens of one segment-bearing token
/// run — a group's words, a plain core, a core
/// segment — as every segment reader reads them
/// (round 27, R27-C, F4 = red team RT27-C1; round
/// 28, R28-A, F1 = review MAJOR-3 + red team
/// RT28-C1): the shared cleaning (`group_token`),
/// plus the ungluing rule. `*` and `|` are this
/// format's own structural separators (a shipped
/// core is "November 1, 2026 * cancelled"), so a
/// separator glued to a word IS the separator plus
/// that word. Round 27 read that rule only at a
/// phrase's HEAD: a separator glued to a phrase's
/// second or third word — or to every word —
/// hid the phrase exactly as the pre-fix head
/// gluing had ("hold *off", "*used *to *be"), and
/// a glued "off", preposition, or auxiliary in a
/// walk line was invisible to the walk. The rule
/// is a property of the TOKEN, not of the phrase
/// head:
///
/// - Governor phrases are judged on the UNGLUED
///   token vector, at every position of the
///   phrase: when a phrase of one to three words
///   (read over the run's words that are
///   something, as the phrase matchers read) is a
///   governor phrase, every word of it is read
///   unglued, the head and the continuations
///   alike.
/// - A glued word the segment machinery keys on
///   structurally — an auxiliary, a preposition
///   of the continuation family, the target
///   preposition "to", or "off" (the preposition
///   the bare test's own inventory carries) — is
///   read unglued in every position, so the
///   spaced form's walk, claim, and bare readings
///   are reachable by its glued form in the same
///   positions.
///
/// Every other token (a glued date word, a glued
/// content word — "*November", "*choir") keeps its
/// standing cleaned form byte for byte: ungluing
/// makes structural words visible, it does not
/// mint content. The separators are NOT format
/// characters: the Cf machinery (`cf_only_token`,
/// `strip_format_chars`) is untouched, a standalone
/// "*" / "|" token still cleans to nothing exactly
/// as it always has, and `group_token` itself is
/// unchanged — this rule lives here, where segment
/// tokens are read, not in the token cleaning.
/// Ungluing only makes the word visible; every
/// contextual judgment on it (the auxiliary-slot
/// and subject-PP narrowings, the determiner-headed
/// -s rule, the for-zones) still runs exactly as
/// it runs on the spaced twin.
/// One cleaned token with its glued separators
/// taken off (round 29, R29-A, F8): the unglued
/// form `segment_tokens` judges phrases on — a
/// separator glued to a word is the separator plus
/// that word, so "*sunday" unglued is "sunday".
pub(super) fn unglued_word(w: &str) -> String {
    w.trim_start_matches(['*', '|'])
        .trim_end_matches(['*', '|'])
        .to_string()
}
