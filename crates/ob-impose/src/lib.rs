//! Imposition: turning reader panels into printer sheets.
//!
//! This is the #1 thing volunteers get wrong in Publisher, so it is a small,
//! pure, heavily tested crate with no other responsibilities.
//!
//! Panel order convention: `doc.panels` is in READING order.
//! - Bi-fold (4 panels): [Cover, InsideLeft, InsideRight, Back]
//! - Tri-fold (6 panels): [FrontCover, InsideFlap, InsideA, InsideB, InsideC, BackCover]
//! - Saddle/booklet (n pages, n % 4 == 0): plain page order.

use ob_model::{inch, Document, Mpt, Panel, Size};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placement {
    pub panel: usize,
    pub x: Mpt,
    pub y: Mpt,
    /// Degrees clockwise. v0 impositions are all upright (0).
    pub rotation: u16,
    pub w: Mpt,
    pub h: Mpt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SheetSide {
    pub size: Size,
    pub placements: Vec<Placement>,
}

fn place(panel: usize, x: Mpt, y: Mpt, size: Size) -> Placement {
    Placement {
        panel,
        x,
        y,
        rotation: 0,
        w: size.w,
        h: size.h,
    }
}

/// The panels a fixed-count imposition works with: the real
/// ones, padded with blank pages when a file has too few, so a
/// shared or hand-edited .obd can never crash the printer path
/// (closing audit: imposition used to assert and panic). Extra
/// panels beyond the count are returned separately and print
/// as their own single sheets.
fn padded_panels(doc: &Document, count: usize) -> (Vec<Panel>, Vec<Panel>) {
    let size = doc
        .panels
        .first()
        .map(|p| p.size)
        .unwrap_or(Size::new(doc.sheet.size.w / 2, doc.sheet.size.h));
    let mut panels: Vec<Panel> = doc.panels.iter().take(count).cloned().collect();
    while panels.len() < count {
        panels.push(Panel {
            id: format!("blank-pad-{}", panels.len()),
            name: "Blank page".into(),
            size,
            frames: Vec::new(),
            background_image: None,
            guides: Vec::new(),
        });
    }
    let extras: Vec<Panel> = doc.panels.iter().skip(count).cloned().collect();
    (panels, extras)
}

/// Extra panels print as single sheets after the folded ones.
fn extra_singles(doc: &Document, extras: &[Panel], offset: usize) -> Vec<SheetSide> {
    if extras.is_empty() {
        return Vec::new();
    }
    let mut d = doc.clone();
    d.panels = extras.to_vec();
    let mut sides = impose_single(&d);
    for side in &mut sides {
        for pl in &mut side.placements {
            pl.panel += offset;
        }
    }
    sides
}

/// Two sheet sides for a bi-fold. Duplex assumption: flip on the long edge
/// (the standard office-copier setting), so both sides are upright.
/// Outside: [Back | Cover]. Inside: [InsideLeft | InsideRight].
pub fn impose_bifold(doc: &Document) -> Vec<SheetSide> {
    let (_, extras) = padded_panels(doc, 4);
    let sheet = doc.sheet.size;
    let pw = sheet.w / 2;
    let ps = Size::new(pw, sheet.h);
    let mut sides = vec![
        SheetSide {
            size: sheet,
            placements: vec![place(3, 0, 0, ps), place(0, pw, 0, ps)],
        },
        SheetSide {
            size: sheet,
            placements: vec![place(1, 0, 0, ps), place(2, pw, 0, ps)],
        },
    ];
    sides.extend(extra_singles(doc, &extras, 4));
    sides
}

/// Two sheet sides for a tri-fold (letter, landscape).
/// The fold-in flap is 1/8" narrower so it tucks inside; the lost width comes
/// off the flap's placement. Outside (left to right): [InsideFlap | Back | Front].
/// Inside (left to right): [InsideA | InsideB | InsideC].
/// NOTE: validate against the church copier with the duplex test sheet before
/// trusting this in production (see ROADMAP, duplex wizard).
pub fn impose_trifold(doc: &Document) -> Vec<SheetSide> {
    let (_, extras) = padded_panels(doc, 6);
    let sheet = doc.sheet.size;
    let narrow = inch(0.125);
    let wide = (sheet.w + narrow) / 3;
    let flap_w = wide - narrow;
    // Outside: flap (narrow) | back (wide) | front (wide)
    let outside = vec![
        place(1, 0, 0, Size::new(flap_w, sheet.h)),
        place(5, flap_w, 0, Size::new(wide, sheet.h)),
        place(
            0,
            flap_w + wide,
            0,
            Size::new(sheet.w - flap_w - wide, sheet.h),
        ),
    ];
    // Inside spread reads left to right A B C; C is the narrow panel position.
    let inside = vec![
        place(2, 0, 0, Size::new(wide, sheet.h)),
        place(3, wide, 0, Size::new(wide, sheet.h)),
        place(4, 2 * wide, 0, Size::new(sheet.w - 2 * wide, sheet.h)),
    ];
    let mut sides = vec![
        SheetSide {
            size: sheet,
            placements: outside,
        },
        SheetSide {
            size: sheet,
            placements: inside,
        },
    ];
    sides.extend(extra_singles(doc, &extras, 6));
    sides
}

/// Saddle-stitch booklet imposition for a portrait document whose panels are
/// pages. Sheet is two pages wide. Standard order, e.g. 8 pages:
/// side 1: [8|1], side 2: [2|7], side 3: [6|3], side 4: [4|5].
pub fn impose_saddle(doc: &Document) -> Vec<SheetSide> {
    // Pad to a multiple of four (the renderer pairs this with
    // padded_document, so padded indices resolve).
    let n = if doc.panels.is_empty() {
        return vec![SheetSide {
            size: doc.sheet.size,
            placements: Vec::new(),
        }];
    } else {
        doc.panels.len().div_ceil(4) * 4
    };
    let page = doc.panels[0].size;
    let sheet = Size::new(page.w * 2, page.h);
    let mut sides = Vec::new();
    let mut lo = 0usize; // 0-based page index from the front
    let mut hi = n - 1; // from the back
    while lo < hi {
        sides.push(SheetSide {
            size: sheet,
            placements: vec![place(hi, 0, 0, page), place(lo, page.w, 0, page)],
        });
        lo += 1;
        hi -= 1;
        sides.push(SheetSide {
            size: sheet,
            placements: vec![place(lo, 0, 0, page), place(hi, page.w, 0, page)],
        });
        lo += 1;
        hi -= 1;
    }
    sides
}

/// Plain portrait/multipage: one panel per sheet side, reading order.
pub fn impose_single(doc: &Document) -> Vec<SheetSide> {
    doc.panels
        .iter()
        .enumerate()
        .map(|(i, p)| SheetSide {
            size: p.size,
            placements: vec![place(i, 0, 0, p.size)],
        })
        .collect()
}

/// Dispatch on the document's preset id.
/// A copy of the document with panels padded (blank pages) to
/// what its preset's imposition expects. Renderers must lay out
/// THIS document with impose()'s placements, so every placed
/// index exists. A file with the wrong number of panels prints
/// with blanks instead of crashing (closing audit).
pub fn padded_document(doc: &Document) -> Document {
    let mut d = doc.clone();
    let target = match doc.sheet.preset.as_str() {
        "letter-bifold" | "legal-bifold" | "tabloid-bifold" => 4,
        "letter-trifold" => 6,
        "saddle-booklet" => {
            if doc.panels.is_empty() {
                0
            } else {
                doc.panels.len().div_ceil(4) * 4
            }
        }
        _ => doc.panels.len(),
    };
    if d.panels.len() < target {
        let size = d
            .panels
            .first()
            .map(|p| p.size)
            .unwrap_or(Size::new(d.sheet.size.w / 2, d.sheet.size.h));
        while d.panels.len() < target {
            d.panels.push(Panel {
                id: format!("blank-pad-{}", d.panels.len()),
                name: "Blank page".into(),
                size,
                frames: Vec::new(),
                background_image: None,
                guides: Vec::new(),
            });
        }
    }
    d
}

pub fn impose(doc: &Document) -> Vec<SheetSide> {
    match doc.sheet.preset.as_str() {
        "letter-bifold" | "legal-bifold" | "tabloid-bifold" => impose_bifold(doc),
        "letter-trifold" => impose_trifold(doc),
        "saddle-booklet" => impose_saddle(doc),
        _ => impose_single(doc),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ob_model::*;

    #[test]
    fn mismatched_panel_counts_never_panic() {
        // Closing-audit repro: shared files whose panel count
        // disagrees with the preset used to crash imposition.
        let sheet = Size::new(inch(11.0), inch(8.5));
        let panel = Size::new(inch(5.5), inch(8.5));
        for (preset, n) in [
            ("letter-bifold", 3usize),
            ("letter-bifold", 5),
            ("letter-trifold", 4),
            ("saddle-booklet", 0),
            ("saddle-booklet", 5),
            ("saddle-booklet", 15),
        ] {
            let doc = doc_with(preset, sheet, n, panel);
            let sides = impose(&doc);
            assert!(!sides.is_empty(), "{preset} x{n} produced nothing");
            let padded = padded_document(&doc);
            for side in &sides {
                for pl in &side.placements {
                    assert!(
                        pl.panel < padded.panels.len(),
                        "{preset} x{n}: placement {} beyond padded {}",
                        pl.panel,
                        padded.panels.len()
                    );
                }
            }
        }
        // The saddle 15-pager pads to 16 pages = 8 sheet sides.
        let doc = doc_with("saddle-booklet", sheet, 15, panel);
        assert_eq!(padded_document(&doc).panels.len(), 16);
        assert_eq!(impose(&doc).len(), 8);
    }

    fn doc_with(preset: &str, sheet: Size, panels: usize, panel: Size) -> Document {
        Document {
            format_version: FORMAT_VERSION,
            title: "t".into(),
            profile: ChurchProfile::default(),
            fields: Default::default(),
            styles: vec![],
            stories: vec![],
            panels: (0..panels)
                .map(|i| Panel {
                    id: format!("p{i}"),
                    name: format!("P{i}"),
                    size: panel,
                    frames: vec![],
                    background_image: None,
                    guides: Vec::new(),
                })
                .collect(),
            sheet: SheetSpec {
                preset: preset.into(),
                size: sheet,
            },
            no_announcements: false,
            assets: Vec::new(),
            asset_bytes: Default::default(),
            occasion_time: None,
            wanted_fonts: Vec::new(),
            participants: Default::default(),
        }
    }

    #[test]
    fn bifold_outside_is_back_then_cover() {
        let d = doc_with(
            "letter-bifold",
            Size::new(inch(11.0), inch(8.5)),
            4,
            Size::new(inch(5.5), inch(8.5)),
        );
        let sides = impose_bifold(&d);
        assert_eq!(sides.len(), 2);
        let out: Vec<usize> = sides[0].placements.iter().map(|p| p.panel).collect();
        let inn: Vec<usize> = sides[1].placements.iter().map(|p| p.panel).collect();
        assert_eq!(out, vec![3, 0]);
        assert_eq!(inn, vec![1, 2]);
        assert_eq!(sides[0].placements[1].x, inch(5.5));
    }

    #[test]
    fn trifold_flap_is_narrower() {
        let d = doc_with(
            "letter-trifold",
            Size::new(inch(11.0), inch(8.5)),
            6,
            Size::new(inch(11.0 / 3.0), inch(8.5)),
        );
        let sides = impose_trifold(&d);
        let flap = sides[0].placements[0];
        let back = sides[0].placements[1];
        assert_eq!(flap.panel, 1);
        assert!(
            flap.w < back.w,
            "flap {} must be narrower than back {}",
            flap.w,
            back.w
        );
        let total: Mpt = sides[0].placements.iter().map(|p| p.w).sum();
        assert_eq!(total, inch(11.0));
    }

    #[test]
    fn saddle_eight_pages() {
        let page = Size::new(inch(5.5), inch(8.5));
        let d = doc_with("saddle-booklet", Size::new(inch(11.0), inch(8.5)), 8, page);
        let sides = impose_saddle(&d);
        let order: Vec<Vec<usize>> = sides
            .iter()
            .map(|s| s.placements.iter().map(|p| p.panel).collect())
            .collect();
        assert_eq!(order, vec![vec![7, 0], vec![1, 6], vec![5, 2], vec![3, 4]]);
    }
}
