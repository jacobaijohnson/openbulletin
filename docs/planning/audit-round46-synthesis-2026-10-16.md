# OpenBulletin convergence round 46 — synthesis

Audited tree: `f8b7d5a` (round-45 final code tree; `4ab0003` is only the
round-45 synthesis close record — its diff against `f8b7d5a` is the
synthesis file alone, confirmed by all three lanes).

## Tally (deduped)

**1 BLOCKER / 2 MAJOR / 6 MINOR. The exit bar is NOT met** — the bar is
a round auditing at zero BLOCKER and zero MAJOR, and this round's
audit carries a BLOCKER. Round 47's audit judges the bar.

| Lane | Tally | File |
|---|---|---|
| verify | 0 / 0 / 0, PASS | `round46-out/lanes/verify.md` |
| review | 0 / 0 / 3 | `round46-out/lanes/review.md` |
| red team | 1 / 2 / 3 | `round46-out/lanes/redteam.md` |

Verify reproduced every standing claim from its own runs: suite
1299/0, cold clippy exit 0 (312 Checking lines; the fingerprint-fresh
5.82 s run was not relied on), booklet 31,717 / 34,977 exact, fuzz
byte-identical (verdict `797758443d101f9339fdd778e14af430`, 40,000
lines, 0 panics; composition `e8b2e73b117c7013d867419da8fb0ab7`,
497,024 lines, 0 panics; rights 5083 / 1024 / 5275,
safety_flips=0), hosted CI 40909651798 / runner 40909651865 / bundle
40923123498 all confirmed on `f8b7d5a`, matching the round-45 close
record. Review verified the round-45 deltas complete for their
witnessed scope (pin arithmetic 1275 + 24 = 1299; no assertion
weakened outside the strictly-stronger «» re-base).

## Findings

### F1 BLOCKER — Two holders, deterministically, through the production lock steps (red team RT46-L1)

The rename-window residual behind the r42e events is no longer
statistical only. The red team drove an interleaving step by step
through the production functions in production order on the fixed
(R45-A) tree and ended with two claimants that both completed a
winning create: B judges dead generation S and pauses mid-displacement;
M displaces S; C passes its judgments in M's displace→create gap
(vacant lock, dead displaced generation); M creates and holds; B's
rename step resumes and renames M→displaced (its generation check
passed against the earlier generation S, and the rename step verifies
nothing about the file it moves); C's create lands in the vacancy B's
rename made. Final state asserted by the probe: lock names C,
displaced names M, both creates `Created`, audit false. Every
generation is complete — no torn/empty/partial file anywhere, so R45-A
correctly does not touch it. Mechanism: the R33-A rename window
composed with the displace→create gap and the audit's
leave-displaced-beside-occupied outcome. The r42e pin's green run on
the same binary is recorded by the lane and explicitly NOT a closure
claim. Closing this is the recorded rename-step contract redesign,
not a routine fix. Re-run-and-forget is closed as a disposition for
it, again.

### F2 MAJOR — Spaced governor-doorstep suffix still disarms the governor (red team RT46-C1; carried item (a) CONFIRMED and wider)

R45-C sheds a suffix glued between an inert group and a following
governor, gated on the buffer ending in a closer immediately before
the suffix run. The spaced neighbour `(notes) ~(cancelled)` — suffix
spaced from its group, glued to the governor — still serves live at
trailing, leading, and mid-line, for every governor word tried, on
both routes. The hole is the R45-C suffix set PLUS `#`, and it is
spacing-agnostic (double space, tab, NBSP, U+2003, U+200B, U+FEFF all
serve); invisible/format characters on either side of the doorstep
suffix break even the glued cure. Trailing weekday-first also loses
the weekday record. Leading weekday-first is correct (the leading
company judgment catches it). Governor-first spaced neighbours are
correct. Suffixes `*`, `**`, `-`, `—`, `!` spaced forms govern
silent (shed by another path). Dress asymmetry, mechanism note: an
exotic-dressed inert group governs (the exotic company path judges
the governor anyway); an exotic governor does not save an ASCII-inert
line. The hole lives on the ASCII unwrap-splice path.

