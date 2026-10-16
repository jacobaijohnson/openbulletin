# Lens review, Wonder lane 2: the design philosophy vs. the shipped code

This lane read the packet in full, then checked the packet's
"as shipped" claims against the code
(`crates/ob-app/src/main.rs`, `crates/ob-layout/src/lib.rs`).
Where the code and the packet disagree, this report follows
the code and says so.

## Where the code disagrees with the packet

- The packet says fields include "hymns by number" filling
  the service order. **The code has no hymn field and no
  sermon field anywhere.** The This Sunday step edits exactly
  three things: the date, the Sunday name, and one scripture
  reading. Hymn numbers are literal text inside the standing
  order-of-service story (`"Hymn of Praise\t432"`), and the
  sermon line is `"Sermon: {Church.pastor}"` — it prints the
  pastor's name from the profile. There is no sermon title in
  the app at all. (Finding 1.)
- The packet lists import badges as "Matched / Check this /
  Picture only". On screen the first badge reads **"Good"**,
  and the review buttons read "Use this draft", "Use it and
  arrange the pages", "Cancel" — not Accept / Edit / Cancel.
  The substance matches the packet; the words are better than
  the packet says. Noted only so the packet stops drifting.
- The packet describes whole-box editing as "click a box,
  type in a text area". The code is both better and worse
  than that: the editor overlays the frame in place, but it
  shows the **raw story text** — field tokens like
  `{Church.pastor}` appear literally — in a plain interface
  font at a clamped size, whatever the frame's real font is.
  (Finding 6.)
- The packet's "one primary action per step" holds on Print
  (only "Open the print copy" is strong). On Start it does
  not: five doors of equal visual weight, mitigated by the
  status headline that names the next thing needed.

Everything else load-bearing in the packet checked out in
code: the single open door with content sniffing, the
last-bulletin door that hides itself when the file is gone,
Next Sunday setting the path to none so a later Save cannot
overwrite last week's file, the journal tick saving the
announcements buffer before snapshotting (typed announcements
are journal-protected within about two seconds), the
print-copy freshness fingerprint, per-printer flip memory,
and Check findings that carry a target and a "Fix this"
button landing on the named panel and box.

## 1. Verdict per persona

**The office administrator: yes, unaided, with one weekly
detour through the least guided surface in the app.** Her
path holds almost everywhere the philosophy promises:
one door for every old file, a Start headline that says which
Sunday this is and what is needed next, announcements as one
plain box with the buffer journal-protected as she types,
Check findings that walk her to the exact box, a Print step
whose one strong button makes the copy, and a Next Sunday
whose result message counts what ended and names the way
back. The philosophy betrays her at the two changes she makes
every single week after the date: the hymns and (for the
pastor) the sermon title, which have no field, no step, and
no check — only raw text inside a story the app keeps
praising for "carrying over" (Findings 1, 5).

**The pastor: not yet, for the vacation week; yes, for
proofreading.** As a proofreader he is well served: Check
speaks plainly, findings jump to their boxes, the sheet
preview renders in the real font and face, and the freshness
warning catches the classic "printed the old copy" error. As
a solo producer after months away, the philosophy's own
principles predict his failures in advance: a first screen
that shows him a plausible sample bulletin that is not his
church's bulletin (Finding 4), a "last week's" door that
rejects the predecessor's Publisher file with an .obd error
(Finding 4), nowhere to put the one thing he always brings —
the sermon title (Finding 1), a printer memory keyed by a
typed name that can silently hand him the office printer's
flip answer for his home printer (Finding 2), and an Escape
key that can erase a long typing session with no way back
(Finding 3).

## 2. Friction findings, ranked by damage on a real Sunday deadline

**Finding 1 — The weekly four are really a weekly two: hymns
and the sermon title have no home.**
*Principles 3, 4, 9 (administrator, not designer; guided;
mental model). Hits both personas, the administrator weekly,
the pastor at his one regular contribution.*
The packet's own persona description names the weekly swaps:
date, readings, hymns, sermon title. The This Sunday step
offers the date, the Sunday name, and one reading. To change
"Hymn of Praise 432" to this week's number, the administrator
must open the Pages window, double-click the order-of-service
box, and edit raw text in which the hymn number sits after a
literal tab and the sermon line reads `Sermon: {Church.pastor}`.
Worse, the app actively steers her away from that box: the
Announcements step's guidance says the order of service
"carries over on their own", and Check congratulates her that
it "is present and carries over from last week". The order of
service is classified Standing, and its most volatile contents
— two hymn numbers — are entombed inside it. The pastor's
sermon title fares worse: it does not exist. What prints after
"Sermon:" is his own name, from the church profile. A pastor
who wants "Sermon: The Sower and the Seed" in the bulletin
must perform the raw-text edit himself and take care not to
disturb the token. This is the largest gap between the design
law and the shipped app, and it is hit every week, not at the
edges.

