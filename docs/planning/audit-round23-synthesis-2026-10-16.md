# Round 23 synthesis (2026-10-18)

Lanes over the round-22 close `e7abe6f` (767 tests): verify PASS
0/0/0; review 0/11/1; red team 0/7/3. Reports:
`~/workspace/openbulletin-audit-wave3/round23-{verify,review,redteam}.md`.

Deduped: **0 BLOCKER / 7 MAJOR / 2 MINOR.** The exit bar is not
met; the Tier-2 final gate stays queued.

Coordinator adjudication on RT23-9 (rights): identical to
rounds 20–22 — permutation flips 19/3,000 fresh seed, all
license-only in effect; zero flips in 127,128 children-bearing
checks; route agreement 80,412 comparisons, zero safety flips.
MINOR restatement of the known residual; no fix, census stands
(fourth consecutive round the safety axes held).

Held and worth naming: groups-only composition across 24
separators (168/168, zero splits); composition fuzz 92,032
lines with zero both-some-differ and zero for-some/keep-none;
verdict contract 40,000 lines × 6 bulletins, zero invariant
violations. The spine holds; the findings are boundary
classification and word classes again — and this round names
the meta-finding plainly: two "closed" lists (the bare
inventory's classes, the fence's content heads) were not
closed. Both fixes below change the shape of the rule, not
just the list.

## MAJORs and dispositions

**R23-1 (review F1 = RT23-6) — deictics are function words.**
All eight (yesterday, today, tomorrow, here, there, together,
alone, re) resurrect a governed date in head and trailing
slots on every surface. Disposition: the deictic class joins
the bare inventory. The mirror held (13/13 in round 22;
possessive-headed events still assert), so the class joins
safely. The inventory comment's claim is corrected: the
classes are the rule; the lists are their current membership,
and a newly found function word is a list gap — graded MAJOR
when the word is ordinary office vocabulary, as every word
in this class is.

**R23-2 (RT23-7) — the "closed" classes have remainders.**
Prepositions (past, off, besides, following, including, vs,
+11), quantifier/determiners (much, many, several, most,
which, whose, +11), connective adverbs (later, earlier,
afterwards, +9), adverbs (once, always, maybe, +9) — each
resurrects on every path. Disposition: the lanes' enumerated
remainders join their classes. Pronouns/modals/interjections
the lane left ungraded stay untouched. The class-based rule
in the comment stands with R23-1's corrected claim.

**R23-3 (review F2/F3/F4/F5 + RT23-1/RT23-2/RT23-3) — one
boundary table.** Five separator defects, one root: boundary
classification is per-site. (a) The wrong-weekday bypass
returns behind a leading group followed by any separator
other than space/semicolon (comma, colon, dash, period,
slash, "and"). (b) A colon after the governor word breaks
governance both ways by witness ("cancelled: November 8"
silences both dates on one surface; "cancelled: service
November 8, 2026" resurrects the governed date) — while
R22-2's own rule reads a colon after a DATE as a clause
break. (c) A colon at a relative-shape boundary, and a
semicolon/dash between trailing groups, destroy the For
line's relative memory (RT22-2's class reopened by new
separators). (d) A slash after a date inverts the postposed
composition. (e) Nested parens closing between a governor
and its date silence the target. Disposition: ONE boundary
classification, applied at every site that judges scope,
memory, weekday, or composition: colon, semicolon, comma,
dash (incl. en/em), slash, and period are clause/group
boundaries exactly as the comma is today; a closing paren
between a governor and its date is a boundary of the same
kind (the comma twin's reading governs every witness);
numeric-date slashes (11/1) are token-internal and
unchanged. Every witness reads as its comma/space twin.

**R23-4 (review F6/F7 = RT23-4) — the auxiliary's slot holds
two words.** "was not loudly played", "was not ever played",
"choir was softly gently singing", "was really not played",
"was not yet played" all silence both dates: the slot fits
a negation OR one adverb, not a negation AND an adverb, nor
two adverbs. And the spending walk never learned R22-4's
negation rule: "(it was not moved for November 1)" silences
the occasion the spent rule asserts. Disposition: for
clause detection AND for spending, the auxiliary's slot is
up to two words, each a negation or an adverb (listed or
not, for detection; the spending walk's tolerated list and
caps otherwise as they stand) — a negation in the slot is
spent on its verb in every scan, and the pair (negation +
adverb, either order; two adverbs) reads as one auxiliary
complement. Held: three-word slots do not spend; the
two-cap elsewhere stands.

**R23-5 (review F8/F9 + RT23-5 + RT23-10) — PP stacks and
PP date objects.** Stacked subject PPs fail when the first
preposition is "from" (the exemption walk stops at the
second PP); swapped order revives the phantom; "on the
Sunday" revives the three-way split (the determiner blocks
the consumption check); a framed date as a subject-PP
object is consumed only after "from" — after on/by/before
it asserts itself or silences the clause, and "the team
after November 1 sang" demotes the explicit target to
alternatives. Disposition: the subject run crosses a PP
STACK of any length and any preposition order; the
consumption checks (weekday, date) apply to every PP in
the stack, determiner or not; a framed date standing as a
subject-PP object after any preposition is consumed by
the subject exactly as after "from".

