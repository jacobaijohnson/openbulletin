# Lens review, Wonder lane 1: the two people, walked through their week

Reviewer note: this lane read the code (crates/ob-app/src/main.rs,
crates/ob-layout/src/lib.rs) alongside the packet, per its brief.
Where the packet and the code disagree, this report follows the
code and says so. Line numbers are from HEAD b264936.

## Packet claims the code corrects

Read these first; three findings below depend on them.

1. **In-place caret editing exists.** The packet says it "does
   NOT exist yet". The code draws a real caret text editor
   exactly over the frame being edited (main.rs, the "inline
   editor, standing exactly where the frame is", ~line 2966).
   What is missing is fidelity inside that editor, not the
   editor itself. The debt is smaller than the packet admits,
   and different in kind.
2. **Hymn numbers are not fields.** The packet lists fields as
   "date, Sunday name, readings, hymns by number". The code's
   only fields are the church profile plus sunday_date,
   sunday_name, and ONE reading (ob-layout/src/lib.rs,
   build_template). The hymn numbers 432 and 365 are literal
   characters inside the Order of Service story text, and the
   sermon line is the literal text "Sermon: {Church.pastor}".
   There is no sermon title anywhere in the model.
3. **Printer memory is per typed name, per machine.** The
   packet says the two-sided answers are "remembered per
   printer". The code keys them to a free-typed name in a text
   field, defaulting to the invented string "Church office
   printer" (main.rs line 1202), stored in settings on that
   one computer.
4. **Print freshness is per session.** The packet states the
   freshness warning without its limit: print_fingerprint is a
   runtime field (main.rs line 966), never written to disk. A
   restart forgets that any print copy was ever made.

## 1. Verdict per persona

**The office administrator: yes, mostly, with one weekly detour
and one unguarded classic error.** Her core loop is genuinely
served: one open door, last bulletin remembered, announcements
as a plain text box, checks that name the box and jump to it,
an honest print step, Next Sunday that cannot overwrite last
week's file, recovery that speaks plainly. The philosophy holds
best exactly where she lives: Forgiveness (armed discards,
journal, undo), Feedback (status lines that count ended
announcements out loud), WYSIWYG at rest (the canvas and the
sheet preview render shaped text in the real faces). She is
betrayed twice a week. First, the words she changes most often
after announcements, the hymn numbers and the sermon line, are
not on any step; they live inside a story on the Pages canvas,
which she must discover, page through, and double-click into.
Second, nothing anywhere checks that the date on screen is
this week's date (finding F2).

**The pastor: not yet unaided for the vacation week.** His two
regular jobs have no home in the flow. There is no sermon title
to write: the template prints "Sermon:" followed by his name,
and changing that means canvas surgery on a standing story,
the least guided surface in the app. Proofreading, the job the
congregation holds him responsible for, has no step and no
full-size reading surface (finding F5). Arriving cold after
months away, he also cannot tell what state the bulletin is in
(finding F4): the app remembers nothing across restarts about
what was finished, checked, or printed. He can succeed with the
administrator's phone number. That is the system churches have
today, and replacing it is the point of the app. Where the
philosophy does hold for him: the recovery window, the armed
discards, and the refusal wording in the learning door are all
written for a frightened occasional user, and they work.

## 2. Friction findings, ranked by Sunday-deadline damage

**F1. The week's most-changed words live outside the flow.**
Hits: both personas, every week. Principle: 3 (See-and-Point),
4 (guided), 9 (mental model).
Moment: Thursday, the administrator has the new bulletin open
at This Sunday. The step offers date, name of the Sunday, and
one Scripture reading field (main.rs 4985-5038), plus a note
that "the cover and the order of service fill themselves from
these". They do not. This week's hymns are different, and the
sermon has a title. Neither is here, and nothing on the step
says where they are. She must open Pages (a top-bar button, not
a step), page to the Order of Service panel (2 of 4, or one of
16 in the booklet), double-click the box, and edit lines like
"Hymn of Praise[TAB]432" in place. The Announcements step's own
guidance calls the order of service a standing item that
"carries over on its own", which is true of its skeleton and
false of its hymn numbers. For the pastor it is worse: his one
routine contribution, the sermon title, has no slot at all. If
he types over the sermon line on the canvas he can silently
destroy the {Church.pastor} field token, and next week's
profile change stops reaching the page.

**F2. Nothing guards the stale date, the most common bulletin
error there is.** Hits: both; the pastor on vacation duty most.
Principle: 6 (never surprise at the printer), 4 (Feedback).
Moment: the administrator opens last week's file directly (not
through the last-week door), updates the announcements, prints.
Check reports "OK: bulletin date is Sunday, October 11, 2026"
because run_checks tests only that a date exists (main.rs
1497-1504). Two hundred copies carry last week's date and, with
it, last week's Sunday name and readings, all internally
consistent, all wrong. The pastor's version: the Start headline
says "Working on: Sunday, October 11" and he has no way to
know whether that is the coming Sunday or three weeks stale;
start_status (main.rs 846) never compares the date to today.
The app knows today's date (it uses it for the date buttons).
It just never says the sentence.

