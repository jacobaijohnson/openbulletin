# OpenBulletin README review (2026-10-15)

## Overall verdict

The README promises features that do not exist. These are imports, keymap presets, and most of preflight. It also hides features that do exist, like the guided flow, the Check step, the duplex wizard, and real text shaping. It says almost nothing true about fonts, which is the area most likely to surprise a church office. Much of it is written for a Rust developer, not an administrator. **Not publishable as is: three blockers.**

## Findings (most severe first)

**1. BLOCKER: Imports section describes software that does not exist.**
> "PDF analysis that drafts an editable template from an old bulletin PDF for you to confirm, Publisher `.pub` rescue import (text, images, geometry via libmspub), and InDesign via IDML export/import."

None of this is built. The only code is a crate that dumps a PDF's pages, sizes, and text spans to JSON. Pointing to ROADMAP.md does not undo present-tense claims. An admin whose Publisher just lost support will download this for exactly these features.

Suggested: "OpenBulletin cannot open your old files yet. It cannot open Publisher (.pub) files. It cannot open InDesign files. It cannot turn an old PDF into a bulletin. Today we only have a first step: a tool that reads a PDF and lists the words and page sizes it finds. Opening old files is planned. See ROADMAP.md for what comes next."

**2. BLOCKER: Preflight bullet lists checks that are not built.**
> "Overset text, text in the fold zone, missing fonts, low-resolution images: flagged before printing, in plain language."

Only overset (text-fit) is real. Fold-zone, missing-font, and image-resolution warnings do not exist. The real Check step also checks the date and the order of service, and the README never says so.

Suggested: "**A Check step before you print.** It makes sure the date is filled in. It makes sure the order of service is there. It measures your words and warns you if any text is too long for its box. More checks are planned, like warnings for text that lands on the fold."

**3. BLOCKER: Keymap presets presented as a feature.**
> "A single canonical UI with keymap presets (OpenBulletin / Publisher / InDesign) and vocabulary aliases in search."

Keymap presets exist only as a design document. "Vocabulary aliases in search" is not in the shipped list either. A Publisher user will press familiar keys and nothing will happen.

Suggested: "**Planned: familiar keys.** We plan to let you pick Publisher or InDesign keyboard shortcuts. This is not built yet. The plan is in docs/MODES.md."

**4. MAJOR: Status section is stale and undersells what works.**
> "The desktop shell shows templates and previews panels; inline text editing arrives with the text-engine spike described in ROADMAP.md."

The app now has a five-step guided flow (Start, This Sunday, Announcements, Check, Print), Open/Save, real PDF export buttons, the Check step, and a two-sided printing test-sheet wizard. The "text-engine spike" is done: shaping is real via parley. Readers cannot tell what to trust when the README is wrong in both directions. See the full rewrite at the end.

**5. MAJOR: Crate table states unbuilt pieces as fact.**
> "`ob-text` | Text measurement, wrapping, dot-leader lines (real shaping engine lands behind this API)."

Shaping already landed. This line now underclaims.

Suggested: "Measures and wraps text using real font shapes (parley). Draws dot-leader lines."

> "`ob-extract` | ... become a versioned JSON dump the Template Analyzer reads."

The Template Analyzer is a written design, not code.

Suggested: "Reads a PDF and writes down its pages, sizes, and words as JSON. Later, a planned Template Analyzer will use this."

> "`ob-format` | ... (v0 JSON; v1 zip container, same document JSON)."

v1 is not shipped.

Suggested: "The .obd file. Today it is JSON (version 0). A zip version is planned."

**6. MAJOR: No path for the main audience.**
> "Needs Rust 1.80+ (install via rustup), then:"

A church administrator cannot use cargo. The README never says whether an installer exists.

Suggested, placed above the commands: "There is no installer yet. To try OpenBulletin today you need to build it from source code. If that sounds unfamiliar, it is not ready for you yet. Watch this page for a download."

**7. MAJOR: Tagline promises print certainty the product cannot yet give.**
> "printed right the first time."

Fonts are not embedded, so output depends on the PDF viewer and printer. The duplex wizard only lands today, and the README never mentions it.

Suggested: "Last week's bulletin, turned into this week's bulletin. A test sheet helps your printer print both sides the right way up."

Then add the wizard as a bullet: "**Two-sided printing help.** Print one test sheet. Tell the app how it came out. It remembers the right setting for that printer."

**8. MAJOR: Unsourced claim about Microsoft.**
> "Microsoft Publisher was discontinued on October 13, 2026."

This is outside the verified ground truth. It needs a link to Microsoft's notice. "Discontinued" also suggests existing copies stop working, which may scare people needlessly.

Suggested: "Microsoft stopped supporting Publisher in October 2026 [link to Microsoft's notice]. Your copy may still open, but it gets no more fixes."

**9. MINOR: Unverified field list.**
> "Church profile, Sunday name and date, service times, and giving links are fields."

The CLI proves church, pastor, date, and Sunday name. Service times and giving links are not verified. Confirm them or cut them.

**10. MINOR: Fold presets never listed.**
> "--preset letter-bifold"

Readers cannot see which paper sizes work.

Suggested: "Paper and folds you can use today: Letter folded in half, Letter folded in thirds, Letter portrait, Legal folded in half, Tabloid folded in half. Booklets with staples in the middle (saddle-stitch) work too."

