//! Fold-aware presets and bulletin templates.
//!
//! Bulletins are defined by the fold, not the page size. A preset starts from
//! the sheet and the fold; the volunteer edits reader panels and never has to
//! know which panel lands where on the printed sheet (that is ob-impose).

use chrono::{Datelike, Duration, NaiveDate};
use ob_model::*;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Preset {
    LetterBifold,
    LetterTrifold,
    LetterPortrait,
    LegalBifold,
    TabloidBifold,
    /// The shape our reference church actually uses: 5.5 x 8.5 inch
    /// pages, saddle-stitched into a booklet (16 pages by default),
    /// printed two pages per letter sheet side.
    HalfLetterBooklet,
}

impl Preset {
    pub fn id(&self) -> &'static str {
        match self {
            Preset::LetterBifold => "letter-bifold",
            Preset::LetterTrifold => "letter-trifold",
            Preset::LetterPortrait => "letter-portrait",
            Preset::LegalBifold => "legal-bifold",
            Preset::TabloidBifold => "tabloid-bifold",
            Preset::HalfLetterBooklet => "saddle-booklet",
        }
    }
    pub fn label(&self) -> &'static str {
        match self {
            Preset::LetterBifold => "Letter Bi-Fold Bulletin (11 x 8.5 sheet, folds to 5.5 x 8.5)",
            Preset::LetterTrifold => "Letter Tri-Fold Bulletin (11 x 8.5 sheet, 6 panels)",
            Preset::LetterPortrait => "Letter Portrait (8.5 x 11, multi-page)",
            Preset::LegalBifold => "Legal Bi-Fold (14 x 8.5 sheet, folds to 7 x 8.5)",
            Preset::TabloidBifold => "Tabloid Bi-Fold (17 x 11 sheet, folds to 8.5 x 11)",
            Preset::HalfLetterBooklet => {
                "Half-Letter Booklet (5.5 x 8.5 pages, 16 pages, stapled spine)"
            }
        }
    }
    pub fn parse(s: &str) -> Option<Preset> {
        Some(match s {
            "letter-bifold" => Preset::LetterBifold,
            "letter-trifold" => Preset::LetterTrifold,
            "letter-portrait" => Preset::LetterPortrait,
            "legal-bifold" => Preset::LegalBifold,
            "tabloid-bifold" => Preset::TabloidBifold,
            "saddle-booklet" => Preset::HalfLetterBooklet,
            _ => return None,
        })
    }
    pub fn all() -> [Preset; 6] {
        [
            Preset::LetterBifold,
            Preset::LetterTrifold,
            Preset::LetterPortrait,
            Preset::LegalBifold,
            Preset::TabloidBifold,
            Preset::HalfLetterBooklet,
        ]
    }
    pub fn sheet_size(&self) -> Size {
        match self {
            Preset::LetterBifold | Preset::LetterTrifold => Size::new(inch(11.0), inch(8.5)),
            Preset::LetterPortrait => Size::new(inch(8.5), inch(11.0)),
            Preset::LegalBifold => Size::new(inch(14.0), inch(8.5)),
            Preset::TabloidBifold => Size::new(inch(17.0), inch(11.0)),
            Preset::HalfLetterBooklet => Size::new(inch(11.0), inch(8.5)),
        }
    }
    pub fn panel_size(&self) -> Size {
        let s = self.sheet_size();
        match self {
            Preset::LetterBifold | Preset::LegalBifold | Preset::TabloidBifold => {
                Size::new(s.w / 2, s.h)
            }
            Preset::LetterTrifold => Size::new(s.w / 3, s.h),
            Preset::LetterPortrait => s,
            Preset::HalfLetterBooklet => Size::new(inch(5.5), inch(8.5)),
        }
    }
    pub fn panel_names(&self) -> Vec<String> {
        match self {
            Preset::LetterTrifold => [
                "Front Cover",
                "Inside Flap (Welcome)",
                "Inside Left",
                "Inside Middle",
                "Inside Right",
                "Back Cover",
            ]
            .iter()
            .map(|n| n.to_string())
            .collect(),
            Preset::LetterPortrait => [
                "Page 1 (Cover)",
                "Page 2 (Order of Service)",
                "Page 3 (Notes and Announcements)",
                "Page 4 (Back)",
            ]
            .iter()
            .map(|n| n.to_string())
            .collect(),
            Preset::HalfLetterBooklet => {
                let mut names = vec!["Cover".to_string(), "Inside Cover".to_string()];
                for page in 3..=15 {
                    names.push(format!("Page {page}"));
                }
                names.push("Back Cover".to_string());
                names
            }
            _ => [
                "Front Cover",
                "Inside Left (Order of Service)",
                "Inside Right (Notes and Announcements)",
                "Back Cover",
            ]
            .iter()
            .map(|n| n.to_string())
            .collect(),
        }
    }
}

fn para(style: &str, text: &str) -> Paragraph {
    Paragraph::new(style, text)
}

fn text_frame(id: &str, name: &str, panel: Size, story: &str, role: ContentRole) -> Frame {
    let m = inch(0.45);
    Frame {
        id: id.to_string(),
        name: name.to_string(),
        kind: FrameKind::Text,
        rect: Rect::new(m, inch(0.4), panel.w - 2 * m, panel.h - inch(0.8)),
        story_id: Some(story.to_string()),
        role,
        picture: None,
        music: None,
    }
}

