# Fonts: what we may use, and how fonts get into the app

Jacob asked three questions (2026-10-15): which free fonts can we
actually use under our license, can the app make fonts easy to get
and install the same way on desktop and in the Docker/NAS shape, and
can fonts come out of imported files (a PDF or .pub that "has" the
font)? Researched against primary sources; summarized here in plain
words. This is design, not built yet.

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
  no metric twins. The app offers the closest free lookalikes and
  the admin judges with the preview below. Close is often fine for a
  heading; body text is where twins matter most.

## The font library (desktop and Docker behave identically)

Our layout engine deliberately uses only font files we hand it, so
the same bulletin lays out identically on every machine. Live system
fonts would break that promise (a NAS has different fonts than a
laptop). So fonts live in one app-managed folder, the Font Library:

- **Bundled starters** ship with the app (Liberation family today;
  a small OFL starter set later), each with its license text.
- **Add a font:** pick a .ttf/.otf. The app reads the font's real
  names and its built-in embedding permission bits (the OpenType
  fsType field: installable, editable, preview-and-print, or
  restricted) and says in plain words what that font may be used
  for. A copy goes into the library; the original stays untouched.
- **Find a font:** search the free catalog (an index shipped with
  the app; downloads come from the google/fonts repository or
  Fontsource, both license-clean), one tap installs into the
  library, license text saved alongside, file hash verified.
- **System fonts (desktop):** offered as "copy into the library,"
  never linked live, for the determinism reason above. In the
  Docker shape the library is a folder in the data volume, and the
  same Add/Find flows work from the browser.

Documents name fonts by family plus file hash. When an .obd travels,
fonts may ride inside the package only when their license or
embedding bits allow it (OFL and Apache: yes; installable/editable:
yes; preview-and-print: the font stays out and the receiver is told
which font to get; restricted: never carried, with a plain warning).
Print PDFs follow the same rule, and the writer now embeds: every
export carries the exact library faces it shaped with, as embedded
TrueType with real width tables, whenever the face's rights allow
travel; a face that may not travel falls back to a core font for
that face only. Text is WinAnsi-encoded today; CID embedding for
the full Unicode range is a later step. Full fonts are embedded
whole (a reader PDF is about a megabyte); subsetting comes later.

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
font it was laid out with. Remaining: the side-by-side comparison
preview in the import review (that belongs to the analyzer's
review screen).
