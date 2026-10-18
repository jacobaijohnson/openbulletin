# Convergence audit round 26 — synthesis (2026-10-18)

Base: round-25 close `9d462f8` (798 tests). Three blind read-only lanes.

- **Verify: PASS, 0 BLOCKER / 0 MAJOR / 0 MINOR.** Every round-25 claim
  re-checked: the programmatic test-body diff shows exactly the three
  flagged R25-A changes and zero in R25-B; `BARE_PHRASE_UNITS` exactly
  63, `GARNISH_NOUNS` exactly 8, pronouns 30, modals 12, abbreviations
  11, interjections 19; fresh unpiped run 798 passed / 0 failed; clippy
  exit 0; booklet 31,717 / 34,977 byte-exact; rights and loader pins ok.
- **Review: 0 BLOCKER / 5 MAJOR / 3 MINOR** (R26-1…R26-8 below).
- **Red team: 1 BLOCKER / 7 MAJOR / 4 MINOR** (RT26-1…RT26-12 below;
  RT26-12 graded MINOR on the standing rights adjudication).

**Deduped: 1 BLOCKER / 7 MAJOR / 8 MINOR. The exit bar is not met; the
loop continues.** The five F5-walk findings (review R26-1…R26-4 +
RT26-7) merge into one family (F2 below). Round-25 bundle 39133000828
and close CI 39132997622 both completed SUCCESS.

## Environment incidents (recorded plainly)

Mid-round, an external sweep (not any lane) deleted the whole
`~/workspace/openbulletin-audit-wave3/` tree and most of
`~/workspace/openbulletin/target/`; the disk had filled to 100%
shortly before. Consequences: the lane report files for rounds ≤25
and this round's review/verify report files are lost, as are the
probe sources and first output sets. What survives: this round's red
team report (rewritten after the sweep, headline witnesses
re-verified, md5s of the lost outputs recorded inside it) at
`~/workspace/openbulletin-audit-wave3/round26-redteam.md`, its
surviving outputs under `~/workspace/rt26-work/out/`, and the
review/verify lanes' handoff summaries, which stand as their record
and are reflected in this synthesis. The adjudicated record of every
closed round lives in the committed syntheses under `docs/planning/`
and is unaffected. The red team also discarded two spurious
background-exec "completion" deliveries that reported fabricated
output while the real process was still compiling; every number in
its report comes from foreground runs re-read from disk.
Process change adopted at this close, per Jacob's clean-build
direction: round-boundary hygiene cleans the repo `target/` and
probe **build** directories only; lane report files are records and
are never cleaned. Registry size is reported at each close (1.2G at
this round's start; disk headroom per the coordinator seat's
measurement, 46G free of 100G).

## BLOCKER

### F1 — `parse_contribution` panics on plausible office text (RT26-1)

The unchanged round-25 verdict fuzz no longer completes at this
HEAD: the parser panics at `contribution.rs:5435` in
`occurrence_is_bare` ("slice index starts at 7 but ends at 6" — one
occurrence's span ends after the next occurrence's start, so the
after-region slice inverts). Minimal shape, every ingredient
necessary: a content head + a framed date + a parenthesized
same-date governor group + a trailing Sunday/date signal —
`service November 1, 2026 (November 1, cancelled) next Sunday` and
ten sibling witnesses all panic; removing any ingredient reads
November 1. Scale: 214 of 40,000 verdict-fuzz lines; 4,760 of
497,024 composition lines. The identical fuzz completed at
`d705baa`: this is a regression landed by R25-A or R25-B (suspects
by site: R25-A's bracket-span mask / M7 no-span change in
`occurrence_is_bare`, R25-B's group-composition changes). A hard
crash on a line a sender can plausibly write is the worst defect
class this loop has found. **Disposition: FIX FIRST, ALONE
(R26-A).** Bisect the batch, fix the span arithmetic at its root
(not a clamp that hides an inverted region — the region bound
itself must be right), and pin the minimized witnesses plus a
fuzz-shaped regression set. The round-27 verify lane re-runs the
unchanged fuzz bins as part of its claim set.

## MAJOR findings and dispositions

### F2 — The F5 backward walk's crossing rules break on intervening material (review R26-1…R26-4 + RT26-7)

One family: `weekday_in_object_continuation_pp`'s backward walk
judges its discriminator by what stands immediately behind the
weekday PP, and five kinds of intervening material defeat it:

1. **Passive + intervening PP (review R26-1).** "(moved November 1,
   beans were served on Sunday from Kent to November 8)" asserts
   phantom Oct 18 with an empty note; base-order passives assert
   Nov 8 naming Oct 18. Crossing a preposition resets the walk's
   object-crossed flag, so the passive verb misreads as "directly
   before the PP". (A "by"-phrase alone does not flip it; the
   pinned alternatives hold there.)
