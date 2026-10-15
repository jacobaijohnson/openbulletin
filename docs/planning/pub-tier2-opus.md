**Verdict: GO WITH CHANGES.** The scope is right. Fix the container bugs that block phase 2 and the clean-room wording before step 1. Reorder the plan so text ships before layout.

---

## Findings

### Clean-room / legal

**MAJOR: "Clean-room" is the wrong label for this process, and the project's own notes contradict it.**
- *Evidence:* FORMAT-NOTES says libmspub was "cloned beside this file" and cites `parseShape`, `getBlockDataLength` and similar functions per fact.
- *Why it matters:* A clean room separates the people who read the source from the people who write the code. Here, the same party read the source and implemented it.
- *Legal position:* This is still defensible. Facts, offsets and constants are not protectable expression, and libmspub is MPL-2.0, whose copyleft is file-level and is triggered only by copying.
- *The exposure is the claim itself.* A public "clean-room" statement that is provably false invites challenge, and the challenge hurts more than the facts do.
- *Fix:* Call it an "independent implementation from documented format facts."
  - Keep FORMAT-NOTES as provenance.
  - Do not mirror libmspub's function names, file decomposition, type names (`MSPUBBlockInfo`) or comments.
  - If code is AI-assisted, review it for structural or verbatim resemblance to libmspub. Models have been trained on it.

**MAJOR: Microsoft publishes the drawing layer, and the plan sources it from reverse engineering instead.**
- *Evidence:* [MS-ODRAW] is a Microsoft Open Specification under the Open Specification Promise, like [MS-CFB].
- *What MS-ODRAW covers:* the Escher record header, container types F000–F011, FSP flags, the FOPT/complex-property layout, shape types, blip records and the BStore entry (FBSE).
  - That includes the 44-byte descriptor, so the "bytes 10–25 all zero ⇒ delayed" heuristic can be replaced with spec semantics for the FBSE fields (`foDelay`, `cRef`).
- *What stays Publisher-specific:* only the ClientAnchor/ClientData payloads (ids 0x2001–0x2004, 0x6801) and the DG/DGG trailing-4-byte quirk.
- *Fix:* Cite MS-ODRAW for everything it covers. This narrows the reverse-engineering surface and gains patent-promise coverage. The same applies to [MS-OLEPS] for SummaryInformation.

**MINOR: Apache POI is a second reverse-engineering source, and it needs a recorded rule.**
- *Evidence:* POI ships HPBF, a .pub text extractor, and Apache-2.0 code under `org.apache.poi.hpbf`.
- *License effect:* Apache-2.0 is compatible with MIT OR Apache-2.0. But any copied code would make that portion Apache-only and require NOTICE handling.
- *Fix:* Record a rule that POI is consulted for facts only, the same as libmspub.
- *Oracle caveat:* POI's expected `.txt` files are HPBF's output, not ground truth. Treat them as a cross-check and normalize newlines before comparing.

**MINOR: The fixture rules need to be written down.**
- *Evidence:* the brief keeps POI and church files outside the repo.
- *Fix:* Record the license and provenance of each external fixture in a manifest that does live in the repo. Keep church files and their names out of CI logs, because test failure messages print paths and text.

### Code: cfb.rs against MS-CFB

**BLOCKER: Hostile headers can panic the reader, which breaks plan item 8's "never panic" rule.**
- *Shift overflow:* `1usize << sector_shift` and `1usize << mini_shift` panic in debug builds when the shift is 64 or more. They run before any validation.
- *Unvalidated major versions:* The sector-size check only fires when `major` is 3 or 4. With `major == 5` and a `sector_shift` of 0 or 1, `sector_size / 4 - 1` underflows. That panics in debug and wraps in release.
- *Spec rules (MS-CFB 2.2):*
  - major MUST be 3 or 4;
  - byte order MUST be 0xFFFE;
  - mini sector shift MUST be 6;
  - the mini cutoff MUST be 4096.
- *Fix:* Validate all four before any arithmetic. Your synthetic writer uses cutoff 64; change it to 4096 and make the test fixture larger.

**BLOCKER for step 1: Directory indices are destroyed, so a tree walk cannot work.**
- *Evidence:* `if object_type == 0 { continue; }` drops unused entries. After that, `entries[i]` is no longer stream ID *i*.
- *Consequence:* The left, right and child sibling IDs (offsets 68, 72, 76) index the raw directory array, so they cannot be resolved against `entries`.
- *Also:* `DirEntry` does not store those IDs at all, and the root is found by "first type 5" instead of the spec's "entry 0".
- *Fix:* Keep every 128-byte slot, store the three IDs, and require entry 0 to be the root.

