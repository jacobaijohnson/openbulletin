//! The .obd file format (docs/FORMAT.md).
//!
//! v1: a zip container holding `document.json` (the same
//! document JSON v0 wrote bare) plus one entry per asset,
//! `assets/<sha256>`, so the pictures a bulletin carries
//! travel inside the file and arrive identically on the next
//! volunteer's machine.
//!
//! v0: a single JSON document. Readers accept every older
//! version and migrate in memory: the pre-occasions field
//! keys are renamed, and a panel's tracing picture that
//! names a sidecar file beside the .obd is read into the
//! asset store. Saving always writes v1.
//!
//! Saves are atomic (temp file + rename) in both versions.

use ob_model::{Asset, AssetKind, Document, Insert, RightsRecord};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::{Read, Seek, Write};
use std::path::Path;

/// document.json stays small; refuse absurd sizes before
/// reading them into memory (closing audit).
const MAX_JSON_BYTES: u64 = 64 * 1024 * 1024;
/// Everything inside a v1 file, uncompressed, stays under
/// this. A bulletin with its pictures is megabytes, not
/// hundreds of megabytes.
const MAX_TOTAL_UNCOMPRESSED: u64 = 512 * 1024 * 1024;

/// The first four bytes of a zip container ("PK" + 0x03 0x04).
const ZIP_MAGIC: [u8; 4] = [b'P', b'K', 0x03, 0x04];

/// The words for a container whose actual insides pass
/// the total cap, whichever container it is.
const TOTAL_TOO_LARGE: &str = "That file's insides are too large to read safely.";

/// The most entries one container may hold (round 4,
/// RT-2). The byte caps count what the entries weigh;
/// this counts how many of them there are. A file of
/// 100,001 tiny assets sat under every byte cap and
/// still took minutes to open. A real bulletin or
/// sheet holds a handful of parts; 8,192 is far past
/// generous, and the count is the archive's raw entry
/// count, entries that share a name included (round 5,
/// RT5-5), read from its central directory before any
/// payload is read.
const MAX_ENTRIES: usize = 8_192;

/// The words for a container holding more parts than
/// the entry cap.
const TOO_MANY_PARTS: &str = "That file has too many parts inside to read safely.";

/// Read one zip entry's bytes through a cap, counting
/// ACTUAL bytes as they arrive (round 3, RT3-B). The
/// loaders used to cap by summing the sizes the file's
/// own directory declares, and a directory can lie: a
/// 612 KB file whose entries claimed 5 bytes expanded
/// to 600 MB in memory on open, because zip streams are
/// not bound by their declared size. The reading itself
/// is the count now, the IDML importer's discipline:
/// one byte past the cap is read, so an entry exactly
/// at the cap still counts as inside; anything more
/// refuses in plain words, and memory never holds more
/// than the cap plus that one byte for this entry.
fn read_entry_capped(
    entry: &mut zip::read::ZipFile,
    cap: u64,
    too_large: &str,
) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    entry
        .by_ref()
        .take(cap + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > cap {
        return Err(too_large.to_string());
    }
    Ok(bytes)
}

/// The text variant of `read_entry_capped`, for the
/// container's JSON entry.
fn read_entry_text_capped(
    entry: &mut zip::read::ZipFile,
    cap: u64,
    too_large: &str,
) -> Result<String, String> {
    let bytes = read_entry_capped(entry, cap, too_large)?;
    String::from_utf8(bytes).map_err(|_| "that file does not hold text we can read".to_string())
}

/// Read `buf` at absolute offset `at`.
fn read_at(file: &mut std::fs::File, at: u64, buf: &mut [u8]) -> Option<()> {
    use std::io::{Read, Seek, SeekFrom};
    file.seek(SeekFrom::Start(at)).ok()?;
    file.read_exact(buf).ok()?;
    Some(())
}

/// One first-signature query for
/// [`resolve_first_hits`]: the first place `sig`
/// occurs in `[from, to)`, or None when it does not
/// occur there.
struct FirstHit {
    sig: [u8; 4],
    from: u64,
    to: u64,
    answer: Option<u64>,
}

/// Resolve every query in `queries` with ONE pass
/// over the file region the queries span
/// (round 9, R9-2). The shipped code searched that
/// region once PER end-record candidate, so a zip
/// comment carrying thousands of end-record
/// signatures paid a full scan for each of them and
/// a crafted 1 MB file took minutes to open. Here
/// the region is read once, in chunks; each query
/// is answered by the first signature at or after
/// its own start, exactly as a search of its own
/// window would answer it, and the pass stops as
/// soon as every query is answered.
fn resolve_first_hits(file: &mut std::fs::File, queries: &mut [FirstHit]) {
    let mut order: Vec<usize> = (0..queries.len())
        .filter(|&i| queries[i].from < queries[i].to)
        .collect();
    if order.is_empty() {
        return;
    }
    let scan_start = order.iter().map(|&i| queries[i].from).min().unwrap_or(0);
    let scan_end = order.iter().map(|&i| queries[i].to).max().unwrap_or(0);
    if scan_start >= scan_end {
        return;
    }
    order.sort_by_key(|&i| (queries[i].sig, queries[i].from));
    // Contiguous groups of one signature in `order`,
    // each with a pointer to its first unanswered
    // query. Queries are answered in `from` order
    // within a group: the first chunk holding a
    // signature at or after a query's start answers
    // it (an earlier chunk either held no signature
    // or held only signatures before its start, or
    // the query would already be answered).
    let mut groups: Vec<(usize, usize, usize)> = Vec::new();
    let mut start = 0usize;
    while start < order.len() {
        let mut end = start + 1;
        while end < order.len() && queries[order[end]].sig == queries[order[start]].sig {
            end += 1;
        }
        groups.push((start, end, start));
        start = end;
    }
    let mut pos = scan_start;
    while pos < scan_end && groups.iter().any(|&(_, end, ptr)| ptr < end) {
        let want = ((scan_end - pos).min(65_536)) as usize;
        let read_end = (pos + want as u64 + 3).min(scan_end);
        let mut hay = vec![0u8; (read_end - pos) as usize];
        if read_at(file, pos, &mut hay).is_none() {
            return;
        }
        for (gstart, gend, ptr) in groups.iter_mut() {
            if *ptr >= *gend {
                continue;
            }
            let sig = queries[order[*gstart]].sig;
            // Signature starts inside this chunk's own
            // span, [pos, pos + want): the three read-
            // ahead bytes let a signature starting at
            // the span's last bytes still be seen whole.
            let mut positions: Vec<u64> = Vec::new();
            for i in 0..want {
                if i + 4 > hay.len() {
                    break;
                }
                if hay[i..].starts_with(&sig[..]) {
                    positions.push(pos + i as u64);
                }
            }
            if positions.is_empty() {
                continue;
            }
            let last = *positions.last().unwrap_or(&0);
            while *ptr < *gend {
                let qi = order[*ptr];
                if queries[qi].from > last {
                    break;
                }
                let idx = positions.partition_point(|&p| p < queries[qi].from);
                let found = positions[idx];
                queries[qi].answer = if found + 4 <= queries[qi].to {
                    Some(found)
                } else {
                    None
                };
                *ptr += 1;
            }
        }
        pos += want as u64;
    }
}

/// One directory's record chain, walked once from
/// its first record: every offset the chain reaches
/// (the start itself, then the end of each record,
/// each measured by its own name, extra, and
/// comment lengths), plus the offset where the
/// walk passed `cap`, when it did. Every candidate
/// naming this directory reads its count off the
/// one chain (round 12, R12-3/R12-5): a candidate
/// counts exactly when the chain reaches its own
/// end — or passes the cap before reaching it, the
/// standing early stop — and candidates whose end
/// the chain never reaches do not count at all.
/// The walk also builds the chain's position index
/// once — offset to the count of records chained
/// at that offset — so reading one candidate's
/// count off the chain is a lookup, never another
/// scan of the chain (round 13, R13-3): with the
/// scan, every candidate paid the chain's full
/// length again, and the shared pass cost
/// O(candidates × chain length) instead of
/// O(span + distinct chains + candidates).
struct ChainWalk {
    positions: Vec<u64>,
    index: HashMap<u64, usize>,
    capped_at: Option<u64>,
}

impl ChainWalk {
    fn new(positions: Vec<u64>, capped_at: Option<u64>) -> ChainWalk {
        let index: HashMap<u64, usize> = positions
            .iter()
            .enumerate()
            .map(|(count, &at)| (at, count))
            .collect();
        ChainWalk {
            positions,
            index,
            capped_at,
        }
    }

    /// The count of records chained at `at`, when
    /// the chain reaches that offset at all.
    fn count_at(&self, at: u64) -> Option<usize> {
        self.index.get(&at).copied()
    }

    /// Whether the chain reaches `at` at all.
    fn reaches(&self, at: u64) -> bool {
        self.index.contains_key(&at)
    }

    /// Whether the chain parses exactly `total`
    /// records from its start — the reader's own
    /// condition (round 13, R13-4): `positions[k]`
    /// stands exactly when the first `k` records
    /// chained, capped walks included, because a
    /// walk that passed the cap walked `cap + 1`
    /// records to get there.
    fn parses_total(&self, total: u64) -> bool {
        total < self.positions.len() as u64
    }
}

/// Walk the record chain from `cd_start` out to
/// `extent` (the farthest end any candidate naming
/// this start describes), stopping early as soon
/// as the count passes `cap` or the records stop
/// chaining.
fn walk_chain_file(file: &mut std::fs::File, cd_start: u64, extent: u64, cap: usize) -> ChainWalk {
    let mut positions: Vec<u64> = vec![cd_start];
    let mut capped_at = None;
    let mut count = 0usize;
    let mut at = cd_start;
    while at < extent {
        if count > cap {
            capped_at = Some(at);
            break;
        }
        let mut header = [0u8; 46];
        if read_at(file, at, &mut header).is_none() {
            break;
        }
        if !header.starts_with(b"PK\x01\x02") {
            break;
        }
        let name_len = u64::from(u16::from_le_bytes([header[28], header[29]]));
        let extra_len = u64::from(u16::from_le_bytes([header[30], header[31]]));
        let comment_len = u64::from(u16::from_le_bytes([header[32], header[33]]));
        count += 1;
        at += 46 + name_len + extra_len + comment_len;
        positions.push(at);
    }
    ChainWalk::new(positions, capped_at)
}

/// The count one candidate reads off a shared
/// chain: the number of records chaining exactly
/// to its `dir_end`, or `cap + 1` when the walk
/// passed the cap at or before that end. None when
/// the chain never reaches the end — the records
/// broke, or stopped short of it. The count is a
/// lookup in the chain's position index, built
/// once when the chain was walked (round 13,
/// R13-3), never a scan of the chain per candidate.
fn chain_count(chain: &ChainWalk, dir_end: u64, cap: usize) -> Option<usize> {
    if let Some(count) = chain.count_at(dir_end) {
        return Some(count);
    }
    if let Some(capped_at) = chain.capped_at {
        if dir_end >= capped_at {
            return Some(cap + 1);
        }
    }
    None
}

/// The same count read off a chain walked over
/// in-memory bytes: the records chaining exactly
/// to `dir_end` in the byte chain starting at
/// `cd_start`, with the same cap stop. The shared
/// fallback resolution builds these chains over
/// `bytes` (round 12, R12-5).
fn walk_chain_bytes(bytes: &[u8], cd_start: u64, extent: u64, cap: usize) -> ChainWalk {
    let mut positions: Vec<u64> = vec![cd_start];
    let mut capped_at = None;
    let mut count = 0usize;
    let mut at = cd_start;
    while at < extent {
        if count > cap {
            capped_at = Some(at);
            break;
        }
        let start = at as usize;
        if start + 46 > bytes.len() || !bytes[start..].starts_with(b"PK\x01\x02") {
            break;
        }
        let header = &bytes[start..start + 46];
        let name_len = u64::from(u16::from_le_bytes([header[28], header[29]]));
        let extra_len = u64::from(u16::from_le_bytes([header[30], header[31]]));
        let comment_len = u64::from(u16::from_le_bytes([header[32], header[33]]));
        count += 1;
        at += 46 + name_len + extra_len + comment_len;
        positions.push(at);
    }
    ChainWalk::new(positions, capped_at)
}

/// The tail window where end records can live: the
/// trailing comment space (at most 65,535 bytes)
/// plus the end structures — the end record itself,
/// a zip64 locator, and a zip64 end record. End
/// records outside this window are not candidates
/// the format's own search would find.
const TAIL_WINDOW: u64 = 65_535 + 22 + 20 + 56;

/// The directory one candidate resolves to, named
/// by the directory it DESCRIBES, never by where
/// its own end record stands (round 12, R12-3):
/// the resolved directory start (None when no
/// central header stands in its window), the
/// directory's size as the resolving record
/// declares it (the zip64 record's size where the
/// candidate resolves through one, else the plain
/// record's size field), the zip64 record it
/// resolves through where one stands, and — only
/// when no directory start resolved, where the
/// declared total is all that separates an honestly
/// empty candidate from an unwalkable one — that
/// total. Two candidates carrying the same key
/// describe the same directory, so the second is a
/// repeat, not another directory (round 10, R10-1):
/// two plain end records at different positions
/// declaring one directory share a key now, where
/// the old key's end offset was each plain
/// candidate's own position and no two plain
/// candidates ever shared one.
type DirKey = (Option<u64>, u64, Option<u64>, u64);

/// One TWIN's reading of a directory: its resolved
/// key plus the total it declares (round 13,
/// R13-4). The key deliberately drops the declared
/// total — identity is the directory described —
/// but the reader's reading of that directory is
/// driven by the total, so two candidates under one
/// key with different totals are different readings:
/// a zero-total twin reads the directory as empty
/// where its honest twin reads it whole. The
/// loaders' dedupe is by twin, not by key alone: a
/// key whose representative yields nothing has its
/// other twins tried, each distinct total once,
/// before the load refuses.
type TwinKey = (DirKey, u64);

/// Every end-record candidate in a file whose
/// directory can be walked, latest first, as
/// (candidate position, raw entry count, resolved
/// directory) triples. Candidates are collected by
/// ONE scan of the tail window; every candidate's
/// directory start (and any zip64 record position
/// its locator names) is resolved in one shared
/// pass per signature over the region those
/// windows span (round 9, R9-2); the per-candidate
/// directory walks then run as they always have,
/// each stopping just past `cap` — except that a
/// directory is walked at most once, however many
/// candidates resolve to it (round 10, R10-1):
/// appended candidates can all point at one shared
/// directory just under the cap, and walking it per
/// candidate multiplied the whole walk by the
/// candidate count.
fn directory_candidates(path: &Path, cap: usize) -> Vec<(u64, usize, DirKey)> {
    let mut out: Vec<(u64, usize, DirKey)> = Vec::new();
    let Ok(mut file) = std::fs::File::open(path) else {
        return out;
    };
    let Ok(meta) = file.metadata() else {
        return out;
    };
    let file_len = meta.len();
    if file_len < 22 {
        return out;
    }
    // One scan of the tail window collects every
    // candidate position, latest first, under the
    // reader's relaxed rule: a "PK\x05\x06" whose
    // comment length fits before the end of the file
    // (the comment may end early — trailing junk).
    // EVERY candidate in the window is collected
    // (round 8, R8-1): an earlier bound of sixteen
    // let fifteen junk records push a flood's own
    // end record past the enumeration, so the cap
    // never counted it.
    let window_start = file_len.saturating_sub(TAIL_WINDOW);
    let window_len = (file_len - window_start) as usize;
    let mut window = vec![0u8; window_len];
    if read_at(&mut file, window_start, &mut window).is_none() {
        return out;
    }
    let mut positions: Vec<u64> = Vec::new();
    if window.len() >= 22 {
        for pos in (0..=window.len() - 22).rev() {
            if !window[pos..].starts_with(b"PK\x05\x06") {
                continue;
            }
            let at = window_start + pos as u64;
            let comment_len = u64::from(u16::from_le_bytes([window[pos + 20], window[pos + 21]]));
            if at + 22 + comment_len <= file_len {
                positions.push(at);
            }
        }
    }
    // Stage 1: parse each candidate's end record.
    // A zip64 locator in front of the record points
    // at the zip64 record, which carries the
    // directory's real offset when the plain fields
    // are too small to hold it (its total-entries
    // field sits at byte 32, its offset at 48). The
    // reader finds that record by searching forward
    // from the locator's declared offset; each such
    // search becomes one query, resolved together
    // below, not one scan per candidate.
    struct Candidate {
        at: u64,
        total16: u16,
        cd_size32: u32,
        cd_offset32: u32,
        zip64_query: Option<usize>,
        locator_at: Option<u64>,
    }
    let mut candidates: Vec<Candidate> = Vec::new();
    let mut queries: Vec<FirstHit> = Vec::new();
    for at in positions {
        let mut eocd = [0u8; 22];
        if read_at(&mut file, at, &mut eocd).is_none() || !eocd.starts_with(b"PK\x05\x06") {
            continue;
        }
        let total16 = u16::from_le_bytes([eocd[10], eocd[11]]);
        let cd_size32 = u32::from_le_bytes(eocd[12..16].try_into().unwrap_or([0; 4]));
        let cd_offset32 = u32::from_le_bytes(eocd[16..20].try_into().unwrap_or([0; 4]));
        let mut zip64_query = None;
        let mut locator_at = None;
        if (total16 == u16::MAX || cd_offset32 == u32::MAX) && at >= 20 {
            let lat = at - 20;
            let mut locator = [0u8; 20];
            if read_at(&mut file, lat, &mut locator).is_some() && locator.starts_with(b"PK\x06\x07")
            {
                let record_declared =
                    u64::from_le_bytes(locator[8..16].try_into().unwrap_or([0; 8]));
                if record_declared < lat {
                    zip64_query = Some(queries.len());
                    queries.push(FirstHit {
                        sig: *b"PK\x06\x06",
                        from: record_declared,
                        to: lat,
                        answer: None,
                    });
                    locator_at = Some(lat);
                }
            }
        }
        candidates.push(Candidate {
            at,
            total16,
            cd_size32,
            cd_offset32,
            zip64_query,
            locator_at,
        });
    }
    if !queries.is_empty() {
        resolve_first_hits(&mut file, &mut queries);
    }
    // Stage 2: each candidate's directory window.
    // The zip64 record, when the shared pass found
    // it and it parses, retargets the window: the
    // directory ends where the record stands, and
    // the record's fields replace the sentinels.
    // The directory's real start is then the first
    // central header at or after the declared
    // offset: prepended bytes shift it forward, and
    // the declared totals (either field, either
    // record) are never consulted again (round 6,
    // RT6-7). Those searches are queries too, one
    // shared pass for every candidate.
    struct Windowed {
        at: u64,
        dir_end: u64,
        declared_total: u64,
        declared_size: u64,
        cd_query: Option<usize>,
        zip64_at: Option<u64>,
    }
    let mut windows: Vec<Windowed> = Vec::new();
    let mut cd_queries: Vec<FirstHit> = Vec::new();
    for cand in &candidates {
        let mut dir_end = cand.at;
        let mut declared_offset = u64::from(cand.cd_offset32);
        let mut declared_total = u64::from(cand.total16);
        let mut declared_size = u64::from(cand.cd_size32);
        let mut zip64_at = None;
        if let (Some(qi), Some(_)) = (cand.zip64_query, cand.locator_at) {
            if let Some(record_at) = queries[qi].answer {
                let mut record = [0u8; 56];
                if read_at(&mut file, record_at, &mut record).is_some()
                    && record.starts_with(b"PK\x06\x06")
                {
                    dir_end = record_at;
                    zip64_at = Some(record_at);
                    declared_total =
                        u64::from_le_bytes(record[32..40].try_into().unwrap_or([0; 8]));
                    declared_size = u64::from_le_bytes(record[40..48].try_into().unwrap_or([0; 8]));
                    declared_offset =
                        u64::from_le_bytes(record[48..56].try_into().unwrap_or([0; 8]));
                }
            }
        }
        let cd_query = if declared_offset < dir_end {
            let qi = cd_queries.len();
            cd_queries.push(FirstHit {
                sig: *b"PK\x01\x02",
                from: declared_offset,
                to: dir_end,
                answer: None,
            });
            Some(qi)
        } else {
            None
        };
        windows.push(Windowed {
            at: cand.at,
            dir_end,
            declared_total,
            declared_size,
            cd_query,
            zip64_at,
        });
    }
    if !cd_queries.is_empty() {
        resolve_first_hits(&mut file, &mut cd_queries);
    }
    // Stage 3: walk each DISTINCT directory once
    // (round 12, R12-3). The walk itself is shared
    // further than the key: every candidate resolving
    // to one directory START pays for one chain walk
    // from that start, out to the farthest end any of
    // them names, and each candidate's count is read
    // off the chain — a candidate counts exactly when
    // the chain reaches its own end, or passes the
    // cap trying. Candidates carrying one key are
    // repeats of one described directory: the first
    // of them (latest first) that the chain counts is
    // the one returned, and the rest are not walked,
    // counted, or returned again — the loaders below
    // open each returned candidate, so a repeat
    // returned here would be opened again there too.
    let mut extents: HashMap<u64, u64> = HashMap::new();
    for win in &windows {
        if let Some(cd_start) = win.cd_query.and_then(|qi| cd_queries[qi].answer) {
            let extent = extents.entry(cd_start).or_insert(0);
            *extent = (*extent).max(win.dir_end);
        }
    }
    let mut chains: HashMap<u64, ChainWalk> = HashMap::new();
    for (cd_start, extent) in &extents {
        let chain = walk_chain_file(&mut file, *cd_start, *extent, cap);
        chains.insert(*cd_start, chain);
    }
    let mut order: Vec<DirKey> = Vec::new();
    let mut best: HashMap<DirKey, Option<(u64, usize)>> = HashMap::new();
    for win in &windows {
        let cd_start = win.cd_query.and_then(|qi| cd_queries[qi].answer);
        let key: DirKey = (
            cd_start,
            win.declared_size,
            win.zip64_at,
            if cd_start.is_none() {
                win.declared_total
            } else {
                0
            },
        );
        if let std::collections::hash_map::Entry::Vacant(slot) = best.entry(key) {
            order.push(key);
            slot.insert(None);
        }
        let count = match cd_start {
            Some(cd_start) => chains
                .get(&cd_start)
                .and_then(|chain| chain_count(chain, win.dir_end, cap)),
            None => {
                // No directory records in front of the
                // end record: an honestly empty archive
                // counts zero; anything else is the
                // reader's error to report.
                if win.declared_total == 0 {
                    Some(0)
                } else {
                    None
                }
            }
        };
        if let Some(count) = count {
            let slot = best.get_mut(&key).expect("the key was just inserted");
            if slot.is_none() {
                *slot = Some((win.at, count));
            }
        }
    }
    for key in order {
        if let Some((at, count)) = best[&key] {
            out.push((at, count, key));
        }
    }
    out
}

/// How many entries a container's central directory
/// lists, counting every record as written: two entries
/// that share a name count twice (round 5, RT5-5).
/// `ZipArchive::len` cannot answer this, because the
/// zip crate keeps entries in a map keyed by name, so
/// its count is the number of distinct names, and a
/// file of half a million entries all named the same
/// reported a count of two.
///
/// The count is WALKED, never read from the
/// directory's own declared totals (round 6, RT6-7):
/// the end record's count fields and the directory's
/// declared size can both lie, in either direction,
/// in the plain record and the zip64 record alike,
/// and a walk that trusted either let floods through.
/// The end record is located the way the zip reader
/// itself locates it, tolerance included: candidates
/// are tried latest first, a comment that ends before
/// the end of the file is accepted (trailing junk),
/// and the directory's real start is the first central
/// header at or after the declared offset, so bytes
/// prepended to the archive shift every offset and
/// the count still lands on the real directory. From
/// there the records are counted one by one, each
/// measured by its own name, extra, and comment
/// lengths, until the end structures begin; no
/// entry's payload is read, and the walk stops as
/// soon as the count passes `cap`, so a flood is
/// refused after reading only the first `cap + 1`
/// small directory records. None when the directory
/// cannot be walked: a damaged file is the archive
/// reader's error to report, in its own words.
///
/// The cap judges the FILE, not the directory the
/// reader happens to use (round 7, RT7-5): trailing
/// junk can hold a second, fake directory and a fake
/// end record, and the reader's latest-first rule
/// reads the fake one, so a count that stopped at the
/// first walkable candidate counted the attacker's
/// directory and let a flood through. Every candidate
/// the file presents is counted here, each with the
/// same early stop, and a count past the cap from ANY
/// candidate is the answer. When no candidate passes
/// the cap, the answer is still the first (latest)
/// walkable candidate's count, exactly as the reader
/// itself would choose, so honest files with trailing
/// junk but no second directory count as they always
/// have.
///
/// The cost is one scan of the tail window where end
/// records can live, one shared pass over the region
/// the candidates' directory windows span, and the
/// bounded walks (round 9, R9-2): the round-8 code
/// searched the file once per candidate, so a comment
/// carrying thousands of end-record signatures made
/// opening take minutes on a 1 MB file.
fn raw_entry_count(path: &Path, cap: usize) -> Option<usize> {
    let mut first_count: Option<usize> = None;
    for (_, count, _) in directory_candidates(path, cap) {
        if count > cap {
            return Some(count);
        }
        if first_count.is_none() {
            first_count = Some(count);
        }
    }
    first_count
}

/// Refuse a container whose raw entry count passes the
/// cap, in the cap's own words. A count that cannot be
/// walked leaves the file to the archive reader, whose
/// distinct-name count still applies there.
fn check_entry_cap(path: &Path) -> Result<(), String> {
    match raw_entry_count(path, MAX_ENTRIES) {
        Some(n) if n > MAX_ENTRIES => Err(TOO_MANY_PARTS.to_string()),
        _ => Ok(()),
    }
}

/// Every end-record candidate position in `bytes`,
/// latest first, under the same rule the count walk
/// uses: a "PK\x05\x06" whose comment length fits
/// before the end of the file (round 8, R8-12).
fn end_record_positions_in(bytes: &[u8]) -> Vec<usize> {
    let mut out = Vec::new();
    if bytes.len() < 22 {
        return out;
    }
    for at in (0..=bytes.len() - 22).rev() {
        if !bytes[at..].starts_with(b"PK\x05\x06") {
            continue;
        }
        let comment_len = usize::from(u16::from_le_bytes([bytes[at + 20], bytes[at + 21]]));
        if at + 22 + comment_len <= bytes.len() {
            out.push(at);
        }
    }
    out
}

/// Resolve every query in `queries` with ONE pass
/// over the in-memory region the queries span: the
/// slice twin of [`resolve_first_hits`] (round 12,
/// R12-5). The region is scanned once per signature
/// the queries carry, every occurrence position is
/// collected in that one scan, and each query is
/// answered by the first occurrence at or after its
/// own start that ends inside its own window —
/// exactly what a search of its own window would
/// answer, at a total cost of O(span + candidates).
fn resolve_first_hits_in(bytes: &[u8], queries: &mut [FirstHit]) {
    let mut sigs: Vec<[u8; 4]> = Vec::new();
    for query in queries.iter() {
        if query.from < query.to && !sigs.contains(&query.sig) {
            sigs.push(query.sig);
        }
    }
    for sig in sigs {
        let scan_start = queries
            .iter()
            .filter(|q| q.sig == sig && q.from < q.to)
            .map(|q| q.from)
            .min()
            .unwrap_or(0);
        let scan_end = queries
            .iter()
            .filter(|q| q.sig == sig && q.from < q.to)
            .map(|q| q.to)
            .max()
            .unwrap_or(0);
        if scan_start >= scan_end || scan_end as usize > bytes.len() {
            continue;
        }
        let mut hits: Vec<u64> = Vec::new();
        let mut pos = scan_start as usize;
        let last_start = (scan_end as usize).saturating_sub(4);
        while pos <= last_start {
            if bytes[pos..].starts_with(&sig[..]) {
                hits.push(pos as u64);
            }
            pos += 1;
        }
        for query in queries.iter_mut() {
            if query.sig != sig || query.from >= query.to {
                continue;
            }
            let index = hits.partition_point(|&hit| hit < query.from);
            if let Some(&hit) = hits.get(index) {
                if hit + 4 <= query.to {
                    query.answer = Some(hit);
                }
            }
        }
    }
}

