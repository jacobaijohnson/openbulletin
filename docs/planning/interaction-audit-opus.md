# Interaction Architecture Audit: OpenBulletin Canvas Editor

## 1. Architecture Dissection

### Hit-Testing Order and Conflicts

The canvas uses a two-phase hit test with implicit priority:

```rust
// panel_canvas(), lines ~1120-1135
if response.drag_started() {
    if let Some(pos) = pointer {
        self.begin_gesture();
        self.drag_guide = near_guide(pos);      // ← Guides checked FIRST
        if self.drag_guide.is_none() {
            let pm = to_mpt(pos);
            match frame_hit(pm) {               // ← Frames second
```

**Conflict**: Double-click bypasses this ordering:
```rust
// line ~1101
if response.double_clicked() {
    if let Some(pos) = pointer {
        let pm = to_mpt(pos);
        if let Some(fi) = frame_hit(pm) {       // ← No guide check
```

A guide passing through a frame is draggable (guides win), but double-clicking the same point enters text edit (frames win). The inconsistency will confuse users who expect uniform behavior.

Frame hit uses `rposition` (line ~1087), correctly selecting the topmost (last-in-list) frame when frames overlap.

---

### Gesture Lifecycle and Undo Coalescing

**State machine:**
```
IDLE ──begin_gesture()──▶ ACTIVE ──end_gesture()──▶ IDLE
              │                         ▲
              │     (1200ms timeout     │
              └────────or drag_stopped)─┘
```

Gestures start explicitly on drag/nudge, end on:
1. `drag_stopped()` (line ~1200)
2. Timeout (update(), lines ~825-832): `gesture_touch.elapsed() > 1200ms && !pointer.any_down()`

**Coalescing**: Arrow nudges call `begin_gesture()` without explicit `end_gesture()`:
```rust
// lines ~1224-1231
if dx != 0 || dy != 0 {
    self.begin_gesture();
    let f = &mut self.doc.panels[idx].frames[fi];
    f.rect.x = (f.rect.x + dx).clamp(0, panel.size.w - f.rect.w);
    // ...no end_gesture()
}
```
Multiple rapid nudges become one undo step via the 1200ms timeout—correct for burst nudging.

---

### Focus Ownership

Three focus domains compete:

| Context | Owns keyboard | Undo behavior |
|---------|---------------|---------------|
| No focus (`memory.focused().is_none()`) | Arrow keys nudge, Ctrl+Z global undo | App-level |
| Canvas edit overlay (`editing.is_some()`) | TextEdit captures, Ctrl+Z is TextEdit undo | Edit session records start snapshot |
| Sidebar fields (text_edit_singleline) | Field captures, Ctrl+Z is field undo | **Not recorded in app undo** |

```rust
// lines ~836-847, global undo guard
if ctx.memory(|m| m.focused()).is_none() && self.editing.is_none() {
    let (undo, redo) = ctx.input(|i| { ... });
    if undo { self.undo(); }
```

**Gap**: Sidebar field edits (church name, Sunday name, reading) are not wrapped in `push_undo_now()`. An accidental overwrite cannot be undone.

---

### Edit-Session State Machine

```
                    double-click frame
                           │
                           ▼
    ┌──────────────────────────────────────┐
    │  enter_edit(pi, fi)                  │
    │  • edit_start = snapshot             │
    │  • editing = Some((pi, fi))          │
    │  • focus_edit = true                 │
    └──────────────────────────────────────┘
                           │
            ┌──────────────┼──────────────┐
            │              │              │
     Done button      Escape key    click outside
            │              │              │
            └──────────────┼──────────────┘
                           │
                           ▼
    ┌──────────────────────────────────────┐
    │  commit_edit()                       │
    │  • write edit_text → story           │
    │  • record_undo(edit_start)           │
    │  • editing = None                    │
    └──────────────────────────────────────┘
```

**Critical defect**: Escape commits instead of canceling:
```rust
// line ~1186
if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
    commit = true;  // ← Should be cancel
}
```

