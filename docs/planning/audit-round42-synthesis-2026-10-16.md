# Audit round 42 — synthesis (coordinator, 2026-10-26)

Audited tree: `48c7c76` (round-41 close record on code
tree `6dce513`). Three blind lanes, read-only: verify
(`round42-verify.md`), review (`round42-review.md`),
red team (`round42-redteam.md`), all in
`~/workspace/openbulletin-audit-wave3/`, scratch in
`round42-out/`.

Lane verdicts: verify PASS on all six claims (0/0/0
handed on — suite 1179/0 in the settled run, fuzz
byte-identical to the standing baselines, rights
5083 / 1024 / 5275 with safety_flips=0, M8 held for
the twenty-second consecutive round, booklet 31,717 /
34,977 byte-exact, hosted CI green on all three OS
legs on the close tree and every round-41 batch-table
citation exact per-job); review 0/1/0 plus carried
records held and the HF-12 advisory; red team 0/5/5.
Deduped: **0 BLOCKER / 5 MAJOR / 5 MINOR** — the exit
bar is NOT met. Majors trend:
10 → 6 → 6 → 1 → 4 → 3 → 2 → 5 → 6 → 8 → 6 → 5 → 8 → 8 → 5 → 5,
and round 40's BLOCKER family (the bounded perf
shapes) does not recur: every R40-bounded family held
at 2–4× round 41's driven sizes. The Tier-2 final
gate is not triggered and is not the coordinator's
to start.

Every finding below was witnessed firsthand by at
least one lane on this tree; cross-lane convergences
and the coordinator's grade adjudications are stated
per finding. All ten findings are dispositioned to
batches R42-A..R42-F.

## Adjudications recorded before the findings

1. **Review's F-R42-1 is RT42-Q1's arm (a).** Both
   lanes witnessed the framing stack of three or
   more words adopting the quoted date where every
   bracket twin serves the base with the record
   (review: `the choir hall`, four words adopts too;
   red team: `the choir rehearsal`, `on the the`,
   `the on the`, and a numbered stack serving
   2026-11-03), and the framing-before-split record
   split one word wider than R41-C consulted. One
   finding (F3), graded MAJOR as F4 was graded: the
   headline arm adopts a wrong date in silence.
2. **The governor-LAST family (RT42-S3) is graded a
   MINOR finding (F8), adopting the red team's grade
   over the review lane's held-record argument.**
   Review's three legs are recorded in its report
   (section (d)) in full: the ellipsis — judged at
   this slot since R40-G for every judged arm — also
   fails to reach this reader, so R41-E did not leave
   it half-widened; the peeled-tail precedent holds
   the same shape as a record; the batch itself left
   it unpinned as a scope observation. The
   coordinator rules otherwise on the cure: the
   peeled-tail residual's cure is a global peel-depth
   change, a design decision standing since R38-F,
   while this family's cure is the same widening
   R41-E already applied to every sibling arm at the
   same slot, and the family is now bounded and fully
   mapped (red team drove it whole: at the trailing
   slot `~`, `#`, `…`, `–`, `~~`, `~*` serve and
   `*`, `-`, `—` govern; the clean and spaced forms
   govern; base-trailing is at parity in every
   dress). A bounded, witnessed arm that serves a
   cancelled date live, with a sibling-shaped cure,
   is a finding, not an open-ended record. Grade
   MINOR, not MAJOR: one order of one reader at one
   slot, known scope since R41-E, and the clean,
   spaced, and governor-first forms all govern.
   Review's dissent stands recorded here and in its
   report. The peeled-tail residual itself is NOT
   re-graded by this ruling (its cure differs in
   kind); it stands as recorded, re-witnessed this
   round at all three slots (review section (c)).
3. **F5 arm (a) scope (lead-in positions).** R41-B
   named first-word weekday/month lead-ins as the
   wall; the red team shows the run segmentation
   makes a weekday or month word in ANY lead-in
   position fatal to the company, and that arm is
   not named. The first-word controls held as named
   (`Monday notes …`, `Sunday school …` stand), and
   `after`-fragment words (afterwards, hereafter,
   afterglow…) do not split. Adjudicated: the
   company is judged behind lead-in runs regardless
   of which non-first position a weekday/month word
   occupies in the run; the first-word wall stands
   exactly as named. The batch (R42-B) composes the
   non-first arms and pins the first-word arms
   unmoved.
