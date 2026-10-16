//! PDF export.
//!
//! A small hand-rolled PDF 1.4 writer. TrueType faces are embedded
//! as CID-keyed fonts (Type0 with a CIDFontType2 descendant,
//! Identity-H encoding, an explicit ToUnicode CMap, and an identity
//! CIDToGIDMap) from the same Font Library bytes the shaper
//! measured with, so what the Check step approves is what prints
//! (docs/FONTS.md). Faces are subset to the glyphs the document
//! actually draws (the `subsetter` crate, whose subsets are only
//! usable under CID embedding; see docs/LEGAL.md), unless the face
//! forbids subsetting, in which case it embeds whole, still
//! CID-keyed. Because glyphs are addressed by ID rather than by a
//! WinAnsi byte, any character the printing face actually has
//! prints, including ones outside Windows-1252. A face whose
//! license does not allow it to travel, or that cannot be found,
//! falls back to the core Helvetica fonts for that face only;
//! CFF-flavored faces keep the older whole-face WinAnsi embedding
//! (their outlines are not TrueType, and subsetter's CID output
//! for them is a different font type than this writer emits).
//! Characters no chosen face can draw render as the face's
//! missing-glyph mark, never as a wrong letter, and the Check step
//! names them through [`undrawable_chars`], which applies the same
//! face resolution and glyph test as this writer.
//!
//! Every stream (page content and embedded fonts) is FlateDecode
//! compressed, and every export carries an ExportPurpose recorded
//! in the file's information dictionary. CMYK and PDF/X remain
//! roadmap items.
//!
//! Model space is top-left origin; PDF space is bottom-left. The conversion
//! happens here and only here.

use ob_impose::SheetSide;
use ob_model::{Document, Panel, Rect};
use std::fmt::Write as _;

fn pt(v: ob_model::Mpt) -> f32 {
    v as f32 / 1000.0
}

/// What a PDF copy is FOR, in the user's own words (Jacob's
/// preset ruling, 2026-10-15). The purpose caps picture
/// resolution at the given pixels per inch once picture frames
/// exist to bite on; today it is threaded through every export
/// and recorded in the file's metadata, never faked. The .obd
/// working file and the IDML/.sla copies are never optimized:
/// they always carry the full originals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportPurpose {
    /// Reading copies: email and the church website.
    ForEmailOrWeb,
    /// The print copy: folded sheets for the church copier.
    ForPrinting,
    /// A print shop or a publication.
    ForPrintShop,
    /// No cap: 4K display and archival copies.
    FullResolution,
}

impl ExportPurpose {
    /// The radio-button label, in Jacob's exact words.
    pub fn label(self) -> &'static str {
        match self {
            ExportPurpose::ForEmailOrWeb => "For email or the web",
            ExportPurpose::ForPrinting => "For printing",
            ExportPurpose::ForPrintShop => "For a print shop or publication",
            ExportPurpose::FullResolution => "Full resolution",
        }
    }

    /// The mid-sentence form for status lines: "written: name,
    /// 212 KB, for printing."
    pub fn words(self) -> &'static str {
        match self {
            ExportPurpose::ForEmailOrWeb => "for email or the web",
            ExportPurpose::ForPrinting => "for printing",
            ExportPurpose::ForPrintShop => "for a print shop or publication",
            ExportPurpose::FullResolution => "at full resolution",
        }
    }

    /// The picture-resolution cap this purpose applies, in
    /// pixels per inch at placed size; None means no cap.
    pub fn ppi_cap(self) -> Option<u32> {
        match self {
            ExportPurpose::ForEmailOrWeb => Some(150),
            ExportPurpose::ForPrinting => Some(300),
            ExportPurpose::ForPrintShop => Some(600),
            ExportPurpose::FullResolution => None,
        }
    }

    /// The short slug the settings file and the CLI spell it with.
    pub fn slug(self) -> &'static str {
        match self {
            ExportPurpose::ForEmailOrWeb => "web",
            ExportPurpose::ForPrinting => "print",
            ExportPurpose::ForPrintShop => "shop",
            ExportPurpose::FullResolution => "full",
        }
    }

    /// Parse a slug from settings or the CLI.
    pub fn from_slug(s: &str) -> Option<ExportPurpose> {
        match s {
            "web" => Some(ExportPurpose::ForEmailOrWeb),
            "print" => Some(ExportPurpose::ForPrinting),
            "shop" => Some(ExportPurpose::ForPrintShop),
            "full" => Some(ExportPurpose::FullResolution),
            _ => None,
        }
    }
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

/// True when this character has a WinAnsi byte: still the rule
/// for the core Helvetica fallback and for CFF-flavored faces,
/// which keep the older simple-font embedding.
fn winansi_supported(c: char) -> bool {
    winansi_byte(c).is_some()
}

/// (family, bold, italic): how faces are keyed everywhere here.
type FaceKey = (String, bool, bool);

/// The outline flavor of a font program, from its sfnt tag.
/// Only OpenType/CFF programs take the legacy embedding path;
/// everything else (the common TrueType tag, `true`, collections)
/// is treated as TrueType outlines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FontFlavor {
    TrueType,
    Cff,
}

fn font_flavor(bytes: &[u8]) -> FontFlavor {
    if bytes.len() >= 4 && &bytes[0..4] == b"OTTO" {
        FontFlavor::Cff
    } else {
        FontFlavor::TrueType
    }
}

/// One embeddable face: its bytes and whether its license bits
/// forbid subsetting it before embedding.
#[derive(Clone)]
struct FaceSource {
    bytes: Vec<u8>,
    no_subsetting: bool,
}

/// Every face whose rights let it travel inside a PDF, keyed by
/// (family, bold, italic): the bundled set first, then the Font
/// Library, which wins over the bundled set (one definition,
/// shared by the writer and the Check step's oracle, so what a
/// warning is computed against is what would actually embed).
fn face_sources(
    library: &[(ob_fonts::FontInfo, Vec<u8>)],
) -> std::collections::HashMap<FaceKey, FaceSource> {
    let mut candidates: Vec<(FaceKey, FaceSource)> = Vec::new();
    for (name, bytes) in ob_text::shaping::bundled_font_files() {
        if let Ok(info) = ob_fonts::inspect(bytes, name) {
            if info.rights.may_package() {
                candidates.push((
                    (info.family, info.bold, info.italic),
                    FaceSource {
                        bytes: bytes.to_vec(),
                        no_subsetting: info.no_subsetting,
                    },
                ));
            }
        }
    }
    for (info, bytes) in library {
        if info.rights.may_package() {
            candidates.push((
                (info.family.clone(), info.bold, info.italic),
                FaceSource {
                    bytes: bytes.clone(),
                    no_subsetting: info.no_subsetting,
                },
            ));
        }
    }
    // One face per key, picked by the shared rule
    // (ob_fonts::pick_face, the same rule the document
    // font pin uses, round 3 R3-13): every candidate for
    // a key carries that key's own flags, so the rule's
    // exact match is the LAST candidate, which keeps the
    // long-standing order: the library wins over the
    // bundled set.
    let mut by_key: std::collections::HashMap<FaceKey, Vec<usize>> =
        std::collections::HashMap::new();
    for (idx, (key, _)) in candidates.iter().enumerate() {
        by_key.entry(key.clone()).or_default().push(idx);
    }
    let mut sources: std::collections::HashMap<FaceKey, FaceSource> =
        std::collections::HashMap::new();
    for (key, idxs) in by_key {
        let flags: Vec<(bool, bool)> = idxs
            .iter()
            .map(|&i| ((candidates[i].0).1, (candidates[i].0).2))
            .collect();
        if let Some(pick) = ob_fonts::pick_face(&flags, key.1, key.2) {
            sources.insert(key, candidates[idxs[pick]].1.clone());
        }
    }
    sources
}

/// The distinct families a face-source map holds: the
/// availability list the resolution chain walks
/// (docs/FONTS.md). One spelling per family, picked
/// deterministically (round 3, R3-14: this list used to
/// come out of HashMap iteration, so the spelling the
/// chain answered with could differ from run to run when
/// a library held two spellings of one family). Spellings
/// group by cleaned lowercase name; the spelling the
/// library uses most wins, ties go to the
/// lexicographically smallest; the list itself comes out
/// in cleaned-name order.
fn source_families(sources: &std::collections::HashMap<FaceKey, FaceSource>) -> Vec<String> {
    let mut groups: std::collections::BTreeMap<String, Vec<(String, usize)>> =
        std::collections::BTreeMap::new();
    for (family, _, _) in sources.keys() {
        let cleaned = family.trim().to_lowercase();
        let spellings = groups.entry(cleaned).or_default();
        match spellings.iter_mut().find(|(s, _)| s == family) {
            Some((_, count)) => *count += 1,
            None => spellings.push((family.clone(), 1)),
        }
    }
    groups
        .into_values()
        .map(|spellings| {
            spellings
                .into_iter()
                .min_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)))
                .map(|(spelling, _)| spelling)
                .unwrap_or_default()
        })
        .collect()
}

/// A glyph-presence oracle over the bundled faces plus one Font
/// Library snapshot: it resolves a requested face exactly the way
/// the writer does (the resolution chain to a family, then the
/// exact face, then the family's plain face, then the core
/// Helvetica fallback) and answers which characters
/// that printing face can actually draw. The Check step builds
/// one oracle and asks about every paragraph, so the warning and
/// the printed output can never disagree.
pub struct FaceOracle {
    sources: std::collections::HashMap<FaceKey, FaceSource>,
    families: Vec<String>,
}

impl FaceOracle {
    pub fn new(library: &[(ob_fonts::FontInfo, Vec<u8>)]) -> FaceOracle {
        let sources = face_sources(library);
        let families = source_families(&sources);
        FaceOracle { sources, families }
    }

    /// The distinct characters in `text` that the face which
    /// would print (family, bold, italic) cannot draw, in the
    /// order they first appear. A TrueType face answers from its
    /// own character map, so characters outside Windows-1252 are
    /// fine when the face has them; the core fallback and
    /// CFF-flavored faces answer with the WinAnsi rule. Tab is
    /// not reported: order lines spend it as the dot-leader
    /// marker before encoding, so it never reaches the printer
    /// as a character.
    pub fn undrawable_chars(
        &self,
        text: &str,
        family: &str,
        bold: bool,
        italic: bool,
    ) -> Vec<char> {
        // The chain first (docs/FONTS.md): a family the
        // library does not hold is answered by the face
        // that would actually print, its twin, lookalike,
        // or the house default, never by a guess.
        let family = ob_fonts::resolve::resolve_family(family, &self.families).family;
        let exact: FaceKey = (family.clone(), bold, italic);
        let plain: FaceKey = (family, false, false);
        let chosen = self
            .sources
            .get(&exact)
            .or_else(|| self.sources.get(&plain));
        let face = chosen.and_then(|src| match font_flavor(&src.bytes) {
            FontFlavor::TrueType => ttf_parser::Face::parse(&src.bytes, 0).ok(),
            FontFlavor::Cff => None,
        });
        let mut out: Vec<char> = Vec::new();
        for c in text.chars() {
            if c == '\t' || out.contains(&c) {
                continue;
            }
            let drawable = match &face {
                Some(face) => face.glyph_index(c).is_some(),
                None => winansi_supported(c),
            };
            if !drawable {
                out.push(c);
            }
        }
        out
    }
}

