# Music notation in OpenBulletin: landscape and verdict

Written 2026-10-15. Question from Jacob: some bulletin pages
carry real music, with notes. Today that music reaches the
bulletin as a flat picture. Can OpenBulletin treat music the
way it treats text: understand it, lay it out, and keep it
honest?

## Scope, as Jacob corrected it

**OpenBulletin will never edit notes.** No transposing, no
pitch fixes, no notation authoring. That work stays in
Sibelius, MuseScore, and their kin, and this report does not
recommend revisiting it. Our job is three things only:

1. **Understand** where the notes are: parse the music file's
   structure (parts, systems, measures, lyrics).
2. **Lay it out** correctly and honestly across the
   bulletin's pages: choose system and page breaks well.
3. **Keep it legible and singable** for a layperson: the
   congregation reads and sings from this, so notation is
   never silently shrunk to make it fit.

An engraving engine is therefore needed for display and
print layout only. Everything below is judged against that
scope.

## The short answer

- The door in is **MusicXML** (`.musicxml` / `.mxl`). Every
  serious notation program exports it, it is an open W3C
  community format, and a real Rust parser for it exists.
- Of the native files, only MuseScore's `.mscz`/`.mscx` is
  readable on its face (a zip of XML). Sibelius `.sib` is
  encrypted. Finale `.musx` is undocumented with partial
  reverse-engineered readers. Dorico `.dorico` is
  undocumented. None of them is worth a reader before
  MusicXML works; MuseScore files can also just export
  MusicXML.
- The engine is **Verovio**, run as an out-of-process helper
  beside the app, exactly like the PDFium helper. It is the
  one mature open engraver that is a library rather than an
  application, its license (LGPL-3.0) is manageable in a
  separate process we do not modify, it renders SVG one page
  at a time at a geometry we choose, and it lets the caller
  control staff size and which breaks to honor.
- In our own corpus, music today is **raster only**: scans
  and pictures at about 150 dpi. Native music support is
  therefore not a rescue upgrade for old bulletins. It pays
  off the first time the music director hands over a
  notation file instead of a picture of one.

## What the corpus tells us (measured 2026-10-15)

Structural inspection of all 17 real bulletin PDFs (272
pages), aggregates only:

- Every file's producer is Microsoft Publisher for
  Microsoft 365.
- **Zero notation fonts** appear on any page. Every embedded
  font is a text face. Engraved notation in a PDF normally
  announces itself with a music font (Opus, Bravura, Maestro,
  and friends); none is present.
- There are **316 embedded images**; **73 of 272 pages**
  (about one page in four) are picture-dominated, with
  images covering more than 40% of the page.
- The typical inserted picture prints at about **150
  effective dpi**, which is screen resolution, not print
  resolution. A handful of high-resolution scans (roughly
  340 to 770 effective dpi) exist.
- Identical picture files recur week after week in the same
  positions, including one tall image (about 740 by 1920 px)
  spanning the same pages of several bulletins. Inference,
  labeled as such: standing inserts are reused, and Jacob
  confirms some of them carry music and notes.
- No file carries an embedded attachment, so no notation
  data is hiding inside these PDFs.

Honest limit: structure alone cannot prove which pictures
are music and which are photos or art. It does not matter
for the verdict: in whatever form the music arrives today,
it arrives as pixels, and pixels cannot be reflowed, only
placed. Anything native has to start from a notation file
someone exports.

## 1. The interchange format: MusicXML

MusicXML is an open XML format for Western notation,
stewarded by the W3C Music Notation Community Group since
2015; version 4.0 (June 2021) is current, and the spec is
published under the W3C Community Final Specification
Agreement, free to implement. The W3C abstract counts over
250 supporting applications. Uncompressed files are
`.musicxml`; `.mxl` is the same document in a zip container
with a `META-INF/container.xml` index, a shape our codebase
already handles for IDML and, soon, `.mscz`.

