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
//! Round 38, F1: four more plant sites stood outside
//! the round-37 spans, which covered an entry's
//! payload (compressed data) only: a fake directory
//! planted in a local header's extra field, in a
//! central record's extra field, in a central
//! record's comment field, or in a digital-signature
//! record's data derived, stood earliest, and was
//! judged while the reader served the real archive.
//! Candidate standing is judged against the whole
//! walked extent now (stated at `central_directory`):
//! a candidate whose end record stands anywhere
//! inside another standing candidate's walked spans
//! — its entries' local headers whole as well as
//! their payloads, its central records whole, and a
//! signature record standing at its walk's end —
//! cannot outrank the candidate that accounts for
//! those bytes.
//!
//! Round 39, F1 + F2: three more regions stood
//! outside the walked extent — the unused gap before
//! the central directory, the region before the
//! first local header, and the local header of an
//! entry whose payload span does not resolve (its
//! span was pushed only inside the payload span's
//! success) — and a directory planted in any of them
//! steered the doors again. The walked extent
//! accounts for every byte between a standing
//! candidate's archive start and the end of its
//! directory walk now, gaps included, and a local
//! header stands in the extent from its own fields
//! whether or not its payload span resolves. The
//! extent's gap fill is judged with an accounting
//! guard (stated at `central_directory`): it
//! subordinates only a candidate whose local
//! headers the filling candidate's walk wholly
//! accounts for, so an appended second directory —
//! which names only the first copies' headers —
//! cannot swallow the real directory's end record.
//! And a zip64 archive whose signature record was not
//! counted in the declared directory size derived
//! nothing at all — both zip64 derivations demanded
//! the directory end equal the zip64 record's
//! position exactly — leaving a directory planted
//! in that signature record the sole standing
//! candidate. A zip64 candidate whose record
//! validation fails only by a signature record
//! standing at its directory walk's end derives
//! under the signature allowance the plain frame
//! already carries: the signature record's own
//! declared length closes the gap, counted or not.
//!
//! Round 40, F3 + F4: two more regions stood outside
//! the discipline — everything *after* a candidate's
//! directory walk (the gap between the directory and
//! the end record, the gap after a signature record,
//! and the three gaps inside a zip64 tail), where a
//! planted directory was again the sole standing
//! candidate; and the gap fill's strict-superset
//! guard, which a plant whose declared local-offset
//! set is *equal* to the real directory's escaped
//! entirely (renamed, phantom, and truncated entries
//! judged over the real ones). The walked extent
//! accounts for the archive's tail as it accounts
//! for its head and middle now — from the archive's
//! start through the candidate's own end record —
//! and the fill subordinates on judged facts, not
//! offset sets alone: a candidate in another's
//! extent whose records differ in any judged fact
//! from the extent owner's records for the same
//! offsets is the owner's bytes, while an exact
//! mirror (no disagreement) judges identically
//! either way and may stand. A host whose tail gaps
//! make it underivable while its structures stand
//! derives under a tail-gap derivation (the same
//! repair class F2 gave the uncounted signature
//! record), and the prefix-free view splices a
//! judged archive's tail gaps out, so at the two
//! locator-side zip64 gaps — whose clean hosts the
//! reader refuses on the unspliced bytes — the view
//! the doors hand the reader opens the real
//! directory, and a plant standing in the gap is
//! spliced away with it.
//!
//! Round 41, F1 + F2: the round-40 equal-set rule's
//! compared facts took only a record's name and spot,
//! grouped per local offset, so a plant declaring
//! exactly the real directory's offset set escaped
//! wherever it differed only in a fact that
//! comparison never read — the walk order across
//! offsets, the uncompressed size, an extra field,
//! the version-made-by. The compared facts take the
//! whole record now, compared as sequences in walk
//! order, so mirror-standing is confined to
//! candidates whose records agree in every field and
//! in their order. And the round-40 splice chose
//! between zip64 record+locator pairs on fields it
//! never validated: a fake pair in the
//! locator-to-end-record gap differing from the real
//! pair only in a version-needed of 99 was laid into
//! the view, and the reader — which refuses a zip64
//! record whose version-needed exceeds its
//! version-made-by — then refused the good archive
//! at every door. The splice's pair choice validates
//! that version consistency now, so the view carries
//! a pair the reader will judge.
//!
//! Round 43, F6: a sixth generation appended a
//! second complete archive after the first — plant
//! locals, plant directory, plant end record, the
//! plant records carrying the real entries' names
//! and offsets with the key entry's record describing
//! plant content. The trailing candidate derives with
//! the real archive's frame (its end record declares
//! its directory's absolute position), so its walked
//! extent spans the whole file; and where the plant
//! content matched the real key entry's length, its
//! declared local-offset set equalled the real
//! directory's, so the extent's record-facts arm read
//! the REAL candidate as a rewriting standing in the
//! trailing candidate's extent and subordinated it —
//! the trailing directory was judged, and the one
//! door that reads through the key record's checksum
//! failed the craft its clean control reads. The fill
//! does not reach back across a complete archive's
//! end now: a candidate whose body stands again
//! after the filled candidate's comment end, before
//! its own directory (`trailing_body_restands`), is
//! a trailing archive appended after that archive,
//! and the first complete directory is the directory
//! judged.
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

