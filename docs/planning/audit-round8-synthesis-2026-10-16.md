# Round 8 audit — synthesis (2026-10-16)

Three blind Wonder lanes over HEAD `9634bcc` (round 7
closed: R7-A `610effa`, R7-B `fe01627`, R7-C `251642c`,
R7-D `9634bcc`). Reports:
`~/workspace/openbulletin-audit-wave3/round8-verify.md`,
`round8-review.md`, `round8-redteam.md`.

**Verdict: 0 BLOCKER, 6 MAJOR, 8 MINOR (deduplicated
across lanes). Exit bar NOT met.** The publish fence
held under a 245,388-sequence brute force; every
round-7 disposition is implemented as written in the
shapes it pinned. The failures are one ring further
out: a bound the fix inherited, a fallback arm with no
condition, two readers of one section disagreeing
about which section, and framing readers that stop
one word too early or one candidate too soon.

Lane raw counts: verify 38 VERIFIED / 2 FAILED / 0
UNTESTABLE of 40 round-7 claims; review 0 BLOCKER /
3 MAJOR / 6 MINOR; red team 0 BLOCKER / 5 MAJOR /
5 MINOR. Deduplication: the entry-cap bound was found
by all three lanes (verify D4, review R8-R1, red team
RT8-1) and the duplicate-section double-print by two
(review R8-R3, red team surface 2); the moved-kinds
MINOR was found by all three (verify D8, review
R8-R7, red team RT8-6); the serving conflict/excuse
MINORs by two (review R8-R8 on code derivation, red
team RT8-7/RT8-8 on runnable probes — the probes
govern where they differ in scope).

Held, on the record: the rights publish fence under
brute force (245,388 fold sequences, ≤4 records, 22
archetypes — zero manufactured permissions, zero
attribution license standing beside children with an
empty credit, and the real save→load→`export_html`
path holds children-noted pictures back); applied-
record forgery against heading-aware presence;
garnish stacking and Sunday-after compositions from
the round-7 task list; presence wrong-story /
loose-sheet / 3×-repeat / re-apply shapes; booklet
baseline exact (31,717 / 34,977); live Bach test
(237 noteheads, staff 6.4–6.6 mm); 590/0 in the
verify lane, clippy clean; all four round-7
changed-pin addenda verified accurate against the
diffs by the review lane.

## Findings and dispositions

### R8-1 (MAJOR, all three lanes) — sixteen candidates exhaust the count's enumeration, and the flood walks in

R7-D taught the cap to count every candidate the
enumeration presents, but the enumeration itself
stops at 16 candidates, latest first
(`end_record_candidates`, ob-format). Append 15
genuinely unwalkable junk end records plus one fake
2-record directory to a 9,001-entry archive (red
team's boundary: 14 junk = refused, 15 = loads), or
sixteen plausible decoy directories (review), and
the real end record is never counted: the cap
passes and `ob_format::load` serves the bulletin.
The R7-D addendum disclosed the bound; the
disposition's words ("every end-record candidate
the file presents") claim more than sixteen.

Disposition (R8-D): the cap check enumerates every
end-record candidate the tail scan finds — no fixed
candidate count. A candidate whose directory walk
fails is cheap (it breaks at the first bad header);
a walkable candidate counts with the cap early-stop
as now; refusal follows if ANY candidate directory
exceeds the cap. If a global work bound is needed,
it must refuse in cap words, never silently skip
candidates. Pins: RT8-1's 15-junk boundary shape and
the review's 16-decoy shape refused on both loaders;
a 64-decoy shape refused; every honest and held
shape from rounds 6–8 still behaving as pinned.

### R8-2 (MAJOR, review R8-R3 + red team) — checker and landing split judge different sections of a repeated heading

For an under-heading part, `target_checker` judges
the first section that holds the part's whole words;
`split_present_paragraphs_under_heading` splits
against `sections.first()` only. With two "Sunday
school" sections in one story and a part partially
present in the second, the split counts the standing
line as missing and inserts it into the first
section: the line prints twice, both times under its
own heading, permanently, via a recorded apply. A
repeated heading in one story (morning and afternoon
blocks) is an ordinary office shape.

Disposition (R8-B): one section judgment for
checker, plan, and landing. A paragraph standing
under the part's heading in ANY of its sections is
not landed again; whole-part presence remains "the
whole words within one section" as the checker has
it; missing paragraphs land in the first section.
Pins: the review's scenario verbatim (line in the
second section + one new line: only the new line
lands, in the first section; the standing line
stands once), the red team's empty-first-section
shape, and the masked whole-in-second-section shape
staying AlreadyThere.

