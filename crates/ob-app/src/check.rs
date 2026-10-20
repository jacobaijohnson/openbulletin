//! The Check step's findings engine: the sentences
//! and notes that judge a bulletin — fonts, pictures
//! and children, hymns, characters, dates, and the
//! serving-line sentences — stated once for every
//! window that speaks them.

use crate::settings::*;
use crate::*;

/// One Check-step note. A note that knows which panel and
/// frame it is about carries them as a target: the Check
/// step puts a "Fix this" button under it that opens the
/// Pages window on that panel with that frame selected
/// (UI audit round 2).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CheckNote {
    pub(crate) text: String,
    /// (panel index, frame index) the note points at.
    pub(crate) target: Option<(usize, Option<usize>)>,
    /// A step the fix button jumps to instead of the Pages
    /// window, when the fix lives on a step (the date
    /// finding's fix lives on This Sunday, J1).
    pub(crate) step: Option<usize>,
    /// The fix lives in the Fonts window (a wanted font
    /// still missing): the button opens it.
    pub(crate) fonts: bool,
    /// The fix lives in the Loose sheets window (a loose
    /// sheet with no words, a dated sheet past its date):
    /// the button opens it.
    pub(crate) inserts: bool,
    /// A hymn pairing the note disagrees with, named by
    /// its hymn-memory key: the Check step puts a "Forget
    /// this pairing" button under the note that removes
    /// the pair from the church's hymn memory, for when
    /// the office says the memory is the wrong one
    /// (round 3, R3-6).
    pub(crate) forget_hymn: Option<String>,
}

impl CheckNote {
    pub(crate) fn plain(text: String) -> Self {
        CheckNote {
            text,
            target: None,
            step: None,
            fonts: false,
            inserts: false,
            forget_hymn: None,
        }
    }

    pub(crate) fn aimed(text: String, panel: usize, frame: Option<usize>) -> Self {
        CheckNote {
            text,
            target: Some((panel, frame)),
            step: None,
            fonts: false,
            inserts: false,
            forget_hymn: None,
        }
    }

    pub(crate) fn to_step(text: String, step: usize) -> Self {
        CheckNote {
            text,
            target: None,
            step: Some(step),
            fonts: false,
            inserts: false,
            forget_hymn: None,
        }
    }

    pub(crate) fn to_fonts(text: String) -> Self {
        CheckNote {
            text,
            target: None,
            step: None,
            fonts: true,
            inserts: false,
            forget_hymn: None,
        }
    }

    pub(crate) fn to_inserts(text: String) -> Self {
        CheckNote {
            text,
            target: None,
            step: None,
            fonts: false,
            inserts: true,
            forget_hymn: None,
        }
    }
}

/// One story whose words use a character the print file cannot
/// show (GPT workflow review, adopted item 1). The character
/// test itself is ob-pdf's own, so this warning and the printed
/// output can never disagree.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CharFinding {
    pub(crate) story_id: String,
    /// The story's first words, so the admin can find it.
    pub(crate) label: String,
    /// The unprintable characters, in the order they appear.
    pub(crate) chars: Vec<char>,
    /// The first frame holding this story, when it sits in
    /// one: the Check step's fix button opens the Pages
    /// window there.
    pub(crate) target: Option<(usize, Option<usize>)>,
    /// A catalog face that verifiably carries one of the
    /// missing marks (its downloaded cmap was checked, see
    /// ob_fonts::catalog): the character and the family, so
    /// the note can point at the way out instead of only
    /// naming the problem.
    pub(crate) helper: Option<(char, String)>,
}

