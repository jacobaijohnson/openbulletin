# Round 15 audit synthesis — 2026-10-17

HEAD audited: `082359e` (round-14 close). Lanes: verify `652322f7`
(report `~/workspace/openbulletin-audit-wave3/round15-verify.md`),
review `e00b18f0`, red team `645acb47`. Verify: **PASS, 8 of 8
claims**, CI for the close push green on all three OS jobs (run
39069321657), bundle 39069321664 SUCCESS, booklet bytes exact, Bach
live passing. Review **0 BLOCKER / 2 MAJOR / 3 MINOR**; red team
**0 BLOCKER / 3 MAJOR / 3 MINOR**. **The exit bar is not met.**
Deduped: **0 BLOCKER / 3 MAJOR / 5 MINOR**. The anchor (Q3) earned a
clean PASS with no finding; the remaining breaks are all in the
governor machinery's completeness.

## Findings and dispositions

**R15-1 (MAJOR; review R15-1 + RT15-1).** The remaining verb family
contributes at full strength, before and trailing, on all paths:
called off / call off, put back, push back / pushed back,
reschedule / rescheduled, move / moved, shift / shifted, bump /
bumped. The lanes verified two-date move shapes are safe (both
dates noted as alternatives), so the break is the single-date forms
asserting the vacated date. Review's recommended rule, adopted:
govern them all — a moved date with no new date named must not
assert, and nothing named is lost by silencing; `to`-target shapes
("(rescheduled to November 8)") assert their target, pinned.

**R15-2 (MAJOR; review R15-2 + RT15-2).** The stem rule misses its
own fourth form: every -s form (cancels, postpones, removes,
delays, drops, scratches, omits, disregards, forgets — including
natural subject shapes "(choir cancels November 1)") and forget's
irregular past (forgot, forgotten, "forgot about") contribute on
all paths. Disposition: the stem rule gains the -s form and the
irregular pasts of its stems (forgot/forgotten for forget).

**R15-3 (MAJOR; RT15-3).** Stacked cancellations kill the
replacement date: "(November 1, postponed, drop it, November 8)"
silently kills BOTH dates — the spent exception covers only the
clause immediately after a date, so the second phrase reads
unspent and governs November 8, leaving no contribution, no
alternatives, no note. Disposition: the spent-trailing exception
covers a RUN of consecutive cancellation-only clauses between the
two dates; the whole run spends on the first date and the second
asserts.

**R15-4 (MINOR; review R15-4).** The mtime anchor's preserved-
timestamp flow (zip/rsync/sync copies keeping an old mtime) can
produce a worse read than the today anchor — an unnamed Sunday
(Sep 27–Oct 18 for the witness). Bounded by the lanes' own
account: framed-first reads never move, the explicit date always
survives in the note, future/invalid already clamp. Disposition: a
plausibility bound — the file date anchors only when it is at most
28 days before today (and not after today); older falls back to
today. The R14-6 harm (a file one to two weeks late) stays fixed.

**R15-5 (MINOR; review).** The same-week identity pin exercises
only anchor == today; the review lane's probe verified different-
day same-week anchors (Oct 12–15) produce zero diffs. Disposition:
extend the pin to a different-day same-week anchor (test-only).

**RT15-5 (MINOR).** The filler reach-back stops at "of":
"(ignore, the date of, November 1)" contributes. Disposition: "of"
joins the filler word set (a clause of pure filler incl. "of"
carries no content).

