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
    let w = cleaned
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
        named: named_word.trim_end_matches([',', '.']).to_string(),
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
        let word = tokens[i].trim_matches([
            '(', ')', '[', ']', '{', '}', ',', '.', ';', ':', '!', '?', '"', '\'', '\u{201d}',
            '\u{201c}', '\u{2018}', '\u{2019}',
        ]);
        if let Some(named_number) = weekday_word(word) {
            let named = word.to_string();
            trailing = Some((named, named_number));
            if i + 1 == tokens.len() {
                trailing_token = Some(tokens[i]);
            }
            core = tokens
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
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
        if let Some(m) = base_trailing_sequence_mismatch(&core, used, trailing_token, base) {
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
    if let Some(m) = base_trailing_sequence_mismatch(&core, consumed, trailing_token, base) {
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
/// every weekday group in it is judged on its
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
/// untouched by construction.
fn base_trailing_sequence_mismatch(
    core: &[&str],
    consumed: usize,
    trailing_token: Option<&str>,
    base: SimpleDate,
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
    let mut rest = garnish.as_str();
    loop {
        // Round 38 (R38-B, F2): the peel set gains
        // U+2019, as at the plain trailing walk —
        // the lone base garnish wearing it ("Sunday
        // after November 1, 2026 (Monday)’") is
        // judged, and its pair U+201D was already
        // here.
        let peeled = rest.trim_end().trim_end_matches([
            '.', ',', ';', ':', '!', '?', '"', '\'', '\u{201d}', '\u{2019}',
        ]);
        if peeled.len() != rest.len() {
            rest = peeled;
            continue;
        }
        match rest.rsplit_once(char::is_whitespace) {
            Some((head, token))
                if !token.is_empty() && token.chars().all(|c| c == '*' || c == '|') =>
            {
                rest = head;
            }
            _ => break,
        }
    }
    let rest = rest.trim_end();
    let spans = balanced_spans(rest);
    if spans.len() < 2 {
        return None;
    }
    if spans[0].0 != 0 || spans[spans.len() - 1].1 != rest.len() {
        return None;
    }
    for pair in spans.windows(2) {
        let gap = &rest[pair[0].1..pair[1].0];
        // Round 38 (R38-B, F2): a gap may carry the
        // sentence separators ";" "." ":" and a
        // U+2019 standing as the preceding group's
        // suffix, as at the plain slots' walks.
        if !gap
            .chars()
            .all(|c| matches!(c, ',' | ';' | '.' | ':' | '\u{2019}') || c.is_whitespace())
        {
            return None;
        }
    }
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

/// The last trailing group of `text`, split as (the
/// words before it, the group's own words). The
/// group's opening delimiter is found by balancing,
/// so a nested group ("(note (November 1))") is one
/// trailing group, not a defect that poisons the
/// line it trails (round 11, R11-6). Round 34
/// (R34-E, F3): brackets and braces are group
/// delimiters alongside parentheses — a bracketed
/// group is judged as its paren twin (the
/// adjudicated reading) — so the balance walk is a
/// stack over all three pairs: a closer closes the
/// opener it matches, and a closer matching nothing
/// on the stack closes nothing. On parenthesis-only
/// text the stack is the depth count it replaces,
/// step for step.
pub(super) fn split_last_trailing_group(text: &str) -> Option<(&str, &str)> {
    let text = text.trim_end_matches(['.', ',']);
    // Round 28 (R28-A, F2 = review MINOR-2): a
    // nothing-token trailing the group in the raw
    // text ("(moved … to November 8) \u{200b}", "…
    // ) *") is nothing after it — the group still
    // ends the line, so trailing whitespace-
    // separated nothings come off before the
    // closing-paren test. The slices returned are
    // slices of the text with those nothings (and
    // the punctuation above) removed.
    let mut text = text;
    loop {
        let trimmed = text.trim_end();
        match trimmed.rsplit_once(char::is_whitespace) {
            Some((head, last)) if nothing_token(last) => {
                text = head.trim_end_matches(['.', ',']);
            }
            _ => {
                text = trimmed;
                break;
            }
        }
    }
    if !text.ends_with([')', ']', '}']) {
        return None;
    }
    let bytes = text.as_bytes();
    let mut stack: Vec<u8> = Vec::new();
    let mut open = None;
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
                        open = Some(i);
                        break;
                    }
                }
            }
            _ => {}
        }
    }
    let open = open?;
    Some((text[..open].trim_end(), &text[open + 1..text.len() - 1]))
}

/// `text` without a trailing group, when it ends
/// with one: the words before the group. (The name
/// is the standing one; since round 34 the group
/// may wear brackets or braces as well as
/// parentheses — see `split_last_trailing_group`.)
pub(super) fn strip_trailing_parenthesized(text: &str) -> Option<&str> {
    split_last_trailing_group(text).map(|(before, _)| before)
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
    const BOUNDARY_TRIM: [char; 10] = [
        ' ', '\t', ';', ',', ':', '.', '/', '-', '\u{2013}', '\u{2014}',
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
        let lo = rest.as_ptr() as usize - text.as_ptr() as usize;
        if relative_shape_words(text, &toks, &ne, lo, lo + rest.len(), shape) {
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
    lo: usize,
    hi: usize,
    shape: fn(&[String]) -> bool,
) -> bool {
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
    let head: Option<String> = head_cut.then(|| group_token(&text[lo..toks[t_first].1.min(hi)]));
    let tail: Option<String> = tail_cut.then(|| group_token(&text[toks[t_last].0..hi]));
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
pub(super) fn strip_trailing_boundary_tokens(text: &str) -> &str {
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
    stripped
        .trim_matches([
            '(', ')', '[', ']', '{', '}', ',', '.', ':', ';', '-', '\u{2013}', '\u{2014}', '/',
            '!', '?', '\'', '"', '\u{2018}', '\u{2019}', '\u{201c}', '\u{201d}',
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
