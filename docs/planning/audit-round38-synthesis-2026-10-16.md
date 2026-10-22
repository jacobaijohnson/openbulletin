# Audit round 38 — synthesis (coordinator, 2026-10-22)

Audited tree: `eabd258` (round-37 close record on code
tree `1167fda`). Three blind lanes, read-only:
verify (`round38-verify.md`), review
(`round38-review.md`), red team
(`round38-redteam.md`), all in
`~/workspace/openbulletin-audit-wave3/`, scratch in
`round38-out/`.

Lane verdicts: verify PASS on all six standing
claims (0/0/0 handed on), review 0/1/2, red team
0/4/3. Deduped: **0 BLOCKER / 5 MAJOR / 5 MINOR** —
the exit bar is NOT met. Majors trend:
10 → 6 → 6 → 1 → 4 → 3 → 2 → 5 → 6 → 8 → 6 → 5.

The verify lane's standing claims all held on the
audited tree: suite 1076/0 (run 3; runs 1–2
recorded below as environment incidents), clippy 0
on a genuinely re-checked tree (the lane rejected
its own 6s cache hit and re-checked after
`cargo clean -p` on all 16 workspace crates),
booklet 31,717 / 34,977 byte-exact, fuzz bins at
their recorded md5s with stdouts byte-identical to
the R36-D baselines (verdict `79775844…`, comp
`77f0addd…`), rights counters 5083 / 1024 / 5275
with safety_flips=0 (M8 held, eighteenth
consecutive round), and all 21 round-37 pins in
the spot-audit set present in-tree and passing.
Every finding below comes from the carried targets
and the lanes' own hunting, not from a failed
standing claim.

## Adjudications recorded before the findings

1. **R38-R1 and RT38-4 are one finding (F2).**
   Review's separator arms cover all three
   sequence-walk slots for `;` `.` `:`; the red
   team's RT38-4 is the `;` arm at the leading
   and Sunday-after base-trailing slots, with the
   lone-form and comma-compound controls witnessed
   in the same run. One mechanism — the walks'
   admitted separator/suffix set is narrower than
   the lone forms' — one batch.
2. **The F2 separator caveat is answered on the
   evidence.** The composition segment reader does
   split on `;` `.` `:` — but no reader judges the
   weekday across that boundary either (the
   segment reader served these lines clean), and
   the lone forms judge across the same marks when
   the group stands alone. A boundary no reader
   observes is not a segment boundary for this
   judgment. The arms stand as graded.
