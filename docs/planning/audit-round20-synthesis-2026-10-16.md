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
