# OpenBulletin Import Architecture: Independent Review

## 0. Problems with the framing (read first)

| # | Issue | Why it matters | Recommendation |
|---|---|---|---|
| F1 | **"PDF → template" is the wrong output.** | The product's core loop is "last week's bulletin → this week's." A separate *template* concept adds a new noun for admins and a second code path. | Produce a **draft bulletin document**. Mark which items are probably this-week-only. It then enters the normal "make this week's" flow. Call it "Rebuilt from PDF," not "template." |
| F2 | **One PDF cannot tell stable content from weekly content.** | Church name and staff list are stable. Sermon title and hymns change. A single file gives no signal to separate them. | Offer an optional step: "Have another week's bulletin? Add it and we'll spot what changes." Diffing two weeks is the strongest signal you can get. It beats any style heuristic. |
| F3 | **The fidelity check depends on shaping that doesn't exist yet.** | After you substitute fonts and reflow, you must test "does this text still fit its frame?" A placeholder estimator gives wrong overflow and wrong labels. That breaks the honesty promise. | Make real shaping (parley) a **hard prerequisite** for shipping any importer. The PDF *extraction* spike can run in parallel because it measures source positions, not reflow. |
| F4 | **"Human confirms" invites rubber-stamping.** | A single "Looks good → OK" dialog gets clicked through. | Ask **specific questions** with pictures, such as "Which of these is the front cover?" Show visible highlights on guessed items. Never ask for a blanket "approve." |
| F5 | **IDML is probably the wrong third format.** | Church office admins use Word, Publisher, Canva, and Pages. Very few use InDesign. | Keep IDML because it is cheap and good for contributors. But evaluate **.docx** before IDML on user value. Canva and Word users are already covered by the PDF path. |
| F6 | **"Fidelity label" should not be a single label.** | A global "85% accurate" is false precision. It is also unhelpful because it doesn't say *what* to check. | Label **per item**, plus a short summary in plain words (see §3). |
| F7 | **Publisher sunset changes who needs .pub rescue.** | Users who still have a working Publisher can simply export a PDF. The .pub path matters most for people whose Publisher is already gone. | Make "Save as PDF from Publisher" a guided first-class option. Do this *before* building the .pub parser. |

---

## 1. PDF extraction route

### Comparison

| Route | Text runs (font, size, position) | Image XObjects | License vs MIT app | Packaging (mac/win/linux) | Maintenance risk |
|---|---|---|---|---|---|
| **lopdf / pdf-rs (pure Rust)** | You must build it all yourself: text state machine, TJ kerning, font encodings (simple, Differences, CID/Identity-H, ToUnicode CMaps), widths, Type3. `pdf-extract` is too crude for positions. Months of edge cases. | Raw stream access is fine. Decoding filters (DCT passthrough, Flate, JBIG2, JPX) and SMask compositing is extra work. | MIT/Apache. Clean. | Trivial (pure Rust). | Low dependency risk, **high self-maintenance risk**. You become a PDF text-extraction vendor. |
| **Pure-Rust interpreters (e.g. hayro family)** | Promising and getting better. Not yet proven for structured text extraction. | Rendering-oriented. | MIT/Apache. | Trivial. | Young. Watch it as a v2 path to drop FFI. |
| **PDFium via `pdfium-render`** | Strong. `FPDFText_*` gives per-char Unicode, char box, origin, font size, font name, and flags/weight. The page-object API gives text objects with transforms. Battle-tested on real-world broken PDFs (Chrome). | Good. Enumerate image objects with matrix and bounds. Get decoded bitmap or raw data. You can also rasterize pages for overlays and thumbnails. | BSD-3/Apache-style. **Compatible**; include notices. Crate is MIT/Apache. | Dynamic lib (~5–8 MB per platform). Prebuilt binaries exist (bblanchon/pdfium-binaries). Must handle notarization/codesigning on macOS. | Low. Google-maintained, stable C API, active Rust wrapper. |
| **MuPDF** | Best-in-class structured text (stext with font, size, bbox, blocks). | Excellent. | **AGPL-3.0 or commercial.** Linking makes the distributed binary AGPL. That is incompatible with the MIT intent. | Static C build, fine. | Low technical risk, **fatal license risk**. |
| **Poppler** | Good (TextOutputDev, glib API). | OK. | **GPL-2/3.** Same problem as MuPDF. | Painful on Windows/macOS (glib, cairo, etc.). | Bindings are thin. |