3. **Doctrine cell (a)(i) is CLOSED as doctrine.**
   The mid-line weekday group is uniform under
   triple confirmation: verify (268 full records;
   10 pair shapes × glued/spaced/comma, sequences
   in both orders, agreeing arm symmetric),
   review (35 lines), red team (14 forms under
   attack, no divergence). Both arguing lanes
   converge on keep-unjudged, on one principle
   stated two ways: the slots judge by attribution,
   not proximity (verify), and `date_line_inner`'s
   head note has stated the direct-framing
   doctrine since R4-7 / R5-D (review). Standing
   doctrine, recorded: weekday-group judgment is
   a framing-slot judgment — leading, behind only
   `on`/`the`; trailing, closing the line; the
   Sunday-after base's own trailing slot — and
   mid-line silence is the stated reading, not a
   gap. Re-opening condition (verify's test): a
   witnessed form framed by the slot grammar's
   own devices yet escaping — a divergence inside
   the grammar, not a new position.
4. **Doctrine cell (a)(ii) is adjudicated IN as
   a finding (F4), as an openly adopted doctrine
   change.** The question was put to the lanes,
   and the evidence returned is stronger than the
   round-37 record held: the leading slot IS
   judged for contents — weekday sequences judge
   there (R37-D) and leading groups are consulted
   beside relative phrases (R36-D / R37-B) — with
   the governor as the one exempt contents class,
   and no ruling records the exemption. Inside
   one leading sequence the two stamps split:
   `(cancelled)(Monday) November 8, 2026` refuses
   on the weekday while the cancellation in the
   same sequence is ignored (review). And the
   slot splits by dress: a quote-wrapped bare
   governor governs there (`"cancelled" November
   8, 2026` → for = None) while every bracket
   form is garnish (review, 39 lines; red team,
   12 forms, uniform under attack). Dress is not
   semantics at a judged slot; a cancellation is
   the highest-stakes content a group can carry,
   and serving its date silently is the standing
   MAJOR outcome class. The counterweight is
   recorded: a leading group is also the
   restatement/framing slot — but that ambiguity
   is the bare stamp's too, and the program
   resolves the bare stamp by governing. The
   change is adopted openly here, not smuggled
   into a batch: F4's batch carries the doctrine
   statement, and any pin found encoding
   governor-as-garnish outside the adjudicated
   family stops the batch and reports.
5. **The month–day separator chains split by
   content (F5 / F6).** RT38-3 (governing
   contents escape glued behind a comma) is
   MAJOR; RT38-6 (inert chains flip on spacing;
   a weekday comma compound loses its record in
   both spacings) is MINOR. The round-36 F1/F10
   and round-37 F5/F6 split, applied to the
   chain family; both land in one batch (R38-E).
   The lane's adjudication-risk note is answered:
   the spaced-comma twin is judged, so the glued
   escape is spacing-as-semantics carrying a
   judgment — the F5 class, not a new position.
6. **RT38-5 is adjudicated IN (F9) over the
   review lane's observation.** Review recorded
   the bare year-tail marks without grading,
   citing the new pass's own doc ("keeps its
   bytes and its standing effect"). A doc
   sentence recording an effect is not an
   adjudication of it; the grouped twins read
   through the same marks at the same slot, and
   on the F9/F12 precedent a mark at a date
   token's edge is shed where the grouped readers
   shed it. IN as MINOR; the fix corrects the
   doc sentence with the behavior.
7. **Rights permutation variance (carried (b))
   is closed under the standing definition.**
   The red team's fresh sweep — 100,000
   quadruples on the round-37 generator and
   seeds — holds the standing M8 definition at
   0 flips and replicates round 37's counters
   exactly (37,716 / 386 / 244), the strongest
   replication the item has had. The widened
   definition (maximal permutation invariance)
   is stated precisely with fresh per-axis
   counts in `round38-out/redteam/rights38.log`
   and preserved as a record; its own lane
   argued no adoption. If a future adjudication
   widens the definition, those witnesses are
   the re-grade base. Not a finding.
8. **The review lane's F11 non-pair quote-dress
   observation stands as a record.** A
   base-leading weekday wearing a lone quote
   (pair or not, straight or curly) is named
   verbatim with its quote, uniformly; wrapped
   forms name the cleaned word. The refusal is
   correct in every form; no correctness stake.
   Recorded, not graded.

## Findings

### F1 (MAJOR) — Archive candidate discipline, next generation: a planted directory standing in a local-header extra field, in a central record's extra or comment field, or in a digital-signature record steers all four doors

Red team RT38-1. R37-A's discipline subordinates
a candidate whose end record stands inside
another candidate's declared comment span, its
entry payload spans (compressed data only), or
its zip64 structural span. Four byte regions no
span covers remain: the local header's fixed
part, name, and extra field; the central
directory's own bytes, including its records'
name, extra, and comment fields; and a
digital-signature record's data. A complete fake
CD + EOCD planted contiguously in any of them
derives, stands earliest, and is judged, while
the reader (zip crate on the `prefix_free` view)
serves the real archive — whose directory carries
a disagreeing duplicate the doors must refuse.
Witnesses (crafts in `round38-out/redteam/
witness38/`): N1 local-header extra (IDML serves
"Secondcopy story words here", .mxl loads
SecondScore, fonts land only the Regular face,
clips land 1 picture); N2b central extra and N3b
central comment, same steering at all four
doors; N4 signature-record data, same. One
candidate-discipline defect, four new plant
sites — the F6/F8 merge precedent. Fix shape
(from the lane, adopted): the structural-span
set a candidate is judged against must cover
every byte another standing candidate's own walk
accounts for — its entries' local headers whole
as well as their payloads, its central records
whole, and any signature record its frame
allowance spans — equivalently, a candidate
whose end record stands anywhere inside another
candidate's walked extent (first local header
through the end of its directory, signature
record included) cannot outrank the candidate
that accounts for those bytes. Disposition:
R38-A.

