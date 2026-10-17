# Round 11 audit synthesis (Wonder verify / review / red team over HEAD `caac9a8`)

Three blind lanes over the round-10 fixes. Verify: **PASS, 11 VERIFIED / 0
FAILED / 0 UNTESTABLE** (651/0 unpiped, clippy exit 0, booklet exact, Bach
live). Review: **0 BLOCKER / 1 MAJOR / 6 MINOR**, four questions answered. Red
team: **0 BLOCKER / 4 MAJOR / 4 MINOR**. Deduped total: **0 BLOCKER, 5 MAJOR,
8 MINOR. The exit bar is NOT met** — the loop continues into round 12; the
Tier-2 final gate stays queued.

**Correction to the round-10 coordinator close, recorded plainly.** The close
accepted the R10-2 route residual on the builder's census, which classified
flips by the (children, license, credit) triple. The review lane's census over
the same pool (its 16,553 divergence total exactly matches) also tracked
`permission_noted` and found it flipping in 2,034 sequences: the residual was
never confined to license/credit cosmetics. The close's acceptance is
superseded on that point (finding R11-1 below). The census facts the close got
right stand: children diverge 0, manufactured values 0 on either route, every
standing license vouched against the full source set, credit divergence
one-directional.

Also recorded: the Windows bundle run 39034721020 from `caac9a8` SUCCEEDED
(the round-10 bundle exists). The CI workflow (`ci.yml`, windows/macos/ubuntu
on every push) has failed on every recent push for two platform defects in
test code — batch R11-0 below, handed to the loop by the coordinator seat's CI
diagnosis and verified against the tree by the coordinator (three
`env!("HOME")` sites, not one; the diagnosis named the first).

One measurement note: the verify lane's shared-2,900 timings under this
machine's current contention (load average ~11–13 from the unrelated shared
job) ran 17–26 s against the addendum's ~4 s on a quieter machine. The
disposition's bar (seconds, not the pre-fix 9–10 minute projection) held in
every run; absolute loader timings in this project are load-sensitive and
future lanes should record load beside them.

## The MAJORs

### R11-1 (MAJOR, review): the route residual flips the children's-photo hold-back itself

`permission_noted` — a credit line, or a license of ChurchOwned / PublicDomain
/ Cc0 / LicensedReprint standing beside the children note — decides whether a
children's picture rides the web page or is held back. Over the lane pool,
**2,034 sequences flip it by route, in both directions**. Minimal witness:
[LicensedReprint, {children}, {children, Cc0}] folds to not-noted (held back)
but single-merges to noted (rides the page). The mechanism: the fold judges
the whole composition at fold end, single merges judge pairwise, and a
permission-noting claim wins on one route and loses on the other. This is the
round-5 BLOCKER's class — a hold-back silently switched off by an accident of
how records arrived — one composition layer down.

Disposition (R11-A): **(children, permission-noted) is route-invariant,
decided by one rule in the shared judgment over the full source set.** Single
merges do NOT gain full end-state judgment (the review lane's reasoning:
provenance cost, and the R8-9 pin's witness is permission-invariant, so the
pin is not the blocker). The permission axis joins the children value as a
safety axis: for every sequence in the pool, fold and single-merge agree on
both, with zero tolerance; the census gains permission-noted as a tracked
field permanently. Where the routes' pairwise histories would differ, the
shared judgment's answer over the whole composition governs both routes, as
the R8-9 deferral principle already does for the fold.

### R11-2 (MAJOR, red team RT11-1): one varying declared field resurrects the multiplication through the fallback

Enumeration dedupe holds (300 shared candidates → 1 returned). The FALLBACK
dedupes by `DeclaredKey` — keys built from declared, sender-controlled
fields. Varying one free declared field (`total16` or `cd_size32`) per
candidate makes each key distinct, and the shared 8,191-entry directory is
reopened ~300 times: load 14,517 ms / 13,821 ms against a 318 ms control
(~45×, linear in candidate count). The R10-1 multiplication, one door down:
the dedupe keyed on what the file SAYS rather than what it RESOLVES to.

