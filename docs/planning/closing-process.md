# Closing audit — PROCESS lane

Auditor: process lane (blind to the other lanes). Date: 2026-10-15.
Scope: docs truthfulness vs code, CI reality, test discipline, git
hygiene, license posture. No code or docs were modified for this
audit; this report is the only file written. Nothing is committed.

Method: every claim below was checked against the working tree at
HEAD (`4014b3d`, "Batch C"), a full local `cargo test --workspace`,
`cargo clippy --workspace --all-targets` (plain and `-D warnings`),
`cargo fmt --all -- --check`, `cargo metadata`, `git ls-files` /
`git log`, and `gh run list`.

## Verdict

The process culture is unusually honest (commit messages disclose
the project's own errors; the one known silent-loss incident was
turned into a named regression test). But the safety net has a hole
exactly where the known accident happened: **CI does not run the
app crate's tests at all**, and the public docs (README, ROADMAP)
drifted out of date within hours of the features they describe
shipping. Process health: sound habits, not yet self-checking.

Counts: **1 BLOCKER, 7 MAJOR, 7 MINOR.**

---

## BLOCKER

### P-01. CI never runs the tests that guard the known failure class
`.github/workflows/ci.yml` and `.github/workflows/build-desktop.yml`
both run tests and clippy for exactly seven packages:
`ob-model ob-layout ob-impose ob-text ob-pdf ob-format
openbulletin-cli`. Excluded from test *and* clippy:

- `openbulletin` (the app) — 12 tests, including
  `announcements_buffer_only_writes_when_dirty`, the regression test
  written after a real behavior (the announcements write gate) was
  silently lost in a file splice while all tests stayed green. In
  CI, that loss would **still** go undetected today: the test
  exists, passes locally, and is never executed by either workflow.
- `ob-fonts` — 8 tests, including the font-rights and catalog
  license-gate tests.
- `ob-extract` — 2 tests; worse, nothing in either workflow's
  package set even *depends* on `ob-extract`, and
  `ob-pdfium-helper` (its only consumer) is never built in CI, so
  both crates are not even compiled there (`grep ob-extract
  .github/workflows/` returns nothing).

The app is only `cargo build --release -p openbulletin` in CI, so
its test code is not even type-checked there. The app is also the
riskiest artifact in the repo: one 3,663-line
`crates/ob-app/src/main.rs` holding the GUI, the autosave journal,
undo, and print logic — the same file the splice loss happened in.

Compounding this, `CONTRIBUTING.md` tells contributors to "Run
`cargo fmt`, `cargo clippy`, and `cargo test` before pushing; CI
runs the same on Windows, macOS, and Linux." That is false as
written: CI runs a subset.

Fix direction (not applied): `cargo test --workspace` and
`cargo clippy --workspace --all-targets` in both lanes, or an
explicit, documented reason for each exclusion.

---

## MAJOR

