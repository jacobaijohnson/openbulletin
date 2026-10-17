# Round 7 audit — synthesis (2026-10-16)

Three blind Wonder lanes over HEAD `9ed543f` (round 6
closed: R6-A `c32d5bb`, R6-B `9715b75`, R6-C `4b04aaf`,
R6-D `9ed543f`). Reports:
`~/workspace/openbulletin-audit-wave3/round7-verify.md`,
`round7-review.md`, `round7-redteam.md`.

**Verdict: 1 BLOCKER, 5 MAJOR, 5 MINOR (deduplicated
across lanes). Exit bar NOT met.** The bars were verified
where they were claimed — the publish fence held against
every ordering the red team threw at it, serving lines
held in full, the walked count is exact at the cap
boundary — and every failure below is a composition the
round-6 fixes did not compose: a rule applied in one
direction, a garnish that does not compose with the shape
it garnishes, one presence notion spent at two scopes.

Lane raw counts: verify 33 VERIFIED / 2 FAILED / 0
UNTESTABLE of 35 round-6 claims (both FAILED are the one
presence root, R7-2 below); review 0 BLOCKER / 3 MAJOR /
3 MINOR; red team 1 BLOCKER / 5 MAJOR / 2 MINOR.
Deduplication: the presence root was found independently
by all three lanes (verify B3+S1, review M1+M2, red team
RT7-2) and is counted once; review M3 and red team RT7-6
(+ RT7-7's D9 shape) are one date family, counted once;
the remaining findings are single-lane.

Held, on the record: the publish fence (no manufactured
permission in any 2-, 3-, or 4-record fold order, through
load and a real `export_html`; children-noted pictures
held back throughout); serving lines in full (alias
cycles, self-alias, dangling alias, multi-save rename
chains, removal + exact-name reuse, hijack and
same-segment refusals); applied-record forgery (a forged
fingerprint never manufactures AlreadyIn for story
parts); the walked count at the 8,192/8,193 boundary on
both loaders; booklet baseline exact (31,717 / 34,977);
live Bach test (237 noteheads, staff 6.4–6.6 mm);
566/0 in every lane, clippy clean, no ETXTBSY flake
anywhere.

## Findings and dispositions

### R7-1 (BLOCKER, red team RT7-1) — license/credit togetherness faces one way

R6-A's travel-together rule fires only when the license
arrives from the incoming record (`license_from_incoming`).
Mirror composition: a CC BY (or CC BY-SA) license already
standing on the children-bearing record, the only credit
arriving from a second record and refused by the children
fence → the composed record keeps the license with an
empty credit, byte-identical to the state RT6-2 was
graded BLOCKER for. Reproduced through the bare merge,
the three-record fold, `add_asset`, sheet attach (same
call shape), and the real load fold. The picture does not
publish (the fence holds); the harm is the record: the
fold is destructive and permanent, the source credit
exists nowhere else, and every copy saved onward claims
a CC BY license whose required attribution was
discarded, against the R6-A addendum's own sentence ("a
license and its credit travel together or not at all…
applies to every merge, fenced or not").

Adjudication against the review lane's cleared item: the
review lane read the same function and cleared
*manufactured permission* and transitivity; RT7-1 is a
different property (credit survival beside a standing
license), demonstrated with runnable probes through
every merge path. Demonstrated behavior governs. R7-1
stands at BLOCKER.

Disposition (R7-A): the rule judges the composed record,
not the direction. After all fills and reversions, if
the composed record notes children, carries an
attribution-requiring license (CC BY / CC BY-SA), and
its credit line is empty, the license does not stand
(it reverts to what the children-bearing side holds
without it — Unknown where nothing else vouches). A
children-bearing record whose OWN credit stands keeps
its license (the R5b control shape). Pins: both
directions, CC BY-SA, three-record fold, `add_asset`,
load fold, and the own-credit control.

### R7-2 (MAJOR, all three lanes) — one presence notion, two spending scopes

The claim's checker (`contribution_words_present`)
shares one spent set across ALL parts of a contribution;
the plan and the apply judge each part with a fresh
checker against the document the earlier parts landed
in. A line or paragraph repeated across two parts splits
the notions permanently, in two faces:

- **Never settles (red team RT7-2, review M2):** two
  `## Announcement` parts carrying the same paragraph.
  Accept lands it once, records the contribution,
  `all_landed=true`; on re-review both plan lines say
  AlreadyThere while the headline reads
  RecordedButWordsGone, forever. Re-accept changes
  nothing; the contribution never reaches AlreadyIn.
- **A heading silently lost (verify B3+S1, review
  M1):** the same line under two headings (probed:
  `## Sunday school` + `## Youth`, and
  `## Announcement` + `## Sunday school`) sharing the
  weekly-story target. Part 2's plan says AlreadyThere
  (the plan simulation landed part 1 first), its
  under-heading landing never runs, and the second
  heading never appears in the bulletin — silently,
  and (verify's shape) under the same never-settling
  headline. This is the R6-12 shape the round-6
  synthesis dispositioned as covered by the R6-B
  design; that disposition holds only for distinct
  named-story targets and is **withdrawn** for the
  shared-target shape.

Disposition (R7-B): claim, plan, and apply use literally
one presence notion, per part, heading-aware. A part is
present when its words stand in its target the way a
complete landing would leave them: for an
under-heading part, standing under its own heading (the
same words elsewhere in the story are not its
presence); for repeated parts sharing one target, one
standing copy placed as the landing places it satisfies
each part that asked for exactly that placement. The
landing places an under-heading part's words under its
own heading even when the same words already stand
elsewhere in the story, and counts the part landed only
when heading and words both stand. Pins: all four probe
shapes (two announcements repeated; line under two
headings; heading part whose line stands elsewhere in
the story; the one-part-twice and hymn controls that
already settle must keep settling).

### R7-3 (MAJOR, red team RT7-3) — "on" framing only counts at the start of the line

"For Sunday: use on Monday, October 25, 2026" lands the
date clean — no mismatch, SameSunday unarmed against
the October 25 bulletin — although Monday, October 25
does not exist. R6-6 established "on" + weekday as
date framing; the shipped handling only recognizes it
at the line's start, and the mid-sentence rule then
reads the weekday as ordinary text.

Disposition (R7-C): a weekday directly in front of a
date, wearing "on", is framing wherever it stands in
the line, and records its mismatch like the leading
form. The R5-D ordinary-text rule stands where it was
documented — a weekday with NO "on" belonging to the
sentence's other words ("bring a friend Friday,
October 25, 2026") still reads clean. Red team's D5b
("please note Monday, …", no "on") is dispositioned as
standing under that rule and recorded for round 8 to
re-probe.

### R7-4 (MAJOR, red team RT7-4) — a failed Sunday-after base salvages the base date itself

"For Sunday: Sunday after Tuesday, October 26, 2026"
(October 26 is a Monday) yields `for_date =
2026-10-26`, no mismatch: the base's wrong weekday
refuses the Sunday-after reading, and the mid-sentence
rule then reads the base date bare and returns IT as
the line's date. The sender's named Sunday (November
1) is lost, the wrong weekday is hidden, and the same
shape on Keep-through silently ends the announcement
on the base date. The agreeing-weekday control resolves
correctly — the salvage fires exactly when the
machinery should speak.

Disposition (R7-C): a date inside a "Sunday after …"
phrase is the base, never the line's own date. When a
Sunday-after reading was attempted and its base failed
on a weekday mismatch, the line records that mismatch
and yields no date, on both lines. No salvage path may
resurrect a base date out of a failed Sunday-after
parse.

### R7-5 (MAJOR, review M3 + red team RT7-6, RT7-7/D9) — the garnishes do not compose with the Sunday-after shapes

- "on Sunday after October 25, 2026" and "on the
  Sunday after October 25, 2026" → NoSundayNamed,
  unarmed, under the sentence "These words do not say
  which Sunday they are for" — false; the date is
  fully determined (November 1, 2026). (Review M3,
  graded MAJOR: the false sentence plus the unarmed
  accept is the wrong-week failure the arm exists to
  prevent.)
- "the Sunday after next Sunday (Monday)" and "next
  Sunday (Monday)" → the trailing weekday garnish
  breaks the exact-shape relative matches; silent
  NoSundayNamed. (RT7-6.)
- "on Sunday after October 25, 2026" also appears in
  RT7-7's combined-shapes list; counted here, once.

Disposition (R7-C): the Sunday-after reader composes
its pieces in order — optional leading "on" and/or
"the", the Sunday-after shape (absolute or relative
base), optional trailing weekday composed against the
base per RT6-8 (agreeing resolves, disagreeing records
the mismatch). A fully determined date resolves and
arms like any named Sunday; no composed shape falls
through to a silent NoSundayNamed.

### R7-6 (MAJOR, red team RT7-5) — a fake directory in trailing junk defeats the entry cap

A 9,001-entry archive plus appended trailing junk
holding a fake 2-record central directory and a fake
end record loads as an ordinary bulletin on both
loaders: the R6-D walk locates the end record the way
the shipped reader does (latest candidate first), so
both count and read the fake directory. The real
directory's cost does not materialize in this shape
(neither reader walks it), so this is MAJOR like
RT6-7, not BLOCKER: what is defeated is the cap
itself, and the R6-D claim that the count is the
archive's raw entry count.

Disposition (R7-D): the cap judges the file, not the
directory the reader happens to use. The count check
examines every end-record candidate the file presents
(latest first, as now, plus any earlier candidates
trailing structures introduce), counts each candidate
directory with the same cap early-stop, and refuses
if ANY candidate directory exceeds the cap. The loader
still reads by the format's latest-directory semantics;
honest files with trailing junk but no second directory
load exactly as before, and every round-6 held shape
(signature-in-comment, EOCD-in-comment, truncated
directory, 17 fake end records) must stay held. Pins:
the f1a/f1b fixtures on both loaders, plus a
trailing-junk file whose only directory is honest and
small (loads).

### R7-7 (MINOR, red team RT7-7 remainder) — combined leading + trailing weekday records nothing

"on Monday, October 25, 2026 (Sunday)" (and the
agreeing-leading / disagreeing-trailing mirror, and
the Keep-through shape) yield no date and no mismatch:
each single shape records its mismatch, the combination
records neither. Nothing lands in a wrong week; only
the explanation is lost.

Disposition (R7-C): when both framings are present,
each is checked; a disagreeing framing records its
mismatch even when no date results.

### R7-8 (MINOR, red team RT7-8; round-6 residual RT6-12, unchanged) — the assertion rides an unrelated fill

`asserted_by/at` stand whenever ANY fill from the
incoming record stands — including a merge whose only
standing fills are unrelated (title, source_url) while
the same record's credit was refused by the fence. The
assertion then reads as vouching for a composed record
its author never saw. The control (incoming carrying
only the refused credit plus the assertion) is held, as
R5-A pinned.

Disposition (R7-A, same function as R7-1): the
assertion stands only when a substantive rights fill
from the same incoming record stands (a credit or a
license that itself stands in the composed record).

### R7-9 (MINOR, review m1) — the slug-conflict sentence can name one line as both sides

When the claimant renames back to its built-in name
inside an edit that is also ambiguous, the pairing's
discarded pairs lose the conflict check's self-excuse,
and the office reads "'Children's message' would take
the place of the built-in line 'Children's message'…".
The save is refused either way; the sentence is
nonsense and names the wrong blocker.

Disposition (R7-D): the claimant must never be
reported as conflicting with itself — the self-excuse
survives an ambiguous pairing (the checker consults
the rename intent the pairing computed before
discarding, or an equivalent record). The refusal
itself, and the ambiguous-refusal sentence that then
applies, stand.

### R7-10 (MINOR, review m3) — the ambiguous-refusal sentence claims a move that may not have happened

"several lines were renamed and moved at once" also
fires for rename-plus-removal refusals where nothing
moved (old [Choir, Bells] → [Choir team] between two
anchors). The refusal is right; its story is wrong.

Disposition (R7-D): the sentence's account matches
the edit classes the pairing actually saw (renamed /
removed / moved), without weakening the refusal or
the remedy. The R6-C pinned sentence is a changed pin
when this lands — flag it in the batch addendum.

### R7-11 (MINOR, review m2) — FORMAT.md still carries the scoped-out overclaim

`docs/FORMAT.md:160` says a slug refusal leaves
"nothing else changed" — the claim R6-D removed from
the status sentence because other profile fields save
during the same edit pass.

Disposition (R7-D, docs): scope the sentence to the
serving lines, matching the shipped status words.

## Fix batches

- **R7-A** — rights, second direction: R7-1 (BLOCKER)
  + R7-8. `merge_rights_records` and every path
  through it.
- **R7-B** — presence, one notion: R7-2, both faces.
- **R7-C** — date composition: R7-3, R7-4, R7-5,
  R7-7.
- **R7-D** — counts and small print: R7-6, R7-9,
  R7-10, R7-11.

Each batch: single writer, scar rules, dated Shipped
addendum to this synthesis, coordinator re-runs the
full suite + clippy unpiped before accepting. Round 8
(verify / review / red team over the new HEAD) follows
R7-D. The Tier-2 final gate runs only after a Wonder
round returns zero BLOCKER + zero MAJOR.

## Shipped addendum, R7-A (2026-10-16)

Rights, second direction, shipped over HEAD
`f40a905`. Both findings are fixed inside
`merge_rights_records` (ob-model), the single funnel
every composition path runs through (load fold for
.obd and .obdi, sheet attach, `add_asset`, sidecar
absorb), so every path inherits both fixes:

- R7-1 (BLOCKER): the togetherness rule now judges
  the composed record, not the direction. After all
  fills and reversions, if the composed record notes
  children, carries an attribution-requiring license
  (CC BY / CC BY-SA), and its credit line is empty,
  the license does not stand — whichever side it
  came from. It reverts to what the merge holds
  without it: the other record's license when that
  license itself stands under the publish fence and
  this same rule (a children-bearing record's CC0
  vouched beside a dropped CC BY is the shape), and
  Unknown where nothing else vouches a license. A
  children-bearing record whose own credit stands
  keeps its license, and licenses requiring no
  attribution (CC0, public domain, the church's own
  work) are untouched by the rule.
- R7-8 (MINOR): `asserted_by` / `asserted_at` now
  stand only when a substantive rights fill from the
  same incoming record itself stands in the final
  composed record — its credit line, or its license.
  The old gate (any change to the composed record)
  let an assertion ride in on an unrelated title or
  source_url fill beside a credit the fence refused
  in the same merge; that ride is closed. The R5-A
  control (incoming carrying only a refused credit
  plus the assertion changes nothing) stays held,
  and an assertion still stands when the same
  record's credit or license genuinely stands.

FORMAT.md's merge wording was updated from the
one-direction travel-together sentence to the
composed rule, mirror case included.

Tests: 566 → 573 passing, 0 failed (full workspace,
unpiped, `--no-fail-fast`, all 28 suite results ok,
exit 0); clippy `--workspace --all-targets --
-D warnings` exit 0. Seven new tests: ob-model 5 (the
RT7-1 probe verbatim for CC BY and CC BY-SA,
asserting the license falls and the children note
stands, with the own-credit control and a
no-attribution-license control in the same test; the
fallback shape — a children-bearing record's vouched
CC0 standing where the dropped CC BY stood; the
three-record fold [children+CC BY, title, credit]
yielding license Unknown, credit "", title standing;
`add_asset` in the mirror order; the R7-8 assertion
shapes — title-only and source_url-only fills beside
a refused credit carry no assertion, the
refused-credit-only control changes nothing, and the
assertion stands when the same record's credit or
license stands), ob-format 1 (the mirror pair folding
on a real .obd load), ob-app 1 (sheet attach in the
mirror order: license falls, credit refused, children
note and source stand). The fake-helper ETXTBSY
flake did not fire.

Pre-existing tests changed — four, each flagged by
name, none buried:

- `r5_merge_never_manufactures_a_web_permission`
  (ob-model) — the `sheet_fuller` pin (a CC BY
  license filling into a children-noted record with
  no credit anywhere) asserted the license standing.
  That composed record is the R7-1 BLOCKER state, so
  its license assertion changed from CcBy("4.0") to
  Unknown; the title fill beside it still lands, and
  every other assertion in the test is unchanged.
- `r5_duplicate_asset_records_fold_on_load`
  (ob-format) and
  `r5_duplicate_records_in_a_file_fold_and_hold_the_picture_back`
  (ob-app) — both pin the same fold: a weak record
  and a record noting children with a CC BY license
  and no credit line, in either order. Both asserted
  the folded license standing; that is again the
  R7-1 BLOCKER state, so both license assertions
  changed from CcBy("4.0") to Unknown. Every other
  assertion in both tests stands unchanged (one
  folded record, children note, title, creator,
  source, and — in the ob-app test — the picture
  still held back from the web page).
- `r5_merge_assertion_fields_ride_only_with_a_real_fill`
  (ob-model) — its substantive fill was a title,
  which R7-8's redefinition (substantive means
  rights-bearing) removes as an assertion carrier.
  The fill in the test is now a credit line; the
  test's assertions (the assertion stands with a
  substantive fill from the same record, and an
  assertion alone changes nothing) are unchanged in
  substance, and the title-only shape is pinned
  directly by the new R7-8 test above.

