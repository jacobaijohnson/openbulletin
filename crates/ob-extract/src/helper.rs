//! Running the PDFium helper: the one door through which foreign
//! PDFs are read (docs/EXTRACT.md). The helper is a separate
//! process on purpose; a bad PDF can hurt it, never the app.

use crate::ExtractionDump;
use std::path::{Path, PathBuf};

/// Find the helper binary: explicit path, $OB_PDFIUM_HELPER,
/// beside the running executable, then PATH.
pub fn resolve_helper(explicit: Option<&Path>) -> PathBuf {
    if let Some(p) = explicit {
        return p.to_path_buf();
    }
    if let Ok(p) = std::env::var("OB_PDFIUM_HELPER") {
        return PathBuf::from(p);
    }
    if let Ok(exe) = std::env::current_exe() {
        // The shipped file carries its platform name: on Windows
        // that is ob-pdfium-helper.exe. Probing only the bare
        // name can never match there (stub sweep, 2026-10-15).
        for name in ["ob-pdfium-helper", "ob-pdfium-helper.exe"] {
            let sibling = exe.with_file_name(name);
            if sibling.exists() {
                return sibling;
            }
        }
    }
    PathBuf::from("ob-pdfium-helper")
}

fn run_helper(helper: &Path, args: &[&str]) -> Result<Vec<u8>, String> {
    run_helper_with_timeout(helper, args, crate::runner::HELPER_TIMEOUT)
}

/// The helper run with its deadline showing, so tests
/// can use a short one. The helper once ran with no
/// deadline at all and a stuck helper wedged the
/// import forever (audit M9, 2026-10-16); the shared
/// runner now carries the deadline and the pipes.
fn run_helper_with_timeout(
    helper: &Path,
    args: &[&str],
    timeout: std::time::Duration,
) -> Result<Vec<u8>, String> {
    let mut cmd = std::process::Command::new(helper);
    cmd.args(args);
    let output = match crate::runner::run_with_timeout(&mut cmd, None, timeout) {
        Ok(o) => o,
        Err(crate::runner::RunFault::Spawn(e)) => {
            return Err(if e.kind() == std::io::ErrorKind::NotFound {
                "the PDF reader is not with the app on this computer, so PDFs \
                 cannot be read here yet. It travels in the same download as \
                 the app itself; nothing else on this computer is broken."
                    .to_string()
            } else {
                format!("the PDF reader is here but could not start: {e}")
            });
        }
        Err(crate::runner::RunFault::TimedOut) => {
            return Err(format!(
                "the PDF reader did not answer in {}, so that file was not read",
                crate::runner::describe_timeout(timeout)
            ));
        }
        Err(crate::runner::RunFault::Wait(e)) => {
            return Err(format!(
                "something went wrong while the PDF reader was working: {e}"
            ));
        }
    };
    if !output.status.success() {
        // Only a trimmed first line of the helper's own
        // words is ever quoted back (runner::stderr_for_user).
        let said = crate::runner::stderr_for_user(&output.stderr);
        return Err(if said.is_empty() {
            "the PDF reader could not read that file".to_string()
        } else {
            format!("the PDF reader could not read that file: {said}")
        });
    }
    Ok(output.stdout)
}

/// Extract the text dump from a PDF.
pub fn dump_pdf(helper: &Path, pdf: &Path) -> Result<ExtractionDump, String> {
    let bytes = run_helper(helper, &[&pdf.to_string_lossy()])?;
    let text =
        String::from_utf8(bytes).map_err(|e| format!("the PDF reader answered oddly: {e}"))?;
    ExtractionDump::from_json(&text)
}

/// Extract the dump AND the page pictures (schema v2's
/// image channel): the helper writes each picture as a PNG
/// into `images_dir`, and the dump's `images` entries name
/// the files, ready for the draft builder to read into the
/// document's asset store.
pub fn dump_pdf_with_images(
    helper: &Path,
    pdf: &Path,
    images_dir: &Path,
) -> Result<ExtractionDump, String> {
    let bytes = run_helper(
        helper,
        &[&pdf.to_string_lossy(), &images_dir.to_string_lossy()],
    )?;
    let text =
        String::from_utf8(bytes).map_err(|e| format!("the PDF reader answered oddly: {e}"))?;
    ExtractionDump::from_json(&text)
}