**Who exports it:** all of the majors. MuseScore (import
and export), Dorico (import and export), Sibelius (native
export since version 7 in 2011, native import since 2020.6;
version 6 and earlier needed the Dolet plug-in), and Finale
(export; MakeMusic shipped a Finale v27 build specifically
so users could export MusicXML 4.0 while migrating away).
Notion, Capella, and many smaller tools also export it.
There is one practical wrinkle for churches: the free tier
of Sibelius has historically limited export functions, so
"just export MusicXML" can depend on which Sibelius license
the music director holds. That is a support sentence in our
import dialog, not a blocker.

**How complete is the transfer?** MusicXML carries the
music: notes, rhythms, voices, staves, lyrics, dynamics,
articulations. It does not promise the original's layout;
a church arranger quoted by HymnCharts found a Finale to
MuseScore move arrived with "the notes and lyrics there but
the formatting completely off", which matches the format's
design intent. Since our scope forbids note editing and
requires us to do our own layout anyway, this split is
acceptable: MusicXML gives us exactly the layer we are
responsible for, and not the layer we are not.

Exporter quality differs, and the best public evidence is a
2022 academic comparison (Hahn and colleagues, tested
Dorico 4.3/5.0, Finale v25.5, MuseScore 4.0/4.1, Sibelius
2022.12 native and via Dolet 8, poster on Zenodo). Its
conclusions, in substance: MuseScore's export and
Sibelius-via-Dolet were the strongest overall, MuseScore a
hair ahead; Finale's export was solid out of the box;
Dorico's export dropped some marks entirely (for example
technical marks such as up-bow and down-bow); Sibelius's
*native* export was not recommended for edition work,
because it mangles some glissandos and expresses many
glyphs as program-specific markup instead of standard
elements. Avid's own launch material for Sibelius 7 notes
the built-in export does carry formatting data (bar widths,
note spacing) that a plug-in cannot reach. Both can be true:
the musical content transfers well from every major; the
fine engraving detail is where dialects diverge. For our
use (understand structure, lay out ourselves) every major's
export clears the bar, with Sibelius-native files the most
likely to need "Check this" labels on exotic marks.

**Finale's status matters to the timing.** MakeMusic ended
Finale development and sales on August 26, 2024, ended
support a year later, and partnered with Steinberg to move
users to Dorico, with MusicXML as the bridge. Decades of
church Finale files are now orphaned in place. That is the
same rescue shape as Publisher's discontinuation, and it
strengthens the case for MusicXML import, not for a Finale
reader.

**MEI, the alternative.** MEI (Music Encoding Initiative)
is a community XML format built for scholarly editions; it
is Verovio's native format and the richer of the two for
editorial markup. But notation applications barely speak
it: MuseScore lists MEI import/export among its features,
and that is close to the whole list. No church workflow
produces MEI files. Verdict: MEI is our engine's internal
language if we ever need it, never an import door.
**MNX**, the W3C group's JSON-based would-be successor, is
still pre-1.0 and explicitly not ready; watch it, do not
build on it.

## 2. Native notation files: readable in principle?

