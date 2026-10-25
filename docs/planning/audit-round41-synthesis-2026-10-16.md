# Audit round 41 — synthesis (coordinator, 2026-10-25)

Audited tree: `1892877` (round-40 close record on code
tree `406ded0`). Three blind lanes, read-only: verify
(`round41-verify.md`), review (`round41-review.md`),
red team (`round41-redteam.md`), all in
`~/workspace/openbulletin-audit-wave3/`, scratch in
`round41-out/`.

Lane verdicts: verify PASS on all six claims plus the
R40-A deviation claim-check (0/0/0 handed on — suite
1156/0, clippy/fmt clean on a genuine re-check,
booklet 31,717 / 34,977 byte-exact, fuzz byte-identical
to the standing baselines, rights 5083 / 1024 / 5275
with safety_flips=0, M8 held for the twenty-first
consecutive round, hosted CI green on all three OS
legs on both the code tree and the tip, and all eight
round-40 batch-tip CI citations exact per-job);
review 0/1/1 plus the carried doctrine question graded
MAJOR (0/2/1 counting it); red team 0/4/2.
Deduped: **0 BLOCKER / 5 MAJOR / 2 MINOR** — the exit
bar is NOT met. Majors trend:
10 → 6 → 6 → 1 → 4 → 3 → 2 → 5 → 6 → 8 → 6 → 5 → 8 → 8 → 5,
and round 40's BLOCKER does not recur (the perf
families it belonged to held at 4–8× their round-40
bounds this round). The Tier-2 final gate is not
triggered and is not the coordinator's to start.

Every finding below was witnessed firsthand by at
least one lane on this tree; cross-lane convergences
and the coordinator's grade adjudications are stated
per finding. All seven findings are dispositioned to
batches R41-A..R41-F.

## Adjudications recorded before the findings

1. **F3's doctrine (the mid-line governor company) is
   adjudicated IN — EXTEND, scoped.** Review argued
   EXTEND on the full witnessed company; red team's
   RT41-D1 found the pinned serve itself is only one
   cell of the company (governor-first / weekday-last
   / unmarked), the other orders and dresses
   collapsing to silent None or taking the composed
   refusal by dress — the F9 shielding structure one
   word further in, internally split. The coordinator
   adopts review's argument: the governor half of the
   company is not (a)(i) territory (a mid-line
   governor group alone is already judged at that
   position), so a governor + weekday group sequence
   standing behind ordinary lead-in words is a judged
   company, not running text. Scope: a leading group
   sequence containing both a governor group and a
   weekday group, in any order, with companions, is
   judged as a company wherever that sequence stands
   — at the line head (R40-E) or behind ordinary
   lead-in words. The composed outcome is the R40-E
   composition: **the governor governs and the
   weekday records its WeekdayMismatch where it
   disagrees; the agreeing companion composes
   silently**. The lone weekday group behind ordinary
   words (`notes (Monday) …` in running text, no
   governor companion) keeps its standing (a)(i)
   unjudged reading, untouched. Marks and order arms
   resolve by construction under the composed
   reading. Any standing pin that encodes the
   mid-line company's pinned serve is restated
   outright in the batch, flagged in its commit and
   record; no other standing pin is weakened.
2. **F5's grade.** Review R41-R2 graded the
   leading-alone quoted date MINOR (nothing wrong
   served on the For route; the loss is memory and
   keep-through). Red team RT41-D3 graded the family
   MAJOR (the trailing-alone arm splits three ways
   across dresses). The facts are agreed; the grade
   turns on the Keep route: the paren twin's memory
   *sets the contribution's keep-through date*
   (keep=2026-11-08) and the quoted form sets
   nothing — a date the app itself carries for the
   sender's contribution is dropped in silence on one
   route, uniformly across quote forms, and the
   trailing-alone slot gives one contents set three
   outcomes by dress. Adjudicated **MAJOR**; review's
   MINOR dissent is recorded here in full reasoning:
   on the For route the quoted form serves silence,
   the safe direction, and no wrong date is adopted
   at the alone slots (adoption is F4's arm, at the
   framing slot).
