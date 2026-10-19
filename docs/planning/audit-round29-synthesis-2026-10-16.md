# Audit round 29 — synthesis (2026-10-16)

HEAD audited: `f87fd8f11f1b9e85675674c6d6d06198ff6e0f6a`
("Round 28 coordinator close: R28-A f57e9ec + R28-B
f29aaaf + R28-C a96f96c verified, 845 tests, clippy
clean"). Base for diffs: round-27 close `57e449c`. Verify
claims re-run fresh against the committed close tree; all
three lanes read-only, each in its brief-assigned scratch
crate (probe29v / probe29r / probe29); repo `git status`
clean at every lane's close, HEAD unchanged. No scratch
collision this round.

**Round 29 verdict: 0 BLOCKER / 6 MAJOR / 3 MINOR — the
exit bar is NOT met.** The verify lane returned a clean
PASS on every round-28 claim (below). The review and red
team lanes consolidated (7 lane-MAJORs → 6; 5 lane-MINORs
→ 3, one arm adjudicated into F6 and one recorded as
standing): the pattern of the last two rounds holds — each
round-28 rule reproduces exactly as far as its batch
stated it, and the failures are one step past the
statement, in the falsifiable records the close itself
named. R28-A's scoped clause-break exception has two
blind spots at its own site (F1). The token-hygiene rules
miss one raw-text reader in another crate (F2) and one
phrase form (F3). The walk's verb slot and destination
rule each exclude a word class their stated rules do not
mention (F4, F5). The determiner line's reach is the
bare-unit list where its disposition stated a
definite-reference test (F6, adjudicated). The phantom
naming R28-B silenced in one shape is systematic across
its family (F7). Fix batches R29-A/B/C are dispositioned
at the end.

Verify lane: **PASS, 0 / 0 / 0.** Every claim in the
round-28 close reproduces: the test-body diff is exactly
the 11 new pins (5 r28a + 4 r28b + 2 r28c; 200 → 211
bodies, 0 removed, 0 pre-existing changed — the R28-C
recovery's stray `#[test]` never appears in the committed
diff and the r28b test is registered exactly once);
`BARE_PHRASE_UNITS` is 166 with `coming` a bare word and
no `coming sunday` unit; fresh unpiped suite **845 passed
/ 0 failed** (`CARGO_EXIT=0`), clippy `CLIPPY_EXIT=0`,
booklet **31,717 / 34,977** byte-exact. The unchanged
fuzz bins re-ran at the briefed md5s (verdict
`a9cac6c399d304bac853f2a64f683fca`, composition
`4f85c605a841329dabed1d9a2aa84f88`): verdict 40,000
lines, 0 panics, **0 violations of I1–I3** (I4/I4b
artefact counts moved 5,722 → 5,740 / 5,585 → 5,588
under round 28's reading changes — observation only, no
claim rests on them); composition 497,024 lines, 0
panics, governed resurrections **943 — identical to
round 28**, attribution complete (every RES line carries
the adjudicated `(in spite of the choir …)` mirror;
round 28's fixes did not move the census). Rights held
(below). Loader held, with one adjudication recorded:
the lane's first runs used a debug build and were
uniformly slower (pure storm 800–1,180 ms); the release
re-run reproduces the established class (shared-CD
10,000 in 35.06 ms, pure storm refused in 133.74 ms at
load 13.6), and round 28's diff touches no loader code.
All named helpers from the three addenda are present at
their claimed roles, and exactly one weekday-bearing
unit stands by enumeration (`see you sunday`).

Review lane: 0 / 3 / 2 (MAJOR-1/2/3, MINOR-1/2 below).
Red team lane: 0 / 4 / 3 (RT29-C1/T1/W1/D1, D2/W2/N1
below). Coordinator consolidation and adjudications are
recorded inside F5, F6, and F7.

## F1 (MAJOR) — The scoped clause-break exception has two blind spots at its own site (red team RT29-C1)

Round 28's close accepted R28-A's scoped exception
(`window_break_after_nothing`, read only by
`governor_window_start` and `governed_after`) and named
it round 29's first falsification target. The review lane
confirms the exception's two arms reproduce exactly and
its other consumers hold (the claim reach's boundedness,
the weekday machinery, the fence tests — twin-identical
or standing as disclosed). The red team falsifies it at
the first of its two sites (`governed_after` holds
throughout: 0 NEITHER in 136 template-D trios):

**(a) Two boundary tokens in a row after a cancellation
phrase defeat the governor window's reach-back — 88/88
shapes.** For 11 cancellation phrases × 8 double-boundary
forms (`* *`, `* |`, `, *`, `* ,`, `* ;`, `; *`, `* -`,
`- *`), the star form asserts the governed date where
comma and clean twins both govern: `(November 1,
cancelled) (cancelled * * November 8)` ⇒ FOR **2026-11-08**
on all surfaces; twins ⇒ None. On the plain trailing
surface a *single* added star suffices, because the comma
is already there: `(back together November 1) cancelled,
* November 1, 2026` ⇒ FOR **2026-11-01**; comma twin and
single-star twin ⇒ None. Mechanism: the reach-back stops
at the first clause it examines; the clause standing
between two consecutive breaks cleans to nothing, the
walk treats that empty clause as the end of the walk, and
the cancellation clause behind it is never examined. A
single nothing holds because there is no empty
intervening clause. Bounds: "scratch that" / "was
cancelled" are not in this family (their comma twins
assert identically — a standing comma reading);
prospective phrases with double boundaries read as their
clean twins; double nothings *inside* a two-word phrase
(`hold * * off`) hold.

**(b) A nothing in the second gap of "used to be" is
invisible to the exception.** `window_break_after_nothing`
checks phrases *starting* at the word before the nothing;
a three-word phrase whose first word stands two positions
back is never checked. `(November 1, cancelled) (used to
* be November 8)` ⇒ FOR **2026-11-08** on all surfaces;
comma and clean twins ⇒ None. The first gap holds (`used
* to be`); the both-gaps form breaks identically; "do not
use", the other three-word phrase, holds at both gaps
because "not use" is seen. The escaping nothing is the
*separator* form — the class the exception was written
for; its Cf form was R28-A's own fixed site.

Direction: a governed date asserting over a cancelled
sibling, on text whose only offence is this format's own
separator debris standing twice, or once in the wrong
gap. MAJOR. Disposition (R29-A): the reach-back must skip
clauses that clean to nothing when consecutive breaks
stand between it and the cancellation clause, and the
exception's phrase check must judge phrases at **every
gap position** — a phrase is seen when any of its words
stands before the nothing with the phrase's remainder
after it, not only when the phrase starts at the
immediately preceding word. The R24-4 pin's arm
(`cancelled * November 8` reads as its comma twin) and
MINOR-3's arm (`not * November 8` governs) are the
boundaries: both reproduce today and must keep doing so.

## F2 (MAJOR) — One raw-text reader outside the enumeration: `parse_flexible_date` never drops nothing-tokens (review MAJOR-1)

R28-A's positional pass covered ten reader classes, all
inside `contribution.rs`. The review lane's insertion
fuzz (19 base lines × every token position, Cf-only /
separator-only / comma twins, 167 variants) diverges from
clean twins at **exactly four positions of one base** —
an unframed month-name date line read through
`parse_flexible_date` (`crates/ob-model/src/lib.rs:626`,
unchanged since round 13; the R28-A diff touches only
contribution.rs). That reader tokenizes the raw line text
and never drops nothing-tokens:

- `November 1, 2026` ⇒ FOR **2026-11-01**; with a Cf-only
  token at any position (`\u200b November 1, 2026`,
  `November \u200b 1, 2026`, `November 1, \u200b 2026`,
  trailing) ⇒ FOR **None** on both date-line surfaces.
  Separator-only `*` insertions behave identically.
  `October \u200b 18, 2026`, `Nov \u200b 1, 2026`,
  `1 November \u200b 2026` ⇒ None likewise.

Bounds, probed: the weekday-framed form survives
(`Sunday November \u200b 1, 2026` ⇒ Nov 1, via
`framed_date_outcome` / `parse_date_prefix`, which drop
nothings); the trailing-garnish form survives; the group
form survives (`(November \u200b 1, 2026)` ⇒ Nov 1). The
loss is specific to the unframed month-name line — the
commonest For Sunday / Keep-through format. Every F2
witness base from round 28 is twin-identical under Cf
insertion at every position in this round's fuzz: the
ten classes hold; this reader was never in them.

Graded MAJOR (the lane's grading, adopted): a zero-width
space pasted into the bulletin's own date line silently
files no date at all; F2's disposition stated the rule
without qualification for raw-text readers, and its
class 8 fixed the sibling raw-text reader in the same
round. Direction is assertion-lost, the demotion case is
stated in the lane's report, and the site — the date line
itself, not a group interior — is why the grade stands.
Disposition (R29-A): `parse_flexible_date` drops
nothing-tokens like every other date reader; the batch
is cross-crate (`ob-model`) and names it; the framed /
garnish / group bounds must not move.

## F3 (MAJOR) — Glue on a bare unit's continuation words breaks the twin rule (red team RT29-T1)

The twin battery at scale (1,410 pairs): **1,391
identical; 19 differ, in exactly two families** — four
are F1(b)'s "used to be" pairs, and fifteen are one
phrase: **`see you sunday` with glue on its second or
third word** (glue-rest, glue-all, pipe, doubled, mixed;
glue on "see" alone holds, as R28-C's pinned `*see *you
sunday` — with "sunday" unglued — holds):

- Worst witness: `November 1, 2026, cancelled (while
  November 1) *see *you *sunday November 1, 2026` ⇒ FOR
  **None**; spaced and unglued-continuation twins ⇒ FOR
  **2026-11-01**. The glued farewell eats the restated
  date outright.
- Relative surface: `(back together November 1) see *you
  *sunday November 1, 2026` ⇒ REL FOR **2026-10-25**
  [2026-11-01]; every twin with "sunday" unglued ⇒ REL
  FOR **2026-11-01** []. The glued unit lets the
  relative Sunday win the line.

Mechanism as traced: R28-C's F3 protection keys on the
unit as written in the token stream and on the weekday
signal standing on its token; a glued `"*sunday"` is not
the token the signal check reads, and the segment
ungluing that saves governor phrases reaches the unit
only in the forms R28-C pinned. The twin rule as
restated by R28-A — glued == spaced at every position of
every phrase — is the claim, and a sender bolding a
whole farewell (`**see** **you** **sunday**`) is the
same debris as F1's `**hold** **off**`. Held inside this
ground: every walk-line mutation, `**`/`||` middles on
destination and spite phrases, nothing-tokens at segment
heads inside governor phrases for all 24 phrases probed.
Disposition (R29-A): the bare-unit judgment — membership
and the F3 signal span alike — reads the unit on the
unglued token vector at every position, as governor
phrases now do; the F3 witness family (plain, group,
relative, glued at each word) is the pin set.

## F4 (MAJOR) — The verb slot, round 2: conjoined bare verbs and preposition-particles (red team RT29-W1)

R28-B's verb-slot rule reproduces for the single verb
(review Q3a PASS: the rule is genuinely shape-based, no
verb list grew — 12 verbs outside the lane's 22,
participles as main verbs, modal-less imperatives,
particle verbs `met up` / `sang out`, verb + adverb,
pronoun subjects, do-support all consume). Two ordinary
verb-slot contents still defeat it in the canonical frame
`(moved November 1, the choir VERB on Sunday from Kent
to November 8)`:

**(a) Conjoined bare verbs: 17 of 20 shapes assert the
phantom, in both "to" and "until" forms (34 of 40
rows).** `… the choir sang and danced on Sunday from
Kent to November 8` ⇒ FOR **2026-10-18** on all surfaces.
Same for `sang or danced`, `worked and prayed`, `rose
and sang`, triple conjunctions, `sang and then danced`.
The exceptions prove the shape: an object anywhere in
the slot restores consumption (`sang and played songs`,
`sang hymns and danced` ⇒ None), as does a participle
(`sang while dancing` ⇒ None). The composition also
defeats the second-clause forms (`sang and sang` with a
subject-less second conjunct ⇒ Oct 18, where `sang hymns
and sang` ⇒ None). "The choir sang and danced on Sunday
from Kent" is as common a verb shape as the bare single
verb F4 fixed in round 28.

**(b) Phrasal verbs whose particle is a continuation
preposition: 8 of 35 shapes assert.** `sat down`, `sat
up`, `sat together`, `went on`, `carried on`, `came in`,
`went in`, `came over` ⇒ FOR **2026-10-18**; the particle
is read as the head of the next PP and the verb slot is
lost. Controls hold (27/35 consume: `sang out`, `met
up`, `got up`, `stood up`, `lay down`, `went out`, `came
back`, …).

One finding, two arms: the slot holds verbal material
the walk does not cross. Disposition (R29-B): the
verb-slot judgment covers the slot's whole verbal
material — a conjunction of bare finite verbs is the
clause's verb exactly as one bare verb is, and a verb's
own particle (a preposition-form word standing in the
particle position directly after the verb, before the
weekday PP) is part of the verb, not the head of the
next PP. The object/participle exceptions and every
pinned boundary (static PPs ⇒ alternatives; no-PP and
destination-only shapes; the §9 standing boundaries)
must not move.

## F5 (MAJOR) — The destination noun run excludes verb-form words; the stated ambiguity choice does not hold for them (review MAJOR-2; red team RT29-W2's first arm)

R28-B's ambiguity choice as shipped: a bare common noun
in destination position reads as the itinerary object,
position makes the NP place-shaped, no vocabulary list
is kept; determiner-headed objects cross as a class. The
code's noun run in `destination_to_end` is `noun_like_
word(w) && !is_finite_verb_form(w)`, so a destination
head that is also a finite verb form is not a destination
at all. In the batch's own consumed frame `(moved
November 1, the choir sang hymns on Sunday from Kent to
X to November 8)`:

- X = `work` ⇒ FOR **2026-10-18**; X = `the work` ⇒
  **2026-10-18**. X = `Rose` / `the Rose` ⇒ **2026-10-18**;
  X = `Reading` / `reading` ⇒ **2026-10-18** (cleaning
  lowercases; capitalization does not save them).
- Controls consume in the same frame: `rehearsal`, `bed`,
  `tears`, `lunch`, `pieces`, `church`, `sleep`, `camp`,
  `Mark`, `the tears`, `the hall` ⇒ None.

So the stated choice holds for non-itinerary nouns
(`tears`, `pieces`, `sleep` all read as itinerary, as
the addendum says) and fails exactly for genuine
itinerary vocabulary that coincides with a verb form —
`work` (a stem in `FINITE_VERB_STEMS`), `rose` (an
irregular finite form), `reading` (the read stem's -ing
form). "From Kent to Reading" and "to work" are ordinary
bulletin travel prose; the phantom asserting is the
worst direction. The red team graded its motion-frame
arm MINOR (strained whole frame); the review lane's
frame is the canonical moved-note frame, and the grade
stands at MAJOR on its witnesses. Disposition (R29-B):
the destination NP's shape is judged as an NP — a word's
finite-verb-form membership does not remove it from the
noun run when it stands as the object of "to" in the
destination position with a further target construction
after it; the determiner-headed arm is the same rule
(the run's exclusion applies in both arms today). The
verb-form exclusion's other consumers, if any, are the
batch's to enumerate: the fix is scoped to
`destination_to_end`'s run unless the batch addendum
names another site with its reason.

**Tail (MINOR) — abbreviation-period destinations.**
`to St. Mary`, `St. Paul`, `Mt. Vernon`, `Dr. Lee's`,
`the Rev. Lee's`, `Ft. Worth`, `Ste. Marie` — 9/9 read
FOR **2026-11-08** [] where their unperioded twins and
the whole destination grid read None: the period ends
the destination's noun run and the crossing is lost.
The resulting reading (target asserts, empty note) is
family-attested elsewhere, hence MINOR — but "to St.
Mary" is a canonical bulletin destination (a choir
travelling to another church) and the split is the
period alone. Disposition (R29-B): an abbreviation
period inside a destination name does not end the noun
run. The rest of the destination grid holds and is sized
in the red team's report (two-word bare places,
possessive people and houses, event nouns — 60+ rows
cross in both "to" and "until" forms).

## F6 (MAJOR, coordinator-adjudicated) — The determiner line's reach is the bare-unit list where the disposition stated a definite-reference test (review MAJOR-3; red team RT29-D1 + RT29-D2)

R28-C's rule as stated: an article reached through a
complex preposition's tail, or standing first on its
side, heads the reference. As implemented,
`determiner_headed_content` gates on (i) the nearest
preceding word being exactly `the`, and (ii) that
article not standing as the object of a lone bare
preposition outside a consumed phrase unit. Three arms
of ordinary definite reference fall outside the gates —
every witness reads FOR **None** beside the cancelled
sibling on all surfaces, where the `the`-headed twin and
the choir/rain/hall twins assert 2026-11-01:

1. **Non-unit complex prepositions** (both lanes; red
   team RT29-D1). 30 unit-headed complex prepositions
   hold for every inventory head noun; **15 complex
   prepositions that are not `BARE_PHRASE_UNITS` members
   flip every head class to None** while their choir
   twins assert: because of, as for, apart from, aside
   from, instead of, except for, but for, save for, as
   of, off of, inside of, outside of, in between, in
   need of, at the time of. `(November 1, cancelled)
   (because of the pastor, November 1)` ⇒ None;
   `(because of the choir, November 1)` ⇒ Nov 1; `due to
   the pastor` (a unit) asserts. The flip reproduces in
   the after slot and on the plain surface; bounds are
   the same-date mirror only, as in round 28's F6. The
   only line between "due to the pastor" (asserts) and
   "because of the pastor" (eaten) is unit membership.
   "Because of" is the commonest causal preposition in
   office prose.
2. **Demonstrative determiners** (review arm 1; the red
   team graded this arm MINOR in RT29-D2 and stated the
   counter-argument). `in spite of this pastor` /
   `that pastor`, `thanks to this pastor`, `in spite of
   this winter` / `this season` / `this weekend` /
   `that morning` / `thanks to that season` ⇒ all None.
   The batch's edge 2 argues only the possessive and
   indefinite exclusions; `this`/`that` are definite
   determiners and were never adjudicated.
3. **Bare-class words between article and head** (review
   arm 3). `in spite of the same morning` / `the other
   morning` / `the same pastor` / `the other pastor` ⇒
   None — while `the old pastor` / `the senior pastor`
   assert and `the last pastor` asserts (`last` is
   itself a head noun). Edge 1 excludes `the same` *as a
   head*; in `the same morning` the head is `morning`
   and `same` is its modifier — that shape was not
   adjudicated.

**Coordinator adjudication: MAJOR stands; the
disposition is restated over the test the round-28
ruling actually used.** That ruling admitted "the
pastor" because it is "a complete definite reference,
the same grammatical object as 'the choir'". By that
test: demonstratives `this` / `that` heading an
inventory noun are complete definite references and are
IN (the red team's MINOR grading rested on the batch's
mechanical letter, "the only"; the principle governs the
letter, as it did in F4/F5); an article reached through
**any** complex preposition's tail heads the reference,
unit member or not — a complex preposition is a
two/three-word preposition phrase, a shape, not a list;
and a bare-class **modifier** between article and head
does not defeat the head (the head governs the reading;
`the same` standing as a head keeps its shipped
exclusion). **One arm is recorded as standing:** lone-
preposition objects (red team RT29-D2(i) — 20 lone
prepositions × 9 head nouns flip, and the with/for sweep
over all 44 head nouns flips 44/44: `(with the pastor,
November 1)` ⇒ None where the choir twin asserts). The
batch stated the exclusion mechanically and its reason
— an article standing as a lone bare preposition's
object belongs to that preposition's phrase, and the
bare inventory's design treats PP remainders as bare —
generalizes past the temporal ring that motivated it:
extending the line to every lone preposition would put
the dispositioned B1 phrase ring ("in the morning" and
its family, pinned bare) and the bare PP remainders at
risk to fix an over-consumption in a fragment-shaped
group, the weakest mirror form. The mechanical line
stands for lone prepositions; a future lane with a
stronger witness (a lone-preposition group carrying
clear content of its own) may re-raise it. Disposition
(R29-C): restate `determiner_headed_content`'s reach as
(i) definite determiners `the` / `this` / `that`;
(ii) article reach through any complex-preposition
tail, judged as a phrase shape; (iii) bare-class
modifiers between determiner and head do not defeat the
head. Edges that stand as shipped: head classes only;
possessives and indefinites excluded; the lone-
preposition block. The batch's 259 mirror checks, the
lone-title readings, and every shipped pin are the
boundary; stop-and-report if the restated reach cannot
be drawn without moving one.

## F7 (MINOR) — The phantom naming is systematic across the destination+trailing family (red team RT29-N1; review MINOR-2)

R28-A recorded the compound `(moved November 1, baked
beans to the hall \u200b to November 8 on Sunday)` as
standing: Nov 8 asserts, the phantom named in the note.
At scale it is the whole family, not one shape (red
team, 72 rows): the subjectless destination+trailing
grid — 5 destination shapes × 5 trailing orders ×
to/until, **50 rows — every row reads FOR 2026-11-08
[2026-11-01, 2026-10-18]** except the accompaniment-
closing trailing, which names Nov 1 only; two-
destination and reordered stacks agree. The plain-line
W4 twin names identically (`moved November 1, the choir
sang hymns on Sunday from Kent to the hall on Sunday to
November 8` ⇒ Nov 8 [Nov 1, Oct 18]) — as R28-B's scope
note recorded it would; its destination variants name
identically (7/7). And the naming consultation R28-B
shipped has one unconsulted shape inside the group
surface itself (review MINOR-2): with `on next Sunday`
as the second PP, the group reads Nov 8 **[2026-10-25]**
— `stack_consumed` admits only entries whose start token
is exactly `"sunday"`, so a relative-phrase Sunday is
named where its bare twin is silenced.

Note-noise only — the target asserts correctly in every
row, no date is eaten, no phantom asserts — MINOR on the
R27-5 pattern, dispositioned once for the family:
**the W4 naming consultation covers what the destination
stack consumed, on every surface it feeds** — the
destination+trailing family in its subjectless and
subjectful forms, the plain assembly, and relative-
phrase Sundays standing in the second PP. Genuine
alternatives naming (Nov 1 named beside a target in a
reading with no destination stack) is not this family
and must not move.

## F8 (MINOR) — Sunday-after phrases are never judged unglued (review MINOR-1)

Every other phrase family the round-29 brief named
verifies twin-identical (review Q2: displacement, target
claims, prospective phrases, the chain's phrases, bare
units glued at every word, the genitive filler phrase,
the cancellation pairs; the REL routing covers every
segment reader, code-verified at nine call sites). The
Sunday-after family is the exception: `segment_tokens`
never judges it and its readers re-clean raw tokens.

- Plain: `the Sunday after November 1, 2026` ⇒ FOR
  **2026-11-08**; `the *Sunday *after November 1, 2026`
  ⇒ FOR **None**; glue on either word alone ⇒ None.
- Group: `(November 1, cancelled) (the Sunday after
  November 1)` ⇒ FOR **2026-11-08**; glue on the head
  (`the *Sunday after November 1`) ⇒ FOR **2026-11-01**
  — the phrase is lost and its base asserts instead of
  the resolved Sunday.

Not a false shipped claim (R28-A's F1 enumeration names
governor phrases, cancellation phrases, bare units, and
structural words; glued weekday words were dispositioned
standing at the `*Monday` pin) — recorded by the lane
for adjudication, and the coordinator rules FIX: a
Sunday-after **phrase** is a phrase family like the
others F1 covers, and the group arm's failure direction
(the base asserting over the resolved date, beside a
cancelled sibling) is worse than the pinned garnish
case. Disposition (R29-A): Sunday-after phrases are
judged on the unglued vector in the readers
`segment_tokens` serves. The standalone glued weekday
garnish (`*Monday` before a date) and the keep-plain
whole-line relative readers stay as dispositioned.

## Held (all lanes)

- **Rights — held, tenth consecutive round.** Verify
  lane: **80,412 comparisons, 0 safety-pair flips** in
  both drivers; the 19 standing sequences reproduce
  literally (license-only). Red team, expanded 41-record
  pool: **150,602 route comparisons, 0 safety flips**;
  4,000 permutations, 0 children or noted flips over
  184,962 children-bearing checks. M8 stands.
- **Loader — held.** Verify (release class, above);
  red team: all standing families plus 5 new storm
  shapes (max-comment candidates refused in 0.6 ms;
  sliding-signature 8 MB refused in 10 ms; zip64 locator
  storm and concatenated honest documents load the honest
  document). Absolute times inflated 3–5× by three-lane
  contention; scaling unchanged; no shape changed sides.
- **Panic — none anywhere.** Verify's unchanged bins: 0
  panics (above). Red team's extended soup (round-28/29
  machinery tokens): 140,000 checks, 0 panics —
  corroboration. Review batteries: zero panics.
