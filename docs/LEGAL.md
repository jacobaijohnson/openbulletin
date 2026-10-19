# Legal posture and scope guards

OpenBulletin is MIT OR Apache-2.0 (dual), chosen by the owner for
permissive use plus Apache's express patent grant. This file records
the standing checks that keep it clean. It is a project note, not
legal advice.

## Dependency license audit (2026-10-15)

`cargo metadata` over the full tree: 524 packages (recounted 2026-10-15 when IDML import added quick-xml and zip as direct dependencies; both were already in the tree transitively in substance). Every package is
MIT, Apache-2.0, MIT OR Apache-2.0, Zlib, BSD, ISC, Unicode-3.0, or
(for egui's bundled default fonts) OFL-1.1 / UFL, which cover font
data, not code. The single multi-license flag, r-efi (present at two
versions), is "MIT OR Apache-2.0 OR LGPL-2.1-or-later": an OR
choice, satisfied by choosing MIT / Apache-2.0. No GPL, AGPL,
LGPL-as-requirement, or SSPL anywhere. Re-run this audit whenever
dependencies change materially. (Recounted 2026-10-15 during the
closing audit; the earlier count of 475 had drifted.)

Deliberately excluded on license grounds: MuPDF (AGPL) and Poppler
(GPL). PDF reading is PDFium (BSD-style, Chromium) in an
out-of-process helper; the PDFium binary is NOT in this repository
(it is fetched to a local toolchain folder, per docs/EXTRACT.md).

Export size work (2026-10-16): ob-pdf now compresses every PDF
stream with FlateDecode via `flate2` (MIT OR Apache-2.0) on the
`miniz_oxide` backend (MIT OR Apache-2.0); both were already in
the tree transitively and are now direct dependencies, so the
package count is unchanged. Font subsetting was evaluated
against the `subsetter` crate (Typst project; license verified
in its crate metadata as MIT OR Apache-2.0) and DEFERRED, not
on license grounds but on format grounds: subsetter removes
the font's cmap table by design and requires the PDF writer to
embed fonts as CID-keyed fonts with its own ToUnicode CMap.
Our writer emits WinAnsi simple fonts whose glyph mapping runs
through the embedded cmap, so a cmap-less subset would render
the wrong glyphs (or none). Subsetting waits for a deliberate
CID-embedding migration of the writer; it must not be bolted
onto the WinAnsi path. So `subsetter` stayed out of the tree
at that writing. That conclusion is superseded: the note
directly below records that the CID migration and subsetting
shipped on 2026-10-16, and `subsetter` 0.2.6 is a dependency
of ob-pdf today.

CID migration and subsetting shipped (2026-10-16, O2): the
migration the note above waited for has landed. ob-pdf now
embeds TrueType faces as CID-keyed fonts (Type0 with a
CIDFontType2 descendant, Identity-H encoding, an explicit
ToUnicode CMap written from the characters drawn, and glyph
IDs as CIDs with an Identity CIDToGIDMap), so the embedded
program's own cmap is no longer on the glyph-mapping path.
`subsetter` 0.2.6 IS now a dependency of ob-pdf; its license
was re-verified in the crate's own Cargo.toml as MIT OR
Apache-2.0 before the dependency was added. Each embedded
face is subset to the glyphs the document draws in it; a face
whose packaging rights forbid subsetting embeds whole in the
same CID shape, and a subsetting error falls back to whole
embedding for that face, so an export never fails over
subsetting. The unembedded core-14 fallback keeps the simple
WinAnsi path. Effect: the 16-panel sample booklet dropped
from 654,910 bytes (compressed, whole faces) to about 31 KB,
and characters outside Windows-1252 that the chosen face
actually has (Greek letters in Liberation Sans, for example)
now print and extract as themselves instead of spaces.

.obd v1 packaging (2026-10-16): the `zip` crate (v2, MIT) is
now also a direct dependency of ob-format, which writes and
reads the v1 zip container, and `sha2` (MIT OR Apache-2.0)
is a direct dependency of ob-model for the assets'
fingerprints. Both were already in the tree; the permissive
posture above is unchanged and no new license family
entered.

Picture decoding (2026-10-16, G2): the new ob-image crate
decodes pictures through `image` (MIT OR Apache-2.0, already
in the tree), `jxl-oxide` 0.12 (JPEG XL, pure Rust, MIT OR
Apache-2.0 for the whole crate family), and `resvg` 0.45.1
for SVG. The resvg pass the build planned for: older resvg
releases were MPL-2.0, but resvg and `usvg` 0.45.1 state
"Apache-2.0 OR MIT" in their own Cargo.toml metadata, and
their rendering dependencies are MIT, Apache-2.0, or
BSD-3-Clause (tiny-skia). `cargo metadata` re-run for this
build: 586 packages, no new copyleft family, the only
LGPL mention still the r-efi OR choice recorded above.
AVIF decoding was evaluated and NOT taken: the `image`
crate's AVIF decode feature binds dav1d (a C library)
through a system-library lookup (dav1d-sys needs libdav1d
>= 1.3.0 via pkg-config), which is absent here and on the
stock Windows bundle runner, so AVIF files are recognized
and refused with convert-first guidance instead, beside
HEIC. No dav1d code or binary enters the tree.

HTML import (2026-10-16, H2): the web-page door parses
with `html5ever` 0.39 (MIT OR Apache-2.0, verified in the
crate's own metadata) over `markup5ever_rcdom` 0.39 (MIT
OR Apache-2.0); their whole closure (markup5ever, tendril,
web_atoms, string_cache, phf, log) is MIT, Apache-2.0, or
MIT OR Apache-2.0. The `scraper` crate was evaluated
first and NOT taken: scraper itself is ISC, but its
selector machinery depends on the `selectors` and
`cssparser` crates, which are MPL-2.0, and this build
walks the DOM directly and needs no selector API, so no
copyleft family enters the tree for it. `cargo metadata`
recount for this build: 603 packages (was 589), no new
license family; the only LGPL mention is still the r-efi
OR choice recorded above. The importer makes no network
calls: pictures are carried only when their bytes are
inside the page itself (data: URIs), and web-addressed
pictures are counted, never fetched.

Music engraving (2026-10-16, M1): hymn notation is engraved
by Verovio (LGPL-3.0), run unmodified and out-of-process by
our own helper binary (ob-verovio-helper), the same posture
as PDFium: nothing of Verovio is linked, copied, or modified,
and the helper speaks to the app over versioned JSON. Checked
2026-10-16: Verovio publishes no pre-built standalone binary
for Windows (its command-line book says binaries exist only
through Homebrew on macOS); the official Windows distribution
is the Python wheel on PyPI (wheels for Windows, macOS, and
Linux), which needs a Python runtime to run. Consequence,
recorded rather than papered over: the Windows bundle ships
our helper but no engraver, so on a stock Windows machine a
music file opens with its notes standing undrawn and the app
says so in plain words. When an engraver is staged in a later
bundle it will be the unmodified official artifact with its
LGPL-3.0 license text and a note offering its source, the
same treatment PDFium gets today. Our own new dependencies
for this build are quick-xml 0.36 and zip 2 (both already in
the tree; see the .obd note above), so no new license family
entered: `cargo metadata` recount is owed at the next
material dependency change. MusicXML itself is a W3C
Community Final Specification (Music Notation Community
Group), free to implement. The one music file used in
development testing (a Bach chorale edition under
CC BY-NC-SA 4.0) lives outside this repository and never
ships, like the bulletin corpus.

## The Publisher reader and the LibreOffice bridge

Our Publisher (.pub) reader is an independent implementation
written from documented format facts: Microsoft's published
container and drawing-layer documentation ([MS-CFB],
[MS-ODRAW]) and facts from the public reverse-engineering
record (field layouts and constants; facts are not protected
expression). It is not a clean-room process and we do not call
it one: the same project read the record and wrote the code.
No libmspub code was copied or adapted.

The LibreOffice bridge distributes nothing. OpenBulletin
never bundles, downloads, or ships LibreOffice; when a user
has it installed, the app asks that program, through its own
command line, to convert the user's file, the same as the
user running it by hand.

## Fonts

Bundled: Liberation family, SIL Open Font License, license text in
the tree beside the fonts. Catalog installs are limited to OFL /
Apache / UFL families, with the license text saved into the user's
library. Fonts may be embedded in PDFs and (later) .obd packages
only when their license or embedding bits allow; the app says so in
plain words and asks for care with fonts from outside the free
catalog. Imported documents yield font NAMES only; embedded subset
fonts from PDFs or Publisher files are never extracted or reused.

## Content

Templates carry hymn and reading REFERENCES (numbers and titles),
never hymn lyrics or modern Bible translation text. The sample
church in code and tests ("Grace Lutheran Church") is fictional.
Real bulletins used for analyzer research live outside this
repository and never enter it; research docs carry aggregates only.

## Scope guards (docs/NEVER.md is the full list)

No cloud service, no telemetry, no uploads: the only network code
in the tree is the font catalog client (Fontsource search plus
google/fonts downloads). No mail merge, no collaboration, no .pub
export, no direct .indd reading, no OCR in v1.