/// Build a ready-to-edit bulletin document for a preset.
/// Standing content uses `{...}` field tokens so the weekly edit is mostly
/// changing field values, not hunting through panels.
pub fn build_template(
    preset: Preset,
    profile: ChurchProfile,
    sunday: NaiveDate,
    sunday_name: &str,
) -> Document {
    let panel = preset.panel_size();
    let names = preset.panel_names();
    let cover_idx = 0usize;
    let order_idx = if preset == Preset::LetterTrifold {
        2
    } else {
        1
    };
    let notes_idx = if preset == Preset::LetterTrifold {
        3
    } else {
        2
    };
    let back_idx = names.len() - 1;

    let mut stories: Vec<Story> = Vec::new();
    let mut panels: Vec<Panel> = Vec::new();

    for (i, name) in names.iter().enumerate() {
        let sid = format!("story-{i}");
        let (paragraphs, role) = if i == cover_idx {
            (
                vec![
                    para("church-name", "{Church.name}"),
                    para("subheading", "{Sunday.name}"),
                    para("body-center", "{Sunday.date}"),
                    para("body-center", "Welcome! We are glad you are here."),
                    para("caption", "Please silence mobile phones during worship."),
                ],
                ContentRole::Standing,
            )
        } else if i == order_idx {
            (
                vec![
                    para("subheading", "Order of Service"),
                    para("order-line", "Prelude\t"),
                    para("order-line", "Welcome and Announcements\t"),
                    para("order-line", "Call to Worship\t"),
                    para("order-line", "Hymn of Praise\t432"),
                    para("order-line", "Prayer of Confession\t"),
                    para("body", "Scripture Reading: {Sunday.reading}"),
                    para("body", "Sermon: {Church.pastor}"),
                    para("order-line", "Hymn of Response\t365"),
                    para("order-line", "Benediction\t"),
                    para("subheading", "Serving This Week"),
                    para("body", "Children's message: {Sunday.childrens_message}"),
                    para("body", "Acolytes: {Sunday.acolytes}"),
                ],
                ContentRole::Standing,
            )
        } else if i == notes_idx {
            (
                vec![
                    para("subheading", "Sermon Notes"),
                    para("notes-line", "________________________________________"),
                    para("notes-line", "________________________________________"),
                    para("notes-line", "________________________________________"),
                    para("subheading", "Announcements"),
                    para("body", "[Replace with this week's announcements.]"),
                ],
                ContentRole::Weekly,
            )
        } else if i == back_idx {
            (
                vec![
                    para("subheading", "Service Times"),
                    para("body", "{Church.times}"),
                    para("body", "{Church.address}"),
                    para("body", "{Church.website}"),
                    para("body", "Pastor: {Church.pastor}"),
                    para("body", "Give online: {Church.giving}"),
                ],
                ContentRole::Standing,
            )
        } else if preset == Preset::HalfLetterBooklet {
            // Booklet content pages: a heading and room for the
            // week's material. Weekly, so Next Sunday clears them.
            (
                vec![
                    para("subheading", name),
                    para("body", "[Text for this page.]"),
                ],
                ContentRole::Weekly,
            )
        } else {
            // Tri-fold welcome flap and extra inside panels.
            (
                vec![
                    para("subheading", "Welcome"),
                    para("body", "New here? Please stop by the welcome table after the service. We would love to meet you."),
                    para("body", "Children are welcome in worship; a nursery is available for ages 0-4."),
                ],
                ContentRole::Standing,
            )
        };
        stories.push(Story {
            id: sid.clone(),
            paragraphs,
            keep_through: None,
        });
        panels.push(Panel {
            id: format!("panel-{i}"),
            name: name.to_string(),
            size: panel,
            frames: vec![text_frame(&format!("frame-{i}"), name, panel, &sid, role)],
            background_image: None,
            guides: Vec::new(),
        });
    }

    let mut fields = BTreeMap::new();
    fields.insert(
        "occasion_date".to_string(),
        sunday.format("%Y-%m-%d").to_string(),
    );
    fields.insert("occasion_date_long".to_string(), long_date(sunday));
    fields.insert("occasion_name".to_string(), sunday_name.to_string());
    fields.insert("reading".to_string(), "Romans 8:18-25".to_string());

    Document {
        format_version: FORMAT_VERSION,
        title: format!("{} Bulletin", profile.name),
        profile,
        fields,
        styles: default_styles(),
        stories,
        panels,
        sheet: SheetSpec {
            preset: preset.id().to_string(),
            size: preset.sheet_size(),
        },
        no_announcements: false,
        assets: Vec::new(),
        asset_bytes: Default::default(),
        occasion_time: None,
        wanted_fonts: Vec::new(),
        participants: BTreeMap::new(),
    }
}

/// "October 18, 2026" style long date, portable across platforms.
pub fn long_date(d: NaiveDate) -> String {
    format!("{} {}, {}", d.format("%B"), d.day(), d.year())
}

