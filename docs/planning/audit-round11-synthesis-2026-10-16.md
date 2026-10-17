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

## Shipped addendum, R11-A (2026-10-17)

All four findings landed in the shared judgment and the fold's end, in
`crates/ob-model/src/lib.rs`; no merge plumbing changed, and a single
merge did not gain end-state judgment. FORMAT.md carries the corrected
invariance wording (R11-12). Local totals: before 651 passed / 0
failed; after **653 passed / 0 failed**, full unpiped
`cargo test --workspace` exit 0; `cargo clippy --workspace
--all-targets -- -D warnings` exit 0; `cargo fmt --all -- --check`
exit 0. The two new tests are the pins below; **no pre-existing test
changed** — the R8-9 pin and the R9-A / R10-2 pins pass with their
original expectations, verified by name in the run
(`r8_fold_judges_the_composed_rule_once_at_the_end`,
`r10_fold_and_single_merge_routes_agree_over_the_pool`, the r9
witnesses). The R10 pin's pool and assertions were *extended* (below),
with every prior assertion kept.

**R11-1 — the permission axis is route-invariant.** One new step in
`judged_composed_license`, the judgment both routes share: when the
composed record notes children and its credit line is empty, a
standing permission-noting license that is itself vouched stands;
otherwise a vouched permission-noting license anywhere in the
judgment's sources (`permission_fallback_license`: ChurchOwned /
PublicDomain / Cc0, children-carried first, then the fallback
precedence) stands in preference to a non-noting license, however
well vouched. It runs at every pairwise judgment, so a single-merge
chain carries the axis in its composed record; the fold reads it off
the full source set at fold end. Both named witnesses now end at
(children, Cc0, "") on both routes. The R10 brute-force pin gains
`rights_permission_noted` as a permanently tracked field: `check_seq`
asserts (children, permission-noted) agreement on *every* sequence it
runs, zero tolerance, alongside the prior assertions (the credit-paired
rule on both routes; on the folded record: unvouched license 0,
manufactured credit/license 0, orphan assertion 0 — all still hold).

**R11-5 — the fold keeps a vouched credit, and its license.** The
blind spot is closed in the pool first: the pin pool gains three
archetypes (indices 24–26) — children-bearing records carrying
fallback-class licenses *with* credits (Cc0 / PublicDomain /
ChurchOwned beside children, each with a credit line) — plus a widened
family (indices 1, 2, 5, 8, 10, 14, 24, 25) run at lengths 2–4. The
code fix is at the fold's end (`judge_folded_composed_record`): when
the folded record notes children and its credit came out empty, a
credit vouched under the fence's rule with the fold's arrival timing
— its own record notes the children, or a children-bearing
permission-noting record stands at or before it in fold order
(`vouched_fold_credit`) — stands in the folded record before the
license is judged, and the credit-pairing step then stands the
license that traveled with it. A credit vouched only by a bearer
arriving *after* it is not restored: both routes drop that credit at
its arrival, and that arrival-timing class is unchanged. The witness
now ends (children, CcBy, "Photo by B") on both routes, pinned by
name. Within a pairwise judgment no fallback can orphan a vouched
credit — the fence has already reverted any credit the pair does not
vouch — so the orphaning was fold-level only, and the restoration is
the repair.

**R11-9 — shared-credit pairing is deterministic.** 
`credit_paired_attribution_license` no longer takes the first pairing
in source order: among the attribution licenses traveling with the
standing credit, the winner is chosen by precedence
(`attribution_rank`: CC BY before CC BY-SA, then the version string),
including when the standing license is itself paired but outranked.
The fold chooses among the pairings its merges actually saw —
tracked per merge in `fold_duplicate_asset_records`: a pairing counts
when its credit stands at its merge (the incoming record, and the
standing record at the first merge where it is still a source), when
it is prefix-vouched on arrival even if the deferred fence reverts
that credit pair-locally, and it stops counting when a later merge
reverts the credit it travels with. A restored credit's pairings are
its prefix-vouched carriers'. The review lane's witness and five
further named shapes (mirror order, reverted pairing, bearer before
both pairings, dropped pairing, both pairings vouched by a later
fallback record) are pinned to the tie-break on both routes in
`r11_shared_credit_pairings_follow_the_precedence_on_both_routes`,
and that test runs the lane-style shared-credit pool (9 archetypes:
both attribution licenses carrying "Photo by X") at lengths 2–4 with
full triple agreement asserted on every sequence.

**Census, before → after** (scratch harness over the pools above,
fold vs single-merge, divergences by outcome field):

- Widened pool, 27 archetypes, lengths 2–4, 551,853 sequences:
  license 34,676 → 33,844; credit 3,360 → **504**; permission-noted
  2,034 → **0**; children 0 → 0. (The same harness over the original
  24-archetype pool reproduces the review lane's baseline exactly —
  16,553 triple divergences, 2,034 permission flips — before the fix,
  and 13,702 license / 288 credit / 0 permission after.)
