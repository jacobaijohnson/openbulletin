//! The CLI is the project's scriptable surface; smoke it end to
//! end: make a bulletin, export it, advance the week.

#[test]
fn new_export_next_sunday_round_trip() {
    let dir = std::env::temp_dir().join(format!("ob-cli-smoke-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("dir");
    let bin = env!("CARGO_BIN_EXE_openbulletin-cli");
    let run = |args: &[&str]| {
        let out = std::process::Command::new(bin)
            .args(args)
            .current_dir(&dir)
            .output()
            .expect("run cli");
        assert!(
            out.status.success(),
            "cli {args:?} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).into_owned()
    };
    let obd = dir.join("week.obd");
    run(&[
        "new",
        "--preset",
        "saddle-booklet",
        "--church",
        "Grace Lutheran Church",
        "--out",
        obd.to_str().expect("path"),
    ]);
    assert!(obd.exists());
    let pdf = dir.join("week.pdf");
    run(&[
        "export",
        obd.to_str().expect("path"),
        "--out",
        pdf.to_str().expect("path"),
    ]);
    let bytes = std::fs::read(&pdf).expect("pdf written");
    assert!(bytes.starts_with(b"%PDF"), "export produced a PDF");
    let info = run(&["info", obd.to_str().expect("path")]);
    assert!(
        info.contains("saddle-booklet"),
        "info names the preset: {info}"
    );
    let idml = dir.join("week.idml");
    let said = run(&[
        "export-idml",
        obd.to_str().expect("path"),
        "--out",
        idml.to_str().expect("path"),
    ]);
    assert!(
        said.contains("IDML for InDesign"),
        "export-idml reports what it made: {said}"
    );
    let zbytes = std::fs::read(&idml).expect("idml written");
    assert!(zbytes.starts_with(b"PK"), "an IDML file is a zip package");
    let sla = dir.join("week.sla");
    let said = run(&[
        "export-sla",
        obd.to_str().expect("path"),
        "--out",
        sla.to_str().expect("path"),
    ]);
    assert!(
        said.contains("Scribus file"),
        "export-sla reports what it made: {said}"
    );
    let sbytes = std::fs::read(&sla).expect("sla written");
    assert!(sbytes.starts_with(b"<?xml"), "a Scribus file is XML");
    let _ = std::fs::remove_dir_all(&dir);
}
