# OpenBulletin convergence round 47 — synthesis

- Audit over code tip `db4209b` (round-46 close record `3a387ad`).
- Lanes (blind, read-only): verify, review, red team. Full reports:
  `~/workspace/openbulletin-audit-wave3/round47-out/verify/report.md`,
  `.../review/report.md`, `.../redteam/report.md`.
- **Deduped tally: 2 BLOCKER / 4 MAJOR / 6 MINOR. The exit bar is NOT
  met; the Tier-2 final gate is NOT triggered.**
- No fixes are landed by this synthesis. Fix phase is NOT dispatched
  by this document; it is reported to the parent first.

## Leg change (adopted for round 47, stated here as the record)

Legs of record from round 47: hosted CI on all three OS legs,
desktop runner build, Windows bundle, fuzz-as-claims, booklet
baseline. The VM full suite is DEMOTED to a provisional pre-close
smoke run, never cited as a leg. Rationale: venue-based (2 cores)
plus repeated load-sensitive timing behavior on the VM suite. The
verify lane still RAN the VM suite but labels it "smoke (not a
leg)" and cites the hosted legs + fuzz + booklet as evidence. A VM
smoke failure is triaged (load-noise solo re-run vs real failure);
only a real failure is a finding, and a smoke result never blocks a
batch whose hosted legs are green.

## Standing-record correction (carried into all wording below)

The fuzz 40,000 / 497,024 figures are the bins' INTERNAL CASE
COUNTS (`total_lines` and `D_TOTAL`), not stdout lines (the
stdouts are 16 and 87 lines respectively). The byte-identical claim
is the md5 match: verdict `797758443d101f9339fdd778e14af430`,
composition `e8b2e73b117c7013d867419da8fb0ab7`, rights
5083 / 1024 / 5275, safety_flips=0. The "lines" wording is not
repeated.

## Verify lane: PASS on legs of record (0 findings)

- Hosted on `db4209b`, all attempt-1 success, re-confirmed via
  `gh run view`: CI `41056961522` (macOS/Windows/Ubuntu jobs all
  success), Desktop Runner Build `41056961555`, Windows bundle
  `41070168938`. No flake on any `db4209b` leg.
- Booklet exact: 31,717 imposed / 34,977 reader.
- Fuzz byte-identical on all four bins (probe sources at recorded
  md5s); rights counters reproduced exactly, safety_flips=0.
- Rights safety axes (M8): PASS under both merge routes and under
  permutation; variation confined to license/credit text.
- Cold clippy `-D warnings`: EXIT=0, 312 Checking lines, all 16
  workspace crates. (A warm run showing 0 Checking lines was
  recorded and NOT relied on.)
- Smoke (not a leg): 1328 passed / 2 failed over 16 binaries,
  EXIT=101. Both failures were ETXTBSY spawn flakes
  (`r40h`, `r45b`, both panicking at `main.rs:7539` with
  `ExecutableFileBusy`), solo re-run green 1/0 each plus the r42e
  control green: triaged load-noise, not a finding, not blocking.
  Cargo stopped at the first failing binary, so 1328/2 is NOT a
  full-suite total and is NOT compared to the hosted 1334/0.
- Method disclosures honored: the lane's first suite and fuzz
  attempts died in a VM restart; partial logs preserved, no verdict
  read from them; the re-runs were from scratch. A first solo
  attempt that ran 0 tests (bare suffix with `--exact`) was
  discarded; only the full-path solo re-runs are cited.

## Findings (deduped)

### BLOCKER

**F1 — Rename-step guard-to-rename gap (RT46-L1 family, uncovered
internal path).** Review B1 + red team RT47-L1, independently
re-verified in code by both lanes. In
`rename_instance_lock_to_displaced_from` (`crates/ob-app/src/main.rs`
at HEAD): guard re-read + liveness judgment at 3782–3786, then a
full `judge_displaced_instance_lock_at_claim_from` call at 3787,
`remove_file(&displaced)` at 3796, and only then the unconditional
`rename(lock, &displaced)` at 3797. A thread descheduled anywhere
in that span resumes to move whatever now stands at the lock path,
including a live winner that landed after the guard passed. The
diagnostic's statement-level witness (guard passes on dead S; M
claims; B resumes, judges displaced dead S, deletes S, renames
M→displaced; C creates in the vacancy; B's audit stands down too
late) is complete in production order and ends in two holders.
Corroboration on the current tree: diagnostic 4 two-holder events
in 60 concurrent runs; red team 5 two-holder events in 80
concurrent runs (2,400 rounds), including the exact CI shape
`[false, false, true, true]`, plus the solo rate 0 in ~1,179 rounds
(diagnostic) and 0 in 30 rounds (red team). The R46-A pins pass on
the same trees — they pause before the rename call and cannot
express the internal gap; pins passing and the CI event are
consistent, not contradictory.