Disposition (R11-B): **the fallback dedupes by the resolved directory** — the
same identity the enumeration uses (directory start, end, zip64 position) —
never by declared fields alone. Candidates whose declared fields differ but
which resolve to the same directory open it once. The shared-directory
fixtures with per-candidate declared-field variation land in the control's
time class.

### R11-3 (MAJOR, red team RT11-2): a shifted honest directory is refused whenever one walkable fake opens

Three bounds compose into a dead zone: (1) enumeration covers only the tail
window (65,633 bytes); (2) `fallback_walkable` resolves a directory by
searching at most 65,633 bytes past the declared offset, so an honest
directory shifted further by a stub entry at the file's head is "unwalkable"
to the fallback though the enumeration's own (unbounded) resolution and the
zip reader both find it; (3) the reader attempt is gated off once any
walkable fake candidate stands latest and opens. Construction: stub entry
(70,000 bytes) + honest bulletin (directory 70,034 bytes past its declared
offset) + 70,000 bytes trailing junk + fakes. The honest bulletin loads alone,
loads via enumeration when its end record is in window, and loads via the
reader's path alone with no fakes — but is REFUSED with 65 fakes, with just 3
opening fakes, and behind 1 junk-document fake (whose JSON error becomes the
refusal). This reopens the R10-3 shadow class for shifted directories — a
shape the format otherwise goes out of its way to support.

Disposition (R11-B): **the dead zone closes.** (a) The fallback's directory
resolution uses the same resolution as the enumeration (a directory shifted
past the old search bound is walkable to the fallback too). (b) A candidate
that opens but yields no bulletin never closes the reader attempt: the
attempt runs whenever no candidate has yielded a document, its cost bounded
by the walkable/resolved checks that already bound the storm fixtures (the
2,900-signature shapes stay in their measured class). Every row of the lane's
construction table loads the honest bulletin; a file with no bulletin
anywhere still refuses with the most informative error (first candidate's
document error where one exists, else the no-bulletin sentence).

### R11-4 (MAJOR, red team RT11-5): negated and "from" dates in wordy groups are read as assertions

`group_date` (R10-5) contributes the one date a wordy group contains, with no
regard for the words governing it. "next Sunday (not November 1)" against the
Oct 25 bulletin arms `DifferentSunday(November 1)` — on the date the sender
negated; against a Nov 1 bulletin the same line reads SameSunday while the
review sentence states "the date written in parentheses" about a parenthesis
that says the date is NOT November 1. "(ignore November 1)" and "(moved from
November 1)" behave identically; a paren-only "(not November 1)" line makes
the negated date the contribution's operative date; a Keep-through line
trailing "(not November 1)" or "(ignore November 1)" silently sets
keep-through to the negated date. Held correctly on the same surface:
"(moved to November 1)" arms as the sender's assertion; "(correction:
November 1)" contributes as R10-5 intends; a group restating the line's own
date records no mismatch. The review lane's R11-6 witness ("choir anthem
(from November 1 service)" setting keep-through in the fallback) is this same
family through the origin word "from".

Disposition (R11-C): **a contained date governed by a negation or an origin
marker does not contribute.** The plain-words governors — negation (not,
ignore, never, no longer, don't/didn't use) and origin (from, moved from,
originally) standing before the date inside the group — disqualify that
date's contribution on every path that consumes group contributions
(relative and absolute composition, paren-only lines, Keep-through trailing
groups and the Keep-through fallback). A disqualified group behaves as a
no-date group: it contributes nothing and (per R11-6 below) does not poison
the line. "(moved to …)" and "(correction: …)" keep contributing, as the lane
held them.

### R11-5 (MAJOR, red team RT11-7): the fold route silently drops a credit line the merge route keeps

[children + CcBy, children + Cc0, CcBy + "Photo by B"] folds to
`(children, Cc0, "")` — the composition's only credit line silently dropped
on the load route — while single merges end `(children, CcBy, "Photo by
B")`. 372 credit divergences in the lane's 48,636-sequence probe. The fold's
children-carried fallback preference picks Cc0 (whose record carries no
credit) without accounting for the credit the CcBy record carries. The lane
also names the structural blind spot: the R10 route-agreement pin's "paired
family" archetype indices exclude every kids record with a fallback-class
license, so the pin cannot see this shape.

