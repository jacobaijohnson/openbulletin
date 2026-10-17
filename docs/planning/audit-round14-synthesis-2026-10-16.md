# Round 14 audit synthesis — 2026-10-17

HEAD audited: `4204531` (round-13 close). Lanes: verify
`4811fe34` (report `~/workspace/openbulletin-audit-wave3/round14-verify.md`),
review `bdbb94c1`, red team `36ad1140`. Verify: **PASS, 10 of 10
claims**, CI for the close push green on all three OS jobs (run
39062936219), bundle 39062937309 SUCCESS (first bundle carrying the
V3b icon and .ico), booklet bytes exact, Bach live passing. Review
**0 BLOCKER / 2 MAJOR / 5 MINOR**; red team **0 BLOCKER / 2 MAJOR /
4 MINOR** (its report carries one self-correction on surface-4 key
tuples; totals unchanged). **The exit bar is not met.** Deduped:
**0 BLOCKER / 2 MAJOR / 6 MINOR**, one family throughout: governors.

## Findings and dispositions

**R14-1 (MAJOR; review R14-1 + RT14-2).** The before-position net is
still incomplete, in two layers. (a) The wider verb set the R13-A
builder flagged as untaken all contributes at full strength: put
off, postponed, delay, delayed, omit, disregard, forget (about),
and the split form "put November 1 off". (b) The attested verbs' own
morphology: cancelling, canceling, postponing, removing, dropping,
scratching, delaying, omitting, dropped — `postpone` is in the class
and its past tense is not, because the list matches whole tokens and
no tense class exists beside the contraction class. Disposition:
complete the verb set (put off incl. the split form, delay, omit,
disregard, forget) AND add a morphology rule: the cancellation verbs
match by stem (base, -ed, -ing incl. doubled-consonant and e-drop
forms), the same structural move R13-1 made for contractions. Pinned
on the relative, paren-only, and Keep-through paths.

**R14-2 (MAJOR; review R14-3 + RT14-1).** `governed_after` was never
widened: after a date, only cancelled/canceled govern; "(November 1,
drop it)", "(November 1, scratch that)", postponed, removed,
dropped, put off all contribute the cancelled date. An asymmetry
R13-A itself created. Disposition: the trailing position draws from
the same family function as the before position (incl. the verb+noun
forms: "scratch that", "drop it", "put off", "hold off").

**R14-3 (MINOR; review R14-2).** "nope" before a date does not
govern. Disposition: add to the negation family.

**R14-4 (MINOR; review R14-4 + RT14-3).** Hyphenated governor
compounds (do-not-use, no-longer, hold-off) with a spaced date
contribute; the fully hyphenated form is held only by the date
reader. Disposition: tokenization treats a hyphenated compound
containing a governor stem or negation as governing (split on
hyphens for the governor test only; date reading unchanged).

**R14-5 (MINOR; review).** Unpunctuated "wont" overreaches on
possessed "his wont" (accustomed), suppressing dates in the safe
direction. Disposition: "wont" does not govern when immediately
preceded by a possessive determiner (his, her, their, my, our,
your, its). The "won't" contraction form is unaffected.

**R14-6 (MINOR; review R14-6 + RT14-5).** Mixed-signal relative
phrases anchor at parse day (scan-time `Local::now`, re-derived
every scan, never recorded). A relative-first line parsed a week
late demotes its own explicit date ("next Sunday and October 25,
2026" parsed Oct 23 reads for Nov 1). Review: document-Sunday
anchoring is circular at scan time; the file's mtime is the
available anchor (when the sender's words arrived/were written).
Disposition: mixed-signal relative resolutions anchor at the
contribution file's modification date when the parse carries one,
else today; the anchor is the file's date wherever the parse knows
it. In-words (standing) anchoring on the open bulletin is unchanged.

**R14-7 (MINOR; review R14-7 + verify variance).** The r13b pins'
flat 5 s caps failed under contention in both lanes (5.27–5.82 s at
loads ~11–15; outcomes passing; solo and release far inside).
Disposition: the same hang-guard treatment R13-8 gave the r10 pin —
test-only, outcome and time-class assertions primary, reason at the
site. (The r12b pins' relative budgets have not failed; they stand.)

**RT14-4 (MINOR).** The unspent-governor reach-back is exactly one
clause: "(ignore, the date, November 1)" and "(ignore, notes,
November 1)" contribute although "(ignore the date, November 1)" is
a held witness. Disposition: reach back over consecutive
intervening clauses while each contains no date token and consists
solely of filler words (the, a, an, date, it, note, notes).

**RT14-6 (no code change).** "drop"/"scratch" widen the R13-6
title/name suppression ("(Drop, Drop Slow Tears, November 1)").
Safe direction, same accepted class as R13-6; the governor cannot
distinguish the hymn from the verb. Recorded; reopens only on a
showing of harm beyond a date unread from a parenthetical.

## Batches

- **R14-A** (governors): R14-1, R14-2, R14-3, R14-4, R14-5, RT14-4.
- **R14-B** (anchor + timing): R14-6, R14-7.

Round 15 audits the fixes. The Tier-2 final gate runs only after a
round returns zero BLOCKER + zero MAJOR.