No other pre-existing test was changed. One
deviation from the brief's expectation, stated
plainly: the brief expected few or no changed pins;
four changed, because three shipped pins asserted
the exact composed state RT7-1 demonstrated as the
BLOCKER and the fourth asserted the title-carries-
assertion reading R7-8 closes. Each carries a
FLAGGED comment in its own source saying so.

## Shipped addendum, R7-B (2026-10-16)

Presence, one notion, shipped over HEAD `610effa`.
The claim, the plan, and the apply now use literally
one presence notion, judged per part, heading-aware:

- The claim (`contribution_words_present`) no longer
  shares one spent-set checker across a contribution's
  parts. It judges every story-landing part with
  `part_words_present` itself — the very judgment the
  plan uses — each part with a fresh checker, loose
  parts still skipped and judged by the record alone.
  One standing copy placed as a landing places it
  satisfies every part that asked for exactly that
  placement, so two `## Announcement` parts carrying
  the same paragraph settle: apply lands the words
  once, records the contribution, and reopening reads
  AlreadyIn with every plan line AlreadyThere
  (RT7-2 / review M2). Within one part the checker
  still spends the story's paragraphs one by one, so
  the controls keep settling: the same paragraph
  twice in one part lands twice, and the same hymn
  line twice in one service part settles on its hymn
  lines (hymn presence itself is untouched: a hymn
  line in the service story).
