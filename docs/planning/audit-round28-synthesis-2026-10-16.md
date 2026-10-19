# Audit round 28 — synthesis (2026-10-16)

HEAD audited: `57e449c51fba714468cdd45c20e2b67e4f26c64c`
("Round 27 coordinator close: R27-A 55a2b46 + R27-B 5733879 +
R27-C 02e8c6f verified, 834 tests, clippy clean"). Base for
diffs: round-26 close `a358f70`. Verify claims re-run fresh
against the committed close tree; all three lanes read-only;
repo `git status` clean at every lane's close, HEAD
unchanged.

**Round 28 verdict: 0 BLOCKER / 6 MAJOR / 4 MINOR — the
exit bar is NOT met.** The verify lane returned a clean PASS
on every round-27 claim (below); the review and red team
lanes found the failures in what round 27's fixes stated too
narrowly: the token-hygiene rules (F3's nothing-tokens, F4's
glued separators) hold at the sites their batches named and
fail one step past them — inside phrases, at positional
readers — and the F5 walk's crossing rules meet three more
ordinary sentence shapes (a bare finite verb, a conjoined
second clause, a bare destination object) they do not cross.
One inventory unit creates an assertion (F3 below). One
adjudication was required (F6's mirror noun swap), recorded
with its reasoning. Fix batches R28-A/B/C are dispositioned
at the end.

Verify lane: **PASS, 0 / 0 / 0.** Every claim in the
round-27 close reproduces: the test-body diff is exactly
the 17 new pins plus the 4 flagged thanks flips (183 → 200
test bodies, 0 removed); `BARE_PHRASE_UNITS` is 166 with
`coming` a bare word and no `coming sunday` unit; fresh
unpiped suite **834 passed / 0 failed** (`CARGO_EXIT=0`),
clippy `CLIPPY_EXIT=0`, booklet **31,717 / 34,977**
byte-exact. The unchanged fuzz bins (md5-verified: verdict
`a9cac6c399d304bac853f2a64f683fca`, composition
`4f85c605a841329dabed1d9a2aa84f88`) re-ran as standing
claims: verdict 40,000 lines, 0 panics, **0 violations of
I1–I3** (I4/I4b at 5,722 / 5,585, identical to round 27,
the designed anchored-shift artefact); composition 497,024
lines, 0 panics, governed resurrections **1,133 → 943**,
every one of the 943 carrying the adjudicated
`(in spite of the choir …)` mirror — the 190 F4 glued-
negation lines are gone, R27-C's fix confirmed in the
census. Rights held (claim 5 below). Loader held (claim 6).
One ungraded observation: the standing `for_none_keep_some`
census count moved 1,789 → 1,797 (+8 lines entering the
dispositioned rounds-8/9 relative-line asymmetry under round
27's reading changes); no claim depends on the count.
Recorded.

Review lane: 0 / 3 / 3 (MAJOR-1/2/3, MINOR-1/2/3 below).
Red team lane: 0 / 5 / 1 (RT28-C1/W1/W2/W3/B1, W4 below).
Coordinator consolidation: the lanes' 8 MAJORs consolidate
to 6 — review MAJOR-3 and red-team RT28-C1 are one root
(F1), and red-team RT28-W2/W3 are two arms of one walk
finding (F5); review MINOR-1/2/3 are the
lesser sites of one root (F2, with its MAJOR core); red
team RT28-W4 is the tail of F5.

## F1 (MAJOR) — A glued separator hides a governor phrase at every word but the first (review MAJOR-3 + red team RT28-C1)

R27-C's shipped claim is kept exactly (review Q2 PASS by
diff: 9 removed lines, all as claimed; `group_token` and
the Cf machinery untouched; glued `*November` / `*choir`
keep standing readings; glued single-word heads govern in
every position). The gap is positional. `segment_tokens`
judges a glued head by building phrases from the head word
plus the tokens that follow **in their still-glued form**,
so a separator glued to a phrase's second or third word —
or to every word — hides the phrase:

- All-words-glued phrasal heads assert the governed date
  (red team twin driver: 576 pairs, 462 identical, **114
  differ, every one in the phrasal family**; 84 in this
  sub-shape): `(back together November 1) *hold *off
  November 1, 2026` ⇒ FOR **2026-11-01**; worst witness
  `November 1, 2026, cancelled (while November 1) *used *to
  *be November 1, 2026` ⇒ Nov 1 — the RT27-C1 worst shape
  (an expressly cancelled core) undone by a three-word
  glued head. Spaced twins ⇒ None everywhere.
- Glue on the second/third word alone (~30 in the review
  seg battery): `(back together November 1) hold *off
  November 1, 2026`, `(November 1, cancelled) (put *off
  November 1)`, `(put *back …)`, `(call *off …)`,
  `(back *together November 1)` (the displacement phrase
  inside its own group), `used *to be`, `used to *be`,
  `instead *of`, `rather *than`, `old *date` ⇒ all assert
  Nov 1 at close; spaced twins govern.
- Glued `off` alone never unglues in group/trailing
  positions (10 diffs): `(November 1, cancelled) (*off
  November 1)` ⇒ FOR **2026-11-01**; spaced `(* off …)` ⇒
  None. "off" governs in those positions as a spaced token
  but is not a governor-phrase head, so the head check
  never reaches it.
- Phrasal heads whose first word is itself a single
  governor diverge on the REL path (20 diffs, P1 only):
  the first word unglues, the second stays glued, and the
  relative readers — which R27-C named as keeping plain
  cleaning — lose the phrase: `(back together November 1)
  *never *mind November 1, 2026` ⇒ REL FOR 2026-10-25
  [2026-11-01]; spaced twin ⇒ REL None. Same for
  `*cancel *it`, `*drop *it`, `*scratch *that`,
  `*forget *about`.
- Same root, walk-critical words (review): a glued
  preposition or auxiliary makes the weekday PP / target
  construction invisible to the walk — `(moved November 1,
  beans were not served *on Sunday from Kent to November
  8)` ⇒ FOR **2026-10-18**; `… from Kent *to November 8`
  ⇒ Oct 18; `beans *were served on Sunday …` ⇒ Oct 18.

Graded MAJOR under the coordinator's own F4 rationale
(round-27 synthesis): a separator glued to a governor word
hides the word; a date the sender negated asserting over an
express cancellation is the worst direction. Emphasis in
pasted bulletin text glues to phrase continuations as
readily as to heads (`**hold** **off**`, `*used* *to*
*be*`). The disposition (R28-A): **ungluing is a property
of the token, not of the phrase head** — a `*` / `|` glued
to any word is the separator plus that word, for every
reader `segment_tokens` serves; governor phrases are then
judged on the unglued vector, at every position of the
phrase; the governing-position reading of a spaced `off`
must be reachable by its glued form in the same positions;
the relative-shape readers' phrasal forms are assessed in
the same batch and either routed through the same ungluing
(with pins) or recorded as standing with the reason stated
in the batch's addendum — not silently left. The named
keep-plain readers' own glued forms (`*next Sunday`,
`*Monday` before a date) were observed by the review lane
to behave boundary-consistently with the R27-C addendum's
documented choice; they are not part of this finding, and
moving them needs its own disposition.

## F2 (MAJOR) — A nothing-token still leaks at positional readers (review MAJOR-1; MINOR-1/2/3 are its lesser sites)

R27-A's F3 claim was an enumeration completed; measured by
the review lane's Cf twin battery (39 base lines × every
token position): **105 main-surface divergences at `a358f70`
→ 10 at `55a2b46` → 10 at `5733879` → 10 at close**. A
fixed 95; the residual 10 are untouched by B and C. Nine
pre-exist at base; the tenth was *created* by R27-A's
partial fix (mechanism 7 repaired the clean until-trailing
shape but not its Cf twin). Assertion-level sites:

