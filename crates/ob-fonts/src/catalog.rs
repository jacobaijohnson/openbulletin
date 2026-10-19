//! The free-font catalog (docs/FONTS.md): search and one-tap install
//! from Fontsource, which mirrors the Google Fonts catalog with each
//! family's license recorded. Only families under OFL, Apache, or
//! UFL are offered; anything else is filtered out before the user
//! ever sees it. Downloads land in the Font Library like any added
//! font, license note saved alongside.

use crate::{FontInfo, FontLibrary};
use serde::Deserialize;

const LIST_URL: &str = "https://api.fontsource.org/v1/fonts";
const DETAIL_URL: &str = "https://api.fontsource.org/v1/fonts";

/// A catalog face's verified glyph coverage: characters a
/// downloaded face's own cmap was checked to carry, and
/// no others. Only faces whose downloaded cmap was
/// actually read may be listed here; the Check step points
/// at these when a bulletin uses a mark the printing face
/// lacks, so an entry is a promise the app makes out loud.
pub struct CatalogGlyphCoverage {
    /// Catalog id (the Fontsource id).
    pub id: &'static str,
    /// The family name as the Fonts list shows it.
    pub family: &'static str,
    /// Characters verified present in the face's cmap.
    pub chars: &'static [char],
}

/// The verified coverage registry. DejaVu Sans is here
/// because the bundled Liberation faces carry neither the
/// Latin cross (U+271D) nor the check mark (U+2713): O2's
/// cmap dump proved the absence, and the DejaVu Sans
/// face's own cmap was read and does carry both (the full
/// face holds 6,253 glyphs, verified 2026-10-16).
pub const VERIFIED_GLYPH_COVERAGE: &[CatalogGlyphCoverage] = &[CatalogGlyphCoverage {
    id: "dejavu-sans",
    family: "DejaVu Sans",
    chars: &['\u{271D}', '\u{2713}'],
}];

/// The family of a catalog face that verifiably carries
/// `c`, when one is registered: the Check step's "this
/// face has this letter" pointer. Registry data only; the
/// cmap test in this crate keeps the promise honest.
pub fn catalog_face_with_glyph(c: char) -> Option<&'static str> {
    VERIFIED_GLYPH_COVERAGE
        .iter()
        .find(|cov| cov.chars.contains(&c))
        .map(|cov| cov.family)
}

#[derive(Debug, Clone, Deserialize)]
pub struct CatalogEntry {
    pub id: String,
    pub family: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub license: String,
}

fn license_is_free(license: &str) -> bool {
    let l = license.to_uppercase();
    l.starts_with("OFL") || l.starts_with("APACHE") || l.starts_with("UFL")
}

/// TLS roots from the operating system's own trust store, merged
/// with the usual public roots as a fallback. The system store is
/// what browsers and curl trust, including the certificate
/// authorities of workplace and home-server proxies; the bundled
/// web roots alone would refuse those networks.
/// TLS roots from the operating system's own trust store, merged
/// with the usual public roots as a fallback. The system store is
/// what browsers and curl trust, including the certificate
/// authorities of workplace and home-server proxies; the bundled
/// web roots alone would refuse those networks.
fn tls_config() -> rustls::ClientConfig {
    let mut roots = rustls::RootCertStore::empty();
    let native = rustls_native_certs::load_native_certs();
    let (added, _) = roots.add_parsable_certificates(native.certs);
    if added == 0 {
        // No readable system store: use the public web roots.
        roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    }
    rustls::ClientConfig::builder_with_provider(std::sync::Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .expect("rustls protocol versions")
    .with_root_certificates(roots)
    .with_no_client_auth()
}

/// The agents to try, best first: a direct connection, then the
/// proxy named by the standard environment variables (HTTPS_PROXY
/// and friends) for networks that require one. A call tries each in
/// order; plenty of real networks break one path or the other.
fn agents() -> Vec<ureq::Agent> {
    let base = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(30))
        .tls_config(std::sync::Arc::new(tls_config()));
    let mut out = vec![base.build()];
    let proxy_url = std::env::var("HTTPS_PROXY")
        .or_else(|_| std::env::var("https_proxy"))
        .or_else(|_| std::env::var("ALL_PROXY"))
        .or_else(|_| std::env::var("all_proxy"));
    if let Ok(url) = proxy_url {
        if let Ok(proxy) = ureq::Proxy::new(&url) {
            out.push(
                ureq::AgentBuilder::new()
                    .timeout(std::time::Duration::from_secs(30))
                    .tls_config(std::sync::Arc::new(tls_config()))
                    .proxy(proxy)
                    .build(),
            );
        }
    }
    out
}

