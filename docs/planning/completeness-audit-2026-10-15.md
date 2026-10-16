# Completeness audit, 2026-10-15

Jacob's question: what else do we need to build to be complete
but useful, from import to text and location to fonts? This
audit walks the shipped code stage by stage and inventories
what is missing, stubbed, placeholder, or silently lossy.
Every claim cites file:line or a doc. Read-only audit; nothing
was changed but this file.

Headline: the text pipeline (import words → guided editing →
shaped print) is real end to end. The graphic pipeline does
not exist: there is no picture a user can place, carry, or
print anywhere in the product. Most other gaps are fidelity
losses concentrated at one chokepoint, the extraction dump
schema.

## 1. Import

Every importer targets one schema, `ExtractionDump`
(crates/ob-extract/src/lib.rs:60-103). A `Span` carries page,
text, trust, x/y/w/h, font size, bold, and a font NAME. It has
no field for alignment, color, italic, rotation, or any image
data. Everything below follows from that schema plus each
reader's own limits.

**PDF (PDFium helper).** The helper emits text segments only
(crates/ob-pdfium-helper/src/main.rs:106-143); it never reads
image objects. A page with no text segments is `picture_only`
and survives only as a whole-page tracing PNG (render mode,
main.rs:150-189). Vector art, photos, and music notation on a
page are indistinguishable: all become the tracing picture.
The in-process lopdf scaffold (ob-extract/src/lib.rs:118-175)
produces spans with no positions, and the draft builder drops
position-less spans (draft.rs:181-186), so a scaffold dump
yields zero text blocks: the helper is load-bearing, not a
fallback.

**The draft builder is where fidelity goes to die.**
`document_from_dump` (draft.rs) rebuilds each block with:
- alignment hardcoded `Align::Left` and color hardcoded black
  for every imported style (draft.rs:392-410). Centered cover
  lines, right-aligned hymn numbers: all come back left and
  black, unbadged. The badge can say Matched while the
  centering is gone.
- one style per block, taken from the block's FIRST line only
  (draft.rs:382-410). A block mixing a 14pt bold first line
  with 11pt body lines collapses to one look.
- italic guessed from the font name containing "italic" or
  "oblique" (draft.rs:386-387).
- the sheet preset guessed from page 1 alone: the saddle
  booklet only when every page is the same size, the count is
  a multiple of 4, and the size is 396×612pt; otherwise
  "letter-portrait" with page 1's size (draft.rs:290-300).

**.pub native.** The strongest reader. Text with per-run
fonts, positions from Contents+Escher, tables via Quill TCD
as text, WordArt as text, masters badged Check this, and a
LossReport that counts everything (pub_text.rs:39-100+).
Pictures: `pictures_found` vs `pictures_carried` are counted
and each picture's place is marked with a placeholder span,
but the bytes are never carried, because the dump schema has
no image channel. Threading is honored at composition time
(pub_layout.rs:973-985, thread boxes fill sequentially), then
flattened by the draft builder (see §2). Native reading
engages only for Publisher 2002+ (0x2C) files; older files
need the bridge.

**.pub bridge (LibreOffice).** soffice converts to PDF and
the PDF path takes over (pub_bridge.rs), so its losses are
LibreOffice's conversion losses plus every PDF loss above.
Still never run live against a real install (recorded
2026-10-15).

**IDML.** Reads story text, per-paragraph size/font/bold,
frame translation and path bounds (idml.rs:325-398). Limits:
- paragraph positions inside a frame are fabricated: the
  frame height is divided evenly among its paragraphs
  (idml.rs:378-398). Real line positions are not in the file
  per paragraph, but the draft then treats these slices as
  measured places.
- `Justification` is never read (zero occurrences in
  idml.rs); rotation and scale in ItemTransform are dropped,
  only the translation is used (idml.rs:109-118), so a rotated
  frame lands unrotated.
- threading: spans are emitted per TextFrame from its
  ParentStory, with no NextTextFrame handling, so a story
  threaded across two frames is emitted whole for each frame.
  The draft then holds the words twice.
