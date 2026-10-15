# Tier-2 state review brief: what we have, what to do better, what completes it

You are an external reviewer for OpenBulletin, a church-specific
open-source desktop-publishing app (Rust, egui) meant to replace
Microsoft Publisher for church office administrators. You reviewed
early pieces of this project before (concept, import architecture,
README). This pass is different: review the CURRENT STATE of the
whole project, say what we could do better, and say what is missing
for this to count as a complete, usable v1 for one real church.

Rules for this review:
- Be specific and blunt. "Looks good" is useless. Name the weak
  seam, the wrong order, the thing that will bite first.
- Separate your findings into: (A) decisions to revisit, (B) quality
  risks in what is built, (C) what is missing for a complete v1,
  (D) the font question below, answered explicitly.
- Stay inside the project's stated scope and principles. Do not
  recommend generic DTP features from the never-list.
- The user is a solo developer directing AI builders. Effort is
  real but correctness for non-technical church admins matters
  more.

## The one decision we need ruled on (section D)

Fonts, today: one app-managed Font Library folder holds every font
the engine may use. Fonts arrive four ways: bundled with the app
(Liberation family), added as files by the user, searched and
installed from a free-font catalog inside the app (Fontsource for
search and license facts, downloads from the google/fonts repo,
four core faces per family), and, in the future import flow,
matched by name against metric twins and free lookalikes. The
engine shapes text ONLY with library font bytes, so a bulletin
lays out identically on the office PC, the pastor's laptop, and a
future NAS/Docker deployment. Documents name fonts by family and
pin the exact file's SHA-256 on save/export. Imported PDFs and
Publisher files yield font NAMES, never usable font files (they
embed subsetted, mostly commercial fonts), which is why the corpus
below is full of Gill Sans and Bookman Old Style we cannot ship.

The open question from the project owner, verbatim: "Maybe allow
installing free font packs? But use system otherwise? What is
best practice?"

Our current leaning, which we want pressure-tested: font packs
YES (curated one-tap sets of free families, e.g. a church starter
pack), system fonts only by COPYING them into the library through
a picker, never by live reference. Grounds: the print industry's
reproducibility rule (InDesign Package / Document fonts), web
tools' server-side libraries, and the NAS shape having no usable
system fonts at all. Rule on it: is copy-in right, or is there a
better practice we are missing? Consider also: admins already
"have" fonts like Calibri on their PCs; a picker that copies in
adds a step. Is that step worth it? What do font managers and
self-hosted webfont practice actually say?

## Project state (ground truth, verified by tests on 2026-10-15)

12 Rust crates, 32 tests passing, 0 lint warnings, public repo,
dual license MIT OR Apache-2.0. Built and working: document model
(zones, panels, stories, styles with named fonts, church profile
fields), layout presets (letter bifold/trifold/portrait, legal and
tabloid bifold), imposition (bifold, tri-fold flap math,
saddle-stitch), real text shaping (parley, bundled Liberation),
hand-rolled PDF export (reader + imposed + duplex test sheet),
.obd file format v0 (JSON, atomic save, version check), a guided
5-step app (Start, This Sunday, Announcements, Check, Print) with
printer duplex wizard persisted per printer, settings persistence,
PDF text extraction two ways (lopdf scaffold, and a PDFium
out-of-process helper emitting versioned JSON), a template
analyzer measured on 17 real bulletins from one church (below),
and the Font Library described above.

Not built yet: the analyzer's draft-building layer (families to
standing blocks to a suggested template with the
Accept/Edit/Cancel review), any actual import path producing a
draft bulletin (IDML, Publisher rescue, PDF analysis), PDF font
embedding (exports currently draw with core Helvetica while
shaping with library metrics), .obd v1 zip packaging (fonts and
assets inside the file), seasonal/occasion template packs, the
Docker/NAS deployment shape, accounts and roles (designed, not
built, deliberately deferred).

## Corpus findings (aggregates only; the bulletins are private)

17 real weekly bulletins from one Lutheran church: all 16-page
half-letter saddle booklets (not bifolds), 1-2 picture-only cover
pages each. PDFium extraction is deterministic; lopdf ordering is
not. Standing-text method: 8-word shingles, 80 percent presence
bar, lines rebuilt from positioned spans before comparison
(page-2 standing share rose from 24 to 43 percent). Automatic
clustering (mutual closest siblings over shingle Jaccard) found
three families unaided: a 12-bulletin main family, and two
related ELW groups it deliberately did not merge. Font inventory
by text runs: GillSansMT dominant, BookmanOldStyle, Arial,
FranklinGothic-Heavy, TimesNewRomanPSMT, Rockwell, Calibri,
Tahoma; 217 distinct name strings with subset prefixes. Every
headliner is commercial; Arial/Times/Calibri have metric twins,
Gill Sans and Bookman are lookalike-only.

## Principles (docs/PRINCIPLES.md, condensed)

1. KISS, the owner's expansion: Keep It Short and Simple.
2. Simple on the surface, powerful underneath. Every screen must
   pass an explain-it-like-I'm-five test; the engine underneath
   is the real thing.
3. Users are office administrators, not layout professionals.
4. Guided flow: always exactly one visible next step.
5. Automate the recurring (the weekly bulletin cycle).
6. Never lose work.
7. Imports are rescues, not magic: badges say Matched / Check
   this / Picture only; no OCR in v1; never extract subset fonts.
8. Strict build isolation from the owner's other project.

Never-list (adopted): no mail merge in v1, no web/EPUB export, no
GREP styles, no footnotes/index, no collaboration or cloud, no
bundled copyrighted hymn lyrics or modern Bible translations, no
.pub export, no direct .indd reading.

Deployment direction: desktop first; later one Docker image for
NAS/home-server local web (same egui UI compiled to WebAssembly);
internet reach only if churches ask, then via the operator's own
Cloudflare Tunnel plus Cloudflare Access sign-in. Accounts:
local first, Google optional later; roles Owner / Editor /
Viewer. Logins never in the file format or desktop v1.

## docs/FONTS.md (the full font design under review)

[The file follows verbatim in the payload.]

## README.md (what the public sees)

[The file follows verbatim in the payload.]

## ROADMAP.md

[The file follows verbatim in the payload.]