- An under-heading part is present only under its own
  heading. `heading_sections` finds the sections of
  the weekly story standing under a heading paragraph
  (a heading-styled paragraph, or the bold body
  paragraph the heading fallback writes): the range
  after the heading and before the next heading of
  any words. The checker searches one section — the
  first that holds the part's whole words — and a
  part whose heading stands nowhere in the story is
  not present however its words stand elsewhere in
  it. The landing matches: `split_present_paragraphs_under_heading`
  splits against that section, so paragraphs already
  under the part's own heading are skipped (no
  double-print in that scope, the R6-B pin kept where
  it belongs) and the missing ones land inside the
  section, or, when the heading stands nowhere, under
  a heading the apply adds — even when the same words
  already stand elsewhere in the story (verify B3+S1,
  review M1, in both the two-heading and the
  single-part forms). The under-heading arm counts
  the part landed only when the one presence notion,
  re-run after the landing, finds its heading and its
  words both standing.
- Headline and plan can no longer disagree, because
  both read the same per-part judgment; words removed
  by hand after a recorded apply still read
  RecordedButWordsGone under the pinned sentence, and
  re-applying lands them once and settles.

FORMAT.md is unchanged: its applied-record wording
(the record is a note; the words themselves are
checked before the review says a part is already in)
stays true as written.

