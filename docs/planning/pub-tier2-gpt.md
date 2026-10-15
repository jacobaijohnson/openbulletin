GO WITH CHANGES — the 0x2C cut is reasonable, but phase 2 needs container hardening and explicit text-loss accounting before imports can be trusted.

## Findings

- **BLOCKER — Hostile headers can panic before validation.**  
  **Evidence:** `Cfb::open`: `1usize << sector_shift` and `1usize << mini_shift` execute on unchecked U16 values. Unsupported major versions bypass the sector-size check entirely. Subsequent sector arithmetic is unchecked. Validate major ∈ {3,4}, the corresponding shift ∈ {9,12}, mini shift = 6, byte order, and cutoff = 4096 **before** shifts/arithmetic; use checked offset calculations. These are header requirements in **[MS-CFB], Compound File Header**.

- **BLOCKER — Resource use is not actually bounded by `MAX_STREAM`.**  
  **Evidence:** DIFAT iteration trusts `num_difat`, does not detect repeated sectors, and keeps accumulating FAT-sector references. Directory, root mini-stream, and regular-stream reads gather entire chains before truncating. A cyclic chain can therefore allocate far beyond its declared size before the `fat.len()+1` limit rejects it. Cap counts against physical file size, reject repeated sectors promptly, and bound aggregate allocation and work—not merely declared stream size.

- **MAJOR — Invalid chain termination and truncated streams return success.**  
  **Evidence:** `while cur < END_OF_CHAIN - 1` treats FATSECT, ENDOFCHAIN, and FREESECT alike as successful termination. `out.truncate(size)` never checks that enough bytes were obtained; root mini-stream loading has the same defect. **[MS-CFB], FAT/Mini FAT sectors** distinguish allocation markers from ENDOFCHAIN. Require proper termination and sufficient capacity; validate declared miniFAT and, for v4, directory-sector counts against their chains.

- **MAJOR — Small-stream fallback can read unrelated regular sectors.**  
  **Evidence:** `read_stream` chooses regular FAT whenever `self.minifat.is_empty()`, even for a nonempty stream below the cutoff. Under **[MS-CFB], Mini Stream/Mini FAT allocation rules**, those streams use mini sectors; missing allocation metadata is corruption, not an alternative encoding. Handle zero-length streams separately and reject missing miniFAT/root capacity for nonempty small streams.

- **MAJOR — Path lookup requires preserving directory IDs, not just adding traversal.**  
  **Evidence:** directory parsing drops unused entries with `continue`, and `DirEntry` discards sibling/child IDs. **[MS-CFB], Compound File Directory Entry** defines those references as directory-entry indices. Preserve every slot or maintain an explicit ID map; validate root entry 0, reference bounds, cycles, object types, and duplicate names within a storage. `eq_ignore_ascii_case` also does not implement CFB’s full directory-name comparison rules.

- **MAJOR — The synthetic fixture is not spec-conforming.**  
  **Evidence:** `build_test_cfb` sets the mini cutoff to 64 instead of the required 4096 and leaves the root’s child ID as `NO_STREAM`, making its streams unreachable through the directory tree. Fix the writer before using it as the phase-2 oracle. Add valid v4, multi-sector/fragmented DIFAT, mini-stream boundary, truncated-chain, cyclic-chain, and duplicate-name-in-different-storage tests.

- **MAJOR — “Clean-room” overstates the separation actually described.**  
  **Evidence:** FORMAT-NOTES explicitly derives facts by reading libmspub source; no researcher/implementer separation is documented. An independently written, facts-only implementation is a coherent posture, but not proof against copying protected expression or a formal clean-room process. Keep source/provenance records and review for copied structure, comments, and code—not just literal matches. Counsel should check applicable reverse-engineering restrictions, patent/specification assurances, fixture permissions, and any distribution obligations of the already-shipped bridge. Public availability alone is not a license grant.

