# Audit round 35 — synthesis (coordinator)

Audited tree: `8b6c5b2` (round-34 close record on code
tree `197ab50`). Lanes, blind and read-only: verify
(`round35-verify.md`), review (`round35-review.md`),
red team (`round35-redteam.md`), all in
`~/workspace/openbulletin-audit-wave3/` with probe
outputs and witnesses under `round35-out/`.

**Verdict: 0 BLOCKER / 6 MAJOR / 2 MINOR deduped —
the exit bar is NOT met.** Majors trend:
10 → 6 → 6 → 1 → 4 → 3 → 2 → 5 → 6.

Verify lane: every standing claim PASS, no findings.
Suite re-run 1011/0 (wall 13m55s, warm, under
parallel-lane load; a first attempt was killed by a
VM restart at 465/0 and discarded as a verdict, its
partial log kept for provenance). Fuzz against the
NEW R34-E baselines: verdict27 40,000 lines,
panics=0, stdout md5 `7bcbfdc0306f9efc642bb03462cba385`
(ruling counters reproduce: 5493 / 5336 / 10829);
comp27 D_TOTAL=497024, D_PANICS=0, stdout md5
`96b0c7b2ed7eedae6f8dcfa64b0d88ff`; bin sources at
their recorded md5s (`a9cac6c3…` / `4f85c605…`).
No lane produced evidence against the re-baseline;
it stands. Rights bins byte-identical to the D2
references, safety_flips=0, route-disagreement
counters exactly at the R34-D2 landed values
(5083 / 1024 / 5275) — M8 held, fourteenth
consecutive round. Booklet pin exact. The verify
lane's clippy PASS carries its own honest caveat:
artifacts were fresh against this docs-only-change
tree, so no crate was re-linted in that run; the
round's clippy claim rests on the round-34 close's
cold clippy over this same code plus the fix
batches' cold clippy below.

