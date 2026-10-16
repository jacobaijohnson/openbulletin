# Beyond print: formatting depth, HTML both ways, and what else a bulletin system carries

Jacob, 2026-10-15 late: text formatting (markdown, rich
text), HTML in and out, and "what other aspects of the
bulletin should be incorporated in this type of system."

## 1. Inline formatting: the correction, then the build

**SHIPPED 2026-10-16 (G1b).** What shipped, against the
plan below: Paragraph carries runs (text + optional
bold/italic, None = inherit the paragraph style) with
full back-compatibility for the old { style, text } shape;
the shaped editor toggles with Ctrl+B / Ctrl+I and the
Bold / Italic toolbar buttons, typing inherits the face
at the caret, and toggles land around field chips, never
inside them; the PDF and every preview draw each run in
its face; IDML imports carry CharacterStyleRange runs,
and the PDF/.pub draft path turns a line whose face
differs from its block into a run of the block's
paragraph; .pub spans carry no runs of their own (the
reader's placement pen dissolves paragraph identity on
purpose; each span still lands as a run via its face).
Markdown: a `.md` file enters through the one open door
as a draft labeled "Notes or web text"; canvas paste
converts markdown markers to runs; the export door and
CLI `export-md` write the words as markdown. Run scope is
bold/italic only: no per-run color or size in this build.

The paste truth, recorded not faked: egui's clipboard is
TEXT-ONLY. A copy made in Word or a web page arrives here
as plain text (or, from tools that offer it, as markdown
text, which the paste conversion above honors). Truly
rich paste, carrying Word's own formatting across, waits
on a platform clipboard path that can read HTML/RTF
clipboard formats; until that exists, no build claims it.

Original plan text: Verified state today: a paragraph has
ONE style. There is no way to bold one word or italicize
a scripture citation inside a paragraph (completeness
audit gap #12; the model carries style id + string, no
runs). Real bulletins need both constantly. The build,
riding the G1 format migration so the model changes once:

- Paragraph gains runs: spans of text with their own
  bold/italic (and later color/size deltas), style still
  the paragraph's base.
- The shaped editor edits runs without exposing them as
  markup: select words, Ctrl+B / Ctrl+I (keymap-respecting),
  the chip rules stay for fields.
- PDF, previews, IDML/.sla export, and the import builders
  read and write runs. Imports stop guessing italic from
  font names once the dump schema carries it (G1).
- Rich paste: pasting from Word or a web page brings its
  bold/italic across as runs (plain text paste unchanged).
- Markdown: paste or import of markdown text converts
  **bold** and *italic* to runs; export to markdown is a
  Save-a-copy format for the text (for the church website
  and newsletter tools). Markdown is an interchange
  convenience, never the document model.

## 2. HTML export: the web bulletin

**SHIPPED 2026-10-16 (H1).** What shipped, against the
verdict below: the exporter lives in
`ob_extract::html_export` and rides the one export door
as "Web page (the words, to read and search on a
screen)" (the file takes the house name, the picked name
with "-web" on it) and the CLI as `export-html` (default
name likewise, beside the bulletin). One self-contained
page: semantic HTML in reading order (heading styles to
h1-h3 by name, then by size; runs to strong/em; tabbed
lines as paired what/who spans; field words resolved as
the reading copy resolves them), embedded style only,
no scripts, no external requests, readable with the
style ignored. Pictures are carried inside the file:
rasters transcoded at the web purpose's sizing (150 PPI
at the shown size, never upscaled; JPEG when opaque,
PNG when the picture has real transparency), SVG clips
and music systems inlined as vector with spoken names
(a music system is "Music: " + its hymn title), with
scripts and event handlers stripped on the way in. A
picture that cannot be carried leaves the plain note "A
picture belongs here; it is missing from the bulletin
file." in its place and is counted in the save line.
Credits are composed from the placed pictures' rights
records (the clipart landscape's section 5 rules): a
recorded credit line rides verbatim with its license
address appended as visible text when it carries none;
without one, a line is composed from title, creator,
copyright line, and license, the address again visible
text; a credit the bulletin already carries as words is
never printed twice; a picture that owes a credit
nobody recorded enough of to compose yields the honest
line "A picture in this bulletin is missing its
credit." Panel tracing pictures never appear (they are
scaffolding, not content). Two as-built notes: a placed
picture shows WHOLE on the web page (the Fill crop is a
paper decision about one printed size; the store keeps
the original either way), and the page names itself a
reading edition in its first line under the header, in
the words the verdict below asked for.

Verdict: yes, and it matters more than import. Churches
post bulletins on their websites; a PDF is a poor web
citizen (fixed pages, weak search, poor screen readers).
The HTML export is the bulletin's CONTENT as one
self-contained web page: real text in reading order,
searchable, screen-reader friendly, pictures embedded,
hymn and reading lines intact, credits lines included
(the rights machinery writes them here too). It is a
reading edition, not the print layout in a browser:
panels and imposition do not translate, and the export
says so in its own words. It rides the one export door as
another format, after G2 gives it pictures to carry.