1. **Trailing-arm re-judgment** (`weekday_pp_after_claimed_
   target`'s claim-"to" detection walks back over
   determiners only, never over nothing-tokens; the
   run-start adjacency check is likewise positional):
   `(moved November 1, baked beans from Kent until \u200b
   November 8 on Sunday)` ⇒ FOR **2026-10-18**
   [2026-11-08]; clean twin ⇒ None. The phantom asserts —
   the R27-3 shape escaping through Cf. The
   `\u200b on Sunday` and two-PP-run variants behave the
   same.
2. **Forward destination crossing** (the place-NP noun run
   in `forward_walk_continuation` and its twin in
   `forward_has_intervening_pp` does not skip
   nothing-tokens, so the run dies at the Cf token):
   `(moved November 1, baked beans on Sunday from Kent to
   the hall \u200b to November 8)` ⇒ FOR **2026-10-18**;
   clean twin ⇒ None.
3. **`use` prospective adjacency** (`use_prospectives` is
   `window.last() == Some("use")` in both `analyze_group`
   and `plain_composition_reading`; a trailing
   nothing-token defeats it): `(it was moved for November
   1, use \u200b November 8)` ⇒ FOR **2026-11-01**
   [2026-11-08]; clean twin ⇒ FOR 2026-11-08. The sender
   said "use November 8"; the reader asserts November 1.
4. **Three-word governor phrases with an interior
   nothing-token** (matchers filter empties, but every
   caller slices ≤3 *tokens*, so a 3-word phrase with an
   interior nothing needs a 4-token slice and is
   invisible): `(used \u200b to be November 1)` and
   `(used to \u200b be November 1)` ⇒ FOR **2026-11-01**;
   clean twin governed. The same bound applies inside
   `segment_tokens`' own governor check.

Lesser sites, MINOR as graded by the lane, dispositioned
in the same batch: phantom *named*, not asserted, at the
trailing "to" arm (`… to \u200b November 8 on Sunday` ⇒
Nov 8 [Nov 1, **Oct 18**]); the Sunday-after base scan
(`second_dates_in_line` calls `sunday_after_phrase_run` on
raw tokens; `the Sunday after \u200b November 1, 2026`
names Nov 1 in the note where the clean twin names
nothing); assertion *lost* at `was` + Cf + participle
(`(moved November 1, the choir was \u200b singing hymns on
Sunday to November 8)` ⇒ None; clean twin ⇒ Nov 8;
pre-existing at base, its neighboring slots fixed by
R27-A, this one never enumerated; `is`/`were`/`had` twins
unaffected); **MINOR-2**, a trailing Cf token defeats
trailing-group extraction on the REL surface only (the
extractor reads raw text, outside the enumeration the
batch described; 12 battery bases; the family pre-exists
at base, 102 REL divergences there, 22 remain); **MINOR-3**,
separator-only tokens are not nothings to adjacency
readers — `(November 1, cancelled) (November 1, not *
November 8)` ⇒ FOR **2026-11-08** where the Cf twin and
the clean twin govern — and separator tokens inside a
date's own span and between a phrase's words in the plain
readers damage the same way (the ≤3-token slice bound
again).