/// Render each page to out_dir/page-NN.png (about 200 dpi), the
/// tracing pictures a rescue draft carries.
pub fn render_pdf(helper: &Path, pdf: &Path, out_dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(out_dir)
        .map_err(|e| format!("cannot create {}: {e}", out_dir.display()))?;
    run_helper(
        helper,
        &["render", &pdf.to_string_lossy(), &out_dir.to_string_lossy()],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    /// A fake helper: a small shell script in a scratch
    /// folder, executable. The red team's method (audit
    /// M9/M10) turned into regression tests.
    #[cfg(unix)]
    fn fake_helper(name: &str, body: &str) -> PathBuf {
        use std::os::unix::fs::PermissionsExt as _;
        let dir = std::env::temp_dir().join(format!(
            "openbulletin-helper-test-{}-{name}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).expect("scratch dir");
        let path = dir.join(name);
        std::fs::write(&path, body).expect("write fake helper");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
            .expect("make it executable");
        path
    }

    const GOOD_DUMP: &str = "{\"schema_version\":2,\"source_kind\":\"pdf\",\
        \"source_name\":\"fake.pdf\",\"source_bytes\":3,\"extractor\":\"fake\",\
        \"pages\":[],\"spans\":[]}";

    #[cfg(unix)]
    #[test]
    fn a_good_fake_helper_still_answers_through_the_new_runner() {
        let helper = fake_helper("good", &format!("#!/bin/sh\nprintf '%s' '{GOOD_DUMP}'\n"));
        let dump = dump_pdf(&helper, Path::new("whatever.pdf")).expect("the dump parses");
        assert_eq!(dump.schema_version, 2);
        assert!(dump.pages.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn a_sleeping_helper_meets_the_deadline() {
        let helper = fake_helper("sleepy", "#!/bin/sh\nsleep 30\n");
        let start = Instant::now();
        let err = run_helper_with_timeout(&helper, &["x.pdf"], Duration::from_secs(1))
            .expect_err("a sleeping helper cannot succeed");
        assert!(err.contains("did not answer in 1 seconds"), "{err}");
        assert!(
            start.elapsed() < Duration::from_secs(15),
            "the deadline, not the sleep, ends the wait"
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_helper_from_the_future_is_refused_in_words() {
        // The red team's other live repro (audit M10):
        // a fake answering schema_version 999 was read
        // as a real dump. Through dump_pdf it now stops
        // at the version check inside from_json.
        let body = GOOD_DUMP.replace("\"schema_version\":2", "\"schema_version\":999");
        let helper = fake_helper("future", &format!("#!/bin/sh\nprintf '%s' '{body}'\n"));
        let err =
            dump_pdf(&helper, Path::new("whatever.pdf")).expect_err("version 999 is not readable");
        assert!(err.contains("newer format (version 999)"), "{err}");
    }

    #[cfg(unix)]
    #[test]
    fn helper_stderr_is_quoted_trimmed_to_one_line() {
        // Multi-line garbage on stderr, and a first line
        // longer than the cap: the person sees one
        // short line, never the wall (audit helpers
        // MINOR, 2026-10-16).
        let shout = "z".repeat(300);
        let script = format!(
            "#!/bin/sh\necho '{shout}' >&2\necho 'second line junk' >&2\necho 'third line' >&2\nexit 3\n"
        );
        let helper = fake_helper("noisy", &script);
        let err = run_helper(&helper, &["x.pdf"]).expect_err("exit 3 is a failure");
        assert!(
            err.starts_with("the PDF reader could not read that file: z"),
            "{err}"
        );
        assert!(!err.contains("second line junk"), "{err}");
        assert!(err.chars().count() <= 245, "the quote is capped: {err}");
    }
}
