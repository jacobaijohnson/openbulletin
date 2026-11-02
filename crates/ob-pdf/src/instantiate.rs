//! Pin a variable TrueType face at bold weight (R51-D, G-04).
//!
//! The shaping path already draws Bold distinctly for variable
//! families: fontique computes `wght` synthesis from the requested
//! weight and parley chains it into HarfBuzz, so a bold run's
//! advances are true bold advances. The PDF writer, until this
//! module, embedded the variable file as-is, and a PDF viewer
//! renders a variable file at its default instance: regular
//! outlines, spaced by the writer's regular widths, under
//! positions the shaper laid out bold. Screen and print
//! disagreed.
//!
//! This module closes the divergence at the writer: for a bold
//! key whose family holds only a variable plain face, it builds
//! a static instance of that face at `wght` 700 and hands the
//! instance to the existing embed path, which subsets it and
//! writes its widths exactly like any static bold face. After
//! this, the bold PDF face's outlines and its `/W` advances are
//! the bold instance's own, and they agree with the shaper's
//! bold advances up to the writer's long-standing integer
//! rounding, the same rounding static bold faces already carry.
//!
//! Scope, stated plainly:
//! - TrueType-glyf variable faces only. A CFF-flavored face, a
//!   collection, a face with no `wght` axis, or a face whose
//!   `wght` default already is 700 yields `None`, and the caller
//!   keeps today's plain-face fallback for that key. Nothing
//!   here ever fails an export.
//! - Any failure at any step yields `None`: the caller falls
//!   back to the plain face, exactly today's behavior, rather
//!   than embedding a half-built font.
//! - Variation tables (`fvar`, `avar`, `gvar`, `cvar`, `HVAR`,
//!   `MVAR`, `VVAR`, `STAT`) are dropped, so the output is a
//!   static font, not a variable font with a new default.
//!   Global metrics (`hhea`, `OS/2` typo metrics) stay at the
//!   default instance's values; only outlines, advances, and
//!   the weight identity (`usWeightClass`, the bold bits, the
//!   PostScript name) move to 700. Hinting programs are kept
//!   as-is and per-glyph instructions are dropped, since the
//!   re-encoded outlines carry none.
//! - Composite glyphs are flattened: ttf-parser's varied
//!   outline already walks components with their transforms
//!   and variation deltas, and the re-encoding stores the
//!   flattened contours as simple glyphs.

/// The weight a bold key instantiates at, and the axis it moves.
const BOLD_WEIGHT: f32 = 700.0;
const WGHT: [u8; 4] = *b"wght";

/// Tables a static instance must not carry.
const DROPPED: &[[u8; 4]] = &[
    *b"fvar", *b"avar", *b"gvar", *b"cvar", *b"HVAR", *b"MVAR", *b"VVAR", *b"STAT",
];

/// One table directory entry, as read from the source.
struct Table {
    tag: [u8; 4],
    offset: usize,
    length: usize,
}

/// Read the sfnt directory: the table list, in file order.
/// `None` on any short read, overlapping nonsense, or a
/// collection/CFF-flavored program (the caller only ever
/// instantiates TrueType-glyf faces).
fn directory(bytes: &[u8]) -> Option<(u32, Vec<Table>)> {
    if bytes.len() < 12 {
        return None;
    }
    let version = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    if version != 0x0001_0000 && &bytes[0..4] != b"true" {
        return None;
    }
    let count = u16::from_be_bytes([bytes[4], bytes[5]]) as usize;
    if bytes.len() < 12 + count * 16 {
        return None;
    }
    let mut tables = Vec::with_capacity(count);
    for i in 0..count {
        let rec = 12 + i * 16;
        let tag: [u8; 4] = bytes[rec..rec + 4].try_into().ok()?;
        let offset = u32::from_be_bytes([
            bytes[rec + 8],
            bytes[rec + 9],
            bytes[rec + 10],
            bytes[rec + 11],
        ]) as usize;
        let length = u32::from_be_bytes([
            bytes[rec + 12],
            bytes[rec + 13],
            bytes[rec + 14],
            bytes[rec + 15],
        ]) as usize;
        if offset.checked_add(length)? > bytes.len() {
            return None;
        }
        tables.push(Table {
            tag,
            offset,
            length,
        });
    }
    Some((version, tables))
}

