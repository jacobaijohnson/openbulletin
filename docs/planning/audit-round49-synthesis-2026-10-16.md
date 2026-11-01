# OpenBulletin convergence round 49 — synthesis

- Audit over code tip `0a1d76e` (round-48 close record `c9a07d6`).
- Lanes (blind, read-only): verify, review, red team. Full reports:
  `~/workspace/openbulletin-audit-wave3/round49-out/verify/report.md`,
  `.../review/report.md`, `.../redteam/report.md`.
- **Deduped tally: 1 BLOCKER / 2 MAJOR / 3 MINOR. The exit bar is NOT
  met; the Tier-2 final gate is NOT triggered.**
- No fixes are landed by this synthesis. Fix phase is NOT dispatched
  by this document; it is reported to the parent first.

## Legs of record (unchanged from round 47)

Hosted CI on all three OS legs, desktop runner build, Windows
bundle, fuzz-as-claims, booklet baseline. The VM full suite is a
provisional pre-close smoke run, never cited as a leg. A VM smoke
failure is triaged (load-noise solo re-run vs real failure); only a
real failure is a finding, and a smoke result never blocks a batch
whose hosted legs are green. Round 49's smoke run captured a
`.status` file, closing the round-48 disclosure (the round-48
smoke re-run lacked one; the round-48 close stood on the hosted
legs regardless).

## Standing-record correction (carried)

The fuzz 40,000 / 497,024 figures are the bins' INTERNAL CASE
COUNTS, not stdout lines. The byte-identical claim is the md5
match: verdict `797758443d101f9339fdd778e14af430`, composition
`e8b2e73b117c7013d867419da8fb0ab7`, rights 5083 / 1024 / 5275,
safety_flips=0. The "lines" wording is not repeated.

## Verify lane: PASS on legs of record (0 findings)

- Hosted on `0a1d76e`, all attempt-1 success, re-confirmed via
  `gh run view` (and re-confirmed firsthand by the coordinator):
  CI `41298381406` (Ubuntu job `124823974843`, macOS
  `124823974993`, Windows `124823975011`), Desktop Runner Build
  `41298381408`, Windows bundle `41303485462` (Gate
  `124839063528`, bundle `124841862921`). Annotations are
  runner-image notices only. No flake on any `0a1d76e` leg.
- **Ubuntu green, stated plainly: the Ubuntu CI leg is green on
  `0a1d76e`. This Ubuntu green does NOT close F11.** The retry
  landing is mitigation for a mechanism that round 49 has now
  named (see F11); F11 stays open as a work item, and the exit
  bar is not met on F11 grounds. A report implying otherwise
  would be a reporting defect; this lane's report states the
  non-closure explicitly.
- Booklet baseline exact: 31,717 imposed / 34,977 reader, exit 0.
- Fuzz-as-claims byte-identical: bin sources md5-matched before
  running; all four bins exit 0, all stderrs 0 bytes; stdout md5s
  as in the standing correction above; rights counters exact,
  safety_flips=0.
- VM suite as SMOKE (NOT A LEG): `SMOKE_EXIT=0`, scripted tally
  1352 passed / 0 failed over 30 test-result records, with a
  `.status` capture. Reported as smoke only, never cited as a leg.
- Cold clippy: `CLIPPY_COLD_EXIT=0` after `cargo clean`, 312
  Checking lines, 0 warning/error lines.
- M8 rights safety axes PASS (route and order invariance re-run
  in this lane's path).

## F11 — BLOCKER as a work item: MECHANISM NAMED, closure path specified (red team)

F11 is the ETXTBSY ("Text file busy") spawn flake in the
fake-peer test harnesses. Round 49 names the mechanism:

**Fork-inherited write-fd blocking execve.** A BUSY event happens
when, and essentially only when, another thread forks a spawn
child while the victim thread's peer copy is still in flight (its
dest fd still open for write). Fork duplicates every fd, and
O_CLOEXEC does not prevent inheritance, only exec-time close, so
the victim's inode still has a write holder when the victim's own
child execs it, and the kernel's exec-time deny_write_access fails
the execve with ETXTBSY. The window closes when the holder child
execs or closes fds — hence the variable sub-ms to low-ms window,
immediate-retry resolution, the +1 ms straggler, strace
suppression, and full copy+spawn serialization eliminating it.

Decisive evidence (standalone C probe, exact per-spawn errno
counts, denominators in the red-team report):
- Enrichment: BUSY 67/429 = 15.6% on spawns where another thread
  forked during the victim's copy vs 0/5571 = 0% without (run C4);
  corroborated 11.0% vs 0.012% (A3) and 15.0% vs 0.036% (C3).
