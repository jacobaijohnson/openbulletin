# OpenBulletin convergence round 48 — synthesis

- Audit over code tip `92451ad` (round-47 close record `7ac0456`).
- Lanes (blind, read-only): verify, review, red team. Full reports:
  `~/workspace/openbulletin-audit-wave3/round48-out/verify/report.md`,
  `.../review/report.md`, `.../redteam/report.md`.
- **Deduped tally: 1 BLOCKER / 0 MAJOR / 4 MINOR. The exit bar is NOT
  met; the Tier-2 final gate is NOT triggered.**
- No fixes are landed by this synthesis. Fix phase is NOT dispatched
  by this document; it is reported to the parent first.

## Legs of record (unchanged from round 47)

Hosted CI on all three OS legs, desktop runner build, Windows bundle,
fuzz-as-claims, booklet baseline. The VM full suite is a provisional
pre-close smoke run, never cited as a leg. A smoke pass never cures a
red hosted leg; a smoke result never blocks a batch whose hosted legs
are green.

## Standing-record correction (carried)

The fuzz 40,000 / 497,024 figures are the bins' INTERNAL CASE COUNTS
(`total_lines` and `D_TOTAL`), not stdout lines (the stdouts are 16 and
87 lines respectively). The byte-identical claim is the md5 match. The
"lines" wording is not repeated.

## Verify lane: FAIL on legs of record (Ubuntu red), everything else PASS

- Hosted on `92451ad`, re-confirmed via `gh run view` AND the Actions
  API (jobs + per-attempt jobs + per-attempt log zips). Sweep of all
  runs on the tip returns exactly 3 runs, no others.
- **CI run `41278644089`, attempt 1, Ubuntu FAILED:** job
  `124765850794`,
  `settings_tests::r42e_four_claimant_races_over_a_dead_lock_stand_single_winner`,
  ob-app bin 240 passed / 1 failed, panicked at
  `crates/ob-app/src/main.rs:8191:14` with `spawn live peer: Os {
  code: 26, kind: ExecutableFileBusy, message: "Text file busy" }`
  (ETXTBSY) at the peer-spawn expect.
- **Attempt 2, Ubuntu FAILED again, different test, same ETXTBSY
  signature, after the single bounded retry:** job `124769309344`,
  `settings_tests::r47a_audit_restore_links_and_never_overwrites_an_occupied_lock`,
  bin 240 passed / 1 failed, same line 8191:14, same ETXTBSY message.
  No third attempt taken or claimed.
- macOS + Windows CI legs success on both attempt views (precision
  note: the attempt-2 log zip contains only the Ubuntu failed-job
  re-run, so macOS/Windows were not re-executed on attempt 2; no
  failure appears for them on either view).
- Desktop Runner Build `41278644067`: success, attempt 1. Windows
  bundle `41280814193`: success, attempt 1, gate + bundle green,
  artifact `openbulletin-windows`.
- Booklet exact: 31,717 / 34,977, EXIT=0.
- Fuzz byte-identical: verdict `797758443d101f9339fdd778e14af430`,
  composition `e8b2e73b117c7013d867419da8fb0ab7`, rights
  5083 / 1024 / 5275, safety_flips=0, all four bin exits 0, stderrs 0
  bytes.
- Cold clippy: `cargo clean` then `cargo clippy --workspace
  --all-targets -- -D warnings`, EXIT=0, 0 warning/error lines
  (5m51s). A first cold attempt cut off by an aborted turn left a
  partial log with no exit code; it was discarded, not cited.
- VM suite, labeled smoke (not a leg): EXIT=0, scripted tally 1348
  passed / 0 failed over 30 bins, ob-app 241/0, no FAILED / panic /
  ETXTBSY lines. This smoke pass does not cure the hosted Ubuntu red.
- Round-47 batches stayed landed (git, not the brief): R47-A commits
  `ce469d5` + `1c85940`, R47-B `e387ad8`, R47-C `de6022c`, R47-D
  `56b947a`, R47-E `92451ad`, all ancestors of HEAD; targeted pins
  7/7 EXIT=0 (r47e 2/2, r46a 2/2, r42e_claim_loop 2/2,
  r47a_serialized 1/1, scouting only).
- Rights safety axes (M8): PASS. Route invariance safety_flips=0;
  order invariance flip counts 0; variation confined to
  license/credit text as counted.
- R47-B stays provisional: no keyboard walk claimed (no GUI run
  available). R46-C two-holder event not re-litigated; fix stayed
  landed and pins bite.

## F11 — BLOCKER for tip verification, NOT a product defect (red team)