### F3 MAJOR — The Cf class beyond the R45-B 19 still splits every boundary/separator seat (red team RT46-C2)

R45-B stated the invisible/format set once as 19 codepoints and shed
it at the trims, but boundary/separator recognition — trailing group
tails, date↔group boundaries, the leading group→date boundary, the
base-trailing intergroup seat, trailing two-group seats, and the
two-date choice — still keys on that 19, so every other Cf codepoint
splits there. Tested: the 27 genuine Cf codepoints outside the 19
(U+00AD, U+0600–U+0605, U+06DD, U+070F, U+0890, U+08E2, U+2061–U+2063,
U+2067–U+2068, U+206A–U+206F, U+FFF9–U+FFFB, U+110BD, U+110CD,
U+E0001, U+E0020, U+E007F). Governed dates serve live, weekday
records are lost, and at the two-date seat the served date flips —
both routes, at six seat families. Seats that are Cf-clean, narrowing
the mechanism: inside the governor word (token cleaning strips full
Cf there), the leading intergroup seat, and the weekday-word
remainder re-parse (32/33). The 19-set controls hold. U+00AD is the
sole handled exception at most seats (R45-F soft-hyphen handling) but
flips the served date at the two-date seat like the rest. The R45-B
cure's "nineteen, no more" was the witnessed set, not the class.

### F4 MINOR — Variation-selector / combining marks split inside-word and intergroup seats (red team RT46-C3)

Outside Cf entirely, U+2065, U+FE00, U+FE01, U+FE0E (Mn), U+0301 (Mn),
U+0610 (Mn) split seats the Cf class does not: inside the governor
word (governor disarmed → live date), the leading intergroup seat,
and (U+0610 only) the remainder re-parse and date interiors (no date,
no record). MINOR, not MAJOR: these marks were never in R45-B's
claimed class and their paste paths are narrower, though U+FE0E
(text-presentation selector) does occur in real emoji-heavy paste.

### F5 MINOR — Governor-free leading sequences in exotic dress still lose the record (red team RT46-C4; carried item (c) REPRODUCED)

R45-F's re-dress consultations are exactly two seats (base-trailing
garnish walk, leading governor-company judgment). The governor-free
leading sequence is neither, and it still loses the record in every
R45-F dress pair, both orders, glued and spaced — plus the same loss
at trailing/mid-line, the agreeing-shape loss (a date the twin serves
is lost, safe direction), and two serving arms: base-trailing
governor-free serves the resolution silently for 12 of 20 pairs
(mixed pairs), and leading notes-first pure-ASCII mixed pairs serve
where the twin refuses. No GOVERNED date serves (governed exotic
companies all refuse with the record). The unlisted pairs (「」,
〖〗, 〘〙, 〚〛, ‹›) behaving this way is standing scope (R45-F:
"No other pair is admitted"), not a separate finding.

### F6 MINOR — Zip64 junk arm, ninth generation: a complete directory in the junk steers the frame (red team RT46-C5)

R45-G's junk arm scans candidate directory ends backward and returns
the FIRST start that walks the declared size — the complete directory
NEAREST the record. A complete directory standing in the junk after
the real directory therefore wins over the real one, in both the
wrongcd shape and the zip64-real-with-junk shape (the direct declared
path does not save it: the real directory ends short of the record,
so the exact arms fail and the junk arm fires). A fake BEFORE the
real directory loses to the real one, confirming nearest-wins. The
demonstrated steering is name-level with a shared local header; the
payload-level extension is reasoned, not demonstrated, and is NOT
claimed as observed. Fix direction is the fix lane's adjudication:
nearest-wins vs earliest-wins trades against other generations' pins.

### F7 MINOR — R45-A leaves orphan staging files; nothing sweeps them (review MINOR 2)

