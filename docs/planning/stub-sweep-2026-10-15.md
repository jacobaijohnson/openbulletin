# Stub sweep, 2026-10-15: every door, in the shipped shape

Ordered by Jacob after his first hands-on run (source build, church
Windows PC): the PDF door failed with a "helper not available"
message, and he directed a sweep of all of them.

Method: static audit of the current tree (HEAD `2592ef7`) against
three questions per user-visible item:

1. Does it do what its label promises, end to end?
2. Does it still work in the shipped bundle layout (app exe +
   helper exe + pdfium.dll side by side in one folder, no dev
   paths, no toolchain dir, no environment variables, no other
   programs a church PC would not have)?
3. Does its failure message tell the plain truth, with a real way
   forward?

Classifications: **STUB** = the label promises, the code does
nothing or cannot work in the shipped shape. **DEAD-END** = works,
but a failure leaves the user stuck, or the message names no way
forward. **COSMETIC** = works; the wart is wording, sizing, or
polish. Everything not listed as a finding was checked and works
as labeled (see the per-item table).

Baseline observed this sweep: `cargo test --workspace` = **78
passed, 0 failed**.

A note on the error Jacob hit: the text he quoted ("the PDF
reader (ob-pdfium-helper) is not available: program not found. It
belongs beside the app; reinstalling brings it back") exists
nowhere in the current tree; commit `dec4793` already replaced
that wording. His build predated the fix. The root cause on his
machine was real, though: a from-source build has no helper
beside the exe. Finding 1 is why the same failure can still bite
inside the shipped bundle on Windows.

## Headline findings

### 1. STUB (Windows, bundle): the app's beside-the-exe probe for the helper can never match

`crates/ob-extract/src/helper.rs:17-21` (`resolve_helper`):

```rust
let sibling = exe.with_file_name("ob-pdfium-helper");
if sibling.exists() { return sibling; }
```

On Windows the file in the bundle is `ob-pdfium-helper.exe`.
`with_file_name("ob-pdfium-helper")` builds a path with no `.exe`,
so `exists()` is always false in the exact layout
`.github/workflows/windows-bundle.yml` ships. Discovery falls
through to the bare name `"ob-pdfium-helper"` and depends on the
OS process search finding it (with `.exe` appended) in the
application's folder. That search is outside the project's
control, differs by launch context, and is untested; if it misses,
the flagship PDF door fails in the shipped bundle exactly as it
did on Jacob's source build. The CLI repeats the identical
pattern at `crates/ob-cli/src/main.rs:456-461`, so
`openbulletin-cli trace` has the same Windows exposure.
The helper's own library lookup does this correctly by contrast:
`crates/ob-pdfium-helper/src/main.rs:28-37` probes `pdfium.dll`
by its real Windows name beside the helper exe.

### 2. DEAD-END: the Publisher door's fallback message points at a button that no longer exists

`crates/ob-app/src/main.rs:2822-2828` (no-LibreOffice error in
`start_pub_import`) tells the user to save a PDF from Publisher
and "use \"Open a PDF bulletin...\" above". The one-door change
removed that button; the Start step now has only "Open an old
bulletin..." (`main.rs:3988`). The way forward it names cannot be
followed as written. The instruction still lands (the same door
accepts the PDF), but the label it cites is gone.

### 3. DEAD-END: one unreadable PDF kills the whole Template Analyzer run, and the error does not say which file

`crates/ob-app/src/main.rs:3066-3068` (`start_analyzer`): every
picked file is dumped with `?` in a loop, so the first bad file
aborts the study of all of them. The surfaced error is the
helper's text for that file; it never names which of the 3+ files
failed. The user's only move is to guess and re-pick. The same
all-or-nothing shape is in the single-PDF rescue
(`main.rs:2740-2745`): if the text dump succeeds but page
rendering fails, the whole import fails instead of arriving
without its tracing pictures.

### 4. DEAD-END (partial bundles): a missing pdfium.dll surfaces as raw library debug text

If the helper exe is found but `pdfium.dll` is not beside it
(e.g. the user copied only the two .exe files out of the bundle),
the helper exits 1 with `cannot load PDFium library: ...`
(`crates/ob-pdfium-helper/src/main.rs:75-78`), and the app shows
it verbatim after "the PDF reader could not read that file:"
(`helper.rs:36-39`). That is a developer message, not a way
forward; nothing tells the user a file from the download is
missing or where it belongs.

### 5. COSMETIC (layout): the center preview does not adapt to the window

`crates/ob-app/src/main.rs:4309-4327`: the congregation preview
renders every panel at a fixed scale of 0.85 inside a
vertical-only ScrollArea. Enlarging the window never enlarges
the preview; narrowing it clips wide panels on the right with no
horizontal scroll. The Print step's sheet preview does adapt
(`draw_sheet`, `main.rs:4673-4676`, width-fit capped at 560) and
the Pages canvas adapts (`main.rs:1968`), so the main preview is
the odd one out, and it is the first thing a user resizes the
window for. Related: the top bar is one non-wrapping horizontal
row of 10+ controls with no minimum window size set (viewport,
`main.rs:4387`, sets only an initial 1180x820), so a narrow
window pushes Back/Next off the right edge. The Print step's
side panel asks for 560px via `default_width` (`main.rs:3929`),
which egui treats as an initial value; once the panel has a
stored width the step change may not widen it (unverified at
runtime).

## Per-item table

Verdict OK means: wired end to end, works in the bundle layout,
failure text honest. Only non-OK items carry a class.

### Plumbing: helper, library, paths

| Item | Verdict | Evidence |
| --- | --- | --- |
| Helper discovery, beside exe | **STUB on Windows** (Finding 1) | ob-extract/src/helper.rs:17-21; ob-cli/src/main.rs:456-461 |
| Helper discovery, $OB_PDFIUM_HELPER / PATH fallback | OK (override is dev-only by design; PATH fallback is the weak link in Finding 1) | helper.rs:14-24 |
| Helper spawn failure message | COSMETIC: wording is now truthful for the bundle ("travels in the same download") but the io error is discarded, so "not with the app" is also asserted when the file exists but cannot be executed | helper.rs:27-34 |
| PDFium library discovery in helper | OK in bundle shape: env override, then beside-helper by real per-OS name | ob-pdfium-helper/src/main.rs:21-40 |
| PDFium library fallback path | COSMETIC: last resort is the dev path `$HOME/workspace/.toolchain/pdfium/lib/libpdfium.so`; dead weight on shipped machines, harmless | ob-pdfium-helper/src/main.rs:38-39 |
| Helper render mode page failures | COSMETIC (silent partial failure): a page that fails to render is logged to the helper's stderr and skipped; exit code stays 0 and the app never checks the count, so a rescued draft can lack a tracing picture the review text promises ("the old pages sit under the new ones") | ob-pdfium-helper/src/main.rs:165-185; main.rs:2985-2989 |
| Helper crash / bad PDF | OK: non-zero exit becomes a plain failure; dump JSON parse guarded | helper.rs:36-45 |
| Settings + journal location | OK: `%APPDATA%\OpenBulletin` on Windows, XDG/home elsewhere; no dev paths | main.rs:361-390 |
| Font library location | OK: same per-OS app-data scheme | ob-fonts/src/lib.rs:129-146 |
| Import work folders | OK with a note: temp dir + pid (`openbulletin-import-{pid}`), shared name between the PDF and .pub doors but runs are serialized by `import_rx`; two app instances can collide (the instance lock is courtesy-only, a known accepted item) | main.rs:2738, 2817, 3055 |

### Top bar and the five steps

| Item | Verdict | Evidence |
| --- | --- | --- |
| New bulletin / Pages / Fonts / Keys buttons | OK | main.rs:3859-3875 |
| Undo / Redo (buttons + keys) | OK: gated during typing, real stacks | main.rs:3877-3897 |
| Open bulletin... / two-click unsaved guard | OK | main.rs:3899-3901, open_flow 3220-3242 |
| Save bulletin (+ first-save picture move) | OK: errors suggest Documents folder | save_flow main.rs:3245-3290 |
| Back / Next step buttons | OK, but see Finding 5 (pushed off-screen at narrow widths) | main.rs:3911-3927 |
| Step 1: last week's bulletin | OK | main.rs:3952-3985 |
| Step 1: church profile fields | OK: commit on focus loss, undo recorded | main.rs:4025-4081 |
| Step 2: date field, Set date / This Sunday / Next Sunday, Sunday name, reading | OK | main.rs:4084-4129 |
| Step 1/2: Make next week's bulletin | OK | main.rs:4129, next_sunday |
| Step 3: announcements editor + Save | OK | main.rs:4132-4153 |
| Step 4: Check again + Fix buttons (date/words/Pages) | OK: buttons jump to the right step | main.rs:4155-4172 |
| Step 5: checklist + sheet preview | OK | main.rs:4176-4182, 1581-1595 |
| Step 5: Open the print copy | COSMETIC: success is reported when the OS launcher spawns, not when a viewer actually opens; with no PDF association the `start` failure is silent and the app still says "opened in your PDF viewer" | main.rs:4184-4197; open_with_os 4783-4800 |
| Step 5: Save print copy / reading copy / test sheet | OK: real exports, save dialogs, honest errors | main.rs:4199-4267 |
| Step 5: printer learning (name, known-printer chips, three answer buttons, persisted) | OK | main.rs:4230-4284 |
| Step 5: Start next Sunday | OK | main.rs:4286-4288 |
| Status line (incl. red "Something needs attention:") | OK | main.rs:4291-4302 |

### Import doors (Start step)

| Item | Verdict | Evidence |
| --- | --- | --- |
| One door: "Open an old bulletin..." routing by extension, then magic bytes | OK; one picker wart below | main.rs:2851-2894, 3986-3997 |
| The door's file picker | COSMETIC/DEAD-END edge: only the "Old bulletins" filter is offered, no "All files"; on Windows a file with an unusual or missing extension cannot be selected at all, which makes the magic-byte fallback unreachable for exactly the files that need it | main.rs:3989-3991 |
| PDF rescue (dump, render, draft, review) | OK in bundle shape except Findings 1, 3, 4 | start_import main.rs:2729-2757 |
| IDML import | OK: in-process, no external program, honest "pictures do not come over" note shown in the review | start_idml_import main.rs:2760-2795 |
| .pub via LibreOffice bridge | OK when LibreOffice is installed in a standard place; candidates cover both Program Files roots, the .app, /usr/bin, then PATH; $OB_SOFFICE override dev-only by design | pub_bridge.rs:19-81 |
| .pub no-LibreOffice message | **DEAD-END** (Finding 2): cites the removed "Open a PDF bulletin..." button | main.rs:2822-2828 |
| .pub conversion profile URL | COSMETIC with teeth: the LibreOffice profile is passed as a hand-built `file:///` URL with no percent-encoding, so a work path containing spaces or non-ASCII (common under `C:\Users\<name with space>\AppData\Local\Temp`) can make LibreOffice refuse the profile and fail the conversion with the generic "could not turn that Publisher file into a PDF" | pub_bridge.rs:126-141 |
| Native .pub reader (phase 1, CFB) | Not a stub: no label anywhere promises it. It is a library module + `pub_probe` example only; no app or CLI path calls it. The .pub door today is the bridge, and the app says so | ob-extract/src/lib.rs:16; examples/pub_probe.rs; main.rs:2797 |
| Template Analyzer (3+ PDFs) | **DEAD-END** on one bad file (Finding 3); otherwise OK: family math and review are real | start_analyzer main.rs:3055-3083; analyzer.rs:125 |
| Import review windows (Accept/Edit/Cancel, badges, unsaved-work arm) | OK | main.rs:2947-3048, 3126-3195 |
| Analyzer base-page render | OK with the Finding-3 shape: render failure of the base bulletin fails the whole study even though the template was learned | main.rs:3072-3075 |

### Windows, panels, resizing

| Item | Verdict | Evidence |
| --- | --- | --- |
| Main window | COSMETIC: resizable by OS default, but no minimum size, so content clips instead of the window refusing (see Finding 5) | main.rs:4387 |
| Steps side panel | OK: resizable(true); Print width caveat in Finding 5 | main.rs:3929-3932 |
| Top panel | COSMETIC: single non-wrapping row (Finding 5) | main.rs:3856-3928 |
| Center preview | **COSMETIC** (Finding 5): fixed 0.85 scale, vertical-only scroll | main.rs:4305-4327 |
| Pages window | OK: resizable, min size, canvas adapts to width, ScrollArea both directions (canvas max_height 820 is a fixed cap inside the window) | main.rs:1849-1854, 1944-1948, 1968 |
| Fonts window | OK: resizable; the list's 320px max_height is fixed regardless of window height (minor) | main.rs:3430-3437, 3441 |
| Keys window | OK: resizable; capture, swap-on-conflict, reset all wired and persisted | main.rs:3352-3412, capture_key 3293-3347 |
| Import / Analyzer review windows | OK: resizable; the item list caps at 300px max_height inside a resizable window (minor) | main.rs:2947-2951, 2979 |
| Recovery window ("Your work from last time") | OK: deliberately not resizable (a two-choice dialog); Bring it back / two-click Throw away both wired | main.rs:1504-1550 |

### Fonts window contents

| Item | Verdict | Evidence |
| --- | --- | --- |
| Library list with rights in plain words | OK | main.rs:3437-3460 |
| Per-style font pickers | OK: set, pin, undo, note | main.rs:3490-3511 |
| Add fonts... (ttf/otf/zip, per-file results, rights caution) | OK | main.rs:3513-3590 |
| Catalog search + Get this font | OK: worker-threaded; failure shown as "Font catalog: {e}" with the catalog's own plain prefix ("cannot reach the font catalog") | main.rs:3592-3660; poll_catalog 1481-1485; catalog.rs:158 |
| Church Starter Pack install | OK: per-family failures reported by name | main.rs:3662-3695; poll_catalog 1464-1480 |
| Fonts on this computer + Add a copy | OK: standard OS font dirs per platform; copies in, never links | main.rs:3697-3760; ob-fonts/src/lib.rs:351-380 |

### Print-step extras and dialogs

| Item | Verdict | Evidence |
| --- | --- | --- |
| Recovery dialog on launch (autosave journal) | OK | main.rs:1504-1550, journal_paths 385-390 |
| Unsaved-work guards (open, last week, imports) | OK: two-click arm pattern everywhere | open_guard; main.rs:2997-3025 |
| Guide/frame right-click menus, exact-inch fields | OK (swept for wiring; every menu item has a handler) | main.rs:2554-2695 |

### CLI (openbulletin-cli)

| Item | Verdict | Evidence |
| --- | --- | --- |
| new / export / next-sunday / info | OK | ob-cli/src/main.rs:137-223 |
| trace | **STUB on Windows** via Finding 1's duplicate resolver; otherwise OK, errors name the helper path | ob-cli/src/main.rs:329-463 |
| fonts list/add/system/search/get/pack | OK; network verbs fail with catalog's plain errors | ob-cli/src/main.rs:224-326 |
| CLI coverage claim | COSMETIC (docs): the header says "every engine operation... If it cannot be done here, the GUI is hiding engine problems", but the Analyzer, IDML import, and .pub bridge have no CLI verbs at all | ob-cli/src/main.rs:1-2 |

### README and shipped-shape promises

| Item | Verdict | Evidence |
| --- | --- | --- |
| Imports section names buttons that no longer exist ("Open a PDF bulletin...", "Open a Publisher (.pub) file...") | COSMETIC (docs): predates the one-door change | README.md, Imports section |
| "Native .pub reading stays out of scope, on purpose" | COSMETIC (docs): contradicted by Jacob's later decision and shipped phase 1; the Tier-2 synthesis (2592ef7) now governs | README.md, Imports section |
| "That swap is planned, not built" (font matching on import) | COSMETIC (docs): metric-twin matching shipped in the draft builder | README.md; ob-extract/src/draft.rs:44-45 |
| Crate table: ob-extract is "a first step... A planned Template Analyzer will use this later" | COSMETIC (docs): the Analyzer ships in the app | README.md, Crates table |
| "Planned: familiar keys" | COSMETIC (docs): keymap presets shipped (Keys window) | README.md:40-44 |
| "There is no installer yet" in Run it | COSMETIC (docs): true for an installer; the one-download Windows zip now exists via windows-bundle.yml and is not mentioned | README.md:100-102; .github/workflows/windows-bundle.yml |
| CLI verb list in "Works today" omits trace and fonts | COSMETIC (docs) | README.md:66-69 |

## Path and program assumptions, inventoried

Everything the shipped shape depends on, in one place:

- Helper exe: beside the app (broken probe on Windows, Finding
  1), else `$OB_PDFIUM_HELPER`, else PATH.
- pdfium.dll: beside the helper exe by real name (correct), else
  `$OB_PDFIUM_LIB`, else a dev-machine path under `$HOME`.
- LibreOffice (only for the .pub bridge): `$OB_SOFFICE`, both
  Program Files roots, `/Applications/LibreOffice.app`,
  `/usr/bin/soffice`, then PATH (with `.exe` handled).
- Opening the print copy: `cmd /c start` (Windows), `open`
  (macOS), `xdg-open` (everything else). A Linux machine without
  xdg-utils gets the honest "could not open the print copy".
- Settings, journal, and the font library: per-OS app-data
  folders via APPDATA / XDG / HOME; no dev paths.
- Font catalog and downloads: HTTPS to Fontsource / Google Fonts
  hosts, user-gesture only; proxy env vars honored.
- Nothing in the app or CLI reads the workspace toolchain dir,
  the repo tree, or any `~/workspace` path at runtime. The one
  dev-shaped string left is the helper's last-resort PDFium path
  (above), which the bundle never reaches.

## Counts

- STUB: 1 root cause, 2 call sites (app helper discovery, CLI
  helper discovery), both Windows-only.
- DEAD-END: 3 (stale button name in the .pub fallback, Analyzer
  all-or-nothing with an unnamed failing file, raw PDFium-library
  error on partial bundles). Plus one picker edge (no All-files
  filter) that can strand extensionless files.
- COSMETIC: the rest, led by the non-adapting center preview and
  a cluster of README lines that describe last week's app.

Nothing else user-facing was found that does nothing: every
button in the app and every CLI verb is wired to a working path,
and the only feature with no door at all (native .pub phase 1)
is labeled nowhere as available.