- italic is smuggled through the font name by appending
  "-Italic" (idml.rs:386-393) because the span has no italic
  field; the draft's name-sniffing happens to catch it.
- images set the page's picture_only flag (idml.rs:400-406);
  bytes not carried. Tables are not modeled; cell words ride
  the story text and the grid is lost.

**Scribus .sla.** Text frames, positions, sizes, bold from
face names (sla.rs). Picture frames are counted and named in
a note, never carried (sla.rs:37-39, 352-465). Paragraph
alignment is not carried (no dump field). Targets the
1.4-1.6 dialect; 1.7 renamed tokens are out of scope.

**Template Analyzer.** Builds its template by running the
base family member through `document_from_dump`
(analyzer.rs), so every draft loss above is baked into the
learned template. Its line rebuilding reads only RawText
spans (draft.rs `rebuilt_page_lines`), so a .pub text-rescue
file in the learning pile contributes geometry but its
unpositioned words never form lines.

**Music (all doors).** Notation has no representation
anywhere: no MusicXML/MIDI importer, no music frame kind, no
SVG anywhere in the codebase (zero hits). In a PDF, a music
page is picture-only (tracing PNG) when the notation is
image/vector, and gibberish "words" badged Check this when a
music font exposes glyphs as text. Corpus fact: 1-2
picture-only pages per bulletin are the music pages. They can
be traced over, never edited, reflowed, or re-split.

## 2. Text

**The model is paragraph-shaped.** A `Paragraph` is a style
id plus a text string (ob-model/src/lib.rs:118-129). There
are no runs inside a paragraph: no bold word, no italic
phrase, no link. `ParagraphStyle` carries size, bold, italic,
align, space_after, color, family, and a font hash
(ob-model/src/lib.rs:83-107). Line height is not a style
property: the PDF writer hardcodes 1.3 (ob-pdf/src/lib.rs,
draw_panel) and shaping calls pass 1.25-1.3. Alignment has
three values (Left/Center/Right); there is no Justify.

**Hyphenation does not exist.** Zero occurrences of "hyphen"
in any crate or doc. Narrow booklet columns wrap ragged with
no hyphen fallback; long words overflow and are flagged by
the fit check instead of breaking.

**Threading is import-time only.** The model lets several
frames name one story, but no frame carries a continuation
offset, and the PDF writer draws the ENTIRE story from its
first paragraph in every frame that references it
(ob-pdf/src/lib.rs:193-262, draw_panel). The canvas preview
is built the same way. So a shared story would print whole,
twice. Nothing in the shipped builders creates shared
stories (templates and paste give every frame its own story;
the draft builder makes one story per block, draft.rs), so
today threading exists only inside the .pub reader's
composition, where it is resolved into per-page spans and
then flattened. The editor has no thread display, no link or
unlink, and no way to repair a story that outgrows its box
except making the box bigger. EDITOR.md promises visible
threading "when the editor grows it": that growth has not
happened.

**Style color is a lie for print.** `ParagraphStyle.color`
exists and the draft sets it (always black), but the PDF
writer never sets a text color: draw_panel sets only the
shape-stroke gray (ob-pdf/src/lib.rs). A user-defined colored
style would preview (the preview path paints per-line faces)
and print black.

**Overflow in the PDF itself is silent.** draw_panel clips at
the frame's bottom edge (ob-pdf/src/lib.rs, "v0: clip at frame
bottom"). The Check step warns; a skipped Check prints a
truncated bulletin with no mark on the page.

**The dump-schema gaps already recorded are confirmed:**
no alignment field, no image channel (lib.rs:60-86). Add to
that list: no italic field, no color field, no rotation, no
per-span trust reason. DUMP_SCHEMA_VERSION is still 1.

## 3. Placement / location

What the canvas has (crates/ob-app/src/main.rs, EDITOR.md):
single-frame selection with drag move/resize, snapping to
guides + panel edges + panel centers via the tested
`snap_to` (ob-model/src/lib.rs), arrow-key nudges (0.01in /
0.1in), right-click exact Left/Top/Width/Height with fraction
parsing (`parse_inches`, ob-model), guide drag + right-click
exact position + lock, add-guide at the right-click point
(main.rs:3747-3770), box copy/paste/delete, zoom.