2. **Flat adverbs in the auxiliary stack (review R26-2).** "was
   still/really/never/just/already/soon/also/now singing hymns
   from Kent on Sunday to November 8" asserts Nov 8 naming Oct 18
   (8/8 flat adverbs); `-ly` adverbs and `ever` in the same slot
   hold clean. The walk's adverb crossing misses the flat class.
3. **A second trailing PP (review R26-3).** "…to November 8 on
   Sunday on Sunday" (and comma / mixed bare+relative pairs)
   asserts the second PP's resolution with Nov 8 demoted; the
   single-trailing-PP pin holds, and a different second weekday
   (Monday) holds. The R25-B trailing arm judges one PP, not a
   trailing run.
4. **A "for" PP inside the continuation (review R26-4).** "baked
   beans for them / for him / for the choir / for Kent on Sunday
   to November 8" asserts Nov 8 naming [Nov 1, Oct 18]; direct
   pronoun objects hold — it is the preposition: the walk reuses
   `is_subject_preposition`, from which "for" is deliberately
   excluded (the passive-for naming rule stands on that
   exclusion). Coordinator adjudication of the flagged tension:
   the two uses separate. The continuation walk gets its own
   crossing set in which "for" is crossed when a target
   construction follows; the subject-PP set and the passive-for
   naming rule are untouched.
