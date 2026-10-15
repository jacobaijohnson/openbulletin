# Closing audit, effectiveness lane (Wonder, blind)

Scope: the weekly journey walked in the code
(`crates/ob-app/src/main.rs`, 3,663 lines, plus ob-layout /
ob-model call sites), judged against docs/PRINCIPLES.md,
docs/EDITOR.md, the eight blockers in
`docs/planning/audit-synthesis-2026-10-15.md`, and the batch
A/B/C dispositions in `docs/planning/findings-backlog.md`.
No code was modified by this lane.

Baseline, verified this session: `cargo test --workspace` =
**53 passed, 0 failed** (ob-app suite: 12). The backlog's
"53 workspace tests" claim is accurate.

## Verdict up front

**No. Do not hand this to a church office admin today.**
The journey's shape is right and most of the famous fixes are
real, but two blockers remain, and both destroy work: one sits
on the very first button of the weekly journey, and one is a
crash/corruption path introduced by batch C's own Delete
feature, in the same stale-index family as original blocker 7.
First thing that would bite: the "Use last week's bulletin..."
button on Step 1 silently throwing away unsaved work and its
autosave copy, with no warning, in the exact spot where the
guarded Open button taught the user to expect one.

Counts: **2 BLOCKER, 6 MAJOR, 10 MINOR.**

---

## BLOCKER

### E-B1. "Use last week's bulletin..." bypasses the unsaved-work guard and destroys the work and its autosave

- Evidence: `crates/ob-app/src/main.rs:2683-2696`. The handler
  loads the picked file, calls `adopt_document(doc, true)`
  (clears Undo history), sets the path, and calls
  `mark_clean()` (line 2692), which clears the autosave journal
  files. There is no dirty check and no two-click arming.
- The top-bar "Open bulletin..." button *does* have the guard
  (main.rs:2590-2610: `dirty = frame_snapshot != saved_snapshot`,
  arm, plain warning "it will be gone for good"). Finding B3
  was fixed at that door only. Batch B then added this second,
  unguarded door (finding G3) in the most prominent spot of
  the primary journey: Step 1, first heading, first button.
- Scenario: the admin typed this week's announcements into a
  fresh, never-saved bulletin, then clicks "Use last week's
  bulletin..." to start properly. The typed work, its Undo
  history, and its journal copy are all gone in one click, with
  no message. This is the B3 defect class the synthesis called
  a blocker, relocated, not retired.
- Fix shape: route this button through the same armed/dirty
  check as Open (one shared guard function), before the picker
  opens.

### E-B2. Deleting a box while a typing session is open leaves a stale edit index: reachable panic and wrong-story overwrite

- Evidence: the frame context menu is live while editing (no
  gate anywhere in `panel_canvas`). "Delete this box"
  (main.rs:2016-2038) removes the frame and clears
  `selected_frame`, but never touches `self.editing` /
  `self.edit_start`. The editor draw then indexes directly:
  `panel.frames[efi]` at main.rs:1914 (and the drag-outside
  check at main.rs:1666), while `commit_edit` resolves the
  frame with `.get(fi)` and writes the buffer into whatever
  story now sits at that index.
- Scenario: double-click the last box on a page and type.
  Right-click an *earlier* box, choose Delete. Indices shift;
  `editing` now points past the end or at a different box. Next
  frame, the draw at line 1914 panics if the edited box was
  last; if not last, the typing session overlays the wrong box
  and Done writes the announcements text into a different
  story. This is original blocker 7's mechanism (raw indices
  never invalidated on document mutation) re-entering through
  batch C's Delete (finding D2), which mutates the document
  outside the `adopt_document` funnel the fix relied on.
- Fix shape: Delete (and any frame-removing path) must end the
  edit session first (commit if the deleted frame is not the
  edited one; cancel or commit-then-delete if it is), or shift
  `editing` with the removal; and the two direct-index sites
  (1666, 1914) should use `.get()` like `commit_edit` does.

---

## MAJOR

### E-M1. Save and every Print export ignore an open canvas typing session: the outputs silently miss the newest words, and Save still says "Saved"

- Evidence: "Save bulletin" (main.rs:2626-2645) calls
  `save_announcements()` and `pin_fonts()`, never
  `commit_edit()`. "Open the print copy" (main.rs:2879-2888),
  "Save print copy" and "Save reading copy" (main.rs:2891-2925)
  do the same. The commit paths that exist are Done typing,
  click-outside on the canvas, panel switch, Pages-window
  close, Next Sunday, rebuild, and Add-a-box; none of the
  output paths are among them, and nothing on screen says the
  words being typed are not yet in the document.