/// The end of the directory walk starting at
/// `cd_start`, and the end of the region that walk
/// accounts for at its tail: the walk's own end, or
/// the end of a digital-signature record standing
/// there (its header and its declared data), when
/// one stands and fits in the file. None when the
/// walk does not complete.
fn directory_walk_and_signature_end(
    bytes: &[u8],
    cd_start: usize,
    total: u64,
) -> Option<(usize, usize)> {
    let walk_end = directory_walk_end(bytes, cd_start, total)?;
    let mut extent_end = walk_end;
    if walk_end < bytes.len() && bytes[walk_end..].starts_with(b"PK\x05\x05") {
        if let Some(header_end) = walk_end.checked_add(6) {
            if header_end <= bytes.len() {
                let data_len = usize::from(u16_at(bytes, walk_end + 4));
                if let Some(sig_end) = header_end.checked_add(data_len) {
                    if sig_end <= bytes.len() {
                        extent_end = sig_end;
                    }
                }
            }
        }
    }
    Some((walk_end, extent_end))
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
        let archive_offset = u64::try_from(record_at)
            .ok()?
            .checked_sub(declared_record)?;
        let cd_start = usize::try_from(cd_offset.checked_add(archive_offset)?).ok()?;
        // The directory ends where the zip64 record
        // stands, in the archive's own coordinates —
        // or at a digital-signature record standing at
        // the walk's end whose own declared length
        // closes the gap to the record, counted in the
        // declared directory size or not (round 39,
        // F2): the signature allowance the plain frame
        // already carries, one flavour over.
        if cd_offset.checked_add(cd_size)? != declared_record
            && !signature_record_closes_to_zip64_record(bytes, cd_start, total, record_at)
        {
            return None;
        }
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
        // The signature allowance, as in the
        // arithmetic derivation (round 39, F2): the
        // record stands when a signature record at
        // the directory walk's end accounts for every
        // byte between the declared directory end and
        // the record's own position.
        let cd_start = usize::try_from(cd_offset.checked_add(archive_offset)?).ok()?;
        if !signature_record_closes_to_zip64_record(bytes, cd_start, total, record_at) {
            return None;
        }
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

/// Whether a digital-signature record standing at
/// the end of the directory walk starting at
/// `cd_start` spans exactly to the zip64 record at
/// `record_at` (round 39, F2): the record's own
/// declared length closes the gap between the
/// directory's end and the zip64 record, whether or
/// not the declared directory size counted its
/// bytes. The walk itself must land where the
/// signature record stands — a gap of any other
/// shape closes nothing.
fn signature_record_closes_to_zip64_record(
    bytes: &[u8],
    cd_start: usize,
    total: u64,
    record_at: usize,
) -> bool {
    let Some(walk_end) = directory_walk_end(bytes, cd_start, total) else {
        return false;
    };
    signature_record_spans_to(bytes, walk_end, record_at)
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

/// The tail-gap derivation (round 40, F3): the one
/// fallback for an archive whose end structures stand
/// where its end record (or, for zip64, its locator)
/// says they stand, but with unused bytes standing
/// *after* the directory walk — between the directory
/// and the end record, after a signature record at
/// the walk's end, or inside a zip64 tail (directory
/// to zip64 record, record to locator, locator to end
/// record). The reader tolerates the plain and
/// CD-side gaps — it locates the directory from the
/// declared offsets and sizes, never by contiguity —
/// so a host carrying one derived nothing while the
/// reader served it, and a directory planted in the
/// gap was the only candidate the scan derived. This
/// derivation validates everything the arithmetic
/// path and the hunt validate — the disk fields, the
/// declared counts and sizes, a walk of the declared
/// records whose length is exactly the declared
/// directory size (a signature record at the walk's
/// end counted in that size or not) — except tail
/// contiguity: the walk's accounted end (the
/// signature record's end, where one stands) must
/// stand at or before the next end structure, and
/// the bytes between are the archive's tail gap.
/// Candidates are tried nearest the end record
/// first, the way the tail itself is read: a
/// directory standing nearer the archive's head
/// validates only where the real directory nearer
/// the end record does not, and the discipline at
/// `central_directory` judges between every candidate
/// that derives, this one included.
fn tail_gap_directory(bytes: &[u8], eocd: usize) -> Option<CentralDirectory> {
    let total_plain = u64::from(u16_at(bytes, eocd + 10));
    let cd_size_plain = u64::from(u32_at(bytes, eocd + 12));
    let cd_offset_plain = u64::from(u32_at(bytes, eocd + 16));
    let zip64_needed = total_plain == u64::from(u16::MAX)
        || cd_size_plain == u64::from(u32::MAX)
        || cd_offset_plain == u64::from(u32::MAX);
    if zip64_needed {
        return tail_gap_zip64_directory(bytes, eocd);
    }
    if !plain_disk_fields_agree(bytes, eocd) {
        return None;
    }
    let total = total_plain;
    let cd_size = usize::try_from(cd_size_plain).ok()?;
    if total == 0 || cd_size == 0 {
        return None;
    }
    // The directory's first record stands at or after
    // its declared offset in file coordinates (the
    // hunt's own starting point), and its accounted
    // end stands at or before the end record.
    let first = usize::try_from(cd_offset_plain).ok()?;
    let last = eocd.checked_sub(cd_size)?;
    if last < first {
        return None;
    }
    let mut at = last;
    loop {
        if bytes[at..].starts_with(b"PK\x01\x02") {
            if let Some((walk_end, extent_end)) = directory_walk_and_signature_end(bytes, at, total)
            {
                let walked = walk_end - at;
                let accounted = extent_end - at;
                if (walked == cd_size || accounted == cd_size) && extent_end <= eocd {
                    let archive_offset = u64::try_from(at).ok()?.checked_sub(cd_offset_plain)?;
                    return Some(CentralDirectory {
                        archive_offset,
                        cd_start: at,
                        total,
                    });
                }
            }
        }
        if at == first {
            return None;
        }
        at -= 1;
    }
}

/// The zip64 arm of the tail-gap derivation: the
/// locator stands somewhere before the end record —
/// immediately before it on a contiguous tail,
/// earlier where a gap stands between them — and the
/// zip64 record stands somewhere before its locator,
/// with gaps allowed on both sides. Nearest the end
/// record first, as on the plain arm.
fn tail_gap_zip64_directory(bytes: &[u8], eocd: usize) -> Option<CentralDirectory> {
    if eocd < 20 {
        return None;
    }
    let mut locator_at = eocd - 20;
    loop {
        if bytes[locator_at..].starts_with(b"PK\x06\x07") {
            if let Some(directory) = tail_gap_zip64_at_locator(bytes, locator_at) {
                return Some(directory);
            }
        }
        if locator_at == 0 {
            return None;
        }
        locator_at -= 1;
    }
}

/// One locator candidate for the tail-gap zip64
/// derivation, at `locator_at`: the zip64 record it
/// names, validated the hunt's way but with the gaps
/// on either side of it allowed — its own declared
/// length (fixed part plus any extensible data
/// sector) must end at or before the locator, its
/// disk fields must agree, the archive offset its
/// position implies is the one its directory fields
/// are read through, and the directory those fields
/// name must walk exactly its declared size and end
/// at or before the record's own position.
fn tail_gap_zip64_at_locator(bytes: &[u8], locator_at: usize) -> Option<CentralDirectory> {
    let declared_record = u64_at(bytes, locator_at + 8);
    if locator_at < 56 {
        return None;
    }
    let mut record_at = locator_at - 56;
    loop {
        if bytes[record_at..].starts_with(b"PK\x06\x06") {
            if let Some(directory) =
                tail_gap_zip64_record(bytes, record_at, locator_at, declared_record)
            {
                return Some(directory);
            }
        }
        if record_at == 0 {
            return None;
        }
        record_at -= 1;
    }
}

/// One zip64 record candidate for the tail-gap
/// derivation, at `record_at`, against the locator at
/// `locator_at` declaring it at `declared_record` (in
/// the archive's own coordinates).
fn tail_gap_zip64_record(
    bytes: &[u8],
    record_at: usize,
    locator_at: usize,
    declared_record: u64,
) -> Option<CentralDirectory> {
    let fixed_end = record_at.checked_add(56)?;
    if fixed_end > bytes.len() {
        return None;
    }
    let size_field = u64_at(bytes, record_at + 4);
    if size_field < 44 {
        return None;
    }
    let record_len = usize::try_from(size_field.checked_add(12)?).ok()?;
    if record_at.checked_add(record_len)? > locator_at {
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
    if total == 0 {
        return None;
    }
    let cd_start = usize::try_from(cd_offset.checked_add(archive_offset)?).ok()?;
    let (walk_end, extent_end) = directory_walk_and_signature_end(bytes, cd_start, total)?;
    let cd_size = usize::try_from(cd_size).ok()?;
    if walk_end - cd_start != cd_size && extent_end - cd_start != cd_size {
        return None;
    }
    if extent_end > record_at {
        return None;
    }
    Some(CentralDirectory {
        archive_offset,
        cd_start,
        total,
    })
}

/// The byte span one central spot's local header
/// occupies in `bytes`, computed from the local
/// header's own fields alone — its fixed thirty
/// bytes and its declared name and extra fields —
/// with checked arithmetic throughout (round 39,
/// F1). Unlike `payload_span` this never consults
/// the declared compressed size, so a header stands
/// even where its entry's payload cannot be read
/// back. None when the header does not stand where
/// the spot says or its fields do not fit in the
/// file.
fn local_header_span(bytes: &[u8], spot: &CentralSpot) -> Option<(usize, usize)> {
    let local = usize::try_from(spot.local_offset).ok()?;
    let header_end = local.checked_add(30)?;
    if header_end > bytes.len() || !bytes[local..].starts_with(b"PK\x03\x04") {
        return None;
    }
    let name_len = usize::from(u16_at(bytes, local + 26));
    let extra_len = usize::from(u16_at(bytes, local + 28));
    let data_start = header_end.checked_add(name_len)?.checked_add(extra_len)?;
    if data_start > bytes.len() {
        return None;
    }
    Some((local, data_start))
}

/// The byte spans one candidate directory's walk
/// accounts for whole (round 38, F1): each declared
/// entry's local header whole — its fixed part, name,
/// and extra field — together with the payload span
/// the checked helper names; the central records
/// whole, from the directory's first record to the
/// walk's end, each record's header, name, extra, and
/// comment included; and a digital-signature record
/// standing at the walk's end, its header and data.
/// The round-37 spans covered only the payloads
/// (compressed data), so a directory planted in a
/// local header's extra field, in a central record's
/// extra or comment field, or in a signature record's
/// data stood in no span and steered every door. A
/// second end record standing inside one of these
/// spans stands inside bytes this candidate's own
/// walk accounts for.
///
/// Round 39, F1: three regions still stood outside
/// — the unused gap between the last entry's payload
/// and the directory, the region between the
/// archive's own start and its first local header,
/// and the local header of an entry whose payload
/// span does not resolve (the header span was pushed
/// only inside the payload span's success). Each
/// local header's span is computed from the local
/// header's own fields now, whether or not its
/// payload span resolves; the two gap regions are
/// the walked extent's fill, stated at
/// `directory_walked_extent` and judged at
/// `central_directory`.
fn directory_walked_spans(bytes: &[u8], directory: &CentralDirectory) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    let mut at = directory.cd_start;
    let mut walked = true;
    for _ in 0..directory.total {
        let header_end = match at.checked_add(46) {
            Some(end) => end,
            None => {
                walked = false;
                break;
            }
        };
        if header_end > bytes.len() || !bytes[at..].starts_with(b"PK\x01\x02") {
            walked = false;
            break;
        }
        let name_len = usize::from(u16_at(bytes, at + 28));
        let extra_len = usize::from(u16_at(bytes, at + 30));
        let comment_len = usize::from(u16_at(bytes, at + 32));
        let name_end = match header_end.checked_add(name_len) {
            Some(end) => end,
            None => {
                walked = false;
                break;
            }
        };
        let extra_end = match name_end.checked_add(extra_len) {
            Some(end) => end,
            None => {
                walked = false;
                break;
            }
        };
        if extra_end > bytes.len() {
            walked = false;
            break;
        }
        let header = &bytes[at..header_end];
        let extra = &bytes[name_end..extra_end];
        let mut spot = raw_spot(header, extra);
        spot.local_offset = spot.local_offset.saturating_add(directory.archive_offset);
        // The local header stands in the extent from
        // its own fields (round 39, F1), whether or
        // not the entry's payload span resolves: an
        // entry whose declared compressed size
        // overruns the file used to account for no
        // bytes at all, its header included.
        if let Some(span) = local_header_span(bytes, &spot) {
            spans.push(span);
        }
        if let Some(span) = payload_span(bytes, &spot) {
            spans.push(span);
        }
        match extra_end.checked_add(comment_len) {
            Some(next) => at = next,
            None => {
                walked = false;
                break;
            }
        }
    }
    if walked {
        spans.push((directory.cd_start, at));
        if at < bytes.len() && bytes[at..].starts_with(b"PK\x05\x05") {
            if let Some(header_end) = at.checked_add(6) {
                if header_end <= bytes.len() {
                    let data_len = usize::from(u16_at(bytes, at + 4));
                    if let Some(sig_end) = header_end.checked_add(data_len) {
                        if sig_end <= bytes.len() {
                            spans.push((at, sig_end));
                        }
                    }
                }
            }
        }
    }
    spans
}

/// The whole run of bytes one candidate's walk
/// covers, from the archive's own start to the end
/// of its directory walk — a signature record
/// standing at the walk's end included (round 39,
/// F1). The walked spans above account for the
/// headers, payloads, and records themselves; this
/// run accounts for everything between them too:
/// the unused gap before the central directory and
/// the region before the first local header are the
/// candidate's bytes the way its accounted spans
/// are. None when the walk does not complete.
fn directory_walked_extent(bytes: &[u8], directory: &CentralDirectory) -> Option<(usize, usize)> {
    let (_, extent_end) =
        directory_walk_and_signature_end(bytes, directory.cd_start, directory.total)?;
    let archive_start = usize::try_from(directory.archive_offset).ok()?;
    if archive_start > extent_end {
        return None;
    }
    Some((archive_start, extent_end))
}

/// Every local offset one candidate's walk accounts
/// for, in the file's own coordinates: the local
/// header each of its central records names, shifted
/// by its archive offset. Two candidates over one
/// body of local headers are told apart by these
/// sets (round 39, F1): a planted or appended
/// directory names each duplicated name's first copy
/// only, so the second copies' local headers stand
/// in the real directory's set and in no other's.
fn directory_local_offsets(bytes: &[u8], directory: &CentralDirectory) -> Vec<u64> {
    let mut offsets = Vec::new();
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
        offsets.push(spot.local_offset);
        match extra_end.checked_add(comment_len) {
            Some(next) => at = next,
            None => break,
        }
    }
    offsets
}

/// One central record's whole judged fact (round 41,
/// F1): everything the doors or the zip reader can
/// read out of the record — the decoded name and its
/// raw bytes, the spot (method, compressed size,
/// checksum, local offset in the file's own
/// coordinates, the sizes and offset resolved through
/// the zip64 extra field), the versions made by and
/// needed, the flags, the modification time and
/// date, the declared uncompressed size, the disk
/// the entry starts on, the internal and external
/// attributes, and the raw extra and comment fields.
/// The round-40 fact carried only the name and the
/// spot, so a plant differing in any other field —
/// or only in the walk's order — read as an exact
/// mirror of the real directory.
#[derive(Debug, Clone, PartialEq, Eq)]
struct RecordFact {
    name: String,
    spot: CentralSpot,
    version_made_by: u16,
    version_needed: u16,
    flags: u16,
    mod_time: u16,
    mod_date: u16,
    uncomp_size: u32,
    disk_start: u16,
    internal_attrs: u16,
    external_attrs: u32,
    name_bytes: Vec<u8>,
    extra: Vec<u8>,
    comment: Vec<u8>,
}

/// Every judged fact one candidate's walk declares,
/// record by record, in walk order: the whole record
/// (`RecordFact`) of each record its directory holds.
/// Two candidates over one body of local headers are
/// told apart by these facts where their offset sets
/// agree (round 40, F4, completed round 41, F1): a
/// planted directory whose records rename an entry,
/// invent one at an offset already named, truncate a
/// declared size, reorder the walk, rewrite an
/// uncompressed size or an extra field, or restate a
/// version declares the same offsets as the real
/// directory and different facts.
fn directory_record_facts(bytes: &[u8], directory: &CentralDirectory) -> Vec<RecordFact> {
    let mut facts = Vec::new();
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
        let flags = u16_at(bytes, at + 8);
        let name = decode_entry_name(&bytes[header_end..name_end], flags);
        let mut spot = raw_spot(header, extra);
        spot.local_offset = spot.local_offset.saturating_add(directory.archive_offset);
        let fact = RecordFact {
            name,
            spot,
            version_made_by: u16_at(bytes, at + 4),
            version_needed: u16_at(bytes, at + 6),
            flags,
            mod_time: u16_at(bytes, at + 12),
            mod_date: u16_at(bytes, at + 14),
            uncomp_size: u32_at(bytes, at + 24),
            disk_start: u16_at(bytes, at + 34),
            internal_attrs: u16_at(bytes, at + 36),
            external_attrs: u32_at(bytes, at + 38),
            name_bytes: bytes[header_end..name_end].to_vec(),
            extra: extra.to_vec(),
            comment: bytes
                .get(extra_end..extra_end + comment_len)
                .unwrap_or(&[])
                .to_vec(),
        };
        facts.push(fact);
        match extra_end.checked_add(comment_len) {
            Some(next) => at = next,
            None => break,
        }
    }
    facts
}

/// Whether two candidates' record facts disagree in
/// any judged fact (round 40, F4, completed round
/// 41, F1): the two walks compared as sequences, in
/// walk order, whole record against whole record —
/// a disagreement in any record's any field, or the
/// same records standing in a different walk order,
/// is a disagreement. The round-40 comparison
/// grouped the records per local offset and read
/// only the name and the spot, so a plant that
/// merely reordered the walk across offsets, or
/// rewrote a field it never read, grouped identically
/// to the real directory. An exact mirror (the same
/// whole records in the same order) does not
/// disagree: it judges identically whichever
/// candidate stands, so subordinating it would cost
/// the standing mirror disciplines (an appended or
/// comment-planted copy over a duplicate-free body)
/// their real candidate for nothing.
fn record_facts_differ(mine: &[RecordFact], theirs: &[RecordFact]) -> bool {
    mine != theirs
}

/// Whether the gap fill of the candidate owning
/// `owner_offsets` / `owner_facts` subordinates the
/// candidate declaring `plant_offsets` / `plant_facts`
/// (round 39, F1, with the round-40 F4 rule): the
/// owner's walk must account for every local header
/// the plant's walk accounts for, and the plant must
/// be the owner's bytes rather than its equal — the
/// owner accounts for at least one header more (the
/// strict-superset guard that keeps an appended
/// second directory from swallowing the real one),
/// or the two candidates' records differ in any
/// judged fact — the whole records, compared as
/// sequences in walk order (round 41, F1) — in which
/// case the plant is a rewriting of the owner's
/// records, not a copy of them.
fn gap_fill_subordinates(
    plant_offsets: &[u64],
    plant_facts: &[RecordFact],
    owner_offsets: &[u64],
    owner_facts: &[RecordFact],
) -> bool {
    if !plant_offsets
        .iter()
        .all(|offset| owner_offsets.contains(offset))
    {
        return false;
    }
    if owner_offsets
        .iter()
        .any(|offset| !plant_offsets.contains(offset))
    {
        return true;
    }
    record_facts_differ(plant_facts, owner_facts)
}

