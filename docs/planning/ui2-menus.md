# UI Audit, round 2 — Menus and Organization (blind Wonder lane, 2026-10-15)

Read-only audit at HEAD 0eaeea3. Coordinator note: findings carry
file:line evidence in crates/ob-app/src/main.rs; the synthesis
(ui2-synthesis) rules on each.

**Verdict.** The skeleton is right and settled law is holding: one
open door, one export door, five named steps, plain words almost
everywhere. Nobody gets lost between steps. The problems are inside
the containers. The top bar is a junk drawer: file actions are split
into two clusters with tools and editing wedged between them, so the
Office File/Edit map that Principle 9 promises does not exist on
screen. The step panel never scrolls, so the longest steps bury
their payoff content at the bottom where a smaller window clips it
away entirely. Two commands compete for the words "next Sunday"
while one command answers to two different names, and the button
most likely to be clicked first, "New bulletin," does not make a
bulletin. No BLOCKERs: the weekly flow completes. Five MAJORs, all
about organization, none about capability.

## Findings

**MAJOR — The step panel does not scroll, and both long steps put
their payoff at the bottom.** Steps live in egui::SidePanel::left
(main.rs:4349); no ScrollArea anywhere in that panel (ScrollAreas
exist at 2151, 3245, 3500, 3804, 3859, 4824; none between 4349 and
4815). Panels clip; they do not scroll. Start (4375–4513) stacks the
status block, three import sections, the shape list, and six church
fields. Print (4647–4805) stacks the sheet preview, checklist, three
PDF buttons, the export door, the whole two-sided-printer section,
and only then "Start next Sunday" (4802). At the minimum window
height (620) the bottom of both steps is unreachable. Same shape as
the first human run's "not resizeable where it should be."
Fix: wrap the step panel's contents in a vertical ScrollArea.

**MAJOR — "New bulletin" does not make a bulletin.** The top-bar
button (4280) and Ctrl+N (4252–4259) only navigate to step 1 and
post a note. The action is "Or start fresh: bulletin shape" (4454),
the last of Start's four sections, fired from clickable list rows
(selectable_label, 4459–4464) where comparable actions are buttons.
Fix: name the section "Start a new bulletin" with the shapes as
buttons, or make the top-bar button open that section directly.

**MAJOR — "Next" vocabulary collision.** "Make next week's bulletin
from this one" (4566) and "Start next Sunday" (4802) are the same
command (next_sunday()). The "Next Sunday" button on This Sunday
(4536) only sets the date +7 days. Add the top bar's "Next:
Announcements" (4333) and "next" carries three meanings, two on one
screen. Fix: one name for advancing the week everywhere ("Start
next Sunday"); rename the date button "Use next Sunday's date."

**MAJOR — Top bar grouping matches no mental map.** Order today
(4279–4327): title, New bulletin, Pages, Fonts, Keys, separator,
Undo, Redo, Open bulletin..., Save bulletin, separator, step
readout. File actions split across both groups.
Fix: regroup in place: New bulletin, Open bulletin..., Save bulletin
| Undo, Redo | Pages, Fonts, Keys. No menu bar (see do-not-change).

**MAJOR — Print step has five ways to make a file and no single
primary action.** "Open the print copy" (4674, plain), "Save print
copy (PDF)" (4692, strong), "Save reading copy (PDF)" (4712), "Save
a copy for another program..." (4733), "Save a test sheet (PDF)"
(4772), plus strong "Start next Sunday" (4802). Instructions point
at the print-copy path but emphasis sits on saving a file. Related
mislabel: saving a reading copy also sets print_fingerprint (4726),
so the freshness warning can claim a print copy exists when only an
email copy was made. Fix: "Open the print copy" as the one strong
primary; group the keep-a-file actions under one plain heading; set
the fingerprint only from print-copy actions.

**MINOR — The church profile has no heading and no home.** Six
church fields begin at 4462 right after the shape list, explanation
trailing at 4512. Fix: one heading: "Your church (prints on the
cover)."

**MINOR — "Save" is six verbs wearing one word.** Save bulletin,
Save announcements (4584, an internal buffer), plus four file
saves. "Save announcements" teaches users typing is lost unless
they save, in an app that autosaves. Fix: drop the button; the
status line can say "Announcements saved" when the write happens.

**MINOR — Check step heading disagrees with its name.** Step is
"Check" everywhere but its panel heading is "Before you print"
(4610). Generic fix trio (4635–4643) is vaguer than per-finding
buttons (4623). Fix: heading "Check"; delete the generic trio.

**MINOR — Fonts window is five jobs in one long scroll.** Library,
"Put a font to work" (3895), add files, catalog search, starter
pack, fonts on this computer (3842–4200). The style-font control
(the weekly font task) hides mid-window. Fix: rename that section
"Which font each kind of text uses," move it under the library list.

**MINOR — "Keys" names nothing; MODES.md promises organization the
app does not have.** Button (4294) and title (3773) say "Keys";
"Keyboard shortcuts" appears once open (3780). MODES.md still
promises Simple/Full workspaces, vocabulary aliases, tooltips; the
app has none (zero on_hover_text calls). Fix: "Shortcuts" label;
bring MODES.md back to what shipped or mark ideas unbuilt.

**MINOR — Vocabulary/consistency drift.** (a) "panels" (Principle
3) vs Pages/page on screen (2056; KEY_ACTIONS 61–62). Pick the
office word "pages" on the surface. (b) Pages/Fonts buttons only
open; keyboard actions toggle; Keys list says "(open or close)"
(58–59). (c) Pages help text hardcodes default keys ("Ctrl+C",
2787; 2113) though keys are customizable. (d) No Help/About/version
string anywhere; "which version are you running?" has no answer.
(e) The open file's name appears nowhere in the chrome.

## Do not change

- No File/Edit menu bar: hiding commands hurts Principle 4 users.
- One open door and one export door: settled, correct.
- Right-click canvas menus (2732–2900): Principle 9 working.
- "Leave unsaved work and open?" in-place confirm labels.
- The recovery window's words (1710–1747).
- The guidance line under the top bar (4347) and the clickable
  step list with current/done markers (4352–4372).
- The Keys window's honesty mechanics (Custom naming, 3780–3786).