Dedup notes: review R3 and red-team RT35-4 are one
finding (F3). Review R4 and red-team RT35-3 are one
finding (F6), graded at the red team's MAJOR: its
witnessed cells include a cancelled date standing
live in silence, the harm this project grades
hardest; the review lane's framing point (the
round-34 record presents an opener-selected family
as one settled behavior) is carried into F6's
disposition as a record correction. Review R2 and
the red team's held-line note on the inner
cancellation token reconcile by form and are one
finding (F2): in trailing form the date surfaces
under a parenthesized-mismatch record (both lanes);
in whole-line form there is no record at all (the
review lane's new evidence).

---

## F1 (MAJOR, new; review R1) — A weekday garnish in braces is never judged, at any door

A disagreeing weekday in braces is silently ignored
and the date stands, where the paren and bracket
twins refuse the date and record the mismatch.
Witnessed spaced, glued, extra-close, doubled,
punctuated, and leading, on the For line and the
Keep-through line (review `twins-35.log`,
`more-35.log`). The agreeing brace garnish reads
clean, so only the disagreeing case diverges — the
silenced check, the defect class of R33-B/F2 and
R34-C/F4, on the one pair those batches' pins did
not reach. Mechanism: the weekday readers trim a
token's wrapping marks with their own character
sets and none includes `{` or `}` —
`composition.rs:1503`, `:1646`, `:1872`, `:1959`;
`dates.rs:434-435`, `:483`, `:526` — while the
shared `group_token` (`dates.rs:1249-1251`) trims
all three pairs. The group machinery strips the
brace group as anonymous garnish and the date
reads. Pin gap: `tests.rs:21001`
(`r34e_glued_bracket_garnish_reads_as_its_spaced_and_paren_twins`)
compares the glued brace form only against its
spaced brace twin — the two unjudged forms agree
with each other, so the pin passes while both
diverge from the paren twin the test's name claims.

Disposition: fix, batch R35-A. The weekday readers'
trim sets learn the brace pair (judged as the
other pairs are), and the pin at `tests.rs:21001`
is corrected to compare against the paren twin as
its name claims, with the brace family pinned
across the R34-C suffix layers like its twins.

## F2 (MAJOR, carried item 1b with new evidence; review R2) — A cancellation word wrapped in brackets or braces inside a group inner does not govern

The round-34 record carried this shape as a
garnish-role holdover. The cross-cells isolate the
INNER token's pair as the deciding variable, and
the whole-line consequence is worse than the
record states (review `inner-35.log`,
`more-35.log`):

- Whole-line: `(Nov 8, 2026 [cancelled])`,
  `[Nov 8, 2026 [cancelled]]`,
  `{Nov 8, 2026 {cancelled}}`,
  `[Nov 8, 2026 [postponed]]` all read
  **for=2026-11-08 with no record — the cancelled
  date is promoted to the bulletin Sunday in
  silence.** The paren-wrapped and bare inner
  tokens govern in every outer pair.
- Trailing: the bracketed inner token kills the
  date but produces a **ParenthesizedDateMismatch
  record** — the sender's cancellation is
  misreported to the user as a date disagreement.
  The control `(… (choir))` produces the same
  record: the bracketed token behaves exactly
  like the garnish word "choir" in every cell,
  including when the token is a governing word.

Mechanism: `group_token` (`dates.rs:1221-1253`)
cleans a wholly bracketed token to nothing by the
round-24 rule — `is_bracketed_token`
(`dates.rs:1078-1083`) covers `[]` and `{}` only —
while a paren-wrapped token edge-trims to the word
itself. The F3 adjudication's cited premise ("a
bracketed token cleans to nothing at every token
site, exactly as garnish does") is sound for
garnish words and wrong for the governing word
class: cancellation and postponement are the words
a group inner exists to carry. The r25a held line
was decided on `[choir]`, a garnish word;
extending it to governing words is the seam.

Disposition: fix, batch R35-B. A governing word
governs however it is wrapped: the token cleaning
distinguishes the governing class (cancellation /
postponement words) from garnish — a wholly
bracketed or braced governing word cleans to the
word, exactly as its paren twin does — while
garnish words keep the round-24 rule untouched.
Pins: the full cross-cell battery (3 outer pairs ×
inner forms: bare, paren, bracket, brace; whole-line
and trailing; cancelled and postponed), the
`[choir]` garnish control unchanged, and the
trailing form must produce NO mismatch record for
a governing inner token (the cancellation is the
record).

## F3 (MAJOR, carried observation 2, witnessed and graded by two blind lanes; review R3 == red team RT35-4) — A weekday group glued mid-line escapes the weekday judgment its spaced twin receives

The garnish gate (`spaced_glued_weekday_garnish`,
`composition.rs:1717`) re-spaces only a TRAILING
glued group. At the leading slot and the month–day
slot a glued weekday group is invisible to every
reader, in all three pairs (review `midline-35.log`,
red team `garnish35.log`, `keep35.log`):

- Leading: `(Monday)November 1, 2026` reads the date
  with no record; spaced `(Monday) November 1, 2026`
  refuses and records. Same for `[]` and `{}`.
- Month–day: `November(Monday) 1, 2026` reads the
  date with no record; the spaced twin and the bare
  word both judge the line (for=None). The agreeing
  glued twin diverges identically (glued reads,
  spaced does not), and the Keep route is the same:
  glued `November[Monday] 15, 2026` keeps the date
  silently, spaced reads none.

The spaced readings are standing behavior at both
slots, so the defect is only the glued escape —
R33-B's shape at the slots its gate never covered.

Coordinator ruling (the adjudication RT35-4 asks
for): spacing is not semantics, the doctrine of
R33-B/R34-C. A glued weekday group judges exactly
as its spaced twin at every slot — leading,
month–day, trailing — whichever way the slot's
standing spaced reading judges. The alternative
rulings (mid-line groups are garnish in both forms;
glued is garnish while spaced refuses) are rejected:
the first would repeal standing spaced behavior at
two slots, the second defends the present split,
which neither form defends.

Disposition: fix, batch R35-C (with F7). The
re-spacing gate extends past the trailing slot, or
the readers gain the equivalent judgment at the
leading and month–day slots; pins cover all three
pairs × both slots × agreeing/disagreeing × the
Keep route.

## F4 (MAJOR, new; red team RT35-1) — A fake central directory planted in the gap before the real one steers the plain-archive hunt

The R34-B hunt validates what it finds, but its two
flavours are not equally armed: a zip64 candidate
must agree with its record on the archive offset
(`record_archive_offset`); a plain candidate is
believed on its walk alone
(`validated_central_candidate` with
`record_archive_offset: None`). A central-directory
digital-signature record (PK\x05\x05, supported by
design) makes the arithmetic derivation fail, and
the hunt then searches forward from the declared
`cd_offset` in file coordinates — across any gap of
unused bytes between the last entry's data and the
real directory on a junk-shifted archive. A
complete fake directory planted at the head of that
gap, mirroring the real names with each duplicated
name's copies replaced by the first copy's record,
validates for the hunt AND for the zip reader (its
own CDFH hunt finds the fake first, and the fake
stands after all entry data, satisfying
`header_start < directory_start`). Scan and reader
judge the fake directory together; the real
directory is never consulted. Witnesses (red team
`witness35/`, `zipgap35*.log`): IDML serves the
first copy of a disagreeing duplicate story with no
refusal; fonts land only the first copy's face;
clips land the first copy's image twice. Controls
(gap-clean, no-signature, zip64 twin) all judge the
real directory. The junk-planted variant of the
same craft fails closed (the reader's
`header_start` discipline), recorded as the dead
end that shaped the witness. Exposure shape: plain
EOCD + signature record + junk shift ≥ gap +
planted directory. The review lane's code read of
the R34-B hardening was clean; the defect is
behavioral and sits in the plain flavour's missing
anchor — observed behavior over the code read,
which is why the lanes are plural.

Disposition: fix, batch R35-D. Either or both of
the red team's shapes: give the arithmetic path the
hunt's signature-terminator allowance so this shape
derives the real directory and never reaches the
hunt; and/or give the plain hunt the offset anchor
the zip64 candidate already carries — the
candidate's implied archive offset must agree with
the end record's own frame. Pins: the three door
witnesses (disagreeing duplicates refused / both
copies landed) plus the three controls unchanged,
and the round-33/34 shifted-archive families
unchanged in outcome.

## F5 (MAJOR, new; red team RT35-2) — A run of unclosed group openers is quadratic in the shared span machinery

Every standing timing pin judges closed groups. A
line of unclosed groups costs quadratic time for
all three delimiter kinds and their mix (release
probe, interleaved best-of-3 vs the absolute-group
control, ratios growing with size — the quadratic
signature): unclosedbracket 16.7x at 8,000 groups →
30.5x at 16,000; unclosedparen 23.4x → 31.2x;
unclosedbrace 21.6x → 28.6x; unclosedmix 42.2x →
55.2x. Single-run scaling reaches 57.7 s for a
~1.9 MB line of 64,000 unclosed groups, far inside
the doors' size caps (red team `timing35.log`,
`timing35b.log`, `scaling35.log`; load ~3.7–5.3).
Mechanism: `parenthesized_spans` searches forward
from each opener for its pair's closer and, finding
none, advances one byte and repeats — every opener
in an unclosed run rescans the whole remaining
line, and the span/mask readers built on it inherit
the square. RT34-7's hypothetical regression shape,
standing in the current code on the unclosed shape;
not a regression.

