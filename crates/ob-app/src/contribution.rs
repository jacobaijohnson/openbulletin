//! Contribution planning and application, app side:
//! slotting a sender's parts into the bulletin,
//! judging headings and targets, the already-applied
//! record, and the Inbox scan of the contributions
//! folder.

use crate::check::*;
use crate::*;

/// Where one part of a contribution will go, decided
/// before Accept so the review can say it in words.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum SlotTarget {
    /// Append the part's paragraphs to this story.
    Story(String),
    /// Append to the announcements story under a heading
    /// paragraph with these words (a Sunday school or youth
    /// part with no box of its own in this bulletin).
    AnnouncementsUnderHeading(&'static str),
    /// A loose sheet: not applied. It is filed with the
    /// contribution and kept safe (inserts get their own
    /// build later; the words are never dropped and never
    /// fake-applied).
    LooseSheet,
    /// Nowhere to land: the bulletin has no box for these
    /// words yet (round 3, R3-8a). They stay with the part
    /// when it is filed, and the plan says exactly that
    /// instead of promising a box that is not there.
    KeptWithPart,
    /// The part's words are already in the bulletin
    /// (round 5, R5-B): the presence check found every
    /// one of its paragraphs in the story the part would
    /// land in (round 6, R6-B), so applying it again
    /// would print the words twice. The apply step
    /// leaves the document untouched and counts the part
    /// as landed: it is where it belongs.
    AlreadyThere,
}

/// One part's slot, with the sentence the review shows.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PartPlan {
    pub(crate) target: SlotTarget,
    pub(crate) line: String,
}

/// A story named for a set of words ("sunday school",
/// "youth"): the story's own id, or the name of a box
/// that shows it, compared without case and with dashes
/// and underscores read as spaces.
pub(crate) fn named_story_id(doc: &Document, words: &str) -> Option<String> {
    let norm = |s: &str| {
        s.chars()
            .map(|c| match c {
                '-' | '_' => ' ',
                c => c,
            })
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_ascii_lowercase()
    };
    let want = norm(words);
    if let Some(s) = doc.stories.iter().find(|s| norm(&s.id) == want) {
        return Some(s.id.clone());
    }
    doc.panels
        .iter()
        .flat_map(|p| p.frames.iter())
        .find(|f| norm(&f.name) == want)
        .and_then(|f| f.story_id.clone())
}

/// Where one line of a service part lands (R-2a, the
/// music slip): hymn lines slot into the bulletin's hymn
/// lines instead of piling up at the end of the service.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum HymnSlot {
    /// Set the existing service line at this paragraph
    /// index to the composed value: a named line replaced,
    /// or an empty line filled.
    Replace {
        para_index: usize,
        label: String,
        was_empty: bool,
        value: String,
    },
    /// A new service line at the end, labelled "Hymn".
    AppendHymn { value: String },
    /// The paragraph goes in as written, at the end.
    Plain,
}

/// A contribution line's own label and value, the way
/// senders write them: "Hymn of Praise\t433" (the tab
/// mirrors the bulletin's own lines) or "Opening hymn:
/// ELW 432". Anything else is all value, no label. The
/// value is everything after the FIRST tab (round 3,
/// R3-8e): a line typed as "Hymn\tELW 432\tJoyful,
/// Joyful" keeps its middle, so the hymnal and number
/// survive to be composed with the title.
pub(crate) fn part_line_label(text: &str) -> (Option<String>, String) {
    let t = text.trim();
    if let Some(tab) = t.find('\t') {
        let label = t[..tab].trim();
        let value = t[tab + 1..].trim();
        return (
            if label.is_empty() {
                None
            } else {
                Some(label.to_string())
            },
            value.to_string(),
        );
    }
    if let Some((head, tail)) = t.split_once(':') {
        let head = head.trim();
        if !head.is_empty()
            && head.split_whitespace().count() <= 6
            && !head.chars().any(|c| c.is_ascii_digit())
        {
            return (Some(head.to_string()), tail.trim().to_string());
        }
    }
    (None, t.to_string())
}

/// Decide where every line of a service part lands
/// against this document's service lines. A line is a
/// hymn entry only when it parses to a hymn value WITH
/// a number, and either its label is a hymn label or
/// the value itself names a hymnal (round 3, R3-8e):
/// a bare number can be a psalm, a page, or a time
/// ("Psalm\t23" stays a plain service line), and prose
/// under a hymn-flavoured label ("Hymn requests: see
/// the director") has no number and stays prose.
/// Named entries replace the hymn line they name; the
/// rest fill empty hymn lines in order; what is left
/// becomes a new line at the end. Pure, so the review's
/// plan and the apply step can both run it and always
/// agree.
pub(crate) fn hymn_slots(doc: &Document, part: &ob_extract::contribution::Part) -> Vec<HymnSlot> {
    use ob_model::hymns::{compose_hymn_value, is_hymn_label, parse_hymn_value};
    let lines = service_lines(doc);
    let mut used: Vec<usize> = Vec::new();
    let mut parsed: Vec<(Option<String>, ob_model::hymns::HymnEntry, bool)> = Vec::new();
    for p in &part.paragraphs {
        let text = p.text();
        let (label, value) = part_line_label(&text);
        let entry = parse_hymn_value(&value);
        let hymn_labelled = label.as_deref().map(is_hymn_label).unwrap_or(false);
        let valued = !text.trim().is_empty()
            && entry.number.is_some()
            && (hymn_labelled || entry.hymnal.is_some());
        parsed.push((label, entry, valued));
    }
    let mut slots: Vec<Option<HymnSlot>> = Vec::new();
    slots.resize_with(part.paragraphs.len(), || None);
    // Named first: a label that names an existing hymn
    // line replaces that line's value.
    for (i, (label, entry, valued)) in parsed.iter().enumerate() {
        if !valued {
            continue;
        }
        let Some(label) = label else { continue };
        let Some(line) = lines.iter().find(|l| {
            is_hymn_label(&l.label)
                && l.label.eq_ignore_ascii_case(label)
                && !used.contains(&l.para_index)
        }) else {
            continue;
        };
        used.push(line.para_index);
        slots[i] = Some(HymnSlot::Replace {
            para_index: line.para_index,
            label: line.label.clone(),
            was_empty: line.value.trim().is_empty(),
            value: compose_hymn_value(entry),
        });
    }
    // Then in order: what is left fills the empty hymn
    // lines.
    for (i, (_, entry, valued)) in parsed.iter().enumerate() {
        if !valued || slots[i].is_some() {
            continue;
        }
        let Some(line) = lines.iter().find(|l| {
            is_hymn_label(&l.label) && l.value.trim().is_empty() && !used.contains(&l.para_index)
        }) else {
            continue;
        };
        used.push(line.para_index);
        slots[i] = Some(HymnSlot::Replace {
            para_index: line.para_index,
            label: line.label.clone(),
            was_empty: true,
            value: compose_hymn_value(entry),
        });
    }
    // What is still standing: a hymn entry becomes a new
    // line; anything else goes in as written.
    for (i, (_, entry, valued)) in parsed.iter().enumerate() {
        if slots[i].is_none() {
            slots[i] = Some(if *valued {
                HymnSlot::AppendHymn {
                    value: compose_hymn_value(entry),
                }
            } else {
                HymnSlot::Plain
            });
        }
    }
    slots
        .into_iter()
        .map(|s| s.expect("every paragraph slotted"))
        .collect()
}

/// The review's words for a service part (R-2a): when the
/// part carries hymn entries, the plan names the hymn
/// lines they fill or replace; when it carries none, the
/// R-1 words stand.
pub(crate) fn service_plan_line(doc: &Document, part: &ob_extract::contribution::Part) -> String {
    let slots = hymn_slots(doc, part);
    if !slots
        .iter()
        .any(|s| matches!(s, HymnSlot::Replace { .. } | HymnSlot::AppendHymn { .. }))
    {
        return "Goes into the service lines, added at the end.".to_string();
    }
    let mut sentences: Vec<String> = Vec::new();
    let mut plain = 0usize;
    for slot in &slots {
        match slot {
            HymnSlot::Replace {
                label,
                was_empty,
                value,
                ..
            } => {
                if *was_empty {
                    sentences.push(format!("Fills the {label} line with {value}."));
                } else {
                    sentences.push(format!("The {label} line becomes {value}."));
                }
            }
            HymnSlot::AppendHymn { value } => {
                sentences.push(format!("Adds a hymn line at the end: {value}."));
            }
            HymnSlot::Plain => plain += 1,
        }
    }
    if plain == 1 {
        sentences.push("1 more line is added at the end.".to_string());
    } else if plain > 1 {
        sentences.push(format!("{plain} more lines are added at the end."));
    }
    sentences.join(" ")
}

/// The style the service lines wear, for a line the app
/// adds itself: the first tabbed line's style in the
/// service story, or body when there is none.
pub(crate) fn service_line_style(doc: &Document) -> String {
    let Some(sid) = service_story_id(doc) else {
        return "body".to_string();
    };
    let Some(story) = doc.story(&sid) else {
        return "body".to_string();
    };
    story
        .paragraphs
        .iter()
        .find(|p| p.text().contains('\t'))
        .map(|p| p.style.clone())
        .unwrap_or_else(|| "body".to_string())
}

