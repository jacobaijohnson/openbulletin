# Audit round 33 — synthesis (2026-10-16)

Audited tree: `12176df` (round-32 close record; code tree
`c0dd8b8`). Three blind lanes: verify, review, red team.

**Deduped verdict: 0 BLOCKER / 2 MAJOR / 4 MINOR.**

**The exit bar is NOT met.** A full round at zero BLOCKER +
zero MAJOR is required; round 33 found two MAJORs. Round 34
is the next candidate. The Tier-2 final gate is not
triggered.

Lane verdicts: verify PASS WITH OBSERVATIONS (0/0/0 graded;
one observation, recorded here as F6 under the round-32 F12
precedent); review 0/0/2; red team 0/2/1 plus one ungraded
observation (bracket cancellation, recorded below). No
finding duplicates across lanes: the two MAJORs are both red
team witnesses on ground the other lanes did not grade, the
MINORs are three distinct subjects plus the coordinator's
record adjudication.

## What held (lane evidence, stated so the fixes do not re-open it)

- Every standing claim reproduced firsthand by verify:
  suite 976/0 exit 0, clippy exit 0, booklet pin exact
  (31,717 imposed / 34,977 reader), both fuzz bins at
  recorded source md5s with stdout md5s identical to the
  baselines (`4941c4f9…` / `efd2096b…`), 0 panics on
  40,000 + 497,024 lines; rights axes (M8) byte-identical,
  eleventh consecutive round, standing adjudicated counters
  reproduced exactly.
- Review: all six round-32 batches implement their
  dispositions in mechanism, at the named sites; zero test
  lines removed and zero assertions loosened anywhere in
  the delta; close arithmetic reconciles (940 + 36 = 976).
- Red team held lines: the R32-C cancelled family is
  genuinely linear (cancelled-with-core 290 ms vs the
  un-fixed 11,605 ms at 8,000 groups; ~20 shapes incl. new
  siblings all linear); mixed UTF-8/CP437 twin names refuse
  at .mxl; font/clip doors land both copies on plain and
  shifted packs; the 400,000-fake-EOCD storm is bounded
  (187 ms); 30,000 new-shape hostile contributions + 12,000
  zip mutations produced 0 panics; R32-A's two-claimant
  core holds; R32-B's relative glued family is clean over
  a 50-pair differential (the surviving glued divergence
  is F2 below).

## F1 (MAJOR) — The R32-A displacement audit restores only into a vacant path: a third claimant in the rename→audit window strands a live peer's lock, and the new holder retires the live peer's journal (red team RT33-1)

