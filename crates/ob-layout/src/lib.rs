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
    /// The participant fields that held names and were
    /// cleared for the new week (Readers, Ushers, and
    /// friends), by their This Sunday labels, in field
    /// order, so the app can name what it cleared the same
    /// way it names ended announcements (J10).
    pub cleared_participants: Vec<String>,
}

/// The signature command: clone the document for next Sunday.
/// - Advances the Sunday date by 7 days (long form too).
/// - Clears Body paragraphs in Weekly stories (announcements), keeping
///   headings and standing structure untouched.
/// - A weekly story with a keep-through date keeps its words
///   while that date is on or after the new Sunday; once the
///   date has passed, the story ends: cleared, and counted in
///   the report. A keep-through date that does not read as a
///   date counts as none ("this Sunday only").
/// - The no-announcements mark is reset: each week answers
///   for itself.
/// - Participant fields (readers, ushers, greeters,
///   communion assistants) are cleared: the new week has
///   new people, and the report names the fields that
///   held names.
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
    let (next, weeks_advanced) = advance_target(date, today);
    out.fields.insert(
        "occasion_date".to_string(),
        next.format("%Y-%m-%d").to_string(),
    );
    out.fields
        .insert("occasion_date_long".to_string(), long_date(next));
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
        ..NextSundayReport::default()
    };
    // Participants are weekly people: the new week starts
    // with empty rows, and the report names the fields that
    // held names, in field order (J10's carried-list shape).
    for (key, label, _token) in ob_model::PARTICIPANT_FIELDS {
        if let Some(v) = out.fields.remove(key) {
            if !v.trim().is_empty() {
                report.cleared_participants.push(label.to_string());
            }
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
            "X",
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
    fn next_sunday_honors_keep_through_dates() {
        // The template's weekly story is the notes/announcements
        // story (index 2): dated 2026-10-18, so Next Sunday
        // makes the 2026-10-25 bulletin.
        let mut d = build_template(
            Preset::LetterBifold,
            profile(),
            NaiveDate::from_ymd_opt(2026, 10, 18).unwrap(),
            "X",
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
            "X",
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
            "X",
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
        for (key, _label, _token) in ob_model::PARTICIPANT_FIELDS {
            assert!(
                !n.fields.contains_key(key),
                "{key} starts the new week empty"
            );
        }
    }

    #[test]
    fn next_sunday_resets_the_no_announcements_mark() {
        let mut d = build_template(
            Preset::LetterBifold,
            profile(),
            NaiveDate::from_ymd_opt(2026, 10, 18).unwrap(),
            "X",
        );
        d.no_announcements = true;
        let n = next_sunday(&d).unwrap();
        assert!(!n.no_announcements, "each week answers for itself");
    }
}
