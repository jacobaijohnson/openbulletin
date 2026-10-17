//! Running a helper program with a deadline: the one
//! spawn pattern every out-of-process helper goes
//! through (the PDFium reader, the music engraver
//! client, and the engravers behind the music helper).
//!
//! Before this module, only the LibreOffice bridge
//! (pub_bridge) ran its child with a deadline; every
//! other call blocked forever, so a stuck helper wedged
//! the import that called it, with no way to say why
//! (audit M9, 2026-10-16). The bridge keeps its own
//! loop because it also manages a log file and a
//! profile folder; everything that talks over pipes
//! comes through here.
//!
//! The shape matters as much as the deadline: the
//! request is written to the child's stdin on its own
//! thread and the handle dropped (stdin closed),
//! while stdout and stderr drain on their own threads
//! into a channel the caller collects from. A helper
//! that answers a lot before reading its request can
//! therefore never deadlock against a full pipe, and
//! a silent one cannot hang the caller: past the
//! deadline the child is killed and the caller hears
//! about it in time to say so in plain words. On Unix
//! the child runs in its own process group and the
//! whole group is stopped, so a wrapper script's own
//! children are stopped with it.
//!
//! The reader threads are never joined (round 2,
//! RT-1). A pipe reaches EOF only when EVERY process
//! holding it lets go, and a helper that forks can
//! leave a grandchild holding stdout open long after
//! the helper itself has finished; joining the
//! readers made the caller wait for that grandchild's
//! whole life, deadline or no deadline. So the
//! readers are detached: they send what they read
//! over the channel and end at EOF, when the caller
//! has stopped listening, or when the process exits.
//! The caller waits for the pipes only until the
//! deadline plus a short grace; past that it reports
//! the run as timed out and walks away, and the
//! detached readers end whenever their pipes finally
//! close.
//!
//! Collected output has a ceiling too (round 2,
//! RT-8): MAX_OUTPUT_BYTES across both pipes
//! together. A helper flooding its pipes could
//! otherwise fill the machine's memory well inside
//! the deadline; past the ceiling the helper is
//! stopped and the run refused in plain words.

