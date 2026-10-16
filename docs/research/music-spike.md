# M0 music spike: can Verovio put a real hymn on the bulletin panel at a legible staff size?

Date: 2026-10-15. Docs-only spike, run against the test in
`docs/research/music-landscape.md` sections 4-5 (the legibility floor and
the kill criteria). No app code was touched.

Short answer: yes. A real four-part hymn with two verses of lyrics fits
the 5.5 x 8.5 inch panel at 6.0-6.5 mm staff height, inside the target
band, with every note, syllable, and fermata accounted for in the output.
The one naive way of doing it (render at A4 and shrink) lands at 4.79 mm,
below the 5.0 mm floor, which is exactly the failure the floor exists to
refuse. Recommendation: GO for M1, with the conditions at the bottom.

## The fixture

"Ein' feste Burg ist unser Gott" (A Mighty Fortress Is Our God), the
four-part chorale setting by J. S. Bach (the file's own metadata says
BWV 302), text by Martin Luther. Public domain work; the digital edition
used is CC BY-NC-SA 4.0, so the files stay in the fixtures directory
outside this repo and never ship. Full provenance:
`~/workspace/openbulletin-fixtures/music-spike/PROVENANCE.md`.

Getting one real hymn took four tries, worth recording: Hymnary's
MusicXML downloads sit behind an anti-bot shield, CPDL returns
Cloudflare 403 to this sandbox, and the Greentree Bach chorales (404
MusicXML files) contain no lyrics at all. The working source was Zenodo
record 17236070, "389 Chorale Settings by J. S. Bach", whose MuseScore 3
files do carry lyrics. MuseScore 3.6.2 converted the vocal-parts-only
file to MusicXML 3.1. That is the brief's allowed fallback: a
MuseScore-exported MusicXML from a public score library with a clear
license.

The hymn as data: 2 staves (Soprano/Alto, Tenor/Bass), 2 voices per
staff, 14 measures, 237 notes (134 quarter, 92 eighth, 6 sixteenth,
5 half), 13 fermatas, key D major, 4/4, lyrics for 2 verses
(58 syllables). The source file's own scaling says 6.4 mm staff;
Verovio does not use that value, it uses its own notation size (below).

## How Verovio's sizes work (measured, not assumed)

Verovio 6.3.0, from the PyPI wheel, in a throwaway venv.

- The Python `setOptions` call takes a dict. Passing a JSON string
  raises "Cannot parse JSON" on stderr and silently keeps the defaults.
  Any helper we ship must check that options actually took.
- The SVG root carries `width`/`height` in the page units you asked
  for; the drawing itself lives in a nested
  `svg.definition-scale` whose viewBox is 10x the page units. Call
  those grid units.
- Staff lines are plain horizontal paths inside `g.staff` groups, five
  per staff, spaced 180 grid units apart. Staff height is therefore
  720 grid units, constant at Verovio's default notation size,
  confirmed identical (min = max = median) across all 28 staff groups
  in every render.
- So the conversion is: staff_mm = 72 x panel_width_mm / pageWidth_units.
  To place music on a 139.7 mm panel at a chosen staff size, set
  pageWidth to 72 x 139.7 / staff_mm and pageHeight in the same ratio.
  The helper picks the staff size; Verovio's page units follow. Every
  target below measured back exact (5.00, 6.00, 6.50, 7.20).

## Measurements: the hymn on a 5.5 x 8.5 panel, 5 mm margins

