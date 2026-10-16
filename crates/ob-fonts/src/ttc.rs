//! Splitting a TrueType Collection (.ttc) into standalone
//! font files (docs/FONTS.md, font file formats).
//!
//! A collection is one file holding several faces that
//! share tables: many system fonts ship this way (the
//! CJK families on Windows and macOS, Cambria's math
//! companion, and others). The library keeps one file per
//! face, so intake splits a collection apart: each face's
//! own table directory and table data are copied into a
//! fresh sfnt file for that face alone, checksums carried
//! over and the whole-font checksum adjustment recomputed.
//! Nothing is re-encoded or re-hinted; the face's tables
//! move byte for byte.
//!
//! The split is validated before it is believed: every
//! output must parse as a font, and any structural
//! surprise (offsets outside the file, absurd counts)
//! refuses the whole collection in plain words rather
//! than guessing.

/// True when these bytes carry the collection tag.
pub fn is_collection(bytes: &[u8]) -> bool {
    bytes.len() >= 4 && &bytes[0..4] == b"ttcf"
}

fn be_u16(bytes: &[u8], at: usize) -> Option<u16> {
    bytes
        .get(at..at + 2)
        .map(|s| u16::from_be_bytes([s[0], s[1]]))
}

fn be_u32(bytes: &[u8], at: usize) -> Option<u32> {
    bytes
        .get(at..at + 4)
        .map(|s| u32::from_be_bytes([s[0], s[1], s[2], s[3]]))
}

/// Split a collection into one standalone font file per
/// face, in the collection's own face order.
pub fn split_collection(bytes: &[u8]) -> Result<Vec<Vec<u8>>, String> {
    if !is_collection(bytes) {
        return Err("that file is not a font collection".into());
    }
    let count = be_u32(bytes, 8).ok_or("the collection header is cut short")? as usize;
    if count == 0 || count > 256 {
        return Err(format!(
            "that collection claims {count} fonts; real collections hold a handful"
        ));
    }
    let mut faces = Vec::new();
    for i in 0..count {
        let base =
            be_u32(bytes, 12 + i * 4).ok_or("the collection's face list is cut short")? as usize;
        faces.push(split_face(bytes, base)?);
    }
    Ok(faces)
}

