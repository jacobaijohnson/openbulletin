# The people, and their journeys

Written 2026-10-15 from the project's record and the four-lane
lens review (docs/planning/lens-*.md, synthesis in
lens-synthesis-2026-10-15.md). This is the identity layer the
design law points at: PRINCIPLES.md says who the user is in
one line; this document says it in full, so builds and audits
can test against real weeks instead of abstractions.

## The identities

**The office administrator (primary).** Church office staff or
a steady volunteer, thinking in Microsoft Office and Publisher
habits. Produces the bulletin weekly, usually Thursday or
Friday, under time pressure, interrupted by the phone, the
doorbell, and the copier. Her path is well worn: open last
week's, change the date, the Sunday name, the readings, the
hymn numbers, the sermon title, replace the announcements,
check, print, fold. She rarely designs anything. She may be
new, trained only by "the last person showed me once", and
her inheritance is a pile of Publisher files and PDFs. What
she fears: printing 200 copies of a mistake. What she values:
the app remembering what she should not have to.

**The pastor (secondary, intermittent, accountable).** Writes
the sermon title, supplies or approves readings, proofreads
the draft, and answers to the congregation when print is
wrong. Thinks in words and liturgy, not layout. Touches the
app in bursts: a title here, a proof there, and then, when the
administrator is on vacation, the whole production alone,
months after his last visit, possibly on a different computer
and printer. What he fears: a wrong date, a wrong hymn, a
name misspelled, in public, with his name on the work. What
he values: being told what state things are in and what the
machine already did.

**The substitute (the administrator's stand-in).** Jacob named
this person in the workflow review request: a layperson
stepping in for a period, experienced or brand new. For
design purposes the substitute is the pastor's cold-start
problem with the administrator's whole job: zero habits, full
responsibility, one week to learn. Any surface that works for
the substitute works for everyone; the reverse is not true.

The standing rule from the lens review, adopted: **diverge on
state, never on identity.** No pastor mode, no beginner mode.
Frequency, elapsed time, and the document's condition are
facts the app already holds; guidance, summaries, and warnings
scale with those, and the screens stay the same.

## The journeys

**1. The first week ever.** A new office, a pile of old files,
no template. Doors: bring one old bulletin in, or bring a
pile and let the Analyzer draft the template, or pick a
shape. Then the church profile, then the first Sunday. Design
demand: nothing on screen may pretend this is week two (the
starter sample is labeled a sample; the default date is the
coming Sunday, never a weekday). Standing after the lens
round: the doors are good; the sample honesty and date
snapping are fixed in the Journey batch; binding an imported
bulletin's date/readings/hymns to fields by pointing at them
is the known next import phase.

**2. The weekly cycle.** Open (or Next Sunday), This Sunday
(date, name, reading, and now the service lines: hymn numbers
and the sermon title, on the step where they belong),
Announcements (with keep-through dates per story), Check,
Print. Design demand: every automatic act reports itself in
counts and names; Check compares against the calendar, not
just against emptiness. Standing: the cycle shipped in the
five steps; the service-lines editor, the date guard, and the
carried-content list land in the Journey batch.

**3. Print morning.** The sheet preview in the real type, the
printer's learned two-sided answers, one primary action, and
then the honest gap: printing itself happens in the system
viewer's dialog. Design demand: the app walks the user to
that dialog with the exact choices in words (Actual size,
which flip, one test sheet first), because that dialog is
where booklets die. Standing: the checklist and freshness
warning shipped; the dialog guidance note lands in the
Journey batch; printer identity becomes a pick list, never a
name typed from memory.

**4. The handoff week.** The administrator is away; the
pastor or substitute opens the app cold. Design demand: the
first screen answers three questions without being asked.
Which Sunday is this? What is already done? What is left?
Standing: the Start status line answers the first and part of
the third; the Journey batch persists the bulletin's checked
and printed state across restarts so the answers survive the
week between Thursday's work and Saturday's printing.

**5. The special Sunday.** Christmas, Easter, a baptism, a
funeral: the shape changes, the template bends, the deadline
does not move. Design demand: seasonal variants are first
class (the Analyzer already clusters them separately), and
bending the template for one week must not break the standing
one (Next Sunday returns to the ordinary shape). Standing:
seasonal clustering shipped in the Analyzer; seasonal packs
are roadmap, and this journey is their acceptance test when
they land.

## What the interviews still owe us

The pastor interview (question set merged in
lens-synthesis-2026-10-15.md) is unrun; Mark Johnson's answers
outrank every assumption in this document. Where his answers
contradict a journey above, the journey gets rewritten, not
defended.
