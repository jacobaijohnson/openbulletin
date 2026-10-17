# Round 5 audit synthesis, 2026-10-16

Over HEAD `4a8a07f` (round 4 closed). Three blind Wonder lanes:
verify, review, red team. Reports:
`~/workspace/openbulletin-audit-wave3/round5-{verify,review,redteam}.md`.

**Verdict: 1 BLOCKER, 6 MAJOR, ~9 MINOR. The exit bar (zero
BLOCKER, zero MAJOR) is NOT met.** Round 6 follows the fixes.

VERIFY: 36/36 round-4 claims VERIFIED, zero FAILED (R4-A 8/8,
R4-B 5/5, R4-C 5/5, R4-D 15/15, measurements 3/3). Full suite
re-run unpiped: 506 passed, 0 failed. Booklet bytes reproduce
the round-4 re-baseline exactly (imposed 31,717, reader
34,977). Music ran live through the real Verovio helper: 237
noteheads, 14 measures, 13 fermatas, staves 6.4-6.6 mm.

One process note: the Windows bundle from `4a8a07f` (run
38987297145) completed successfully, gate included.

## The BLOCKER

**RT5-1 (red team). Duplicate asset records for one hash.**
`document.json` may carry two records for the same asset
hash; every reader takes the FIRST (`.find(|a| a.sha256 ==)`,
confirmed at main.rs:1259, 10245, 16028, 19740 and in
`ob-extract/src/html_export.rs`). A weak first record silently
disarms a `children_present` Some(true) note (the photo
publishes in the web export) and drops a CC BY credit; the
lane demonstrated the order flip through the real
`export_html`. No app path creates duplicates, but the loader
accepts them silently, and `.obd` files travel between
machines and tools. Fix in R5-A: fold duplicate records on
load with the R4-5 merge, in both loaders; first-wins ends.

## The MAJORs

**F1 (review) + RT5-3 (red team). The applied record can lie.**
The contribution record says "already in the bulletin" in two
situations where it is not true: (a) a part that landed partly
or not at all (no announcements box, missing story) is still
recorded fully applied at apply time (main.rs:11333), so on
the failed-save path R4-4 exists for, the reopened review
swears the words are in and offers only File it, and the words
can never print; (b) after an apply, a failed save, and a
manual delete of the words, the record still blocks Accept.
Fix in R5-B: record only parts that landed completely, name
incomplete landings in plain words, and let the review claim
"already in" only when the record AND the part's words are
both present; when the record says applied but the words are
gone, Accept is offered again with an honest sentence. The
red team's plantability footnote (the fingerprint is 8 hex
chars of a public hash) is adjudicated NOT a threat-model
change: anyone who can edit the `.obd` can do anything to it;
the fix is truthfulness, not a longer hash.

**F2 (review). The rights merge can manufacture a web
permission.** `permission_noted` (html_export.rs:738) counts
a bare credit line, or a ChurchOwned / PublicDomain / CC0
license, as the noted permission that lets a picture with
children noted in it ride the web page. Under winner-take-all
those fields always came from one record a human wrote. The
R4-5 fill-only merge can now complete the pair across two
records: the bulletin's children note plus the sheet's credit
line becomes "permission noted" although no one ever noted
permission for that picture. Fix in R5-A: the merge must not,
by filling, turn a `children_present` Some(true) record from
no-permission-noted into permission-noted; the dangerous
fills (credit_text, and license ChurchOwned / PublicDomain /
Cc0) are skipped into such a record, and skipped fills do
not count as "filled in". The underlying R-2b rule (a credit
line a human wrote on that picture's own record counts as
the note) stands as designed; what ends is manufacturing it
across records. Same batch: `asserted_by` / `asserted_at`
fill only when a substantive field was filled from the same
incoming record in the same merge, and the stale
winner-take-all doc comment above `merge_rights_records` is
replaced with the real rule.

**RT5-2 (red team). `add_asset` discards incoming rights.**
Placing a clip-library picture whose bytes are already in the
bulletin loses its CC BY license and title whole; the same
class R4-5 fixed for sheet attach, one path over. Fix in
R5-A: `add_asset` merges with `merge_rights_records` instead
of discarding (subject to the F2 guard).

