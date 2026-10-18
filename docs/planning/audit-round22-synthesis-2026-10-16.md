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

## Shipped addendum, R22-A (2026-10-18)

R22-1, R22-2, R22-3, and R22-7 shipped in one batch, all
in `crates/ob-extract/src/contribution.rs`:

- **R22-1.** A groups-only line whose de-grouped core
  carries no signal of its own now composes in
  `whole_line_composition` exactly as the paren-only
  composition — last contributing group stands, earlier
  groups on the mismatch record, relative losers on the
  note — so groups separated by `;`, `-`, commas, or
  plain words (even filler "note", even "cancelled,")
  read identically on the For Sunday and Keep-through
  paths. The relative-shape readers are separator-
  transparent: a separator token left standing where a
  leading group was taken off cleans to no word of the
  shape, and the strip loop skips separator punctuation
  between groups, so "(November 1); next Sunday" and
  "(November 1), next Sunday" keep the in-words memory
  (and the arming verdict) of their space twins.
- **R22-2.** `parse_date_prefix` reads a date token
  without a trailing `;`/`:` — a sender's semicolon
  attached to the year ("2026;") is a clause break
  standing after the date, never part of it. The year
  stays inside the date's own span, the governor scope
  behind it is the comma twin's, and a bare restatement
  binds by date value: "November 1, 2026; cancelled,
  November 8, 2026" asserts November 8 with November 1
  named; the same-date restatement reads nothing. A
  token of nothing but the punctuation (the
  composition's own standalone `;` group marker) keeps
  its shape and stays a boundary no date run may start
  on. The r21a pins stand (see below).
- **R22-3.** `date_line` judges the de-grouped core
  segment by segment with the standing inner reader
  before any composed reading: the first segment whose
  framed date disagrees with its weekday records the
  mismatch and yields no date, on both paths —
  "(November 8) Monday, November 15, 2026" now matches
  its group-less twin. Correct-weekday framed dates
  behind leading groups assert as pinned in R21-A.
- **R22-7.** The lone-framed fence
  (`lone_framed_fence`) fires only when every clause
  of the line beside the framed date is a weekday-
  modifier phrase (weekday + noun-like window, noun-
  like words and determiners only) and no content head
  (`content_head_word`: service/choir/meeting/dinner
  and their kin) stands anywhere on the line. "Monday
  meeting, service November 8, 2026", "service
  November 8, 2026, Monday meeting", and "Tuesday
  choir, November 1, 2026" read nothing; "Sunday
  bulletins, November 1, 2026", "Sunday school,
  November 8, 2026", and the r21b shapes ("printed
  Monday bulletins", "the Sunday bulletins", "Sunday
  morning") keep asserting, and governed lines under
  the fence keep their governed readings.

Tests: five new pins (`r22a_separators_between_groups_
compose`, `r22a_separator_after_a_leading_group_keeps_
relative_memory`, `r22a_a_senders_semicolon_is_a_
clause_break`, `r22a_wrong_weekday_behind_a_leading_
group`, `r22a_lone_framed_fence_needs_a_content_free_
line`), every witness on both paths. Workspace suite
758 → **763 passed, 0 failed** (cargo exit 0, full
unpiped run); clippy `--workspace --all-targets` exit
0, no warnings. **No pre-existing test changed.**
Two pre-existing failures surfaced mid-batch and were
root-caused to this batch's own edits, then fixed in
the edits rather than in the tests: the first fence
draft required the modifier phrase's weekday to agree
with the date, which broke the r21b pin "printed
Monday bulletins, November 8, 2026" (the fence is
content-based, not agreement-based, as shipped); and
the first semicolon normalization emptied a standalone
`;` token, letting a date run start on the
composition's group marker and breaking two
`r21a_post_group_plain_segments_compose` witnesses
until the marker was fenced out of the normalization.
Booklet bytes untouched (no export code in this batch).

## Shipped addendum, R22-B (2026-10-18)

