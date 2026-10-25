# Audit round 40 — synthesis (coordinator, 2026-10-24)

Audited tree: `9998e1e` (round-39 close record on code
tree `2786a6d`). Three blind lanes, read-only: verify
(`round40-verify.md`), review (`round40-review.md`),
red team (`round40-redteam.md`), all in
`~/workspace/openbulletin-audit-wave3/`, scratch in
`round40-out/`.

Lane verdicts: verify PASS on all six claims (0/0/0
handed on — suite 1131/0, clippy/fmt clean on a genuine
re-check, booklet 31,717 / 34,977 byte-exact, fuzz
byte-identical to the standing baselines, rights
5083 / 1024 / 5275 with safety_flips=0, M8 held for the
twentieth consecutive round, hosted CI green on all
three OS legs on both the code tree and the tip);
review 0/4/2 plus the carried doctrine question graded
MAJOR (0/5/2 counting it); red team 1/8/1.
Deduped: **1 BLOCKER / 8 MAJOR / 1 MINOR** — the exit
bar is NOT met. Majors trend:
10 → 6 → 6 → 1 → 4 → 3 → 2 → 5 → 6 → 8 → 6 → 5 → 8 → 8,
and this round adds the loop's first BLOCKER since the
round-26 parser panic. The Tier-2 final gate is not
triggered and is not the coordinator's to start.

Every finding below was witnessed firsthand by at
least one lane on this tree; cross-lane convergences
and the coordinator's grade adjudications are stated
per finding. All twelve findings are dispositioned to
batches R40-A..R40-H.

## Adjudications recorded before the findings

1. **F1's grade.** Red team RT40-P1 (two completed
   timeouts: >120 s and >90 s on a 15,405-byte line,
   control 32 ms in-window) and review R40-R3 (61.65 s
   min-of-3 at the same size, interleaved ratio
   1,041×) are one defect: the letterless `()`×n run.
   The F9 precedent (MAJOR at 25 s / 15 KB) is
   distinguished: those witnesses *completed*; these
   did not complete at 90–120 s. A sender-controlled
   15 KB line that does not finish is a hang.
   Adjudicated **BLOCKER**.
2. **F2's grade.** Red team RT40-H and review R40-R2
   overlap on nested_day. Review graded that arm
   MINOR (hang threshold ~51 KB of pure nesting,
   beyond plausible office input); red team's wider
   family — nested_month, nested_whole, a nested
   trailing group steepening to a 21.6× top step, and
   the *plain* trailing / base-trailing sequence
   walks over ordinary `(notes)` runs reaching
   4.8–6.1 s at 102 KB with rising ratios — is
   sender-plausible throughout. One finding, graded
   **MAJOR** on the whole family; review's MINOR
   dissent on the nested_day arm alone is recorded
   here.
3. **The carried doctrine question (target (a)) is
   adjudicated IN — EXTEND, scoped.** Verify
   confirmed the pinned parity firsthand (no twin
   divergence in the governor-first + weekday-
   companion company; handed on as a held record).
   Review graded the pinned reading itself a doctrine
   MAJOR; red team found the company's edges split
   (F9 arms (d)/(e) below). The coordinator adopts
   review's argument and the R39-C record's Option 2:
   two judged contents must not shield each other by
   pass order. At the leading slot, a weekday group
   standing in a leading group sequence is judged —
   records its WeekdayMismatch and refuses — whether
   or not a governor group stands first in the same
   sequence; the governor still governs. The composed
   outcome for `(cancelled)(Monday) November 1,
   2026` is therefore: **governed (the date is not
   served), with WeekdayMismatch(Monday@2026-11-01)
   recorded** — the governor-free leading sequence's
   own composition at this slot. The agreeing
   companion composes silently, as the governor
   alone does. Scope: (a)(i) is re-opened ONLY in
   governor-companion company at the leading slot;
   the plain mid-line weekday group behind ordinary
   words (`notes (Monday) …` in running text) keeps
   its standing unjudged reading, untouched. The two
   pin arms that encode the old reading — R37-D's
   companion pin (`r37d_companion_contents_keep_
   their_standing_judgments…`, tests.rs:23057–23061
   as restated by R38-D) and R38-B's separator pin
   (`r38b_sequence_walks_take_the_lone_forms_
   sentence_separators`, tests.rs:26931–26934) — are
   restated outright in the batch, flagged in its
   commit and record, and no other standing pin is
   weakened. Disposition: R40-E, with arms (d)/(e).