**F3. The typing moment breaks WYSIWYG.** Hits: administrator
weekly, pastor whenever he edits. Principle: 6 (WYSIWYG),
9 (conform; Publisher shows real text while typing).
Moment: idle, the canvas renders the story shaped, in the real
font and face, exactly as it will print (main.rs ~2610). The
second she double-clicks, those shaped lines are hidden and an
egui editor stands in, drawn in the interface font at the FIRST
paragraph's size only (main.rs 2983-3001). A cover box with a
24pt name and 11pt lines types at one size in the wrong face.
Dot leaders vanish: the hymn lines' tabs render as plain gaps
while typing, so the number she is aligning to the right edge
is not visibly aligned to anything. Line breaks while typing
are not the printed breaks, so she cannot judge fit until she
clicks Done and visits Check. The packet undersells what
exists (the editor is in place, with a caret, Escape cancels);
the remaining debt is that the app is least truthful at the
exact moment the user is writing.

**F4. The state of the work does not survive, and barely
exists.** Hits: pastor (cold start), administrator (returning
after interruptions). Principle: 4 (Feedback), 6 (Perceived
Stability).
Moment: Saturday, the pastor opens the app to finish what the
administrator started Thursday. Start tells him "Working on:
Sunday, [date]" and one next line chosen from exactly three
conditions: no church name, no date, announcements empty
(main.rs 846-885). It cannot say "announcements are in but
Check has not been run since the last edit", or "a print copy
was made Friday and the text changed after". Check results are
recomputed on arrival and forgotten at close; the print
fingerprint is memory-only (packet correction 4). The step
list deliberately shows no done marks. So the one question a
handoff asks, "what is left?", has no answer anywhere in the
app. The administrator hits the mild version weekly: after the
phone, the doorbell, and the copier, she re-derives her place
by walking the steps again.

**F5. Proofreading has no home, and "Check" does not check
words.** Hits: pastor primarily. Principle: 2 (serve the real
task), 4 (the flow omits a step its accountable user performs).
Moment: the pastor wants to read the whole bulletin through,
as the congregation will. His surfaces are the canvas one
panel at a time (16 pagers for the booklet), a print preview
capped at 300px tall in a scroll box so the buttons stay
reachable (a deliberate trade, main.rs 2074), and a reading
copy PDF labeled "for email and the website". None is
presented as the proof. The step named Check verifies fit,
fonts, dates present, unprintable characters, and never the
words themselves; there is no spell check (it sits in the
v0.5 roadmap, unshipped, and the packet does not claim it).
A misspelled sermon title and a transposed hymn number pass
every check the app runs. The philosophy says the screen must
match the printed sheet; for the person responsible for what
the sheet SAYS, the app checks everything except the saying.

**F6. A different printer is a research project at print
time.** Hits: pastor (proof at home), any new office printer.
Principle: 9 (mental model: a printer is a device, not a name
you must invent identically twice), 5 (automation).
Moment: the pastor opens his home install. Its settings know
no printers at all; the name field offers "Church office
printer", a place he is not. The checklist hedges: "we have
not learned the Church office printer yet", and the remedy is
the full test-sheet exercise, on print morning, for a proof he
wanted in two minutes. On the office machine the same trap is
quieter: retype the name with one letter different and the
learned flip is silently not found. The packet's "remembered
per printer" is true only after a human has done the naming
perfectly, twice.

