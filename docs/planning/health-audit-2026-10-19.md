# Whole-codebase health audit — synthesis (2026-10-19)

Ordered by Jacob at the round-29 pausing place, before
round 30. Six blind, read-only lanes (completeness,
stability, efficiency, effectiveness, best practices,
security), each over the whole codebase — all 15 crates —
none reading another's report. Lane reports:
`~/workspace/openbulletin-audit-wave3/health-{completeness,
stability,efficiency,effectiveness,practices,security}.md`.

**Base tree: `da09b71`** — the round-29 coordinator close
(`6899603`) plus the standalone icon commit (`da09b71`,
"Icon: Jacob's cleaned-edge art as master"). The Windows
bundle completed **success** on both commits (runs
`39275277370` on the close, `39282449985` on the icon
commit; the bundle of record ships Jacob's cleaned-edge
icon).

**Headline: 1 BLOCKER / 13 MAJOR / 38 MINOR deduped**
(52 unique findings), against a long held list: the
security lane alone records 21 held items, and the
no-telemetry claim is verified in code (H-1/H-2 of its
report: `ureq` exists in exactly one source file, the
fonts catalog; `std::net` / `TcpStream` / `reqwest` /
`hyper` = 0 hits; every catalog call site is a
user-pressed button or an explicit CLI subcommand).

**Jacob's stub suspicion, answered:** largely not borne
out. Two lanes independently found **zero** `todo!` /
`unimplemented!` in the tree, zero production `panic!`,
no empty button handlers in 139 click sites, all nine
Settings fields traced write → persist → read →
behavior, all twelve windows reachable. What the audit
did find is more specific: one real fence hole at Save
(HF-1), a cluster of docs that describe behaviors never
built (HF-6 and the docs-truth bundle), and accretion
debt concentrated in three very large files (HF-9/10/11).
The panic surface of the whole workspace is 4 guarded
`unwrap()`s and 52 invariant `expect()`s (stability
lane, production-split counts).

## Pre-audit sequence (on the record)

- The round-30 parser lanes were dispatched under the
  prior sequence and **stopped** when Jacob's amendment
  landed; they were read-only, produced nothing of
  record, and their scratch was deleted in the clean.
  Round 30 has not run.
- Round-boundary cache clean, per Jacob's standing
  instruction: disk 22G free (79% used) before; repo
  target 14G removed by `cargo clean` (16.1 GiB);
  audit tree 14G → 36M (all lane reports and the
  round-28/29 output logs kept; probe29/29r/29v sources
  kept). Registry NOT pruned. The unchanged fuzz bins
  survive at their recorded md5s in `probe29v/src/bin/`
  (verdict `a9cac6c399d304bac853f2a64f683fca`,
  composition `4f85c605a841329dabed1d9a2aa84f88`),
  re-verified after the clean. 29G free after the
  clean; one cold full-suite run then proved the fresh
  start: **862 passed / 0 failed, CARGO_EXIT=0, clippy
  exit 0, booklet 31,717 / 34,977 byte-exact**; disk
  26G free after the rebuild (target 8.6G).
- Lane incidents: the first security lane was killed by
  a runtime restart before writing (re-dispatched; the
  second completed). The best-practices lane's session
  was interrupted after its report was fully written;
  the report on disk is structurally complete (counts,
  measured tables, findings, coverage) and is used as
  written. The stability lane corrected one probe
  figure in its final message (30.3 s, not 51.5 s);
  the report carries the corrected number.

## Deduped findings

Grades are the coordinator's after consolidation;
lane ids are given so each finding traces to its
report(s). Where lanes graded the same root
differently, the adjudication is stated.

### BLOCKER

**HF-1 — The Save-time tracing-picture absorb is an
unfenced second copy of a fenced function; a hostile
bulletin turns Save into a read of any local file and
embeds the bytes in the saved .obd.** (Security S-1.)
`absorb_tracing_pictures` (ob-app/src/main.rs:13765,
called on every Save) reads whatever path a received
document's `panel.background_image` names — absolute
paths discard the base, `../` escapes it — with no
canonicalize, no inside-tree check, no size cap, no
picture-magic check, and embeds the bytes into the
saved bulletin, which then travels to the next church.
The loader's twin in ob-format (lib.rs:2041-2110) has
all four fences and states the attack in its comment.
Probe-demonstrated precondition: v0 files carrying
`../secret.png` and `/etc/hostname` references survive
`ob_format::load` unabsorbed and reach the app live.
Companion: `load_bg_texture` (main.rs:11646) reads the
same reference unfenced and bypasses ob-image's limits.

