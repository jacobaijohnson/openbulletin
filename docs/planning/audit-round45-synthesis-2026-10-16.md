# Audit round 45 — synthesis (coordinator)

Audited tree: `34690e3` (round-44 final code tree; close
record `139551d`). Three blind lanes: verify (recovered
after a restart drain killed the first lane mid-suite;
the successor re-ran every claim from scratch and
consumed the predecessor's CI evidence on disk), review,
red team. Lane reports: `round45-verify.md`,
`round45-review.md`, `round45-redteam.md` in
`~/workspace/openbulletin-audit-wave3/`.

## Lane tallies

- Verify: claims PASS on all six standing claims
  (suite 1275/0 own-run, clippy clean incl. a cold
  scratch-target re-check, booklet pin 31,717 / 34,977
  exact, fuzz byte-identical to the standing baselines
  with rights 5083/1024/5275 safety_flips=0, hosted CI
  for `34690e3` per-job success ×3 + runner + bundle);
  findings 0 BLOCKER / 0 MAJOR / 2 MINOR (reporting
  class).
- Review: 0 BLOCKER / 0 MAJOR / 5 MINOR, 3 observations.
- Red team: 1 BLOCKER / 4 MAJOR / 2 MINOR, 5
  observations.

## Deduped tally: 1 BLOCKER / 4 MAJOR / 6 MINOR

Eleven findings. **The exit bar is NOT met** — the round
carries a BLOCKER. The Tier-2 Opus + GPT final gate is
not triggered.

**Headline: the named race-pin target resolved AGAINST
the standing presumption.** The r42e four-claimant flake
is not a test-composition defect. The red team
reproduced both failure shapes locally (zero-winner
under concurrent stress, and a shape the hosted record
never showed: **two winners**), and witnessed both
deterministically at step level through the production
functions in production order. Production is at fault;
the pin's assertion states the contract correctly. The
round-44 carry's fix direction (test composition) is
superseded by this evidence — see adjudication 1.

## Findings

### F1 (BLOCKER) — Two legitimate claimants can both hold the instance lock (red team RT45-L1)

`try_create_instance_lock_from` creates the lock file
with `create_new` and then writes the holder text;
between the two steps the path holds an empty file. A
rival reading in that window captures `judged = None`,
and `None` acts as a wildcard generation: the
displacement generation check passes `None == None`,
the live-peer check is skipped for `judged = None`,
and `rename_instance_lock_to_displaced_from` — the
step that moves the file — re-judges only the
*displaced* path and carries no generation check of
its own on the lock it moves. Deterministic witness
(red team L45d, through production functions): B's
rename step moves A's completed live generation to
the displaced path; C's ordinary create lands in the
vacancy; final state `two_winners=true`, A's live
generation standing displaced. Composed witness: the
round-20 two-winner run under 6-way concurrent stress
(3 anomalous runs in 150 runs / 4,500 rounds; plain
and single-core-under-load runs clean over 3,600
rounds). This is the mutual-exclusion contract the
journal machinery rests on, failing in the safety
direction.

### F2 (MAJOR) — The hosted flake itself: a legitimate creator stands down and its generation wedges the lock (red team RT45-L2)

Same root, second audit timing. Deterministic witness
L45a reproduces the hosted panic shape verbatim
(`[false, false, false, false]` with the lock naming
live W, who holds nothing); witness L45b shows the
demoted-but-live generation at the displaced path
holding every later claim off for its whole lifetime
(`LiveGenerationBeside OccupiedLock` beside a dead
holder's lock). A liveness break in the safe
direction: no second holder, no journal loss. This is
the shape behind all six hosted events in rounds
43–44 (and the earlier ledger entries).

### F3 (MAJOR) — The invisible/format class: R44-F's "nothing" set was drawn two members wide (red team RT45-C1 + review MINOR 2 + review MINOR 3)

One mechanism, three arms. The garnish/suffix readers
shed an invisible set containing exactly U+200B and
U+FEFF (R44-F). The rest of the class splits against
the clean twin at the judged seats, both routes:

- (a) Beyond the pair (red team: U+200C, U+200D,
  U+2060, U+180E, U+200E, U+200F, U+061C, U+034F,
  U+FE0F; review's sixteen further codepoints at the
  suffix seats overlap and extend the set): trailing
  governor serves a cancelled date live; trailing
  weekday group loses its WeekdayMismatch record;
  base-trailing company serves the resolution
  silently; **inside the governor word**
  (`(cancel⁠led)`) the governor never fires; inside
  the weekday word the record's named word is
  polluted. These are ordinary paste artifacts
  (ZWNJ/ZWJ in web/Word/PDF copies, ZWJ in compound
  emoji, VS16 on phone keyboards) and are invisible —
  the served text looks judged.
- (b) The admitted pair at seats R44-F missed
  (review): date↔group boundaries glued either side,
  leading group→date boundary, two-group trailing
  company, and **date interiors at trailing seats**
  (`November\u{200b} 1, 2026 (Monday)` serves, record
  lost — while the same interiors at base-trailing
  refuse with the record, so two readers disagree;
  and a zero-width after the first date of a two-date
  line flips which date serves). This falsifies, at
  these positions, R44-F's stated premise that inside
  a date the pair is "unreadable rather than
  misread".
- (c) Inter-group at the leading seat splits for
  U+034F / U+FE0F (red team arm).

Graded MAJOR (red team's grade adopted; the review
lane's MINOR covered only the suffix seats): both
harms, both routes, inside-word arms, invisible
characters with ordinary paste paths.

### F4 (MAJOR) — A judged suffix glued between an inert group and a following governor disarms the governor (red team RT45-C2)

A suffix is shed when it trails a *judged* member, not
when it trails an inert group on the governor's
doorstep: `(notes)~(cancelled)` serves the cancelled
date live at trailing, leading, and mid-line seats,
both routes (`~`, `…`, `~~`, and the fullwidth twins
`～` / `＊`); `(Monday)~(cancelled)` loses the record
with the governor. Controls hold: governor-first
forms judged, spaced form governs, the record seat
`(notes)~(Monday)` keeps its record. Governor-last
companies are the uncovered seat family.

### F5 (MAJOR) — Perf cliff: the nested shape in fullwidth-group dress (red team RT45-P1)

`nested_seq_mix_fwgroups` (the pinned nested shape
with fullwidth-dress trailing groups, judged since
R44-E): interleaved ratio medians 20.5× at n=1,600,
67.6× at n=6,400, and a single parse at n=25,600 did
not finish inside a 600 s driver timeout (control
~0.5 s; ratio >850× as a lower bound). The climb
steepens (~×3.3 per 4× step against the pinned
shape's ~×1.35): a new superlinear shape, unpinned,
hanging at the family's own reference size. The
pinned paren shape and the quoted residual measured
in-family in the same windows (observation O5 in the
red team report covers the quoted-bracket extent).

### F6 (MINOR) — The I15 split leaks at the governor-free leading sequence seat (review MINOR 1)

No leading-sequence reader consults
`quoted_members_as_parens` (its call sites are the
trailing run, the trailing sequence, and
base-trailing). `(Monday) "notes" November 1, 2026`
serves silently where its paren twin refuses with
the record — splitting *inside* the anchor gate that
confines I15 at the trailing seats. Quoted members do
compose at leading whenever a governor is in the
company, so the governor-free weekday sequence is
the one neighbour the consultation never reached.
Harm: lost record; in two shapes a framed date serves
where its twin refuses. No governed date serves.

### F7 (MINOR) — Zip64 plant, eighth generation: junk after the real directory defeats frame-closing (review MINOR 4)

The generation-7 craft plus 37 junk bytes between the
real directory and the zip64 record: the frame no
longer closes, `frame_closing_directory_start` finds
nothing, and the declared plant directory is judged
again (model-level witness; door harm is F9's
recorded refusal mechanism, not re-driven). Controls
hold (plain zip64-real, generation-7, signature arms,
legitimate signed shapes, tail-gap host). Same
family/preconditions/harm direction as generations
1–7.

### F8 (MINOR) — The space-width family splits at the leading-company inter-group seat (review MINOR 5)

`leading_group_sequence`'s inter-group trim admits
only `' '` and `'\t'`; all twelve other space
characters split a leading two-group company there
while twin-equal at the trailing, base-trailing, and
group→date seats. Witness: `(cancelled)\u{a0}(Monday)
November 1, 2026` serves with the governor unjudged;
the space twin refuses with the record. NBSP is the
most common pasted-space variant.

### F9 (MINOR) — CJK and mixed dress pairs split at the base-trailing and leading seats (red team RT45-C3)

The company-seat dress set is seven matched pairs.
Five matched CJK pairs (`《》` etc., plus `«»`) and
every mixed pair — including pure-ASCII
`(cancelled]` — split at base-trailing (resolution
serves silently / company reads unjudged) and at
leading (governs but the record is lost), while
agreeing at trailing and mid-line seats. On the
review lane's dress sweep (uniform limit at the arms
it probed) see adjudication 4.

### F10 (MINOR) — U+00AD splits the routes at the trailing-alone parenthesized seat (red team RT45-C4)

The admitted soft hyphen agrees with its twins at
every seat except one: `next Sunday (November 8,
2026)\u{ad}` loses the For route's parenthesized
reading entirely (no date, no record) while the Keep
route keeps through the date as clean. One admitted
mark, one seat, a route disagreement in the safe
direction.

### F11 (MINOR) — Flake-accounting gaps on non-batch pushes (verify F1; verify F2 folded as an arm)

Three hosted flake attempts on non-batch pushes
appear in no round-43/44 record: 40386848237 (r42e
outcome class, round 18, ubuntu, on the round-42
close-record push `86f3caa`), 40386848133 (ETXTBSY,
same push), and 40396061207 (ETXTBSY on `7626f6f`,
**never re-run — it stands red**). The standing rule
accounted batch tips; these rode close-record and
other non-batch pushes and fell outside it. Arm (b):
the first verify lane's extra-failure-runs.txt omits
runner run 40368442121 (the `c80f8ab` fmt incident's
runner leg — the incident itself is accounted in
`ae4064e`'s commit message; the omission is in this
round's lane evidence and is corrected by the
successor lane's report, recorded here so the ledger
is whole).

## Adjudications

1. **Race pin: production at fault; the pin stands.**
   The verify lane's CI-only judgment (pattern supports
   a scheduling cause; production not excludable from
   CI evidence alone) is superseded by the red team's
   direct evidence: both shapes reproduced locally and
   witnessed deterministically through the production
   functions in production order, with one root
   (torn create → `None` wildcard capture → rename
   step moving a file it never judged). The round-44
   carry's fix direction (test composition) is
   therefore NOT taken: F1/F2 are production findings
   and their fix is a production fix. The named
   target's constraint holds unchanged: the r42e pin's
   assertion (exactly one holder per round) must not
   be weakened or restated to accommodate anything.
   Pin hygiene from the red team's sketch (start
   barrier, longer peer lifetime) is secondary and
   optional, only if it touches no assertion.
2. **F3 grade MAJOR** (red team) over MINOR (review):
   the review arm covered the suffix seats; the
   inside-word governor flip and the invisibility of
   the class carry the higher grade. One finding,
   three arms, one cure direction: define the
   invisible/format set once (the class the evidence
   enumerates) and shed it at every seat the admitted
   pair is judged at, plus the boundary/interior
   seats arm (b) witnesses.
3. **F2 stands as its own MAJOR**, not folded into
   F1: same root, different harm (liveness vs safety)
   and different pin witnesses; the fix batch is
   shared. The F7 standing record (the rename-step
   window rate, 5/400 sleep-lander) is a different
   window and is not displaced by F1/F2.
4. **F9 vs the review lane's dress observation:** the
   review sweep found the exotic dress pairs uniform
   at the arms it probed (lone governors govern;
   company seats split as F4-of-round-44 recorded).
   The red team's witnesses are two-member companies
   at base-trailing and the leading record seat,
   which the review sweep did not witness. No
   contradiction on the overlapping arms; F9 stands
   as MINOR scoped to its witnessed arms, and its
   batch pins exactly those arms.
5. **Verify F2 is an arm of F11**, not a twelfth
   finding: it is a completeness defect in a lane
   evidence file, corrected in-lane; the durable
   defect is the accounting gap in F11 proper.
6. **HF-12 second slice is DEFERRED this round.**
   The round carries a BLOCKER production fix in the
   lock machinery plus a perf cliff; the slice
   (`from_json` typed version refusal, then IDML)
   does not displace them. It carries to round 46 as
   the first slice candidate, maps classified by
   conversion position per the r44i correction. This
   is the standing-terms deferral, recorded here so
   the close can cite it.
7. **The still-red run 40396061207** (ETXTBSY on the
   superseded tree `7626f6f`) is dispositioned under
   F11: accounted here by name and class; not re-run
   (its tree is two rounds superseded; the class is
   the standing spawn flake on a lock test, and the
   current tree's hosted legs are green). It stands
   in the ledger as red-on-a-superseded-non-batch-tip,
   not as an open defect.

## Batch plan

| Batch | Findings | Shape |
|---|---|---|
| R45-A | F1 + F2 | Lock creation/judgment fix (production). Direction per the red team sketch, writer's choice with evidence: structural (write-temp-then-`link()` atomic creation, so a capture at the lock path always reads a complete generation) preferred; the layered alternative (rename-step generation re-check + empty-file-as-in-flight in the claim loop) acceptable only if it kills BOTH witnesses. Pins: new deterministic step-level pins for the L45a/L45b/L45d interleavings (must fail on the base tree), the r42e pin passing with its assertion untouched, every standing lock/retirement pin unweakened, recycled-pid boundary untouched. |
| R45-B | F3 | Invisible/format class: one enumerated set shed at every judged seat (suffix, inter-group, boundary, date-interior, inside-word arms per the witnesses). |
| R45-C | F4 | Suffix shed between an inert group and a following governor (governor-last companies) at trailing/leading/mid-line seats. |
| R45-D | F5 | Fullwidth-dress nested cliff: bound the shape; ratio pins in the standing interleaved form. |
| R45-E | F6 + F8 | Leading-seat consultations: quoted-member re-dress for the governor-free leading sequence; space-width family in the leading inter-group trim. |
| R45-F | F9 + F10 | Dress pairs at base-trailing/leading company seats (scoped to witnessed arms); U+00AD trailing-alone route agreement. |
| R45-G | F7 | Zip64 frame-closing vs junk standing after the real directory (eighth generation). |
| R45-H | F11 | Record correction only (no code): flake-ledger addendum in the round records accounting the three non-batch attempts + the evidence-file omission; the still-red run dispositioned per adjudication 7. |

Every batch under the standing discipline: one writer,
commit + push + HEAD == origin, `cargo clean`, cold
verification from the pushed tree, fuzz claims
byte-identical to the standing baselines (verdict
`797758443d101f9339fdd778e14af430`, composition
`e8b2e73b117c7013d867419da8fb0ab7`, rights
5083/1024/5275 safety_flips=0), hosted CI green on all
three OS legs for the tip with any flake event named
verbatim in the batch record, no tip called verified
on local evidence alone.

Carried standing items not re-graded this round (they
stand as recorded in the round-44 close): F7's
rename-step window rate record; the L10/L11
observation; the peeled-tail residual; split-position
governor uniformity; lock leading-newline pid
spelling; recycled-pid residual; the R40-C calibration
note; the seven-mark exotic limit (review re-probed:
no eighth sibling among 31 candidates); R44-C's I15
split proper (F6 is its leading-seat leak, a separate
arm); the `nested_seq_mix_brackets` witnessed-standing
sibling and the quoted residual at 37× (red team O5
extends its extent, mechanism unchanged); adjudication-8
observations as re-verified by the review lane (emoji
uniform garnish limit; `(off)~` asymmetry
word-independent; exotic dress pairs uniform at the
arms the review probed — F9 scopes the exception).

Majors trend: … 5 (R42) → 3 (R43) → 3 (R44) → this
round's audit 1 BLOCKER / 4 MAJOR / 6 MINOR, judged on
round 46's audit for the exit bar if all batches land.
