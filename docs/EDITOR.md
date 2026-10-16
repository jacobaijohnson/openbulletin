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

## Picture boxes (built 2026-10-16, G2)

- "Add a picture box" puts an empty box mid-page: dashed
  outline, the words "No picture yet". A picture box is a
  frame: it moves, resizes, snaps, and takes exact numbers
  like every other box. It holds a picture or nothing,
  never words.
- "Picture..." (any box selected) opens the put-a-picture
  window: one file door, plus "Pictures from the old
  bulletin", the pictures a rescue carried that no box
  uses yet. "Clip art..." opens the clip library window:
  collections, name search, thumbnails; a clip goes into
  the selected box, and its bytes copy into the bulletin,
  so the bulletin always travels complete.
- A picture shows one of two ways, in the right-click
  menu's own words: "Fill the box (sides may be trimmed)"
  or "Show the whole picture". "Take the picture out"
  empties the box; the box stays.
- Fill is direct manipulation: with the box already
  selected, drag INSIDE it and the picture follows the
  pointer (the crop is stored as fractions of the trimmed
  overflow, so it survives resizing the box).
  Double-click centers the picture again. The box itself
  still moves by its edges, its corner, the arrow keys,
  and the right-click numbers.
- One geometry function in the model decides the shown
  picture; the canvas, the previews, the print writer,
  and the Check blur finding all read it, so no two views
  can disagree about the crop.
- Check speaks for pictures: an empty box, a picture
  missing from the bulletin, and a picture under 120
  dots per inch at its shown size each get a finding
  with a working Fix-this target.

## Typing on the page, and Undo (built 2026-10-15)

- The canvas draws every frame's text, shaped by the same code
  path as the print preview, so the page on screen and the sheet
  from the printer cannot drift apart.
- **Double-click a frame to type in it**, right where it sits.
  Enter adds a paragraph; new paragraphs inherit the style of
  the one above. A small bar above the page names the box and
  offers the current paragraph's style (Heading, Body, Scripture,
  and friends). Done or clicking another part of the page
  finishes and saves; Escape throws the typing away (the
  office convention: Escape in a text box means cancel). Since
  the lens review (2026-10-15), thrown-away typing that
  differed from what was there is kept: the status line offers
  "Bring the typed words back", reopening the box with them.
  Cancel still cancels; it no longer destroys.
- **Typing shows the page as it will print** (E1, 2026-10-15).
  While a box is being edited, its words stay in their real
  typefaces and sizes, break lines exactly where the idle page
  and the PDF break them, and tabbed lines (a hymn name and its
  number) show their row of dots. Words that fill themselves in
  from the church and the Sunday (the pastor's name, the date)
  appear filled in and shaded, as one unbreakable piece: the
  caret steps over the whole piece in one move, Backspace next
  to one selects it whole and a second press removes it, and
  typing over a selected piece replaces it. They cannot be
  typed inside. The bar above the page says where those words
  are changed (the Start and This Sunday steps). Composition
  input (IME, for languages that compose characters) works:
  the composition previews at the caret underlined and lands
  as ordinary text when it commits. Two recorded limits: text
  that overflows the bottom of its box is not drawn while
  typing (the Check step's overflow finding still names it;
  the idle page clips the same way), and Ctrl+Z inside a
  typing session does nothing, because the session itself is
  the undo step (below). If the shaping machinery ever fails
  to lay a box out, the session falls back to the plain text
  overlay it replaced, so editing is never blocked.
- **Bold and italic inside a paragraph** (G1b, 2026-10-16).
  A paragraph's words can carry their own bold/italic on top
  of the paragraph style ("runs"): select words and press
  Ctrl+B / Ctrl+I, or use the Bold / Italic buttons in the
  bar above the page, which show the state at the caret or
  over the selection. The rule is the word-processor one: if
  every selected word already has the face, the press takes
  it off, otherwise it puts it on. (The keymap has no
  bold/italic bindings to respect; these two keys read the
  modifiers directly, and docs/MODES.md's keymap section
  says so.) Typing inherits the face in force where the
  caret sits: type after a bold word and the new words are
  bold, type after a plain word and they are plain; at the
  start of a paragraph the first word's face leads. A
  selection that covers a shaded field word toggles AROUND
  it, never inside it: the field word keeps the face it
  resolves with. Words keep their faces through the .obd
  file, the PDF, the previews, and Markdown copies. IDML
  and Scribus copies carry the words and their paragraph
  styles today; the runs inside a paragraph flatten to the
  style's face there (their exporters write per-paragraph
  styles, and teaching them runs is a later build).
  Pasting text that carries markdown
  marks (**bold**, *italic*) lands the marks as faces;
  plain paste is unchanged. If a session ever falls back to
  the plain overlay editor, words typed there come back
  plain: the faces from before the fallback are not laid
  over changed words and guessed at.
- **Undo** covers the canvas: typing sessions (one step each),
  drags, nudges (a burst counts once), right-click applies, guide
  add/move/delete, and the Next Sunday command. Ctrl+Z and the
  Undo button in the top bar; Ctrl+Y or Ctrl+Shift+Z and Redo
  put it back. The Announcements step's box is a plain text box
  and keeps its own undo while it has the keyboard, as in
  Office; the canvas editor's session is one step as a whole.
- Undo history is per session and per file opened; it is not
  saved into the bulletin.

Not yet: dragging new guides out of the rulers, master
(all-pages) guides, spell-check underlines on the canvas.

## After the audits (2026-10-15, batches A-C)

- **Zoom**: Smaller / Fit / Bigger in the Pages window; the page
  scrolls when it outgrows the window.
- **Boxes are born and die here**: "Add a text box" (middle of
  the page, its own words), and a box's right-click menu offers
  Copy and Delete. Ctrl+C / Ctrl+V do the same from the keys.
  A paste gets its own copy of the words. Undo covers all of it.
- **Guide lock**: a guide's menu can lock it (gray, undraggable)
  or unlock it. Its exact place can still be typed.
- **Keyboard**: Tab cycles boxes, Enter types in the selected
  box, arrows nudge, Shift+arrows stride, Ctrl+Z/Y undo/redo.
- **Escape cancels typing** (the office convention); Done or
  clicking away saves.

## Interaction vocabulary (integrated 2026-10-15)

The canvas borrows its habits from the two programs the office
worker's hands already know, filtered by PRINCIPLES.md:

- **Frames are the atom of the page** (InDesign). Boxes hold
  text and pictures; the word "frame" stays in this document
  and the code, never on the guided path (Principle 3 says
  "boxes" and "panels").
- **One pointer, not two.** InDesign splits Selection and
  Direct Selection; administrators get one pointer that does
  the obvious thing (click selects the box, drag moves it,
  double-click types in it), and the right-click menu carries
  everything else, in inches.
- **Threading is made visible when the editor grows it.** A
  story flowing from one box into another shows its link on
  the canvas, and overflow is a mark on the box itself, never
  a surprise at the printer. The .pub reader already honors
  threading on import; the editor owes the same honesty.
- **Smart guides grow from what shipped.** Boxes already snap
  to guides, edges, and centers; frame-to-frame snapping joins
  in the editor's next phase, with the same live inch readout.
- **Grab the thing itself** (Apple's Direct Manipulation).
  Every canvas operation starts from the object under the
  pointer. Dialogs that ask for instructions about an object
  the user could be holding are a recorded debt.
- **What we leave behind:** layers panels, effects, and the
  rest of the professional depth. A bulletin that needs them
  belongs in the export-to-InDesign door, not in this app.
