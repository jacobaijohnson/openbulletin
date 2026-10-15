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

Not yet: typing text on the page itself, dragging new guides out
of the rulers, master (all-pages) guides. Those come with the
inline editing layer.
