# Audit round 31 — synthesis (2026-10-16)

HEAD audited: `3722169c823a795116df52b44d3262bf9e9d0960`
("Round 30 coordinator close"). Base for diffs: the
round-30 audited tree `f03d47e` (health-fix close).
All three lanes read-only and blind; repo `git
status` clean at every lane's close, HEAD unchanged.
Lane scratch: `round31-out/verify/` (+ `probe31v`),
`round31-out/review/`, `round31-out/redteam/`
(+ `probe31rt`); lane reports `round31-verify.md`,
`round31-review.md`, `round31-redteam.md` in the
audit dir.

**Round 31 verdict: 0 BLOCKER / 4 MAJOR / 6 MINOR
deduped — the exit bar is NOT met.** Round 31 was
the first round that could have met the bar (zero
BLOCKER + zero MAJOR). It did not: the round-30
fixes themselves carry four majors, three of them
in the two mechanisms round 30 was proudest of —
the journal peer fence (F2, F3) and the F6
duplicate-name boundary (F1) — and the fourth is
the recorded parenthesized-group residual, now
localized to the header lines and measured worse
than the record said (F4). The loop continues to
round 32 after the fix batches below.

Verify lane: **PASS, 0 / 0 / 0.** Every standing
claim re-run fresh on the audited tree: suite
**915 passed / 0 failed** (`SUITE_EXIT=0` read from
disk), clippy `-D warnings` exit 0, booklet
**31,717 / 34,977** byte-exact. Fuzz bins as claims
at their recorded md5s (verdict `a9cac6c3…`, comp
`4f85c605…`): verdict 40,000 lines, panics=0, I1–I3
violations 0; composition 497,024 lines,
D_PANICS=0, governed resurrections 943; **both
stdouts byte-identical to the round-30 close
baselines (diffs 0 lines)**. Rights axes (M8):
children_present 0 flips on every route and check
(187,236 route comparisons in the bins' combined
account), permission 0 route flips, credit
invariant on the standing sequences; both rights
stdouts byte-identical to round 30 — **M8 holds,
ninth consecutive round** in the current counting.
Test-body integrity `f03d47e` → `3722169`: 0 test
functions removed, 20 added, zero removed assert
lines; the three token-diff flags all resolve as
slicer artifacts or one F5 call-site adaptation.
F6 boundary under an independent probe (`dup31`):
all three adjudicated edges behave as recorded for
honest duplicates (disagreeing read names refuse,
exact repeats load, never-read names uncounted).
Environment incidents, on the record: the VM
rebooted at least three times during the lane and
the exec layer twice delivered other sessions'
outputs as the lane's results (including a
fabricated 915/0 tail); the lane discarded all of
it and every number in its report comes from a
completed log re-read from disk. Its first suite
run also failed one test on ENOSPC from a missing
TMPDIR export (the 512 MB tmpfs), re-run in full
with TMPDIR on the workspace disk.

Review lane: **0 / 2 / 5.** Baseline re-run fresh
(suite 915/0, clippy 0). Both majors are places
where a round-30 batch's stated claim is broader
than its code (F1, F2 below). Cleared with evidence:
the Unix tri-state at both call sites including
error paths; the lock survey (single writer, no
deleter); **R30-D probe-verified** — old vs new
`parse_date_prefix` differenced over 1,514,424
adversarial token sequences, 0 mismatches, and it
is the only prefix-search entry point; the F2
content-hash memo (filters agree, failure
directions fail safe); F3 temp cleanup on every
failure path of both durable-write twins; F4(a)/(b)
note conditionality and landed-name pairing; F6's
six acceptance paths each passing exactly one
per-candidate count.

Red team lane: **0 / 3 / 2**, and a clean panic
hunt: 46,000 generated hostile contributions plus
targeted edge shapes (10,000-deep nesting, a 2 MB
single token, embedded NULs, absurd years) — **0
panics**; the round-26 panic class did not recur.
Held under attack: the .obd decompression bomb
(600 MB of zeros refused by the actual-bytes
512 MB cap in 1.2 s), the H-A sidecar fence
(canonicalize + containment + magic bytes),
case-sensitivity at the F6 boundary, helper
containment in source.

Raw lane totals 0 BLOCKER / 5 MAJOR / 7 MINOR
dedupe to the 4 / 6 below: the CRC32 finding was
reported independently by review and red team
(one root, F1); the claim race appears in both
lanes at different grades and is adjudicated once
(F3).

