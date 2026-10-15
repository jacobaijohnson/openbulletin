# Tier-2 README review brief (Jacob-requested, 2026-10-15)

You are reviewing the README.md of OpenBulletin, an open-source Rust
desktop-publishing app for church bulletins (repo is public on GitHub).
Judge it on three axes: (1) CORRECTNESS against the ground truth below;
(2) PRESENTATION: does it read well and look like a project a church
office administrator and a Rust developer can both trust; (3) KISS/ELI5:
simple words a five-year-old could follow, but explicit: nothing vague
hiding behind the simplicity.

## Ground truth (verified 2026-10-15; anything in the README beyond this is an overclaim)

SHIPPED NOW: headless engine + CLI (new/export/next-sunday/info) fully
working; fold presets (Letter bi-fold/tri-fold/portrait, Legal bi-fold,
Tabloid bi-fold); imposition incl. saddle-stitch; .obd format v0 (JSON);
desktop app with a guided 5-step flow (Start, This Sunday,
Announcements, Check, Print), panel preview, Open/Save .obd, real PDF
export buttons (reader + imposed), a Check step that verifies date and
order-of-service presence plus REAL text-fit checks (shaped widths),
and a two-sided-printing test-sheet wizard that remembers per-printer
flip settings (landing today).
TEXT AND FONTS: real shaping via parley, registered fonts only (no
system-font machinery in the engine). Bundled fonts TODAY: Liberation
Sans and Liberation Serif, regular + bold only (metric-compatible with
Arial; close to Helvetica). No italics bundled yet. PDF output draws
core-font Helvetica, fonts are NOT embedded yet. The document model has
no font-family field yet; all text shapes as Liberation Sans today.
Substitution of look-alike fonts for imported documents (Calibri to
Carlito etc.) is PLANNED, not built. Subset fonts from PDFs are never
extracted or re-embedded (license + technical reasons).
IMPORTS: only scaffolding exists: an extraction crate that turns a PDF
into a versioned JSON dump (pages, sizes, text spans, trust marks).
No working PDF-to-bulletin conversion, no .pub import, no IDML import
yet; the Template Analyzer is a written design, not code.
NOT YET TRUE despite sounding close: keymap presets (Publisher/InDesign
keys) are a documented design (docs/MODES.md), NOT implemented.
Preflight beyond the Check step list above (fold-zone warnings, missing
font warnings, image resolution) is NOT implemented. NAS/Docker and
accounts are documented plans only.

## Output contract

- Overall verdict, max 5 lines.
- Findings, most severe first. Each finding: SEVERITY (blocker / major /
  minor), the exact README line quoted, what is wrong or unclear in one
  or two sentences, and a suggested replacement written in plain ELI5
  words (short sentences, no jargon, no em dashes).
- A special pass on FONT handling: is what the README says or implies
  about fonts true, complete enough, and clear to a non-expert?
- End with: the single section you would rewrite first, rewritten in
  full, KISS/ELI5 but explicit.
- Total under 1,800 words. Do not praise; find problems.

## The README under review

# OpenBulletin

**Page layout for churches, in Rust. Last week's bulletin, safely turned into
this week's bulletin, printed right the first time.**

Microsoft Publisher was discontinued on October 13, 2026. InDesign is a
subscription built for professionals. Scribus is capable but hard for a
volunteer to love. OpenBulletin is a church-first, open-source desktop
publishing app for macOS, Windows, and Linux, built around the document
churches actually make every week: the bulletin.

## The idea

- **Fold-aware from the start.** You edit reader panels (cover, order of
  service, notes, back). The app owns the sheet: imposition, fold-in panel
  widths, and printer spreads are its problem, not yours.
- **Fields, not find-and-replace.** Church profile, Sunday name and date,
  service times, and giving links are fields. The weekly edit is filling in
  this week, not hunting through last week's file.
- **Next Sunday.** One command clones the bulletin, advances the date, and
  clears weekly content while standing content stays put.
