# Round 13 audit synthesis (Wonder verify / review / red team over HEAD `ee2f2b3`)

Three blind lanes over the round-12 fixes. Verify: **PASS, 12 VERIFIED / 0
FAILED / 0 UNTESTABLE** (674/0 unpiped, clippy -D warnings exit 0, booklet
exact, Bach live; CI run 39053505785 for `ee2f2b3` SUCCESS on all three OS
jobs; bundle run 39053507357 SUCCESS). Review: **0 BLOCKER / 2 MAJOR / 5
MINOR**, four questions answered. Red team: **0 BLOCKER / 2 MAJOR / 4
MINOR**. Deduped total: **0 BLOCKER, 3 MAJOR, 5 MINOR. The exit bar is NOT
met** — the loop continues into round 14; the Tier-2 final gate stays queued.

Held broadly, for the record: the full round-12 regression re-run (every
diff is an R12 disposition landing; serving byte-identical; plain-2900 at
505 ms); clause separators (semicolon, dash, em dash) and governor scoping
across them; uppercase governors; two-date plain lines in all extensions
(three framed, descending, same date in two formats, ranges both ways);
loader identity under dual plain/zip64 descriptions and the 8,193-record cap
under merged keys; disjoint-window cost tracking the bounding span; the pure
R12-10 gap shape; save-speaks naming many events in full; the insert path
held by construction (no migration can arise there).

## The MAJORs

### R13-1 (MAJOR, review R13-1 + red team RT13-1): the governor families still miss the contraction class, the cancellation verbs, and the "wrong" label

The two phrasings R12-A left at the disposition's edge both reproduce the
full R11-4 harm on every path, as the round-12 close predicted they might:
"(won't be November 1)" and "(wrong: November 1)" let the disowned date arm,
bless, and set keep-through. The lanes widen the family in two directions.
Review: **wont / cannot / can't / shouldn't** fail identically (while
will-not / must-not / is-not govern correctly — the token "not" is listed,
its contractions are not). Red team: the cancellation **verbs** an office
actually writes — **cancel** (verb form), **scratch**, **drop**, **hold
off**, **postpone**, **remove** — none govern.

Disposition (R13-A): **the families are completed structurally, not by
another word harvest.** Negation gains the contraction class as a class:
any token ending in "n't", plus wont and cannot, is a negation governor —
the same rule a reader applies. Cancellation gains the attested verbs:
cancel, scratch, drop, postpone, remove, hold off. Labels gain "wrong:".
Each attested form is pinned on the consuming paths. The list mechanism
stands (round-12 Q3); what changes is that the classes are now stated the
way the language states them, so the next novel phrasing has a class to fall
in or out of, and lanes can grade a miss against the classes.

### R13-2 (MAJOR, review): a clause break erases an unspent governor

R12-A scoped a governor to its own clause. The segmentation is
position-blind: a comma or semicolon standing between a governor and its
first date erases the governor entirely — "(ignore, November 1)", "(from,
November 1)", "(skip, November 1)", "(please ignore, November 1)", "(do not
use, November 1)", "(ignore the date, November 1)" and six more all
contribute with full harm. Governor-between-two-dates cases are clean, and
the red team held semicolon/dash separators and governor scoping across
them — the defect is specifically the break that lands AFTER a governor
which has governed nothing yet.