/// One candidate's resolution, computed by the
/// shared pass ([`candidate_resolutions_in`]): the
/// resolved directory's identity — the enumeration's
/// own [`DirKey`] — whether its records chain
/// exactly to the directory's end (`walkable`, the
/// count walk's rule), whether the archive reader
/// itself would read it (`reader_walkable`: the
/// chain parses exactly the candidate's declared
/// total of records from the resolved start, the
/// count the reader itself checks — it derives the
/// start the same way and parses that many central
/// headers in sequence, never consulting the
/// declared size, so a size lying by a few bytes
/// no longer closes the gate on a directory the
/// reader reads whole, bytes standing after the
/// directory — a gap, junk, further end records —
/// notwithstanding; round 13, R13-4), the total the
/// candidate declares (post-zip64-retarget, the
/// total its own open would present — the field
/// that separates one twin's reading from
/// another's under a shared key), and the
/// directory's exact record count when a chain
/// reaches the end it was measured to.
struct CandidateResolution {
    key: DirKey,
    walkable: bool,
    reader_walkable: bool,
    declared_total: u64,
    exact_count: Option<usize>,
}

/// Resolve EVERY end-record candidate in
/// `positions` in one shared pass (round 12,
/// R12-5), the way the count walk's enumeration
/// resolves its candidates: through each one's
/// zip64 record where the plain fields are
/// sentinels, then the first central header at or
/// after its declared offset, then its records
/// chaining toward the directory's end. The two
/// signature searches are one pass each over the
/// span all the candidates' windows cover
/// ([`resolve_first_hits_in`]), and the record
/// chains are walked once per distinct directory
/// START, every candidate naming that start
/// reading its walk off the one chain
/// ([`walk_chain_bytes`]) — so total resolution
/// cost is O(span + candidates), however the
/// candidates' declared fields vary.
///
/// There is no memo and no cross-candidate service
/// of any kind (round 12, R12-4): each candidate's
/// resolution is computed from its own position —
/// resolution depends on that position, so
/// identical declared fields do NOT resolve
/// identically, and the round-11 memo, which keyed
/// resolutions by declared fields alone, could hand
/// one candidate another's failed resolution and
/// refuse an honest file standing behind an
/// identical-fields decoy. Both searches run over
/// the whole span each candidate leaves, exactly
/// as the enumeration's shared pass runs them
/// (round 11, R11-3). The fallback sees the same
/// directories the enumeration sees, under the
/// same identity — and dedupes by that identity,
/// never by the declared fields the sender
/// controls (round 11, R11-2).
fn candidate_resolutions_in(bytes: &[u8], positions: &[usize]) -> Vec<CandidateResolution> {
    // Stage 1: parse each candidate's end record and
    // collect its zip64-record search as a query.
    struct Parsed {
        at: usize,
        live: bool,
        dir_end: u64,
        declared_offset: u64,
        declared_total: u64,
        declared_size: u64,
        zip64_query: Option<usize>,
        zip64_at: Option<u64>,
        cd_query: Option<usize>,
    }
    let mut parsed: Vec<Parsed> = Vec::with_capacity(positions.len());
    let mut queries: Vec<FirstHit> = Vec::new();
    for &at in positions {
        if at + 22 > bytes.len() || !bytes[at..].starts_with(b"PK\x05\x06") {
            // Not an end record at all: the slot
            // resolves to the unresolved key at
            // stage 4, marked by `live`.
            parsed.push(Parsed {
                at,
                live: false,
                dir_end: at as u64,
                declared_offset: 0,
                declared_total: 0,
                declared_size: 0,
                zip64_query: None,
                zip64_at: None,
                cd_query: None,
            });
            continue;
        }
        let total16 = u16::from_le_bytes([bytes[at + 10], bytes[at + 11]]);
        let cd_size32 = u32::from_le_bytes(bytes[at + 12..at + 16].try_into().unwrap_or([0; 4]));
        let cd_offset32 = u32::from_le_bytes(bytes[at + 16..at + 20].try_into().unwrap_or([0; 4]));
        let mut zip64_query = None;
        if (total16 == u16::MAX || cd_offset32 == u32::MAX) && at >= 20 {
            let lat = at - 20;
            let locator = &bytes[lat..at];
            if locator.starts_with(b"PK\x06\x07") {
                let record_declared =
                    u64::from_le_bytes(locator[8..16].try_into().unwrap_or([0; 8]));
                if record_declared < lat as u64 {
                    zip64_query = Some(queries.len());
                    queries.push(FirstHit {
                        sig: *b"PK\x06\x06",
                        from: record_declared,
                        to: lat as u64,
                        answer: None,
                    });
                }
            }
        }
        parsed.push(Parsed {
            at,
            live: true,
            dir_end: at as u64,
            declared_offset: u64::from(cd_offset32),
            declared_total: u64::from(total16),
            declared_size: u64::from(cd_size32),
            zip64_query,
            zip64_at: None,
            cd_query: None,
        });
    }
    if !queries.is_empty() {
        resolve_first_hits_in(bytes, &mut queries);
    }
    // Stage 2: a found zip64 record retargets its
    // candidate's window and replaces the sentinel
    // fields, exactly as in the enumeration; then
    // every candidate's directory-start search is a
    // query for the second shared pass.
    let mut cd_queries: Vec<FirstHit> = Vec::new();
    for cand in parsed.iter_mut() {
        if let Some(qi) = cand.zip64_query {
            if let Some(record_at) = queries[qi].answer {
                let record_at = record_at as usize;
                if record_at + 56 <= bytes.len() {
                    let record = &bytes[record_at..record_at + 56];
                    cand.dir_end = record_at as u64;
                    cand.zip64_at = Some(record_at as u64);
                    cand.declared_total =
                        u64::from_le_bytes(record[32..40].try_into().unwrap_or([0; 8]));
                    cand.declared_size =
                        u64::from_le_bytes(record[40..48].try_into().unwrap_or([0; 8]));
                    cand.declared_offset =
                        u64::from_le_bytes(record[48..56].try_into().unwrap_or([0; 8]));
                }
            }
        }
        if cand.live && cand.declared_offset < cand.dir_end {
            cand.cd_query = Some(cd_queries.len());
            cd_queries.push(FirstHit {
                sig: *b"PK\x01\x02",
                from: cand.declared_offset,
                to: cand.dir_end,
                answer: None,
            });
        }
    }
    if !cd_queries.is_empty() {
        resolve_first_hits_in(bytes, &mut cd_queries);
    }
    // Stage 3: one chain walk per distinct directory
    // start, out to the farthest end any candidate
    // naming it needs. No declared-size target joins
    // the extent (round 13, R13-4): the reader's own
    // condition is its declared total's records
    // chaining from the start, and those records
    // stand before the directory's end, inside the
    // extent the ends already give.
    let mut extents: HashMap<u64, u64> = HashMap::new();
    for cand in &parsed {
        if let Some(cd_start) = cand.cd_query.and_then(|qi| cd_queries[qi].answer) {
            let extent = extents.entry(cd_start).or_insert(0);
            *extent = (*extent).max(cand.dir_end);
        }
    }
    let mut chains: HashMap<u64, ChainWalk> = HashMap::new();
    for (cd_start, extent) in &extents {
        let chain = walk_chain_bytes(bytes, *cd_start, *extent, MAX_ENTRIES);
        chains.insert(*cd_start, chain);
    }
    // Stage 4: read each candidate's resolution off
    // its chain (or its empty window).
    let mut out: Vec<CandidateResolution> = Vec::with_capacity(parsed.len());
    for cand in &parsed {
        if !cand.live {
            let key: DirKey = (None, cand.at as u64, None, 0);
            out.push(CandidateResolution {
                key,
                walkable: false,
                reader_walkable: false,
                declared_total: 0,
                exact_count: None,
            });
            continue;
        }
        let cd_start = cand.cd_query.and_then(|qi| cd_queries[qi].answer);
        let key: DirKey = (
            cd_start,
            cand.declared_size,
            cand.zip64_at,
            if cd_start.is_none() {
                cand.declared_total
            } else {
                0
            },
        );
        if cand.declared_offset >= cand.dir_end {
            // No directory window at all: an honestly
            // empty archive counts zero; anything else
            // does not walk.
            let empty = cand.declared_total == 0;
            out.push(CandidateResolution {
                key,
                walkable: empty,
                reader_walkable: empty,
                declared_total: cand.declared_total,
                exact_count: if empty { Some(0) } else { None },
            });
            continue;
        }
        let Some(cd_start) = cd_start else {
            out.push(CandidateResolution {
                key,
                walkable: false,
                reader_walkable: false,
                declared_total: cand.declared_total,
                exact_count: None,
            });
            continue;
        };
        let chain = chains.get(&cd_start).expect("a chain per start");
        let walked = chain_count(chain, cand.dir_end, MAX_ENTRIES);
        // A directory past the cap walks (the archive
        // read refuses it in the cap's words), but its
        // count is the cap stop's, not an exact count.
        let walkable = walked.is_some();
        let mut exact_count = match walked {
            Some(count) if count <= MAX_ENTRIES => {
                if chain.reaches(cand.dir_end) {
                    Some(count)
                } else {
                    None
                }
            }
            _ => None,
        };
        // The reader's own reading is total-driven:
        // it derives the directory's start the way
        // this pass does and parses exactly the
        // declared total of central headers in
        // sequence, wherever the end structures stand
        // after them and whatever the declared size
        // claims (round 12, R12-10's shape; round 13,
        // R13-4: the size-bounded reading this code
        // used to mirror is not the reader's — a size
        // lying by a few bytes closed the gate on a
        // directory the reader reads whole). The
        // count the reader will parse is the total
        // itself, so a zip64 candidate standing only
        // by this reading still presents an exact
        // count for its total patch.
        let mut reader_walkable = walkable;
        if !reader_walkable && chain.parses_total(cand.declared_total) {
            reader_walkable = true;
            if cand.declared_total <= MAX_ENTRIES as u64 {
                exact_count = Some(cand.declared_total as usize);
            }
        }
        out.push(CandidateResolution {
            key,
            walkable,
            reader_walkable,
            declared_total: cand.declared_total,
            exact_count,
        });
    }
    out
}

/// Resolve the one end-record candidate at `at` in
/// `bytes`, through the shared pass run on that
/// candidate alone. Test-only: the walk gate's
/// direct pins use this; the loaders' fallback
/// always resolves the whole position list at once
/// ([`candidate_resolutions_in`]).
#[cfg(test)]
fn candidate_resolution_in(bytes: &[u8], at: usize) -> (DirKey, bool) {
    let mut resolutions = candidate_resolutions_in(bytes, &[at]);
    let resolution = resolutions.pop().expect("one candidate, one resolution");
    (resolution.key, resolution.walkable)
}

/// Whether the end-record candidate at `at` in
/// `bytes` names a directory that walks, under the
/// shared resolution ([`candidate_resolution_in`]).
/// Test-only, pinning the walk half of the gate
/// the fallback applies (round 10, R10-1/R10-3):
/// handing the archive reader a candidate whose
/// directory does not walk makes the reader retry
/// every earlier end record in the stream itself,
/// so thousands of fake signatures in a comment or
/// a file body cost seconds EACH — a candidate
/// that cannot walk is not opened at all, and one
/// that can walk is the reader's own first choice
/// in its stream, which the reader opens at once.
#[cfg(test)]
fn fallback_walkable(bytes: &[u8], at: usize) -> bool {
    candidate_resolution_in(bytes, at).1
}

/// The zip64 patch a candidate's open carries
/// where it resolves through a zip64 record: the
/// record's position and the directory's exact
/// walked record count. The archive reader checks
/// the zip64 record's entry totals against the
/// records it parses and refuses a record whose
/// total lies, while the count walk simply never
/// consults those totals (round 12, R12-9b): the
/// open rewrites both total fields to the walked
/// count — in the trimmed copy or stream only,
/// never in the file — so the reader path tolerates
/// the lie the way the walk does. An honest total
/// already equals the count, so the patch is a
/// no-op for honest files.
type Zip64TotalPatch = (u64, u64);

/// Open the archive ONE end-record candidate
/// describes: the file's bytes up to that record,
/// with the record's comment length zeroed — the
/// comment is no part of the directory, and any
/// signatures inside it are exactly what is being
/// stepped past. None when that candidate's
/// directory cannot be opened at all.
fn archive_from_candidate(
    bytes: &[u8],
    at: usize,
    zip64_patch: Option<Zip64TotalPatch>,
) -> Option<zip::ZipArchive<std::io::Cursor<Vec<u8>>>> {
    let end = at.checked_add(22)?;
    if end > bytes.len() {
        return None;
    }
    let mut trimmed = bytes[..end].to_vec();
    trimmed[at + 20] = 0;
    trimmed[at + 21] = 0;
    if let Some((record_at, count)) = zip64_patch {
        let record_at = record_at as usize;
        if record_at + 40 <= trimmed.len() {
            trimmed[record_at + 24..record_at + 32].copy_from_slice(&count.to_le_bytes());
            trimmed[record_at + 32..record_at + 40].copy_from_slice(&count.to_le_bytes());
        }
    }
    zip::ZipArchive::new(std::io::Cursor::new(trimmed)).ok()
}

/// A seekable stream over the file's bytes up to ONE
/// end-record candidate, with that record's comment
/// length reading as zero: the same trimmed archive
/// `archive_from_candidate` builds, without copying
/// the file's prefix — a picture-heavy bulletin is
/// hundreds of megabytes, and a candidate try must
/// not cost a copy of it (round 9, R9-2). The zip
/// reader handed this stream finds the candidate's
/// own end record at its end at once, instead of
/// searching the whole file past every fake end
/// record a comment can carry.
struct TrimmedReader {
    file: std::fs::File,
    /// The stream's length: the candidate's end.
    end: u64,
    /// The two comment-length bytes of the
    /// candidate's end record read as zero.
    zero_at: u64,
    /// Eight-byte fields read as a fixed value
    /// instead of the file's bytes: a lying zip64
    /// record's two total fields, read as the
    /// directory's walked count ([`Zip64TotalPatch`]).
    patches: Vec<(u64, u64)>,
    pos: u64,
    /// Where the file's own cursor stands: a read
    /// that starts there needs no seek first. The
    /// archive reader makes many small reads, most
    /// of them sequential, and a seek per read
    /// multiplied an open's cost (round 10, R10-1).
    file_at: u64,
}

impl Read for TrimmedReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.pos >= self.end || buf.is_empty() {
            return Ok(0);
        }
        let want = ((self.end - self.pos).min(buf.len() as u64)) as usize;
        if self.file_at != self.pos {
            self.file.seek(std::io::SeekFrom::Start(self.pos))?;
            self.file_at = self.pos;
        }
        let n = self.file.read(&mut buf[..want])?;
        for (i, byte) in buf.iter_mut().enumerate().take(n) {
            let at = self.pos + i as u64;
            if at == self.zero_at || at == self.zero_at + 1 {
                *byte = 0;
            }
            for &(start, value) in &self.patches {
                if at >= start && at < start + 8 {
                    *byte = (value >> ((at - start) * 8)) as u8;
                }
            }
        }
        self.pos += n as u64;
        self.file_at = self.pos;
        Ok(n)
    }
}

impl Seek for TrimmedReader {
    fn seek(&mut self, from: std::io::SeekFrom) -> std::io::Result<u64> {
        let target = match from {
            std::io::SeekFrom::Start(n) => n as i64,
            std::io::SeekFrom::End(n) => self.end as i64 + n,
            std::io::SeekFrom::Current(n) => self.pos as i64 + n,
        };
        if target < 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "seek before the start of the trimmed archive",
            ));
        }
        self.pos = target as u64;
        Ok(self.pos)
    }
}

/// Open the archive ONE end-record candidate
/// describes, straight from the file: the bytes up
/// to that record through a [`TrimmedReader`]. None
/// when that candidate's directory cannot be opened
/// at all.
fn archive_from_candidate_file(
    path: &Path,
    at: u64,
    zip64_patch: Option<Zip64TotalPatch>,
) -> Option<zip::ZipArchive<TrimmedReader>> {
    let end = at.checked_add(22)?;
    let file = std::fs::File::open(path).ok()?;
    let patches: Vec<(u64, u64)> = match zip64_patch {
        Some((record_at, count)) => vec![(record_at + 24, count), (record_at + 32, count)],
        None => Vec::new(),
    };
    let reader = TrimmedReader {
        file,
        end,
        zero_at: at + 20,
        patches,
        pos: 0,
        file_at: 0,
    };
    zip::ZipArchive::new(reader).ok()
}

pub fn save(doc: &Document, path: &Path) -> Result<(), String> {
    let mut doc = doc.clone();
    // Save what the document will say it is: the current
    // format version, whichever version it was loaded from.
    doc.format_version = ob_model::FORMAT_VERSION;
    let json = serde_json::to_string_pretty(&doc).map_err(|e| e.to_string())?;
    // Zip into memory, then the atomic write below.
    let mut buf: Vec<u8> = Vec::new();
    {
        let cursor = std::io::Cursor::new(&mut buf);
        let mut zip = zip::ZipWriter::new(cursor);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        zip.start_file("document.json", options)
            .map_err(|e| e.to_string())?;
        zip.write_all(json.as_bytes()).map_err(|e| e.to_string())?;
        // Asset bytes ride under their hash. Every byte set
        // in the map is written, whether or not its metadata
        // row survived an edit: bytes without a row get one
        // on load, never a silent drop here.
        let mut hashes: Vec<&String> = doc.asset_bytes.keys().collect();
        hashes.sort();
        for hash in hashes {
            zip.start_file(format!("assets/{hash}"), options)
                .map_err(|e| e.to_string())?;
            zip.write_all(&doc.asset_bytes[hash])
                .map_err(|e| e.to_string())?;
        }
        zip.finish().map_err(|e| e.to_string())?;
    }
    // Atomic save: write to a sibling temp file, then rename over the target.
    // A bulletin lost on Saturday night is a trust-ending event.
    let tmp = path.with_extension("obd.tmp");
    std::fs::write(&tmp, buf).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn load(path: &Path) -> Result<Document, String> {
    let head = read_head(path)?;
    let mut doc = if head == ZIP_MAGIC {
        load_v1(path)?
    } else {
        load_v0_json(path)?
    };
    // Records that share a hash fold into one before
    // anything reads them (round 5, RT5-1): a reader
    // takes the first record it finds for a picture,
    // so a weak duplicate standing first would
    // silently shadow the fuller record.
    ob_model::fold_duplicate_asset_records(&mut doc.assets);
    doc.migrate_occasion_keys();
    doc.migrate_participants();
    absorb_sidecar_pictures(&mut doc, path);
    Ok(doc.sanitized())
}

/// The first bytes of the file: enough to tell a zip (v1)
/// from bare JSON (v0) by the container, never the name.
fn read_head(path: &Path) -> Result<Vec<u8>, String> {
    let mut f = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut head = vec![0u8; 4];
    let n = f.read(&mut head).map_err(|e| e.to_string())?;
    head.truncate(n);
    Ok(head)
}

fn check_version(doc: &Document) -> Result<(), String> {
    if doc.format_version > ob_model::FORMAT_VERSION {
        return Err(format!(
            "document format {} is newer than this app understands ({})",
            doc.format_version,
            ob_model::FORMAT_VERSION
        ));
    }
    Ok(())
}

/// A v0 file: bare JSON, size-guarded as it always was.
fn load_v0_json(path: &Path) -> Result<Document, String> {
    if let Ok(md) = std::fs::metadata(path) {
        if md.len() > MAX_JSON_BYTES {
            return Err("that file is far larger than a bulletin ever is".to_string());
        }
    }
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let doc: Document = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    check_version(&doc)?;
    Ok(doc)
}

/// A v1 file: document.json plus its asset entries. Every
/// asset entry's bytes are checked against the hash in its
/// name before anything is trusted: a picture that does not
/// match its fingerprint is refused in plain words, never
/// silently used.
fn load_v1(path: &Path) -> Result<Document, String> {
    // The raw entry count is checked before the archive
    // is opened at all, so a flood of same-name entries
    // is refused from the directory alone (round 5,
    // RT5-5).
    check_entry_cap(path)?;
    // The directories are tried latest first, the
    // reader's own order (round 8, R8-12): a zip
    // comment can carry end-record signatures of its
    // own, and the latest end record can name an
    // empty or fake directory while the bulletin
    // stands behind it. The candidates whose
    // directories walk are tried first, each opened
    // at its own end record (round 9, R9-2: handing
    // the whole file to the archive reader makes the
    // reader search past every fake signature itself,
    // the same per-candidate cost the count paid).
    // A candidate whose directory holds no
    // document.json steps aside, and so does one
    // whose document FAILS to read — bad JSON, a
    // newer version, a lying fingerprint or size
    // (round 9, R9-6): the retry's business is trying
    // the other candidates before declaring there is
    // no bulletin. Only when every candidate fails
    // does the load refuse, with the FIRST failed
    // candidate's error — the most informative
    // refusal the office would have gotten without
    // the junk. The flood shapes are already refused
    // by the cap above. The candidates are distinct
    // directories (round 10, R10-1): a directory is
    // opened at most once here, however many
    // candidates pointed at it.
    let mut first_error: Option<String> = None;
    let mut opened_any = false;
    let mut tried_positions: HashSet<u64> = HashSet::new();
    // The resolved identity of every directory
    // tried in this pass, under the enumeration's
    // own key: the fallback below dedupes against
    // it, so a directory tried here is not opened
    // again there under another candidate's name
    // (round 11, R11-2) — by TWIN, key plus declared
    // total, so a twin whose total differs from the
    // tried one's is not a repeat: its reading of
    // the same directory can differ (round 13,
    // R13-4). The set is filled from the shared
    // resolutions below, where each tried position's
    // total is known.
    let mut tried_resolved: HashSet<TwinKey> = HashSet::new();
    for (at, count, key) in directory_candidates(path, MAX_ENTRIES) {
        tried_positions.insert(at);
        let zip64_patch = key.2.map(|record_at| (record_at, count as u64));
        let Some(mut zip) = archive_from_candidate_file(path, at, zip64_patch) else {
            continue;
        };
        opened_any = true;
        if !zip.file_names().any(|n| n == "document.json") {
            continue;
        }
        match read_bulletin_archive(&mut zip) {
            Ok(doc) => return Ok(doc),
            Err(e) => {
                if first_error.is_none() {
                    first_error = Some(e);
                }
            }
        }
    }
    // No candidate YIELDED a bulletin: the reader's
    // own path runs now (round 10, R10-3). It used to
    // run only when no walkable candidate could even
    // be opened, but opening is not evidence — a
    // walkable fake candidate standing in front of
    // trailing junk opened, held no bulletin, and
    // closed this gate on the honest directory the
    // junk had pushed out of the tail window. The
    // reader's path decides, with the same stepping
    // aside: its chosen directory first, then every
    // enumerated candidate's — each directory once,
    // the ones already tried above skipped, and no
    // candidate whose directory neither walks nor
    // reads opened at all (the walk gate below: the
    // reader retries every earlier end record in
    // a stream whose chosen candidate fails, so an
    // unwalkable fake at the head of thousands of
    // signatures costs seconds per retry if it is
    // handed over).
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let positions = end_record_positions_in(&bytes);
    // Every candidate in the file is resolved ONCE,
    // in one shared pass over the span (round 12,
    // R12-5), before either the gate or the fallback
    // loop below consults a resolution: no candidate
    // is ever resolved twice in a load, and no
    // resolution is ever served across candidates
    // (round 12, R12-4).
    let resolutions = candidate_resolutions_in(&bytes, &positions);
    // Name the twins the enumeration above tried,
    // by key plus declared total, from those same
    // resolutions (round 13, R13-4): a directory
    // tried under one total stands tried; the same
    // directory under another total does not.
    for (at, resolution) in positions.iter().zip(resolutions.iter()) {
        if tried_positions.contains(&(*at as u64)) {
            tried_resolved.insert((resolution.key, resolution.declared_total));
        }
    }
    // The reader's chosen directory is the one its
    // backward search lands on — the latest candidate
    // in the file. That attempt runs when there is
    // no candidate at all (the reader's damaged
    // words are the answer then), when its chosen
    // candidate's directory walks or reads, when no
    // candidate above could even be opened and the
    // candidate count is small (the reader's damaged
    // words are the informative answer for a corrupt
    // file, and a handful of candidates cannot
    // storm), or when a directory that walks or
    // reads stands anywhere in the file untried —
    // the enumeration above never saw it, or saw it
    // and could not count it (a gap standing after
    // its directory, which the reader tolerates,
    // breaks the chain-exact count while the
    // directory itself stands whole: round 12,
    // R12-10) — where untried is by TWIN: a twin
    // whose declared total differs from the tried
    // twin's is an untried reading of a tried
    // directory, and it leaves the attempt open
    // (round 13, R13-4). Whether a candidate above already
    // opened — or this very candidate was already
    // tried — never closes the attempt (round 11,
    // R11-3): a candidate that opens but yields no
    // bulletin is not evidence the file holds
    // nothing, and the attempt's cost is the one
    // directory the resolution just measured.
    // Otherwise — no directory anywhere walks or
    // reads untried, among many — the reader has
    // nothing the enumeration below does not cover.
    let any_untried_readable = positions
        .iter()
        .zip(resolutions.iter())
        .any(|(at, resolution)| {
            (resolution.walkable || resolution.reader_walkable)
                && !tried_positions.contains(&(*at as u64))
                && !tried_resolved.contains(&(resolution.key, resolution.declared_total))
        });
    let reader_gate_open = match resolutions.first() {
        None => true,
        Some(first) => {
            first.walkable
                || first.reader_walkable
                || (!opened_any && positions.len() <= 64)
                || any_untried_readable
        }
    };
    let mut damaged: Option<String> = None;
    if reader_gate_open {
        let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
        match zip::ZipArchive::new(file) {
            Ok(mut zip) => {
                if zip.file_names().any(|n| n == "document.json") {
                    match read_bulletin_archive(&mut zip) {
                        Ok(doc) => return Ok(doc),
                        Err(e) => {
                            if first_error.is_none() {
                                first_error = Some(e);
                            }
                        }
                    }
                }
            }
            Err(e) => {
                damaged = Some(format!("that file's container is damaged: {e}"));
            }
        }
    }
    // The fallback loop proper, over the resolutions
    // the shared pass already computed — it resolves
    // nothing itself. The dedupe is by the RESOLVED
    // directory (round 11, R11-2): candidates whose
    // declared fields differ but which resolve to
    // the same directory — its start, its size, the
    // zip64 record it resolves through — open it
    // once, and a directory already tried above
    // (its identity stands in `tried_resolved`) is
    // not opened again under another candidate's
    // name. The dedupe unit is the TWIN — that
    // identity plus the declared total (round 13,
    // R13-4): the reader's reading of one directory
    // is driven by the total, so a twin declaring
    // another total is another reading of the
    // directory, tried once under its own total
    // when the key's representative yielded
    // nothing, never a repeat of the tried twin.
    for (at, resolution) in positions.iter().zip(resolutions.iter()) {
        let at = *at;
        // A candidate already tried above is not
        // opened again (round 10, R10-1); a candidate
        // whose directory neither walks nor reads is
        // not opened either (the walk gate, round 10,
        // R10-3 — a directory whose records chain
        // exactly the declared total IS the reader's
        // own first choice in its trimmed stream, so
        // opening it costs the one read, never a
        // retry storm); and a candidate resolving to
        // a twin already tried is a repeat of it, not
        // another reading (round 11, R11-2; round 13,
        // R13-4).
        if tried_positions.contains(&(at as u64)) {
            continue;
        }
        if !resolution.walkable && !resolution.reader_walkable {
            continue;
        }
        if !tried_resolved.insert((resolution.key, resolution.declared_total)) {
            continue;
        }
        let zip64_patch = match (resolution.key.2, resolution.exact_count) {
            (Some(record_at), Some(count)) => Some((record_at, count as u64)),
            _ => None,
        };
        let Some(mut zip) = archive_from_candidate(&bytes, at, zip64_patch) else {
            continue;
        };
        if !zip.file_names().any(|n| n == "document.json") {
            continue;
        }
        match read_bulletin_archive(&mut zip) {
            Ok(doc) => return Ok(doc),
            Err(e) => {
                if first_error.is_none() {
                    first_error = Some(e);
                }
            }
        }
    }
    if let Some(e) = first_error {
        return Err(e);
    }
    if let Some(damaged) = damaged {
        return Err(damaged);
    }
    Err("that file has no bulletin inside it (no document.json)".to_string())
}

/// Read a bulletin out of an opened archive: the
/// entry cap on the archive's own count, the total
/// cap counted in actual bytes as entries are read,
/// and every asset entry's bytes checked against the
/// hash in its name before anything is trusted.
fn read_bulletin_archive<R: Read + Seek>(zip: &mut zip::ZipArchive<R>) -> Result<Document, String> {
    // The entry cap counts the archive's parts before
    // any payload is read (round 4, RT-2).
    if zip.len() > MAX_ENTRIES {
        return Err(TOO_MANY_PARTS.to_string());
    }
    // The total cap counts actual bytes as each entry is
    // read (round 3, RT3-B): the directory's declared
    // sizes are never summed, because a directory can
    // lie about them. `total` never passes the cap:
    // every read is capped at what is left of it.
    let mut total: u64 = 0;
    let mut doc: Document = {
        let mut entry = zip
            .by_name("document.json")
            .map_err(|_| "that file has no bulletin inside it (no document.json)".to_string())?;
        let text = read_entry_text_capped(
            &mut entry,
            MAX_JSON_BYTES,
            "that file is far larger than a bulletin ever is",
        )?;
        total += text.len() as u64;
        serde_json::from_str(&text).map_err(|e| e.to_string())?
    };
    check_version(&doc)?;
    // The hashes the document already holds a record
    // for, kept as a set (round 4, RT-2): asking the
    // growing list once per asset made opening cost
    // rise with the square of the asset count.
    let mut recorded: HashSet<String> = doc.assets.iter().map(|a| a.sha256.clone()).collect();
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| e.to_string())?;
        let name = entry.name().to_string();
        let Some(hash) = name.strip_prefix("assets/") else {
            continue;
        };
        let hash = hash.to_string();
        let bytes = read_entry_capped(&mut entry, MAX_TOTAL_UNCOMPRESSED - total, TOTAL_TOO_LARGE)?;
        total += bytes.len() as u64;
        if ob_model::sha256_hex(&bytes) != hash {
            let short: String = hash.chars().take(12).collect();
            return Err(format!(
                "a picture inside that file does not match its fingerprint ({short}) \
                 so the file was not opened; it may be damaged."
            ));
        }
        if recorded.insert(hash.clone()) {
            // Bytes whose metadata row is missing still
            // belong to the bulletin; give them an honest
            // record rather than dropping them.
            doc.assets.push(ob_model::Asset {
                sha256: hash.clone(),
                kind: AssetKind::Other,
                media_type: "application/octet-stream".to_string(),
                rights: RightsRecord {
                    source: "inside the bulletin file".to_string(),
                    ..Default::default()
                },
            });
        }
        doc.asset_bytes.insert(hash, bytes);
    }
    Ok(doc)
}

