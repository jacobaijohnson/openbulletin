# Audit round 32 — synthesis (2026-10-16)

HEAD audited: `8264801ef33a391e7f24d9df5d85a12de9a59876`
("Round 31 coordinator close"). Base for diffs: the
round-31 audited tree `3722169` (round-30 close); the
delta is exactly the R31-A..E fix batches plus the
round-31 close commit. All three lanes read-only and
blind; repo `git status` clean at every lane's close,
HEAD unchanged. Lane scratch:
`round32-out/{verify,review,redteam}` (+ `probe32v`,
`probe32rt`); lane reports `round32-verify.md`,
`round32-review.md`, `round32-redteam.md` in the
audit dir.

**Round 32 verdict: 0 BLOCKER / 3 MAJOR / 9 MINOR
deduped — the exit bar is NOT met.** Round 31's
pattern repeats one level down: every major lives in
machinery a fix batch built. R31-B's atomic claim
kept a non-atomic displacement step (F1). R31-C
moved a date reading its standing gate forbade
moving (F2) and left the cancelled-group shape
quadratic in the arrangement F4 actually measured
(F3). The loop continues to round 33 after the fix
batches below.

Verify lane: **PASS, 0 / 0 / 1** (the one MINOR is a
measurement-record note, F12). Every standing claim
re-run fresh on the audited tree: suite **940 passed
/ 0 failed** (`SUITE_EXIT=0` read from disk), clippy
`-D warnings` exit 0, booklet **31,717 / 34,977**
byte-exact. Fuzz bins as claims at their recorded
md5s (verdict `a9cac6c3…`, comp `4f85c605…`):
verdict 40,000 lines, panics=0; composition 497,024
lines, D_PANICS=0, governed resurrections 943; **both
stdouts byte-identical to the round-31 coordinator
baselines (md5 verdict `4941c4f9…`, comp
`efd2096b…`)**. Rights axes (M8): 187,236 route
comparisons, safety flips 0 on every axis; rights
stdouts byte-identical once an echo line inside the
old baseline files themselves is accounted.
Test-body integrity `3722169` → `8264801`: exactly
1 test removed and exactly 2 assert lines removed,
both the adjudicated R31-B off-Unix pin rewrite
(replaced by a stronger pin); the 8 same-name
body-change flags all resolve to the adjudicated
adaptations or extractor artifacts. Spot-checks of
the round-31 close: both forged-CRC twins now refuse
at the loader; the five R31-C timing shapes are
linear (relative-core groups at 100 KB: 95.6 ms
median here vs 11,154 ms pre-fix); read-name
literals exist only at the three constants (F12
records the one number that did not reproduce as a
median). Environment incident, on the record: the
exec layer delivered a premature "940/0 complete"
summary while the suite log was still mid-run; the
lane discarded it and every number above was
re-read from completed on-disk logs.

Review lane: **0 / 2 / 1.** MAJOR 1 (≡ F1) read
directly from source; MAJOR 2 (≡ F2) proven by a
differential probe through the public parser against
a `git archive` copy of `3722169`. Cleared with
evidence: the ob-format payload judgment at both
JSON roots (fail-closed, zip64 and data descriptors
handled, assets sha256-bound); the decision table's
uniformity, the Windows FFI's handle hygiene, the
unnamable-pid carve-out at claim and retirement, and
every standing pin unweakened in assertion
(mechanical assert-multiset comparison); R31-C
fixes 1–4 proven equivalent; the whole of R31-D and
R31-E.