/// Push one paragraph onto a story. Answers false when
/// the story is not there.
pub(crate) fn push_story_paragraph(doc: &mut Document, sid: &str, p: ob_model::Paragraph) -> bool {
    match doc.story_mut(sid) {
        Some(story) => {
            story.paragraphs.push(p);
            true
        }
        None => false,
    }
}

/// The date a story keeps through once a contribution's
/// date lands in it: the later of the story's own date
/// and the contribution's (round 3, R3-8c). An existing
/// date that does not read is simply replaced; the new
/// date can extend a stay, never silently shorten one.
pub(crate) fn effective_keep_through(
    doc: &Document,
    sid: &str,
    kt: ob_model::SimpleDate,
) -> ob_model::SimpleDate {
    if let Some(existing) = doc
        .story(sid)
        .and_then(|s| s.keep_through.as_deref())
        .and_then(|iso| ob_model::parse_flexible_date(iso, kt))
    {
        if existing > kt {
            return existing;
        }
    }
    kt
}

/// The keep-through sentence for a part landing in the
/// story `sid`, worded with the date that will actually
/// be in force there. Nothing when the contribution
/// names no date: then no date is in play.
pub(crate) fn stays_in_sentence(
    doc: &Document,
    sid: &str,
    c: &ob_extract::contribution::Contribution,
) -> String {
    let Some(kt) = c.keep_through else {
        return String::new();
    };
    format!(
        " Stays in through {}.",
        ob_extract::contribution::format_date_long(effective_keep_through(doc, sid, kt))
    )
}

/// Land a contribution's keep-through date in the story
/// that received its words (round 3, R3-8b): every arm
/// that places words, not only the announcement arm.
/// The later of the two dates wins (R3-8c).
pub(crate) fn apply_keep_through(doc: &mut Document, sid: &str, kt: ob_model::SimpleDate) {
    let effective = effective_keep_through(doc, sid, kt);
    if let Some(story) = doc.story_mut(sid) {
        story.keep_through = Some(format!(
            "{:04}-{:02}-{:02}",
            effective.year, effective.month, effective.day
        ));
    }
}

/// Decide one part's slot against this document.
pub(crate) fn plan_part(
    doc: &Document,
    c: &ob_extract::contribution::Contribution,
    part: &ob_extract::contribution::Part,
) -> PartPlan {
    let judged = judged_headings(doc, c);
    plan_part_judged(doc, c, part, &judged)
}

/// `plan_part` with the judging context given (round 8,
/// R8-7).
pub(crate) fn plan_part_judged(
    doc: &Document,
    c: &ob_extract::contribution::Contribution,
    part: &ob_extract::contribution::Part,
    judged: &[JudgedHeading],
) -> PartPlan {
    // Round 5 (R5-B): a part whose words are already in
    // the bulletin is planned as already there, whatever
    // box it would otherwise have gone into, so the
    // review's sentence and the apply step agree that
    // nothing is added. Round 6 (R6-B): presence is
    // judged against the part's own planned target (see
    // `part_words_present`), never against the whole
    // document, so the plan target is decided first and
    // the presence judgment follows it.
    let plan = planned_part_plan(doc, c, part);
    if part_words_present_judged(doc, c, part, judged) {
        let line =
            "These words are already in the bulletin, so they are not added again.".to_string();
        return PartPlan {
            target: SlotTarget::AlreadyThere,
            line,
        };
    }
    plan
}

/// The slot a part would land in, decided without the
/// presence judgment: `plan_part`'s match, shared with
/// the presence machinery, which must know a part's
/// target before it can say whether the part's words
/// are already in the story that target names.
pub(crate) fn planned_part_plan(
    doc: &Document,
    c: &ob_extract::contribution::Contribution,
    part: &ob_extract::contribution::Part,
) -> PartPlan {
    use ob_extract::contribution::PartKind;
    let service = service_story_id(doc);
    let weekly = weekly_story_id(doc);
    match part.kind {
        PartKind::Service => match &service {
            Some(sid) => PartPlan {
                target: SlotTarget::Story(sid.clone()),
                line: format!(
                    "{}{}",
                    service_plan_line(doc, part),
                    stays_in_sentence(doc, sid, c)
                ),
            },
            None => {
                let mut line = "This bulletin has no service box, so these words go into \
                       the announcements, under a \u{201c}For the service\u{201d} heading."
                    .to_string();
                if let Some(wid) = &weekly {
                    line.push_str(&stays_in_sentence(doc, wid, c));
                }
                PartPlan {
                    target: SlotTarget::AnnouncementsUnderHeading("For the service"),
                    line,
                }
            }
        },
        PartKind::Announcement => match &weekly {
            Some(wid) => PartPlan {
                target: SlotTarget::Story(wid.clone()),
                line: format!(
                    "Goes into the announcements.{}",
                    stays_in_sentence(doc, wid, c)
                ),
            },
            // Round 3 (R3-8a): with no announcements box,
            // the plan says the words stay with the part.
            // Promising a box that is not there sent the
            // office looking for words Accept never placed.
            None => PartPlan {
                target: SlotTarget::KeptWithPart,
                line: "There is no announcements box in this bulletin yet, so these \
                       words are kept with the part when it is filed; nothing is \
                       added to the pages."
                    .to_string(),
            },
        },
        PartKind::SundaySchool | PartKind::Youth => {
            let words = if part.kind == PartKind::SundaySchool {
                "sunday school"
            } else {
                "youth"
            };
            match named_story_id(doc, words) {
                Some(sid) => PartPlan {
                    line: format!(
                        "Goes into the {words} box.{}",
                        stays_in_sentence(doc, &sid, c)
                    ),
                    target: SlotTarget::Story(sid),
                },
                None => {
                    let mut line = format!(
                        "Goes into the announcements, under a \u{201c}{}\u{201d} heading.",
                        part.kind.heading()
                    );
                    if let Some(wid) = &weekly {
                        line.push_str(&stays_in_sentence(doc, wid, c));
                    }
                    PartPlan {
                        target: SlotTarget::AnnouncementsUnderHeading(part.kind.heading()),
                        line,
                    }
                }
            }
        }
        PartKind::Insert => PartPlan {
            target: SlotTarget::LooseSheet,
            line: "A loose sheet. It is filed with this part and kept safe; \
                   it does not go into the bulletin pages."
                .to_string(),
        },
    }
}

/// Decide every part's slot, in file order. The plan is
/// a simulation (round 3, R3-8d): each part is planned
/// against a clone the earlier parts have already been
/// applied to, with the very function Accept will run,
/// so the sentences the review shows are the outcomes
/// Accept produces, including a second service part
/// slotting its hymns against the first part's work.
pub(crate) fn plan_parts(
    doc: &Document,
    c: &ob_extract::contribution::Contribution,
) -> Vec<PartPlan> {
    let mut sim = doc.clone();
    let mut plans = Vec::with_capacity(c.parts.len());
    // Round 8 (R8-7): the simulation applies one part at
    // a time, but the judging context stays the whole
    // contribution's — a bold paragraph carrying another
    // part's heading delimits sections in the simulation
    // exactly as it does in the real apply.
    let judged = judged_headings(doc, c);
    for part in &c.parts {
        plans.push(plan_part(&sim, c, part));
        let single = ob_extract::contribution::Contribution {
            from: c.from.clone(),
            for_date: c.for_date,
            keep_through: c.keep_through,
            parts: vec![part.clone()],
            named_weekday_mismatch: c.named_weekday_mismatch.clone(),
            keep_through_weekday_mismatch: c.keep_through_weekday_mismatch.clone(),
            for_sunday_in_words: c.for_sunday_in_words.clone(),
            for_sunday_after_in_words: c.for_sunday_after_in_words.clone(),
            for_sunday_parenthesized_date: c.for_sunday_parenthesized_date,
            for_sunday_parenthesized_dates: c.for_sunday_parenthesized_dates.clone(),
            for_sunday_parenthesized_mismatch: c.for_sunday_parenthesized_mismatch.clone(),
            keep_through_parenthesized_mismatch: c.keep_through_parenthesized_mismatch.clone(),
            for_sunday_second_date: c.for_sunday_second_date,
            keep_through_second_date: c.keep_through_second_date,
            for_sunday_second_dates: c.for_sunday_second_dates.clone(),
            keep_through_second_dates: c.keep_through_second_dates.clone(),
        };
        apply_contribution_judged(&mut sim, &single, &judged);
    }
    plans
}

/// Whether a style is the one named `wanted`, by its id
/// or by its name (round 8, R8-8b): the one recognition
/// the heading writer and the heading recognizer share,
/// mirroring the markdown import's name-or-id lookup.
pub(crate) fn style_is_named(style: &ob_model::ParagraphStyle, wanted: &str) -> bool {
    style.id == wanted || style.name.eq_ignore_ascii_case(wanted)
}

