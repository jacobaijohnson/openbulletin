# Tier-2 Conceptualization Brief — OpenBulletin (working name, renameable)

You are an independent expert in desktop publishing software, Rust desktop architecture, and product scoping. You are blind to the other auditor's answer. Give a direct, opinionated conceptualization. No hedging lists of everything — commit to recommendations.

## The mission
Build an open-source, cross-platform (macOS, Windows, Linux) desktop publishing application in Rust. Working name: OpenBulletin.

Trigger facts (verified Oct 15, 2026):
- Microsoft Publisher was discontinued Oct 13, 2026: removed from Microsoft 365 Oct 1, 2026; perpetual versions lose support Oct 13, 2026. Its .pub format is proprietary; very little other software opens .pub well (LibreOffice and Scribus import with losses).
- Adobe InDesign is the pro standard: $22.99/mo single-app subscription, steep learning curve, no Linux version, heavy.
- Scribus is the incumbent open-source DTP app: free, CMYK/spot/ICC, PDF/X, runs on all three OSes — but its UI feels dated/unpolished, workflow friction is a common complaint, and it cannot open/save native InDesign files.
- Affinity Publisher proves demand for a one-time-price pro tool ($69.99, now Canva-owned); Canva proves demand for template-first ease, but is cloud/subscription and weak at precise print imposition.

## Beachhead use case (v1 wedge)
Church bulletins, built weekly by non-designer volunteers/staff, often inherited hand-to-hand like the old Publisher files:
- Typical format: US Letter 11x8.5 landscape sheet, bi-fold to 5.5x8.5 (4 panels/pages: front cover, inside left = order of service, inside right = sermon notes/announcements, back cover), also tri-fold 11x8.5 (6 panels) and plain 8.5x11 portrait multi-page.
- Recurring content: date/Sunday name, lectionary readings, order of service with hymn numbers, sermon title/pastor, announcements, giving/QR, contact/service times.
- Workflow reality: last week's file is copied, dates/text swapped, printed in-house on an office printer/copier, folded. Speed and "don't break the layout" matter more than design freedom. Large, readable type matters.
- Later expansion: newsletters, flyers, brochures, booklets — general Publisher-class documents first, InDesign-class long documents much later if ever.

## Product idea to evaluate
- One document engine, three switchable UX personalities over the SAME file format and shortcuts-learnable core:
  1. "OpenBulletin Native" — a simpler, opinionated, template-first layout (the default).
  2. "Publisher mode" — ribbon-ish, wizard/template-gallery feel, Publisher vocabulary and familiar shortcuts for refugees.
  3. "InDesign mode" — panels/tools, master pages, paragraph/character styles vocabulary, InDesign-ish shortcuts for pros.
- Question: is mode-switching a real feature or a trap? If real, what exactly varies (labels? panel arrangement? shortcuts? defaults?) and what must NEVER vary?

## Rust ecosystem facts to work with (July 2026 ecosystem surveys)
- GUI options: egui/eframe (immediate mode, MIT, wgpu, fast to build, custom-drawn everything, weaker native accessibility/widgets), iced (Elm-style, MIT, COSMIC uses it, pre-1.0), Slint (DSL, GPLv3-or-commercial dual license — licensing is a real consideration for an OSS project), Tauri 2 (webview UI + Rust core, MIT, best accessibility via web tech, but webview quirks on Linux and a JS frontend), GPUI (Zed's, Apache-2.0, less library-ready).
- Text: cosmic-text (pure Rust shaping/layout via rustybuzz+swash, fontdb fallback), parley (Linebender, HarfRust), Typst's text stack exists as libraries. Text flow across linked frames, hyphenation, OpenType features are the hard core of DTP.
- Rendering/PDF: wgpu/skia-safe (Skia bindings), tiny-skia, vello; PDF via pdf-writer / printpdf / Typst's PDF export; CMYK and PDF/X are specialist territory.
- The document canvas needs: infinite-zoom vector canvas, snapping/guides, text-frame threading, master pages, styles cascade, undo/redo, imposition (printer spreads for folded booklets), preflight.

## Your tasks
1. USEFUL AND NEEDFUL vs BLOAT: Audit Publisher and InDesign feature-by-feature at category level. Three buckets: (a) Must have in v1 for the bulletin wedge, (b) Must have eventually for a credible Publisher replacement (name the phase), (c) Bloat / trap features to deliberately NEVER build or defer indefinitely — with the reasoning a small volunteer team can defend in public. Be specific: name features (e.g. mail merge, data merge, GREP styles, footnotes/endnotes, EPUB export, web banners, mailings tab, building blocks, etc.) and place each.
2. ARCHITECTURE: Recommend a concrete Rust stack (GUI framework, canvas renderer, text engine, PDF path) with the tradeoffs stated plainly. Recommend crate/workspace structure. Say what to prototype first to kill the biggest risk.
3. UX MODES: Verdict on the three-personality idea; if viable, define the exact surface that changes and the invariants.
4. FILE FORMAT + INTEROP: Native format choice; realistic stance on .pub import, IDML, PDF import/edit, SVG, and Scribus files. What is honest to promise refugees in year 1?
5. MVP DEFINITION: The smallest v0.1 that a real church secretary could use for a real Sunday, and a phased roadmap (v0.1, v0.5, v1.0) with each phase independently useful.
6. RISKS: Top 5 ways this project dies, and the earliest observable signal of each.

Output: structured markdown, decisive, specific. Length is fine up to ~3000 words. Do not restate the brief.