use std::io::Write as _;
use std::process::{Command, ExitStatus, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// How long any helper may take before it is stopped:
/// two minutes, the same patience the Publisher
/// bridge gives LibreOffice.
pub const HELPER_TIMEOUT: Duration = Duration::from_secs(120);

/// How much longer than the deadline the caller waits
/// for a finished (or stopped) helper's pipes to close
/// before it stops waiting and reports the run as
/// timed out (round 2, RT-1). Five seconds is a
/// moment for an honest pipe to drain; a pipe still
/// open after that is being held by something the
/// helper left behind, and its lifetime is not ours
/// to wait out.
const PIPE_GRACE: Duration = Duration::from_secs(5);

/// The most helper output one run will hold: 512 MB
/// across stdout and stderr together, the project's
/// standing total-cap discipline (round 2, RT-8).
/// Real answers are single-digit megabytes (a PDF
/// reading, an engraved page); past the cap the run
/// is refused, not stored.
pub const MAX_OUTPUT_BYTES: usize = 512 * 1024 * 1024;

/// A helper run that finished, whatever its exit
/// status was. Reading the status is the caller's
/// business; a refusal spoken in JSON is still a
/// finished run.
#[derive(Debug)]
pub struct RunOutput {
    pub status: ExitStatus,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

/// Why a helper run did not finish.
#[derive(Debug)]
pub enum RunFault {
    /// The program could not be started at all.
    Spawn(std::io::Error),
    /// Waiting on the program failed.
    Wait(std::io::Error),
    /// The deadline passed; the child was killed.
    /// Also the answer when the child finished but
    /// its pipes stayed open past the grace: something
    /// it started is still holding them, and the run
    /// is over either way.
    TimedOut,
    /// The helper answered with more than we can
    /// hold (past MAX_OUTPUT_BYTES); it was stopped.
    TooMuchOutput,
}

/// Run `cmd` to completion or to the deadline. The
/// caller builds the command (program and arguments);
/// the pipes are set here: `stdin_bytes` is written
/// and stdin closed, or stdin is /dev/null when None,
/// and stdout/stderr are always collected.
pub fn run_with_timeout(
    cmd: &mut Command,
    stdin_bytes: Option<&[u8]>,
    timeout: Duration,
) -> Result<RunOutput, RunFault> {
    if stdin_bytes.is_some() {
        cmd.stdin(Stdio::piped());
    } else {
        cmd.stdin(Stdio::null());
    }
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt as _;
        // Its own process group, so stop_child can stop
        // the whole family (see the module docs).
        cmd.process_group(0);
    }
    let mut child = cmd.spawn().map_err(RunFault::Spawn)?;
    // The request goes in on its own thread, which
    // drops the handle when it is done (stdin closed).
    // The thread is detached: a helper that never reads
    // cannot wedge the caller on a full stdin pipe, and
    // a failed write (the helper stopped early, closing
    // its end) is the helper's answer, carried by what
    // the readers below collect.
    if let (Some(mut sin), Some(bytes)) = (child.stdin.take(), stdin_bytes) {
        let bytes = bytes.to_vec();
        std::thread::spawn(move || {
            let _ = sin.write_all(&bytes);
        });
    }
    let (tx, rx) = mpsc::channel::<PipeChunk>();
    if let Some(so) = child.stdout.take() {
        spawn_pipe_reader(so, tx.clone(), PipeChunk::Out);
    }
    if let Some(se) = child.stderr.take() {
        spawn_pipe_reader(se, tx.clone(), PipeChunk::Err);
    }
    // The caller's own sender goes away here: the
    // channel disconnects when both readers end, and
    // that disconnection is how the caller learns the
    // pipes reached EOF.
    drop(tx);
    let started = Instant::now();
    let child_deadline = started + timeout;
    let collect_deadline = child_deadline + PIPE_GRACE;
    let mut status: Option<ExitStatus> = None;
    let mut timed_out = false;
    let mut pipes_done = false;
    let mut stdout: Vec<u8> = Vec::new();
    let mut stderr: Vec<u8> = Vec::new();
    loop {
        if status.is_none() && !timed_out {
            match child.try_wait() {
                Ok(Some(s)) => status = Some(s),
                Ok(None) => {
                    if Instant::now() >= child_deadline {
                        stop_child(&mut child);
                        timed_out = true;
                    }
                }
                Err(e) => {
                    stop_child(&mut child);
                    return Err(RunFault::Wait(e));
                }
            }
        }
        if pipes_done {
            if let Some(s) = status {
                return Ok(RunOutput {
                    status: s,
                    stdout,
                    stderr,
                });
            }
            if timed_out {
                return Err(RunFault::TimedOut);
            }
        }
        if Instant::now() >= collect_deadline {
            // The child missed its deadline, or it
            // finished but something it started is
            // still holding its pipes open past the
            // grace. Either way this run is a timeout,
            // returned WITHOUT waiting for the pipes:
            // the detached readers end when their pipes
            // finally close or the process exits.
            if status.is_none() && !timed_out {
                stop_child(&mut child);
            }
            return Err(RunFault::TimedOut);
        }
        match rx.recv_timeout(Duration::from_millis(10)) {
            Ok(chunk) => {
                match chunk {
                    PipeChunk::Out(data) => stdout.extend_from_slice(&data),
                    PipeChunk::Err(data) => stderr.extend_from_slice(&data),
                }
                if stdout.len() + stderr.len() > MAX_OUTPUT_BYTES {
                    stop_child(&mut child);
                    return Err(RunFault::TooMuchOutput);
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                pipes_done = true;
                // Nothing more can arrive; nap instead
                // of spinning while the child finishes.
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    }
}

/// Which pipe a chunk of helper output came down.
enum PipeChunk {
    Out(Vec<u8>),
    Err(Vec<u8>),
}

/// Drain one pipe on its own thread, sending what it
/// reads to the caller in chunks. The thread is
/// detached and never joined (see the module docs):
/// it ends at pipe EOF, when the caller has stopped
/// listening (the send fails), or when the process
/// exits.
fn spawn_pipe_reader<R: std::io::Read + Send + 'static>(
    mut pipe: R,
    tx: mpsc::Sender<PipeChunk>,
    tag: fn(Vec<u8>) -> PipeChunk,
) {
    std::thread::spawn(move || {
        let mut buf = vec![0u8; 64 * 1024];
        loop {
            match pipe.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    if tx.send(tag(buf[..n].to_vec())).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });
}

/// Stop a child and, on Unix, its whole process
/// group: a wrapper script's children are stopped
/// with it, so nothing the helper started keeps
/// working (or holding its pipes) in the background
/// after the run is over. On Windows the child
/// alone is stopped; the helpers there are single
/// programs, not wrappers.
fn stop_child(child: &mut std::process::Child) {
    #[cfg(unix)]
    {
        let pgid = child.id() as libc::pid_t;
        // SAFETY: killpg on the group this child leads;
        // it exists because process_group(0) was set
        // before spawn. A group already gone answers
        // ESRCH, which is fine.
        unsafe {
            libc::killpg(pgid, libc::SIGKILL);
        }
    }
    let _ = child.kill();
    let _ = child.wait();
}

/// How the plain-words timeout messages name the
/// wait: "two minutes" for the standard deadline,
/// seconds for the short ones tests use.
pub fn describe_timeout(timeout: Duration) -> String {
    let secs = timeout.as_secs();
    if secs == 120 {
        "two minutes".to_string()
    } else if secs >= 60 && secs % 60 == 0 {
        format!("{} minutes", secs / 60)
    } else {
        format!("{secs} seconds")
    }
}

/// The most of a helper's stderr we will ever quote
/// back to the person: the first line, capped at
/// about 200 characters. Helper wording is the
/// helper's own, not ours, and a wall of it in an
/// error message helps no one (audit helpers MINOR,
/// 2026-10-16).
pub fn stderr_for_user(stderr: &[u8]) -> String {
    let text = String::from_utf8_lossy(stderr);
    let first = text.lines().next().unwrap_or_default().trim();
    const CAP: usize = 200;
    if first.chars().count() > CAP {
        let mut out: String = first.chars().take(CAP).collect();
        out.push('…');
        out
    } else {
        first.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stderr_for_user_takes_the_first_line_capped() {
        let long = format!("{} tail\nsecond line\nthird line", "x".repeat(300));
        let said = stderr_for_user(long.as_bytes());
        assert_eq!(said.chars().count(), 201, "200 characters and the ellipsis");
        assert!(said.ends_with('…'));
        assert!(!said.contains("second line"));
        assert_eq!(stderr_for_user(b"  one line  \nmore"), "one line");
        assert_eq!(stderr_for_user(b""), "");
        assert_eq!(stderr_for_user(b"\n"), "");
    }

    #[test]
    fn describe_timeout_names_the_standard_wait() {
        assert_eq!(describe_timeout(HELPER_TIMEOUT), "two minutes");
        assert_eq!(describe_timeout(Duration::from_secs(2)), "2 seconds");
        assert_eq!(describe_timeout(Duration::from_secs(180)), "3 minutes");
    }

    #[cfg(unix)]
    #[test]
    fn a_sleeping_child_is_killed_at_the_deadline() {
        let start = Instant::now();
        let fault = run_with_timeout(
            Command::new("sleep").arg("30"),
            None,
            Duration::from_secs(1),
        )
        .expect_err("a sleeping child cannot finish");
        assert!(matches!(fault, RunFault::TimedOut), "{fault:?}");
        assert!(
            start.elapsed() < Duration::from_secs(15),
            "the kill lands near the deadline, not at 30 s"
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_chatty_child_cannot_deadlock_the_pipes() {
        // The child writes 200 KB to stdout BEFORE it
        // reads stdin, then consumes stdin and answers.
        // Writing on the caller's thread while nobody
        // drained stdout would wedge both sides at the
        // 64 KB pipe buffer; the threaded shape sails.
        let payload = vec![b'x'; 200_000];
        let out = run_with_timeout(
            Command::new("sh")
                .arg("-c")
                .arg("head -c 200000 /dev/zero | tr '\\0' 'y'; cat > /dev/null; printf 'ok\\n'"),
            Some(&payload),
            Duration::from_secs(30),
        )
        .expect("the run finishes");
        assert!(out.status.success());
        assert_eq!(out.stdout.len(), 200_003);
        assert!(out.stdout.starts_with(b"yyy"));
        assert!(out.stdout.ends_with(b"ok\n"));
    }

    #[cfg(unix)]
    #[test]
    fn a_pipe_holding_grandchild_cannot_hold_the_caller() {
        // Round 2, RT-1: the helper exits at once, but a
        // process it started keeps stdout open for 7 s.
        // Pipe EOF waits for every holder, so the old
        // joining runner answered only when the holder
        // died. The caller must now hear the timeout at
        // the deadline (0.5 s) plus the grace (5 s),
        // while the holder is still alive. Both shapes:
        // a same-group grandchild, and one detached into
        // its own session (outside the reach of the group
        // kill). Stock macOS ships no `setsid` command, so
        // the detached shape detaches through the system
        // perl's POSIX::setsid — present on stock macOS
        // and on Linux — which puts the holder in its own
        // session before it becomes the sleep, holding the
        // inherited stdout pipe for the sleep's full life
        // while the parent shell exits at once, exactly as
        // the setsid command's shape did.
        let detached = "perl -e 'use POSIX qw(setsid); \
                        setsid() or die \"setsid: $!\"; \
                        exec \"sleep\", \"7\" or die \"exec: $!\"' & exec true";
        for script in ["sleep 7 & exec true", detached] {
            let start = Instant::now();
            let fault = run_with_timeout(
                Command::new("sh").arg("-c").arg(script),
                None,
                Duration::from_millis(500),
            )
            .expect_err("a held pipe past the grace is a timeout");
            assert!(matches!(fault, RunFault::TimedOut), "{script}: {fault:?}");
            let elapsed = start.elapsed();
            assert!(
                elapsed >= Duration::from_secs(5),
                "{script}: the grace was waited out: {elapsed:?}"
            );
            assert!(
                elapsed < Duration::from_secs(7),
                "{script}: the caller did not wait for the holder's life: {elapsed:?}"
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn a_flooding_helper_is_refused_at_the_byte_ceiling() {
        // Round 2, RT-8: `yes` fills any pipe as fast as
        // the machine allows (the lane measured 577 MB
        // inside a 1 s deadline). The deadline here is a
        // full minute, so only the 512 MB ceiling can
        // stop the run; the refusal itself, arriving
        // long before the deadline, is the evidence the
        // collection stayed bounded.
        let start = Instant::now();
        let fault = run_with_timeout(
            Command::new("yes").arg("ABCDEFGH"),
            None,
            Duration::from_secs(60),
        )
        .expect_err("a flood cannot finish");
        assert!(matches!(fault, RunFault::TooMuchOutput), "{fault:?}");
        assert!(
            start.elapsed() < Duration::from_secs(30),
            "the ceiling, not the deadline, stopped it: {:?}",
            start.elapsed()
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_missing_program_is_a_spawn_fault() {
        let fault = run_with_timeout(
            &mut Command::new("/definitely/not/a/program"),
            None,
            Duration::from_secs(5),
        )
        .expect_err("nothing there to run");
        assert!(matches!(fault, RunFault::Spawn(_)), "{fault:?}");
    }
}
