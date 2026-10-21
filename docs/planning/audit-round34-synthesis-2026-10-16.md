# Audit round 34 — synthesis (2026-10-16)

Audited tree: `e7196c1` (round-33 close record; code tree
`601776c`). Three blind lanes: verify, review, red team.

**Deduped verdict: 0 BLOCKER / 5 MAJOR / 3 MINOR.**

**The exit bar is NOT met.** A full round at zero BLOCKER +
zero MAJOR is required; round 34 found five MAJORs. Majors
trend: 10 → 6 → 6 → 1 → 4 → 3 → 2 → 5. Round 35 is the
next candidate. The Tier-2 final gate is not triggered.

Lane verdicts: verify 0/0/0 (every standing claim
reproduced exactly); review 0/2/3; red team 0/3/3 plus
one proposed grade (RT34-4). Dedup: the bracket finding
is review MAJOR-2 and red team RT34-4 and the bracket
half of RT34-2, counted once (F3); the zip fallback
steering is review MINOR-1 and red team RT34-3, counted
once at the higher grade (F2, rationale below); the
garnish finding is the paren half of RT34-2 with review
MINOR-2 and RT34-5 folded in as sub-families (F4). Two
lane disagreements were adjudicated by coordinator
territory re-check (see F4 and F5).

## What held (lane evidence, stated so the fixes do not re-open it)

- Every standing claim reproduced firsthand by verify:
  suite 986/0 exit 0 (debug, 13m28s), clippy exit 0,
  booklet pin exact (31,717 imposed / 34,977 reader),
  both fuzz bins at recorded source md5s
  (`a9cac6c3…` / `4f85c605…`) with stdout md5s
  identical to the round-30 close baselines
  (`4941c4f91e8025e2b282efdbed6a663c` /
  `efd2096b198ec89fc36c2917b904c3f8`), 0 panics on
  40,000 + 497,024 lines; rights axes (M8) held,
  byte-identical to round 33, twelfth consecutive
  round. R33-A–D spot-verified in code, all named
  pins `ok`.
- R33-A's pinned interleavings under red-team attack:
  the third-claimant scenario, two-displaced-generation
  sequences, 50 concurrent claim-vs-restore races — no
  live generation stranded outside F5's one cell.
- R33-C's arithmetic derivation at every door, plain
  and no-sector zip64, shifted and unshifted, against
  fake-signature payloads; the .mxl/.obd head-sniff
  refusal of shifted bytes re-verified.
- R33-D's seven shapes plus five new siblings linear
  in release at load ~4–7; no shipped shape
  superlinear (F7 grades the pins' sampling, not the
  code). Panic hunt: 28,004 hostile cases across the
  contribution and archive surfaces, 0 panics —
  outside the one helper F1 names.
- Rights safety axes under a dedicated red-team
  checker: 6,760 route/order outcomes, `children_
  present` always the inputs' OR, no permission
  composed that no input vouched, 0 violations.
  (F6 is about license *choice*, not safety.)
- Review posture sweep: unsafe only at the three
  recorded sites; network only in the ob-fonts
  catalog client; no new document-path reads; HF-12
  untouched, not filed.
- The recorded recycled-pid residual (round-32 F6/F9
  family) was not re-filed; no new mechanism in its
  family was found. RT34-1/F5 is a different
  mechanism (a decision-table cell) and is filed as
  new on that basis, as the red team stated.

## Findings

### F1 (MAJOR) — The zip64 fallback panics on crafted offsets (review MAJOR-1)

`hunted_directory` (`crates/ob-model/src/zip_directory.rs:
219` and `:239`) trusts two attacker-controlled u64
fields as loop positions: the locator's declared
record offset (`record_at + 4`) and a found record's
cd-offset (`at + 4`). A 98-byte skeleton (zip64 record
with size field 45, so the arithmetic derivation
refuses as designed) panics "attempt to add with
overflow" at both sites in debug; in release the
addition wraps and the following slice panics instead
(code-read). Reachable at all four zip doors, which
hand raw bytes to `prefix_free` before any reader
runs; no `catch_unwind` exists in production code.
At the font door it sinks the whole pack call, against
that door's "a bad file never sinks the rest"
contract. No data loss, no wrong content — a crash —
so MAJOR, not BLOCKER.

