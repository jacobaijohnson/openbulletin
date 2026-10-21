//! The one derivation of a zip archive's central
//! directory location, shared by every zip door in the
//! workspace (round 33, F3).
//!
//! A shifted archive (junk prepended before the zip,
//! the self-extracting shape) declares every offset
//! relative to its own start, so the reader and the
//! duplicate scans must add the same archive offset to
//! those declarations before they mean anything in the
//! file's own coordinates. The derivation used to be
//! stated per door, as a forward hunt for the first
//! `PK\x01\x02` at-or-after the declared offset — the
//! hunt the zip reader itself runs — so reader and
//! scans agreed even in failure: a well-formed shifted
//! archive whose last entry is Stored and whose payload
//! carries those four signature bytes inside the window
//! between the declared offset and the real directory
//! poisoned the hunt at its first step, the reader
//! failed with "Could not find EOCD", and the IDML,
//! font-pack, and clip doors refused the whole package.
//!
//! The directory's true location is arithmetic, not a
//! hunt. The end record stands at a position the file
//! itself shows; the directory ends where the end
//! record (or, for zip64, the zip64 record the locator
//! names) begins; and the end record declares the
//! directory's size and offset. So the archive offset
//! is what is left of the end record's position once
//! the declared size and offset are taken away, the
//! directory starts at its declared offset plus that
//! archive offset, and a candidate derivation is only
//! believed once the records it names walk, entry by
//! entry, to exactly the end the arithmetic promised.
//! An archive whose directory is not arithmetically
//! derivable that way (a record standing between the
//! directory and the end record, a zip64 record with an
//! extensible data sector) falls back to the hunt this
//! derivation replaced — stated once, here, so the
//! fallback is the same for every door too. The hunt
//! validates what it finds (round 34, F1 + F2): a
//! hunted zip64 record must validate against the
//! locator that follows it, a hunted directory must
//! walk to the promised end the way an arithmetic
//! candidate must, the search keeps going past a
//! candidate that does not validate, and every
//! position it computes is checked arithmetic — the
//! unvalidated hunt took planted signature bytes in a
//! shifted archive's windows for the directory, or for
//! no directory, and panicked on crafted offsets.
//!
//! This lives in ob-model, the lowest crate every zip
//! door can reach, stated over raw bytes the way
//! `read_capped` is stated over `Read`, so this crate
//! needs no zip dependency to hold it.

use std::borrow::Cow;

/// Where one archive's central directory truly stands,
/// in the file's own coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CentralDirectory {
    /// How far the archive's own start stands into the
    /// file: every offset the archive declares is
    /// relative to that start, so a record's local
    /// offset means this much more in the file.
    pub archive_offset: u64,
    /// The file position of the directory's first
    /// record: its declared offset plus the archive
    /// offset.
    pub cd_start: usize,
    /// How many entries the directory holds, through
    /// the zip64 record where the plain end record's
    /// fields are sentinels.
    pub total: u64,
}

fn u16_at(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([bytes[at], bytes[at + 1]])
}

fn u32_at(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap_or([0; 4]))
}

fn u64_at(bytes: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(bytes[at..at + 8].try_into().unwrap_or([0; 8]))
}

/// The latest end record in the file: the last
/// `PK\x05\x06` whose stated comment length fits in
/// what follows it, the candidate every door's scan
/// used to pick for itself.
fn end_record_at(bytes: &[u8]) -> Option<usize> {
    if bytes.len() < 22 {
        return None;
    }
    for at in (0..=bytes.len() - 22).rev() {
        if !bytes[at..].starts_with(b"PK\x05\x06") {
            continue;
        }
        let comment = usize::from(u16_at(bytes, at + 20));
        if at + 22 + comment <= bytes.len() {
            return Some(at);
        }
    }
    None
}

/// Where the walk of `total` central records starting
/// at `cd_start`, in the shape the directory scans
/// themselves walk, lands: the position just past the
/// last record. None when any record does not stand
/// where the walk puts it.
fn directory_walk_end(bytes: &[u8], cd_start: usize, total: u64) -> Option<usize> {
    let mut at = cd_start;
    for _ in 0..total {
        let header_end = at.checked_add(46)?;
        if header_end > bytes.len() || !bytes[at..].starts_with(b"PK\x01\x02") {
            return None;
        }
        let name_len = usize::from(u16_at(bytes, at + 28));
        let extra_len = usize::from(u16_at(bytes, at + 30));
        let comment_len = usize::from(u16_at(bytes, at + 32));
        let name_end = header_end.checked_add(name_len)?;
        let extra_end = name_end.checked_add(extra_len)?;
        if extra_end > bytes.len() {
            return None;
        }
        at = extra_end.checked_add(comment_len)?;
    }
    Some(at)
}

/// Whether the central records starting at `cd_start`
/// walk, `total` of them, to exactly `end` and no
/// further: the proof an arithmetic candidate is the
/// real directory and not signature bytes standing in
/// an entry's payload.
fn directory_walks_to(bytes: &[u8], cd_start: usize, total: u64, end: usize) -> bool {
    directory_walk_end(bytes, cd_start, total) == Some(end)
}

/// Whether a hunted candidate directory is the real
/// one, by the same walk to the promised end the
/// arithmetic path demands (`directory_walks_to`),
/// with the one terminator the hunt exists to stand
/// past: a central-directory digital-signature record
/// (PK\x05\x05) counted in the declared directory
/// size but not in the entry count, spanning exactly
/// from the walk's end to the promised end.
fn hunted_directory_walks_to(
    bytes: &[u8],
    cd_start: usize,
    total: u64,
    promised_end: usize,
) -> bool {
    let Some(at) = directory_walk_end(bytes, cd_start, total) else {
        return false;
    };
    if at == promised_end {
        return true;
    }
    if at > bytes.len() || !bytes[at..].starts_with(b"PK\x05\x05") {
        return false;
    }
    let Some(header_end) = at.checked_add(6) else {
        return false;
    };
    if header_end > bytes.len() {
        return false;
    }
    let data_len = usize::from(u16_at(bytes, at + 4));
    header_end.checked_add(data_len) == Some(promised_end)
}