4. **F5's reading.** Governor contents standing
   between `after` and the Sunday-after base are
   judged as a governor in every dress (spaced,
   glued on either or both sides, any pair, impure
   contents): the line reads nothing, as its bare
   form does. Inert groups at the slot keep serving
   the base exactly as today (the `(notes)` control
   is the standing reading, not a defect). The
   glued-to-base resolution serve and the
   glued-both-sides silence both collapse into the
   judged reading.
5. **F8's reading.** A trailing suffix outside the
   standing suffix sets, standing after a judged
   word, group, or sequence, is garnish dress: the
   judgments read through it as they read through
   the standing marks (a wrapped governor governs,
   a weekday records its mismatch). Scope is the
   witnessed families — `…` (U+2026) and the
   sequence-tail set `~ * # - —` — at the slots the
   finding names; the batch does not generalize to
   unwitnessed suffixes without witnesses.
6. **F7's `.` arm.** Whether `(Mon.day)` is a
   weekday group at all is decided in the batch
   against the abbreviation controls (`Mon.` forms),
   under the standing stop rule: if no reading pins
   both twins together without moving standing pins,
   the batch stops and reports rather than forcing
   one (the R39-C arm-(d) pattern).
7. **F10's disposition.** Witness attempt first,
   through the in-tree claim-loop test seams that
   forced the R33-A interleavings. The fix lands
   only on a witness; if the interleaving cannot be
   witnessed after a genuine attempt, the
   observation stands as a record with the attempt
   logged in the batch record.

## Findings

### F1 (BLOCKER) — A line of empty groups `()`×n hangs the contribution parse at plausible paste size

Red team RT40-P1 + review R40-R3, merged per
adjudication 1. `()`×7,680 (15,405-byte document) did
not finish in 120 s (window 1) or 90 s (window 2);
review's completed witness at the same size is
61.65 s min-of-3, interleaved ratio 1,041× against a
byte-matched long-word control (32–64 ms). The ladder
steepens: 480 reps 245–652 ms, 1,920 reps ~2.7–4.2 s.
The reading is None at every size — the shape is pure
cost. Mechanism (both lanes, code-read): the run is
one letterless glued token, so the bounded token-edge
scans in the shape/strip loops — which stop at the
first alphanumeric, and the line has none — run
whole-text per iteration, one iteration per group,
across several whole-line passes (relative-shape
head/tail re-clean, the date-line strip loops,
`split_last_trailing_group`). R39-G named this shape
an observation at 1,920 reps (1,641 ms) and bounded
the lettered shapes; the letterless arm was left.
Disposition: R40-B. The fix gives the letterless
shape the same bounded-scan treatment; readings stay
None and every standing pin stands.

### F2 (MAJOR) — The group-sequence and nesting readers keep growing superlinearly past the R39-G pinned range: nested shapes continue ~9–11× per 4× step to depth 25,600, and the plain trailing / base-trailing sequence walks reach seconds on ordinary group runs

Red team RT40-H + review R40-R2, merged per
adjudication 2. Nested_day: 59 ms / 535 ms / 4,500 ms
at depths 1,600 / 6,400 / 25,600 (two windows agree);
nested_month 5,471 ms and nested_whole 6,235 ms at
25,600; a Monday group nested at the trailing slot
steepens 2.0× / 6.9× / **21.6×** (2,473 ms at depth
25,600). Plain runs with no nesting at all:
trail_seq (`Nov 1 (notes) ×n`) 4,843 ms and
base_trail_seq 6,056 ms at 12,800 groups (102 KB),
per-step ratios still rising; (Monday)(notes)
sequence pairs 2–3 s at 54 KB. Review's code read:
the descents themselves are flat post-R39-G; the
cost is the whole-line pass stack in `date_line`
re-deriving the top-level span per pass and the
group-sequence / spaced-glued readers walking per
group start. Readings are preserved at every size
in both lanes' runs. Disposition: R40-C. New timing
pins take the interleaved ratio form (standing rule
since R39-CI); no absolute wall-clock asserts.

### F3 (MAJOR) — Archive candidate discipline, fourth generation (post-directory arm): a planted directory standing in any byte region after the real directory's walk is the sole standing candidate and steers all four doors

