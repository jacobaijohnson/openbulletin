//! Seasonal and event templates (round 51, G-08a-seasonal).
//!
//! The Template Analyzer learns one family of look-alike
//! bulletins and does not identify seasonal or special
//! services (docs/TEMPLATE-ANALYZER.md). This set is the
//! other half: six shipped templates for the services a
//! church does not hold every Sunday, built on the same
//! [`crate::build_template`] every built-in preset uses,
//! with the order of service replaced by the occasion's
//! own order. They are first-class templates — a caller
//! gets a ready-to-edit [`ob_model::Document`], not a
//! name — and each one is pinned by a generation test
//! below. The Start step's picker offers them (round 53),
//! and each template files a [`SEASONAL_SERVICE_FIELD`]
//! marker the Next Sunday command honors by refusing,
//! with guidance, rather than mangling the occasion.

use crate::{build_template, Preset};
use chrono::NaiveDate;
use ob_model::{ChurchProfile, Document, Paragraph};

/// One seasonal or event service with a shipped template.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeasonalService {
    Wedding,
    Baptism,
    Christmas,
    Easter,
    Pentecost,
    Funeral,
}

impl SeasonalService {
    /// Every service in the set, in the order the docs list them.
    pub fn all() -> [SeasonalService; 6] {
        [
            SeasonalService::Wedding,
            SeasonalService::Baptism,
            SeasonalService::Christmas,
            SeasonalService::Easter,
            SeasonalService::Pentecost,
            SeasonalService::Funeral,
        ]
    }

    /// The occasion name the template files under: it lands
    /// in the document's `occasion_name` field, so the cover's
    /// `{Sunday.name}` token prints it.
    pub fn label(&self) -> &'static str {
        match self {
            SeasonalService::Wedding => "Wedding",
            SeasonalService::Baptism => "Baptism",
            SeasonalService::Christmas => "Christmas Day",
            SeasonalService::Easter => "Easter Sunday",
            SeasonalService::Pentecost => "Day of Pentecost",
            SeasonalService::Funeral => "Funeral",
        }
    }

    /// The occasion's order of service, as (label, value)
    /// pairs in the order-of-service story's tabbed shape.
    /// A blank value is a slot the week fills (a hymn number
    /// not yet chosen, a reading not yet set); a filled value
    /// is the occasion's own standing content. Named hymns
    /// and readings are the widely shared ones for the day,
    /// editable like any other words.
    fn order(&self) -> Vec<(&'static str, &'static str)> {
        match self {
            SeasonalService::Wedding => vec![
                ("Prelude", ""),
                ("Processional", ""),
                ("Welcome and Opening Prayer", ""),
                ("Declaration of Consent", ""),
                ("Exchange of Vows", ""),
                ("Exchange of Rings", ""),
                ("Hymn", ""),
                ("Scripture Reading", ""),
                ("Homily", ""),
                ("Prayers", ""),
                ("Pronouncement", ""),
                ("Recessional", ""),
            ],
            SeasonalService::Baptism => vec![
                ("Prelude", ""),
                ("Welcome", ""),
                ("Presentation of the Candidates", ""),
                ("Hymn", ""),
                ("Thanksgiving over the Water", ""),
                ("Profession of Faith", ""),
                ("Baptism", ""),
                ("Welcome into the Church", ""),
                ("Prayers", ""),
                ("Blessing", ""),
            ],
            SeasonalService::Christmas => vec![
                ("Prelude", ""),
                ("Welcome", ""),
                ("Call to Worship", ""),
                ("Carol", "Silent Night"),
                ("Scripture Reading", "Luke 2:1-20"),
                ("Carol", "Hark! The Herald Angels Sing"),
                ("Homily", ""),
                ("Prayers", ""),
                ("Lighting of the Christ Candle", ""),
                ("Benediction", ""),
            ],
            SeasonalService::Easter => vec![
                ("Prelude", ""),
                ("Welcome", ""),
                ("Call to Worship", ""),
                ("Hymn", "Christ the Lord Is Risen Today"),
                ("Scripture Reading", ""),
                ("Sermon", ""),
                ("Renewal of Baptismal Vows", ""),
                ("Prayers", ""),
                ("Hymn", ""),
                ("Benediction", ""),
            ],
            SeasonalService::Pentecost => vec![
                ("Prelude", ""),
                ("Welcome", ""),
                ("Call to Worship", ""),
                ("Hymn", ""),
                ("Scripture Reading", "Acts 2:1-21"),
                ("Sermon", ""),
                ("Prayers", ""),
                ("Benediction", ""),
            ],
            SeasonalService::Funeral => vec![
                ("Prelude", ""),
                ("Welcome and Opening Prayer", ""),
                ("Scripture Reading", ""),
                ("Remembrance", ""),
                ("Homily", ""),
                ("Prayers", ""),
                ("Hymn", ""),
                ("Commendation", ""),
                ("Benediction", ""),
                ("Recessional", ""),
            ],
        }
    }
}

