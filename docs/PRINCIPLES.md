# Principles

1. **KISS.** If a church office administrator cannot describe what a
   feature does in one sentence, it does not ship. Complexity lives in the
   engine (imposition, fields, print geometry), never on the user.
2. **The user is an administrator, not a designer.** Church office staff
   and volunteers, working fast, often on a deadline, often covering for
   someone else. No layout jargon on the main path: no "frames", "stories",
   "imposition" in the guided flow. Say "panels", "this week's text",
   "print copy".
3. **Guided, with the next step always visible.** The home screen is the
   weekly flow: Start, This Sunday, Announcements, Check, Print. One
   primary action per step. The app suggests the next step after every
   action (after Print: "Start next Sunday").
4. **Automate the recurring.** Dates advance themselves, standing content
   carries over, weekly content clears itself, fields fill the cover and
   the order of service. The user's job shrinks to what actually changed.
5. **Never lose work, never surprise at the printer.** Autosave, atomic
   saves, plain-language preflight, and print output that matches the
   preview. A Saturday-night failure is a release blocker.
6. **Imports are rescues, not magic.** Publisher (.pub), InDesign (via
   IDML), PDF, and our own .obd all open here, with honest fidelity
   labels. PDF analysis can draft a template from an old bulletin PDF;
   a human always confirms before it becomes the template.

7. **Build isolation.** OpenBulletin never shares space with Jacob's other
   project (the EV Classic Mac harness): separate repo, separate chats,
   separate desktop runner, container, and caches. No cross-contamination.
