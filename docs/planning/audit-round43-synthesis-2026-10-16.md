# Audit round 43 — synthesis (coordinator, 2026-10-26)

Audited tree: `86f3caa` (round-42 close record on code
tree `ae4064e`). Three blind lanes, read-only: verify
(`round43-verify.md`), review (`round43-review.md`),
red team (`round43-redteam.md`), all in
`~/workspace/openbulletin-audit-wave3/`, scratch in
`round43-out/`.

Lane verdicts: verify PASS on all six claims (0/0/0
handed on — suite 1208/0 in a single run, `SUITE_EXIT=0`,
no ratio pin failed and no solo disposition was needed;
clippy exit 0, cache-served and disclosed as in round
42; fmt exit 0; fuzz byte-identical to the CURRENT
baselines, verdict `79775844…` / composition
`e8b2e73b…`, totals 40,000 / 497,024, panics 0; rights
5083 / 1024 / 5275 with safety_flips=0, M8 held for the
twenty-fourth consecutive round; booklet 31,717 /
34,977 byte-exact; hosted CI re-derived per job from
the gh API, every cited run and job exact); review
0/0/3 plus all four carried standing records re-graded
STAND and the R40-C calibration note re-graded stands
honest; red team 0/3/3 plus observations, rights held
whole under its own quadruple and quintuple sweeps,
and R42-E's F1 fix verified closed through the seam.
Deduped: **0 BLOCKER / 3 MAJOR / 4 MINOR** — the exit
bar is NOT met. Majors trend:
10 → 6 → 6 → 1 → 4 → 3 → 2 → 5 → 6 → 8 → 6 → 5 → 8 → 8 → 5 → 5 → 3.
The Tier-2 final gate is not triggered and is not the
coordinator's to start.

Every finding below was witnessed firsthand by at
least one lane on this tree; three findings were
witnessed independently by two blind lanes.
Cross-lane convergences and the coordinator's grade
adjudications are stated per finding. All seven
findings are dispositioned: batches R43-A..R43-E plus
one record restatement carried in the close section.

## Adjudications recorded before the findings

1. **The en dash at the R42-C/R42-D seats (red team
   RT43-E1, review F43-1) is one finding, graded
   MAJOR.** The two lanes witnessed the same splits
   seat for seat (review's ten-row table; the red
   team's six families, indices 124–164, en/hy/em
   triples on identical frames, both routes). Review
   grades MINOR on the adjudicated scope as filed
   (round-42 adjudication 7 named RT42-S4's witness
   table; R42-F met the table exactly) and states the
   re-grade argument in full. The coordinator adopts
   the re-grade: the seats split at are arms where
   `-` and `—` are judged on the closed tree — built
   by R42-C and R42-D earlier in the same round — so
   R42-F's own headline claim, "the en dash is judged
   at the arms where `-` and `—` are judged", is not
   true of the tree the round closed on; three arms
   serve a cancelled date live (the lone mid-line
   governor, the trailing interior governor, the
   trailing-governor doubles), which is F6's and
   F7's headline harm verbatim. The seat map is part
   of the judged set's definition — the R42-C/R42-D
   seat-building batches treated it so. Review's
   MINOR and its scope ground stand recorded here and
   in its report. Grade MAJOR (F1).
2. **F9's re-grade trigger is met, and the finding
   it converts grades MAJOR.** Round-42 adjudication
   7 stated the trigger precisely: a witnessed
   typist-produced sibling of a judged mark, outside
   the set, splitting in the unsafe direction. Two
   blind lanes witnessed it independently (red team
   RT43-M1 at all five S4 arms, both routes; review
   F43-2 at all three trailing arms, both routes).
   The marks are U+2010 HYPHEN and U+2011
   NON-BREAKING HYPHEN (the hyphen codepoints word
   processors and smart-punctuation input actually
   emit — the same autocorrect family that produces
   the en dash itself) and U+2212 MINUS SIGN (LaTeX
   renderers, calculators, AI-chat output). The split
   is total: every arm, both unsafe directions (a
   governed date serving live; a weekday mismatch
   losing its record). Red team grades MAJOR on the
   adjudication's own terms — the trigger exists to
   re-grade exactly this shape above F9's family
   MINOR. Review grades MINOR, as F9 was graded; its
   grade stands recorded. The coordinator adopts
   MAJOR (F2): a trigger the coordinator stated in
   advance, met in full by two blind lanes, does not
   grade at the level of the finding it re-grades.
