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
        let sibling = exe.with_file_name("ob-pdfium-helper");
        if sibling.exists() {
            return sibling;
        }
    }
    PathBuf::from("ob-pdfium-helper")
}

fn run_helper(helper: &Path, args: &[&str]) -> Result<Vec<u8>, String> {
    let output = std::process::Command::new(helper)
        .args(args)
        .output()
        .map_err(|e| {
            format!(
                "the PDF reader is not with the app on this computer, so PDFs \
                 cannot be read here yet. It travels in the same download as \
                 the app itself; nothing else on this computer is broken."
            )
        })?;
    if !output.status.success() {
        return Err(format!(
            "the PDF reader could not read that file: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
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
