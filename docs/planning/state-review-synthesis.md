# Tier-2 state review: synthesis (2026-10-15)

Reviewers: Opus 5.5 (13,061 chars) and GPT-6.1-sol (20,292 chars),
brief at tier2-state-review-brief.md. Raw reviews beside this file.
Spend (estimate): about $0.24 total, far under the $6 cap.

## The font ruling: unanimous

Copy-in is correct, font packs yes, live system fonts no. Both
reviewers grounded it the same way: print's reproducibility rule
(InDesign Package / Document fonts), self-hosted webfont practice,
and the NAS shape having no usable system fonts. The copy-in step
must be made invisible: picker with previews, one click, plain
words at the copy moment ("copied to your Font Library so your
bulletins look the same on every computer"). Ship a Church
Starter Pack (Liberation family, Carlito, Caladea, Gelasio, plus
2-3 OFL display faces like Playfair Display or Lora).

## Where both reviewers converged (strongest signal)

1. Acceptance mismatch: the corpus church makes 16-page
   half-letter saddle booklets; the roadmap's v0.1 acceptance
   test is a letter bi-fold. Make the saddle booklet a
   first-class v1 preset and fix the acceptance words.
2. PDF font embedding is v1-critical, not later: today we shape
   with library metrics and draw with core Helvetica, so the
   Check step's promise is not fully honest yet. Embed the exact
   library bytes.
3. .obd v1 packaging (zip with fonts and assets inside) moves
   forward into the v1 line: SHA pins without packaging cannot
   keep the "identical on every machine" promise, and
   multi-machine use is already the normal case.
4. Autosave and crash recovery: atomic save prevents corruption
   but a crash mid-edit still loses work; "never lose work"
   needs a journal and a restore offer.
5. Inline text editing on the page, undo included, is
   v1-necessary; panel-only editing feels like a form, not
   Publisher.
6. Imports: invert the order. Trace-layer rescue first (last
   week's PDF as a locked picture, rebuild over it), then
   single-PDF rescue, then the corpus analyzer. Do not stake v1
   on the hardest import problem.
7. lopdf stays internal scaffolding only; PDFium is the source
   of truth everywhere user-facing.
8. Church v1 acceptance should be narrower than "credible for
   small organizations": one church, weekly bulletin in bifold
   OR saddle booklet, template carries over, one rescue path,
   zero font surprises between two machines.

## Single-reviewer findings worth keeping

- Opus: bundle the italic faces (they exist, they are OFL, we
  only bundled Regular and Bold). One-hour fix, real bulletins
  need italics. VERIFIED TRUE against the tree.
- Opus: fsType alone is not license safety. For fonts added
  outside the free catalog, ask once in plain words to confirm
  the right to share, and remember the answer.
- Opus: image handling is undocumented in the docs; the corpus
  church uses cover pictures weekly. Needs a documented story.
- Opus: the 5-step flow and the analyzer's thresholds are fit
  to one church and one tradition; test with 2-3 real admins and
  more traditions before locking.
- GPT: printer identity for the duplex wizard should be robust
  (drivers rename printers); test across all three OSes.
- GPT: offline and timeout messaging for the catalog in plain
  words (a 30 s timeout exists; the wording can improve).
- GPT: missing-font handling must be a clear "you don't have
  this font, here's the fix" state, never a silent fallback.

## Reviewer claims checked

- "No italics bundled": true (ob-text assets hold Regular and
  Bold only).
- "Shaper and exporter use different fonts": true by design
  today; embedding was already the documented next step.
- Opus's exact advance-width numbers for Liberation vs
  Helvetica are his figures, not measured here; the direction
  (small drift, visible on dot leaders and right-aligned page
  numbers) matches our own notes.
- "No font selection in the UI": partly dated. Styles name
  families and shaping honors them since the font increment;
  there is still no font picker inside the editing UI.
