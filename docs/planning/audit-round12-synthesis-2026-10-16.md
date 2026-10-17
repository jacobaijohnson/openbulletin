# Round 12 audit synthesis (Wonder verify / review / red team over HEAD `421c1c9`)

Three blind lanes over the round-11 fixes. Verify: **PASS, 12 VERIFIED / 0
FAILED / 0 UNTESTABLE** (662/0 unpiped, clippy -D warnings exit 0, booklet
exact, Bach live; CI run 39038915426 confirmed green on all three OS jobs).
Review: **0 BLOCKER / 3 MAJOR / 5 MINOR**, four questions answered. Red team:
**0 BLOCKER / 4 MAJOR / 3 MINOR**. Deduped total: **0 BLOCKER, 6 MAJOR, 6
MINOR. The exit bar is NOT met** — the loop continues into round 13; the
Tier-2 final gate stays queued.

What held, stated first because it bounds the rest: **the rights safety axes
are now solid.** The red team ran ~1.69M fold/merge sequences (the 27-pool at
length 4, a widened family at lengths 5–6, the shared-credit pool at lengths
5–6 with zero divergences of any kind, a 12-archetype core carrying the
outside-pool shapes, plus 1,699,353 composed fold-prefix → single-merge
splits) and found **zero (children, permission-noted) flips anywhere**, zero
one-sided credit loss at every length, and save/reload landing on the same
triple (a fixed point). Per-occurrence serving held on every construction,
including triple segments, both-sides reservoir, and the Next Sunday
rollover. The review lane's census reproduced the R11-A addendum exactly and
held it at lengths 5–6 in 232k further sequences (Q1: the license/credit
residuals are honestly bounded; the credit class's MINOR standing is correct
— its real reach is the printed attribution words, see R12-9).

CI and bundle for `421c1c9`: ci.yml run 39045028113 SUCCESS on all three OS
jobs; bundle run 39045029389 SUCCESS.

One cross-lane observation: both the verify and review lanes hit the R10
shared-directory pin's *timing-budget* assertion failing solo under machine
load 8.4–13 (outcomes always passing; green once the machine quieted). The
budget is derived from an in-test control measured under the same contention,
so contention alone can fail it. It is a test-budget defect, not a product
one; R12-8 dispositions it.

## The MAJORs

### R12-1 (MAJOR, red team RT12-1 + review R12-3): the governor list misses ordinary words, and every miss reproduces the R11-4 harm

R11-C's governors cover not / ignore / never / no longer / don't-didn't use
and from / originally. The lanes probed the ordinary phrasings an office
actually writes and found the full harm — the disowned date arms a verdict,
blesses SameSunday, or sets keep-through — for: **skip, except, rather than,
old date:, previously, was, used to be, formerly, instead of, cancelled, bare
no, isn't** (and family). The review lane's Q3 verdict: a word list IS the
right mechanism (it is this machinery's universal pattern, and the
one-analysis architecture is correct); the list is simply incomplete in its
own two families.

Disposition (R12-A): **the governor families are completed with the attested
ordinary set.** Negation/cancellation: skip, except, rather than, instead of,
cancelled/canceled, no (standing as the group's governing word), isn't /
aren't / wasn't / weren't, never mind. Origin/past: previously, formerly,
used to be, was (as the past marker in "was November 1" shapes), old date.
Each attested phrasing is pinned on every consuming path (relative, absolute,
paren-only, Keep-through trailing, KT fallback). The standing risk of any
list — a novel phrasing — is accepted as the mechanism's known limit, as Q3
adjudged; the list now covers the attested ordinary set, and lanes keep
hunting.

### R12-2 (MAJOR, red team RT12-2; review R12-4 graded the scope half MINOR — the coordinator takes MAJOR on the red team's evidence that the asserted date appears in NO field)

A governor currently scopes over the whole rest of its group. "(ignore
November 1, use November 8)", "(not November 1, November 8)", and "(moved
from November 1 to November 8)" all silently lose the asserted Nov 8: not
contributed, not an alternative, not a second date — nowhere.

