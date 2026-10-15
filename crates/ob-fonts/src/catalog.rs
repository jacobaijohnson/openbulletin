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

/// GET and parse JSON, trying each agent in turn.
fn get_json(url: &str) -> Result<serde_json::Value, String> {
    let mut last = String::new();
    for agent in agents() {
        match agent.get(url).call() {
            Ok(r) => match r.into_json() {
                Ok(v) => return Ok(v),
                Err(e) => last = format!("{url}: {e}"),
            },
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
            Ok(r) => match r.into_string() {
                Ok(v) => return Ok(v),
                Err(e) => last = format!("{url}: {e}"),
            },
            Err(e) => last = format!("{url}: {e}"),
        }
    }
    Err(last)
}

/// One whole-file download, trying each agent in turn. Some proxies
/// and CDNs close the connection without the TLS farewell message,
/// which strict readers report as an error after the last byte;
/// when we already hold exactly the promised byte count the file is
/// complete, and the font parser is the final judge either way.
fn get_bytes(url: &str) -> Result<Vec<u8>, String> {
    let mut last = String::new();
    for agent in agents() {
        match agent.get(url).call() {
            Ok(response) => {
                let expected: Option<usize> = response
                    .header("Content-Length")
                    .and_then(|v| v.parse().ok());
                // Cap downloads: a font file is a few MB; a
                // hostile or broken endpoint must not stream
                // forever into memory (closing audit).
                let mut reader = std::io::Read::take(response.into_reader(), 33_554_432);
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
                match failed {
                    None => return Ok(bytes),
                    Some(e) => last = e,
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
    // Prefer the four core static faces; a variable font ships as a
    // single file and is taken as it comes. Match on the exact style
    // suffix after the last dash: substring matching is a trap
    // ("SemiBold" contains "Bold", "Thin" is nobody's Regular).
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
    if !core.is_empty() {
        files = core;
    }
    files.sort();
    files.dedup();
    files.truncate(4);

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
}
