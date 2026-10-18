# Convergence audit round 25 — synthesis (2026-10-18)

Base: round-24 close `d705baa` (782 tests). Three blind read-only lanes,
reports at `~/workspace/openbulletin-audit-wave3/round25-{verify,review,redteam}.md`.

- **Verify: PASS, 0 BLOCKER / 0 MAJOR / 0 MINOR.** Every round-24 claim
  re-checked: all r24a_*/r24b_* pins pass; programmatic test-body diffs show
  zero pre-existing changes in R24-A (140→144 bodies) and exactly the one
  flagged pin in R24-B (144→148); `BARE_PHRASE_UNITS` in source is exactly
  the closed 13 and `GARNISH_NOUNS` exactly 8, word for word; fresh unpiped
  full run 782 passed / 0 failed; clippy `-D warnings` exit 0; booklet
  31,717 imposed / 34,977 reader, byte-exact; rights brute-force pins and
  loader pins all ok.
- **Review: 0 BLOCKER / 3 MAJOR / 7 MINOR.**
- **Red team: 0 BLOCKER / 4 MAJOR / 6 MINOR** (RT25-10 graded MINOR on the
  standing rights adjudication, as in rounds 21–24).

**Deduped: 0 BLOCKER / 5 MAJOR / 11 MINOR. The exit bar is not met; the
loop continues.** Merges: review R25-4 folds into the phrase-unit family
(F2); review R25-2, review R25-3 and RT25-4 are three mechanisms of one
object-continuation family (F5); review R25-8 and RT25-8 are one
token-coherence finding (M4).

Held, with counts (both lanes): the round-24 families are closed — the
round-24 oracle re-run records zero resurrections from the subordinator,
shipped-phrase-unit (26/26 slots, 130/130 split slots, 30/30 stack slots),
and boundary-inside-group families; its entire remaining residual (113 of
155,648 lines) is M2's attached-separator shape. Subject-PP relative
objects hold in all new positions (195 readings, no demotion); the fence
centre and garnish/possessive/modifier stacks hold (69/74, the 5 diffs
resolving to already-held lines, including the possessive-on-the-weekday
silence round 24 recorded for "Sunday's bulletins"); the loader stays
linear under load (8M storm, 62,500 candidates, 63 ms, truncations and
decoys refused as before); the verdict contract holds over 40,000 lines ×
6 bulletins (0 invariant violations; anchored-vs-unanchored differences
are all the designed one-week relative shift); negation as the second aux
slot word spends on all three surfaces, three-word slots never spend
(review, verified correct).

## MAJOR findings and dispositions

### F1 — The preposition / participial remainder is still outside the inventory (RT25-1)

`till`, `along`, `excepting`, `save`, `pending`, `considering`,
`assuming`, `granted`, `given`, `excluding` resurrect a governed date in
head and trailing slots, on every path and on the plain surface (10 of 10,
mirror-checked with `choir`/`service` so the class does not eat a real
event in the sample). These are ordinary office words: `till` is the
everyday synonym of the inventoried `until`; `along` is a core preposition
whose neighbours are all in; `pending` is office vocabulary; `given` /
`considering` / `assuming` / `granted` are the participial subordinators.
**Disposition: FIX (R25-A).** The ten words join the inventory classes
(prepositions; participial prepositions/subordinators). Held neighbours
bound the class: `despite`, `since`, `via`, `per`, `except`, `regarding`,
`concerning`, `following`, `between`, `within`, `without`, `upon`,
`onto`, `toward(s)`, `against`, `across`, `behind`, `beyond`, `plus`,
`minus`, `versus`, `barring` and the connective set resurrect nowhere.
The remaining singleton fragments from the same sweep (`ago`, `out`,
`up`, `down`, `away`, `home` as sole companions) are recorded, not added:
they are not plausible restatements alone, and the phrases that matter
are covered by F2's units; `anyhow` joins the connective adverbs with
its twin `anyway`.

### F2 — Complex prepositions and phrase-unit extensions resurrect at scale (RT25-2 + review R25-4)