Disposition (R12-A): **a governor scopes to the one date it governs.** A date
asserted later in the same group — after a comma pivot, "use", or the "to"
of "moved from X to Y" — is read on its own terms: it contributes, or is
named as an alternative/second date, under the group's other rules exactly
as if the governed date were absent. "(moved from November 1 to November 8)"
reads as what it is: Nov 1 the origin, Nov 8 the assertion (the "moved to"
shape in compound form).

### R12-3 (MAJOR, red team RT12-3): plain candidates never dedupe — the multiplication survives for plain end records

2,900 PLAIN end records naming one 8,191-entry directory take 113.0 s to
refuse against a 228 ms control (300 candidates: ~10 s). Every prior storm
fixture used zip64-shared candidates, which dedupe. Plain candidates do not:
the DirKey's `dir_end` is each plain candidate's own end-record position, so
no two plain candidates ever share a key, and the enumeration re-walks the
directory per candidate.

Disposition (R12-B): **candidate identity is the directory described, not
the end record's position.** The dedupe key for both the enumeration and the
fallback is (directory start, directory size, zip64 record position where
present). Two plain end records at different positions declaring the same
directory are the same candidate for walking and opening. The plain-2,900
and plain-300 fixtures land in the control's time class; the zip64 families
keep their outcomes and classes.

### R12-4 (MAJOR, review R12-1): the DeclaredKey memo transfers resolutions between candidates — its stated mechanism is false

R11-B's addendum claims "identical declared fields resolve identically."
They do not: resolution depends on the candidate's position (`dir_end`), so
the memo can serve one candidate another's resolution. Proven by
construction: a crafted file loads with no decoy and with a 1-byte-varied
decoy, but is REFUSED with an identical-fields decoy standing before the
honest candidate (the memo hands the honest candidate the decoy's failed
resolution). The behavior predates R11-B, but the batch's stated mechanism
does not hold, and the refusal is real.

Disposition (R12-B): **the memo never serves a resolution across
candidates.** Either the memo key contains everything resolution depends on
(including the candidate's position — which makes cross-candidate service
impossible by construction), or the memo goes away in favor of the shared
resolution in R12-5. The crafted decoy file loads the honest bulletin.

### R12-5 (MAJOR, review R12-2): fallback resolution costs O(distinct declared descriptions × span)

With the search bound deleted (R11-B), each distinct declared description
pays a whole-span resolution search: 0.79 s / 2.47 s / 10.95 s for 1 / 2 / 4
MB distinct-key files, against 6.5–42 ms for identical-key files. Distinct
declared fields are free for a sender to vary, so the cost is
sender-controlled.

Disposition (R12-B): **resolution is computed once per load, not once per
candidate.** The fallback resolves its candidates the way the enumeration
does — one shared pass over the span computing every candidate's resolution
— so total resolution cost is O(span + candidates), independent of how the
declared fields vary. The distinct-key fixtures land in the identical-key
class (sub-second at 4 MB). Together R12-3/R12-4/R12-5 make one statement:
a load's directory work is bounded by the directories actually in the file,
each resolved and walked once, however the end records describe them.

### R12-6 (MAJOR, red team RT12-4): a plain line naming two dates reads as naming none

"October 25, 2026 and November 1, 2026" on a plain For Sunday line — and
even the same date written twice — reads NoSundayNamed, and the review's
first line says "These words do not say which Sunday they are for," which is
false: the words name two specific Sundays, and Accept stands unarmed.

Disposition (R12-A): **first-date primacy, the rest noted.** The framed
reader takes the first framed date as the line's date (the standing
first-date primacy of R11-11) and the remaining distinct dates join the
second-date note, exactly as the whole-line scan treats them elsewhere; the
same date written twice reads as that date, plainly. NoSundayNamed is
reserved for lines that name no date.

## The MINORs

### R12-7 (MINOR, review R12-5 + red team RT12-5): governors after the date, and dates inside nested groups

(a) "(November 1, cancelled)" contributes Nov 1 — a trailing cancellation in
the same group is never read. (Red team's held case "(November 1, not
October 25)" reads Nov 1 correctly: the trailing "not" governs the date
after it. The gap is a trailing governor with no second date to govern.)
(b) "((November 1))" standing alone reads NoSundayNamed: the balanced-paren
split makes the nested group one group, but its content scan never sees
through the inner parentheses.

