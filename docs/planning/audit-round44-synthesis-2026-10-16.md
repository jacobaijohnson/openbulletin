# Audit round 44 — synthesis (coordinator)

Audited tree: HEAD `6f74609` (round-43 close record)
on code tree `f16d13e` (R43-E). Three blind lanes,
read-only, each filed in the audit dir:
`round44-verify.md`, `round44-review.md`,
`round44-redteam.md`; scratch under `round44-out/`.

## Lane tallies

- **Verify: PASS on all six claims** (tally
  0/0/0). Suite 1236/0 by scripted sum, clippy
  exit 0 (honestly noted cache-served), fmt exit 0;
  fuzz byte-identical at the CURRENT baselines
  (verdict `79775844…`, 40,000 lines / 0 panics;
  composition `e8b2e73b…`, 497,024 / 0 panics; bin
  sources at the recorded md5s); rights counters
  5083 / 1024 / 5275, safety_flips=0 — M8 held,
  twenty-fifth round; booklet 31,717 / 34,977
  byte-exact; hosted CI 40461904465 / runner
  40461904458 / bundle 40470669392 re-derived from
  the API, all success on `f16d13e`.
- **Review: 0 BLOCKER / 1 MAJOR / 1 MINOR.**
  All five round-43 batches conform at their named
  seats; pin/restatement arithmetic reconciles
  exactly; no standing pin silently weakened.
- **Red team: 0 BLOCKER / 1 MAJOR / 7 MINOR.**
  Rights axes held under fresh quadruple and
  quintuple sweeps (standing_flips=0,
  safety2_flips=0); lock records stand at or under
  their recorded rates; the admitted classes held
  at all fourteen sweep arms.

## Deduped tally: 0 BLOCKER / 3 MAJOR / 7 MINOR

The exit bar is **NOT met**; the Tier-2 Opus + GPT
final gate is not triggered and is not the
coordinator's to start. Majors trend:
… → 5 → 5 → 3 → 3.

## Findings

- **F1 (MAJOR; review F44-1)** — The en dash still
  splits at the sequence-tail seats: a suffix on
  the FINAL group of a trailing / base-trailing /
  leading sequence is judged for `-`, `—`, `~`,
  `*`, `#`, `…` and every R43-B admitted mark but
  not for U+2013, which stands in no peel set and
  whose one retry (R43-A) is gated to
  governor/company final groups. Cancelled dates
  serve live at three shapes (glued governor-first
  with inert tail, spaced, spaced-token); weekday
  records are lost at three more (base-trailing,
  trailing, leading sequences), both routes.
  R43-A's scope-out rests on a presupposition —
  that the governor-first sequence closing on an
  inert group serves under the twins — which is
  not true of the closed tree: the twins GOVERN
  there (review probe44c G1, probe44d H1/H2), and
  the batch's control pin asserts only the en-dash
  reading. F43-1's failure shape (a scoping
  rationale not true of the tree) one seat over.
- **F2 (MAJOR; red team RT44-C1)** — Fullwidth
  separators and quote closers disarm companies at
  every inter-group seat: U+FF0C / U+FF1B / U+FF1A /
  U+FF0E / U+3001 / U+3002 standing between groups
  (glued or spaced) and U+FF02 / U+FF07 standing
  where R43-C judged the straight closers let a
  governed date serve live at the trailing slot,
  silently at leading, mid-line, and base-trailing
  seats; fullwidth quoted members are not consulted
  at all. One mechanism: the fullwidth block stands
  in none of the gap/separator sets the company
  walks consult. The typist path is R43-B's own
  rationale — a CJK IME in fullwidth mode produces
  these for the ordinary punctuation keys.
- **F3 (MINOR; red team RT44-C2)** — The remaining
  fullwidth siblings of judged suffix/sentence
  marks (U+FF5E ～, U+FF03 ＃, U+FF0E ．, U+FF0C ，,
  U+FF1A ：, U+FF1B ；) split at all fourteen sweep
  arms: trailing governors serve live, bare weekday
  records are lost, grouped/leading/base arms serve
  silently. Two of the marks stand in R42-F's
  remaining-marks control pin as never-judged; the
  pin predates the fullwidth adjudication.