### Recommendation for v1: **PDFium via `pdfium-render`, loaded dynamically, run out-of-process**

- **Run it in a child process.** Use the same binary with an `analyze-pdf` subcommand, or a small helper. This gives crash isolation, a timeout, and memory caps. Hostile or broken PDFs will crash C code eventually, and a crash must not lose the admin's open document.
- **Use a narrow interface.** The helper emits a versioned **JSON "extraction dump."** All heuristics live in pure Rust on the app side. They can be tested from fixture dumps without PDFium present.
- **Use lopdf as a sidecar only** for cheap structural facts: page boxes (Media/Crop/Trim/Bleed), `/Rotate`, `/UserUnit`, page count, producer string. Use these as classifier hints, for example Producer = "Microsoft® Publisher".
- **Keep the dump format engine-neutral.** This lets a future pure-Rust extractor replace PDFium without touching the heuristics.
- **Geometry:** PDF user space is float points with a bottom-left origin. Convert once at the boundary to top-left integer milli-points, rounding half-even. Apply `/Rotate` and CropBox offsets before conversion.

### Dump schema (minimum)

| Level | Contents |
|---|---|
| Per page | Boxes, rotation, raster thumbnail ref. |
| Per char | Unicode, bbox, baseline origin, font id, size (effective, after the text matrix), fill color, render mode. Render mode 3 means invisible text, i.e. an OCR layer. |
| Per font | Base name with the subset prefix `ABCDEF+` stripped, flags, weight, italic angle, embedded yes/no. |
| Per image | Bounds, transform, pixel dimensions, clip, extracted bytes. |
| Per vector path summary | Count and area only, used to detect "text was outlined" and rules/boxes. |

---

## 2. From runs to an editable draft

### 2a. Grouping pipeline (deterministic, explainable)

1. **Chars → words.** Split on gap > ~0.25 em of the current font size, or on a font change with a gap.
2. **Words → lines.** Cluster by baseline (tolerance ~0.2 em) and same rotation. Then split a "line" wherever a horizontal gap exceeds ~1.5 em *and* crosses a detected gutter.
3. **Order-of-service detection** (special-case it; it's the bulletin's signature pattern):
   - The line has a left text cluster and a right-aligned cluster, for example `Hymn of Praise ........ 432` or `Scripture    John 3:16`.
   - Leader dots or spaces in between → a **right tab stop with optional dot leader** at the right cluster's right edge.
   - Three or more consecutive lines with the same right edge confirm it as an order-of-service block. This is high confidence and very valuable.
