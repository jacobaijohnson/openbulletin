# Audit round 39 — synthesis (coordinator, 2026-10-23)

Audited tree: `3f70cfe` (round-38 close record on code
tree `3ce5e0a`). Three blind lanes, read-only:
verify (`round39-verify.md`), review
(`round39-review.md`), red team
(`round39-redteam.md`), all in
`~/workspace/openbulletin-audit-wave3/`, scratch in
`round39-out/`. (The first review dispatch died in a
runtime restart with only a probe skeleton built; the
lane was re-dispatched and its report stands on its
own witnessed evidence.)

Lane verdicts: verify PASS on all six claims
(0/0/0 handed on), review 0/5/3, red team 0/5/3.
Deduped: **0 BLOCKER / 8 MAJOR / 2 MINOR** — the
exit bar is NOT met. Majors trend:
10 → 6 → 6 → 1 → 4 → 3 → 2 → 5 → 6 → 8 → 6 → 5 → 8.

The verify lane's standing claims all held on the
audited tree: suite 1094/0 (run 6; runs 1/2/5 killed
by VM reboots and runs 3/4's wall-clock cliff-pin
failures under load 9–14 are recorded in the lane
report and in the records below — every load-failed
pin passed solo quiet and passed in-run in run 6),
clippy 0 on a genuinely re-checked tree, booklet
31,717 / 34,977 byte-exact, fuzz bins at their
recorded md5s with stdouts byte-identical to the
standing baselines (verdict `79775844…`, comp
`5868726a…` — the R38-C re-baseline confirmed exact,
no diff to report), rights counters 5083 / 1024 /
5275 with safety_flips=0 (M8 held; the streak count
is recorded under adjudication 5), and all 16
round-38 pins in the spot-audit set present in-tree
and passing. Every finding below comes from the
carried doctrine question and the lanes' own
hunting, not from a failed standing claim.

## Adjudications recorded before the findings