Office and Publisher convention: Escape discards, Enter/Done commits. This violates Principle 9.

---

### Selection Model

Single selection only:
```rust
selected_frame: Option<usize>,
```

Selection is set on click (line ~1207) and drag start (line ~1133). No mechanism for multi-selection (Shift+click, rubber-band).

---

### Undo Model

**Snapshot granularity**: Full JSON serialization:
```rust
fn snapshot(doc: &Document) -> String {
    serde_json::to_string(doc).unwrap_or_default()
}
```

**Stack management** (line ~722):
```rust
fn record_undo(&mut self, state: String) {
    self.undo_stack.push(state);
    if self.undo_stack.len() > 60 { self.undo_stack.remove(0); }
    self.redo_stack.clear();  // ← Any new edit invalidates redo
}
```

**Covered operations**:
- Frame drags/resizes (gesture system)
- Arrow nudges (gesture with timeout coalescing)
- Right-click exact position applies (`push_undo_now`)
- Guide add/move/delete (`push_undo_now`)
- Canvas text editing (edit_start snapshot)
- Next Sunday command (explicit begin/end_gesture)

**NOT covered**:
- Church name field (line ~935)
- Sunday name/reading fields (lines ~942, ~948)
- Announcements textarea (line ~963, though `save_announcements()` exists)
- Paragraph style picker during edit (line ~1059)—but this is OK, style changes are part of the edit session that IS undoable

---

## 2. Defects and Fragilities

### CORRECTNESS

**C1. Escape commits instead of cancels** (line ~1186)
```rust
if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
    commit = true;
}
```
*Impact*: User types wrong text, presses Escape expecting to revert, changes are saved. Lost work.

**C2. Arrow nudges can be orphaned on sudden exit**
Nudges call `begin_gesture()` but rely on timeout or next `drag_stopped`. If:
- User nudges frame
- Immediately clicks sidebar field (taking focus)
- Gesture timeout hasn't fired
- The nudge snapshot sits unreachable in `gesture_start`

Next gesture will incorrectly include the old nudge as part of a different operation.

**C3. ctx_pos can target wrong element**
```rust
// line ~1092
if response.hovered() {
    if let Some(pos) = response.hover_pos() {
        self.ctx_pos = Some(pos);
    }
}
```
Later, context menu uses `self.ctx_pos` (line ~1315). If hover and right-click positions differ (e.g., touchpad right-click), the menu may operate on the wrong guide/frame.

**C4. Style picker not recorded as separate undo** (debatable)
Changing a paragraph's style during edit modifies `edit_styles[]` only. When commit happens, the whole session is one undo step. User cannot undo just the style change while keeping text changes.

---

### ROBUSTNESS

**R1. Frame index can become stale**
```rust
// line ~1290, during drag
let f = &mut self.doc.panels[idx].frames[fi];
```
If a future feature deletes frames while `drag_frame` holds an index, panic. Currently impossible but the structure invites it.

**R2. guide_menu_for persists across context menu dismissals**
```rust
// line ~1325
if self.guide_menu_for != Some(gi) {
    self.guide_menu_for = Some(gi);
    self.guide_input = format!("{:.3}", ob_model::mpt_to_inches(g.pos));
}
```
If user opens menu on guide, dismisses without action, guide is moved programmatically, then user re-opens menu on same guide index: the input field shows the old cached value, not current position.

**R3. Large document causes undo/cache overhead**
```rust
fn snapshot(doc: &Document) -> String {
    serde_json::to_string(doc).unwrap_or_default()
}
```
Called on:
- Every `begin_gesture()`
- Every `end_gesture()` (for comparison)
- Every `shaped_panel_lines()` (cache key, line ~1004)
- Every `preview_sheets()` (cache key, line ~866)
- Every `journal_tick()` (every 2 seconds)

A document with embedded image references (future v1 zip format) could serialize to megabytes. String comparison for cache keys is O(n).

