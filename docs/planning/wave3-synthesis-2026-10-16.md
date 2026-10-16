# Wave 3 synthesis — the overnight build wave of 2026-10-15/16

Coordinator synthesis, written 2026-10-16 after the last build landed.
Every build below was dispatched from the wave-2 synthesis order
(E1 → O1 → G1 → G2 → T1 → M1) plus the evening's additions (A1, H1,
Extras, X1, H2), built by one writer at a time, and independently
re-verified by the coordinator (full test suite re-run + spot greps)
after every handoff. Jacob directed the wave's scope across the
evening and signed off at 23:29 PDT; the builds ran overnight.

## The numbers

- Tests: 138 (start of wave) → **294 passed, 0 failed**; clippy
  `--workspace --all-targets -D warnings` clean at every landing.
- Crates: 12 → **15** (ob-image, ob-clips, ob-music,
  ob-verovio-helper; ob-html folded into ob-extract modules).
- Dependency tree: 524 → **603 packages**, no new copyleft family
  (the H2 lane caught scraper dragging MPL-2.0 into the tree and
  used html5ever directly instead).
- Export size arc for the 16-panel sample booklet: 1,251,892 bytes
  (audit) → 654,910 (O1 compression) → **31,136** (O2 CID +
  subsetting).

## What shipped, in order

- **E1** (628120c, 152 tests): real typeface on the canvas. Typed
  text is shaped; field chips and dot leaders are atomic in the
  edit map; caret, selection, word jumps, and IME all run on real
  cluster geometry. Editor, canvas, and PDF cannot disagree about
  line breaks.
- **O1** (c81b035, 157): FlateDecode on content + font streams
  (~48% smaller) and the export purpose presets (web 150 / print
  300 / shop 600 / full) as policy + PDF metadata. Subsetting
  STOPPED at its own stop rule: the subsetter strips the cmap our
  WinAnsi writer depended on; recorded, not faked.
- **G1a** (377a6b2, 174): .obd v1 (zip + content-addressed assets
  with SHA-256 fingerprints + per-asset rights records), dump
  schema v2 (alignment, italic, color, images), IDML Justification
  + .sla alignment read, draft style consensus, style color prints,
  and the occasions model (any date in any accepted format, service
  name + time, {Service.time} token).
- **G1b** (8b3bca0, 207): inline formatting runs everywhere:
  model, shaped layout, per-run PDF faces, editor toggles with the
  chip rule, IDML CharacterStyleRange runs in, markdown both ways
  (one-door .md import as "Notes or web text", rich paste
  conversion, export-md). Scoped out with reason: per-note .pub
  runs (the placement pen dissolves paragraph identity; faces
  still land as runs).
- **A1** (e175231, 210): app comfort: text size in three steps,
  high contrast (test-asserted ratios: body text 21:1, problem
  8.4:1, warn 8.1:1, good 7.4:1, info 11.5:1), stronger focus,
  scale-aware canvas handles.
- **O2** (df8876f, 214): CID font embedding (Type0/CIDFontType2,
  Identity, ToUnicode) + subsetting. Booklet 31 KB. Text now
  extracts/searches correctly from our PDFs. Premise correction:
  the bundled Liberation faces contain NO U+271D/U+2713 at all
  (cmap dump, all 8 faces) — the ceiling lift is real (Greek
  proven), and the cross now prints from any face that has it
  (DejaVu proven end to end). CFF faces keep the legacy path,
  stated in the Check predicate so it never overpromises.
- **G2** (21feb36, 236): picture frames (one shared geometry
  function for canvas/preview/PDF/Check), the recognition door
  (PNG/JPEG/WebP/GIF/SVG/JPEG XL pure-Rust via jxl-oxide),
  EXIF orientation, print right-sizing at the purpose PPI, the
  blur check (fires only under 120 effective DPI; the church's
  own ~150 stays silent), the clip library (folder/zip intake
  with counted skips, WMF/EMF via LibreOffice when present,
  never blank thumbnails), and a 12-piece verified CC0/PD
  starter set (sourced from Wikimedia Commons after
  Openclipart/FreeSVG failed from this network; per-item
  manifest; render-eyeball QA rejected color art, a hazard
  pictogram, a political emblem, and a photo-trace).
  **AVIF: stop rule triggered with evidence** — decoding needs
  dav1d via pkg-config, absent here and unprovisioned in the
  Windows bundle workflow, so AVIF ships as recognized
  convert-first beside HEIC, reason recorded in LEGAL.md.
  License surprise, good kind: resvg/usvg are Apache-2.0 OR MIT
  in their own manifests, not MPL-2.0.