5. **"until" targets and conjoined clauses (RT26-7).** "to"
   consumes at 1–4 PP depths in all orders; "until" names or
   asserts the phantom from 2-PP weekday-first stacks and 1-PP
   relative objects, identically in full-clause and subjectless
   shapes, and the plain witness asserts Oct 18. Two of five
   conjoined-clause shapes assert the phantom (the second
   conjunct's subject+verb resets the walk); one names it.

**Disposition: FIX (R26-C).** The walk's crossing rules judge the
whole span between the claim phrase and the PP, not the token
immediately behind the PP: PPs inside the continuation are crossed
without resetting object material; the adverb crossing covers the
flat class as it covers `-ly` adverbs; a trailing PP **run** is
judged as the single trailing PP is today; "for" is crossed per
the adjudication above; "until" consumes exactly as "to" does at
every depth, and a conjoined clause inside the continuation does
not reset the walk. Boundaries that stand (red team Held, surface
3): a finite verb standing directly before the PP keeps its
pinned alternatives reading; a second clause standing **after** a
completed claim asserts its own Sunday (standing composition, not
part of this family); 1-PP weekday "until" and base-order shapes
keep their readings.

### F3 — The spatial-preposition remainder (RT26-2)

`above/below/inside/outside/beneath/underneath/throughout/
opposite/alongside/round/notwithstanding` resurrect 22/22 slots on
every path, mirrors asserting, at composition-driver scale
(933–960 lines each). Core spatial prepositions whose neighbours
are all in; the class has been "completed" in four consecutive
rounds and its most frequent members were in none of them.
**Disposition: FIX (R26-B)** — the eleven words join the
preposition class.

### F4 — The complex-preposition phrase ring, second turn (RT26-3)

103 of 168 candidate-unit slots resurrect; drivers `due to` (979)
and `in addition to` (950) are the two largest new families in
the fuzz. Office core: `in addition to`, `in accordance with`,
`in connection with`, `in relation to`, `with respect to`, `in
response to`, `in reference to`, `by means of`, `in exchange
for`, `in place of`, `in favor of`, `in favour of`, `on account
of`, `by reason of`, `as a result of`, `in the event of`, `due
to`, `owing to`, `prior to`, `subsequent to`, `contrary to`, `as
opposed to`, `close to`; tail: `at best/at worst/at first/at
present/for good/so far/up until/up till/down to`, the
`by`-iteration set, `first of all`, `above all`, `in the end`,
`in the meantime`, `for the time being`, and kin. Units that
partition word-by-word into inventory words resurrect nowhere
(65 slots) — the break needs genuinely new phrases, again.
**Disposition: FIX (R26-B)** — the resurrecting units join
`BARE_PHRASE_UNITS`, membership from the lane's sweep,
mirror-checked; `thanks to` joins for the head slot it
resurrected in (its trailing behaviour is the R20-3 "thanks"
adjudication and does not change).

### F5 — The general adverb remainder (RT26-4)

47 words × 2 slots = 94/94 resurrect: frequency/office
(`usually/generally/typically/frequently/sometimes/seldom/
rarely/daily/weekly/monthly/annually/lately/presently`), formal
connectives (`henceforth/forthwith/heretofore/thereby/hereby/
wherein/thereafter`), locative/directional (`somewhere/
anywhere/everywhere/nowhere/indoors/outdoors/upstairs/
downstairs/ahead/aside/apart/overnight/underway/backwards/
forwards`), ordinal adverbs (`firstly/secondly/thirdly/
lastly`), and kin. Adverbs reached the bare test only through
the tolerated list and the connective subset; the general
adverb class was never a class in the inventory.
**Disposition: FIX (R26-B)** — the general adverb class joins
the bare inventory, membership from the lane's sweep,
mirror-checked. (The aux-slot tolerated list is a different
surface and is not widened by this disposition.)

### F6 — The do-auxiliary family (RT26-5)

The full 16 × 23 pronoun × modal/auxiliary matrix (736 checks)
resurrects in **exactly** the do/does/did columns (96 cells);
`do/does/did/done/doing` resurrect 10/10 slots, mirrors
asserting; every other cell, including all "we will be / one
is / will have" triples, holds. The do-paradigm — the auxiliary
of ordinary questions and emphasis — was in no list.
**Disposition: FIX (R26-B)** — `do/does/did/done/doing` join
the auxiliary class wherever the be/have/get paradigms are
carried for the bare test.

### F7 — The recorded singletons in combination (RT26-6)

The round-24/25 recorded fragments (`ago/out/up/down/away/
home`) in combination: pairs 60/60 resurrect, triples 120/120,
the "… ago" phrase family 22/22 ("two days ago", "a week ago",
"a while ago", …). The reason they were recorded ungraded —
fragments, not plausible restatements — does not survive the
pair: "two days ago" beside a cancelled sibling is a shape the
office writes. **Disposition: FIX in part (R26-B).** The six
words join the inventory (pairs and triples then partition
word by word), and the "… ago" phrases are covered by the
number/time words joining with F9's class below. The verb +
home family (`come home`, `went home`, … 38/38) is **recorded,
not fixed**: a verb phrase standing as a restatement's only
companion is a content clause fragment, and the finding stands
on the pairs and ago phrases alone, as the lane offered.
(`away from home` / `working from home` already hold by
from-governance.)

### F8 — An abbreviation's period inside a place name breaks the from-PP (review R26-5)

"(moved November 1, baked beans from Mt. Kent on Sunday to
November 8)" — and the St. Kent / Dr. Kent twins, subjectless,
full-clause, and plain — assert Nov 8 where every unperioded
twin reads nothing: the period on the M5 abbreviation reads
as a boundary inside the PP and from-governance is silently
lost. No phantom is named; the harm is the governed target
asserting. **Disposition: FIX (R26-C)** — an abbreviation of
the M5 class keeps its period from acting as a boundary when
it stands inside a PP before a name word; the general
period-boundary rule is otherwise untouched.