**R23-6 (review F10) — weekday + participle is a modifier
phrase.** "Sunday left luggage / sung hymns / taken seats"
spawn the phantom October 18 and demote the framed date:
the weekday-modifier rule (R20-6/R21-7) recognizes a bare
noun after the weekday but not a participle adjective
heading the noun phrase. Disposition: weekday + verb-form
word + noun-like word is a modifier phrase on both
surfaces, the verb-form word read as the phrase's
adjective; no phantom is asserted, offered, or named, and
the framed date keeps its standing.

**R23-7 (review F11) — the fence's list inverts.** The
lone-framed fence fires for every off-list event noun
("Sunday picnic/brunch/festival/banquet" assert November 8)
while on-list "Sunday rehearsal" stays silent: the split
is the ~70-word content-head list itself, and its failure
direction is assertion. Disposition: invert the test. The
fence fires only when the line's content beside the framed
date is a weekday-modifier phrase whose head noun is on a
closed GARNISH-noun set (the pinned shapes: bulletins,
school, morning, and their kin as pinned); any other noun
— event or not, listed or not — blocks the fence. The
failure direction for an unlisted noun becomes silence,
which is the safe direction and matches the held rule for
lone content lines. The content-head list leaves the fence
(it may stand elsewhere if another rule uses it).

## MINORs and dispositions

- **R23-8 (review M1 + RT23-8) — the past-only split scopes
  both shields.** "the sang hymns" claims (immediate shield
  unscoped) while "the big sang hymns" does not (spanned
  shield scoped). Disposition: past-only forms are finite
  under BOTH shields; the remaining shield shapes in
  RT23-8's 10-of-58 are accepted as the verb test's stated
  limits unless the same rule reaches them.
- **R23-9 (RT23-9)** — adjudicated above; no fix.

## Fix batches

- **R23-A** (inventory + boundaries): R23-1, R23-2, R23-3.
- **R23-B** (reach + fence): R23-4, R23-5, R23-6, R23-7,
  R23-8.

Held lines: the R19–R22 families, defensible silence,
claim/scope pins, the standing relative-line asymmetry,
content labels assert, "thanks" head content, rights pin,
loader pins, booklet bytes 31,717 / 34,977. Flag any changed
pre-existing test loudly by name with its reason.

## Shipped addendum, R23-A (2026-10-18)

R23-1, R23-2, R23-3 shipped in one batch, all in
`crates/ob-extract/src/contribution.rs`. Suite 767 → 769
(+2 new pin tests, `r23a_bare_inventory_deictics_and_
remainders` and `r23a_one_boundary_classification`; no
pre-existing test changed, none removed — the full
unpiped workspace run is exit 0 with 0 failed, and
clippy `--workspace --all-targets` is clean). Booklet
bytes untouched (no format/model file changed).