Disposition (R12-A): (a) a trailing governor word in the same group clause
(cancelled, no longer, not) with no later date to govern disqualifies the
date it follows. (b) The group content reading sees through balanced inner
parentheses: the inner content is the group's content for date-reading
purposes.

### R12-8 (MINOR, cross-lane): the shared-directory timing pin's budget is load-derived and fails under contention alone

Both audit lanes hit it (load 8.4–13; outcomes always passing). A pin that
fails when the shared machine is busy teaches lanes to ignore red runs.

Disposition (R12-B, test-only): the pin's budget gains a fixed floor per
phase (the budget becomes max(control-multiple, control + a floor scaled to
the candidate count)), sized so contention alone cannot fail it while a true
multiplication (minutes) still does. The pin's outcome assertions are
unchanged.

### R12-9 (MINOR, red team RT12-6 + RT12-7, review R12-7): note accuracy and tolerance in three places

(a) A second "Sunday after" phrase's resolution is never noted, and a
restated base is noted as "another date" — the note machinery recognizes
only the first phrase and treats a base restatement as a distinct date.
(b) A lying zip64 total refuses an otherwise honest bulletin via the reader,
while the count walk counts the same file fine — the reader's strictness
and the walk's leniency diverge on `cd_size`/total lies.
(c) The R11-A addendum's wording outruns its census ("provably"; the
residual classes can change the printed attribution words, which the
addendum does not mention).

Disposition: (a) (R12-A) the Sunday-after phrase consumes its run wherever
it stands in the line, and a run equal to the standing base is a restatement,
never "another date". (b) (R12-B) the reader path tolerates a lying zip64
total the way the walk does; the honest bulletin loads. (c) (R12-C, docs)
addenda are dated records and are not rewritten; this synthesis records the
correction — the R11-A residuals are *measured*, not proven, and the credit
residual's reach includes the printed credit line — and FORMAT.md is checked
for the same overreach and corrected where found.

### R12-10 (MINOR, review R12-6): a gap after the directory closes the gate composition

A gap AFTER the directory (which the zip reader tolerates) is refused once
more than 64 candidates close the small-count branch of the reader gate.
Another composed dead zone in the enumeration × fallback × reader chain,
smaller than R11-3's.

Disposition (R12-B): the lane's construction loads. The reader attempt's
conditions are revisited as part of the R12-5 shared-resolution work so the
chain has no composed dead zone of this family: whenever a directory the
app's own walk counts as walkable stands in the file and nothing has
yielded, the reader attempt is reachable.

### R12-11 (MINOR, review R12-8): a dropped duplicate pairing is accepted silently

R11-D's first-pairing-wins is adjudged right (Q4: the value is stored once
per name, so the alternative winners are undefined or recreate the desync).
But when the second pairing was the intended one, the office sees a silently
accepted save — names under the first target, a blank row on the intended
one, and no note of any kind.

Disposition (R12-C): **the save speaks.** When a migration does not follow a
duplicate pairing (or a removal of an already-paired name), the status names
it in plain words in the app's voice — which name, which pairing was kept,
that the other was not followed — so the office can correct it by hand. The
accounting itself is unchanged.

## Fix batches

- **R12-A (dates):** R12-1 (governor families completed) + R12-2 (governor
  scopes to one date; later asserted dates read) + R12-6 (two framed dates:
  primacy + notes) + R12-7 (trailing governors; nested-group reading) +
  R12-9(a) (Sunday-after consumption and restatement in the notes).
- **R12-B (loader):** R12-3 (identity = the directory described) + R12-4
  (no cross-candidate memo service) + R12-5 (one shared resolution pass) +
  R12-10 (gate composition) + R12-9(b) (zip64-total tolerance) + R12-8
  (timing-pin floor, test-only).
- **R12-C (small print):** R12-11 (the save speaks on an unfollowed duplicate
  pairing) + R12-9(c) (FORMAT.md precision check).

Each batch appends its dated Shipped addendum here with real numbers. Round
13 audits the fixes; the exit bar is unchanged.

## Shipped addendum, R12-A