- **F2's ten classes and the scoped exception's arms**
  reproduce exactly (review Q1): every class twin-
  identical under Cf insertion at every position; both
  exception arms read as recorded, in groups and on
  plain lines; the exception's *other* consumers hold.
  The breaks are F1's two shapes and F2's one reader —
  inside and beside the record, as recorded there.
- **F1's phrase families** (review Q2): displacement,
  target, prospective, chain, bare units at every word,
  genitive filler, cancellation pairs — all
  twin-identical; REL routing code-verified at nine call
  sites. The exceptions are F3's unit-continuation glue
  and F8's Sunday-after family.
- **F4's single-verb rule** is shape-based (review Q3a,
  above); **F5(a)'s conjoined crossing** reproduces
  (from/until forms, pronoun row, bare-verb second
  clause composing with F4); **the destination grid**
  outside F5's edges crosses (60+ rows: two-word places,
  possessives, houses, event nouns); **from-scope in new
  orders** consumes 14/14; **third-conjunct from-forms**
  hold.
- **F3's enumeration** stands (review Q4a; red team §6):
  exactly one weekday-bearing unit; abbreviations spawn
  no signal; the word-by-word families keep their pinned
  readings. **F6's shipped edges** hold for heads
  (`the first` asserts; `the 21st` is a date occurrence,
  not a head case), possessives, and indefinites; plural
  heads, stacked heads, and content-modifier groups
  assert; the passive-predicate lead-in check is uniform
  across pastor/choir/winter in all 17 shapes probed.
