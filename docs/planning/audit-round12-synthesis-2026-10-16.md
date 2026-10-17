# Round 12 audit synthesis (Wonder verify / review / red team over HEAD `421c1c9`)

Three blind lanes over the round-11 fixes. Verify: **PASS, 12 VERIFIED / 0
FAILED / 0 UNTESTABLE** (662/0 unpiped, clippy -D warnings exit 0, booklet
exact, Bach live; CI run 39038915426 confirmed green on all three OS jobs).
Review: **0 BLOCKER / 3 MAJOR / 5 MINOR**, four questions answered. Red team:
**0 BLOCKER / 4 MAJOR / 3 MINOR**. Deduped total: **0 BLOCKER, 6 MAJOR, 6
MINOR. The exit bar is NOT met** — the loop continues into round 13; the
Tier-2 final gate stays queued.

What held, stated first because it bounds the rest: **the rights safety axes
are now solid.** The red team ran ~1.69M fold/merge sequences (the 27-pool at
length 4, a widened family at lengths 5–6, the shared-credit pool at lengths
5–6 with zero divergences of any kind, a 12-archetype core carrying the
outside-pool shapes, plus 1,699,353 composed fold-prefix → single-merge
splits) and found **zero (children, permission-noted) flips anywhere**, zero
one-sided credit loss at every length, and save/reload landing on the same
triple (a fixed point). Per-occurrence serving held on every construction,
including triple segments, both-sides reservoir, and the Next Sunday
rollover. The review lane's census reproduced the R11-A addendum exactly and
held it at lengths 5–6 in 232k further sequences (Q1: the license/credit
residuals are honestly bounded; the credit class's MINOR standing is correct
— its real reach is the printed attribution words, see R12-9).

CI and bundle for `421c1c9`: ci.yml run 39045028113 SUCCESS on all three OS
jobs; bundle run 39045029389 SUCCESS.

One cross-lane observation: both the verify and review lanes hit the R10
shared-directory pin's *timing-budget* assertion failing solo under machine
load 8.4–13 (outcomes always passing; green once the machine quieted). The
budget is derived from an in-test control measured under the same contention,
so contention alone can fail it. It is a test-budget defect, not a product
one; R12-8 dispositions it.

## The MAJORs

### R12-1 (MAJOR, red team RT12-1 + review R12-3): the governor list misses ordinary words, and every miss reproduces the R11-4 harm

R11-C's governors cover not / ignore / never / no longer / don't-didn't use
and from / originally. The lanes probed the ordinary phrasings an office
actually writes and found the full harm — the disowned date arms a verdict,
blesses SameSunday, or sets keep-through — for: **skip, except, rather than,
old date:, previously, was, used to be, formerly, instead of, cancelled, bare
no, isn't** (and family). The review lane's Q3 verdict: a word list IS the
right mechanism (it is this machinery's universal pattern, and the
one-analysis architecture is correct); the list is simply incomplete in its
own two families.

Disposition (R12-A): **the governor families are completed with the attested
ordinary set.** Negation/cancellation: skip, except, rather than, instead of,
cancelled/canceled, no (standing as the group's governing word), isn't /
aren't / wasn't / weren't, never mind. Origin/past: previously, formerly,
used to be, was (as the past marker in "was November 1" shapes), old date.
Each attested phrasing is pinned on every consuming path (relative, absolute,
paren-only, Keep-through trailing, KT fallback). The standing risk of any
list — a novel phrasing — is accepted as the mechanism's known limit, as Q3
adjudged; the list now covers the attested ordinary set, and lanes keep
hunting.

### R12-2 (MAJOR, red team RT12-2; review R12-4 graded the scope half MINOR — the coordinator takes MAJOR on the red team's evidence that the asserted date appears in NO field)

A governor currently scopes over the whole rest of its group. "(ignore
November 1, use November 8)", "(not November 1, November 8)", and "(moved
from November 1 to November 8)" all silently lose the asserted Nov 8: not
contributed, not an alternative, not a second date — nowhere.

Disposition (R12-A): **a governor scopes to the one date it governs.** A date
asserted later in the same group — after a comma pivot, "use", or the "to"
of "moved from X to Y" — is read on its own terms: it contributes, or is
named as an alternative/second date, under the group's other rules exactly
as if the governed date were absent. "(moved from November 1 to November 8)"
reads as what it is: Nov 1 the origin, Nov 8 the assertion (the "moved to"
shape in compound form).