### R8-3 (MAJOR, review R8-R2) — the fallback's LicensedReprint arm has no condition and displaces a standing license

The R7-A composed rule's fallback installs the other
record's license "when that license itself stands
under the publish fence and this same rule" (the
addendum's words). For `LicensedReprint` the arm
returns true unconditionally: merge {children noted,
CC BY, no credit} with {LicensedReprint only — no
children note, no credit} and the composed record
claims LicensedReprint, a license the fill-only law
would never have taken (the slot was filled),
overwriting the standing CC BY claim on the say-so
of a record that vouches for nothing. The picture
does not publish; the harm is the record, in the
R7-1 family, one grade down. (The red team's brute
force checked permission manufacture and attribution
states and held; this is a record-claim property it
did not enumerate, verified by the review lane's
direct probe.)

Disposition (R8-A): the fallback arms are uniform.
A fallback license stands only if its own source
record notes children or a bearer noted permission —
the condition the Cc0/PublicDomain/ChurchOwned arm
already applies — and LicensedReprint from a record
meeting neither condition falls to Unknown like any
other unvouched license. Pin the review's probe
verbatim; the standing-LicensedReprint mirror
control (license already standing, incoming refused
credit) stays as the review lane cleared it.

### R8-4 (MAJOR, red team RT8-3) — a framed date dies on any word after the date

`framed_date_outcome` parses everything after the
framing weekday as one date, so "use on Monday,
October 25, 2026 for the picnic" records nothing:
no date, no mismatch, NoSundayNamed under a false
first line — on both lines, and for the agreeing
control too (the shape dies on the trailing words,
not the mismatch). The Sunday-after variant with a
trailing word loses a determined date (Nov 1) the
same way.