| File | What it is | Readable by us? |
|---|---|---|
| MuseScore `.mscz` / `.mscx` | `.mscx` is plain XML; `.mscz` is a zip of it with a `META-INF/container.xml` manifest, thumbnails, and images (MuseScore Handbook) | **Yes, in principle.** The structure is open on its face and third-party parsers exist. Caveat: the schema shifts between MuseScore major versions and carries MuseScore's own layout data, which we would ignore. |
| Sibelius `.sib` | Proprietary binary; the file-format record at Just Solve the File Format Problem states current files are "encrypted using an unspecified algorithm" | **No.** MuseScore's own forum answer is blunt: the format is proprietary and closed, and direct import is unlikely to be implemented anywhere outside Sibelius. The .pub analogy fails here: .pub sat in a documented container, unencrypted. |
| Finale `.mus` / `.musx` | Proprietary, never publicly documented. Michael Good (MusicXML's inventor, at MakeMusic) is quoted on the Steinberg forum: much of how Finale displays music is computed in the program, not stored in the file | **Partially, at unjustifiable cost.** The open-source Denigma utility reads `.musx` for basic notation, and the musx2mxl converter built on it describes itself as supporting "only basic notations", a work in progress. A reader would be a research project to reach what MusicXML export already gives us. |
| Dorico `.dorico` | Proprietary project file; Steinberg publishes no format documentation (unknown: no public description of its internals was found in this research) | **No basis to attempt.** Dorico imports and exports MusicXML by design; that is the supported path, and Steinberg's own Finale-migration guidance runs through it. |
| Guitar Pro `.gp`, Capella `.capx`, others | Zip/XML shapes exist | Out of scope: not church-bulletin sources. |

The pattern matches the rest of OpenBulletin: one open
interchange format done well beats four proprietary readers
done thinly. `.mscz` is the only candidate for a later
native reader, and only if real church MuseScore files show
up whose MusicXML export loses something that matters.

## 3. Engraving engines we can use without writing one

Writing an engraver is explicitly out of scope (it is a
multi-decade discipline; Gould's *Behind Bars* runs 676
pages on notation practice alone). The candidates:

| Engine | License | Shape | Output granularity | Caller control of size and breaks |
|---|---|---|---|---|
| **Verovio** | LGPL-3.0 | C++20 library and CLI; also WASM | SVG, one page per render call | Page width/height, scale, spacing, lyric size, music font; `breaks` option: auto, line, encoded, none |
| MuseScore engine | GPL-3.0 | A whole application (Qt), not a separable library | PDF/SVG/PNG pages via CLI conversion | Layout lives in the score/style; the CLI converts, it does not re-lay-out to your geometry |
| LilyPond | GPL v3 | A batch typesetter with its own input language (`.ly`, Scheme-extensible) | Whole-score PDF/PS/SVG | Total, but only by generating its language; staff size is a document setting |
| VexFlow | MIT | JavaScript rendering API | Whatever you draw: you place every measure yourself | Total programmatic control, because there is no layout engine to ask |
| OpenSheetMusicDisplay | MIT (project site and repo) | TypeScript, built on VexFlow, parses MusicXML | SVG in a browser, systems and measures addressable | Good for a viewer; engraving quality is viewer-grade |

**Verovio** is the serious candidate, and the only one that
fits our architecture. It is a library for engraving scores
into SVG, MEI-native, with built-in converters for
MusicXML (plain and `.mxl`), ABC, Humdrum, and others. Its
toolkit options are the layout contract we need: `pageWidth`
and `pageHeight` in its internal units, `scale`, spacing
options for staves and systems, `lyricSize`, a choice of
five SMuFL music fonts (Bravura, Leipzig, Leland, Gootville,
Petaluma, all openly licensed fonts), `adjustPageHeight` to
crop a render to its content, and the `breaks` option:
`auto` (engine decides), `line` (honor line breaks encoded
in the source, which is where MusicXML `<print
new-system="yes"/>` lands), `encoded` (honor system and page
breaks from the source), `none` (one flowing stream). Every
SVG element carries its source element's ID unless told
otherwise, so a rendered system stays traceable to the
measures it holds. Two honest cautions from Verovio's own
documentation: its MusicXML converter page says the feature
"is still under development, and its capabilities are
limited", and its changelog shows a steady drip of
MusicXML importer fixes (multi-measure rests, header
conversion) as recently as 2025. The importer is real and
improving, and it will mangle some files; our import review
labels exist for exactly this. Version note: Verovio now
releases yearly (5.x in 2025), actively maintained by RISM
Digital. There are also young Rust bindings on crates.io
(`verovio` 0.3.6, a cxx bridge, plus `verovio-data` with the
SMuFL resources; a few hundred downloads), which prove the
FFI path exists but are not something to bet the build on
yet.

**Why not the others:**

- **MuseScore's engine** produces excellent engraving, but
  it is GPL-3.0 and fused into a Qt application; MuseScore's
  own forum advice to would-be embedders is to run MuseScore
  as a service and call its CLI to convert between formats.
  As an *external install* (the LibreOffice-bridge pattern)
  it could render notation PDFs for placement, the same way
  the bridge handles `.pub` today. As a bundled or linked
  component it fails our license posture, and its CLI cannot
  re-lay a score out to a 5.5-inch panel: the layout in the
  file is the layout you get.
- **LilyPond** is a typesetter, not a library: you write or
  generate `.ly` code and it compiles beautiful pages. Its
  MusicXML door is `musicxml2ly`, whose own manual page is
  a list of things conversion may drop or reinterpret
  (articulation directions, rest positions, system and page
  breaks are each individually switchable, which tells you
  how partial the mapping is). It is also GPL, slow as a
  batch compiler, and its output is whole pages at the
  document's own size. Wrong shape for a helper; skip.
- **VexFlow and OpenSheetMusicDisplay** are permissively
  licensed (MIT) and genuinely good at putting notes on a
  web page, with per-measure control. They are JavaScript,
  so using them means shipping a JS runtime or a webview in
  a Rust desktop app, and their engraving is aimed at
  screens and interactivity rather than print hymnody.
  Wrong runtime, wrong finish line; skip, while noting
  OSMD proves the MusicXML-to-systems parsing model we want
  for Phase B is a solved shape.

**The helper pattern and licenses.** Our PDFium helper is
the template: an unmodified third-party binary beside the
app, spoken to over files and JSON, failure-isolated. For
Verovio that pattern is also the license-clean one. LGPL-3.0
permits distributing the unmodified helper with its license
text and a source offer for the library; there is no
linking question at all because there is no linking. (The
alternative, statically binding Verovio into our binary via
the young Rust crates, imports LGPL relinking obligations
into a static Rust build and a C++ toolchain into every
developer's machine. Do not.) Bundled SMuFL fonts ride
under their own open licenses (SIL OFL for Bravura and
Leland). None of this touches our own code's MIT OR
Apache-2.0 grant.

One integration seam to name now: Verovio emits SVG, not
PDF. Getting its vector output into our print PDF means an
SVG-to-PDF-path step in the helper chain (a resvg-class
renderer; its MPL-2.0 license needs a pass through our
existing license-audit process) or, as an honest fallback,
rasterizing the SVG at true print resolution. Either is
Phase A work, and neither changes the engine choice.

## 4. Legibility: the floor "fit" may not cross

Congregational music is read at arm's length, often by
older eyes, in uneven light, while singing. The norms below
are for staff height, the distance from the top to the
bottom line of one staff, which is the size every engraver
quotes.

| Source | Norm |
|---|---|
| MOLA (Major Orchestra Librarians' Association) guidelines, via the Indiana University Jacobs School style guide | Parts: staff 7.0 to 7.5 mm. Full scores: minimum 4 mm (a conductor's desk document, not a singer's) |
| Universal Edition product design standards | Orchestra parts 7.5 mm; choral scores, piano music, and sales editions: 7 mm "is sufficient" |
| Rastral (staff-size) scale, trade reference | Size 3 = 7.0 mm is the standard institutional size; size 8 = 3.7 mm exists for pocket scores and dense orchestral layouts |
| Working engravers' practice (engraving handouts and forums) | Piano/vocal scores commonly 7 mm; choral scores on letter paper as low as 5 mm; professional piano music "usually 6 mm or a bit bigger" |

Synthesis for our pages, labeled as our judgment on those
sources: hymnody is vocal/choral music, so its band is
**5 to 7 mm**. Therefore:

- **Hard floor: 5.0 mm staff height** (the measurement
  allows 0.02 mm of slack). Below every cited
  vocal and choral norm. If a hymn cannot fit the panel at
  5.0 mm, the answer is more panels or fewer systems per
  panel, never a smaller staff. The app refuses and says why
  in plain words, the way the learning door refuses a
  too-small pick today.
- **Target: 6.0 to 7.0 mm** on the 5.5-inch bulletin panel
  where the music allows it.
- **Lyrics ride with the staff.** In Verovio, lyric size is
  set in the same units as the staff (default `lyricSize`
  4.5 against a staff height of 8 units), so holding the
  staff floor holds lyric size proportionally, at roughly a
  9 to 10 point equivalent when the staff is at the floor.
  A separate lyric floor is unnecessary if the staff floor
  holds; if a source file forces compressed lyrics, that is
  a "Check this" label, not a silent accept.
- **Spacing is part of legibility.** Systems need air:
  Verovio's default system spacing (12 units, one and a
  half staff heights) is the starting point, and cramming
  systems together to win a panel is the same sin as
  shrinking the staff, one axis over. The floor applies to
  both.

Every candidate engine's size control, for the record:
Verovio exposes page geometry, scale, and spacing directly
to the caller, and its SVG output can be measured, so the
floor is enforceable mechanically (render, measure the
staff in millimetres, refuse below 5.0). MuseScore takes
staff size from the score's style, not from the converting
caller. LilyPond would take it from generated code.
VexFlow would take it from our own drawing calls. Only
Verovio gives us an enforceable floor as a library call.

## 5. What "native" means, in phases

**Phase A: recognize, render, place as vector.**
The one open door learns the music kinds alongside the
image kinds Jacob named (SVG, PNG, JPEG, WebP stay pictures,
honestly labeled): a `.musicxml` or `.mxl` file is music,
and so is a notation PDF exported from an engraver. The
helper renders the MusicXML at a chosen geometry; the
result is placed as scalable vector that can split across
bulletin pages at page boundaries, at or above the
legibility floor, with Matched / Check this labels for
anything the importer dropped. Cost: one helper binary to
build and stage per platform (the PDFium lesson: the
bundle must be one download), the SVG-to-print path above,
and import review wiring that already exists for other
formats. This phase alone beats today's 150 dpi pictures
for any hymn that arrives as a file.

**Phase A as built (shipped 2026-10-16, M1).** What
actually landed, where it differs from the sketch above:
- The parse is our own light walk in a new crate,
  `ob-music` (title, creators, rights, part names, and the
  structural counts), not the `musicxml` manipulation
  crate: Phase A needs to count and label, not to model
  the music, and a hand-rolled walk keeps it that way.
  `.mxl` unzips through its container; files are also
  recognized by content sniff at the one open door. A
  notation PDF is NOT a music door in this build; it
  comes in through the ordinary PDF rescue as pictures,
  as before.
- The helper is `ob-verovio-helper`, speaking versioned
  JSON like the PDFium helper. It drives Verovio two ways,
  in order: a command-line program when one is present,
  else a Python carrying the official Verovio wheel
  (development points it at a venv through
  `$OB_VEROVIO_PYTHON`). There is no third route: checked
  2026-10-16, Verovio publishes no standalone Windows
  binary, so the Windows bundle ships the helper without
  an engraver and the app degrades honestly (the draft
  forms, the notes stand undrawn, a Check-this says why).
  docs/LEGAL.md carries the entry.
- Rendering follows the spike's method (page width from
  the staff formula, options as a dict read back after
  setting), one tall page per score. This build does NOT
  render one page per system; it cuts the page SVG at
  system boundaries, each system's crop measured from its
  own ink in a real rasterization, which keeps tempo
  marks and dynamics that staff arithmetic alone would
  clip.
- The floor is enforced BY MEASUREMENT on every render:
  the staff height is read out of the drawing, and a
  render measuring under the floor (5.0 mm, with 0.02 mm
  of measurement slack) is refused whole, naming
  the measured size (a request for 3.0 mm came back
  measuring 3.7 mm and was refused; a narrow 60 mm panel
  at the 6.5 mm target simply wrapped into more systems,
  all measuring 6.50 mm). Lyrics ride the staff size;
  there is no separate control.
- The draft carries the file's words as words: title and
  composer in a heading, and the file's rights line (or a
  composed Words/Music line) as the credit under the
  music, closing the spike's named loss deliberately.
  Each system is a picture box marked as music in the
  model: it can be MOVED on Pages but not resized, and
  its selection line and right-click menu say so in one
  sentence. The music file itself is stored in the
  document, so the bulletin travels complete.