/// The document field that marks a bulletin as a seasonal
/// service's own bulletin (round 53, R53-RED-2): the value
/// is the service label ([`SeasonalService::label`]). The
/// field travels with the document — it is part of the
/// saved form, so a re-opened seasonal bulletin still
/// refuses Next Sunday. Next Sunday checks this marker before the
/// name test, so Easter Sunday (which contains "Sunday")
/// refuses exactly like the other five. How a rename
/// treats the marker is [`marker_after_rename`]'s rule
/// (round 55, R54-RED-1), not an unconditional clear.
pub const SEASONAL_SERVICE_FIELD: &str = "seasonal_service";

/// The seasonal marker after a rename on This Sunday
/// (round 55, R54-RED-1). Round 53 cleared the marker on
/// every rename, on the theory that renaming asserts a
/// Sunday bulletin; that theory is false for
/// occasion-to-occasion renames — a Christmas bulletin
/// renamed to Christmas Eve kept the Christmas order and
/// lost its protection, so Next Sunday mangled it in the
/// exact round-53 shape — and it never covered the
/// reverse direction, a Sunday bulletin renamed to an
/// occasion name gaining no marker at all. The rule now:
/// - rename exactly to Sunday: the marker clears. This is
///   the designed way back to a Sunday bulletin;
/// - rename a marked bulletin to anything else: the marker
///   stays, re-filed under the new name, so the refusal
///   names the occasion as it now stands and the
///   seasonal order still refuses Next Sunday;
/// - rename an unmarked bulletin to a seasonal label or
///   any other one-time occasion name: the marker files
///   under the new name, so the bulletin refuses Next
///   Sunday until it is renamed back to Sunday. This is
///   deliberately fail-safe: a Sunday bulletin renamed to
///   an occasion name refuses even before the office has
///   edited its order to match, rather than advancing a
///   possibly-seasonal order as if it were a Sunday one;
/// - rename an unmarked bulletin to a Sunday-rhythm name
///   (a name carrying the word Sunday, or an empty name):
///   no marker files, and Next Sunday advances as before.
///
/// An empty new name on a marked bulletin keeps the old
/// marker rather than filing an empty one (Next Sunday
/// ignores an empty marker).
pub fn marker_after_rename(existing: Option<&str>, new_name: &str) -> Option<String> {
    let name = new_name.trim();
    if name.eq_ignore_ascii_case(crate::DEFAULT_SERVICE_NAME) {
        return None;
    }
    if name.is_empty() {
        return existing
            .filter(|s| !s.trim().is_empty())
            .map(|s| s.to_string());
    }
    let marked = existing.map(|s| !s.trim().is_empty()).unwrap_or(false);
    if marked {
        return Some(name.to_string());
    }
    let seasonal_label = SeasonalService::all()
        .iter()
        .any(|s| s.label().eq_ignore_ascii_case(name));
    if seasonal_label || crate::is_one_time_occasion(name) {
        Some(name.to_string())
    } else {
        None
    }
}

