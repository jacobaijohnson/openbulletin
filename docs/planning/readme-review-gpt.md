## Overall verdict

The README blurs working features, unfinished work, and written plans.
Its import and print-check claims could lead a church office to rely on tools that do not exist.
The Status section understates the desktop app, while other sections overstate it.
Font limits are missing. Technical slogans also hide details that both audiences need.

## Findings, most severe first

### 1. BLOCKER: Imports are presented as working

> Your old documents are welcome here, with honest labels: native `.obd`  
> (full fidelity), PDF analysis that drafts an editable template from an old  
> bulletin PDF for you to confirm, Publisher `.pub` rescue import (text,  
> images, geometry via libmspub), and InDesign via IDML export/import.

Only `.obd` opening and saving work. PDF extraction produces a JSON dump, not an editable bulletin; `.pub` import, IDML import, and the Template Analyzer are not built.

**Replacement:**  
“You can open and save OpenBulletin `.obd` files. You cannot yet turn a PDF, Publisher file, or IDML file into a bulletin. A developer tool can read page sizes and text from a PDF and save that information. It does not make an editable bulletin.”

### 2. BLOCKER: Print checks promise protection that does not exist

> - **Preflight that speaks bulletin.** Overset text, text in the fold zone,  
>   missing fonts, low-resolution images: flagged before printing, in plain  
>   language.

Only date, order-of-service presence, and real text-fit checks are implemented. Fold-zone, missing-font, and image-resolution warnings are not.

**Replacement:**  
“**Check before printing.** The Check step checks the date, checks that an order of service is present, and checks whether text fits. It does not yet warn about text near folds, missing fonts, or blurry pictures.”

### 3. MAJOR: Familiar keyboard modes are only a design

> - **One interface, familiar keys.** A single canonical UI with keymap  
>   presets (OpenBulletin / Publisher / InDesign) and vocabulary aliases in  
>   search. The file, the menus, and the output never change with a mode.  
>   See [docs/MODES.md](docs/MODES.md).

The keyboard presets are not implemented. Search aliases are also not established by the supplied ground truth.

**Replacement:**  
“**Planned keyboard choices.** Publisher-style and InDesign-style keyboard shortcuts are not built yet. The plan is in [docs/MODES.md](docs/MODES.md).”

### 4. MAJOR: PDF export hides important font limits

> | `ob-pdf` | PDF export: reader panels and imposed sheets. |

The export claim is true but incomplete: text is measured using Liberation Sans, while PDFs draw core-font Helvetica without embedding fonts. Readers cannot tell that font choice is unavailable or that the preview and PDF use different fonts.

**Replacement:**  
“`ob-pdf` makes PDFs for reading and printing. Today, PDFs use Helvetica and do not include font files. The app measures text with Liberation Sans, so the preview and PDF use different fonts.”

### 5. MAJOR: Status describes an obsolete desktop shell

> v0.1 scaffold: the headless engine is real and tested (document model,  
> fold-aware templates, imposition, text helpers, PDF export, file format,  
> CLI). The desktop shell shows templates and previews panels; inline text  
> editing arrives with the text-engine spike described in  
> [ROADMAP.md](ROADMAP.md).

The desktop now has a guided five-step flow, Open/Save, working PDF buttons, real Check checks, and a two-sided-printing test wizard. This wording also incorrectly suggests that real text shaping remains future work.

**Replacement:**  
“The engine, command-line tool, and desktop app work today. The desktop guides you through Start, This Sunday, Announcements, Check, and Print. It can open and save bulletins, show panel previews, and export PDFs. See the limits below before using it for a weekly bulletin.”

### 6. MAJOR: The text-engine description is stale

> | `ob-text` | Text measurement, wrapping, dot-leader lines (real shaping engine lands behind this API). |

Real shaping through parley is already working. “Lands behind this API” is both outdated and difficult for a non-developer to understand.

**Replacement:**  
“`ob-text` uses parley to measure how much space text needs. It uses fonts registered with the engine, not fonts found on your computer.”

### 7. MAJOR: Extraction implies a working analyzer and broader file support

> | `ob-extract` | Import extraction: foreign files (PDF first) become a versioned JSON dump the Template Analyzer reads. |

Only PDF extraction scaffolding exists. The Template Analyzer is a written design, not a program that reads these dumps.

**Replacement:**  
“`ob-extract` reads information from PDFs and saves it as JSON with a format version. This includes pages, page sizes, text pieces, and marks about how much to trust the results. It does not turn the PDF into a bulletin. The planned Template Analyzer is not built.”

### 8. MAJOR: The headline makes an unconditional printing promise

> **Page layout for churches, in Rust. Last week's bulletin, safely turned into  
> this week's bulletin, printed right the first time.**

“Printed right the first time” promises an outcome that limited checks and a printer setup wizard cannot guarantee. The old-bulletin wording also reinforces the false impression that Publisher and PDF conversion work.

**Replacement:**  
“**Make church bulletins with OpenBulletin, built in Rust. Start a new bulletin or reuse an OpenBulletin file. Check it and test two-sided printing before printing copies.**”