4. **F5 arm (b) scope (base-trailing behind a
   word).** R41-B's controls pin named base-trailing
   companies "unmoved". Ruled: the pin's scope is
   the direct slot — the company standing
   immediately at the base-trailing position. The
   company behind an ordinary word is the same
   company one word further in, and under the R41-B
   line (a judged company is judged wherever the
   sequence stands) it takes the composed reading.
   If the writer's probe shows the pin's text claims
   the slot generally, the restatement is flagged
   in the batch record per the standing rule; no
   other reading of the pin is weakened.
5. **Record correction (R41-B).** R41-B's record
   claims "the quote closers after any group, glued
   or spaced". The composition walk's separator set
   carries the curly closers only; the straight
   closers `"` and `'` disarm the company (F5 arm
   (c)). The record overclaimed. R42-B makes the
   claim true; the correction stands here.
6. **F2's disposition (the rename step's residual
   window).** RT42-L2 witnesses the window at
   1/400 swept landings in each of two independent
   runs — a witness, at a very low rate, with the
   displaced record surviving for the audit pass.
   Per the F7 precedent (a fix lands only on a
   witness; a witness stands here), the lock batch
   (R42-E) attempts the narrowing of the step's own
   path — the R41-F pattern applied inside the
   rename step to the lock-path generation at the
   last moment before its removal. If the writer
   finds no clean shape, the batch records the
   attempt and F2 carries as a record with its rate
   stated; it is not silently dropped either way.
7. **F9's scope (marks outside every judged set).**
   The batch (R42-F) takes the en dash U+2013 only:
   the codepoint sibling of the judged set's own
   `-` and `—`, the dash a typist actually
   produces, already shed by the codebase's own
   trim sets elsewhere (`dates.rs` lines 1222/2047,
   per the lane). The remaining 24 marks stand as
   the closed-peel record — a finite judged set has
   outsiders by construction — pinned as controls in
   the batch so the record is witnessed, not
   anecdotal. Re-grade trigger, stated for round
   43's lanes: a witnessed typist-produced sibling
   of a judged mark, outside the set, splitting in
   the unsafe direction.

## Findings

### F1 (MAJOR) — The lock claim loop judges and captures in two separate reads: a live winner landing between the check and the capture is displaced as if dead, and two holders can hold at once

Red team RT42-L1. In `acquire_instance_lock_from`
(`crates/ob-app/src/main.rs`), the AlreadyExists
branch judges the current holder through
`peer_instance_alive_from` and then captures "the
exact generation just judged takeable" through a
second read, `read_instance_lock_from`. The capture's
comment claims stability because "a same-host peer
cannot be mid-takeover" — it can: the peer check
reads the pid without capturing the generation, so a
winner landing between the check and the capture is
never the generation the check judged, and the
capture feeds `displace_instance_lock_from` a LIVE
winner's generation for displacement as if dead.
Witnessed deterministically through the seam
(probe42lock `lock42b.log` L8: peer alive = true,
`both_hold = true` — the claimant returns Ok holding
the lock while the winner's own record still says it
holds it, the winner's generation at the displaced
path; the L8b control, where the capture returns the
very dead generation the check judged, refuses
correctly). Composed fallout: four claimants racing
over a dead lock ended with two winners in 3 of 30
rounds in one run, 0 of 30 in the other; the ordinary
races held (live holder 30/30 single winner; live
displaced generation never touched, 30/30). This is
not F7's window (the displaced path inside the rename
step); it is the claim loop's own two-read judgment
one level up. Fix shape (from the lane, adopted):
judge the captured generation itself — liveness on
the pid inside the captured record — or re-read
after capture and compare before displacing: the
check and the capture must be one judgment about one
generation. Disposition: R42-E (with F2).

### F2 (MINOR) — The rename step's internal judge→delete window survives R41-F at a witnessed 1/400 rate

