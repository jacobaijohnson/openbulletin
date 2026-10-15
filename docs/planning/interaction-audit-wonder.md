# Interaction Structure Audit — Muse Wonder pass

Auditor: Muse Wonder (internal model, external-auditor posture), 2026-10-15.
Evidence: the audit brief only — PRINCIPLES.md, EDITOR.md, FORMAT.md,
`crates/ob-model/src/lib.rs`, and `crates/ob-app/src/main.rs` in full.
No GUI was run (none has ever been run). Every finding quotes the code it
rests on. Classes: **CORRECTNESS** (can corrupt state or lose work),
**ROBUSTNESS** (edge cases, fragility), **DESIGN DEBT** (works now, blocks
what comes next).

## Verdict

The interaction layer is a coherent, thoughtfully-grown prototype whose
safety story (autosave journal, recovery window, snapshot undo) is stronger
than its interaction discipline: undo, selection, and the text-edit session
are all keyed to a document that several operations replace wholesale, and
none of those operations reset or re-validate the interaction state. Two
reachable paths lose user work outright, one reachable path can write the
wrong document into the wrong file, and one reachable path panics. The
documented contracts in EDITOR.md ("Undo history is … per file opened",
"Right-click a guide for its menu") are not true of the code. Nothing here
is hard to fix, but the fixes are structural, not cosmetic: one transaction
path for mutation, one session-reset discipline, one hit-test table.

## 1. The architecture as built

**One funnel.** Every canvas gesture passes through a single egui widget:

```rust
let response = ui.interact(
    panel_rect,
    egui::Id::new("panel-canvas"),
    egui::Sense::click_and_drag(),
);
```

All hit-testing after that is hand-rolled math against
`response.hover_pos().or(self.ctx_pos)`, via two closures (`near_guide`,
`frame_hit`/`to_mpt`) rebuilt per frame. This is a legitimate minimal
architecture — but priority between competing targets is decided by the
*order code happens to run*, separately in three places (drag, click,
context menu), and the three orders do not agree (see C5).

**Three undo idioms.** (a) Gesture snapshots: `begin_gesture()` /
`end_gesture()`, closed by `drag_stopped` or by a 1,200 ms idle timer in
`update()`. (b) `push_undo_now()` for one-shot menu applies, correctly
called *before* mutation. (c) A private `edit_start` snapshot for text
sessions, recorded in `commit_edit()`. There is no single entry point a
mutation is required to pass through — which is exactly why coverage has
holes (C4, D1).

**Snapshot undo.** Each step is a full-document `serde_json` string
(`fn snapshot(doc: &Document) -> String`). Total and simple — no per-op
inverse logic to get wrong — at the cost of full-document serialization on
hot paths (R6).

**Index-based interaction state.** Roughly 25 interaction fields sit flat
on `App`: `selected_frame: Option<usize>`, `drag_frame`,
`editing: Option<(usize, usize)>`, `guide_menu_for`, `frame_menu_for`,
`canvas_panel`. Guides have no identity in the model at all
(`pub struct Guide { pub vertical: bool, pub pos: Mpt }`). Indices are
interpreted against whatever document and panel happen to be current when
the code runs (C3).

**Focus tri-regime.** (1) egui widget focus (`ctx.memory().focused()`),
(2) the canvas overlay's hard-coded `egui::Id::new("canvas-edit")` with a
manual `focus_edit` request flag, (3) global shortcuts in `update()`
gated on regimes 1 and 2. The gates are not applied uniformly (C6, R8).

**The edit session.** `editing + edit_text + edit_styles + edit_start +
edit_done + focus_edit` form an implicit state machine. Entry is
double-click only. Exit paths are: Done button (sets `edit_done`),
Escape, press-outside-frame (via `drag_started`), and panel switch.
There is no cancel path, and two exits are missing entirely (C2, R1).

## 2. CORRECTNESS findings

### C1. Undo crosses document replacement — and a later Save can overwrite the wrong file
Neither opening a file nor rebuilding from a preset touches the undo
stacks. The Open handler is:

```rust
self.doc = doc;
self.path = Some(path);
self.step = 1;
self.status = "Bulletin opened.".into();
self.mark_clean();
```

`rebuild()` (preset change) likewise replaces `self.doc` wholesale with no
stack handling. So: edit document A (stack fills), open document B,
press Ctrl+Z — `undo()` pops an A-state and installs it as `self.doc`,
while `self.path` still names B. The autosave journal then records A's
content against B's path, and **Save bulletin writes A's content into
B's file**. The recovery bring-back path has the same shape. This also
falsifies EDITOR.md's own contract: "Undo history is per session and
per file opened." It is per session only.