/// Whether a paragraph style id wears a heading style
/// (round 8, R8-8b): the literal "heading" / "subheading"
/// ids, or a style in the document's own table whose id
/// the paragraph carries and whose name or id is
/// "heading" or "subheading" — so a subheading style
/// with a non-literal id is recognized wherever it
/// stands.
pub(crate) fn is_heading_style(doc: &Document, style_id: &str) -> bool {
    if style_id == "heading" || style_id == "subheading" {
        return true;
    }
    doc.styles.iter().any(|s| {
        s.id == style_id && (style_is_named(s, "heading") || style_is_named(s, "subheading"))
    })
}

/// A heading paragraph in the document's own styles: the
/// subheading style when the document has one, else body
/// words made bold (the markdown parser's own fallback).
pub(crate) fn heading_paragraph(doc: &Document, text: &str) -> ob_model::Paragraph {
    if let Some(s) = doc.styles.iter().find(|s| style_is_named(s, "subheading")) {
        return ob_model::Paragraph::plain(&s.id, text);
    }
    let mut p = ob_model::Paragraph::plain("body", text);
    for r in &mut p.runs {
        if r.bold.is_none() {
            r.bold = Some(true);
        }
    }
    p
}

/// The running counts one apply builds toward its
/// summary sentences.
#[derive(Default)]
pub(crate) struct ApplyCounts {
    pub(crate) service_lines: usize,
    pub(crate) announcements: usize,
    pub(crate) named: usize,
    pub(crate) under_headings: usize,
    pub(crate) loose: usize,
    pub(crate) unplaced: usize,
    pub(crate) already: usize,
}

/// Split a part's paragraphs against one target story
/// (round 6, R6-B): the paragraphs still missing from
/// that story, in the part's own order, and how many
/// were already there. Presence spends the story's
/// paragraphs one by one, so a paragraph the part
/// carries twice needs the story to carry it twice
/// before both are skipped. An empty paragraph has no
/// words that could be missing; it lands as before.
pub(crate) fn split_present_paragraphs(
    doc: &Document,
    sid: &str,
    part: &ob_extract::contribution::Part,
) -> (Vec<ob_model::Paragraph>, usize) {
    let mut spent: Vec<usize> = Vec::new();
    let mut to_land: Vec<ob_model::Paragraph> = Vec::new();
    let mut skipped = 0usize;
    for p in &part.paragraphs {
        if normalized_words(&p.text()).is_empty() {
            to_land.push(p.clone());
            continue;
        }
        match story_paragraph_index(doc, sid, &p.text(), &spent) {
            Some(i) => {
                spent.push(i);
                skipped += 1;
            }
            None => to_land.push(p.clone()),
        }
    }
    (to_land, skipped)
}

/// Split an under-heading part's paragraphs against its
/// own heading's sections of the weekly story (round 7,
/// R7-B; round 8, R8-2): the paragraphs still missing
/// FROM THOSE SECTIONS, and the section they land in —
/// the first. Words standing elsewhere in the story do
/// not skip a paragraph here — the part's landing is
/// its placement under its heading, so a paragraph that
/// stands only elsewhere lands again, under the
/// heading. When the heading stands nowhere in the
/// story there is no section: every paragraph is to
/// land, under a heading the apply adds.
pub(crate) fn split_present_paragraphs_under_heading(
    doc: &Document,
    wid: &str,
    heading: &str,
    part: &ob_extract::contribution::Part,
    judged: &[JudgedHeading],
) -> (Vec<ob_model::Paragraph>, usize, Option<(usize, usize)>) {
    let sections = heading_sections(doc, wid, heading, judged);
    let Some(section) = sections.first().copied() else {
        let all = part.paragraphs.clone();
        return (all, 0, None);
    };
    // Round 8 (R8-2): one section judgment with the
    // checker. A paragraph standing under the part's
    // heading in ANY of its sections is already there
    // and is not landed again; the paragraphs still
    // missing land in the FIRST section, which is the
    // section this returns.
    let mut spent: Vec<usize> = Vec::new();
    let mut to_land: Vec<ob_model::Paragraph> = Vec::new();
    let mut skipped = 0usize;
    for p in &part.paragraphs {
        if normalized_words(&p.text()).is_empty() {
            to_land.push(p.clone());
            continue;
        }
        let found = sections
            .iter()
            .find_map(|range| story_paragraph_index_in_range(doc, wid, &p.text(), &spent, *range));
        match found {
            Some(i) => {
                spent.push(i);
                skipped += 1;
            }
            None => to_land.push(p.clone()),
        }
    }
    (to_land, skipped, Some(section))
}

/// Put one part into the document, per its plan. The
/// plan is decided against the document as it stands
/// when the part lands, so an apply walks the same
/// ground the review's simulation walked. Answers
/// whether the part landed COMPLETELY (round 5, R5-B):
/// every one of its words placed, or already there, or
/// kept as a loose sheet is meant to be kept. The
/// applied record is written only for contributions
/// whose every part answers true, so a part that
/// landed partly or not at all is never remembered as
/// applied. Round 6 (R6-B): landing is per paragraph —
/// a paragraph already in the part's target story is
/// skipped and counted as already there, and only the
/// missing ones land, so a partly present part never
/// prints its present paragraphs twice. Round 8
/// (R8-7): the judging context — the headings the app
/// is judging in this story — is threaded in by the
/// apply loop, so a simulation slice keeps the whole
/// contribution's headings.
#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_part(
    doc: &mut Document,
    c: &ob_extract::contribution::Contribution,
    part: &ob_extract::contribution::Part,
    plan: &PartPlan,
    counts: &mut ApplyCounts,
    judged: &[JudgedHeading],
) -> bool {
    use ob_extract::contribution::PartKind;
    // The music slip (R-2a): a service part aimed at
    // the service story slots its hymn lines into the
    // bulletin's hymn lines, the way the review's plan
    // said, instead of appending the whole part.
    if part.kind == PartKind::Service {
        if let SlotTarget::Story(sid) = &plan.target {
            if service_story_id(doc).as_deref() == Some(sid.as_str()) {
                let slots = hymn_slots(doc, part);
                let mut spent: Vec<usize> = Vec::new();
                let mut landed = 0usize;
                let mut complete = 0usize;
                for (p, slot) in part.paragraphs.iter().zip(slots.iter()) {
                    match slot {
                        HymnSlot::Replace {
                            para_index, value, ..
                        } => {
                            // Replace stays as it is: setting
                            // the line to the same value is
                            // its own no-op.
                            set_service_line(doc, *para_index, value);
                            spent.push(*para_index);
                            landed += 1;
                            complete += 1;
                        }
                        HymnSlot::AppendHymn { value } => {
                            // Round 6 (R6-B): a hymn whose
                            // composed value already sits on
                            // a hymn line in the service story
                            // (the rule-2 judgment for this
                            // paragraph) counts as already
                            // there instead of appending a
                            // duplicate line.
                            if let Some(i) = find_hymn_line(doc, &p.text(), &spent) {
                                spent.push(i);
                                counts.already += 1;
                                complete += 1;
                            } else {
                                let style = service_line_style(doc);
                                let text = format!("Hymn\t{value}");
                                let para = ob_model::Paragraph::plain(&style, &text);
                                let new_index = doc.story(sid).map(|s| s.paragraphs.len());
                                if push_story_paragraph(doc, sid, para) {
                                    if let Some(i) = new_index {
                                        spent.push(i);
                                    }
                                    landed += 1;
                                    complete += 1;
                                }
                            }
                        }
                        HymnSlot::Plain => {
                            // A plain line already in the
                            // service story is skipped, not
                            // printed twice.
                            if let Some(i) = story_paragraph_index(doc, sid, &p.text(), &spent) {
                                spent.push(i);
                                counts.already += 1;
                                complete += 1;
                            } else {
                                let new_index = doc.story(sid).map(|s| s.paragraphs.len());
                                if push_story_paragraph(doc, sid, p.clone()) {
                                    if let Some(i) = new_index {
                                        spent.push(i);
                                    }
                                    landed += 1;
                                    complete += 1;
                                }
                            }
                        }
                    }
                }
                counts.service_lines += landed;
                counts.unplaced += part.paragraphs.len() - complete;
                if landed > 0 {
                    if let Some(kt) = c.keep_through {
                        apply_keep_through(doc, sid, kt);
                    }
                }
                return complete == part.paragraphs.len();
            }
        }
    }
    match &plan.target {
        SlotTarget::Story(sid) => {
            if doc.story(sid).is_none() {
                counts.unplaced += part.paragraphs.len();
                return false;
            }
            // Per-paragraph landing: the paragraphs
            // already in this story are skipped, only the
            // missing ones land. The part is complete when
            // every paragraph ended up in its target,
            // landed or already there, and the target
            // story exists.
            let (to_land, skipped) = split_present_paragraphs(doc, sid, part);
            let landed = to_land.len();
            if landed > 0 {
                if let Some(story) = doc.story_mut(sid) {
                    story.paragraphs.extend(to_land);
                }
            }
            match part.kind {
                PartKind::Service => counts.service_lines += landed,
                PartKind::Announcement => counts.announcements += landed,
                _ => counts.named += landed,
            }
            counts.already += skipped;
            if landed > 0 {
                if let Some(kt) = c.keep_through {
                    apply_keep_through(doc, sid, kt);
                }
            }
            true
        }
        SlotTarget::AnnouncementsUnderHeading(heading) => {
            let wid = weekly_story_id(doc);
            let Some(wid) = wid.as_ref() else {
                counts.unplaced += part.paragraphs.len();
                return false;
            };
            if doc.story(wid).is_none() {
                counts.unplaced += part.paragraphs.len();
                return false;
            }
            // Round 7 (R7-B): presence and landing are
            // scoped to the part's own heading section of
            // the weekly story. Paragraphs already under
            // the heading are skipped; the missing ones
            // land under the heading even when the same
            // words stand elsewhere in the story. When
            // the heading already stands, the missing
            // paragraphs go in at the end of its section;
            // when it stands nowhere, the heading is
            // added (only when a paragraph actually lands
            // under it, or the part carries no words at
            // all) with the paragraphs after it.
            let (to_land, skipped, section) =
                split_present_paragraphs_under_heading(doc, wid, heading, part, judged);
            let landed = to_land.len();
            match section {
                Some(section) => {
                    if landed > 0 {
                        if let Some(story) = doc.story_mut(wid) {
                            let at = section.1.min(story.paragraphs.len());
                            for (n, p) in to_land.into_iter().enumerate() {
                                story.paragraphs.insert(at + n, p);
                            }
                        }
                        counts.under_headings += landed;
                        if let Some(kt) = c.keep_through {
                            apply_keep_through(doc, wid, kt);
                        }
                    }
                }
                None => {
                    if landed > 0 || part.paragraphs.is_empty() {
                        let head = heading_paragraph(doc, heading);
                        if let Some(story) = doc.story_mut(wid) {
                            story.paragraphs.push(head);
                            story.paragraphs.extend(to_land);
                        }
                        counts.under_headings += landed;
                        if landed > 0 {
                            if let Some(kt) = c.keep_through {
                                apply_keep_through(doc, wid, kt);
                            }
                        }
                    }
                }
            }
            counts.already += skipped;
            // The part counts as landed only when its
            // heading and its words both stand, judged
            // with the one presence notion.
            match target_checker(doc, part, &plan.target, judged) {
                Some(mut checker) => checker.part_present(part),
                None => false,
            }
        }
        // A loose sheet lands by being kept with the
        // part, exactly as its plan said: complete.
        SlotTarget::LooseSheet => {
            counts.loose += 1;
            true
        }
        SlotTarget::KeptWithPart => {
            counts.unplaced += part.paragraphs.len();
            // Nothing landed, unless the part carried no
            // words at all: then there was nothing to lose.
            part.paragraphs.is_empty()
        }
        SlotTarget::AlreadyThere => {
            counts.already += part.paragraphs.len();
            true
        }
    }
}

