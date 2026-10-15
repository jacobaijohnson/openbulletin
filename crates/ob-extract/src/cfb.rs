//! A reader for the OLE Compound File Binary container
//! (Microsoft's published [MS-CFB] format): the envelope every
//! legacy Office binary file ships in, Publisher included. This
//! is our own implementation from the container specification,
//! written for the native .pub reader (Jacob's directive:
//! self-contained, our own ownership). All parsing is
//! bounds-checked and capped: these files are strangers, so the
//! header is validated before any arithmetic runs on it, chains
//! error instead of truncating silently, and sector counts are
//! capped by the physical size of the file in hand.

use std::collections::HashSet;

const SIGNATURE: [u8; 8] = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
/// Largest regular sector id ([MS-CFB] MAXREGSECT).
const MAX_REG_SECT: u32 = 0xFFFF_FFFA;
const DIF_SECT: u32 = 0xFFFF_FFFC;
const FAT_SECT: u32 = 0xFFFF_FFFD;
const END_OF_CHAIN: u32 = 0xFFFF_FFFE;
const FREE_SECT: u32 = 0xFFFF_FFFF;
/// Directory id meaning "no entry" ([MS-CFB] NOSTREAM).
const NO_STREAM: u32 = 0xFFFF_FFFF;
/// No single stream may claim more than this.
const MAX_STREAM: u64 = 256 * 1024 * 1024;
/// No input file may claim more than this.
const MAX_INPUT: u64 = 512 * 1024 * 1024;
/// Deepest sibling-tree nesting the path walk will follow.
const MAX_TREE_DEPTH: usize = 1024;

#[derive(Debug, Clone)]
pub struct DirEntry {
    pub name: String,
    /// 1 = storage (folder), 2 = stream (file), 5 = root.
    pub object_type: u8,
    /// Directory ids of the left sibling, right sibling, and
    /// child entries: raw slots in the directory array, exactly
    /// as [MS-CFB] stores them. `NO_STREAM` when absent.
    pub left_id: u32,
    pub right_id: u32,
    pub child_id: u32,
    pub start_sector: u32,
    pub size: u64,
}

pub struct Cfb {
    sector_size: usize,
    mini_sector_size: usize,
    mini_cutoff: u64,
    fat: Vec<u32>,
    minifat: Vec<u32>,
    entries: Vec<DirEntry>,
    mini_stream: Vec<u8>,
    data: Vec<u8>,
}

fn u16le(b: &[u8], off: usize) -> Result<u16, String> {
    let s = b.get(off..off + 2).ok_or("compound file is truncated")?;
    Ok(u16::from_le_bytes([s[0], s[1]]))
}