- Every import compares counts: the file's notes,
  measures, syllables, and fermatas against the drawn
  SVG's notehead, measure, syllable, and fermata groups.
  Shortfalls land as Check-this items naming the numbers;
  a full count lands as "Every note in the file is in the
  printed music." Measured on the spike's Bach chorale
  through the real helper: 237 of 237 notes, 14 of 14
  measures, 13 of 13 fermatas (fermatas ARE countable in
  the SVG), staff 6.50 mm at the 6.5 mm target. Syllables
  drew 59 groups against 58 counted in the file: Verovio
  also groups the extend-only lyric, so syllable counts
  can read one high on the drawn side; the comparison
  accuses only when the print shows FEWER than the file
  holds, and both numbers always print in the item.
- Print rides the G2 picture path: each system's SVG is
  rasterized at the export purpose's resolution, so at
  the "print shop or publication" purpose (600) the
  notation is crisp vector-born print, and a music box
  whose drawing is missing prints the placeholder and is
  counted, never a silent gap.

**Phase B: understand structure, choose the breaks.**
Parse the MusicXML ourselves in Rust (the `musicxml` crate,
version 1.1.2 on crates.io, is a real parsing and
manipulation library, about 21,000 downloads; MEI the same
shape if ever needed) into parts, staves, measures, and
lyric lines. The document model gains a music block whose
frames hold measure ranges; the app, or the admin with
plain controls ("start the new page here"), chooses system
and page breaks at measure boundaries, and the helper
re-renders with `breaks` set so those choices are honored
(the structure-level way to do this is to mark the chosen
breaks in the interchange data handed to the engraver,
which is layout instruction, not note editing). Reflow
across panels of a different size is then just: same music,
new geometry, new breaks, floor enforced. Cost: a real
model and UI build, comparable to the `.pub` layout phase,
plus a test corpus of real hymn files. This is the phase
that makes music a citizen like text.