Red team RT42-L2. `rename_instance_lock_to_
displaced_from` judges the lock path's current
generation, then removes/renames it in a later step;
a live generation landing after that internal judge
is still destroyed unjudged — witnessed 1 landing in
400 swept trials (landing delay swept 0–300 µs) in
each of two independent runs, the owner alive, the
displaced path afterwards holding the judged-dead
generation. The rate fell versus the round-41 shape
(1/400 against the winning arm's 7/64) and the
displaced record survives for the audit pass — hence
MINOR. Attribution is closed: the R41-F pin shape
re-driven on this tree stands the step down with both
files byte-untouched; the judge works, the residual
gap inside the step remains. Disposition: R42-E,
per adjudication 6. The recycled-pid residual is
unchanged in scope and stands as recorded (verify
claim 6: `peer_provably_gone` still rests on
`PeerLiveness::Dead` alone; the boundary stands).

### F3 (MAJOR) — The quoted-group re-dress consultations still miss positions where the composition judges a bracket group; at the missed positions the quoted date is adopted or the company collapses

Red team RT42-Q1 + review F-R42-1, merged per
adjudication 1. R41-C/R41-D extended the consultation
to the framing and alone positions. Three position
classes remain unconsulted (full `parse_contribution`
records, both routes; `contrib42.log` /
`contrib42b.log`, review's probe42 logs):

- **(a) Framing stacks past the consulted widths.**
  Stacks of three or more words adopt the quoted
  date (`Sunday after the choir rehearsal "November
  8, 2026" November 1, 2026` → Nov 8, base demoted,
  no record; a numbered stack `after the 3 "…"`
  serves 2026-11-03, the "3" read as a day) where
  the paren/bracket twins serve Nov 1 WITH
  ParenthesizedDateMismatch. Mechanism, code-read
  (review): `quoted_group_at_framing_position`
  (composition.rs:8822) caps the stack at
  `(1..=2).contains(&framing.len())`; the whole-line
  composition judges the bracket group at every
  stack width probed (three and four words).
  Framing-before-split stacks beyond the one named
  word split on the record only (outcomes at parity,
  quoted carries second-date notes, the twin the
  mismatch record).
- **(b) Plain-line interior quoted date groups.** On
  an ordinary date line (no Sunday-after phrase, no
  governor), a quoted date group is adopted at every
  interior position where its bracket twins are
  judged: `notes "Nov 8" Nov 1`, `"Nov 8" Nov 1`,
  `notes "Nov 8" rehearsal Nov 1` all serve Nov 8;
  trailing `Nov 1 "Nov 8"` demotes the quoted date
  to a second-date note with no record. The twins
  serve Nov 1 with the record at all four positions.
  With a governor group before it the split reverses
  but persists: `notes (cancelled) "Nov 8" Nov 1`
  serves Nov 1 with a second-date note and
  `"Nov 8" (cancelled) Nov 1` is silently forgotten,
  where the paren twins serve the group's date.
- **(c) Quoted members inside a company.** A lone
  quoted governor and a lone quoted weekday are
  judged exactly as their paren twins at every slot
  tried (held). But the company collapses when a
  member wears quotes: `notes "cancelled" (Monday)
  Nov 1`, head `"cancelled" (Monday) Nov 1`, and the
  both-quoted company all serve Nov 1 silently,
  where the all-paren company governs with the
  weekday record; `(cancelled) "Monday" Nov 1`
  serves at the head (mid-line it composes, via the
  bare-weekday path). R41-B named quoted groups
  "unmoved" as contents; the company split was not
  named.

Fix shape (from the lanes, adopted): the consultation
extends to every position where the composition
judges a bracket group — the framing statements
widened to the widths the composition itself judges,
the plain-line interior positions consulted, and
company members in quoted dress taken as their
bracket twins by the company walk — matched against
the composition's own pinned paren readings at those
positions. Disposition: R42-A (with F4).

### F4 (MINOR) — Doubled quote forms at the alone positions lose the R41-D memory entirely

Red team RT42-Q2. `""November 8, 2026"" next Sunday`
and `next Sunday ""November 8, 2026""` read None with
no parenthesized-date memory and no keep-through,
where the single-quoted forms take the bracket twins'
reading (memory carried, keep=2026-11-08) — F5's harm
in doubled dress. Mixed quote pairs at the same
positions are remembered (held); doubled quotes at
framing and interior positions split nothing (held).
Disposition: R42-A (with F3).

### F5 (MAJOR) — Governor/weekday company edges after R41-B's EXTEND: non-first lead-in words, the base-trailing company behind a word, and the straight quote closers

Red team RT42-C1. Three arms, one finding (the
company's judged reach after the EXTEND):

- **(a) A weekday or month word in a non-first
  lead-in position collapses the company to a silent
  serve.** `notes Monday (cancelled)(Monday) Nov 1`,
  `notes November (cancelled)(Monday) Nov 1`,
  `notes on Monday (cancelled)(Wednesday) Nov 1` all
  serve Nov 1 with no record, where ordinary-word
  twins compose governed with the record. Scope per
  adjudication 3.
- **(b) The base-trailing company behind an ordinary
  word serves the resolution silently.** `Sunday
  after November 1, 2026 notes (cancelled)(Monday)`
  (and the weekday-first and inert-word variants)
  serves Nov 8 with no record, where the direct
  base-trailing company refuses with the weekday
  record; the same company at the trailing slot
  behind a word composes (held contrast). Scope per
  adjudication 4.
- **(c) Straight quote closers after a group disarm
  the company.** `(cancelled)(Monday)" Nov 1` and
  `(cancelled)(Monday)' Nov 1` serve silently at the
  head; mid-line the same closers serve as well;
  spaced forms split too (head: governed but the
  weekday record lost; mid-line: serves). The curly
  closers are judged (no split). The walk's
  separator set carries the curly closers only —
  against R41-B's own record claim, corrected at
  adjudication 5.

Held within the surface: two-weekday companies in
both orders; the governor-completion boundary in
every dress at head and mid-line; the named
`(today cancelled)(Monday)` boundary serves as
recorded. Disposition: R42-B.

### F6 (MAJOR) — The suffix set is unjudged at the mid-line slots and between the groups of a company

Red team RT42-S1. R40-G/R41-E scoped the peels to
leading / trailing / base-trailing. At the mid-line
slot there is no suffix peel at all:

- Lone mid-line governor: `notes (cancelled)X
  November 8, 2026` and the spaced-token form serve
  Nov 8 **live** for every X in `~ * # - —` — and
  for the ellipsis control `…`, which no peel
  anywhere claims at this slot. The clean line
  governs.
- Mid-line company: a suffix on the governor group
  OR on the weekday group serves Nov 1 silently,
  where the clean company composes governed with
  the record. Spaced double tokens disarm here too.
- Inter-group suffix inside a head company splits by
  subset: `(cancelled)~ (Monday)`, `(cancelled)#
  (Monday)`, `(cancelled)… (Monday)` serve silently,
  while `*`, `-`, `—` in the same seat are judged —
  the mid-line reader peels a subset, and the subset
  is not the R41-E set.
- Base-trailing company: `Sunday after Nov 1
  (cancelled)~(Monday)` serves the resolution
  silently, while the line-final form
  `(cancelled)(Monday)~` composes — R41-E's own arm
  intact, the inter-group seat unjudged.

Reading: judgments read through the witnessed set at
the mid-line slots and between a company's groups as
they already do at the leading slot. Disposition:
R42-C.

### F7 (MAJOR) — At R41-E's own trailing slot: a suffix on a non-final group of a trailing sequence, and spaced double tokens, disarm the judgment

Red team RT42-S2. The peels strip the sequence's
tail; a suffix worn by an interior group is never
reached: `November 1, 2026 (Monday)~ (notes)` serves
Nov 1 silently (the weekday record lost) and
`November 8, 2026 (cancelled)~ (notes)` serves the
cancelled date live. Control held: a suffix on the
inert last group is judged. And spaced DOUBLE suffix
tokens at the trailing governor serve live —
`November 8, 2026 (cancelled) ~ ~` and
`(cancelled)~ ~` serve Nov 8 on both routes, where
the single spaced token and the glued double `~~`
are judged; the weekday arm with `~ ~` is judged
(held) — the asymmetry is the governor run's
one-token peel. Disposition: R42-D (with F8, F10).

### F8 (MINOR) — The governor-LAST family at the trailing slot: `~`, `#`, `…`, `–`, `~~`, `~*` serve the cancelled date; the rest of the family governs or stands at parity

Red team RT42-S3, graded per adjudication 2. On this
tree the clean governor-last forms GOVERN at the
trailing slot, glued and spaced — `November 8, 2026
(notes)(cancelled)` reads None with no record (the
round-41 observation's clean case refined: it does
not serve). The suffixed forms split exactly along
R41-E's recorded partial reach: `*`, `-`, `—`
govern; `~`, `#`, the ellipsis `…`, the en dash `–`,
and the doubles `~~` and `~*` serve Nov 8 live. At
the base-trailing slot the whole family is at parity
in every dress (serves the resolution, as the clean
governor does there) — not an arm. Review's probe42
table agrees cell for cell and adds the spaced forms
(governed, both marks probed). Disposition: R42-D —
the trailing group-sequence reader takes the full
witnessed set at its final group in governor-last
order as it does in governor-first order.

### F9 (MINOR) — Marks outside every judged set disarm the trailing and leading weekday judgments; the en dash U+2013 is the headline cell

Red team RT42-S4. At the trailing slots, every mark
tested that belongs to no peel, suffix, or sentence
set splits in the unsafe direction (governor serves,
grouped weekday serves silently, bare weekday loses
its record): en dash –, horizontal bar ―, underscore,
+, =, /, \, %, @, &, $, ^, |, backtick, >, <, •, ·,
°, ‰, ∼, fullwidth tilde ～, asterisk operator ∗,
fullwidth ＃. The sentence marks `: ; ! ? . ,` and the
closers `) ] } ' "` do NOT split (judged). At the
leading slot the weekday arm disarms under –, %, |, •
while the governor arm HOLDS under the same four
marks — the same mark at the same slot shed for one
group kind and not the other; base-trailing weekday
disarms under all four. Graded MINOR as a family:
these marks were never in any judged set, so this is
the general closed-peel limit rather than a broken
pin. Disposition: R42-F, scoped per adjudication 7
(the en dash judged; the remaining marks pinned as
the standing closed-peel record).

### F10 (MINOR) — A suffix on a trailing-alone group disarms the R41-D memory in both dresses

Red team RT42-D1. `next Sunday (November 8, 2026)X`
and `next Sunday "November 8, 2026"X` lose the alone
memory AND the keep-through entirely for X in
`*, #, ~, …, !` (clean twins: memory carried,
keep=2026-11-08); for X in `-, —` the For-route
memory survives but the Keep-route keep-through is
lost. Dress parity holds — the suffix disarms both
dresses equally — so this is the suffix mechanism
reaching the alone slot, not a dress split.
Leading-alone suffixes are judged for the whole set
(held). Disposition: R42-D (with F7, F8).

## Records (not findings)

- **Archive candidate discipline — HELD whole, sixth
  generation** (red team surface 1; review section
  (a) concurs on a full code read). 62 single-field
  record mutations (both version fields, flags,
  methods, times, CRC, sizes ±1, disk, attrs,
  extra-field subfields and their order, comments,
  name spellings) all subordinated — the real
  directory judged, reader and all four doors
  identical to the byte-identical-mirror control;
  16 zip64 fake-pair variants + 5 EOCD-pairing
  variants outcome-identical at all four doors plus
  the model door; signature-record plants move
  nothing. The whole-record comparison is the whole
  record: review verified every fixed field is
  either captured directly or determined by captured
  bytes.
- **Rights widened sweeps — HELD** (red team surface
  7): the 20,000-quadruple sweep reproduces round
  41's counters to the unit (standing_flips=0;
  credit-text flips only, as recorded since round
  38); the 8,000-quintuple × 120-permutation sweep
  shows safety2_flips=0 and children variance 0
  everywhere. M8's safety axes hold for the
  twenty-second consecutive round (verify claim 2).
- **Perf — HELD, with the standing observation
  carried** (verify claim 5; red team §6). All
  R40-bounded families held at 2–4× round 41's
  driven sizes, linear against their controls
  (nested_day / month / whole / trail-group,
  empty-group runs, glued weekday runs, leading
  sequences, deep mixed day). Two observations,
  both lanes' windows stated: (i) `nested_seq_mix`
  keeps its shape — verify's quieter window (load
  3.6–5.6, debug frame, interleaved) measured ratios
  4.76–5.32 at n=6,400 and 5.59–6.92 at n=25,600,
  with a small real size step (~1.25× per 4× size,
  bands disjoint) and round 41's 2.7–20.7×
  cross-window scatter NOT reproduced; the red
  team's loaded window (load 3.4–10.3) reads the
  ladder 3.78s → 7.67s → 21.2s best readings across
  25,600 → 51,200 → 102,400 (7–17× its same-length
  control, 2.6× spread). No standing bound exists
  for this probe shape and none was missed; the
  family's in-tree pins held in the settled suite
  run. Carried to round 43 as a named target: a
  dedicated quiet-window measurement, and the
  standing question whether nested_seq_mix earns an
  interleaved ratio pin of its own. (ii)
  Trailing-sequence families at ~1.6–2.5MB documents
  cost 58–81s of DEBUG parse in absolute terms, but
  their ratio to the (also bending) same-length
  control stays in the established band (7.4× →
  8.4×); no ratio beyond the band can be certified
  at this noise level. Absolute debug times are not
  production estimates; both statements name their
  frame and load, per the standing rule.
- **Peeled-tail residual — HELD, re-witnessed**
  (review section (c)): every peeled form serves
  with no record at all three slots, exactly as
  recorded; no slot moved, no arm widened. Not
  re-graded (adjudication 2 distinguishes F8's
  cure); cure unchanged (a peel-depth change).
- **Split-position governor uniformity — HELD,
  re-witnessed** (review section (c)): uniform None
  in every dress — paren, bracket, bare, quoted —
  both routes, no record.
- **Lock leading-newline pid spelling — HELD,
  code-read re-witness** (review section (c)):
  `instance_lock_text` writes `{pid}\n{token}\n`;
  a leading-newline file parses pid=None and fails
  the legacy fallback, naming no holder under the
  standing decision table, exactly as recorded.
  R41-F touched neither function.
- **R40-C calibration — HONEST** (review section
  (e)): both sequence pins' comments disclose the
  steepness-ceiling shape with bite evidence named
  (16× arm: pre-fix solo failure at a 10× bound;
  20× arm: bite at a 9× bound, worst in-suite
  standing disclosed). The disclosure stands on
  this tree.
- **The restated pins' named arms — HELD** (red team
  surface 8; review section (g) and its
  corroboration runs, `test-r40e.log` 3/3 and
  `test-r39h.log` 2/2, TEST_EXIT=0): every split
  found adjacent to the R41-B/R41-D restatements
  decomposes into the named arms plus the named
  bare-governor lapsing serve; this round's findings
  sit outside the named arms (positions, company
  seats, lead-in positions, suffix seats). No pin in
  the round-41 diff was weakened without a flagged
  restatement; no new absolute wall-clock timing
  assert was introduced anywhere in the diff (review
  section (g), grep over the full diff).