- **F4 (MINOR; red team RT44-C3)** — Fullwidth
  dress pairs （）［］｛｝ are not dresses at the
  company seats: a base-trailing company in
  fullwidth dress serves the resolution silently,
  a leading company is not seen at all (no record),
  a fullwidth weekday member behind a judged
  governor is not consulted, and a trailing-alone
  fullwidth group carries no memory. The lone
  trailing and mid-line governors in fullwidth
  dress DO govern — the unsafe direction is
  confined to the company seats.
- **F5 (MAJOR as adjudicated, adjudication 1;
  review F44-2, graded MINOR by its lane)** —
  Quoted inert members still collapse the trailing
  company: `quoted_members_as_parens` (R43-C)
  re-dresses only spans judging as governor or
  weekday members, so `November 8, 2026
  (cancelled) "notes"` serves the cancelled date
  live in every quote dress (straight, single,
  curly) where the paren and bracket twins govern;
  the weekday and three-group shapes split the
  same way. An inert quoted span is ordinary copy —
  a quoted aside after a governor group.
- **F6 (MINOR; red team RT44-C4)** — `?` stands in
  no trim of `LineScan::split_last`: the
  trailing-alone group wearing it loses the
  For-route memory and the Keep-through where its
  sibling `!` keeps both (`next Sunday (November
  8, 2026)?` reads paren=None / keep=None; quoted
  dress identical; stacks `??` / `?~` lose both;
  bracket dress falls to a second-date reading).
  Mechanism pinned in the source (dates.rs); the
  seat is R42-D's F10 seat.
- **F7 (MINOR; red team RT44-C5)** — U+02DC SMALL
  TILDE has a documented ordinary path: the macOS
  Option+N dead-tilde followed by Space produces
  U+02DC, not ASCII `~` (reported sources named in
  the red-team report). Its split is total at the
  peel arms. Carried item (d) is partially
  re-graded (adjudication 3): U+02DC leaves the
  exotic list; the other seven marks stand as the
  closed-peel limit, no ordinary path found.
- **F8 (MINOR; red team RT44-C6)** — Zero-width
  paste artifacts (U+200B, U+FEFF) standing at a
  garnish seat disarm the judgment the visible text
  carries: `(cancelled)\u200B~` serves the
  cancelled date live, `(cancelled)\u200B(Monday)`
  serves the Sunday-after resolution silently,
  `(cancelled)~\uFEFF` serves live — while the
  served text looks judged to a proofreader. Other
  space widths at the same seats are judged as
  their space twins (held).
