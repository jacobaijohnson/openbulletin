# Round 6 audit synthesis, 2026-10-16

Over HEAD `012ffdc` (round 5 fixes closed). Three blind
Wonder lanes: verify, review, red team. Reports:
`~/workspace/openbulletin-audit-wave3/round6-{verify,review,redteam}.md`.

**Verdict: 2 BLOCKER, 10 MAJOR, ~9 MINOR. The exit bar
(zero BLOCKER, zero MAJOR) is NOT met.** Round 7 follows
the fixes.

VERIFY: 28/28 round-5 claims VERIFIED, zero FAILED
(R5-A 7/7, R5-B 8/8, R5-C 6/6, R5-D 7/7). Full suite
532 passed, 0 failed. Booklet bytes exact (31,717 /
34,977). Music live through the real helper: 237
noteheads, staves 6.4-6.6 mm. The verify lane also
filed the probe note that became R6-1: the merge fence
is directional.

Process note: the Windows bundle from `012ffdc` (run
38993904354) completed successfully, gate included.

## The BLOCKERs (both in rights composition)

**R6-1 / RT6-1. The fence faces one way.** The R5-A
fence checks only the EXISTING record ("into a record
noting children with no permission noted, the
dangerous fills do not go"). Compose in the other
order, a credit-bearing record first and the
children-noted record second, and the children note
fills in by OR beside a permission no source record
ever claimed for a children picture. All three lanes
found it (verify probe note; review BLOCKER; red team
BLOCKER, end to end through load and the real
`export_html`: the picture publishes, pictures=1,
where the children-first order holds it back,
children_left_out=1). Every composition path inherits
it: load fold, sheet attach, add_asset, sidecar
absorb. Fix in R6-A: the fence becomes a property of
the COMPOSED record. After any merge, if the result
notes children and notes a permission, some single
source record must have carried both the children
note and that permission together; if none did, the
permission-satisfying fields that arrived from a
record without the children note do not stand
(credit_text reverts to the children-bearing side's,
a permission license likewise). All orders and
three-record folds are the test.

**RT6-2. The fence splits a license from its credit.**
Into a children-noted record, the fence blocks the
incoming credit line but lets its CC BY license
through (CC BY is not a permission license, so it was
never fenced). The composed record then claims a CC
BY license whose required attribution was discarded,
and fold-on-load saves that state onward permanently.
Fix in R6-A: a license and the credit line that
belongs to it travel together or not at all. When a
merge would take an incoming license but the fence
(or anything else) refuses that same record's credit
line, the license does not fill either. Applies to
every merge, fenced or not: no path may hold a
license while dropping the credit that came with it.

## The MAJORs

**Presence, as one system (R6-B).** Round 5 judged
presence doc-wide; four findings are that choice
surfacing in different rooms, and the fix is one:
presence is judged against where the part would
land.

- R6-2: a loose-sheet part's words never enter
  `doc.stories`, so a contribution carrying one
  reopens as RecordedButWordsGone forever, under a
  plan line in the same window saying the sheet is
  kept safe. A loose part's landing IS its filing:
  the claim must judge loose parts by the record and
  story parts by presence, and a loose-only
  contribution is AlreadyIn exactly when recorded.
- R6-3 + RT6-4: hymn presence matches the composed
  value anywhere in any story: an announcement
  mentioning ELW 500, or the template's own standing
  432 line, makes a music slip "already in" and the
  named line never gets its hymn. Presence for a
  hymn line is a hymn line: in the service story,
  carrying the composed value, and when the slip
  names a line, on a line with that name (or an
  appended Hymn line). Nothing outside the service
  story's hymn lines counts.
- RT6-4 (second half): an announcement assembled
  from standing fragments in three different stories
  reads AlreadyIn and can never be applied as a
  unit. With target-aware presence (the story the
  part would land in), scattered fragments no longer
  swear the part is in.
- RT6-3: Accept of a partly present part re-appends
  every paragraph, printing the present ones twice
  (the longer re-send and the one-word-changed
  re-send both demonstrated). Landing becomes
  per-paragraph: paragraphs already in the target
  story are skipped, the missing ones land, and a
  part whose paragraphs all end up in its target is
  complete for the record.
- Review MINOR folded in: the review's first line
  and the per-part plan lines can contradict (a
  RecordedButWordsGone headline over plan lines
  saying "already in"). One presence notion, used by
  the claim, the plan, and the apply alike, ends
  the contradiction by construction.

**Serving lines (R6-C).**

- RT6-5: `participant_value`'s rename bridge is
  positional; a rename plus a front insertion crosses
  built-in tokens ({Sunday.readers} printing another
  line's names). The bridge must stop guessing.
  Pairings are KNOWN at save time (R5-C computes
  them): record them. The profile gains an alias
  record (old slug to current slug, written when a
  pairing is made), the token lookup consults it,
  and the positional bridge is deleted. An in-place
  rename keeps its old token working through the
  alias (the R3-C pin), and a token that cannot
  resolve by slug or alias resolves empty, never to
  another line's names.
- R6-4: the slug guard's "line itself" test matches
  display names and ignores rename history, both
  ways: Ushers renamed to "Children's message" passes
  while the built-in lives as "Kids message" (the
  token then prints the ushers' names), and the
  renamed built-in itself is refused its own name
  back. Claimed-ness is a question about resolution,
  not spelling: a built-in's canonical slug is
  claimed by the line its token currently resolves
  to (by slug or by the new alias record), and only
  that line may take a name deriving it.
- R6-5 + RT6-6: the unfollowed machinery strands
  people. Two in-place renames plus one ADDED line
  read as ambiguous (nothing moved) because the
  pairing test compares raw lists; a line DELETED in
  the same save as a rename is reported "unfollowed"
  instead of removed, its names lingering in the map
  to reattach later; and the status sentence's cure
  ("rename one line at a time") cannot reach values
  stranded under a dead name. The rework: pairing
  anchors on the names present in both lists, so
  additions and removals elsewhere do not create
  ambiguity; a rename that still cannot pair is NOT
  TAKEN (the saved list keeps the old name, the
  values stay attached to a living line, and the
  status names the lines and the remedy, which now
  works); a name gone with no rename candidate is a
  removal, cleared and named as removed, in the same
  save as any renames.

**Dates and counts (R6-D).**

- R6-6 + RT6-8/9: date framing gaps. "on Monday,
  October 25, 2026" drops its weekday silently (no
  mismatch, no warning). "Sunday after October 25,
  2026 (Monday)", and even "(Sunday)", collapse to
  NoSundayNamed with no mismatch and no arm, on both
  lines; "the Sunday after next Sunday" likewise.
  Framing grows: an optional "on" before a framing
  weekday; the Sunday-after form composes with the
  trailing-weekday check (the weekday describes the
  date it follows, so an agreeing one resolves and a
  disagreeing one records the mismatch) and with a
  relative-Sunday base (the shape is remembered and
  the verdict resolves and arms it like any named
  Sunday).
- RT6-7: `raw_entry_count` trusts the central
  directory's declared size fields; lying fields
  (plain EOCD and zip64) let 9,001 and 200,001
  entries load. The count must be walked, not read:
  count the directory records themselves up to the
  end record's position, with the same junk/shift
  tolerance the zip reader itself applies (review
  MINOR: prepended and trailing junk), refusing
  past the cap as soon as the walk passes it.

## MINORs (all to R6-D unless noted)

- SundayInWords accepts never arm, and its sentence
  says "Sunday" even for a non-Sunday occasion: give
  it the two-click arm with its reading stated, and
  use the occasion's own word where the machinery
  has it.
- The wanted-font Check note names one substitute
  when matching styles resolve to different tiers;
  name what the worst style resolves to (the R5-D
  oracle already computes it).
- The profile save's "nothing else was changed"
  sentence overclaims in a multi-field edit pass;
  scope it to what it covers.
- The fake-helper ETXTBSY flake fired repeatedly
  through rounds 5 and 6 verification. Cure it in
  the test harness: each fake-helper test gets its
  own script file, so parallel tests stop racing on
  one path. (Test infrastructure only; the flake was
  never a product defect.)
- Identical line under two headings in one
  contribution: DISPOSITIONED by the R6-B design.
  Landing skip is target-scoped, so the second
  heading's copy lands in its own target; the claim
  stays per part against its target.

## Held and accepted (do not re-report in round 7)

Everything on the round-5 held list stands. The
R5-C swapped-pair residual stands as recorded, and
the review lane's extension is noted without
changing its disposition: the blind spot covers any
rename-only cycle, for the same reason (no
list-only signal separates a cycle from in-place
renames); the alias record in R6-C does not cure
it, because the pairing that would be recorded is
the ambiguous one. Its cure remains row identity in
the editor phase.

## Fix batches (one writer at a time, in order)

- **R6-A** rights composition integrity: R6-1, RT6-2.
- **R6-B** target-aware presence: R6-2, R6-3, RT6-3,
  RT6-4 (presence half).
- **R6-C** serving-line identity, second pass: RT6-5,
  R6-4, R6-5, RT6-6.
- **R6-D** dates, counts, small print: R6-6,
  RT6-8/9, RT6-7, the MINORs above, the fake-helper
  script cure.

Baseline entering the fixes: 532 tests, clippy clean.

## Shipped addendum, R6-A (2026-10-16)

Rights composition integrity, shipped over HEAD
`9ca9cd7`. `merge_rights_records` (ob-model) is still
the single funnel every composition path runs
through (load fold for .obd and .obdi, sheet attach,
`add_asset`, sidecar absorb), and the two BLOCKERs
are now properties of the composed record there, not
a fence on the existing record:

- R6-1 / RT6-1: after the fills, when the composed
  record notes children, each permission-satisfying
  field (credit line; ChurchOwned / PublicDomain /
  CC0 license) stands only when the record it came
  from itself notes children, or a children-bearing
  record already noted a permission on its own fields
  before the merge; otherwise the field reverts to
  the children-bearing record's value. Provenance is
  two booleans computed from the fill conditions
  before any fill lands (`credit_from_incoming`,
  `license_from_incoming`), so the check follows the
  supplying side, not file order, and three-record
  folds hold by composition.
- RT6-2: when a merge takes the incoming record's
  license but the composed record holds that license
  with an empty credit line while the incoming record
  carried a credit line that did not stand, the
  license does not fill either. A credit already
  standing on the existing record lets the license
  fill. Refused fills still count as no fill (the
  assertion pairing and the attach count read the
  final composed record).

FORMAT.md's merge wording was rewritten from the
directional fence to the composed property, license
and credit travelling together included.

Tests: 532 → 541 passing, 0 failed (full workspace,
unpiped, exit 0); clippy `--workspace --all-targets
-- -D warnings` exit 0. Nine new tests: ob-model 4
(either-order merge for credit, CC0, and ChurchOwned;
three-record fold in four orders; license-and-credit
together, including the refused pair counting as no
fill and the standing-credit case; `add_asset` in
both orders), ob-format 2 (the credit-first pair
folding on load for .obd and for .obdi, both orders),
ob-extract 1 (direct merge in both orders for credit
and CC0 through the real `export_html`: picture held
back), ob-app 2 (.obd load fold plus the real export
holding the picture back in both orders; sheet
attach with the credit on the bulletin side and the
children note on the sheet side holding the picture
back). The fake-helper ETXTBSY flake did not fire.

One pre-existing test changed, flagged by name:
`r5_duplicate_records_fold_in_a_sheet_file_too`
(ob-format). Its scenario is a ChurchOwned record
(no children note) standing first and a
children-noted record second, and it pinned the
ChurchOwned license standing in the folded record —
exactly the manufactured permission RT6-1
demonstrated, since no single record carried the
license together with the children note. Its license
assertion changed from ChurchOwned to Unknown; its
other assertions (one folded record, source, title,
children note) stand unchanged, and the test carries
a comment saying so. No other pre-existing test was
changed.

## Shipped addendum, R6-B (2026-10-16)

Target-aware presence, shipped over HEAD `c32d5bb`.
Presence is now judged per part against the story
that part would land in, and the claim, the plan, and
the apply all use that same judgment:

- `plan_part` decides the part's planned target first
  (`planned_part_plan`), then judges presence against
  it. Story target: the part's paragraphs are searched
  within that story only. AnnouncementsUnderHeading:
  within the weekly story. KeptWithPart: never
  present, unless the part carries no words.
  LooseSheet: never present by words, and never plans
  AlreadyThere. A service part aimed at the service
  story judges its hymn-valued paragraphs by the
  hymn-line rule and its other paragraphs within the
  service story.
- Hymn presence is a hymn line (R6-3 / RT6-4): a
  hymn-valued paragraph is present only when a hymn
  line in the service story carries its composed
  value — on a service line with the slip's named
  label (case-insensitive), or on an appended line
  labelled "Hymn", when the slip names a line; on any
  hymn-labelled service line when it names none. An
  announcement mentioning ELW 500, and the template's
  own Hymn of Praise line carrying 432 under a
  different label than the slip names, no longer make
  a slip present.
- The claim (R6-2): `applied_claim` judges
  story-landing parts by target-scoped presence and
  loose parts by the record. AlreadyIn is every story
  part present in its target AND (no loose parts OR
  recorded); RecordedButWordsGone is recorded with
  some story part absent from its target; a loose-only
  contribution is AlreadyIn exactly when recorded and
  NotApplied when not. The claim and the per-part plan
  lines now use the one presence notion, so the
  review's first line and its plan lines cannot
  contradict each other.
- Landing is per paragraph (RT6-3): the Story and
  AnnouncementsUnderHeading arms skip the paragraphs
  already present in the target story (scoped, spent
  one by one) and land only the missing ones; skipped
  paragraphs count toward the existing `already`
  summary. The under-heading arm adds its heading
  only when at least one paragraph actually lands,
  and keep-through lands only when at least one
  paragraph landed. A part is complete for the record
  when every paragraph ended up in its target, landed
  or already there, and its target story exists. In
  the hymn path, an AppendHymn whose composed value
  already sits on a hymn line in the service story
  (the hymn-line judgment for that paragraph) counts
  as already there instead of appending a duplicate
  line, a plain line already in the service story is
  skipped the same way, and Replace stays as it was.

FORMAT.md is unchanged: the record's rules did not
move; the claim's evidence is what moved.

Tests: 541 → 550 passing, 0 failed (full workspace,
unpiped, `--no-fail-fast`, exit 0); clippy
`--workspace --all-targets -- -D warnings` exit 0.
Nine new tests, all in ob-app: a hymn mention in the
announcements does not make a slip present and the
slip lands on its named line; a hymn value carried
under a different label is not present on that line;
fragments scattered across three other stories leave
an announcement NotApplied and it applies as a unit
into its target; the longer re-send and the
one-word-changed re-send each land only the missing
or changed paragraph (paragraph counts exact, no
duplicates); a loose-sheet part is judged by the
record (mixed contribution AlreadyIn once recorded,
RecordedButWordsGone when the story part's words are
deleted, the loose part never planning AlreadyThere);
a loose-only contribution is NotApplied before any
record and AlreadyIn once recorded; an under-heading
part lands only its missing paragraph, heading
included once; an appended hymn already on a hymn
line is not appended twice.

No pre-existing test was changed. The two R5-B tests
whose fixtures touch the old doc-wide haystack keep
their original assertions because their truths are
unchanged under scoping: `r5_one_shared_line_does_not_make_a_part_already_in`
(the shared line stands in the part's own target, the
weekly story, and the rest is missing either way) and
`r5_a_hymn_line_is_present_by_its_composed_form` (the
slip lands on an appended line labelled "Hymn", which
the hymn-line rule counts). The fake-helper ETXTBSY
flake did not fire.