Red team lane: **0 / 2 / 7**, and a clean panic hunt:
46,000 generated hostile contributions plus targeted
shapes (100,000-deep nesting, 2 MB tokens, embedded
NULs, absurd years) — **0 panics**, worst targeted
input 1.3 s. RT32-1 (≡ F1) carries the runtime
witness review could not build: an instrumented
interleaving of the verbatim extracted claim steps
with real processes, ending with the loser retiring
the winner's live journal; the uninstrumented rate
is honestly bounded (0/300 barrier-synchronized
paired launches — the window is a few syscalls).
Held under attack: every forged-CRC twin refused at
the .obd/.obdi roots in all packagings (stored,
deflated, descriptor, zip64, three-copy); .mxl/IDML
duplicate doors and their caps; the zombie-pid
recorded boundary exactly as recorded; **rights
composition — M8 holds, tenth consecutive round**
(39,627 record sets × both routes × all orders: 0
manufactured permissions, 0 unvouched licenses, 0
children/permission flips).

Raw lane totals 0 BLOCKER / 4 MAJOR / 9 MINOR dedupe
to the 3 / 9 below: the lock displacement race was
reported independently by review and red team (one
root, F1).

## F1 (MAJOR) — The atomic claim's displacement step renames whatever stands at the lock path, including a live winner's lock (review MAJOR-1 ≡ red team RT32-1)

`acquire_instance_lock_from`
(`crates/ob-app/src/main.rs:3614`) re-judges the
standing lock, then `displace_instance_lock_from`
(`main.rs:3576`) renames **whatever file currently
stands at the path** to
`openbulletin.lock.displaced` and proceeds straight
to create-new. The displaced file is never inspected:
no content re-read, no generation check, and after a
successful rename nothing re-reads the displaced
file before the claimant creates its own lock.

The interleaving (the stale lock a previous run left
behind is the standing state — no code path deletes
`openbulletin.lock`, so this is the common
double-launch path): B re-judges the stale lock
takeable; A displaces it, creates its own, re-reads,
holds; B resumes and displaces **A's live lock**,
creates its own, its post-write re-read names B.
Both claims answer true, and B's later saves can
retire A's live journal — F1's data-loss shape
entering through the F3 fix, exactly the outcome
R31-B's adjudication feared for the pre-fix claim.
The function's own comment (`main.rs:3571`) states
the intended arbiter property — "the loser of the
rename finds the path gone or already re-created and
re-judges what stands there instead of proceeding" —
and the code does not do it. R31-B's interleaving
pins call the claim sequentially and structurally
cannot force check-check-displace-displace.

Evidence is twofold: the source read (review), and
an instrumented interleaving of the verbatim
extracted steps with real processes (red team,
`round32-out/redteam/lockstep.log`): the displaced
file held A's live lock (`"43208\nA-token\n"`), B
claimed true, and `retire_journal_from` then allowed
and cleared live A's journal. The natural rate is
bounded and stated: 0/300 barrier-synchronized
paired launches produced two holders; the structural
defect is not in doubt, because the rename verifies
nothing about the file it moves.

**Fix (R32-A):** displacement becomes conditional
on generation. A claimant may displace only the
exact lock generation it judged: verify the standing
file still is that generation at displacement time,
and audit after the rename — re-read the displaced
file; if it names anyone other than the judged lock
(a live peer or a different generation), restore it
and stand down (the claim answers false or re-enters
the judge loop; it never proceeds to create over a
displaced live lock). Pins must force the
check-check-displace-displace interleaving at the
`_from` seams (extracted-step seams if the claim's
steps need naming to be interleavable), plus the
restore path. The standing R30-A / CI-FIX / R31-B
pins stand unweakened.

## F2 (MAJOR) — R31-C changed a date reading: a one-token relative remainder cut at its tail is judged on the token's full pre-cleaning (review MAJOR-2)

`relative_shape_words`
(`crates/ob-extract/src/contribution/dates.rs:705`)
judges a remainder range on the header line's
pre-cleaned tokens; edge tokens cut by the range are
supposed to be re-cleaned from their in-range part.
The head edge is re-cleaned; the tail edge is gated
by `t_last > t_first` (`dates.rs:724`), so when the
range holds exactly one token, uncut at the head and
cut at the tail, `tail_cut` is false and the word
judged is the token's **full** pre-cleaning. The
pre-fix code (`3722169`) judged the stripped
remainder's own text, cleaned on its own.