Disposition: fix, batch R35-E. One-pass span
finding, or a per-pair closer-absence memory that
stops re-searching a suffix already known to hold
no closer of that pair. HARD TERMS: readings do
not move — the extended differential corpus stays
byte-identical and the fuzz stdouts stay
byte-identical to the standing baselines; if the
perf fix proves inseparable from span semantics,
the writer stops and the coordinator re-decides.
New timing pins: the unclosed shapes (all three
kinds + mix) at two sizes under the interleaved
ratio form, bounds set between the fixed code's
measured ratios and the un-fixed code's.

## F6 (MAJOR, proposed by red team RT35-3, framing by review R4; carried item 1a past its recorded shapes) — Unclosed and mismatched groups judge by opener-selected rules the adjudicated sentence does not describe

The two recorded carried shapes behave
byte-identically to `bracket34.log` (verified by
both lanes) and are not re-filed. Past them, the
family is opener-selected, not one behavior (red
team `bracket35.log`, 65 cases; review
`mixed-35.log`, `bare-35.log`):

- Swapped nested closers `([…, cancelled)]` /
  `[(…, cancelled])` erase the whole line
  (for=None, silently); the correctly nested twins
  cancel cleanly.
- An unclosed bracket or brace group is
  transparent: `[Nov 8, 2026, cancelled` leaves the
  cancelled date live as a second date, in silence.
  An unclosed paren group poisons the whole line.
