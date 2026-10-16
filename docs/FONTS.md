# Fonts: what we may use, and how fonts get into the app

Jacob asked three questions (2026-10-15): which free fonts can we
actually use under our license, can the app make fonts easy to get
and install the same way on desktop and in the Docker/NAS shape, and
can fonts come out of imported files (a PDF or .pub that "has" the
font)? Researched against primary sources; summarized here in plain
words. Written as design first (2026-10-15); the Font Library, the
catalog, the packs, and the rights gate described here are now
built and tested.

## The short answers

1. **Yes to free fonts, with room to spare.** Nearly every font in
   the Google Fonts catalog (~1,900 families) is under the SIL Open
   Font License (OFL) or Apache 2.0. Both let us bundle the font in
   the app, let users download it from inside the app, embed it in
   .obd files and in print PDFs, and use it commercially. OFL asks
   three things: keep the copyright note and license text next to
   the font file, never sell the font by itself, and if we ever
   modified a font it could not keep its Reserved Font Name (we do
   not modify fonts). Documents made WITH an OFL font carry no
   restrictions at all. That all sits comfortably inside our
   MIT OR Apache-2.0 project.
2. **One font library, same on desktop and server.** See below.
3. **Imported files give us font NAMES, not font files.** A PDF does
   embed pieces of its fonts, but they are subsets (only the letters
   that document used) and usually commercial fonts whose licenses
   do not allow lifting them out for reuse. A .pub or IDML file
   carries font names in its style definitions, not font programs.
   Names are exactly what the matching flow needs.

## What the real corpus uses (measured, 2026-10-15)

The 17 real bulletins use, by text volume: Gill Sans MT (regular,
bold, italic), Bookman Old Style, Franklin Gothic Heavy, Arial,
Times New Roman, Rockwell, Calibri, Tahoma. 217 distinct name
strings including subset tags. Every headliner is a commercial
Microsoft/Monotype font. None can be downloaded or redistributed by
us. The subset tags are visible right in the data (a name like
`UOETYK+GillSansMT` means "a sliced-out piece of Gill Sans"), which
is the concrete proof of short answer 3 for these files.

## Matching: exact twins first, lookalikes second

- **Same font, free:** the name is itself an OFL/Apache font (common
  for bulletins made with Google Fonts or Linux tools). One tap
  downloads the real thing.
- **Metric twins:** some commercial fonts have free clones with
  identical letter widths, so text wraps in exactly the same places:
  Arial -> Liberation Sans, Times New Roman -> Liberation Serif,
  Calibri -> Carlito, Cambria -> Caladea, Georgia -> Gelasio. The
  rebuild is line-for-line faithful even though the letterforms are
  a hair different.
- **Lookalikes:** Gill Sans, Bookman, Franklin Gothic, Rockwell have
  no metric twins. A lookalike shares the feel, not the letter
  widths, and is always called a lookalike, never a twin or a match.
  The one lookalike in the table today is Gill Sans (every spelling
  the corpus uses, GillSansMT included) -> Cabin, an OFL family drawn
  by Pablo Impallari after the typefaces of Edward Johnston and
  Eric Gill. A commercial font with no twin and no lookalike in the
  table prints in the house default, Liberation Sans, until the
  church adds the real font.

## Wanted fonts: the real name is kept (2026-10-16)

Jacob's ruling: when an imported bulletin names a font the library
does not have, substitute a similar free font, BUT note the actual
font name, so the user can find and install the real font
themselves; and once it is installed, documents must use it with no
editing. Built that way:

- Styles an import creates carry the SOURCE family name from the
  file ("Gill Sans MT"), never the substitute's name. The document
  records what it was made in.
- The document also gains a wanted-fonts record (`wanted_fonts` in
  the .obd): each font the library did not hold at import time, with
  the free family that stood in for it then. The import review
  shows one "Check this" line per wanted font, in these words:
  "This bulletin was made in Gill Sans MT. We do not have that
  font, so its words are shown in Cabin for now. If you add Gill
  Sans MT in Fonts, this bulletin will use it by itself."
