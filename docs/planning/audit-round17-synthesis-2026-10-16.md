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