- **F9 (MINOR; red team RT44-A1)** — Archive,
  seventh generation: a zip64 EOCD whose CD offset
  points at a gap plant directory is judged. The
  model judges the plant directory on all four
  entry sets (spot lists identical to clean; the
  divergence is again the key record's CRC); the
  clip door fails on the idml, clip, and mxl sets
  and every door refuses on the font set, where
  every clean control reads OK. No door serves
  plant content (refusal direction throughout).
  The zip64 arm was not part of R43-D's
  walked-extent guard. Controls held: zip64-real
  clean-identical everywhere; bad-locator refused
  uniformly.
- **F10 (MINOR; red team RT44-P1)** — The quoted
  sibling of the pinned nested shape is unguarded:
  `nested_seq_mix_quoted` (the R43-E shape with
  trailing `(notes) ` groups replaced by quoted
  `"notes" ` groups) reads median interleaved
  ratios 18.87 / 25.77 / 34.63 at n = 1,600 /
  6,400 / 25,600 against the pinned shape's
  ~2.6 / 4.5 / 5.1 re-measured in the same windows
  — ~3.5× the 10× bound at the pinned size, the
  climb per size step matching the pinned shape's
  exponent. No pin covers it. The brackets sibling
  reads 5.70 / 10.49 (noisier rounds, lower
  confidence), recorded as the finding's second
  shape. No hang at realistic sizes; graded MINOR
  on the R43-E posture.

## Adjudications

1. **F5 re-graded MAJOR.** The review lane graded
   its finding MINOR on breadth (one reader family)
   and stated the re-grade argument itself: the
   trailing-slot shape serves a cancelled date live
   on ordinary copy, which is the harm round 43
   graded MAJOR for F4 at six seats, weighing
   live-serve over breadth for F4's trailing-slot
   arm. The coordinator adopts that argument and
   the round-43 precedent: live-serve of a governed
   date on ordinary copy is this family's MAJOR
   harm. F5 is MAJOR.
2. **Carried item (c) is CLOSED, not reproduced.**
   Two lanes independently: the review lane found
   the round-43 witness reads CLIP OK in its own
   lane's filed log (`doors43.log` block [5]) and
   drove eleven standard streaming shapes clean on
   the closed tree; the red team re-drove the
   round-43 witness file verbatim (CLIP OK) plus
   eleven further descriptor shapes, all
   clean-identical, and pinned the clip failure
   mechanism positively: a judged central record
   whose CRC/sizes describe bytes the local at its
   offset does not hold (judged-record/local
   mismatch) — the shape of F9 and R43-D's plant,
   never of a descriptor, whose central record
   describes the data exactly. The round-43
   observation is recorded as a conflation with the
   trailing craft's clip failure in the same run.
   No finding in either direction; the named
   mechanism path (ob-clips `raw_payload` /
   ob-model `payload_span` + central CRC) stands in
   the record for any future witness.
3. **Carried item (d) partially re-graded.** On
   RT44-C5's documented path (macOS dead-tilde;
   reported sources, named as such), U+02DC leaves
   the exotic-sibling list and is admitted beside
   its twin `~` (F7 / R44-D). The other seven marks
   (U+2012, U+2053, U+2025, U+22EF, U+FE61,
   U+2731, U+266F) stand as the closed-peel limit;
   their R43-B controls are verified honest by two
   lanes. The review lane's note stands in the
   record: U+2012 FIGURE DASH is the weakest
   remaining member (typeset phone numbers/ranges);
   if a future lane witnesses one arriving by an
   ordinary path, it is judged first.
4. **F1's fix is the peel widening.** The review
   lane's witness places the twins' governed
   reading at the tail seats in the peels
   themselves, and the G6 base-trailing parity arm
   upstream of them (its serve is the composed
   Sunday-after reading, identical for all marks
   today). R44-A therefore admits U+2013 to the
   sequence-tail peel sets (the R43-B shape) with
   G6 pinned as the parity control; the objection
   recorded in R43-A's scope-out is answered by its
   own evidence — the widening moves the
   governor-first inert-closing sequence to its
   twins' governed reading, which is the cure.
   R43-A's scope-controls pin asserts the defective
   en-dash readings at exactly these arms as
   controls; those arms are restated, flagged, and
   no others.
