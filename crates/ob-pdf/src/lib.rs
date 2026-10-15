//! PDF export.
//!
//! v0 uses a small hand-rolled PDF 1.4 writer with the core Helvetica fonts
//! so the scaffold has zero dependency risk and perfectly deterministic
//! output. The production path (font embedding, CMYK, PDF/X) is the krilla
//! evaluation in the roadmap; this module's job is correct geometry: what
//! you see on the panel is what lands on the sheet, in milli-point space.
//!
//! Model space is top-left origin; PDF space is bottom-left. The conversion
//! happens here and only here.

use ob_impose::SheetSide;
use ob_model::{Align, Document, Panel, Rect};
use std::fmt::Write as _;

fn pt(v: ob_model::Mpt) -> f32 {
    v as f32 / 1000.0
}

fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        const BS: char = '\u{5C}'; // backslash
        match c {
            '(' | ')' => {
                out.push(BS);
                out.push(c);
            }
            '\u{5C}' => {
                out.push(BS);
                out.push(BS);
            }
            c if (' '..='~').contains(&c) => out.push(c),
            _ => out.push(' '), // v0 core fonts are Latin-1-ish; non-ASCII becomes a space, never a crash.
        }
    }
    out
}

struct PageContent {
    buf: String,
}

impl PageContent {
    fn text(
        &mut self,
        bold: bool,
        size_mpt: ob_model::Mpt,
        x: ob_model::Mpt,
        y_top: ob_model::Mpt,
        page_h: ob_model::Mpt,
        s: &str,
    ) {
        let y = page_h - y_top; // baseline approx: y_top is the top of the line box
        let font = if bold { "F2" } else { "F1" };
        let _ = writeln!(
            self.buf,
            "BT /{} {:.2} Tf 1 0 0 1 {:.2} {:.2} Tm ({}) Tj ET",
            font,
            pt(size_mpt),
            pt(x),
            pt(y),
            esc(s)
        );
    }
    fn rect_stroke(&mut self, r: Rect, page_h: ob_model::Mpt) {
        let y = page_h - r.y - r.h;
        let _ = writeln!(
            self.buf,
            "0.75 w {:.2} {:.2} {:.2} {:.2} re S",
            pt(r.x),
            pt(y),
            pt(r.w),
            pt(r.h)
        );
    }
}

/// Draw one panel's frames into `pc`, offset by (ox, oy) in a space whose
/// total height is `page_h` (panel height for reader PDFs, sheet height for
/// imposed PDFs).
fn draw_panel(
    pc: &mut PageContent,
    doc: &Document,
    shaper: &mut ob_text::shaping::Shaper,
    panel: &Panel,
    ox: ob_model::Mpt,
    oy: ob_model::Mpt,
    page_h: ob_model::Mpt,
) {
    for frame in &panel.frames {
        let fr = Rect::new(
            frame.rect.x + ox,
            frame.rect.y + oy,
            frame.rect.w,
            frame.rect.h,
        );
        if frame.kind == ob_model::FrameKind::Shape {
            pc.rect_stroke(fr, page_h);
            continue;
        }
        let Some(story_id) = &frame.story_id else {
            continue;
        };
        let Some(story) = doc.story(story_id) else {
            continue;
        };
        let mut y = fr.y;
        for para in &story.paragraphs {
            let style = doc
                .style(&para.style)
                .cloned()
                .unwrap_or(ob_model::ParagraphStyle {
                    id: "body".into(),
                    name: "Body".into(),
                    font_size: 11_000,
                    bold: false,
                    align: Align::Left,
                    space_after: 4_000,
                    color: ob_model::Rgb::BLACK,
                    font_family: "Liberation Sans".into(),
                    font_sha256: None,
                });
            let resolved = ob_model::resolve_fields(&para.text, doc);
            let line_h = style.font_size * 13 / 10;
            // Real shaping decides line breaks and widths. The PDF draws
            // core-font Helvetica; Liberation Sans metrics are Arial's and
            // track Helvetica closely, so breaks land where print puts
            // them. Font embedding removes the residue later.
            let spec = ob_text::shaping::FontSpec {
                family: style.font_family.clone(),
                bold: style.bold,
                italic: false,
            };
            let shaped: Vec<(String, ob_model::Mpt)> = if resolved.contains('\t') {
                let line = shaper.expand_order_line(&resolved, fr.w, &spec, style.font_size);
                let w = shaper.measure(&line, &spec, style.font_size);
                vec![(line, w)]
            } else {
                shaper
                    .wrap(&resolved, fr.w, &spec, style.font_size, 1.3)
                    .into_iter()
                    .map(|l| (l.text, l.width))
                    .collect()
            };
            for (line, tw) in shaped {
                if y + line_h > fr.y + fr.h {
                    break; // v0: clip at frame bottom (overflow indicator is a GUI feature)
                }
                y += line_h;
                let x = match style.align {
                    Align::Left => fr.x,
                    Align::Center => fr.x + (fr.w - tw) / 2,
                    Align::Right => fr.x + fr.w - tw,
                };
                pc.text(style.bold, style.font_size, x, y, page_h, &line);
            }
            y += style.space_after;
        }
    }
}

