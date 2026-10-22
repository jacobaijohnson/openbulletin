# Audit round 36 — synthesis (coordinator, 2026-10-21)

Audited tree: `c99a605` (round-35 close record on code
tree `4a3c9c1`). Three blind lanes, read-only:
verify (`round36-verify.md`), review
(`round36-review.md`), red team
(`round36-redteam.md`), all in
`~/workspace/openbulletin-audit-wave3/`, scratch in
`round36-out/`.

Lane verdicts: verify PASS on all six claims
(0/1/2 handed on), review 0/8/2, red team 0/4/1.
Deduped: **0 BLOCKER / 8 MAJOR / 4 MINOR** — the
exit bar is NOT met. Majors trend:
10 → 6 → 6 → 1 → 4 → 3 → 2 → 5 → 6 → 8.

The verify lane's standing claims all held on the
audited tree: suite 1028/0 (run 2; run 1's single
failure is recorded below as a load flake, not
hidden), clippy 0 on a genuinely re-checked tree,
booklet 31,717 / 34,977 byte-exact, fuzz bins at
their recorded md5s with stdouts byte-identical to
the standing baselines (verdict `7bcbfdc0…`, comp
`96b0c7b2…`), rights counters 5083 / 1024 / 5275
with safety_flips=0 (M8 held, sixteenth consecutive
round), and every round-35 pin the spot audit named
exists in-tree and passes. Every finding below
comes from the carried observations and the lanes'
own hunting, not from a failed standing claim.

## Adjudications recorded before the findings

1. **The canary's date fields (territory re-check).**
   The review lane's R36-3 witness reported the
   brace canary form keeping For = Nov 15 where the
   paren twin adopts Nov 8. The coordinator
   re-checked the exact lines through
   `parse_contribution` (probe
   `round36-out/coord/probe36check/`, run.log):
   all date fields are identical across the brace,
   paren, and bare forms (for = 2026-11-08,
   second = 2026-11-15, both routes, canary /
   agreeing / whole-line forms). The review lane's
   date-divergence arm is NOT borne out and is
   dropped from F3's statement; the red team's and
   verify lane's readings stand. What survives —
   agreed by all three lanes — is the phantom
   record itself. F3 is graded on that basis.
2. **F3's grade.** Review MAJOR, red team MAJOR,
   verify MINOR (record-only effect, dates
   identical — verified true). Adjudicated MAJOR:
   the record is the product's Check surface; a
   false "check the date with the sender" on a line
   whose dates fully agree, firing for two of three
   wrapper pairs on both routes, is a wrong outcome
   presented to the user, and the both-braced form
   additionally diverges in the record's *content*
   (names the base where its twin names the
   resolution). Verify's dissent and its (correct)
   date-field evidence are recorded here.
3. **F1 / F10 split (target (a)).** Verify split
   the month–day family by content: contents
   carrying a judgment (weekday words, governing
   words) escape it when glued — MAJOR; inert
   contents (`notes`, `choir`) merely flip
   date-vs-no-date — MINOR. Review and red team
   graded the whole family MAJOR with the governing
   words as headline. Adjudicated as verify's
   split: severity follows the escaped judgment.
   Both land in the same batch (R36-C); the
   adjudicated reading for the whole slot is the
   standing one — spacing is not semantics; the
   glued form takes the spaced form's reading.
4. **F6 merge.** Review R36-6 (appended central
   directory; derivation ignores EOCD disk fields,
   reader falls back to the real directory) and red
   team RT36-5 (fake directory planted inside the
   real EOCD's own comment; derivation and reader
   both take the planted one) are one finding: a
   second self-consistent EOCD candidate steers the
   doors' judgment away from the archive's real
   directory. Two witnessed vectors, one candidate-
   discipline defect in `zip_directory`, one batch.