**F2 — Converters C2/C3: unconditional restore renames overwrite a
fresh create.** Review B1 (as converters) + red team RT47-L2
(graded BLOCKER on its own). C2: audit restore checks lock vacancy
at 3842 (`metadata(lock).is_err()`) then renames displaced→lock at
3843; a create landing between check and rename is silently
overwritten after its creator already answered `true`. C3: judge
restore captures vacancy at 3902, restores at 3908/3915, with a
displaced read and a `kill` syscall between — a wider window, and
the vulnerable state persists a whole timeslice while a bad
rename's audit is pending, so every spinning claimant's judge can
race it. Fix direction: link-based restores, which fail rather
than overwrite when occupied, for both audit and judge restores,
ideally under the same serialization as F1.

### MAJOR

**F3 — Converter C4: displaced-deletion gap.** Review B1 (as C4) +
red team RT47-L3. The line-3787 displaced judgment and the
line-3796 `remove_file` are check-then-act: a live generation
landing at the displaced path between them (via another claimant's
bad rename or an audit refusal) is deleted unjudged, destroying a
live peer's generation and yielding two holders by return-value
counting. Fix direction: displaced judgment immediately before
removal at minimum, ideally under the same serialization as F1.

**F4 — Code comments assert a closure that does not exist.**
Red team RT47-L4; review B1 flags the same comment. The rename
step's docstring (3743–3745) says the audit "guards the one window
this step cannot close, the re-read→rename syscall gap, which holds
no production step boundary": both halves false as written (the
gap holds a full function call plus two filesystem operations, and
the audit is repair after the fact, not a guard — it cannot
un-return a `true`). The `displace_instance_lock_from` docstring
(3937) likewise says the audit "closes the window between that
re-read and the rename." Graded MAJOR because this exact wording
is what let the R46-A close be read as complete. Both docstrings
must be corrected as part of the race fix; no close record may cite
the audit as closing a check-then-act window.

**F5 — Second production anomaly: wins == 1 but the lock does not
name the winner.** Red team RT47-L5, observed in its own
concurrent run (batch 15, process 4, round 17): the pin's
`wins == 1` assertion PASSED, but the next assertion failed at
`main.rs:9272:13` — "round 17: the lock names the winning
claimant's pid", left `Some(95300)`, right `Some(95303)`. Exactly
one claimant returned `true`, yet at end of round (all claimant
threads joined) the lock named a different claimant, one that
returned `false`. The red team does NOT claim this is the F1
gateway (candidate traces sketched each restore the winner rather
than the false claimant) and does NOT dismiss it as a test
artifact: a `true` return followed by a lock naming someone else
is a contract violation by the pin's own terms. Fix-phase
requirement: trace this event before closing round 47's lock work;
do not fold it into F1 without a trace.

**F6 — Keyboard path narrowed: resize, frame menu, and numeric
placement remain pointer-only (provisional, pending the keyboard
walk).** Review M1 + red team §4.3, agreeing. The relay's blanket
claim is PARTLY FALSIFIED by code: Tab cycles selection
(`windows.rs:5207–5217`), arrow keys nudge the selected frame
(5253–5279), Enter enters editing (5217–5224), and the help line
says so (4237–4238) — so select/move are NOT pointer-only, even
though `KEY_ACTIONS` (`settings.rs:231`) lacks those actions (they
live outside it as canvas keys). What remains keyboard-unreachable
in code: resize (corner-drag only), opening the frame menu
(`secondary_pressed` only, 4910; `context_menu` 5654, no Menu-key
trigger found), and therefore the numeric place/size fields inside
that menu (5682, 5694–5695, 5712). Per the relay's own condition
this stays a carried observation, graded MAJOR provisional, NOT a
blocker, until a keyboard walk (human on a real machine, or a
future GUI run) confirms the rendered behavior. No GUI run was
available to any lane this round.