- A mixed range `[Nov 8 … through Nov 15, 2026,
  cancelled)` leaves BOTH range dates live; the
  pure twins record Nov 8 as a mismatch and keep
  Nov 15.
- A mixed group before the core `[…, cancelled)
  November 1, 2026` erases the core (for=None); the
  pure bracket twin composes clean.

Every probed cell reduces to the OPENER's standing
unclosed-group behavior, and those behaviors are
opposite: paren-era rules when it opens with `(`
(loud: everything refused), no rules when it opens
with `[` or `{` (silent: a cancelled date inside
stands live with no record). The R34-E sentence —
brackets/braces are group delimiters "judged as
their paren twins" — describes matched groups
only; the round-34 record's presentation of the
mixed family as one settled behavior is corrected
by this synthesis.

Coordinator ruling (the adjudication RT35-3 asks
for): a group is exactly what the shared stack
walk closes. An opener the walk never closes forms
NO group; its words judge as bare text, under the
same readers that judge bare words everywhere
else. A closer that does not match the walk's open
span is a stray mark, as stray marks already
judge. Every span consumer reads from the walk,
not from a private search. Consequences, stated
plainly: an unclosed paren group stops poisoning a
line its words do not govern (the core before it
stands; a bare `cancelled` inside governs its date
loudly, as bare `cancelled` already does); an
unclosed bracket group stops being transparent
(its bare words judge, so its cancelled date is
refused like any bare cancellation); twin-equality
for MALFORMED groups is explicitly not the rule —
the rule is one rule, and no silence: no malformed
group may leave standing a date its words would
refuse if bare, nor erase a date its words would
leave standing if bare, without the record the
bare reading produces.

Disposition: fix, batch R35-F, after R35-E (the
reading change lands on the fast machinery; R35-E's
timing pins must still pass). The writer pins the
adjudicated outcomes over the red team's 65-case
battery (`bracket35.log`) and the review lane's
mixed battery before changing readers, and surfaces
any cell the ruling does not plainly cover to the
coordinator BEFORE commit. Fuzz gate: hard term —
if the contribution stdouts move, the batch stops
and the coordinator adjudicates the itemized delta
(the movement, if any, must be exactly the
malformed-group family this ruling re-reads).

## F7 (MINOR, new; red team RT35-5) — A disagreeing garnish followed by `'` or `”` is judged in neither form

`November 1, 2026 (Monday)'` and `(Monday)”`, glued
and spaced alike, read the date clean with no
mismatch record (red team `garnish35.log`). The
R34-C peel set (`. , ; : ! ? "`) stops before the
apostrophe and the typographic closing quote a
phone keyboard produces after a garnish. Both forms
agree, so nothing diverges; the check is simply
absent for these two marks. Disposition: fix with
F3 in batch R35-C — the peel set learns both marks,
pinned in both forms.

## F8 (MINOR, new; red team RT35-6) — The displaced-path journal hold's "exactly as" comment overstates one mirror cell; the behavior is sound