- **T1** (9b8e205, 247): document threading. One continuation
  function (ob-text::threading) serves canvas, previews, Check,
  and PDF. .pub threaded boxes now share one story; hand
  threading ("Continue this text in another box..." / "Make this
  box start its own text") with plain-words armed confirms;
  badges for the selected thread; the red overflow mark on the
  last box; PDF proof: every probe word occurs exactly once,
  leftover words zero times.
- **M1** (4548037, 265): music Phase A. Our own MusicXML parse
  (counts, credits, rights), the Verovio helper (PDFium pattern,
  versioned JSON), systems as picture frames at full panel width,
  move-only (size is the legibility contract), count comparison
  on every import (fixture: 237/237 noteheads, 14/14 measures,
  13/13 fermatas; syllables drew 59 vs 58 parsed — the comparison
  accuses only on shortfall and prints both numbers). Floor
  proven by measurement: 6.50 mm at target width and at 60 mm
  (11 systems instead of smaller notes); a 3.0 mm request
  refused with the measured 3.7 mm named. Bundle truth: Verovio
  publishes no standalone Windows binary (the official route is
  the PyPI wheel), so the bundle stages our helper and says in
  plain words that notes stay undrawn until an engraver is
  staged; LEGAL.md records the LGPL posture.
- **H1** (f905189, 269): HTML export, the web bulletin. One
  self-contained file (2,907 bytes for the text-only sample),
  reading order, runs as strong/em, tabbed lines as paired
  spans, pictures embedded (SVG inlined and sanitized), credits
  composed from rights records with license addresses as visible
  text, the reading-edition sentence at the top.
- **Extras** (4889bb2, 280): large-print copy (11pt body → 17pt,
  4 reader pages → 5, identical word sets), participant fields
  (Readers/Ushers/Greeters/Communion assistants as weekly fields
  with tokens; Next Sunday clears and names them), spoken-only
  paragraphs (excluded from all seven export doors, proven
  end-to-end through the CLI; the .obd keeps them), DejaVu Sans
  in the starter pack via its own release (its embedded license
  is the DejaVu/Bitstream Vera license, not the OFL the catalog
  claims; recorded truthfully) + the Check pointer sentence.
- **X1** (61e4dda, 285): IDML and .sla exports carry runs
  (round-trips exact); one inexpressible Scribus state (bold-off
  inside a name-carrying family) counted in the export note.
- **H2** (20439c2, 294): HTML import as a rescue kind. Layout
  tables walked as flow, content tables as tabbed lines, runs
  carried, data-URI pictures into the asset store, drops
  counted in plain words, web pictures never fetched (import
  makes no network calls).

## Honest residue (unchanged gates and known limits)

- No human has clicked the new surfaces (picture boxes, clip
  library, music door, comfort settings, threading actions,
  spoken-only toggle, web page arm). The standing GUI gate is
  now the whole frontier; a Windows bundle from HEAD 20439c2
  (run 38880870493) carries everything.
- AVIF decode and the Verovio engraver both wait on bundle
  staging decisions (dav1d provisioning; the Python-wheel
  question), not on our code.
- Still recorded as debt: PDF silent overflow clip when Check
  is skipped; font pin (font_sha256) enforcement + .ttc;
  IDML fabricated paragraph positions + rotation on import;
  multi-select/align/distribute/rotation/master guides;
  frame-identity refactor; seasonal packs; announcement
  intake; archive search; lectionary lookup; JPEG→JXL store
  recompression (option); true rich clipboard paste (egui
  clipboard is text-only).
- Blocked on the world, unchanged: a real .musicxml from the
  church's music director; the first live .pub test at the
  church PC; Pastor Mark's workflow notes; the INDD probe
  fixtures.
