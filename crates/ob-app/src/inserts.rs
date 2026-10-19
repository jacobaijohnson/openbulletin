//! Loose-sheet (insert) helpers: making and swapping
//! inserts, the words that describe a sheet, and the
//! findings an insert earns at Check.

use crate::check::*;
use crate::*;

/// Exchange a document's panels, stories, and styles
/// with one of its loose sheets' (R-3): the canvas swap.
/// The swap is its own inverse, so entering and leaving
/// a sheet's edit are the same act. Nothing is copied
/// and nothing is duplicated: the parts live in exactly
/// one of the two places at any moment. False when the
/// document holds no sheet under that id.
pub(crate) fn swap_insert_parts(doc: &mut Document, id: &str) -> bool {
    let Some(pos) = doc.inserts.iter().position(|i| i.id == id) else {
        return false;
    };
    std::mem::swap(&mut doc.panels, &mut doc.inserts[pos].panels);
    std::mem::swap(&mut doc.stories, &mut doc.inserts[pos].stories);
    std::mem::swap(&mut doc.styles, &mut doc.inserts[pos].styles);
    true
}

/// An id no sheet in this document already uses.
pub(crate) fn fresh_insert_id(doc: &Document) -> String {
    let mut n = doc.inserts.len() + 1;
    loop {
        let id = format!("sheet-{n}");
        if !doc.inserts.iter().any(|i| i.id == id) {
            return id;
        }
        n += 1;
    }
}

/// A new loose sheet: one panel of the half-letter
/// shape, one text box, one empty story, this week only
/// until the office says otherwise (R-3).
pub(crate) fn new_insert(id: &str, name: &str) -> ob_model::Insert {
    let size = ob_model::insert_sheet_size("half-letter").expect("half-letter is a sheet shape");
    let m = 36_000; // a half-inch margin, in milli-points
    ob_model::Insert {
        id: id.to_string(),
        name: name.to_string(),
        life: ob_model::InsertLife::ThisWeekOnly,
        keep_through: None,
        panels: vec![ob_model::Panel {
            id: format!("{id}-p1"),
            name: name.to_string(),
            size,
            frames: vec![ob_model::Frame {
                id: format!("{id}-f1"),
                name: "Words".into(),
                kind: ob_model::FrameKind::Text,
                rect: ob_model::Rect::new(m, m, size.w - 2 * m, size.h - 2 * m),
                story_id: Some(format!("{id}-s1")),
                role: ob_model::ContentRole::Standing,
                picture: None,
                music: None,
            }],
            guides: vec![],
            background_image: None,
        }],
        stories: vec![ob_model::Story {
            id: format!("{id}-s1"),
            paragraphs: vec![ob_model::Paragraph::new("body", "")],
            keep_through: None,
        }],
        styles: ob_model::default_styles(),
        sheet: ob_model::InsertSheet::default(),
    }
}

/// A sheet's name as a file name's middle: letters,
/// numbers, spaces, and dashes stay; anything a file
/// system would trip on becomes a dash.
pub(crate) fn insert_file_stem(name: &str) -> String {
    let stem: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == ' ' || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let stem = stem.trim();
    if stem.is_empty() {
        "Loose sheet".to_string()
    } else {
        stem.to_string()
    }
}

/// A sheet's life in the words the office hears.
pub(crate) fn insert_life_words(ins: &ob_model::Insert) -> String {
    match ins.life {
        ob_model::InsertLife::Standing => "Standing: rides every week".to_string(),
        ob_model::InsertLife::ThisWeekOnly => "This week only".to_string(),
        ob_model::InsertLife::Dated => match ins.keep_through {
            Some(d) => format!("Until {}", ob_layout::long_date(naive_of(d))),
            None => "Until a date (no date set yet)".to_string(),
        },
    }
}

/// A sheet's size in words: "Half letter".
pub(crate) fn insert_size_words(sheet: &ob_model::InsertSheet) -> String {
    for (key, words) in ob_model::INSERT_SHEET_SHAPES {
        if key == sheet.preset {
            return words.split(" (").next().unwrap_or(words).to_string();
        }
    }
    sheet.preset.clone()
}

/// How many copies, in words: zero is the usual case
/// and means the sheet prints with the bulletin run.
pub(crate) fn insert_copies_words(sheet: &ob_model::InsertSheet) -> String {
    match sheet.copies {
        0 => "as many as the bulletin".to_string(),
        1 => "1 copy".to_string(),
        n => format!("{n} copies"),
    }
}

/// The paper, in words: no note means plain paper.
pub(crate) fn insert_paper_words(sheet: &ob_model::InsertSheet) -> String {
    let note = sheet.paper_note.trim();
    if note.is_empty() {
        "plain paper".to_string()
    } else {
        note.to_string()
    }
}

/// The Check findings only loose sheets can have (R-3):
/// a sheet with no words would print as a blank sheet of
/// colored paper, and a dated sheet whose date has
/// passed is still riding until Next Sunday takes it
/// out, so Check names it before it prints stale. Both
/// aim at the Loose sheets window, where the fix lives.
pub(crate) fn insert_findings(doc: &Document) -> Vec<CheckNote> {
    let mut out = Vec::new();
    let doc_date = doc
        .fields
        .get("occasion_date")
        .and_then(|iso| chrono::NaiveDate::parse_from_str(iso.trim(), "%Y-%m-%d").ok())
        .map(simple_of);
    for ins in &doc.inserts {
        if !ins.has_words() {
            out.push(CheckNote::to_inserts(format!(
                "Check: the loose sheet '{}' has no words yet.",
                ins.name
            )));
        }
        if ins.life == ob_model::InsertLife::Dated {
            if let (Some(until), Some(date)) = (ins.keep_through, doc_date) {
                if until.is_valid() && until < date {
                    out.push(CheckNote::to_inserts(format!(
                        "Check: the loose sheet '{}' was kept through {}, which has passed.",
                        ins.name,
                        ob_layout::long_date(naive_of(until))
                    )));
                }
            }
        }
    }
    out
}
