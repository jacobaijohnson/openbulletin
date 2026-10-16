# UI audit round 2 — synthesis (2026-10-15)

Three blind Wonder lanes over HEAD 0eaeea3: menus/organization
(ui2-menus.md), flow (ui2-flow.md), visual design (ui2-visual.md).
Coordinator spot-verified the load-bearing claims in the code:
the date field applies only on lost-focus-with-Enter or the Set
date button (main.rs:4523–4527); print_fingerprint is set by all
three copy actions including the reading copy (4684, 4704, 4724);
zero .fill( calls exist in the app; PreviewLine carries no font
family, bold, or italic (964–971). All confirmed as reported.

Totals: 2 BLOCKERs (one is the same defect seen by two lanes),
11 MAJORs, ~20 MINORs. Every lane independently praised the same
foundation: one open door, one export door, the guard patterns,
the review windows, the recovery window, plain words. The defects
are organization and honesty-of-state, not capability.

## Convergences (two or three lanes, one fix)

1. **The step sidebar does not scroll** (visual BLOCKER B1;
   menus MAJOR). Content is clipped, unreachable at default and
   minimum window sizes; Print is worst because the sheet
   preview is drawn before the buttons; the status/error line
   sits in the clipped zone.
2. **The first screen presents a fictional church as the user's
   own** (flow MAJOR; visual M3). "123 Main Street" is seeded
   into the profile buffer while the document's address is
   empty; tabbing through commits it; Check certifies any
   non-empty church name.
3. **The week advance answers to two names, and "next" means
   three things** (menus MAJOR; visual m3).
4. **Check's findings do not take you to the problem, and look
   the same whether fine or broken** (flow MAJOR; menus MINOR;
   visual m2). Go-fix buttons exist for one finding type and do
   not navigate to the named panel/box.
5. **Step markers claim "done" for steps only walked past**
   (flow MINOR; visual m8).

## The fix order

**Batch 1 — correctness and reachability.**
1. ScrollArea for the step panel; Print preview capped or the
   copy buttons moved above it.
2. The date applies on focus loss and on leaving step 2, like
   every neighboring field; unreadable dates refuse through
   fail()'s banner. (Flow BLOCKER.)
3. First-run identity: profile buffers seed from the document
   (empty), fields show hint text; nothing fictional can print.
4. Top-bar Open routes every format through open_old_bulletin
   (one door, two entrances).
5. The .obd route through the old-bulletin door runs the
   unsaved-work guard BEFORE the file dialog, like its siblings.
6. A second app instance says plainly at startup that its work
   is not being kept safe (or refuses to start).
7. print_fingerprint is set only by the two print-copy actions.
8. apply_date, keep-through, and next_sunday errors route
   through fail().

**Batch 2 — organization and emphasis.**
9. Top bar regrouped: New/Open/Save | Undo/Redo | Pages/Fonts/
   Keys(Shortcuts).
10. One name for advancing the week ("Start next Sunday")
    everywhere; date buttons renamed "Use this/next Sunday's
    date"; Check heading becomes "Check"; drop Check's generic
    fix trio.
11. Print step: "Open the print copy" is the one strong primary;
    keep-a-file actions grouped under one plain heading.
12. Start: the fresh-start section is named "Start a new
    bulletin" with button-shaped shapes; the shape list arms on
    first click like every other destructive path.
13. Start remembers the last bulletin this office worked on and
    offers it as the first door.
14. Check findings carry panel/frame and navigate; "Check:"
    lines render in the problem color; checks re-run when the
    Pages window closes on the Check step; step markers derive
    from document state or lose "done".
15. Fonts window: style-font section renamed "Which font each
    kind of text uses" and moved up; collapsible(false); outer
    scroll.
16. "Save announcements" button removed; status says the words
    are on the page. Import-in-flight gets a spinner + safety
    line; a failed native .pub read gets honest first words;
    review windows relabel both buttons when armed; door
    buttons disabled while a review is unanswered; Next Sunday
    status mentions Undo.

**Batch 3 — design system and honesty of the picture.**
17. One palette (PROBLEM/WARN/GOOD/INFO + one canvas accent);
    text styles set once at startup (body ≈15–16px).
18. An app icon, wired into the window and the Windows bundle;
    app version visible on screen.
19. Preview fidelity: PreviewLine carries family/bold/italic
    and draws with library faces; sheet preview stops drawing
    what never prints or labels it a guide; captions say what
    is true ("Lines break where they will print").
20. Small drifts: pager words instead of "<" ">"; fit-note
    drops the roadmap promise; status at body size with file
    names, not full paths; guidance line strong; MODES.md
    synced to what shipped (workspaces/search/tooltips marked
    unbuilt ideas).

## Not adopted / deferred

- A menu bar (menus lane: do not change; visibility wins).
- Workspace switch, search, tooltips: unbuilt ideas, docs now
  say so (item 20) rather than features to build this round.
- "Bigger text" toggle: the startup size bump lands now; a
  toggle waits for a user to need it.
- Shaped-caret in-place editing remains the known big editor
  phase, separately scheduled; nothing in this round pretends
  otherwise.

## Shipped (2026-10-15 evening, Jacob's order: all of it, in order)

- Batch 1 (d6c6fac, 119 tests): sidebar scroll + capped print
  preview; date applies on focus loss and step exit; first-run
  identity emptied with hint text; top-bar Open routes all
  formats; armed opens keep the picked file; second instance
  warns; fingerprint set by print copies only; failures banner.
- Batch 2 (7ca84c7, 122 tests): top bar regrouped; "Start next
  Sunday" everywhere; Print's one primary; Start-a-new-bulletin
  buttons with arm; last-bulletin door; Check findings navigate
  to the named panel/box; Fonts window reachable and renamed
  sections; Save announcements removed; import spinner words;
  honest native-.pub failure; review arms relabel both buttons;
  doors disabled during an unanswered review.
- Batch 3 (57dedf2, 123 tests): PROBLEM/WARN/GOOD/INFO palette +
  one canvas accent; startup text styles (Body 15.5px); app
  icon generated and wired (window + bundle); version beside
  the app name; previews render in the real family and face
  (PreviewLine carries family/bold/italic; limit: a family with
  only a regular face draws styled lines in that regular face);
  captions now state exactly that; pager words; fit note
  actionable; status at body size with file names; MODES.md
  synced (workspaces/search/tooltips recorded as unbuilt).
