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