**Phase C: editing notes. Rejected, permanently.**
Per Jacob's scope ruling. Transposition and pitch fixes
belong to notation software; if a church needs them, the
export door can hand the file back out. (For the record,
Verovio does have a transpose option, and it changes
nothing about this ruling.)

## Recommendation

Build **Phase A on Verovio-as-helper**, behind the same
one-door recognition as everything else, with the 5.0 mm
staff floor enforced from the first day, because the floor
is cheap now and embarrassing to retrofit. Treat Phase B as
the follow-on once real hymn files exist in hand, parsed by
our own Rust MusicXML reader, with the helper demoted to
pure rendering. Do not build readers for `.sib`, `.musx`,
or `.dorico`; do not bundle MuseScore or LilyPond; keep
the LibreOffice-style bridge idea (an installed MuseScore
rendering pages) in the drawer only if Verovio's importer
fails real files.

### Kill criteria

Stop or demote this workstream if any of these prove true:

1. **No input.** The church's music people cannot or will
   not export MusicXML (locked Sibelius tiers, lost source
   files), and a season passes with zero real `.musicxml`
   or `.mxl` files in hand. Native support with no native
   files is a demo, not a feature; Phase A placement of
   engraver PDFs is all that survives.
2. **Importer fidelity.** On a test set of real hymn files,
   Verovio's MusicXML import loses notes or lyric lines in
   ways the review labels cannot make safe (our bar from
   the PDF spike applies: losses must be detected and
   labeled, never silent). If two of the first five real
   files fail that bar, stop and reassess the engine before
   building Phase B on it.
