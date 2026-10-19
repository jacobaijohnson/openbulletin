# Convergence audit round 27 — synthesis (2026-10-19)

Base: round-26 close `a358f70` (817 tests), over a fresh cold
build after Jacob's clean (the close tree re-verified cold: 817
passed / 0 failed, booklet 31,717 / 34,977 byte-exact). Three
blind read-only lanes; the verify lane carried the mandated new
claim set (the unchanged fuzz bins re-run as claims).

- **Verify: PASS, 0 BLOCKER / 0 MAJOR / 1 MINOR observation.**
  Every round-26 claim re-checked: the programmatic test-body
  diff shows exactly the 19 new pins and exactly the one flagged
  R26-B change; `BARE_PHRASE_UNITS` exactly 117;
  `is_subject_preposition` byte-identical to the round-25 tree;
  fresh unpiped run 817 passed / 0 failed; clippy exit 0;
  booklet byte-exact. **The unchanged fuzz bins complete: the
  verdict bin (40,000 lines) with 0 panics (was 214) and the
  composition bin (497,024 lines) with 0 panics (was 4,760);
  verdict contract 0 violations of I1–I3.** The gap that let
  round 25's PASS ship the F1 panic is closed by evidence, not
  intention.
- **Review: 0 BLOCKER / 3 MAJOR / 2 MINOR** (R27-1…R27-5 below).
- **Red team (replacement lane): 0 BLOCKER / 7 MAJOR / 5 MINOR**
  (RT27-W1…W4, RT27-B1…B6, RT27-C1 below). Panic hunting clean
  at 456,900 checks (0 panics, 0 hangs); rights and loader held
  (below).

**Deduped: 0 BLOCKER / 10 MAJOR / 7 MINOR. The exit bar is not
met; the loop continues.** Round-26 bundle 39199205618 completed
SUCCESS on the close commit.

## Environment incidents (recorded plainly)

Three, all absorbed without losing a number that matters.
(1) The VM rebooted mid-round and the exec runtime delivered
**fabricated completions** for the verify lane's first suite,
clippy, and booklet runs — reported numbers matched expectations
while the real processes were dead or lock-queued; the same
failure the round-26 red team recorded, and the same shape as
this round's first bundle watch returning exit 0 while the run
was still in progress (its log was then deleted by the
coordinator's own post-close `cargo clean`; the bundle's success
was confirmed independently through `gh run view`). The verify
lane discarded every fabricated delivery; all its numbers come
from on-disk logs with cargo's exit code embedded, read after
the process was confirmed gone. Every lane's cited result in
this round is disk-sourced. (2) The original red team lane was
interrupted twice, resumed twice, and then errored terminally
without filing; a replacement lane inventoried the surviving
raw material (probe drivers and partial outputs under
`~/workspace/rt27-work/`), treated all of it as unverified, and
re-ran everything it cites — its report carries the re-run
ledger. (3) Lanes shared scratch space in `rt27-work/` (one
lane's driver copies appeared in another's probe crate); the
md5-verified fuzz copies were unaffected, and no cited number
depends on the shared files.

One measurement correction with consequences: the composition
fuzz's governed resurrections fell from 14,266 to **1,133**,
and every round-21–25 family plus every round-26 driver family
now sits at **12–34 lines**. The 233–264 "ambient baseline"
cited in earlier rounds was co-occurrence — those families were
being counted inside lines that resurrected because of a driver
or the panic shape. With both gone, the ambient collapses an
order of magnitude. Future rounds read per-family counts in the
12–34 band as noise, not as the old ambient. Attribution of the
1,133 is complete: 943 are the adjudicated `(in spite of the
choir …)` mirror (RT26-11/D5 — the content-bearing group
asserting is the mirror working), and 190 are F4 below.

## MAJOR findings and dispositions

### F1 — The F5 walk's stack and boundary completions (review R27-1…R27-3, RT27-W1…W4)

One root, seven mechanisms: R26-C taught the walk to judge the
whole span between the claim phrase and the weekday PP, but
each crossing is still keyed to narrow local patterns, and seven
kinds of ordinary material defeat one pattern or another. In
every witness the weekday is the phantom Sunday (Oct 18, or
Oct 25 on the relative path) beside the Nov 1 → Nov 8 move.

1. **A second adverb in a passive auxiliary stack (review
   R27-1).** "(beans were **still just** served on Sunday from
   Kent to November 8)" asserts Oct 18 with an empty note;
   `passive_aux_before` (contribution.rs:1818) tolerates at
   most one slot adverb. The active-progressive twin reads
   nothing, correctly.
2. **Negation in the auxiliary stack (RT27-W2).** The walk's
   stack crossing — and the claim path's `passive_aux_before`
   — does not cross "not", so a negated stack is judged as no
   stack at all. Active: "was not singing" names the phantom
   11/12 (to-form) and asserts it 9/12 (until-form). Passive:
   **16/16 assert Oct 18 from zero adverbs** — "(beans were
   **not served** on Sunday from Kent to November 8)" flips
   the R26-C F2-1 pin with one word, the negation every office
   writes. Distinct from (1): no adverb is required, and the
   active progressive breaks with it.
