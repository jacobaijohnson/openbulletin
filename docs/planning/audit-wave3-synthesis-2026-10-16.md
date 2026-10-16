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

## Shipped addendum (2026-10-16): disposition of every finding

Jacob ordered all of it fixed in the order above. Batch 1 =
1cea852, Batch 2 = 75b16e5, Batch 3 = the "Audit Batch 3"
commit following this addendum. Workspace after Batch 3:
329 tests passing, clippy clean.

BLOCKER
- B1 (HTML import recursion abort) — fixed, Batch 1: the DOM
  walk is depth-capped; the same discipline caps the IDML and
  SLA walks (M5, M6).

MAJOR
- M1 (Analyzer learns spoken-only as Standing) — fixed,
  Batch 1: spoken-only paragraphs never enter learning and
  are counted in the analysis mix line.
- M2 (.obd sidecar absorption reads arbitrary files) —
  fixed, Batch 1: containment to the bulletin's folder,
  magic-byte validation, a size cap, and the absorption is
  surfaced in the load path.
- M3 (Announcements save drops the spoken-only flag) —
  fixed, Batch 1: flags are carried by paragraph position,
  not by text matching.
- M4 (.mxl unbounded read) — fixed, Batch 1: entry caps on
  the .mxl reader, in line with IDML's.
- M5 (IDML depth abort, quadratic finder) — fixed, Batch 1:
  depth cap and an iterative finder.
- M6 (SLA group-depth abort) — fixed, Batch 1: depth cap.
- M7 (markdown closer-scan hang) — fixed, Batch 1: the
  closer scan is linear.
- M8 (SVG sanitizer bypass family) — fixed, Batch 1: the
  string strip is replaced by an allowlist sanitizer,
  tested against the red team's survivor fixtures.
- M9 (no helper timeouts) — fixed, Batch 2: every helper
  call (PDFium in app and CLI, the Verovio path and its
  backend probes) runs through the shared runner with a
  deadline, on the .pub bridge's pattern.
- M10 (protocol/schema versions unchecked) — fixed,
  Batch 2: both protocols check versions in both
  directions and refuse in plain words.
- M11 (release pipeline) — fixed, Batch 3, except signing:
  staging is stop-on-error with a named-file assertion
  step (app, both helpers, pdfium.dll, the PDFium LICENSE
  out of the tarball itself); actions are pinned to full
  commit SHAs; the PDFium download is checked against its
  recorded SHA-256 (from the release API's asset digest,
  confirmed against a local download); a format + tests +
  clippy gate job runs before the bundle job on tags; the
  `contents: write` permission lives only on the bundle
  job; ci.yml release-builds both helpers; the runner
  image base and its rustup toolchain are pinned to named
  versions. Still open, as before this audit: the release
  itself is unsigned (no certificate; signed installers
  are already recorded as not built in ROADMAP.md).
- M12 (Announcements rebuild flattens runs) — fixed,
  Batch 1, with M3: runs are carried by position too.

MINOR
- JXL decode bypassing the image size limits — fixed,
  Batch 2: the JXL path applies the same limits.
- Corrupted autosave journal silently ignored — fixed,
  Batch 3: startup says so once in plain words and sets
  the damaged copy aside (autosave-unreadable); startup
  never blocks on it.
- Year 9999 advancing to year 10000 — fixed, Batch 3:
  Next Sunday refuses in plain words past the end of the
  supported calendar, and the flexible date parser's
  coming-occurrence search stops at 9999.
- Zero-width picture frame yielding infinite DPI — fixed,
  Batch 3: a box with no width or height gets its own
  Check finding; the DPI measure returns none instead.
- Music floor stated as an absolute 5.0 mm — fixed,
  Batch 3 (docs): EDITOR.md, music-landscape.md, and
  music-spike.md now state the floor as 5.0 mm with
  0.02 mm of measurement slack, which is what the code
  enforces. The code did not change.
- IDML dangling frames / inf-NaN geometry; entity-using
  content vanishing — split. Impossible geometry can no
  longer reach the document: fixed, Batch 3, at the draft
  builder (spans at a place no page can hold are dropped
  and counted in the review; page sizes are guarded the
  same way). Loss notes for dangling IDML frames, and the
  silently vanishing entity content in IDML/SLA, are
  deferred: that dump path has no loss channel yet, and
  building one is its own piece of work.
- Helper MINORs — mostly fixed, Batch 2: stdout and
  stderr drain on their own threads while stdin is
  written (the deadlock shape is gone), and helper stderr
  reaches the user as a capped first line, never verbatim.
  Two pieces stand: helper output is collected without a
  byte cap (the Batch 2 deadline is the practical bound),
  and helper resolution stays first-match (explicit path,
  environment, beside the executable, then PATH) with no
  fallthrough when the chosen binary fails to start; the
  beside-exe-first order is accepted, and a failed start
  reports in plain words.
- `dump.images[].file` join unsanitized in the GUI import
  — deferred: the names come from our own helper's dump,
  so abusing the join needs a compromised helper; both
  join sites in the app are recorded here for a future
  hardening pass.
- SVG `<use>` expansion costing ~10 s per parse —
  accepted, investigated in Batch 3: the cost is one usvg
  parse on the first decode of an asset at a size. Every
  display surface already reads through a cache (the
  app's texture cache for canvas and Print preview, the
  on-disk thumbnail cache for clips), and ob-pdf dedupes
  decodes within an export, so no repeated parse exists
  to remove and no new cache was built.
- Bundle README payload count and no-network sentence;
  top-level `contents: write` — fixed, Batch 3, with M11:
  the README names all three programs and its network
  sentence matches docs/LEGAL.md.
- Docs (README crate table and stale lists, the pre-O2
  fonts sentence, FORMAT.md's migration-fixture
  sentence, ROADMAP.md's shaped-editing line, NEVER.md's
  two falsified entries) — fixed, Batch 3: README's table
  lists all 15 crates as they are; NEVER.md carries dated
  amendments for HTML export and the clip starter set,
  citing Jacob's 2026-10-15 directives; the rest state
  current behavior in place.
- main.rs at 12,897 lines, selection index-keyed —
  accepted: the recorded frame-identity debt stands as
  recorded; this round did not pretend otherwise.
- Dependency nits (quick-xml at three versions, ob-app's
  extra PNG-only `image` dependency, the lopdf scaffold
  as a runtime dependency) — deferred: duplication with
  no user-visible effect; they belong to the next
  dependency pass.
- Music in the web edition loses the engraver's style
  block — accepted (Batch 1 note): the allowlist
  sanitizer drops `<style>` by design; the notation
  itself carries over.

Verification note: during Batch 3's first full-suite run,
one Batch 2 helper test (a fake helper answering through
the runner) failed once under full parallel load; it
passed alone and the full suite passed on re-run
(329/0). Recorded as a flake, not reproduced.
