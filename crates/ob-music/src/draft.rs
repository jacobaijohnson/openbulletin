//! The draft a music import leaves: a heading with the
//! file's own words (title, composer; Verovio draws no
//! credits, the spike's named loss, so we print them as
//! text ourselves), one picture box per engraved line of
//! music flowing across the booklet's panels, and the
//! file's rights line as the credit under the music.
//!
//! Music boxes are laid out at the panel's full text
//! width and the height the engraving's own proportions
//! give; they are never squeezed to fit a space, because
//! their size IS the legibility contract (the staff was
//! measured at that width). When the engraver is not on
//! the computer the draft still forms, with one empty
//! music box standing where the notes will be drawn and
//! a plain Check-this saying why.

use crate::parse::MusicInfo;
use crate::render::{RenderOutcome, FLOOR_STAFF_MM, TARGET_STAFF_MM};
use ob_extract::draft::{Badge, DraftOutcome, ImportItem};
use ob_model::{
    default_styles, AssetKind, ChurchProfile, ContentRole, Document, Frame, FrameKind, LicenseKind,
    MusicFrame, Panel, Paragraph, PictureFit, PicturePlacement, Rect, RightsRecord, SheetSpec,
    Size, Story, FORMAT_VERSION,
};

/// The draft's panel: the half-letter booklet panel the
/// corpus church uses, one page of the folded bulletin.
pub const PANEL_W: i64 = 396_000;
pub const PANEL_H: i64 = 612_000;
/// Breathing room at the panel edges.
pub const MARGIN: i64 = 14_400;
/// The gap between the heading and the music, and between
/// lines of music.
const GAP: i64 = 16_000;
/// The heading box's height: title, a subtitle or
/// composer line or two, at bulletin sizes.
const HEADING_H: i64 = 60_000;
/// The credit line's height under the music.
const CREDIT_H: i64 = 34_000;

/// The width every music box is drawn at: the panel's
/// full text width. The engraver is asked for exactly
/// this width, so the staff size it measures is the
/// staff size that prints.
pub fn music_frame_width_mpt() -> i64 {
    PANEL_W - 2 * MARGIN
}

pub fn music_frame_width_mm() -> f64 {
    ob_model::mpt_to_inches(music_frame_width_mpt()) * 25.4
}

/// The rights record for an asset that came from a music
/// file: the file is the source, and its rights line,
/// when it parses to a known kind, is the license.
pub fn rights_from_music(info: &MusicInfo, source_name: &str) -> RightsRecord {
    RightsRecord {
        source: source_name.to_string(),
        source_url: None,
        license: license_from_rights(info.rights.as_deref()),
        title: info.title.clone().unwrap_or_default(),
        creator: info.composer().unwrap_or_default().to_string(),
        copyright_line: info.rights.clone().unwrap_or_default(),
        credit_text: info.credit_line().unwrap_or_default(),
        licensor_ref: String::new(),
        asserted_by: String::new(),
        asserted_at: String::new(),
        children_present: None,
    }
}

/// A conservative read of a rights line into a license
/// kind: a line that clearly says public domain, CC0,
/// CC BY(-SA), or names a church reprint license is taken
/// at its word; anything else stays Unknown rather than
/// guessed.
pub fn license_from_rights(rights: Option<&str>) -> LicenseKind {
    let Some(rights) = rights else {
        return LicenseKind::Unknown;
    };
    let line = rights.to_ascii_lowercase();
    let version = if line.contains("4.0") {
        "4.0"
    } else if line.contains("3.0") {
        "3.0"
    } else {
        ""
    };
    if line.contains("public domain") {
        LicenseKind::PublicDomain
    } else if line.contains("cc0") {
        LicenseKind::Cc0
    } else if line.contains("cc by-sa") || line.contains("cc by sa") {
        LicenseKind::CcBySa(version.to_string())
    } else if line.contains("cc by") {
        LicenseKind::CcBy(version.to_string())
    } else if line.contains("ccli") || line.contains("one license") || line.contains("onelicense") {
        LicenseKind::LicensedReprint
    } else {
        LicenseKind::Unknown
    }
}