/// Every story in the document that uses characters the print
/// copy cannot draw in the face that would print them. Fields
/// are filled in first, because the filled-in words are what
/// actually prints; each run is tested against its own face
/// (the paragraph style's family with the run's bold/italic),
/// resolved exactly the way the PDF writer resolves it, so a
/// character is only named when that face truly has no glyph
/// for it.
/// The Check step's wanted-font findings (docs/FONTS.md):
/// for each font the document was made in that the
/// library still does not hold exactly, one finding in
/// the import review's own words, with the stand-in the
/// chain picks TODAY (a nearer font added since the
/// import is named, not the stale one on the record).
/// Only families a style still names are reported
/// (round 3, R3-16): restyle the whole bulletin away
/// from a wanted font and the record stays on the file
/// but the finding goes silent, because nothing prints
/// in that font anymore. Pure over (document, available
/// families) so the words are testable without a library
/// on disk.
pub(crate) fn wanted_font_notes(doc: &Document, available: &[String]) -> Vec<CheckNote> {
    let mut out = Vec::new();
    let used: Vec<String> = doc
        .styles
        .iter()
        .map(|s| s.font_family.clone())
        .chain(
            doc.inserts
                .iter()
                .flat_map(|ins| ins.styles.iter().map(|s| s.font_family.clone())),
        )
        .collect();
    // How far from Exact a resolution landed, worst
    // last: the finding a style earns is the worst one
    // among the styles that use its family (round 5,
    // R5-D).
    pub(crate) fn tier_rank(tier: ob_fonts::resolve::SubstituteTier) -> u8 {
        match tier {
            ob_fonts::resolve::SubstituteTier::Exact => 0,
            ob_fonts::resolve::SubstituteTier::MetricTwin => 1,
            ob_fonts::resolve::SubstituteTier::Lookalike => 2,
            ob_fonts::resolve::SubstituteTier::HouseDefault => 3,
        }
    }
    for w in &doc.wanted_fonts {
        // The record names the family in display form;
        // the styles that still use it carry the full
        // source name. Resolution keys off a style's full
        // name, not the record (round 4, R4-1): the two
        // can differ ("Verdana-Bold" on the style,
        // "Verdana" on the record), and what prints is
        // decided by the name on the style.
        // EVERY style whose family matches the record is
        // examined (round 5, R5-D): two source families
        // can share one display name, and resolving only
        // the first let an Exact first style certify a
        // second style that prints in a stand-in. The
        // finding reflects the worst resolution among
        // them.
        let mut worst: Option<ob_fonts::resolve::Resolution> = None;
        // Whether the matching styles resolve to more
        // than one stand-in family (round 6, R6-9): the
        // note's claim is scoped when they do.
        let mut first_family: Option<String> = None;
        let mut stand_ins_differ = false;
        for family in used.iter().filter(|family| {
            family.eq_ignore_ascii_case(&w.name)
                || ob_fonts::resolve::display_name_for(family).eq_ignore_ascii_case(&w.name)
        }) {
            let r = ob_fonts::resolve::resolve_family(family, available);
            match &first_family {
                None => first_family = Some(r.family.clone()),
                Some(first) if *first != r.family => stand_ins_differ = true,
                _ => {}
            }
            let is_worse = match &worst {
                None => true,
                Some(worst_so_far) => tier_rank(r.tier) > tier_rank(worst_so_far.tier),
            };
            if is_worse {
                worst = Some(r);
            }
        }
        let Some(r) = worst else {
            continue;
        };
        if r.tier != ob_fonts::resolve::SubstituteTier::Exact {
            // The stand-in named is the worst style's
            // (the one the finding is about); when the
            // styles resolve to different stand-ins, the
            // sentence says SOME of its words, because
            // the whole-bulletin wording would be false
            // for every other style (round 6, R6-9).
            // A single stand-in keeps the round-3 words.
            let words = if stand_ins_differ {
                ob_fonts::resolve::wanted_font_words_some(&w.name, &r.family)
            } else {
                ob_fonts::resolve::wanted_font_words(&w.name, &r.family)
            };
            out.push(CheckNote::to_fonts(format!("Check: {words}")));
        }
    }
    out
}

/// Does this placed picture still need its children
/// note (R-2b)? True when the box holds a picture that
/// is in the bulletin and its rights record carries no
/// answer yet: Check asks, because the web page's rule
/// can only take the care the note describes. A box
/// whose picture is missing in fact is another
/// finding's business. A music box is never asked
/// (round 3, R3-4): its picture is engraved sheet
/// music, not a photograph of anybody, and its menu
/// has no children answer to give, so the question
/// would be unanswerable noise. Pure over (document,
/// frame).
pub(crate) fn picture_children_unnoted(doc: &Document, frame: &ob_model::Frame) -> bool {
    if frame.music.is_some() {
        return false;
    }
    let Some(placement) = &frame.picture else {
        return false;
    };
    if doc.asset_data(&placement.asset).is_none() {
        return false;
    }
    doc.assets
        .iter()
        .find(|a| a.sha256 == placement.asset)
        .and_then(|a| a.rights.children_present)
        .is_none()
}

/// Check's children's-names reminder (R-2b): when the
/// church's rule is not full names, one plain note
/// repeating the rule, honest about its limit. The app
/// cannot tell which names are children's; it can only
/// remind a person to look. Pure over the profile.
pub(crate) fn children_names_note(profile: &ob_model::ChurchProfile) -> Option<CheckNote> {
    match profile.children_names {
        ob_model::ChildrenNames::FullNames => None,
        rule => Some(CheckNote::plain(format!(
            "Your church's rule: {}. This is a reminder to look over the names; \
             the app cannot tell which names are children's.",
            rule.rule_words()
        ))),
    }
}

