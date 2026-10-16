//! The engraver helper protocol: how the app asks for
//! music to be drawn, and how it finds the helper.
//!
//! The engraving itself (Verovio, LGPL-3.0) runs in a
//! separate helper process, `ob-verovio-helper`, resolved
//! the same way as the PDFium helper: an environment
//! override, then beside the app, then the PATH. The
//! helper answers over versioned JSON on stdin/stdout, so
//! what the app sees is identical whichever engraver
//! backend the helper found (docs/LEGAL.md carries the
//! licensing posture).

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;

/// The protocol version both ends speak. Bump on any
/// incompatible change to the request or response shape.
pub const PROTOCOL: u32 = 1;

/// The staff size we ask for by default: big enough to
/// sing from without squinting (music-spike.md measured
/// the hymn range at 6.0 to 7.0 mm).
pub const TARGET_STAFF_MM: f64 = 6.5;

/// The legibility floor: no system is ever placed smaller
/// than this, however narrow the box. A render that comes
/// back below it is refused, never shrunk into the page.
pub const FLOOR_STAFF_MM: f64 = 5.0;

/// One engraved line of music, as the helper reports it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RenderedSystem {
    /// The system's standalone SVG.
    pub svg: String,
    /// Width in engraver page units (72 per inch).
    pub width_units: f64,
    /// Height in the same units.
    pub height_units: f64,
    /// The staff height MEASURED in the render, in mm at
    /// the requested width. Checked against the floor
    /// before any system is used.
    pub staff_mm: f64,
    pub noteheads: u32,
    pub measures: u32,
    pub syllables: u32,
    pub fermatas: u32,
}

/// A finished render: every system's drawing plus the
/// counts the review flow compares against the file.
#[derive(Debug, Clone, PartialEq)]
pub struct RenderOutcome {
    /// Which engraver drew it, in words ("Verovio 6.3.0").
    pub backend: String,
    pub systems: Vec<RenderedSystem>,
}

impl RenderOutcome {
    pub fn noteheads(&self) -> u32 {
        self.systems.iter().map(|s| s.noteheads).sum()
    }
    pub fn measures(&self) -> u32 {
        self.systems.iter().map(|s| s.measures).sum()
    }
    pub fn syllables(&self) -> u32 {
        self.systems.iter().map(|s| s.syllables).sum()
    }
    pub fn fermatas(&self) -> u32 {
        self.systems.iter().map(|s| s.fermatas).sum()
    }
}

/// Why a render did not happen, in the words the review
/// flow repeats.
#[derive(Debug, Clone, PartialEq)]
pub enum RenderError {
    /// The helper or the engraver behind it is not on this
    /// computer. The draft still forms; the notes are
    /// simply not drawn yet.
    NotAvailable(String),
    /// Something else went wrong: the render was refused
    /// or came back below the legibility floor.
    Failed(String),
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RenderError::NotAvailable(why) | RenderError::Failed(why) => write!(f, "{why}"),
        }
    }
}

impl std::error::Error for RenderError {}

#[derive(Serialize)]
struct Request {
    protocol: u32,
    command: &'static str,
    musicxml: String,
    width_mm: f64,
    staff_mm: f64,
}

#[derive(Deserialize)]
struct Response {
    protocol: u32,
    ok: bool,
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    backend: Option<String>,
    #[serde(default)]
    systems: Vec<RenderedSystem>,
}

/// Candidate names per OS, in the order probed for the
/// helper binary itself: env value verbatim, then beside
/// the running program (a from-source build is missed by
/// the PATH alone), then the bare name for the PATH.
/// Called with the override so tests can point elsewhere.
pub fn helper_candidates(explicit: Option<std::path::PathBuf>) -> Vec<PathBuf> {
    let names: &[&str] = if cfg!(windows) {
        &["ob-verovio-helper.exe", "ob-verovio-helper"]
    } else {
        &["ob-verovio-helper"]
    };
    let mut out: Vec<PathBuf> = Vec::new();
    if let Some(p) = explicit {
        out.push(p);
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for name in names {
                out.push(dir.join(name));
            }
        }
    }
    for name in names {
        out.push(PathBuf::from(name));
    }
    out
}

/// Render one score's systems at `width_mm` with a target
/// staff of `staff_mm`. The helper measures the staff in
/// what it drew; a system below the floor never comes back
/// as a success.
pub fn render_music(xml: &str, width_mm: f64, staff_mm: f64) -> Result<RenderOutcome, RenderError> {
    let explicit = std::env::var("OB_VEROVIO_HELPER").ok().map(PathBuf::from);
    let candidates = helper_candidates(explicit);
    render_with_candidates(
        xml,
        width_mm,
        staff_mm,
        &candidates,
        ob_extract::runner::HELPER_TIMEOUT,
    )
}