3. **The interior-`.` arm is RULED, closing R40-F's
   carried record.** Both lanes' evidence agrees:
   grouped interior-`.` forms compose as inert
   contents at every slot (`(Mon.day)` serves
   slot-for-slot identically to the `(notes)` control;
   bare forms read identically to bare `notes`), the
   abbreviation family refuses with the record named
   "Mon" in both dresses at all slots, and no shed
   ruling pins both twins without moving standing
   pins. **Ruling: interior `.` is a sentence
   separator / inert-contents boundary, not a shed
   mark.** The grouped form is not a weekday group;
   the abbreviation refusals stand on the standing
   abbreviation reading. R40-F's pins stand as they
   stand; no batch. The arm does not carry further.
4. **F7's disposition.** R40-H narrowed the lock
   displacement window; RT41-L witnesses, at step
   level through the verbatim extraction, that the
   re-judge and the rename step's unconditional
   `remove_file` are still not atomic, while the
   composed claim loop holds every shape driven
   through it (four-claimant, directory, symlink
   included). Per the F10 precedent (adjudication 7
   of round 40): witness attempt through the in-tree
   seams first; the fix lands only on a witness. The
   suggested narrowing — the displaced path is judged
   inside the rename helper at the last moment before
   the deletion, standing down to the judge loop when
   a live generation stands there — is the batch's
   shape if the witness lands.
5. **The R40-A `prefix_free` deviation stands
   accepted.** Review's re-audit (target (d)) held
   the splice sound on a full code read plus 13
   crafts: judged facts on the spliced view equal the
   original's on every craft, and the only reader
   divergence is the adjudicated repair itself.
   Verify's claim-check confirms the tree matches the
   recorded description. RT41-B is a finding about
   which zip64 pair the splice *chooses*, not about
   the splice's soundness, and is dispositioned with
   the archive batch.

## Findings

### F1 (MAJOR) — Archive candidate discipline, fifth generation: an equal-offset-set plant whose records differ only in facts the F4 comparison never reads escapes the R40-A equal-set rule and is judged as the directory at all four doors

Red team RT41-A. R40-A's equal-set rule subordinates
a plant only where `record_facts_differ` sees a
disagreement; `directory_record_facts` captures, per
record, the decoded name and the spot (method,
compressed size, CRC, local offset), grouped **per
local offset**. Four fact dimensions stand outside
it (one craft per door each, gap of 2,000 bytes
before the real CD, plant CD+EOCD at gap+100,
declaring exactly the real directory's offset set;
`doors41.log`):
- **eqorder** — the plant's records are the real
  records in reverse walk order. Judged at every door
  with the spots in the plant's order, while the zip
  reader on the same view serves the real directory
  in real order — two different directories over one
  body. Door steering witnessed at fonts: the clean
  pack lands `MyFont.ttf` = REG with BOLD under the
  hashed name; the planted pack lands `MyFont.ttf` =
  **BOLD** with REG hashed — the plant decides which
  face bears the canonical file name.
- **equncomp** — the plant's records declare
  `uncomp_size = 1` for every entry, all compared
  facts identical. Judged at every door.
- **eqextra / eqversion** — the plant's records
  differ only in a UT extra field, resp. in
  `version_made_by` (99). Judged at every door.
One guard-comparison defect in four spellings: a
plant that rewrites any un-compared fact — or merely
re-orders the walk — is an "exact mirror" to the
rule and stands first in file order. Fix shape (from
the lane, adopted): the compared facts take the
whole record (the walk order itself, and the fields
the doors or the reader can read: uncompressed size
at least), or mirror-standing is confined to
byte-identical records. Disposition: R41-A (with
F2).

### F2 (MINOR) — The R40-A splice chooses a planted fake zip64 pair differing only in unvalidated `version_needed`, and the reader then refuses a good archive at all four doors

Red team RT41-B. A fake zip64 record+locator pair
standing in the locator→EOCD gap, differing from the
real pair only in `version_needed = 99`, is the pair
the splice lays into the prefix-free view; the zip
reader then refuses the archive at all four doors,
where the `version_needed = 45` control serves — a
serving clean host becomes a refused planted one.
The splice chooses between record pairs on fields it
never validates. Fix shape: the splice's pair choice
validates the fields the reader will judge (version
sanity / pair consistency with the EOCD and the
walked extent), or prefers the pair the unspliced
derivation validates. Disposition: R41-A (with F1).

### F3 (MAJOR, doctrine) — The mid-line governor company: the pinned serve is one cell of a split family — weekday-first orders, a third group, the agreeing order, and half the mark set collapse to silent None, and one bare dress takes the composed refusal

