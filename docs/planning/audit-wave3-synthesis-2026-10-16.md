# Wave-3 closing audit synthesis — 2026-10-16

Three blind Wonder lanes over the whole project at HEAD 6cbf964
(verify / review / red team), coordinator adjudication after.
Lane reports: ~/workspace/openbulletin-audit-wave3/{verify,review,
redteam}.md (kept outside the repo; the fixtures with them).

## Lane verdicts

- **VERIFY: every headline claim VERIFIED.** Tests 294/0 in its
  own run and target dir; the booklet reproduces at exactly
  31,136 bytes imposed / 34,386 reader; threading probes occur
  exactly once, leftovers zero; spoken-only absent from all
  seven export doors, present in the .obd; music 237/237
  noteheads through the real helper, staff 6.500 mm, the 3.0 mm
  request refused naming the measured 3.7 mm; the .obd tamper
  refusal verbatim; large print 11 pt → 17 pt; HTML export
  reproduces at 2,907 bytes with hostile-fixture sanitization
  of the simple cases; dump schema v2 confirmed on a real
  corpus bulletin; LEGAL posture confirmed over all 603
  packages. Its FAILED items are documentation claims (below).
- **REVIEW: no BLOCKERs, 1 MAJOR, 6 MINORs.** The claimed
  singularities are real (one continuation function, one
  picture geometry, one field-resolution core, one open door,
  one PDF funnel with the spoken-only filter on top); imposition
  and threading math read line by line and hold; recorded debts
  are recorded accurately, none worse.
- **RED TEAM: 1 BLOCKER, 10 MAJORs, and a strong HELD list.**
  All seven spoken-only export doors held under live attack;
  single-pass field resolution held against a live trap; the
  .pub CFB parser held against 15 hostile fixtures; .obd caps
  held (a header-lying 600 MB bomb refused in ~2.4 s at ~63 MB
  RSS); clip zip traversal held; command injection on both
  helper chains held. The breaks cluster in the NEWEST doors,
  which shipped without the input caps the older layers carry.

## Coordinator adjudication (territory re-checks)

- Red-team MAJOR "Template Analyzer makes spoken-only content
  Standing": CONFIRMED in code by the coordinator: `learn.rs`
  contains zero `spoken_only` handling and the dump `Span` has
  no field for the flag; the lane's harness proof stands.
- Red-team MAJOR "hostile .obd absorbs arbitrary local files":
  CONFIRMED in code by the coordinator: `absorb_sidecar_pictures`
  (ob-format) does `base.join(reference)` + `fs::read` with no
  containment (absolute paths replace the base, `../` walks),
  extension-only media typing, no size cap, and no surface
  reports it. The lane's canary proof stands.
- Review MAJOR (Announcements save recovers the spoken-only
  flag by exact-text HashMap) and red-team MAJOR 1 are the same
  wound from two sides: the export funnel is airtight; the
  flag's survival through editing and learning is not.
- Verify's doc FAILEDs and review's m2 agree independently:
  README's crate table and "will not build" lists and NEVER.md
  still forbid things Jacob later directed to be built (HTML
  export, bundled clip art). The shipped behavior follows the
  later directives; the docs lie to a new reader.

## The findings, graded (union of lanes)

BLOCKER
- B1. HTML import recursion: ~7,500 nested elements overflow
  the stack and abort the app on the UI thread (red team, live).

MAJOR
- M1. Analyzer strips spoken-only and learns it as Standing
  content that prints every week (red team, live + code).
- M2. .obd sidecar absorption reads arbitrary local files into
  the asset store (red team, live + code).
- M3. Announcements save silently drops a spoken-only flag when
  its line is edited, and collapses duplicate lines' flags
  (review, code).
- M4. .mxl read_to_string unbounded: a 4 MB file inflated to
  4 GiB in RAM (red team, live).
- M5. IDML nested-range recursion aborts at ~50k depth; the
  recursive finder is also quadratic at 20k (red team, live).
- M6. SLA nested-group recursion aborts at 200k (red team, live).
- M7. Markdown `**a ` × 200k never finishes (super-quadratic
  closer scan); the app freezes on the UI thread (red team, live).
- M8. HTML export's SVG sanitizer is a string-strip; the red
  team's bypass family survives into exported pages (spaced /
  entity-encoded javascript: hrefs, foreignObject, set,
  in-SVG @import, external use/image) (red team, live).
- M9. No timeout on any helper call except the .pub bridge;
  a sleeping helper hangs imports forever (red team, live).
- M10. Helper protocol/schema versions are never checked on
  either side; a helper answering schema 999 is accepted
  (red team, live).
- M11. Release pipeline: staging can silently omit pdfium.dll
  or its LICENSE (no stop-on-error, no assertions); actions
  and downloads unpinned/unverified; any v* tag publishes an
  unsigned release with no test gate (red team, file:line).
- M12. Announcements rebuild also flattens inline runs in the
  weekly story (review m3, same function as M3).

MINOR (union, condensed)
- JXL decode bypasses the image size limits (review m1).
- Corrupted autosave journal silently ignored at startup.
- Year 9999 Next Sunday advances to year 10000 silently.
- Zero-width picture frame yields inf DPI, read as "sharp".
- Music floor in code is 5.0 − 0.02 mm; docs say never below 5.0.
- IDML dangling frames / inf-NaN geometry flow into dumps
  without loss notes; entity-using IDML/SLA content can vanish
  silently.
- Helpers: unbounded output collection, stdin/stdout deadlock
  shape, first-candidate-wins with no fallthrough, stderr
  quoted verbatim into user errors, `dump.images[].file` join
  unsanitized in the GUI import (needs a compromised helper).
- SVG `<use>` expansion burns ~10 s CPU per parse.
- Bundle README miscounts payloads and overstates the
  no-network claim vs LEGAL.md; windows-bundle.yml carries a
  top-level `contents: write`.
- Docs: README crate table (9 of 15 crates, "version 0" .obd),
  stale CLI/arms lists, the pre-O2 fonts sentence, the bundle
  sentence missing the Verovio helper; FORMAT.md's migration
  fixtures sentence describes a practice that does not exist;
  ROADMAP.md:15 still calls shaped caret editing a later phase.
- main.rs at 12,897 lines; selection still index-keyed (the
  recorded frame-identity debt, accurate as recorded).

## Fix order (proposed to Jacob)

Batch 1 — privacy and crash class: B1 + M5 + M6 (one
depth-capped tree-walk discipline across the three importers),
M1 (spoken-only never enters learning; counted in the mix
line), M2 (containment + magic-byte validation + size cap on
sidecar absorption, surfaced in the load path), M3 + M12
(Announcements save carries flags and runs by paragraph
position, not by text), M7 (linear markdown closer scan),
M4 (.mxl entry caps like IDML's), M8 (allowlist SVG sanitizer
for the HTML inliner, tested against the red team's survivor
fixtures).
Batch 2 — helper and decode hardening: M9 (the bridge's
timeout pattern everywhere), M10 (version checks with
plain-words refusals), JXL limits, the helper MINORs that are
cheap beside them.
Batch 3 — pipeline and paper: M11 (stop-on-error, staged-file
assertions, pinned actions, PDFium hash, test gate on tags),
the docs FAILEDs (README, NEVER.md amendments citing the
later directives, FORMAT.md, ROADMAP.md), the bundle README,
and the remaining MINORs worth their cost.