**R15-6 (MINOR; RT15-6).** Two trailing-scope defects. (a) A phrase
after the second date silently drops it: "(November 1, November 8,
drop it)" stands on Nov 1 with Nov 8 unrecorded anywhere.
Disposition: the trailing phrase governs the date it follows, and
that date is still named on the second-date note like any other
distinct date the line named. (b) `governed_after` is clause-blind:
"(November 1, choir practice, ignore)" disqualifies Nov 1 — a
cancellation word behind a content clause still reaches it.
Disposition: a trailing cancellation governs the preceding date
only when every clause between the date and the phrase is itself a
cancellation or filler clause (one rule with R15-3's run).

**RT15-4 (no code change).** Innocent-phrase suppression widened
again (drop-off, forget-me-nots, drop-in, no-bake, never-ending,
scratch pads, cancel culture, "(weather delay, November 1)").
Suppression-only, the R13-6/RT14-6 accepted class; named here in
full because these are weekly church phrases. The fence stands: a
governor cannot distinguish a label from a verb without syntax the
app does not have, and the failure direction loses a date from a
parenthetical, never arms one. Reopens only on harm beyond that.

## Batches

- **R15-A** (governors): R15-1, R15-2, R15-3, RT15-5, R15-6.
- **R15-B** (anchor bound + pin coverage): R15-4, R15-5.

Round 16 audits the fixes. The Tier-2 final gate runs only after a
round returns zero BLOCKER + zero MAJOR.

## Shipped addendum, R15-A — 2026-10-17

Batch R15-A (governors) shipped in `crates/ob-extract/src/contribution.rs`
only: **693 → 698 tests passing, 0 failed** (full workspace suite run
unpiped on the final tree, cargo exit code 0 read from a file; run
twice, same totals), **clippy clean** (`--workspace --all-targets
-- -D warnings`, exit 0). Five new pins, all `r15a_*`. **No
pre-existing test was changed** — every round 11–15 held shape stands
at its shipped expectation, including the r12a held care
"(it was moved, November 1)" (see the judgment call below). Neither
the fake-helper ETXTBSY flake nor timing contention fired in either
full run.

What landed, per disposition:

- **R15-1.** The displacement stems `move`, `shift`, `bump`,
  `reschedule` joined the cancellation stem set, so every stem form
  governs before and trailing, on all consuming paths. The phrasal
  pairs are form-aware through one helper (`is_phrasal_governor_pair`):
  "call off", "put back", "push back", "put off", "forget about" in
  any inflection of their first word. Split forms govern as "put …
  off" has since round 14: "(called November 1 off)", "(put
  November 1 back)", "(push November 1 back)". **To-target rule:**
  in `governed_before`, a displacement phrase standing directly
  before a "to" is skipped — it governs the FROM date (through
  `governed_after` when one stands before it), and the target
  asserts. Pinned both ways: "(rescheduled to November 8)" /
  "(moved to November 8)" assert November 8 with no from-date;
  "(move November 1 to November 8)", "(November 1, moved to
  November 8)", "(reschedule …)", "(bump …)" assert November 8 with
  November 1 governed.
- **R15-2.** `is_verb_form` gained the -s form (bare -s and the
  sibilant -es: cancels, postpones, scratches, pushes) and forget's
  irregular pasts (forgot, forgotten — so "forgot about" governs
  through the pair rule). Subject shapes pinned: "(choir cancels
  November 1)", "(Sue postpones November 1)", "(we forgot
  November 1)", plus trailing "(November 1, cancels)" /
  "(November 1, forgot)" and the two-date corrections
  "(cancels November 1, use November 8)" / "(forgot November 1,
  use November 8)", which assert November 8 again.
- **R15-3.** The reach-back walk in `governor_window_start` never
  extends a later date's window into a cancellation-only clause
  standing after a previous date; filler clauses are crossed as
  before, so a RUN of consecutive cancellation-only clauses (with
  filler between phrases) spends together on the first date. All
  seven lane constructions pinned on the relative, paren-only, and
  Keep-through paths, plus the "scratch that" second-phrase
  variant: each asserts November 8.
- **RT15-5.** "of" joined the filler word set in the one clause
  test (`is_filler_clause`). Rule for the second witness, stated
  plainly: **a clause headed by "date of" — "(ignore, the date of
  the service, November 1)" — is a genitive fragment of the date
  phrase itself; it names the date by naming the occasion the date
  belongs to and asserts no event of its own, so the reach-back
  crosses it like any filler clause.** The content-clause control
  "(ignore, the date, choir, November 1)" still contributes, pinned.
- **R15-6.** One clause-scope rule, in `governed_after`: a trailing
  phrase governs the date it follows only when every clause
  between that date and the phrase is itself a cancellation or
  filler clause. (a) "(November 1, choir practice, ignore)" now
  contributes November 1 on all paths; the in-scope twins
  "(November 1, ignore)" and "(November 1, the date, ignore)" still
  govern. (b) A date disqualified only by a trailing phrase after
  the group's last date is governed but NAMED: beside a standing
  date it joins the alternatives the second-date note prints, on
  the paren-only, relative, and Keep-through paths alike —
  "(November 1, November 8, drop it)" asserts November 1 and names
  November 8; the mirror "(November 8, November 1, postponed, drop
  it)" asserts November 8 and names November 1. A lone governed
  date keeps the standing silence ("(November 1, cancelled)"
  names nothing anywhere).