/// Check's hymn pass (R-2a): each hymn line in the order
/// of service, held up against itself and against the
/// church's hymn memory. A number with no title and a
/// title with no number are both easy to mistype and hard
/// to spot; a pair the church has printed differently
/// before names both versions, and the note carries a
/// "Forget this pairing" button for when the office says
/// the memory is the wrong one (round 3, R3-6). When the
/// memory holds no pair, it says nothing: nothing is
/// invented. Pure over (document, memory); every finding
/// points at the This Sunday step, where the hymn fields
/// live.
pub(crate) fn hymn_findings(
    doc: &Document,
    memory: &ob_model::hymns::HymnMemory,
) -> Vec<CheckNote> {
    use ob_model::hymns::{compose_hymn_value, is_hymn_label, parse_hymn_value};
    let mut out = Vec::new();
    let mut by_number: Vec<(String, Option<String>, String, String)> = Vec::new(); // (number, hymnal, shown, label)
    for line in service_lines(doc) {
        if !is_hymn_label(&line.label) {
            continue;
        }
        if line.value.trim().is_empty() {
            continue;
        }
        let entry = parse_hymn_value(&line.value);
        match (&entry.number, &entry.title) {
            (Some(number), None) => out.push(CheckNote::to_step(
                format!(
                    "Check: the {} line gives hymn {number} but no title. \
                     A number alone is easy to mistype; add the title so it can be checked.",
                    line.label
                ),
                1,
            )),
            (None, Some(title)) => out.push(CheckNote::to_step(
                format!(
                    "Check: the {} line gives the title \u{201c}{title}\u{201d} but no hymn number. \
                     A title alone is easy to mistype; add the number so it can be checked.",
                    line.label
                ),
                1,
            )),
            (Some(number), Some(title)) => {
                if let Some(pair) = memory.lookup(entry.hymnal.as_deref(), number) {
                    let remembered = pair.title.trim();
                    if !remembered.eq_ignore_ascii_case(title.trim()) {
                        let hymn_words = match &entry.hymnal {
                            Some(h) => format!("{h} {number}"),
                            None => number.clone(),
                        };
                        let mut note = CheckNote::to_step(
                            format!(
                                "Check: hymn {hymn_words} was printed as \u{2018}{remembered}\u{2019} \
                                 for {}. This week it says \u{2018}{title}\u{2019}. One of them may be wrong.",
                                ob_layout::long_date(naive_of(pair.last_seen))
                            ),
                            1,
                        );
                        // If the office says the memory is
                        // the wrong one, the note's button
                        // forgets the pairing (R3-6).
                        note.forget_hymn = Some(ob_model::hymns::hymn_key(
                            entry.hymnal.as_deref(),
                            number,
                        ));
                        out.push(note);
                    }
                }
            }
            (None, None) => {}
        }
        if let Some(number) = &entry.number {
            by_number.push((
                number.clone(),
                entry.hymnal.clone(),
                compose_hymn_value(&entry),
                line.label.clone(),
            ));
        }
    }
    // The same number on two hymn lines: named once,
    // however many lines share it. Two lines name the
    // same hymn when their numbers match and their
    // hymnals agree, or when either line names no hymnal
    // at all (round 3, R3-17): "ELW 432" on one line and
    // a bare "432" on another are one song sung twice,
    // not two. Different hymnals with the same number
    // are different books, and stay quiet.
    let mut named: Vec<String> = Vec::new();
    for (i, (number, hymnal, shown, label)) in by_number.iter().enumerate() {
        if named.contains(number) {
            continue;
        }
        let other = by_number.iter().skip(i + 1).find(|(n2, h2, _, _)| {
            n2 == number
                && match (hymnal, h2) {
                    (Some(a), Some(b)) => a.eq_ignore_ascii_case(b),
                    _ => true,
                }
        });
        let Some((_, other_hymnal, _other_shown, other_label)) = other else {
            continue;
        };
        named.push(number.clone());
        let text = match (hymnal.as_deref(), other_hymnal.as_deref()) {
            (Some(h), None) | (None, Some(h)) => format!(
                "Check: hymn {number} is listed twice, once as {h} {number} and once with no hymnal named. \
                 Only one of them can be right."
            ),
            _ => format!(
                "Check: hymn {shown} is named by two hymn lines, {label} and {other_label}. \
                 Only one of them can be right."
            ),
        };
        out.push(CheckNote::to_step(text, 1));
    }
    out
}

/// What a music proof carries, counted for its status
/// line (R-2a). The spoken-only count rides along so the
/// Print status says what was left out, like every other
/// door does (round 3, R3-15).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MusicProofCounts {
    pub(crate) hymns: usize,
    pub(crate) music_pages: usize,
    pub(crate) spoken_only_left_out: usize,
}

