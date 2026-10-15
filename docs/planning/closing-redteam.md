# Closing audit — RED TEAM lane report

Date: 2026-10-15. Method: hostile inputs crafted in /tmp and run
against the real CLI (`openbulletin-cli`), the format loader, and
the font library, plus code-traced attacks on the GUI workflows
that cannot run headlessly (no display in this sandbox). The repo
was not modified; nothing was committed.

Baseline verification: `cargo test --workspace` run by this lane:
**53 passed, 0 failed.** (The lane brief said 62; the observed
number on this tree is 53. Every per-suite line was summed from
the run output, not trusted from memory.)

Severity key: BLOCKER = loses user work or crashes on realistic
input. MAJOR = serious harm under plausible conditions. MINOR =
harm needs unusual conditions, or the harm is small.

---

## BLOCKER 1 — A shared .obd with the wrong panel count crashes the app (panic in imposition)

**Where:** `crates/ob-impose/src/lib.rs:43` (`assert_eq!` bi-fold),
`:68` (tri-fold), `:105-108` (saddle `assert!` + `panels[0]`),
reached through `impose()` at `:150-158`. The loader that feeds
it performs **no validation at all**:
`crates/ob-format/src/lib.rs:21-31` checks only the format
version. `crates/ob-model` likewise accepts any counts and any
geometry.

**Reproduction (all run, all observed):**

```sh
# format-valid JSON, preset letter-bifold, 3 panels
openbulletin-cli info bifold3.obd
# thread 'main' panicked at crates/ob-impose/src/lib.rs:45:5:
# assertion `left == right` failed: bi-fold needs exactly 4 panels
# exit code 101

openbulletin-cli info saddle0.obd    # saddle-booklet, 0 panels
openbulletin-cli info saddle5.obd    # saddle-booklet, 5 panels
# both panic at crates/ob-impose/src/lib.rs:107:5:
# "saddle-stitch needs a multiple of 4 pages"

openbulletin-cli info trifold4.obd   # letter-trifold, 4 panels
# panics at lib.rs:68:5
openbulletin-cli export saddle5.obd --imposed --out s5.pdf   # same panic
```

**Why it matters:** an .obd is a file that travels — emailed
between churches, produced by the template analyzer, edited by
hand (the format docs advertise JSON as "diffable, scriptable").
Any of those can produce a document whose preset and panel count
disagree (a user deletes a page from a booklet draft, an analyzer
builds 5 pages, a hand edit slips). In the GUI the same `impose()`
is called by the Print step's sheet preview
(`crates/ob-app/src/main.rs:3204`, `build_preview`), by the print
checklist (`main.rs:3319`, `print_checklist_for`), and by both
print-copy export buttons (`main.rs:2861`, `2878`). So: open the
file, click Print, the whole app dies. The engine's most-tested
crate enforces its core contract with `assert!` against data the
loader never checked. Imposition must return a Result (or the
loader must reject count/preset mismatches with a plain-words
error); asserts on untrusted file data are crash-on-open bugs.

## BLOCKER 2 — The autosave journal deletes the recovered work while the recovery window is still asking about it

**Where:** `crates/ob-app/src/main.rs`: `startup()` (line 858),
`journal_tick()` (line 877), `recovery_window()` (line 967),
called in that order every frame from `update()` (lines 2537-2538).
Code-traced; the GUI cannot run headlessly here.

**The sequence:**

1. A previous session ends with unsaved work. The journal
   (`autosave.obd` + `autosave.txt` beside settings.json) holds
   the only copy.
2. Next launch: `startup()` sets `saved_snapshot` to the fresh
   default document and, finding the journal differs, stores it
   in memory as `self.recovery` and sets `journal_snapshot` to
   the journal's snapshot. The files stay on disk. Good so far.
3. About two seconds later the first `journal_tick()` runs —
   **before** `recovery_window()` renders, with no check that a
   recovery decision is pending. The screen still shows the
   default document, so `snap == saved_snapshot`, and since
   `snap != journal_snapshot`, the tick does exactly what it is
   designed to do for the "user saved everything" case:
   `clear_journal_from()` **deletes both journal files**
   (main.rs:884-890).
4. The "Your work from last time" window is still on screen,
   offering work whose only crash-proof copy the app just
   destroyed. If the admin is interrupted (a phone call, a power
   flicker, closing the laptop, the app crashing for any other
   reason in this list) before clicking "Bring it back", the
   unsaved bulletin is gone permanently. The safety mechanism
   consumed its own backup during the decision window.