Review target (a), graded doctrine MAJOR and
adjudicated IN per adjudication 1; red team RT41-D1
is this finding's arms. The pinned cell: `notes
(cancelled)(Monday) November 1, 2026` and its whole
governor-first / weekday-last / unmarked family serve
2026-11-01 silently (any lead-in word, any distance,
all pair dresses, both routes), agreeing companion
included — while the mid-line governor alone governs
and the bare-weekday twin is governed *with* the
record. The company's edges split (RT41-D1):
weekday-first orders (`notes (Monday)(cancelled)
…`), a third group after the weekday, the agreeing
`(Sunday)(cancelled)` order, and marks `! ? ; :`
after the weekday group all collapse to silent
None; held for all 14 leading words thrown. Under
the adjudicated reading the batch judges the
company as a company — all orders, sizes, marks,
dresses — so the arms resolve by construction:
the composed outcome (governed, with the record
where the weekday disagrees) everywhere the
company stands. Disposition: R41-B.

### F4 (MAJOR) — A framing word between "after" and a quoted date group: the quoted form adopts the group's date where every bracket twin serves the base with the record

Review R41-R1 + red team RT41-D2, merged. `Sunday
after the "November 8, 2026" November 1, 2026` →
the quoted group is **adopted** (for = 2026-11-08,
the written base demoted to a second-date note, no
record), where the paren and bracket twins serve
the base with `ParenthesizedDateMismatch`. Uniform
over framing words (`the` / `on`, and per red team
also `choir` / `for` / `at` / `dear` and doubled
stacks), quote forms (straight / curly / single),
full and short date forms, both routes; the glued
arm reads silent None. The agreeing form is at
parity — the hole is exactly the disagreeing quoted
date group at the one interior position the R40-D
re-dress does not reach: the guard does not fire at
the framing position even for brackets, and the
paren reading there is the whole-line composition's
(parenthesized date, recorded), which the quoted
text, never re-dressed, never receives. One partial
arm: framing *before* the split group — outcomes at
parity, records diverge (paren carries the mismatch
record; quoted carries second-date notes only).
Fix shape (from review, adopted): the re-dress
consultation extends to the positions where the
composition judges a bracket group, framing stack
included, matched against the composition's own
pinned paren reading. Disposition: R41-C.

### F5 (MAJOR) — A leading-alone or trailing-alone quoted date is forgotten where its bracket twins are remembered: no in-words or parenthesized memory at all, and the Keep route loses the keep-through date

Review R41-R2 + red team RT41-D3, merged; grade per
adjudication 2. Leading-alone: `"November 8, 2026"
next Sunday` → for = None with **no memory fields**
(`words=None`, `paren=None`), both routes, all
quote forms and the short form — where the paren
and bracket twins read the same None *with* the
memory (in-words = the line, parenthesized =
2026-11-08) and on the Keep route the memory sets
keep-through = 2026-11-08, which the quoted form
loses. R40-D's record described the memories as
keeping "their quoted dress"; on this tree the
quoted form's in-words field is None — the relative
phrase itself is not remembered, so there is no
dress to keep (the memory readers in mod.rs are
gated on the in-words fields and
`relative_sunday_phrase` does not recognize the
quoted-group phrase). Trailing-alone: `next Sunday
"November 8, 2026"` serves a resolution with the
quoted date demoted to a second-date note, where
the paren twin reads None with the Nov 8 memory and
the bare twin serves Nov 8 itself — three outcomes
for one contents set at one slot. After-alone is a
parity arm, stated for the map and NOT moved by
this finding: `Sunday after "November 8, 2026"`
resolves exactly as its bare twin does. Fix shape
(from review, adopted): the memory readers
(`relative_sunday_phrase` / `all_parenthesized_
dates`) consult the same re-dressed text the date
readers consult where the guard fired, or the
quoted group is remembered as a group in its own
right — leading-alone and trailing-alone take
their bracket twins' memory and keep-through.
Disposition: R41-D.

### F6 (MAJOR) — The R40-G sequence-tail suffix set (`~ * # - —`) disarms every judgment it follows at the trailing and base-trailing slots; the set is judged only at the leading slot R40-G covered