Disposition (R13-A): **a clause break erases only a spent governor.** A
governor that has governed no date and stands as its clause's trailing
element — nothing between it and the break but date-referring filler ("the
date", "it") — still governs the first date of the next clause. A governor
followed by other words in its own clause ("notes from the pastor, …")
still reaches nothing across the break: the R12-A garnish pin stands.
Formally: the governor's window is its clause, plus the head of the next
clause when the governor is unspent and clause-trailing.

### R13-3 (MAJOR, red team RT13-2): the shared resolution pass is O(candidates × chain length), not O(span + candidates)

R12-B's account claimed one chain walk per distinct directory start. The
implementation's `chain_count` is a per-candidate linear scan: with 40k
body signatures, cost goes 26–71 ms at a 2-record chain to 1.32–1.60 s at
an 8,000-record chain; with 200k fakes on a 5.2 MB file, 6.80 s (the honest
bulletin still loads — this is a cost break, not a refusal). The
multiplication moved from per-candidate directory walks to per-candidate
chain scans.

Disposition (R13-B): **a directory's chain is walked once per load, full
stop.** Chain results are shared across every candidate and both passes
(enumeration and fallback) by directory start, the way R12-B described and
did not quite do. The 200k-fake / 8,000-record-chain fixture lands in the
sub-second class; total resolution cost is O(span + distinct chains +
candidates).

## The MINORs

### R13-4 (MINOR, review R13-3 + R13-4 + red team RT13-3): the fallback's keyed representative can be the wrong twin, and reader_walkable approximates the reader

Three faces of one family. (a) A gap-standing directory whose declared size
lies by −7 or +1 byte, behind 100 fakes, is refused although the trimmed
reader reads it: `reader_walkable` chains records across the declared
*size*, while the reader (zip 2.4.2, verified in its source) is driven by
the declared *total*. (b) A same-key zero-total later twin makes the
fallback skip the honest earlier twin — the key's representative resolution
is the later candidate's failed one. (c) A lying-total twin counted and
opened under the same key as a gap-standing honest candidate, with 70
fakes, refuses at both doors while reader-alone controls load it.

Disposition (R13-B): **per key, the representative is the best resolution
any twin achieves, and the walkable test mirrors the reader.** If any
candidate under a key walks or reads, the key walks or reads; a key whose
representative yields nothing tries its other twins before the gate closes;
`reader_walkable` chains records and compares the walked count against the
totals the reader itself checks (the R12-9(b) walked-count presentation
already computes the right number). All three constructions load; the storm
and dead-zone families keep their outcomes.

### R13-5 (MINOR, red team RT13-4): a framed date plus a relative phrase reads as no date

"October 25, 2026 and next Sunday" reads NoSundayNamed, nothing noted. The
R12-6 plain multi-date reading covers framed + framed only.

Disposition (R13-C): **the plain multi-signal reading covers mixed
signals.** A framed date and a relative phrase in either order: the first
signal is the line's date (standing primacy), the other is noted — the
relative phrase's resolution named as a second date, or the framed date
named beside the resolution — through the existing note machinery.

### R13-6 (MINOR, review R13-5 + red team RT13-5): governor words inside titles and names suppress asserted dates — accepted limitation, documented

"(song: Skip to My Lou November 1)", a volunteer named Skip, uncomma'd
"choir was singing November 1": list words in non-governing positions
inside a group disqualify a date the sender asserts. Both lanes grade it
MINOR and the shapes are plausible. The failure direction is the safe one:
nothing arms, nothing is blessed; the date is simply not read from that
group, the part shows its words in review, and the office can still file
it. Every narrowing the lanes or the coordinator could state (governing
distance, title labels) misclassifies an attested governor shape in the
other direction ("(ignore the date, November 1)" on one side, "(old date:
…)" on the other).

Disposition: **accepted as a documented limitation; no code change in
round 13.** This paragraph is the documentation. It reopens if a lane
shows the suppression causing harm beyond a date unread from a
parenthetical (for instance, a keep-through silently shortened in a shape
an office plausibly writes).

### R13-7 (MINOR, review R13-6 + red team RT13-6): the same-target pairing note is a false alarm, and unfollowed events print out of order

A duplicate pairing to the SAME target ("'Xray' was paired with 'Yankee'
and with 'Yankee'… the pairing with 'Yankee' was not followed. Correct the
names by hand…") alarms about an end state identical to the single pairing.
Separately, multiple unfollowed events print pairings-then-removals rather
than in the segment order the office wrote them. (The many-events wording
itself held: 4 events → 4 full sentences.)

Disposition (R13-C): an unfollowed pairing whose target equals the kept
pairing's target is not recorded as an unfollowed event (the end state is
the single pairing's); unfollowed events are worded in segment order.

### R13-8 (MINOR, verify variance + review R13-7): the timing pin's floor still fails under contention alone

The R12-8 floor (max(control ×20, control + 15 ms × candidates)) failed
solo at loads 10–13.5 in both audit lanes (54.7 s / 89.8 s / 65.9 s against
~45 s budgets; outcomes always passing; the pin green in-suite). Three
rounds of lanes have now spent words on this one assertion. The structural
assertions (one enumerated candidate, refusal wording) are the real guard
against multiplication, backed by the r12b timing pins at smaller counts.
One record correction: R12-B's new formula also *tightened* the
300-candidate budget versus the old control ×20 + 10 s; the R12-B addendum
did not mention that direction. Noted here for the record.

Disposition (R13-C, test-only): the r10 pin's wall-clock assertion becomes
a hang guard (a cap no contention at recorded loads approaches and no
honest load reaches — 300 s — whose only job is to catch a hung or
runaway load), with the pin's outcome and structural assertions unchanged
and primary. Per-candidate multiplication remains guarded by the
one-candidate structural assertion in the same pin and by the r11/r12b
timing pins.

## Fix batches

- **R13-A (governors):** R13-1 (contraction class, cancellation verbs,
  "wrong:" label) + R13-2 (a break erases only a spent governor).
- **R13-B (loader):** R13-3 (chain walked once per load, shared by both
  passes) + R13-4 (best-twin representatives; reader-mirroring walkable).
- **R13-C (small print):** R13-5 (mixed plain signals) + R13-7 (same-target
  suppression; segment-order events) + R13-8 (timing hang guard, test-only).
  R13-6 is documentation-only, above.

Each batch appends its dated Shipped addendum here with real numbers. Round
14 audits the fixes; the exit bar is unchanged.

## Shipped addendum, R13-A

Shipped 2026-10-17 on top of `2156588`. **677 tests passing / 0 failed**
(full unpiped `cargo test --workspace`, cargo exit 0; baseline 674),
**clippy clean** (`cargo clippy --workspace --all-targets -- -D warnings`,
exit 0). All changes are in `crates/ob-extract/src/contribution.rs`, in
the one group analysis every consuming path reads.

- **R13-1.** The governor families are stated once, in a new
  `is_governor_phrase` helper that the window check and the new
  clause-trailing check both read. Negation is the contraction class as
  a class: any token ending in "n't" (either apostrophe), plus
  unpunctuated "wont" and "cannot", is a negation governor; the R12
  unpunctuated forms (isnt / arent / wasnt / werent) stand as before.
  Cancellation gains the verbs cancel, scratch, drop, postpone, remove,
  and the pair "hold off". Labels gain "wrong" — `group_token` strips
  the colon, so the token governs bare as well as in the "wrong:" label
  form; both forms reproduce the harm in the review lane's probe and
  both are now disqualified. Held controls stand: will-not / must-not /
  is-not still govern.
- **R13-2.** `governor_window_start` now takes the group's tokens and,
  when the last clause break before a date closes a clause whose
  trailing element is an unspent governor (nothing after the governor
  phrase but the filler "the" / "date" / "it"), the window reaches back
  to that clause's start, so the governor governs the first date of the
  next clause. A cancellation standing in the clause directly after a
  date is spent on that date (the `governed_after` rule) and carries no
  further, so "(November 1, cancelled, November 8)" still asserts
  November 8. `clause_break_after` now classes dash breaks with comma
  and semicolon breaks (a standalone "-" / "--" token, or a token
  wearing a trailing hyphen or en/em dash; `group_token` strips the
  same dashes), in both directions: "(ignore - November 1)"
  disqualifies, "(notes from the pastor - November 1)" contributes as
  garnish. One consequence of the disposition's formal rule, named for
  the record: "(ignore, use November 1)" no longer contributes — the
  unspent trailing "ignore" governs the next clause's first date,
  where round 13's probe recorded the pre-fix contribution as
  accidentally right.
- **Pins.** Three new `r13a_*` pins, all in ob-extract (171 → 174 in
  that crate): `r13a_contraction_class_and_wrong_label_on_every_path`
  (won't / wont / cannot / can't / shouldn't / couldn't on the relative
  path; won't + "wrong:" on all five paths; will-not / must-not /
  is-not held), `r13a_cancellation_verbs_disqualify` (all six verbs on
  the relative path; cancel + scratch on both Keep-through paths),
  `r13a_unspent_governor_survives_clause_break` (all six witnesses plus
  the semicolon, hyphen-dash, and em-dash forms on the relative,
  paren-only, and Keep-through paths; the garnish pin on comma,
  semicolon, and dash; "(choir was singing, November 1)" still
  contributes; "(choir from November)" still reads no date; the
  between-two-dates pins re-asserted with original expectations).
- **Pre-existing test changes: none.** The diff removes no pre-existing
  test line; the removed lines are the production functions this batch
  rewrote (`governed_before`, `governor_window_start`,
  `clause_break_after`, `group_token`) and their doc comments.

## Shipped addendum, R13-B

Shipped 2026-10-17 on top of `63f1468`. **681 tests passing / 0 failed**
(full unpiped `cargo test --workspace`, cargo exit 0; baseline 677),
**clippy clean** (`cargo clippy --workspace --all-targets -- -D warnings`,
exit 0). All changes are in `crates/ob-format/src/lib.rs`, plus the
FORMAT.md wording corrections named below.

- **R13-3.** `ChainWalk` now carries a position index — offset to the
  count of records chained there — built once when the chain is
  walked. `chain_count` and every reachability read are index lookups,
  in both passes (the enumeration's file chains in
  `directory_candidates` and the fallback's byte chains in
  `candidate_resolutions_in`); no candidate rescans a chain, and a
  chain is walked once per distinct directory start in each pass, so
  total resolution cost is O(span + distinct chains + candidates).
  The R12-B account's "one chain walk per distinct start" wording is
  now actually true of the count reads as well as the walks.
  Measured on the new pin's fixtures (release build, machine load
  average ~3.9–5.5 during the run; the pre-fix column is RT13-2's
  release measurements): chain of 2 records with 40k fakes (883 KB):
  26–71 ms before, **22.9 ms** after. Chain of 8,000 records with
  40k fakes (1.30 MB): 1.32–1.60 s before, **92.8 ms** after. Chain
  of 8,000 records with 200k fakes (4.82 MB): 6.80 s before,
  **152.9 ms** after — the sub-second class. The residual long-chain
  constant is one chain walk plus one reader parse of that directory
  per load; it does not scale with the candidate count (×5 candidates
  at the 8,000 chain: 92.8 → 152.9 ms). In the debug suite build
  under the same load the three rows measured 143 / 274 / 752 ms.
  The honest bulletin loads in every row.
- **R13-4.** `reader_walkable` mirrors the reader: the chain parses
  exactly the candidate's declared total of records from the resolved
  start (`ChainWalk::parses_total`) — the count zip 2.4.2 itself
  checks. The declared size is no longer consulted anywhere in the
  resolution: the stage-3 extent drops its size target, and a
  candidate standing only by the total reading presents the total as
  its exact count for the zip64 patch. The loaders' dedupe unit is
  the TWIN (`TwinKey`: the resolved `DirKey` plus the declared
  total, post-zip64-retarget, carried on `CandidateResolution`): the
  tried set is seeded from the shared resolutions at the positions
  the enumeration tried, the reader gate's untried test is by twin,
  and the fallback inserts by twin — so a key whose representative
  yields nothing has each distinct-total twin tried once before the
  load refuses, at both doors. First-error precedence is unchanged.
  The R11 declared-variants family still dedupes to one open: its
  candidates resolve through one zip64 record, whose retarget makes
  their totals identical, so varied plain fields remain one twin.
- **Pins.** Four new `r13b_*` pins, all in ob-format (51 → 55 in that
  crate): `r13b_chain_cost_no_longer_scales_with_chain_length` (the
  three RT13-2 rows, outcomes plus time-class budgets),
  `r13b_a_gap_directory_with_a_lying_size_loads_behind_many_fakes`
  (size −7 and +1 behind 100 fakes, both doors, premises asserted:
  the resolution does not walk, does read, and the trimmed reader
  reads the document), `r13b_a_zero_total_later_twin_does_not_skip_the_honest_twin`
  (premises: one key, totals 0 and 2, the whole-file reader reads
  the zero twin empty, the honest twin's stream reads the document),
  and `r13b_a_lying_total_twin_does_not_close_the_honest_twin_out`
  (totals 1 and 5000 at the directory's end, honest twin behind a
  gap, 70 fakes, both doors, premises asserted per twin).
  Regression families unchanged: the full workspace suite passes
  with the storm, dead-zone, shared-directory (plain and zip64),
  declared-varied, gap, and lying-zip64-total pins at their original
  expectations.
- **Pre-existing test changes: none.** No pre-existing test line is
  removed or altered; the diff's removed lines are the production
  functions and comments this batch rewrote (`chain_count`, the
  size-bounded `reader_walkable` branch, the stage-3 size extent,
  the loaders' key-only dedupe) and the two FORMAT.md passages below.
- **FORMAT.md.** The guards bullet now states the count reads come
  off the one chain's position index (R13-3) and that the fallback
  dedupes by resolved identity plus declared total, each distinct
  total opened at most once (R13-4) — the earlier "walked and opened
  at most once per load" overstated the twin case. The reader-path
  sentence's "reads by its declared size" is corrected to the
  total-driven reading (R13-4).

## Shipped addendum, R13-C

Shipped 2026-10-17 on top of `ac08ea2`. **685 tests passing / 0 failed**
(full unpiped `cargo test --workspace`, cargo exit 0; baseline 681),
**clippy clean** (`cargo clippy --workspace --all-targets -- -D warnings`,
exit 0). Changes in `crates/ob-extract/src/contribution.rs` (R13-5),
`crates/ob-model/src/lib.rs` and `crates/ob-app/src/main.rs` (R13-7),
and `crates/ob-format/src/lib.rs` (R13-8, test-only).

- **R13-5.** `plain_multi_date_reading` now scans a plain line for
  SIGNALS in written order — a framed date, or a relative phrase
  carried by its resolution — instead of framed runs only. A relative
  phrase resolves against `today`, the only anchor a plain line
  carries at parse time, under the reading in which "this" and
  "next" name different days: "this Sunday" and a bare "Sunday" name
  the coming Sunday (October 18 at the fixtures' today, Friday
  October 16, 2026), "next Sunday" the Sunday after it (October 25),
  a Sunday-after phrase its standing first-Sunday-after resolution,
  its base consumed as the second-date scan already consumes it.
  The FIRST signal is the line's date (standing first-date primacy,
  either order); every other distinct signal date joins the
  second-date note through the existing machinery — a relative
  phrase's resolution named as a second date, or a framed date named
  beside the resolution. The same signal twice reads plainly with
  nothing noted: the framed date restated, a relative phrase whose
  resolution IS the framed date (in either order), or the same
  relative phrase twice. A bare "Sunday" standing directly after a
  framed Sunday date is that date's trailing weekday garnish, not a
  second signal. Lines whose only signals are relative phrases
  naming different Sundays are not this shape: a bare relative line
  keeps its standing in-words reading, anchored on the bulletin, and
  differing relatives with no framed signal still read
  NoSundayNamed. The Keep-through line shares the reading, as it
  shares R12-6.
- **R13-7.** (a) In `migrate_serving_line_renames_detailed`, a later
  pairing whose target equals the kept pairing's target is still not
  followed (the name is spent; the accounting is unchanged) but is
  NOT recorded as an unfollowed event — the end state is the single
  pairing's, and the shipped sentence was a false alarm. An
  unfollowed removal is unaffected: a removal is never the same
  event as the kept pairing. (b) The collected events are reordered
  into SEGMENT ORDER before the migration returns: each event takes
  the place of its own item in the pairing's segment runs
  (recomputed on the filtered runs the pairing paired from), an
  event no segment item matches keeps its collected place after
  every matched event, and the app's sentence — which words the
  events in carried order — now speaks them as the office wrote
  them. The migration accounting is unchanged: pairings are still
  followed (and spent) before removals are judged, exactly as
  before; only the event recording and the carried order changed.
- **R13-8 (test-only, FLAGGED as by-disposition).** The r10
  shared-directory pin's two wall-clock budget computations are now
  a single hang guard, `Duration::from_secs(300)`, whose only job is
  catching a hung or runaway load; the assertion messages say so.
  The reason is recorded in a comment at the budget site: the
  contention measurements (23–90 s at machine loads 8–13.5 across
  rounds 12–13, outcomes always passing) and that both previous
  formulas — the control ×20 multiple, then control + 15 ms ×
  candidates — failed under contention alone. The pin's name and
  structure are unchanged; its outcome assertions and its
  one-candidate structural assertion are unchanged and primary, and
  the r12b timing pins in the same crate keep
  `shared_directory_budget` and their time-class budgets, so
  per-candidate multiplication remains guarded there and by the
  structural assertion here. The pin passes with the hang guard
  (11.75 s solo in this run; its outcome assertions untouched).
- **Pins.** Four new `r13c_*` pins: in ob-extract (174 → 175)
  `r13c_mixed_plain_signals_read_the_first_and_note_the_other`
  (framed-first and relative-first in both directions, a
  Sunday-after relative phrase in both orders, the restated-signal
  cases reading plainly, the Keep-through mirror, and the held
  bare-relative and differing-relatives lines); in ob-model
  (112 → 114) `r13c_same_target_pairings_are_not_unfollowed_events`
  (same-target duplicate carries no event and no note, the
  participants and alias standing exactly as the single pairing's;
  a Yankee/Zulu/Yankee triple carries only the real Zulu event; a
  different target still notes) and
  `r13c_unfollowed_events_are_carried_in_segment_order`
  (pair/removal/pair and a two-name interleaving, carried in
  segment order with the accounting standing); in ob-app
  (173 → 174) `r13c_same_target_pairings_speak_no_note_and_events_follow_segment_order`
  (the status sentence is empty for the same-target duplicate, and
  the pair/removal/pair wording names the removal before the Zulu
  pairing). The R12-C pins pass with their original expectations in
  both crates. The r10 pin is counted under R13-8 above, not here:
  ob-format stays at 55 pins, its only change the flagged budget.
- **Pre-existing test changes: one, flagged.** The r10
  shared-directory pin's budget computations, assertion messages,
  and timing comment (R13-8, by disposition, FLAGGED in the pin's
  body) — its outcome and structural assertions are untouched. No
  other pre-existing test line is removed or altered; the diff's
  other removed lines are the production code this batch rewrote
  (`plain_multi_date_reading`'s framed-run scan, the pairs loop's
  event recording in the migration) and their comments.

## Coordinator close (2026-10-17)

All three batches shipped and were independently re-verified by the
coordinator (full unpiped suite + clippy -D warnings at each step): R13-A
`cd531ce` (677 tests; governor classes completed structurally, unspent
governors survive clause breaks; zero changed pins), R13-B `ac08ea2` (681
tests; chain indexed once per walk — 200k fakes 6.80 s → 152.9 ms — and
twin-keyed dedupe with a total-driven walkable test; zero changed pins),
R13-C `046ebf7` (685 tests; mixed plain signals, same-target suppression
with segment-order events; one flagged change by disposition, the timing
hang guard). Between R13-A and R13-B the V3b app icon Jacob chose was
adopted (`63f1468`: full set in crates/ob-app/assets/icon/, master in
icon.png, openbulletin.ico in the Windows bundle, transparent glyph in
docs/assets/ and the README header; .icns noted as later work). Round 14
audits these fixes; its briefs re-probe the wider governor net R13-A left
outside its attested set (put off, postponed, delay, omit, disregard,
forget, nope; trailing cancellation verbs in governed_after; hyphenated
compounds) so each gets a disposition. The Tier-2 final gate runs only
after a round returns zero BLOCKER + zero MAJOR.