/// The directory derived arithmetically (see the
/// module's note): from the end record's own position
/// and declared sizes, through the zip64 record where
/// the plain fields are sentinels. The zip64 record's
/// own position is arithmetic too: it stands
/// immediately before its locator, which stands
/// immediately before the end record, and a record
/// with no extensible data sector is 56 bytes — its
/// size field says so, and the derivation checks that
/// it does. None when the arithmetic does not close
/// or the records do not walk to the promised end.
fn arithmetic_directory(bytes: &[u8], eocd: usize) -> Option<CentralDirectory> {
    let total_plain = u64::from(u16_at(bytes, eocd + 10));
    let cd_size_plain = u64::from(u32_at(bytes, eocd + 12));
    let cd_offset_plain = u64::from(u32_at(bytes, eocd + 16));
    let zip64_needed = total_plain == u64::from(u16::MAX)
        || cd_size_plain == u64::from(u32::MAX)
        || cd_offset_plain == u64::from(u32::MAX);
    if zip64_needed {
        let locator_at = eocd.checked_sub(20)?;
        if !bytes[locator_at..].starts_with(b"PK\x06\x07") {
            return None;
        }
        let declared_record = u64_at(bytes, locator_at + 8);
        let record_at = locator_at.checked_sub(56)?;
        if !bytes[record_at..].starts_with(b"PK\x06\x06") {
            return None;
        }
        // No extensible data sector: the record is
        // exactly its fixed 56 bytes, so the size it
        // declares for itself (everything past its
        // signature and size fields) is 44.
        if u64_at(bytes, record_at + 4) != 44 {
            return None;
        }
        let total = u64_at(bytes, record_at + 32);
        let cd_size = u64_at(bytes, record_at + 40);
        let cd_offset = u64_at(bytes, record_at + 48);
        // The directory ends where the zip64 record
        // stands, in the archive's own coordinates.
        if cd_offset.checked_add(cd_size)? != declared_record {
            return None;
        }
        let archive_offset = u64::try_from(record_at)
            .ok()?
            .checked_sub(declared_record)?;
        let cd_start = usize::try_from(cd_offset.checked_add(archive_offset)?).ok()?;
        if !directory_walks_to(bytes, cd_start, total, record_at) {
            return None;
        }
        return Some(CentralDirectory {
            archive_offset,
            cd_start,
            total,
        });
    }
    let total = total_plain;
    let archive_offset = u64::try_from(eocd)
        .ok()?
        .checked_sub(cd_size_plain)?
        .checked_sub(cd_offset_plain)?;
    let cd_start = usize::try_from(cd_offset_plain.checked_add(archive_offset)?).ok()?;
    if !directory_walks_to(bytes, cd_start, total, eocd) {
        return None;
    }
    Some(CentralDirectory {
        archive_offset,
        cd_start,
        total,
    })
}

/// A zip64 record candidate at `record_at`, validated
/// against the locator that must follow it (round 34,
/// F2): the record's own declared length — its size
/// field plus the twelve bytes of signature and size —
/// must land exactly on the locator at `locator_at`,
/// its directory fields must close over the locator's
/// declared record offset (the directory's offset and
/// size sum to it, in the archive's own coordinates),
/// and the position it stands at must agree with that
/// declaration. Returns the record's (total, directory
/// size, directory offset, archive offset) when it
/// validates; a signature planted in a record's name
/// or an entry's payload validates on none of these.
fn validated_zip64_record(
    bytes: &[u8],
    record_at: usize,
    locator_at: usize,
    declared_record: u64,
) -> Option<(u64, u64, u64, u64)> {
    let fixed_end = record_at.checked_add(56)?;
    if fixed_end > bytes.len() {
        return None;
    }
    let size_field = u64_at(bytes, record_at + 4);
    if size_field < 44 {
        return None;
    }
    let record_len = usize::try_from(size_field.checked_add(12)?).ok()?;
    if record_at.checked_add(record_len)? != locator_at {
        return None;
    }
    let archive_offset = u64::try_from(record_at)
        .ok()?
        .checked_sub(declared_record)?;
    let total = u64_at(bytes, record_at + 32);
    let cd_size = u64_at(bytes, record_at + 40);
    let cd_offset = u64_at(bytes, record_at + 48);
    if cd_offset.checked_add(cd_size)? != declared_record {
        return None;
    }
    Some((total, cd_size, cd_offset, archive_offset))
}

/// One central-hunt candidate at `at`, validated the
/// arithmetic path's way (round 34, F2): the archive
/// offset the candidate implies must be the one the
/// validated zip64 record implied, where there is one,
/// and the records at `at` must walk, `total` of them,
/// to the promised end — the declared directory size
/// taken from the candidate's own start.
fn validated_central_candidate(
    bytes: &[u8],
    at: usize,
    total: u64,
    cd_size: u64,
    cd_offset: u64,
    record_archive_offset: Option<u64>,
) -> Option<CentralDirectory> {
    let archive_offset = u64::try_from(at).ok()?.checked_sub(cd_offset)?;
    if record_archive_offset.is_some_and(|expected| expected != archive_offset) {
        return None;
    }
    let promised_end = at.checked_add(usize::try_from(cd_size).ok()?)?;
    if !hunted_directory_walks_to(bytes, at, total, promised_end) {
        return None;
    }
    Some(CentralDirectory {
        archive_offset,
        cd_start: at,
        total,
    })
}

