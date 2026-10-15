//! Imposition: turning reader panels into printer sheets.
//!
//! This is the #1 thing volunteers get wrong in Publisher, so it is a small,
//! pure, heavily tested crate with no other responsibilities.
//!
//! Panel order convention: `doc.panels` is in READING order.
//! - Bi-fold (4 panels): [Cover, InsideLeft, InsideRight, Back]
//! - Tri-fold (6 panels): [FrontCover, InsideFlap, InsideA, InsideB, InsideC, BackCover]
//! - Saddle/booklet (n pages, n % 4 == 0): plain page order.

use ob_model::{inch, Document, Mpt, Size};

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

/// Two sheet sides for a bi-fold. Duplex assumption: flip on the long edge
/// (the standard office-copier setting), so both sides are upright.
/// Outside: [Back | Cover]. Inside: [InsideLeft | InsideRight].
pub fn impose_bifold(doc: &Document) -> Vec<SheetSide> {
    assert_eq!(doc.panels.len(), 4, "bi-fold needs exactly 4 panels");
    let sheet = doc.sheet.size;
    let pw = sheet.w / 2;
    let ps = Size::new(pw, sheet.h);
    vec![
        SheetSide {
            size: sheet,
            placements: vec![place(3, 0, 0, ps), place(0, pw, 0, ps)],
        },
        SheetSide {
            size: sheet,
            placements: vec![place(1, 0, 0, ps), place(2, pw, 0, ps)],
        },
    ]
}

/// Two sheet sides for a tri-fold (letter, landscape).
/// The fold-in flap is 1/8" narrower so it tucks inside; the lost width comes
/// off the flap's placement. Outside (left to right): [InsideFlap | Back | Front].
/// Inside (left to right): [InsideA | InsideB | InsideC].
/// NOTE: validate against the church copier with the duplex test sheet before
/// trusting this in production (see ROADMAP, duplex wizard).
pub fn impose_trifold(doc: &Document) -> Vec<SheetSide> {
    assert_eq!(doc.panels.len(), 6, "tri-fold needs exactly 6 panels");
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
    vec![
        SheetSide {
            size: sheet,
            placements: outside,
        },
        SheetSide {
            size: sheet,
            placements: inside,
        },
    ]
}

/// Saddle-stitch booklet imposition for a portrait document whose panels are
/// pages. Sheet is two pages wide. Standard order, e.g. 8 pages:
/// side 1: [8|1], side 2: [2|7], side 3: [6|3], side 4: [4|5].
pub fn impose_saddle(doc: &Document) -> Vec<SheetSide> {
    let n = doc.panels.len();
    assert!(
        n % 4 == 0 && n > 0,
        "saddle-stitch needs a multiple of 4 pages, got {n}"
    );
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
                })
                .collect(),
            sheet: SheetSpec {
                preset: preset.into(),
                size: sheet,
            },
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