5. **F10's disposition is mechanism-first.** The
   batch investigates why the quoted dress costs
   ~5–7× the pinned shape (the quoted-member
   consultation's re-walk is the named suspect).
   If a bounded production fix exists, it lands
   with a biting ratio pin at the quoted shape,
   calibrated and disclosed in the R43-E shape.
   If none exists, the batch lands the calibrated
   pin alone, with the same disclosure terms (the
   pin's work stated as regression-keeping), and
   the brackets sibling is witnessed and stated
   either way. No bound is widened under load; a
   ratio miss under co-tenant load is dispositioned
   solo.
6. **HF-12 first slice approved as R44-I**, at the
   review lane's updated map (its section 6):
   `crates/ob-extract/src/lib.rs` only — one typed
   error, `OpenDoorError { TooLarge,
   Unreadable(io::Error) }`, `Display` rendering
   byte-for-byte today's plain words; the three
   door functions (`check_open_door_size`,
   `read_open_door_file`, `read_open_door_text`)
   and `extract_pdf`'s entry TooLarge site (which
   rides the same kind without widening the
   slice); a `From<OpenDoorError> for String` at
   the boundary so every caller compiles and
   behaves unchanged (the lane verified no caller
   branches on error text). `from_json`'s version
   refusal is expressly deferred to the second
   slice: it is the one surface whose wording is
   pinned by an assertion outside the file
   (helper.rs). The slice runs under the full
   batch discipline after the fix batches; fuzz
   byte-identical is its standing gate (error text
   unchanged by construction).
7. **Carried records stand into round 45:**
   (a) F7's rename-step rate record (this round's
   lanes measured at or under it — red team L7
   1/400, L9 46/2000; review's L10 witness 2/400
   with the production audit refusing 2/2; red
   team's L10 21/400, audit 21/21, L11 composed
   0/400 — no restatement: landing counts under
   load are shape evidence, and no rate materially
   above the record was witnessed); (b) the
   L10/L11 step-level sibling as a standing
   observation (composition still closed by the
   only production caller); (e) the four standing
   records (peeled-tail residual, split-position
   governor uniformity, lock leading-newline pid
   spelling, recycled-pid residual) — all STAND,
   re-witnessed by two lanes; (f) the R40-C
   calibration note — stands honest.
8. **Red-team observations recorded, not graded:**
   emoji garnish (codepoint-choosing path, the
   garnish-vocabulary limit); the grouped `(off)~`
   governor-vocabulary asymmetry (a word no batch
   has named); the non-fullwidth exotic dress
   pairs (`«»`, `「」`, `⟪⟫`, etc.) splitting at
   the same company seats as F4 (the closed-dress
   limit standing; no ordinary Latin-typist path
   claimed). These stand in the record beside the
   closed-peel limit; a witnessed ordinary path
   re-opens any of them.

## Batch plan

- **R44-A** — F1: U+2013 joins the sequence-tail
  peel sets (`trailing_run_peeled_end`,
  `trailing_group_sequence`,
  `base_trailing_sequence_mismatch`, the leading
  sequence readers). Pins: the review lane's
  G1–G5/G8/H witnesses (glued, spaced,
  spaced-token; trailing, base-trailing, leading;
  governor and weekday arms; both routes) with G6
  as the parity control and G7/G9/G10 held.
  Restatement, flagged: exactly the en-dash tail
  arms of `r43a_en_dash_scope_controls_unmoved`.
  Every other standing suffix/peel pin unweakened.
- **R44-B** — F2: the fullwidth separators
  (U+FF0C, U+FF1B, U+FF1A, U+FF0E, U+3001, U+3002)
  join the gap/separator sets the company walks
  consult, and the fullwidth closers (U+FF02,
  U+FF07) are judged where R43-C judged the
  straight closers, including the consultation of
  quoted members in fullwidth dress. Pins: the
  RT44-C1 families at leading, mid-line, trailing,
  and base-trailing seats, glued and spaced, both
  routes, against their ASCII twins.
- **R44-C** — F5: the trailing readers take quoted
  INERT members as their bracket twins — the
  re-dress consultation extends so an inert quoted
  span counts as a sequence member at the seats
  R43-C named, scoped as that batch scoped its
  arms. Pins: the review lane's I-series (I2/I4/I5
  dresses, I6 weekday, I8 three-group) with I9 /
  I10 / I15 / I16 and the I11/I12 base-trailing
  parity held as controls.