5. **RT36-2 / RT36-3 "proposed" majors** are
   adjudicated IN as MAJORs (they form F1 with
   R36-1): the readings they need are the standing
   adjudicated ones (R35-C's principle), not new
   doctrine.

## Findings

### F1 (MAJOR) — Glued groups carrying a judgment escape it at the month–day and leading slots; mixed-pair wrappers escape at both

Review R36-1 + red team RT36-2, RT36-3 + verify
(a) MAJOR arm. The R35-C re-spacing gate
(`respaced_glued_weekday_groups`, composition.rs
:1809) re-spaces only a standalone group holding
one weekday word in openers of one pair, and skips
a group standing immediately inside another
opener. Past that pattern, glued and spaced split:

- Month–day slot, governing contents:
  `November(cancelled) 1, 2026` serves Nov 1 live
  with no record; spaced and bare twins read
  nothing. Same for `(postponed)`, and the
  multi-word governor `November[called off] 1,
  2026`. A glued date group `November(November 8,
  2026) 1, 2026` reads Nov 1 with a record naming
  Nov 8 where the spaced twin reads Nov 8 clean.
- Month–day slot, mixed wrappers:
  `November([Monday]) 1, 2026` and its three mixed
  siblings serve Nov 1 silently; every spaced twin
  reads nothing.
- Leading slot, mixed wrappers:
  `([Monday])November 1, 2026` serves Nov 1 with
  no record; the spaced twin refuses with
  `WeekdayMismatch(Monday@2026-11-01)`, both
  routes.
- Keep-through mirrors every For cell.

The comment at composition.rs:1793-1796 claiming
both wrapper forms "have always read" alike is
falsified by these witnesses and must be corrected
with the fix. Disposition: R36-C.

### F2 (MAJOR) — A wrapped weekday leading the base of a whole-line Sunday-after phrase collapses the phrase to its base date

Review R36-2. `sunday_after_base` (dates.rs:468)
judges the base's leading weekday through
`weekday_word` (dates.rs:263), which never unwraps
a bracket pair; R35-H's `bracketed_weekday_word`
reached the phrase-scan sites, not this reader.
`Sunday after (Monday) November 1, 2026` (any
wrapper, doubled and mixed included) reads
for = Nov 1 with the resolution demoted to a
second date and **no record**; the bare twin
refuses with the mismatch recorded. The agreeing
form `Sunday after (Sunday) November 1, 2026`
collapses the same way (Nov 1, not Nov 8). The
behind-"on" path judges correctly in all forms;
the hole is the no-"on" whole-line path.
Disposition: R36-B.

### F3 (MAJOR) — Carried target (b), the R35-H canary: a bracketed/braced shape word inside a group records a phantom mismatch naming the phrase's base

Review R36-3 + red team RT36-1 + verify (b), as
adjudicated above. Inside a group the Sunday-after
phrase is assembled from `group_token`-cleaned
tokens; a wholly bracketed token cleans to nothing
(dates.rs:1266), so the phrase never forms in the
group analysis, its base stands as a plain signal,
and `mismatch_from_groups` records it against the
resolution. The group-signals gate is the proximate
site (red team): `dates_inside_group_signals`
consults the phrase run only when the *cleaned*
token already equals `"sunday"` (~composition.rs
:3113), so R35-H's raw-token fix is unreachable
there. Witnesses (coordinator re-check included):
`November 15, 2026 ({Sunday} after November 1,
2026)` and the agreeing `November 8, 2026
({Sunday} after November 1, 2026)` both record
`ParenthesizedDateMismatch { named: 2026-11-01,
read: 2026-11-08 }` with its "check the date with
the sender" sentence, on both routes, under every
outer wrapper; paren and bare twins record none;
all date fields are twin-identical. The
both-braced form names the base where its
paren-shape twin names the resolution.
Disposition: R36-B (the gate asks the raw token
for its bracketed word, as the scan now does).

### F4 (MAJOR) — Leading-group checks beside relative phrases are paren-only

Review R36-4. Three sites (composition.rs:1100
`sunday_after_relative_phrase`, :1251
`relative_sunday_phrase`, :1151
`leading_group_disagrees`, whose loop is
`while rest.starts_with('(')`) enter the R21-1
machinery only for a literal `(`. A bracket/brace
leading group is instead adopted as the line's
date by the generic reading and the phrase is
forgotten: `[November 8, 2026] next Sunday` reads
for = Nov 8 (a group disagreeing with the phrase
silently wins), where the paren form surfaces the
group and remembers the phrase in words. A
restating paren group beside a Sunday-after
phrase silences that line entirely. Keep-through
mirrors. Disposition: R36-D.

### F5 (MAJOR) — A governing word wearing `! ? ' "` inside a bracket/brace group stops governing

Review R36-5. `analyze_group` unwraps parens only
(composition.rs:45; same pattern :846);
`is_bracketed_token`'s trailing trim (dates.rs
:1098) and `clean_group_word` (:1317) both lack
`! ? ' "`. `(November 8, 2026 [cancelled]!)`
adopts Nov 8 — also `{cancelled}!`, and `?`, `'`,
`"` in both pairs — while the paren-wrapped and
bare forms govern (for = none). Marks `;` and `:`
govern in all pairs; the split is exactly the four
missing marks. Disposition: R36-E.

### F6 (MAJOR) — A second self-consistent central directory steers every zip door's judgment (R34-B family, two vectors)

