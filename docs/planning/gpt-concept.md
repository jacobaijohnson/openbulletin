# OpenBulletin: Build the Weekly Workflow, Not a Smaller InDesign

## Recommendation

Build an **offline, template-first publishing application with a serious document engine**. Its defining feature should be **“prepare next Sunday without damaging the layout,”** not a ribbon, unlimited tools, or compatibility claims.

Keep **OpenBulletin** for the church-focused launch. Do not market v1 as a complete Publisher replacement. Earn that description through later releases.

Use one editing model, with configurable workspaces—not three product personalities.

---

## 1. Useful and Needful vs. Bloat

### Phase definitions

- **v0.1:** A dependable four-page bi-fold bulletin.
- **v0.5:** Repeatable weekly production, additional formats, and better reuse.
- **v1.0:** A complete bulletin tool suitable for nontechnical volunteers.
- **P2:** Credible Publisher-class replacement: newsletters, flyers, brochures, and modest booklets.
- **Indefinite:** Outside the product commitment; reconsider only with demonstrated demand and funded maintenance.

### Feature audit

| Feature category | Bulletin commitment | Publisher-replacement commitment | Deliberately excluded or indefinitely deferred |
|---|---|---|---|
| **Document setup** | v0.1: page size, orientation, margins, four-page bi-fold preset. v0.5: tri-fold and portrait documents. | P2: arbitrary document sizes, bleed, facing pages, reusable presets. | Mixed-orientation and elaborate section geometry until demonstrated demand. |
| **Frames and objects** | v0.1: text frames, images, rectangles, lines, selection, move/resize, stacking, alignment, snapping, numeric dimensions. | P2: grouping, distribution, rotation, layers, reusable objects. | Illustrator-class path editing, mesh gradients, complex effects. They create a second application. |
| **Typography** | v0.1: paragraph/character styles, alignment, spacing, line height, lists, basic tabs, explicit font fallback. v1: discretionary hyphenation and essential OpenType controls. | P2: advanced tabs/leaders, drop caps, baseline grids, keep rules, richer OpenType controls. | Variable-font animation, optical-margin refinement, exhaustive typographic automation until core layout is mature. |
| **Text flow** | v0.1: fixed frames with unmistakable overset warnings. v0.5: linked frames and predictable continuation. | P2: automatic page creation and more sophisticated pagination. | InDesign-scale long-document composition in the initial product line. |
| **Master pages / parent pages** | v0.5: reusable page backgrounds, headers, footers, page numbers, protected furniture. | P2: multiple parent-page sets and controlled overrides. | Deep inheritance hierarchies. They are difficult to explain and debug. |
| **Tables** | v1: simple editable tables with column widths, cell padding, borders, and header styling. | P2: tables continuing across pages, repeated headers, reusable table/cell styles. | Spreadsheet formulas, charts, and embedded spreadsheet engines. |
| **Images** | v0.1: PNG/JPEG, crop, fit/fill, rotation, resolution warnings. v1: reliable SVG placement. | P2: clipping, restrained text wrap, broader image support. | Photo retouching, background-removal services, image generation. |
| **Templates / building blocks** | v0.1: several polished bulletin templates. v0.5: reusable announcement/contact/giving blocks. | P2: broader galleries and user-authored building-block libraries. | A template marketplace or cloud asset subscription. |
| **Recurring content** | v0.5: “Create next issue,” date fields, editable issue variables, protected layout. | P2: reusable document metadata and richer field substitution. | Automatic sermon writing, theological recommendations, or mandatory lectionary services. Content neutrality matters. |
| **Print and imposition** | v0.1: reading-order PDF plus four-page printer spread. v0.5: tri-fold sheet output and duplex test sheet. v1: modest booklet imposition. | P2: booklet creep, signature options, commercial-printer handoff. | Production RIPs, arbitrary gang-run optimization, finishing-machine control. |
| **Preflight** | v0.1: missing fonts/assets, overset, low-resolution images, printable-area risks. | P2: bleed, transparency/color-space checks and professional export profiles. | Claiming certification without a validation pipeline. |
| **Color and PDF** | v0.1–v1: dependable RGB PDF for office printing, embedded fonts, vector text. | P2: ICC-aware workflows; CMYK/spot and a narrowly specified PDF/X target only after specialist work. | Every PDF standard and every separation workflow. Scribus remains the professional-print fallback. |
| **Mail merge / data merge** | Not needed for the wedge. | P2: CSV-driven text/image fields for labels, certificates, directories, and flyers. | Publisher-style **Mailings tab**, address-book management, postal automation, email campaigns. |
| **Find and replace** | v0.5: text search and replace. | P2: style-aware search and saved queries. | **GREP styles** and scriptable composition rules: powerful, but specialist complexity with little wedge value. |
| **References / long documents** | v0.5: page numbers; manual contents are sufficient. | P2: basic automatic contents and bookmarks. | **Footnotes/endnotes**, indexes, cross-reference networks, multi-file books, scholarly publishing. |
| **Interactive publishing** | v0.5: clickable links and QR codes. | P2: PDF bookmarks. | **EPUB export**, HTML publishing, web banners, interactive forms, animations, presentations. They require different layout and accessibility models. |
| **Automation / extensions** | v1: documented format and command-line PDF export. | P2: constrained batch operations; extension API only after engine stabilization. | Early scripting ecosystem, arbitrary plugins, embedded macros. |
| **Collaboration** | v0.1: local files, autosave, recovery, clear document ownership. | P2: optional packaged handoff and external version-control friendliness. | Real-time multiplayer, hosted accounts, approval portals, social publishing. |
| **Accessibility** | v0.1: keyboard-operated editing, accessible content list, visible focus, scalable UI. v1: tested screen-reader workflow. | P2: tagged PDF with deliberate reading order and alternate text. | Do not equate an accessible application with accessible exported PDFs. They are separate commitments. |