### R12-3 (MAJOR, red team RT12-3): plain candidates never dedupe — the multiplication survives for plain end records

2,900 PLAIN end records naming one 8,191-entry directory take 113.0 s to
refuse against a 228 ms control (300 candidates: ~10 s). Every prior storm
fixture used zip64-shared candidates, which dedupe. Plain candidates do not:
the DirKey's `dir_end` is each plain candidate's own end-record position, so
no two plain candidates ever share a key, and the enumeration re-walks the
directory per candidate.

Disposition (R12-B): **candidate identity is the directory described, not
the end record's position.** The dedupe key for both the enumeration and the
fallback is (directory start, directory size, zip64 record position where
present). Two plain end records at different positions declaring the same
directory are the same candidate for walking and opening. The plain-2,900
and plain-300 fixtures land in the control's time class; the zip64 families
keep their outcomes and classes.

### R12-4 (MAJOR, review R12-1): the DeclaredKey memo transfers resolutions between candidates — its stated mechanism is false

R11-B's addendum claims "identical declared fields resolve identically."
They do not: resolution depends on the candidate's position (`dir_end`), so
the memo can serve one candidate another's resolution. Proven by
construction: a crafted file loads with no decoy and with a 1-byte-varied
decoy, but is REFUSED with an identical-fields decoy standing before the
honest candidate (the memo hands the honest candidate the decoy's failed
resolution). The behavior predates R11-B, but the batch's stated mechanism
does not hold, and the refusal is real.

Disposition (R12-B): **the memo never serves a resolution across
candidates.** Either the memo key contains everything resolution depends on
(including the candidate's position — which makes cross-candidate service
impossible by construction), or the memo goes away in favor of the shared
resolution in R12-5. The crafted decoy file loads the honest bulletin.

### R12-5 (MAJOR, review R12-2): fallback resolution costs O(distinct declared descriptions × span)

With the search bound deleted (R11-B), each distinct declared description
pays a whole-span resolution search: 0.79 s / 2.47 s / 10.95 s for 1 / 2 / 4
MB distinct-key files, against 6.5–42 ms for identical-key files. Distinct
declared fields are free for a sender to vary, so the cost is
sender-controlled.

Disposition (R12-B): **resolution is computed once per load, not once per
candidate.** The fallback resolves its candidates the way the enumeration
does — one shared pass over the span computing every candidate's resolution
— so total resolution cost is O(span + candidates), independent of how the
declared fields vary. The distinct-key fixtures land in the identical-key
class (sub-second at 4 MB). Together R12-3/R12-4/R12-5 make one statement:
a load's directory work is bounded by the directories actually in the file,
each resolved and walked once, however the end records describe them.

### R12-6 (MAJOR, red team RT12-4): a plain line naming two dates reads as naming none

"October 25, 2026 and November 1, 2026" on a plain For Sunday line — and
even the same date written twice — reads NoSundayNamed, and the review's
first line says "These words do not say which Sunday they are for," which is
false: the words name two specific Sundays, and Accept stands unarmed.

Disposition (R12-A): **first-date primacy, the rest noted.** The framed
reader takes the first framed date as the line's date (the standing
first-date primacy of R11-11) and the remaining distinct dates join the
second-date note, exactly as the whole-line scan treats them elsewhere; the
same date written twice reads as that date, plainly. NoSundayNamed is
reserved for lines that name no date.

## The MINORs

### R12-7 (MINOR, review R12-5 + red team RT12-5): governors after the date, and dates inside nested groups

(a) "(November 1, cancelled)" contributes Nov 1 — a trailing cancellation in
the same group is never read. (Red team's held case "(November 1, not
October 25)" reads Nov 1 correctly: the trailing "not" governs the date
after it. The gap is a trailing governor with no second date to govern.)
(b) "((November 1))" standing alone reads NoSundayNamed: the balanced-paren
split makes the nested group one group, but its content scan never sees
through the inner parentheses.