46 of 65 candidate extension units resurrect in both slots on all
readings, plain twins resurrecting, and three of the families are top-8
drivers of the widened 266,624-line composition fuzz (4,543
resurrections; drivers at 582–599 lines each, against the 116–141
ambient baseline of every round-21–24 family). The office core: `in
spite of`, `as long as`, `on behalf of`, `in lieu of`, `with regard
to`, `in front of`, `in back of`, `on top of`, `by way of`, `for the
sake of`, `in light of`, `in view of`, `in regard to`, `ahead of`, `out
of`, `up to`, `in case`, `in time`, `on time`; the idiom tail: `at
last`, `at long last`, `together at long last`, `back together at
last`, `back home`, `gone home`, `at home`, `long ago`, `sooner or
later`, `back and forth`, `to and fro`, `in and out`, `up and down`,
`one by one`, `day by day`, `first and last`, and kin; subordinator
phrases: `as far as`, `given that`, `seeing that`, `save that`.
Review R25-4 is the same family from the overlap side: "back together
at last" is two shipped units sharing the word `together`, no disjoint
partition exists, and it resurrects. What held inside the family is as
informative: the shipped 13 units, their splits (crossed by the
empty-token rule, as R24-2's comment promises), and their stacks
resurrect nowhere; the break needs a genuinely new phrase.
**Disposition: FIX (R25-A).** The resurrecting units enumerated in the
red team's sweep join `BARE_PHRASE_UNITS` (the builder takes the exact
membership from `bare25`/`bare25b` output, mirror-checked); longest-first
matching already in place partitions "back together at last" whole.
Members standing alone keep their readings, as R24-2 required.

### F3 — Pronouns and modals, graded: they resurrect identically, and the class is MAJOR (RT25-3)

The sets round 24 recorded ungraded were graded under the corrected
claim, as the round-25 brief directed: pronouns 48/48 slot checks
resurrect, modals 24/24, for a second consecutive round, and `we` /
`will` are top-8 drivers of the widened fuzz (595 / 594 lines). The
check-carefully members bound the class honestly: `it`, `this`, `that`,
`now` do NOT resurrect (already in the inventory); the additions sweep
resurrects only `i`, `who`, `whom`, `whomever`, `one`, `others` beyond
the recorded pronoun set. The parity with R24-1 is direct: `(when
November 1)` was a fragment too, graded MAJOR and fixed by inventory
addition because "when" is ordinary vocabulary standing as a
restatement's only companion word; `we`, `they`, `will`, `would`, `you`
meet the same test in the same slot. The R24-A pins asserting three of
these shapes were deferral pins ("the ungraded sets stay untouched"),
not an adjudication that the class is correct; grading has now happened
and the class does not survive it. Pronouns and modals as bare words
matter only inside an occurrence region beside a governed date; a
pronoun with its verb ("we sang") still carries the verb as content,
so real events still assert — the lane's mirror checks confirm.
**Disposition: FIX (R25-A).** The pronoun class additions (`i`, `we`,
`they`, `you`, `who`, `whom`, `whomever`, `one`, `others`, and the
recorded set's remaining members as probed) and the modal class
(`will`, `would`, and the recorded modal set as probed) join the
inventory, membership taken from the lane's probe output and
mirror-checked. The R24-A deferral pins flip; flagged loudly per
protocol.

### F4 — The fence's verb-form shape bypasses the R24-3 head test (review R25-1)

"Sunday baked / printed / left / taken … rehearsal, November 8" asserts
Nov 8 (also stacked: "Sunday baked beans rehearsal", possessive: "our
Sunday baked beans rehearsal") while the control "Sunday morning
rehearsal, November 8" reads nothing. `lone_framed_fence`'s verb-form
branch passes the phrase by shape and never computes its final noun;
the R24-3 head test runs only in the bare-shape branch, so the pinned
two-word shape became a prefix exempting an arbitrary stacked tail.
**Disposition: FIX (R25-A).** The fence judges the phrase's HEAD — its
final noun — against `GARNISH_NOUNS` whatever the phrase's shape;
R24-3's head rule is the fence's law and the R23-6 shape pass, written
before the head rule existed, is superseded where the two conflict.
Consequence, flagged: the R23-6 pin "Sunday baked beans" (head `beans`,
not garnish) goes silent under the composed rule. The distinction the
old pair of rules produced — an event head fences when its modifier is
"morning" but not when it is "baked" — is indefensible, and silence is
this machinery's safe direction. The builder flags the pin change by
name with this reason.

### F5 — Object-continuation consumption: three escapes (RT25-4 + review R25-2 + review R25-3)

One family, `weekday_in_object_continuation_pp` and its consultation,
three mechanisms:

1. **The claim's own full clause exempts it (RT25-4).** R24-5's
   boundary — no consumption when a subject and finite verb stand
   between — cannot distinguish a second clause from the claim's own
   clause, so a claim written as a full sentence escapes: "(moved
   November 1, the choir sang hymns from Kent on Sunday to November
   8)" names phantom Oct 18 beside Nov 8, and in weekday-first order
   "(… sang baked beans on Sunday from Kent to November 8)" asserts
   Oct 18 outright on every path, Nov 8 demoted. Relative PP objects
   in the same continuation behave the same way (Oct 25 phantom).
2. **Relative-phrase PP objects are never consulted (review R25-2).**
   "(moved November 1, and baked beans from Kent on next Sunday to
   November 8)" flips the bare twin's from-governed silence to
   asserting Nov 8 and naming Oct 25, on group, plain, and relative
   surfaces; consumption is only consulted for bare-weekday signals.
   The forward arm also cannot read a relative target after
   `to`/`until`: "(… on Sunday to next Sunday)" asserts Oct 25 naming
   phantom Oct 18.