/// Whether a shape rebuild may carry the old occasion
/// name over a fresh Sunday template (round 55,
/// R54-RED-4). Round 53 exempted the occasion name on
/// the seasonal-pick path, but the shape path still
/// carried it: a Christmas bulletin rebuilt as a new
/// shape printed "Christmas Day" over the Sunday order
/// with the marker dropped, because the fresh template
/// files no marker. Name and marker must travel
/// together, so on a shape rebuild the name carries
/// only when it needs no marker at all: an unmarked
/// bulletin whose name is a Sunday-rhythm name. A
/// marked bulletin never carries its name (its order
/// is being replaced, so the fresh "Sunday" stands and
/// no marker files); neither does an unmarked bulletin
/// whose name is a shipped seasonal label or any other
/// one-time occasion name, since that name over the
/// Sunday order is the same split with no marker to
/// lose. An empty old name never carries either, so it
/// cannot blank the fresh template's "Sunday".
pub fn shape_rebuild_carries_occasion_name(existing: Option<&str>, old_name: &str) -> bool {
    let marked = existing.map(|s| !s.trim().is_empty()).unwrap_or(false);
    if marked {
        return false;
    }
    let name = old_name.trim();
    if name.is_empty() {
        return false;
    }
    if SeasonalService::all()
        .iter()
        .any(|s| s.label().eq_ignore_ascii_case(name))
    {
        return false;
    }
    !crate::is_one_time_occasion(name)
}

/// Build a ready-to-edit bulletin document for a seasonal
/// or event service: the preset's panels, cover, notes,
/// and back cover exactly as [`crate::build_template`]
/// builds them, with the order-of-service story replaced
/// by the occasion's order, the document titled for
/// the occasion, and the [`SEASONAL_SERVICE_FIELD`] marker
/// filed under the occasion label.
pub fn build_seasonal_template(
    service: SeasonalService,
    preset: Preset,
    profile: ChurchProfile,
    date: NaiveDate,
) -> Document {
    let label = service.label();
    let mut doc = build_template(preset, profile, date, label);
    let order_paragraphs = {
        let mut paragraphs = vec![Paragraph::new("subheading", "Order of Service")];
        for (name, value) in service.order() {
            let line = format!("{name}\t{value}");
            paragraphs.push(Paragraph::new("order-line", &line));
        }
        paragraphs
    };
    let order_story = doc.stories.iter_mut().find(|story| {
        story
            .paragraphs
            .first()
            .map(|p| p.text() == "Order of Service")
            .unwrap_or(false)
    });
    match order_story {
        Some(story) => {
            story.paragraphs = order_paragraphs;
        }
        None => {
            // build_template always carries an order story;
            // reaching here means its shape changed under
            // us. Leave the Sunday order standing rather
            // than inventing a second one.
        }
    }
    let church_name = doc.profile.name.clone();
    let title = format!("{church_name} {label}");
    doc.title = title;
    doc.fields
        .insert(SEASONAL_SERVICE_FIELD.to_string(), label.to_string());
    doc
}

#[cfg(test)]
mod tests {
    use super::*;
    use ob_model::ChurchProfile;

    fn profile() -> ChurchProfile {
        ChurchProfile {
            name: "Grace Church".to_string(),
            ..ChurchProfile::default()
        }
    }

