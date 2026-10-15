# Tier-2 brief: OpenBulletin import architecture (independent pass)

You are an independent expert reviewer for OpenBulletin, an open-source
(MIT) Rust desktop app that helps church office administrators (not layout
professionals) produce weekly church bulletins. Publisher is discontinued;
the app is fold-aware (bi-fold/tri-fold presets), field-driven
({Sunday.date}, church profile), and built around "last week's bulletin,
safely turned into this week's." Design law: KISS, guided steps, plain
language, no pro jargon on the main path. Engine state: document model
(panels -> frames -> stories of styled paragraphs, integer milli-point
geometry), imposition, PDF export, and a JSON-based .obd format all exist
and are tested. Text shaping is currently a placeholder estimator; a real
shaping engine (parley-class) is planned.

The maintainer has now required an import track. Imports are framed to
users as "rescues with honest fidelity labels," never magic. Scope:
(1) native .obd (done); (2) Publisher .pub rescue import; (3) InDesign via
IDML (direct .indd reading is out of scope: closed binary); (4) PDF
analysis -> template: the user drops in an old bulletin PDF and the app
drafts an editable template from it (page geometry, text blocks with
positions/fonts/sizes, images), which a human confirms before use.

Answer with structured markdown only:

1. PDF analysis pipeline in Rust: compare realistic extraction routes
   (pure-Rust lopdf-class parsing of content streams, pdfium via FFI,
   MuPDF, poppler bindings) on extraction fidelity (text runs with font,
   size, position; image XObjects), license, packaging on
   macOS/Windows/Linux, and maintenance risk. Recommend one for v1.
2. From extracted runs to an editable template: block grouping heuristics,
   column/panel detection on folded sheets, mapping font size/weight to
   semantic styles (church name, heading, body, order-of-service lines),
   detecting the fold panels vs the imposed sheet problem (most bulletin
   PDFs are reader panels; some are imposed printer spreads - how to
   detect and handle each), and a confidence model: what the app may
   auto-apply vs must ask the admin to confirm, in plain language.
3. Fidelity honesty: what PDF import can never recover (styles, threading,
   fields) and how the result should be labeled/presented so a non-expert
   trusts it appropriately.
4. IDML import: the subset worth supporting first (stories, frames,
   paragraph styles, spreads) and the mapping onto a panel/frame/story
   model; what to deliberately skip.
5. .pub rescue: libmspub (MPL) via out-of-process helper vs shelling to
   LibreOffice headless vs both; licensing and packaging implications for
   an MIT app; what fidelity is realistic.
6. Sequencing: order the four importers by user value per engineering
   risk for a small team, name the first spike for the PDF analyzer
   (inputs, pass/fail evidence), and list what to avoid entirely.

Be specific and opinionated. Flag anything in the framing above you think
is wrong.