- **R44-D** — F3 + F6 + F7 (mark-set completion):
  the remaining fullwidth suffix siblings
  (U+FF5E, U+FF03, and U+FF0E/U+FF0C/U+FF1A/U+FF1B
  at the suffix arms R44-B does not cover) join
  the peel/suffix sets beside their twins; `?`
  joins `LineScan::split_last`'s end trims beside
  `!`, and U+FF1F's standing at that seat is judged
  by the same seat logic and stated in the record
  either way; U+02DC joins beside `~` and leaves
  the exotic list, whose control pin is restated
  to the seven standing marks, flagged. Pins: the
  RT44-C2 fourteen-arm families, the RT44-C4
  alone-seat family (incl. stacks and the bracket
  dress), the RT44-C5 arms; R42-F's remaining-marks
  control restated exactly where it names U+FF5E /
  U+FF03 as never-judged, flagged.
- **R44-E** — F4: fullwidth dress pairs
  (U+FF08/U+FF09, U+FF3B/U+FF3D, U+FF5B/U+FF5D)
  are read as groups at the company seats (the
  consultation/re-dress shape of R43-C, extended
  to the fullwidth pairs), scoped to the witnessed
  seats. Pins: the RT44-C3 families (base-trailing
  company, leading company with its record, the
  quoted-member seat, trailing-alone memory) with
  the held lone-governor arms as controls.
- **R44-F** — F8: the garnish/peel consultations
  treat U+200B and U+FEFF as nothing at the
  witnessed seats (a zero-width artifact standing
  between a group and its judged suffix, between
  a governor group and its company member, or
  after a suffix), scoped to the garnish seats —
  the inside-date reading (unreadable, not
  misread) is untouched. Pins: the RT44-C6
  witnesses with the held space-width family as
  controls.
- **R44-G** — F9: the zip64 arm is subordinated —
  a zip64 EOCD whose CD offset points at a gap
  plant directory is judged as the walked-extent
  and trailing arms judge their plants (the first
  complete directory the file's own frame
  supports), scoped to the zip64 addressing arm.
  Pins: the RT44-A1 crafts × all four doors × all
  four entry sets against their clean controls,
  with zip64-real and bad-locator as standing
  controls and every standing archive pin
  unweakened. If the subordination breaks a
  standing pin or a legitimate zip64 shape in the
  corpus, the writer stops and records rather
  than forcing it (the R43-D rule).
- **R44-H** — F10: per adjudication 5. Pins: the
  quoted shape at n=25,600 (and the size-climb
  stated), judged by `r33d_interleaved_best`
  against the longline control, bound calibrated
  from quiet-window readings on the fixed tree
  and disclosed in the pin's comment in the
  R43-E shape; the brackets sibling witnessed and
  stated.
- **R44-I** — HF-12 first slice, per adjudication
  6. One crate file (`crates/ob-extract/src/
  lib.rs`), two kinds, no caller behavior change,
  no user-visible message change. Pins: the
  door-surface tests restated to the typed kinds
  where they assert on the door functions, with
  the rendered words asserted byte-identical;
  every other standing pin unweakened.

Then the coordinator close: five-leg evidence on
the final tree (local cold, fuzz as claims against
the CURRENT baselines — verdict `79775844…` /
composition `e8b2e73b…`, rights 5083 / 1024 / 5275
safety_flips=0 — hosted CI ×3, desktop runner
build, Windows bundle), the build-cache policy
disposition at the confirmed completion (df
reading + window + latch state over the round's
recorded readings), and boundary hygiene. The exit
bar is judged on round 45's audit of the closed
tree; the Tier-2 final gate is not triggered
(0/3/7) and is not the coordinator's to start.

## Round-44 close (coordinator, 2026-10-27)

Final code tree: `34690e3` (R44-I). Round-44 audit:
0 BLOCKER / 3 MAJOR / 7 MINOR; all ten findings fixed or
dispositioned in nine batches, plus the HF-12 first slice
under adjudication 6. The exit bar is judged on round
45's audit of this tree.

### Batches