### F2 (MAJOR) — The hunted fallback can be steered to a wrong or no directory while the reader opens the archive (review MINOR-1 + red team RT34-3)

One root: the fallback validates nothing — the
arithmetic path's `directory_walks_to` proof has no
counterpart in `hunted_directory`. Two witnessed
failure modes on shifted, not-arithmetically-derivable
archives (zip64 record with an extensible data sector,
the documented fallback case):

- The central hunt takes the first `PK\x01\x02`
  at-or-after the declared cd-offset, which on a
  shifted archive stands inside the last entry's
  stored payload: signature bytes there become the
  directory, `prefix_free` strips nothing, and the
  doors' scans judge the fake record while the
  reader serves the real entries (review witness,
  debug probe; the reader opens this archive, unlike
  round-33 F3's shape).
- A fake `PK\x06\x06` planted in the last central
  record's *name* — inside the record hunt's window
  on shifted archives — with a bogus cd-offset makes
  `central_directory` answer `None`: the scans see
  an empty directory, and disagreeing duplicates are
  *served* at IDML (`idml-namefake-sector-shift.
  idml` opens and serves the second copy of the
  disagreeing duplicate story), while font and clip
  duplicates silently collapse to one copy (red team
  witnesses at all three shifted doors; controls —
  unshifted, no-sector shifted, clean-name shifted —
  all judge correctly).

Grade: MAJOR, raised over the review lane's MINOR.
The review lane graded on the round-32 F4/F5/F6
precedent (scan-vs-reader disagreement on crafted
archives); the red team's witness exceeds that
precedent in the respect the review lane itself
flagged in F3's case: the duplicate *judgment* three
rounds built (R31-F5 / R32-D / R33-C) is not merely
inconsistent here, it is absent while the content it
exists to refuse is served. Silence is the defect.

### F3 (MAJOR) — Bracket groups do not compose with cancellation or garnish the way paren groups do (review MAJOR-2 + red team RT34-4 + RT34-2 bracket half)

Brackets are group delimiters nowhere in the engines:
every extractor is paren-only (`balanced_spans`,
`split_last_trailing_group`, `strip_trailing_
parenthesized`); the only bracket machinery is
token-level (`is_bracketed_token`, whose doc states
brackets are NOT group delimiters, and
`bracket_span_mask`, feeding only the bare tests).
Bracketed text therefore composes on the plain
surface, and the readings diverge from the paren
twins in both directions, witnessed across 35 shapes
(red team, `bracket34.log`) plus the review lane's
discriminators:

- A cancelled bracket date stays live:
  `November 1, 2026 [November 8, 2026, cancelled]`
  records Nov 8 as a live second date; the paren
  twin records nothing. Same for cancelled ranges
  (both range dates stay live), `canceled`,
  `postponed`, no-comma forms, double brackets, and
  a cancelled bracket date standing before the core.
- A bracketed cancellation word kills the core:
  `November 1, 2026 [cancelled]` reads no date at
  all; `(cancelled)` keeps Nov 1. The cancellation
  word is not the mechanism — `[choir]` and
  `[foo bar]` kill Nov 1 identically: a trailing
  bracket group is stripped by no site, so it stays
  in the core text and every core reader trips on
  it, while the bracketed token itself cleans to
  nothing at token level.
- Nesting flips the headline: `[(November 8, 2026),
  cancelled]` makes the cancelled date the Sunday.
- Bracket garnish (from RT34-2): glued
  `2026[Monday]` yields no date and no record where
  the spaced form records the mismatch; glued
  agreeing `2026[Sunday]` yields no date and no
  record where spaced reads the date clean. The
  agreeing case silently deletes a correct Sunday.

Curly braces behave as brackets in the first family
(review probe), consistent with `is_bracketed_token`
treating the pairs alike.

**Intent adjudication (coordinator, on the record;
the coordinator seat may overrule before R34-E
runs).** Bracketed groups in contribution text are
to be judged as their paren twins. Grounds: (1) the
token machinery already assigns brackets the aside
role — a bracketed token cleans to nothing at every
token site, exactly as garnish does; the current
split, where the same bracketed words are too weak
to cancel a date yet strong enough to erase one,
is not a semantics a sender can be read as meaning
(round 33's recorded words). (2) The cost asymmetry
offends the standing posture: a cancelled date
announced as a live alternative with no record it
was cancelled, and a plainly written date lost in
silence. (3) Sender plausibility: brackets are the
ordinary editorial mark for exactly this annotation
in hand-written office text, the plausibility class
already accepted for R33-B/F2. Scope rulings: the
paren range reading (a cancelled range splits: the
cancelled date records as a mismatch, the rest
stands) is inherited by brackets as-is — the red
team flagged that "read like parens" needs the range
case decided, and it is decided for consistency,
not re-opened; the pair types are the ones the
token machinery already treats as brackets (`[]`,
and `{}` where it treats them alike). Mid-line
glued/spaced divergence recorded by the red team
(`November(Monday) 1, 2026` glued reads where spaced
reads nothing) is NOT part of this finding: it is a
glued/spaced question, not a bracket question, and
is carried as an open observation for round 35.

### F4 (MAJOR) — The glued-garnish gate stops one shape short: extra closing marks, doubled garnish, trailing separators and punctuation (red team RT34-2 paren half + review MINOR-2 + RT34-5)

`spaced_glued_weekday_garnish` (composition.rs,
R33-B) re-spaces only when the line, after trimming
trailing `.`/`,`, ends in `)` and the group's whole
contents are one weekday word. Past that fence (43
glued/spaced pairs, `garnish34.log` /
`garnish-keep34.log`; 37 For-line pairs and 4 of 5
Keep-through pairs SAME, including every R33-B
pinned shape):

- **Extra closing mark** (MAJOR weight): `November
  1, 2026(Monday))` and `…(Monday)]` read the date
  clean with no record; the spaced twins are refused
  and record the WeekdayMismatch. A doubled closing
  parenthesis is an ordinary typing slip.
- **Doubled garnish** (MAJOR weight): `November 1,
  2026((Monday))` glued reads clean; spaced refuses
  and records (log pair `nested-garnish`, DIFF).
  Adjudication note: the review lane's held section
  states the spaced twin of the nested form is
  judged identically; the red team's disk log shows
  the DIFF pair above. Observed behavior over the
  code-read claim: the witness stands, the held
  sentence does not cover this shape.
- **Trailing separator** (MINOR weight, review
  MINOR-2): `November 1, 2026(Monday) *` (and `|`)
  reads the date with no mismatch; the spaced twin
  is judged — the trailing-weekday scan skips
  non-alphanumeric tokens, the re-spacing gate does
  not. `*`/`|` are the format's own separators.
- **Trailing punctuation** (MINOR weight, RT34-5):
  a disagreeing garnish followed by `;`, `:`, `!`,
  `?`, or `"` is judged in *neither* form — the
  trailing-word machinery's trim set stops before
  these marks, although `weekday_word` honors `;`
  and `:` as boundary endings elsewhere in the same
  parser.

Grade follows R33-B/F2 (MAJOR) on the two
twin-divergent sub-families; the batch fixes all
four at the one gate.

### F5 (MAJOR) — `retire_journal_from`'s displaced hold does not mirror the lock path for a tokenless journal (red team RT34-1; coordinator-confirmed)

R33-A's stated rule: a live generation at the
displaced path "holds its owner's journal from the
displaced path exactly as it would from the lock
path". The displaced hold is computed as
`displaced_holds_the_journal`, whose `None` arm
(a tokenless journal — a legacy, pre-token note) is
only `displaced_names_live_holder_by_pid`, which is
true solely when the displaced file is itself
tokenless. At the lock path, the decision table's
`None` arm blocks retirement whenever any live
foreign holder stands there. So the one unmirrored
cell is (tokenless journal × token-carrying
displaced owner): the lock-path holder retires the
journal while the displaced owner lives. Witness
(stepped replica of the verbatim-extracted lock
functions, real child pids, `lockstep34.log`): T1
allowed=true, journal cleared, A alive; T1b control
(same A at the lock path) refused; T1c (A-token
journal) and T1d (pid-only displaced A) hold.

Coordinator territory re-check, firsthand: in the
shipped `retire_journal_from`, the `None` arm of
`displaced_holds_the_journal` reads exactly
`None => displaced_names_live_holder_by_pid`, and
`displaced_owner_is_a_live_other` (pid-based,
token-independent) is computed immediately above and
used only in the `Some(token)` arm. The asymmetry
is in the shipped code, not an artifact of the
replica. The review lane's held verdict on R33-A
described the displaced-file-tokenless cell and did
not test this cross-cell; the witness stands.
Preconditions are narrow (the R33-A displaced state
plus a legacy tokenless journal note) and the
consequence is R33-A/F1's own: a live peer's journal
deleted by another instance.

Recorded inside this finding, ungraded: the raw
rename step (`rename_instance_lock_to_displaced_
from`) begins by deleting whatever stands at the
displaced path, unchecked (red team T5, in
isolation); every full-path interleaving the lane
constructed routes through the generation check or
the audit, and the review lane's reachability
argument (depositing a live displaced generation
requires the lock path the states contradict)
stands unrebutted by any full-path witness. The
step's safety is borrowed from its callers'
ordering — noted for the fixers' attention, not a
batch term.

### F6 (MINOR) — Rights merge license choice is route-dependent for one triple family (red team RT34-6)

The safety axes held (above). What differs by route
is which license stands when two permission-free /
BY licenses compete before any credit arrives: 9
triples where assembling by sequential `add_asset`
(the chain) and by the duplicate fold give the same
bytes different license facts (e.g. chain ends at
`Cc0`, fold at `CcBy("4.0")`, same credit standing).
Both outcomes are individually safe; no safety fact
(children, permission, credit presence) moves. A
document's license fact should not depend on the
route that assembled it.

### F7 (MINOR) — The R33-D ratio pins sample one size (red team RT34-7, design note)

The seven pins measure at exactly 8,000 groups. A
genuinely quadratic term contributing ~12x of
control at 8,000 passes the loosest pins and costs
~100x the control at 64,000 groups — a size a
contribution line can actually carry under the
open-door caps. This grades the pins, not the code:
no shipped shape is superlinear (red team release
measurements, load ~4–7, all ratios under bounds;
the converse failure — a linear implementation
failing spuriously — was not observed). The two
historical regressions in this family (R31-C /
R32-C) were exactly small-coefficient rescans.

### F8 (MINOR, maintainability) — The central-directory walk remains in four byte-identical copies (review MINOR-3)

R33-C shared the derivation in `ob_model::
zip_directory`; each door still walks records with
its own `central_spots` (idml.rs:556, ob-fonts
lib.rs:374, ob-clips lib.rs:400, ob-music
parse.rs:326) — verified byte-identical (normalized
hashes agree), with `raw_payload` companions modulo
each door's cap constant. No present divergence;
this is the same per-door copy surface whose
derivation half drifting apart was round-32 F4, and
any walk fix (F2's validation, say) must currently
land four times.

## Dispositions — fix batches

One writer at a time, in this order. Every batch
under the standing batch-boundary discipline (Jacob,
2026-10-20; recorded at `12176df`): after the batch
commits and pushes and HEAD == origin/main is
confirmed on its commit, `cargo clean` (registry
never pruned), then the batch's verification runs
**cold** from the clean tree — full unpiped suite +
clippy `-D warnings` + booklet pin, plus the fuzz
bins re-run as claims where the batch touches the
contribution engines. Record per batch: disk free
before clean / after clean / after cold rebuild, and
the cold run's wall time from log timestamps.

- **R34-A** — F5 (MAJOR). The `None` arm of the
  displaced hold mirrors the lock path: a tokenless
  journal is held by a displaced file naming any
  live other process — the already-computed
  `displaced_owner_is_a_live_other` joins
  `displaced_names_live_holder_by_pid` in that arm.
  RED-first in-suite pins for the T1/T1b/T1c/T1d
  cells; every standing lock/retirement pin
  unweakened. The T5 rename-step note is
  attention-only, not a batch term.
- **R34-B** — F1 + F2 (MAJOR). Harden
  `hunted_directory`: checked arithmetic at every
  position computation in the hunt (no overflow in
  debug, no wrap in release); a hunted zip64 record
  is validated against the locator that follows it;
  a hunted directory is validated by a walk (the
  `directory_walks_to` standard) before it is used,
  and the hunt keeps searching past a candidate that
  does not validate; where no directory validates,
  the derivation answers as it does for underivable
  bytes, and no door may scan an empty or fake
  directory while its reader serves the entries —
  a shifted, reader-openable archive with no
  derivable directory refuses at the doors in the
  standing refusal words (the fail-closed direction
  RT33-3 already established as acceptable). Pins:
  both panic skeletons, the fake-record witness,
  the name-fake witnesses at IDML / fonts / clips
  (and .mxl where reachable), all round-33 F3 pins
  unchanged in outcome.
- **R34-C** — F4 (MAJOR). The garnish gate learns
  the four sub-families: the re-spacing tolerates
  extra closing marks and a trailing separator
  token after a glued garnish; a doubled garnish is
  judged as its spaced twin; the trailing-word trim
  set extends to `; : ! ? "` so the garnish check
  fires in both forms. RED-first twin pins per
  sub-family; every non-garnish glued group and
  every spaced line keeps its text and readings
  byte-for-byte. Fuzz-as-claims gate applies: any
  stdout movement is reported before commit, not
  absorbed.
- **R34-D** — F6 + F7 + F8 (MINOR). Rights: one
  standing license-choice rule stated at the merge
  site so fold and chain choose the same license
  for the witnessed triple family (safety axes and
  their pins untouched). Timing pins: a second
  measurement size (doubling check) joins the seven
  pins, without loosening the 8,000-group bounds or
  re-framing any recorded number. `central_spots`:
  the walk single-sourced into ob-model beside
  `zip_directory`; per-door caps stay at the call
  sites; the doors' existing pins are the
  byte-identity proof.
- **R34-E** — F3 (MAJOR), LAST and separately gated:
  implements the adjudicated reading above —
  bracket groups compose, strip, and cancel as
  their paren twins across the witnessed family,
  bracket garnish included. RED-first pins from
  the 35-shape family and the bracket-garnish
  pairs, built before the change. HARD TERM: the
  fuzz bins re-run as claims before commit; if
  either stdout moves, the batch STOPS — no commit,
  no re-baseline — and the delta is reported
  itemized for the coordinator seat's decision.

HF-12: untouched, Jacob's decision. The mid-line
glued/spaced observation (RT34-2, recorded) carries
to round 35 ungraded.

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

## Close (coordinator, 2026-10-21)

All six batches landed (R34-D split into D and D2 at
the fuzz gate; see below). Suite: 986 at the round-33
close, **1011 / 0 failed** at this close. Booklet
baseline (31,717 imposed / 34,977 reader) exact in
suite at every batch. Per-batch records (cold wall
times, disk numbers, RED evidence) live in the audit
dir at `round34-out/coord/r34{a,b,c,d,d2,e}-record.md`
(D and D2 share `r34d-record.md`).

- R34-A `6a4606b` (F5, MAJOR) — 987 tests. The None
  arm of the displaced hold now mirrors the lock
  path (`displaced_owner_is_a_live_other` joins the
  arm); one RED-first pin covers the T1/T1b/T1c/T1d
  cells plus the gone-peer control; only T1 flipped.
  A VM restart mid-verification wiped the first
  cold attempt's logs; the writer discarded a
  not-cold clippy and re-ran the full cold sequence
  (suite 22m36s, clippy 4m52s) — the record
  documents both attempts.
- R34-B `3b7738a` (F1 + F2, MAJOR) — 1000 tests.
  `hunted_directory` rewritten validating: checked
  arithmetic everywhere, hunted records validated
  against their locator, hunted directories walked
  to the promised end, search continues past
  non-validating candidates; all four doors fail
  closed in standing words when the reader opens a
  non-empty archive whose directory cannot be
  derived. Two refusal-over-service choices
  recorded with reasons (a craft whose record does
  not close over its locator; the unaccounted-record
  shape, deliberately distinct from the accepted
  signature-record shape).
- R34-C `5e89273` (F4, MAJOR) — 1005 tests. The one
  garnish gate widened (suffix layers peeled: extra
  closing marks, doubled garnish, trailing
  separator, extended trim set); 5 twin pins; fuzz
  byte-identical, nothing re-baselined.
- R34-D `a5da987` (F7 + F8, MINOR) — 1005 tests.
  Timing pins measure at 16,000 groups as well as
  8,000 (bounds and the R33-D header untouched);
  `central_spots` single-sourced into ob-model,
  door counts reproduced exactly.
- R34-D2 `ff32ffa` (F6, MINOR) — 1006 tests. The
  batch first stopped at its own fuzz gate: the
  initial global-precedence rule fixed the 7
  permission-free witnesses but RAISED the rights
  bins' route-disagreement counters (triple_diff
  5823→6747). Coordinator adjudication: not cleared
  — F6's substance is route-invariance, and a rule
  that widens measured divergence does not land.
  The narrower rule (the fold remembers the
  permission claim its merges stood while the
  credit line was empty) passed the gate outright:
  counters FELL on all three measures (5823→5083,
  1155→1024, 6015→5275), safety sections
  byte-identical, witnessed family agreeing. Two of
  RT34-6's nine triples are the round-8 deferral
  residual, pinned as fold≠chain by the standing
  r8 pin — adjudicated out of F6's scope, untouched.
  Record correction, disclosed in `r34d-record.md`:
  a garbled exec poll made the writer report
  R34-D cold numbers that never ran; the real cold
  suite for `a5da987` overlapped the D2 edits and
  failed on the D2 RED pin (the expected RED state).
  `a5da987`'s content received cold-from-clean
  verification as the base of `ff32ffa`'s completed
  cold run (1006/0); the record carries the
  polluted run's real numbers and citation.
- R34-E `197ab50` (F3, MAJOR) — 1011 tests. The
  adjudicated reading implemented: brackets/braces
  are group delimiters in the shared machinery
  (stack-based spans step-equivalent to the old
  depth count on paren-only text, so no paren
  reading moved); 5 twin pins over the witnessed
  family; pin flips bracket-side only, each
  probe-verified against its paren twin's standing
  record. The batch STOPPED at the fuzz gate as its
  hard term required — both contribution stdouts
  moved — and the coordinator ruled on the itemized
  delta:

**Re-baseline ruling (coordinator, 2026-10-21).**
The movement is the adjudicated reading made
visible, and the fuzz baselines are re-set on it.
Deltas: verdict27 stdout `4941c4f9…` → `7bcbfdc0…`
— 40,000 lines and panics=0 unchanged, every EX line
byte-identical, exactly 3 counter lines differ, all
down (I4-anchor-diff 5614→5493, I4b-anchor-keep-diff
5462→5336, invariant_violations 11076→10829).
comp27 stdout `efd2096b…` → `96b0c7b2…` —
D_TOTAL=497024, D_PANICS=0, D_STANDING_RELATIVE and
D_RESURRECT_GOVERNED unchanged; for_none_keep_some
1797→2357 and soup 470→502 (bracket groups now
contribute on Keep lines, as adjudicated), and an
EX-sample rotation whose incoming examples are all
bracket shapes and whose outgoing are a sampling
artifact of the class's growth. rights27/rights27s
byte-identical. No non-bracket example changed; no
total moved. New standing baselines from this close:
verdict27 `7bcbfdc0306f9efc642bb03462cba385`,
comp27 `96b0c7b2ed7eedae6f8dcfa64b0d88ff` (bin
sources unchanged at their recorded md5s). Round
35's verify lane claims against these, with this
provenance.

Carried to round 35, recorded in `r34e-record.md`:
(1) mixed closers in the bracket family (verified
byte-identical to `bracket34.log` — deliberately
unchanged by R34-E); (2) a cancellation word
wearing brackets INSIDE a group inner does not
govern there (its round-24 garnish role stands;
the adjudication made the pair a delimiter, not a
rewrite of the inner token rules it cited as
premise). Plus the round-34 open observation: the
mid-line glued/spaced divergence (`November(Monday)
1, 2026`), ungraded, not a bracket question.

Five-leg evidence on the close tree `197ab50`:

1. **Local, cold** (landing pass, `round34-out/
   coord/r34e-land-*` logs): after `cargo clean`,
   full workspace suite **1011 passed / 0 failed**
   (exit 0, 11:18:08Z → 11:41:36Z, wall 23m28s),
   booklet pin ok in suite; clippy `--workspace
   --all-targets -- -D warnings` exit 0
   (11:41:36Z → 11:45:23Z, wall 3m47s). A VM restart
   killed the first cold fuzz build; re-run cold,
   all four bins at the landed values.
2. **Fuzz as claims:** bins at recorded source md5s
   (verdict27 `a9cac6c3…`, comp27 `4f85c605…`):
   verdict 40,000 lines, panics=0, stdout md5
   `7bcbfdc0306f9efc642bb03462cba385`; composition
   D_TOTAL=497024, D_PANICS=0, stdout md5
   `96b0c7b2ed7eedae6f8dcfa64b0d88ff` — the new
   baselines set by the ruling above. Rights axes
   (M8) held for the thirteenth consecutive round:
   rights27/rights27s byte-identical to the D2
   references, safety_flips=0 throughout.
3. **Hosted CI** on `197ab50`: run `39590890242`,
   **success** — test (ubuntu-latest) ✓ job
   119191256238, test (macos-latest) ✓ job
   119191256614, test (windows-latest) ✓ job
   119191256621.
4. **Desktop runner build** on `197ab50`: run
   `39590890220`, **success** — build ✓ job
   119191256402.
5. **Windows bundle** on `197ab50`: run
   `39598049259`, **success** — Gate (format, tests,
   clippy) ✓ job 119215208945, bundle ✓ job
   119218886733; dispatched by the coordinator via
   `gh workflow run` on main at the close commit;
   confirmed via `gh run view` / watch exit 0.

Round 34 audit verdict stands at 0 BLOCKER /
5 MAJOR / 3 MINOR — the exit bar is NOT met by this
round either; all eight findings are fixed or
dispositioned above. Round 35 audits the closed
tree `197ab50`. The Tier-2 final gate is not
triggered. HF-12 remains Jacob's decision,
untouched. The build-cache policy's first
application (latched clear at this confirmed
completion) is executed and recorded in the close
status note in the audit dir.
