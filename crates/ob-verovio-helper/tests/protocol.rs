//! The helper binary's own protocol gate, end to end
//! (audit M10, 2026-10-16): a request naming another
//! protocol version is answered with the structured
//! "protocol" refusal, and a request in the spoken
//! version is answered in kind. Before the fix the
//! binary answered protocol 999 and 0 normally.

use std::io::Write as _;
use std::process::{Command, Stdio};

fn ask(request: &str) -> serde_json::Value {
    let mut child = Command::new(env!("CARGO_BIN_EXE_ob-verovio-helper"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("the helper starts");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(request.as_bytes())
        .expect("the request is written");
    let out = child.wait_with_output().expect("the helper answers");
    assert!(
        out.status.success(),
        "a readable request always gets a JSON answer: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("the answer is JSON")
}

#[test]
fn a_request_from_another_protocol_is_refused_as_protocol() {
    for v in [999, 0] {
        let resp = ask(&format!(
            "{{\"protocol\":{v},\"command\":\"probe\",\"musicxml\":\"\",\
             \"width_mm\":0,\"staff_mm\":0}}"
        ));
        assert_eq!(resp["ok"], false, "version {v}");
        assert_eq!(resp["kind"], "protocol", "version {v}");
        assert_eq!(resp["protocol"], 1, "the answer speaks our version");
    }
}

#[test]
fn a_request_in_the_spoken_protocol_is_answered() {
    // "probe" answers ok where an engraver is installed
    // and unavailable where none is; both are shaped
    // answers in protocol 1, never a protocol refusal.
    let resp = ask("{\"protocol\":1,\"command\":\"probe\",\"musicxml\":\"\",\
         \"width_mm\":0,\"staff_mm\":0}");
    assert_eq!(resp["protocol"], 1);
    assert_ne!(resp["kind"], "protocol");
    assert!(resp["ok"].as_bool() == Some(true) || resp["kind"] == "unavailable");
}
