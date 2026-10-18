# Round 22 synthesis (2026-10-18)

Lanes over the round-21 close `e103cb6` (758 tests): verify PASS
0/0/0; review 0/5/1; red team 0/8/2. Reports:
`~/workspace/openbulletin-audit-wave3/round22-{verify,review,redteam}.md`.

Deduped: **0 BLOCKER / 8 MAJOR / 2 MINOR.** The exit bar is not
met; the Tier-2 final gate stays queued.

Coordinator adjudication on RT22-10 (rights): identical to
rounds 20/21 — 23/3,000 permutation flips, all license-only in
effect; zero flips where children stands; route agreement
100,412 comparisons, zero safety flips. MINOR restatement of
the known residual; no fix, census stands (third consecutive
round the safety axes held under a dedicated hunt).

Held and worth naming: the review lane's composition census
(11 new core families × 7 group shapes, progressive and passive
cores included) returned **zero For/Keep divergences**, and the
red team's verdict invariants held over 30,000 lines × 5
bulletins. The composition's spine is no longer where the
findings live; this round's are separators, fences, and word
classes at its edges.

## MAJORs and dispositions

**R22-1 (RT22-1 + RT22-2) — separators between groups break
the For path.** Groups-only lines whose groups are separated
by `;`, `-`, or any plain word (even filler "note") read
nothing silently on For Sunday while Keep asserts the last
group (882 fuzz divergences, all for-none/keep-some); and a
comma or semicolon after a leading group destroys the For
line's relative memory entirely (verdict NoSundayNamed where
the space control arms). Disposition: separators are
separators — groups compose across `;`, `-`, commas, and
intervening plain words exactly as adjacent groups do, on
both paths; separator punctuation after a group never
destroys the in-words memory. The fuzz family goes to zero
outside the standing relative-line asymmetry.

**R22-2 (review F3 = RT22-3) — a sender's semicolon breaks
governance.** "November 1, 2026; cancelled, November 8,
2026" asserts the cancelled November 1 (comma twin governs);
the attached form "November 1, 2026; cancelled, November 1,
2026" resurrects it on both paths. Disposition: a semicolon
is a clause break everywhere governance, governors, and
composition are judged, attached or spaced — the comma
twin's reading in every witness. (The composition's
internal group marker is already a semicolon token; sender
semicolons must be indistinguishable from it.)

**R22-3 (RT22-4) — a leading group bypasses the wrong-weekday
check.** A framed date whose weekday is wrong yields no date
— except behind a leading group, where it asserts on both
paths with no mismatch record. Disposition: the weekday
check applies to every framed occurrence however the line
is composed; no segment position exempts it.

**R22-4 (review F1 + RT22-5 + RT22-9) — the auxiliary fence
family.** "(moved November 1, then was played to November 8)"
is total silence: `target_claim`'s between-words fence kills
the claim AND bare "was" is itself a `direct_single_governor`,
so November 8 is governed, not merely unclaimed. Every other
auxiliary, and "was" with a subject, lands on November 8
alone. The same fence breaks "choir was loudly singing"
(any unlisted -ly adverb between auxiliary and participle
silences both dates). Disposition: in the claim reach, a
subjectless auxiliary + participle construction is a
second clause, not a fence: the claim dies there, the date
after it stands on its own standing (November 8 alone,
nothing named), and the auxiliary does not govern it. An
auxiliary followed by at most one adverb of any kind and
then a participle/-ing form, with a subject before the
auxiliary, is a finite clause for the reach's test.
Held: the claim's own passive constructions (auxiliary +
adverb walk spending) keep their standing readings.

**R22-5 (RT22-6 + RT22-7) — subject prepositional phrases.**
"the team from Kent" as the second clause's subject silences
both dates for all 23 verb forms; "the choir on Sunday"
fails three ways by verb class (phantom October 18 asserts,
alternatives naming the phantom, false from-date naming).
Disposition: a subject run crosses a prepositional phrase —
determiner/noun head + preposition + its own noun-like run
counts as the subject, and the phrase's words are consumed
wholly (a weekday inside a subject PP spawns no signal and
is named nowhere). All 23 forms read November 8 alone.

**R22-6 (review F2) — the determiner shield stops one word
early.** "(moved November 1, and the big left luggage to
November 8)" and "the big sung hymns" kill the claim where
the regular twins claim: the shield covers only the word
immediately before the verb form. Disposition: within its
noun phrase, a verb-form adjective is shielded by a
determiner standing anywhere before it (intervening
adjectives included), the R19-6 active-for rule applied to
the reach: "the big left luggage / the big sung hymns"
claim like "the big baked beans".

**R22-7 (review F5) — the lone-framed fence overreaches.**
R21-B's fence (a lone framed date may assert beside a
weekday-modifier phrase) triggers on ANY weekday+noun
window anywhere: "Monday meeting, service November 8,
2026", "service November 8, 2026, Monday meeting", and
"Tuesday choir, November 1, 2026" all assert, against the
held rule that lone content-headed lines read nothing.
Disposition: the fence requires the weekday-modifier phrase
to be the line's only content beside the framed date — no
content-headed segment (service/choir/meeting heads and
their kin) anywhere on the line; the intended Sunday
shapes ("Sunday bulletins, November 1, 2026", "Sunday
school, November 8, 2026") keep asserting, and governed
lines under the fence keep their governed readings.

**R22-8 (review F4 + RT22-8 + review F6) — inventory,
function classes round three.** Resurrecting classes:
remaining prepositions led by "with" (280 fuzz
resurrections, all via "(with November 1)"), "as per"/"as
of", determiner/quantifiers (each, every, both, all, same,
and the lanes' full list), possessive + "own", the review
tail (lest, albeit, per, via, amid, among, within, upon,
unto, despite, unless, whereas, hence, thus, moreover),
and head-position "too" (bare trailing, content in head —
"(too November 1 too)" asserts). Disposition: those
classes join the bare inventory; "too" is bare in both
slots. The inventory's stated rule stands: function words
are bare, content labels are not; "thanks" in head position
stays content per the R20-3 adjudication.

## MINORs

- **R22-9 (RT22-10)** — adjudicated above; no fix.
- Review F6 is folded into R22-8 (the "too" parity).

## Fix batches

- **R22-A** (composition separators + fences): R22-1,
  R22-2, R22-3, R22-7.
- **R22-B** (reach + inventory): R22-4, R22-5, R22-6, R22-8.

Held lines: the R19–R21 families, defensible silence,
claim/scope pins, the standing relative-line asymmetry,
content labels assert, rights pin, loader pins, booklet
bytes 31,717 / 34,977. Flag any changed pre-existing test
loudly by name with its reason.