- **Process records** (all three lanes): phantom
  completion notices recurred in every lane and were
  discarded in every lane — review discarded two
  (a build reported complete before its binary
  existed; a test run reported at exit 101 whose disk
  log records TEST_EXIT=0), red team one (a probe
  build announced complete while its cargo process
  was still alive), and verify read no verdict from
  any notice. Verify's suite runs 1–2 each failed a
  single ratio pin under co-tenant load (run 1:
  `r31c_conjoined…` at ~103.5× against its 75× bound
  at load ~17, the same pin's round-41 load shape;
  run 2: `r40b_empty_group_run…` at 21.46× against
  its 20× bound) — both passed solo and in-suite in
  the settled runs, the standing load disposition,
  no bound widened. Verify's run 3 and the red
  team's perf script B were each killed by a VM
  replacement (uptime reset ~07:08 in the red team's
  window); partial runs were discarded, never
  quoted, and the lost measurements re-run on the
  fresh VM. No verdict in this round rests on a
  phantom notice or a partial run.

## Batch plan

One writer at a time; every batch under the
batch-boundary discipline (commit + push +
HEAD == origin confirmed, `cargo clean`, cold
verification replacing incremental; per-batch
records in `round42-out/coord/` in the standing
format). **Every batch record states its hosted-CI
conclusions for its tip (all three OS legs, checked
via `gh run view`); no tip is "verified" on local
evidence alone** — the standing consequence of the
R39-B incident, Jacob's direct order. Contribution
batches carry the fuzz hard term: verdict/comp
byte-identical to the standing baselines
(`79775844…` / `5868726a…`), rights counters
5083 / 1024 / 5275, safety_flips=0 — a batch that
moves a fuzz reading stops and reports; a movement
that is an adjudicated reading made visible is
ruled on by the coordinator on the diff, on the
R34-E / R36-D / R38-C precedent, with the delta
table preserved in the batch record.

