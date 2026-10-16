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
- [x] Desktop shell: inline text editing, save/open, autosave + recovery (shipped 2026-10-15; true shaped caret editing on the canvas, in the real typeface, shipped with the E1 build)
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
`.pub` rescue import (shipped early, 2026-10-15), shareable template packs, file-format stability
guarantee (every 1.x opens every 1.x file).

## Import and conversion track (Jacob, 2026-10-15)
Imports are rescues with honest fidelity labels, never magic (PRINCIPLES.md):
- [x] Native .obd (v1 since 2026-10-16: zip container with
      content-addressed assets and rights records; v0 files
      still open and migrate) — ours, full fidelity by
      definition.
- [x] PDF rescue (shipped 2026-10-15): open an old bulletin PDF in the
      app; words come back as editable boxes with Good / Check this /
      Picture only labels, and each old page stays underneath as a
      tracing picture until removed. Accept / Edit / Cancel review.
- [x] .pub rescue import (shipped 2026-10-15): with LibreOffice
      installed, the bridge converts the Publisher file to PDF
      headlessly and the PDF rescue takes over (words, boxes,
      tracing pictures, the same review). Without it, our own
      native reader (no libmspub code) reads Publisher 2002+
      files directly: words in their boxes, masters, tables,
      and WordArt marked Check this; pictures counted and their
      places marked, bytes not yet carried. Design and measures:
      docs/EXTRACT.md.
- [x] IDML import (shipped 2026-10-15): words, frames, sizes, and fonts
      out of InDesign's interchange format, through the same draft
      review as PDF rescue; placed pictures do not come over yet.
      (.indd itself is a closed binary and stays out of scope.)
- [x] IDML export (shipped 2026-10-15): any bulletin saves as an
      IDML package for InDesign from the Print step's one export
      door ("Save a copy for another program...", format chosen
      in the save dialog) or the CLI's export-idml; the export
      report states in plain words what was not carried.
- [x] Scribus import (shipped 2026-10-15): .sla files open through
      the same one door as everything else; words, boxes, and
      fonts as a draft to review; picture frames counted and
      named, not carried.
- [x] Scribus export (shipped 2026-10-15): any bulletin saves as a
      .sla file for Scribus from the same export door ("Save a
      copy for another program...", format chosen in the save
      dialog) or the CLI's export-sla; the export report states
      in plain words what was not carried. The import/export
      matrix is now complete: in and out for .obd, PDF, IDML,
      and Scribus, in only for Publisher.
- [ ] PDF place as graphic + trace layer (also v0.5).
- [x] Template Analyzer (shipped 2026-10-15): pick 3 or more old
      bulletins; corpus consensus separates standing text from
      weekly slots and drafts a template to Accept, Edit, or Cancel.
      Design: docs/TEMPLATE-ANALYZER.md. Inputs are one mixed pick
      (PDF / .pub / IDML / .sla / .obd) since 2026-10-15: learning
      needs at least three bulletins that count, structured files
      one-for-one and PDFs only in a group of three or more, and
      the app states the rule and any shortfall in plain words.

## Later, only if earned
Print-shop phase (CMYK, spot, ICC, bleed/marks, PDF/X-4), CSV data merge
(name tags, certificates).

## Deployment track (Jacob, 2026-10-15; design in docs/DEPLOYMENT.md)
- [ ] Docker image for NAS / home-server use: same egui UI compiled to
      WebAssembly, tiny file server, `docker compose up` install.
- [ ] (Only with the server shape) local accounts, optional Google
      sign-in, and Owner / Editor / Viewer roles. Desktop stays
      login-free; the file format never carries accounts.
