# Audit round 37 — synthesis (coordinator, 2026-10-22)

Audited tree: `d985de8` (round-36 close record on code
tree `e7f3be8`). Three blind lanes, read-only:
verify (`round37-verify.md`), review
(`round37-review.md`), red team
(`round37-redteam.md`), all in
`~/workspace/openbulletin-audit-wave3/`, scratch in
`round37-out/`.

Lane verdicts: verify PASS on all five standing
claims (0/3/2 handed on), review 0/5/3, red team
0/4/4. Deduped: **0 BLOCKER / 6 MAJOR / 5 MINOR** —
the exit bar is NOT met. Majors trend:
10 → 6 → 6 → 1 → 4 → 3 → 2 → 5 → 6 → 8 → 6.

The verify lane's standing claims all held on the
audited tree: suite 1053/0 (run 2; run 1 was killed
mid-flight by a VM reboot and discarded as evidence,
see Records), clippy 0 on a genuinely re-checked
tree (the lane rejected its own first run as a cache
hit and re-checked all 16 crates after
`cargo clean -p`), booklet 31,717 / 34,977
byte-exact, fuzz bins at their recorded md5s with
stdouts byte-identical to the R36-D baselines
(verdict `79775844…`, comp `77f0addd…`), rights
counters 5083 / 1024 / 5275 with safety_flips=0
(M8 held, seventeenth consecutive round), and all
thirteen round-36 pins present in-tree and passing.
Every finding below comes from the carried
observations and the lanes' own hunting, not from a
failed standing claim.

## Adjudications recorded before the findings

1. **The curly family splits by outcome (F2 / F3).**
   All three lanes witnessed carried target (b).
   Arms where a judgment is escaped and a wrong
   outcome is served — a governed date adopted, a
   phrase collapsed to its base, a phantom record
   on agreeing dates, a refusal all twins record
   replaced by silent service — are MAJOR (F2).
   Arms where the line merely goes silent or a
   record is lost, no wrong date adopted, are
   MINOR (F3), on the round-36 F12 precedent.
2. **The two-group compound splits by content
   (F5 / F6).** Carried target (c)(ii), witnessed
   by all three lanes. Contents carrying a
   judgment (governing words in either position, a
   mixed weekday wrapper, a date group) escape it
   glued — MAJOR; inert contents flip
   date-vs-no-date on spacing alone — MINOR. The
   round-36 F1/F10 split, applied to the compound
   family; both land in one batch (R37-C).
