# OpenBulletin: Tier-2 Conceptualization

## 0. Core Thesis

OpenBulletin is not "Scribus but prettier" and not "InDesign in Rust." It is **"last week's bulletin, safely turned into this week's bulletin, printed right the first time."**

Every feature decision should be tested against the volunteer at 4 p.m. on a Saturday. Pro features earn their way in only after that user is served.

---

## 1. Useful and Needful vs. Bloat

### (a) Must-have for the bulletin wedge (v0.1 → v1.0)

| Category | Feature | Why it's non-negotiable |
|---|---|---|
| Document setup | **Fold-aware presets.** Letter bi-fold, Letter tri-fold, Legal bi-fold (8.5×14 → 7×8.5), Tabloid bi-fold (11×17 → 8.5×11), Letter portrait multi-page. | Bulletins are defined by the fold, not the page size. The user edits *reader pages/panels*; the app owns the sheet. |
| Imposition | **Automatic printer spreads.** Bi-fold: outside = [p4 \| p1], inside = [p2 \| p3]. Tri-fold: the fold-in panel is 1/16–1/8" narrower. Saddle-stitch booklets in multiples of 4. | This is the #1 thing volunteers get wrong in Publisher. Owning it is the wedge. |
| Printing | **Duplex test-sheet wizard.** Print a 1-sheet "arrow test," ask "which way is the arrow?", and remember the answer per printer. | "Printed upside down on the back" is the most common bulletin failure. |
| Text | Text frames, **frame threading**, overflow indicator, columns within a frame. | Announcements spill. Overflow must be loud, not silent. |
| Text | **Tab stops with right-aligned dot leaders**. | The order of service *is* "Hymn of Praise ........ 432." Without this, users fake it with spaces and layouts break. |
| Text | Paragraph and character styles (named, based-on, next-style). | Keeps "don't break the layout" true when someone else edits. |
| Text | Spell check (`spellbook`, Hunspell-compatible). Find/replace. | Secretaries depend on red squiggles. |
| Text | **Paste as text matching current style** by default. Rich paste (bold/italic/links only) on demand. | Announcements arrive via email and Word. Pasted fonts are the #2 layout-breaker. |
| Content | **Fields.** `{Sunday.date}`, `{Sunday.name}`, `{Church.name}`, service times, giving URL. | The weekly delta is mostly fields. |
| Content | **"Next Sunday" command.** Clones the document, advances the date, clears frames marked *weekly*, keeps *standing* content. | This replaces the copy-the-file ritual with a safe one. It is the signature feature. |
| Content | **Church Profile.** Publisher's "Business Information" equivalent: logo, address, times, website, giving link, auto-filled into templates. | Direct refugee mapping; zero learning cost. |
| Content | **Blocks library.** Publisher's "Building Blocks": Doxology, Communion liturgy, standing announcements, nursery notice. | Recurring liturgy is reusable content, not retyping. |
| Content | Built-in **QR code** generator (giving, website, livestream). | Every bulletin now has one. Trivial with the `qrcode` crate. |
| Graphics | JPEG/PNG/SVG place, crop, fit/fill. Rectangles, lines, rounded rects, ellipses. Fill/stroke. | Logos, cover photos, dividers. |
| Graphics | Text wrap around bounding boxes. | Cover image with text beside it. |
| Canvas | Guides, margins, **visible fold lines and a no-go fold zone**, smart snapping, zoom. | The fold zone is where text gets eaten. |
| Safety | Unlimited undo/redo, autosave, crash recovery, atomic saves, rolling backups. | One lost Saturday-night file kills trust permanently. |
| Output | PDF export: reader view for email/website, imposed for print. | Bulletins are also emailed and posted online. |
| Output | **Preflight-lite.** Overset text, missing fonts, images under 150 dpi at placed size, content in the fold zone or printer dead margin (~0.25"). | Publisher's "Design Checker," reduced to the things that actually bite. |
| Fonts | Bundled OFL families: Atkinson Hyperlegible, Source Serif 4, Source Sans 3, plus one display face. | Large readable type, and identical rendering on the pastor's laptop. |
| Templates | 10–20 bulletin and flyer templates designed with **large-print defaults** (≥12pt body). | Template-first is the default UX. |

### (b) Must-have eventually for a credible Publisher replacement

| Feature | Phase |
|---|---|
| Master pages (simplified "repeating items"), page numbers, sections | v0.5 (simple), v1.0 (full) |
| Simple tables (rows/cols, borders, cell padding) | v0.5 |
| Liturgical calendar fields (computus, seasons, Sunday names; pluggable per tradition) | v0.5 |
| PDF place as graphic (including "trace layer" for rebuilding old bulletins) | v0.5 |
| Native OS print dialog per platform | v0.5 |
| Newsletter features: facing pages, **"continued on/from page X" jump lines**, columns across pages | v1.0 |
| Drop caps, hyphenation (`hypher`), H&J controls, widow/orphan control | v1.0 |
| Large-print companion export (same stories reflowed to 8.5×11 at 16–18pt) | v1.0 |
| Tagged/accessible PDF (PDF/UA, via krilla) | v1.0 |
| Best-effort `.pub` rescue import | v1.0 |
| Template packs as shareable files (diocese/denomination distribution) | v1.0 |
| Scribus `.sla` import | v1.x |
| Bleed, crop marks, CMYK, spot colors, ICC, PDF/X-4 ("print-shop phase") | v1.x–v2 |
| CSV data merge (name tags, labels, certificates; not email) | v2 |
| Text on a path, object styles, baseline grid | v2 |
| IDML import (read-only, best-effort) | v2+ |
| Simple TOC ("In this issue") | v2 |

### (c) Never build, or defer indefinitely, with public defense

| Feature | Verdict | Public reasoning |
|---|---|---|
| Mailings tab / email merge / "send as email" | **Never** | Email is the email client's job. It brings SMTP, contacts, and privacy liabilities. |
| Web publishing, HTML export, web banners | **Never** | Microsoft itself removed these from Publisher. Websites are a different product. |
| EPUB, fixed-layout EPUB, interactive PDF (forms, buttons, animation) | **Never** | No bulletin user needs them. They roughly double the output-path test matrix. |
| GREP styles, nested styles, conditional text | **Defer indefinitely** | Pro-typesetter power with a huge edge-case surface. Paragraph/character styles cover 99% of users. |
| Footnotes/endnotes, cross-references, index, book files | **Defer indefinitely** ("long-doc phase, if ever") | This is InDesign-class territory. Typst and LaTeX already serve long documents well. |
| Track changes, real-time collaboration, cloud sync | **Never in core** | Files on a shared drive or Dropbox already work. We will not run servers for churches. |
| Built-in stock photos / clip art / online pictures | **Never** | It creates licensing liability we cannot police. Link out to openly licensed sources. |
| Bundled hymn lyrics or modern Scripture translations | **Never** | These are copyrighted (CCLI, NRSV, ESV and others). Ship only public-domain texts (KJV, WEB) and readings *references*. |
| AI writing/generation | **Never in core** | No server, no cost, no privacy questions. Users can paste from whatever tool they like. |
| `.pub` export, `.indd` reading | **Never** | These are undocumented proprietary binaries; writing `.pub` would extend a dead format. |
| IDML export | **Never** (revisit only with a sponsor) | Round-trip fidelity is a promise we can't keep. |
| Full PDF editing | **Never** | PDF is an output format. "Edit any PDF" is a separate, bottomless product. |
| WordArt gallery | **Never** (outline/shadow/text-on-path later) | Effects galleries age badly. Basic text effects cover the real need. |
| Scripting / plugin marketplace | **Defer to v2+** | A stable API freezes internals too early. Ship a CLI first. |

---

## 2. Architecture

### Committed stack

| Layer | Choice | Plain tradeoff |
|---|---|---|
| GUI shell | **egui/eframe**, **glow (OpenGL) backend by default**, wgpu optional; `egui_dock` for panels | **Pros:** fastest to build, one Rust codebase, full control to draw a ribbon *and* panels, AccessKit integration, proven in tool apps (Rerun), and it runs on old church-office Intel iGPUs. **Cons:** non-native look, partial accessibility, and retained UI state must be designed deliberately. We accept these. |
| Canvas rendering | **CPU tile rasterizer** (`vello_cpu`, or `tiny-skia` if `vello_cpu` isn't ready) to textures shown in egui. GPU vello later as an optimization. | A 5.5×8.5 page is trivially CPU-renderable. It is deterministic and avoids GPU driver roulette on 2015 Dell desktops. |
| Text | **parley** (HarfRust shaping, fontique discovery, bidi, styled spans) for paragraph shaping and line breaking. **Our own flow engine** for threading, wrap, tabs/leaders, and frame distribution. `hypher` for hyphenation. | No crate does DTP flow; we must own it. Choose parley over cosmic-text because it is rich-text-first and backed by Linebender. **Verify in the spike that its line-by-line breaker accepts per-line widths.** |
| Display list | Our own renderer-neutral scene (paths, glyph runs with font ID + glyph IDs + positions, images, colors) | **Invariant:** the screen raster and the PDF consume the *same* display list. Same glyph positions means WYSIWYG holds. |
| PDF | **krilla** (Typst's PDF crate): font subsetting, tagging, PDF/A and PDF/UA. `pdf-writer` underneath if PDF/X output intents need custom work. **hayro** for reading/rendering placed PDFs. | krilla is the best-maintained high-level PDF writer in Rust. PDF/X support is unconfirmed, so plan to extend it, upstream changes, or drop to `pdf-writer`. |
| Undo | **Persistent immutable document** (`imbl` structural sharing). Undo stack = document snapshots. Text edits are coalesced by time/word. | Far less bug-prone for a small team than inverse operations. Memory is fine at bulletin scale. |
| Units | Geometry as **i64 in 1/1000 pt**; f64 only inside layout | Deterministic snapping, stable diffs, no float drift across saves. |

### Why not the others

- **Tauri:** you end up with either two renderers (JS canvas on screen, Rust PDF on export, so fidelity diverges) or a fragile native-surface/webview compositing bridge. You also add a JS toolchain and WebKitGTK quirks on Linux. Its accessibility is genuinely better, which is why the shell is kept thin and swappable.
- **iced:** pre-1.0 churn, weaker accessibility, and no advantage for a custom-canvas app.
- **Slint:** the GPLv3-or-commercial license constrains contributors and downstream redistribution, and the DSL adds friction.
- **GPUI:** not library-ready and thin on documentation outside Zed.
- **Typst as the engine:** it is a batch markup compiler, not an interactive frame editor. Reuse its *crates* (krilla, hypher, hayro), not its model.

### Workspace structure

```
openbulletin/
  crates/
    ob-model      # pages, panels, frames, stories, styles, fields, assets. No GUI/IO deps.
    ob-edit       # commands, transactions, undo (imbl snapshots), selection model
    ob-text       # rich text spans, parley shaping, flow across threaded frames, tabs/leaders, hyphenation
    ob-layout     # page/panel geometry, wrap, masters, field resolution -> display list
    ob-scene      # renderer-neutral display list
    ob-raster     # CPU tile renderer (canvas, thumbnails, golden tests)
    ob-impose     # pure functions: reader pages -> sheets (bi/tri-fold, saddle-stitch). Heavily tested.
    ob-pdf        # krilla export: reader PDF, imposed PDF, later PDF/UA, PDF/X
    ob-format     # zip+JSON container, schema versions, migrations, atomic save
    ob-fonts      # bundled fonts, discovery, substitution policy, font hashing
    ob-liturgy    # computus, seasons, lectionary references (data-driven, per tradition)
    ob-import-pub # (v1.0) out-of-process helper wrapping libmspub
    ob-import-sla # (v1.x)
    ob-app        # egui shell: tools, panels, keymaps, workspaces, print
    ob-cli        # headless: render, export, impose, "next-sunday", golden-image CI
  templates/      # template packs as native files
  tests/golden/   # fixture docs -> expected PNG/PDF hashes
```

**Rule:** everything except `ob-app` must build and test headless. `ob-cli` is the CI oracle and protects you if the GUI framework ever needs to change.

### Prototype first: the "Hymn of the Day" spike (4–6 weeks)

The project-killing risk is **interactive text editing across threaded frames with print-identical output.** Build exactly this:

1. Two threaded text frames on a 5.5×8.5 panel, with one paragraph style and a right-tab dot leader.
2. Caret movement and selection across the frame boundary, typing, deletion, and IME (winit IME events into our own canvas editor).
3. An overflow indicator, plus undo/redo with coalescing.
4. Export imposed PDF through krilla, rasterize it with hayro, and **pixel-diff it against the canvas raster**.
5. Run it on a 10-year-old Windows 10 iGPU machine.

**Kill/continue criterion:** if a 1,000-word story doesn't edit at 60 fps with stable line breaks and a PDF that matches the screen, re-evaluate the text stack *before* writing any UI chrome.

A parallel half-week task: print the bi-fold imposition on three real office copiers (one HP, one Canon or Ricoh, one Brother) to validate the duplex wizard.

---

## 3. UX Modes

### Verdict

**Three full personalities are a trap. A thin version is a real feature.**

Three UIs triple the test matrix, the screenshots, the docs, and the tutorials. Worse, they break the *phone-help* scenario that defines this user base. When the pastor's spouse says "click Insert, then Text Box," both people must see the same thing. Inherited files travel hand-to-hand; so do instructions.

### What to build instead

1. **One canonical UI, two workspace densities:**
   - **Simple** (default): template gallery on launch, a compact contextual toolbar, a Properties panel, the Blocks library, and the Fields panel.
   - **Full:** dockable panels for Pages, Masters, Styles, Layers, Swatches, and Preflight, plus measurement fields in pts/picas.
2. **Keymap presets:** *OpenBulletin* (default), *Publisher*, *InDesign*.
   - Only the InDesign preset enables single-letter tool keys (V, A, T) when not editing text.
   - The user can mix and match per command.
3. **Vocabulary aliases in search, not in menus:**
   - The command palette and Help search accept "Text Box" (Publisher), "Text Frame" (InDesign), "Building Blocks," "Business Information," "Master Page," and "Parent Page."
   - All of them resolve to the canonical command.
   - Tooltips show aliases, e.g. "Text Frame (Publisher: Text Box)."
4. **Defaults per preset:** units (inches vs. picas), new-document flow (gallery vs. blank-document dialog), and nudge distances.

### What may vary (stored in user prefs, never in the file)

- Keymap.
- Workspace layout (which panels open, docking positions).
- Default units and measurement precision.
- Launch flow (gallery vs. blank).
- Onboarding hints and tooltip alias display.

### What must never vary

- **File format and its contents.** A document never records which mode created it.
- **Menu tree and menu labels.** The canonical vocabulary is the same for everyone.
- **Command IDs and behavior,** including undo granularity, snapping semantics, and paste behavior.
- **Rendering and export output.** It is byte-identical regardless of mode.
- **Style names and inheritance semantics.**
- **Document-level settings** (margins, bleed, fold zones). These live in the file, not in a mode.

---

## 4. File Format and Interop

### Native format

A **zip container** (extension e.g. `.obd`) with the following contents:

```
manifest.json      # format_version, min_reader_version, app_version, generator
document.json      # model: pages, panels, frames, stories (spans + style refs), styles, fields
assets/<sha256>.*  # images/SVG/PDF, content-addressed, deduplicated
fonts/<sha256>.*   # embedded only when license permits (OFL/Apache); others referenced by name + hash + metrics
preview.png        # thumbnail for gallery and OS file browsers
```

Design commitments:

- **Self-contained by default.** "Pack and Go" is built in, so an inherited file opens identically on the next volunteer's machine.
- **JSON** so files are diffable, scriptable, and survivable. Reject a bespoke binary format. Reject SQLite: it is overkill and complicates atomic copies via email or USB.
- **Migration tests from day one.** Every released format version gets a fixture in CI that must open in every later version.
- **Saving is safe.** Writes go to a temp file, then fsync and rename. The last 10 versions are kept in a sidecar backup folder.

### Interop stance

| Format | Stance | Phase |
|---|---|---|
| **.pub** | **Rescue import, best-effort.** Extract frames, text, images, and page geometry via libmspub (MPL-2.0) in an **out-of-process helper** so crashes don't take down the app. Map the result to the closest native template. | v1.0 |
| **.pub (now)** | **Rescue guide + trace layer.** Perpetual Publisher installs still *run* after support ends, so instruct users to export PDF and copy text now. Place that PDF as a locked background layer and rebuild on top in an afternoon. | v0.5 |
| **PDF** | **Place as graphic: yes.** Edit: **never.** | v0.5 |
| **SVG** | Place as graphic (`usvg`). Export a single page as SVG later if requested. | v0.5 |
| **Scribus .sla** | Import is feasible because it's documented-ish XML. It helps a real community, but not the wedge. | v1.x |
| **IDML** | Read-only, best-effort import. No export. | v2+ |
| **.indd** | Never. | — |
| **Word/RTF/HTML** | Clipboard paste with basic formatting; `.docx` text import for announcements. | v0.5 |

### Honest year-1 promise to refugees

> "OpenBulletin will not open your old Publisher file perfectly, and we won't pretend it will. It will give you a bulletin template that matches yours in an afternoon, a tool to trace over a PDF of your old one, and a weekly workflow that's faster and safer than copying last week's file. By v1.0 we'll import the text and pictures from most .pub files so you're not retyping."

Never claim "opens Publisher files" in marketing. Use the phrase "rescues content from Publisher files."

---

## 5. MVP and Roadmap

### v0.1: "The Sunday Test"

**Acceptance criterion:** a real church secretary, with no help from us, produces and prints a real Sunday's letter bi-fold bulletin, then uses "Next Sunday" for the following week.

Scope:

- **Document types:** Letter bi-fold and Letter portrait multi-page.
- **Templates:** 3 bi-fold and 2 portrait.
- **Text:** reader-panel editing; text frames with threading and an overflow indicator; 6 preset paragraph styles (editable, and users can create new ones); bold/italic/size; tabs with dot leaders.
- **Graphics:** PNG/JPEG place, crop, and fit; QR code; rectangle and line.
- **Fields:** date, Sunday name (manual entry), church name. Plus the **Next Sunday** command.
- **Safety:** undo/redo, autosave, crash recovery, atomic save.
- **Output:** reader PDF and imposed PDF. "Print" opens the imposed PDF in the system viewer, accompanied by the **duplex test-sheet wizard** and its remembered setting.
- **Fonts:** the bundled readable font set.
- **Platforms:**
  - Windows first (church offices skew Windows), signed via Azure Trusted Signing.
  - macOS notarized.
  - Linux via Flatpak and AppImage.
- **Explicitly out:** tri-fold, masters, spell check, tables, text wrap, `.pub`, CMYK.

### v0.5: "The Church Office"

*Independently useful:* covers every common bulletin and flyer format and replaces Publisher for most single-sheet work.

- **Formats:** tri-fold, Legal and Tabloid bi-fold, saddle-stitch booklets (8/12/16 pp).
- **Content:**
  - Church Profile.
  - Blocks library.
  - Liturgical calendar fields: auto Sunday name and lectionary references for at least RCL and Roman Catholic; data packs for others.
  - Simple master pages and simple tables.
- **Text and graphics:** spell check, find/replace, rich paste and `.docx` text import, text wrap (bounding box), SVG/PDF place, and the trace layer.
- **Quality and output:** preflight-lite, native print dialogs (macOS NSPrintOperation, Linux CUPS/GTK, Windows GDI raster at 600 dpi).
- **UX:** Publisher keymap plus alias search.
- **Templates:** 25+ covering bulletins, flyers, and simple newsletters.

### v1.0: "Credible Publisher Replacement for Small Organizations"

*Independently useful:* newsletters and brochures with a stability promise.

- **Newsletter layout:** facing pages, jump lines, sections and page numbers, full masters, full style cascade.
- **Typography:** drop caps, hyphenation and H&J, widows/orphans.
- **Tables:** full tables.
- **Output:** large-print companion export and tagged PDF/UA.
- **Interop:** `.pub` rescue import.
- **Sharing:** shareable template packs.
- **UX:** InDesign keymap and Full workspace.
- **Stability:** a **file-format stability guarantee** (every 1.x opens every 1.x file).

After v1.0, the next phase is print-shop work (CMYK, spot, ICC, bleed and crop marks, PDF/X-4), then data merge.

---

## 6. Risks: Top 5 Ways This Dies

| # | Death | Earliest observable signal | Mitigation |
|---|---|---|---|
| 1 | **Text-engine sinkhole.** Threading, editing, IME, and selection consume the project. | 3 months in, the spike still has caret or selection bugs across frames, or line breaks shift between screen and PDF. Text bugs exceed 50% of open issues. | Run the spike first with a hard kill criterion. Keep greedy line breaking (no Knuth-Plass) until v1.0. Run golden-image CI on every PR. |
| 2 | **Wedge abandonment / pro scope creep.** Contributors chase InDesign parity. | Roadmap discussions are dominated by PDF/X, GREP, or footnotes while fewer than 10 real churches use it weekly. Contributors stop opening the app with a bulletin template. | Publish the bucket (c) list as policy. Treat "Does a church secretary need this before v1.0?" as a merge gate for features. Recruit 5–10 pilot churches before v0.1. |
| 3 | **It never reaches the secretary.** Distribution and install trust fail. | Downloads without repeat weekly opens (opt-in ping or pilot check-ins). Support threads about "Windows protected your PC" or "can't be opened because Apple…" | Code-sign from the first public build. Use a one-click installer and silent auto-update. Distribute through denominational networks and diocesan communications offices, not just GitHub. |
| 4 | **A Saturday-night trust failure.** Data loss, or the print doesn't match the screen. | Any single report of a lost or corrupted file. Any "printed upside down," "margins cut off," or "font changed" report. | Atomic saves, rolling backups, and recovery. A self-contained format with embedded fonts. The duplex wizard. Defaulting to "actual size," never "fit to page." Every such report is a release blocker. |
| 5 | **Volunteer burnout and dependency churn.** Bus factor of one; egui or parley breaking changes eat release cycles. | One person authors more than 70% of commits for two consecutive quarters. A release slips because of a dependency upgrade. | Keep the GUI shell thin and the headless core well-tested. Pin dependencies and upgrade on a schedule. Recruit a second maintainer per core crate before v0.5. Seek fiscal hosting (e.g., Open Collective) and a denominational sponsor early. |

**Bottom line:** win the Saturday-afternoon bulletin. Bi-fold, Next Sunday, dot leaders, and a duplex print that comes out right the first time are the product. Everything else must earn its way in.