3. **A trailing weekday PP asserts a phantom (review R25-3).** "(moved
   November 1, baked beans from Kent to November 8 on Sunday)" asserts
   Oct 18 — a date written nowhere — and demotes the framed Nov 8 to
   the note, where the twin without the trailing PP reads nothing.
   The R24-B "not consumed" pin covered only the no-claim shape,
   which merely offers alternatives and stands.

**Disposition: FIX (R25-B).** Consumption reaches weekday and relative
PP objects inside a target claim's object continuation whether or not
the claim is written as a full clause — the subject+finite-verb
boundary may exempt only a genuinely second clause (one standing
outside the claim's own clause, as the R24-B pinned boundary requires:
a weekday PP in a second clause with its own subject and finite verb
keeps its signal, and one with no target phrase behind it in a
claim-free line still offers alternatives). A trailing weekday PP with
no target phrase behind it, on a line carrying a claim construction,
is consumed with the construction. Every witness above then reads as
its subjectless / bare / no-trailing-PP twin reads today.

## MINOR findings and dispositions

- **M1 — Interjections / address resurrect identically (RT25-5).**
  28/28 slots for a second round (`oh/ah/oops/wow/hey/hi/hello/dear/
  yeah/ok/okay/amen/friends/folks` plus `bye/welcome/sorry/greetings/
  congratulations`). Graded MINOR by the lane (discourse words;
  greeting fragments, not plausible annotations). **FIX (R25-A)**
  anyway: the class is the same kind as the shipped discourse garnish
  (`please/yes/again/ps`) and it is one more list in the same batch.
- **M2 — Boundary characters attached to a date token defeat the token
  rule (RT25-6).** The round-24 oracle's entire residual (113 of
  155,648) is one shape: `*November 1, 2026, cancelled (November 1)`,
  `🎉November…`, `•November…`, `November 1, 2026*` assert — the fused
  token never parses as a date, so its clause's governance is never
  judged. An asterisk or bullet fused to a date is ordinary footnote
  garnish in office copy. **FIX (R25-B):** the date-prefix parse trims
  boundary garnish at a token's head and tail as it already trims
  `;`/`:`/brackets, so the date reads and governance judges it.