**R4. edit_styles can desync on complex paste**
```rust
fn sync_edit_styles(&mut self) {
    let count = self.edit_text.split('\n').count();
    while self.edit_styles.len() < count {
        let inherit = self.edit_styles.last().cloned().unwrap_or_else(|| "body".to_string());
        self.edit_styles.push(inherit);
    }
    self.edit_styles.truncate(count);
}
```
Pasting multi-line text grows styles by inheriting. But if user pastes, then undoes within TextEdit (Ctrl+Z), `edit_text` shrinks but `sync_edit_styles` only truncates, never restores original styles for those positions.

---

### DESIGN DEBT

**D1. Single selection blocks multi-frame operations**
`selected_frame: Option<usize>` makes "select all frames and nudge" impossible.

**D2. Undo stack stores full snapshots**
60 entries × multi-KB documents = memory pressure. No structural sharing or delta compression.

**D3. No gesture grouping API**
`begin_gesture()`/`end_gesture()` is implicit. A complex operation must remember to bracket correctly; easy to miss or nest incorrectly.

**D4. Text overlay lies about layout** (lines ~1306-1312)
```rust
let first_size = self.edit_styles.first()
    .and_then(|sid| self.doc.style(sid))
    .map(|st| st.font_size as f32 / 1000.0 * scale)
    .unwrap_or(12.0)
    .clamp(10.0, 20.0);
let editor = egui::TextEdit::multiline(&mut self.edit_text)
    .font(egui::FontId::proportional(first_size))
```
The overlay uses one font size for all text. Real shaped output uses per-paragraph styles. Line breaks user sees while typing ≠ line breaks in final layout. This is acknowledged in the code but remains a fidelity gap.

**D5. No cancellation path for editing**
`commit_edit()` always writes. There is no `cancel_edit()` that restores `edit_start` without recording undo.

---

## 3. Structured Work Still Needed