**11. MINOR: Jargon throughout "The idea".**
> "The app owns the sheet: imposition, fold-in panel widths, and printer spreads are its problem, not yours."

"Imposition," "printer spreads," "overset," "headless," "CI oracle," and "milli-point" mean nothing to an administrator. "Fold-in panel widths" is also not in the verified list.

Suggested: "You work on the pages people read: the cover, the order of service, the back. The app works out how to place those pages on the paper so they come out in the right order after you fold."

**12. MINOR: Platforms not verified.**
> "for macOS, Windows, and Linux"

This is not in the ground truth. If the app is only known to build on these systems, say that: "Made to run on macOS, Windows, and Linux."

**13. MINOR: Tone toward Scribus.**
> "Scribus is capable but hard for a volunteer to love."

A dig at a fellow open-source project costs trust with developers. Suggested: "Scribus is powerful, but it was built for layout experts."

**14. MINOR: Formatting.** In "What we will not build," the line beginning "in [docs/NEVER.md]" is far longer than the lines around it. "GREP styles" and "EPUB" are jargon an admin will not know. Re-wrap the paragraph and gloss or drop those terms.

## Font pass

**What the README says:** almost nothing. The only font mention is the false "missing fonts" preflight claim. The ob-text row implies shaping is still coming.

**What is true and missing:**

1. **Which fonts ship.** Liberation Sans and Liberation Serif, regular and bold only. They match Arial's letter widths and are close to Helvetica. An admin needs this before deciding to switch.
2. **No italics.** Bulletins often italicize congregational responses, rubrics, and hymn sources. This must be stated plainly.
3. **No font choice yet.** The document model has no font-family field, so all text uses Liberation Sans. Liberation Serif is bundled but cannot be selected. Anyone who reads "Serif bundled" will assume they can use it.
4. **The PDF uses a different font than the measurements.** Text is measured with Liberation Sans, but the PDF names core-font Helvetica, unembedded. The viewer or printer supplies its own Helvetica-like font. Widths are close but not guaranteed identical. So a line the Check step says fits could print slightly differently. This is the most important font fact and it directly undercuts "printed right the first time."
5. **Fonts are not embedded.** Some print shops reject PDFs with unembedded fonts. An administrator sending files to a copy shop must know this.
6. **No system fonts.** You cannot use fonts installed on your computer. Only the bundled fonts work.
7. **Imports and fonts.** Look-alike substitution (Calibri to Carlito) is planned, not built. Fonts inside old PDFs will never be copied out, for license and technical reasons. This belongs in the imports section as an honest limit.

**Clarity to a non-expert:** currently zero, because nothing is said. Words like "shaping," "embedded," and "metric-compatible" need one plain sentence each, as in the rewrite below.

## Rewrite first: "Status" (replaced with "What works today")

This section carries the most trust. It fixes findings 1 to 5 and the font gaps in one place.

```markdown
## What works today, and what does not yet

This is early software (version 0.1). Here is the honest list.

### Works today

- **The desktop app** walks you through five steps:
  Start, This Sunday, Announcements, Check, Print.
- **Preview.** You can see each page of your bulletin.
- **Open and Save.** Bulletins are saved as `.obd` files.
- **Make a PDF.** One button makes a PDF for reading on screen.
  Another makes a PDF laid out for your printer, ready to fold.
- **Check step.** Before you print, it checks three things.
  Is the date filled in? Is the order of service there?
  Does every piece of text fit in its box?
- **Two-sided printing help.** Print one test sheet. Tell the app
  how it came out. It remembers the right setting for that printer.
- **Paper and folds:** Letter folded in half, Letter folded in thirds,
  Letter portrait, Legal folded in half, Tabloid folded in half.
  Stapled booklets (saddle-stitch) work too.
- **Command line tool** for people who like typing commands:
  `new`, `export`, `next-sunday`, `info`.

### Fonts today

- Two font families come with the app: Liberation Sans (looks like
  Arial) and Liberation Serif. Each has regular and bold.
- There is **no italic** yet.
- You **cannot pick a font** yet. All text uses Liberation Sans.
- You **cannot use fonts installed on your computer.**
- The PDF **does not include the font file.** It asks the viewer or
  printer to use Helvetica, which looks very close. Because of this,
  a line may print a tiny bit wider or narrower than the Check step
  measured. Some print shops ask for PDFs with fonts included.
  Ours do not do that yet.

### Not built yet

- **Opening old files.** We cannot open Publisher (.pub) files,
  InDesign files, or turn an old PDF into a bulletin. We only have a
  first piece: a tool that lists the pages and words inside a PDF.
- **Swapping in look-alike fonts** for old documents (for example,
  Carlito for Calibri). Planned.
- **Publisher or InDesign keyboard shortcuts.** Planned. See
  docs/MODES.md.
- **More checks**, like text on the fold, missing fonts, or blurry
  pictures. Planned.
- **Typing directly on the page.** Planned.
- **An installer.** Today you must build the app from source code.

### Never

We will never copy fonts out of old PDFs. The law usually does not
allow it, and those fonts are often only partial copies.

Plans and order of work: [ROADMAP.md](ROADMAP.md).
```