/// What Next Sunday did to announcements that carry a
/// keep-through date, so the app can say it out loud.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NextSundayReport {
    /// Weekly stories whose keep-through date had passed by the
    /// new Sunday and whose words were cleared because of it.
    pub ended_announcements: usize,
    /// The first words of each announcement that ended, in the
    /// order they were ended, so the app can name what it
    /// cleared and not only count it (J10).
    pub ended_names: Vec<String>,
    /// Weekly stories whose keep-through date still covers the
    /// new Sunday, so their words carried over.
    pub kept_announcements: usize,
    /// How many weeks the date moved: 1 in the normal week, more
    /// when a stale file had to snap forward to the coming
    /// Sunday (see [`next_sunday_report_as_of`]).
    pub weeks_advanced: u32,
    /// The serving lines that held names and were
    /// cleared for the new week (Readers, Ushers, and
    /// friends), by their display names as the profile
    /// shows them, so the app can name what it cleared
    /// the same way it names ended announcements (J10).
    pub cleared_participants: Vec<String>,
    /// The hymn lines whose values were cleared for the
    /// new week (R-2a), by their labels, in document
    /// order: this week's numbers and titles are last
    /// week's news, and the report names the lines so the
    /// office hears which ones to fill in again. A hymn
    /// line that was already empty is not named.
    pub cleared_hymns: Vec<String>,
    /// Set when the document was a one-time occasion (a
    /// Wedding, an Ash Wednesday) rather than the regular
    /// weekly service: the sentence, in plain words, saying
    /// the copy was made for the coming regular Sunday
    /// instead and the occasion name was not carried
    /// forward (round 2, R2-11). None for a regular week,
    /// whose advance changes nothing but the date.
    pub occasion_note: Option<String>,
}

/// The name every fresh bulletin starts with: the church's
/// regular weekly service, before This Sunday gives the week
/// its own name ("Twentieth Sunday after Pentecost"). It is
/// what a one-time occasion's copy falls back to.
pub const DEFAULT_SERVICE_NAME: &str = "Sunday";

/// Is this service name a one-time occasion rather than the
/// church's regular weekly service? The occasions design
/// (docs/planning/occasions.md) names the regular rhythm by
/// its Sundays: the default name itself, or a Sunday name
/// like "Twentieth Sunday after Pentecost" or "Easter
/// Sunday". Any other name that names a service at all
/// (Wedding, Funeral, Ash Wednesday, Christmas Eve) is an
/// occasion that happens once. The document carries no
/// other marker: a regular service held on a weekday and
/// named without the word Sunday cannot be told apart from
/// a one-time occasion, which is why the guard announces
/// itself in the report and names the way back (rename it
/// on This Sunday) instead of acting silently.
fn is_one_time_occasion(name: &str) -> bool {
    let name = name.trim();
    if name.is_empty() || name.eq_ignore_ascii_case(DEFAULT_SERVICE_NAME) {
        return false;
    }
    !name
        .split(|c: char| !c.is_alphanumeric())
        .any(|w| w.eq_ignore_ascii_case("Sunday") || w.eq_ignore_ascii_case("Sundays"))
}

/// The signature command: clone the document for next Sunday.
/// - Advances the Sunday date by 7 days (long form too).
/// - A one-time occasion (a service whose name is not the
///   regular Sunday name: a Wedding, an Ash Wednesday) does
///   not repeat a week later: the copy is set for the coming
///   regular Sunday under the regular name, and the report
///   carries a note saying so (round 2, R2-11).
/// - Clears Body paragraphs in Weekly stories (announcements), keeping
///   headings and standing structure untouched.
/// - A weekly story with a keep-through date keeps its words
///   while that date is on or after the new Sunday; once the
///   date has passed, the story ends: cleared, and counted in
///   the report. A keep-through date that does not read as a
///   date counts as none ("this Sunday only").
/// - The no-announcements mark is reset: each week answers
///   for itself.
/// - Serving lines are cleared: the new week has new
///   people, and the report names the lines that held
///   names, in the profile's shown order.
/// - Hymn lines keep their labels and lose their values
///   (R-2a): the new week sings different hymns, and the
///   report names the lines that had a value.
///
/// Returns an error string if the stored date cannot be parsed.
pub fn next_sunday(doc: &Document) -> Result<Document, String> {
    Ok(next_sunday_report(doc)?.0)
}

/// The bulletin's own Sunday date, parsed.
fn doc_occasion_date(doc: &Document) -> Result<NaiveDate, String> {
    let iso = doc
        .fields
        .get("occasion_date")
        .ok_or("document has no occasion_date field")?;
    NaiveDate::parse_from_str(iso, "%Y-%m-%d").map_err(|e| format!("bad occasion_date {iso}: {e}"))
}

/// Where Next Sunday lands: one week on from the file's date,
/// then forward by whole weeks until the result is strictly
/// after `today`. A current file moves exactly one week; a
/// file left for three weeks cannot advance into a Sunday
/// that has already gone by. Returns the date and how many
/// weeks it moved.
fn advance_target(date: NaiveDate, today: NaiveDate) -> (NaiveDate, u32) {
    let mut next = date + Duration::days(7);
    let mut weeks = 1u32;
    while next <= today {
        next += Duration::days(7);
        weeks += 1;
    }
    (next, weeks)
}

