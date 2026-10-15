//! The Publisher bridge (docs/EXTRACT.md): native `.pub` reading
//! stays out of scope. Instead, LibreOffice (which reads `.pub`
//! through its own importer) converts the file to PDF headlessly,
//! and from there the ordinary PDF rescue takes over: dump,
//! render, draft. LibreOffice is a separate program the office
//! installs for free; this module only finds it and runs it.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// How long a conversion may run before we give up on it.
/// Publisher files are small; two minutes is generous.
pub const CONVERT_TIMEOUT: Duration = Duration::from_secs(120);

/// Well-known LibreOffice locations for an OS, best first.
/// Pure: the caller decides what "exists" means, so this is
/// testable without the program installed.
pub fn soffice_candidates(os: &str) -> Vec<PathBuf> {
    match os {
        "windows" => vec![
            PathBuf::from(r"C:\Program Files\LibreOffice\program\soffice.exe"),
            PathBuf::from(r"C:\Program Files (x86)\LibreOffice\program\soffice.exe"),
        ],
        "macos" => vec![PathBuf::from(
            "/Applications/LibreOffice.app/Contents/MacOS/soffice",
        )],
        "linux" => vec![PathBuf::from("/usr/bin/soffice")],
        _ => Vec::new(),
    }
}

/// Names to look up on PATH, best first.
pub const PATH_NAMES: [&str; 2] = ["soffice", "libreoffice"];

/// The first candidate that exists, if any.
pub fn first_existing(candidates: &[PathBuf], exists: &dyn Fn(&Path) -> bool) -> Option<PathBuf> {
    candidates.iter().find(|p| exists(p)).cloned()
}

/// Find a name on the PATH environment variable.
fn path_lookup(name: &str) -> Option<PathBuf> {
    let paths = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&paths) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
        if cfg!(windows) {
            let exe = dir.join(format!("{name}.exe"));
            if exe.is_file() {
                return Some(exe);
            }
        }
    }
    None
}

/// Locate LibreOffice: $OB_SOFFICE first (taken as given), then
/// the well-known install locations for this OS, then PATH.
pub fn find_soffice() -> Option<PathBuf> {
    find_soffice_from(
        std::env::var_os("OB_SOFFICE").map(PathBuf::from),
        std::env::consts::OS,
        &|p: &Path| p.exists(),
        &path_lookup,
    )
}

/// The composition behind find_soffice, with every input
/// injectable so the order can be tested.
pub fn find_soffice_from(
    env_override: Option<PathBuf>,
    os: &str,
    exists: &dyn Fn(&Path) -> bool,
    path_lookup: &dyn Fn(&str) -> Option<PathBuf>,
) -> Option<PathBuf> {
    if let Some(p) = env_override {
        return Some(p);
    }
    if let Some(p) = first_existing(&soffice_candidates(os), exists) {
        return Some(p);
    }
    for name in PATH_NAMES {
        if let Some(p) = path_lookup(name) {
            return Some(p);
        }
    }
    None
}

/// Where a converted PDF lands: LibreOffice names it after the
/// source file, in the output folder.
pub fn expected_pdf_path(pub_path: &Path, out_dir: &Path) -> PathBuf {
    let stem = pub_path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "bulletin".into());
    out_dir.join(format!("{stem}.pdf"))
}

/// Convert a .pub file to PDF with LibreOffice, headless, into
/// out_dir (created if needed). Returns the PDF's path. A private
/// profile folder inside out_dir keeps the conversion from
/// clashing with a LibreOffice window the office already has
/// open, which otherwise makes the conversion refuse to run.
pub fn convert_pub_to_pdf(
    soffice: &Path,
    pub_path: &Path,
    out_dir: &Path,
) -> Result<PathBuf, String> {
    convert_pub_to_pdf_with_timeout(soffice, pub_path, out_dir, CONVERT_TIMEOUT)
}