/// Compose the music proof document (R-2a): a heading,
/// the hymn lines as words, then one page per engraved
/// music system at its bulletin size, then the anthem
/// and special music lines. Composed from the
/// publication copy, so spoken-only words never reach
/// it. The music pages follow the piece's own order,
/// system by system, however the systems' boxes sit in
/// the bulletin, and a system carried by two boxes
/// prints once: the proof is a reading copy of the
/// piece, not of the layout (round 3, R3-15). The music
/// director checks this sheet, or carries it to the
/// piano; the organist's copy is the same sheet. Pure:
/// the same document in, the same proof out.
pub(crate) fn music_proof_document(doc: &Document) -> (Document, MusicProofCounts) {
    use ob_model::hymns::{compose_hymn_value, is_hymn_label, parse_hymn_value};
    let (base, spoken_only_left_out) = doc.without_spoken_only();
    let page = ob_model::Size::LETTER_PORTRAIT;
    let margin: ob_model::Mpt = 54_000; // 3/4 inch
    let usable_w = page.w - 2 * margin;
    let usable_h = page.h - 2 * margin;
    let heading_style = if base.style("heading").is_some() {
        "heading"
    } else {
        "body"
    };
    let church = base.profile.name.trim();
    let date_long = base
        .fields
        .get("occasion_date_long")
        .map(|s| s.trim())
        .unwrap_or("");
    let heading = match (church.is_empty(), date_long.is_empty()) {
        (false, false) => format!("{church}: music for {date_long}"),
        (false, true) => format!("{church}: music"),
        (true, false) => format!("Music for {date_long}"),
        (true, true) => "Music".to_string(),
    };
    let mut word_paras = vec![ob_model::Paragraph::plain(heading_style, &heading)];
    let mut anthem_paras: Vec<ob_model::Paragraph> = Vec::new();
    let mut hymns = 0usize;
    for line in service_lines(&base) {
        let value = line.value.trim();
        if value.is_empty() {
            continue;
        }
        if is_hymn_label(&line.label) {
            let composed = compose_hymn_value(&parse_hymn_value(value));
            if composed.is_empty() {
                continue;
            }
            word_paras.push(ob_model::Paragraph::plain(
                "body",
                &format!("{}: {}", line.label, composed),
            ));
            hymns += 1;
        } else {
            let l = line.label.to_lowercase();
            if l.contains("anthem")
                || l.contains("special music")
                || l.contains("solo")
                || l.contains("choir")
            {
                anthem_paras.push(ob_model::Paragraph::plain(
                    "body",
                    &format!("{}: {}", line.label, value),
                ));
            }
        }
    }
    let text_frame = |id: &str, name: &str, story: &str| ob_model::Frame {
        id: id.to_string(),
        name: name.to_string(),
        kind: ob_model::FrameKind::Text,
        rect: ob_model::Rect::new(margin, margin, usable_w, usable_h),
        story_id: Some(story.to_string()),
        role: ob_model::ContentRole::Standing,
        picture: None,
        music: None,
    };
    let mut stories = vec![ob_model::Story {
        id: "proof-words".to_string(),
        paragraphs: word_paras,
        keep_through: None,
    }];
    let mut panels = vec![ob_model::Panel {
        id: "proof-page-1".to_string(),
        name: "Music proof".to_string(),
        size: page,
        frames: vec![text_frame(
            "proof-words-frame",
            "Music proof",
            "proof-words",
        )],
        guides: Vec::new(),
        background_image: None,
    }];
    // One page per music system, at its bulletin size
    // (shrunk only when the bulletin box is bigger than
    // this page), the whole system shown: notation is
    // never trimmed. The systems come in the piece's
    // own order (round 3, R3-15): a box moved on the
    // page in the editor reorders the bulletin, not the
    // piece, and two boxes carrying the same system
    // print that system once. The key is (source,
    // system), and the pieces come in the order their
    // first box appears in the bulletin (round 4,
    // R4-3): system numbers count from 0 per piece, so
    // on their own they collide across pieces, and the
    // later piece's systems used to vanish from the
    // proof the music director checks.
    let mut systems: Vec<&ob_model::Frame> = Vec::new();
    for panel in &base.panels {
        for frame in &panel.frames {
            if frame.music.is_some() && frame.picture.is_some() {
                systems.push(frame);
            }
        }
    }
    let mut piece_order: Vec<&str> = Vec::new();
    for f in &systems {
        let src = f.music.as_ref().map(|m| m.source.as_str()).unwrap_or("");
        if !piece_order.contains(&src) {
            piece_order.push(src);
        }
    }
    let piece_rank = |f: &ob_model::Frame| {
        let src = f.music.as_ref().map(|m| m.source.as_str()).unwrap_or("");
        piece_order.iter().position(|s| *s == src).unwrap_or(0)
    };
    systems.sort_by_key(|f| {
        (
            piece_rank(f),
            f.music.as_ref().map(|m| m.system).unwrap_or(0),
        )
    });
    systems.dedup_by_key(|f| {
        f.music
            .as_ref()
            .map(|m| (m.source.clone(), m.system))
            .unwrap_or_default()
    });
    let mut music_pages = 0usize;
    for frame in systems {
        let Some(picture) = &frame.picture else {
            continue;
        };
        music_pages += 1;
        let (mut w, mut h) = (frame.rect.w.max(1), frame.rect.h.max(1));
        if w > usable_w || h > usable_h {
            let scale = (usable_w as f64 / w as f64).min(usable_h as f64 / h as f64);
            w = (w as f64 * scale).round() as ob_model::Mpt;
            h = (h as f64 * scale).round() as ob_model::Mpt;
        }
        let mut nf = frame.clone();
        nf.id = format!("proof-music-{music_pages}");
        nf.name = "Printed music".to_string();
        nf.rect = ob_model::Rect::new(margin, margin, w, h);
        nf.story_id = None;
        nf.picture = Some(ob_model::PicturePlacement {
            fit: ob_model::PictureFit::ShowWhole,
            ..picture.clone()
        });
        panels.push(ob_model::Panel {
            id: format!("proof-music-page-{music_pages}"),
            name: "Printed music".to_string(),
            size: page,
            frames: vec![nf],
            guides: Vec::new(),
            background_image: None,
        });
    }
    if !anthem_paras.is_empty() {
        stories.push(ob_model::Story {
            id: "proof-anthems".to_string(),
            paragraphs: anthem_paras,
            keep_through: None,
        });
        panels.push(ob_model::Panel {
            id: "proof-anthem-page".to_string(),
            name: "Anthems and special music".to_string(),
            size: page,
            frames: vec![text_frame(
                "proof-anthems-frame",
                "Anthems and special music",
                "proof-anthems",
            )],
            guides: Vec::new(),
            background_image: None,
        });
    }
    let mut out = base;
    out.title = "Music proof".to_string();
    out.stories = stories;
    out.panels = panels;
    out.sheet = ob_model::SheetSpec {
        preset: "letter-portrait".to_string(),
        size: page,
    };
    (
        out,
        MusicProofCounts {
            hymns,
            music_pages,
            spoken_only_left_out,
        },
    )
}