/// The derivation the doors used to state per door
/// (round 32, F4), kept as the one fallback for an
/// archive whose directory is not arithmetically
/// derivable: the zip64 record found by searching
/// forward from its declared offset to its locator,
/// and the directory's first record found by searching
/// forward from the declared offset to the end record.
///
/// Round 34, F1 + F2: the hunt validates what it finds
/// and trusts no field as a position unchecked. Every
/// position computation is checked arithmetic, so a
/// crafted offset near u64::MAX is a candidate that
/// does not validate, never an overflow in debug, a
/// wrap in release, or a slice out of range. A hunted
/// zip64 record is believed only when it validates
/// against the locator that follows it
/// (`validated_zip64_record`); a hunted directory
/// only when its records walk to the promised end
/// (`validated_central_candidate`); and the hunt keeps
/// searching past a candidate that does not validate
/// instead of taking the first signature as final —
/// on a shifted archive the search window crosses the
/// last entry's stored tail and the real directory's
/// own tail, where signature bytes planted in a
/// payload or a record's name used to become, or
/// erase, the directory the doors judged. Where no
/// candidate validates the hunt answers None, as it
/// does for underivable bytes; the doors refuse a
/// reader-openable archive in that state rather than
/// scan the empty directory this used to return.
fn hunted_directory(bytes: &[u8], eocd: usize) -> Option<CentralDirectory> {
    let mut total = u64::from(u16_at(bytes, eocd + 10));
    let mut cd_size = u64::from(u32_at(bytes, eocd + 12));
    let mut cd_offset = u64::from(u32_at(bytes, eocd + 16));
    let mut record_archive_offset: Option<u64> = None;
    let zip64_needed = total == u64::from(u16::MAX)
        || cd_size == u64::from(u32::MAX)
        || cd_offset == u64::from(u32::MAX);
    if zip64_needed {
        // The plain fields are sentinels: the true
        // fields ride in the zip64 record, and without
        // a locator standing where one must stand there
        // is nothing to validate a record against.
        let locator_at = eocd.checked_sub(20)?;
        if !bytes[locator_at..].starts_with(b"PK\x06\x07") {
            return None;
        }
        let declared_record = u64_at(bytes, locator_at + 8);
        let mut record_at = usize::try_from(declared_record).unwrap_or(usize::MAX);
        let mut validated = None;
        while let Some(window_end) = record_at.checked_add(4) {
            if window_end > locator_at {
                break;
            }
            if bytes[record_at..].starts_with(b"PK\x06\x06") {
                if let Some(fields) =
                    validated_zip64_record(bytes, record_at, locator_at, declared_record)
                {
                    validated = Some(fields);
                    break;
                }
            }
            let Some(next) = record_at.checked_add(1) else {
                break;
            };
            record_at = next;
        }
        let (rec_total, rec_size, rec_offset, rec_archive_offset) = validated?;
        total = rec_total;
        cd_size = rec_size;
        cd_offset = rec_offset;
        record_archive_offset = Some(rec_archive_offset);
    }
    if total == 0 {
        return None;
    }
    let mut at = usize::try_from(cd_offset).unwrap_or(usize::MAX);
    while let Some(window_end) = at.checked_add(4) {
        if window_end > eocd {
            break;
        }
        if bytes[at..].starts_with(b"PK\x01\x02") {
            if let Some(directory) = validated_central_candidate(
                bytes,
                at,
                total,
                cd_size,
                cd_offset,
                record_archive_offset,
            ) {
                return Some(directory);
            }
        }
        let Some(next) = at.checked_add(1) else {
            break;
        };
        at = next;
    }
    None
}

/// The central directory of the archive in `bytes`:
/// where its first record stands, how many entries it
/// holds, and the archive offset every declared offset
/// in it must be shifted by. Arithmetic first (see the
/// module's note), the hunted derivation as fallback.
/// None when no end record stands or no directory can
/// be derived either way — the archive reader reports
/// that damage in its own words.
pub fn central_directory(bytes: &[u8]) -> Option<CentralDirectory> {
    let eocd = end_record_at(bytes)?;
    arithmetic_directory(bytes, eocd).or_else(|| hunted_directory(bytes, eocd))
}

/// The prefix-free view of an archive: the same bytes
/// with whatever junk precedes the archive itself cut
/// away, so the archive's declared offsets are its
/// positions in the view and neither the reader nor a
/// scan ever traverses the window the junk opened
/// between a declared offset and the real directory
/// (round 33, F3). A door hands this one view to both
/// its reader and its scans. Borrowed, unchanged, when
/// no junk precedes the archive or no directory can be
/// derived.
pub fn prefix_free(bytes: &[u8]) -> Cow<'_, [u8]> {
    let Some(directory) = central_directory(bytes) else {
        return Cow::Borrowed(bytes);
    };
    let Ok(start) = usize::try_from(directory.archive_offset) else {
        return Cow::Borrowed(bytes);
    };
    if start == 0 || start > bytes.len() {
        return Cow::Borrowed(bytes);
    }
    Cow::Owned(bytes[start..].to_vec())
}

/// Where one central record's entry payload stands
/// and how it is packed: its method, its compressed
/// size, the checksum its record declares, and its
/// local offset, the sizes and offset resolved
/// through the zip64 extra field wherever a plain
/// field carries the sentinel. One spot type for
/// every door (round 34, F8): the doors that read no
/// checksum simply never look at that field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CentralSpot {
    pub method: u16,
    pub comp_size: u64,
    pub crc32: u32,
    pub local_offset: u64,
}

