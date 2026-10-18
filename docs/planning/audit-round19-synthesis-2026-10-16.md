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
