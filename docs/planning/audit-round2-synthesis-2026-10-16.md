# Audit round 2 synthesis — 2026-10-16

Three blind Wonder lanes over HEAD f9a63fc (code 8e28646,
the round-1 fix batches), coordinator adjudication after.
Lane reports: ~/workspace/openbulletin-audit-wave3/
round2-{verify,review,redteam}.md.

## Lane verdicts

- **VERIFY: every check VERIFIED, nothing FAILED.** 329/0 in
  its own target dir, clippy clean, the known flake did not
  recur. Round-1 fixes exercised live (depth refusals in
  milliseconds for IDML/SLA, the .mxl cap, spoken-only
  learning exclusion with the count verbatim, sidecar
  containment 4/4, helper timeout at the deadline, schema
  999 refused in the exact words). No round-1 regressions:
  booklet 31,136/34,386 exactly, music 237/237, the 2,907-
  byte HTML export reproduced byte-identical. The bundle
  workflow gates as written; the PDFium hash matches the
  GitHub API digest the lane fetched itself.
- **REVIEW: 0 BLOCKER, 1 MAJOR, 11 MINOR.** The fix code held
  up under hostile reading (runner threading and cfg gates,
  version gates with no mixed slip, the sanitizer's
  spelling write-back provably bounded, containment
  component-wise, markdown Closers predicate-identical to
  the pre-fix rescan, Batch 3 minors all correct).
- **RED TEAM: 0 BLOCKER, 4 MAJOR, 4 MINOR, 1 NOTE.** Its
  HELD list is long and specific (sanitizer 11 of 12 shapes,
  sidecar 10/10 including symlink-out and FIFO, journal live
  under Xvfb, settings 10 hostile files, protocol doors,
  hostile field values into PDF with zero operator breaks,
  non-finite floats refused by the JSON parser itself).

## The round-2 findings (union, graded)

Three of the red team's four MAJORs are one shape: a
parser whose cost discipline stops at a boundary the file
controls.

- RT-1 (MAJOR): the helper runner's deadline governs the
  child wait, but thread::scope then joins reader threads
  that block on pipe EOF; a grandchild holding the pipe
  blocks the caller for the holder's lifetime (measured
  8.02 s against a 2 s deadline, holder-lived). The module
  doc's claim about pipes closing is false for forking
  helpers.
- RT-2 (MAJOR): a deeply nested SVG (2,000 <g>) stack-
  overflows usvg inside ob_image::probe, BEFORE the
  sanitizer runs, on every graphics path. One handed-over
  clip aborts the app with unsaved work.
- RT-3 (MAJOR): HTML import measures depth after a parse
  whose cost is quadratic in depth: 10,000 divs refuse at
  26 s, 25,000+ never refuse inside 60 s. On the UI thread
  the abort became a hang of the same consequence.
- RT-4 (MAJOR): IDML reads every entry (≤32 MB each) into
  memory with NO total cap: a 6.5 MB file inflating toward
  6.4 GB was refused only by the allocator, not by design.
- R2-1 (MAJOR, review; coordinator-confirmed in code): the
  analyzer's comment promises "the dump whose neighborhood
  is largest" but the code seeds from the single closest
  pair and grows by single linkage; a duplicated oddity
  beats the real family, and with family = 2 the 80%
  standing rule degenerates to unanimity, so dated text
  classifies Standing — the round-1 M1 failure class
  reached through the algorithm.

MINORs (union): IDML entries truncate silently at the
32 MB cap with words uncounted; the sanitizer checks only
the first url( in a value; the runner collects output with
no byte ceiling (577 MB inside a 1 s deadline); CLI export
doors overwrite their own input (two lanes, same finding);
the WMF converter's timeout bounds the wait but never kills
LibreOffice; settings.json loads all-or-nothing and saves
non-atomically; keymap data is never validated on load (a
duplicated press fires both actions); dump_from_document
emits a threaded story once per frame (counts inflate);
spans addressed to unlisted pages vanish uncounted; README
font-travel sentences run ahead of the unbuilt font pin;
Next Sunday advances any occasion document +7 days with
its name intact (a Wedding file becomes a Wedding a week
later, silently); single-linkage chaining dilutes standing
detection on drifting corpora (fails safe).

NOTE: the app panicked at startup on the sandbox's Linux
font configuration (epaint "family not bound"), control-
verified identical with an empty config and no hostile
input. Jacob's Windows run is unaffected. To investigate in
the fix round: if the app's font registration can reference
an unbound family on some configurations, startup must
degrade to a bound fallback, never panic.

## Fix order

R2-A (parser/runner class): RT-1 + output byte ceiling,
RT-2 (pre-parse SVG depth/node gate in ob-image), RT-3
(pre-parse HTML depth/tag budget), RT-4 + IDML truncation
counting, sanitizer all-url( check.
R2-B (review set): R2-1 analyzer seeding (largest-
neighborhood + average-linkage growth + family < 3 refuses
standing classification in plain words), Next Sunday
occasion guard, settings salvage + atomic save, keymap
validation on load, CLI out!=in guards + atomic writes,
WMF kill, dump threading/count fixes, README sentences,
the startup font-panic investigation.
Then the loop continues: wanted fonts build, roles builds,
round 3 audit.

## Shipped addendum (2026-10-16, coordinator-verified)

R2-A (2af35e2, 342 tests): runner collects over channels
with deadline + 5 s pipe grace and never joins a blocked
reader; output capped at 512 MB with a plain-words refusal;
ob-image gates every SVG through an iterative pre-scan
(256 depth / 250,000 elements; real Verovio pages measure
2,199 elements); HTML import pre-scans source depth/tags
before html5ever (25,000 divs now refuse immediately; the
import suite runs in 0.27 s); IDML totals capped at
512 MB, and truncated parts are rescued to their last whole
paragraph with only the tail counted, via the new
ExtractionDump.loss_notes channel; the sanitizer validates
every url( in a value.
R2-B (214f3b4, 361 tests): analyzer family = largest
neighborhood + average-linkage growth; families under 3
classify nothing Standing and say why in the review;
Next Sunday occasion guard with a spoken note (heuristic:
a name set, not "Sunday", with no Sunday word; a regular
weekday service so named cannot be told apart — the guard
announces itself and the rename path, never silent);
settings per-section salvage + set-aside + atomic save;
keymap validated on load with capture's swap discipline;
all CLI export doors refuse to replace their own input and
write atomically; fonts add skips-and-names; the WMF
converter is killed and reaped at its deadline; learning
dumps emit threaded stories once and count unlisted-page
spans as losses; README font sentences corrected.
NOTE 9 resolved as a REAL BUG: reproduced byte-for-byte —
a one-frame ordering bug painted shaped preview lines
naming library families in the same frame the refresh
installed them, while egui applies new font definitions
the next frame; a fresh start with a bulletin open could
panic. Fixed by snapshotting bound families per frame and
falling back for exactly one frame; runtime-verified
under Xvfb.
Coordinator re-ran the full suite after each batch:
342/0 and 361/0.
