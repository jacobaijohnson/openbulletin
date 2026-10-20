# Audit round 30 — synthesis (2026-10-16)

HEAD audited: `f03d47e8f4d9dba0b31742dcb982b1203b6f61cb`
("Health-fix coordinator close: H-A..H-J verified, 896
tests, clippy clean, fuzz byte-identical"). Base for
diffs: round-29 close `6899603`. All three lanes
read-only and blind; repo `git status` clean at every
lane's close, HEAD unchanged. Lane scratch:
probe30v (verify), probe30r (review), probe30 (red
team); outputs under `round30-out/`, with the verify
lane's in `round30-out/verify/` (see incidents).

**Round 30 verdict: 0 BLOCKER / 1 MAJOR / 9 MINOR —
the exit bar is NOT met.** The audited range is the
health phase, and its central claim survives every
lane: no date reading moved. The verify lane's dual-
tree fuzz attribution is airtight (below), the red
team's 1.87M-line full-state differential dump is
byte-identical across the range, and its panic hunt
(300k soup iterations, 3,013 .obd mutations) found
nothing. The one MAJOR was found independently by
two blind lanes in the journal peer machinery the
health phase hardened (F1). Separately, and gating
everything below, hosted CI has been red on macOS
since H-D; that incident has its own section and its
own first batch (CI-FIX), ahead of every round-30
fix batch.

Verify lane: **PASS, 0 / 0 / 1 observation.** Every
standing claim re-run fresh: suite **896 passed / 0
failed** (`SUITE_EXIT=0`), clippy exit 0, booklet
**31,717 / 34,977** byte-exact. Test-body account
matches the health close exactly (per-gate counts
862→864→878→884→888→888→889→890→890→894→896→896;
38 added named pins, 4 removed = the ob-text v0
estimator tests H-F deleted with the API, named in
its commit and the close; all 228 contribution test
bodies token-identical modulo relocation; the 14
changed non-contribution bodies are mechanical batch
adaptations, no assertion value changed anywhere).
Fuzz bins as claims: verdict 40,000 lines, 0 panics,
0 violations of I1–I3; composition 497,024 lines, 0
panics, governed resurrections **943** with the choir
mirror on every RES line. The two stdout diffs
against the round-29 logs (verdict I4/I4b counts,
one comp soup count) are attributed by dual-tree
re-run: the same bins on `6899603` produce output
byte-identical to the `f03d47e` runs, so the deltas
belong to the round-29 batches, and across the
audited range the fuzz output is byte-identical.
Rights bins byte-identical to round 29 (route
agreement 80,412, 0 safety flips, 19 sequences
license-only). Loader held (one observation, F10).
Health-restructure claims 6a–6d all PASS (bins
compile against the narrowed surface; `read_capped`
single-sourced; weekday twins recorded on both
sides; the H-A fence present and pinned; exactly one
unsafe block workspace-wide).

Review lane: **1 MAJOR / 4 MINOR**, 28 items checked
and cleared (all three H-H pure moves with shingle
deltas fully accounted; `read_capped` judgments at
all six doors; H-J image cache and field table; H-B
door gate; H-D worker ordering and Stored↔Deflated
compatibility both directions; H-I runner semantics
and fail-closed helper; H-A fence read line by
line).

Red team lane: **0 BLOCKER / 1 MAJOR / 5 MINOR.**
Headline held: no reading moved (full-state dump,
~64,700 corpus lines → 1,867,397 output lines,
byte-identical between `6899603` and `f03d47e`).
Rights M8 axes held with 0 flips (102,736 route
checks, 21,952 associativity, 195,720 permutations).
H-A fence held on 18 shapes; `read_capped` held
against an infinite reader; loader storms refused
honestly.

## The CI incident (gating; read first)

Hosted CI on main has been **red on macOS since
c5f30e1 (H-D)** and through the health-fix close
`f03d47e` (run 39355115950: macos job 118408011008
failure; windows and ubuntu jobs success). Exactly
one test fails:
`settings_tests::journal_retirement_respects_the_owner_token`,
panicking "a live foreign owner's journal is not
retired". Local verification and the Windows bundle
were green throughout, which is how this shipped
unnoticed: every health batch from H-D onward, and
the close itself, was called verified on local
evidence while a hosted platform job was red. That
was a verification gap, recorded here in plain
language; the close's local claims were all true,
and they were not the whole evidence.

Root cause, proven from the code (not a summary):
`peer_provably_gone` (crates/ob-app/src/main.rs)
treats a missing `/proc/{pid}` as proof the peer is
dead. /proc does not exist on macOS, so every pid —
including a live foreign owner's — reads as provably
gone, and `retire_journal_from` retires a live
owner's journal. Linux passes because /proc exists
and the exe-identity check recognizes the app's own
test binary; Windows passes because its arm is
unconditionally conservative. A second site shares
the root: `peer_instance_alive_from` requires
`/proc/{pid}` to exist on unix, so a live peer is
never detected on macOS.

**New standing gate (from Jacob's steering,
2026-10-19):** no close is verified on local evidence
alone. Every close from now on cites the hosted CI
conclusions for all three OS jobs (macos, windows,
ubuntu) alongside the local suite and the bundle.

## F1 (MAJOR) — The journal peer fence is defeated by the startup lock overwrite (review MAJOR-1; red team RT30-1; two-lane corroboration)

Instance B's startup overwrites the instance lock
with its own token unconditionally
(crates/ob-app/src/windows.rs:979-980), even when a
peer is alive. B's ordinary save then retires
instance A's live journal through `mark_clean` →
`request_journal_retire` → `retire_journal_from`:
with the lock now naming B,
`journal_retirement_allowed(meta=A, own=B, lock=B,
gone=false)` is true — the "lock no longer names
the owner" rule fires because B itself replaced the
name. A's tick never rewrites the journal (its
in-memory snapshot still matches), so A's unsaved
work silently loses crash recovery. The pinned case
(lock still naming A) refuses correctly
(main.rs:6451). Not a regression against pre-H-C
unconditional clearing, but a real hole in the
hardened-peer-detection claim H-C made. Witnesses:
review report §MAJOR-1 interleaving; red team
`round30-out/r2-extra.log` (the rule copied verbatim
from `f03d47e` source, evaluated on the witness
inputs). Disposition (R30-A): startup must not take
the lock from a live peer — when the lock names a
live foreign owner, the starting instance leaves it
in place and its own journal stewardship rides the
token rules as designed. The batch rides CI-FIX's
platform liveness primitive (a trustworthy "live"
answer is its precondition, and on macOS that
answer does not exist until CI-FIX lands).