`audit_displaced_instance_lock_from` restores a wrongly
displaced lock only while the lock path stands vacant;
when a newer lock stands there, the displaced file is left
at `openbulletin.lock.displaced` permanently, and no code
path ever re-reads it. Inside one displacement — rename →
audit-read → restore-check — a third claimant C whose
create-new lands in that window (C parked in its own claim
loop after judging the same stale lock takeable) leaves
the audit answering false with the live peer A's lock
stranded. A still runs, still believes it holds the lock;
the lock file names C; `retire_journal_from` then lets C
retire A's *live* journal because the lock no longer names
A's token. This is F1's data-loss shape (round 32),
entered through the R32-A audit instead of the R31-B
displacement, and it defeats the round-32 synthesis claim
as stated ("a displacement that nonetheless landed on an
unjudged generation restores it and answers false"):
restoration is conditional on a race of its own.

Witness: stepped verbatim extraction with real processes
(`round33-out/redteam/lockstep33.log`): A3 (live) claims;
B renames A3's live lock aside on a stale-generation
judgment; C claims the vacant path in full; B's audit
answers false; final state `a3_live=true
a3_stranded_displaced=true c_holds=true
journal_cleared=true`. The trigger is narrower than F1's
(three near-simultaneous launches, one create inside a
few-syscall window); the structural point is not in
doubt.

Disposition: **R33-A.** The invariant: a displacement
audit must never leave a live generation stranded at the
displaced path, and its answer must not depend on a
vacancy race. Candidate shapes named by the lane:
restore by atomic exchange so the interloper's lock goes
to the displaced path and its own post-write re-read
truthfully fails its claim; or a claimant whose audit is
refused with the path occupied re-enters the judge loop
holding the displaced file instead of abandoning it. The
batch chooses the mechanism; the invariant binds. Pins
must force the rename→audit third-claimant interleaving
(the stepped-extraction evidence form of rounds 31–33 is
acceptable, a real-process pin is better) alongside the
standing two-claimant pins, unweakened. The journal
consequence (C retiring stranded A's journal) must be
impossible on the witness's final state, not merely
unlikely.

## F2 (MAJOR) — A glued weekday garnish on an absolute header line is silently swallowed (red team RT33-2)

F2's family (round 32: a remainder cut at its edge judged
on the wrong text), on ground R32-B's family pin did not
cover: the absolute lines' trailing weekday garnish. The
spaced form is the pinned R4-7 behavior: `October 25, 2026
(Monday)` names the wrong weekday, so the date does not
read and a `WeekdayMismatch` is recorded. Glue the group
to the year — `November 1, 2026(Monday)`, ordinary typing,
no space before the parenthesis — and the garnish
vanishes into the date token's cleaning: the date reads,
no mismatch is recorded, nothing warns. Witnessed on the
For Sunday line and the Keep-through line, with and
without a leading weekday word, disagreeing and agreeing
garnishes (`round33-out/redteam/parse33.log`,
`parse33c.log`). A glued parenthesized *date* composes
correctly glued and spaced; the loss is specific to the
weekday-word garnish — the one garnish whose whole
purpose is the warning. Whether this predates R32-B or is
its sibling, it is a sender-plausible input producing a
wrong reading with a silenced check; grade follows F2.

Disposition: **R33-B.** The glued garnish is judged
exactly as the spaced twin: a disagreeing glued weekday
refuses the date and records the `WeekdayMismatch`; an
agreeing one reads as spaced. Pins: glued/spaced twins on
both absolute lines, agreeing and disagreeing, plus the
leading-weekday variant from the witness. This batch
touches the contribution engines: the standing fuzz bins
re-run as claims at its cold verification, and no other
reading may move.

## F3 (MINOR) — Shifted archives carrying `PK\x01\x02` in the scan window are refused wholesale at the IDML, font-pack, and clip doors (red team RT33-3)

R32-D's scans derive the archive offset the way the
reader (zip 2.4.2) does — a forward hunt for the first
`PK\x01\x02` at-or-after the declared offset — so reader
and scans agree even in failure. A well-formed shifted
archive whose last entry is Stored and whose payload
carries the signature bytes inside the window makes
`ZipArchive::new` itself fail ("Could not find EOCD"),
and every door reaching the reader refuses the whole
package. Two payload shapes witnessed at all three doors
(`round33-out/redteam/zipdoors33.log`, `fontclip33.log`,
`wellformed33.log`); Python's arithmetic-offset zipfile
opens both and lists all entries, duplicates included.
Fail-closed throughout — nothing wrong is served and no
duplicate check is bypassed — so graded MINOR as F4 was:
the claim "shifted packages load, duplicates refused"
holds only for shifted archives whose stored tails are
free of the signature in the window. The same forward
search guards the zip64-record search in all four scan
copies (code read; not separately witnessed). The .mxl
and .obd doors' head-sniff refusal of junk-prefixed bytes
is standing (F4's note) and not in scope.

Disposition: **R33-C.** The invariant: a well-formed
shifted archive whose central directory is arithmetically
derivable opens at these doors, with duplicate judgment
exactly as today. Mechanism is the batch's problem (the
reader itself shares the hunt; pre-normalization or an
arithmetically-derived reader are the shapes the lane's
evidence points to). Pins carry both fake witnesses at
all three doors plus the shift-plain controls unchanged.

## F4 (MINOR) — The R32-C timing pins' stated tightness does not hold, and the headline pin has no headroom under load (review MINOR-1)

The pin block's header (`crates/ob-extract/src/contribution/tests.rs:19097`,
claim at :19100) states the bounds are "at most a third
of the un-fixed code's" at the same size. On the round's
own recorded numbers that is false: cancelled-with-core
un-fixed 11,605 ms → a third ≈ 3.9 s, the bound is 20 s;
relative-cancelled 35,199 ms → ≈ 11.7 s, the bound is
15 s. The five-times-the-fixed-measurement half holds.
Runtime witness: in a contended batch run (three lanes
building concurrently) the cancelled-with-core pin
**failed** at 22.24 s against its 20 s bound and passed
standalone at 19.77 s — 1% headroom for a shape the batch
measured at 3,987 ms fixed. The fix's scaling is not in
doubt; the pin tests machine load as much as code, and
its header claims a tightness the numbers contradict.

Disposition: **R33-D** (with F5). The record is made true:
the header states the ratios the recorded numbers
support, and the bounds are re-derived so the pins judge
the code, not the machine — an in-suite interleaved
control in the disposition's original terms, or
load-absolute bounds whose headroom survives a contended
run. The coordinator judges the batch on a contended
re-run, not a quiet one.

