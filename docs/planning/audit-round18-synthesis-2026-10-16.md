# Round 18 audit synthesis — 2026-10-17

HEAD audited: `485530e` (round-17 close). Lanes: verify `8cffc3b0`
(report `~/workspace/openbulletin-audit-wave3/round18-verify.md`),
review `055c72e7`, red team `f100f218`. Verify: **PASS, 6 of 6
claims**, CI for the close push green on all three OS jobs (run
39084286101), bundle 39084290647 SUCCESS, booklet bytes exact,
Bach live passing. Review **0 BLOCKER / 2 MAJOR / 6 MINOR**; red
team **0 BLOCKER / 3 MAJOR / 4 MINOR**. **The exit bar is not
met.** Deduped: **0 BLOCKER / 5 MAJOR / 7 MINOR**.

## Findings and dispositions

**R18-1 (MAJOR; review).** Tokenization asymmetry: the plain-line
cleaner does not strip ":" while the group tokenizer does, so
"cancelled: November 1, 2026, next Sunday" ARMS the cancelled
November 1 (the colon-free twin correctly asserts October 25).
Disposition: one token-cleaning behavior — the plain path strips
":" exactly as the group path does.

**R18-2 (MAJOR; RT18-2).** Governance attaches to occurrences,
not dates: a cancelled date restated anywhere on the line
resurrects and asserts. "November 1, 2026, cancelled, November 1,
2026" asserts November 1; parsed Oct 23, "November 1, 2026,
cancelled, next Sunday" asserts it too (the relative restates the
cancelled date). Disposition: within a line/group, a date value
governed in ANY occurrence does not assert from another
occurrence, unless a target claim names it as the (last) target.
Restated UNGOVERNED signals keep reading plainly (R13-C held).

**R18-3 (MAJOR; RT18-1).** Last-target ranking ignores relative
targets on plain lines: "November 1, 2026, moved to next Sunday,
then to November 15, 2026" files for the superseded October 25
with November 15 merely noted; the framed and group twins assert
November 15. Disposition: target ranking is by written order
across framed and relative targets alike, both surfaces.

**R18-4 (MAJOR; review).** An intervening date inside the claim
reach collapses the claim: "(moved the meeting, November 1, 2026,
and the dinner to November 8)" reads both dates as alternatives
and asserts nothing, where its comma-after-date twin asserts
November 8 with November 1 named. Disposition: an intervening
date in the reach is a from-date (governed + named), not a
reach-breaker.

**R18-5 (MAJOR; RT18-3).** The passive-"for" occasion collapses
when any second date stands in the group: "(it was scheduled for
November 1 and November 8)" and "(it was moved for November 1,
use November 8)" assert nothing, both dates merely noted.
Disposition: the for-date enters the line's composition as a
signal like any other — two for-dates read by primacy with the
other noted; a prospective second date ("use November 8")
asserts with the for-date named.

**R18-6 (MINOR; review).** Groups carry no relative signals:
"(November 1, cancelled, next Sunday)" as a group reads nothing,
while the plain line asserts October 25. Disposition: groups
resolve relative phrases at the anchor and judge them as signals
in written order alongside framed ones (one signal set
everywhere). Governors inside the group judge them as they judge
framed signals.

**R18-7 (MINOR; review).** Salvaged plain lines never name the
governed relative ("cancelled, next Sunday, November 1, 2026"
asserts Nov 1, Oct 25 unnamed). Disposition: governed relative
signals are named on the note by their resolved date, as framed
ones are.

**R18-8 (MINOR; review R18-5).** The claim reach crosses a real
second clause: "(moved November 1, choir sang to November 8)"
claims Nov 8 across "choir sang". The date-span exemption covers
commas inside the span, but a new clause after the span must end
the reach. Disposition: after an intervening date's span, the
reach continues only into the target construction itself ("to",
"then to", "and to"); other clause content ends it.

