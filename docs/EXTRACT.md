# Extraction dump (the importers' shared form)

The extraction importers share one form: a versioned JSON
**extraction dump** (`ob-extract`, `DUMP_SCHEMA_VERSION = 2`).
The Template Analyzer reads dumps only.

A dump carries: source kind and name, page count and page sizes in
points, and text spans. Each span is marked **raw_text** or
**picture_only** (a page with no readable text: a scan or outlined
fonts; the analyzer treats those as tracing guides, never as words).
Spans carry position (x, y, width, height), font size and font name,
bold, alignment, italic, fill color, and inline runs where the
source states them. Those facts arrived with schema v2; readers
still accept v1 dumps, whose spans default the newer fields.

Engine (from the Opus import pass, docs/planning/opus-import.md):
PDFium in an out-of-process helper fills the dump for PDFs; that is
the production PDF path. The lopdf reader that scaffolded the first
dump remains in the crate, used by its examples and tests only.
MuPDF (AGPL) and Poppler (GPL) stay
out on license grounds, consistent with the MIT OR Apache-2.0 project
license.

## The Publisher bridge (shipped 2026-10-15)

`.pub` takes one extra step: `ob_extract::pub_bridge` finds
LibreOffice ($OB_SOFFICE, then the standard install locations,
then PATH) and runs it headless to convert the file to PDF, in
a private profile folder so an open LibreOffice window cannot
block it. The PDF then goes through the same dump/render/draft
path as any rescued PDF, and the review carries a note that the
file came by way of LibreOffice, so small layout differences
are normal. Without LibreOffice installed, the app says so in
plain words and explains the Save-As-PDF route instead.

## Helper confinement: what separation does and does not do (round 51, G-06)

The helpers below run out of process. That buys crash
isolation (a bad file kills the helper, not the app), a
deadline, and an output cap. It does NOT buy filesystem
confinement on any OS: no helper runs sandboxed, and every
helper inherits the app's full user privileges and
environment. The shared runner (`ob-runner::run_with_timeout`)
sets exactly piped stdin/stdout/stderr, a Unix process group
(so a timeout kill stops the whole family), a 120-second
deadline, and a 512 MB combined output cap. It sets no
`env_clear`, no chroot, no namespace, no uid/gid drop, and
no Windows job object. The per-OS picture is the same
picture three times: Windows adds no job object or
AppContainer; macOS adds no sandbox profile; Linux adds
no namespace, seccomp filter, or chroot — the Unix
process group is a kill family, not a boundary. What a
helper may legitimately touch is narrow; what it COULD
touch if its parser were escaped is everything the user
can touch. That distinction is the whole of this section,
and the old wording that invited a stronger reading
("a bad PDF can hurt it, never the app",
`crates/ob-extract/src/helper.rs`) is corrected to say
crash isolation only.

- PDFium helper (`ob-pdfium-helper`, via
  `ob_extract::helper`): takes the PDF path and an images
  directory as argv; reads the file (a 512 MB pre-read
  size gate) and writes page PNGs into the caller-supplied
  directory; 120-second deadline via the shared runner.
  No restriction on what else it could read or write if
  exploited, on Windows, macOS, or Linux alike.
- Verovio helper (`ob-verovio-helper`, via
  `ob_music::render`): stdin JSON request, stdout JSON
  response, same shared-runner deadline and output cap.
  The engraver behind it (a Verovio CLI or the Python
  driver) writes into a scratch folder it creates
  exclusively and removes on drop. Same user privileges
  on every OS.
- LibreOffice `.pub` bridge (`ob_extract::pub_bridge`):
  its own wait loop with a 120-second deadline (not the
  shared runner); runs `soffice --headless --convert-to
  pdf --outdir <dir>` with its profile at
  `<out_dir>/lo-profile` and the PDF into the same work
  folder. No sandbox, on any OS.
- WMF converter (`ob-app`'s `wmf_converter`, via
  `ob_clips::run_converter_with_deadline`): `soffice
  --headless --norestore --convert-to png` into a fresh
  scratch folder, removed afterwards, with a 30-second
  deadline. The input filename is reduced to its file-name
  component before the join, so a crafted name cannot
  steer the write elsewhere. That is input hygiene plus
  cleanup, not confinement, on any OS.