Review R36-6 + red team RT36-5, merged per
adjudication 4. The derivation
(`zip_directory.rs`: `end_record_at` :118 takes
the last EOCD whose declared comment fits;
`arithmetic_directory` :213, `hunted_directory`
:425, `validated_zip64_record` :287) never
consults the EOCD disk fields (or their zip64
counterparts) and takes a candidate standing
inside another EOCD's declared comment at face
value.

- Vector 1 (appended directory): a second
  well-formed directory + EOCD appended after the
  real one. The scan judges the appended directory
  (no duplicates, no refusal); with dirty disk
  fields the reader rejects it and serves the real
  directory — including the duplicate copy the
  scan never saw (music door served SECONDCOPY);
  with clean disk fields the appended directory
  silently re-points which copy is served
  (FIRSTCOPY). Witnessed at the music and IDML
  doors; all four doors share the derivation and
  reader.
- Vector 2 (EOCD comment): `[locals][real
  CD][real EOCD with a 402-byte comment]`, the
  comment being a complete second directory +
  EOCD. Both the derivation (cd_start = the
  planted directory, 7697 vs real 7295) and the
  reader take the planted one. IDML serves the
  first story, .mxl loads the first score, fonts
  land only the Regular face (the Bold never
  lands), clips land the red PNG twice — while
  the plain-duplicate controls are refused / land
  both copies correctly at all four doors.

Disposition: R36-A. Fix shape (from the lanes,
adopted): candidate discipline in one place —
a candidate found inside another candidate's
declared comment span never outranks the candidate
that declares the span; the earliest fully-valid
candidate whose walk and comment close over the
file exactly is preferred; the disk fields join
the facts the derivation validates, so the scan
and the reader judge the same directory or the
package refuses.

### F7 (MAJOR) — `raw_payload`'s unchecked offset arithmetic panics in all four doors

Review R36-7. Four identical private copies
(idml.rs:449, ob-fonts lib.rs:263, ob-clips
lib.rs:290, ob-music parse.rs:221) compute
`local + 30` and `local + 30 + name_len +
extra_len` with plain `+` on a usize cast of the
spot's zip64-resolved local offset, unbounded
against the file; only `data_end` uses
`checked_add`. Witness (zpain3, exit 101): a
compound archive in F6's appended-directory shape
whose planted record resolves a local offset of
u64::MAX-10 panics at idml.rs:452, "attempt to
add with overflow" (a release build wraps and the
slice panics instead). The fonts and clips doors
call `raw_payload` for every spot, no duplicate
needed. Disposition: R36-A — checked arithmetic
in all four copies (single-source the helper if
the shape allows), refusal in the standing words,
never a panic.

### F8 (MAJOR) — Stray-mark recovery is paren-only

Review R36-8. The R35-F recovery reads
`(November 1, 2026` and `)November 1, 2026` as
their bare twins; the bracket/brace twins read
nothing: `[November 1, 2026`, `{November 1,
2026`, `]November 1, 2026` all drop the sender's
date with no record. Disposition: R36-F.

### F9 (MINOR) — A trailing garnish in a mixed-pair wrapper is judged in neither form

Red team RT36-4. `November 1, 2026 ([Monday])`
reads clean, glued and spaced, all four mixed
pairs, both routes; single-pair and doubled
same-pair trailing twins are judged (refused with
the record). No glued/spaced divergence — the
check is simply absent for mixed pairs.
Disposition: R36-C with the wrapper family.

### F10 (MINOR) — A glued inert non-weekday group at the month–day slot reads a date its spaced twin refuses

Verify (a) MINOR arm (cells also witnessed inside
review R36-1 / red team RT36-2). `November(notes)
1, 2026` reads Nov 1; `November (notes) 1, 2026`
reads nothing; identically for `[]` and `{}` and
for `choir` and the other inert words tried. No
judgment is escaped and all pairs behave alike;
the split is spacing-as-semantics outside R35-C's
weekday-scoped ruling. Adjudicated reading (with
F1): the glued form takes the spaced reading.
Disposition: R36-C.

### F11 (MINOR) — `[next Sunday` is remembered in words verbatim; the paren twin is not remembered

Review R36-9. The stray bracket is repeated back
as part of the sender's phrase on the in-words
memory surface; `(next Sunday` is not remembered
at all. No date is adopted or refused on it.
Disposition: R36-F.

### F12 (MINOR) — The Sunday-after shape word wearing `!`, `?`, or quotes reads nothing; wearing `:` or `;` it reads

Review R36-10. `sunday_after_outcome`'s shape
trim (dates.rs:440-441) and `weekday_word`'s
trailing trim (:263) between them judge two
sentence marks and not the other three:
`Sunday! after November 1, 2026` is silent (no
date, no record), as are `Sunday?` and `"Sunday"`;
`Sunday:` / `Sunday;` read Nov 8. No wrong date
is adopted; the line goes silent. Disposition:
R36-F.