/// What one apply did, in full: the summary lines
/// Accept reports, and whether every part landed
/// completely (round 5, R5-B). The applied record is
/// written only when `all_landed` is true.
pub(crate) struct ApplyOutcome {
    pub(crate) lines: Vec<String>,
    pub(crate) all_landed: bool,
}

/// The sentence that names a part whose words did not
/// all land, and says why in the office's words. The
/// words themselves stay with the part when it is
/// filed, as the plan already promised.
pub(crate) fn unlanded_sentence(part: &ob_extract::contribution::Part, plan: &PartPlan) -> String {
    let kind = part.kind.heading();
    match &plan.target {
        SlotTarget::KeptWithPart => format!(
            "The {kind} words are not in the bulletin: there is no \
             announcements box in it yet, so the words stay with \
             the part when it is filed."
        ),
        _ => format!(
            "The {kind} words are not all in the bulletin: the box \
             they belong in could not be found in it, so the words \
             stay with the part when it is filed."
        ),
    }
}

/// Put a reviewed contribution's parts into the document,
/// each planned against the document the parts before
/// it left behind, exactly as the review's plan was
/// simulated. Returns the summary lines Accept reports:
/// counts per part, the loose sheet's fate said out
/// loud, and a naming sentence for every part whose
/// words did not all land.
pub(crate) fn apply_contribution_outcome(
    doc: &mut Document,
    c: &ob_extract::contribution::Contribution,
) -> ApplyOutcome {
    let judged = judged_headings(doc, c);
    apply_contribution_outcome_judged(doc, c, &judged)
}

/// `apply_contribution_outcome` with the judging context
/// given (round 8, R8-7).
pub(crate) fn apply_contribution_outcome_judged(
    doc: &mut Document,
    c: &ob_extract::contribution::Contribution,
    judged: &[JudgedHeading],
) -> ApplyOutcome {
    let mut counts = ApplyCounts::default();
    let mut all_landed = true;
    let mut unlanded: Vec<String> = Vec::new();
    for part in &c.parts {
        let plan = plan_part_judged(doc, c, part, judged);
        if !apply_part(doc, c, part, &plan, &mut counts, judged) {
            all_landed = false;
            unlanded.push(unlanded_sentence(part, &plan));
        }
    }
    let ApplyCounts {
        service_lines,
        announcements,
        named,
        under_headings,
        loose,
        unplaced,
        already,
    } = counts;
    if announcements + under_headings > 0 {
        // Words arrived: the office is no longer saying
        // there are no announcements this week.
        doc.no_announcements = false;
    }
    let mut out: Vec<String> = Vec::new();
    if service_lines > 0 {
        out.push(format!(
            "Added {service_lines} line{} to the service.",
            if service_lines == 1 { "" } else { "s" }
        ));
    }
    if announcements > 0 {
        out.push(format!(
            "Added {announcements} announcement line{}.",
            if announcements == 1 { "" } else { "s" }
        ));
    }
    if named > 0 {
        out.push(format!(
            "Added {named} line{} to their boxes.",
            if named == 1 { "" } else { "s" }
        ));
    }
    if under_headings > 0 {
        out.push(format!(
            "Added {under_headings} line{} to the announcements, under their headings.",
            if under_headings == 1 { "" } else { "s" }
        ));
    }
    if loose > 0 {
        out.push("The loose sheet is filed with this part and kept safe.".to_string());
    }
    if unplaced > 0 {
        out.push(format!(
            "{unplaced} line{} had no box to go into; they are filed with this part.",
            if unplaced == 1 { "" } else { "s" }
        ));
    }
    if already > 0 {
        out.push(if already == 1 {
            "1 line was already in the bulletin, so it was not added again.".to_string()
        } else {
            format!("{already} lines were already in the bulletin, so they were not added again.")
        });
    }
    out.extend(unlanded);
    ApplyOutcome {
        lines: out,
        all_landed,
    }
}

/// Apply a contribution and report only the summary
/// lines: the shape the older tests use. (The review's
/// plan simulation goes through
/// `apply_contribution_judged`, so its per-part slices
/// keep the whole contribution's judging context.)
/// Accept itself goes through
/// `apply_contribution_recording`, which also knows
/// whether the words may be remembered as applied.
#[allow(dead_code)]
pub(crate) fn apply_contribution(
    doc: &mut Document,
    c: &ob_extract::contribution::Contribution,
) -> Vec<String> {
    apply_contribution_outcome(doc, c).lines
}

/// `apply_contribution` with the judging context given
/// (round 8, R8-7): the plan simulation's per-part slice
/// keeps the whole contribution's headings.
pub(crate) fn apply_contribution_judged(
    doc: &mut Document,
    c: &ob_extract::contribution::Contribution,
    judged: &[JudgedHeading],
) -> Vec<String> {
    apply_contribution_outcome_judged(doc, c, judged).lines
}

/// Apply a reviewed contribution the way Accept does,
/// and record it in the document when, and only when,
/// every part landed completely (round 5, R5-B). A
/// part that landed partly or not at all leaves no
/// record behind, so the review can offer its words
/// again honestly instead of swearing they are in the
/// bulletin when they are not.
pub(crate) fn apply_contribution_recording(
    doc: &mut Document,
    c: &ob_extract::contribution::Contribution,
    text: &str,
) -> ApplyOutcome {
    let outcome = apply_contribution_outcome(doc, c);
    if outcome.all_landed {
        record_applied_fingerprint(doc, text);
    }
    outcome
}

/// One file-name piece made safe for any folder: letters,
/// digits, spaces, and dashes stay; everything else
/// becomes a dash.
pub(crate) fn sanitize_file_part(s: &str) -> String {
    let cleaned: String = s
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == ' ' || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let cleaned = cleaned.trim().trim_matches('-').trim().to_string();
    if cleaned.is_empty() {
        "no name".to_string()
    } else {
        cleaned
    }
}

