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
