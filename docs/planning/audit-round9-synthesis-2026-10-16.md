# Round 9 audit — synthesis (2026-10-16)

Three blind Wonder lanes over HEAD `bda6fc3` (round 8
closed: R8-A `18c3520`, R8-B `f6f756d`, R8-C `d6afce9`,
R8-D `bda6fc3`). Reports:
`~/workspace/openbulletin-audit-wave3/round9-verify.md`,
`round9-review.md`, `round9-redteam.md`.

**Verdict: 0 BLOCKER, 3 MAJOR, 12 MINOR (deduplicated
across lanes). Exit bar NOT met.** The verify lane
returned the loop's first complete pass — 51 of 51
round-8 claims VERIFIED, booklet exact, Bach live,
614/0 — and every round-9 failure is a NEW composition
beyond the pinned shapes, not a failed claim. The
publish fence held under an 8,308,800-sequence brute
force. What remains is one exempt license class, one
cost claim that was never true, and one judging set
scoped one level too wide, over a tail of
parenthesized dates and refusal-sentence accounts.

Lane raw counts: verify PASS (51/51); review
0 BLOCKER / 1 MAJOR / 4 MINOR; red team 0 BLOCKER /
3 MAJOR / 12 MINOR. Deduplication: the enumeration
cost was found by both review and red team (RT9-2)
with independent measurements; the assertion-at-fold-
end, retry step-aside, Keep-through parenthesis, and
kinds findings each appear in two lanes and are
counted once.

## Findings and dispositions

### R9-1 (MAJOR, red team RT9-1) — LicensedReprint is exempt from both judges when it is the standing license

R8-A unified the FALLBACK arms. But the togetherness
rule fires only when the composed license is an
attribution license, and the fence reversion covers
only ChurchOwned / PublicDomain / Cc0 — so a
LicensedReprint that IS the standing license is
judged by nothing, in single merges and folds alike:

- `[lr, kidsCcBy]` composes to LicensedReprint
  standing (children noted, credit empty, no vouching
  record) where the pinned order `[kidsCcBy, lr]`
  composes to Unknown.
- `[kids, lr]` — the unvouched reprint license simply
  FILLS the empty slot beside a children note.
- `[kidsCcBy, titleOnly, lr]` — the single-merge path
  and the fold DISAGREE (LicensedReprint vs Unknown).

Brute force: 73,430 sequences (of 8,308,800) end with
an unvouched LicensedReprint standing; 497 record
multisets flip between LicensedReprint and Unknown
purely by fold order. The picture never publishes;
the harm is R8-3's, one ring out: the composed record
asserts a reprint license no record vouched, and the
answer depends on record order in the file.

Disposition (R9-A): ONE final judgment on every
composed record, whatever path composed it (fill,
standing, or fallback; single merge or fold). If the
composed record notes children and its credit line
is empty and it carries a license, that license
stands only if vouched: some record in the
composition held that same license together with a
children note, or together with the credit that
stands, or a children-bearing record in the
composition noted a permission under the fence's
bearer rule. Unvouched, it falls — to another license
in the composition that IS vouched under the same
judgment, else Unknown. Pins: all three witness
shapes in both paths composing identically
(Unknown); the vouched controls (a record holding
LicensedReprint + children, or LicensedReprint +
its credit) standing in every order; permutation
invariance of the (children, license, credit) triple
across each witness multiset; every R7-A/R8-A pin
unchanged in expectation — flag any that asserted an
unvouched standing LicensedReprint, loudly.

### R9-2 (MAJOR, review + red team RT9-2) — enumeration cost: one full scan per candidate

R8-D removed the candidate bound and claimed the
cost stays "proportional to the file's own tail
structures". It does not: each candidate pays a
full-file `find_signature` scan. Measured (red team,
identical 1 MB files, identical comment length):
plain pad loads in 0.02–0.04 s; 500 comment
signatures 1.26 s; 1,500 signatures 2.01 s; 2,900
signatures burned over 3:02 of CPU, unfinished. The
review lane measured ≈1.2 s per candidate on a 3 MB
file (≈206 s for 20 signatures on 30 MB). A crafted
comment can hang the app on open.

Disposition (R9-D): candidates are collected by ONE
scan of the tail region where end records can live
(the format's own search window — the trailing
64 KiB comment space plus the end structures it
points to), each candidate position recorded once;
per-candidate walks keep the cap early-stop. The
cost claim is restated honestly in the addendum
with before/after timings measured on the red
team's fixtures: one pass over the tail window plus
bounded walks, independent of how many signatures
the comment carries. Pins: the 500 / 1,500 / 2,900
signature fixtures load with correct counts (the
addendum records the measured times; round 10
re-times), and every R8-1 coverage pin (15-junk,
16-decoy, 64-decoy refusals) unchanged.