- **MAJOR — Paragraph-only spans cannot meet the stated character-style target.**  
  **Evidence:** FORMAT-NOTES **Text** says FDPC character runs and FDPP paragraphs partition text separately; defaults involve STSH and paragraph default-character-style indices. Plan item 6 emits one span per paragraph. Emit paragraphs containing styled runs, or split spans at the intersection of paragraph and character-run boundaries; otherwise mixed bold, italic, fonts, and sizes disappear. Include STRS story lengths and explicit default-style resolution in item 5.

- **MAJOR — Master-page content is omitted from the plan.**  
  **Evidence:** FORMAT-NOTES **Pages** identifies master pages through block `0x0E`, applied masters through `0x0D`, and four dummy-page seqNums. Implement master composition and dummy-page handling before ordinary page emission. Otherwise recurring service headings, church names, and footer content can vanish or appear as extra pages.

- **MAJOR — Tables and story placement need a rescue policy.**  
  **Evidence:** FORMAT-NOTES describes table shape/cells chunks (`0x10`/`0x63`), row/column geometry, and Quill `TCD` cell-text ends; the plan handles none of these. Flattening a service schedule or financial table into one box loses meaningful associations. Recover cell text with row/column labels, or explicitly flag unresolved table structure. Test linked text boxes/shared stories too: a story-to-text-ID join alone does not establish how text is distributed among boxes.

- **MAJOR — “Shallow Escher” omits geometry needed even for text boxes.**  
  **Evidence:** FORMAT-NOTES **Shapes and geometry** specifies extra DGG/DG tails, repeated client-record lengths, rotation, flips, text insets, and text columns. Item 4 lists anchors and group rescaling only. Add framing quirks, signed-coordinate handling, and zero-extent group checks; support or flag rotation/columns/insets. Preserve traversal z-order. Approximate paragraph stacking must receive **Check this**, not **Matched** geometry.

- **MAJOR — Unsupported objects need visible loss reporting, not misleading badges.**  
  **Evidence:** item 6 extracts no image bytes, yet labels pictures **Picture only**; that label suggests a retained visual representation. Use “Picture omitted” with a positioned placeholder unless an actual preview exists. Inventory unhandled objects—including WordArt, whose extraction is not established by these notes—and unresolved/missing shape-text joins. Account for every Quill story as emitted, intentionally excluded, or unresolved; otherwise readable words can disappear silently.

- **MAJOR — Current wording oracles do not validate layout or feature fidelity.**  
  **Evidence:** item 8 proposes POI expected-text files as the wording oracle; the packet establishes container success, not expected geometry, styles, tables, or master composition. Add consented church-bulletin fixtures and verified structural expectations across available 0x2C releases. Include mixed formatting, non-ASCII/surrogate pairs, tables, masters, groups, linked boxes, and unsupported objects.

## Strict versus tolerant

Be **strict about unknown framing, tolerant about unknown semantics**. An unknown block **type** has no trustworthy payload boundary: stop that parse unit rather than guessing zero length or scanning for a plausible next header. An unknown **ID** with a known type can be skipped using its validated length. Partial rescue is acceptable only when another unit has independently validated boundaries and joins; mark the import incomplete and report what was omitted. Do not continue inside a desynchronized unit or label its output Matched. Whole-file rejection is the safe initial policy where such isolation is not yet implemented.

## Top three changes, ranked

1. **Harden CFB first:** validated header constants, checked arithmetic, physical-size/work budgets, exact chain validation, preserved directory IDs, and conforming adversarial fixtures.
2. **Make text completeness the core milestone:** STRS/SYID stories, styled runs/defaults, master composition, table recovery, and explicit accounting for unresolved stories and objects.
3. **Define fidelity acceptance tests before layout work:** verified 0x2C bulletin fixtures, approximate-geometry badges, truthful omitted-picture labels, and checks for signed/group/rotated coordinates.

The proposed Contents → Escher → Quill dependency order is sound after path-aware lookup; retain 0x22 deferral and its explicit fallback message.