**Public defense:** These exclusions preserve reliable weekly production. The team is not rejecting professional features; it is rejecting unrelated markets and unfunded promises.

---

## 2. Architecture: Rust Owns the Document, Not Necessarily Every Widget

### Concrete stack

| Layer | Recommendation | Plain tradeoff |
|---|---|---|
| **Desktop shell and UI** | **Tauri 2 + Svelte/TypeScript**, using semantic HTML controls | Accept a small web frontend. Native accessibility, forms, menus, and volunteer-friendly interaction matter more than “all Rust.” Linux webview behavior needs early testing. |
| **Canvas renderer** | **`skia-safe`**, initially CPU-rendered viewport tiles, displayed by the web UI | Mature raster/vector machinery, but heavy builds and native dependencies. Avoid GPU/webview integration complexity in the first release. |
| **Text composition** | **Parley**, with its font/shaping stack | Appropriate foundation for styled paragraph layout. It does **not** supply a finished DTP composer: frame flow, hyphenation policy, overset, pagination, and style resolution remain our responsibility. |
| **PDF output** | **`pdf-writer`** over the engine’s positioned display list | Low-level and labor-intensive, but avoids a second layout engine. Font embedding, subsetting, Unicode mappings, clipping, and image handling must be explicitly implemented and tested. |
| **Persistence** | Versioned ZIP package with JSON manifests and embedded assets | Inspectable and portable; requires careful atomic saving, migrations, and archive security. |

Do **not** adopt Typst as the hidden document model. It is valuable technology, but translating interactive frame editing into a separate composition language would put the product’s central behavior behind another abstraction.

Do **not** begin with egui. Its development speed is attractive, but accessible text editing and desktop forms are more important here than a fast custom-drawn prototype.

### Critical implementation rules

1. **One layout authority.** Rust computes every line break, glyph position, frame overflow, object bound, and print transformation.
2. **The browser never lays out document text.** HTML controls edit content; they do not determine printed geometry.
3. **Skia never independently reshapes text.** Render the positioned glyph runs produced by the text engine, using the same font instances and variation settings.
4. **PDF consumes the same layout result.** Different output backends, identical geometry.
5. **Layout is revisioned and incremental.** Editing one announcement should invalidate its story and dependents—not every page.
6. **Undo records document transactions.** It does not record mouse events or frontend component state.

An “infinite-zoom” canvas means vector document coordinates and rerendering at the requested scale—not an enormous backing bitmap. Send visible tiles through a binary/custom-protocol path, discard stale revisions, and benchmark the transport before building elaborate UI.

