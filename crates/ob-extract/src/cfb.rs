//! A reader for the OLE Compound File Binary container
//! (Microsoft's published [MS-CFB] format): the envelope every
//! legacy Office binary file ships in, Publisher included. This
//! is our own implementation from the container specification,
//! written for the native .pub reader (Jacob's directive:
//! self-contained, our own ownership). All parsing is
//! bounds-checked and capped: these files are strangers.

const SIGNATURE: [u8; 8] = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
const END_OF_CHAIN: u32 = 0xFFFF_FFFE;
const FREE_SECT: u32 = 0xFFFF_FFFF;
#[allow(dead_code)]
const NO_STREAM: u32 = 0xFFFF_FFFF;
const MAX_STREAM: u64 = 256 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct DirEntry {
    pub name: String,
    /// 1 = storage (folder), 2 = stream (file), 5 = root.
    pub object_type: u8,
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

impl Cfb {
    pub fn open(data: Vec<u8>) -> Result<Cfb, String> {
        if data.len() < 512 || data[0..8] != SIGNATURE {
            return Err("that is not an OLE compound file (the envelope Publisher uses)".into());
        }
        let major = u16le(&data, 26)?;
        let sector_shift = u16le(&data, 30)?;
        let sector_size = 1usize << sector_shift;
        if major == 3 && sector_size != 512 || major == 4 && sector_size != 4096 {
            return Err("that compound file has an unusual sector size".into());
        }
        let mini_shift = u16le(&data, 32)?;
        let mini_sector_size = 1usize << mini_shift;
        let num_fat = u32le(&data, 44)? as usize;
        let first_dir = u32le(&data, 48)?;
        let mini_cutoff = u32le(&data, 56)? as u64;
        let first_minifat = u32le(&data, 60)?;
        let num_minifat = u32le(&data, 64)? as usize;
        let first_difat = u32le(&data, 68)?;
        let num_difat = u32le(&data, 72)? as usize;

        let sector = |n: u32| -> Result<&[u8], String> {
            let start = (n as usize + 1) * sector_size;
            data.get(start..start + sector_size)
                .ok_or_else(|| "compound file points outside itself".to_string())
        };

        // DIFAT: 109 entries in the header, then a chain.
        let mut fat_sectors: Vec<u32> = Vec::new();
        for i in 0..109 {
            let v = u32le(&data, 76 + i * 4)?;
            if v != FREE_SECT {
                fat_sectors.push(v);
            }
        }
        let mut next_difat = first_difat;
        for _ in 0..num_difat {
            if next_difat >= END_OF_CHAIN - 1 {
                break;
            }
            let sec = sector(next_difat)?;
            let per = sector_size / 4 - 1;
            for i in 0..per {
                let v = u32le(sec, i * 4)?;
                if v != FREE_SECT {
                    fat_sectors.push(v);
                }
            }
            next_difat = u32le(sec, per * 4)?;
        }
        if fat_sectors.len() < num_fat {
            return Err("that compound file's sector table is incomplete".into());
        }
        let mut fat: Vec<u32> = Vec::new();
        for s in fat_sectors.iter().take(num_fat) {
            let sec = sector(*s)?;
            for i in 0..sector_size / 4 {
                fat.push(u32le(sec, i * 4)?);
            }
        }

        let chain = |start: u32, fat: &[u32]| -> Result<Vec<u32>, String> {
            let mut out = Vec::new();
            let mut cur = start;
            while cur < END_OF_CHAIN - 1 {
                out.push(cur);
                if out.len() > fat.len() + 1 {
                    return Err("that compound file has a looping sector chain".into());
                }
                cur = *fat
                    .get(cur as usize)
                    .ok_or("compound file chain runs off its table")?;
            }
            Ok(out)
        };

        // Directory: a stream of 128-byte entries.
        let mut dir_bytes = Vec::new();
        for s in chain(first_dir, &fat)? {
            dir_bytes.extend_from_slice(sector(s)?);
        }
        let mut entries: Vec<DirEntry> = Vec::new();
        for chunk in dir_bytes.chunks_exact(128) {
            let object_type = chunk[66];
            if object_type == 0 {
                continue;
            }
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
                start_sector: u32le(chunk, 116)?,
                size,
            });
        }
        let root = entries
            .iter()
            .find(|e| e.object_type == 5)
            .cloned()
            .ok_or("that compound file has no root entry")?;

        // The mini stream is the root entry's own stream.
        let mut mini_stream = Vec::new();
        if root.size > 0 {
            if root.size > MAX_STREAM {
                return Err("that compound file's mini stream is implausibly large".into());
            }
            for s in chain(root.start_sector, &fat)? {
                mini_stream.extend_from_slice(sector(s)?);
            }
            mini_stream.truncate(root.size as usize);
        }
        let mut minifat: Vec<u32> = Vec::new();
        if num_minifat > 0 && first_minifat < END_OF_CHAIN - 1 {
            for s in chain(first_minifat, &fat)? {
                let sec = sector(s)?;
                for i in 0..sector_size / 4 {
                    minifat.push(u32le(sec, i * 4)?);
                }
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

    /// Follow a FAT chain and gather the bytes (regular stream).
    fn read_chain(&self, start: u32, size: u64) -> Result<Vec<u8>, String> {
        if size > MAX_STREAM {
            return Err("a stream in that file is implausibly large".into());
        }
        let mut out = Vec::new();
        let mut cur = start;
        let max_steps = self.fat.len() + 1;
        let mut steps = 0usize;
        while cur < END_OF_CHAIN - 1 {
            steps += 1;
            if steps > max_steps {
                return Err("that compound file has a looping sector chain".into());
            }
            let off = (cur as usize + 1) * self.sector_size;
            let sec = self
                .data
                .get(off..off + self.sector_size)
                .ok_or("compound file points outside itself")?;
            out.extend_from_slice(sec);
            cur = *self
                .fat
                .get(cur as usize)
                .ok_or("compound file chain runs off its table")?;
        }
        out.truncate(size as usize);
        Ok(out)
    }

    /// Follow a miniFAT chain inside the mini stream.
    fn read_mini_chain(&self, start: u32, size: u64) -> Result<Vec<u8>, String> {
        let mut out = Vec::new();
        let mut cur = start;
        let max_steps = self.minifat.len() + 1;
        let mut steps = 0usize;
        while cur < END_OF_CHAIN - 1 {
            steps += 1;
            if steps > max_steps {
                return Err("that compound file has a looping mini chain".into());
            }
            let off = cur as usize * self.mini_sector_size;
            let sec = self
                .mini_stream
                .get(off..off + self.mini_sector_size)
                .ok_or("compound file mini stream is truncated")?;
            out.extend_from_slice(sec);
            cur = *self
                .minifat
                .get(cur as usize)
                .ok_or("compound file mini chain runs off its table")?;
        }
        out.truncate(size as usize);
        Ok(out)
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

    /// Read one stream by name (case-insensitive).
    pub fn read_stream(&self, name: &str) -> Result<Vec<u8>, String> {
        let entry = self
            .entries
            .iter()
            .find(|e| e.object_type == 2 && e.name.eq_ignore_ascii_case(name))
            .ok_or_else(|| format!("that file has no stream named {name}"))?;
        if entry.size >= self.mini_cutoff || self.minifat.is_empty() {
            self.read_chain(entry.start_sector, entry.size)
        } else {
            self.read_mini_chain(entry.start_sector, entry.size)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal CFB v3 writer for tests: header, FAT,
    /// directory, mini stream, miniFAT, and streams laid out as
    /// real files lay them:  sector 0 FAT, 1 directory, 2 mini
    /// stream container, 3-4 a two-sector regular stream,
    /// 5 miniFAT.
    fn build_test_cfb() -> Vec<u8> {
        let sector = 512usize;
        let mut file = vec![0u8; sector * 7];
        // Header
        file[0..8].copy_from_slice(&SIGNATURE);
        file[24..26].copy_from_slice(&3u16.to_le_bytes());
        file[26..28].copy_from_slice(&3u16.to_le_bytes()); // major v3
        file[28..30].copy_from_slice(&0xFFFEu16.to_le_bytes());
        file[30..32].copy_from_slice(&9u16.to_le_bytes()); // 512-byte sectors
        file[32..34].copy_from_slice(&6u16.to_le_bytes()); // 64-byte mini sectors
        file[44..48].copy_from_slice(&1u32.to_le_bytes()); // one FAT sector
        file[48..52].copy_from_slice(&1u32.to_le_bytes()); // first dir sector = 1
        file[56..60].copy_from_slice(&64u32.to_le_bytes()); // mini cutoff (small, so the fixture stays tiny)
        file[60..64].copy_from_slice(&5u32.to_le_bytes()); // first miniFAT = 5
        file[64..68].copy_from_slice(&1u32.to_le_bytes()); // one miniFAT sector
        file[68..72].copy_from_slice(&END_OF_CHAIN.to_le_bytes());
        file[72..76].copy_from_slice(&0u32.to_le_bytes());
        file[76..80].copy_from_slice(&0u32.to_le_bytes()); // DIFAT[0] = FAT at 0
        for i in 1..109 {
            file[76 + i * 4..80 + i * 4].copy_from_slice(&FREE_SECT.to_le_bytes());
        }
        // FAT at file offset 512: 0=FATSECT 1=END 2=END 3->4 4=END 5=END
        let fat_off = sector;
        let fat_entries = [
            0xFFFF_FFFDu32,
            END_OF_CHAIN,
            END_OF_CHAIN,
            4,
            END_OF_CHAIN,
            END_OF_CHAIN,
        ];
        for i in 0..sector / 4 {
            let v = fat_entries.get(i).copied().unwrap_or(FREE_SECT);
            file[fat_off + i * 4..fat_off + i * 4 + 4].copy_from_slice(&v.to_le_bytes());
        }
        // MiniFAT at sector 5 (file offset 6*512): mini 0 -> END.
        let mf_off = sector * 6;
        file[mf_off..mf_off + 4].copy_from_slice(&END_OF_CHAIN.to_le_bytes());
        for i in 1..sector / 4 {
            file[mf_off + i * 4..mf_off + i * 4 + 4].copy_from_slice(&FREE_SECT.to_le_bytes());
        }
        // Directory at sector 1 (file offset 2*512).
        let dir_off = sector * 2;
        let entry = |file: &mut Vec<u8>, idx: usize, name: &str, ty: u8, start: u32, size: u32| {
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
            file[base + 76..base + 80].copy_from_slice(&NO_STREAM.to_le_bytes());
            file[base + 116..base + 120].copy_from_slice(&start.to_le_bytes());
            file[base + 120..base + 124].copy_from_slice(&size.to_le_bytes());
        };
        entry(&mut file, 0, "Root Entry", 5, 2, 64);
        entry(&mut file, 1, "Big", 2, 3, 1000);
        entry(&mut file, 2, "Small", 2, 0, 11);
        // Mini stream at sector 2 (file offset 3*512).
        let mini_off = sector * 3;
        file[mini_off..mini_off + 11].copy_from_slice(b"hello world");
        // Big stream at sectors 3 and 4 (file offsets 4*512, 5*512).
        for i in 0..sector * 2 {
            file[sector * 4 + i] = b'A';
        }
        file
    }

    #[test]
    fn cfb_reads_regular_and_mini_streams() {
        let cfb = Cfb::open(build_test_cfb()).expect("open");
        let names = cfb.stream_names();
        assert!(names.iter().any(|n| n == "Big"), "{names:?}");
        assert!(names.iter().any(|n| n == "Small"), "{names:?}");
        let big = cfb.read_stream("Big").expect("big");
        assert_eq!(big.len(), 1000);
        assert!(big.iter().all(|b| *b == b'A'));
        let small = cfb.read_stream("small").expect("small"); // case-insensitive
        assert_eq!(small, b"hello world");
    }

    #[test]
    fn cfb_rejects_non_cfb() {
        assert!(Cfb::open(b"not a compound file at all".to_vec()).is_err());
        assert!(Cfb::open(vec![0u8; 100]).is_err());
    }
}
