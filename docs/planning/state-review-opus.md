# Tier-2 Review: OpenBulletin State Assessment

## D. The Font Question — Ruling First

**Copy-in is correct. The current leaning is right. Pressure test passed.**

### The decisive argument
The NAS/Docker deployment shape has no usable system fonts. Period. A bulletin created on a Windows desktop with a live Calibri reference would break immediately when opened from a Docker container on a Synology NAS. Copy-in is the only architecture that satisfies "the same bulletin lays out identically on every machine" across both the desktop and server shapes.

### Industry practice confirms this

| Practice | What it does | Why |
|----------|--------------|-----|
| InDesign Package | Copies fonts into the package folder | The receiving machine may not have them |
| Google Fonts hosting | Serves font files from servers | Servers have no "system fonts" |
| Fontsource / self-hosted webfonts | Downloads font files to project | Same reason |
| Web-to-print tools (Canva, Vista) | Server-side font libraries | Rendering must be deterministic across sessions |

Font managers (Suitcase Fusion, FontExplorer) solve a different problem: which fonts are *active* on one machine at one time. They reduce menu clutter and app startup time. They do not solve portability. A Suitcase-managed font is still a system font; it breaks when the file travels.

### The "extra step" concern is real but solvable

The picker UX must make copy-in feel like "just using the font":

1. Show all system fonts in a searchable list with live preview.
2. One click copies to library, deduplicates if already present, confirms in a toast.
3. First-run or first-import prompts: "Your bulletin uses Calibri. Add its free twin Carlito to your library?" with a one-tap yes.

The step exists, but it vanishes into normal workflow. The alternative—documents that silently break on other machines—is unacceptable for a tool targeting non-technical admins.

### Refinement worth adopting

Add a plain-language explanation at the copy moment: *"To use [Font Name] in OpenBulletin, it will be copied to your Font Library. This ensures your bulletins look the same on every computer."* This educates without blocking.

### Font packs: yes, ship them

A "Church Starter Pack" should be available at first run and from the Fonts panel:
- Liberation Sans, Serif, Mono (Arial/Times/Courier metric twins)
- Carlito, Caladea (Calibri/Cambria twins)
- Gelasio (Georgia twin)
- 2-3 decorative OFL families suitable for headings (e.g., Playfair Display, Lora, Source Sans Pro)

One tap installs the pack. Seasonal packs (Christmas script faces, Easter elegance) can follow.

---

## A. Decisions to Revisit

### 1. The test corpus is booklets; the acceptance criterion is bifolds

The 17 real bulletins are 16-page half-letter saddle-stitch booklets. The ROADMAP's v0.1 acceptance test is "a real church secretary produces and prints a real Sunday letter bi-fold bulletin." These are different products. A bifold is 4 panels on one sheet; a 16-page booklet is 4 sheets nested and stapled.

**The risk:** If the pilot churches use booklets (as this test church does), they cannot complete the acceptance test with the bifold path. Either:
- Change the acceptance criterion to match the corpus ("a real Sunday bulletin in any supported format including saddle-stitch"), or
- Recruit pilot churches that actually use bifolds, or
- Acknowledge that this church cannot pilot v0.1.

Imposition math for saddle-stitch is already built. The mismatch is in stated goals, not capability.

### 2. PDF export currently shapes one way and renders another

The engine shapes text with Liberation metrics. The PDF asks the viewer to use Helvetica. These are close but not identical:

| Font | 'a' advance at 12pt |
|------|---------------------|
| Liberation Sans | ~7.02 pt |
| Helvetica | ~6.67 pt |

Across a 50-character line, that's potentially 17 points of drift—enough to wrap differently or overflow a frame.

**The risk:** The Check step measures with Liberation. The print measures with Helvetica. A bulletin that passes Check may still overflow on paper. An admin who trusts Check gets burned.

**Decision:** Either embed Liberation (the v1 requirement anyway) before pilot, or have Check measure with Helvetica metrics (kludge, not recommended), or accept that Check is advisory-only until font embedding lands. The current silent mismatch is the worst option.