- `open_with_os` (`ob-app`): execs the OS default handler
  (`cmd /c start` on Windows, `open` on macOS, `xdg-open`
  on Linux) on a path. No restriction at all beyond the
  OS's own: the opened program runs as the user, with
  the user's reach.

No new sandbox control rides with this correction, for
reasons the batch record states (round 51, G-06):
separation here never added a privilege — no helper runs
elevated, setuid, or in another account — so an escaped
helper reaches no further than the app itself already
does, and a cross-platform sandbox that still lets each
helper read the user-chosen input and write its output
folder is a project of its own, not a line in this fix.
What the office should take from this section is the
import discipline, not a boundary: import files from
sources the office trusts, the way it already treats
any file it opens.

Why no new control was added, in full (round 51, G-06;
the condensed rationale above is preserved, and this
states the batch record's four reasons and reopen
triggers in the repo so they do not live only in the
audit tree):

1. No evidenced vulnerability: no known exploit, no
   incident, no bypass of a control that exists. The
   finding is a wording/assurance gap, as the round-51
   synthesis states.
2. No privilege is added by the helpers: they run as
   the user, exactly as the app does. A sandbox would
   narrow the helper's reach, not the app's, and the
   helper's legitimate work — read the user-chosen
   input, write the output folder — is exactly what a
   cross-platform sandbox broker would have to permit
   (Windows job objects/AppContainer, macOS sandbox
   profiles, Linux namespaces/seccomp, each with its
   own broker for fonts, temp, profiles, and the Python
   runtime Verovio needs). That is a project with its
   own regression surface (over-brokered = broken
   imports; under-brokered = theater), not a line in
   this fix.
3. The product's import discipline matches its class
   (Scribus and LibreOffice run parsers as the user
   too): import files from sources the office trusts.
