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

_(appended by the R10-A builder with real numbers and flagged pin changes)_

## Shipped addendum, R10-B

_(appended by the R10-B builder with real numbers and flagged pin changes)_

## Shipped addendum, R10-C

_(appended by the R10-C builder with real numbers and flagged pin changes)_