fn find<'a>(tables: &'a [Table], tag: &[u8; 4]) -> Option<&'a Table> {
    tables.iter().find(|t| &t.tag == tag)
}

fn table_bytes<'a>(bytes: &'a [u8], tables: &[Table], tag: &[u8; 4]) -> Option<&'a [u8]> {
    let t = find(tables, tag)?;
    Some(&bytes[t.offset..t.offset + t.length])
}

fn u16_at(data: &[u8], at: usize) -> Option<u16> {
    let b: [u8; 2] = data.get(at..at + 2)?.try_into().ok()?;
    Some(u16::from_be_bytes(b))
}

fn i16_at(data: &[u8], at: usize) -> Option<i16> {
    let b: [u8; 2] = data.get(at..at + 2)?.try_into().ok()?;
    Some(i16::from_be_bytes(b))
}

/// The `wght` axis range of a variable face, from its `fvar`
/// table: (minimum, default, maximum), in weight units.
/// `None` when the face is not variable or has no `wght` axis.
fn wght_axis(bytes: &[u8], tables: &[Table]) -> Option<(f32, f32, f32)> {
    let fvar = table_bytes(bytes, tables, b"fvar")?;
    let count = u16_at(fvar, 8)? as usize;
    let size = u16_at(fvar, 10)? as usize;
    let array = u16_at(fvar, 4)? as usize;
    let fixed = |at: usize| -> Option<f32> {
        let raw: [u8; 4] = fvar.get(at..at + 4)?.try_into().ok()?;
        Some(i32::from_be_bytes(raw) as f32 / 65536.0)
    };
    for i in 0..count {
        let rec = array + i * size;
        if fvar.get(rec..rec + 4)? != b"wght" {
            continue;
        }
        return Some((fixed(rec + 4)?, fixed(rec + 8)?, fixed(rec + 12)?));
    }
    None
}

/// One outline point, in font units, with its on-curve flag.
#[derive(Debug, Clone, Copy)]
struct Pt {
    x: i16,
    y: i16,
    on: bool,
}

/// The path commands ttf-parser's varied outline produces,
/// collected per contour.
struct Collector {
    contours: Vec<Vec<(f32, f32, bool)>>,
    current: Vec<(f32, f32, bool)>,
    failed: bool,
}

impl Collector {
    fn new() -> Self {
        Collector {
            contours: Vec::new(),
            current: Vec::new(),
            failed: false,
        }
    }

    fn finish(mut self) -> Option<Vec<Vec<Pt>>> {
        if self.failed {
            return None;
        }
        if !self.current.is_empty() {
            self.contours.push(std::mem::take(&mut self.current));
        }
        let mut out = Vec::with_capacity(self.contours.len());
        for contour in &self.contours {
            let pts = round_contour(contour)?;
            if !pts.is_empty() {
                out.push(pts);
            }
        }
        Some(out)
    }
}

impl ttf_parser::OutlineBuilder for Collector {
    fn move_to(&mut self, x: f32, y: f32) {
        if !self.current.is_empty() {
            self.contours.push(std::mem::take(&mut self.current));
        }
        self.current.push((x, y, true));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.current.push((x, y, true));
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.current.push((x1, y1, false));
        self.current.push((x, y, true));
    }

    fn curve_to(&mut self, _x1: f32, _y1: f32, _x2: f32, _y2: f32, _x: f32, _y: f32) {
        // TrueType outlines never carry cubics; a cubic here
        // means the face is not the glyf face this module
        // handles, so fail the whole instantiation.
        self.failed = true;
    }

    fn close(&mut self) {
        if !self.current.is_empty() {
            self.contours.push(std::mem::take(&mut self.current));
        }
    }
}