## MINOR findings and dispositions

- **M1 — A Cf-only token between "moved" and its date breaks
  the claim (review R26-6).** A token carrying only U+200B /
  U+FEFF (its `*` twin behaves identically) asserts + names a
  phantom where Cf before "moved" and inside date words hold.
  **FIX (R26-C):** a token that cleans to nothing is nothing at
  every site the claim scan reads, not only where R25-A
  stripped it.
- **M2 — Unclosed bracket spans resurrect on group surfaces
  (review R26-7).** `[November 1` and `[our choir)` beside a
  governed sibling assert; closed twins and the plain surface
  hold. **FIX (R26-C):** an unclosed bracket span binds as a
  closed one does in composition; the alone-unread limitation
  stands.
- **M3 — A second clause with object material after the
  claimed date names a phantom (review R26-8).** Graded MINOR
  by the lane; after-target second clauses without the object
  material are inert. **FIX (R26-C)** with the F2 batch, same
  walk; the standing composition for second clauses after a
  completed claim is not this shape and stands.
- **M4 — The reflexive remainder and "nor" (RT26-8).**
  `myself/yourself/ourselves/yourselves/oneself/whatsoever/
  whosoever` resurrect 14/28 slots; the shipped reflexives
  hold; `nor` resurrects 2/2. **FIX (R26-B)** — one more list
  in the inventory batch.
- **M5 — The quantifier/determiner remainder (RT26-9).**
  `plenty/various/numerous/countless/fewer/lots/lot` and the
  number/ordinal words `first/second/third/two/three/four`
  resurrect in both slots. **FIX (R26-B)** — the class joins;
  the number words also serve F7's ago phrases.
- **M6 — The adjective remainder (RT26-10).** `whole/entire/
  certain/particular/specific/additional/extra/further/
  single/sole/half/double` resurrect 24/24; mirrors assert.
  **FIX (R26-B)** — determiner-adjectives, same batch.
- **M7 — One-word-overlap unit merges (RT26-11).** 7 of 20
  merged forms resurrect (`as far as long as`, `up to and
  fro`, …); stacks 0/2,450, splits 0/200, two-word overlaps
  0/15 all hold, and content extensions assert 200/200 +
  20/20 (the mirror working). The resurrecting merges are
  word salad no office writes. **RECORDED, not fixed** — the
  pattern is on record if a plausible overlap ever appears.
- **M8 — Rights permutation flips, seventh consecutive round
  (RT26-12).** Outputs byte-identical to rounds 23–25 (md5s
  verified): 80,412 route comparisons, 0 safety-pair flips;
  19/3,000 license-only permutation flips; 127,128
  children-bearing checks clean. **Standing adjudication,
  restated, no fix.**

## Held, with counts

The round-25 families are closed: the 497,024-line composition
fuzz attributes every resurrection to this round's new families
or the F1 panic shape, with all round-21–25 families at the
233–264 ambient baseline; path structure 0 `both_some_differ`,
0 `for_some_keep_none` outside the standing rounds-8/9
asymmetry. The over-consumption hunt came up **empty** — the
round-25 inventory widening eats no real date (the brief's
named family reads November 8 naming November 1 on all paths;
the 34 plain lines that read nothing match their content-word
twins exactly: baseline grammar, not consumption). The fence
garnish candidates hold at 288 checks / 0 deviations in widened
shapes. The loader holds on all families (8M storm, 62,500
candidates, 6.8 ms at load ~2.7). The verdict contract holds
wherever the parse survives (0 invariant violations over
40,000 lines × 6 bulletins) — the surface's failure is F1's
panic, not the contract. Review verified correct, no findings:
the fence under the F4 head rule in all probed shapes, and the
phrase units under different-date cancellations, claim-zone
placement, plain-surface splits, and trailing slots.

## Fix batches

- **R26-A — the panic (F1), first and alone.** Bisect, root-fix
  the inverted region, pin the minimized witnesses; no other
  item rides this batch.