**F3 (review) + RT5-4 (red team). Serving-line identity can
still cross names.** R4-C fixed swap and reorder, but two
renames in ONE profile save pair by list order and cross
silently (probe-verified: Acolytes and Readers renamed and
moved in one save put the Readers' names on the other row).
Separately, slug collisions share storage: "Prayer Team" and
"Prayer-Team" are one slot, and a line named "Childrens
Message" hijacks the built-in alias, so the This Sunday row
and the printed `{Sunday.childrens_message}` token can name
different people. Fix in R5-C: renames pair only when
unambiguous; an ambiguous multi-rename never crosses (values
stay under their old slugs and the save status names them);
a profile save whose display names slug-collide with each
other or with a built-in alias owned by another line is
refused with a plain-words sentence naming both.

## MINORs (all to R5-D unless noted)

- RT5-5: the 8,192 entry cap counts unique names, not entries
  (zip IndexMap dedupe happens first); 500,000 same-name
  entries (43.5 MB) load in 4.9 s. The cap's purpose holds;
  count raw entries so the wording is true.
- RT5-6: wordy For Sunday lines ("next Sunday", "the Sunday
  after October 25, 2026") read as no-Sunday-named, and the
  review misdescribes the file. Fix: "Sunday after [date]"
  resolves to that Sunday; a bare "next Sunday" / "this
  Sunday" resolves against the open bulletin's date and the
  review says that is what it did; with no bulletin open the
  wording admits the file named a Sunday in words.
- Review: Check's wanted-font oracle resolves only the first
  style matching a record; examine every matching style.
- Review: the trailing-weekday mismatch sentence can
  misdescribe a line whose weekday word was not about the
  date; tighten so the sentence only speaks when the weekday
  word directly framed the date.
- Verify + review both noted the stale winner-take-all doc
  comment (handled in R5-A).

## Held and accepted (do not re-report in round 6)

Applied-list scale is fine (100k entries load in 0.21 s);
`.obdi` duplicate records are safe (attach merge is
monotonic); the R4-1 font battery holds under separator
attacks; past keep-through behavior is disclosed in the plan;
duplicate For Sunday lines behave. R4-8 and R4-12/RT-8
dispositions re-weighed by the review lane and match the
code exactly. The R4-C token-bridge residual matches the
code for the single-rename case it describes; F3 above is
the wider failure, upstream of the bridge.

## Fix batches (one writer at a time, in order)

- **R5-A** assets and rights integrity: RT5-1, RT5-2, F2,
  asserted-by/at pairing, stale comment. FORMAT.md gains the
  duplicate-fold rule.
- **R5-B** contribution record truth: F1 + RT5-3.
- **R5-C** serving-line identity: F3 + RT5-4.
- **R5-D** small print: RT5-5, RT5-6, Check oracle, weekday
  sentence.

Baseline entering the fixes: 506 tests, clippy clean.

## Roadmap items recorded this round

- **Reusable snippets from imports (Jacob, 2026-10-16).**
  On import, resources and their attributions can be stored
  as reusable snippets in the local library, so what a church
  imports once is at hand for later bulletins with its credit
  intact. Feasible on shipped machinery (clip library,
  `.obdi`, RightsRecords, asset fingerprints for dedupe).
  Sequenced after the fingerprint identity build, which is
  itself queued after the convergence loop's Tier-2 gate.
- **Fingerprint design amendments (Tier-2, 2026-10-16).**
  The Opus + GPT cross-check of the content-fingerprint
  design amended three points, binding on the fingerprint
  build brief: only the spelling-sensitive canonical music
  hash earns the silent "same content" tier (the MIDI-pitch
  hash gets its own label; melody equality is a proposal);
  the MinHash rejection rationale is corrected (error at most
  ~0.044 at k=128; the full-shingle-set choice stands on
  exactness and size); candidate gating for music edit
  distance is a requirement, not an optimization.
  Adjudication record:
  `~/workspace/openbulletin-audit-wave3/tier2-fingerprint-2026-10-16/ADJUDICATION.md`.

## Shipped addendum, R5-A (2026-10-16)

R5-A landed: 506 to 513 tests, all passing, clippy
clean (full workspace runs, unpiped; the fake-helper
ETXTBSY flake did not fire).

- RT5-1: `fold_duplicate_asset_records` (ob-model)
  runs in ob-format's `load()` and `load_insert()`,
  before any reader sees the records. Tests: the fold
  in an .obd in both orders and in an .obdi
  (ob-format), and end to end (ob-app): a hand-built
  file with two records for one picture loads folded,
  and the real HTML export holds the picture back in
  both orders.
- RT5-2: `Document::add_asset` merges the incoming
  rights into the record already held (fill only)
  instead of discarding them. The loaders' honest
  record pushes for orphan bytes and the sheet
  attach's None branch are exempt by construction:
  no incoming record exists there to merge. The
  sidecar absorb path goes through add_asset, so it
  gains the merge with everything else.
- F2: the merge carries the fence. Into a record
  noting children with no permission noted, the
  credit line and the permission licenses
  (ChurchOwned, PublicDomain, Cc0) do not fill, and a
  skipped fill does not count as a fill. The
  permission test is replicated in ob-model as
  `rights_permission_noted`, with comments pointing
  at html_export.rs and back. Tests at record level
  (ob-model) and through the real export (ob-extract).
- asserted_by / asserted_at fill only when a
  substantive field filled from the same incoming
  record in the same merge; a test pins an
  assertion-only record changing nothing.
- Placement change (flagged): `merge_rights_records`
  moved from ob-app's main.rs to ob-model, beside
  RightsRecord, because add_asset (ob-model) and the
  loaders (ob-format) must call it and neither crate
  can call into the binary. main.rs keeps the name
  through a crate-root use; its attach call site and
  the round-3 and round-4 tests resolve unchanged.
  The stale winner-take-all doc comment left main.rs
  with the old definition; the corrected doc
  (fill-only rule, fence, assertion pairing) sits on
  the ob-model function. FORMAT.md states the fold
  rule and the fence where the attach merge is
  described.
- Changed pre-existing tests: NONE. The round-4 merge
  test and the round-3 attach test in ob-app pass
  unchanged against the moved function. The mapper's
  check held: no credit-fill case there carries a
  children note, and the attach test's bulletin
  record already notes its permission.