## F5 (MINOR) — The weekday-fallback suffix skip rests on a cross-crate predicate identity that is unstated and unpinned (review MINOR-2)

`date_line_inner`'s suffix skip (`composition.rs:1743`,
applied :1773) counts "pieces" by contribution's
`nothing_token` (`dates.rs:1171`); the bound is sound
exactly when that predicate agrees with ob-model's
private `nothing_token` (`crates/ob-model/src/lib.rs:782`),
which `parse_flexible_date` actually tokenizes by. The
two copies are identical today (verified by the lane's
read) and the skip provably changes no reading today —
but the site comment never names the premise and no pin
couples the copies, so a future edit to ob-model's
predicate (round 29 changed it once) could silently turn
the optimization into a moved reading. F2's
claim-vs-coverage family, one level down. No present
divergence.

Disposition: **R33-D** (with F4). The premise is named at
the skip site, and a pin couples the two predicates over
a fragment family that routes through the fallback (or
the predicate is shared — the batch chooses; a silent
pair of copies is the one outcome not accepted). This
batch touches the contribution engines: fuzz-as-claims
at its cold verification.

## F6 (MINOR, record) — The 66,620-case differential corpus is not preserved evidence (verify lane observation; coordinator adjudication)

Verify states it plainly: no corpus tooling, runner, or
baseline is preserved in the audit tree (probe29v holds
only the five standing bins), so the R32-B/C byte-identity
claim could not be re-run this round and is **not**
re-asserted by this round's evidence. It rests on the
batch records, the in-suite readings pins, and the fuzz
identity — which did reproduce. Recorded as a MINOR
under the round-32 F12 precedent (a provenance limit on
a standing claim). Going forward: the standing evidence
set is the fuzz bins plus in-suite pins; a future batch
that wants differential-corpus weight must preserve its
tooling and baseline in the audit tree at its close.

## Observation (ungraded) — Bracket groups do not apply cancellation the way paren groups do (red team)

`November 1, 2026 [November 8, 2026, cancelled]` records
the bracket-cancelled Nov 8 as a live second date where
the paren twin records nothing; `November 1, 2026
[cancelled]` reads no date at all where `(cancelled)`
keeps it (`round33-out/redteam/brc33.log`). No round's
claim covers bracket cancellation semantics — the
behavior is long-standing and its current readings are
fuzz-pinned — so no grade this round. But the two
readings cannot both be what a sender means by the same
words in brackets. Recorded as an open semantic question:
round 34's lanes inherit it, and any change is a reading
change requiring adjudication of intent first, not a
drive-by fix.

## Dispositions — fix batches

One writer at a time, in this order. Every batch under
the standing batch-boundary discipline (Jacob,
2026-10-20; recorded at `12176df`): after the batch
commits and pushes and HEAD == origin/main is confirmed
on its commit, `cargo clean` (registry never pruned),
then the batch's verification runs **cold** from the
clean tree — full unpiped suite + clippy `-D warnings` +
booklet pin, plus the fuzz bins re-run as claims where
the batch touches the contribution engines (R33-B and
R33-D do). Record per batch: disk free before clean /
after clean / after cold rebuild, and the cold run's wall
time from log timestamps.

- **R33-A** — F1 (MAJOR): the displacement audit never
  strands a live generation. Standing lock pins
  unweakened; three-claimant interleaving pinned.
- **R33-B** — F2 (MAJOR): glued weekday garnish judged as
  its spaced twin on the absolute lines. Fuzz gate.
- **R33-C** — F3 (MINOR): well-formed shifted archives
  open at the IDML, font-pack, and clip doors; duplicate
  judgment unchanged.
- **R33-D** — F4 + F5 (MINOR): the R32-C record layer
  made true (timing-pin header and bounds; the
  `nothing_token` premise named and coupled). Fuzz gate.