A claimant killed between stage and link (or between link and the
temp unlink on the losing path) leaves `openbulletin.lock.tmp-*` in
the app data dir forever: no startup sweep, no lock-dir cleanup, and
no reader ever looks at temp paths again. Harm is bounded (inert
litter; a collision self-heals at the cost of at most one
safe-direction claim refusal), which is why this is MINOR. Fix: sweep
`openbulletin.lock.tmp-*` at claim time, or document the orphan as
accepted litter.

### F8 MINOR — R45-F's base-trailing CJK redress feeds the quoted consultations, contrary to its comment, with no pin on the combination (review MINOR 3)

`dates.rs:895-910` (`base_trailing_sequence_mismatch`) comments that
"the quoted consultations below then read the garnish with its
groups standing as written", but the next statements compute
`quoted_members_as_parens` from the CJK-redressed garnish, and
`balanced_spans` is ASCII-byte-based — so a quoted span standing
inside CJK/mixed dress is consulted through ASCII brackets the
as-written garnish never formed. No wrong reading is demonstrated
(comment + coverage only, hence MINOR). Fix: correct the comment and
add one pin combining a quoted member with a CJK-dressed company at
base-trailing (agreeing and refusing shapes).

### F9 MINOR — Review MINOR 1, dispositioned without a code batch (see adjudication 2)

The R45-D timing pin bound (`witness_best < control_best * 27`) sits
inside measurement noise: the round-45 close record's own cold run
measured 27.37x against the 27x bound. Dispositioned below, not by a
bound change.

## Failed falsifications (no finding)

- Carried item (b) CONFIRMED safe: weekday-first whole-cloth quoted
  lines read no date and mint no record across all 4 quote dresses ×
  6 weekday/date combos, both routes, with glued/double-spaced/
  NBSP-joined/invisible-joined/mixed-dress/three-group variants all
  safe; disagreeing weekday-first refuses WITH the correct record;
  notes-first whole-cloth serves exactly as its paren twin. Item (b)
  is closed as an observation, not a finding.
- R45-D perf: linear to 4× the pinned size in fullwidth, CJK, and
  quoted dress (release, best-of-3); no new nested cliff.
- R45-F soft-hyphen guard holds; R45-B 19-set seats hold; R45-C
  controls hold; R45-G baselines hold; R45-A torn-create root is dead
  (the F1 witness uses only complete generations).
- Base-trailing is twin-equal, NOT a hole: `Sunday after November 8,
  2026 (notes) ~(cancelled)` serves 2026-11-15, but so does its clean
  twin — a notes+governor company does not block the resolution by
  standing semantics there. Weekday-carrying base companies do refuse.
  Fix batches must NOT "align" base-trailing with trailing by analogy.

## Coordinator adjudications

1. **F1 stands alongside review's R45-A-clean verdict; no
   contradiction.** Review judged R45-A complete for the torn-create
   root it was chartered against (its observation O1 names the
   rename-window residual as disclosed, not hidden). Red team has now
   evidenced that residual deterministically. The BLOCKER is the
   residual's, not a reopening of the torn-create root.
2. **Review MINOR 1's fix direction (raise the bound) is REJECTED
   under the standing no-widening rule.** No timing bound is ever
   widened to accommodate load or noise; the 27.37x cold-run
   exceedance was already dispositioned in round 45 by a green solo
   re-run with the bound untouched. F9 is dispositioned as accepted
   noise: the pin stands at 27x, and any future cold-run exceedance
   is dispositioned solo/quiet, never by widening. No code batch.
3. **The red team's worktree probe test must NOT be carried into the
   repo as-is.** The R46-A writer authors a clean repo pin for the
   witnessed interleaving (same production steps, same final-state
   assertions), and the pin must bite on the base tree (fail before
   the fix, pass after).
4. **F6's tie-break is the fix lane's adjudication**, recorded here
   so it is not lost: earliest-complete-directory vs nearest-wins
   trades against other generations' pins; the writer states the
   trade and pins both sides.
5. **Carried item (e) (run 40396061207, red on a superseded
   non-batch tip per round-45 adjudication 7) was NOT re-litigated**,
   per the brief. It is not an open defect.