A 40-cell mirror sweep over the retirement
decision table (4 journal shapes × 10 file shapes,
lock path vs displaced path; red team
`locksweep35.log`, replica re-extracted from this
tree) finds exactly two differing cells, one shape:
a file naming the RETIRING process's own pid under
a foreign token blocks retirement at the lock path
and does not hold at the displaced path. No live
peer can stand behind such a file (two live
processes never share a pid; its owner is this
instance's earlier generation or a dead process
whose pid recycled to this instance), so no journal
of any live peer is retirable through the cell; the
displaced reading follows the code's own
construction and the lock path is the
over-conservative side. The casualty is the code
comment's claim ("holds its owner's journal from
the displaced path exactly as it would from the
lock path"), not a safety axis. The sweep also
verifies the F5 cell (R34-A) fixed on this tree and
the rest of the table mirroring exactly.
Disposition: fix, batch R35-G — correct the comment
to state the mirror with its one named exception
and the reason; no behavior change; the sweep's
cells pinned if the harness allows without new
machinery, else the sweep log stands as the record.

---

## Held lines and coverage (from the lanes, condensed)

- R34-E pure twins agree across all 12 red-team
  groups and the review lane's 14-shape × 3-pair
  sweep, apart from F1: cancel dates, cancel words,
  ranges (incl. the adjudicated range split),
  alternatives, relatives, nesting, postponed,
  Keep route. R34-C garnish layers hold through
  triple and mixed-pair doublings (31/35 pairs SAME;
  the 4 diffs are F3).
- Rights M8 attacked at quadruples: 114,244 route
  outcomes, 0 safety violations, 0 credit drops;
  interleaved-hash folds order-invariant
  (`rights4-35.log`, `rights5-35.log`).
- Stray foreign delimiters inside matched groups
  judge as the clean controls; deep cross-nesting
  to 20,000 without panic; 28,004 generated cases,
  0 panics (red team `fuzz35.log`). Closed-shape
  timing linear to 32,000 groups.
- Lock: R34-A/F5 cell verified fixed on this tree;
  50 concurrent claim-vs-restore races hold. The
  F6/F9 recycled-pid residual (round 32) stands
  recorded-not-fixed; no lane re-raised it and no
  new evidence appeared.
- Zip: the junk-planted fake family fails closed at
  every door; shifted clean archives judge exactly
  like unshifted.

## Batch plan