    fn date() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 12, 25).unwrap()
    }

    /// The signature line each order must carry: the
    /// line no other service's order has, proving the
    /// template is the occasion's own and not the
    /// Sunday order with a new name.
    fn signature(service: SeasonalService) -> &'static str {
        match service {
            SeasonalService::Wedding => "Exchange of Vows",
            SeasonalService::Baptism => "Thanksgiving over the Water",
            SeasonalService::Christmas => "Silent Night",
            SeasonalService::Easter => "Christ the Lord Is Risen Today",
            SeasonalService::Pentecost => "Acts 2:1-21",
            SeasonalService::Funeral => "Commendation",
        }
    }

    fn order_text(doc: &Document) -> String {
        let story = doc
            .stories
            .iter()
            .find(|story| {
                story
                    .paragraphs
                    .first()
                    .map(|p| p.text() == "Order of Service")
                    .unwrap_or(false)
            })
            .expect("the seasonal template carries an order story");
        story
            .paragraphs
            .iter()
            .map(|p| p.text())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn every_seasonal_service_generates_its_own_order() {
        for service in SeasonalService::all() {
            let doc = build_seasonal_template(service, Preset::LetterBifold, profile(), date());
            assert_eq!(
                doc.fields["occasion_name"],
                service.label(),
                "occasion filed under its own name"
            );
            assert!(
                doc.title.contains(service.label()),
                "title names the occasion: {}",
                doc.title
            );
            assert_eq!(doc.panels.len(), 4, "the preset's panels survive");
            let order = order_text(&doc);
            assert!(
                order.contains(signature(service)),
                "the {label} order carries its signature line:\n{order}",
                label = service.label()
            );
            assert!(
                !order.contains("Hymn of Praise"),
                "the Sunday order did not survive under a seasonal name"
            );
        }
    }

    #[test]
    fn seasonal_templates_keep_the_preset_around_the_order() {
        let doc = build_seasonal_template(
            SeasonalService::Christmas,
            Preset::LetterBifold,
            profile(),
            date(),
        );
        let cover = &doc.stories[0];
        assert!(
            cover.paragraphs.iter().any(|p| p.text() == "{Church.name}"),
            "the cover still prints the church name token"
        );
        assert!(
            cover.paragraphs.iter().any(|p| p.text() == "{Sunday.name}"),
            "the cover still prints the occasion token"
        );
        let back = doc.stories.last().expect("a back story");
        assert!(
            back.paragraphs.iter().any(|p| p.text() == "{Church.times}"),
            "the back cover still prints service times"
        );
    }

    #[test]
    fn seasonal_templates_generate_on_every_preset() {
        // Round 53 trifold pin (R53-RED-2 coverage note):
        // only LetterBifold and HalfLetterBooklet were
        // pinned before, and the order-story fallback would
        // silently leave the Sunday order standing under a
        // seasonal name if build_template ever stopped
        // carrying an order story. Every preset now pins
        // every service's own order, name, and marker.
        for preset in Preset::all() {
            for service in SeasonalService::all() {
                let doc = build_seasonal_template(service, preset, profile(), date());
                assert_eq!(
                    doc.fields["occasion_name"],
                    service.label(),
                    "occasion filed under its own name on {:?}",
                    preset
                );
                assert_eq!(
                    doc.fields[SEASONAL_SERVICE_FIELD],
                    service.label(),
                    "seasonal marker filed on {:?}",
                    preset
                );
                assert!(
                    doc.title.contains(service.label()),
                    "title names the occasion on {:?}: {}",
                    preset,
                    doc.title
                );
                let order = order_text(&doc);
                assert!(
                    order.contains(signature(service)),
                    "the {label} order carries its signature line on {preset:?}:\n{order}",
                    label = service.label()
                );
                assert!(
                    !order.contains("Hymn of Praise"),
                    "the Sunday order did not survive under a seasonal name on {preset:?}",
                );
            }
        }
    }

    #[test]
    fn seasonal_marker_survives_save_and_load() {
        // The marker must survive the real saved form
        // (ob-format's .obd), not just the in-memory
        // document: a re-opened Christmas bulletin still
        // refuses Next Sunday.
        let dir = std::env::temp_dir().join(format!("ob-seasonal-marker-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("christmas.obd");
        let doc = build_seasonal_template(
            SeasonalService::Christmas,
            Preset::LetterBifold,
            profile(),
            date(),
        );
        ob_format::save(&doc, &path).expect("save");
        let back = ob_format::load(&path).expect("load");
        assert_eq!(back.fields[SEASONAL_SERVICE_FIELD], "Christmas Day");
        assert_eq!(back.fields["occasion_name"], "Christmas Day");
        let order = order_text(&back);
        assert!(order.contains("Silent Night"), "{order}");
    }

    #[test]
    fn rename_marker_rule_covers_both_directions() {
        // Round 55, R54-RED-1: the marker_after_rename
        // matrix, pinned at the rule itself so every
        // rename path through it is covered, not just the
        // app seam's three shapes.
        // Exactly Sunday clears, even from a marked doc.
        assert_eq!(marker_after_rename(Some("Christmas Day"), "Sunday"), None);
        assert_eq!(marker_after_rename(Some("Christmas Day"), " sunday "), None);
        assert_eq!(marker_after_rename(None, "Sunday"), None);
        // A marked doc renamed to another occasion keeps
        // the mark, re-filed under the new name — the S3
        // shape (Christmas Eve is not a shipped label, so
        // the one-time-name arm, not the label arm, files
        // it for an unmarked doc; the sticky arm covers
        // the marked doc either way).
        assert_eq!(
            marker_after_rename(Some("Christmas Day"), "Christmas Eve"),
            Some("Christmas Eve".to_string())
        );
        // A marked doc renamed to a Sunday-rhythm name
        // still keeps the mark: its order is still the
        // seasonal order, and only a rename exactly to
        // Sunday is the way back.
        assert_eq!(
            marker_after_rename(Some("Christmas Day"), "Twentieth Sunday after Pentecost"),
            Some("Twentieth Sunday after Pentecost".to_string())
        );
        // A marked doc renamed to a seasonal label keeps
        // the mark under the new label (Easter Sunday
        // contains "Sunday" but is a shipped label, so it
        // must not clear through the Sunday-word path).
        assert_eq!(
            marker_after_rename(Some("Christmas Day"), "Easter Sunday"),
            Some("Easter Sunday".to_string())
        );
        // An unmarked doc renamed to an occasion name
        // gains the mark: the reverse direction.
        assert_eq!(
            marker_after_rename(None, "Christmas Eve"),
            Some("Christmas Eve".to_string())
        );
        assert_eq!(
            marker_after_rename(None, "Easter Sunday"),
            Some("Easter Sunday".to_string())
        );
        assert_eq!(
            marker_after_rename(None, "Ash Wednesday"),
            Some("Ash Wednesday".to_string())
        );
        // An unmarked doc renamed to a Sunday-rhythm name
        // gains nothing and advances as before.
        assert_eq!(
            marker_after_rename(None, "Twentieth Sunday after Pentecost"),
            None
        );
        // An empty rename keeps an existing mark rather
        // than filing an empty one Next Sunday ignores.
        assert_eq!(
            marker_after_rename(Some("Christmas Day"), "   "),
            Some("Christmas Day".to_string())
        );
        assert_eq!(marker_after_rename(None, "   "), None);
    }

    #[test]
    fn shape_rebuild_occasion_name_rule() {
        // Round 55, R54-RED-4: the shape-rebuild rule
        // at the rule itself, so every shape path
        // through it is covered, not just the app
        // seam's Christmas shape.
        // A marked bulletin never carries its name,
        // whatever the name now stands as: the order
        // is being replaced, so the fresh "Sunday"
        // stands and no marker files.
        assert!(!shape_rebuild_carries_occasion_name(
            Some("Christmas Day"),
            "Christmas Day"
        ));
        assert!(!shape_rebuild_carries_occasion_name(
            Some("Christmas Eve"),
            "Twentieth Sunday after Pentecost"
        ));
        assert!(!shape_rebuild_carries_occasion_name(
            Some("Christmas Day"),
            "Sunday"
        ));
        // An unmarked bulletin whose name is a shipped
        // seasonal label or a one-time occasion name
        // does not carry it either: that name over the
        // Sunday order is the same split, with no
        // marker to lose. Easter Sunday counts even
        // though it contains "Sunday", because it is a
        // shipped label.
        assert!(!shape_rebuild_carries_occasion_name(None, "Christmas Day"));
        assert!(!shape_rebuild_carries_occasion_name(None, "Easter Sunday"));
        assert!(!shape_rebuild_carries_occasion_name(None, "Ash Wednesday"));
        assert!(!shape_rebuild_carries_occasion_name(None, "Christmas Eve"));
        // An empty old name never carries, so it
        // cannot blank the fresh template's "Sunday".
        assert!(!shape_rebuild_carries_occasion_name(None, "   "));
        // A Sunday-rhythm name on an unmarked bulletin
        // carries: it needs no marker, and the Sunday
        // order it lands on is its own kind. An empty
        // marker value counts as unmarked.
        assert!(shape_rebuild_carries_occasion_name(None, "Sunday"));
        assert!(shape_rebuild_carries_occasion_name(
            None,
            "Twentieth Sunday after Pentecost"
        ));
        assert!(shape_rebuild_carries_occasion_name(
            Some(""),
            "Twentieth Sunday after Pentecost"
        ));
    }

    #[test]
    fn seasonal_templates_generate_on_the_booklet_preset_too() {
        let doc = build_seasonal_template(
            SeasonalService::Funeral,
            Preset::HalfLetterBooklet,
            profile(),
            date(),
        );
        assert_eq!(doc.panels.len(), 16, "the booklet's pages survive");
        let order = order_text(&doc);
        assert!(order.contains("Commendation"), "{order}");
        assert_eq!(doc.fields["occasion_name"], "Funeral");
    }
}