Reading changes to lane witnesses, flagged as directed: the
two-date move shapes the lanes verified as noting both dates as
alternatives ("(move November 1 to November 8)" and twins) now
**assert the target date**, exactly as the disposition orders and
as the held "(moved from November 1 to November 8)" always did;
likewise "(November 1, called off, November 8)" and its
rescheduled / moved twins now assert November 8 like
"(November 1, cancelled, November 8)", instead of offering both
dates as alternatives. No other witness reading changed.

Judgment calls, stated plainly:

1. **Auxiliary-spent displacement.** A displacement verb standing
   directly after an auxiliary ("it was moved") is that clause's
   own predicate, spent on its subject, so it does not reach
   across a clause break as an unspent governor. Without this, the
   r12a held pin "(it was moved, November 1)" — which names its
   date — would have flipped when "moved" joined the stem set.
   Scoped to the displacement stems only.
2. **The -s noun/verb identity.** Adding the -s form means a
   plural noun spelled as a verb's -s form ("drops" standing
   alone as its clause) now governs, the same verb-form identity
   the review lane recorded for "delay" the noun. The disposition
   lists "(drops November 1)" as the harm to fix; the noun reading
   in that exact shape is indistinguishable without syntax the
   app does not have, and the failure direction is the accepted
   suppression class. Recorded here, not reopened.
3. RT15-4's innocent-suppression family (drop-off, forget-me-nots,
   weather delay, and the rest) was not touched, per the standing
   R13-6/RT14-6 acceptance.

## Shipped addendum, R15-B — 2026-10-17

Batch R15-B (anchor bound + pin coverage) shipped in
`crates/ob-extract/src/contribution.rs` only: **698 → 700 tests
passing, 0 failed** (full workspace suite run unpiped on the
final tree, cargo exit code 0 read from a file; run twice, same
totals), **clippy clean** (`--workspace --all-targets -- -D
warnings`, exit 0). Two new pins, both `r15b_*`. **No
pre-existing test was changed** — the r14b pins use file dates
within days of their todays (7 days for the mixed pin, same-day
for the same-week pin), so none falls outside the new bound, and
every round 13–15 held shape stands at its shipped expectation.
Neither the fake-helper ETXTBSY flake nor timing contention
fired in either full run.

What landed, per disposition:

- **R15-4.** The anchor resolution in
  `parse_contribution_anchored` — the one site every caller
  inherits, so the app scan needed no change — gains the
  plausibility bound: the file date anchors only when it is
  valid, not after today, and at most 28 days before today
  (`add_days(d, 28) >= today`); an older file date falls back
  to today exactly like missing/invalid. The reason stands in
  a comment at the site: four weeks covers the late-file harm
  (the R14-6 file one to two weeks late stays anchored);
  beyond it a preserved timestamp is more likely than a
  month-old contribution. Pinned by
  `r15b_file_date_plausibility_bound`, scanned Friday,
  October 23, 2026 on the review lane's witness line "next
  Sunday and October 25, 2026": a file date of Friday,
  September 25 (exactly 28 days before) anchors — "next
  Sunday" reads October 4 with October 25 noted; September 24
  (29 days before) falls back, field-for-field identical to
  the unanchored parse (November 1, October 25 noted); and
  the old-September witness September 20 (33 days before)
  likewise reads as the unanchored parse does. The
  Keep-through mirror is pinned both ways through the same
  resolution.
- **R15-5 (test-only).** The existing r14b same-week pin
  stands intact (anchor == today). Its new sibling
  `r15b_different_day_same_week_file_date_is_identical` runs
  the same 12-line round-13 family plus the Keep-through
  mirror under four different-day same-week anchors —
  Monday, October 12 through Thursday, October 15, 2026,
  against today Friday, October 16 — asserting whole-struct
  identity (`assert_eq!` on the whole `Contribution`) with
  the unanchored parse for every anchor, as the review
  lane's probe verified (0/14 diffs).

No judgment calls were needed: the bound sits inside the
anchor resolution as disposed, and no witness reading
changed beyond what the R15-4 disposition orders — a file
date more than 28 days old now reads at today instead of at
the file date.