3. **The fullwidth siblings are F2's sibling
   finding, graded MINOR (F3).** U+FF0D, U+FF0A,
   U+FF01, U+FF1F (red team RT43-M2; review witnessed
   U+FF0D inside F43-2) split identically at all five
   arms. The typist path is real but narrower (a CJK
   IME in fullwidth mode), hence MINOR as its own
   finding rather than folded into F2 — the red
   team's split, adopted. The cure shape is shared
   with F2, and the two land in one batch (R43-B).
4. **The straight-closer family (RT43-B1) and the
   quoted-member family (review F43-3) are two
   findings, one batch.** Carried item (b) named one
   witness-only line at the base-trailing inter-group
   seat. The red team widened it to six seats in two
   walks the company walk never reaches (the
   base-trailing garnish walk's gap set and the
   trailing group-sequence walk), including a
   trailing-slot arm that serves a cancelled date
   live — graded MAJOR (F4). Review's F43-3 is a
   distinct mechanism at the same seats: a whole
   group wearing quotes instead of parens is never
   re-dressed by the trailing readers, so the company
   collapses where its paren twin composes — graded
   MINOR (F5). The red team's quoted-member arm at
   base-trailing (`(cancelled)"Monday"`) is a witness
   of F5's mechanism standing in F4's log; it is
   counted in F5, not twice. Both land in R43-C.
5. **F2's lock residual: the rate is restated, the
   disposition stands.** RT43-L1 reproduced the
   rename-step window at 5/400 in the original
   sleep-lander shape (round 42 witnessed 1/400 in
   each of two runs) and measured 153/2000 with a
   spin lander sweeping 0–500 µs: the window spans
   tens of microseconds from the judge's read to the
   remove/rename pair, so the sleep-lander rate
   under-samples a window this wide. The harm class
   is unchanged (a live generation at the *displaced*
   path is destroyed; the lock-path record survives;
   no composed claim lost a lock-path generation).
   F2 carries as a record with the restated rate;
   no production change is attempted this round —
   R42-E's full attempt stands, and no clean shape
   is known. The restatement lands in this round's
   close section (F7). The step-level lock-path
   sibling the red team probed (L10: 32/400 step-level
   moves, 32/32 audit refusals; L11 composed: 0/400
   losses) is an observation, not a finding — the
   only production caller closes it — and is named
   for round 44's red team as a standing observation.
6. **`nested_seq_mix` earns its ratio pin, on the
   verify lane's measured terms.** Carried item (c)
   is answered: YES. The pin lands at the fixed shape
   n=25,600 (doclen 115,262) against the `longline`
   control at 102,800 through the suite's own
   `r33d_interleaved_best`, bound **10×** (worst
   quiet-window reading at that size across two
   independent windows is 7.09), with the R40-C
   disclosure conditions: the pin's comment states
   the measured size-climb (medians 3.89 / 4.52 /
   6.31 / 8.64 / 11.94 at 1,600 / 6,400 / 25,600 /
   51,200 / 102,400, round-43 verify, debug frame) so
   the bound is never read family-wide, and the
   standing solo-disposition load discipline applies
   unchanged. The larger sizes stay the probe's job.
   Disposition: R43-E.