6. **HF-12 slice 2** (round-45 adjudication 6 candidate: from_json
   version refusal, then IDML) is the first candidate after the fix
   batches, under the standing terms (typed core errors, plain-words
   rendering at the user boundary, gradual, crate by crate, no
   big-bang migration, no user-visible message changes). It gets its
   own batch letter and only if it does not displace the audit/fix
   sequence.

## Batch dispositions

| Batch | Findings | Notes |
|---|---|---|
| R46-A | F1 BLOCKER | Rename-step contract redesign. Single writer first. New step-level pins for the witnessed interleaving must bite on the base tree. The r42e pin's assertion stays untouched. |
| R46-B | F2 MAJOR | Spaced doorstep suffix set + `#`, spacing- and invisible-agnostic. Base-trailing stays twin-equal (see failed falsifications). |
| R46-C | F3 MAJOR + F4 MINOR | Boundary/separator recognition keyed on the full Cf class (F3) plus the adjacent Mn/VS marks (F4) at the same seats. |
| R46-D | F5 MINOR + F8 MINOR | Governor-free exotic-dress arms (F5) and the quoted-consultation comment + pin (F8); both live in the dress/consultation machinery. |
| R46-E | F7 MINOR | Orphan staging sweep at claim time, or a documented accepted-litter ruling. |
| R46-F | F6 MINOR | Zip64 junk-arm tie-break (adjudication 4). |
| R46-G | HF-12 slice 2 (conditional) | Own batch letter, only if it does not displace the sequence. |

## Close

**Round 46 closed. Audit tally 1 BLOCKER / 2 MAJOR / 6 MINOR; the
exit bar is NOT met** — the bar is a round auditing at zero BLOCKER
and zero MAJOR, and this round's audit carried a BLOCKER. All nine
findings are closed or dispositioned below; round 47's audit judges
the bar. Final code tip `db4209b`, close record this commit, suite
**1334 passed / 0 failed** on every leg that ran it.

### Batches landed and verified

| Batch | Findings | Commit | Hosted CI (all three OS legs) |
|---|---|---|---|
| R46-A | F1 BLOCKER | `eb619df` | 40957590729, success |
| R46-B | F2 MAJOR | `1f664ef` | 40975117415, success (attempt 2; see flake notes) |
| R46-C | F3 MAJOR + F4 MINOR | `92d469b` | 40991486313, success on attempt 3 (see the unsafe-direction event) |
| R46-D | F5 MINOR + F8 MINOR | `e0f4abf` | 41009677080, success |
| R46-E | F7 MINOR | `f3de1ff` | 41021156251, success on first attempt |
| R46-F | F6 MINOR | `075de32` | 41034069993, success (attempt 2; see flake notes) |
| R46-G | HF-12 slice 2, from_json half | `db4209b` | 41056961522, success on attempt 1 |

F9 MINOR was dispositioned without a code batch (adjudication 2:
the 27x bound stands; exceedance is dispositioned solo/quiet, never
by widening). R46-G landed the from_json half of HF-12 slice 2
(`FromJsonError`, Display rendering the standing plain words
byte-for-byte, byte-identical Display pins biting on the base tree);
the IDML half is carried (see carried items). Records:
`round46-out/coord/r46a-record.md` through `r46g-record.md`.

### The unsafe-direction r42e event — carried, NOT closed

This event is carried verbatim and is not softened, not closed, and
not adjudicated by anything in this round:

- Verified firsthand from the per-attempt Actions log
  (runs/40991486313/attempts/2/logs, Ubuntu):
  `settings_tests::r42e_four_claimant_races_over_a_dead_lock_stand_single_winner`
  FAILED, assertion "round 7: exactly one of four claimants holds
  (results [false, false, true, true])" — **TWO holders, unsafe
  direction**. It was the SOLE failure of that attempt's run (all
  other suites green, including the R46-A pins, which passed on the
  same attempt). The green attempt 3 does NOT close this.