Disposition (R11-A): **credit selection follows the credit-pairing rule on
both routes, and the fallback accounts for credits.** When a vouched credit
stands in the composition, the composed record keeps a credit: the license
that traveled with the standing credit stands with it (R10-2's rule) unless
the credit itself is displaced by the standing rules, and a fallback license
that would orphan the composition's only vouched credit does not win on that
account alone. The witness agrees on both routes with the credit standing.
The brute-force pool and the route-agreement pin gain kids records carrying
fallback-class licenses (Cc0 / PublicDomain beside children, with and without
credits), closing the named blind spot; credit divergences over the widened
pool are reported in the addendum and any residual class is named with its
bounds, held to the R11-1 bar for the safety axes regardless.

## The MINORs

### R11-6 (MINOR, red team RT11-6 + review R11-7): groups that contribute nothing poison the line, and alternatives are laundered

"October 25, 2026 (November 1 or November 8)" reads NoSundayNamed — the
multi-date group contributes nothing, and because the core re-read only runs
when a group contributed, the plain Oct 25 is never recovered. Nested
parentheses ("(note (November 1))") land in the same branch and kill the
plain reading on both lines. "next Sunday (November 1 or November 8)" reads
SundayInWords(Oct 25) UNARMED — two named alternatives laundered into a clean
acceptance. "(November 1 or the 8th)" promotes Nov 1 alone to a certainty
(the partial second date does not parse, so the group "contains exactly one
date"). Review R11-7 is the sibling shape: a mid-date group plus a trailing
group together read NoSundayNamed.

Disposition (R11-C): **a group that contributes nothing never takes the
line's reading with it, and offered alternatives are named, not laundered.**
The line's own reading proceeds as if a non-contributing group were absent
(the plain date stands; the relative phrase resolves); a group containing two
or more dates — counting an "or <partial date>" tail as an offered
alternative — records its dates as named alternatives in the account (the
existing note machinery, unarmed), on relative lines as on absolute ones.

### R11-7 (MINOR, review R11-3 + red team RT11-3): the second-date anchor is chosen by Sunday-proximity, not identity

"October 20, then Sunday, October 25, 2026" notes nothing: Oct 20 is treated
as the read date's anchor because its Sunday is the read date, and vanishes.
The red team generalizes: any date in the six days before the read date
anchors and vanishes on plain lines too, and on Sunday-after lines the base
itself can be flagged as the "other date".

Disposition (R11-C): **the anchor is the read date's own occurrence.** A
distinct date is a second date even when it shares the read date's Sunday;
the resolution's base is never its own second date. The note fires for Oct 20
in the witness, and no line flags its own base.

### R11-8 (MINOR, review R11-4 + red team RT11-6 tail): wordy groups skip the weekday check

"(Monday November 1)" governs a paren-only line silently and "(correction:
Monday, November 1)" contributes Nov 1 and arms on it, although November 1,
2026 is a Sunday — the inside-group scan never applies the weekday law the
line readers and bare groups obey (a wrong weekday yields no date).

Disposition (R11-C): **a contained date whose group names a contradicting
weekday does not contribute** — the same wrong-weekday-yields-no-date rule,
applied inside groups.

### R11-9 (MINOR, review R11-2): shared-credit pairing is order-chosen

Two attribution licenses carrying the SAME credit text: the fold picks the
first pairing, single merges can pick another (132 divergences in a
shared-credit pool the lane pool cannot see). Both pairings are vouched; the
choice between them is arrival order, undisclosed.

Disposition (R11-A): **the pairing tie-break is deterministic and
route-invariant**: when one standing credit pairs with two attribution
licenses in the composition, the license is chosen by the standing precedence
order (the fallback precedence), identically on both routes. The 132
divergences go to zero; the shared-credit shapes join the pin pool.

### R11-10 (MINOR, red team RT11-8): duplicate serving names desync the pairing accounting

With the same name standing in two segments, the reservoir absorbs a name
occurrence the pairing itself already spent: kinds read {renamed, moved}
with a real removal hidden; a double pairing desyncs participants from
aliases.

Disposition (R11-D): **the accounting is per occurrence, not per name.** A
name occurrence spent by a pairing cannot also feed the reservoir; reservoir
absorption consumes unspent occurrences only; participants and aliases stay
in step under duplicates. The unique-name shapes read exactly as today.

### R11-11 (MINOR, red team RT11-4): framed-date primacy demotes the service date to a note — accepted, documented

"Monday, October 19, 2026 minutes; service Sunday, October 25, 2026" frames
Oct 19 (the first framing candidate) and arms DifferentSunday(Oct 19), with
Oct 25 present only as the second-date note. The framed reader's
first-date primacy predates round 10; the line is genuinely ambiguous; the
whole-line scan (R10-4) guarantees the service date is VISIBLE in the account,
which is what the scan is for. No shape was found where primacy blesses a
wrong date silently — the note prevents silence.

Disposition: **accepted as a documented limitation; no code change in round
11.** First-date primacy stays the framed reader's rule. This paragraph is
the documentation; if a later lane shows a sender shape where primacy misleads
WITHOUT the note surfacing the service date, it reopens as a finding.

### R11-12 (MINOR, review R11-5, docs): two documents state the invariance too broadly

FORMAT.md's "whether a permission stands beside it never depends on order"
is falsified by R11-1's witnesses, and the R10-C addendum's confinement
sentence overstates what the census measured.

Disposition (R11-A, docs part): after the R11-1/R11-5 fixes land, FORMAT.md
states exactly which axes are route- and order-invariant (the children note
and whether a permission stands beside it) and which follow the stated rules
(license by standing-wins and credit-pairing; credit by the pairing rule).
The round-10 addendum is not rewritten (it is a dated record); this synthesis
is the correcting document, and the R11-A addendum states the final wording.

## Questions answered by the review lane (for the record)

Q2 (storm gate): refuses/slows no honest file shape — the gate governs only
the reader's whole-file attempt; R11-3's dead zone is a crafted
double-anomaly, graded by the red team on reachability grounds. Q3 (verdict
contract): at the right level — every `week_verdict` arm and the sole
mismatch constructor provably avoid the self-contradiction (the red team's
468-case fuzz concurs: 0 violations). Q4 (harness retry): truly test-only,
bound right (a broken helper still fails after exactly 6 attempts). R10-8
reservoir sweep: no over-crediting in the unique-name shapes. All flagged pin
changes across R10-A/B/C verified honest; no unflagged pin moved.

## Fix batches

- **R11-0 (CI platform fixes; from the coordinator seat's diagnosis):** the
  three `env!("HOME")` compile-time lookups in test code
  (`crates/ob-extract/src/html_export.rs`,
  `crates/ob-music/src/parse.rs`, `crates/ob-music/src/render.rs`) become
  runtime lookups (HOME, falling back to USERPROFILE), skip-when-absent
  behavior preserved; the detached-grandchild variant of
  `a_pipe_holding_grandchild_cannot_hold_the_caller`
  (`crates/ob-extract/src/runner.rs`) detaches portably on stock macOS (no
  `setsid` command there) with the property and both timing bounds
  unweakened. Acceptance is CI itself: the batch is done when its push's
  `ci.yml` run is green on all three OS jobs.
- **R11-A (rights routes):** R11-1 (permission axis route-invariant) + R11-5
  (credit-pairing on both routes; pool gains kids + fallback-class
  archetypes) + R11-9 (shared-credit deterministic pairing) + R11-12
  (FORMAT.md exact invariance wording).
- **R11-B (loader):** R11-2 (fallback dedupes by resolved directory) + R11-3
  (dead zone closes: shared resolution bound; yieldless openers never close
  the reader attempt).
- **R11-C (dates):** R11-4 (negation/origin guard on group contributions) +
  R11-6 (non-contributing groups never poison; alternatives named) + R11-7
  (anchor by identity) + R11-8 (weekday check inside groups). R11-11 is
  documentation-only, above.
- **R11-D (serving):** R11-10 (per-occurrence pairing/reservoir accounting).

Each batch appends its dated Shipped addendum here with real numbers. Round
12 audits the fixes; the exit bar is unchanged.

## Shipped addendum, R11-0 (2026-10-17)

Both named defects fixed, plus one defect of the same class that the
first fix unmasked; CI acceptance met on the final run below.

**Defect 1 — `env!("HOME")` (commits `18c445e`).** The three
spike-fixture test paths (`crates/ob-extract/src/html_export.rs`,
`crates/ob-music/src/parse.rs`, `crates/ob-music/src/render.rs`) now
resolve the home directory at runtime — `std::env::var_os("HOME")`,
falling back to `USERPROFILE` — and join the fixture path onto it.
Skip-when-absent is preserved exactly: with neither variable set the
path is empty and the fixture read fails, so the test skips as it did
for an absent fixture (verified locally both ways: fixtures present,
the Bach counts run and pass; HOME and USERPROFILE unset, the test
prints its skip line and passes).

**Defect 2 — detached grandchild (commit `18c445e`).** The detached
shape of `a_pipe_holding_grandchild_cannot_hold_the_caller`
(`crates/ob-extract/src/runner.rs`) now detaches through the system
perl's `POSIX::setsid` — `perl -e 'use POSIX qw(setsid); setsid() or
die …; exec "sleep", "7" …' & exec true` — present on stock macOS and
on Linux, where stock macOS ships no `setsid` command. The holder
lands in its own session, keeps the inherited stdout pipe for the
full sleep, and the parent shell exits at once, as the `setsid` shape
did. Both shapes stay under `#[cfg(unix)]`; the bounds (elapsed ≥ 5 s
and < 7 s) and the `TimedOut` fault are unchanged. Local solo run:
both shapes pass, 11.02 s total (≈5.5 s each: 0.5 s deadline + 5 s
grace).

**Unmasked follow-up (commit `379feff`), recorded as a scope note.**
With the lib-test target compiling on Windows for the first time,
the Windows job reached clippy and failed there (run 39037987472):
in `crates/ob-extract/src/helper.rs` the test module's `use
super::*;`, `use std::time::{Duration, Instant};`, and `const
GOOD_DUMP` were dead on Windows — every test and helper they serve
is a shell-script fake already gated `#[cfg(unix)]`, so the module
is empty off Unix — and `-D warnings` made the three lints errors.
The two imports and the const now carry the same `#[cfg(unix)]` gate
as everything they serve; on Unix nothing changes. This was a third
platform defect in test code, hidden behind defect 1; the batch's
acceptance (all three OS jobs green) could not be met without it.

**Local totals.** Before: 651 passed / 0 failed (the round-11
baseline). After `18c445e`: full unpiped `cargo test --workspace`
651 passed / 0 failed, exit 0; `cargo clippy --workspace
--all-targets -- -D warnings` exit 0; `cargo fmt --all -- --check`
exit 0. After `379feff`: the same three gates re-run, same numbers
(651 / 0, clippy 0, fmt 0).

**CI outcome.** Run 39037987472 (push of `18c445e`): macOS ✓,
ubuntu ✓, Windows — Engine tests ✓ (defect 1 gone), Clippy ✗ (the
unmasked helper.rs lints above), run failed. Run **39038915426**
(push of `379feff`): **all three OS jobs green** — ubuntu-latest
10m42s, windows-latest 12m44s (every step, including Clippy and
Build desktop app), macos-latest 8m44s. The R11-0 acceptance bar is
met.

## Shipped addendum, R11-A

_(appended by the R11-A builder with real numbers and flagged pin changes)_

## Shipped addendum, R11-B

_(appended by the R11-B builder with real numbers and flagged pin changes)_

## Shipped addendum, R11-C

_(appended by the R11-C builder with real numbers and flagged pin changes)_

## Shipped addendum, R11-D

_(appended by the R11-D builder with real numbers and flagged pin changes)_