| Batch | Tip | Finding | Suite |
|---|---|---|---|
| R44-A | `f1323fb` | F1 (en-dash peels) | 1239 |
| R44-B | `62fcc6a` | F2 (fullwidth separators/closers) | 1245 |
| R44-C | `72fd6d0` | F5 (quoted inert members, anchor gate) | 1248 |
| R44-D | `b50b3cb` | F3+F6+F7 (suffix siblings, `?` split_last, U+02DC) | 1254 |
| R44-E | `a4dc5e9` | F4 (fullwidth dress pairs) | 1260 |
| R44-F | `b25dd83` | F8 (zero-width garnish) | 1266 |
| R44-G | `58e4f8a` | F9 (zip64 gap-plant subordination) | 1271 |
| R44-H | `40b80ce` | F10 (quoted-shape cliff: bounded fix + pin) | 1272 |
| R44-I | `34690e3` | HF-12 slice 1 (typed open-door error) | 1275 |

Three batches (A, D, E) landed only after a writer's
STOP-and-record on a standing-pin restatement the brief
had not authorized; each was ruled in writing
(`r44a-ruling.md`, `r44d-ruling.md`, `r44e-ruling.md` in
`round44-out/coord/`) and the restatement extended to
exactly the named witness arms. R44-I likewise stopped
on a compile-witnessed gap in its readiness map and
landed under `r44i-ruling.md`. Three writers died at or
after their work was complete; every batch was judged
from its record on disk and the gh API, never from an
agent report alone.

### Close evidence (five legs, final tree `34690e3`)

- Local cold (coordinator close-verify, `cargo clean`
  then the suite from the clean tree; logs in
  `round44-out/coord/close/`): **1275 passed / 0
  failed**, SUITE_EXIT=0; clippy `-D warnings`
  CLIPPY_EXIT=0; fmt `--check` FMT_EXIT=0.
- Fuzz as claims (bins rebuilt from this tree's code,
  sources at their recorded md5s): verdict
  `797758443d101f9339fdd778e14af430` (40,000 lines,
  0 panics), composition `e8b2e73b117c7013d867419da8fb0ab7`
  (497,024 lines, 0 panics), rights **5083 / 1024 /
  5275, safety_flips=0** — all byte-identical to the
  standing baselines; no re-baseline this round (the
  rights safety axes hold for the twenty-fifth
  consecutive round).
- Hosted CI on `34690e3`: run 40631788809 — windows ✓,
  ubuntu ✓, macos ✓ per-job conclusions.
- Desktop Runner Build: run 40631788595 ✓ success.
- Windows bundle: run 40636718032 ✓ success
  (dispatched at the close on the final tree).

### Hosted flake pattern this round (stated plainly)

Six hosted flake events, two classes, every one re-run
to green on an unchanged commit; no bound widened, no
pin weakened, no production change made for any of
them. Per-event detail: `round44-out/coord/ci-flake-
accounting.md`.

- Class 1 — the R42-E four-claimant OUTCOME flake
  (3 events): CI 40573033666 att1 (macOS, R44-E tip),
  CI 40590289083 att1 (Ubuntu, R44-F tip), CI
  40623211515 att1 (macOS, R44-H tip) — the
  composed-race outcome assertion, all four claimants
  standing down in round 1 on a loaded hosted runner.
  It also flaked three times in round 43. **The
  recurrence rate now exceeds the round-43
  disposition's "single flake" posture**; round 45
  inherits it as a named target (below).
- Class 2 — the ETXTBSY fake-helper SPAWN flake
  (3 events): runner 40493362276 att1 (R44-A tip),
  runner 40605594721 att1+att2 (R44-G tip) — two lock
  tests failing at the "spawn live peer" step, code 26
  ExecutableFileBusy; the standing disclosed class.

### HF-12 first slice (decided 2026-10-25)

Landed in R44-I: `OpenDoorError { TooLarge,
Unreadable(io::Error) }` at the open-door surface of
ob-extract, `Display` byte-for-byte the standing plain
words, a `From` impl at the boundary, and — under
`r44i-ruling.md` after the writer's compile-witnessed
STOP — four one-line caller conversions in `windows.rs`
(`e.to_string()` ×2, `.map_err(String::from)` ×2).
`from_json`'s version refusal is expressly deferred to
the second slice. **Correction to the readiness map,
for future slices:** "no caller branches on error text"
does not imply "callers compile unchanged" — call sites
must be classified by conversion position (`?` /
argument / chain / render), not by branching alone.