## 3. HTML import: a rescue kind, like the others

Verdict: yes, later, with the standard honesty. An HTML
page (a Mailchimp newsletter export is the real-world
case: churches already write these) imports through the
one door into a draft bulletin: text blocks, headings,
pictures, bold/italic as runs. Layout fidelity is NOT
promised; CSS positioning does not survive, and the draft
review labels it like every other rescue. A tolerant
parser maps block elements to frames; scripts and forms
are dropped with a count, never silently.

## 4. What else belongs in a bulletin system (ranked)

Status (2026-10-16, G2): the three adopted extras below
remain planned; they ride the Extras batch. What shipped
in this neighborhood with G2 is the graphics pipeline
itself: picture boxes on the page, the recognition door,
the clip art library with its starter set, and
resolution-honest printing, recorded in section 6.

Adopted into the plan:
- **Large-print edition** as an export purpose beside the
  PDF ones: same content reflowed at 16pt+, for the
  congregants who need it. An aging congregation makes
  this a weekly, not a someday, feature.
- **Participant fields**: readers, ushers, greeters,
  communion assistants as This Sunday fields beside the
  sermon title (the J2 pattern), printed wherever the
  template puts them.
- **A privacy flag on prayer-list content**: "print" vs
  "spoken only", so a name meant for the spoken prayers
  cannot wander into the printed sheet by accident.

## 5. The administrator's eyes (Jacob, 2026-10-15)

**SHIPPED 2026-10-16 (A1).** What shipped, against the
plan below: a "Text size" button in the top bar opens a
"Text size and contrast" window with the three steps in
plain words (Standard / Large / Extra large), a live
sample line, and the high-contrast switch, labeled "High
contrast (black text on white)". The steps scale every
chrome text style (x1.0 / x1.2 / x1.4) with padding and
spacing a step behind; the canvas and previews are
document space and keep the document's own sizes, which
the window says in one line. High contrast is a full
palette variant reached through a Theme value on the
App: true black on white, every accent darkened until
the WCAG pairs asserted in the app tests hold (text
pairs at 7:1 or better, the canvas accent at 4.5:1 or
better), focus and selection strokes widened and
blackened; the standard theme's focus stroke is widened
too, since egui paints keyboard focus with its "active"
visuals. On the canvas, the corner square's grab zone
and the guides' grab tolerance grow with the text step
(the drawn square stays 9 px). Fixed-height audit: the
step sidebar and every long list already live in scroll
areas, and the Print preview's 300 px cap stays, because
it caps a picture inside its own scroll box while the
step around it scrolls. The honest residue: no human
has yet run the largest size through a full week of
work; that stays the standing human gate, exactly as
the plan below says.

Original plan text: The large-print edition serves the
congregation. The same fact applies at the desk: church
administrators skew
older, and the application itself must conform to their
eyes (Principle 9 applied to the user, not only to the
reader). This is an objective, not a polish item:

- A text-size setting for the app itself (three steps),
  scaling the whole interface, not only the canvas. The
  audit bar: every step of the weekly flow stays usable
  and unclipped at the largest size.
- A high-contrast option beside the standard palette.
- Nothing essential rides on a small target, a subtle
  color difference, or an unlabeled icon. Words on
  buttons (already the naming rule), generous click
  areas, visible keyboard focus everywhere.
- The canvas editing work (E1) already draws text at real
  sizes; the comfort settings govern the chrome around it.
- Verification is partly human (the standing deferred
  accessibility item): the settings ship with the design
  rules above, and the first older-administrator user is
  the real test, the same gate the GUI itself passed.

## 6. Image formats: wide door, conservative exports

