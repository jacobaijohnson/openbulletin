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

## Shipped addendum, R9-B (2026-10-17)

Presence, scope and suffixes: R9-3 (MAJOR) + R9-7 +
R9-8, in `crates/ob-app/src/main.rs` and in
`part_heading` in
`crates/ob-extract/src/contribution.rs`. Full suite
**626 passed / 0 failed** (baseline 619; +7 new
pins: 6 in ob-app, 1 in ob-extract), clippy clean,
exit codes read unpiped from cargo's own output.
The fake-helper ETXTBSY flake did not fire.

- R9-3: the judged set is scoped per story.
  `judged_headings` now returns `JudgedHeading`
  pairs — a part heading's normalized words paired
  with the story that part targets (its named story,
  or the weekly story for a part landing under a
  heading there; a part landing in no story judges
  nothing) — and `heading_sections` filters the set
  to its own story before judging boundaries. A part
  targeting a named story never redraws another
  story's boundaries. Pin
  (`r9b_a_part_targeting_another_story_does_not_redraw_this_storys_sections`):
  the RT9-3 A/B verbatim. Control B1 (Sunday school
  part alone, bold "Youth" label inside its section)
  reads AlreadyIn; B2 (the same, plus a ## Youth
  part targeting a standing named "youth" story)
  changes nothing for the Sunday school part — its
  section is (1, 4), running past the bold label,
  the plan reads AlreadyThere, the claim reads
  AlreadyIn, apply lands nothing and "Bring a
  friend to class." stands once. The R8-7 converse
  pin (a bold line carrying the heading of a part
  that DOES target this story still delimits) and
  every R8-B pin pass unmodified, so plan and apply
  agree in both directions.
- R9-7: one matching helper, `heading_words_match`,
  serves styled and bold-fallback headings alike:
  the heading's words exactly, or plus a
  parenthesized suffix, with trailing punctuation
  (a final period) stripped before the match.
  `is_section_boundary` and
  `is_target_heading_paragraph` both read through
  it; the old styled-only suffix arm is gone. Pins:
  `r9b_a_bold_fallback_heading_with_a_room_suffix_counts_as_its_section`
  (RT9-7 D4: the bold "Sunday school (Room 2)"
  section is found, the standing line is not
  duplicated, the missing line lands inside it),
  `r9b_a_suffix_heading_with_a_trailing_period_counts_for_either_kind`
  (RT9-7 D6, the styled "Sunday school (Room 2).",
  plus the bold exact-words "Sunday school."), and
  `r9b_a_bold_longer_heading_still_does_not_count_as_the_parts_heading`
  (a bold "Sunday school teachers" still does not
  match; the styled form was already pinned by
  R8-B).