F6 is a record; no batch. The bracket observation goes to
round 34. HF-12 untouched (Jacob's decision).

## Close gate (standing)

The close cites all five legs on the close tree: local
cold suite + clippy with booklet exact; fuzz as claims
(byte-identical stdout, 0 panics both bins); hosted CI
success on all three OS jobs (cited by run id); desktop
runner build success; Windows bundle success via
`gh run view`. After the close: the standing boundary
hygiene (clean + stale probe sweep, fuzz bins kept at
recorded md5s, registry not pruned, disk numbers, cold
proof run). A close missing any leg is not verified.

## Close (coordinator, 2026-10-20)

All four batches landed. Round 33 is the first round
verified under the batch-boundary storage discipline
(Jacob, 2026-10-20): every batch was committed and
pushed, HEAD == origin/main confirmed on its commit,
`cargo clean` run (registry never pruned), and the
batch's verification then run **cold** from the clean
tree, replacing incremental verification. Per-batch
records (disk free before clean / after clean /
after cold rebuild, cold wall times from log
timestamps) live in the audit dir at
`round33-out/coord/r33{a,b,c,d}-record.md`. Booklet
baseline (31,717 imposed / 34,977 reader) exact in
suite at every step. Suite count: 976 at the
round-32 close, 986 at this close.

- R33-A `a1b9d39` (F1, MAJOR) — 978 tests. Cold:
  suite 14m44s, clippy 5m45s, both exit 0. The
  displaced lock path is now a judged location at
  claim and at journal retirement; the red team's
  verbatim witness inverts (`a3_live=true
  a3_stranded_displaced=false
  journal_cleared=false`); no standing lock pin
  edited.
- R33-B `9d11d4d` (F2, MAJOR) — 979 tests. Cold:
  suite 14m05s, clippy 4m18s, both exit 0. Glued
  weekday garnish is re-spaced inside
  `date_line_inner` so it is judged as its spaced
  twin; RED-first twin pin. Fuzz gate: both stdouts
  byte-identical — the changed lines are a
  refuse/judge class outside both fuzz corpora.
- R33-C `b5352df` (F3, MINOR) — 985 tests. Cold:
  suite 13m59s, clippy 3m41s, both exit 0. One
  arithmetic central-directory derivation in
  `ob_model::zip_directory`; all four scan copies
  deleted their own preambles; six new pins.
  **Deviation, accepted and recorded:** a zip64
  record carrying an extensible data sector is not
  arithmetically derivable from the end record, so
  that one case falls back to the forward signature
  hunt, which now exists only inside the shared
  helper as its documented fallback. Round 34's
  review lane inherits the fallback as a named
  site.
- R33-D `601776c` (F4, F5, MINOR) — 986 tests. The
  close tree. Cold: suite 14m47s, clippy 4m03s,
  both exit 0. All seven R32-C timing pins are
  ratio-form (witness against an interleaved
  absolute-group control, best-of-3); the false
  header claim is corrected on the record with its
  frames named; same-frame measurements put every
  bound between the un-fixed and fixed frames. The
  coordinator additionally re-ran all seven pins
  under load (five concurrent with the cold clippy
  build, two under tenant load): all pass, where
  the old 20 s form failed at 22.24 s in the review
  lane's contended witness. F5 closed by sharing:
  `ob_model::nothing_token` is now the one public
  statement of the flexible readers' nothing
  predicate and contribution's delegates to it,
  with the premise stated at the skip site and a
  coupling pin over the fragment family. Fuzz
  gate: both stdouts byte-identical, no reading
  moved.

F6 (MINOR, record) stands as written in the
findings: the 66,620-case differential corpus has
no preserved runner or baseline in the audit tree,
so future rounds cite it as history, not as a
re-runnable claim; any future corpus of that kind
is preserved with its runner and baseline or not
cited. The ungraded bracket observation
(`[November 8, 2026, cancelled]` surfaces as a live
second date; bracket groups do not apply
cancellation the way paren groups do) goes to
round 34 ungraded: no round's claim covers bracket
semantics, and any change there is a reading
change that needs its intent adjudicated first.

**Fuzz as claims on the closed tree**
(coordinator's own runs, bins at recorded md5s
verdict `a9cac6c3…`, comp `4f85c605…`): verdict
40,000 lines, panics=0; composition 497,024 lines,
D_PANICS=0. Both stdouts **byte-identical to the
round-30 close baselines** (md5 verdict
`4941c4f9…`, comp `efd2096b…`). The round's one
intentional reading change (R33-B) is outside both
corpora and is pinned in suite. Coordinator
stdouts: `round33-out/coord/r33close-fuzz-verdict.stdout`,
`r33close-fuzz-comp.stdout`.

**Exit bar: NOT met by round 33's audit verdict
(0 / 2 / 4); all six findings are now fixed or
dispositioned (F6 is a record), so round 34 audits
the closed tree `601776c` as the next candidate.**
Majors trend: 10 → 6 → 6 → 1 → 4 → 3 → 2.
HF-12 remains untouched (Jacob's decision). The
Tier-2 final gate is not started. The close's
five-leg evidence (local cold on the close tree,
fuzz, three-OS CI, desktop runner build, Windows
bundle) and the round-boundary hygiene record live
in the audit dir (`round33-close-status.md`), per
the standing gate.
