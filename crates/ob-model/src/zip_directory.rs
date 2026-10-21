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
//! fallback is the same for every door too.
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

/// Whether the central records starting at `cd_start`
/// walk, `total` of them in the shape the directory
/// scans themselves walk, to exactly `end` and no
/// further: the proof an arithmetic candidate is the
/// real directory and not signature bytes standing in
/// an entry's payload.
fn directory_walks_to(bytes: &[u8], cd_start: usize, total: u64, end: usize) -> bool {
    let mut at = cd_start;
    for _ in 0..total {
        if at + 46 > bytes.len() || !bytes[at..].starts_with(b"PK\x01\x02") {
            return false;
        }
        let name_len = usize::from(u16_at(bytes, at + 28));
        let extra_len = usize::from(u16_at(bytes, at + 30));
        let comment_len = usize::from(u16_at(bytes, at + 32));
        let Some(name_end) = (at + 46).checked_add(name_len) else {
            return false;
        };
        let Some(extra_end) = name_end.checked_add(extra_len) else {
            return false;
        };
        if extra_end > bytes.len() {
            return false;
        }
        let Some(next) = extra_end.checked_add(comment_len) else {
            return false;
        };
        at = next;
    }
    at == end
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

/// The derivation the doors used to state per door
/// (round 32, F4), kept as the one fallback for an
/// archive whose directory is not arithmetically
/// derivable: the zip64 record found by searching
/// forward from its declared offset to its locator,
/// and the directory's first record found by searching
/// forward from the declared offset to the end record.
fn hunted_directory(bytes: &[u8], eocd: usize) -> Option<CentralDirectory> {
    let mut total = u64::from(u16_at(bytes, eocd + 10));
    let mut cd_offset = u64::from(u32_at(bytes, eocd + 16));
    if (total == u64::from(u16::MAX) || cd_offset == u64::from(u32::MAX)) && eocd >= 20 {
        let locator = &bytes[eocd - 20..eocd];
        if locator.starts_with(b"PK\x06\x07") {
            let declared = u64_at(locator, 8);
            let mut record_at = usize::try_from(declared).ok()?;
            let mut found = None;
            while record_at + 4 <= eocd - 20 {
                if bytes[record_at..].starts_with(b"PK\x06\x06") {
                    found = Some(record_at);
                    break;
                }
                record_at += 1;
            }
            if let Some(record_at) = found {
                if record_at + 56 <= bytes.len() {
                    total = u64_at(bytes, record_at + 32);
                    cd_offset = u64_at(bytes, record_at + 48);
                }
            }
        }
    }
    if total == 0 {
        return None;
    }
    let mut at = usize::try_from(cd_offset).ok()?;
    let mut cd_start = None;
    while at + 4 <= eocd {
        if bytes[at..].starts_with(b"PK\x01\x02") {
            cd_start = Some(at);
            break;
        }
        at += 1;
    }
    let cd_start = cd_start?;
    let archive_offset = (cd_start as u64).saturating_sub(cd_offset);
    Some(CentralDirectory {
        archive_offset,
        cd_start,
        total,
    })
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

    #[test]
    fn bytes_with_no_end_record_derive_nothing_and_pass_through() {
        assert_eq!(central_directory(b"not a zip"), None);
        let bytes = b"not a zip".to_vec();
        assert_eq!(&*prefix_free(&bytes), &bytes[..]);
    }
}