struct Pdf {
    objects: Vec<Vec<u8>>,
}

impl Pdf {
    fn add(&mut self, body: Vec<u8>) -> usize {
        self.objects.push(body);
        // Object 1 = catalog, 2 = pages tree, so stored objects are numbered from 3.
        self.objects.len() + 2
    }
    fn finish(self, page_ids: &[usize], font1: usize, font2: usize) -> Vec<u8> {
        let mut out: Vec<u8> = Vec::new();
        out.extend_from_slice(
            b"%PDF-1.4
",
        );
        // Object 1: catalog, 2: pages tree; content objects follow.
        let pages_id = 2usize;
        let mut bodies: Vec<(usize, Vec<u8>)> = Vec::new();
        bodies.push((
            1,
            format!("<< /Type /Catalog /Pages {} 0 R >>", pages_id).into_bytes(),
        ));
        let kids: Vec<String> = page_ids.iter().map(|i| format!("{} 0 R", i)).collect();
        bodies.push((
            pages_id,
            format!(
                "<< /Type /Pages /Kids [{}] /Count {} >>",
                kids.join(" "),
                page_ids.len()
            )
            .into_bytes(),
        ));
        for (i, obj) in self.objects.iter().enumerate() {
            bodies.push((i + 3, obj.clone()));
        }
        let _ = (font1, font2);
        bodies.sort_by_key(|(n, _)| *n);
        let max_obj_early = bodies.iter().map(|(n, _)| *n).max().unwrap_or(0);
        let mut offsets: Vec<usize> = vec![0; max_obj_early + 1];
        for (num, body) in &bodies {
            offsets[*num] = out.len();
            out.extend_from_slice(
                format!(
                    "{} 0 obj
",
                    num
                )
                .as_bytes(),
            );
            out.extend_from_slice(body);
            out.extend_from_slice(
                b"
endobj
",
            );
        }
        let xref = out.len();
        let max_obj = bodies.iter().map(|(n, _)| *n).max().unwrap_or(0);
        out.extend_from_slice(
            format!(
                "xref
0 {}
",
                max_obj + 1
            )
            .as_bytes(),
        );
        out.extend_from_slice(
            b"0000000000 65535 f 
",
        );
        for off in offsets.iter().skip(1).take(max_obj) {
            out.extend_from_slice(
                format!(
                    "{:010} 00000 n 
",
                    off
                )
                .as_bytes(),
            );
        }
        out.extend_from_slice(
            format!(
                "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n",
                max_obj + 1,
                xref
            )
            .as_bytes(),
        );
        // The EOF marker is exactly two percent signs. (A doubled %%
        // once slipped through here as %%%%EOF, which strict parsers
        // such as lopdf reject; lenient readers had hidden the bug.)
        out.extend_from_slice(b"%%EOF\n");
        out
    }
}

