# Closing audit — EFFICIENCY lane

Scope: performance and resource use in `crates/ob-app` and the hot
paths it drives (`ob-text` shaping, `ob-fonts` library, `ob-pdf`
export, `ob-impose`, `ob-format`). Method: full read of the render
and action paths in `crates/ob-app/src/main.rs` (3,663 lines), plus
a timing probe (release build, this VM's server-class CPU, warm
page cache) exercising the app's own crates the way the app calls
them. Template documents (sparse text) were the test fixtures, so
shaping figures are floors; the library figures use the app's
first-run state (8 bundled Liberation faces, 3.17 MB).

One-sentence summary: real shaping is fast and exports are fine,
but the app rebuilds its entire font engine — disk reads, parses,
and SHA-256 hashes of every font in the library — once per panel
per repaint and once per paragraph in the Check step. That single
design choice dominates everything else in this report.

## Measured anchors (probe, release, this VM)

| Operation | Measured |
|---|---|
| `bundled_shaper()` construction (copies + registers 8 faces, 3.1 MB) | 1.97 ms |
| `FontLibrary::scan()`, 8 fonts / 3.17 MB (read + parse + SHA-256 each) | 33.4 ms |
| `full_shaper()` equivalent (bundled + open + scan + re-read + register) | ~29 ms |
| Shaping one booklet panel with a ready shaper | 0.52 ms |
| Check-step pattern on a 16-panel booklet (shaper rebuilt per paragraph, 51 paragraphs) | 1,249 ms |
| `export_reader_pdf` booklet / bifold | 51.6 ms / 35.2 ms, 1.25 MB out |
| `export_imposed_pdf` booklet | 31.1 ms, 1.25 MB out |
| Document snapshot (`serde_json::to_string`): bifold / booklet template | 4.9 KB / 9.2 KB, 0.01–0.02 ms |
| `system_fonts()` on this VM (only 8 fonts installed) | 29.4 ms |

Scan cost scales with total library bytes (hashing dominates:
~10 ms per MB). A realistic church library with the Starter Pack
is 10–40 MB, so one `full_shaper()` on such a machine is an
estimated 100–400 ms before it shapes a single letter. Every
figure below that multiplies `full_shaper()` gets worse in direct
proportion to how many fonts the church has installed.

## BLOCKER

### E1. The center preview defeats its own cache: every panel, every repaint, reshapes with a brand-new font engine

- `lines_cache` holds exactly one panel: `lines_cache: Option<(String, usize, Vec<PreviewLine>)>` (`crates/ob-app/src/main.rs:366`).
- The CentralPanel render loop calls `shaped_panel_lines(idx)` for every panel in order, every repaint (`main.rs:2980`, `main.rs:3013`). Panel 1's call evicts panel 0's entry, panel 2's evicts panel 1's, and so on. For any document with more than one panel the cache hit rate in this loop is zero, by construction.
- Every miss builds a new shaper from scratch: `shaped_panel_lines` calls `self.full_shaper()` (`main.rs:1263–1281`, shaper at `:1271`), and `full_shaper` (`main.rs:677–687`) copies and registers the 8 bundled faces (3.1 MB, ~2 ms), then opens the Font Library and `scan()`s it — reading every font file off disk, parsing it, and SHA-256 hashing every byte (~33 ms at 3.17 MB) — then reads each file a second time (`lib.read`, `main.rs:681–683`) and registers it with fontique.
- Arithmetic for the church's real shape, a 16-panel booklet, minimal library, this VM's fast CPU: 16 misses × ~29 ms ≈ **0.46 s per repaint**, of which actual shaping is ~8 ms. egui repaints on input events, so this is the cost of each keystroke in any field, each mouse-move frame over the window, each scroll tick, and each drag frame. For the default 4-panel bifold it is ~0.12 s per repaint: usable, but visibly sticky typing.
- On a 5-year-old office PC (slower single-thread, and a 10–40 MB library once the Starter Pack is installed), the same arithmetic lands in **seconds per repaint** for a booklet. The perverse detail: installing more fonts — a feature the app encourages — makes every screen slower, linearly.
- The Pages canvas pays the same toll: it renders after the CentralPanel in the same `update()` and calls `shaped_panel_lines` for its panel (`main.rs:1556`), one more miss per repaint even when its panel was just shaped for the center preview.

