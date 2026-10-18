# Round 19 audit synthesis — 2026-10-17

HEAD audited: `916ae52` (round-18 close). Lanes: verify `759d6208`
(report `~/workspace/openbulletin-audit-wave3/round19-verify.md`),
review `3f734ac9`, red team `312a8ad8`. Verify: **PASS, 8 of 8
claims**, CI green on all three OS jobs (39090506155), bundle
39090508026 SUCCESS, booklet bytes exact, Bach live passing; the
round's one moved pin confirmed as exactly the flagged r12a block.
Review **0 BLOCKER / 3 MAJOR / 4 MINOR**; red team **0 BLOCKER /
2 MAJOR / 5 MINOR**. **The exit bar is not met.** Deduped:
**0 BLOCKER / 4 MAJOR / 7 MINOR**.

## Findings and dispositions

**R19-1 (MAJOR; review Q1).** Governance-by-date overreaches
within a line when two EVENTS share one date value: "(early
service November 1, late service cancelled November 1)" and its
twins read nothing, unnamed, on every surface — the early
service's filing dies because a different event on the same
Sunday was cancelled. Disposition: value governance binds only
occurrences that are BARE restatements of the date (their clause
carries the date and its framing, no event content of its own).
An occurrence heading its own event clause asserts independently.
R18-2's witnesses are all bare restatements and stay silent; the
two-event line asserts November 1.