/// The render with its candidates and deadline showing,
/// so tests can point at fake helpers and short waits.
/// The calls go through the shared deadline runner
/// (audit M9, 2026-10-16): a helper that never answers
/// once wedged the music door forever, and the request
/// used to be written before stdout was drained, a
/// deadlock shape the runner's threads remove.
fn render_with_candidates(
    xml: &str,
    width_mm: f64,
    staff_mm: f64,
    candidates: &[PathBuf],
    timeout: std::time::Duration,
) -> Result<RenderOutcome, RenderError> {
    let req = Request {
        protocol: PROTOCOL,
        command: "render",
        musicxml: xml.to_string(),
        width_mm,
        staff_mm,
    };
    let payload = serde_json::to_string(&req).expect("request serializes");
    let mut last_spawn_err: Option<std::io::Error> = None;
    for cand in candidates {
        let mut cmd = Command::new(cand);
        let output =
            match ob_extract::runner::run_with_timeout(&mut cmd, Some(payload.as_bytes()), timeout)
            {
                Ok(o) => o,
                Err(ob_extract::runner::RunFault::Spawn(e)) => {
                    last_spawn_err = Some(e);
                    continue;
                }
                Err(ob_extract::runner::RunFault::TimedOut) => {
                    return Err(RenderError::Failed(format!(
                        "the music engraver helper did not answer in {}, so the music \
                     was not drawn",
                        ob_extract::runner::describe_timeout(timeout)
                    )));
                }
                Err(ob_extract::runner::RunFault::Wait(e)) => {
                    return Err(RenderError::Failed(format!(
                        "the music engraver helper stopped unexpectedly ({e})"
                    )));
                }
            };
        let text = String::from_utf8_lossy(&output.stdout);
        let resp: Response = serde_json::from_str(&text).map_err(|_| {
            RenderError::Failed(
                "the music engraver helper answered in a way we cannot read".to_string(),
            )
        })?;
        // The protocol is compared, not just carried
        // (audit M10, 2026-10-16): a helper answering in
        // a different version than the request spoke is
        // refused in words, never read as a render.
        if resp.protocol != PROTOCOL {
            return Err(RenderError::Failed(format!(
                "the music engraver helper answered in protocol version {}, but \
                 this copy of OpenBulletin speaks version {}; the two do not \
                 match, so the music was not drawn",
                resp.protocol, PROTOCOL
            )));
        }
        if resp.ok {
            let systems = resp.systems;
            if systems.is_empty() {
                return Err(RenderError::Failed(
                    "the music engraver drew no lines of music from that file".to_string(),
                ));
            }
            if let Some(s) = systems.iter().find(|s| s.staff_mm < FLOOR_STAFF_MM - 0.02) {
                return Err(RenderError::Failed(format!(
                    "the engraver drew the notes too small to sing from (staff {:.1} mm; \
                     nothing under {:.1} mm is ever placed)",
                    s.staff_mm, FLOOR_STAFF_MM
                )));
            }
            return Ok(RenderOutcome {
                backend: resp.backend.unwrap_or_else(|| "the music engraver".into()),
                systems,
            });
        }
        let why = resp
            .error
            .unwrap_or_else(|| "the music engraver could not draw that file".to_string());
        return Err(match resp.kind.as_deref() {
            Some("unavailable") => RenderError::NotAvailable(why),
            _ => RenderError::Failed(why),
        });
    }
    let _ = last_spawn_err;
    Err(RenderError::NotAvailable(format!(
        "the music engraver is not on this computer (looked for {} in {} places)",
        "ob-verovio-helper",
        candidates.len()
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_helper_is_not_available_not_a_crash() {
        // Point the override at a path that cannot exist and
        // rely on the beside-the-test-binary candidates
        // missing too in this environment.
        let err = render_music("<score-partwise/>", 120.0, TARGET_STAFF_MM).unwrap_err();
        // The dev machine may have a helper beside the test
        // binary; accept either the not-available verdict or
        // a real render failure from an empty score, but
        // never a panic or a success.
        match err {
            RenderError::NotAvailable(_) | RenderError::Failed(_) => {}
        }
    }

    /// The whole M1 path the app walks, against the spike's
    /// Bach fixture and a real engraver: parse, render
    /// through the helper binary, build the draft, and the
    /// count comparison at full marks. Runs only when the
    /// development engraver is configured
    /// (OB_VEROVIO_PYTHON) and the fixture is on this
    /// machine; otherwise it skips, like every fixture test.
    #[test]
    fn end_to_end_bach_when_the_dev_engraver_is_configured() {
        if std::env::var("OB_VEROVIO_PYTHON").is_err() {
            eprintln!("OB_VEROVIO_PYTHON unset; skipping the live engraver test");
            return;
        }
        // The test binary lives in target/<profile>/deps;
        // the helper binary is built one level up. Point
        // the resolver at it the way the bundle points the
        // app at the helper beside it.
        if std::env::var("OB_VEROVIO_HELPER").is_err() {
            if let Ok(exe) = std::env::current_exe() {
                if let Some(helper) = exe
                    .parent()
                    .and_then(|d| d.parent())
                    .map(|d| d.join("ob-verovio-helper"))
                {
                    if helper.exists() {
                        std::env::set_var("OB_VEROVIO_HELPER", &helper);
                    }
                }
            }
        }
        let path = concat!(
            env!("HOME"),
            "/workspace/openbulletin-fixtures/music-spike/ein-feste-burg.musicxml"
        );
        let Ok(bytes) = std::fs::read(path) else {
            eprintln!("spike fixture absent; skipping");
            return;
        };
        let (xml, info) = crate::parse::load_music(&bytes).expect("fixture loads");
        let out = render_music(&xml, crate::draft::music_frame_width_mm(), TARGET_STAFF_MM)
            .expect("the configured engraver renders");
        assert_eq!(out.noteheads(), 237);
        assert_eq!(out.measures(), 14);
        assert_eq!(out.fermatas(), 13);
        assert!(out.syllables() >= 58, "drew {}", out.syllables());
        assert!(
            out.systems
                .iter()
                .all(|s| (6.4..=6.6).contains(&s.staff_mm)),
            "every staff at the 6.5 mm target: {:?}",
            out.systems.iter().map(|s| s.staff_mm).collect::<Vec<_>>()
        );
        let draft = crate::draft::build_music_draft(
            &info,
            &bytes,
            "ein-feste-burg.musicxml",
            "application/vnd.recordare.musicxml+xml",
            ob_model::ChurchProfile::default(),
            Some(&out),
        );
        assert!(draft.items.iter().any(|i| {
            i.label == "Every note in the file is in the printed music"
                && i.badge == ob_extract::draft::Badge::Matched
        }));
        let music_boxes = draft
            .doc
            .panels
            .iter()
            .flat_map(|p| &p.frames)
            .filter(|f| f.music.is_some())
            .count();
        assert_eq!(music_boxes, out.systems.len());
    }

    /// A fake engraver helper: a small shell script in
    /// a scratch folder, executable (the red team's
    /// method, audit M9/M10, as regression tests).
    #[cfg(unix)]
    fn fake_helper(name: &str, body: &str) -> PathBuf {
        use std::os::unix::fs::PermissionsExt as _;
        let dir = std::env::temp_dir().join(format!(
            "openbulletin-music-test-{}-{name}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).expect("scratch dir");
        let path = dir.join(name);
        std::fs::write(&path, body).expect("write fake helper");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
            .expect("make it executable");
        path
    }

    #[cfg(unix)]
    fn fake_answer(protocol: u32) -> String {
        format!(
            "#!/bin/sh\ncat > /dev/null\nprintf '%s' '{{\"protocol\":{protocol},\"ok\":true,\
             \"backend\":\"Fake engraver\",\"systems\":[{{\"svg\":\"<svg/>\",\
             \"width_units\":100.0,\"height_units\":20.0,\"staff_mm\":6.5,\
             \"noteheads\":3,\"measures\":1,\"syllables\":2,\"fermatas\":0}}]}}'\n"
        )
    }

    #[cfg(unix)]
    #[test]
    fn a_good_fake_helper_renders_through_the_new_runner() {
        let helper = fake_helper("good", &fake_answer(PROTOCOL));
        let out = render_with_candidates(
            "<score-partwise/>",
            120.0,
            TARGET_STAFF_MM,
            &[helper],
            std::time::Duration::from_secs(10),
        )
        .expect("a matching answer renders");
        assert_eq!(out.systems.len(), 1);
        assert_eq!(out.noteheads(), 3);
    }

    #[cfg(unix)]
    #[test]
    fn a_helper_speaking_another_protocol_is_refused_in_words() {
        // The red team's staged fake (audit M10):
        // protocol 999, ok true, a floor-passing system.
        // It used to be accepted as a genuine render.
        let helper = fake_helper("future", &fake_answer(999));
        let err = render_with_candidates(
            "<score-partwise/>",
            120.0,
            TARGET_STAFF_MM,
            &[helper],
            std::time::Duration::from_secs(10),
        )
        .expect_err("protocol 999 is not a render");
        match err {
            RenderError::Failed(why) => {
                assert!(why.contains("protocol version 999"), "{why}")
            }
            other => panic!("expected a plain refusal, got {other:?}"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn a_sleeping_helper_meets_the_deadline() {
        let helper = fake_helper("sleepy", "#!/bin/sh\ncat > /dev/null\nsleep 30\n");
        let start = std::time::Instant::now();
        let err = render_with_candidates(
            "<score-partwise/>",
            120.0,
            TARGET_STAFF_MM,
            &[helper],
            std::time::Duration::from_secs(1),
        )
        .expect_err("a sleeping helper cannot render");
        match err {
            RenderError::Failed(why) => {
                assert!(why.contains("did not answer in 1 seconds"), "{why}")
            }
            other => panic!("expected a timeout refusal, got {other:?}"),
        }
        assert!(
            start.elapsed() < std::time::Duration::from_secs(15),
            "the deadline, not the sleep, ends the wait"
        );
    }

    #[test]
    fn candidates_lead_with_the_override() {
        let c = helper_candidates(Some(PathBuf::from("/x/ob-verovio-helper")));
        assert_eq!(c[0], PathBuf::from("/x/ob-verovio-helper"));
    }
}
