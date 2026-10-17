# Round 10 audit synthesis (Wonder verify / review / red team over HEAD `c26cdf4`)

Three blind lanes over the round-9 fixes. Verify: **PASS, 34 VERIFIED / 0 FAILED /
0 UNTESTABLE** (637/0 unpiped, clippy exit 0, booklet 31,717 / 34,977 exact, Bach
live). Review: **0 BLOCKER / 0 MAJOR / 4 MINOR**, with the three questions put to
it answered (below). Red team: **0 BLOCKER / 1 MAJOR / 8 MINOR** (its report's
timing figures for one fixture were corrected by a follow-up settling run; the
report file carries the corrected numbers). Deduped total: **0 BLOCKER, 1 MAJOR,
10 MINOR, one census, three questions answered. The exit bar is NOT met** — one
MAJOR stands — so the loop continues into an eleventh round; the Tier-2 final
gate stays queued behind a zero-BLOCKER + zero-MAJOR round.

Also recorded here: the Windows bundle run 39023914623 from `c26cdf4` FAILED
its gate job — the workspace-tests step died on
`helper::tests::a_helper_from_the_future_is_refused_in_words` with "the PDF
reader is here but could not start: Text file busy (os error 26)". That is
finding R10-9 below (the harness race) firing on the CI runner: a test-infra
failure, not a product defect, but it means no round-9 bundle exists. The cure
ships in R10-C; the bundle re-triggers from the round-10 close commit.

## The MAJOR

### R10-1 (MAJOR, red team RT10-T1): walkable candidates sharing one directory multiply the whole walk

Construction: one central directory of 8,191 entries (cap 8,192), one shared
zip64 end record, then N appended candidates, each a locator pointing at the
SAME zip64 record plus a sentinel end record — every candidate walkable, all
resolving to the same large directory. Each candidate pays the full 8,191-record
directory walk in the cap walk, the same walk again in the loader's own
enumeration, plus one trimmed `ZipArchive` open — ≈0.33 s per candidate for
load: 30 candidates 6.0 s; 300 candidates 100.5 s (insert 86.6 s; the
bad-document variant 41.6 s quiet / 97.9 s contended); 2,900 candidates (≈1,562
in the tail window) did not finish in 100 s, projecting roughly 9–10 minutes.
The boundary check holds the other side: 8,193 entries refuse cheaply (53 ms) —
the blowup lives just under the cap, in the product (walkable candidates ×
directory size), which R9-D bounded only per candidate. An honest file cannot
reach this; a received `.obd` is untrusted input, and the round-9 comment flood
was graded MAJOR on the same reasoning at far smaller numbers.

Disposition (R10-A): **a directory is walked and opened at most once per load,
however many candidates point at it.** Candidates are deduplicated by the
directory they resolve to (its offset, size, and zip64 target where present)
before any walk or open; each distinct directory pays the cap walk once and at
most one trimmed open in the load loop. The shared-directory fixtures must land
in the control's time class (the one-candidate control reads in 0.143 s): the
300-candidate shared file loads in well under a second's multiple of the
control, and the 2,900-candidate shared file finishes in seconds, not minutes.
The per-candidate scan itself stays as R9-D built it (the round-9 signature
fixtures and the body-spread signature shapes re-timed flat by both lanes:
0.22–1.26 s across every shape). If dedupe by resolved directory cannot cover a
shape, the batch reports the shape rather than weakening the outcome.

## The answered questions

