# The page canvas and guides

The editor grows toward Publisher's habits, because that is what
the office worker's hands already know (docs/PRINCIPLES.md, 9).

## Guides (v1, built 2026-10-15)

- Guides live on a panel, in the panel's own coordinates, stored
  in the document: a vertical guide is an x position, a horizontal
  guide is a y position, both in milli-points.
- The Pages window draws one panel at a time with inch rulers on
  the top and left edges (quarter-inch ticks, a number each inch),
  the panel's tracing picture underneath when it has one, its
  frames outlined, and its guides as colored lines.
- **Drag** a guide to move it. A readout shows its position while
  dragging, in inches.
- **Right-click** a guide for its menu: an exact position box
  (type 1.25, or a fraction like 1 1/4), applied on Enter, and a
  Delete button. Right-click empty page to add a vertical or
  horizontal guide exactly where you clicked.
- Units on screen are inches; the file stores milli-points. The
  conversion lives in one place and is unit-tested, fractions
  included.

## Frames on the canvas (built 2026-10-15)

- Click a frame to select it (blue outline, corner handle).
- Drag to move; it snaps to the panel's guides, the panel edges,
  and the panel center lines, by its left, center, or right edge
  (same for top, middle, bottom). The snap helper is one tested
  function in the model crate.
- Drag the corner square to resize; the moving edge snaps too.
  Smallest size is a quarter inch.
- Arrow keys nudge the selected frame a hundredth of an inch;
  Shift nudges a tenth.
- Right-click a frame: exact Left, Top, Width, Height in inches
  (fractions understood), applied together; values are clamped so
  the frame stays on the page.
- While dragging, a readout shows the frame's place and size in
  inches, live.

## Typing on the page, and Undo (built 2026-10-15)

- The canvas draws every frame's text, shaped by the same code
  path as the print preview, so the page on screen and the sheet
  from the printer cannot drift apart.
- **Double-click a frame to type in it**, right where it sits.
  Enter adds a paragraph; new paragraphs inherit the style of
  the one above. A small bar above the page names the box and
  offers the current paragraph's style (Heading, Body, Scripture,
  and friends). Done, Escape, or clicking another part of the
  page finishes; the words are in the story immediately.
- **Undo** covers the canvas: typing sessions (one step each),
  drags, nudges (a burst counts once), right-click applies, guide
  add/move/delete, and the Next Sunday command. Ctrl+Z and the
  Undo button in the top bar; Ctrl+Y or Ctrl+Shift+Z and Redo
  put it back. While a text box anywhere has the keyboard, that
  box keeps its own undo, as in Office.
- Undo history is per session and per file opened; it is not
  saved into the bulletin.

Not yet: dragging new guides out of the rulers, master
(all-pages) guides, spell-check underlines on the canvas.