- R46-A closed the witnessed RT46-L1 rename-window (B pauses
  mid-displacement, M displaces S, C passes judgments in the gap, M
  creates and holds, B's rename moves M→displaced, C creates into
  the vacancy). This attempt-2 event is a DISTINCT observation: it
  needs a deterministic witness/fix or a written adjudication,
  carried to round 47 as a named investigation target under the
  standing constraint (it may NOT close as re-run-and-forget).
  R46-A's acceptance stays scoped to RT46-L1 and does not adjudicate
  the lock generally. R46-C's acceptance is likewise scoped to
  F3/F4 only and does not adjudicate the lock.
- **Decision deferred, flagged for the parent seat: immediate
  diagnostic now vs the round-47 target.** This close does not make
  that call; it surfaces it with the evidence. Companion evidence
  for the investigation: the R46-E runner run 41021156201
  (superseded tip `f3de1ff`) showed the same pin in the SAFE
  direction (round 7, [false, false, false, false], zero holders);
  the R46-B and R46-F attempt-1 macOS failures were safe-direction
  r42e notes (see flake notes). The unsafe direction has now been
  seen exactly once, on hosted CI, on a tip whose lock code R46-A
  had already redesigned.

### Hosted flake events this round (named verbatim, per the standing rule)

- R46-A: ETXTBSY event, named in `r46a-record.md`.
- R46-B: CI 40975117415 attempt-1 macOS failure on the r42e pin
  (safe direction); failed-job re-run only, attempt 2 green.
- R46-C: CI 40991486313 attempt-2 Ubuntu r42e failure, unsafe
  direction (the event above); attempt 3 green. The green re-run is
  recorded, not treated as a disposition.
- R46-E: ETXTBSY event, named in `r46e-record.md`.
- R46-F: CI 41034069993 attempt-1 macOS failure, verbatim r42e
  safe-direction note; failed-job re-run only, attempt 2 green.
- R46-D and R46-G: no test flake events (R46-G green on attempt 1).

### Close verification — five legs, all firsthand

The previous close attempt was INCOMPLETE, not failed: its
`close-clean.log` shows CLEAN_EXIT=0 (2026-10-30 14:15 UTC) but its
`close-suite.log` ends mid-run (last write 2026-10-30 14:40 UTC, no
exit line), and the process was verified dead on 2026-10-31. The
truncated suite log is NOT evidence of anything; no verdict was read
from it. The prior attempt's logs are archived at
`round46-out/coord/logs/prev-attempt-2026-10-30/`, and this close
re-ran the full cold verification from scratch on the pushed tip
`db4209b` (script `round46-out/coord/close-verify.sh`, logs
`round46-out/coord/logs/close-*.log`):

1. **Cold local run:** `cargo clean` (CLEAN_EXIT=0, 8.0 GiB
   removed), then full workspace suite **1334 passed / 0 failed**
   (EXIT=0; every `test result` line ok, no FAILED line), then
   `cargo clippy --workspace --all-targets -- -D warnings` EXIT=0.
   Cold wall time: 21:57–22:36 UTC.
2. **Fuzz as claims, byte-identical:** bin sources at the recorded
   md5s (`verdict27.rs` a9cac6c399d304bac853f2a64f683fca,
   `comp27.rs` 4f85c605a841329dabed1d9a2aa84f88); verdict md5
   `797758443d101f9339fdd778e14af430` (standing baseline, exact);
   composition md5 `e8b2e73b117c7013d867419da8fb0ab7` (standing
   baseline, exact); all four bins exit 0 with 0 stderr bytes
   (0 panics); rights counters 5083 / 1024 / 5275,
   safety_flips=0. The summary's 40,000 / 497,024 figures are the
   bins' internal case counts, not stdout lines; the byte-identical
   claim is the md5 match, and it holds.
3. **Hosted CI on the final tip:** run 41056961522, conclusion
   success, head `db4209b` — macOS, Windows, Ubuntu all green
   (checked firsthand via `gh run view` and the Actions API).
4. **Desktop runner build on the final tip:** run 41056961555,
   conclusion success, head `db4209b` (checked firsthand).