**R19-2 (MAJOR; RT19-1), as adjudicated.** The red team showed
governance stopping at the parenthesis: "(potluck November 1
cancelled) (service November 1)" asserts the governed value. The
coordinator's adjudication: the two groups name DIFFERENT events
(the potluck was cancelled, the service was not), so November 1
asserting for the service is the correct reading under R19-1's
rule — a content-carrying occurrence asserts independently on
either side of a group boundary. The fixable core is narrower:
a BARE restatement in a later group of a date governed earlier
on the line stays governed ("(November 1, cancelled)
(November 1)" reads nothing), and where two content groups
assert different dates, the line's two-date machinery asserts
one and names the other rather than dropping either.
Disposition: cross-group, value governance binds bare
restatements only; content groups assert independently with
normal naming.

**R19-3 (MAJOR; review Q2).** The anchor asymmetry reproduces
R14-6 in miniature: a relative-only group "(next Sunday)" in a
file dated Oct 16 scanned Oct 23 asserts November 1 (today's
resolution), while its mixed twin asserts October 25 (the
anchor's). Relative-only plain lines assert no hard date at all,
so the group form converted R18-6's former safe silence into a
wrong-Sunday hard date. Disposition: groups resolve ALL relative
signals at the parse anchor (the R18-A judgment call is reversed
on this evidence); relative-only and mixed groups read alike.

**R19-4 (MAJOR; review Q3 + RT19-2).** The finite-verb reach test
shields only the word directly after a determiner: in R18-4's
own witness shape, "(moved the meeting, November 1, 2026, and
baked beans to November 8)" — and canned goods, sliced bread,
printed bulletins — the bare -ed adjective heading the object
continuation kills the claim and the line collapses to
alternatives (the plain twin files for the from-date).
Disposition: in the claim reach, an -ed word immediately
followed by a noun-like word is an adjective, not a finite verb,
determiner or none; a finite-verb reading needs a subject
standing before the -ed word in its clause (the "choir sang"
pattern, whose verb the irregular/stem lists already catch).

**R19-5 (MINOR; review Q3 + RT19-3).** The finite-verb list
misses goes, plans, rehearses, gathers ("go" stands in neither
the stems nor the irregulars). Disposition: the go family
(go/goes/went/gone) and the rehearses/gathers/plans stems join
the verb test.

**R19-6 (MINOR; review Q4 + RT19-4).** The active-for fence loses
any object whose head noun wears a modifier: "the big meeting",
"the church meeting", "the Johnson meeting" all fall silent
("meeting" is meet's -ing form, unprotected behind a modifier),
while bare "the meeting" asserts. Disposition: in the active-for
object scan, a determiner standing before the phrase protects
its head — modifiers between determiner and head do not break
the fence.

**R19-7 (MINOR; RT19-5/6).** Adverb seams: unlisted intensifiers
(very, plain, simply) break a stack in every scan, and a fronted
stack spends where the same stack post-auxiliary governs.
Disposition: very, plain, simply join the closed list, and the
tolerance applies to a listed stack standing before the
auxiliary as it does after it (one rule, both positions).

**R19-8 (MINOR; review sweep).** The R18-7 naming merge leaks:
"the Sunday after November 1, 2026, November 1, 2026, cancelled"
names the Sunday-after phrase's restated base, which the standing
scan deliberately never names. Disposition: composition naming
excludes a relative phrase's consumed base, as the standing scan
does.

**R19-9 (MINOR; review sweep).** R18-11's chain-vs-stack test
rides on any to/until token in the last group: "(November 15,
note to self)" and "(November 15, back to church)" misclassify
as chains and suppress a real garnish mismatch. Disposition:
the test requires an actual target claim in the last group, not
a bare token.

**R19-10 (MINOR; RT19-7).** Bare two-group primacy is last-group-
wins with the loser unnamed: "(November 8) (next Sunday)"
asserts Oct 25, Nov 8 unnamed. Disposition: the unnamed loser of
a two-group reading is named on the note, as on plain lines.

## Batches

- **R19-A** (governance scope): R19-1, R19-2, R19-3.
- **R19-B** (reach/for/adverb/naming edges): R19-4, R19-5,
  R19-6, R19-7, R19-8, R19-9, R19-10.

Round 20 audits the fixes. The Tier-2 final gate runs only after
a round returns zero BLOCKER + zero MAJOR.

## Shipped addendum, R19-A — 2026-10-17

Batch R19-A (R19-1, R19-2, R19-3) shipped in
`crates/ob-extract/src/contribution.rs` only. Tests:
727 passing at baseline (HEAD `ae04dc6`) → **730
passing, 0 failed**, full workspace, unpiped, cargo
exit 0; ob-extract 214 → 217 (three new `r19a_*`
pins). Clippy (`cargo clippy --workspace
--all-targets`): exit 0, zero warnings. Booklet
bytes untouched (no layout code in this batch; the
held 31,717 / 34,977 stand as the verify lane left
them). The fake-helper ETXTBSY flake did not fire;
no timing contention fired (one probe build was
slow under sibling load, ~5 minutes, and completed
— no run was discarded and no finding rests on it).

**The bare-restatement test, in one sentence:** an
occurrence is a bare restatement when its own
region of the line — the words after the previous
occurrence and before it, and the words after it up
to the next occurrence or the end — contains, apart
from the date itself, no word that is not a framing
word (a weekday, "on", "the"), a filler or joining
word, an auxiliary, a tolerated adverb, or part of
a governor phrase. Implemented as
`occurrence_is_bare` / `bare_filler_word`; the
by-date step in both `analyze_group` and
`plain_composition` now requires it, on top of the
standing last-target override.

**R19-1.** "(early service November 1, late
service cancelled November 1)" and its reversed and
party/service twins now assert November 1 on the
relative, paren-only, and Keep-through paths, group
and plain surfaces alike. What the note carries:
nothing — the governed twin is the same value as
the asserted date, so there is no distinct second
date to name (`second_dates` is empty on every
path). One mechanism the disposition's reading
required, stated plainly: the second clause's
"cancelled" was reaching the first occurrence
through `governed_after` as a between-dates
governor. `governed_after` now leaves a
cancellation phrase alone when it stands in a
clause of event content that runs directly into
the next date (no clause break closes it) — that
cancellation is the next date's own clause
governor, judged by that date's window. The
standing between-dates rule is unchanged where the
phrase's clause is closed by a break or is itself
a governor/filler clause. Round 18's bare
witnesses stay silent, including the moved r12a
pin at its round-18 reading ("(not November 1,
November 1)"), the Oct 23 relative restatement,
and a restatement wearing only "on".

**R19-2.** Cross-group, same rule, implemented as
a line-level pass (`trailing_group_analyses`) over
a line's trailing groups: each group now reports
the values it governs directly and whether its
contribution stands only on bare restatements, and
a bare contribution of a value a sibling group
governs is suppressed. "(November 1, cancelled)
(November 1)" reads nothing on all three paths,
framing variants ("(on November 1)", "(the
November 1)") included, in either group order
(judgment call, stated: the cross-group rule is
order-independent, as the within-line rule is —
a governor standing after a bare restatement binds
it inside one group, e.g. "(November 1, November
1, cancelled)", and the same reading is taken
across the parenthesis). Content groups assert
independently per the adjudication: "(potluck
November 1 cancelled) (service November 1)"
asserts November 1 for the service on all paths.
The different-dates construction is pinned as it
reads: "(November 1, cancelled) (service November
8) (service November 1)" asserts November 1 (the
standing paren-only primacy — the last group
stands) and names November 8 on the
parenthesized-mismatch record (Keep-through mirror
included); on the relative path both asserted
group dates are remembered ([November 8, November
1]). Neither asserted date is dropped unnamed. A
claim's target in its own group is never
suppressed ("(November 1, cancelled) (moved to
November 1)" asserts), and a suppressed bare group
leaves a clean sibling standing ("(November 8)
(November 1, cancelled) (November 1)" asserts
November 8). The plain core is not a sibling
group: the standing core/group separation is
unchanged, so the review lane's X3 bound
("November 1, 2026 (November 1, cancelled)"
asserts) stands.