- R9-8: part headings match document headings
  symmetrically. `part_heading` (ob-extract) reads
  a heading line by the same shape — exact words
  (`part_kind_for_words`), or the words plus a
  parenthesized suffix, trailing period aside — so
  "## Sunday school (Room 2)" is the Sunday school
  part. Because the part's kind carries the base
  heading, it plans as an under-heading part for
  "Sunday school" and no suffixed heading is ever
  written. Pins:
  `a_part_heading_with_a_parenthesized_suffix_is_that_headings_part`
  in ob-extract (the witness parse, the
  second-part-after-Announcement shape, and "Sunday
  school teachers" staying body text),
  `r9b_a_suffixed_part_heading_plans_and_lands_under_the_base_heading`
  (plans as Sunday school under-heading, lands
  under the standing "Sunday school" heading, no
  "(Room 2)" text written, claim settles), and
  `r9b_a_suffixed_part_heading_with_no_standing_heading_writes_the_base_heading`
  (against a document with no such heading, the
  BASE heading "Sunday school" is written, not the
  suffixed one — shipped as the disposition
  directs; the sender's room note is a qualifier on
  the part, preserved in the filed contribution
  text, not a heading the bulletin prints).
- **Changed pre-existing tests: one, mechanical.**
  ob-app
  `r8b_a_subheading_style_with_a_non_literal_id_is_recognized`:
  its one setup line changed from
  `judged_headings(&c, None)` to
  `judged_headings(&doc, &c)` — the function now
  takes the document so it can scope each heading
  to its part's target story, and its unused
  target argument is gone. Every assertion in the
  test is unchanged, and it passes in its original
  expectation. No other pre-existing test changed;
  every R7-B / R8-B presence pin passes in its
  original expectation.

## Shipped addendum, R9-C (2026-10-17)

Parenthesized dates, everywhere: R9-9 + R9-10 +
R9-11 + R9-12 + R9-13, in
`crates/ob-extract/src/contribution.rs` with the
review sentences wired in
`crates/ob-app/src/main.rs`. Full suite
**631 passed / 0 failed** (baseline 626; +5 new
pins, all in ob-extract), clippy clean (exit 0, no
warnings), exit codes read unpiped from files
holding cargo's own output. The fake-helper ETXTBSY
flake did not fire.

- R9-9: a Keep-through line whose own words yield
  no date now reads a trailing parenthesized
  absolute date as the keep-through date (the last
  group stands, as the For Sunday memory has always
  taken it); when the line yields a date AND a
  parenthesized date, they compose as garnish —
  agreeing stands, disagreeing records the
  keep-through mismatch
  (`keep_through_parenthesized_mismatch`, naming
  both dates), exactly as the dated garnish rules
  do elsewhere; stacked groups on a relative
  Keep-through line that disagree with each other
  record the same mismatch against the group that
  stands. **Standing, recorded as directed:
  relative phrases on Keep-through otherwise remain
  unread BY DESIGN — no anchor machinery, recorded
  since R6-D; this batch built none.** Pin:
  `r9c_keep_through_reads_a_trailing_parenthesized_date`
  (the RT9-9 witness "the Sunday after next Sunday
  (November 8)", the review lane's "next Sunday
  (November 1, 2026)", the no-parenthesis relative
  standing, and both garnish shapes).
- R9-10: every trailing parenthesized date group
  on a relative phrase is remembered, in written
  order, in the new field
  `for_sunday_parenthesized_dates` (the round-8
  single field stays the last group).
  `week_verdict` compares each distinct group: all
  agreeing reads as today; ANY disagreement arms
  and names the disagreeing date. Pin:
  `r9c_stacked_parentheses_are_all_compared` —
  the bare witness "(November 1) (October 25)"
  against the October 25 bulletin now arms as
  DifferentSunday(November 1) instead of reading
  SundayInWords clean; the Sunday-after mirror
  "(October 25) (November 1)" arms naming
  October 25, and the review's sentence names the
  resolution and both groups; both-agree shapes
  read clean on both phrase shapes, and non-date
  groups ("(Room 2)", "(Sunday)") are skipped in
  both orders, as the round-8 probes held.
- R9-11: the parenthesized date is the sender's
  explicit date and the stronger signal. When it
  equals the open bulletin's date, the verdict is
  SameSunday (unarmed) and the review's first line
  is: "These words say they are for Sunday,
  October 25, 2026, the date written in
  parentheses. The words also name a Sunday in
  relative words that would be Sunday, November 1,
  2026." When it matches neither the resolution
  nor the bulletin, the armed sentence names the
  resolution and the parenthesized date: "These
  words name a Sunday in relative words that would
  be Sunday, November 1, 2026. The date in
  parentheses is Sunday, November 8, 2026. The
  bulletin open is for Sunday, October 25, 2026."
  Never one date on both sides. Pin:
  `r9c_a_parenthesized_date_that_matches_the_bulletin_reads_same_sunday`.
  The review's SameSunday arm no longer assumes
  `for_date` is set (this shape reads no date from
  the words alone).
- R9-12: absolute lines compose a trailing
  parenthesized absolute date as garnish, on both
  lines where the machinery is shared
  (`date_line`): the line's own date governs, a
  disagreeing parenthesis records a
  `ParenthesizedDateMismatch` (For Sunday:
  `for_sunday_parenthesized_mismatch`; Keep-through:
  the same record as R9-9's), agreeing stands
  clean. On a Sunday-after line the comparison is
  with the resolution. A plain date line the
  parenthesis used to poison whole ("October 25,
  2026 (November 1)" read as NO date) is re-read on
  its core with the trailing groups off, so the
  parenthesis never silently kills the plain
  reading — that shape now yields October 25 with
  the mismatch recorded. The trailing-weekday
  parenthesis control ("(Monday)") stands as it
  always has. Pin:
  `r9c_absolute_lines_compose_a_parenthesized_date_as_garnish`
  (both shapes, agreeing and disagreeing, both
  lines).
- R9-13: when a date line contains a second
  parseable absolute date after the one that was
  read (`for_sunday_second_date` /
  `keep_through_second_date`), the review adds a
  note in plain words — "This file named another
  date too: Sunday, November 1, 2026. Check which
  date was meant." (Keep-through: "The
  Keep-through line named another date too: …") —
  without changing which date governs. The
  Sunday-after base is the anchor's source, not a
  second date; a parenthesized garnish is
  remembered on its own record, not double-counted;
  the same date named twice is not another date.
  Single-date lines' sentences are byte-identical
  to before (the note renders only when a second
  date stands). Pin:
  `r9c_a_second_date_in_a_date_line_is_noted_not_substituted`
  (the RT9-13 witness verbatim on both lines).
- **Changed pre-existing tests: one, BY DESIGN
  (flagged loudly).** ob-extract
  `r8c_a_parenthesized_date_on_a_relative_sunday_is_remembered_and_compared`:
  for "the Sunday after next Sunday (October 25)"
  against the October 25 bulletin, it asserted
  `WeekVerdict::DifferentSunday(oct(25))` — the
  R9-11 witness shape, whose armed sentence named
  October 25 on both sides. Under R9-11 that
  assertion now expects `WeekVerdict::SameSunday`,
  and the test carries a comment recording the
  change and its reason. That is the fix, not a
  regression: every other assertion in the test
  (the anchored-agreeing DifferentSunday(October 25)
  against October 18, the November 8 third-Sunday
  shape, the no-anchor NoSundayNamed, the bare
  relative shapes, and the unparseable-garnish
  shapes) passes in its original expectation, as
  does every other R8-C pin. No other pre-existing
  test changed. The full suite went 626 → 631 with
  that one expectation updated and additions
  otherwise.

## Shipped addendum, R9-D (2026-10-17)

Counts and serving, third pass: R9-2 (MAJOR) + R9-6 +
R9-14 + R9-15, in `crates/ob-format/src/lib.rs` and
`crates/ob-model/src/lib.rs`. Full suite **637 passed /
0 failed** (baseline 631; +6 new pins: 4 in ob-format,
2 in ob-model), clippy clean (exit 0, no warnings),
exit codes read unpiped from files holding cargo's
own output. The fake-helper ETXTBSY flake fired
twice under parallel load, in crates this batch does
not touch, and is stated exactly: ob-extract
`a_sleeping_helper_meets_the_deadline` panicked
"Text file busy (os error 26)" in the first full run
(cargo stopped there; its solo rerun passed in
1.02 s, and ob-extract passed 156/156 in the
no-fail-fast run), and ob-clips
`a_prompt_converter_reports_its_own_result` failed
in the no-fail-fast run (636 passed there); the
ob-clips suite passed solo, 7/7.

- R9-2: the enumeration no longer pays a scan per
  candidate, in either of the two places that
  charged it. For the count: end-record candidates
  are collected by ONE scan of the tail window where
  end records can live — 65,633 bytes (the 65,535-byte
  comment space plus the end record, a zip64 locator,
  and a zip64 end record) — each position recorded
  once, latest first. Every candidate's directory
  start (and any zip64 record position its locator
  names) is resolved by ONE shared first-signature
  pass per signature family over the region the
  candidates' windows span (`resolve_first_hits`; a
  second pass runs only for zip64 records, whose
  positions must be known before their directory
  windows exist — the shipped code's own dependency).
  The per-candidate directory walks are unchanged,
  cap early-stop included, and a count past the cap
  from ANY candidate still refuses the file, at both
  loaders. For the load itself, an isolation probe
  showed the zip reader's OWN candidate search was a
  large share of the cost (`ZipArchive::new` alone on
  the 500-signature fixture: 5.89 s, against 0.008 s
  for the plain pad), so the loaders now walk the
  candidates themselves and open each walkable one
  at its own end record through a trimmed stream
  (`TrimmedReader` — the file's prefix is never
  copied). The honest cost claim, restated: one tail
  scan, one shared pass per signature family, bounded
  walks, and one trimmed open per walkable candidate
  — independent of how many signatures the comment
  carries. Measured with a scratch harness calling
  the shipped `load` (debug build, this shared
  machine), on fixtures rebuilt to the red team's
  builder byte for byte (a 1 MB incompressible
  picture; the comment padded to identical length
  both ways). BEFORE, the lanes' numbers: plain pad
  0.02–0.04 s; 500 signatures 1.26 s; 1,500 → 2.01 s;
  2,900 → did not finish (the load + load-insert pair
  burned over 3:02 of CPU, killed). Spot-checked on
  this machine before the change with the same
  harness: plain-500 0.109 s, sigs-500 21.58 s,
  sigs-1500 unfinished inside a 60 s cap. AFTER, same
  harness, same fixtures: plain-500 0.071 s,
  sigs-500 0.132 s; plain-1500 0.088 s, sigs-1500
  0.147 s; plain-2900 0.068 s, sigs-2900 0.158 s;
  and `load_insert` on sigs-2900 (a bulletin at the
  sheet door) refuses "no loose sheet" in 0.120 s.
  No unit test asserts a wall clock; the behavior at
  scale is pinned (`r9_thousands_of_comment_signatures_count_and_load_the_honest_bulletin`:
  2,900 comment signatures in both fake shapes count
  the real directory's two records and load), and
  every R8-1 coverage pin (15-junk, 16-decoy,
  64-decoy refusals; honest loads; boundary
  exactness; the RT8-9 comment loads) passes in its
  original expectation.
- R9-6: both loaders try candidates latest first,
  and a candidate steps aside on ANY failure of its
  document — missing, unparseable, wrong version,
  failed hash or size checks — not only when its
  directory holds no document. When every candidate
  fails, the load refuses with the FIRST failed
  candidate's error. One bound, stated plainly: when
  no walkable candidate can even be opened (a
  directory the count walk cannot walk, or one
  outside the tail window), the reader's own path
  still decides, with the same stepping aside; when
  a walkable directory opened and held no document,
  the no-document words are the answer. Pins
  (`r9_a_failed_candidate_document_steps_aside_for_the_honest_directory`,
  `r9_a_failed_candidate_sheet_steps_aside_for_the_honest_directory`):
  each junk flavor (bad JSON, version 999, a lying
  asset fingerprint, a lying declared size) in the
  chosen directory AND behind an appended empty
  directory loads the honest bulletin; at the sheet
  door the flavors that translate (bad JSON, lying
  fingerprint, lying size — a sheet package carries
  no format version) load the honest sheet.
  `r9_a_file_whose_every_candidate_is_junk_refuses_with_the_first_error`:
  an all-junk file refuses with the latest
  document-holding candidate's error (the version
  error, standing in front of a bad-JSON decoy), not
  the error behind it and not the no-bulletin words;
  the sheet door's all-junk shape refuses with its
  first error likewise. Flood shapes still refuse by
  the cap before any candidate is tried.
- R9-14: `ServingLineEditKinds` are computed from
  the pairing's per-segment outcomes
  (`serving_pairing`), not from global totals plus a
  surplus inference. The rule, segment by segment:
  a segment's renames are what it paired or left
  unresolved; surplus new names and deficit old
  names in unresolved segments first answer each
  other (a surplus and a deficit spread over
  segments are the same crossed renames — neither an
  addition nor a removal); a removal-only segment
  BETWEEN anchors is a removal the pairing resolved
  in place, never a partner reservoir; the segment
  after the last anchor can hold a leftover
  surplus's partners (the canonical crossed
  renames), up to as many of its names as the
  surplus needs, and its names past the need are
  removals; a surplus with nothing left to pair
  with is an addition, and a deficit left over
  after any crossing is a removal; unresolved
  names spread over two or more segments still
  claim `moved` on their own. Pin
  (`r9_kinds_come_from_each_segments_own_outcome`):
  the review lane's K1 (deficit segment +
  addition-only segment) reads {renamed, removed,
  added}; the red team's RT9-14 witness (surplus
  segment + a bounded removal segment) reads
  {renamed, removed, added}, and the same shape
  with a clean addition-only segment added reads
  the same — the shipped {renamed, moved} and
  {renamed, added, moved} claimed a move the
  pairing could not tell from an addition while
  denying the removal standing in its own segment;
  the mirror control K2 (surplus segment + an
  old-only segment after the last anchor) keeps
  the canonical {renamed, moved}; a surplus
  segment standing alone reads {renamed, added}.
  The canonical crossed-renames kinds {renamed,
  moved} and its round-6 sentence stand word for
  word, and the R8-D pins S-C {renamed, removed,
  added}, S-D {renamed, removed}, D8 {renamed,
  removed} stand as pinned — all verified by the
  unmodified pre-existing tests.