### F2 (MAJOR) — The round-37 group-sequence walks judge a narrower separator and suffix set than the lone forms: a `;` `.` or `:` between the groups, or a trailing U+2019 after the sequence, disarms the weekday judgment at every slot the walks serve

Review R38-R1 + red team RT38-4, merged per
adjudication 1. The lone forms judge a weekday
group followed by sentence punctuation and by a
typographic closing quote (`(Monday); November
1, 2026`, `November 1, 2026 (Monday);`,
`November 1, 2026 (Monday)’` all refuse with
WeekdayMismatch(Monday@2026-11-01)). The
R37-D / R37-G walks admit only whitespace and
comma separations and peel U+201D but not
U+2019. Witnesses (both routes): leading
`(Monday); (notes) November 1, 2026` → served
silently, and the `.` / `:` forms; trailing
`November 1, 2026 (notes); (Monday)` → served
silently, and siblings; base-trailing
`Sunday after November 1, 2026 (Monday);
(notes)` → 2026-11-08 served silently; suffix
arm `November 1, 2026 (Monday)(notes)’` and the
base-trailing twin → served silently, where the
`”` `'` `"` `!` `.` suffixed sequences refuse.
The walks' own comments claim "the separations
the lone forms allow"; the claim is falsified by
these witnesses and is corrected with the fix.
Disposition: R38-B.

### F3 (MAJOR) — R37-B's guard is not consulted at two further positions: a bracket/brace date group standing between a Sunday-after phrase's words, or after a weekday word before a relative phrase, is adopted as the line's date; the paren twins read nothing

Red team RT38-2. Between the phrase's words:
`Sunday [November 8, 2026] after November 1,
2026` → for = 2026-11-08 with the base demoted
to a second-date note; the disagreeing
`Sunday [November 15, 2026] after November 1,
2026` adopts Nov 15; the agreeing form diverges
the same way. After a weekday word:
`Sunday [November 8, 2026] next Sunday` →
2026-11-08 adopted. Paren twins totally silent
in every form, both routes. Held neighbour in
the same sweep: a governor group at the split
position is garnish in both pairs — the hole is
the date group's adoption, the round-36 F1 /
round-37 F1 mechanism at the phrase's interior
slots. Disposition: R38-C.

### F4 (MAJOR) — Doctrine change, adopted per adjudication 4: a wrapped leading governor is garnish in every wrapper form while the bare governor and the quote-wrapped bare governor govern at the same slot

Review doctrine section + red team uniformity
family. Uniformity is total: all three pairs,
doubled, mixed, marked, glued, spaced, behind
the `on`/`the` framing stack, with a companion,
before a Sunday-after phrase, mid-line, and
trailing forms serve the date with no record
(39 review lines + 12 red-team forms, both
routes); the bare governor governs in every
corresponding form, and the quote-wrapped bare
governor governs (`"cancelled"`, `“cancelled”`,
`‘cancelled’` all → for = None). Adjudicated
reading, adopted openly: a leading group's
governor contents are judged as the bare
governor is judged, at the leading slot, in
every wrapper — the slot already judges group
contents for weekdays and dates; the governor
is not exempt. Disposition: R38-D, with the
standing stop-and-report term stated in
adjudication 4 and the inert controls
(`(notes) November 8, 2026` serves) pinned
unmoved.

### F5 (MAJOR) — A comma-separated compound glued at the month–day slot lets a governor escape: the chain rule keys on spans standing immediately against each other, and a comma between the groups defeats it

Red team RT38-3. `November(cancelled),(notes)
1, 2026` → for = 2026-11-01 served live, no
record; the spaced twin `November (cancelled),
(notes) 1, 2026` reads nothing (the governor
governs), and the space-around-comma form
governs too — the escape is the glued comma
specifically, both routes. Disposition: R38-E,
with F6.