**R18-9 (MINOR; RT18-4).** Active "for" never names the occasion:
"(we moved it for November 1)" is silent. Disposition: a
displacement verb + "for" + date names the date in the active
voice too, as the passive rule does.

**R18-10 (MINOR; RT18-5/6 + review R18-8).** Adverb tolerance
seams: two stacked tolerated adverbs break it ("(it was just
recently moved, November 1)" silences while the has-been twin
spends), and the closed list omits then, briefly, shortly,
formally, unanimously, happily. Disposition: tolerance is up to
TWO consecutive listed adverbs; the list gains then, briefly,
shortly, formally, unanimously, happily.

**R18-11 (MINOR; RT18-7).** Chains split across two groups assert
correctly but drop the superseded target unnamed (the single-
group twin names it). Disposition: superseded targets are named
on the line's note across group boundaries.

## Batches

- **R18-A** (composition core): R18-1, R18-2, R18-3, R18-6, R18-7.
- **R18-B** (claim/for/adverb edges): R18-4, R18-5, R18-8, R18-9,
  R18-10, R18-11.

Round 19 audits the fixes. The Tier-2 final gate runs only after
a round returns zero BLOCKER + zero MAJOR.

## Shipped addendum, R18-A — 2026-10-17

Batch R18-A (composition core) shipped in
`crates/ob-extract/src/contribution.rs`: R18-1, R18-2, R18-3,
R18-6, R18-7. Five new pin tests, all `r18a_*`, one per finding.
**Tests: 716 → 721 passed, 0 failed** (full workspace suite run
unpiped on the final code, cargo exit 0; ob-extract 203 → 208).
**Clippy: clean** (`--workspace --all-targets -- -D warnings`,
exit 0, zero warnings). Neither the fake-helper ETXTBSY flake
nor timing contention fired in any run. Booklet bytes 31,717 /
34,977 exact, held by the full suite. Held lines stand:
R13-C first-signal primacy on governor-free lines (all
r13c/r14b/r15b pins unmoved), restated-plain readings, the
target-claim override, the defensible-silence family, the
anchor family (28-day bound, same-week identity), and
"(November 1, cancelled, November 8)" asserting November 8 with
November 1 named.