### C2. Double-clicking another frame while typing silently discards the text
The double-click handler runs *before* the editing-exit block in
`panel_canvas`, and calls `enter_edit` unconditionally:

```rust
if response.double_clicked() {
    ...
    self.enter_edit(idx, fi);
}
```

`enter_edit` overwrites the session without committing it:

```rust
self.edit_start = Some(snapshot(&self.doc));
self.edit_text = story.paragraphs.iter()...join("\n");
```

If the admin is mid-edit in frame A and double-clicks frame B, A's buffer
is never written to the document and no undo entry is created for it —
the typed words are simply gone. The code *does* commit on panel switch
("if self.editing.map(|(pi, _)| pi) != Some(idx) … self.commit_edit()"),
which makes the missing frame-switch commit an inconsistency, not a
philosophy. Principle 6 ("never lose work") is violated by the flagship
feature built the same day.

### C3. Interaction state is index-based and never invalidated on document swap
`selected_frame` carries no panel qualifier: it is interpreted against
whichever panel is on the canvas (`panel.frames.get(fi)` in paint, and
direct indexing elsewhere). Paging from Cover to Inside silently
"selects" the same-indexed frame there, and arrow keys will nudge a
frame the user never selected.

Worse, several sites index directly instead of using `.get()`:

```rust
// nudge handler
let f = &mut self.doc.panels[idx].frames[fi];
// commit_edit
let Some(frame) = self.doc.panels[pi].frames.get(fi) else { ... }
```

(`commit_edit` guards the frame but indexes `panels[pi]` directly.)
Reachable panic: select the last frame of a frame-rich panel, open a
bulletin whose same panel has fewer frames, open Pages, press an arrow
key — index out of bounds, app crashes. The autosave journal makes this
recoverable rather than fatal, but a crash on an ordinary sequence is a
correctness defect. Root cause is singular: no document-swap hook resets
or re-validates `selected_frame`, `editing`, `drag_*`, `*_menu_for`.

### C4. Undo does not cover the primary editing surfaces, and undo leaves a stale name buffer
Undo covers canvas gestures and canvas typing. It does **not** cover:
the Announcements box (the main weekly text flow — `self.announcements`
is a side buffer folded into the doc by `journal_tick()` every ~2 s via
`save_announcements()`, with no undo record anywhere), nor the Step 1/2
fields, which mutate the doc directly on focus loss:

```rust
if ui.text_edit_singleline(&mut sn).lost_focus() {
    self.doc.fields.insert("sunday_name".to_string(), sn);
}
```

A user who learns "Ctrl+Z fixes mistakes" from the canvas will find it
does nothing for the text they edit most. Separately, `undo()` resyncs
the announcements buffer (`self.announcements = weekly_body_text(...)`)
but **not** `self.church_name`, which is a free-standing buffer written
back on focus loss — undo a state with a different church name, later
tab through the Step 1 field, and the stale buffer silently overwrites
the restored name.

### C5. The guide context menu is unreachable wherever a frame covers the guide
Precedence disagrees between gestures. Dragging prefers guides:

```rust
self.drag_guide = near_guide(pos);
if self.drag_guide.is_none() { ... frame ... }
```

The context menu prefers frames:

```rust
if let Some(fi) = frame_at_menu {
    ...
} else if let Some(gi) = hit {
```

Guides are full-length lines; in real templates frames tile most of the
panel, so most of a guide's length opens the *frame* menu. EDITOR.md
promises "Right-click a guide for its menu: an exact position box" —
for the covered majority of any guide, exact-position entry and Delete
are simply unavailable. (The drag path still works everywhere.)

### C6. The top-bar Undo/Redo buttons bypass the focus and editing gates
Keyboard undo in `update()` is carefully gated:

```rust
if ctx.memory(|m| m.focused()).is_none() && self.editing.is_none() {
```

The top-bar buttons call `self.undo()` directly, gated only on stack
non-emptiness. Clicking Undo mid-edit swaps `self.doc` under the live
session; `commit_edit()` then writes the buffer into the restored
document's story *by id* — and template story ids are reused across
document generations, so the text can land in a document the user is
not looking at. Two triggers for one operation must not have two
different safety predicates.

## 3. ROBUSTNESS findings

### R1. Closing the Pages window mid-edit orphans the session and kills global undo
`pages_window` early-returns when `!self.show_pages`, and the window's
close box only flips that flag. Nothing commits or clears `editing`.
Consequences while the zombie session persists: the global-undo gate
`self.editing.is_none()` is false forever, so Ctrl+Z silently stops
working app-wide; the buffer is neither in the document nor reachable
until the window reopens (where, to its credit, the session resumes or
commits on panel mismatch — one frame later than the user expects).