## F1 (MAJOR) — F6 "exact repeat" is CRC32 + sizes, not bytes; a forged CRC twin is shadow-read (review MAJOR-2 ≡ red team RT31-2)

`note_read_name` / `record_facts`
(`crates/ob-format/src/lib.rs:781-808`) judge
whether two copies of a read name are "the same"
from the central directory's declared facts only:
CRC32, compressed size, uncompressed size. The
entry bytes are never compared (and for zip64
entries the size fields are the `0xFFFFFFFF`
sentinel, so real sizes are not compared either).
CRC32 equality plus equal length does not imply
equal bytes; a 4-byte GF(2) patch makes two
different payloads share a CRC32.

Two blind lanes built the witness independently
against the real loader:

- Review: honest `document.json` (title
  `" Bulletin"`, CRC32 `0x6622f8b0`) plus a shadow
  body (title `" Shadow!!"`, patched to the same
  CRC32 and length) in `crafted-crc-twin.obd` →
  **LOADED title `" Shadow!!"`** — the last copy
  served, the exact shadow read the F6 pins are
  named against. Controls: disagreeing copies with
  natural CRCs refused with the F6 words;
  byte-identical repeat loads.
- Red team: `dup_forged.obd`, two `document.json`
  entries, same length 4,133 B, same CRC32
  `2e3da984`, different titles → **`LOAD OK`** with
  the second copy's title; the natural-CRC control
  `dup_plain.obd` refused.