### 3. The 5-step flow is untested with real users

Start → This Sunday → Announcements → Check → Print is a hypothesis. Real church workflows vary:
- Pastor sends order of service Monday; announcements trickle in through Thursday.
- Some admins paste the entire Order of Service as a block; others fill fields line by line.
- Weddings, funerals, and special services don't follow the "This Sunday" frame.

**Decision:** Before locking the flow into UI and onboarding, test it with 2-3 real admins at real churches doing their real weekly bulletin. A 30-minute screen-share reveals friction invisible in code.

### 4. Template Analyzer parameters are fit to one church

8-word shingles and an 80% presence threshold were tuned on 17 Lutheran bulletins from one congregation. Lutheran ELW liturgy has specific structural patterns (the two ELW clusters the analyzer found). A Baptist church's bulletin may have no standing liturgy; a Catholic parish's may have far more.

**Decision:** Before shipping the analyzer, test on at least 3 more churches of different traditions (one mainline Protestant, one evangelical, one Catholic) and verify the parameters generalize or expose tuning controls for edge cases.

---

## B. Quality Risks in What Is Built

### 1. lopdf extraction is non-deterministic

The corpus findings note PDFium is deterministic; lopdf is not. Non-determinism in a data pipeline means:
- The same PDF yields different extracted text on different runs.
- Test assertions are flaky.
- User-facing behavior varies inexplicably.

**Risk:** Any path that falls back to lopdf (when PDFium is unavailable or for license reasons) will produce unreproducible results.

**Mitigation:** Either (a) make PDFium mandatory and drop lopdf, (b) sort/normalize lopdf output into a deterministic order, or (c) quarantine lopdf behind a "results may vary" warning and never use it for standing-text detection.

### 2. No italic fonts bundled

Liberation Sans and Serif have italic faces. They're not bundled. A church bulletin without italics cannot:
- Italicize scripture citations ("John 3:16")
- Indicate sung responses vs. spoken
- Follow standard typographic conventions for titles

This is not a feature gap; it's an oversight in bundling. The fonts exist and are OFL.

**Fix:** Bundle Liberation Sans Italic and Liberation Serif Italic. Add them to the starter pack. Unblock italic styles in the model.

### 3. fsType is not sufficient for license safety