4. **Lines → paragraphs.** Merge consecutive lines when:
   - leading is consistent (±10%),
   - font and size are the same,
   - left edges agree (or right/center edges for centered or right-aligned text),
   - the previous line is "full" (near the column's right edge).

   A short previous line, or a leading jump > 1.4×, means a paragraph break. Classify alignment from edge variance: left, center, right, or justified (both edges consistent).
5. **Paragraphs → frames.** Union paragraphs that share a column x-range with vertical gap < ~2× body leading. Never cross a detected panel boundary or gutter.
6. **Reading order** within a panel: top-to-bottom, then left-to-right across detected columns.

### 2b. Panel and column detection

- **Gutter finding.** Build an x-occupancy histogram of ink (text plus images) per page. Gutters are long zero-ink runs that span ≥ 80% of the page height.
- **Snap to known presets before trusting raw gutters.** Score each candidate against a preset table with ~3 mm tolerance:

| Page size | Interpretation |
|---|---|
| 5.5×8.5 in, 8.5×11 in, A5, A4 | **Single reader panel per page** |
| 11×8.5 in, two panels of 5.5 in | Letter bi-fold sheet |
| 14×8.5 in, two panels of 7 in | Legal bi-fold sheet |
| 11×8.5 in, three panels (~3.625 / 3.6875 / 3.6875 in, fold-in panel narrower) or equal 3.667 in | Letter tri-fold sheet |
| 11×17 in | Tabloid → letter booklet sheet |

- **Fold/crop marks, short hairlines in margins, and TrimBox < MediaBox** are strong evidence of a **printer file**.

### 2c. Reader panels vs imposed sheets

| Signal | Reader panels | Reader spreads (facing pages) | Printer imposition |
|---|---|---|---|
| Page size | Panel-sized | Sheet-sized | Sheet-sized |
| Page count | 2, 4, 6, 8… (booklet: multiple of 4) | Often odd, e.g. cover alone, then spreads | Even; 2 sheets for a bi-fold (outside + inside) |
| Where is the "cover"? (largest type, church name, logo/photo, date) | Page 1 | Page 1 alone | **Right half of sheet 1** (back \| front). Tri-fold: right third of outside sheet. |
| Page numbers (if any) | Sequential | 2-3, 4-5 | Non-sequential across halves: 4 \| 1, 2 \| 3 |
| Marks / bleed | Rare | Rare | Common |
| Upside-down content (head-to-head, rotated 180°) | No | No | Possible (tumble / 4-up) |

**Handling:**

- **Reader panels:** each page maps to one panel in order. Pick the preset by count and size.
- **Imposed sheet:** split pages at the snapped fold lines into panels. Then **ask about the order**. Never silently guess, because a wrong guess produces a bulletin printed in the wrong order. Show the panel thumbnails and ask two questions:
  - "Which one is the **front cover**?"
  - "Which one is on the **back**?"

  Derive the rest from the fold preset.
- **Cannot classify:** fall back to "Here are your pages. Drag each into the spot where it goes on the folded bulletin." Always show this as a picture of the fold.

### 2d. Font size and weight → semantic styles

- Build a histogram of (size, weight, italic, font family) **weighted by character count**.

| Style | Rule |
|---|---|
| **Body** | Most frequent combination by char count. |
| **Church name / title** | Largest size on the cover panel, short (1–2 lines). Boosted if the text matches the church profile name. |
| **Heading** | Size > body × 1.15, or the same size but bold/caps, short lines, followed by body text. Cluster into ≤ 2 heading levels. Never create more. |
| **Order-of-service line** | Paragraphs with the tab/leader pattern from 2a. Use a dedicated style with the detected tab stop. |
| **Fine print** | Size < body × 0.85 (footers, CCLI license lines, copyright). |
| Everything else | Snap to the nearest of these styles if within ~1 pt, else "body (adjusted)." |

- Cap the result at roughly **6 styles**. Admins should see "Heading" and "Body," not "Style 14."
- **Fonts.** The embedded fonts are *subsets*, often not licensed for extraction, and missing the glyphs needed for next week's text.
  - **Never extract or re-embed them.**
  - Map each font to an installed font, matching on family name with normalized weight.
  - Otherwise use a bundled metric-compatible substitute: Liberation for Arial/Times, Carlito for Calibri, Caladea for Cambria, plus a decent serif and sans.
  - Record every substitution for labeling.

### 2e. Field detection

| Detection | Action |
|---|---|
| Date strings near the cover top that match "the Sunday nearest the PDF's creation date" | Suggest `{Sunday.date}` |
| Exact matches to profile values (name, address, phone, URL, pastor) | Suggest the profile field |
| Liturgical labels ("Third Sunday after Pentecost") | Suggest a field only if one exists, else leave as text |

### 2f. Confidence model

| Tier | What | UI behavior | Example plain-language copy |
|---|---|---|---|
| **Apply automatically** | Text content (when Unicode is clean), sheet size, fold preset when page sizes match exactly, positions of frames and images, body style | Applied silently. Covered by the global "Rebuilt from PDF" notice. | — |
| **Apply and highlight** | Heading/title style guesses, order-of-service tab layout, font substitutions, profile-field matches that are exact | Applied, with a soft highlight and a one-click "Undo this" | "We used **Carlito** because **Calibri** isn't on this computer. It looks nearly the same." |
| **Ask first** | Panel order on imposed sheets, fold type when ambiguous, date → `{Sunday.date}`, "this changes weekly?" marks, background images vs photos | A blocking question with a picture before the draft opens | "Is this the date of the service? If yes, we'll update it automatically every week." |
| **Can't rebuild: say so** | Outlined text, scanned pages, vector clip art, Type3 garbage text | Placed as a picture, clearly marked | "This part was a picture of text, so you can't type in it. Retype it or keep it as a picture." |

**Gating rules for the "can't rebuild" tier:**

- If more than ~2% of chars on a page have no Unicode mapping or are in the Private Use Area, treat the page's text as **unreadable**.
- If a page has no text objects but a large raster, or dense filled paths, treat it as **scanned or outlined**.
- **Do not OCR in v1.** Offer the page as a locked **"tracing guide"** underlay, faded behind the panel, so the admin can rebuild by typing over it. This is honest and very useful.

---

## 3. Fidelity honesty

### Never recoverable from PDF

| Lost | Consequence | What we do |
|---|---|---|
| **Paragraph and character styles** | We only see appearance; "Heading" is a guess | Inferred styles, flagged "guessed" |
| **Text threading / overflow links** | Each frame becomes standalone | Never invent links. The admin can link frames later. |
| **Fields / variables** | Dates and names are frozen text | Suggestions only (§2e) |
| **Real paragraph breaks vs line breaks** (in some producers), hyphenation intent, soft returns | Reflow may differ | Rebuild paragraphs; show a "check line breaks" hint if a frame overflows after reflow |
| **Tab and indent intent** beyond detected patterns | Spacing approximations | Order-of-service is special-cased; the rest stays approximate |
| **Fonts** (subsets) | Substitution | Name the substitution per item |
| **Master/repeating items**, groups, layers, z-order intent, text wrap | Flattened | Positioned as independent items |
| **Original image resolution and crops** | Images may be downsampled or recompressed | Warn if under ~150 effective ppi: "May print blurry" |
| **Vector artwork** | Not editable as shapes | Rasterize at 300 ppi as a picture |
| **Which content is weekly vs permanent** | — | Ask, or diff two weeks (F2) |

### Presentation

- **Name it honestly.** The banner on the draft reads: *"Rebuilt from your PDF. The words and layout were copied; styles and automatic fields are our best guesses. Check the highlighted items before printing."*
- **Side-by-side check view (most important trust tool).** Show the original page raster next to the rebuilt panel, with a **swipe or opacity slider overlay**. Non-experts trust what they can compare with their own eyes. They do not trust scores.
- **Per-item badges, three words only:**
  - **Matched**
  - **Check this** (guessed style, substituted font, reflowed text)
  - **Picture only** (not editable)
- **Summary in counts, not percentages.** For example: "42 text blocks rebuilt · 3 to check · 1 picture-only."
- **Fit check after reflow (needs real shaping):** "This box has more text than room after the font swap" is shown as **Check this**.
- **Keep the original PDF attached** to the document as a reference so the comparison is always available.
- **Exit criterion:** "Mark as checked" clears the highlights. This is the human-confirmation step, done per highlight, not as one button.

---

## 4. IDML import

IDML is a zip of documented XML. It is low technical risk, deterministic, and has high fidelity. The main user value is the few churches with designer-made InDesign layouts.

### First subset

| IDML part | Use | Mapping |
|---|---|---|
| `designmap.xml` | Document index, page size, facing pages | Document metadata |
| `Spreads/*.xml` → `Page` | Page `GeometricBounds` and `ItemTransform` | **Pages → panels**, if page width matches a panel preset. If one page is sheet-sized (common for tri-folds), split at fold guides or by preset snapping, same as §2b. |
| `Spreads/*.xml` → `TextFrame` | Geometry (from `PathGeometry` anchors × `ItemTransform`, spread coordinates → page coordinates), `ParentStory`, `PreviousTextFrame` / `NextTextFrame` | **Frames, with threading preserved.** This is IDML's big win over PDF. |
| `Spreads/*.xml` → `Rectangle` with `Image` | Image frames and placement | Picture frames |
| `Stories/*.xml` | `ParagraphStyleRange` > `CharacterStyleRange` > `Content` / `Br` / `Tab` | **Story → styled paragraphs** |
| `Resources/Styles.xml` | Paragraph styles with `BasedOn` chains (resolve fully), font, size, leading, alignment, indents, space before/after, tab stops (with leaders) | Native paragraph styles, **names kept**. Keeping names is a genuine fidelity gain worth labeling. |
| `Resources/Fonts.xml` | Font family/style | Font mapping and substitution, as for PDF |
| Character style overrides | Bold, italic, size, color | Inline character formatting (flattened if no character-style concept) |
| Basic swatches | RGB/CMYK/process colors | Simple colors |

**Units and coordinates.** IDML uses points with spread-centered coordinates. Compose the transforms, then convert to milli-points (×1000, round) at a single boundary.

### Deliberately skip (and label as skipped)

- **Linked images.** IDML does *not* embed them; it references paths. Prompt with: "Where is the Links folder?" Use the file picker and match by filename. Missing images become labeled placeholders.
- **Master spreads.** Convert master page items to ordinary items on each panel, labeled "came from a master page." Don't build a master system for this.
- **Tables.** Convert to tab-separated paragraphs with a **Check this** badge.
- **Anchored/inline objects, text on path, footnotes:** flatten or drop with a label.
- **GREP styles, nested styles, conditional text, XML tags, scripting labels, layers** (honor *hidden* layers by skipping them), **effects** (transparency, drop shadows), **object styles**, **text wrap**: skip.
- **.indd and .idml round-trip export:** out of scope.

---

## 5. Publisher .pub rescue

### Options

| Option | Fidelity | Licensing for an MIT app | Packaging | Risk |
|---|---|---|---|---|
| **"Save as PDF" from Publisher** (guided) | Best available, since Publisher renders itself. Then the PDF analyzer applies. | None | None | Only works while the user still has Publisher |
| **LibreOffice headless** (`soffice --headless --convert-to pdf` or `odg`), user-installed | Same parser as libmspub (LO uses it internally), plus LO's layout | **No impact.** It is an external program invoked by the user's choice. | Requires ~300+ MB LO install. Detect it and give a link. Headless is flaky to drive (profile locks, first-run, timeouts). | Medium operational risk, low engineering cost |
| **Bundled libmspub helper**: a small C++ executable implementing a librevenge drawing interface, emitting the same JSON dump schema | Structured text boxes with paragraphs and spans, positions, images, basic shapes. Better text structure than going via PDF. | **libmspub is MPL-2.0. librevenge is MPL-2.0/LGPL-2.1+.** MPL is file-level copyleft: an MIT app may ship it. You must provide source for the MPL files and any modifications, and include notices. Out-of-process is **not required** for license compliance. Do it anyway for crash isolation. | C++ toolchain on 3 OSes; boost (headers), **ICU** (heavy data, ~30 MB unless trimmed), zlib. Real CI cost. | Medium. libmspub is maintained but low-activity, so expect to patch it. |
| Rust-native .pub parser (OLE2 compound file plus undocumented records) | Low for years | Clean | Easy | **Avoid.** It is a multi-year reverse-engineering project. |

### Recommendation: staged "both," in this order

1. **v1:** guided "Save as PDF from Publisher" option, plus a **LibreOffice bridge**. If LO is installed, convert `.pub → PDF` and run the PDF analyzer. Zero new parsing code and one pipeline. Label it "Rescued through LibreOffice."
2. **v1.x:** if telemetry or support show real demand, build the **libmspub helper** emitting the PDF-dump JSON schema extended with paragraph boundaries. It reuses all the §2 heuristics, but with better paragraph and frame truth.

### Realistic .pub fidelity

| Element | Expected fidelity |
|---|---|
| Text content and frame positions | Good |
| Images | Mostly good |
| Basic fonts and sizes | Fair |
| Styles | Weak (no named-style reliability) |
| Linked text boxes | Unreliable; treat as unthreaded |
| Tables, WordArt, borders art, master pages, some versions' special objects | Poor or missing |

Promise "your words and rough layout back," not "your Publisher file."

---

## 6. Sequencing, first spike, and what to avoid

### Order: value per engineering risk for a small team

| # | Importer | User value | Eng. risk | Notes |
|---|---|---|---|---|
| 0 | **Real shaping (parley)** | Enabler | Medium | Hard prerequisite for honest fit labels (F3) |
| 1 | **PDF analyzer** | **Highest.** Every church has old bulletin PDFs from Word, Publisher, Canva, or Pages. | Medium-high (heuristics), mitigated by PDFium | Its dump schema becomes the hub for the other importers |
| 2 | **.pub via "Save as PDF" + LibreOffice bridge** | High (Publisher refugees) | **Low** (reuses #1) | Ships weeks after #1 |
| 3 | **IDML** | Low–medium (small audience) | **Low** (documented XML, deterministic) | Good contributor project; can run in parallel with #1 |
| 4 | **libmspub helper** | Medium (incremental over #2) | Medium-high (C++ packaging) | Only on demonstrated demand |
| (eval) | **.docx** | Possibly higher than IDML | Low–medium | Decide after reviewing the PDF spike corpus (see the Producer-string tally below) |

### First PDF spike (timebox: 2 weeks)

**Inputs: a corpus of ~30 real bulletins**, used with permission and stored as fixtures:

| Category | Count |
|---|---|
| Word-exported | 6 |
| Publisher-exported | 6 |
| Canva | 3 |
| InDesign | 3 |
| Pages / Google Docs | 2 |
| **Imposed printer spreads**, bi-fold and tri-fold, some with crop marks/bleed | 4 |
| Reader half-letter pages | 3 |
| Scanned or outlined-text | 2 |
| Deliberately broken (truncated, odd encodings) | 1 |

**Hand-written ground truth per file:**

- sheet type (reader or imposed) and fold;
- panel order;
- text per panel;
- 10 sampled lines with font size;
- image count.

**Build:**

1. PDFium helper process → JSON dump.
2. Rust grouping into lines, paragraphs, frames.
3. Sheet classifier.
4. A debug **SVG overlay**: page raster plus colored boxes for chars, lines, paragraphs, gutters, and panels. This also becomes the seed of the user-facing check view.

**Pass criteria (all required):**

| Area | Criterion |
|---|---|
| Text extraction | ≥ 98% character accuracy vs ground truth on text-based PDFs |
| Unreadable pages | Scanned/outlined/garbled pages detected **100%**, with no garbage text emitted |
| Font size | Within ±0.5 pt on ≥ 95% of sampled lines |
| Line bboxes | Within ±1 pt |
| Line grouping | ≥ 95% correct |
| Paragraph grouping | ≥ 85% correct, judged by eye on the overlay |
| Order-of-service | Tab/leader lines detected on ≥ 90% of bulletins that have them |
| Sheet classification | Correct on all reader-panel files. On imposed files, either correct or **it asks**. **Zero confident wrong answers.** |
| Images | All extracted with correct placement |
| Performance | < 2 s per page |
| Robustness | No app crash across the corpus plus a quick fuzz pass; helper crashes are fine if they are caught and reported |
| Packaging | PDFium loads and runs in CI on macOS (signed/notarized), Windows, and Linux |

**Fail signals:**

- Text accuracy can't reach 98% without per-producer hacks, which suggests evaluating the extraction route again.
- Packaging blocks on any OS.
- The classifier makes confident wrong calls. If so, redesign it to ask more often rather than tune thresholds.

**Byproduct:** tally the Producer strings in the corpus. This tally informs the .docx-vs-IDML decision (F5).

### Avoid entirely

- **Linking MuPDF or Poppler** (AGPL/GPL) into the distributed app.
- **Reading .indd**, and writing a **Rust-native .pub parser**.
- **Extracting or re-embedding fonts from PDFs.**
- **OCR in v1.** Use the tracing-guide underlay instead.
- **Inventing text threading** from PDF geometry.
- **Global accuracy percentages**, "AI magic" wording, and any promise of exact replication.
- **Confident silent guesses about panel order** on imposed sheets.
- **Reconstructing vector clip art** into editable shapes. Rasterize it.
- **A separate "template" object model** for imports (F1). Import into the normal document and the normal weekly flow.
- **Running PDFium in the UI process.**
- **More than ~6 inferred styles**, or exposing style-system jargon on the main path.