### MAJOR

**HF-2 — No input-size cap at the open door.**
(Security S-2.) Every in-process importer (IDML, SLA,
MusicXML, HTML, Markdown, native .pub, the analyzer per
file) and the PDFium helper begin from an unbounded
whole-file `fs::read`; internal caps engage only after
the file is fully resident. A multi-GB hostile file
exhausts the machine before any parser cap speaks.

**HF-3 — HTML import data-URI pictures are uncapped.**
(Security S-3.) No per-picture, count, or aggregate cap
in html_import.rs. Demonstrated: an 80,256,633-byte
page with 20 × ~3 MB data-URI PNGs imported cleanly,
landing 60,191,780 bytes in the asset store.

**HF-4 — The autosave journal does whole-document zip
work synchronously on the egui frame thread, and Save
deflates every already-compressed asset byte.**
(Stability S-MAJ-1 + Efficiency MAJOR-1 — one finding,
two lanes.) `journal_tick` runs inside `App::update`;
`write_journal_to` is a full `ob_format::save`, which
clones the document and Deflates every entry
(ob-format/src/lib.rs:1301-1319). Measured (release):
save = ~62 ms per MiB of incompressible asset (112 ms
@ 2 MiB, 495 ms @ 8 MiB; the same 2 MiB compressible
saves in 7 ms; load of 8 MiB is 22 ms). A photo-heavy
bulletin freezes the UI ~0.5 s per edit burst,
recurring every ≥2 s while dirty. Stability's debug
probe puts the same shape at 30.3 s for a 30 MB-asset
load + re-save through the CLI.

**HF-5 — No durable write is ever fsynced; "Saved"
does not survive power loss.** (Stability S-MAJ-2.)
`sync_all|sync_data|fsync` = 0 hits workspace-wide.
Every durable write is temp + rename without syncing
file or directory: `ob_format::save`, `save_settings`,
`persist_hymn_memory`, the journal. Rename survives an
app crash, not an OS crash or power cut — under a
"Saved as …" message, in code whose own comment names
a bulletin lost on Saturday night as trust-ending.

**HF-6 — docs/TEMPLATE-ANALYZER.md promises four
behaviors the code never built.** (Completeness F1.)
The doc is top-level and present-tense. Delivered: the
minimums (3+ counting, PDFs only in groups of 3+),
standing/weekly consensus, Accept/Edit/Cancel. Never
built: field-like classification and field proposals;
church profile harvested from the cover; the seasonal
set-aside said out loud ("2 of these look like special
services; we set them aside" — the implementation
grows one family and silently leaves the rest out);
low-confidence items listed in words and highlighted
in a review preview (no confidence tier, no preview
pane). ROADMAP's narrower shipped claim is true; the
top-level doc's is not. The seasonal overclaim echoes
at docs/planning/users-and-journeys.md:145.

**HF-7 — `STARTER_MANIFEST` is dead, and three
comments claim the clipart provenance record installs
with the starter set when it does not.**
(Effectiveness E-2 + Completeness F3 — one finding;
graded MAJOR on effectiveness' axis: the false claim,
not the dead data. Completeness graded the data
MINOR; the claim governs, as in HF-8.) The manifest
(per-piece source page, author, license, retrieval
date) is `#[allow(dead_code)]` and referenced nowhere;
`ensure_starter_clips` installs from `STARTER_FILES`
only, with hard-coded notes (license "CC0",
`added: ""`), flattening a mixed PD/CC0 set's record —
while starter_clips.rs:5-8, :38-43 and main.rs:11855-57
all say the manifest is read / rides along, and
docs/NEVER.md's amendment promises each piece
"recorded in a manifest with its author and license".

