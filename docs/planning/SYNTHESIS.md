# OpenBulletin synthesis — build, adopt, or layer?
Date: 2026-10-15. Inputs: Tier-2 Opus 5.5 conceptualization (opus-concept.md, 20,977 completion tokens), DesignCraft evaluation (../research/designcraft-evaluation.md), Publisher/InDesign/Scribus research. GPT 6.1 Sol Pro pass: FAILED twice (run 1 timed out at 280s with no output; run 2 returned HTTP 524 provider timeout after ~254s, no content). Opus pass stands as the Tier-2 record; a GPT pass can be re-run later if Jacob wants the second family on file.

## Opus verdict (independent, written without knowledge of DesignCraft's current state)
- Thesis: the product is "last week's bulletin, safely turned into this week's bulletin, printed right the first time." Test every feature against a volunteer on Saturday afternoon.
- Needful v1: fold-aware presets (edit reader panels, app owns the sheet), automatic printer spreads, duplex test-sheet wizard (print an arrow sheet once per printer, remember flip direction), threaded text frames with loud overflow, right-tab dot leaders for the order of service, styles, spellcheck, paste-matching-style by default, fields ({Sunday.date}, {Sunday.name}, church profile), a "Next Sunday" command (clone, advance date, clear weekly frames, keep standing content), blocks library (recurring liturgy), built-in QR, preflight-lite (overset, missing fonts, low-dpi, fold-zone/dead-margin violations), bundled large-print OFL fonts, 10-20 templates.
- Bloat, never/defend-in-public: mail merge/email sending, web/HTML export, EPUB/interactive PDF, GREP/nested/conditional styles, footnotes/index/book files, real-time collaboration/cloud, stock art, bundled copyrighted hymn lyrics or modern Bible translations, AI generation in core, .pub export, .indd reading, IDML export, full PDF editing, WordArt, plugin marketplace.
- UX modes: three full personalities are a TRAP (triples test/docs surface, breaks phone-help where both people must see the same menus). Thin version is real: one canonical UI, Simple vs Full workspace density, keymap presets (OpenBulletin/Publisher/InDesign), vocabulary aliases in search/tooltips only. Invariants: file format, menu tree/labels, command behavior, rendering output never vary by mode.
- Architecture if building: egui + CPU tile raster (vello_cpu/tiny-skia), parley for shaping + own flow engine, krilla for PDF, immutable document snapshots for undo, i64 1/1000pt geometry, headless core crates + CLI as CI oracle. First spike: threaded-frame text editing with screen/PDF pixel-diff ("Hymn of the Day" spike) — the project-killing risk.
- Format: zip + JSON, content-addressed assets, self-contained by default; .pub = rescue import only, marketed as "rescues content," never "opens Publisher files."

## Where DesignCraft changes the calculus
DesignCraft already implements the Opus "eventually" engine list (composer, threading, styles, CMYK/PDF-X, IDML, data merge, booklet imposition) in two weeks of agent-driven development, and is agent-drivable via MCP + CLI. What it lacks is exactly Opus's v0.1 wedge list: fold-aware bulletin presets/templates, Next Sunday workflow, church profile/fields, duplex wizard, preflight-lite tuned to bulletins, and a volunteer-simple workspace. Its gaps doc also lists beta-blocking basic-editing bugs, so it is not yet proven for the Sunday test.

## Recommended shape (supersedes "build a new app")
1. Trial DesignCraft against the 5 pre-written criteria (evaluation doc). Jacob's decision pending.
2. If engine passes: "OpenBulletin" becomes a LAYER on DesignCraft, not a fork and not a new engine — a bulletin template pack + church profile/fields conventions + Next Sunday automation built on DesignCraft's CLI/MCP/data-merge, contributed upstream where it fits (templates, simple workspace proposal). Small, ownable, useful even if upstream moves fast.
3. If engine fails the trial: fall back to the Opus build plan, starting with the text spike, reusing Opus's crate map and never-list as policy.

## Amendment 2026-10-15 (later): GPT record corrected + GPT-family pass landed

Correction to the history above: the GPT sol-pro attempt history was
mis-recorded. Attempt 1 timed out at 280s. Attempt 2 (medium, 32k)
SUCCEEDED (13,890 prompt / 8,684 completion tokens, 18,932 chars) and its
output was saved to gpt-concept.md, but the coordinator deleted that file
during commit cleanup, unread, trusting the wrong failure summary.
Attempt 3 returned an empty provider stub. The GPT-family pass was then
completed on gpt-6.1-sol (same brief); its output is the current
gpt-concept.md. The loss and the error are recorded here deliberately.

GPT (sol) vs Opus convergence: template-first weekly workflow ("prepare
next Sunday without damaging the layout"); one editing model with
configurable workspaces, never three personalities; local files; a
typography-to-paper spike first; strict bloat exclusions (GREP styles,
footnotes/index, EPUB/web, collaboration/cloud, plugin ecosystems).
Divergences: GPT phases mail merge/data merge and simple tables into a
later Publisher-replacement tier (P2) instead of Opus's never-list, and
phases tri-fold to v0.5 where the shipped scaffold already includes it.
Adopted stance: keep Opus's never-list as the public commitment for the
bulletin product; GPT's P2 framing is noted as the revisit path if a
Publisher-replacement tier is ever funded deliberately.

## Amendment 2026-10-15: import architecture (Opus import pass)

Full text: opus-import.md. Adopted decisions:
- PDF extraction: PDFium via pdfium-render, out-of-process helper
  emitting a versioned JSON extraction dump; all heuristics in pure Rust
  on the dump; lopdf as sidecar for structural facts. MuPDF and Poppler
  are excluded on license (AGPL/GPL vs the MIT app). The dump schema is
  the hub every importer targets.
- Imports produce a draft bulletin ("Rebuilt from your old bulletins")
  that enters the normal weekly flow, not a separate template object.
  The Template Analyzer's Accept/Edit/Cancel review applies to that
  draft; corpus diffing across weeks is the standing-vs-weekly signal
  (independently matches Jacob's analyzer design).
- Real text shaping (parley) is a hard prerequisite for shipping any
  importer; the extraction spike may run in parallel.
- Confirmation is specific questions with pictures, never blanket OK.
  Per-item badges: Matched / Check this / Picture only. Side-by-side
  swipe comparison against the original is the primary trust tool.
  No OCR in v1: scanned/outlined pages become a tracing-guide underlay.
- .pub order: guided "Save as PDF from Publisher" first, then a
  LibreOffice bridge (soffice convert-to PDF, then the PDF analyzer),
  then the libmspub helper only on demonstrated demand. MPL licensing is
  compatible with shipping in an MIT app with source and notices.
- IDML subset as spec'd in opus-import.md section 4 (threading
  preserved, style names kept, linked images prompted, masters
  flattened and labeled). A .docx evaluation is flagged before IDML on
  user value (admins live in Word); IDML stays on the roadmap per
  Jacob's requirement.
- First spike: 2 weeks, ~30 real bulletins corpus with hand-written
  ground truth, hard pass bars (98% char accuracy, 100% unreadable-page
  detection, zero confident wrong sheet classifications).