- The stand-in is chosen fresh every time text is shaped, painted,
  or printed, by one resolution chain (`ob-fonts`, walked by the
  draft builder, the text shaper's one layout path, the PDF writer
  and its Check oracle, and the app's preview painters): the real
  family if the library holds it now, else its metric twin, else
  its lookalike, else the house default. So adding the real font
  later upgrades every old bulletin by itself, with no document
  edit, and the Check step's wanted-font finding (the same words)
  clears itself.
- The app never points at font download sites. A church's fonts
  come from the places fonts legitimately come from: the free
  catalog inside the app, the computer's own fonts copied in, or
  the church's own purchases and subscriptions, added as files.

A note on Gill Sans MT specifically, because the corpus runs on it:
it is Monotype's commercial font. Web sites offering it as a "free
download" are handing out Monotype's own file under a personal-use
label; neither we nor our users should take fonts from there. The
legitimate ways a church ends up with it: it ships with Microsoft
Office (most church office computers already have it, and the Fonts
panel can copy system fonts into the library), or Monotype sells it
directly. The free lookalike is Cabin, above.

## The font library (desktop and Docker behave identically)

Our layout engine deliberately uses only font files we hand it, so
the same bulletin lays out identically on every machine. Live system
fonts would break that promise (a NAS has different fonts than a
laptop). So fonts live in one app-managed folder, the Font Library:

- **Bundled starters** ship with the app (Liberation family today;
  a small OFL starter set later), each with its license text.
- **Add a font:** pick a font file (which formats, next section).
  The app reads the font's real names and its built-in embedding
  permission bits (the OpenType fsType field: installable, editable,
  preview-and-print, or restricted) and says in plain words what
  that font may be used for. A copy goes into the library; the
  original stays untouched.
- **Find a font:** search the free catalog (the Fontsource index,
  read live; downloads come from the google/fonts repository or
  Fontsource, both license-clean), one tap installs into the
  library, license text saved alongside, file hash verified.
  Cabin, the Gill Sans lookalike, is in that catalog under the
  OFL-1.1 license and installs by this same road.
- **System fonts (desktop):** offered as "copy into the library,"
  never linked live, for the determinism reason above. In the
  Docker shape the library is a folder in the data volume, and the
  same Add/Find flows work from the browser.

Documents name fonts by family plus file hash. When an .obd travels,
fonts may ride inside the package only when their license or
embedding bits allow it (OFL and Apache: yes; installable/editable:
yes; preview-and-print: the font stays out and the receiver is told
which font to get; restricted: never carried, with a plain warning).
Print PDFs follow the same rule: every export embeds the library
faces it shaped with, as CID-keyed TrueType (Type0/CIDFontType2,
Identity encoding, a ToUnicode map of the characters actually
drawn), subset to the glyphs used whenever the face's rights allow
subsetting, whole-face when they do not or the subsetter declines;
a face that may not travel at all falls back to a core font for
that face only. (One boundary: CFF-flavored OpenType faces keep
the older simple-font embedding; see docs/LEGAL.md.)

## Font file formats (Jacob's rulings, 2026-10-16)

Users supply fonts themselves; the app never bundles or links
commercial fonts. What the Fonts doors accept:

- **.ttf**: full support.
- **.otf**: accepted, both outline flavors (TrueType-flavored and
  CFF-flavored; a CFF face keeps its recorded print path in the
  PDF writer).
- **.zip packs**: already in; every file inside is counted, and a
  pack can carry collections and refusals like any other door.
- **System fonts**: copied into the library, never linked live.
- **.ttc collections**: a collection is several fonts sharing one
  file. Intake splits it and copies each face in as its own font
  file, named after the face itself, so the library stays one file
  per face. (The splitter copies each face's tables byte for byte
  and validates every result by parsing it before it is believed.)
- **WOFF / WOFF2**: refused, in these words: "That is the web
  version of a font. OpenBulletin prints, so it needs the TTF or
  OTF version of the same font."
- **Type 1 (.pfb / .afm)**: refused plainly as a format too old
  for OpenBulletin to use; the TTF or OTF version of the same font
  works.

Every refusal names the file it refused and counts in the door's
result, in the Fonts panel and in `fonts add` alike. Nothing is
silently skipped.

## Live preview and comparison

PDFium, already our extractor, can also draw a page of the original
PDF as a picture. The import review therefore shows: the original
page picture on one side, our rebuilt page on the other, with a
swipe between them (already the analyzer's design). For font
matching specifically, the same line of text is shown in the
original and in each candidate font, big enough to judge. The admin
picks; nothing is silently substituted.

## Guardrails

- No font is ever extracted from an imported file for reuse.
- No font is bundled or offered for download unless its license is
  OFL, Apache, UFL, or public domain, with the license text stored
  next to it in the library.
- fsType bits are a signal inside the font's license, not the whole
  license; when they disagree with a known license, the stricter
  reading wins and the user sees a plain-words note.

## Status (2026-10-15)

Built: the `ob-fonts` crate (scan a library folder, read family and
face names, fsType rights with plain-words explanations, SHA-256
identity, add-with-dedup) and CLI access (`openbulletin-cli fonts
list|add`). Verified against the bundled Liberation faces, whose
permission bits read Installable, matching their OFL license.
Also built: styles name a font family (and may pin its SHA-256);
the PDF path registers every library font with the shaper and shapes
each style in its named family. The catalog works from the command
line (`fonts search`, `fonts get`): Fontsource for search and
license facts, downloads from the google/fonts repository itself,
four core faces per family, license text saved beside them. Its
transport tries a direct connection first and the proxy from the
standard environment variables second, and trusts the operating
system's certificate store (proxied church networks included).
The in-app Fonts panel lists the library with a live preview line
drawn in each font's own face, the plain-words rights, and an
Add-a-font button; first run seeds the library with Liberation.
The panel also searches the catalog and installs from it (on a
worker thread, so the window never freezes), and saving or
exporting stamps each style with the fingerprint of the library
font it was laid out with.

Also built (the review-confirmed build order, 2026-10-15): italic
faces bundled and an italic style in the model; PDF exports embed
the real library fonts; the Church Starter Pack (Carlito, Caladea,
Gelasio, Lora, Playfair Display) installs in one tap from the panel
or the CLI; the panel adds fonts by multi-file pick, by zip pack,
and by copying in from a list of the computer's own fonts, with a
plain-words caution when a font did not come from the free catalog
and its license is not clearly shareable. One known limit: three
pack families are variable fonts and currently shape at their
default weight only; the static families carry all four faces.
Remaining: the side-by-side comparison preview in the import
review (that belongs to the analyzer's review screen).

Wanted fonts landed later on 2026-10-16 (see the section above):
styles keep the real source family name, documents carry the
`wanted_fonts` record, and one resolution chain in `ob-fonts`
(exact, then metric twin, then lookalike, then the house default)
is walked at draft-build time, by the shaper's single layout path,
by the PDF writer and Check's glyph oracle, and by the app's
preview painters, so a font added later upgrades old documents
with no edits. The substitution table moved into `ob-fonts` with
it, gaining its first distinctly-typed lookalike (Gill Sans ->
Cabin). Font intake learned collections (.ttc split into per-face
files at the door) and the plain-words refusals for web fonts
(WOFF/WOFF2) and Type 1.

DejaVu Sans joined the pack 2026-10-16, for one plain reason: it
carries the marks the Liberation faces do not. O2 read the cmap of
every bundled Liberation face and found neither the Latin cross
(✝, U+271D) nor the check mark (✓, U+2713); DejaVu Sans has both,
verified by reading its own cmap, and the catalog keeps that
verification as data (`VERIFIED_GLYPH_COVERAGE`), so when a
bulletin uses a mark the printing face lacks, Check can say
"DejaVu Sans has this letter; it is in the Fonts list." DejaVu is
not in the google/fonts repository, so its install takes its own
route: the DejaVu project's own release (the four core faces plus
its LICENSE file). Its license needs two true sentences: the
Fontsource catalog records the family as OFL-1.1, and the font's
own embedded license is the DejaVu license (the Bitstream Vera
terms, DejaVu changes in the public domain). The license note
saved beside the faces carries the project's own text, so the
record never rests on the catalog's classification alone.
