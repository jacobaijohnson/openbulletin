# Wave 2 synthesis, 2026-10-15 (night)

Inputs: docs/planning/completeness-audit-2026-10-15.md (code
audit, every claim cited) and docs/research/music-landscape.md
(web research, sources cited), plus Jacob's rulings of the
same evening (real typeface on canvas; graphics just work;
optimization in the PDF exports with purpose presets; music
is recognize-and-adapt, never edit).

## What the audit changed

The text pipeline is real end to end. The graphic pipeline
does not exist (audit gap #1): no picture frame kind, no
placement flow, no asset storage, decode matrix is PNG-only
for one tracing underlay that lives OUTSIDE the .obd file.
Most fidelity loss concentrates at one chokepoint: the
extraction dump schema (no alignment, italic, color,
rotation, or image channel), and the draft builder hardcodes
Left/black from each block's first line. Music today is
raster only in the corpus (measured: zero notation fonts,
316 images at ~150 effective dpi typical), so native music
is a forward feature, not a rescue upgrade.

## Build order (one writer in the tree at a time)

1. **E1, the shaped canvas editor** (audit section 9 design).
   ob-text grows a TextLayout over parley (per-cluster
   geometry, caret rect, hit testing); the canvas keeps
   drawing shaped lines during the session; the editor is a
   caret + selection + hidden focus sink, no TextEdit widget;
   field tokens render as chips (resolved text, atomic caret,
   whole-token delete, never edited in place); commit/cancel
   semantics unchanged (one undo step, Escape cancels with
   J3 recovery, edit_styles rides along). Risks recorded:
   IME composition, screen-reader access to a hand-rolled
   caret. Jacob's "real type face on canvas" ask.
2. **O1, export size** (audit section 8). FlateDecode on all
   streams; font subsetting via the `subsetter` crate
   (MIT OR Apache-2.0) honoring may_package and fsType
   no_subsetting; export purpose presets per Jacob's ruling
   (web 150 / print 300 / publication 600 / full resolution;
   imposed = print policy, reader = web policy until picture
   frames give the caps something to bite). Expected: the
   1.25 MB text-only export lands near 150-250 KB.
3. **G1, the two foundations.** Dump schema v2: alignment,
   italic, color fields + an image channel (bytes by
   reference into the asset store); draft builder stops
   hardcoding Left/black; IDML and .sla importers read
   alignment back (closing the round-trip asymmetry);
   PDF writer prints style color. AND .obd v1: zip +
   document.json + SHA-256 content-addressed assets per
   FORMAT.md, v0 files migrate on open, tracing pictures move
   inside the file.
4. **G2, picture frames.** FrameKind::Picture with an asset
   reference; place-a-picture from the canvas (right-click
   and a Start/Print-adjacent door are NOT the shape: it
   belongs on the canvas, on the selected frame, per
   direct manipulation); decode PNG/JPEG/WebP/GIF via the
   image crate's features, SVG via resvg (MPL-2.0, needs the
   LEGAL.md audit pass before it lands); fit/fill/crop in
   plain words; renders on canvas and embeds in PDF; the
   blurry-picture check joins Check using effective DPI at
   placed size (the deferred P3 item, now with a home).
5. **T1, document threading.** Frames sharing a story draw
   their own slice (continuation offsets in the model), the
   PDF writer stops printing a shared story whole in every
   frame (a latent bug the audit caught: today nothing
   creates shared stories, so it cannot fire, and it must
   never be allowed to start), the canvas shows thread links
   and the overflow mark EDITOR.md promised.
6. **M1, music Phase A** (per music-landscape.md): the one
   door recognizes .musicxml/.mxl; a Verovio helper
   (unmodified LGPL binary beside the app, PDFium pattern)
   renders SVG systems at our geometry; placed as vector
   frames that may split across pages at system boundaries;
   the 5.0 mm staff floor is enforced mechanically (render,
   measure, refuse below the floor in plain words); losses
   labeled Check this. Gated on the M0 spike below.

## M0, the music spike (runs beside E1, docs only)

Fetch one public-domain hymn in MusicXML (CPDL/Hymnary),
render it with the Verovio toolkit (PyPI wheel is fine for
the spike) at the 5.5-inch panel width, and measure: staff
height in mm at target scale, systems per panel, what the
importer drops. Verdict against kill criterion 3 (the floor
must fit typical four-part hymnody) and criterion 2 (losses
detected, not silent) before M1 is scheduled. Report to
docs/research/music-spike.md.

## Not in this wave (recorded, with reasons)

Hyphenation and justified text (audit gap #12, bulletins
live without them); multi-select and align/distribute
(gap #8, after frame-identity work); the font pin
(font_sha256) enforcement and .ttc support (gap #6, real but
below the wave line); import field-binding by pointing
(lens synthesis, unchanged); signed installers and the
pilot (adoption gates, not pipeline completeness).