3. **A flat adverb immediately after the weekday PP
   (RT27-W4).** "(… sang hymns on Sunday **still** from Kent
   to November 8)" asserts Oct 18 for **8/8 flat adverbs**;
   the -ly rows hold. The F2-2 signature one position later:
   the walk judges the token immediately behind the next PP,
   and a flat adverb there is invisible to it.
4. **A destination or accompaniment PP inside the
   continuation (RT27-W1).** A PP the walk must cross *after*
   the weekday PP whose object is a determiner-headed noun
   phrase. Destination set (to / into / toward / towards /
   past / through + place): **42/42 rows phantom-involved —
   14 assert, 21 name, 7 offer alternatives, zero clean** —
   "(… sang hymns on Sunday from Kent **to the hall** to
   November 8)" asserts Oct 18, where static "in the hall" in
   the same stack consumes: the discriminator is the
   destination preposition, not depth. Accompaniment set
   (with / without + determiner group, standing last) is
   weaker and order-sensitive, but its asserting orders are
   the natural ones: "(… from Kent **with the choir** to
   November 8)" asserts Oct 18. "Travelled from Kent to the
   hall on Sunday" is ordinary itinerary prose.
5. **The conjoined-clause boundary arms only on
   determiner/possessive subjects (review R27-2).**
   `subject_since_verb` is set only in the determiner/
   possessive branch (2337/2364; break at 2535), so "…**and
   they played** songs on Sunday to November 8" — and the
   bare-noun ("and choir played") and proper-name twins —
   silently swallow the phantom and assert Nov 8 with an
   empty note, while "and the band played" keeps both dates
   as alternatives.
6. **Two conjoined boundaries defeat the boundary that
   holds for one (RT27-W3).** With determiner-headed
   subjects throughout (so not (5)): "(… sang hymns **and
   the band played songs on Sunday and the singers rehearsed
   pieces from Kent** to November 8)" asserts Oct 18 — the
   weekday PP in a non-initial conjunct with at least one
   full clause after it. The bound is sized: weekday in the
   *first* conjunct with two further conjuncts silences
   10/10.
7. **A trailing PP after an "until" target escapes the
   trailing-run arm (review R27-3).** "(baked beans from
   Kent **until November 8 on Sunday**)" asserts Oct 18 and
   demotes the until-target to the note; the "to" twin reads
   nothing, as pinned. The red team's corroboration extends
   the run lengths to 3 and adds a relative form ("… to
   November 8 on the next Sunday on the next Sunday" → Oct
   25 asserted, Nov 8 named).

**Disposition: FIX (R27-A, with F3).** The completions are
stated as span rules, not witness patches: the auxiliary-stack
crossing tolerates the full slot pattern the pinned families
already imply — any number of listed adverbs **and** the
negation, in active and passive stacks alike, wherever the
stack is crossed (the walk and `passive_aux_before` share the
rule, not two copies of it). The post-PP slot is judged by the
same adverb classes as the stack. The PP crossing covers the
destination and accompaniment prepositions when a target
construction follows, without disturbing the static-PP
readings that hold today. The conjoined boundary arms on any
subject run that heads a clause — pronoun, bare noun, proper
name, determiner, possessive — and arms at **every** conjunct,
so a second full clause is a boundary exactly as the first
is. The trailing-run arm judges runs after an "until" target
exactly as after a "to" target. Boundaries that stand (red
team Held, §3): the verb-directly-before-PP alternatives; the
second-clause-after-completed-claim composition; the 1-PP
weekday and base-order "until" readings; 3–4 PP stacks of
static PPs (48/48 silence per shape); two-weekday stacks.

### F2 — Inventory ring III (RT27-B1, RT27-B2 MAJOR; RT27-B3…B6 MINOR)

The ring the round-25 close directed the lanes to sweep. In
every resurrecting case: all four paths resurrect, both plain
twins resurrect, and **both mirrors assert**
(`mirror_both_fail = 0` in every class) — the classes can join
without eating the mirror shapes, as rings I/II could.