5. Second act: if the admin does click "Bring it back",
   `journal_snapshot` is set to the restored document's snapshot
   (main.rs:1008) — but the journal files no longer exist, and
   the tick only rewrites the journal when the snapshot
   *changes*. So the restored work has **no on-disk backup at
   all** until the admin makes another edit. A crash in that
   window loses the same work a second time.

**Fix direction:** `journal_tick` must not clear or overwrite the
journal while `self.recovery.is_some()`, and "Bring it back"
must force a fresh journal write of the restored state.

## BLOCKER 3 — "Use last week's bulletin..." throws away unsaved work with no guard at all

**Where:** `crates/ob-app/src/main.rs`, Step 1 button handler
(≈line 2686), versus the top-bar Open guard (lines 2575-2614).

The top-bar Open button got the two-click guard in the earlier
audit round (finding B3): with unsaved work on screen, the first
click only arms the button and warns. The Step 1 "Use last
week's bulletin..." button — the door the guided flow actually
steers the admin through — has **no guard**: it runs
`adopt_document(doc, true)` (history cleared), then
`mark_clean()`, which sets the saved snapshot to the new
document and **deletes the autosave journal**. Whatever unsaved
work was on screen is destroyed in the same click: not in the
file, not in Undo (history cleared), not in the journal (just
retired). One friendly button, total loss, no warning. Two
doors into the same room must share one lock.

Related decay in the same guard (would be MAJOR on its own):
`open_armed` is never reset except by completing an open
(main.rs:519, 2582-2590). Arm it, change your mind and Save
instead, edit something new, then click Open days later in the
same session: the button is still armed, so the *new* unsaved
work is discarded with no warning. A safety latch that stays
latched forever eventually fires on the wrong work.

---

## MAJOR 1 — Font zip path traversal on Windows (unsanitized `\` in entry names)

**Where:** `crates/ob-fonts/src/lib.rs:213-238`
(`add_zip_bytes`), `:192-202` (`add_bytes`).

Zip entry names are reduced with `name.rsplit('/')`, which
strips only forward slashes. An entry named `..\..\pwned.ttf`
keeps its backslashes and is passed as `suggested_name` to
`add_bytes`, which does `self.dir.join(suggested_name)` and
writes the bytes — no component check anywhere.