- **R42-A** — F3 + F4 (quoted re-dress reach): the
  consultation extends to every position where the
  composition judges a bracket group — the framing
  stack statements widened to the widths the
  composition itself judges (both arms, the
  before-split arm included); the plain-line
  interior positions consulted (the four witnessed
  positions and the governor-adjacent arms); company
  members in quoted dress taken as their bracket
  twins; doubled quote forms at the alone positions
  take the single-quote reading. Pins: the lanes'
  families (framing stacks of 3 and 4 words,
  numbered stacks, before-split stacks; interior
  positions × quote forms × both routes against the
  bracket twins; the quoted-company family in both
  orders and dresses; doubled-quote alone forms
  against their single-quote twins), every standing
  quoted-group pin unweakened, the alone/framing
  scope controls restated only where an adjudicated
  arm names them.
- **R42-B** — F5 (company edges): the lead-in run no
  longer collapses the company for a weekday/month
  word in a non-first position (adjudication 3; the
  first-word wall stands as named and is pinned
  unmoved); the base-trailing company behind an
  ordinary word takes the composed reading
  (adjudication 4); the company walk's separator set
  takes the straight quote closers `"` and `'`,
  glued and spaced, making R41-B's record claim true
  (adjudication 5). Pins: RT42-C1's families in full
  (lead-in positions × words × companies; the
  behind-a-word base-trailing family against its
  direct-slot twin; closers at head and mid-line,
  glued and spaced, with the curly controls),
  standing company pins unweakened except any
  restatement adjudication 4 names, flagged.