Impact: the app's home screen — the surface the admin stares at
all week — is the slowest code in the program. Fix direction
(for the fixing lane, not done here): cache one `Shaper` on the
app, keyed by a library fingerprint taken once per scan; make
`lines_cache` a map keyed by panel index. Shaping itself costs
0.5 ms per panel; with a cached shaper the whole center preview
is a few ms per repaint.

### E2. The Check step rebuilds the font engine once per paragraph: a measured 1.25 s freeze on the golden path

- `run_checks` calls `self.full_shaper()` inside the per-paragraph loop (`main.rs:739–741`, loop over panels × frames × paragraphs at `main.rs:714–748`). Each call is the full ~29 ms construction above; the resulting shaper shapes exactly one paragraph and is dropped.
- Measured end-to-end equivalent on the 16-panel booklet template (51 paragraphs, minimal library, server CPU): **1,249 ms**. `run_checks` runs when the user arrives at Step 4 (Check) via `goto_step` (`main.rs:566–574`) and on "Check again" — i.e., every week, on the guided path, the app freezes for over a second in the best case this code can ever produce.
- With a Starter-Pack-sized library the same step is an estimated 5–20 s on office hardware (51 × 100–400 ms). The user will read that as a hang and may force-quit mid-week.
- This is the same root cause as E1 (no shaper is ever reused), expressed per action instead of per frame; it is listed separately because the trigger, the fix site, and the user-visible symptom differ.

## MAJOR

### E3. No consumer ever reuses a shaper or a scan — the "build the font engine" pattern is repeated at six sites

Call sites that each pay open + full scan (read + parse + SHA-256
of every library byte) + re-read + register:

- `full_shaper` itself (`main.rs:677`), consumed per paragraph (E2) and per panel-miss (E1).
- `build_preview` (`main.rs:3195–3205`) — at least this one builds once per preview rebuild.
- `FontBook::build` in the PDF writer (`crates/ob-pdf/src/lib.rs:365`) and `register_library_fonts` (`crates/ob-pdf/src/lib.rs:659`) — every export pays the scan **twice**, once to embed and once to shape.
- `pin_document_fonts` (`crates/ob-fonts/src/lib.rs:367`) — one more scan on every Save, every export, and every style-font change (`main.rs:900–904`).

Individually these are 30–400 ms actions; the problem is that
there is no fingerprint, no cache, and no shared instance anywhere
in the design, so every new feature that needs text measurement
will copy the pattern (two already have). `FontLibrary::scan`
also re-reads files it hashed microseconds earlier when a caller
follows scan with `read` — the scan result carries `bytes: usize`
but not the bytes, so the pair is always two disk reads per font
per call.

### E4. "Show fonts on this computer" runs an unbounded whole-disk font census on the UI thread

- The button handler (`main.rs:2439–2444`) calls `ob_fonts::system_fonts()` synchronously. That function recursively walks every OS font folder and, for each `.ttf`/`.otf`, reads the whole file, parses it, and SHA-256 hashes it for dedup (`crates/ob-fonts/src/lib.rs:308–365`).
- Floor evidence: 29.4 ms for the 8 fonts on this VM (~3.7 ms per font). A real Windows office PC carries 200–400 fonts including multi-megabyte CJK faces; the click plausibly freezes the window for several seconds with no spinner, no progress, and no way to cancel.
- The inconsistency is the tell: catalog search and install — the same "wait for something slow" shape — are correctly run on worker threads with an mpsc channel (`main.rs:2334` onward, `:2401` onward). The system census is more expensive than either and runs on the UI thread.

