//! The OpenBulletin Font Library (docs/FONTS.md).
//!
//! One app-managed folder of font files, identical on a desktop and
//! in the Docker/NAS shape, because the layout engine only ever uses
//! fonts handed to it from here: the same bulletin must lay out the
//! same way on every machine. System fonts are copied in, never
//! linked live.
//!
//! Every font carries its embedding permission (the OpenType fsType
//! bits) translated into plain words, and a SHA-256 of its bytes:
//! documents name fonts by family + hash, so a bulletin that travels
//! also says exactly which font file it was laid out with.

pub mod catalog;
pub mod resolve;
pub mod ttc;

use serde::Serialize;
use std::path::{Path, PathBuf};

/// What the font's own permission bits say about embedding it in
/// files that travel (an .obd package or a shared PDF).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum EmbeddingRights {
    /// May be embedded and installed by the receiver.
    Installable,
    /// May be embedded; receiver may edit with it temporarily.
    Editable,
    /// May be embedded for looking and printing only.
    PreviewAndPrint,
    /// May not be embedded at all.
    Restricted,
    /// The font carries no readable permission bits.
    Unknown,
}

impl EmbeddingRights {
    /// Plain-words explanation for the admin, per the ELI5 rule.
    pub fn plain_words(&self) -> &'static str {
        match self {
            EmbeddingRights::Installable => {
                "Free to share: this font may travel inside bulletin files, and whoever receives one may keep and use the font."
            }
            EmbeddingRights::Editable => {
                "May travel inside bulletin files so others can open and edit them, but it is not theirs to keep."
            }
            EmbeddingRights::PreviewAndPrint => {
                "May be used to show and print this bulletin, but the font itself must not travel inside shared files."
            }
            EmbeddingRights::Restricted => {
                "This font must not be embedded or shared. You can use it on this machine only."
            }
            EmbeddingRights::Unknown => {
                "This font does not say what sharing it allows. Treat it as this machine only, unless you know its license."
            }
        }
    }

    /// May this font's bytes ride inside an .obd package?
    pub fn may_package(&self) -> bool {
        matches!(
            self,
            EmbeddingRights::Installable | EmbeddingRights::Editable
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FontInfo {
    /// Family name, e.g. "Liberation Sans".
    pub family: String,
    /// Full face name, e.g. "Liberation Sans Bold".
    pub full_name: String,
    pub bold: bool,
    pub italic: bool,
    pub rights: EmbeddingRights,
    /// True when the font forbids subsetting before embedding.
    pub no_subsetting: bool,
    /// File name inside the library folder.
    pub file_name: String,
    /// Hex SHA-256 of the font file bytes.
    pub sha256: String,
    pub bytes: usize,
}

/// Read the facts out of one font file's bytes.
pub fn inspect(bytes: &[u8], file_name: &str) -> Result<FontInfo, String> {
    let face = ttf_parser::Face::parse(bytes, 0)
        .map_err(|e| format!("not a readable font file: {e:?}"))?;
    let name = |id: u16| -> Option<String> {
        face.names()
            .into_iter()
            .find(|n| n.name_id == id && n.is_unicode())
            .and_then(|n| n.to_string())
    };
    let family = name(ttf_parser::name_id::TYPOGRAPHIC_FAMILY)
        .or_else(|| name(ttf_parser::name_id::FAMILY))
        .ok_or("the font does not carry a family name")?;
    let full_name = name(ttf_parser::name_id::FULL_NAME).unwrap_or_else(|| family.clone());
    let (rights, no_subsetting) = match face.tables().os2 {
        Some(os2) => {
            use ttf_parser::Permissions::*;
            let rights = match os2.permissions() {
                Some(Installable) => EmbeddingRights::Installable,
                Some(Editable) => EmbeddingRights::Editable,
                Some(PreviewAndPrint) => EmbeddingRights::PreviewAndPrint,
                Some(Restricted) => EmbeddingRights::Restricted,
                None => EmbeddingRights::Unknown,
            };
            (rights, !os2.is_subsetting_allowed())
        }
        None => (EmbeddingRights::Unknown, false),
    };
    Ok(FontInfo {
        family,
        full_name,
        bold: face.is_bold(),
        italic: face.is_italic(),
        rights,
        no_subsetting,
        file_name: file_name.to_string(),
        // The one SHA-256 statement (health audit
        // E-10): ob-model's, as every other crate uses.
        sha256: ob_model::sha256_hex(bytes),
        bytes: bytes.len(),
    })
}

/// The default library folder: next to the app settings, in an
/// `OpenBulletin/fonts` folder, on every platform shape.
pub fn default_library_dir() -> PathBuf {
    let base = if let Ok(appdata) = std::env::var("APPDATA") {
        PathBuf::from(appdata)
    } else if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
        PathBuf::from(xdg)
    } else if let Ok(home) = std::env::var("HOME") {
        let mac = PathBuf::from(&home).join("Library/Application Support");
        if mac.exists() {
            mac
        } else {
            PathBuf::from(home).join(".local/share")
        }
    } else {
        PathBuf::from(".")
    };
    base.join("OpenBulletin").join("fonts")
}

/// The plain-words refusal for a font file in a format
/// OpenBulletin cannot use (docs/FONTS.md, font file
/// formats), naming the file: web fonts (WOFF and WOFF2
/// are the same font dressed for a browser; the file's
/// magic bytes say so even when the name lies) and Type 1
/// fonts (the 1980s format modern systems have retired).
/// None for anything else: the font parser is the judge
/// of the formats we do take.
pub fn format_refusal(bytes: &[u8], file_name: &str) -> Option<String> {
    let lower = file_name.to_lowercase();
    let web = bytes.starts_with(b"wOFF")
        || bytes.starts_with(b"wOF2")
        || lower.ends_with(".woff")
        || lower.ends_with(".woff2");
    if web {
        return Some(format!(
            "{file_name}: That is the web version of a font. OpenBulletin \
             prints, so it needs the TTF or OTF version of the same font."
        ));
    }
    let type1 = lower.ends_with(".pfb")
        || lower.ends_with(".afm")
        || lower.ends_with(".pfm")
        || bytes.starts_with(b"%!PS")
        || bytes.starts_with(b"StartFontMetrics");
    if type1 {
        return Some(format!(
            "{file_name}: That font is in the Type 1 format, which is too \
             old for OpenBulletin to use. The TTF or OTF version of the \
             same font will work."
        ));
    }
    None
}

/// The file name a split-out collection face is saved
/// under: its own full name, spaces out, with the
/// extension its outlines call for. A face that will not
/// inspect keeps the collection's stem and its number.
fn collection_face_name(face: &[u8], stem: &str, index: usize) -> String {
    match inspect(face, "face") {
        Ok(info) => {
            let ext = if face.starts_with(b"OTTO") {
                "otf"
            } else {
                "ttf"
            };
            format!("{}.{ext}", info.full_name.replace(' ', ""))
        }
        Err(_) => format!("{stem}-{index}.ttf"),
    }
}

/// Reduce any suggested or archived name to a bare file name:
/// split on both slash kinds (a zip made on Windows can carry
/// backslash paths), drop anything empty or dotty.
pub fn sanitize_file_name(name: &str) -> String {
    let base = name
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("")
        .trim()
        .trim_start_matches('.')
        .to_string();
    if base.is_empty() {
        "font.ttf".to_string()
    } else {
        base
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
/// clear, the reader decodes the bytes as CP437.
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

/// Where one central record's entry payload stands
/// and how it is packed, its sizes and local offset
/// resolved through the zip64 extra field wherever a
/// plain field carries the sentinel — the same spot
/// the sibling doors' duplicate walks read (round 31,
/// F1).
struct RawSpot {
    method: u16,
    comp_size: u64,
    crc32: u32,
    local_offset: u64,
}

/// Read one central record's spot out of its fixed
/// header and extra field (the zip64 values ride in
/// the order uncompressed, compressed, local offset,
/// disk, one per sentinel field).
fn raw_spot(header: &[u8], extra: &[u8]) -> RawSpot {
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
    RawSpot {
        method: u16::from_le_bytes([header[10], header[11]]),
        comp_size,
        crc32: field(16),
        local_offset,
    }
}

/// The CRC-32 (zip checksum, IEEE polynomial) table,
/// built at compile time.
const CRC_TABLE: [u32; 256] = {
    let mut table = [0u32; 256];
    let mut i = 0;
    while i < 256 {
        let mut c = i as u32;
        let mut k = 0;
        while k < 8 {
            c = if c & 1 != 0 {
                0xEDB8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
            k += 1;
        }
        table[i] = c;
        i += 1;
    }
    table
};

/// CRC-32 over `bytes`, the checksum the central
/// record declares for its entry: a raw-read copy is
/// judged against it the way the archive reader
/// judges the copies it serves (a declared SIZE is
/// never trusted — round 3, RT3-B — but the checksum
/// is the copy's own fingerprint).
fn crc32_ieee(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in bytes {
        let at = ((crc ^ u32::from(byte)) & 0xFF) as usize;
        crc = CRC_TABLE[at] ^ (crc >> 8);
    }
    !crc
}

/// The (name, spot) pairs of a zip's central directory,
/// walked off the raw records, in archive order and
/// WITH duplicates standing (round 32, F6): the zip
/// reader's own map is keyed by name, so a duplicated
/// name reaches a door that enumerates through the
/// reader as only its last copy, and the first copy is
/// silently absent before the door's own defenses run.
/// The directory is the one the latest end record
/// names, through its zip64 record where the plain
/// fields are sentinels; its first record stands
/// at-or-after the declared offset (junk prepended
/// before the archive shifts every declared offset,
/// the reader's own derivation, round 32, F4), and
/// that archive offset applies to the local offsets
/// in the records. Empty when no end record stands —
/// the caller falls back to the reader's enumeration.
fn central_spots(bytes: &[u8]) -> Vec<(String, RawSpot)> {
    let mut eocd: Option<usize> = None;
    if bytes.len() >= 22 {
        for at in (0..=bytes.len() - 22).rev() {
            if !bytes[at..].starts_with(b"PK\x05\x06") {
                continue;
            }
            let comment = usize::from(u16::from_le_bytes([bytes[at + 20], bytes[at + 21]]));
            if at + 22 + comment <= bytes.len() {
                eocd = Some(at);
                break;
            }
        }
    }
    let Some(eocd) = eocd else {
        return Vec::new();
    };
    let field = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap_or([0; 4]));
    let mut total = u64::from(u16::from_le_bytes([bytes[eocd + 10], bytes[eocd + 11]]));
    let mut cd_offset = u64::from(field(eocd + 16));
    if (total == u64::from(u16::MAX) || cd_offset == u64::from(u32::MAX)) && eocd >= 20 {
        let locator = &bytes[eocd - 20..eocd];
        if locator.starts_with(b"PK\x06\x07") {
            let declared = u64::from_le_bytes(locator[8..16].try_into().unwrap_or([0; 8]));
            let mut record_at = declared as usize;
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
                    let record = &bytes[record_at..record_at + 56];
                    total = u64::from_le_bytes(record[32..40].try_into().unwrap_or([0; 8]));
                    cd_offset = u64::from_le_bytes(record[48..56].try_into().unwrap_or([0; 8]));
                }
            }
        }
    }
    if total == 0 {
        return Vec::new();
    }
    let mut at = cd_offset as usize;
    let mut cd_start = None;
    while at + 4 <= eocd {
        if bytes[at..].starts_with(b"PK\x01\x02") {
            cd_start = Some(at);
            break;
        }
        at += 1;
    }
    let Some(cd_start) = cd_start else {
        return Vec::new();
    };
    let archive_offset = (cd_start as u64).saturating_sub(cd_offset);
    let mut out: Vec<(String, RawSpot)> = Vec::new();
    let mut at = cd_start;
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

/// One entry's decompressed bytes off its raw spot,
/// read one byte past `cap` (so a copy past the cap is
/// known to be past it). The caller judges: a result
/// longer than the cap, or whose checksum is not the
/// one its record declares, is not a readable copy.
/// None when the payload cannot be read back at all.
fn raw_payload(bytes: &[u8], spot: &RawSpot, cap: u64) -> Option<Vec<u8>> {
    use std::io::Read as _;
    let local = spot.local_offset as usize;
    if local + 30 > bytes.len() || !bytes[local..].starts_with(b"PK\x03\x04") {
        return None;
    }
    let name_len = usize::from(u16::from_le_bytes([bytes[local + 26], bytes[local + 27]]));
    let extra_len = usize::from(u16::from_le_bytes([bytes[local + 28], bytes[local + 29]]));
    let data_start = local + 30 + name_len + extra_len;
    let data_end = data_start.checked_add(spot.comp_size as usize)?;
    if data_end > bytes.len() {
        return None;
    }
    let raw = &bytes[data_start..data_end];
    match spot.method {
        0 => Some(raw[..(raw.len() as u64).min(cap + 1) as usize].to_vec()),
        8 => {
            let mut decoder = flate2::read::DeflateDecoder::new(raw);
            let mut out = Vec::new();
            decoder.by_ref().take(cap + 1).read_to_end(&mut out).ok()?;
            Some(out)
        }
        _ => None,
    }
}

/// One folder listing's fingerprint: (file name, byte size,
/// modification time, content hash) per entry, sorted by
/// name. The content hash is computed for font files only
/// (the files a scan actually reads facts from): a
/// replacement that preserves a font's size and modification
/// time — `cp -p`, `touch -r`, an archive extraction with
/// stored timestamps — still changes the hash, so the memo
/// can never serve the replaced font's stale facts (round
/// 30, F2). Hashing reads the font bytes but never parses
/// them; a repeat scan pays one read + hash per font and
/// still skips every parse, which is where a scan's cost
/// lives. Non-font entries carry no hash: no fact the scan
/// serves comes from them, so their metadata is identity
/// enough, as before.
type DirFingerprint = Vec<(String, u64, Option<std::time::SystemTime>, Option<String>)>;

fn dir_fingerprint(dir: &Path) -> DirFingerprint {
    let mut out: DirFingerprint = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let md = entry.metadata().ok();
            let name = entry.file_name().to_string_lossy().into_owned();
            let ext = Path::new(&name)
                .extension()
                .map(|e| e.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            let content_hash = if ext == "ttf" || ext == "otf" {
                std::fs::read(entry.path())
                    .ok()
                    .map(|bytes| ob_model::sha256_hex(&bytes))
            } else {
                None
            };
            out.push((
                name,
                md.as_ref().map(|m| m.len()).unwrap_or(0),
                md.and_then(|m| m.modified().ok()),
                content_hash,
            ));
        }
    }
    out.sort();
    out
}

/// The scan memo's store: one (fingerprint, facts) entry
/// per library folder.
type ScanMemo = std::collections::HashMap<PathBuf, (DirFingerprint, Vec<FontInfo>)>;

/// The scan memo (see `FontLibrary::scan`): one entry per
/// folder, replaced whenever that folder's fingerprint
/// changes. Process-wide, because call sites build a fresh
/// `FontLibrary` for the same folder on nearly every call —
/// a per-instance memo would never be hit twice.
fn scan_memo() -> &'static std::sync::Mutex<ScanMemo> {
    static MEMO: std::sync::OnceLock<std::sync::Mutex<ScanMemo>> = std::sync::OnceLock::new();
    MEMO.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
}