- Shared-credit pool, 9 archetypes, lengths 2–4, 7,371 sequences:
  license 280 → **0**; credit 9 → **0**; permission 60 → **0**; pure
  paired ambiguity (same standing credit both routes, two paired
  attribution licenses, different winners) 211 → **0**. The review
  lane's differently-shaped pool counted 132 such divergences; on
  this pool the class is zero, named shapes included.
- Permutation check (widened pool, lengths 2–3, 4,032 multisets):
  children-bearing outcomes whose permission answer differs across
  any permutation or route: **0**.

**Residual classes, named with bounds.** (1) License-only divergence
33,844 of 551,853 widened sequences (6.1%): the round-10 adjudicated
deferral / fence-timing class — every outcome vouched against its
composition (unvouched 0 on both routes), nothing manufactured, and
now provably disjoint from the safety axes. (2) Credit divergence
504, all at length 4 (lengths 2–3 are 0), and all
*both-present-but-different*: each route stands a vouched credit, a
different one (which credit fills first under each route's fence
timing), one-sided credit loss is 0 in both directions at every
length, and since a standing credit notes a permission, this class
cannot touch the safety axes either. No other divergence class
remains on either pool.

**R11-12 — FORMAT.md.** The composition account now states exactly:
the children note and whether a permission stands beside it are
route- and order-invariant (with the permission rule stated — a
vouched permission-noting license stands in preference to a
non-noting one when children stand and the credit is empty); the
license follows standing-wins and the fallback precedence, with the
credit-pairing rule as a stated exception; the credit follows the
pairing rule, now including the R11-9 precedence tie-break among
shared-credit pairings and the R11-5 fold-end restoration. The
R10-2 credit-paired sentence and the R10-11 standing-wins paragraph
were revised to match the shipped code.


## Shipped addendum, R11-B (2026-10-17)

Both findings landed in `crates/ob-format/src/lib.rs`, at both
doors (`load` / `load_insert`) identically. Local totals: before
653 passed / 0 failed; after **656 passed / 0 failed**, full
unpiped `cargo test --workspace` exit 0; `cargo clippy
--workspace --all-targets -- -D warnings` exit 0; `cargo fmt
--all -- --check` exit 0. Three new tests (below); **no
pre-existing test changed** — the round-9/round-10 fixture
families (comment signature storms 500/1500/2900 in both fake
shapes, shared-directory 30/300/2900, fake-shadow, the
first-error precedence pins) pass with their original
expectations, verified in the full run.

**R11-2 — the fallback dedupes by the resolved directory.**
One resolution function, `candidate_resolution_in`, now serves
the fallback at both doors: it resolves a candidate exactly as
the count walk's enumeration resolves it and returns the
enumeration's own identity (`DirKey`: directory start, end,
zip64 record position) plus walkability. The fallback loop
dedupes on that identity — a directory tried in the first pass
(the enumeration's keys are carried into the fallback) or
earlier in the fallback is never opened again under another
candidate's name — and never on declared fields. `DeclaredKey`
survives only as a memo key: identical declared fields resolve
identically, so a resolution is computed once per declared
description (the 2,900-signature bodies stay cheap); it decides
what a resolution costs, never what counts as a repeat.

**R11-3 — the dead zone closes.** (a) `FALLBACK_SEARCH_BOUND`
is gone: both of the resolution's searches (the zip64 record,
the first central header) run over the whole span the candidate
leaves, matching the enumeration's unbounded resolution, so
`fallback_walkable(honest)` is true on the gapped file exactly
as on the ungapped one — pinned directly in the new test.
(b) The reader gate no longer asks whether anything opened, or
whether the reader's chosen candidate was already tried: the
attempt runs whenever no candidate has yielded a document and
the chosen candidate's directory walks (the small-count
damaged-words branch is unchanged). Its cost is the one
walkable directory the walk check just resolved.

**Fixture pins.** `r11_the_fallback_dedupes_by_the_resolved_directory_not_the_declared_fields`:
the RT11-1 shapes (shared 8,191-entry directory, 300
candidates, per-candidate `total16` variation and the
`cd_size32` twin) refuse with the control's no-bulletin
sentence at both doors, inside the r10 pin's budget shape
(control × 20 + 10 s). `r11_a_shifted_honest_directory_loads_behind_walkable_fakes`:
every row of the RT11-2 construction table at both doors —
honest alone; stub with end record in window; no-gap + junk +
65 fakes; gap + junk with no fakes / 65 fakes / 3 fakes / 1
junk-document fake; no-gap + junk + junk-document fake — loads
the honest bulletin/sheet. `r11_a_file_with_no_bulletin_anywhere_refuses_with_the_first_error`:
in the dead-zone shapes with no bulletin anywhere, the refusal
is the junk-document fake's JSON error ("key must be a
string…") where a document error exists, else the no-bulletin
sentence.