What it does not have, checked against EDITOR.md's own list:
- **Multi-select: absent.** Selection is one frame index
  (the raw-index state the closing audit flagged). The
  backlog records multi-select DEFERRED pending the selection
  model rework (findings-backlog.md, Opus R-series).
- **Align/distribute tools: absent** (zero occurrences). Six
  boxes are aligned by dragging each one to a snap, one at a
  time.
- **Frame-to-frame snapping: absent.** Snapping targets are
  guides, edges, centers only; EDITOR.md lists frame-to-frame
  snapping as next-phase.
- **Master (all-pages) guides: absent.** Guides live on one
  Panel (ob-model/src/lib.rs, Panel.guides). A 16-panel
  booklet's shared margin guide is placed 16 times.
- **Dragging guides out of the rulers: absent** (EDITOR.md
  "Not yet" list admits all three of these last items).
- **Rotation: absent from the model.** Rect is axis-aligned;
  a rotated source frame (IDML) is silently un-rotated on
  import (§1).
- **Layers: absent, consistently.** EDITOR.md's vocabulary
  section says layers are left behind on purpose; the model
  has no z-order field either (frames draw in vector order),
  so "bring to front" does not exist even informally. The
  absence is a decision, and it is applied evenly.

## 4. Fonts

Strong half: one library folder, copy-in (never live-linked),
SHA-256 identity, fsType translated to plain-words rights
(ob-fonts/src/lib.rs:27-100), font packs via the catalog,
system-font picker. `may_package()` gates which faces the PDF
writer embeds (ob-pdf/src/lib.rs, FontBook::build).

Weak half:
- **The font pin is inert.** `ParagraphStyle.font_sha256` is
  documented as pinning the exact font file, but every
  constructor sets it to None (main.rs:1983, main.rs:6238,
  ob-pdf/src/lib.rs:208) and nothing ever reads it. FontBook
  keys faces by (family, bold, italic) only. A same-named but
  different font file on the next volunteer's machine changes
  the metrics and the line breaks, and nothing warns: the
  Check step's missing-font finding is by family presence.
- **fsType enforcement is silent at the point of pain.** A
  PreviewAndPrint or Restricted face in the library is
  registered with the shaper and shapes the screen, but is
  excluded from embedding, so the PDF falls back (exact face →
  family regular → core Helvetica, FontBook::resource_for).
  An italic style whose italic face may not travel prints in
  the regular face. No Print-step line names which faces fell
  back.
- **Font collections (.ttc) are skipped** by the system scan
  (ob-fonts/src/lib.rs), which is where several platform
  families live.
- **Import matching is a 19-row name table** (draft.rs:56-93)
  plus lookalike judgment. Corpus reality: 217 distinct font
  name strings, mostly commercial (GillSansMT dominant). For
  most real bulletins the imported font does not match, the
  draft falls back to Liberation Sans in the style itself, and
  the look difference is the admin's problem to notice from
  the badge text.

## 5. Pictures and graphics: the state audit

**A picture frame does not exist.** `FrameKind` has exactly
two values, Text and Shape (ob-model/src/lib.rs:154-158). A
Shape frame is a placeholder rectangle: the PDF writer draws
it as a light-gray stroked outline (ob-pdf/src/lib.rs,
draw_panel + the 0.92 gray set in build_pdf). There is no
frame field for an image asset, no "place a picture" action
anywhere in the app (no insertion flow exists in main.rs),
and no crop/fit/fill concept.

**The only image in the product is the tracing underlay.**
`Panel.background_image` names a PNG file that lives BESIDE
the .obd (`assets/page-NN.png`); the app loads it through the
`image` crate restricted to PNG (crates/ob-app/Cargo.toml:
`image = { default-features = false, features = ["png"] }`),
draws it under the panel, and never prints it
(main.rs:3782-3798, load_bg_texture). Consequences:
- decode matrix for the whole product is: PNG only. The
  helper has the same restriction (ob-pdfium-helper
  Cargo.toml). JPEG, WebP, GIF, TIFF decode nowhere in shipped
  code; SVG is not referenced anywhere.