### MINOR

**F7 — Bare "Fix this" buttons (wording consistency).** Review m1
+ red team RT47-L6, code-confirmed. Four bare buttons at
`main.rs:2411` (goto step), `2431` (open Pages), `2435` (Fonts),
`2438` (inserts) against named neighbours at 2451–2458 ("Fix the
announcements", "Fix this in the Pages window", "Fix it in the
Pages window"). Fix direction: label each with its destination.

**F8 — Settings-save errors name the failure, give no next step
(one root cause, four sites).** Review m2 + red team RT47-L7,
code-confirmed. `windows.rs:8400` ("Your key was set, but the
settings could not be saved. ({e})"), `8437`, `8474`, `8488`
("Settings could not be saved. ({e})"). Fix direction: append one
next step to each string in a single pass (at 8400, state plainly
whether the key stays set for this session).

**F9 — WMF converter joins an unsanitized name into its scratch
dir.** Review m3. `main.rs:4774`: `dir.join(name)` in
`wmf_converter`, where `name` arrives from the caller and is handed
to LibreOffice. A name carrying a path separator would escape the
pid+timestamp scratch dir. Hardening note, not a demonstrated
exploit (current flows pass user-picked local names). Fix
direction: reduce `name` to its file-name component before the
join, reject empty results.

**F10 — Scratch/temp dirs keyed by pid only are predictable.**
Review m4. Repeated pattern
`temp_dir().join(format!("openbulletin-…-{pid}"))` with
`create_dir_all` rather than exclusive creation (e.g.
`windows.rs:6269, 6429, 7649`; `main.rs:3349, 5162`). On a shared
machine a pre-created directory or symlink at the predictable path
could redirect a scratch write. Local single-user church-office
threat, hence MINOR. Fix direction: per-run nonce (as the WMF
converter does) or exclusive creation, at least for import/template
scratch dirs.

**F11 — r46a pins: one parallel-run ETXTBSY failure, serial rerun
green (test-only transient, cause NOT established).** Review m5.
Parallel `cargo test -p openbulletin --bin openbulletin r46a`
failed 1 of 2 with ETXTBSY at `main.rs:7539`
(`spawn_live_peer_named`); `--test-threads=1` then passed 2/2,
exit 0. Cause not established (pid-keyed temp dirs are per-test
distinct, so a simple same-dir collision does not explain it; a
stale `sleep 30` peer or a concurrent lane's overlapping temp path
are unproven hypotheses). Do not cite the parallel failure as a
product defect, and do not cite the serial pass as proof the
transient is gone. This is distinct from the verify lane's smoke
ETXTBSY pair (triaged load-noise, not counted); the known ETXTBSY
family is not double-counted as two findings. Fix direction: give
each peer spawn a per-test unique dir/name (nonce, not pid alone)
and kill/reap peers in test teardown; if the transient recurs
under the serial shape, investigate as its own item.

**F12 — `main.rs` monolith: 15,689 lines in one binary file.**
Review m6. `windows.rs` is 8,934; workspace ~103,051 lines across
69 `.rs` files. Maintainability note, not a defect: the code is
heavily commented and test-pinned, but the claim loop — where
statement-level gaps like F1 hide — shares one file with app logic
and tests. Fix direction (fix-phase, not audit): extract the
instance-lock machinery into its own module with its tests,
keeping line numbers stable in the move or re-pinning them.
Dispositioned as deferred structural work, not a round-47 fix
batch (see batches).

## The deterministic witness: what exists and what does not

The red team executed a **mirror probe** (deterministic, bites):
`round47-out/redteam/probe_mirror.py` mirrors the rename step's
production order statement by statement — guard re-read passes on
dead S, deterministic pause, M's full claim interleaved (displace
S, hard-link create + re-read → `Created`), B resumes (displaced
judgment dead S → `NoLiveGeneration`, `remove_file`, unconditional
`rename`), C's create in the rename→audit gap, B's audit (displaced
M ≠ judged S, lock occupied → no restore, stands down too late) —
ending lock = C, displaced = M, `Created` returns M and C: the
pin's forbidden state, with complete generations throughout (so
R45-A correctly does not touch it). **This probe is a sequence
witness, not a production-code witness, and must be labeled as such
everywhere it is cited. Do not upgrade it in any downstream record.**