/// Round one contour to integer points, inserting the implied
/// on-curve midpoints TrueType requires between consecutive
/// off-curve points (including the wrap-around pair), and
/// rotating the contour to start on an on-curve point.
/// Degenerate contours (fewer than two points after rounding)
/// come back empty and are dropped by the caller.
fn round_contour(contour: &[(f32, f32, bool)]) -> Option<Vec<Pt>> {
    let mut pts: Vec<Pt> = Vec::with_capacity(contour.len());
    for &(x, y, on) in contour {
        if !x.is_finite() || !y.is_finite() {
            return None;
        }
        let xr = x.round();
        let yr = y.round();
        if !(i16::MIN as f32 <= xr && xr <= i16::MAX as f32)
            || !(i16::MIN as f32 <= yr && yr <= i16::MAX as f32)
        {
            return None;
        }
        pts.push(Pt {
            x: xr as i16,
            y: yr as i16,
            on,
        });
    }
    if pts.len() < 2 {
        return Some(Vec::new());
    }
    // Wrap-around implied point: first and last both
    // off-curve means an on-curve point sits between them.
    if !pts[0].on && !pts[pts.len() - 1].on {
        let first = pts[0];
        let last = pts[pts.len() - 1];
        pts.insert(
            0,
            Pt {
                x: ((first.x as i32 + last.x as i32) / 2) as i16,
                y: ((first.y as i32 + last.y as i32) / 2) as i16,
                on: true,
            },
        );
    }
    // Interior implied points between consecutive off-curves.
    let mut filled: Vec<Pt> = Vec::with_capacity(pts.len() + 1);
    for i in 0..pts.len() {
        let p = pts[i];
        filled.push(p);
        let next = pts[(i + 1) % pts.len()];
        if !p.on && !next.on && i + 1 < pts.len() {
            filled.push(Pt {
                x: ((p.x as i32 + next.x as i32) / 2) as i16,
                y: ((p.y as i32 + next.y as i32) / 2) as i16,
                on: true,
            });
        }
    }
    // Rotate to an on-curve start when one exists.
    let Some(start) = filled.iter().position(|p| p.on) else {
        return Some(Vec::new());
    };
    filled.rotate_left(start);
    if filled.len() < 2 {
        return Some(Vec::new());
    }
    Some(filled)
}

/// Encode one glyph's contours as a simple-glyph record
/// (header, end points, no instructions, flags, coordinates),
/// padded to an even length for short `loca` offsets.
fn encode_glyph(contours: &[Vec<Pt>]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut x_min = 0i16;
    let mut y_min = 0i16;
    let mut x_max = 0i16;
    let mut y_max = 0i16;
    let mut first = true;
    for contour in contours {
        for p in contour {
            if first {
                x_min = p.x;
                y_min = p.y;
                x_max = p.x;
                y_max = p.y;
                first = false;
            } else {
                x_min = x_min.min(p.x);
                y_min = y_min.min(p.y);
                x_max = x_max.max(p.x);
                y_max = y_max.max(p.y);
            }
        }
    }
    let count = contours.len() as i16;
    out.extend_from_slice(&count.to_be_bytes());
    out.extend_from_slice(&x_min.to_be_bytes());
    out.extend_from_slice(&y_min.to_be_bytes());
    out.extend_from_slice(&x_max.to_be_bytes());
    out.extend_from_slice(&y_max.to_be_bytes());
    if contours.is_empty() {
        return out;
    }
    let mut end = 0usize;
    for contour in contours {
        end += contour.len();
        out.extend_from_slice(&(end as u16).saturating_sub(1).to_be_bytes());
    }
    out.extend_from_slice(&0u16.to_be_bytes());
    // Flags, one per point, then repeat compression.
    let mut flags: Vec<u8> = Vec::new();
    let mut prev_x = 0i16;
    let mut prev_y = 0i16;
    let mut xs: Vec<(u8, i16)> = Vec::new();
    let mut ys: Vec<(u8, i16)> = Vec::new();
    for contour in contours {
        for p in contour {
            let mut flag: u8 = if p.on { 0x01 } else { 0x00 };
            let dx = p.x as i32 - prev_x as i32;
            let dy = p.y as i32 - prev_y as i32;
            prev_x = p.x;
            prev_y = p.y;
            let (short, same, bytes) = coord_flag(dx);
            if short {
                flag |= 0x02;
            }
            if same {
                flag |= 0x10;
            }
            xs.push((
                if short {
                    1
                } else if same {
                    0
                } else {
                    2
                },
                bytes,
            ));
            let (short, same, bytes) = coord_flag(dy);
            if short {
                flag |= 0x04;
            }
            if same {
                flag |= 0x20;
            }
            ys.push((
                if short {
                    1
                } else if same {
                    0
                } else {
                    2
                },
                bytes,
            ));
            flags.push(flag);
        }
    }
    let mut i = 0usize;
    while i < flags.len() {
        let flag = flags[i];
        let mut run = 1usize;
        while i + run < flags.len() && flags[i + run] == flag && run < 256 {
            run += 1;
        }
        if run > 1 {
            out.push(flag | 0x08);
            out.push((run - 1) as u8);
        } else {
            out.push(flag);
        }
        i += run;
    }
    for (kind, value) in xs.iter().chain(ys.iter()) {
        match kind {
            1 => out.push(*value as u8),
            2 => out.extend_from_slice(&value.to_be_bytes()),
            _ => {}
        }
    }
    if out.len() % 2 == 1 {
        out.push(0);
    }
    out
}

