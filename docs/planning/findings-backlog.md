# Findings backlog: every audit finding, dispositioned

From the four 2026-10-15 audits (GPT UI/UX, Opus interaction,
Wonder x2; synthesis in audit-synthesis-2026-10-15.md). The
eight blockers were fixed first. Everything else is listed here
with its disposition: FIX (batch A/B/C), BY-DESIGN (reason
recorded), or DEFERRED (reason + what would unblock it).

## Wonder UI/UX
- B3 Open discards unsaved work + its autosave silently. FIX-B:
  two-click guard on Open when the screen differs from the file.
- B6 The always-visible main preview misrepresents the bulletin
  (unshaped text). FIX-B: the center shows the real shaped panel.
- F1 Steps look clickable but are not; Check points at steps the
  user cannot jump to. FIX-B: steps are buttons; Check gains
  go-there buttons.
- F3 All feedback is one small line. FIX-B: failures get a
  visible banner treatment via a fail() path at the main sites.
- F4 Print step squeezed into the sidebar. FIX-B: the sidebar
  widens on the Print step.
- F5 (fonts window copy density) FIX-B with GPT-8: shorter copy.
- F6 Printer memory keyed by free-typed name; manual-mode words
  disagree between wizard and checklist. FIX-B: known-printer
  picker; the checklist quotes the stored instruction verbatim.
- F7 Pages window undiscoverable; hint is one breathless line.
  FIX-B: buttons from Steps 2-3 and Check; two short hint lines.
- F8 Announcements box silently rewrites typed text (trims,
  drops blank lines). FIX-A: preserve lines; test.
- F9 Check measures fit with fonts the printer will not use.
  FIX-A: one full shaper (bundled + library) for Check, canvas,
  and preview alike.
- F10 Installing a font visibly changes nothing. FIX-B: the
  Fonts window gains per-style font pickers, so a font can be
  put to work in one click.
- F11 Recovery hides which file the work belongs to; Throw it
  away is unguarded. FIX-B: show the source; two-click discard.
- F13 Start next Sunday marks unsaved work as saved and keeps
  pointing at last week's file. FIX-A: no mark_clean; the path
  clears so Save asks for the new week's name.

## GPT UI/UX (beyond overlaps)
- G3 No explicit last-week path. FIX-B: "Use last week's
  bulletin..." on Start (opens a file and advances the week).
- G6 Start next Sunday is far away. FIX-B: also offered on the
  This Sunday step.
- G7 Raw error text shown to users. FIX-B with F3.
- G9 "Open print copy" vs "Save print copy" unclear. FIX-B:
  relabel with one line each about which is which.
- G10 No visible New command. FIX-B: top-bar New goes to Start.

## Opus interaction
- C1 Escape commits instead of cancels. FIXED BY RULE CHANGE:
  Escape now cancels the typing session (Excel habit); Done or
  clicking away saves. EDITOR.md updated.
- C2 Nudge gesture can be orphaned by a focus change. FIX-A:
  gaining text focus closes any open gesture.
- C3 Menu uses stale pointer. FIX-A: the position is captured
  on the right-click itself.
- C4 Style change is not its own undo step. BY-DESIGN: a typing
  session is one step, documented in EDITOR.md.
- R-series: bounds-checked indices at drag sites (FIX-A);
  menu state keyed by target identity (FIX-A); shortcut help
  line in the Pages window (FIX-C); frame clipboard (FIX-C);
  keyboard frame cycling (FIX-C); multi-select DEFERRED (needs
  the selection model rework; single selection + undo covers
  the office workflow for v1); undo storage as strings
  BY-DESIGN at church scale (60 steps, small documents);
  "layout will differ" badge DEFERRED (would cry wolf: the
  overlay wraps differently almost always); accessibility and
  IME verification DEFERRED to a human on real platforms
  (headless here); rich-text overlay DEFERRED (its own phase).

## Wonder interaction (beyond the fixed blockers)
- C4 Undo skips the step fields and leaves stale buffers.
  FIX-A: field applies record undo; buffers sync on undo
  (buffers already sync via adopt; the recording is the fix).
- R1 Window close orphans the session. FIXED with blocker 6.
- R2 Paragraph styles shift when lines are inserted mid-text.
  FIX-A: style inheritance follows the cursor paragraph.
- R3 = Opus C3. FIX-A.
- R4 One shared editor Id leaks state between sessions. FIX-A:
  the editor Id is salted by story.
- R5 Menu buffers keyed by index. FIX-A: keyed by identity
  (position snapshot for guides, rect for frames).
- R6 Full-document JSON per frame for cache keys. FIX-A: one
  snapshot per frame, shared by the caches.
- R7 Missing tracing picture fails silently. FIX-A: plain note
  in the Pages window when the picture cannot be found.
- R8 Focus dead zone while editing. FIX-A: toolbar states how
  to finish; Escape rule clarified with C1.
- R9 No zoom or pan. FIX-C: zoom buttons + scroll area.
- R10 Overlay/gates layering. NOTED: behavior verified by the
  gate tests added in Batch A (edit-session unit tests).
- R11 Escape during IME. DEFERRED with Opus IME item.
- D1 No transaction layer. PARTIAL BY-DESIGN: gestures, adopt,
  and record_undo are the funnel; a command-pattern rewrite is
  not justified at this scale.
- D2 No frame lifecycle. FIX-C: add a box, delete a box.
- D3 Overlay is not the real text. DEFERRED (own phase).
- D4 Three renderers, two fidelities. FIXED by B6 + F9: every
  surface now draws from panel_lines with the full shaper.
- D5 Guides have no identity or states. FIX-C: guides gain a
  lock; identity stays positional within a panel (menus key by
  position snapshot, R5).
- D6 Canvas invisible to keyboard-only use. FIX-C slice: Tab
  cycles boxes, arrows move, Enter types. Full assistive
  support DEFERRED with the accessibility item.
- D7 Interaction layer untested. FIX-A/C: unit tests for the
  edit session, gesture-to-undo, menu target choice, and the
  announcements write rule.
