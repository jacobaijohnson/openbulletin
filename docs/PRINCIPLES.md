# Principles

1. **KISS: Keep It Short & Simple** (Jacob's expansion). If a church office administrator cannot describe what a
   feature does in one sentence, it does not ship. Complexity lives in the
   engine (imposition, fields, print geometry), never on the user.
   (Apple's Simplicity: the best tools are the ones users never notice.)
2. **Simple on the surface, powerful underneath.** Every screen must
   pass the ELI5 test: explainable to a five-year-old, in plain words,
   without losing the person reading it. The engine underneath does the
   opposite: real text shaping, real print geometry, no shortcuts that
   print wrong. Easy to use is not the same as weak.
   (Apple's Aesthetic Integrity: decoration never competes with the task.)
3. **The user is an administrator, not a designer.** Church office staff
   and volunteers, working fast, often on a deadline, often covering for
   someone else. No layout jargon on the main path: no "frames", "stories",
   "imposition" in the guided flow. Say "panels", "this week's text",
   "print copy". Two working rules ride along: show people their options
   instead of making them remember and type (Apple's See-and-Point), and
   name every action for its object, in plain words ("Open the print
   copy" is a fine label; a bare "OK" is not) (Apple's Explicit and
   Implied Actions, which is about acting on a visible object, noun
   first in the hand, not label grammar; settled 2026-10-15 in the
   lens review synthesis).
4. **Guided, with the next step always visible.** The home screen is the
   weekly flow: Start, This Sunday, Announcements, Check, Print. One
   primary action per step. The app suggests the next step after every
   action (after Print: "Start next Sunday"). The app always says what
   state the work is in and what happens next (Apple's Feedback and
   Communication), and its windows sit beside the flow rather than
   trapping the user inside a mode (Apple's Modelessness).
5. **Automate the recurring.** Dates advance themselves, standing content
   carries over, weekly content clears itself, fields fill the cover and
   the order of service. The user's job shrinks to what actually changed.
   Automation proposes and carries; the human fires every consequential
   act (Apple's User Control).
6. **Never lose work, never surprise at the printer.** Autosave, atomic
   saves, plain-language preflight, and print output that matches the
   preview. A Saturday-night failure is a release blocker. Every
   destructive act is reversible or armed, so nobody is afraid to try
   (Apple's Forgiveness), and the app never moves, renames, or removes
   things on its own motion between visits (Apple's Perceived Stability).
7. **Imports are rescues, not magic.** Publisher (.pub), InDesign (via
   IDML), PDF, and our own .obd all open here, with honest fidelity
   labels. PDF analysis can draft a template from an old bulletin PDF;
   a human always confirms before it becomes the template.

8. **Build isolation.** OpenBulletin never shares space with Jacob's other
   project (the EV Classic Mac harness): separate repo, separate chats,
   separate desktop runner, container, and caches. No cross-contamination.

9. **Conform to the office worker, not the other way round**
   (Jacob's words, 2026-10-15). The user already knows Office and
   Publisher habits: rulers in inches, right-click menus, typing an
   exact number into a box. Power is welcome, but it must arrive in
   the shapes they already know, one visible step at a time. If a
   feature makes them learn us instead of the other way round, the
   feature is wrong, however elegant the engine is. Wherever the user
   can grab the thing itself (a box, a guide), they do; making them type
   instructions about it instead is a debt to record, not a style to
   copy (Apple's Direct Manipulation, on top of Metaphors and the Mental
   Model).

## Where this list comes from

These nine are ours. Woven through them, marked in parentheses,
are the demands of Apple's OS X Human Interface Guidelines
philosophy set, incorporated on Jacob's order 2026-10-15 and
integrated here rather than kept as a second rulebook. The
interaction habits borrowed from InDesign and Publisher (frames
underneath, threading made visible, one pointer instead of two,
Publisher's template-first start and Business Information)
live where builders use them: docs/EDITOR.md for the canvas,
and the step design above for the flow.

## Standing checks for every build and audit

1. Can the action be named noun first, then verb, in office
   words? If not, redesign it.
2. Does the screen say what state the work is in, and what
   happens next, without being asked?
3. Is the destructive version of this action armed or
   reversible, in the same words as every other one?
4. Does anything on screen claim more than it does (a preview,
   a "saved", a "done")? Downgrade the claim or upgrade the
   behavior until they match.
5. Could the user do this by grabbing the thing itself? If
   they must type instructions about it instead, record the
   debt.