fn u32le(b: &[u8], off: usize) -> Result<u32, String> {
    let s = b.get(off..off + 4).ok_or("compound file is truncated")?;
    Ok(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

fn u64le(b: &[u8], off: usize) -> Result<u64, String> {
    let s = b.get(off..off + 8).ok_or("compound file is truncated")?;
    let mut a = [0u8; 8];
    a.copy_from_slice(s);
    Ok(u64::from_le_bytes(a))
}

/// The bytes of one regular sector, with checked arithmetic.
fn sector_bytes(data: &[u8], sector_size: usize, n: u32) -> Result<&[u8], String> {
    let start = (n as usize)
        .checked_add(1)
        .and_then(|v| v.checked_mul(sector_size))
        .ok_or("compound file sector arithmetic overflows")?;
    let end = start
        .checked_add(sector_size)
        .ok_or("compound file sector arithmetic overflows")?;
    data.get(start..end)
        .ok_or_else(|| "compound file points outside itself".to_string())
}

/// Follow a FAT chain to its end, strictly: a reserved marker
/// (FREESECT, FATSECT, DIFSECT) mid-chain is corruption, a
/// repeated sector is a loop, and every step must land inside
/// the FAT itself.
fn fat_chain(fat: &[u32], start: u32) -> Result<Vec<u32>, String> {
    let mut out = Vec::new();
    let mut seen: HashSet<u32> = HashSet::new();
    let mut cur = start;
    loop {
        match cur {
            END_OF_CHAIN => break,
            id if id <= MAX_REG_SECT => {
                if !seen.insert(id) {
                    return Err("that compound file has a looping sector chain".into());
                }
                let next = fat
                    .get(id as usize)
                    .ok_or("compound file chain runs off its table")?;
                out.push(id);
                cur = *next;
            }
            _ => {
                return Err(
                    "that compound file's chain runs through a reserved sector marker".into(),
                )
            }
        }
    }
    Ok(out)
}

/// Gather exactly `size` bytes from a FAT chain: gathering
/// stops as soon as the declared size is reached, and a chain
/// that ends (or breaks) before supplying it is an error, not
/// a short read.
fn gather_chain(
    data: &[u8],
    sector_size: usize,
    fat: &[u32],
    start: u32,
    size: u64,
) -> Result<Vec<u8>, String> {
    debug_assert!(size > 0 && size <= MAX_STREAM);
    let mut out: Vec<u8> = Vec::new();
    let mut seen: HashSet<u32> = HashSet::new();
    let mut cur = start;
    while (out.len() as u64) < size {
        let id = match cur {
            END_OF_CHAIN => break,
            id if id <= MAX_REG_SECT => id,
            _ => {
                return Err(
                    "that compound file's chain runs through a reserved sector marker".into(),
                )
            }
        };
        if !seen.insert(id) {
            return Err("that compound file has a looping sector chain".into());
        }
        out.extend_from_slice(sector_bytes(data, sector_size, id)?);
        cur = *fat
            .get(id as usize)
            .ok_or("compound file chain runs off its table")?;
    }
    if (out.len() as u64) < size {
        return Err("that compound file's chain is shorter than the stream it holds".into());
    }
    out.truncate(size as usize);
    Ok(out)
}

/// [MS-CFB] compares directory names case-insensitively, by
/// uppercase: two names are the same when their uppercased
/// forms are the same. (Unicode uppercase is used here; for
/// the names Publisher writes, ASCII-insensitive comparison
/// would reach the same answers.)
fn name_eq(a: &str, b: &str) -> bool {
    a.chars()
        .flat_map(char::to_uppercase)
        .eq(b.chars().flat_map(char::to_uppercase))
}

impl Cfb {
    pub fn open(data: Vec<u8>) -> Result<Cfb, String> {
        if (data.len() as u64) > MAX_INPUT {
            return Err("that file is implausibly large for a compound file".into());
        }
        if data.len() < 512 || data[0..8] != SIGNATURE {
            return Err("that is not an OLE compound file (the envelope Publisher uses)".into());
        }
        // Validate the header BEFORE any arithmetic runs on it:
        // [MS-CFB] 2.2 fixes every one of these fields.
        let major = u16le(&data, 26)?;
        if major != 3 && major != 4 {
            return Err("that compound file has an unsupported version".into());
        }
        if u16le(&data, 28)? != 0xFFFE {
            return Err("that compound file has an unusual byte order".into());
        }
        let sector_shift = u16le(&data, 30)?;
        let want_shift: u16 = if major == 3 { 9 } else { 12 };
        if sector_shift != want_shift {
            return Err("that compound file has an unusual sector size".into());
        }
        if u16le(&data, 32)? != 6 {
            return Err("that compound file has an unusual mini sector size".into());
        }
        let mini_cutoff = u32le(&data, 56)? as u64;
        if mini_cutoff != 4096 {
            return Err("that compound file has an unusual mini stream cutoff".into());
        }
        let sector_size = 1usize << sector_shift;
        let mini_sector_size = 1usize << 6;
        if data.len() < sector_size {
            return Err("compound file is truncated".into());
        }
        // Sector counts can never exceed the sectors physically
        // present (the header occupies the first one).
        let total_sectors = (data.len() / sector_size - 1) as u64;
        let num_fat = u32le(&data, 44)?;
        let first_dir = u32le(&data, 48)?;
        let first_minifat = u32le(&data, 60)?;
        let num_minifat = u32le(&data, 64)?;
        let first_difat = u32le(&data, 68)?;
        let num_difat = u32le(&data, 72)?;
        if num_fat == 0 || num_fat as u64 > total_sectors {
            return Err("that compound file's sector table is incomplete".into());
        }
        if num_difat as u64 > total_sectors || num_minifat as u64 > total_sectors {
            return Err("that compound file claims more table sectors than it holds".into());
        }

        // DIFAT: 109 entries in the header, then a chain of
        // sectors each holding (sector_size/4 - 1) more. A FAT
        // sector named twice is corruption; the chain itself is
        // visited-checked and count-capped.
        let mut fat_sectors: Vec<u32> = Vec::new();
        let mut fat_sector_set: HashSet<u32> = HashSet::new();
        let mut take_fat_sector = |v: u32, fat_sectors: &mut Vec<u32>| -> Result<(), String> {
            match v {
                FREE_SECT => Ok(()),
                DIF_SECT | FAT_SECT => {
                    Err("that compound file's table list holds a reserved marker".into())
                }
                id if id <= MAX_REG_SECT => {
                    if !fat_sector_set.insert(id) {
                        return Err("that compound file names the same table sector twice".into());
                    }
                    fat_sectors.push(id);
                    Ok(())
                }
                _ => Err("that compound file's table list holds a reserved marker".into()),
            }
        };
        for i in 0..109 {
            take_fat_sector(u32le(&data, 76 + i * 4)?, &mut fat_sectors)?;
        }
        let mut visited_difat: HashSet<u32> = HashSet::new();
        let mut next_difat = first_difat;
        for _ in 0..num_difat {
            let id = match next_difat {
                END_OF_CHAIN => break,
                id if id <= MAX_REG_SECT => id,
                _ => {
                    return Err(
                        "that compound file's table chain runs through a reserved marker".into(),
                    )
                }
            };
            if !visited_difat.insert(id) {
                return Err("that compound file has a looping table chain".into());
            }
            let sec = sector_bytes(&data, sector_size, id)?;
            let per = sector_size / 4 - 1;
            for i in 0..per {
                take_fat_sector(u32le(sec, i * 4)?, &mut fat_sectors)?;
            }
            next_difat = u32le(sec, per * 4)?;
        }
        if (fat_sectors.len() as u64) < num_fat as u64 {
            return Err("that compound file's sector table is incomplete".into());
        }
        let mut fat: Vec<u32> = Vec::new();
        for s in fat_sectors.iter().take(num_fat as usize) {
            let sec = sector_bytes(&data, sector_size, *s)?;
            for i in 0..sector_size / 4 {
                fat.push(u32le(sec, i * 4)?);
            }
        }

        // Directory: a stream of 128-byte entries. EVERY slot
        // is kept: sibling and child ids are indices into this
        // raw array, so dropping an unused slot would renumber
        // the world.
        let mut dir_bytes = Vec::new();
        for s in fat_chain(&fat, first_dir)? {
            dir_bytes.extend_from_slice(sector_bytes(&data, sector_size, s)?);
        }
        let mut entries: Vec<DirEntry> = Vec::new();
        for chunk in dir_bytes.chunks_exact(128) {
            let object_type = chunk[66];
            let name_len = u16le(chunk, 64)? as usize;
            let name = if (2..=64).contains(&name_len) {
                let units: Vec<u16> = chunk[0..name_len - 2]
                    .chunks_exact(2)
                    .map(|c| u16::from_le_bytes([c[0], c[1]]))
                    .collect();
                String::from_utf16_lossy(&units)
            } else {
                String::new()
            };
            let size = if major == 3 {
                u32le(chunk, 120)? as u64
            } else {
                u64le(chunk, 120)?
            };
            entries.push(DirEntry {
                name,
                object_type,
                left_id: u32le(chunk, 68)?,
                right_id: u32le(chunk, 72)?,
                child_id: u32le(chunk, 76)?,
                start_sector: u32le(chunk, 116)?,
                size,
            });
        }
        // [MS-CFB]: the root entry is directory entry 0.
        let root = entries
            .first()
            .filter(|e| e.object_type == 5)
            .cloned()
            .ok_or("that compound file has no root entry")?;

        // The mini stream is the root entry's own stream, read
        // through the regular FAT.
        let mut mini_stream = Vec::new();
        if root.size > 0 {
            if root.size > MAX_STREAM {
                return Err("that compound file's mini stream is implausibly large".into());
            }
            mini_stream = gather_chain(&data, sector_size, &fat, root.start_sector, root.size)?;
        }
        let mut minifat: Vec<u32> = Vec::new();
        if num_minifat > 0 {
            let budget = num_minifat as usize * (sector_size / 4);
            for s in fat_chain(&fat, first_minifat)? {
                let sec = sector_bytes(&data, sector_size, s)?;
                for i in 0..sector_size / 4 {
                    if minifat.len() < budget {
                        minifat.push(u32le(sec, i * 4)?);
                    }
                }
            }
            if minifat.is_empty() {
                return Err("that compound file's mini sector table is missing".into());
            }
        }

        Ok(Cfb {
            sector_size,
            mini_sector_size,
            mini_cutoff,
            fat,
            minifat,
            entries,
            mini_stream,
            data,
        })
    }

    /// Follow a FAT chain and gather exactly `size` bytes.
    fn read_chain(&self, start: u32, size: u64) -> Result<Vec<u8>, String> {
        if size > MAX_STREAM {
            return Err("a stream in that file is implausibly large".into());
        }
        gather_chain(&self.data, self.sector_size, &self.fat, start, size)
    }

    /// Follow a miniFAT chain inside the mini stream and
    /// gather exactly `size` bytes.
    fn read_mini_chain(&self, start: u32, size: u64) -> Result<Vec<u8>, String> {
        if size > MAX_STREAM {
            return Err("a stream in that file is implausibly large".into());
        }
        let mut out: Vec<u8> = Vec::new();
        let mut seen: HashSet<u32> = HashSet::new();
        let mut cur = start;
        while (out.len() as u64) < size {
            let id = match cur {
                END_OF_CHAIN => break,
                id if id <= MAX_REG_SECT => id,
                _ => {
                    return Err(
                        "that compound file's mini chain runs through a reserved marker".into(),
                    )
                }
            };
            if !seen.insert(id) {
                return Err("that compound file has a looping mini chain".into());
            }
            let off = (id as usize)
                .checked_mul(self.mini_sector_size)
                .ok_or("compound file mini sector arithmetic overflows")?;
            let end = off
                .checked_add(self.mini_sector_size)
                .ok_or("compound file mini sector arithmetic overflows")?;
            let sec = self
                .mini_stream
                .get(off..end)
                .ok_or("compound file mini stream is truncated")?;
            out.extend_from_slice(sec);
            cur = *self
                .minifat
                .get(id as usize)
                .ok_or("compound file mini chain runs off its table")?;
        }
        if (out.len() as u64) < size {
            return Err(
                "that compound file's mini chain is shorter than the stream it holds".into(),
            );
        }
        out.truncate(size as usize);
        Ok(out)
    }

    /// Read the bytes of one directory entry, per its size:
    /// below the mini cutoff, mini sectors; at or above it,
    /// regular sectors. A nonempty small stream in a file with
    /// no miniFAT is corruption, not a reason to go looking in
    /// the regular FAT.
    fn read_entry(&self, entry: &DirEntry) -> Result<Vec<u8>, String> {
        if entry.size == 0 {
            return Ok(Vec::new());
        }
        if entry.size < self.mini_cutoff {
            if self.minifat.is_empty() {
                return Err(format!(
                    "that file's stream {} sits in the mini stream, but the file has no mini sector table",
                    entry.name
                ));
            }
            self.read_mini_chain(entry.start_sector, entry.size)
        } else {
            self.read_chain(entry.start_sector, entry.size)
        }
    }

    /// Search one storage's child subtree for `name`,
    /// exhaustively: writers do not always keep the sibling
    /// tree in its specified order, so ordering is never
    /// trusted, only walked. Cycle-guarded by a visited set
    /// and a depth cap.
    fn find_child(&self, storage: usize, name: &str) -> Result<Option<usize>, String> {
        let head = self.entries[storage].child_id;
        if head == NO_STREAM {
            return Ok(None);
        }
        let mut visited: HashSet<usize> = HashSet::new();
        let mut stack: Vec<(usize, usize)> = vec![(head as usize, 0)];
        while let Some((idx, depth)) = stack.pop() {
            if depth > MAX_TREE_DEPTH {
                return Err("that compound file's folder tree is implausibly deep".into());
            }
            if !visited.insert(idx) {
                continue;
            }
            let entry = self
                .entries
                .get(idx)
                .ok_or("that compound file's folder tree points outside itself")?;
            if entry.object_type == 0 {
                continue; // an unused slot holds no name and no links worth following
            }
            if name_eq(&entry.name, name) {
                return Ok(Some(idx));
            }
            if entry.right_id != NO_STREAM {
                stack.push((entry.right_id as usize, depth + 1));
            }
            if entry.left_id != NO_STREAM {
                stack.push((entry.left_id as usize, depth + 1));
            }
        }
        Ok(None)
    }

    pub fn entries(&self) -> &[DirEntry] {
        &self.entries
    }

    pub fn stream_names(&self) -> Vec<String> {
        self.entries
            .iter()
            .filter(|e| e.object_type == 2)
            .map(|e| e.name.clone())
            .collect()
    }

    /// Read one stream by its path through the storages, e.g.
    /// `&["Quill", "QuillSub", "CONTENTS"]`. Name matching is
    /// case-insensitive per [MS-CFB]. A path is the only way to
    /// name a stream: leaf names alone repeat across storages
    /// (the root's "Contents" is not Quill's "CONTENTS").
    pub fn read_stream_path(&self, path: &[&str]) -> Result<Vec<u8>, String> {
        if path.is_empty() {
            return Err("no stream path was given".into());
        }
        let mut storage = 0usize; // the root entry
        for (i, component) in path.iter().enumerate() {
            let found = self
                .find_child(storage, component)?
                .ok_or_else(|| format!("that file has no entry named {component} there"))?;
            let entry = &self.entries[found];
            let last = i + 1 == path.len();
            if last {
                if entry.object_type != 2 {
                    return Err(format!("{component} is a folder, not a stream"));
                }
                return self.read_entry(entry);
            }
            if entry.object_type != 1 {
                return Err(format!("{component} is not a folder"));
            }
            storage = found;
        }
        unreachable!("the loop always returns on the last component")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A spec-true CFB v3 writer for tests: mini cutoff 4096
    /// as [MS-CFB] fixes it, and a directory tree that is
    /// actually reachable (root's child id set, sibling links
    /// set). Layout, 512-byte sectors: 0 FAT, 1 directory (4
    /// entries), 2 mini stream container, 3-12 a ten-sector
    /// regular stream, 13 miniFAT, 14 directory continued
    /// (2 more entries). Entries: 0 Root Entry, 1 Big (regular
    /// stream, 5000 bytes), 2 Small (mini stream, 11 bytes),
    /// 3 Quill (storage), 4 QuillSub (storage inside Quill),
    /// 5 CONTENTS (mini stream inside QuillSub, 20 bytes).
    fn build_test_cfb() -> Vec<u8> {
        let sector = 512usize;
        let mut file = vec![0u8; sector * 16];
        // Header
        file[0..8].copy_from_slice(&SIGNATURE);
        file[24..26].copy_from_slice(&62u16.to_le_bytes()); // minor
        file[26..28].copy_from_slice(&3u16.to_le_bytes()); // major v3
        file[28..30].copy_from_slice(&0xFFFEu16.to_le_bytes());
        file[30..32].copy_from_slice(&9u16.to_le_bytes()); // 512-byte sectors
        file[32..34].copy_from_slice(&6u16.to_le_bytes()); // 64-byte mini sectors
        file[44..48].copy_from_slice(&1u32.to_le_bytes()); // one FAT sector
        file[48..52].copy_from_slice(&1u32.to_le_bytes()); // first dir sector = 1
        file[56..60].copy_from_slice(&4096u32.to_le_bytes()); // mini cutoff, the spec value
        file[60..64].copy_from_slice(&13u32.to_le_bytes()); // first miniFAT = 13
        file[64..68].copy_from_slice(&1u32.to_le_bytes()); // one miniFAT sector
        file[68..72].copy_from_slice(&END_OF_CHAIN.to_le_bytes()); // no DIFAT chain
        file[72..76].copy_from_slice(&0u32.to_le_bytes());
        file[76..80].copy_from_slice(&0u32.to_le_bytes()); // DIFAT[0] = FAT at 0
        for i in 1..109 {
            file[76 + i * 4..80 + i * 4].copy_from_slice(&FREE_SECT.to_le_bytes());
        }
        // FAT at sector 0: 0=FATSECT, 1->14 (directory), 2=END
        // (mini stream), 3->4->...->12 (Big), 13=END (miniFAT),
        // 14=END (directory end).
        let fat_off = sector;
        let mut fat_entries = [FREE_SECT; 128];
        fat_entries[0] = FAT_SECT;
        fat_entries[1] = 14;
        fat_entries[2] = END_OF_CHAIN;
        for (i, slot) in fat_entries.iter_mut().enumerate().skip(3).take(9) {
            *slot = (i + 1) as u32;
        }
        fat_entries[12] = END_OF_CHAIN;
        fat_entries[13] = END_OF_CHAIN;
        fat_entries[14] = END_OF_CHAIN;
        for (i, v) in fat_entries.iter().enumerate() {
            file[fat_off + i * 4..fat_off + i * 4 + 4].copy_from_slice(&v.to_le_bytes());
        }
        // MiniFAT at sector 13: mini 0 -> END, mini 1 -> END.
        let mf_off = sector * 14;
        for i in 0..sector / 4 {
            let v = if i < 2 { END_OF_CHAIN } else { FREE_SECT };
            file[mf_off + i * 4..mf_off + i * 4 + 4].copy_from_slice(&v.to_le_bytes());
        }
        // Directory: entries 0-3 in sector 1, entries 4-5 in
        // sector 14.
        let entry = |file: &mut Vec<u8>,
                     idx: usize,
                     name: &str,
                     ty: u8,
                     left: u32,
                     right: u32,
                     child: u32,
                     start: u32,
                     size: u64| {
            let sec = if idx < 4 { 1 } else { 14 };
            let base = (sec + 1) * sector + (idx % 4) * 128;
            let units: Vec<u16> = name.encode_utf16().collect();
            for (i, u) in units.iter().enumerate() {
                file[base + i * 2..base + i * 2 + 2].copy_from_slice(&u.to_le_bytes());
            }
            file[base + 64..base + 66]
                .copy_from_slice(&(((units.len() + 1) * 2) as u16).to_le_bytes());
            file[base + 66] = ty;
            file[base + 68..base + 72].copy_from_slice(&left.to_le_bytes());
            file[base + 72..base + 76].copy_from_slice(&right.to_le_bytes());
            file[base + 76..base + 80].copy_from_slice(&child.to_le_bytes());
            file[base + 116..base + 120].copy_from_slice(&start.to_le_bytes());
            file[base + 120..base + 128].copy_from_slice(&size.to_le_bytes());
        };
        // Root's children hang off entry 1: Big -right-> Quill
        // -right-> Small (deliberately not name order at the
        // head: the walk must not care).
        entry(
            &mut file,
            0,
            "Root Entry",
            5,
            NO_STREAM,
            NO_STREAM,
            1,
            2,
            128,
        );
        entry(&mut file, 1, "Big", 2, NO_STREAM, 3, NO_STREAM, 3, 5000);
        entry(
            &mut file, 2, "Small", 2, NO_STREAM, NO_STREAM, NO_STREAM, 0, 11,
        );
        entry(&mut file, 3, "Quill", 1, NO_STREAM, 2, 4, 0, 0);
        entry(&mut file, 4, "QuillSub", 1, NO_STREAM, NO_STREAM, 5, 0, 0);
        entry(
            &mut file, 5, "CONTENTS", 2, NO_STREAM, NO_STREAM, NO_STREAM, 1, 20,
        );
        // Mini stream at sector 2: mini 0 = "hello world",
        // mini 1 = the Quill text payload.
        let mini_off = sector * 3;
        file[mini_off..mini_off + 11].copy_from_slice(b"hello world");
        file[mini_off + 64..mini_off + 84].copy_from_slice(b"quill text payload!!");
        // Big stream at sectors 3-12.
        for i in 0..sector * 10 {
            file[sector * 4 + i] = b'A';
        }
        file
    }

    /// A minimal CFB v4 file: 4096-byte sectors, one FAT
    /// sector (0), one directory sector (1), and a "Big"
    /// stream of 5000 bytes across sectors 2-3, its size in
    /// the 64-bit field v4 uses.
    fn build_test_cfb_v4() -> Vec<u8> {
        let sector = 4096usize;
        let mut file = vec![0u8; sector * 5];
        file[0..8].copy_from_slice(&SIGNATURE);
        file[24..26].copy_from_slice(&62u16.to_le_bytes());
        file[26..28].copy_from_slice(&4u16.to_le_bytes()); // major v4
        file[28..30].copy_from_slice(&0xFFFEu16.to_le_bytes());
        file[30..32].copy_from_slice(&12u16.to_le_bytes()); // 4096-byte sectors
        file[32..34].copy_from_slice(&6u16.to_le_bytes());
        file[44..48].copy_from_slice(&1u32.to_le_bytes()); // one FAT sector
        file[48..52].copy_from_slice(&1u32.to_le_bytes()); // first dir sector = 1
        file[56..60].copy_from_slice(&4096u32.to_le_bytes()); // mini cutoff
        file[60..64].copy_from_slice(&END_OF_CHAIN.to_le_bytes()); // no miniFAT
        file[64..68].copy_from_slice(&0u32.to_le_bytes());
        file[68..72].copy_from_slice(&END_OF_CHAIN.to_le_bytes());
        file[72..76].copy_from_slice(&0u32.to_le_bytes());
        file[76..80].copy_from_slice(&0u32.to_le_bytes()); // DIFAT[0] = FAT at 0
        for i in 1..109 {
            file[76 + i * 4..80 + i * 4].copy_from_slice(&FREE_SECT.to_le_bytes());
        }
        // FAT at sector 0 (file offset 4096): 0=FATSECT,
        // 1=END (directory), 2->3, 3=END (Big).
        let fat_off = sector;
        let mut fat_entries = [FREE_SECT; 1024];
        fat_entries[0] = FAT_SECT;
        fat_entries[1] = END_OF_CHAIN;
        fat_entries[2] = 3;
        fat_entries[3] = END_OF_CHAIN;
        for (i, v) in fat_entries.iter().enumerate() {
            file[fat_off + i * 4..fat_off + i * 4 + 4].copy_from_slice(&v.to_le_bytes());
        }
        // Directory at sector 1 (file offset 8192).
        let dir_off = sector * 2;
        let entry = |file: &mut Vec<u8>,
                     idx: usize,
                     name: &str,
                     ty: u8,
                     child: u32,
                     start: u32,
                     size: u64| {
            let base = dir_off + idx * 128;
            let units: Vec<u16> = name.encode_utf16().collect();
            for (i, u) in units.iter().enumerate() {
                file[base + i * 2..base + i * 2 + 2].copy_from_slice(&u.to_le_bytes());
            }
            file[base + 64..base + 66]
                .copy_from_slice(&(((units.len() + 1) * 2) as u16).to_le_bytes());
            file[base + 66] = ty;
            file[base + 68..base + 72].copy_from_slice(&NO_STREAM.to_le_bytes());
            file[base + 72..base + 76].copy_from_slice(&NO_STREAM.to_le_bytes());
            file[base + 76..base + 80].copy_from_slice(&child.to_le_bytes());
            file[base + 116..base + 120].copy_from_slice(&start.to_le_bytes());
            file[base + 120..base + 128].copy_from_slice(&size.to_le_bytes());
        };
        entry(&mut file, 0, "Root Entry", 5, 1, 0, 0);
        entry(&mut file, 1, "Big", 2, NO_STREAM, 2, 5000);
        // Big stream at sectors 2-3 (file offsets 12288..).
        for i in 0..5000 {
            file[sector * 3 + i] = b'B';
        }
        file
    }

    fn fat_entry_mut(file: &mut [u8], idx: usize) -> &mut [u8] {
        let off = 512 + idx * 4;
        &mut file[off..off + 4]
    }

    #[test]
    fn cfb_reads_by_path() {
        let cfb = Cfb::open(build_test_cfb()).expect("open");
        let names = cfb.stream_names();
        assert!(names.iter().any(|n| n == "Big"), "{names:?}");
        assert!(names.iter().any(|n| n == "CONTENTS"), "{names:?}");
        let big = cfb.read_stream_path(&["Big"]).expect("big");
        assert_eq!(big.len(), 5000);
        assert!(big.iter().all(|b| *b == b'A'));
        // Case-insensitive, per the spec's name comparison.
        let small = cfb.read_stream_path(&["small"]).expect("small");
        assert_eq!(small, b"hello world");
        // The nested Quill text stream, by its full path only:
        // the same leaf name means nothing at the root.
        let quill = cfb
            .read_stream_path(&["Quill", "QuillSub", "CONTENTS"])
            .expect("quill");
        assert_eq!(quill, b"quill text payload!!");
        assert!(cfb.read_stream_path(&["CONTENTS"]).is_err());
        assert!(cfb.read_stream_path(&["Quill", "CONTENTS"]).is_err());
        assert!(cfb.read_stream_path(&["Big", "Small"]).is_err());
        assert!(cfb.read_stream_path(&["Missing"]).is_err());
    }

    #[test]
    fn cfb_reads_v4() {
        let cfb = Cfb::open(build_test_cfb_v4()).expect("open v4");
        let big = cfb.read_stream_path(&["Big"]).expect("big");
        assert_eq!(big.len(), 5000);
        assert!(big.iter().all(|b| *b == b'B'));
    }

    #[test]
    fn cfb_rejects_non_cfb() {
        assert!(Cfb::open(b"not a compound file at all".to_vec()).is_err());
        assert!(Cfb::open(vec![0u8; 100]).is_err());
    }

    #[test]
    fn cfb_rejects_bad_headers() {
        // A sector shift that is not the one the major version
        // fixes (the old code would shift by it, or panic).
        let mut f = build_test_cfb();
        f[30..32].copy_from_slice(&10u16.to_le_bytes());
        assert!(Cfb::open(f).is_err());
        let mut f = build_test_cfb();
        f[30..32].copy_from_slice(&0xFFFFu16.to_le_bytes());
        assert!(Cfb::open(f).is_err());
        // A major version other than 3 or 4 used to slip past
        // the size check entirely.
        let mut f = build_test_cfb();
        f[26..28].copy_from_slice(&5u16.to_le_bytes());
        assert!(Cfb::open(f).is_err());
        // Byte order must be 0xFFFE.
        let mut f = build_test_cfb();
        f[28..30].copy_from_slice(&0xFEFFu16.to_le_bytes());
        assert!(Cfb::open(f).is_err());
        // Mini sector shift must be 6.
        let mut f = build_test_cfb();
        f[32..34].copy_from_slice(&5u16.to_le_bytes());
        assert!(Cfb::open(f).is_err());
        // Mini cutoff must be 4096 (the pre-hardening fixture
        // used 64; the spec does not allow it).
        let mut f = build_test_cfb();
        f[56..60].copy_from_slice(&64u32.to_le_bytes());
        assert!(Cfb::open(f).is_err());
        // Entry 0 must be the root.
        let mut f = build_test_cfb();
        f[1024 + 66] = 1;
        assert!(Cfb::open(f).is_err());
    }

    #[test]
    fn cfb_rejects_difat_cycle() {
        let mut f = build_test_cfb();
        // Point the DIFAT chain at sector 14 and make sector
        // 14 point at itself.
        f[68..72].copy_from_slice(&14u32.to_le_bytes());
        f[72..76].copy_from_slice(&4u32.to_le_bytes());
        let sec_off = 512 * 15;
        for i in 0..128 {
            let v = if i == 127 { 14 } else { FREE_SECT };
            f[sec_off + i * 4..sec_off + i * 4 + 4].copy_from_slice(&v.to_le_bytes());
        }
        assert!(Cfb::open(f).is_err());
    }

    #[test]
    fn cfb_rejects_short_and_broken_chains() {
        // Big declares 5000 bytes; end its chain after two
        // sectors and the read must fail, not truncate.
        let mut f = build_test_cfb();
        fat_entry_mut(&mut f, 4).copy_from_slice(&END_OF_CHAIN.to_le_bytes());
        let cfb = Cfb::open(f).expect("open");
        assert!(cfb.read_stream_path(&["Big"]).is_err());
        // A FREESECT in the middle of a chain is corruption.
        let mut f = build_test_cfb();
        fat_entry_mut(&mut f, 4).copy_from_slice(&FREE_SECT.to_le_bytes());
        let cfb = Cfb::open(f).expect("open");
        assert!(cfb.read_stream_path(&["Big"]).is_err());
        // A chain that loops back on itself.
        let mut f = build_test_cfb();
        fat_entry_mut(&mut f, 5).copy_from_slice(&3u32.to_le_bytes());
        let cfb = Cfb::open(f).expect("open");
        assert!(cfb.read_stream_path(&["Big"]).is_err());
    }

    #[test]
    fn cfb_small_stream_without_minifat_is_an_error() {
        let mut f = build_test_cfb();
        f[60..64].copy_from_slice(&END_OF_CHAIN.to_le_bytes());
        f[64..68].copy_from_slice(&0u32.to_le_bytes());
        let cfb = Cfb::open(f).expect("open");
        assert!(cfb.read_stream_path(&["Small"]).is_err());
        // A big stream in the same file still reads.
        assert_eq!(cfb.read_stream_path(&["Big"]).expect("big").len(), 5000);
    }

    #[test]
    fn cfb_rejects_truncated_files() {
        let f = build_test_cfb();
        // Cut inside the second directory sector.
        assert!(Cfb::open(f[..4000].to_vec()).is_err());
        // Cut inside the mini stream container.
        assert!(Cfb::open(f[..1800].to_vec()).is_err());
        // Shorter than a header.
        assert!(Cfb::open(f[..300].to_vec()).is_err());
    }
}