Tests: 573 → 581 passing, 0 failed (full workspace,
unpiped, `--no-fail-fast`, all 28 suite results ok,
exit 0); clippy `--workspace --all-targets --
-D warnings` exit 0. Eight new tests, all in ob-app:
RT7-2 verbatim (two Announcement parts, same
paragraph → apply complete → AlreadyIn, plan lines
agree, words stand once, re-accept changes nothing);
review M2's probe shape (a two-paragraph part and a
second part carrying its first line alone); verify
S1 verbatim (the same line under ## Sunday school +
## Youth → both headings land, each with the line
under it, reopen AlreadyIn); review M1 verbatim
(## Announcement + ## Sunday school sharing a line →
the Sunday school heading lands with its line); a
line already under its OWN heading is skipped, not
printed twice, and the missing paragraph lands
inside the section; the two controls (same paragraph
twice in one part; same hymn line twice in one
service part, no third line on re-apply); and words
removed by hand after the shared-paragraph apply
reading RecordedButWordsGone, then landing once more
and settling. The fake-helper ETXTBSY flake did not
fire.

Pre-existing tests changed — one, flagged by name,
none buried:

- `r6b_an_under_heading_part_lands_only_its_missing_paragraphs`
  (ob-app) — RENAMED to
  `r7b_an_under_heading_part_lands_its_words_under_its_own_heading`,
  disposition changed. It pinned the round-6
  whole-story scope: a Sunday school part whose first
  line already stood anywhere in the weekly story
  skipped that line, landed only the missing one,
  and counted one already-there line. That scope is
  the R7-2 root (it is what silently lost the second
  heading in M1/S1), so the same fixture now pins
  the heading-aware reading: no Sunday school heading
  stands, so the heading lands and both paragraphs
  land under it — the line now stands twice (once
  where it was, once under its own heading), the
  summary reports 2 added lines and no already-there
  line, and the claim reads AlreadyIn. The test
  carries a FLAGGED comment in its own source saying
  so. No other pre-existing test was changed: every
  other R6-B pin (per-paragraph landing for story
  parts, the hymn-line rule, loose-by-record, the
  scattered-fragments and re-send shapes) holds under
  the unified notion exactly as written.

