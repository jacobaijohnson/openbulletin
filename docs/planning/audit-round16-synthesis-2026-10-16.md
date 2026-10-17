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

## Shipped addendum, R16-A — 2026-10-17

Batch R16-A (governors) shipped in `crates/ob-extract/src/
contribution.rs`: R16-1, R16-2, R16-3, R16-4, R16-5, R16-7,
RT16-6. Seven new pin tests, all `r16a_*`, one per finding.
**Tests: 700 → 707 passed, 0 failed** (full workspace suite run
unpiped twice on the final code, cargo exit 0 both times;
ob-extract 189 → 196). **Clippy: clean** (`--workspace
--all-targets`, exit 0, zero warnings). Neither the fake-helper
ETXTBSY flake nor timing contention fired in any run.

**Changed pre-existing tests: none.** No test body was removed
or altered; every round 11–16 held shape stands at its shipped
expectation, including "(it was moved, November 1)", the
pure-run pins, the run-after-the-second-date naming, and the
defensible silence of "(November 1, cancelled, November 8,
drop it)". In particular, the symmetric-spent widening (R16-3)
moved no pin: no pre-existing test asserted that an
auxiliary + cancellation participle governs, so the change the
batch brief braced for did not exist in the tree. Readings that
changed are exactly the dispositions' witnesses, all pinned
new in the `r16a_*` tests.

What shipped, per finding:

- **R16-1 — target rule at group level** (`target_claim`,
  `governed_before_target`). Scope, stated exactly: the group
  contains a displacement phrase (single displacement verb or
  put/push-back pair) or a form of "postpone", and a "to" or
  "until" whose next date — determiners only between — is the
  claimed target. Between phrase and "to" at most four words,
  none of them a governor, a cancellation verb form, an
  auxiliary, or a weekday (the object noun phrase / "back"
  fence). The claim zone extends back from the phrase over
  its auxiliaries and one tolerated adverb, so the passive's
  "was" is part of the construction and does not govern the
  target. A governor standing before the zone still governs
  ("(not moved to November 8)" pins None), and "delayed until"
  claims no target — "delay" is outside the disposition's
  phrase family, as written. Any other distinct date in a
  claimed group is the phrase's from-date: governed and named,
  wherever it stands ("(it was moved to November 8,
  November 1)" asserts November 8 and names November 1).
- **R16-2 — run composition** (`is_prospective_only_clause`,
  `is_governor_family_clause`). Between-dates clauses with no
  date of their own spend together on the first date across
  cancellation, prospective, and filler families, in both the
  window walk and `governed_after`. Judgment call, stated
  plainly: a prospective phrase spends on the first date only
  when it stands as its own COMPLETE clause (a clause break
  closes it inside the segment); a prospective word in the
  second date's own clause still governs that date, so
  "(November 1, postponed, not November 8)" keeps its held
  silence and "(November 1, postponed, use November 8)" keeps
  asserting — both pinned.
- **R16-3 — auxiliary class, symmetric spent**
  (`AUXILIARY_WORDS`, `is_spent_predicate`). The spent rule
  covers every cancellation phrase (displacement included,
  single and phrasal) standing as an auxiliary's predicate.
  The auxiliary set is the disposition's: be-forms (am, is,
  are, was, were, be, been, being), have-forms (has, had,
  have), get-forms (get, gets, got, getting) — "am" and "get"
  included as their families' base forms. The one-adverb
  tolerance set, stated plainly: any -ly adverb, plus just,
  already, still, also, even, now, soon, often; the backward
  scan crosses any number of auxiliaries with at most one
  adverb in total ("it has just been moved" spends; "just
  moved" with no auxiliary does not). The no-clause-break
  parity is held and pinned for both families.
- **R16-4 — plain-line parity** (`plain_composition`, called
  from `plain_multi_date_reading`). Each framed signal is
  judged with the group's own machinery (window, run
  spending, target claim, trailing governors, split phrasal,
  weekday contradiction). Scope: lines whose signals are ALL
  framed. A mixed line (framed beside relative) keeps the
  standing first-signal reading — a relative phrase has no
  token span for the composition to judge; single-date plain
  lines never reach the reading and are unchanged. When some
  signal is governed, the first UNGOVERNED signal reads and
  every other distinct date is named; when all are governed
  the line reads no date, the group's silence.
- **R16-5 — naming between dates.** The governed-date naming
  now collects: a date governed after with a later date
  standing, a date governed before by a governor standing
  after a previous date, and a target-disqualified date.
  Stated limits: a group-initial before-governor keeps the
  standing silence ("(not November 1, November 8)" names
  nothing, as before), and a group where every date is
  governed names nothing (the defensible-silence held line).
  "(November 1, cancelled, November 8)" now names November 1
  beside November 8's assertion — the disposition's "whatever
  family governed it", pinned.
