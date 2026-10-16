# Review packet: OpenBulletin through the design-philosophy lens

You are an external design reviewer for OpenBulletin, an
open-source desktop publishing app built specifically for
churches: a Publisher replacement for the weekly bulletin.
You are reviewing DOCUMENTS AND A DESCRIPTION, not running
code. Judge what is in this packet; say plainly where the
packet's claims sound like aspiration rather than shipped
behavior, if you can tell.

## The design law (docs/PRINCIPLES.md, in force)

1. KISS: Keep It Short & Simple. If a church office
   administrator cannot describe a feature in one sentence, it
   does not ship. (Apple: Simplicity.)
2. Simple on the surface, powerful underneath. Every screen
   passes an explain-it-simply test; the engine does real text
   shaping and real print geometry. (Apple: Aesthetic Integrity.)
3. The user is an administrator, not a designer. No layout
   jargon on the main path. Working rules: show options instead
   of requiring recall (Apple: See-and-Point); name every action
   noun first, then verb (Apple: Explicit and Implied Actions).
4. Guided, next step always visible: five steps (Start, This
   Sunday, Announcements, Check, Print), one primary action per
   step. The app states the work's state and what happens next
   (Apple: Feedback). Windows sit beside the flow, never trap
   (Apple: Modelessness).
5. Automate the recurring: dates advance, standing content
   carries, weekly content clears, fields fill. Automation
   proposes and carries; the human fires every consequential
   act (Apple: User Control).
6. Never lose work, never surprise at the printer. Autosave,
   recovery, undo, two-click arms on destructive acts
   (Apple: Forgiveness); nothing moves or vanishes between
   visits (Apple: Perceived Stability). The screen must match
   the printed sheet (Apple: WYSIWYG).
7. Imports are rescues, not magic: PDF, Publisher .pub,
   InDesign IDML, Scribus .sla and native .obd all open
   through ONE open door; exports through ONE save-a-copy door.
   Imports land as drafts a human accepts, edits, or cancels,
   with honest labels (Matched / Check this / Picture only).
8. (Build isolation from the founder's other project; not a
   design principle, ignore for this review.)
9. Conform to the office worker, not the other way round.
   Power arrives in shapes users already know: inch rulers,
   right-click menus, typing exact numbers including
   fractions. Wherever the user can grab the thing itself, they
   do; making them type instructions about it is a recorded
   debt (Apple: Direct Manipulation, Metaphors, Mental Model).
   The app is a paper object: panels fold, sheets have fronts
   and backs.

A Template Analyzer learns a church's standing layout from 3+
old bulletins (mixed formats in one pick) and drafts a template
for Accept / Edit / Cancel. Seasonal variants (Christmas,
Easter) cluster separately.

## The app as shipped (2026-10-15, verified by tests, run once by a human on Windows)

- Five-step guided shell over a real engine: fold presets
  (bi-fold, tri-fold, 16-page half-letter saddle booklet),
  automatic printer spreads, fields (date, Sunday name,
  readings, hymns by number) filling cover and service order.
- Pages canvas window: boxes on panels, drag to move/resize
  with live inch readouts, snapping to guides/edges/centers,
  right-click menus with exact positions (fractions allowed),
  guides dragged or placed by right-click, whole-box text
  editing (click a box, type in a text area; in-place caret
  editing does NOT exist yet and is a known, recorded debt).
- Start: one open door for every format; "pick up where you
  left off" (last bulletin remembered); church profile fields
  (name, service time, address) printing on the cover;
  fresh-start shapes behind a two-click arm.
- Check step: plain-word findings (text overflows its box,
  text near a fold, missing fonts, characters the printer
  encoding cannot print e.g. the Latin cross), each with a
  button that jumps to the named panel and box.
- Print step: printer sheet preview in the real font and face,
  two-sided printing questions in plain words remembered per
  printer, "Open the print copy" as the single primary action,
  a freshness warning if the bulletin changed after the print
  copy was made, one export door (PDF / IDML / Scribus / .obd).
- Next Sunday: one command that advances the date, carries
  standing content, clears weekly content, honors per-story
  "keep through" dates for announcements, and never overwrites
  the current file. Undo returns to this week.
- Autosave journal with a recovery offer on next launch;
  keybindings with Publisher / InDesign presets that only set
  defaults; font library with free-font matching and metric
  twins for imports; no accounts, no cloud, all local.

## The two people

**The office administrator** (primary user): church office
staff or a steady volunteer. Works in Microsoft Office and
Publisher habits. Produces the bulletin weekly, usually
Thursday or Friday, under time pressure, with interruptions
(phone, doorbell, the copier). Covers a well-worn path: open
last week's, change the date/readings/hymns/sermon title,
replace announcements, check, print, fold. Rarely designs
anything new. May be new to the job with no training beyond
"the last person showed me once". Their predecessor's files
are a pile of Publisher documents and PDFs.

**The pastor** (secondary, intermittent user): writes the
sermon title and sometimes the readings; proofreads the draft;
occasionally must produce the whole bulletin alone when the
administrator is on vacation, having last touched the app
months ago. Thinks in words and liturgy, not layout. May print
a proof at a different printer. Is the person the congregation
holds responsible when the printed bulletin is wrong.

Known real-world anchors: the reference church prints a
16-page half-letter booklet in-house; hymns are cited by
number only (copyright); the founder's father has been a
pastor ~30 years and is being interviewed separately (your
questions for him are wanted).

## Your task

Use the design philosophy ABOVE as a LENS: for each persona,
walk their real week with this app (first open ever; the
weekly cycle; print morning; the vacation handoff where the
pastor runs it alone) and test the interaction against the
principles, principle by principle where it bites.

Deliver, in markdown:

1. Verdict per persona: could this person succeed unaided
   today, through this lens? Where exactly does the philosophy
   hold, and where does the shipped app betray it?
2. Friction findings, each tied to the principle it violates
   and the persona it hits, ranked by how much damage it does
   on a real Sunday deadline. Be specific to moments (what is
   on screen, what the person expects, what happens).
3. Where the two personas' needs DIVERGE under the same
   principle (frequency of use changes what forgiveness,
   guidance, and automation should do), and how the design
   should serve both without becoming two apps.
4. The five changes you would make first, in order, each
   stated as the interaction change and the principle it
   restores.
5. Questions for the pastor interview: what should we ask a
   30-year pastor to learn what documents cannot tell us?

Rules: stay inside the packet's facts about what is shipped;
do not invent features. Prefer findings a design team can act
on over general praise. Plain words. No filler.