### Keyboard-Only Operation
- No Tab navigation between frames
- No keyboard shortcut to create/delete guides
- No way to cycle selection through frames (Tab/Shift+Tab)
- No keyboard command to exit edit mode with discard (Escape should do this, but currently doesn't)

### Accessibility
- egui has limited accessibility support; the web target will need ARIA
- Color-only selection indication (blue outline) fails WCAG
- No focus indicators for keyboard navigation
- Screen reader cannot describe selected frame or read guide positions

### Clipboard
- Frame cut/copy/paste not implemented
- Rich text paste from Word would require RTF parsing
- No cross-panel frame duplication

### IME (Input Method Editor)
- CJK composition untested
- Composition window position relative to TextEdit overlay unknown
- Potential issues with inline editing coordinates

### Multi-Object Selection
Requires:
```rust
selected_frames: BTreeSet<usize>,  // not Option<usize>
```
Plus:
- Shift+click to extend selection
- Ctrl+click to toggle individual
- Click-drag on empty space for rubber-band
- Group move/resize respecting relative positions

### Text Caret Truthfulness
Current overlay shows text at one size with natural TextEdit wrapping. Actual layout uses:
- Per-paragraph font sizes
- Shaped line breaks at frame width
- Tab-leader expansion for order-of-service lines

Options:
1. Accept the lie, add "Line breaks may differ" indicator
2. Shaped rich-text overlay (significant work)
3. Side-by-side preview while editing

### Interaction Testing Strategy
The code is verified by "compilation and logic review"—no actual input simulation.

**Headless test approach**:
```rust
// Hypothetical test harness
#[test]
fn drag_frame_records_undo() {
    let (mut app, ctx) = test_app_with_doc(bifold_template());
    
    // Open Pages window
    app.show_pages = true;
    
    // Simulate click on first frame
    let frame_center = frame_center_pos(&app, 0, 0);
    simulate_pointer_down(&ctx, frame_center);
    app.update(&ctx, &mut dummy_frame());
    
    // Drag
    simulate_pointer_move(&ctx, frame_center + vec2(50.0, 50.0));
    app.update(&ctx, &mut dummy_frame());
    
    // Release
    simulate_pointer_up(&ctx);
    app.update(&ctx, &mut dummy_frame());
    
    assert_eq!(app.undo_stack.len(), 1, "drag should create one undo entry");
}
```

This requires:
1. Synthetic input injection into egui::Context
2. Deterministic frame timing
3. State inspection helpers

### What Current Structure Makes Impossible

| Feature | Blocking Structure |
|---------|-------------------|
| Multi-select operations | `selected_frame: Option<usize>` |
| Undo with description ("Undo Move Frame") | `undo_stack: Vec<String>` (no metadata) |
| Partial undo (undo style change, keep text) | Atomic snapshots |
| Collaborative editing | Snapshot-based (no OT/CRDT) |
| Cancel edit without undo entry | No `cancel_edit()` function |
| Accessible canvas | egui's current accessibility model |

---

## 4. Ordered Build List

### Foundations (v1-required)

1. **FIX: Escape cancels editing** — Replace `commit = true` with restoration of `edit_start` snapshot when Escape pressed. Add `cancel_edit()` alongside `commit_edit()`. ~15 lines.

2. **FIX: Sidebar field undo** — Wrap church_name, sunday_name, reading field `.lost_focus()` handlers with `push_undo_now()`. ~6 lines.

3. **FIX: End gesture on focus loss** — In `update()`, when `memory.focused()` transitions to Some(_) and `gesture_start.is_some()`, call `end_gesture()`. Prevents orphaned nudge snapshots.

4. **FIX: Use click position for context menu** — Replace `self.ctx_pos` usage in context_menu closure with the menu's trigger position (if egui exposes it) or record position on right-click specifically.

5. **FIX: Clear menu state on dismiss** — Set `guide_menu_for = None` and `frame_menu_for = None` when context menu is not showing.

6. **ROBUSTNESS: Bounds-check frame indices** — Replace `self.doc.panels[idx].frames[fi]` with `get_mut()` and graceful handling. ~10 call sites.

7. **Test harness scaffold** — Create `crates/ob-app/src/test_harness.rs` with `TestApp` wrapper enabling synthetic pointer/key injection. Does not need full coverage yet, just the mechanism.

8. **Keyboard shortcut documentation** — Add status-bar tooltip or Help section listing: Ctrl+Z/Y, Escape, Enter, Arrow, Shift+Arrow, double-click.

### Quality-of-Life (v1-desirable)

9. **Multi-frame selection** — Change to `selected_frames: BTreeSet<usize>`, implement Shift+click extend, Ctrl+click toggle. Group move via averaged delta.

10. **Frame clipboard** — Ctrl+C serializes selected frame(s) to internal clipboard string, Ctrl+V deserializes with offset.

11. **Keyboard frame cycling** — Tab/Shift+Tab cycle through `panel.frames`, Enter on selected enters edit (if has story).

12. **Efficient undo storage** — Replace `Vec<String>` with `Vec<Arc<Document>>` using COW cloning for unchanged structures, or implement command-pattern deltas.

13. **Editing indication on mismatch** — When overlay TextEdit line count differs from shaped line count, show "Layout will differ" badge.

### Later (post-v1)

14. **Accessibility layer** — When egui/eframe ship AccessKit integration, wire up frame selection and guide positions.

15. **IME verification** — Test with Windows IME, macOS Japanese input, and ibus; file issues for composition window positioning if broken.

16. **Rich-text overlay** — Replace TextEdit with custom widget that shapes per-paragraph and shows true line breaks.

17. **Interaction fuzz testing** — Random sequence generator for CI: spawn thousands of click/drag/type sequences, assert no panics and valid undo stack.

18. **Screenshot regression** — Render canvas to headless buffer, compare to golden images, fail on unexpected pixel differences.

---

**Summary**: The interaction skeleton is sound—gesture coalescing, hit-testing, and undo snapshotting all follow recognizable patterns. The critical v1 bugs are Escape-as-commit and unprotected sidebar edits. The design debt (single selection, snapshot size, text overlay fidelity) is acceptable for launch but each item has a clear upgrade path. The largest gap is testability: without a test harness exercising real gestures, "45 tests green" proves the model works, not the interaction.