Witness (differential probe through the public
`parse_contribution`, both trees): the For Sunday
line `Sunday(November 8, 2026)` — the trailing group
strips without needing a space before `(`, leaving
the one-token remainder "Sunday", which on HEAD is
judged as `sunday(november` and reads **nothing**;
on `3722169` the line reads (in-words Sunday plus
the parenthesized Nov 8). The spaced twin and the
two-token `next Sunday(...)` form read identically
on both trees, isolating the shape: a one-token
remainder cut at its tail. Sender-plausible (no
space before a parenthesis is ordinary typing). The
shape sat in neither R31-C's 65,720-case
differential corpus nor the fuzz bins, so the
close's "no reading moved" is broader than its
evidence — the same claim-vs-coverage gap as F1,
in the batch whose standing gate was that no
reading may move.

**Fix (R32-B):** the tail re-clean runs whenever
the token is cut at the tail, one-token remainders
included; the judged word is always the in-range
part's cleaning. Pins: the witness reads as on
`3722169`; the spaced and two-token twins unchanged;
the family pinned (glued one-token remainders for
the relative shapes generally). The batch adds the
family to the differential corpus the R31-C gate
uses, and its no-reading-moved evidence is that
corpus plus the fuzz bins byte-identical.

## F3 (MAJOR) — The cancelled-group shape is still quadratic with a standing core; four sibling shapes (red team RT32-2)

R31-C's close states all five measured shapes are
linear. Four are (verify re-measured them; red team
confirms at control cost). The fifth — the cancelled
full-date group — is linear only without a standing
core. With a core date on the For Sunday line
followed by groups carrying `cancelled`/`canceled`
**inside** the group (the arrangement round-31 F4
measured), the cost is quadratic on the close tree
(min-of-5, absolute-group control interleaved at the
same group count; `round32-out/redteam/shapes2-*.log`):
core + `(November 1, 2026, cancelled)` ×8000,
240 KB → **11,605 ms** (145× control, 3.2–4.1× per
doubling); core + `(next Sunday, cancelled)` ×8000,
200 KB → **35,199 ms** (447×). Siblings in the same
family: distinct-date cancelled groups (116×),
one group holding n same dates (188×) or n distinct
dates (36×), leading groups before a relative core
(32×), and bracket groups superlinear by 608 KB
(~11× and growing). The discriminators are measured:
no core → linear; the word outside the group →
linear; the same groups in a body line → flat. The
heat is the cancellation machinery rescanning per
cancelled value against the line's standing signals
(plus the one-big-group and leading-garnish sites),
which R31-C's five fixes did not reach.

There is no contradiction with the verify lane: the
pinned cancelled shape held; the pin did not carry
a standing core. The round-31 record measured this
form pre-fix at 54,034 B → 2,168.5 ms; this tree
measures 60,050 B → 791 ms for a related form —
faster, and still quadratic where the close claims
linear. A 240 KB header line (a season's schedule
pasted into one contribution) freezes the parse for
~12 s; the cancelled-relative form reaches ~35 s at
200 KB.