| Batch | Findings | Shape |
|---|---|---|
| R35-A | F1 | Weekday readers' trim sets learn `{`/`}`; the `tests.rs:21001` pin corrected to its name's claim; brace garnish pinned across the suffix layers |
| R35-B | F2 | Governing words govern inside group inners in every wrap; garnish rule untouched; cross-cell battery pinned; no mismatch record for a governing inner token |
| R35-C | F3 + F7 | Glued weekday groups judged as spaced twins at leading and month–day slots (Keep route too); peel set learns `'` and `”` |
| R35-D | F4 | Plain-hunt offset anchor and/or arithmetic signature-terminator allowance; gap-craft witnesses + controls pinned at the three doors |
| R35-E | F5 | One-pass span finding / closer-absence memory; readings byte-preserved (differential + fuzz hard terms); unclosed timing pins at two sizes |
| R35-F | F6 | The stack-walk ruling: no group without a walk close; unclosed/mismatched words judge bare; 65-case battery pinned to adjudicated outcomes; record correction carried by this synthesis |
| R35-G | F8 | Comment correction only (the mirror's one exception, with its reason); no behavior change |

Each batch: one writer; commit + push + HEAD ==
origin confirmed; `cargo clean`; verification COLD
from the clean tree (full unpiped suite + clippy
-D warnings + booklet pin; fuzz-as-claims against
the standing baselines where the batch touches the
contribution engines), with per-batch disk and
wall-time records, per the batch-boundary
discipline. Contribution-reading batches (A, B, C,
F) carry the fuzz hard term: a moved stdout stops
the batch for coordinator adjudication of the
itemized delta; a rights-counter rise does not
land (the standing R34-D2 acceptance shape).

Round 35 audited 0 BLOCKER / 6 MAJOR / 2 MINOR —
the exit bar is NOT met by this round's audit
verdict. The Tier-2 final gate is not triggered.
HF-12 remains Jacob's decision, untouched.

---

## F6 adjudication addendum (coordinator, 2026-10-21)

R35-F stopped at its procedure step 2, correctly:
the step-1 battery (108 cases with bare twins,
`round35-out/coord/r35f-*`) proved the F6 ruling as
written underdetermined, and three of the brief's
stated consequences were wrong about their own
bare references. The writer's cells file
(`r35f-cells.md`) is the evidence; this addendum
is the adjudication it asked for, and it narrows
F6's mechanism and restates two of its cells.

What the battery established: the group machinery
already forms NO group for every malformed shape
in the battery — the stack walk closes nothing,
the reverse walk finds no trailing group, the flat
search passes the openers over. The divergent
readings do not come from a span consumer at all.
They come from one asymmetry in the plain readers:
`parse_date_prefix`'s normalization trims garnish
at a token's edges EXCEPT the two parentheses,
which its comment defends as structural ("a date
token wearing one stands at a group edge, and the
raw-text scans rely on this parse failing there").
So `(November` is not a date signal while
`[November` trims and reads. That asymmetry is
the whole live mechanism of F6. 86 of 108 battery
cells already read exactly as their bare twins;
8 more are determined holds (stray closers and
mixed garnishes, judging as the ruling's
stray-mark clause already provides).

Adjudication of the three questions the cells file
poses:

1. **The glued unmatched mark.** "Judge as bare
   text" means reading (ii), scoped: an opener
   the shared stack walk does not close is a
   STRAY MARK, inert at the plain readers — the
   treatment `[` and `{` unmatched already
   receive — while a walk-closed group's edge
   marks keep their structural effect and the
   paren rule's designed reliance stands there.
   The ruled outcome for every battery cell is
   the reading of the text with unmatched marks
   inert and everything else (walk-closed groups,
   commas, all other text) untouched: the
   battery's bare-twin fields. Implementation
   shape is the writer's (the trim learns the
   walk's verdict, or the composition pre-marks
   unmatched openers); the pins judge outcomes.
2. **Cell C** (`[…, cancelled)` after a core):
   `secs=[2026-11-08]` — governed and NAMED on
   the second-date note, exactly as bare governed
   dates are named — IS the ruled outcome; the
   fields already match the bare twin today. The
   F6 text above and the round-34 record describe
   this cell as the cancelled date "standing
   live"; that phrasing is corrected here: no
   silent service exists in this cell, and the
   brief's "changes" for it is a provenance
   statement, not an outcome. This cell is a hold.
3. **Cell D** (`[…, cancelled)` before the core):
   malformed marks do NOT break a governor
   window's reach. The bare cancellation standing
   before the core governs it, as bare
   `cancelled November 1, 2026` governs its date;
   `for=None` is the ruled outcome and the
   current one. The brief's implication that the
   core stands is restated as wrong. Hold at
   field level.

Restated change surface (the whole of F6 as
adjudicated): the paren-poison family — a date
token wearing an UNCLOSED `(` becomes a signal,
as its bracket/brace twins already are. Cells A
and B and their family and the swapped-nested
cells E change to their bare-twin fields
(A's recorded round-34 outcome, `for=None`, is
NOT kept: it was the poison, and the poison is
the defect). Cell F (mixed outer, walk-closed
inner) takes its bare-twin fields — inner group
structural, outer unmatched marks inert, commas
real text with their standing effects. The mixed
range and the remaining named cells already
equal their bare readings and hold. The
no-silence invariant survives in its correct
form: after this batch, no malformed shape's
reading differs from the reading of the same
words with their unmatched marks removed — one
rule, checkable cell by cell against the battery.

R35-E's timing pins are untouched by this
adjudication (their witnesses read `for=None`
under both candidate models, per the cells file).
The fuzz gate for the re-dispatched batch: rights
byte-identical and counters unrisen, as standing;
a contribution-stdout delta is itemized and must
be confined to unmatched-opener shapes — a moved
walk-closed or well-formed case is a defect, not
a delta.