/// The most one catalog download may be: 32 MB, counted
/// in actual bytes as the body arrives. Font files have
/// always been held to it (a font file is a few MB); the
/// JSON and text answers go through the same capped
/// reader, so no catalog answer can stream without
/// limit into memory (health audit S-5).
const MAX_DOWNLOAD_BYTES: u64 = 33_554_432;

/// Read one response body through the download cap. Some
/// proxies and CDNs close the connection without the TLS
/// farewell message, which strict readers report as an
/// error after the last byte; when we already hold
/// exactly the promised byte count the file is complete,
/// and the caller's own parser is the final judge either
/// way.
fn read_body_capped(
    reader: impl std::io::Read,
    expected: Option<usize>,
    url: &str,
) -> Result<Vec<u8>, String> {
    let mut reader = std::io::Read::take(reader, MAX_DOWNLOAD_BYTES + 1);
    let mut bytes = Vec::new();
    let mut buf = [0u8; 16384];
    let mut failed: Option<String> = None;
    loop {
        match std::io::Read::read(&mut reader, &mut buf) {
            Ok(0) => break,
            Ok(k) => bytes.extend_from_slice(&buf[..k]),
            Err(e) => {
                if expected != Some(bytes.len()) {
                    failed = Some(format!("{url}: {e}"));
                }
                break;
            }
        }
    }
    if let Some(e) = failed {
        return Err(e);
    }
    if bytes.len() as u64 > MAX_DOWNLOAD_BYTES {
        return Err(format!(
            "{url}: that download is too large (over 32 megabytes)"
        ));
    }
    Ok(bytes)
}

/// GET and parse JSON, trying each agent in turn.
fn get_json(url: &str) -> Result<serde_json::Value, String> {
    let mut last = String::new();
    for agent in agents() {
        match agent.get(url).call() {
            Ok(response) => {
                let expected: Option<usize> = response
                    .header("Content-Length")
                    .and_then(|v| v.parse().ok());
                match read_body_capped(response.into_reader(), expected, url).and_then(|bytes| {
                    serde_json::from_slice(&bytes).map_err(|e| format!("{url}: {e}"))
                }) {
                    Ok(v) => return Ok(v),
                    Err(e) => last = e,
                }
            }
            Err(e) => last = format!("{url}: {e}"),
        }
    }
    Err(last)
}

/// GET a text body, trying each agent in turn.
fn get_string(url: &str) -> Result<String, String> {
    let mut last = String::new();
    for agent in agents() {
        match agent.get(url).call() {
            Ok(response) => {
                let expected: Option<usize> = response
                    .header("Content-Length")
                    .and_then(|v| v.parse().ok());
                match read_body_capped(response.into_reader(), expected, url)
                    .and_then(|bytes| String::from_utf8(bytes).map_err(|e| format!("{url}: {e}")))
                {
                    Ok(v) => return Ok(v),
                    Err(e) => last = e,
                }
            }
            Err(e) => last = format!("{url}: {e}"),
        }
    }
    Err(last)
}