/// Read one central record's spot out of its fixed
/// header and extra field (the zip64 values ride in
/// the order uncompressed, compressed, local offset,
/// disk, one per sentinel field).
fn raw_spot(header: &[u8], extra: &[u8]) -> CentralSpot {
    let field = |at: usize| u32::from_le_bytes(header[at..at + 4].try_into().unwrap_or([0; 4]));
    let comp32 = field(20);
    let uncomp32 = field(24);
    let local32 = field(42);
    let disk16 = u16::from_le_bytes([header[34], header[35]]);
    let mut comp_size = u64::from(comp32);
    let mut local_offset = u64::from(local32);
    if comp32 == u32::MAX || uncomp32 == u32::MAX || local32 == u32::MAX || disk16 == u16::MAX {
        let mut at = 0usize;
        while at + 4 <= extra.len() {
            let id = u16::from_le_bytes([extra[at], extra[at + 1]]);
            let size = usize::from(u16::from_le_bytes([extra[at + 2], extra[at + 3]]));
            let data = &extra[at + 4..(at + 4 + size).min(extra.len())];
            if id == 0x0001 {
                let mut pos = 0usize;
                if uncomp32 == u32::MAX {
                    pos += 8;
                }
                if comp32 == u32::MAX && pos + 8 <= data.len() {
                    comp_size = u64::from_le_bytes(data[pos..pos + 8].try_into().unwrap_or([0; 8]));
                    pos += 8;
                }
                if local32 == u32::MAX && pos + 8 <= data.len() {
                    local_offset =
                        u64::from_le_bytes(data[pos..pos + 8].try_into().unwrap_or([0; 8]));
                }
                break;
            }
            at += 4 + size;
        }
    }
    CentralSpot {
        method: u16::from_le_bytes([header[10], header[11]]),
        comp_size,
        crc32: field(16),
        local_offset,
    }
}

/// The CP437 characters for bytes 0x80..=0xFF, in byte
/// order: the zip reader's own table, copied so a name
/// the reader decodes as CP437 is keyed here by the
/// characters the reader resolves it to (round 32, F4).
const CP437_HIGH: &str = "\
    \u{00c7}\u{00fc}\u{00e9}\u{00e2}\u{00e4}\u{00e0}\u{00e5}\u{00e7}\
    \u{00ea}\u{00eb}\u{00e8}\u{00ef}\u{00ee}\u{00ec}\u{00c4}\u{00c5}\
    \u{00c9}\u{00e6}\u{00c6}\u{00f4}\u{00f6}\u{00f2}\u{00fb}\u{00f9}\
    \u{00ff}\u{00d6}\u{00dc}\u{00a2}\u{00a3}\u{00a5}\u{20a7}\u{0192}\
    \u{00e1}\u{00ed}\u{00f3}\u{00fa}\u{00f1}\u{00d1}\u{00aa}\u{00ba}\
    \u{00bf}\u{2310}\u{00ac}\u{00bd}\u{00bc}\u{00a1}\u{00ab}\u{00bb}\
    \u{2591}\u{2592}\u{2593}\u{2502}\u{2524}\u{2561}\u{2562}\u{2556}\
    \u{2555}\u{2563}\u{2551}\u{2557}\u{255d}\u{255c}\u{255b}\u{2510}\
    \u{2514}\u{2534}\u{252c}\u{251c}\u{2500}\u{253c}\u{255e}\u{255f}\
    \u{255a}\u{2554}\u{2569}\u{2566}\u{2560}\u{2550}\u{256c}\u{2567}\
    \u{2568}\u{2564}\u{2565}\u{2559}\u{2558}\u{2552}\u{2553}\u{256b}\
    \u{256a}\u{2518}\u{250c}\u{2588}\u{2584}\u{258c}\u{2590}\u{2580}\
    \u{03b1}\u{00df}\u{0393}\u{03c0}\u{03a3}\u{03c3}\u{00b5}\u{03c4}\
    \u{03a6}\u{0398}\u{03a9}\u{03b4}\u{221e}\u{03c6}\u{03b5}\u{2229}\
    \u{2261}\u{00b1}\u{2265}\u{2264}\u{2320}\u{2321}\u{00f7}\u{2248}\
    \u{00b0}\u{2219}\u{00b7}\u{221a}\u{207f}\u{00b2}\u{25a0}\u{00a0}";

/// Key one raw central-directory name the way the zip
/// reader resolves it (round 32, F4): with the entry's
/// UTF-8 flag set, the name is UTF-8 text; with it
/// clear, the reader decodes the bytes as CP437. The
/// duplicate scans keyed names by lossy UTF-8 at
/// first, so a CP437-named pair keyed to a name the
/// reader never serves and the checks passed over it.
fn decode_entry_name(raw: &[u8], flags: u16) -> String {
    if flags & 0x0800 != 0 {
        return String::from_utf8_lossy(raw).into_owned();
    }
    raw.iter()
        .map(|&byte| {
            if byte < 0x80 {
                byte as char
            } else {
                CP437_HIGH
                    .chars()
                    .nth(usize::from(byte - 0x80))
                    .unwrap_or('\u{fffd}')
            }
        })
        .collect()
}