/// The name a contribution is filed under: sender, Sunday,
/// and content fingerprint, so which words were used stays
/// answerable later.
pub(crate) fn filed_file_name(
    from: Option<&str>,
    for_date: Option<ob_model::SimpleDate>,
    fingerprint: &str,
) -> String {
    let sender = sanitize_file_part(from.unwrap_or(""));
    let sunday = for_date
        .map(|d| format!("{:04}-{:02}-{:02}", d.year, d.month, d.day))
        .unwrap_or_else(|| "no Sunday named".to_string());
    format!("{sender} - {sunday} - {fingerprint}.md")
}

/// Rebuild a contribution's text from its pieces (the
/// review's Edit mode): the header lines as written, then
/// each part under its heading. The rebuilt text is what
/// gets re-parsed, and what gets filed after an edit.
pub(crate) fn rebuild_contribution_text(
    from: &str,
    for_sunday: &str,
    keep_through: &str,
    parts: &[(ob_extract::contribution::PartKind, String)],
) -> String {
    let mut out = String::new();
    if !from.trim().is_empty() {
        out.push_str(&format!("From: {}\n", from.trim()));
    }
    if !for_sunday.trim().is_empty() {
        out.push_str(&format!("For Sunday: {}\n", for_sunday.trim()));
    }
    if !keep_through.trim().is_empty() {
        out.push_str(&format!("Keep-through: {}\n", keep_through.trim()));
    }
    for (kind, body) in parts {
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&format!("## {}\n\n{}\n", kind.heading(), body.trim()));
    }
    out
}

/// The sender's card as markdown: one page that teaches
/// the whole contribution shape, example included. It is
/// the entire training program, so the words are written
/// for the sender, not about them.
pub(crate) fn sender_card_markdown(church_name: &str) -> String {
    let church = if church_name.trim().is_empty() {
        "your church office".to_string()
    } else {
        church_name.trim().to_string()
    };
    format!(
        "# Sending words for the bulletin\n\
         \n\
         From {church}\n\
         \n\
         Anyone can send words for the bulletin: the pastor, the music director, \
         Sunday school, the youth group, or you. You do not need any program. \
         Type your words in an email or a notes app, in the shape below, and send \
         them to the church office.\n\
         \n\
         ## The shape\n\
         \n\
         Start with two short lines:\n\
         \n\
         From: your name or your job (Pastor, Music director)\n\
         \n\
         For Sunday: the date, written any way you like (October 25, 2026)\n\
         \n\
         Then give each part its own heading, spelled like one of these:\n\
         \n\
         ## For the service\n\
         \n\
         ## Announcement\n\
         \n\
         ## Insert\n\
         \n\
         ## Sunday school\n\
         \n\
         ## Youth\n\
         \n\
         Under each heading, type the words just as they should print. \
         **Bold** and *italic* work the way you would expect.\n\
         \n\
         An announcement can stay in for a few weeks. Add a third line at the \
         top: Keep-through: the last Sunday it should print.\n\
         \n\
         ## An example\n\
         \n\
         From: Pastor Lee\n\
         \n\
         For Sunday: October 25, 2026\n\
         \n\
         ## For the service\n\
         \n\
         Sermon: **The Good Shepherd**\n\
         \n\
         ## Announcement\n\
         \n\
         Harvest supper is Saturday at 5 PM. Bring a dish to share.\n\
         \n\
         Please send your words by ______________ (the office fills this in).\n"
    )
}

/// Where contributions wait: an "inbox" folder beside the
/// Font Library folder (the clips discipline), with a
/// "filed" subfolder for parts already used.
pub(crate) fn inbox_dir() -> std::path::PathBuf {
    let fonts = ob_fonts::default_library_dir();
    match fonts.parent() {
        Some(base) => base.join("inbox"),
        None => std::path::PathBuf::from("inbox"),
    }
}

pub(crate) fn filed_dir() -> std::path::PathBuf {
    inbox_dir().join("filed")
}

/// Where the loose sheet library lives: an "inserts"
/// folder beside the Font Library folder (the clips and
/// inbox discipline), holding one .obdi file per sheet.
pub(crate) fn insert_library_dir() -> std::path::PathBuf {
    let fonts = ob_fonts::default_library_dir();
    match fonts.parent() {
        Some(base) => base.join("inserts"),
        None => std::path::PathBuf::from("inserts"),
    }
}

/// Words compared as words (round 5, R5-B): case
/// folded and every run of whitespace, tabs included,
/// read as one space. Presence is a question about the
/// words, not about the bytes they arrived in, so a
/// part re-sent with different spacing is still the
/// same words.
pub(crate) fn normalized_words(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Whether `needle` appears in `entry` as whole words:
/// the whole entry, or fenced by spaces on the side it
/// does not run off. Both sides are normalized first,
/// so a short line cannot claim presence by hiding
/// inside a longer word.
pub(crate) fn contains_words(entry: &str, needle: &str) -> bool {
    !needle.is_empty()
        && (entry == needle
            || entry.starts_with(&format!("{needle} "))
            || entry.ends_with(&format!(" {needle}"))
            || entry.contains(&format!(" {needle} ")))
}

/// The composed value a service part's hymn line lands
/// as, when the line is a hymn entry: the same test
/// `hymn_slots` slots by (a number, and a hymn label
/// or a named hymnal). A hymn line is not copied into
/// the bulletin word for word; it is composed into
/// the bulletin's own hymn line, so its presence is
/// judged by the composed form.
pub(crate) fn hymn_composed_form(text: &str) -> Option<String> {
    use ob_model::hymns::{compose_hymn_value, is_hymn_label, parse_hymn_value};
    let (label, value) = part_line_label(text);
    let entry = parse_hymn_value(&value);
    let hymn_labelled = label.as_deref().map(is_hymn_label).unwrap_or(false);
    if !text.trim().is_empty()
        && entry.number.is_some()
        && (hymn_labelled || entry.hymnal.is_some())
    {
        Some(compose_hymn_value(&entry))
    } else {
        None
    }
}

/// The index of the paragraph in story `sid` whose
/// words contain `text`'s words, skipping the indexes
/// in `spent`. Presence is scoped to one story (round
/// 6, R6-B): words standing in some other story never
/// count for a part that would land here. A match
/// spends its paragraph, so a line the part carries
/// twice needs the story to carry it twice.
pub(crate) fn story_paragraph_index(
    doc: &Document,
    sid: &str,
    text: &str,
    spent: &[usize],
) -> Option<usize> {
    let needle = normalized_words(text);
    if needle.is_empty() {
        return None;
    }
    let story = doc.story(sid)?;
    story
        .paragraphs
        .iter()
        .enumerate()
        .find(|(i, p)| !spent.contains(i) && contains_words(&normalized_words(&p.text()), &needle))
        .map(|(i, _)| i)
}

/// Whether a paragraph wears a heading style (round 7,
/// R7-B; round 8, R8-8b: recognized by the shared
/// name-or-id rule, so a heading style whose id is not
/// literally "heading" / "subheading" still reads).
pub(crate) fn is_styled_heading_paragraph(doc: &Document, p: &ob_model::Paragraph) -> bool {
    is_heading_style(doc, &p.style)
}

/// Whether a paragraph is the bold body paragraph
/// `heading_paragraph` falls back to when the document
/// has no heading style.
pub(crate) fn is_bold_fallback_paragraph(p: &ob_model::Paragraph) -> bool {
    !p.runs.is_empty()
        && !normalized_words(&p.text()).is_empty()
        && p.runs.iter().all(|r| r.bold == Some(true))
}

/// Whether a heading paragraph's normalized words are
/// the heading `want` (round 8, R8-8a; round 9, R9-7):
/// the words exactly, or the words plus a parenthesized
/// suffix ("Sunday school (Room 2)"). A longer heading
/// ("Sunday school teachers") does not match. Trailing
/// punctuation on the heading paragraph — a final
/// period, at minimum — does not defeat the match.
/// One rule for styled and bold-fallback headings
/// alike, and for part headings (see `part_heading`
/// in ob-extract, which applies the same shape on the
/// sender's side).
pub(crate) fn heading_words_match(words: &str, want: &str) -> bool {
    let core = words.trim_end_matches('.');
    if core == want {
        return true;
    }
    let prefix = format!("{want} (");
    core.starts_with(&prefix) && core.ends_with(')')
}

/// Whether a paragraph ends a heading section (round 8,
/// R8-7): a heading-styled paragraph always does, as
/// before. A bold-fallback paragraph does only when its
/// words are a heading the app is judging in this story
/// — the target heading itself, or one of `judged` (the
/// other part headings of the contribution being
/// planned or applied), each read by the one suffix
/// rule (round 9, R9-7). Any other bold paragraph is
/// content: a bold emphasis line inside a section does
/// not end it.
pub(crate) fn is_section_boundary(
    doc: &Document,
    p: &ob_model::Paragraph,
    target: &str,
    judged: &[String],
) -> bool {
    if is_styled_heading_paragraph(doc, p) {
        return true;
    }
    if !is_bold_fallback_paragraph(p) {
        return false;
    }
    let words = normalized_words(&p.text());
    heading_words_match(&words, target) || judged.iter().any(|j| heading_words_match(&words, j))
}

/// Whether a paragraph is a heading paragraph for the
/// heading `want` (normalized words) whose section is
/// being found (round 8, R8-8a; round 9, R9-7): its
/// words match by the one rule (`heading_words_match`)
/// — exactly, or plus a parenthesized suffix, trailing
/// punctuation aside — for a heading-styled paragraph
/// and for a bold-fallback paragraph alike. A
/// bold-fallback paragraph must also stand as a judged
/// heading to reach the match at all (see
/// `is_section_boundary`).
pub(crate) fn is_target_heading_paragraph(
    doc: &Document,
    p: &ob_model::Paragraph,
    want: &str,
    judged: &[String],
) -> bool {
    if !is_section_boundary(doc, p, want, judged) {
        return false;
    }
    let words = normalized_words(&p.text());
    heading_words_match(&words, want)
}

/// The sections of story `sid` that stand under a
/// heading with these words (round 7, R7-B): each
/// section is the paragraph range (start, end) after
/// one heading paragraph and before the next heading
/// paragraph of any words. An under-heading part's
/// words count as present only inside one of its own
/// heading's sections: the same words standing
/// elsewhere in the story — before the heading, or
/// under another heading — are not its presence.
pub(crate) fn heading_sections(
    doc: &Document,
    sid: &str,
    heading: &str,
    judged: &[JudgedHeading],
) -> Vec<(usize, usize)> {
    let Some(story) = doc.story(sid) else {
        return Vec::new();
    };
    // Round 9 (R9-3): the judging set is scoped to THIS
    // story. Only the headings of parts whose target is
    // this story delimit its sections; a part aimed at
    // another story never redraws this story's
    // boundaries.
    let scoped: Vec<String> = judged
        .iter()
        .filter(|j| j.story == sid)
        .map(|j| j.words.clone())
        .collect();
    let want = normalized_words(heading);
    let paras = &story.paragraphs;
    let mut sections = Vec::new();
    let mut i = 0usize;
    while i < paras.len() {
        if is_target_heading_paragraph(doc, &paras[i], &want, &scoped) {
            let start = i + 1;
            let mut end = start;
            while end < paras.len() && !is_section_boundary(doc, &paras[end], &want, &scoped) {
                end += 1;
            }
            sections.push((start, end));
            i = end;
        } else {
            i += 1;
        }
    }
    sections
}

/// One heading the app is judging, scoped to the story
/// its part targets (round 9, R9-3): the story id, and
/// the part heading's words, normalized.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct JudgedHeading {
    pub(crate) story: String,
    pub(crate) words: String,
}

