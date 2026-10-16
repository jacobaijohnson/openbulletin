# Design reference

Adopted 2026-10-15 on Jacob's order, and incorporated into our
design. This document is a reference library, NOT a replacement:
the nine principles in PRINCIPLES.md remain the law of this
project, and where this document and that list disagree, that
list wins.

Two sources, both chosen because our users already live in them:

1. **Apple's OS X Human Interface Guidelines, "The Philosophy of
   UI Design"** — the design philosophy set for document-based
   desktop apps with printed output, which is what we are.
2. **The interaction habits of InDesign and Publisher** — the
   two programs our users' hands already know. We borrow their
   vocabulary where it helps and refuse it where it serves
   designers instead of administrators (PRINCIPLE 3 rules).

## The OS X philosophy set, and what each one means here

**Metaphors.** Borrow from the real world so people bring their
expectations with them.
Here: the bulletin is a paper object. Panels fold, sheets have
fronts and backs, the print copy is a physical thing. We say
"sheet" and "fold", never "imposition".

**Reflect the user's mental model.** The app matches how the
person already thinks the task works.
Here: this is Principle 9 in Apple's words. The weekly job is
"take last week's, change what changed, print it". The five
steps are that sentence.

**Explicit and implied actions.** Actions act on visible
objects; noun first, then verb.
Here: select a box, then act on it. Buttons name their object
("Save print copy", never bare "Save"). Any new control that
cannot be named noun-then-verb gets redesigned until it can.

**Direct manipulation.** Grab the thing itself and move it
instead of typing instructions about it.
Here: the Pages canvas (drag boxes, drag guides, live inch
readouts). Its unfinished edge is text: editing is still
whole-box, and true in-place typing is the editing phase this
principle argues for loudest.

**See-and-point.** Show people their options; never make them
remember and type.
Here: pickers, buttons, and preset lists everywhere. Nothing on
the guided path requires recalling a file format, a command
name, or a number format.

**User control.** The person initiates and directs; the
computer does not take over.
Here: automation (Principle 5) proposes and carries, but the
human fires every consequential act, and exports warn instead
of blocking. The app never advances a week, replaces a
document, or discards a draft on its own motion.

**Feedback and communication.** Always show what happened and
what is happening.
Here: the status line, the red failure banner, spinners with
honest words on long reads, counts stated out loud ("N
announcements ended"). Silence after an action is a bug.

**Consistency.** The same words, look, and behavior, within the
app and across the apps the user knows.
Here: one palette, one name per action everywhere it appears,
armed-confirm wording identical on every discard path, review
windows built on one shared pattern.

**WYSIWYG.** What is on screen is what comes out.
Here: previews render in the real font and face, line breaks
are the printer's line breaks, and captions say only what is
true. A preview that flatters is a defect, not a kindness.

**Forgiveness.** Make actions easy to reverse so people are not
afraid to explore.
Here: undo everywhere, two-click arms on destructive acts,
the recovery journal, imports that land as drafts a human
accepts or cancels. Principle 6 is this principle's local name.

**Perceived stability.** The environment stays familiar and
predictable; things do not move or vanish on their own.
Here: steps do not reorder, buttons do not relocate between
visits, a second app window announcing itself instead of
silently changing the rules (fixed in UI round 2).

**Aesthetic integrity.** Information well organized; graphics
subordinate to the task.
Here: one primary action per step, calm type scale, decoration
never competes with the next step. The canvas furniture serves
the paper, not the app's ego.

**Modelessness.** Let the user do what they want at any time;
no mode that traps.
Here: windows (Pages, Fonts, Shortcuts) open beside the flow,
not instead of it; the canvas typing session is the one modal
pocket we keep, and it is fenced accordingly (Escape cancels,
clicking away commits, both stated in EDITOR.md).

**Simplicity.** The best tools are the ones users never notice.
Here: Principle 1. If a design needs an explanation paragraph
in the app, the design is wrong; the words budget goes to the
state of the user's work instead.

## Interaction habits borrowed from InDesign and Publisher

**From Publisher (our users' home program):**

- *Template-first starting.* ADOPTED: Start is doors over old
  work and shapes, and the Analyzer builds the template from
  the church's own bulletins. Nobody starts from a blank page
  unless they ask to.
- *The wizard.* ADOPTED IN SPIRIT: Publisher's PageWizards
  became our guided steps and the two-sided printing help:
  questions in office words, one at a time, with the result
  remembered per printer.
- *Business Information.* ADOPTED: a saved set of facts about
  the organization, filled once, appearing everywhere. Ours is
  the Church Profile with fields that resolve across the cover
  and the order of service.
- *Task-shaped panels in office words.* ADOPTED: our steps are
  the task pane, named for the job ("This Sunday"), never for
  the tool.

**From InDesign (the professional vocabulary, filtered):**

- *Frames as the atom of the page.* ADOPTED UNDERNEATH: boxes
  hold text and pictures on the canvas. The word "frame" stays
  off the guided path (Principle 3); the concept carries the
  editor.
- *Threading made visible.* ADOPT WHEN THE EDITOR GROWS IT:
  linked boxes show their link, and overflow is a mark on the
  box, not a surprise at the printer. Our .pub reader already
  honors threading on import; the editor should show it too.
- *Smart guides and snapping.* ADOPTED AND GROWING: guide
  snapping ships; frame-to-frame snapping joins when the
  editor's next phase lands.
- *One selection tool for content, one for the container.*
  ADAPTED DOWN: InDesign splits Selection and Direct
  Selection; administrators get one pointer that does the
  obvious thing, with the right-click menu carrying the rest.
- *Layers, effects panels, GREP anything.* LEFT BEHIND: depth
  that serves designers. If a bulletin needs it, that is the
  signal it belongs in the export-to-InDesign door, not in
  this app.

## Standing checks for future builds and audits

Every UI change and every audit lane checks, in this order:

1. Can the action be named noun first, then verb, in office
   words? If not, redesign it.
2. Does the screen say what state the work is in, and what
   happens next, without being asked?
3. Is the destructive version of this action armed or
   reversible, in the same words as every other one?
4. Does anything on screen claim more than it does (a preview,
   a "saved", a "done")? Downgrade the claim or upgrade the
   behavior until they match.
5. Could the user do this from the canvas by grabbing the
   thing itself? If they must type instructions about it
   instead, that is a debt to record, not a style to copy.