- **M3 — Bracketed restatements resurrect beside a governed group
  (RT25-7).** `(November 1, cancelled) [November 1]` asserts on
  grp+keep; alone they stay unread and the plain surface holds.
  **FIX (R25-B):** governance binds a bracketed bare restatement in
  group composition exactly as it binds an unbracketed one (the plain
  composition already does this); the recorded alone-unread
  limitation stands.
- **M4 — Token coherence (review R25-8 + RT25-8).** Three seams of one
  rule: (i) the implementation's boundary test checks letters only,
  while the stated R24-4 rule is letters **or digits** — `#1` and
  keycap tokens clean to empty (phantom boundaries), and `1st` cleans
  to empty and takes its date's span with it; (ii) singleton
  alphanumeric tokens (`(November 1 1)`, `(x November 1)`) are
  content by construction and resurrect; (iii) multi-token bracket
  spans (`[our choir]`) are no site's bracketed token, inconsistent
  with `[choir]`'s garnish cleaning. **FIX (split A/B):** the boundary
  test aligns to the stated rule at every site (letters or digits =
  content-bearing, never a boundary); in the bare partition a cleaned
  single-character token is garnish, and a bracket span cleans to
  garnish as a wholly bracketed token does. Under the composed rule
  `#1`/keycap tokens are no longer phantom boundaries and no longer
  leave phantom-empty regions; `(November 1 1st)` asserts its date,
  as a mistyped "November 1st" should.
- **M5 — "St." / "Mt." abbreviations resurrect bare restatements
  (review R25-9).** `(November 1, cancelled) (St November 1)` and the
  trailing twin assert on every surface; `St.` cleans to `st`, a
  content word. In a church office "St." before a name is peak
  ordinary vocabulary. **FIX (R25-A):** a small closed abbreviation
  class joins the bare inventory by cleaned form (`st`, `mt`, and the
  common title set — the builder fixes membership, church-office
  shaped, mirror-checked).
- **M6 — A zero-width character glued inside a governor word kills the
  governor (review R25-10).** "cancelled" carrying U+200B reads as no
  governor anywhere, while stars/bullets/emoji around it are handled.
  Clipboard text carries these invisibly. **FIX (R25-A):** token
  cleaning strips Unicode format characters (general category Cf)
  everywhere tokens are cleaned — the one cleaning rule, extended by
  character class, not by word.
- **M7 — A phrase unit is matched across the date span itself
  (review R25-5).** `(gone November 1 tomorrow)` and `(care November
  1 of)` read nothing: `occurrence_is_bare` concatenates the
  before-date and after-date words before unit matching, so split
  members become consecutive although the date stands between them;
  the passive-predicate test in the same function is already barred
  from spanning. **FIX (R25-A):** unit matching judges the before-date
  and after-date word lists as separate sequences, the same no-span
  rule.
- **M8 — The claim scan's own passive keeps the pre-R24-6 slot rule
  (review R25-6).** After R24-6 the asymmetry inverted: passive-for
  and detection spend an unlisted adverb, the claim scan's `to`
  surface does not — "(moved November 1, it was softly moved to
  November 8)" reads nothing while its twins assert. **FIX (R25-B):**
  the claim scan's passive complement uses the shared slot walk, as
  detection and passive-for already do.
- **M9 — Slot intensifiers spend nowhere; phrasal slots recorded
  (RT25-9).** `so gently` / `too loudly` spend on neither scan
  (passive-for and detection agree in silence — no asymmetry to
  exploit, a lost occasion only), and `at all` / `at once` / `in all`
  likewise; "it was not at all moved" is a three-word slot under the
  pinned two-word cap. **FIX in part (R25-B):** `so`/`too` join the
  slot vocabulary as intensifiers in both walks, keeping the scans
  aligned. **Recorded, not fixed:** the phrasal adverbials
  (`at all`, `at once`, `in all`) would need unit slots, and
  `not at all` exceeds the pinned cap; the cap stands.
- **M10 — The "-ly" noun seam (review R25-7).** Confirmed in all slot
  positions ("the family was loudly moved for November 1" spends its
  unlisted adverb the same way), behaving exactly as detection does —
  the behaviour R24-6 accepted and the round-24 close recorded.
  **ACCEPTED as a documented standing limitation** (already on record
  from round 24); no fix this round.