/// The story one part's words would land in, as a story
/// id: the named story its plan targets, or the weekly
/// story for a part that lands under a heading there.
/// None when the part lands in no story (a loose sheet,
/// or words kept with the part).
pub(crate) fn part_target_story(
    doc: &Document,
    c: &ob_extract::contribution::Contribution,
    part: &ob_extract::contribution::Part,
) -> Option<String> {
    match planned_part_plan(doc, c, part).target {
        SlotTarget::Story(sid) => Some(sid),
        SlotTarget::AnnouncementsUnderHeading(_) => weekly_story_id(doc),
        _ => None,
    }
}

/// The headings the app is judging for one contribution
/// (round 8, R8-7; round 9, R9-3: scoped per story):
/// each part's heading, paired with the story that part
/// targets. When sections are computed in a story, the
/// bold paragraphs that delimit are the target heading
/// and the headings in this set scoped to THAT story —
/// a part targeting a named story never redraws
/// another story's boundaries.
pub(crate) fn judged_headings(
    doc: &Document,
    c: &ob_extract::contribution::Contribution,
) -> Vec<JudgedHeading> {
    let mut out: Vec<JudgedHeading> = Vec::new();
    for part in &c.parts {
        let Some(story) = part_target_story(doc, c, part) else {
            continue;
        };
        let words = normalized_words(part.kind.heading());
        let entry = JudgedHeading { story, words };
        if !out.contains(&entry) {
            out.push(entry);
        }
    }
    out
}

/// `story_paragraph_index` restricted to one paragraph
/// range of the story (round 7, R7-B): the search an
/// under-heading part's presence and landing share, so
/// both judge the same section the same way.
pub(crate) fn story_paragraph_index_in_range(
    doc: &Document,
    sid: &str,
    text: &str,
    spent: &[usize],
    range: (usize, usize),
) -> Option<usize> {
    let needle = normalized_words(text);
    if needle.is_empty() {
        return None;
    }
    let story = doc.story(sid)?;
    story
        .paragraphs
        .iter()
        .enumerate()
        .skip(range.0)
        .take(range.1.saturating_sub(range.0))
        .find(|(i, p)| !spent.contains(i) && contains_words(&normalized_words(&p.text()), &needle))
        .map(|(i, _)| i)
}

/// The paragraph index of the service-story hymn line
/// carrying a hymn paragraph's composed value, when
/// the paragraph counts as present (round 6, R6-B,
/// rule 2). A hymn-valued paragraph is present only on
/// a hymn line in the service story: when the paragraph
/// names a line (its label), the value must sit on a
/// service line with that label, or on an appended line
/// labelled "Hymn"; when it names none, any
/// hymn-labelled service line carrying the value
/// counts. Text anywhere else — announcements, other
/// stories, non-hymn service lines — never counts.
/// Indexes in `spent` are skipped, so each line can
/// satisfy one paragraph.
pub(crate) fn find_hymn_line(doc: &Document, text: &str, spent: &[usize]) -> Option<usize> {
    let composed = normalized_words(&hymn_composed_form(text)?);
    if composed.is_empty() {
        return None;
    }
    let (label, _) = part_line_label(text);
    service_lines(doc)
        .into_iter()
        .find(|line| {
            if spent.contains(&line.para_index) {
                return false;
            }
            let label_ok = match &label {
                Some(named) => {
                    line.label.eq_ignore_ascii_case(named)
                        || line.label.eq_ignore_ascii_case("Hymn")
                }
                None => ob_model::hymns::is_hymn_label(&line.label),
            };
            label_ok && contains_words(&normalized_words(&line.value), &composed)
        })
        .map(|line| line.para_index)
}

/// Presence judged against one target story (round 6,
/// R6-B): the paragraphs of that story only, each
/// spendable once. In hymn mode (a service part whose
/// target is the service story itself), hymn-valued
/// paragraphs are judged by the hymn-line rule and the
/// rest by their words in the service story.
pub(crate) struct TargetPresence<'a> {
    pub(crate) doc: &'a Document,
    pub(crate) sid: Option<String>,
    pub(crate) hymn_mode: bool,
    pub(crate) spent: Vec<usize>,
    /// Round 7 (R7-B): for an under-heading part, the
    /// sections of the target story the part's words
    /// must stand in (see `heading_sections`). Empty
    /// searches the whole story, as a plain story part
    /// always has. Round 8 (R8-2): the checker searches
    /// every section of the part's heading — one section
    /// judgment for checker, plan, and landing — so a
    /// paragraph standing under the heading in any of
    /// its sections counts, and none standing only
    /// outside them does.
    pub(crate) ranges: Vec<(usize, usize)>,
    /// Round 7 (R7-B): the placement itself does not
    /// stand — an under-heading part whose heading is
    /// nowhere in the target story. Nothing is present
    /// for such a part, however its words stand
    /// elsewhere in the story.
    pub(crate) absent: bool,
}

impl TargetPresence<'_> {
    pub(crate) fn paragraph_present(&mut self, text: &str) -> bool {
        if self.absent {
            return false;
        }
        if normalized_words(text).is_empty() {
            return true;
        }
        if self.hymn_mode && hymn_composed_form(text).is_some() {
            match find_hymn_line(self.doc, text, &self.spent) {
                Some(i) => {
                    self.spent.push(i);
                    return true;
                }
                None => return false,
            }
        }
        let Some(sid) = self.sid.clone() else {
            return false;
        };
        let found = if self.ranges.is_empty() {
            story_paragraph_index(self.doc, &sid, text, &self.spent)
        } else {
            let ranges = self.ranges.clone();
            ranges.iter().find_map(|range| {
                story_paragraph_index_in_range(self.doc, &sid, text, &self.spent, *range)
            })
        };
        match found {
            Some(i) => {
                self.spent.push(i);
                true
            }
            None => false,
        }
    }

    pub(crate) fn part_present(&mut self, part: &ob_extract::contribution::Part) -> bool {
        if self.absent {
            return false;
        }
        part.paragraphs
            .iter()
            .all(|p| self.paragraph_present(&p.text()))
    }
}

