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
//! while stdout and stderr drain on their own threads.
//! A helper that answers a lot before reading its
//! request can therefore never deadlock against a
//! full pipe, and a silent one cannot hang the
//! caller: past the deadline the child is killed and
//! the caller hears about it in time to say so in
//! plain words. On Unix the child runs in its own
//! process group and the whole group is stopped, so
//! a helper that is itself a wrapper script cannot
//! leave its own children holding the pipes (and the
//! caller) open after it is gone.

use std::io::{Read as _, Write as _};
use std::process::{Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

/// How long any helper may take before it is stopped:
/// two minutes, the same patience the Publisher
/// bridge gives LibreOffice.
pub const HELPER_TIMEOUT: Duration = Duration::from_secs(120);

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
    TimedOut,
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
    let stdin = child.stdin.take();
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let (status, stdout, stderr) = std::thread::scope(|scope| {
        let writer = scope.spawn(move || {
            if let (Some(mut sin), Some(bytes)) = (stdin, stdin_bytes) {
                // A helper that stopped early closes its
                // end; a failed write is its answer, and
                // the readers below carry what it said.
                let _ = sin.write_all(bytes);
                // The handle drops here: stdin closed.
            }
        });
        let out_reader = scope.spawn(move || {
            let mut buf = Vec::new();
            if let Some(mut so) = stdout {
                let _ = so.read_to_end(&mut buf);
            }
            buf
        });
        let err_reader = scope.spawn(move || {
            let mut buf = Vec::new();
            if let Some(mut se) = stderr {
                let _ = se.read_to_end(&mut buf);
            }
            buf
        });
        let deadline = Instant::now() + timeout;
        let status = loop {
            match child.try_wait() {
                Ok(Some(s)) => break Ok(s),
                Ok(None) => {
                    if Instant::now() >= deadline {
                        stop_child(&mut child);
                        break Err(RunFault::TimedOut);
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(e) => {
                    stop_child(&mut child);
                    break Err(RunFault::Wait(e));
                }
            }
        };
        // Once the child is done or killed its pipes are
        // closed, so the three threads all finish.
        let _ = writer.join();
        let stdout = out_reader.join().unwrap_or_default();
        let stderr = err_reader.join().unwrap_or_default();
        (status, stdout, stderr)
    });
    match status {
        Ok(status) => Ok(RunOutput {
            status,
            stdout,
            stderr,
        }),
        Err(fault) => Err(fault),
    }
}

/// Stop a child and, on Unix, its whole process
/// group: a wrapper script's children inherit the
/// pipes, and a pipe held open by a survivor would
/// keep the drain threads (and so the caller) waiting
/// long after the deadline. On Windows the child
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