Disposition (R12-A): (a) a trailing governor word in the same group clause
(cancelled, no longer, not) with no later date to govern disqualifies the
date it follows. (b) The group content reading sees through balanced inner
parentheses: the inner content is the group's content for date-reading
purposes.

### R12-8 (MINOR, cross-lane): the shared-directory timing pin's budget is load-derived and fails under contention alone

Both audit lanes hit it (load 8.4–13; outcomes always passing). A pin that
fails when the shared machine is busy teaches lanes to ignore red runs.

Disposition (R12-B, test-only): the pin's budget gains a fixed floor per
phase (the budget becomes max(control-multiple, control + a floor scaled to
the candidate count)), sized so contention alone cannot fail it while a true
multiplication (minutes) still does. The pin's outcome assertions are
unchanged.

### R12-9 (MINOR, red team RT12-6 + RT12-7, review R12-7): note accuracy and tolerance in three places

(a) A second "Sunday after" phrase's resolution is never noted, and a
restated base is noted as "another date" — the note machinery recognizes
only the first phrase and treats a base restatement as a distinct date.
(b) A lying zip64 total refuses an otherwise honest bulletin via the reader,
while the count walk counts the same file fine — the reader's strictness
and the walk's leniency diverge on `cd_size`/total lies.
(c) The R11-A addendum's wording outruns its census ("provably"; the
residual classes can change the printed attribution words, which the
addendum does not mention).

Disposition: (a) (R12-A) the Sunday-after phrase consumes its run wherever
it stands in the line, and a run equal to the standing base is a restatement,
never "another date". (b) (R12-B) the reader path tolerates a lying zip64
total the way the walk does; the honest bulletin loads. (c) (R12-C, docs)
addenda are dated records and are not rewritten; this synthesis records the
correction — the R11-A residuals are *measured*, not proven, and the credit
residual's reach includes the printed credit line — and FORMAT.md is checked
for the same overreach and corrected where found.

### R12-10 (MINOR, review R12-6): a gap after the directory closes the gate composition

A gap AFTER the directory (which the zip reader tolerates) is refused once
more than 64 candidates close the small-count branch of the reader gate.
Another composed dead zone in the enumeration × fallback × reader chain,
smaller than R11-3's.

Disposition (R12-B): the lane's construction loads. The reader attempt's
conditions are revisited as part of the R12-5 shared-resolution work so the
chain has no composed dead zone of this family: whenever a directory the
app's own walk counts as walkable stands in the file and nothing has
yielded, the reader attempt is reachable.

### R12-11 (MINOR, review R12-8): a dropped duplicate pairing is accepted silently

R11-D's first-pairing-wins is adjudged right (Q4: the value is stored once
per name, so the alternative winners are undefined or recreate the desync).
But when the second pairing was the intended one, the office sees a silently
accepted save — names under the first target, a blank row on the intended
one, and no note of any kind.

Disposition (R12-C): **the save speaks.** When a migration does not follow a
duplicate pairing (or a removal of an already-paired name), the status names
it in plain words in the app's voice — which name, which pairing was kept,
that the other was not followed — so the office can correct it by hand. The
accounting itself is unchanged.

## Fix batches

- **R12-A (dates):** R12-1 (governor families completed) + R12-2 (governor
  scopes to one date; later asserted dates read) + R12-6 (two framed dates:
  primacy + notes) + R12-7 (trailing governors; nested-group reading) +
  R12-9(a) (Sunday-after consumption and restatement in the notes).
- **R12-B (loader):** R12-3 (identity = the directory described) + R12-4
  (no cross-candidate memo service) + R12-5 (one shared resolution pass) +
  R12-10 (gate composition) + R12-9(b) (zip64-total tolerance) + R12-8
  (timing-pin floor, test-only).
- **R12-C (small print):** R12-11 (the save speaks on an unfollowed duplicate
  pairing) + R12-9(c) (FORMAT.md precision check).

Each batch appends its dated Shipped addendum here with real numbers. Round
13 audits the fixes; the exit bar is unchanged.

## Shipped addendum, R12-A

_(appended by the R12-A builder with real numbers and flagged pin changes)_

## Shipped addendum, R12-B

_(appended by the R12-B builder with real numbers and flagged pin changes)_

## Shipped addendum, R12-C

_(appended by the R12-C builder with real numbers and flagged pin changes)_