## F2 (MINOR) — Font-scan memo serves stale facts after a same-size, timestamp-preserving replacement (review MINOR-1; red team RT30-3)

After a font file is replaced with same-length
bytes and its mtime restored (`touch -r`), the H-J
memo returns the old family facts and the old
SHA-256 while a fresh inspect of the on-disk bytes
disagrees. A control with a changed mtime reads
correctly. Witnesses: `round30-out/r2-fontprobe3.log`
(review), red team §RT30-3. Disposition (R30-C):
the memo key must cover content identity, or the
memo must fall back to a cheap content check on the
size+mtime hit; either way the stale-facts witness
becomes a pin.

## F3 (MINOR) — ob-format's sync_and_rename leaves its temp file behind on failure (review MINOR-2)

`sync_and_rename` (crates/ob-format/src/lib.rs:1305)
does not remove its `.obd.tmp` / `.obdi.tmp` when
the sync or rename fails; the app-side twin from
the same batch (H-C) cleans up. Litter only; the
target stays whole. Disposition (R30-C): match the
twin's cleanup, with a failure-path pin.

## F4 (MINOR) — Provenance truth in the H-F funnel, two mechanisms (review MINOR-3; red team RT30-2)

(a) The starter-set collection note claims per-piece
records come from the manifest even in the
missing-row / unparseable fallback, where they do
not. The piece-level fallback itself is truthful,
and the shipped manifest is verified complete
(12/12 rows, public domain not flattened to CC0);
unreachable under the suite pin today, but the
words overclaim. (b) The funnel keys per-piece
provenance by basename: duplicate basenames in an
imported collection collide in `provenance.json`
(the first piece ends up carrying the second's
record, the second recordless), and
metafile-converted pieces lose their records —
against the funnel's own "keyed by the name the
piece lands under" comment. The starter set is
unaffected (unique flat names). Disposition
(R30-B): make the words true in (a); key provenance
by the landed name the comment already promises in
(b), with pins for the duplicate-basename and
converted-piece witnesses.

## F5 (MINOR) — Sanitized license-note names can collide (review MINOR-4)

The H-I note-name sanitization squashes distinct
catalog family names onto one file name (a dashed
family squashed vs a real family of the squashed
name), silently overwriting the earlier note. The
surviving note's contents still name their own
family and license, so the harm is a lost note, not
a wrong attribution. Disposition (R30-B): make the
mapping collision-free (or refuse loudly), with the
twin-family witness as a pin.

## F6 (MINOR) — Zip duplicate-name shadowing against the entry-count caps (red team RT30-5)

The zip reader collapses duplicate entry names
(a crafted 3-entry zip reads as len 2, last copy
wins), so the 512 entry-count caps count unique
names, and a duplicated `document.json` is read
from its last copy. No cap is exceeded in bytes —
each read still goes through the capped readers —
but the counting basis is not what the cap claims.
Disposition (R30-C): count raw entries, or refuse
duplicated names in documents the loader accepts;
the crafted zip becomes a pin either way.