Red team RT40-A. R39-A's walked extent accounts for
every byte from a candidate's archive start to the
end of its directory walk; everything *after* the
walk's end is unaccounted, and the F2 pattern
repeats there. Five regions witnessed, one craft
per door per region (20 crafts, `doors40c.log`):
the gap between the central directory and the EOCD
(postcd-gap), the gap after a signature record
(sig-gap), and the three gaps inside a zip64 tail
(CD→zip64 record, record→locator, locator→EOCD).
In each, the clean host is underivable while the
reader serves it (the doors refuse a good archive —
the safe direction on its own), and a plant in the
region is the only candidate the scan derives:
IDML serves the second copy / first story, .mxl
loads the planted score, fonts land only the
Regular face, clips land one picture; at the two
locator-side zip64 gaps the reader opens the fake
directory on the `prefix_free` view while the
unplanted file is refused outright. Clean duplicate
controls refuse / land both at every door.
Disposition: R40-A (with F4). Fix shape (from the
lane, adopted): the walked extent accounts for the
archive's tail as it accounts for its head and
middle — every byte from a standing candidate's
walk end through its own end record, the zip64
structural records and the gaps between them
included — and a host whose tail gaps make it
underivable while the reader serves it gets the
same derivation repair F2 gave the uncounted
signature record, so the real candidate stands and
the plant is subordinated.

### F4 (MAJOR) — Archive candidate discipline, fourth generation (equal-set arm): a directory planted in the gap before the central directory whose records declare exactly the real directory's local-offset set escapes the R39 accounting guard and steers the scan

Review R40-R5. R39-A's gap fill subordinates a plant
only where the filling candidate accounts for every
local header the plant accounts for *and at least
one more* — the strict-superset guard. A plant
standing inside the gap whose declared offset set
is *equal* to the real directory's is subordinated
by nothing: not the fill, not any walked span (the
gap is in no span), not the comment/payload/zip64
disciplines. Witnessed (`zip40b-40.log`; one real
entry `a.txt`, a 400-byte gap carrying a fake
CD+EOCD; the reader serves the real archive in
every row): **rename** — the plant's record names
offset 0 as `evil.txt` and is judged (total=1,
cd_start=60, spot span (35,44)) while the reader's
entry is `a.txt`; **phantom** — two records at
offset 0 judge total=2, inventing an entry the
archive does not contain; **trunc** — a record at
offset 0 with comp_size 4 judges a 4-byte payload
span where the reader serves nine bytes. The doors
take their spots from the judged directory, so the
steering outcome is F1/F2/F6's, one guard-condition
further. The clean control and both predecessor
subset plants are judged correctly. Fix shape
(from the lane, adopted): a candidate whose end
record stands in another candidate's walked extent
and whose records differ in any judged fact
(names, counts, sizes) from that candidate's
records for the same offsets is the extent owner's
bytes, not its directory; exact mirror copies
judge identically either way, so subordinating
them costs nothing. Disposition: R40-A (with F3).

### F5 (MAJOR) — Governor after "after": the wrapped governor at the Sunday-after interior slot escapes governance entirely — spaced serves the base live, glued-to-base serves the *resolved* date, glued on both sides reads nothing

Review R40-R1 + red team RT40-C, merged; reading
per adjudication 4. `Sunday after (cancelled)
November 1, 2026` serves the base live
(for = 2026-11-01) with no record on either route,
in every pair, doubled, glued-to-"after", impure,
and framing-word forms; the bare twin reads
nothing. The inert control `Sunday after (notes)
November 1, 2026` serves identically — the slot
consumes any group as garnish; the governor's
escape is complete and invisible on the Check
surface. Glued arms (RT40-C): `(cancelled)November
1, 2026` glued to the base serves the *resolution*
(2026-11-08; at base Nov 8, 2026-11-15) in every
dress, companions included — the group stops
standing beside the phrase and the phrase resolves
through it; glued on both sides
(`after(cancelled)November`) reads nothing, the
bare outcome from a wrapped group. Three outcomes
for one contents family at one slot, decided by
which side of the group the space stands on.
Weekday-bearing bases split too (review):
`Sunday after (cancelled) Monday November 1, 2026`
refuses with the record while the bare-governor
form serves silently — the batch pins the family
whole. Disposition: R40-D (with F6).

### F6 (MAJOR) — A fully quote-wrapped date group escapes the Sunday-after interior-slot guard at every position the guard covers for bracket groups

Red team RT40-B. The R38-C / R39-B guard
(`group_beside_relative_core` /
`group_split_sunday_after_phrase`) is built for
balanced bracket groups; a date wrapped in straight
`"…"`, curly `“…”`, or single `'…'` quotes is not a
group to it, and the plain readers adopt the quoted
date where the bracket twins are judged. Uniform
across quote forms, both routes:
- Split position: `Sunday "November 8, 2026" after
  November 1, 2026` → for = 2026-11-08 adopted, the
  written base demoted to a second-date note, no
  record; the bracket twins read nothing. The
  agreeing form splits too.
- Between `after` and the base: `Sunday after
  "November 8, 2026" November 1, 2026` → for =
  2026-11-15, a resolution computed from the quoted
  group as phantom base, the written base demoted,
  no record; the bracket twins serve the base with
  a ParenthesizedDateMismatch record.