FONTS.md acknowledges fsType is a signal, not the full license. But many commercial fonts have fsType=0 (Installable) while their EULA prohibits redistribution. A church admin adding a corporate font (e.g., their denomination's custom face) from their system could:
- Copy it into the library (permitted by fsType)
- Embed it in an .obd (permitted by fsType)
- Share that .obd, violating the license

**Risk:** OpenBulletin becomes a vector for accidental license violation.

**Mitigation:** When adding a font that isn't from the known-free catalog, show a plain-language prompt: *"This font was not downloaded from our free-font catalog. Please confirm you have the right to use it in documents you share."* Log the choice. This shifts responsibility without blocking legitimate use.

### 4. Image handling is undocumented

The corpus has "1-2 picture-only cover pages each." Images are mentioned nowhere else in the docs. Questions unanswered:
- What formats are supported?
- How are images placed and sized?
- Are images embedded in .obd or linked?
- Does export include images?

**Risk:** If images exist in the model but are undertested, the pilot will hit image bugs immediately (the test church uses cover images weekly).

**Mitigation:** Document image handling, or if it's truly not built, add it to the critical path for v0.1.

### 5. Documentation contradicts itself on duplex wizard

Project state says "printer duplex wizard persisted per printer" is working. README "Works today" says "Two-sided printing help described above." But the "described above" section is brief and doesn't confirm persistence works. README "Not built yet" doesn't mention it.

**Risk:** External users (including pilot churches) won't know whether to expect this feature.

**Fix:** Sync the README to reality. If the wizard works, say clearly: "The app remembers your printer's flip setting after one test print."

---

## C. What Is Missing for a Complete v1

### Critical for v0.1 (before any pilot can succeed)

| Missing piece | Why it blocks pilot |
|---------------|---------------------|
| **Inline text editing on the page** | Admins expect to click on text and type. Editing in side panels only is disorienting. The README admits this isn't built. |
| **At least one working import path** | A church switching from Publisher cannot start. Even PDF→template analysis (the flagship) doesn't produce output yet—it measures but doesn't draft. |
| **Signed installers** | "Build from source" excludes 100% of the target user base. No installer = no pilot. |
| **PDF font embedding** | Print shops reject PDFs without embedded fonts. Home printers substitute unpredictably. A bulletin that reflows at print time is a failure. |
| **Font selection in the UI** | Only Liberation Sans is usable. A bulletin can't be made to look like the church's existing bulletin (which uses Gill Sans, Bookman, etc.) without at least 4-5 selectable families. |
| **Undo** | Mentioned in the text-spike task but not confirmed working. The "never lose work" principle requires robust, multi-level undo before anyone edits real content. |

### Required for "one real church" v1 (beyond v0.1)

| Missing piece | Why it matters |
|---------------|----------------|
| **Spell check** | Church bulletins are distributed to hundreds of people. Typos ("Pastor Lew" for "Pastor Lee") are embarrassing. Planned for v0.5 but essential for trust. |
| **Hyphenation** | Justified text in narrow columns (common in bulletins) looks terrible without hyphenation. Also v0.5. |
| **Native print dialogs** | "Export PDF then print from your PDF viewer" is an extra step every week. Direct print is expected. |
| **PDF trace layer** | The admin needs to compare their rebuilt bulletin against the original. The swipe-compare is designed but depends on the review screen that isn't built. |
| **.pub rescue import** | The corpus churches are coming from Publisher. IDML helps InDesign users; .pub helps Publisher users. This is the larger population. |

### The gap in the analyzer

The Template Analyzer has measured 17 bulletins and found standing text. But:
- The draft-building layer (families → standing blocks → suggested template) is not built.
- The Accept/Edit/Cancel review screen is not built.
- Font matching preview (original vs. candidate side-by-side) is not built.

The analyzer currently produces *insights for developers*, not *output for users*. Until it drafts a template and lets the admin confirm it, the import story is incomplete.

---

## Summary Table

| Section | Key finding | Severity |
|---------|-------------|----------|
| D (Fonts) | Copy-in is correct; implement frictionless picker and starter pack | Design confirmed |
| A1 | Acceptance criterion (bifold) mismatches corpus (booklet) | High—pilot may fail |
| A2 | Shaper vs. exporter font mismatch undermines Check step | High—user trust |
| A3 | 5-step flow untested with real admins | Medium—may need rework |
| A4 | Analyzer tuned on one tradition only | Medium—may not generalize |
| B1 | lopdf non-determinism | Medium—testing/debugging |
| B2 | No italics bundled | Low effort to fix |
| B3 | fsType insufficient for license safety | Medium—liability |
| B4 | Image handling undocumented | Unknown until clarified |
| B5 | README contradicts project state on duplex | Low—documentation only |
| C | No inline editing, no working import, no installer, no font embedding, no font picker | Blocking—v0.1 cannot ship |

---

## Recommended Immediate Actions (in order)

1. **Bundle italic fonts** — one-hour fix, unblocks basic typography.
2. **Embed Liberation in PDF export** — unblocks accurate Check and printable output.
3. **Build the system font picker** — copy-in UX, enables font selection in styles.
4. **Ship the Church Starter Pack** — covers 80% of metric-twin needs at first run.
5. **Build inline text editing** — without this, the app is not an editor.
6. **Build signed installers** — without these, no one outside development can test.
7. **Build the analyzer's draft output + review screen** — turns measurement into a usable import path.
8. **Resolve the bifold/booklet acceptance criterion** — know what you're testing.