fn scan_memo_get(dir: &Path, fingerprint: &DirFingerprint) -> Option<Vec<FontInfo>> {
    let guard = scan_memo().lock().ok()?;
    let (stored_fp, infos) = guard.get(dir)?;
    if stored_fp == fingerprint {
        Some(infos.clone())
    } else {
        None
    }
}

fn scan_memo_put(dir: &Path, fingerprint: DirFingerprint, infos: Vec<FontInfo>) {
    if let Ok(mut guard) = scan_memo().lock() {
        guard.insert(dir.to_path_buf(), (fingerprint, infos));
    }
}

pub struct FontLibrary {
    dir: PathBuf,
}

impl FontLibrary {
    /// Open (creating if needed) the library at `dir`.
    pub fn open(dir: PathBuf) -> Result<FontLibrary, String> {
        std::fs::create_dir_all(&dir)
            .map_err(|e| format!("cannot create font library folder: {e}"))?;
        Ok(FontLibrary { dir })
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Every readable font in the folder, sorted by family then face.
    ///
    /// The scan itself (read, parse, and SHA-256 every file) is
    /// memoized per folder on a fingerprint of file names,
    /// sizes, modification times, and — for font files — a
    /// SHA-256 of the contents (health audit Efficiency
    /// MINOR-1: a scan runs ≈0.5 ms per font and is called
    /// from at least seven app sites, several times per
    /// user action). A folder whose fingerprint is unchanged
    /// since its last scan gets the memoized facts back; any
    /// file added, removed, or resized changes the
    /// fingerprint, and so does a font replaced in place with
    /// its size and modification time preserved, because its
    /// content hash changes (round 30, F2). The fingerprint's
    /// hashing reads each font's bytes on every scan but
    /// never parses them, so a repeat scan still skips the
    /// parse cost the memo exists to save. A changed folder
    /// is scanned again in full, so newly added fonts appear
    /// on the very next scan, exactly as before the memo.
    pub fn scan(&self) -> Vec<FontInfo> {
        let fingerprint = dir_fingerprint(&self.dir);
        if let Some(hit) = scan_memo_get(&self.dir, &fingerprint) {
            return hit;
        }
        let out = self.scan_uncached();
        scan_memo_put(&self.dir, fingerprint, out.clone());
        out
    }

    /// The scan itself, with no memo in the way.
    fn scan_uncached(&self) -> Vec<FontInfo> {
        let mut out = Vec::new();
        let Ok(entries) = std::fs::read_dir(&self.dir) else {
            return out;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let ext = path
                .extension()
                .map(|e| e.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            if ext != "ttf" && ext != "otf" {
                continue;
            }
            if let Ok(bytes) = std::fs::read(&path) {
                let file_name = entry.file_name().to_string_lossy().to_string();
                if let Ok(info) = inspect(&bytes, &file_name) {
                    out.push(info);
                }
            }
        }
        out.sort_by(|a, b| a.family.cmp(&b.family).then(a.full_name.cmp(&b.full_name)));
        out
    }

    /// Copy a font into the library. Identical bytes already present
    /// are returned as-is (no duplicates). Formats we cannot
    /// use are refused in plain words (see `format_refusal`).
    /// A font collection (.ttc) is split and every face is
    /// added; the first face's facts are returned, so callers
    /// that need every face's outcome (the Fonts doors) use
    /// `add_path` or `add_collection_bytes` instead.
    pub fn add_bytes(&self, bytes: &[u8], suggested_name: &str) -> Result<FontInfo, String> {
        let clean = sanitize_file_name(suggested_name);
        if let Some(msg) = format_refusal(bytes, &clean) {
            return Err(msg);
        }
        if ttc::is_collection(bytes) {
            let mut first_err: Option<String> = None;
            for result in self.add_collection_bytes(bytes, &clean) {
                match result {
                    Ok(info) => return Ok(info),
                    Err(e) => {
                        if first_err.is_none() {
                            first_err = Some(e);
                        }
                    }
                }
            }
            return Err(first_err.unwrap_or_else(|| "that collection held no fonts".into()));
        }
        self.add_single(bytes, suggested_name)
    }

    /// Add one single-face font (the funnel `add_bytes`
    /// ends at once the format questions are answered).
    fn add_single(&self, bytes: &[u8], suggested_name: &str) -> Result<FontInfo, String> {
        let mut known: std::collections::HashSet<String> =
            self.scan().iter().map(|f| f.sha256.clone()).collect();
        self.add_bytes_known(bytes, suggested_name, &mut known)
    }

    /// Split a font collection and add every face as its
    /// own font file, one result per face: a collection is
    /// several fonts that happen to share a file, and the
    /// library keeps one file per face (docs/FONTS.md).
    pub fn add_collection_bytes(
        &self,
        bytes: &[u8],
        suggested_name: &str,
    ) -> Vec<Result<FontInfo, String>> {
        let clean = sanitize_file_name(suggested_name);
        let stem = clean
            .rsplit_once('.')
            .map(|(s, _)| s.to_string())
            .unwrap_or_else(|| clean.clone());
        let faces = match ttc::split_collection(bytes) {
            Ok(faces) => faces,
            Err(e) => return vec![Err(format!("{clean}: {e}"))],
        };
        let mut known: std::collections::HashSet<String> =
            self.scan().iter().map(|f| f.sha256.clone()).collect();
        faces
            .iter()
            .enumerate()
            .map(|(i, face)| {
                let name = collection_face_name(face, &stem, i);
                self.add_bytes_known(face, &name, &mut known)
            })
            .collect()
    }

    /// add_bytes with the library's known hashes already collected:
    /// importing a whole zip used to re-scan (re-read, re-parse,
    /// re-hash) the entire library for every font, which made big
    /// packs quadratic (closing audit). One scan per batch now.
    /// Names are sanitized to a bare file name on every platform:
    /// an archive entry like ..\..\evil.ttf must never write
    /// outside the library (closing audit, Windows traversal).
    fn add_bytes_known(
        &self,
        bytes: &[u8],
        suggested_name: &str,
        known: &mut std::collections::HashSet<String>,
    ) -> Result<FontInfo, String> {
        const MAX_FONT_BYTES: usize = 64 * 1024 * 1024;
        if bytes.len() > MAX_FONT_BYTES {
            return Err("that font file is larger than 64 MB; real fonts are not".into());
        }
        let clean = sanitize_file_name(suggested_name);
        let mut info = inspect(bytes, &clean)?;
        if known.contains(&info.sha256) {
            // Return the existing entry's facts when we can find them.
            if let Some(existing) = self.scan().into_iter().find(|f| f.sha256 == info.sha256) {
                return Ok(existing);
            }
            return Ok(info);
        }
        // A DIFFERENT font already owns this file name: never
        // overwrite it (a bulletin pinned to the old font would
        // silently change face). Write under a hashed name.
        let target = self.dir.join(&clean);
        let final_name = if target.exists() {
            let hashed = self.free_hashed_name(&clean, &info.sha256, bytes);
            info.file_name = hashed.clone();
            hashed
        } else {
            clean
        };
        std::fs::write(self.dir.join(&final_name), bytes)
            .map_err(|e| format!("cannot save font into the library: {e}"))?;
        known.insert(info.sha256.clone());
        Ok(info)
    }

    /// The hashed name an incoming font lands under when a
    /// different font owns its clean name, checked for
    /// occupancy (round 32, F7): the first hashed name can
    /// itself already stand in the library — planted by
    /// hand, or left by an earlier import — and the old
    /// code wrote over whatever stood there, so a font
    /// seeded at `plant.ttf-<hash of the incoming font>`
    /// silently became the incoming font's bytes. A
    /// candidate owned by DIFFERENT bytes is never
    /// overwritten: the name disambiguates further, first
    /// with a longer prefix of the incoming font's own
    /// hash (8, then 16, then 32, then the whole hash),
    /// then with a counter past the whole hash. A
    /// candidate owned by the SAME bytes is the incoming
    /// font already standing under that name, and it
    /// lands there as it always has.
    fn free_hashed_name(&self, clean: &str, sha256: &str, bytes: &[u8]) -> String {
        let mut candidates: Vec<String> = [8usize, 16, 32, 64]
            .iter()
            .map(|take| {
                let prefix: String = sha256.chars().take(*take).collect();
                format!("{clean}-{prefix}")
            })
            .collect();
        let mut n = 2;
        loop {
            if candidates.is_empty() {
                candidates.push(format!("{clean}-{sha256}-{n}"));
                n += 1;
            }
            let candidate = candidates.remove(0);
            match std::fs::read(self.dir.join(&candidate)) {
                Ok(existing) if existing == bytes => return candidate,
                Ok(_) => continue,
                Err(_) => return candidate,
            }
        }
    }

    /// Add a font from a file on disk (a download, a USB stick, a
    /// system font being copied in): one result per font the
    /// file carries. A plain font gives one result; a
    /// collection gives one per face; a format we cannot use
    /// gives one refusal naming the file.
    pub fn add_path(&self, path: &std::path::Path) -> Vec<Result<FontInfo, String>> {
        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(e) => return vec![Err(format!("cannot read {}: {e}", path.display()))],
        };
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("font.ttf");
        let clean = sanitize_file_name(name);
        if let Some(msg) = format_refusal(&bytes, &clean) {
            return vec![Err(msg)];
        }
        if ttc::is_collection(&bytes) {
            return self.add_collection_bytes(&bytes, &clean);
        }
        vec![self.add_single(&bytes, name)]
    }

    /// Land one license text in the library folder
    /// under `file_name` (round 32, F6): the same
    /// discipline as the fonts themselves. A name
    /// already owned by the SAME text is an exact
    /// repeat and collapses; a name owned by a
    /// DIFFERENT text is never overwritten — the
    /// incoming text lands under the hashed name (the
    /// font door's own idiom), and under a counted
    /// name past that when the hashed name is also
    /// owned by different words.
    fn land_license_text(&self, file_name: &str, content: &[u8]) {
        let target = self.dir.join(file_name);
        match std::fs::read(&target) {
            Ok(existing) if existing == content => return,
            Ok(_) => {}
            Err(_) => {
                let _ = std::fs::write(&target, content);
                return;
            }
        }
        let short: String = ob_model::sha256_hex(content).chars().take(8).collect();
        let mut candidate = format!("{file_name}-{short}");
        let mut n = 2;
        loop {
            let path = self.dir.join(&candidate);
            match std::fs::read(&path) {
                Ok(existing) if existing == content => return,
                Ok(_) => {
                    candidate = format!("{file_name}-{short}-{n}");
                    n += 1;
                }
                Err(_) => {
                    let _ = std::fs::write(&path, content);
                    return;
                }
            }
        }
    }

    /// Add every font inside a zip archive (how font sites ship
    /// packs). License text files in the archive are copied into
    /// the library folder beside the fonts. Returns one result per
    /// font file found; a bad file never sinks the rest.
    pub fn add_zip_bytes(&self, bytes: &[u8]) -> Vec<Result<FontInfo, String>> {
        // Caps against hostile or broken archives (closing
        // audit): a small zip must not decompress to gigabytes,
        // and a pack means dozens of fonts, not thousands.
        const MAX_ENTRIES: usize = 512;
        const MAX_ENTRY_BYTES: u64 = 64 * 1024 * 1024;
        const MAX_TOTAL_BYTES: u64 = 256 * 1024 * 1024;
        let mut results = Vec::new();
        let cursor = std::io::Cursor::new(bytes.to_vec());
        let Ok(mut archive) = zip::ZipArchive::new(cursor) else {
            return vec![Err("that file is not a readable zip archive".to_string())];
        };
        let mut known: std::collections::HashSet<String> =
            self.scan().iter().map(|f| f.sha256.clone()).collect();
        // The pack's entries, in archive order, collected
        // off the raw central directory (central_spots)
        // so EVERY copy of a duplicated name arrives
        // (round 32, F6): the reader's name-keyed map
        // collapses a duplicated name to its last copy
        // before this door's own defenses run, so two
        // different fonts under one name used to land as
        // only the last, the first silently absent. Fed
        // one by one through the door's existing funnel
        // below, the copies land the way the door lands
        // any two fonts that share a name — the first
        // under the clean name, the next under its
        // hashed name — and an exact repeat collapses on
        // its hash, as always. A file that fails to read
        // back, or runs past the per-entry cap, is passed
        // by and the pack goes on without it (this door's
        // judgment is Skip, health audit E-9). Only when
        // no raw directory stands does the collection
        // fall back to the reader's own enumeration.
        let spots = central_spots(bytes);
        let (raw_count, entries): (usize, Vec<(String, Vec<u8>)>) = if !spots.is_empty() {
            let entries = spots
                .iter()
                .take(MAX_ENTRIES)
                .filter_map(|(name, spot)| {
                    if name.ends_with('/') || name.ends_with('\\') {
                        return None;
                    }
                    let content = raw_payload(bytes, spot, MAX_ENTRY_BYTES)?;
                    if content.len() as u64 > MAX_ENTRY_BYTES || crc32_ieee(&content) != spot.crc32
                    {
                        return None;
                    }
                    Some((name.clone(), content))
                })
                .collect();
            (spots.len(), entries)
        } else {
            let mut entries = Vec::new();
            for i in 0..archive.len().min(MAX_ENTRIES) {
                let Ok(mut entry) = archive.by_index(i) else {
                    continue;
                };
                let name = entry.name().to_string();
                let read = match ob_model::read_capped(
                    &mut entry,
                    MAX_ENTRY_BYTES,
                    ob_model::CapOverflow::Skip,
                ) {
                    Ok(read) if !read.overflowed => read,
                    _ => continue,
                };
                entries.push((name, read.bytes));
            }
            (archive.len(), entries)
        };
        let mut total: u64 = 0;
        for (name, content) in &entries {
            let lower = name.to_lowercase();
            let file_name = sanitize_file_name(name);
            total += content.len() as u64;
            if total > MAX_TOTAL_BYTES {
                results.push(Err(
                    "that archive holds more than 256 MB of files; stopping there".to_string(),
                ));
                break;
            }
            if lower.ends_with(".ttf") || lower.ends_with(".otf") {
                // A web font wearing a .ttf name is still a
                // web font: the refusal names it, the pack
                // goes on without it.
                match format_refusal(content, &file_name) {
                    Some(msg) => results.push(Err(msg)),
                    None => results.push(self.add_bytes_known(content, &file_name, &mut known)),
                }
            } else if lower.ends_with(".ttc") {
                // A collection inside the pack: split it,
                // one result per face, like the file door.
                let stem = file_name
                    .rsplit_once('.')
                    .map(|(s, _)| s.to_string())
                    .unwrap_or_else(|| file_name.clone());
                match ttc::split_collection(content) {
                    Ok(faces) => {
                        for (i, face) in faces.iter().enumerate() {
                            let name = collection_face_name(face, &stem, i);
                            results.push(self.add_bytes_known(face, &name, &mut known));
                        }
                    }
                    Err(e) => results.push(Err(format!("{file_name}: {e}"))),
                }
            } else if lower.ends_with(".woff")
                || lower.ends_with(".woff2")
                || lower.ends_with(".pfb")
                || lower.ends_with(".afm")
                || lower.ends_with(".pfm")
            {
                // Named and refused, never silently skipped:
                // the pack's result list is the count.
                if let Some(msg) = format_refusal(content, &file_name) {
                    results.push(Err(msg));
                }
            } else if (lower.ends_with(".txt") || lower.ends_with(".md"))
                && (lower.contains("license") || lower.contains("ofl") || lower.contains("copying"))
            {
                // License texts get the fonts' own
                // treatment (round 32, F6): a repeated
                // license name never silently collapses
                // to the last text.
                self.land_license_text(&file_name, content);
            }
        }
        if raw_count > MAX_ENTRIES {
            // Entries past the cap are counted and named,
            // never silently dropped (round 3, RT3-F): the
            // result list is what both the Fonts window and
            // the CLI read out, so the line lands at both
            // doors with no extra wiring.
            results.push(Err(format!(
                "{} more files were in the pack than can be taken at once; \
                 add them in a second pass.",
                raw_count - MAX_ENTRIES
            )));
        }
        if results.is_empty() {
            results.push(Err(
                "no font files (.ttf, .otf, or .ttc) inside that zip".to_string()
            ));
        }
        results
    }

    /// The library folder itself (for cheap state fingerprints).
    pub fn dir_path(&self) -> &std::path::Path {
        &self.dir
    }

    /// The bytes of one library font, for handing to the text engine.
    pub fn read(&self, info: &FontInfo) -> Result<Vec<u8>, String> {
        std::fs::read(self.dir.join(&info.file_name))
            .map_err(|e| format!("cannot read font {}: {e}", info.file_name))
    }
}

/// One font found on this computer, outside the library: its facts
/// and where the file lives. The panel offers these with an
/// "add a copy" button; adding copies the file into the library
/// (copy-in, never a live link: docs/FONTS.md).
#[derive(Debug, Clone)]
pub struct SystemFont {
    pub info: FontInfo,
    pub path: std::path::PathBuf,
}

/// The folders this operating system keeps fonts in.
fn system_font_dirs() -> Vec<std::path::PathBuf> {
    let mut dirs: Vec<std::path::PathBuf> = Vec::new();
    let home = std::env::var_os("HOME").map(std::path::PathBuf::from);
    if cfg!(target_os = "windows") {
        let windir = std::env::var_os("WINDIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::path::PathBuf::from(r"C:\Windows"));
        dirs.push(windir.join("Fonts"));
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            dirs.push(
                std::path::PathBuf::from(local)
                    .join("Microsoft")
                    .join("Windows")
                    .join("Fonts"),
            );
        }
    } else if cfg!(target_os = "macos") {
        dirs.push(std::path::PathBuf::from("/System/Library/Fonts"));
        dirs.push(std::path::PathBuf::from("/Library/Fonts"));
        if let Some(h) = &home {
            dirs.push(h.join("Library").join("Fonts"));
        }
    } else {
        dirs.push(std::path::PathBuf::from("/usr/share/fonts"));
        dirs.push(std::path::PathBuf::from("/usr/local/share/fonts"));
        if let Some(h) = &home {
            dirs.push(h.join(".local").join("share").join("fonts"));
            dirs.push(h.join(".fonts"));
        }
    }
    dirs
}

/// Every font file on this computer (recursive, .ttf and .otf),
/// deduplicated by hash. Font collections (.ttc) are not listed
/// here (each row is one face, and a collection is several);
/// adding a collection through the Fonts panel's file pick
/// splits it into its faces instead.
///
/// The enumeration is memoized for the process (health audit
/// Efficiency MINOR-1): it walks every system font folder and
/// reads, parses, and hashes every font file it finds, which
/// costs real time on a font-rich machine, and nothing about
/// the answer changes while the program runs except fonts
/// installed mid-session — those appear after a restart.
/// That is the same staleness bargain the SVG renderer's
/// system font database already takes (ob-image's fontdb is
/// loaded once per process). The user's own Font Library is
/// NOT part of this snapshot: `FontLibrary::scan` re-checks
/// its folder's fingerprint on every call, so a font the
/// user adds to the library appears immediately.
pub fn system_fonts() -> Vec<SystemFont> {
    static SYSTEM_FONTS: std::sync::OnceLock<Vec<SystemFont>> = std::sync::OnceLock::new();
    SYSTEM_FONTS.get_or_init(enumerate_system_fonts).clone()
}

/// The system enumeration itself, with no memo in the way.
fn enumerate_system_fonts() -> Vec<SystemFont> {
    fn walk(
        dir: &std::path::Path,
        out: &mut Vec<SystemFont>,
        seen: &mut std::collections::HashSet<String>,
    ) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out, seen);
                continue;
            }
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            if ext != "ttf" && ext != "otf" {
                continue;
            }
            let Ok(bytes) = std::fs::read(&path) else {
                continue;
            };
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("font")
                .to_string();
            if let Ok(info) = inspect(&bytes, &name) {
                if seen.insert(info.sha256.clone()) {
                    out.push(SystemFont { info, path });
                }
            }
        }
    }
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for dir in system_font_dirs() {
        walk(&dir, &mut out, &mut seen);
    }
    out.sort_by(|a, b| {
        a.info
            .family
            .cmp(&b.info.family)
            .then(a.info.bold.cmp(&b.info.bold))
            .then(a.info.italic.cmp(&b.info.italic))
    });
    out
}