/// One whole-file download, trying each agent in turn,
/// its body read through the same capped reader as every
/// catalog answer (see read_body_capped).
fn get_bytes(url: &str) -> Result<Vec<u8>, String> {
    let mut last = String::new();
    for agent in agents() {
        match agent.get(url).call() {
            Ok(response) => {
                let expected: Option<usize> = response
                    .header("Content-Length")
                    .and_then(|v| v.parse().ok());
                match read_body_capped(response.into_reader(), expected, url) {
                    Ok(bytes) => return Ok(bytes),
                    Err(e) => last = e,
                }
            }
            Err(e) => last = format!("{url}: {e}"),
        }
    }
    Err(last)
}

/// Search the catalog by family name. Network call.
pub fn search(query: &str) -> Result<Vec<CatalogEntry>, String> {
    let raw = get_json(LIST_URL).map_err(|e| format!("cannot reach the font catalog: {e}"))?;
    let all: Vec<CatalogEntry> =
        serde_json::from_value(raw).map_err(|e| format!("the font catalog answered oddly: {e}"))?;
    let q = query.to_lowercase();
    let mut hits: Vec<CatalogEntry> = all
        .into_iter()
        .filter(|e| license_is_free(&e.license))
        .filter(|e| e.family.to_lowercase().contains(&q) || e.id.contains(&q))
        .collect();
    hits.sort_by(|a, b| a.family.cmp(&b.family));
    hits.truncate(25);
    Ok(hits)
}

/// Choose the files of one family that an install fetches:
/// the four core static faces when the family ships them;
/// a variable font ships as a single file (or an upright
/// and an italic) and is taken as it comes. Match on the
/// exact style suffix after the last dash: substring
/// matching is a trap ("SemiBold" contains "Bold", "Thin"
/// is nobody's Regular). Pure list work, no network: the
/// catalog tests pin the shapes real families ship,
/// Cabin's among them.
fn core_face_files(files: &[String]) -> Vec<String> {
    let style_of = |f: &str| -> String {
        let stem = f.trim_end_matches(".ttf").trim_end_matches(".otf");
        match stem.rfind('-') {
            Some(i) => stem[i + 1..].to_string(),
            None => String::new(),
        }
    };
    let pick = |styles: &[&str]| {
        files
            .iter()
            .find(|f| styles.contains(&style_of(f).as_str()))
            .cloned()
    };
    let core: Vec<String> = [
        pick(&["BoldItalic"]),
        pick(&["Regular", ""]),
        pick(&["Bold"]),
        pick(&["Italic"]),
    ]
    .into_iter()
    .flatten()
    .collect();
    let mut files = if core.is_empty() {
        files.to_vec()
    } else {
        core
    };
    files.sort();
    files.dedup();
    files.truncate(4);
    files
}

/// Download the core faces of one catalog family into the library,
/// with a license note beside them. Network call.
///
/// Files come from the google/fonts repository itself (the catalog's
/// own upstream) rather than a CDN mirror: the repository answers
/// everywhere, including networks where CDN connections get cut.
pub fn install(lib: &FontLibrary, id: &str) -> Result<Vec<FontInfo>, String> {
    let detail: serde_json::Value = get_json(&format!("{DETAIL_URL}/{id}"))
        .map_err(|e| format!("cannot reach the font catalog: {e}"))?;
    let license = detail["license"].as_str().unwrap_or("").to_string();
    if !license_is_free(&license) {
        return Err(format!(
            "{id} is under {license}, which is not one of the free licenses we can offer"
        ));
    }
    // DejaVu is not in the google/fonts repository the probe
    // below walks; it has its own route (see install_dejavu).
    if id == "dejavu-sans" {
        return install_dejavu(lib, &detail);
    }
    // The family's folder in google/fonts, by license directory.
    // Repository folders drop dashes (Open Sans lives in opensans/),
    // so probe the plain id and the dash-less id in each directory.
    let squashed = id.replace('-', "");
    let mut found: Option<(String, String, String)> = None;
    'probe: for d in ["ofl", "apache", "ufl"] {
        for cand in [id, squashed.as_str()] {
            let url = format!(
                "https://raw.githubusercontent.com/google/fonts/main/{d}/{cand}/METADATA.pb"
            );
            if let Ok(text) = get_string(&url) {
                found = Some((d.to_string(), cand.to_string(), text));
                break 'probe;
            }
        }
    }
    let (license_dir, repo_id, metadata) =
        found.ok_or_else(|| format!("cannot find {id} in the free font repository"))?;
    let mut files: Vec<String> = metadata
        .lines()
        .filter_map(|l| {
            let l = l.trim();
            l.strip_prefix("filename:")?
                .trim()
                .trim_matches('"')
                .to_string()
                .into()
        })
        .filter(|f: &String| f.to_lowercase().ends_with(".ttf"))
        .collect();
    if files.is_empty() {
        return Err(format!("no font files listed for {id}"));
    }
    files = core_face_files(&files);

    let mut installed = Vec::new();
    for file_name in &files {
        let url = format!(
            "https://raw.githubusercontent.com/google/fonts/main/{license_dir}/{repo_id}/{file_name}"
        );
        let bytes = get_bytes(&url).map_err(|e| format!("download failed for {id}: {e}"))?;
        installed.push(lib.add_bytes(&bytes, file_name)?);
    }
    if installed.is_empty() {
        return Err(format!("no downloadable faces found for {id}"));
    }
    write_license_note(lib, &repo_id, &detail);
    Ok(installed)
}