pub(crate) fn char_findings(doc: &Document) -> Vec<CharFinding> {
    let oracle = ob_pdf::FaceOracle::new(&ob_pdf::library_font_bytes());
    // One field table for the whole pass (see
    // ob_model::FieldTable): resolution used to rebuild it
    // per paragraph.
    let table = ob_model::FieldTable::for_document(doc);
    let mut out = Vec::new();
    for story in &doc.stories {
        let mut chars: Vec<char> = Vec::new();
        for para in &story.paragraphs {
            // Spoken-only paragraphs never print, so their
            // marks can never go missing from a printed
            // copy: not a finding (working as intended).
            if para.spoken_only {
                continue;
            }
            // The same fallback style the PDF writer draws
            // with when a paragraph names a style the document
            // no longer carries.
            let style = doc
                .style(&para.style)
                .cloned()
                .unwrap_or(ob_model::ParagraphStyle {
                    id: "body".into(),
                    name: "Body".into(),
                    font_size: 11_000,
                    bold: false,
                    italic: false,
                    align: ob_model::Align::Left,
                    space_after: 4_000,
                    color: ob_model::Rgb::BLACK,
                    font_family: "Liberation Sans".into(),
                    font_sha256: None,
                });
            for run in table.resolve_runs(&para.runs) {
                let bold = run.bold.unwrap_or(style.bold);
                let italic = run.italic.unwrap_or(style.italic);
                for c in oracle.undrawable_chars(&run.text, &style.font_family, bold, italic) {
                    if !chars.contains(&c) {
                        chars.push(c);
                    }
                }
            }
        }
        if chars.is_empty() {
            continue;
        }
        let label = story
            .paragraphs
            .iter()
            .map(|p| p.text())
            .find(|t| !t.trim().is_empty())
            .map(|t| t.trim().chars().take(38).collect::<String>())
            .unwrap_or_else(|| "A text block".to_string());
        let target = frame_target_for_story(doc, &story.id);
        // The way out, when the catalog holds one: a face
        // whose cmap verifiably carries the missing mark.
        let helper = chars.iter().find_map(|c| {
            ob_fonts::catalog::catalog_face_with_glyph(*c).map(|fam| (*c, fam.to_string()))
        });
        out.push(CharFinding {
            story_id: story.id.clone(),
            label,
            chars,
            target,
            helper,
        });
    }
    out
}

/// The first frame holding a story, as (panel, frame)
/// indices, when the story sits in a frame at all. Shared
/// by the character findings and the font findings so a
/// Check note can point at the box it means.
pub(crate) fn frame_target_for_story(
    doc: &Document,
    story_id: &str,
) -> Option<(usize, Option<usize>)> {
    for (pi, panel) in doc.panels.iter().enumerate() {
        for (fi, frame) in panel.frames.iter().enumerate() {
            if frame.story_id.as_deref() == Some(story_id) {
                return Some((pi, Some(fi)));
            }
        }
    }
    None
}

/// The Check-step words for one unprintable-character finding.
pub(crate) fn char_note(f: &CharFinding) -> String {
    let marks = f
        .chars
        .iter()
        .map(|c| format!("\"{c}\""))
        .collect::<Vec<_>>()
        .join(", ");
    let mut note = if f.chars.len() == 1 {
        format!(
            "Check: the words in \"{}\" use a mark the print copy cannot show: {marks}. \
             It would print as an empty box. Type a plain mark instead, or take it out.",
            f.label
        )
    } else {
        format!(
            "Check: the words in \"{}\" use marks the print copy cannot show: {marks}. \
             They would print as empty boxes. Type plain marks instead, or take them out.",
            f.label
        )
    };
    // The way out, when the catalog verifiably holds the
    // mark: name the face that carries it.
    if let Some((_, family)) = &f.helper {
        note.push_str(&format!(
            " {family} has this letter; it is in the Fonts list."
        ));
    }
    note
}

/// The Check-step line about announcements, in plain words.
/// An empty box is only fine when the office said so out loud
/// (adopted item 4).
pub(crate) fn announcements_check(announcements_empty: bool, none_marked: bool) -> String {
    if !announcements_empty {
        "OK: announcements are filled in.".to_string()
    } else if none_marked {
        "OK: no announcements this week, as marked.".to_string()
    } else {
        "Check: announcements are still needed. Add them on the \
         Announcements step, or mark \"No announcements this week\" \
         there if there are none."
            .to_string()
    }
}

