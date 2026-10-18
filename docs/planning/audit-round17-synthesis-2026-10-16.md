# Round 17 audit synthesis — 2026-10-17

HEAD audited: `c1754d0` (round-16 close). Lanes: verify `36840604`
(report `~/workspace/openbulletin-audit-wave3/round17-verify.md`),
review `69d9341c`, red team `f4e2ba46` (its session errored on a
runtime restart AFTER its report landed complete on disk; the file
is authoritative and is what this synthesis cites). Verify:
**PASS, 8 of 8 claims**, CI for the close push green on all three
OS jobs (39078776068), bundle 39078776437 SUCCESS, round 16's open
CI/bundle items confirmed. Review **0 BLOCKER / 1 MAJOR /
4 MINOR**; red team **0 BLOCKER / 2 MAJOR / 4 MINOR**. **The exit
bar is not met.** Deduped: **0 BLOCKER / 2 MAJOR / 6 MINOR**. Both
MAJORs are one clause past a round-16 rule.

## Findings and dispositions

**R17-1 (MAJOR; RT17-1).** A from-date plus two targets collapses:
"(November 1, moved to November 8, then to November 15)" asserts
nothing on any group path (all three dates merely noted), while
its plain-line twin asserts November 8 — the superseded target —
so the surfaces disagree. "Moved to the 8th, then to the 15th" is
the ordinary way an office corrects a move. Disposition: the
target rule takes the LAST target in the claim run; earlier
targets are governed and named like the from-date, on both
surfaces (one reading).

**R17-2 (MAJOR; RT17-2 + review Q4).** Mixed plain lines apply no
governor at all: "November 1, 2026, cancelled, next Sunday" ARMS
November 1 with October 25 merely noted; the postponed, moved-to,
and postponed-drop-it-run twins behave identically, while their
all-framed twins assert the replacement. R16-4's harm, one signal
type over. Disposition: plain-line composition reaches mixed
lines — when a governor phrase, run, or target claim stands on a
mixed line, the governed signals yield (named on the note) and
the first ungoverned signal asserts, relatives resolved exactly
as R13-C resolved them. Mixed lines with no governing phrase keep
the R13-C first-signal reading untouched (held pins stand).

**R17-3 (MINOR; RT17-4).** In the "from … to" shape the governed
from-date is never named: "(moved from November 1 to
November 8)" asserts Nov 8 with an empty note. Disposition: the
from-date of a claimed group is named on the second-date note,
per the R16-5 naming principle.

**R17-4 (MINOR; RT17-5).** "was + participle + for + date"
silences the occasion date: "(it was moved for November 1)",
"(it was rescheduled for November 1)", "(it was scheduled for
November 1)" read nothing, while the comma forms and present-
tense forms name the date. "For" is the canonical preposition
for stating an occasion date. Disposition: after a passive
participle (auxiliary + participle), "for" + date names the date
as the occasion; the pinned bare same-clause parity ("(it was
moved November 1)" governs) stands unchanged.

**R17-5 (MINOR; review Q1).** The claim zone reaches 4 words past
its phrase; a 5-word object ("moved the November potluck planning
meeting to November 8") loses its target silently. Disposition:
the claim zone is clause-bounded, not word-counted — it reaches
to the end of the phrase's clause (a clause break, another date,
or the group's end bounds it). The 4-word boundary pins re-pin at
the clause boundary if they move, flagged.

**R17-6 (MINOR; review).** The genitive crossing's "no verb-form"
test calls the raw verb test, contradicting the same batch's
determiner rule: "(ignore, the date of, the drops, November 1)"
arms under an explicit "ignore" while "the service" / "the
classes" controls disqualify. Disposition: the completing-clause
test uses the determiner-aware form test (an -s word headed by a
determiner is not a verb-form there either).

**R17-7 (MINOR; review).** The one-adverb tolerance's -ly
catch-all admits nouns and names ("it was family moved", "it was
kelly moved" spend like "recently"). Disposition: the tolerance
set is a closed list — the named words (just, already, still,
also, even, now, soon, often) plus attested -ly adverbs (recently,
previously, originally, finally, officially, tentatively,
temporarily, currently, newly, jointly); no -ly catch-all.

**Accepted, recorded (no code change).** RT17-3 + review Q2: the
determiner walk-back's cost on canonical subjects ("(the council
postpones November 1)" arms) is the R16-A disposition's own
accepted cost, named in its addendum; the boundary flipping on
subject length is that cost's shape. RT17-6 + review Q3: a hand
rename of a filed copy's date segment moves its still-waiting
count and nothing catches it — the trust model's recorded edge;
malformed segments all fall back correctly, double-dated names
resolve deterministically, and the blast radius is the reminder
list, not application.

## Batches

- **R17-A** (governors): R17-1..R17-7, one batch.

Round 18 audits the fixes. The Tier-2 final gate runs only after
a round returns zero BLOCKER + zero MAJOR.

## Shipped addendum, R17-A — 2026-10-17

Batch R17-A (governors, fifth pass) shipped in
`crates/ob-extract/src/contribution.rs`: R17-1..R17-7. Seven new
pin tests, all `r17a_*`, one per finding. **Tests: 709 → 716
passed, 0 failed** (full workspace suite run unpiped on the
final code, cargo exit 0; ob-extract 196 → 203). **Clippy:
clean** (`--workspace --all-targets`, exit 0, zero warnings).
Neither the fake-helper ETXTBSY flake nor timing contention
fired in any run.

**Changed pre-existing tests: none.** No test body was removed
or altered; every round 11–16 held shape stands at its shipped
expectation, including the R13-C/R14-B mixed-signal pins on
governor-free lines (the fence for R17-2), "(November 1,
cancelled, November 8)" asserting and naming, pure runs and
run-after-date naming, the defensible silence of "(November 1,
cancelled, November 8, drop it)", the deadline "until" after
non-governor verbs contributing by the lone-date rule ("(not
moved to November 8)" and "delayed until" claim no target, as
pinned), the anchor family, and booklet bytes 31,717 / 34,977
exact. In particular, R17-5's flagged-change clause does not
fire: no r16a pin asserted the four-word boundary as a reading
— the bound lived only in `target_claim`'s own comment and the
round-16 addendum's scope sentence — so removing the count
moved no pin, and the four-word shapes in the r16a set read
exactly as before through the same rule.

