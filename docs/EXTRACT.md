# Extraction dump (import scaffolding)

Every importer (PDF first, then .pub rescue and IDML) produces the same
thing: a versioned JSON **extraction dump** (`ob-extract`,
`DUMP_SCHEMA_VERSION = 1`). The Template Analyzer reads dumps only.

A dump carries: source kind and name, page count and page sizes in
points, and text spans. Each span is marked **raw_text** or
**picture_only** (a page with no readable text: a scan or outlined
fonts; the analyzer treats those as tracing guides, never as words).
Spans will carry position, size, and font facts once the real engine
lands; the fields exist now so the contract does not change later.

Engine plan (from the Opus import pass, docs/planning/opus-import.md):
PDFium in an out-of-process helper fills the dump; lopdf stays as the
structural sidecar (this scaffold). MuPDF (AGPL) and Poppler (GPL) stay
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

Spike pass bars, agreed before the real engine work: at least 98%
character accuracy against hand-checked ground truth on real bulletins,
100% detection of unreadable pages, and zero confident wrong sheet
classifications.

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
Tool: `crates/ob-extract/examples/consensus.rs`. Full example passages
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

The consensus tool now clusters on its own (Jaccard similarity over
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
app's page canvas (which will show the guides under the text) is
the next layer; the draft already opens as a normal bulletin.
