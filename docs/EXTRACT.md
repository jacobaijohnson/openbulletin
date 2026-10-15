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