- **M11 — Rights permutation flips, sixth consecutive round
  (RT25-10).** Outputs byte-identical to rounds 23 and 24: 80,412
  route comparisons, 0 safety-pair flips; 19/3,000 permutation flips,
  every one license-only in effect (no sequence notes children as
  present; 127,128 children-bearing checks clean). **Standing
  adjudication, restated, no fix** — the license-only residual
  measured since round 11; FORMAT.md already states the agreement as
  measured, not proved.

## Recorded without grade

- The fence's garnish set remainder: twelve unlisted printed-matter
  candidates (`programs`, `program`, `handouts`, `handout`,
  `booklets`, `booklet`, `folders`, `inserts`, `pages`, `orders`,
  `guides`, `prints`) all read nothing beside a framed date — the set
  is closed as shipped and the failure direction is silence. Recorded
  as round 24 recorded the set's other exclusions; a future round or
  Jacob's word can widen it.
- `Sunday's`-family fence silence extended to stacks (fence25's 5
  diffs) resolves against the already-held line; not a break.

## Fix batches

- **R25-A — inventory, bare partition, cleaning, fence.** F1 (ten
  words + `anyhow`), F2 (phrase-unit membership from the lane's
  sweeps), F3 (pronoun + modal classes), M1 (interjection/address
  class), M5 (abbreviation class), M6 (Cf stripping in token
  cleaning), M7 (no-span unit matching), F4 (fence head test for every
  phrase shape), M4 part 1 (single-character garnish + bracket spans
  in the bare partition). Expected flagged pins: the R24-A deferral
  pins for the ungraded sets (F3) and the R23-6 "Sunday baked beans"
  fence pin (F4); any further changed pre-existing test is flagged by
  name with its reason, loudly.
- **R25-B — consumption, claim scan, parse edges.** F5 (all three
  mechanisms), M8 (shared slot walk in the claim scan), M9 (`so`/`too`
  slot intensifiers), M2 (boundary garnish at date-token edges), M3
  (bracketed restatement governance in group composition), M4 part 2
  (boundary test letters-or-digits at every site). The R24-B pinned
  boundaries stand: a weekday PP in a genuinely second clause keeps
  its signal; a claim-free line's trailing PP still offers
  alternatives; subjectless twins keep their readings.

## Shipped addendum, R25-A (2026-10-18)

Shipped over `dea3c7a` in `crates/ob-extract/src/contribution.rs`
only. Full unpiped workspace run: **790 passed / 0 failed** (782 at
the round-25 base + 8 new `r25a_*` tests), cargo exit 0; clippy
`--workspace --all-targets -- -D warnings` exit 0.

- **F1** — `till`, `along`, `excepting`, `save`, `pending`,
  `excluding`, `considering`, `assuming`, `granted`, `given` join
  `bare_filler_word`; `anyhow` joins the connective adverbs.
  Mirror-checked per word (content label still asserts).
- **F2** — `BARE_PHRASE_UNITS` 13 → 63: the sweep's resurrecting
  set exactly — office core 23, idiom tail 23, subordinator phrases
  4 (`as far as`, `given that`, `seeing that`, `save that`) — plus
  the shipped 13. Longest-first matching partitions "back together
  at last" whole. Members standing alone keep their readings
  (pinned: `seeing`, `long`, `case`, `spite`, `behalf`, and the
  recorded-not-added singletons `ago` / `away` / `home` still
  assert).
- **F3** — pronoun class (30: the recorded 24 plus `i`, `who`,
  `whom`, `whomever`, `one`, `others`) and modal class (12: `will`,
  `would`, `can`, `could`, `may`, `might`, `must`, `shall`,
  `should`, `ought`, `dare`, `need`) join the inventory.
  Mirror-checked: "(we sang November 1)" / "(they sang
  November 1)" still assert beside a sibling cancellation.
- **M1** — interjection / address class (19 words as listed in
  RT25-5 plus its tail) joins the discourse garnish.