/// Whether the candidate owning `owner`'s frame has
/// its body standing again after the candidate whose
/// archive ends at `filled_comment_end` (round 43,
/// F6): the owner's first record's local header
/// stands at the position the owner's own frame
/// predicts once that frame is moved to the filled
/// candidate's comment end — a position that is not
/// the owner's own named local — and the name it
/// carries there is the name the owner's first record
/// declares. That is a second, complete body of local
/// headers standing between the filled archive's end
/// and the owner's directory: the owner is a trailing
/// archive appended after a complete first archive,
/// not an archive whose gap bytes the filled
/// candidate stands in, and its walked extent's span
/// of the whole file is the concatenation's shape,
/// not an accounting of the filled candidate's bytes.
/// A gap plant's owner places no such body in the
/// interval — between the plant's end and the owner's
/// directory stand gap bytes, or the owner's own
/// first local header in the owner's own frame — so
/// the check is silent for every prior generation.
fn trailing_body_restands(
    bytes: &[u8],
    filled_comment_end: usize,
    owner: &CentralDirectory,
    owner_offsets: &[u64],
    owner_facts: &[RecordFact],
) -> bool {
    if filled_comment_end > owner.cd_start {
        return false;
    }
    let (Some(&first_local), Some(first_fact)) = (owner_offsets.first(), owner_facts.first())
    else {
        return false;
    };
    let Some(relative) = first_local.checked_sub(owner.archive_offset) else {
        return false;
    };
    let Ok(relative) = usize::try_from(relative) else {
        return false;
    };
    let Some(at) = filled_comment_end.checked_add(relative) else {
        return false;
    };
    if at >= owner.cd_start || u64::try_from(at).ok() == Some(first_local) {
        return false;
    }
    let Some(header_end) = at.checked_add(30) else {
        return false;
    };
    if header_end > bytes.len() || !bytes[at..].starts_with(b"PK\x03\x04") {
        return false;
    }
    let name_len = usize::from(u16_at(bytes, at + 26));
    let Some(name_end) = header_end.checked_add(name_len) else {
        return false;
    };
    if name_end > bytes.len() {
        return false;
    }
    bytes[header_end..name_end] == first_fact.name_bytes[..]
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
///
/// Round 38, F1: the structural spans are the whole
/// walked extent now. A candidate whose end record
/// stands anywhere inside another standing
/// candidate's walked spans — the local headers whole
/// (fixed part, name, and extra field) and payloads
/// of the entries its own walk declares, its central
/// records whole (header, name, extra, and comment),
/// and a digital-signature record standing at its
/// walk's end — cannot outrank the candidate that
/// accounts for those bytes: a directory planted in
/// a local header's extra field, a central record's
/// extra or comment field, or a signature record's
/// data is that archive's header, record, or
/// signature bytes, not its directory.
///
/// Round 39, F1 + F2: the walked extent runs from
/// the candidate's archive start to the end of its
/// directory walk, every gap byte included, and a
/// local header stands in it from its own fields
/// whether or not its entry's payload span resolves
/// — a directory planted in the gap before the
/// directory, before the first local header, or in
/// the header of a spanless entry is this archive's
/// bytes, not its directory. The extent's gap fill
/// subordinates a candidate only where the filling
/// candidate's walk accounts for every local header
/// the filled candidate's walk accounts for, and at
/// least one more: an appended second directory
/// names only the first copies' local headers (or,
/// over a duplicate-free body, exactly the real
/// one's), so its fill cannot swallow the end record
/// of the directory that accounts for them all, and
/// the round-36 appended discipline stands. And a
/// zip64 candidate
/// failing record validation only by an uncounted
/// signature record at its walk's end derives under
/// the signature allowance, so the real candidate
/// stands and the plant inside that signature
/// record is subordinated as on plain hosts.
///
/// Round 40, F3 + F4: the walked extent runs from
/// the candidate's archive start through its own end
/// record now — the post-directory gaps and, for
/// zip64, the structural records and the gaps
/// between them are the candidate's bytes the way
/// the gaps before its directory are — and a host
/// whose tail gaps make it underivable while its
/// end structures stand derives under the tail-gap
/// derivation (`tail_gap_directory`), so the real
/// candidate stands and a plant in the tail is
/// subordinated. The fill's accounting guard judges
/// record facts, not offset sets alone: a candidate
/// in the extent whose records differ in any judged
/// fact from the extent owner's records for the
/// same offsets is subordinated even where its
/// declared offset set equals the owner's, while an
/// exact mirror — no judged fact differing — judges
/// identically either way and may stand.
///
/// Round 41, F1: the judged facts are the whole
/// record now, compared as sequences in walk order
/// (see `RecordFact`): a candidate in the extent
/// whose records differ from the extent owner's in
/// any field of any record — the walk order itself
/// included — is subordinated even where every
/// round-40 fact agreed, and mirror-standing is
/// confined to candidates whose whole records agree
/// in every field and in their order.
///
/// Round 43, F6: the extent's gap fill does not
/// reach back across a complete archive's end. A
/// second complete archive appended after the first
/// derives with the first archive's own frame, so
/// its walked extent spans the whole file and the
/// first end record stands in it; where its plant
/// content matched the real key entry's length, its
/// declared offset set equalled the real directory's
/// and the record-facts arm read the real candidate
/// as a rewriting in its extent. But a candidate
/// whose body stands again after the filled
/// candidate's comment end, before its own directory
/// (`trailing_body_restands`), is a trailing archive,
/// not a gap owner: the first complete directory is
/// the directory judged, and the trailing archive is
/// subordinated the way a gap plant is.
pub fn central_directory(bytes: &[u8]) -> Option<CentralDirectory> {
    judged_directory(bytes).map(|(_, directory)| directory)
}

/// The discipline above, returning the standing
/// candidate with the position of the end record it
/// was derived under: `prefix_free` lays the judged
/// archive's tail out from that position.
fn judged_directory(bytes: &[u8]) -> Option<(usize, CentralDirectory)> {
    // Every candidate that derives, in file order,
    // with the span its declared comment covers:
    // (end record position, comment start, comment
    // end, the directory it derives).
    let mut derived: Vec<(usize, usize, usize, CentralDirectory)> = Vec::new();
    for eocd in end_record_candidates(bytes) {
        let Some(directory) = arithmetic_directory(bytes, eocd)
            .or_else(|| hunted_directory(bytes, eocd))
            .or_else(|| tail_gap_directory(bytes, eocd))
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
    let walked_spans: Vec<Vec<(usize, usize)>> = derived
        .iter()
        .map(|(_, _, _, directory)| directory_walked_spans(bytes, directory))
        .collect();
    // The walked extent run through the candidate's
    // own end record (round 40, F3): the tail gaps and
    // the zip64 end structures standing between the
    // walk's end and that end record are the
    // candidate's bytes the way its gaps before the
    // directory are, so a candidate planted anywhere
    // in the tail stands inside the extent — judged,
    // as every fill is, by the accounting guard below.
    let walked_extents: Vec<Option<(usize, usize)>> = derived
        .iter()
        .map(|(eocd, _, _, directory)| {
            directory_walked_extent(bytes, directory)
                .filter(|(start, _)| *start <= *eocd)
                .map(|(start, _)| (start, *eocd))
        })
        .collect();
    let local_offsets: Vec<Vec<u64>> = derived
        .iter()
        .map(|(_, _, _, directory)| directory_local_offsets(bytes, directory))
        .collect();
    let record_facts: Vec<Vec<RecordFact>> = derived
        .iter()
        .map(|(_, _, _, directory)| directory_record_facts(bytes, directory))
        .collect();
    for (i, (at, _, comment_end, directory)) in derived.iter().enumerate() {
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
                    || walked_spans[j]
                        .iter()
                        .any(|(start, end)| at >= start && at < end)
                    || zip64_spans[j].is_some_and(|(start, end)| *at >= start && *at < end)
                    // The walked extent's gap fill
                    // (round 39, F1, extended round 40,
                    // F3 + F4): this candidate's end
                    // record stands between the other
                    // candidate's archive start and its
                    // own end record — the extent runs
                    // through the tail now, so the post-
                    // walk gaps and the zip64 end
                    // structures are filled the way the
                    // gaps before the directory are —
                    // in a region no accounted span
                    // covers, and the other candidate's
                    // walk accounts for this candidate
                    // as `gap_fill_subordinates` states:
                    // every local header this candidate
                    // accounts for, and either at least
                    // one more (the strict-superset
                    // guard that keeps an appended
                    // second directory from swallowing
                    // the real one — it names only the
                    // first copies' headers, so the
                    // directory that accounts for them
                    // all is not inside its fill) or a
                    // record-level disagreement in any
                    // judged fact (round 40, F4, completed
                    // round 41, F1: the facts are the
                    // whole records, compared as
                    // sequences in walk order — a plant
                    // whose declared offset set *equals*
                    // the real directory's, renaming an
                    // entry, inventing one at a named
                    // offset, truncating a declared size,
                    // reordering the walk, or rewriting
                    // a field beyond the name and the
                    // spot, is the extent owner's bytes,
                    // not its directory). A second
                    // directory that mirrors the real
                    // one's records exactly — same
                    // offsets, the same whole records in
                    // the same order (an appended or
                    // comment-planted copy over a body
                    // with no duplicates) — is
                    // subordinated by neither arm: it
                    // judges identically to the real one
                    // either way, and its fill swallows
                    // nothing. The fill does not reach
                    // back across a complete archive's
                    // end either (round 43, F6): where
                    // the other candidate's body stands
                    // again after this candidate's
                    // comment end, before the other
                    // candidate's directory
                    // (`trailing_body_restands`), the
                    // other candidate is a trailing
                    // archive appended after this
                    // complete one — its extent spans
                    // this archive the way a
                    // concatenation's does, not the way
                    // a gap owner's accounts for a
                    // plant — and this candidate stands
                    // on its own: the first complete
                    // directory is the directory judged.
                    || (walked_extents[j]
                        .is_some_and(|(start, end)| *at >= start && *at < end)
                        && !trailing_body_restands(
                            bytes,
                            *comment_end,
                            &derived[j].3,
                            &local_offsets[j],
                            &record_facts[j],
                        )
                        && gap_fill_subordinates(
                            &local_offsets[i],
                            &record_facts[i],
                            &local_offsets[j],
                            &record_facts[j],
                        )))
        });
        if in_structural {
            continue;
        }
        return Some((*at, *directory));
    }
    None
}

/// The judged archive's tail layout (round 40, F3):
/// where its directory walk's accounted end stands,
/// and — for a zip64 archive — where its zip64 record
/// (with any extensible data sector, `record_len`
/// long in all) and its locator stand, validated
/// against the judged directory the way the tail-gap
/// derivation validates them. None when the layout
/// cannot be stated (a judged directory whose walk
/// does not complete, or whose zip64 end structures
/// do not stand in its tail): the view then keeps the
/// archive's bytes as they stand.
enum TailLayout {
    Plain {
        extent_end: usize,
    },
    Zip64 {
        extent_end: usize,
        record_at: usize,
        record_len: usize,
        locator_at: usize,
    },
}

fn tail_layout(bytes: &[u8], eocd: usize, directory: &CentralDirectory) -> Option<TailLayout> {
    let (walk_end, extent_end) =
        directory_walk_and_signature_end(bytes, directory.cd_start, directory.total)?;
    let total_plain = u64::from(u16_at(bytes, eocd + 10));
    let cd_size_plain = u64::from(u32_at(bytes, eocd + 12));
    let cd_offset_plain = u64::from(u32_at(bytes, eocd + 16));
    let zip64_needed = total_plain == u64::from(u16::MAX)
        || cd_size_plain == u64::from(u32::MAX)
        || cd_offset_plain == u64::from(u32::MAX);
    if !zip64_needed {
        return Some(TailLayout::Plain { extent_end });
    }
    // The locator nearest the end record whose named
    // record validates against the judged directory.
    if eocd < 20 {
        return None;
    }
    let mut locator_at = eocd - 20;
    loop {
        if bytes[locator_at..].starts_with(b"PK\x06\x07") {
            let declared_record = u64_at(bytes, locator_at + 8);
            if locator_at >= 56 {
                let mut record_at = locator_at - 56;
                loop {
                    if bytes[record_at..].starts_with(b"PK\x06\x06") {
                        if let Some(record_len) = zip64_record_layout_at(
                            bytes,
                            record_at,
                            locator_at,
                            declared_record,
                            directory,
                            walk_end,
                            extent_end,
                        ) {
                            return Some(TailLayout::Zip64 {
                                extent_end,
                                record_at,
                                record_len,
                                locator_at,
                            });
                        }
                    }
                    if record_at == 0 {
                        break;
                    }
                    record_at -= 1;
                }
            }
        }
        if locator_at == 0 {
            return None;
        }
        locator_at -= 1;
    }
}

