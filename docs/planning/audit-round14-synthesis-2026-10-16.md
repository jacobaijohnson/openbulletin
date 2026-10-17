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

## Shipped addendum, R14-A — 2026-10-17

Batch R14-A (governors) shipped: R14-1, R14-2, R14-3, R14-4,
R14-5, RT14-4, all in `crates/ob-extract/src/contribution.rs`
(the one governor machinery every consuming path reads through
`analyze_group`). R14-B (R14-6 anchor, R14-7 timing) is not in
this batch.

What landed, by disposition:

- **R14-1(a)** the verb set is completed in the before position:
  "put off" (pair, plus the split form "(put November 1 off)",
  read as "put" last in the window with "off" first after the
  date), delay, omit, disregard, forget, and the pair "forget
  about" — on the relative, paren-only, and Keep-through paths.
  A bare "put" with no "off" governs nothing (pinned).
- **R14-1(b)** morphology: the cancellation verbs (cancel,
  scratch, drop, postpone, remove, delay, omit, disregard,
  forget) match by stem — a stem rule beside the contraction
  class, not a longer literal list. Base, -ed, and -ing forms
  govern, including doubled-consonant spellings (dropping,
  omitting, cancelling) and e-drop spellings (canceling,
  postponing, removing; postponed/removed via the bare-d past).
  All attested literals keep working through the same rule.
- **R14-2** `governed_after` parity: the trailing position now
  draws from the same family definition. A cancellation phrase
  after a date cancels it wherever it stands (verb stem forms,
  "hold off" / "put off", "never mind", "forget about", and
  verb+object "drop it" / "scratch that" / "cancel it"); the
  prospective rest of the family falls back on the date it
  follows only when no later date is left for it to govern —
  the standing "not" / "no longer" scoping, generalized, so
  "(November 1, not November 8)" still asserts November 1. The
  spent exception in the reach-back now covers the whole
  cancellation family, so "(November 1, postponed, November 8)"
  and "(November 1, drop it, November 8)" assert November 8
  exactly as "(November 1, cancelled, November 8)" always has.
- **R14-3** "nope" joins the negation family (before position;
  its trailing form disqualifies under the last-date fallback).
- **R14-4** hyphenated compounds: a token is split on hyphens
  for the governor test only — a part that governs, or parts
  forming a governor phrase ("do-not-use", "no-longer",
  "hold-off", "never-mind", "scratch-that", "don't-use"),
  governs. Date reading and group tokenization are otherwise
  unchanged; a compound with no governor part ("follow-up")
  governs nothing (pinned).
- **R14-5** "wont" does not govern when immediately preceded
  by a possessive determiner (his, her, their, my, our, your,
  its): "(his wont, November 1)" now NAMES its date on the
  relative, paren-only, and Keep-through paths. The "won't"
  contraction is unaffected, including after a possessive.
- **RT14-4** the unspent-governor reach-back crosses
  consecutive intervening clauses while each consists solely
  of filler words (the, a, an, date, it, note, notes — a clause
  of those words holds no date token): "(ignore, the date,
  November 1)" and "(ignore, notes, November 1)" disqualify.
  An intervening clause with real content ("choir notes",
  "notes from the pastor") still stops the reach-back
  (pinned as controls).

Judgment calls, stated plainly:

- **Noun derivations do not govern.** The stem rule matches
  verb inflections only: "cancellation" and "postponement"
  govern nothing, so "(cancellation policy, November 1)" and
  "(postponement notice, November 1)" NAME their dates
  (pinned). "dropped" in a serving note is a governor by
  design (it is the verb's past form); the noun forms were
  the line drawn.
- **"called off" / "put back" / "push back" / "reschedule" /
  "move"** were recorded by the review lane as contributing
  but were not in the disposition's verb set; they are NOT
  taken here.
- RT14-6 stands as recorded in the synthesis (no code change):
  "drop"/"scratch" as titles still suppress — accepted, not
  revisited.

Tests: five new pins, all `r14a_*` in `contribution.rs` —
`r14a_wider_verb_set_governs_before_the_date`,
`r14a_cancellation_verbs_govern_by_stem` (twelve morphology
forms incl. one doubled-consonant and one e-drop family,
plus the noun/near-miss controls),
`r14a_trailing_position_draws_from_the_same_family`
(before/after symmetry pairs, the spent-trailing held lines,
the scoped "not"), `r14a_nope_hyphenated_compounds_and_possessed_wont`,
`r14a_reach_back_crosses_filler_only_clauses` (both RT14-4
witnesses plus a two-filler-clause chain and content-clause
controls). **No pre-existing test changed** — the full suite
ran green on the production changes before the new pins were
added, and every round 11–14 held shape stands at its shipped
expectation (garnish, between-two-dates, per-occurrence
scoping, title false-governors).

Numbers: baseline 685 tests passing (HEAD `81c1a91`); after
R14-A, **690 passed / 0 failed** (`cargo test --workspace`,
unpiped, cargo exit 0, run twice on the final tree with the
same totals). Clippy `cargo clippy --workspace --all-targets
-- -D warnings` clean (exit 0). Neither the fake-helper
ETXTBSY flake nor a timing-pin contention failure fired in
either full run, so no solo-rerun adjudication was needed.

## Shipped addendum, R14-B — 2026-10-17

Batch R14-B (anchor + timing) shipped: R14-6 and R14-7.
R14-A (governors) is not in this batch.

**R14-6 — the anchor rule, exactly.** A new parse entry,
`parse_contribution_anchored(text, today, file_date)`
in `crates/ob-extract/src/contribution.rs`, takes the
contribution file's modification date when the caller
has one; `parse_contribution(text, today)` keeps its
signature and passes None. The effective anchor is the
file date when it is a real calendar date and is not
after `today`, and `today` in every other case — no
file date, an invalid date, or a file date in the
future: the anchor never stands in the future beyond
today. The anchor reaches only
`plain_multi_date_reading`, and there it re-resolves
a relative signal only when the line proves truly
mixed — at least one framed signal standing beside at
least one pure relative signal ("next Sunday",
"this Sunday", a bare "Sunday", "the Sunday after
next/this Sunday"). Framed signals still read as
written against `today`, a Sunday-after phrase over a
framed base still resolves off that base, the
relative-only readings (restated phrases, the bare
in-words line, differing relatives) are unchanged
against `today`, and the standing in-words reading —
anchored on the open bulletin at verdict time — is
untouched. When the file date and today agree, the
parse is field-for-field identical to the standing
parse (pinned across the round-13 family, below).

In the app, the one file-reading flow threads it:
`scan_contribution_folder` (`crates/ob-app/src/main.rs`)
already read each file's mtime as `modified_secs` for
version ordering; it now converts that same value to
a local calendar date (`file_date_of`: zero or
unconvertible carries None) and passes it to the
anchored parse. The paste path and the review's edit
re-parse carry no file and keep the standing
`parse_contribution` at today.

Pinned witnesses (file date Friday, October 16, 2026;
today Friday, October 23, 2026): "next Sunday and
October 25, 2026" reads for **October 25 with nothing
noted** — at the file's date the relative phrase
restates the framed date — where the unanchored
control reads November 1 with October 25 demoted to
the note; the reversed order likewise (anchored, the
spurious November 1 note the late parse manufactures
is gone). The distinct-signal witnesses keep their
notes at the anchor: "next Sunday and November 1,
2026" reads for **October 25 with November 1 noted**,
in either order and on the Keep-through line. "The
Sunday after next Sunday and November 1, 2026" reads
November 1 plainly, where the unanchored control read
November 8 over it. Future, invalid, and missing file
dates all fall back to the today reading (pinned).

**R14-7 — timing, test-only.** In
`crates/ob-format/src/lib.rs`, the pin
`r13b_chain_cost_no_longer_scales_with_chain_length`
(**FLAGGED pre-existing test change, by disposition —
the only one in this batch**) had its two flat
`Duration::from_secs(5)` caps (the chain8000-40k row
and the chain8000-200k row) become a 300 s hang guard,
exactly as R13-8 treated the r10 pin: the cap now
catches a hung or runaway load only. Its outcome
assertions (the honest bulletin loads in every row)
and its relative time-class budget (the long-chain
row against the short-chain row) are unchanged in
strength and stay primary. A comment at the site
records why: in round 14 the 5 s caps failed under
contention in two lanes at 5.27–5.82 s under loads
~11–15, outcomes passing. The other r13b pins carry
no wall-clock budgets, so no other budget moved; the
r12b pins' budgets — relative and absolute alike —
stand untouched, as they have not failed.

Tests: three new pins — `r14b_mixed_relatives_anchor_at_the_file_date`
and `r14b_same_week_file_date_is_identical` in
`contribution.rs`, and
`r14b_inbox_scan_anchors_mixed_relatives_at_the_file_mtime`
in `ob-app` (a file whose mtime is set to October 16,
scanned with today October 23, reads its mixed line
for October 25). **One flagged pre-existing test
change** (the r13b budget above, by disposition); the
r13c pins pass unmoved, as does every other
pre-existing test.

Numbers: baseline 690 tests passing (HEAD `737cd68`);
after R14-B, **693 passed / 0 failed** (`cargo test
--workspace`, unpiped, cargo exit 0, run twice on the
final tree with the same totals). Clippy
`cargo clippy --workspace --all-targets -- -D warnings`
clean (exit 0). Neither the fake-helper ETXTBSY flake
nor a timing-pin contention failure fired in either
full run, so no solo-rerun adjudication was needed.

## Coordinator close (2026-10-17)

Both batches shipped and were independently re-verified by the
coordinator (full unpiped suite + clippy -D warnings at each step):
R14-A `737cd68` (690 tests; verb set completed, stem morphology,
governed_after parity, nope, hyphen-split tests, possessive wont,
multi-clause filler reach-back; zero changed pins; noun derivations
pinned as non-governing; called off / put back / reschedule / move
recorded as outside the disposition's set for round-15 grading),
R14-B `585b084` (693 tests; mixed-signal relatives anchor at the
file's modification date when the parse carries one, future/invalid
fall back to today, same-week parses field-for-field identical; the
r13b flat caps became the 300 s hang guard, one flagged change by
disposition). Round 15 audits these fixes and dispositions the
remaining verb family (called off, put back, push back, reschedule,
move) plus stem-morphology edges. The Tier-2 final gate runs only
after a round returns zero BLOCKER + zero MAJOR.