- **M5** — abbreviation class by cleaned form: `st`, `mt`, `mr`,
  `mrs`, `ms`, `dr`, `rev`, `fr`, `br`, `sr`, `jr`. Witnesses
  "(November 1, cancelled) (St November 1)" and "(November 1
  St.)" read nothing on every surface.
- **M6** — token cleaning strips Unicode format characters
  (category Cf, by range table) at the cleaning sites in this
  file: `group_token`, `weekday_word`, and the date-prefix
  normalization. "cancelled" carrying U+200B / U+200D / U+FEFF /
  U+00AD governs again, group and plain; a Cf glued inside a
  date token no longer poisons its parse.
- **M7** — `occurrence_is_bare`'s phrase-unit matching may not
  span the date: a unit whose non-empty members stand on both
  sides of the date span is not consumed as a unit.
  "(gone November 1 tomorrow)" and "(care November 1 of)"
  assert November 1 again on all surfaces.
- **F4** — `lone_framed_fence`'s verb-form branch now computes
  the phrase's head (final noun of the noun run) and judges it
  against `GARNISH_NOUNS`, as the bare-shape branch does.
  Garnish-headed verb-form phrases still assert ("Sunday baked
  bulletins", "Sunday printed sheets", "Sunday baked beans
  bulletins").
- **M4 part 1** — in the bare partition, a cleaned
  single-character token is garnish, and a multi-token bracket
  span is garnish via a new span mask over the raw tokens
  (`occurrence_is_bare` now takes the raw tokens; its three
  call sites pass them). "[choir]" single-token behaviour
  unchanged; unbracketed "choir" is still content.

**Flagged pre-existing test changes (3 tests, all intended):**

1. `r24a_bare_inventory_subordinator_class` — its closing block
   pinned "(he November 1)", "(will November 1)", "(oh
   November 1)" ASSERTING as "the ungraded sets stay untouched".
   Those were R24-A deferral pins, not adjudications; round 25
   graded the sets (F3 MAJOR, M1 MINOR) and they joined the
   inventory, so the block now pins the same shapes governed
   (None), with the reason in the test comment.
2. `r24a_fence_judges_the_phrase_head` — "Sunday left luggage,
   November 8, 2026" moved from the asserting list to the
   silent list. F4: the verb-form phrase is judged by its head
   ("luggage", not garnish), superseding the R23-6 shape pass it
   was pinned under.
3. `r23b_weekday_verb_form_modifier_phrase` — the eight
   verb-form lines (heads luggage / hymns / seats / choir /
   beans, none garnish) pinned the framed date asserting under
   the R23-6 shape pass; they now read nothing and name
   nothing. This includes the synthesis's named consequence,
   the "Sunday baked beans" pin (head "beans"). The test's
   R23-6 core claim — no phantom October 18 asserted, offered,
   or named — stands and is still pinned; the group-twin claim
   shape in the same test is untouched and stands.

No other pre-existing test changed. The inventory comment's
claim stands as written: the classes are the rule, the lists
are their current membership; the `BARE_PHRASE_UNITS` comment
now records the 63-unit membership by family.

## Shipped addendum, R25-B (2026-10-18)

Shipped over `9c39abf` in `crates/ob-extract/src/contribution.rs`
only. Full unpiped workspace run: **798 passed / 0 failed** (790 at
the R25-A base + 8 new `r25b_*` tests), cargo exit 0; clippy
`--workspace --all-targets -- -D warnings` exit 0.

- **F5a** — the object-continuation backward walk
  (`weekday_in_object_continuation_pp`) now crosses the claim's
  OWN clause verb, and its auxiliaries, when object material
  stands behind them, then crosses the verb's subject by the
  standing branches. The second-clause boundary is discriminated
  by position: a finite verb standing directly before the PP
  (no object material crossed behind it) still ends the walk.
  Every full-clause witness ("the choir sang baked beans from
  Kent on Sunday to November 8", both orders, hymns/beans,
  conjunction or not, plain twins) reads exactly as its
  subjectless twin; the pinned second-clause readings
  ("beans were served on Sunday", "choir sang on Sunday",
  claim-free) keep both dates as alternatives, unchanged.