Shipped 2026-10-17. One file changed:
`crates/ob-extract/src/contribution.rs`. No `ob-app` sentence
changed — with the line's date read again (R12-6), the review's
first line follows the standing verdict sentences, and the
NoSundayNamed arm's own guard already speaks the read date
when one stands. Tests: **662 → 667 passing** (workspace,
unpiped, exit 0), clippy `--workspace --all-targets -D warnings`
exit 0, `cargo fmt --all` applied. Five new pins, all `r12a_*`
in the contribution tests; one flagged pre-existing pin change
(below).

What shipped, per finding:

- **R12-1.** The governor families are completed behind the
  one group analysis: negation/cancellation adds skip, except,
  rather than, instead of, cancelled/canceled, bare no,
  isn't/aren't/wasn't/weren't (with unpunctuated twins), and
  never mind beside the R11 set; origin/past adds previously,
  formerly, used to be, was, and old date. The full set is
  pinned on the relative path, and every phrasing is also
  pinned paren-only, on the Keep-through trailing path, on
  the Keep-through fallback, and in absolute composition.
  Two review-probed phrasings stay outside the disposition's
  set and outside the families — "(won't be November 1)" and
  "(wrong: November 1)" still contribute — the word list's
  known limit, recorded here so round 13 does not rediscover
  it as a regression.
- **R12-2.** Qualification is per date occurrence (the
  positional scan no longer dedupes by value), and a
  governor's window is the tokens after the previous
  occurrence's end and after the last clause break before
  the date: a governor is spent on the one date in its own
  clause. "(ignore November 1, use November 8)",
  "(not November 1, November 8)", and "(moved from
  November 1 to November 8)" all read November 8 as the
  group's date on the relative, paren-only, and
  Keep-through paths; "(not November 1, November 1)" reads
  November 1 by its ungoverned second standing. The same
  clause rule frees the dates a governor never reached:
  "(notes from the pastor, November 1)" now names
  November 1 as garnish instead of dropping it in silence,
  and the common-word care is pinned — "(choir was
  singing, November 1)", "(no rehearsal, November 1)",
  and "(it was moved, November 1)" all still contribute
  November 1, while "(ignore it: November 1)" still
  governs. "(moved to …)" and "(correction: …)" are
  untouched and still contribute.
- **R12-6.** A plain line naming two or more framed dates
  and reading in no other way now reads by first-date
  primacy (`plain_multi_date_reading` in `date_line`, on
  the bare line and on the de-grouped core): "October 25,
  2026 and November 1, 2026" (and the "/" and mid-group
  "(Room 2)" forms) reads October 25 with November 1 on
  the second-date note, armed against a November 1
  bulletin; the same date written twice reads as that
  date plainly, nothing noted. A line naming a single
  passing date ("October 25, 2026 please") keeps its
  standing no-read, and NoSundayNamed is again reserved
  for lines naming no date.
- **R12-7.** (a) A trailing governor disqualifies the
  date it follows: cancelled/canceled anywhere in the
  words after a date (up to the next date) cancels it,
  and not / no longer after the group's last date —
  with no later date to govern — falls back on the date
  before it. "(November 1, cancelled)" contributes
  nothing on every path; the held "(November 1, not
  October 25)" still reads November 1, the trailing
  "not" governing the date after it; "(November 1,
  cancelled, November 8)" reads November 8. (b)
  `analyze_group` unwraps inner parentheses before
  reading, so "((November 1))" reads its date on the
  paren-only, relative, and Keep-through paths, and a
  nested governor ("(note (not November 1))") still
  governs through them.
- **R12-9(a).** `second_dates_in_line` collects
  Sunday-after phrase runs wherever they stand: a
  phrase run's value is its resolution and its base is
  consumed with it. "Sunday after October 18, 2026 and
  Sunday after October 25, 2026" reads October 25 and
  now notes November 1 (the second phrase's
  resolution); a plain run equal to the standing base
  is a restatement and is never noted — the
  October 20 restatement witness notes nothing, and
  the same phrase twice notes nothing. The round-11
  held controls (two plain runs after a phrase; a base
  standing after a plainly read date) stand unchanged.