Red team RT41-D4; review's target (f) witness table
agrees (grouped / bare / base-trailing arms serve or
lose the record across the set). For each of
`~ * # - —` (the ellipsis `…` is the control, judged
at every slot):
- Trailing governor: `November 8, 2026 (cancelled)~`
  (and each sibling) serves **2026-11-08 live**; the
  bare twin `cancelled~` governs. A cancellation is
  served on the strength of one trailing character —
  F8's governor arm, one suffix set over.
- Trailing grouped weekday: `November 1, 2026
  (Monday)~`, `(Monday)(notes)~`, `(notes)(Monday)~`
  serve silently; the clean forms refuse with
  WeekdayMismatch(Monday@2026-11-01).
- Trailing bare weekday: `November 1, 2026 Monday~`
  reads None with **no record**; the clean bare form
  refuses with the record.
- Base-trailing: `Sunday after November 1, 2026
  (Monday)~` serves the resolution silently; the
  bare `Monday~` serves it too (record lost). The
  base-trailing governor arm is parity (the clean
  `(cancelled)` serves the resolution there as
  well — not an arm).
- Doubles (`~~`, `~*`), spaced (` ~`), and
  trailing-space forms disarm identically.
- Leading slot (control): every suffix in the set
  is judged — R40-G's fix stands where it was
  scoped.
One mechanism, the F8 mechanism: the trailing peels
gained the ellipsis only in R40-G, so the five
witnessed-at-leading marks stand in no trailing
reader's peel set. Reading: judgments read through
the witnessed set at the trailing and base-trailing
slots as they already do at the leading slot.
Disposition: R41-E.

### F7 (MINOR) — The lock's R40-H re-judgment and the rename step's deletion are not atomic: a live generation landing between the re-judge and the rename is still deleted unjudged

