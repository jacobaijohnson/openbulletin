# Round 16 audit synthesis — 2026-10-17

HEAD audited: `4570461` (round-15 close). Lanes: verify `0fcc0142`
(report `~/workspace/openbulletin-audit-wave3/round16-verify.md`),
review `395b71e9`, red team `ccc8a91b`. Verify: **PASS, 8 of 8
claims**; CI and bundle for the close push were in progress at its
check (macOS and the bundle gate already success; conclusions land
with round 17's verify). Review **0 BLOCKER / 2 MAJOR / 4 MINOR**;
red team **0 BLOCKER / 5 MAJOR / 1 MINOR**. **The exit bar is not
met.** Deduped: **0 BLOCKER / 4 MAJOR / 4 MINOR**.

## Findings and dispositions

**R16-1 (MAJOR; review R16-1 + RT16-1/2/3).** The to-target rule
fires only for the telegraphic "(moved to November 8)". The
natural forms silently lose the named target on every path: the
passive "(it was moved to November 8)" ("was" itself governs the
target as a past marker because its next token is not "to"), an
object between verb and "to" ("(moved the potluck to
November 8)"), "moved back to", and the postpone targets —
"(November 1, postponed to November 8)" loses BOTH dates with no
note anywhere. Disposition: the target rule works at group level,
not token adjacency — a group containing a displacement or
postpone phrase and a "to"/"until" + date later in the group (in
the phrase's own clause or a following one, across auxiliaries,
objects, or "back") reads that date as the target: it asserts,
and an earlier date in the group is governed by the phrase and
named on the second-date note. Witnesses pinned on all paths.

**R16-2 (MAJOR; review).** A prospective clause inside a
between-dates run breaks R15-3's run-spending: "(November 1, not,
drop it, November 8)" asserts Nov 1 and kills Nov 8 unnamed;
"(November 1, postponed, not, November 8)" and "(November 1, drop
it, not, November 8)" kill both dates in total silence — the
prospective word itself reads as an unspent governor of the
second date. R15-3's harm, one word wider. Disposition: clauses
between the two dates that contain no date token and belong to the
governor families (cancellation, negation/prospective-without-a-
date, filler) spend together on the first date; the second
asserts. (A prospective clause carrying its own date — "use
November 8" — is the second date's own clause and already
behaves; held.)

**R16-3 (MAJOR; RT16-4 + review R16-3).** The auxiliary-spent rule
is a five-word list scoped to single displacement stems, and the
review lane states plainly the boundary is arbitrary: "(it was
cancelled, November 1)" governs while "(it was moved,
November 1)" contributes — no linguistic difference; "(they are
moved)", "(it was being moved)", "(it has moved)", "(it was just
moved)" all govern through gaps in the list; and "(it was
postponed, November 8)" loses its elliptical new date while the
moved twin lands its date. Disposition: auxiliaries as a class
(be-forms incl. are/being, have-forms, get-forms, with one adverb
of tolerance between auxiliary and participle), and the spent
rule covers ALL governor participles — cancellation and
displacement, single and phrasal ("it was put back"). A passive
predicate is spent on its subject whatever the verb; the
following date is the group's occasion date, the held garnish
reading. The harm asymmetry decides the direction: governing
loses a date silently (the loop's MAJOR class); spending reads it
as garnish before a human reviewer (the MINOR class).

**R16-4 (MAJOR; RT16-5).** The run/governor composition does not
reach plain header lines: "November 1, 2026, postponed, drop it,
November 8, 2026" ARMS November 1 with November 8 merely noted,
on For Sunday and Keep-through — although the same words in a
parenthesized group assert November 8, and single-date plain
lines govern. Disposition: the plain-line two-date reading
applies the same clause/run rules as the group reading (one
composition, both surfaces).

**R16-5 (MINOR; review R16-4).** Displacement noun labels (usher
shift, parking bump) suppress per the accepted fence, but in the
between-dates shape the suppressed date drops UNNAMED — the
R15-6a cost class, not the garnish class. Disposition: a date
governed between dates is named on the second-date note whatever
family governed it (the naming R15-6a gave the trailing case
extends to the between-dates case).

**R16-6 (MINOR; review R16-5).** The 28-day rescan flip is
inherent, but `still_waiting` trusts a filed copy's re-parsed
date over the Sunday in its filename, so a crossed bound can make
a filed part count for the wrong Sunday. Disposition: for filed
copies, the filename's Sunday is the record (the file was filed
FOR that Sunday); the re-parse may not override it.

**R16-7 (MINOR; review R16-6).** The genitive filler rule is
clause-local: comma-splitting "(ignore, the date of, the service,
November 1)" arms the date under an explicit "ignore".
Disposition: when a filler clause ends in "of", the reach-back
also crosses a following determiner-headed clause (determiner +
nouns, no date, no verb-form) that completes the genitive.

**RT16-6 (MINOR).** Determiner-headed -s noun phrases suppress:
"(the drops, November 1)", "(the cough drops, …)". A new subclass
of the accepted -s class, but distinguishable by the determiner,
per the R14-5 wont precedent. Disposition: an -s stem word
immediately preceded by a determiner (the, a, an, these, those)
does not govern in its -s form. (Bare -s nouns — ticket drops,
choir drops, dance moves, speed bumps — remain the accepted
class, named in the lane report.)

Recorded, no finding: the surface-5 harmful-flip witnesses (inside
the bound, an anchored read can arm a past Sunday) are the
accepted R15-4 residual; two-file reconciliation before
application remains partial by design (fingerprint claims act
after application).

## Batches

- **R16-A** (governors): R16-1, R16-2, R16-3, R16-4, R16-5, R16-7,
  RT16-6.
- **R16-B** (app): R16-6 (still_waiting filename trust).

Round 17 audits the fixes. The Tier-2 final gate runs only after
a round returns zero BLOCKER + zero MAJOR.
