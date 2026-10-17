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