**R19-3.** `analyze_group` resolves every relative
signal at the parse anchor (`anchored` is applied
unconditionally; the mixed-only gate and the now
unread `framed` field on the group signal scan are
removed). "(next Sunday)" and "(choir picnic next
Sunday)" in a file dated October 16, scanned
October 23, contribute October 25 on all paths;
"(this Sunday)" anchored contributes October 18;
a Sunday-after group over a relative base anchors
too ("(the Sunday after next Sunday)" → November
1 at that anchor), while a framed-base
Sunday-after keeps its framed-base resolution
("(the Sunday after November 1)" → November 8). A
lone "(Sunday)" group stays weekday garnish; the
Oct 30 merge ("this Sunday" = the governed
November 1) holds; the 28-day bound holds in both
directions for groups (file September 25 anchors —
"(next Sunday)" → October 4; file September 24
falls back to the unanchored November 1). Plain
relative-only lines are unchanged.

**Changed pre-existing tests: none.** The full
suite passes with zero modifications to any
pre-existing test — the flag the task asked for is
an empty list, verified by the unpiped workspace
run (730 = 727 baseline + 3 new). The fear behind
the flag did not materialize: every pre-existing
pin that exercised value governance carries a bare
restatement in the governed position. Judgment
calls: (1) the cross-group rule is order
independent (above); (2) in the bare test, "for"
counts as a joining word only — R18-5's
for-composition fences are judged elsewhere and
were not disturbed; (3) a "use" prospective
occurrence is not bare ("use" is neither a governor
nor a filler word in the test), so it asserts
across a group boundary by its own content, beside
the claim override. Known limit, unchanged and
out of this batch's dispositions: a governed plain
core beside a contributing trailing group still
splits by path ("November 1, 2026, cancelled
(November 1)" reads nothing on For Sunday, asserts
on Keep-through) — the core/group separation the
R19-2 adjudication left standing.

## Shipped addendum, R19-B — 2026-10-17

Batch R19-B (R19-4 through R19-10) shipped in
`crates/ob-extract/src/contribution.rs` only.
Tests: 730 passing at baseline (HEAD `9c8d650`,
the R19-A close) → **737 passing, 0 failed**,
full workspace, unpiped, cargo exit 0; ob-extract
217 → 224 (seven new `r19b_*` pins, one per
disposition). Clippy (`cargo clippy --workspace
--all-targets`): exit 0, zero warnings. Booklet
bytes untouched (no layout code in this batch;
the held 31,717 / 34,977 stand). The fake-helper
ETXTBSY flake did not fire. Timing contention
did fire, in the mild form: two directory-size
refusal tests in another crate ran over 60
seconds under sibling load in both full runs and
passed; no run was discarded and no finding rests
on a timing measurement. Every witness below was
also probed against the pre-fix code in a scratch
harness before the fix and re-probed after: the
before/after diff of the full witness set is
exactly the changes described and nothing else.

**R19-4, the -ed test, in one sentence:** an -ed
word whose only claim to being a finite verb is
the generic -ed suffix counts as one in the claim
reach only when no noun-like word stands
immediately after it (else it is an adjective
heading that noun phrase, determiner or none) and
a noun-like subject stands before it in its
segment; the listed stems and irregulars are
unambiguous verb forms and are untouched by the
tempering. "(moved the meeting, November 1, 2026,
and baked beans to November 8)" — and the canned
goods, sliced bread, and printed bulletins twins —
now claim November 8 with November 1 named, on
the relative, paren-only, and Keep-through paths,
group and plain, as their determiner twins
already did; the plain twin asserts November 8
instead of filing for its from-date. Held inside
the same pin: "(moved November 1, choir sang to
November 8)" still claims nothing (sang, through
the irregular list), and "(moved November 1, team
played to November 8)" and "(… choir practiced
…)" still end the reach — an -ed verb with its
own subject standing before it is a verb.

**R19-5.** `FINITE_VERB_STEMS` 46 → 50 (go, plan,
rehearse, gather join) and `IRREGULAR_FINITE_FORMS`
57 → 58 ("gone", the go family's participle the
stem rules cannot build; "went" was already
there, and "goes" is go's -es form). "(moved
November 1, choir goes to November 8)", "(… team
plans …)", "(… choir rehearses …)", "(…
congregation gathers …)" end the reach like sang
does: the claim dies, the note is empty, and
November 8 keeps its own standing per the R18-8
reading.