**Changed pre-existing tests: one, flagged loudly.**
`r12a_a_governor_scopes_to_the_one_date_it_governs` pinned
"(not November 1, November 1)" as reading November 1 by its
second standing (round 12, R12-2: a date restated ungoverned
after its governed first standing asserts). That is exactly
R18-2's witness shape — a governed date restated — so the pin
moves under R18-2, and only there: the group now contributes
nothing, the line stands on its relative words alone, and the
block carries a "PIN MOVED, round 18 (R18-2)" comment stating
the reason. No other pre-existing test changed. (Two further
readings changed in passing and are pinned as NEW r18a
expectations, not moved pins: nothing previously pinned a
relative phrase inside a group, and nothing previously pinned
a salvaged line's governed relative.)

What shipped, per finding:

- **R18-1 — one token cleaning.** The plain path cleans its
  tokens with `group_token`, exactly as the group path does;
  the second-date scan and the Sunday-after run reader clean
  the same way. Audit of the other punctuation the two
  tokenizers treated differently: ";" was already shared;
  "-" and the en/em dashes were group-only and are now shared
  with it (a standalone dash token now cleans to empty on the
  plain surface, as in groups). The weekday-framing readers
  keep their borrowed `&str` trims — they compare only
  weekday / "on" / "the" identity, and no witness was found
  where their narrower set changes a governor or date
  reading. "cancelled: November 1, 2026, next Sunday" now
  asserts October 25 with November 1 named, like its
  colon-free twin, on For Sunday and Keep-through.
- **R18-2 — governance by date value.** Both compositions
  (`analyze_group`, `plain_composition`) now collect the
  per-occurrence judgments first (direct governance apart
  from the claim machinery), then apply the value rule: a
  date governed in ANY occurrence does not assert from
  another occurrence, unless a target claim names it as the
  (last) target. Judgment calls, stated plainly: (1) The
  override shape the rule gives is "(meeting November 1,
  cancelled, moved to November 1)" — the first standing is
  governed by "cancelled", the second is claimed as the
  last target, and November 1 asserts by the claim, with
  nothing named (the governed standing and the asserted
  date are one date). "(November 1, moved to November 1)"
  asserts the same way. (2) Direct governance — a governor
  before or after the occurrence, the split phrasal wrap,
  a contradicting weekday — is what enters the governed
  set; superseded and from-date claim demotions do not, so
  a return-to-earlier chain's last target is untouched by
  this rule and still asserts. Witnesses: "November 1,
  2026, cancelled, November 1, 2026" and "(November 1,
  cancelled, November 1)" read nothing; at today October
  23, "November 1, 2026, cancelled, next Sunday" reads
  nothing (the relative restates the governed date), as do
  the framed/relative orders both ways ("October 25, 2026,
  cancelled, next Sunday"; "next Sunday, cancelled,
  October 25, 2026") and the anchored copy. Restated
  UNGOVERNED signals are untouched — the governed set is
  empty there by construction.
- **R18-3 — target ranking by written order.** In
  `target_claim`, a word standing inside an intervening
  signal's own span is exempt from the weekday test in the
  between-words check: a framed target's words already
  passed that test, but a relative target's "Sunday" is a
  weekday word, so every later claim in the chain died and
  the superseded relative asserted. Governor, cancellation,
  and auxiliary words still end the reach wherever they
  stand. "November 1, 2026, moved to next Sunday, then to
  November 15, 2026" asserts November 15 with October 25
  (the relative target, resolved) and November 1 named —
  matching its framed twin and, after R18-6, its group
  twin. The postponed form (relative target standing
  after the phrase with no "to" of its own) ranks the
  same way, through the framed target's claim.
- **R18-6 — groups gain relative signals.** `analyze_group`
  takes the parse anchor, and its scan is the plain line's
  signal scan (Sunday-after runs, "next/this Sunday", a
  framed date, a bare "Sunday"), judged by the same
  machinery — governors, claims, R18-2 — in written order.
  "(November 1, cancelled, next Sunday)" contributes
  October 25 with November 1 named on the relative,
  paren-only, and Keep-through paths; Keep-through shares
  the reading. Judgment calls, stated plainly: (1) The
  anchor rule mirrors plain lines exactly — only a truly
  mixed group re-resolves its relatives at the anchor; a
  relative-only group keeps its `today` resolutions, as a
  relative-only plain line does. A group standing on an
  undated (relative) line gets the parse anchor the same
  way, no separate anchor is invented. Anchored witness:
  at today October 23 with file date October 16,
  "(November 1, cancelled, next Sunday)" asserts
  October 25; unanchored at October 23 its relative
  resolves to the governed November 1 and R18-2 silences
  the group. (2) A relative-only group contributes its
  resolution: "(next Sunday)" on a date line contributes
  October 25 (and is no longer an in-words line),
  "(this Sunday)" October 18. (3) A bare "Sunday"
  standing directly in front of a framed date inside a
  group is that date's framing weekday, not a signal,
  and a group that is a bare "(Sunday)" alone stays the
  weekday garnish the r9c stacked-parentheses pin
  carries — it contributes nothing, as "(Monday)" never
  has. (4) A governor-free mixed group follows the
  standing two-date group rule, as two framed dates do:
  "(November 1, next Sunday)" contributes neither and
  offers both as alternatives, in written order.
- **R18-7 — governed relatives named.** The plain signal
  scan is extracted as `scan_plain_signals` (the plain
  reading uses it unchanged), and `date_line` now asks
  the composition alone — never the first-signal
  fallback — for its reading of a salvaged line's core:
  when the composition reads the SAME date the inner
  readers read, its naming joins the second-date note.
  "cancelled, next Sunday, November 1, 2026" asserts
  November 1 and names October 25; the "drop" twin reads
  the same. The composition-only fence is what keeps a
  Sunday-after line's restated base a restatement: the
  fallback's second dates are never merged.

## Shipped addendum, R18-B — 2026-10-17

Batch R18-B (claim/for/adverb edges) shipped in
`crates/ob-extract/src/contribution.rs`: R18-4, R18-5,
R18-8, R18-9, R18-10, R18-11. Six new pin tests, all
`r18b_*`, one per finding. **Tests: 721 → 727 passed,
0 failed** (full workspace suite run unpiped on the
final code, cargo exit 0; ob-extract 208 → 214).
**Clippy: clean** (`--workspace --all-targets -- -D
warnings`, exit 0, zero warnings). Neither the
fake-helper ETXTBSY flake nor timing contention fired
in any run. Booklet bytes 31,717 / 34,977 exact, held
by the full suite. Held lines stand: the claim/scope
pins ("(not moved to November 8)" nothing, "delayed
until" no claim, deadline-until unclaimed), the
defensible-silence family, the R18-A family
(governance by date, group relatives, naming), the
anchor family, and the bare-parity pin ("(it was
moved November 1)" governs; its active twin "(we
moved it November 1)" governs too).

**Changed pre-existing tests: none.** Every witness
below is pinned as a new `r18b_*` expectation; no
pre-existing pin moved, and the ob-extract suite
passed 208/208 on the production changes before the
new pins were added.

What shipped, per finding:

- **R18-4 — intervening date is a from-date.** In
  `target_claim`, a clause break standing directly
  before an intervening date is that object's list
  comma in front of the date, exempt like the commas
  inside the date's own span. "(moved the meeting,
  November 1, 2026, and the dinner to November 8)"
  now asserts November 8 with November 1 governed and
  named, exactly as its comma-after-date twin did, on
  the relative, paren-only, and Keep-through paths;
  the plain-line twin (which asserted the from-date
  itself, November 1, before) reads the same way.
- **R18-8 — the reach ends at a real second clause.**
  The rule, stated plainly: after an intervening
  date's span, the reach continues only into the
  target construction itself — the "to" directly,
  its lead-ins "then to" / "and to" — or into an
  object continuation of the phrase's own object
  ("and the dinner to …", "the dinner to …"). A new
  finite verb clause standing there ends the reach.
  The distinguisher is the finite verb: object
  continuations name things; a second clause does
  something. The finite-verb test is a closed list
  of common clause-verb stems plus their irregular
  forms ("sang" the witness word), the regular -ed
  form, and a determiner rule — a verb form standing
  directly after a determiner is that noun phrase's
  own material ("the meeting", "the baked beans"),
  never its verb. Judgment call, stated plainly:
  "(moved November 1, choir sang to November 8)" no
  longer CLAIMS November 8 and no longer names
  November 1 as a from-date (the note is empty);
  November 8 still asserts, standing on its own as
  the group's lone ungoverned date — the same
  standing "(choir sang to November 8)" has always
  had, pinned in the same test. The claim is what
  died; the date's own standing was never the
  claim's to give. The check judges only the words
  after the intervening date's span, so a
  verb-repeated target ("moved November 1, moved to
  November 8") still claims: the repetition
  re-anchors the phrase, and no intervening date
  stands after the claiming phrase.
- **R18-5 — the for-date enters composition.** Each
  occurrence's for-standing is recorded from its own
  window (a passive or active "for" zone ending it;
  a prospective "use" standing as the window's last
  word). In a group no target claim already orders,
  a "use" date standing after a for-date asserts
  with the for-date named, and otherwise the
  for-date reads by primacy with the other noted:
  "(it was scheduled for November 1 and November 8)"
  asserts November 1, the reversed order asserts
  November 8, "(it was moved for November 1, use
  November 8)" asserts November 8 with November 1
  named. The lone for-date forms keep contributing,
  and a "then to …" chain still reads by its claim.
  Judgment call: the plain-line twin of the "use"
  witness is fixed by the same composition in
  `plain_composition` ("it was moved for November 1,
  2026, use November 8, 2026" asserts November 8);
  the plain primacy half needed no new code, because
  an ungoverned plain line already reads its first
  signal and the for-date stands first in its
  witness.
- **R18-9 — active "for".** `active_for_zone_start`
  mirrors the passive zone: the window ends in
  "for", a displacement phrase stands before it,
  and nothing stands between the phrase and the
  "for" but the phrase's own object (object words
  only — no second verb, governor, auxiliary, or
  weekday). `governed_before` takes the earlier of
  the passive and active zone starts. "(we moved it
  for November 1)", "(moved it for November 1)",
  "(she moved it for November 1)", the shifted /
  bumped / rescheduled forms, and "(we moved the
  meeting for November 1)" all contribute November
  1. Scope fences pinned: the bare active parity
  "(we moved it November 1)" still governs, and an
  active cancellation "(we cancelled it for
  November 1)" stays silent — the family is the
  displacement family (move/shift/bump/reschedule
  plus the phrasal pairs), as the disposition
  scoped it. Beside a second date the active
  for-date composes by R18-5 primacy: "(we moved it
  for November 1, November 8)" asserts November 1
  with November 8 named, where RT18-4 found the
  for-date dropped off the note entirely.