**Q1 — the R9-A standing-wins order dependence: SOUND (review, grade A−),
independently confirmed by the red team's census.** Of the six fold orders of
[kidsCcBy, kidsCc0, church], five end Cc0; only [church, kidsCc0, kidsCcBy]
ends ChurchOwned (the standing record itself bearer-vouched; precedence never
installs a license). The red team's census over 118,730 multisets (lengths
2–5): 93,634 yield more than one (children, license, credit) triple across
orderings, but **the children value never varies (0 groups)**, and **no flip
crosses between a vouched (true) claim and an unvouched (false) one** — classes
K4/K5/K5b are all zero. The flips are: K2, 84,432 groups (license flips between
two claims each vouched by a bearer in the multiset — the standing-wins class,
accepted); K1, 3,218 (credit text only, between two vouched records); K3, 503
(license flips between a vouched claim and Unknown — the pairwise fence reverts
a license that arrives before its vouching credit, the fold-end judgment cannot
restore it once the credit stands, and the weaker claim wins: the safe
direction; recorded as census context, not a finding, per the lane's grading).
The brute force re-ran clean: 8,308,800 sequences, zero unvouched licenses
standing beside children with empty credit, zero manufactured permissions,
zero assertion orphans. The coordinator's R9-A adjudication stands: two
different licenses cannot both be THE license of one asset; the standing
record keeps its claim, and every candidate claim is vouched.

**Q2 — the ETXTBSY flake: root-caused (review).** Not an OS race and not
parallelism alone: the fake-helper test harnesses write a script file and exec
it immediately; any lag between the write completing and the file becoming
exec-able surfaces as ETXTBSY at spawn. The subprocess-spawn error text is
doing double duty (the same production spawn function serves the tests' fake
helpers and the real helpers), which is why a harness timing issue presents as
a product-looking error. Cure graded MINOR as a defect, high recurring cost —
and it has now cost a bundle (above). See R10-9.

**Q3 — the R9-D trailing-segment reservoir rule: honest, SOUND (review, B+).**
The lane re-derived the rule against shapes beyond the ones that motivated it
and found the stated rule applied exactly; its one MINOR-grade observation (a
mirror asymmetry) joins the red team's two kinds findings as R10-7 below.

## The MINORs

### R10-2 (MINOR, red team RT10-R1): the fold route and the single-merge route pick different attribution licenses

23,823 of 346,176 sequences of length ≤ 4 fold to a different triple than the
single-merge composition of the same records. Every sampled divergence has the
same shape — two attribution licenses plus a credit record: the fold keeps the
first license; the single merge ends with the credit record's license. Both
licenses are vouched; children and credit agree; nothing is manufactured.
Wrong-attribution risk only: the same records arriving by two composition
routes carry different attribution claims into a document.

Disposition (R10-C): **the license that travels with the credit that stands
wins, on both routes.** At final judgment (and at the equivalent point of a
single merge), when the standing attribution license is not the one vouched by
the standing credit but another attribution license in the composition is, the
vouched-by-the-standing-credit license stands. Where no attribution license is
vouched by the standing credit, today's judgment is unchanged. The brute-force
harness gains the route-agreement property: fold and single-merge compositions
of the same records agree on (children, license, credit) for every sequence in
the pool.

### R10-3 (MINOR, red team RT10-L1): a walkable fake shadows the fallback

Honest zip + 70,000 bytes of signature-free trailing junk (the honest end
record pushed out of the tail window) + one appended walkable empty candidate:
the load refuses ("that file has no bulletin inside it") because the fake
opens, `opened_any` is set, and the reader-side fallback never runs; the
control without the fake loads in 97 ms. A variant whose fake directory holds
bad-JSON `document.json` returns the fake's error; the honest document is never
tried. Same family as R9-6, through the fallback gate: R9-D's restated rule
("the reader's own path runs only when no walkable candidate can be opened")
is exactly the hole — opening is not evidence. Crafted files only.

Disposition (R10-A): **the reader's own path runs whenever no candidate YIELDS
a bulletin**, not merely when none opens. A candidate that opens but holds no
readable document.json (or whose document fails) does not close the fallback
gate; the fake-shadow fixtures load the honest bulletin, and when nothing
anywhere yields a document the surfaced error stays the most informative one
(the first candidate's document error where one exists — R9-D's rule — else the
no-bulletin sentence).

### R10-4 (MINOR, red team RT10-D1): the second-date scan only looks after the read date

"Use November 1, not Sunday, October 25, 2026" reads Oct 25, verdict SameSunday
unarmed, no second date noted: the sender's operative date stands BEFORE the
read date, where the R9-13 scan never looks, and the date the verdict blesses
is the one the sender negated. Its twin ("The fair moved from November 1 to
Sunday, October 25, 2026") is indistinguishable to the machinery — there,
following the framed date matches intent — which is why the first misleads
silently. No date is ever armed wrongly; graded MINOR.

Disposition (R10-B): **the second-date scan reads the whole line.** A distinct
absolute date standing anywhere else in the line — before or after the read
date — earns the same note ("These words also name Sunday, November 1, 2026")
and the same arming behavior as today's after-the-date case. The framed-prose
correction that already fires (5d1) is unchanged.

### R10-5 (MINOR, red team RT10-D2): a trailing group containing words poisons the whole line

"October 25, 2026 (correction: November 1)" and "October 25, 2026 (use
November 1 instead)" read NO date at all: the R9-12 re-read triggers only when
a trailing group parses as a bare date; a group with words empties
`trailing_parenthesized_dates`, the re-read never runs, and even the plain
Oct 25 is dropped. The sender's most explicit correction shape produces the
least information.

Disposition (R10-B): **a trailing parenthesized group contributes the date it
contains.** When a trailing group is not itself a bare date, the reader looks
inside it for one absolute date (the same readers the line itself uses); a
group containing exactly one contributes it as a parenthesized date for the
existing composition (compared, mismatched, and named exactly as a bare-date
group is); a group containing none or several contributes nothing, as today.
The two witnesses read Oct 25 with Nov 1 named as the parenthesized mismatch.

### R10-6 (MINOR, review MINOR-1 + red team RT10-D3): stacked-group accounts outside the Sunday-after shape

Two faces of one gap. (a) The Keep-through stacked-garnish sentence misdescribes
where the governing date came from when the kept date arrives through garnish
composition. (b) Outside the Sunday-after shape, only one disagreeing date is
ever named: Keep-through relative + (Nov 1)(Nov 8)(Nov 15) keeps Nov 15 and
names Nov 1 — Nov 8 appears nowhere; the same single-naming holds on absolute
Keep-through lines with two groups, on absolute For Sunday lines, on the bare
relative shape, and in the second-date note (two second dates: only the first
is named). The one named date is always genuinely disagreeing, and where a
middle group agrees with the standing date the accounts already surface every
distinct date — the Sunday-after sentence does this correctly today.

Disposition (R10-B): **every distinct disagreeing date is named, on every
shape, and the governing date's source is described as it actually was.** The
Sunday-after listing discipline (every group named, the bulletin named)
extends to the Keep-through mismatch sentence, the absolute-line mismatch, the
bare shape (which gains the listing sentence), and the second-date note (every
distinct second date named). The Keep-through sentence states where the kept
date came from in the case at hand (the line's own words vs a parenthesized
group) rather than one fixed account.

### R10-7 (MINOR, review MINOR-2): `week_verdict` can return DifferentSunday carrying the bulletin's own date

Under stacked-parenthesis composition the verdict can name, as the "different
Sunday", the very date of the bulletin the words were checked against — a
verdict whose payload contradicts its name. No arming is involved; the review
surface would print a confusing sentence.

Disposition (R10-B): **a DifferentSunday verdict never carries the open
bulletin's date.** Where composition would produce it, the outcome is the
SameSunday-with-note composition (the standing date is the bulletin's; the
disagreeing garnish is named in the note), matching the R9-C rule that a
bulletin-equal parenthesis is agreement evidence. A pin asserts the contract
over the stacked shapes wholesale: for every witness, DifferentSunday(d)
implies d ≠ the bulletin's date.

### R10-8 (MINOR, red team RT10-K1 + RT10-K2 + review's Q3 observation): kinds sentences assert what the pairing leaves open, and the trailing mirrors disagree

(a) Middle segments: surplus in one segment, its only possible partner in a
MIDDLE removal-only segment — kinds {renamed, removed, added}, moved = false,
though a pure crossed-rename reading exists. The sentence asserts an addition
and a removal as fact and denies a move the pairing never ruled out. (b)
Trailing mirrors: deficit + trailing addition-only reads {renamed, removed,
added}, moved = false, while its mirror (surplus + trailing removal-only)
reads {renamed, moved} with added/removed denied — same geometry, opposite
accounts; one template must be the honest one for both. (The review lane's Q3
observation is the (b) mirror.) The refusal to auto-apply is correct in every
shape; only the accounts overstate. Every shape the stated rule names reads
exactly as designed (both lanes re-derived them).

Disposition (R10-C): **the kinds and the sentence claim only what the pairing
establishes, and mirror geometries read identically.** A removal-only segment
whose names a standing surplus could absorb (by count, anywhere in the list —
not only trailing) is not asserted as a plain removal: either `moved` joins
the kinds for that shape or the sentence names the ambiguity in plain words
("some of these lines may have been renamed from another section" — the
builder chooses the mechanism that keeps every currently-certain shape reading
exactly as today). RT9-14's certain shapes (a standing removal no surplus can
absorb stays a removal; a surplus past its partners' need leaves removals) are
unchanged. The two trailing mirrors produce the same kinds and the same
sentence shape.

### R10-9 (MINOR as a defect, review MINOR-4 / Q2; high recurring cost): the fake-helper harness write→exec race

Root cause at Q2 above. It fired in the coordinator's R9-C and R9-D
verification runs, twice in the R9-D builder's runs, and on the CI runner,
where it failed the round-9 bundle's gate job (run 39023914623).

Disposition (R10-C): **test-harness-only bounded retry.** The fake-helper /
converter test harnesses retry the spawn-and-handshake a small bounded number
of times when the failure is ETXTBSY (text file busy) on a helper the test
itself just wrote. Production helper-spawn code does NOT gain a silent retry —
a real helper that cannot start on a user's machine must still say so at once,
in words. A pin or harness comment states the boundary: retry lives in test
support only.

### R10-10 (MINOR, red team RT10-D4): a group that is not trailing is invisible — and can take the whole line with it

"next Sunday (October 25) please" (one word after the group) reads nothing;
"(October 25) next Sunday" reads nothing; "October 25 (November 1), 2026"
reads nothing — the middle group poisons the plain date; and a paren-only For
Sunday line "(October 25)" reads NoSundayNamed against every bulletin while
the identical paren-only Keep-through line reads its date fine — an asymmetry
between the two date lines for the same sender shape.

Disposition (R10-B): (a) **the For Sunday line reads a paren-only date exactly
as Keep-through does** — "(October 25)" is a Framed outcome against the
bulletin, composed by the existing rules. (b) **a parenthesized group standing
between a month-day and its year does not poison the plain date**: "October 25
(November 1), 2026" reads October 25, 2026 with the Nov 1 group compared as
garnish. (c) A group standing before the phrase or followed by other words
stays unread, as today — the safer direction — and the synthesis records that
boundary as accepted: the readers compose a date with its garnish where the
garnish touches the date or the line is the garnish; free-floating groups
mid-sentence remain the sender's prose, not a date claim.

### R10-11 (MINOR, review MINOR-3, docs): FORMAT.md does not disclose the standing-wins order dependence

The R9-A judgment is order-dependent by adjudicated design (Q1: SOUND), but
FORMAT.md's account of the composed record does not say so; a reader of the
format spec would expect composition to be order-free.

Disposition (R10-C): FORMAT.md states it plainly where the record composition
is described: when two records in one composition each carry a vouched claim
to be the record's license, the claim that stands is the one already standing
when they meet; every claim the composition can produce is vouched, and the
children's-photos answer never depends on order. One or two sentences, in the
spec's voice.

## Fix batches

- **R10-A (counts/loader):** R10-1 (dedupe directories across candidates) +
  R10-3 (the fallback gate opens on "no candidate yields a bulletin").
- **R10-B (dates accounts):** R10-4 (whole-line second-date scan) + R10-5
  (wordy trailing groups contribute their date) + R10-6 (every disagreeing
  date named; governing source described truly) + R10-7 (DifferentSunday never
  carries the bulletin's date) + R10-10 (paren-only For Sunday parity;
  mid-date group does not poison).
- **R10-C (rights routes, serving kinds, docs, harness):** R10-2 (the license
  vouched by the standing credit wins on both routes; route-agreement in the
  brute force) + R10-8 (kinds claim only what the pairing establishes; mirrors
  agree) + R10-9 (harness-only ETXTBSY retry) + R10-11 (FORMAT.md disclosure).

Each batch appends its dated Shipped addendum here with real numbers. Round 11
audits the fixes; the exit bar is unchanged. The Tier-2 final gate runs only
after a round returns zero BLOCKER + zero MAJOR.

## Shipped addendum, R10-A

Shipped 2026-10-17 on top of `039cfc0`. Both findings are fixed in
`crates/ob-format/src/lib.rs`; FORMAT.md's loading-rules bullet now states both
rules. Suite: **640 passed / 0 failed** unpiped (baseline 637 + 3 new pins, all
in ob-format, 40 → 43 there), clippy exit 0 with no warnings. No pre-existing
test was changed or removed; the R9-D first-error pins
(`r9_a_file_whose_every_candidate_is_junk_refuses_with_the_first_error` and its
sheet-door twin) pass unchanged and now also exercise the widened fallback.
Timings below are debug builds on this machine from a probe driving
`Document::load` / `load_insert`; the red team's contended-machine figures are
cited where this lane did not re-measure.

**R10-1 — a directory is walked and opened at most once per load.**
`directory_candidates` now keys each candidate by the directory it resolves to
(resolved start, directory end, zip64 record position where one stands, and
the declared total only when no start resolved) and walks, counts, and returns
each distinct directory once, so the load loops open it at most once; the
fallback enumeration likewise skips candidates already tried and repeats of a
directory description already tried (by an O(1) declared-fields key — total,
size, offset, zip64 target — read straight off each candidate), and opens no
candidate whose directory does not walk (`fallback_walkable`, the count walk's
resolution over the file's bytes with bounded searches). Measured, load door:

| Fixture | Before | After |
|---|---|---|
| Control, same 8,191-entry directory, one plain end record | ERR (no bulletin) 0.091 s | ERR (no bulletin) 0.084 s |
| 30 shared candidates | ERR 2.55 s | ERR 0.091 s (≈1.0× control) |
| 300 shared candidates | ERR 35.3 s (red team: 100.5 s) | ERR 0.128 s (≈1.5× control) |
| 300 shared, directory's document fails | ERR "Invalid checksum" 28.2 s (red team: 41.6 s quiet / 97.9 s contended) | ERR "Invalid checksum" 0.151 s |
| 2,900 shared candidates (≈1,562 in the tail window) | did not finish in 100 s on the red team's lane (projected ~9–10 min); not run to completion here | ERR 4.05 s (sheet door 4.12 s) |

The 2,900 residual is one trimmed archive open, paid once per load: the zip
crate's own open cost grows with the candidate records standing in the trimmed
stream even when the chosen candidate is the valid one. It is a constant per
load now, not a per-candidate product. Pins:
`r10_candidates_sharing_one_directory_pay_for_it_once` asserts the enumeration
returns exactly one candidate for the shared directory at 30 / 300 / 2,900, the
control's refusal wording (and the sheet door's, at 300), the bad-document
variant's first-candidate error ("Invalid checksum"), and a timing budget of
20× the in-test control + 10 s at each size.

**R10-3 — the reader's own path runs whenever no candidate YIELDS a bulletin.**
Both loaders dropped the `opened_any` gate (and the early first-error return in
front of it): a candidate that opens but holds no readable document, or whose
document fails, no longer closes the fallback. The fallback itself is gated so
the widened rule cannot storm: the whole-file reader attempt runs only when
there is no candidate at all, when its chosen (latest) candidate is new and
walks, or when nothing above could even be opened and at most 64 candidates
stand; the enumerated candidates open only when their directory walks. That
gate matters — the first cut of this fix, with the fallback simply widened,
took 7.5 s / 35 s / 66 s at the sheet door over the round-9 files carrying
500 / 1,500 / 2,900 comment signatures (the reader retries every fake end
record whose directory fails), where the shipped gate takes 0.098 s / 0.133 s /
0.121 s with the identical refusal. Measured:

| Fixture | Before | After |
|---|---|---|
| Honest zip + 70,000 B junk + walkable empty fake | ERR "no bulletin inside it" (11.5 ms) | loads the honest bulletin (7.3 ms); sheet door refuses in 3.8 ms |
| Same, fake's directory holds bad-JSON document.json | ERR, the fake's JSON error (32 ms) | loads the honest bulletin (12.9 ms) |
| Control, same junk, no fake | loads (5.1 ms) | loads |

Pins: `r10_a_walkable_fake_candidate_does_not_shadow_the_fallback` (empty fake
and bad-document fake, bulletin and sheet doors, with the no-fake control) and
`r10_the_fallback_does_not_storm_over_comment_signatures` (2,900 comment fakes
at both doors refuse in the plain words inside a 20 s budget; shipped ~0.2 s).

**Round-9 shapes re-measured, outcomes unchanged.** Timing-signature fixtures
(load / insert): 500 — 0.198 s / 0.075 s before, 0.140 s / 0.098 s after;
1,500 — 0.168 / 0.075 before, 0.339 / 0.133 after; 2,900 — 0.189 / 0.087
before, 0.187 / 0.121 after. Body-signature fixtures (no comment / plain
comment / signature comment, load / insert): 0.515 s / 0.344 s, 0.619 / 0.273,
0.814 / 0.326 before; 0.604 / 0.372, 0.461 / 0.475, 0.543 / 0.413 after. All
load or refuse exactly as before. The mid-record declared-offset fixture keeps
its refusal wording verbatim ("that file's container is damaged: invalid Zip
archive: Could not find EOCD") at both doors — the widened reader gate's
nothing-opened clause exists to keep it. The honest zip64 file, the
prepend shapes, the size-lie, and the into-another-directory candidate all
keep their round-10 outcomes. Also shipped in passing: `TrimmedReader` no
longer seeks before a read that starts where the file's cursor already stands
(the archive reader's many small sequential reads each paid a seek).

## Shipped addendum, R10-B

Shipped 2026-10-17 on top of `73f7362` (R10-A). All five findings are fixed in
`crates/ob-extract/src/contribution.rs`, with the review window's second-date
notes in `crates/ob-app/src/main.rs` switched to the plural forms. Suite:
**645 passed / 0 failed** unpiped (baseline 640 + 5 new pins, all in
ob-extract's contribution tests, 36 → 41 there; ob-extract 156 → 161),
clippy exit 0 with no warnings. The fake-helper ETXTBSY flake did not fire
in this batch's runs. One pre-existing test was changed, flagged below.

**R10-4 — the second-date scan reads the whole line.** `second_dates_in_line`
(replacing `second_date_in_line`) scans every date-shaped run in the line,
before the read date as well as after it; the read date itself — or, on a
Sunday-after line, its base — is the anchor and never a second date, and
every other distinct date is collected in written order into the new
`for_sunday_second_dates` / `keep_through_second_dates` lists (the round-9
single fields stay, as the first entry). "Use November 1, not Sunday,
October 25, 2026" now notes November 1 with the verdict exactly as the
after-the-date case: SameSunday, unarmed, the framed date governing. The
framed-prose correction that already fired (5d1) is unchanged, sentence
included.

**R10-5 — a trailing group contributes the date it contains.** A shared
`group_date` reader serves both trailing-group walks: a bare-date group
contributes its date as before; a group with words contributes the one
absolute date found inside it by the line's own prefix reader, and a group
containing none or several contributes nothing, as today — the no-date and
two-date witnesses keep their existing verdicts (no date read) verbatim.
"October 25, 2026 (correction: November 1)" and "(use November 1 instead)"
read October 25 with November 1 recorded as the parenthesized mismatch,
named exactly as a bare-date group is, and not double-counted as a second
date.

**R10-6 — every distinct disagreeing date is named, and the governing
date's source is described as it actually was.** `ParenthesizedDateMismatch`
now carries every distinct disagreeing group date (`named` plus
`also_named`, written order) and a `read_from_parentheses` flag. The
mismatch sentences list all of them on the absolute For Sunday and
Keep-through shapes; when the kept date itself came from a parenthesized
group (stacked groups on a relative Keep-through line, or a paren-only
line), the Keep-through sentence states that instead of framing a plain
reading against a parenthesis: "The dates in parentheses do not agree with
each other: … This part is read as keep through …". The bare relative shape
gains the Sunday-after listing sentence (every group named, the bulletin
named) via `parenthesized_different_sentence`, and the second-date notes
gain plural forms (`second_dates_sentence`,
`keep_through_second_dates_sentence`) naming every distinct second date;
the single-date sentences are byte-for-byte as before. Witnesses pinned:
Keep-through relative with three groups keeps November 15 and names
November 1 and November 8; absolute lines with two groups name both, in
either order; a middle group agreeing with the standing date hides
nothing.

**R10-7 — a DifferentSunday verdict never carries the bulletin's date.**
In the Sunday-after stacked arm of `week_verdict`, where the first group
disagreeing with the resolution IS the open bulletin's date, the outcome
is now SameSunday, and `parenthesized_same_sunday_sentence` extends to
that shape: it names the bulletin date as the parenthesized standing date,
the relative resolution, and every group, the disagreeing garnish
included. A wholesale pin asserts the contract over ten stacked witness
shapes in both group orders against both bulletins: for every witness,
DifferentSunday(d) implies d ≠ the bulletin's date.

**R10-10 — paren-only For Sunday parity, and a mid-date group does not
poison.** (a) `date_line` reads a line that is only parenthesized groups
exactly as Keep-through has since round 9: the last group stands, an
earlier disagreeing group is recorded against it with its source stated
as the parentheses — "(October 25)" is SameSunday against the October 25
bulletin, DifferentSunday(October 25) against November 1. (b) A group
standing between a month-day and its year (pinned by the comma-and-year
that follows it, and by the line reading as a date once the group is
taken out) is compared as garnish: "October 25 (November 1), 2026" reads
October 25, 2026 with November 1 as the mismatch, on both lines, and the
garnish is not double-counted as a second date. (c) The accepted boundary
stands, pinned in the new test: "next Sunday (October 25) please" and
"(October 25) next Sunday" still read nothing.

**Flagged pre-existing test change (1).**
`r9c_stacked_parentheses_are_all_compared`: only its Sunday-after mirror
block changed. It expected `DifferentSunday(October 25)` against the
October 25 bulletin for "the Sunday after next Sunday (October 25)
(November 1)" — the exact R10-7 contract violation (a DifferentSunday
carrying the bulletin's own date). It now expects SameSunday, and its
sentence assertion moved from `parenthesized_different_sentence` to the
extended `parenthesized_same_sunday_sentence`, whose text names the
standing date, the resolution, and both groups. The change and its reason
are also recorded in a comment in the test. Every other assertion in that
test, and every other pre-existing test in the suite, stands in its
original expectation; all other sentence pins in this family (single
mismatch, single second date, plain-read Keep-through provenance) are
unchanged.

## Shipped addendum, R10-C

Shipped 2026-10-17 on top of `22e42fb` (R10-B). R10-2 is fixed in
`crates/ob-model/src/lib.rs` (one new judgment step,
`credit_paired_attribution_license`, called first by the one final
judgment both routes share), R10-8 in the same file's
`serving_pairing`, R10-9 in the three test harnesses that write and
then exec a helper script (`crates/ob-extract/src/helper.rs`,
`crates/ob-music/src/render.rs`, `crates/ob-clips/src/lib.rs` — test
modules only), and R10-11 in `docs/FORMAT.md`. Suite: **651 passed /
0 failed** unpiped (baseline 645 + 6 new pins: ob-model 105 → 107,
ob-extract +1, ob-music +1, ob-clips +1, ob-app +1), clippy exit 0
with no warnings. The ETXTBSY flake did not fire in this batch's
runs. Four pre-existing tests were changed, all flagged below, plus
harness plumbing in the fake-helper tests whose expectations are
unchanged.

**R10-2 — the license that travels with the credit that stands
wins, on both routes.** `judged_composed_license` now opens with the
credit-paired step: when the standing license is not itself held
together with the standing credit by any record in the composition,
but another attribution license is, that license stands. Where no
attribution license is vouched by the standing credit, the judgment
is exactly as before. Measured on the round-10 lane's own census
harness (baseline run on the pre-batch tree reproduces the lane's
published numbers exactly; the repo-side sweep in the new pin
reproduces the post-fix totals independently):

| Measure (lengths ≤ 4 unless noted) | Before | After |
|---|---|---|
| Fold vs single-merge triple divergences | 23,823 / 346,176 | 16,553 / 346,176 |
| R10-2 rule violations at either route's judgment point (repo sweep, lengths 2–4) | — | **0 / 346,176** |
| Paired family (8 archetypes, lengths 2–4): full triple agreement | — | **4,672 / 4,672** |
| Length-2 sequences, full pool: full triple agreement | — | **576 / 576** |
| Fold invariants over 8,308,800 sequences (lengths 2–5): unvouched license beside children + empty credit / manufactured credit / manufactured license / orphan assertions | 0 / 0 / 0 / 0 | 0 / 0 / 0 / 0 |
| Permutation flip groups (lengths 2–5) | 93,634 | 78,137 |
| Children-varying flip groups | 0 | 0 |

Census classes (the lane harness's classification over all groups):
two-vouched flips 84,523 → 67,290; true/false crossings 3,864 →
1,105; vouched-vs-Unknown 503 → 503 (the report's K3, unchanged);
unvouched-license flips 1,470 → 1,470; credit-only flips 3,274 →
7,769. The credit-only growth is reclassification, not new
order-dependence: the set of standing credits across a group's
permutations is untouched by this fix (no credit fill or fence
changed), so every group counted there already flipped at baseline
on license, credit, or both — curing its license flip leaves its
pre-existing credit flip standing alone. No previously
order-invariant group flips: the flip total fell by 15,497.

The residual 16,553 divergences all stand where the disposition's
own rule does not reach: no attribution license is vouched by the
standing credit on the disagreeing route. They are the R8-9
deferral shapes — a credit arriving after the license's pairwise
judgment, traveling with no attribution license, where fold and
single-merge outcomes are pinned to differ by
`r8_fold_judges_the_composed_rule_once_at_the_end` — and the
fence-timing credit shapes of the same family. **Deviation,
stated plainly:** the disposition's hope that the harness assert
agreement "for every sequence in the pool" is met for the paired
family and at length 2 pool-wide, and the rule itself is asserted
at every judgment point pool-wide (lengths 2–3 in the shipped pin,
2–4 in this lane's sweep); pool-wide triple agreement over
arbitrary sequences is precluded by the standing R8-9 pin, which
this batch does not change. Shipped pin:
`r10_fold_and_single_merge_routes_agree_over_the_pool` (the lane's
24 archetypes; rule conclusion on both routes plus the round-9
invariants over lengths 2–3, full agreement over the paired
family at lengths 2–4 and over the full pool at length 2, and the
named two-licenses-plus-credit witness in both orders).

**R10-8 — kinds claim only what the pairing establishes; mirrors
read identically.** The partner reservoir in `serving_pairing` is
now counted by name anywhere in the list, in both directions: a
surplus left after the crossing answers removal-only names
wherever they stand (not only in the trailing segment), and a
deficit left after the crossing answers addition-only names the
same way. `removed` is asserted only for removal-only names past
every partner's need or a deficit still standing after the
reservoir; `added` only for addition-only names past a deficit's
need or a surplus still standing; either reservoir use joins
`moved`. The middle-segment witness and both trailing mirrors now
read the crossed kinds {renamed, moved}, and in the app all three
read the canonical crossed-renames sentence word for word. The
certain shapes read verbatim as before, all pinned in the new
`r10_kinds_claim_only_what_the_pairing_establishes`: surplus 1 or
2 against three trailing old names ({renamed, removed, moved} —
one or two partners used, the rest removals), a first-segment
surplus with a trailing partner plus an additional bounded removal
({renamed, removed, moved}), and a standing removal no surplus can
absorb (the D8 shape, {renamed, removed}, no move). The refusal to
auto-apply is untouched. The lane's probe shapes re-read on the
shipped code: surplus + middle removal-only → {renamed, moved};
deficit + middle addition-only → {renamed, moved}; both trailing
mirrors → {renamed, moved}; surplus 1 + trailing ×3 and surplus 2
+ trailing ×3 → {renamed, removed, moved}.

**Flagged pre-existing test changes (R10-8, four).**
(1) `r9_kinds_come_from_each_segments_own_outcome` (ob-model):
three assertions changed. The review lane's K1 shape (deficit plus
a trailing addition-only segment) expected {renamed, removed,
added, moved: false} and now expects {renamed, moved} — it is the
trailing mirror of the canonical crossed renames, the R10-8 (b)
witness itself. The RT9-14 witness (surplus plus a removal-only
segment between anchors) expected {renamed, removed, added,
moved: false} and now expects {renamed, moved} — the R10-8 (a)
witness: a standing surplus can absorb that segment's name by
count, so the removal is no longer asserted as plain. The same
shape plus a trailing addition now expects {renamed, added,
moved} (the surplus is spent on the bounded segment's name; the
trailing addition still stands). Reasons are recorded in comments
at each assertion. (2)
`ambiguous_kinds_count_only_segments_the_pairing_could_not_resolve`
(ob-model): the S-C assertion changed from {renamed, removed,
added, moved: false} to {renamed, removed, moved} — S-C's deficit
segment's unpaired name can answer its trailing addition across
the anchor (the mirror of (1)'s middle witness); its clean
removal, which no surplus can absorb, still stands. (3)
`the_ambiguous_refusal_sentence_for_resolved_segments_names_no_move`
(ob-app): the S-C sentence changed from "renamed, removed and
added … Rename, remove or add one line at a time" to "renamed,
removed and moved … Rename or remove one line at a time",
following (2); the test's D8 half is unchanged and still names no
move. (4) The four fake-helper tests in `helper.rs`, the three in
`render.rs`, and the prompt-converter test in ob-clips had their
call sites routed through the R10-9 harness retries; no
expectation in any of them changed.

**R10-9 — test-harness-only bounded ETXTBSY retry.** Each of the
three write-then-exec harnesses gained a test-support retry: up to
6 attempts, 10 ms apart, only on the busy failure (ob-extract:
the error text contains "Text file busy"; ob-music: the run
reports the just-written helper unavailable; ob-clips: a
just-written exit-0 converter reporting failure). Production
spawn paths gained NO retry, verified at the diff level:
`crates/ob-extract/src/runner.rs` is untouched (zero diff), and
every R10-9 hunk in the three harness files sits inside its
`#[cfg(test)]` module; the boundary is also stated in each retry
helper's comment. Pins (the busy window is a race and cannot be
scheduled, so the retries are pinned on the error values
directly): `the_fake_helper_retry_answers_only_a_busy_spawn`
(busy twice then an answer stands on the third attempt; busy
forever fails after exactly 6 attempts; any other failure is
returned on the first attempt),
`the_fake_render_retry_answers_only_an_unavailable_spawn` (same
shape), and `the_fresh_converter_retry_is_bounded` (a just-written
exit-0 script answers true through the retry; a missing program
answers false and the attempts end).

**R10-11 — FORMAT.md discloses the standing-wins order
dependence.** The merge account in `docs/FORMAT.md` now states it
where record composition is described: when two records in one
composition each carry a vouched claim to be the record's license,
the claim that stands is the one already standing when they meet;
every claim a composition can produce is vouched; and the
children's-photos answer never depends on order. The same
paragraph also gained one sentence for R10-2: where the credit
line that stands arrived with a different attribution license
from the one standing, the license that traveled with that credit
is the one that stands, on the fold route and on a single merge
alike.

## Coordinator close (2026-10-17)

All three batches shipped and were independently re-verified by the
coordinator (full unpiped suite + clippy at each step): R10-A `73f7362` (640
tests; shared-300 35.3 s → 0.128 s, shared-2,900 unfinished → 4.05 s, fake
shadow loads; zero changed pins; the storm-gated fallback accepted as
disclosed), R10-B `22e42fb` (645 tests; one flagged pin change — the pin had
asserted the R10-7 violation itself), R10-C `c45f287` (651 tests; four flagged
pin changes, all R10-8 witness assertions and the S-C sentence).

One builder deviation is adjudicated here. R10-C's disposition hoped for
pool-wide fold/single-merge triple agreement; what shipped is agreement for
the credit-paired family (4,672 / 4,672) and at length 2 pool-wide, with
divergences down 23,823 → 16,553 of 346,176 and every safety invariant still
zero. The residual sits entirely in the deferral class pinned by the standing
R8-9 pin (`r8_fold_judges_the_composed_rule_once_at_the_end`), which pins the
two routes to differ where a credit arrives after a license's pairwise
judgment; the disposition's own "today's judgment is unchanged" clause governs
that class, and the batch did not touch the pin. **Accepted**: the residual is
bounded, disclosed, manufactures nothing, and never varies the children value.
Round 11's review lane is asked the standing question explicitly: is the
deferral-class route divergence acceptable as adjudicated, or should single
merges gain the same end-state judgment the fold has (and if so, at what cost
to the R8-9 pin)?
