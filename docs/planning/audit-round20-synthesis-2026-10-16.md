# Round 20 synthesis (2026-10-18)

Lanes over the round-19 close `0d79aa3` (737 tests): verify PASS
4/4 claims, 0/0/0 (CI for the close and bundle 39095834150 in
progress at its check, reported as statuses); review 0/4/5; red
team 0/2/6. Reports:
`~/workspace/openbulletin-audit-wave3/round20-{verify,review,redteam}.md`.

Deduped: **0 BLOCKER / 5 MAJOR / 9 MINOR.** The exit bar is not
met; the Tier-2 final gate stays queued.

Coordinator adjudication on RT20-3 (rights): the lane's literal
brief said any permutation flip is a BLOCKER. The flips it found
(26/3,000 sequences) are license first-fill among conflicting
licenses where NO record notes children — the license-only
residual class measured and recorded since round 11. The safety
pair (children, permission_noted) is permutation-invariant over
5,665 children-bearing sequences × all permutations, and the two
routes agree on all 784,621 comparisons. The safety axes held;
RT20-3 is recorded as a MINOR restatement of the known residual,
no fix, census stands.

## MAJORs and dispositions

**R20-1 (review F1 = RT20-1) — the path split is general, and
Keep-through asserts governed dates.** A governed plain core
beside ANY contributing trailing group: For Sunday drops the
group silently and unnamed; the Keep-only fallback
(contribution.rs:5230) asserts the last group blind, in the base
witness asserting the very date the core cancelled. Disposition:
one composition for core + trailing groups on both paths. The
core's governed values bind bare trailing-group restatements
(the cross-group rule extends across the core/group boundary —
R19-2's adjudication already rejected the sibling distinction
inside group space); surviving trailing groups assert on For
Sunday by the trailing-group primacy the paren-only path uses,
with governed values named as the composition names them; the
Keep fallback then reads the same composition and the paths
agree field for field. Held bounds: trailing group itself
governed → nothing on both paths; core asserting → unchanged;
the base witness "(November 1) (cancelled core)" reads nothing
on both paths (the trailing restatement is bare and bound).

