# Roadmap

## v0.1 — The Sunday Test (in progress)
Acceptance: a real church secretary, unaided, produces and prints a real
Sunday bulletin in the shape their church actually uses (letter bi-fold
or the 16-page half-letter saddle booklet), then runs Next Sunday for
the week after. (Corrected 2026-10-15 on Tier-2 review advice: the
reference church's bulletins are booklets, not bi-folds.)

- [x] Engine scaffold: model, presets/templates, imposition, PDF export, CLI
- [x] Next Sunday command (CLI + shell button)
- [ ] Text spike: threaded-frame editing with print-identical output
      ("Hymn of the Day" spike: two threaded frames, dot-leader tab,
      overflow indicator, undo; canvas vs PDF pixel-diff in CI)
- [x] Desktop shell: inline text editing, save/open, autosave + recovery (shipped 2026-10-15; inline editing is whole-box words, true shaped caret editing is a later phase)
- [x] Duplex test-sheet wizard (print an arrow sheet once per printer; (shipped 2026-10-15 as the test sheet plus plain-words questions on the Print step)
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
- [x] PDF rescue (shipped 2026-10-15): open an old bulletin PDF in the
      app; words come back as editable boxes with Good / Check this /
      Picture only labels, and each old page stays underneath as a
      tracing picture until removed. Accept / Edit / Cancel review.
- [ ] .pub rescue import: text, images, and page geometry out of Publisher
      files via libmspub (the MPL library LibreOffice uses) in an
      out-of-process helper, mapped onto the closest template.
- [ ] IDML import (InDesign's interchange format; .indd itself is a closed
      binary and stays out of scope — InDesign users export IDML, which is
      Adobe's own supported path out).
- [ ] PDF place as graphic + trace layer (also v0.5).
- [x] Template Analyzer (shipped 2026-10-15 for PDFs): pick 3 or more
      old bulletin PDFs; corpus consensus separates standing text from
      weekly slots and drafts a template to Accept, Edit, or Cancel.
      Design: docs/TEMPLATE-ANALYZER.md. (.pub / IDML / .obd inputs join
      when those importers land.)

## Later, only if earned
Print-shop phase (CMYK, spot, ICC, bleed/marks, PDF/X-4), CSV data merge
(name tags, certificates), Scribus import, IDML read-only import.

## Deployment track (Jacob, 2026-10-15; design in docs/DEPLOYMENT.md)
- [ ] Docker image for NAS / home-server use: same egui UI compiled to
      WebAssembly, tiny file server, `docker compose up` install.
- [ ] (Only with the server shape) local accounts, optional Google
      sign-in, and Owner / Editor / Viewer roles. Desktop stays
      login-free; the file format never carries accounts.