- After a framing word: the quoted form adopts
  Nov 8; the twins serve the base with the record.
- After a weekday word: `Monday "November 8, 2026"
  next Sunday` *refuses* with a manufactured
  WeekdayMismatch(Monday@2026-11-08); the bracket
  twins adopt Nov 8.
The quoted weekday and quoted governor forms at
the same slots are at parity (held lines) — the
hole is the quoted *date* group, the one contents
class the guard exists to judge. Disposition:
R40-D (with F5). Fix shape: the guard's group
notion extends to fully quote-wrapped date groups
at the positions it already covers, judged as
their bracket twins are judged.

### F7 (MAJOR) — A weekday word inside a group carrying two interior sentence marks escapes the grouped weekday judgment at all three garnish slots; the bare twins refuse with the record

Red team RT40-F + review R40-R4, merged. R39-E
gave the grouped readers a one-mark shed
(`cleaned_weekday`, composition.rs:1702ff — its
comment states the one-mark scope); R39-F extended
the bare path to mark runs and a second interior
mark, and the grouped arm never took the
extension. `(Mon!!day)`, `(Mo!n!day)`,
`(Mon!da!y)` serve silently at the trailing,
leading, and base-trailing slots; the bare twins
refuse with WeekdayMismatch(Monday@2026-11-01) at
all three. Single-mark grouped controls refuse
with the record, field-identical to their bare
twins, at every slot. Fix shape (from review,
adopted): `cleaned_weekday`'s shed takes the bare
path's standing scope — a run of interior marks
and a second interior mark, under the same
weekday-vocabulary anchor (the anchor is what
keeps the governor-completion boundary: no
governor word is completed by this shed).
**Interior-`.` arm (MINOR, RT40-J):** `(Mon.day)`
serves at the leading and trailing slots while the
bare twins' lines die silently (for = None, no
record on either side; base-trailing at parity).
Disposition per adjudication 6. Disposition:
R40-F.

### F8 (MAJOR) — A trailing suffix outside the standing suffix sets disarms the judgment it follows: a trailing wrapped governor serves its cancelled date, grouped weekday sequences serve, and the bare weekday loses its record

Red team RT40-G. The suffix (U+2026 `…`, and at
sequence tails also `~`, `*`, `#`, `-`, `—`) stands
in no reader's peel set, and it disarms every
judgment it touches:
- Trailing governor: `November 8, 2026
  (cancelled)…` serves 2026-11-08 live; the bare
  twin `cancelled…` governs (for = None). Post
  R39-H the trailing wrapped governor governs as
  its bare twin — the suffix alone resurrects the
  cancelled date.
- Grouped weekday: `November 1, 2026 (Monday)…`,
  `(Monday)(notes)…` at the leading slot, and
  `Sunday after November 1, 2026 (Monday)…`
  (→ 2026-11-08) all serve silently; the lone and
  base-trailing arms are new past the round-39
  record, which carried only the trailing-sequence
  arm.
- Bare weekday: `November 1, 2026 Monday…` and
  `Monday… November 1, 2026` read None with **no
  record**, where the clean bare forms refuse with
  WeekdayMismatch(Monday@2026-11-01); at the
  base-trailing slot the bare form serves instead.
- Closed-list support: `(Monday)(notes)` wearing
  `~ * # - —` at the leading slot serves silently,
  while `..`, `!!`, `??`, `!?`, `:;` judge with the
  record — the split is exactly membership in the
  standing suffix sets.
Review's narrower ellipsis family (lone weekday
groups, lone and sequence alike) held at parity
with the round-39 record; the divergence stands
on the governor and record arms, which review did
not witness. Reading per adjudication 5.
Disposition: R40-G.

### F9 (MAJOR, doctrine) — Governor-first + weekday-companion at the leading slot: the pinned parity serves a date both contents independently condemn; the company's edges split on marks and order

Review target (a), graded doctrine MAJOR and
adjudicated IN per adjudication 3; verify's
confirmation stands as the parity record; red
team RT40-D / RT40-E are this finding's arms.
The pinned cell: `(cancelled)(Monday) November 1,
2026` and every twin serve 2026-11-01 silently
(leading, all pairs, all separator widths, both
routes) — while `(cancelled)` alone governs and
`(Monday)` alone refuses with the record. The
mechanism is pass order: `unwrapped_leading_
governor_group` rewrites the line to `cancelled
(Monday) November 1, 2026` before the sequence
walk runs, and in the rewritten line the weekday
group is mid-line behind a bare word, the one
position (a)(i) leaves unjudged.
- **Arm (d) (RT40-D):** a sentence mark after the
  weekday group flips the pinned serve to silent
  None for exactly half the mark set:
  `(cancelled) (Monday)! / ? / ; / : November 1,
  2026` → None, no record, both routes;
  `(Monday)” / ’ / . / ,` serve field-identical to
  the pinned cell. The lone twin `(Monday)!
  November 8, 2026` refuses *with* the record, so
  the marks transfer neither the pinned outcome
  nor the lone judgment.