- **Dot leaders are first-class.** "Hymn of Praise .......... 432" is a
  real layout operation, not forty hand-typed spaces.
- **Preflight that speaks bulletin.** Overset text, text in the fold zone,
  missing fonts, low-resolution images: flagged before printing, in plain
  language.
- **One interface, familiar keys.** A single canonical UI with keymap
  presets (OpenBulletin / Publisher / InDesign) and vocabulary aliases in
  search. The file, the menus, and the output never change with a mode.
  See [docs/MODES.md](docs/MODES.md).

## Status

v0.1 scaffold: the headless engine is real and tested (document model,
fold-aware templates, imposition, text helpers, PDF export, file format,
CLI). The desktop shell shows templates and previews panels; inline text
editing arrives with the text-engine spike described in
[ROADMAP.md](ROADMAP.md).

## Run it

Needs Rust 1.80+ (install via rustup), then:

```sh
# Desktop app
cargo run --release -p openbulletin

# Headless CLI (the engine, scriptable)
cargo run --release -p openbulletin-cli -- new --preset letter-bifold \
  --church "Grace Lutheran Church" --pastor "Pastor Lee" \
  --date 2026-10-18 --sunday-name "Twentieth Sunday after Pentecost" \
  --out bulletin.obd
cargo run --release -p openbulletin-cli -- export bulletin.obd --imposed --out bulletin-print.pdf
cargo run --release -p openbulletin-cli -- export bulletin.obd --out bulletin-reader.pdf
cargo run --release -p openbulletin-cli -- next-sunday bulletin.obd --out bulletin-next.obd
cargo run --release -p openbulletin-cli -- info bulletin.obd
```

## Crates

| Crate | Job |
| --- | --- |
| `ob-model` | Document model: panels, frames, stories, styles, fields, church profile. Integer milli-point geometry. |
| `ob-layout` | Fold-aware presets, bulletin templates, the Next Sunday command. |
| `ob-impose` | Reader panels to printer sheets: bi-fold, tri-fold, saddle-stitch. Pure and heavily tested. |
| `ob-text` | Text measurement, wrapping, dot-leader lines (real shaping engine lands behind this API). |
| `ob-pdf` | PDF export: reader panels and imposed sheets. |
| `ob-extract` | Import extraction: foreign files (PDF first) become a versioned JSON dump the Template Analyzer reads. |
| `ob-format` | The `.obd` file format (v0 JSON; v1 zip container, same document JSON). |
| `openbulletin-cli` | Headless CLI. The CI oracle: if it cannot be done here, the GUI is hiding it. |
| `openbulletin` | The egui desktop app. Thin shell on purpose. |

## Imports and conversions

Your old documents are welcome here, with honest labels: native `.obd`
(full fidelity), PDF analysis that drafts an editable template from an old
bulletin PDF for you to confirm, Publisher `.pub` rescue import (text,
images, geometry via libmspub), and InDesign via IDML export/import.
Direct `.indd` reading stays out of scope; IDML is Adobe's supported way
out. Status per format lives in [ROADMAP.md](ROADMAP.md).

## Principles

KISS. Built for church office administrators, not layout professionals:
a guided weekly flow, plain language, and automation of everything that
recurs. Full text in [docs/PRINCIPLES.md](docs/PRINCIPLES.md).

## What we will not build

Email merge, web/HTML export, EPUB, GREP styles, footnotes and indexes,
real-time collaboration, stock-art libraries, AI generation in the app,
`.pub` export, `.indd` reading, full PDF editing. The reasoning is public
in [docs/NEVER.md](docs/NEVER.md). We will help you get your text and pictures out of old files; we will not
pretend any tool opens Publisher files perfectly.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option. The Apache option is there on purpose: it is just as
permissive, and it carries an express patent grant, so anyone who
contributes or uses OpenBulletin has that protection in writing.

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in OpenBulletin shall be dual licensed as above,
without any additional terms or conditions.