/// What is being carried into this week, as information
/// lines for the Check step (J10): every story whose
/// keep-through date covers the bulletin's Sunday, named by
/// its first words and its through date. Automation shows
/// its hand; these are facts, not problems, so the lines
/// never start with "Check:".
pub(crate) fn carried_lines(doc: &Document) -> Vec<String> {
    let Some(bulletin) = doc
        .fields
        .get("occasion_date")
        .and_then(|iso| chrono::NaiveDate::parse_from_str(iso.trim(), "%Y-%m-%d").ok())
    else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for story in &doc.stories {
        let Some(raw) = story.keep_through.as_deref() else {
            continue;
        };
        let Ok(until) = chrono::NaiveDate::parse_from_str(raw.trim(), "%Y-%m-%d") else {
            continue;
        };
        if until < bulletin {
            continue;
        }
        let owned_text = story
            .paragraphs
            .iter()
            .filter(|p| p.style == "body")
            .map(|p| p.text())
            .find(|t| !t.trim().is_empty())
            .or_else(|| {
                story
                    .paragraphs
                    .iter()
                    .map(|p| p.text())
                    .find(|t| !t.trim().is_empty())
            });
        let first_text = owned_text.as_deref().unwrap_or("An announcement");
        let first: String = first_text
            .lines()
            .next()
            .unwrap_or(first_text)
            .trim()
            .chars()
            .take(40)
            .collect();
        let through = ob_layout::long_date(until);
        out.push(format!(
            "Carried into this week: \"{first}\" (kept through {through})."
        ));
    }
    out
}

/// Whether a date field holds an edit: some text, and not
/// the bulletin's stored date. A blank field is not an
/// edit; the bulletin always has a date.
pub(crate) fn date_differs(input: &str, stored: Option<&str>) -> bool {
    let input = input.trim();
    !input.is_empty() && input != stored.unwrap_or("")
}

/// "Sunday, October 18, 2026": the long form with the
/// weekday, the way dates are spoken on the steps.
/// The weekday name is ob-model's one table
/// ([`WEEKDAY_NAMES`], health audit E-7), shared with
/// the `SimpleDate` formatters.
pub(crate) fn long_form(d: chrono::NaiveDate) -> String {
    use chrono::Datelike;
    let weekday = ob_model::WEEKDAY_NAMES[d.weekday().num_days_from_sunday() as usize];
    format!("{}, {}", weekday, ob_layout::long_date(d))
}

/// Today's date in the model's own date type, for the
/// occasions parser (docs/planning/occasions.md).
pub(crate) fn today_simple() -> ob_model::SimpleDate {
    use chrono::Datelike;
    let t = chrono::Local::now().date_naive();
    ob_model::SimpleDate {
        year: t.year(),
        month: t.month(),
        day: t.day(),
    }
}

/// A file's modification time (seconds since the
/// epoch, as the inbox scan records it) as a calendar
/// date in local time — the anchor a contribution
/// file's mixed-signal relative phrases resolve
/// against (round 14, R14-6). Zero or unconvertible
/// times carry no date: the parse falls back to today.
pub(crate) fn file_date_of(modified_secs: u64) -> Option<ob_model::SimpleDate> {
    if modified_secs == 0 {
        return None;
    }
    let utc = chrono::DateTime::from_timestamp(modified_secs as i64, 0)?;
    Some(simple_of(utc.with_timezone(&chrono::Local).date_naive()))
}

/// A chrono date in the model's own date type.
pub(crate) fn simple_of(d: chrono::NaiveDate) -> ob_model::SimpleDate {
    use chrono::Datelike;
    ob_model::SimpleDate {
        year: d.year(),
        month: d.month(),
        day: d.day(),
    }
}

/// The model's date type back as a chrono date (for the
/// weekday-inclusive formatting the steps speak in).
pub(crate) fn naive_of(d: ob_model::SimpleDate) -> chrono::NaiveDate {
    chrono::NaiveDate::from_ymd_opt(d.year, d.month, d.day)
        .unwrap_or_else(|| chrono::NaiveDate::from_ymd_opt(1970, 1, 1).expect("epoch"))
}

/// The Check step's date finding (J1, generalized by
/// occasions): a Sunday-dated bulletin whose date is not
/// the coming Sunday keeps the coming-Sunday wording; a
/// bulletin for another day (Ash Wednesday, a wedding)
/// is only flagged when its day has passed, and the words
/// say so without claiming Sunday. A date that does not
/// read gets no verdict here; the missing-date note
/// covers that case.
pub(crate) fn date_verdict(doc_iso: &str, today: chrono::NaiveDate) -> Option<String> {
    let parsed = ob_model::parse_flexible_date(doc_iso.trim(), simple_of(today))?;
    let date = naive_of(parsed);
    if parsed.weekday() == 0 {
        let coming = sunday_on_or_after(today);
        if date == coming {
            return None;
        }
        return Some(format!(
            "Check: this bulletin says {}, but this coming Sunday is {}.",
            long_form(date),
            long_form(coming)
        ));
    }
    if date < today {
        return Some(format!(
            "Check: this bulletin says {}, and that day has passed.",
            long_form(date)
        ));
    }
    None
}