- **Standing boundaries** spot-checked verbatim (red
  team §10): serial-verb composition, static-PP stacks,
  the conjoined alternatives, F4/F5(a) canonical shapes,
  the adjudicated mirror and its R28-C pastor twin, the
  F3 witness — all reproduce.
- **Hygiene (review Q5 PASS):** all three batches
  append-only on tests (+5/+4/+2 at the test tail, one
  hunk each); no pre-existing test body changed in any
  batch; `BARE_PHRASE_UNITS` 166 at all four commits;
  R28-B extended no verb list; the R28-C recovery record
  is consistent with the committed diff (673 insertions
  / 0 deletions in exactly the claimed hunks; the stray
  attribute's absence is structural, as the close
  recorded).

## Environment incidents (recorded per standing practice)

Fabricated exec completions again (the review lane
repeatedly — builds and a battery "delivered" as
successes while cargo was lock-blocked; the red team's
first build "completed" in ~3 minutes while the real
build ran 39m 38s under three-lane contention). A VM
reboot (~12:41 UTC) killed the verify lane's first
release build mid-compile; its log carries no exit
marker and is not cited. Cross-lane load reached ~17;
loader timings are reported beside that load and rest on
scaling, not absolutes. Every cited number in all three
reports is from an on-disk log or output file with its
own exit marker, read after the producing process was
confirmed gone. Lane scratch separation (assigned crate
names) held this round: no collision.