1. **Merges.** Review R39-R2 and red team RT39-1's
   spanless-entry arm are one defect (the
   local-header span gated on `payload_span`
   succeeding); RT39-1's gap and pre-first-header
   regions are the same walked-extent defect at
   two further byte regions — one finding (F1),
   on the F6/F8/F1 merge precedent. Review R39-R1
   and RT39-3 are one defect — the sequence walks'
   (and the R38-D pass's) admitted separator/suffix
   sets are narrower than the lone forms' at the
   boundary positions past R38-B's completions —
   one finding (F4). Review R39-R5/R6 and RT39-4
   are the month–day chain family, split by
   content per adjudication 5 of round 38 (F5
   governing / F6 inert, one batch). Review
   R39-R3/R4 and RT39-5 are one guard-coverage
   defect at the Sunday-after phrase's interior
   slots (F3); review's MINOR grade on the
   after-weekday-word arm is recorded in F3 and
   adjudicated there. Review R39-R8, RT39-6, and
   RT39-7 are the bare-path parity tail, one
   finding (F8) on the R38-F one-batch precedent.
2. **RT39-8 (performance) is adjudicated IN as
   MAJOR over the lane's MINOR.** Grounds: the
   evidence is the standing kind — per-step ratios
   (36× and 13.8× per 4× on the group-run shape,
   42× per 4× on mixed unclosed groups, 21×/22×
   per 4× on nested day groups) against a control
   measured linear in the same windows, the same
   ratio evidence R32-C and R35-E were graded
   MAJOR on; the sizes are sender-reachable (a
   15 KB pasted line); and one shape lives in
   R38-F's own new code. The lane's caution is
   recorded and priced into the disposition: the
   measurements are debug-frame under cross-lane
   load 8.7–11.4, so the batch (R39-G) opens with
   a quiet-window re-measurement witness on the
   pre-fix tree and proceeds only on confirmation;
   no bound is widened and no fix is built on the
   loaded numbers alone.
3. **The doctrine question is adjudicated IN as a
   doctrine change (F10), scoped: the trailing
   slot and the plain mid-line form. The
   base-trailing slot needs no change.** All three
   lanes' uniformity evidence agrees on the facts:
   the trailing split is total in clean dress
   (verify: systematic across pairs, glued,
   doubled, mixed, marked, quoted, impure contents,
   companion sequences in both orders, with parity
   pockets only where the dress stops being a
   balanced segment — crossed pairs govern; review:
   61 lines, both routes; red team: uniform within
   dress, mirrored on both routes, no keep-through
   resurrection anywhere); the mid-line split is
   total for spaced clean dress before a plain
   date; and the base-trailing slot carries **no
   split on this tree** (see adjudication 4).
   Positions: review argues EXTEND; verify argues
   HOLD (a standing trailing-aside doctrine);
   the red team hands on evidence, including the
   round's decisive new fact — the split is
   outcome-active inside judged sequences: beside
   a disagreeing weekday group the wrapped
   governor leaves the sender-correcting record
   standing (`November 1, 2026 (cancelled)(Monday)`
   refuses with the record, as the lone weekday
   form does) while the bare governor in the same
   company deletes it entirely (`November 1, 2026
   cancelled (Monday)` is totally silent); beside
   a date group the two dresses serve *different
   primary dates* (`(cancelled)(November 8, 2026)`
   serves Nov 1 with the group as garnish; bare
   `cancelled (November 8, 2026)` serves Nov 8 with
   Nov 1 demoted). Adjudicated for EXTEND, on
   these grounds:
   (a) The configuration is adjudication 4 of
   round 38 exactly, one slot over: the trailing
   slot *is* a judged slot for group contents —
   weekday sequences judge there (R37-D), and at
   the base-trailing slot a trailing weekday group
   is judged against the base — with the governor
   as the one exempt contents class, and the
   exemption resting on dress: the quote-wrapped
   bare governor governs at the trailing slot in
   every quote form (all three lanes), so the
   current line is not "governors are asides
   after a date" but "governors are asides after
   a date only in brackets". A doctrine that
   holds only in one wrapper is the dress split
   itself, not a doctrine about position; the
   hold position, adopted, would canonize dress,
   not position.
   (b) The red team's sequence evidence meets the
   substance of the hold position's own stated
   change-condition (a bare governor governing
   while the wrapped form escapes *inside a
   judged companion sequence*, with outcome at
   stake): inside judged trailing sequences the
   governor's dress decides whether the Check
   surface keeps its weekday record and which
   date is primary. The harm is witnessed, in
   the standing wrong-record / wrong-date
   classes.
   (c) The mid-line form follows from how the
   bare governor's judgment works: it is a
   segment judgment, not a slot judgment — the
   bare stamp governs anywhere inside the date's
   segment, and the wrapped form's whole
   exemption is one ordinary word standing
   before it. Extension at mid-line is parity
   with that segment reading, scoped to where
   the bare stamp itself governs: before a
   plain date in-segment. Where the bare stamp
   lapses, parity already holds and nothing
   moves — before phrases in both dresses
   (verify T5, red team), beside a weekday group
   mid-line (red team), and after a Sunday-after
   base in every dress (adjudication 4 below).
   (d) The counterweight is weighed as round 38
   weighed it: a trailing group is the
   conventional aside position, and "(cancelled)"
   after a named date can annotate something
   other than the date's event. Adjudication 4
   resolved the identical ambiguity at the
   leading slot by the bare stamp's own
   treatment; the same resolution is adopted
   here, and the sender who writes the stamp
   bare and the sender who writes it bracketed
   are read the same way.
   (e) Cell (a)(i) is NOT re-opened: review's
   separateness argument is adopted (the
   re-opening condition's text is weekday-cell-
   specific; the weekday cell closed on
   uniformity and this round's probes re-confirm
   mid-line weekday groups uniform-unjudged in
   both dresses; the governor question traveled
   the (a)(ii) road by the R38-D ruling's own
   carriage). Mid-line weekday-group judgment
   remains framing-slot only; what changes is
   governor *contents* in a mid-line group,
   judged as the bare stamp in the segment is
   judged. The doctrine statement the batch
   carries says exactly this.
   (f) The hold position's remaining argument —
   that extension restates five pin families
   adjudicated on their own evidence — is priced
   in F10's disposition: the restatements are
   named, bounded, and flagged, exactly as
   R38-D's were; what those families adjudicated
   in their own rounds was parity *among
   wrappers*, and no round before this one put
   bracket-vs-bare at the trailing slot to a
   ruling with the sequence evidence in front of
   it.
4. **The carried base-trailing premise is
   corrected on the evidence.** The R38-D writer
   note's probe (`Sunday after November 1, 2026
   cancelled` → for=None) does not reproduce on
   the audited tree: all three lanes find the
   bare governor garnish at the base-trailing
   slot in every dress (red team re-probed at
   four todays), field-identical to the wrapped
   forms, and no pin states the bare-governs
   reading. Verify's provenance note stands:
   the probe ran on pre-fix `b741fca`, and one
   of R38-D/E/F moved the bare reading without
   any batch record claiming the move (the
   reading's history is not reconstructible from
   the record — review). Recorded as an evidence
   correction to the round-38 record. The R38-D
   ruling itself is unaffected: its scope
   decision rested on the leading adjudication,
   and its construction term (the fix must not
   move the trailing/base-trailing readings) is
   what the record shows. The base-trailing
   doctrine question *dissolves*: garnish in
   every dress is the slot's standing reading,
   the R37-G garnish pin stands unweakened, and
   whether garnish is the *right* reading for a
   cancellation after a Sunday-after base is a
   question no lane put this round; it is not
   opened here.
5. **M8 streak count.** Verify counts M8 held
   for the nineteenth consecutive round; the red
   team's sweep says twentieth by its count. The
   facts both lanes stand on are identical:
   counters 5083 / 1024 / 5275 from the standing
   bins, safety_flips=0, and 0 standing flips in
   100,000 fresh quadruples with round 38's
   inherited counters replicated to the unit
   (the red team's one counted difference — its
   license-selection variance +24/+19 — is a
   counting-definition difference, fold-vs-chain,
   stated precisely in its report; the graded
   definition replicates exactly). The one-round
   streak difference is a convention artifact
   (audit rounds vs closes); recorded, not
   resolved, and nothing grades on it.
6. **A round-38 record claim is corrected by
   reference.** The R38-F batch record and the
   round-38 close section state that four
   hard-coded counts inside r37b/r37d pins moved
   7 → 6 in R38-F. Review's check (record 3c):
   the R38-F diff touches tests.rs purely
   additively (494 insertions, 0 deletions) and
   the cited writer note contains no count
   update. The claim is unborne by the diff; if
   counts moved, they moved in an uncommitted
   draft. No behavior stake — the readings are
   pinned and pass. Under standing verdict
   ruling 3 (a stale record claim with correct
   code is a doc fix, not a defect), the
   round-38 close section's sentence stands
   corrected here: no counts moved in R38-F.

## Findings

### F1 (MAJOR) — Archive candidate discipline, third generation: a planted directory standing in the unused gap before the central directory, in the region before the first local header, or in the local header of an entry whose payload span does not resolve, steers the doors

Review R39-R2 + red team RT39-1, merged per
adjudication 1. R38-A stated the walked extent as
"its entries' local headers whole (fixed part,
name, and extra field) together with their
payloads" (`directory_walked_spans`,
zip_directory.rs), but in the code the local-header
span `(local, data_start)` is pushed only inside
`if let Some((data_start, data_end)) =
payload_span(...)` — an entry whose declared
compressed size overruns the file (or whose local
signature or name/extra lengths fail) contributes
*no* span at all, its header included. And two byte
regions no candidate's walk accounts for at all:
the unused gap between the last entry's payload end
and the central directory's first record, and the
region between the archive's own start and its
first local header. A complete fake CD + EOCD
planted contiguously in any of the three derives,
stands earliest (its end record stands in no span,
comment, or absorption of the real candidate), and
is judged, while the reader serves the real archive
whose disagreeing duplicate the doors must refuse.
Witnesses (crafts in both lanes' scratch): the
broken-filler local-extra plant (central record
declares 0xFFFFFFF0; the scan judges total=1 at the
planted directory, the second copy under its own
record, the duplicate and the filler invisible —
all four doors steered); the gap plant, plain and
zip64 hosts (IDML serves "Secondcopy story words
here", .mxl loads SecondScore, fonts land only the
Regular face, clips land 1 picture); the
pre-first-header plant (IDML, fonts, clips steered;
the music door refuses on its own payload check
while scanning under the fake directory). Clean
controls at every site refuse / land both copies
exactly as pinned. One candidate-discipline defect,
three new plant regions — the F6/F8/F1 precedent.
Fix shape (adopted): the walked extent accounts
for every byte between a standing candidate's
archive start and the end of its directory walk —
gaps between accounted spans included — and an
entry's local header stands in the extent computed
from the local header's own fields, whether or not
its payload span resolves. Disposition: R39-A.

### F2 (MAJOR) — A zip64 archive whose signature record is not counted in the declared directory size is underivable at all, and a directory planted in that signature record is the sole standing candidate and steers all four doors

Red team RT39-2. The round-38 sector-archive
variants were not re-attacked in round 38;
attacking them found the split one level down. On a
plain host the signature record after the directory
is handled (the hunt's frame anchor carries the
signature allowance; the R38 N4 plant is
subordinated by the walked signature span —
re-witnessed held at all four doors). On a zip64
host whose signature record's bytes are *not*
counted in the zip64 record's declared directory
size, both derivations fail on an equality:
`validated_zip64_record` and
`arithmetic_directory` each require
`cd_offset + cd_size` to equal the record's
position, and the uncounted signature record
standing between the directory and the zip64
record breaks the equality. The clean craft
derives None at the scan while the reader serves
it and every door refuses ("its central directory
cannot be derived" — the standing safe-direction
posture, the round-34 / N7d record, on its own).
But a complete fake CD + EOCD planted inside that
signature record's data is then the *only*
standing candidate: it is judged at all four
doors, sector and non-sector zip64 alike — IDML
serves Secondcopy, .mxl loads SecondScore, fonts
land only Regular, clips land 1 picture. The
counted variant derives the real directory and
refuses the duplicates at all four doors; the
sector variants of the other three R38 sites and
the extensible-data-sector plant are held on
derivable hosts. The hole is the
underivable-host-plus-plant combination. Fix
shape (from the lane, adopted): a zip64 candidate
whose record validation fails *only* by bytes a
signature record standing at its directory's walk
end accounts for derives under the signature
allowance the plain frame already carries (the
signature record's own declared length closes the
gap, counted or not), so the real candidate
stands and the walked-span discipline subordinates
the plant as on plain hosts. Disposition: R39-A,
with F1.

### F3 (MAJOR) — R38-C's interior-slot guard splits the pairs at four further positions: a framing word after "after", a group between "after" and the base, a split group glued to "Sunday", a weekday group at the split position, and non-Sunday weekday words at the guarded slots

Review R39-R3/R4 + red team RT39-5, merged per
adjudication 1. R38-C consulted
`group_beside_relative_core` /
`group_split_sunday_after_phrase` at the Sunday
split position (gated on the words after `after`
being *exactly* the base) and after a *Sunday*
word before a relative phrase. Past those gates,
both routes:

- **Framing word after `after`:** `Sunday
  [November 8, 2026] after the November 1, 2026`
  → the bracket group is *adopted* (for = Nov 8,
  base demoted to a second-date note); the paren
  twin serves the base with a
  ParenthesizedDateMismatch record. The R38-C
  adoption itself, one framing word further in.
- **Group between `after` and the base** (the
  position R38-C's record lists as deliberately
  not reached): `Sunday after [November 8, 2026]
  November 1, 2026` → the bracket group becomes
  the base, for = **2026-11-15** — a resolution
  computed from a date the sender never wrote as
  the base, the written base demoted to a
  second-date note, no mismatch record. The paren
  twin serves Nov 1 with the mismatch record.
- **Glued to the phrase's first word:**
  `Sunday[November 8, 2026] after November 1,
  2026` adopts the group as the line's date with
  no record, and the paren twin is field-identical
  — both pairs adopt where the spaced twins read
  nothing in every pair. Pair-uniform but
  spacing-divergent: glued forms take their
  spaced readings everywhere else in this program
  (R37-C chains, R38-E).
- **Weekday group at the split position:**
  `Sunday [Monday] after November 1, 2026` →
  bracket totally silent; the paren twin serves
  for = 2026-10-25 with the base as second date.
- **Non-Sunday weekday words at the guarded
  slots:** `Monday [November 8, 2026] after
  November 1, 2026` (and Tuesday/Wednesday) →
  bracket refuses with WeekdayMismatch against
  the group; the paren twin serves the base with
  the mismatch record. After a weekday word:
  `Monday [November 8, 2026] next Sunday` →
  bracket refuses with the manufactured record;
  the paren twin serves the group's date as
  primary with the phrase demoted. Review graded
  this arm MINOR (no wrong date served on the
  bracket side; the refusal and its record are
  the harm); the dissent is recorded. Adjudicated
  into the family's grade: the arm is the same
  Sunday-only gate defect, and its paren side
  *adopts* the group as the line's date — the F3
  outcome class — so the dress decides between
  adoption and a manufactured refusal on
  identical contents, which is the defect the
  guard exists to close.

Fix shape (adopted): the guard's judgment extends
to a group standing between `after` and the base
(judged as its paren twin is: base read, group
recorded), to a framing word standing after
`after` before the base, to the split detection
seeing a group glued against the phrase's
"Sunday", to a weekday group at the split
position, and the after-weekday-word arm takes
any weekday-word prefix under the Sunday arm's
reading. Scope note for the batch: verify
recorded a governor-contents neighbour at the
between-`after`-and-base position (bare
`cancelled` there kills the phrase; the wrapped
group serves the base) as context, not graded;
the batch states in its record whether its fix
moves those forms — movement is reported, not
judged, and adjudicating them is a later round's
act. Disposition: R39-B.

### F4 (MAJOR) — The sequence walks' and the governor pass's boundary sets are still narrower than the lone forms': a separator at a walk's head, a separator run between groups, a `!` `?` or U+201D standing between groups, and a stacked suffix each disarm the judgment

Review R39-R1 + red team RT39-3, merged per
adjudication 1. R38-B completed the walks'
separator/suffix sets *between* groups and at
their tails, and R38-D's governor pass took the
same separation set. The lone forms judge further:
a weekday group wearing `!`, `?`, or U+201D as its
suffix refuses in every slot. Past the completed
positions, both routes:

- **Between groups (walks):** `(Monday)! (notes)
  November 1, 2026`, `(Monday)? …`, `(Monday)”
  …` serve silently at the leading slot; the
  same at the trailing slot and at the
  base-trailing slot (`Sunday after November 1,
  2026 (Monday)! (notes)` → Nov 8 silent). The
  same marks after the *last* leading group
  disarm it too. U+2019 in the same positions
  judges, as R38-B pinned — the set is exactly
  three marks short.
- **Governor pass arm:** `unwrapped_leading_
  governor_group` skips the same narrower set
  between the leading run's groups, so a marked
  companion shields a following governor:
  `(notes)” (cancelled) November 8, 2026` and
  `(notes)! (cancelled) …` serve, where the
  unmarked companion governs. A governor group
  standing *first* is unwrapped before the walk
  reaches the mark — the escape needs the marked
  companion first, the ordinary ordering a sender
  writes. F4-of-round-38's outcome (a
  cancellation escaping at the leading slot)
  through the pass R38-D built.
- **Walk heads:** `;(Monday) November 1, 2026`,
  `;; (Monday) …`, `.(Monday) …` serve silently;
  the trailing twin `November 1, 2026
  ;(Monday)(notes)` likewise.
- **Separator runs between groups:**
  `(cancelled),,(Monday) November 1, 2026`
  serves silently — the run defeats the walk
  for the weekday group behind it (the governor
  group in the same run is judged, per the held
  doctrine families: the run splits the two
  contents classes).
- **Stacked suffix at the leading walk:**
  `(Monday)(notes)!’ November 1, 2026` serves
  silently, where `’`, `’’`, and `!` alone judge;
  the leading walk's suffix peel takes one mark
  where the lone form's tail takes the stack.

Mechanism: `group_sequence_weekday_mismatch`
(composition.rs:2044) and its callers, and
`leading_group_sequence` / `trailing_group_
sequence` / `base_trailing_sequence_mismatch`,
admit the R38-B sets only in the positions R38-B
witnessed; the walks' own comments claim the
borne-out set and are corrected with the fix.
Fix shape (adopted): the walks and the governor
pass take the lone forms' full separator/suffix
discipline at every boundary position — heads,
gaps (the three omitted marks admitted as a
group's suffix), runs of admitted separators,
and stacked suffix peels — with pins at all
three slots in both orders plus the
companion-first governor forms. Grade follows
RT38-4 / F2: a weekday judgment (and at the
pass, a cancellation) escaped by dress, a date
served silently in its place. Disposition: R39-C.

### F5 (MAJOR) — Separator chains at the month–day slot take two more separators and their runs than R38-E admitted: a `.` or `:` link, a separator run, or separator dress on a lone group lets a governor escape

Review R39-R5 + red team RT39-4, merged per
adjudication 1. R38-E stated the slot for chains
whose links are bare adjacency or one `,`/`;`
(`chain_gap_is_link`, composition.rs:2462;
`dressed_day_start`, composition.rs:2434 admits
the same two marks as dress before the day). The
sentence separators R38-B established for group
separations — `.` and `:` — are not chain links,
and runs are not links in either set, both
routes: `November(cancelled).(notes) 1, 2026` and
`November(cancelled):(notes) 1, 2026` serve Nov 1
live with no record, where the spaced twin
governs; `November(cancelled),,(notes) 1, 2026`,
`;;`, `,;` runs serve the same way; the lone
group wearing the mark as dress before its day
escapes identically (`November(cancelled). 1,
2026`, `November(cancelled): 1, 2026` serve;
`November(cancelled),; 1, 2026` serves), where
the `,`/`;` dresses have governed since R38-E.
A cancellation serving its date live through a
one-character dress change is F5-of-round-38's
outcome at its own site, one separator further.
Fix shape (adopted): `chain_gap_is_link` and
`dressed_day_start` take the full set — bare
adjacency, or one or a run of `,` `;` `.` `:` —
with the R38-E pin family re-run in the new
links (governor in either position, three-group
chains, the dressed singles). Disposition: R39-D,
with F6.

### F6 (MINOR) — The inert arm of F5: `.`/`:`-linked and run-linked inert chains at the month–day slot read a date their spaced twins refuse

Review R39-R6 + the inert forms of RT39-4's
mechanism. `November(notes).(choir) 1, 2026` and
the three-group `November(notes).(choir).(band)
1, 2026` read Nov 1 glued; the spaced twins
refuse, as the `,`/`;` chains have since R38-E.
Spacing-as-semantics alone, no governing content
and no record at stake — the F6-of-round-38 arm
of the same mechanism, graded as it was. (The
weekday arms at this site are at parity and are
not part of this finding: the comma chains and
spaced forms alike read nothing with no record —
the record loss there is uniform, as review
witnessed.) Disposition: R39-D, with F5.

### F7 (MAJOR) — A sentence mark inside a weekday word inside a group disarms the weekday judgment at all three garnish slots; the bare twin refuses with the record at all three

Review R39-R7. R38-F shed an interior mark from
a bare date word, vocabulary-anchored, guarded
to marks *outside* every group; the writer note
scoped the in-group case as the grouped readers'
standing silence, witnessed there for a month
word inside a wrapped date (the line dies — the
safe direction). For a *weekday* group the same
silence is the judgment's death, not the line's:
`November 1, 2026 (Mon!day)` serves Nov 1 with
no record, where bare `November 1, 2026 Mon!day`
refuses with WeekdayMismatch(Monday@2026-11-01);
leading `(Mon!day) November 1, 2026` serves
silently where the bare form refuses; base-
trailing `Sunday after November 1, 2026
(Mon!day)` serves Nov 8 silently where the bare
form refuses. The edge form inside a group
judges (`(Monday!)` refuses with the record), so
the grouped weekday readers trim a token's edges
but not its interior, while the bare path sheds
both since R38-F. A disagreeing weekday escaping
judgment with the date served is the standing
MAJOR outcome class (F2/F7 shape). Fix shape
(adopted): in the grouped weekday readers
(`garnish_group_weekday` and the framing
readers' group cleaning), a weekday word whose
interior holds one ASCII sentence mark is judged
as the word with the mark shed, under the same
vocabulary anchor as `bare_date_mark_shed`'s
interior arm — the shed completes only a weekday
name. The boundary is stated with the fix: no
governor word is ever completed by shedding (the
F10 governor-stem hold stands; review's
interior-mark governor groups — `(cancell!ed)
November 8, 2026` serving — are recorded below,
not fixed, because the stamp as written is not a
governor word and the bare twin's silence there
is the same non-word). The wrapped-date month
case (line-death either way) is untouched.
Disposition: R39-E.

### F8 (MINOR) — Bare-path dress parity, the third tail: mark runs, a second interior mark, stray runs wearing closing quotes, doubled periods, the comma-less year tail, a triple-nested month group, a marked doubled day group, a day-head mark, and quote-wrapped month/day words each kill the reading silently

Review R39-R8 + red team RT39-6/RT39-7, merged
per adjudication 1. All arms silence-only (no
date, no record, both routes), each against a
twin that reads:

- Stray run wearing a closing quote:
  `November 1, 2026)”` / `)’` / `)"` / `)'` read
  nothing — `stray_closer_is_spaced`'s
  trailing-run set admits closers, sentence
  marks, and whitespace but not the closing-
  quote family the year-tail peel sheds.
