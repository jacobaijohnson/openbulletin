# Audit round 37 — synthesis (coordinator, 2026-10-22)

Audited tree: `d985de8` (round-36 close record on code
tree `e7f3be8`). Three blind lanes, read-only:
verify (`round37-verify.md`), review
(`round37-review.md`), red team
(`round37-redteam.md`), all in
`~/workspace/openbulletin-audit-wave3/`, scratch in
`round37-out/`.

Lane verdicts: verify PASS on all five standing
claims (0/3/2 handed on), review 0/5/3, red team
0/4/4. Deduped: **0 BLOCKER / 6 MAJOR / 5 MINOR** —
the exit bar is NOT met. Majors trend:
10 → 6 → 6 → 1 → 4 → 3 → 2 → 5 → 6 → 8 → 6.

The verify lane's standing claims all held on the
audited tree: suite 1053/0 (run 2; run 1 was killed
mid-flight by a VM reboot and discarded as evidence,
see Records), clippy 0 on a genuinely re-checked
tree (the lane rejected its own first run as a cache
hit and re-checked all 16 crates after
`cargo clean -p`), booklet 31,717 / 34,977
byte-exact, fuzz bins at their recorded md5s with
stdouts byte-identical to the R36-D baselines
(verdict `79775844…`, comp `77f0addd…`), rights
counters 5083 / 1024 / 5275 with safety_flips=0
(M8 held, seventeenth consecutive round), and all
thirteen round-36 pins present in-tree and passing.
Every finding below comes from the carried
observations and the lanes' own hunting, not from a
failed standing claim.

## Adjudications recorded before the findings

1. **The curly family splits by outcome (F2 / F3).**
   All three lanes witnessed carried target (b).
   Arms where a judgment is escaped and a wrong
   outcome is served — a governed date adopted, a
   phrase collapsed to its base, a phantom record
   on agreeing dates, a refusal all twins record
   replaced by silent service — are MAJOR (F2).
   Arms where the line merely goes silent or a
   record is lost, no wrong date adopted, are
   MINOR (F3), on the round-36 F12 precedent.
2. **The two-group compound splits by content
   (F5 / F6).** Carried target (c)(ii), witnessed
   by all three lanes. Contents carrying a
   judgment (governing words in either position, a
   mixed weekday wrapper, a date group) escape it
   glued — MAJOR; inert contents flip
   date-vs-no-date on spacing alone — MINOR. The
   round-36 F1/F10 split, applied to the compound
   family; both land in one batch (R37-C).