- **R26-B — inventory ring II.** F3 (eleven spatial
  prepositions), F4 (phrase-unit ring II), F5 (general adverb
  class), F6 (do-paradigm), F7 (the six singleton words; ago
  phrases via M5's number/time words), M4 (reflexives + nor),
  M5 (quantifier/number remainder), M6 (determiner-adjectives).
  Membership from the lanes' sweeps, mirror-checked per class;
  flagged pins named loudly.
- **R26-C — the walk and the edges.** F2 (all five
  mechanisms), F8 (abbreviation period in PPs), M1 (Cf-only
  tokens at claim sites), M2 (unclosed bracket spans), M3
  (after-target second clause with object material). The
  pinned boundaries named in F2 stand.

## Shipped addendum, R26-A (2026-10-18)

F1 is fixed, alone, as dispositioned. **Bisect:** the lead
witness (`service November 1, 2026 (November 1, cancelled)
next Sunday`, fixture today 2026-10-16) reads November 1 at
R25-A (`9c39abf`) and panics at R25-B (`8f287ab`) — the
regression landed in **R25-B**, in its M3 change
(`core_bracketed_bare_restatement`, new in that batch).
**Root cause, in plain words:** that function built its
occurrence spans as (date, start, *end*) — each signal's
region end, i.e. the next signal's start or the line's
length — and handed them to `occurrence_is_bare`, whose
span convention is (date, start, *used*), a token count.
The bare test therefore placed an occurrence's end at
start + end, which runs past the next occurrence's start
whenever the occurrence does not stand at token 0, and the
after-region slice inverted and panicked. A content head
before the framed date is what pushes the first occurrence
off token 0, which is why every ingredient of the minimized
shape was necessary. **Fix:** the spans now carry each
signal's own token count (`s.end - s.start`), converted
exactly as `plain_composition` converts its spans; the
scan's signals are written-ordered and non-overlapping, so
occurrence regions are well-ordered by construction (each
occurrence ends no later than the next occurrence's start,
the R21-5 bounding rule) — no clamp was added at the slice
site. The span vector is local to this one function and
feeds only this one `occurrence_is_bare` call; its
neighbours — the M3 shapes the function exists for — are
re-pinned in the new test and stand unchanged. **Pins:**
one new test, `r26a_bare_restatement_spans_do_not_panic` —
the eleven minimized witnesses (the ten November 1
witnesses read November 1 naming the trailing signal; the
November 8 witness reads November 8 naming October 18, its
own pattern), the five controls (all read November 1, as
the lane recorded), the two M3 neighbour shapes, and the
lane's two fuzz-shaped sample lines pinned for
panic-freedom. No witness reads differently after the fix
than the controls' pattern predicts; **no flagged items**.
Suite: 799 tests (798 + 1 new), clippy clean.

## Shipped addendum, R26-B (2026-10-18)

Inventory ring II is shipped, as dispositioned. All
additions land in `crates/ob-extract/src/contribution.rs`
(the bare test's battleground); the mirror held for every
class (a content label standing with a new word — "(above
choir November 1)" and kin — still asserts, pinned per
class). **F3:** the eleven spatial prepositions join
`bare_filler_word`. **F4:** the 52 resurrecting units of
RT26-3's sweep join `BARE_PHRASE_UNITS` (the 25 office-core
units, the 26 discourse/idiom-tail units, and "thanks
to"), and two further units join under F7 below, bringing
`BARE_PHRASE_UNITS` from **63 to 117**. "thanks to" pins
exactly as the machinery does it: the unit matches as a
unit in head position and the head witness governs on
every path; the trailing witness also governs, as it
already did word by word before this batch (trailing
"thanks" bare by the R20-3 rule, "to" bare), so the unit
changes nothing there — and a lone head "thanks" still
asserts, the R20-3 adjudication untouched. No surprise was
found. **F5:** the general adverb class joins the bare
inventory, exactly the lane's 47 words; the aux-slot
tolerated-adverb list is NOT widened. **F6:** the
do-paradigm (do / does / did / done / doing) joins the
auxiliary class as carried for the bare test
(`bare_filler_word`); the claim machinery's own auxiliary
walks are a different surface and were not widened. The
do columns of the lane's 736-cell matrix now govern, with
"(we do November 1)" and "(they did November 1)" pinned
plus matrix spot-cells in and outside the do columns.
**F7 (in part):** the six singleton words (ago / out / up /
down / away / home) join; representative pairs and
triples partition word by word, pinned. The "… ago"
phrases partition through M5's number words, the
time-unit words that join with them (day / days / week /
weeks / month / months / year / years / hour / hours /
time, each pinned on its own), and two phrase units —
"long time ago" and "bit ago" — chosen so "long" and
"bit" standing alone keep their readings ("(long
November 1)" still asserts, its R25-A pin unmoved).
"away from home" / "working from home" still hold by
from-governance, pinned. The verb + home family is NOT
added, per adjudication: "(come home November 1)" and
"(went home November 1)" still assert, now pinned as
recorded-not-fixed. **M4:** the reflexive remainder
(myself / yourself / ourselves / yourselves / oneself /
whatsoever / whosoever) and "nor" join. **M5:** plenty /
various / numerous / countless / fewer / lots / lot and
first / second / third / two / three / four join. **M6:**
the twelve determiner-adjectives join. A claim-zone spot
check pins real claims wearing new words still claiming:
"given the whole choir moved November 1, 2026 to November
8, 2026" asserts November 8 naming November 1, and
"(moved November 1, choir sang hymns round the hall to
November 8)" asserts November 8. **Pins:** nine new tests
(`r26b_spatial_prepositions`, `r26b_phrase_units_ring2`,
`r26b_general_adverb_class`, `r26b_do_paradigm`,
`r26b_singletons_and_ago_phrases`, `r26b_reflexives_and_nor`,
`r26b_quantifier_number_remainder`,
`r26b_determiner_adjectives`,
`r26b_claim_zone_spot_checks`) — head + trailing slots on
group surfaces, relative path, plain twins where the
lanes probed, a mirror per class. **Flagged change (one,
loud):** `r25a_bare_phrase_units_round25` — its
"recorded singletons still assert" block pinned "(ago
November 1)", "(away November 1)", and "(home November 1)"
as asserting; F7 joins those words, so the three shapes
now govern and moved to `r26b_singletons_and_ago_phrases`;
the block's other lone members (seeing / long / case /
spite / behalf) are unchanged. The mid-run failure list
named this test alone. Suite: **808 tests (799 + 9 new),
0 failed**, full unpiped `cargo test --workspace
--no-fail-fast` exit 0; clippy `-D warnings` clean.