**MAJOR: The flat case-insensitive lookup is already ambiguous on real files.**
- *Evidence:* `read_stream("Contents")` uses `eq_ignore_ascii_case`, which also matches `Quill/QuillSub/CONTENTS`. The first match in directory order wins.
- *Consequence:* Sample.pub working proves directory order on that one file, not correctness.
- *Fix:* Delete the flat API rather than keep it beside path lookup.
- *For the tree walk:*
  - Do not trust red-black ordering (some writers emit misordered trees); search the child subtree exhaustively.
  - Guard against cycles with a visited bitset.
  - Cap the depth.
  - Compare names with spec-style uppercase comparison, not ASCII-only.

**MAJOR: Broken chains truncate streams silently.**
- *Evidence 1:* `while cur < END_OF_CHAIN - 1` ends a chain on FREESECT (0xFFFFFFFF) or FATSECT (0xFFFFFFFD) mid-chain without complaint. FREESECT mid-chain is corruption.
- *Evidence 2:* `out.truncate(size)` never checks that the chain supplied at least `size` bytes, so a short stream returns `Ok`. The same is true for the mini chain and the root mini-stream.
- *Why it matters:* This undermines the strictness you want in Contents. A truncated Contents could parse "cleanly" up to the cut.
- *Fix:*
  - Error on any value above MAXREGSECT (0xFFFFFFFA) other than ENDOFCHAIN.
  - Error when the gathered length is less than `size`.
  - Stop gathering once `size` is reached.

**MAJOR: A cyclic DIFAT chain is not guarded and can exhaust memory.**
- *Evidence:* `for _ in 0..num_difat` trusts a header u32 (up to about 4 billion) and has no cycle detection. A self-referencing DIFAT sector pushes 127 entries per iteration.
- *Fix:*
  - Cap `num_difat` at the number of sectors in the file.
  - Track visited sectors.
  - Also stop on DIFSECT (0xFFFFFFFC); the current guard lets it through to `sector()`.
  - Cap `num_fat` the same way.

**MINOR: Short final sectors are rejected.**
- *Evidence:* `sector()` requires a full `sector_size` slice. Real files are often truncated mid-last-sector, and olefile and POI tolerate this by zero-padding.
- *Recommendation:* Accept a short final sector, zero-pad it, and log it, but only when the declared stream size still fits.

**MINOR: Several smaller inputs are trusted without checks.**
- If `minifat.is_empty()`, a small stream is read through the regular FAT, which is wrong per spec; it should be an error.
- The mini chain has no `MAX_STREAM` check.
- `Cfb::open` takes an input of unbounded size.
- On a 32-bit target, `(n as usize + 1) * sector_size` overflows. Use checked arithmetic.

**MINOR: Test coverage stops at the happy path.**
- *Evidence:* there is one v3 fixture with a single FAT sector, no DIFAT chain, no v4 file, and no hostile cases.
- *Add:*
  - a v4 file with 4096-byte sectors and a 64-bit size field;
  - a header with more than 109 FAT sectors, forcing a DIFAT chain;
  - FAT, DIFAT and sibling-tree cycles;
  - a short chain;
  - bad shift values;
  - an in-tree `cargo fuzz` target, in addition to the POI clusterfuzz files.

### Scope and ordering

**MAJOR: The plan builds the riskiest layer before the most valuable one.**
- *Evidence:* Text needs only Quill (step 5). Steps 2–4 (the Contents grammar, which can desynchronize, and the Escher join) are prerequisites for positions only. POI's `.txt` oracle checks text, not layout.
- *Recommendation:* Ship a words-only rescue first.
  1. Path lookup.
  2. Quill: TEXT, STRS, SYID, FDPC/FDPP, FONT and STSH.
  3. Emit spans with every item labeled "Check this" and no positions.
  4. Then add the Contents and Escher join for positions.
- *Benefit:* A layout-layer failure then degrades to text-only instead of "could not be read."

**MINOR: The 0x2C-only cut is right.**
- *Reason:* The notes say one layout spans Publisher 2002–2019, which covers the versions church offices plausibly run. The 0x22 family needs two separate fixed-offset parsers.
- *Gap:* Publisher 2002 and later can still "Save As" Publisher 98/2000. The message for those files should say how to re-save them, not just that they are unsupported.

### Gaps for a church-office rescue

**MAJOR: Master pages are missing.**
- *Evidence:* The notes describe ids 0x0E and 0x0D (master name, applied master), but the plan's step 3 emits only a page's own shapes.
- *Consequence:* Bulletin headers, the church name, the logo and footers typically live on the master, so they would silently disappear.
- *Fix:* Emit applied-master shapes onto each page, or as a master, labeled "Check this."

**MAJOR: Tables are missing.**
- *Evidence:* Chunk types 0x10 and 0x63 and the Quill `TCD ` chunk are in the notes but not in the plan. A table's text is one story split by TCD cell ends.
- *Consequence:* Order of service, schedules and prayer lists are often tables. As planned, they are either dropped or merged into one wrong-confidence block.
- *Minimum for v1:* Split cells by TCD, place them by the row/column size arrays (ids 0x66, 0x67, 0x6D), and label them "Check this."

