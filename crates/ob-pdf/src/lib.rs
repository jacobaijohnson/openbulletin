//! PDF export.
//!
//! A small hand-rolled PDF 1.4 writer. Fonts are embedded as TrueType
//! from the same Font Library bytes the shaper measured with, so what
//! the Check step approves is what prints (docs/FONTS.md). A face whose
//! license does not allow it to travel, or that cannot be found, falls
//! back to the core Helvetica fonts for that face only. Text is encoded
//! as WinAnsi; glyphs outside WinAnsi wait on CID embedding and render
//! as spaces rather than crashing. CMYK and PDF/X remain roadmap items.
//!
//! Model space is top-left origin; PDF space is bottom-left. The conversion
//! happens here and only here.

use ob_impose::SheetSide;
use ob_model::{Align, Document, Panel, Rect};
use std::fmt::Write as _;

fn pt(v: ob_model::Mpt) -> f32 {
    v as f32 / 1000.0
}

/// The printable punctuation Windows-1252 adds in 0x80..=0x9F.
const CP1252_SPECIALS: &[(char, u8)] = &[
    ('\u{20AC}', 0x80),
    ('\u{201A}', 0x82),
    ('\u{0192}', 0x83),
    ('\u{201E}', 0x84),
    ('\u{2026}', 0x85),
    ('\u{2020}', 0x86),
    ('\u{2021}', 0x87),
    ('\u{02C6}', 0x88),
    ('\u{2030}', 0x89),
    ('\u{0160}', 0x8A),
    ('\u{2039}', 0x8B),
    ('\u{0152}', 0x8C),
    ('\u{017D}', 0x8E),
    ('\u{2018}', 0x91),
    ('\u{2019}', 0x92),
    ('\u{201C}', 0x93),
    ('\u{201D}', 0x94),
    ('\u{2022}', 0x95),
    ('\u{2013}', 0x96),
    ('\u{2014}', 0x97),
    ('\u{02DC}', 0x98),
    ('\u{2122}', 0x99),
    ('\u{0161}', 0x9A),
    ('\u{203A}', 0x9B),
    ('\u{0153}', 0x9C),
    ('\u{017E}', 0x9E),
    ('\u{0178}', 0x9F),
];

/// The WinAnsi (Windows-1252) byte for a character, when it has one.
/// Latin-1 code points map to themselves; the specials table covers
/// the punctuation block; everything else has no WinAnsi byte.
fn winansi_byte(c: char) -> Option<u8> {
    match c {
        ' '..='~' => Some(c as u8),
        '\u{A0}'..='\u{FF}' => Some(c as u8),
        _ => CP1252_SPECIALS
            .iter()
            .find(|(ch, _)| *ch == c)
            .map(|(_, b)| *b),
    }
}

/// The character a WinAnsi byte stands for (for width tables).
fn winansi_char(b: u8) -> Option<char> {
    match b {
        0x20..=0x7E => Some(b as char),
        0xA0..=0xFF => char::from_u32(b as u32),
        _ => CP1252_SPECIALS
            .iter()
            .find(|(_, byte)| *byte == b)
            .map(|(ch, _)| *ch),
    }
}

