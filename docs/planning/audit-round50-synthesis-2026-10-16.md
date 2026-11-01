# OpenBulletin convergence round 50 — synthesis

- Audit over code tip `fa4df16` (round-49 close record `7045943`).
- Lanes (blind, read-only): verify, review, red team. Full reports:
  `~/workspace/openbulletin-audit-wave3/round50-out/verify/report.md`,
  `.../review/report.md`, `.../redteam/report.md`.
- **Deduped tally: 0 BLOCKER / 0 MAJOR / 4 MINOR. The exit bar IS met
  on this audit's tally; the Tier-2 final gate is the next step, NOT
  started by this document (that decision is the parent's).**
- No fixes are landed by this synthesis. Fix phase is NOT dispatched
  by this document; it is reported to the parent first.

## Legs of record (unchanged from round 47)

Hosted CI on all three OS legs, desktop runner build, Windows bundle,
fuzz-as-claims, booklet baseline. The VM full suite is a provisional
pre-close smoke run, never cited as a leg, and its run MUST capture a
`.status` file. A VM smoke failure is triaged (load-noise solo re-run
vs real failure); only a real failure is a finding, and a smoke result
never blocks a batch whose hosted legs are green.

## Standing-record correction (carried)

The fuzz 40,000 / 497,024 figures are the bins' INTERNAL CASE COUNTS,
not stdout lines. The byte-identical claim is the md5 match: verdict
`797758443d101f9339fdd778e14af430`, composition
`e8b2e73b117c7013d867419da8fb0ab7`, rights 5083 / 1024 / 5275,
safety_flips=0. The "lines" wording is not repeated.

## Verify lane: PASS on legs of record (0 findings)

- Hosted on `fa4df16`, all re-read firsthand via `gh run view`, every
  job attempt 1: CI `41320362634` (Ubuntu `124889042275`, macOS
  `124889042382`, Windows `124889042410`, all success); Desktop
  Runner Build `41320362614` (job `124889042262`, success); Windows
  bundle `41324286124` (Gate `124900855328`, bundle `124904269797`,
  both success). Annotations everywhere are runner notices only.
- **F11 re-open trigger check — DID NOT FIRE.** All 6 hosted job logs
  downloaded firsthand and searched: `ETXTBSY` count 0 in every log;
  case-sensitive `ETXTBSY|absorbed` returns no matches;
  case-insensitive `absorb` hits are passing test names only. No
  `ETXTBSY absorbed` line exists, so F11 stays closed. No BLOCKER on
  this ground.
- Booklet exact, no movement: imposed 31,717 bytes, reader 34,977.
- Fuzz-as-claims byte-identical: bin-source md5s verified BEFORE
  running and matched the standing values; all four bins exit 0,
  stderr 0 bytes; stdout md5s byte-identical (verdict, composition,
  rights as above); verdict internal case count 40,000, panics 0;
  comp internal case count 497,024, panics 0; rights 5083 / 1024 /
  5275, safety_flips=0.
- VM suite as SMOKE (NOT A LEG), reported-not-cited, with `.status`
  capture: exit 0, scripted tally 1358/0 over 30 `test result:`
  records, 0 FAILED/panicked lines — exactly the round-49 figure.
  Completed in one continuous span, no interruption, no re-run.
- Cold clippy clean: after `cargo clean` (exit 0), `cargo clippy
  --workspace --all-targets -- -D warnings` exit 0, 0 lines beginning
  `warning:`/`error:`. Disclosed delta: 313 Checking lines vs round
  49's 312, confined by sorted diff to checked dependency units
  (`static_assertions`, `zbus_names` vs `zvariant`) — a checked-unit
  artifact, not a warning.
- M8 rights safety axes PASS from the lane's own fuzz outputs
  (runnable form from the round-49 verify report §6, not invented):
  ROUTE/ROUTE2 safety_flips=0; PERM `seqs=3000
  flip_any_safety=10 flip_children=0
  flip_noted_among_children_bearing=0`; Part B `19/19/0/0/0`; Part C
  `3000/2161/0/0/1555` — all matching round-49 values exactly.
- Landed items spot-check, all PRESENT with cites: inbox landing
  state, fingerprint gloss, shape/template sentence, kept-announcement
  names in the Next Sunday note, H2 Announcements line, Check
  `state_sentence` line.
- Timeline clarification (not a finding): the bundle run
  `41324286124` was created 2026-11-01T16:03Z and is green; this
  corrects the r49c batch record's "bundle not run" statement, which
  was true at r49c time. The round-49 close record already cited the
  run.

## Review lane: 0 BLOCKER / 0 MAJOR / 2 MINOR

Fixes held: R49-A mutex held across copy + spawn, all 23 dead-owner
`/bin/sh` sites routed through `spawn_dead_sh_child` (zero raw sites
remain), retry unchanged (ETXTBSY-only, 5 attempts); production spawns
(soffice, `open_with_os`) still NOT mutex-covered, exactly as
disclosed. R49-B `saturating_clamp` at all six Pages geometry groups;
`sanitized()` untouched. R49-C kept names, H2 line, Check line all
present and cross-checked (H2 classification agrees with the inbox row
for all five `SlotTarget` variants). R48/49 UX re-checks all present.
Timing pins: R49 diff touches no timing file; ratio-form pins intact.

### MINOR 1 — Check-step doc comment misstates fingerprint provenance (comment only; behavior correct)

`check.rs:1105-1112` says the Check line is fed session fingerprints
"instead of a remembered file state" — false in substance, since
`seed_doc_state` (`windows.rs:970-978`) seeds those fields from
remembered state at load. Behavior is correct (consistent with J6);
comment-only fix.

### MINOR 2 — `paste_box` inverts its offset for oversize frames (pre-existing, not an R49 regression)

`windows.rs:1498-1499`: e.g. panel 200,000 / frame w 300,000 pastes at
x = -100,000. No panic, recoverable via undo/drag, hence MINOR.

## Red team lane: no BLOCKER / 2 MINOR, F11 not re-opened

Failed attacks (enumerated in the report): ETXTBSY fork inventory
complete — exactly two `Command` sites in `settings_tests`, both under
the R49-A mutex, zero in all other test modules; production-spawn
bypass narrowed to unreachable (no test calls `wmf_converter` /
`open_with_os`); guard-drop timings, nonce uniqueness, cross-process
forks, non-`Command` fork sources all checked and safe. Lock/claim:
guard reentrancy, drop order, bounded wait, orphan sweep, seam-pause
walk all hold. Clamps: all six R49-B integer sites confirmed
converted; remaining plain clamps audited (constant-bound or
panel-dimension with sanitizer bounds `0..=10M`, worst-case arithmetic
far from `i64` overflow); a zero-size Fill picture frame attack was
refuted by running code (`f64::clamp` returns NaN rather than
panicking on a NaN value). Wording: Next Sunday kept count/names
cannot diverge on the production path (single push site); Check
staleness branches are all true given their fingerprint comparisons.
General sweep: undo parse failure fail-safe, rights merge fill-only
with no flip path found, production `.expect()`s checked-just-above
patterns — none filed without a reproduction.

### MINOR-1 — Announcements fate line mislabels its set

The step renders `announcements_contribution_status(&doc,
&inbox_items)` with the prefix "Filed contributions:", but
`refresh_inbox` scans `inbox_items` with `filed=false` while
`inbox_filed` (filed folder, `filed=true`) is the filed set, and
filing is the post-review move into `filed/`. A fresh, never-reviewed
inbox arrival renders as "Filed contributions: 1 part not yet in the
bulletin." Second facet: the line rescans only on arrival at Start, so
going Start → file → Announcements directly can count an
already-filed contribution as still "not yet in." Counts themselves
match the Start rows; only the label/set pairing and staleness are
wrong. Distinct from the H2 line (which review confirmed present and
correct); this is the fate line, not the H2 line.

### MINOR-2 — Claim-file steal removes without a content check (narrow TOCTOU)

In `acquire_instance_claim_serialization_file`, after judging the
standing claim owner's pid provably gone, the code removes the claim
file without checking it still names that dead owner. Full
interleaving in the report ends in two concurrent claim *sequences*
(rival links between the gone-check and the remove if the first
claimant is preempted between two adjacent syscalls). Kept below
BLOCKER by verified defense in depth: link-atomic lock create, rename
step's last-moment live-peer re-read, and post-rename audit restore —
a walked seam-pause shape ends with one holder, not two.

## Coverage limits (stated, not hidden)

- Red team trio guard batches (11 trios × 2, R49-A shape, plus a
  heavier 22-trio variant) were DEFERRED, not run: scans showed active
  cargo/rustc processes including a workspace test run, and per the
  process rule nothing was labeled, signaled, or killed; trio runs
  would share the repo target dir and CPU with that run. The report
  gives the exact follow-up commands to run after a fresh scan shows
  no cargo processes. Until then, F11's round-50 standing rests on the
  static inventory plus R49-A's own green guard batches, not on new
  load evidence from this lane. This is a coverage limit, not a
  finding: the F11 re-open trigger check (hosted logs, verify lane)
  is the governing check and it did not fire.
- Review lane claims no suite run, GUI render, or hosted-leg verdict
  (by design; those belong to verify).
- Verify lane's smoke figure (1358/0) is reported-not-cited, never a
  leg.

## Deduped tally and dispositions

**0 BLOCKER / 0 MAJOR / 4 MINOR.** No overlaps: review MINOR 1
(Check doc comment) and red MINOR-1 (fate line) are different lines;
review MINOR 2 (paste offset) and red MINOR-2 (claim steal) are
different mechanisms. No lane contradicts a carried item with
evidence; all carried items concurred independently.

Fix-phase dispositions (NOT dispatched by this document):
- **R50-A FIRST (red MINOR-2):** claim-file content check — the only
  finding touching the lock family; needs care, and its batch must
  show the walked interleaving can no longer end in two claim
  sequences (or show why the existing defense in depth already
  covers it, with the same rigor).
- **R50-B (review MINOR 2):** `paste_box` oversize offset.
- **R50-C (wording pair):** review MINOR 1 Check doc comment +
  red MINOR-1 fate line label/set and staleness.
- **Verification follow-up (not a batch):** run the deferred red-team
  trio guard batches after a fresh scan shows no cargo processes,
  per the commands in the red team report §1.4; report the outcome
  in the fix-phase record whether green or red.

## Carried items

- MINOR 2's H1 refined-rule sentence: still gated on the live check,
  NOT landed (all three lanes concur; code-side confirmation is not
  the live check). It is the one round-49 finding still open, and it
  stays open.
- Outstanding, needing a GUI run or a human (neither a blocker on code
  merits, neither claimable until it happens): R47-B keyboard walk;
  H1 live check (hand-edit a page, go back a step, start next Sunday,
  check the edit on screen and in print).
- R47-F monolith extraction: still deferred, correctly so (no bug
  traces to file size).
- H6: not a finding — a proposed real-user task for Jacob's decision.
  Not a batch.
- Superseded R49-C tip `927482e` not re-litigated; only `fa4df16`'s
  verification was checked (diff: 1 file, `main.rs`, 10 insertions /
  6 deletions, test-only).

## Bar judgment

**The exit bar IS met on this audit's tally: zero BLOCKER and zero
MAJOR.** The bar is judged on the audit round, and this is the first
round of the loop to return zero BLOCKER and zero MAJOR. The four
MINORs above are fixed or dispositioned on the record by the fix
phase; they do not hold the bar. **Next step: the Tier-2 Opus + GPT
final gate, as chartered — NOT started by this document. That
decision is the parent's.** Nothing in this synthesis authorizes it,
and no fix-phase batch may be presented as a substitute for it.