/// The (name, spot) pairs of an archive's central
/// directory, walked off the raw records, in archive
/// order and WITH duplicates standing: the zip
/// reader's own map is keyed by name, so a duplicated
/// name reaches it as only its last copy, and the
/// copies can only be counted here, before that map
/// is trusted (rounds 31/32, F5/F6). The directory is
/// the one `central_directory` derives — the latest
/// end record's, through its zip64 record where the
/// plain fields are sentinels — and the archive offset
/// it derives applies to the local offsets in the
/// records. Empty when no directory can be derived;
/// each door's own words cover that case. One walk
/// for every door (round 34, F8): the four doors'
/// copies were byte-identical, and each door keeps
/// only its payload read, capped by its own cap.
pub fn central_spots(bytes: &[u8]) -> Vec<(String, CentralSpot)> {
    let Some(directory) = central_directory(bytes) else {
        return Vec::new();
    };
    if directory.total == 0 {
        return Vec::new();
    }
    let total = directory.total;
    let archive_offset = directory.archive_offset;
    let mut out: Vec<(String, CentralSpot)> = Vec::new();
    let mut at = directory.cd_start;
    for _ in 0..total {
        if at + 46 > bytes.len() || !bytes[at..].starts_with(b"PK\x01\x02") {
            break;
        }
        let header = &bytes[at..at + 46];
        let name_len = usize::from(u16::from_le_bytes([header[28], header[29]]));
        let extra_len = usize::from(u16::from_le_bytes([header[30], header[31]]));
        let comment_len = usize::from(u16::from_le_bytes([header[32], header[33]]));
        let name_start = at + 46;
        let name_end = name_start + name_len;
        let extra_end = name_end + extra_len;
        if extra_end > bytes.len() {
            break;
        }
        let flags = u16::from_le_bytes([header[8], header[9]]);
        let name = decode_entry_name(&bytes[name_start..name_end], flags);
        let mut spot = raw_spot(header, &bytes[name_end..extra_end]);
        spot.local_offset = spot.local_offset.saturating_add(archive_offset);
        out.push((name, spot));
        at = extra_end + comment_len;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One Stored entry's payload in the witness's
    /// shape (round 33, RT33-3): 600 bytes whose last
    /// 64 carry a fake central-directory signature, in
    /// either of the two payload shapes the lane
    /// witnessed — a fake header whose name length is
    /// 0xFFFF, or a well-formed fake record naming
    /// `JUNK`.
    fn fake_filler_payload(wellformed: bool) -> Vec<u8> {
        let mut payload = vec![b'A'; 600];
        let at = 600 - 64;
        payload[at..at + 4].copy_from_slice(b"PK\x01\x02");
        if wellformed {
            for byte in &mut payload[at + 4..at + 46] {
                *byte = b'B';
            }
            payload[at + 28..at + 30].copy_from_slice(&4u16.to_le_bytes());
            payload[at + 30..at + 34].copy_from_slice(&0u32.to_le_bytes());
            payload[at + 46..at + 50].copy_from_slice(b"JUNK");
        } else {
            payload[at + 28..at + 30].copy_from_slice(&0xFFFFu16.to_le_bytes());
        }
        payload
    }

    /// A minimal archive assembled byte by byte: two
    /// Stored entries (a small one, then the filler, so
    /// the filler's tail is the window a forward hunt
    /// from the declared directory offset would cross),
    /// then the directory and the end structures, the
    /// zip64 pair included when `zip64` is set (with the
    /// plain end fields sentinels, as a zip64 writer
    /// leaves them).
    fn stored_archive(zip64: bool, filler: &[u8]) -> Vec<u8> {
        let entries: [(&[u8], &[u8]); 2] = [(b"a.txt", b"hi"), (b"filler.bin", filler)];
        let mut out: Vec<u8> = Vec::new();
        let mut centrals: Vec<u8> = Vec::new();
        for (name, data) in entries {
            let local_offset = out.len() as u32;
            out.extend_from_slice(b"PK\x03\x04");
            out.extend_from_slice(&20u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&0u32.to_le_bytes());
            out.extend_from_slice(&(data.len() as u32).to_le_bytes());
            out.extend_from_slice(&(data.len() as u32).to_le_bytes());
            out.extend_from_slice(&(name.len() as u16).to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(name);
            out.extend_from_slice(data);

            centrals.extend_from_slice(b"PK\x01\x02");
            centrals.extend_from_slice(&20u16.to_le_bytes());
            centrals.extend_from_slice(&20u16.to_le_bytes());
            centrals.extend_from_slice(&0u16.to_le_bytes());
            centrals.extend_from_slice(&0u16.to_le_bytes());
            centrals.extend_from_slice(&0u16.to_le_bytes());
            centrals.extend_from_slice(&0u16.to_le_bytes());
            centrals.extend_from_slice(&0u32.to_le_bytes());
            centrals.extend_from_slice(&(data.len() as u32).to_le_bytes());
            centrals.extend_from_slice(&(data.len() as u32).to_le_bytes());
            centrals.extend_from_slice(&(name.len() as u16).to_le_bytes());
            centrals.extend_from_slice(&0u16.to_le_bytes());
            centrals.extend_from_slice(&0u16.to_le_bytes());
            centrals.extend_from_slice(&0u16.to_le_bytes());
            centrals.extend_from_slice(&0u16.to_le_bytes());
            centrals.extend_from_slice(&0u32.to_le_bytes());
            centrals.extend_from_slice(&local_offset.to_le_bytes());
            centrals.extend_from_slice(name);
        }
        let cd_offset = out.len() as u64;
        let cd_size = centrals.len() as u64;
        out.extend_from_slice(&centrals);
        if zip64 {
            let record_at = out.len() as u64;
            out.extend_from_slice(b"PK\x06\x06");
            out.extend_from_slice(&44u64.to_le_bytes());
            out.extend_from_slice(&45u16.to_le_bytes());
            out.extend_from_slice(&45u16.to_le_bytes());
            out.extend_from_slice(&0u32.to_le_bytes());
            out.extend_from_slice(&0u32.to_le_bytes());
            out.extend_from_slice(&2u64.to_le_bytes());
            out.extend_from_slice(&2u64.to_le_bytes());
            out.extend_from_slice(&cd_size.to_le_bytes());
            out.extend_from_slice(&cd_offset.to_le_bytes());
            out.extend_from_slice(b"PK\x06\x07");
            out.extend_from_slice(&0u32.to_le_bytes());
            out.extend_from_slice(&record_at.to_le_bytes());
            out.extend_from_slice(&1u32.to_le_bytes());
        }
        out.extend_from_slice(b"PK\x05\x06");
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        if zip64 {
            out.extend_from_slice(&u16::MAX.to_le_bytes());
            out.extend_from_slice(&u16::MAX.to_le_bytes());
            out.extend_from_slice(&u32::MAX.to_le_bytes());
            out.extend_from_slice(&u32::MAX.to_le_bytes());
        } else {
            out.extend_from_slice(&2u16.to_le_bytes());
            out.extend_from_slice(&2u16.to_le_bytes());
            out.extend_from_slice(&(cd_size as u32).to_le_bytes());
            out.extend_from_slice(&(cd_offset as u32).to_le_bytes());
        }
        out.extend_from_slice(&0u16.to_le_bytes());
        out
    }

    fn shifted(bytes: &[u8], junk_len: usize) -> Vec<u8> {
        let mut out = vec![b'J'; junk_len];
        out.extend_from_slice(bytes);
        out
    }

    #[test]
    fn a_shifted_directory_is_derived_arithmetically_not_hunted() {
        // Round 33, F3: in both witnessed payload
        // shapes, a forward hunt from the declared
        // offset stops at the fake signature in the
        // filler's tail; the arithmetic derivation
        // never looks there. The unshifted control
        // derives a zero offset and stays borrowed.
        for wellformed in [false, true] {
            let filler = fake_filler_payload(wellformed);
            let plain = stored_archive(false, &filler);
            let control = central_directory(&plain).expect("unshifted directory");
            assert_eq!(control.archive_offset, 0);
            assert_eq!(control.total, 2);
            assert!(matches!(prefix_free(&plain), Cow::Borrowed(_)));

            let moved = shifted(&plain, 64);
            let directory = central_directory(&moved).expect("shifted directory");
            assert_eq!(directory.archive_offset, 64);
            assert_eq!(directory.total, 2);
            assert_eq!(directory.cd_start, control.cd_start + 64);
            assert_eq!(
                &moved[directory.cd_start..directory.cd_start + 4],
                b"PK\x01\x02",
                "the derived start is the real first record, not the fake"
            );
            assert_eq!(&*prefix_free(&moved), &plain[..]);
        }
    }

    #[test]
    fn a_shifted_zip64_directory_is_derived_arithmetically_too() {
        // The zip64 twin of the witness: the plain end
        // fields are sentinels, so the totals ride in
        // the zip64 record — derived from its arithmetic
        // place before its locator, never hunted from
        // its declared offset, where the fake signature
        // in the window would stop the search the same
        // way (round 33, F3, the code-read half).
        for wellformed in [false, true] {
            let filler = fake_filler_payload(wellformed);
            let plain = stored_archive(true, &filler);
            let control = central_directory(&plain).expect("unshifted zip64 directory");
            assert_eq!(control.archive_offset, 0);
            assert_eq!(control.total, 2);

            let moved = shifted(&plain, 64);
            let directory = central_directory(&moved).expect("shifted zip64 directory");
            assert_eq!(directory.archive_offset, 64);
            assert_eq!(directory.total, 2);
            assert_eq!(directory.cd_start, control.cd_start + 64);
            assert_eq!(&*prefix_free(&moved), &plain[..]);
        }
    }

    /// A zip64 archive assembled byte by byte, its
    /// zip64 record carrying an extensible data sector
    /// of `sector` bytes (the record's size field says
    /// 44 + sector, so the arithmetic derivation
    /// refuses it and the hunted fallback is the only
    /// path — the documented fallback case), over
    /// arbitrary Stored entries.
    fn sector_archive(entries: &[(&[u8], &[u8])], sector: usize) -> Vec<u8> {
        let mut out: Vec<u8> = Vec::new();
        let mut centrals: Vec<u8> = Vec::new();
        for (name, data) in entries {
            let local_offset = out.len() as u32;
            out.extend_from_slice(b"PK\x03\x04");
            out.extend_from_slice(&20u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&0u32.to_le_bytes());
            out.extend_from_slice(&(data.len() as u32).to_le_bytes());
            out.extend_from_slice(&(data.len() as u32).to_le_bytes());
            out.extend_from_slice(&(name.len() as u16).to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(name);
            out.extend_from_slice(data);

            centrals.extend_from_slice(b"PK\x01\x02");
            centrals.extend_from_slice(&45u16.to_le_bytes());
            centrals.extend_from_slice(&20u16.to_le_bytes());
            centrals.extend_from_slice(&0u16.to_le_bytes());
            centrals.extend_from_slice(&0u16.to_le_bytes());
            centrals.extend_from_slice(&0u16.to_le_bytes());
            centrals.extend_from_slice(&0u16.to_le_bytes());
            centrals.extend_from_slice(&0u32.to_le_bytes());
            centrals.extend_from_slice(&(data.len() as u32).to_le_bytes());
            centrals.extend_from_slice(&(data.len() as u32).to_le_bytes());
            centrals.extend_from_slice(&(name.len() as u16).to_le_bytes());
            centrals.extend_from_slice(&0u16.to_le_bytes());
            centrals.extend_from_slice(&0u16.to_le_bytes());
            centrals.extend_from_slice(&0u16.to_le_bytes());
            centrals.extend_from_slice(&0u16.to_le_bytes());
            centrals.extend_from_slice(&0u32.to_le_bytes());
            centrals.extend_from_slice(&local_offset.to_le_bytes());
            centrals.extend_from_slice(name);
        }
        let cd_offset = out.len() as u64;
        let cd_size = centrals.len() as u64;
        out.extend_from_slice(&centrals);
        let record_at = out.len() as u64;
        out.extend_from_slice(b"PK\x06\x06");
        out.extend_from_slice(&((44 + sector) as u64).to_le_bytes());
        out.extend_from_slice(&45u16.to_le_bytes());
        out.extend_from_slice(&45u16.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&(entries.len() as u64).to_le_bytes());
        out.extend_from_slice(&(entries.len() as u64).to_le_bytes());
        out.extend_from_slice(&cd_size.to_le_bytes());
        out.extend_from_slice(&cd_offset.to_le_bytes());
        out.extend_from_slice(&vec![b'S'; sector]);
        out.extend_from_slice(b"PK\x06\x07");
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&record_at.to_le_bytes());
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(b"PK\x05\x06");
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&u16::MAX.to_le_bytes());
        out.extend_from_slice(&u16::MAX.to_le_bytes());
        out.extend_from_slice(&u32::MAX.to_le_bytes());
        out.extend_from_slice(&u32::MAX.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out
    }

    /// The last entry name in the red team's name-fake
    /// witnesses (round 34, RT34-3): 30 filler bytes,
    /// then a fake zip64 record whose fields name a
    /// total of 2, a directory size of 257, and a
    /// directory offset of 0x7F7F7F7F — planted where a
    /// shifted archive's record hunt crosses the last
    /// bytes of the real directory.
    fn fake_record_name() -> Vec<u8> {
        let mut name = vec![b'A'; 30];
        name.extend_from_slice(b"PK\x06\x06");
        name.extend_from_slice(&52u64.to_le_bytes());
        name.extend_from_slice(&45u16.to_le_bytes());
        name.extend_from_slice(&45u16.to_le_bytes());
        name.extend_from_slice(&0u32.to_le_bytes());
        name.extend_from_slice(&0u32.to_le_bytes());
        name.extend_from_slice(&2u64.to_le_bytes());
        name.extend_from_slice(&2u64.to_le_bytes());
        name.extend_from_slice(&257u64.to_le_bytes());
        name.extend_from_slice(&0x7F7F7F7Fu64.to_le_bytes());
        name
    }

    #[test]
    fn the_fallback_never_panics_on_crafted_offsets() {
        // Round 34, F1 (review MAJOR-1): a 98-byte
        // skeleton — a zip64 record at 0 whose size
        // field is 45 (so the arithmetic derivation
        // refuses it, as designed), a locator at 56
        // declaring the record offset, an end record at
        // 76 with sentinel plain fields. The hunt used
        // the locator's declared offset and a found
        // record's directory offset as loop positions
        // unchecked: declared offsets of u64::MAX and
        // u64::MAX - 1 panicked "attempt to add with
        // overflow" at the record hunt, and a found
        // record carrying directory offset u64::MAX
        // panicked the same way at the central hunt
        // (in release the additions wrap and the
        // following slices panic instead). Every shape
        // answers None now, with the bytes passed
        // through untouched.
        let skeleton = |declared: u64, record_cd_offset: Option<u64>| -> Vec<u8> {
            let mut bytes = Vec::new();
            bytes.extend_from_slice(b"PK\x06\x06");
            bytes.extend_from_slice(&45u64.to_le_bytes());
            bytes.extend_from_slice(&[0u8; 44]);
            if let Some(cd_offset) = record_cd_offset {
                bytes[32..40].copy_from_slice(&2u64.to_le_bytes());
                bytes[48..56].copy_from_slice(&cd_offset.to_le_bytes());
            }
            bytes.extend_from_slice(b"PK\x06\x07");
            bytes.extend_from_slice(&0u32.to_le_bytes());
            bytes.extend_from_slice(&declared.to_le_bytes());
            bytes.extend_from_slice(&1u32.to_le_bytes());
            bytes.extend_from_slice(b"PK\x05\x06");
            bytes.extend_from_slice(&0u16.to_le_bytes());
            bytes.extend_from_slice(&0u16.to_le_bytes());
            bytes.extend_from_slice(&u16::MAX.to_le_bytes());
            bytes.extend_from_slice(&u16::MAX.to_le_bytes());
            bytes.extend_from_slice(&u32::MAX.to_le_bytes());
            bytes.extend_from_slice(&u32::MAX.to_le_bytes());
            bytes.extend_from_slice(&0u16.to_le_bytes());
            assert_eq!(bytes.len(), 98);
            bytes
        };
        for bytes in [
            skeleton(u64::MAX, None),
            skeleton(u64::MAX - 1, None),
            skeleton(0, Some(u64::MAX)),
            skeleton(0, None),
        ] {
            assert_eq!(central_directory(&bytes), None);
            assert!(matches!(prefix_free(&bytes), Cow::Borrowed(_)));
        }
    }

    #[test]
    fn a_sector_shifted_directory_is_derived_by_the_validating_hunt() {
        // Round 34, F2 (review MINOR-1): a zip64
        // archive with a 16-byte extensible data sector,
        // shifted by 64 junk bytes, its last entry
        // Stored with the round-33 filler shapes in its
        // tail. The sector is not arithmetically
        // derivable, and the old hunt took the first
        // PK\x01\x02 at-or-after the declared offset —
        // which on a shifted archive stands inside that
        // tail — so it returned the fake record as the
        // directory and prefix_free stripped nothing.
        // The hunt validates each candidate by a walk
        // to the promised end now, keeps searching past
        // one that does not validate, and derives the
        // real directory in both filler shapes; the
        // plain-filler control and the unshifted twin
        // derive exactly as before.
        for wellformed in [false, true] {
            let filler = fake_filler_payload(wellformed);
            let entries: [(&[u8], &[u8]); 2] = [(b"a.txt", b"hi"), (b"filler.bin", &filler)];
            let plain = sector_archive(&entries, 16);
            let control = central_directory(&plain).expect("unshifted sector directory");
            assert_eq!(control.archive_offset, 0);
            assert_eq!(control.total, 2);

            let moved = shifted(&plain, 64);
            let directory = central_directory(&moved).expect("shifted sector directory");
            assert_eq!(directory.archive_offset, 64);
            assert_eq!(directory.total, 2);
            assert_eq!(directory.cd_start, control.cd_start + 64);
            assert_eq!(
                &moved[directory.cd_start..directory.cd_start + 4],
                b"PK\x01\x02",
                "the derived start is the real first record, not the fake"
            );
            assert_eq!(&*prefix_free(&moved), &plain[..]);
        }
        let plain_filler = vec![b'A'; 600];
        let entries: [(&[u8], &[u8]); 2] = [(b"a.txt", b"hi"), (b"filler.bin", &plain_filler)];
        let plain = sector_archive(&entries, 16);
        let moved = shifted(&plain, 64);
        let directory = central_directory(&moved).expect("clean sector shifted directory");
        assert_eq!(directory.archive_offset, 64);
        assert_eq!(&*prefix_free(&moved), &plain[..]);
    }

    #[test]
    fn a_fake_zip64_record_in_the_last_name_does_not_steer_the_hunt() {
        // Round 34, F2 (red team RT34-3): on a shifted
        // sector archive the record hunt's window
        // includes the last bytes of the real directory,
        // and the last record's name there carried a
        // fake PK\x06\x06 with a bogus directory offset:
        // the hunt took it as final and answered None,
        // so the doors scanned an empty directory while
        // the reader served the entries. A hunted
        // record is validated against the locator that
        // follows it now, the hunt keeps searching past
        // the fake, and the real directory is derived —
        // shifted exactly as unshifted.
        let fake_name = fake_record_name();
        let entries: [(&[u8], &[u8]); 2] = [(b"a.txt", b"hi"), (&fake_name, b"tail")];
        let plain = sector_archive(&entries, 8);
        let control = central_directory(&plain).expect("unshifted name-fake directory");
        assert_eq!(control.archive_offset, 0);
        assert_eq!(control.total, 2);

        let moved = shifted(&plain, 64);
        let directory = central_directory(&moved).expect("shifted name-fake directory");
        assert_eq!(directory.archive_offset, 64);
        assert_eq!(directory.total, 2);
        assert_eq!(directory.cd_start, control.cd_start + 64);
        assert_eq!(&*prefix_free(&moved), &plain[..]);
    }

    #[test]
    fn a_directory_followed_by_a_signature_record_is_derived_by_the_hunt() {
        // The other interposed-record shape the
        // fallback exists for: a central-directory
        // digital-signature record (PK\x05\x05) standing
        // between the directory and the end record, its
        // bytes counted in the declared directory size
        // but not in the entry count. The arithmetic
        // walk to the end record cannot close over it;
        // the hunt's walk ends at the signature record
        // and the signature spans exactly to the
        // promised end, unshifted and shifted alike.
        let filler = fake_filler_payload(false);
        let plain_archive = stored_archive(false, &filler);
        let eocd = plain_archive.len() - 22;
        let mut signed = plain_archive[..eocd].to_vec();
        signed.extend_from_slice(b"PK\x05\x05");
        signed.extend_from_slice(&4u16.to_le_bytes());
        signed.extend_from_slice(b"sign");
        signed.extend_from_slice(&plain_archive[eocd..]);
        let signed_eocd = signed.len() - 22;
        let cd_size = u32::from_le_bytes(
            signed[signed_eocd + 12..signed_eocd + 16]
                .try_into()
                .unwrap(),
        ) + 10;
        signed[signed_eocd + 12..signed_eocd + 16].copy_from_slice(&cd_size.to_le_bytes());
        let control = central_directory(&signed).expect("signed directory");
        assert_eq!(control.archive_offset, 0);
        assert_eq!(control.total, 2);

        let moved = shifted(&signed, 64);
        let directory = central_directory(&moved).expect("shifted signed directory");
        assert_eq!(directory.archive_offset, 64);
        assert_eq!(directory.total, 2);
        assert_eq!(directory.cd_start, control.cd_start + 64);
        assert_eq!(&*prefix_free(&moved), &signed[..]);
    }

    #[test]
    fn a_directory_whose_declared_size_covers_an_unaccounted_record_derives_nothing() {
        // The fail-closed twin of the signature shape:
        // an extra central record standing between the
        // directory and the end record, counted in the
        // declared directory size but not in the entry
        // count. The records the count names walk to
        // neither the promised end nor a signature
        // record spanning to it, so no candidate
        // validates and the derivation answers None —
        // the doors refuse such an archive rather than
        // scan a directory that is not the one the
        // reader serves (round 34, F2). The hunt used
        // to return the first record it found here,
        // unvalidated.
        let filler = fake_filler_payload(false);
        let plain_archive = stored_archive(false, &filler);
        let eocd = plain_archive.len() - 22;
        let mut padded = plain_archive[..eocd].to_vec();
        let mut extra = b"PK\x01\x02".to_vec();
        extra.extend_from_slice(&[0u8; 42]);
        extra[28..30].copy_from_slice(&4u16.to_le_bytes());
        extra.extend_from_slice(b"JUNK");
        let extra_len = extra.len() as u32;
        padded.extend_from_slice(&extra);
        padded.extend_from_slice(&plain_archive[eocd..]);
        let padded_eocd = padded.len() - 22;
        let cd_size = u32::from_le_bytes(
            padded[padded_eocd + 12..padded_eocd + 16]
                .try_into()
                .unwrap(),
        ) + extra_len;
        padded[padded_eocd + 12..padded_eocd + 16].copy_from_slice(&cd_size.to_le_bytes());
        assert_eq!(central_directory(&padded), None);
        assert!(matches!(prefix_free(&padded), Cow::Borrowed(_)));
    }

    #[test]
    fn bytes_with_no_end_record_derive_nothing_and_pass_through() {
        assert_eq!(central_directory(b"not a zip"), None);
        let bytes = b"not a zip".to_vec();
        assert_eq!(&*prefix_free(&bytes), &bytes[..]);
    }
}