- **Arm (e) (RT40-E):** the company is pinned only
  at exactly two groups. `(cancelled)(Monday)
  (notes) November 1, 2026` → None, no record, in
  every dress; `(cancelled)(notes)(Monday) …`
  serves (the pinned outcome); `(Monday)
  (cancelled) November 1, 2026` and `(Monday)
  (cancelled)(notes) …` → None, no record, where
  lone `(Monday)` refuses with the record. The
  trailing and base-trailing slots do not split
  this way (trailing uniform governed-None,
  base-trailing refuses with the record, all
  orders — held lines); the discontinuity is the
  leading slot's own.
Under the adjudicated reading the batch judges
the weekday across the whole leading company —
all orders, all company sizes, marks after any
group — so arms (d) and (e) resolve by
construction: marked and reordered forms take
the composed outcome (governed, with the record
where the weekday disagrees). Disposition: R40-E.

### F10 (MINOR, unwitnessed) — The lock's displacement step deletes whatever stands at the displaced path without re-judging it, in a window the claim loop's displaced judgment does not cover

Review R40-R6, code-read only (the lock functions
are private to the ob-app binary crate and cannot
be driven from a scratch probe). `rename_instance_
lock_to_displaced_from` (main.rs:3579ff)
unconditionally `remove_file`s the displaced path
before its rename lands; its comment scopes the
deletion to "a previous displaced copy". The claim
loop judges the displaced path at the top of each
pass (`judge_displaced_instance_lock_at_claim_
from`), but the displacement that deletes happens
later, in the `AlreadyExists` branch, after a lost
create race and a re-judgment at the *lock* path
only. Between the two, a third claimant's refused
audit can leave a *live* generation standing at
the displaced path (the audit's own documented
outcome when the lock path is occupied,
main.rs:3627ff). If that interleaving lands, a
live peer's lock generation is destroyed, and
with it the displaced-path hold `retire_journal_
from` computes from it — the stranding class this
machinery exists to prevent, reachable only in a
three-claimant race with a refused audit inside
the window. Red team's lock sweep this round
(N16–N22, incl. third-claimant shapes through the
verbatim extraction) held every shape it drove;
whether its extraction reproduces this intra-pass
window is not established either way — the
observation stands beside that sweep, not against
it. Suggested shape (from the lane): `displace_
instance_lock_from` re-judges the displaced path
immediately before the rename step and stands
down (re-enters the judge loop) when a live
generation stands there. Disposition: R40-H, per
adjudication 7. The standing recycled-pid residual
is unchanged in scope and stands as recorded.

## Records (not findings)

- **Peeled-tail extensions** (red team): `November
  1, 2026 (Monday) !!`, `(Monday) ! ?`, `(Monday)
  ) !`, `(Monday)]!` serve with no weekday record,
  extending the R38-F peeled-tail residual (tails
  needing more than the one peel serve with no
  record; its cure is a peel-depth change). The
  recorded arms re-witness. Not re-graded; the
  residual stands as recorded.
- **Interior-mark governor groups** (both lanes):
  `(cancell!ed)` / `(cancel!led) November 8, 2026`
  serve; bare twins read nothing — the standing
  round-39 record (no governor word is completed
  by shedding). Re-witnessed, not re-graded.
- **Split-position governor uniformity** (red
  team): `Sunday (cancelled) after November 1,
  2026` reads nothing in *every* dress on this
  tree — a uniform movement from the R38-C
  record's garnish neighbour, with no twin
  divergence to grade. Noted, as R39-B's scope
  note did for the after-`after` forms.
- **Lock-file pid spelling** (red team): a lock
  file whose pid stands on the second line
  (leading newline) is claimed over even while
  that pid lives; the spelling is not one
  `instance_lock_text` ever writes — its first
  line is empty, so under the standing decision
  table it names no holder, exactly as the
  garbage-content shapes do (N11 / N17,
  table-conformant in prior rounds). Boundary
  observation, not a finding.
- **Rights widened permutation variance** (red
  team): the fresh 20,000-quadruple sweep's
  per-axis counts (children = 0, permission-noted
  = 134, credit-presence = 753) replicate the R38
  adjudication-7 widened-definition variance at
  its recorded rates; the standing M8 definition
  holds at 0. Not re-filed, per that adjudication.