**R19-6.** The active-for object scan gets its
own verb test (`active_for_object_has_finite_verb`):
a determiner or possessive standing anywhere
before the verb-form word in the object protects
it, so "(we moved the big meeting for November
1)", "(we moved the church meeting for November
1)", "(we moved the Johnson meeting for November
1)", and "(we moved a big meeting for November
1)" contribute November 1, as the bare heads
already did. Judgment call, stated: the widening
is scoped to this scan — the claim reach keeps
the immediate-determiner rule, because there a
determiner-headed subject plus its verb ("the
choir sang") is a clause, not a noun phrase. The
fences stand, pinned: bare active parity governs
("(we moved the meeting November 1)" is silent),
and active cancellation with "for" stays silent.

**R19-7.** very, plain, and simply join
`is_tolerated_adverb` — the closed list is 27
words. "(it was just plain moved, November 1)",
"(it was recently plain moved, November 1)",
"(it was just simply moved, November 1)", "(it
was very recently moved, November 1)", the
has-been form, and the passive-for twins now
spend in every scan; "(it was just plain moved
to November 8)" claims its target (before this,
the unlisted stack left "moved" outside its own
claim zone, and the target was governed by it).
The fronted position needed no scan change, and
that is a judgment call stated plainly: judged
against the machinery, all three scans traverse
backward from the participle or phrase and cross
the auxiliary first, so a listed stack standing
before the auxiliary is crossed and tolerated
exactly as one standing after it — the list was
the only thing the fronted position was missing
in the target-claim scan, and the pins now lock
both positions in all three scans (spent
predicate, passive-for, target claim), fronted
whole-clause ("(just plain, it was moved,
November 1)") and pre-auxiliary ("(it just plain
was moved, November 1)") alike. Held: an unlisted
word in the post-auxiliary slot still does not
spend ("it was family moved"), and three stacked
listed adverbs there still break the tolerance.

**R19-8.** The R18-7 naming merge in `date_line`
now filters the composition's named dates against
`sunday_after_consumed_bases` — the bases of the
core's Sunday-after phrase runs, read with the
second-date scan's own run scan. "the Sunday
after November 1, 2026, November 1, 2026,
cancelled" asserts November 8 and names nothing,
on both lines; the salvaged reading itself is
unchanged. The merge still names what it exists
to name: the R18-7 witness ("cancelled, next
Sunday, November 1, 2026") still names October 25
— a governed relative is no phrase's base.

**R19-9.** `chain_superseded_dates` replaces its
last-group bare-token test with
`group_has_target_claim`: the last contributing
group must carry an actual target claim for the
read date — a `target_claim` judged inside the
group, or the chain's continuation link standing
in it (a "to"/"until" before the read occurrence
with nothing between it and the date but
determiners and the link's lead-ins "then"/"and",
and no other date between). "(moved to November
8) (November 15, note to self)" and "(… November
15, back to church)" are garnish stacks again:
November 15 asserts, nothing is named on the
note, and the mismatch naming November 8 is no
longer suppressed, on the For Sunday and
Keep-through records alike. The true chain family
still names its superseded targets, pinned in the
"(then to …)", "(to …)", "(until …)", and "(moved
to …)" last-group forms.

**R19-10.** In the paren-only branch of
`date_line`, a two-group reading names its loser
on the note when at least one of the two groups
carries a relative phrase (`group_inner_is_relative`):
"(November 8) (next Sunday)" asserts October 25
AND names November 8 on the note, the mismatch
record standing as it did; the reversed order
names October 25 beside November 8's assertion;
the Keep-through mirror names the loser the same
way. Scope, stated as a judgment call: the bare
framed stack is not this shape — "(November 1)
(November 8)" keeps the r18b pin's reading
exactly (its loser named on the mismatch record
alone, nothing on the note), because that pin is
a held line and its naming channel already
exists; what R19-10's witness lacked was any
naming on the note for a mixed framed/relative
pair, and the fix is scoped to that pair shape
(exactly two contributing groups, one relative).

**Changed pre-existing tests: none.** The full
suite passes with zero modifications to any
pre-existing test — verified in the diff (every
removed line is production code or its comments)
and by the unpiped workspace run (737 = 730
baseline + 7 new). Held lines confirmed standing
by the probe diff and the suite: R18-8's lone-date
standing, the claim/scope pins ("(not moved to
November 8)", "(delayed until November 8)"), the
defensible-silence pins, the R19-A family, the
determiner twins, and the bare-stack mismatch
control.
