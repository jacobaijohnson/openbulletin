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