### E5. Dragging and typing pay full reshaping per frame because the cache key is the whole document

- Both caches key on `frame_snapshot`, the entire document serialized (`main.rs:2495`, key clones at `main.rs:1023` and `main.rs:1264`). Any change — including each intermediate position of a frame drag — changes the key, so during a canvas drag `shaped_panel_lines` misses on every frame and pays a full shaper build plus reshape (~30 ms on this VM, minimal library) before the frame can be drawn. A drag is dozens of such frames: the box trails the pointer in visible steps.
- The same applies while the Print step is open during a drag: `preview_sheets` rebuilds the imposed preview per drag frame (one shaper build + shaping all panels + imposition, ~40 ms measured-equivalent), on top of E1's center-preview cost.
- The invalidation itself is sound — content keys cannot serve stale lines (see "What is fine"), and a mid-frame key skew self-corrects next frame. The problem is purely granularity: one global key for per-panel data, and a key whose "changed" state is the normal state during every gesture.

## MINOR

### E6. Per-frame serialization and string clones are real but measured negligible — today

- `update()` serializes the whole document every repaint (`main.rs:2495`), and the caches clone that string as a key (up to ~18 clones per repaint across the center loop and print preview). Measured: 0.02 ms and 9.2 KB for a booklet template. A filled-in real bulletin (order of service, full announcements) will be several times larger and still sub-millisecond. Recorded so the next lane does not have to re-derive it: this is a watch item if documents grow images or long histories, not a current cost. (It was the right fix for audit R6; the bill just turned out to be tiny.)

### E7. Undo memory is bounded and modest

- Undo/redo are `Vec<String>` of full-document JSON, capped at 60 (`main.rs:165–171`). At template size that is ≤ ~0.6 MB per stack; at a generous 100 KB per filled document, ~6 MB per stack, ~12 MB with redo full. Several actions serialize 2–3 times (before-snapshot plus compare: e.g. `main.rs:254` + `:262`, `:587` + `:602`, `:625` + `:668`), each sub-millisecond. `record_undo`'s `remove(0)` shifts at most 60 string headers. All fine at church scale; noted for completeness.

### E8. PDF export: fast enough per click, pays two scans, ships 1.25 MB files

- Measured: 31–52 ms per export on this VM (booklet and bifold), so even 5× slower office hardware stays under ~0.3 s per click. Acceptable.
- But each export scans the library twice (E3) and embeds every wanted face **whole and unsubsetted**, producing a 1.25 MB PDF for a 4-panel bifold whose text is a few KB (measured output sizes above). The reading copy is the one churches email and post; on rural connections and old mailboxes, 1.25 MB where ~100 KB would do is a real, recurring cost. Font subsetting is the known fix and is already noted as a later step in the writer's own docs (`crates/ob-pdf/src/lib.rs:1–9`).

### E9. Adding fonts is quadratic in library size, on the UI thread

- `FontLibrary::add_bytes` dedups by scanning the whole library — one full read + parse + hash of every existing font per added font (`crates/ob-fonts/src/lib.rs:190–196`). The Add-fonts file-picker path calls `add_zip_bytes` on the UI thread (`main.rs:2268` area), so a 20-font zip into a 30-font library is ~50 full scans ≈ seconds of freeze. First-run seeding in `refresh_fonts` inherits the same shape at smaller scale, and `refresh_fonts` also calls `scan()` twice back to back (`main.rs:819`, `main.rs:826`). The Starter Pack path escapes the freeze only because it happens to run on the catalog worker thread.

### E10. Print-step per-frame extras: individually cheap, listed for completeness