/// Build a PDF from a set of sheet sides (each side becomes one page).
fn build_pdf(doc: &Document, sides: &[SheetSide]) -> Vec<u8> {
    let mut pdf = Pdf {
        objects: Vec::new(),
    };
    // Fonts are objects 3 and 4.
    let f1 = pdf.add(
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>"
            .to_vec(),
    );
    let f2 = pdf.add(
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold /Encoding /WinAnsiEncoding >>"
            .to_vec(),
    );
    let mut shaper = ob_text::shaping::bundled_shaper();
    register_library_fonts(&mut shaper);
    let mut page_ids = Vec::new();
    for side in sides {
        let mut pc = PageContent { buf: String::new() };
        pc.buf.push_str("0.92 0.92 0.92 RG "); // light frame guides color for shapes
        for pl in &side.placements {
            if let Some(panel) = doc.panels.get(pl.panel) {
                draw_panel(&mut pc, doc, &mut shaper, panel, pl.x, pl.y, side.size.h);
            }
        }
        let content = pc.buf.into_bytes();
        let content_id = pdf.add({
            let mut v = format!(
                "<< /Length {} >>
stream
",
                content.len()
            )
            .into_bytes();
            v.extend_from_slice(&content);
            v.extend_from_slice(
                b"
endstream",
            );
            v
        });
        let page_id = pdf.add(
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {:.2} {:.2}] /Resources << /Font << /F1 {} 0 R /F2 {} 0 R >> >> /Contents {} 0 R >>",
                pt(side.size.w),
                pt(side.size.h),
                f1, f2, content_id
            )
            .into_bytes(),
        );
        page_ids.push(page_id);
    }
    pdf.finish(&page_ids, f1, f2)
}

/// The two-sided printing test sheet (one sheet, FRONT and BACK).
///
/// Print it with two-sided printing on, "flip on long edge" if the
/// printer asks. Hold FRONT so it reads right, turn the sheet like a
/// book page: if BACK reads right too, long-edge flipping is correct
/// for that printer; if BACK is upside down, that printer needs
/// short-edge. The app asks the question and remembers the answer.
pub fn export_duplex_test_pdf() -> Vec<u8> {
    use ob_model::{
        Align, ContentRole, Frame, FrameKind, Panel, Paragraph, ParagraphStyle, Rect, Size, Story,
    };
    let page = Size::new(612_000, 792_000); // US Letter portrait
    let frame_rect = Rect::new(54_000, 54_000, page.w - 108_000, page.h - 108_000);
    let side = |id: &str, big: &str, last: &str| -> (Panel, Story) {
        let story = Story {
            id: format!("{id}-story"),
            paragraphs: vec![
                Paragraph::new("test-big", big),
                Paragraph::new(
                    "test-body",
                    "The top of this side is the edge nearest the big word.",
                ),
                Paragraph::new("test-body", last),
            ],
        };
        let panel = Panel {
            id: id.to_string(),
            name: big.to_string(),
            size: page,
            frames: vec![Frame {
                id: format!("{id}-frame"),
                name: "Test sheet".into(),
                kind: FrameKind::Text,
                rect: frame_rect,
                story_id: Some(story.id.clone()),
                role: ContentRole::Standing,
            }],
        };
        (panel, story)
    };
    let (p1, s1) = side(
        "front",
        "FRONT",
        "Print BOTH sides of one sheet, then hold this FRONT so you can read it.",
    );
    let (p2, s2) = side(
        "back",
        "BACK",
        "Turn the sheet over like a book page. In the app, tell us: is this BACK right side up, or upside down?",
    );
    let doc = Document {
        format_version: ob_model::FORMAT_VERSION,
        title: "Two-sided printing test sheet".into(),
        profile: Default::default(),
        fields: Default::default(),
        styles: vec![
            ParagraphStyle {
                id: "test-big".into(),
                name: "Big".into(),
                font_size: 40_000,
                bold: true,
                align: Align::Center,
                space_after: 24_000,
                color: ob_model::Rgb::BLACK,
                font_family: "Liberation Sans".into(),
                font_sha256: None,
            },
            ParagraphStyle {
                id: "test-body".into(),
                name: "Body".into(),
                font_size: 13_000,
                bold: false,
                align: Align::Center,
                space_after: 12_000,
                color: ob_model::Rgb::BLACK,
                font_family: "Liberation Sans".into(),
                font_sha256: None,
            },
        ],
        stories: vec![s1, s2],
        panels: vec![p1, p2],
        sheet: ob_model::SheetSpec {
            preset: "letter-portrait".into(),
            size: page,
        },
    };
    export_reader_pdf(&doc)
}