**Fix (R32-C):** find and bound the per-cancelled-
value rescans against standing signals in the header
group/composition path, and the one-big-group,
leading-garnish, and bracket-group sites. Timing
pins on the measured family: cancelled-with-core
(absolute, relative, distinct-date), one-big-group
(same and distinct dates), leading groups, bracket
groups — ratios near the interleaved control, the
way R31-C pinned its five. Standing gate, doubled:
fuzz bins byte-identical **and** no date reading
moves (readings pins plus the differential corpus,
by then carrying F2's family).

## F4 (MINOR) — The .mxl/IDML duplicate scans and the doors' own name resolution disagree on two shapes (review MINOR-1)

R31-A's raw central-directory scans key names by
`String::from_utf8_lossy` and walk exactly the
EOCD's declared `cd_offset`. The doors resolve names
by the zip reader's rules, and the two disagree:

- **CP437 (.mxl).** With the UTF-8 flag clear, the
  reader decodes names as CP437; the scan's lossy key
  never equals the resolved name, the duplicate
  check passes over a CP437-named pair, and
  `by_name` serves the last copy. Witness
  `cp437_dup.mxl` loads "Other Test Hymn"; the ASCII
  control refuses and a single-copy CP437 control
  loads, so the door genuinely reads such names.
- **Offset-shifted archives (IDML).** The reader
  tolerates junk prepended before the archive
  (derives an archive offset); the scan starts inside
  the last entry's body, breaks on the first
  non-`PK\x01\x02`, and returns no spots — no
  duplicate check happens at all. Witness
  `shifted_idml_dup.idml` loads "Other story words";
  the unshifted control refuses. (At .mxl this shape
  fails closed at the `PK` sniff instead — by the
  sniff, not the duplicate machinery.) ob-format's
  candidate machinery handles the shifted shape; the
  IDML scan does not.

Graded as F5 was: crafted files, review-surface
imports, no real exporter emits duplicates. Within
those bounds the claim "duplicates refused at these
doors" is broader than the code. **Fix (R32-D):**
scan and reader must not disagree about the name
set — the scans key names the way the reader
resolves them (CP437-aware) and derive the archive
offset the way the reader does (or refuse the
shifted shape in the door's own words). Pins carry
both witnesses plus the controls.

## F5 (MINOR) — An exact root repeat packed two ways (stored + deflated) refuses: payload judgment is gated on compressed-size facts (red team RT32-3)

R31-A's rule states root repeats are judged on the
payload bytes. The walk consults `note_read_name`
first, whose facts are (CRC32, **compressed size**,
uncompressed size) from the central record. Two
copies of the identical `document.json` payload,
one Stored and one Deflated, agree on CRC and
uncompressed size but differ in compressed size
(4133 vs 770), so the facts disagree and the file
refuses with the duplicate words **before the
payload comparison is reached** (witness
`t04-identical-mixed.obd`; the stored/stored and
deflated/deflated identical twins load). Fails safe
— nothing wrong is ever served — but payload
comparison only ever runs between same-packaging
copies, which is narrower than the batch's wording.
**Fix (R32-D):** for read names, a facts disagreement
that is only a compressed-size disagreement goes to
the payloads, which judge; the refusal words stay
for copies whose payloads actually differ. Pins:
the mixed-packaging identical twin loads; disagreeing
twins in mixed packaging still refuse.

## F6 (MINOR) — Duplicate names at the font-pack and clip doors still resolve as the archive's quiet choice (red team RT32-4)

R31-A put duplicate scans at the .obd, .mxl, and
IDML doors. Two zip doors never got one, and both
consume the reader's name-keyed map, which collapses
a duplicated name to its last copy before the door's
own defenses run:

- Font pack (`FontLibrary::add_zip_bytes`): two
  different fonts under one name land **only the
  last** — one result, no error, the first font
  silently absent (witness `f03-dup-font-name.zip`:
  DejaVuSans regular then Bold as `MyFont.ttf`;
  only Bold landed). The door's hashed-rename
  defense never sees the first copy. License files
  likewise: two `LICENSE.txt` entries land as the
  second text only, and two folders' license files
  flatten through `sanitize_file_name` to one name,
  last wins, with no count and no note.
- Clip zip (`ClipLibrary::add_collection_from_zip`):
  two `pic.png` entries with different bytes land as
  **one item, the last copy**, although
  `add_collection`'s `unique_name` dedup exists to
  land both — it never receives the first copy.

Graded as F5 was (MINOR). The project's stated
principle — which copy a file was read from must
never be the archive's quiet choice — still stops
at three of five zip doors. **Fix (R32-D):** both
doors gain the duplicate discipline in their own
idiom: disagreeing duplicates of a name are never
silently collapsed — fonts and clips land both
copies under distinct names via the doors' existing
dedup machinery, or refuse with the door's words;
exact repeats may collapse. License files at the
font door get the same treatment as the fonts
themselves. Pins carry the three witnesses.

## F7 (MINOR) — The font library's hashed-name fallback writes over whatever already owns the hashed name (red team RT32-5)

`add_bytes_known` (`crates/ob-fonts/src/lib.rs:440`):
when a different font already owns the clean name,
the incoming font is written under
`"{clean}-{sha256[:8]}"` with no existence check on
that target — the final `std::fs::write` is
unconditional. A file already standing under the
hashed name is silently replaced (witness: library
seeded with DejaVuSansMono at `plant.ttf-5c1247ac`;
importing DejaVuSans-Bold as `plant.ttf` returned
`file=plant.ttf-5c1247ac` and the mono file then
held the Bold bytes). The suffix is computable by
anyone holding the incoming font; the occupant can
arrive by any route (manual copy, earlier import).
Same shape R31-D fixed for catalog notes — plain
and hashed name spaces not disjoint, no occupancy
check — surviving here because the hashed name is a
plausible plain file name. The pack door itself
cannot plant the occupant (a `plant.ttf-<hash>`
entry is skipped as not ending in `.ttf`), so the
trigger is a pre-seeded library. **Fix (R32-F):**
the fallback checks occupancy: if the hashed target
is owned by different bytes, disambiguate further
(longer hash, then a counter) — a different font's
file is never overwritten. Pin carries the witness;
the same-bytes occupant case still lands as today.

## F8 (MINOR) — A tokenless lock disarms journal retirement even when its pid names a live peer (red team RT32-6)

`retire_journal_from` computes
`owner_provably_gone` from the lock's pid correctly,
but `journal_retirement_allowed`'s veto for a foreign
journal is token equality only, and a lock with no
token line reads as "the lock names no holder". At
the `_from` seam, a tokenless (legacy-shape) lock
naming a **live** openbulletin pid: the claim refuses
(`peer_alive=true`), but retirement allows and clears
a foreign journal in both journal shapes (witnesses
`lockcase onlypid-live-peer` and
`onlypid-live-peer-legacyjournal`). The function's
own doc says a tokenless note "is retired only when
the lock names no other live holder"; the lock names
one, by pid, and the pid never reaches the decision
table as a holder. Realistic standings: a pre-token
instance still running while a new version launches
(upgrade overlap), or a lock torn to its pid line.
Stated plainly, as the lane states it: whether the
app's flow calls retirement while a peer holds the
refused state was not driven end-to-end (the seam is
what the suite pins); the seam's contract contradicts
its doc. **Fix (R32-E):** a tokenless lock's pid
reaches the decision table as a holder — retirement
of a foreign journal under a tokenless lock whose
pid names a live peer is refused, exactly as a
token-carrying lock's is; doc and seam agree. Pins
carry both journal shapes.

## F9 (MINOR) — Peer identity is the exe file-name substring "openbulletin": a renamed live peer reads as provably gone (red team RT32-7)

`peer_provably_gone` (Unix arm): if
`/proc/<pid>/exe` resolves, a live pid is "provably
gone" whenever its exe file name does not contain
`openbulletin`; the cmdline fallback runs only when
the exe link is unreadable. A live peer whose binary
name lacks the substring — a renamed copy, a distro
or wrapper name — is therefore judged a recycled
stranger: its lock is taken and its journal retirable
while it runs (witness `lockcase live-renamed-peer`:
the same process named `openbulletin-peer` is
protected; named `bulletin` it is claimed over and
its journal cleared). The converse holds and is the
safe direction: an unrelated binary with
`openbulletin` in its name wedges the claim as a
"live peer". Stock installs (binary named
`openbulletin`) are safe. This is not the recorded
F6 residual — the zombie boundary held exactly as
recorded — it is the adjacent identity definition
giving way.

**Adjudication and fix (R32-E):** the substring test
can only ever prove the safe direction. From outside
a process, a renamed copy of the app is
indistinguishable from another program, so absence
of the substring cannot prove gone-ness; the
identity evidence is demoted to proof-of-peer only.
`peer_provably_gone` on Unix then rests on the
tri-state's Dead answer, and a live pid under a
foreign-token lock reads as a live peer whatever
its exe is named. This amends the round-31 F6
record: the recycled-pid case — identity readable
or not — joins the recorded-not-fixed family
(indistinguishable from a live peer; the wedge is
the safe direction working), which is also the
posture Windows already has (review's observation:
no identity refinement there at all). The identity
refinement stays where it is sound: confirming that
a live pid **is** the app.

## F10 (MINOR) — Catalog note names are injective only on byte-exact, case-sensitive filesystems (red team RT32-8)

`license_note_name` held over the exact-name space:
69 hostile ids (separator, dot-run, trim, unicode,
300-byte, crafted-hash-form) produced **zero exact
collisions** and an empty plain∩hashed intersection
— the R31-D construction does what it claims on
this filesystem. On the filesystems two of the
three release platforms use by default, distinct ids
still share one note: case variants (`Lato`/`lato`
on case-insensitive APFS/NTFS) and unicode
normalization variants (NFC vs NFD `café`) collapse,
so a later family's install silently overwrites the
earlier family's note on macOS/Windows. Real catalog
ids are lowercase slugs, so the trigger is a hostile
or unusual catalog answer (the S-4 threat model the
fence exists for). Separately, ids whose note name
exceeds 255 bytes fail the ignored `fs::write`
silently: the note is lost, though nothing is
overwritten. **Fix (R32-F):** injectivity is judged
under the filesystem semantics the app ships on —
the lossy test that routes an id to the hashed form
also fires when the plain form could collide under
case-folding or unicode normalization; over-long
names take the hashed form (short by construction)
instead of failing silently. Pins: case-variant and
NFC/NFD id pairs land as two notes on any
filesystem; a 300-byte id lands its note.

## F11 (MINOR) — The payload comparison's cap is asymmetric: the in-memory path materializes Stored copies whole (red team RT32-9)

`payload_bytes_file` refuses a root copy whose
compressed size exceeds `MAX_JSON_BYTES` (64 MB);
`payload_bytes_in` has no such check — for a Stored
copy it materializes the full data region before the
length test, and `root_repeat_disagrees` caches the
first copy while the current one is built. Witness:
a 168 MB file holding two identical 80 MB Stored
`document.json` copies peaks at **331,556 KB RSS**
(vs 167,796 KB for the tail-differing twin) — both
payloads materialized to prove a repeat the capped
reader then refuses anyway. Fail-safe in outcome
(nothing oversized is served; the JSON cap still
refuses), but a crafted file roughly triples its own
footprint in memory on the fallback path, past every
cap the loader advertises. The deflated arm is capped
in both paths; only Stored is asymmetric. (Review
noted the same pair as a cleared observation with a
narrower scope — refusal the safe direction, no
close claim covering the twins; red team's memory
measurement is what grades it. One root, recorded
once, here.) **Fix (R32-F):** the in-memory path
applies the same cap discipline before materializing
a Stored copy — over-cap copies are judged from the
central facts (and refuse, as the capped reader
does) without being built in memory; the compare
never holds two over-cap payloads. Pin: the witness
file's peak RSS stays within the loader's advertised
envelope.

## F12 (MINOR, record) — R31-C's "48 ms" point figure does not reproduce as a median (verify lane)

The verify lane re-measured the worst R31-C shape
(relative-core paren groups, 100 KB) at **95.6 ms
median, 62.8 ms best**, under load average 6–11, vs
the builder's 48 ms point in the round-31 close.
The scaling substance is confirmed by two lanes
(6.0× size → 6.9× time; four of five shapes at
control cost in red team's interleaved harness) —
the fix is real; the point number was a best-case
reading stated without its estimator or load
context. **Disposition: recorded, corrected here.**
The standing figure for that shape is ~96 ms median
under load (62.8 ms best). Future timing claims in
closes cite the estimator (min/median of N) and the
load context, as the lanes' harnesses do. No batch.

## Dispositions — fix batches

- **R32-A — the displacement verifies its
  generation (F1).** Conditional displacement +
  post-rename audit (restore and stand down if the
  displaced file names anyone but the judged lock).
  Pins force check-check-displace-displace at the
  seams, plus the restore path. Standing pins
  unweakened.
- **R32-B — the moved reading is restored (F2).**
  Tail re-clean for one-token remainders in
  `relative_shape_words`; witness + family pins;
  the differential corpus gains the glued
  one-token-remainder family and re-runs clean;
  fuzz byte-identical.
- **R32-C — the cancelled-group machinery stops
  rescanning (F3).** Bounds on the
  per-cancelled-value rescans and the sibling sites;
  timing pins on the measured family near
  interleaved-control cost; fuzz byte-identical and
  no reading moves (corpus + readings pins).
- **R32-D — duplicate discipline at every door,
  judged the same way (F4, F5, F6).** Reader-true
  name keys and archive offset at the .mxl/IDML
  scans; payload judgment for mixed-packaging root
  repeats; duplicate scans (or land-both dedup) at
  the font-pack and clip doors, license files
  included. Pins carry every lane witness.
- **R32-E — the peer tells the truth about holders
  (F8, F9).** A tokenless lock's pid counts as a
  holder at retirement; the identity refinement is
  demoted to proof-of-peer only and
  `peer_provably_gone` rests on Dead. The F6 record
  is amended as F9 states. Pins: tokenless-live-pid
  retirement refused in both journal shapes; a
  renamed live peer protected; the zombie boundary
  pin stands.
- **R32-F — naming and compare residuals (F7, F10,
  F11).** Occupancy-checked hashed fallback in the
  font library; catalog note injectivity under
  case-folding/normalization with over-long ids
  landing via the hashed form; the in-memory
  payload compare capped before materializing.
  Pins carry the three witnesses.

Recorded, not fixed: the F6 residual as amended by
F9 (a recycled pid, identity readable or not, is
indistinguishable from a live peer; the wedge is
the safe direction). F12 is a record correction, no
batch. HF-12 remains untouched (Jacob's decision).

## Close gate (standing)

The round-32 close must cite all five legs: local
suite + clippy on the close tree (cold), the fuzz
bins as claims (byte-identical), hosted CI
conclusions for macos, windows, and ubuntu on the
close commit, the desktop runner build, and the
Windows bundle result on the close commit via
`gh run view`. After the close: the standing
boundary hygiene (cargo clean + stale probe dirs,
reports/syntheses/fuzz bins at recorded md5s kept,
registry not pruned), disk numbers
before/after/after-cold-rebuild, and one cold
full-suite run. If the round-33 audit then returns
zero BLOCKER + zero MAJOR, the exit bar is met and
the Tier-2 final gate is the next step — not this
loop's to start.

## Close (coordinator, 2026-10-20)

All six batches landed. Each was verified on its
committed tree by the coordinator's own unpiped
suite and clippy runs, with the booklet baseline
(31,717 imposed / 34,977 reader) exact in suite at
every step. Suite count: 940 at the round-31 close,
976 at this close.

- R32-A `d20b5ae` (F1) — 944 tests. Landed under
  the predecessor coordinator (recovered from a
  partial tree).
- R32-B `31c6f90` (F2) — 946 tests. Extended
  differential corpus (66,620 cases) byte-identical
  to the round-30 close `3722169`.
- R32-C `b7d6c06` (F3) — 954 tests. Recovery
  record below.
- R32-D `3cf476f` (F4, F5, F6) — 966 tests.
- R32-E `1afccb3` (F8, F9) — 969 tests. The
  round-31 F6 record stands amended as F9 states,
  recorded in `peer_provably_gone`'s doc.
- R32-F `c0dd8b8` (F7, F10, F11) — 976 tests. The
  close tree. Its F11 pin is the behavioral
  equivalent of the RSS witness (refusal from the
  central facts before any payload is built); a
  true RSS assertion is impractical in-suite, and
  the pin says so.

**R32-C recovery record.** The batch's writer was
reported interrupted, leaving one dirty file. The
successor coordinator inspected the full uncommitted
diff: it was timer instrumentation only
(`Instant` probes and `TIMER` eprintln lines in
three functions of contribution/composition.rs),
no fix content. That one file was reset to HEAD
and a fresh writer dispatched. The fresh writer
stood down with zero edits on detecting a second
writer active in the same tree: the original
writer was in fact still running, and it completed
the batch across composition.rs, grammar.rs, and
tests.rs and committed it as `b7d6c06`. A recovery
writer dispatched against the dirty tree entered
after that commit had landed and was closed once
the commit was established. The batch as committed
was then verified from scratch by the coordinator
(suite 954/0, clippy 0, fuzz byte-identical, the
eight `r32c` pins present and passing). The reset
discarded instrumentation only; no fix content was
lost. The collision is recorded here because the
close must show it: two writers were live in one
tree for part of the batch, against the one-writer
rule, through a false interruption report.

**Timing provenance (F12 applied).** R32-C's
commit message mixes estimators: the un-fixed
figures are the red team's release min-of-5; the
fixed figures (3,987 ms cancelled-with-core at
8,000 groups; 2,617 ms relative-cancelled) are from
a debug probe on a loaded VM. Read the point
figures with that provenance. The shape claim
(bounded growth per doubling in place of the
quadratic) is what the in-suite timing pins carry.

**Fuzz as claims on the closed tree**
(coordinator's own runs, bins at recorded md5s
verdict `a9cac6c3…`, comp `4f85c605…`): verdict
40,000 lines, panics=0; composition 497,024 lines,
D_PANICS=0. Both stdouts **byte-identical to the
round-30 close baselines** (md5 verdict
`4941c4f9…`, comp `efd2096b…`). No date reading
moved anywhere in the round. Coordinator stdouts:
`round32-out/coord/close-verdict.stdout`,
`close-comp.stdout`. One process note: after
R32-D changed workspace dependency requirements,
the probe crate's lock file had to be refreshed
(runs without `--locked`); the bin sources are
untouched at their recorded md5s and the outputs
are unchanged.

**Exit bar: NOT met by round 32's audit verdict
(0 / 3 / 9); all twelve findings are now fixed or
dispositioned (F12 is a record correction; the F6
residual as amended by F9 is recorded, not fixed),
so round 33 audits the closed tree as the next
candidate.** HF-12 remains untouched (Jacob's
decision). The Tier-2 final gate is not started.
The close's hosted evidence (three-OS CI, desktop
runner build, Windows bundle on the close commit)
and the round-boundary hygiene record live in the
audit dir (`round32-close-status.md`), per the
standing gate.

**Standing practice change (Jacob, 2026-10-20
14:53 PDT), recorded at this close.** Storage
discipline at batch boundaries: after a batch
commits and pushes and HEAD == origin/main is
confirmed on its commit, `cargo clean` runs in the
repo (registry not pruned), and the batch's
verification then runs cold from that clean tree
(full unpiped suite + clippy + booklet pin +
fuzz-as-claims where the batch touches the
contribution engines), replacing the incremental
verification for that batch. Per batch, the record
carries disk free before the clean, after the
clean, and after the cold rebuild, plus the cold
run's wall time. Rationale: a round of incremental
batches grows `target/` to 20-25 GB of debris; a
single cold build's footprint is ~2-4 GB; the
pushed GitHub tree is the source of truth being
rebuilt. First application: this round's close
itself, whose cold run already followed the shape
(clean, then cold suite 976/0 and clippy 0 on
`c0dd8b8`; disk 11G free before the clean, 18G
after, 16G after the cold rebuild; cold suite wall
~13.5 min, clippy ~3.7 min, matching the directive's
measured expectations; full numbers in
`round32-close-status.md`). Per-batch cold
verification applies from the next batch onward,
which is round 33's batches under the next
coordinator.