Word classes added to the bare inventory (65 words):
the deictic class (8: yesterday, today, tomorrow,
here, there, together, alone, re); the preposition
remainder (16: past, off, besides, plus, minus,
concerning, regarding, barring, following, including,
atop, amidst, amongst, thru, versus, vs); the
quantifier/determiner remainder (17: much, many, few,
several, enough, most, more, less, least, none, little,
what, which, whose, whatever, whichever, whoever); the
connective-adverb remainder (12: accordingly,
consequently, subsequently, next, later, earlier,
afterwards, afterward, beforehand, simultaneously,
eventually, initially); the adverb remainder (12:
once, twice, always, ever, perhaps, maybe, almost,
nearly, pretty, fairly, basically, merely). The
inventory comment's claim is corrected per the
disposition: the classes are the rule, the lists are
their current membership, and a newly found function
word is a list gap (MAJOR when the word is ordinary
office vocabulary). The lanes' ungraded pronouns /
modals / interjections were NOT added, as instructed.
Mirror held on the audit probes: every new-class word
before a content label (choir/service) still asserts,
"thanks" head content and possessive-headed events
still assert.

The boundary rule in one sentence: colon, semicolon,
comma, dash (incl. en/em), slash, and period are
clause/group boundaries exactly as the comma already
was — at governor scope, relative-shape memory,
weekday checks, group composition, and postposed
composition alike — a closing paren between a governor
and its date is a boundary of the same kind, and
numeric-date slashes (11/1) stay token-internal.
Mechanisms: the shared cleaning (`group_token`) and
the clause-break test (`clause_break_after`) carry the
full set; the relative-shape readers use the shared
cleaning and trim boundaries at both ends of the
remainder; the composition's weekday check strips a
leading boundary (incl. the word "and") from each
segment; a date run may not start on a boundary token
(`parse_date_prefix`), which is what lets a nested
group closing between a governor and its date read as
its comma twin; and boundary punctuation attached at
a token's head ("(please):November 1, …") is read off
the token as at its tail.

Audit-probe adjudication after the fix (same probe
crates the lanes used, run against the shipped tree):
bare23 — every enumerated class resurrect=false in
head and probed trailing slots, mirror asserts
throughout; comp23 — part B 0/30 differ from comma
twins (was 1), part C 0 asserts across 233 wrong-
weekday lines with the mismatch recorded on all 224
census lines, part D governed-resurrection oracle
0 of 92,032 fuzz lines (was 394); focus23 F1/F2/F3 —
every colon-governor line reads as its comma twin,
every separator lead records Monday@2026-11-15, every
relative shape remembers and arms. Flagged test
changes: NONE — no pre-existing test was changed.
One expectation in the NEW boundary pin was corrected
during the batch (the `((cancelled, November 1),
November 8)` twin names nothing, matching its flat
comma twin's standing group-initial-governor silence);
that was a new-test authoring fix, not a behavior
change. Remaining outside this batch's exact lists,
recorded not fixed: the two-word stack units "care
of", "back together", "gone tomorrow" still resurrect
(their non-deictic members — care, back, gone — stand
in no enumerated class), as do the lanes' ungraded
pronouns/modals/interjections by design.

## Shipped addendum, R23-B (2026-10-18)

R23-4, R23-5, R23-6, R23-7, R23-8 shipped in one batch,
all in `crates/ob-extract/src/contribution.rs`. Suite
769 → 774 (+5 new pin tests, `r23b_aux_slot_holds_two_
words`, `r23b_subject_pp_stacks_and_date_objects`,
`r23b_weekday_verb_form_modifier_phrase`,
`r23b_lone_framed_fence_fires_only_for_garnish`,
`r23b_past_only_split_scopes_both_shields`; no
pre-existing test changed, none removed — the full
unpiped workspace run is exit 0 with 0 failed, and
clippy `--workspace --all-targets` is clean). Booklet
bytes untouched (no format/model file changed).

R23-4: the auxiliary's slot holds two words in both
scans. Detection (`aux_complement`) crosses tolerated
adverbs as before plus up to two slot words, each a
negation or an adverb listed or not ("ever" / "yet"
join the slot's non--ly adverbs): "was not loudly
played", "was not ever played", "choir was softly
gently singing", "was really not played", "was not
yet played" all read November 8 alone. The negation
exemption in `phrase_governs` is generalized
(`negation_in_aux_slot`): the negation may stand in
either slot position, spent on its verb whenever an
auxiliary behind the slot carries a complement past
it. The spending walk (`aux_adverb_walk_bounded`,
the one walk behind the spent-predicate, passive-for,
and claim scans) learned the same rule: a negation
crosses when an auxiliary stands beyond the slot, it
counts toward the two-word cap, and an unlisted
adverb crosses only beside a negation ("not loudly",
"loudly never", "not ever" spend; an unlisted adverb
alone and three-word slots still do not). "(it was
not moved for November 1)" and its never/really/ever
twins assert November 1; "(moved November 1, it was
not moved to November 8)" claims as "then was moved"
does. Held: "then was moved" claims with naming,
"then was cancelled" silences, the claim's own
two-adverb caps stand elsewhere.