### Cache disposition at the confirmed completion

Inputs, all on the record: the round's peak reading was
**88% used in the overnight window** at R44-A's start
(latch armed under the standing rule, recorded in
`r44a-ruling.md`); on 2026-10-27 the Jacob-approved
sweep under the widened disk-clear ritual deleted the
round 28–42 outputs and returned the disk to **69%
used / 32G free**; per-batch readings after the sweep
ran 68–69%; the completion reading at this close
(22:46 PDT, daytime window, 94% line) is **69% used /
32G free**. The close-verify itself executed
`cargo clean` (8.7 GiB removed) and rebuilt the target
cold from the final tree — the clear-and-cold-start the
armed latch called for was performed as the close's
own cold leg. **Disposition: the fresh cold cache
carries into round 45.** The registry cache was not
touched.

### Disk-clear ritual terms (recorded for round 45)

Per Jacob's 2026-10-27 approval (scope:
`~/workspace/research/disk-clear-scope-2026-10-27/
SCOPE.md`): keep the current round's out dir plus the
previous round's; older round outs are sweep-eligible
at round boundaries, executed by this lane;
`openbulletin/target` is clearable at round boundaries
when the latch has fired; `probe29v` is ACTIVE and
excluded from the first sweep; **no deletion happens
on the ritual's say-so alone — every sweep presents
Jacob its exact list (paths, sizes, df reading) and
runs only on his approval.** The first sweep executed
2026-10-27 under his approved list (rounds 28–42
deleted; sweep record in the scope dir). Boundary
hygiene at this close: `round44-out` and `round43-out`
kept; `probe29v` untouched (bin sources re-verified at
their recorded md5s during the close fuzz leg); lane
reports, syntheses, and close-status notes kept.

### Carried into round 45

1. **Named target: the R42-E four-claimant race pin's
   scheduling sensitivity on hosted runners.** Fix
   direction is the test's own composition (its round
   window / stand-down tolerance), judged as a
   test-harness finding — not a bound-widen and not a
   production change unless new evidence points into
   production code.
2. F7's restated rename-step window rate stands as a
   record (5/400 sleep-lander, 153/2000 spin-lander);
   round-44 measurements at or under it; no clean shape
   known; disposition unchanged.
3. The L10/L11 lock-path step-level sibling stands as
   an observation; the only production caller (the
   audit) refuses every witnessed move.
4. The standing records stand: peeled-tail residual,
   split-position governor uniformity, lock
   leading-newline pid spelling, recycled-pid residual,
   the R40-C calibration note.
5. The exotic-sibling closed-peel limit stands at
   seven marks (U+2012 FIGURE DASH the weakest).
6. R44-C's I15 standing split: a quoted inert member
   at the sequence seat keeps its no-record reading,
   pinned by the batch record; not claimed by the
   anchor gate.
7. The `nested_seq_mix_brackets` sibling is
   witnessed-standing (at/below the paren shape at
   n=25,600), not pinned; the quoted shape's residual
   post-fix cost stands as pinned at 37× (R44-H) with
   its architectural cause recorded.
8. Adjudication-8 observations: emoji garnish seats,
   grouped `(off)~` governor-vocabulary asymmetry,
   non-fullwidth exotic dress pairs split at the same
   seats as F4.
9. HF-12 second slice: `from_json`'s typed version
   refusal (its wording is pinned from a test helper
   outside the file); then IDML's dump door
   next-tightest per the readiness map — slice maps
   classified by conversion position per the correction
   above.

Majors trend: … 5 (R42) → 3 (R43) → 3 (R44). Round
45's audit of `34690e3` judges the exit bar.