4. A narrowed control is still a project, not a line:
   a chroot that bind-mounts only the chosen input and
   output would narrow an escaped helper's reach to
   those paths — the round-53 correction to this reason
   is that the earlier wording ("would not change the
   reach assessment") overstated the case, since
   restricting access to chosen paths does narrow reach
   versus everything the user can touch — but the
   broker must still permit everything the helpers
   legitimately need (fonts, temp, profiles, and the
   Python runtime Verovio needs), which is reason 2's
   project with its own regression surface, and with
   no evidenced vulnerability and no added privilege
   that project is not justified by this finding. A
   loosely brokered control added for show would be
   theater; a tightly brokered one is the project
   above, not a line in this fix.

Reopen condition: if a parser exploit is evidenced, if
helpers ever run elevated or in another account, or if
the product ever imports untrusted-by-default input
(e.g. auto-import from a watched folder), the control
question reopens — the threat model, not the wording,
will have changed.

## The native Publisher reader (shipped 2026-10-15)

Our own reader, an independent implementation from documented
format facts ([MS-CFB] for the container, [MS-ODRAW] for the
drawing layer, and facts from the public reverse-engineering
record; no libmspub code, no LibreOffice needed). It covers the
Publisher 2002-and-later family (the 0x2C files); the 2000-and-
earlier family gets the bridge-or-PDF guidance instead. When
LibreOffice is absent, the app uses this reader: the file's
words land in the boxes Publisher put them in (Contents chunk
directory joins each text box to its story; Escher anchors give
the box, with coordinates relative to the page center). Master
pages, tables, and WordArt text come over marked Check this.
Pictures are counted and their places marked, but their image
bytes do not come over yet; the review note says so. When a
file's layout cannot be read, the words still come over in
reading order and the note says their places could not be read.
Words that run past the bottom of their page keep what fits in
the box; the rest follow in reading order at the end, counted.
Measured on the Apache POI public sample files: every 2002+
sample positions fully, with no placed span outside its page
bounds; the sample newsletter places 59 of 62 shapes and the
sample brochure 28 of 30, with tables, alternates, and picture
counts matching the files.

Spike pass bars, agreed before the real engine work: at least 98%
character accuracy against hand-checked ground truth on real bulletins,
100% detection of unreadable pages, and zero confident wrong sheet
classifications.

## QuarkXPress files: export first, then open here

There is no native QuarkXPress reader: `.qxd` and `.qxp` are
undocumented proprietary binaries, and no public parser covers
the versions most church bulletins were made in. Export from
Quark itself first, in one of two ways. From QuarkXPress 2024
or later, export the print layout as IDML (File, then Export
as, then IDML) and open that IDML here: it goes through the
existing IDML door. This route is new and not yet tested on a
real church file, so check the draft against the original.
From Quark 6 (2003) or later, export the print layout as a
PDF and open that PDF here: it comes in through the PDF door,
with the exact look kept and the words arriving as a rescue
to check. From Quark 5 or earlier, print to PDF, or make a
PDF through PostScript and Acrobat Distiller, then open that
PDF here. For very old files (QuarkXPress 3.x-6.x), Quark's
free Legacy Document Converter moves them to QuarkXPress 9.1
format, so a modern Quark can open them; it is only a
stepping stone, and you still need a modern Quark to export
IDML or PDF from there. A paid converter, QXPMarkz
(markzware.com), is another way to turn a Quark file into
IDML. A Quark file picked directly is refused in plain words,
with these next steps in that order. Honest limits: an IDML
export is a common-denominator copy, so text shading is lost
and some fonts may differ; a PDF keeps the look but not the
layout structure.

## First real corpus (2026-10-15, local only)

Jacob provided 16 real bulletins from one church (Feb 2025 to Oct 2026:
Epiphany, Lent, Easter, Pentecost seasons). The files live OUTSIDE this
repo (real people's names appear in prayer lists and announcements;
test fixtures in the repo stay fictional). Scaffold results, verified
by reading the probe output line by line:

- Every file parses: **16 pages, each 396 x 612 pt (5.5 x 8.5 in)**.
  This church's bulletin is a 16-page half-letter booklet, not a
  4-panel bifold. The saddle-stitch imposition preset is the one that
  matters here.
- Text recovery varies by which tool produced the PDF: 73 to 668 spans
  and 11.5k to 17.4k characters per bulletin. Same church, same basic
  layout, very different internals. The analyzer must not assume one
  producer's habits.
- Every file has 1 to 2 pages with no extractable text (covers with
  artwork). They are marked picture-only, as designed.
- A few files contain a single 395 x 611 page. Real files are messy;
  consensus across a corpus is what smooths that out.

## Found by this work: our own PDFs were slightly invalid

The scaffold's first strict reader (lopdf) rejected OpenBulletin's own
output: the file ended in a doubled EOF marker (`%%%%EOF`). Lenient
viewers had silently accepted it. Fixed, with a regression test
(`eof_marker_is_exactly_two_percents`). Lesson recorded: verify with a
strict parser, not just by opening the file.

## Corpus consensus: first pass results (2026-10-15)

Method: 8-word shingles over normalized text (whole-line matching was
tried first and found almost nothing; producers chunk text too
differently for line equality to survive). A passage is "standing"
when it appears in at least 80% of the bulletins being compared.
Tool: a standalone prototype example, since retired. The method it
proved — 8-word shingles, standing passages, clustering first — is
the production Template Analyzer's, implemented in-crate at
`crates/ob-extract/src/analyzer.rs`. Full example passages
stay in the private fixtures folder; aggregates only here.

- **Whole pile mixed (17 bulletins):** 116 standing passages, but only
  page 2 holds its position week to week (53% standing). Pages 4-16
  show zero page-stable standing text. The standing words exist; they
  MOVE, because a 16-page booklet reflows as each week's readings and
  music change length.
- **Clustered by series, the picture snaps into focus.** The 6 LOTR
  bulletins: 737 standing passages, 28-34% of each bulletin is
  standing text, page 2 is 76% standing and page 4 is 100% standing.
  The 5 ELW bulletins score lower (147 passages, 8-13%), which may be
  word-order scrambling in this crude extractor rather than reality;
  the position-aware PDFium helper will settle that question.
- **Design consequence, evidence-backed:** the analyzer must cluster
  first (series, season, producer) and run consensus inside each
  cluster, tracking standing text by reading order, not by page
  number. That was the design's assumption; now it is measured.

## Automatic clustering + a hard limit of the scaffold (2026-10-15)

The consensus prototype now clusters on its own (Jaccard similarity over
shingle sets, no filename peeking). On the 17-bulletin pile it found:
one big 12-bulletin family (417 standing passages, 21% median standing
share), one ELW pair (595 passages, 35%), and three ELW singletons.
So the machine finds the main family unaided, and the ELW files are
different enough from everything, including most of each other, to
strand. Singleton clusters report a meaningless 100% standing share;
a family of one has no consensus, and the product must treat it that
way (ask for more bulletins from that family).

**Measurement instability, verified:** the same tool on the same 17
files reports 116 standing passages on one run and 91 on the next.
lopdf hands text back in an order that varies between runs, and
order-sensitive comparisons wobble by about a quarter. Every number
from this scaffold is therefore approximate. This is the measured
reason the real extractor is the position-aware PDFium helper:
positions make order a fact instead of a guess. Scaffold work stops
here; PDFium is the next extraction step.

## PDFium helper: built and measured (2026-10-15)

`crates/ob-pdfium-helper` is a real out-of-process helper on PDFium
157 (pdfium-render 0.8): same JSON dump schema, now with x/y/w/h,
font size, and bold on every span. The first ELW file it read went
from 77 crude text chunks to 866 positioned spans.

- **Deterministic.** Two runs over all 17 dumps agree exactly (198
  standing passages both times). The scaffold's wobble is gone.
- **Cluster structure reproduced** from positioned data: the same
  12-bulletin family, the same ELW pair (753 passages, 33% standing),
  the same three ELW singletons. Two independent extractors agreeing
  on the families is good evidence the families are real.
- **ELW mystery, refined verdict:** it was not word-order scrambling.
  The three stranded ELW files are fragmented far more finely
  (average span ~15 characters vs ~27 for the main family), the mark
  of a different producer or export setting. Where one producer
  splits text differently every week, word runs break at the seams.
- **Next analyzer step, now precisely specified:** rebuild true
  lines from positioned spans (group by vertical position, sort by
  horizontal, join on gaps) BEFORE consensus. Positions turn that
  from guesswork into arithmetic.

## Line rebuild: measured gains, and the ELW family at 0.11 (2026-10-15)

The consensus tool now rebuilds true lines from positioned spans
before comparing (rows by vertical overlap, left-to-right order, a
space only where the gap looks like a word space, so mid-word splits
heal). Gains on the same 17 dumps: standing passages 198 -> 212, and
page 2's median standing share 24% -> 43%. Reading order as
arithmetic works.

The three stranded ELW files still miss the 0.12 merge bar, but the
new closest-sibling diagnostic shows by how little, and it changes
the conclusion: every ELW file's closest sibling is another ELW file.
The trio sits at 0.11 similarity, just under the bar, and two of
them are each other's closest match. The family is real; a single
global threshold is simply the wrong shape for it, because the big
main family chains together at 0.14+ and would swallow strangers if
the bar dropped. Specified next rule: merge mutual closest siblings
first (pairs that name each other), then attach their near neighbors.
That is a clustering refinement, not an extraction problem; the
extraction layer is done proving itself.

## Clustering v2: mutual siblings (2026-10-15)

Implemented as specified. On the 17 real dumps the machine now
finds three families with no filename help: the 12-bulletin main
family; an ELW family of three (the previously stranded 09-27
attached to its closest sibling's family, as the rule intends); and
an ELW pair (10-11, 10-18, mutual closest siblings, 467 standing
passages, 24% standing share). The two ELW groups are NOT merged;
instead the tool prints a "looks related" hint (best pair 0.10),
which is the honest shape of the evidence: same series, two
production flavors, and the review screen is where a human merges
them if they wish. Hints also fire between the main family and the
ELW trio (0.11, shared standing material), advisory only.

## Trace rescue (built 2026-10-15)

The reviewers' import order starts here: before any analysis, a
church can rescue last week's PDF as pictures. `ob-pdfium-helper
render <file.pdf> <dir>` draws every page to a PNG at 200 dpi.
`openbulletin-cli trace <file.pdf> --out <dir>` runs the helper,
reads the page sizes back from the dump, and writes a draft
bulletin (<name>-traced.obd) plus an assets folder: one panel per
page, each carrying its page picture as `background_image` (a
tracing guide that never prints; the model field is optional and
defaults to none). Sixteen same-size half-letter pages become a
saddle-booklet draft automatically; anything else becomes plain
pages. Verified on a real 16-page corpus bulletin: 16 panels,
16 guides, and the guide render is a faithful page image. The
app's page canvas shows the guides under the text, and the draft
opens as a normal bulletin.