/// DejaVu Sans's own install route. The family is not in
/// the google/fonts repository; its home is the DejaVu
/// project's own releases, whose sans zip carries the four
/// core faces and the project's LICENSE file. The license
/// note says what is actually true, both halves of it: the
/// Fontsource catalog records the family as OFL-1.1, and
/// the font's own embedded license text (and the release's
/// LICENSE file) is the DejaVu license: the Bitstream Vera
/// terms, with the DejaVu changes in the public domain.
/// Both are permissive; the note carries the project's own
/// text verbatim so nothing rests on the catalog's word
/// alone.
/// The DejaVu release zip is read under the workspace
/// font-pack reader's caps (the pack intake in lib.rs):
/// at most 512 entries, 64 MB a file, 256 MB in all,
/// counted in actual bytes as read (health audit S-5).
const PACK_MAX_ENTRIES: usize = 512;
const PACK_MAX_ENTRY_BYTES: u64 = 64 * 1024 * 1024;
const PACK_MAX_TOTAL_BYTES: u64 = 256 * 1024 * 1024;

/// What the DejaVu release zip gave up: the core faces
/// it carried (each under the name the library files it
/// by) and the project's license text, when it carried
/// one.
#[derive(Debug)]
struct DejaVuZipContents {
    faces: Vec<(&'static str, Vec<u8>)>,
    license: Option<String>,
}

/// One entry of the release zip, read under the pack
/// caps and added to the running total. None when the
/// entry is absent, unreadable, or past a cap: the
/// caller simply goes on without it, and the install's
/// second route stands behind a zip that gives up
/// nothing usable.
fn read_pack_entry(
    archive: &mut zip::ZipArchive<std::io::Cursor<Vec<u8>>>,
    name: &str,
    total: &mut u64,
) -> Option<Vec<u8>> {
    // The read goes through ob-model's one capped read
    // (health audit E-9). This door's judgment is Skip:
    // an entry that fails to read, or runs past the
    // per-entry cap, is simply not given up.
    let mut f = archive.by_name(name).ok()?;
    let read =
        ob_model::read_capped(&mut f, PACK_MAX_ENTRY_BYTES, ob_model::CapOverflow::Skip).ok()?;
    if read.overflowed {
        return None;
    }
    let data = read.bytes;
    *total += data.len() as u64;
    if *total > PACK_MAX_TOTAL_BYTES {
        return None;
    }
    Some(data)
}

/// Read the DejaVu release zip under the pack caps. An
/// archive past the entry-count cap is refused whole,
/// in plain words; a face or the license past its own
/// cap is left out of the contents, as a failed read
/// always has been on this route.
fn read_dejavu_zip(bytes: &[u8]) -> Result<DejaVuZipContents, String> {
    const FACES: [&str; 4] = [
        "DejaVuSans.ttf",
        "DejaVuSans-Bold.ttf",
        "DejaVuSans-Oblique.ttf",
        "DejaVuSans-BoldOblique.ttf",
    ];
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes.to_vec()))
        .map_err(|e| format!("that font pack cannot be read: {e}"))?;
    if archive.len() > PACK_MAX_ENTRIES {
        return Err(format!(
            "that font pack holds {} files; a pack may carry at most {}",
            archive.len(),
            PACK_MAX_ENTRIES
        ));
    }
    let names: Vec<String> = (0..archive.len())
        .filter_map(|i| archive.by_index(i).ok().map(|f| f.name().to_string()))
        .collect();
    let mut total: u64 = 0;
    let mut faces: Vec<(&'static str, Vec<u8>)> = Vec::new();
    for want in FACES {
        if let Some(entry) = names.iter().find(|n| n.ends_with(want)) {
            if let Some(data) = read_pack_entry(&mut archive, entry, &mut total) {
                if !data.is_empty() {
                    faces.push((want, data));
                }
            }
        }
    }
    let mut license = None;
    if let Some(entry) = names.iter().find(|n| n.ends_with("LICENSE")) {
        if let Some(data) = read_pack_entry(&mut archive, entry, &mut total) {
            license = String::from_utf8(data).ok();
        }
    }
    Ok(DejaVuZipContents { faces, license })
}