## Fix batches (dispositioned; one writer each, in order)

- **R29-A — F1 + F2 + F3 + F8 (token hygiene, round 3).**
  The clause-break reach-back skips empty clauses
  between consecutive breaks, and the exception's phrase
  check judges phrases at every gap position (F1);
  `parse_flexible_date` in ob-model drops nothing-tokens
  (F2, cross-crate, named); bare-unit membership and the
  F3 signal span read the unit on the unglued vector at
  every position (F3); Sunday-after phrases are judged
  unglued in the readers `segment_tokens` serves (F8).
  Boundaries: the R24-4 and MINOR-3 arms; the framed /
  garnish / group forms around F2; the `*Monday` garnish
  and keep-plain readers outside F8's phrase judgment.
- **R29-B — F4 + F5 + F7 (the walk, round 3, and the
  naming family).** The verb slot judged over its whole
  verbal material (conjoined bare verbs; verb particles)
  (F4); the destination noun run judged as an NP —
  verb-form membership does not remove a destination
  head, in bare and determiner arms alike, and an
  abbreviation period inside a destination name does not
  end the run (F5 + its tail); the W4 naming consultation
  extended to the destination+trailing family on every
  surface it feeds, relative-phrase Sundays included
  (F7). Boundaries: the object/participle exceptions,
  static-PP alternatives, genuine alternatives naming,
  and the §9 standing boundaries.