- the .obd does not contain the picture: save the file
  somewhere else, email it, or lose the assets folder and the
  underlays are gone (the app does say so on screen,
  main.rs:3124-3129).
- imports never extract embedded images from source files
  (§1): a photo in a PDF or .pub is, at best, part of the
  whole-page tracing render.

**What exports do with pictures:** there is nothing to carry.
IDML and .sla exports report shape frames and tracing
pictures as left behind, in plain words
(idml_export.rs:368-396, sla_export.rs:355-383). PDF export
prints Shape outlines and nothing else graphic.

**Music pages** (Jacob's ask) are the most common picture in
the real corpus and get the worst deal: see §1. A native
music path (MusicXML import → engraved frames that reflow
across panels) has no substrate to build on until picture
frames and .obd v1 assets exist; engraved-music rendering
itself (a Verovio-class engine) is a separate licensing and
scope decision on top.

## 6. File format

.obd v0 is one pretty-printed JSON document (ob-format
src/lib.rs), atomic save, 64 MB load refusal
(ob-format/src/lib.rs:20-28). FORMAT_VERSION is still 0
(ob-model/src/lib.rs). It stores no binary assets: there is
no asset table in `Document` at all, only the external
sidecar path in `Panel.background_image`. The v1 design in
docs/FORMAT.md (zip + the same document.json + SHA-256
content-addressed assets, migration fixtures per version) is
unbuilt. Until v1 lands, pictures and music cannot live
inside a bulletin file, full stop; base64-in-JSON would break
the format's own rules (diffable, emailable, 64 MB cap) and
nobody should do it as a stopgap.

Also missing from the format story: rolling backups (FORMAT.md
says plainly there are none; the autosave journal covers
crashes, not "yesterday's good version"), and a format-level
record of which app version wrote the file.

## 7. Print / export

**PDF** (ob-pdf): see §2 (color dropped, overflow clipped,
WinAnsi-only with non-WinAnsi characters becoming spaces,
ob-pdf/src/lib.rs `esc`) and §8 (size). The fallback ladder
for faces (exact → family regular → Helvetica) is silent.

**IDML export** carries words, boxes, styles, and style-level
Justification, and reports its losses in plain words:
shape frames, tracing pictures, frames whose story is missing
(written empty), threaded frames (each gets its own copy of
the words), unknown-style paragraphs (idml_export.rs:368-396).

**.sla export** reports the same loss classes
(sla_export.rs:355-383) and writes alignment in the 1.5/1.6
dialect.

**Round-trip asymmetry:** both exports WRITE alignment; our
own importers cannot READ it back (the dump schema has no
alignment field, §1). Export an .sla, re-import it, and every
centered line returns left-aligned. Scribus and InDesign keep
the information; we drop it on our own doorstep.

**Reconciliation with the recorded deferred lists:**
- findings-backlog.md: multi-select DEFERRED (confirmed
  absent, §3); "layout will differ" badge DEFERRED;
  accessibility/IME verification DEFERRED to a human;
  rich-text overlay DEFERRED as its own phase (that phase is
  §9 below).
- lens-synthesis-2026-10-15.md deferred items: shaped-caret
  in-place editing (confirmed: §9), frame-identity refactor
  over raw indices (confirmed: selection and editing still
  key on (panel, frame) index pairs, main.rs:1237), import
  field-binding, seasonal packs.
- ROADMAP.md unchecked: the text spike (threaded-frame
  editing with print-identical output, §9 is its design),
  "PDF place as graphic + trace layer" (the user-facing half
  is still unbuilt: a user cannot place a PDF page or any
  graphic into a bulletin, §5), signed installers, the 5-10
  church pilot, Docker/NAS shape, accounts. Those last four
  are product completeness, not pipeline completeness, and are
  listed here only so the reconciliation is total.

## 8. Size and optimization

**Boundary (Jacob's ruling, 2026-10-15, applied throughout
this section).** Optimization lives in the PDF exports, never
in the working file. The .obd and its stored assets always
keep the full-quality originals, so OpenBulletin or any other
tool opening the content later gets the full thing; IDML and
.sla exports also carry full originals. The print-copy PDF
gets print-safe reduction only: images right-sized to their
placed size at a 300 DPI cap, plus font subsetting, so it is
visually identical on paper. The reading PDF (email/website)
gets the full perceptual treatment: right-sizing plus
quality-targeted recompression, so it stays emailable and
web-publishable.

**What our PDF export embeds today, measured.** A fresh
16-panel booklet from the CLI (no pictures, template text
only): reader PDF 1,254,242 bytes, imposed PDF 1,251,892
bytes. The three largest streams are 415,921 + 414,569 +
410,821 bytes: whole Liberation faces, stored raw.
`add_embedded_font` writes the entire font file as the
FontFile2 stream (ob-pdf/src/lib.rs), and no stream in the
writer is compressed: the output contains no FlateDecode at
all. Fonts are ≈99% of the file; the actual content streams
are under 1 KB each. Each Liberation face is 370-416 KB on
disk, and FontBook embeds every (family, bold, italic) face
any style names, so a bulletin using a second family grows by
another ~0.4-1.2 MB.

**The real-world problem statement (corpus aggregates).**
The 17 real bulletin PDFs: min 0.97 MB, median 1.35 MB, mean
2.73 MB, max 7.05 MB, total 46.3 MB, 16 pages each. So a real
finished bulletin WITH photos and scanned music pages has a
median size roughly equal to our text-only export. Our empty
is their full. The mean and max show the other failure mode:
when pictures arrive unoptimized (a 7 MB bulletin), email and
church-website uploads are where it hurts.

**Pictures in export today:** none exist (§5), so there is no
image compression story at all, good or bad. The day picture
frames land, the boundary above must land with them, or print
and reading copies inherit raw camera files.

**Levers available in the Rust ecosystem, with licenses**
(license column: Verified = read from the project's own docs
in this audit; Reported = project's stated license, not
re-read line by line):
- **Font subsetting: `subsetter`** (MIT OR Apache-2.0,
  Verified). Pure Rust, no unsafe, built by the Typst project
  exactly for PDF embedding; subsets TTF/CFF to a glyph set.
  A bulletin uses a few hundred glyphs of a ~2,500-glyph face:
  expect a 410 KB face to land in the tens of KB. Must respect
  `FontInfo.no_subsetting` (ob-fonts/src/lib.rs, read from
  fsType) and only subset faces that may_package. This one
  lever takes the text-only export from ~1.25 MB to roughly
  150-250 KB. Alternative: `allsorts` (Apache-2.0, Reported),
  heavier, full OpenType shaping + subsetting.
- **Stream compression: FlateDecode via `flate2`/`miniz_oxide`**
  (MIT/Apache-2.0, Reported). Zero-risk, applies to content
  and font streams today even before subsetting; roughly
  halves raw TTF streams. There is no reason any export ships
  uncompressed streams.
- **Image right-sizing** needs no crate beyond the `image`
  crate we already carry (MIT/Apache): decode, resize to
  placed points × DPI cap (300 print / ~150 reading), re-encode.
  Today the app builds `image` with PNG only; JPEG/WebP
  features must be turned on for this to exist.
- **JPEG re-encoding: `mozjpeg`** (crate wraps the C library;
  BSD-style license, Reported). Best-in-class quality-per-byte
  and the quality-targeted search the reading PDF wants, at
  the cost of a native build (C compiler, nasm for SIMD).
  Pure-Rust fallback: the `image` crate's baseline JPEG
  encoder (MIT/Apache), visibly worse ratio at the same
  quality. This is a real build-complexity tradeoff, not a
  licensing one.
- **Lossless PNG recompression: `oxipng`** (MIT, Verified).
  Pure Rust. Inside a PDF, PNG pixels travel as FlateDecode
  image XObjects, so the win is recompressing stored samples;
  oxipng proper pays off for standalone PNG assets and any
  future web export.
- **Palette quantization: careful.** The quality engine
  everyone uses, libimagequant (pngquant), is GPL/commercial
  dual-licensed (Reported): it does not belong in an
  MIT/Apache-2.0 permissive project. The pure-Rust
  alternative is `color_quant` (MIT, NeuQuant; the quantizer
  the `image` crate itself uses), at lower quality. Print
  path should never quantize anyway (boundary: print-safe
  means lossless-or-right-sized only); quantization is a
  reading-copy lever, with a quality floor, if it is used at
  all.
- **WebP**: not embeddable in PDF (PDF image XObjects are
  JPEG/JBIG2/JPEG2000/CCITT/Flate). WebP matters only for a
  future HTML/web export, where `image`'s encoder or the
  `webp` crate (libwebp, BSD, Reported) applies. Do not let
  anyone sell WebP as a PDF size lever.

**Where each lever applies, under the boundary:**
- .obd + assets: never. Originals only. (Also IDML/.sla.)
- Print PDF (today's imposed export): subsetting + Flate
  now; image right-size at 300 DPI when pictures exist.
  Nothing lossy-perceptual.
- Reading PDF (today's reader export): subsetting + Flate +
  right-size at ~150 DPI + quality-targeted JPEG re-encode.
- The export functions today take no profile/flavor argument
  (ob-pdf `export_reader_pdf` / `export_imposed_pdf`); the
  imposed/reader split maps cleanly onto print/reading, so
  the boundary can be implemented as per-function policy
  without new UI.

## 9. Design note: the shaped canvas editor (next build)

**How the inline editor works today.** State: `editing:
Option<(usize, usize)>` (panel and frame raw indices),
`edit_text: String`, `edit_styles: Vec<String>`, `edit_salt`
(the story id), in crates/ob-app/src/main.rs:1237-1250.
`begin_edit` (~main.rs:2790) clones the story, snapshots the
document for undo, and joins the paragraphs with '\n' into
one buffer. The widget is an egui `TextEdit::multiline`
placed exactly over the frame's rect (main.rs:3547-3578),
rendered in ONE egui proportional font at the FIRST
paragraph's size clamped to 10-20pt: not the document's
family, not shaped, one size for every paragraph, dot leaders
and real line breaks invisible while typing (the shaped
rendering is hidden during the session). `cursor_paragraph`
(main.rs:6514) recovers the current paragraph by reading
egui's TextEditState cursor and counting newlines before it;
`sync_edit_styles` (main.rs:2615) keeps the parallel style
vec aligned as lines split and merge. `commit_edit`
(main.rs:2815) splits the buffer on '\n', zips lines with
`edit_styles` (missing entries default to "body"), replaces
the story's paragraphs wholesale, and records ONE undo step
from the session snapshot. Escape cancels; since J3 a changed
cancel keeps the discarded words for one-click restore.

**The field-token problem.** The buffer holds the RAW story
text, including tokens like `{Church.pastor}`. Resolution
happens only at draw/export (`resolve_fields`,
ob-model/src/lib.rs). So the user types around machinery they
can see but not understand, and can silently break a token
(lens review, Wonder lane 1 F3 / lane 2 F6). Any shaped
editor must solve this, not inherit it.

**What parley gives us, and what ob-text keeps.** Parley
0.12's `Layout` carries lines with text ranges, glyph runs
with x offsets, and cursor/selection geometry (point →
cursor, cursor → rects). ob-text's `Shaper` builds that
Layout in a private method and throws it away, returning
only `Vec<ShapedLine { text, width }>` (ob-text/src/shaping.rs).
So today there is NO caret geometry, NO hit testing, and NO
per-cluster positions anywhere above parley. The editor build
starts in ob-text, not in the app.

**A concrete buildable design.**
1. **ob-text grows a `TextLayout`**: per paragraph, build the
   parley Layout once (style-resolved spec, frame width) and
   expose lines with per-cluster (byte range, x, width),
   line baselines, `caret_rect(byte_offset)`, and
   `byte_offset_at(point)` on top of parley's own cursor
   APIs. Cache per (paragraph text, style id, width); a
   bulletin frame holds a handful of paragraphs.
2. **Render what prints.** The canvas keeps drawing the
   shaped lines through the existing preview path DURING the
   session; the editor adds only a caret rect, selection
   rects, and a hidden egui focus sink that receives keys and
   text input. No TextEdit widget. Reflow on each keystroke
   is a re-wrap of the edited paragraph only; frame_fit
   already exists for the overflow mark.
3. **Tokens become chips.** Edit operations live in the raw
   buffer (commit path unchanged), display lives in resolved
   space. Build the raw↔resolved byte map during resolution
   (resolve_fields is a pure replace; instrument it to also
   emit spans). A token renders as a shaded chip with its
   resolved text. The caret steps over a chip atomically;
   Backspace at a chip selects it whole, a second press (or
   Delete) removes it whole, and undo restores it. Chips are
   not editable in place: changing one means the step fields
   or the profile, which is where that fact lives. This turns
   the token hazard into the app's best teaching surface.
4. **Semantics preserved, exactly.** Session = one undo
   step; Done or click-away commits; Escape cancels with J3
   recovery; the buffer is still the same '\n'-joined string
   and `edit_styles` still rides alongside, so commit and the
   Announcements sync do not change. Scope: ONE frame at a
   time (as today), paragraphs keep their single style (the
   model has no runs; the editor must not invent them).
5. **Honest risks.** IME composition must go through egui's
   IME events and is the flakiest part on real platforms (the
   backlog already defers IME verification to a human);
   selection across paragraphs is in scope, across frames is
   not; screen-reader access to a hand-rolled caret is worse
   than TextEdit's and should be recorded as a cost, not
   discovered later.

## 10. The gaps, ranked

Ranked by (how often a church office hits it) × (damage when
it does). Evidence in the sections cited.

1. **No picture frames exist.** No kind, no placement UI, no
   asset storage, no print path; imports count pictures and
   drop them. Every real bulletin has cover art, a logo, or
   photos. (§5, §1, §6)
2. **Import fidelity dies at the dump schema and the draft
   builder.** Alignment, color, italic, rotation have no
   channel; imported styles are hardcoded Left/black from the
   first line of each block. Every rescue, and every learned
   template, needs manual re-dressing. (§1, §2)
3. **Export size: 1.25 MB for a text-only bulletin.** Whole
   raw fonts, zero stream compression. The reading copy is
   corpus-median-sized while empty. Fix exists, is licensed
   clean, and is boundary-approved. (§8)
4. **Threading has no document-level existence.** Import
   flattens it; the model would print a shared story whole in
   every frame; the editor cannot show or repair a thread.
   Multi-column bulletins are the .pub corpus' own pattern.
   (§2)
5. **The editor is unshaped and token-blind.** One egui font,
   first-paragraph size, raw tokens editable and breakable,
   real breaks invisible while typing. Hit in every editing
   session by every user. Design ready in §9.
6. **Font rights enforcement is silent, and the font pin is
   inert.** Restricted faces shape the screen and vanish at
   print; font_sha256 is never written or read, so cross-
   machine font swaps change the layout unannounced. (§4)
7. **Music has no home.** Notation pages are trace-only
   pictures (1-2 per real bulletin); notation-software
   exports have no import path; the pages can never be
   edited or reflowed. (§1, §5)
8. **Placement depth:** no multi-select, no align/distribute,
   no frame-to-frame snapping, no master guides, no ruler
   drag-out. Template work is one box at a time. (§3)
9. **.obd v1 is the gate under #1 and #7.** v0 cannot hold an
   asset; tracing pictures are sidecar files that vanish when
   the .obd travels alone. (§6, §5)
10. **Round-trip asymmetry:** our IDML/.sla exports write
    alignment our importers cannot read back; IDML import
    fabricates paragraph positions by slicing and drops
    rotation without a note. (§1, §7)
11. **Print fidelity leftovers:** style color never prints;
    PDF clips overflow silently when Check is skipped; face
    fallback at print is unnamed. (§2, §7, §4)
12. **Text niceties:** no hyphenation, no justified align, no
    line-height control, no intra-paragraph runs. Real, but
    bulletins live fine without them today. (§2)

Signed installers, the pilot, and the Docker shape remain
ROADMAP items (§7); they gate adoption, not pipeline
completeness.