| Geometry | Staff | Panels needed | Systems per panel | Lyric type |
|---|---|---|---|---|
| Naive: A4 geometry (2100 units) shown at panel width | 4.79 mm | 1 | 3 | 2.69 mm, about 7.6 pt |
| Floor: 5.0 mm (2012 units) | 5.00 mm | 1 | 3 | 2.81 mm, about 8.0 pt |
| 6.0 mm (1676 units) | 6.00 mm | 2 | 3 + 1 | 3.38 mm, about 9.6 pt |
| 6.5 mm (1547 units) | 6.50 mm | 2 | 3 + 1 | 3.66 mm, about 10.4 pt |
| 7.2 mm (1397 units, Verovio's natural size) | 7.20 mm | 2 | 2 + 3 | 4.05 mm, about 11.5 pt |

Reading the table:

- At the target band (6.0-7.0 mm), a 14-measure hymn with two verse
  lines takes two panels: three systems on the first, the rest on the
  next. On a half-letter booklet that is one sheet side, or a page
  turn, which is normal hymn practice.
- At the 5.0 mm floor it fits one panel in three systems. The floor is
  a real option, not just a refusal line.
- The naive render also fits one panel, at 4.79 mm, with 7.6 pt
  lyrics. Nothing about it looks broken until you measure it. This is
  why the floor has to be enforced by measurement in the helper, not
  by trusting the render.
- Lyric size rides with the staff (about 0.56x staff height in all
  renders), so no separate lyric floor was needed for this file,
  matching the landscape doc's expectation.
- System spacing at 6.5 mm: ink height 34.5-39.1 mm per system (two
  staves, two lyric lines), system pitch about 65 mm, roughly 10 staff
  heights, because Verovio's defaults justify systems vertically to
  fill the page. In a bulletin panel we would cap that spread rather
  than stretch; the binding number is systems per panel, above.

Caveat: this hymn carries two verse lines. A hymn printed with three
or four verse lines under one staff is taller per system and will fit
fewer systems per panel. The designed answer stays the same: more
panels, never a smaller staff.

## Fidelity: what the importer kept, and how we know

Kill criterion 2 asks whether losses are detected or silent. Verovio
logged nothing at all during load and render (stderr empty), so the
renderer's own log cannot be the detection mechanism. Structural
counting can, and it is cheap. Source MusicXML counted against the
6.5 mm SVG:

- Notes: 237 in, 237 noteheads out. 232 filled heads for the quarter,
  eighth, and sixteenth notes; the 5 half notes render with a distinct
  open notehead glyph. Note values survive.
- Lyrics: 58 syllables in, 58 out, and the full syllable stream is
  identical in order and text on both sides, both verses.
- Fermatas: 13 in, 13 out.
- Measures: 14 in, 14 measure groups out.
- Key (D major, two sharps) and 4/4 time signatures render on both
  staves.
- Not rendered: the file's title, movement title, and composer credit
  lines. Two causes, both detectable: the spike set `header: "none"`
  on purpose, and Verovio does not draw MusicXML credit furniture.
  In production the bulletin supplies its own headings, but Phase A
  must still say so in the review labels ("the file's title and
  credits were not carried over"), not let it pass silently.

Verdict on criterion 2 for this file: no silent loss in the music
itself; every counted thing in the source has a counted counterpart in
the output, and the one loss class (page furniture) is visible to the
same counting. The criterion's real form (two of the first five real
files failing) can only be tested as real church files arrive; M1
should run this same count comparison on every import and label the
result.

## Packaging smell (criterion 4)

The wheel installs into a 44 MB venv; the native library
(`_verovio.abi3.so`) is about 17 MB, plus SMuFL font data (Bravura,
Leipzig, Petaluma). It ran headless in this sandbox with no system
dependencies to install. That is the same shape as the PDFium helper:
one self-contained native library beside the app. The shipped form (a
small helper binary driving the library) is still an M1 build
question, but nothing in the spike blocks it. Verovio is LGPL-3.0;
the landscape doc already plans for bundling the unmodified helper
with notices and a source offer.

## Verdicts against the kill criteria

- Criterion 3, the floor does not fit: NOT triggered. Typical
  four-part hymnody lives on the panel at 6.0-6.5 mm with three
  systems on a panel. (a) Floor fit: PASS.
- Criterion 2, importer fidelity: PASS on this file, with the
  detection method proven (structural counts, not the renderer log)
  and one labeled loss class (title/credits). (b) Losses detected,
  not silent: PASS, provided M1 builds the counting in.
- Criterion 4, packaging: PASS on size and standalone running.
- Criteria 1 (no real files ever arrive) and 5 (license friction)
  are not spike questions; they stay open by design.

## Recommendation: GO for M1 (Phase A), with these conditions

1. The helper drives Verovio by target staff size using the measured
   formula (72 x width_mm / staff_mm page units), defaults to 6.5 mm
   on the half-letter panel, and refuses below the 5.0 mm floor
   (0.02 mm of measurement slack) in plain words.
2. Every music import runs the structural count comparison (notes by
   value class, syllables per verse, measures, fermatas) and the
   review labels show the result. A mismatch is a "Check this" label,
   never a silent pass.
3. Title and credit lines from the file are a named, labeled loss
   until we deliberately render them.
4. System spacing gets a cap for bulletin panels instead of Verovio's
   fill-the-page justification.
5. The `setOptions` dict-not-JSON gotcha and the options-took check
   go into the helper from day one.

Fixture, renders, and provenance live outside the repo at
`~/workspace/openbulletin-fixtures/music-spike/`.