3. **R37-R4 is adjudicated IN as MAJOR (F7).** The
   review lane's own caveat is recorded: a bare
   word between a weekday group and the date also
   reads clean, and that arm is the standing R5-D
   adjacency doctrine — it is NOT part of the
   finding. The group-companion arm stands: no pin
   or scope note records adjacency intent for
   groups, R35-C's recorded claim asserts the
   opposite ("spaced and two-group compound forms
   keep the R35-C weekday reading"), lone groups
   judge at the leading and trailing slots, and
   month–day compounds judge. Adjudicated reading:
   a companion group does not disarm a weekday
   group's judgment at the leading or trailing
   slot, glued or spaced. If the fix round finds a
   pin encoding compound-adjacency intent, the
   batch stops and reports, per the standing term.
4. **RT37-2's three vectors are one finding
   (F4-archive, stated as F8 below).** A planted
   directory inside a zip64 extensible sector,
   inside an entry's payload, and behind a prefix
   archive whose EOCD comment reaches into the
   real archive are one candidate-discipline
   defect by the round-36 F6 merge precedent:
   R36-A's discipline names only candidates inside
   another candidate's declared comment span and
   the absorbed prefix; a candidate standing
   anywhere else in the file, earliest in file
   order, is judged. The review lane's zip cells
   (appended and comment-carried candidates in all
   zip64 framings) hold — those are the round-36
   vectors; the red team's are new plant sites.
5. **Two uniform cells are observations, not
   findings.** (i) A weekday group standing
   mid-line before the month word
   (`notes ([Monday])November 1, 2026` and its
   spaced twin) is unjudged in every form — no
   twin diverges; all three lanes touched the cell
   and none graded it. (ii) A wrapped leading
   governor (`(cancelled) November 8, 2026`, all
   pairs) is garnish while the bare leading
   governor governs — uniform across wrappers;
   the split is wrapped-vs-bare, not pair-vs-pair.
   Neither breaks an adjudicated reading. Both are
   recorded under Records and carried to round 38
   as doctrine questions, not defects.
6. **Rights permutation variance is not a
   finding.** The red team's 100,000-quadruple
   sweep: 0 flips on the standing M8 safety axes
   (children, permission) under the standing
   definition (fold vs same-order chain), and 0 on
   credit-presence under that definition. Three
   permutation-variance classes (license/credit
   text selection; credit presence when a
   permission bearer arrives after the credit it
   vouches — the R11-5 arrival-timing class;
   permission-noted where no children note stands)
   are the code's documented arrival-order
   semantics, witnessed and preserved in
   `round37-out/redteam/rights37*.log` for
   re-grade if the definition is ever widened.
7. **Claim-4 wording, corrected on the record.**
   The verdict bin's stdout is 16 physical lines;
   its summary line reads `total_lines=40000
   panics=0`. The standing claim's "40,000 lines"
   is the bin's internal case total, as the
   round-36 close record states; the byte-identical
   md5 is the dispositive check. Future briefs
   phrase the claim accordingly.

## Findings

### F1 (MAJOR) — Carried target (a): a non-leading bracket/brace group beside a relative phrase is adopted as the line's date; the paren twin reads nothing

Verify V1 + review R37-R1 + red team RT37-1,
three lanes converged. Same root mechanism as
round-36 F4: `date_line`'s plain multi-date
reading reads a bracket/brace group's date through
the R24-4 edge trim as a plain signal, and R36-D's
guard `leading_group_beside_relative_core`
(composition.rs:5036) is consulted only for
leading groups. Past any leading word the
bracket/brace group is field-identical to bare
text. Witnesses (both routes throughout):
`notes [November 8, 2026] next Sunday` → for =
2026-11-08 with the phrase's resolution demoted to
a second date; the paren twin is totally silent
and unremembered. The family: disagreeing dates
(`[November 15, 2026]`), restating dates, the
Sunday-after relative (`[November 8, 2026] the
Sunday after next Sunday` — the resolution
adopted, not even named second), stacked groups
(the bracket/brace group wins in either position),
doubled wrappers, prefixes (`the`, `choir notes`,
`notes,`). Held positions in the same sweeps:
leading forms converge in all pairs (R36-D);
groups after the phrase, beside a Sunday-after
phrase in both orders, weekday-garnish and
governor groups beside the phrase, stacked
agreeing groups, and mixed wrappers beside the
phrase all read as their twins. The hole is the
non-leading, pre-phrase slot. Disposition: R37-B.

### F2 (MAJOR) — Carried target (b), adopting arms: curly-quoted words escape judgment at the group, phrase, and garnish sites

Verify V3 + review R37-R2 + red team RT37-3.
Curly pair = U+201C/U+201D and U+2018/U+2019; the
straight-quoted twins read at full parity with
bare since R36-F, and that parity is the target
shape. Arms (both routes):

- **Governor inside a group.**
  `(November 8, 2026 “cancelled”)` and
  `(November 8, 2026 [“cancelled”])` adopt
  Nov 8; curly-single forms likewise. Straight
  and bare govern (for = None). Site:
  `clean_group_word` (dates.rs:1380) — its trim
  set contains no curly quotes — reached via
  `group_token` (dates.rs:1319).
- **Group canary.** `November 15, 2026 (“Sunday”
  after November 1, 2026)` → for = 2026-11-15
  with phantom ParenthesizedDateMismatch
  { named: 2026-11-01, read: 2026-11-15 }; the
  agreeing form serves Nov 8 with the same
  phantom shape. Straight/bare twins read the
  resolution with the line's date as second date
  and record nothing. The F3 mechanism: the
  in-group phrase never forms because the
  `dates_inside_group_signals` gate
  (dates.rs:929) and `sunday_after_phrase_run`
  (composition.rs:2981) judge cleaned tokens and
  `group_token` leaves `“sunday”` uncleaned.
- **Phrase collapse.** `(“Sunday” after
  November 1, 2026)` → for = 2026-11-01 (the
  base adopted as the answer); same under
  bracket/brace outer wrappers and in
  trailing-group forms.
  `Sunday after (“Sunday”) November 1, 2026`
  collapses the same way, while its unwrapped
  curly twin reads Nov 8. Disagreeing forms:
  `(Sunday after “Monday” November 1, 2026)` →
  for = 2026-11-01, no record; the bare twin is
  refused with the mismatch recorded.
- **Base trailing garnish.**
  `Sunday after November 1, 2026 “Monday”` →
  for = 2026-11-08 served with no record; bare,
  straight, and paren twins all refuse with
  `WeekdayMismatch(Monday@2026-11-01)`. Site:
  `sunday_after_base`'s trailing trim
  (dates.rs ~:514) holds `\u{201d}` but not
  `\u{201c}`, so a full curly pair is not
  trimmed.

Disposition: R37-E, with F3 — one fix over the
full site set the lanes mapped (see R37-E).

### F3 (MINOR) — Carried target (b), silence arms: curly-quoted words read nothing where their straight and bare twins read

Verify V2 + review R37-R6 + red team RT37-5.
No wrong date is adopted in any arm; the line
goes silent or a record is lost (F12 class):

- Shape word whole-line: `“Sunday” after
  November 1, 2026` → total silence; also
  curly-single, open-quote-only, close-quote-only,
  and behind `the`. (Behind-"on" forms are silent
  in curly, straight, and bare alike — no
  divergence there.)
- Second word: `Sunday “after” November 1, 2026`
  → silence; straight reads Nov 8.
- Base-leading weekday, disagreeing (unwrapped):
  `Sunday after “Monday” November 1, 2026` →
  silence, **no mismatch record**; bare and paren
  twins refuse with the record. Agreeing arm
  reads Nov 8, at parity.
- Line-head framing: `“Monday” November 1, 2026`
  and `on “Monday” November 1, 2026` → silence;
  the agreeing `“Sunday” November 1, 2026` →
  silence where straight/bare serve 2026-11-01 —
  a correct date dropped entirely.
- Plain trailing garnish: `November 1, 2026
  “Monday”` → silence; straight/bare refuse with
  a record.
- Relative-phrase memory: `“next Sunday”` and
  `next “Sunday”` are not remembered; the straight
  form is remembered verbatim and the bare form
  is remembered.

Disposition: R37-E, with F2.

### F4 (MAJOR) — The typographic closing quotes U+201D / U+2019 are missing from the group trim sites: a governor wearing one escapes inside bracket/brace groups, and a wrapped base-leading weekday collapses the phrase

Review R37-R5 (round-36 fix-diff incompleteness,
R36-E / R36-B). R36-E widened the trims to
`! ? ' "` (ASCII), and the line readers learned
`\u{201d}` in R35-C — the group cleaners never
did. Witnesses (both routes):
`(November 8, 2026 [cancelled]”)` adopts Nov 8,
as do `{cancelled}”`, `[cancelled”]` (mark inside
the pair), `[cancelled]’`, and the bare-in-group
`(November 8, 2026 cancelled”)`; the ASCII-mark
and paren controls govern. The trailing-group
form `November 15, 2026 (November 8, 2026
[cancelled]”)` additionally records a phantom
mismatch naming the ungoverned group date. And
`Sunday after [Monday]” November 1, 2026` →
for = 2026-11-01, no record — the phrase
collapses to its base because
`bracketed_weekday_word` (dates.rs:1169) cannot
see the wrapped weekday. Sites:
`is_bracketed_token` (dates.rs:1151),
`group_token` (dates.rs:1319),
`clean_group_word` (dates.rs:1380),
`bracketed_weekday_word` (dates.rs:1169) — the
one shared trim set learns the typographic pair
everywhere it is stated. Disposition: R37-E,
with the curly family (the sites are the same
set F2/F3's fix must touch).

### F5 (MAJOR) — Carried target (c)(ii), judgment arms: two-group compounds glued at the month–day slot escape the judgments their spaced twins apply

Verify V4 + review R37-R3 + red team RT37-4.
`respaced_glued_groups_at_date_slots`
(composition.rs:1979) iterates top-level balanced
spans; in a glued compound neither span qualifies
(the first is followed by a group, not a day
digit; the second is preceded by a group, not a
month word), so the compound is never re-spaced.
Witnesses (both routes): a governing group in
either position serves its date live —
`November(cancelled)(notes) 1, 2026` and
`November(notes)(cancelled) 1, 2026` →
for = 2026-11-01, no record; spaced reads
nothing. A mixed weekday wrapper
`November([Monday])(notes) 1, 2026` serves
silently. A date group first
`November(November 8, 2026)(notes) 1, 2026` reads
the outer date with a mismatch record where the
spaced twin reads Nov 8 clean. Weekday-only
compounds and the half-glued control are
twin-equal, held by all lanes. Disposition:
R37-C, with F6.

### F6 (MINOR) — Carried target (c)(ii), inert arm: inert two-group compounds glued at the month–day slot read a date their spaced twins refuse

Verify V5 + review R37-R7 + red team RT37-6.
`November(notes)(choir) 1, 2026` and its
mixed-pair, repeated, and doubled forms read
2026-11-01 glued, nothing spaced, both routes.
No judgment is escaped; spacing-as-semantics
alone (F10 class). Same mechanism as F5.
Disposition: R37-C, with F5.

### F7 (MAJOR) — A weekday group with any companion group escapes judgment at the leading and trailing slots, in every spacing

Review R37-R4, adjudicated per adjudication 3.
Lone forms refuse with the record at both slots
(`(Monday)November 1, 2026`,
`November 1, 2026 (Monday)` →
WeekdayMismatch(Monday@2026-11-01)). Append one
inert group in any spacing — `(Monday)(notes)
November 1, 2026`, `(Monday) (notes) November 1,
2026`, `[Monday][notes]November 1, 2026`,
`November 1, 2026 (notes)(Monday)`,
`November 1, 2026 (Monday) (notes)` — and the
date is served silently, both routes. The
month–day slot judges weekday compounds in every
form (R37-R3's controls), so the machinery walks
compound groups where its readers reach them; the
leading/trailing garnish readers stop at one
group. This breaks R35-C's recorded claim
outright. Disposition: R37-D.

### F8 (MAJOR) — Archive candidate discipline has three new steering vectors: a directory planted in a zip64 extensible sector, in an entry's payload, or behind a prefix archive whose comment reaches into the real archive

Red team RT37-2, merged per adjudication 4.
Crafts preserved in `round37-out/redteam/
witness37/` (`mkfake37.py`). At every vector the
scan (`central_spots` on the derived directory)
judges the planted directory while the reader
(zip crate on the `prefix_free` view) serves the
real one, and every craft's real directory
carries a disagreeing duplicate the doors must
refuse:

- **Z1, zip64 extensible sector.** The real
  archive is a sector archive (arithmetic
  derivation refuses it by design; the hunt
  serves it); a complete fake CD + EOCD stands
  inside the sector bytes, inside no comment
  span. IDML serves "Secondcopy story words
  here"; .mxl loads SecondScore; fonts land
  only the Regular face (the Bold copy vanishes
  silently); clips land 1 picture. Controls
  refuse / land both.
- **Z2, entry payload.** Fake CD + EOCD inside a
  stored filler entry's payload. Same steering:
  IDML serves the second copy, .mxl loads
  SecondScore, fonts land only Regular. (Clips
  refuse on a read-back error — the safe
  direction, but on the planted directory's
  say-so.)
- **Z3, prefix comment reach.** A complete tiny
  zip before the real archive declares an EOCD
  comment (64 bytes; z3b: to the u16 cap) whose
  span covers the real archive's head.
  Absorption requires the later candidate's
  archive offset ≥ the prefix's comment end; the
  lying comment pushes comment end past the real
  start, so the tiny archive is not absorbed
  and, earliest, is judged (total 1, tiny.txt)
  while the reader serves the real archive.
  IDML serves the second copy; .mxl loads
  SecondScore; fonts report "no font files"
  (the user's real fonts silently dropped);
  clips add nothing. The comment-length-0
  control is round 36's held V7 — the only
  difference is the declared comment length.

Disposition: R37-A. Fix shape (from the lane,
adopted): candidate standing is judged against
the whole file layout — a candidate whose EOCD
stands inside another candidate's structural
span (its zip64 record + sector + locator, an
entry payload the standing candidate's walk
accounts for, or a span another candidate's
declared comment overlaps into a later archive)
cannot outrank the candidate that accounts for
those bytes; and an absorption test a comment
reaching past the next archive's start cannot
defeat.

### F9 (MINOR) — A sentence mark glued to a bare month word or day number kills the date silently; the grouped twins read through it

Red team RT37-7. `November! 1, 2026`,
`November? 1, 2026`, `November: 1, 2026` →
nothing; `November 1!, 2026`,
`November 1?, 2026` → nothing; the grouped
twins `(November! 8, 2026)` and
`[November! 8, 2026]` read Nov 8. The reading
exists in the grouped path; only the bare path
loses it, in silence (F12 class).
Disposition: R37-F.

### F10 (MINOR) — Stray-closer recovery gaps: a doubled head stray, a trailing stray, and a bare nested day group drop the date

Red team RT37-8 (round-36 F8 family, new cells).
R36-F's recovery spaces a word-head stray closer
and reads an unclosed opener; past it:
`))November 1, 2026` → nothing (the single
strays `)`, `]`, `}` read Nov 1);
`November 1, 2026)` (also `]`, `}`) → nothing
(the wrapped `(November 1, 2026))` reads);
`November (8), 2026` → nothing (the wrapped
twins `(November (8), 2026)`,
`[November [8], 2026]`, `(November [8], 2026)`
read Nov 8). No record in any form.
Disposition: R37-F.

### F11 (MINOR) — The weekday-mismatch record names a wrapped or marked weekday verbatim, wrapper residue included

Review R37-R8. The refusal is correct in every
form; the record's named string — the weekday
the Check surface quotes back to the sender —
is wrong: `Sunday after [Monday]! November 1,
2026` records `Monday]!`; `{Monday}?` records
`Monday}?`; `(Monday)!` records `Monday)!`;
`"Monday"` records `"Monday"`; the bare
`Monday!` records `Monday!`. Site:
`sunday_after_base` (dates.rs) builds the named
string with an edge trim of `()[]{},.` only —
sentence marks are absent, and `trim_matches`
cannot reach the closing bracket past a
trailing mark (F11 class: the record misquotes
the sender's word). Disposition: R37-F.

## Records (not findings)

- **(d) The F6/F9 recycled-pid residual**: its
  stated boundary re-verified by the review lane
  (code read: `peer_provably_gone`, ob-app
  main.rs:4001, still rests on PeerLiveness::Dead
  alone) and by the red team's lock sweep (the
  extraction body-identical to round 36's; prior
  shapes reproduced; 8 new combined shapes —
  displaced × tokenless × foreign-token — 0
  deviations from the decision table's stated
  AND-of-verdicts; N2 and N8 named in the lane
  report as the table's own stated branches, not
  breaks). Stands recorded-not-fixed.
- **The two uniform cells** (adjudication 5):
  the mid-line weekday group unjudged in all
  forms (verify's (c)(i) note, review's
  unwitnessed observation 2, red team's held
  mid-line families); and the wrapped leading
  governor as garnish in all pairs (red team
  observation). Carried to round 38 as doctrine
  questions.
- **Rights permutation variance** (adjudication
  6): three classes documented with witnesses in
  the red team's logs; the standing M8
  definition held at 0 flips in 100,000
  quadruples.
- **Archive held lines**: Z4 (zip64 plain
  disk-field disagreement) diverges nowhere —
  derivation and reader agree on the real
  directory in both the disagreeing and the 1/1
  forms; the review lane likewise confirmed
  agreeing nonzero disk fields are scan/reader
  parity, not a defect. Z6 extremes: 12 crafts ×
  4 doors, 0 panics; R36-A's checked
  `payload_span` holds (the review lane
  confirmed all four doors delegate to the one
  checked helper, witnessed refusing at the
  fonts door).
- **Contribution held lines** (red team sweep):
  181 families / 932 unique lines, both routes,
  0 panics; single-group twin parity held across
  54 slot families × 10 wrappers × 2 routes,
  including the carried (c)(i) mid-line cell;
  F5's in-group governor marks held for every
  mark including `. ,` and the curly closers
  (the in-group wrapped-governor position —
  distinct from F4's mark-trailing-the-token
  positions); F2/F3/F12 straight forms held.
- **Verify lane incidents, recorded**: suite
  run 1 killed mid-flight by a VM reboot
  (~10:29Z; partial log preserved and discarded);
  claim 1 rests on the complete run 2, in which
  the r32c ratio pin passed under load — no
  quiet re-run needed this round. The lane's
  first clippy run was a 5.37s cache hit and was
  not accepted; claim 2 rests on the genuine
  re-checks after `cargo clean -p`.
- **Review lane unwitnessed observations**:
  the fonts-door refusal wording varies with the
  steered field (both wordings refuse; no
  content served); curly opening quotes as
  governor marks witnessed only in pair form
  (open-only forms unprobed, predicted to escape
  under F2/F4's site sets); the zip64
  extra-field size/version gates in
  `payload_span` code-read only.

## Batch plan

One writer at a time; every batch under the
batch-boundary discipline (commit + push +
HEAD == origin confirmed, cargo clean, cold
verification replacing incremental; per-batch
records in `round37-out/coord/` in the standing
format). Contribution batches (B–F) carry the
fuzz hard term: verdict/comp byte-identical to
the R36-D baselines (verdict `79775844…`, comp
`77f0addd…`), rights counters 5083 / 1024 /
5275, safety_flips=0 — a batch that moves a fuzz
reading stops and reports. Any re-baseline is a
coordinator ruling on the R34-E / R36-D
precedent, never a writer's call.

- **R37-A** — F8 (zip doors): candidate standing
  judged against the whole file layout.
  Pins: Z1/Z2/Z3 (incl. z3b) at all four doors
  with controls; the sector-archive service
  control; every standing door pin unweakened.
- **R37-B** — F1 (non-leading groups beside
  relative phrases take the leading reading):
  the group judgment is consulted at the plain
  reading's non-leading pre-phrase positions as
  at the line's head, in one place. Pins: the
  three lanes' families — mid positions with
  prefixes, disagreeing / restating /
  Sunday-after relatives, stacked groups,
  doubled wrappers, both routes, field-for-field
  against the paren twins; the held positions
  (after-phrase, agreeing stacks, mixed
  wrappers) stated unchanged.
- **R37-C** — F5 + F6 (the date-slot pass
  reaches two-group compounds): a glued compound
  at the month–day slot takes its spaced
  reading, governing contents judged, inert
  contents refused as spaced. Pins: governing
  in either position, multi-word governor,
  mixed weekday wrapper, date-group first,
  inert family, doubled/mixed/repeated forms;
  single-group and half-glued controls
  unweakened.
- **R37-D** — F7 (a companion group does not
  disarm a weekday group): the leading and
  trailing garnish judgments walk the group
  sequence as the month–day readers do. Pins:
  companion in any spacing and pair at both
  slots, agreeing and disagreeing; the R5-D
  bare-word adjacency controls stated outright
  and unchanged.
- **R37-E** — F2 + F3 + F4 (typographic parity):
  the curly pair is learned at the full site set
  the lanes mapped — `clean_group_word`,
  `group_token`, the `dates_inside_group_signals`
  gate, `sunday_after_phrase_run`'s token
  cleaning, `sunday_after_outcome`'s shape trims,
  `sunday_after_base` (lead weekday and trailing
  trim), `bracketed_weekday_word`,
  `framed_date_outcome` / `clean`, and the
  relative-phrase memory reader — and
  U+201D/U+2019 join the trailing trim sets at
  `is_bracketed_token`, `group_token`,
  `clean_group_word`, `bracketed_weekday_word`.
  The adjudicated reading is parity with the
  straight-quoted twins at every site; a
  trim-only shape is the round-36 reverted
  anti-pattern and does not land. Pins: every
  witnessed arm from all three lanes, both
  routes.
- **R37-F** — F9 + F10 + F11 (bare-path and
  recovery parity; the record names the word):
  a sentence mark glued to a bare month word or
  day number reads as its grouped twin does;
  stray-closer recovery covers the doubled head
  stray, the trailing stray, and the bare nested
  day group as their wrapped twins read; the
  mismatch record's named string is the cleaned
  weekday at the base-leading site. Pins: the
  lanes' witness families, both routes.

Then the coordinator close: five-leg evidence on
the final tree (local cold, fuzz as claims,
hosted CI ×3, desktop runner build, Windows
bundle), boundary hygiene, and the build-cache
policy disposition at the confirmed completion
(df reading + window + latch state over the
round's recorded readings). HF-12 untouched.
The Tier-2 final gate is not triggered (0/6/5)
and is not the coordinator's to start in any case.

---

# Round 37 — close (coordinator, 2026-10-22)

Close code tree: `1167fda` (R37-G). Suite: 1053
(round-36 close) → **1076 / 0 failed**. All eleven
findings fixed or dispositioned; the exit bar was
NOT met by this round's audit verdict (0/6/5), so
round 38 audits this closed tree and the Tier-2
final gate is not triggered. Majors trend:
10 → 6 → 6 → 1 → 4 → 3 → 2 → 5 → 6 → 8 → 6.

## Batches, each cold-verified under the batch-boundary discipline

| Batch | Commit | Findings | Cold suite |
|---|---|---|---|
| R37-A | `b420c2d` | F8 | 1059/0 |
| R37-B | `b83bd4b` | F1 | 1064/0 |
| R37-C | `e5800cf` | F5, F6 | 1066/0 |
| R37-D | `15f2dd2` | F7 | 1068/0 |
| R37-E | `9c5949e` | F2, F3, F4 | 1071/0 |
| R37-F | `bae38b4` | F9, F10, F11 | 1074/0 |
| R37-G | `1167fda` | late batch (below) | 1076/0 |

Per-batch records: `round37-out/coord/r37{a..g}-record.md`.
Booklet baseline 31,717 imposed / 34,977 reader
exact in suite at every batch. Fuzz-as-claims ran
on every contribution batch (B–G): verdict/comp
byte-identical to the R36-D baselines, rights
counters 5083 / 1024 / 5275, safety_flips=0
throughout.

**R37-G (late batch).** The synthesis plan named
six batches; a seventh was dispositioned during
the fix phase from the R37-D writer's witnessed
flag, on the R35-H precedent: the Sunday-after
base's trailing garnish in compound form — a lone
disagreeing garnish refuses with the record
against the base date, but one inert companion
group appended in any spacing served the
resolution silently. The base's machinery now
walks the group sequence standing after the base
date, as R37-D's walk does at the plain slots.
Record: `r37g-record.md`.

**Synthesis correction.** The first synthesis
commit (`e7e31b6`) was truncated mid-F3; the
restoration commit `d59d13d` restored the missing
sections verbatim as adjudicated, flagged by the
R37-A writer and verified against the committed
file.

## The interrupted close and its recovery

The predecessor coordinator launched the close
script (`close-cold-verify.sh`, nohup, pid 36714)
at 2026-10-22T21:23:50Z. Its exec session then
disappeared from the runtime's view, and the close
was handed to a successor as interrupted
mid-clippy: no CLIPPY_EXIT recorded at handoff,
the clippy log still growing.

The successor's first act was the standing
live-writer check — and the premise was false.
The script was alive, parented to init, its
clippy actively compiling in the tree. The
successor stood down as sole writer, renamed the
still-growing clippy log aside and back (an
inode-preserving rename; the file is the complete
original log, 14,097 bytes, the standing size),
verified the hosted legs read-only, and monitored
the disk logs only.

The predecessor's script then completed every
remaining leg itself, in order: CLIPPY_EXIT=0
(21:55:21Z — the clippy leg is the predecessor's
own genuinely-cold run on the cleaned tree; the
successor's contingency warm re-run proved
unnecessary and was NOT used), BOOKLET_EXIT=0,
and the full fuzz block, ending FUZZ_END
22:09:38Z / CLOSE_SCRIPT_DONE 22:10:31Z. No second
writer ever entered the tree. The successor
performed only the close record, the cache-policy
disposition, and boundary hygiene.

Recorded lesson, the round-32 scar confirmed from
the other side: an exec session's disappearance
is not the process's death. The live-writer check
(processes, recent mtimes, growing logs) precedes
any recovery dispatch, and a recovery brief's
premise is a claim to verify, not a fact.

## Five-leg evidence on the close tree (`1167fda`)

1. **Local, cold (the predecessor's close run,
   completed as above):** after `cargo clean`
   (11,668 files, 8.6 GiB), suite **1076 passed /
   0 failed** (SUITE_EXIT=0, 21:24:15Z→21:48:49Z);
   clippy `--workspace --all-targets --
   -D warnings` CLIPPY_EXIT=0; booklet direct run
   imposed=31717 reader=34977, BOOKLET_SIZE_MISMATCH=0,
   BOOKLET_EXIT=0 (`close-cold-suite.log`,
   `close-cold-clippy.log`, `close-booklet.log`;
   exits in `close-cold-exits.log`).
2. **Fuzz as claims:** bin sources at recorded
   md5s (verdict `a9cac6c3…`, comp `4f85c605…`,
   rights `95a47ee1…`, rightsS `9346879d…`);
   verdict27 stdout md5 `797758443d101f9339fdd778e14af430`
   (total_lines=40000 panics=0); comp27 stdout md5
   `77f0adddc908d8cd4ada60bd48b94bae` (D_TOTAL=497024,
   D_PANICS=0); rights stdouts cmp-clean against
   the R34-D2 references, counters 5083 / 1024 /
   5275, safety_flips=0 on all three rights
   outputs; all four stderrs empty; no stray
   processes after the runs (`close-fuzz-exits.log`).
   M8 held.
3. **Hosted CI** on `1167fda`: run `39849263172`,
   **success** — test (macos-latest), test
   (windows-latest), test (ubuntu-latest) all
   success.
4. **Desktop runner build** on `1167fda`: run
   `39849262986`, **success**.
5. **Windows bundle** on `1167fda`: run
   `39856862283` (workflow_dispatch, dispatched by
   the predecessor at 21:23:28Z), **success** —
   Gate (format, tests, clippy) and bundle jobs
   both success.

## Carried to round 38

- The two uniform cells, as doctrine questions
  (adjudication 5): a weekday group standing
  mid-line before the month word is unjudged in
  every form; a wrapped leading governor is
  garnish in every pair while the bare leading
  governor governs. Neither breaks an adjudicated
  reading; both want a doctrine ruling, not a fix
  smuggled into a batch.
- Rights permutation variance (adjudication 6):
  three classes documented with witnesses in
  `round37-out/redteam/rights37*.log`; not
  findings under the standing M8 definition;
  re-grade only if that definition is widened.
- The review lane's remaining unwitnessed
  observations: the fonts-door refusal wording
  varies with the steered field (both wordings
  refuse; no content served); the zip64
  extra-field size/version gates in
  `payload_span` are code-read only. (The lane's
  curly open-only governor observation was
  witnessed and fixed inside R37-E.)
- The F6/F9 recycled-pid residual stands
  recorded-not-fixed, its boundary re-verified by
  this round's audit.
- The scheduling note stands: heavy lane runs
  serialize where practical, and timing-sensitive
  pins are not judged under concurrent load.
  HF-12 untouched throughout.

## Build-cache policy at the confirmed completion

Charter: `docs/planning/convergence-loop.md`;
source note `build-cache-policy-2026-10-20.md`.
Readings recorded during the round
(df -h /home/hatch, from the per-batch and close
exits logs): 82–85% across all batch verifies and
the close cold pass; the highest
overnight-window reading was 85% (line 87%); one
87% reading at the close fuzz start
(2026-10-22 15:04 PDT, daytime window, line 94%).
No reading met its window's threshold; no trigger
latched. Completion reading, after boundary
hygiene: 2026-10-22 15:04 PDT (daytime window),
**84% used / 17G free**. Disposition: **CARRY** —
no clear is owed; the target directory rebuilt by
the close cold pass carries into round 38.

Boundary hygiene in the same pass: seven probe
target directories swept — `round37-out/redteam/
probe37/target`, `round37-out/redteam/
probe37lock/target`, `round37-out/review/
probe37/target`, `round37-out/verify/probe37/
target`, `round37-out/coord/probe37e/target`,
`round37-out/coord/probe37f/target`, and
`probe29v/target` (~4.3 GB total). Probe sources,
logs, records, witnesses, and fuzz stdouts kept;
the four fuzz bin sources re-verified at their
recorded md5s after the sweep.