**F7. First open presumes week two.** Hits: both, once, at the
adoption moment. Principle: 9 (mental model), 1 (KISS).
Moment: a brand-new install starts on a LetterBifold template
dated TODAY, whatever weekday today is, named "Sunday"
(main.rs 1167-1176); the headline reads "Working on: Sunday,
[Thursday's date]". Start's guidance line says "Everything
else starts from last week's file" to a user who has no last
week's file. The doors below handle the real first-run jobs
well (old bulletin in, learn from a pile, pick a shape, church
profile at the bottom of the step), but the first two lines on
screen describe a church that already uses the app. For the
reference church, whose shape is the 16-page booklet, the
default shape is also the wrong one until the Analyzer or a
shape pick corrects it.

**F8. The recovery window can be worked around, then
overrides.** Hits: either, rarely. Principle: 6 (Forgiveness
cuts both ways).
Moment: the recovery window is a floating window over a live,
editable fresh document (main.rs 1971). Nothing requires an
answer first. A user can start typing in the template behind
it, then click "Bring it back" later and watch the journal
document replace their new work with history cleared. The
armed discard button is well built; the window's modelessness
is the one place Modelessness works against the user.

**F9. The first Save proposes the same name every week.**
Hits: administrator weekly. Principle: 5 (automate the
recurring).
Moment: save_flow proposes "bulletin.obd" for any unsaved
document (main.rs 3995-4000). In a folder of weekly files the
name carries no date, so the archive depends on the
administrator inventing and typing a naming scheme, the exact
clerical work the app exists to absorb. Next Sunday's status
says "Save it under its own name" and the dialog then declines
to suggest one.

**F10. The fit check is blind sideways.** Hits: administrator,
rarely, then at the printer. Principle: 6 (WYSIWYG honesty).
The Check fit pass counts any line containing a tab as exactly
one line regardless of width (main.rs ~1582) and estimates
overflow in fixed 14pt units. A long order-of-service line can
run off the right edge of its box with every check green. The
canvas shows the truth; Check, the step she is told to trust,
does not.

## 3. Where the two personas diverge under the same principle

**Forgiveness (6).** The administrator needs forgiveness that
is FAST: deep undo, arms she has already mentally approved,
nothing that asks twice. The pastor needs forgiveness that is
LOUD: he does not know which acts are dangerous, so narration
matters more than speed. Same mechanisms serve both if the app
spends its words where the risk is: the armed labels and the
Next Sunday status already do this; extend the pattern by
having Next Sunday NAME the announcements that ended (it
currently only counts them). The administrator skims a line
she expected; the pastor can verify a list he feared.

**Guidance (4).** She wants the flow out of the way; he wants
more of it. The resolution is not two modes. It is guidance
computed from the DOCUMENT, not the user: a standing "where
this bulletin stands" line (change 3) is invisible overhead to
her (she already knows; it confirms) and a map to him. Recency
can carry the rest: the app knows when a file was last opened,
and a cold file can earn one extra sentence ("This bulletin
was last worked on 6 weeks ago") without any persona setting.

**Automation (5).** She trusts carry-over because she watches
it weekly. He fears it because he cannot tell what it did.
Automation that reports itself in counts (already shipped for
Next Sunday) is the bridge; automation that is silent is where
they split. Keep one rule: every automatic act names what it
carried, cleared, and ended, in the status, every time.

**See-and-Point (3) vs recall.** The keymap presets and step
shortcuts are hers; the buttons are his. The shipped design
already layers these correctly (presets only set defaults;
every action also exists as a labeled button). Guard it: no
future feature may ship keyboard-only.

## 4. The five changes to make first

1. **Put the whole service on This Sunday.** Hymn numbers and
   a sermon title become plain fields beside date, name, and
   reading, resolving into the order-of-service lines; the
   canvas stays the fine-tuning surface. Restores principles
   3, 4, and 9, and ends the weekly canvas trip (F1). The
   sermon line stops being a field token a pastor can break.
2. **Guard the date like the app means it.** Check and the
   Start headline compare the bulletin date to today and say
   "That was last Sunday" or "That date has passed" in the
   problem color; This Sunday says it beside the field.
   Restores 6 and 4 (F2). Cheapest change on this list, aimed
   at the costliest error.
3. **Give the bulletin a memory of its own state.** A "where
   this stands" line computed from the document (date is the
   coming Sunday, announcements answered, Check passed since
   the last change, print copy current) and persisted with the
   journal/settings so it survives restarts, shown on Start
   and repeated at Print. Restores 4 and 6 (F4), and it is the
   whole handoff fix in one surface.
4. **Make the typing moment truthful.** The canvas editor
   renders each paragraph in its real face and size and keeps
   the dot leaders visible while typing; until that lands, a
   live fit readout under the editor ("fits" / "about N lines
   too many", the number Check already computes). Restores 6
   at the moment of writing (F3).
5. **Give proofreading a door and an honest name.** Print
   gains "Open a copy to read through" (the reading PDF,
   labeled for proofing, not only for email), and Check's
   guidance line says it checks printing, not spelling, until
   a spell check ships. Restores 2 and 4 (F5) without
   pretending the words are verified.

## 5. Questions for the pastor interview

1. Tell me about the last bulletin you had to produce alone.
   Where did you get stuck, and who did you call?
2. When you proofread a draft, what do you actually read for?
   Names, hymn numbers, times, theology? What error has
   reached print that you still remember?
3. When in your week do you know your sermon title, and who
   types it today? Has it ever been wrong or missing in print?
4. How do announcements reach the bulletin, and who decides
   one has run long enough? How often does something print
   after it should have ended?
5. What changes on the big Sundays (Christmas, Easter,
   baptisms, funerals), and who builds those bulletins?
6. When the printed copies come off the copier, what do you
   physically check? Fold, page order, upside-down backs?
7. If your administrator left tomorrow, what knowledge walks
   out with them that nobody else has?
8. Do you ever read or print a proof away from the office?
   On what device, in what program?
9. Whose names must never be misspelled, and are there things
   (prayer lists, giving, pastoral care) you keep OUT of the
   printed bulletin on purpose?
10. What do members actually do with the bulletin after the
    service, and what do the homebound or large-print readers
    get today?