- **Absolute-time budgets outside the cliff-pin
  family** (verify observation, ungraded):
  ob-format tests outside the named family use
  `Duration::from_secs` elapsed budgets; they are
  outside the R39-CI conversion's stated scope and
  passed in the settled suite under load. Recorded
  so the scope line is explicit; any future
  conversion is a separate disposition.
- **Process records** (verify + review): the
  runtime's background completion notices again
  reported exit 0 while processes were still
  running (verify, twice; review's probe builds
  were phantom-reported complete while alive, the
  duplicate killed by `ps`). No evidence was taken
  from any unverified run in either lane. The
  standing rule — disk logs with embedded exit
  codes only — is re-confirmed, not new.

## Batch plan

One writer at a time; every batch under the
batch-boundary discipline (commit + push +
HEAD == origin confirmed, `cargo clean`, cold
verification replacing incremental; per-batch
records in `round40-out/coord/` in the standing
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

- **R40-A** — F3 + F4 (archive candidate
  discipline, both arms): the walked extent
  accounts for the archive's tail (walk end
  through the candidate's own end record, zip64
  structural records and their gaps included);
  the F2-class derivation repair for hosts whose
  tail gaps make them underivable while the
  reader serves them; and the equal-set rule —
  a candidate standing in another's walked
  extent whose records differ in any judged fact
  from the extent owner's records for the same
  offsets is subordinated. Pins: RT40-A's five
  regions × four doors with clean and duplicate
  controls; R40-R5's rename / phantom / trunc
  crafts with the clean control and both
  predecessor subset plants unweakened; every
  standing door/zip pin unweakened.