fn story(id: &str, paragraphs: Vec<(&str, String)>) -> Story {
    Story {
        id: id.to_string(),
        paragraphs: paragraphs
            .into_iter()
            .map(|(style, text)| Paragraph::plain(style, &text))
            .collect(),
        keep_through: None,
    }
}

fn panel(number: usize) -> Panel {
    Panel {
        id: format!("page-{}", number + 1),
        name: format!("Page {}", number + 1),
        size: Size::new(PANEL_W, PANEL_H),
        frames: Vec::new(),
        guides: Vec::new(),
        background_image: None,
    }
}

fn text_frame(id: &str, name: &str, story_id: &str, rect: Rect) -> Frame {
    Frame {
        id: id.to_string(),
        name: name.to_string(),
        kind: FrameKind::Text,
        rect,
        story_id: Some(story_id.to_string()),
        role: ContentRole::Standing,
        picture: None,
        music: None,
    }
}

fn item(panel: usize, label: &str, detail: String, badge: Badge) -> ImportItem {
    ImportItem {
        panel,
        label: label.to_string(),
        badge,
        detail,
    }
}

/// Compare what the file holds against what the engraving
/// drew, in the review flow's own words. Every number on
/// both sides is in the item, so a shortfall is checkable
/// rather than a vibe.
pub fn count_item(info: &MusicInfo, render: &RenderOutcome) -> ImportItem {
    let file = &info.counts;
    let expected_heads = file.noteheads_expected();
    let mut shortfalls: Vec<String> = Vec::new();
    if render.noteheads() < expected_heads {
        shortfalls.push(format!(
            "The file has {} notes; the printed music shows {}.",
            expected_heads,
            render.noteheads()
        ));
    }
    if render.measures() < file.measures {
        shortfalls.push(format!(
            "The file has {} measures; the printed music shows {}.",
            file.measures,
            render.measures()
        ));
    }
    if render.syllables() < file.syllables {
        shortfalls.push(format!(
            "The file has {} sung syllables; the printed music shows {}.",
            file.syllables,
            render.syllables()
        ));
    }
    if render.fermatas() < file.fermatas {
        shortfalls.push(format!(
            "The file has {} holds (fermatas); the printed music shows {}.",
            file.fermatas,
            render.fermatas()
        ));
    }
    let detail = format!(
        "Counted on both sides: the file holds {} notes, {} measures, {} sung syllables, \
         and {} holds. The printed music shows {} notes, {} measures, {} syllables, and {} \
         holds. Counted by {} at a staff of {:.1} mm.",
        expected_heads,
        file.measures,
        file.syllables,
        file.fermatas,
        render.noteheads(),
        render.measures(),
        render.syllables(),
        render.fermatas(),
        render.backend,
        render
            .systems
            .first()
            .map(|s| s.staff_mm)
            .unwrap_or(TARGET_STAFF_MM)
    );
    if shortfalls.is_empty() {
        item(
            0,
            "Every note in the file is in the printed music",
            detail,
            Badge::Matched,
        )
    } else {
        item(
            0,
            "The printed music does not show everything in the file",
            format!("{} {}", shortfalls.join(" "), detail),
            Badge::CheckThis,
        )
    }
}