One scope note, stated plainly: a heading is
recognized by its style (heading / subheading) or by
the bold-body fallback `heading_paragraph` writes. A
heading typed by hand as plain body text is not
recognized as one, so an under-heading part for it
lands its heading and words again under a real
heading paragraph; the failure direction is a
repeated placement, never a lost heading.

## Shipped addendum, R7-C (2026-10-16)

Date composition, shipped over HEAD `fe01627`. All
four findings are fixed inside
`crates/ob-extract/src/contribution.rs`, the one
parser both date lines share (`date_line`), so the
For Sunday and Keep-through lines inherit every fix
together; the in-words memory stays a For Sunday
matter, as shipped.

- R7-3 (MAJOR): a new composed reading,
  `framed_date_outcome`, runs before the plainer
  readings in `date_line`. A weekday wearing "on"
  directly in front of a date frames that date
  wherever it stands in the line: "use on Monday,
  October 25, 2026" now records the Monday mismatch
  and yields no date (the unarmed SameSunday against
  the October 25 bulletin is gone), and an agreeing
  mid-sentence "on" weekday reads clean. The R5-D
  rule stands untouched where there is no "on": "bring
  a friend Friday, October 25, 2026" still reads
  clean, and red team's D5b ("please note Monday,
  October 25, 2026", no "on") is unchanged — it reads
  the date clean with no mismatch, pinned as standing
  in the new test for round 8 to re-probe, exactly as
  the disposition recorded it.
