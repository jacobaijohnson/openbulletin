# Audit synthesis, 2026-10-15: is OpenBulletin complete?

Four audits ran blind on identical evidence (the full app source,
the principles, the editor spec): GPT-5.1 on UI/UX, Opus 4.5 on
interaction structure, and two Muse Wonder subagents, one per
brief. The coordinator verified every headline claim against the
code before repeating it here.

## The verdict, unanimous

**No. Not yet usable by an unaided church office admin.** The
weekly loop, the plain language, the print truthfulness, and the
autosave net are genuinely in place (all four audits say so). But
a cluster of blockers, most of them cheap to fix, breaks the
project's own laws: never lose work, never surprise the user.

## Verified blockers (coordinator-checked in the code)

1. **The bulletin date cannot be set.** Step 2 shows the date as
   a read-only label; a new document is stamped with today's
   date. An admin building Sunday's bulletin on Wednesday gets a
   Wednesday date and no way to fix it. (GPT #1, Wonder B1.)
2. **Only the church's name is editable.** Address, service
   times, website, and pastor exist in the engine and on the
   printed page; the app offers no field for any of them.
   (GPT, confirmed: the Start step's only profile field is the
   church name.)
3. **Clicking a different bulletin shape silently discards the
   document.** The preset picker calls rebuild() with no
   confirmation and no undo; rebuild also retires the autosave
   journal. (Wonder B2.)
4. **The autosave tick can erase canvas typing.** Every two
   seconds the journal writes the Announcements buffer back into
   its story, whether or not the buffer is current. Type the
   announcements on the page instead, and the stale buffer
   overwrites them. (Wonder B5.)
5. **Undo crosses file boundaries, and Save can then overwrite
   the wrong file.** Open, rebuild, and recovery never clear the
   undo stacks: open bulletin B, press Ctrl+Z, and file A's
   content returns under B's path; Save writes it into B.
   (Wonder interaction C1. The single worst finding of the day.)
6. **Double-clicking another box while typing silently drops
   the text.** The double-click handler starts a new edit
   session before the old one commits; the buffer and its undo
   snapshot are overwritten. (Wonder interaction C2.)
7. **A reachable panic.** Selection and edit state are raw
   indices, never reset on document swap; select a box, open a
   smaller bulletin, press an arrow key, and a direct index
   panics. (Wonder interaction C3.)
8. **The guide menu is unreachable where a box covers the
   guide** (Wonder C5; the coordinator had noticed and accepted
   this during the build; the auditor is right that it breaks
   the documented right-click-a-guide contract), and the top-bar
   Undo/Redo buttons bypass the editing gates the keyboard path
   respects (Wonder C6).

## Where the auditors converged without seeing each other

- Date + profile depth (GPT and Wonder, independently, as their
  top UI findings).
- The stale pointer position feeding the right-click menu
  (Opus C3 and Wonder R3, the same mechanism).
- The text overlay is not the real text: one font size, no
  shaped caret (Opus "caret truthfulness", Wonder D3). Both call
  it the deepest design debt in the editor.
- Interaction testing strategy: both Opus and Wonder demand
  headless gesture tests, because the interaction layer is the
  one layer with no tests at all (Wonder D7).

## Where they differed

- Opus's Escape finding ("Escape commits instead of cancels")
  is a spec question, not a defect: EDITOR.md documents commit-
  on-Escape as the behavior, and the coordinator's report to
  Jacob said the same. It stays flagged as an expectation risk
  worth one decision, not a bug.
- Opus was broadest on the structured-work taxonomy (keyboard-
  only operation, accessibility, clipboard, IME as named
  workstreams). Wonder was deepest on reachable defects in this
  codebase. GPT walked the journey most completely, stop by stop.

## The routing answer (Jacob's question: subagents in future?)

On defect-finding against our own code, the Wonder subagents
matched or beat the external models: the three worst work-loss
findings (3, 4, 5 above) are Wonder-only catches; GPT matched
Wonder on the missing-UI blockers; Opus contributed framing
breadth more than defect depth. The blindness and the brief did
most of the work; the model family mattered less than the tier
doctrine assumed, at least for code the auditor can read whole.

Recommendation: default to Wonder subagent audits with strong
briefs for routine gates; reserve Tier-2 (Opus + GPT) for
milestone judgments and first-of-kind questions, where a
different family's framing is itself the evidence wanted.
External spend for this round: about $0.52 estimated (GPT 28.2k
in / 12.8k out; Opus 38.3k in / 10.2k out).

## Fix order (proposed)

1. Invalidate interaction state + clear undo on any document
   swap; gate the Undo/Redo buttons like the keyboard. (5, 7, C6)
2. Commit the edit session before starting another; likewise on
   any exit path. (6)
3. Stop the journal tick from writing a stale announcements
   buffer over the story; sync the buffer from the story when
   canvas editing touches it. (4)
4. Confirm before a preset click discards a changed document,
   or make rebuild itself an undoable gesture. (3)
5. A date field on Step 2 (type or pick), and the remaining
   profile fields on Step 1. (1, 2)
6. Guide hit wins the context menu when the click is on the
   line itself. (8)
Then the friction tier from the four reports.

## Resolution (same day)

All eight blockers are fixed in the app, in the order above:
adopt_document is now the single swap path (history cleared
across files, interaction indices reset on every swap, Undo/Redo
buttons gated like the keyboard); edit sessions commit before
any new session or window close; the announcements buffer writes
only when it changed and canvas commits refresh it; a preset
change is an undoable gesture that carries the profile across;
Step 2 has a real date field with This Sunday / Next Sunday
buttons and Step 1 has address, service times, website, and
pastor fields; a click on a guide line itself opens the guide
menu even under a frame. New unit tests cover the swap behavior
and the Sunday math. 47 tests green, clippy clean. The friction
tier from the four reports remains open.