/// The root of an .obdi file: the sheet itself, plus the
/// asset records of the pictures it uses (their bytes
/// ride as `assets/<sha256>` entries, exactly as in a
/// bulletin file). The records travel so a sheet that
/// arrives from another church keeps the credit lines
/// its pictures are owed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct InsertPackage {
    insert: Insert,
    #[serde(default)]
    assets: Vec<Asset>,
}

/// Save one loose sheet as an .obdi file: the v1
/// container discipline under another name (roles build
/// R-3). Exactly the assets the sheet's frames use are
/// packed, from the host document's store; a referenced
/// picture whose bytes the host does not hold is left
/// out, and the sheet keeps saying that picture is
/// missing, as it did in the bulletin. The write is
/// atomic, like a bulletin's.
pub fn save_insert(insert: &Insert, host: &Document, path: &Path) -> Result<(), String> {
    let referenced = insert.referenced_assets();
    let package = InsertPackage {
        insert: insert.clone(),
        assets: host
            .assets
            .iter()
            .filter(|a| referenced.contains(&a.sha256))
            .cloned()
            .collect(),
    };
    let json = serde_json::to_string_pretty(&package).map_err(|e| e.to_string())?;
    let mut buf: Vec<u8> = Vec::new();
    {
        let cursor = std::io::Cursor::new(&mut buf);
        let mut zip = zip::ZipWriter::new(cursor);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        zip.start_file("insert.json", options)
            .map_err(|e| e.to_string())?;
        zip.write_all(json.as_bytes()).map_err(|e| e.to_string())?;
        for hash in &referenced {
            let Some(bytes) = host.asset_data(hash) else {
                continue;
            };
            zip.start_file(format!("assets/{hash}"), options)
                .map_err(|e| e.to_string())?;
            zip.write_all(bytes).map_err(|e| e.to_string())?;
        }
        zip.finish().map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension("obdi.tmp");
    std::fs::write(&tmp, buf).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())?;
    Ok(())
}

/// Load an .obdi file: the sheet, the asset records it
/// carried, and the picture bytes it packed, by hash.
/// The .obd discipline throughout: the total is capped
/// by counting actual bytes as entries are read (the
/// directory's declared sizes are never trusted —
/// round 3, RT3-B), and every asset entry's bytes are
/// checked against the fingerprint in its name before
/// anything is trusted; a mismatch refuses in plain
/// words.
/// What a sheet file carries out: the sheet itself,
/// the records of the pictures it uses, and their
/// bytes keyed by fingerprint.
pub type LoadedInsert = (Insert, Vec<Asset>, BTreeMap<String, Vec<u8>>);

pub fn load_insert(path: &Path) -> Result<LoadedInsert, String> {
    let head = read_head(path)?;
    if head != ZIP_MAGIC {
        return Err("that file is not a loose sheet file (.obdi)".to_string());
    }
    // The same raw entry count as the bulletin door,
    // checked before the archive is opened (round 5,
    // RT5-5).
    check_entry_cap(path)?;
    // The same candidate order and stepping aside as
    // the bulletin door (round 8, R8-12; round 9,
    // R9-2 and R9-6): walkable candidates latest
    // first, each opened at its own end record; a
    // candidate holding no insert.json steps aside,
    // and so does one whose sheet fails to read; only
    // when every candidate fails does the load
    // refuse, with the first failed candidate's
    // error. The candidates are distinct directories
    // (round 10, R10-1), each opened at most once.
    // And as at the bulletin door, the reader's own
    // path runs whenever no candidate YIELDS a sheet
    // (round 10, R10-3): a candidate that merely
    // opens does not close the fallback gate.
    let mut first_error: Option<String> = None;
    let mut opened_any = false;
    let mut tried_positions: HashSet<u64> = HashSet::new();
    // The resolved identity of every directory
    // tried in this pass, as at the bulletin door:
    // the fallback below dedupes against it
    // (round 11, R11-2), by twin — key plus declared
    // total (round 13, R13-4). The set is filled
    // from the shared resolutions below.
    let mut tried_resolved: HashSet<TwinKey> = HashSet::new();
    for (at, count, key) in directory_candidates(path, MAX_ENTRIES) {
        tried_positions.insert(at);
        let zip64_patch = key.2.map(|record_at| (record_at, count as u64));
        let Some(mut zip) = archive_from_candidate_file(path, at, zip64_patch) else {
            continue;
        };
        opened_any = true;
        if !zip.file_names().any(|n| n == "insert.json") {
            continue;
        }
        match read_insert_archive(&mut zip) {
            Ok(loaded) => return Ok(loaded),
            Err(e) => {
                if first_error.is_none() {
                    first_error = Some(e);
                }
            }
        }
    }
    // The reader's own path, gated exactly as at
    // the bulletin door (round 10, R10-3; round 11,
    // R11-3; round 12, R12-5/R12-10): one shared
    // resolution of every candidate first, then its
    // chosen directory first when that candidate's
    // directory walks or reads — a candidate that
    // opened above but yielded no sheet never closes
    // the attempt — then every enumerated
    // candidate's, each directory once under its
    // resolved identity, the ones already tried
    // above skipped, and no candidate whose
    // directory neither walks nor reads opened at
    // all.
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let positions = end_record_positions_in(&bytes);
    let resolutions = candidate_resolutions_in(&bytes, &positions);
    // The twins the enumeration above tried, named
    // as at the bulletin door (round 13, R13-4).
    for (at, resolution) in positions.iter().zip(resolutions.iter()) {
        if tried_positions.contains(&(*at as u64)) {
            tried_resolved.insert((resolution.key, resolution.declared_total));
        }
    }
    let any_untried_readable = positions
        .iter()
        .zip(resolutions.iter())
        .any(|(at, resolution)| {
            (resolution.walkable || resolution.reader_walkable)
                && !tried_positions.contains(&(*at as u64))
                && !tried_resolved.contains(&(resolution.key, resolution.declared_total))
        });
    let reader_gate_open = match resolutions.first() {
        None => true,
        Some(first) => {
            first.walkable
                || first.reader_walkable
                || (!opened_any && positions.len() <= 64)
                || any_untried_readable
        }
    };
    let mut damaged: Option<String> = None;
    if reader_gate_open {
        let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
        match zip::ZipArchive::new(file) {
            Ok(mut zip) => {
                if zip.file_names().any(|n| n == "insert.json") {
                    match read_insert_archive(&mut zip) {
                        Ok(loaded) => return Ok(loaded),
                        Err(e) => {
                            if first_error.is_none() {
                                first_error = Some(e);
                            }
                        }
                    }
                }
            }
            Err(e) => {
                damaged = Some(format!("that file's container is damaged: {e}"));
            }
        }
    }
    // The fallback enumeration, exactly as at the
    // bulletin door: every candidate resolved once
    // by the shared pass above, deduped by the
    // RESOLVED directory against `tried_resolved` —
    // by twin, key plus declared total (round 13,
    // R13-4) — no resolution served across
    // candidates (round 12, R12-4/R12-5).
    for (at, resolution) in positions.iter().zip(resolutions.iter()) {
        let at = *at;
        // A candidate already tried above is not
        // opened again (round 10, R10-1); a candidate
        // whose directory neither walks nor reads is
        // not opened either (the walk gate, round 10,
        // R10-3); and a candidate resolving to a
        // twin already tried is a repeat of it, not
        // another reading (round 11, R11-2; round 13,
        // R13-4).
        if tried_positions.contains(&(at as u64)) {
            continue;
        }
        if !resolution.walkable && !resolution.reader_walkable {
            continue;
        }
        if !tried_resolved.insert((resolution.key, resolution.declared_total)) {
            continue;
        }
        let zip64_patch = match (resolution.key.2, resolution.exact_count) {
            (Some(record_at), Some(count)) => Some((record_at, count as u64)),
            _ => None,
        };
        let Some(mut zip) = archive_from_candidate(&bytes, at, zip64_patch) else {
            continue;
        };
        if !zip.file_names().any(|n| n == "insert.json") {
            continue;
        }
        match read_insert_archive(&mut zip) {
            Ok(loaded) => return Ok(loaded),
            Err(e) => {
                if first_error.is_none() {
                    first_error = Some(e);
                }
            }
        }
    }
    if let Some(e) = first_error {
        return Err(e);
    }
    if let Some(damaged) = damaged {
        return Err(damaged);
    }
    Err("that file has no loose sheet inside it (no insert.json)".to_string())
}

/// Read a loose sheet out of an opened archive, the
/// discipline `load_insert` states: caps counted in
/// actual bytes as entries are read, and every asset
/// entry's bytes checked against the fingerprint in
/// its name before anything is trusted.
fn read_insert_archive<R: Read + Seek>(
    zip: &mut zip::ZipArchive<R>,
) -> Result<LoadedInsert, String> {
    // The entry cap counts the archive's parts before
    // any payload is read (round 4, RT-2).
    if zip.len() > MAX_ENTRIES {
        return Err(TOO_MANY_PARTS.to_string());
    }
    // Actual bytes, counted as they are read — the same
    // discipline as the bulletin loader (round 3, RT3-B).
    let mut total: u64 = 0;
    let mut package: InsertPackage = {
        let mut entry = zip
            .by_name("insert.json")
            .map_err(|_| "that file has no loose sheet inside it (no insert.json)".to_string())?;
        let text = read_entry_text_capped(
            &mut entry,
            MAX_JSON_BYTES,
            "that file is far larger than a loose sheet ever is",
        )?;
        total += text.len() as u64;
        serde_json::from_str(&text).map_err(|e| e.to_string())?
    };
    let mut bytes_by_hash: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| e.to_string())?;
        let name = entry.name().to_string();
        let Some(hash) = name.strip_prefix("assets/") else {
            continue;
        };
        let hash = hash.to_string();
        let bytes = read_entry_capped(&mut entry, MAX_TOTAL_UNCOMPRESSED - total, TOTAL_TOO_LARGE)?;
        total += bytes.len() as u64;
        if ob_model::sha256_hex(&bytes) != hash {
            let short: String = hash.chars().take(12).collect();
            return Err(format!(
                "a picture inside that file does not match its fingerprint ({short}) \
                 so the file was not opened; it may be damaged."
            ));
        }
        bytes_by_hash.insert(hash, bytes);
    }
    // Bytes whose record did not travel still belong to
    // the sheet; give them an honest record rather than
    // dropping them, the same rule as a bulletin file.
    // The membership question is asked of a set, not by
    // scanning the records once per picture (round 4,
    // RT-2: the same squared cost the bulletin loader
    // had).
    let mut recorded: HashSet<String> = package.assets.iter().map(|a| a.sha256.clone()).collect();
    for hash in bytes_by_hash.keys() {
        if recorded.insert(hash.clone()) {
            package.assets.push(Asset {
                sha256: hash.clone(),
                kind: AssetKind::Other,
                media_type: "application/octet-stream".to_string(),
                rights: RightsRecord {
                    source: "inside the sheet file".to_string(),
                    ..Default::default()
                },
            });
        }
    }
    // Records that share a hash fold into one before
    // the attach reads them, the same rule as a
    // bulletin file (round 5, RT5-1).
    ob_model::fold_duplicate_asset_records(&mut package.assets);
    // Geometry from a file gets the same clamping the
    // bulletin loader promises: a hostile .obdi must not
    // plant numbers the writer cannot print.
    Ok((package.insert.sanitized(), package.assets, bytes_by_hash))
}

/// One sidecar picture's size cap. A tracing picture is a
/// page render; tens of megabytes is generous, and a
/// "picture" bigger than this is not one (audit Batch 1).
const MAX_SIDECAR_BYTES: u64 = 64 * 1024 * 1024;

/// The media type a picture's own first bytes state, or
/// None when the bytes are none of the four: a text file
/// renamed .png is not a picture and is never gathered
/// in (audit Batch 1).
fn picture_media_type(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        Some("image/png")
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("image/jpeg")
    } else if bytes.starts_with(b"GIF8") {
        Some("image/gif")
    } else if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some("image/webp")
    } else {
        None
    }
}

