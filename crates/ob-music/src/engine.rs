//! The engine room: the code the helper binary runs.
//! It finds the engraver on this computer, drives it by
//! the spike's measured method, cuts the page render into
//! systems, measures the staff in what came back, and
//! refuses anything below the legibility floor.
//!
//! Backends, in order:
//! 1. a Verovio command-line program: $OB_VEROVIO_BIN,
//!    then `verovio` beside the helper, then on the PATH.
//!    Driven with its documented long options, which map
//!    one-to-one onto the toolkit options the spike used.
//! 2. the Verovio Python wheel (the official distribution
//!    on Windows, where no standalone CLI binary is
//!    published): a Python with the verovio module, from
//!    $OB_VEROVIO_PYTHON, then a `verovio-python` folder
//!    beside the helper, then python3/python on the PATH
//!    that can import it. This is also the development
//!    route: the spike's venv supplies it via the env var.
//!
//! The options are only trusted once measured: the Python
//! driver reads its options back (a JSON string passed
//! where a dict belongs fails silently to defaults, the
//! spike's gotcha), and the staff size is measured in the
//! returned SVG on every render, whichever backend drew.

use crate::render::{RenderedSystem, FLOOR_STAFF_MM, PROTOCOL};
use crate::svg;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Deserialize)]
pub struct EngineRequest {
    pub protocol: u32,
    pub command: String,
    #[serde(default)]
    pub musicxml: String,
    #[serde(default)]
    pub width_mm: f64,
    #[serde(default)]
    pub staff_mm: f64,
}

#[derive(Debug, Serialize)]
pub struct EngineResponse {
    pub protocol: u32,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub systems: Vec<RenderedSystem>,
}

impl EngineResponse {
    fn fail(kind: &str, why: impl Into<String>) -> EngineResponse {
        EngineResponse {
            protocol: PROTOCOL,
            ok: false,
            kind: Some(kind.to_string()),
            error: Some(why.into()),
            backend: None,
            systems: Vec::new(),
        }
    }
    fn unavailable() -> EngineResponse {
        EngineResponse::fail("unavailable", "the music engraver is not on this computer")
    }
}

enum Backend {
    Cli(PathBuf),
    Python(PathBuf),
}

impl Backend {
    fn label(&self) -> &'static str {
        match self {
            Backend::Cli(_) => "Verovio (program)",
            Backend::Python(_) => "Verovio (Python)",
        }
    }
}

/// The page height asked for, in page units: tall enough
/// that a hymn's systems all land on one page (they are
/// cut apart here), short enough that the rasterizer
/// stays sane. A score longer than this still renders:
/// the Python backend paginates and every page is cut;
/// the CLI backend's single page would then hold fewer
/// systems, which the count comparison reports openly.
fn page_height_units(page_width_units: u32) -> u32 {
    page_width_units.saturating_mul(24).max(20_000)
}

/// Why a backend run did not finish, kept apart so
/// the render paths can word a timeout as a timeout
/// instead of as a program that would not start.
enum CaptureFault {
    Start(String),
    TimedOut,
    TooMuchOutput,
}

/// Run a backend program to completion or to the
/// standard deadline, through the shared runner
/// (audit M9, 2026-10-16): the engraver behind the
/// helper once waited forever too, probes included.
fn run_capture(
    cmd: &mut Command,
    stdin: Option<&[u8]>,
) -> Result<(bool, String, String), CaptureFault> {
    run_capture_with_timeout(cmd, stdin, ob_runner::HELPER_TIMEOUT)
}