/// THE face-pick rule, shared by the pin below and the
/// PDF writer's face sources (round 3, R3-13): given one
/// family's faces in a stable order, each as (bold,
/// italic), pick the face for a wanted (bold, italic):
/// an exact flag match, the LAST such face winning, else
/// the family's plain face (the last of those), else
/// None. One rule in one place, so the file the pin names
/// and the face the writer embeds can never be two
/// different faces of one family.
pub fn pick_face(candidates: &[(bool, bool)], bold: bool, italic: bool) -> Option<usize> {
    candidates
        .iter()
        .rposition(|&(b, i)| b == bold && i == italic)
        .or_else(|| candidates.iter().rposition(|&(b, i)| !b && !i))
}

/// THE candidate faces, shared by the document font pin
/// and the PDF writer (round 4, R4-9): the bundled faces
/// first, then the library's, and only faces whose
/// embedding rights let them travel inside a file
/// (may_package) are candidates at all. One winner per
/// (family, bold, italic) key, picked by pick_face, so
/// the library's face wins over a bundled face of the
/// same key, the writer's long-standing order. The pin
/// used to scan the library unfiltered with no bundled
/// faces at all, so it could name a file the writer
/// would never embed, and miss the bundled face the
/// writer would; one enumeration in one place ends
/// that drift. Callers pass the bundled faces as facts
/// (ob_text::shaping::bundled_font_infos) because the
/// bundled files live with the shaper.
pub fn candidate_faces(bundled: &[FontInfo], library: &[FontInfo]) -> Vec<FontInfo> {
    let mut groups: Vec<((String, bool, bool), Vec<FontInfo>)> = Vec::new();
    for info in bundled.iter().chain(library.iter()) {
        if !info.rights.may_package() {
            continue;
        }
        let key = (info.family.clone(), info.bold, info.italic);
        match groups.iter_mut().find(|(k, _)| *k == key) {
            Some((_, group)) => group.push(info.clone()),
            None => groups.push((key, vec![info.clone()])),
        }
    }
    groups
        .into_iter()
        .filter_map(|((_, bold, italic), group)| {
            let flags: Vec<(bool, bool)> = group.iter().map(|f| (f.bold, f.italic)).collect();
            pick_face(&flags, bold, italic).map(|idx| group[idx].clone())
        })
        .collect()
}

