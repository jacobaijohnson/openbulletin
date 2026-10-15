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

Not yet: dragging frames themselves, snapping frames to guides,
dragging new guides out of the rulers, master (all-pages) guides.
Those come with the inline editing layer.