R22-4, R22-5, R22-6, and R22-8 shipped in one batch, all
in `crates/ob-extract/src/contribution.rs`:

- **R22-4.** Clause detection in the reach is now
  shared: `aux_complement` finds an auxiliary's
  complement verb form across the tolerated adverbs
  (as before) plus at most one adverb of any kind
  (an unlisted -ly adverb, "not", "never"). A
  subjectless auxiliary + participle construction
  ("then was played", "then was singing") is a
  second clause — `phrase_governs` no longer lets
  bare "was" govern the date its own clause names,
  and a negation standing in the auxiliary's own
  adverb slot ("then was not played") is spent on
  the clause's verb the same way — so November 8
  stands on its own standing, nothing named, where
  before both dates silenced. With a subject before
  the auxiliary, the same complement scan makes
  "choir was loudly singing" (any one unlisted
  adverb) a finite clause for the reach's test.
  Held: the claim's own passive constructions are
  unchanged ("then was moved" still claims with
  naming, "then was cancelled" still silences), the
  shared adverb walk's spending rules (the listed
  list, the two-cap) are untouched — this is clause
  detection, not spending — and a bare "was" with
  no participle after it ("(was November 1)") still
  governs as the past marker it is. One witness
  needed the complement visible at all: "taken"
  joins `IRREGULAR_FINITE_FORMS` (58 → 59), the
  take family's participle the suffix rules cannot
  build, on the R19-5 "gone" precedent.