- **R40-B** — F1 (the letterless cliff): the
  letterless shape gets the bounded-scan
  treatment R39-G gave the lettered shapes, at
  the passes both lanes named. Pins: the `()`×n
  ladder in interleaved ratio form (witness vs
  byte-matched control in the same run, bound
  set between the fixed code's measured ratio
  and the un-fixed code's, per the R33-D form);
  readings stay None at every size; the R39-G
  shapes' pins unweakened.
- **R40-C** — F2 (nested + plain sequence
  walks): the per-group-start walks in the
  spaced/glued and sequence readers are bounded
  the same way, across the nested_day /
  nested_month / nested_whole / nested-trailing
  shapes and the plain trail / base-trail runs.
  Pins: each shape's ladder in interleaved ratio
  form; readings preserved at every size stated
  outright; the R39-G fixed shapes unweakened.
- **R40-D** — F5 + F6 (Sunday-after interior
  slots): governor contents between `after` and
  the base judged as governors in every dress
  (adjudication 4), inert groups unmoved; the
  guard's group notion extends to fully
  quote-wrapped date groups at the positions it
  covers (F6). Pins: the R40-R1 family (bare /
  wrapped / impure / framing / weekday-bearing
  bases, both routes), RT40-C's glued arms,
  RT40-B's four positions × three quote forms ×
  both routes against their bracket twins, the
  `(notes)` and quoted-weekday/governor parity
  controls unweakened.
- **R40-E** — F9 (doctrine, arms (d)+(e)): the
  leading sequence walk judges its weekday
  groups independently of the governor unwrap,
  per adjudication 3. The two named pin arms are
  restated outright, flagged in the commit and
  record. Pins: the full company — governor-first
  and weekday-first orders, two- and three-group
  companies, companions in every position, marks
  after any group (arm (d)'s split set closed),
  all pairs and separator widths, both routes,
  full records against the composed reading
  stated outright; the (a)(i) plain mid-line
  controls (`notes (Monday) …` in running text)
  stated unjudged and unchanged.
- **R40-F** — F7 (grouped weekday interior
  marks): `cleaned_weekday`'s shed takes the
  bare path's scope (runs + second mark,
  vocabulary-anchored); the interior-`.` arm per
  adjudication 6. Pins: the two-mark family at
  all three slots × both routes against the bare
  twins, single-mark controls unweakened, the
  governor-completion boundary held (`(cancell!
  ed)` stands as recorded); the `.` family
  against the abbreviation controls, as the
  batch's ruling states.
- **R40-G** — F8 (out-of-set trailing suffixes):
  judgments read through the witnessed suffixes
  at the named slots, per adjudication 5. Pins:
  the governor arm, the grouped-weekday arms
  (trailing / leading-sequence / base-trailing),
  the bare-weekday record arms, the sequence-tail
  closed-list family (`~ * # - —` judge;
  `.. !! ?? !? :;` controls unweakened), both
  routes.
- **R40-H** — F10 (lock displacement window):
  witness attempt through the in-tree seams
  first (adjudication 7); on a witness, the fix
  re-judges the displaced path immediately
  before the rename and stands down to the judge
  loop when a live generation stands there; the
  standing lock/retirement pins (incl. the
  R33-A forced interleavings) unweakened. On no
  witness, the batch is the attempt's record and
  the observation carries to round 41 as a
  record.

Then the coordinator close: five-leg evidence on
the final tree (local cold, fuzz as claims,
hosted CI ×3, desktop runner build, Windows
bundle), the build-cache policy disposition at
the confirmed completion (df reading + window +
latch state over the round's recorded readings),
and boundary hygiene. HF-12 untouched. The exit
bar is judged on round 41's audit of the closed
tree; the Tier-2 final gate is not triggered
(1/8/1) and is not the coordinator's to start in
any case.

## Coordinator close (2026-10-25)

All twelve findings fixed or dispositioned in eight
batches, one writer at a time, each batch committed +
pushed + HEAD == origin confirmed, then `cargo clean`
and cold verification replacing incremental, each
batch record stating its hosted-CI conclusions.
Final code tree: `406ded0` (R40-H). Suite: **1156
passed / 0 failed** (1131 at the audited tree + 25
new pins). Records: `round40-out/coord/r40{a..h}-
record.md`.

| Batch | Finding(s) | Tip | Suite | Hosted CI (all 3 OS legs) |
|---|---|---|---|---|
| R40-A | F3 + F4 (archive, both arms) | `98d4f3a` | 1135/0 | 40161570450 success ×3 |
| R40-B | F1 (BLOCKER, `()` cliff) | `69c0405` | 1136/0 | 40168086542 success ×3 |
| R40-C | F2 (nested + sequence walks) | `51268d8` | 1142/0 | 40177137616 success ×3 |
| R40-D | F5 + F6 (Sunday-after slots) | `b71be06` | 1145/0 | 40185334796 success ×3 |
| R40-E | F9 (doctrine, arms d+e) | `9d2da1b` | 1148/0 | 40191886932 success ×3 |
| R40-F | F7 (interior marks; `.` arm stopped) | `5fa4d79` | 1151/0 | 40198134505 success ×3 |
| R40-G | F8 (out-of-set suffixes) | `2c2c2bc` | 1155/0 | 40204790291 success ×3 |
| R40-H | F10 (lock window, witnessed) | `406ded0` | 1156/0 | 40211842907 success ×3 |

Desktop Runner Build green on every batch tip
(40161570405 / 40168086528 / 40177137670 /
40185334783 / 40191886904 / 40198134552 /
40204790369 / 40211842919).

**Five-leg evidence on the final tree `406ded0`:**
(1) local cold — suite 1156/0 (SUITE_COLD_EXIT=0),
clippy CLIPPY_COLD_EXIT=0, fmt FMT_COLD_EXIT=0
(`round40-out/coord/h/*-cold-recovery.log`); booklet
pin passes in-suite and on a direct run on the
final tree (BOOKLET_EXIT=0), 31,717 imposed /
34,977 reader byte-exact. (2) Fuzz as claims —
verdict 40,000/0 md5 `79775844…`, composition
497,024/0 md5 `5868726a…`, rights 5083 / 1024 /
5275, safety_flips=0, byte-identical at every batch
and on the final tree; M8 held for the twentieth
consecutive round in the audit and unbroken through
the fixes. (3) Hosted CI run 40211842907 — ubuntu /
windows / macos all success. (4) Desktop Runner
Build run 40211842919 success. (5) Windows bundle
run 40216697724 — Gate + bundle both success.

**Dispositions of record inside the batches.**
R40-A's arms (4)/(5) needed one step beyond this
synthesis's literal text: `prefix_free` splices a
judged archive's non-contiguous tail into a
contiguous owned view (the doors open the reader on
that view before judging, and the reader refuses the
clean locator-side hosts on the unspliced bytes);
standing `prefix_free` pins pass unchanged and the
outcome is the adjudicated one — accepted by the
coordinator and named here for round 41's lanes.
R40-D stopped once, correctly, on two standing
control arms encoding the old after-"after" reading;
the coordinator ruled the restatements in
`round40-out/coord/r40d-ruling.md` (the synthesis's
R40-D entry should have named them, as the R40-E
entry named its two — the omission was the
coordinator's) and the batch re-ran under the
ruling. R40-E's two authorized restatements landed;
its mid-line form (`notes (cancelled)(Monday)
November 1, 2026`, serving silently under R39-H's
standing arm) is a different reader's territory and
is pinned unmoved — carried below. R40-F landed the
two-mark shed as ONE shared statement
(`interior_mark_shed`) so the bare and grouped
paths cannot drift again; one r39e boundary-array
arm that encoded the defect itself was removed and
flagged in commit + test + record. **R40-F's `.`
arm did not land**: per adjudication 6 the writer
witnessed the family whole (the abbreviation forms
refuse with the record in both dresses; the
`(Mon.day)` split is the generic inert-contents
split) and found every candidate ruling moves
standing pins; the standing readings are pinned as
they stand and the full witness table is in
`r40f-record.md` — F7's `.` arm is dispositioned as
a record, carried below, and F7 itself (the
record-carrying two-mark arm) is fixed. R40-G held
its scope: unwitnessed trailing `~` forms still
serve, recorded as an observation in its record.
R40-H witnessed its interleaving (the pin fails on
the base tree) and fixed it on the witness, per
adjudication 7.

**Incidents.** Four runtime/VM restarts touched
this round (the verify lane's suite run 2, R40-D
mid-batch, and R40-H twice — the batch's first
writer died mid-verification and its recovery
writer died after the fuzz outputs were written,
before writing the record; the coordinator
completed `r40h-record.md` from the disk logs and
checked its CI via `gh`). Every recovery verified
death before touching state (process scan + mtime)
and judged inherited work from disk before building
on it; nothing was committed on an unverified
claim. One load failure, dispositioned as load:
the r40b ratio pin missed its bound at 22× inside a
980 s saturated ob-extract leg during R40-H's first
run, passed solo at its ~9× standing and inside
both recovery suites; no bound was widened. The
R40-C writer received a brief naming witness shapes
(`((Date))`×n, `(Dec 25)`×n) that appear in no filed
report, with function names that do not exist in
the tree and canary numbers off measurement by up
to 18×; it fixed the filed F2 family instead and
flagged the brief in its record — the discrepancy
stands in `r40c-record.md` and is recorded here so
the round's provenance is complete. Phantom
completion notices recurred in every lane and both
recoveries; no verdict in this round rests on one.

**Carried into round 41 (named targets):**
(a) the mid-line governor company — `notes
(cancelled)(Monday) November 1, 2026` serves
silently (governor lapses, R39-H's standing arm;
the same shape as `(today cancelled)(Monday) …`)
— whether the R40-E composed judgment extends
behind an ordinary leading word is round 41's
doctrine question, with the company witnessed by
both R40-E and the round-40 review lane;
(b) F7's `.` arm as a record with R40-F's witness
table (`(Mon.day)` grouped-serves / bare-dies, the
abbreviation family refusing with the record named
"Mon" in both dresses) — needs a coordinator
ruling on interior `.` (sentence separator vs
shed mark) before any batch;
(c) R40-D's two recorded neighbours: a framing
word standing between "after" and a quoted date
group (`Sunday after the "November 8, 2026" …`
still adopts where its paren twin serves with the
record — unwitnessed by RT40-B, not guard-covered
for brackets), and the leading-alone quoted date
whose contribution in-words memories keep the
quoted dress;
(d) the R38-F peel residual with its round-40
extensions (`(Monday) !!`, `(Monday) ! ?`,
`(Monday) ) !`, `(Monday)]!`), the split-position
governor uniformity note, the lock leading-newline
pid spelling, and the recycled-pid residual — all
standing as recorded;
(e) R40-G's scope observation (unwitnessed trailing
`~` forms still serve) and R40-C's sequence-pin
calibration note (the two sequence pins ship as
steepness ceilings at 16×/20× because in-suite
parallel contention inflates their ratios ~1.7×;
their solo-frame bite evidence is in
`r40c-record.md`).

**Build-cache disposition at this confirmed
completion** (charter policy): final reading Sun
2026-10-25 11:50 PDT (daytime window): **79% used /
22G free**. The round's completion readings ran
74–79%; no confirmed completion in the round read
at/above the overnight 87% or daytime 94% line, and
no over-limit reading latched at any point.
Disposition: **CARRY** — the target dir carries
into round 41. Boundary hygiene: batch scratch
under `round40-out/coord/{b,c,e,f,g,h}` and the
lane scratch stand as the round's evidence;
registry untouched; no probe dirs created in the
repo tree this round.

**Exit bar: NOT met by round 40's audit**
(1 BLOCKER / 8 MAJOR / 1 MINOR), so the bar is
judged on round 41's audit of this closed tree.
The Tier-2 Opus + GPT final gate is not triggered
and is not the coordinator's to start in any case.
HF-12 untouched.