**HF-8 — ob-text's root API is the dead v0 placeholder
estimator, and the crate header still describes it as
the plan.** (Effectiveness E-1 + Completeness F4 +
Efficiency MINOR-5 — one finding.) `measure` / `wrap`
/ `leader_line` / `expand_order_line` (lib.rs:13-89)
are average-glyph-width estimates under a header
claiming the real engine would land "behind this same
API"; it landed beside it (`Shaper::*`,
shaping.rs:324-429), and every production caller uses
the Shaper. The root functions have zero production
callers; their `wrap` hard-split is also O(n²) in word
length (efficiency, measured). The crate's Cargo.toml
description still sells the estimate; README's table
describes the crate correctly. A standing trap: one
future `use ob_text::wrap` silently downgrades a real
path to estimates that disagree with print.

**HF-9 — ob-app/src/main.rs is 24,393 lines holding at
least six coherent modules; the "thin shell" claim in
its header, the README, and CONTRIBUTING.md is false.**
(Practices M-1.) Settings/keymap, the Check step's
findings engine, preview/editor layout machinery,
contribution planning/application, every window, and
the eframe dispatch share one file; `update` is 1,602
lines, `panel_canvas` 1,532. The Check findings and the
contribution-apply half-engines live only in the GUI,
unreachable from the CLI. Every round's fix touches
this file; merge and review risk concentrates here.

**HF-10 — contribution.rs is a 13,726-line production
grammar engine in one file, public API at the end.**
(Practices M-2.) Four separable engines share the
file (date/header sentences; token and clause
segmentation; the verb/governor grammar;
group/composition analysis) plus an 18,871-line single
test module. Heavily tested and clippy-clean; the cost
is paid by every future reader and reviewer.

**HF-11 — ob-model/src/lib.rs is a 9,127-line single
file holding four responsibilities at the bottom of
the graph.** (Practices M-3.) Geometry/document types
(the stated job), the rights/license engine, the
serving-line rename migration engine, and field
resolution/story surgery. Eleven of fourteen sibling
crates depend on it directly; every edit has
workspace-wide blast radius. The split pattern already
exists in-crate (hymns.rs, markdown.rs).

**HF-12 — `Result<_, String>` is the de-facto shared
error architecture.** (Practices M-4.) 110 occurrences
across 8 crates, ~90 `map_err(...to_string())`
flattening sites; four typed errors exist in the whole
workspace. The plain-words String is a deliberate
product choice at the user boundary; inside the
engine, callers cannot match on failure kind and tests
assert on message substrings. Recorded as a decision
item for Jacob (below), not silently batched: a typed
core error with plain-words rendering at the boundary
is the lane's recommended shape, and it touches eight
crates' public surfaces.

**HF-13 — ob-music depends on all of ob-extract to
borrow a process runner and three draft types.**
(Practices M-5.) 17 `ob_extract::` hits in ob-music:
the runner (462 lines, self-contained) plus
`Badge` / `DraftOutcome` / `ImportItem`. For that,
ob-music links the full import stack (lopdf,
html5ever, the CFB readers, the contribution grammar),
and the workspace's only `unsafe` (the runner's
killpg) is owned by a domain crate.

**HF-14 — ob-extract exposes every module; the
contract and the plumbing are indistinguishable.**
(Practices M-6; measured workspace-wide by
Effectiveness E-15: 87 of 519 pub items have no
cross-file caller.) `pub mod` for all 17 files,
`pub(crate)` 16 times in the entire workspace. In a
lockstep sibling workspace the cost is not today's
correctness; it is that no maintainer can tell the
API from the internals, and any pub item can silently
acquire a caller.

### MINOR (38, grouped)

*Write-path robustness (stability):* **S-MIN-1**
export copies written non-atomically over the previous
good copy. **S-MIN-2** a corrupt hymn-memory.json is
silently reset (siblings set aside + say so);
`persist_hymn_memory` swallows a failed rename.
**S-MIN-3** journal peer detection fails open on
non-Unix (two Windows instances can both own the
journal) and can false-positive on a recycled PID.
**S-MIN-4** a catalog worker panic is silent where an
import worker panic speaks. **S-MIN-5** journal write
failures never surfaced; the journal meta file is not
written atomically, so an intact journal bulletin can
be demoted by its torn note file.