3. **R37-R4 is adjudicated IN as MAJOR (F7).** The
   review lane's own caveat is recorded: a bare
   word between a weekday group and the date also
   reads clean, and that arm is the standing R5-D
   adjacency doctrine — it is NOT part of the
   finding. The group-companion arm stands: no pin
   or scope note records adjacency intent for
   groups, R35-C's recorded claim asserts the
   opposite ("spaced and two-group compound forms
   keep the R35-C weekday reading"), lone groups
   judge at the leading and trailing slots, and
   month–day compounds judge. Adjudicated reading:
   a companion group does not disarm a weekday
   group's judgment at the leading or trailing
   slot, glued or spaced. If the fix round finds a
   pin encoding compound-adjacency intent, the
   batch stops and reports, per the standing term.
4. **RT37-2's three vectors are one finding
   (F4-archive, stated as F8 below).** A planted
   directory inside a zip64 extensible sector,
   inside an entry's payload, and behind a prefix
   archive whose EOCD comment reaches into the
   real archive are one candidate-discipline
   defect by the round-36 F6 merge precedent:
   R36-A's discipline names only candidates inside
   another candidate's declared comment span and
   the absorbed prefix; a candidate standing
   anywhere else in the file, earliest in file
   order, is judged. The review lane's zip cells
   (appended and comment-carried candidates in all
   zip64 framings) hold — those are the round-36
   vectors; the red team's are new plant sites.
5. **Two uniform cells are observations, not
   findings.** (i) A weekday group standing
   mid-line before the month word
   (`notes ([Monday])November 1, 2026` and its
   spaced twin) is unjudged in every form — no
   twin diverges; all three lanes touched the cell
   and none graded it. (ii) A wrapped leading
   governor (`(cancelled) November 8, 2026`, all
   pairs) is garnish while the bare leading
   governor governs — uniform across wrappers;
   the split is wrapped-vs-bare, not pair-vs-pair.
   Neither breaks an adjudicated reading. Both are
   recorded under Records and carried to round 38
   as doctrine questions, not defects.
6. **Rights permutation variance is not a
   finding.** The red team's 100,000-quadruple
   sweep: 0 flips on the standing M8 safety axes
   (children, permission) under the standing
   definition (fold vs same-order chain), and 0 on
   credit-presence under that definition. Three
   permutation-variance classes (license/credit
   text selection; credit presence when a
   permission bearer arrives after the credit it
   vouches — the R11-5 arrival-timing class;
   permission-noted where no children note stands)
   are the code's documented arrival-order
   semantics, witnessed and preserved in
   `round37-out/redteam/rights37*.log` for
   re-grade if the definition is ever widened.
7. **Claim-4 wording, corrected on the record.**
   The verdict bin's stdout is 16 physical lines;
   its summary line reads `total_lines=40000
   panics=0`. The standing claim's "40,000 lines"
   is the bin's internal case total, as the
   round-36 close record states; the byte-identical
   md5 is the dispositive check. Future briefs
   phrase the claim accordingly.

## Findings

### F1 (MAJOR) — Carried target (a): a non-leading bracket/brace group beside a relative phrase is adopted as the line's date; the paren twin reads nothing

Verify V1 + review R37-R1 + red team RT37-1,
three lanes converged. Same root mechanism as
round-36 F4: `date_line`'s plain multi-date
reading reads a bracket/brace group's date through
the R24-4 edge trim as a plain signal, and R36-D's
guard `leading_group_beside_relative_core`
(composition.rs:5036) is consulted only for
leading groups. Past any leading word the
bracket/brace group is field-identical to bare
text. Witnesses (both routes throughout):
`notes [November 8, 2026] next Sunday` → for =
2026-11-08 with the phrase's resolution demoted to
a second date; the paren twin is totally silent
and unremembered. The family: disagreeing dates
(`[November 15, 2026]`), restating dates, the
Sunday-after relative (`[November 8, 2026] the
Sunday after next Sunday` — the resolution
adopted, not even named second), stacked groups
(the bracket/brace group wins in either position),
doubled wrappers, prefixes (`the`, `choir notes`,
`notes,`). Held positions in the same sweeps:
leading forms converge in all pairs (R36-D);
groups after the phrase, beside a Sunday-after
phrase in both orders, weekday-garnish and
governor groups beside the phrase, stacked
agreeing groups, and mixed wrappers beside the
phrase all read as their twins. The hole is the
non-leading, pre-phrase slot. Disposition: R37-B.

### F2 (MAJOR) — Carried target (b), adopting arms: curly-quoted words escape judgment at the group, phrase, and garnish sites

Verify V3 + review R37-R2 + red team RT37-3.
Curly pair = U+201C/U+201D and U+2018/U+2019; the
straight-quoted twins read at full parity with
bare since R36-F, and that parity is the target
shape. Arms (both routes):

- **Governor inside a group.**
  `(November 8, 2026 “cancelled”)` and
  `(November 8, 2026 [“cancelled”])` adopt
  Nov 8; curly-single forms likewise. Straight
  and bare govern (for = None). Site:
  `clean_group_word` (dates.rs:1380) — its trim
  set contains no curly quotes — reached via
  `group_token` (dates.rs:1319).
- **Group canary.** `November 15, 2026 (“Sunday”
  after November 1, 2026)` → for = 2026-11-15
  with phantom ParenthesizedDateMismatch
  { named: 2026-11-01, read: 2026-11-15 }; the
  agreeing form serves Nov 8 with the same
  phantom shape. Straight/bare twins read the
  resolution with the line's date as second date
  and record nothing. The F3 mechanism: the
  in-group phrase never forms because the
  `dates_inside_group_signals` gate
  (dates.rs:929) and `sunday_after_phrase_run`
  (composition.rs:2981) judge cleaned tokens and
  `group_token` leaves `“sunday”` uncleaned.
- **Phrase collapse.** `(“Sunday” after
  November 1, 2026)` → for = 2026-11-01 (the
  base adopted as the answer); same under
  bracket/brace outer wrappers and in
  trailing-group forms.
  `Sunday after (“Sunday”) November 1, 2026`
  collapses the same way, while its unwrapped
  curly twin reads Nov 8. Disagreeing forms:
  `(Sunday after “Monday” November 1, 2026)` →
  for = 2026-11-01, no record; the bare twin is
  refused with the mismatch recorded.
- **Base trailing garnish.**
  `Sunday after November 1, 2026 “Monday”` →
  for = 2026-11-08 served with no record; bare,
  straight, and paren twins all refuse with
  `WeekdayMismatch(Monday@2026-11-01)`. Site:
  `sunday_after_base`'s trailing trim
  (dates.rs ~:514) holds `\u{201d}` but not
  `\u{201c}`, so a full curly pair is not
  trimmed.

Disposition: R37-E, with F3 — one fix over the
full site set the lanes mapped (see R37-E).

### F3 (MINOR) — Carried target (b), silence arms: curly-quoted words read nothing where their straight and bare twins read

Verify V2 + review R37-R6 + red team RT37-5.
No wrong date is adopted in any arm; the line
goes silent or a record is lost (F12 class):

- Shape word whole-line: `“Sunday” after
  November 1, 2026` → total silence; also
  curly-single, open-quote-only, close-quote-only,
  and behind `the`. (Behind-"on" forms are silent
  in curly, straight, and bare alike — no
  divergence there.)
- Second word: `Sunday “after” November 1, 2026`
  → silence; straight reads Nov 8.
- Base-leading weekday, disagreeing (unwrapped):
  `Sunday after “Monday” November 1, 2026` →
  silence, **no mismatch record**; bare and paren
  twins refuse with the record. Agreeing arm
  reads Nov 8, at parity.
- Line-head framing: `“Monday” November 1, 2026`
  and `on “Monday” November 1, 2026` → silence;
  the agreeing `“Sunday” November 1, 2026` →
  silence where straight/bare serve 2026-11-01 —
  a correct date dropped entirely.
- Plain trailing garnish: `Novembe
...[truncated 10716 chars]