- R9-15: `serving_pairing`'s stored segment runs
  are the segments' lines AS THEY STAND — kept
  names included as in-segment anchors — and the
  slug-conflict check reads its positional evidence
  on those unfiltered runs. (The pairing itself
  still pairs on the filtered runs, as it always
  has; only the stored evidence changed.) Pin
  (`r9_positional_evidence_reads_the_segments_unfiltered_runs`):
  the Beta/Wanderer witness verbatim — old
  [Alpha, Kids message, Choir, Wanderer, Beta,
  Omega], new [Alpha, Beta, Children's message,
  New B, Extra, Wanderer, Omega] — now names Choir
  as the taker of the claimed name, where the
  filtered runs had aligned the contested name with
  the claimant and excused the hijack; the save is
  still refused, as ambiguous. The R8-D positional
  pins (S-B, S-A2, S-A, and the R7-9 excuse) are
  unchanged in outcome.
- FORMAT.md: the serving-kinds paragraph now states
  the per-segment rule and the unfiltered positional
  runs, and the loading rules gained the entry-count
  walk (tail-window collection, one shared pass, any
  candidate over the cap refuses) and the candidate
  stepping (any failed document steps aside; the
  first candidate's error when all fail).
- **Changed pre-existing tests: none.** No
  pre-existing assertion was removed or altered
  anywhere in the suite (verified on the diff);
  every R8-1 / R8-D pin, the canonical kinds and
  sentence, and the R8-D positional pins pass in
  their original expectation. The full suite went
  631 → 637 with additions only.
