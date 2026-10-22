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
//! Round 35, F4: the two flavours were still not
//! equally armed — a zip64 hunt candidate had to
//! agree with its record on the archive offset, a
//! plain candidate was believed on its walk alone.
//! A signature record broke the arithmetic walk (it
//! ended at the signature record, not the end record),
//! and the plain hunt, searching forward from the
//! declared offset in file coordinates, crossed the
//! unused gap between the last entry's data and the
//! real directory on a shifted archive, where a fake
//! directory planted at the gap's head — mirroring the
//! real names, each duplicated name's copies replaced
//! by the first copy's record — walked exactly the
//! declared size and was believed. Both flavours are
//! anchored now, at both derivations: the arithmetic
//! walk carries the hunt's signature-terminator
//! allowance, so a directory followed by a signature
//! record derives arithmetically and never reaches
//! the hunt; and a plain hunt candidate must agree
//! with the end record's own frame — the archive
//! offset left of the end record's position once the
//! declared size and offset are taken away, the
//! signature allowance included — the way a zip64
//! candidate must agree with its record.
//!
//! Round 36, F6 + F7: a second self-consistent end
//! record steered the derivation once more — one
//! appended past the real end record (the scan judged
//! it; the reader, whose disk-field checks the
//! derivation never made, fell back to the real
//! directory or served the appended one), or one
//! planted inside the real end record's own declared
//! comment (both judges took the planted one). The
//! derivation now weighs every end-record candidate
//! under one discipline, stated at
//! `central_directory`: the disk fields (plain and
//! zip64 alike) join the facts a candidate must
//! validate, a candidate standing inside another
//! candidate's declared comment never outranks the
//! candidate that declares the span, a candidate
//! standing entirely before a later candidate's
//! archive offset is that archive's prefix junk, and
//! the earliest standing candidate is the directory
//! — so the scan and the reader judge the same
//! directory, or the package refuses. And the
//! payload span a spot names is computed once, here,
//! with checked arithmetic (`payload_span`): the
//! per-door copies computed it with plain addition
//! on a zip64-resolved offset and panicked on an
//! offset near u64::MAX.
//!
//! Round 37, F8: three more plant sites stood outside
//! that discipline — a fake directory inside a zip64
//! extensible data sector, one inside a stored
//! entry's payload, and one behind a prefix archive
//! whose declared comment length reached into the
//! real archive and defeated its absorption. The
//! discipline is judged against the whole file layout
//! now (stated at `central_directory`): a candidate
//! whose end record stands inside another standing
//! candidate's structural span — its declared
//! entries' payload spans, or its zip64 record,
//! sector, and locator span — cannot outrank the
//! candidate that accounts for those bytes, and a
//! prefix archive's absorption is judged on the later
//! archive's actual start, which a declared comment
//! length reaching into that start cannot defeat.
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

/// Every end-record candidate in the file, in file
/// order: each `PK\x05\x06` whose stated comment
/// length fits in what follows it. The derivation
/// used to take the LAST such candidate as the
/// archive's end record (round 36, F6) — the pick a
/// second self-consistent directory, appended past
/// the real end record or planted inside its
/// declared comment, steered. Every candidate is on
/// the table now, and the discipline at
/// `central_directory` decides between them in one
/// place.
fn end_record_candidates(bytes: &[u8]) -> Vec<usize> {
    let mut out = Vec::new();
    if bytes.len() < 22 {
        return out;
    }
    for at in 0..=bytes.len() - 22 {
        if !bytes[at..].starts_with(b"PK\x05\x06") {
            continue;
        }
        let comment = usize::from(u16_at(bytes, at + 20));
        if at + 22 + comment <= bytes.len() {
            out.push(at);
        }
    }
    out
}

/// Whether a plain end record's disk fields agree
/// the way the archive reader demands (round 36,
/// F6): the disk the record stands on is the disk
/// its directory stands on, and the count it carries
/// for this disk is its whole count — a single-disk
/// archive, the only kind these doors open. The
/// reader rejects a candidate whose disk fields
/// disagree and falls back to an earlier end record;
/// a derivation that never consulted them judged
/// directories the reader never serves.
fn plain_disk_fields_agree(bytes: &[u8], eocd: usize) -> bool {
    u16_at(bytes, eocd + 4) == u16_at(bytes, eocd + 6)
        && u16_at(bytes, eocd + 8) == u16_at(bytes, eocd + 10)
}

