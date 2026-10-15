# Roadmap

## v0.1 — The Sunday Test (in progress)
Acceptance: a real church secretary, unaided, produces and prints a real
Sunday letter bi-fold bulletin, then runs Next Sunday for the week after.

- [x] Engine scaffold: model, presets/templates, imposition, PDF export, CLI
- [x] Next Sunday command (CLI + shell button)
- [ ] Text spike: threaded-frame editing with print-identical output
      ("Hymn of the Day" spike: two threaded frames, dot-leader tab,
      overflow indicator, undo; canvas vs PDF pixel-diff in CI)
- [ ] Desktop shell: inline text editing, save/open, autosave + recovery
- [ ] Duplex test-sheet wizard (print an arrow sheet once per printer;
      remember the flip answer per printer)
- [ ] Signed installers: Windows, macOS (notarized), Linux (AppImage/Flatpak)
- [ ] Pilot: 5-10 real churches using it weekly
- [x] Guided weekly flow in the shell (Start, This Sunday, Announcements,
      Check, Print) with plain-language checks

## v0.5 — The Church Office
Tri-fold and tabloid templates, church profile UI, blocks library
(recurring liturgy), liturgical calendar fields (Sunday names, lectionary
references as data packs), spell check, preflight-lite, native print
dialogs, Publisher keymap preset, 25+ templates, PDF trace layer for
rebuilding an old bulletin over a PDF of it.

## v1.0 — Credible for Small Organizations
Newsletters (facing pages, jump lines), hyphenation and H&J controls,
large-print companion export (same stories at 16-18pt), tagged PDF,
`.pub` rescue import, shareable template packs, file-format stability
guarantee (every 1.x opens every 1.x file).

## Import and conversion track (Jacob, 2026-10-15)
Imports are rescues with honest fidelity labels, never magic (PRINCIPLES.md):
- [x] Native .obd (v0) — ours, full fidelity by definition.
- [ ] PDF analysis -> template (flagship): drop in an old bulletin PDF; the
      app extracts page geometry, text blocks with positions and fonts, and
      images, then drafts an editable OpenBulletin template that a human
      confirms. The old PDF also stays placeable as a locked trace layer.
- [ ] .pub rescue import: text, images, and page geometry out of Publisher
      files via libmspub (the MPL library LibreOffice uses) in an
      out-of-process helper, mapped onto the closest template.
- [ ] IDML import (InDesign's interchange format; .indd itself is a closed
      binary and stays out of scope — InDesign users export IDML, which is
      Adobe's own supported path out).
- [ ] PDF place as graphic + trace layer (also v0.5).

## Later, only if earned
Print-shop phase (CMYK, spot, ICC, bleed/marks, PDF/X-4), CSV data merge
(name tags, certificates), Scribus import, IDML read-only import.
