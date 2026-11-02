# Carried items register — OpenBulletin

A standing register, not a round record. Round records
(`docs/planning/audit-round*-synthesis-*.md`, the
`round*-out/coord/` batch records) describe one round;
this file carries what stays open or watched AFTER the
loop ends, so it cannot be lost when the loop's working
papers stop being written. That is why it lives here in
the repo's docs, beside the other standing records
(`docs/reviews/checklists/`), instead of in the audit
tree: the audit tree is per-round working papers, and
this register must outlive it.

## C-1 — R50-A adjacent-syscall residual (OPEN, bounded)

The claim-steal removal is content-checked, and the
walked interleaving is closed (round 50, R50-A). What
remains, stated plainly and NOT claimed closed, is the
adjacent-syscall window — preemption between a re-read
and the unlink that immediately follows it — in three
places:

- Steal path: the helper's read returns, then
  `remove_file` runs; a rival claim file could land
  between the two syscalls.
- Drop path: the identical window, already accepted
  by `Drop` before R50-A.
- Displaced path: `displaced_path_safe_to_clear`'s
  fresh read returns, then `remove_file(&displaced)`
  runs; a live generation could land between them.

Defending layers (unchanged by R50-A, still bounding
the outcome):

- Link-atomic create: the claim file is created with
  link-only-on-vacant semantics, so two claim sequences
  cannot both hold the path.
- Last-moment live-peer re-read (R46-A): the check
  sits immediately before the removal, so a whole rival
  claim sequence no longer fits any window containing
  judgment work — only the adjacent syscalls do.
- Post-rename audit restore (R47-A): the audit pass
  restores a claim a faulty step removed, bounding a
  bad removal's blast radius.

Re-open triggers (a demonstrated outcome through this
residual is a NEW finding, not a re-open of R50-A):

- A steal removal deleting a claim file not naming the
  judged dead owner.
- A displaced-path removal deleting a live generation.
- A demonstrated two-holder or unsafe-direction
  outcome through the residual window.

Distinct from R51-RED-1 (the claim-owner-text
uniqueness failure across restarts, repaired by
R51-B with the instance token in claim owner text):
that hole defeated the check itself via identical
bytes; this residual races the check's adjacent
syscalls. Do not conflate them.

## F11 tripwire — CLOSED, watched (owner + cadence)

F11 (fork-inherited write-fd blocking execve) is
CLOSED since round 49 (R49-A): the process-wide mutex
across peer copy + spawn, the guard extension to the
module's `/bin/sh` spawns, two consecutive 11-trio
validation batches green with zero BUSY events, and
the kept R48-A ETXTBSY-only bounded retry (5 attempts)
as backstop for fork sources outside the module
(including all production spawns, which are NOT
mutex-covered — see the corrected inventory in
`crates/ob-app/src/main.rs`, round 51, R51-F, G-14).

- Owner: Jacob (product owner), executed by Envoy
  (the audit agent) — a named owner, not "someone
  sometime".
- Watch cadence: EVERY hosted log pulled for any
  batch or close is searched for `ETXTBSY` and
  `ETXTBSY absorbed` lines, and the result is stated
  in that batch's record (as every round-51 batch
  record does). After the audit loop ends, the cadence
  continues as a standing item on the pre-release
  checklist (`docs/PRE-RELEASE-CHECKLIST.md`): no
  release ships without a tripwire check of the
  release candidate's hosted CI logs.
- Re-open trigger, verbatim: any future `ETXTBSY
  absorbed` line in hosted CI re-opens F11.
- Tripwire history: did NOT fire in the round-50
  close legs (30 files, 30,419 lines, 0 lines), nor
  in any round-51 batch leg to date (each batch
  record states its own check).

## Carried note — R50-C first-smoke-death (NOT a finding)

The first VM smoke attempt in round 50 (R50-C) died
without a status file: its background session was
lost, its log froze at compiling lines, and no
cargo/rustc process remained. No verdict was read
from it; the re-run from scratch (with `.status` 0)
is the only smoke result reported. Cause-unknown in
substance — no cause is invented here. Full note:
`round50-out/coord/r50c-record.md`. Carried here so
the note has a standing home, not just a batch
record.
