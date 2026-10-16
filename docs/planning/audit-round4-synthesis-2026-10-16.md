# Round 4 audit synthesis (2026-10-16)

HEAD audited: `1dc1ea9`. Lanes: verify, review, red team (three blind
Wonder lanes over the round-3 fix batches).

**Verdict: 0 BLOCKER, 8 MAJOR, ~20 MINOR. Exit bar NOT met.** (The bar:
a full round at zero BLOCKER and zero MAJOR; then the Tier-2 final gate.)

Verify: 7 VERIFIED, 1 FAILED, 0 UNVERIFIABLE. All round-3 fix claims
verified against shipped tests read line by line, plus an independent
hand-built size-lie .obd refused by the real CLI in ~3 s. The music
claim ran live through the real Verovio helper: 237 noteheads, staves
6.4-6.6 mm. The FAILED claim is a stale constant, not a regression:
the sample booklet is now 31,717 imposed / 34,977 reader bytes (was
31,136 / 34,386) because R-2b grew the starter template. Re-baselined
below; future rounds measure against the new numbers.

Review: 0 BLOCKER, 5 MAJOR, 7 MINOR. Red team: 3 MAJOR, 11 MINOR,
and the round-3 hardening HELD under re-attack (declared-size lies
ignored in both directions at ~1 ms; the quote-blinding class stayed
dead across five evasion shapes; the clips bomb stopped at its cap).

## The MAJORs

1. **R4-1 Fonts identity (review + red team 1a).** `base_name` strips
   "Roman", so "Times New Roman" becomes "Times New": its Liberation
   Serif row is unreachable, the document is laid out in a sans, and
   installing the real font never upgrades it. Same root, wider: the
   import path stores the stripped display name on the styles and
   throws the raw source name away, so any family whose name ends in
   a face word ("Latin Modern Roman", "Acme Bold") can never
   self-heal, and a document set in "Acme Bold" prints in "Acme" when
   both are installed (the stripped attempt wins before the raw one).
2. **R4-2 Rename migration is positional (review + red team 6).**
   Reordering or swapping serving-line names in the profile editor,
   with no rename intended, silently loses one line's stored names
   and prints another line's names against the wrong duty. Root:
   positional diff + `or_insert` discarding found values.
3. **R4-3 Music proof dedupe key (review).** The proof dedupes on the
   system index alone, but indices count from 0 per piece: with two
   MusicXML pieces in one service, the second piece's systems vanish
   from the proof the music director checks.
4. **R4-4 Applied-part guard timing (review + red team 5b).** The
   fingerprint is recorded only after Accept's own save succeeds.
   On the failed-save path the app's own status text leads the office
   to save by hand and re-open the part, which offers Accept again:
   every appended word lands twice. The recovery door reaches the
   same place (the set clears on adopt; the journaled words remain).
5. **R4-5 Rights merge is winner-take-all (review + red team 7).**
   Attaching a sheet whose record beats the existing one on credit or
   license replaces the WHOLE record: title, creator, provenance are
   lost; a split-strength pair loses the CC BY license from the
   printed credits; and `children_present` can be merged from
   Some(true) down to Some(false), disarming the web hold-back.
6. **RT-2 Entry-count freeze (red team).** A 22.6 MB .obd with
   100,001 tiny assets loads in 229 s: no entry-count cap, and the
   asset dedupe scans the accumulated list (quadratic). The .obdi
   loader is not affected (BTreeMap).

## MINORs (dispositioned)

- RT-1c Check names a stored "Verdana-Bold" verbatim: cured by the
  R4-1 identity fix (the wanted record keeps a name a person can
  install; resolution keys off the preserved full name).
- R4-6 Guard survives Undo / Next Sunday: cured by the R4-4 fix
  (fingerprints live in the document state; Undo removes them with
  the words; Next Sunday clears the list for the new week).
- R4-7 / RT-5d Weekday kindness only covers For Sunday, first
  position only: extend mismatch detection to the Keep-through line
  and to a weekday written after the date. Fix in R4-D.
- R4-8 Plan freshness: the plan redraws against the live document
  every frame and the after-apply summary is truthful. ACCEPTED as
  a residual; no fix.
- R4-9 Pin/writer candidate sets differ; a stale pin is never
  cleared. Dormant today (font_sha256 read nowhere). Fix in R4-A:
  one candidate rule for pin and writer; a None pick clears the pin.
- R4-10 / RT-3 HTML scanner: a `>` inside an honestly quoted value
  can still split a tag for counting (conservative direction only).
  Fix in R4-B: a quote that legitimately opens a value (first
  character after `=`) is honored until its matching quote; the
  stray-quote rule that killed RT3-A is untouched, and the
  post-parse depth check remains the backstop.
- R4-11 / RT-4 CDATA backoff fooled by `<![CDATA[` inside an XML
  comment (>32 MB parts only; the loss note understates). Fix in
  R4-B: the backoff scan tracks comment state.
- R4-12 / RT-8 Hymn memory save-order effects (correction war,
  resurrection by reprinting an old bulletin): inherent to
  last-confirmation-wins; the forget escape works and Check names
  the remembered date. ACCEPTED, recorded so round 5 does not
  rediscover them.
- RT-6b Rename onto an existing line drops the renamed line's names;
  RT-6c a deleted line's names keep printing invisibly. Fix in
  R4-C with R4-2: renames are set-based (a name that still exists in
  the new list is never migrated); a line removed with no rename
  target has its stored names cleared at profile save, and the
  status line says so. Nothing vanishes silently.