Carried from round 47, still open. Every failure is in test setup
before any claim runs; no lock assertion failed anywhere in this lane.

**Reproduced discrimination (matches the round-47 record, no
contradiction):** parallel trio (`r42e` filter, default threads) 5/11
pass, 6/11 fail (record: 6/11 pass across btrfs+tmpfs — within the
flake's own variance); serial trio 3/3; solo four-claimant 5/5; pairs
12/12. All 6 parallel failures at `main.rs:8191:14`, code 26, setup
only. New datum: the 6 failures split 3/3 between the four-claimant
test and the claim-loop test; with the hosted attempt-2 event (a third
test, same spawn expect, same error), the transient is now observed in
three different tests. It is not test-specific.

**Traced trio — strace suppresses the flake; passing trace is
normal:** 12/12 traced trios passed, zero ETXTBSY, zero failed execve.
At the untraced pass rate of 5/11, 12 straight passes has probability
~6e-5 if strace changed nothing, so the trace very likely serializes
the fork/exec/copy window the flake needs. That is itself evidence:
the contention window is narrow and timing-sensitive, closed by
syscall-level serialization — consistent with a race at execve time,
not with a persistent state (a stale file, a leaked peer, a bad
permission would all survive strace and still fail). The passing trace
shows the normal sequence: per spawn, the thread opens its own nonced
dest `O_WRONLY|O_CREAT|O_TRUNC|O_CLOEXEC` (mode 0100755), then execves
that same path ~1 ms later; no other thread opens that path.
Precise inconclusiveness: no traced run failed, so the trace never
observed a failing execve and cannot name what a failing execve
contends with.

**Probe-print fallback — 7 F11 events, fd scan empty, retry datum
(n=1 for delayed):** probe in `/tmp/ob-probe` only (repo untouched;
probe binary verified distinct by an `R48-PROBE` marker string; build
artifacts went to lane scratch via per-invocation `--target-dir`,
since the 512 MB `/tmp` tmpfs cannot hold the ~500 MB test binary).
On spawn error the probe printed the failing path, every
`/proc/self/fd` entry equal to that path, the file metadata, and a
same-path re-spawn outcome (record only — the test still fails
afterwards, so nothing is masked; a probe child that starts is killed
+ reaped). All 7 events: fd scan `[]`, file complete (35336 bytes,
the sleep size, mode 755), copy times 83 µs–1.8 ms with no correlation
to failure. Immediate same-path retry succeeded in 4/7; of the 3
immediate failures, only one received a delayed attempt, which
succeeded at +1 ms. Delayed-resolution evidence is therefore thin
(n=1); no claim is made that a backed-off retry is proven.
- OUT (strongly disfavored): a still-open in-process writer at failure
  time — with the honest caveat that a writer opening and closing
  entirely in the sub-ms window between execve attempt and scan would
  escape it, but no shared-path write exists after the R47-E nonce.
- OUT: a partial or corrupt copy; a fast copy fails too (83 µs), so
  "slow copy" is not the trigger.
- OUT: claimant-index- or test-specificity (failing contexts: the
  must_judge sole peer, four-claimant claimants 1 and 3, the mirror
  test's peer).
- IN: a narrow, variable-duration busy window at execve, roughly
  sub-ms to low-ms.
- NOT ESTABLISHED: the kernel-level mechanism. Remaining candidates:
  (a) kernel/filesystem writecount visibility lag (close on one thread
  not yet visible to execve on another under load — needs
  kernel-level evidence this lane does not have); (b) an out-of-process
  holder (implausible — no other process knows a nonced path — but not
  excluded by the in-process-only fd scan); (c) execve racing with
  something in the child pre-exec path (Rust spawn internals) — not
  discriminated by this probe. None is cited as a cause.

**Bounded-spawn-retry candidate — recommendation: BOTH.** A retry
masks by construction and does not explain; it must never be cited as
closing F11. It is nevertheless a legitimate mitigation, not a
cover-up: test-only (`mod settings_tests`, `#[cfg(test)]` — no
production code spawns copied executables, verified by grep), failures
are setup-only so a retry cannot turn a failing claim into a pass, and
deterministic breakage (permission, missing binary, bad path) fails
identically on every attempt, so a bounded retry cannot hide a genuine
regression. Conditions, all of them: test-only inside
`spawn_live_peer_named` only; retries ONLY on ETXTBSY (code 26), every
other spawn error failing immediately; bounded attempts WITH backoff
(immediate-only resolves 4/7, not good enough — the probe's 0 / 1 / 5 /
25 / 100 ms delays are a reasonable starting shape with a small cap,
e.g. ≤5 attempts total, proposed not asserted, carrying no timing
assertion); every retry logged so CI logs still show the transient
(the canary stays audible; a silent retry would be masking in the
strong sense); F11 stays OPEN with the named next check below even
after the retry lands; validation batch before the retry is trusted
(11+ parallel trios green AND retry log lines showing transients
absorbed — green alone would be closing-by-absence again). This
verdict is therefore NOT "F11 reduced to a candidate": the
delayed-retry evidence is n=1 and no retry-enabled validation batch
exists yet, so the retry is not proven sufficient to reduce F11 to it.

**Named next discriminating check (single, decisive as far as a probe
can be):** a copy-serialization probe in a `/tmp` copy — guard the
copy+spawn sequence in `spawn_live_peer_named` with a process-wide
static Mutex so no two test threads copy or spawn concurrently, then
run 11 parallel trios. If ETXTBSY disappears across the batch, the
mechanism involves concurrent copies even to different paths (a
filesystem/kernel-global effect, not a same-path write) — and the hunt
moves to the copy path itself (e.g. `copy_file_range`/reflink behavior
under concurrency). If it persists under full serialization, copy
concurrency is ruled out and the hunt moves to execve/fork internals
or an out-of-process holder. Complementary check if the Mutex probe is
inconclusive: a per-thread copy open/close timestamp log to close the
sub-ms fd-scan escape hatch.

**Hostile re-checks — none bit (and filing from a non-biting attempt
would be fabrication, so none is filed):**
- R47-A serialization stayed landed: `acquire_instance_lock_from`
  (`main.rs:4464`) holds `_claim_guard` across the whole claim loop
  (mutex at `:3910`, guard at `:3919`); the production-path pin
  `r47a_serialized` passes on the current tree and in the probe
  binary. Pin-design trap re-confirmed EMPIRICALLY: a probe test
  mirroring `r47a_serialized` with raw fs ops (direct lock-file write
  naming a live peer pid, never through `acquire_instance_lock_from`)
  lands on the fixed tree — raw ops bypass the serialization, so any
  pin must go through the production path. (Probe artifact disclosed
  in the red-team report: a doubled `#[test]` attribute registered
  the mirror test twice in the probe binary only; fixed, probe binary
  re-verified as repo test list plus the one mirror test, mirror
  re-run clean. Repo tree and repo binary never affected.)
- WMF converter (R47-D F9): `scratch_file_name` (`main.rs:5197`)
  reduces to the last segment and refuses empty/`.`/`..`; the
  converter joins only the reduced name into a fresh exclusive
  `fresh_scratch_dir("wmf")`; each conversion gets its own dir.
  Pins pass (`r47d` filter 2/2).
- Print-copy temp site: R47-D's record DID fix it (pid-only to
  pid+nanos, `main.rs:2529`; current tree matches). The
  `fresh_scratch_dir` counter+exclusive-create shape was R47-D's fix
  for scratch dirs, while the print-copy fix was deliberately
  pid+timestamp for a file that must stay while the viewer holds it.
  Empirical support: 2,000,000 tight-loop `SystemTime::now()` nanos
  readings produced ZERO consecutive duplicates. The residual `.tmp`
  symlink concern is bounded by the nonce (planting requires predicting
  the exact pid+nanos path; the final rename replaces a symlink rather
  than following it). No collision demonstrated.
- Docstring overclaim (`main.rs:359`, `web_page_path`): the `-web`
  suffix branch returns the pick itself, which IS the `-web` file by
  construction, not the source bulletin (`.obd`). Claim holds in its
  domain.

## Review lane: 4 MINOR, no BLOCKER, no MAJOR

- **R48-R1 (MINOR, batch):** R47-C missed two settings-save error
  sites — `main.rs:2689` (PDF-purpose radio) and `:2783` (printer
  learning) — still lacking the next-step/session wording the four
  `windows.rs` sites got. Same-class miss, not a failure of the R47-C
  re-check.
- **R48-R2 (MINOR, batch):** H2 genuine — the Start inbox list
  (`main.rs:1712-1745`) shows name + fingerprint + Open only, no
  landing state. Fix placement is the Start inbox list, not the
  Announcements editor; derivation already exists (`applied_claim` at
  `contribution.rs:1806`, `SlotTarget::KeptWithPart` +
  `unlanded_sentence` at `contribution.rs:905-918`).
- **R48-R3 (MINOR, batch, narrowed):** H4 mostly already landed (plain
  staleness phrasing at `main.rs:2512` and `check.rs:1112-1119`); only
  the bare 8-hex fingerprint in the inbox list (`main.rs:1736-1741`)
  is left unexplained. Batch with R2 (same UI region): keep the hex,
  gloss it.
- **R48-R4 (MINOR, defer):** clamp-panic invariant class in
  `keyboard_resize_rect` (`main.rs:5010-5033`) and drag/move paths —
  pre-existing, not an R47-B regression; no crafted-file trigger
  proven, so deferred pending a pin.
- **Glossary micro-gap:** Start uses both "shape" (`main.rs:1867`)
  and "template" (`main.rs:1834`) with no distinguishing sentence —
  fold one line into the wording batch. Shape commitment, print-choice
  load, and step preview are already addressed (preset labels with
  dimensions, purpose-labeled print choices, `sheet_preview_ui` at
  `main.rs:2471`); no user-visible "applied" string exists (review
  already says "already in the bulletin"), so that watch item needs no
  action. H6 not raised, per instruction.

**Carried items judged by review:**
- R47-A, R47-C Fix buttons, R47-D six temp sites: all confirmed
  landed with pins present. Precision note only: print-copy's `.tmp`
  sibling uses `File::create`, not exclusive creation — the property
  holds via the unpredictable pid+nanos base path (see the red team's
  independent 2M-reading support above).
- R47-B code path judged sound (Alt+arrows resize, Pages editor
  sharing the menu's exact fields/store/apply path); walk still
  outstanding, not a blocker on code merits. Menu-key
  non-implementability cited from the R47-B record, not re-verified
  against vendored egui.
- R47-F: STAY DEFERRED, with corrected framing — `main.rs`'s 16,853
  lines include an ~11.2k-line `settings_tests` module; production
  `main.rs` is ~5.4k lines and `windows.rs` (9,085 lines, 0 tests) is
  the bigger production file. No round 46–47 bug traces to file size.
- HF-12 second half: CONDITIONAL batch candidate — redesign shape now
  clear (typed-error enum following the proven `OpenDoorError` /
  `FromJsonError` pattern; `parse_xml` call sites enumerable at
  `idml.rs:181, 253, 625, 656`). Current substring match
  (`idml.rs:260`) is functionally safe, so no user-visible defect;
  batch only if fix phase has room, else carry.
- H1: NOT a finding pending the live check — code supports the
  refined rule (clone at `ob-layout/src/lib.rs:471`, frames never
  rebuilt; `goto_step` doesn't rebuild; only the two-click-armed shape
  pick replaces the document). Wording in Pages/Next Sunday report is
  owed only if the live check holds.
- F11: code-read note only, verdict deferred to red team (nonce
  triple, drop-reap, unique spawn paths all present).
- Review deliberately did not check: no GUI run (keyboard walk, H1
  live check, rendered UI); no cargo tests run by that lane — a live
  red-team parallel-rate run observed and left untouched, since
  scouting could contaminate its measurement; no full-suite verdict
  (verify's job); CI run `41278644089` not re-verified firsthand by
  review (verify's job, done there).

## Process-correction note (coordinator)

The round-48 brief described the running cargo process as Encore's.
The review lane observed it was the red team's own OpenBulletin rate
run; the coordinator sent a correction to the verify lane (touch no
running process; verify ownership from command line + cwd; do not
contend with a measurement run). Verify obeyed: the red-team rate run
was already gone on its `ps` check, a different red-team probe build
was in flight (disjoint probe-target, `--no-run`), so heavy runs were
deferred until it finished and fresh scans showed no cargo; cold
clippy and smoke each started only after a `NO_CARGO_PROCESSES` scan.
No process was signalled, killed, or assumed dead on appearance.

## Batch dispositions (not dispatched)

- **R48-A FIRST (F11 follow-up):** (a) the copy-serialization probe
  (process-wide Mutex around copy+spawn in a `/tmp` copy, 11 parallel
  trios) to settle the mechanism direction; (b) the bounded,
  ETXTBSY-only, logged, backed-off spawn retry as mitigation, under
  all six red-team conditions, validated by a green trio batch that
  ALSO shows absorbed-transient log lines. Order: probe first for
  direction is cleaner, but the retry mitigation may proceed in
  parallel — it does not depend on the probe's outcome. **F11 stays
  OPEN either way** until the mechanism is named or the serialization
  probe (or its successor) settles it. The retry landing is recorded
  as mitigation, never as closure. The Ubuntu leg stays red until a
  tip goes green on it; no re-run-and-forget closes it.
- **R48-B (wording):** R48-R1 (two missed settings-save sites) +
  R48-R2 (H2 inbox landing state) + R48-R3 (bare inbox fingerprint
  gloss) + the one-line shape/template glossary sentence.
- **R48-C (conditional):** HF-12 IDML second half (typed-error enum
  following the proven pattern; `story_paragraphs` on TOO_DEEP) — only
  if the fix phase has room after R48-A and R48-B, else it stays
  carried. Not a blocker.
- **Deferred, explicitly:** R48-R4 (clamp-panic class, pending a
  crafted-file pin); R47-F monolith extraction (no bug traces to file
  size; corrected framing above).
- **Outstanding, needing a GUI run or a human:** R47-B keyboard walk;
  H1 live check (hand-edit a page, go back a step, start next Sunday,
  check the edit on screen and in print). Neither is a blocker on code
  merits; neither is claimable until the walk/check happens.

## Round 48 close (fix-phase coordinator)

- Final code tip `0a1d76e`, suite **1352 passed / 0 failed**
  (1348 baseline + 2 R48-B tests + 2 R48-C tests). Batches, each
  verified firsthand from disk and the gh API before the next was
  dispatched: R48-A `8d6172b` (F11 probe + bounded retry mitigation),
  R48-B `0526d38` (wording: R1+R2+R3+glossary line), R48-C `0a1d76e`
  (HF-12 IDML half, landed whole). Batch records at
  `~/workspace/openbulletin-audit-wave3/round48-out/coord/r48{a,b,c}-record.md`
  (that directory is not inside any git repository, so the records
  are delivered alongside the commits, not committed — same as
  rounds 47–48 lane records).
- **Legs on the final tip, verified firsthand by the coordinator:**
  hosted CI run `41298381406`, ALL THREE OS legs green, attempt 1
  each (Ubuntu job `124823974843`, macOS `124823974993`, Windows
  `124823975011`); Desktop Runner Build `41298381408`, success;
  Windows bundle `41303485462` (dispatched at the close), gate +
  bundle jobs green. No hosted flake event on the final tip.
- Fuzz-as-claims (coordinator close run, logs in
  `round48-out/coord/close-logs/`): bin sources md5-unchanged from
  the standing values; all four bins exit 0, stderrs 0 bytes;
  byte-identical — verdict `797758443d101f9339fdd778e14af430`
  (16 stdout lines), composition `e8b2e73b117c7013d867419da8fb0ab7`
  (87 lines), rights `327cef3c7954c6f7950ed405a289e44b` and
  `8e7c11b4e241336f53f4e2030a705d66`; counters exact
  (total_lines=40000, D_TOTAL=497024, rights 5083 / 1024 / 5275,
  safety_flips=0). The 40,000 / 497,024 figures are the bins'
  INTERNAL CASE COUNTS, not stdout lines.
- Booklet baseline (coordinator close run): exact, 31,717 imposed /
  34,977 reader, exit 0.
- VM suite: R48-C's pre-close smoke run `suite2.log`, 1352/0 over 30
  bins, reported as a smoke run only, never cited as a leg.
  Disclosure carried from the R48-C record: the smoke re-run was
  launched via nohup without a `.status` capture, so cargo's numeric
  exit code for suite2 was not recorded; completion is evidenced by
  the log reaching the final doc-tests bin, zero failure lines, and
  the 1352/0 tally matching hosted Ubuntu exactly. The close stands
  on the hosted legs, so this does not affect the verdict.
- **F11 STAYS OPEN — and so the exit bar is NOT met.**
  The Ubuntu leg went green on the R48-A tip (`8d6172b`) and stayed
  green through R48-C, but green on a retry landing is not a closure:
  the retry is mitigation, validated by 11/11 trios green WITH 6
  quoted absorbed-transient lines, and the probe verdict (ETXTBSY
  disappears under full copy+spawn serialization, 11/11) moves the
  hunt to the copy path itself. The mechanism is still unnamed, so
  F11 remains the round's open BLOCKER and the Tier-2 final gate is
  NOT triggered.
- Cache-policy disposition at this confirmed completion: disk 50% /
  76 GB free — far under both thresholds — so CARRY, no clean
  triggered. (Round-47 close: 47% / 80 GB.)
- Disk-ritual note: round48-out + round47-out kept; round46-out
  becomes sweep-eligible at this boundary — eligibility only,
  deletion waits for Jacob's approved list.
- **Carried to round 49:** F11 (copy-path hunt: copy_file_range /
  reflink behavior under concurrency, or its successor check);
  R48-R4 (clamp-panic class, pending a crafted-file pin); R47-F
  monolith extraction (deferred; corrected framing: production
  main.rs ~5.4k lines, windows.rs the bigger production file);
  R47-B keyboard walk and H1 live check (both need a GUI run or a
  human; neither a blocker on code merits, neither claimable until
  it happens).