R23-5: the subject run crosses a PP stack of any
length and any preposition order. Two shared walks
carry it — `subject_head_before` (backward, crossing
earlier PPs in a stack) and `verb_after_pp_objects`
(forward, crossing each PP's object: noun run,
determiner-wearing weekday, or framed date) — used
by `weekday_in_subject_pp`, `from_stands_in_subject_
pp`, and the new `date_in_subject_pp`. A determiner
inside the PP blocks nothing ("the choir on the
Sunday sang" and its `played` twin read November 8
alone). A framed date standing as a subject-PP
object after ANY preposition is consumed by the
subject in both compositions: the target asserts
(not alternatives) and the PP's date is named beside
it exactly as the "from" twin's is ("the team after
November 1 sang to November 8" asserts with
November 1 named; "the choir on November 15 sang
November 8, 2026" asserts November 8, November 15
noted). Held: R22-5's single-PP pins, and the
object-continuation PP ("baked beans from Kent")
still reads nothing by "from" governance.

R23-6: weekday + verb-form word + noun-like word is
a modifier phrase on both surfaces, one reading in
`weekday_modifier_at`, applied in the group and
plain signal scans, the claim reach's between-words
test, and the fence; `verb_form_is_finite` reads the
verb-form word after a weekday (not standing as a
subject PP's object) as the phrase's adjective when
it heads a noun phrase. "Sunday left luggage,
November 8, 2026" asserts November 8, "Sunday sung
hymns, November 1, 2026" asserts November 1, and the
group twin "(moved the meeting, November 1, 2026,
and Sunday taken seats to November 8)" claims
November 8 and names November 1 — no October 18 is
asserted, offered, or named anywhere. Held: the
bare-noun modifier pins, true relative phrases, and
the subject-PP weekday consumption.

R23-7: the lone-framed fence is inverted. It fires
only when the line's content beside the framed date
is a weekday-modifier phrase that is garnish: for
the bare weekday + noun shape, the head noun must
stand on the closed GARNISH-noun set as shipped —
**bulletins, school, morning** (`GARNISH_NOUNS`,
taken from the r21b/r22a asserting pins, which use
no other bare head noun) — and any other noun,
event or not, listed or not, blocks the fence.
"Sunday picnic/brunch/festival/banquet, November 8,
2026" read nothing; "Sunday rehearsal, November 1,
2026" still reads nothing, now by the garnish rule
rather than list membership; "Sunday bulletins",
"Sunday school", and "Sunday morning" keep
asserting. The `content_head_word` list (~70 words)
has left the fence; the function is removed. One
adjudication, flagged loudly: the verb-form
adjective shape of R23-6 ("Sunday left luggage",
"Sunday baked beans") passes the fence by its
phrase shape, not by a garnish head — a garnish-only
reading of that shape would contradict R23-6's
explicit pins and the review's correct "Sunday
baked beans" twin, so the garnish set governs the
bare weekday + noun shape, which is the shape
F11's witnesses all wear.

R23-8: the past-only split scopes both determiner
shields — the immediate shield at the head of
`verb_form_is_finite` now exempts
`IRREGULAR_PAST_ONLY_FORMS` exactly as the spanned
shield does, so "the sang hymns" in the reach reads
as a clause (November 8 alone, November 1 unnamed),
exactly as "the big sang hymns" does. Participle
forms keep their shield under both ("the taken
seats", "the big taken seats" claim with naming).

Flagged test changes: NONE — no pre-existing test
was changed or removed. Every witness was probed
before and after on the lanes' own paths (relative,
paren-only, keep-through; plain twins where the
findings carry them).