**R20-2 (review F2) — the bare test eats passive predicates.**
A region whose words form a passive predicate (auxiliary +
governor participle, with/without "for": "it was moved for
November 1", "it was moved, November 1", "it was rescheduled
for November 1") classifies as a bare restatement and is
suppressed by any sibling governance, while the same
construction with a non-governor participle ("scheduled",
"held") asserts, as the spent/passive-for rules require.
Disposition: an occurrence whose region carries a passive
predicate construction is NOT bare — it heads its own event
clause (R19-1's rule). "(for November 1)" stays bare (pinned);
the "use" judgment call stands.

**R20-3 (review F3 + RT20-2) — the bare inventory misses
function words, and governed dates resurrect.** Possessive
determiners (his/her/their/my/our/your/its), demonstratives
(these/those/this/that), and discourse garnish (please, yes,
again, ps — a closed list, stated) are absent from
`bare_filler_word`, so "(our November 1)" and "(please
November 1)" restatements count as content and assert their
governed date on every path. Disposition: those word classes
join the bare inventory. A label still asserts ("choir
November 1" stands — R19-1's rule); only function words join.

**R20-4 (review F4 + RT20-4 + RT20-5) — the reach's verb test
is two half-rules that each misfire.** (a) R19-4 tempered only
generic -ed forms: listed-stem, irregular, and -ing adjectives
heading the object continuation (planned dinner, gathered
supplies, sung hymns, left luggage, planning meeting) kill the
claim identically. (b) The generic -ed adjective test consults
only the word AFTER: "choir folded bulletins" (subject +
verb + object, a clause) reads as an adjective phrase, the
reach crosses it, and a false from-date is named. (c) A
postposed -ed after its noun ("dinner rolls, baked to
November 8") passes the verb test and kills the claim.
Disposition: ONE verb test for every verb-form word in the
reach, stated in one sentence for the addendum: a verb-form
word counts as a finite verb only when a noun-like subject
stands before it in its segment; otherwise, with a noun-like
word after it or standing postposed after its noun, it reads
as a modifier and the reach continues. Held: "team played",
"choir sang hymns", "choir practiced" end the reach (subject
present); "baked beans / canned goods / planned dinner / sung
hymns" continue it (no subject).

**R20-5 (review F5 + RT20-7 + RT20-8) — adverb tolerance is
three different rules.** The list omits the ordinary tier
(really, actually, truly, quickly, definitely, probably,
carefully); a single unlisted adverb silences even a target
claim; fronted/pre-auxiliary stacks consult neither the list
nor the two-cap in some scans, so three unlisted adverbs spend
in front where one listed adverb governs behind.
Disposition: the seven words join the list; all three scans
apply the SAME list and the SAME two-word cap in BOTH
positions (fronted stack before the auxiliary, post-auxiliary
stack), pinned in each scan.

## MINORs and dispositions

- **R20-6 (RT20-6)** — a bare weekday noun-modifier inside a
  group ("printed Sunday bulletins") spawns a relative signal
  and names a date nobody wrote (October 18). Disposition: a
  weekday word followed by a noun-like word inside a group is
  a modifier, not a relative signal; the lone-"(Sunday)"
  garnish fence and true relative phrases stand.
- **R20-7 (review F6)** — R19-8's consumed-base exclusion
  holds only on the plain-core merge; group alternatives still
  name the base. Disposition: the exclusion applies wherever
  second dates are named from a Sunday-after phrase.
- **R20-8 (review F7)** — R19-9's continuation link admits
  any pre-date "to" ("note to self" shapes in the LAST group
  when the group also carries a claim elsewhere). Disposition:
  the link counts only when the "to" belongs to a target
  construction (a displacement/postpone phrase in the group
  governs the link, or the link directly continues one);
  garnish "to" never classifies a chain. The r19b witnesses
  stand both ways.
- **R20-9 (review F8)** — loser naming stops at exactly two
  groups. Disposition: the two-group rule generalizes — in a
  multi-group reading with at least one relative group, losers
  are named on the note the same way. The bare framed stack's
  mismatch-only naming stands.
- **R20-10 (review F9)** — a Sunday-after phrase over a
  relative base dies with its restated governed value; the
  framed-base twin salvages. Disposition: parity — an
  ungoverned Sunday-after phrase salvages by its own
  resolution whatever its base's form; a governed phrase
  still dies.
- **RT20-3** — adjudicated above; no fix.

## Fix batches

- **R20-A** (composition + bare test): R20-1, R20-2, R20-3.
- **R20-B** (reach + adverbs + naming): R20-4, R20-5, R20-6,
  R20-7, R20-8, R20-9, R20-10.

Held lines for both batches: the R19-1 event rule (a labelled
occurrence asserts independently), "(for November 1)" bare,
the "use" non-bare call, defensible silence, claim/scope pins,
the rights brute-force pin, loader pins, booklet bytes
31,717 / 34,977. Flag any changed pre-existing test loudly by
name with its reason.

## Shipped addendum, R20-A (2026-10-18)

R20-1, R20-2, and R20-3 shipped, all work in
`crates/ob-extract/src/contribution.rs`. **737 → 740 tests
passing (ob-extract 224 → 227), clippy clean, zero
pre-existing tests changed.** Three new pins:
`r20a_core_group_composition`, `r20a_passive_predicate_is_not_bare`,
`r20a_bare_function_words`. Held lines stand: the R19-1
event rule, "(for November 1)" bare, the "use" non-bare
call, defensible silence, claim/scope pins, the R19-A/R19-B
families, the rights brute-force pin, loader pins, booklet
bytes 31,717 / 34,977 untouched.

**R20-1 — one composition for core + trailing groups.**
When a plain core's own composition governs every signal
it carries, the core now stands in the composition the
way a sibling group stands in the cross-group rule: its
governed values are read off the composition itself
(`every_governed_core_values` — the composition's
every-signal-governed silence is the judge, no second
governor test), and they bind a trailing group's BARE
restatement of the same value across the core/group
boundary. A surviving group then asserts in `date_line`
by the trailing-group primacy the paren-only path uses —
the last surviving group is the line's date, earlier
surviving groups disagree on the mismatch record
(read-from-parentheses), a split chain's superseded
targets and a two-group relative loser are named as the
paren-only path names them, and the core's governed
values are named on the second-date note as the plain
composition names its governed dates. When no group
survives, the line reads nothing. The Keep-through
fallback (R9-9) reads its groups through the same core
governance (`trailing_dates_after_core_governance`), so
it can no longer assert a date the core governed and the
group only restated bare. Pinned field-for-field on both
lines: the base witness reads nothing on both paths;
`(November 8)`, `(next Sunday)`, `(this Sunday)`, a
Sunday-after group, a labelled `(service November 1)`,
and `(moved to …)` groups assert on both; two-group
shapes follow the same primacy with the bare governed
restatement suppressed in either order; the held bounds
stand (trailing group itself governed → nothing; an
asserting core is unchanged, including the salvaging
witness naming November 15 on the mismatch record).

**R20-2 — a passive predicate is not bare.** The bare
test now checks each side of an occurrence's date
separately for a passive predicate construction — a
cancellation/displacement participle standing as an
auxiliary's predicate, by the standing
`is_spent_predicate` test — and such an occurrence is
not bare: "it was moved for November 1", "it was moved,
November 1", "it was rescheduled for November 1", and
"it was cancelled, November 1" head their own event
clauses and assert beside any sibling governance, in
groups, within one group, on plain lines, and in either
order. The two sides are judged separately so a
predicate cannot span the date itself ("was November 1,
cancelled" forms no construction). Held: "(for
November 1)" stays bare and bound; "use November 1"
stays non-bare; "(it was scheduled for November 1)",
"(it was held, November 1)", and the active-for twins
keep asserting.

**R20-3 — function words join the bare inventory.**
`bare_filler_word` gains the possessive determiners
(his, her, their, my, our, your, its), the demonstratives
(this, these, those — "that" was already there), and the
discourse garnish list, which is **closed: please, yes,
again, ps** — no other garnish words join. A restatement
wearing only one of those words is bare and stays
governed on all paths, plain twins included. The
boundary is pinned: a content label still asserts —
"(choir November 1)", "(meeting November 1)", and
"(November 1 reminder)" beside a sibling cancellation
assert per R19-1; function words bind, labels assert.

## Shipped addendum, R20-B (2026-10-18)

R20-4 through R20-10 shipped, all work in
`crates/ob-extract/src/contribution.rs`. **740 → 747 tests
passing (ob-extract 227 → 234), clippy clean, zero
pre-existing tests changed.** Seven new pins:
`r20b_one_verb_test_for_every_verb_form`,
`r20b_adverb_tolerance_unified`,
`r20b_weekday_modifier_in_a_group_is_not_a_signal`,
`r20b_consumed_base_excluded_in_group_alternatives`,
`r20b_chain_link_must_belong_to_a_target_construction`,
`r20b_multi_group_losers_are_named`,
`r20b_sunday_after_relative_base_salvages`. Held lines
stand: the R19-A/R19-B/R20-A families, defensible silence,
claim/scope pins, the rights brute-force pin, loader pins,
booklet bytes 31,717 / 34,977 untouched.

**R20-4 — one verb test for every verb-form word in the
reach.** As implemented, in one sentence: a verb-form
word (generic -ed, listed stem form, irregular, -ing
form) counts as a finite verb only when a noun-like
subject stands before it in its segment; otherwise, with
a noun-like word after it (an adjective heading the noun
phrase) or standing postposed after its noun, it reads
as a modifier and the reach continues. The subject test
(`segment_subject_before`): the subject is the segment's
only noun-like, non-verb-form word before the verb form,
with nothing but adverbs and auxiliaries between it and
the verb form; a segment opening with a conjunction
continues the target phrase's object list (the R18-4
reading), so no subject stands in it; and two noun-like
words before the verb form are the noun phrase it
postposes. The F4 family claims on all paths (planned
dinner, gathered supplies, sung hymns, left luggage,
read books, planning meeting, meeting notes, singing
lessons, gathering supplies; the plain twins assert the
target instead of filing for the from-date); "choir
folded bulletins", "team grilled chicken", and "ushers
stacked chairs" end the reach like "choir sang hymns"
(RT20-4); the postposed "(… and dinner rolls, baked to
November 8)" claims with November 1 named (RT20-5). The
postposed comma is the modifier phrase's own
punctuation, so the reach's clause-boundary test exempts
a break standing on a noun-like word directly before a
verb form the test reads as a postposed modifier — one
consequence, recorded deliberately: the comma variant of
the stacked family ("and beans, baked canned goods to
November 8", review probe S3o, previously collapsed
through the clause-boundary rule as a standing
mechanism) now claims like its comma-less twin. The
R19-5 witnesses were checked against the unified rule
before it landed: every pinned second clause carries its
subject ("team plans", "choir rehearses", "congregation
gathers", "choir goes"), so no held line conflicted and
none was moved.

**R20-5 — adverb tolerance unified.** really, actually,
truly, quickly, definitely, probably, and carefully join
the tolerated list (**27 → 34 words**). All three scans
(target claim, passive spent, passive-for) now run the
one backward walk (`aux_adverb_walk`): the same list and
the same two-word cap in both positions — a stack split
across the auxiliary spends its budget as one stack, a
third listed adverb anywhere in the walk stops it, an
unlisted adverb in the post-auxiliary, pre-auxiliary, or
fronted slot does not spend, and a fronted stack
standing in its own clause before the construction is
judged by the same list and cap (counted against the
walk's budget where the scan can see it). "(it was
really moved to November 8)" and "(really, it was moved
to November 8)" claim the target; RT20-7's
single-unlisted witnesses now spend (all seven words are
on the list); RT20-8's fronted three-word stack "(really
truly actually, it was moved, November 1)" does NOT
spend, matching the post-auxiliary cap, in all three
scans. Pinned per scan per position.

**R20-6 — a weekday modifier in a group is not a
signal.** A weekday word followed by a noun-like word
inside a group is a modifier, not a relative signal:
"(… and printed Sunday bulletins to November 8)" asserts
November 8, names November 1, and names NO October 18;
the same reading in `target_claim`'s between-words test
lets the reach cross the noun phrase. The lone-"(Sunday)"
garnish fence stands, and true relative phrases in
groups ("(next Sunday)", "(this Sunday)", "(the Sunday
after November 1)") keep contributing as pinned.
Observed, not fixed (out of the disposition's group
scope): the plain-line twin still names October 18
through `scan_plain_signals`' bare-Sunday branch — the
same mechanism on the plain surface, recorded here for
a future round to take deliberately.

**R20-7 — consumed-base exclusion in group
alternatives.** The R19-8 exclusion now applies
wherever second dates are named from a Sunday-after
phrase: `analyze_group` computes its own consumed bases
with the standing phrase-run scan and filters them out
of every `trailing_named` fold into alternatives.
"(the Sunday after November 1, November 1, cancelled)"
asserts November 8 and names nothing for its base, on
the group path as on the plain path; the asserted
reading is unchanged.

**R20-8 — the continuation link must belong to a
target construction.** `group_has_target_claim`'s link
counts only when a displacement/postpone phrase in the
group governs the link (a target phrase before the "to"
with nothing between it and the "to" but its object
words), or the link directly continues one (nothing
before the "to" but the link's own lead-ins: "(then to
November 15)", "(to November 15)"). Garnish fragments —
"(back to November 15)", "(note to November 15)",
"(return to November 15)" — never classify a chain:
November 8 stands on the garnish mismatch, the note
names nothing, and November 15 asserts as before. The
r19b R19-9 witnesses stand both ways.

**R20-9 — loser naming generalized.** The two-group
rule now reads "at least two groups, at least one
relative": "(November 8) (next Sunday) (November 15)"
asserts November 15 and names November 8 and October 25
on the note, the mismatch record standing as it did.
The generalization applies on the governed-core
composition path as on the paren-only path. The bare
framed stack keeps mismatch-only naming (the r18b pin).

**R20-10 — Sunday-after salvage parity.** A
Sunday-after phrase signal is never bound by date value
in `analyze_group` or `plain_composition`: an
ungoverned phrase salvages by its own resolution
whatever its base's form — "the Sunday after next
Sunday, November 1, 2026, cancelled" asserts November 1
with nothing named, its this-Sunday twin asserts
October 25, and the group twin asserts November 1. A
phrase governed in its own span still dies ("the Sunday
after next Sunday, cancelled, November 1, 2026" reads
nothing), and the framed-base witnesses are unchanged.