/// [`next_sunday`] with the announcement bookkeeping attached.
/// File-date semantics, as this function has always had: the
/// target is measured against the bulletin's own date, so the
/// result is exactly one week on. Callers who know the real
/// today (the app) use [`next_sunday_report_as_of`] instead,
/// so a stale file snaps forward to the coming Sunday.
pub fn next_sunday_report(doc: &Document) -> Result<(Document, NextSundayReport), String> {
    let date = doc_occasion_date(doc)?;
    next_sunday_report_as_of(doc, date)
}

/// [`next_sunday_report`], measured against a real today: when
/// one week on from the file's date would land on or before
/// today, the date snaps forward by whole weeks to the first
/// Sunday strictly after today (the coming Sunday, since the
/// file's date is a Sunday). The report's `weeks_advanced`
/// says how far it moved, so the caller can say it out loud.
pub fn next_sunday_report_as_of(
    doc: &Document,
    today: NaiveDate,
) -> Result<(Document, NextSundayReport), String> {
    let mut out = doc.clone();
    let date = doc_occasion_date(doc)?;
    // A one-time occasion (R2-11): the copy is not "the same
    // service a week later" (there is no second Wedding next
    // Saturday); it is the coming regular Sunday's bulletin.
    // The target is the first Sunday after both the file's
    // date and today, and the occasion name stays behind.
    let old_name = doc
        .fields
        .get("occasion_name")
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    let one_time = is_one_time_occasion(&old_name);
    let (next, weeks_advanced) = if one_time {
        let base = date.max(today);
        let mut target = base + Duration::days(1);
        while target.weekday() != chrono::Weekday::Sun {
            target += Duration::days(1);
        }
        let weeks = (((target - date).num_days() as f32 / 7.0).ceil() as u32).max(1);
        (target, weeks)
    } else {
        advance_target(date, today)
    };
    // The supported calendar ends with the year 9999: a date
    // in its last week has no next week to move to, and the
    // advance used to roll into the year 10000 without a word
    // (audit Batch 3). Refuse in plain words instead.
    if next.year() > 9999 {
        return Err(
            "That service date is in the last week of the year 9999, so there is no \
             next week to move to. Set a date in an ordinary year on the This Sunday \
             step first."
                .into(),
        );
    }
    out.fields.insert(
        "occasion_date".to_string(),
        next.format("%Y-%m-%d").to_string(),
    );
    out.fields
        .insert("occasion_date_long".to_string(), long_date(next));
    // A one-time occasion's name does not travel (R2-11):
    // the copy is the regular Sunday's bulletin, so it takes
    // the regular name, and the report says what happened.
    let occasion_note = if one_time {
        out.fields.insert(
            "occasion_name".to_string(),
            DEFAULT_SERVICE_NAME.to_string(),
        );
        Some(format!(
            "This was a one-time service ({old_name}). The copy is set for \
             Sunday, {}; if it is a different service, rename it on This Sunday.",
            long_date(next)
        ))
    } else {
        None
    };
    out.no_announcements = false;

    // Find weekly story ids from frames marked Weekly. Two
    // frames can share one story; visit each story once so the
    // report counts announcements, not frames.
    let mut weekly: Vec<String> = Vec::new();
    for frame in out.panels.iter().flat_map(|p| p.frames.iter()) {
        if frame.role == ContentRole::Weekly {
            if let Some(sid) = &frame.story_id {
                if !weekly.contains(sid) {
                    weekly.push(sid.clone());
                }
            }
        }
    }
    let mut report = NextSundayReport {
        weeks_advanced,
        occasion_note,
        ..NextSundayReport::default()
    };
    // Participants are weekly people: the new week starts
    // with empty rows, and the report names the lines that
    // held names (J10's carried-list shape).
    report.cleared_participants = clear_participants(&mut out);
    // Hymn values are weekly too (R-2a): the number (and
    // title, when it was written in) belongs to the week
    // that sang it, and a standing template that kept last
    // week's number would print it as this week's. The
    // label stays; the value goes; the report names the
    // lines that had one. Any tabbed line whose label
    // names a hymn is a hymn line, wherever it sits.
    for story in out.stories.iter_mut() {
        for para in story.paragraphs.iter_mut() {
            let text = para.text();
            let Some(tab) = text.find('\t') else {
                continue;
            };
            let label = text[..tab].trim().to_string();
            if !ob_model::hymns::is_hymn_label(&label) {
                continue;
            }
            let value = text.rsplit('\t').next().unwrap_or("").trim();
            if value.is_empty() {
                continue;
            }
            para.set_text(&text[..=tab]);
            report.cleared_hymns.push(label);
        }
    }
    for sid in weekly {
        if let Some(story) = out.story_mut(&sid) {
            let keep = story
                .keep_through
                .as_deref()
                .and_then(|s| NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d").ok());
            match keep {
                Some(until) if until >= next => {
                    // Still running: the words carry over.
                    report.kept_announcements += 1;
                }
                Some(_) => {
                    // The date has passed: the announcement
                    // ends. Count it only if words were
                    // actually cleared, and drop the spent
                    // date so it cannot end twice.
                    let had_words = story
                        .paragraphs
                        .iter()
                        .any(|p| p.style == "body" && !p.text().trim().is_empty());
                    if had_words {
                        // Name it before the words go (J10).
                        report.ended_names.push(announcement_first_line(story));
                    }
                    clear_body(story);
                    story.keep_through = None;
                    if had_words {
                        report.ended_announcements += 1;
                    }
                }
                None => clear_body(story),
            }
        }
    }
    Ok((out, report))
}

/// The identifying first words of an announcement story
/// (J10): its first non-empty Body paragraph, first line
/// only, trimmed to about 40 characters. Falls back to the
/// first non-empty paragraph of any style, then a plain
/// label, so the caller always has something to name.
fn announcement_first_line(story: &Story) -> String {
    let first_text = |want_body: bool| {
        story
            .paragraphs
            .iter()
            .filter(|p| !want_body || p.style == "body")
            .map(|p| p.text())
            .find(|t| !t.trim().is_empty())
    };
    let owned = first_text(true).or_else(|| first_text(false));
    let text = owned.as_deref().unwrap_or("An announcement");
    text.lines()
        .next()
        .unwrap_or(text)
        .trim()
        .chars()
        .take(40)
        .collect()
}

/// Clear the week's serving names (roles build R-2b)
/// and name the lines that held names, in the order the
/// report speaks them: the profile's lines first, in
/// their shown order, under their shown names; then any
/// stored value whose line the profile no longer shows
/// (a renamed or removed line), named by its built-in
/// name when it has one, else by its slug in words; then
/// the legacy weekly fields of a document that never
/// went through the load migration. Whitespace-only
/// values held no name and are cleared silently, as
/// they always were.
fn clear_participants(doc: &mut Document) -> Vec<String> {
    let mut named: Vec<String> = Vec::new();
    let push_name = |named: &mut Vec<String>, name: String| {
        if !named.contains(&name) {
            named.push(name);
        }
    };
    // The profile's lines, in shown order. A value can
    // sit under the slug of the name as shown or, for a
    // built-in line, under its permanent slug: clear
    // both keys, name the line once.
    for name in doc.profile.serving_lines.clone() {
        let mut keys = vec![ob_model::serving_slug(&name)];
        if let Some(bslug) = ob_model::builtin_serving_slug(&name) {
            if !keys.contains(&bslug.to_string()) {
                keys.push(bslug.to_string());
            }
        }
        let mut held = false;
        for key in keys {
            if let Some(v) = doc.participants.remove(&key) {
                if !v.trim().is_empty() {
                    held = true;
                }
            }
        }
        if held {
            push_name(&mut named, name);
        }
    }
    // Whatever is left in the map belongs to a line the
    // profile no longer shows.
    let leftover: Vec<(String, String)> =
        std::mem::take(&mut doc.participants).into_iter().collect();
    for (key, v) in leftover {
        if v.trim().is_empty() {
            continue;
        }
        let name = ob_model::builtin_serving_name(&key)
            .map(|n| n.to_string())
            .unwrap_or_else(|| key.replace('_', " "));
        push_name(&mut named, name);
    }
    // The legacy weekly fields (readers and friends in
    // `fields`), for an in-memory document that never
    // went through the load migration.
    for (slug, name) in ob_model::PARTICIPANT_FIELDS {
        if let Some(v) = doc.fields.remove(slug) {
            if !v.trim().is_empty() {
                push_name(&mut named, name.to_string());
            }
        }
    }
    named
}

/// Empty a story's Body paragraphs, keeping headings and
/// standing structure untouched (the long-standing weekly
/// clearing rule).
fn clear_body(story: &mut Story) {
    for p in story.paragraphs.iter_mut() {
        if p.style == "body" {
            p.set_text("");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn booklet_template_is_sixteen_half_letter_pages() {
        let doc = build_template(
            Preset::HalfLetterBooklet,
            ChurchProfile::default(),
            NaiveDate::from_ymd_opt(2026, 10, 18).expect("date"),
            "Sunday",
        );
        assert_eq!(doc.panels.len(), 16);
        assert_eq!(doc.sheet.preset, "saddle-booklet");
        for panel in &doc.panels {
            assert_eq!(panel.size, Size::new(inch(5.5), inch(8.5)));
        }
        assert_eq!(doc.panels[0].name, "Cover");
        assert_eq!(doc.panels[15].name, "Back Cover");
        // Imposition pairs first and last page on the outer sheet.
        let sides = ob_impose::impose(&doc);
        assert_eq!(sides.len(), 8);
        assert_eq!(sides[0].size, Size::new(inch(11.0), inch(8.5)));
    }

    fn profile() -> ChurchProfile {
        ChurchProfile {
            name: "Grace Lutheran Church".into(),
            address: "123 Main St".into(),
            service_times: "Sundays 9:30 AM".into(),
            website: "grace.example.org".into(),
            giving_url: "grace.example.org/give".into(),
            pastor: "Pastor Lee".into(),
            expected_parts: Vec::new(),
            children_names: Default::default(),
            serving_lines: ob_model::default_serving_lines(),
        }
    }

    #[test]
    fn bifold_template_shape() {
        let d = build_template(
            Preset::LetterBifold,
            profile(),
            NaiveDate::from_ymd_opt(2026, 10, 18).unwrap(),
            "Twentieth Sunday after Pentecost",
        );
        assert_eq!(d.panels.len(), 4);
        assert_eq!(d.sheet.size, Size::new(inch(11.0), inch(8.5)));
        assert_eq!(d.panels[0].size, Size::new(inch(5.5), inch(8.5)));
        let cover = &d.stories[0];
        assert!(cover
            .paragraphs
            .iter()
            .any(|p| p.text().contains("{Sunday.name}")));
    }

    #[test]
    fn trifold_has_six_panels() {
        let d = build_template(
            Preset::LetterTrifold,
            profile(),
            NaiveDate::from_ymd_opt(2026, 10, 18).unwrap(),
            "X",
        );
        assert_eq!(d.panels.len(), 6);
    }

    #[test]
    fn next_sunday_advances_and_clears() {
        let d = build_template(
            Preset::LetterBifold,
            profile(),
            NaiveDate::from_ymd_opt(2026, 10, 18).unwrap(),
            "Sunday",
        );
        let n = next_sunday(&d).unwrap();
        assert_eq!(n.fields["occasion_date"], "2026-10-25");
        assert_eq!(n.fields["occasion_date_long"], "October 25, 2026");
        // Announcements (body in the weekly story) cleared; heading kept.
        let notes = &n.stories[2];
        assert!(notes
            .paragraphs
            .iter()
            .any(|p| p.style == "subheading" && p.text() == "Announcements"));
        assert!(notes
            .paragraphs
            .iter()
            .filter(|p| p.style == "body")
            .all(|p| p.text().is_empty()));
        // Standing order of service untouched.
        assert!(n.stories[1]
            .paragraphs
            .iter()
            .any(|p| p.text().contains("Hymn of Praise")));
    }

    #[test]
    fn next_sunday_clears_hymn_values_and_names_the_lines() {
        // R-2a: hymn numbers belong to the week that sang
        // them. The labels stay; the values go; the report
        // names the lines that had one.
        let mut d = build_template(
            Preset::LetterBifold,
            profile(),
            NaiveDate::from_ymd_opt(2026, 10, 18).unwrap(),
            "Sunday",
        );
        let order = &mut d.stories[1];
        // One hymn line starts empty, and a non-hymn line
        // carries a value of its own.
        for p in order.paragraphs.iter_mut() {
            if p.text().starts_with("Hymn of Response") {
                p.set_text("Hymn of Response\t");
            }
            if p.text().starts_with("Prelude") {
                p.set_text("Prelude\tOrgan voluntary");
            }
        }
        let (n, report) = next_sunday_report(&d).unwrap();
        assert_eq!(report.cleared_hymns, vec!["Hymn of Praise".to_string()]);
        let texts: Vec<String> = n.stories[1].paragraphs.iter().map(|p| p.text()).collect();
        assert!(texts.iter().any(|t| t == "Hymn of Praise\t"), "{texts:?}");
        assert!(texts.iter().any(|t| t == "Hymn of Response\t"), "{texts:?}");
        // A non-hymn line keeps its value: only hymn lines clear.
        assert!(
            texts.iter().any(|t| t == "Prelude\tOrgan voluntary"),
            "{texts:?}"
        );
    }

    #[test]
    fn next_sunday_refuses_to_leave_the_year_9999() {
        // Audit Batch 3: the last week of the supported
        // calendar has no next week; the advance used to
        // produce the year 10000 without a word.
        let d = build_template(
            Preset::LetterBifold,
            profile(),
            NaiveDate::from_ymd_opt(9999, 12, 25).unwrap(),
            "Sunday",
        );
        let err = next_sunday(&d).expect_err("year 10000 must be refused");
        assert!(err.contains("9999"), "plain-words refusal: {err}");
        // One week earlier still advances, landing on the
        // last supported day of the year.
        let d = build_template(
            Preset::LetterBifold,
            profile(),
            NaiveDate::from_ymd_opt(9999, 12, 18).unwrap(),
            "Sunday",
        );
        let n = next_sunday(&d).unwrap();
        assert_eq!(n.fields["occasion_date"], "9999-12-25");
    }

    #[test]
    fn next_sunday_honors_keep_through_dates() {
        // The template's weekly story is the notes/announcements
        // story (index 2): dated 2026-10-18, so Next Sunday
        // makes the 2026-10-25 bulletin.
        let mut d = build_template(
            Preset::LetterBifold,
            profile(),
            NaiveDate::from_ymd_opt(2026, 10, 18).unwrap(),
            "Sunday",
        );
        let body_text = |doc: &Document| {
            doc.stories[2]
                .paragraphs
                .iter()
                .filter(|p| p.style == "body")
                .map(|p| p.text())
                .collect::<Vec<_>>()
                .join(" ")
        };
        d.stories[2]
            .paragraphs
            .iter_mut()
            .filter(|p| p.style == "body")
            .for_each(|p| p.set_text("Food bank collection"));

        // Keep-through before the new Sunday: ended and cleared,
        // counted once, and the spent date is gone.
        d.stories[2].keep_through = Some("2026-10-24".into());
        let (n, report) = next_sunday_report(&d).unwrap();
        assert_eq!(report.ended_announcements, 1);
        // J10: the ended announcement is named by its first
        // words, not only counted.
        assert_eq!(report.ended_names, vec!["Food bank collection".to_string()]);
        assert_eq!(report.kept_announcements, 0);
        assert!(body_text(&n).trim().is_empty());
        assert_eq!(n.stories[2].keep_through, None);

        // Keep-through on the new Sunday itself: kept whole.
        d.stories[2].keep_through = Some("2026-10-25".into());
        let (n, report) = next_sunday_report(&d).unwrap();
        assert_eq!(report.ended_announcements, 0);
        assert_eq!(report.kept_announcements, 1);
        assert!(body_text(&n).contains("Food bank collection"));
        assert_eq!(n.stories[2].keep_through.as_deref(), Some("2026-10-25"));

        // Keep-through after the new Sunday: kept whole.
        d.stories[2].keep_through = Some("2026-11-08".into());
        let (n, report) = next_sunday_report(&d).unwrap();
        assert_eq!(report.kept_announcements, 1);
        assert!(body_text(&n).contains("Food bank collection"));

        // No keep-through: cleared as always, and not counted
        // as an announcement that ended (it never had a date).
        d.stories[2].keep_through = None;
        let (n, report) = next_sunday_report(&d).unwrap();
        assert_eq!(report.ended_announcements, 0);
        assert!(body_text(&n).trim().is_empty());

        // A date that does not read counts as none.
        d.stories[2].keep_through = Some("next month".into());
        let (n, report) = next_sunday_report(&d).unwrap();
        assert_eq!(report.ended_announcements, 0);
        assert!(body_text(&n).trim().is_empty());
    }

    #[test]
    fn next_sunday_as_of_snaps_a_stale_file_to_the_coming_sunday() {
        // J1: a file left alone for weeks must not advance into
        // a Sunday that has already gone by.
        let d = build_template(
            Preset::LetterBifold,
            profile(),
            NaiveDate::from_ymd_opt(2026, 10, 4).unwrap(),
            "Sunday",
        );
        // Today is Thursday 2026-10-15: +7 lands on 10-11, in
        // the past, so the target snaps to the coming Sunday.
        let (n, report) =
            next_sunday_report_as_of(&d, NaiveDate::from_ymd_opt(2026, 10, 15).unwrap()).unwrap();
        assert_eq!(n.fields["occasion_date"], "2026-10-18");
        assert_eq!(n.fields["occasion_date_long"], "October 18, 2026");
        assert_eq!(report.weeks_advanced, 2);

        // Today IS the +7 Sunday: strictly after today means
        // one more week.
        let (n, report) =
            next_sunday_report_as_of(&d, NaiveDate::from_ymd_opt(2026, 10, 11).unwrap()).unwrap();
        assert_eq!(n.fields["occasion_date"], "2026-10-18");
        assert_eq!(report.weeks_advanced, 2);

        // A current file moves exactly one week, as always.
        let (n, report) =
            next_sunday_report_as_of(&d, NaiveDate::from_ymd_opt(2026, 10, 4).unwrap()).unwrap();
        assert_eq!(n.fields["occasion_date"], "2026-10-11");
        assert_eq!(report.weeks_advanced, 1);

        // The plain wrapper keeps file-date semantics: one
        // week on, whatever the calendar says.
        let (n, report) = next_sunday_report(&d).unwrap();
        assert_eq!(n.fields["occasion_date"], "2026-10-11");
        assert_eq!(report.weeks_advanced, 1);
    }

    #[test]
    fn next_sunday_clears_participants_and_names_them() {
        let mut d = build_template(
            Preset::LetterBifold,
            profile(),
            NaiveDate::from_ymd_opt(2026, 10, 18).unwrap(),
            "Sunday",
        );
        // No participants set: nothing cleared, nothing named.
        let (n, report) = next_sunday_report(&d).unwrap();
        assert!(report.cleared_participants.is_empty());
        assert_eq!(resolve_fields("Readers: {Sunday.readers}", &n), "Readers: ");

        d.fields
            .insert("readers".to_string(), "Ana Ruiz".to_string());
        d.fields
            .insert("ushers".to_string(), "Tom and Bea".to_string());
        d.fields.insert("greeters".to_string(), "  ".to_string());
        let (n, report) = next_sunday_report(&d).unwrap();
        // Named in field order; a whitespace-only value held
        // no name and is not named.
        assert_eq!(
            report.cleared_participants,
            vec!["Readers".to_string(), "Ushers".to_string()]
        );
        for (key, _label) in ob_model::PARTICIPANT_FIELDS {
            assert!(
                !n.fields.contains_key(key),
                "{key} starts the new week empty"
            );
        }
    }

    #[test]
    fn next_sunday_clears_the_participants_map_and_names_profile_lines() {
        // R-2b: names live in the participants map now.
        // The report names the lines that held names by
        // their profile display names, in profile order,
        // including a line the church named itself.
        let mut prof = profile();
        prof.serving_lines = vec![
            "Readers".to_string(),
            "Candle lighters".to_string(),
            "Acolytes".to_string(),
        ];
        let mut d = build_template(
            Preset::LetterBifold,
            prof,
            NaiveDate::from_ymd_opt(2026, 10, 18).unwrap(),
            "Sunday",
        );
        d.set_participant_value("Acolytes", "Sam, Jo");
        d.set_participant_value("Candle lighters", "Rae");
        d.set_participant_value("Readers", "   ");
        let (n, report) = next_sunday_report(&d).unwrap();
        assert!(n.participants.is_empty(), "the new week starts empty");
        // Profile order; the whitespace-only Readers line
        // held no name and is not named.
        assert_eq!(
            report.cleared_participants,
            vec!["Candle lighters".to_string(), "Acolytes".to_string()]
        );
        assert_eq!(
            resolve_fields("Acolytes: {Sunday.acolytes}", &n),
            "Acolytes: "
        );
    }

    #[test]
    fn starter_template_carries_the_new_serving_lines() {
        // R-2b item 5: a fresh church sees the two new
        // built-in serving lines in the template's
        // Serving section, tokens and all.
        let d = build_template(
            Preset::LetterBifold,
            profile(),
            NaiveDate::from_ymd_opt(2026, 10, 18).unwrap(),
            "Sunday",
        );
        let order = &d.stories[1];
        assert!(order
            .paragraphs
            .iter()
            .any(|p| p.text() == "Serving This Week"));
        assert!(order
            .paragraphs
            .iter()
            .any(|p| p.text() == "Children's message: {Sunday.childrens_message}"));
        assert!(order
            .paragraphs
            .iter()
            .any(|p| p.text() == "Acolytes: {Sunday.acolytes}"));
    }

    #[test]
    fn next_sunday_resets_the_no_announcements_mark() {
        let mut d = build_template(
            Preset::LetterBifold,
            profile(),
            NaiveDate::from_ymd_opt(2026, 10, 18).unwrap(),
            "Sunday",
        );
        d.no_announcements = true;
        let n = next_sunday(&d).unwrap();
        assert!(!n.no_announcements, "each week answers for itself");
    }

    #[test]
    fn next_sunday_from_a_one_time_occasion_makes_the_regular_sunday() {
        // A Wedding on Saturday 2026-10-17 (R2-11): there is
        // no second wedding a week later. The copy is the
        // coming regular Sunday's bulletin, under the regular
        // name, and the report says so in plain words.
        let d = build_template(
            Preset::LetterBifold,
            profile(),
            NaiveDate::from_ymd_opt(2026, 10, 17).unwrap(),
            "Wedding",
        );
        let (n, report) =
            next_sunday_report_as_of(&d, NaiveDate::from_ymd_opt(2026, 10, 16).unwrap()).unwrap();
        assert_eq!(n.fields["occasion_date"], "2026-10-18");
        assert_eq!(n.fields["occasion_date_long"], "October 18, 2026");
        assert_eq!(n.fields["occasion_name"], DEFAULT_SERVICE_NAME);
        let note = report.occasion_note.expect("the guard speaks");
        assert_eq!(
            note,
            "This was a one-time service (Wedding). The copy is set for \
             Sunday, October 18, 2026; if it is a different service, rename \
             it on This Sunday."
        );
        // The weekly machinery still ran on the copy.
        assert!(n.stories[2]
            .paragraphs
            .iter()
            .filter(|p| p.style == "body")
            .all(|p| p.text().is_empty()));
    }

    #[test]
    fn next_sunday_from_a_stale_occasion_lands_on_the_coming_sunday() {
        // An Ash Wednesday file (Wednesday 2026-02-18) picked
        // up again the following Monday: the copy is for the
        // coming Sunday after today, not the Sunday after the
        // service and not another Wednesday.
        let d = build_template(
            Preset::LetterBifold,
            profile(),
            NaiveDate::from_ymd_opt(2026, 2, 18).unwrap(),
            "Ash Wednesday",
        );
        let (n, report) =
            next_sunday_report_as_of(&d, NaiveDate::from_ymd_opt(2026, 2, 23).unwrap()).unwrap();
        assert_eq!(n.fields["occasion_date"], "2026-03-01");
        assert_eq!(n.fields["occasion_name"], DEFAULT_SERVICE_NAME);
        assert!(report.occasion_note.is_some());
    }

    #[test]
    fn next_sunday_keeps_a_regular_sundays_own_name() {
        // A liturgical Sunday name is the regular rhythm, not
        // an occasion: it advances a week and keeps its name,
        // and the report carries no occasion note.
        let d = build_template(
            Preset::LetterBifold,
            profile(),
            NaiveDate::from_ymd_opt(2026, 10, 18).unwrap(),
            "Twentieth Sunday after Pentecost",
        );
        let (n, report) = next_sunday_report(&d).unwrap();
        assert_eq!(n.fields["occasion_date"], "2026-10-25");
        assert_eq!(
            n.fields["occasion_name"],
            "Twentieth Sunday after Pentecost"
        );
        assert!(report.occasion_note.is_none());
        assert_eq!(report.weeks_advanced, 1);
    }
}