- Scenario: admin double-clicks a box on the page, types the
  new announcement, leaves the Pages window open, clicks
  "Save bulletin" in the top bar: status reports "Saved to
  ...". The file does not contain the typing. Same for the
  print copy: the PDF prints the pre-edit text while the
  overlay on screen shows the new words, a print-truth
  violation of Principle 6, and the autosave journal cannot
  rescue the buffer either (it stores the document, not the
  edit buffer), so closing the app from the OS window loses
  the session outright.
- Fix shape: every output path (Save, all three exports)
  commits an open edit session first, exactly as
  `next_sunday` and `rebuild` already do.

### E-M2. Changing the bulletin shape silently resets the date, the Sunday name, and the reading

- Evidence: `rebuild()` (main.rs:576-609) builds the new
  template with `chrono::Local::now().date_naive()` and the
  literal sunday name `"Sunday"`, and carries only the church
  profile across. The old document's `sunday_date`,
  `sunday_name`, and `reading` fields are discarded.
- The blocker-3 fix made this undoable and says so ("Undo
  brings the old bulletin back"), so it is no longer silent
  data destruction; but the *expected* use, an admin switching
  shape mid-week and continuing, quietly re-dates the bulletin
  to today (a Wednesday, say) and renames the Sunday to plain
  "Sunday". Check (E-m2 below) will not catch either. A wrong
  date on the cover is the classic bulletin error this product
  exists to prevent.
- Fix shape: carry the fields map across rebuild the way the
  profile is carried; only the layout should change.

### E-M3. The failure banner (F3) is only half-wired: most messages bypass it, and its state goes stale

- Evidence: 37 direct `self.status = ...` assignments vs 10
  `note()` / `fail()` calls in main.rs. The banner renders from
  `status_error` (main.rs:3017-3026), but direct assignments
  never update that flag. Consequences both ways:
  - Print-step failures, "Could not save the print copy: {e}"
    (main.rs:2903) and the reading-copy twin, render as small
    gray text, the exact F3 failure mode: an admin can miss
    that the file she needs on Sunday was never written.
  - After any `fail()` (a bad Open or Save), later successes
    written by direct assignment (exports, wizard answers,
    Next Sunday) display under "Something needs attention:" in
    red. Success dressed as alarm teaches the admin to ignore
    the banner, which defeats it.
- Fix shape: make `status` assignment impossible outside
  `note()`/`fail()` (a setter), or sweep the direct sites.

### E-M4. The autosave tick deletes the recovery copy while the recovery decision is still pending

- Evidence: `startup()` (main.rs:860-875) preserves a leftover
  journal as `self.recovery` for the dialog. `journal_tick()`
  (main.rs:877-898) never checks `self.recovery`: when the
  screen equals the saved snapshot (the normal state behind
  the recovery window), the first arm clears the journal files
  within about two seconds of launch.
- In-session, "Bring it back" still works from the in-memory
  copy, so this is not user-visible until the worst moment:
  the app crashes (or the laptop dies) while the admin is
  still deciding, and the copy the dialog is offering no
  longer exists on disk. The safety net removes itself while
  it is being offered.
- Fix shape: `journal_tick` returns early while
  `self.recovery.is_some()`.

### E-M5. Check rebuilds the entire font library from disk for every paragraph

- Evidence: `run_checks` calls `self.full_shaper()` inside the
  per-paragraph loop (main.rs:739). `full_shaper()`
  (main.rs:677-687) re-opens the Font Library, reads every
  font file, and re-registers every face, each call. A
  16-panel booklet holds a few hundred paragraphs; a real
  church library is 10-40 MB of fonts. Arriving at Check (or
  pressing "Check again") can stall the app for seconds with
  no progress indication, indistinguishable from a freeze, on
  the step whose whole job is reassurance.
- Secondary: if the library cannot be opened, `full_shaper`
  silently falls back to bundled fonts only, quietly reopening
  the F9 lie (Check measuring with fonts the printer will not
  use) with no note anywhere.
- Fix shape: build the shaper once per `run_checks` call (the
  canvas and preview paths already do this correctly).

### E-M6. Every fresh bulletin prints "Give online: " with nothing after it, and the app offers no way to fill it in

- Evidence: the template's back panel includes
  `para("body", "Give online: {Church.giving}")`
  (`crates/ob-layout/src/lib.rs:223`). The profile's
  `giving_url` is carried through rebuild (main.rs:595) but no
  field for it exists anywhere in the app (grep: the only app
  references are that carry-through). Default is empty, so the
  congregation's paper shows a dangling "Give online:" line,
  and Check has no note about it. Blocker 2's field list
  (name, address, times, website, pastor) is complete in the
  UI; this sixth printed field was missed by every pass,
  including this backlog's.
- Fix shape: either a Giving link field on Step 1 beside
  Website, or the template omits the line when the field is
  empty; plus a Check note while it is empty and printed.

---

## MINOR

- **E-m1. Typed date only applies on Enter or "Set date".**
  Step 2's date field applies on `lost_focus() && Enter`
  (main.rs:2766-2771) or the Set date button; tabbing or
  clicking away leaves the typed date unapplied while the
  field still shows it, and Check then blesses the *old* date.
  The button makes this recoverable; the divergence is the
  trap. Apply on lost focus like every other field in the app.
- **E-m2. Check validates the date's presence, not its
  correctness.** `run_checks` (main.rs:695-698) only checks
  `sunday_date_long` is non-empty. A Thursday date passes with
  "OK: bulletin date is Thursday, ...", and the placeholder
  Sunday name "Sunday" (seeded by `App::default` and by
  `rebuild`) is never flagged. For a product whose cover is a
  date, one "that date is not a Sunday" note is cheap.
- **E-m3. Two long-date formats.** `apply_date` writes
  "Sunday, October 18, 2026" (main.rs:262-265); ob-layout's
  `long_date` (used by Next Sunday) writes "October 18, 2026"
  (ob-layout/src/lib.rs:287-289). Same field, two shapes,
  depending on which door set it.
- **E-m4. Pages-window buttons do not navigate to the panel
  they name.** "See the announcements on the page" (Step 3 of
  the flow, main.rs:2812-2815) and Check's "Open the Pages
  window" (main.rs:2852-2854) open the window on whatever
  panel was last viewed (`canvas_panel` persists; adopt resets
  it to 0, the cover). The admin hunting announcements lands
  on the cover and must page through. Open it on the panel
  that holds the announcements story.
- **E-m5. Stale copy in the fit warning.** The Check fit note
  still ends "...or we can shrink them in a later version"
  (main.rs:754), while the small print under the Check buttons
  correctly says to open Pages and make the box bigger. The
  two instructions contradict; the note is the one attached to
  the problem.
- **E-m6. The Open guard never disarms.** `open_armed` is set
  on first click and only cleared by completing an Open
  (main.rs:2590-2611). Click Open, read the warning, work for
  an hour, click Open again: it goes straight to the picker.
  The warning's protection decays silently. Arm with a short
  expiry or disarm on any other action.
- **E-m7. The starter document prints sample data verbatim.**
  `App::default` seeds "Grace Lutheran Church",
  "123 Main Street", "Sundays 9:30 AM" (main.rs:451-470), and
  the Step 1 buffers carry "123 Main Street" into any rebuild.
  It is labeled sample data nowhere on screen; an admin who
  prints the starter to test the printer gets a plausible,
  wrong bulletin. Harmless in a demo, worth one "sample" note.
- **E-m8. Undo/Redo stack hygiene.** `undo()` and `redo()`
  push to the opposite stack directly (main.rs:173-199),
  bypassing the 60-step cap in `record_undo`; long undo/redo
  ping-pong can grow the redo stack past the documented cap.
  No user harm at church scale; noted for the code lane.
- **E-m9. README now understates the app in two places.**
  "You cannot pick a font for a style inside the app yet" and
  "Typing words directly on the page... Not built yet" are
  both false since batches B and the editor build (style
  pickers at main.rs:2194-2241; inline editing is the canvas's
  core). The Pages window is not mentioned at all. A returning
  admin reading the front door learns the app is weaker than
  it is. (The rest of the README's honesty, including "no
  installer yet," is intact and important.)
- **E-m10. Paste of a story-less box reuses one frame id.**
  `paste_box` names story-less pastes `frame-shape` every time
  (main.rs:1118-1121), so repeated pastes share an id; and the
  paste offset clamp can go negative if the copied box is
  larger than the panel. Neither bites the weekly journey;
  both are one-line fixes for the code lane.

---

## The eight blockers, re-verified in behavior

1. **Date cannot be set: FIXED.** Step 2 field + Set date +
   This/Next Sunday buttons + undo recording
   (main.rs:251-277, 2755-2795). Caveat E-m1 (apply trigger).
2. **Only the church name editable: FIXED, one field missed.**
   Address, service times, website, pastor all present with
   undo (main.rs:2707-2750). The sixth printed field,
   giving_url, was missed by everyone (E-M6).
3. **Shape click silently discards: FIXED as specified.**
   `rebuild` records the old document for Undo and keeps the
   journal on the new one (main.rs:576-609). Side effect
   E-M2 (fields reset) is the remaining sharp edge.
4. **Autosave tick erases canvas typing: FIXED.** Dirty gate
   at main.rs:611-625, canvas commits refresh the buffer
   (main.rs:1243-1252), and the regression test
   `announcements_buffer_only_writes_when_dirty` guards the
   gate that was once lost in a splice. This is the best-
   evidenced fix of the eight.
5. **Undo crosses file boundaries: FIXED.** One funnel,
   `adopt_document` (main.rs:280-289); Open and recovery pass
   `clear_history = true`; test
   `opening_another_file_clears_undo_and_selection`.
6. **Double-click drops the edit: FIXED for session starts.**
   `enter_edit` commits first (main.rs:1182-1185); panel
   switch and window close also commit. The family is not
   fully closed: outputs do not commit (E-M1).
7. **Reachable panic via stale indices: FIXED on the swap
   paths, REOPENED by Delete.** Adopt resets every index; the
   remaining direct-index sites are safe until a frame is
   removed mid-session (E-B2).
8. **Guide menu under a frame + ungated Undo buttons:
   FIXED.** `choose_menu_target` gives the line itself
   priority (main.rs:3168-3182) with a unit test; the top-bar
   buttons are gated on `editing.is_none()` and stack
   emptiness (main.rs:2566-2586), matching the keyboard path.

## Do batches A/B/C hold together as one journey?

Mostly yes. The funnel discipline (adopt, record_undo,
push_undo_now, commit-before-start) is consistent everywhere
it was applied, and the failures above are precisely the
places a new surface was added without re-entering the
funnel: the G3 button skipped B3's guard, batch C's Delete
skipped blocker 7's invalidation, the exports were never
brought into blocker 6's commit rule, and the F3 banner was
added as a rendering feature without a write-path rule.
That is one lesson, not four: every new door needs the same
four questions (dirty? editing? indices? feedback?) the
funnel answers. The plain language throughout is genuinely
good: the wizard, checklist, recovery dialog, and Check notes
all pass the ELI5 test, and the print preview's claim
("Lines break where they will print") is backed by a real
shared code path (`panel_lines`) and matching clip behavior
in ob-pdf (lib.rs:208-209 clips at the frame bottom exactly
as the preview does).

## Stop-by-stop journey verdict

- **Start:** The weekly shape is right: last-week button,
  shape picker, five profile fields. Blocked by E-B1 and
  nicked by E-M2. **Fail (blocker).**
- **This Sunday:** Date, name, reading all editable and
  undoable; next-week command present. Nicks: E-m1, E-m3.
  **Pass with nicks.**
- **Announcements:** The box is honest now (dirty gate, blank
  lines kept), and the canvas is a real second door. Nick:
  E-m4. **Pass.**
- **Check:** Real shaped measurement, plain words, buttons
  that go somewhere. Nicks: E-M5 (stall), E-m2, E-m5.
  **Pass with a major.**
- **Print:** Preview and checklist are the most truthful part
  of the app; the wizard's learned instruction is quoted
  verbatim, one source of words (F6 genuinely closed). One
  major: E-M1 (an open typing session is not in the PDF) and
  E-M3's print-step half. **Pass with majors.**
- **Start next Sunday:** The model behavior is right: date
  +7, weekly words cleared, standing kept, path cleared so
  Save asks for a new name, status says so. **Pass.**
- **Fonts (side door):** Install visibly does something now:
  per-style pickers with plain rights words and Undo
  (E-F10 closed). **Pass.**
- **Recovery (the bad morning):** The dialog names the file
  and the time, and discard is two-click. One major: E-M4.
  **Pass with a major.**

## The handover question

**Could you hand this to a church office admin today? No.**
Not because the journey is wrong; it is recognizably the
right journey, in her words, with a print step that tells
the truth. No because two one-click paths still destroy the
week's work, and "never lose work" is Principle 6 and the
product's founding promise. E-B1 and E-B2 are both small
fixes (a shared guard call; a session check in Delete plus
two `.get()`s). With those two closed, and E-M1 behind them,
my answer flips to yes for a supervised first Sunday.

**The first thing that would bite:** the "Use last week's
bulletin..." button, Step 1, first click of the week,
silently discarding whatever the admin had already typed.
It is the button we taught her to press.