/// One-off form of [`FaceOracle::undrawable_chars`] for callers
/// with a single question (it builds the oracle, so callers with
/// many questions should keep a [`FaceOracle`] instead).
pub fn undrawable_chars(
    text: &str,
    family: &str,
    bold: bool,
    italic: bool,
    library: &[(ob_fonts::FontInfo, Vec<u8>)],
) -> Vec<char> {
    FaceOracle::new(library).undrawable_chars(text, family, bold, italic)
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

/// Which font one text operation draws with: an embedded face,
/// already resolved by the FontBook's ladder, or a core font.
/// The operations record this (not a resource name) because
/// subsetting needs the final draw list before any face is cut
/// down and its encoding exists.
#[derive(Clone)]
enum DrawFont {
    Embedded(FaceKey),
    Core(&'static str),
}

/// One recorded drawing operation. Pages are drawn into these
/// first; only after every side is drawn are the faces subset
/// to the glyphs these operations actually use, and the
/// operations rendered to a content stream.
/// One picture to embed: the asset's fingerprint and the
/// pixel size it is right-sized to for this copy's purpose.
/// The same picture placed at two sizes is two keys (two
/// embedded copies); placed twice at one size it is one.
type ImageKey = (String, u32, u32);

enum ContentOp {
    /// A literal fragment (the shape-guide color push).
    Raw(String),
    RectStroke {
        r: Rect,
        page_h: ob_model::Mpt,
    },
    /// A placed picture: clip to the box, draw the (possibly
    /// larger, for Fill) shown rectangle, both in page
    /// coordinates with a top-left origin like everything
    /// else here.
    Image {
        key: ImageKey,
        clip: Rect,
        draw: Rect,
        page_h: ob_model::Mpt,
    },
    Text {
        font: DrawFont,
        size: ob_model::Mpt,
        x: ob_model::Mpt,
        y_top: ob_model::Mpt,
        page_h: ob_model::Mpt,
        text: String,
        color: ob_model::Rgb,
    },
}

#[derive(Default)]
struct PageContent {
    ops: Vec<ContentOp>,
}

impl PageContent {
    fn push_raw(&mut self, s: &str) {
        self.ops.push(ContentOp::Raw(s.to_string()));
    }

    #[allow(clippy::too_many_arguments)]
    fn text(
        &mut self,
        font: DrawFont,
        size_mpt: ob_model::Mpt,
        x: ob_model::Mpt,
        y_top: ob_model::Mpt,
        page_h: ob_model::Mpt,
        s: &str,
        color: ob_model::Rgb,
    ) {
        self.ops.push(ContentOp::Text {
            font,
            size: size_mpt,
            x,
            y_top,
            page_h,
            text: s.to_string(),
            color,
        });
    }

    fn rect_stroke(&mut self, r: Rect, page_h: ob_model::Mpt) {
        self.ops.push(ContentOp::RectStroke { r, page_h });
    }

    fn image(&mut self, key: ImageKey, clip: Rect, draw: Rect, page_h: ob_model::Mpt) {
        self.ops.push(ContentOp::Image {
            key,
            clip,
            draw,
            page_h,
        });
    }

    /// The picture keys this page uses, in first-use order.
    fn image_keys(&self) -> Vec<ImageKey> {
        let mut out: Vec<ImageKey> = Vec::new();
        for op in &self.ops {
            if let ContentOp::Image { key, .. } = op {
                if !out.contains(key) {
                    out.push(key.clone());
                }
            }
        }
        out
    }

    /// Every character this page draws, per embedded face: the
    /// glyph set the subsetter is allowed to keep.
    fn collect_used(
        &self,
        used: &mut std::collections::HashMap<FaceKey, std::collections::BTreeSet<char>>,
    ) {
        for op in &self.ops {
            if let ContentOp::Text {
                font: DrawFont::Embedded(key),
                text,
                ..
            } = op
            {
                let set = used.entry(key.clone()).or_default();
                set.extend(text.chars());
            }
        }
    }

    /// Render the recorded operations to a content stream, now
    /// that the FontBook has embedded the faces and knows each
    /// one's encoding, and the pictures are embedded under
    /// their resource names.
    fn render(
        &self,
        fonts: &FontBook,
        images: &std::collections::HashMap<ImageKey, (String, usize)>,
    ) -> String {
        let mut buf = String::new();
        for op in &self.ops {
            match op {
                ContentOp::Raw(s) => buf.push_str(s),
                ContentOp::Image {
                    key,
                    clip,
                    draw,
                    page_h,
                } => match images.get(key) {
                    Some((name, _obj)) => {
                        let cy = page_h - clip.y - clip.h;
                        let dy = page_h - draw.y - draw.h;
                        let _ = writeln!(
                            buf,
                            "q {:.2} {:.2} {:.2} {:.2} re W n {:.2} 0 0 {:.2} {:.2} {:.2} cm /{} Do Q",
                            pt(clip.x),
                            pt(cy),
                            pt(clip.w),
                            pt(clip.h),
                            pt(draw.w),
                            pt(draw.h),
                            pt(draw.x),
                            pt(dy),
                            name
                        );
                    }
                    None => {
                        // The picture's bytes could not be
                        // decoded at embed time: the box prints
                        // as the same placeholder the canvas
                        // shows, never as a hole or a wrong
                        // picture.
                        let y = page_h - clip.y - clip.h;
                        let _ = writeln!(
                            buf,
                            "0.5 g 0.75 w {:.2} {:.2} {:.2} {:.2} re S 0 g BT /F1 10.00 Tf 1 0 0 1 {:.2} {:.2} Tm (No picture yet) Tj ET",
                            pt(clip.x),
                            pt(y),
                            pt(clip.w),
                            pt(clip.h),
                            pt(clip.x + 6_000),
                            pt(page_h - clip.y - clip.h / 2 - 3_500)
                        );
                    }
                },
                ContentOp::RectStroke { r, page_h } => {
                    let y = page_h - r.y - r.h;
                    let _ = writeln!(
                        buf,
                        "0.75 w {:.2} {:.2} {:.2} {:.2} re S",
                        pt(r.x),
                        pt(y),
                        pt(r.w),
                        pt(r.h)
                    );
                }
                ContentOp::Text {
                    font,
                    size,
                    x,
                    y_top,
                    page_h,
                    text,
                    color,
                } => {
                    let y = page_h - y_top; // baseline approx: y_top is the top of the line box
                                            // The paragraph style's color, set on every text op
                                            // (non-stroking `rg`), so color never leaks from one
                                            // paragraph into the next. Black stays the default.
                    let shown = match font {
                        DrawFont::Embedded(key) => match fonts.encodings.get(key) {
                            Some(FaceEncoding::Cid(cids)) => {
                                let mut hex = String::with_capacity(text.len() * 4 + 2);
                                hex.push('<');
                                for c in text.chars() {
                                    // A character the face has no
                                    // glyph for emits CID 0, the
                                    // face's own missing-glyph mark:
                                    // visible as missing, never a
                                    // wrong letter. The Check step
                                    // names these characters.
                                    let cid = cids.get(&c).copied().unwrap_or(0);
                                    let _ = write!(hex, "{cid:04X}");
                                }
                                hex.push('>');
                                hex
                            }
                            _ => format!("({})", esc(text)),
                        },
                        DrawFont::Core(_) => format!("({})", esc(text)),
                    };
                    let _ = writeln!(
                        buf,
                        "{:.2} {:.2} {:.2} rg BT /{} {:.2} Tf 1 0 0 1 {:.2} {:.2} Tm {} Tj ET",
                        color.r as f32 / 255.0,
                        color.g as f32 / 255.0,
                        color.b as f32 / 255.0,
                        fonts.resource_for(font),
                        pt(*size),
                        pt(*x),
                        pt(y),
                        shown
                    );
                }
            }
        }
        buf
    }
}

/// Draw one picture box. Returns how many pictures on it
/// could not be drawn (an empty box, a missing asset, bytes
/// that do not decode): each prints as the "No picture yet"
/// placeholder instead, and the export report counts them,
/// so a copy never fails over a picture.
#[allow(clippy::too_many_arguments)]
fn draw_picture_frame(
    pc: &mut PageContent,
    doc: &Document,
    frame: &ob_model::Frame,
    fr: Rect,
    page_h: ob_model::Mpt,
    purpose: ExportPurpose,
    fonts: &FontBook,
) -> usize {
    let placeholder = |pc: &mut PageContent| -> usize {
        pc.rect_stroke(fr, page_h);
        pc.text(
            fonts.resource_for_face("Liberation Sans", false, false),
            10_000,
            fr.x + 6_000,
            fr.y + fr.h / 2 + 3_500,
            page_h,
            "No picture yet",
            ob_model::Rgb {
                r: 110,
                g: 110,
                b: 110,
            },
        );
        1
    };
    let Some(placement) = &frame.picture else {
        return placeholder(pc);
    };
    let Some(bytes) = doc.asset_data(&placement.asset) else {
        return placeholder(pc);
    };
    let Ok(probe) = ob_image::probe(bytes) else {
        return placeholder(pc);
    };
    // Where the picture shows, in points: the one geometry
    // function the canvas and the Check step also use.
    let (dx, dy, dw, dh) = ob_model::picture_shown_rect(
        placement.fit,
        probe.width as f64,
        probe.height as f64,
        pt(fr.w) as f64,
        pt(fr.h) as f64,
        placement.crop_x,
        placement.crop_y,
    );
    // The pixels this copy needs: the purpose's cap at the
    // DRAWN size (a picture trimmed by Fill prints at its
    // shown size, not its box size), and never more than the
    // picture has. SVG has no pixels of its own; it renders
    // at the purpose's resolution (600 per inch when the
    // purpose sets no cap).
    let ppi = purpose.ppi_cap().map(|p| p as f64);
    let (tw, th) = if probe.format == ob_image::PictureFormat::Svg {
        let p = ppi.unwrap_or(600.0);
        (
            ((dw / 72.0 * p).round() as u32).clamp(1, 16_384),
            ((dh / 72.0 * p).round() as u32).clamp(1, 16_384),
        )
    } else {
        match ppi {
            Some(p) => {
                let k = ((dw / 72.0 * p) / probe.width as f64)
                    .min((dh / 72.0 * p) / probe.height as f64)
                    .min(1.0);
                (
                    ((probe.width as f64 * k).round() as u32).max(1),
                    ((probe.height as f64 * k).round() as u32).max(1),
                )
            }
            None => (probe.width, probe.height),
        }
    };
    let draw = Rect::new(
        fr.x + (dx * 1000.0).round() as ob_model::Mpt,
        fr.y + (dy * 1000.0).round() as ob_model::Mpt,
        (dw * 1000.0).round() as ob_model::Mpt,
        (dh * 1000.0).round() as ob_model::Mpt,
    );
    pc.image((placement.asset.clone(), tw, th), fr, draw, page_h);
    0
}

/// Draw one panel's frames into `pc`, offset by (ox, oy) in a space whose
/// total height is `page_h` (panel height for reader PDFs, sheet height for
/// imposed PDFs). Returns the count of pictures that printed
/// as placeholders (see draw_picture_frame).
#[allow(clippy::too_many_arguments)]
fn draw_panel(
    pc: &mut PageContent,
    doc: &Document,
    fonts: &FontBook,
    panel: &Panel,
    panel_idx: usize,
    threads: &std::collections::HashMap<String, ob_text::threading::StoryThread>,
    ox: ob_model::Mpt,
    oy: ob_model::Mpt,
    page_h: ob_model::Mpt,
    purpose: ExportPurpose,
) -> usize {
    let mut missing_pictures = 0usize;
    for (fi, frame) in panel.frames.iter().enumerate() {
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
        if frame.kind == ob_model::FrameKind::Picture {
            missing_pictures += draw_picture_frame(pc, doc, frame, fr, page_h, purpose, fonts);
            continue;
        }
        let Some(story_id) = &frame.story_id else {
            continue;
        };
        // Threading (T1): the story's layout through all its
        // boxes was computed once for the document; this box
        // draws only its slice, so a story in two boxes
        // prints each line exactly once. Lines no box could
        // hold are not in any slice and never print (Check
        // carries that warning in the app).
        let Some(thread) = threads.get(story_id.as_str()) else {
            continue;
        };
        let Some(slice) = thread.slice_for(panel_idx, fi) else {
            continue;
        };
        for piece in &slice.paras {
            let style = &piece.style;
            let layout = &piece.layout;
            for (li, line) in layout.lines().iter().enumerate() {
                // The baseline sits one line height below
                // the line's box top, in sheet coordinates.
                let y = oy + piece.top + (li as ob_model::Mpt + 1) * piece.line_h;
                // Group the line's clusters by face; a segment
                // is drawn at its first cluster's position.
                let mut segs: Vec<(usize, usize, f32, bool, bool)> = Vec::new();
                for c in &line.clusters {
                    match segs.last_mut() {
                        Some((_, end, _, b, i)) if *b == c.bold && *i == c.italic => {
                            *end = c.range.end;
                        }
                        _ => segs.push((c.range.start, c.range.end, c.x, c.bold, c.italic)),
                    }
                }
                for (start, end, cx, bold, italic) in segs {
                    let seg_text = &layout.text()[start..end];
                    if seg_text.is_empty() {
                        continue;
                    }
                    let x = fr.x + ((line.x0 + cx) * 1000.0).round() as ob_model::Mpt;
                    pc.text(
                        fonts.resource_for_face(&style.font_family, bold, italic),
                        style.font_size,
                        x,
                        y,
                        page_h,
                        seg_text,
                        style.color,
                    );
                }
            }
        }
    }
    missing_pictures
}

/// Compress a stream's bytes for /FlateDecode (zlib form, via
/// miniz_oxide). Falls back to the raw bytes when compression
/// fails or does not help: an export must never fail, or grow,
/// because of compression.
fn flate(data: &[u8]) -> (Vec<u8>, bool) {
    use std::io::Write as _;
    let mut enc = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    match enc.write_all(data).and_then(|_| enc.finish()) {
        Ok(out) if out.len() < data.len() => (out, true),
        _ => (data.to_vec(), false),
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
    fn finish(
        self,
        page_ids: &[usize],
        font1: usize,
        font2: usize,
        info: Option<usize>,
    ) -> Vec<u8> {
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
        let info_entry = match info {
            Some(id) => format!(" /Info {} 0 R", id),
            None => String::new(),
        };
        out.extend_from_slice(
            format!(
                "trailer\n<< /Size {} /Root 1 0 R{} >>\nstartxref\n{}\n",
                max_obj + 1,
                info_entry,
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

/// How content encodes text for one embedded face.
enum FaceEncoding {
    /// Character -> CID, emitted as a 2-byte code in a hex
    /// string. Characters absent here (the face has no glyph
    /// for them) emit CID 0 at render time.
    Cid(std::collections::HashMap<char, u16>),
    /// The legacy literal-string encoding, kept for
    /// CFF-flavored faces, which embed whole under the older
    /// simple-font path.
    WinAnsi,
}

/// Embedded faces for one export, in two phases. `prepare`
/// resolves which faces the document wants and which of them
/// can travel, and hands out resource names; pages are then
/// drawn against those names while their operations are
/// recorded; `embed` finally writes the font objects, cutting
/// each TrueType face down to the glyphs the recorded draw
/// list actually used. Faces that cannot be found, cannot be
/// parsed, or whose license does not let them travel, are
/// absent and fall back to the core fonts at draw time.
struct FontBook {
    /// Face key -> page resource name, in assignment order.
    faces: std::collections::HashMap<FaceKey, String>,
    order: Vec<FaceKey>,
    sources: std::collections::HashMap<FaceKey, FaceSource>,
    /// The families the sources hold: the availability
    /// list for the resolution chain at draw time.
    families: Vec<String>,
    /// Face key -> encoding, filled by `embed`.
    encodings: std::collections::HashMap<FaceKey, FaceEncoding>,
    /// Extra resource entries for the page dictionary,
    /// filled by `embed`.
    resources: String,
}

impl FontBook {
    fn prepare(
        doc: &Document,
        sources: std::collections::HashMap<FaceKey, FaceSource>,
    ) -> FontBook {
        let mut wanted: Vec<FaceKey> = Vec::new();
        for style in &doc.styles {
            let key = (style.font_family.clone(), style.bold, style.italic);
            if !wanted.contains(&key) {
                wanted.push(key);
            }
        }
        // Faces the runs ask for (G1b): a bold word inside a
        // regular paragraph needs the bold face embedded even
        // though no style names it.
        for story in &doc.stories {
            for para in &story.paragraphs {
                let Some(style) = doc.style(&para.style) else {
                    continue;
                };
                for run in &para.runs {
                    let key = (
                        style.font_family.clone(),
                        run.bold.unwrap_or(style.bold),
                        run.italic.unwrap_or(style.italic),
                    );
                    if !wanted.contains(&key) {
                        wanted.push(key);
                    }
                }
            }
        }
        let fallback = ("Liberation Sans".to_string(), false, false);
        if !wanted.contains(&fallback) {
            wanted.push(fallback);
        }
        // Wanted fonts (docs/FONTS.md): a style naming a
        // family the sources do not hold prints in the
        // resolution chain's stand-in, so the stand-in's
        // faces (in the style's own bold and italic) are
        // wanted too.
        let families = source_families(&sources);
        let mut expanded = wanted.clone();
        for key in &wanted {
            let resolved = ob_fonts::resolve::resolve_family(&key.0, &families).family;
            let rkey = (resolved, key.1, key.2);
            if !expanded.contains(&rkey) {
                expanded.push(rkey);
            }
        }
        let wanted = expanded;
        let mut faces = std::collections::HashMap::new();
        let mut order = Vec::new();
        let mut n = 0;
        for key in &wanted {
            let Some(src) = sources.get(key) else {
                continue;
            };
            // A face that does not parse cannot be embedded;
            // it falls back at draw time like a missing one.
            if ttf_parser::Face::parse(&src.bytes, 0).is_err() {
                continue;
            }
            n += 1;
            faces.insert(key.clone(), format!("FE{n}"));
            order.push(key.clone());
        }
        FontBook {
            faces,
            order,
            sources,
            families,
            encodings: std::collections::HashMap::new(),
            resources: String::new(),
        }
    }

    /// Write every prepared face into the PDF, subset to the
    /// characters the recorded draw list used in it.
    fn embed(
        &mut self,
        pdf: &mut Pdf,
        used: &std::collections::HashMap<FaceKey, std::collections::BTreeSet<char>>,
    ) {
        let empty = std::collections::BTreeSet::new();
        let mut resources = String::new();
        for key in self.order.clone() {
            let src = self.sources.get(&key).expect("prepared face has a source");
            let face = ttf_parser::Face::parse(&src.bytes, 0).expect("prepared face parses");
            let chars = used.get(&key).unwrap_or(&empty);
            let (font_obj, encoding) = match font_flavor(&src.bytes) {
                FontFlavor::TrueType => add_cid_font(pdf, &face, src, chars),
                FontFlavor::Cff => (
                    add_embedded_font(pdf, &face, &src.bytes),
                    FaceEncoding::WinAnsi,
                ),
            };
            let resource = self.faces.get(&key).expect("prepared face has a resource");
            resources.push_str(&format!(" /{} {} 0 R", resource, font_obj));
            self.encodings.insert(key, encoding);
        }
        self.resources = resources;
    }

    /// The font to draw one face request with, walking down the
    /// ladder: the resolution chain answers the family first
    /// (docs/FONTS.md: the real family when the library holds
    /// it, else its twin, its lookalike, or the house default),
    /// then the exact (family, bold, italic) face of the
    /// answered family when embedded, its plain face when not,
    /// else the core fallback (F2 bold / F1 regular). How a
    /// style and a run ask for their faces (G1b).
    fn resource_for_face(&self, family: &str, bold: bool, italic: bool) -> DrawFont {
        let family = ob_fonts::resolve::resolve_family(family, &self.families).family;
        let exact: FaceKey = (family.clone(), bold, italic);
        if self.faces.contains_key(&exact) {
            return DrawFont::Embedded(exact);
        }
        let plain: FaceKey = (family, false, false);
        if self.faces.contains_key(&plain) {
            return DrawFont::Embedded(plain);
        }
        DrawFont::Core(if bold { "F2" } else { "F1" })
    }

    /// The page resource name a draw font renders under.
    fn resource_for(&self, font: &DrawFont) -> String {
        match font {
            DrawFont::Embedded(key) => self
                .faces
                .get(key)
                .cloned()
                .unwrap_or_else(|| "F1".to_string()),
            DrawFont::Core(name) => name.to_string(),
        }
    }
}

/// Add the three objects an embedded TrueType face needs (font
/// stream, descriptor, font dictionary) and return the font object.
fn add_embedded_font(pdf: &mut Pdf, face: &ttf_parser::Face, bytes: &[u8]) -> usize {
    let upem = face.units_per_em() as i32;
    let scale = |v: i16| v as i32 * 1000 / upem;
    let name = postscript_name(face);
    let (font_bytes, compressed) = flate(bytes);
    let filter = if compressed {
        " /Filter /FlateDecode"
    } else {
        ""
    };
    let mut stream = format!(
        "<< /Length {} /Length1 {} {}>>
stream
",
        font_bytes.len(),
        bytes.len(),
        filter
    )
    .into_bytes();
    stream.extend_from_slice(&font_bytes);
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

/// Write one stream object (font program, ToUnicode CMap),
/// FlateDecode-compressed when that helps, and return its id.
fn add_stream(pdf: &mut Pdf, dict_extra: &str, data: &[u8]) -> usize {
    let (bytes, compressed) = flate(data);
    let filter = if compressed {
        " /Filter /FlateDecode"
    } else {
        ""
    };
    let mut stream = format!(
        "<< /Length {} {}{}>>
stream
",
        bytes.len(),
        dict_extra,
        filter
    )
    .into_bytes();
    stream.extend_from_slice(&bytes);
    stream.extend_from_slice(
        b"
endstream",
    );
    pdf.add(stream)
}

/// The ToUnicode CMap for one CID face: each emitted CID mapped
/// back to the character it stands for, so text extraction and
/// search recover the real words. Entries are sorted by CID and
/// chunked at the format's 100-entry block limit; characters
/// outside the Basic Multilingual Plane are written as UTF-16
/// surrogate pairs, as the format requires.
fn to_unicode_cmap(cids: &std::collections::HashMap<char, u16>) -> Vec<u8> {
    let mut pairs: Vec<(u16, char)> = cids.iter().map(|(c, cid)| (*cid, *c)).collect();
    pairs.sort_by_key(|(cid, _)| *cid);
    let mut out = String::new();
    out.push_str("/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n/CMapName /Adobe-Identity-UCS def\n/CMapType 2 def\n1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n");
    for chunk in pairs.chunks(100) {
        if chunk.is_empty() {
            continue;
        }
        let _ = writeln!(out, "{} beginbfchar", chunk.len());
        for (cid, c) in chunk {
            let mut utf16 = [0u16; 2];
            let encoded = c.encode_utf16(&mut utf16);
            let mut val = String::new();
            for unit in encoded.iter() {
                let _ = write!(val, "{unit:04X}");
            }
            let _ = writeln!(out, "<{cid:04X}> <{val}>");
        }
        out.push_str("endbfchar\n");
    }
    out.push_str("endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n");
    out.into_bytes()
}

/// Add a TrueType face as a CID-keyed font (Type0 + CIDFontType2,
/// Identity-H, identity CIDToGIDMap, ToUnicode) and return its
/// font object plus the encoding content rendering must use.
///
/// The CID scheme: when the face is subset, the subsetter
/// remaps glyph IDs consecutively and guarantees an identity
/// GID-to-CID mapping in its output, so a glyph's CID is its
/// remapped glyph ID; when the face forbids subsetting (or
/// subsetting fails), it embeds whole and a glyph's CID is its
/// original glyph ID. Either way /CIDToGIDMap is /Identity and
/// the character -> CID map returned here is the only place the
/// choice lives. Widths always come from the original face's
/// metrics, the same numbers shaping used.
fn add_cid_font(
    pdf: &mut Pdf,
    face: &ttf_parser::Face,
    source: &FaceSource,
    used: &std::collections::BTreeSet<char>,
) -> (usize, FaceEncoding) {
    let upem = face.units_per_em() as i32;
    let scale = |v: i16| v as i32 * 1000 / upem;
    let advance = |gid: u16| -> i32 {
        face.glyph_hor_advance(ttf_parser::GlyphId(gid))
            .map(|a| a as i32 * 1000 / upem)
            .unwrap_or(0)
    };
    let name = postscript_name(face);

    // Decide the embedded program and the character -> CID map.
    let mut cids: std::collections::HashMap<char, u16> = std::collections::HashMap::new();
    let mut program: Option<Vec<u8>> = None;
    if !source.no_subsetting {
        let mut remapper = subsetter::GlyphRemapper::new();
        for &c in used {
            if let Some(g) = face.glyph_index(c) {
                remapper.remap(g.0);
            }
        }
        if let Ok(subset) = subsetter::subset(&source.bytes, 0, &remapper) {
            for &c in used {
                if let Some(g) = face.glyph_index(c) {
                    if let Some(cid) = remapper.get(g.0) {
                        cids.insert(c, cid);
                    }
                }
            }
            program = Some(subset);
        }
        // A subsetting error is not an export error: fall
        // through and embed the face whole instead.
    }
    let program = match program {
        Some(p) => p,
        None => {
            cids.clear();
            for &c in used {
                if let Some(g) = face.glyph_index(c) {
                    cids.insert(c, g.0);
                }
            }
            source.bytes.clone()
        }
    };

    let stream_id = add_stream(pdf, &format!(" /Length1 {}", program.len()), &program);
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

    // /W: CID 0 (the missing-glyph mark) plus one width per
    // emitted CID, from the original face's advances.
    let mut w_entries: Vec<(u16, i32)> = vec![(0, advance(0))];
    for (c, cid) in &cids {
        if let Some(g) = face.glyph_index(*c) {
            w_entries.push((*cid, advance(g.0)));
        }
    }
    w_entries.sort_by_key(|(cid, _)| *cid);
    w_entries.dedup_by_key(|(cid, _)| *cid);
    let widths: Vec<String> = w_entries
        .iter()
        .map(|(cid, w)| format!("{cid} [{w}]"))
        .collect();
    let cidfont = format!(
        "<< /Type /Font /Subtype /CIDFontType2 /BaseFont /{} /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /FontDescriptor {} 0 R /CIDToGIDMap /Identity /DW 1000 /W [{}] >>",
        name,
        descriptor_id,
        widths.join(" ")
    );
    let cidfont_id = pdf.add(cidfont.into_bytes());
    let to_unicode_id = add_stream(pdf, "", &to_unicode_cmap(&cids));
    let font = format!(
        "<< /Type /Font /Subtype /Type0 /BaseFont /{} /Encoding /Identity-H /DescendantFonts [{} 0 R] /ToUnicode {} 0 R >>",
        name, cidfont_id, to_unicode_id
    );
    let font_id = pdf.add(font.into_bytes());
    (font_id, FaceEncoding::Cid(cids))
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

/// The document information dictionary: title, producer, and
/// the export purpose in words, so the file itself says what it
/// was made for (and at what picture cap, once pictures exist).
fn info_object(doc: &Document, purpose: ExportPurpose, subject: Option<&str>) -> Vec<u8> {
    let subject = match subject {
        Some(s) => s.to_string(),
        None => {
            let cap_note = match purpose.ppi_cap() {
                Some(ppi) => format!(" Pictures are capped at {ppi} pixels per inch."),
                None => " Pictures are kept at full resolution.".to_string(),
            };
            format!("Made {}.{}", purpose.words(), cap_note)
        }
    };
    format!(
        "<< /Title ({}) /Producer (OpenBulletin) /Subject ({}) >>",
        esc(&doc.title),
        esc(&subject)
    )
    .into_bytes()
}

/// Build a PDF from a set of sheet sides (each side becomes one page).
/// Embed one decoded picture as an image XObject and return
/// its object id. Opaque pictures ride as JPEG (the print
/// path's photo encoding, quality 85); a picture with real
/// transparency rides as FlateDecode samples with a
/// grayscale SMask, so its see-through parts stay see-through.
fn add_image_object(pdf: &mut Pdf, decoded: &ob_image::Decoded) -> Option<usize> {
    let (w, h) = (decoded.width, decoded.height);
    if decoded.has_alpha {
        let mut rgb = Vec::with_capacity(decoded.rgba.len() / 4 * 3);
        let mut alpha = Vec::with_capacity(decoded.rgba.len() / 4);
        for px in decoded.rgba.chunks_exact(4) {
            rgb.extend_from_slice(&px[0..3]);
            alpha.push(px[3]);
        }
        let smask = add_stream(
            pdf,
            &format!(
                "/Type /XObject /Subtype /Image /Width {w} /Height {h} /ColorSpace /DeviceGray /BitsPerComponent 8"
            ),
            &alpha,
        );
        Some(add_stream(
            pdf,
            &format!(
                "/Type /XObject /Subtype /Image /Width {w} /Height {h} /ColorSpace /DeviceRGB /BitsPerComponent 8 /SMask {smask} 0 R"
            ),
            &rgb,
        ))
    } else {
        match ob_image::encode_jpeg(decoded, 85) {
            Ok(jpeg) => {
                let mut obj = format!(
                    "<< /Type /XObject /Subtype /Image /Width {w} /Height {h} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode /Length {} >>\nstream\n",
                    jpeg.len()
                )
                .into_bytes();
                obj.extend_from_slice(&jpeg);
                obj.extend_from_slice(b"\nendstream");
                Some(pdf.add(obj))
            }
            Err(_) => Some(add_stream(
                pdf,
                &format!(
                    "/Type /XObject /Subtype /Image /Width {w} /Height {h} /ColorSpace /DeviceRGB /BitsPerComponent 8"
                ),
                &decoded
                    .rgba
                    .chunks_exact(4)
                    .flat_map(|px| [px[0], px[1], px[2]])
                    .collect::<Vec<u8>>(),
            )),
        }
    }
}

/// Decode and embed every picture the drawn pages use, once
/// per (asset, right-sized size). Returns the resource name
/// and object id per key; a key whose bytes fail to decode
/// is simply absent, and its operations render as
/// placeholders (the caller counts those placements from
/// the same absence).
fn prepare_images(
    pdf: &mut Pdf,
    doc: &Document,
    keys: &[ImageKey],
) -> std::collections::HashMap<ImageKey, (String, usize)> {
    let mut names = std::collections::HashMap::new();
    for (i, key) in keys.iter().enumerate() {
        let (sha, tw, th) = key;
        let Some(bytes) = doc.asset_data(sha) else {
            continue;
        };
        let Ok(decoded) = ob_image::decode_at(bytes, *tw, *th) else {
            continue;
        };
        if let Some(obj) = add_image_object(pdf, &decoded) {
            names.insert(key.clone(), (format!("Im{i}"), obj));
        }
    }
    names
}

fn build_pdf(
    doc: &Document,
    sides: &[SheetSide],
    purpose: ExportPurpose,
) -> (Vec<u8>, ExportReport) {
    build_pdf_full(doc, sides, purpose, &library_font_bytes(), None)
}

/// The full build: [`build_pdf`] with the Font Library
/// snapshot given (so tests can put a document in front of
/// faces the machine's real library does not hold) plus an
/// optional /Subject line that replaces the purpose's own
/// words (the large-print copy names itself, whatever
/// purpose's picture cap it rides with).
fn build_pdf_full(
    doc: &Document,
    sides: &[SheetSide],
    purpose: ExportPurpose,
    library: &[(ob_fonts::FontInfo, Vec<u8>)],
    subject: Option<&str>,
) -> (Vec<u8>, ExportReport) {
    // Spoken-only paragraphs never reach a PDF (the one
    // publication rule, ob_model::Document::without_spoken_only):
    // every PDF door flows through here, so filtering here
    // covers the reader, imposed, and large-print copies
    // alike, and the count rides out in the report.
    let (filtered, spoken_only_left_out) = doc.without_spoken_only();
    let doc = &filtered;
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
    let mut fonts = FontBook::prepare(doc, face_sources(library));
    let mut shaper = ob_text::shaping::bundled_shaper();
    register_library_fonts(&mut shaper, library);
    // Every story's thread layout, computed once (T1): the
    // panels below draw their slices of it.
    let threads = ob_text::threading::layout_document_threads(&mut shaper, doc);
    // Draw every side first, recording operations: subsetting
    // needs the final draw list (the resolved words, dot
    // leaders and all) before any face can be cut down to it.
    let mut drawn: Vec<PageContent> = Vec::new();
    let mut missing_pictures = 0usize;
    for side in sides {
        let mut pc = PageContent::default();
        pc.push_raw("0.92 0.92 0.92 RG "); // light frame guides color for shapes
        for pl in &side.placements {
            if let Some(panel) = doc.panels.get(pl.panel) {
                missing_pictures += draw_panel(
                    &mut pc,
                    doc,
                    &fonts,
                    panel,
                    pl.panel,
                    &threads,
                    pl.x,
                    pl.y,
                    side.size.h,
                    purpose,
                );
            }
        }
        drawn.push(pc);
    }
    let mut used: std::collections::HashMap<FaceKey, std::collections::BTreeSet<char>> =
        std::collections::HashMap::new();
    for pc in &drawn {
        pc.collect_used(&mut used);
    }
    fonts.embed(&mut pdf, &used);
    // Pictures embed once per (asset, right-sized size)
    // across the whole copy; a placement whose picture
    // failed to embed renders as a placeholder and counts
    // here, beside the ones already counted at draw time.
    let mut image_keys: Vec<ImageKey> = Vec::new();
    for pc in &drawn {
        for key in pc.image_keys() {
            if !image_keys.contains(&key) {
                image_keys.push(key);
            }
        }
    }
    let images = prepare_images(&mut pdf, doc, &image_keys);
    for pc in &drawn {
        for op in &pc.ops {
            if let ContentOp::Image { key, .. } = op {
                if !images.contains_key(key) {
                    missing_pictures += 1;
                }
            }
        }
    }
    let mut page_ids = Vec::new();
    for (side, pc) in sides.iter().zip(drawn.iter()) {
        let content = pc.render(&fonts, &images).into_bytes();
        let (content, compressed) = flate(&content);
        let filter = if compressed {
            " /Filter /FlateDecode"
        } else {
            ""
        };
        let content_id = pdf.add({
            let mut v = format!(
                "<< /Length {} {}>>
stream
",
                content.len(),
                filter
            )
            .into_bytes();
            v.extend_from_slice(&content);
            v.extend_from_slice(
                b"
endstream",
            );
            v
        });
        let mut xobjects = String::new();
        for key in pc.image_keys() {
            if let Some((name, obj)) = images.get(&key) {
                xobjects.push_str(&format!(" /{name} {obj} 0 R"));
            }
        }
        let xobject_dict = if xobjects.is_empty() {
            String::new()
        } else {
            format!(" /XObject <<{xobjects} >>")
        };
        let page_id = pdf.add(
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {:.2} {:.2}] /Resources << /Font << /F1 {} 0 R /F2 {} 0 R{} >>{} >> /Contents {} 0 R >>",
                pt(side.size.w),
                pt(side.size.h),
                f1, f2, fonts.resources, xobject_dict, content_id
            )
            .into_bytes(),
        );
        page_ids.push(page_id);
    }
    let info = pdf.add(info_object(doc, purpose, subject));
    (
        pdf.finish(&page_ids, f1, f2, Some(info)),
        ExportReport {
            missing_pictures,
            spoken_only_left_out,
        },
    )
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
            keep_through: None,
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
                picture: None,
                music: None,
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
        no_announcements: false,
        assets: Vec::new(),
        asset_bytes: Default::default(),
        occasion_time: None,
        wanted_fonts: Vec::new(),
        participants: Default::default(),
        inserts: Default::default(),
    };
    // The sheet exists to be printed, so it takes the print purpose.
    export_reader_pdf(&doc, ExportPurpose::ForPrinting)
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
/// (closing audit). Public so the app's Check step works from
/// the same snapshot the exports use.
pub fn library_font_bytes() -> Vec<(ob_fonts::FontInfo, Vec<u8>)> {
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

/// What an export wants to say about itself beside the
/// bytes: how many picture placements printed as
/// "No picture yet" placeholders because their box was
/// empty, their asset was missing, or its bytes would not
/// decode, and how many spoken-only paragraphs were left
/// out (they are read aloud in the service and never
/// publish). The export itself never fails over either.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ExportReport {
    pub missing_pictures: usize,
    pub spoken_only_left_out: usize,
}

pub fn export_reader_pdf(doc: &Document, purpose: ExportPurpose) -> Vec<u8> {
    export_reader_pdf_report(doc, purpose).0
}

/// Reader PDF with its export report.
pub fn export_reader_pdf_report(doc: &Document, purpose: ExportPurpose) -> (Vec<u8>, ExportReport) {
    let sides = ob_impose::impose_single(doc);
    build_pdf(doc, &sides, purpose)
}

pub fn export_imposed_pdf(doc: &Document, purpose: ExportPurpose) -> Vec<u8> {
    export_imposed_pdf_report(doc, purpose).0
}

/// Imposed PDF with its export report: printer sheets,
/// ready to print duplex and fold.
pub fn export_imposed_pdf_report(
    doc: &Document,
    purpose: ExportPurpose,
) -> (Vec<u8>, ExportReport) {
    // Lay out the padded document so a file with the wrong
    // number of panels prints with blanks, never crashes and
    // never drops a page (closing audit).
    let padded = ob_impose::padded_document(doc);
    let sides = ob_impose::impose(&padded);
    build_pdf(&padded, &sides, purpose)
}

/// The /Subject a large-print copy carries, in the file
/// itself: what it is, in plain words.
pub const LARGE_PRINT_SUBJECT: &str = "Large print. The same words, bigger.";

/// The large-print copy (docs/planning/beyond-print.md
/// section 4): the same words, bigger, for the readers who
/// need them. A reader-layout PDF of single Letter-portrait
/// pages; the imposed booklet flow is untouched.
pub fn export_large_print_pdf(doc: &Document, purpose: ExportPurpose) -> Vec<u8> {
    export_large_print_pdf_report(doc, purpose).0
}

/// Large-print copy with its export report.
pub fn export_large_print_pdf_report(
    doc: &Document,
    purpose: ExportPurpose,
) -> (Vec<u8>, ExportReport) {
    let library = library_font_bytes();
    let lp = large_print_document_with_library(doc, &library);
    let sides = ob_impose::impose_single(&lp);
    let (bytes, mut report) =
        build_pdf_full(&lp, &sides, purpose, &library, Some(LARGE_PRINT_SUBJECT));
    // The transform already worked from the publication
    // copy, so the build saw nothing spoken-only to leave
    // out; the report names the source document's count.
    report.spoken_only_left_out = doc.spoken_only_count();
    (bytes, report)
}

/// The large-print transformation of a document, with the
/// machine's real Font Library: styles scaled so the body
/// style reaches at least 17pt, content reflowed onto
/// Letter-portrait pages in reading order (see
/// [`large_print_document_with_library`]).
pub fn large_print_document(doc: &Document) -> Document {
    large_print_document_with_library(doc, &library_font_bytes())
}

/// The transformation itself. The factor is whatever brings
/// the body style (the style named "body", else the
/// smallest style in the document) to 17pt, never below
/// 1.0; every style's size and space-after scale by it, so
/// line heights follow the existing style math. Each source
/// panel becomes a section of one or more Letter pages:
/// picture and music boxes sit at the top of the section's
/// first page at their proportions, text boxes stack down
/// each page at the full text width with heights in their
/// source proportions, and a story flows from page to page
/// of its section through the ordinary threading until
/// every word fits. Sections are added pages until the
/// thread layout reports nothing left over.
fn large_print_document_with_library(
    doc: &Document,
    library: &[(ob_fonts::FontInfo, Vec<u8>)],
) -> Document {
    // Fit is measured on the words that will print: the
    // publication copy, spoken-only paragraphs already out.
    let (base, _) = doc.without_spoken_only();
    let page = ob_model::Size::new(612_000, 792_000); // US Letter portrait
    let margin: ob_model::Mpt = 54_000; // 3/4 inch
    let usable_w = page.w - 2 * margin;
    let usable_h = page.h - 2 * margin;

    let body_size = base
        .style("body")
        .map(|s| s.font_size)
        .or_else(|| base.styles.iter().map(|s| s.font_size).min())
        .unwrap_or(11_000)
        .max(1);
    let factor = (17_000.0 / body_size as f64).max(1.0);
    let scale_mpt = |v: ob_model::Mpt, lo: ob_model::Mpt, hi: ob_model::Mpt| -> ob_model::Mpt {
        ((v as f64 * factor).round() as ob_model::Mpt).clamp(lo, hi)
    };

    let mut out = base.clone();
    for style in &mut out.styles {
        style.font_size = scale_mpt(style.font_size, 500, 288_000);
        style.space_after = scale_mpt(style.space_after, 0, 144_000);
    }
    out.sheet = ob_model::SheetSpec {
        preset: "letter-portrait".to_string(),
        size: page,
    };

    let mut pages_per_section = vec![1usize; base.panels.len()];
    let mut shaper = ob_text::shaping::bundled_shaper();
    register_library_fonts(&mut shaper, library);
    // Sections gain pages until nothing is left over. Every
    // added page gives each of the section's stories at
    // least one more line of room, so this converges; the
    // cap only guards against a pathological document.
    for _ in 0..64 {
        let (panels, panel_section) =
            build_large_print_panels(&base, &pages_per_section, page, margin, usable_w, usable_h);
        out.panels = panels;
        let threads = ob_text::threading::layout_document_threads(&mut shaper, &out);
        let mut grew = false;
        for (sid, thread) in &threads {
            if thread.leftover_lines > 0 {
                // The section that owns the story: the
                // section of its first frame, in document
                // order, mapped back through the candidate
                // panels.
                let section = out
                    .frames_of_story(sid)
                    .first()
                    .and_then(|&(pi, _)| panel_section.get(pi).copied());
                if let Some(section) = section {
                    if pages_per_section[section] < 200 {
                        pages_per_section[section] += 1;
                        grew = true;
                    }
                }
            }
        }
        if !grew {
            break;
        }
    }
    out
}

/// One pass of section building for
/// [`large_print_document_with_library`]: `pages_per_section`
/// Letter pages per source panel, furniture (picture and
/// music boxes) at the top of the first page, text boxes
/// stacked beneath and repeated on the later pages so
/// their stories thread through.
#[allow(clippy::too_many_arguments)]
fn build_large_print_panels(
    base: &Document,
    pages_per_section: &[usize],
    page: ob_model::Size,
    margin: ob_model::Mpt,
    usable_w: ob_model::Mpt,
    usable_h: ob_model::Mpt,
) -> (Vec<Panel>, Vec<usize>) {
    let mut panels = Vec::new();
    let mut panel_section = Vec::new();
    for (si, src) in base.panels.iter().enumerate() {
        let is_text =
            |f: &ob_model::Frame| f.story_id.is_some() && f.picture.is_none() && f.music.is_none();
        let furniture: Vec<&ob_model::Frame> = src.frames.iter().filter(|f| !is_text(f)).collect();
        let text_frames: Vec<&ob_model::Frame> = src.frames.iter().filter(|f| is_text(f)).collect();

        // Furniture keeps its proportions, scaled to the
        // text width, and the stack never takes more than
        // two fifths of the first page.
        let width_ratio = if src.size.w > 0 {
            usable_w as f64 / src.size.w as f64
        } else {
            1.0
        };
        let mut furniture_sizes: Vec<(ob_model::Mpt, ob_model::Mpt)> = furniture
            .iter()
            .map(|f| {
                let w = ((f.rect.w as f64 * width_ratio).round() as ob_model::Mpt).min(usable_w);
                let h = if f.rect.w > 0 {
                    (f.rect.h as f64 * w as f64 / f.rect.w as f64).round() as ob_model::Mpt
                } else {
                    f.rect.h
                };
                (w, h)
            })
            .collect();
        let gap: ob_model::Mpt = 12_000;
        let stack_h: ob_model::Mpt = furniture_sizes
            .iter()
            .map(|&(_, h)| h)
            .sum::<ob_model::Mpt>()
            + gap * furniture_sizes.len().saturating_sub(1) as ob_model::Mpt;
        let cap = usable_h * 2 / 5;
        if stack_h > cap && stack_h > 0 {
            let shrink = cap as f64 / stack_h as f64;
            for (w, h) in &mut furniture_sizes {
                *w = (*w as f64 * shrink).round() as ob_model::Mpt;
                *h = (*h as f64 * shrink).round() as ob_model::Mpt;
            }
        }
        let furniture_h: ob_model::Mpt = furniture_sizes
            .iter()
            .map(|&(_, h)| h)
            .sum::<ob_model::Mpt>()
            + gap * furniture_sizes.len().saturating_sub(1) as ob_model::Mpt;

        let pages = pages_per_section.get(si).copied().unwrap_or(1).max(1);
        for p in 0..pages {
            let mut frames: Vec<ob_model::Frame> = Vec::new();
            let mut text_top = margin;
            if p == 0 {
                let mut y = margin;
                for (f, &(w, h)) in furniture.iter().zip(furniture_sizes.iter()) {
                    let mut nf = (*f).clone();
                    nf.id = format!("{}-lp", f.id);
                    nf.rect = Rect::new(margin + (usable_w - w) / 2, y, w, h);
                    frames.push(nf);
                    y += h + gap;
                }
                if !furniture.is_empty() {
                    text_top = margin + furniture_h + gap;
                }
            }
            // Text boxes stack down the page at the full
            // text width, heights in their source
            // proportions, each at least a line tall.
            let text_area_h = (margin + usable_h - text_top).max(1);
            let total_src_h: ob_model::Mpt = text_frames.iter().map(|f| f.rect.h).sum();
            let n = text_frames.len() as ob_model::Mpt;
            let mut y = text_top;
            for (i, f) in text_frames.iter().enumerate() {
                let share = if total_src_h > 0 && n > 0 {
                    (text_area_h as f64 * f.rect.h as f64 / total_src_h as f64).round()
                        as ob_model::Mpt
                } else if n > 0 {
                    text_area_h / n
                } else {
                    0
                };
                let h = if i + 1 == text_frames.len() {
                    margin + usable_h - y // the last box takes the rest
                } else {
                    share.max(18_000)
                };
                let mut nf = (*f).clone();
                nf.id = format!("{}-lp{}", f.id, p);
                nf.rect = Rect::new(margin, y, usable_w, h.max(1));
                frames.push(nf);
                y += h;
            }
            panels.push(Panel {
                id: format!("lp-{si}-{p}"),
                name: if p == 0 {
                    src.name.clone()
                } else {
                    format!("{} (continued)", src.name)
                },
                size: page,
                frames,
                background_image: None,
                guides: Vec::new(),
            });
            panel_section.push(si);
        }
    }
    (panels, panel_section)
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

    /// A document whose one story ("s1") is threaded
    /// through two boxes on one page (T1): the first box
    /// takes the given height, the second is tall.
    fn threaded_doc(first_h: ob_model::Mpt, second_h: ob_model::Mpt) -> Document {
        use ob_model::{
            ContentRole, Frame, FrameKind, Paragraph, SheetSpec, Size, Story, FORMAT_VERSION,
        };
        let text = "Zephyr opens the morning with quiet words for everyone \
            gathered here today, and Quill writes them down in the big \
            book by the door, while Harbor bells ring over the water \
            and Meadow flowers line the path to the Candle that burns \
            beside the Summit cross above the Velvet curtain near the \
            Anchor stone at the garden gate, where everyone can see it.";
        let story = Story {
            id: "s1".into(),
            paragraphs: vec![Paragraph::new("body", text)],
            keep_through: None,
        };
        let frame = |id: &str, y: ob_model::Mpt, h: ob_model::Mpt| Frame {
            id: id.into(),
            name: id.into(),
            kind: FrameKind::Text,
            rect: Rect::new(72_000, y, 200_000, h),
            story_id: Some("s1".into()),
            role: ContentRole::Weekly,
            picture: None,
            music: None,
        };
        Document {
            format_version: FORMAT_VERSION,
            title: "Threaded".into(),
            profile: ChurchProfile::default(),
            fields: Default::default(),
            styles: ob_model::default_styles(),
            stories: vec![story],
            panels: vec![Panel {
                id: "p1".into(),
                name: "Page 1".into(),
                size: Size::new(612_000, 792_000),
                frames: vec![frame("a", 72_000, first_h), frame("b", 300_000, second_h)],
                guides: Vec::new(),
                background_image: None,
            }],
            sheet: SheetSpec {
                preset: "letter-portrait".into(),
                size: Size::new(612_000, 792_000),
            },
            no_announcements: false,
            assets: Vec::new(),
            asset_bytes: Default::default(),
            occasion_time: None,
            wanted_fonts: Vec::new(),
            participants: Default::default(),
            inserts: Default::default(),
        }
    }

    const PROBES: [&str; 8] = [
        "Zephyr", "Quill", "Harbor", "Meadow", "Candle", "Summit", "Velvet", "Anchor",
    ];

    #[test]
    fn a_threaded_story_prints_each_word_exactly_once() {
        // T1's invariant, proven on the extracted text: a
        // story in two boxes prints each of its words once
        // across the whole PDF, never whole in both boxes.
        let doc = threaded_doc(45_000, 400_000);
        let bytes = export_reader_pdf(&doc, ExportPurpose::ForPrinting);
        let text = extracted_text(&bytes);
        for word in PROBES {
            assert_eq!(
                text.matches(word).count(),
                1,
                "{word} prints exactly once: {text}"
            );
        }
    }

    #[test]
    fn leftover_thread_lines_do_not_print() {
        // Both boxes are short: the story's tail fits
        // nowhere. Its words stay out of the PDF (Check
        // carries the warning in the app); the head still
        // prints exactly once.
        let doc = threaded_doc(30_000, 30_000);
        let bytes = export_reader_pdf(&doc, ExportPurpose::ForPrinting);
        let text = extracted_text(&bytes);
        assert_eq!(text.matches("Zephyr").count(), 1, "the head prints: {text}");
        for word in ["Velvet", "Anchor"] {
            assert_eq!(
                text.matches(word).count(),
                0,
                "{word} is past the last box and never prints: {text}"
            );
        }
    }

    #[test]
    fn eof_marker_is_exactly_two_percents() {
        let bytes = export_reader_pdf(&sample_doc(), ExportPurpose::ForEmailOrWeb);
        let tail = &bytes[bytes.len().saturating_sub(16)..];
        let tail = String::from_utf8_lossy(tail);
        assert!(tail.trim_end().ends_with("%%EOF"), "tail was: {tail:?}");
        assert!(!tail.contains("%%%%"), "doubled EOF marker: {tail:?}");
    }

    #[test]
    fn undrawable_chars_follow_the_printing_face() {
        // Plain ASCII and the punctuation people actually type
        // (curly quotes, em dash, the dagger itself, the bullet)
        // are glyphs the bundled Liberation faces have: not
        // flagged.
        assert!(
            undrawable_chars("Plain text, 123!", "Liberation Sans", false, false, &[]).is_empty()
        );
        assert!(undrawable_chars(
            "\u{201C}quoted\u{201D} \u{2014} and \u{2020} \u{2022}",
            "Liberation Sans",
            false,
            false,
            &[]
        )
        .is_empty());
        // Greek letters have no WinAnsi byte but Liberation has
        // the glyphs, and CID embedding draws glyphs, not bytes:
        // not flagged anymore. This is the ceiling O2 lifts.
        assert!(
            undrawable_chars("\u{3B1}\u{3C9}", "Liberation Sans", false, false, &[]).is_empty()
        );
        // A check mark and the Latin cross are in no bundled
        // Liberation face (verified against the faces' own
        // character maps), so they are still named.
        assert_eq!(
            undrawable_chars("done \u{2713}", "Liberation Sans", false, false, &[]),
            vec!['\u{2713}']
        );
        assert_eq!(
            undrawable_chars("a \u{271D} b", "Liberation Sans", false, false, &[]),
            vec!['\u{271D}']
        );
        // An emoji is flagged, once, however often it appears.
        assert_eq!(
            undrawable_chars(
                "\u{1F600} and \u{1F600}",
                "Liberation Sans",
                false,
                false,
                &[]
            ),
            vec!['\u{1F600}']
        );
        // A family no source provides resolves to the house
        // default (docs/FONTS.md): it prints in Liberation
        // Sans, so the oracle answers with that face's map
        // (Greek is drawable there; the cross is not).
        assert!(undrawable_chars("\u{3B1}", "No Such Family", false, false, &[]).is_empty());
        assert_eq!(
            undrawable_chars("\u{271D}", "No Such Family", false, false, &[]),
            vec!['\u{271D}']
        );
        // A tab is the dot-leader marker, never a character:
        // skipped.
        assert!(undrawable_chars("Prelude\t", "Liberation Sans", false, false, &[]).is_empty());
    }

    /// Every page's content stream, decoded (content streams
    /// are FlateDecode-compressed, so the words live behind the
    /// filter; a strict parser decodes them for us).
    fn decoded_content(pdf: &[u8]) -> String {
        let parsed = lopdf::Document::load_mem(pdf).expect("lopdf parses the export");
        let mut out = String::new();
        for (_, page_id) in parsed.get_pages() {
            if let Ok(content) = parsed.get_page_content(page_id) {
                out.push_str(&String::from_utf8_lossy(&content));
                out.push('\n');
            }
        }
        out
    }

    /// One font object's ToUnicode map: CID -> the text it
    /// stands for, parsed from the bfchar blocks.
    fn to_unicode_map(
        parsed: &lopdf::Document,
        font: &lopdf::Dictionary,
    ) -> std::collections::HashMap<u16, String> {
        let mut map = std::collections::HashMap::new();
        let Some(tu_ref) = font
            .get(b"ToUnicode")
            .ok()
            .and_then(|o| o.as_reference().ok())
        else {
            return map;
        };
        let Some(stream) = parsed
            .get_object(tu_ref)
            .ok()
            .and_then(|o| o.as_stream().ok())
        else {
            return map;
        };
        let Ok(content) = stream.decompressed_content() else {
            return map;
        };
        let text = String::from_utf8_lossy(&content).into_owned();
        for block in text.split("beginbfchar").skip(1) {
            let block = block.split("endbfchar").next().unwrap_or("");
            let toks: Vec<&str> = block
                .split(|c: char| c == '<' || c == '>' || c.is_whitespace())
                .filter(|t| !t.is_empty())
                .collect();
            for pair in toks.chunks_exact(2) {
                let Ok(cid) = u16::from_str_radix(pair[0], 16) else {
                    continue;
                };
                let units: Vec<u16> = (0..pair[1].len() / 4)
                    .filter_map(|i| u16::from_str_radix(&pair[1][i * 4..i * 4 + 4], 16).ok())
                    .collect();
                if let Ok(s) = String::from_utf16(&units) {
                    map.insert(cid, s);
                }
            }
        }
        map
    }

    /// The text an export actually draws, recovered the way a
    /// reader would: each show-text operation decoded through
    /// its font's ToUnicode map (or WinAnsi for the core fonts).
    /// Returns (resource, text) per operation, in draw order.
    /// A CID with no map entry (a missing glyph) extracts as
    /// U+FFFD, as readers conventionally show it.
    fn extracted_runs(pdf: &[u8]) -> Vec<(String, String)> {
        let parsed = lopdf::Document::load_mem(pdf).expect("lopdf parses the export");
        let mut maps: std::collections::HashMap<String, std::collections::HashMap<u16, String>> =
            std::collections::HashMap::new();
        for (_, page_id) in parsed.get_pages() {
            let Ok((Some(resources), _)) = parsed.get_page_resources(page_id) else {
                continue;
            };
            let Ok(font_dict) = resources.get(b"Font").and_then(|o| o.as_dict()) else {
                continue;
            };
            for (name, obj) in font_dict.iter() {
                let name = String::from_utf8_lossy(name).into_owned();
                if maps.contains_key(&name) {
                    continue;
                }
                let Some(font_obj) = obj
                    .as_reference()
                    .ok()
                    .and_then(|id| parsed.get_object(id).ok())
                else {
                    continue;
                };
                let Ok(dict) = font_obj.as_dict() else {
                    continue;
                };
                maps.insert(name, to_unicode_map(&parsed, dict));
            }
        }
        let mut out: Vec<(String, String)> = Vec::new();
        for (_, page_id) in parsed.get_pages() {
            let Ok(content) = parsed.get_page_content(page_id) else {
                continue;
            };
            let Ok(ops) = lopdf::content::Content::decode(&content) else {
                continue;
            };
            let mut current = String::new();
            for op in ops.operations {
                match op.operator.as_str() {
                    "Tf" => {
                        if let Some(lopdf::Object::Name(n)) = op.operands.first() {
                            current = String::from_utf8_lossy(n).into_owned();
                        }
                    }
                    "Tj" => {
                        if let Some(lopdf::Object::String(bytes, _)) = op.operands.first() {
                            let text = match maps.get(&current) {
                                Some(map) if !map.is_empty() => {
                                    let mut s = String::new();
                                    for pair in bytes.chunks_exact(2) {
                                        let cid = u16::from_be_bytes([pair[0], pair[1]]);
                                        match map.get(&cid) {
                                            Some(t) => s.push_str(t),
                                            None => s.push('\u{FFFD}'),
                                        }
                                    }
                                    s
                                }
                                _ => bytes.iter().filter_map(|b| winansi_char(*b)).collect(),
                            };
                            out.push((current.clone(), text));
                        }
                    }
                    _ => {}
                }
            }
        }
        out
    }

    /// All drawn text of an export, operations joined by spaces.
    fn extracted_text(pdf: &[u8]) -> String {
        extracted_runs(pdf)
            .iter()
            .map(|(_, t)| t.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }

    #[test]
    fn duplex_test_sheet_has_front_and_back() {
        let bytes = export_duplex_test_pdf();
        assert!(bytes.starts_with(b"%PDF-1.4"));
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.contains("/Count 2"), "two pages: {text:.200}");
        let content = extracted_text(&bytes);
        assert!(content.contains("FRONT"), "front missing: {content:.300}");
        assert!(content.contains("BACK"), "back missing: {content:.300}");
    }

    #[test]
    fn reader_pdf_looks_like_pdf() {
        let bytes = export_reader_pdf(&sample_doc(), ExportPurpose::ForEmailOrWeb);
        assert!(bytes.starts_with(b"%PDF-1.4"));
        let text = String::from_utf8_lossy(&bytes);
        assert!(
            text.contains("/Count 4"),
            "reader PDF must have 4 pages: {text:.200}"
        );
        assert!(
            extracted_text(&bytes).contains("Grace Lutheran Church"),
            "cover text missing"
        );
    }

    /// A bifold sample whose announcements fill the notes
    /// box at body size: the large-print copy needs a second
    /// page for them, the reader copy does not.
    fn full_notes_doc() -> Document {
        let mut doc = sample_doc();
        let notes = &mut doc.stories[2];
        for i in 0..22 {
            notes.paragraphs.push(ob_model::Paragraph::new(
                "body",
                &format!("Announcement {i} about the picnic quilts"),
            ));
        }
        doc
    }

    fn page_count(pdf: &[u8]) -> usize {
        let text = String::from_utf8_lossy(pdf);
        let at = text.find("/Count ").expect("page tree count");
        text[at + 7..]
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>()
            .parse()
            .expect("page count number")
    }

    fn word_set(pdf: &[u8]) -> std::collections::BTreeSet<String> {
        extracted_text(pdf)
            .split_whitespace()
            // Dot leaders are spacing, not words: their run
            // length follows the box width, which differs
            // between the two copies on purpose.
            .filter(|w| !w.chars().all(|c| c == '.'))
            .map(|w| w.to_string())
            .collect()
    }

    #[test]
    fn large_print_scales_the_body_and_keeps_every_word() {
        let doc = full_notes_doc();
        // The source body is 11pt (the default styles).
        assert_eq!(doc.style("body").expect("body style").font_size, 11_000);
        let lp = large_print_document(&doc);
        let body = lp.style("body").expect("body style");
        assert!(
            body.font_size >= 17_000,
            "large-print body must reach 17pt, got {}",
            body.font_size
        );
        for panel in &lp.panels {
            assert_eq!(
                panel.size,
                ob_model::Size::new(612_000, 792_000),
                "large print lives on Letter portrait pages"
            );
        }

        let (bytes, report) = export_large_print_pdf_report(&doc, ExportPurpose::ForPrinting);
        assert!(bytes.starts_with(b"%PDF-1.4"));
        // The file names itself in its own metadata.
        let raw = String::from_utf8_lossy(&bytes);
        assert!(
            raw.contains("Large print. The same words, bigger."),
            "the /Subject says what the copy is"
        );
        assert_eq!(report.spoken_only_left_out, 0);

        let reader = export_reader_pdf(&doc, ExportPurpose::ForEmailOrWeb);
        assert_eq!(
            word_set(&bytes),
            word_set(&reader),
            "large print carries exactly the reader copy's words"
        );
        assert!(
            page_count(&bytes) > page_count(&reader),
            "bigger words need more pages: large print {}, reader {}",
            page_count(&bytes),
            page_count(&reader)
        );
    }

    #[test]
    fn spoken_only_words_never_reach_a_pdf() {
        let mut doc = sample_doc();
        let mut spoken = ob_model::Paragraph::new("body", "For Wilhelmina Quietname");
        spoken.spoken_only = true;
        doc.stories[2].paragraphs.push(spoken);
        doc.stories[2]
            .paragraphs
            .push(ob_model::Paragraph::new("body", "For the food bank crew"));

        let copies: Vec<(&str, Vec<u8>, ExportReport)> = [
            ("reader", ExportPurpose::ForEmailOrWeb),
            ("imposed", ExportPurpose::ForPrinting),
            ("large print", ExportPurpose::ForPrinting),
        ]
        .iter()
        .map(|(name, purpose)| {
            let (b, r) = match *name {
                "reader" => export_reader_pdf_report(&doc, *purpose),
                "imposed" => export_imposed_pdf_report(&doc, *purpose),
                _ => export_large_print_pdf_report(&doc, *purpose),
            };
            (*name, b, r)
        })
        .collect();
        for (name, bytes, report) in &copies {
            let text = extracted_text(bytes);
            assert!(
                !text.contains("Wilhelmina"),
                "{name} copy leaked a spoken-only name: {text:.400}"
            );
            assert!(
                text.contains("food bank crew"),
                "{name} copy lost the printable neighbor: {text:.400}"
            );
            assert_eq!(
                report.spoken_only_left_out, 1,
                "{name} copy counts the name it left out"
            );
        }
        // The working document still holds the paragraph.
        assert_eq!(doc.spoken_only_count(), 1);
    }

    #[test]
    fn a_loose_sheet_prints_with_the_hosts_fields_and_keeps_its_secrets() {
        // R-3: an insert exports through the same funnel
        // as the bulletin, synthesized by as_document: its
        // field tokens resolve against the host, and a
        // spoken-only paragraph inside it is filtered by
        // the funnel like any other.
        let host = sample_doc();
        let mut spoken = ob_model::Paragraph::new("body", "For Wilhelmina Quietname");
        spoken.spoken_only = true;
        let ins = ob_model::Insert {
            id: "sheet-1".into(),
            name: "Prayer list".into(),
            life: ob_model::InsertLife::ThisWeekOnly,
            keep_through: None,
            panels: vec![ob_model::Panel {
                id: "sheet-1-p1".into(),
                name: "Prayer list".into(),
                size: ob_model::Size::new(396_000, 612_000),
                frames: vec![ob_model::Frame {
                    id: "sheet-1-f1".into(),
                    name: "Words".into(),
                    kind: ob_model::FrameKind::Text,
                    rect: ob_model::Rect::new(36_000, 36_000, 324_000, 540_000),
                    story_id: Some("sheet-1-s1".into()),
                    role: ob_model::ContentRole::Standing,
                    picture: None,
                    music: None,
                }],
                guides: vec![],
                background_image: None,
            }],
            stories: vec![ob_model::Story {
                id: "sheet-1-s1".into(),
                paragraphs: vec![
                    ob_model::Paragraph::new("body", "Prayers for {Sunday.date} at {Church.name}"),
                    spoken,
                    ob_model::Paragraph::new("body", "For the food bank crew"),
                ],
                keep_through: None,
            }],
            styles: ob_model::default_styles(),
            sheet: ob_model::InsertSheet {
                preset: "half-letter".into(),
                copies: 60,
                paper_note: "blue paper".into(),
            },
        };
        let doc = ins.as_document(&host);
        let (bytes, report) = export_reader_pdf_report(&doc, ExportPurpose::ForPrinting);
        assert!(bytes.starts_with(b"%PDF-1.4"));
        let text = extracted_text(&bytes);
        assert!(
            text.contains("October 18, 2026"),
            "the host's date resolved in the sheet: {text:.400}"
        );
        assert!(
            text.contains("Grace Lutheran Church"),
            "the host's church name resolved in the sheet: {text:.400}"
        );
        assert!(
            !text.contains("Wilhelmina"),
            "a spoken-only name inside a sheet never prints: {text:.400}"
        );
        assert!(
            text.contains("food bank crew"),
            "the printable words print: {text:.400}"
        );
        assert_eq!(report.spoken_only_left_out, 1);
    }

    #[test]
    fn web_hidden_words_stay_in_the_printed_copies() {
        // R-2b: not-for-web is a web-page rule only. The
        // reader copy (and with it every print copy, which
        // shares the funnel) keeps the paragraph whole.
        let mut doc = sample_doc();
        let mut hidden = ob_model::Paragraph::new("body", "Timmy and June, front row");
        hidden.web_hidden = true;
        doc.stories[2].paragraphs.push(hidden);
        let bytes = export_reader_pdf(&doc, ExportPurpose::ForPrinting);
        let text = extracted_text(&bytes);
        assert!(
            text.contains("Timmy and June"),
            "the printed copy keeps not-for-web words: {text:.400}"
        );
        assert_eq!(doc.web_hidden_count(), 1);
    }

    #[test]
    fn extraction_round_trips_the_documents_words() {
        // The whole point of CID + ToUnicode: a reader (and a
        // screen reader, and a search box) gets back the words
        // the document holds, fields filled in as printed.
        let bytes = export_reader_pdf(&sample_doc(), ExportPurpose::ForPrinting);
        let text = extracted_text(&bytes);
        assert!(
            text.contains("Grace Lutheran Church"),
            "church name round-trips: {text:.400}"
        );
        assert!(
            text.contains("Pastor Lee"),
            "the filled-in pastor field round-trips: {text:.400}"
        );
        assert!(
            text.contains("Twentieth Sunday after Pentecost"),
            "the occasion name round-trips: {text:.400}"
        );
    }

    #[test]
    fn characters_beyond_winansi_draw_when_the_face_has_them() {
        // Greek is outside Windows-1252 but inside Liberation
        // Sans: with glyph addressing it must print, and the
        // Check predicate must agree (it uses the same oracle).
        let mut doc = sample_doc();
        let sid = doc.stories[0].id.clone();
        doc.story_mut(&sid)
            .expect("story")
            .paragraphs
            .push(ob_model::Paragraph::new(
                "body",
                "Peace \u{03B1} and \u{03C9}",
            ));
        let bytes = export_reader_pdf(&doc, ExportPurpose::ForPrinting);
        let text = extracted_text(&bytes);
        assert!(
            text.contains('\u{03B1}') && text.contains('\u{03C9}'),
            "the Greek letters extract back out of the PDF: {text:.400}"
        );
        assert!(
            undrawable_chars("Peace \u{03B1}", "Liberation Sans", false, false, &[]).is_empty(),
            "and the Check does not flag them"
        );
    }

    #[test]
    fn a_bold_run_switches_font_resource_mid_line() {
        // One paragraph, one line: "Sing " regular, "hymn"
        // bold, " 432" regular again. The content stream must
        // draw the middle word with the bold face's resource.
        let mut doc = sample_doc();
        let sid = doc.stories[0].id.clone();
        doc.story_mut(&sid).expect("story").paragraphs = vec![ob_model::Paragraph::from_runs(
            "body",
            vec![
                ob_model::Run::plain("Sing "),
                ob_model::Run {
                    text: "hymn".into(),
                    bold: Some(true),
                    italic: None,
                },
                ob_model::Run::plain(" 432"),
            ],
        )];
        let bytes = export_reader_pdf(&doc, ExportPurpose::ForPrinting);
        let raw = String::from_utf8_lossy(&bytes).into_owned();
        // Map each /FE resource to whether its font object is
        // a bold face, via the page resource tables and the
        // font dictionaries in the raw file.
        let mut bold_resources: Vec<String> = Vec::new();
        let mut plain_resources: Vec<String> = Vec::new();
        for (name, obj) in resource_object_ids(&raw) {
            let at = raw
                .find(&format!("\n{obj} 0 obj"))
                .or_else(|| raw.find(&format!("{obj} 0 obj")))
                .expect("font object present");
            let dict: String = raw[at..].chars().take(400).collect();
            let dict = dict.as_str();
            if dict.contains("Bold") {
                bold_resources.push(name);
            } else if dict.contains("/BaseFont") {
                plain_resources.push(name);
            }
        }
        assert!(
            !bold_resources.is_empty(),
            "a bold face was embedded for the run"
        );
        // The resource on the operation that shows a word,
        // with the shown text recovered through ToUnicode.
        let runs = extracted_runs(&bytes);
        let resource_showing = |word: &str| -> String {
            runs.iter()
                .find(|(_, t)| t == word)
                .map(|(r, _)| r.clone())
                .unwrap_or_else(|| panic!("no text op shows {word}: {runs:?}"))
        };
        assert!(
            bold_resources.contains(&resource_showing("hymn")),
            "the bold word draws with the bold resource"
        );
        assert!(
            plain_resources.contains(&resource_showing("Sing ")),
            "the plain word draws with a regular resource"
        );
        assert!(
            plain_resources.contains(&resource_showing(" 432")),
            "the trailing word is regular again"
        );
    }

    /// (resource name, object id) pairs from the pages'
    /// resource tables in a raw PDF.
    fn resource_object_ids(raw: &str) -> Vec<(String, usize)> {
        let mut out: Vec<(String, usize)> = Vec::new();
        let bytes = raw.as_bytes();
        let mut i = 0usize;
        while let Some(pos) = raw[i..].find("/FE").map(|p| p + i) {
            let rest = &raw[pos + 3..];
            let name_end = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(0);
            let (name, after) = if name_end > 0 {
                (
                    raw[pos + 1..pos + 3 + name_end].to_string(),
                    &rest[name_end..],
                )
            } else {
                i = pos + 3;
                continue;
            };
            let after = after.trim_start();
            let id_str: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
            if !id_str.is_empty() && after[id_str.len()..].starts_with(" 0 R") {
                let id: usize = id_str.parse().expect("object id");
                if !out.iter().any(|(n, _)| *n == name) {
                    out.push((name, id));
                }
            }
            i = pos + 3;
            let _ = bytes;
        }
        out
    }

    #[test]
    fn style_color_is_set_in_the_content_stream() {
        // A paragraph styled pure red must print red: the
        // decoded content stream carries `1.00 0.00 0.00 rg`
        // alongside its words, set on the text op itself.
        let mut doc = sample_doc();
        for style in &mut doc.styles {
            style.color = ob_model::Rgb::new(255, 0, 0);
        }
        let bytes = export_reader_pdf(&doc, ExportPurpose::ForEmailOrWeb);
        let content = decoded_content(&bytes);
        assert!(
            content.contains("1.00 0.00 0.00 rg"),
            "red text op missing its color: {content:.400}"
        );
        // And the default black still states itself plainly.
        let bytes = export_reader_pdf(&sample_doc(), ExportPurpose::ForEmailOrWeb);
        let content = decoded_content(&bytes);
        assert!(
            content.contains("0.00 0.00 0.00 rg"),
            "black text ops state black: {content:.400}"
        );
    }

    #[test]
    fn streams_are_flate_compressed_and_decode_intact() {
        let bytes = export_reader_pdf(&sample_doc(), ExportPurpose::ForEmailOrWeb);
        let text = String::from_utf8_lossy(&bytes);
        assert!(
            text.contains("/FlateDecode"),
            "content and font streams are compressed"
        );
        // Decompressing a content stream brings the operators
        // back exactly: text blocks carrying the glyph codes.
        let content = decoded_content(&bytes);
        assert!(
            content.contains("BT"),
            "text blocks survive: {content:.200}"
        );
        assert!(content.contains("Tj"), "show-text operators survive");
        assert!(
            extracted_text(&bytes).contains("Grace Lutheran Church"),
            "the words survive, recovered through ToUnicode"
        );
        // The embedded font stream is compressed too, and a
        // strict parser still reads the file structure whole.
        let parsed = lopdf::Document::load_mem(&bytes).expect("lopdf parses the export");
        assert_eq!(parsed.get_pages().len(), 4);
    }

    #[test]
    fn purpose_is_recorded_in_the_file() {
        let doc = sample_doc();
        let print = String::from_utf8_lossy(&export_imposed_pdf(&doc, ExportPurpose::ForPrinting))
            .into_owned();
        assert!(
            print.contains("/Producer (OpenBulletin)"),
            "producer recorded"
        );
        assert!(
            print.contains("Made for printing."),
            "print purpose recorded in words"
        );
        assert!(print.contains("300 pixels per inch"), "print cap recorded");
        let web = String::from_utf8_lossy(&export_reader_pdf(&doc, ExportPurpose::ForEmailOrWeb))
            .into_owned();
        assert!(
            web.contains("Made for email or the web."),
            "web purpose recorded in words"
        );
        let full = String::from_utf8_lossy(&export_reader_pdf(&doc, ExportPurpose::FullResolution))
            .into_owned();
        assert!(
            full.contains("Made at full resolution."),
            "full-resolution purpose recorded"
        );
    }

    #[test]
    fn imposed_pdf_has_two_sheet_sides() {
        let bytes = export_imposed_pdf(&sample_doc(), ExportPurpose::ForPrinting);
        let text = String::from_utf8_lossy(&bytes);
        assert!(
            text.contains("/Count 2"),
            "bifold imposes to 2 sides: {text:.200}"
        );
    }

    #[test]
    fn booklet_export_size_stays_down() {
        // The audit measured this fresh 16-panel booklet at
        // 1,254,242 bytes (reader) and 1,251,892 (imposed):
        // whole raw font faces, no compressed streams. Stream
        // compression alone (O1) roughly halved it. CID
        // embedding plus subsetting (O2) cuts each embedded
        // face to the glyphs the booklet actually draws, which
        // is where nearly all of the rest was. This bound
        // guards both wins.
        let doc = ob_layout::build_template(
            ob_layout::Preset::HalfLetterBooklet,
            ob_model::ChurchProfile::default(),
            chrono::NaiveDate::from_ymd_opt(2026, 10, 18).expect("date"),
            "Sunday",
        );
        let imposed = export_imposed_pdf(&doc, ExportPurpose::ForPrinting);
        let reader = export_reader_pdf(&doc, ExportPurpose::ForEmailOrWeb);
        println!(
            "booklet sizes after O2: imposed {} bytes, reader {} bytes",
            imposed.len(),
            reader.len()
        );
        assert!(
            imposed.len() < 400_000,
            "imposed booklet regressed: {} bytes",
            imposed.len()
        );
        assert!(
            reader.len() < 400_000,
            "reader booklet regressed: {} bytes",
            reader.len()
        );
    }

    /// The bytes of one stream object our writer produced:
    /// the payload between the stream markers, decompressed.
    fn stream_payload(object: &[u8]) -> Vec<u8> {
        let start = object
            .windows(b"stream\n".len())
            .position(|w| w == b"stream\n")
            .expect("stream marker")
            + b"stream\n".len();
        let end = object
            .windows(b"\nendstream".len())
            .rposition(|w| w == b"\nendstream")
            .expect("endstream marker");
        let raw = &object[start..end];
        if object[..start]
            .windows(b"FlateDecode".len())
            .any(|w| w == b"FlateDecode")
        {
            use std::io::Read as _;
            let mut dec = flate2::read::ZlibDecoder::new(raw);
            let mut out = Vec::new();
            dec.read_to_end(&mut out).expect("stream inflates");
            out
        } else {
            raw.to_vec()
        }
    }

    #[test]
    fn subsetting_shrinks_the_program_and_no_subsetting_stays_whole() {
        let (_, bytes) = ob_text::shaping::bundled_font_files()
            .into_iter()
            .find(|(name, _)| *name == "LiberationSans-Regular.ttf")
            .expect("bundled regular face");
        let face = ttf_parser::Face::parse(bytes, 0).expect("face parses");
        let used: std::collections::BTreeSet<char> = "abc".chars().collect();

        // Subset: three glyphs of a ~400 KB face leave a
        // program a small fraction of the source, and the
        // encoding names exactly those characters.
        let mut pdf = Pdf {
            objects: Vec::new(),
        };
        let subset_source = FaceSource {
            bytes: bytes.to_vec(),
            no_subsetting: false,
        };
        let (_, encoding) = add_cid_font(&mut pdf, &face, &subset_source, &used);
        let program = stream_payload(&pdf.objects[0]);
        assert!(
            program.len() < bytes.len() / 4,
            "subset program {} bytes vs source {} bytes",
            program.len(),
            bytes.len()
        );
        match &encoding {
            FaceEncoding::Cid(cids) => {
                assert_eq!(cids.len(), 3, "a, b and c each got a CID: {cids:?}");
                assert!(
                    cids.values().all(|cid| *cid > 0),
                    "no drawn character rides the missing-glyph CID"
                );
            }
            FaceEncoding::WinAnsi => panic!("a TrueType face encodes as CID"),
        }

        // No-subsetting: the same face embeds whole, its
        // program byte-for-byte the source, and characters
        // address their original glyph IDs.
        let mut pdf = Pdf {
            objects: Vec::new(),
        };
        let whole_source = FaceSource {
            bytes: bytes.to_vec(),
            no_subsetting: true,
        };
        let (_, encoding) = add_cid_font(&mut pdf, &face, &whole_source, &used);
        let program = stream_payload(&pdf.objects[0]);
        assert_eq!(program, bytes, "a no-subsetting face embeds whole");
        match &encoding {
            FaceEncoding::Cid(cids) => {
                for c in "abc".chars() {
                    assert_eq!(
                        cids.get(&c).copied(),
                        face.glyph_index(c).map(|g| g.0),
                        "whole-face CIDs are the original glyph IDs"
                    );
                }
            }
            FaceEncoding::WinAnsi => panic!("a TrueType face encodes as CID"),
        }
    }

    #[test]
    fn a_face_that_has_the_cross_draws_it() {
        // No bundled Liberation face carries U+271D (verified
        // against their character maps), but a library face
        // that has it must draw it and pass the Check: DejaVu
        // Sans is such a face. The file lives with the OS, not
        // in this repo, so the test steps aside where it is
        // absent instead of failing on a missing fixture.
        let path = "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf";
        let Ok(bytes) = std::fs::read(path) else {
            eprintln!("DejaVu Sans not present at {path}; cross test skipped");
            return;
        };
        let info = ob_fonts::inspect(&bytes, "DejaVuSans.ttf").expect("DejaVu inspects");
        assert_eq!(info.family, "DejaVu Sans");
        let library = vec![(info, bytes)];
        assert!(
            undrawable_chars("\u{271D}", "DejaVu Sans", false, false, &library).is_empty(),
            "the Check does not flag a character this face has"
        );
        let mut doc = sample_doc();
        for style in &mut doc.styles {
            style.font_family = "DejaVu Sans".into();
        }
        let sid = doc.stories[0].id.clone();
        doc.story_mut(&sid).expect("story").paragraphs =
            vec![ob_model::Paragraph::new("body", "Peace \u{271D}")];
        let sides = ob_impose::impose_single(&doc);
        let (pdf, _) = build_pdf_full(&doc, &sides, ExportPurpose::ForPrinting, &library, None);
        let text = extracted_text(&pdf);
        assert!(
            text.contains('\u{271D}'),
            "the cross extracts back out of the PDF: {text:.300}"
        );
    }

    /// A copy of a font whose family-name records (IDs 1
    /// and 16) say `new` instead of `old`: the fixture for
    /// a font the library gains later (see ob-text's copy
    /// of this helper).
    fn renamed_family(bytes: &[u8], old: &str, new: &str) -> Vec<u8> {
        let mut out = bytes.to_vec();
        let num_tables = u16::from_be_bytes([out[4], out[5]]) as usize;
        let mut name_off = 0usize;
        for i in 0..num_tables {
            let rec = 12 + i * 16;
            if &out[rec..rec + 4] == b"name" {
                name_off =
                    u32::from_be_bytes([out[rec + 8], out[rec + 9], out[rec + 10], out[rec + 11]])
                        as usize;
            }
        }
        assert!(name_off > 0, "the font has a name table");
        let count = u16::from_be_bytes([out[name_off + 2], out[name_off + 3]]) as usize;
        let storage = u16::from_be_bytes([out[name_off + 4], out[name_off + 5]]) as usize;
        let mut patched: Vec<usize> = Vec::new();
        for i in 0..count {
            let rec = name_off + 6 + i * 12;
            let platform = u16::from_be_bytes([out[rec], out[rec + 1]]);
            let name_id = u16::from_be_bytes([out[rec + 6], out[rec + 7]]);
            if name_id != 1 && name_id != 16 {
                continue;
            }
            let len = u16::from_be_bytes([out[rec + 8], out[rec + 9]]) as usize;
            let off = u16::from_be_bytes([out[rec + 10], out[rec + 11]]) as usize;
            let abs = name_off + storage + off;
            let raw = &bytes[abs..abs + len];
            let text: String = if platform == 1 {
                raw.iter().map(|&b| b as char).collect()
            } else {
                let units: Vec<u16> = raw
                    .chunks(2)
                    .map(|c| u16::from_be_bytes([c[0], c[1]]))
                    .collect();
                String::from_utf16_lossy(&units)
            };
            if text != old {
                continue;
            }
            let enc_len = if platform == 1 {
                new.len()
            } else {
                new.len() * 2
            };
            if !patched.contains(&abs) {
                let encoded: Vec<u8> = if platform == 1 {
                    new.bytes().collect()
                } else {
                    new.encode_utf16().flat_map(|u| u.to_be_bytes()).collect()
                };
                assert!(encoded.len() <= len, "the new name must fit the record");
                out[abs..abs + encoded.len()].copy_from_slice(&encoded);
                for b in &mut out[abs + encoded.len()..abs + len] {
                    *b = 0;
                }
                patched.push(abs);
            }
            out[rec + 8..rec + 10].copy_from_slice(&(enc_len as u16).to_be_bytes());
        }
        assert!(!patched.is_empty(), "a family record was patched");
        out
    }

    /// One style, one story, one page: a document made in
    /// `family`, saying words that include the Latin cross.
    fn gill_doc(family: &str) -> Document {
        use ob_model::{
            ContentRole, Frame, FrameKind, Paragraph, SheetSpec, Size, Story, FORMAT_VERSION,
        };
        let style = ob_model::ParagraphStyle {
            id: "body".into(),
            name: "Body".into(),
            font_size: 11_000,
            bold: false,
            italic: false,
            align: ob_model::Align::Left,
            space_after: 4_000,
            color: ob_model::Rgb::BLACK,
            font_family: family.into(),
            font_sha256: None,
        };
        Document {
            format_version: FORMAT_VERSION,
            title: "Gill test".into(),
            profile: ChurchProfile::default(),
            fields: Default::default(),
            styles: vec![style],
            stories: vec![Story {
                id: "s1".into(),
                paragraphs: vec![Paragraph::new(
                    "body",
                    "Peace \u{271D} and quiet rest for all who gather \
                     here this morning in the house of prayer",
                )],
                keep_through: None,
            }],
            panels: vec![Panel {
                id: "p1".into(),
                name: "Page 1".into(),
                size: Size::new(612_000, 792_000),
                frames: vec![Frame {
                    id: "f1".into(),
                    name: "Words".into(),
                    kind: FrameKind::Text,
                    rect: Rect::new(72_000, 72_000, 200_000, 400_000),
                    story_id: Some("s1".into()),
                    role: ContentRole::Weekly,
                    picture: None,
                    music: None,
                }],
                guides: Vec::new(),
                background_image: None,
            }],
            sheet: SheetSpec {
                preset: "letter-portrait".into(),
                size: Size::new(612_000, 792_000),
            },
            no_announcements: false,
            assets: Vec::new(),
            asset_bytes: Default::default(),
            occasion_time: None,
            wanted_fonts: Vec::new(),
            participants: Default::default(),
            inserts: Default::default(),
        }
    }

    /// Page content with the embedded-face resource names
    /// (FE1, FE2, ...) normalized away: two exports draw
    /// the same words in the same face exactly when their
    /// normalized content agrees, whatever order the faces
    /// were assigned in.
    fn normalized_content(pdf: &[u8]) -> String {
        let raw = decoded_content(pdf);
        let mut out = String::with_capacity(raw.len());
        let mut rest = raw.as_str();
        while let Some(i) = rest.find("/FE") {
            out.push_str(&rest[..i]);
            out.push_str("/FE");
            rest = &rest[i + 3..];
            let digits = rest.chars().take_while(|c| c.is_ascii_digit()).count();
            rest = &rest[digits..];
        }
        out.push_str(rest);
        out
    }

    #[test]
    fn a_wanted_font_prints_in_its_lookalike_then_in_itself() {
        // The install-later promise on the print path
        // (docs/FONTS.md). The document is made in Gill
        // Sans MT, a commercial font no test may ship; the
        // faces here are free fonts wearing other names,
        // which is exactly what the library holds at each
        // stage. DejaVu carries the cross and Liberation
        // does not, so the cross in the extracted text tells
        // us which face truly printed.
        let path = "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf";
        let Ok(dejavu) = std::fs::read(path) else {
            eprintln!("DejaVu Sans not present at {path}; wanted-font print test skipped");
            return;
        };
        // Cabin wears DejaVu's bytes (it carries the
        // cross); the Gill-named face wears Liberation
        // Sans's (it does not). Which face printed is
        // therefore visible in the extracted text.
        let cabin = renamed_family(&dejavu, "DejaVu Sans", "Cabin");
        let gill = renamed_family(
            include_bytes!("../../ob-text/assets/fonts/LiberationSans-Regular.ttf"),
            "Liberation Sans",
            "Gill Sans MT",
        );
        let cabin_lib = vec![(
            ob_fonts::inspect(&cabin, "Cabin.ttf").expect("cabin inspects"),
            cabin.clone(),
        )];
        let full_lib = vec![
            (
                ob_fonts::inspect(&cabin, "Cabin.ttf").expect("cabin inspects"),
                cabin,
            ),
            (
                ob_fonts::inspect(&gill, "GillSansMT.ttf").expect("gill inspects"),
                gill,
            ),
        ];
        let export = |doc: &Document,
                      library: &[(ob_fonts::FontInfo, Vec<u8>)]|
         -> (Vec<u8>, String) {
            let sides = ob_impose::impose_single(doc);
            let (pdf, _) = build_pdf_full(doc, &sides, ExportPurpose::ForPrinting, library, None);
            let text = extracted_text(&pdf);
            (pdf, text)
        };
        let doc = gill_doc("Gill Sans MT");
        // Stage 1: only the lookalike is in the library.
        // The export must draw exactly as the same words
        // set in DejaVu Sans itself (the bytes Cabin wears
        // here), cross and all.
        let (pdf_look, text_look) = export(&doc, &cabin_lib);
        let control = gill_doc("DejaVu Sans");
        let dejavu_lib = vec![(
            ob_fonts::inspect(&dejavu, "DejaVuSans.ttf").expect("dejavu inspects"),
            dejavu.clone(),
        )];
        let (pdf_dejavu, _) = export(&control, &dejavu_lib);
        assert_eq!(
            normalized_content(&pdf_look),
            normalized_content(&pdf_dejavu),
            "with only its lookalike present, the Gill words print in the lookalike"
        );
        assert!(
            text_look.contains('\u{271D}'),
            "the lookalike face carries the cross: {text_look:.200}"
        );
        // Stage 2: the real name joins the library (a face
        // whose own family is Gill Sans MT). The same
        // document, unedited, now prints in that face:
        // exactly as the words set in Liberation Sans
        // itself (the bytes the Gill-named face wears),
        // and the cross is gone with the stand-in.
        let (pdf_real, text_real) = export(&doc, &full_lib);
        let control = gill_doc("Liberation Sans");
        let (pdf_sans, _) = export(&control, &[]);
        assert_eq!(
            normalized_content(&pdf_real),
            normalized_content(&pdf_sans),
            "once the real font is in the library, the same document prints in it"
        );
        assert!(
            !text_real.contains('\u{271D}'),
            "the real face here has no cross: {text_real:.200}"
        );
        // And the Check oracle walks the same chain.
        let oracle = FaceOracle::new(&full_lib);
        assert_eq!(
            oracle.undrawable_chars("\u{271D}", "Gill Sans MT", false, false),
            vec!['\u{271D}']
        );
        let oracle = FaceOracle::new(&cabin_lib);
        assert!(oracle
            .undrawable_chars("\u{271D}", "Gill Sans MT", false, false)
            .is_empty());
    }

    #[test]
    fn booklet_exports_eight_sheet_sides() {
        let doc = ob_layout::build_template(
            ob_layout::Preset::HalfLetterBooklet,
            ob_model::ChurchProfile::default(),
            chrono::NaiveDate::from_ymd_opt(2026, 10, 18).expect("date"),
            "Sunday",
        );
        let pdf = export_imposed_pdf(&doc, ExportPurpose::ForPrinting);
        let parsed = lopdf::Document::load_mem(&pdf).expect("lopdf parses the booklet");
        assert_eq!(parsed.get_pages().len(), 8);
    }

    #[test]
    fn exports_embed_the_library_fonts() {
        let doc = sample_doc();
        let pdf = export_reader_pdf(&doc, ExportPurpose::ForEmailOrWeb);
        let text = String::from_utf8_lossy(&pdf);
        assert!(
            text.contains("/Subtype /Type0"),
            "faces embedded as CID-keyed Type0 fonts"
        );
        assert!(
            text.contains("/Subtype /CIDFontType2"),
            "with CIDFontType2 descendants"
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
        // Liberation Sans at 2048 units per em, verified against
        // the font file itself: space advances 569 units (277
        // thousandths scaled) and A advances 1366 (666). The
        // regular face's CIDFont /W entries, keyed through its
        // ToUnicode map, must carry the real font's own numbers
        // for the glyphs drawn, not core-font stand-ins.
        let mut doc = sample_doc();
        let sid = doc.stories[0].id.clone();
        doc.story_mut(&sid)
            .expect("story")
            .paragraphs
            .push(ob_model::Paragraph::new("body", "A A"));
        let pdf = export_reader_pdf(&doc, ExportPurpose::ForEmailOrWeb);
        let parsed = lopdf::Document::load_mem(&pdf).expect("lopdf parses the export");
        // Walk the page resources to the regular face's Type0
        // dictionary, then down to its CIDFont descendant.
        let mut checked = false;
        for (_, page_id) in parsed.get_pages() {
            let Ok((Some(resources), _)) = parsed.get_page_resources(page_id) else {
                continue;
            };
            let Ok(font_dict) = resources.get(b"Font").and_then(|o| o.as_dict()) else {
                continue;
            };
            for (_, obj) in font_dict.iter() {
                let Some(font_obj) = obj
                    .as_reference()
                    .ok()
                    .and_then(|id| parsed.get_object(id).ok())
                else {
                    continue;
                };
                let Ok(dict) = font_obj.as_dict() else {
                    continue;
                };
                let base = dict
                    .get(b"BaseFont")
                    .and_then(|o| o.as_name())
                    .map(|n| String::from_utf8_lossy(n).into_owned())
                    .unwrap_or_default();
                if base != "LiberationSans" {
                    continue;
                }
                let to_unicode = to_unicode_map(&parsed, dict);
                let by_char: std::collections::HashMap<char, u16> = to_unicode
                    .iter()
                    .filter_map(|(cid, s)| s.chars().next().map(|c| (c, *cid)))
                    .collect();
                let descendants = dict
                    .get(b"DescendantFonts")
                    .and_then(|o| o.as_array())
                    .expect("Type0 names its descendant");
                let cidfont = descendants[0]
                    .as_reference()
                    .and_then(|id| parsed.get_object(id))
                    .expect("descendant object")
                    .as_dict()
                    .expect("descendant dictionary")
                    .clone();
                let w = cidfont
                    .get(b"W")
                    .and_then(|o| o.as_array())
                    .expect("CIDFont carries /W");
                let mut widths: std::collections::HashMap<u16, i64> =
                    std::collections::HashMap::new();
                let mut i = 0;
                while i + 1 < w.len() {
                    if let (Ok(cid), Ok(arr)) = (w[i].as_i64(), w[i + 1].as_array()) {
                        for (j, width) in arr.iter().enumerate() {
                            if let Ok(width) = width.as_i64() {
                                widths.insert((cid as u16) + (j as u16), width);
                            }
                        }
                    }
                    i += 2;
                }
                assert_eq!(
                    widths.get(&by_char[&' ']),
                    Some(&277),
                    "space keeps the face's own advance"
                );
                assert_eq!(
                    widths.get(&by_char[&'A']),
                    Some(&666),
                    "A keeps the face's own advance"
                );
                checked = true;
            }
        }
        assert!(checked, "the regular Liberation Sans face was embedded");
    }

    #[test]
    fn reader_pages_are_panel_sized() {
        let doc = sample_doc();
        let pdf = String::from_utf8_lossy(&export_reader_pdf(&doc, ExportPurpose::ForEmailOrWeb))
            .to_string();
        // Bi-fold panel is 5.5 x 8.5 in = 396 x 612 pt; sheet is 792 x 612.
        assert!(
            pdf.contains("/MediaBox [0 0 396.00 612.00]"),
            "reader pages must be panel-sized"
        );
        assert!(
            !pdf.contains("/MediaBox [0 0 792.00 612.00]"),
            "reader pages must not be sheet-sized"
        );
        let imposed =
            String::from_utf8_lossy(&export_imposed_pdf(&doc, ExportPurpose::ForPrinting))
                .to_string();
        assert!(
            imposed.contains("/MediaBox [0 0 792.00 612.00]"),
            "imposed pages must be sheet-sized"
        );
    }

    /// A document with one picture box on its first panel,
    /// plus the picture's bytes when `with_bytes` (the asset
    /// record always exists; the bytes are what can go
    /// missing).
    fn picture_doc(with_bytes: bool) -> (Document, String) {
        let mut doc = sample_doc();
        let png = {
            let mut rgba = vec![0u8; 1200 * 900 * 4];
            for px in rgba.chunks_exact_mut(4) {
                px[0] = 30;
                px[1] = 90;
                px[2] = 160;
                px[3] = 255;
            }
            let decoded = ob_image::Decoded {
                width: 1200,
                height: 900,
                rgba,
                has_alpha: false,
            };
            ob_image::encode_png(&decoded).expect("encode test png")
        };
        let sha = doc.add_asset(
            &png,
            ob_model::AssetKind::Picture,
            "image/png",
            ob_model::RightsRecord::default(),
        );
        if !with_bytes {
            doc.asset_bytes.remove(&sha);
        }
        let panel = &mut doc.panels[0];
        panel.frames.push(ob_model::Frame {
            id: "pic-1".into(),
            name: "Cover picture".into(),
            kind: ob_model::FrameKind::Picture,
            rect: Rect::new(36_000, 36_000, 216_000, 162_000),
            story_id: None,
            role: ob_model::ContentRole::Weekly,
            picture: Some(ob_model::PicturePlacement::new(sha.clone())),
            music: None,
        });
        (doc, sha)
    }

    #[test]
    fn a_placed_picture_embeds_right_sized_for_the_purpose() {
        let (doc, _sha) = picture_doc(true);
        // The box is 3 x 2.25 in. For printing (300/in) the
        // 1200x900 picture right-sizes to 900x675 as a JPEG.
        let (pdf, report) = export_reader_pdf_report(&doc, ExportPurpose::ForPrinting);
        assert_eq!(report.missing_pictures, 0);
        let text = String::from_utf8_lossy(&pdf).to_string();
        assert!(
            text.contains("/Subtype /Image"),
            "an image XObject is embedded"
        );
        assert!(
            text.contains("/Width 900 /Height 675"),
            "right-sized to 300/in: {text:.0}"
        );
        assert!(
            text.contains("/Filter /DCTDecode"),
            "opaque pictures ride as JPEG"
        );
        let content = decoded_content(&pdf);
        assert!(
            content.contains(" re W n "),
            "the picture is clipped to its box"
        );
        assert!(content.contains("/Im0 Do"), "the picture is drawn");
        // For email or the web (150/in) it right-sizes to half.
        let (pdf, _) = export_reader_pdf_report(&doc, ExportPurpose::ForEmailOrWeb);
        let text = String::from_utf8_lossy(&pdf).to_string();
        assert!(
            text.contains("/Width 450 /Height 337") || text.contains("/Width 450 /Height 338"),
            "right-sized to 150/in"
        );
        // Full resolution keeps the picture's own pixels.
        let (pdf, _) = export_reader_pdf_report(&doc, ExportPurpose::FullResolution);
        let text = String::from_utf8_lossy(&pdf).to_string();
        assert!(
            text.contains("/Width 1200 /Height 900"),
            "no cap, no shrink"
        );
    }

    #[test]
    fn a_picture_with_transparency_embeds_with_a_mask() {
        let mut doc = sample_doc();
        let decoded = ob_image::Decoded {
            width: 40,
            height: 40,
            rgba: {
                let mut v = vec![0u8; 40 * 40 * 4];
                for px in v.chunks_exact_mut(4) {
                    px[0] = 200;
                    px[3] = 128;
                }
                v
            },
            has_alpha: true,
        };
        let png = ob_image::encode_png(&decoded).expect("encode alpha png");
        let sha = doc.add_asset(
            &png,
            ob_model::AssetKind::Picture,
            "image/png",
            ob_model::RightsRecord::default(),
        );
        doc.panels[0].frames.push(ob_model::Frame {
            id: "pic-a".into(),
            name: "See-through picture".into(),
            kind: ob_model::FrameKind::Picture,
            rect: Rect::new(36_000, 36_000, 72_000, 72_000),
            story_id: None,
            role: ob_model::ContentRole::Weekly,
            picture: Some(ob_model::PicturePlacement::new(sha)),
            music: None,
        });
        let (pdf, report) = export_reader_pdf_report(&doc, ExportPurpose::ForPrinting);
        assert_eq!(report.missing_pictures, 0);
        let text = String::from_utf8_lossy(&pdf).to_string();
        assert!(text.contains("/SMask"), "alpha rides as a mask");
        assert!(
            text.contains("/ColorSpace /DeviceGray"),
            "the mask is grayscale"
        );
        assert!(
            text.contains("/Filter /FlateDecode"),
            "alpha pictures ride flate"
        );
    }

    #[test]
    fn a_missing_picture_prints_the_placeholder_and_counts() {
        let (doc, _sha) = picture_doc(false);
        let (pdf, report) = export_reader_pdf_report(&doc, ExportPurpose::ForPrinting);
        assert_eq!(
            report.missing_pictures, 1,
            "the export counts the placeholder"
        );
        let text = extracted_text(&pdf);
        assert!(
            text.contains("No picture yet"),
            "the box says what happened, in words: {text:.200}"
        );
        // An empty picture box (no placement at all) counts
        // the same way.
        let mut doc2 = sample_doc();
        doc2.panels[0].frames.push(ob_model::Frame {
            id: "pic-empty".into(),
            name: "Empty picture box".into(),
            kind: ob_model::FrameKind::Picture,
            rect: Rect::new(36_000, 36_000, 72_000, 72_000),
            story_id: None,
            role: ob_model::ContentRole::Weekly,
            picture: None,
            music: None,
        });
        let (_, report) = export_reader_pdf_report(&doc2, ExportPurpose::ForPrinting);
        assert_eq!(report.missing_pictures, 1);
    }

    #[test]
    fn an_svg_picture_prints_at_the_purposes_resolution() {
        let mut doc = sample_doc();
        let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="72" height="72" viewBox="0 0 72 72"><rect width="72" height="72" fill="#336699"/></svg>"##;
        let sha = doc.add_asset(
            svg,
            ob_model::AssetKind::Picture,
            "image/svg+xml",
            ob_model::RightsRecord::default(),
        );
        doc.panels[0].frames.push(ob_model::Frame {
            id: "pic-svg".into(),
            name: "Vector picture".into(),
            kind: ob_model::FrameKind::Picture,
            rect: Rect::new(36_000, 36_000, 72_000, 72_000),
            story_id: None,
            role: ob_model::ContentRole::Weekly,
            picture: Some(ob_model::PicturePlacement::new(sha)),
            music: None,
        });
        // The box is 1x1 in: for printing the art renders at
        // 300 pixels, whatever its file's nominal size says.
        let (pdf, report) = export_reader_pdf_report(&doc, ExportPurpose::ForPrinting);
        assert_eq!(report.missing_pictures, 0);
        let text = String::from_utf8_lossy(&pdf).to_string();
        assert!(
            text.contains("/Width 300 /Height 300"),
            "SVG renders at the copy's purpose resolution"
        );
    }

    fn fake_info(family: &str, bold: bool, italic: bool) -> ob_fonts::FontInfo {
        ob_fonts::FontInfo {
            family: family.into(),
            full_name: family.into(),
            bold,
            italic,
            rights: ob_fonts::EmbeddingRights::Installable,
            no_subsetting: false,
            file_name: "fake.ttf".into(),
            sha256: String::new(),
            bytes: 0,
        }
    }

    #[test]
    fn face_sources_keeps_the_last_face_of_a_key() {
        // R3-13: the writer's per-key winner is the shared
        // pick rule's answer, the last candidate, so the
        // face embedded is the face the pin names.
        let library = vec![
            (fake_info("Acme Sans", false, false), b"first".to_vec()),
            (fake_info("Acme Sans", false, false), b"second".to_vec()),
        ];
        let sources = face_sources(&library);
        let got = &sources[&("Acme Sans".to_string(), false, false)];
        assert_eq!(got.bytes, b"second".to_vec());
    }

    #[test]
    fn source_families_picks_one_spelling_deterministically() {
        // R3-14: two spellings of one family in the
        // library; the answer is the same whatever order
        // the map is walked in. Most faces wins.
        let mut sources: std::collections::HashMap<FaceKey, FaceSource> =
            std::collections::HashMap::new();
        let src = FaceSource {
            bytes: Vec::new(),
            no_subsetting: false,
        };
        sources.insert(("Acme Sans".into(), false, false), src.clone());
        sources.insert(("ACME SANS".into(), true, false), src.clone());
        sources.insert(("ACME SANS".into(), false, true), src);
        let families = source_families(&sources);
        assert_eq!(families, vec!["ACME SANS".to_string()]);
        // A tie goes to the lexicographically smallest.
        let mut tied: std::collections::HashMap<FaceKey, FaceSource> =
            std::collections::HashMap::new();
        let src = FaceSource {
            bytes: Vec::new(),
            no_subsetting: false,
        };
        tied.insert(("acme sans".into(), false, false), src.clone());
        tied.insert(("Acme Sans".into(), true, false), src);
        let families = source_families(&tied);
        assert_eq!(families, vec!["Acme Sans".to_string()]);
    }
}