A deterministic PRODUCTION witness was NOT driven, for bounded
reasons stated by the red team: `ob-app` is a binary crate, the lock
functions are private to `main.rs` and not importable by an external
harness; no existing test pauses inside
`rename_instance_lock_to_displaced_from` (R46-A pins pause at
function boundaries); a mirror probe therefore cannot call the
production rename step with a pause inside it. Its B-side sequence
and M/C creates are script implementations of the production
statement order using the same atomic primitives (hard link +
re-read), not production function calls. The claim that production
contains that sequence rests on firsthand code reading (F1, with
line numbers), corroborated by the production concurrent runs —
not on the probe alone.

**Pin-design trap the fix phase must not fall into:** a mirror pin
that performs B's act with raw `remove` + `rename` will bite on
EVERY tree, including a fixed one, whenever the fix is external
serialization — the mirror bypasses the serialization. Such a pin
can demonstrate the current gap; it cannot be the regression pin
for a serialization fix. Valid regression-pin shapes, depending on
the fix: (a) if the fix makes the move itself conditional, the pin
goes through the production move primitive with M's production
claim interleaved after the guard (bites now, passes when the move
refuses or fails on a changed generation); (b) if the fix is
serialization, the pin must go through the production claim path
under that serialization with an interleaving attempt — e.g. a
source test with a test-only pause seam inside the serialized
section, or syscall interposition; a raw-ops mirror is then a
demonstration only, and the pin's pass on the fixed tree must be
shown through the production path; (c) syscall interposition
against the existing test binary (dynamically linked, not stripped,
with debug_info — verified on
`target/debug/deps/openbulletin-07d0af34795a1209`): an `LD_PRELOAD`
shim on the rename path, or a gdb breakpoint at the rename call
site, could pause a real production rename-step thread at its
rename syscall, let an external driver swap the lock file to a live
generation, resume, and assert the step does not move the swapped
generation. This was NOT executed (the shim would still need a
mirror-implemented driver claim, and identifying the rename-step's
rename among all renames without deadlocking the harness is
fix-phase engineering).

Evidence that would settle the witness in the fix phase: (i) the
pin above, run against the current tree (must fail with two
creates / a moved live generation) and against the fixed tree (must
pass); (ii) the production concurrent r42e shape going to zero
two-holder events under load, not merely a green solo run; (iii) a
trace of the F5 lock/winner-mismatch event, showing which
interleaving produced it and that the fix closes that trace too.

## Governing adjudication for the fix (recorded, not re-litigated)

1. **PREVENTION, not repair.** A post-hoc restore cannot un-return
   a `true`, so the bad rename itself must become unreachable.
2. **Moving the guard to immediately before the rename only shrinks
   the window (to remove+rename, two syscalls) and must NOT be
   accepted as a close.**
3. Real directions: serialize the claim sequence across claimants
   (e.g. a directory-level lock held across judge→rename→audit),
   or restructure displacement so the move itself verifies what it
   moves. Converters need the same treatment: link-based restores
   that fail rather than overwrite a fresh create (closing C2/C3 by
   construction); displaced judgment immediately before removal at
   minimum, ideally under the same serialization (C4).
4. Round 47 needs a pin mirroring the rename step's statement order
   that bites on the current tree and passes on the fixed tree,
   subject to the pin-design trap above. The r42e pin's assertion
   stays untouched.

## Carried items — disposition

- **R46-C CI attempt-2 event: stays on the record verbatim, NOT
  closed.** Run `40991486313` attempt 2, Ubuntu, round 7,
  `[false, false, true, true]`, sole failure of the run, R46-A pins
  passed on the same attempt; preserved log at
  `round46-out/coord/r46c-attempt2-ubuntu-logs.zip`. Not closed by
  the green attempt 3, never closes as re-run-and-forget. It is the
  CI instance of the F1 family, now with a production reproduction
  and a deterministic sequence witness behind it. (Red team
  re-verified the log firsthand: `0_test (ubuntu-latest).txt`,
  FAILED at `main.rs:9178:13` on the R46-C tree, ob-app bin
  `224 passed; 1 failed`; the run/attempt label itself comes from
  the R46-C batch record, not from inside the log text.)