### R2. Paragraph styles are tracked by index; insert/delete shifts them onto the wrong text
`sync_edit_styles()` keeps a `Vec<String>` parallel to the buffer's
line count, growing by "inherit the last" and truncating. It cannot
tell an inserted line from a shifted one. Delete the heading line of a
story and the following body paragraph commits with the heading style;
insert a line at the top and every style below lands one paragraph off.
EDITOR.md documents only the happy path ("new paragraphs inherit the
style of the one above"); the boundary cases violate even that, silently.

### R3. Hit tests act on a stale pointer fallback
```rust
let pointer = response.hover_pos().or(self.ctx_pos);
```
`ctx_pos` is refreshed only while the canvas is hovered, yet it feeds
drag start, double-click, click-selection, the context-menu hit tests,
and the "Add guide here" position. Any path where `hover_pos()` is
`None` on the decisive frame acts on where the pointer *was*, not where
it is. Low probability per event, wrong in a way users describe as
"the app clicked the wrong thing."

### R4. One shared editor Id leaks cursor and internal undo state across sessions
The overlay is always `egui::Id::new("canvas-edit")`, whatever frame or
story is being edited. egui keeps per-Id `TextEditState` — cursor,
selection, and the widget's own undo history. `cursor_paragraph()` reads
that state back for the style picker, so a session can open with the
previous story's cursor (the `edit_styles.get(para)` fallback limits the
blast radius to a wrong initial style display), and the TextEdit's
internal Ctrl+Z history is not scoped to the current story. Verify
against the pinned egui version and, if confirmed, salt the Id per
edit session.

### R5. Menu input buffers are keyed by index identity
`guide_menu_for`/`frame_menu_for` decide whether to (re)initialise the
input fields by comparing a bare index. Delete guide 0 and the former
guide 1 becomes index 0; if `guide_menu_for == Some(0)` survives, the
menu shows the new guide's label with the *deleted* guide's number in
the box, and Set applies it to the wrong guide. Same family as C3,
smaller blast radius.

### R6. Full-document JSON serialization runs on the per-frame path
Change detection for both caches is a whole-document serialize:
`preview_sheets()` and `shaped_panel_lines()` each begin with
`let key = snapshot(&self.doc);` and are called every frame their view
is visible. For booklet-sized documents this is survivable; it scales
with total text, not with what changed, and it will be the first thing
to jank on a big imported document. A monotonically increasing document
generation counter (bumped by the transaction path in D1) replaces all
of it. `record_undo`'s `self.undo_stack.remove(0)` is the same pattern
in miniature — harmless at 60 entries, noted only for consistency.

