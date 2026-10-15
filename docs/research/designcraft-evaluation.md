# DesignCraft evaluation — is it enough to skip building OpenBulletin?
Date: 2026-10-15. Sources: getartcraft.com/apps/designcraft, github.com/storytold/designcraft README, ROADMAP.md, docs/gaps.md (reviewed 2026-10-10), GitHub API repo/releases/code-search (2026-10-15).

## What it is
- Pure-Rust, egui, clean-room InDesign reimplementation. MIT/Apache-2.0. macOS/Windows/Linux + WebAssembly. Signed installers (MSI, DMG notarized, AppImage/Flatpak/deb/rpm).
- Repo created 2026-10-01; 1,025 commits, v0.8.0 released 2026-10-14, 474 open issues at check time. Stage per its own ROADMAP: pre-alpha.

## Already has (maps to the needful list)
- Spreads, parent pages, threaded stories/frames, paragraph+character styles, swatches (CMYK/spot), text wrap.
- Knuth-Plass paragraph composer, dictionary hyphenation, baseline grid, optical margins.
- Print-ready PDF: selectable text, embedded fonts, CMYK/spot, bleed/crop marks, PDF/X-4, PDF/A-2b.
- File > Print Booklet: saddle-stitch / 2-up printer spreads as PDF (the bi-fold bulletin imposition).
- IDML import AND export; Data Merge (CSV/TSV/Excel, QR placeholders); footnotes, tables, hyperlinks; EPUB export.
- Agent-native: JSON control channel + MCP server — an agent can drive layout directly.
- No subscription, no telemetry.

## Gaps that matter for a church bulletin (from its own docs/gaps.md)
- BETA BLOCKER: basic editing fails for many users — copy/paste broken on Windows/Linux in 0.4/0.5 builds, context menus not opening, caret character formatting discarded (16 issues, Oct 5-10).
- ALPHA BLOCKER: Primary Text Frame autoflow adds pages users cannot delete.
- Installed fonts not always found; startup crashes on some GPU/driver combos (fallback landed, unconfirmed by reporters).
- Printing immature: no printer-driver options UI or print presets; physical fidelity unverified.
- IDML round-trip unproven on real files; composition reflows vs InDesign.
- NO .pub import (code search: zero hits). NO Publisher-style mode, NO template gallery, NO bulletin templates. UI is an InDesign clone — powerful, not volunteer-simple.
- No CI on PRs per gaps.md.

## Verdict
Engine problem: solved well enough that building a competing Rust DTP engine from scratch is not justified now.
Product problem (Publisher-easy, template-first, bulletin-shaped): NOT solved by DesignCraft.
Rational path: trial DesignCraft on a real bulletin; if it passes, contribute the missing bulletin layer upstream (templates, simple mode proposal) instead of a new app. Keep OpenBulletin as the fallback if the trial fails or upstream stalls.

## Trial pass/fail criteria (pre-written)
1. Install standalone build on Jacob's Windows desktop; opens, finds the church fonts.
2. Recreate one real Sunday bulletin (11x8.5 landscape bi-fold) as a reusable template.
3. Weekly edit test: change date, readings, hymn numbers, sermon title, announcements in <15 min without breaking layout; copy/paste works throughout.
4. Print Booklet PDF imposes the 4 panels in correct order/orientation.
5. Print on the actual printer/copier; output matches screen (fonts, margins, fold).
Fail any of 1/3/4 => OpenBulletin build decision comes back to the table with the Tier-2 plan.