/// The presence checker for a part's planned target:
/// the story it would land in (the weekly story for an
/// under-heading part). None when the target names no
/// story a checker can search.
pub(crate) fn target_checker<'a>(
    doc: &'a Document,
    part: &ob_extract::contribution::Part,
    target: &SlotTarget,
    judged: &[JudgedHeading],
) -> Option<TargetPresence<'a>> {
    use ob_extract::contribution::PartKind;
    match target {
        SlotTarget::Story(sid) => {
            let hymn_mode = part.kind == PartKind::Service
                && service_story_id(doc).as_deref() == Some(sid.as_str());
            Some(TargetPresence {
                doc,
                sid: Some(sid.clone()),
                hymn_mode,
                spent: Vec::new(),
                ranges: Vec::new(),
                absent: false,
            })
        }
        SlotTarget::AnnouncementsUnderHeading(heading) => {
            // Round 7 (R7-B): an under-heading part is
            // present only under its own heading. Round 8
            // (R8-2): the checker searches the heading's
            // sections — every one of them, the same
            // sections the landing splits against — so a
            // paragraph standing under the heading in any
            // of its sections counts, and the whole part
            // is present when every one of its paragraphs
            // stands under the heading in one of them.
            // No section at all: the placement is absent,
            // and the part is not present however its
            // words stand elsewhere.
            let wid = weekly_story_id(doc)?;
            let sections = heading_sections(doc, &wid, heading, judged);
            let absent = sections.is_empty();
            Some(TargetPresence {
                doc,
                sid: Some(wid),
                hymn_mode: false,
                spent: Vec::new(),
                ranges: sections,
                absent,
            })
        }
        _ => None,
    }
}

/// Whether a part carries no words at all: every
/// paragraph empty. There is nothing that could be
/// missing, so such a part is present trivially
/// wherever it would land — the one case a part with
/// no landing story (KeptWithPart) counts as present.
pub(crate) fn part_has_no_words(part: &ob_extract::contribution::Part) -> bool {
    part.paragraphs
        .iter()
        .all(|p| normalized_words(&p.text()).is_empty())
}

/// Whether every one of a part's paragraphs is present
/// in the story the part would land in, the way a
/// complete landing would leave them (round 6, R6-B;
/// round 7, R7-B: for an under-heading part, present
/// means standing under its own heading — the same
/// words elsewhere in the story are not its presence,
/// and a missing heading means the part is not
/// present). A part is more than its first line: one
/// shared boilerplate line cannot make a part "already
/// in" while the rest of its words are missing, and
/// words standing in another story cannot make it
/// present here. A loose-sheet part is never present
/// by words: its landing is its filing, judged by the
/// record in `applied_claim`, so the plan never calls
/// it AlreadyThere.
pub(crate) fn part_words_present(
    doc: &Document,
    c: &ob_extract::contribution::Contribution,
    part: &ob_extract::contribution::Part,
) -> bool {
    let judged = judged_headings(doc, c);
    part_words_present_judged(doc, c, part, &judged)
}

/// `part_words_present` with the judging context given
/// (round 8, R8-7): the headings the app is judging in
/// this story, threaded in by callers whose contribution
/// is a simulation slice of a larger one.
pub(crate) fn part_words_present_judged(
    doc: &Document,
    c: &ob_extract::contribution::Contribution,
    part: &ob_extract::contribution::Part,
    judged: &[JudgedHeading],
) -> bool {
    let plan = planned_part_plan(doc, c, part);
    match &plan.target {
        SlotTarget::LooseSheet => false,
        SlotTarget::KeptWithPart => part_has_no_words(part),
        SlotTarget::AlreadyThere => true,
        target => match target_checker(doc, part, target, judged) {
            Some(mut checker) => checker.part_present(part),
            None => part_has_no_words(part),
        },
    }
}

/// Whether every story-landing part of a contribution
/// has all its words present in its own target, judged
/// part by part with the very judgment the plan uses
/// (`part_words_present`), so the claim and the plan
/// lines can never disagree (round 7, R7-B). Each part
/// gets a fresh checker: one standing copy placed as a
/// landing places it satisfies every part that asked
/// for exactly that placement, so a paragraph repeated
/// across two parts settles instead of demanding the
/// words stand twice. Loose-sheet parts are not judged
/// here at all: their words never enter a story, so
/// their presence is the applied record's business
/// (see `applied_claim`).
pub(crate) fn contribution_words_present(
    doc: &Document,
    c: &ob_extract::contribution::Contribution,
) -> bool {
    for part in &c.parts {
        let plan = planned_part_plan(doc, c, part);
        match &plan.target {
            SlotTarget::LooseSheet => continue,
            SlotTarget::AlreadyThere => continue,
            _ => {
                if !part_words_present(doc, c, part) {
                    return false;
                }
            }
        }
    }
    true
}

/// Whether any part of a contribution lands as a loose
/// sheet: filed with the part, never in a story.
pub(crate) fn contribution_has_loose_part(
    doc: &Document,
    c: &ob_extract::contribution::Contribution,
) -> bool {
    c.parts.iter().any(|part| {
        matches!(
            planned_part_plan(doc, c, part).target,
            SlotTarget::LooseSheet
        )
    })
}

/// What the review may honestly say about a part and
/// this bulletin (round 5, R5-B). The applied record
/// alone never swears the words are in: the claim of
/// "already in" is made only when the words themselves
/// are found in the document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AppliedClaim {
    /// The words are in the bulletin (with or without
    /// a record: a re-sent part whose bytes differ
    /// still lands on its words). File it; applying
    /// again would print the words twice.
    AlreadyIn,
    /// The document's record says the words were
    /// applied, but they are not in it now (deleted by
    /// hand after a failed save, the road round 5's
    /// RT5-3 walked). Accept is offered again, under a
    /// sentence that says exactly this.
    RecordedButWordsGone,
    /// Neither a record nor the words: an ordinary
    /// review, with an ordinary Accept.
    NotApplied,
}

/// The claim for one reviewed contribution against
/// this document (round 6, R6-B, rule 3). Story-landing
/// parts are judged by target-scoped presence; a loose
/// part's landing IS its filing, so loose parts are
/// judged by the record. AlreadyIn: every story part is
/// present in its target, and the contribution either
/// carries no loose parts or is recorded. A loose-only
/// contribution is therefore AlreadyIn exactly when
/// recorded, and NotApplied when not.
pub(crate) fn applied_claim(
    doc: &Document,
    c: &ob_extract::contribution::Contribution,
    text: &str,
) -> AppliedClaim {
    let recorded = part_already_applied(doc, text);
    let story_present = contribution_words_present(doc, c);
    let has_loose = contribution_has_loose_part(doc, c);
    if story_present && (!has_loose || recorded) {
        AppliedClaim::AlreadyIn
    } else if recorded {
        AppliedClaim::RecordedButWordsGone
    } else {
        AppliedClaim::NotApplied
    }
}

/// A short content fingerprint: the first eight letters
/// of the content hash. Two versions of a part with the
/// same fingerprint are the same words.
pub(crate) fn content_fingerprint(bytes: &[u8]) -> String {
    ob_model::sha256_hex(bytes).chars().take(8).collect()
}

/// The review's first line when the part under review
/// was already applied to the bulletin on screen in
/// this session (round 3, R3-5): said once, in these
/// words, and pinned by a test. Round 5 (R5-B): the
/// line is earned by the claim in `applied_claim`,
/// which finds the words themselves in the bulletin;
/// the record alone no longer swears it.
pub(crate) const APPLIED_PART_FIRST_LINE: &str =
    "These words are already in the bulletin on screen. \
    Saving the bulletin finishes this part; there is nothing more to add.";

/// The review's first line when the document's record
/// says a part was applied but its words are not in
/// the bulletin now (round 5, R5-B): the record is a
/// note, not the words, and the office is told the
/// words can simply be added again.
pub(crate) const APPLIED_RECORD_MISSING_LINE: &str =
    "Our note says these words were added to this \
    bulletin before, but they are not in it now, so you can add them again.";

/// Whether the document's record says these exact
/// words were applied to it. The record lives in the
/// document itself (round 4, R4-4/R4-6), so it survives
/// a save that failed, a save done by hand, and a
/// crash the journal recovers from; Undo takes it away
/// with the words, and Next Sunday starts the new week
/// without it. The fingerprint is the same one the
/// filed name carries, so the file a failed save left
/// in the inbox and the words on screen compare equal.
/// Round 5 (R5-B): this is the record half of the
/// story only. What the review may claim is decided
/// by `applied_claim`, which also looks for the words
/// themselves: a record whose words were deleted by
/// hand no longer blocks Accept, and a re-sent part
/// whose bytes changed is still caught by its words.
pub(crate) fn part_already_applied(doc: &Document, text: &str) -> bool {
    doc.applied_contributions
        .iter()
        .any(|fp| fp == &content_fingerprint(text.as_bytes()))
}

/// Record a contribution's words as applied to the
/// document, at the moment they land (round 4, R4-4):
/// before any save is attempted, so no path that
/// follows (a failed save, a manual save, a recovery)
/// can offer to apply them again.
pub(crate) fn record_applied_fingerprint(doc: &mut Document, text: &str) {
    let fp = content_fingerprint(text.as_bytes());
    if !doc.applied_contributions.contains(&fp) {
        doc.applied_contributions.push(fp);
    }
}