## Records (not findings)

- **(c) The F6/F9 recycled-pid residual**: its
  stated boundary re-verified against the code by
  the review lane (peer_provably_gone, ob-app
  main.rs:4001, still rests on PeerLiveness::Dead
  alone; no identity test has crept back) and
  independently by the red team's combined-path
  sweep (676 cases, both paths populated, 0
  deviations from the AND of the isolated
  verdicts). Stands recorded-not-fixed; the wedge
  remains the safe direction.
- **The r32c ratio pin's load limit** (verify
  observation): run 1 of the verify lane's suite
  failed `r32c_one_big_group_of_distinct_dates_
  parses_without_the_cliff` at 14.0× against its
  10× bound under cross-lane load ≈ 9; the pin
  passed on an exact single-test retry (load 8.8)
  and in the full run 2. Recorded as a witnessed
  load flake of the audit environment, NOT a code
  regression and NOT grounds to widen the bound
  (the bound is the gate R33 built; the close's
  serial cold runs are its standing context).
  Carried to round 37 as a scheduling note: heavy
  lane runs serialize where practical.
- **Red team's held lines**, recorded as
  evidence: R35-C's pure-pair gate (108 systematic
  cases, zero diffs); R35-E's timing fix on every
  edge attacked (mixed / alternating / mismatched
  / long-single-group all linear; 64,000 unclosed
  bracket groups 2.58 s against the pre-fix
  57.7 s, release frame); R35-H on its pinned
  surfaces; mixed wrappers around dates (84 cases
  twin-equal); the lock mirror plus the new
  676-case sweep; rights quadruples (114,244
  outcomes, 0 violations); 28,004 generated cases,
  0 panics, outside F7's crafted compound.

## Batch plan

One writer at a time; every batch under the
batch-boundary discipline (commit + push +
HEAD == origin confirmed, cargo clean, cold
verification replacing incremental; per-batch
records in `round36-out/coord/` in the standing
format). Contribution batches (B–F) carry the
fuzz hard term: verdict/comp byte-identical to
the standing baselines, rights counters
5083 / 1024 / 5275, safety_flips=0 — a batch that
moves a fuzz reading stops and reports.

- **R36-A** — F6 + F7 (zip doors): candidate
  discipline in `zip_directory`; checked
  arithmetic in `raw_payload` ×4. Pins: both F6
  vectors at all four doors with their controls;
  the zpain3 compound refuses in standing words;
  a direct over-range spot at the font and clip
  doors refuses; every standing door pin
  unweakened.
- **R36-B** — F3 + F2 (Sunday-after consumption):
  the group-signals gate asks the raw token for
  its bracketed word; `sunday_after_base` unwraps
  the base's leading weekday as its twins do.
  Pins: the canary family (canary / agreeing /
  whole-line, all outer wrappers, both routes)
  field-identical to paren twins; the F2 family
  (wrapped base-leading weekday, agreeing and
  disagreeing, behind-"on" control unchanged).
- **R36-C** — F1 + F10 + F9 (re-spacing gate past
  its pattern limits): mixed-pair wrappers and
  any standalone group at the month–day and
  leading slots take the spaced reading; the
  trailing mixed-pair garnish is judged. The
  falsified comment at composition.rs:1793-1796
  is corrected. Pins: the full witnessed families
  from all three lanes, glued == spaced
  field-for-field, both routes; `((Monday))` and
  the single-pair pins unweakened.
- **R36-D** — F4 (leading-group checks at the
  three relative-phrase sites recognize all three
  pairs). Pins: restating / disagreeing /
  Sunday-after-relative families, all pairs,
  both routes, against the paren readings.
- **R36-E** — F5 (governing words wearing
  `! ? ' "` inside bracket/brace groups govern as
  their paren and bare twins). Pins: the four
  marks × both pairs × the governor family, with
  the `;`/`:` controls unchanged.
- **R36-F** — F8 + F11 + F12 (recovery and trim
  parity for the other pairs): stray marks judge
  as the paren pair's do; the in-words memory
  treats a stray `[` as the paren twin treats
  `(`; the shape-word trim learns the sentence
  marks. Pins: the F8 witnesses, `[next Sunday`
  against `(next Sunday`, the F12 mark family.

Then the coordinator close: five-leg evidence on
the final tree (local cold, fuzz as claims,
hosted CI ×3, desktop runner build, Windows
bundle), boundary hygiene, and the build-cache
policy disposition at the confirmed completion
(df reading + window + latch state over the
round's recorded readings). HF-12 untouched.
The Tier-2 final gate is not triggered (0/8/4)
and is not the coordinator's to start in any case.