7. **U+00AD SOFT HYPHEN is adjudicated inside
   R43-B, from witnesses.** The red team records it
   partial: the governor and grouped arms split
   unsafe; the bare arm keeps a record naming the
   word as written (`Monday\u{ad}` — the slash
   precedent's shape). Word processors insert soft
   hyphens on the ordinary copy path, so the batch
   witnesses the full family and either admits the
   mark with the sibling class or records, arm by
   arm, why the partial shape stands. The exotic
   siblings (U+2012, U+02DC, U+2053, U+2025, U+22EF,
   U+FE61, U+2731, U+266F — witnessed splitting
   identically, no ordinary typist path) are NOT
   admitted: they are the closed-peel limit standing
   as recorded, and R43-B pins them as controls so
   the limit is witnessed, not anecdotal. U+2044 is
   not a sibling of a judged mark (its ASCII twin is
   itself unjudged) and stands outside this question.
   ASCII `...` is held judged, exactly as `…` is
   (red team control).
8. **HF-12 is not a batch of this round.** The
   review lane's readiness map is confirmed against
   this tree (section 5 of its report): the ground
   did not move in round 42, the open doors stand at
   the cited lines still `Result<_, String>`, and
   one uncited pre-existing surface is noted
   (`parse_contribution`'s own `Result<_, String>`,
   contribution/mod.rs:351 — the bulletin parser's
   door, not an import door; whether it belongs to a
   later slice is the proposal's question). The
   coordinator's proposal for the first slice goes
   to the coordinator seat with this synthesis; the
   migration starts deliberately, between rounds, on
   the seat's confirmation, not mid-round. Cited as
   decided (Jacob, 2026-10-25), never as held.

## Findings

### F1 (MAJOR) — The en dash is judged only at R42-F's named arms; at the seats R42-C and R42-D built for `-` and `—` in the same round, U+2013 still splits in the unsafe direction

Red team RT43-E1 + review F43-1, merged per
adjudication 1. R42-F added U+2013 at exactly
RT42-S4's witnessed arms (review verified by
enumerating every U+2013 site in the two contribution
files on the tip). S4's table was witnessed on the
round-41 tree, before the mid-line, inter-group,
interior-sequence, and split seats existed for `-`
and `—`. At every seat below the en dash is not shed
and splits against both twins (full records, both
routes; red team `contrib43.log` indices 124–164,
review probe sets A/G):

- **Lone mid-line governor**, glued and spaced:
  `notes (cancelled)– November 8, 2026` serves
  2026-11-08 live; both twins govern.
- **Mid-line company**: suffix on the governor member,
  on the weekday member, and the spaced inter-group
  form all serve 2026-11-01 silently; both twins
  compose governed WITH WeekdayMismatch.
- **Base-trailing inter-group**: `Sunday after
  November 1, 2026 (cancelled)–(Monday)`, glued and
  spaced, serves the resolution silently; both twins
  refuse with the record.
- **Trailing interior sequence**: `(Monday)– (notes)`
  serves silently, record lost; `(cancelled)– (notes)`
  serves the cancelled date live; both twins judged
  (R42-D's interior seats).
- **Trailing-governor doubles**: `(cancelled)––` and
  the spaced `(cancelled) – –` serve 2026-11-08 live;
  the single en dash at this arm is R42-F's own judged
  cell and governs, and both twins' doubles govern.
- **Trailing-alone, Keep route only**: `next Sunday
  (November 8, 2026)–` (and the quoted dress) carries
  the For-route memory as its twins do but keeps
  nothing, where both twins keep 2026-11-08 — R42-D's
  F10 Keep arm, un-widened to the mark.
- **Leading company** (R42-F's own deviation 4(a),
  flagged for this round): `(cancelled)(Monday)–
  November 1, 2026` governs but the record is lost;
  the twins govern WITH the record.

Mechanism, code-read (review): U+2013 stands in none
of `midline_governor_before_plain_date`'s sets
(composition.rs:2396), the company walk's `SEPARATORS`
(composition.rs:3405), the trailing readers' R42-D
gap sets, or `LineScan::split_last`'s end trims
(dates.rs ~1250). Held within the surface: every arm
R42-F named stands as the batch recorded, including
the head inter-group seat. Fix shape (from the lanes,
adopted): add U+2013 to the four sets named above and
to the trailing-alone Keep arm and the leading-company
record path — the mark joins the sets, so seats built
later inherit it; no seat-by-seat patch list.
Disposition: R43-A.

### F2 (MAJOR) — F9's re-grade trigger, met: typist-produced hyphen siblings U+2010, U+2011, and U+2212, outside every judged set, disarm all five S4 arms in the unsafe direction

Red team RT43-M1 + review F43-2, merged per
adjudication 2. None of the three marks is in S4's
35-mark table or in R42-F's control pin (review
verified by extracting the pin's mark escapes). For
each mark M, at the five S4 arms, both routes, against
the `-`/`—`/`–` twins' judged readings (red team
`contrib43.log` indices 34–58; review probe set D):

- `November 8, 2026 (cancelled)M` serves 2026-11-08
  live (twins govern; the Keep route keeps it live).
- `November 1, 2026 (Monday)M` serves 2026-11-01
  silently (twins refuse with the record).
- `November 1, 2026 MondayM` reads None with no
  record on either route (twins refuse with
  WeekdayMismatch(Monday@2026-11-01)).
- `(Monday)M November 1, 2026` serves silently
  (twins refuse with the record).
- `Sunday after November 1, 2026 (Monday)M` serves
  the resolution silently (twins refuse with the
  record).

Typist provenance (the trigger's qualifier): U+2010
is the hyphen word processors and PDF text layers
emit for an ordinary hyphen — bulletin copy pasted
from Word and PDFs arrives with it; U+2011 is the
non-breaking hyphen word processors substitute in
hyphenated compounds; U+2212 is what LaTeX renderers,
calculators, and AI-chat output emit for a minus.
A contributor does not choose these codepoints; they
arrive by the ordinary copy-paste path, attached to
exactly the garnish words these readers judge. This
is the en dash's own story one family over.
Disposition: R43-B (with F3), where the marks join
the judged sets at the set level — every seat their
twins are judged at, not a five-arm patch — so the
F1 failure mode (a mark admitted at a table, splitting
at seats built later) does not recur for this class.

### F3 (MINOR) — The fullwidth (IME) siblings U+FF0D, U+FF0A, U+FF01, U+FF1F split identically at all five arms

Red team RT43-M2 (review witnessed U+FF0D's rows
inside F43-2). The fullwidth forms of the judged
`-`, `*`, `!`, `?` split at the same five arms in
the same directions (`contrib43.log` indices 54–58,
64–68, 104–113; the trailing-governor Keep route
verified in the raw log: the cancelled date kept
live). These are what a CJK IME produces in fullwidth
mode for the ordinary ASCII keys — a real typist
path, narrower than F2's copy-paste path. Graded
MINOR per adjudication 3. Disposition: R43-B (with
F2), same set-level admission.

### F4 (MAJOR) — The straight quote closers disarm governor/weekday companies at the inter-group seats the company walk never reaches: the base-trailing seat (carried item (b)) plus five sibling seats

Red team RT43-B1. The carried witness stands, and it
is not a lone line. At every seat below, the straight
closers `"` and `'` (glued and spaced) disarm the
company on both routes, while the curly closers
`”`/`’` at the same seats are judged and the
line-final straight closer at the base-trailing slot
is judged (red team `contrib43.log` indices 0–33,
full records both routes):

- **Base-trailing (the named seat):** `Sunday after
  November 1, 2026 (cancelled)"(Monday)` serves the
  resolution 2026-11-08 with no record; the clean
  company refuses WITH WeekdayMismatch(Monday@
  2026-11-01). The single-quote and spaced forms
  split identically.
- **Trailing slot (plain date line):** `November 1,
  2026 (cancelled)"(Monday)` serves 2026-11-01
  silently, where the clean trailing company governs.
  A cancelled date serves live on an ordinary line —
  F5(c)'s harm at a slot R42-B's fix did not cover.
- **Reversed order at base-trailing:**
  `(Monday)"(cancelled)` serves the resolution
  silently; the clean reversed company refuses with
  the record.
- **Three-group companies at base-trailing:** the
  closer after the governor, after an inert group,
  and after the weekday each serve the resolution
  silently; the clean three-group company refuses
  with the record.
- **Weekday-only seat at base-trailing:**
  `(notes)"(Monday)` serves silently; the clean
  `(notes)(Monday)` refuses with the record.
- **Behind an ordinary word at base-trailing:**
  `Sunday after November 1, 2026 notes
  (cancelled)"(Monday)` serves silently; its clean
  twin refuses with the record (the R42-B
  behind-a-word arm, in quoted dress).

Mechanism (from the records): two walks R42-B/R42-C
did not reach — the base-trailing garnish walk's gap
set (named in R42-B deviation (c) and R42-C's record)
and the trailing group-sequence walk. Held within the
surface: head and mid-line inter-group straight
closers compose with the record (R42-B's seats); the
line-final straight closer at base-trailing refuses
with the record; the agreeing company behind the
closer serves exactly as its clean twin (parity); a
lone governor behind the closer at base-trailing
stands at its standing parity serve. Grade MAJOR:
the named arm alone is the carried record-loss, the
trailing-slot arm serves a cancelled date live, and
the family is six seats wide in one mechanism — F5's
unsafe-direction profile at the seats that fix never
reached. Disposition: R43-C (with F5).

### F5 (MINOR) — Quoted members inside trailing and base-trailing companies still collapse the company: R42-A's quoted-member reading reaches the company walk's seats but not the trailing readers

Review F43-3 (the red team's `(cancelled)"Monday"`
base-trailing arm is the same mechanism witnessed in
F4's log run, counted here per adjudication 4).
R42-A wired the quoted-member arm at the company
walk's two call sites — the head and mid-line seats.
The trailing and base-trailing companies are read by
different readers (`trailing_group_run` /
`trailing_group_sequence`,
`base_trailing_sequence_mismatch`), which never
re-dress a quoted member:

- `November 1, 2026 (cancelled) "Monday"` serves
  2026-11-01 with no record on both routes, where
  the paren twin governs (review probe F3c vs F3b).
- `Sunday after November 1, 2026 "cancelled"
  (Monday)` serves the resolution 2026-11-08
  silently on both routes, where the paren twin
  refuses with WeekdayMismatch(Monday@2026-11-01)
  (review probe H2 pair).
- `Sunday after November 1, 2026 (cancelled)"Monday"`
  serves the resolution silently (red team, F4's
  log).

Controls at parity (same probes): the quoted governor
member at the trailing slot composes
(`November 1, 2026 "cancelled" (Monday)` governs,
== its paren twin), and the weekday-first order with
a quoted weekday member governs
(`November 1, 2026 "Monday" (cancelled)` == its paren
twin). The split is the quoted weekday member in
governor-first trailing order, and the quoted governor
member at the base-trailing slot. Grade MINOR per
the review lane: two seats of one reader family, one
dress per seat, every neighbouring dress at parity —
the F4/F10 breadth class, not F3's. Fix shape (the
batch's own, adopted): extend the re-dress
consultation (or an equivalent member reading) to the
trailing readers. Disposition: R43-C (with F4).

### F6 (MINOR) — Archive candidate discipline, sixth generation: a concatenated trailing archive is judged at the trailing directory, and the clip door fails crafts it reads clean in the control

Red team RT43-A1. Five generations planted the fake
directory in a gap before the real one; this
generation appends a second complete archive after
the first (`trailing-eocd-*` crafts: real body + real
CD + real EOCD, then plant locals + plant CD + plant
EOCD, the plant records carrying the real entries'
names and offsets with the key entry's record
describing plant content). Witnesses: `doors43.log`
blocks 2, 10, 18, 26 against the `clean-*` controls
in the same run:

- The model judges the **trailing** directory for
  three of four entry sets (idml cd=15002, the plant
  position, vs the real CD at 7300; clip cd=12688 vs
  6252; mxl cd=14170 vs 6952); the font set's
  directory stays at the real position and holds.
- At the clip door the outcome differs from the clean
  control: clean idml and clip sets read OK, while
  the trailing crafts fail with "a file in that zip
  cannot be read: its bytes could not be read back".
  Reader, IDML, MXL, and font outcomes on the same
  crafts are identical to clean.
- The judged directory's spot list is identical to
  clean (names, offsets, sizes); the divergence is
  below the spot level — the plant record's CRC for
  the key entry describes plant bytes the real local
  does not hold, and the clip door is the one door
  that reads through it and fails.

Grade MINOR: the direction is refusal, not wrong
content served — no door served plant content on any
craft in the run — but the discipline's claim (a
plant the doors cannot be steered by) is broken in
outcome at one door, by a file shape (two concatenated
zips) that occurs in the wild. Adjudicated outcome
for the fix: the trailing archive is subordinated as
the gap plants of prior generations were — the first
complete directory is the directory judged, and every
door's outcome on the trailing crafts matches the
clean control. Held in the same surface: the
comment-embedded EOCD craft (a complete fake EOCD in
the real EOCD's comment) was subordinated whole, all
doors clean-identical; local-header mutations
clean-identical everywhere; EOCD disk fields set to 1
clean-identical; EOCD entry-count falsified to 1
refused uniformly at every door (the safe direction).
Disposition: R43-D.

### F7 (MINOR, record) — The rename step's residual window (carried F2): rate restated from witnessed landings; the window spans tens of microseconds

Red team RT43-L1, per adjudication 5. The window in
`rename_instance_lock_to_displaced_from` (judge of the
displaced path → `remove_file` → rename) survives on
the closed tree: 5/400 in the original sleep-lander
shape and 153/2000 with a spin lander sweeping
0–500 µs in 1 µs steps, witnessed deletions at delays
of 14–108 µs, the owner alive at the end in every
printed witness (`lock43.log`, EXIT=0; the probe's
lockmod re-extracted from this tip). Round 42's
standing rate (1/400 in each of two runs) under-samples
a window this wide and is restated by this finding;
the close section carries the restated rate into the
standing record. Attribution is closed on this tree:
the R41-F pin shape re-driven stands the step down
with both files byte-untouched when the live
generation is already standing at the judge, and the
claim succeeds after the landed owner dies. Held in
the same surface: R42-E's F1 fix verified closed
through the L8 seam (`displaced_winner=false`,
`both_hold=false`); the L8b control refuses as
before; all three composed races held. Disposition:
record restatement in the close section; no batch,
no production change.

## Standing records and held surfaces

- **Carried item (e), all four records STAND**,
  re-witnessed by the review lane on this tree:
  (i) the peeled-tail residual stands at all three
  slots (R42-D's `split_last` widening demonstrably
  did not reach it; the cure remains a peel-depth
  change, a design decision standing since R38-F);
  (ii) split-position governor uniformity stands in
  all four dresses including quoted; (iii) the lock
  leading-newline pid spelling stands (main.rs:3467/
  3488, untouched by R42-E); (iv) the recycled-pid
  residual stands (`peer_provably_gone` =
  `PeerLiveness::Dead` alone; `peer_liveness`
  byte-untouched by R42-E, and the factored
  `instance_lock_names_live_peer` preserves the
  table's treatment of the recycled cell).
- **Carried item (f): the R40-C calibration note
  STANDS HONEST as recorded** — both pins' comments
  re-read verbatim on this tree (tests.rs:19502/
  19528), neither pin touched by the round-42 diff.
- **Rights safety axes HELD whole** under the red
  team's own sweeps as well as the verify lane's
  fuzz claims: quadruple (20,000 sets × 24
  permutations + fold route) standing_flips=0,
  children variance 0; quintuple (8,000 × 120)
  safety2_flips=0. Credit-text variance stands as
  recorded since round 38.
- **Round-40 BLOCKER family**: every R40-bounded
  perf shape held (verify claim 1 and claim 6; no
  cliff ratio pin failed in the settled suite run).
- **Observations carried to round 44 (not findings):**
  the L10/L11 lock-path step-level sibling (32/400
  step-level moves, the production audit refusing
  32/32, composed losses 0/400); the data-descriptor
  clip-door interaction (a standard streaming-zip
  shape on the idml entry set fails at the clip door
  where its clean control reads OK; the same craft
  on the clip entry set reads OK; mechanism unpinned,
  no clip content at stake in the witness as driven);
  the exotic-sibling list standing as the closed-peel
  limit (adjudication 7).

## Batch plan

- **R43-A** — F1 (en dash completion): U+2013 joins
  the four sets named in F1 (the mid-line governor's
  gap/rest sets, the company walk's `SEPARATORS`,
  the trailing readers' gap sets, `LineScan::
  split_last`'s end trims) and the two arms standing
  outside them (the trailing-alone Keep arm; the
  leading-company record path, R42-F deviation 4(a)).
  The mark joins the sets, so seats built later
  inherit it. Pins: RT43-E1's and review F43-1's
  families in full (every seat in F1's list × both
  routes against the `-` and `—` twins, the doubles
  and spaced forms included), R42-F's named arms and
  every standing suffix/peel pin unweakened.
- **R43-B** — F2 + F3 (the sibling class admitted):
  U+2010, U+2011, U+2212 and the fullwidth U+FF0D,
  U+FF0A, U+FF01, U+FF1F join the judged sets at the
  set level — every seat where their twins are
  judged, not a five-arm patch. U+00AD is witnessed
  across the full family and adjudicated inside the
  batch per adjudication 7 (admit with the class, or
  record arm by arm why the partial shape stands).
  Pins: both lanes' families (the five S4 arms ×
  both routes for each admitted mark, plus the seats
  F1's list names for the dash siblings), the exotic
  siblings pinned as controls standing as recorded,
  every standing suffix/peel pin unweakened.
- **R43-C** — F4 + F5 (quote dress at the trailing
  seats): the straight closers `"` and `'` are judged
  at the inter-group seats of the base-trailing
  garnish walk and the trailing group-sequence walk
  (glued and spaced, both routes), and the trailing
  readers take quoted members as their bracket twins
  (the re-dress consultation extended, or an
  equivalent member reading). Pins: RT43-B1's six
  seats in full (named seat, trailing slot, reversed
  order, three-group in all three closer positions,
  weekday-only, behind-a-word; single-quote and
  spaced forms; both routes) and review F43-3's two
  seats, with the parity controls standing (the
  line-final closer judged, head/mid-line seats
  judged, agreeing companies at parity, the quoted
  governor at the trailing slot composing, the
  weekday-first quoted member governing), every
  standing quoted-dress and company pin unweakened
  except any restatement the batch record flags.
- **R43-D** — F6 (archive, sixth generation): the
  concatenated trailing archive is subordinated —
  the first complete directory is the directory
  judged, and every door's outcome on the trailing
  crafts matches the clean control. Pins: RT43-A1's
  crafts × all four doors × all four entry sets,
  with the held arms as controls (comment-embedded
  EOCD, local-header mutations, disk fields,
  count-1 uniform refusal), every standing archive
  pin unweakened. If the first-directory rule breaks
  a standing pin or a legitimate appended-archive
  shape in the corpus, the writer stops and records
  rather than forcing it.
- **R43-E** — carried item (c) closed (the
  `nested_seq_mix` ratio pin): the pin lands at the
  shape, bound, and disclosure terms of adjudication
  6. Pins: the new pin, witnessed biting on the base
  tree per the standing practice (a pre-fix tree
  reading above the bound at the pinned shape, or
  the recorded reason the shape cannot bite — the
  shape is a standing measurement, not a defect, so
  the bite is stated as the calibration record:
  the base tree's quiet-window worst at 25,600 is
  7.09 against the 10× bound, and the pin's work is
  regression-keeping, disclosed in its comment).
  No other pin moves.

Then the coordinator close: five-leg evidence on the
final tree (local cold, fuzz as claims against the
CURRENT baselines — verdict `79775844…` / composition
`e8b2e73b…`, rights 5083 / 1024 / 5275 safety_flips=0 —
hosted CI ×3, desktop runner build, Windows bundle),
the F7 rate restatement carried into the standing
record, the build-cache policy disposition at the
confirmed completion (df reading + window + latch
state over the round's recorded readings), and
boundary hygiene. HF-12 is NOT a batch of this round
(adjudication 8). The exit bar is judged on round
44's audit of the closed tree; the Tier-2 final gate
is not triggered (0/3/4) and is not the coordinator's
to start in any case.

## Coordinator close (successor coordinator, 2026-10-27)

**Round 43 is closed.** All seven findings were
fixed or dispositioned in five batches R43-A..R43-E
plus the F7 record restatement carried below. The
fuzz hard term held byte-identical at the CURRENT
standing baselines at every batch — no re-baseline
was needed this round. Final code tree **`f16d13e`**
(R43-E, a test-only batch); suite **1236 passed /
0 failed**.

| Batch | Tip | Disposition | Hosted CI |
|---|---|---|---|
| R43-A | `390f593` | F1 (en dash completion at the R42-C/R42-D seats) | 40402036495 ×3 ✓, runner 40402036449 ✓ |
| R43-B | `4746ee2` | F2 + F3 (typist sibling class admitted at the set level; U+00AD admitted per adjudication 7) | 40416945893 ×3 ✓, runner 40416946169 ✓ |
| R43-C | `8e2c046` | F4 + F5 (straight closers at the trailing seats; quoted members re-dressed) | 40434853037 ×3 ✓, runner 40434852768 ✓ |
| R43-D | `ace821f` | F6 (trailing archive subordinated to the first complete directory) | 40451491919 ×3 ✓, runner 40451491786 ✓ |
| R43-E | `f16d13e` | Carried item (c) closed (the `nested_seq_mix` ratio pin, bound 10×) | 40461904465 ×3 ✓, runner 40461904458 ✓ |

**F7 restatement, carried into the standing record
(per adjudication 5):** the rename step's residual
window in `rename_instance_lock_to_displaced_from`
(judge of the displaced path → `remove_file` →
rename) survives on the closed tree at a restated
rate: **5/400 in the original sleep-lander shape
and 153/2000 with a spin lander sweeping 0–500 µs**,
with witnessed deletions at delays of 14–108 µs
and the owner alive at the end in every printed
witness (`lock43.log`, EXIT=0). Round 42's standing
rate (1/400 in each of two runs) under-samples a
window spanning tens of microseconds from the
judge's read to the remove/rename pair and is
superseded by this restatement. The harm class is
unchanged (a live generation at the *displaced*
path is destroyed; the lock-path record survives;
no composed claim lost a lock-path generation),
the disposition stands (record only; no production
change attempted this round — R42-E's full attempt
stands, and no clean shape is known), and
attribution is closed on this tree: the R41-F pin
shape re-driven stands the step down with both
files byte-untouched when the live generation is
already standing at the judge, and the claim
succeeds after the landed owner dies.

**Five-leg close evidence on the final tree
`f16d13e`:**

1. Local cold, from the close-verify run's disk
   logs (`round43-out/coord/close/`): `cargo clean`
   exit 0, cold suite **1236/0** (SUITE_EXIT=0,
   SUITE_SUM passed=1236 failed=0), CLIPPY_EXIT=0,
   FMT_EXIT=0; booklet 31,717 / 34,977 byte-exact
   (pin inside the suite).
2. Fuzz as claims, the same close run, cold from
   the final tree's code: bin sources at the
   recorded md5s (`a9cac6c3…` / `4f85c605…`);
   verdict stdout md5 `797758443d101f9339fdd778e14af430`
   byte-identical, total_lines=40000, panics=0;
   composition stdout md5 `e8b2e73b117c7013d867419da8fb0ab7`
   byte-identical to the current baseline,
   D_TOTAL=497024, D_PANICS=0; rights counters
   5083 / 1024 / 5275 with safety_flips=0 on every
   route — M8 held for the twenty-fourth
   consecutive round. All four bins EXIT=0, stderr
   empty.
3. Hosted CI **40461904465** ✓ — ubuntu / windows /
   macos all success (jobs checked individually).
4. Desktop Runner Build **40461904458** ✓.
5. Windows bundle **40470669392** ✓.

**Hosted CI flake of the round (of record):** the
R42-E four-claimant race pin
(`r42e_four_claimant_races_over_a_dead_lock_stand_single_winner`)
flaked on hosted CI at three batch tips — R43-A
(ubuntu, attempt 1), R43-C (macOS, attempts 1–3),
R43-D (ubuntu, attempt 1) — each time re-run to
green, and every batch tip stands green on all
three OS legs in its record. No bound was widened
and no pin was weakened; the pattern is recorded
in the batch records for the lanes' attention.

**Completion incident (of record):** the round's
first coordinator died mid-close. Its first
close-verify attempt was interrupted by a VM
replacement after `cargo clean` (8.7 GiB) and a
partial suite; its logs stand renamed in the close
dir (`close-attempt1-vmreplaced.log`,
`suite-attempt1-partial.log`) and no verdict was
read from them. The second attempt
(`close-verify2.nohup`, launched 2026-10-27
08:31 UTC) was still alive when the successor
coordinator took over — established by process
scan, mtime freshness, and the script's own logs
— and was left to finish; all five legs above
were judged from its disk outputs. No duplicate
verification was started against the same
directories, and nothing was killed on suspicion.

**Build-cache disposition:** confirmed completion
Tue 2026-10-27 ~02:20 PDT; final reading 86% used
/ 14G free (overnight window, 87% line); the
round's recorded readings ran 81–86%, with no
reading at or above the overnight 87% line and no
latch armed. **CARRY.**

**Boundary hygiene:** the lanes' round-43 working
dirs stand in `round43-out/` with their records,
including both close-verify attempts' logs;
`probe29v` is retained as the standing fuzz home
at its recorded bin md5s; reports, syntheses,
status notes, and fuzz records are kept as filed.
No registry prune.

**HF-12:** decided (Jacob, 2026-10-25) — typed
core, plain words at the boundary, gradual from
the import layer. Not a batch of this round
(adjudication 8); the first-slice proposal went
to the coordinator seat with the synthesis and
the migration starts deliberately, between
rounds, on the seat's confirmation. Cited as
decided, never as held.

**Exit bar NOT met** — the round-43 audit was
0 BLOCKER / 3 MAJOR / 4 MINOR. Majors trend:
… → 5 → 5 → 3. The bar is judged on round 44's
audit of this closed tree. The Tier-2 Opus + GPT
final gate is not triggered and is not the
coordinator's to start.

**Carried into round 44 (named targets):**
(a) F7's restated rate standing as a record (the
rename-step window at 5/400 sleep-lander,
153/2000 spin-lander; no clean shape known);
(b) the L10/L11 lock-path step-level sibling as a
standing observation (32/400 step-level moves, the
production audit refusing 32/32, composed losses
0/400 — the only production caller closes it);
(c) the data-descriptor clip-door interaction
(a standard streaming-zip shape on the idml entry
set fails at the clip door where its clean
control reads OK; mechanism unpinned, no clip
content at stake in the witness as driven);
(d) the exotic-sibling list standing as the
closed-peel limit, pinned as controls in R43-B
(U+2012, U+02DC, U+2053, U+2025, U+22EF, U+FE61,
U+2731, U+266F — no ordinary typist path);
(e) the standing records held whole: peeled-tail
residual (cure = a peel-depth change),
split-position governor uniformity, lock
leading-newline pid spelling, recycled-pid
residual; (f) the R40-C calibration note, standing
honest as recorded (re-graded in this round's
audit); (g) the HF-12 first slice, queued for the
coordinator seat's between-rounds confirmation.