### F6 (MINOR) — Separator compounds at the month–day slot, silence and inert arms: inert comma/semicolon chains glued read a date their spaced twins refuse, and a weekday comma compound loses its record in both spacings

Red team RT38-6. Inert arms:
`November(notes),(choir) 1, 2026`, the
three-group chain, and the semicolon chain
`November(notes);(choir) 1, 2026` read Nov 1
glued, nothing spaced (spacing-as-semantics
alone, F10 class). Weekday comma arm:
`November(Monday), (notes) 1, 2026` and its
spaced twin both read nothing with no mismatch
record — the comma defeats the month–day
weekday judgment in both spacings, where the
unseparated spaced twin refuses with the record
(record lost, no wrong date served; F12 class).
Disposition: R38-E, with F5.

### F7 (MINOR) — A quoted word standing mid-line before the date silences the line, whatever the word is; the bare and grouped twins serve the date

Review R38-R2. `notes "Monday" November 1,
2026` → total silence (the grouped twin
`notes (Monday) November 1, 2026` serves
2026-11-01 as ordinary text under the (a)(i)
doctrine, and the bare twin serves it too);
`notes "Sunday" November 1, 2026` drops a
correct date; content does not matter
(`"notes"`, curly and single forms identical —
post-R37-E parity holds inside the silence).
The head position judges the quoted word
(`"Monday" November 1, 2026` refuses with the
record), so a quote-wrapped word is ordinary
text nowhere: judged at the head, poisonous
mid-line, where bare and grouped words at the
same position are ordinary text. Silence only,
F12 class. No pin or scope note records an
intent for quoted mid-line words to bound the
date line (the lane read for one). Disposition:
R38-F.

### F8 (MINOR) — R37-F's bare-path parity stops one shape short in three places: the day-slot period, the doubled/part-wearing trailing stray, and the doubled day group

Review R38-R3. All arms silence-only, both
routes, each against a wrapped/grouped twin
that reads: (i) `November 1., 2026` → nothing;
`(November 8., 2026)` reads Nov 8 —
`bare_date_mark_shed` matches only `! ? : ;`
where the grouped cleaning also sheds `.` at a
token's edge (the month slot does not diverge:
`November. 1, 2026` reads). (ii)
`November 1, 2026))`, `2026]]`, `2026)]`,
`2026).` → nothing; the single strays read and
the wrapped twins read through the run and the
period — the trailing arm spaces only a stray
with nothing but whitespace after it, and the
head arm's run logic was not carried to the
tail. (iii) `November ((8)), 2026` and
`November ([8]), 2026` → nothing; the wrapped
twin `(November ((8)), 2026)` reads Nov 8 —
`mid_date_group_reading` iterates flat spans,
and a span whose inner text holds an opener of
its own pair never reaches the splice.
Disposition: R38-F.

### F9 (MINOR) — A sentence mark glued to the bare year kills the date silently; the grouped twins read through it

Red team RT38-5, adjudicated per adjudication
6. `November 1, 2026!`, `2026?`, `2026:`,
`2026;` → nothing, both routes; the grouped
twins `(November 1, 2026!)`, `[November 1,
2026:]`, `{November 1, 2026;}` read Nov 1. The
year's tail is the one date-token edge
R37-F's bare-path shed does not reach. Held
neighbours in the same sweep: a year mark
inside a Sunday-after phrase is not a
divergence (phrase forms field-identical to
their clean twins) — the hole is the plain
bare line's year tail only. Disposition:
R38-F; the `bare_date_mark_shed` doc sentence
recording the standing effect is corrected
with the behavior.

### F10 (MINOR) — A sentence mark inside a bare word kills the reading silently

Red team RT38-7. Past F9's edge positions, a
mark inside the token, both routes:
`Novem!ber 1, 2026` → nothing;
`November 1, 20!26` → nothing; `Sun!day after
November 1, 2026` → nothing. The clean twins
read; no record in any form (F12 class). Held
neighbour: a mark at a weekday head's edge
reads at parity (`Sunday! after …` and
`Sunday? after …` field-identical to the clean
form — the R36-F family, held).
Disposition: R38-F.

## Records (not findings)