- R7-4 (MAJOR): `sunday_after_outcome` now composes
  its base's pieces instead of giving up on them:
  after the whole-base reading fails it takes an
  optional trailing weekday out, reads the core, then
  strips the base's own leading weekday (and a filler
  "on" in front of it) and reads the date that
  remains. A base weekday that disagrees records the
  mismatch against the base date and yields NO date
  — "Sunday after Tuesday, October 26, 2026" records
  Tuesday/October 26 on both lines — and because
  `date_line` returns that record before any other
  reading runs, no salvage path can resurrect the
  base date as the line's own date. The agreeing
  control ("Sunday after Monday, October 26, 2026"
  → November 1) keeps resolving.
- R7-5 (MAJOR): the Sunday-after reader strips its
  leading garnishes in a loop — an optional "on"
  and/or "the", in either order — so "on Sunday
  after October 25, 2026" and "on the Sunday after
  October 25, 2026" resolve to November 1, 2026 and
  arm like any named Sunday, on both lines, and the
  trailing weekday still composes against the base
  under the garnishes (agreeing resolves, disagreeing
  records the mismatch against the base). The
  relative readers (`sunday_after_relative_phrase`,
  `relative_sunday_phrase`) compose the same way:
  leading "on"/"the" garnishes, a "the" worn by the
  relative base itself, and an optional trailing
  weekday stripped for the shape match but kept in
  the sender's remembered words — "the Sunday after
  next Sunday (Monday)" is remembered in
  `for_sunday_after_in_words` with the garnish
  attached and resolves DifferentSunday against the
  open bulletin, and "next Sunday (Monday)" is
  remembered in `for_sunday_in_words` and reads
  SundayInWords, exactly as the ungarnished shapes
  behave. No composed shape falls through to a
  silent NoSundayNamed.