*Labels and doc truth (completeness/effectiveness/
practices):* **F2** "Open a copy to read through" only
saves; it never opens (its sibling print copy does).
**Docs-truth bundle** — EXTRACT.md still states
`DUMP_SCHEMA_VERSION = 1` (code: 2) and future-tense
span facts; ob-extract's lib.rs header and Cargo.toml
still describe the lopdf scaffold ("later .pub, IDML"
— both built); FONTS.md's "Live preview and
comparison" section describes a swipe side-by-side
and font-candidate picking in the present tense that
its own Status section lists as remaining (no such UI
exists), and speaks of the Docker shape as existing
(DEPLOYMENT.md/ROADMAP mark it planned); LEGAL.md
keeps a superseded "subsetter is therefore NOT a
dependency today" paragraph in force above the note
recording that it shipped; ob-pdfium-helper's manifest
is the only one with no description.

*Dead and duplicated surface (effectiveness):*
**E-3** dead wrapper `export_large_print_pdf` (the
`_report` twin does the work). **E-4** test-only pub
entry points presented as the API (`ob_image::decode`,
`Cfb::stream_names`, `large_print_document`).
**E-5** `inch` and `inches_to_mpt`: identical bodies,
one file, two names. **E-6** the weekday-name table
restated with *different* cleaning rules (ob-model
trims `.`; contribution strips Cf + five trims) —
unlike the nothing-token restatement, unrecorded.
**E-7** long-date formatting implemented three times
for two date types. **E-8** contribution.rs sibling
pairs (second-dates sentences, WeekdayMismatch
sentences) plus the verbatim inner `clean()` defined
twice. **E-9** the capped zip-entry read reimplemented
five times with divergent overflow judgments
(refuse / note / skip — the caps themselves hold;
security's held list agrees). **E-10** ob-fonts
hand-rolls SHA-256 inside `inspect` and carries a
`sha2` dependency for it, instead of calling
`ob_model::sha256_hex`. **E-12** the `consensus.rs`
example is the superseded prototype of the production
analyzer, and EXTRACT.md still presents it as the
tool. **E-13** Letter-portrait geometry restated as
literals in three places; sheet-name table restated in
ob-app. **E-14** stale `#[allow(dead_code)]` on
`Chunk::id` (its predicted reader exists and reads
it). **E-15** public surface wider than use
(87/519 pub items with no cross-file caller; rides
with HF-14).

*Measured performance tail (efficiency):*
**MINOR-1** font library scan re-reads/re-parses/
re-hashes every font per call (~0.54 ms/font, ≥7
call sites; a 100-font library ≈ 55 ms per scan,
several scans per click). **MINOR-2** the contribution
date-line readers re-walk the header per pass (a
40-group header: 0.9–1.6 ms and 16,112 allocations on
a ~250-byte body; the body path is linear and fast).
**MINOR-3** PDF export re-decodes/re-encodes every
photo on every export (+167 ms for one 2.5 MB photo;
reader + imposed + large print each repay it).
**MINOR-4** threaded layout rebuilt per document
change (2.3 ms for the 16-page booklet, per keystroke)
and `resolve_fields` scans ~25 tokens at every byte
position (~80–146 ns/char, token table rebuilt per
paragraph).

*Posture (practices):* **m-1** no
`workspace.package.version`, no `workspace.lints`
(the drift has not happened; nothing prevents it).
**m-2** the lint set is clippy defaults only; the
MSRV claim (rust-version 1.80) is gated by no CI job;
an `unsafe_code` deny would make the one-unsafe-block
fact a checked invariant. **m-3** the one unsafe
block's SAFETY comment asserts its precondition
without naming the spawn site 40 lines away.
**m-4** the load-bearing external surfaces (PDFium,
parley, subsetter, jxl-oxide) are all pre-1.0 pins
with no upgrade-risk record beside LEGAL.md's license
discipline. **m-5** resvg resolves with default
features including `system-fonts` — an undocumented
exception in a project whose law is the engine never
uses system fonts. **m-6** 21 `#[allow]`s; the 11
`too_many_arguments` suppressions mark
parameter-object candidates instead of the shape being
fixed once. **m-8** the bundled Liberation TTFs live
in ob-text's assets and are reached from ob-fonts /
ob-pdf / CLI tests by relative filesystem path,
off-graph.

*Security tail:* **S-4** the catalog license-note
filename is built from a remote-supplied id without
`sanitize_file_name` (latent; one call from closed).
**S-5** catalog `get_json` / `get_string` uncapped;
the DejaVu zip route has no entry-count cap and
uncapped reads. **S-6** zip entry-count caps missing
in ob-clips and .mxl (30,001-entry .mxl measured at
92 ms; inconsistency, not impact). **S-7** MusicXML
has no depth budget (3,000,000-deep nesting parses Ok;
siblings cap at 256). **S-8** clipart folder intake
follows symlinks and reads whole files before its
size check; no batch total on that door. **S-9**
`actions/checkout@v4` unpinned in build-desktop.yml
on the self-hosted runner (the other workflows pin
full SHAs; the bundle workflow states pinning as the
project's rule). **S-10** the PDFium helper ships a
`$HOME/workspace/.toolchain` fallback for its native
library — a production binary should fail closed.
**S-11** lopdf `extract_pdf` is an uncapped API whose
only callers at HEAD are tests and examples (rides
with E-12).

### Held and verified (the other half of the record)

- Security's 21 held items, including: the .obd loader
  as exemplar (entry count from the central directory
  before open; caps counted in actual bytes; SHA-256
  verified against entry names); IDML/MusicXML/CFB
  caps and chain guards; XXE posture clean at all five
  quick-xml sites (`unescape_with` = 0 hits); helper
  containment (120 s deadline, 512 MB output ceiling,
  process-group kill, 200-char stderr quote); no zip
  reader extracts entries to disk under entry names;
  zero credential-shaped strings repo-wide.
- Stability's nine observation groups: every
  production `unwrap`/`expect`/`unreachable` traced to
  its invariant; the changed-on-disk guard working as
  designed; no mutexes in production code at all.
- Effectiveness' seven observations: the nothing-token
  restatement IS recorded in the code on both sides
  and the tables are character-identical; rights
  merging is canonical (one `merge_rights_records`);
  settings keys have no orphans; FONTS.md's matching
  order, FORMAT.md's version, LEGAL.md's 603 packages,
  and the 862 test count all verify current.
- Completeness' claims table: 29 doc claims checked —
  25 delivered, 2 partial (both in HF-6/HF-7), 1
  missing (the FONTS.md comparison review), 2 honestly
  not claimed as built.
- Practices' positives: `cargo fmt --check` clean;
  clippy re-run clean firsthand; one unsafe block in
  the whole workspace; panic discipline measured.

## Fix batches (dispositioned; one writer each, in order)

The parser convergence loop (round 30) resumes only
after these land, under the next coordinator. Batches
that touch contribution.rs readings (H-G, H-J) carry a
standing condition: no reading may move; the round-30
verify (unchanged fuzz bins re-run as claims) is the
net that proves it.

- **H-A — Fence the Save-time absorb (HF-1).** Route
  `absorb_tracing_pictures` through ob-format's fenced
  discipline (canonicalize + inside-tree + 64 MB +
  picture magic) or delete the app-side gather in
  favor of the loader's; route `load_bg_texture`
  through `ob_image::decode`. Pins: the security
  lane's two probe files (../ and absolute references)
  absorb nothing at Save; a legitimate sidecar still
  absorbs, as the loader's pins already prove.
- **H-B — Cap the doors (HF-2, HF-3, S-5, S-6, S-7).**
  One judgment, stated once: no unbounded read. A
  metadata length gate at each open door (per kind;
  the CFB 512 MB precedent), counted in actual bytes;
  HTML data-URI per-picture / count / aggregate caps;
  catalog `get_json`/`get_string` caps and DejaVu zip
  caps; entry-count caps in ob-clips and .mxl;
  MusicXML depth budget at the sibling constant (256).
- **H-C — Writes that report must be durable, atomic,
  and spoken (HF-5, S-MIN-1..5).** fsync the temp file
  (and the directory after rename) in
  `ob_format::save`, `save_settings`,
  `persist_hymn_memory`; export copies via temp +
  rename; a corrupt hymn-memory.json is set aside with
  a one-time note (the settings/journal pattern);
  journal meta written atomically with its bulletin;
  journal write failures surface once per session;
  a catalog worker panic speaks like an import worker
  panic; journal peer detection hardened (an instance
  token in the lock, so two Windows instances cannot
  both retire the journal).
- **H-D — The journal leaves the frame thread
  (HF-4).** Move the journal's save to a worker with
  `try_recv` polling (the import/catalog pattern);
  stop deflating already-compressed asset entries
  (store picture assets, deflate text). Target, from
  efficiency's table: the 8 MiB incompressible save
  moves from ~495 ms toward load-class times, and no
  frame ever waits on a zip.
- **H-E — Truth in docs and labels (HF-6, the
  docs-truth bundle, F2).** TEMPLATE-ANALYZER.md
  corrected to the shipped behavior, with the four
  unbuilt behaviors marked as not built (building
  them is feature work — Jacob's call, recorded as
  the open question under HF-6; the same correction
  at users-and-journeys.md:145); FONTS.md present-tense
  fixes; EXTRACT.md's schema version and scaffold
  text; ob-extract's header and manifest description;
  ob-pdfium-helper's missing description; LEGAL.md's
  superseded subsetter paragraph; "Save a copy to
  read through".
- **H-F — Retire the placeholder and the dead
  surface (HF-7, HF-8, E-3, E-4, E-12, E-14, S-11).**
  Delete ob-text's root placeholder API and its
  tests; the crate header and Cargo.toml describe the
  shaping engine. Wire `STARTER_MANIFEST` into the
  starter-clips install so per-piece provenance
  reaches the library record (the NEVER.md promise)
  and correct the three comments to the wired truth.
  Delete `export_large_print_pdf`; narrow or mark the
  test-only entry points; remove the stale
  `Chunk::id` allow; retire the `consensus.rs`
  example and cap or remove the uncapped
  `extract_pdf` API it leaves behind (stop-and-report
  if a production caller appears).
- **H-G — One canonical helper per judgment
  (E-5..E-10, E-13).** Unify `inch`/`inches_to_mpt`;
  the weekday tables: unify the cleaning or record
  the twin in code as the nothing-token restatement
  is recorded (batch judges, on the record); single
  month/weekday name tables behind both date types;
  contribution.rs' sibling sentence pairs merged
  behind a lead-string parameter and the doubled
  `clean()` factored; the capped zip-entry read
  factored into one helper with per-door cap and a
  named overflow judgment; ob-fonts calls
  `ob_model::sha256_hex` and drops its `sha2`
  dependency; Letter-portrait geometry named once.
- **H-H — Seams for the three big files (HF-9,
  HF-10, HF-11).** Behavior-preserving moves only, one
  file per commit, suite green between: main.rs split
  along its own section seams (settings/keymap,
  check-findings, preview-layout, contribution-apply,
  windows); contribution.rs into its four engines
  with the public API at the crate surface; ob-model's
  rights engine and serving-migration engine moved
  beside hymns.rs/markdown.rs. After H-A..H-G, so
  behavior fixes land before the moves that would
  churn their diffs.
- **H-I — Boundaries and posture (HF-13, HF-14,
  E-15, m-1..m-6, m-8, S-4, S-8, S-9, S-10).** The
  runner extracted to a small home of its own so
  ob-music drops the ob-extract edge; ob-extract's
  module visibility narrowed to the contract
  (E-15's list as the checklist);
  `workspace.package.version` and `workspace.lints`
  adopted, including an `unsafe_code` deny with the
  single allow at runner.rs and its SAFETY comment
  linked to the spawn site; the resvg feature
  exception documented in ob-image's manifest; the
  `too_many_arguments` cluster answered with
  parameter objects or recorded allows; a
  pre-1.0-pin upgrade-risk note in LEGAL.md beside
  the license record; tests reach the bundled fonts
  through ob-text's documented hand-off, not relative
  paths; catalog license-note filename sanitized
  (S-4); clipart folder intake refuses symlinks,
  checks size before reading, and gains a batch total
  (S-8); `actions/checkout` pinned by SHA in
  build-desktop.yml (S-9); the PDFium helper's
  dev-path fallback removed — fail closed (S-10).
- **H-J — The measured performance tail (Efficiency
  MINOR-1..4).** Font-library scan memo keyed on the
  existing metadata fingerprint; a per-document image
  cache across exports; `resolve_fields`' token table
  built once per call set; the contribution header
  passes sharing one tokenization where the
  architecture allows. Measurement-led: each item
  lands with its before/after number, or is recorded
  as not worth its risk.

## Decision item for Jacob (not batched)

**HF-12 (error architecture).** The practices lane
recommends a typed core error with plain-words
rendering at the user boundary, introduced crate by
crate at the import/library layer. It touches eight
crates' public shapes and is a direction, not a fix;
it should be Jacob's call, taken with the H-batch
results in hand. The H batches above neither need it
nor preclude it.

## What this audit did not do

No lane changed code. The parser convergence loop's
own jurisdiction (contribution readings) was not
re-audited here — round 30 does that, after the
H batches, with the fuzz bins as claims. PDF import
timing through the helper remains unmeasured (no
helper binary is staged on this box; efficiency
appendix). The Tier-2 final gate remains unstarted
and is not the coordinator's to start.

## Health-fix coordinator close (2026-10-19)

All ten fix batches shipped in the synthesis order and
were independently re-verified by the coordinator on
their exact committed trees — full unpiped suite plus
clippy `-D warnings`, verdicts read from cargo's own
exit codes in on-disk logs, never the builder's word.
Final tree before this close: `1d0d226`, **896 passed /
0 failed, clippy exit 0**. The booklet baseline
reproduced byte-exact at every batch verification
(imposed 31,717 / reader 34,977). Test count walked
862 → 896 across the phase; every delta is a named pin
or an explained relocation (H-H moved tests without
changing the count).

- **H-A `93a3e32` — the BLOCKER (HF-1).** The
  Save-time tracing-picture absorb is fenced in
  ob-format (canonicalize, inside-tree, 64 MB,
  picture-magic), the app's unfenced gather deleted,
  background textures decode through the guarded
  helper. Coordinator: 864/0, clippy 0, both fence
  pins green.
- **H-B `814575e` — caps (HF-2, HF-3).** One 512 MB
  door gate at every open door, HTML data-URI caps
  (64 MB each / 2,000 / 512 MB total, dropped and
  counted), catalog caps, zip entry-count caps,
  MusicXML depth cap. Coordinator: 878/0, clippy 0,
  14 pins green.
- **H-C `a21fc5d` — durable writes (HF-5).**
  sync-and-rename under document saves and the app's
  kept copies; corrupt hymn memory set aside with a
  one-time note; journal peer detection hardened
  with a per-instance token and recycled-PID guard.
  Coordinator: 884/0, clippy 0.
- **H-D `c5f30e1` — journal off the frame thread,
  stored pictures (HF-4).** Journal saves run on a
  dedicated worker; picture assets are Stored, not
  re-deflated, in the .obd (no format-version
  change). Coordinator: 888/0, clippy 0. The batch's
  own before/after save numbers never arrived, so
  the coordinator measured the pair at close with one
  harness on both trees (8 MiB incompressible picture
  document, five saves, median): pre-H-D tree
  `a21fc5d` 562.2 ms → final tree 13.0 ms. The audit
  lane's independent pre-fix measure was 494.730 ms,
  the same magnitude.
- **H-E `1ba2bbc` — docs truth (HF-6).** The four
  never-built TEMPLATE-ANALYZER behaviors marked not
  built in place; FONTS.md, EXTRACT.md, LEGAL.md,
  the crate manifests, and the reading-copy button
  label made truthful. Coordinator: 888/0, clippy 0,
  plus firsthand greps of each correction.
- **H-F `9d9054b` — dead surface (HF-7, HF-8).**
  The v0 estimator API deleted; the starter manifest
  parsed at install so per-piece provenance (public
  domain no longer flattened to CC0) reaches the
  collection record; dead wrapper and prototype
  deleted; test-only surfaces narrowed. Coordinator:
  889/0, clippy 0.
- **H-G `c3a1b5c` — one canonical helper (E-5..E-10,
  E-13).** One inches conversion, one month/weekday
  name source, one capped zip read with each door's
  overflow judgment named, one sha256, letter
  geometry and sheet names single-sourced. The
  weekday tables stay recorded twins: unifying
  either way would move a reading, and the reason is
  written on both sides. Coordinator: 890/0,
  clippy 0, and the fuzz bins re-run on the committed
  tree — both stdouts byte-identical to the
  pre-batch baselines, bin md5s unchanged.
- **H-H `6f370ab` + `2ccb5a6` + `f7dff0b` — seams
  (HF-9, HF-10, HF-11).** ob-model's rights and
  serving engines moved beside hymns/markdown;
  contribution.rs became four engine modules with
  the public API at the top; main.rs split along its
  section seams (its "thin shell" claim corrected in
  the header, README, and CONTRIBUTING). Pure moves,
  one file per commit, suite green between.
  Coordinator: 890/0 at each gate and at the batch
  close, clippy 0, and a dual-tree fuzz comparison
  (baseline worktree at `c3a1b5c` against the final
  tree): both bins byte-identical. One honest
  partial: `settings_tests` (7,512 lines) stayed in
  main.rs — it reaches across every seam — so
  main.rs remains the app's largest file at ~11.7k
  lines, most of it that test module and the App
  core.
- **H-I `b33973e` + `d355282` + `9caca41` —
  boundaries and posture (HF-13, HF-14, E-15,
  m-1..m-6, m-8, S-4, S-8, S-9, S-10).** The process
  runner lives in its own crate (ob-runner) and
  ob-music carries no ob-extract dependency;
  ob-extract's public surface narrowed to its
  contract; workspace version and lints adopted
  with `unsafe_code` denied workspace-wide — exactly
  one unsafe block remains (the runner's killpg),
  under a scoped allow whose SAFETY comment names
  the spawn site it protects. Catalog note names
  sanitized; clipart intake refuses symlinks and
  caps the batch; checkout pinned by SHA in all
  workflows; the PDFium helper fails closed.
  Coordinator: 894/0, clippy 0, manifest and
  workflow checks firsthand, and all five preserved
  fuzz bins still compile against the narrowed crate
  (probe build exit 0) — round 30's net is intact.
- **H-J `af080be` + `c803646` + `1d0d226` — the
  performance tail (Efficiency MINOR-1..4),
  measurement-led.** Landed: the font-scan memo
  (repeat library scan 39.9 ms → 0.049 ms; repeat
  system enumeration 19.5 ms → 0.002 ms on the probe
  harness); a per-document image cache across
  exports (imposed export 380 ms → 18 ms on a shared
  cache; a warm three-export job 1,087–1,451 ms →
  29–58 ms); the resolve_fields token table built
  once per document state (full-document pass
  marginal cost 14.3 ms → 0.76 ms; short call
  12.5 µs → 0.09 µs). Recorded not worth the risk:
  MINOR-4, contribution header tokenization —
  re-tokenization proper measures 3.3 ms of a
  169.2 ms parse-200 median (~2%; the realistic
  count including index rebuilds is 3.3–6.6 ms,
  2–4%), and the doubled clause re-walk it rides on
  measured net-negative under default similarity.
  No parser edit was landed. Coordinator: 896/0,
  clippy 0, fuzz bins byte-identical on the
  coordinator's own runs.

**The standing fuzz condition held across the whole
phase.** On the final tree the unchanged bins
(verdict27 md5 `a9cac6c399d304bac853f2a64f683fca`,
comp27 md5 `4f85c605a841329dabed1d9a2aa84f88`) read:
verdict total_lines=40000, panics=0; composition
D_TOTAL=497024, D_PANICS=0 — byte-identical stdout
to the pre-fix-phase baselines. No existing date
reading moved under any batch.

**Held item.** HF-12 (the `Result<_, String>` error
architecture) was not batched and not started, as
the synthesis directs. It is Jacob's call, now
taken with these results in hand: ten batches, one
BLOCKER and thirteen MAJORs closed, 896 tests,
clippy clean, readings unmoved.

The parser convergence loop resumes from here:
round 30 audits the round-29 close's successor tree
— this close — under the next coordinator.