- **F5b** — the relative-signal sites (group scan and plain
  scan) consult the same check at the phrase's weekday word;
  the check's preposition detection admits a relative object
  ("on next Sunday", "before this Sunday", "on the next
  Sunday"), and the forward arm accepts a relative phrase as
  the claim's target after "to"/"until". "(moved November 1,
  baked beans from Kent on Sunday to next Sunday)" reads
  nothing and names nothing — pinned from the machinery's own
  twin behaviour: its Sunday-free "from Kent" twin reads the
  same, the "until" twin pair keeps its standing reading
  (October 25 asserting, November 1 named). A relative object
  deeper in the stack ("on Sunday from next Sunday to
  November 8") is consumed by the same consultation.
- **F5c** — the trailing arm (`weekday_pp_after_claimed_target`):
  when the backward walk dies at a claimed date's "to", a
  weekday PP standing after that date is re-judged in its
  forward-twin position (end of the continuation, just before
  the "to") on a rebuilt token run. "(moved November 1, baked
  beans from Kent to November 8 on Sunday)" reads as its twin
  without the trailing PP; the claim-free "(baked beans to
  November 8 on Sunday)" keeps both dates as alternatives,
  as pinned. The same arm consumes a trailing relative PP.
- **M8** — the claim scan's zone extension (`target_claim`)
  also runs the shared slot walk (`passive_for_slot_walk`) and
  prefers its reach when that reach starts nearer the
  auxiliary, while the bounded walk's reaches still win where
  they reach at least as near. "(moved November 1, it was
  softly moved to November 8)" (and loudly / really-softly)
  asserts November 8 naming November 1, like its passive-for
  and detection twins; the pinned three-word death
  ("not really ever") holds — both walks refuse it.
- **M9** — "so" and "too" join the slot vocabulary in both
  slot walks (`aux_complement` and `passive_for_slot_walk`):
  "(it was so gently moved for November 1)" and "(it was too
  loudly moved for November 1)" assert November 1, detection
  twins agree. The two-word cap stands; the phrasal adverbials
  "at all" / "at once" / "in all" are recorded, not fixed —
  pinned as still reading nothing.
- **M2** — `parse_date_prefix`'s edge trim is by character
  class: any character that is not a letter or a digit (the
  garnish class of `token_has_no_letters_or_digits`) trims at
  a token's head and tail. The class stops at the two
  parentheses: they are the system's structural group
  delimiters, not garnish, and the raw-text scans rely on the
  parse failing at a group edge. (An interim full-class trim
  failed five pre-existing tests — the leading-group and
  relative-memory compositions `r10b_paren_only_for_sunday_
  parity_and_a_mid_date_group`, `r21a_leading_groups_compose`,
  `r22a_separator_after_a_leading_group_keeps_relative_memory`,
  `r23a_one_boundary_classification`, `r24a_boundary_token_rule`
  — and the class was refined before shipping; all five pass
  unchanged in the shipped tree.) The witnesses "*November 1",
  "🎉November 1", "•November 1", "2026*" now read exactly as
  their clean twins: governed, nothing read.
- **M3** — the whole-line composition's Assert arm binds a
  bracketed bare restatement: when a group on the line governs
  the core's asserted value directly and the asserting
  occurrence is bare and bracket-wrapped (glued, spaced, or
  doubled, square or brace), the line reads nothing
  (`core_bracketed_bare_restatement`). "(November 1,
  cancelled) [November 1]" and its dressed forms read nothing
  on grp+keep; the alone-unread limitation stands; the R24-4
  pin "[November 1, cancelled] [November 8]" still reads
  November 8 naming November 1.