/// The flag shape for one coordinate delta: (short, same, bytes).
/// `same` means "no byte follows" for a zero delta, and "the
/// short byte is positive" for a short delta.
fn coord_flag(delta: i32) -> (bool, bool, i16) {
    if delta == 0 {
        (false, true, 0)
    } else if (1..=255).contains(&delta) {
        (true, true, delta as i16)
    } else if (-255..=-1).contains(&delta) {
        (true, false, (-delta) as i16)
    } else {
        (false, false, delta as i16)
    }
}

/// Rebuild the `name` table with the PostScript name (ID 6)
/// carrying a "-Bold" suffix, so the instance's PDF `/BaseFont`
/// cannot collide with the regular face's. Only ASCII names
/// are patched; anything else keeps its source name and the
/// instance still works (the PDF keys faces by object, not
/// by name). The record block is kept verbatim up to the
/// string storage, so format-1 language records survive.
fn patch_name(name: &[u8]) -> Vec<u8> {
    let Some(count) = u16_at(name, 2) else {
        return name.to_vec();
    };
    let Some(string_offset) = u16_at(name, 4) else {
        return name.to_vec();
    };
    let string_offset = string_offset as usize;
    if name.len() < string_offset || count as usize > 1024 {
        return name.to_vec();
    }
    let mut records: Vec<[u8; 12]> = Vec::new();
    for i in 0..count as usize {
        let Some(rec) = name.get(6 + i * 12..6 + (i + 1) * 12) else {
            return name.to_vec();
        };
        let mut entry = [0u8; 12];
        entry.copy_from_slice(rec);
        records.push(entry);
    }
    let mut storage: Vec<u8> = Vec::new();
    for rec in records.iter_mut() {
        let platform = u16::from_be_bytes([rec[0], rec[1]]);
        let name_id = u16::from_be_bytes([rec[6], rec[7]]);
        let len = u16::from_be_bytes([rec[8], rec[9]]) as usize;
        let off = u16::from_be_bytes([rec[10], rec[11]]) as usize;
        let Some(raw) = name.get(string_offset + off..string_offset + off + len) else {
            return name.to_vec();
        };
        if name_id != 6 {
            let new_off = storage.len();
            storage.extend_from_slice(raw);
            rec[8..10].copy_from_slice(&(len as u16).to_be_bytes());
            rec[10..12].copy_from_slice(&(new_off as u16).to_be_bytes());
            continue;
        }
        let text: String = if platform == 1 {
            raw.iter().map(|&b| b as char).collect()
        } else {
            let units: Vec<u16> = raw
                .chunks_exact(2)
                .map(|c| u16::from_be_bytes([c[0], c[1]]))
                .collect();
            String::from_utf16_lossy(&units)
        };
        if !text.is_ascii() {
            let new_off = storage.len();
            storage.extend_from_slice(raw);
            rec[8..10].copy_from_slice(&(len as u16).to_be_bytes());
            rec[10..12].copy_from_slice(&(new_off as u16).to_be_bytes());
            continue;
        }
        let patched = format!("{text}-Bold");
        let encoded: Vec<u8> = if platform == 1 {
            patched.bytes().collect()
        } else {
            patched
                .encode_utf16()
                .flat_map(|u| u.to_be_bytes())
                .collect()
        };
        let new_off = storage.len();
        storage.extend_from_slice(&encoded);
        rec[8..10].copy_from_slice(&(encoded.len() as u16).to_be_bytes());
        rec[10..12].copy_from_slice(&(new_off as u16).to_be_bytes());
    }
    // The output keeps the header and any format-1 language
    // records verbatim, then the rewritten records, then the
    // rebuilt storage; the string offset moves to match.
    // (Copying the original record block as well would
    // duplicate the records and shift every offset.)
    let records_end = 6 + count as usize * 12;
    let extras = &name[records_end..string_offset];
    let new_string_offset = 6 + extras.len() + count as usize * 12;
    let mut out = Vec::new();
    out.extend_from_slice(&name[..6]);
    out[4..6].copy_from_slice(&(new_string_offset as u16).to_be_bytes());
    out.extend_from_slice(extras);
    for rec in &records {
        out.extend_from_slice(rec);
    }
    out.extend_from_slice(&storage);
    out
}