/// Escape text as a PDF literal string in WinAnsi bytes. Bytes above
/// ASCII travel as octal escapes; characters with no WinAnsi byte
/// become spaces, never a crash.
fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match winansi_byte(c) {
            Some(b @ (b'(' | b')' | b'\\')) => {
                out.push('\\');
                out.push(b as char);
            }
            Some(b) if b < 0x80 => out.push(b as char),
            Some(b) => {
                let _ = write!(out, "\\{:03o}", b);
            }
            None => out.push(' '),
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
        font: &str,
        size_mpt: ob_model::Mpt,
        x: ob_model::Mpt,
        y_top: ob_model::Mpt,
        page_h: ob_model::Mpt,
        s: &str,
    ) {
        let y = page_h - y_top; // baseline approx: y_top is the top of the line box
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
#[allow(clippy::too_many_arguments)]
fn draw_panel(
    pc: &mut PageContent,
    doc: &Document,
    shaper: &mut ob_text::shaping::Shaper,
    fonts: &FontBook,
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
                    italic: false,
                    align: Align::Left,
                    space_after: 4_000,
                    color: ob_model::Rgb::BLACK,
                    font_family: "Liberation Sans".into(),
                    font_sha256: None,
                });
            let resolved = ob_model::resolve_fields(&para.text, doc);
            let line_h = style.font_size * 13 / 10;
            // Real shaping decides line breaks and widths, and the PDF
            // embeds the very font bytes that were shaped with, so the
            // breaks land where print puts them.
            let spec = ob_text::shaping::FontSpec {
                family: style.font_family.clone(),
                bold: style.bold,
                italic: style.italic,
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
                pc.text(
                    &fonts.resource_for(&style),
                    style.font_size,
                    x,
                    y,
                    page_h,
                    &line,
                );
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

/// Embedded faces for one export. Each face is a real TrueType font
/// embedded from the same library bytes the shaper measured with,
/// keyed by (family, bold, italic). Faces that cannot be found, or
/// whose license does not let them travel, are absent here and fall
/// back to the core fonts at draw time.
struct FontBook {
    faces: std::collections::HashMap<(String, bool, bool), String>,
    /// Extra resource entries for the page dictionary.
    resources: String,
}

impl FontBook {
    fn build(doc: &Document, pdf: &mut Pdf, library: &[(ob_fonts::FontInfo, Vec<u8>)]) -> FontBook {
        let mut wanted: Vec<(String, bool, bool)> = Vec::new();
        for style in &doc.styles {
            let key = (style.font_family.clone(), style.bold, style.italic);
            if !wanted.contains(&key) {
                wanted.push(key);
            }
        }
        let fallback = ("Liberation Sans".to_string(), false, false);
        if !wanted.contains(&fallback) {
            wanted.push(fallback);
        }
        // Candidate bytes by face: the library wins over the bundled
        // set, and only faces whose rights let them travel are used.
        let mut sources: std::collections::HashMap<(String, bool, bool), Vec<u8>> =
            std::collections::HashMap::new();
        for (name, bytes) in ob_text::shaping::bundled_font_files() {
            if let Ok(info) = ob_fonts::inspect(bytes, name) {
                if info.rights.may_package() {
                    sources.insert((info.family, info.bold, info.italic), bytes.to_vec());
                }
            }
        }
        for (info, bytes) in library {
            if info.rights.may_package() {
                sources.insert((info.family.clone(), info.bold, info.italic), bytes.clone());
            }
        }
        let mut faces = std::collections::HashMap::new();
        let mut resources = String::new();
        let mut n = 0;
        for (family, bold, italic) in &wanted {
            let key = (family.clone(), *bold, *italic);
            let Some(bytes) = sources.get(&key) else {
                continue;
            };
            let Ok(face) = ttf_parser::Face::parse(bytes, 0) else {
                continue;
            };
            n += 1;
            let resource = format!("FE{n}");
            let font_obj = add_embedded_font(pdf, &face, bytes);
            resources.push_str(&format!(" /{} {} 0 R", resource, font_obj));
            faces.insert(key, resource);
        }
        FontBook { faces, resources }
    }

    /// The resource name to draw a style with: its exact face when
    /// embedded, its family's plain face when not, else core fonts.
    fn resource_for(&self, style: &ob_model::ParagraphStyle) -> String {
        let exact = (style.font_family.clone(), style.bold, style.italic);
        if let Some(r) = self.faces.get(&exact) {
            return r.clone();
        }
        let plain = (style.font_family.clone(), false, false);
        if let Some(r) = self.faces.get(&plain) {
            return r.clone();
        }
        if style.bold {
            "F2".to_string()
        } else {
            "F1".to_string()
        }
    }
}

/// Add the three objects an embedded TrueType face needs (font
/// stream, descriptor, font dictionary) and return the font object.
fn add_embedded_font(pdf: &mut Pdf, face: &ttf_parser::Face, bytes: &[u8]) -> usize {
    let upem = face.units_per_em() as i32;
    let scale = |v: i16| v as i32 * 1000 / upem;
    let name = postscript_name(face);
    let mut stream = format!(
        "<< /Length {} /Length1 {} >>
stream
",
        bytes.len(),
        bytes.len()
    )
    .into_bytes();
    stream.extend_from_slice(bytes);
    stream.extend_from_slice(
        b"
endstream",
    );
    let stream_id = pdf.add(stream);
    let bb = face.global_bounding_box();
    let cap = face
        .capital_height()
        .map(scale)
        .unwrap_or_else(|| scale(face.ascender()) * 7 / 10);
    let descriptor = format!(
        "<< /Type /FontDescriptor /FontName /{} /Flags 32 /FontBBox [{} {} {} {}] /ItalicAngle {:.1} /Ascent {} /Descent {} /CapHeight {} /StemV 80 /FontFile2 {} 0 R >>",
        name,
        scale(bb.x_min),
        scale(bb.y_min),
        scale(bb.x_max),
        scale(bb.y_max),
        face.italic_angle(),
        scale(face.ascender()),
        scale(face.descender()),
        cap,
        stream_id
    );
    let descriptor_id = pdf.add(descriptor.into_bytes());
    let widths: Vec<String> = (32u8..=255)
        .map(|b| {
            winansi_char(b)
                .and_then(|c| face.glyph_index(c))
                .and_then(|g| face.glyph_hor_advance(g))
                .map(|a| a as i32 * 1000 / upem)
                .unwrap_or(0)
                .to_string()
        })
        .collect();
    let font = format!(
        "<< /Type /Font /Subtype /TrueType /BaseFont /{} /FirstChar 32 /LastChar 255 /Widths [{}] /FontDescriptor {} 0 R /Encoding /WinAnsiEncoding >>",
        name,
        widths.join(" "),
        descriptor_id
    );
    pdf.add(font.into_bytes())
}

/// The font's PostScript name, sanitized for a PDF name object.
fn postscript_name(face: &ttf_parser::Face) -> String {
    let mut found: Option<String> = None;
    for name in face.names() {
        if name.name_id == ttf_parser::name_id::POST_SCRIPT_NAME {
            if let Some(text) = name.to_string() {
                found = Some(text);
                break;
            }
        }
    }
    let raw = found.unwrap_or_else(|| "OpenBulletinFont".to_string());
    let clean: String = raw
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
        .collect();
    if clean.is_empty() {
        "OpenBulletinFont".to_string()
    } else {
        clean
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
    let library = library_font_bytes();
    let fonts = FontBook::build(doc, &mut pdf, &library);
    let mut shaper = ob_text::shaping::bundled_shaper();
    register_library_fonts(&mut shaper, &library);
    let mut page_ids = Vec::new();
    for side in sides {
        let mut pc = PageContent { buf: String::new() };
        pc.buf.push_str("0.92 0.92 0.92 RG "); // light frame guides color for shapes
        for pl in &side.placements {
            if let Some(panel) = doc.panels.get(pl.panel) {
                draw_panel(
                    &mut pc,
                    doc,
                    &mut shaper,
                    &fonts,
                    panel,
                    pl.x,
                    pl.y,
                    side.size.h,
                );
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
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {:.2} {:.2}] /Resources << /Font << /F1 {} 0 R /F2 {} 0 R{} >> >> /Contents {} 0 R >>",
                pt(side.size.w),
                pt(side.size.h),
                f1, f2, fonts.resources, content_id
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
            background_image: None,
            guides: Vec::new(),
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
                italic: false,
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
                italic: false,
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
fn register_library_fonts(
    shaper: &mut ob_text::shaping::Shaper,
    library: &[(ob_fonts::FontInfo, Vec<u8>)],
) {
    for (_, bytes) in library {
        shaper.register_font(bytes.clone());
    }
}

/// The Font Library's fonts, read once per export: the embedder
/// and the shaper used to each scan and read the whole library
/// (closing audit).
fn library_font_bytes() -> Vec<(ob_fonts::FontInfo, Vec<u8>)> {
    let mut out = Vec::new();
    if let Ok(lib) = ob_fonts::FontLibrary::open(ob_fonts::default_library_dir()) {
        for info in lib.scan() {
            if let Ok(bytes) = lib.read(&info) {
                out.push((info, bytes));
            }
        }
    }
    out
}

pub fn export_reader_pdf(doc: &Document) -> Vec<u8> {
    let sides = ob_impose::impose_single(doc);
    build_pdf(doc, &sides)
}

/// Imposed PDF: printer sheets, ready to print duplex and fold.
pub fn export_imposed_pdf(doc: &Document) -> Vec<u8> {
    // Lay out the padded document so a file with the wrong
    // number of panels prints with blanks, never crashes and
    // never drops a page (closing audit).
    let padded = ob_impose::padded_document(doc);
    let sides = ob_impose::impose(&padded);
    build_pdf(&padded, &sides)
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
    fn booklet_exports_eight_sheet_sides() {
        let doc = ob_layout::build_template(
            ob_layout::Preset::HalfLetterBooklet,
            ob_model::ChurchProfile::default(),
            chrono::NaiveDate::from_ymd_opt(2026, 10, 18).expect("date"),
            "Sunday",
        );
        let pdf = export_imposed_pdf(&doc);
        let parsed = lopdf::Document::load_mem(&pdf).expect("lopdf parses the booklet");
        assert_eq!(parsed.get_pages().len(), 8);
    }

    #[test]
    fn exports_embed_the_library_fonts() {
        let doc = sample_doc();
        let pdf = export_reader_pdf(&doc);
        let text = String::from_utf8_lossy(&pdf);
        assert!(
            text.contains("/Subtype /TrueType"),
            "embedded TrueType faces"
        );
        assert!(text.contains("/FontFile2"), "font programs embedded");
        assert!(
            text.contains("/BaseFont /LiberationSans"),
            "real font names"
        );
        // And a strict parser accepts the file with its pages intact.
        let parsed = lopdf::Document::load_mem(&pdf).expect("lopdf parses the export");
        assert_eq!(parsed.get_pages().len(), 4);
    }

    #[test]
    fn embedded_widths_match_the_real_font() {
        // Liberation Sans at 2048 units per em, verified against the
        // font file itself: space advances 569 units (277 thousandths
        // scaled) and A advances 1366 (666). The embedded /Widths
        // table must carry the real font's own numbers, not
        // core-font stand-ins.
        let doc = sample_doc();
        let pdf = export_reader_pdf(&doc);
        let text = String::from_utf8_lossy(&pdf);
        let mut arrays: Vec<Vec<i32>> = Vec::new();
        let mut rest = text.as_ref();
        while let Some(start) = rest.find("/Widths [") {
            let body = &rest[start + "/Widths [".len()..];
            let end = body.find(']').expect("widths end");
            arrays.push(
                body[..end]
                    .split_whitespace()
                    .map(|w| w.parse().expect("width number"))
                    .collect(),
            );
            rest = &body[end..];
        }
        assert!(arrays.len() >= 2, "regular and bold faces embedded");
        assert!(arrays.iter().all(|a| a.len() == 224));
        assert!(
            arrays
                .iter()
                .any(|a| a[0] == 277 && a[(b'A' - 32) as usize] == 666),
            "the regular face's real metrics are among the embedded tables"
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