- **R18-10 — adverb tolerance.** The three tolerance
  scans (`passive_for_zone_start`,
  `is_spent_predicate`, the claim-zone extension in
  `target_claim`) now cross up to TWO consecutive
  listed adverbs, and the closed list gains then,
  briefly, shortly, formally, unanimously, happily —
  24 words in all. "(it was just recently moved,
  November 1)" spends like its has-been twin, as do
  the reversed stack, the "has been just recently"
  position, and the for-form. The cap is pinned:
  "(it was still just recently moved, November 1)"
  and "(it was just recently finally moved,
  November 1)" still govern, and an unlisted word
  ("family") still does not spend.
- **R18-11 — cross-group naming.** A new reading,
  `chain_superseded_dates`, walks a line's trailing
  groups in written order: when two or more groups
  contribute dates, a target phrase with its
  "to"/"until" stands in some contributing group,
  and the last contributing group wears a
  "to"/"until" link of its own (the phrase may
  stand in the first group while the last is only
  its continuation — "(moved to November 8) (then to
  November 15)" is the witness shape), every earlier
  contributing group's date is a superseded target.
  Such dates join the line's second-date note on
  the paren-only, Keep-through, relative, and
  salvaged-absolute paths, and they leave the
  parenthesized-mismatch comparison — a chain link
  is not a garnish disagreement, so the split chain
  now carries no mismatch, exactly like its
  single-group twin. "(November 1, moved to
  November 8) (then to November 15)" names both
  November 1 and November 8; the three-link split
  names both superseded targets. Held control: a
  bare stack "(November 1) (November 8)" has no
  target construction, keeps its garnish mismatch
  (named November 1 against read November 8), and
  names nothing on the note.

## Coordinator close (2026-10-17)

Both batches shipped and were independently re-verified by the
coordinator (full unpiped suite + clippy -D warnings at each step):
R18-A `172e80c` (721 tests; one token cleaning, governance by date
value, target ranking across framed + relative targets, groups
with relative signals at the anchor, governed relatives named; ONE
flagged pin change - the R12-A "(not November 1, November 1)" block
was R18-2's witness shape and moved under it, marked in the test),
R18-B `fe37adc` (727 tests; intervening date as from-date, reach
ending at a finite verb clause, for-date composition, active
"for", two-adverb tolerance with the extended list, cross-group
superseded naming; zero changed pins; one judgment call recorded -
in the R18-8 witness the claim dies but November 8 keeps its own
standing as a lone ungoverned date). Round 19 audits these fixes.
The Tier-2 final gate runs only after a round returns zero
BLOCKER + zero MAJOR.