### R9-3 (MAJOR, red team RT9-3) — judged headings are contribution-global, not story-scoped

R8-7's disposition said a bold paragraph delimits
"when its words are a heading the app is judging
**in this story**". `judged_headings` collects every
part's heading with no target check. A/B probe: the
same weekly story, the same Sunday school part —
adding a `## Youth` part that targets a NAMED youth
story flips the Sunday school part from AlreadyIn
to NotApplied, because the bold content line "Youth"
inside the Sunday school block now delimits its
section; on apply, "Bring a friend to class." lands
a second time and prints twice. The part that
redrew the boundaries never touches the weekly
story.

Disposition (R9-B): the judged set is scoped per
story. When sections are computed in a story, the
delimiting bold headings are the target heading
plus the headings of parts whose target is THAT
story. Pin the A/B verbatim (B1 unchanged by adding
the Youth part: AlreadyIn, no landing, no
duplication), plus the R8-7 converse pin (a bold
line carrying a heading of a part that DOES target
the story still delimits).

### R9-4 (MINOR, red team RT9-4) — the fold's fallback winner is first-match in fold order

Two vouched licenses in one fold compose to
different final claims by order: the fallback picks
the first match it walks past.

Disposition (R9-A, with R9-1's final judgment):
fallback selection is order-independent — prefer a
vouched license carried by a children-bearing
source record; among equals, a fixed precedence
stated in the code and the addendum. Pin the
witness pair composing identically in both orders.

### R9-5 (MINOR, review + red team RT9-5) — in a fold, the assertion outlives the license it rode in on

The fold-end judge drops the license but leaves the
`asserted_by/at` that the same incoming record's
license fill had carried in; single merges and
folds diverge (in a single merge the R7-8 gate
judges the assertion against what finally stands).

Disposition (R9-A): at fold end, the assertion is
re-judged by the R7-8 rule against the FINAL folded
record: it stands only if a substantive rights fill
from its own record stands there. Pin the witness;
single-merge pins unchanged.

### R9-6 (MINOR, review + red team RT9-6) — the loader retry steps aside only for a missing document, never for a failed one

A fake directory that LISTS `document.json` pointing
at junk (bad JSON, version 999, lying fingerprint or
size) aborts the load on the chosen candidate; the
honest candidate behind it is never tried. The
retry's verification is identical to the primary
path's (held) — it is the step-aside condition that
is too narrow.

Disposition (R9-D): the retry steps aside on ANY
failure of a candidate's document — missing,
unparseable, wrong version, failed hash/size
checks — and tries the next candidate; only when
every candidate fails does the load refuse, with
the first candidate's error. Pins: both lanes'
witness shapes load the honest bulletin; a file
whose every candidate is junk still refuses.

### R9-7 (MINOR, red team RT9-7) — the suffix rule is brittle at its edges

The parenthesized-suffix match (R8-8) applies only
to STYLED headings: a bold-fallback heading
"Sunday school (Room 2)" is not the heading, and a
styled suffix heading with a trailing period
("Sunday school (Room 2).") is not either. Both
land a second heading and a second copy.

Disposition (R9-B): the suffix rule applies to
bold-fallback headings as it does to styled ones,
and trailing punctuation on a heading paragraph
does not defeat the match. Pins: both shapes
recognized; "Sunday school teachers" still not.

### R9-8 (MINOR, red team RT9-8) — the contribution's own suffixed heading is not a part heading

A sender writes `## Sunday school (Room 2)`: the
part is not recognized as the Sunday school part at
all — it lands as a plain announcement and its
heading line is swallowed.

Disposition (R9-B): part headings match document
headings symmetrically — a part heading whose words
are a document heading's words plus a parenthesized
suffix (or the reverse) is that heading's part, and
its words land under the document's heading as
written there. Pin the witness (the part plans as
Sunday school, lands under the standing "Sunday
school" heading, no suffixed heading is written).

### R9-9 (MINOR, review + red team RT9-9) — the parenthesized date is never read on Keep-through lines

The R8-C memory lives only on the For Sunday path.
"Keep through the Sunday after next Sunday
(November 8)" reads nothing at all on the
Keep-through line — the sender's one absolute date
is lost in silence. (Relative phrases on
Keep-through remain unread by design: no anchor
machinery, recorded since R6-D.)

Disposition (R9-C): on a Keep-through line, a
trailing parenthesized absolute date is read as
the keep-through date when the line's own words
yield none; when the line yields a date AND a
parenthesized date, they compose as garnish —
agreeing stands, disagreeing records the
keep-through mismatch. Pin both shapes and the
For Sunday pins unchanged.

### R9-10 (MINOR, red team RT9-10) — stacked parentheses compare only the last group

"the Sunday after next Sunday (November 1)
(October 25)": only the last group is remembered;
when it agrees, a disagreeing earlier group is
laundered clean.

Disposition (R9-C): every parenthesized date group
on the phrase is remembered; the verdict compares
each — all agreeing reads as today, any
disagreement arms and names the disagreeing date.
Pin the witness (last-agrees/earlier-disagrees
arms; both-agree reads clean).

### R9-11 (MINOR, red team RT9-11) — when the parenthesized date agrees with the bulletin, the armed sentence names one date on both sides

Bulletin October 25; "the Sunday after next Sunday
(October 25)": the relative resolution is
November 1, the parenthesis disagrees with it, and
the armed sentence names October 25 as both the
file's Sunday and the bulletin's — the slug
sentence's disease in a new place.

Disposition (R9-C): the parenthesized date is the
sender's explicit date and the stronger signal.
When it equals the open bulletin's date, the
verdict is SameSunday (unarmed) and the review
notes the words also named a relative Sunday that
would be November 1. When it matches neither the
resolution nor the bulletin, the armed sentence
names the resolution and the parenthesized date —
never one date on both sides. Pin the witness in
both branches.

### R9-12 (MINOR, red team RT9-12) — on absolute lines a parenthesized date is never examined

A plain date with a trailing parenthesized date
reads as no date at all; a Sunday-after line's
contradictory parenthesis is invisible.

Disposition (R9-C): a trailing parenthesized
absolute date composes on absolute lines as other
garnishes do — on a plain date line it is compared
with the date itself, on a Sunday-after line with
the resolution; agreeing stands clean, disagreeing
records the mismatch, and it never silently kills
the plain reading. Pins: both shapes, agreeing and
disagreeing, both lines where the machinery is
shared.

### R9-13 (MINOR, red team RT9-13) — a second full date later in a date line is never remarked

"on Sunday, October 25, 2026 was when the fair was
supposed to be, use November 1 instead" concludes
SameSunday on the framed date; no sentence is
false, but the sender's operative second date is
invisible to the review.

Disposition (R9-C): when a date line contains a
second parseable absolute date after the one that
was read, the review's line for the contribution
says so in plain words (the file named another
date too — check which one was meant), without
changing which date governs. Pin the witness; the
single-date pins' sentences unchanged.

### R9-14 (MINOR, review + red team RT9-14) — the kinds misdescribe two more compositions

Review: a deficit rename segment plus an
addition-only segment yields {renamed, added},
matching no possible reading of the edit. Red team:
surplus shapes assert `moved` while denying a
removal that stands. The R8-D builder's documented
reading (an old-only segment counts toward moved
only when an unresolved segment holds surplus new
names) fails both compositions.

Disposition (R9-D): the kinds are computed from
the pairing's per-segment outcomes directly — each
segment contributes the kinds its own evidence
shows (renames it paired or left unresolved,
removals and additions it resolved, a move only
when names demonstrably crossed segments) — not
from global totals plus a surplus inference. Pins:
both lanes' witness shapes with their true kind
sets, and the canonical crossed-renames kinds and
sentence unchanged.

### R9-15 (MINOR, red team RT9-15) — the positional excuse reads filtered runs

`serving_pairing`'s segment runs are filtered of
kept names before positions are read, so a kept
name moved into the segment shifts every later
position: a different line's name lands at the
claimant's raw position, positional evidence
assigns the contested name wrongly, and a possible
hijack is excused with its reason lost (the
Beta/Wanderer witness; the save is still refused).

Disposition (R9-D): positional evidence is read on
the segment's UNfiltered runs — positions as the
lines stand, kept names included as in-segment
anchors. Pin the witness (the hijack's reason is
named, not excused) and the R8-D positional pins
unchanged in outcome.

## Fix batches

- **R9-A** — rights, one final judgment: R9-1
  (MAJOR) + R9-4 + R9-5.
- **R9-B** — presence, scope and suffixes: R9-3
  (MAJOR) + R9-7 + R9-8.
- **R9-C** — parenthesized dates, everywhere:
  R9-9 + R9-10 + R9-11 + R9-12 + R9-13 (all MINOR,
  one family).
- **R9-D** — counts and serving, third pass: R9-2
  (MAJOR) + R9-6 + R9-14 + R9-15.

Each batch: single writer, scar rules, dated Shipped
addendum to this synthesis, coordinator re-runs the
full suite + clippy unpiped before accepting. Round
10 (verify / review / red team over the new HEAD)
follows R9-D. The Tier-2 final gate runs only after
a Wonder round returns zero BLOCKER + zero MAJOR.

## Shipped addendum, R9-A (2026-10-17)

Rights, one final judgment: R9-1 + R9-4 + R9-5,
in `crates/ob-model/src/lib.rs`. Full suite
**619 passed / 0 failed** (baseline 614; +5 new
pins, all in ob-model), clippy clean, exit codes
read unpiped from cargo's own output.

- R9-1: the class-specific arms are replaced by
  ONE final judgment, `judged_composed_license`,
  run on every composed record — in a single merge
  at the end of `merge_rights_records_inner`, and
  in a fold once at the fold's end over the fold's
  own source records (`judge_folded_composed_record`,
  replacing `judge_folded_composed_license`). If
  the composed record notes children and its credit
  line is empty and it carries a license, the
  license stands only if vouched: some record in
  the composition held that same license together
  with a children note, or together with the credit
  that stands in the composed record, or a
  children-bearing record in the composition noted
  a permission under the fence's bearer rule. An
  attribution license is never vouched on an empty
  credit line (its credit is its own requirement),
  which preserves the round-7 togetherness outcome
  inside the one judgment. Unvouched, a license
  falls to a vouched license from the composition,
  else Unknown. The publish fence itself is
  untouched; every fence pin passes unchanged.
  Pins (`r9_unvouched_licensed_reprint_falls_in_both_paths`,
  `r9_vouched_licensed_reprint_controls_stand_in_every_order`):
  the three RT9-1 witness shapes — `[lr, kidsCcBy]`,
  `[kids, lr]`, `[kidsCcBy, titleOnly, lr]`, plus
  their reverses — compose to Unknown in BOTH the
  single-merge path and the fold, and the two paths
  agree triple-for-triple on each shape. The vouched
  controls stand in every order in both paths: a
  record holding LicensedReprint + children, and a
  record holding LicensedReprint + its own credit
  (both as a children-bearing record whose credit
  stands, and paired with a children-bearing record
  holding the same credit).
- Permutation check
  (`r9_fold_triple_is_permutation_invariant_over_the_pool`):
  an in-tree brute-force-style check over a small
  archetype pool — every permutation of each tested
  multiset folds to the same (children, license,
  credit) triple (7 multisets, 26 permutations).
  Scope, stated plainly: each multiset tested
  carries at most one vouched non-attribution
  license, so the final judgment fully determines
  its triple. A multiset carrying two vouched
  licenses is NOT triple-invariant under all
  permutations, by the fill law rather than the
  judgment: when a vouched license is itself the
  standing license it stands (the first noted value
  is never overwritten), while a fallen license
  drops to the precedence winner — those multisets
  are pinned by their RT9-4 witness orders, next.
- R9-4 (`r9_fold_fallback_winner_is_order_independent`):
  fallback selection in `fallback_license` is a
  function of the composition, not of fold order:
  a vouched license carried by a children-bearing
  source record (tier 0) beats one vouched only
  through the bearer rule (tier 1); among equals
  the fixed precedence is **ChurchOwned, then
  PublicDomain, then Cc0, then LicensedReprint**
  (strongest claim first; a reprint license rests
  on an outside licensor and stands last), stated
  here and at `fallback_rank` in the code. The
  RT9-4 witness pairs compose identically in both
  orders: `[kidsCcBy, kidsCc0, church]` → Cc0
  (children-carried beats bearer-only),
  `[kidsCcBy, kidsPD, kidsChurch]` → ChurchOwned
  (precedence among children-carried equals), and
  `[kidsCcBy, kidsLR, kidsCc0]` → Cc0 (precedence
  over a reprint license).
- R9-5 (`r9_fold_rejudges_the_assertion_against_the_final_record`):
  at fold end, `rejudge_folded_assertion` re-judges
  the assertion by the R7-8 rule against the FINAL
  folded record — it stands only if a substantive
  rights fill from its own record stands there (its
  credit line, or its license). Pins: both RT9-5
  witness folds (`[empty, kidsCcByAsserted]` and
  `[titleOnly, kidsCcByAsserted, titleOnly]`) end
  at license Unknown with `asserted_by` /
  `asserted_at` empty; the single-merge control is
  unchanged; and a fold whose asserting record's
  own credit stands keeps its assertion.
- FORMAT.md: the merge paragraph now states the
  final-judgment rule (every license class, every
  composing path), the order-independent fallback
  with its precedence, and the fold-end assertion
  re-judgment, in place of the class-specific arms
  it described before.
- **Changed pre-existing tests: none.** No R7-A /
  R8-A pin changed, and no pin anywhere in the
  suite asserted an unvouched standing
  LicensedReprint — the R8-3 pin's fallback
  direction was the only LicensedReprint shape the
  old suite pinned, as the disposition expected.
  The full suite went 614 → 619 with additions
  only.