fn run_capture_with_timeout(
    cmd: &mut Command,
    stdin: Option<&[u8]>,
    timeout: std::time::Duration,
) -> Result<(bool, String, String), CaptureFault> {
    match ob_runner::run_with_timeout(cmd, stdin, timeout) {
        Ok(out) => Ok((
            out.status.success(),
            String::from_utf8_lossy(&out.stdout).into_owned(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
        )),
        Err(ob_runner::RunFault::Spawn(e)) => Err(CaptureFault::Start(e.to_string())),
        Err(ob_runner::RunFault::Wait(e)) => Err(CaptureFault::Start(e.to_string())),
        Err(ob_runner::RunFault::TimedOut) => Err(CaptureFault::TimedOut),
        Err(ob_runner::RunFault::TooMuchOutput) => Err(CaptureFault::TooMuchOutput),
    }
}

/// How a failed backend start reads in the refusal
/// the helper sends back.
fn start_failure(fault: CaptureFault) -> EngineResponse {
    match fault {
        CaptureFault::Start(e) => {
            EngineResponse::fail("failed", format!("the engraver would not start ({e})"))
        }
        CaptureFault::TimedOut => EngineResponse::fail(
            "failed",
            "the engraver did not finish in two minutes, so the music was not drawn",
        ),
        CaptureFault::TooMuchOutput => EngineResponse::fail(
            "failed",
            "the engraver answered with more than we can hold, so the music was not drawn",
        ),
    }
}

/// Probe a Verovio CLI: it answers --version.
fn cli_works(bin: &Path) -> Option<String> {
    let (ok, stdout, _) = run_capture(Command::new(bin).arg("--version"), None).ok()?;
    if ok && stdout.to_lowercase().contains("verovio") {
        Some(
            stdout
                .trim()
                .lines()
                .next()
                .unwrap_or("Verovio")
                .to_string(),
        )
    } else {
        None
    }
}

/// Probe a Python: it can import the verovio wheel.
fn python_works(py: &Path) -> Option<String> {
    let (ok, stdout, _) = run_capture(
        Command::new(py)
            .arg("-c")
            .arg("import verovio; print(verovio.toolkit().getVersion())"),
        None,
    )
    .ok()?;
    if ok {
        let v = stdout.trim();
        if v.is_empty() {
            Some("Verovio".to_string())
        } else {
            Some(format!("Verovio {v}"))
        }
    } else {
        None
    }
}

fn find_backend() -> Option<(Backend, String)> {
    // 1. The command-line program.
    let mut cli_candidates: Vec<PathBuf> = Vec::new();
    if let Ok(p) = std::env::var("OB_VEROVIO_BIN") {
        cli_candidates.push(PathBuf::from(p));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            cli_candidates.push(dir.join("verovio"));
            if cfg!(windows) {
                cli_candidates.push(dir.join("verovio.exe"));
            }
        }
    }
    cli_candidates.push(PathBuf::from("verovio"));
    for cand in &cli_candidates {
        if let Some(version) = cli_works(cand) {
            return Some((Backend::Cli(cand.clone()), version));
        }
    }
    // 2. A Python carrying the wheel.
    let mut py_candidates: Vec<PathBuf> = Vec::new();
    if let Ok(p) = std::env::var("OB_VEROVIO_PYTHON") {
        py_candidates.push(PathBuf::from(p));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let folder = dir.join("verovio-python");
            for rel in [
                "python.exe",
                "Scripts/python.exe",
                "bin/python3",
                "bin/python",
            ] {
                py_candidates.push(folder.join(rel));
            }
        }
    }
    py_candidates.push(PathBuf::from("python3"));
    py_candidates.push(PathBuf::from("python"));
    for cand in &py_candidates {
        if let Some(version) = python_works(cand) {
            return Some((Backend::Python(cand.clone()), version));
        }
    }
    None
}

/// The Python driver, written to a temp file per render.
/// It passes the options as a DICT (a JSON string would
/// fail silently back to defaults) and reads the page
/// width back to prove the settings took.
const DRIVER: &str = r#"import sys, json

