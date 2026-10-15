# Closing audit — CODE CORRECTNESS lane

Read cold: all 12 crates (ob-app 3,663 lines; ob-pdf 823; ob-fonts,
ob-layout, ob-model, ob-impose, ob-text, ob-format, ob-extract,
ob-cli, ob-pdfium-helper). No code modified.

## Baseline, verified by this lane (not taken on trust)

- `cargo test --workspace`: **53 passed, 0 failed.**
- `cargo clippy --workspace --all-targets`: **zero warnings.**

What is genuinely sound: atomic .obd saves (temp + rename),
newer-format rejection at load, the dirty-gated announcements
buffer, snapshot-keyed preview caches, the strict re-break in the
shaper's wrap, embedded-width oracle tests in ob-pdf, and most
canvas indexing going through `.get()`. The defects below are
what remains around those strengths.

## BLOCKER

### B-1. Stale frame selection across a panel switch panics on arrow keys, Enter, or Ctrl+C
Evidence: `selected_frame` is an index into the *current* panel,
but the Pages pager never resets or clamps it
(`crates/ob-app/src/main.rs` pages_window pager, ~lines 1310-1330),
and three handlers index with it directly:
nudge `self.doc.panels[idx].frames[fi]` (line 1846),
Enter-to-edit `frames[fi].story_id` (line 1798),
Ctrl+C `frames[fi].clone()` (line 1806).
Failure scenario: on the Cover (4+ boxes in a traced booklet
draft, select box 4), page to a panel with 2 boxes, press an
arrow key to nudge "the selected box" — index out of bounds,
the app crashes mid-edit. No malformed file needed; the journal
limits the loss to the last two seconds of work.

### B-2. Deleting a box while typing in another leaves `editing` pointing at a removed/shifted frame: panic, or the buffer written into the wrong story
Evidence: the canvas right-click menu (frame Delete,
`crates/ob-app/src/main.rs` ~line 2010) removes frames without
touching `self.editing`; the editor overlay then indexes
`panel.frames[efi]` directly (line 1914) and the editing branch
indexes `self.doc.panels[idx].frames[efi]` (line 1666), while
`commit_edit` looks the frame up by index with `.get(fi)` and
writes the buffer into whatever story now sits there.
Failure scenario: double-click box C to type, right-click box A
and Delete it — if C was last, the next frame panics; if not,
the overlay now shows box B's rectangle holding C's words, and
Done writes C's text into B's story. The journal then faithfully
preserves the corrupted document.

## MAJOR

### M-1. No structural validation at load; imposition asserts panic on any panel-count-mismatched .obd — proven
Evidence: `ob_format::load` (`crates/ob-format/src/lib.rs:26`)
checks only `format_version`. `impose_bifold` asserts exactly 4
panels (`crates/ob-impose/src/lib.rs:45`), trifold 6 (line 74),
saddle asserts `n % 4 == 0 && n > 0` and indexes `panels[0]`
(lines 105-111). The app reaches these from the Print step
(`build_preview`, `print_checklist_for`, `export_imposed_pdf`)
and the CLI from `info` / `export --imposed`.
Proof (this lane, real runs): a bifold file with 3 panels and a
saddle file with 15 panels both load fine, then the CLI panics
with exit 101 at ob-impose/src/lib.rs:45 and :107 respectively.
Failure scenario: a volunteer hand-edits a bulletin JSON, or a
file from a future/older build has a panel added or dropped;
the admin opens it, clicks Print, and the app dies (the journal
saves the document, not the moment).

### M-2. The autosave tick deletes the recovery journal while the recovery offer is still undecided
Evidence: `startup()` (main.rs:860) stashes the journal as
`self.recovery`; `journal_tick` (main.rs:877-891) sees
screen == saved and journal != screen and clears the journal
files; `update()` calls `journal_tick()` (line 2537) before
`recovery_window()`, every frame, from ~2 seconds after launch.
Failure scenario: the admin launches the app, sees "Your work
from last time," gets called away and closes the app (or it
crashes) without choosing — the only copy of that unsaved work
was deleted in the first seconds of the session. The work then
exists only if it was ever saved to a file.

### M-3. Preview shaping thrashes: one-entry line cache + a fresh full shaper per miss (and per paragraph in Check)
Evidence: `lines_cache` holds a single (snapshot, panel, lines)
entry (main.rs:366); the always-visible CentralPanel calls
`shaped_panel_lines(idx)` for every panel each frame, and each
call rebuilds `full_shaper()` on a miss (line 1271) — which
opens the Font Library, reads every font file from disk, and
registers all faces with parley. The Pages canvas fights the
same single cache slot. `run_checks` calls `full_shaper()`
inside the per-paragraph loop (line 739).
Failure scenario: on a 16-page booklet, every repaint — cursor
blink and keystrokes included — re-reads the whole font library
and re-shapes every panel; typing in the Announcements box
lags or appears frozen on a modest church PC. Check on a large
library takes seconds for the same reason. Output is correct;
the app feels broken.