- **(a)(i) doctrine** — closed per adjudication
  3; the framing-slot doctrine is the standing
  reading, with verify's re-opening test on the
  record.
- **Rights permutation variance** — closed per
  adjudication 7; the widened definition and
  its fresh counts are preserved in
  `round38-out/redteam/rights38.log`.
- **(c)(i) fonts-door refusal wording** —
  closed, record-only (review): the wording
  tracks which layer refuses (the reader cannot
  open the archive → "not a readable zip
  archive"; the archive opens but every entry
  fails the payload/crc funnel → "no font files
  inside that zip"), both are the door's
  standing words for real states, and the
  per-entry pass-by is the documented E-9 Skip
  judgment. The residual discomfort stands: a
  one-font pack whose only entry is steered is
  told the pack holds no font files. A third
  wording for the all-entries-unreadable state
  is a wording-design call, not a correctness
  gap; not dispositioned.
- **(c)(ii) payload_span zip64 gates** — closed
  with evidence (review): undersized/swallowed
  zip64 extra fields leave sentinels unresolved
  → span None → door refusal; valid forms (sizes
  8/16/24, zip64 block second) serve; the
  local-offset boundary matrix is all clean, no
  panics (gates logs byte-identical across a
  rerun, md5 `cea9774d…`). Version fields carry
  no gate in either judge (10–65535 uniform).
  Scan-side note, recorded as the fence: G8's
  oversized declared block size resolves at
  `raw_spot` (`.min()` truncation) while every
  door refuses through the reader-open check —
  if a future door ever reads spots without
  opening the reader first, this note is where
  the fence is documented.
- **(c)(iii) open-only curly forms** — closed
  with evidence (verify): parity held at every
  probed site post-fix (governor in-group in
  bracket/bare/stray-closer/unclosed/brace
  forms; trailing-group and month–day slots;
  weekday parity at base-leading, line-head,
  trailing, and base-trailing positions; shape/
  phrase/memory parity), both routes.
- **(d) The F6/F9 recycled-pid residual** —
  boundary re-verified twice over: review's
  code read (`peer_provably_gone`, ob-app
  main.rs:4001-4003, still rests on
  PeerLiveness::Dead alone; callers unchanged
  in shape) and the red team's lock sweep (all
  prior shapes reproduced, T6 ×50 bad=0, four
  new combined shapes N9–N12 landing on the
  decision table's stated AND-of-verdicts, 0
  deviations; N9/N10 are the table's
  own-generation branch neighbouring the
  residual, as N8 was). Stands
  recorded-not-fixed.