def main():
    req = json.load(sys.stdin)
    try:
        import verovio
    except Exception as e:
        print(json.dumps({"ok": False, "kind": "unavailable",
                          "error": "the engraver's Python module is not on this computer (%s)" % e}))
        return
    tk = verovio.toolkit()
    opts = {
        "pageWidth": int(req["page_width"]),
        "pageHeight": int(req["page_height"]),
        "pageMarginTop": 0,
        "pageMarginBottom": 0,
        "pageMarginLeft": 0,
        "pageMarginRight": 0,
        "header": "none",
        "footer": "none",
        "breaks": "auto",
        "spacingSystem": int(req.get("spacing_system", 12)),
        "adjustPageHeight": True,
        "scale": 100,
    }
    if not tk.setOptions(opts):
        print(json.dumps({"ok": False, "error": "the engraver refused its settings"}))
        return
    got = tk.getOptions()
    if isinstance(got, str):
        got = json.loads(got)
    if int(got.get("pageWidth", -1)) != int(req["page_width"]):
        print(json.dumps({"ok": False, "error": "the engraver did not take the page width setting"}))
        return
    with open(req["musicxml_file"], "r", encoding="utf-8") as f:
        data = f.read()
    if not tk.loadData(data):
        print(json.dumps({"ok": False, "error": "the engraver could not read the music in that file"}))
        return
    pages = []
    for p in range(1, tk.getPageCount() + 1):
        pages.append(tk.renderToSVG(p))
    print(json.dumps({"ok": True, "version": tk.getVersion(), "pages": pages}))

main()
"#;

#[derive(Deserialize)]
struct DriverReply {
    ok: bool,
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    pages: Vec<String>,
}

/// Scratch files for one run, removed on drop.
struct Scratch {
    dir: PathBuf,
}