/// The stale-date sentence (J1), shared by the Start status
/// and the This Sunday step: the bulletin's date is behind
/// today, so the bulletin on screen is for a day that has
/// already happened. A Sunday keeps the coming-Sunday
/// wording; another day is named plainly.
pub(crate) fn date_passed_sentence(
    doc_iso: Option<&str>,
    today: chrono::NaiveDate,
) -> Option<String> {
    let parsed = ob_model::parse_flexible_date(doc_iso?.trim(), simple_of(today))?;
    let date = naive_of(parsed);
    if date >= today {
        return None;
    }
    Some(if parsed.weekday() == 0 {
        "That date has passed. Set the date for this Sunday on the This Sunday step.".to_string()
    } else {
        "That date has passed. Set the date on the This Sunday step.".to_string()
    })
}

/// The profile save's words when serving lines were
/// removed with names still saved for them (round 4,
/// RT-6c): the names are cleared with the line, and
/// the status says so plainly, one line or several.
pub(crate) fn removed_serving_lines_sentence(removed: &[String]) -> String {
    let quoted = removed
        .iter()
        .map(|n| format!("'{n}'"))
        .collect::<Vec<_>>()
        .join(", ");
    if removed.len() == 1 {
        format!("Names saved for {quoted} were removed with the line.")
    } else {
        format!("Names saved for {quoted} were removed with their lines.")
    }
}

/// The profile save's words when a serving-lines
/// migration did not follow a pairing or a removal
/// because the old name was already paired (round
/// 12, R12-11): the save stands, the names stand
/// with the pairing that was kept, and the status
/// names the name, the kept pairing, and the event
/// that was not followed, so the office can correct
/// the names by hand.
pub(crate) fn unfollowed_serving_lines_sentence(
    unfollowed: &[ob_model::ServingLineUnfollowed],
) -> String {
    let mut sentences: Vec<String> = Vec::new();
    for event in unfollowed {
        let old = format!("'{}'", event.old_name);
        let kept = format!("'{}'", event.kept_new_name);
        match &event.unfollowed_new_name {
            Some(unfollowed_new_name) => {
                let other = format!("'{unfollowed_new_name}'");
                let sentence = format!(
                    "{old} was paired with {kept} and with {other}: the names \
                     went to {kept}, and the pairing with {other} was not \
                     followed. Correct the names by hand if they belong \
                     under {other}."
                );
                sentences.push(sentence);
            }
            None => {
                let sentence = format!(
                    "{old} was paired with {kept} and was also removed: the \
                     names went to {kept}, and the removal was not followed. \
                     Correct the names by hand if they do not belong under \
                     {kept}."
                );
                sentences.push(sentence);
            }
        }
    }
    sentences.join(" ")
}

/// The profile save's words when a serving-lines
/// change is refused as a whole (round 6, R6-C):
/// the app cannot tell which names belong to which
/// line; the lines were left as they were, and the
/// remedy — one line at a time — now works, because
/// nothing half-applied. Round 7 (R7-10): the
/// account names only the kinds of change the
/// pairing actually saw, so a rename-plus-removal
/// refusal no longer claims a move that never
/// happened; the round-6 wording stands exactly as
/// written for the shape it was written for —
/// renames that crossed anchors, with nothing
/// removed or added.
pub(crate) fn ambiguous_serving_lines_sentence(kinds: &ob_model::ServingLineEditKinds) -> String {
    let canonical = ob_model::ServingLineEditKinds {
        renamed: true,
        removed: false,
        added: false,
        moved: true,
    };
    if *kinds == canonical {
        return "Serving lines not saved: several lines were renamed and moved at once, \
             and the app cannot tell which names belong to which line. The lines \
             were left as they were. Rename one line at a time, then save again."
            .to_string();
    }
    // Join a short word list the plain way: "a and
    // b", or "a, b and c" — with `last` before the
    // final word.
    let join_words = |words: &[&str], last: &str| -> String {
        match words.len() {
            0 => String::new(),
            1 => words[0].to_string(),
            2 => format!("{} {last} {}", words[0], words[1]),
            _ => {
                let head = words[..words.len() - 1].join(", ");
                format!("{head} {last} {}", words[words.len() - 1])
            }
        }
    };
    let mut seen: Vec<&str> = Vec::new();
    if kinds.renamed {
        seen.push("renamed");
    }
    if kinds.removed {
        seen.push("removed");
    }
    if kinds.added {
        seen.push("added");
    }
    if kinds.moved {
        seen.push("moved");
    }
    if seen.is_empty() {
        seen.push("changed");
    }
    let account = join_words(&seen, "and");
    // The remedy names the worker's own actions the
    // same way: make each rename, removal, or
    // addition on its own, then save again.
    let mut verbs: Vec<&str> = Vec::new();
    if kinds.renamed {
        verbs.push("rename");
    }
    if kinds.removed {
        verbs.push("remove");
    }
    if kinds.added {
        verbs.push("add");
    }
    if verbs.is_empty() {
        verbs.push("rename");
    }
    let remedy_joined = join_words(&verbs, "or");
    let mut remedy = remedy_joined;
    if let Some(first) = remedy.get_mut(0..1) {
        first.make_ascii_uppercase();
    }
    format!(
        "Serving lines not saved: lines were {account} at once, \
         and the app cannot tell which names belong to which line. The lines \
         were left as they were. {remedy} one line at a time, then save again."
    )
}