/// Gather a panel's tracing picture into the asset store
/// when its reference is a sidecar path that resolves beside
/// the bulletin file (a v0 file, or a v1 file written by an
/// early draft of the format). A reference already naming a
/// stored asset stays as it is; a path that does not resolve
/// stays a path too, so the app keeps saying the picture is
/// missing, exactly as it always has.
///
/// The gathering is fenced (audit Batch 1): the sidecar
/// must resolve INSIDE the bulletin's own folder tree,
/// must be no larger than a page picture ever is, and its
/// bytes must actually be a picture. Before the fence, a
/// file that named "../secret.png" (or an absolute path to
/// anything at all) had that file read into the asset
/// store on load, silently, ready to travel with the next
/// save to every recipient.
fn absorb_sidecar_pictures(doc: &mut Document, obd_path: &Path) {
    let base = obd_path.parent().unwrap_or_else(|| Path::new("."));
    let Ok(base) = std::fs::canonicalize(base) else {
        return;
    };
    // First pass (read-only): which panels carry a resolvable
    // sidecar, and what its bytes and type are.
    let mut pending: Vec<(usize, Vec<u8>, &'static str, String)> = Vec::new();
    for (i, panel) in doc.panels.iter().enumerate() {
        let Some(reference) = &panel.background_image else {
            continue;
        };
        if doc.asset_bytes.contains_key(reference) {
            continue;
        }
        // Canonicalizing settles "..", "." and symlinks in
        // one step: what remains must sit inside the
        // bulletin's own folder tree.
        let Ok(candidate) = std::fs::canonicalize(base.join(reference)) else {
            continue;
        };
        if !candidate.starts_with(&base) {
            continue;
        }
        let Ok(meta) = std::fs::metadata(&candidate) else {
            continue;
        };
        if !meta.is_file() || meta.len() > MAX_SIDECAR_BYTES {
            continue;
        }
        let Ok(bytes) = std::fs::read(&candidate) else {
            continue;
        };
        if bytes.is_empty() || bytes.len() as u64 > MAX_SIDECAR_BYTES {
            continue;
        }
        let Some(media_type) = picture_media_type(&bytes) else {
            continue;
        };
        pending.push((i, bytes, media_type, reference.clone()));
    }
    // Second pass: store the bytes and point the panel at
    // the asset.
    for (i, bytes, media_type, source) in pending {
        let hash = doc.add_asset(
            &bytes,
            AssetKind::TracingPicture,
            media_type,
            RightsRecord {
                source,
                ..Default::default()
            },
        );
        doc.panels[i].background_image = Some(hash);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ob_model::*;

    fn scratch(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("ob-format-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("dir");
        dir
    }

    fn test_doc() -> Document {
        Document {
            format_version: FORMAT_VERSION,
            title: "Roundtrip".into(),
            profile: ChurchProfile {
                name: "Grace".into(),
                ..Default::default()
            },
            fields: Default::default(),
            styles: default_styles(),
            stories: vec![Story {
                id: "s".into(),
                paragraphs: vec![Paragraph::new("body", "Hello")],
                keep_through: None,
            }],
            panels: vec![Panel {
                background_image: None,
                id: "p".into(),
                name: "P".into(),
                size: Size::new(1, 2),
                frames: vec![],
                guides: vec![],
            }],
            sheet: SheetSpec {
                preset: "letter-portrait".into(),
                size: Size::new(1, 2),
            },
            no_announcements: false,
            assets: Vec::new(),
            asset_bytes: Default::default(),
            occasion_time: None,
            wanted_fonts: Vec::new(),
            participants: Default::default(),
            inserts: Default::default(),
            applied_contributions: Vec::new(),
        }
    }

    #[test]
    fn extreme_values_are_clamped_on_load() {
        let dir = scratch("x");
        let path = dir.join("hostile.obd");
        let mut doc = test_doc();
        doc.styles = vec![ParagraphStyle {
            id: "body".into(),
            name: "Body".into(),
            font_family: "Liberation Sans".into(),
            font_sha256: None,
            font_size: i64::MAX,
            bold: false,
            italic: false,
            align: Align::Left,
            space_after: i64::MAX,
            color: Rgb::BLACK,
        }];
        ob_format_save_helper(&path, &doc);
        let loaded = load(&path).expect("loads");
        assert_eq!(loaded.styles[0].font_size, 288_000);
        assert_eq!(loaded.styles[0].space_after, 144_000);
        doc.format_version = FORMAT_VERSION + 1;
        ob_format_save_helper(&path, &doc);
        assert!(load(&path).is_err(), "newer versions are refused");
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn ob_format_save_helper(path: &std::path::Path, doc: &Document) {
        std::fs::write(path, serde_json::to_string(doc).expect("json")).expect("write");
    }

    #[test]
    fn roundtrip() {
        let doc = test_doc();
        let dir = scratch("rt");
        let path = dir.join("roundtrip.obd");
        save(&doc, &path).unwrap();
        // What landed on disk is the v1 container.
        let head = std::fs::read(&path).unwrap();
        assert_eq!(&head[..4], b"PK\x03\x04", "v1 files are zips");
        let back = load(&path).unwrap();
        assert_eq!(doc, back);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_applied_parts_record_survives_a_save_and_an_old_file_loads_without_one() {
        // Round 4 (R4-4): the record rides in the file,
        // so a bulletin saved by hand and reopened still
        // knows which parts were applied to it. A file
        // written before the field existed has no key
        // and loads with an empty record.
        let mut doc = test_doc();
        doc.applied_contributions = vec!["abcd1234".to_string(), "ef567890".to_string()];
        let dir = scratch("applied");
        let path = dir.join("applied.obd");
        save(&doc, &path).unwrap();
        let back = load(&path).unwrap();
        assert_eq!(back.applied_contributions, doc.applied_contributions);
        let mut json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&doc).expect("json")).expect("value");
        json.as_object_mut()
            .expect("object")
            .remove("applied_contributions");
        let old_path = dir.join("old.obd");
        std::fs::write(&old_path, serde_json::to_string(&json).expect("json")).expect("write");
        let old = load(&old_path).unwrap();
        assert!(old.applied_contributions.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_file_written_before_the_participants_map_loads_with_its_names() {
        // R-2b: an old file keeps its serving names in
        // `fields` under their slugs and has no
        // `participants` key at all. Loading folds them
        // into the map, and the tokens resolve.
        let mut doc = test_doc();
        doc.fields
            .insert("readers".to_string(), "Ana Ruiz".to_string());
        doc.fields
            .insert("ushers".to_string(), "Tom and Bea".to_string());
        let mut json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&doc).expect("json")).expect("value");
        json.as_object_mut().expect("object").remove("participants");
        let dir = scratch("parts");
        let path = dir.join("old.obd");
        std::fs::write(&path, json.to_string()).expect("write");
        let loaded = load(&path).expect("loads");
        assert_eq!(loaded.participant_value("readers"), "Ana Ruiz");
        assert_eq!(
            ob_model::resolve_fields("Readers: {Sunday.readers}", &loaded),
            "Readers: Ana Ruiz"
        );
        assert!(
            !loaded.fields.contains_key("readers"),
            "the fold moved the name out of fields"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn runs_survive_the_v1_round_trip_in_the_runs_shape() {
        let mut doc = test_doc();
        doc.stories[0].paragraphs = vec![Paragraph::from_runs(
            "body",
            vec![
                Run::plain("Sing "),
                Run {
                    text: "hymn".into(),
                    bold: Some(true),
                    italic: None,
                },
                Run::plain(" 432"),
            ],
        )];
        let dir = scratch("runs");
        let path = dir.join("runs.obd");
        save(&doc, &path).unwrap();
        // The document.json inside the zip speaks runs.
        let file = std::fs::File::open(&path).unwrap();
        let mut zip = zip::ZipArchive::new(file).unwrap();
        let mut json = String::new();
        zip.by_name("document.json")
            .unwrap()
            .read_to_string(&mut json)
            .unwrap();
        assert!(json.contains("\"runs\""), "runs written: {json}");
        let back = load(&path).unwrap();
        assert_eq!(back.stories[0].paragraphs, doc.stories[0].paragraphs);
        assert_eq!(back.stories[0].paragraphs[0].text(), "Sing hymn 432");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn v0_file_with_text_shape_paragraphs_loads_with_words_intact() {
        // A G1a-era (and earlier) file on disk: bare JSON,
        // paragraphs stored as { style, text }.
        let mut doc = test_doc();
        doc.stories[0].paragraphs = vec![Paragraph::new("body", "Welcome to worship")];
        let json = serde_json::to_string(&doc).expect("json");
        assert!(
            json.contains("\"runs\""),
            "the writer today speaks runs even for this doc"
        );
        // Rewrite the story's paragraph in the old shape, as
        // an old file on disk would hold it.
        let old_json = json.replace(
            r#""paragraphs":[{"style":"body","runs":[{"text":"Welcome to worship"}]}]"#,
            r#""paragraphs":[{"style":"body","text":"Welcome to worship"}]"#,
        );
        assert_ne!(old_json, json, "the old shape was substituted");
        let dir = scratch("v0runs");
        let path = dir.join("old.obd");
        std::fs::write(&path, old_json).unwrap();
        let back = load(&path).unwrap();
        assert_eq!(back.stories[0].paragraphs[0].text(), "Welcome to worship");
        assert_eq!(
            back.stories[0].paragraphs[0].runs,
            vec![Run::plain("Welcome to worship")]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn v1_carries_asset_bytes_by_hash_and_v0_resaves_as_v1() {
        let dir = scratch("v1");
        // A v0 file on disk: bare JSON, an asset recorded in
        // its table-less past (v0 JSON could not hold bytes;
        // the record alone is written here by hand).
        let mut doc = test_doc();
        let hash = doc.add_asset(
            b"fake-png-bytes",
            AssetKind::Picture,
            "image/png",
            RightsRecord {
                source: "the church camera".into(),
                license: LicenseKind::ChurchOwned,
                ..Default::default()
            },
        );
        let path = dir.join("week.obd");
        save(&doc, &path).unwrap();
        // The zip holds document.json and the asset entry.
        let file = std::fs::File::open(&path).unwrap();
        let mut zip = zip::ZipArchive::new(file).unwrap();
        assert!(zip.by_name("document.json").is_ok());
        let bytes = {
            let mut entry = zip.by_name(&format!("assets/{hash}")).expect("asset entry");
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes).unwrap();
            bytes
        };
        assert_eq!(bytes, b"fake-png-bytes");
        drop(zip);
        // And it loads back whole: bytes by hash, rights intact.
        let back = load(&path).unwrap();
        assert_eq!(back.asset_data(&hash), Some(b"fake-png-bytes".as_slice()));
        assert_eq!(back.assets.len(), 1);
        assert_eq!(back.assets[0].rights.source, "the church camera");
        assert_eq!(doc, back);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn v0_sidecar_picture_is_gathered_in_and_travels() {
        let dir = scratch("sidecar");
        // A v0 file in the old shape: JSON naming a sidecar
        // picture, the sidecar sitting beside it.
        let mut doc = test_doc();
        doc.format_version = 0;
        doc.panels[0].background_image = Some("assets/page-01.png".into());
        std::fs::create_dir_all(dir.join("assets")).unwrap();
        // A real PNG's first bytes (audit Batch 1 requires
        // them: text bytes wearing a .png name no longer
        // count as a picture).
        const PNG_BYTES: &[u8] = b"\x89PNG\r\n\x1a\npage-one-render";
        std::fs::write(dir.join("assets/page-01.png"), PNG_BYTES).unwrap();
        let path = dir.join("old.obd");
        ob_format_save_helper(&path, &doc);
        let loaded = load(&path).expect("v0 loads");
        let hash = ob_model::sha256_hex(PNG_BYTES);
        assert_eq!(
            loaded.panels[0].background_image.as_deref(),
            Some(hash.as_str()),
            "the panel now names the asset, not the path"
        );
        assert_eq!(loaded.asset_data(&hash), Some(PNG_BYTES));
        assert_eq!(loaded.assets[0].kind, AssetKind::TracingPicture);
        // Re-saved (always v1) and moved ALONE to a new home:
        // the picture is still attached, no sidecar needed.
        let moved_dir = scratch("moved");
        let moved = moved_dir.join("week.obd");
        save(&loaded, &moved).unwrap();
        let back = load(&moved).expect("moved file loads");
        assert_eq!(
            back.panels[0].background_image.as_deref(),
            Some(hash.as_str())
        );
        assert_eq!(back.asset_data(&hash), Some(PNG_BYTES));
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&moved_dir);
    }

    #[test]
    fn sidecars_outside_the_folder_or_not_pictures_are_never_gathered() {
        // Audit Batch 1: a hostile "old bulletin" must not
        // be able to pull files off the computer into
        // itself. Three shapes: a ".." walk to a canary
        // outside the folder, an absolute path to the same
        // canary, and text bytes wearing a picture's name
        // inside the folder. None may be gathered.
        const PNG_BYTES: &[u8] = b"\x89PNG\r\n\x1a\ncanary";
        let root = scratch("fence");
        let sub = root.join("sub");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(root.join("secret.png"), PNG_BYTES).unwrap();
        std::fs::write(sub.join("fake.png"), b"donor list, page one").unwrap();

        let write_doc = |reference: String| -> std::path::PathBuf {
            let mut doc = test_doc();
            doc.format_version = 0;
            doc.panels[0].background_image = Some(reference);
            let path = sub.join("old.obd");
            ob_format_save_helper(&path, &doc);
            path
        };

        // The ".." walk out of the folder.
        let path = write_doc("../secret.png".to_string());
        let loaded = load(&path).expect("loads");
        assert_eq!(
            loaded.panels[0].background_image.as_deref(),
            Some("../secret.png"),
            "the reference stays a path; nothing was gathered"
        );
        assert!(loaded.asset_bytes.is_empty());

        // The absolute path to the same canary.
        let absolute = root.join("secret.png").to_string_lossy().into_owned();
        let path = write_doc(absolute.clone());
        let loaded = load(&path).expect("loads");
        assert_eq!(
            loaded.panels[0].background_image.as_deref(),
            Some(absolute.as_str())
        );
        assert!(loaded.asset_bytes.is_empty());

        // Text bytes with a picture's name, inside the folder.
        let path = write_doc("fake.png".to_string());
        let loaded = load(&path).expect("loads");
        assert_eq!(
            loaded.panels[0].background_image.as_deref(),
            Some("fake.png")
        );
        assert!(loaded.asset_bytes.is_empty());

        // And the honest case still gathers: a real picture
        // sitting beside the bulletin under a plain name.
        std::fs::write(sub.join("page.png"), PNG_BYTES).unwrap();
        let path = write_doc("page.png".to_string());
        let loaded = load(&path).expect("loads");
        let hash = ob_model::sha256_hex(PNG_BYTES);
        assert_eq!(
            loaded.panels[0].background_image.as_deref(),
            Some(hash.as_str())
        );
        assert_eq!(loaded.asset_data(&hash), Some(PNG_BYTES));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn v0_missing_sidecar_keeps_its_path_and_its_honest_note() {
        let dir = scratch("missing");
        let mut doc = test_doc();
        doc.format_version = 0;
        doc.panels[0].background_image = Some("assets/page-01.png".into());
        let path = dir.join("old.obd");
        ob_format_save_helper(&path, &doc);
        let loaded = load(&path).expect("loads without the sidecar");
        // Nothing was invented: the reference stays the path
        // it was, so the app still reports the picture
        // missing instead of pretending it was gathered.
        assert_eq!(
            loaded.panels[0].background_image.as_deref(),
            Some("assets/page-01.png")
        );
        assert!(loaded.asset_bytes.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn tampered_asset_bytes_are_refused() {
        let dir = scratch("tamper");
        let mut doc = test_doc();
        doc.add_asset(
            b"real-bytes",
            AssetKind::Picture,
            "image/png",
            RightsRecord::default(),
        );
        let path = dir.join("week.obd");
        save(&doc, &path).unwrap();
        // Rewrite the zip with the asset entry's bytes
        // swapped: the entry still answers to the old hash.
        let file = std::fs::File::open(&path).unwrap();
        let mut zip = zip::ZipArchive::new(file).unwrap();
        let mut doc_json = String::new();
        zip.by_name("document.json")
            .unwrap()
            .read_to_string(&mut doc_json)
            .unwrap();
        let hash = ob_model::sha256_hex(b"real-bytes");
        drop(zip);
        let bad = dir.join("bad.obd");
        {
            let cursor = std::io::Cursor::new(Vec::new());
            let mut zw = zip::ZipWriter::new(cursor);
            let opts = zip::write::SimpleFileOptions::default();
            zw.start_file("document.json", opts).unwrap();
            zw.write_all(doc_json.as_bytes()).unwrap();
            zw.start_file(format!("assets/{hash}"), opts).unwrap();
            zw.write_all(b"swapped-bytes").unwrap();
            let cursor = zw.finish().unwrap();
            std::fs::write(&bad, cursor.into_inner()).unwrap();
        }
        let err = load(&bad).expect_err("tampered bytes refuse");
        assert!(err.contains("does not match its fingerprint"), "err: {err}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn r5_duplicate_asset_records_fold_on_load() {
        // Round 5 (RT5-1): document.json carries two
        // records for one picture: a weak one and the
        // record noting the children and the CC BY
        // license. Readers take the first record they
        // find, so a weak record standing first
        // disarmed the children note and dropped the
        // license. The load folds the pair into one
        // composed record, in either order.
        // FLAGGED, round 7 (R7-1): this test used to
        // pin the folded record's CC BY license
        // standing. The noting record carries no credit
        // line at all, so the folded record claimed a
        // CC BY license beside a children note with an
        // empty credit line — the composed state
        // RT7-1 graded BLOCKER. The license assertion
        // below therefore changed from CcBy("4.0") to
        // Unknown in R7-A; every other assertion (one
        // folded record, children note, title, source)
        // stands unchanged, and the fold's
        // anti-shadowing point stands with them.
        let bytes: &[u8] = b"picture bytes whose fingerprint both records share";
        let hash = ob_model::sha256_hex(bytes);
        let weak = RightsRecord {
            source: "a plainer copy".into(),
            ..Default::default()
        };
        let noting = RightsRecord {
            title: "Harvest supper".into(),
            license: LicenseKind::CcBy("4.0".into()),
            children_present: Some(true),
            ..Default::default()
        };
        let asset = |rights: RightsRecord| Asset {
            sha256: hash.clone(),
            kind: AssetKind::Picture,
            media_type: "image/png".into(),
            rights,
        };
        let dir = scratch("dup-records");
        for (tag, records) in [
            (
                "weak-first",
                vec![asset(weak.clone()), asset(noting.clone())],
            ),
            (
                "noting-first",
                vec![asset(noting.clone()), asset(weak.clone())],
            ),
        ] {
            let mut doc = test_doc();
            doc.assets = records;
            doc.asset_bytes.insert(hash.clone(), bytes.to_vec());
            let file = dir.join(format!("{tag}.obd"));
            save(&doc, &file).expect("the file saves");
            let loaded = load(&file).expect("the file loads");
            assert_eq!(loaded.assets.len(), 1, "{tag}: one record after the fold");
            let rights = &loaded.assets[0].rights;
            assert_eq!(rights.children_present, Some(true), "{tag}");
            assert_eq!(rights.license, LicenseKind::Unknown, "{tag}");
            assert_eq!(rights.title, "Harvest supper", "{tag}");
            assert_eq!(rights.source, "a plainer copy", "{tag}");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn r6_credit_first_records_fold_on_load_without_a_permission() {
        // Round 6 (R6-1): an .obd carrying the credit
        // record first and the children record second
        // used to fold into a record noting children
        // WITH a permission. Both orders now fold to a
        // record that notes the children and no
        // permission.
        let bytes: &[u8] = b"picture bytes the credit-first pair shares";
        let hash = ob_model::sha256_hex(bytes);
        let credit = RightsRecord {
            credit_text: "Photo by Lee".into(),
            ..Default::default()
        };
        let children = RightsRecord {
            children_present: Some(true),
            ..Default::default()
        };
        let asset = |rights: RightsRecord| Asset {
            sha256: hash.clone(),
            kind: AssetKind::Picture,
            media_type: "image/png".into(),
            rights,
        };
        let dir = scratch("dup-r6-fold");
        for (tag, records) in [
            (
                "credit-first",
                vec![asset(credit.clone()), asset(children.clone())],
            ),
            (
                "children-first",
                vec![asset(children.clone()), asset(credit.clone())],
            ),
        ] {
            let mut doc = test_doc();
            doc.assets = records;
            doc.asset_bytes.insert(hash.clone(), bytes.to_vec());
            let file = dir.join(format!("{tag}.obd"));
            save(&doc, &file).expect("the file saves");
            let loaded = load(&file).expect("the file loads");
            assert_eq!(loaded.assets.len(), 1, "{tag}: one record after the fold");
            let rights = &loaded.assets[0].rights;
            assert_eq!(rights.children_present, Some(true), "{tag}");
            assert_eq!(rights.credit_text, "", "{tag}");
            assert_eq!(rights.license, LicenseKind::Unknown, "{tag}");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn r7_mirror_pair_folds_on_load_without_its_license() {
        // Round 7 (R7-1 / RT7-1): an .obd whose
        // document.json carries the mirror pair — a
        // record noting children with a CC BY license
        // already standing, and a second record holding
        // only the credit. The credit is refused in the
        // fold, and the composed record must not keep
        // claiming the license whose attribution the
        // fold discarded.
        // FLAGGED, round 7: the older round-5 fold test
        // `r5_duplicate_asset_records_fold_on_load`
        // pins this same composed state (children
        // noted, CC BY standing, no credit anywhere)
        // with its license standing; that assertion is
        // corrected there, as the state is the R7-1
        // BLOCKER state.
        let bytes: &[u8] = b"picture bytes the mirror pair shares";
        let hash = ob_model::sha256_hex(bytes);
        let noting = RightsRecord {
            license: LicenseKind::CcBy("4.0".into()),
            children_present: Some(true),
            ..Default::default()
        };
        let credit = RightsRecord {
            credit_text: "Photo by Alice, CC BY 4.0".into(),
            ..Default::default()
        };
        let asset = |rights: RightsRecord| Asset {
            sha256: hash.clone(),
            kind: AssetKind::Picture,
            media_type: "image/png".into(),
            rights,
        };
        let dir = scratch("dup-r7-fold");
        let mut doc = test_doc();
        doc.assets = vec![asset(noting), asset(credit)];
        doc.asset_bytes.insert(hash.clone(), bytes.to_vec());
        let file = dir.join("mirror.obd");
        save(&doc, &file).expect("the file saves");
        let loaded = load(&file).expect("the file loads");
        assert_eq!(loaded.assets.len(), 1, "one record after the fold");
        let rights = &loaded.assets[0].rights;
        assert_eq!(rights.children_present, Some(true));
        assert_eq!(rights.credit_text, "");
        assert_eq!(rights.license, LicenseKind::Unknown);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn r8_fold_orders_compose_identically_on_load() {
        // Round 8 (R8-9 / R8-R6): a real .obd whose
        // document.json carries three records for one
        // picture — A {children, CC BY, no credit},
        // T {title only}, C {children, credit} — in
        // both orders. The load fold judges the
        // composed rule once, on the folded record, so
        // both orders load as CC BY + credit + title.
        let bytes: &[u8] = b"picture bytes the r8 fold orders share";
        let hash = ob_model::sha256_hex(bytes);
        let credit_text = "Photo by Alice".to_string();
        let title_text = "Harvest supper".to_string();
        let a = RightsRecord {
            children_present: Some(true),
            license: LicenseKind::CcBy("4.0".into()),
            ..Default::default()
        };
        let t = RightsRecord {
            title: title_text.clone(),
            ..Default::default()
        };
        let c = RightsRecord {
            children_present: Some(true),
            credit_text: credit_text.clone(),
            ..Default::default()
        };
        let asset = |rights: RightsRecord| Asset {
            sha256: hash.clone(),
            kind: AssetKind::Picture,
            media_type: "image/png".into(),
            rights,
        };
        let dir = scratch("dup-r8-fold");
        for (tag, records) in [
            ("a-t-c", vec![a.clone(), t.clone(), c.clone()]),
            ("a-c-t", vec![a.clone(), c.clone(), t.clone()]),
        ] {
            let mut doc = test_doc();
            doc.assets = records.into_iter().map(asset).collect();
            doc.asset_bytes.insert(hash.clone(), bytes.to_vec());
            let file = dir.join(format!("{tag}.obd"));
            save(&doc, &file).expect("the file saves");
            let loaded = load(&file).expect("the file loads");
            assert_eq!(loaded.assets.len(), 1, "{tag}: one record after the fold");
            let rights = &loaded.assets[0].rights;
            assert_eq!(rights.children_present, Some(true), "{tag}");
            assert_eq!(rights.license, LicenseKind::CcBy("4.0".into()), "{tag}");
            assert_eq!(rights.credit_text, credit_text, "{tag}");
            assert_eq!(rights.title, title_text, "{tag}");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn r5_duplicate_records_fold_in_a_sheet_file_too() {
        // Round 5 (RT5-1): the same fold in an .obdi.
        // The host holds two records for the sheet's
        // picture; both travel, and the load composes
        // them into the one record the attach reads.
        // FLAGGED, round 6 (R6-A): this test's pair is a
        // ChurchOwned record (no children note) standing
        // first and a children-noted record second, and
        // it pinned the ChurchOwned license standing in
        // the folded record. That composition is exactly
        // the manufactured permission RT6-1 demonstrated:
        // no single record carried the license together
        // with the children note. The license assertion
        // below therefore changed from ChurchOwned to
        // Unknown in R6-A; every other assertion (one
        // folded record, source, title, children note)
        // stands unchanged.
        let dir = scratch("obdi-dup");
        let mut host = test_doc();
        let (ins, hash) = sheet_with_picture(&mut host);
        host.assets.push(Asset {
            sha256: hash.clone(),
            kind: AssetKind::Picture,
            media_type: "image/png".into(),
            rights: RightsRecord {
                title: "Prayer sheet photo".into(),
                children_present: Some(true),
                ..Default::default()
            },
        });
        let path = dir.join("Prayer list.obdi");
        save_insert(&ins, &host, &path).unwrap();
        let (_back, assets, _bytes) = load_insert(&path).unwrap();
        assert_eq!(assets.len(), 1, "the pair folds into one record");
        assert_eq!(assets[0].rights.source, "the church camera");
        assert_eq!(assets[0].rights.license, LicenseKind::Unknown);
        assert_eq!(assets[0].rights.title, "Prayer sheet photo");
        assert_eq!(assets[0].rights.children_present, Some(true));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn r6_duplicate_records_fold_in_a_sheet_file_too() {
        // Round 6 (R6-1): the credit-first pair in an
        // .obdi, in both orders. The folded record notes
        // the children and no permission, whichever
        // record stood first in the file.
        let bytes: &[u8] = b"sheet picture bytes whose fingerprint both records share";
        let hash = ob_model::sha256_hex(bytes);
        let credit = RightsRecord {
            credit_text: "Photo by Lee".into(),
            ..Default::default()
        };
        let children = RightsRecord {
            children_present: Some(true),
            ..Default::default()
        };
        let asset = |rights: RightsRecord| Asset {
            sha256: hash.clone(),
            kind: AssetKind::Picture,
            media_type: "image/png".into(),
            rights,
        };
        let dir = scratch("obdi-r6-fold");
        for (tag, records) in [
            (
                "credit-first",
                vec![asset(credit.clone()), asset(children.clone())],
            ),
            (
                "children-first",
                vec![asset(children.clone()), asset(credit.clone())],
            ),
        ] {
            let mut host = test_doc();
            let (ins, _sheet_hash) = sheet_with_picture(&mut host);
            host.assets = records;
            host.asset_bytes.insert(hash.clone(), bytes.to_vec());
            // Point the sheet's frame at this picture, so
            // both records travel in the file.
            let mut ins = ins;
            ins.panels[0].frames[0].picture = Some(PicturePlacement::new(hash.clone()));
            let path = dir.join(format!("{tag}.obdi"));
            save_insert(&ins, &host, &path).unwrap();
            let (_back, assets, _bytes) = load_insert(&path).unwrap();
            assert_eq!(assets.len(), 1, "{tag}: one record after the fold");
            let rights = &assets[0].rights;
            assert_eq!(rights.children_present, Some(true), "{tag}");
            assert_eq!(rights.credit_text, "", "{tag}");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn sheet_with_picture(host: &mut Document) -> (Insert, String) {
        let hash = host.add_asset(
            b"sheet-picture-bytes",
            AssetKind::Picture,
            "image/png",
            RightsRecord {
                source: "the church camera".into(),
                license: LicenseKind::ChurchOwned,
                ..Default::default()
            },
        );
        let ins = Insert {
            id: "sheet-1".into(),
            name: "Prayer list".into(),
            life: ob_model::InsertLife::ThisWeekOnly,
            keep_through: None,
            panels: vec![Panel {
                background_image: None,
                id: "sheet-1-p1".into(),
                name: "Prayer list".into(),
                size: Size::new(396_000, 612_000),
                frames: vec![Frame {
                    id: "sheet-1-f1".into(),
                    name: "Photo".into(),
                    kind: FrameKind::Picture,
                    rect: Rect::new(0, 0, 100_000, 100_000),
                    story_id: None,
                    role: ContentRole::Standing,
                    picture: Some(PicturePlacement::new(hash.clone())),
                    music: None,
                }],
                guides: vec![],
            }],
            stories: vec![Story {
                id: "sheet-1-s1".into(),
                paragraphs: vec![Paragraph::new("body", "Pray for us")],
                keep_through: None,
            }],
            styles: default_styles(),
            sheet: ob_model::InsertSheet {
                preset: "half-letter".into(),
                copies: 60,
                paper_note: "blue paper".into(),
            },
        };
        (ins, hash)
    }

    #[test]
    fn an_obdi_sheet_round_trips_with_its_picture() {
        let dir = scratch("obdi");
        let mut host = test_doc();
        let (ins, hash) = sheet_with_picture(&mut host);
        // A picture the sheet does not use must not travel.
        host.add_asset(
            b"unrelated-bytes",
            AssetKind::Picture,
            "image/png",
            RightsRecord::default(),
        );
        let path = dir.join("Prayer list.obdi");
        save_insert(&ins, &host, &path).unwrap();
        let head = std::fs::read(&path).unwrap();
        assert_eq!(&head[..4], b"PK\x03\x04", "an .obdi is the v1 container");
        let (back, assets, bytes) = load_insert(&path).unwrap();
        assert_eq!(back, ins);
        assert_eq!(
            bytes.get(&hash).map(|b| b.as_slice()),
            Some(b"sheet-picture-bytes".as_slice())
        );
        assert_eq!(bytes.len(), 1, "only the used picture travels");
        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].sha256, hash);
        assert_eq!(assets[0].rights.source, "the church camera");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_obdi_with_swapped_picture_bytes_is_refused() {
        let dir = scratch("obdi-tamper");
        let mut host = test_doc();
        let (ins, hash) = sheet_with_picture(&mut host);
        let path = dir.join("sheet.obdi");
        save_insert(&ins, &host, &path).unwrap();
        // Rewrite the zip with the asset entry's bytes
        // swapped: the entry still answers to the old hash.
        let file = std::fs::File::open(&path).unwrap();
        let mut zip = zip::ZipArchive::new(file).unwrap();
        let mut insert_json = String::new();
        zip.by_name("insert.json")
            .unwrap()
            .read_to_string(&mut insert_json)
            .unwrap();
        drop(zip);
        let bad = dir.join("bad.obdi");
        {
            let cursor = std::io::Cursor::new(Vec::new());
            let mut zw = zip::ZipWriter::new(cursor);
            let opts = zip::write::SimpleFileOptions::default();
            zw.start_file("insert.json", opts).unwrap();
            zw.write_all(insert_json.as_bytes()).unwrap();
            zw.start_file(format!("assets/{hash}"), opts).unwrap();
            zw.write_all(b"swapped-bytes").unwrap();
            let cursor = zw.finish().unwrap();
            std::fs::write(&bad, cursor.into_inner()).unwrap();
        }
        let err = load_insert(&bad).expect_err("tampered bytes refuse");
        assert!(err.contains("does not match its fingerprint"), "err: {err}");
        // And a file that is not a container at all is
        // refused in plain words, not misparsed.
        let not_zip = dir.join("plain.obdi");
        std::fs::write(&not_zip, b"just some text").unwrap();
        let err = load_insert(&not_zip).expect_err("not a sheet file");
        assert!(err.contains("not a loose sheet"), "err: {err}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_bulletin_written_before_loose_sheets_loads_with_none() {
        // The inserts key is simply absent from an old
        // file; loading must come back with no sheets.
        let mut doc = test_doc();
        doc.inserts = vec![];
        let mut json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&doc).expect("json")).expect("value");
        json.as_object_mut().expect("object").remove("inserts");
        let dir = scratch("noinserts");
        let path = dir.join("old.obd");
        std::fs::write(&path, json.to_string()).expect("write");
        let loaded = load(&path).expect("loads");
        assert!(loaded.inserts.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn files_written_before_keep_through_and_no_announcements_load() {
        // A v0 file from before the keep-through date and the
        // no-announcements mark existed: neither field is in the
        // JSON, and both must come back as their defaults (no
        // keep-through, announcements not marked none).
        let json = r#"{
            "format_version": 0,
            "title": "Old file",
            "profile": {"name": "Grace", "address": "", "service_times": "",
                        "website": "", "giving_url": "", "pastor": ""},
            "fields": {"sunday_date": "2026-10-18", "sunday_name": "Pentecost"},
            "styles": [],
            "stories": [{"id": "s", "paragraphs": [{"style": "body", "text": "Hello"}]}],
            "panels": [],
            "sheet": {"preset": "letter-portrait", "size": {"w": 1, "h": 2}}
        }"#;
        let dir = scratch("old");
        let path = dir.join("old.obd");
        std::fs::write(&path, json).expect("write");
        let doc = load(&path).expect("old file loads");
        assert_eq!(doc.stories[0].keep_through, None);
        assert!(!doc.no_announcements);
        // The occasion keys were renamed on the way in.
        assert_eq!(doc.fields["occasion_date"], "2026-10-18");
        assert_eq!(doc.fields["occasion_name"], "Pentecost");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Find a byte pattern in a slice (fixture plumbing).
    fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        haystack.windows(needle.len()).position(|w| w == needle)
    }

    /// Rewrite the declared uncompressed size of one
    /// entry, in both its local header and its central
    /// directory record, leaving the real bytes and the
    /// honest checksum untouched: the red team's lie
    /// (round 3, RT3-B). The declared field sits at a
    /// fixed distance ahead of the entry's name in each
    /// record, 12 bytes in a local header and 22 in a
    /// central one, and the record's signature says
    /// which record a name belongs to.
    fn patch_declared_size(zip_bytes: &mut [u8], entry_name: &str, declared: u32) {
        let name = entry_name.as_bytes();
        let mut patched = 0;
        let mut from = 0;
        while let Some(found) = find_bytes(&zip_bytes[from..], name) {
            let at = from + found;
            if at >= 30 && &zip_bytes[at - 30..at - 26] == b"PK\x03\x04" {
                zip_bytes[at - 12..at - 8].copy_from_slice(&declared.to_le_bytes());
                patched += 1;
            } else if at >= 46 && &zip_bytes[at - 46..at - 42] == b"PK\x01\x02" {
                zip_bytes[at - 22..at - 18].copy_from_slice(&declared.to_le_bytes());
                patched += 1;
            }
            from = at + 1;
        }
        assert_eq!(patched, 2, "both records for {entry_name} were patched");
    }

    #[test]
    fn capped_reads_count_actual_bytes_not_declared_ones() {
        // One 100-byte entry in a fresh in-memory zip
        // each time. A cap of 100 reads it whole; a cap
        // of 50 refuses it, and patching the declared
        // size down to 5 changes neither outcome,
        // because the count follows the bytes that
        // actually arrive.
        let honest = || {
            let mut buf = Vec::new();
            {
                let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
                zip.start_file(
                    "entry.bin",
                    zip::write::SimpleFileOptions::default()
                        .compression_method(zip::CompressionMethod::Deflated),
                )
                .expect("start");
                zip.write_all(&[7u8; 100]).expect("write");
                zip.finish().expect("finish");
            }
            buf
        };
        let mut lying = honest();
        patch_declared_size(&mut lying, "entry.bin", 5);
        for bytes in [honest(), lying] {
            let mut zip =
                zip::ZipArchive::new(std::io::Cursor::new(&bytes)).expect("archive opens");
            let mut entry = zip.by_index(0).expect("entry");
            assert_eq!(
                read_entry_capped(&mut entry, 100, "too large")
                    .expect("at the cap reads")
                    .len(),
                100
            );
            drop(entry);
            let mut entry = zip.by_index(0).expect("entry again");
            let err =
                read_entry_capped(&mut entry, 50, "too large").expect_err("past the cap refuses");
            assert_eq!(err, "too large");
        }
    }

    #[test]
    fn a_bulletin_whose_directory_lies_about_its_size_is_refused() {
        // The red team's .obd bomb: one asset of zeroes
        // a byte past the whole-container cap, deflated
        // down to a small file, with its declared size
        // patched down to 5 bytes. The old loader summed
        // the declarations, saw a tiny file, and read
        // the whole thing into memory. The loader now
        // counts actual bytes and refuses.
        let zeros = vec![0u8; (MAX_TOTAL_UNCOMPRESSED + 1) as usize];
        let mut doc = test_doc();
        let hash = doc.add_asset(
            &zeros,
            AssetKind::Picture,
            "image/png",
            RightsRecord::default(),
        );
        let dir = scratch("bomb-obd");
        let honest_path = dir.join("honest.obd");
        save(&doc, &honest_path).expect("save");
        assert!(load(&honest_path).is_err(), "even honest, it is too big");
        let mut bytes = std::fs::read(&honest_path).expect("read");
        let entry_name = format!("assets/{hash}");
        patch_declared_size(&mut bytes, &entry_name, 5);
        let lying_path = dir.join("lying.obd");
        std::fs::write(&lying_path, &bytes).expect("write lying copy");
        let err = load(&lying_path).expect_err("the lie no longer opens it");
        assert_eq!(err, TOTAL_TOO_LARGE);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_loose_sheet_whose_directory_lies_about_its_size_is_refused() {
        // The same bomb through the .obdi door.
        let zeros = vec![0u8; (MAX_TOTAL_UNCOMPRESSED + 1) as usize];
        let mut host = test_doc();
        let hash = host.add_asset(
            &zeros,
            AssetKind::Picture,
            "image/png",
            RightsRecord::default(),
        );
        let insert = Insert {
            id: "sheet-big".into(),
            name: "Big art".into(),
            life: ob_model::InsertLife::ThisWeekOnly,
            keep_through: None,
            panels: vec![Panel {
                background_image: None,
                id: "sheet-big-p1".into(),
                name: "Big art".into(),
                size: Size::new(396_000, 612_000),
                frames: vec![Frame {
                    id: "sheet-big-f1".into(),
                    name: "Photo".into(),
                    kind: FrameKind::Picture,
                    rect: Rect::new(0, 0, 100_000, 100_000),
                    story_id: None,
                    role: ContentRole::Standing,
                    picture: Some(PicturePlacement::new(hash.clone())),
                    music: None,
                }],
                guides: vec![],
            }],
            stories: vec![],
            styles: default_styles(),
            sheet: ob_model::InsertSheet {
                preset: "half-letter".into(),
                copies: 1,
                paper_note: String::new(),
            },
        };
        let dir = scratch("bomb-obdi");
        let honest_path = dir.join("honest.obdi");
        save_insert(&insert, &host, &honest_path).expect("save");
        let mut bytes = std::fs::read(&honest_path).expect("read");
        let entry_name = format!("assets/{hash}");
        patch_declared_size(&mut bytes, &entry_name, 5);
        let lying_path = dir.join("lying.obdi");
        std::fs::write(&lying_path, &bytes).expect("write lying copy");
        let err = load_insert(&lying_path).expect_err("the lie no longer opens it");
        assert_eq!(err, TOTAL_TOO_LARGE);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A zip holding one JSON entry under `json_name`
    /// plus `fillers` one-byte stored entries behind
    /// it: the shape the entry cap counts (round 4,
    /// RT-2). Nothing in it is large; only numerous.
    fn zip_with_fillers(json_name: &str, json: &str, fillers: usize) -> Vec<u8> {
        let stored = || {
            zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored)
        };
        let mut buf = Vec::new();
        {
            let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
            zip.start_file(json_name, stored()).expect("json entry");
            zip.write_all(json.as_bytes()).expect("json bytes");
            for i in 0..fillers {
                zip.start_file(format!("filler/{i}"), stored())
                    .expect("filler entry");
                zip.write_all(b"x").expect("filler bytes");
            }
            zip.finish().expect("finish");
        }
        buf
    }

    #[test]
    fn a_bulletin_with_too_many_parts_is_refused_before_any_is_read() {
        // Round 4, RT-2: a file of 100,001 tiny assets
        // sat under every byte cap and still took
        // minutes to open, because nothing counted its
        // parts. The count now comes from the archive's
        // entry list, before any payload is read: one
        // part past the cap refuses the whole file, and
        // a file AT the cap still opens.
        let json = serde_json::to_string(&test_doc()).expect("json");
        let dir = scratch("many-parts-obd");
        let over = dir.join("over.obd");
        std::fs::write(&over, zip_with_fillers("document.json", &json, MAX_ENTRIES))
            .expect("write over");
        let err = load(&over).expect_err("one part past the cap refuses");
        assert_eq!(err, TOO_MANY_PARTS);
        let at = dir.join("at.obd");
        std::fs::write(
            &at,
            zip_with_fillers("document.json", &json, MAX_ENTRIES - 1),
        )
        .expect("write at");
        load(&at).expect("a file at the cap still opens");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_loose_sheet_with_too_many_parts_is_refused_before_any_is_read() {
        // The same count at the sheet door (round 4,
        // RT-2).
        let package = InsertPackage {
            insert: Insert {
                id: "sheet-many".into(),
                name: "Many parts".into(),
                life: ob_model::InsertLife::ThisWeekOnly,
                keep_through: None,
                panels: vec![],
                stories: vec![],
                styles: default_styles(),
                sheet: ob_model::InsertSheet {
                    preset: "half-letter".into(),
                    copies: 1,
                    paper_note: String::new(),
                },
            },
            assets: vec![],
        };
        let json = serde_json::to_string(&package).expect("json");
        let dir = scratch("many-parts-obdi");
        let over = dir.join("over.obdi");
        std::fs::write(&over, zip_with_fillers("insert.json", &json, MAX_ENTRIES))
            .expect("write over");
        let err = load_insert(&over).expect_err("one part past the cap refuses");
        assert_eq!(err, TOO_MANY_PARTS);
        let at = dir.join("at.obdi");
        std::fs::write(&at, zip_with_fillers("insert.json", &json, MAX_ENTRIES - 1))
            .expect("write at");
        load_insert(&at).expect("a file at the cap still opens");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Rewrite every filler name in a zip built by
    /// `zip_with_fillers`-style code to one shared name,
    /// in a single pass: the digits after "dup/" become
    /// "shared" in the local header and the central
    /// directory alike (the header guards keep payload
    /// bytes from being mistaken for a name).
    fn patch_all_filler_names(zip_bytes: &mut [u8]) {
        let mut patched = 0;
        let mut from = 0;
        while let Some(found) = find_bytes(&zip_bytes[from..], b"dup/") {
            let at = from + found;
            let guarded = (at >= 30 && &zip_bytes[at - 30..at - 26] == b"PK\x03\x04")
                || (at >= 46 && &zip_bytes[at - 46..at - 42] == b"PK\x01\x02");
            if guarded {
                zip_bytes[at + 4..at + 10].copy_from_slice(b"shared");
                patched += 1;
            }
            from = at + 1;
        }
        assert!(patched >= 2, "the fillers' names were patched");
    }

    /// A zip holding one JSON entry plus `fillers`
    /// one-byte stored entries that ALL share one name:
    /// the shape that walked past the old cap, which
    /// counted distinct names (round 5, RT5-5). The
    /// writer refuses a duplicate name outright, so the
    /// fillers are written under stand-in names of the
    /// same length and patched to the shared one.
    fn zip_with_same_name_fillers(json_name: &str, json: &str, fillers: usize) -> Vec<u8> {
        let stored = || {
            zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored)
        };
        let mut buf = Vec::new();
        {
            let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
            zip.start_file(json_name, stored()).expect("json entry");
            zip.write_all(json.as_bytes()).expect("json bytes");
            for i in 0..fillers {
                zip.start_file(format!("dup/{i:06}"), stored())
                    .expect("filler entry");
                zip.write_all(b"x").expect("filler bytes");
            }
            zip.finish().expect("finish");
        }
        patch_all_filler_names(&mut buf);
        buf
    }

    #[test]
    fn same_name_entries_count_against_the_cap_in_a_bulletin() {
        // Round 5, RT5-5: the cap counts entries as
        // written, not distinct names. A file whose
        // 8,192 fillers all share one name holds 8,193
        // entries and is refused in the cap's own words,
        // while the same shape one entry under the cap
        // (8,192 raw entries, only two distinct names)
        // still opens.
        let json = serde_json::to_string(&test_doc()).expect("json");
        let dir = scratch("dup-parts-obd");
        let over = dir.join("over.obd");
        std::fs::write(
            &over,
            zip_with_same_name_fillers("document.json", &json, MAX_ENTRIES),
        )
        .expect("write over");
        let err = load(&over).expect_err("same-name entries past the cap refuse");
        assert_eq!(err, TOO_MANY_PARTS);
        let at = dir.join("at.obd");
        std::fs::write(
            &at,
            zip_with_same_name_fillers("document.json", &json, MAX_ENTRIES - 1),
        )
        .expect("write at");
        load(&at).expect("a file at the raw cap still opens");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn same_name_entries_count_against_the_cap_in_a_loose_sheet() {
        // The same raw count at the sheet door (round 5,
        // RT5-5).
        let package = InsertPackage {
            insert: Insert {
                id: "sheet-dup".into(),
                name: "Same names".into(),
                life: ob_model::InsertLife::ThisWeekOnly,
                keep_through: None,
                panels: vec![],
                stories: vec![],
                styles: default_styles(),
                sheet: ob_model::InsertSheet {
                    preset: "half-letter".into(),
                    copies: 1,
                    paper_note: String::new(),
                },
            },
            assets: vec![],
        };
        let json = serde_json::to_string(&package).expect("json");
        let dir = scratch("dup-parts-obdi");
        let over = dir.join("over.obdi");
        std::fs::write(
            &over,
            zip_with_same_name_fillers("insert.json", &json, MAX_ENTRIES),
        )
        .expect("write over");
        let err = load_insert(&over).expect_err("same-name entries past the cap refuse");
        assert_eq!(err, TOO_MANY_PARTS);
        let at = dir.join("at.obdi");
        std::fs::write(
            &at,
            zip_with_same_name_fillers("insert.json", &json, MAX_ENTRIES - 1),
        )
        .expect("write at");
        load_insert(&at).expect("a sheet at the raw cap still opens");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The end-of-directory record's position in a zip
    /// built by the writer helpers: the last PK\x05\x06
    /// whose comment lands on the end of the file.
    fn find_eocd(zip_bytes: &[u8]) -> usize {
        let mut found = None;
        let mut from = 0;
        while let Some(hit) = find_bytes(&zip_bytes[from..], b"PK\x05\x06") {
            let at = from + hit;
            if at + 22 <= zip_bytes.len() {
                let comment = u16::from_le_bytes([zip_bytes[at + 20], zip_bytes[at + 21]]) as usize;
                if at + 22 + comment == zip_bytes.len() {
                    found = Some(at);
                }
            }
            from = at + 1;
        }
        found.expect("an end record")
    }

    /// Splice a zip64 end record and locator in front
    /// of a plain zip's end record, the plain fields
    /// turned to their sentinels: the shape the round-6
    /// red team's zip64 fixtures took. The zip64
    /// record's own total-count and directory-size
    /// fields are the caller's to set — honest, or
    /// lying in either direction.
    fn splice_zip64(zip_bytes: &[u8], total: u64, cd_size: u64) -> Vec<u8> {
        let eocd = find_eocd(zip_bytes);
        let cd_offset = u64::from(u32::from_le_bytes(
            zip_bytes[eocd + 16..eocd + 20]
                .try_into()
                .expect("offset field"),
        ));
        let mut out = zip_bytes[..eocd].to_vec();
        let mut record = Vec::new();
        record.extend_from_slice(b"PK\x06\x06");
        record.extend_from_slice(&44u64.to_le_bytes());
        record.extend_from_slice(&45u16.to_le_bytes());
        record.extend_from_slice(&45u16.to_le_bytes());
        record.extend_from_slice(&0u32.to_le_bytes());
        record.extend_from_slice(&0u32.to_le_bytes());
        record.extend_from_slice(&total.to_le_bytes());
        record.extend_from_slice(&total.to_le_bytes());
        record.extend_from_slice(&cd_size.to_le_bytes());
        record.extend_from_slice(&cd_offset.to_le_bytes());
        assert_eq!(record.len(), 56);
        out.extend_from_slice(&record);
        let mut locator = Vec::new();
        locator.extend_from_slice(b"PK\x06\x07");
        locator.extend_from_slice(&0u32.to_le_bytes());
        locator.extend_from_slice(&(eocd as u64).to_le_bytes());
        locator.extend_from_slice(&1u32.to_le_bytes());
        assert_eq!(locator.len(), 20);
        out.extend_from_slice(&locator);
        let mut tail = zip_bytes[eocd..].to_vec();
        tail[8..10].copy_from_slice(&u16::MAX.to_le_bytes());
        tail[10..12].copy_from_slice(&u16::MAX.to_le_bytes());
        tail[12..16].copy_from_slice(&u32::MAX.to_le_bytes());
        tail[16..20].copy_from_slice(&u32::MAX.to_le_bytes());
        out.extend_from_slice(&tail);
        out
    }

    /// One loose-sheet package's JSON, for the lying
    /// directory tests at the sheet door.
    fn lying_sheet_json() -> String {
        let package = InsertPackage {
            insert: Insert {
                id: "sheet-lying".into(),
                name: "Lying directory".into(),
                life: ob_model::InsertLife::ThisWeekOnly,
                keep_through: None,
                panels: vec![],
                stories: vec![],
                styles: default_styles(),
                sheet: ob_model::InsertSheet {
                    preset: "half-letter".into(),
                    copies: 1,
                    paper_note: String::new(),
                },
            },
            assets: vec![],
        };
        serde_json::to_string(&package).expect("json")
    }

    #[test]
    fn a_lying_directory_size_or_count_does_not_hide_entries_past_the_cap() {
        // Round 6 (RT6-7): the count walk used to follow
        // the directory's declared SIZE, so patching it
        // down to the first record let 9,001 entries
        // load. The count is walked from the records
        // themselves now: declared fields are ignored in
        // either direction — a size shrunk to one record
        // or a count field claiming two both still count
        // 9,001, and both loaders refuse.
        let doc_json = serde_json::to_string(&test_doc()).expect("json");
        let sheet_json = lying_sheet_json();
        for (json_name, json, ext) in [
            ("document.json", doc_json.as_str(), "obd"),
            ("insert.json", sheet_json.as_str(), "obdi"),
        ] {
            let dir = scratch(&format!("lying-dir-{ext}"));
            let honest = zip_with_same_name_fillers(json_name, json, 9_000);
            let eocd = find_eocd(&honest);
            // The size lie: the directory claims to cover
            // only its first record.
            let first_record = 46 + json_name.len() as u32;
            let mut size_lie = honest.clone();
            size_lie[eocd + 12..eocd + 16].copy_from_slice(&first_record.to_le_bytes());
            // The count lie: both count fields claim two.
            let mut count_lie = honest;
            count_lie[eocd + 8..eocd + 10].copy_from_slice(&2u16.to_le_bytes());
            count_lie[eocd + 10..eocd + 12].copy_from_slice(&2u16.to_le_bytes());
            for (tag, bytes) in [("size", size_lie), ("count", count_lie)] {
                let path = dir.join(format!("{tag}-lie.{ext}"));
                std::fs::write(&path, &bytes).expect("write lying copy");
                assert_eq!(
                    raw_entry_count(&path, MAX_ENTRIES),
                    Some(MAX_ENTRIES + 1),
                    "{ext} {tag} lie: the walk counts records until it passes \
                     the cap; the declared fields play no part"
                );
                let err = if ext == "obd" {
                    load(&path).expect_err("the flood refuses at the bulletin door")
                } else {
                    load_insert(&path).expect_err("the flood refuses at the sheet door")
                };
                assert_eq!(err, TOO_MANY_PARTS, "{ext} {tag} lie");
            }
            let _ = std::fs::remove_dir_all(&dir);
        }
    }

    #[test]
    fn a_lying_zip64_directory_is_counted_by_its_records() {
        // Round 6 (RT6-7): the same lies in a spliced
        // zip64 end record — its size field shrunk, or
        // its count field claiming two — fooled the old
        // walk the same way. Both loaders refuse: the
        // walk counts the 9,001 records in front of the
        // zip64 record and never consults its totals.
        let doc_json = serde_json::to_string(&test_doc()).expect("json");
        let sheet_json = lying_sheet_json();
        for (json_name, json, ext) in [
            ("document.json", doc_json.as_str(), "obd"),
            ("insert.json", sheet_json.as_str(), "obdi"),
        ] {
            let dir = scratch(&format!("lying-zip64-{ext}"));
            let honest = zip_with_same_name_fillers(json_name, json, 9_000);
            let eocd = find_eocd(&honest);
            let honest_size = u64::from(u32::from_le_bytes(
                honest[eocd + 12..eocd + 16].try_into().expect("size field"),
            ));
            let first_record = 46 + json_name.len() as u64;
            for (tag, bytes) in [
                ("size", splice_zip64(&honest, 9_001, first_record)),
                ("count", splice_zip64(&honest, 2, honest_size)),
            ] {
                let path = dir.join(format!("{tag}-lie.{ext}"));
                std::fs::write(&path, &bytes).expect("write lying copy");
                assert_eq!(
                    raw_entry_count(&path, MAX_ENTRIES),
                    Some(MAX_ENTRIES + 1),
                    "{ext} zip64 {tag} lie: the walk counts records until it \
                     passes the cap; the zip64 totals play no part"
                );
                let err = if ext == "obd" {
                    load(&path).expect_err("the flood refuses at the bulletin door")
                } else {
                    load_insert(&path).expect_err("the flood refuses at the sheet door")
                };
                assert_eq!(err, TOO_MANY_PARTS, "{ext} zip64 {tag} lie");
            }
            let _ = std::fs::remove_dir_all(&dir);
        }
    }

    #[test]
    fn a_declared_huge_size_with_few_real_entries_still_loads() {
        // Round 6 (RT6-7), the other direction: a
        // directory size claiming far more than the file
        // holds, with two real entries in it. The
        // declared size is ignored, the two records are
        // counted, and the file loads — plain and zip64.
        let doc_json = serde_json::to_string(&test_doc()).expect("json");
        let sheet_json = lying_sheet_json();
        for (json_name, json, ext) in [
            ("document.json", doc_json.as_str(), "obd"),
            ("insert.json", sheet_json.as_str(), "obdi"),
        ] {
            let dir = scratch(&format!("huge-size-{ext}"));
            let small = zip_with_fillers(json_name, json, 1);
            let eocd = find_eocd(&small);
            let mut plain = small.clone();
            plain[eocd + 12..eocd + 16].copy_from_slice(&0xFFFF_FFF0u32.to_le_bytes());
            let zip64 = splice_zip64(&small, 2, 0xFFFF_FFFF_FFFF);
            for (tag, bytes) in [("plain", plain), ("zip64", zip64)] {
                let path = dir.join(format!("{tag}-huge.{ext}"));
                std::fs::write(&path, &bytes).expect("write copy");
                assert_eq!(
                    raw_entry_count(&path, MAX_ENTRIES),
                    Some(2),
                    "{ext} {tag}: two records, whatever the size claims"
                );
                if ext == "obd" {
                    load(&path).expect("a huge declared size with few entries loads");
                } else {
                    load_insert(&path).expect("a huge declared size with few entries loads");
                }
            }
            let _ = std::fs::remove_dir_all(&dir);
        }
    }

    #[test]
    fn junk_around_the_archive_shifts_the_count_but_not_the_answer() {
        // Round 6 (R6-10 in the review lane, RT6-7's
        // tolerance half): bytes prepended to an archive
        // shift every offset the directory declares, and
        // bytes after the end record push it off the end
        // of the file. The count walk now locates the
        // end record and the directory the reader's way,
        // so the count is exact under either kind of
        // junk. What the LOADERS then do with a junked
        // file is pinned as it actually stands (this
        // test IS the pin): trailing junk loads, and a
        // prepended file is refused at the loaders' head
        // check — while the cap check itself counts the
        // shifted flood and refuses it.
        let doc_json = serde_json::to_string(&test_doc()).expect("json");
        let junk: Vec<u8> = b"self-extracting stub bytes, no signatures here ".repeat(13);
        let dir = scratch("junk-around");
        let small = zip_with_fillers("document.json", &doc_json, 1);
        // Prepended: the WALK counts the shifted archive
        // exactly (2), but the loaders never reach the
        // archive: both dispatch on the file's first
        // four bytes, which are junk here, not the zip
        // signature. That is what the shipped reader
        // does with a prepended file today (the round-6
        // probe's Z3 recorded the same refusal), and
        // this test pins it rather than wishing it
        // otherwise.
        let mut prepended = junk.clone();
        prepended.extend_from_slice(&small);
        let prepended_path = dir.join("prepended.obd");
        std::fs::write(&prepended_path, &prepended).expect("write prepended");
        assert_eq!(raw_entry_count(&prepended_path, MAX_ENTRIES), Some(2));
        let err = load(&prepended_path).expect_err("a prepended file is refused at the door");
        assert!(err.contains("UTF-8"), "the v0 words, as probed: {err}");
        // Trailing junk is different: the head is a real
        // signature, the reader tolerates the end record
        // sitting off the end of the file, and the file
        // loads.
        let mut trailed = small;
        trailed.extend_from_slice(&junk);
        let trailed_path = dir.join("trailed.obd");
        std::fs::write(&trailed_path, &trailed).expect("write trailed");
        assert_eq!(raw_entry_count(&trailed_path, MAX_ENTRIES), Some(2));
        load(&trailed_path).expect("the reader loads a trailed archive, and so do we");
        // Over the cap, with the same junk in front: the
        // count itself passes the cap (the cap check
        // refuses it in the cap's own words), and the
        // loaders refuse the file as well — at their
        // head check, before the archive is opened.
        let sheet_json = lying_sheet_json();
        for (json_name, json, ext) in [
            ("document.json", doc_json.as_str(), "obd"),
            ("insert.json", sheet_json.as_str(), "obdi"),
        ] {
            let big = zip_with_same_name_fillers(json_name, json, 9_000);
            let mut shifted = junk.clone();
            shifted.extend_from_slice(&big);
            let path = dir.join(format!("shifted-over.{ext}"));
            std::fs::write(&path, &shifted).expect("write shifted flood");
            assert_eq!(
                raw_entry_count(&path, MAX_ENTRIES),
                Some(MAX_ENTRIES + 1),
                "{ext}: the walk stops just past the cap, junk or not"
            );
            assert_eq!(
                check_entry_cap(&path),
                Err(TOO_MANY_PARTS.to_string()),
                "{ext}: the cap itself refuses the shifted flood"
            );
            if ext == "obd" {
                load(&path).expect_err("a shifted flood is refused at the bulletin door");
            } else {
                let err =
                    load_insert(&path).expect_err("a shifted flood is refused at the sheet door");
                assert_eq!(err, "that file is not a loose sheet file (.obdi)");
            }
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The bytes of the `index`-th central directory
    /// record in a zip built by the writer helpers,
    /// measured by its own header lengths, the walk's
    /// own measure.
    fn central_record(zip_bytes: &[u8], index: usize) -> Vec<u8> {
        let eocd = find_eocd(zip_bytes);
        let mut at = u32::from_le_bytes(
            zip_bytes[eocd + 16..eocd + 20]
                .try_into()
                .expect("offset field"),
        ) as usize;
        // Advance to the requested record, then take it.
        for _ in 0..index {
            let name_len = u16::from_le_bytes([zip_bytes[at + 28], zip_bytes[at + 29]]) as usize;
            let extra_len = u16::from_le_bytes([zip_bytes[at + 30], zip_bytes[at + 31]]) as usize;
            let comment_len = u16::from_le_bytes([zip_bytes[at + 32], zip_bytes[at + 33]]) as usize;
            at += 46 + name_len + extra_len + comment_len;
        }
        let name_len = u16::from_le_bytes([zip_bytes[at + 28], zip_bytes[at + 29]]) as usize;
        let extra_len = u16::from_le_bytes([zip_bytes[at + 30], zip_bytes[at + 31]]) as usize;
        let comment_len = u16::from_le_bytes([zip_bytes[at + 32], zip_bytes[at + 33]]) as usize;
        zip_bytes[at..at + 46 + name_len + extra_len + comment_len].to_vec()
    }

    /// Append trailing junk holding a FAKE central
    /// directory — two copies of `record` — and a fake
    /// end record claiming those two entries: the
    /// round-7 red team's f1 shape. The reader's
    /// latest-first rule reads the fake directory; a
    /// copied document record still points at the real
    /// local header, so the fake directory even serves
    /// the real bulletin.
    fn append_fake_directory(base: &[u8], record: &[u8]) -> Vec<u8> {
        let mut out = base.to_vec();
        let cd_offset = out.len() as u32;
        out.extend_from_slice(record);
        out.extend_from_slice(record);
        let cd_size = (record.len() * 2) as u32;
        let mut eocd = Vec::new();
        eocd.extend_from_slice(b"PK\x05\x06");
        eocd.extend_from_slice(&0u16.to_le_bytes());
        eocd.extend_from_slice(&0u16.to_le_bytes());
        eocd.extend_from_slice(&2u16.to_le_bytes());
        eocd.extend_from_slice(&2u16.to_le_bytes());
        eocd.extend_from_slice(&cd_size.to_le_bytes());
        eocd.extend_from_slice(&cd_offset.to_le_bytes());
        eocd.extend_from_slice(&0u16.to_le_bytes());
        assert_eq!(eocd.len(), 22);
        out.extend_from_slice(&eocd);
        out
    }

    #[test]
    fn a_fake_directory_in_trailing_junk_does_not_hide_a_flood() {
        // Round 7 (RT7-5): a 9,001-entry archive plus
        // appended trailing junk holding a fake
        // 2-record central directory and a fake end
        // record loaded as an ordinary bulletin on
        // both loaders: the walk located the end record
        // latest-first, exactly like the shipped
        // reader, so both counted and read the
        // attacker's fake directory. The cap judges
        // the file now: every end-record candidate is
        // counted, and the real directory's 9,001
        // entries refuse the file in the cap's own
        // words, at both doors, whether the fake
        // directory carries the document record (f1b)
        // or not (f1a).
        let doc_json = serde_json::to_string(&test_doc()).expect("json");
        let sheet_json = lying_sheet_json();
        for (json_name, json, ext) in [
            ("document.json", doc_json.as_str(), "obd"),
            ("insert.json", sheet_json.as_str(), "obdi"),
        ] {
            let dir = scratch(&format!("fake-dir-{ext}"));
            let flood = zip_with_same_name_fillers(json_name, json, 9_000);
            let doc_record = central_record(&flood, 0);
            let filler_record = central_record(&flood, 1);
            for (tag, record) in [
                ("f1b-with-doc", &doc_record),
                ("f1a-no-doc", &filler_record),
            ] {
                let bytes = append_fake_directory(&flood, record);
                let path = dir.join(format!("{tag}.{ext}"));
                std::fs::write(&path, &bytes).expect("write fake-directory copy");
                assert_eq!(
                    raw_entry_count(&path, MAX_ENTRIES),
                    Some(MAX_ENTRIES + 1),
                    "{ext} {tag}: some candidate directory holds the flood; \
                     the fake one's count of two must not be the answer"
                );
                assert_eq!(
                    check_entry_cap(&path),
                    Err(TOO_MANY_PARTS.to_string()),
                    "{ext} {tag}: the cap itself refuses"
                );
                let err = load(&path).expect_err("the flood refuses at the bulletin door");
                assert_eq!(err, TOO_MANY_PARTS, "{ext} {tag} at the bulletin door");
                let err = load_insert(&path).expect_err("the flood refuses at the sheet door");
                assert_eq!(err, TOO_MANY_PARTS, "{ext} {tag} at the sheet door");
            }
            // The held shape beside it: trailing junk
            // whose only directory is honest and small
            // still loads, and counts as it always has.
            let mut trailed = zip_with_fillers(json_name, json, 1);
            let junk: Vec<u8> = b"trailing bytes, no signatures here ".repeat(7);
            trailed.extend_from_slice(&junk);
            let trailed_path = dir.join(format!("trailed.{ext}"));
            std::fs::write(&trailed_path, &trailed).expect("write trailed copy");
            assert_eq!(raw_entry_count(&trailed_path, MAX_ENTRIES), Some(2));
            if ext == "obd" {
                load(&trailed_path).expect("honest trailing junk still loads");
            } else {
                load_insert(&trailed_path).expect("honest trailing junk still loads");
            }
            let _ = std::fs::remove_dir_all(&dir);
        }
    }

    /// Append one genuinely unwalkable junk end
    /// record: it declares seven entries and a
    /// directory window of the eight bytes just
    /// before it, in which no central-header
    /// signature stands, so its count walk returns
    /// None (round 8, R8-1). Junk records like this
    /// burn candidate slots without ever being
    /// counted themselves.
    fn append_junk_end_record(base: &[u8]) -> Vec<u8> {
        let mut out = base.to_vec();
        let at = out.len() as u32;
        let mut eocd = Vec::new();
        eocd.extend_from_slice(b"PK\x05\x06");
        eocd.extend_from_slice(&0u16.to_le_bytes());
        eocd.extend_from_slice(&0u16.to_le_bytes());
        eocd.extend_from_slice(&7u16.to_le_bytes());
        eocd.extend_from_slice(&7u16.to_le_bytes());
        eocd.extend_from_slice(&8u32.to_le_bytes());
        eocd.extend_from_slice(&(at - 8).to_le_bytes());
        eocd.extend_from_slice(&0u16.to_le_bytes());
        assert_eq!(eocd.len(), 22);
        out.extend_from_slice(&eocd);
        out
    }

    #[test]
    fn junk_end_records_and_many_decoys_do_not_hide_a_flood() {
        // Round 8 (R8-1, all three lanes): the
        // candidate enumeration used to stop at
        // sixteen candidates. Fifteen unwalkable junk
        // end records plus one fake directory — in
        // either order — pushed a 9,001-entry flood's
        // own end record to position seventeen, where
        // it was never counted, and the file loaded
        // through the fake directory; sixteen decoy
        // directories did the same. The enumeration
        // collects every candidate now, so each of
        // these shapes is counted and refused with
        // the cap sentence at both doors.
        let doc_json = serde_json::to_string(&test_doc()).expect("json");
        let sheet_json = lying_sheet_json();
        for (json_name, json, ext) in [
            ("document.json", doc_json.as_str(), "obd"),
            ("insert.json", sheet_json.as_str(), "obdi"),
        ] {
            let dir = scratch(&format!("junk-candidates-{ext}"));
            let flood = zip_with_same_name_fillers(json_name, json, 9_000);
            let doc_record = central_record(&flood, 0);
            let mut junk_then_fake = flood.clone();
            for _ in 0..15 {
                junk_then_fake = append_junk_end_record(&junk_then_fake);
            }
            junk_then_fake = append_fake_directory(&junk_then_fake, &doc_record);
            let mut fake_then_junk = append_fake_directory(&flood, &doc_record);
            for _ in 0..15 {
                fake_then_junk = append_junk_end_record(&fake_then_junk);
            }
            let mut decoys16 = flood.clone();
            for _ in 0..16 {
                decoys16 = append_fake_directory(&decoys16, &doc_record);
            }
            let mut decoys64 = flood.clone();
            for _ in 0..64 {
                decoys64 = append_fake_directory(&decoys64, &doc_record);
            }
            for (tag, bytes) in [
                ("junk-then-fake", junk_then_fake),
                ("fake-then-junk", fake_then_junk),
                ("decoys-16", decoys16),
                ("decoys-64", decoys64),
            ] {
                let path = dir.join(format!("{tag}.{ext}"));
                std::fs::write(&path, &bytes).expect("write candidate copy");
                assert_eq!(
                    raw_entry_count(&path, MAX_ENTRIES),
                    Some(MAX_ENTRIES + 1),
                    "{ext} {tag}: the flood's own candidate is counted, \
                     however many candidates stand in front of it"
                );
                assert_eq!(
                    check_entry_cap(&path),
                    Err(TOO_MANY_PARTS.to_string()),
                    "{ext} {tag}: the cap itself refuses"
                );
                let err = load(&path).expect_err("the flood refuses at the bulletin door");
                assert_eq!(err, TOO_MANY_PARTS, "{ext} {tag} at the bulletin door");
                let err = load_insert(&path).expect_err("the flood refuses at the sheet door");
                assert_eq!(err, TOO_MANY_PARTS, "{ext} {tag} at the sheet door");
            }
            let _ = std::fs::remove_dir_all(&dir);
        }
    }

    /// Give a zip a comment carrying `fakes` fake
    /// end records: each is a full record whose
    /// comment length reaches exactly to the end of
    /// the file and whose directory is empty (zero
    /// entries at offset zero), so the reader's
    /// latest-first search takes the last fake as the
    /// file's end record and reads its empty
    /// directory (round 8, R8-12, the red team's
    /// RT8-9 shape).
    fn with_fake_end_records_in_comment(zip_bytes: &[u8], fakes: usize) -> Vec<u8> {
        let eocd = find_eocd(zip_bytes);
        let comment_len = fakes * 22;
        let file_len = zip_bytes.len() + comment_len;
        let mut out = zip_bytes.to_vec();
        out[eocd + 20..eocd + 22].copy_from_slice(&(comment_len as u16).to_le_bytes());
        for i in 0..fakes {
            let pos = zip_bytes.len() + i * 22;
            let mut rec = Vec::new();
            rec.extend_from_slice(b"PK\x05\x06");
            rec.extend_from_slice(&0u16.to_le_bytes());
            rec.extend_from_slice(&0u16.to_le_bytes());
            rec.extend_from_slice(&0u16.to_le_bytes());
            rec.extend_from_slice(&0u16.to_le_bytes());
            rec.extend_from_slice(&0u32.to_le_bytes());
            rec.extend_from_slice(&0u32.to_le_bytes());
            rec.extend_from_slice(&((file_len - pos - 22) as u16).to_le_bytes());
            assert_eq!(rec.len(), 22);
            out.extend_from_slice(&rec);
        }
        assert_eq!(out.len(), file_len);
        out
    }

    #[test]
    fn end_record_signatures_inside_the_zip_comment_do_not_hide_the_bulletin() {
        // Round 8 (R8-12): a well-formed 2-entry
        // bulletin whose zip comment contains twenty
        // end-record signatures with self-consistent
        // lengths failed to load — the reader took a
        // comment-internal fake as its end record,
        // read its empty directory, and reported no
        // bulletin inside. The loaders now try the
        // other enumerated candidates' directories
        // when the chosen one holds no document, so
        // both of the red team's fixtures load, at
        // both doors; and a file whose every candidate
        // yields no bulletin still refuses as
        // no-bulletin.
        let doc_json = serde_json::to_string(&test_doc()).expect("json");
        let sheet_json = lying_sheet_json();
        for (json_name, json, ext) in [
            ("document.json", doc_json.as_str(), "obd"),
            ("insert.json", sheet_json.as_str(), "obdi"),
        ] {
            let dir = scratch(&format!("comment-sigs-{ext}"));
            let honest = zip_with_fillers(json_name, json, 1);
            let commented = with_fake_end_records_in_comment(&honest, 20);
            let mut padded = commented.clone();
            padded.extend_from_slice(b"pad");
            for (tag, bytes) in [("comment", commented), ("comment-padded", padded)] {
                let path = dir.join(format!("{tag}.{ext}"));
                std::fs::write(&path, &bytes).expect("write commented copy");
                assert_eq!(
                    raw_entry_count(&path, MAX_ENTRIES),
                    Some(2),
                    "{ext} {tag}: the count walk defers on the fakes and \
                     counts the real directory's two records"
                );
                if ext == "obd" {
                    let loaded = load(&path).expect("the bulletin behind the comment loads");
                    assert_eq!(loaded.title, "Roundtrip", "{ext} {tag}");
                } else {
                    let (insert, _, _) =
                        load_insert(&path).expect("the sheet behind the comment loads");
                    assert_eq!(insert.id, "sheet-lying", "{ext} {tag}");
                }
            }
            let _ = std::fs::remove_dir_all(&dir);
        }
        // The f1a shape at honest size: a fake
        // directory carrying no document record, over
        // a real directory that holds no document
        // either. The chosen fake yields no bulletin
        // and no other candidate yields one, so the
        // refusal stands, in the no-bulletin words.
        let dir = scratch("comment-sigs-nodoc");
        let nodoc = zip_with_fillers("other.json", &doc_json, 1);
        let filler_record = central_record(&nodoc, 1);
        let bytes = append_fake_directory(&nodoc, &filler_record);
        let path = dir.join("nodoc.obd");
        std::fs::write(&path, &bytes).expect("write no-document copy");
        let err = load(&path).expect_err("no candidate yields a bulletin");
        assert_eq!(
            err,
            "that file has no bulletin inside it (no document.json)"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// One decoy flavor for the round-9 retry pins:
    /// a tag, the decoy directory's entries, and the
    /// entry whose central record lies about its
    /// size, if this flavor is the lying-size one.
    type DecoyFlavor<'a> = (&'a str, Vec<(&'a str, &'a [u8])>, Option<&'a str>);

    /// A small stored zip holding exactly `entries`,
    /// built by the writer so its checksums are
    /// honest; `entry_crcs` reads them back for the
    /// hand-appended decoy directories below.
    fn stored_zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let stored = || {
            zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored)
        };
        let mut buf = Vec::new();
        {
            let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
            for (name, payload) in entries {
                zip.start_file(*name, stored()).expect("decoy entry");
                zip.write_all(payload).expect("decoy bytes");
            }
            zip.finish().expect("finish");
        }
        buf
    }

    /// The (name, CRC) pairs of a zip built by
    /// `stored_zip`, read off its central records.
    fn entry_crcs(zip_bytes: &[u8]) -> Vec<(String, u32)> {
        let eocd = find_eocd(zip_bytes);
        let mut at = u32::from_le_bytes(
            zip_bytes[eocd + 16..eocd + 20]
                .try_into()
                .expect("offset field"),
        ) as usize;
        let mut out = Vec::new();
        while at < eocd {
            assert!(zip_bytes[at..].starts_with(b"PK\x01\x02"), "a record");
            let crc =
                u32::from_le_bytes(zip_bytes[at + 16..at + 20].try_into().expect("crc field"));
            let name_len = u16::from_le_bytes([zip_bytes[at + 28], zip_bytes[at + 29]]) as usize;
            let extra_len = u16::from_le_bytes([zip_bytes[at + 30], zip_bytes[at + 31]]) as usize;
            let comment_len = u16::from_le_bytes([zip_bytes[at + 32], zip_bytes[at + 33]]) as usize;
            let name = String::from_utf8(zip_bytes[at + 46..at + 46 + name_len].to_vec())
                .expect("entry name is text");
            out.push((name, crc));
            at += 46 + name_len + extra_len + comment_len;
        }
        out
    }

    /// Append a whole second directory to a zip:
    /// stored local headers for `entries` (each with
    /// the CRC the writer computed for it), their
    /// central records, and an end record naming
    /// them, all at absolute file offsets — the red
    /// team's decoy-directory shape (round 9,
    /// RT9-6). When `lie_size_for` names an entry,
    /// its CENTRAL record declares 5 bytes for the
    /// real payload while its local header stays
    /// honest: the lie the round-9 probes used.
    fn append_directory(
        base: &[u8],
        entries: &[(&str, &[u8], u32)],
        lie_size_for: Option<&str>,
    ) -> Vec<u8> {
        let mut out = base.to_vec();
        let mut centrals: Vec<Vec<u8>> = Vec::new();
        for (name, payload, crc) in entries {
            let local_offset = out.len() as u32;
            let mut local = Vec::new();
            local.extend_from_slice(b"PK\x03\x04");
            local.extend_from_slice(&20u16.to_le_bytes());
            local.extend_from_slice(&0u16.to_le_bytes());
            local.extend_from_slice(&0u16.to_le_bytes());
            local.extend_from_slice(&0u16.to_le_bytes());
            local.extend_from_slice(&0u16.to_le_bytes());
            local.extend_from_slice(&crc.to_le_bytes());
            local.extend_from_slice(&(payload.len() as u32).to_le_bytes());
            local.extend_from_slice(&(payload.len() as u32).to_le_bytes());
            local.extend_from_slice(&(name.len() as u16).to_le_bytes());
            local.extend_from_slice(&0u16.to_le_bytes());
            local.extend_from_slice(name.as_bytes());
            local.extend_from_slice(payload);
            out.extend_from_slice(&local);
            let declared = if lie_size_for == Some(name) {
                5u32
            } else {
                payload.len() as u32
            };
            let mut central = Vec::new();
            central.extend_from_slice(b"PK\x01\x02");
            central.extend_from_slice(&20u16.to_le_bytes());
            central.extend_from_slice(&20u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&crc.to_le_bytes());
            central.extend_from_slice(&declared.to_le_bytes());
            central.extend_from_slice(&declared.to_le_bytes());
            central.extend_from_slice(&(name.len() as u16).to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u32.to_le_bytes());
            central.extend_from_slice(&local_offset.to_le_bytes());
            central.extend_from_slice(name.as_bytes());
            centrals.push(central);
        }
        let cd_offset = out.len() as u32;
        let mut cd_size = 0u32;
        for central in &centrals {
            cd_size += central.len() as u32;
            out.extend_from_slice(central);
        }
        let mut eocd = Vec::new();
        eocd.extend_from_slice(b"PK\x05\x06");
        eocd.extend_from_slice(&0u16.to_le_bytes());
        eocd.extend_from_slice(&0u16.to_le_bytes());
        eocd.extend_from_slice(&(entries.len() as u16).to_le_bytes());
        eocd.extend_from_slice(&(entries.len() as u16).to_le_bytes());
        eocd.extend_from_slice(&cd_size.to_le_bytes());
        eocd.extend_from_slice(&cd_offset.to_le_bytes());
        eocd.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&eocd);
        out
    }

    /// Append a decoy directory carrying `entries`
    /// to `base`, checksums taken from a writer-built
    /// zip of the same entries.
    fn append_decoy_directory(
        base: &[u8],
        entries: &[(&str, &[u8])],
        lie_size_for: Option<&str>,
    ) -> Vec<u8> {
        let crcs = entry_crcs(&stored_zip(entries));
        let with_crc: Vec<(&str, &[u8], u32)> = entries
            .iter()
            .map(|(name, payload)| {
                let crc = crcs
                    .iter()
                    .find(|(n, _)| n == name)
                    .map(|(_, c)| *c)
                    .expect("the writer recorded a CRC");
                (*name, *payload, crc)
            })
            .collect();
        append_directory(base, &with_crc, lie_size_for)
    }

    /// Append an end record naming an EMPTY
    /// directory (its offset is its own position):
    /// the primary directory of the RT9-6 shapes, the
    /// one the reader chooses first.
    fn append_empty_end_record(base: &[u8]) -> Vec<u8> {
        let mut out = base.to_vec();
        let pos = out.len() as u32;
        let mut eocd = Vec::new();
        eocd.extend_from_slice(b"PK\x05\x06");
        eocd.extend_from_slice(&0u16.to_le_bytes());
        eocd.extend_from_slice(&0u16.to_le_bytes());
        eocd.extend_from_slice(&0u16.to_le_bytes());
        eocd.extend_from_slice(&0u16.to_le_bytes());
        eocd.extend_from_slice(&0u32.to_le_bytes());
        eocd.extend_from_slice(&pos.to_le_bytes());
        eocd.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&eocd);
        out
    }

    #[test]
    fn r9_a_failed_candidate_document_steps_aside_for_the_honest_directory() {
        // Round 9 (R9-6, review R9-R5 + red team
        // RT9-6): the retry used to step aside only
        // when the chosen directory held NO
        // document.json. A decoy directory whose
        // document.json points at junk — bad JSON, a
        // version from the future, a picture whose
        // bytes lie about their fingerprint, a
        // directory record lying about the document's
        // size — aborted the load, and the honest
        // directory behind it was never tried. Every
        // junk flavor now steps aside, in both
        // positions: the decoy as the reader's chosen
        // directory, and the decoy behind an appended
        // empty directory (the RT9-6 layout).
        let doc_json = serde_json::to_string(&test_doc()).expect("json");
        let mut version_value: serde_json::Value = serde_json::from_str(&doc_json).expect("value");
        version_value["format_version"] = serde_json::json!(999);
        let version_json = serde_json::to_string(&version_value).expect("json");
        let zero_hash = "0".repeat(64);
        let lying_asset_name = format!("assets/{zero_hash}");
        let flavors: Vec<DecoyFlavor> = vec![
            (
                "bad-json",
                vec![("document.json", b"not json at all")],
                None,
            ),
            (
                "version-999",
                vec![("document.json", version_json.as_bytes())],
                None,
            ),
            (
                "lying-asset",
                vec![
                    ("document.json", doc_json.as_bytes()),
                    (lying_asset_name.as_str(), b"not the hashed bytes"),
                ],
                None,
            ),
            (
                "lie-size",
                vec![("document.json", doc_json.as_bytes())],
                Some("document.json"),
            ),
        ];
        let dir = scratch("r9-retry-doc");
        let honest = zip_with_fillers("document.json", &doc_json, 1);
        for (tag, entries, lie) in &flavors {
            let as_primary = append_decoy_directory(&honest, entries, *lie);
            let behind_empty = append_empty_end_record(&as_primary);
            for (pos_tag, bytes) in [("primary", as_primary), ("behind-empty", behind_empty)] {
                let path = dir.join(format!("{tag}-{pos_tag}.obd"));
                std::fs::write(&path, &bytes).expect("write decoy copy");
                let loaded = load(&path)
                    .unwrap_or_else(|e| panic!("{tag} {pos_tag}: honest loads, got {e}"));
                assert_eq!(loaded.title, "Roundtrip", "{tag} {pos_tag}");
            }
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn r9_a_file_whose_every_candidate_is_junk_refuses_with_the_first_error() {
        // Round 9 (R9-6), the other half: when every
        // candidate fails, the load still refuses —
        // with the FIRST candidate's error, the refusal
        // the office would have gotten without the
        // junk behind it. Here the latest
        // document-holding candidate is a
        // version-from-the-future decoy standing in
        // front of a bad-JSON decoy: the version error
        // is the refusal, not the JSON error behind it
        // and not the no-bulletin words.
        let doc_json = serde_json::to_string(&test_doc()).expect("json");
        let mut version_value: serde_json::Value = serde_json::from_str(&doc_json).expect("value");
        version_value["format_version"] = serde_json::json!(999);
        let version_json = serde_json::to_string(&version_value).expect("json");
        let dir = scratch("r9-retry-junk");
        let nodoc = zip_with_fillers("other.json", &doc_json, 1);
        let bad: Vec<(&str, &[u8])> = vec![("document.json", b"not json at all")];
        let with_bad = append_decoy_directory(&nodoc, &bad, None);
        let versioned: Vec<(&str, &[u8])> = vec![("document.json", version_json.as_bytes())];
        let with_both = append_decoy_directory(&with_bad, &versioned, None);
        let bytes = append_empty_end_record(&with_both);
        let path = dir.join("all-junk.obd");
        std::fs::write(&path, &bytes).expect("write all-junk copy");
        let err = load(&path).expect_err("every candidate junk refuses");
        assert!(
            err.contains("document format 999 is newer"),
            "the first candidate's error stands: {err}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn r9_a_failed_candidate_sheet_steps_aside_for_the_honest_directory() {
        // Round 9 (R9-6) at the sheet door, the
        // flavors that translate: bad JSON, a lying
        // fingerprint, a lying size. (A sheet package
        // carries no format version of its own, so
        // the version flavor has no shape here.)
        let sheet_json = lying_sheet_json();
        let zero_hash = "0".repeat(64);
        let lying_asset_name = format!("assets/{zero_hash}");
        let flavors: Vec<DecoyFlavor> = vec![
            ("bad-json", vec![("insert.json", b"not json at all")], None),
            (
                "lying-asset",
                vec![
                    ("insert.json", sheet_json.as_bytes()),
                    (lying_asset_name.as_str(), b"not the hashed bytes"),
                ],
                None,
            ),
            (
                "lie-size",
                vec![("insert.json", sheet_json.as_bytes())],
                Some("insert.json"),
            ),
        ];
        let dir = scratch("r9-retry-sheet");
        let honest = zip_with_fillers("insert.json", &sheet_json, 1);
        for (tag, entries, lie) in &flavors {
            let as_primary = append_decoy_directory(&honest, entries, *lie);
            let behind_empty = append_empty_end_record(&as_primary);
            for (pos_tag, bytes) in [("primary", as_primary), ("behind-empty", behind_empty)] {
                let path = dir.join(format!("{tag}-{pos_tag}.obdi"));
                std::fs::write(&path, &bytes).expect("write decoy copy");
                let (insert, _, _) = load_insert(&path)
                    .unwrap_or_else(|e| panic!("{tag} {pos_tag}: honest loads, got {e}"));
                assert_eq!(insert.id, "sheet-lying", "{tag} {pos_tag}");
            }
        }
        // And a sheet file whose every candidate is
        // junk refuses with the first candidate's
        // error, not the no-sheet words.
        let nodoc = zip_with_fillers("other.json", &sheet_json, 1);
        let bad: Vec<(&str, &[u8])> = vec![("insert.json", b"not json at all")];
        let with_bad = append_decoy_directory(&nodoc, &bad, None);
        let bytes = append_empty_end_record(&with_bad);
        let path = dir.join("all-junk.obdi");
        std::fs::write(&path, &bytes).expect("write all-junk copy");
        let err = load_insert(&path).expect_err("every candidate junk refuses");
        assert!(err.contains("ident"), "the first error stands: {err}");
        let _ = std::fs::remove_dir_all(&dir);
    }
    /// Give a zip a comment carrying `fakes` fake end
    /// records in the red team's timing shape (round
    /// 9, RT9-2): each record claims two entries at
    /// offset zero and a zero-length comment of its
    /// own, so the file's comment is a solid run of
    /// end-record signatures.
    fn with_timing_end_records_in_comment(zip_bytes: &[u8], fakes: usize) -> Vec<u8> {
        let eocd = find_eocd(zip_bytes);
        let comment_len = fakes * 22;
        let mut out = zip_bytes.to_vec();
        out[eocd + 20..eocd + 22].copy_from_slice(&(comment_len as u16).to_le_bytes());
        for _ in 0..fakes {
            let mut rec = Vec::new();
            rec.extend_from_slice(b"PK\x05\x06");
            rec.extend_from_slice(&0u16.to_le_bytes());
            rec.extend_from_slice(&0u16.to_le_bytes());
            rec.extend_from_slice(&2u16.to_le_bytes());
            rec.extend_from_slice(&2u16.to_le_bytes());
            rec.extend_from_slice(&0u32.to_le_bytes());
            rec.extend_from_slice(&0u32.to_le_bytes());
            rec.extend_from_slice(&0u16.to_le_bytes());
            assert_eq!(rec.len(), 22);
            out.extend_from_slice(&rec);
        }
        out
    }

    #[test]
    fn r9_thousands_of_comment_signatures_count_and_load_the_honest_bulletin() {
        // Round 9 (R9-2): 2,900 end-record signatures
        // in a zip comment — the most a legal comment
        // carries, the red team's largest fixture —
        // in BOTH fake shapes (the round-8 records,
        // whose comment lengths reach the end of the
        // file, and the timing records, claiming two
        // entries at offset zero). The count walk
        // counts the real directory's two records and
        // the bulletin loads. No wall-clock is
        // asserted here — the measured times live in
        // the round-9 synthesis addendum (R9-D); this
        // pin holds the behavior at the fixture's
        // scale.
        let doc_json = serde_json::to_string(&test_doc()).expect("json");
        let dir = scratch("r9-comment-sigs");
        let honest = zip_with_fillers("document.json", &doc_json, 1);
        for (tag, bytes) in [
            ("carried", with_fake_end_records_in_comment(&honest, 2900)),
            ("timing", with_timing_end_records_in_comment(&honest, 2900)),
        ] {
            let path = dir.join(format!("{tag}.obd"));
            std::fs::write(&path, &bytes).expect("write signature copy");
            assert_eq!(
                raw_entry_count(&path, MAX_ENTRIES),
                Some(2),
                "{tag}: the real directory's two records are the count"
            );
            let loaded = load(&path).expect("the honest bulletin loads");
            assert_eq!(loaded.title, "Roundtrip", "{tag}");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Rename one entry everywhere its name is
    /// written (the local header and the central
    /// directory), the two names the same length: the
    /// writer refuses a duplicate name outright, so a
    /// file that carries a repeated entry is assembled
    /// by patch, like the lying-size files above.
    fn patch_entry_name(zip_bytes: &mut [u8], from_name: &str, to_name: &str) {
        assert_eq!(from_name.len(), to_name.len(), "the patch keeps lengths");
        let name = from_name.as_bytes();
        let mut patched = 0;
        let mut from = 0;
        while let Some(found) = find_bytes(&zip_bytes[from..], name) {
            let at = from + found;
            let guarded = (at >= 30 && &zip_bytes[at - 30..at - 26] == b"PK\x03\x04")
                || (at >= 46 && &zip_bytes[at - 46..at - 42] == b"PK\x01\x02");
            if guarded {
                zip_bytes[at..at + name.len()].copy_from_slice(to_name.as_bytes());
                patched += 1;
            }
            from = at + 1;
        }
        assert_eq!(patched, 2, "both records for {from_name} were patched");
    }

    #[test]
    fn many_assets_load_with_one_record_each_even_when_entries_repeat() {
        // Round 4, RT-2's other half: the dedupe
        // question is asked of a set now, not of the
        // growing record list. This hand-built file
        // carries 300 pictures and NO records for them
        // in its document.json, and every picture's
        // entry rides twice (the repeats patched under
        // their real names, per the helper above): the
        // loader writes the record for the first entry
        // it meets, the repeat adds nothing, and the
        // bytes land once.
        let mut doc = test_doc();
        doc.assets.clear();
        let mut buf = Vec::new();
        let mut renames: Vec<(String, String)> = Vec::new();
        {
            let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            zip.start_file("document.json", options)
                .expect("json entry");
            zip.write_all(serde_json::to_string(&doc).expect("json").as_bytes())
                .expect("json bytes");
            for i in 0..300 {
                let bytes = format!("picture number {i}").into_bytes();
                let hash = ob_model::sha256_hex(&bytes);
                let name = format!("assets/{hash}");
                zip.start_file(&name, options).expect("asset entry");
                zip.write_all(&bytes).expect("asset bytes");
                let stand_in = format!("zzzz{i:067}");
                assert_eq!(stand_in.len(), name.len());
                zip.start_file(&stand_in, options).expect("repeat entry");
                zip.write_all(&bytes).expect("repeat bytes");
                renames.push((stand_in, name));
            }
            zip.finish().expect("finish");
        }
        for (from_name, to_name) in &renames {
            patch_entry_name(&mut buf, from_name, to_name);
        }
        let dir = scratch("many-assets");
        let path = dir.join("many.obd");
        std::fs::write(&path, &buf).expect("write");
        let loaded = load(&path).expect("loads");
        assert_eq!(loaded.assets.len(), 300, "one record per picture");
        assert_eq!(
            loaded.asset_bytes.len(),
            300,
            "one set of bytes per picture"
        );
        let bytes = "picture number 7".as_bytes().to_vec();
        let hash = ob_model::sha256_hex(&bytes);
        assert_eq!(loaded.asset_bytes.get(&hash), Some(&bytes));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// One central directory record for the shared-
    /// directory builder below, in the shape
    /// `append_directory` writes.
    fn shared_central_record(name: &str, crc: u32, size: u32, local_offset: u32) -> Vec<u8> {
        let mut central = Vec::new();
        central.extend_from_slice(b"PK\x01\x02");
        central.extend_from_slice(&20u16.to_le_bytes());
        central.extend_from_slice(&20u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&crc.to_le_bytes());
        central.extend_from_slice(&size.to_le_bytes());
        central.extend_from_slice(&size.to_le_bytes());
        central.extend_from_slice(&(name.len() as u16).to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u32.to_le_bytes());
        central.extend_from_slice(&local_offset.to_le_bytes());
        central.extend_from_slice(name.as_bytes());
        central
    }

    /// The round-10 red team's shared-directory
    /// shape (RT10-T1): one local header, one central
    /// directory of `entries` records just under the
    /// cap, one shared zip64 end record, then
    /// `candidates` appended candidates, each a
    /// locator pointing at the SAME zip64 record
    /// plus a sentinel end record — every candidate
    /// walkable, all resolving to the same directory.
    /// With `doc_payload`, the directory's first
    /// record is a document.json carrying those bytes
    /// (its central record declares their CRC and
    /// size; the only local header in the file is
    /// another entry's, so reading it fails).
    fn shared_directory_zip(
        candidates: usize,
        entries: usize,
        doc_payload: Option<&[u8]>,
    ) -> Vec<u8> {
        let crc_x = entry_crcs(&stored_zip(&[("a", b"x")]))[0].1;
        let mut out = Vec::new();
        // One local header: entry "a", payload "x".
        out.extend_from_slice(b"PK\x03\x04");
        out.extend_from_slice(&20u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&crc_x.to_le_bytes());
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(b"a");
        out.extend_from_slice(b"x");
        let cd_offset = out.len() as u64;
        let mut names: Vec<String> = Vec::new();
        if doc_payload.is_some() {
            names.push("document.json".to_string());
        }
        for i in 0..entries - names.len() {
            names.push(format!("f{i:04}"));
        }
        let doc_crc =
            doc_payload.map(|payload| entry_crcs(&stored_zip(&[("document.json", payload)]))[0].1);
        let mut cd_size = 0u64;
        for name in &names {
            let (crc, size) = if name == "document.json" {
                let payload = doc_payload.expect("the document payload");
                (doc_crc.expect("the document crc"), payload.len() as u32)
            } else {
                (crc_x, 1u32)
            };
            let record = shared_central_record(name, crc, size, 0);
            cd_size += record.len() as u64;
            out.extend_from_slice(&record);
        }
        // The shared zip64 end record.
        let z64_at = out.len() as u64;
        let mut record = Vec::new();
        record.extend_from_slice(b"PK\x06\x06");
        record.extend_from_slice(&44u64.to_le_bytes());
        record.extend_from_slice(&45u16.to_le_bytes());
        record.extend_from_slice(&45u16.to_le_bytes());
        record.extend_from_slice(&0u32.to_le_bytes());
        record.extend_from_slice(&0u32.to_le_bytes());
        record.extend_from_slice(&(entries as u64).to_le_bytes());
        record.extend_from_slice(&(entries as u64).to_le_bytes());
        record.extend_from_slice(&cd_size.to_le_bytes());
        record.extend_from_slice(&cd_offset.to_le_bytes());
        assert_eq!(record.len(), 56);
        out.extend_from_slice(&record);
        // The appended candidates.
        for _ in 0..candidates {
            let mut locator = Vec::new();
            locator.extend_from_slice(b"PK\x06\x07");
            locator.extend_from_slice(&0u32.to_le_bytes());
            locator.extend_from_slice(&z64_at.to_le_bytes());
            locator.extend_from_slice(&1u32.to_le_bytes());
            assert_eq!(locator.len(), 20);
            out.extend_from_slice(&locator);
            let mut eocd = Vec::new();
            eocd.extend_from_slice(b"PK\x05\x06");
            eocd.extend_from_slice(&0u16.to_le_bytes());
            eocd.extend_from_slice(&0u16.to_le_bytes());
            eocd.extend_from_slice(&u16::MAX.to_le_bytes());
            eocd.extend_from_slice(&u16::MAX.to_le_bytes());
            eocd.extend_from_slice(&u32::MAX.to_le_bytes());
            eocd.extend_from_slice(&u32::MAX.to_le_bytes());
            eocd.extend_from_slice(&0u16.to_le_bytes());
            assert_eq!(eocd.len(), 22);
            out.extend_from_slice(&eocd);
        }
        out
    }

    /// The control beside the shared shape: the same
    /// 8,191-entry directory (no document record)
    /// under ONE plain end record.
    fn shared_directory_control(entries: usize) -> Vec<u8> {
        let full = shared_directory_zip(1, entries, None);
        // The plain control is the shared file's head
        // — local header plus directory — with the
        // zip64 structures replaced by one plain end
        // record naming the same directory.
        let z64_at = full.len() - 56 - 20 - 22;
        let cd_offset = u64::from_le_bytes(
            full[z64_at + 48..z64_at + 56]
                .try_into()
                .expect("offset field"),
        );
        let cd_size = (z64_at as u64 - cd_offset) as u32;
        let mut out = full[..z64_at].to_vec();
        let mut eocd = Vec::new();
        eocd.extend_from_slice(b"PK\x05\x06");
        eocd.extend_from_slice(&0u16.to_le_bytes());
        eocd.extend_from_slice(&0u16.to_le_bytes());
        eocd.extend_from_slice(&(entries as u16).to_le_bytes());
        eocd.extend_from_slice(&(entries as u16).to_le_bytes());
        eocd.extend_from_slice(&cd_size.to_le_bytes());
        eocd.extend_from_slice(&(cd_offset as u32).to_le_bytes());
        eocd.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&eocd);
        out
    }

    /// The shared-directory timing budget (round 12,
    /// R12-8): the wider of a control multiple and
    /// the control plus a fixed floor scaled to the
    /// candidate count — 15 ms per candidate. The
    /// old budget was the multiple alone (plus a
    /// flat ten seconds), derived entirely from a
    /// control measured under the same contention it
    /// was meant to absorb, and two round-12 lanes
    /// watched it fail under machine load 8.4–13
    /// with every outcome passing (the 2,900 phase
    /// at 23–30 s against a ~20–27 s budget). The
    /// floor puts the 2,900 budget past 43 s, clear
    /// of that contention, while a true per-candidate
    /// multiplication — ≈39 ms of walk per candidate
    /// for this directory, 100 s and more at these
    /// counts — still fails it.
    fn shared_directory_budget(
        control: std::time::Duration,
        candidates: usize,
    ) -> std::time::Duration {
        let floor = std::time::Duration::from_millis(15) * (candidates as u32);
        std::cmp::max(control * 20, control + floor)
    }

    #[test]
    fn r10_candidates_sharing_one_directory_pay_for_it_once() {
        // Round 10 (R10-1, red team RT10-T1): many
        // walkable candidates sharing ONE directory
        // just under the entry cap multiplied the
        // whole walk — every candidate paid the full
        // 8,191-record walk in the cap check, the
        // same walk again in the loader's own
        // enumeration, and one trimmed archive open:
        // ≈0.33 s per candidate on the red team's
        // machine (30 candidates 6.0 s, 300 in
        // 100.5 s, 2,900 unfinished in 100 s). A
        // directory is walked and opened at most once
        // per load now, however many candidates point
        // at it: the enumeration returns ONE candidate
        // for the shared directory, and the loads land
        // in the one-candidate control's time class.
        let dir = scratch("r10-shared-dir");
        let entries = MAX_ENTRIES - 1;
        let control_path = dir.join("control.obd");
        std::fs::write(&control_path, shared_directory_control(entries)).expect("write control");
        let control_start = std::time::Instant::now();
        let control_err = load(&control_path).expect_err("the control holds no bulletin");
        let control_elapsed = control_start.elapsed();
        assert_eq!(
            control_err, "that file has no bulletin inside it (no document.json)",
            "the control's refusal"
        );
        for (tag, candidates) in [("30", 30usize), ("300", 300), ("2900", 2900)] {
            let path = dir.join(format!("shared-{tag}.obd"));
            std::fs::write(&path, shared_directory_zip(candidates, entries, None))
                .expect("write shared copy");
            // The structural pin: one distinct
            // directory, however many candidates.
            assert_eq!(
                directory_candidates(&path, MAX_ENTRIES).len(),
                1,
                "{tag} candidates: the shared directory is enumerated once"
            );
            let start = std::time::Instant::now();
            let err = load(&path).expect_err("the shared directory holds no bulletin");
            let elapsed = start.elapsed();
            assert_eq!(
                err, "that file has no bulletin inside it (no document.json)",
                "{tag} candidates: the refusal is the control's"
            );
            // The timing assertion is a HANG GUARD
            // now (FLAGGED, round 13 R13-8, by
            // disposition): a 300 s cap whose only
            // job is catching a hung or runaway load.
            // Before the round-10 fix this shape cost
            // ≈0.33 s PER CANDIDATE — the 300-candidate
            // file took 100.5 s on the red team's
            // machine and the 2,900 one did not finish
            // in 100 s — so a true multiplication is
            // still caught, at these counts, well
            // under the cap on an uncontended machine;
            // what the cap no longer tries to do is
            // grade the load's time class. The
            // contention measurements across rounds
            // 12–13 are why: this pin's phases measured
            // 23–90 s at machine loads 8–13.5 with
            // every outcome assertion passing, and
            // BOTH previous budget formulas failed
            // under contention alone — first the
            // control ×20 multiple, then the round-12
            // formula (control + 15 ms × candidates).
            // The outcome assertions above and the
            // one-candidate structural assertion are
            // unchanged and primary; they, plus the
            // r11/r12b timing pins at smaller counts
            // (which keep `shared_directory_budget`),
            // guard per-candidate multiplication.
            let budget = std::time::Duration::from_secs(300);
            assert!(
                elapsed < budget,
                "{tag} candidates: {elapsed:?} must finish within the hang \
                 guard {budget:?} (control {control_elapsed:?})"
            );
            if candidates == 300 {
                // The sheet door shares the enumeration.
                let err = load_insert(&path).expect_err("the shared directory holds no sheet");
                assert_eq!(
                    err, "that file has no loose sheet inside it (no insert.json)",
                    "300 candidates at the sheet door"
                );
            }
        }
        // The bad-document variant at scale: the
        // shared directory's document fails to read,
        // and the refusal is that first candidate's
        // error — surfaced after the fallback has also
        // been tried (round 10, R10-3), not multiplied
        // by the candidate count.
        let bad_path = dir.join("shared-baddoc-300.obd");
        std::fs::write(
            &bad_path,
            shared_directory_zip(300, entries, Some(b"{not json")),
        )
        .expect("write bad-document copy");
        let start = std::time::Instant::now();
        let err = load(&bad_path).expect_err("the shared document fails");
        let elapsed = start.elapsed();
        assert!(
            err.contains("checksum"),
            "the first candidate's error stands: {err}"
        );
        // The same hang guard as the loop above
        // (round 13, R13-8): 300 s, catching a hung
        // or runaway load only.
        let budget = std::time::Duration::from_secs(300);
        assert!(
            elapsed < budget,
            "bad-document 300: {elapsed:?} within the hang guard {budget:?}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn r10_the_fallback_does_not_storm_over_comment_signatures() {
        // Round 10 (R10-3's cost half): the fallback
        // runs whenever no candidate yields, so it
        // now runs at the sheet door over a bulletin
        // file — and over a file whose candidates all
        // fail. A comment carrying 2,900 fake end
        // records must not make those loads storm:
        // handing the archive reader a stream whose
        // chosen candidate cannot walk makes the
        // reader retry every fake in turn (the round-9
        // signature fixtures took 7.5 s / 35 s / 66 s
        // at the sheet door for 500 / 1,500 / 2,900
        // fakes in this build's first cut, before the
        // walk gate). The gate refuses to open a
        // candidate whose directory does not walk, so
        // both loads refuse in the plain no-inside
        // words, fast.
        let doc_json = serde_json::to_string(&test_doc()).expect("json");
        let dir = scratch("r10-fallback-storm");
        let honest = zip_with_fillers("document.json", &doc_json, 1);
        let faked = with_timing_end_records_in_comment(&honest, 2900);
        let sheet_path = dir.join("bulletin-at-the-sheet-door.obd");
        std::fs::write(&sheet_path, &faked).expect("write faked bulletin");
        let start = std::time::Instant::now();
        let err = load_insert(&sheet_path).expect_err("a bulletin holds no sheet");
        let elapsed = start.elapsed();
        assert_eq!(
            err,
            "that file has no loose sheet inside it (no insert.json)"
        );
        assert!(
            elapsed < std::time::Duration::from_secs(20),
            "the sheet door over 2,900 comment fakes: {elapsed:?}"
        );
        let nodoc = zip_with_fillers("other.json", &doc_json, 1);
        let faked_nodoc = with_timing_end_records_in_comment(&nodoc, 2900);
        let nodoc_path = dir.join("nodoc-faked.obd");
        std::fs::write(&nodoc_path, &faked_nodoc).expect("write faked nodoc");
        let start = std::time::Instant::now();
        let err = load(&nodoc_path).expect_err("no candidate yields a bulletin");
        let elapsed = start.elapsed();
        assert_eq!(
            err,
            "that file has no bulletin inside it (no document.json)"
        );
        assert!(
            elapsed < std::time::Duration::from_secs(20),
            "the bulletin door over 2,900 comment fakes: {elapsed:?}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn r10_a_walkable_fake_candidate_does_not_shadow_the_fallback() {
        // Round 10 (R10-3, red team RT10-L1): an
        // honest zip plus 70,000 bytes of signature-
        // free trailing junk (the honest end record
        // pushed out of the tail window) plus one
        // appended walkable empty candidate refused
        // with "no bulletin inside it" — the fake
        // OPENED, and opening closed the reader-side
        // fallback gate on the honest directory.
        // Opening is not evidence: the fallback runs
        // whenever no candidate YIELDS a bulletin, so
        // both the empty fake and the bad-document
        // fake now step aside for the honest bulletin
        // behind the junk, at both doors.
        let doc_json = serde_json::to_string(&test_doc()).expect("json");
        let sheet_json = lying_sheet_json();
        let junk: Vec<u8> = b"JUNKJUNK".repeat(8_750);
        assert_eq!(junk.len(), 70_000);
        let dir = scratch("r10-fake-shadow");
        for (json_name, json, ext, honest_title) in [
            ("document.json", doc_json.as_str(), "obd", "Roundtrip"),
            ("insert.json", sheet_json.as_str(), "obdi", "sheet-lying"),
        ] {
            let honest = zip_with_fillers(json_name, json, 1);
            let mut junked = honest.clone();
            junked.extend_from_slice(&junk);
            // The control: the same junk, no fake —
            // the fallback finds the honest directory.
            let control_path = dir.join(format!("control.{ext}"));
            std::fs::write(&control_path, &junked).expect("write control");
            // The empty fake standing in front of it.
            let shadowed_path = dir.join(format!("shadowed.{ext}"));
            std::fs::write(&shadowed_path, append_empty_end_record(&junked))
                .expect("write shadowed copy");
            // The bad-document fake in front of it.
            let bad_name = json_name;
            let bad_payload: &[u8] = b"{not json";
            let bad_entries: Vec<(&str, &[u8])> = vec![(bad_name, bad_payload)];
            let bad_path = dir.join(format!("bad-fake.{ext}"));
            std::fs::write(
                &bad_path,
                append_decoy_directory(&junked, &bad_entries, None),
            )
            .expect("write bad-fake copy");
            for (tag, path) in [
                ("control", &control_path),
                ("shadowed", &shadowed_path),
                ("bad-fake", &bad_path),
            ] {
                if ext == "obd" {
                    let loaded =
                        load(path).unwrap_or_else(|e| panic!("{ext} {tag}: honest loads, got {e}"));
                    assert_eq!(loaded.title, honest_title, "{ext} {tag}");
                } else {
                    let (insert, _, _) = load_insert(path)
                        .unwrap_or_else(|e| panic!("{ext} {tag}: honest loads, got {e}"));
                    assert_eq!(insert.id, honest_title, "{ext} {tag}");
                }
            }
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The shared-directory shape with ONE free
    /// declared field varied per appended candidate
    /// (round 11, RT11-1): every candidate still
    /// points at the same zip64 record and the same
    /// directory; only its end record's `total16`
    /// (vary_size = false) or `cd_size32` (true) is
    /// set to the candidate's index. The appended
    /// candidates are the file's last 42-byte blocks
    /// (a 20-byte locator plus the 22-byte record).
    fn shared_directory_zip_declared_variants(
        candidates: usize,
        entries: usize,
        vary_size: bool,
    ) -> Vec<u8> {
        let mut out = shared_directory_zip(candidates, entries, None);
        let block = 20 + 22;
        for i in 0..candidates {
            let eocd = out.len() - (candidates - i) * block + 20;
            if vary_size {
                out[eocd + 12..eocd + 16].copy_from_slice(&(i as u32).to_le_bytes());
            } else {
                out[eocd + 10..eocd + 12].copy_from_slice(&(i as u16).to_le_bytes());
            }
        }
        out
    }

    #[test]
    fn r11_the_fallback_dedupes_by_the_resolved_directory_not_the_declared_fields() {
        // Round 11 (R11-2, red team RT11-1): the
        // enumeration deduped by the resolved
        // directory, but the FALLBACK deduped by the
        // candidate's DECLARED fields — and two of
        // those fields are free variables the
        // resolution never consults. Varying one per
        // candidate (total16, or cd_size32) made
        // every fallback key distinct, and the shared
        // 8,191-entry directory was reopened ~300
        // times: 14,517 ms / 13,821 ms against a
        // 318 ms control on the red team's machine,
        // ~45x and linear. The fallback dedupes by
        // the resolved directory now — the
        // enumeration's own identity — so candidates
        // whose declared fields differ but which
        // resolve to the same directory open it once,
        // and both doors land in the control's time
        // class with the control's refusal.
        let dir = scratch("r11-fallback-dedupe");
        let entries = MAX_ENTRIES - 1;
        let control_path = dir.join("control.obd");
        std::fs::write(&control_path, shared_directory_control(entries)).expect("write control");
        let control_start = std::time::Instant::now();
        let control_err = load(&control_path).expect_err("the control holds no bulletin");
        let control_elapsed = control_start.elapsed();
        assert_eq!(
            control_err, "that file has no bulletin inside it (no document.json)",
            "the control's refusal"
        );
        // The timing budget, in the r10 pin's style:
        // margins wide enough for a contended
        // machine; the pre-fix shape cost ~45x the
        // control at this candidate count.
        let budget = control_elapsed * 20 + std::time::Duration::from_secs(10);
        for (tag, vary_size) in [("total16", false), ("cd-size32", true)] {
            let bytes = shared_directory_zip_declared_variants(300, entries, vary_size);
            let path = dir.join(format!("varied-{tag}.obd"));
            std::fs::write(&path, &bytes).expect("write varied copy");
            assert_eq!(
                directory_candidates(&path, MAX_ENTRIES).len(),
                1,
                "{tag}: the enumeration dedupes the varied candidates to one"
            );
            let start = std::time::Instant::now();
            let err = load(&path).expect_err("the shared directory holds no bulletin");
            let elapsed = start.elapsed();
            assert_eq!(
                err, "that file has no bulletin inside it (no document.json)",
                "{tag}: the refusal is the control's"
            );
            assert!(
                elapsed < budget,
                "{tag} at the bulletin door: {elapsed:?} must land in the \
                 control's time class (control {control_elapsed:?}, budget {budget:?})"
            );
            let start = std::time::Instant::now();
            let err = load_insert(&path).expect_err("the shared directory holds no sheet");
            let elapsed = start.elapsed();
            assert_eq!(
                err, "that file has no loose sheet inside it (no insert.json)",
                "{tag}: the sheet door's refusal"
            );
            assert!(
                elapsed < budget,
                "{tag} at the sheet door: {elapsed:?} must land in the \
                 control's time class (control {control_elapsed:?}, budget {budget:?})"
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A signature-free stub entry standing at the
    /// head of a file (round 11, RT11-2): one local
    /// header naming a stored payload of `S` bytes,
    /// sized so header plus name plus payload is
    /// exactly `total` bytes. An honest zip appended
    /// behind it keeps its base-0 declared offsets,
    /// so its directory stands some `total` bytes
    /// past where its end record declares it.
    fn stub_prefix(total: usize) -> Vec<u8> {
        let payload_len = total - 30 - 1;
        let mut out = Vec::new();
        out.extend_from_slice(b"PK\x03\x04");
        out.extend_from_slice(&20u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&(payload_len as u32).to_le_bytes());
        out.extend_from_slice(&(payload_len as u32).to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(b"s");
        out.extend(vec![b'S'; payload_len]);
        assert_eq!(out.len(), total);
        out
    }

    /// The round-11 dead-zone shape (RT11-2): an
    /// honest zip behind a 70,000-byte stub entry
    /// (or not, when `stub` is false), then 70,000
    /// bytes of signature-free trailing junk (or
    /// none), then `fakes` appended empty end
    /// records. With the stub standing, the honest
    /// directory stands past the fallback
    /// resolution's old 65,633-byte search bound;
    /// with the junk standing, its end record stands
    /// outside the enumeration's tail window.
    fn dead_zone_file(honest: &[u8], stub: bool, junk: bool, fakes: usize) -> Vec<u8> {
        let mut out = Vec::new();
        if stub {
            out.extend_from_slice(&stub_prefix(70_000));
        }
        out.extend_from_slice(honest);
        if junk {
            out.extend(vec![b'J'; 70_000]);
        }
        for _ in 0..fakes {
            out = append_empty_end_record(&out);
        }
        out
    }

    #[test]
    fn r11_a_shifted_honest_directory_loads_behind_walkable_fakes() {
        // Round 11 (R11-3, red team RT11-2): three
        // bounds composed into a dead zone — the
        // enumeration covers only the tail window,
        // the fallback's resolution searched at most
        // 65,633 bytes past a declared offset, and
        // the reader attempt was gated off once any
        // walkable fake candidate stood latest and
        // opened. An honest bulletin whose directory
        // a head stub shifted past the search bound
        // was REFUSED with 65 fakes, with just 3
        // opening fakes, and behind 1 junk-document
        // fake (whose JSON error became the refusal).
        // The fallback's resolution is the
        // enumeration's now, and a candidate that
        // opens but yields no bulletin never closes
        // the reader attempt: every row of the lane's
        // construction table loads the honest
        // bulletin, at both doors.
        let doc_json = serde_json::to_string(&test_doc()).expect("json");
        let sheet_json = lying_sheet_json();
        let dir = scratch("r11-dead-zone");
        // The walk gate's direct evidence first: on
        // the gapped file the honest candidate
        // resolves walkable, exactly as on the
        // ungapped one. (The honest end record is
        // the honest zip's last 22 bytes; the stub
        // stands in front of the zip.)
        let honest = zip_with_fillers("document.json", &doc_json, 1);
        let gapped = dead_zone_file(&honest, true, false, 0);
        assert!(
            fallback_walkable(&gapped, gapped.len() - 22),
            "the shifted directory is walkable to the fallback"
        );
        assert!(
            fallback_walkable(&honest, honest.len() - 22),
            "and so is the same directory unshifted"
        );
        for (json_name, json, ext, honest_title) in [
            ("document.json", doc_json.as_str(), "obd", "Roundtrip"),
            ("insert.json", sheet_json.as_str(), "obdi", "sheet-lying"),
        ] {
            let honest = zip_with_fillers(json_name, json, 1);
            let rows: Vec<(&str, Vec<u8>)> = vec![
                ("alone", honest.clone()),
                ("stub-in-window", dead_zone_file(&honest, true, false, 0)),
                (
                    "no-gap-junk-65-fakes",
                    dead_zone_file(&honest, false, true, 65),
                ),
                ("gap-junk-no-fakes", dead_zone_file(&honest, true, true, 0)),
                ("gap-junk-65-fakes", dead_zone_file(&honest, true, true, 65)),
                ("gap-junk-3-fakes", dead_zone_file(&honest, true, true, 3)),
            ];
            for (tag, bytes) in rows {
                let path = dir.join(format!("{tag}.{ext}"));
                std::fs::write(&path, &bytes).expect("write dead-zone copy");
                if ext == "obd" {
                    let loaded = load(&path)
                        .unwrap_or_else(|e| panic!("{ext} {tag}: honest loads, got {e}"));
                    assert_eq!(loaded.title, honest_title, "{ext} {tag}");
                } else {
                    let (insert, _, _) = load_insert(&path)
                        .unwrap_or_else(|e| panic!("{ext} {tag}: honest loads, got {e}"));
                    assert_eq!(insert.id, honest_title, "{ext} {tag}");
                }
            }
            // The junk-document fake rows: a walkable
            // fake whose document is junk stands
            // latest, and the honest bulletin behind
            // the gap still loads — the fake's JSON
            // error is not the answer.
            let bad_entries: Vec<(&str, &[u8])> = vec![(json_name, b"{not json")];
            for (tag, base) in [
                (
                    "gap-junk-junkdoc-fake",
                    dead_zone_file(&honest, true, true, 0),
                ),
                (
                    "no-gap-junk-junkdoc-fake",
                    dead_zone_file(&honest, false, true, 0),
                ),
            ] {
                let bytes = append_decoy_directory(&base, &bad_entries, None);
                let path = dir.join(format!("{tag}.{ext}"));
                std::fs::write(&path, &bytes).expect("write junk-document copy");
                if ext == "obd" {
                    let loaded = load(&path)
                        .unwrap_or_else(|e| panic!("{ext} {tag}: honest loads, got {e}"));
                    assert_eq!(loaded.title, honest_title, "{ext} {tag}");
                } else {
                    let (insert, _, _) = load_insert(&path)
                        .unwrap_or_else(|e| panic!("{ext} {tag}: honest loads, got {e}"));
                    assert_eq!(insert.id, honest_title, "{ext} {tag}");
                }
            }
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn r11_a_file_with_no_bulletin_anywhere_refuses_with_the_first_error() {
        // Round 11 (R11-3), the refusal half: closing
        // the dead zone does not soften refusals. In
        // the dead-zone shapes with no bulletin
        // anywhere, the load still refuses with the
        // most informative error — the first
        // candidate's document error where one
        // exists, else the plain no-bulletin
        // sentence.
        let dir = scratch("r11-dead-zone-refusals");
        // A junk-document fake alone behind the stub
        // and the junk: its JSON error is the
        // refusal.
        let bad_entries: Vec<(&str, &[u8])> = vec![("document.json", b"{not json")];
        let mut bytes = stub_prefix(70_000);
        bytes.extend(vec![b'J'; 70_000]);
        let bytes = append_decoy_directory(&bytes, &bad_entries, None);
        let path = dir.join("junkdoc-only.obd");
        std::fs::write(&path, &bytes).expect("write junk-document-only copy");
        let err = load(&path).expect_err("no bulletin anywhere refuses");
        assert!(
            err.contains("key must be a string"),
            "the first candidate's document error stands: {err}"
        );
        // Empty fakes alone behind the stub and the
        // junk: no candidate yields a document error,
        // so the refusal is the no-bulletin sentence.
        let mut bytes = stub_prefix(70_000);
        bytes.extend(vec![b'J'; 70_000]);
        for _ in 0..3 {
            bytes = append_empty_end_record(&bytes);
        }
        let path = dir.join("fakes-only.obd");
        std::fs::write(&path, &bytes).expect("write fakes-only copy");
        let err = load(&path).expect_err("no bulletin anywhere refuses");
        assert_eq!(
            err, "that file has no bulletin inside it (no document.json)",
            "no document error anywhere: the no-bulletin sentence"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The shared-directory shape with PLAIN end
    /// records (round 12, RT12-3): one local header
    /// and one central directory — the head of the
    /// zip64 shared builder, up to its zip64 record —
    /// then `candidates` plain end records, every one
    /// declaring that same directory (same offset,
    /// same size, same total). No zip64 structure
    /// stands anywhere in the file.
    fn shared_directory_plain(candidates: usize, entries: usize) -> Vec<u8> {
        let full = shared_directory_zip(1, entries, None);
        let z64_at = full.len() - 56 - 20 - 22;
        let cd_offset = u64::from_le_bytes(
            full[z64_at + 48..z64_at + 56]
                .try_into()
                .expect("offset field"),
        );
        let cd_size = (z64_at as u64 - cd_offset) as u32;
        let mut out = full[..z64_at].to_vec();
        for _ in 0..candidates {
            let mut eocd = Vec::new();
            eocd.extend_from_slice(b"PK\x05\x06");
            eocd.extend_from_slice(&0u16.to_le_bytes());
            eocd.extend_from_slice(&0u16.to_le_bytes());
            eocd.extend_from_slice(&(entries as u16).to_le_bytes());
            eocd.extend_from_slice(&(entries as u16).to_le_bytes());
            eocd.extend_from_slice(&cd_size.to_le_bytes());
            eocd.extend_from_slice(&(cd_offset as u32).to_le_bytes());
            eocd.extend_from_slice(&0u16.to_le_bytes());
            assert_eq!(eocd.len(), 22);
            out.extend_from_slice(&eocd);
        }
        out
    }

    #[test]
    fn r12b_plain_candidates_sharing_one_directory_pay_for_it_once() {
        // Round 12 (R12-3, red team RT12-3): the
        // shared-directory dedupe keyed a candidate by
        // its own end record's position, and a PLAIN
        // candidate's end is its own position — so
        // plain candidates never deduped, and 2,900
        // plain end records naming one 8,191-entry
        // directory took 113.0 s to refuse against a
        // 228 ms control (300: ~10 s), where the zip64
        // shape landed in class because its candidates
        // share one zip64 record. Candidate identity
        // is the directory described now — its start,
        // its size, the zip64 record where one stands —
        // so the plain shape enumerates once and both
        // doors land in the control's time class.
        let dir = scratch("r12b-plain-shared");
        let entries = MAX_ENTRIES - 1;
        let control_path = dir.join("control.obd");
        std::fs::write(&control_path, shared_directory_control(entries)).expect("write control");
        let control_start = std::time::Instant::now();
        let control_err = load(&control_path).expect_err("the control holds no bulletin");
        let control_elapsed = control_start.elapsed();
        assert_eq!(
            control_err, "that file has no bulletin inside it (no document.json)",
            "the control's refusal"
        );
        let sheet_start = std::time::Instant::now();
        let sheet_err = load_insert(&control_path).expect_err("the control holds no sheet");
        let sheet_elapsed = sheet_start.elapsed();
        assert_eq!(
            sheet_err, "that file has no loose sheet inside it (no insert.json)",
            "the control's sheet refusal"
        );
        for (tag, candidates) in [("300", 300usize), ("2900", 2900)] {
            let path = dir.join(format!("plain-{tag}.obd"));
            std::fs::write(&path, shared_directory_plain(candidates, entries))
                .expect("write plain shared copy");
            assert_eq!(
                directory_candidates(&path, MAX_ENTRIES).len(),
                1,
                "{tag} plain candidates: the shared directory is enumerated once"
            );
            let start = std::time::Instant::now();
            let err = load(&path).expect_err("the shared directory holds no bulletin");
            let elapsed = start.elapsed();
            assert_eq!(
                err, "that file has no bulletin inside it (no document.json)",
                "{tag} plain candidates: the refusal is the control's"
            );
            println!("r12b plain-{tag} bulletin door: {elapsed:?} (control {control_elapsed:?})");
            let budget = shared_directory_budget(control_elapsed, candidates);
            assert!(
                elapsed < budget,
                "{tag} plain candidates: {elapsed:?} must land in the control's \
                 time class (control {control_elapsed:?}, budget {budget:?})"
            );
            let start = std::time::Instant::now();
            let err = load_insert(&path).expect_err("the shared directory holds no sheet");
            let elapsed = start.elapsed();
            assert_eq!(
                err, "that file has no loose sheet inside it (no insert.json)",
                "{tag} plain candidates at the sheet door"
            );
            println!("r12b plain-{tag} sheet door: {elapsed:?} (control {sheet_elapsed:?})");
            let budget = shared_directory_budget(sheet_elapsed, candidates);
            assert!(
                elapsed < budget,
                "{tag} plain candidates at the sheet door: {elapsed:?} must land \
                 in the control's time class (control {sheet_elapsed:?}, budget {budget:?})"
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 70,000 bytes of signature-free junk carrying
    /// 100 fake end records (the round-12 review
    /// lane's memo-witness filler): each fake stands
    /// 700 bytes after the last, declares a distinct
    /// total and an offset far past the file, and so
    /// fails its own resolution at once — while the
    /// file's candidate count passes 64.
    fn junk_with_fake_end_records() -> Vec<u8> {
        let mut out = Vec::new();
        for i in 0..100u32 {
            let mut fake = Vec::new();
            fake.extend_from_slice(b"PK\x05\x06");
            fake.extend_from_slice(&0u16.to_le_bytes());
            fake.extend_from_slice(&0u16.to_le_bytes());
            fake.extend_from_slice(&(1000u16 + i as u16).to_le_bytes());
            fake.extend_from_slice(&(1000u16 + i as u16).to_le_bytes());
            fake.extend_from_slice(&0u32.to_le_bytes());
            fake.extend_from_slice(&0x00FF_FFF0u32.to_le_bytes());
            fake.extend_from_slice(&0u16.to_le_bytes());
            assert_eq!(fake.len(), 22);
            out.extend_from_slice(&fake);
            out.extend(vec![b'J'; 678]);
        }
        assert_eq!(out.len(), 70_000);
        out
    }

    #[test]
    fn r12b_an_identical_fields_decoy_never_poisons_the_honest_candidate() {
        // Round 12 (R12-4, review R12-1): the fallback
        // memoized resolutions under the DECLARED
        // fields alone, but resolution depends on the
        // candidate's own position — an exact copy of
        // the honest end record, appended after 70,000
        // bytes of junk carrying 100 fakes, resolves
        // at ITS position to a walk that fails across
        // the junk, and the memo then handed that
        // failure to the honest candidate standing
        // earlier, which was never resolved at all:
        // the file was refused. The same file with no
        // decoy loaded, and with a decoy varied by one
        // byte in its size field loaded (a fresh memo
        // key). There is no memo now: every candidate
        // is resolved at its own position in the one
        // shared pass, and all three arrangements load
        // the honest file, at both doors.
        let doc_json = serde_json::to_string(&test_doc()).expect("json");
        let sheet_json = lying_sheet_json();
        let dir = scratch("r12b-memo-decoy");
        for (json_name, json, ext, honest_title) in [
            ("document.json", doc_json.as_str(), "obd", "Roundtrip"),
            ("insert.json", sheet_json.as_str(), "obdi", "sheet-lying"),
        ] {
            let honest = zip_with_fillers(json_name, json, 1);
            let eocd_at = find_eocd(&honest);
            let eocd = honest[eocd_at..].to_vec();
            let mut base = honest.clone();
            base.extend_from_slice(&junk_with_fake_end_records());
            let mut varied = eocd.clone();
            let size = u32::from_le_bytes(varied[12..16].try_into().expect("size field"));
            varied[12..16].copy_from_slice(&(size + 1).to_le_bytes());
            let mut with_varied = base.clone();
            with_varied.extend_from_slice(&varied);
            let mut with_identical = base.clone();
            with_identical.extend_from_slice(&eocd);
            for (tag, bytes) in [
                ("no-decoy", base),
                ("varied-decoy", with_varied),
                ("identical-decoy", with_identical),
            ] {
                let path = dir.join(format!("{tag}.{ext}"));
                std::fs::write(&path, &bytes).expect("write decoy copy");
                if ext == "obd" {
                    let loaded = load(&path)
                        .unwrap_or_else(|e| panic!("{ext} {tag}: honest loads, got {e}"));
                    assert_eq!(loaded.title, honest_title, "{ext} {tag}");
                } else {
                    let (insert, _, _) = load_insert(&path)
                        .unwrap_or_else(|e| panic!("{ext} {tag}: honest loads, got {e}"));
                    assert_eq!(insert.id, honest_title, "{ext} {tag}");
                }
            }
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// One cost-fixture file (the round-12 review
    /// lane's shape): `total` bytes of filler with a
    /// zip head and a fake end record every 512
    /// bytes, each declaring offset zero and no
    /// directory anywhere. `vary` gives each record a
    /// distinct total — a distinct declared
    /// description — where the identical shape gives
    /// every record the same one.
    fn cost_fixture(total: usize, vary: bool) -> Vec<u8> {
        let mut out = vec![b'J'; total];
        out[0..4].copy_from_slice(b"PK\x03\x04");
        let mut i = 0u16;
        let mut at = 512usize;
        while at + 22 <= total {
            for byte in &mut out[at..at + 22] {
                *byte = 0;
            }
            out[at..at + 4].copy_from_slice(b"PK\x05\x06");
            let total16: u16 = if vary { i } else { 7 };
            out[at + 10..at + 12].copy_from_slice(&total16.to_le_bytes());
            i += 1;
            at += 512;
        }
        out
    }

    #[test]
    fn r12b_distinct_declared_fields_pay_one_shared_resolution() {
        // Round 12 (R12-5, review R12-2): with the
        // per-candidate search bound deleted, each
        // DISTINCT declared description paid a
        // whole-span resolution search of its own —
        // 0.79 s / 2.47 s / 10.95 s at 1 / 2 / 4 MB in
        // the lane's release build against 6.5–42 ms
        // for identical descriptions. Resolution is
        // one shared pass over the span now, computing
        // every candidate's resolution positionally,
        // so the distinct files land in the identical
        // files' time class at every size.
        let dir = scratch("r12b-distinct-cost");
        for total in [1_000_000usize, 2_000_000, 4_000_000] {
            let identical_path = dir.join(format!("identical-{total}.obd"));
            std::fs::write(&identical_path, cost_fixture(total, false))
                .expect("write identical copy");
            let start = std::time::Instant::now();
            let err = load(&identical_path).expect_err("no candidate yields a bulletin");
            let identical_elapsed = start.elapsed();
            assert_eq!(
                err, "that file has no bulletin inside it (no document.json)",
                "identical {total}: the plain refusal"
            );
            let distinct_path = dir.join(format!("distinct-{total}.obd"));
            std::fs::write(&distinct_path, cost_fixture(total, true)).expect("write distinct copy");
            let start = std::time::Instant::now();
            let err = load(&distinct_path).expect_err("no candidate yields a bulletin");
            let distinct_elapsed = start.elapsed();
            assert_eq!(
                err, "that file has no bulletin inside it (no document.json)",
                "distinct {total}: the plain refusal"
            );
            println!(
                "r12b cost {total}: identical {identical_elapsed:?}, distinct {distinct_elapsed:?}"
            );
            let budget = std::cmp::max(
                identical_elapsed * 20,
                identical_elapsed + std::time::Duration::from_secs(2),
            );
            assert!(
                distinct_elapsed < budget,
                "distinct {total}: {distinct_elapsed:?} must land in the identical \
                 files' time class (identical {identical_elapsed:?}, budget {budget:?})"
            );
            assert!(
                distinct_elapsed < std::time::Duration::from_secs(5),
                "distinct {total}: {distinct_elapsed:?} is no shared-pass time at all"
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn r12b_a_gap_after_the_directory_loads_behind_many_fakes() {
        // Round 12 (R12-10, review R12-6): an honest
        // file with 16 bytes standing between its
        // directory's end and its end record loads
        // alone — the zip reader reads the directory
        // by its declared offset and size and tolerates
        // the gap, and the reader gate's small-count
        // clause lets the attempt run. Appending 100
        // fake end records closed that clause (more
        // than 64 candidates), the latest fake did not
        // walk, and the chain-exact walk rejects the
        // honest candidate at the gap: refused. The
        // honest directory's records chain exactly
        // across its declared size, so it stands as a
        // readable directory the walk can name, the
        // reader attempt is reachable while it stands
        // untried, and the fallback can open it at its
        // own end record — the construction loads, at
        // both doors.
        let doc_json = serde_json::to_string(&test_doc()).expect("json");
        let sheet_json = lying_sheet_json();
        let dir = scratch("r12b-gap-after-directory");
        for (json_name, json, ext, honest_title) in [
            ("document.json", doc_json.as_str(), "obd", "Roundtrip"),
            ("insert.json", sheet_json.as_str(), "obdi", "sheet-lying"),
        ] {
            let honest = zip_with_fillers(json_name, json, 1);
            let eocd = find_eocd(&honest);
            let mut gapped = honest[..eocd].to_vec();
            gapped.extend_from_slice(b"GGGGGGGGGGGGGGGG");
            gapped.extend_from_slice(&honest[eocd..]);
            let mut with_fakes = gapped.clone();
            for i in 0..100u32 {
                let mut fake = Vec::new();
                fake.extend_from_slice(b"PK\x05\x06");
                fake.extend_from_slice(&0u16.to_le_bytes());
                fake.extend_from_slice(&0u16.to_le_bytes());
                fake.extend_from_slice(&(2000u16 + i as u16).to_le_bytes());
                fake.extend_from_slice(&(2000u16 + i as u16).to_le_bytes());
                fake.extend_from_slice(&0u32.to_le_bytes());
                fake.extend_from_slice(&0x00FF_FFF0u32.to_le_bytes());
                fake.extend_from_slice(&0u16.to_le_bytes());
                assert_eq!(fake.len(), 22);
                with_fakes.extend_from_slice(&fake);
            }
            for (tag, bytes) in [("gapped-alone", gapped), ("gapped-100-fakes", with_fakes)] {
                let path = dir.join(format!("{tag}.{ext}"));
                std::fs::write(&path, &bytes).expect("write gapped copy");
                if ext == "obd" {
                    let loaded = load(&path)
                        .unwrap_or_else(|e| panic!("{ext} {tag}: honest loads, got {e}"));
                    assert_eq!(loaded.title, honest_title, "{ext} {tag}");
                } else {
                    let (insert, _, _) = load_insert(&path)
                        .unwrap_or_else(|e| panic!("{ext} {tag}: honest loads, got {e}"));
                    assert_eq!(insert.id, honest_title, "{ext} {tag}");
                }
            }
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn r12b_a_lying_zip64_total_loads() {
        // Round 12 (R12-9b, red team RT12-7): a plain
        // honest bulletin spliced with a zip64 record
        // whose total lies — 999,999, or 1, against two
        // real records; offsets and size honest — was
        // REFUSED with the reader's "inconsistent
        // number of files", while the count walk counts
        // its two records without complaint: the walk
        // never consults the total, and the reader
        // checks it against the records it parses. The
        // loaders' opens now rewrite the record's two
        // total fields to the walked count in the
        // trimmed stream or copy they hand the reader
        // (never in the file), so the reader path
        // tolerates the lie the way the walk does and
        // the bulletin loads, at both doors.
        let doc_json = serde_json::to_string(&test_doc()).expect("json");
        let sheet_json = lying_sheet_json();
        let dir = scratch("r12b-lying-zip64-total");
        for (json_name, json, ext, honest_title) in [
            ("document.json", doc_json.as_str(), "obd", "Roundtrip"),
            ("insert.json", sheet_json.as_str(), "obdi", "sheet-lying"),
        ] {
            let small = zip_with_fillers(json_name, json, 1);
            let eocd = find_eocd(&small);
            let honest_size = u64::from(u32::from_le_bytes(
                small[eocd + 12..eocd + 16].try_into().expect("size field"),
            ));
            for (tag, lying_total) in [("lie-up", 999_999u64), ("lie-down", 1)] {
                let bytes = splice_zip64(&small, lying_total, honest_size);
                let path = dir.join(format!("{tag}.{ext}"));
                std::fs::write(&path, &bytes).expect("write lying-total copy");
                assert_eq!(
                    raw_entry_count(&path, MAX_ENTRIES),
                    Some(2),
                    "{ext} {tag}: the walk counts the two records, total or no total"
                );
                if ext == "obd" {
                    let loaded = load(&path)
                        .unwrap_or_else(|e| panic!("{ext} {tag}: honest loads, got {e}"));
                    assert_eq!(loaded.title, honest_title, "{ext} {tag}");
                } else {
                    let (insert, _, _) = load_insert(&path)
                        .unwrap_or_else(|e| panic!("{ext} {tag}: honest loads, got {e}"));
                    assert_eq!(insert.id, honest_title, "{ext} {tag}");
                }
            }
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// One fake end record with caller-set fields
    /// (round 13's builders): both total fields carry
    /// `total`, the size and offset fields carry the
    /// caller's values, the comment is empty.
    fn fake_end_record(total: u16, size: u32, offset: u32) -> Vec<u8> {
        let mut eocd = Vec::new();
        eocd.extend_from_slice(b"PK\x05\x06");
        eocd.extend_from_slice(&0u16.to_le_bytes());
        eocd.extend_from_slice(&0u16.to_le_bytes());
        eocd.extend_from_slice(&total.to_le_bytes());
        eocd.extend_from_slice(&total.to_le_bytes());
        eocd.extend_from_slice(&size.to_le_bytes());
        eocd.extend_from_slice(&offset.to_le_bytes());
        eocd.extend_from_slice(&0u16.to_le_bytes());
        assert_eq!(eocd.len(), 22);
        eocd
    }

    /// The round-13 chain-cost file (red team
    /// RT13-2): a gap-standing honest two-record zip
    /// first (fallback-only, like the R12-10 shape),
    /// then one documentless directory of
    /// `chain_entries` records, then `fakes` fake
    /// end records naming that directory — every
    /// fake resolves to the one long chain, so a
    /// per-candidate chain scan pays the chain's
    /// length per fake.
    fn chain_cost_file(json_name: &str, json: &str, chain_entries: usize, fakes: usize) -> Vec<u8> {
        let honest = zip_with_fillers(json_name, json, 1);
        let eocd = find_eocd(&honest);
        let mut out = honest[..eocd].to_vec();
        out.extend_from_slice(b"GGGGGGGGGGGGGGGG");
        out.extend_from_slice(&honest[eocd..]);
        let chain_start = out.len();
        let mut chain_len = 0usize;
        for i in 0..chain_entries {
            let name = format!("f{i:05}");
            let record = shared_central_record(&name, 0, 0, 0);
            chain_len += record.len();
            out.extend_from_slice(&record);
        }
        for _ in 0..fakes {
            out.extend_from_slice(&fake_end_record(
                chain_entries as u16,
                chain_len as u32,
                chain_start as u32,
            ));
        }
        out
    }

    #[test]
    fn r13b_chain_cost_no_longer_scales_with_chain_length() {
        // Round 13 (R13-3, red team RT13-2): the
        // shared resolution pass read each candidate's
        // count off the shared chain with a LINEAR
        // scan, so cost was O(candidates × chain
        // length), not the O(span + candidates) the
        // R12-B account claimed — 40k body signatures
        // cost 26–71 ms at a 2-record chain but
        // 1.32–1.60 s at an 8,000-record chain, and
        // 200k fakes on a 5.2 MB file cost 6.80 s in
        // the lane's release build. Counts are read
        // off the chain's position index now, built
        // once per chain when it is walked, so the
        // long-chain rows land in the short-chain
        // row's time class and the 200k row lands in
        // the sub-second class. The honest bulletin
        // loads in every row.
        let doc_json = serde_json::to_string(&test_doc()).expect("json");
        let dir = scratch("r13b-chain-cost");
        let mut elapsed_rows: Vec<std::time::Duration> = Vec::new();
        for (tag, chain_entries, fakes) in [
            ("chain2-40k", 2usize, 40_000usize),
            ("chain8000-40k", 8_000, 40_000),
            ("chain8000-200k", 8_000, 200_000),
        ] {
            let bytes = chain_cost_file("document.json", &doc_json, chain_entries, fakes);
            let path = dir.join(format!("{tag}.obd"));
            std::fs::write(&path, &bytes).expect("write chain-cost copy");
            let start = std::time::Instant::now();
            let loaded = load(&path).unwrap_or_else(|e| panic!("{tag}: honest loads, got {e}"));
            let elapsed = start.elapsed();
            assert_eq!(loaded.title, "Roundtrip", "{tag}");
            println!("r13b {tag}: {elapsed:?} ({} bytes)", bytes.len());
            elapsed_rows.push(elapsed);
        }
        let short = elapsed_rows[0];
        let long = elapsed_rows[1];
        let budget = std::cmp::max(short * 20, short + std::time::Duration::from_secs(2));
        assert!(
            long < budget,
            "chain8000-40k: {long:?} must land in the chain2-40k time class \
             (chain2-40k {short:?}, budget {budget:?})"
        );
        // The flat 5 s caps that stood here are a
        // HANG GUARD now (FLAGGED, round 14 R14-7,
        // by disposition — the same treatment round
        // 13's R13-8 gave the r10 pin): a 300 s cap
        // whose only job is catching a hung or
        // runaway load. In round 14 the 5 s caps
        // failed under contention in two lanes —
        // rows measured 5.27–5.82 s at machine loads
        // ~11–15, with every outcome assertion
        // passing (the honest bulletin loaded in
        // every row) and solo/release runs far
        // inside — so the caps were grading the
        // machine's load, not this code. The
        // outcome assertions above and the relative
        // time-class budget (long against the
        // short-chain row) are unchanged and
        // primary; they guard the chain-cost class.
        // The r12b pins' relative budgets stand
        // untouched: they have not failed.
        let hang_guard = std::time::Duration::from_secs(300);
        assert!(
            long < hang_guard,
            "chain8000-40k: {long:?} must finish within the hang guard {hang_guard:?}"
        );
        assert!(
            elapsed_rows[2] < hang_guard,
            "chain8000-200k: {:?} must finish within the hang guard {hang_guard:?}",
            elapsed_rows[2]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn r13b_a_gap_directory_with_a_lying_size_loads_behind_many_fakes() {
        // Round 13 (R13-4a, review R13-3): the R12-10
        // gap shape with the end record's declared
        // SIZE patched by −7 (lands mid-record) or +1
        // (lands in the gap), behind 100 fakes, was
        // REFUSED although the reader reads the
        // directory whole: `reader_walkable` chained
        // records across the declared size, while the
        // reader (zip 2.4.2) parses exactly its
        // declared TOTAL of records and never consults
        // the size. The walkable test mirrors the
        // reader now — the chain parses the declared
        // total — so both deltas load, at both doors.
        // The premises are asserted in-test: the
        // resolution walks not at all and reads by
        // the reader's rule, and the trimmed reader
        // stream reads the document entry.
        let doc_json = serde_json::to_string(&test_doc()).expect("json");
        let sheet_json = lying_sheet_json();
        let dir = scratch("r13b-lying-size-gap");
        for (json_name, json, ext, honest_title) in [
            ("document.json", doc_json.as_str(), "obd", "Roundtrip"),
            ("insert.json", sheet_json.as_str(), "obdi", "sheet-lying"),
        ] {
            let honest = zip_with_fillers(json_name, json, 1);
            let eocd = find_eocd(&honest);
            let honest_size =
                u32::from_le_bytes(honest[eocd + 12..eocd + 16].try_into().expect("size field"));
            for (tag, delta) in [("minus-seven", -7i64), ("plus-one", 1i64)] {
                let mut gapped = honest[..eocd].to_vec();
                gapped.extend_from_slice(b"GGGGGGGGGGGGGGGG");
                gapped.extend_from_slice(&honest[eocd..]);
                let gapped_eocd = eocd + 16;
                let lying_size = (i64::from(honest_size) + delta) as u32;
                gapped[gapped_eocd + 12..gapped_eocd + 16]
                    .copy_from_slice(&lying_size.to_le_bytes());
                let resolutions = candidate_resolutions_in(&gapped, &[gapped_eocd]);
                let resolution = &resolutions[0];
                assert!(
                    !resolution.walkable,
                    "{ext} {tag}: the gap breaks the chain-exact walk"
                );
                assert!(
                    resolution.reader_walkable,
                    "{ext} {tag}: the reader's total-driven reading stands"
                );
                let trimmed = archive_from_candidate(&gapped, gapped_eocd, None)
                    .expect("the reader reads the trimmed stream");
                assert!(
                    trimmed.file_names().any(|n| n == json_name),
                    "{ext} {tag}: the reader reads the document entry"
                );
                let mut bytes = gapped.clone();
                for i in 0..100u32 {
                    bytes.extend_from_slice(&fake_end_record((2000 + i) as u16, 0, 0x00FF_FFF0));
                }
                let path = dir.join(format!("{tag}.{ext}"));
                std::fs::write(&path, &bytes).expect("write lying-size copy");
                if ext == "obd" {
                    let loaded = load(&path)
                        .unwrap_or_else(|e| panic!("{ext} {tag}: honest loads, got {e}"));
                    assert_eq!(loaded.title, honest_title, "{ext} {tag}");
                } else {
                    let (insert, _, _) = load_insert(&path)
                        .unwrap_or_else(|e| panic!("{ext} {tag}: honest loads, got {e}"));
                    assert_eq!(insert.id, honest_title, "{ext} {tag}");
                }
            }
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn r13b_a_zero_total_later_twin_does_not_skip_the_honest_twin() {
        // Round 13 (R13-4b, review R13-4): two plain
        // end records describing the same directory —
        // one DirKey — the LATER twin declaring total
        // 0, the earlier one honest (total 2), a gap
        // before the earlier so neither chain-counts.
        // The whole-file reader stops at the later
        // twin and reads an empty archive (a successful
        // open); the fallback then opened that empty
        // twin, marked the shared key tried, and
        // skipped the honest one: refused. The dedupe
        // unit is the twin — key plus declared total —
        // now, so the honest twin is tried under its
        // own total and the file loads, at both doors.
        // Premises asserted in-test: one key, totals
        // 0 and 2; the whole-file reader reads empty;
        // the honest twin's trimmed stream reads the
        // document.
        let doc_json = serde_json::to_string(&test_doc()).expect("json");
        let sheet_json = lying_sheet_json();
        let dir = scratch("r13b-zero-total-twin");
        for (json_name, json, ext, honest_title) in [
            ("document.json", doc_json.as_str(), "obd", "Roundtrip"),
            ("insert.json", sheet_json.as_str(), "obdi", "sheet-lying"),
        ] {
            let honest = zip_with_fillers(json_name, json, 1);
            let eocd = find_eocd(&honest);
            let mut bytes = honest[..eocd].to_vec();
            bytes.extend_from_slice(b"GGGGGGGGGGGGGGGG");
            let honest_twin_at = bytes.len();
            bytes.extend_from_slice(&honest[eocd..]);
            let mut zero_twin = honest[eocd..].to_vec();
            zero_twin[8..10].copy_from_slice(&0u16.to_le_bytes());
            zero_twin[10..12].copy_from_slice(&0u16.to_le_bytes());
            let zero_twin_at = bytes.len();
            bytes.extend_from_slice(&zero_twin);
            let resolutions = candidate_resolutions_in(&bytes, &[zero_twin_at, honest_twin_at]);
            assert_eq!(
                resolutions[0].key, resolutions[1].key,
                "{ext}: the twins share one resolved directory"
            );
            assert_eq!(
                resolutions[0].declared_total, 0,
                "{ext}: the later twin's total"
            );
            assert_eq!(
                resolutions[1].declared_total, 2,
                "{ext}: the honest twin's total"
            );
            let whole = zip::ZipArchive::new(std::io::Cursor::new(bytes.clone()))
                .expect("the reader opens the zero twin");
            assert_eq!(
                whole.len(),
                0,
                "{ext}: the reader's chosen twin reads empty"
            );
            let trimmed = archive_from_candidate(&bytes, honest_twin_at, None)
                .expect("the honest twin's stream opens");
            assert!(
                trimmed.file_names().any(|n| n == json_name),
                "{ext}: the honest twin's stream reads the document entry"
            );
            let path = dir.join(format!("twins.{ext}"));
            std::fs::write(&path, &bytes).expect("write twin copy");
            if ext == "obd" {
                let loaded = load(&path).unwrap_or_else(|e| panic!("{ext}: honest loads, got {e}"));
                assert_eq!(loaded.title, honest_title, "{ext}");
            } else {
                let (insert, _, _) =
                    load_insert(&path).unwrap_or_else(|e| panic!("{ext}: honest loads, got {e}"));
                assert_eq!(insert.id, honest_title, "{ext}");
            }
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn r13b_a_lying_total_twin_does_not_close_the_honest_twin_out() {
        // Round 13 (R13-4c, red team RT13-3): one
        // honest 2-record directory (junk.txt first,
        // the document second). Candidate F stands AT
        // the directory's end with its total lying —
        // 1 (reads only junk.txt) or 5000 (does not
        // open at all); candidate H stands after a
        // 16-byte gap with honest fields, under the
        // SAME DirKey (the key drops the total once a
        // start resolves); 70 unwalkable fakes stand
        // last. The load refused at both doors: F's
        // key in the tried set made H untried-readable
        // nowhere and a repeat in the fallback,
        // although reader-alone controls load the
        // file. Per key, the representative is now the
        // best reading any twin achieves — the dedupe
        // is by twin, key plus total — so H is tried
        // under its own total and the file loads, at
        // both doors, for both lies. Premises asserted
        // in-test: one key, the two totals; F's own
        // stream yields junk only or nothing; H's
        // stream reads the document.
        let doc_json = serde_json::to_string(&test_doc()).expect("json");
        let sheet_json = lying_sheet_json();
        let dir = scratch("r13b-lying-total-twin");
        for (json_name, json, ext, honest_title) in [
            ("document.json", doc_json.as_str(), "obd", "Roundtrip"),
            ("insert.json", sheet_json.as_str(), "obdi", "sheet-lying"),
        ] {
            // The document stands SECOND, so a total
            // of 1 reads only the junk entry.
            let honest = stored_zip(&[("junk.txt", b"x".as_slice()), (json_name, json.as_bytes())]);
            let eocd = find_eocd(&honest);
            for (tag, lying_total) in [("total-one", 1u16), ("total-5000", 5000u16)] {
                let mut bytes = honest[..eocd].to_vec();
                let mut lying_twin = honest[eocd..].to_vec();
                lying_twin[8..10].copy_from_slice(&lying_total.to_le_bytes());
                lying_twin[10..12].copy_from_slice(&lying_total.to_le_bytes());
                let lying_twin_at = bytes.len();
                bytes.extend_from_slice(&lying_twin);
                bytes.extend_from_slice(b"GGGGGGGGGGGGGGGG");
                let honest_twin_at = bytes.len();
                bytes.extend_from_slice(&honest[eocd..]);
                for i in 0..70u32 {
                    bytes.extend_from_slice(&fake_end_record((3000 + i) as u16, 0, 0x00FF_FFF0));
                }
                let resolutions =
                    candidate_resolutions_in(&bytes, &[lying_twin_at, honest_twin_at]);
                assert_eq!(
                    resolutions[0].key, resolutions[1].key,
                    "{ext} {tag}: the twins share one resolved directory"
                );
                assert_eq!(
                    resolutions[0].declared_total,
                    u64::from(lying_total),
                    "{ext} {tag}: the lying twin's total"
                );
                assert_eq!(
                    resolutions[1].declared_total, 2,
                    "{ext} {tag}: the honest total"
                );
                match archive_from_candidate(&bytes, lying_twin_at, None) {
                    Some(archive) => {
                        assert!(
                            !archive.file_names().any(|n| n == json_name),
                            "{ext} {tag}: the lying twin's stream holds no document"
                        );
                    }
                    None => {
                        assert_eq!(
                            tag, "total-5000",
                            "{ext}: only the 5000 twin fails to open at all"
                        );
                    }
                }
                let trimmed = archive_from_candidate(&bytes, honest_twin_at, None)
                    .expect("the honest twin's stream opens");
                assert!(
                    trimmed.file_names().any(|n| n == json_name),
                    "{ext} {tag}: the honest twin's stream reads the document entry"
                );
                let path = dir.join(format!("{tag}.{ext}"));
                std::fs::write(&path, &bytes).expect("write lying-total copy");
                if ext == "obd" {
                    let loaded = load(&path)
                        .unwrap_or_else(|e| panic!("{ext} {tag}: honest loads, got {e}"));
                    assert_eq!(loaded.title, honest_title, "{ext} {tag}");
                } else {
                    let (insert, _, _) = load_insert(&path)
                        .unwrap_or_else(|e| panic!("{ext} {tag}: honest loads, got {e}"));
                    assert_eq!(insert.id, honest_title, "{ext} {tag}");
                }
            }
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
