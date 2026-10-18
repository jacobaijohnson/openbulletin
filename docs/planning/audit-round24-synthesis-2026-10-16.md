# Round 24 synthesis (2026-10-18)

Lanes over the round-23 close `289bdf0` (774 tests): verify PASS
0/0/0; review 0/6/5; red team 0/2/5. Reports:
`~/workspace/openbulletin-audit-wave3/round24-{verify,review,redteam}.md`.

Deduped: **0 BLOCKER / 5 MAJOR / 4 MINOR.** The exit bar is not
met; the Tier-2 final gate stays queued.

Coordinator adjudication on RT24-7 (rights): the flips
reproduce byte-identically to round 23 (output md5s match) —
19/3,000, all license-only in effect; zero children flips;
zero safety flips in 80,412 route comparisons; 127,128
children-bearing checks clean. MINOR restatement of the known
residual; no fix, census stands (fifth consecutive round).

Held and worth naming: the composition oracle's bad-divergence
counts are zero outside the resurrection families below
(155,648 fuzz lines, 0 both-some-differ, 0 for-some/keep-none
outside the standing asymmetry); PP stacks of 3–4 all correct;
every two-word auxiliary slot correct in all new orders;
loader families linear; verdict contract 40,000 × 6 clean.
The remaining resurrections (2,851 in the fuzz) are entirely
accounted for by R24-1/R24-2 and the in-group bracket shapes
of R24-4 — the rounds-21–23 families contribute zero.

## MAJORs and dispositions

**R24-1 (review F1 = RT24-1) — the subordinator class.**
when, whenever, while, whilst, although, though, because,
if, yet, provided, supposing, wherever, where, why, how,
whether resurrect in head and trailing slots on every
surface (32/32 slot checks). The inventory held the class's
rare members (lest, albeit, whereas) but not its commonest —
exactly the corrected claim's MAJOR case (ordinary office
vocabulary). Disposition: the subordinator class joins the
inventory. "than" (review M1) joins the connective class in
the same batch.

**R24-2 (RT24-2) — phrase units.** The stack units R23-A
recorded-not-fixed still resurrect (24/24): "care of",
"back together", "gone tomorrow" and the lane's extensions.
Their members are not individually function words ("care"
is a noun; "gone" a verb form), so no word list reaches
them. Disposition: the bare test gains PHRASE units — a
region is bare when its words partition into inventory
words and listed phrase units, matched as units. The units
are the lane's enumerated set, stated as a closed list in
the code; "per se" (review M1) is one of them.

**R24-3 (review F2 + M2, RT24-6 part) — the fence judges
the phrase's head.** The inverted fence reads
`cleaned[i+1]`, the word after the weekday, so a stacked
phrase passes on its middle word: "Sunday morning
rehearsal", "Sunday school picnic", "Sunday morning
service", "morning meeting" all assert while their bare
twins read nothing. Disposition: the fence judges the
phrase's HEAD — the final noun of the weekday phrase —
against the garnish set; a stacked phrase passes only when
its head is garnish. The garnish set completes with the
printed-matter kin the lanes found missing in the safe
direction (bulletin singular, papers, newsletter, leaflet,
sheets). Held: "Sunday service" reads nothing (service is
an event head, not garnish — the review's own answer);
"Monday bulletins" asserts as pinned.

**R24-4 (review F4/F5/F6 = RT24-3, review M5 part) — the
boundary rule is a token rule.** Standalone `*`, `•`, `|`,
`~`, `_`, and emoji tokens are in no site's character set,
so they invert postposed compositions, revive the
wrong-weekday bypass, and destroy relative memory — the
R23-3 defects behind unlisted characters. Disposition:
the boundary classification stops being a character table.
Any standalone token containing no letters and no digits
is a boundary token at every site. Bracketed tokens
(`[x]`, `{x}`) count as boundary/garnish tokens in group
cleaning and date spans (they neither resurrect inside a
group nor poison a following date); brackets are NOT group
delimiters (a bracketed date stays unread — the safe
direction, recorded as a limitation).

**R24-5 (review F3) — weekday PPs inside the object
continuation.** "baked beans from Kent on Sunday to
November 8" claims November 8 naming a phantom October 18;
swapped order reads October 18 outright. A weekday PP
inside the claim's object continuation is a modifier of
the object, and nothing consumes it. Disposition: a
weekday standing as a PP object inside the object
continuation is consumed wholly (no signal, no naming),
exactly as in a subject PP; the standing "from"
governance reading of the line is unchanged (both orders
read nothing, name nothing).

## MINORs and dispositions

- **R24-6 (review M3 = RT24-5) — detection and spending
  disagree on the passive-for complement.** "(it was
  softly gently moved for November 1)" asserts nothing
  while its detection twin reads the clause. Disposition:
  the passive-for scan's complement crosses adverbs as
  detection does (uncapped adverbs between auxiliary and
  participle; a negation counts as one slot word), so the
  occasion asserts; governor-phrase spending caps are
  unchanged (they guard reach across dates, not a local
  complement).