- **R22-5.** A subject run crosses a prepositional
  phrase: `segment_subject_before` crosses a
  preposition once it has crossed the noun-like
  run that is its object (a weekday object
  included), so "the team from Kent sang / played /
  was singing …" (all 23 of RT22-6's verb forms,
  probed) and "the choir on Sunday …" end the
  reach — November 8 alone, nothing named. The
  subject's "from" no longer governs the target:
  `phrase_governs` exempts a "from" standing in a
  subject PP (subject head before it, its object
  after it, the clause's finite verb after that).
  The PP's weekday is consumed wholly:
  `weekday_in_subject_pp` keeps "on Sunday" inside
  a subject phrase from spawning a signal in both
  scans (group and plain), so no phantom October
  18 asserts, is offered as an alternative, or is
  named, for was-forms, plain forms, and -ed forms
  alike. Held: subject runs without PPs behave
  exactly as R21-B pinned, and an object PP keeps
  its standing reading ("baked beans from Kent"
  in the object continuation reads byte-identically
  to HEAD — the reach is not ended by a clause
  there; its silence is the standing "from"
  governance, untouched).
- **R22-6.** The determiner shield spans its noun
  phrase for participle forms: in
  `verb_form_is_finite`, an irregular participle
  heading a noun phrase is shielded by a
  determiner or possessive standing anywhere
  before it in the segment (the R19-6 active-"for"
  rule applied to the reach's shield), so "the
  big left luggage" and "the big sung hymns" claim
  November 8 with November 1 named, like the
  regular twins. The span is scoped to forms with
  a participle use: the new
  `IRREGULAR_PAST_ONLY_FORMS` list (25 words —
  sang, rang, took, and their kin, whose
  participles are different words) keeps a
  past-tense-only form finite in every position,
  so "the choir sang hymns" and "the team from
  Kent sang hymns" stay clauses, and regular forms
  keep the subject rule ("the choir folded
  bulletins"). Held: unshielded subjectless
  irregulars ("sang hymns", "sung hymns", "left
  luggage") still end the reach per verb test v2,
  including the "moved the meeting" alternatives
  landing.
- **R22-8.** The bare inventory joins, in
  `bare_filler_word`: the remaining prepositions
  (with, during, since, within, without, upon,
  unto, into, over, under, between, among,
  against, per, via, onto, toward, towards,
  across, behind, beyond, beside, despite, amid,
  as — so "as per" / "as of" read bare word by
  word); the determiner / quantifier class (each,
  every, either, neither, both, all, any, some,
  another, other, same, such); possessive + "own";
  the conjunction / discourse tail (lest, albeit,
  unless, whereas, hence, thus, moreover,
  furthermore, nonetheless, nevertheless, instead,
  meanwhile, otherwise, therefore, anyway, rather,
  quite, indeed); and "too", bare in both slots.
  The inventory comment now states the classes are
  closed as listed and the rule is class-based
  (function words bare, content labels not).
  Every witness "(November 1, cancelled) (WORD
  November 1)" — the 56 head words of the pin's
  list (57 class words with "own"), the same
  classes in the trailing slot, the stacks ("the
  same", "my own", "each and every", "with the"),
  and "(too November 1 too)" — reads nothing on
  all paths (240 probe checks, zero non-empty). Held per
  standing adjudications: "thanks" in head
  position stays content, content labels assert,
  and "(our choir November 1)", "(each choir
  November 1)", "(with choir November 1)" assert.

Tests: four new pins (`r22b_subjectless_auxiliary_
is_a_second_clause`, `r22b_subject_prepositional_
phrases`, `r22b_determiner_shield_spans_its_noun_
phrase`, `r22b_bare_inventory_function_classes_
round_three`), every witness on the lanes' paths.
Workspace suite 763 → **767 passed, 0 failed**
(cargo exit 0, full unpiped run); clippy
`--workspace --all-targets` exit 0, no warnings.
**No pre-existing test changed.** Two defects
surfaced mid-batch in this batch's own first
drafts and were fixed in the edits rather than in
any test: the first shield draft was unscoped, so
a subject's determiner shielded regular -ed clause
verbs ("the team from Kent folded bulletins"
silenced, "the choir on Sunday folded bulletins"
falsely named November 1) until the span was
scoped to participle-capable irregulars; and
"then was taken" stayed silent on every path
until "taken" joined the irregulars (above).
Booklet bytes untouched (no export code in this
batch).

## Coordinator close, round 22 (2026-10-18)

Both batches independently verified by the coordinator on the exact
committed trees (full unpiped suite + clippy `-D warnings` after
each): R22-A `d33a6c3` at 763 tests, R22-B `5d0b439` at **767 tests
passing, 0 failed**, clippy clean at both steps. **Zero changed
pre-existing tests in the whole round.** Booklet bytes untouched
(31,717 / 34,977).

All eight MAJORs closed: groups compose across separators on both
paths and separator punctuation no longer destroys the relative
memory (R22-1); a sender's semicolon is a clause break everywhere
— root cause was the year token "2026;" failing the date parse and
dropping out of the date's span (R22-2); the wrong-weekday check
applies at every segment position (R22-3); a subjectless
auxiliary + participle is a second clause, not a governor, and
clause detection tolerates one adverb of any kind (R22-4); subject
runs cross prepositional phrases, whose words are consumed wholly
(R22-5); the determiner shield spans its noun phrase for
participle forms, scoped by a past-only irregular split so
"sang hymns" stays a clause (R22-6); the lone-framed fence fires
only on content-free lines (R22-7); the bare inventory's function
classes are completed and stated as closed, class-based (R22-8).
R22-9 stands adjudicated (rights safety pair invariant;
license-only residual).

Recorded for round 23's lanes: the trailing deictics RT22-8 left
ungraded (yesterday/today/tomorrow/here/there/together/alone/re)
were deliberately NOT added to the inventory — they sit outside
the enumerated classes; lanes should grade whether any of them
can resurrect a governed date in a restatement, and if one can,
that is a finding against the class boundary, not a list gap.
Also standing: "(moved November 1, baked beans from Kent to
November 8)" reads nothing by the standing "from" governance,
verified unchanged by this round (R22-B addendum).

The loop has not converged: round 22 returned findings, so the
Tier-2 final gate stays queued behind a Wonder round with zero
BLOCKER and zero MAJOR.