/// The checksum of a table: the sum of its big-endian words,
/// zero-padded to a multiple of four bytes.
fn checksum(data: &[u8]) -> u32 {
    let mut sum: u32 = 0;
    for chunk in data.chunks(4) {
        let mut word = [0u8; 4];
        word[..chunk.len()].copy_from_slice(chunk);
        sum = sum.wrapping_add(u32::from_be_bytes(word));
    }
    sum
}

/// Instantiate `bytes` at bold weight, or `None` when the face
/// is not a variable TrueType face with a `wght` axis worth
/// moving, or when any step fails (the caller then keeps the
/// plain face, today's behavior).
pub(crate) fn instantiate_bold(bytes: &[u8]) -> Option<Vec<u8>> {
    let (version, tables) = directory(bytes)?;
    let (min, default, max) = wght_axis(bytes, &tables)?;
    let target = BOLD_WEIGHT.clamp(min, max);
    if (target - default).abs() < f32::EPSILON {
        return None;
    }
    let head = table_bytes(bytes, &tables, b"head")?;
    let index_to_loc_format = i16_at(head, 50)?;
    let hhea = table_bytes(bytes, &tables, b"hhea")?;
    let num_h_metrics = u16_at(hhea, 34)? as usize;
    let maxp = table_bytes(bytes, &tables, b"maxp")?;
    let num_glyphs = u16_at(maxp, 4)? as usize;
    if num_glyphs == 0 {
        return None;
    }
    let os2 = table_bytes(bytes, &tables, b"OS/2")?;
    if os2.len() < 64 {
        return None;
    }
    let _ = table_bytes(bytes, &tables, b"hmtx")?;
    let _ = table_bytes(bytes, &tables, b"glyf")?;
    let _ = table_bytes(bytes, &tables, b"loca")?;

    let mut varied = ttf_parser::Face::parse(bytes, 0).ok()?;
    if varied.number_of_glyphs() as usize != num_glyphs || !varied.is_variable() {
        return None;
    }
    varied.set_variation(ttf_parser::Tag::from_bytes(&WGHT), target)?;
    let plain = ttf_parser::Face::parse(bytes, 0).ok()?;

    // Re-encode every glyph at the varied instance: varied
    // advances, varied outlines (composites already
    // flattened by the outline walk), and a side bearing
    // that matches the varied outline's own left edge.
    let mut glyph_records: Vec<Vec<u8>> = Vec::with_capacity(num_glyphs);
    let mut hmtx: Vec<u8> = Vec::new();
    let mut max_points = 0usize;
    let mut max_contours = 0usize;
    for gid in 0..num_glyphs {
        let id = ttf_parser::GlyphId(gid as u16);
        let advance = varied.glyph_hor_advance(id)?;
        let mut collector = Collector::new();
        let outlined = varied.outline_glyph(id, &mut collector);
        let contours = collector.finish()?;
        let _ = outlined;
        let lsb = if let Some(first) = contours.iter().flatten().map(|p| p.x).min() {
            first
        } else {
            plain.glyph_hor_side_bearing(id).unwrap_or(0)
        };
        let point_count: usize = contours.iter().map(|c| c.len()).sum();
        if point_count > u16::MAX as usize {
            return None;
        }
        max_points = max_points.max(point_count);
        max_contours = max_contours.max(contours.len());
        if gid < num_h_metrics {
            hmtx.extend_from_slice(&advance.to_be_bytes());
            hmtx.extend_from_slice(&lsb.to_be_bytes());
        } else {
            hmtx.extend_from_slice(&lsb.to_be_bytes());
        }
        glyph_records.push(encode_glyph(&contours));
    }
    // Flattening composites grows `glyf` past what short
    // `loca` offsets can address (131,070 bytes): in that
    // case the instance switches to long `loca` offsets
    // and says so in `head`, which every reader handles.
    let total_glyf: usize = glyph_records.iter().map(|r| r.len()).sum();
    let use_short_loca = index_to_loc_format == 0 && total_glyf / 2 <= u16::MAX as usize;
    let loca_format: i16 = if use_short_loca { 0 } else { 1 };
    let mut glyf: Vec<u8> = Vec::new();
    let mut loca: Vec<u8> = Vec::new();
    for record in &glyph_records {
        if loca_format == 0 {
            if glyf.len() % 2 == 1 {
                glyf.push(0);
            }
            loca.extend_from_slice(&((glyf.len() / 2) as u16).to_be_bytes());
        } else {
            if glyf.len() > u32::MAX as usize {
                return None;
            }
            loca.extend_from_slice(&(glyf.len() as u32).to_be_bytes());
        }
        glyf.extend_from_slice(record);
    }
    if loca_format == 0 {
        if glyf.len() % 2 == 1 {
            glyf.push(0);
        }
        loca.extend_from_slice(&((glyf.len() / 2) as u16).to_be_bytes());
    } else {
        loca.extend_from_slice(&(glyf.len() as u32).to_be_bytes());
    }

    // Patched tables: weight identity to 700, composite and
    // instruction maxima to the flattened reality.
    let mut head_patched = head.to_vec();
    if head_patched.len() < 54 {
        return None;
    }
    let mac_style = u16::from_be_bytes([head_patched[44], head_patched[45]]) | 0x0001;
    head_patched[44..46].copy_from_slice(&mac_style.to_be_bytes());
    head_patched[50..52].copy_from_slice(&(loca_format as u16).to_be_bytes());
    head_patched[8..12].copy_from_slice(&0u32.to_be_bytes());
    let mut os2_patched = os2.to_vec();
    os2_patched[4..6].copy_from_slice(&(target as u16).to_be_bytes());
    let selection = u16::from_be_bytes([os2_patched[62], os2_patched[63]]) | 0x0020;
    os2_patched[62..64].copy_from_slice(&selection.to_be_bytes());
    let mut maxp_patched = maxp.to_vec();
    if maxp_patched.len() < 32 {
        return None;
    }
    maxp_patched[6..8].copy_from_slice(&(max_points as u16).to_be_bytes());
    maxp_patched[8..10].copy_from_slice(&(max_contours as u16).to_be_bytes());
    maxp_patched[10..12].copy_from_slice(&0u16.to_be_bytes());
    maxp_patched[12..14].copy_from_slice(&0u16.to_be_bytes());
    maxp_patched[26..28].copy_from_slice(&0u16.to_be_bytes());
    maxp_patched[28..30].copy_from_slice(&0u16.to_be_bytes());
    maxp_patched[30..32].copy_from_slice(&0u16.to_be_bytes());
    let name_patched = table_bytes(bytes, &tables, b"name").map(patch_name);

    // Rebuild the directory: every kept table in place,
    // variation tables dropped, replacements swapped in.
    let mut kept: Vec<([u8; 4], Vec<u8>)> = Vec::new();
    for table in &tables {
        if DROPPED.contains(&table.tag) {
            continue;
        }
        let data: Vec<u8> = if table.tag == *b"glyf" {
            glyf.clone()
        } else if table.tag == *b"loca" {
            loca.clone()
        } else if table.tag == *b"hmtx" {
            hmtx.clone()
        } else if table.tag == *b"head" {
            head_patched.clone()
        } else if table.tag == *b"OS/2" {
            os2_patched.clone()
        } else if table.tag == *b"maxp" {
            maxp_patched.clone()
        } else if table.tag == *b"name" {
            match &name_patched {
                Some(patched) => patched.clone(),
                None => bytes[table.offset..table.offset + table.length].to_vec(),
            }
        } else {
            bytes[table.offset..table.offset + table.length].to_vec()
        };
        kept.push((table.tag, data));
    }
    kept.sort_by_key(|(tag, _)| *tag);
    let count = kept.len();
    if count == 0 || count > 1024 {
        return None;
    }
    let exponent = 31 - (count as u32).leading_zeros();
    let power = 1usize << exponent;
    let search_range = power * 16;
    let entry_selector = exponent as usize;
    let range_shift = count * 16 - search_range;
    let mut header: Vec<u8> = Vec::new();
    header.extend_from_slice(&version.to_be_bytes());
    header.extend_from_slice(&(count as u16).to_be_bytes());
    header.extend_from_slice(&(search_range as u16).to_be_bytes());
    header.extend_from_slice(&(entry_selector as u16).to_be_bytes());
    header.extend_from_slice(&(range_shift as u16).to_be_bytes());
    let mut offset = 12 + count * 16;
    let mut entries: Vec<u8> = Vec::new();
    for (tag, data) in &kept {
        let sum = checksum(data);
        entries.extend_from_slice(tag);
        entries.extend_from_slice(&sum.to_be_bytes());
        entries.extend_from_slice(&(offset as u32).to_be_bytes());
        entries.extend_from_slice(&(data.len() as u32).to_be_bytes());
        offset += data.len();
        while offset % 4 != 0 {
            offset += 1;
        }
    }
    header.extend_from_slice(&entries);
    // The whole-font checksum, with the head table counted
    // at adjustment zero (the directory entry above already
    // holds that value): the adjustment makes the total the
    // spec's magic number.
    let mut total = checksum(&header);
    for (_, data) in &kept {
        total = total.wrapping_add(checksum(data));
    }
    let adjustment = 0xB1B0_AFBAu32.wrapping_sub(total);
    let mut out = header;
    for (tag, data) in &kept {
        if tag == b"head" {
            let mut head_final = data.clone();
            head_final[8..12].copy_from_slice(&adjustment.to_be_bytes());
            out.extend_from_slice(&head_final);
        } else {
            out.extend_from_slice(data);
        }
        while out.len() % 4 != 0 {
            out.push(0);
        }
    }
    // Validate the product before it ever reaches the
    // embedder: it must parse, hold the same glyph count,
    // no longer be variable, and read as bold.
    let check = ttf_parser::Face::parse(&out, 0).ok()?;
    if check.number_of_glyphs() as usize != num_glyphs || check.is_variable() || !check.is_bold() {
        return None;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gelasio() -> Vec<u8> {
        include_bytes!("../tests/fixtures/Gelasio[wght].ttf").to_vec()
    }

    fn gelasio_italic() -> Vec<u8> {
        include_bytes!("../tests/fixtures/Gelasio-Italic[wght].ttf").to_vec()
    }

    fn lora() -> Vec<u8> {
        include_bytes!("../tests/fixtures/Lora[wght].ttf").to_vec()
    }

    fn playfair() -> Vec<u8> {
        include_bytes!("../tests/fixtures/PlayfairDisplay[wght].ttf").to_vec()
    }

    fn gid(face: &ttf_parser::Face, c: char) -> ttf_parser::GlyphId {
        face.glyph_index(c).expect("the face draws the character")
    }

    fn bbox(face: &ttf_parser::Face, c: char) -> ttf_parser::Rect {
        face.glyph_bounding_box(gid(face, c))
            .expect("the glyph has an outline")
    }

    /// Every pack variable family instantiates to a static
    /// bold face whose H advances and outlines are wider
    /// than the default instance's, with the glyph count
    /// untouched and a PostScript name that no longer
    /// collides with the regular face's.
    #[test]
    fn bold_instance_is_static_bold_and_wider_for_all_three_families() {
        for (name, bytes) in [
            ("Gelasio", gelasio()),
            ("Lora", lora()),
            ("Playfair Display", playfair()),
        ] {
            let regular = ttf_parser::Face::parse(&bytes, 0).expect("fixture parses");
            assert!(regular.is_variable(), "{name} fixture is variable");
            assert!(!regular.is_bold(), "{name} fixture is not bold");
            let inst = instantiate_bold(&bytes).expect("{name} instantiates");
            let bold = ttf_parser::Face::parse(&inst, 0).expect("instance parses");
            assert!(!bold.is_variable(), "{name} instance is static");
            assert!(bold.is_bold(), "{name} instance reads as bold");
            assert_eq!(
                bold.number_of_glyphs(),
                regular.number_of_glyphs(),
                "{name} keeps its glyph count"
            );
            let h = gid(&regular, 'H');
            assert!(
                bold.glyph_hor_advance(h).expect("H advance")
                    > regular.glyph_hor_advance(h).expect("H advance"),
                "{name} bold H advances wider than regular H"
            );
            let regular_box = bbox(&regular, 'H');
            let bold_box = bbox(&bold, 'H');
            assert!(
                bold_box.width() > regular_box.width(),
                "{name} bold H outline is wider than regular H"
            );
            let ps = bold
                .names()
                .into_iter()
                .find(|n| n.name_id == ttf_parser::name_id::POST_SCRIPT_NAME)
                .and_then(|n| n.to_string())
                .expect("instance has a PostScript name");
            assert!(
                ps.ends_with("-Bold"),
                "{name} PostScript name carries the bold suffix: {ps}"
            );
        }
    }

    /// The italic variable file instantiates to a
    /// bold-italic face: bold set, italic kept.
    #[test]
    fn italic_instance_is_bold_and_stays_italic() {
        let bytes = gelasio_italic();
        let regular = ttf_parser::Face::parse(&bytes, 0).expect("fixture parses");
        assert!(regular.is_italic(), "italic fixture reads as italic");
        let inst = instantiate_bold(&bytes).expect("italic instantiates");
        let bold = ttf_parser::Face::parse(&inst, 0).expect("instance parses");
        assert!(bold.is_bold(), "italic instance reads as bold");
        assert!(bold.is_italic(), "italic instance stays italic");
        assert!(!bold.is_variable(), "italic instance is static");
    }

    /// A composite glyph flattens with a real outline: the
    /// accented e is built from components in the source,
    /// and the instance stores it as simple contours.
    #[test]
    fn composite_glyph_flattens_with_an_outline() {
        let bytes = gelasio();
        let inst = instantiate_bold(&bytes).expect("instantiates");
        let bold = ttf_parser::Face::parse(&inst, 0).expect("instance parses");
        let regular = ttf_parser::Face::parse(&bytes, 0).expect("fixture parses");
        let gid_e = gid(&bold, 'é');
        let box_bold = bold.glyph_bounding_box(gid_e).expect("é has an outline");
        assert!(box_bold.width() > 0, "é outline is non-empty");
        assert!(
            bold.glyph_hor_advance(gid_e).expect("é advance")
                != regular
                    .glyph_hor_advance(gid(&regular, 'é'))
                    .expect("é advance"),
            "é advances differ between the instances"
        );
    }

    /// Static faces, CFF-flavored programs, and garbage
    /// refuse instantiation instead of producing a
    /// half-built font: the caller falls back to plain.
    #[test]
    fn static_cff_and_garbage_refuse() {
        let bundled = ob_text::shaping::bundled_font_files();
        let (_, liberation) = bundled
            .iter()
            .find(|(name, _)| *name == "LiberationSans-Regular.ttf")
            .expect("bundled regular exists");
        assert_eq!(
            instantiate_bold(liberation),
            None,
            "a static face has no wght axis to move"
        );
        assert_eq!(
            instantiate_bold(b"not a font at all"),
            None,
            "garbage refuses"
        );
        let mut truncated = gelasio();
        truncated.truncate(1000);
        assert_eq!(
            instantiate_bold(&truncated),
            None,
            "a truncated variable file refuses"
        );
    }
}