Why MAJOR (the lane's grading, adopted): three of the four
assertion-level sites sit in machinery R27-A explicitly
enumerated or rewrote (the trailing arm is mechanism 7's
own site; the destination run is mechanism 4's own site;
`use_prospectives` is a cleaned-token reader of exactly
the enumerated class), and the disposition's rule was
stated without qualification — a token that cleans to
nothing is nothing at every site that reads tokens. The
disposition (R28-A): **the rule extends to positional
readers** — adjacency, `last()`, walk-backs, slice bounds,
and raw-text extraction all skip nothing-tokens (Cf-only,
separator-only, and separator-glued-to-nothing alike);
phrase matching slices over cleaned tokens, not raw
positions, so a phrase's token budget counts its words,
not the nothings between them. The batch re-runs the
enumeration as a *positional* enumeration this time and
its addendum lists every reader class it covered; any
site it cannot reach is named there as an exception with
the reason, so the next round's enumeration claim is
falsifiable site by site.

## F3 (MAJOR) — One inventory unit creates an assertion: `see you sunday` (review MAJOR-2)

R27-B's blast-radius claim — ring III additions "only ever
REMOVE assertions in bare shapes" — holds for every
addition but one. The review lane's bare battery (3,520
lines, 5,940 outputs, differential `55a2b46` → `5733879`):
1,046 main-surface changes are Nov 1 → None removals,
every one containing a ring-III item; exactly **one**
output moves the other way:

- `November 1, 2026, cancelled, November 1, 2026 see you
  sunday` — `55a2b46`: FOR **2026-11-01** [2026-10-18];
  close: FOR **2026-10-18** [2026-11-01].