### 9. MAJOR: The fields claim exceeds verified behavior

> - **Fields, not find-and-replace.** Church profile, Sunday name and date,  
>   service times, and giving links are fields. The weekly edit is filling in  
>   this week, not hunting through last week's file.

The supplied facts do not establish this full set of fields or the claimed weekly editing behavior. Do not present service times, giving links, and automatic reuse as shipped without verification.

**Replacement:**  
“**Start with this Sunday.** When you create a bulletin with the command-line tool, you can set the church name, pastor, date, and Sunday name.”

### 10. MAJOR: “Next Sunday” promises unverified content rules

> - **Next Sunday.** One command clones the bulletin, advances the date, and  
>   clears weekly content while standing content stays put.

The command is shipped, but the supplied facts do not verify these exact rules for clearing and keeping content. Those rules matter because users could lose or accidentally reuse weekly material.

**Replacement:**  
“**Next Sunday.** The command-line tool has a `next-sunday` command to make next week’s bulletin file. Open the new file and check its date and content before printing.”

### 11. MINOR: The file-format row mixes current and future formats

> | `ob-format` | The `.obd` file format (v0 JSON; v1 zip container, same document JSON). |

Only v0 JSON is verified as shipped. The row makes v1 look like an available format rather than an unverified future detail.

**Replacement:**  
“`ob-format` reads and writes `.obd` files. The current format is version 0 and stores the document as JSON.”

### 12. MINOR: Dot-leader behavior is asserted without verified support

> - **Dot leaders are first-class.** "Hymn of Praise .......... 432" is a  
>   real layout operation, not forty hand-typed spaces.

The supplied facts do not verify this feature. “First-class” and “real layout operation” also fail to explain what a user actually does.

**Replacement, until verified:**  
“**Text fit.** The Check step measures text and checks whether it fits in its space.”

### 13. MINOR: The CLI description is an unexplained absolute claim

> | `openbulletin-cli` | Headless CLI. The CI oracle: if it cannot be done here, the GUI is hiding it. |

“CI oracle” is opaque to office users, and the assertion that every desktop action must exist in the CLI is not established. It also sounds accusatory rather than informative.

**Replacement:**  
“`openbulletin-cli` runs without opening a window. It can create a bulletin, export PDFs, make next week’s file, and show file information.”

## Special pass: font handling

**True?** The README’s explicit font-related promise is not true: it claims missing-font warnings that are not implemented. Its statement that real shaping is still coming is also false.

**Complete enough?** No. It omits all of these material limits:

- Bundled fonts are Liberation Sans and Liberation Serif, regular and bold only. There are no bundled italics.
- All document text currently shapes as Liberation Sans. Bundling Serif does not make it selectable.
- The document model has no font-family field.
- The engine uses registered fonts, not automatic system-font discovery.
- PDF output uses core-font Helvetica and embeds no fonts.
- Look-alike replacement for imported fonts is planned, not built.
- Fonts stored as subsets inside PDFs are never extracted or re-embedded, for licensing and technical reasons.

**Clear to a non-expert?** No font explanation exists. Add this plain-language note:

“Today, all bulletin text is measured with Liberation Sans. You cannot choose another font yet. We also include Liberation Serif, but bulletins do not use it yet. Both bundled fonts have regular and bold letters. Neither has bundled italic letters.

Liberation Sans has matching letter sizes to Arial and is close to Helvetica. Exported PDFs use Helvetica, not the bundled font, and contain no font files. Check the exported PDF before printing.

The engine does not automatically use fonts installed on your computer. Replacing fonts from old documents with similar fonts is planned, not built. We never copy embedded font subsets out of PDFs.”

## Rewrite first: Status

## Status

The engine, command-line tool, and desktop app work today. Importing old documents is not ready.

### What works now

- The desktop guides you through five steps: **Start, This Sunday, Announcements, Check, and Print**.
- You can preview panels and open or save `.obd` bulletin files.
- PDF buttons make a reading copy and a copy with pages arranged for printing and folding.
- Check checks the date, checks that an order of service is present, and measures whether text fits.
- A test-sheet wizard helps set up two-sided printing. It remembers the flip setting for each printer.
- The command-line tool supports `new`, `export`, `next-sunday`, and `info`.
- Page choices include Letter bi-fold, tri-fold, and portrait; Legal bi-fold; and Tabloid bi-fold. Page arrangement also supports booklets stapled through the fold.
- `.obd` files currently use version 0, stored as JSON.

### Limits to know before printing

All text is measured with Liberation Sans. You cannot choose a font yet. PDFs use Helvetica and do not include font files. Check the exported PDF before printing copies.

Check does not yet warn about text near folds, missing fonts, or blurry pictures.

### What is not built yet

PDF-to-bulletin conversion, Publisher import, and IDML import do not work yet. A developer tool can extract information from PDFs, but it does not make a bulletin.

The Template Analyzer, Publisher-style and InDesign-style keyboard shortcuts, automatic font replacements, NAS/Docker support, and accounts are plans only.