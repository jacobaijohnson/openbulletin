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
use ob_model::{
    default_styles, parse_flexible_date, Paragraph, SimpleDate, MONTH_NAMES, WEEKDAY_NAMES,
};

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

mod composition;
mod dates;
mod grammar;
mod segments;

use composition::*;
use dates::*;
pub use dates::{
    keep_through_second_date_sentence, keep_through_second_dates_sentence, second_date_sentence,
    second_dates_sentence, ParenthesizedDateMismatch, WeekdayMismatch,
};

#[cfg(test)]
mod tests;

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
    // Round 41 (R41-D, F5): the parenthesized
    // memory below is read on the same re-dressed
    // text the date readers consult where the
    // re-dress consultation fires (see
    // `quoted_groups_redressed_where_judged` in
    // composition.rs) — a quoted date group
    // standing alone beside the phrase is
    // remembered as a group in its own right,
    // carrying the date its bracket twins' groups
    // carry, where as written it was no group to
    // these readers and nothing was remembered at
    // all.
    let for_sunday_memory_text: Option<String> = raw
        .for_sunday
        .as_deref()
        .and_then(|t| quoted_groups_redressed_where_judged(t, today, anchor));
    // A trailing parenthesized absolute date on a
    // relative phrase is remembered with the phrase
    // (round 8, carried shape): the verdict compares
    // it with the relative resolution once a bulletin
    // anchors it. A parenthesis that does not read as
    // a date is garnish and leaves no memory.
    let for_sunday_parenthesized_date =
        if for_sunday_in_words.is_some() || for_sunday_after_in_words.is_some() {
            raw.for_sunday.as_deref().and_then(|t| {
                all_parenthesized_date(
                    for_sunday_memory_text.as_deref().unwrap_or(t),
                    today,
                    anchor,
                )
            })
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
                .map(|t| {
                    all_parenthesized_dates(
                        for_sunday_memory_text.as_deref().unwrap_or(t),
                        today,
                        anchor,
                    )
                })
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

/// "October 25, 2026": the date without its weekday, for
/// sentences that already say the day ("the bulletin for
/// Sunday, ..."). The month and weekday names are
/// ob-model's one table ([`MONTH_NAMES`] /
/// [`WEEKDAY_NAMES`], health audit E-7), shared with
/// the chrono date type's formatters.
pub fn format_date_short(d: SimpleDate) -> String {
    let month = MONTH_NAMES
        .get((d.month as usize).wrapping_sub(1))
        .copied()
        .unwrap_or("");
    format!("{month} {}, {}", d.day, d.year)
}

/// "Sunday, October 25, 2026": the date in full, the way
/// the steps speak of a service day.
pub fn format_date_long(d: SimpleDate) -> String {
    let weekday = WEEKDAY_NAMES[(d.weekday() % 7) as usize];
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