**Finding 2 — Print morning ends in another program, and the
printer memory can learn the wrong printer under the right
name.**
*Principles 6 and 4 (WYSIWYG; feedback). Hits both; the
pastor printing a proof at home is the sharp edge.*
"Open the print copy" writes `openbulletin-print-copy-<pid>.pdf`
to the temp folder and hands it to the operating system's PDF
viewer. The app never prints. The checklist that makes the
copy print correctly — paper size, "Actual size, never Fit to
page", the learned flip instruction, copies — sits in our
window while every one of those settings lives in the viewer's
dialog. If the viewer defaults to Fit, an 16-page booklet's
folds land in the wrong place and nothing in OpenBulletin can
see it happen; the freshness fingerprint certifies the file,
not the printout. Compounding it, printer memory is keyed by
a name in a free-text box whose initial content is the literal
string "Church office printer". A pastor who learns his home
printer's flip without renaming the box stores the home
answer under the office printer's name; the next office print
checklist then quotes the wrong instruction with full
confidence ("the checklist quotes the very instruction the
wizard learned" — the code is proud of this consistency, and
it is consistency in service of a wrong fact). One text box
is the only thing standing between two physical printers and
a confidently wrong checklist.

**Finding 3 — Escape throws away a whole typing session, and
nothing can bring it back.**
*Principle 6 (forgiveness). Hits the pastor hardest; any long
canvas typing session is exposed.*
Canvas editing is one session per box: double-click, type,
click away to commit. Escape cancels — deliberately, per a
recorded decision citing office convention — but
`cancel_edit` drops the session's start snapshot without
recording an undo step, and the top-bar Undo is disabled
while editing. The amount at risk is unbounded: ten minutes
of sermon notes typed into a box, one reflexive Escape (to
dismiss a menu, out of spreadsheet habit), and the status
line's "Typing thrown away. The page keeps the words it had."
is the only memorial. The office convention governs a cell
edit, a bounded thing; here it governs an unbounded one, and
the screen never says, before the fact, that Escape here
means *all of it*. Forgiveness in this app is otherwise
exemplary — armed discards, journal, freshness — which makes
this one unrecoverable gesture stand out.

**Finding 4 — First open shows a sample that looks like a
bulletin, and the two "last week" doors disagree about what
last week is allowed to be.**
*Principles 9 and 6 (mental model; perceived stability).
Hits the pastor on vacation and the brand-new administrator —
exactly the two people with no habits to protect them.*
A fresh launch opens a letter bi-fold sample (not the
reference church's booklet), Sunday name literally "Sunday",
the date set to *today's* date unsnapped (a Thursday setup
visit puts a Thursday date on the cover), placeholder
announcements in place. The Start status line does say a
church name is needed, and the doors are all present — but
the document on screen is a plausible bulletin, and a man who
opens the app "to do the bulletin" can start typing into the
sample before he understands it is one. The remembered
last-bulletin door softens this only on a machine that has
run the app before with that file. Below it, "Start from last
week" and "Bring in an old bulletin" sit side by side: the
first accepts .obd only and, handed the predecessor's .pub —
the file a new administrator is holding, per the packet —
answers "Could not open that file. Pick a bulletin file
ending in .obd." The one-door principle is real, but it has
a gated side entrance with the same promise on the sign.