3. **The floor does not fit.** If typical four-part hymns
   cannot be laid out on the 5.5-inch panel at or above
   5.0 mm staff height with acceptable systems per panel,
   the honest answer is that this booklet shape cannot
   carry native hymnody, and music stays a placed-PDF
   feature. Measure this in the Phase A spike, before the
   UI is built.
4. **Packaging.** If the Verovio helper cannot be built and
   staged into the one-download Windows bundle the way
   PDFium is, the feature does not ship in a form that
   needs a toolchain at the church.
5. **License friction.** If bundling the unmodified LGPL
   helper with its notices and source offer cannot be kept
   clean in our release process, demote to the bridge
   pattern (user-installed tool) or stop. Our own code's
   license is never negotiated to accommodate an engine.

## Sources

Formats and interchange:

- MusicXML 4.0, W3C Music Notation Community Group:
  https://www.w3.org/2021/06/musicxml40/ and
  https://en.wikipedia.org/wiki/MusicXML
- MakeMusic ends Finale (Aug 26, 2024), Dorico partnership,
  MusicXML migration path:
  https://en.wikipedia.org/wiki/Finale_(scorewriter) and
  https://cdm.link/finale-music-notation-software-discontinued-devs-embrace-dorico/
- Hahn et al., Comparison of MusicXML export capabilities
  of different scorewriters (poster, Zenodo):
  https://zenodo.org/records/8356336/files/MusicXMLExportComparisonPoster.pdf?download=1