**Reproduction (run):** a zip containing `..\..\pwned.ttf`
(valid font bytes), `../../pwned2.ttf`, and
`..\..\license-pwn.txt` was added via `fonts add`. On this Linux
box the library afterwards contained files *literally named*
`..\..\pwned.ttf` and `..\..\license-pwn.txt` — proof the name
reaches the filesystem unsanitized. On Windows, where `\` is a
path separator and the desktop app is a primary target, the
same join climbs out of the library folder: `..\` segments are
unbounded in an entry name. The font branch can write any
parseable font's bytes to any `.ttf`/`.otf` path; the license
branch (`.txt`/`.md` whose name contains "license"/"ofl"/
"copying") writes **fully attacker-controlled bytes**. The
threat model is the documented one: church admins download
font zips from font sites. A name like
`..\..\..\..\AppData\Roaming\Microsoft\Windows\Start Menu\Programs\Startup\license-ofl.txt`
is a write primitive in a folder that executes things. The fix
is one place: reject or strip any name containing `\` or `..`
components in `add_bytes` itself, so every caller is covered.

## MAJOR 2 — Font zip decompression is unbounded (zip bomb shape)

**Where:** `crates/ob-fonts/src/lib.rs:223-225`:
`read_to_end(&mut entry, &mut content)` with no size cap, per
entry, before any inspection.

**Reproduction (run):** a 306 KB zip whose single entry expands
to 300 MB of zeros. `fonts add` read it all into memory (peak
child RSS measured at **313 MB**) before rejecting it as not a
font. The zip format lets the same trick reach multiple GB from
a ~1 MB file; on a modest church PC that is an out-of-memory
kill. The GUI's Add-fonts path calls the same function inside
the app process, so the bomb takes the whole app down with it
(the journal limits work loss, but see BLOCKER 2 for the
journal's own window). Entries need a hard cap (a real font face
is well under 50 MB) enforced *while* reading, e.g.
`Read::take`.

## MAJOR 3 — Adding fonts is quadratic: every add re-scans and re-parses the whole library

**Where:** `crates/ob-fonts/src/lib.rs:195-199`: `add_bytes`
calls `self.scan()` (read + parse + SHA-256 every file in the
library, `:171-190`) to check for a duplicate, so importing N
fonts costs O(N²) full font parses.

**Reproduction (run):** zips of the same real font with distinct
padding (distinct hashes, so all are accepted):

- 30 fonts: **19.9 s** wall (7.4 s CPU) in this debug build.
- 300 fonts: still running after **10 minutes** when this lane
  killed it; consistent with the quadratic projection
  (~30-35 min), not with the ~200 s a linear import would take.

Debug numbers exaggerate the constant, not the shape; release
builds shrink the seconds, not the exponent. Real font families
ship as zips of dozens of faces, and the Starter Pack plus a
couple of family downloads is exactly the first-run experience.
Dedup should hash first and compare against a cached hash set,
not re-read the library from disk per file.

## MAJOR 4 — Extreme values in a valid .obd overflow arithmetic in the PDF writer

**Where:** `crates/ob-pdf/src/lib.rs:186-187`
(`style.font_size * 13 / 10`); loader accepts the values
(`crates/ob-format`, `crates/ob-model`: no ranges anywhere).

**Reproduction (run):** a format-valid .obd identical to the
control bulletin except `font_size: 9223372036854775807`:

```
openbulletin-cli export huge.obd --out huge.pdf
# thread 'main' panicked at crates/ob-pdf/src/lib.rs:187:26:
# attempt to multiply with overflow      (exit 101)
```

This binary is a debug build, where overflow panics. The
workspace `Cargo.toml` sets no `[profile.release]
overflow-checks`, so a release build instead wraps to a garbage
line height and silently emits a corrupt PDF. Either way the
file is accepted without a word. Same class, quieter:
negative panel sizes and negative font sizes (tested:
`neggeo.obd`, `negfont.obd`) export "successfully" into PDFs
with negative MediaBox dimensions and negative font sizes —
invalid output presented as success. The loader needs a sanity
pass (positive sheet/panel sizes, font sizes in a sane range,
rects with non-negative w/h) with a plain-words refusal.

## MAJOR 5 — After recovery, Save writes wherever a two-line text file says

**Where:** `crates/ob-app/src/main.rs:110-127`
(`read_journal_from`), `:997-1010` (recovery accept),
`write_journal_to` at `:91-108`.

The journal note `autosave.txt` is a plain, unsigned two-line
text file: a timestamp and a path. On "Bring it back" the app
sets `self.path` to that path unchecked. The next ordinary Save
click writes bulletin JSON over **whatever file that line
names**. In the normal flow the app wrote the note itself, and
the recovery window does show the path (an earlier fix). But
the note lives in the config folder — synced by OneDrive/
Dropbox on many church PCs, editable by any local process, and
stale-path accidents happen without malice (the original file
renamed, the name reused by a different document). An integrity
check costs little: refuse or re-confirm when the journal's
source path does not name an existing .obd, or when its content
is not the document the journal was written from.

## MAJOR 6 — No single-instance protection: two apps share one journal and one settings file (paper)

**Where:** `crates/ob-app/src/main.rs`: `journal_paths()`
(:80-89), `write_journal_to` (:91), `save_settings` (:302 —
also note it writes `settings.json` with a plain non-atomic
`fs::write`, unlike the journal's tmp+rename).

Nothing stops the admin double-clicking the icon twice. Both
instances tick every 2 s into the same `autosave.obd`, using
the **same temp name** (`autosave.obd.tmp`, from
`ob_format::save`'s `with_extension`). Interleavings: A writes
tmp, B overwrites tmp, A renames B's content in as its own,
B's rename fails and retries — the journal usually converges to
one instance's work, but the doc/meta pair can cross (A's
bulletin with B's note), which feeds straight into MAJOR 5's
trusted-path problem, and two simultaneous `save_settings`
writes can tear `settings.json` (the loader then silently
falls back to defaults and the learned printer answers are
gone). A lockfile or single-instance check is the standard
guard; at minimum the journal temp name should be per-process.

---

## MINOR findings

1. **Silent reference rot.** A frame whose `story_id` names no
   story is skipped without a word, and duplicate story ids
   resolve to the first match (tested: `badrefs.obd` exports
   "successfully"; the duplicate's text never appears, no
   warning anywhere — `crates/ob-pdf/src/lib.rs:175-181` uses
   `doc.story()` = first match). Content can vanish from print
   with the app reporting success.
2. **Corrupt journal = recovery never offered, no notice.**
   `read_journal_from` returns `None` on any parse failure
   (main.rs:110-127), including the case where the .obd half is
   fine but the note's timestamp line is mangled — the two-file
   journal write is not atomic as a pair, so a crash between
   the two writes produces exactly this. The admin is never
   told a recovery copy existed and failed to load.
3. **Corrupt settings = silent amnesia.** `load_settings`
   (main.rs:295-300) falls back to defaults on any error:
   printer memory and preferences vanish without a message.
4. **Trace renders unbounded page counts.** `run_trace`
   (`crates/ob-cli/src/main.rs`, trace section) renders every
   page of the input PDF to a ~200 dpi PNG with no page-count
   or output-size sanity check; a 10,000-page PDF is a disk-
   filling exercise. (Also noted: the PDFium helper itself runs
   unsandboxed with the user's privileges on untrusted PDFs —
   out-of-process is crash isolation, not a security boundary —
   and the helper binary is chosen via `--helper`,
   `$OB_PDFIUM_HELPER`, or a sibling file, i.e. operator trust
   by design.)
5. **System font walk follows symlinks with no cycle guard.**
   `system_fonts()`'s recursive `walk`
   (`crates/ob-fonts/src/lib.rs:301-343`) recurses into any
   directory entry where `path.is_dir()` (symlink-following)
   with no visited set or depth limit. A symlink loop inside a
   fonts directory means unbounded recursion and a stack
   overflow crash when the Fonts panel is opened. Paper
   finding; low likelihood, standard fix (track visited
   canonical paths or cap depth).
6. **Per-frame full-document serialization.**
   `update()` computes `frame_snapshot = snapshot(&self.doc)`
   (a full JSON serialization) every frame, and Undo keeps up
   to 60 full snapshots (main.rs:2495, 163-170). For the tested
   5,000-panel document (1.33 MB of JSON) that is ~80 MB of
   undo strings plus a multi-millisecond serialization on every
   repaint. Fine at church scale (16 panels); a cliff, not a
   slope, beyond it. Reasoned from code; the GUI cannot be
   profiled headlessly here.

## Attacks that HELD (verified, for the record)

- Truncated, garbage, empty, and missing-field .obd files:
  clean one-line errors, exit 1, no panic
  (`trunc.obd`, `garbage.obd`, `empty.obd`, `nofield.obd`).
- Future `format_version` (4294967295): refused with a plain
  message naming both versions.
- 200,000-deep nested JSON: parser error, no stack overflow.
- Non-font, truncated, and 4-byte files renamed `.ttf`:
  refused at `inspect` with plain errors
  ("not a readable font file").
- A font with `unitsPerEm` patched to 0 (one-byte-pair change
  to a real Liberation face): refused by the font parser at
  inspect time, so the division by `units_per_em` in the PDF
  embedder (`crates/ob-pdf/src/lib.rs:435`) is not reachable
  with a zero divisor through the library path.
- Non-PDF, zero-page, and truncated PDFs fed to `trace`:
  graceful helper failure, exit 1, error text relayed.
- Trace of the real 16-page corpus bulletin: works, 16 panels,
  saddle-booklet detected.
- Filenames with shell metacharacters (`evil;touch${IFS}PWNED.pdf`):
  passed as argv data end to end; no shell involved anywhere —
  the helper is spawned via `Command` args, and the app's
  open-the-print-copy uses a fixed temp path
  (`open_with_os`, main.rs:3351-3365). No injection file was
  created.
- A 5,000-panel document: `info` instant, reader export 4.3 s,
  2.1 MB PDF. Scale alone is not a crash vector in the CLI.
- The journal `.obd` write itself is atomic (tmp + rename in
  `ob_format::save`), and a corrupt settings or journal file
  never crashes startup — the failures above are silent, not
  fatal.

## The single most dangerous thing

**BLOCKER 2.** Every other finding needs an unusual file or a
second actor. This one needs only a tired church admin and a
phone call: the app's flagship promise is "never lose work"
(PRINCIPLES #6), yet on the launch after a crash, the app
deletes the only copy of the unsaved bulletin from disk roughly
two seconds in — while its own recovery window is still open
asking the admin what to do with it. If anything interrupts
that decision, the work the journal existed to protect is
destroyed by the journal's own housekeeping. Fix the tick's
recovery blindness first; BLOCKER 3 (the unguarded second
door) is the same promise broken from the other side.