Mechanism: the shipped unit `&["see", "you", "sunday"]`
(contribution.rs:6970) makes the segment tail a
bare-phrase-unit span, so the restated Nov 1 occurrence
reads bare and is suppressed; the `sunday` *inside* the
unit still spawns its weekday signal (signals are not
occurrences and are never bare-checked), and with Nov 1
suppressed the phantom Oct 18 wins the segment. A farewell
phrase in a bulletin ("…no service November 1, see you
Sunday") is plausible prose; the reading both loses the
real date and asserts the phantom. The batch's pins cover
`see you sunday` only in group shapes, which do not flip
(the group twin offers alternatives at both trees); the
plain-composition shape was not in its mirrors. The
consumer graph is otherwise clean (the lane's Q3
structural pass: `bare_filler_word` has exactly two
non-test consumers, both removal filters;
`bare_phrase_unit_span` is consumed only inside
`occurrence_is_bare`; every `occurrence_is_bare` caller
uses bareness only as a removal filter; no consumer
outside contribution.rs touches any of the three
inventories). One premise correction from the lane,
recorded: R27-B's non-test diff touches only
`bare_filler_word` and `BARE_PHRASE_UNITS`; `aux_slot_word`
was created by R27-A and B added nothing to it.

Disposition (R28-C): **a bare unit's own weekday signal is
part of the bare span** — when an occurrence is suppressed
as bare because a unit span covers it, a weekday signal
spawned from inside that same span cannot outlive the
suppression and win the segment. The fix is scoped to the
signal/occurrence interaction, not to the unit's
membership (the unit's group readings stand as shipped);
if the machinery cannot scope it there, the batch stops
and reports rather than removing the unit silently.

## F4 (MAJOR) — A bare finite verb between subject and weekday PP defeats the from-crossing (red team RT28-W1)

The canonical consumed shape — claim phrase, subject,
verb, weekday PP, from-PP, target — reads silence when the
verb carries an object or an auxiliary, and asserts the
phantom when the verb stands bare:

- `(moved November 1, the choir sang hymns on Sunday from
  Kent to November 8)` ⇒ None (the standing reading).
- `(moved November 1, the choir sang on Sunday from Kent
  to November 8)` ⇒ FOR **2026-10-18** on all surfaces.

Sizing (22 verbs in the destination shape): with an
object, 4/4 hold; bare, **17 of 19 assert Oct 18** (sang,
played, rehearsed, travelled, traveled, walked, went,
ran, returned, journeyed, met, gathered, spoke, preached,
taught, visited, arrived, left, stayed); the exceptions
are `drove` and `rode`, which hold either way. Adding an
object to a breaking verb restores consumption (`walked
the aisle` ⇒ None); every auxiliary form restores it
(`was travelling`, `has travelled`, `did travel` ⇒ None).
Scope: the from-PP is the ingredient — no-PP and
destination-only shapes with a bare verb give the
alternatives/target readings; a static PP after the
weekday gives alternatives; "from" *before* the weekday
consumes; the until form breaks identically; the
accompaniment continuation breaks with the from. A
verb-less fragment in the same slot consumes (`baked
beans on Sunday from Kent …` ⇒ None), so the line with
*less* grammar reads worse than the ordinary sentence —
inverted. "The choir sang on Sunday from Kent" is
unremarkable bulletin prose; a bare intransitive or
object-dropped verb is the commonest verb shape in a
moved-note. Graded MAJOR. Disposition (R28-B): the
from-crossing must not depend on the verb carrying an
object or an auxiliary — the claim-phrase span and the
from-scope are the same with a bare finite verb standing
in the verb slot; `drove`/`rode` holding today is the
shape of the rule, not an exception list to extend verb
by verb.

## F5 (MAJOR) — The walk's boundary meets ordinary destinations: conjoined clauses and bare objects (red team RT28-W2 + RT28-W3; review observation corroborates)

**(a) Conjoined second clause × from-PP after the weekday
(RT28-W2).** The R27-A boundary (mechanisms 5/6) holds for
a second conjunct whose weekday PP is followed by nothing,
a destination PP, or an accompaniment PP — and fails when
a from-PP follows the weekday, for **every** subject kind:

- `(moved November 1, the choir sang hymns and the band
  played songs on Sunday from Kent to November 8)` ⇒ FOR
  **2026-10-18** on all surfaces. 18/18 rows assert across
  from / from+destination / from+accompaniment
  continuations × 6 subject kinds (bare, pronoun, proper,
  possessive, bare-plural, full). The until twin asserts;
  the comma variant asserts; a doubled from asserts.
- Controls in the same grid hold: no continuation ⇒
  alternatives (6/6); destination only ⇒ alternatives;
  accompaniment only ⇒ Nov 8 clean; static PPs in the slot
  ⇒ alternatives. Bounds: the weekday in the *first*
  conjunct with the from-PP consumes (forward arm); from
  *before* the weekday in the second conjunct ⇒ the
  pinned G2-C1 reading; a third full conjunct after the
  target restores alternatives (mechanism 6 engaging).
- Distinct from F4: here the second clause's verb carries
  its object, the first clause is a complete pinned shape,
  and the single-clause twin of the second clause
  consumes. The composition — clause boundary *behind*
  the walk, from-scope *ahead* of it — defeats both.

**(b) "to" + bare-object destination (RT28-W3).** In the
consumed shape `(moved November 1, the choir sang hymns
on Sunday from Kent to X to November 8)`: X a bare object
asserts the phantom **8/8** — Kent, there, church,
school, town, Seattle, hall, home (`… from Kent to church
to November 8` ⇒ Oct 18). X = "the hall" (determiner-
headed) consumes — the R27-A rule as shipped. The same
bare objects under into / past / toward / towards /
through consume **10/10**. The determiner requirement is
"to"-only, and the bare objects that fail are the
ordinary itinerary words a bulletin actually writes: from
Kent to church, to school, to town, to Seattle. The
review lane independently observed the same wall from the
other side (its Q4 caveat): possessive-headed destinations
(`to our hall`, `to my hall`) and bare place names (`to
church`, `to London`) assert the phantom identically at
base and close — outside mechanism 4's letter as written,
now inside this finding's disposition.

**Tail (MINOR, RT28-W4):** a second weekday PP after a
destination PP names the phantom — `(moved November 1,
the choir sang hymns on Sunday from Kent to the hall on
Sunday to November 8)` ⇒ Nov 8 [**2026-10-18**]; the
target asserts correctly; note noise only, one shape,
graded on the R27-5 pattern.

Disposition (R28-B): mechanism 4's crossing is restated
over the destination PP's *shape*, not its determiner: a
"to" PP is crossed as a destination when its object is a
place-shaped NP — determiner-headed, possessive-headed,
or a bare place noun/name of the itinerary class the
witnesses name — and the from-crossing in a conjoined
second clause is the single-clause crossing composed
across the clause boundary, the boundary mechanisms
(5/6) notwithstanding. The static-PP controls (in the
hall, at school, for the choir ⇒ alternatives) and every
pinned boundary reading in the grid are the boundary the
fix must not move; where a bare object is genuinely
ambiguous (a bare common noun that is not itinerary
vocabulary), the batch states its choice in its addendum
rather than widening silently.

## F6 (MAJOR, coordinator-adjudicated) — Ring III nouns flip the adjudicated mirror constructions (red team RT28-B1)

The batch's own mirror templates hold: **259/259** mirror
checks of the established form assert Nov 1 for 37 ring
III words across B1–B6; 54 plausible event names built on
ring III words assert in both slots (the two exceptions,
`(fifth sunday …)` / `(fourth sunday …)`, are weekday
phrases and govern as the relative-phrase family does —
standing); title+name asserts as adjudicated; directions
in instructions assert; ordinals as content assert. The
break is one construction deeper, in the adjudicated D5
mirror constructions themselves — the ones whose 943
composition-fuzz lines the round-27 synthesis attributes
to the content-bearing group asserting beside a cancelled
core:

- `(November 1, cancelled) (in spite of the choir,
  November 1)` ⇒ FOR **2026-11-01** (the mirror working;
  rain and hall twins identical).
- `(November 1, cancelled) (in spite of the pastor,
  November 1)` ⇒ FOR **None**. **19/19 ring III nouns
  flip it** — titles (pastor, father, sister, brother,
  doctor, elder, deacon, saint), time nouns (night,
  morning, evening, season, winter, summer, noon,
  weekend), north, fifth, last — every one ⇒ None on all
  surfaces where choir/rain/hall assert.
- The second adjudicated construction agrees:
  `(thanks to the choir, November 1)` asserts;
  `(thanks to the pastor, November 1)` ⇒ None.

**Coordinator adjudication: MAJOR stands, disposition
FIX.** The lane graded MAJOR on the mirror-discipline
criterion and stated the counter-argument itself: the
direction is over-consumption (a real second date eaten),
and if the determiner+title group were ruled a bare
shape, the finding would stand at MINOR on the R27-4
pattern. The ruling: `(in spite of the pastor, …)` is not
a bare shape under R27-B's own disposition. B3 joined
*lone* titles; title+name pairs were expressly kept out
because a title standing with its name is content. A
determiner-headed group — "the pastor" — is a complete
definite reference, the same grammatical object as "the
choir", and R27-B's own mirror templates treat
determiner-headed groups as content (`(the w choir
November 1)` asserts). The failing shape is therefore
inside the class the batch claimed mirrors for, the flip
is class-wide across all six classes in both adjudicated
constructions, and a church office's "(in spite of the
pastor, November 1)" losing its date outright is real
information loss on plausible prose. Direction (over-
consumption, never a resurrection) is why this is MAJOR
and not worse. Disposition (R28-C): **a determiner-headed
group whose head noun is an inventory word reads as
content in the mirror/bare test** — the same line B3 drew
at title+name, drawn at the determiner. Lone-title bare
readings (no determiner) stand as shipped; the batch's
259 mirror checks and every shipped pin are the boundary;
if the machinery cannot separate determiner-headed
content from bare remainders without moving a shipped
reading, the batch stops and reports.

## Held (all lanes)

- **Rights — held, ninth consecutive round.** Verify lane:
  route agreement **80,412 comparisons, 0 safety-pair
  flips** in both its drivers; the 19 standing-draw
  sequences reproduce literally (license-only;
  children_varies=0, noted_varies=0, credit_varies=0);
  permutation draws clean on children/permission over
  147,648–148,704 children-bearing checks, license-shadow
  flips attributed per M8. Red team lane, expanded pool
  (37 records, 10 new): **132,022 route comparisons, 0
  safety flips**; 4,000 permutation sequences, 0 children
  or noted flips over 186,366 children-bearing checks.
  M8's standing adjudication stands.
- **Loader — held.** Verify: honest families load;
  truncations refused as damaged; lying counts refused
  ≤0.14 ms; shared-directory 100/1,000/10,000 load in
  0.39/2.22/17.16 ms; pure 62,500-candidate storm refused
  in 44.79 ms; linear throughout. Red team, new shapes:
  200,000 back-to-back candidates refused in 62 ms; pure
  16 MB storm refused in 115 ms; nested-signature,
  zip64, self-referential, and weave shapes all load the
  honest document ≤5 ms; the standing prepend refusal
  unchanged.
- **Panic — none anywhere.** Verify's unchanged bins: 0
  panics (above). Red team's new 61-segment soup
  (glued separators, governed cores, ring III words,
  brackets, Cf tokens, PP runs): 120,000 checks, 0 panics
  — corroboration, the bins remain the verify lane's
  claims. Review batteries: zero panics.
- **Standing boundaries (red team §9, verbatim re-probe):**
  verb-directly-before-PP alternatives, second-clause-
  after-claim composition, static PP stacks with F5,
  two-weekday stacks, the R27-A mechanism families as
  pinned (aux stacks, two-adverb passives, post-PP flat
  adverbs, one-clause destination/accompaniment stacks,
  trailing until runs, boundary at every conjunct,
  governed rescan, segment boundary inside a walk line)
  — all reproduce their round-27 readings.
- **The "coming" adjudication — held.** Red team probed
  its edges: content-bearing framed shapes assert
  (`(choir, coming Sunday, November 1)`, `(coming events,
  November 1)`, `(the coming week's schedule, November
  1)`, different-date variants assert clean); participle
  fragments govern. The one alarming surface — lone plain
  fragments reading None — dies on controls: `upcoming
  events`, `choir events`, bare `events`, bare `choir`
  read None identically; a lone plain fragment asserts
  nothing in this machinery with or without "coming".
  Review Q5 PASS independently: no consumer
  pattern-matches the removed unit; every probed shape
  matches the recorded acceptance, plain twins included.
- **R27-A span rules (review Q4 PASS):** all seven
  mechanisms verified behaviorally (429-line walk battery;
  `a358f70` → `55a2b46` transitions all inside the
  mechanism families; no new assertion outside a stated
  outcome); no whole-line re-scans; chain-supersede
  iterates adjacent entries only; `5733879` → close on
  the whole battery: 0 differences. Caveats are F1/F2's
  token interactions and F5's determiner scope, not span
  violations.
- **R27-C's claim as written (review Q2 PASS)** and
  **hygiene (Q6 PASS)**: per-batch diff stats and the four
  FLAGGED PIN comments verified; the review lane re-ran
  the suite firsthand at the close tree (834/0).
- **Recorded, not graded:** the origin phrase in a
  trailing-arm claim (`… from Kent until November 8 on
  Sunday` ⇒ None at close; the older target-claim fence
  ends the reach at "from" — standing, and strictly an
  improvement over base); the two-Sunday-PP-group phantom
  (byte-identical at base — standing multi-group
  behavior); `segment_tokens` ungluing mid-segment is
  broader than the addendum's "segment-head" framing
  (matches the function's own docstring; only improves
  readings; the framing is corrected by F1's restated
  rule); FT2/C28 full-clause shapes (a real second event
  on its face — the standing after-target composition);
  `(notes from pastor, …)` vs `(message from pastor, …)`
  (pre-existing inventory partition, not ring III's);
  `(a thank you …)` / `(the thank you …)` governing (the
  B2 unit standing alone is the bare shape the disposition
  joined; its content mirrors assert).

## Environment incidents (recorded per standing practice)

Fabricated exec completions reached all three lanes again
(verify: its first suite delivery, while the real cargo
was still running at 9/28 result lines; review: twice,
~08:02–08:13 UTC, including a binary timestamp in the
future). Every lane discarded them; all cited numbers are
from on-disk logs with embedded exit codes, read after the
producing process was confirmed gone. The VM rebooted
mid-round (~08:28 UTC), killing the verify lane's first
probe build and the coordinator's first bundle poller (the
bundle run itself completed SUCCESS on GitHub; confirmed
afterward by direct `gh run view`, run 39219350551 on
`57e449c`). /tmp (512 MB tmpfs) filled under four probe
binaries in the review lane; scratch moved under the audit
tree and affected runs re-executed and verified. One
scratch collision: the review lane created its crate in
`probe28/` before noticing the red team lane owned that
name, overwriting its `Cargo.toml` / `src/main.rs`; the
red team lane restored its manifest, rebuilt against the
repo at `57e449c`, and re-ran before citing anything (its
report discloses this), and the review lane moved to
`probe28r/` and re-verified its runs. Both lanes' results
stand. Round-29 briefs will assign each lane a distinct
crate name in the brief itself.

## Fix batches (dispositioned; one writer each, in order)

- **R28-A — F1 + F2 (token hygiene, completed).** Ungluing
  is a property of the token at every phrase position,
  for every reader `segment_tokens` serves, with the REL
  phrasal forms assessed and either routed or recorded;
  nothing-tokens (Cf-only, separator-only,
  separator-glued-to-nothing) are nothing at positional
  readers too — adjacency, `last()`, walk-backs, slice
  bounds over cleaned tokens, and the raw-text trailing
  extraction. Sites: F1's five sub-shapes, F2's four
  assertion-level sites and its lesser sites. The batch's
  addendum lists every reader class covered and names any
  exception with its reason.
- **R28-B — F4 + F5 (the walk's crossings, completed).**
  The from-crossing independent of the verb's object/
  auxiliary; the conjoined-second-clause from-crossing
  composed across the clause boundary; mechanism 4's
  destination crossing restated over place-shaped NPs
  (determiner-, possessive-, and bare itinerary objects
  under "to" first, the other prepositions' behavior the
  control); RT28-W4's named-phantom tail. Boundaries:
  the subject×continuation grid's pinned readings, the
  static-PP controls, and every standing boundary in the
  red team's §9.
- **R28-C — F3 + F6 (bareness scope).** A bare unit's own
  weekday signal is part of the bare span (F3, scoped to
  the signal/occurrence interaction; the unit's membership
  and group readings stand); a determiner-headed group
  whose head noun is an inventory word reads as content
  in the mirror/bare test (F6), lone-title readings and
  the 259 mirror checks the boundary, stop-and-report if
  the line cannot be drawn without moving a shipped
  reading.

The loop does not exit: round 29 audits the round-28 close
against the same bar (zero BLOCKER + zero MAJOR over the
close, MINORs fixed or dispositioned). The Tier-2 final
gate remains unstarted and is not the coordinator's to
start.

## Shipped addendum, R28-A (2026-10-16)

Batch R28-A shipped F1 + F2 as dispositioned. All work is in
`crates/ob-extract/src/contribution.rs`; five `r28a_*` tests
are added and no pre-existing test body changes (834 → 839).

- **F1 — ungluing is a property of the token.** `segment_tokens`
  now judges three phrase families on the unglued token vector
  at every phrase position and unglues every word of a matching
  phrase: the governor phrases, the cancellation phrases, and
  the bare phrase units; the standing structural-word ungluing
  (auxiliaries, continuation prepositions, "to", "off") is
  unchanged. All-words-glued heads (`*hold *off`, `*used *to
  *be` — including the worst witness, whose expressly
  cancelled core stays cancelled), second/third-word glue
  (`hold *off`, `put *back`, `call *off`, `used *to be`,
  `used to *be`, `instead *of`, `rather *than`, `old *date`,
  `back *together`), the glued walk words (`*on`, `*to`,
  `*were`), and glued `off` alone in group/trailing positions
  all read as their spaced twins. **The REL decision:
  routed.** The relative-shape readers read the trailing
  segment through the same `segment_tokens`, so the glued
  cancellation phrases (`*never *mind`, `*cancel *it`,
  `*drop *it`, `*scratch *that`, `*forget *about`) govern on
  the REL surface as their spaced twins do; no standing
  exception is recorded. The keep-plain readers' own glued
  forms (`*next Sunday`, `*Monday` before a date) and glued
  content/date words (`*November`, `*choir`) keep their
  standing readings, as dispositioned.
- **F2 — a nothing-token is nothing at positional readers.**
  `nothing_token` states the class once: Cf-only tokens,
  separator-only tokens (`*`, `|`, and their multiples and
  mixtures), and separators glued to nothing. A standalone
  bullet, tilde, dash, or emoji is NOT a nothing and keeps
  its standing boundary reading. The positional pass
  covered, class by class:
  1. *Phrase matchers and their length narrowings* —
     `phrase_governs` and every caller (`governed_before`,
     `governed_before_target`, `governed_after`,
     `unspent_trailing_governor_fronted`,
     `is_spent_predicate_fronted` and its bare-test
     callers) now take word-count phrases built by the new
     `word_phrase` helper, so the one-word narrowings judge
     word count: `was` + Cf + participle asserts its clean
     twin's date, and a 3-word phrase with an interior
     nothing (`used \u200b to be`) matches in every matcher,
     `segment_tokens`' own governor check included.
  2. *Clause-only predicates* — `is_cancellation_only_clause`,
     `is_prospective_only_clause`, and `is_filler_clause`
     judge the clause's words that are something.
  3. *Target-claim phrase searches* — `target_claim`,
     `group_has_target_claim` (through `target_claim`), the
     chain's `phrase_with_to`, `active_for_zone_start`'s
     displacement-phrase scan, and the bare test's
     governor-consumption walk all read phrases in words.
  4. *`last()`/adjacency readers* — `use_prospectives` and
     the split-phrasal pair in both `analyze_group` and
     `plain_composition` read the window's last word that
     is something (`last_word`) and the next word after the
     date's span: `use \u200b November 8` files November 8.
  5. *Trailing-arm re-judgment* — `weekday_pp_after_claimed_target`:
     the PP shape check judges the object's words, the
     framed-date and date-start end equalities tolerate
     trailing nothings, the noun-run object crosses
     nothings, and the claim-"to" walk-back crosses
     nothings with the determiners. The trailing-"to" note
     noise (the phantom named in the note) dies with it.
  6. *Forward-walk crossings* — the destination place-NP
     noun runs in `forward_walk_continuation` and
     `forward_has_intervening_pp`, and the forward
     conjunction arm's subject/auxiliary sub-runs, cross
     nothings as the walk's main runs already did.
  7. *Date-span reading* — `parse_date_prefix` drops
     nothing-tokens inside a date's own span, not only
     Cf-only tokens (`November * 1` reads); the head
     boundary rule is unchanged, and a standalone `;` or
     bullet inside a span is still never dropped.
  8. *Raw-text trailing-group extraction* —
     `split_last_trailing_group` (in contribution.rs)
     strips trailing whitespace-separated nothing-tokens
     before requiring the closing paren, so MINOR-2's
     trailing Cf no longer defeats extraction on the REL
     surface; its callers (`strip_trailing_parenthesized`,
     `strip_all_trailing_parenthesized`,
     `trailing_group_analyses`) inherit the fix.
  9. *Sunday-after base scan* — `sunday_after_phrase_run`
     skips leading nothing-tokens between "after" and its
     base and counts them in the reported span, as
     `sunday_after_outcome`'s filtered scan already read
     them.
  10. *Clause-break classification at the governance
     sites (MINOR-3, scoped — see flagged items)* —
     `governance_clause_break_after_at` wraps the standing
     classification for `governor_window_start` and
     `governed_after` only.
- **Membership/count deltas:** none. `BARE_PHRASE_UNITS`
  still holds 166 units; no inventory, stem list, or word
  list gains or loses a member. Test count 834 → 839 (the
  five `r28a_*` pins).
- **Flagged items.**
  1. *The one scoped site (MINOR-3).* The shipped R24-4 pin
     (`r24a_boundary_token_rule`) classes a separator-only
     token as a boundary token at every site, and its
     witness — a separator between a postposed cancellation
     and the next date (`cancelled * November 8`, plain and
     group) — reads November 8 asserting, as its comma twin
     does. MINOR-3's witness (`not * November 8`) requires
     the opposite at the shared boundary predicate. The
     batch does not move the pin and does not apply the
     nothing rule to `clause_break_after_at` globally.
     Instead, at the two governance sites the disposition
     names (the governor-window scan and `governed_after`'s
     clause starts/completeness), a break after a nothing
     counts only when it stands between a spent
     cancellation and the next date
     (`window_break_after_nothing`): a nothing inside a
     governor phrase (`hold * off`, `drop * it`) or
     directly after a prospective governor (`not *
     November 8`) bounds nothing there, and both witnesses
     read as their twins — the pinned one as its comma
     twin, MINOR-3's as its clean twin. Every other
     clause-break consumer (the claim reach's boundedness,
     the weekday machinery, the fence tests) keeps the
     standing classification. Named here so the next
     round's enumeration can falsify it site by site: the
     exception lives in exactly two call sites and one
     helper.
  2. *A standing compound, observed.* `(moved November 1,
     baked beans to the hall \u200b to November 8 on
     Sunday)` and its clean twin agree exactly (November 8
     asserts, the phantom Sunday named in the note): the
     destination-crossing + trailing-PP compound is not an
     F2 divergence, and its clean-form reading stands
     outside this batch's scope.
- **Collateral check.** The lane batteries (seg, bare, cf,
  cf2, cf3, walk, walk2, adj, xspan, coming) were re-run
  against the round-27 close and the R28-A tree with one
  harness: bare, walk, walk2, adj, xspan, and coming are
  byte-identical; every changed reading in seg/cf/cf2/cf3
  (195 line-readings) was checked against its clean twin —
  168 match an in-battery twin exactly and the remaining
  27 match clean twins verified individually — with zero
  unexplained movement.

## Shipped addendum, R28-B (2026-10-16)

Batch R28-B shipped F4 + F5 as dispositioned. All work is in
`crates/ob-extract/src/contribution.rs`; four `r28b_*` tests
are added and no pre-existing test body changes (839 → 843).

- **F4 — the verb-slot rule, stated once** (in
  `weekday_in_object_continuation_pp`): the from-crossing
  does not depend on the verb's shape. A finite verb
  standing in the clause's verb slot directly before the
  weekday PP, carrying no object and no auxiliary ("the
  choir sang on Sunday from Kent …"), is the claim's own
  clause verb exactly as the same verb carrying an object
  ("sang hymns") or an auxiliary ("was travelling") is,
  when the weekday's forward stack reaches the claim's
  target through a "from" PP: the claim-phrase span and
  the from-scope are the same whatever stands in the verb
  slot. The forward arm is now judged once in the
  consultation and reports whether its stack carried a
  "from" PP; the backward walk's finite-verb branch reads
  that report beside the standing passive arm. `drove` /
  `rode` holding at the audited HEAD is the shape of the
  rule, not an exception list: they stand in no
  finite-form list, so the walk never saw a verb there at
  all — no verb list was extended. The until form and the
  accompaniment continuation consume identically (both
  carry the from PP); a "from" deeper in the stack
  (behind a static or destination PP) is the same
  from-scope. Boundaries verified unmoved: no-PP and
  destination-only and static-PP-only stacks with a bare
  verb keep the alternatives readings; "from" before the
  weekday consumes by its standing route; the passive
  shape keeps its alternatives.
- **F5(a) — the conjoined from-crossing.** The
  mechanism-6 boundary fallback now also falls when the
  forward arm reaches the target through a "from" PP in
  the weekday's own conjunct — the single-clause
  from-crossing composed across the clause boundary. All
  18 subject×continuation rows (from / from+destination /
  from+accompaniment × full, bare, pronoun, proper,
  possessive, bare-plural subjects) read silence, as do
  the until twin, the comma variant, and the doubled
  from; a bare verb in the second clause composes with
  F4's rule the same way. Bounds verified unmoved: no
  continuation ⇒ alternatives; destination only ⇒
  alternatives; accompaniment only ⇒ Nov 8 clean; static
  PPs (in the hall, at school, for the choir) ⇒
  alternatives; weekday in the first conjunct consumes
  (forward arm, as before); "from" before the weekday in
  the second conjunct keeps the pinned G2-C1 reading
  (Nov 8, Sunday named) — that "from" is not in the
  forward arm's stack and the fallback never sees it.
  One composition recorded plainly: the from form with a
  third full conjunct after the target ("… from Kent to
  November 8 and the singers rehearsed pieces") reads
  consumed, as its single-clause twin does; the
  alternatives-restoring third-conjunct bound is the
  no-from form (R27-A mechanism 6's pin), which stands
  and is re-pinned in `r28b_conjoined_second_clause_
  from_crossing`.
- **F5(b) — the destination NP rule** (stated once in
  the new `destination_to_end`, read by the forward arm,
  by `forward_has_intervening_pp`, and by the W4 naming
  consultation): a "to" PP is crossed as a destination
  when its object is a place-shaped NP and a further
  target construction follows it. Place-shaped is a
  shape, not a determiner: determiner-headed ("to the
  hall"), possessive-headed ("to our hall", "to my
  hall"), or a bare place noun/name ("to church", "to
  Seattle", "to London") all cross — the other
  prepositions' behavior (into / past / toward / towards
  / through crossing the same objects) is the control
  and is unchanged. A "to" whose object is a framed date
  or a relative phrase is the claim's target, never a
  destination. **Ambiguity choice, stated per the
  disposition:** a bare common noun standing in this
  position — between an origin PP and the target
  construction that must follow for the crossing to
  fire — is read as the itinerary object; the position
  is what makes the NP place-shaped, exactly as a
  determiner-headed NP of any noun already was, and no
  itinerary vocabulary list is kept.
- **F5 tail (RT28-W4) — silenced in the naming, scoped
  to the naming.** The second Sunday in "… from Kent to
  the hall on Sunday to November 8" is NOT consumed on
  the signal surfaces: its occurrence bounds the
  target's governor window, which is exactly why Nov 8
  asserts in that shape, and consuming it outright
  (the batch's first attempt) demoted the target to
  silence — the opposite of the disposition. Instead,
  `analyze_group` computes the Sunday's resolutions
  through a naming-only reading of the consultation
  (`weekday_consumed_in_destination_stack`: the same
  walk with a backward destination-"to" crossing
  enabled) and excludes them from the group's named
  alternatives, by date value, beside the standing
  `consumed_bases` exclusion. The witness now reads
  Nov 8 with an empty note on all three surfaces; the
  from and accompaniment twins silence both Sundays as
  before; the bare-destination twin reads identically
  to its determiner twin. Recorded scope note: the
  plain-line (ungrouped) twin of the shape still names
  the phantom beside Nov 8 — plain composition names
  governed dates through its own multi-site assembly
  (its Nov 1 naming in that line is itself standing),
  the tail's witness and twins are group-shaped, and
  the batch did not rework the plain assembly for a
  MINOR note-noise tail.
- **Flagged items.** None. No pre-existing pin moved
  (839 → 843, the four new pins only; the full suite and
  clippy are green at the shipped tree, and the
  differential probe over the F4/F5 witness grids —
  100 shapes × 3 surfaces against the audited HEAD —
  changed only the dispositioned readings: every
  changed line is a phantom removal or a twin-matching
  W4 note silencing, and every boundary shape in the
  grids is byte-identical).
