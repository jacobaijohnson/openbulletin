# UI Audit, round 2 — Flow (blind Wonder lane, 2026-10-15)

Read-only walk of crates/ob-app/src/main.rs at HEAD 0eaeea3 against
PRINCIPLES.md, README.md, and the workflow adjudication. Four
journeys traced in code. Headline: the skeleton is sound; the
failures cluster in three places: things that look applied but are
not (the date), doors that look equal but are not (the two Open
buttons), and state the app knows but never says out loud (sample
data, a second instance, a stale Check).

## Journey (a): brand-new office

**MAJOR — The first screen is a filled-in fictional bulletin, and
nothing says so.** App::default() builds a complete bulletin for
"Grace Lutheran Church", "123 Main Street", "Sundays 9:30 AM"
(main.rs:986, 1003). No first-run flag in Settings. start_status
treats any non-empty church name as done (746–770); Check reports
"OK: church name is set." Print without importing and another
church's name lands on the cover. Fix: ship the first-run document
with a blank church name and address; existing machinery then
guides correctly. One default changed, no new UI.

**MAJOR — The most visible Open button cannot open the files a new
office has.** Top-bar "Open bulletin..." (open_flow) filters to
.obd only (3576–3581); PDF/Publisher files are not selectable. The
all-formats door is partway down the Start step (4414–4428). Fix:
point the top-bar Open at open_old_bulletin's routing (handles .obd
at 3127). One door, two entrances, same powers.

**MINOR — A long import looks frozen.** While a rescue runs, Start
shows static "Reading your old bulletin..." (4412–4413): no
spinner, no time expectation, no cancel; the LibreOffice route can
take two minutes. Fix: spinner plus "Big files can take a minute.
Your work on screen is safe while you wait."

**MINOR — A failed native Publisher read blames the wrong thing.**
When LibreOffice is absent and the native reader fails, the error
returned is the "LibreOffice is not installed" message. Fix: when
the native reader was tried and failed, the first sentence becomes
"That Publisher file could not be read here."; keep both ways
forward.

## Journey (b): the weekly regular

**BLOCKER — A typed date silently never applies.** On This Sunday,
the date applies only when focus is lost while Enter is pressed,
or via "Set date" (4523–4524). Every neighboring field applies on
plain focus loss (4463–4560). Type the new date, click Next, and
the typed date is discarded silently; the bulletin keeps last
week's date and Check certifies it ("OK: bulletin date is {old}").
Fix: apply the date when leaving step 2 (goto_step already saves
announcements leaving step 3, 1117–1125) and on focus loss like
its neighbors; keep the unreadable-date refusal.

**MAJOR — One click on a shape name replaces the whole bulletin.**
Start's preset list rows call rebuild() immediately (4456–4458),
no hint picking is destructive; every other destructive path uses
a two-click arm. Fix: first click arms ("Click again to start
fresh with this shape. Your bulletin stays undoable."); second
click rebuilds.

**MINOR — Next Sunday by accident never says Undo exists.** The
success message (1492–1508) unlike rebuild's never mentions Undo.
Fix: append "Undo brings this week back."

**MINOR — Real failures whisper while trivia gets the banner.**
fail() (527–532) is bypassed by apply_date's refusal (584), the
keep-through refusal, and next_sunday's error branch (1509, raw
engine wording). Fix: route all three through fail().

## Journey (c): the stand-in

**MAJOR — The app remembers printers and keys, but not the
bulletin.** Settings holds printers and keymap only (40–54). No
recent file, no resume offer; the real bulletin needs filesystem
browsing by name. Fix: remember the last opened .obd path; on
Start: "The bulletin this office worked on last: {name}. Open it."

**MAJOR — Check's fix buttons do not take you to the problem.**
Only character findings get a per-item button, and for non-
announcements stories it opens Pages at the last-viewed panel
(4626–4627), not the named panel/box. Fit, edge, and missing-font
notes have no per-item button (4636–4644). The adjudication
recorded go-fix buttons as shipped; they are shipped for one
finding type and do not navigate. Fix: carry panel (and frame)
index in each note; the button opens Pages on that panel with
that box selected. Data is in hand in run_checks (1300–1475).

**MINOR — Check results go stale after a Pages-window fix.** Notes
compute on arrival at Check and on "Check again" (1123,
4630–4632) only. Print recomputes character findings fresh (4670).
Fix: re-run checks when the Pages window closes while on Check, or
stamp the list as Print stamps its copy.

**MINOR — The step list claims work is done that was only walked
past.** Sidebar markers are positional ("done -" for any step
above current, 4358) though start_status derives honest state.
Fix: drop "done -" where document state does not support it, or
drive markers from the same derivations.

## Journey (d): interruptions

**MAJOR — Through the old-bulletin door, the unsaved-work guard
fires after the file pick, and eats it.** open_old_bulletin routes
.obd picks through open_guard only after the dialog closes
(3127–3131): pick, dialog vanishes, nothing opens, explanation is
small text saying click again and re-browse. Both other doors
guard before the dialog (3576; 4385–4391) and one relabels itself;
the Start door's button does not (4414). Fix: guard before the
dialog for the .obd case, or carry the picked path through the arm.

**MAJOR — A second app window silently loses its autosave.**
startup() detects a live peer (1571); journal_decision then Holds
forever (5032–5036); peer_alive appears in no UI code. A second
window is fully editable, never journaled, never recovered, with
no sentence saying so. Fix: one startup status line in the second
instance ("OpenBulletin is already open in another window. Work in
this window is not being kept safe; close one of them."), or
refuse to start it with that message.

**MINOR — Review-window armed state is half-labelled.** First
accept relabels when armed; "Use it and arrange the pages" never
changes (3290, 3537). Fix: relabel both when armed.

**MINOR — A second import can replace an unanswered review.**
Start's import button is live once a review is open; a new run
makes poll_import overwrite import_review (3177), discarding the
draft (and leaking its temp folder). Fix: disable door buttons
while a review is pending, or route the new pick through the arm.

**MINOR — "Announcements saved" overpromises** (status at ~4593
when words are only in the on-screen document; cross-ref menus
lane's Save-word finding). Fix: "Announcements are on the page."

## Do not change

- The one open door with sniffing fallback (3117–3168).
- The recovery window (1704–1768): the best flow in the app.
- The open guard's re-arm on changed work (1623–1645); the flaw
  is where one door calls it, not the guard.
- Next Sunday's file safety (1488–1508).
- Review windows' Accept/Edit/Cancel shape and honesty notes.
- Print step discipline (verbatim printer instruction, freshness
  warning, character warnings repeated).
- Escape cancels canvas typing (2389–2392); step guidance text
  and the Next button naming the next step (~4330).

Counts: 1 BLOCKER, 6 MAJOR, 7 MINOR.