/// The declared length of the zip64 record at
/// `record_at` when it is the judged directory's own
/// record: the locator at `locator_at` declares it at
/// `declared_record`, its disk fields agree, its
/// version fields are consistent the way the zip
/// reader judges them (round 41, F2: the reader
/// refuses a zip64 record whose version-needed
/// exceeds its version-made-by, so a record carrying
/// that inconsistency is not a pair the splice may
/// lay into the view — a fake pair standing in the
/// locator-to-end-record gap, field-identical to the
/// real pair except a version-needed of 99 against a
/// version-made-by of 45, was the pair the splice
/// chose, and the reader then refused the good
/// archive at every door), its fields name the judged
/// directory (its total, and its directory offset
/// read through the judged archive offset landing on
/// the judged directory's start, its directory size
/// the walk's length, counted signature record
/// included or not), and its declared length ends at
/// or before the locator.
fn zip64_record_layout_at(
    bytes: &[u8],
    record_at: usize,
    locator_at: usize,
    declared_record: u64,
    directory: &CentralDirectory,
    walk_end: usize,
    extent_end: usize,
) -> Option<usize> {
    let fixed_end = record_at.checked_add(56)?;
    if fixed_end > bytes.len() {
        return None;
    }
    let size_field = u64_at(bytes, record_at + 4);
    if size_field < 44 {
        return None;
    }
    let record_len = usize::try_from(size_field.checked_add(12)?).ok()?;
    if record_at.checked_add(record_len)? > locator_at {
        return None;
    }
    if !zip64_disk_fields_agree(bytes, record_at, locator_at) {
        return None;
    }
    if u16_at(bytes, record_at + 14) > u16_at(bytes, record_at + 12) {
        return None;
    }
    let archive_offset = u64::try_from(record_at)
        .ok()?
        .checked_sub(declared_record)?;
    if archive_offset != directory.archive_offset {
        return None;
    }
    if u64_at(bytes, record_at + 32) != directory.total {
        return None;
    }
    let cd_size = u64_at(bytes, record_at + 40);
    let cd_offset = u64_at(bytes, record_at + 48);
    if cd_offset.checked_add(archive_offset)? != u64::try_from(directory.cd_start).ok()? {
        return None;
    }
    if cd_size != u64::try_from(walk_end - directory.cd_start).ok()?
        && cd_size != u64::try_from(extent_end - directory.cd_start).ok()?
    {
        return None;
    }
    if extent_end > record_at {
        return None;
    }
    Some(record_len)
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
///
/// Round 40, F3: the view also splices the judged
/// archive's tail gaps out — the bytes from the
/// directory walk's accounted end through its own end
/// record are laid out contiguously (the zip64
/// record and locator, where they stand, moved up
/// against the walk's end and each other, the
/// locator's declared record offset rewritten to the
/// record's position in the view; the end record and
/// its comment follow). Every position a central
/// record names stands before the walk's end, so no
/// judged fact moves; what the splice removes is the
/// unaccounted tail bytes — a plant standing in them
/// included — and what it restores is the reader at
/// the two locator-side zip64 gaps, whose clean hosts
/// it refuses on the unspliced bytes (its zip64
/// checks are contiguity checks) while a plant there
/// used to make it open the planted directory
/// instead. A tail already contiguous is never
/// spliced: the view is the archive's own bytes, cut
/// at its start as always.
pub fn prefix_free(bytes: &[u8]) -> Cow<'_, [u8]> {
    let Some((eocd, directory)) = judged_directory(bytes) else {
        return Cow::Borrowed(bytes);
    };
    let Ok(start) = usize::try_from(directory.archive_offset) else {
        return Cow::Borrowed(bytes);
    };
    if start > bytes.len() {
        return Cow::Borrowed(bytes);
    }
    let contiguous_view = |start: usize| {
        if start == 0 {
            Cow::Borrowed(bytes)
        } else {
            Cow::Owned(bytes[start..].to_vec())
        }
    };
    let Some(layout) = tail_layout(bytes, eocd, &directory) else {
        return contiguous_view(start);
    };
    let comment_end = eocd + 22 + usize::from(u16_at(bytes, eocd + 20));
    match layout {
        TailLayout::Plain { extent_end } => {
            if extent_end == eocd {
                return contiguous_view(start);
            }
            let mut view = bytes[start..extent_end].to_vec();
            view.extend_from_slice(&bytes[eocd..comment_end]);
            Cow::Owned(view)
        }
        TailLayout::Zip64 {
            extent_end,
            record_at,
            record_len,
            locator_at,
        } => {
            if extent_end == record_at
                && record_at + record_len == locator_at
                && locator_at + 20 == eocd
            {
                return contiguous_view(start);
            }
            let mut view = bytes[start..extent_end].to_vec();
            view.extend_from_slice(&bytes[record_at..record_at + record_len]);
            let mut locator = bytes[locator_at..locator_at + 20].to_vec();
            let record_in_view = u64::try_from(extent_end - start).unwrap_or(u64::MAX);
            locator[8..16].copy_from_slice(&record_in_view.to_le_bytes());
            view.extend_from_slice(&locator);
            view.extend_from_slice(&bytes[eocd..comment_end]);
            Cow::Owned(view)
        }
    }
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

    /// The round-38 walked-extent crafts (red team
    /// RT38-1), assembled byte by byte over the same
    /// Stored entries as the round-37 crafter: per
    /// `shape` — "plain" nothing more; "n1" a complete
    /// fake directory (the first copies' records plus
    /// its own end record) planted inside the filler
    /// entry's local-header extra field (4,000 bytes
    /// the real local header declares); "n2b" that
    /// directory planted contiguously inside the
    /// filler record's central extra field; "n3b" the
    /// same plant inside its central comment field;
    /// "n4" the same plant inside a digital-signature
    /// record's data, the record standing between the
    /// real directory and its end record in the
    /// standing allowance's shape (the declared
    /// directory size counts the central records only);
    /// "n2split" the split form — the fake directory
    /// in the filler's payload, its end record moved
    /// into the filler record's central extra field;
    /// "n5" the real archive standing behind a chain
    /// of two tiny prefix archives, the first declaring
    /// a 400-byte comment reaching into what follows.
    /// Returns the bytes, the real directory's start,
    /// and the real archive offset.
    fn walked_plant_archive(shape: &str) -> (Vec<u8>, usize, u64) {
        let entries: [(&[u8], &[u8]); 3] = [
            (b"a.txt", b"first copy"),
            (b"a.txt", b"second copy!"),
            (b"filler.bin", &[b'A'; 600]),
        ];
        let central =
            |name: &[u8], data: &[u8], local_offset: u32, extra: &[u8], comment: &[u8]| {
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
                rec.extend_from_slice(&(extra.len() as u16).to_le_bytes());
                rec.extend_from_slice(&(comment.len() as u16).to_le_bytes());
                rec.extend_from_slice(&0u16.to_le_bytes());
                rec.extend_from_slice(&0u16.to_le_bytes());
                rec.extend_from_slice(&0u32.to_le_bytes());
                rec.extend_from_slice(&local_offset.to_le_bytes());
                rec.extend_from_slice(name);
                rec.extend_from_slice(extra);
                rec.extend_from_slice(comment);
                rec
            };
        let mut body: Vec<u8> = Vec::new();
        let mut locals: Vec<u32> = Vec::new();
        let mut filler_extra_start = 0usize;
        for (i, (name, data)) in entries.iter().enumerate() {
            locals.push(body.len() as u32);
            let local_extra: usize = if shape == "n1" && i + 1 == entries.len() {
                4000
            } else {
                0
            };
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
            body.extend_from_slice(&(local_extra as u16).to_le_bytes());
            body.extend_from_slice(name);
            if local_extra > 0 {
                filler_extra_start = body.len();
                body.extend_from_slice(&vec![b'E'; local_extra]);
            }
            body.extend_from_slice(data);
        }
        let mut fake_recs: Vec<u8> = Vec::new();
        let mut fake_total: u16 = 0;
        for (i, (name, data)) in entries.iter().enumerate() {
            if entries[..i].iter().any(|(other, _)| other == name) {
                continue;
            }
            fake_recs.extend_from_slice(&central(name, data, locals[i], &[], &[]));
            fake_total += 1;
        }
        let fake_eocd =
            |cd_pos: usize| plain_eocd(fake_total, fake_recs.len() as u32, cd_pos as u32, 0, &[]);
        if shape == "n1" {
            let fake_cd_pos = filler_extra_start + 100;
            body[fake_cd_pos..fake_cd_pos + fake_recs.len()].copy_from_slice(&fake_recs);
            let planted_eocd = fake_eocd(fake_cd_pos);
            let eocd_at = fake_cd_pos + fake_recs.len();
            body[eocd_at..eocd_at + 22].copy_from_slice(&planted_eocd);
        }
        if shape == "n2split" {
            let payload_at = locals[2] as usize + 30 + entries[2].0.len();
            let fake_cd_pos = payload_at + 100;
            body[fake_cd_pos..fake_cd_pos + fake_recs.len()].copy_from_slice(&fake_recs);
        }
        let cd_offset = body.len();
        let mut centrals: Vec<u8> = Vec::new();
        for (i, (name, data)) in entries.iter().enumerate() {
            let last = i + 1 == entries.len();
            let (extra, comment): (Vec<u8>, Vec<u8>) = if last && shape == "n2b" {
                let fake_cd_pos = cd_offset + centrals.len() + 46 + name.len() + 4 + 50;
                let mut blob = vec![b'Z'; fake_recs.len() + 400];
                blob[50..50 + fake_recs.len()].copy_from_slice(&fake_recs);
                let planted_eocd = fake_eocd(fake_cd_pos);
                blob[50 + fake_recs.len()..50 + fake_recs.len() + 22]
                    .copy_from_slice(&planted_eocd);
                let mut extra = Vec::new();
                extra.extend_from_slice(&0x9999u16.to_le_bytes());
                extra.extend_from_slice(&(blob.len() as u16).to_le_bytes());
                extra.extend_from_slice(&blob);
                (extra, Vec::new())
            } else if last && shape == "n3b" {
                let fake_cd_pos = cd_offset + centrals.len() + 46 + name.len() + 50;
                let mut blob = vec![b'Z'; fake_recs.len() + 400];
                blob[50..50 + fake_recs.len()].copy_from_slice(&fake_recs);
                let planted_eocd = fake_eocd(fake_cd_pos);
                blob[50 + fake_recs.len()..50 + fake_recs.len() + 22]
                    .copy_from_slice(&planted_eocd);
                (Vec::new(), blob)
            } else if last && shape == "n2split" {
                let payload_at = locals[2] as usize + 30 + entries[2].0.len();
                let fake_cd_pos = payload_at + 100;
                let mut blob = vec![b'X'; 3000];
                let planted_eocd = fake_eocd(fake_cd_pos);
                blob[100..122].copy_from_slice(&planted_eocd);
                let mut extra = Vec::new();
                extra.extend_from_slice(&0x9999u16.to_le_bytes());
                extra.extend_from_slice(&(blob.len() as u16).to_le_bytes());
                extra.extend_from_slice(&blob);
                (extra, Vec::new())
            } else {
                (Vec::new(), Vec::new())
            };
            centrals.extend_from_slice(&central(name, data, locals[i], &extra, &comment));
        }
        match shape {
            "n4" => {
                let mut sig_data = vec![b'S'; 4000];
                let fake_cd_pos = cd_offset + centrals.len() + 6 + 100;
                sig_data[100..100 + fake_recs.len()].copy_from_slice(&fake_recs);
                let planted_eocd = fake_eocd(fake_cd_pos);
                sig_data[100 + fake_recs.len()..100 + fake_recs.len() + 22]
                    .copy_from_slice(&planted_eocd);
                let mut out = body;
                out.extend_from_slice(&centrals);
                out.extend_from_slice(b"PK\x05\x05");
                out.extend_from_slice(&(sig_data.len() as u16).to_le_bytes());
                out.extend_from_slice(&sig_data);
                out.extend_from_slice(&plain_eocd(
                    entries.len() as u16,
                    centrals.len() as u32,
                    cd_offset as u32,
                    0,
                    &[],
                ));
                (out, cd_offset, 0)
            }
            "n5" => {
                let mut real = body;
                real.extend_from_slice(&centrals);
                real.extend_from_slice(&plain_eocd(
                    entries.len() as u16,
                    centrals.len() as u32,
                    cd_offset as u32,
                    0,
                    &[],
                ));
                let tiny_archive = |comment_len: u16| {
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
                    let tiny_central = central(tiny_name, tiny_data, 0, &[], &[]);
                    tiny.extend_from_slice(&tiny_central);
                    tiny.extend_from_slice(b"PK\x05\x06");
                    tiny.extend_from_slice(&0u16.to_le_bytes());
                    tiny.extend_from_slice(&0u16.to_le_bytes());
                    tiny.extend_from_slice(&1u16.to_le_bytes());
                    tiny.extend_from_slice(&1u16.to_le_bytes());
                    tiny.extend_from_slice(&(tiny_central.len() as u32).to_le_bytes());
                    tiny.extend_from_slice(&(tiny_cd_offset as u32).to_le_bytes());
                    tiny.extend_from_slice(&comment_len.to_le_bytes());
                    tiny
                };
                let mut out = tiny_archive(400);
                out.extend_from_slice(&tiny_archive(0));
                let archive_offset = out.len() as u64;
                let real_cd_start = out.len() + cd_offset;
                out.extend_from_slice(&real);
                (out, real_cd_start, archive_offset)
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
    fn a_directory_planted_in_a_local_extra_a_central_extra_a_central_comment_or_a_signature_record_never_steers(
    ) {
        // Round 38, F1 (red team RT38-1): round 37's
        // structural spans covered a candidate's
        // declared comment, its entries' payloads
        // (compressed data only), and its zip64 end
        // structures. Four byte regions no span covered
        // remained — a local header's extra field, a
        // central record's extra field, a central
        // record's comment field, and a signature
        // record's data — and a complete fake directory
        // planted contiguously in any of them derived,
        // stood earliest, and was judged while the
        // reader served the real archive. Standing is
        // judged against the whole walked extent now:
        // the real directory is judged in every shape,
        // the plain control included, the second copy
        // under its own record.
        for shape in ["plain", "n1", "n2b", "n3b", "n4"] {
            let (bytes, cd_start, archive_offset) = walked_plant_archive(shape);
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
        }
    }

    #[test]
    fn split_plants_derive_nothing_and_the_two_prefix_chain_absorbs_whole() {
        // The held lines beside F1, stated on the same
        // crafts: a fake directory whose end record
        // stands away from its records (the directory
        // in a payload, the end record in a central
        // extra field) derives nothing at all — the
        // hunt's frame anchor forces the directory to
        // stand immediately before its end record — so
        // the real directory is judged; and a chain of
        // two tiny prefix archives before the real one
        // is absorbed whole, the first archive's
        // 400-byte declared comment reaching into what
        // follows notwithstanding, so the real directory
        // is judged at its own offset.
        for shape in ["n2split", "n5"] {
            let (bytes, cd_start, archive_offset) = walked_plant_archive(shape);
            let directory = central_directory(&bytes).expect("the real directory derives");
            assert_eq!(directory.archive_offset, archive_offset, "{shape}");
            assert_eq!(directory.total, 3, "{shape}");
            assert_eq!(directory.cd_start, cd_start, "{shape}");
            let spots = central_spots(&bytes);
            let names: Vec<&str> = spots.iter().map(|(name, _)| name.as_str()).collect();
            assert_eq!(names, ["a.txt", "a.txt", "filler.bin"], "{shape}");
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

    /// The round-39 walked-extent crafts (red team
    /// RT39-1, review R39-R2), assembled byte by byte
    /// over Stored entries a.txt in two disagreeing
    /// copies and a filler: per `shape` — "gap" a
    /// 5,000-byte unused gap between the last entry's
    /// payload and the real directory, a complete fake
    /// directory (the first copies' records plus its
    /// own end record) planted at the gap's head;
    /// "gapclean" the gap with no plant; "prefirst" a
    /// 3,000-byte region before the first local header
    /// carrying the plant; "broken" the filler gaining
    /// a 4,000-byte local extra carrying the plant
    /// while its central record declares a compressed
    /// size of 0xFFFFFFF0, so its payload span does not
    /// resolve. The zip64 twins of "gap" and "broken"
    /// ride in the F2 crafter's host table. Returns
    /// the bytes and the real directory's start.
    fn r39a_gap_archive(shape: &str) -> (Vec<u8>, usize) {
        let entries: [(&[u8], &[u8]); 3] = [
            (b"a.txt", b"first copy"),
            (b"a.txt", b"second copy!"),
            (b"filler.bin", &[b'A'; 600]),
        ];
        let central = |name: &[u8], data: &[u8], local_offset: u32, comp: u32| {
            let mut rec: Vec<u8> = Vec::new();
            rec.extend_from_slice(b"PK\x01\x02");
            rec.extend_from_slice(&45u16.to_le_bytes());
            rec.extend_from_slice(&20u16.to_le_bytes());
            rec.extend_from_slice(&0u16.to_le_bytes());
            rec.extend_from_slice(&0u16.to_le_bytes());
            rec.extend_from_slice(&0u16.to_le_bytes());
            rec.extend_from_slice(&0u16.to_le_bytes());
            rec.extend_from_slice(&0u32.to_le_bytes());
            rec.extend_from_slice(&comp.to_le_bytes());
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
        let prefix: usize = if shape == "prefirst" { 3000 } else { 0 };
        let gap: usize = if matches!(shape, "gap" | "gapclean") {
            5000
        } else {
            0
        };
        let broken = shape == "broken";
        let mut body: Vec<u8> = vec![b'P'; prefix];
        let mut locals: Vec<u32> = Vec::new();
        let mut filler_extra_start = 0usize;
        for (i, (name, data)) in entries.iter().enumerate() {
            locals.push(body.len() as u32);
            let local_extra: usize = if broken && i + 1 == entries.len() {
                4000
            } else {
                0
            };
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
            body.extend_from_slice(&(local_extra as u16).to_le_bytes());
            body.extend_from_slice(name);
            if local_extra > 0 {
                filler_extra_start = body.len();
                body.extend_from_slice(&vec![b'E'; local_extra]);
            }
            body.extend_from_slice(data);
        }
        let mut fake_recs: Vec<u8> = Vec::new();
        let mut fake_total: u16 = 0;
        for (i, (name, data)) in entries.iter().enumerate() {
            if entries[..i].iter().any(|(other, _)| other == name) {
                continue;
            }
            fake_recs.extend_from_slice(&central(name, data, locals[i], data.len() as u32));
            fake_total += 1;
        }
        let fake_len = fake_recs.len();
        let plant = |host: &mut Vec<u8>, at: usize| {
            host[at..at + fake_len].copy_from_slice(&fake_recs);
            let eocd = plain_eocd(fake_total, fake_len as u32, at as u32, 0, &[]);
            host[at + fake_len..at + fake_len + 22].copy_from_slice(&eocd);
        };
        if shape == "prefirst" {
            plant(&mut body, 100);
        }
        if broken {
            plant(&mut body, filler_extra_start + 100);
        }
        if gap > 0 {
            let gap_start = body.len();
            body.extend_from_slice(&vec![b'G'; gap]);
            if shape == "gap" {
                plant(&mut body, gap_start + 100);
            }
        }
        let cd_start = body.len();
        let mut centrals: Vec<u8> = Vec::new();
        for (i, (name, data)) in entries.iter().enumerate() {
            let comp = if broken && i + 1 == entries.len() {
                0xFFFF_FFF0
            } else {
                data.len() as u32
            };
            centrals.extend_from_slice(&central(name, data, locals[i], comp));
        }
        let mut out = body;
        out.extend_from_slice(&centrals);
        out.extend_from_slice(&plain_eocd(
            entries.len() as u16,
            centrals.len() as u32,
            cd_start as u32,
            0,
            &[],
        ));
        (out, cd_start)
    }

    #[test]
    fn a_directory_planted_in_the_gap_before_the_directory_before_the_first_header_or_in_a_spanless_entrys_header_never_steers(
    ) {
        // Round 39, F1 (red team RT39-1 + review
        // R39-R2): three byte regions stood outside
        // the walked extent — the unused gap between
        // the last entry's payload and the central
        // directory, the region between the archive's
        // own start and its first local header, and
        // the local header of an entry whose payload
        // span does not resolve (its span was pushed
        // only inside the payload span's success, so a
        // filler whose central record declares a
        // compressed size overrunning the file
        // accounted for no bytes at all, its header
        // included). A complete fake directory planted
        // in any of them derived, stood earliest, and
        // was judged. The real directory is judged in
        // every shape now, the gap-clean control
        // included: every entry stands in it, the
        // second copy under its own record.
        for shape in ["gap", "gapclean", "prefirst", "broken"] {
            let (bytes, cd_start) = r39a_gap_archive(shape);
            let directory = central_directory(&bytes).expect("the real directory derives");
            assert_eq!(directory.archive_offset, 0, "{shape}");
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
        }
    }

    /// The round-39 zip64 signature crafts (red team
    /// RT39-2), assembled byte by byte over the same
    /// Stored entries as the gap crafter: a zip64 host
    /// whose digital-signature record (4,000 data
    /// bytes) stands between the real directory and
    /// the zip64 record. Per `shape` — "clean" the
    /// signature record uncounted in the zip64
    /// record's declared directory size and no plant;
    /// "plant" a complete fake directory planted
    /// inside the signature record's data, uncounted;
    /// "counted" the plant with the signature record
    /// counted in the declared size; "sectorclean" /
    /// "sectorplant" the uncounted shapes on a host
    /// whose zip64 record carries a 2,048-byte
    /// extensible data sector, so the validating hunt
    /// is the only derivation; "gap64" a fake
    /// directory planted in a 5,000-byte gap before
    /// the directory of a zip64 host; "broken64" the
    /// gap crafter's broken filler on a zip64 host.
    /// Returns the bytes and the real directory's
    /// start.
    fn r39a_zip64_sig_archive(shape: &str) -> (Vec<u8>, usize) {
        if matches!(shape, "gap64" | "broken64") {
            // The plain gap crafts rebuilt on a zip64
            // host: the same body (the gap plant rides
            // in the rebuilt body's gap; the broken
            // filler's plant rides in its local extra),
            // then the zip64 end structures in place of
            // the plain end record.
            let plain_shape = if shape == "gap64" { "gap" } else { "broken" };
            let (plain, cd_start) = r39a_gap_archive(plain_shape);
            let plain_eocd = plain.len() - 22;
            let mut out = plain[..plain_eocd].to_vec();
            let centrals_len = plain_eocd - cd_start;
            let record_at = out.len() as u64;
            out.extend_from_slice(b"PK\x06\x06");
            out.extend_from_slice(&44u64.to_le_bytes());
            out.extend_from_slice(&45u16.to_le_bytes());
            out.extend_from_slice(&45u16.to_le_bytes());
            out.extend_from_slice(&0u32.to_le_bytes());
            out.extend_from_slice(&0u32.to_le_bytes());
            out.extend_from_slice(&3u64.to_le_bytes());
            out.extend_from_slice(&3u64.to_le_bytes());
            out.extend_from_slice(&(centrals_len as u64).to_le_bytes());
            out.extend_from_slice(&(cd_start as u64).to_le_bytes());
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
            return (out, cd_start);
        }
        let entries: [(&[u8], &[u8]); 3] = [
            (b"a.txt", b"first copy"),
            (b"a.txt", b"second copy!"),
            (b"filler.bin", &[b'A'; 600]),
        ];
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
        let sector: usize = if shape.starts_with("sector") { 2048 } else { 0 };
        let mut body: Vec<u8> = Vec::new();
        let mut locals: Vec<u32> = Vec::new();
        for (name, data) in &entries {
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
        let cd_start = body.len();
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
        let mut sig_data = vec![b'S'; 4000];
        if matches!(shape, "plant" | "counted" | "sectorplant") {
            let fake_at = cd_start + centrals.len() + 6 + 100;
            sig_data[100..100 + fake_recs.len()].copy_from_slice(&fake_recs);
            let eocd = plain_eocd(fake_total, fake_recs.len() as u32, fake_at as u32, 0, &[]);
            let eocd_at = 100 + fake_recs.len();
            sig_data[eocd_at..eocd_at + 22].copy_from_slice(&eocd);
        }
        let cd_size = centrals.len() as u64 + if shape == "counted" { 4006 } else { 0 };
        let mut out = body;
        out.extend_from_slice(&centrals);
        out.extend_from_slice(b"PK\x05\x05");
        out.extend_from_slice(&(sig_data.len() as u16).to_le_bytes());
        out.extend_from_slice(&sig_data);
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
        out.extend_from_slice(&(cd_start as u64).to_le_bytes());
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
        (out, cd_start)
    }

    #[test]
    fn a_zip64_archive_whose_signature_record_is_uncounted_derives_and_a_directory_planted_in_it_never_steers(
    ) {
        // Round 39, F2 (red team RT39-2): on a plain
        // host a signature record standing after the
        // directory is handled — the hunt's frame
        // anchor carries the signature allowance — but
        // on a zip64 host whose signature record's
        // bytes are NOT counted in the declared
        // directory size, both derivations demanded
        // the directory end equal the zip64 record's
        // position exactly, so the clean craft derived
        // nothing at all while the reader served it,
        // and a fake directory planted inside the
        // signature record was the only standing
        // candidate and was judged. A zip64 candidate
        // failing validation only by that signature
        // record derives under the signature allowance
        // now — the record's own declared length
        // closes the gap, counted or not — on the
        // arithmetic path and the hunt alike: the real
        // directory is judged in every shape, the
        // counted variant and the F1 zip64 twins (a
        // plant in the gap, a plant in a spanless
        // entry's header) included, the second copy
        // under its own record. The plain-host
        // signature plant stands subordinated as the
        // round-38 pin above states (its "n4" shape).
        for shape in [
            "clean",
            "plant",
            "counted",
            "sectorclean",
            "sectorplant",
            "gap64",
            "broken64",
        ] {
            let (bytes, cd_start) = r39a_zip64_sig_archive(shape);
            let directory = central_directory(&bytes).expect("the real directory derives");
            assert_eq!(directory.archive_offset, 0, "{shape}");
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
        }
    }

    /// One Stored local header plus payload, in the
    /// round-40 crafters' shape.
    fn r40a_local(name: &[u8], data: &[u8]) -> Vec<u8> {
        let mut out: Vec<u8> = Vec::new();
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
        out
    }

    /// One central record in the round-40 crafters'
    /// shape: Stored, no extra, no comment, the
    /// declared compressed size stated outright (the
    /// trunc craft declares a size the entry does not
    /// carry).
    fn r40a_central(name: &[u8], local_offset: u32, comp: u32, uncomp: u32) -> Vec<u8> {
        let mut rec: Vec<u8> = Vec::new();
        rec.extend_from_slice(b"PK\x01\x02");
        rec.extend_from_slice(&45u16.to_le_bytes());
        rec.extend_from_slice(&20u16.to_le_bytes());
        rec.extend_from_slice(&0u16.to_le_bytes());
        rec.extend_from_slice(&0u16.to_le_bytes());
        rec.extend_from_slice(&0u16.to_le_bytes());
        rec.extend_from_slice(&0u16.to_le_bytes());
        rec.extend_from_slice(&0u32.to_le_bytes());
        rec.extend_from_slice(&comp.to_le_bytes());
        rec.extend_from_slice(&uncomp.to_le_bytes());
        rec.extend_from_slice(&(name.len() as u16).to_le_bytes());
        rec.extend_from_slice(&0u16.to_le_bytes());
        rec.extend_from_slice(&0u16.to_le_bytes());
        rec.extend_from_slice(&0u16.to_le_bytes());
        rec.extend_from_slice(&0u16.to_le_bytes());
        rec.extend_from_slice(&0u32.to_le_bytes());
        rec.extend_from_slice(&local_offset.to_le_bytes());
        rec.extend_from_slice(name);
        rec
    }

    /// The round-40 tail crafts (red team RT40-A),
    /// assembled byte by byte over Stored entries
    /// a.txt in two disagreeing copies and a filler:
    /// a 5,000-byte gap standing in one of the five
    /// regions after the directory walk — between the
    /// central directory and the end record
    /// ("postcd"), after a signature record standing
    /// at the walk's end ("sig"), or inside a zip64
    /// tail, between the directory and the zip64
    /// record ("z64gapcd"), between the zip64 record
    /// and its locator ("z64gaprec"), or between the
    /// locator and the end record ("z64gaploc") —
    /// with ("-plant") or without ("-clean") a
    /// complete fake directory (the first copies'
    /// records plus its own end record) planted at
    /// the gap's head. Returns the bytes and the real
    /// directory's start.
    fn r40a_tail_archive(shape: &str) -> (Vec<u8>, usize) {
        let entries: [(&[u8], &[u8]); 3] = [
            (b"a.txt", b"first copy"),
            (b"a.txt", b"second copy!"),
            (b"filler.bin", &[b'A'; 600]),
        ];
        let mut body: Vec<u8> = Vec::new();
        let mut locals: Vec<u32> = Vec::new();
        for (name, data) in entries {
            locals.push(body.len() as u32);
            body.extend_from_slice(&r40a_local(name, data));
        }
        let cd_start = body.len();
        let mut centrals: Vec<u8> = Vec::new();
        for (i, (name, data)) in entries.iter().enumerate() {
            centrals.extend_from_slice(&r40a_central(
                name,
                locals[i],
                data.len() as u32,
                data.len() as u32,
            ));
        }
        let cd_end = cd_start + centrals.len();
        // The fake directory: the first copy's record
        // for every name, in archive order, plus its
        // own end record.
        let mut fake_recs: Vec<u8> = Vec::new();
        let mut fake_total: u16 = 0;
        for (i, (name, data)) in entries.iter().enumerate() {
            if entries[..i].iter().any(|(other, _)| other == name) {
                continue;
            }
            fake_recs.extend_from_slice(&r40a_central(
                name,
                locals[i],
                data.len() as u32,
                data.len() as u32,
            ));
            fake_total += 1;
        }
        let plant = |host: &mut Vec<u8>, at: usize| {
            host[at..at + fake_recs.len()].copy_from_slice(&fake_recs);
            let eocd = plain_eocd(fake_total, fake_recs.len() as u32, at as u32, 0, &[]);
            host[at + fake_recs.len()..at + fake_recs.len() + 22].copy_from_slice(&eocd);
        };
        let zip64_tail = |host: &mut Vec<u8>, gap1: usize, gap2: usize, gap3: usize| {
            host.extend_from_slice(&vec![b'G'; gap1]);
            let record_at = host.len() as u64;
            host.extend_from_slice(b"PK\x06\x06");
            host.extend_from_slice(&44u64.to_le_bytes());
            host.extend_from_slice(&45u16.to_le_bytes());
            host.extend_from_slice(&45u16.to_le_bytes());
            host.extend_from_slice(&0u32.to_le_bytes());
            host.extend_from_slice(&0u32.to_le_bytes());
            host.extend_from_slice(&3u64.to_le_bytes());
            host.extend_from_slice(&3u64.to_le_bytes());
            host.extend_from_slice(&(centrals.len() as u64).to_le_bytes());
            host.extend_from_slice(&(cd_start as u64).to_le_bytes());
            host.extend_from_slice(&vec![b'G'; gap2]);
            host.extend_from_slice(b"PK\x06\x07");
            host.extend_from_slice(&0u32.to_le_bytes());
            host.extend_from_slice(&record_at.to_le_bytes());
            host.extend_from_slice(&1u32.to_le_bytes());
            host.extend_from_slice(&vec![b'G'; gap3]);
            host.extend_from_slice(b"PK\x05\x06");
            host.extend_from_slice(&0u16.to_le_bytes());
            host.extend_from_slice(&0u16.to_le_bytes());
            host.extend_from_slice(&u16::MAX.to_le_bytes());
            host.extend_from_slice(&u16::MAX.to_le_bytes());
            host.extend_from_slice(&u32::MAX.to_le_bytes());
            host.extend_from_slice(&u32::MAX.to_le_bytes());
            host.extend_from_slice(&0u16.to_le_bytes());
        };
        let mut out = body;
        out.extend_from_slice(&centrals);
        match shape {
            "postcd-clean" | "postcd-plant" => {
                out.extend_from_slice(&vec![b'G'; 5000]);
                out.extend_from_slice(&plain_eocd(
                    3,
                    centrals.len() as u32,
                    cd_start as u32,
                    0,
                    &[],
                ));
                if shape == "postcd-plant" {
                    plant(&mut out, cd_end + 100);
                }
            }
            "sig-clean" | "sig-plant" => {
                out.extend_from_slice(b"PK\x05\x05");
                out.extend_from_slice(&100u16.to_le_bytes());
                out.extend_from_slice(&[b'S'; 100]);
                out.extend_from_slice(&vec![b'G'; 5000]);
                out.extend_from_slice(&plain_eocd(
                    3,
                    centrals.len() as u32,
                    cd_start as u32,
                    0,
                    &[],
                ));
                if shape == "sig-plant" {
                    plant(&mut out, cd_end + 106 + 100);
                }
            }
            "z64gapcd-clean" | "z64gapcd-plant" => {
                zip64_tail(&mut out, 5000, 0, 0);
                if shape == "z64gapcd-plant" {
                    plant(&mut out, cd_end + 100);
                }
            }
            "z64gaprec-clean" | "z64gaprec-plant" => {
                zip64_tail(&mut out, 0, 5000, 0);
                if shape == "z64gaprec-plant" {
                    plant(&mut out, cd_end + 56 + 100);
                }
            }
            "z64gaploc-clean" | "z64gaploc-plant" => {
                zip64_tail(&mut out, 0, 0, 5000);
                if shape == "z64gaploc-plant" {
                    plant(&mut out, cd_end + 56 + 20 + 100);
                }
            }
            _ => unreachable!("unknown shape {shape}"),
        }
        (out, cd_start)
    }

    #[test]
    fn r40a_a_directory_planted_after_the_directory_walk_never_steers_and_tail_gap_hosts_derive() {
        // Round 40, F3 (red team RT40-A): everything
        // after a candidate's directory walk was
        // unaccounted, and the pattern of the earlier
        // generations repeated in the archive's tail.
        // A host whose tail carries a 5,000-byte gap
        // in any of the five post-walk regions derived
        // nothing while the reader served it (the
        // plain and CD-side zip64 gaps) or while the
        // reader served a plant standing in the gap
        // (the locator-side zip64 gaps, whose clean
        // hosts the reader refuses outright on the
        // unspliced bytes); and a fake directory
        // planted in the gap was the sole standing
        // candidate and steered every door. The real
        // directory is derived and judged in every
        // shape now, the second copy under its own
        // record, and the prefix-free view the doors
        // hand the reader derives the same directory
        // — on the locator-side zip64 shapes the view
        // is the spliced tail the reader can open, so
        // the clean hosts serve through the doors and
        // the planted hosts judge the real,
        // duplicate-bearing directory there too.
        for shape in [
            "postcd-clean",
            "postcd-plant",
            "sig-clean",
            "sig-plant",
            "z64gapcd-clean",
            "z64gapcd-plant",
            "z64gaprec-clean",
            "z64gaprec-plant",
            "z64gaploc-clean",
            "z64gaploc-plant",
        ] {
            let (bytes, cd_start) = r40a_tail_archive(shape);
            let directory = central_directory(&bytes).expect("the real directory derives");
            assert_eq!(directory.archive_offset, 0, "{shape}");
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
            let first_span = payload_span(&bytes, &spots[0].1).expect("the first payload");
            assert_eq!(&bytes[first_span.0..first_span.1], b"first copy", "{shape}");
            let second_span = payload_span(&bytes, &spots[1].1).expect("the second payload");
            assert_eq!(
                &bytes[second_span.0..second_span.1],
                b"second copy!",
                "{shape}"
            );
            let view = prefix_free(&bytes);
            let view_directory =
                central_directory(&view).expect("the prefix-free view derives its directory");
            assert_eq!(view_directory.archive_offset, 0, "{shape}");
            assert_eq!(view_directory.total, 3, "{shape}");
            assert_eq!(view_directory.cd_start, cd_start, "{shape}");
            let view_spots = central_spots(&view);
            let view_names: Vec<&str> = view_spots.iter().map(|(name, _)| name.as_str()).collect();
            assert_eq!(view_names, ["a.txt", "a.txt", "filler.bin"], "{shape}");
        }
    }

    /// The round-40 equal-set crafts (review R40-R5),
    /// assembled byte by byte: one real entry
    /// a.txt = "real data", a 400-byte gap between
    /// its payload and the real directory, and inside
    /// the gap a fake directory whose records declare
    /// exactly the real directory's local-offset set
    /// ({0}) — "rename" naming offset 0 as evil.txt,
    /// "phantom" carrying two records at offset 0,
    /// "trunc" declaring a compressed size of 4 for
    /// the 9-byte entry, "mirror" carrying an exact
    /// copy of the real record. "clean" carries no
    /// plant. The real directory stands at 444 in
    /// every shape.
    fn r40a_gap_set_archive(shape: &str) -> Vec<u8> {
        let name: &[u8] = b"a.txt";
        let data: &[u8] = b"real data";
        let mut body: Vec<u8> = Vec::new();
        body.extend_from_slice(&r40a_local(name, data));
        let gap_start = body.len();
        body.extend_from_slice(&vec![b'G'; 400]);
        let fake_cd_at = gap_start + 16;
        let fake_recs: Vec<u8> = match shape {
            "rename" => r40a_central(b"evil.txt", 0, 9, 9),
            "phantom" => {
                let mut recs = r40a_central(name, 0, 9, 9);
                recs.extend_from_slice(&r40a_central(b"evil.txt", 0, 9, 9));
                recs
            }
            "trunc" => r40a_central(name, 0, 4, 4),
            "mirror" => r40a_central(name, 0, 9, 9),
            _ => Vec::new(),
        };
        if shape != "clean" {
            let total: u16 = if shape == "phantom" { 2 } else { 1 };
            body[fake_cd_at..fake_cd_at + fake_recs.len()].copy_from_slice(&fake_recs);
            let eocd = plain_eocd(total, fake_recs.len() as u32, fake_cd_at as u32, 0, &[]);
            let eocd_at = fake_cd_at + fake_recs.len();
            body[eocd_at..eocd_at + 22].copy_from_slice(&eocd);
        }
        let cd_start = body.len();
        assert_eq!(cd_start, 444, "the craft's real directory stands at 444");
        let real_rec = r40a_central(name, 0, 9, 9);
        let cd_size = real_rec.len() as u32;
        body.extend_from_slice(&real_rec);
        body.extend_from_slice(&plain_eocd(1, cd_size, cd_start as u32, 0, &[]));
        body
    }

    #[test]
    fn r40a_an_equal_offset_set_plant_in_the_gap_before_the_directory_never_steers() {
        // Round 40, F4 (review R40-R5): the walked
        // extent's gap fill subordinated a plant only
        // where the filling candidate accounted for
        // every local header the plant accounted for
        // and at least one more, so a plant inside the
        // gap before the directory whose declared
        // offset set is EQUAL to the real directory's
        // escaped the guard entirely and steered the
        // scan — renamed, phantom, and truncated
        // entries judged while the reader served the
        // real archive. A candidate standing in
        // another candidate's walked extent whose
        // records differ in any judged fact from that
        // candidate's records for the same offsets is
        // the extent owner's bytes now: the real
        // directory is judged in every differing
        // shape. The exact-mirror plant carries no
        // disagreement, so it may stand — its judged
        // facts are the real directory's, stated
        // outright below either way.
        for shape in ["clean", "rename", "phantom", "trunc"] {
            let bytes = r40a_gap_set_archive(shape);
            let directory = central_directory(&bytes).expect("the real directory derives");
            assert_eq!(directory.archive_offset, 0, "{shape}");
            assert_eq!(directory.total, 1, "{shape}");
            assert_eq!(directory.cd_start, 444, "{shape}");
            let spots = central_spots(&bytes);
            assert_eq!(spots.len(), 1, "{shape}");
            assert_eq!(spots[0].0, "a.txt", "{shape}");
            assert_eq!(spots[0].1.comp_size, 9, "{shape}");
            assert_eq!(payload_span(&bytes, &spots[0].1), Some((35, 44)), "{shape}");
        }
        let bytes = r40a_gap_set_archive("mirror");
        let directory = central_directory(&bytes).expect("a directory derives");
        assert_eq!(directory.total, 1, "mirror");
        let spots = central_spots(&bytes);
        assert_eq!(spots.len(), 1, "mirror");
        assert_eq!(spots[0].0, "a.txt", "mirror");
        assert_eq!(spots[0].1.comp_size, 9, "mirror");
        assert_eq!(payload_span(&bytes, &spots[0].1), Some((35, 44)), "mirror");
    }

    /// One central record in the round-41 crafters'
    /// shape: like `r40a_central`, with the extra
    /// field stated outright (the eqextra plant
    /// carries a UT extra field the real records do
    /// not).
    fn r41a_central(
        name: &[u8],
        local_offset: u32,
        comp: u32,
        uncomp: u32,
        extra: &[u8],
    ) -> Vec<u8> {
        let mut rec: Vec<u8> = Vec::new();
        rec.extend_from_slice(b"PK\x01\x02");
        rec.extend_from_slice(&45u16.to_le_bytes());
        rec.extend_from_slice(&20u16.to_le_bytes());
        rec.extend_from_slice(&0u16.to_le_bytes());
        rec.extend_from_slice(&0u16.to_le_bytes());
        rec.extend_from_slice(&0u16.to_le_bytes());
        rec.extend_from_slice(&0u16.to_le_bytes());
        rec.extend_from_slice(&0u32.to_le_bytes());
        rec.extend_from_slice(&comp.to_le_bytes());
        rec.extend_from_slice(&uncomp.to_le_bytes());
        rec.extend_from_slice(&(name.len() as u16).to_le_bytes());
        rec.extend_from_slice(&(extra.len() as u16).to_le_bytes());
        rec.extend_from_slice(&0u16.to_le_bytes());
        rec.extend_from_slice(&0u16.to_le_bytes());
        rec.extend_from_slice(&0u16.to_le_bytes());
        rec.extend_from_slice(&0u32.to_le_bytes());
        rec.extend_from_slice(&local_offset.to_le_bytes());
        rec.extend_from_slice(name);
        rec.extend_from_slice(extra);
        rec
    }

    /// The round-41 equal-set crafts (red team
    /// RT41-A), assembled byte by byte over the
    /// caller's Stored entries: a 2,000-byte gap
    /// between the last entry's payload and the real
    /// directory, and inside the gap a fake directory
    /// (at gap head + 100) whose records declare
    /// exactly the real directory's local-offset set
    /// — one record per real entry — differing from
    /// the real records in exactly one fact the
    /// round-40 comparison never read: "eqorder" the
    /// real records in reverse walk order, "equncomp"
    /// an uncompressed size of 1 on every record,
    /// "eqextra" a UT extra field on every record,
    /// "eqversion" a version-made-by of 99 on every
    /// record. "mirror" carries byte-identical copies
    /// of the real records; "clean" carries no plant.
    /// Returns the bytes and the real directory's
    /// start.
    fn r41a_gap_archive(entries: &[(&[u8], &[u8])], shape: &str) -> (Vec<u8>, usize) {
        let mut body: Vec<u8> = Vec::new();
        let mut locals: Vec<u32> = Vec::new();
        for (name, data) in entries {
            locals.push(body.len() as u32);
            body.extend_from_slice(&r40a_local(name, data));
        }
        let plant_at = body.len() + 100;
        body.extend_from_slice(&vec![b'G'; 2000]);
        if shape != "clean" {
            let ut_extra: &[u8] = b"\x55\x54\x05\x00\x01\x00\x00\x00\x00";
            let order: Vec<usize> = if shape == "eqorder" {
                (0..entries.len()).rev().collect()
            } else {
                (0..entries.len()).collect()
            };
            let mut fake_recs: Vec<u8> = Vec::new();
            for i in order {
                let (name, data) = entries[i];
                let uncomp = if shape == "equncomp" {
                    1
                } else {
                    data.len() as u32
                };
                let extra: &[u8] = if shape == "eqextra" { ut_extra } else { &[] };
                let mut rec = r41a_central(name, locals[i], data.len() as u32, uncomp, extra);
                if shape == "eqversion" {
                    rec[4..6].copy_from_slice(&99u16.to_le_bytes());
                }
                fake_recs.extend_from_slice(&rec);
            }
            body[plant_at..plant_at + fake_recs.len()].copy_from_slice(&fake_recs);
            let eocd = plain_eocd(
                entries.len() as u16,
                fake_recs.len() as u32,
                plant_at as u32,
                0,
                &[],
            );
            let eocd_at = plant_at + fake_recs.len();
            body[eocd_at..eocd_at + 22].copy_from_slice(&eocd);
        }
        let cd_start = body.len();
        let mut centrals: Vec<u8> = Vec::new();
        for (i, (name, data)) in entries.iter().enumerate() {
            centrals.extend_from_slice(&r41a_central(
                name,
                locals[i],
                data.len() as u32,
                data.len() as u32,
                &[],
            ));
        }
        let cd_size = centrals.len() as u32;
        body.extend_from_slice(&centrals);
        body.extend_from_slice(&plain_eocd(
            entries.len() as u16,
            cd_size,
            cd_start as u32,
            0,
            &[],
        ));
        (body, cd_start)
    }

    #[test]
    fn r41a_an_equal_offset_set_plant_differing_only_in_unread_facts_never_steers() {
        // Round 41, F1 (red team RT41-A): the round-40
        // equal-set rule compared, per local offset,
        // only the decoded name and the spot (method,
        // compressed size, checksum, local offset), so
        // a plant declaring exactly the real
        // directory's offset set escaped it wherever
        // it differed only in a fact that comparison
        // never read — the walk order itself, the
        // uncompressed size, an extra field, or the
        // version-made-by — and was judged as the
        // archive's directory (at the fonts door,
        // steering the canonical name to the other
        // face). The compared facts take the whole
        // record now, in walk order: the real
        // directory is judged in every spelling, on a
        // duplicate-bearing entry set and a
        // duplicate-free one alike. The byte-identical
        // mirror plant still carries no disagreement,
        // so it may stand — its judged facts are the
        // real directory's, stated outright either way.
        let filler: &[u8] = &[b'A'; 600];
        let entry_sets: [Vec<(&[u8], &[u8])>; 2] = [
            vec![
                (b"a.txt", b"first copy"),
                (b"a.txt", b"second copy!"),
                (b"filler.bin", filler),
            ],
            vec![(b"a.txt", b"first copy"), (b"filler.bin", filler)],
        ];
        for entries in &entry_sets {
            for shape in ["clean", "eqorder", "equncomp", "eqextra", "eqversion"] {
                let (bytes, cd_start) = r41a_gap_archive(entries, shape);
                let directory = central_directory(&bytes).expect("the real directory derives");
                assert_eq!(directory.archive_offset, 0, "{shape}");
                assert_eq!(directory.total, entries.len() as u64, "{shape}");
                assert_eq!(directory.cd_start, cd_start, "{shape}");
                let spots = central_spots(&bytes);
                let names: Vec<&str> = spots.iter().map(|(name, _)| name.as_str()).collect();
                let want_names: Vec<&str> = entries
                    .iter()
                    .map(|(name, _)| std::str::from_utf8(name).expect("name"))
                    .collect();
                assert_eq!(names, want_names, "{shape}: the real walk order");
                for (i, (_, data)) in entries.iter().enumerate() {
                    assert_eq!(
                        spots[i].1.comp_size,
                        data.len() as u64,
                        "{shape}: record {i} is the real one"
                    );
                    let span = payload_span(&bytes, &spots[i].1).expect("the payload reads back");
                    assert_eq!(&bytes[span.0..span.1], *data, "{shape}: record {i}");
                }
            }
            let (clean, _) = r41a_gap_archive(entries, "clean");
            let (mirror, _) = r41a_gap_archive(entries, "mirror");
            let directory = central_directory(&mirror).expect("a directory derives");
            assert_eq!(directory.total, entries.len() as u64, "mirror");
            assert_eq!(
                central_spots(&mirror),
                central_spots(&clean),
                "mirror: the judged facts are the real directory's either way"
            );
        }
    }

    /// The round-41 fake-pair crafts (red team
    /// RT41-B), assembled byte by byte over the
    /// caller's Stored entries: a zip64 host whose
    /// locator-to-end-record gap is 5,000 bytes, with
    /// — where `fake_needed` is set — a fake zip64
    /// record+locator pair standing in that gap at
    /// offset 200, the fake record field-identical to
    /// the real one except its version-needed, which
    /// the fake locator declares at its own position.
    /// Returns the bytes, the real directory's start,
    /// and the directory walk's end.
    fn r41a_fakepair_archive(
        entries: &[(&[u8], &[u8])],
        fake_needed: Option<u16>,
    ) -> (Vec<u8>, usize, usize) {
        let mut body: Vec<u8> = Vec::new();
        let mut locals: Vec<u32> = Vec::new();
        for (name, data) in entries {
            locals.push(body.len() as u32);
            body.extend_from_slice(&r40a_local(name, data));
        }
        let cd_start = body.len();
        let mut centrals: Vec<u8> = Vec::new();
        for (i, (name, data)) in entries.iter().enumerate() {
            centrals.extend_from_slice(&r41a_central(
                name,
                locals[i],
                data.len() as u32,
                data.len() as u32,
                &[],
            ));
        }
        let cd_len = centrals.len();
        let mut out = body;
        out.extend_from_slice(&centrals);
        let record_at = out.len() as u64;
        out.extend_from_slice(b"PK\x06\x06");
        out.extend_from_slice(&44u64.to_le_bytes());
        out.extend_from_slice(&45u16.to_le_bytes());
        out.extend_from_slice(&45u16.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&(entries.len() as u64).to_le_bytes());
        out.extend_from_slice(&(entries.len() as u64).to_le_bytes());
        out.extend_from_slice(&(cd_len as u64).to_le_bytes());
        out.extend_from_slice(&(cd_start as u64).to_le_bytes());
        out.extend_from_slice(b"PK\x06\x07");
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&record_at.to_le_bytes());
        out.extend_from_slice(&1u32.to_le_bytes());
        let mut gap = vec![b'G'; 5000];
        if let Some(needed) = fake_needed {
            let fake_at = out.len() + 200;
            let mut fake_rec: Vec<u8> = Vec::new();
            fake_rec.extend_from_slice(b"PK\x06\x06");
            fake_rec.extend_from_slice(&44u64.to_le_bytes());
            fake_rec.extend_from_slice(&45u16.to_le_bytes());
            fake_rec.extend_from_slice(&needed.to_le_bytes());
            fake_rec.extend_from_slice(&0u32.to_le_bytes());
            fake_rec.extend_from_slice(&0u32.to_le_bytes());
            fake_rec.extend_from_slice(&(entries.len() as u64).to_le_bytes());
            fake_rec.extend_from_slice(&(entries.len() as u64).to_le_bytes());
            fake_rec.extend_from_slice(&(cd_len as u64).to_le_bytes());
            fake_rec.extend_from_slice(&(cd_start as u64).to_le_bytes());
            gap[200..200 + fake_rec.len()].copy_from_slice(&fake_rec);
            let fake_loc_at = 200 + fake_rec.len();
            gap[fake_loc_at..fake_loc_at + 4].copy_from_slice(b"PK\x06\x07");
            gap[fake_loc_at + 4..fake_loc_at + 8].copy_from_slice(&0u32.to_le_bytes());
            gap[fake_loc_at + 8..fake_loc_at + 16].copy_from_slice(&(fake_at as u64).to_le_bytes());
            gap[fake_loc_at + 16..fake_loc_at + 20].copy_from_slice(&1u32.to_le_bytes());
        }
        out.extend_from_slice(&gap);
        out.extend_from_slice(b"PK\x05\x06");
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&u16::MAX.to_le_bytes());
        out.extend_from_slice(&u16::MAX.to_le_bytes());
        out.extend_from_slice(&u32::MAX.to_le_bytes());
        out.extend_from_slice(&u32::MAX.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        (out, cd_start, cd_start + cd_len)
    }

    #[test]
    fn r41a_the_splice_never_lays_a_fake_zip64_pair_into_the_prefix_free_view() {
        // Round 41, F2 (red team RT41-B): the
        // prefix-free splice chose between zip64
        // record+locator pairs on fields it never
        // validated, so a fake pair standing in the
        // locator-to-end-record gap, differing from
        // the real pair only in a version-needed of
        // 99 (against a version-made-by of 45), was
        // the pair laid into the view — and the zip
        // reader, which refuses a zip64 record whose
        // version-needed exceeds its version-made-by,
        // then refused the good archive at every door.
        // The splice's pair choice validates that pair
        // consistency now: the view carries the real
        // pair in every shape, the version-needed = 45
        // fake pair (field-identical to the real one)
        // serves as the standing control, and the
        // judged directory is the real one throughout.
        let filler: &[u8] = &[b'A'; 600];
        let entries: [(&[u8], &[u8]); 3] = [
            (b"a.txt", b"first copy"),
            (b"a.txt", b"second copy!"),
            (b"filler.bin", filler),
        ];
        for fake_needed in [None, Some(45u16), Some(99u16)] {
            let label = match fake_needed {
                None => "clean-gap".to_string(),
                Some(needed) => format!("fakepair{needed}"),
            };
            let (bytes, cd_start, walk_end) = r41a_fakepair_archive(&entries, fake_needed);
            let directory = central_directory(&bytes).expect("the real directory derives");
            assert_eq!(directory.cd_start, cd_start, "{label}");
            assert_eq!(directory.total, 3, "{label}");
            let view = prefix_free(&bytes);
            assert_eq!(
                view.len(),
                walk_end + 56 + 20 + 22,
                "{label}: the spliced view is the walk, the pair, and the end record"
            );
            let view_directory = central_directory(&view).expect("the view derives its directory");
            assert_eq!(view_directory.cd_start, cd_start, "{label}");
            assert_eq!(view_directory.total, 3, "{label}");
            assert_eq!(central_spots(&view), central_spots(&bytes), "{label}");
            // Every zip64 record standing in the view
            // is a pair the reader will judge: its
            // version-needed does not exceed its
            // version-made-by.
            let mut records = 0usize;
            let mut at = 0usize;
            while at + 16 <= view.len() {
                if view[at..].starts_with(b"PK\x06\x06") {
                    records += 1;
                    let made = u16::from_le_bytes([view[at + 12], view[at + 13]]);
                    let needed = u16::from_le_bytes([view[at + 14], view[at + 15]]);
                    assert!(
                        needed <= made,
                        "{label}: the view's zip64 record declares \
                         version-needed {needed} against version-made-by {made}"
                    );
                }
                at += 1;
            }
            assert_eq!(
                records, 1,
                "{label}: exactly the real record stands in the view"
            );
        }
    }

    /// One Stored local header in the round-43
    /// crafts' shape, with the checksum and flags
    /// stated outright and the declared sizes zeroed
    /// where the descriptor form asks for it.
    fn r43d_local(name: &[u8], data: &[u8], crc: u32, flags: u16, zero_sizes: bool) -> Vec<u8> {
        let mut out: Vec<u8> = Vec::new();
        out.extend_from_slice(b"PK\x03\x04");
        out.extend_from_slice(&20u16.to_le_bytes());
        out.extend_from_slice(&flags.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&crc.to_le_bytes());
        let size = if zero_sizes { 0 } else { data.len() as u32 };
        out.extend_from_slice(&size.to_le_bytes());
        out.extend_from_slice(&size.to_le_bytes());
        out.extend_from_slice(&(name.len() as u16).to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(name);
        out.extend_from_slice(data);
        out
    }

    /// One central record in the round-43 crafts'
    /// shape: the entry's name, its stated checksum,
    /// its data length for both size fields, and its
    /// local offset, with the flags stated outright.
    fn r43d_central(
        name: &[u8],
        data_len: u32,
        crc: u32,
        local_offset: u32,
        flags: u16,
    ) -> Vec<u8> {
        let mut rec: Vec<u8> = Vec::new();
        rec.extend_from_slice(b"PK\x01\x02");
        rec.extend_from_slice(&45u16.to_le_bytes());
        rec.extend_from_slice(&20u16.to_le_bytes());
        rec.extend_from_slice(&flags.to_le_bytes());
        rec.extend_from_slice(&0u16.to_le_bytes());
        rec.extend_from_slice(&0u16.to_le_bytes());
        rec.extend_from_slice(&0u16.to_le_bytes());
        rec.extend_from_slice(&crc.to_le_bytes());
        rec.extend_from_slice(&data_len.to_le_bytes());
        rec.extend_from_slice(&data_len.to_le_bytes());
        rec.extend_from_slice(&(name.len() as u16).to_le_bytes());
        rec.extend_from_slice(&0u16.to_le_bytes());
        rec.extend_from_slice(&0u16.to_le_bytes());
        rec.extend_from_slice(&0u16.to_le_bytes());
        rec.extend_from_slice(&0u16.to_le_bytes());
        rec.extend_from_slice(&0u32.to_le_bytes());
        rec.extend_from_slice(&local_offset.to_le_bytes());
        rec.extend_from_slice(name);
        rec
    }

    /// One end record in the round-43 crafts' shape:
    /// the disk number written to both disk fields, as
    /// the generator's disk craft writes it (1 and 1).
    fn r43d_eocd(total: u16, cd_size: u32, cd_offset: u32, disk: u16, comment: &[u8]) -> Vec<u8> {
        let mut out: Vec<u8> = Vec::new();
        out.extend_from_slice(b"PK\x05\x06");
        out.extend_from_slice(&disk.to_le_bytes());
        out.extend_from_slice(&disk.to_le_bytes());
        out.extend_from_slice(&total.to_le_bytes());
        out.extend_from_slice(&total.to_le_bytes());
        out.extend_from_slice(&cd_size.to_le_bytes());
        out.extend_from_slice(&cd_offset.to_le_bytes());
        out.extend_from_slice(&(comment.len() as u16).to_le_bytes());
        out.extend_from_slice(comment);
        out
    }

    /// The round-43 archive crafts (red team RT43-A1,
    /// generator `mk43.py`), assembled byte by byte
    /// over Stored entries whose checksums are stated
    /// per entry — `(name, data, crc)`; the model reads
    /// checksums as record facts, never as payload
    /// facts, so a stated checksum is the craft's fact
    /// at this level. `key` is the entry the crafts
    /// mutate; `plant` is the `(data, crc)` the plant
    /// directory's key record describes. Shapes:
    /// "clean" the real archive alone; "trailing" a
    /// complete second archive appended after the real
    /// end record — plant locals, plant directory,
    /// plant end record — whose records carry the real
    /// entries' names and the plant body's own local
    /// offsets while its end record declares its
    /// directory's absolute file position, so its
    /// archive offset derives to the real archive's;
    /// "comment" the plant directory standing in a gap
    /// before the real directory, with a fake end
    /// record embedded in the real end record's
    /// declared comment; "nameflip" the key entry's
    /// local name case-flipped; "crcbad" the key
    /// entry's local checksum falsified; "descriptor"
    /// the key entry in descriptor form (local sizes
    /// zero, bit-3 flag, a descriptor after its data,
    /// its central record normal); "disk1" both
    /// end-record disk fields set to 1; "count1" the
    /// end record's entry counts falsified to 1.
    /// Returns the bytes and the real directory's
    /// start.
    fn r43d_archive(
        entries: &[(&[u8], &[u8], u32)],
        key: usize,
        plant: (&[u8], u32),
        shape: &str,
    ) -> (Vec<u8>, usize) {
        let mut body: Vec<u8> = Vec::new();
        let mut locals: Vec<u32> = Vec::new();
        for (i, (name, data, crc)) in entries.iter().enumerate() {
            locals.push(body.len() as u32);
            if i == key && shape == "descriptor" {
                let mut local = r43d_local(name, data, 0, 0x08, true);
                local.extend_from_slice(b"PK\x07\x08");
                local.extend_from_slice(&crc.to_le_bytes());
                local.extend_from_slice(&(data.len() as u32).to_le_bytes());
                local.extend_from_slice(&(data.len() as u32).to_le_bytes());
                body.extend_from_slice(&local);
            } else if i == key && shape == "nameflip" {
                let flipped: Vec<u8> = name
                    .iter()
                    .map(|b| {
                        if b.is_ascii_lowercase() {
                            b.to_ascii_uppercase()
                        } else if b.is_ascii_uppercase() {
                            b.to_ascii_lowercase()
                        } else {
                            *b
                        }
                    })
                    .collect();
                body.extend_from_slice(&r43d_local(&flipped, data, *crc, 0, false));
            } else if i == key && shape == "crcbad" {
                body.extend_from_slice(&r43d_local(name, data, crc ^ 0xFFFF_FFFF, 0, false));
            } else {
                body.extend_from_slice(&r43d_local(name, data, *crc, 0, false));
            }
        }
        let cd_start = body.len();
        let mut centrals: Vec<u8> = Vec::new();
        for (i, (name, data, crc)) in entries.iter().enumerate() {
            let flags = if i == key && shape == "descriptor" {
                0x08
            } else {
                0
            };
            centrals.extend_from_slice(&r43d_central(
                name,
                data.len() as u32,
                *crc,
                locals[i],
                flags,
            ));
        }
        let total = entries.len() as u16;
        let real_cd = if shape == "comment" {
            cd_start + 5000
        } else {
            cd_start
        };
        // The plant body and directory: the real
        // entries with the key entry's content swapped
        // for the plant's, locals and records in the
        // plant body's own frame.
        let mut pbody: Vec<u8> = Vec::new();
        let mut plocals: Vec<u32> = Vec::new();
        for (i, (name, data, crc)) in entries.iter().enumerate() {
            plocals.push(pbody.len() as u32);
            let (pdata, pcrc) = if i == key { plant } else { (*data, *crc) };
            pbody.extend_from_slice(&r43d_local(name, pdata, pcrc, 0, false));
        }
        let mut pcentrals: Vec<u8> = Vec::new();
        for (i, (name, data, crc)) in entries.iter().enumerate() {
            let (pdata, pcrc) = if i == key { plant } else { (*data, *crc) };
            pcentrals.extend_from_slice(&r43d_central(
                name,
                pdata.len() as u32,
                pcrc,
                plocals[i],
                0,
            ));
        }
        let mut out = body;
        match shape {
            "trailing" => {
                out.extend_from_slice(&centrals);
                out.extend_from_slice(&r43d_eocd(
                    total,
                    centrals.len() as u32,
                    cd_start as u32,
                    0,
                    &[],
                ));
                let tcd = (out.len() + pbody.len()) as u32;
                out.extend_from_slice(&pbody);
                out.extend_from_slice(&pcentrals);
                out.extend_from_slice(&r43d_eocd(total, pcentrals.len() as u32, tcd, 0, &[]));
            }
            "comment" => {
                let mut gap = vec![b'G'; 5000];
                gap[200..200 + pcentrals.len()].copy_from_slice(&pcentrals);
                let fake = r43d_eocd(
                    total,
                    pcentrals.len() as u32,
                    (cd_start + 200) as u32,
                    0,
                    &[],
                );
                let mut comment = b"CC".to_vec();
                comment.extend_from_slice(&fake);
                comment.extend_from_slice(b"DDDDDDDDDDDDDDDDDDDD");
                out.extend_from_slice(&gap);
                out.extend_from_slice(&centrals);
                out.extend_from_slice(&r43d_eocd(
                    total,
                    centrals.len() as u32,
                    real_cd as u32,
                    0,
                    &comment,
                ));
            }
            "disk1" => {
                out.extend_from_slice(&centrals);
                out.extend_from_slice(&r43d_eocd(
                    total,
                    centrals.len() as u32,
                    cd_start as u32,
                    1,
                    &[],
                ));
            }
            "count1" => {
                out.extend_from_slice(&centrals);
                out.extend_from_slice(&r43d_eocd(
                    1,
                    centrals.len() as u32,
                    cd_start as u32,
                    0,
                    &[],
                ));
            }
            _ => {
                out.extend_from_slice(&centrals);
                out.extend_from_slice(&r43d_eocd(
                    total,
                    centrals.len() as u32,
                    cd_start as u32,
                    0,
                    &[],
                ));
            }
        }
        (out, real_cd)
    }

    /// One round-43 entry set at model scale:
    /// `(set, entries, key, plant)` — the set's name,
    /// its `(name, data, crc)` entries, the key
    /// entry's index, and the plant's `(data, crc)`.
    type R43dSet = (
        &'static str,
        Vec<(Vec<u8>, Vec<u8>, u32)>,
        usize,
        (Vec<u8>, u32),
    );

    /// The four entry sets of the round-43 crafts, at
    /// model scale: the generator's names and duplicate
    /// shapes, with contents sized the way the
    /// generator's stand — the idml, clip, and mxl key
    /// contents the same length as their plant (so the
    /// plant's declared local-offset set equals the
    /// real directory's), the font key content a
    /// different length from its plant (so it does
    /// not). Owned contents; the caller maps them to
    /// the builder's slices.
    fn r43d_entry_sets() -> Vec<R43dSet> {
        let filler = vec![b'A'; 600];
        vec![
            (
                "idml",
                vec![
                    (b"designmap.xml".to_vec(), vec![b'd'; 79], 101),
                    (b"Resources/Styles.xml".to_vec(), vec![b's'; 77], 102),
                    (b"Stories/Story_u9.xml".to_vec(), vec![b'a'; 207], 103),
                    (b"Stories/Story_u9.xml".to_vec(), vec![b'b'; 212], 104),
                    (b"Spreads/Spread_u1.xml".to_vec(), vec![b'p'; 441], 105),
                    (b"filler.bin".to_vec(), filler.clone(), 106),
                ],
                2,
                (vec![b'z'; 207], 903),
            ),
            (
                "font",
                vec![
                    (b"MyFont.ttf".to_vec(), vec![b'R'; 1000], 201),
                    (b"MyFont.ttf".to_vec(), vec![b'B'; 900], 202),
                    (b"filler.bin".to_vec(), filler.clone(), 203),
                ],
                0,
                (vec![b'Q'; 800], 901),
            ),
            (
                "clip",
                vec![
                    (b"pic.png".to_vec(), vec![b'r'; 69], 301),
                    (b"pic.png".to_vec(), vec![b'u'; 69], 302),
                    (b"filler.bin".to_vec(), filler.clone(), 303),
                ],
                0,
                (vec![b'v'; 69], 904),
            ),
            (
                "mxl",
                vec![
                    (b"META-INF/container.xml".to_vec(), vec![b'c'; 105], 401),
                    (b"score.musicxml".to_vec(), vec![b'f'; 333], 402),
                    (b"score.musicxml".to_vec(), vec![b'g'; 334], 403),
                    (b"filler.bin".to_vec(), filler, 404),
                ],
                1,
                (vec![b'h'; 333], 905),
            ),
        ]
    }

    #[test]
    fn r43d_a_complete_trailing_archive_never_steers_the_derivation() {
        // Round 43, F6 (red team RT43-A1): the sixth
        // generation of the archive discipline — a
        // second complete archive appended after the
        // first (real body + real directory + real end
        // record, then plant locals + plant directory +
        // plant end record, the plant records carrying
        // the real entries' names and offsets with the
        // key entry's record describing plant content)
        // was judged at the TRAILING directory for three
        // of the four entry sets, and the clip door
        // failed those crafts on the plant record's
        // checksum where the clean controls read OK.
        // The first complete directory is the directory
        // judged now, in every set: the derived start is
        // the real directory's, and the judged spots are
        // the clean control's spot for spot, checksums
        // included. The font set is the standing
        // control inside the family: its plant content
        // is a different length from the real key
        // entry's, so its plant's offset set never
        // equalled the real one's and the real directory
        // was judged even before this fix.
        for (set, entries, key, plant) in r43d_entry_sets() {
            let slices: Vec<(&[u8], &[u8], u32)> = entries
                .iter()
                .map(|(name, data, crc)| (name.as_slice(), data.as_slice(), *crc))
                .collect();
            let plant_ref = (plant.0.as_slice(), plant.1);
            let (clean, clean_cd) = r43d_archive(&slices, key, plant_ref, "clean");
            let control = central_directory(&clean).expect("the clean control derives");
            assert_eq!(control.cd_start, clean_cd, "{set}: the control stands");
            let (bytes, real_cd) = r43d_archive(&slices, key, plant_ref, "trailing");
            let directory = central_directory(&bytes).expect("the trailing craft derives");
            assert_eq!(
                directory.cd_start, real_cd,
                "{set}: the first directory is judged, not the trailing one"
            );
            assert_eq!(directory.cd_start, control.cd_start, "{set}");
            assert_eq!(directory.total, control.total, "{set}");
            assert_eq!(directory.archive_offset, control.archive_offset, "{set}");
            assert_eq!(
                central_spots(&bytes),
                central_spots(&clean),
                "{set}: the judged spots are the clean control's, checksums included"
            );
        }
    }

    #[test]
    fn r43d_held_archive_arms_stand_as_controls() {
        // Round 43, F6's held surface (red team RT43-A1,
        // the arms that held on the audited tree, pinned
        // here as controls so this batch's change is
        // witnessed not to move them): the
        // comment-embedded end record subordinated whole,
        // the local-header mutations (the key entry's
        // local name case-flipped, its local checksum
        // falsified, its descriptor form) and the
        // end-record disk fields set to 1 all judge the
        // real directory with the clean control's judged
        // facts — name, checksum, compressed size, and
        // method, spot for spot (the descriptor form
        // shifts the later entries' local offsets by its
        // sixteen bytes, which is the form's own shape,
        // so offsets are not among the compared facts
        // here) — and the entry count falsified to 1
        // derives no directory at all, the uniform
        // refusal every door reports. These arms pass
        // on the base tree by design; they are controls,
        // not witnesses.
        for (set, entries, key, plant) in r43d_entry_sets() {
            let slices: Vec<(&[u8], &[u8], u32)> = entries
                .iter()
                .map(|(name, data, crc)| (name.as_slice(), data.as_slice(), *crc))
                .collect();
            let plant_ref = (plant.0.as_slice(), plant.1);
            let (clean, _) = r43d_archive(&slices, key, plant_ref, "clean");
            let control = central_directory(&clean).expect("the clean control derives");
            let clean_spots = central_spots(&clean);
            for shape in ["comment", "nameflip", "crcbad", "descriptor", "disk1"] {
                let (bytes, real_cd) = r43d_archive(&slices, key, plant_ref, shape);
                let directory = central_directory(&bytes)
                    .unwrap_or_else(|| panic!("{set} {shape}: the real directory derives"));
                assert_eq!(directory.cd_start, real_cd, "{set} {shape}");
                assert_eq!(directory.total, control.total, "{set} {shape}");
                let spots = central_spots(&bytes);
                assert_eq!(spots.len(), clean_spots.len(), "{set} {shape}");
                for (spot, clean_spot) in spots.iter().zip(clean_spots.iter()) {
                    assert_eq!(spot.0, clean_spot.0, "{set} {shape}: the name stands");
                    assert_eq!(
                        spot.1.crc32, clean_spot.1.crc32,
                        "{set} {shape}: the checksum stands"
                    );
                    assert_eq!(
                        spot.1.comp_size, clean_spot.1.comp_size,
                        "{set} {shape}: the size stands"
                    );
                    assert_eq!(
                        spot.1.method, clean_spot.1.method,
                        "{set} {shape}: the method stands"
                    );
                }
            }
            let (bytes, _) = r43d_archive(&slices, key, plant_ref, "count1");
            assert_eq!(
                central_directory(&bytes),
                None,
                "{set} count1: no directory derives, the uniform refusal"
            );
        }
    }
}