**Measured timings** (temporary probe in the same style as the
lane's, run once and removed; this machine, load average ≈3.3
during the probe — loader timings here are load-sensitive):
one-candidate control over the 8,191-entry directory **131
ms**; declared-varied 300-candidate files — `total16`: load
**354 ms**, load_insert **363 ms**; `cd_size32`: load **375
ms**, load_insert **400 ms** — ≈2.7–3× the one-candidate
control, the control's time class, against the red team's
pre-fix 14,517 / 13,821 ms vs a 318 ms control (≈45×) on their
machine. Regression classes hold: the 2,900 timing-signature
nodoc file refuses in **45 ms**; the dead-zone gap + junk +
65-fake file loads in **11 ms**.

**FORMAT.md.** The loading-rules bullet now states that the
fallback dedupes by the same resolved identity (never the
declared fields), that a directory is resolved the same way
in both enumerations however far prepended bytes shifted it,
and that the reader attempt is gated on its chosen directory
walking rather than on whether an earlier candidate opened.

## Shipped addendum, R11-C (2026-10-17)

All four findings landed in `crates/ob-extract/src/contribution.rs`;
no sentence in `crates/ob-app/src/main.rs` changed — the
alternatives ride the existing second-date note machinery, whose
fields the review simulation already copies. Local totals: before
656 passed / 0 failed; after **660 passed / 0 failed**, full
unpiped `cargo test --workspace` exit 0; `cargo clippy --workspace
--all-targets -- -D warnings` exit 0; `cargo fmt --all -- --check`
exit 0. Four new tests (below); **one pre-existing test changed**,
flagged in its body and below.

**R11-4 — governed dates do not contribute.** One group analysis
(`analyze_group`) now stands behind `group_date`, so every consumer
reads the same rule: the dates a wordy group contains are read with
their positions, and a date is disqualified when a governor stands
before it inside the group — negation (not, ignore, never, no
longer, don't/didn't use) or origin (from, originally; "moved
from" is the "from" case) — on every consuming path (relative and
absolute composition, paren-only lines, Keep-through trailing
groups, and the Keep-through fallback, which reads the same
trailing-group walk). A disqualified group behaves as a no-date
group. Held shapes pinned unchanged: "(moved to November 1)" arms
as an assertion, "(correction: November 1)" contributes, and a
group restating the line's own date records no mismatch.

**R11-6 — non-contributing groups never poison; alternatives are
named.** Three changes compose. (a) Trailing groups are split by
balancing parentheses, so a nested group ("(note (November 1))")
is one trailing group rather than a defect. (b) `date_line`'s core
re-read no longer waits for a group to have contributed: a group
that contributes nothing behaves as if absent and the line's own
reading proceeds on its core, on absolute lines, relative phrases,
and paren-only lines alike; `mid_date_group_reading` reads its
de-grouped candidate on its core too, so a mid-date group plus a
trailing group together read the plain date with both groups
compared as garnish (review R11-7's witness). (c) A group
containing two or more qualified dates — counting an "or
<partial date>" tail ("November 1 or the 8th", the partial date
taking the full date's month and year) as an offered alternative —
contributes none of them as a single date; its dates are recorded
as named alternatives on the line's second-date note
(`for_sunday_second_dates` / `keep_through_second_dates`), the
existing unarmed note machinery, on relative lines as on absolute
ones: "next Sunday (November 1 or November 8)" resolves
SundayInWords(October 25) with both alternatives named in the
account and no group feeding the verdict, instead of the previous
silent clean acceptance or the promoted single date.

**R11-7 — the anchor is the read date's own occurrence.**
`second_dates_in_line` collects the line's date runs first, then
anchors only the first run that IS the read date, plus — on a
Sunday-after line — the one run the "Sunday after" phrase itself
consumes (the phrase standing in or directly before that run),
which is never flagged as its own second date. Sunday-proximity
no longer anchors: "October 20, then Sunday, October 25, 2026"
notes October 20, the red team's stray-date shape notes the stray
(October 22) rather than the base (October 20), and any date in
the six days before the read date is a second date.

**R11-8 — the weekday law inside groups.** The same group
analysis disqualifies a contained date when a weekday word
standing directly before it inside the group contradicts it:
"(Monday November 1)" and "(correction: Monday, November 1)"
contribute nothing (November 1, 2026 is a Sunday); an agreeing
weekday ("(correction: Sunday, November 1)") still contributes.

**Tests.** Four new pins, all in `contribution.rs`:
`r11c_negated_and_origin_groups_contribute_nothing` (every RT11-5
governor, on relative lines, paren-only lines, Keep-through
trailing groups and the fallback, plus the choir-anthem origin
witness and the held moved-to / correction / restatement shapes),
`r11c_non_contributing_groups_name_alternatives_and_never_poison`
(the RT11-6 witnesses: absolute and relative multi-date groups,
nested groups, "or the 8th", and the mid-date-plus-trailing
composition), `r11c_the_second_date_anchor_is_the_read_dates_own_occurrence`
(the October 20 witnesses on both lines, the Sunday-after base
never self-flagged, the stray-date inversion), and
`r11c_a_contradicting_weekday_inside_a_wordy_group_contributes_nothing`
(both weekday witnesses plus the agreeing-weekday control).

**Flagged pre-existing test change (one).**
`r10b_a_wordy_trailing_group_contributes_its_date`: its two tail
blocks asserted `for_date = None` / `NoSundayNamed` for "October
25, 2026 (Room 2)" and "October 25, 2026 (November 1 and November
8)" — the pins asserted the R11-6 defect itself (a
non-contributing group poisoning the plain reading). Both blocks
now expect the plain date to read and stand (SameSunday), the
no-date group recording nothing, and the two-date group's dates
named as alternatives on the second-date note; the reason is
stated in a FLAGGED comment in the test body. No other
pre-existing test changed: the R9/R10 stacked-group, verdict-
contract, restatement, and second-date families pass with their
original expectations in the full run.

