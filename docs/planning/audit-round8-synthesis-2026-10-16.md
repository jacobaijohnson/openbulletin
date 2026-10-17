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