5. **Windows bundle on the final tip:** run 41070168938, conclusion
   success, head `db4209b` — Gate (format, tests, clippy) green,
   bundle green, artifact `openbulletin-windows` (checked firsthand).

Booklet baseline exact on the close run: imposed 31,717 bytes,
reader 34,977 bytes (`booklet sizes after O2: imposed 31717 bytes,
reader 34977 bytes`, test exit 0). Cold-gate notes from the batch
records stand: R46-F's cold suite was accepted on the R46-F
precedent (two load-sensitive interleaved timing pins dispositioned
solo/quiet, bounds untouched); R46-G's on the same precedent (cold
suite stopped at the load-sensitive r45d pin, solo disposition
green, bound untouched). This close's own cold suite ran full-green
in one run, so no solo disposition was needed at the close.

### Cache-policy disposition at the confirmed completion

Disk at completion: 48% used / 79 GB free (Saturday 2026-10-31
~15:37 PDT, daytime window, threshold 94%). **Disposition: CARRY.**
No reading above the limit, no latch armed, no clean owed at this
completion. (The close's own cold verification already cleaned and
rebuilt; the target now stands rebuilt for the next round.)

### Disk-ritual note

`round46-out/` + `round45-out/` kept. `round44-out/` becomes
sweep-eligible at this boundary — eligibility only. No deletion
happens on the ritual's say-so alone; any sweep waits for Jacob's
approved exact list.

### Verification-leg reconciliation — recommendation

The round-46 brief's open question, answered explicitly, not by
silent reinterpretation: the desktop-compute brief's "do not build
on the Muse VM" rule (Jacob, 2026-10-29: VM full suites stopped;
evidence only via the envoy-runner wrapper or the GitHub dispatched
run) vs this loop's VM local verification leg. The workspace
AGENTS.md text of that rule is written for the Encore lane (it names
Encore wave lanes, envoy-runner v4, Wave 2.495), so it does not by
itself settle whether it binds this loop — but its rationale is
venue-based, not project-based (2 cores; the VM cannot hold a full
run), and it applies with equal force to this loop's suite, which
repeatedly shows load-sensitive timing behavior on this VM.

**Recommendation: from round 47, the legs of record at the close
are hosted CI ×3, the desktop runner build, and the Windows bundle
— the same three hosted legs this close already verifies — plus
the fuzz-as-claims and booklet checks, which are small targeted
checks (probe29v bins; a single test filter), not full suites. The
VM full suite is demoted to a provisional pre-close smoke run:
still run when cheap, never cited as a leg, never the basis for the
word "verified."** The alternative — keeping the VM leg as a leg of
record on the grounds that the brief was Encore-scoped — is not
recommended: it keeps citing the exact class of number the rule was
written to stop citing. This recommendation is for the parent seat
to adopt or reject; this close still reports the VM leg as run,
because it was run under the round-46 brief, and the hosted legs
stand on their own regardless.

### Carried to round 47

- **(1) The unsafe-direction r42e event** (above): named
  investigation target — deterministic witness/fix or written
  adjudication; never re-run-and-forget. Plus the deferred
  decision: immediate diagnostic now vs the round-47 target.
- **(2) HF-12 slice 2, second half: IDML.** `dump_from_idml` stays
  `Result<_, String>`; the IDML carry reason (including the
  TOO_DEEP branch) is in `r46g-record.md`. The from_json half is
  landed and is not reopened.
- **(3) R46-B coordinator note, left visible for the round-47
  audit to re-check (not an open finding):** trailing weekday-first
  governs silent because its clean twin reads the same
  (probe-verified, same as the R45-C glued pin).
- **(4) F9 timing-pin disposition stands:** the 27x bound, solo/quiet
  disposition on any future cold-run exceedance, never widening.
- **(5) The verification-leg reconciliation decision** (above):
  adopt or reject the demotion of the VM full suite to a smoke run.
- **(6) Round-45 carried item (e)** (run 40396061207, red on a
  superseded non-batch tip) was not re-litigated per the brief and
  is not an open defect.

Round 47 audits the tip `db4209b` (plus this close record, which
touches only the synthesis file).
