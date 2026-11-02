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
//! below. Wiring them into the Start step's picker is a
//! separate UI step; the set itself ships here.

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

/// Build a ready-to-edit bulletin document for a seasonal
/// or event service: the preset's panels, cover, notes,
/// and back cover exactly as [`crate::build_template`]
/// builds them, with the order-of-service story replaced
/// by the occasion's order and the document titled for
/// the occasion.
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