impl Scratch {
    fn new() -> Result<Scratch, String> {
        let dir = std::env::temp_dir().join(format!("openbulletin-verovio-{}", std::process::id()));
        std::fs::create_dir_all(&dir).map_err(|e| format!("cannot make a scratch folder ({e})"))?;
        Ok(Scratch { dir })
    }
    fn write(&self, name: &str, bytes: &[u8]) -> Result<PathBuf, String> {
        let path = self.dir.join(name);
        std::fs::write(&path, bytes).map_err(|e| format!("cannot write a scratch file ({e})"))?;
        Ok(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn render_via_cli(
    bin: &Path,
    xml: &str,
    page_width: u32,
    page_height: u32,
) -> Result<Vec<String>, EngineResponse> {
    let scratch = Scratch::new().map_err(|e| EngineResponse::fail("failed", e))?;
    let input = scratch
        .write("score.musicxml", xml.as_bytes())
        .map_err(|e| EngineResponse::fail("failed", e))?;
    let output = scratch.dir.join("page.svg");
    let mut cmd = Command::new(bin);
    cmd.arg("-t")
        .arg("svg")
        .arg("--page-width")
        .arg(page_width.to_string())
        .arg("--page-height")
        .arg(page_height.to_string())
        .arg("--page-margin-top")
        .arg("0")
        .arg("--page-margin-bottom")
        .arg("0")
        .arg("--page-margin-left")
        .arg("0")
        .arg("--page-margin-right")
        .arg("0")
        .arg("--header")
        .arg("none")
        .arg("--footer")
        .arg("none")
        .arg("--breaks")
        .arg("auto")
        .arg("--spacing-system")
        .arg("12")
        .arg("--adjust-page-height")
        .arg("-o")
        .arg(&output)
        .arg(&input);
    // A from-source Verovio keeps its glyph resources in a
    // data folder beside the program; name it when present.
    if let Some(dir) = bin.parent() {
        let data = dir.join("data");
        if data.is_dir() {
            cmd.arg("-r").arg(&data);
        }
    }
    let (ok, _stdout, stderr) = run_capture(&mut cmd, None).map_err(start_failure)?;
    if !ok {
        // One trimmed line of the engraver's own words
        // is quoted, never the whole wall.
        let tail = ob_runner::stderr_for_user(stderr.as_bytes());
        return Err(EngineResponse::fail(
            "failed",
            format!("the engraver could not draw that file ({tail})"),
        ));
    }
    let svg = std::fs::read_to_string(&output)
        .map_err(|_| EngineResponse::fail("failed", "the engraver drew nothing we can read"))?;
    Ok(vec![svg])
}

fn render_via_python(
    py: &Path,
    xml: &str,
    page_width: u32,
    page_height: u32,
) -> Result<Vec<String>, EngineResponse> {
    let scratch = Scratch::new().map_err(|e| EngineResponse::fail("failed", e))?;
    let input = scratch
        .write("score.musicxml", xml.as_bytes())
        .map_err(|e| EngineResponse::fail("failed", e))?;
    let driver = scratch
        .write("driver.py", DRIVER.as_bytes())
        .map_err(|e| EngineResponse::fail("failed", e))?;
    let req = serde_json::json!({
        "musicxml_file": input.to_string_lossy(),
        "page_width": page_width,
        "page_height": page_height,
        "spacing_system": 12,
    });
    let payload = serde_json::to_vec(&req).expect("driver request serializes");
    let (ok, stdout, stderr) =
        run_capture(Command::new(py).arg(&driver), Some(&payload)).map_err(start_failure)?;
    if !ok {
        let tail = ob_runner::stderr_for_user(stderr.as_bytes());
        return Err(EngineResponse::fail(
            "failed",
            format!("the engraver stopped unexpectedly ({tail})"),
        ));
    }
    let reply: DriverReply = serde_json::from_str(stdout.trim()).map_err(|_| {
        EngineResponse::fail("failed", "the engraver answered in a way we cannot read")
    })?;
    if !reply.ok {
        return Err(match reply.kind.as_deref() {
            Some("unavailable") => EngineResponse::unavailable(),
            _ => EngineResponse::fail(
                "failed",
                reply
                    .error
                    .unwrap_or_else(|| "the engraver could not draw that file".to_string()),
            ),
        });
    }
    Ok(reply.pages)
}

/// Run one engine command ("probe" or "render").
pub fn run(req: &EngineRequest) -> EngineResponse {
    // The protocol is compared before anything else
    // is read (audit M10, 2026-10-16): this helper
    // once answered requests from any version at all,
    // including 999 and 0, exactly as if they were its
    // own. A mismatch is a structured refusal the
    // client can repeat in words.
    if req.protocol != PROTOCOL {
        return EngineResponse::fail(
            "protocol",
            format!(
                "this copy of the music engraver helper speaks protocol {PROTOCOL}, \
                 but the request used protocol {}; the two do not match",
                req.protocol
            ),
        );
    }
    match req.command.as_str() {
        "probe" => match find_backend() {
            Some((backend, version)) => EngineResponse {
                protocol: PROTOCOL,
                ok: true,
                kind: None,
                error: None,
                backend: Some(format!("{version} via {}", backend.label())),
                systems: Vec::new(),
            },
            None => EngineResponse::unavailable(),
        },
        "render" => render(req),
        other => EngineResponse::fail("failed", format!("unknown engraver command {other:?}")),
    }
}

fn render(req: &EngineRequest) -> EngineResponse {
    let width_mm = if req.width_mm > 0.0 {
        req.width_mm
    } else {
        129.54
    };
    let target = if req.staff_mm > 0.0 {
        req.staff_mm
    } else {
        crate::render::TARGET_STAFF_MM
    };
    if req.musicxml.trim().is_empty() {
        return EngineResponse::fail("failed", "no music was given to the engraver");
    }
    let Some((backend, version)) = find_backend() else {
        return EngineResponse::unavailable();
    };
    // The spike's formula: the staff comes out at
    // 72 x width / pageWidth grid units of 720, so the
    // page width that lands the target is derived, not
    // guessed. The measurement below still rules.
    let page_width = ((72.0 * width_mm / target).round() as u32).max(100);
    let page_height = page_height_units(page_width);
    let pages = match &backend {
        Backend::Cli(bin) => render_via_cli(bin, &req.musicxml, page_width, page_height),
        Backend::Python(py) => render_via_python(py, &req.musicxml, page_width, page_height),
    };
    let pages = match pages {
        Ok(p) => p,
        Err(resp) => return resp,
    };
    let mut systems: Vec<svg::SystemSvg> = Vec::new();
    for page in &pages {
        match svg::split_systems(page, width_mm) {
            Ok(mut cut) => systems.append(&mut cut),
            Err(why) => {
                if pages.len() == 1 {
                    return EngineResponse::fail("failed", why);
                }
                // A later page that comes back empty (a
                // trailing blank) is skipped; the count
                // comparison will say if anything is lost.
                continue;
            }
        }
    }
    if systems.is_empty() {
        return EngineResponse::fail(
            "failed",
            "the engraver drew no lines of music from that file",
        );
    }
    if let Some(s) = systems.iter().find(|s| s.staff_mm < FLOOR_STAFF_MM - 0.02) {
        return EngineResponse::fail(
            "floor",
            format!(
                "the engraver drew the notes too small to sing from (staff {:.1} mm; \
                 nothing under {:.1} mm is ever placed)",
                s.staff_mm, FLOOR_STAFF_MM
            ),
        );
    }
    EngineResponse {
        protocol: PROTOCOL,
        ok: true,
        kind: None,
        error: None,
        backend: Some(format!("{version} via {}", backend.label())),
        systems: systems
            .into_iter()
            .map(|s| RenderedSystem {
                svg: s.svg,
                width_units: s.width_units,
                height_units: s.height_units,
                staff_mm: s.staff_mm,
                noteheads: s.counts.noteheads,
                measures: s.counts.measures,
                syllables: s.counts.syllables,
                fermatas: s.counts.fermatas,
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_request_in_another_protocol_is_refused_as_protocol() {
        // The red team's live probe (audit M10): the
        // helper accepted protocol 999 and 0 and
        // answered normally. Both now stop at the gate.
        for v in [999, 0] {
            let resp = run(&EngineRequest {
                protocol: v,
                command: "probe".into(),
                musicxml: String::new(),
                width_mm: 0.0,
                staff_mm: 0.0,
            });
            assert!(!resp.ok);
            assert_eq!(resp.kind.as_deref(), Some("protocol"), "version {v}");
            assert_eq!(resp.protocol, PROTOCOL, "the answer speaks our version");
        }
    }

    #[cfg(unix)]
    #[test]
    fn a_backend_that_sleeps_meets_the_deadline() {
        let fault = run_capture_with_timeout(
            Command::new("sleep").arg("30"),
            None,
            std::time::Duration::from_secs(1),
        )
        .expect_err("a sleeping backend cannot finish");
        assert!(matches!(fault, CaptureFault::TimedOut));
    }

    #[test]
    fn empty_score_fails_in_words() {
        let resp = run(&EngineRequest {
            protocol: PROTOCOL,
            command: "render".into(),
            musicxml: String::new(),
            width_mm: 120.0,
            staff_mm: 6.5,
        });
        assert!(!resp.ok);
        assert!(resp.error.unwrap().contains("no music"));
    }

    #[test]
    fn probe_never_panics_without_an_engraver() {
        // On a machine with an engraver this answers ok;
        // without one it answers unavailable. Both are
        // shaped answers, never a crash.
        let resp = run(&EngineRequest {
            protocol: PROTOCOL,
            command: "probe".into(),
            musicxml: String::new(),
            width_mm: 0.0,
            staff_mm: 0.0,
        });
        assert_eq!(resp.protocol, PROTOCOL);
        assert!(resp.ok || resp.kind.as_deref() == Some("unavailable"));
    }
}