Red team RT41-L; review's target (e) code read
agrees (R40-H narrows but cannot eliminate the
judge→rename TOCTOU — observation there, witnessed
here). Witnessed at step level through the verbatim
extraction: a live generation landing at the
displaced path between `displace_instance_lock_
from`'s re-judge and `rename_instance_lock_to_
displaced_from`'s unconditional `remove_file` is
deleted unjudged. The composed claim loop holds
every shape driven through it — including
four-claimant, directory, and symlink shapes — so
no composed witness stands. The standing
recycled-pid residual is unchanged in scope.
Disposition: R41-F, per adjudication 4 (witness
attempt first; the last-moment-judge narrowing
lands only on a witness; on no witness the batch
is the attempt's record and the observation
carries).

## Records (not findings)

- **Perf — HELD whole** (red team target (a)): all
  R40-bounded families driven to 4–8× their round-40
  bounds with interleaved ratios flat — nesting
  ~1.5× at depth 102,400, trail sequences 8.5×
  (R40-C's recorded band), glued runs ~7×. Two 5-rep
  batch timeouts under load 4–11 are recorded as a
  limitation, not hangs. The `nested_seq_mix` ratio
  is the noisiest line in the sweep and is flagged
  for round 42's lanes as an observation.
- **Interior `.` — RULED** (adjudication 3): closes
  as a record; R40-F's pins stand.
- **Peeled-tail residual — HELD** (review target
  (f)): the R38-F residual and its round-40
  extensions serve with no record at all three
  slots (the leading and base-trailing arms are new
  slot coverage of the same record). Not re-graded;
  the residual stands as recorded, cure unchanged
  (a peel-depth change).
- **Split-position governor uniformity — HELD**:
  uniform None in every dress, re-witnessed.
- **R40-C calibration — HONEST** (review target
  (f), code read): the 16×/20× sequence pins are
  interleaved ratio ceilings, disclosed in the pins'
  own comments with bite evidence in the batch
  record — with the stated cost that the pre-fix
  tree would pass both at shipped bounds. Not
  disguised absolute bounds.
- **Rights widened sweeps — HELD** (red team): the
  fresh 20,000-quadruple sweep stands at
  standing_flips=0 (21st consecutive round) and the
  new 8,000-quintuple × 120-permutation sweep shows
  safety-axes flips 0, children variance 0.
- **Process records** (all three lanes): phantom
  completion notices recurred — most sharply, a
  runtime delivery to the verify lane contained
  CI run IDs (40221142276 / 40221142285, "completed /
  success" on `1892877`) that return HTTP 404 on
  direct query and appear in no disk log; discarded,
  never quoted, every CI citation in the lane
  reports re-derived from disk logs and the gh API.
  A VM restart killed the red team's first probe
  build behind a fabricated exit-0 (rebuilt to a
  disk-logged BUILD_EXIT=0 before any measurement).
  One load disposition: the r31c conjoined-groups
  ratio pin missed its 75× bound in the verify
  lane's suite runs 1–2 (77.5× / 86.5× at load peaks
  ~8→14 with sibling lanes compiling), passed solo
  and in-suite in the settled run 3; no bound was
  widened — the R40-B load-disposition shape.
  Blindness note: one `ps` listing in the red team
  lane incidentally exposed a sibling lane's process
  argv; no sibling file was read and no sibling
  evidence was used — recorded so the round's
  blindness provenance is complete.

## Batch plan

One writer at a time; every batch under the
batch-boundary discipline (commit + push +
HEAD == origin confirmed, `cargo clean`, cold
verification replacing incremental; per-batch
records in `round41-out/coord/` in the standing
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

- **R41-A** — F1 + F2 (archive candidate
  discipline, fifth generation): the compared
  facts take the whole record — walk order and
  every field the doors or the reader can read,
  uncompressed size at least — or mirror-standing
  is confined to byte-identical records; and the
  splice's zip64 pair choice validates the fields
  the reader will judge (version sanity / pair
  consistency), or prefers the pair the unspliced
  derivation validates. Pins: RT41-A's four
  spellings × four doors with clean and duplicate
  controls, the fonts canonical-name steering
  craft foremost; RT41-B's fake-pair craft with
  its `version_needed = 45` control; every
  standing door/zip pin unweakened, the R40-A
  crafts included.
- **R41-B** — F3 (doctrine, mid-line company):
  a leading group sequence containing a governor
  group and a weekday group is judged as a
  company wherever the sequence stands, per
  adjudication 1. Pins: the full company —
  governor-first and weekday-first orders, two-
  and three-group companies, companions in every
  position, marks after any group (RT41-D1's
  split set closed), the agreeing order, all
  pairs and lead-in words, both routes, full
  records against the composed reading stated
  outright; standing pins encoding the old
  mid-line serve restated outright and flagged;
  the (a)(i) plain mid-line controls (`notes
  (Monday) …` in running text) stated unjudged
  and unchanged.
- **R41-C** — F4 (framing word before a quoted
  date group): the re-dress consultation extends
  to the framing positions where the composition
  judges a bracket group, framing stack included.
  Pins: RT41-D2's family (every framing word and
  stack, three quote forms, full and short forms,
  both routes) against the bracket twins, the
  framing-before-split record arm, the agreeing
  controls and every R40-D pin unweakened.
- **R41-D** — F5 (alone quoted dates): the memory
  readers consult the re-dressed text where the
  guard fired, or the quoted group is remembered
  as a group in its own right. Pins: leading-alone
  and trailing-alone quoted forms × three quote
  forms × both routes against their bracket twins
  (memory fields and keep-through stated
  outright); the after-alone parity arm stated
  unmoved; the lone-quoted-line silence (R40-D's
  corrected pin) unweakened.
- **R41-E** — F6 (suffix set at trailing slots):
  judgments read through `~ * # - —` at the
  trailing and base-trailing slots as they do at
  the leading slot. Pins: the governor arm, the
  grouped-weekday arms, the bare-weekday record
  arms, doubles / spaced / trailing-space forms,
  the ellipsis controls and every R40-G pin
  unweakened, both routes.
- **R41-F** — F7 (lock judge→rename window):
  witness attempt through the in-tree seams first
  (adjudication 4); on a witness, the rename
  helper judges the displaced path at the last
  moment before its deletion and stands down to
  the judge loop when a live generation stands
  there; the standing lock/retirement pins (incl.
  the R33-A forced interleavings and R40-H's pin)
  unweakened. On no witness, the batch is the
  attempt's record and the observation carries to
  round 42 as a record.

Then the coordinator close: five-leg evidence on
the final tree (local cold, fuzz as claims, hosted
CI ×3, desktop runner build, Windows bundle), the
build-cache policy disposition at the confirmed
completion (df reading + window + latch state over
the round's recorded readings), and boundary
hygiene. HF-12 untouched. The exit bar is judged on
round 42's audit of the closed tree; the Tier-2
final gate is not triggered (0/5/2) and is not the
coordinator's to start in any case.