fn install_dejavu(lib: &FontLibrary, detail: &serde_json::Value) -> Result<Vec<FontInfo>, String> {
    let catalog_license = detail["license"].as_str().unwrap_or("OFL-1.1");
    let source = detail["source"]
        .as_str()
        .unwrap_or("https://github.com/dejavu-fonts/dejavu-fonts");
    let mut installed: Vec<FontInfo> = Vec::new();
    let mut project_license: Option<String> = None;

    // First route: the project's own release zip, read
    // under the font-pack caps (S-5).
    let zip_url = "https://github.com/dejavu-fonts/dejavu-fonts/releases/download/version_2_37/dejavu-sans-ttf-2.37.zip";
    if let Ok(bytes) = get_bytes(zip_url) {
        if let Ok(contents) = read_dejavu_zip(&bytes) {
            project_license = contents.license;
            for (want, data) in &contents.faces {
                if let Ok(info) = lib.add_bytes(data, want) {
                    installed.push(info);
                }
            }
        }
    }
    if !installed.is_empty() {
        let note = format!(
            "DejaVu Sans\nCatalog license record: {catalog_license} (Fontsource catalog)\nSource: {source}\n\nThe font's own license, from the project's release:\n\n{}",
            project_license.as_deref().unwrap_or(
                "The release's LICENSE file could not be read back; the license text lives at https://dejavu-fonts.github.io/License.html"
            )
        );
        let _ = std::fs::write(lib.dir().join("dejavu-sans-LICENSE.txt"), note);
        return Ok(installed);
    }

    // Second route: the Fontsource CDN files, one face at a
    // time (the full faces, not subsets).
    const CDN: [(&str, &str); 4] = [
        ("latin-400-normal.ttf", "DejaVuSans.ttf"),
        ("latin-700-normal.ttf", "DejaVuSans-Bold.ttf"),
        ("latin-400-italic.ttf", "DejaVuSans-Oblique.ttf"),
        ("latin-700-italic.ttf", "DejaVuSans-BoldOblique.ttf"),
    ];
    for (cdn_name, file_name) in CDN {
        let url =
            format!("https://cdn.jsdelivr.net/fontsource/fonts/dejavu-sans@latest/{cdn_name}");
        if let Ok(bytes) = get_bytes(&url) {
            if let Ok(info) = lib.add_bytes(&bytes, file_name) {
                installed.push(info);
            }
        }
    }
    if installed.is_empty() {
        return Err(
            "dejavu-sans could not be downloaded from the DejaVu project or the font catalog"
                .to_string(),
        );
    }
    let note = format!(
        "DejaVu Sans\nCatalog license record: {catalog_license} (Fontsource catalog)\nSource: {source}\nThe font's own license text: https://dejavu-fonts.github.io/License.html\n"
    );
    let _ = std::fs::write(lib.dir().join("dejavu-sans-LICENSE.txt"), note);
    Ok(installed)
}

