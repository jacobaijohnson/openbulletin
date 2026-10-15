# Tier-2 synthesis: the native .pub reader (2026-10-15)

Jacob directed a Tier-2 review ("take time and have opus and gpt
also review") of the decision to write our own Rust .pub reader.
Reports: pub-tier2-opus.md, pub-tier2-gpt.md (same packet: the
ownership decision, FORMAT-NOTES, cfb.rs as committed, the
phase-2 plan). Coordinator adjudication below; every code claim
was re-checked against cfb.rs before adoption.

## Verdicts

- Opus: GO WITH CHANGES. GPT: GO WITH CHANGES.
- Both endorse the 0x2C-only v1 cut and the strict-on-framing
  answer. Estimated spend for the round: about $0.42.

## Adopted (verified against the territory)

1. **CFB hardening is a phase of its own, before anything builds
   on the container.** Both reviewers found, and the coordinator
   confirmed at cfb.rs lines 58-65: sector shifts execute before
   validation, a major version other than 3/4 bypasses the size
   check entirely, the mini shift is unchecked. Also adopted:
   chains must error on broken/short gathers instead of
   truncating silently; DIFAT iteration needs caps and cycle
   detection; a missing miniFAT for a small stream is corruption,
   not a fallback; the input size is capped.
2. **Directory entries keep their raw indices.** Phase 1 drops
   unused entries and discards sibling/child ids, which makes
   the planned tree walk impossible (Opus BLOCKER, confirmed).
   Phase 2a stores every slot plus the three ids, requires entry
   0 as root, searches child subtrees exhaustively (writers
   misorder the tree), with cycle guards and a depth cap. The
   flat read_stream API is deleted, not kept beside path lookup
   (it is already ambiguous: case-insensitive "Contents" also
   matches Quill's CONTENTS).
3. **Order flips: text before layout.** Phase 2b is a Quill-only
   rescue (path lookup, TEXT/STRS/SYID/FDPC/FDPP/FONT/STSH),
   spans per character run (paragraph-only spans lose mixed
   bold/italic: both reviewers), every item badged Check this,
   no positions. Phase 2c adds Contents + Escher positions and
   fails soft to the text-only result. POI's .txt files are
   HPBF's output, a cross-check, not ground truth (Opus).
4. **Bulletin-critical content joins v1:** applied master-page
   shapes (badged Check this), table cells split by Quill TCD
   with row/column placement (Check this), WordArt text from
   the Escher gtextUNICODE property (Check this), deleted
   shapes skipped, off-page shapes badged, text insets applied.
5. **Loss accounting (GPT):** every story is emitted,
   intentionally excluded, or reported unresolved; the review
   note says the counts. Pictures: extract PNG/JPEG blips if
   cheap, else the honest label is "Picture not carried over,"
   never "Picture only" without a picture.
6. **Strict vs tolerant, settled:** never guess a length. An
   unknown block type inside a length-carrying container
   abandons that container and resumes at its declared end
   (items badged Check this); at an unbounded level it aborts
   the Contents/layout layer only, and the Quill text rescue
   still lands. Unknown (id, type) pairs are logged so the
   framing table grows from data.
7. **Wording (both reviewers, MAJOR):** this is an independent
   implementation from documented format facts, NOT a
   clean-room process (the same party read the record and
   writes the code; facts and constants are not protectable
   expression, and no libmspub code is copied). Escher facts
   should cite [MS-ODRAW] and [MS-OLEPS] where Microsoft
   publishes them, narrowing the reverse-engineered surface to
   the Publisher-specific payloads. POI is facts-only too,
   recorded. The LibreOffice bridge distributes nothing: it
   invokes a user-installed program; LEGAL.md will say so.
8. **Tests:** the synthetic CFB writer must become spec-true
   (4096 cutoff, reachable tree); add a v4 fixture, a DIFAT
   chain fixture, cycle/short-chain/bad-shift hostiles, then
   the POI clusterfuzz files. The 0x22 family's message will
   say how to re-save (newer Publisher Save As, or the bridge).

## Phase plan as revised

- 2a: CFB hardening + path-aware lookup (items 1, 2, 8).
- 2b: Quill text rescue, per-run spans, STSH defaults, loss
  accounting (items 3, 4 partial, 5).
- 2c: Contents + Escher positions, masters, tables, WordArt,
  blips, scoped-strict Contents walk (items 4, 5, 6).
