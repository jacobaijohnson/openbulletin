# OpenBulletin

**Page layout for churches, in Rust. Last week's bulletin, safely turned into
this week's bulletin.**

Microsoft ended Publisher in October 2026: it was removed from Microsoft 365
and support ended on October 13, 2026. InDesign is a subscription built for
professionals. Scribus is powerful, but it was built for layout experts.
OpenBulletin is a church-first, open-source desktop publishing app for
macOS, Windows, and Linux, built around the document churches actually make
every week: the bulletin.

## The idea

- **You edit pages. The app handles the sheet.** You work on the pages
  people read: the cover, the order of service, the notes, the back. The
  app works out how to place those pages on the paper so they come out in
  the right order after you print and fold.
- **Fields, not find-and-replace.** Church name and address, service times,
  website, giving link, pastor, the Sunday name and date: these are fields.
  The weekly edit is filling in this week, not hunting through last week's
  file.
- **Next Sunday.** One command copies the bulletin, moves the date forward
  a week, clears the parts that change every week, and keeps the parts that
  stay the same.
- **Dot leaders are built in.** "Hymn of Praise .......... 432" is lined up
  by the app, dots and all, not typed by hand.
- **A Check step before you print.** It makes sure the date is filled in.
  It makes sure the order of service is there. It measures your words with
  real font measurements and warns you, in plain words, if text is too long
  for its box. It also warns about fonts the bulletin names that are
  not in your Font Library, and about boxes running close to a page
  edge or the fold.
- **Two-sided printing help.** Every printer flips the paper its own way.
  Print one test sheet, tell the app how it came out, and it remembers the
  right setting for that printer.
- **Paper and folds you can use today.** Letter folded in half, Letter
  folded in thirds, Letter portrait, Legal folded in half, Tabloid folded
  in half. Booklets stapled in the middle fold (saddle-stitch) work too.
- **Familiar keys.** The Keys window (top bar) starts from
  OpenBulletin's own set of keyboard shortcuts, Publisher's, or
  InDesign's, so your hands feel at home; any key you change makes
  the set your own Custom set. Details in
  [docs/MODES.md](docs/MODES.md).

## What works today, and what does not yet

This is early software (version 0.1). Here is the honest list.

### Works today

- **The desktop app** walks you through five steps: Start, This Sunday,
  Announcements, Check, Print.
- **Preview.** You can see each page of your bulletin.
- **Open and save.** Bulletins are saved as `.obd` files.
- **Make a PDF.** One button makes a reading copy, one page per panel, for
  email and the website. Another makes a print copy: sheets laid out in
  the right order, ready to print two-sided and fold.
- **The Check step** described above.
- **A Pages window** for arranging each page: drag boxes, set guides
  and positions by typing exact inches, and type words right on the
  page.
- **Autosave.** The app quietly keeps a safety copy while you work.
  If it closes suddenly, your work is offered back when it opens.
- **Two-sided printing help** described above.
- **Save a copy for another program.** On the Print step, one button
  saves a copy in another program's format (today: InDesign's IDML),
  and tells you in plain words anything the copy could not carry.
- **A command line tool** for people who like typing commands: `new`,
  `export`, `export-idml`, `next-sunday`, `info`, `fonts`, `trace`.
  The engine underneath the app is real and tested.

### Fonts today

- Two font families come with the app: Liberation Sans (it looks like
  Arial) and Liberation Serif. Each has regular, bold, italic, and
  bold italic faces, eight in all.
- **Italics work.** The bundled set includes italic faces, and there is
  an italic Scripture style for readings.
- **There is a Font Library and a Fonts panel.** The app keeps its fonts
  in one folder (the same on every computer), shows each one with a
  live preview and its sharing rights in plain words, and can search a
  catalog of free fonts and install them from inside the app. Styles
  name their font family, and the layout uses the real font.
- **You can pick a font for each style inside the app:** the Fonts
  panel has a picker per style, and it takes effect everywhere at
  once. Fonts already on your computer can be copied in with one
  tap from the same panel, or added as files or zip packs.
- **PDFs include the real fonts.** Print and reading PDFs embed the
  same font files the layout measured with (when the font's license
  lets it travel), so what the Check step measured is what prints.
  One limit: letters outside the western European set (like a cross
  symbol typed as a character) print as spaces for now.

### Not built yet

- **A signed installer.** On Windows there is a ready bundle instead
  (see "Run it" below): one zip you unzip anywhere, nothing to
  install. Signed installers for Windows, macOS, and Linux are not
  built yet; on macOS and Linux you build the app from source code
  for now.

Plans and order of work: [ROADMAP.md](ROADMAP.md).

## Run it