fn write_license_note(lib: &FontLibrary, repo_id: &str, detail: &serde_json::Value) {
    let license = detail["license"].as_str().unwrap_or("unknown");
    let source = detail["source"].as_str().unwrap_or("Fontsource");
    let family = detail["family"].as_str().unwrap_or(repo_id);
    // For Google-sourced OFL families the full license text lives in
    // the google/fonts repo beside the font. Best effort: a failure
    // leaves the note, which still names the license and its source.
    let text = get_string(&format!(
        "https://raw.githubusercontent.com/google/fonts/main/ofl/{repo_id}/OFL.txt"
    ))
    .ok()
    .unwrap_or_else(|| {
            format!(
                "{family}\nLicense: {license}\nSource: {source}\nFull license text: see the family's entry in the Google Fonts catalog.\n"
            )
        });
    let _ = std::fs::write(lib.dir().join(format!("{repo_id}-LICENSE.txt")), text);
}

/// The Church Starter Pack: metric twins for the fonts church
/// computers already have, plus two graceful faces for headings and
/// readings. Liberation (the Arial/Times twin) is bundled with the
/// app itself, so it is not listed here. Ids are catalog ids.
pub const CHURCH_STARTER_PACK: &[&str] = &[
    "carlito",          // Calibri twin
    "caladea",          // Cambria twin
    "gelasio",          // Georgia twin
    "lora",             // a warm reading serif
    "playfair-display", // a display face for titles and holidays
    // DejaVu Sans rides along because it carries the marks
    // the Liberation faces do not: the Latin cross and the
    // check mark (see VERIFIED_GLYPH_COVERAGE).
    "dejavu-sans",
];

/// What a pack install did: the faces that landed, and the families
/// that failed with the reason (one bad family never sinks a pack).
pub struct PackReport {
    pub installed: Vec<crate::FontInfo>,
    pub failed: Vec<(String, String)>,
}