- **N7d (red team record)**: a zip64 record
  whose on-this-disk count ≠ its total — the
  reader serves the archive while the
  derivation derives nothing and every door
  refuses ("its central directory cannot be
  derived"). The standing safe-direction
  posture (round 34): the doors refuse rather
  than scan an underived directory; no content
  is served on a planted directory's say-so.
  Recorded, not graded.
- **F11 non-pair quote dress** — recorded per
  adjudication 8.
- **Verify lane incidents, recorded**: suite
  run 1 killed mid-flight by a VM reboot
  (~22:22Z; partial preserved, discarded); run
  2 exited 101 on exactly one failure — the
  r31c wall-clock cliff pin (5.95s against its
  5s budget) under cross-lane load ~9–12 —
  re-run quiet it passed in 2.74s, and it
  passed in-run in run 3, on which claim 1
  rests. The wall-clock pins' load-flake
  pattern is now witnessed in three rounds
  (r32c twice, r31c once); the scheduling note
  (serialize heavy lane runs where practical)
  stands, and no bound is widened on an
  environment's account.
- **Red team held lines**, recorded as
  evidence: R37-C chains hold at 3–4 groups
  (17/18 families), R37-D/G walks hold (19/21),
  R37-E curly parity holds at 14 new-site
  families; split CD/EOCD plants killed by the
  frame anchor; the two-prefix chain absorbed
  whole (N5); zip64 disk lies N7a–c in
  derivation/reader agreement; 48 archive
  crafts × 4 doors, 406 + 58 contribution
  parses, 100,000 rights quadruples, the lock
  suite — 0 panics anywhere. Not re-attacked
  this round (stated by the lane): sector-
  archive variants of the F1 plant sites.

## Batch plan

One writer at a time; every batch under the
batch-boundary discipline (commit + push +
HEAD == origin confirmed, cargo clean, cold
verification replacing incremental; per-batch
records in `round38-out/coord/` in the standing
format). Contribution batches (B–F) carry the
fuzz hard term: verdict/comp byte-identical to
the standing baselines (verdict `79775844…`,
comp `77f0addd…`), rights counters
5083 / 1024 / 5275, safety_flips=0 — a batch
that moves a fuzz reading stops and reports.
Any re-baseline is a coordinator ruling on the
R34-E / R36-D precedent, never a writer's call.

- **R38-A** — F1 (zip doors): candidate standing
  judged against the whole walked extent — a
  candidate whose end record stands inside
  another candidate's local headers, central
  records whole, or signature record cannot
  outrank the candidate that accounts for those
  bytes. Pins: N1 / N2b / N3b / N4 at all four
  doors with their refuse / land-both controls;
  the split-plant and two-prefix held lines
  unweakened; every standing door pin
  unweakened.
- **R38-B** — F2 (sequence walks take the lone
  forms' separator and suffix sets): `;` `.`
  `:` admitted as separations inside the
  leading, trailing, and base-trailing sequence
  walks, and U+2019 joins the walks' suffix
  peels. Pins: the R1 / RT38-4 families at all
  three slots, both routes, field-for-field
  against the lone twins; the walks' parity
  comments corrected to the borne-out set;
  R37-D/G pins unweakened.
- **R38-C** — F3 (the guard at the phrase's
  interior slots): the R36-D / R37-B group
  judgment is consulted between a Sunday-after
  phrase's words and after a weekday word
  before a relative phrase, as at the
  non-leading pre-phrase slot. Pins: the
  RT38-2 families (split-phrase agreeing /
  disagreeing / restating; after-weekday-word),
  all pairs, both routes, against the paren
  twins; the held neighbour (governor at the
  split position, garnish in both pairs) stated
  unchanged.
- **R38-D** — F4 (doctrine change: a wrapped
  leading governor governs): a leading group's
  governor contents are judged at the leading
  slot as the bare governor is judged. Pins:
  the witnessed family (all pairs, doubled,
  mixed, marked, glued, framing stack,
  companion, before a Sunday-after phrase)
  against the bare twins; the inert controls
  (`(notes) November 8, 2026` serves) and the
  quote-wrapped forms stated unmoved; leading
  date-group framing and weekday-sequence pins
  unweakened. The standing stop-and-report
  term applies with force: a pin found
  encoding governor-as-garnish outside the
  adjudicated family stops the batch.
- **R38-E** — F5 + F6 (separator chains at the
  month–day slot take their spaced readings):
  comma/semicolon-joined glued compounds are
  walked as chains (governing contents judged,
  inert contents refused as spaced), and a
  weekday comma compound records in both
  spacings. Pins: the RT38-3 / RT38-6 families;
  R37-C's glued 3–4-group chains and the
  single-group pins unweakened.
- **R38-F** — F7 + F8 + F9 + F10 (bare-path
  silence parity tail): a quoted mid-line word
  stops poisoning the plain reading; the
  day-slot period joins the bare mark shed;
  trailing stray runs and part-wearing strays
  are spaced as their wrapped twins read; a
  doubled day group reaches the splice; the
  year's tail joins the shed (doc sentence
  corrected); a mark inside a bare word no
  longer kills the reading. Pins per family,
  both routes, against the reading twins; the
  R37-F pins unweakened except where a family
  above restates one outright, flagged in the
  batch record.

Then the coordinator close: five-leg evidence
on the final tree (local cold, fuzz as claims,
hosted CI ×3, desktop runner build, Windows
bundle), boundary hygiene, and the build-cache
policy disposition at the confirmed completion
(df reading + window + latch state over the
round's recorded readings). HF-12 untouched.
The Tier-2 final gate is not triggered (0/5/5)
and is not the coordinator's to start in any
case.