Flagged pre-existing pin change (one):
`r11c_non_contributing_groups_name_alternatives_and_never_poison`,
its nested-group block. It expected "next Sunday (note
(November 1))" to resolve as SundayInWords with the
nested date invisible — the R12-7(b) defect itself
(RT12-5: the flat "(note: November 1)" control has always
contributed). The block now expects the through-parentheses
reading: November 1 remembered as the group's date,
DifferentSunday(November 1) against the October 25
bulletin, and the same date recorded as garnish mismatch
on the absolute line; a FLAGGED comment in the test body
states the reason. No other pre-existing test changed.

## Shipped addendum, R12-B

Shipped 2026-10-17. One code file changed:
`crates/ob-format/src/lib.rs`, plus the loading-rules
bullet in `docs/FORMAT.md` (updated to the identity /
resolution account below; no rule's substance changed
beyond what the findings state). Tests: **667 → 672
passing** (workspace, unpiped, exit 0; final run's log
read back complete: 28 result lines, 672 / 0), clippy
`--workspace --all-targets -D warnings` exit 0,
`cargo fmt --all` applied. Five new pins, all `r12b_*`
in the ob-format tests (46 → 51 in that crate); one
flagged pre-existing pin change (below). All timings
are from the debug test build on this shared machine
and are load-sensitive, as the standing measurement
note says; machine load is noted beside each set.

What shipped, per finding:

- **R12-3.** `DirKey`'s second component is no longer
  the candidate's own end offset. Candidate identity
  is now the directory described — (resolved
  directory start, declared directory size — the
  zip64 record's size where the candidate resolves
  through one, else the plain record's size field —
  zip64 record position where present, declared
  total only when no start resolved) — in the
  enumeration and in the fallback alike, so two
  plain end records at different positions
  declaring one directory share a key. The
  enumeration's walks are shared one step further:
  one record-chain walk per distinct directory
  START (replacing the per-key `count_directory_records`
  walk), out to the farthest end any candidate
  naming that start describes, each candidate's
  count read off the one chain. Measured
  (standalone pin run, load ≈1.4–3.0): plain-300
  refuses in **641 ms** (bulletin door) / **236 ms**
  (sheet door) against controls of 305 ms / 385 ms;
  plain-2,900 in **893 ms** / **636 ms** — the
  control's time class at both counts and both
  doors, against the pre-fix 113.0 s at 2,900 the
  red team measured. The zip64-shared, declared-
  varied, signature-storm, and body-signature
  families keep their outcomes and classes (the
  full regression suite below).
- **R12-4 + R12-5 (one mechanism, as the
  dispositions anticipated).** The `DeclaredKey`
  memo is **deleted** — the type, the key function,
  and both memo tables — and with it the question
  of what a memo may serve: nothing is served
  across candidates, by construction. In its place
  the fallback resolves every candidate in the
  file **once, in one shared pass**
  (`candidate_resolutions_in`): each candidate's
  zip64 and directory-start searches become
  queries answered by one scan per signature over
  the span all windows cover, and the record
  chains are walked once per distinct directory
  start, every candidate reading its resolution —
  computed from its **own** position — off the
  shared results. Total resolution cost is
  O(span + candidates), independent of declared-
  field variation. The memo-witness construction
  (honest bulletin + 70,000 bytes of junk carrying
  100 fakes) now loads the honest bulletin in all
  three arrangements — no decoy, one-byte-varied
  decoy, identical-fields decoy — at both doors.
  The distinct-key cost fixtures land in the
  identical-key class (same standalone run):
  1 MB identical **99 ms** / distinct **136 ms**;
  2 MB **436 ms** / **430 ms**; 4 MB **827 ms** /
  **501 ms** (pre-fix, the review lane's release
  build: 6.5–42 ms identical against 0.79 s /
  2.47 s / 10.95 s distinct).
- **R12-10.** A resolution now also records
  whether the candidate **reads** the way the zip
  reader reads: its records chaining exactly
  across the declared directory size (the reader's
  size-bounded reading tolerates bytes standing
  after the directory). The reader gate is
  revisited on the shared resolutions: the attempt
  runs when the chosen candidate walks or reads,
  under the standing small-count clause, or when
  an untried directory that walks or reads stands
  anywhere in the file; the fallback likewise
  opens a candidate that walks **or reads** (its
  directory is the reader's own first choice in
  its trimmed stream, so the opening costs one
  read, not a retry storm — the storm pins hold:
  the 2,900-comment-signature sheet-door pin and
  every signature family pass unchanged). The
  lane's construction — an honest file with a
  16-byte gap after its directory, plus 100 fake
  end records — loads at both doors.