Disposition (R8-C): the framed reading needs the
weekday directly in front of the date; what follows
the date is the line's remainder, not a parse
failure. A disagreeing framed date records its
mismatch wherever the date stands in the line; an
agreeing one yields its date by the same rules as
the line-ending form. The review lane's cleared
shape stands: a candidate whose following words do
not parse as a date ("on Monday we prayed, service
on Sunday, October 25, 2026") is not a framing, and
scanning continues.

### R8-5 (MAJOR, red team RT8-4) — a second "on" framing is laundered by the ordinary-text fallback

`framed_date_outcome` tries the FIRST "on"+weekday
it finds and gives up; `date_line`'s ordinary-text
fallback then returns the second framed date clean:
"meet on Friday, then use on Monday, October 25,
2026" reads SameSunday, unarmed, no mismatch — the
exact harm R7-3 was graded for, reached by any
ordinary "on <weekday>" earlier in the line. Both
lines.

Disposition (R8-C): the framed reader scans every
"on"+weekday candidate in the line until one frames
a date; and the ordinary-text fallback never returns
a date whose directly preceding weekday wears "on"
without the framing check having judged it. Pin F2
verbatim and its Keep-through mirror.

### R8-6 (MAJOR, red team RT8-5) — "on the Monday" defeats every framing check

The Sunday-after reader strips "on" and/or "the"
garnishes in a loop (R7-5); the plain-date readers
require the weekday immediately after "on". "For
Sunday: on the Monday, October 25, 2026" reads
clean — SameSunday, unarmed — the one garnish stack
that does not compose.

Disposition (R8-C): the plain-date framing readers
accept the same garnish stack in front of the
weekday as the Sunday-after reader does — optional
"on", optional "the" — wherever the framing stands.
Pin F4 verbatim and its mirror; the shipped "on
Monday" and bare-weekday controls unchanged.

### R8-7 (MINOR, review R8-R4) — a fully-bold body line truncates a heading section

`is_heading_paragraph` counts any all-bold paragraph
as a heading, so a bold emphasis line inside a
section ("Bring money!") ends the section; the
part's words standing after it read as absent and
land again, before the bold line. The R7-B scope
note disclosed the opposite failure (plain-typed
headings unrecognized), not this one.

Disposition (R8-B): a bold-fallback paragraph
delimits sections only when its words match a
heading the app is judging in this story (the target
heading, or another part heading of the contribution
being planned/applied); other bold paragraphs are
content. Heading-styled paragraphs always delimit,
as now. Pin the review's scenario verbatim.

### R8-8 (MINOR, review R8-R5) — heading recognition is exact-text and literal-id

(a) A styled heading carrying extra words ("Sunday
school (Room 2)") never matches the part's heading,
so a second heading and a second copy land. (b)
Latent: `heading_paragraph` finds the subheading
style by name OR id, but `is_heading_paragraph`
recognizes only the literal ids — a style table
whose subheading id differs would make the apply's
own re-check report not-landed forever. Not
reachable through shipped templates or the import
draft today.

Disposition (R8-B): (a) a heading-styled paragraph
whose words are the part's heading words plus a
parenthesized suffix counts as that heading's
section; a longer heading ("Sunday school
teachers") does not. (b) one shared name-or-id
recognition for the subheading style, used by both
the writer and the recognizer (the codebase's other
recognition, in markdown import, is already
name-or-id).

### R8-9 (MINOR, review R8-R6) — the composed rule is fold-order-sensitive across a neutral record

Pairwise judging has no memory: fold [A, T, C] —
A {children, CC BY, no credit}, T {title only},
C {children, credit} — and A+T drops the license
before C's credit arrives to stand beside it. Final:
credit standing, license Unknown. The same records
as [A, C, T] keep CC BY. Direction is conservative
(a true claim lost, none manufactured).

Disposition (R8-A): the fold paths
(`fold_duplicate_asset_records` and its callers —
load fold, sheet attach) judge the composed rule
once, on the FINAL folded record: fills and fence
reversions run per merge as now, but the
license-fall for an empty credit is deferred to
the fold's end, so a credit arriving later in the
same fold stands beside the license it belongs to.
Single merges outside a fold (`add_asset`) keep
the R7-A behavior exactly. Pins: [A, T, C] and
[A, C, T] compose identically (CC BY + credit +
title); every R7-A pairwise pin unchanged.

### R8-10 (MINOR, all three lanes) — "moved" counts segments the pairing resolved

`unpaired_segments` increments for removal-only and
addition-only segments too, so an ambiguous rename
segment plus a clean removal elsewhere yields kinds
{renamed, removed, moved} — a move that never
happened, claimed in the refusal sentence — and the
addition in the red team's S-C shape is omitted
while the absent move is claimed.

Disposition (R8-D): only segments with leftover
names the pairing could not resolve count toward
`moved`; resolved removal-only and addition-only
segments contribute their own kinds only. Pins:
RT8-6 S-C ({renamed, removed, added}, no moved) and
S-D ({renamed, removed}), verify D8's shape, and
the canonical crossed-renames sentence unchanged.

### R8-11 (MINOR, review R8-R8 + red team RT8-7/RT8-8) — around the built-in's own name, the conflict machinery names the wrong line, excuses the wrong save, and can name one line twice

Three faces, one root: the conflict judgment works
from set membership and a fixed second name instead
of from who is taking the name.

- RT8-7: `ServingLineSlugConflict.second` is always
  the built-in's display name, so whenever the
  contested name IS that name, the sentence names it
  on both sides — including the claimant-renames-
  back shape with NO ambiguity (S-B), and a shape
  where a different line takes the name while the
  holder stands as "Kids message" (S-A2: refusal
  right, sentence names the wrong holder).
- RT8-8: the ambiguous self-excuse is membership
  in the unpaired sets; a genuine hijack sharing
  those sets is excused, and the office hears only
  the ambiguous sentence, never that a line tried
  to take the children's-message line's place (the
  save is still refused; the reason is lost).
- Review R8-R8: the same membership test misses a
  claimant the pairing guessed across inside an
  equal-count segment, reporting the self-conflict
  sentence R7-9 ruled must never be reported.

Disposition (R8-D): the conflict judgment names
lines by the pairing's best evidence. (1) The
sentence identifies the taking line distinguishably
(the name it had, or "a new line") and the holding
side by the name the bulletin currently prints for
the claimed line — never the same unqualified name
on both sides. (2) Positional evidence inside
unequal segments counts: where the claimant's
in-segment position pairs it elsewhere and another
line sits at the contested name's position, the
conflict is reported against that line. (3) The
excuse is reserved for shapes where no positional
evidence assigns the contested name to a different
line. Pins: RT8-7 S-B and S-A2, RT8-8 S-A, the
review's crossed-segment shape, and R7-9's pin —
each refused (or excused) for the stated reason,
in sentences that name the right lines.

### R8-12 (MINOR, red team RT8-9) — an honest file whose comment carries end-record signatures is refused by misreading

A well-formed 2-entry bulletin whose zip comment
contains `PK\x05\x06` strings with self-consistent
lengths near the file's end: the reader takes a
comment-internal fake as its end record and reports
"no bulletin inside it"; the count walk defers
(None), so the cap is not the refuser. The round-7
held claim for EOCD-in-comment shapes survives
only outside that placement.

Disposition (R8-D): when the directory the reader
chose holds no document.json, the loaders try the
other enumerated end-record candidates' directories
before declaring no bulletin — the candidate
machinery from R8-1 already enumerates them. A
well-formed file whose real directory holds the
bulletin loads. Pin RT8-9's two fixtures loading;
every flood shape still refused (a fake directory
that yields no bulletin never satisfies a load).

### R8-13 (MINOR, review R8-R9, docs) — FORMAT.md states the pre-R7-10 sentence as universal

`docs/FORMAT.md:137-145` describes every ambiguous
refusal as "several lines were renamed and moved at
once… rename one line at a time". Since R7-D only
the canonical {renamed, moved} kinds produce that
wording.

Disposition (R8-D, docs): describe the refusal by
its kinds, matching the shipped sentences.

### Known-open carried shape — a trailing absolute date on a relative Sunday-after (MINOR)

"the Sunday after next Sunday (October 25)" still
names no Sunday (all three lanes confirm the
R7-C addendum's record). It has now been open for
two rounds; disposition it rather than carry it
again: the parenthesized absolute date is remembered
with the in-words phrase, and the verdict compares
it with the relative resolution once a bulletin
date anchors it — agreement reads clean, disagreement
names both dates and arms like any differing Sunday.
With no anchor, the review's in-words sentence
carries the sender's phrase as now. (R8-C.)

## Fix batches

- **R8-A** — rights, fold-final judging: R8-3
  (MAJOR) + R8-9. The merge funnel and its fold
  callers.
- **R8-B** — presence sections: R8-2 (MAJOR) +
  R8-7 + R8-8.
- **R8-C** — date framing, completed: R8-4, R8-5,
  R8-6 (MAJORs) + the carried trailing-date shape.
- **R8-D** — counts and small print, second pass:
  R8-1 (MAJOR) + R8-12 + R8-10 + R8-11 + R8-13.

Each batch: single writer, scar rules, dated Shipped
addendum to this synthesis, coordinator re-runs the
full suite + clippy unpiped before accepting. Round
9 (verify / review / red team over the new HEAD)
follows R8-D. The Tier-2 final gate runs only after
a Wonder round returns zero BLOCKER + zero MAJOR.

## Shipped addendum, R8-A (2026-10-16)

R8-3 + R8-9 shipped in one batch, in
`crates/ob-model/src/lib.rs`, with the load-fold pin
in `crates/ob-format` and the merge wording in
`docs/FORMAT.md` updated in the same commit.

- R8-3: the composed rule's fallback arms are
  uniform. `LicensedReprint` now stands as a
  fallback only under the condition the
  Cc0/PublicDomain/ChurchOwned arm already applied —
  its own source record notes children, or a
  children-bearing record noted a permission — and
  an unvouched LicensedReprint falls to Unknown.
  The review probe verbatim (standing {children,
  CC BY 4.0, no credit} merged with {LicensedReprint
  only}) now composes to Unknown, at merge level
  and through a fold
  (`r8_unvouched_licensed_reprint_fallback_falls`,
  which also pins the vouched control: a
  LicensedReprint from a children-bearing source
  stands). The cleared controls are pinned in
  `r8_fallback_controls_from_the_cleared_shapes`:
  ChurchOwned from a non-children source falls, a
  CC BY-SA "other" license falls, and the mirror
  direction (standing LicensedReprint, incoming
  CC BY with a fence-refused credit) leaves the
  standing license and refuses the credit. The
  vouched-CC0 fallback and the own-credit control
  were already pinned by the R7-A tests and pass
  unchanged.
- R8-9: `fold_duplicate_asset_records` runs its
  merges with the composed togetherness step
  deferred (`merge_rights_records_inner` with the
  defer flag; fills, fence reversions, the incoming
  pairing, and the assertion rule run per merge as
  before) and judges that step once, at the fold's
  end, on each record a merge actually composed
  (`judge_folded_composed_license`): an attribution
  license beside a children note with an empty
  credit falls — to a license one of the fold's own
  records carried that stands under the uniform
  fallback condition above (the fallback a single
  merge would have chosen for a two-record fold),
  and otherwise to Unknown. A record no duplicate
  folded into is untouched. Single merges outside a
  fold (including `add_asset`) call the undeferred
  merge and keep the R7-A behavior byte-for-byte.
  Pins: `r8_fold_judges_the_composed_rule_once_at_the_end`
  (both fold orders compose to CC BY + credit +
  title; the same [A, T, C] sequence through
  `add_asset` keeps the pairwise outcome, license
  Unknown with the credit standing) and
  `r8_fold_orders_compose_identically_on_load` in
  ob-format (both orders through a real .obd save
  and load fold). The R7-A three-record fold pin
  `r7_three_record_fold_drops_the_orphaned_license`
  passes unchanged: its credit is fence-refused, so
  the final judging still drops that license.
- Changed pre-existing tests: none. Every R7-A pin,
  and every other pre-existing test, passes
  unmodified.
- Numbers: full workspace suite 594 passed /
  0 failed (baseline 590 / 0; +4 new tests: 3 in
  ob-model, 1 in ob-format), cargo's own exit code
  0, read unpiped. Clippy (`cargo clippy
  --workspace --all-targets`) clean, exit 0, no
  warnings. `cargo fmt --all` run before the final
  run.

## Shipped addendum, R8-B (2026-10-16)

R8-2 (MAJOR) + R8-7 + R8-8 shipped in one batch, all
in `crates/ob-app/src/main.rs`. Checker, plan, and
landing now share one section judgment:
`heading_sections` computes every section standing
under the part's heading, and the checker
(`TargetPresence`), the plan (through the same
checker), and the landing split all search that same
set of sections.

- R8-2: `split_present_paragraphs_under_heading` no
  longer splits against `sections.first()` only. A
  paragraph standing under the part's heading in ANY
  of its sections is skipped (spending is shared
  across the sections, so a paragraph the part
  carries twice still needs two standing copies);
  the paragraphs still missing land in the FIRST
  section. The checker makes the same judgment over
  the same sections: the whole part is present when
  every one of its paragraphs stands under the
  heading in one of its sections, and no paragraph
  standing only outside them counts. Pins:
  `r8b_a_line_in_the_second_section_is_not_landed_again`
  (the review scenario verbatim — only the new line
  lands, into the first section; the standing line
  stands once; the claim reads AlreadyIn and a
  re-apply changes nothing),
  `r8b_an_empty_first_section_takes_the_missing_line`
  (the red team's empty-first-section shape),
  `r8b_the_whole_part_in_the_second_section_is_already_there`
  (the masked whole-in-second-section shape stays
  AlreadyThere, nothing lands), and
  `r8b_words_split_across_two_sections_of_one_heading_are_present`.
- R8-7: a bold-fallback paragraph delimits sections
  only when its normalized words are a heading the
  app is judging in this story — the target heading,
  or another part heading of the contribution being
  planned/applied (`judged_headings`, threaded
  through the checker, the split, the apply loop,
  and the plan simulation, whose per-part slices keep
  the whole contribution's context). Other bold
  paragraphs are content; heading-styled paragraphs
  always delimit, as before. Pins:
  `r8b_a_bold_emphasis_line_does_not_end_a_section`
  (the review scenario verbatim — present, nothing
  lands, nothing moves) and the converse,
  `r8b_a_bold_paragraph_carrying_another_parts_heading_still_delimits`
  (a bold "Youth" heading still starts the Youth
  section for a contribution carrying both parts;
  the R7-B pins exercise the same rule and pass).
- R8-8: (a) a heading-styled paragraph whose
  normalized words are the part's heading words plus
  a parenthesized suffix counts as that heading's
  section; a longer heading does not. Pins:
  `r8b_a_styled_heading_with_a_room_suffix_counts_as_its_section`
  and `r8b_a_room_suffix_section_takes_the_missing_line`
  ("Sunday school (Room 2)" matches, nothing
  duplicates, a missing line lands inside it), and
  `r8b_a_longer_heading_does_not_count_as_the_parts_heading`
  ("Sunday school teachers" does not match; the part
  lands its own heading as before). (b) One shared
  name-or-id recognition (`style_is_named` /
  `is_heading_style`) now serves both the heading
  writer (`heading_paragraph`) and the recognizer, so
  a subheading style whose id is not literally
  "subheading" is written and read back by the same
  rule. Pin:
  `r8b_a_subheading_style_with_a_non_literal_id_is_recognized`
  (hand-built style table with id "sub-2", name
  "Subheading": the writer writes in that id, the
  section is found, the part lands and settles).
- Supersession note: the R7-B addendum's scope note
  (round-7 synthesis) described heading recognition
  as "its style (heading / subheading) or the
  bold-body fallback", and its landing paragraph as
  splitting "against that section" (the one section
  the checker chose). Both are superseded by this
  batch, in the two respects above: the bold fallback
  delimits only for a judged heading, and checker
  and landing both judge all of the heading's
  sections. The round-7 text stands as history; this
  note is the current scope.
- Changed pre-existing tests: none. Every R7-B and
  R6-B presence pin passes unmodified, in its
  original expectation.
- Numbers: full workspace suite 604 passed /
  0 failed (baseline 594 / 0; +10 new tests, all in
  ob-app), cargo's own exit code 0, read unpiped.
  Clippy (`cargo clippy --workspace --all-targets`)
  clean, exit 0, no warnings. `cargo fmt --all` run
  before the final run. The fake-helper ETXTBSY flake
  did not fire.

## Shipped addendum, R8-C (2026-10-16)

R8-4 + R8-5 + R8-6 (MAJORs) and the carried
trailing-date shape shipped in one batch, in
`crates/ob-extract/src/contribution.rs`, with the
one field copy in `crates/ob-app/src/main.rs`'s
plan simulation.

- R8-4: the framed reading needs the weekday
  directly in front of the date; what follows the
  date is the line's remainder, not a parse failure.
  A shared prefix reader (`parse_date_prefix`: the
  longest opening run of words that reads as a date)
  serves the framed reader and the Sunday-after
  base, so a disagreeing framed date records its
  mismatch wherever the date stands in the line, an
  agreeing one yields its date by the same rules as
  the line-ending form, and the Sunday-after reader
  composes past trailing words — and now finds its
  shape wherever it stands in the line ("words for
  Sunday after October 25, 2026 please" resolves to
  November 1), each "Sunday after" candidate tried
  in order. The trailing weekday frames a date only
  when it stands directly after it, in both readers:
  words between the date and a trailing weekday word
  make that word part of the remainder, so no
  mismatch fires across prose. One consequence to
  state plainly: the review lane's round-8
  observation that a "…we honor Monday" tail "yields
  no record, because the suffix stops parsing"
  described the pre-fix mechanism; under this
  disposition's remainder rule that tail now reads
  its framed date (agreeing) with no mismatch
  recorded — the theft bound the observation was
  clearing (no trailing mismatch across prose)
  holds, and the R8-4 agreeing control ("use on
  Sunday, October 25, 2026 for the picnic" yields
  its date) is the same shape. Pins:
  `r8c_a_framed_date_reads_past_words_after_the_date`
  (RT8-3's E1/E3/E6 verbatim with Keep-through
  mirrors, the E2 agreeing control on both lines,
  and E5 on both lines; verdicts pinned where the
  red team probed them).
- R8-5: `framed_date_outcome` scans EVERY framing
  candidate in the line until one frames a date;
  a candidate whose following words do not parse as
  a date is not a framing and scanning continues —
  the round-8 review's D4 control ("on Monday we
  prayed, service on Sunday, October 25, 2026")
  still reads clean by exactly that rule and is
  pinned. Defense in depth shipped with it:
  `date_line`'s ordinary-text fallback never
  returns a date whose directly preceding weekday
  wears the framing garnish stack. Pin:
  `r8c_a_second_on_framing_is_judged_not_laundered`
  (RT8-4's F2 verbatim + Keep-through mirror + the
  D4 control on both lines).
- R8-6: the framing readers accept the
  Sunday-after reader's garnish stack in front of
  the weekday — optional "on", optional "the" —
  wherever the framing stands (`weekday_is_framed`,
  shared by the framed reader and the fallback
  guard; `named_weekday_mismatch` takes the same
  stack at the line's start). Pin:
  `r8c_on_the_weekday_frames_with_its_garnish_stack`
  (RT8-5's F4 verbatim + Keep-through mirror, the
  mid-line form, and the agreeing "on the Sunday"
  control).
- Carried shape: a trailing parenthesized absolute
  date on a relative Sunday-after or bare relative
  Sunday phrase is remembered on the Contribution
  as `for_sunday_parenthesized_date`, beside the
  in-words phrase (the phrase readers now let
  trailing parenthesized groups come off one by one
  until the shape stands alone, and the remembered
  phrase is the sender's words as written,
  parenthesis included). The verdict compares the
  remembered date with the relative resolution once
  an open bulletin anchors it: agreement reads as
  the relative verdict does today; disagreement
  returns `DifferentSunday` carrying the sender's
  date, so the review's differing-Sunday sentence
  names it beside the open bulletin's date and
  Accept arms two-click like any differing named
  Sunday. With no anchor the verdict stays
  NoSundayNamed and the review's in-words sentence
  carries the sender's phrase as now. A parenthesis
  that does not parse as a date at all is garnish,
  not memory. Pin:
  `r8c_a_parenthesized_date_on_a_relative_sunday_is_remembered_and_compared`
  (RT8-10's C1/C2/C3 verbatim — anchored agreeing,
  anchored disagreeing, no-anchor — plus a
  third-Sunday parenthesis, the weekday-garnish
  control, and an unreadable-date parenthesis).
- Changed pre-existing tests: none. Every R7-C
  family pin, the D3 ordinary-text control, and
  D5b pass unmodified, in their original
  expectations.
- Numbers: full workspace suite 608 passed /
  0 failed (baseline 604 / 0; +4 new tests, all in
  ob-extract), cargo's own exit code 0, read
  unpiped. Clippy (`cargo clippy --workspace
  --all-targets`) clean, exit 0, no warnings.
  `cargo fmt --all` run before the final run. The
  fake-helper ETXTBSY flake did not fire.