/// Copy one face (its offset table sits at `base` inside
/// the collection) into a standalone sfnt file.
fn split_face(bytes: &[u8], base: usize) -> Result<Vec<u8>, String> {
    let version = be_u32(bytes, base).ok_or("a face's header is cut short")?;
    let num_tables = be_u16(bytes, base + 4).ok_or("a face's header is cut short")? as usize;
    if num_tables == 0 || num_tables > 128 {
        return Err(format!(
            "a face in that collection claims {num_tables} parts; that is not a font"
        ));
    }
    // (tag, checksum, data) per table, DSIG dropped: a
    // digital signature covers the collection's bytes and
    // can never be true of a split-out face.
    let mut tables: Vec<([u8; 4], u32, Vec<u8>)> = Vec::new();
    for t in 0..num_tables {
        let rec = base + 12 + t * 16;
        let tag: [u8; 4] = bytes
            .get(rec..rec + 4)
            .and_then(|s| s.try_into().ok())
            .ok_or("a face's table list is cut short")?;
        let checksum = be_u32(bytes, rec + 4).ok_or("a face's table list is cut short")?;
        let off = be_u32(bytes, rec + 8).ok_or("a face's table list is cut short")? as usize;
        let len = be_u32(bytes, rec + 12).ok_or("a face's table list is cut short")? as usize;
        if len > 64 * 1024 * 1024 {
            return Err("a face in that collection claims a part larger than 64 MB".into());
        }
        let data = bytes
            .get(off..off + len)
            .ok_or("a face's table points outside the collection")?
            .to_vec();
        if &tag == b"DSIG" {
            continue;
        }
        tables.push((tag, checksum, data));
    }
    tables.sort_by_key(|a| a.0);
    let n = tables.len();
    // The sfnt search aids, by the book: searchRange is
    // the largest power of two of table slots times 16.
    let mut pow = 1usize;
    while pow * 2 <= n {
        pow *= 2;
    }
    let search_range = (pow * 16) as u16;
    let entry_selector = pow.trailing_zeros() as u16;
    let range_shift = (n * 16 - pow * 16) as u16;
    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(&version.to_be_bytes());
    out.extend_from_slice(&(n as u16).to_be_bytes());
    out.extend_from_slice(&search_range.to_be_bytes());
    out.extend_from_slice(&entry_selector.to_be_bytes());
    out.extend_from_slice(&range_shift.to_be_bytes());
    let dir_at = out.len();
    out.resize(dir_at + n * 16, 0);
    let mut head_at: Option<usize> = None;
    for (i, (tag, checksum, data)) in tables.iter().enumerate() {
        while out.len() % 4 != 0 {
            out.push(0);
        }
        let rec = dir_at + i * 16;
        let data_at = out.len() as u32;
        out[rec..rec + 4].copy_from_slice(tag);
        out[rec + 4..rec + 8].copy_from_slice(&checksum.to_be_bytes());
        out[rec + 8..rec + 12].copy_from_slice(&data_at.to_be_bytes());
        out[rec + 12..rec + 16].copy_from_slice(&(data.len() as u32).to_be_bytes());
        if tag == b"head" {
            head_at = Some(out.len());
        }
        out.extend_from_slice(data);
    }
    while out.len() % 4 != 0 {
        out.push(0);
    }
    // The whole-font checksum: zero the adjustment field,
    // sum the file as big-endian words, and write the
    // adjustment that brings the total to 0xB1B0AFBA.
    if let Some(head) = head_at {
        if head + 12 <= out.len() {
            out[head + 8..head + 12].copy_from_slice(&0u32.to_be_bytes());
            let sum = out.chunks(4).fold(0u32, |acc, c| {
                let mut word = [0u8; 4];
                word[..c.len()].copy_from_slice(c);
                acc.wrapping_add(u32::from_be_bytes(word))
            });
            let adjustment = 0xB1B0AFBAu32.wrapping_sub(sum);
            out[head + 8..head + 12].copy_from_slice(&adjustment.to_be_bytes());
        }
    }
    // Believed only when it parses: a split that does not
    // read back as a font is refused, never saved.
    if ttf_parser::Face::parse(&out, 0).is_err() {
        return Err("a face from that collection would not read back as a font".into());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bundled(name: &str) -> Vec<u8> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../crates/ob-text/assets/fonts")
            .join(name);
        std::fs::read(&path).expect("bundled font present")
    }

    /// Lay two standalone fonts side by side behind a
    /// collection header, each face's table offsets
    /// shifted by where its image landed: the shape a
    /// real .ttc carries, built from fonts we may ship.
    fn make_ttc(faces: &[&[u8]]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"ttcf");
        out.extend_from_slice(&1u16.to_be_bytes());
        out.extend_from_slice(&0u16.to_be_bytes());
        out.extend_from_slice(&(faces.len() as u32).to_be_bytes());
        let header = 12 + faces.len() * 4;
        let mut bases = Vec::new();
        let mut at = header;
        for f in faces {
            bases.push(at);
            at += f.len();
            while at % 4 != 0 {
                at += 1;
            }
        }
        for b in &bases {
            out.extend_from_slice(&(*b as u32).to_be_bytes());
        }
        for (face, base) in faces.iter().zip(&bases) {
            while out.len() < *base {
                out.push(0);
            }
            let start = out.len();
            out.extend_from_slice(face);
            let num_tables = be_u16(face, 4).expect("tables") as usize;
            for t in 0..num_tables {
                let rec = start + 12 + t * 16;
                let off = be_u32(face, 12 + t * 16 + 8).expect("offset") as usize;
                out[rec + 8..rec + 12].copy_from_slice(&((start + off) as u32).to_be_bytes());
            }
        }
        out
    }

    #[test]
    fn a_collection_splits_into_its_faces() {
        let sans = bundled("LiberationSans-Regular.ttf");
        let serif = bundled("LiberationSerif-Regular.ttf");
        let ttc = make_ttc(&[&sans, &serif]);
        assert!(is_collection(&ttc));
        assert_eq!(ttf_parser::fonts_in_collection(&ttc), Some(2));
        let faces = split_collection(&ttc).expect("split");
        assert_eq!(faces.len(), 2);
        let names: Vec<String> = faces
            .iter()
            .map(|f| crate::inspect(f, "face.ttf").expect("inspect").family)
            .collect();
        assert_eq!(names, ["Liberation Sans", "Liberation Serif"]);
        // The faces carry their own glyphs: an advance
        // straight from a split face equals the original's.
        for (split, orig) in faces.iter().zip([&sans, &serif]) {
            let a = ttf_parser::Face::parse(split, 0).expect("split parses");
            let b = ttf_parser::Face::parse(orig, 0).expect("original parses");
            let ga = a.glyph_index('M').expect("glyph");
            let gb = b.glyph_index('M').expect("glyph");
            assert_eq!(a.glyph_hor_advance(ga), b.glyph_hor_advance(gb));
        }
    }

    #[test]
    fn broken_collections_are_refused_in_words() {
        assert!(split_collection(b"not a font at all").is_err());
        let mut ttc = make_ttc(&[&bundled("LiberationSans-Regular.ttf")]);
        // A face offset pointing outside the file.
        ttc[12..16].copy_from_slice(&9_000_000u32.to_be_bytes());
        assert!(split_collection(&ttc).is_err());
    }
}