- **R42-C** — F6 (suffix at mid-line and between
  groups): judgments read through the witnessed set
  `~ * # - —` and the ellipsis at the mid-line slots
  (lone governor, company members) and between a
  company's groups, the inter-group subset unified
  to the full set; spaced double tokens at the
  mid-line company judged as at the leading slot.
  Pins: RT42-S1's families (mid-line governor over
  the set + ellipsis, glued and spaced; suffix on
  either company member; inter-group seats over the
  full set; the base-trailing inter-group arm),
  the leading/trailing controls and every R40-G /
  R41-E pin unweakened.
- **R42-D** — F7 + F8 + F10 (trailing seats): a
  suffix on any group of a trailing sequence is
  judged, not only the sequence's tail (the interior
  seats and the governor-last order, per F8's map);
  spaced double suffix tokens are judged at the
  trailing governor run as the glued doubles are;
  a suffix on a trailing-alone group no longer
  disarms the R41-D memory in either dress or route
  (the `-` / `—` Keep-route arm included). Pins:
  RT42-S2's families, RT42-S3's full governor-last
  table, RT42-D1's alone families (both dresses,
  both routes, memory and keep-through stated
  outright), the R41-E controls and the R41-D alone
  pins unweakened.
- **R42-E** — F1 + F2 (lock claim loop): the check
  and the capture become one judgment about one
  generation — the captured generation is itself
  judged (liveness on the pid inside the captured
  record), or the generation is re-read and compared
  before any displacement — so a live winner landing
  between the two reads is the generation judged,
  never displaced as dead; and, per adjudication 6,
  the rename step's own path is judged at the last
  moment before its removal in the R41-F pattern, or
  the attempt is recorded and F2 carries with its
  rate. Pins: the seam witness (the L8 shape driven
  through the production functions: both_hold never
  true; the L8b control refuses as today), composed
  four-claimant races over a dead lock standing
  single-winner, every standing lock/retirement pin
  (incl. the R33-A forced interleavings, R40-H's and
  R41-F's pins) unweakened; `peer_provably_gone`
  still rests on `PeerLiveness::Dead` alone; the
  recycled-pid boundary untouched.
- **R42-F** — F9 scoped (the en dash): U+2013 is
  judged at the arms where `-` and `—` are judged
  (trailing governor / grouped weekday / bare
  weekday, leading weekday, base-trailing weekday —
  RT42-S4's witness table), and the remaining marks
  of RT42-S4's table are pinned as controls stating
  the closed-peel record (they stand as recorded).
  Pins: the en-dash family at every witnessed arm ×
  both routes against its `-` / `—` twins; the
  control pin over the remaining marks; every
  standing suffix/peel pin unweakened.

Then the coordinator close: five-leg evidence on
the final tree (local cold, fuzz as claims, hosted
CI ×3, desktop runner build, Windows bundle), the
build-cache policy disposition at the confirmed
completion (df reading + window + latch state over
the round's recorded readings), and boundary
hygiene. HF-12 is NOT a batch of this round: the
review lane's readiness map (round42-review.md
section (f)) scopes the first slice to ob-extract's
open doors, and the coordinator's proposal goes to
the coordinator seat with this synthesis — the
migration starts deliberately, between rounds, on
the seat's confirmation, not mid-round. The exit
bar is judged on round 43's audit of the closed
tree; the Tier-2 final gate is not triggered
(0/5/5) and is not the coordinator's to start in
any case.