- RT-3 attr-deep page refusal and RT-4 note wording ride along with
  the R4-10 / R4-11 fixes.

## Checked and sound (do not re-report)

Round-3 loaders' at-cap exactness; inbox cap serving both folders;
source_families determinism; the used-family wanted filter; RT3-F
counted skips; Next Sunday clearing scope via the shared service
story lookup; filed-name readback; canonical snapshot coverage (no
live-shape dirty comparison remains); simulated planning's clone
discipline; all four fix commits inside their briefs.

## Re-baseline (verify 1b)

Sample booklet at HEAD `1dc1ea9`, library path: **31,717 bytes
imposed / 34,977 reader**. CLI path (titled starter): 32,334 /
35,593. Cause: R-2b's Serving This Week section in the starter
template. The property stands (~31-35 KB vs the 1.25 MB pre-O2
baseline).

## Fix batches

- **R4-A fonts identity**: R4-1 + RT-1a/1c + R4-9. Preserve the full
  source family name as the style's identity; resolution tries the
  full name exact before any stripped form at every tier; the
  user-facing wanted name never strips a space-separated "Roman" or
  "Regular" (separator forms - , and glued still strip); "Times New
  Roman" reaches its metric-twin row. Pin and writer share one
  candidate rule; a None pick clears the pin.
- **R4-B hostile input II**: RT-2 entry-count cap (8,192 entries,
  .obd and .obdi) + set-based asset dedupe; R4-10 scanner quote
  honor; R4-11 comment-aware CDATA backoff.
- **R4-C people + proof**: R4-2 set-based rename migration + RT-6b/c
  removal honesty; R4-3 proof key (source, system), pieces in
  document order.
- **R4-D seam + rights + dates**: R4-4/R4-6 fingerprints move into
  the document (applied list, serde-default, cleared by Next
  Sunday, ignored by every export door); R4-5 field-wise rights
  merge (children_present merges as OR); R4-7/RT-5d weekday
  mismatch on Keep-through and after-date positions.

Then round 5.

## Shipped addendum, R4-A: font identity (2026-10-16)

- **Identity preserved (R4-1, RT-1a).** The draft builder
  now stores the FULL source family name on each import
  style (subset tag off, nothing stripped: "Times New
  Roman", "Verdana-Bold", "Latin Modern Roman",
  "GillSansMT" all stay whole); the display form survives
  only in words: the style's label, the review sentences,
  the wanted record. Every dump door (PDF, IDML, .pub,
  .sla) funnels through this one builder, and every
  resolution consumer (shaper, PDF writer, Check oracle,
  painters, pin) already keys off the style's family, so
  identity now flows from the full name everywhere. The
  app's wanted-font Check note also resolves from a
  style's full name rather than the record.
- **The shown name (R4-1, RT-1c).** `base_name` is
  separator-aware: after a dash or comma the full
  face-word set comes off (Bold, Italic, BoldItalic,
  Oblique, BoldOblique, Regular, Roman); after a space
  only Bold, Italic, BoldItalic, Oblique, BoldOblique
  come off, never Roman or Regular. "Times New Roman"
  bases to itself and reaches its Liberation Serif
  metric-twin row; "Verdana Bold" still names "Verdana".
  New records therefore never carry a face's name
  (RT-1c's wart for new files); a legacy record written
  by an older build still shows its stored string in the
  note, but resolution no longer depends on it.
- **Resolution order.** `resolve_family` tries the full
  name at Exact before the display form, then the base,
  then the row (consulted on the full name). With both
  "Acme" and "Acme Bold" installed, a document set in
  "Acme Bold" resolves Exact "Acme Bold". This supersedes
  the R3-A addendum's display-first order, and its pinned
  "Acme Roman" → "Acme" expectation: the spaced form now
  keeps Roman knowingly, for the reason above; the dash
  form still strips it.
- **Pin agreement (R4-9).** One candidate enumeration,
  `ob_fonts::candidate_faces`: bundled faces first, then
  the library's, only may_package faces, one winner per
  (family, bold, italic) key by pick_face. The PDF
  writer's `face_sources` and `pin_document_fonts` both
  draw from it (the writer attaches bytes by hash; the
  pin takes the bundled facts from
  `ob_text::shaping::bundled_font_infos`). When
  pick_face returns None for a style, the pin is
  cleared, never left stale. Tests pin the set equality
  (a Restricted and a PreviewAndPrint face excluded, the
  library winning a shared key), bundled faces pinning
  with an empty library, the stale pin clearing, and the
  writer excluding a Restricted face.
- Tests changed knowingly: `every_name_bases_to_its_family`
  ("Acme Roman", above); the draft tests
  `rescue_builds_frames_styles_and_badges` (the heading
  style's identity is now "Arial-Bold", the full name
  its file stated; the wanted record still names
  "Arial") and the GillSansMT draft test (style identity
  "GillSansMT", label and record still "Gill Sans MT").
  FONTS.md corrected: styles keep the full source name,
  the shown name is derived by the separator rule, and
  the pin/writer candidate rule is stated.
- Suite: 486 passed / 0 failed (477 + 9 new), clippy
  `-D warnings` clean, both from unpiped runs. One full
  run hit the known helper ETXTBSY flake
  (`a_helper_from_the_future_is_refused_in_words`); it
  passed solo and the clean re-run is the 486 above.
