# Principles

1. **KISS: Keep It Short & Simple** (Jacob's expansion). If a church office administrator cannot describe what a
   feature does in one sentence, it does not ship. Complexity lives in the
   engine (imposition, fields, print geometry), never on the user.
2. **Simple on the surface, powerful underneath.** Every screen must
   pass the ELI5 test: explainable to a five-year-old, in plain words,
   without losing the person reading it. The engine underneath does the
   opposite: real text shaping, real print geometry, no shortcuts that
   print wrong. Easy to use is not the same as weak.
3. **The user is an administrator, not a designer.** Church office staff
   and volunteers, working fast, often on a deadline, often covering for
   someone else. No layout jargon on the main path: no "frames", "stories",
   "imposition" in the guided flow. Say "panels", "this week's text",
   "print copy".
4. **Guided, with the next step always visible.** The home screen is the
   weekly flow: Start, This Sunday, Announcements, Check, Print. One
   primary action per step. The app suggests the next step after every
   action (after Print: "Start next Sunday").
5. **Automate the recurring.** Dates advance themselves, standing content
   carries over, weekly content clears itself, fields fill the cover and
   the order of service. The user's job shrinks to what actually changed.
6. **Never lose work, never surprise at the printer.** Autosave, atomic
   saves, plain-language preflight, and print output that matches the
   preview. A Saturday-night failure is a release blocker.
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
   feature is wrong, however elegant the engine is.