**MAJOR: WordArt text is not in Quill.**
- *Evidence:* MS-ODRAW geometry-text properties gtextUNICODE (0x00C0, complex form 0x80C0) and gtextFont (0x00C5) live in the Escher FOPT.
- *Consequence:* Bulletin titles are commonly WordArt. Under this plan they would be shapes with no text id, so they would be lost or mislabeled.
- *Fix:* Read gtextUNICODE and emit it as a span labeled "Check this."

**MAJOR: One span per paragraph loses the v1 target.**
- *Evidence:* The dump contract carries bold, italic and size per span, and FDPC runs are finer than FDPP paragraphs. Collapsing to paragraphs drops mixed bold and italic.
- *Fix:* Emit one span per character run, grouped by paragraph.

**MAJOR: Without STSH, most runs will be missing size and font.**
- *Evidence:* Character runs carry only overrides; defaults come from the second STSH chunk, and the notes call that reading empirical.
- *Fix:* Implement STSH. When a resolved property comes only from an unverified default, label that item "Check this."

**MINOR: Linked text frames could duplicate or misplace text.**
- *Evidence:* The notes do not describe how a story continues across multiple boxes, a common Publisher feature.
- *Fix:* Detect several shapes sharing one text id, or a story longer than its box, and badge rather than duplicate.

**MINOR: Deleted, scratch-area and alternate shapes could leak into the import.**
- *Evidence:*
  - FSP flag bit 3 marks deleted shapes;
  - Publisher's scratch area holds shapes outside the page bounds;
  - chunk type 0x20 alternate shapes have an undocumented winner.
- *Fix:* Skip deleted shapes, badge off-page ones, and refuse to emit both members of an alternate pair.

**MINOR: Control characters in TEXT are not mapped.**
- *Evidence:* TEXT is raw UTF-16LE, and the 97/98 path shows 0x0B and 0x0C semantics. Field codes (page number, date) and object-anchor placeholder characters are not mapped.
- *Fix:* Define a mapping table for these characters, and test it against the POI oracle.

**MINOR: Pictures marked "Picture only" carry no image.**
- *Evidence:* Step 6 extracts no bytes, so a "Picture only" badge on an empty box overstates what was carried over.
- *Fix:* Extract PNG and JPEG blips, which are cheap: fixed 0x11 or 0x21 offset, no inflation. Otherwise relabel the badge as "Picture not carried over."

**MINOR: Rotation, insets and columns are known but unused.**
- *Evidence:* The notes document 90° box swapping, text insets 0x81–0x84 and column count 0x8C.
- *Consequence:* Stacking paragraphs down a two-column box overlaps text.
- *Fix:* Apply the insets. Badge rotated or multi-column boxes "Check this."

---

## Strict vs tolerant

**Be strict, but scope the strictness, and never fall back to treating an unknown type as zero-length.**
- **Never guess a length.** An unknown type byte means the reader cannot know how long the payload is. Treating it as zero-length is guessing, and a guess that desynchronizes produces exactly the confident wrong text you fear.
- **Recover only where the file supplies a boundary.** Every variable-length container carries a U32 length that includes itself, and every chunk starts with its own length. When an unknown type appears inside such a container, abandon that container, resume at its declared end, and mark every item derived from it "Check this."
- **Cross-check each chunk's own length against the next chunk's offset.** If they disagree, treat the chunk as unbounded.
- **When there is no boundary, abort Contents only.** If the unknown type appears at an unbounded level, or the declared lengths disagree, stop the Contents/layout layer rather than the whole import. Quill text is independent, so the user still gets a text-only rescue, honestly labeled.
- **Learn from what you refuse.** Log every unknown (id, type) pair and run the reader over the corpus; the framing table should grow from data rather than from tolerance.

---

## Top three changes, ranked

1. **Ship text before layout.** Reorder to path lookup, then Quill text with per-run spans and STSH defaults, validated against the POI oracle, released as a text-only rescue. Contents and Escher then add positions and fail gracefully back to text-only.
2. **Fix cfb.rs before building on it.**
   - Validate the header fields.
   - Keep raw directory indices and the sibling and child IDs.
   - Walk the tree exhaustively with cycle guards.
   - Delete the flat `read_stream`.
   - Error on short or broken chains.
   - Cap and cycle-guard the DIFAT chain.
   - Add v4, DIFAT and hostile-input tests plus a fuzz target.
3. **Add the bulletin-critical items to v1, and fix the clean-room wording.**
   - Master-page shapes, table cells via TCD, and WordArt via gtextUNICODE.
   - Skip deleted shapes and badge off-page ones.
   - Replace "clean-room" with "independent implementation," and source Escher facts from MS-ODRAW rather than libmspub.