Assets are not exposed (their bytes are
sha256-checked against the entry name,
`lib.rs:1945`); the exposed names are the two JSON
roots, `document.json` and `insert.json` — the
sheet door shares the same helper, so it falls by
code identity. The harm needs a hand-crafted file,
but this loader's candidate machinery exists for
hand-crafted files, and the batch's stated
invariant ("whichever copy the reader serves, it
serves the same bytes") is false as written.
**Fix (R31-A):** for the root names, "exact
repeat" is judged on the payloads themselves —
read and compare (or cryptographically hash) both
copies before the repeat is allowed to load; a
CRC/size match with differing bytes refuses with
the existing F6 words. Pins: both lanes' forged
witnesses refuse; genuine byte-identical repeats
still load (the round-4 pin stands).

## F2 (MAJOR) — R30-A lock stewardship does not exist on non-Unix; a live peer's journal is retirable on Windows exactly as before F1 (review MAJOR-1)

Off Unix, `peer_liveness` can only answer `Unknown`
(`crates/ob-app/src/main.rs:3542-3546`), and
`peer_provably_gone` honors it (`Unknown => false`,
line 3589). But `peer_instance_alive_from`'s
non-Unix arm discards the tri-state and returns
`false` unconditionally (`main.rs:3605-3630`), so
`claim_instance_lock_from` always overwrites a
foreign lock on Windows (`main.rs:3650-3656`), and
`peer_alive` is therefore always false there
(`windows.rs:984`). Instance B's save then retires
instance A's live journal through the
`lock_token != meta_token` rule — the lock names B
only because B replaced it. The full F1 kill chain,
intact, deterministically, on one of the three CI
platforms.

The record overstated this and the correction is
part of the finding: the round-30 synthesis and
close state the invariant ("unknown never reads as
gone"; "a live foreign owner keeps its lock")
without a platform qualifier. That is true of the
Unix arms and of `peer_provably_gone` everywhere;
it is false of the claim composition off Unix. The
pin `peer_alive_stays_token_only_off_unix`
(`main.rs:6774`ff) asserts the shipped behavior —
token-only protection — which is precisely the
posture that fails, since the token rule protects
the journal only while the lock still names its
owner. CI stayed green on Windows because the test
asserted the behavior, not the invariant.

**Fix (R31-B):** the invariant holds on every
platform: a lock naming a foreign token whose
owner cannot be proven gone is not overwritten,
and reads as a live peer for the claim, the
warning, and the journal hold. How Windows proves
liveness (a real process query) or refuses
conservatively is the builder's choice inside that
invariant; what is not acceptable is narrowing the
claim's wording again. The off-Unix pin is
rewritten to assert the invariant, and the claim
site must be exercisable off Unix in the suite
(the `_from` test seams already exist).

## F3 (MAJOR, adjudicated up from review MINOR-1) — The lock claim is a non-atomic check-then-write; two near-simultaneous launches both hold the lock (review MINOR-1 ≡ red team RT31-3)

`claim_instance_lock_from` (`main.rs:3650-3656`)
reads liveness, then `write_instance_lock_to`
(`main.rs:3486`) does a plain `std::fs::write`
(truncate-then-write), swallowing its error, with
no atomic create, no rename, and no post-write
re-read. The doc contract says the claim "answers
true when this instance took the lock and now
holds it" (`main.rs:3632-3645`); both halves can
be false: (a) two instances launched together can
both read "no live peer" before either writes,
both return true, both run as sole owner sharing
one journal; (b) a failed write still returns
true.

Grade adjudication: review graded the contract
falsity MINOR; red team proved the consequence —
with both instances holding, the shipped
retirement rule lets the last writer retire the
first instance's *live* journal
(`lockreplica.log`: both claims true; "B may
retire live A's journal" true), which is F1's
data-loss shape re-entering through the startup
race R30-A did not close. The trigger is an
ordinary double launch, not a crafted attack. The
proof is a faithful algorithm replica (the
functions are private to the GUI crate and no
two-binary run was made); the source facts are
read identically by both lanes. **MAJOR.**

**Fix (R31-B, with F2):** the claim becomes
atomic and verified — create-new semantics (or
rename-if-absent) so exactly one claimant wins, a
write failure answers false, and the winner
re-reads the lock and proceeds as holder only if
it still names the claimant. Pins force the
check-check-write-write interleaving at the
`_from` seams and a write-failure case.

## F4 (MAJOR) — The parenthesized-group quadratic lives in the header lines; measured worse than recorded, with two sibling shapes (red team RT31-1; verify claim 7, measurement)

The round-30 residual is confirmed, localized, and
re-measured by two harnesses. The cost is in the
header lines (For Sunday / Keep through); the same
groups in an announcement body are flat (304 KB →
67 ms). It is not the F7 cliff (R30-D bounded
`parse_date_prefix`; review differenced that fix
clean) — it lives in the group/composition
machinery the header lines run per group.

Verify (`paren31b`, min/median-of-3, release),
`next Sunday (November 1) ×N` on a For Sunday line:
3,478 B → **12.9 ms**; 33,469 B → **1,193.5 ms**;
100,328 B → **11,154 ms**. Scaling is quadratic
(9.6× size → 92× time). Second hot shape: repeated
cancelled full-date groups `(November 1, 2026,
cancelled) ×N`: 5,584 B → 28.4 ms; 54,034 B →
2,168.5 ms. Controls (groups alone, absolute core,
For+Keep sharing) run 20–30× faster at the same
sizes; readings unchanged everywhere.

Red team (`probe31rt`, min-of-3), absolute-date
paren groups on the For Sunday line: 38,050 B →
147 ms; 171,050 B → 1,714 ms; 342,050 B →
**9,046 ms**; 513,050 B → 16,621 ms. Keep through
matches (342,064 B → 7,699 ms). Sibling superlinear
shapes on the same lines: semicolon groups
(`; (November 1, 2026)` ×N) 360,049 B → 4,016 ms;
conjoined groups (`(November 1, 2026) and ` ×N)
414,050 B → 3,236 ms.

The recorded round-30 numbers (~0.74–0.95 s at
36 KB, "bites around ~340 KB") describe the
mildest measured shape. On the worst shape the
bite is already ~11 s at 100 KB. A 171 KB
contribution — a season's announcements pasted
into one file, a mail-merge accident repeating one
group — freezes the parse for seconds on a single
header line. **Fix (R31-C):** find the per-group
rescans in the header group/composition path and
bound them so every measured shape (paren groups
on absolute and relative cores, cancelled groups,
semicolon groups, conjoined groups) runs near the
control shapes' cost, with timing pins on all five
shapes and the standing gate: no date reading may
move (fuzz bins byte-identical after the batch).

## F5 (MINOR) — Duplicate last-copy-wins at the .mxl and IDML doors (red team RT31-4)

Neither import door carries the duplicate-name
discipline. `.mxl` (`crates/ob-music/src/parse.rs`,
`zip_score_text`): duplicate `score.musicxml`
entries serve the last copy; duplicate
`META-INF/container.xml` entries redirect the
score choice to the last container. IDML
(`crates/ob-extract/src/idml.rs`): duplicate story
entries serve the last copy (`files.insert`).
Witnesses on disk in `round31-out/redteam/`
(`dup_score.mxl`, `dup_container.mxl`,
`dup_story.idml`). Graded MINOR: no adjudication
promise exists at these doors, imports land in a
review surface before use, and no real exporter
emits duplicate entries. Recorded and fixed
because the project's own stated principle — which
copy a file was read from must never be the
archive's quiet choice — does not stop at the
.obd door. **Fix (R31-A, with F1):** at both doors,
a duplicated name whose copies disagree refuses
(or, for names the door resolves by lookup, is
treated exactly as F6 treats read names); exact
repeats may load. Pins carry both lanes' witness
shapes.

## F6 (MINOR) — A corrupt lock naming an unnamable pid wedges the claim forever (red team RT31-5)

`peer_liveness` answers `Unknown` without a
syscall for pid 0 and pid > `i32::MAX`
(`main.rs:3525`), and Unknown reads as a live peer
at the claim. A lock file naming a foreign token
and pid 0 (or 4294967295) therefore blocks the
claim, mutes the journal, and cannot be retired —
forever; nothing ages a lock out. The app's own
writer always writes its real pid, so such a lock
is provably not a healthy instance's: it is a
damaged file (the lock write is a non-atomic
`fs::write`; a crash mid-write, a restored backup
of the settings folder, or hand tinkering can
leave one), and it has no in-app recovery short of
deleting `openbulletin.lock` by hand. The sibling
case — a recycled pid whose /proc identity is
unreadable — is **recorded, not fixed**: it is
indistinguishable from a live peer, and the wedge
there is the safe direction doing its job.
**Fix (R31-B, with F2/F3):** a lock whose pid
field cannot name a process at all (0 or above
the namable range, or unparseable) is corrupt,
not a peer: the claim may take it and the
retirement path may clear its journal. The
boundary is provability — a pid that *could* be a
live process still reads as a peer when its
liveness is unknown. Pin covers pid 0, an
over-range pid, and a garbage pid field.

## F7 (MINOR) — F5 note naming still collides through the sanitizer (review MINOR-2)

`license_note_name` (`crates/ob-fonts/src/
catalog.rs:581-583`) runs the family id through
`sanitize_file_name` (`crates/ob-fonts/src/
lib.rs:205-219`), which keeps only the text after
the last `/` or `\` and strips leading dots.
Distinct ids still collapse onto one note:
`"a/b/c"` ≡ `"c"`, and `"../../escaped-note"` ≡
`"escaped-note"` — the batch's own updated S-4
test (`catalog.rs:731`) writes the note under the
first form expecting the second's file. A later
install silently overwrites the earlier family's
note: F5's exact harm, one shape later. Catalog
ids are remote-supplied and treated as hostile by
the S-4 fence, so the trigger is a crafted or
merely slash-bearing catalog answer. **Fix
(R31-D):** the note name must be injective over
distinct ids before sanitization can squash them
(name from a form that keeps the whole id
distinguishable, e.g. a hash of the raw id riding
alongside the readable part); the S-4 test is
corrected to expect two distinct notes. Pins:
separator-differing and dot-differing id pairs
land as two notes, each with its own family's
contents.

## F8 (MINOR) — F6's read-name coverage is a hand-kept list that can drift from the read sites (review MINOR-3)

`note_read_name` (`crates/ob-format/src/
lib.rs:781-800`) decides coverage from a passed-in
root string plus a hard-coded `assets/` literal.
Today that matches the readers exactly (review
verified all six acceptance paths pass exactly one
per-candidate count into both readers), but
nothing ties the list to the read sites: a future
door reading a new name class (a `media/` prefix,
a second root) is silently uncounted and no test
fails. **Fix (R31-E):** a pin that enumerates the
read names from the readers' side (or the list
derived from one shared definition both the
readers and the checker use), plus cross-reference
comments at both read sites and the checker.
Behavior unchanged; this is the drift alarm, not
a behavior fix.

## F9 (MINOR) — The durable-write helper still lives twice; F3 was a manual re-sync (review MINOR-4)

`sync_and_rename` (`crates/ob-format/src/
lib.rs:1467-1493`) and `write_file_atomically`
(`crates/ob-app/src/settings.rs:630-654`) are the
same helper maintained as two bodies in two
crates. Behavior today is equivalent (review read
both failure-path by failure-path; MINOR, not a
defect), but F3 existed because the two drifted,
and R30-C repaired the drift by hand, leaving the
structure that produced it. The health phase's own
precedent is on record: `read_capped` was
single-sourced into the lowest common crate so
six doors could not drift (H-G). **Fix (R31-E):**
one canonical durable-write helper in the lowest
crate both callers can reach (ob-model, per the
`read_capped` precedent, stated over the pieces it
needs), both call sites moved to it, failure-path
pins carried over and run against the shared body.

## F10 (MINOR) — The `write_license_note` doc comment misstates the code (review MINOR-5)

The comment added by R30-B (`crates/ob-fonts/src/
catalog.rs:528-539`) says `repo_id` "is used only
to fetch the license text that lives beside the
font files". The function also uses it as the
note's family name when the catalog detail lacks
one (`catalog.rs:548`), and the fetch is not
beside the files for a third of the probe space:
install probes `ofl`, `apache`, and `ufl` folders
(`catalog.rs:318`), but the fetch is hard-coded to
`ofl/{repo_id}/OFL.txt` (`catalog.rs:553`), so
apache/ufl families silently fall back to the
short catalog note. The fetch behavior predates
the batch; the comment describing it does not
match it. **Fix (R31-D, with F7):** the comment
states the truth (both uses of `repo_id`; which
folders the fetch actually tries), and the fetch
tries the same folder the probe found the family
in, so an apache/ufl family gets its full license
text when one exists. Pins: an apache-folder
family lands the full text, not the short note.

## Dispositions — fix batches

- **R31-A — duplicate names judged on content, at
  every zip door (F1, F5).** ob-format root names
  compare payloads before a repeat counts as
  exact; .mxl and IDML doors refuse disagreeing
  duplicates of the names they resolve. Pins: both
  forged witnesses refuse, natural-disagreeing
  controls refuse, byte-identical repeats load,
  the round-4 and RT5-5 pins stand.
- **R31-B — the lock tells the truth on every
  platform (F2, F3, F6).** Platform-true
  stewardship invariant (foreign lock not provably
  ownerless is never overwritten, on any OS);
  atomic verified claim with post-write re-read;
  unnamable-pid locks treated as corrupt, not
  peers. The off-Unix pin is rewritten to assert
  the invariant. Pins: the interleaving, the
  write-failure case, pid 0 / over-range / garbage,
  and the two standing R30-A pins unweakened.
- **R31-C — the header-line group machinery stops
  rescanning (F4).** All five measured shapes
  (absolute-core paren groups, relative-core paren
  groups, cancelled full-date groups, semicolon
  groups, conjoined groups) pinned with timing
  bounds near the control shapes' cost. Standing
  gate: fuzz bins byte-identical after the batch —
  no date reading moves.
- **R31-D — catalog naming and license truth
  (F7, F10).** Injective note names; S-4 test
  corrected; fetch tries the probed folder; comment
  states both uses of `repo_id`. Pins per F7/F10.
- **R31-E — drift alarms and single-sourcing
  (F8, F9).** Read-name coverage pinned from the
  readers' side; one canonical durable-write
  helper, both callers moved, failure-path pins
  on the shared body.

Recorded, not fixed: the recycled-pid-with-
unreadable-identity half of F6 (indistinguishable
from a live peer; the wedge is the safe direction
working). Round-30 F8/F9 stand as that synthesis
left them. HF-12 is untouched (Jacob's decision).

## Close gate (standing, from the CI incident)

The round-31 close must cite all five legs: local
suite + clippy on the close tree, the fuzz bins as
claims (byte-identical), the Windows bundle result
on the close commit via `gh run view`, and hosted
CI conclusions for macos, windows, and ubuntu on
the close commit. After the close: the standing
boundary hygiene (cargo clean + stale probe dirs,
fuzz bins and reports kept, registry not pruned),
disk numbers, and one cold full-suite run.

## Coordinator close (round 31)

All ten findings are fixed or dispositioned on the
record. Batches, each verified by the coordinator on
its committed tree (full unpiped suite + clippy
`-D warnings` + booklet pin, run independently of
the builder):

- **R31-A** `4b6794f` (F1, F5): 925 tests. Root-name
  repeats are judged on payload bytes (located from
  each central record, zip64 resolved, read raw
  never through the archive reader); the .mxl and
  IDML doors refuse disagreeing duplicates found by
  a raw central-directory scan (the zip crate's name
  map collapses duplicates before either door's loop
  could see them). Ten new pins, including an
  in-test GF(2)-forged CRC twin at both roots.
- **R31-B** `20b5aed` (F2, F3, F6): 929 tests.
  Windows gets a real liveness answer (kernel32
  OpenProcess/GetExitCodeProcess, scoped unsafe,
  no new dependency); the platform cfg split in
  `peer_instance_alive_from` is gone — one decision
  table everywhere, Unknown reads as a live peer.
  The claim is atomic (create-new + sync + post-write
  re-read; a takeover displaces the old lock file by
  rename, never deletes `openbulletin.lock`) and a
  write failure answers false. Unnamable-pid locks
  are corrupt, not peers, at claim and retirement.
  Hosted CI on this commit: **success on macos,
  windows, and ubuntu** (run 39405317555) — the
  Windows job is the settler for the FFI code and
  the rewritten off-Unix pin, which could not be
  executed locally.
  *Deviation adjudicated and accepted:* the brief
  required the R30-A/CI-FIX pins "unmodified" and
  also required pid 0 to be corrupt; two pins
  modeled a live peer *as* pid 0, so both texts
  could not hold literally. The builder kept every
  assertion and changed the model: the live peer is
  now a real spawned process (a copy of the system
  sleep binary named `openbulletin-peer`), judged by
  the /proc identity refinement exactly as a real
  peer is. One setup line in
  `journal_retirement_respects_the_owner_token` was
  adapted to the new claim API; its assertions are
  untouched. The coordinator accepts this: the pins
  now assert the same invariant against an honest
  model instead of the pid-0 fiction F6 retired.
- **R31-C** `ac98ee8` (F4): 935 tests. Five rescan
  sites fixed (cross-group governance counted per
  date value once; relative-shape judging cleans
  tokens once; a one-pass next-nonempty table;
  first-word gates on the governor/target phrase
  families; the superseded-chain screen). All five
  measured shapes are linear now (worst before/after
  on the builder's harness: 100 KB relative groups
  7,393 ms → 48 ms). No-reading-moved evidence is
  threefold: a 65,720-case differential corpus
  byte-identical before/after (md5
  `1a8f1369493ef629738844c6e59aabda`), readings pins
  taken from the un-fixed code, and the coordinator's
  fuzz re-run below.
- **R31-D** `fd08caf` (F7, F10): 937 tests. Note
  names stay plain for lossless ids and carry a
  truncated sha256 of the raw id when sanitization
  was lossy (the hashed form is disjoint from every
  plain name). The license fetch walks the probed
  folders in probe order (ofl/OFL.txt,
  apache/LICENSE.txt, ufl/UFL.txt). The S-4 hostile-id
  pin was adapted in expectation (two distinct notes)
  as the batch brief named in advance.
- **R31-E** `5d9b38a` (F8, F9): 940 tests. The
  read-name space is stated once (`BULLETIN_ROOT_NAME`,
  `INSERT_ROOT_NAME`, `ASSET_ENTRY_PREFIX`,
  `is_read_name`) and used by both readers, both
  writers, and the checker — no root/`assets/`
  literal remains in production code outside the
  constants. The durable-write helper is
  single-sourced as `ob_model::sync_and_rename`;
  both former bodies are deleted, failure-path pins
  run against the shared body (one re-pointed pin
  reads the io error's text form where the local
  copy answered String; assertions otherwise intact).

**Fuzz as claims on the closed code tree**
(coordinator's own runs, bins at recorded md5s
verdict `a9cac6c3…`, comp `4f85c605…`, run after
R31-C and again on the final tree `5d9b38a`):
verdict 40,000 lines, panics=0; composition
497,024 lines, D_PANICS=0, governed resurrections
943. Both stdouts **byte-identical to the round-30
close baselines at both runs** (md5 verdict
`4941c4f9…`, comp `efd2096b…`). No date reading
moved anywhere in the round. Coordinator stdouts:
`round31-out/coord/close-verdict.stdout`,
`close-comp.stdout`. One process note: an early
coordinator fuzz attempt through `cargo run`
produced a delivered summary that the disk files
did not support (empty captures; a stale stderr);
it was discarded in full and the claims above rest
only on direct binary runs whose outputs were
md5-compared on disk — the same fabricated-delivery
shape the verify lane recorded this round.

**Exit bar: NOT met by round 31's audit verdict
(0 / 4 / 6); all ten findings are now fixed or
dispositioned, so round 32 audits the closed tree
as the next candidate.** HF-12 remains untouched
(Jacob's decision). The Tier-2 final gate is not
started. The close's hosted evidence (bundle +
three-OS CI on the close commit) and the
round-boundary hygiene record live in the audit
dir (`round31-close-status.md`), per the standing
gate.