/// Install several families, tolerating per-family failure.
pub fn install_pack(lib: &crate::FontLibrary, ids: &[&str]) -> PackReport {
    let mut report = PackReport {
        installed: Vec::new(),
        failed: Vec::new(),
    };
    for id in ids {
        match install(lib, id) {
            Ok(mut faces) => report.installed.append(&mut faces),
            Err(e) => report.failed.push((id.to_string(), e)),
        }
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capped_body_reader_refuses_past_32_mb_and_passes_ordinary_bodies() {
        // The reader every catalog answer now goes
        // through (S-5): an ordinary body passes whole,
        // and a body past the cap is refused on the bytes
        // that actually arrived, counted as read.
        let body = read_body_capped(std::io::Cursor::new(b"[]".to_vec()), None, "test://x")
            .expect("small body");
        assert_eq!(body, b"[]");
        let big = vec![0u8; (MAX_DOWNLOAD_BYTES + 1) as usize];
        let err = read_body_capped(std::io::Cursor::new(big), None, "test://x")
            .expect_err("over-cap body refused");
        assert!(err.contains("too large"), "{err}");
        assert!(err.contains("32 megabytes"), "{err}");
    }

    fn zip_bytes(entries: &[(&str, Vec<u8>)]) -> Vec<u8> {
        let mut buf = Vec::new();
        {
            let mut writer = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
            let opts = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);
            for (name, data) in entries {
                use std::io::Write as _;
                writer.start_file(*name, opts).expect("start entry");
                writer.write_all(data).expect("write entry");
            }
            writer.finish().expect("finish zip");
        }
        buf
    }

    #[test]
    fn dejavu_zip_past_the_entry_count_cap_is_refused() {
        // 513 tiny entries against the pack's 512 cap.
        let entries: Vec<(String, Vec<u8>)> = (0..(PACK_MAX_ENTRIES + 1))
            .map(|i| (format!("file-{i}.txt"), b"x".to_vec()))
            .collect();
        let refs: Vec<(&str, Vec<u8>)> = entries
            .iter()
            .map(|(n, d)| (n.as_str(), d.clone()))
            .collect();
        let err = read_dejavu_zip(&zip_bytes(&refs)).expect_err("too many entries refused");
        assert!(err.contains("512"), "{err}");
    }

    #[test]
    fn dejavu_zip_entry_past_its_cap_is_left_out_and_the_rest_reads() {
        // One face a hair past 64 MB of almost-nothing
        // (deflated to very little on the wire): it is
        // left out, the license still reads, and nothing
        // fails whole.
        let big = vec![0u8; (PACK_MAX_ENTRY_BYTES + 1) as usize];
        let bytes = zip_bytes(&[
            ("dejavu-sans-ttf-2.37/ttf/DejaVuSans.ttf", big),
            ("dejavu-sans-ttf-2.37/LICENSE", b"the license".to_vec()),
        ]);
        let contents = read_dejavu_zip(&bytes).expect("the zip reads");
        assert!(contents.faces.is_empty(), "the over-cap face is left out");
        assert_eq!(contents.license.as_deref(), Some("the license"));
    }

    #[test]
    fn dejavu_zip_reads_faces_and_license_under_the_caps() {
        let bytes = zip_bytes(&[
            (
                "dejavu-sans-ttf-2.37/ttf/DejaVuSans.ttf",
                b"font bytes".to_vec(),
            ),
            ("dejavu-sans-ttf-2.37/LICENSE", b"the license".to_vec()),
        ]);
        let contents = read_dejavu_zip(&bytes).expect("the zip reads");
        assert_eq!(contents.faces.len(), 1);
        assert_eq!(contents.faces[0].0, "DejaVuSans.ttf");
        assert_eq!(contents.faces[0].1, b"font bytes");
        assert_eq!(contents.license.as_deref(), Some("the license"));
    }

    #[test]
    fn license_gate() {
        assert!(license_is_free("OFL-1.1"));
        assert!(license_is_free("Apache-2.0"));
        assert!(license_is_free("UFL-1.0"));
        assert!(!license_is_free("proprietary"));
    }

    #[test]
    fn catalog_entries_parse() {
        let json = r#"[{"id":"lato","family":"Lato","category":"sans-serif","license":"OFL-1.1"}]"#;
        let entries: Vec<CatalogEntry> = serde_json::from_str(json).expect("parse");
        assert_eq!(entries[0].family, "Lato");
    }

    #[test]
    fn cabin_is_a_free_catalog_family_the_installer_can_fetch() {
        // Cabin (docs/FONTS.md): the Gill Sans lookalike,
        // drawn by Pablo Impallari after Edward Johnston's
        // and Eric Gill's typefaces, under the SIL Open Font
        // License 1.1. It is not a static list entry here:
        // the catalog is the live Fontsource index, and this
        // is the entry shape that index serves for it. The
        // license gate must pass it, and the resolution
        // table's lookalike must be this very family.
        let json =
            r#"[{"id":"cabin","family":"Cabin","category":"sans-serif","license":"OFL-1.1"}]"#;
        let entries: Vec<CatalogEntry> = serde_json::from_str(json).expect("parse");
        assert_eq!(entries[0].id, "cabin");
        assert_eq!(entries[0].family, "Cabin");
        assert!(license_is_free(&entries[0].license));
        let sub = crate::resolve::substitution_for("GillSansMT").expect("gill is in the table");
        assert_eq!(sub.substitute, entries[0].family);
        assert_eq!(sub.tier, crate::resolve::SubstituteTier::Lookalike);
        // The installer's own road for it: google/fonts
        // carries Cabin as a variable font (an upright and
        // an italic file), and the core-face pick takes a
        // variable family as it comes, the upright first.
        let files = vec![
            "Cabin[wdth,wght].ttf".to_string(),
            "Cabin-Italic[wdth,wght].ttf".to_string(),
        ];
        let picked = core_face_files(&files);
        assert!(
            picked.contains(&"Cabin[wdth,wght].ttf".to_string()),
            "{picked:?}"
        );
        // A family shipping static faces yields its four
        // core faces, no more.
        let statics: Vec<String> = [
            "Fam-Thin.ttf",
            "Fam-Regular.ttf",
            "Fam-Bold.ttf",
            "Fam-Italic.ttf",
            "Fam-BoldItalic.ttf",
            "Fam-Black.ttf",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let picked = core_face_files(&statics);
        assert_eq!(picked.len(), 4, "{picked:?}");
        assert!(picked.contains(&"Fam-Regular.ttf".to_string()));
        assert!(picked.contains(&"Fam-BoldItalic.ttf".to_string()));
    }

    #[test]
    fn the_registry_names_dejavu_for_the_marks_liberation_lacks() {
        assert_eq!(catalog_face_with_glyph('\u{271D}'), Some("DejaVu Sans"));
        assert_eq!(catalog_face_with_glyph('\u{2713}'), Some("DejaVu Sans"));
        // An ordinary letter is in every face; the registry
        // must not claim it (the pointer is only for marks
        // the printing faces are known to lack).
        assert_eq!(catalog_face_with_glyph('A'), None);
        assert!(CHURCH_STARTER_PACK.contains(&"dejavu-sans"));
    }

    /// The DejaVu Sans face's own cmap, wherever a real copy
    /// lives on this machine (an OS copy or a test-provided
    /// directory): every character the registry promises
    /// must really be there. Skips cleanly where no copy
    /// exists; the promise is only made from verified data,
    /// and this test is what keeps it verified.
    fn dejavu_sans_bytes() -> Option<Vec<u8>> {
        let mut candidates: Vec<std::path::PathBuf> = Vec::new();
        if let Ok(dir) = std::env::var("OB_DEJAVU_DIR") {
            candidates.push(std::path::PathBuf::from(dir).join("DejaVuSans.ttf"));
        }
        candidates.push("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf".into());
        candidates.push("/usr/share/fonts/TTF/DejaVuSans.ttf".into());
        candidates.push("/Library/Fonts/DejaVuSans.ttf".into());
        candidates.push("C:\\Windows\\Fonts\\DejaVuSans.ttf".into());
        candidates.iter().find_map(|p| std::fs::read(p).ok())
    }

    #[test]
    fn dejavu_sans_cmap_carries_the_registered_marks() {
        let Some(bytes) = dejavu_sans_bytes() else {
            return; // no DejaVu on this machine: nothing to verify against
        };
        let face = ttf_parser::Face::parse(&bytes, 0).expect("parse DejaVu Sans");
        assert_eq!(
            crate::inspect(&bytes, "DejaVuSans.ttf").map(|i| i.family),
            Ok("DejaVu Sans".to_string()),
            "the located copy really is DejaVu Sans"
        );
        for cov in VERIFIED_GLYPH_COVERAGE {
            if cov.id != "dejavu-sans" {
                continue;
            }
            for c in cov.chars {
                assert!(
                    face.glyph_index(*c).is_some(),
                    "DejaVu Sans is registered as carrying {c} (U+{:04X}) but its cmap has no glyph",
                    *c as u32
                );
            }
        }
    }

    #[test]
    fn bundled_liberation_sans_has_no_latin_cross() {
        // The registry's premise, pinned: if a future
        // Liberation ever gains the cross, this test says
        // the DejaVu pointer has become unnecessary, not
        // wrong, and the registry should be revisited.
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../ob-text/assets/fonts/LiberationSans-Regular.ttf"
        );
        let Ok(bytes) = std::fs::read(path) else {
            return;
        };
        let face = ttf_parser::Face::parse(&bytes, 0).expect("parse Liberation Sans");
        assert!(face.glyph_index('\u{271D}').is_none());
        assert!(face.glyph_index('\u{2713}').is_none());
    }
}