## F7 (MINOR) — The contribution parser has a ~cubic cliff on repeated-date lines (red team RT30-4)

Measured identical on both trees (standing
behavior, not a moved reading): a 3.4 KB
repeated-date contribution parses in ≈8–9 s; a
34 KB one exceeds 120 s. A pasted contribution can
therefore freeze the app for a lay user.
Disposition (R30-D): profile the repeated-date
path, remove the superlinear term, and pin the
witness sizes at a sane bound; before/after numbers
recorded in the batch, per the H-J measurement-led
pattern. No reading may move (the fuzz bins are
the batch's net, as for every contribution batch).

## F8 (MINOR) — The health close's H-D record overstates (red team RT30-6)

The health-fix close records H-D (journal worker)
as a pure speed win. The journal crash window did
grow: from tick-atomic to tick + worker latency.
The growth is small and bounded, and no batch
claim about correctness depends on it, but the
record should have said it. Disposition: this
synthesis is the correction of record; the
health-audit document is not rewritten (its close
stands as written, with this note as its addendum).

## F9 (MINOR, observation) — Loader per-candidate scan cost rose ~2–3× across the health phase (verify claim 5)

Shared-CD 10,000-candidate load: 96.30 ms at load
2.77, vs round 29's 35.06 ms at load 13.6; scaling
stays linear and every refusal behavior is
unchanged. Consistent with H-B/H-G adding per-entry
cap checks. No close claimed a timing bound, so
this grades as an observation. Disposition:
recorded, not fixed; the numbers stand here so a
future round can see if the cost moves again.

## Environment incidents (lanes' records, condensed)

The VM restarted three times during the round
(~02:16, ~02:51, ~03:15 UTC). The first review and
red team attempts were killed before writing and
were re-dispatched as replacement attempts, which
reused predecessor artifacts only after independent
validation (review: all 16 per-commit patches
byte-matched fresh `git show`; one predecessor
analysis output rejected as a tokenizer artifact).
The exec-delivery layer repeatedly delivered other
sessions' outputs as a lane's own results; the
verify lane caught it and trusted only files
re-read from disk — every cited number in all three
reports comes from a completed on-disk log with the
process's own exit code. A `target/tmp` deletion
mid-audit and a filled 512 MB `/tmp` tmpfs each
cost one suite run, adjudicated environmental and
re-run to completion on the workspace disk. The
verify lane's final report reached disk complete
(its closing message to the coordinator did not
survive; the report is the record).

## Fix batches (dispositioned; CI-FIX gates all)

- **CI-FIX (first, ahead of every batch below):**
  one platform liveness primitive for the journal
  peer machinery — `kill(pid, 0)` via libc on unix
  (ESRCH = provably dead; success/EPERM = alive;
  anything else = unknown), the /proc identity
  refinement kept only where /proc is readable,
  unknown never reads as gone at either call site
  (`peer_provably_gone`, `peer_instance_alive_from`).
  The existing test's assertions are not weakened
  and nothing is cfg'd away. `libc 0.2` (already in
  the workspace lock via ob-runner) joins ob-app;
  the single unsafe call takes the ob-runner
  pattern (scoped allow + SAFETY comment) under the
  workspace's `unsafe_code` deny. Verified only
  when hosted CI on its commit concludes success on
  macos + windows + ubuntu, on top of the local
  gates.
- **R30-A:** F1 — startup lock stewardship (rides
  CI-FIX's primitive). Pins: the two-lane witness
  interleaving, plus the existing token pins green.
- **R30-B:** F4 + F5 — provenance and note-name
  truth (landed-name keying, collision-free note
  names or a loud refusal, fallback words made
  true), each witness a pin.
- **R30-C:** F2 + F3 + F6 — memo content identity,
  temp-file cleanup on the failure path, zip
  duplicate-name counting/refusal, each witness a
  pin.
- **R30-D:** F7 — the repeated-date cliff,
  measurement-led, fuzz bins as the no-reading-moved
  net.
- Recorded, not batched: F8 (corrected by this
  synthesis), F9 (numbers on record), HF-12 (the
  `Result<_, String>` error architecture — Jacob's
  decision, untouched by this round).

After the batches: the round-30 coordinator close
cites, per the new standing gate, the local suite +
clippy + booklet, the Windows bundle, **and the
hosted CI conclusions for macos, windows, and
ubuntu** on the close commit. The exit bar for the
loop is unchanged: a full round at zero BLOCKER +
zero MAJOR. This round does not meet it (F1, and
the CI red CI-FIX exists to close).