- Smoking gun: holder scan caught a live pre-exec child (comm =
  probe, i.e. not yet exec'd) holding the exact failing victim
  path on a write fd in 40/67 C4 events; 143/281 across all
  fixed-scan runs.
- Single-thread control: 0/1500 — cross-thread forking is
  required; post-close kernel lag ruled out as the main mechanism.
- Decomposition: full copy+fork lock 0/6000; copy-lock alone
  2/6000 (nearly, NOT fully, eliminated); spawn-lock alone
  45/6000 (no elimination); readwrite 58/6000 and
  copy_file_range 62/6000 (copy syscall irrelevant); tmpfs
  45/6000 (filesystem irrelevant); post-copy delay 5 ms 2/6000
  (holders finish first).
- Refinement the lock rows force: fork-during-copy is necessary
  but not sufficient — the fork must land in the TAIL of the
  victim's copy window (~10–15% conversion), which explains why
  copy-lock nearly eliminates (lock handoff orders forks to copy
  START).
- Reflink / copy_file_range branch CLOSED: the R48 trace re-read
  firsthand shows copy_file_range returning EXDEV on every peer
  copy (cross-device: source on overlay, dest on btrfs), std
  falling back to sendfile; reflink is impossible there and
  unsupported on the CI leg's ext4.

Disclosures carried from the red-team report, not hidden: the
first holder scan compared relative paths against absolute
/proc targets, so its zeroes are VOID (fixed with realpath,
validated by a positive control; only fixed-scan results are
cited); under copy/full locks the fork-marker window straddles
lock-wait, so L1/L3 marker counts overcount true
fork-during-copy (BUSY counts unaffected).

**Closure path (fix phase's choice; this lane does not land
code).** F11 closes when a change removes the fork-during-copy
window (or makes it harmless) AND a validation batch shows it:
1. Serialize copy+fork in `spawn_live_peer_named` (process-wide
   Mutex across `fs::copy` AND `Command::spawn`, as the R48-A
   probe did — 11/11 trios green, 0/6000 in the full-lock probe
   run). Copy-lock alone and spawn-lock alone are NOT closures.
2. Keep the R48-A bounded ETXTBSY-only retry as the permanent
   guard, now justified by a named mechanism rather than as blind
   mitigation, with the absorbed-transient lines kept audible.
3. Close-then-fork fd hygiene narrows but does not vanish the
   window; NOT recommended as a sole closure.
Validation batch for closure (any option): the r42e trio batch at
the R48 scale (11 parallel trios) green WITH the mechanism check
— zero BUSY events under option 1, or absorbed-transient lines
only (no failures) under option 2 — plus a statement of which
option landed. A green batch alone, without naming the option,
is not closure; neither is a green hosted leg.

**What this verdict does and does not do.** It names the
mechanism and the closure path. It does NOT by itself close F11
as a work item: the closure is a change the fix phase must land
plus a validation batch. Until then F11 stays OPEN, though it is
no longer an unnamed mechanism. It is not a product defect (the
retry and the spawn helper are test-only; the one production
spawn, `open_with_os`, execs a system binary, not a copied
executable — red-team falsification attempt 6).

## MAJOR 1 — R48-R4 clamp-panic class: PIN, do not defer (review, verdict on the candidate)

The deferred clamp-panic invariant class is real, reachable by
ordinary Pages actions on a file that passes the app's own
sanitizer, and cheap to pin. It should not stay deferred.

Evidence chain: `sanitize_panels` (ob-model:1865-1871, called by
`Document::sanitized` on every load) clamps each coordinate
independently — panel size to [0, 10_000_000], frame x/y to
±10_000_000, frame w/h to [0, 10_000_000]. Nothing enforces
`w <= panel.w`, `x + 18_000 <= panel.w`, or
`panel.w >= 18_000`. A hand-edited, older-version, or
foreign-produced `.obd` can therefore carry a frame wider than
its panel, or a frame whose left edge sits within 18_000 mpt (a
quarter inch) of the panel's right edge. Every Pages geometry
path then clamps against a computed maximum that can fall below
the minimum, and Rust's `Ord::clamp` panics when min > max:
`keyboard_resize_rect` (main.rs:5056-5057, whose doc comment
falsely asserts "every place and move path guarantees" the
room), corner-drag resize, drag move, arrow-key nudge, music
move, and place/size dialog (windows.rs:4157-4180, 5266-5286,
5385-5386). `keyboard_resize_rect` is called on the selected
frame of the loaded document (windows.rs:5429); the existing pin
exercises only in-invariant rects, so nothing today would catch
this. Consequence is fail-stop, which is why this is MAJOR and
not BLOCKER: the app runs through `eframe::run_native` with no
`catch_unwind`, so a panic in a Pages gesture kills the app and
loses unsaved session work (journal recovery aside). It does not
silently misprint or corrupt a file.

Fix direction: one saturating helper (clamp against
`max(min, computed_max)`) at all six sites, preserving
in-invariant behavior exactly; or extend `sanitized()` to
enforce the relational invariant — but that silently moves boxes
on load, so the call-site guard is the safer fix. Pin with unit
tests feeding `keyboard_resize_rect` and
`apply_frame_place_size` a `sanitized()` document carrying an
oversize frame and a near-right-edge frame, asserting no panic
and a sane resulting rect. The pin is the deliverable that ends
the deferral; the guard is the fix it should accompany.

## MAJOR 2 — Next Sunday note states weekly announcements were cleared, even when keep-through announcements ride (review)

`windows.rs:635` builds the Next Sunday note unconditionally as
"Started the {} bulletin. Weekly announcements were cleared;
standing items carried over." But ob-layout counts announcements
whose keep-through date still covers the new Sunday into
`report.kept_announcements` (tested, never surfaced) — only ended
counts/names, cleared participants, and cleared hymns are
appended. A user who set a keep-through announcement reads that
it was cleared, finds it still on the page, and may reasonably
re-add it and print it twice. "Standing items" does not rescue
the sentence: in this codebase "standing" is the non-weekly
story role, not a keep-through weekly announcement. Pre-existing
wording, not an R48 regression.

Fix direction: when `kept_announcements > 0`, qualify the
sentence and name the kept announcements the way ended ones are
named (the J10 "automation shows its hand" shape already used
for ended announcements, participants, hymns, and loose sheets
in the same function).

## MINOR 1 — H2 is half-landed: the Announcements step itself still has no applied/unapplied/unlanded line (review)

R48-B's inbox row status answers the fate question at Start,
where files arrive. But H2's literal ask was "a short status line
on Announcements naming applied, unapplied, and unlanded parts,"
and step 2 (main.rs:2377-2425) contains no such line: it shows
the announcements text box, the keep-through field, and the
children's-names rule, but nothing about filed contributions.

Fix direction: either add one line on Announcements derived from
the same `applied_claim` / `planned_part_plan` sources R48-B
uses, or explicitly close H2 as satisfied-by-inbox with the
rationale recorded. Either closes it; leaving the literal ask
unaddressed without a recorded decision does not. No data loss
is involved, hence MINOR.

## MINOR 2 — H1 refined-rule sentence still not stated in the Pages window or the Next Sunday report (review; live check outstanding)

Code side CONFIRMED: `next_sunday_report_as_of`
(ob-layout:466) clones the document, reads frames only to find
weekly story ids, and never rebuilds or removes a frame; only
story bodies (weekly, keep-through excepted), service-story hymn
values, participants, and ended loose sheets change.
`goto_step` (windows.rs:18) rebuilds nothing; only the
two-click-armed shape pick (main.rs:1901-1915 → `rebuild()`)
replaces the document. But neither the Pages window nor the Next
Sunday note states the refined rule (a geometry edit rides the
clone; words in a non-weekly story ride; words in a weekly story
clear by design; a shape pick replaces the whole bulletin). The
H1 live check (hand-edit a page, go back a step, start next
Sunday, check on screen and in print) still needs a GUI run or a
human and is NOT confirmed here. File the wording as owed once
that check holds; do not state the rule in the UI before it
holds. The fix phase therefore CANNOT land this sentence yet —
it stays outstanding, gated on the live check.

## MINOR 3 — H4 check-side sentence lives only on Start, not on the Check step itself (review)

The print side of H4 is landed: the Print step warns "This
bulletin changed since the print copy was made" (main.rs:2544).
The check side exists only as the Start sentence "Checked, and
nothing has changed since" / "Not checked since the last change"
(check.rs:1110-1126, `state_sentence`). The Check step itself
shows findings and the "does not check spelling" note but no
"changed since last Check" line.

Fix direction: one line on the Check step rendering the same
`state_sentence` comparison. Small, wording-only, hence MINOR.

## Re-checks that stand (not re-filed)

- **R48-B, all four wording items — STAND.** Start inbox landing
  state (`inbox_row_status`, contribution.rs:1834, wired at
  main.rs:1755, four phrases matching `AppliedClaim` plus the
  `KeptWithPart` nowhere-to-land cases); fingerprint gloss with
  hex kept (gloss at main.rs:1723, `"{name} ({fingerprint})"`
  rendering unchanged); shape-versus-template sentence
  (main.rs:1899); missed settings-save sites (all six
  user-visible sites carry session-only wording; the three
  silent `let _ = save_settings` sites are documented
  best-effort writes for implicit state, not missed sites).
- **HF-12 — CLOSED, confirmed for the landed halves, not
  re-raised.** IDML half verified firsthand (typed
  `IdmlXmlError::TooDeep` / `Damaged`, variant match at
  idml.rs:288-300, no substring branch, byte-for-byte sentence
  pinned); `from_json` half likewise typed. Residual String
  depth errors in Scribus (sla.rs:110) and MusicXML
  (parse.rs:501) branch on no substring — gradual future rollout
  at most, noted, not filed. The round-44 open-doors half is
  taken as carried, not re-audited.
- **R48-A / R48-C regression checks — clean.** The retry is
  test-only, ETXTBSY-only, bounded at 5 with guarded backoff
  indexing and logging; no production code path touched. The
  IDML change preserves bytes at the dump boundary and both new
  branches are pinned; the retired `TOO_DEEP` constant is gone
  with no remaining substring branch in idml.rs.
- **R47-F monolith extraction, corrected framing — CONFIRMED,
  still deferred, correctly so.** Measured: main.rs 16,979 lines
  total with 5,603 production lines; windows.rs 9,085 production
  lines with no test module — windows.rs is the bigger
  production file. No bug found traces to file size (the one
  crash class found, MAJOR 1, traces to a missing relational
  invariant, not module size). Extraction remains a
  maintainability preference, not convergence work; do not
  charter it as a fix round.
- **UX walkthrough + vocabulary items — checked, no further
  findings.** Shape commitment at Start now explained three ways;
  a step's printed result is previewable via the announcements
  page link and the Print step's sheet preview; print choices are
  labeled by purpose; inbox row statuses avoid the "applied"
  jargon in favor of plain words, satisfying the vocabulary item
  in substance. H6 (substitute completion) is NOT filed — it
  remains a proposed real-user task for Jacob's decision only.
- **Red-team falsification attempts — 7 attempted, 0 bit.**
  R48-C typed IDML error (no substring branch remains); R48-B
  wording sites (all present); R47-A guard hold scope (no
  regression); WMF scratch-name escapes (new edges refused);
  print-copy pid+nanos nonce (no regression); production spawns
  of copied executables (only `open_with_os`, a system binary,
  not a copied executable); retry retrying non-ETXTBSY
  (ETXTBSY-only by construction). Filing a finding from a
  non-biting attempt would be fabrication; none is filed.

## Adjudication notes (coordinator)

- Lane reports read in full; no contradictions between lanes.
  Review's R48-A/C clean verdicts agree with red-team
  falsification attempts 1, 2, 3, 7; review's H2 MINOR 1
  (Announcements step lacks the line) is compatible with, not
  contradicted by, red-team attempt 2 (R48-B inbox row present)
  — different locations, and the review finding is precisely
  that the line is missing at the second location.
- The verify lane's execution metadata shows one failed exec
  call; the verify report itself is complete (235 lines, all six
  checks with exit codes and run ids, no check left unrun), and
  the coordinator re-confirmed all three hosted legs firsthand
  above. The failed exec is subagent-internal tool noise, not a
  check gap, and no verdict rests on it.
- The workspace `~/workspace/AGENTS.md` file read during this
  span governs the Encore seat (encore-build.json, envoy-runner,
  Wave 2.495); it is not this lane's operating manual and
  changes nothing here. This lane's legs of record (hosted CI,
  runner, bundle, fuzz, booklet; VM suite as smoke only) stand
  as adopted for round 47.