### M-4. Changing the bulletin shape keeps the old file path, so one Save overwrites the previous bulletin with a fresh template
Evidence: `rebuild()` (main.rs:576) replaces the whole document
and records Undo, but never touches `self.path`; `next_sunday()`
does clear it, with a comment naming exactly this hazard
(main.rs:798).
Failure scenario: the admin opens last week's bulletin, clicks
a different shape on Step 1 to look at it, then hits Save out of
habit — last week's file on disk is now a blank template dated
today. Undo still works inside the session, but once the app
closes, the old file is gone.

### M-5. Adding a font whose file name matches a different font already in the library silently replaces it
Evidence: `FontLibrary::add_bytes` (`crates/ob-fonts/src/lib.rs:192`)
dedups by content hash, then writes unconditionally to
`dir.join(suggested_name)` (line 199). The app's Add-fonts flow
and the catalog installer both arrive here by bare file name.
Failure scenario: two different fonts both named `font.ttf`
(or a zip containing same-named faces from different folders):
the second add erases the first from the library. Bulletins
laid out and pinned against the first font's bytes now shape
and print with a different font — or lose the family entirely
and fall back — with no warning anywhere. This breaks the
library's core promise (content-addressed identity).

## MINOR

1. **Italic styles get the wrong font pin.** `pin_document_fonts`
   searches for a face with `f.bold == style.bold && !f.italic`
   (`crates/ob-fonts/src/lib.rs:372`), so an italic style (the
   Scripture style) is pinned to the non-italic face's SHA-256.
   The pin is informational today; the saved fingerprint is
   simply wrong for every italic style.
2. **The PDF silently substitutes fonts that Check measured.**
   When a style's exact face cannot be embedded (rights or
   parse), `FontBook::resource_for` (`crates/ob-pdf/src/lib.rs`)
   falls back to the family's plain face, then core Helvetica,
   while the shaper still measured the real font. Check can
   approve line breaks the printed page does not have, and no
   surface tells the user a substitution happened at export.
3. **Trifold geometry disagrees across three crates.** Panel
   width is sheet/3 = 264,000 mpt (`ob-layout`), imposition
   places 267,000/258,000 mpt slots (`crates/ob-impose/src/lib.rs:66-95`),
   and both the PDF writer and the preview draw panel content
   unscaled at panel width. Template margins (0.45 in) absorb
   the 6,000 mpt difference today; a frame dragged to the panel
   edge on the flap or inside-right panel prints into the
   neighbouring panel or off the sheet edge, and Check measures
   only within the panel, so nothing warns.
4. **Check is blind to over-wide lines.** Tab lines count as
   exactly 1 line in `run_checks` (main.rs ~731-745) and
   `has_wide_line` is never consulted; `draw_panel` in ob-pdf
   has no horizontal clip, so an order line whose two sides
   exceed the frame prints its dot leader past the frame edge.
5. **Journal write failures are silent forever.** `journal_tick`
   gates on `write_journal_to(...).is_ok()` (main.rs:894) and
   never surfaces an error: if the settings folder is
   unwritable, autosave silently never protects anything.
6. **The Open guard stays armed.** `open_armed` (top bar,
   main.rs ~2570) is set on first dirty click and only cleared
   when a picker actually runs; arm it, do other work, make new
   unsaved edits, and a later Open goes straight to the picker
   with no second warning.
7. **Two different long-date formats.** `apply_date` writes
   `sunday_date_long` as "%A, %B %-d, %Y" ("Sunday, October 18,
   2026", main.rs ~262) while `ob_layout::long_date` (templates,
   Next Sunday) writes "October 18, 2026". The cover's printed
   date changes style depending on which path last set it.
8. **settings.json is written non-atomically** (`save_settings`,
   main.rs ~310, plain `fs::write`), unlike the .obd save the
   codebase treats as sacred. A crash mid-write loses printer
   memory. Low stakes, inconsistent rule.
9. **Trace accepts any page size.** `run_trace`
   (`crates/ob-cli/src/main.rs:270`) builds a frame as
   `size - 2 * margins` with no floor; a harvested page narrower
   than 0.9 in yields a frame with negative width/height in the
   draft .obd.
10. **Helper render failures keep exit code 0.**
    `render_pages` (`crates/ob-pdfium-helper/src/main.rs:145`)
    prints per-page errors but always succeeds, so `trace`
    proceeds with page pictures missing (the app does name a
    missing picture on the canvas, which is the only signal).
11. **Untrusted-file arithmetic.** `style.font_size * 13`,
    `frame.rect.y + frame.rect.h`, and similar i64 math in
    ob-pdf, `panel_lines`, and `run_checks` trust values from
    the loaded file; a crafted .obd with absurd sizes overflows
    (panic in debug builds, wrap in release). Same root as M-1:
    nothing range-checks a loaded document.

## Verdict

Not yet — close, but not trustworthy for next Sunday's bulletin
today. The engine crates are in good shape (verified baseline,
sound imposition math for well-formed documents, honest font
embedding), and most of the app's invariants hold. But the two
BLOCKERs are reachable through ordinary clicking in the app's
flagship surface — no malformed file required — and B-2 can
write one box's words into another box's story and then
autosave the result. Both are small, surgical fixes (clamp or
reset selection per panel; invalidate the edit session when its
frame is deleted), and M-1/M-2 are one validation function and
one tick guard away. Fix the two BLOCKERs and M-1/M-2, and this
lane would sign off; the MINORs can ride the next pass.
