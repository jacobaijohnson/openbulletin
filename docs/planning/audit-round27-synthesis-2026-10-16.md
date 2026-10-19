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