- Doubled period at the day slot: `November 1..,
  2026` reads nothing; the wrapped twin reads,
  and the single period has read since R38-F —
  the day slot drops a period only with the
  comma immediately after it, so neither mark of
  the pair is shed. (Also in RT39-6's day-slot
  runs.)
- Year tail on a comma-less date: `November 1
  2026!` / `2026”` read nothing; the clean
  comma-less form serves and the grouped twin
  reads — the year-tail arm requires the date's
  comma the comma-less form lawfully lacks.
- Mark runs at the date slots: `November 1!!,
  2026`, `November!! 1, 2026` read nothing —
  the shed takes single marks.
- A second interior mark: `Sun!day! after
  November 1, 2026` reads nothing; the
  single-mark form held in R38-F's family.
- Triple-nested month group: `(((November))) 1,
  2026` reads nothing (doubled whole-date and
  doubled day groups held this round).
- A mark inside a doubled day group: `November
  ((8!)), 2026` reads nothing;
  `mid_date_group_reading` /
  `nested_bare_day_group_inner` meet the mark
  inside the nesting.
- A mark at the day token's head: `November !1,
  2026` reads nothing.
- Quote-wrapped month/day words: `"November" 1,
  2026` and `November "1", 2026` read nothing;
  the clean twins read — past R38-F's
  quoted-weekday fix, the quoted *date words*
  at their own slots.

One batch shape (the R38-F precedent): the
stray-run set gains the closing quotes; the day
slot sheds a run of periods as it sheds one;
the year-tail arm admits a year standing after
a day number with whitespace between, comma or
none; mark runs and a second interior mark shed
under the standing vocabulary anchor; the
nested-group readers step past a mark inside
the nesting; a fully quote-wrapped date word is
transparent at its slot as the quoted weekday
word became at R38-F. Disposition: R39-F.

### F9 (MAJOR) — Three superlinear shapes in the contribution readers: group runs without a date, mixed unclosed groups, and nested day groups

Red team RT39-8, adjudicated per adjudication
2. Measured with the lane's scratch probe
(debug profile, min-of-N within each run; the
box carried cross-lane load of 8.7–11.4, and
every shape was measured in the same windows as
its control — the evidence is the per-step
ratio, not the absolute time; `perf39.log`,
`perf39b.log`):

- **Group run, no date** (`(notes)` × k): 276 B
  → 8.2 ms; 976 B → 50 ms; 3,776 B → 1,837 ms;
  14,976 B → 25,437 ms. Per-4× ratios 6.1 / 36 /
  13.8; time per group grows 0.25 ms → 11.9 ms
  across the run.
- **Mixed unclosed groups** (`([({` repeated,
  no date): 245 B → 28.6 ms; 845 B → 1,205 ms
  (42× for 4×). (A larger run was killed by the
  lane and discarded; the two completed sizes
  stand.)
- **Nested day group** (`November (((…8…))),
  2026` — R38-F's own `nested_bare_day_group_
  inner` territory): 461 B → 6.3 ms; 1,661 B →
  133 ms; 6,461 B → 2,912 ms (21× / 22× per 4×).
- Lesser arms in the same windows: unclosed `(`
  run, separator-group run, a leading group
  sequence with a date at 664 ms / 12,862 B.
- **Control in the same windows**: a single long
  word scales linearly (0.46 / 1.6 / 6.3 /
  26.8 ms across the same sizes) — the blow-ups
  are shape-bound, not box-bound.

The neighbourhood is the pipeline's whole-line
passes (span enumeration, sequence walks, chain
and group readers in composition.rs) each
rescanning group structure the earlier passes
already walked; no profile was taken, so the
per-pass attribution is the shape's
neighbourhood, not a measured split — the batch
establishes it. A 15 KB group-run line costing
~25 s in a debug parse is a cliff on
sender-controlled text at sizes a pasted
bulletin can reach. Disposition: R39-G — quiet-
window re-measurement first (frame named,
min-of-N, in-window linear control), fix on
confirmation, readings byte-preserved under the
fuzz hard term; if the quiet witness refutes
superlinearity, the batch closes as a record
and reports instead of fixing.

### F10 (MAJOR) — Doctrine change, adopted per adjudication 3: a wrapped governor governs as its bare twin at the trailing slot and in a mid-line group before a plain date; the base-trailing slot is uniform and does not move

Uniformity evidence: adjudication 3 and the
three lane reports (verify's 171-line probe with
full tables, review's 61-line doctrine sweep,
the red team's doctrine families with the
sequence-interaction witnesses). The adjudicated
reading: a group's governor contents are judged
as the bare governor is judged **at the trailing
slot** (after a plain date, in every wrapper,
glued, doubled, mixed, marked, with companions —
the companions' own judgments standing as the
walks give them) and **in a mid-line group
standing in the date's segment before a plain
date**, where the bare stamp's segment judgment
reaches. Where the bare stamp is itself garnish,
the wrapped form stands as it stands: after a
Sunday-after base in every dress (adjudication
4), before phrases in both dresses, beside a
weekday group mid-line, and at the R38-C
interior positions whose bare governor is
garnish (the held-neighbours parity). The
doctrine statement the batch carries in code:
governor contents in a group are judged as the
bare governor is judged wherever the bare
governor is judged — the leading slot (R38-D),
the trailing slot, and the date's segment
mid-line; weekday-group judgment remains
framing-slot only (cell (a)(i) stands closed,
not re-opened — adjudication 3(e)).

**The standing stop-and-report term applies
with force** (the R38-D precedent): the batch
restates exactly the named families and stops
and reports on any other pin found encoding
governor-as-garnish —

- **R34-E family 2** —
  `r34e_bracketed_cancellation_word_leaves_the_
  core_as_its_paren_twin` (trailing governor
  group in every pair plus "(called off)",
  headline serving).
- **R37-D trailing companion arms** —
  `r37d_companion_contents_keep_their_standing_
  judgments_and_the_r5d_boundary_stands`
  (trailing half only; its leading arms were
  R38-D's to restate and stand).
- **R38-B trailing arms** —
  `r38b_sequence_walks_take_the_lone_forms_
  sentence_separators` (the `;`-separated
  trailing companion arms; its base-trailing
  arms stand).
- **R35-F battery** — `R35F_BATTERY` entries
  `rt-cancel-word-after-core/{paren,bracket,
  brace}` and `rv-mixed/ctl-cancelword-*`.
- **R38-D's scope controls** —
  `r38d_wrapped_leading_governor_governs_as_its_
  bare_twin`: its trailing-form statement is
  restated; its base-trailing control stands.

**Not restated:** R37-G's garnish pin and every
base-trailing pin (uniform — adjudication 4);
the R38-C held-neighbours pin and the R37-B
relative-phrase shapes (parity in both dresses);
every weekday pin (cell (a)(i) closed). The
fuzz hard term applies: any movement is reported
by shape before commit, and a re-baseline is a
coordinator ruling, never the writer's call.
Disposition: R39-H.

## Records (not findings)

- **The base-trailing premise correction** —
  adjudication 4. The R38-D stop report's
  base-trailing arm is corrected on three lanes'
  evidence; its leading and trailing evidence
  stands as probed.
- **The R38-F count claim** — adjudication 6.
- **The F6/F9 recycled-pid residual** — boundary
  re-verified twice over: review's code read
  (`peer_provably_gone`, ob-app main.rs:4001-4003,
  still rests on PeerLiveness::Dead alone; the
  round-38 diff does not touch ob-app) and the
  red team's lock sweep (all prior shapes
  reproduced, T6 ×50 bad=0, N9–N12 verdicts
  unchanged, three new combined shapes N13–N15
  landing on the decision table's stated
  AND-of-verdicts, 0 deviations). Stands
  recorded-not-fixed; its boundary has not
  widened.
- **The R38-F peeled-tail residual** — stands as
  recorded, now witnessed firsthand by two lanes:
  tails needing more than the one peel
  (`November 1, 2026 (Monday) !`, the
  doubled-stray tails `(Monday)).` / `(Monday))!`)
  serve with no weekday record, exactly as
  `r38f-record.md` records; review maps the
  peel's boundary precisely (the mark *after*
  the group refuses; a sequence tail refuses).
  Carried as a standing record — its fix, if a
  future round takes it, is a peel-depth change,
  not a parity repair.
- **Interior-mark governor groups** (review
  record): `(cancell!ed) November 8, 2026`
  serves, where the bare twins read nothing —
  the stamp as written is not a governor word,
  and completing it by shedding is the move F10
  of round 38 forbids for governors. Recorded
  beside F7, whose weekday arm has the opposite
  twin shape; F7's fix is bounded to weekday
  names for exactly this reason.
- **The governor after `after`** (verify
  record): `Sunday after cancelled November 1,
  2026` reads nothing; `Sunday after (cancelled)
  November 1, 2026` serves the base — a standing
  R38-C neighbour reading at a position F3's
  batch will work beside. Named in F3's scope
  note; carried to round 40 as a witnessed
  neighbour, not graded here.
- **Ellipsis suffixes on sequences** (review
  record): `November 1, 2026 (Monday)(notes)…`
  serves silently — and the lone twin serves
  identically, U+2026 being in neither the
  walks' nor the lone readers' suffix peels.
  No twin divergence; the lone form's own `…`
  reading is a question no lane has put.
- **Crossed pairs govern at the trailing and
  mid-line slots** (verify record): `[cancelled)`
  / `(cancelled]` govern as the bare twin does —
  the parity pockets inside F10's cells, where
  the dress stops being a balanced segment.
  Under F10 they already read as adjudicated;
  no batch work attaches.
- **Quoted words at the date's interior slots**
  (review record): quoted == bare silence at the
  month-interior and day–year slots, grouped
  serving at the day–year slot being the
  segment design — parity under F7-of-round-38's
  adjudication; no adjacency stands.
- **Red team held lines**, recorded as evidence:
  the sector-archive variants of the R38-A sites
  held on derivable hosts; the counted-signature
  zip64 variant derives and refuses; the plain
  signature-record plant held (R38 N4
  re-witnessed); split CD/EOCD plants killed by
  the frame anchor; rights quadruples 100,000 at
  0 standing flips; the lock sweep above; 682 +
  8,000 generated contribution parses, 0 panics;
  84 archive crafts × 4 doors, 0 panics.
- **Verify lane incidents**, recorded: three VM
  reboots killed suite runs 1, 2, and 5 (partials
  preserved, discarded as evidence); runs 3 and
  4's cliff-pin failures under load 9–14 were
  handled by the standing remedy (quiet solo
  re-runs passed; run 6 passed all three pins
  in-run). The scheduling note stands: heavy
  lane runs serialize where practical; no bound
  is widened on an environment's account.
- **Review lane incidents**, recorded: two probe
  builds were reported complete by the runtime
  while still running (their processes found
  alive, the duplicate killed), and a restart
  killed the surviving build; no evidence was
  taken from any phantom-completed or
  interrupted run. The evidence discipline —
  verdicts from disk logs with embedded exits
  only — is what caught both.

## Batch plan

One writer at a time; every batch under the
batch-boundary discipline (commit + push +
HEAD == origin confirmed, cargo clean, cold
verification replacing incremental; per-batch
records in `round39-out/coord/` in the standing
format). Contribution batches (B–H) carry the
fuzz hard term: verdict/comp byte-identical to
the standing baselines (verdict `79775844…`,
comp `5868726a…`), rights counters
5083 / 1024 / 5275, safety_flips=0 — a batch
that moves a fuzz reading stops and reports.
Any re-baseline is a coordinator ruling on the
R34-E / R36-D / R38-C precedent, never a
writer's call.

- **R39-A** — F1 + F2 (zip doors): the walked
  extent accounts for every byte from a
  candidate's archive start through the end of
  its walked directory, gaps included, and a
  local header stands in the extent from its own
  fields whether or not its payload span
  resolves; a zip64 candidate failing validation
  only by an uncounted signature record at its
  walk's end derives under the signature
  allowance. Pins: the three F1 regions and the
  F2 plants at all four doors with their
  refuse / land-both controls; the counted and
  plain signature held lines unweakened; every
  standing door pin unweakened.
- **R39-B** — F3 (interior-slot guard at the
  further positions): framing word after
  `after`; group between `after` and the base
  judged as its paren twin; glued-to-"Sunday"
  split detection; weekday group at the split
  position; any weekday-word prefix at the
  after-weekday slot. Pins: the lanes' families,
  all pairs, glued and spaced, both routes,
  against the paren twins; the R38-C pins
  unweakened. Scope note: the batch states in
  its record whether the governor-after-`after`
  forms moved (report, not judge).
- **R39-C** — F4 (walks + governor pass take
  the lone forms' boundary sets): heads, runs,
  the `!` `?` U+201D gap marks, stacked suffix
  peels; the walks' parity comments corrected
  to the borne-out set. Pins: the lanes'
  families at all three slots in both orders,
  plus the companion-first governor forms;
  R37-D/G and R38-B pins unweakened.
- **R39-D** — F5 + F6 (separator chains at the
  month–day slot take the full link set): runs
  and `.`/`:` links in `chain_gap_is_link` and
  `dressed_day_start`. Pins: the lanes'
  families (governor in either position,
  three-group chains, dressed singles, inert
  arms); R38-E pins unweakened.
- **R39-E** — F7 (grouped weekday interior
  mark): the grouped weekday readers judge a
  weekday word with one interior ASCII sentence
  mark shed, vocabulary-anchored to weekday
  names only. Pins: the three garnish slots,
  both routes, against the bare twins; the
  governor-completion boundary stated in the
  pin set (no governor word is completed); the
  wrapped-date month case stated untouched.
- **R39-F** — F8 (bare-path parity tail, third):
  the stray-run set gains the closing quotes;
  the day slot sheds period runs; the
  comma-less year tail; mark runs and second
  interior marks; the nested readers step past
  a mark inside the nesting; quote-wrapped
  date words transparent at their slots. Pins
  per family, both routes, against the reading
  twins; R37-F / R38-F pins unweakened except
  where a family restates one outright,
  flagged in the batch record.
- **R39-G** — F9 (superlinear shapes): quiet
  re-measurement witness first (frame named,
  min-of-N, in-window linear control); on
  confirmation, the rescanning shape(s) fixed
  with readings byte-preserved. Pins: the
  ratio-bound form of the standing timing-pin
  precedent (interleaved control, per-shape
  bounds stated from the confirmed witness);
  the batch reports instead of fixing if the
  quiet witness refutes.
- **R39-H** — F10 (doctrine change: wrapped
  governor governs at the trailing slot and in
  the mid-line segment): the doctrine statement
  carried in code; restatements exactly the
  families named in F10, flagged in the writer
  note; the stop-and-report term with force —
  any other pin encoding governor-as-garnish
  stops the batch. Pins: the doctrine families
  (trailing: all pairs, doubled, mixed, marked,
  glued, companions both orders, sequences;
  mid-line: spaced clean dress before a plain
  date, prefixes, companions) against the bare
  twins on both routes; the standing controls
  stated outright and unmoved — base-trailing
  in every dress, before-phrase forms,
  mid-line beside a weekday group, the R38-C
  held neighbours.

Then the coordinator close: five-leg evidence
on the final tree (local cold, fuzz as claims,
hosted CI ×3, desktop runner build, Windows
bundle), boundary hygiene, and the build-cache
policy disposition at the confirmed completion
(df reading + window + latch state over the
round's recorded readings). HF-12 untouched.
The Tier-2 final gate is not triggered (0/8/2)
and is not the coordinator's to start in any
case.