/// The profile save's words when serving lines are
/// refused over a shared slug (round 5, RT5-4): the
/// save does not apply, the typed lines stay in the
/// editor for correction, and the sentence names
/// both lines so the office can find the clash.
/// Round 6 (R6-11): the closing words claim only
/// what this refusal covers — the serving lines were
/// left as they were. The profile's other fields
/// save on their own as they are edited, so a wider
/// "nothing else was changed" could be false of the
/// editing pass the person is in the middle of.
pub(crate) fn serving_line_conflict_sentence(
    conflict: &ob_model::ServingLineSlugConflict,
) -> String {
    let first = format!("'{}'", conflict.first);
    let second = format!("'{}'", conflict.second);
    if conflict.builtin && conflict.taker_is_new {
        // The taking line is a new line (round 8,
        // R8-11): it has no old name to be named by,
        // and the holding side is named by the name
        // the bulletin currently prints for the
        // claimed line, so the two sides never read
        // as one unqualified name.
        format!(
            "Serving lines not saved: a new line, {first}, would take the \
             place of the built-in line {second}, so the line's row and \
             the names the bulletin prints could name different people. \
             Rename the new line, then save again. The serving lines \
             were left as they were."
        )
    } else if conflict.builtin {
        format!(
            "Serving lines not saved: {first} would take the place of the \
             built-in line {second}, so the line's row and the names the \
             bulletin prints could name different people. Rename {first}, \
             then save again. The serving lines were left as they were."
        )
    } else {
        format!(
            "Serving lines not saved: {first} and {second} would be stored \
             as the same line, so typing names for one would change the \
             other. Rename one of them, then save again. The serving \
             lines were left as they were."
        )
    }
}

/// The Start step's words for a remembered file state (J6):
/// the strongest true statement about the print copy and the
/// last clean check, compared against the document as it
/// stands now. A stale print copy alone says nothing here;
/// the Print step carries that warning.
pub(crate) fn state_sentence(state: Option<&DocState>, current: &str) -> Option<String> {
    let state = state?;
    if state.print_fingerprint.as_deref() == Some(current) {
        return Some("Print copy is current.".to_string());
    }
    if state.checked_fingerprint.as_deref() == Some(current) {
        return Some("Checked, and nothing has changed since.".to_string());
    }
    if state.checked_fingerprint.is_some() {
        return Some("Not checked since the last change.".to_string());
    }
    None
}

/// The Start step's opening lines (adopted item 3): which
/// Sunday this is, and the next thing needed, worked out from
/// the document itself in step order. Nothing here is kept by
/// hand, so it cannot go stale. A date that has passed outranks
/// every other need (J1): the bulletin on screen is for the
/// wrong week until it is fixed. The current file's remembered
/// state (J6) appends its sentence when it has one.
pub(crate) fn start_status(
    doc: &Document,
    announcements_empty: bool,
    today: chrono::NaiveDate,
    state: Option<&DocState>,
) -> (String, String) {
    let long = doc
        .fields
        .get("occasion_date_long")
        .map(|s| s.trim())
        .filter(|s| !s.is_empty());
    // The headline names the day the bulletin is for,
    // weekday included, whatever day of the week it is
    // (occasions). When the stored date does not parse,
    // fall back to the stored long form as before.
    let headline = match doc
        .fields
        .get("occasion_date")
        .and_then(|iso| ob_model::parse_flexible_date(iso.trim(), simple_of(today)))
    {
        Some(d) => format!("Working on: {}", long_form(naive_of(d))),
        None => match long {
            Some(d) => format!("Working on: Sunday, {d}"),
            None => "Working on: a new bulletin".to_string(),
        },
    };
    let date_missing = long.is_none()
        || doc
            .fields
            .get("occasion_date")
            .map(|s| s.trim().is_empty())
            .unwrap_or(true);
    let passed = date_passed_sentence(doc.fields.get("occasion_date").map(String::as_str), today);
    let mut next = if let Some(sentence) = passed {
        sentence
    } else if doc.profile.name.trim().is_empty() {
        "Next: add your church name on this step.".to_string()
    } else if date_missing {
        "Next: set the date on the This Sunday step.".to_string()
    } else if announcements_empty && !doc.no_announcements {
        "Next: add this week's announcements on the Announcements step, \
         or mark there that there are none this week."
            .to_string()
    } else {
        "Ready to print.".to_string()
    };
    if let Some(words) = state_sentence(state, &snapshot(doc)) {
        next = format!("{next} {words}");
    }
    (headline, next)
}

/// True while the bulletin on screen is the starter sample
/// (J7): no file behind it, and no church named yet. Start
/// says so out loud, so a sample is never mistaken for the
/// church's own bulletin.
pub(crate) fn is_starter_sample(has_file: bool, church_name: &str) -> bool {
    !has_file && church_name.trim().is_empty()
}

/// True when a print copy exists but the bulletin has changed
/// since it was made (adopted item 5). The fingerprint is the
/// same snapshot text the undo history keeps.
pub(crate) fn print_copy_stale(fingerprint: Option<&str>, current: &str) -> bool {
    fingerprint.map(|fp| fp != current).unwrap_or(false)
}