### R7. A missing tracing picture fails silently and is never retried
```rust
if self.bg_tex_key != key {
    self.bg_tex_key = key.clone();
    self.bg_tex = self.load_bg_texture(ctx, bg);
}
```
The key is committed before the load; a missing/unreadable asset yields
`bg_tex = None` forever (until path or filename changes) and no message.
For a feature whose entire purpose is "rebuild over the old page," a
blank panel with no explanation contradicts the rescue-honesty
principle. One plain-words line on the canvas ("We couldn't find the
picture for this page") fixes the honesty; retry-on-open fixes the rest.

### R8. Focus dead zone while editing
While `editing.is_some()` but focus sits outside the overlay (e.g. in
the style ComboBox), Ctrl+Z does nothing at all: the global handler is
gated off by `editing.is_none()`, and the TextEdit's own undo only
exists while it holds focus. Dead keys in exactly the moment a user is
most likely to reach for undo.

### R9. No zoom or pan on the canvas
Scale is fit-to-window, `min(2.0)`. On a tabloid panel the shaped text
floors at `max(4.0)` px — decoration, not reading — and precise guide
work happens at whatever magnification the window dictates. The 5 px
guide-grab band is screen-constant (good), but everything else about
precision is at the mercy of panel size. Publisher users will reach
for zoom that isn't there.

### R10. The overlay is registered after the canvas interact — gates, not layers, keep them apart
`panel_canvas` creates the canvas `interact` first and `ui.put`s the
TextEdit later in the same pass. Pointer presses inside the editor are
therefore also visible to the canvas handlers; today's behaviour is
correct only because the `dragged()` path and the exit-check explicitly
test `editing`. That is a load-bearing comment's worth of fragility:
the next handler added without the gate inherits drags that start
inside text edits.

### R11. Escape is read globally during editing — including during IME composition
The exit check is `ui.input(|i| i.key_pressed(egui::Key::Escape))`,
unconditional within the session. During an IME composition in the
overlay, Escape conventionally cancels the *composition*; here it also
commits the whole session. Edge-case, but the user population pasting
and typing multilingual text (congregation names, Spanish-language
ministries) is real for this product.

## 4. DESIGN DEBT findings

### D1. No transaction layer for document mutation (root cause of C4)
Mutations are scattered through paint/layout code — `save_announcements()`
is even called from inside the Print step's render block. Each call site
must individually remember undo, journal, and cache implications; the
coverage gaps in C4 are what that discipline debt looks like. Every
future feature multiplies it.

### D2. No frame lifecycle, no multi-object anything
The canvas can move and resize the frames templates give it, but cannot
add, delete, duplicate, reorder (z-order), or multi-select them; there
is no marquee, no alignment among frames, and snapping targets are
guides/edges/center only — never other frames' edges, which is the
common alignment case in real layouts. For a template-driven v1 this is
defensible; it is still the ceiling on the editor's vocabulary, and the
data structures (single index selection) preclude growing it in place.

### D3. The text overlay is not the text
The overlay is one `egui::FontId::proportional(first_size)` where
`first_size` is the *first* paragraph's size clamped to 10–20 px. It
uses egui's default family — not the style's library face, though those
faces are registered in egui under `lib-<sha>` names for the Fonts
panel. Mixed-style stories (26 pt church name over 11 pt body), center/
right alignment, and tab-leader order lines all edit as uniform
left-aligned text, wrapped by egui inside the widget's own padding
rather than by the shaper at the frame width. Text visibly reflows the
moment Done is pressed. EDITOR.md's promise that screen and sheet
"cannot drift apart" is true of the *displayed* lines and false of the
*editing* surface. A truthful editor (per-paragraph shaped editing, or a
layout-driven caret over shaped lines) is the largest single piece of
structured work remaining.

### D4. Three panel renderers, two fidelities
The print sheet preview and the canvas share `panel_lines` (good).
The central "what the congregation will hold" preview is a third,
independent renderer: no wrapping, tabs flattened, text truncated —
`text.chars().take(64)`. The surface a user reads most is the one that
can disagree with print. Either route it through `panel_lines` or label
it as the sketch it is.

### D5. Guides have no identity and no states
Model `Guide` is two fields. There is no id (see R5), no lock, no
visibility toggle, no master/per-panel distinction (already listed as
"not yet" in EDITOR.md). Every guide feature on the roadmap needs the
identity first.

### D6. The canvas is invisible to keyboard-only and assistive use
All canvas content is custom painter output: no focus ring, no Tab
traversal among frames, no keyboard path to select/edit a guide, and
no semantic or textual mirror of the page for a screen reader (egui's
AccessKit support gets nothing to work with here). The guided *steps*
are ordinary widgets and fare better; the editor is mouse-only by
construction. For a church office this is a later item — volunteers
cover for each other in person — but it must be on the list before the
canvas becomes the primary surface, which is the stated direction.

### D7. The interaction layer is the untested layer
The suite covers the model (`snap_to`, `parse_inches`), settings,
journal, and the print checklist. Nothing covers hit-testing, the drag
reducer logic, gesture/undo coalescing, `enter_edit`/`commit_edit`
transitions, or menu applies — those live fused inside a ~300-line
paint method and are unreachable by tests as written. Given the brief's
fact that no human has run the GUI, the highest-risk code in the
project is also its least-examined. Two test layers fit:
(a) extract the pure core — hit-test, drag-step, session transitions —
as functions over plain state and unit-test them; (b) drive the real
`update()` headlessly with synthetic `egui::RawInput` pointer/key
events and assert document mutations (golden-path integration tests).

## 5. Structured work still needed (summary map)

- **Session discipline**: a document-instance boundary that resets
  undo/redo and validates or clears every index-based interaction
  field. (C1, C3, R1, R5)
- **A transaction API** for document mutation that owns snapshot,
  undo recording, and cache generation. (C4, D1, R6)
- **An explicit edit-session state machine** with commit/cancel on
  every exit, including frame switch and window close. (C2, R1)
- **One hit-test function, one precedence table**, shared by drag,
  click, double-click, and menus. (C5, R3)
- **One undo-trigger predicate** shared by keyboard and buttons.
  (C6, R8)
- **Paragraph identity through the edit buffer**, so styles follow
  text, not line numbers. (R2)
- **A testable interaction core + synthetic-input tests.** (D7)
- **A truthful text editor** — the long pole. (D3)
- **Frame lifecycle, multi-select, guide identity** — the editor's
  growth path. (D2, D5)
- **Keyboard operation and accessibility semantics** for the canvas,
  before it becomes the primary surface. (D6)

## 6. Ordered build list

Foundations first. **[v1]** = required before an unaided office admin
should trust it; **[later]** = scheduled growth.

1. **[v1] Document-session boundary.** On Open, preset rebuild, and
   recovery bring-back: clear undo/redo stacks, clear `editing`
   (commit first if a session is live), selection, drag and menu
   state. Stamp snapshots with a session id as a belt-and-braces
   check in `undo()`/`redo()`. (C1, C3)
2. **[v1] Transaction entry point.** One `transact(label, f)` that
   snapshots, applies, records undo, and bumps a generation counter;
   route the Step 1/2 fields, Announcements save, menus, canvas
   gestures, and Next Sunday through it. Replace snapshot-string
   cache keys with the generation counter. (C4, D1, R6)
3. **[v1] Edit-session state machine.** Make the session a single
   struct with explicit transitions; *every* exit commits or cancels,
   including double-click frame switch (commit the old session first)
   and Pages-window close. Add a visible "Throw away these changes"
   cancel in the typing bar, since Escape currently means commit.
   (C2, R1)
4. **[v1] Selection by (panel, frame), validated everywhere.**
   Replace direct indexing in nudge/drag/commit paths with a checked
   accessor; re-validate on panel switch and document change.
   (C3, R5)
5. **[v1] Single hit-test + precedence table.** One function returns
   the target for a point (guide-in-band beats frame in *all*
   gestures, menus included, or an explicit modifier for the covered
   case); delete the `ctx_pos` fallback from decision paths — use
   the event's own position. (C5, R3)
6. **[v1] One undo gate.** Extract the focus/editing predicate;
   keyboard shortcuts and top-bar buttons both consult it; buttons
   disable during a live edit session. (C6, R8)
7. **[v1] Interaction tests, layer (a).** Extract hit-test and
   drag/session reducers as pure functions over plain state; cover
   select/drag/snap/resize/commit/undo-coalescing. Then layer (b):
   synthetic-`RawInput` integration tests through `update()` for
   the golden paths. (D7)
8. **[v1] Styles follow text.** Carry per-line style identity in the
   edit buffer (or move to per-paragraph buffers) so insert/delete
   cannot re-style surviving paragraphs. (R2)
9. **[v1] Missing-picture honesty.** When a tracing asset fails to
   load, say so on the canvas in plain words and retry when the
   document path changes. Small, and principle 6/7 demand it. (R7)
10. **[later] Canvas zoom and pan; resize handles on all corners;
    frame-edge snap targets.** Publisher muscle memory, and a
    precondition for comfortable work on large panels. (R9, D2)
11. **[later] Frame lifecycle and multi-select.** Add/duplicate/
    delete frames, marquee + shift-click selection, group move,
    frame/guide clipboard; add stable ids to `Guide` (and keep
    frame ids as the selection currency). (D2, D5, D8 in §4 terms)
12. **[later] Truthful text editing.** Per-paragraph shaped editors
    (each paragraph its own shaped surface with its real face, size,
    and alignment) as the pragmatic step; a layout-driven caret over
    shaped lines as the full answer. Until then, set the overlay's
    family from the paragraph's registered library face. (D3)
13. **[later] Keyboard-only canvas operation + accessibility
    semantics.** Tab traversal of frames, keyboard guide placement,
    and an AccessKit/textual mirror of panel contents (frame names,
    positions in inches, guide list). (D6)
14. **[later] Small hardening.** Per-session editor Id (or verified
    state reset on `enter_edit`); IME-composition guard on global
    Escape; promote the central preview to `panel_lines` or label
    it a sketch. (R4, R11, D4)

## Appendix: what is genuinely good here

Fairness requires noting the load-bearing strengths, because the build
list above should not demolish them: the single-widget funnel keeps all
gestures in one auditable place; snapshot undo, whatever its coverage
gaps, cannot produce a half-applied inverse; `push_undo_now` is called
before mutation, every time; nudge coalescing via idle timeout is the
right instinct; the autosave journal + recovery window demote most
interaction bugs from "lost the bulletin" to "annoying"; menus speak
inches and accept fractions, exactly as the office-worker principle
demands; and the snap/guide maths that *is* extracted into the model
(`snap_to`, `parse_inches`) is precisely the part that has tests. The
pattern to replicate is already in the codebase — it just hasn't been
applied to the interaction layer yet.