pub fn convert_pub_to_pdf_with_timeout(
    soffice: &Path,
    pub_path: &Path,
    out_dir: &Path,
    timeout: Duration,
) -> Result<PathBuf, String> {
    std::fs::create_dir_all(out_dir)
        .map_err(|e| format!("cannot create the work folder {}: {e}", out_dir.display()))?;
    let log_path = out_dir.join("libreoffice-output.txt");
    let log = std::fs::File::create(&log_path)
        .map_err(|e| format!("cannot open a log file in {}: {e}", out_dir.display()))?;
    let log_err = log
        .try_clone()
        .map_err(|e| format!("cannot open a log file in {}: {e}", out_dir.display()))?;

    // LibreOffice wants its profile as a file URL.
    let profile_dir = out_dir.join("lo-profile");
    let profile_url = format!(
        "file:///{}",
        profile_dir
            .to_string_lossy()
            .replace('\\', "/")
            .trim_start_matches('/')
    );

    let mut child = Command::new(soffice)
        .arg(format!("-env:UserInstallation={profile_url}"))
        .arg("--headless")
        .arg("--convert-to")
        .arg("pdf")
        .arg("--outdir")
        .arg(out_dir)
        .arg(pub_path)
        .stdin(Stdio::null())
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(log_err))
        .spawn()
        .map_err(|e| {
            format!(
                "LibreOffice could not be started ({}): {e}. \
                 If it is not installed, it is free at libreoffice.org.",
                soffice.display()
            )
        })?;

    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!(
                        "LibreOffice took longer than {} seconds to convert that file, \
                         so the conversion was stopped. The file may be damaged, or \
                         very large.",
                        timeout.as_secs()
                    ));
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => {
                return Err(format!(
                    "something went wrong while LibreOffice was converting: {e}"
                ))
            }
        }
    };

    let pdf = expected_pdf_path(pub_path, out_dir);
    if status.success() && pdf.is_file() {
        return Ok(pdf);
    }
    let said = std::fs::read_to_string(&log_path)
        .unwrap_or_default()
        .trim()
        .to_string();
    let tail: String = said
        .lines()
        .rev()
        .take(2)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join(" ");
    Err(format!(
        "LibreOffice could not turn that Publisher file into a PDF. \
         The file may be damaged, or made by a much newer Publisher than \
         LibreOffice understands.{}",
        if tail.is_empty() {
            String::new()
        } else {
            format!(" It said: {tail}")
        }
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_candidates_cover_both_program_files() {
        let c = soffice_candidates("windows");
        assert_eq!(c.len(), 2);
        assert!(c[0]
            .to_string_lossy()
            .contains("Program Files\\LibreOffice"));
        assert!(c[1]
            .to_string_lossy()
            .contains("Program Files (x86)\\LibreOffice"));
        assert!(c
            .iter()
            .all(|p| p.to_string_lossy().ends_with("soffice.exe")));
    }

    #[test]
    fn macos_and_linux_candidates_are_the_standard_places() {
        assert_eq!(
            soffice_candidates("macos"),
            vec![PathBuf::from(
                "/Applications/LibreOffice.app/Contents/MacOS/soffice"
            )]
        );
        assert_eq!(
            soffice_candidates("linux"),
            vec![PathBuf::from("/usr/bin/soffice")]
        );
        assert!(soffice_candidates("plan9").is_empty());
    }

    #[test]
    fn find_order_is_env_then_install_then_path() {
        let no_path = |_: &str| None;
        // Env override wins even when nothing exists.
        let found = find_soffice_from(
            Some(PathBuf::from("/custom/soffice")),
            "linux",
            &|_| false,
            &no_path,
        );
        assert_eq!(found, Some(PathBuf::from("/custom/soffice")));
        // Install location beats PATH.
        let fake_path = |name: &str| -> Option<PathBuf> {
            if name == "soffice" {
                Some(PathBuf::from("/from/path/soffice"))
            } else {
                None
            }
        };
        let found = find_soffice_from(None, "linux", &|_| true, &fake_path);
        assert_eq!(found, Some(PathBuf::from("/usr/bin/soffice")));
        // PATH is the fallback; "libreoffice" is tried too.
        let lo_only = |name: &str| -> Option<PathBuf> {
            if name == "libreoffice" {
                Some(PathBuf::from("/usr/bin/libreoffice"))
            } else {
                None
            }
        };
        let found = find_soffice_from(None, "linux", &|_| false, &lo_only);
        assert_eq!(found, Some(PathBuf::from("/usr/bin/libreoffice")));
        // Nothing anywhere: None.
        assert_eq!(find_soffice_from(None, "linux", &|_| false, &no_path), None);
    }

    #[test]
    fn expected_pdf_keeps_the_file_name() {
        assert_eq!(
            expected_pdf_path(Path::new("/church/Easter.pub"), Path::new("/tmp/work")),
            PathBuf::from("/tmp/work/Easter.pdf")
        );
        assert_eq!(
            expected_pdf_path(Path::new("bulletin.pub"), Path::new("out")),
            PathBuf::from("out/bulletin.pdf")
        );
    }

    #[test]
    fn missing_soffice_is_a_plain_words_error() {
        // The error the office sees when the program is not
        // really there must say so plainly (this sandbox has no
        // LibreOffice, and the test must not need one).
        let dir =
            std::env::temp_dir().join(format!("openbulletin-pub-test-{}", std::process::id()));
        let err = convert_pub_to_pdf(
            Path::new("/definitely/not/installed/soffice"),
            Path::new("bulletin.pub"),
            &dir,
        )
        .expect_err("a missing program cannot convert");
        assert!(err.contains("could not be started"), "{err}");
        assert!(err.contains("libreoffice.org"), "{err}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