**Finding 5 — Check can certify a bulletin that is wrong in
the way bulletins are actually wrong.**
*Principle 4 (feedback). Hits both; damage lands on Sunday
morning, in print, in front of the congregation.*
Check verifies presence and printability: words fit, fonts
exist, boxes keep off the fold, characters can print, a date
exists, an order of service exists. It never compares the
bulletin's date against the coming Sunday. The single most
common real bulletin error — last week's date carried forward
— passes Check in silence, because the date field is filled,
which is all Check asks. (The This Sunday step has a "Use this
Sunday's date" button; Check simply never checks.) The same
blindness covers the hymns, necessarily: since hymn numbers
are not data (Finding 1), no check can notice they are last
week's. Check's "OK:" lines therefore certify more than they
know, which the design law's own honesty standard — downgrade
the claim or upgrade the behavior — forbids.

**Finding 6 — Double-clicking a box shows a different text
than the page shows.**
*Principle 6 (WYSIWYG). Hits whoever edits on the canvas.*
The canvas renders resolved, shaped text: "Sermon: Pastor
Lee", the church's name filled in, dot leaders after hymn
names. Double-click and the same box becomes raw story text —
`Sermon: {Church.pastor}` — in the interface's plain font at
a clamped size unrelated to the frame's. Two failure modes
follow. The mild one: the user cannot judge fit while typing,
because the editing text neither wraps nor measures like the
printed text (the fit check will tell her later, on another
step). The bad one: a user "repairs" the strange-looking
token — deletes `{Church.pastor}` and types the pastor's name
as fixed text — and silently breaks the field for every future
week and for the day the pastor changes. Nothing on screen
explains that braces mean "this fills itself in".

**Finding 7 — Announcements are one bucket wearing a plural
name.**
*Principle 5 (automate the recurring, under user control).
Hits the administrator.*
The step's text box edits only the *body* paragraphs of the
*first* frame marked Weekly, and the single "Keep through"
date governs all of them together. A potluck announcement
that runs three weeks and a one-week choir note cannot
coexist without retyping one of them weekly — the automation
that exists (keep-through) is all-or-nothing across the set.
On the 16-page booklet, every content page is a Weekly frame
in the shipped template, and Next Sunday clears body text in
all of them, while its message reports only that "Weekly
announcements were cleared" — a pastor who put the liturgy
notes on page 9 learns on the following Thursday that they
were "announcements". The words the app uses for what it
clears do not cover what it clears.

## 3. Where the personas diverge under the same principle

**Forgiveness.** The administrator needs forgiveness to be
fast: she is interrupted mid-task, works weekly, and resents
ceremony — the two-click arms are already at her tolerance
limit, and rightly undoable beats rightly armed for her. The
pastor needs forgiveness to be *deep*: after months away his
errors are conceptual (wrong door, wrong file, wrong printer),
not slips, and they surface late. The shipped design mostly
threads this needle well — arms are cheap, undo is total,
 the journal is silent until needed — and the divergence is
best served by keeping one behavior and widening what is
recoverable (Finding 3's fix), not by per-persona strictness.
The one place frequency should change the design: a user who
has not opened the app in months should meet the recovery and
orientation surfaces (Start status, last-bulletin door) at
full strength, which is a function of elapsed time and
document state — both already known to the app — never a
"pastor mode".

**Guidance.** The administrator's guidance should compress:
she knows the five steps; the GUIDANCE sentence is furniture
to her by week three. The pastor needs the same sentence to
expand into orientation: which Sunday is this, whose work is
on screen, what is safe to touch. One app serves both because
the shipped guidance is *state-derived, not user-derived*:
`start_status` reads the document and names the next needed
thing. That mechanism should absorb the divergence — the same
line that tells the administrator "Ready to print" tells the
returning pastor "Next: set the date on the This Sunday step"
— rather than any identity switch. Where the mechanism is
absent (the This Sunday step says nothing about hymns because
hymns are not state), both users go unguided together;
Finding 1's fix is a guidance fix as much as a data fix.

**Automation.** Next Sunday is the sharpest divergence. To
the administrator, its clearing is a promise kept weekly; to
the pastor, an unfamiliar machine deleting words is the
scariest button in the app. The shipped message already
serves both readings honestly (counts what ended, names the
way back, never overwrites the old file). Keep-through cuts
the other way: it is the administrator's memory aid and the
pastor's proofreading hazard, because a kept announcement
looks intentional to fresh eyes. The design answer is not
less automation but automation that *shows its hand at proof
time*: Check should list what is being carried and through
when, so the frequent user's convenience becomes the
infrequent user's checklist. (This is the natural companion
to Finding 5's date check.)

**Printing.** One printer, learned once, is the
administrator's whole world; a second printer at home is the
pastor's. Name-keyed memory silently assumes one person, one
printer, one name typed carefully. The divergence is served
by making printer identity a choice the app presents (learned
printers by name, plus "a printer we have not learned") at
the moment of printing, which costs the administrator one
click a week and saves the pastor from a confidently wrong
checklist (Finding 2).

The general rule the review converges on: **diverge on state,
never on identity.** Frequency, elapsed time, and document
condition are facts the app already holds; "administrator
mode" and "pastor mode" would be two apps wearing one icon,
and the packet's one-UI law is right to forbid them.

## 4. The five changes to make first, in order

1. **Give the weekly changes fields.** Add sermon title,
   hymn numbers (each order line that carries one), and
   plural readings to the This Sunday step as plain labeled
   boxes that fill the order of service the way the date
   already fills the cover. Restores Principles 3, 4, and 9:
   the week's real work happens on the guided path, in office
   words, and the order of service stops being a standing
   text the weekly edits must tunnel into.
2. **Teach Check the calendar.** A finding when the
   bulletin's date is not the coming Sunday ("This says
   October 11; this coming Sunday is October 18") with a
   one-click fix that sets it, and a carried-content list
   naming what Next Sunday kept and through when. Restores
   Principle 4: Check stops certifying the one error
   bulletins actually ship with.
3. **Make the printer a choice, not a typed name.** On the
   Print step, printer identity becomes a pick-list of learned
   printers plus "a new printer"; the flip wizard's answer
   binds to the picked entry, and using an unlearned printer
   says so on the checklist instead of borrowing another
   printer's name. Restores Principles 3 (see-and-point over
   recall) and 6 (the checklist stops quoting a wrong printer's
   instruction as fact).
4. **Bound the Escape.** A canvas typing session's buffer is
   journal-protected like everything else, and Escape on a
   session with substantial uncommitted typing leaves the
   discarded text recoverable through Undo (one Undo restores
   it into the box). Restores Principle 6: no single keystroke
   in the app can destroy unbounded work.
5. **Make the first screen honest about the sample, and make
   the last-week doors one door.** Until a real file is opened
   or the church profile is filled, the on-screen bulletin is
   labeled a starter sample in the Start headline, and "Start
   from last week" routes through the same one open door as
   everything else, so a predecessor's .pub is a welcome file,
   not an .obd error. Restores Principles 9 and 7: the first
   thing a new or returning user sees matches what it is, and
   one promise has one door.

## 5. Questions for the pastor interview

1. Walk me through the last bulletin that went out wrong.
   What was wrong, who caught it, and when — before printing,
   at the copier, or in the pews?
2. When your administrator is on vacation today, what
   actually happens? Do you open Publisher yourself, recruit
   a volunteer, simplify the bulletin, or retype it somewhere
   familiar?
3. The sermon title: when is it final, how often does it
   change after Thursday, and how does the administrator
   learn the final version today — email, sticky note,
   hallway?
4. Hymns: who chooses them, when, and where is the choice
   recorded before it reaches the bulletin? Is there ever a
   week the hymns are the *same* as last week on purpose?
5. Readings: how many does your order actually carry, who
   picks them, and do they follow a lectionary or your
   sermon series? Does the bulletin print the text or only
   the references?
6. When you proofread, what do you read for, in what order?
   Do you proof the screen or a printed sheet, and has a
   mistake ever survived your proof because it *looked* like
   last week's correct text?
7. Announcements: where do they arrive from, in what forms,
   and who decides how many weeks one runs? When something
   should have ended but keeps printing, how does that get
   caught today?
8. The prayer list and names of the sick or grieving: what
   are your rules for what may be printed, and for how long?
   Is there anything that must *never* carry over by default,
   however convenient that would be?
9. What is in the bulletin that exists nowhere in writing —
   spellings of names, standing courtesies, the thing a new
   administrator would get wrong in week one and nobody
   would think to warn them about?
10. Printing and folding: where does it happen, on what
    machine, who folds, how many copies, and what is your
    fallback when the copier fails on Saturday?
11. Funerals, weddings, baptisms: separate documents today?
    Built from what — last time's file, a template, memory?
12. Seasonal services (Christmas, Easter): how different is
    the bulletin really, and who builds it — same person,
    same file, or a rescue operation every year?
13. What would a bulletin program have to do for you to
    trust a printout without reading every line? And what
    would it have to do once for you to never trust it again?