- R7-7 (MINOR): the same `framed_date_outcome`
  checks each framing when a leading and a trailing
  weekday frame one date: the disagreeing one —
  leading ("on Monday, October 25, 2026 (Sunday)") or
  trailing ("on Sunday, October 25, 2026 (Monday)") —
  records its mismatch and no date results, on both
  lines; both agreeing reads the date clean.

Tests: 581 → 587 passing, 0 failed (full workspace,
unpiped, `--no-fail-fast`, all 28 suite results ok,
exit 0, re-run on the final code after the fmt and
clippy passes); clippy `--workspace --all-targets --
-D warnings` exit 0. Six new tests, all in
ob-extract's contribution module, each probe verbatim
with parse-level fields and `week_verdict` where the
red team probed the verdict: the mid-sentence "on"
mismatch on both lines plus its agreeing form and
the D5b standing pin (R7-3); the failed Sunday-after
base on both lines, plain and "the"-led (R7-4); the
composed "on"/"on the" Sunday-after shapes resolving
and arming to November 1, Keep-through included,
plus the trailing weekday under the garnishes
(R7-5); the relative shapes with garnish attached —
the trailing weekday, a leading "on the", a "the"
before the relative base — remembered and resolved
(R7-5); the combined leading + trailing shapes, both
mirrors and Keep-through, plus the both-agreeing
control (R7-7); and a controls test pinning the bare
"Sunday after October 25, 2026", the agreeing-base
"Sunday after Monday, October 26, 2026", and "bring
a friend Friday, ..." as still standing (the
ungarnished relative controls stand in the
pre-existing RT6-9 / RT5-6 tests, unchanged and
green). The fake-helper ETXTBSY flake did not fire.

Pre-existing tests changed — none. Every round-3
through round-6 date pin (leading "on", trailing
weekday, Sunday-after trailing composition, relative
shapes, ordinary text) holds exactly as written
under the composed readings.

One scope note, stated plainly: review M3's probe
list also carried "the Sunday after next Sunday
(October 25)" — a trailing absolute date in
parentheses, not a weekday garnish. That shape is
not among R7-5's dispositioned shapes, and it still
reads as naming no Sunday; the trailing garnish this
batch composes is the weekday, as RT6-8 established
and R7-5 dispositioned.