/// Build the draft. `render` is None when the engraver is
/// not on this computer: the draft still forms with the
/// heading and credit as words and one empty music box.
pub fn build_music_draft(
    info: &MusicInfo,
    source_bytes: &[u8],
    source_name: &str,
    media_type: &str,
    profile: ChurchProfile,
    render: Option<&RenderOutcome>,
) -> DraftOutcome {
    let rights = rights_from_music(info, source_name);
    let mut doc = Document {
        format_version: FORMAT_VERSION,
        title: info
            .title
            .clone()
            .unwrap_or_else(|| source_name.to_string()),
        profile,
        fields: Default::default(),
        styles: default_styles(),
        stories: Vec::new(),
        panels: Vec::new(),
        sheet: SheetSpec {
            preset: "saddle-booklet".to_string(),
            size: Size::new(PANEL_W * 2, PANEL_H),
        },
        no_announcements: false,
        assets: Vec::new(),
        asset_bytes: Default::default(),
        occasion_time: None,
        wanted_fonts: Vec::new(),
        participants: Default::default(),
        inserts: Default::default(),
        applied_contributions: Vec::new(),
    };
    // The music file itself rides in the document, so the
    // bulletin travels complete and the music can be
    // drawn again on any computer.
    doc.add_asset(source_bytes, AssetKind::Music, media_type, rights.clone());
    let source_hash = ob_model::sha256_hex(source_bytes);

    // The heading: the file's words, as words.
    let mut paras: Vec<(&str, String)> = Vec::new();
    if let Some(title) = &info.title {
        paras.push(("heading", title.clone()));
    }
    if let Some(sub) = &info.subtitle {
        paras.push(("subheading", sub.clone()));
    }
    if let Some(composer) = info.composer() {
        paras.push(("body", composer.to_string()));
    }
    if let Some(poet) = info.creator("lyricist") {
        paras.push(("body", poet.to_string()));
    }
    if paras.is_empty() {
        paras.push(("heading", source_name.to_string()));
    }
    doc.stories.push(story("music-heading", paras));

    let mut panels = vec![panel(0)];
    panels[0].frames.push(text_frame(
        "music-heading",
        "Hymn title and composer",
        "music-heading",
        Rect::new(MARGIN, MARGIN, music_frame_width_mpt(), HEADING_H),
    ));
    let mut cursor = MARGIN + HEADING_H + GAP;
    let mut here = 0usize; // current panel index

    let mut items: Vec<ImportItem> = vec![item(
        0,
        "Title and composer",
        format!(
            "The words above the music came from the file itself: {}",
            info.title.as_deref().unwrap_or(source_name)
        ),
        Badge::Matched,
    )];

    match render {
        Some(render) => {
            for (i, system) in render.systems.iter().enumerate() {
                let h = ((music_frame_width_mpt() as f64 * system.height_units
                    / system.width_units.max(1.0))
                .round() as i64)
                    .max(1);
                if cursor + h > PANEL_H - MARGIN && cursor > MARGIN {
                    panels.push(panel(panels.len()));
                    here = panels.len() - 1;
                    cursor = MARGIN;
                }
                let hash = doc.add_asset(
                    system.svg.as_bytes(),
                    AssetKind::Picture,
                    "image/svg+xml",
                    rights.clone(),
                );
                panels[here].frames.push(Frame {
                    id: format!("music-line-{}", i + 1),
                    name: format!("Music line {}", i + 1),
                    kind: FrameKind::Picture,
                    rect: Rect::new(MARGIN, cursor, music_frame_width_mpt(), h),
                    story_id: None,
                    role: ContentRole::Standing,
                    picture: Some(PicturePlacement {
                        asset: hash,
                        // Notation is never trimmed: the
                        // whole drawn line shows.
                        fit: PictureFit::ShowWhole,
                        crop_x: 0.5,
                        crop_y: 0.5,
                    }),
                    music: Some(MusicFrame {
                        system: i as u32,
                        staff_mm: system.staff_mm,
                        source: source_hash.clone(),
                    }),
                });
                cursor += h + GAP;
            }
            items.push(count_item(info, render));
            if let Some(smallest) = render.systems.iter().map(|s| s.staff_mm).reduce(f64::min) {
                if smallest < FLOOR_STAFF_MM {
                    // The render path refuses below-floor
                    // renders before they reach here; this is
                    // the builder's own belt on the same rule.
                    items.push(item(
                        0,
                        "The notes came out too small",
                        format!(
                            "One line of music measures {smallest:.1} mm of staff, under \
                             the {:.1} mm floor. Bring the file in again to have it \
                             drawn at a singing size.",
                            FLOOR_STAFF_MM
                        ),
                        Badge::CheckThis,
                    ));
                }
            }
        }
        None => {
            // The engraver is not on this computer: one
            // empty music box stands where the notes will
            // be drawn, at the size they will be drawn.
            let h = (music_frame_width_mpt() as f64 * 0.42).round() as i64;
            panels[here].frames.push(Frame {
                id: "music-line-1".into(),
                name: "Music line 1".into(),
                kind: FrameKind::Picture,
                rect: Rect::new(MARGIN, cursor, music_frame_width_mpt(), h),
                story_id: None,
                role: ContentRole::Standing,
                picture: None,
                music: Some(MusicFrame {
                    system: 0,
                    staff_mm: TARGET_STAFF_MM,
                    source: source_hash.clone(),
                }),
            });
            cursor += h + GAP;
            items.push(item(
                0,
                "The notes are not drawn yet",
                format!(
                    "The music engraver is not on this computer, so the notes cannot be \
                     drawn yet. The file itself is saved in this draft ({} measures, {} \
                     notes counted in it), and bringing the file in again on a computer \
                     with the engraver draws every line at a singing size.",
                    info.counts.measures,
                    info.counts.noteheads_expected()
                ),
                Badge::CheckThis,
            ));
        }
    }

    // The credit line under the music, as words.
    if let Some(credit) = info.credit_line() {
        if cursor + CREDIT_H > PANEL_H - MARGIN && cursor > MARGIN {
            panels.push(panel(panels.len()));
            here = panels.len() - 1;
            cursor = MARGIN;
        }
        doc.stories
            .push(story("music-credit", vec![("caption", credit)]));
        panels[here].frames.push(text_frame(
            "music-credit",
            "Music credit",
            "music-credit",
            Rect::new(MARGIN, cursor, music_frame_width_mpt(), CREDIT_H),
        ));
        items.push(item(
            here,
            "Credit line",
            "The credit printed under the music came from the file, as words that can \
             be corrected like any other words."
                .to_string(),
            Badge::Matched,
        ));
    }

    doc.panels = panels;
    items.push(item(
        0,
        "Lines of music",
        match render {
            Some(r) => format!(
                "{} lines of music, drawn at the panel's full width so the staff stays \
                 at a singing size (about {:.1} mm, never under {:.1} mm). Each line is \
                 a box you can move on the Pages step; music boxes are not resized, \
                 because their size is what keeps the notes readable.",
                r.systems.len(),
                r.systems
                    .first()
                    .map(|s| s.staff_mm)
                    .unwrap_or(TARGET_STAFF_MM),
                FLOOR_STAFF_MM
            ),
            None => "When the engraver draws the music, each line becomes a box at the \
                     panel's full width, sized to keep the notes readable."
                .to_string(),
        },
        Badge::Matched,
    ));

    DraftOutcome { doc, items }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::{parse_musicxml, TINY_SCORE};
    use crate::render::RenderedSystem;

    fn outcome_for(render: Option<RenderOutcome>) -> DraftOutcome {
        let info = parse_musicxml(TINY_SCORE).expect("tiny score parses");
        build_music_draft(
            &info,
            TINY_SCORE.as_bytes(),
            "tiny.musicxml",
            "application/vnd.recordare.musicxml+xml",
            ChurchProfile::default(),
            render.as_ref(),
        )
    }

    fn fake_render() -> RenderOutcome {
        RenderOutcome {
            backend: "Verovio 6.3.0 via test".into(),
            systems: vec![RenderedSystem {
                svg: "<svg xmlns=\"http://www.w3.org/2000/svg\"/>".into(),
                width_units: 1437.0,
                height_units: 231.0,
                staff_mm: 6.5,
                noteheads: 5, // 4 notes + the grace note
                measures: 2,
                syllables: 3,
                fermatas: 1,
            }],
        }
    }

    #[test]
    fn rendered_draft_places_every_line_with_its_marker() {
        let out = outcome_for(Some(fake_render()));
        let doc = &out.doc;
        let music: Vec<&Frame> = doc
            .panels
            .iter()
            .flat_map(|p| &p.frames)
            .filter(|f| f.music.is_some())
            .collect();
        assert_eq!(music.len(), 1);
        let f = music[0];
        assert_eq!(f.rect.w, music_frame_width_mpt());
        assert_eq!(f.music.as_ref().unwrap().system, 0);
        assert_eq!(f.music.as_ref().unwrap().staff_mm, 6.5);
        assert_eq!(
            f.music.as_ref().unwrap().source,
            ob_model::sha256_hex(TINY_SCORE.as_bytes())
        );
        // The heading carries the file's own words, and the
        // credit story the file's rights line.
        let heading = doc.story("music-heading").unwrap();
        assert!(heading
            .paragraphs
            .iter()
            .any(|p| p.text() == "A Tiny Test Hymn"));
        assert!(heading
            .paragraphs
            .iter()
            .any(|p| p.text() == "Test Composer"));
        let credit = doc.story("music-credit").unwrap();
        assert_eq!(
            credit.paragraphs[0].text(),
            "Public domain in the file's own words"
        );
        // The file itself and the drawn line are assets.
        assert_eq!(doc.assets.len(), 2);
        // Full marks on the counts.
        assert!(out.items.iter().any(|i| i.label
            == "Every note in the file is in the printed music"
            && i.badge == Badge::Matched));
    }

    #[test]
    fn shortfall_names_the_numbers() {
        let mut render = fake_render();
        render.systems[0].noteheads = 3;
        let out = outcome_for(Some(render));
        let check = out
            .items
            .iter()
            .find(|i| i.badge == Badge::CheckThis)
            .expect("a check-this item");
        assert!(
            check.detail.contains("The file has 5 notes"),
            "{}",
            check.detail
        );
        assert!(check.detail.contains("shows 3"), "{}", check.detail);
    }

    #[test]
    fn no_engraver_still_forms_a_draft_with_an_honest_box() {
        let out = outcome_for(None);
        let doc = &out.doc;
        let music: Vec<&Frame> = doc
            .panels
            .iter()
            .flat_map(|p| &p.frames)
            .filter(|f| f.music.is_some())
            .collect();
        assert_eq!(music.len(), 1);
        assert!(music[0].picture.is_none(), "no drawing is faked");
        let check = out
            .items
            .iter()
            .find(|i| i.detail.contains("music engraver is not on this computer"))
            .expect("the plain-words item");
        assert_eq!(check.badge, Badge::CheckThis);
        // The heading and credit still formed.
        assert!(doc.story("music-heading").is_some());
        assert!(doc.story("music-credit").is_some());
    }

    #[test]
    fn license_reads_are_conservative() {
        assert_eq!(
            license_from_rights(Some("Public domain")),
            LicenseKind::PublicDomain
        );
        assert_eq!(
            license_from_rights(Some("CC BY-SA 4.0")),
            LicenseKind::CcBySa("4.0".into())
        );
        assert_eq!(
            license_from_rights(Some("CC BY 3.0")),
            LicenseKind::CcBy("3.0".into())
        );
        assert_eq!(
            license_from_rights(Some("Used by permission, CCLI #123")),
            LicenseKind::LicensedReprint
        );
        assert_eq!(
            license_from_rights(Some("All rights reserved")),
            LicenseKind::Unknown
        );
        assert_eq!(license_from_rights(None), LicenseKind::Unknown);
    }
}