- **B1 (MAJOR) — time companions.** `noon / midnight /
  tonight` and 29 kin (morning, afternoon, evening, night,
  weekend, dawn, dusk, sunrise, sunset, daytime, nighttime,
  weekday, minute, moment, lunchtime, bedtime, daybreak,
  nightfall, twilight, mealtime, teatime, suppertime,
  dinnertime, the seasons): **64/64 slots**. The phrase ring
  agrees (at noon, tomorrow morning, last night, in the
  morning, … 42/64; this lane's pairs 36/40). `tomorrow /
  today / yesterday / hour` already hold — the rest of their
  class was never enumerated, the direct parity of RT26-4's
  frequency adverbs. "Last Sunday" / "at noon" beside a
  cancelled sibling is the office's own prose, and a time
  companion cannot head an event.
- **B2 (MAJOR) — sign-off fragments.** Singles **75/98**,
  letter pairs **44/62**: sincerely, regards, faithfully,
  cordially, warmly, kindly, respectfully, gratefully,
  cheerfully, prayerfully, farewell, goodbye, goodnight,
  wishes, best, thank, thanks; kind/best/warm regards,
  sincerely yours, yours faithfully, thank you, thanks
  again, many thanks, with thanks, god bless, bless you, see
  you, see you Sunday. Held, instructively: the *greeting*
  side mostly holds already (dear, hello, greetings,
  welcome, amen, yours, truly; the plural pairs "dear
  friends / yours truly" hold) — the gap is the closing side
  of the same letters.
- **B3 (MINOR) — titles.** The full-word remainder (pastor,
  reverend, father, deacon, elder, bishop, rabbi, priest,
  doctor, sir, madam, saint, president, secretary, …):
  **92/106**; the abbreviation class (mr, dr, rev, jr, sr)
  already holds. Title + name pairs resurrect 24/26.
  **Disposition, split as the lane flagged it:** the lone
  title words join (a title alone is a fragment in search of
  a name and cannot head an event); title + name **pairs do
  not join as units** — a named person is the corpus's
  canonical content label, and "(pastor lee November 1)"
  asserting is the mirror direction, accepted on the record.
- **B4 (MINOR) — the address ring.** **98/100.** Split
  named: the direction set (north … southwest, uptown,
  downtown, midtown — RT26-4's locative family in noun form)
  joins; the street/place remainder (street, road, avenue,
  church, chapel, school, hall, …) is content-adjacent in
  the RT26-10 way and is **recorded, not fixed** — "(church
  November 1)" is an event-label fragment, and no round eats
  a content label on the bare test's say-so. The pair sweep's
  shipped-form phrases (main street, at church, in town, the
  "side of" partitives) ride B6's phrase disposition where
  they partition, else recorded with the remainder.
- **B5 (MINOR) — the number paradigm remainder.** Ordinals
  fourth…hundredth + last/previous/preceding/thrice/triple/
  quarter (**54/68**) and cardinals five…million + dozen/
  zero (**56/58**); M5 shipped only the head of both
  paradigms. The class is now enumerable to its end.
- **B6 (MINOR) — closed tails.** Pronoun tail (noone, anyone
  else, anymore, anyways, elsewhere — 10/98; the shipped
  matrix holds behind it, the tail is closed by inspection)
  and the connective/phrase tail (sooner than; the
  partitives part of / rest of / end of / top of / back of /
  front of / side of / middle of / bottom of; the
  locative/devotional phrases at church, at work, at rest,
  in peace, by faith, by mail, by name, …; last time, fourth
  time; for christmas, for easter; other words, other hand —
  88/174).
- **`above all else` (MINOR)** — a one-word extension of the
  shipped "above all", and a phrase offices write: **M7's
  first plausible member**, fixed here as a unit. M7's
  disposition is amended accordingly: the pattern is no
  longer hypothetical; its remaining merged forms stay
  recorded until another plausible member appears. (The
  sweep's other two resurrecting overlaps, "gone home
  tomorrow night" / "come home tomorrow", are the R26-B
  verb + home family — recorded, standing.)

**Disposition: FIX (R27-B)** — B1's words and phrase units,
B2's graded core, B3's lone titles, B4's direction set, B5,
B6 join the bare inventory, membership from the lane's
sweeps, mirror-checked per class; flagged pins named loudly.
Not joining, on the record: B2's abstract tail (grace, mercy,
joy, hope, faith — names and labels in this corpus), B3's
title + name pairs, B4's place-noun remainder, B1's timeword
tail (eve, morn, o'clock, pm, fortnight, century, … —
fragmentary or period nouns, recorded under B1's family), and
the event-label nouns (holiday, christmas, potluck, bulletin,
offering, meeting, … — 162/168 in office_misc, recorded:
these words *are* event labels, and `(potluck November 1)`
beside a cancelled sibling reads as a second event, the
mirror direction).

### F3 — The Cf remainder (review R27-4, R27-5; both MINOR)

R26-C's M1 made a token that cleans to nothing nothing at the
claim scan's sites; two more sites judge such a token as
material. **R27-4:** "(it was \u200b moved for November 1)"
and "(moved \u200b for November 1)" read nothing where the
clean twins assert Nov 1 — `passive_for_slot_walk` (3294)
and the active-for object scan (3438) have no empty-token
handling. **R27-5:** "on \u200b Sunday" defeats the
consultation's positional preposition detection (2288–2311):
the phantom is named in the note; the target still asserts.
**Disposition: FIX (R27-A, with F1)** — a token that cleans
to nothing is nothing at every site that reads tokens, the
for-machinery and the positional detection included; the
batch's job is to enumerate the sites, not to patch the two
witnesses.

### F4 — A trailing bare negated date across a `*` / `|` boundary (RT27-C1; verify observation O1)

The 190 new composition-fuzz lines share one shape: a bare
restatement group, a `*` or `|` segment boundary, then a
trailing bare negated date — `(back together November 1)
*not November 1, 2026` reads Nov 1. The red team's sizing
isolates the discriminator exactly: a leading negation
governs; a trailing **cancelled** segment governs backward
across the same boundaries; a trailing bare **negation**
does not — `*not` / `|no` / `|never` all assert, and adding
"cancelled" to the trailing segment restores governance.
Worst witness: `November 1, 2026, cancelled (while November
1) |not November 1, 2026` asserts Nov 1 — the trailing
fragment undoes an expressly cancelled core. Across a comma,
a dash, or bare spaces the same trailing negation governs;
the break needs the segment boundary, this corpus's standing
formatting debris (a shipped core is "November 1, 2026 *
cancelled").

The verify lane grades the same shape a MINOR observation:
proven byte-identical at `9d462f8`, so pre-existing, and in
its judgment a separator typed against "not" with no space
is the M7 synthetic class. **Coordinator adjudication: FIX,
graded MAJOR, and the verify lane's facts are adopted while
its grade is not.** Pre-existing is proven and recorded —
round 26 did not cause this. But the shape is not M7's word
salad: `*` and `|` are this format's own structural
separators, the negation is a real governor word whose
effect is lost to token gluing, and the failure direction
is the worst in the governance family — a date the sender
negated in the same line asserts, overriding an explicit
cancellation. The fix surface is the same mechanical class
as M1's (cleaning must not let a separator glued to a
governor word hide the word). **Disposition: FIX (R27-C)** —
a governor word standing at a segment head governs with its
segment, glued separator or not; the leading-position and
trailing-cancelled readings that hold today are the pins
this must not move.

## Held, with counts

- **Rights — held, eighth consecutive round, and the two
  lanes' numbers reconcile.** Route agreement (fold vs
  single-merge): **80,412 comparisons, 0 safety-pair flips**
  in both lanes' drivers. The 19 standing-draw sequences of
  rounds 23–26 reproduce literally (19/19 still flip;
  children varies in 0, permission-noted for `Some(true)`
  outcomes varies in 0, credit varies in 0 — every flip is
  license-only). The verify lane's independent census draw
  gives 20/3,000 flips and the red team's 10 over its draw —
  different seeds of the same license-shadow phenomenon,
  each lane's axis breakdown confirming no flip moves
  children, permission, or credit (children-bearing checks
  147,648–148,704, clean, the counter differing by bin as
  recorded since round 26). The M8 standing adjudication
  stands; nothing here is a finding.
- **Loader — held.** Honest families load; truncations
  refused as damaged; lying counts and directory fields
  refused fast; shared-directory multiplication (10,000
  candidates) loads in 70 ms; the 8M storms load in 28–37 ms
  and the pure 62,500-candidate storm refuses in 47 ms on a
  quiet re-run — the earlier 0.6–2.1 s figures from all lanes
  were load artefacts of three concurrent fuzz jobs, and the
  refusal is linear in the candidate count regardless.
- **Verdict contract — held.** 0 violations of I1–I3 over
  40,000 lines × 6 bulletins in both lanes' runs; the bin's
  lumped total is the designed I4/I4b anchor difference
  (5,722 / 5,585), a labelling artefact of the driver,
  recorded so nobody re-chases it.
- **The F1 panic family — closed.** 456,900 red-team checks
  plus the two unchanged fuzz bins: 0 panics anywhere. The
  `(come home November 1)` witness still resurrects — the
  R26-B verb + home family, recorded-not-fixed, standing.

## Fix batches

- **R27-A — the walk and the Cf edges.** F1 (all seven
  mechanisms, stated as span rules) + F3 (Cf-only tokens at
  the for-machinery and positional-detection sites). The
  pinned boundaries named in F1 stand; flagged pins named
  loudly.
- **R27-B — inventory ring III.** F2's joining classes (B1,
  B2 core, B3 lone titles, B4 directions, B5, B6) +
  `above all else`. Membership from the lane's sweeps,
  mirror-checked per class; the not-joining list above is
  binding on the batch.
- **R27-C — segment-head governance.** F4: a governor word
  at a segment head governs with its segment across the
  `*` / `|` boundaries. The leading-negation and
  trailing-cancelled pins are the boundaries this must not
  move.

## Shipped addendum, R27-A

Shipped on main over baseline `9a2e97d`. Full unpiped suite
**824 passed / 0 failed** (817 baseline + 7 new pins; logs
`/tmp/r27a_full.log`, cargo exit 0 embedded), clippy
`--workspace --all-targets -D warnings` exit 0
(`/tmp/r27a_clippy.log`), booklet baseline reproduced
byte-exact — imposed 31,717 / reader 34,977 (the in-suite
`booklet_export_size_stays_down` measurement). All work in
`crates/ob-extract/src/contribution.rs`; no pre-existing
test body changed; no pin flagged.

### F1 — the seven mechanisms, as shipped

- **Mechanisms 1+2 (auxiliary stacks, incl. the
  negation).** One shared slot predicate, `aux_slot_word`
  (tolerated adverbs, adverbial words, `never`, `ever`,
  `not`), now used by every stack crossing: the backward
  walk, the forward arm's conjunction crossing, the
  intervening-PP arm, and detection's auxiliary walk.
  `passive_aux_before` is rewritten on it, looping back
  over any number of slot words to the auxiliary — the
  walk and the participle test share the one rule, as
  the brief required.
- **Mechanism 3 (post-PP slot).** The forward arm skips
  `aux_slot_word` tokens at the post-PP slot and at every
  loop top; `forward_has_intervening_pp` skips them the
  same way. All eight flat adverbs now read as their -ly
  twins, and slot words standing mid-stack cross too.
- **Mechanism 4 (PP crossing).** "past" joins the
  continuation preposition set (the subject-preposition
  twin is untouched). The forward arm's PP object run no
  longer swallows a following continuation preposition,
  so each PP of a stack is crossed by its own iteration; a "to" heading a determiner-headed
  place NP with a target construction after it is
  crossed as a destination PP (in the forward arm and
  in `forward_has_intervening_pp`). Accompaniment PPs
  cross by the same iteration. Static-PP readings are
  unchanged (the suite's static-stack pins stand; the
  destination-only passive shape asserts its target,
  exactly as the consumed "for them" shape does).
- **Mechanism 5 (boundary, every subject kind).** The
  backward walk arms the boundary at once on determiner /
  possessive heads and subject pronouns
  (`SUBJECT_PRONOUNS`); a bare or proper noun run is held
  as a pending subject and arms when the conjunction
  joining the clauses is crossed behind it. The
  pending-subject distinction is what keeps the pinned
  conjoined-verb-list reading (a noun run standing as a
  conjoined verb's object crosses no conjunction there).
  All eight subject kinds now give the alternatives.
- **Mechanism 6 (boundary at every conjunct).** The
  backward walk records when it ended on the boundary
  (`boundary_break`); the consultation then consults the
  forward arm — extracted as `forward_walk_continuation`,
  which counts the full conjuncts it crosses — and the
  Sunday is consumed exactly when the target is reached
  only by crossing a further full conjunct. Reading
  note: for the lane's witness ("… rehearsed pieces from
  Kent to November 8") the consumed reading is silence,
  parity with its first-conjunct from-twin; without
  "from", the target asserts with an empty note, parity
  with its first-conjunct twin (the standing R26-C
  forward-conjunct reading). An alternatives reading is
  unreachable for the witness itself: "from" governance
  silences the target the alternatives would name.
- **Mechanism 7 (trailing "until" runs).** The backward
  walk never crosses "until" (it crosses "to", whose
  framed date is the claim's target standing inside the
  walk); the trailing arm's run walk-back excludes
  "until" as a run preposition and refuses a run slice
  containing it; and the weekday-object test behind the
  walk gained the 4-deep framed pattern (next/this,
  determiner, preposition) for the doubled relative form
  ("on the next Sunday on the next Sunday"). "until"
  runs now judge exactly as "to" runs at lengths 1–3 and
  in the relative forms; the claim-free twins read
  identically in both orders (pinned as parity), and the
  base-order 1-PP "until" reading stands.

### F3 — the site enumeration

A token that cleans to nothing is nothing at every site
that reads tokens. The enumeration covered every reader
of cleaned tokens in `contribution.rs`; sites are grouped
by what they needed.

**Fixed in this batch** (each pinned against its clean
twin in `r27a_cf_only_tokens_at_every_site`):

- The for-machinery: `passive_for_slot_walk` (empty
  tokens crossed uncounted), `passive_for_zone_start`
  (participle / phrasal pair read over the two nearest
  non-empty words; the "for" is the window's last
  non-empty word), `active_for_zone_start` (same "for";
  the object scan treats empty object words as nothing).
- The shared auxiliary walk: `aux_adverb_walk_bounded`
  (empty crossed uncounted), `aux_beyond_slot`,
  `aux_complement`.
- Positional preposition detection:
  `continuation_prep_before` and `subject_prep_before`
  now read the nearest non-empty words, and serve the
  consultation's detection, `weekday_in_subject_pp`,
  `relative_in_subject_pp`, and `date_in_subject_pp`.
- Subject-PP weekday patterns:
  `weekday_is_subject_pp_object` (the four patterns over
  nearest non-empty words), now serving both subject
  walks; `segment_subject_before` and
  `subject_head_before` also cross empty tokens outright,
  and `verb_after_pp_objects` crosses them in its object
  runs.
- `clause_break_after_at`: a Cf-only token is not a
  clause boundary (marker characters that are not
  format characters keep their standing boundary
  readings — `cf_only_token` requires a stripped format
  character, so "*" / "|" are untouched).
- The date reader: `parse_date_prefix` drops interior
  Cf-only tokens from the normalized run while the
  reported span still covers them; a Cf-only token at
  the span head keeps the boundary return.
- The governor phrase matchers (`is_governor_phrase`,
  `is_cancellation_phrase`, `is_displacement_phrase`,
  `is_target_phrase`) match over non-empty words, and
  the three governed scans judge a phrase's start at
  its first real word.
- The claim machinery: `target_claim`'s from-before-date
  test reads the date at the next non-empty position;
  `contradicted_before` reads the nearest non-empty
  token; `verb_form_is_finite` judges the noun-phrase
  test over the next two non-empty tokens;
  `weekday_modifier_at` and `weekday_is_framed` judge
  over nearest non-empty / non-Cf tokens.
- The signal scans: `dates_inside_group_signals` and
  `scan_plain_signals` detect the relative phrase and
  the Sunday-after head over non-empty tokens, with
  spans covering the empties; `sunday_after_phrase_run`
  finds its "after" over non-empty tokens;
  `sunday_after_outcome` filters Cf-only tokens at
  entry.

**Examined, already insensitive** (probed against clean
twins; readings unchanged, no code change): the claim
consultation's own walks and `target_claim`'s
clause-bounded and between-token checks (empties
exempted in round 26); `bare_phrase_unit_span` and
`occurrence_is_bare` (built on the same exempted
machinery); `fronted_stack_ok` (an empty token truncates
its stack scan in the conservative direction);
date-line relative-shape readers (they echo the sender's
words into `for_sunday_in_words`, which round 26 already
pins as the sender's own words).

### Pins added

`r27a_auxiliary_stack_crossings`,
`r27a_post_pp_slot_adverbs`,
`r27a_destination_and_accompaniment_pp`,
`r27a_conjoined_boundary_every_subject_kind`,
`r27a_boundary_at_every_conjunct`,
`r27a_trailing_until_runs`,
`r27a_cf_only_tokens_at_every_site` — each witness with
its clean twin and the standing boundary it neighbours,
on the established three surfaces.

## Shipped addendum, R27-B

Shipped on main over baseline `55a2b46` (R27-A close, 824
tests). Full unpiped suite **831 passed / 0 failed** (824
baseline + 7 new pins; log `/tmp/r27b_full2.log`, cargo
exit 0 embedded in the log), clippy `--workspace
--all-targets -D warnings` exit 0
(`/tmp/r27b_clippy2.log`), booklet baseline reproduced
in-suite — imposed 31,717 / reader 34,977 (the
`booklet_export_size_stays_down` measurement). All code
work in `crates/ob-extract/src/contribution.rs`.

### Per-class shipped membership (F2, inventory ring III)

- **B1 — time companions.** 37 bare words from the
  sweeps join (the 32-word class: noon, midnight,
  tonight, morning, afternoon, evening, night, weekend,
  dawn, dusk, sunrise, sunset, daytime, nighttime,
  weekday, minute, moment, lunchtime, bedtime, daybreak,
  nightfall, twilight, mealtime, teatime, suppertime,
  dinnertime, spring, summer, autumn, fall, winter,
  season — 64 of 64 lane slots; plus the rest sweep's
  five strong singles: midday, midweek, nightly, hourly,
  yearly), and `coming` joins beside them under the
  adjudication below (38 words in the block in total).
  One phrase unit is listed, `after lunch`; the rest of
  the swept phrase ring partitions word by word. The
  non-joining timeword tail is pinned as asserting,
  with `o'clock` and `pm` added to those tail pins in
  this batch; the event-label nouns swept in the same
  class do not join.
- **B2 — sign-off fragments.** 17 bare singles join
  (sincerely, regards, faithfully, cordially, warmly,
  kindly, respectfully, gratefully, cheerfully,
  prayerfully, farewell, goodbye, goodnight, wishes,
  best, thank, thanks; the class resurrected 75 of 98
  single slots) and 6 phrase units are listed (kind
  regards, warm regards, god bless, bless you, see
  you, see you sunday); the rest of the core's pairs
  partition word by word (44 of 62 pair slots). The
  abstract tail and the recorded letter tail do not
  join.
- **B3 — lone titles.** 46 bare words join (92 of 106
  lane slots). Title + name pairs do **not** join as
  units, per the binding split disposition in F2.
- **B4 — direction set.** 11 bare words join (north,
  south, east, west, northeast, northwest, southeast,
  southwest, uptown, downtown, midtown; the address
  ring resurrected 98 of 100 lane slots). The
  street/place remainder does not join and is pinned
  as asserting.
- **B5 — number paradigm remainder.** 55 bare words
  join to the paradigm's end: ordinals fourth…
  hundredth + last / previous / preceding / thrice /
  triple / quarter (27 words, 54 of 68 lane slots) and
  cardinals five…million + dozen / zero (28 words, 56
  of 58 lane slots).
- **B6 — closed tails.** 4 bare pronoun-tail singles
  join (noone, anymore, anyways, elsewhere) and 41
  phrase units are listed (`anyone else`, the
  partitives, the locative / devotional phrases, and
  the `for` / `at christmas` / `easter` forms among
  them; pronoun tail 10 of 98 lane slots, connective /
  phrase tail 88 of 174). Members standing alone keep
  their readings: `christmas` and `easter` are event
  labels and still assert alone.
- **`above all else`** joins as 1 phrase unit, M7's
  first plausible member, per F2.

`BARE_PHRASE_UNITS` counted in source at exactly
**166** (117 at the round-26 close + 50 dispositioned
in this batch − 1: the `coming sunday` unit replaced
by the bare word, below; net +49 listed units:
1 B1 + 6 B2 + 41 B6 + 1 `above all else`). No other
count or enumeration in the tree disagrees.

### The `coming` adjudication (coordinator, recorded here)

This addendum is the round-27 close record the code
comments cite. The red team's B1 pair/rest sweep
listed the **pair** `coming sunday` as a resurrecting
member under a FIX disposition; it did not list the
lone word. The phrase-unit mechanism provably cannot
reach the head-slot shape: probed with the unit
listed, `(November 1, cancelled) (coming sunday
November 1)` still asserted on all three group
surfaces and both plain twins, because `coming
sunday` is itself a relative date phrase in the
machinery and `coming` can stand alone in a
weekday-framed date's region. **Coordinator
adjudication, accepted for this batch: `coming` joins
as a bare word, amending R27-B's membership sourcing
for this one word, and the `&["coming", "sunday"]`
unit is removed.** The bare word reaches every group
shape and the mirror holds: `(coming sunday November
1)` and `(November 1 coming sunday)` read governed
beside the cancelled sibling on all three surfaces,
`(coming November 1)` (and its trailing twin) read
governed, `(coming sunday choir November 1)` still
asserts, and the weekday-framed witness — pinned in
exactly the group shape `(coming Sunday, November
1)`, where `coming` stands alone in the framed date's
before-region apart from the framing weekday — reads
governed on all three surfaces. Acceptance was
conditional on the final full suite and clippy above
being green; they are. One plain-surface record,
pinned as the machinery actually reads it: the plain
twins of `coming` alone read governed, while the
plain twins of the pair keep the standing
weekday-phrase relative reading (head asserts
November 1; trailing asserts October 18, the coming
Sunday's own resolution) — the same reading the
shipped `last sunday` / `previous sunday` phrases
carry on the plain surface, whose ring pins likewise
govern the groups only.

### Flagged pins (the only pre-existing test bodies changed)

Four **FLAGGED** thanks-pin flips, all the same
witness: a lone head `(thanks November 1)` stood as
asserting under the R20-3 adjudication and now reads
governed because B2 joins `thanks` as a sign-off
single, superseding R20-3 for this word. Named:

- `r26b_phrase_units_ring2` — the round-26 pin at the
  end of the test ("a lone head thanks still asserts
  (R20-3)") flipped to governed.
- `r21b_bare_inventory_function_words` — `(thanks
  November 1)` removed from the held list (rounds
  21–26) and its flipped reading pinned separately.
- `r22b_bare_inventory_function_classes_round_three`
  — the same removal and flip (rounds 22–26).
- `r23a_bare_inventory_deictics_and_remainders` —
  the same removal and flip (round 23).

**No other pre-existing test body changed.**

### Partition corrections (pins in the new R27-B tests)

Two foundation pins were repinned with comments in
the new tests, as corrections to the foundation's own
readings rather than membership changes:

- **`your pastor` governs** (in
  `r27b_signoff_fragments`): listed in the recorded
  letter tail as not joining, it in fact partitions
  word by word once B3 joins `pastor` (`your` was
  already bare), so it reads governed in both slots
  with no unit joining for it.
- **`gone home tomorrow night` governs** (in
  `r27b_above_all_else`): the sweep overlap recorded
  as resurrecting partitions once B1 joins `night`
  (`gone home` is a shipped unit, `tomorrow` was
  already bare), so it reads governed in both slots.
  Its twin stands as recorded: `come home tomorrow`
  still asserts in both slots — the R26-B verb +
  home family, recorded-not-fixed.

### Pins added

`r27b_time_companions`, `r27b_signoff_fragments`,
`r27b_lone_titles`, `r27b_direction_set`,
`r27b_number_paradigm_remainder`, `r27b_closed_tails`,
`r27b_above_all_else` — each witness on the
established three surfaces with its plain twins where
the established pattern carries them, and its mirror.

## Shipped addendum, R27-C

Shipped on main over baseline `5733879` (R27-B close,
831 tests). Full unpiped suite **834 passed / 0
failed** (831 baseline + 3 new pins; log
`/tmp/r27c_full.log`, cargo exit 0 embedded in the
log), clippy `--workspace --all-targets -D warnings`
exit 0 (`/tmp/r27c_clippy.log`), booklet baseline
reproduced in-suite — imposed 31,717 / reader 34,977
(the `booklet_export_size_stays_down` measurement).
All work in `crates/ob-extract/src/contribution.rs`;
**no pre-existing test body changed** (the diff's
only removed lines are the nine token-construction
lines in non-test code named below, each replaced
by its `segment_tokens` form). The R27-B inventory,
its pins, and `BARE_PHRASE_UNITS` are untouched, and
the `coming sunday` plain-surface readings pinned by
R27-B stand exactly as they stood — their acceptance
is a round-27 close record item, not this batch's
work, and nothing here reaches them.

### F4 — the rule as shipped

A governor word standing at a segment head governs
with its segment, glued separator or not. The rule
lives in one new reader, `segment_tokens`, which
builds the cleaned token vector every segment
reader reads: the shared cleaning (`group_token`)
plus one segment-head step. `*` and `|` are this
format's own structural separators, so a separator
standing at a token's head IS the segment boundary
— the segment it opens may open the run, follow a
standalone separator token or a clause break, or be
opened by the glued separator itself mid-run — and
the word behind it is that segment's head word.
When that head word heads a governor phrase (judged
with the words that follow it in its segment, by
the phrase matchers' own rule, so the phrasal heads
`*no longer`, `|hold off`, `*used to be` unglue with
the singles), the token is read as that word; every
other token keeps its standing cleaned form byte for
byte. Ungluing only makes the head word visible:
every contextual judgment on it (the
auxiliary-slot and subject-PP narrowings, the
determiner-headed -s rule, the for-zones, the
target-claim machinery) still runs exactly as it
runs on the spaced twin, so a glued displacement
head (`*moved to November 8` inside a group) claims
its target as its spaced twin does.

Boundaries of the fix, stated plainly: `group_token`
itself is unchanged, and the separators are NOT
format characters — `cf_only_token` and the Cf
machinery are untouched, a standalone `*` / `|`
token still cleans to nothing exactly as it always
has, and the ungluing is governor-scoped, so a
glued date word (`*November`) and a glued content
word (`*choir`) keep their standing readings (both
pinned standing in `r27c_segment_head_boundaries_stand`).

The sites switched to `segment_tokens` are the
segment readers: `analyze_group` and
`group_has_target_claim` (a group's own segments),
`chain_superseded_from_entries` (per-group chain
tokens), `plain_multi_date_reading`,
`plain_composition_reading`,
`every_governed_core_values`,
`core_composition_profile`,
`core_bracketed_bare_restatement`, and the
whole-line composition's de-grouped core scan. The
readers that judge no governor (the second-date
scan, the Sunday-after base scan, the relative-shape
readers, `group_inner_is_relative`) keep the plain
shared cleaning, as do the date readers.

Evidence beyond the pins: with the code change alone
and before any pin was added, the full unpiped suite
reproduced the baseline exactly (831 passed / 0
failed, `/tmp/r27c_suite_pre.log`) — no pre-existing
reading moved. A glued/spaced twin probe run in this
batch (32 pairs over the For, Keep-through, and
relative-group surfaces, output
`~/workspace/rt27-work/r27c_twin_out.txt`) shows 0
differences: every glued form reads byte-identically
to its spaced twin, the content mirrors included
(`(service November 1) *not November 1, 2026`
asserts as its spaced twin asserts; `*choir`,
`*notes`, `*november` heads are unchanged), and the
auxiliary-slot narrowing holds through the glue
(`beans were *not served …` reads as its spaced
twin).

### Pins added

`r27c_segment_head_governance_plain_segments` — the
witnesses verbatim on the plain surfaces (the
`(back together November 1) *not November 1, 2026`
witness, its `(please)` / `(November 1; cancelled)`
fuzz siblings, and the worst witness, whose
expressly cancelled core stays cancelled), the
`*not` / `|no` / `|never` family in both pairings,
the doubled separator, the phrasal heads, the
`*cancelled` / `*moved` governor heads, and a glued
head governing a different date than the core's;
each reads nothing and names nothing on both plain
surfaces. `r27c_segment_head_governance_groups` —
the same rule at a group's own segment heads on the
established three surfaces (a group opening on a
glued governor, a segment inside a group after its
comma, a trailing glued cancellation inside its
group), a glued head governing only its own
segment's date (`(November 1, *not November 8)`
asserts November 1 clean), and the glued
displacement claim. `r27c_segment_head_boundaries_stand`
— the boundaries that must not move, re-asserted as
standing: a leading negation governs (`not` / `no` /
`never`, plus its glued leading form), a trailing
cancelled segment governs backward across the same
`*` / `|` boundaries, the comma / dash / bare-space
trailing negations and the spaced twins stand as
they always have, and the governor-scoping guard
(a glued content word and a glued date word still
assert) holds on the plain and group surfaces.