/// One markdown file found in the inbox, read and parsed.
/// A file that does not parse stays on the list as
/// unreadable, with the reason: it is never hidden.
pub(crate) struct InboxItem {
    pub(crate) path: std::path::PathBuf,
    pub(crate) file_name: String,
    /// Seconds since the epoch, for newest-first order.
    pub(crate) modified_secs: u64,
    pub(crate) fingerprint: String,
    pub(crate) text: String,
    pub(crate) parsed: Result<ob_extract::contribution::Contribution, String>,
    /// Filed items only: the Sunday the filed name
    /// carries (round 3, R3-10), read back out of the
    /// name file_contribution wrote. That is the
    /// Sunday the part was filed for, whatever its
    /// words name; the still-waiting count trusts it
    /// and nothing else (round 16, R16-6).
    pub(crate) filed_for: Option<ob_model::SimpleDate>,
}

/// The Sunday a filed part's name carries: the date
/// segment between the sender and the fingerprint,
/// split from the right so a sender whose name holds
/// dashes survives. None when the name carries no date
/// (the "no Sunday named" names older filings used).
pub(crate) fn filed_sunday_from_name(file_name: &str) -> Option<ob_model::SimpleDate> {
    let stem = file_name
        .strip_suffix(".md")
        .or_else(|| file_name.strip_suffix(".MD"))?;
    let (rest, _fingerprint) = stem.rsplit_once(" - ")?;
    let (_sender, date) = rest.rsplit_once(" - ")?;
    ob_model::parse_flexible_date(
        date.trim(),
        ob_model::SimpleDate {
            year: 2000,
            month: 1,
            day: 1,
        },
    )
}

/// The largest contribution file the inbox will read
/// (round 3, RT3-E). A part is a page or two of
/// markdown, so 512 KB is already generous. Past it
/// the file is listed as unreadable with the reason
/// below and is never parsed: the scan used to read
/// every file whole and parse whatever came back, so
/// one 5 MB file cost the Start step seconds before
/// anything could be shown.
pub(crate) const MAX_PART_BYTES: u64 = 512 * 1024;

/// Read one inbox file through a cap, counting actual
/// bytes as they arrive, so no file, however the
/// filesystem describes it, can make the scan hold
/// more than the cap plus one byte. Read failures
/// keep the scan's long-standing words.
pub(crate) fn read_part_file(path: &std::path::Path) -> Result<Vec<u8>, String> {
    use std::io::Read as _;
    let mut file =
        std::fs::File::open(path).map_err(|e| format!("This file could not be read. ({e})"))?;
    let mut bytes = Vec::new();
    file.by_ref()
        .take(MAX_PART_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("This file could not be read. ({e})"))?;
    if bytes.len() as u64 > MAX_PART_BYTES {
        return Err("This file is too large to read as a part.".to_string());
    }
    Ok(bytes)
}

/// Read every `.md` file in a folder (the inbox, or the
/// filed folder when arrivals are counted) and parse each
/// as a contribution. Reading never moves a file. Filed
/// items also carry the Sunday their name was filed
/// under (round 3, R3-10).
pub(crate) fn scan_contribution_folder(
    dir: &std::path::Path,
    today: ob_model::SimpleDate,
    filed: bool,
) -> Vec<InboxItem> {
    let mut items: Vec<InboxItem> = Vec::new();
    let Ok(rd) = std::fs::read_dir(dir) else {
        return items;
    };
    for entry in rd.flatten() {
        let path = entry.path();
        let is_md = path
            .extension()
            .map(|e| e.eq_ignore_ascii_case("md"))
            .unwrap_or(false);
        if !is_md || !path.is_file() {
            continue;
        }
        let file_name = entry.file_name().to_string_lossy().into_owned();
        let modified_secs = entry
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let (fingerprint, text, parsed) = match read_part_file(&path) {
            Ok(bytes) => {
                let fp = content_fingerprint(&bytes);
                let text = String::from_utf8_lossy(&bytes).into_owned();
                let file_date = file_date_of(modified_secs);
                let parsed =
                    ob_extract::contribution::parse_contribution_anchored(&text, today, file_date);
                (fp, text, parsed)
            }
            Err(reason) => (String::new(), String::new(), Err(reason)),
        };
        let filed_for = if filed {
            filed_sunday_from_name(&file_name)
        } else {
            None
        };
        items.push(InboxItem {
            path,
            file_name,
            modified_secs,
            fingerprint,
            text,
            parsed,
            filed_for,
        });
    }
    items.sort_by(|a, b| a.file_name.cmp(&b.file_name));
    items
}

/// One sender-and-Sunday family of inbox items: versions
/// of (probably) the same part, newest on top.
pub(crate) struct InboxGroup {
    /// The sender as written, or "" when the words name
    /// nobody.
    pub(crate) from: String,
    pub(crate) for_date: Option<ob_model::SimpleDate>,
    /// Indexes into the scanned items, newest first.
    pub(crate) items: Vec<usize>,
}

/// Group parsed inbox items by (sender, Sunday). The
/// grouping is a plain name match, nothing smarter: the
/// sender words as written, compared without case.
pub(crate) fn group_inbox(items: &[InboxItem]) -> Vec<InboxGroup> {
    let mut groups: Vec<InboxGroup> = Vec::new();
    for (i, item) in items.iter().enumerate() {
        let Ok(c) = &item.parsed else { continue };
        let from = c.from.clone().unwrap_or_default();
        let key = from.trim().to_ascii_lowercase();
        let at = groups.iter().position(|g: &InboxGroup| {
            g.from.trim().to_ascii_lowercase() == key && g.for_date == c.for_date
        });
        match at {
            Some(at) => groups[at].items.push(i),
            None => groups.push(InboxGroup {
                from,
                for_date: c.for_date,
                items: vec![i],
            }),
        }
    }
    for g in &mut groups {
        g.items
            .sort_by_key(|&i| std::cmp::Reverse(items[i].modified_secs));
    }
    groups
}

/// The expected parts with no arrived contribution yet:
/// the Start step's "still waiting for" list. A
/// contribution counts as arrived when its From IS the
/// expected name, compared without case and ignoring
/// spaces at the ends (round 3, R3-10): "Lee" is not
/// satisfied by Leanne's part, however close the names
/// sit; the old substring match let exactly that
/// happen. Dating: an inbox part that names a Sunday
/// counts only for that Sunday. An undated part still
/// sitting in the inbox counts (it is waiting to be
/// reviewed, for the open Sunday by default). A FILED
/// part counts only for the Sunday in its filed name
/// (round 16, R16-6): the name is the record of the
/// Sunday it was filed for, and a re-parse of its
/// words — which anchors relatives at the file's date
/// bounded by scan day, so the same copy can re-read
/// to a different Sunday as days pass — may not
/// override it. Only a filed name carrying no date
/// (the older filings) falls back to the re-parsed
/// date, and an undated part under such a name counts
/// for no Sunday.
pub(crate) fn still_waiting(
    expected: &[String],
    inbox: &[ob_extract::contribution::Contribution],
    filed: &[(
        &ob_extract::contribution::Contribution,
        Option<ob_model::SimpleDate>,
    )],
    bulletin: Option<ob_model::SimpleDate>,
) -> Vec<String> {
    let same_sender = |from: &str, want: &str| {
        !want.trim().is_empty() && from.trim().eq_ignore_ascii_case(want.trim())
    };
    expected
        .iter()
        .filter(|want| {
            let in_inbox = inbox.iter().any(|c| {
                same_sender(c.from.as_deref().unwrap_or(""), want)
                    && match (c.for_date, bulletin) {
                        (Some(d), Some(b)) => d == b,
                        _ => true,
                    }
            });
            let in_filed = filed.iter().any(|(c, filed_for)| {
                same_sender(c.from.as_deref().unwrap_or(""), want)
                    && match filed_for {
                        // The filed name is the record
                        // (round 16, R16-6): the
                        // re-parsed date may not override
                        // the Sunday the part was filed
                        // for.
                        Some(f) => match bulletin {
                            Some(b) => *f == b,
                            None => true,
                        },
                        None => match (c.for_date, bulletin) {
                            (Some(d), Some(b)) => d == b,
                            (Some(_), None) => true,
                            (None, Some(_)) => false,
                            (None, None) => true,
                        },
                    }
            });
            !(in_inbox || in_filed)
        })
        .cloned()
        .collect()
}

/// The "Parts we usually get" field as a list: split on
/// commas and new lines, trimmed, empties dropped, and a
/// name kept once (compared without case).
pub(crate) fn parse_expected_parts(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for piece in text.split([',', '\n']) {
        let p = piece.trim();
        if !p.is_empty() && !out.iter().any(|e| e.eq_ignore_ascii_case(p)) {
            out.push(p.to_string());
        }
    }
    out
}

/// The reminder sentences for the missing parts, one per
/// part, ready to paste into the admin's own email or
/// message thread. The app sends nothing itself and never
/// claims to.
pub(crate) fn reminder_text(missing: &[String], bulletin: Option<ob_model::SimpleDate>) -> String {
    let date = bulletin
        .map(ob_extract::contribution::format_date_short)
        .unwrap_or_else(|| "this week".to_string());
    missing
        .iter()
        .map(|part| {
            format!(
                "A friendly reminder: the bulletin for Sunday, {date} is still \
                 waiting for {part}. Whenever it is ready, send it over and it \
                 will be added."
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}