Use document units independent of screens—points are suitable—and define rounding rules for serialization and layout.

### Workspace structure

```text
crates/
  document/       # schema, IDs, styles, assets, migrations
  commands/       # transactions, undo/redo, validation
  text/           # fonts, shaping, paragraph composition
  layout/         # frames, stories, pagination, imposition
  render/         # display list, Skia tiles, hit-test geometry
  pdf/            # fonts, resources, vector export
  package/        # native-file read/write, recovery
  preflight/      # checks and actionable diagnostics
  cli/            # validate and export without the GUI
  desktop/        # Tauri integration
ui/               # Svelte application
fixtures/         # fonts, documents, expected layouts/PDFs
```

The engine must run headlessly. GUI testing alone cannot protect print output.

### Prototype first: the typography-to-paper spike

Before templates or ribbons, build a two-page document containing mixed styles, accented text, bidirectional text, lists, a linked story, images, and deliberate overflow.

Prove:

- Caret placement, selection, undo, and input-method composition work.
- Screen and PDF line breaks and glyph positions agree.
- Export embeds permitted fonts and preserves searchable text.
- Editing remains responsive with a realistic bulletin.
- The same file exports consistently on all three operating systems.
- A printed four-page test folds into the correct order.

**Gate:** Do not proceed until the output is correct and the editor is usable. A beautiful shell around unreliable text is not a publishing product.

---

## 3. UX Modes: Reject Three Personalities; Offer Workspaces

The proposed personalities are a trap if they imply three applications, three command systems, or partial InDesign compatibility.

Ship **one default workspace: Bulletin**. Later add **Layout**, plus migration aids for Publisher users.

### What may change

- Visible panels and their arrangement.
- Toolbar density and command grouping.
- Start-screen templates and help examples.
- Optional shortcut presets, with conflicts disclosed.
- Temporary onboarding aliases: “Master page—also called Parent page.”

The Bulletin workspace emphasizes issue content, templates, warnings, and printing. Layout exposes styles, parent pages, objects, guides, and stories.

### What must never change

- Canonical command names and command identifiers.
- File format, document semantics, units, and geometry.
- Selection behavior, snapping rules, style inheritance, text flow, or export.
- Undo history, autosave, validation, and preflight.
- Object identity and protection state.
- Features already present in a document.

A workspace switch must never reflow a page, silently alter defaults, or unlock protected objects.

Keep **application preferences** separate from **document settings**. Shortcut presets are preferences; margins and styles belong to the file.

Do not ship “InDesign mode.” It implies behavioral familiarity and scope the product cannot honestly supply. Experienced designers can use Layout without a misleading badge.

---

## 4. Native Format and Interoperability

### Native format

Use a versioned ZIP-based **`.obdoc`** package:

```text
manifest.json
document.json
assets/<content-hash>
fonts/<content-hash>       # only when embedding is permitted
preview.png
```

Store stable IDs, explicit page geometry, separate stories and frames, named styles, parent-page relationships, and issue fields. Store content and structure—not derived glyph positions as authoritative data.

Use deterministic serialization, migrations with fixtures, atomic replacement, and crash recovery. Handle archive limits, path traversal, corrupt images, and unsupported versions safely.

Bundle a small, permissively licensed font family set. Record exact font identity; show missing-font substitution explicitly. Do not promise portability while silently replacing fonts.

### Interoperability stance

| Format | Commitment |
|---|---|
| **`.pub`** | **No faithful native import promise in year 1.** Provide a documented conversion/rebuild workflow using existing tools and PDF references. Do not build a proprietary-format parser as the wedge’s critical path. |
| **IDML** | Defer to P2. Begin with a documented subset for pages, basic frames, text, images, and styles; issue a conversion report. No INDD support promise. |
| **PDF** | Place PDF pages as non-editable artwork in v1. Raster preview is acceptable if export preserves the placed vector page. Editable PDF reconstruction is indefinitely deferred. |
| **SVG** | v1 placement through a supported, sanitized subset; preserve vectors where feasible. Arbitrary SVG is not a native object-editing format. |
| **Scribus SLA** | P2 best-effort import of basic objects/styles only, if users justify it. No lossless round trip or use of SLA as our native format. |
| **PNG/JPEG** | v0.1 import. PDF is the primary distribution output; image exports come later. |

