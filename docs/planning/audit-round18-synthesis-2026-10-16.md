# Round 18 audit synthesis — 2026-10-17

HEAD audited: `485530e` (round-17 close). Lanes: verify `8cffc3b0`
(report `~/workspace/openbulletin-audit-wave3/round18-verify.md`),
review `055c72e7`, red team `f100f218`. Verify: **PASS, 6 of 6
claims**, CI for the close push green on all three OS jobs (run
39084286101), bundle 39084290647 SUCCESS, booklet bytes exact,
Bach live passing. Review **0 BLOCKER / 2 MAJOR / 6 MINOR**; red
team **0 BLOCKER / 3 MAJOR / 4 MINOR**. **The exit bar is not
met.** Deduped: **0 BLOCKER / 5 MAJOR / 7 MINOR**.

## Findings and dispositions

**R18-1 (MAJOR; review).** Tokenization asymmetry: the plain-line
cleaner does not strip ":" while the group tokenizer does, so
"cancelled: November 1, 2026, next Sunday" ARMS the cancelled
November 1 (the colon-free twin correctly asserts October 25).
Disposition: one token-cleaning behavior — the plain path strips
":" exactly as the group path does.

**R18-2 (MAJOR; RT18-2).** Governance attaches to occurrences,
not dates: a cancelled date restated anywhere on the line
resurrects and asserts. "November 1, 2026, cancelled, November 1,
2026" asserts November 1; parsed Oct 23, "November 1, 2026,
cancelled, next Sunday" asserts it too (the relative restates the
cancelled date). Disposition: within a line/group, a date value
governed in ANY occurrence does not assert from another
occurrence, unless a target claim names it as the (last) target.
Restated UNGOVERNED signals keep reading plainly (R13-C held).

**R18-3 (MAJOR; RT18-1).** Last-target ranking ignores relative
targets on plain lines: "November 1, 2026, moved to next Sunday,
then to November 15, 2026" files for the superseded October 25
with November 15 merely noted; the framed and group twins assert
November 15. Disposition: target ranking is by written order
across framed and relative targets alike, both surfaces.

**R18-4 (MAJOR; review).** An intervening date inside the claim
reach collapses the claim: "(moved the meeting, November 1, 2026,
and the dinner to November 8)" reads both dates as alternatives
and asserts nothing, where its comma-after-date twin asserts
November 8 with November 1 named. Disposition: an intervening
date in the reach is a from-date (governed + named), not a
reach-breaker.

**R18-5 (MAJOR; RT18-3).** The passive-"for" occasion collapses
when any second date stands in the group: "(it was scheduled for
November 1 and November 8)" and "(it was moved for November 1,
use November 8)" assert nothing, both dates merely noted.
Disposition: the for-date enters the line's composition as a
signal like any other — two for-dates read by primacy with the
other noted; a prospective second date ("use November 8")
asserts with the for-date named.

**R18-6 (MINOR; review).** Groups carry no relative signals:
"(November 1, cancelled, next Sunday)" as a group reads nothing,
while the plain line asserts October 25. Disposition: groups
resolve relative phrases at the anchor and judge them as signals
in written order alongside framed ones (one signal set
everywhere). Governors inside the group judge them as they judge
framed signals.

**R18-7 (MINOR; review).** Salvaged plain lines never name the
governed relative ("cancelled, next Sunday, November 1, 2026"
asserts Nov 1, Oct 25 unnamed). Disposition: governed relative
signals are named on the note by their resolved date, as framed
ones are.

**R18-8 (MINOR; review R18-5).** The claim reach crosses a real
second clause: "(moved November 1, choir sang to November 8)"
claims Nov 8 across "choir sang". The date-span exemption covers
commas inside the span, but a new clause after the span must end
the reach. Disposition: after an intervening date's span, the
reach continues only into the target construction itself ("to",
"then to", "and to"); other clause content ends it.

**R18-9 (MINOR; RT18-4).** Active "for" never names the occasion:
"(we moved it for November 1)" is silent. Disposition: a
displacement verb + "for" + date names the date in the active
voice too, as the passive rule does.

**R18-10 (MINOR; RT18-5/6 + review R18-8).** Adverb tolerance
seams: two stacked tolerated adverbs break it ("(it was just
recently moved, November 1)" silences while the has-been twin
spends), and the closed list omits then, briefly, shortly,
formally, unanimously, happily. Disposition: tolerance is up to
TWO consecutive listed adverbs; the list gains then, briefly,
shortly, formally, unanimously, happily.

**R18-11 (MINOR; RT18-7).** Chains split across two groups assert
correctly but drop the superseded target unnamed (the single-
group twin names it). Disposition: superseded targets are named
on the line's note across group boundaries.

## Batches

- **R18-A** (composition core): R18-1, R18-2, R18-3, R18-6, R18-7.
- **R18-B** (claim/for/adverb edges): R18-4, R18-5, R18-8, R18-9,
  R18-10, R18-11.

Round 19 audits the fixes. The Tier-2 final gate runs only after
a round returns zero BLOCKER + zero MAJOR.