- **R16-7 — genitive across clauses** (`genitive_completion`,
  `clause_span`). When the prior clause is filler ending in
  "of", the reach-back crosses ONE following determiner-headed
  clause (determiner + nouns only; "verb-form" is judged by
  the machinery's own stem families, the only verb notion the
  fence has). A following clause carrying a stem verb-form
  ("the moved service") still stops the reach-back, pinned.
- **RT16-6 — determiner-headed -s nouns**
  (`determiner_headed_s_form`). Applied in `phrase_governs`,
  `governed_after`, and `is_cancellation_only_clause`, so
  before, trailing, and run positions read alike. The
  determiner is found by walking back over at most three
  noun-like words (not verb forms, governors, auxiliaries, or
  weekdays), which is what covers "(the cough drops, …)" as
  the lane graded it, not only "(the drops, …)". Accepted
  cost, stated plainly: the same walk reads a determiner +
  subject-noun + -s verb ("the choir cancels") as
  noun-headed too; the two shapes are indistinguishable to
  this machinery, and the lane's fence precedent (the
  possessive "wont") resolves the ambiguity toward the noun.
  Bare -s forms keep governing, pinned ("(ticket drops, …)",
  "(choir cancels …)").

## Shipped addendum, R16-B — 2026-10-17

Batch R16-B (filed-copy Sunday trust) shipped in
`crates/ob-app/src/main.rs`: R16-6. Two new pin tests, both
`r16b_*`. **Tests: 707 → 709 passed, 0 failed** (full
workspace suite run unpiped twice on the final code, cargo
exit 0 both times; ob-app 175 → 177). **Clippy: clean**
(`--workspace --all-targets`, exit 0, zero warnings).
Neither the fake-helper ETXTBSY flake nor timing contention
fired in any run.

**Changed pre-existing tests: none.** No test body was
removed or altered; the round-3 still-waiting pins
(`still_waiting_counts_only_this_sundays_arrivals`,
`r3_still_waiting_needs_the_same_name_not_a_near_one`,
`r3_an_undated_filed_part_counts_only_for_its_filing_sunday`)
stand at their shipped expectations and pass unchanged.

What was wired, stated exactly: no new filename decoding
was needed — the filing side already encodes the Sunday
(`file_contribution` writes `filed_file_name(from,
for_date.or(filing_sunday), fingerprint)`), and the filed
scan already read it back (`filed_sunday_from_name` →
`InboxItem.filed_for`, round 3, R3-10). The defect was
only in `still_waiting`'s filed branch, which consulted
`filed_for` solely when the re-parse named no date. The
branch now matches on `filed_for` first: when the name
carries a Sunday, that Sunday alone decides (it counts
iff it equals the open bulletin's Sunday; with no bulletin
open it counts, as before), and the re-parsed `for_date`
is never consulted. When the name carries no date (the
older "no Sunday named" filings), the branch falls back
to exactly the previous re-parse arms, with the undated
case's old outcome (`*filed_for == Some(b)`, false when
`filed_for` is None) spelled out as `false`. The
`InboxItem` and `still_waiting` comments now state the
rule for every filed part, not only undated ones. Sibling
check: `still_waiting` was the only logic asking which
Sunday a filed copy is for — `group_inbox` runs only on
the inbox scan, and the applied-part record is
date-independent (R16-5's own tracing) — so no sibling
needed the same change. Unfiled/Inbox copies are
unaffected: they carry no filed Sunday and the parse
rules stand, pinned both at the `still_waiting` level and
through a scan of the same folder as the inbox.

The pins: `r16b_a_filed_part_counts_for_its_filename_sunday_not_its_reparse`
(a contribution re-reading to November 22, 2026 under a
name filed for October 25 counts for October 25 and not
for November 22; an ordinary filed copy whose re-parse
agrees with its name is unchanged; a no-date name falls
back to the re-parse; the same words in the inbox count
for the Sunday they name) and
`r16b_a_filed_copy_counts_for_its_sunday_after_the_bound_crosses`
(the lane's construction end to end: the mixed line
"next Sunday and October 25, 2026" in a file dated
October 16, 2026, filed under an October 25 name, scanned
November 14 — past the 28-day bound — re-parses to
November 22 in the scan itself, yet still counts for
October 25 and not for November 22).

## Coordinator close (2026-10-17)

Both batches shipped and were independently re-verified by the
coordinator (full unpiped suite + clippy -D warnings at each step):
R16-A `0fb1947` (707 tests; group-level target rule, run
composition incl. prospective clauses, auxiliary class with
symmetric spent, plain-line parity for all-framed lines, between-
dates naming, genitive crossing, determiner-headed -s nouns; zero
changed pins; scope calls recorded for round-17 grading: delayed-
until claims no target, mixed framed+relative plain lines keep the
first-signal reading, "the choir cancels" reads noun-headed),
R16-B `e9cfb5b` (709 tests; a filed copy counts for the Sunday in
its filename - the decoding already existed; still_waiting's
precedence was the whole defect; zero changed pins). Round 17
audits these fixes. The Tier-2 final gate runs only after a round
returns zero BLOCKER + zero MAJOR.
