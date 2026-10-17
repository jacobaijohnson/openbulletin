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

_(appended by the R13-A builder with real numbers and flagged pin changes)_

## Shipped addendum, R13-B

_(appended by the R13-B builder with real numbers and flagged pin changes)_

## Shipped addendum, R13-C

_(appended by the R13-C builder with real numbers and flagged pin changes)_