- **R29-C — F6 (the determiner line's reach, restated).**
  Definite determiners `the` / `this` / `that`; article
  reach through any complex-preposition tail judged as a
  phrase shape; bare-class modifiers do not defeat the
  head. Lone-preposition objects recorded as standing
  (adjudication above). Boundaries: head classes,
  possessive/indefinite exclusions, the 259 mirror
  checks, lone-title readings, every shipped pin;
  stop-and-report if the restated reach cannot be drawn
  without moving one.

The loop does not exit: round 30 audits the round-29
close against the same bar (zero BLOCKER + zero MAJOR
over the close, MINORs fixed or dispositioned). The
Tier-2 final gate remains unstarted and is not the
coordinator's to start.

## Shipped addendum, R29-A (F1 + F2 + F3 + F8)

Shipped as one batch on the round-29 synthesis tree
(5842128). Files: `crates/ob-extract/src/contribution.rs`
and `crates/ob-model/src/lib.rs` (the one cross-crate
site F2 names), plus this addendum. Suite **850 passed
/ 0 failed** (845 + 5 new `r29a_*` pins, zero
pre-existing test bodies changed), clippy
`--workspace --all-targets -- -D warnings` exit 0,
booklet **31,717 / 34,977** byte-exact.
`BARE_PHRASE_UNITS` unchanged at 166 (byte-identical
block).

- **F1(a) — double boundaries.** In
  `governor_window_start`'s reach-back, a clause that
  cleans to nothing (the empty clause standing
  between two consecutive breaks) is now skipped
  (`continue`) instead of ending the walk, so the
  cancellation clause behind it is examined. All 88
  shapes (11 phrases × 8 double-boundary forms) and
  the plain `cancelled, * November 1, 2026` form are
  pinned governing on FOR / KEEP / REL. Boundaries
  pinned standing: the R24-4 arm (`cancelled *
  November 8` ⇒ Nov 8 [Nov 1]) and the MINOR-3 arm
  (`not * November 8` ⇒ Nov 1); "scratch that" /
  "was cancelled" double-boundary forms read exactly
  as their comma twins; prospective phrases with
  double boundaries and double nothings inside a
  two-word phrase (`hold * * off`) read as their
  clean twins. `governed_after` was not touched by
  this arm and holds.
- **F1(b) — second gap.** `window_break_after_nothing`'s
  inside-phrase check now tries phrase starts at
  both of the two nearest words before the nothing
  (a phrase of at most three words straddles a
  nothing only from one of those starts), so a
  phrase is seen when any of its words stands before
  the nothing with the remainder after it. Pinned:
  `used to * be` and `used * to * be` govern on all
  surfaces; the first gap and "do not use" at both
  gaps hold as controls; the plain `used to * be`
  twin reads Nov 1 [Nov 8] as its clean twin does.
- **F2 — `parse_flexible_date` (ob-model).** The
  reader now drops nothing-tokens at tokenization.
  The nothing-token class is **restated minimally in
  ob-model**, not shared by reference: ob-model
  cannot read ob-extract's statement, so `lib.rs`
  carries its own `is_format_char` (the same Cf
  ranges contribution.rs states) and its own
  `nothing_token` (Cf-only, separator-only, or a
  separator glued to nothing, with the same edge-trim
  set) — the two statements name the same class, and
  the addendum records the duplication so a future
  lane checks both if the class ever changes.
  Pinned: Cf-only tokens at all four positions and
  separator-only forms across the month-name forms
  (`November` / `October` / `Nov` / `1 November`),
  on FOR and KEEP; the weekday-framed, trailing-
  garnish, and group bounds read as before.
- **F3 — bare-unit continuation glue.** Traced to
  the decisive reader: on the P7 / P1 surfaces the
  restated date is salvaged by `date_line_inner`'s
  weekday-before-a-trailing-date fallback, which
  read the unit's weekday word off the raw token —
  a glued `"*sunday"` was not the weekday the check
  reads, the salvage failed, and the composition
  governed the date (P7) or let the relative Sunday
  win (P1 REL). `segment_tokens` already unglues the
  unit in all forms (verified in isolation: its
  output is byte-identical for the clean and glued
  forms), and `occurrence_is_bare`'s membership and
  signal-span checks read that vector, so they
  needed no change. The fix is scoped to that
  reader: the fallback now also accepts a weekday
  word that ends a bare phrase unit judged on the
  unglued token vector (`bare_unit_weekday_at` —
  unit membership by `bare_phrase_unit_span` over
  unglued, `group_token`-cleaned words). A glued
  weekday ending no unit (the standalone `*Monday`
  garnish) is untouched by construction. Pinned:
  the unit glued at each word and in combination
  (star and pipe forms) across the P7, P1-REL, and
  trailing surfaces, with the R28-C originals
  standing (the trailing forms name Oct 18 beside
  Nov 1, as shipped).
- **F8 — Sunday-after glue.** The phrase is judged
  unglued at three sites, all inside contribution.rs:
  `segment_tokens` unglues "sunday" + "after" when
  they stand as consecutive words (the phrase block
  beside the governor / cancellation / bare-unit
  blocks); the shared phrase-run reader
  (`sunday_after_phrase_run`) reads its two words
  unglued, which also covers the readers that clean
  with `group_token` only (the second-date scan,
  `sunday_after_consumed_bases`); and the whole-line
  reader (`sunday_after_outcome`) judges its two
  candidate words unglued, which is what the plain
  witness reads through. Pinned: plain glue on
  either or both words ⇒ Nov 8 on FOR / KEEP; group
  glue on the head, on "after", or on both ⇒ Nov 8
  (the resolved Sunday, not its base). Boundaries:
  the `*Monday` garnish pin stands (asserted in the
  F8 test), and the keep-plain whole-line relative
  readers (`relative_sunday_shape`,
  `sunday_after_relative_shape`) are untouched.

Flagged items: none. No pre-existing pin was hit;
no boundary in the disposition moved; no
stop-and-report condition triggered.

## Shipped addendum, R29-B (F4 + F5 + F7)

Shipped as one batch on the round-29 synthesis tree,
completing the recovered interrupted tree: a previous
writer was interrupted mid-batch with the implementation
standing uncommitted in `contribution.rs`; the
coordinator inspected the full recovered diff and
adjudicated it sound and in-scope, and this writer
verified it against the synthesis witnesses (probe
families F4a / F4b / F5 / F7, with HEAD baselines taken
for every boundary reading), added the pin set, and
shipped it. Files: `crates/ob-extract/src/contribution.rs`
plus this addendum. Suite **857 passed / 0 failed**
(850 + 7 new `r29b_*` pins, append-only at the test
tail, zero pre-existing test bodies changed), clippy
`--workspace --all-targets -- -D warnings` exit 0,
booklet **31,717 / 34,977** byte-exact.
`BARE_PHRASE_UNITS` unchanged at 166 (the block does
not appear in the diff).

- **F4(a) — conjoined bare verbs.** In
  `weekday_in_object_continuation_pp_crossing`'s
  backward walk, `conjunction_only_since_verb`
  tracks whether nothing but the conjunction and
  its slot words has been crossed since the last
  verb; a further bare verb crossed in that state
  (with no subject run crossed since) is another
  conjunct of the same verb slot and crosses under
  the same from-scope as one bare verb
  (`conjoined_bare_cross`). Pinned: "sang and
  danced", "sang or danced", "worked and prayed",
  "rose and sang", triples, "sang and then
  danced", the subject-less second conjunct ("sang
  and sang"), and "sat" in both conjunct seats, in
  "to" and "until" forms, on FOR / KEEP / REL.
  Boundaries pinned standing: an object anywhere
  in the slot ("sang and played songs", "sang
  hymns and danced") consumes by the object rule;
  a participle ("sang while dancing") consumes.
- **F4(b) — verb particles.** The particle is
  pre-judged before the walk: a continuation-
  preposition word directly after a finite verb,
  directly before the weekday PP, is the verb's
  own particle (`particle_crossed` /
  `verb_particle_crossed`), judged by the new
  helpers `verb_segment_start` / `finite_verb_at`
  / `subject_verb_at` / `verb_slot_finite_at`, which
  factor out the finite-verb branch's own segment
  judgment. The "sat" arm stands in the same
  crossing: a weekday-abbreviation word that is
  also a finite verb form, standing in the verb
  slot behind a subject, is the clause's verb, not
  an earlier PP's weekday object. Pinned: "sat
  down", "sat up", "sat together", "went on",
  "carried on", "came in", "went in", "came over"
  consume; the controls ("sang out", "met up",
  "got up", "stood up", "lay down", "went out",
  "came back") consume as before. Boundary pinned
  standing, verified twin-identical against the
  R29-A tree: a weekday object in a PP stack
  ("... sang hymns on sat from Kent on Sunday")
  keeps its weekday reading.
- **F5 — the destination noun run judged as an
  NP.** `destination_noun_word` admits a noun-like
  word or a finite verb form whose spelling is a
  noun in this position (the `noun_like_word`
  exclusions — auxiliaries, governor words,
  possessive determiners, weekday words, tolerated
  adverbs — apply to the verb-form arm alike), and
  `destination_to_end`'s run reads it in the bare
  arm and the determiner-headed arm alike (both
  read this one run). Pinned: X = work / the work /
  Rose / the Rose / Reading / reading / the
  Reading cross (read silence) in "to" and "until"
  forms; the controls (rehearsal, bed, tears,
  lunch, pieces, church, sleep, camp, Mark, the
  tears, the hall) cross as before. Scope, as
  dispositioned: the fix is scoped to
  `destination_to_end`'s run; the same
  noun-plus-verb-form pattern at other sites
  judges non-destination runs (object and
  continuation material) and is untouched, no
  other site being named by F5's witnesses.
- **F5 tail — abbreviation-period destinations.**
  `clause_break_after_at` now judges the
  abbreviation shape in the destination position
  (the object of a "to", directly or behind its
  determiner) by its form — a short alphabetic
  word — with the same name-after requirement the
  PP arm has always carried, so a period inside a
  destination name does not end the run; a period
  anywhere else still breaks the clause. Pinned:
  St. Mary, St. Paul, Mt. Vernon, Dr. Lee's, the
  Rev. Lee's, Ft. Worth, Ste. Marie cross; their
  unperioded twins are unchanged.
- **F7 — the naming consultation covers the
  destination+trailing family.** The naming
  reading (`cross_destination_to`) is threaded
  into `weekday_pp_after_claimed_target`, so a
  trailing Sunday standing after a target whose
  construction carries a destination stack is
  consumed for naming exactly as one inside the
  stack is; the signal reading is unchanged (its
  callers pass the flag unset). `stack_consumed`
  in `analyze_group` is extended to relative-
  phrase Sundays (asked at the Sunday word, the
  last word of the entry's span), and
  `plain_composition` computes the same set and
  excludes it from `second_dates` on both its
  assembly paths. Pinned: the target asserts
  November 8 and the consumed Sunday resolutions
  (October 18, October 25) are named nowhere in
  the family — subjectless and subjectful group
  forms (the subjectful forms name nothing at
  all), the two-destination stack, the trailing
  relative form, and the plain assembly.
  Boundaries pinned standing, each verified
  twin-identical against the R29-A tree: the
  explicit moved date (November 1) is not stack
  material and keeps its standing naming in the
  subjectless and plain shapes, as the family's
  accompaniment-closing rows always read; genuine
  alternatives naming in readings with no
  destination stack is unchanged; the subjectful
  trailing shape (a trailing Sunday after the
  target asserting as the reading, the target
  named) stands as shipped, outside the family.

Flagged items: none. No pre-existing pin was hit;
no boundary in the disposition moved; no
stop-and-report condition triggered. Environment
note, recorded per standing practice: a service
restart during this batch's verification killed the
first full-suite run before its log persisted; the
suite, clippy, and booklet numbers above are from
the re-run, each redirected to a durable on-disk
log with its exit marker appended and read after
the producing process was confirmed gone.