### Honest year-one promise

> “Create and reuse reliable local bulletins, print correctly, and export PDFs. Existing Publisher files may require reconstruction; we provide migration guidance, not guaranteed conversion.”

Offer a **rebuild-from-reference workflow**: place an exported PDF as a locked tracing/reference layer, recreate editable content, then remove the reference. Label it reconstruction, not import.

Inherited-file migration deserves documentation and practical help, but it must not consume the engine team.

---

## 5. Smallest Usable Release and Roadmap

### v0.1 — One real Sunday

A secretary can:

1. Choose one of several readable four-page bi-fold templates.
2. Enter the date, service order, sermon details, announcements, and contact information.
3. Replace a logo or photograph.
4. Save locally and recover after a crash.
5. See overset and missing-font warnings.
6. Export reading-order PDF and correctly imposed Letter-landscape PDF.
7. Print through the operating system’s PDF viewer and fold a verified test.

Include essential object editing, text styles, keyboard navigation, and undo/redo. Template furniture is protected by default, with an explicit unlock action. Never silently shrink text to make it fit.

**Exclude:** tri-fold, linked-frame automation, tables, professional color, direct printer-driver management, and legacy import.

Acceptance requires several churches to produce actual Sunday bulletins—not merely approve screenshots.

### v0.5 — Repeatable production

Add:

- “Create next issue” with explicit carry-forward content.
- Issue fields and reusable content blocks.
- Linked text frames.
- Parent pages and page numbers.
- Tri-fold and portrait multi-page documents.
- Find/replace, hyperlinks, QR codes.
- Improved recovery and packaged handoff.
- Printer calibration/test sheet and persistent per-printer notes.

Tri-fold uses **panel geometry**, not booklet imposition. Support fold-specific panel widths and validate folding order.

This release is independently useful because it reduces weekly rework and supports more church document types.

### v1.0 — Dependable volunteer handoff

Add:

- Simple tables.
- Modest multi-sheet booklet imposition with padding pages.
- Hyphenation and stronger typography controls.
- SVG and placed PDF artwork.
- Comprehensive preflight.
- Tested screen-reader and keyboard workflows.
- Signed installers, migration stability, documentation, and predictable updates.
- Headless validation/export.

A new volunteer must be able to inherit the file and finish without a designer supervising them.

**P2 follows usage evidence, not calendar pressure:** general brochures/newsletters, data merge, richer tables, commercial-print capabilities, and selected interchange formats.

---

## 6. Top Five Failure Modes

| Failure mode | Earliest observable signal | Required response |
|---|---|---|
| **1. Text and PDF never become trustworthy** | Carets drift, wrapping differs between preview/export, fonts vary by OS, or every change breaks fixtures. | Stop feature work. Resolve font ownership, shared layout, and deterministic tests before adding UI. |
| **2. Publisher migration becomes the whole project** | Early adopters refuse trials without `.pub` import; contributors spend more effort on converters than weekly editing. | Test reconstruction-assisted adoption immediately. If users will not accept it, change the beachhead—not the compatibility claim. |
| **3. Volunteers find the application slower than copying last week’s file** | Users repeatedly unlock layouts, hunt for frames, miss overset, or need developer help to print. | Simplify the content-editing workspace and templates. Measure time to finished bulletin, not feature count. |
| **4. Cross-platform packaging overwhelms maintainers** | Only one OS works reliably; Skia builds or Linux webviews routinely block releases; keyboard/accessibility behavior diverges. | Make three-OS smoke tests and installer delivery part of the first prototype. Cut optional features before dropping platform reliability. |
| **5. Scope outruns the volunteer team** | Multiple unfinished subsystems, no usable releases, and arguments about InDesign parity dominate issue traffic. | Enforce milestone gates and the exclusion table. Require a named maintainer and test plan for every new subsystem. |

## Final Judgment

The opportunity is not “open-source InDesign with a Publisher skin.” It is **a dependable successor to the inherited weekly bulletin file**.

Win on protected layouts, readable templates, explicit overflow, portable fonts, correct folding, and painless handoff. The underlying engine should be extensible—but extensibility is not permission to build every publishing market at once.