- **M4 part 2** — the alignment landed at the parse site the
  review named: because "#1" and the keycap digit carry a
  digit, the M2 class trim lets a date run reach across them
  exactly as across "1", so they no longer act as phantom
  clause boundaries anywhere `parse_date_prefix` feeds
  (separator, restatement-resurrection, and in-group shapes
  all read as their digit twins; "#" and "*" remain true
  boundaries). "(November 1 1st)" asserts November 1. The
  remaining boundary sites were audited and already judge by
  the letters-or-digits rule (`token_has_no_letters_or_digits`,
  `group_token`, `clause_break_after`, the boundary-token
  strippers, and the R25-A bare partition), and compose with
  R25-A's single-character garnish unchanged. Unpinned probe
  shapes beside the witnesses moved consistently with the
  rule (a group date after a digit separator is named rather
  than silently dropped; a trailing digit token no longer
  blocks a relative+group composition) — twin-consistent in
  every case, and no pre-existing test covered them.

**Flagged pre-existing test changes: none.** No pre-existing
test was edited, moved, or re-pinned. The five interim
failures listed under M2 were resolved by refining the fix,
not the tests. No R24-B/R24-5 boundary pin's witness turned
out to be the full-clause shape in disguise: the pinned
second-clause witnesses have their verb standing directly
before the PP, which is exactly the discriminating structure
F5a ships.

## Coordinator close, round 25 (2026-10-18)

Both fix batches shipped and were independently re-verified by the
coordinator (full unpiped suite + clippy `-D warnings` on the exact
committed trees, counts read from cargo's own output):

- **R25-A `9c39abf`** — 782 → 790 tests. F1's ten words + `anyhow`, F2's
  50 phrase units (`BARE_PHRASE_UNITS` counted in source at exactly 63,
  `GARNISH_NOUNS` still 8), F3's pronoun (30) and modal (12) classes,
  M1's interjection/address class, M5's abbreviation class, M6's Cf
  stripping at the cleaning sites, M7's no-span unit matching, F4's
  fence head test on the verb-form branch, M4 part 1 (single-character
  and bracket-span garnish in the bare partition). Three flagged
  pre-existing test changes, all pre-authorized by this synthesis: the
  R24-A deferral pins for the ungraded sets, "Sunday left luggage"
  under the F4 head rule, and the r23b verb-form test (its eight lines,
  including the expected "Sunday baked beans", now silent; its core
  no-phantom claim still pinned).
- **R25-B `8f287ab`** — 790 → 798 tests, **zero changed pre-existing
  tests**. F5's three mechanisms (full-clause consumption via the
  object-material discriminator — a finite verb directly before the PP
  still ends the walk, so the pinned second-clause readings stand;
  relative PP objects consulted on both scans, relative targets read
  by the forward arm; the trailing-PP arm on claim lines, claim-free
  alternatives pin untouched), M8's shared slot walk in the claim
  scan, M9's so/too intensifiers (cap untouched), M2's date-token
  garnish class (stops at the two parens, the system's structural
  delimiters — refined in-batch after five pre-existing tests showed
  the full class broke group-edge parsing; the tests were not moved),
  M3's bracketed-restatement binding, M4 part 2's digit-token boundary
  alignment at the parse site.

Round totals: 782 → **798 tests, all passing, clippy clean** at the
close tree. Every MAJOR and every fix-graded MINOR in this synthesis
is dispositioned and shipped; M10 (the "-ly" seam) stands as a
documented limitation, M11 (the rights license-only residual) under
its standing adjudication, M9's phrasal remainder (`at all`, `at
once`, `in all`) recorded under the pinned slot cap.

Carried to round 26, recorded: the singleton fragments (`ago`, `out`,
`up`, `down`, `away`, `home` as sole companions — phrases covering
them shipped in F2); the garnish set's twelve unlisted printed-matter
candidates (silence direction); the F5 discriminator's edge (a finite
verb directly before the PP ends the walk — the round-26 lanes should
probe fuller shapes around it); bracket spans in composition beyond
the restatement binding. The inventory-remainder pattern is now three
rounds old: each round closed the class it aimed at and left the
adjacent remainder standing. Round 26's briefs direct the lanes to
sweep the next ring deliberately — candidate classes beyond the ones
fixed here — rather than re-probing only the shipped lists.