### P-02. README says page typing is not built. It is.
README.md, "Not built yet": "**Typing words directly on the page.**
Today you edit in the step panels, not on the page picture.
Planned." False at HEAD: double-click inline canvas editing is
implemented (`enter_edit` / `commit_edit` / `cancel_edit` in
`crates/ob-app/src/main.rs`), specified in docs/EDITOR.md ("Typing
on the page, and Undo (built 2026-10-15)"), and covered by
`edit_session_commit_then_undo_round_trip`. The README is the
public front door and underclaims a headline feature.

### P-03. README says font picking is not built. It is.
README.md, "Fonts today": "**You cannot pick a font for a style
inside the app yet**" and "A one-tap picker for fonts already
installed on your computer is also still to come; today you add
the font file in the Fonts panel." Both false at HEAD: a per-style
font picker exists (`ComboBox::from_id_salt(("style-font", ...))`,
ob-app main.rs ~line 2221), a "Fonts on this computer" copy-in
section exists (main.rs ~2424, backed by `ob_fonts::system_fonts()`),
and a one-tap Church Starter Pack install exists in the panel
(main.rs ~2393). docs/FONTS.md's own Status section describes all
of this correctly; the README was not updated when it shipped
(Batch B, commit `bf857d8`).

### P-04. README's crate table and CLI description omit shipped surface
The README crate table lists 9 crates; the workspace has 11
(`cargo metadata`: 11 workspace members). Missing from the table:
`ob-fonts` and `ob-pdfium-helper`. The CLI is described in the
README (and in `crates/ob-cli/Cargo.toml`'s own description) as
"`new`, `export`, `next-sunday`, `info`", but
`crates/ob-cli/src/main.rs` also implements `trace` (PDF → traced
draft `.obd`) and `fonts list|add|search|get|pack|system`. The
README Imports section ("You cannot yet bring in ... an old
bulletin PDF and turn it into an editable bulletin") never mentions
the shipped `trace` rescue, which does a first version of exactly
that.

### P-05. ROADMAP checkboxes contradict the shipped state
ROADMAP.md v0.1 still shows as open:
- "[ ] Desktop shell: inline text editing, save/open, autosave +
  recovery" — all built (journal + recovery tested by
  `journal_round_trip_and_clear`; commit `0866180`).
- "[ ] Duplex test-sheet wizard" — built (commit `d708b76`;
  tested by `duplex_test_sheet_has_front_and_back` and
  `print_checklist_names_paper_scale_and_learned_flip`).

The roadmap is the project's own tracking doc; right now it
understates progress in the same way the README does (P-02/P-03),
which suggests feature batches update EDITOR.md/FONTS.md but not
the two summary docs.

### P-06. docs/FORMAT.md makes claims the code does not keep
- "The GUI adds rolling backups and crash recovery in v0.1."
  There are no rolling backups anywhere: `grep -ri backup crates/`
  returns nothing in code. What exists is a single autosave
  journal (`autosave.obd` + `autosave.txt` beside settings),
  written atomically. Crash recovery is real; "rolling backups"
  is not.
- "readers reject newer-than-known versions ... and accept all
  older ones." Only version 0 exists (`FORMAT_VERSION = 0`), so
  older-version acceptance is vacuous and untested. The single
  ob-format test is a same-version roundtrip; the newer-version
  rejection branch in `crates/ob-format/src/lib.rs` (lines 23–29)
  has no test.
- "Migration fixtures for every released version live in CI from
  the first release." No fixtures exist in the repo
  (`git ls-files` shows none) and CI has no fixture step. There
  has been no release yet, so the commitment is not yet breached,
  but nothing is in place to keep it.

### P-07. docs/LEGAL.md's dependency audit is stale by its own rule
LEGAL.md records a 2026-10-15 audit of "475 packages" and says
"Re-run this audit whenever dependencies change materially."
Dependencies did change materially after that audit (ob-fonts with
ureq/rustls/zip, ob-pdfium-helper with pdfium-render/image, etc.).
This audit's fresh `cargo metadata` count: **523 packages** (512
external + 11 workspace), i.e. +48. The recorded license list is
also incomplete now: the tree additionally contains BSL-1.0,
CDLA-Permissive-2.0, Unlicense, CC0-1.0, 0BSD, BSD-2-Clause, MIT-0,
and Apache-2.0 WITH LLVM-exception variants, none named in
LEGAL.md.

Substantive posture re-verified today and **unchanged**: every
package is permissive; the only copyleft string in the tree is
r-efi's "MIT OR Apache-2.0 OR LGPL-2.1-or-later" (now present in
two versions), an OR choice satisfied by MIT/Apache-2.0. No GPL,
AGPL, SSPL, or LGPL-as-requirement. So this is a process failure
(the guard was written down and not re-run), not a legal exposure.

### P-08. Load-bearing negative paths are untested
- **Format versioning:** no test rejects a newer-version file
  (see P-06); no corrupt/truncated-file test; no older-version
  fixture.
- **Font rights gating:** `EmbeddingRights::may_package()` is
  asserted only for the permissive case (Liberation, fsType 0 →
  Installable, `bundled_liberation_faces_inspect_cleanly`). No
  test constructs a PreviewAndPrint or Restricted font, and the
  PDF writer's exclusion branch (`crates/ob-pdf/src/lib.rs`
  ~359–366, faces whose rights do not allow travel fall back) has
  no negative-path test. fsType parsing for non-zero values is
  likewise untested. This is a legal guardrail whose "no" direction
  is entirely unverified. (The catalog's license gate *is* tested
  both ways: OFL/Apache/UFL accepted, "proprietary" rejected.)
- **Autosave decision logic:** the journal file helpers are
  tested (write/read/clear, source path, timestamp), but
  `journal_tick`'s actual decisions — write only when the screen
  differs from the last save, retire when equal — are not.
- **The CLI has zero tests** (0 `#[test]` in
  `crates/ob-cli/src/main.rs`), despite its Cargo.toml calling it
  "The CI oracle for the engine." `new`, `export`, `trace`, and
  the `fonts` subcommands have no automated coverage anywhere.

---

## MINOR

### P-09. docs/FONTS.md contradicts itself in the first screen
Header: "Researched against primary sources; summarized here in
plain words. **This is design, not built yet.**" Its own Status
section (same file) then documents the built crate, CLI, catalog,
in-app panel, starter pack, and PDF embedding. The header is a
leftover from the design phase.

### P-10. README understates the bundled font set
"Two font families come with the app ... Each has regular and
bold." The bundle is 8 faces (regular, bold, italic, bold italic
for both families), asserted by ob-text's
`bundled_set_has_all_eight_faces`. The next README bullet does
say italics work, so the file disagrees with itself mildly.

### P-11. DEPLOYMENT.md calls the desktop shape "shipping"
"**Desktop app (v0.1, shipping shape).**" The README is more
honest: no installer, build from source, "OpenBulletin is not
ready for you yet." The rest of DEPLOYMENT.md is accurately
labeled planning and matches the code's posture (no accounts, no
telemetry; the only network code in the tree is the ob-fonts
catalog client — verified via crate dependencies: only ob-fonts
pulls ureq). PDFium runs out-of-process as claimed, via
ob-pdfium-helper.