- **R46-B twin-reading note: re-checked, stands as a note, not a
  finding.** The R46-B pin
  (`r46b_spaced_doorstep_weekday_first`,
  `crates/ob-extract/src/contribution/tests.rs:40848`) asserts every
  arm reads exactly as its clean same-seat twin, both routes:
  trailing weekday-first governs silent because its clean twin
  reads the same. The round-46 red-team trailing-SERVES observation
  was about the pre-fix hole, not the fixed twin reading; no fix by
  analogy with trailing should be applied to leading/mid-line (where
  the record stands) or vice versa.
- **Better-interface input:** (a) narrowed to resize + frame-menu +
  numeric placement, provisional pending the keyboard walk (F6);
  (b) and (c) are F7 and F8. The cleared items in the relay file
  were NOT re-raised by any lane.
- **HF-12:** slice 1 (round 44, open doors) + slice 2 `from_json`
  half (round 46, R46-G) confirmed landed (`FromJsonError` at
  `crates/ob-extract/src/lib.rs:95–134`, `from_json` returning it
  at 380–398, byte-for-byte Display tests at 1002–1063). The IDML
  second half is carried as a fix-phase candidate, not an audit
  finding: exactly one IDML site branches on error text today —
  `story_paragraphs` at `crates/ob-extract/src/idml.rs:251–260`
  (`Err(e) if e.contains(TOO_DEEP) => return Err(e)`, other damage
  reads empty; `TOO_DEEP` at line 62). Honest typing there needs a
  branch redesign, not a ride-along.

## Batch dispositions (fix phase — NOT dispatched by this synthesis)

- **R47-A (first): the rename-gap redesign + converters + pin.**
  F1 + F2 + F3 + F4 (docstring corrections are mandatory sub-items
  of this batch, not a separate batch) + the F5 trace requirement
  (trace before closing lock work; do not fold F5 into F1 without
  a trace). Includes the statement-order pin subject to the
  pin-design trap, and the three-part settling evidence (pin fails
  now / passes fixed; concurrent r42e shape to zero two-holder
  events under load; F5 trace closed). The r42e pin's assertion
  stays untouched.
- **R47-B: keyboard path** (F6): keyboard path to the frame menu
  for the selected frame (e.g. menu key / Pages-window control
  reusing the existing numeric fields) plus a keyboard resize step.
  The keyboard walk gates blocker treatment; the batch itself is
  not gated on the walk.
- **R47-C: wording pass** (F7 + F8): label each Fix button with its
  destination; add one next step to each of the four settings-save
  strings in a single pass.
- **R47-D: hardening** (F9 + F10): WMF name reduced to its
  file-name component; per-run nonce or exclusive creation for
  import/template scratch dirs.
- **R47-E: peer-spawn transient hardening** (F11): per-test unique
  peer dir/name (nonce, not pid alone) + kill/reap peers in test
  teardown; if the transient recurs under the serial shape,
  investigate as its own item.
- **R47-F (deferred structural work, not a round-47 fix batch):**
  F12 monolith extraction — a future structural pass, recorded
  here so it is not lost.
- **HF-12 IDML second half (conditional candidate):** the
  `story_paragraphs` branch redesign, at fix-phase discretion under
  the standing HF-12 terms (own batch letter, full batch
  discipline, placed so the audit/fix sequence is not displaced).

## Close section (to be appended by the fix-phase coordinator)

- [ ] R47-A…R47-E landed or dispositioned; R47-F deferred on the
  record; HF-12 IDML half dispositioned.
- [ ] Five legs re-verified on the fix tip (hosted CI ×3 OS,
  desktop runner build, Windows bundle, fuzz-as-claims, booklet
  baseline); VM suite cited as smoke only.
- [ ] R46-C attempt-2 event disposition: closed only by the F1 fix
  evidence above, never by a green re-run.
- [ ] Cache-policy disposition + disk-ritual eligibility note.
