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

#[test]
fn export_refuses_to_replace_its_own_input() {
    // Round 2, R2-6: no export door may write over the
    // bulletin it is reading, and the refusal happens
    // before anything is written: the file is byte for
    // byte what it was.
    let dir = std::env::temp_dir().join(format!("ob-cli-guard-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("dir");
    let bin = env!("CARGO_BIN_EXE_openbulletin-cli");
    let raw = |args: &[&str]| {
        std::process::Command::new(bin)
            .args(args)
            .current_dir(&dir)
            .output()
            .expect("run cli")
    };
    let obd = dir.join("week.obd");
    let made = raw(&["new", "--out", obd.to_str().expect("path")]);
    assert!(made.status.success(), "new makes the bulletin");
    let before = std::fs::read(&obd).expect("read bulletin");

    for door in [
        "export",
        "export-idml",
        "export-sla",
        "export-md",
        "export-html",
    ] {
        let out = raw(&[door, obd.to_str().unwrap(), "--out", obd.to_str().unwrap()]);
        assert!(!out.status.success(), "{door} over its input must fail");
        let err = String::from_utf8_lossy(&out.stderr).into_owned()
            + &String::from_utf8_lossy(&out.stdout);
        assert!(
            err.contains("the copy would replace the bulletin itself"),
            "{door} refuses in plain words: {err}"
        );
        assert_eq!(
            std::fs::read(&obd).expect("bulletin still there"),
            before,
            "{door} left the bulletin byte-identical"
        );
    }
    // A path that only reaches the input by walking up a
    // folder is the same file too.
    std::fs::create_dir_all(dir.join("sub")).expect("sub dir");
    let sneaky = dir.join("sub").join("..").join("week.obd");
    let out = raw(&[
        "export-md",
        obd.to_str().unwrap(),
        "--out",
        sneaky.to_str().unwrap(),
    ]);
    assert!(!out.status.success(), "the .. path is refused too");
    assert_eq!(std::fs::read(&obd).expect("bulletin still there"), before);
    // And no temp files were left behind by any refusal.
    let leftovers: Vec<String> = std::fs::read_dir(&dir)
        .expect("list dir")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.contains(".tmp"))
        .collect();
    assert!(leftovers.is_empty(), "no temp litter: {leftovers:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn fonts_add_skips_the_bad_and_lands_the_good() {
    // Round 2, R2-6: one bad file in a `fonts add` batch is
    // skipped and named; it does not stop the batch. The
    // bad file goes FIRST here to prove the good one still
    // lands after it.
    let dir = std::env::temp_dir().join(format!("ob-cli-fonts-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("dir");
    let good_src = ob_text::bundled_asset_path("LiberationSans-Regular.ttf");
    let good = dir.join("LiberationSans-Regular.ttf");
    std::fs::copy(&good_src, &good).expect("copy the good font");
    let bad = dir.join("notafont.txt");
    std::fs::write(&bad, b"this is not a font").expect("write the bad file");
    let lib = dir.join("library");
    let bin = env!("CARGO_BIN_EXE_openbulletin-cli");
    let out = std::process::Command::new(bin)
        .args([
            "fonts",
            "add",
            bad.to_str().unwrap(),
            good.to_str().unwrap(),
            "--dir",
            lib.to_str().unwrap(),
        ])
        .output()
        .expect("run cli");
    assert!(
        out.status.success(),
        "the batch completes: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let said = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        said.contains("added Liberation Sans"),
        "the good font landed: {said}"
    );
    assert!(
        said.contains("skipped") && said.contains("notafont.txt"),
        "the bad file is named as skipped: {said}"
    );
    assert!(
        said.contains("1 added, 1 skipped"),
        "the batch is counted out loud: {said}"
    );
    assert!(
        lib.join("LiberationSans-Regular.ttf").exists(),
        "the good font is in the library"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn new_with_a_bad_date_speaks_plainly() {
    // Round 2, R2-6: a date the CLI cannot read is refused
    // in our words, never in the date library's dialect.
    let dir = std::env::temp_dir().join(format!("ob-cli-date-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("dir");
    let bin = env!("CARGO_BIN_EXE_openbulletin-cli");
    let target = dir.join("week.obd");
    let out = std::process::Command::new(bin)
        .args([
            "new",
            "--date",
            "Oct 18 2026",
            "--out",
            target.to_str().unwrap(),
        ])
        .current_dir(&dir)
        .output()
        .expect("run cli");
    assert!(!out.status.success(), "the bad date fails");
    let err = String::from_utf8_lossy(&out.stderr).into_owned();
    assert!(
        err.contains("does not read as a date") && err.contains("2026-10-18"),
        "the refusal shows the expected shape: {err}"
    );
    assert!(!target.exists(), "no bulletin was written");
    let _ = std::fs::remove_dir_all(&dir);
}