## Outstanding, needing a GUI run or a human (neither a blocker on code merits, neither claimable until it happens)

- R47-B keyboard walk (resize, frame-menu opening, numeric
  place/size keyboard path — code side landed in round 47,
  rendered behavior unconfirmed).
- H1 live check (hand-edit a page, go back a step, start next
  Sunday, check the edit on screen and in print). MINOR 2's UI
  sentence is gated on this check and cannot land before it.

## Batch dispositions (not dispatched)

- **R49-A FIRST (F11 closure):** land option 1 (serialize
  copy+fork in `spawn_live_peer_named`) or ratify option 2 (the
  R48-A retry as the permanent guard, mechanism named
  alongside it, absorbed-transient lines kept audible), plus
  the section-4 validation batch (11 parallel trios green WITH
  the mechanism check and a statement of which option landed).
  Copy-lock alone and spawn-lock alone are NOT closures. This
  batch closes F11 only with that evidence, never by
  re-run-and-forget, never by a green hosted leg alone.
- **R49-B (MAJOR 1):** saturating clamp helper at all six Pages
  geometry sites + pins feeding `sanitized()` oversize and
  near-right-edge frames. Also correct the false doc comment on
  `keyboard_resize_rect` (the guarantee it asserts does not
  exist) inside this batch.
- **R49-C (wording):** MAJOR 2 Next Sunday note qualification +
  kept-announcement names; MINOR 1 H2 line on Announcements OR
  an explicit satisfied-by-inbox closure with rationale
  recorded; MINOR 3 Check-step `state_sentence` line. MINOR 2's
  H1 sentence is NOT in this batch — it stays gated on the
  live check.
- **R47-F:** stays deferred, not a batch. **H6:** not a finding,
  not a batch — a proposed real-user task for Jacob's decision.