/// Record, on each of the document's styles, the SHA-256 of the
/// exact font file that style will be laid out with
/// (docs/FONTS.md). Run before saving or exporting, so the file
/// remembers which font bytes its layout depends on. The family
/// a style names is walked through the resolution chain first:
/// a style made in a font the library does not hold is laid out
/// with the chain's stand-in, so the pin names the stand-in's
/// file, the one the layout truly used. When the real font is
/// added later, the next save re-pins to it by itself. The face
/// within the family is picked by pick_face, the shared rule:
/// an italic style pins the italic face, bold-italic the
/// bold-italic face. The faces themselves come from
/// candidate_faces, the same enumeration the PDF writer
/// draws from (round 4, R4-9): bundled faces plus the
/// library's, rights-filtered, so the pin names a face
/// that would actually print. When no face can be picked
/// (the resolved family has no candidate at all), the
/// pin is CLEARED, never left stale from an earlier save.
pub fn pin_document_fonts(doc: &mut ob_model::Document, lib: &FontLibrary, bundled: &[FontInfo]) {
    let fonts = candidate_faces(bundled, &lib.scan());
    let families: Vec<String> = {
        let mut out: Vec<String> = Vec::new();
        for f in &fonts {
            if !out.contains(&f.family) {
                out.push(f.family.clone());
            }
        }
        out
    };
    for style in &mut doc.styles {
        let resolved = resolve::resolve_family(&style.font_family, &families).family;
        let family_faces: Vec<&FontInfo> = fonts.iter().filter(|f| f.family == resolved).collect();
        let flags: Vec<(bool, bool)> = family_faces.iter().map(|f| (f.bold, f.italic)).collect();
        match pick_face(&flags, style.bold, style.italic) {
            Some(idx) => style.font_sha256 = Some(family_faces[idx].sha256.clone()),
            None => style.font_sha256 = None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bundled(name: &str) -> Vec<u8> {
        std::fs::read(ob_text::bundled_asset_path(name)).expect("bundled font present")
    }

    /// The bundled faces as inspected facts, the way
    /// ob_text::shaping::bundled_font_infos hands them to
    /// the pin in the app.
    fn bundled_infos() -> Vec<FontInfo> {
        [
            "LiberationSans-Regular.ttf",
            "LiberationSans-Bold.ttf",
            "LiberationSans-Italic.ttf",
            "LiberationSans-BoldItalic.ttf",
            "LiberationSerif-Regular.ttf",
            "LiberationSerif-Bold.ttf",
            "LiberationSerif-Italic.ttf",
            "LiberationSerif-BoldItalic.ttf",
        ]
        .iter()
        .map(|name| inspect(&bundled(name), name).expect("inspect bundled face"))
        .collect()
    }

    /// A fabricated face fact: candidate_faces reads
    /// facts, not font bytes, so the rule is testable
    /// without a font file for every rights class.
    fn fake_info(family: &str, bold: bool, rights: EmbeddingRights, sha: &str) -> FontInfo {
        FontInfo {
            family: family.into(),
            full_name: family.into(),
            bold,
            italic: false,
            rights,
            no_subsetting: false,
            file_name: "fake.ttf".into(),
            sha256: sha.into(),
            bytes: 0,
        }
    }

    #[test]
    fn bundled_liberation_faces_inspect_cleanly() {
        let regular = inspect(
            &bundled("LiberationSans-Regular.ttf"),
            "LiberationSans-Regular.ttf",
        )
        .expect("inspect sans regular");
        assert_eq!(regular.family, "Liberation Sans");
        assert!(!regular.bold);
        let bold = inspect(
            &bundled("LiberationSans-Bold.ttf"),
            "LiberationSans-Bold.ttf",
        )
        .expect("inspect sans bold");
        assert!(bold.bold);
        let serif = inspect(
            &bundled("LiberationSerif-Regular.ttf"),
            "LiberationSerif-Regular.ttf",
        )
        .expect("inspect serif");
        assert_eq!(serif.family, "Liberation Serif");
        // Liberation ships with installable embedding (fsType 0).
        assert_eq!(regular.rights, EmbeddingRights::Installable);
        assert!(regular.rights.may_package());
        assert!(!regular.rights.plain_words().is_empty());
    }

    fn temp_dir(tag: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("ob-fonts-test-{tag}-{}", std::process::id()))
    }

    #[test]
    fn names_are_sanitized_on_every_platform() {
        assert_eq!(sanitize_file_name("../../evil.ttf"), "evil.ttf");
        assert_eq!(sanitize_file_name("..\\..\\evil.ttf"), "evil.ttf");
        assert_eq!(sanitize_file_name("a/b\\c.ttf"), "c.ttf");
        assert_eq!(sanitize_file_name(".."), "font.ttf");
        assert_eq!(
            sanitize_file_name("Carlito-Regular.ttf"),
            "Carlito-Regular.ttf"
        );
    }

    #[test]
    fn zip_traversal_names_stay_in_the_library() {
        let dir = temp_dir("traversal");
        let lib = FontLibrary::open(dir.clone()).expect("open");
        let font = bundled("LiberationSans-Regular.ttf");
        // Build a zip whose entry name tries to escape.
        let mut zip_bytes = Vec::new();
        {
            let cursor = std::io::Cursor::new(&mut zip_bytes);
            let mut zw = zip::ZipWriter::new(cursor);
            let opts = zip::write::SimpleFileOptions::default();
            zw.start_file("..\\..\\escaped.ttf", opts).expect("start");
            std::io::Write::write_all(&mut zw, &font).expect("write font");
            zw.finish().expect("finish zip");
        }
        let results = lib.add_zip_bytes(&zip_bytes);
        assert!(
            results.iter().any(|r| r.is_ok()),
            "the font itself is fine: {results:?}"
        );
        assert!(
            !dir.parent().expect("parent").join("escaped.ttf").exists()
                && !std::path::Path::new("escaped.ttf").exists(),
            "nothing may be written outside the library"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_second_font_never_overwrites_a_namesake() {
        let dir = temp_dir("collision");
        let lib = FontLibrary::open(dir.clone()).expect("open");
        let a = bundled("LiberationSans-Regular.ttf");
        let b = bundled("LiberationSerif-Regular.ttf");
        let first = lib.add_bytes(&a, "Same.ttf").expect("add a");
        let second = lib.add_bytes(&b, "Same.ttf").expect("add b");
        assert_ne!(first.file_name, second.file_name);
        // Both still read back as themselves.
        assert_eq!(lib.read(&first).expect("read a"), a);
        assert_eq!(lib.read(&second).expect("read b"), b);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn restricted_fonts_may_not_be_packaged() {
        // Patch the fsType of a real font to Restricted (0x0002)
        // in its OS/2 table and confirm the rights gate closes.
        let mut bytes = bundled("LiberationSans-Regular.ttf");
        let num_tables = u16::from_be_bytes([bytes[4], bytes[5]]) as usize;
        let mut patched = false;
        for i in 0..num_tables {
            let rec = 12 + i * 16;
            if &bytes[rec..rec + 4] == b"OS/2" {
                let off = u32::from_be_bytes([
                    bytes[rec + 8],
                    bytes[rec + 9],
                    bytes[rec + 10],
                    bytes[rec + 11],
                ]) as usize;
                // fsType is the uint16 at OS/2 + 8.
                bytes[off + 8] = 0x00;
                bytes[off + 9] = 0x02;
                patched = true;
            }
        }
        assert!(patched, "the test font has an OS/2 table");
        let info = inspect(&bytes, "Restricted.ttf").expect("inspect");
        assert!(
            !info.rights.may_package(),
            "restricted fonts must not package"
        );
        assert!(
            info.rights.plain_words().to_lowercase().contains("cannot")
                || info.rights.plain_words().to_lowercase().contains("not"),
            "the refusal speaks plainly: {}",
            info.rights.plain_words()
        );
    }

    #[test]
    fn library_add_scan_dedup_round_trip() {
        let dir = std::env::temp_dir().join(format!("ob-fonts-test-{}", std::process::id()));
        let lib = FontLibrary::open(dir.clone()).expect("open library");
        let bytes = bundled("LiberationSans-Regular.ttf");
        let info = lib
            .add_bytes(&bytes, "LiberationSans-Regular.ttf")
            .expect("add font");
        let again = lib
            .add_bytes(&bytes, "LiberationSans-Regular.ttf")
            .expect("add same font again");
        assert_eq!(info.sha256, again.sha256);
        let scanned = lib.scan();
        assert_eq!(
            scanned.len(),
            1,
            "duplicate bytes must not duplicate the font"
        );
        assert_eq!(lib.read(&scanned[0]).expect("read back").len(), info.bytes);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn scan_never_serves_stale_facts_after_a_preserved_stamp_replacement() {
        // Round 30, F2 (review MINOR-1 / red team RT30-3):
        // the scan memo used to key on (name, size, mtime)
        // only, so a font replaced in place at the same
        // length with its modification time restored was
        // served its predecessor's family facts and SHA-256
        // for the rest of the process, while style pinning
        // and face lookup keyed on those stale facts. The
        // fingerprint now carries a content hash, so this
        // witness must read as the new font.
        let dir = temp_dir("stale-memo");
        let lib = FontLibrary::open(dir.clone()).expect("open");
        let original = bundled("LiberationSans-Regular.ttf");
        let path = dir.join("Witness.ttf");
        std::fs::write(&path, &original).expect("write font");
        let stamp = std::fs::metadata(&path)
            .expect("metadata")
            .modified()
            .expect("modification time");
        let first = lib.scan();
        assert_eq!(first.len(), 1, "one font scanned: {first:?}");
        assert_eq!(
            first[0].sha256,
            inspect(&original, "Witness.ttf")
                .expect("inspect original")
                .sha256
        );

        // Flip ONE byte inside the font's name table: the
        // 'L' of a UTF-16BE "Li…" name string becomes 'M'.
        // The name table's place is read from the font's own
        // table directory, so the flip is at a real
        // name-table offset, not at the file head.
        let num_tables = u16::from_be_bytes([original[4], original[5]]) as usize;
        let mut name_range: Option<(usize, usize)> = None;
        for i in 0..num_tables {
            let rec = 12 + i * 16;
            if &original[rec..rec + 4] == b"name" {
                let off = u32::from_be_bytes([
                    original[rec + 8],
                    original[rec + 9],
                    original[rec + 10],
                    original[rec + 11],
                ]) as usize;
                let len = u32::from_be_bytes([
                    original[rec + 12],
                    original[rec + 13],
                    original[rec + 14],
                    original[rec + 15],
                ]) as usize;
                name_range = Some((off, off + len));
            }
        }
        let (table_start, table_end) = name_range.expect("the font has a name table");
        let needle = [0u8, b'L', 0u8, b'i'];
        let hit = (table_start..table_end - needle.len())
            .find(|&at| original[at..at + needle.len()] == needle)
            .expect("a UTF-16BE name string stands in the name table");
        let mut replaced = original.clone();
        replaced[hit + 1] = b'M';
        assert_eq!(replaced.len(), original.len(), "same length");
        assert_ne!(replaced, original, "one byte flipped");

        std::fs::write(&path, &replaced).expect("replace font in place");
        let file = std::fs::File::options()
            .write(true)
            .open(&path)
            .expect("open replaced font");
        file.set_modified(stamp).expect("restore the stamp");
        drop(file);
        assert_eq!(
            std::fs::metadata(&path)
                .expect("metadata")
                .modified()
                .expect("modification time"),
            stamp,
            "size and stamp both preserved: the old memo key would hit"
        );

        let fresh = inspect(&replaced, "Witness.ttf").expect("inspect replaced");
        assert_ne!(fresh.sha256, first[0].sha256, "the bytes really changed");
        let second = lib.scan();
        assert_eq!(second.len(), 1, "one font scanned: {second:?}");
        assert_eq!(
            second[0].sha256, fresh.sha256,
            "the second scan must serve the new font's hash, not the stale one"
        );
        assert_eq!(second[0].family, fresh.family);
        assert_eq!(second[0].full_name, fresh.full_name);
        // The corrected facts are what the memo now serves.
        assert_eq!(lib.scan(), second);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn garbage_is_refused() {
        assert!(inspect(b"this is not a font", "junk.ttf").is_err());
    }

    #[test]
    fn zip_import_takes_fonts_and_license() {
        use std::io::Write as _;
        let mut buf = Vec::new();
        {
            let mut zw = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
            let opts = zip::write::SimpleFileOptions::default();
            zw.start_file("Lato-Regular.ttf", opts).expect("zip entry");
            zw.write_all(&bundled("LiberationSans-Regular.ttf"))
                .expect("zip write");
            zw.start_file("Lato-Bold.ttf", opts).expect("zip entry");
            zw.write_all(&bundled("LiberationSans-Bold.ttf"))
                .expect("zip write");
            zw.start_file("OFL.txt", opts).expect("zip entry");
            zw.write_all(b"SIL Open Font License").expect("zip write");
            zw.finish().expect("zip finish");
        }
        let dir = std::env::temp_dir().join(format!("ob-fonts-zip-{}", std::process::id()));
        let lib = FontLibrary::open(dir.clone()).expect("open library");
        let results = lib.add_zip_bytes(&buf);
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.is_ok()));
        assert!(dir.join("OFL.txt").exists(), "license text kept");
        std::fs::remove_dir_all(&dir).ok();
    }

    fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        haystack.windows(needle.len()).position(|w| w == needle)
    }

    /// Rename one entry everywhere its name is written
    /// (the local header and the central directory),
    /// the two names the same length: the zip writer
    /// refuses a duplicate name outright, so a pack
    /// carrying a repeated entry is assembled by
    /// patch, the same helper shape as the sibling
    /// doors' tests.
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

    fn pack_zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
        use std::io::Write as _;
        let mut buf = Vec::new();
        {
            let mut zw = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
            let opts = zip::write::SimpleFileOptions::default();
            for (name, body) in entries {
                zw.start_file(*name, opts).expect("zip entry");
                zw.write_all(body).expect("zip write");
            }
            zw.finish().expect("zip finish");
        }
        buf
    }

    #[test]
    fn a_pack_with_two_fonts_under_one_name_lands_both() {
        // Round 32, F6 (red team RT32-4, witness
        // f03-dup-font-name.zip): two different fonts
        // under one name — the reader's name-keyed map
        // collapsed the pair to the last copy before
        // the door's hashed-rename defense could run,
        // so only the last font landed and the first
        // was silently absent. Every copy arrives now:
        // the first lands under the clean name, the
        // second under its hashed name, each readable
        // back as itself.
        let regular = bundled("LiberationSans-Regular.ttf");
        let bold = bundled("LiberationSans-Bold.ttf");
        let mut buf = pack_zip(&[("MyFont.ttf", &regular), ("MyFont.ttX", &bold)]);
        patch_entry_name(&mut buf, "MyFont.ttX", "MyFont.ttf");
        let dir = temp_dir("dup-font-name");
        let lib = FontLibrary::open(dir.clone()).expect("open");
        let results = lib.add_zip_bytes(&buf);
        assert_eq!(results.len(), 2, "one result per copy: {results:?}");
        let first = results[0].as_ref().expect("the first copy lands");
        let second = results[1].as_ref().expect("the second copy lands");
        assert_eq!(first.file_name, "MyFont.ttf");
        assert_eq!(
            first.sha256,
            inspect(&regular, "MyFont.ttf")
                .expect("inspect regular")
                .sha256
        );
        let bold_info = inspect(&bold, "MyFont.ttf").expect("inspect bold");
        assert_eq!(second.sha256, bold_info.sha256);
        let short: String = bold_info.sha256.chars().take(8).collect();
        assert_eq!(second.file_name, format!("MyFont.ttf-{short}"));
        // Both files stand in the library folder, each
        // reading back as itself. (The scan's extension
        // check does not pick up the hashed form — the
        // door's long-standing shape for a renamed
        // namesake, on every door that lands one; the
        // pack door's promise is the two landings
        // themselves, in its own idiom.)
        assert!(dir.join(&first.file_name).exists(), "the first copy stands");
        assert!(
            dir.join(&second.file_name).exists(),
            "the second copy stands"
        );
        assert_eq!(lib.read(first).expect("read first"), regular);
        assert_eq!(lib.read(second).expect("read second"), bold);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_hashed_name_occupant_is_never_overwritten() {
        // Round 32, F7 (red team RT32-5): the hashed-name
        // fallback wrote over whatever already owned the
        // hashed name — the witness library was seeded
        // with one font standing at the incoming font's
        // hashed name, and importing the incoming font
        // under its clean name (owned by a third font)
        // returned the hashed name with the occupant's
        // file silently holding the incoming bytes. The
        // fallback checks occupancy now: the incoming
        // font disambiguates to the longer-hash name,
        // and both standing files keep their own bytes.
        let dir = temp_dir("hashed-occupant");
        let lib = FontLibrary::open(dir.clone()).expect("open");
        let regular = bundled("LiberationSans-Regular.ttf");
        let bold = bundled("LiberationSans-Bold.ttf");
        let occupant = bundled("LiberationSerif-Regular.ttf");
        let owner = lib
            .add_bytes(&regular, "plant.ttf")
            .expect("the clean name's owner lands");
        assert_eq!(owner.file_name, "plant.ttf");
        let bold_info = inspect(&bold, "plant.ttf").expect("inspect bold");
        let short: String = bold_info.sha256.chars().take(8).collect();
        let hashed_name = format!("plant.ttf-{short}");
        std::fs::write(dir.join(&hashed_name), &occupant).expect("seed the occupant");
        let landed = lib
            .add_bytes(&bold, "plant.ttf")
            .expect("the incoming font lands");
        let longer: String = bold_info.sha256.chars().take(16).collect();
        assert_eq!(
            landed.file_name,
            format!("plant.ttf-{longer}"),
            "the incoming font disambiguates past the occupied hashed name"
        );
        assert_eq!(
            std::fs::read(dir.join(&hashed_name)).expect("the occupant still stands"),
            occupant,
            "the occupant's file still holds its own bytes"
        );
        assert_eq!(
            std::fs::read(dir.join("plant.ttf")).expect("the owner still stands"),
            regular,
            "the clean name's owner is untouched"
        );
        assert_eq!(lib.read(&landed).expect("read the landed font"), bold);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_hashed_name_holding_the_same_font_lands_as_before() {
        // Round 32, F7, the other side of the occupancy
        // check: a hashed name already holding the
        // incoming font's own bytes is that font standing
        // under its fallback name, and the import lands
        // there exactly as it did before — no further
        // disambiguation, no second copy.
        let dir = temp_dir("hashed-same-bytes");
        let lib = FontLibrary::open(dir.clone()).expect("open");
        let regular = bundled("LiberationSans-Regular.ttf");
        let bold = bundled("LiberationSans-Bold.ttf");
        lib.add_bytes(&regular, "plant.ttf")
            .expect("the clean name's owner lands");
        let bold_info = inspect(&bold, "plant.ttf").expect("inspect bold");
        let short: String = bold_info.sha256.chars().take(8).collect();
        let hashed_name = format!("plant.ttf-{short}");
        std::fs::write(dir.join(&hashed_name), &bold).expect("seed the same bytes");
        let landed = lib
            .add_bytes(&bold, "plant.ttf")
            .expect("the incoming font lands");
        assert_eq!(landed.file_name, hashed_name);
        assert_eq!(
            std::fs::read(dir.join(&hashed_name)).expect("the hashed file stands"),
            bold
        );
        assert_eq!(
            std::fs::read(dir.join("plant.ttf")).expect("the owner still stands"),
            regular
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn hashed_fallback_walks_past_every_occupied_hash_length_to_a_counter() {
        // Round 32, F7, the end of the disambiguation
        // ladder: when the hashed names at every hash
        // length (8, 16, 32, and the whole hash) are all
        // owned by different bytes, the incoming font
        // lands under a counted name past the whole
        // hash, and every occupant keeps its bytes.
        let dir = temp_dir("hashed-counter");
        let lib = FontLibrary::open(dir.clone()).expect("open");
        let regular = bundled("LiberationSans-Regular.ttf");
        let bold = bundled("LiberationSans-Bold.ttf");
        lib.add_bytes(&regular, "plant.ttf")
            .expect("the clean name's owner lands");
        let bold_info = inspect(&bold, "plant.ttf").expect("inspect bold");
        let mut occupants: Vec<(String, Vec<u8>)> = Vec::new();
        for take in [8usize, 16, 32, 64] {
            let prefix: String = bold_info.sha256.chars().take(take).collect();
            let name = format!("plant.ttf-{prefix}");
            let marker = format!("occupant at {take}").into_bytes();
            std::fs::write(dir.join(&name), &marker).expect("seed an occupant");
            occupants.push((name, marker));
        }
        let landed = lib
            .add_bytes(&bold, "plant.ttf")
            .expect("the incoming font lands");
        assert_eq!(
            landed.file_name,
            format!("plant.ttf-{}-2", bold_info.sha256),
            "past the whole hash, a counter disambiguates"
        );
        assert_eq!(lib.read(&landed).expect("read the landed font"), bold);
        for (name, marker) in &occupants {
            assert_eq!(
                &std::fs::read(dir.join(name)).expect("an occupant still stands"),
                marker,
                "the occupant at {name} keeps its bytes"
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_pack_repeating_one_font_exactly_lands_it_once() {
        // Round 32, F6, the other side: an exact repeat
        // of one font under one name may collapse — the
        // same bytes whichever copy is served, and the
        // door's hash dedup lands it once.
        let regular = bundled("LiberationSans-Regular.ttf");
        let mut buf = pack_zip(&[("MyFont.ttf", &regular), ("MyFont.ttX", &regular)]);
        patch_entry_name(&mut buf, "MyFont.ttX", "MyFont.ttf");
        let dir = temp_dir("dup-font-exact");
        let lib = FontLibrary::open(dir.clone()).expect("open");
        let results = lib.add_zip_bytes(&buf);
        assert_eq!(results.len(), 2, "one result per copy: {results:?}");
        assert!(results.iter().all(|r| r.is_ok()), "{results:?}");
        let scanned = lib.scan();
        assert_eq!(scanned.len(), 1, "the repeat lands as one font");
        assert_eq!(scanned[0].file_name, "MyFont.ttf");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_pack_with_two_license_texts_under_one_name_lands_both() {
        // Round 32, F6 (red team RT32-4, witness
        // f01-dup-license.zip): two LICENSE.txt entries
        // with different texts used to land as the
        // second text only, the first overwritten in
        // silence. License files get the fonts' own
        // treatment now: the first keeps the name, the
        // second lands under its hashed name.
        let font = bundled("LiberationSans-Regular.ttf");
        let text_a: &[u8] = b"FIRST LICENSE TEXT AAA";
        let text_b: &[u8] = b"SECOND LICENSE TEXT BBB";
        let mut buf = pack_zip(&[
            ("Lato-Regular.ttf", &font),
            ("LICENSE.txt", text_a),
            ("LICENSE.txX", text_b),
        ]);
        patch_entry_name(&mut buf, "LICENSE.txX", "LICENSE.txt");
        let dir = temp_dir("dup-license");
        let lib = FontLibrary::open(dir.clone()).expect("open");
        let results = lib.add_zip_bytes(&buf);
        assert_eq!(results.len(), 1, "only the font is a result: {results:?}");
        assert_eq!(
            std::fs::read(dir.join("LICENSE.txt")).expect("first license"),
            text_a
        );
        let short: String = ob_model::sha256_hex(text_b).chars().take(8).collect();
        let second_name = format!("LICENSE.txt-{short}");
        assert_eq!(
            std::fs::read(dir.join(&second_name)).expect("second license"),
            text_b
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn license_files_flattened_from_two_folders_land_both() {
        // Round 32, F6 (red team RT32-4, witness
        // f02-flatten-license.zip): two folders' license
        // files flatten through sanitize_file_name to
        // one name — ofl/LICENSE.txt then
        // apache/LICENSE.txt — and the second used to
        // overwrite the first with no count and no
        // note. Both texts land now, under distinct
        // names.
        let font = bundled("LiberationSans-Regular.ttf");
        let text_a: &[u8] = b"FOLDER ONE LICENSE";
        let text_b: &[u8] = b"FOLDER TWO LICENSE";
        let buf = pack_zip(&[
            ("Lato-Regular.ttf", &font),
            ("ofl/LICENSE.txt", text_a),
            ("apache/LICENSE.txt", text_b),
        ]);
        let dir = temp_dir("flatten-license");
        let lib = FontLibrary::open(dir.clone()).expect("open");
        let results = lib.add_zip_bytes(&buf);
        assert_eq!(results.len(), 1, "only the font is a result: {results:?}");
        assert_eq!(
            std::fs::read(dir.join("LICENSE.txt")).expect("first folder's license"),
            text_a
        );
        let short: String = ob_model::sha256_hex(text_b).chars().take(8).collect();
        assert_eq!(
            std::fs::read(dir.join(format!("LICENSE.txt-{short}")))
                .expect("second folder's license"),
            text_b
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_pack_repeating_one_license_text_exactly_lands_it_once() {
        // Round 32, F6, the exact side for licenses: the
        // same text twice under one name collapses to
        // the one file — nothing is renamed, nothing
        // duplicated.
        let font = bundled("LiberationSans-Regular.ttf");
        let text: &[u8] = b"THE ONE LICENSE TEXT";
        let mut buf = pack_zip(&[
            ("Lato-Regular.ttf", &font),
            ("LICENSE.txt", text),
            ("LICENSE.txX", text),
        ]);
        patch_entry_name(&mut buf, "LICENSE.txX", "LICENSE.txt");
        let dir = temp_dir("dup-license-exact");
        let lib = FontLibrary::open(dir.clone()).expect("open");
        let results = lib.add_zip_bytes(&buf);
        assert_eq!(results.len(), 1, "only the font is a result: {results:?}");
        assert_eq!(
            std::fs::read(dir.join("LICENSE.txt")).expect("the license"),
            text
        );
        let license_files: Vec<String> = std::fs::read_dir(&dir)
            .expect("read library")
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.starts_with("LICENSE"))
            .collect();
        assert_eq!(license_files, vec!["LICENSE.txt".to_string()]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Two bundled fonts behind a collection header (the
    /// same construction ttc::tests proves the splitter
    /// against): the fixture for the intake door.
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
            let num_tables = u16::from_be_bytes([face[4], face[5]]) as usize;
            for t in 0..num_tables {
                let rec = start + 12 + t * 16;
                let off = u32::from_be_bytes([
                    face[12 + t * 16 + 8],
                    face[12 + t * 16 + 9],
                    face[12 + t * 16 + 10],
                    face[12 + t * 16 + 11],
                ]) as usize;
                out[rec + 8..rec + 12].copy_from_slice(&((start + off) as u32).to_be_bytes());
            }
        }
        out
    }

    #[test]
    fn a_collection_added_by_path_lands_as_its_faces() {
        let dir = temp_dir("ttc-intake");
        let lib = FontLibrary::open(dir.clone()).expect("open");
        let ttc = make_ttc(&[
            &bundled("LiberationSans-Regular.ttf"),
            &bundled("LiberationSerif-Regular.ttf"),
        ]);
        let path = dir.join("TwoFaces.ttc");
        std::fs::write(&path, &ttc).expect("write fixture");
        let results = lib.add_path(&path);
        assert_eq!(results.len(), 2, "one result per face: {results:?}");
        assert!(results.iter().all(|r| r.is_ok()), "{results:?}");
        let scanned = lib.scan();
        let families: Vec<&str> = scanned
            .iter()
            .map(|f| f.family.as_str())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        assert!(families.contains(&"Liberation Sans"), "{families:?}");
        assert!(families.contains(&"Liberation Serif"), "{families:?}");
        // And a collection inside a zip pack splits the
        // same way.
        let mut zip_bytes = Vec::new();
        {
            use std::io::Write as _;
            let cursor = std::io::Cursor::new(&mut zip_bytes);
            let mut zw = zip::ZipWriter::new(cursor);
            let opts = zip::write::SimpleFileOptions::default();
            zw.start_file("TwoFaces.ttc", opts).expect("start");
            zw.write_all(&ttc).expect("write ttc");
            zw.finish().expect("finish zip");
        }
        let dir2 = temp_dir("ttc-zip");
        let lib2 = FontLibrary::open(dir2.clone()).expect("open");
        let results = lib2.add_zip_bytes(&zip_bytes);
        assert_eq!(results.len(), 2, "{results:?}");
        assert!(results.iter().all(|r| r.is_ok()));
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&dir2);
    }

    #[test]
    fn web_fonts_are_refused_in_plain_words() {
        let lib = FontLibrary::open(temp_dir("woff")).expect("open");
        // Magic bytes decide, not the name: a WOFF wearing
        // a .ttf name is refused with the web sentence.
        let mut woff = b"wOFF".to_vec();
        woff.extend_from_slice(&bundled("LiberationSans-Regular.ttf"));
        let err = lib
            .add_bytes(&woff, "LiberationSans-Regular.ttf")
            .expect_err("a web font is refused");
        assert!(
            err.contains(
                "That is the web version of a font. OpenBulletin \
                          prints, so it needs the TTF or OTF version of the same font."
            ),
            "{err}"
        );
        assert!(err.contains("LiberationSans-Regular.ttf"), "named: {err}");
        // And by extension at the path door, WOFF2 as well.
        let dir = temp_dir("woff2");
        let lib = FontLibrary::open(dir.clone()).expect("open");
        let path = dir.join("SomeFont.woff2");
        std::fs::write(&path, b"wOF2 dressed for the web").expect("write");
        let results = lib.add_path(&path);
        assert_eq!(results.len(), 1);
        let err = results[0].as_ref().expect_err("refused");
        assert!(err.contains("SomeFont.woff2"), "named: {err}");
        assert!(err.contains("web version of a font"), "{err}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn type1_fonts_are_refused_in_plain_words() {
        let lib = FontLibrary::open(temp_dir("type1")).expect("open");
        let err = lib
            .add_bytes(b"%!PS-AdobeFont-1.0: Old 1.0", "OldFont.pfb")
            .expect_err("a Type 1 font is refused");
        assert!(err.contains("Type 1 format"), "{err}");
        assert!(err.contains("OldFont.pfb"), "named: {err}");
        assert!(err.contains("too old for OpenBulletin to use"), "{err}");
    }

    #[test]
    fn system_scan_never_errors() {
        // Whatever this machine has (possibly nothing), the scan
        // returns parsed facts. (It used to also assert each path
        // exists; a font can vanish mid-scan on a live system,
        // which made the test environment-flaky — closing audit.)
        for found in system_fonts() {
            assert!(!found.info.family.is_empty());
        }
    }

    #[test]
    fn pinning_records_the_library_files_hash() {
        let dir = std::env::temp_dir().join(format!("ob-fonts-pin-{}", std::process::id()));
        let lib = FontLibrary::open(dir.clone()).expect("open library");
        let info = lib
            .add_bytes(
                &bundled("LiberationSans-Regular.ttf"),
                "LiberationSans-Regular.ttf",
            )
            .expect("add font");
        let mut doc = ob_model::Document {
            format_version: ob_model::FORMAT_VERSION,
            title: "Test".into(),
            profile: Default::default(),
            fields: Default::default(),
            styles: vec![ob_model::ParagraphStyle {
                id: "body".into(),
                name: "Body".into(),
                font_size: 11_000,
                bold: false,
                italic: false,
                align: ob_model::Align::Left,
                space_after: 4_000,
                color: ob_model::Rgb::BLACK,
                font_family: "Liberation Sans".into(),
                font_sha256: None,
            }],
            stories: Vec::new(),
            panels: Vec::new(),
            sheet: ob_model::SheetSpec {
                preset: "letter-bifold".into(),
                size: ob_model::Size {
                    w: 612_000,
                    h: 792_000,
                },
            },
            no_announcements: false,
            assets: Vec::new(),
            asset_bytes: Default::default(),
            occasion_time: None,
            wanted_fonts: Vec::new(),
            participants: Default::default(),
            inserts: Default::default(),
            applied_contributions: Vec::new(),
        };
        pin_document_fonts(&mut doc, &lib, &bundled_infos());
        assert_eq!(
            doc.styles[0].font_sha256.as_deref(),
            Some(info.sha256.as_str())
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn pick_face_picks_exact_then_plain_last_wins() {
        // One family's faces in scan order: plain, bold,
        // italic, bold-italic, then a second plain.
        let faces = [
            (false, false),
            (true, false),
            (false, true),
            (true, true),
            (false, false),
        ];
        assert_eq!(pick_face(&faces, true, true), Some(3));
        assert_eq!(pick_face(&faces, false, true), Some(2));
        assert_eq!(pick_face(&faces, true, false), Some(1));
        // A plain want takes the LAST plain face, matching
        // the PDF writer's last-one-wins sources.
        assert_eq!(pick_face(&faces, false, false), Some(4));
        // No exact match and no plain face: nothing.
        assert_eq!(pick_face(&[(true, false)], false, true), None);
        assert_eq!(pick_face(&[], false, false), None);
    }

    #[test]
    fn candidate_faces_are_the_writers_set() {
        // R4-9: one enumeration for the pin and the PDF
        // writer: bundled first, library after (so the
        // library wins a shared key), and only faces
        // whose rights let them travel are candidates.
        let bundled = vec![fake_info(
            "Liberation Sans",
            false,
            EmbeddingRights::Installable,
            "bundled-sha",
        )];
        let library = vec![
            fake_info(
                "Liberation Sans",
                false,
                EmbeddingRights::Installable,
                "library-sha",
            ),
            fake_info("Acme", false, EmbeddingRights::Restricted, "acme-sha"),
            fake_info("Beta", false, EmbeddingRights::PreviewAndPrint, "beta-sha"),
            fake_info("Gamma", true, EmbeddingRights::Editable, "gamma-sha"),
        ];
        let winners = candidate_faces(&bundled, &library);
        let keys: Vec<(&str, bool, &str)> = winners
            .iter()
            .map(|f| (f.family.as_str(), f.bold, f.sha256.as_str()))
            .collect();
        assert_eq!(
            keys,
            vec![
                ("Liberation Sans", false, "library-sha"),
                ("Gamma", true, "gamma-sha"),
            ]
        );
    }

    #[test]
    fn pinning_finds_bundled_faces_with_an_empty_library() {
        // R4-9: the writer embeds the bundled faces even
        // when the library folder is empty, so the pin
        // names them too; before the shared enumeration
        // it named nothing and the pin silently stayed
        // unset.
        let dir = std::env::temp_dir().join(format!("ob-fonts-pin3-{}", std::process::id()));
        let lib = FontLibrary::open(dir.clone()).expect("open library");
        let mut doc = ob_model::Document {
            format_version: ob_model::FORMAT_VERSION,
            title: "Test".into(),
            profile: Default::default(),
            fields: Default::default(),
            styles: vec![ob_model::ParagraphStyle {
                id: "body".into(),
                name: "Body".into(),
                font_size: 11_000,
                bold: false,
                italic: false,
                align: ob_model::Align::Left,
                space_after: 4_000,
                color: ob_model::Rgb::BLACK,
                font_family: "Liberation Sans".into(),
                font_sha256: None,
            }],
            stories: Vec::new(),
            panels: Vec::new(),
            sheet: ob_model::SheetSpec {
                preset: "letter-bifold".into(),
                size: ob_model::Size {
                    w: 612_000,
                    h: 792_000,
                },
            },
            no_announcements: false,
            assets: Vec::new(),
            asset_bytes: Default::default(),
            occasion_time: None,
            wanted_fonts: Vec::new(),
            participants: Default::default(),
            inserts: Default::default(),
            applied_contributions: Vec::new(),
        };
        let bundled = bundled_infos();
        let regular_sha = bundled
            .iter()
            .find(|f| f.family == "Liberation Sans" && !f.bold && !f.italic)
            .expect("bundled regular face")
            .sha256
            .clone();
        pin_document_fonts(&mut doc, &lib, &bundled);
        assert_eq!(
            doc.styles[0].font_sha256.as_deref(),
            Some(regular_sha.as_str())
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_style_with_no_pickable_face_loses_its_stale_pin() {
        // R4-9: when pick_face has nothing to pick (here
        // no candidates at all: an empty library and no
        // bundled faces handed in), the style's old pin
        // is cleared, not left naming a file nothing
        // would print with.
        let dir = std::env::temp_dir().join(format!("ob-fonts-pin4-{}", std::process::id()));
        let lib = FontLibrary::open(dir.clone()).expect("open library");
        let mut doc = ob_model::Document {
            format_version: ob_model::FORMAT_VERSION,
            title: "Test".into(),
            profile: Default::default(),
            fields: Default::default(),
            styles: vec![ob_model::ParagraphStyle {
                id: "body".into(),
                name: "Body".into(),
                font_size: 11_000,
                bold: false,
                italic: false,
                align: ob_model::Align::Left,
                space_after: 4_000,
                color: ob_model::Rgb::BLACK,
                font_family: "Liberation Serif".into(),
                font_sha256: Some("a-stale-hash".into()),
            }],
            stories: Vec::new(),
            panels: Vec::new(),
            sheet: ob_model::SheetSpec {
                preset: "letter-bifold".into(),
                size: ob_model::Size {
                    w: 612_000,
                    h: 792_000,
                },
            },
            no_announcements: false,
            assets: Vec::new(),
            asset_bytes: Default::default(),
            occasion_time: None,
            wanted_fonts: Vec::new(),
            participants: Default::default(),
            inserts: Default::default(),
            applied_contributions: Vec::new(),
        };
        pin_document_fonts(&mut doc, &lib, &[]);
        assert_eq!(doc.styles[0].font_sha256, None);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn pinning_records_each_faces_own_file() {
        // R3-13: an italic style pins the italic face's
        // file, bold-italic the bold-italic file; before
        // the shared pick, italic styles pinned a
        // non-italic face (or the family's first).
        let dir = std::env::temp_dir().join(format!("ob-fonts-pin2-{}", std::process::id()));
        let lib = FontLibrary::open(dir.clone()).expect("open library");
        let mut shas = std::collections::HashMap::new();
        for name in [
            "LiberationSans-Regular.ttf",
            "LiberationSans-Italic.ttf",
            "LiberationSans-BoldItalic.ttf",
        ] {
            let info = lib.add_bytes(&bundled(name), name).expect("add font");
            shas.insert(name, info.sha256);
        }
        let style = |id: &str, bold: bool, italic: bool| ob_model::ParagraphStyle {
            id: id.into(),
            name: id.into(),
            font_size: 11_000,
            bold,
            italic,
            align: ob_model::Align::Left,
            space_after: 4_000,
            color: ob_model::Rgb::BLACK,
            font_family: "Liberation Sans".into(),
            font_sha256: None,
        };
        let mut doc = ob_model::Document {
            format_version: ob_model::FORMAT_VERSION,
            title: "Test".into(),
            profile: Default::default(),
            fields: Default::default(),
            styles: vec![
                style("plain", false, false),
                style("italic", false, true),
                style("bolditalic", true, true),
            ],
            stories: Vec::new(),
            panels: Vec::new(),
            sheet: ob_model::SheetSpec {
                preset: "letter-bifold".into(),
                size: ob_model::Size {
                    w: 612_000,
                    h: 792_000,
                },
            },
            no_announcements: false,
            assets: Vec::new(),
            asset_bytes: Default::default(),
            occasion_time: None,
            wanted_fonts: Vec::new(),
            participants: Default::default(),
            inserts: Default::default(),
            applied_contributions: Vec::new(),
        };
        pin_document_fonts(&mut doc, &lib, &bundled_infos());
        assert_eq!(
            doc.styles[0].font_sha256.as_deref(),
            Some(shas["LiberationSans-Regular.ttf"].as_str())
        );
        assert_eq!(
            doc.styles[1].font_sha256.as_deref(),
            Some(shas["LiberationSans-Italic.ttf"].as_str())
        );
        assert_eq!(
            doc.styles[2].font_sha256.as_deref(),
            Some(shas["LiberationSans-BoldItalic.ttf"].as_str())
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn zip_entries_past_the_cap_are_counted_not_dropped() {
        // RT3-F: a pack with more entries than the cap
        // says how many were left out, in plain words.
        use std::io::Write as _;
        let mut buf = Vec::new();
        {
            let mut zw = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
            let opts = zip::write::SimpleFileOptions::default();
            for i in 0..600 {
                zw.start_file(format!("f{i}.ttf"), opts).expect("zip entry");
                zw.write_all(b"not a font").expect("zip write");
            }
            zw.finish().expect("zip finish");
        }
        let dir = std::env::temp_dir().join(format!("ob-fonts-zipcap-{}", std::process::id()));
        let lib = FontLibrary::open(dir.clone()).expect("open library");
        let results = lib.add_zip_bytes(&buf);
        let messages: Vec<&str> = results
            .iter()
            .filter_map(|r| r.as_ref().err().map(|e| e.as_str()))
            .collect();
        assert!(
            messages
                .iter()
                .any(|m| m.contains("88 more files were in the pack")),
            "the skip line names the count: {messages:?}"
        );
        std::fs::remove_dir_all(&dir).ok();
    }
}