/// Reader PDF: one panel per page, in reading order (for email/website).
/// Register every font in the Font Library (docs/FONTS.md) with the
/// shaper, so styles naming those families shape with the real font.
/// A missing or unreadable library is not an error: bundled fonts
/// still cover the default styles.
fn register_library_fonts(shaper: &mut ob_text::shaping::Shaper) {
    if let Ok(lib) = ob_fonts::FontLibrary::open(ob_fonts::default_library_dir()) {
        for info in lib.scan() {
            if let Ok(bytes) = lib.read(&info) {
                shaper.register_font(bytes);
            }
        }
    }
}

pub fn export_reader_pdf(doc: &Document) -> Vec<u8> {
    let sides = ob_impose::impose_single(doc);
    build_pdf(doc, &sides)
}

/// Imposed PDF: printer sheets, ready to print duplex and fold.
pub fn export_imposed_pdf(doc: &Document) -> Vec<u8> {
    let sides = ob_impose::impose(doc);
    build_pdf(doc, &sides)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use ob_layout::{build_template, Preset};
    use ob_model::ChurchProfile;

    fn sample_doc() -> Document {
        build_template(
            Preset::LetterBifold,
            ChurchProfile {
                name: "Grace Lutheran Church".into(),
                pastor: "Pastor Lee".into(),
                ..Default::default()
            },
            NaiveDate::from_ymd_opt(2026, 10, 18).unwrap(),
            "Twentieth Sunday after Pentecost",
        )
    }

    #[test]
    fn eof_marker_is_exactly_two_percents() {
        let bytes = export_reader_pdf(&sample_doc());
        let tail = &bytes[bytes.len().saturating_sub(16)..];
        let tail = String::from_utf8_lossy(tail);
        assert!(tail.trim_end().ends_with("%%EOF"), "tail was: {tail:?}");
        assert!(!tail.contains("%%%%"), "doubled EOF marker: {tail:?}");
    }

    #[test]
    fn duplex_test_sheet_has_front_and_back() {
        let bytes = export_duplex_test_pdf();
        assert!(bytes.starts_with(b"%PDF-1.4"));
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.contains("/Count 2"), "two pages: {text:.200}");
        assert!(text.contains("FRONT"), "front missing");
        assert!(text.contains("BACK"), "back missing");
    }

    #[test]
    fn reader_pdf_looks_like_pdf() {
        let bytes = export_reader_pdf(&sample_doc());
        assert!(bytes.starts_with(b"%PDF-1.4"));
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.contains("Grace Lutheran Church"), "cover text missing");
        assert!(
            text.contains("/Count 4"),
            "reader PDF must have 4 pages: {text:.200}"
        );
    }

    #[test]
    fn imposed_pdf_has_two_sheet_sides() {
        let bytes = export_imposed_pdf(&sample_doc());
        let text = String::from_utf8_lossy(&bytes);
        assert!(
            text.contains("/Count 2"),
            "bifold imposes to 2 sides: {text:.200}"
        );
    }

    #[test]
    fn reader_pages_are_panel_sized() {
        let doc = sample_doc();
        let pdf = String::from_utf8_lossy(&export_reader_pdf(&doc)).to_string();
        // Bi-fold panel is 5.5 x 8.5 in = 396 x 612 pt; sheet is 792 x 612.
        assert!(
            pdf.contains("/MediaBox [0 0 396.00 612.00]"),
            "reader pages must be panel-sized"
        );
        assert!(
            !pdf.contains("/MediaBox [0 0 792.00 612.00]"),
            "reader pages must not be sheet-sized"
        );
        let imposed = String::from_utf8_lossy(&export_imposed_pdf(&doc)).to_string();
        assert!(
            imposed.contains("/MediaBox [0 0 792.00 612.00]"),
            "imposed pages must be sheet-sized"
        );
    }
}