- **R12-9(b).** Where a candidate resolves through
  a zip64 record and its directory's exact walked
  count is known, the loaders' opens present that
  count to the reader in place of the record's two
  entry-total fields — patched only in the trimmed
  stream (`TrimmedReader`) or the trimmed copy,
  never in the file — so a lying zip64 total is
  tolerated the way the count walk tolerates it.
  An honest total equals the walked count, so the
  patch is a no-op for honest files. The pinned
  shape (totals lying up at 999,999 and down at 1
  against two real records; offsets and size
  honest) loads at both doors, and the walk still
  counts its two records.
- **R12-8 (test-only, FLAGGED).**
  `r10_candidates_sharing_one_directory_pay_for_it_once`'s
  two budget computations (the candidate loop and
  the bad-document phase) now use
  `shared_directory_budget`: `max(control × 20,
  control + 15 ms × candidates)` — the fixed floor
  scaled to the candidate count the disposition
  names (the 2,900 budget becomes control + 43.5 s,
  clear of the 23–30 s contention failures both
  round-12 lanes recorded, while a true
  multiplication — ≈39 ms of walk per candidate,
  100 s and more at these counts — still fails).
  The pin's outcome assertions are unchanged; no
  other pre-existing test changed. (The round-11
  fallback-dedupe pin's similar budget is a
  different pin and was not touched.)

Regression, as pinned by the suite: all round-9/10/11
fixture families pass unchanged — signature storms
in comments and bodies, zip64-shared directories,
declared-varied candidates, fake-shadow and
dead-zone tables at both doors, the lying-size and
lying-count cap families — and first-error
precedence is unchanged (the all-junk and dead-zone
refusal pins hold verbatim). Two internal helpers
moved with the mechanism, no test outcome attached:
`candidate_resolution_in` and `fallback_walkable`
are now `#[cfg(test)]` (their only callers are the
direct walk pins; the loaders consult the shared
pass), and `count_directory_records` is replaced by
the shared chain walk (`walk_chain_file` /
`walk_chain_bytes` + `chain_count`) its logic moved
into.

## Shipped addendum, R12-C

Shipped 2026-10-17. Files changed:
`crates/ob-model/src/lib.rs` (the migration detail),
`crates/ob-app/src/main.rs` (the status sentence and
its wiring in the serving-lines save), and
`docs/FORMAT.md` (below). Tests: **672 → 674
passing** (workspace, unpiped, exit 0; the log read
back complete after the run settled: 28 result
lines, 674 / 0), clippy `--workspace --all-targets
-D warnings` exit 0, `cargo fmt --all` applied. Two
new pins, both `r12c_*`; **no pre-existing test
changed** — the R11-D pins pass with their original
expectations, the accounting untouched. Run note,
stated exactly: one solo rerun of the full suite
stopped at the openbulletin binary with ETXTBSY
("Resource temporarily unavailable" launching the
test binary) after 502 passed / 0 failed, exit 101;
the solo rerun of that binary adjudicates it —
173 passed / 0 failed, exit 0 — a transient launch
failure, not a test outcome.

What shipped, per finding:

- **R12-11.** The migration detail no longer
  discards the unfollowed events.
  `ServingLineMigration` gains `unfollowed:
  Vec<ServingLineUnfollowed>` — each event naming
  the old name, the new name in the pairing that
  was kept, and the new name in the pairing that
  was not followed (`None` when the event not
  followed was a removal of the already-paired
  name). The spent-name tracking in
  `migrate_serving_line_renames_detailed` now
  records which pairing each name was spent on, so
  both unfollowed shapes are reported in the order
  the pairing computed them (identical repeats are
  noted once). The accounting is the R11-D
  accounting, unchanged: first pairing in segment
  order wins; participants and aliases behave
  exactly as the R11-D pins expect. In `ob-app`,
  the serving-lines save speaks from
  `migration.unfollowed` through the same notes
  channel as `migration.removed`, in a new sentence
  beside the other migration sentences
  (`unfollowed_serving_lines_sentence`). The exact
  words shipped, for the review's double-pairing
  shape (Xray paired to Yankee, then to Zulu):
  "'Xray' was paired with 'Yankee' and with
  'Zulu': the names went to 'Yankee', and the
  pairing with 'Zulu' was not followed. Correct
  the names by hand if they belong under 'Zulu'."
  For an unfollowed removal: "'Xray' was paired
  with 'Yankee' and was also removed: the names
  went to 'Yankee', and the removal was not
  followed. Correct the names by hand if they do
  not belong under 'Yankee'." A migration with no
  duplicates carries an empty `unfollowed` and the
  status says nothing. Pins:
  `r12c_unfollowed_pairings_and_removals_are_carried_on_the_migration`
  (ob-model: both events carried, accounting as
  pinned by R11-D, a no-duplicate rename-plus-
  removal carrying no note) and
  `r12c_unfollowed_serving_pairings_are_named_in_the_status_words`
  (ob-app: both sentences word for word, and no
  note on a clean migration).
- **R12-9(c).** FORMAT.md's composition and
  loading sections were read end to end against
  the code as shipped at `7639c51`. Two sentences
  were corrected, one was added, and the rest was
  confirmed (below). Corrected, quoted: (1) the
  guards bullet's "The total is summed from the
  zip directory before anything is decompressed"
  was false at this HEAD — the loaders count
  actual bytes as each entry is read
  (`read_entry_capped` / `read_bulletin_archive` /
  `read_insert_archive` in `ob-format`) and never
  sum declared sizes; the bullet now says so.
  (2) The composition section's "the fold route
  and a chain of single merges agree on both,
  always" overstated a census result the way
  R12-7 flagged in the R11-A addendum; it now
  reads that the two routes agreed on both across
  the pools and lengths the round-11 and round-12
  censuses ran, and that "that agreement is
  measured, not proved". Added, in the serving-
  lines account: the per-occurrence rule FORMAT.md
  had never stated — a name standing in more than
  one segment is spent once (R11-D), a later
  pairing or a removal of an already-paired name
  is not followed, and the save's status names the
  name, the kept pairing, and the event not
  followed (R12-11, above). Confirmed against the
  code, unchanged: the composition rules (the
  fence, license-and-credit traveling together,
  the credit-pairing precedence, the final
  judgment and its fallback precedence, standing-
  wins for license and credit) match
  `merge_rights_records` / `fold_duplicate_asset_records`
  as shipped, including "no merge manufactures a
  permission", which the per-merge fence
  reversions and the final judgment carry by
  construction; the loading section's identity,
  counting, and resolution claims (entry count by
  walking records, never declared totals; candidate
  identity as the directory described; one shared
  resolution pass, never carried across candidates;
  the zip64-total tolerance; latest-first trials
  and the reader-attempt conditions) match
  `ob-format` at this HEAD, as R12-B left them.

Flagged pre-existing pin changes: **none.** No
pre-existing test was modified; the two `r12c_*`
pins are new.

## Coordinator close (2026-10-17)

All three batches shipped and were independently re-verified by the
coordinator (full unpiped suite + clippy -D warnings at each step): R12-A
`490ecb4` (667 tests; governor families completed and clause-scoped; one
flagged pin change — the pin had asserted the nested-group invisibility),
R12-B `7639c51` (672 tests; plain-2,900 113 s → 893 ms, distinct-key 4 MB
10.95 s → 501 ms, the DeclaredKey memo deleted; one flagged change by
disposition, the timing pin's floor), R12-C `4cf20e4` (674 tests; the save
speaks on unfollowed pairings; two FORMAT.md sentences corrected; zero
changed pins). One ETXTBSY variant fired during R12-C (a transient
test-binary launch failure); adjudicated by solo rerun (173/0) and stated
exactly in the addendum. Round 13 audits these fixes, and its lanes are
briefed to re-probe the two governor phrasings R12-A deliberately left at
the disposition's edge ("won't be November 1", "wrong: November 1") — if
they reproduce the harm, the families gain them by disposition there. The
Tier-2 final gate runs only after a round returns zero BLOCKER + zero
MAJOR.