What shipped, per finding:

- **R17-1 — last target wins** (`analyze_group`,
  `plain_composition`). A claimed target superseded by a later
  distinct claimed target in the same run is disqualified like
  the from-date and named wherever the naming rules name a
  governed date: "(November 1, moved to November 8, then to
  November 15)" asserts November 15 with [November 1,
  November 8] named on the group paths, and its plain-line
  twin asserts November 15 with the same note, where it
  asserted the superseded November 8 before. The
  verb-repeated forms already read this way and stand as held
  twins in the same pin.
- **R17-2 — mixed plain-line composition**
  (`plain_multi_date_reading`, `plain_composition`). The
  composition now runs whenever a plain line carries at least
  one framed signal, and every signal — framed or relative —
  is judged by the words around its own token span (a relative
  phrase's span is its own words). Judgment call, stated
  plainly: the round-16 scope note ("a relative phrase has no
  token span for the composition to judge") was the scope
  call this finding reversed; the span a "next Sunday" stands
  in is all the composition needs. The four witnesses assert
  October 25 with November 1 named on For Sunday and
  Keep-through. The composition still returns None when no
  signal is governed, so mixed lines with no governor phrase,
  run, or target claim keep the R13-C first-signal reading
  byte-for-byte (the r13c/r14b pins are the fence and pass
  unmoved), and the anchor re-resolution of relatives happens
  before the composition judges, exactly as before.
- **R17-3 — from-date naming** (`target_claim`). A "from"
  standing directly before an intervening date among the
  words between the phrase and its "to" is part of the
  construction, not a governor in the reach, so "(moved from
  November 1 to November 8)" claims its target — and
  November 1, another distinct date in a claimed group, is
  governed and named on the second-date note per R16-5. The
  shape asserted November 8 before, by the lone-date
  fallback; what changed is the empty note.
- **R17-4 — passive "for" names the occasion**
  (`passive_for_zone_start`, `governed_before`). When a
  governor window ends in "for" preceded by an auxiliary +
  participle construction — a governor participle in any
  form, or any -ed participle, "scheduled" the canonical
  case, phrasal pairs included, one tolerated adverb allowed
  in the scan back — the auxiliary and participle inside the
  construction do not govern the date the "for" names:
  "(it was moved for November 1)", "(it was rescheduled for
  November 1)", and "(it was scheduled for November 1)"
  contribute November 1. A governor standing before the
  construction still governs, and the pinned bare
  same-clause parity ("(it was moved November 1)" governs)
  stands unchanged — "for" is the distinguisher, pinned as
  the held twin in the same test.
- **R17-5 — clause-bounded claim zone** (`target_claim`).
  The four-word count between the phrase and its "to" is
  gone; the reach is bounded by the phrase's clause: a
  clause break standing among the in-between words ends it.
  Judgment call, stated plainly: a comma inside or closing
  an intervening date's own span is that date's punctuation,
  not a clause break in the reach — without that reading the
  second "to" of an R17-1 correction ("moved to November 8,
  then to November 15", and its plain twin wearing years,
  "November 8, 2026, then to…") could never claim its date,
  and the two findings would contradict each other.
  "(moved the November potluck planning meeting to
  November 8)" and its postponed-until twin assert
  November 8 on all three group paths.
- **R17-6 — genitive consistency** (`genitive_completion`).
  The completing clause's "no verb-form" test now exempts a
  word the determiner-aware form test reads as a
  determiner-headed noun head, for both the cancellation
  test and the single-governor test that contains it:
  "(ignore, the date of, the drops, November 1)" and the
  "the shifts" twin disqualify like the "the service" /
  "the classes" controls. A completing clause whose head is
  a bare verb-form ("the moved service") still stops the
  crossing — pinned as the held twin.
- **R17-7 — closed adverb list** (`is_tolerated_adverb`).
  The tolerance set is exactly the disposition's list: just,
  already, still, also, even, now, soon, often, recently,
  previously, originally, finally, officially, tentatively,
  temporarily, currently, newly, jointly. The -ly catch-all
  is gone, so "(it was family moved, November 1)", "(it was
  kelly moved, November 1)", and "(it was family cancelled,
  November 1)" no longer spend — the participle governs, as
  the non--ly control "(it was household moved, November 1)"
  always did — while the attested adverbs still spend,
  pinned with "recently", "just", "finally", and
  "originally".