### P-12. CI is a post-push report, not a gate, in practice
All 45 commits were pushed directly to `main` by the single
author on 2026-10-15; every workflow run in the sampled history
is a `push` event (no pull-request runs). Hosted CI takes
~6–9 minutes, so during the audit-fix batches the next batch was
pushed while the previous batch's hosted run was still in
progress. Final state is green (see "What is healthy"), but
"CI green" during the day usually meant "a recent commit was
green." Acceptable for a solo pre-release project; worth naming.
(Branch protection was not verified — the `gh api .../protection`
call did not return during this audit.)

### P-13. ob-fonts tests are slow and machine-dependent
The full workspace suite (53 tests) took ~90 s locally, of which
ob-fonts alone took 79.94 s. `system_scan_never_errors` scans the
real machine's font folders and inspects whatever it finds, so the
suite's cost and coverage vary by machine. This discourages
running the full suite — the exact habit P-01 needs.

### P-14. Public tree carries raw working artifacts
`docs/planning/` tracks raw Tier-2 outputs (`gpt-raw.json`,
`opus-raw.json`, `gpt-sol-raw.json`, ~28 KB each) and error stubs
(`gpt-err.txt`, `opus-err.txt`, etc., one containing an empty-stub
JSON response). This audit grepped them for credential patterns
and found none — they contain model output text only. It is
public-tree noise, not exposure.

### P-15. One-file concentration in the app crate
`crates/ob-app/src/main.rs` is 3,663 lines / 152 KB tracked: GUI,
settings, journal, undo, print checklist, and all 12 app tests in
one file. Not a defect by itself, but it is the structural reason
a splice could silently remove a behavior (the P-01 incident) and
why CI's exclusion of this crate matters so much.

---

## What is healthy (verified, not asserted)

- **Tests:** `cargo test --workspace` locally: **53 passed,
  0 failed** — ob-model 5, ob-layout 4, ob-impose 3, ob-text 10,
  ob-pdf 8, ob-format 1, ob-fonts 8, ob-extract 2, openbulletin
  (app) 12, openbulletin-cli 0, ob-pdfium-helper 0. No doc-tests
  or integration tests exist anywhere.
- **Lint/format:** `cargo clippy --workspace --all-targets`
  clean, including with `-D warnings`; `cargo fmt --all -- --check`
  clean. Local discipline exceeds CI scope (see P-01).
- **CI outcomes:** every completed run in the sampled `gh run
  list` history (20 runs, both lanes) is `success`, including the
  final HEAD runs for Batch B and Batch C on both the hosted
  3-OS lane and the self-hosted desktop lane. What CI *runs*:
  fmt check, the 7-package test/clippy subset (P-01), and a
  release build of app + CLI, on windows/macos/ubuntu.
- **Imposition coverage is genuinely strong:** bifold panel order
  ([3,0]/[1,2]), tri-fold flap narrowing plus total-width
  conservation, and saddle-stitch order for 8 pages are all
  asserted exactly.
- **The splice-loss scar tissue is real:** the regression test
  names the incident in its comment ("this gate was once lost in
  a file splice while every test stayed green") and tests both
  the dirty and clean branches. The class of loss is now
  detectable *locally* — just not in CI (P-01).
- **Shaping/PDF oracles:** hmtx width oracles, embedded-width
  array checks, and an EOF-marker regression test give the PDF
  writer real verification, not smoke tests.
- **Git hygiene:** tree clean at audit time; 45 commits, all
  2026-10-15, single author (Jacob Johnson); messages are
  descriptive and self-critical (commit `1b8d31a` discloses in
  its own message that a predecessor commit landed before its
  test run finished). `.gitignore` covers `/target`, `*.obd`,
  `*.pdf`, `.DS_Store`. **No** bulletin fixtures, generated
  `.obd`/`.pdf` files, or the PDFium binary are tracked
  (`git ls-files` verified); real-bulletin research data stays
  outside the repo as LEGAL.md claims. No TODO/FIXME/todo!
  markers in the code. .git is 6.2 MB; the largest tracked files
  are the Liberation TTFs (~400 KB each, OFL, license file in
  tree beside them) — legitimate assets, not bloat.
- **License mechanics:** LICENSE-MIT and LICENSE-APACHE are
  present with correct texts and a copyright line; the workspace
  declares `license = "MIT OR Apache-2.0"` and all 11 crates
  inherit it via `license.workspace = true` (verified per-crate);
  Cargo.lock is tracked. See P-07 for the audit-freshness caveat.
- **EDITOR.md is accurate:** every checked claim (guides in-file,
  snapping helper in ob-model with tests, inline editing, undo
  scope, zoom, add/copy/delete boxes, guide lock, Tab cycling)
  was found in the code and, where applicable, in tests. It is
  the best-maintained doc in the repo and the model the other
  docs should be synced against.