- **R24-7 (review M4 = RT24-4) — relative phrases as
  subject-PP objects.** "the choir from next Sunday sang"
  names October 25; "before this Sunday" + auxiliary
  demotes the target to alternatives. Disposition: a
  relative phrase standing as a subject-PP object is
  consumed by the subject exactly as a framed date object
  is (R23-5), its resolution named as the framed twin's
  object is named; the clause's own date keeps its
  standing.
- **R24-8 (RT24-6 part) — possessive garnish.** "our
  Sunday bulletins" silences (safe direction). Disposition:
  a possessive determiner before the weekday phrase does
  not block the fence. Conjoined garnish ("bulletins and
  school…") stays silent — accepted, recorded (safe
  direction).
- **R24-9 (RT24-7)** — adjudicated above; no fix.

## Fix batches

- **R24-A** (inventory + boundaries + fence): R24-1,
  R24-2, R24-3, R24-4.
- **R24-B** (consumption + alignment): R24-5, R24-6,
  R24-7, R24-8.

Held lines: the R19–R23 families, defensible silence,
claim/scope pins, the standing relative-line asymmetry,
content labels assert, "thanks" head content, brackets
are not group delimiters, rights pin, loader pins,
booklet bytes 31,717 / 34,977. Flag any changed
pre-existing test loudly by name with its reason.

## Shipped addendum, R24-A (2026-10-18)

R24-A shipped in `crates/ob-extract/src/contribution.rs`
only: R24-1, R24-2, R24-3, R24-4. Numbers: baseline
774 tests passing at `d7bea77`; after this batch the
full unpiped workspace suite reads **778 passed,
0 failed** (774 + 4 new tests, one per item, all
witnesses pinned on both the For Sunday and
Keep-through paths), and clippy over the workspace
(`--all-targets`) is **clean, 0 warnings**. The
fake-helper ETXTBSY flake did not fire. **No
pre-existing test was changed** — no changed test
to flag; the diff touches no pre-existing test body.

- **R24-1.** The subordinator class joins
  `bare_filler_word`: when, whenever, while, whilst,
  although, though, because, if, yet, provided,
  supposing, wherever, where, why, how, whether —
  head and trailing slots, all surfaces; "than"
  joins the connective class. The mirror held
  (labels assert, "(our choir November 1)" asserts);
  the recorded-not-graded pronoun / modal /
  interjection sets stay untouched, pinned asserting
  in the new test.
- **R24-2.** The bare test (`occurrence_is_bare`)
  gains phrase units, matched as whole consecutive
  phrases before any member is judged alone. The
  phrase-unit list as shipped (closed): "care of",
  "in care of", "care of the", "back together",
  "back together again", "gone tomorrow", "gone by
  tomorrow", "per se", "if and when", "when and if",
  "as and when", "yet again", "together at last".
  A unit's member standing alone keeps its current
  reading ("(care November 1)" and "(gone
  November 1)" still assert, pinned).
- **R24-3.** `lone_framed_fence` judges the
  weekday phrase's HEAD — its final noun — against
  the garnish set. "Sunday morning rehearsal",
  "Sunday school picnic", "Sunday morning service",
  "Sunday morning picnic", "Sunday bulletins
  rehearsal", "Sunday morning meeting" now read
  nothing; "Sunday morning", "Sunday school",
  "Sunday bulletins", "Sunday school bulletin"
  assert. The garnish set as shipped (closed):
  bulletins, school, morning, bulletin, papers,
  newsletter, leaflet, sheets — so "Sunday bulletin,
  November 1, 2026" and "Sunday newsletter,
  November 8, 2026" assert. Held: "Sunday service"
  reads nothing; "Monday bulletins, November 8,
  2026" asserts as pinned; the verb-form adjective
  phrase ("Sunday left luggage, November 8, 2026")
  still passes by phrase shape per the R23
  adjudication, not through the garnish-head test.
- **R24-4.** The boundary rule is a token rule:
  any standalone token containing no letters and
  no digits is a boundary token at every site using
  the boundary classification — implemented in the
  shared `group_token` cleaning and
  `clause_break_after` (governor scope, group and
  postposed composition), in `relative_shape_matches`
  (relative-shape memory), and in
  `composition_segment_weekday_mismatch` (weekday
  check). The F4 witness reads as its comma twin;
  F5's wrong-weekday witnesses record the mismatch
  behind "*", "•", "|", "~", "_", and emoji
  separators; F6's relative-memory witnesses keep
  the in-words reading across those tokens.
  Bracketed tokens ("[x]", "{x}") clean to boundary/
  garnish in group cleaning, a token closing a
  bracketed span carries the span's boundary for
  clause scope, and "[" / "]" at a date token's
  edges no longer poison `parse_date_prefix`:
  "[November 8, 2026], November 15, 2026" reads
  November 8 naming November 15, "Sunday bulletins,
  [November 8, 2026]" asserts, "[November 1,
  cancelled] [November 8]" reads November 8 naming
  November 1, and "cancelled [November 8, 2026]"
  stays governed. Brackets are NOT group delimiters:
  "[November 8]" and "{November 8}" alone stay
  unread, recorded as the limitation. Held: the
  tabled separators' readings are unchanged (the
  r23a boundary pin stands unmodified); numeric-date
  slashes stay token-internal.