## Shipped addendum, R11-D (2026-10-17)

The finding landed in `crates/ob-model/src/lib.rs`, in
`serving_pairing`'s ambiguous-kinds accounting and in
`migrate_serving_line_renames_detailed`; no sentence in
`crates/ob-app/src/main.rs` changed. Local totals: before
660 passed / 0 failed; after **662 passed / 0 failed**,
full unpiped `cargo test --workspace` exit 0; `cargo clippy
--workspace --all-targets -- -D warnings` exit 0; `cargo
fmt --all -- --check` exit 0. Two new tests (below); **no
pre-existing test changed** — the R9-D / R10-8 witness
families (`r9_kinds_come_from_each_segments_own_outcome`,
`r10_kinds_claim_only_what_the_pairing_establishes`,
`ambiguous_kinds_count_only_segments_the_pairing_could_not_resolve`,
the canonical crossed renames and both trailing mirrors,
surplus-past-need removals) pass with their original
expectations in the full run.

**R11-10 — the accounting is per occurrence.** Two halves.
(a) The reservoir: the removal-only and addition-only
segment tallies now keep their names instead of bare
counts, and reservoir absorption spends only unspent
occurrences — an occurrence whose name a pairing already
spent (on the old side for the removal reservoir, on the
new side for the addition reservoir) is not the
reservoir's to absorb, because the name's stored names
sit under one slug and its alias under one key, so the
name can be spent only once. In the RT11-8 witness (Papa
paired Papa→Quebec in one segment, removal-only in
another, Uniform's segment holding the surplus) the kinds
now read {renamed, removed, added, moved: false}: the
second Papa's removal stands, the unabsorbed surplus
reads as an addition, and no phantom move is claimed. The
deficit-side mirror (duplicate new names) reads
identically. With unique names no removal-only or
addition-only name is ever a paired name, so the
certain-shape families are untouched, and the refusal to
auto-apply in ambiguous shapes is unchanged. (b) The
migration: each old name is spent once, in segment
order. The first pairing of a name is the pairing
actually made — it moves the stored names and records
the alias; a later pairing of the same old name is not
followed (it would move nothing, the source being spent,
yet overwrite the recorded alias), and a removal of a
name a pairing already spent is not followed either (it
would clear the alias and names the pairing just made).
The recorded aliases correspond one-to-one with the
pairings actually made: in the pair-plus-removal shape
(Xray paired to Yankee in one segment, removed in
another) the people land on Yankee, the alias Xray→Yankee
survives, and nothing is reported removed; in the
double-pairing shape (Xray paired to Yankee and to Zulu)
the people land on Yankee and the surviving alias points
at Yankee, not Zulu. Duplicates on the new side (two
distinct old names paired to one duplicated new name)
follow both pairings, record both aliases, and keep the
standing target-conflict rule — the first value lands,
the second stays where it was.

**Tests.** Two new pins, both in `crates/ob-model`:
`r11d_duplicate_names_spend_each_occurrence_once_in_the_kinds`
(the RT11-8 ambiguous witness and its deficit-side
mirror, both now {renamed, removed, added} with no moved,
plus the unique-name control — the same geometry with a
unique removal-only name still reads as the canonical
crossed renames {renamed, moved}), and
`r11d_duplicate_pairings_keep_participants_and_aliases_in_step`
(the pair-plus-removal shape, the double-pairing shape,
and the duplicate-new-name shape, asserting participants
and aliases together in each).