/// Whether a zip64 record's disk fields agree with
/// its locator's and with its own counts (round 36,
/// F6), the zip64 counterparts of the plain check:
/// the record's disk is the disk its directory
/// stands on, the locator names that same disk, the
/// archive stands on at most one disk, and the
/// record's count for this disk is its whole count.
fn zip64_disk_fields_agree(bytes: &[u8], record_at: usize, locator_at: usize) -> bool {
    let disk = u32_at(bytes, record_at + 16);
    let cd_disk = u32_at(bytes, record_at + 20);
    let on_this = u64_at(bytes, record_at + 24);
    let total = u64_at(bytes, record_at + 32);
    disk == cd_disk
        && cd_disk == u32_at(bytes, locator_at + 4)
        && u32_at(bytes, locator_at + 16) <= 1
        && on_this == total
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

/// Whether a candidate directory is the real one, by
/// a walk of the central records starting at
/// `cd_start`, `total` of them, to the promised end:
/// the walk may land exactly on the promised end, or
/// on a central-directory digital-signature record
/// (PK\x05\x05) counted in the declared directory
/// size but not in the entry count, spanning exactly
/// from the walk's end to the promised end. The proof
/// a candidate is the real directory and not signature
/// bytes standing in an entry's payload, demanded by
/// the arithmetic path and the hunt alike (the hunt
/// exists to stand past this terminator; since round
/// 35, F4, the arithmetic path allows it too).
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
/// it does. The walk to the promised end carries the
/// hunt's signature-terminator allowance (round 35,
/// F4): the records may end at a digital-signature
/// record spanning exactly to the promised end, the
/// shape that used to drop a signed archive to the
/// hunt. None when the arithmetic does not close
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
        if !zip64_disk_fields_agree(bytes, record_at, locator_at) {
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
        if !hunted_directory_walks_to(bytes, cd_start, total, record_at) {
            return None;
        }
        return Some(CentralDirectory {
            archive_offset,
            cd_start,
            total,
        });
    }
    if !plain_disk_fields_agree(bytes, eocd) {
        return None;
    }
    let total = total_plain;
    let archive_offset = u64::try_from(eocd)
        .ok()?
        .checked_sub(cd_size_plain)?
        .checked_sub(cd_offset_plain)?;
    let cd_start = usize::try_from(cd_offset_plain.checked_add(archive_offset)?).ok()?;
    if !hunted_directory_walks_to(bytes, cd_start, total, eocd) {
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
    if !zip64_disk_fields_agree(bytes, record_at, locator_at) {
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

/// Whether a signature record standing at
/// `promised_end` spans exactly to the end record at
/// `eocd`: the frame's signature allowance, for a
/// signature record the declared directory size does
/// not count (its bytes stand between the promised
/// end and the end record, so the end record's frame
/// overshoots the candidate's implied archive offset
/// by exactly the record's length).
fn signature_record_spans_to(bytes: &[u8], promised_end: usize, eocd: usize) -> bool {
    if promised_end >= eocd || !bytes[promised_end..].starts_with(b"PK\x05\x05") {
        return false;
    }
    let Some(header_end) = promised_end.checked_add(6) else {
        return false;
    };
    if header_end > bytes.len() {
        return false;
    }
    let data_len = usize::from(u16_at(bytes, promised_end + 4));
    header_end.checked_add(data_len) == Some(eocd)
}

/// One central-hunt candidate at `at`, validated the
/// arithmetic path's way (round 34, F2): the archive
/// offset the candidate implies must be the one the
/// validated zip64 record implied, where there is one,
/// and the records at `at` must walk, `total` of them,
/// to the promised end — the declared directory size
/// taken from the candidate's own start. Round 35,
/// F4: a plain candidate carries the same kind of
/// anchor — `plain_frame` is the end record's own
/// frame (the archive offset its position leaves once
/// the declared size and offset are taken away, and
/// the end record's position), and the candidate's
/// implied offset must agree with it, the signature
/// allowance included: a signature record standing at
/// the candidate's promised end and spanning exactly
/// to the end record accounts for the difference.
/// None for the frame itself when the declared size
/// and offset do not fit before the end record — no
/// plain candidate can agree with such a frame.
fn validated_central_candidate(
    bytes: &[u8],
    at: usize,
    total: u64,
    cd_size: u64,
    cd_offset: u64,
    record_archive_offset: Option<u64>,
    plain_frame: Option<(u64, usize)>,
) -> Option<CentralDirectory> {
    let archive_offset = u64::try_from(at).ok()?.checked_sub(cd_offset)?;
    if record_archive_offset.is_some_and(|expected| expected != archive_offset) {
        return None;
    }
    let promised_end = at.checked_add(usize::try_from(cd_size).ok()?)?;
    if !hunted_directory_walks_to(bytes, at, total, promised_end) {
        return None;
    }
    if let Some((frame_offset, eocd)) = plain_frame {
        if archive_offset != frame_offset {
            let sig_len = u64::try_from(eocd.checked_sub(promised_end)?).ok()?;
            if archive_offset.checked_add(sig_len)? != frame_offset
                || !signature_record_spans_to(bytes, promised_end, eocd)
            {
                return None;
            }
        }
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
/// and — for the plain flavour, round 35, F4 — its
/// implied archive offset agrees with the end
/// record's own frame (`validated_central_candidate`);
/// and the hunt keeps
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
    if !zip64_needed && !plain_disk_fields_agree(bytes, eocd) {
        return None;
    }
    // The plain flavour's frame anchor (round 35, F4):
    // the archive offset the end record's own position
    // leaves once the declared size and offset are
    // taken away. A declared size and offset that do
    // not fit before the end record frame no directory
    // at all, so no plain candidate is hunted for.
    let plain_frame: Option<(u64, usize)> = if zip64_needed {
        None
    } else {
        let frame_offset = u64::try_from(eocd)
            .ok()?
            .checked_sub(cd_size)?
            .checked_sub(cd_offset)?;
        Some((frame_offset, eocd))
    };
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
                plain_frame,
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

/// The byte spans one candidate directory's declared
/// entries' payloads occupy in the file, per that
/// candidate's own walk (round 37, F8): each central
/// record the walk stands on names a spot, and the
/// spot's payload span is the checked span every door
/// reads through. A second end record standing inside
/// one of these spans stands inside bytes this
/// candidate accounts for, entry by entry.
fn directory_payload_spans(bytes: &[u8], directory: &CentralDirectory) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    let mut at = directory.cd_start;
    for _ in 0..directory.total {
        let header_end = match at.checked_add(46) {
            Some(end) => end,
            None => break,
        };
        if header_end > bytes.len() || !bytes[at..].starts_with(b"PK\x01\x02") {
            break;
        }
        let name_len = usize::from(u16_at(bytes, at + 28));
        let extra_len = usize::from(u16_at(bytes, at + 30));
        let comment_len = usize::from(u16_at(bytes, at + 32));
        let name_end = match header_end.checked_add(name_len) {
            Some(end) => end,
            None => break,
        };
        let extra_end = match name_end.checked_add(extra_len) {
            Some(end) => end,
            None => break,
        };
        if extra_end > bytes.len() {
            break;
        }
        let header = &bytes[at..header_end];
        let extra = &bytes[name_end..extra_end];
        let mut spot = raw_spot(header, extra);
        spot.local_offset = spot.local_offset.saturating_add(directory.archive_offset);
        if let Some(span) = payload_span(bytes, &spot) {
            spans.push(span);
        }
        match extra_end.checked_add(comment_len) {
            Some(next) => at = next,
            None => break,
        }
    }
    spans
}

/// The byte span one zip64 candidate's end structures
/// occupy (round 37, F8): its zip64 record, the
/// extensible data sector the record's size field
/// carries, and the locator that follows them, from
/// the record's first byte to the end record's first
/// byte. The locator standing immediately before the
/// end record names the record's position in the
/// archive's own coordinates, and the record's
/// declared length must land exactly on that locator
/// — the hunt's own validation, restated as a span.
/// None when the candidate is not a zip64 candidate
/// or its end structures do not stand that way.
fn zip64_structural_span(
    bytes: &[u8],
    eocd: usize,
    directory: &CentralDirectory,
) -> Option<(usize, usize)> {
    let total_plain = u64::from(u16_at(bytes, eocd + 10));
    let cd_size_plain = u64::from(u32_at(bytes, eocd + 12));
    let cd_offset_plain = u64::from(u32_at(bytes, eocd + 16));
    let zip64_needed = total_plain == u64::from(u16::MAX)
        || cd_size_plain == u64::from(u32::MAX)
        || cd_offset_plain == u64::from(u32::MAX);
    if !zip64_needed {
        return None;
    }
    let locator_at = eocd.checked_sub(20)?;
    if !bytes[locator_at..].starts_with(b"PK\x06\x07") {
        return None;
    }
    let declared_record = u64_at(bytes, locator_at + 8);
    let record_at = usize::try_from(directory.archive_offset.checked_add(declared_record)?).ok()?;
    if record_at.checked_add(12)? > bytes.len() || !bytes[record_at..].starts_with(b"PK\x06\x06") {
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
    Some((record_at, eocd))
}

/// The central directory of the archive in `bytes`:
/// where its first record stands, how many entries it
/// holds, and the archive offset every declared offset
/// in it must be shifted by. Arithmetic first (see the
/// module's note), the hunted derivation as fallback.
/// None when no end record stands or no directory can
/// be derived either way — the archive reader reports
/// that damage in its own words.
///
/// Round 36, F6: every end-record candidate that
/// derives a directory stands for judgment, and one
/// discipline decides between them. A candidate
/// standing inside another standing candidate's
/// declared comment span never outranks the candidate
/// that declares the span — a directory planted in an
/// end record's comment is that archive's comment,
/// not its directory. A candidate standing entirely
/// before a later candidate's archive offset is that
/// archive's prefix junk (the self-extracting shape:
/// a whole small archive prepended before the real
/// one), not a competitor. And a candidate whose disk
/// fields disagree never stands at all (the checks in
/// the derivations above), which is the reader's own
/// rule. The earliest standing candidate is the
/// directory: for an appended second directory — its
/// disk fields clean or dirty — the real end record
/// stands earlier and is not absorbed, so the real
/// directory is judged and the doors refuse or serve
/// exactly as the archive's own directory judges.
///
/// Round 37, F8: the discipline is judged against the
/// whole file layout now, not only comment spans and
/// the absorbed prefix. A candidate whose end record
/// stands inside another standing candidate's
/// structural span — the payload spans that
/// candidate's own walk declares, or its zip64 record
/// with its extensible data sector and locator —
/// cannot outrank the candidate that accounts for
/// those bytes: a directory planted in a sector or in
/// an entry's payload is that archive's sector or
/// payload, not its directory. And absorption of a
/// prefix archive is judged on the later archive's
/// actual start: a candidate is absorbed when a later
/// candidate's archive stands at or after the first
/// candidate's end record itself, so a declared
/// comment length that reaches into (or past) the
/// later archive's start cannot defeat absorption —
/// the bytes the comment claims are the later
/// archive's own bytes, and an absorbed candidate's
/// declared comment span subordinates nothing.
pub fn central_directory(bytes: &[u8]) -> Option<CentralDirectory> {
    // Every candidate that derives, in file order,
    // with the span its declared comment covers:
    // (end record position, comment start, comment
    // end, the directory it derives).
    let mut derived: Vec<(usize, usize, usize, CentralDirectory)> = Vec::new();
    for eocd in end_record_candidates(bytes) {
        let Some(directory) =
            arithmetic_directory(bytes, eocd).or_else(|| hunted_directory(bytes, eocd))
        else {
            continue;
        };
        let comment_start = eocd + 22;
        let comment_end = comment_start + usize::from(u16_at(bytes, eocd + 20));
        derived.push((eocd, comment_start, comment_end, directory));
    }
    // Absorption first, judged on each later
    // candidate's actual start against the end record
    // itself (the comment start), not against the end
    // of the comment the earlier candidate declares.
    let absorbed: Vec<bool> = derived
        .iter()
        .enumerate()
        .map(|(i, (_, comment_start, _, _))| {
            derived.iter().enumerate().any(|(j, (_, _, _, other))| {
                j != i && other.archive_offset >= *comment_start as u64
            })
        })
        .collect();
    // Each standing candidate's structural spans, the
    // payload spans of its declared entries and its
    // zip64 end structures, computed once.
    let payload_spans: Vec<Vec<(usize, usize)>> = derived
        .iter()
        .map(|(_, _, _, directory)| directory_payload_spans(bytes, directory))
        .collect();
    let zip64_spans: Vec<Option<(usize, usize)>> = derived
        .iter()
        .map(|(eocd, _, _, directory)| zip64_structural_span(bytes, *eocd, directory))
        .collect();
    for (i, (at, _, _, directory)) in derived.iter().enumerate() {
        if absorbed[i] {
            continue;
        }
        let in_comment = derived
            .iter()
            .enumerate()
            .any(|(j, (_, start, span_end, _))| {
                j != i && !absorbed[j] && *at >= *start && *at < *span_end
            });
        if in_comment {
            continue;
        }
        let in_structural = derived.iter().enumerate().any(|(j, _)| {
            j != i
                && !absorbed[j]
                && (payload_spans[j]
                    .iter()
                    .any(|(start, end)| at >= start && at < end)
                    || zip64_spans[j].is_some_and(|(start, end)| *at >= start && *at < end))
        });
        if in_structural {
            continue;
        }
        return Some(*directory);
    }
    None
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
/// the one `central_directory` derives — the standing
/// candidate's, through its zip64 record where the
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

/// The byte span one central spot's payload occupies
/// in `bytes`, checked end to end (round 36, F7):
/// the local header the spot names must stand where
/// it says, its fixed thirty bytes and its declared
/// name and extra fields must fit in the file, and
/// the compressed bytes the central record declares
/// must fit past them. Every door's payload read
/// starts from this span. The arithmetic that finds
/// it used to be stated per door with plain `+` on
/// the spot's zip64-resolved local offset, unbounded
/// against the file, so an offset near u64::MAX
/// panicked in every door (in debug, "attempt to add
/// with overflow"; in release, the addition wrapped
/// and the slice panicked instead). None when any
/// step does not fit — the payload cannot be read
/// back, which each door reports in its own words.
pub fn payload_span(bytes: &[u8], spot: &CentralSpot) -> Option<(usize, usize)> {
    let local = usize::try_from(spot.local_offset).ok()?;
    let header_end = local.checked_add(30)?;
    if header_end > bytes.len() || !bytes[local..].starts_with(b"PK\x03\x04") {
        return None;
    }
    let name_len = usize::from(u16_at(bytes, local + 26));
    let extra_len = usize::from(u16_at(bytes, local + 28));
    let data_start = header_end.checked_add(name_len)?.checked_add(extra_len)?;
    let data_end = data_start.checked_add(usize::try_from(spot.comp_size).ok()?)?;
    if data_end > bytes.len() {
        return None;
    }
    Some((data_start, data_end))
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
        // but not in the entry count. The walk ends at
        // the signature record and the signature spans
        // exactly to the promised end, unshifted and
        // shifted alike — the standard the hunt's walk
        // defined, which the arithmetic derivation
        // carries since round 35, F4 (this shape now
        // derives arithmetically; the hunt is no longer
        // reached for it, and the outcome is the one
        // this pin has always stated).
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

    /// The round-35 gap craft (red team RT35-1),
    /// assembled byte by byte in the crafter's shape:
    /// Stored entries (a duplicated name carrying two
    /// different payloads, then a filler), a 512-byte
    /// gap of unused bytes between the last entry's
    /// data and the real directory, a central-directory
    /// digital-signature record standing after the real
    /// directory when `sig` is set (counted in the
    /// declared directory size unless `sig_counted` is
    /// false), and a complete fake directory planted at
    /// the head of the gap when `fake` is set — the
    /// real records mirrored by name, each duplicated
    /// name's copies replaced by the first copy's
    /// record (its local offset re-based by the gap),
    /// the last record's comment padding the fake walk
    /// to exactly the declared directory size. The
    /// whole archive is shifted by 4096 junk bytes.
    /// Returns the bytes and the real directory's
    /// declared offset, in archive coordinates.
    fn gap_archive(fake: bool, sig: bool, sig_counted: bool, zip64: bool) -> (Vec<u8>, u64) {
        const GAP: u64 = 512;
        const SHIFT: usize = 4096;
        let filler = vec![b'A'; 600];
        let entries: [(&[u8], &[u8]); 3] = [
            (b"a.txt", b"first copy"),
            (b"a.txt", b"second copy"),
            (b"filler.bin", &filler),
        ];
        let mut body: Vec<u8> = Vec::new();
        let mut locals: Vec<u32> = Vec::new();
        for (name, data) in entries {
            locals.push(body.len() as u32);
            body.extend_from_slice(b"PK\x03\x04");
            body.extend_from_slice(&20u16.to_le_bytes());
            body.extend_from_slice(&0u16.to_le_bytes());
            body.extend_from_slice(&0u16.to_le_bytes());
            body.extend_from_slice(&0u16.to_le_bytes());
            body.extend_from_slice(&0u16.to_le_bytes());
            body.extend_from_slice(&0u32.to_le_bytes());
            body.extend_from_slice(&(data.len() as u32).to_le_bytes());
            body.extend_from_slice(&(data.len() as u32).to_le_bytes());
            body.extend_from_slice(&(name.len() as u16).to_le_bytes());
            body.extend_from_slice(&0u16.to_le_bytes());
            body.extend_from_slice(name);
            body.extend_from_slice(data);
        }
        let cd_offset = body.len() as u64 + GAP;
        let central = |name: &[u8], data: &[u8], local_offset: u32, comment: u16| {
            let mut rec: Vec<u8> = Vec::new();
            rec.extend_from_slice(b"PK\x01\x02");
            rec.extend_from_slice(&45u16.to_le_bytes());
            rec.extend_from_slice(&20u16.to_le_bytes());
            rec.extend_from_slice(&0u16.to_le_bytes());
            rec.extend_from_slice(&0u16.to_le_bytes());
            rec.extend_from_slice(&0u16.to_le_bytes());
            rec.extend_from_slice(&0u16.to_le_bytes());
            rec.extend_from_slice(&0u32.to_le_bytes());
            rec.extend_from_slice(&(data.len() as u32).to_le_bytes());
            rec.extend_from_slice(&(data.len() as u32).to_le_bytes());
            rec.extend_from_slice(&(name.len() as u16).to_le_bytes());
            rec.extend_from_slice(&0u16.to_le_bytes());
            rec.extend_from_slice(&comment.to_le_bytes());
            rec.extend_from_slice(&0u16.to_le_bytes());
            rec.extend_from_slice(&0u16.to_le_bytes());
            rec.extend_from_slice(&0u32.to_le_bytes());
            rec.extend_from_slice(&local_offset.to_le_bytes());
            rec.extend_from_slice(name);
            rec.extend_from_slice(&vec![b'C'; usize::from(comment)]);
            rec
        };
        let mut centrals: Vec<u8> = Vec::new();
        for (i, (name, data)) in entries.iter().enumerate() {
            centrals.extend_from_slice(&central(name, data, locals[i], 0));
        }
        let sig_len: u64 = if sig { 22 } else { 0 };
        let cd_size = centrals.len() as u64 + if sig_counted { sig_len } else { 0 };
        // The fake: the first copy's record for every
        // entry, in archive order, its local offset
        // re-based by the gap, the last record's
        // comment padding the walk to the declared size.
        let mut fake_recs: Vec<u8> = Vec::new();
        for (i, (name, _)) in entries.iter().enumerate() {
            let first = entries
                .iter()
                .position(|(other, _)| other == name)
                .expect("the entry's own first copy stands");
            let comment = if i + 1 == entries.len() {
                u16::try_from(cd_size - fake_recs.len() as u64 - 46 - name.len() as u64)
                    .expect("the fake walk pads to the declared size")
            } else {
                0
            };
            fake_recs.extend_from_slice(&central(
                name,
                entries[first].1,
                locals[first] + GAP as u32,
                comment,
            ));
        }
        assert_eq!(fake_recs.len() as u64, cd_size);
        assert!(fake_recs.len() as u64 <= GAP);
        let mut gap = vec![b'G'; GAP as usize];
        if fake {
            gap[..fake_recs.len()].copy_from_slice(&fake_recs);
        }
        let mut archive: Vec<u8> = Vec::new();
        archive.extend_from_slice(&body);
        archive.extend_from_slice(&gap);
        archive.extend_from_slice(&centrals);
        if sig {
            archive.extend_from_slice(b"PK\x05\x05");
            archive.extend_from_slice(&16u16.to_le_bytes());
            archive.extend_from_slice(b"SSSSSSSSSSSSSSSS");
        }
        if zip64 {
            let record_at = archive.len() as u64;
            archive.extend_from_slice(b"PK\x06\x06");
            archive.extend_from_slice(&44u64.to_le_bytes());
            archive.extend_from_slice(&45u16.to_le_bytes());
            archive.extend_from_slice(&45u16.to_le_bytes());
            archive.extend_from_slice(&0u32.to_le_bytes());
            archive.extend_from_slice(&0u32.to_le_bytes());
            archive.extend_from_slice(&(entries.len() as u64).to_le_bytes());
            archive.extend_from_slice(&(entries.len() as u64).to_le_bytes());
            archive.extend_from_slice(&cd_size.to_le_bytes());
            archive.extend_from_slice(&cd_offset.to_le_bytes());
            archive.extend_from_slice(b"PK\x06\x07");
            archive.extend_from_slice(&0u32.to_le_bytes());
            archive.extend_from_slice(&record_at.to_le_bytes());
            archive.extend_from_slice(&1u32.to_le_bytes());
            archive.extend_from_slice(b"PK\x05\x06");
            archive.extend_from_slice(&0u16.to_le_bytes());
            archive.extend_from_slice(&0u16.to_le_bytes());
            archive.extend_from_slice(&u16::MAX.to_le_bytes());
            archive.extend_from_slice(&u16::MAX.to_le_bytes());
            archive.extend_from_slice(&u32::MAX.to_le_bytes());
            archive.extend_from_slice(&u32::MAX.to_le_bytes());
        } else {
            archive.extend_from_slice(b"PK\x05\x06");
            archive.extend_from_slice(&0u16.to_le_bytes());
            archive.extend_from_slice(&0u16.to_le_bytes());
            archive.extend_from_slice(&(entries.len() as u16).to_le_bytes());
            archive.extend_from_slice(&(entries.len() as u16).to_le_bytes());
            archive.extend_from_slice(&(cd_size as u32).to_le_bytes());
            archive.extend_from_slice(&(cd_offset as u32).to_le_bytes());
        }
        archive.extend_from_slice(&0u16.to_le_bytes());
        let mut out = vec![b'J'; SHIFT];
        out.extend_from_slice(&archive);
        (out, cd_offset)
    }

    #[test]
    fn a_fake_directory_in_the_gap_before_the_real_one_does_not_steer_the_derivation() {
        // Round 35, F4 (red team RT35-1): the signature
        // record made the arithmetic derivation fail
        // its walk, and the plain hunt that followed
        // believed a candidate on its walk alone — so
        // a fake directory planted in the gap between
        // the last entry's data and the real directory
        // validated, with an implied archive offset
        // short of the real shift by exactly the gap.
        // Every shape below derives the REAL directory:
        // the gap craft itself, the gap-clean control
        // (gap and signature record, no fake), the
        // no-signature control (fake planted, nothing
        // to break the arithmetic walk), the zip64 twin
        // (the hunt's record-offset agreement already
        // rejected its fake), and the craft with the
        // signature record standing past the declared
        // size.
        let shapes: [(&str, bool, bool, bool, bool); 5] = [
            ("gapfake", true, true, true, false),
            ("gapclean", false, true, true, false),
            ("gapnosig", true, false, true, false),
            ("zip64gapfake", true, true, true, true),
            ("gapfake-uncounted-sig", true, true, false, false),
        ];
        for (label, fake, sig, sig_counted, zip64) in shapes {
            let (bytes, cd_offset) = gap_archive(fake, sig, sig_counted, zip64);
            let directory = central_directory(&bytes).expect("the real directory derives");
            assert_eq!(directory.archive_offset, 4096, "{label}");
            assert_eq!(directory.total, 3, "{label}");
            assert_eq!(directory.cd_start as u64, cd_offset + 4096, "{label}");
            assert_eq!(
                &bytes[directory.cd_start..directory.cd_start + 4],
                b"PK\x01\x02",
                "{label}: the derived start is the real first record"
            );
            assert_eq!(&*prefix_free(&bytes), &bytes[4096..], "{label}");
            let spots = central_spots(&bytes);
            let names: Vec<&str> = spots.iter().map(|(name, _)| name.as_str()).collect();
            assert_eq!(names, ["a.txt", "a.txt", "filler.bin"], "{label}");
            assert_eq!(spots[0].1.comp_size, 10, "{label}: the first copy's record");
            assert_eq!(
                spots[1].1.comp_size, 11,
                "{label}: the second copy's own record, not the first copy's"
            );
        }
    }

    #[test]
    fn bytes_with_no_end_record_derive_nothing_and_pass_through() {
        assert_eq!(central_directory(b"not a zip"), None);
        let bytes = b"not a zip".to_vec();
        assert_eq!(&*prefix_free(&bytes), &bytes[..]);
    }

    /// One plain end record with the given disk fields
    /// and comment, the tail of every hand-built
    /// archive below.
    fn plain_eocd(total: u16, cd_size: u32, cd_offset: u32, disk: u16, comment: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"PK\x05\x06");
        out.extend_from_slice(&disk.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&total.to_le_bytes());
        out.extend_from_slice(&total.to_le_bytes());
        out.extend_from_slice(&cd_size.to_le_bytes());
        out.extend_from_slice(&cd_offset.to_le_bytes());
        out.extend_from_slice(&(comment.len() as u16).to_le_bytes());
        out.extend_from_slice(comment);
        out
    }

    /// The round-36 second-directory crafts (review
    /// R36-6 + red team RT36-5), assembled byte by
    /// byte: Stored entries a.txt in two disagreeing
    /// copies (payloads of different lengths, so the
    /// copies' records differ) and a filler, then the
    /// real directory and end record — and past them,
    /// per `shape`, nothing (the control), a second
    /// well-formed directory naming each name once
    /// (its records the first copies') with its own
    /// end record whose disk field is `disk`, or that
    /// same second directory planted inside the real
    /// end record's own comment, where the planted end
    /// record stands at the file's end. Returns the
    /// bytes and the real directory's start.
    fn second_directory_archive(shape: &str) -> (Vec<u8>, usize) {
        let entries: [(&[u8], &[u8]); 3] = [
            (b"a.txt", b"first copy"),
            (b"a.txt", b"second copy!"),
            (b"filler.bin", b"tail"),
        ];
        let mut body: Vec<u8> = Vec::new();
        let mut locals: Vec<u32> = Vec::new();
        for (name, data) in entries {
            locals.push(body.len() as u32);
            body.extend_from_slice(b"PK\x03\x04");
            body.extend_from_slice(&20u16.to_le_bytes());
            body.extend_from_slice(&0u16.to_le_bytes());
            body.extend_from_slice(&0u16.to_le_bytes());
            body.extend_from_slice(&0u16.to_le_bytes());
            body.extend_from_slice(&0u16.to_le_bytes());
            body.extend_from_slice(&0u32.to_le_bytes());
            body.extend_from_slice(&(data.len() as u32).to_le_bytes());
            body.extend_from_slice(&(data.len() as u32).to_le_bytes());
            body.extend_from_slice(&(name.len() as u16).to_le_bytes());
            body.extend_from_slice(&0u16.to_le_bytes());
            body.extend_from_slice(name);
            body.extend_from_slice(data);
        }
        let central = |name: &[u8], data: &[u8], local_offset: u32| {
            let mut rec: Vec<u8> = Vec::new();
            rec.extend_from_slice(b"PK\x01\x02");
            rec.extend_from_slice(&45u16.to_le_bytes());
            rec.extend_from_slice(&20u16.to_le_bytes());
            rec.extend_from_slice(&0u16.to_le_bytes());
            rec.extend_from_slice(&0u16.to_le_bytes());
            rec.extend_from_slice(&0u16.to_le_bytes());
            rec.extend_from_slice(&0u16.to_le_bytes());
            rec.extend_from_slice(&0u32.to_le_bytes());
            rec.extend_from_slice(&(data.len() as u32).to_le_bytes());
            rec.extend_from_slice(&(data.len() as u32).to_le_bytes());
            rec.extend_from_slice(&(name.len() as u16).to_le_bytes());
            rec.extend_from_slice(&0u16.to_le_bytes());
            rec.extend_from_slice(&0u16.to_le_bytes());
            rec.extend_from_slice(&0u16.to_le_bytes());
            rec.extend_from_slice(&0u16.to_le_bytes());
            rec.extend_from_slice(&0u32.to_le_bytes());
            rec.extend_from_slice(&local_offset.to_le_bytes());
            rec.extend_from_slice(name);
            rec
        };
        let cd_offset = body.len();
        let mut centrals: Vec<u8> = Vec::new();
        for (i, (name, data)) in entries.iter().enumerate() {
            centrals.extend_from_slice(&central(name, data, locals[i]));
        }
        // The second directory: the first copy's record
        // for every name, in archive order.
        let mut fake_recs: Vec<u8> = Vec::new();
        for (i, (name, data)) in entries.iter().enumerate() {
            if entries[..i].iter().any(|(other, _)| other == name) {
                continue;
            }
            fake_recs.extend_from_slice(&central(name, data, locals[i]));
        }
        let mut out = body;
        out.extend_from_slice(&centrals);
        match shape {
            "plain" => {
                out.extend_from_slice(&plain_eocd(
                    3,
                    centrals.len() as u32,
                    cd_offset as u32,
                    0,
                    &[],
                ));
            }
            "comment" => {
                let planted_offset = (cd_offset + centrals.len() + 22) as u32;
                let mut comment = fake_recs.clone();
                comment.extend_from_slice(&plain_eocd(
                    2,
                    fake_recs.len() as u32,
                    planted_offset,
                    0,
                    &[],
                ));
                out.extend_from_slice(&plain_eocd(
                    3,
                    centrals.len() as u32,
                    cd_offset as u32,
                    0,
                    &comment,
                ));
            }
            _ => {
                // "appended-clean" (disk 0) or
                // "appended-dirty" (disk 1).
                let disk = if shape == "appended-dirty" { 1 } else { 0 };
                out.extend_from_slice(&plain_eocd(
                    3,
                    centrals.len() as u32,
                    cd_offset as u32,
                    0,
                    &[],
                ));
                let fake_offset = out.len() as u32;
                out.extend_from_slice(&fake_recs);
                out.extend_from_slice(&plain_eocd(
                    2,
                    fake_recs.len() as u32,
                    fake_offset,
                    disk,
                    &[],
                ));
            }
        }
        (out, cd_offset)
    }

    #[test]
    fn a_second_self_consistent_directory_never_steers_the_derivation() {
        // Round 36, F6: a second well-formed directory
        // standing past the real end record, or inside
        // the real end record's own declared comment,
        // was the directory the derivation judged —
        // it names each part once, its records the
        // first copies', so the duplicate the real
        // directory records never stood in the judged
        // directory at all. The real directory is
        // derived in every shape now: the appended
        // shapes (its end record's disk fields clean
        // or dirty — the reader rejects the dirty one
        // and falls back, and the clean one must not
        // outrank the real candidate either) and the
        // comment shape (a candidate standing inside
        // another candidate's declared comment span
        // never outranks the candidate that declares
        // the span).
        for shape in ["plain", "appended-clean", "appended-dirty", "comment"] {
            let (bytes, cd_offset) = second_directory_archive(shape);
            let directory = central_directory(&bytes).expect("the real directory derives");
            assert_eq!(directory.archive_offset, 0, "{shape}");
            assert_eq!(directory.total, 3, "{shape}");
            assert_eq!(directory.cd_start, cd_offset, "{shape}");
            let spots = central_spots(&bytes);
            let names: Vec<&str> = spots.iter().map(|(name, _)| name.as_str()).collect();
            assert_eq!(names, ["a.txt", "a.txt", "filler.bin"], "{shape}");
            assert_eq!(spots[0].1.comp_size, 10, "{shape}: the first copy's record");
            assert_eq!(
                spots[1].1.comp_size, 12,
                "{shape}: the second copy's own record stands in the judged directory"
            );
        }
    }

    #[test]
    fn a_complete_archive_standing_before_the_real_one_is_prefix_junk() {
        // The tiny-prefix control (red team, round 36,
        // held line): a whole small archive prepended
        // before the real one is the self-extracting
        // shape — the real archive is the shifted one
        // whose directory the reader serves, and the
        // small archive standing entirely before its
        // archive offset is junk, not a competing
        // candidate. The earliest-candidate discipline
        // above must not hand the derivation to it.
        let filler = fake_filler_payload(false);
        let real = stored_archive(false, &filler);
        let tiny_entries: [(&[u8], &[u8]); 1] = [(b"tiny.txt", b"tiny")];
        let mut tiny: Vec<u8> = Vec::new();
        let mut tiny_centrals: Vec<u8> = Vec::new();
        for (name, data) in tiny_entries {
            let local_offset = tiny.len() as u32;
            tiny.extend_from_slice(b"PK\x03\x04");
            tiny.extend_from_slice(&20u16.to_le_bytes());
            tiny.extend_from_slice(&0u16.to_le_bytes());
            tiny.extend_from_slice(&0u16.to_le_bytes());
            tiny.extend_from_slice(&0u16.to_le_bytes());
            tiny.extend_from_slice(&0u16.to_le_bytes());
            tiny.extend_from_slice(&0u32.to_le_bytes());
            tiny.extend_from_slice(&(data.len() as u32).to_le_bytes());
            tiny.extend_from_slice(&(data.len() as u32).to_le_bytes());
            tiny.extend_from_slice(&(name.len() as u16).to_le_bytes());
            tiny.extend_from_slice(&0u16.to_le_bytes());
            tiny.extend_from_slice(name);
            tiny.extend_from_slice(data);
            tiny_centrals.extend_from_slice(b"PK\x01\x02");
            tiny_centrals.extend_from_slice(&20u16.to_le_bytes());
            tiny_centrals.extend_from_slice(&20u16.to_le_bytes());
            tiny_centrals.extend_from_slice(&0u16.to_le_bytes());
            tiny_centrals.extend_from_slice(&0u16.to_le_bytes());
            tiny_centrals.extend_from_slice(&0u16.to_le_bytes());
            tiny_centrals.extend_from_slice(&0u16.to_le_bytes());
            tiny_centrals.extend_from_slice(&0u32.to_le_bytes());
            tiny_centrals.extend_from_slice(&(data.len() as u32).to_le_bytes());
            tiny_centrals.extend_from_slice(&(data.len() as u32).to_le_bytes());
            tiny_centrals.extend_from_slice(&(name.len() as u16).to_le_bytes());
            tiny_centrals.extend_from_slice(&0u16.to_le_bytes());
            tiny_centrals.extend_from_slice(&0u16.to_le_bytes());
            tiny_centrals.extend_from_slice(&0u16.to_le_bytes());
            tiny_centrals.extend_from_slice(&0u16.to_le_bytes());
            tiny_centrals.extend_from_slice(&0u32.to_le_bytes());
            tiny_centrals.extend_from_slice(&local_offset.to_le_bytes());
            tiny_centrals.extend_from_slice(name);
        }
        let tiny_cd_offset = tiny.len() as u32;
        tiny.extend_from_slice(&tiny_centrals);
        tiny.extend_from_slice(&plain_eocd(
            1,
            tiny_centrals.len() as u32,
            tiny_cd_offset,
            0,
            &[],
        ));
        let shift = tiny.len();
        let mut bytes = tiny;
        bytes.extend_from_slice(&real);
        let directory = central_directory(&bytes).expect("the real directory derives");
        assert_eq!(directory.archive_offset, shift as u64);
        assert_eq!(directory.total, 2);
        assert_eq!(&*prefix_free(&bytes), &real[..]);
    }

    #[test]
    fn the_disk_fields_join_the_facts_the_derivation_validates() {
        // Round 36, F6: the derivation arithmetically
        // trusted every end-record field except the
        // disk fields, which the reader does judge —
        // an end record whose disk number disagrees
        // with the disk its directory stands on is one
        // the reader rejects, so a derivation that
        // believed it judged a directory the reader
        // never serves. The plain form and the zip64
        // counterparts (the record's disk fields, its
        // count for this disk against its total) all
        // refuse to derive now.
        let filler = fake_filler_payload(false);
        let plain = stored_archive(false, &filler);
        let eocd = plain.len() - 22;
        let mut dirty_disk = plain.clone();
        dirty_disk[eocd + 4..eocd + 6].copy_from_slice(&1u16.to_le_bytes());
        assert_eq!(central_directory(&dirty_disk), None);
        let mut short_count = plain.clone();
        short_count[eocd + 8..eocd + 10].copy_from_slice(&1u16.to_le_bytes());
        assert_eq!(central_directory(&short_count), None);

        let zip64 = stored_archive(true, &filler);
        let record_at = zip64.len() - 22 - 20 - 56;
        assert_eq!(&zip64[record_at..record_at + 4], b"PK\x06\x06");
        let mut dirty_record_disk = zip64.clone();
        dirty_record_disk[record_at + 16..record_at + 20].copy_from_slice(&1u32.to_le_bytes());
        assert_eq!(central_directory(&dirty_record_disk), None);
        let mut short_record_count = zip64.clone();
        short_record_count[record_at + 24..record_at + 32].copy_from_slice(&1u64.to_le_bytes());
        assert_eq!(central_directory(&short_record_count), None);
    }

    /// The round-37 planted-directory crafts (red
    /// team RT37-2), assembled byte by byte: Stored
    /// entries a.txt in two disagreeing copies and a
    /// filler, then the real directory — and per
    /// `shape`, a second self-consistent directory
    /// naming each name once (the first copies'
    /// records) planted where round 36's discipline
    /// does not name: "z1" inside the real archive's
    /// zip64 extensible data sector (the real archive
    /// is a sector archive), "z2" inside the filler
    /// entry's payload, "z3" behind a complete tiny
    /// prefix archive whose end record declares a
    /// 64-byte comment reaching into the real archive,
    /// "z3b" the same prefix with the declared comment
    /// covering the whole real archive. "plain" is the
    /// unplanted control. Returns the bytes, the real
    /// directory's start in file coordinates, and the
    /// real archive's offset.
    fn planted_directory_archive(shape: &str) -> (Vec<u8>, usize, u64) {
        let entries: [(&[u8], &[u8]); 3] = [
            (b"a.txt", b"first copy"),
            (b"a.txt", b"second copy!"),
            (b"filler.bin", &[b'A'; 600]),
        ];
        let mut body: Vec<u8> = Vec::new();
        let mut locals: Vec<u32> = Vec::new();
        for (name, data) in entries {
            locals.push(body.len() as u32);
            body.extend_from_slice(b"PK\x03\x04");
            body.extend_from_slice(&20u16.to_le_bytes());
            body.extend_from_slice(&0u16.to_le_bytes());
            body.extend_from_slice(&0u16.to_le_bytes());
            body.extend_from_slice(&0u16.to_le_bytes());
            body.extend_from_slice(&0u16.to_le_bytes());
            body.extend_from_slice(&0u32.to_le_bytes());
            body.extend_from_slice(&(data.len() as u32).to_le_bytes());
            body.extend_from_slice(&(data.len() as u32).to_le_bytes());
            body.extend_from_slice(&(name.len() as u16).to_le_bytes());
            body.extend_from_slice(&0u16.to_le_bytes());
            body.extend_from_slice(name);
            body.extend_from_slice(data);
        }
        let central = |name: &[u8], data: &[u8], local_offset: u32| {
            let mut rec: Vec<u8> = Vec::new();
            rec.extend_from_slice(b"PK\x01\x02");
            rec.extend_from_slice(&45u16.to_le_bytes());
            rec.extend_from_slice(&20u16.to_le_bytes());
            rec.extend_from_slice(&0u16.to_le_bytes());
            rec.extend_from_slice(&0u16.to_le_bytes());
            rec.extend_from_slice(&0u16.to_le_bytes());
            rec.extend_from_slice(&0u16.to_le_bytes());
            rec.extend_from_slice(&0u32.to_le_bytes());
            rec.extend_from_slice(&(data.len() as u32).to_le_bytes());
            rec.extend_from_slice(&(data.len() as u32).to_le_bytes());
            rec.extend_from_slice(&(name.len() as u16).to_le_bytes());
            rec.extend_from_slice(&0u16.to_le_bytes());
            rec.extend_from_slice(&0u16.to_le_bytes());
            rec.extend_from_slice(&0u16.to_le_bytes());
            rec.extend_from_slice(&0u16.to_le_bytes());
            rec.extend_from_slice(&0u32.to_le_bytes());
            rec.extend_from_slice(&local_offset.to_le_bytes());
            rec.extend_from_slice(name);
            rec
        };
        let cd_offset = body.len();
        let mut centrals: Vec<u8> = Vec::new();
        for (i, (name, data)) in entries.iter().enumerate() {
            centrals.extend_from_slice(&central(name, data, locals[i]));
        }
        let mut fake_recs: Vec<u8> = Vec::new();
        let mut fake_total: u16 = 0;
        for (i, (name, data)) in entries.iter().enumerate() {
            if entries[..i].iter().any(|(other, _)| other == name) {
                continue;
            }
            fake_recs.extend_from_slice(&central(name, data, locals[i]));
            fake_total += 1;
        }
        let fake_eocd =
            |cd_pos: usize| plain_eocd(fake_total, fake_recs.len() as u32, cd_pos as u32, 0, &[]);
        match shape {
            "z1" => {
                let mut out = body;
                out.extend_from_slice(&centrals);
                let record_at = out.len();
                let sector_len = 2048usize;
                let fake_cd_pos = record_at + 56 + 16;
                let mut sector = vec![b'S'; sector_len];
                sector[16..16 + fake_recs.len()].copy_from_slice(&fake_recs);
                let planted_eocd = fake_eocd(fake_cd_pos);
                let eocd_at = 16 + fake_recs.len();
                sector[eocd_at..eocd_at + 22].copy_from_slice(&planted_eocd);
                out.extend_from_slice(b"PK\x06\x06");
                out.extend_from_slice(&((44 + sector_len) as u64).to_le_bytes());
                out.extend_from_slice(&45u16.to_le_bytes());
                out.extend_from_slice(&45u16.to_le_bytes());
                out.extend_from_slice(&0u32.to_le_bytes());
                out.extend_from_slice(&0u32.to_le_bytes());
                out.extend_from_slice(&(entries.len() as u64).to_le_bytes());
                out.extend_from_slice(&(entries.len() as u64).to_le_bytes());
                out.extend_from_slice(&(centrals.len() as u64).to_le_bytes());
                out.extend_from_slice(&(cd_offset as u64).to_le_bytes());
                out.extend_from_slice(&sector);
                out.extend_from_slice(b"PK\x06\x07");
                out.extend_from_slice(&0u32.to_le_bytes());
                out.extend_from_slice(&(record_at as u64).to_le_bytes());
                out.extend_from_slice(&1u32.to_le_bytes());
                out.extend_from_slice(b"PK\x05\x06");
                out.extend_from_slice(&0u16.to_le_bytes());
                out.extend_from_slice(&0u16.to_le_bytes());
                out.extend_from_slice(&u16::MAX.to_le_bytes());
                out.extend_from_slice(&u16::MAX.to_le_bytes());
                out.extend_from_slice(&u32::MAX.to_le_bytes());
                out.extend_from_slice(&u32::MAX.to_le_bytes());
                out.extend_from_slice(&0u16.to_le_bytes());
                (out, cd_offset, 0)
            }
            "z2" => {
                let payload_at = locals[2] as usize + 30 + entries[2].0.len();
                let fake_cd_pos = payload_at + 100;
                body[fake_cd_pos..fake_cd_pos + fake_recs.len()].copy_from_slice(&fake_recs);
                let planted_eocd = fake_eocd(fake_cd_pos);
                let eocd_at = fake_cd_pos + fake_recs.len();
                body[eocd_at..eocd_at + 22].copy_from_slice(&planted_eocd);
                let mut out = body;
                out.extend_from_slice(&centrals);
                out.extend_from_slice(&plain_eocd(
                    entries.len() as u16,
                    centrals.len() as u32,
                    cd_offset as u32,
                    0,
                    &[],
                ));
                (out, cd_offset, 0)
            }
            "z3" | "z3b" => {
                let mut real = body;
                real.extend_from_slice(&centrals);
                real.extend_from_slice(&plain_eocd(
                    entries.len() as u16,
                    centrals.len() as u32,
                    cd_offset as u32,
                    0,
                    &[],
                ));
                // The tiny prefix archive: one entry,
                // its own directory, and an end record
                // whose declared comment length is
                // stated outright below — no comment
                // bytes follow it; the real archive
                // stands where they would.
                let tiny_name: &[u8] = b"tiny.txt";
                let tiny_data: &[u8] = b"tiny";
                let mut tiny: Vec<u8> = Vec::new();
                tiny.extend_from_slice(b"PK\x03\x04");
                tiny.extend_from_slice(&20u16.to_le_bytes());
                tiny.extend_from_slice(&0u16.to_le_bytes());
                tiny.extend_from_slice(&0u16.to_le_bytes());
                tiny.extend_from_slice(&0u16.to_le_bytes());
                tiny.extend_from_slice(&0u16.to_le_bytes());
                tiny.extend_from_slice(&0u32.to_le_bytes());
                tiny.extend_from_slice(&(tiny_data.len() as u32).to_le_bytes());
                tiny.extend_from_slice(&(tiny_data.len() as u32).to_le_bytes());
                tiny.extend_from_slice(&(tiny_name.len() as u16).to_le_bytes());
                tiny.extend_from_slice(&0u16.to_le_bytes());
                tiny.extend_from_slice(tiny_name);
                tiny.extend_from_slice(tiny_data);
                let tiny_cd_offset = tiny.len();
                let tiny_central = central(tiny_name, tiny_data, 0);
                tiny.extend_from_slice(&tiny_central);
                let comment_len: u16 = if shape == "z3b" {
                    u16::try_from(real.len().min(usize::from(u16::MAX)))
                        .expect("the declared comment fits its field")
                } else {
                    64
                };
                tiny.extend_from_slice(b"PK\x05\x06");
                tiny.extend_from_slice(&0u16.to_le_bytes());
                tiny.extend_from_slice(&0u16.to_le_bytes());
                tiny.extend_from_slice(&1u16.to_le_bytes());
                tiny.extend_from_slice(&1u16.to_le_bytes());
                tiny.extend_from_slice(&(tiny_central.len() as u32).to_le_bytes());
                tiny.extend_from_slice(&(tiny_cd_offset as u32).to_le_bytes());
                tiny.extend_from_slice(&comment_len.to_le_bytes());
                let archive_offset = tiny.len() as u64;
                let real_cd_start = tiny.len() + cd_offset;
                tiny.extend_from_slice(&real);
                (tiny, real_cd_start, archive_offset)
            }
            _ => {
                let mut out = body;
                out.extend_from_slice(&centrals);
                out.extend_from_slice(&plain_eocd(
                    entries.len() as u16,
                    centrals.len() as u32,
                    cd_offset as u32,
                    0,
                    &[],
                ));
                (out, cd_offset, 0)
            }
        }
    }

    #[test]
    fn a_directory_planted_in_a_sector_a_payload_or_behind_a_prefix_comment_never_steers() {
        // Round 37, F8 (red team RT37-2): round 36's
        // discipline named only two plant sites — a
        // candidate inside another candidate's declared
        // comment span, and a candidate standing
        // entirely before a later candidate's archive
        // offset. A complete fake directory planted in
        // a zip64 extensible data sector, in a stored
        // entry's payload, or behind a prefix archive
        // whose declared comment reaches into the real
        // archive stood in none of those places, stood
        // earliest in file order, and was judged — the
        // scan judged the planted directory (each name
        // once) while the reader served the real one.
        // The real directory is judged in every shape
        // now, the plain control included: every entry
        // stands in it, the second copy under its own
        // record.
        for shape in ["plain", "z1", "z2", "z3", "z3b"] {
            let (bytes, cd_start, archive_offset) = planted_directory_archive(shape);
            let directory = central_directory(&bytes).expect("the real directory derives");
            assert_eq!(directory.archive_offset, archive_offset, "{shape}");
            assert_eq!(directory.total, 3, "{shape}");
            assert_eq!(directory.cd_start, cd_start, "{shape}");
            let spots = central_spots(&bytes);
            let names: Vec<&str> = spots.iter().map(|(name, _)| name.as_str()).collect();
            assert_eq!(names, ["a.txt", "a.txt", "filler.bin"], "{shape}");
            assert_eq!(spots[0].1.comp_size, 10, "{shape}: the first copy's record");
            assert_eq!(
                spots[1].1.comp_size, 12,
                "{shape}: the second copy's own record stands in the judged directory"
            );
            if archive_offset > 0 {
                assert_eq!(
                    &*prefix_free(&bytes),
                    &bytes[archive_offset as usize..],
                    "{shape}: the prefix-free view is the real archive"
                );
            }
        }
    }

    #[test]
    fn a_legitimate_sector_archive_still_derives_and_lists_every_entry() {
        // The hunt-safety control for F8: the
        // arithmetic derivation refuses a zip64 archive
        // carrying an extensible data sector by design
        // and the hunt serves it — the structural-span
        // discipline above must not make such an
        // archive unreadable. A plain sector archive,
        // unshifted and shifted, derives its real
        // directory and lists every entry.
        let filler = vec![b'A'; 600];
        let entries: [(&[u8], &[u8]); 2] = [(b"a.txt", b"hi"), (b"filler.bin", &filler)];
        let plain = sector_archive(&entries, 512);
        let control = central_directory(&plain).expect("unshifted sector directory");
        assert_eq!(control.archive_offset, 0);
        assert_eq!(control.total, 2);
        let names: Vec<String> = central_spots(&plain)
            .into_iter()
            .map(|(name, _)| name)
            .collect();
        assert_eq!(names, ["a.txt", "filler.bin"]);

        let moved = shifted(&plain, 64);
        let directory = central_directory(&moved).expect("shifted sector directory");
        assert_eq!(directory.archive_offset, 64);
        assert_eq!(directory.total, 2);
        assert_eq!(directory.cd_start, control.cd_start + 64);
        assert_eq!(&*prefix_free(&moved), &plain[..]);
    }
}