- While Step 5 (Print) is shown, each repaint calls `save_announcements()` (dirty-gated, early-returns: `main.rs:611–620`), `print_checklist()` → `print_checklist_for` → `ob_impose::impose(doc)` (pure placement arithmetic, no shaping: `crates/ob-impose/src/lib.rs:44` onward), and clones the entire `Vec<PreviewSheet>` to draw it (`main.rs:1046`). The clone is the only one worth a second look on a booklet (all panels' line strings, per repaint, while on the Print step); it is still small next to E1/E5.

## What is fine (audited, no finding)

- **Shaping itself.** With a ready shaper, a panel shapes in ~0.5 ms and `wrap`'s strict re-break only re-measures over-wide lines (`crates/ob-text/src/shaping.rs:198–261`). Parley is not the problem; shaper construction is.
- **Cache invalidation correctness.** Both preview caches and the journal compare on serialized content, so they cannot hand back stale lines; the failure mode is cost, never wrong output.
- **Autosave.** `journal_tick` is throttled to 2 s, snapshots once per tick, and writes the journal file only when the document actually changed (`main.rs:877–898`); journal writes go through the atomic temp + rename in `ob-format` (`crates/ob-format/src/lib.rs:11–16`).
- **Catalog network work** is threaded with a channel and a busy spinner; the UI thread only polls (`main.rs:907–953`).
- **Startup.** `App::default` builds a template from strings and reads one small settings file; no font loading, no scans, no network happen until a feature asks (`main.rs:452–536`, `main.rs:3054–3071`). The first repaint then pays E1 once, but process launch itself is lean.
- **Panel/guide clones in the canvas** (e.g. `main.rs:1398`, `main.rs:1611`) are small structs per repaint — noise next to E1.
- **No repaint storm:** nothing requests continuous repaints; the only timed repaint is a 120 ms poll while a catalog job runs (`main.rs:945`). Idle CPU is zero. (During that poll, however, each repaint costs E1's full price — a busy font search makes the rest of the window sluggish.)

## Baseline verification

`cargo test --workspace` was run for this audit:

- First invocation: **1 failure** — `ob-fonts` test `system_scan_never_errors` failed its `found.path.exists()` assertion, and that suite took 43.87 s (cold). All suites before it passed; the run stops at the failing suite by default.
- Re-run with `--no-fail-fast`: **53 passed, 0 failed** across all 11 suites (ob-extract 2, ob-fonts 8, ob-format 1, ob-impose 3, ob-layout 4, ob-model 5, ob-pdf 8, ob-pdfium-helper 0, ob-text 10, ob-app 12, ob-cli 0), ob-fonts suite 1.88 s warm.
- The one failure is an environment-sensitive system-font scan test (a path it enumerated did not exist when re-checked), not a code regression this lane can attribute; the cold-vs-warm suite times (43.87 s vs 1.88 s) are themselves consistent with how I/O-heavy a full system font census is (E4). Flagged as flaky-in-this-environment for the coordinator.

## Verdict: will this feel fast on a 5-year-old office PC?

For the default bifold and light use — opening the app, stepping
through the five steps, saving, exporting — mostly yes, with a
visible stickiness in typing and dragging once a few fonts are
installed (~0.1–0.3 s per repaint, estimated).

For this church's actual shape — the 16-panel booklet — and for
the Check step in any shape once the Starter Pack is in: **no**.
Entering Check freezes for seconds (E2, measured 1.25 s in the
best possible configuration on a server CPU), and every keystroke,
scroll, and drag in the main window costs a full panel-by-panel
reshape with a rebuilt font engine (E1), landing in seconds per
repaint on modest hardware with a realistic library. The failure
is architectural, not a matter of micro-optimization — and it is
also unusually fixable: shaping is 0.5 ms per panel, so a cached
shaper plus a per-panel lines cache turns the home screen from
the slowest code in the app into a few ms per frame. Until that
lands, the honest answer is that the app feels fast exactly where
users spend the least time (startup, export) and slow exactly
where they spend the most (looking at their bulletin and checking
it).