**SHIPPED 2026-10-16 (G2).** What shipped, against the
plan below: a picture box is a frame kind on the page
(born empty with the words "No picture yet"; filled from
a file, from the clip library, or from the pictures a
rescue already carried). One geometry function in the
model decides how a picture shows in its box (Fill, its
sides trimmed by crop fractions, or ShowWhole), and the
canvas, the previews, the print writer, and the blur
check all read that same function, so the views cannot
disagree. The recognition door is the ob-image crate:
PNG, JPEG, WebP, GIF, BMP, and TIFF decode through the
image crate (EXIF orientation applied, so phone photos
do not land sideways); JPEG XL decodes pure-Rust through
jxl-oxide; SVG renders through resvg at whatever
resolution the caller asks for. **AVIF came out
convert-first, beside HEIC, by measurement:** the image
crate's AVIF support is encode-only, and decoding needs
dav1d through a system library this machine does not
have and the one-download Windows bundle does not carry
(the build fails at the dav1d pkg-config probe; the
evidence is in docs/LEGAL.md). AVIF and HEIC files are
recognized, named, and refused in plain words with the
convert-to-JPEG-or-PNG guidance, never silently dropped.
Printing: a picture is right-sized to its placed inches
times the copy purpose's PPI cap (never upscaled),
encoded JPEG at quality 85 when opaque, Flate with an
alpha mask when it has real transparency. **SVG sources
rasterize at that same target resolution (600 PPI when
the purpose sets no cap): pictures print at the
resolution for the copy's purpose.** A box whose picture
is missing prints the "No picture yet" placeholder and
is counted in the copy's status line; Check flags empty
boxes, missing pictures, and pictures under 120
effective dots per inch at their shown size (the
church's own rescued pictures live near 150, so the line
waits below them). The clip art library
(docs/research/clipart-landscape.md) shipped with it:
collections in a managed folder beside the Font Library,
name search, thumbnails, folder and zip intake with
every skipped file counted and explained, WMF/EMF
converted through a user-installed LibreOffice when one
is present and otherwise counted and left out, and a
bundled starter set of twelve public-domain/CC0 pieces
(see crates/ob-app/assets/clipart/manifest.json for its
per-piece record and the subjects still to find).

Original plan text: (Jacob, 2026-10-15, extended the
same night to JPEG XL.)

The recognition door takes every format a church office
can plausibly be handed: PNG, JPEG, WebP, GIF, SVG,
JPEG XL, and AVIF. Facts checked 2026-10-15: JPEG XL just
crossed into the mainstream (Chrome 155 enables it by
default this month via a pure-Rust decoder; Firefox 158
follows; Safari has carried it since 2023; iPhone ProRAW
uses it), it beats JPEG by roughly a third on photos, and
it can shrink an existing JPEG about 20% losslessly and
reversibly. Its Rust decoders are pure-Rust (jxl-rs,
BSD-3, the decoder Chrome and Firefox ship; jxl-oxide
beside it), so support costs no native dependencies.
AVIF (added the same night) is the web's high-efficiency
format: every browser decodes it and its compression at
web sizes is excellent, but in Rust its ENCODER is the
pure-Rust half (ravif) while DECODING binds dav1d, a
small BSD-2 C library, the same decoder the browsers
bundle. AVIF therefore joins the door in G2 with one
open measurement: if the dav1d native dependency keeps
the one-download bundle sane, it decodes in-app like the
rest; if not, it starts as a labeled convert-first. For
a print-first program JPEG XL is the better citizen
(lossless, high fidelity); AVIF is the web's format, and
the door takes both.

The asymmetry from the optimization ruling holds: the
asset store keeps originals untouched whatever their
format; exports transcode to what the destination
actually speaks. PDF cannot carry WebP or JPEG XL as
native image data today, so print and reading PDFs get
JPEG/Flate at the purpose preset, exactly as ruled. The
HTML export may serve WebP/JPEG XL with a JPEG fallback
now that browsers decode them. Encoding JPEG XL
ourselves is deferred: no destination needs it yet.
HEIC (the iPhone default) is the adjacent hard case:
patent-encumbered and library-poor in Rust; it stays an
honest "convert first" label until a clean path exists.

Adjudicated the same night: Jacob proposed converting
EVERYTHING to JPEG XL as the one internal standard. The
verdict is originals in the store, JPEG XL at the edges,
for three reasons. (1) HEIC cannot be converted at all
until it can be decoded, and decoding is the blocked step;
a pixel-lossless re-encode of an HDR photo would also be
far larger than the HEIC and would strand its HDR gain
maps. (2) Lossy sources (WebP, most HEIC) re-encoded to
JPEG XL either lose a generation (against the originals
rule) or grow. JPEG is the exception: its recompression
into JPEG XL is bit-exact reversible, so normalizing
JPEGs would lose nothing, but the roughly 20% saving does
not justify making the content-addressed store hold
anything but the true original bytes; recorded as an
option if asset sizes ever hurt. (3) Print never sees
the store format anyway: PDF export transcodes to
JPEG/Flate at the purpose preset regardless. Where one
standard format does pay is delivery: the web export
serves JPEG XL first with fallbacks. Platform check
(2026-10-15): all three desktop OSes now read JPEG XL at
the system level (macOS Sonoma Finder/Preview, Windows 11
24H2 Explorer/Photos, Linux via GNOME 45+ and KDE's
KImageFormats, GIMP 3, ImageMagick), so the format is a
safe citizen everywhere the app runs.

Recorded, not scheduled:
- **Announcement intake**: a simple submission path
  (form or email-in) landing in the Announcements step.
  Real workflow win; touches online surfaces, so it waits
  for the deployment story.
- **Archive search**: every past .obd searchable (find
  last Easter's readings, when a prayer request started).
  Grows naturally from the file library; medium value.
- **Lectionary lookup**: readings by date from a public
  source. Attractive, but licensing and source stability
  need their own pass before it is promised.