- Sibelius 7 launch, built-in MusicXML export vs Dolet,
  Scoring Notes: https://www.scoringnotes.com/news/sibelius-7-is-here/
- How to make MusicXML in each program (CPDL):
  https://www.cpdl.org/wiki/index.php/Help:Format_choice

Native files:

- Sibelius format record (encryption, versions), Just
  Solve the File Format Problem:
  http://justsolve.archiveteam.org/wiki/Sibelius
- MuseScore forum, ".sib is proprietary and closed":
  https://musescore.org/en/node/91181
- MuseScore Handbook, file formats (.mscz is a zip of
  .mscx): https://musescore.org/en/node/456
- Finale files without Finale (Denigma, format not
  documented, Michael Good quotation), Steinberg forum:
  https://forums.steinberg.net/t/converting-mus-or-musx-files-to-xml-without-finale/1017042
- Maestro opens Finale files; Denigma background,
  Scoring Notes:
  https://www.scoringnotes.com/news/maestro-opens-finale-files-without-finale/
- musx2mxl converter ("basic notations" only):
  https://github.com/mouradaniel/musx2mxl

Engines:

- Verovio project (LGPL, C++, SVG, converters):
  https://pypi.org/project/verovio/5.7.0/ and
  https://github.com/rettinghaus/verovio
- Verovio input formats, MusicXML converters:
  https://book.verovio.org/toolkit-reference/input-formats.html
- Verovio MusicXML converter ("still under development,
  capabilities are limited"): https://www.verovio.org/musicxml.html
- Verovio toolkit options (breaks, scale, spacing,
  lyricSize, fonts):
  https://book.verovio.org/toolkitreference/toolkitoptions.html
- MuseScore license (GPL 3.0) and features:
  https://github.com/LaffeyOvO/MuseScore (upstream mirror
  of musescore/MuseScore README and LICENSE.GPL)
- MuseScore as a conversion service via CLI (forum):
  https://musescore.org/en/node/242946
- musicxml2ly manual page (conversion switches):
  https://manpages.debian.org/testing/lilypond/musicxml2ly.1.en.html
- LilyPond as a GPL command-based typesetter (ConTeXt
  journal): https://articles.contextgarden.net/journal/2016/70-74.pdf
- VexFlow (MIT): https://vexflow.com
- OpenSheetMusicDisplay (MIT, VexFlow-based):
  https://opensheetmusicdisplay.org/blog/hello-world-opensheetmusicdisplay-musicxml/
- Rust crates checked on crates.io 2026-10-15: `musicxml`
  1.1.2, `verovio` 0.3.6 with `verovio-sys` and
  `verovio-data`, `verovioxide` 0.3.0. No Rust VexFlow; no
  native Rust engraver of consequence.

Legibility:

- Indiana University Jacobs School, Music Notation Style
  Guide (MOLA sizes: parts 7.0 to 7.5 mm, score minimum
  4 mm): https://blogs.iu.edu/jsomcomposition/music-notation-style-guide/
- Universal Edition, Product Design Standards (parts
  7.5 mm; choral, piano, sales editions 7 mm):
  https://notat.io/download/file.php?id=916&sid=c6b56e9c0e143fece493f3c8054fc441
- Engraving practice handout (piano/vocal 7 mm, choral
  5 mm on letter paper):
  https://mostlymodernfestival.org/wp-content/uploads/sites/2/2025/12/MMP-MMF-EngravingPresentationHandout.pdf

Corpus method: structural pass over the 17 fixture PDFs
with pypdf (fonts, image inventory, drawn sizes, producer
strings), 2026-10-15. Aggregates only; no fixture content
is reproduced here, and no fixture file enters the repo.