## Shipped addendum, R26-C (2026-10-18)

All nine R26-C items shipped in `crates/ob-extract/src/
contribution.rs`, on the F5 object-continuation walk and
the parse edges, each landed as small anchored edits with
`cargo check` between items. **F2-1:** the backward walk
crosses a PP inside the continuation without resetting
the object material already crossed (one exception: a
"by" agent phrase keeps the reset, which is what holds
the pinned by-phrase alternatives); a passive verb
standing directly before the weekday PP is crossed when
its auxiliary stands before it AND a further PP
intervenes between the weekday PP and the target — both
orders of the served/sung witnesses now read nothing,
and the direct no-intervening-PP passive keeps its
pinned alternatives. **F2-2:** the walk's adverb crossing
covers the eight flat adverbs (still / really / never /
just / already / soon / also / now) as it covers -ly
forms and "ever". **F2-3:** the trailing arm walks back
over a whole trailing PP run (bare, comma-separated,
mixed bare+relative) and re-judges it in forward-twin
position; the claim-free alternatives and the
different-second-weekday reading stand. **F2-4:** the
walk carries its own crossing set
(`is_continuation_preposition`) in which "for" is crossed
when a target construction follows; `is_subject_preposition`
and the passive-for naming rule are untouched, and
"(it was moved for November 1)" still asserts. Deviation,
stated plainly: the batch brief asked for the for-witness
to "read nothing"; with the Sunday consumed, the target
asserts with an empty note — the reading of the standing
pinned consumed-PP twin ("(moved November 1, baked beans
on Sunday to November 8)", `r25b_relative_pp_object_
consulted`) and of the Sunday-free twin, both of which
assert November 8. A nothing reading would have required
the for-PP to govern the target, which no standing rule
supports; the phantom (October 18 asserted/named,
November 1 demoted) is gone, and the pin records the
twin-consistent reading. **F2-5:** "until" terminates
the forward arm's noun crossing exactly as "to" does, at
every stack depth and order (bare and relative objects,
relative targets after "until" invent no phantom), and
the forward arm crosses a conjoined clause — subject
run, auxiliary stack, verb form — to a target in the
second conjunct; the backward walk crosses a further
conjoined verb behind object material only when no
determiner-headed subject run has been crossed since the
last verb, which is what keeps a genuine second clause's
subject a boundary. Held boundaries re-pinned: the
verb-directly-before-PP alternatives; the
second-clause-after-completed-claim composition (its
Sunday asserts, naming the target); the 1-PP weekday and
relative "until" readings (target asserts, from-date
named by the until rule); the base-order 2-PP "until"
shape. **F8:** one contextual boundary classification
(`clause_break_after_at`) feeds every boundary site that
fed on the bare period (governor window, governed-after,
the claim scan's bounded reach and bound loops, the lone
framed fence): an R25-A-class abbreviation keeps its
period from acting as a boundary when it stands inside a
PP before a name word; sentence periods elsewhere still
break clauses. **M1:** a token that cleans to nothing is
crossed without judgment in the consultation's backward
and forward walks and is exempt in the claim scan's
bounded reach — U+200B / U+FEFF / "*" tokens between
"moved" and its date (standalone or attached) now read
exactly as their clean twins, on group and plain
surfaces. **M2:** an unclosed bracket span runs to the
end of the token run in the span mask, and the
line-level bracketed-restatement test counts a span open
at the date and unclosed through the end of the core;
"[November 1" alone still reads nothing. One distinction
recorded: a span that CLOSED before the date
("[our choir] November 1") binds nothing and the date
still asserts — content beside the governed sibling, not
a restatement of it. **M3:** the review's witness string
was lost with its report; ~90 re-derived shapes were
probed across the between-date and after-target
families. No probed shape matches the synthesis's stated
direction (with-object names, without-object inert): in
every discriminating family the direction is the mirror
image — object material makes the inserted clause inert,
and the without-object twin is the one whose verb stands
directly before the PP. That twin is the pinned R25-B
boundary and stands. The with-object shape — a second
clause carrying object material between the claimed date
and the claim's own target phrase — names nothing (the
target asserts, the note empty, on group, keep-through,
relative, and plain surfaces) and is pinned as it stands;
no code change was needed for it beyond the F2 walk work,
and none was invented to match the lost witness.
**Pins:** nine new tests (`r26c_passive_intervening_pp`,
`r26c_flat_adverbs_in_auxiliary_stack`,
`r26c_trailing_pp_run`, `r26c_for_pp_in_continuation`,
`r26c_until_targets_and_conjoined_clauses`,
`r26c_abbreviation_period_inside_pp`,
`r26c_cf_only_tokens_at_claim_sites`,
`r26c_unclosed_bracket_spans`,
`r26c_second_clause_object_material_names_nothing`) —
witnesses on the group, keep-through, relative, and
plain surfaces the lanes probed, with the standing
boundaries re-pinned in the same tests. **Flagged
changes to pre-existing tests: none.** The full
ob-extract library suite passed against the changed
code before the new pins were added, and the mid-run
failure list named no pre-existing test; the only
failure during the run was one of the new pins itself
(the 1-PP relative "until" shape first filed with the
reads-nothing witnesses; it carries the standing until
reading and was re-pinned in the boundary block, per
above). Suite: **817 tests (808 + 9 new), 0 failed**,
full unpiped `cargo test --workspace --no-fail-fast`
exit 0; clippy `-D warnings` clean.