On Windows there is a ready bundle: one zip holding the app, its PDF
reader, and the PDF library in a single folder. Unzip it anywhere and
run `openbulletin.exe`; there is nothing to install. (Bundles are
attached to releases and to the "Windows bundle" workflow runs.)
Building from source needs Rust 1.80+ (install via rustup), then:

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

For developers. The engine does not know which front door served it:
desktop app, CLI, or (someday) web.

| Crate | Job |
| --- | --- |
| `ob-model` | Document model: panels, frames, stories, styles, fields, church profile. Sizes are stored in thousandths of a point so layout math is exact. |
| `ob-layout` | Fold-aware presets (bi-fold, tri-fold, and the half-letter saddle booklet), bulletin templates, the Next Sunday command. |
| `ob-impose` | Turns reader pages into printer sheets: bi-fold, tri-fold, saddle-stitch. |
| `ob-text` | Measures and wraps text using real font shapes (parley), and builds dot-leader lines. Uses fonts given to it, never fonts found on your computer. |
| `ob-pdf` | Makes the PDFs: reading copies and print sheets, with the real fonts embedded. |
| `ob-extract` | Imports and exports: reads old bulletins (PDFs through the helper, Publisher files with our own reader or the LibreOffice bridge, InDesign IDML, Scribus) into one shared dump the Template Analyzer learns from, and writes documents back out as IDML for InDesign. |
| `ob-format` | Reads and writes `.obd` files. Today's format is version 0, stored as JSON. A zip version is planned. |
| `openbulletin-cli` | The same engine with no window: create, export (PDF and IDML), next-sunday, info, fonts, trace. |
| `openbulletin` | The egui desktop app. A thin shell over the crates above, on purpose. |

## Imports and conversions

Honest labels, in plain words:

- **OpenBulletin `.obd` files open today**, fully.
- **Old bulletins open today through one door**: on the Start step,
  "Open an old bulletin..." takes a PDF, a Publisher file, an
  InDesign (IDML) file, or a Scribus file and works out which it
  is. A helper program reads PDFs and rebuilds the words as boxes
  you can edit, with each old page kept underneath as a picture to
  check against. Pieces are labeled Good, Check this, or Picture
  only before you accept anything.
- **InDesign files open through IDML**, InDesign's own export:
  words, boxes, and fonts come over as a draft to review; placed
  pictures do not yet. Direct `.indd` reading stays out of scope:
  `.indd` is a closed binary with no published format.
- **Publisher (`.pub`) files open two ways.** When LibreOffice
  (a free program) is installed, it turns the file into a PDF
  behind the scenes and it comes in like a PDF rescue. When it
  is not, OpenBulletin's own reader opens Publisher 2002-2019
  files directly: the words land in the boxes Publisher put
  them in, with master pages, tables, and styled text art
  marked Check this; pictures are counted and their places
  marked, but their image bytes do not come over yet. Publisher
  2000 and earlier files need the LibreOffice route, or a PDF
  saved from Publisher itself; the app says so in plain words.
- **Scribus (`.sla`) files open through the same door**: words,
  boxes, and fonts come over as a draft to review; picture
  frames are counted and named in the review, not carried.
- **Old bulletins can also teach the app your church's template**
  (Start step, "Learn from your old bulletins"): pick old
  bulletins in any mix of the kinds above. Learning needs at
  least three bulletins that count: Publisher, InDesign,
  Scribus, and OpenBulletin files count one-for-one, and PDFs
  count only in a group of three or more, because one or two
  PDFs alone cannot show what repeats. The app states the rule
  before it starts, and says exactly what to add when a pick
  falls short.
- **Copies go back out, too.** On the Print step, "Save a copy
  for another program..." writes an InDesign (IDML) file with
  the words, their boxes, and their fonts, and lists in plain
  words anything the copy could not carry. (Writing `.pub` or
  `.indd` files stays out of scope: those formats are theirs
  to write, not ours.)
- **One limit that will never change:** we will never copy fonts out of
  old PDFs. Those fonts are usually partial copies, and the law usually
  does not allow it. Instead, imports swap in a free font with the
  same letter widths where one exists (Arial to Liberation Sans, for
  example), and flag the piece Check this where none does.

We will help you get your text and pictures out of old files. We will not
pretend any tool opens Publisher files perfectly.

## Principles

KISS: Keep It Short & Simple. Built for church office administrators, not
layout professionals: a guided weekly flow, plain language, and
automation of everything that recurs. Simple on the surface, powerful
underneath. Full text in [docs/PRINCIPLES.md](docs/PRINCIPLES.md).

## What we will not build

Email merge, web/HTML export, e-book (EPUB) export, pattern-based style
rules (GREP styles), footnotes and indexes, real-time collaboration,
stock-art libraries, AI generation in the app, `.pub` export, `.indd`
reading, full PDF editing. The reasoning is public in
[docs/NEVER.md](docs/NEVER.md).

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
