# Lens review synthesis, 2026-10-15

Four blind passes over one packet (docs/planning/lens-packet.md):
Opus (Tier-2), GPT (Tier-2), and two Wonder lanes with code
access. Reports: lens-opus.md, lens-gpt.md, lens-wonder-1.md,
lens-wonder-2.md. Coordinator adjudication below; every
load-bearing claim was re-checked against the code at HEAD.

## Verdicts, converged

- **Administrator:** yes, unaided, on an established bulletin,
  with one weekly detour (all four lanes agree): the hymn
  numbers and the sermon title, the changes she makes most
  often after the announcements, have no home in the flow.
- **Pastor:** fine as proofreader; NOT yet unaided for the
  vacation week (all four agree). His failures are the ones
  the philosophy itself predicts for a cold start: wrong or
  stale starting point, invisible work state, a printer that
  is not the office printer, and no slot for the one thing he
  always brings, the sermon title.

## Packet corrections (the Wonder lanes read code; they win)

1. Hymn numbers are literal text in the order-of-service
   story ("Hymn of Praise[TAB]432"); there is NO sermon title
   anywhere; the sermon line prints {Church.pastor}. The
   packet's "fields fill the hymns" was wrong.
2. In-place caret editing EXISTS (an editor over the frame).
   The packet's "does not exist" was wrong in the other
   direction. The real debt is fidelity while typing: interface
   font, first-paragraph size, dot leaders hidden, raw field
   tokens visible ({Church.pastor} shown literally, with no
   explanation that braces mean "fills itself in").
3. Printer memory is keyed by a free-typed name seeded with
   the literal string "Church office printer", per machine.
4. Print freshness is session-only; a restart forgets it.
5. Next Sunday advances from the FILE's date (+7). A file
   three weeks stale advances to a date still in the past.
   Opus's worry (F1) is real.
6. A leftover .obd-only open path exists (main.rs ~4815,
   "Pick a bulletin file ending in .obd"), beside the one
   door. Wonder lane 2's Finding 4 is real.
7. Tier-2's "Analyzer/labels not established as shipped" is a
   packet artifact: both ARE shipped (the packet's shipped
   list under-described them). Territory truth stands with the
   code and the day's commits, not the packet.

## The adjustment plan (Jacob's order: update/adjust)

**Batch J-A, state and the service (builds first):**
- J1 Date guard + honest Next Sunday. Check flags a bulletin
  date that is not the coming Sunday, in the problem color,
  with a fix button; Start's headline and the This Sunday step
  say it in words; Next Sunday snaps forward to the coming
  Sunday when +7 would land in the past and says how many
  weeks it advanced.
- J2 The service on This Sunday. The step lists the order of
  service's tabbed lines (hymn numbers and friends) as labeled
  fields, plus a sermon-title field that edits the Sermon:
  line while preserving the {Church.pastor} token. Pure
  extract/apply functions over the order story; honest empty
  state when a bulletin has no such lines.
- J6 A memory of the bulletin's state. Settings persist, per
  file path, the print fingerprint and the checked fingerprint;
  the Start status and Print step say "print copy is current"
  / "checked, nothing changed since" / "not checked since the
  last change" across restarts. The app also reopens on the
  step the user left.
- J7 First-run honesty. The fresh document's date snaps to
  the coming Sunday; until a file is opened or the church is
  named, Start calls the on-screen bulletin a starter sample.

**Batch J-B, print, forgiveness, and polish:**
- J3 Escape forgiveness. Cancel keeps its meaning (Batch A
  law), but a changed session's words are recorded, and the
  status offers "Bring the typed words back", reopening the
  editor with them.
- J4 Printer as a choice. The Print step picks from learned
  printers by name plus "a new printer"; the name field starts
  empty with a hint (the seeded fake name dies); the checklist
  says the answers live on this computer.
- J5 Dialog guidance. After the print copy opens, a note built
  from the learned answers says what to choose in the print
  window: Actual size (never Fit), the flip in words, one
  sheet first to check.
- J8 Save proposes a dated name ("Bulletin 2026-10-18.obd").
- J9 The recovery offer gates editing until answered.
- J10 Automation shows its hand. Next Sunday's status names
  the announcements it ended (first words, not just a count);
  Check lists what is being carried and through when, as
  information, not problems.
- J11 Proofing honesty. The reading copy is labeled "Open a
  copy to read through"; Check's guidance says it checks
  printing, not spelling, until spell check ships (v0.5).
- J12 Wording sweep on the guided path (no "encoding", no
  unexplained "story"; braces in the canvas editor get one
  explaining line).
- J13 The leftover .obd-only path routes through the one door.
- J14 The fit check measures tabbed lines by shaped width,
  not by counting them as one line sight unseen.

**Deferred, recorded:** the import field-binding pass (point
at the date, readings, hymns in an imported draft to bind
them to fields; Opus change 5, the Analyzer's missing half
for single imports); spell check (v0.5); shaped-fidelity
typing (the editing phase: real faces and sizes and leaders
while typing, tokens rendered resolved); native printing
(the print dialog problem's real cure, a platform decision,
not this batch).

**Doc fix from Opus F10 / GPT naming finding:** PRINCIPLES.md
#3's rider now says the Apple rule is interaction order and
plain naming, not label grammar. "Open the print copy" is a
fine label.

## Pastor interview: the merged question set

For Mark Johnson (30 years a pastor), deduplicated from the
four reports (22 + 20 + 13 + 10 questions). Ask for stories
and real bulletins, not preferences.

1. Walk me through the last bulletin you produced alone, or
   the last one that went out wrong. What happened, who caught
   it, and when: before printing, at the copier, or in the pews?
2. Part by part: what changes every week, what changes by
   season, what never changes? Where does each item come from,
   and which arrives late?
3. The sermon title: when is it final, how often does it change
   after Thursday, and how does the office learn it today?
4. Hymns: who chooses them, when, and where is the choice
   written before it reaches the bulletin?
5. Readings: how many, who picks them, lectionary or series,
   and does the bulletin print the text or only the references?
6. Announcements: who decides one has run long enough? What
   has printed after it should have ended, or vanished early?
7. The prayer list: what may be printed, for how long, and
   what must never carry over by default?
8. When you proofread, what do you read for, in what order:
   screen or paper, and has an error ever survived because it
   looked like last week's correct text?
9. The vacation week, step by step as it happens today: whose
   computer, where are the files, who prints, who folds?
10. When administrators changed over the years, how was the job
    handed over, and what knowledge walked out the door?
11. Printing the booklet: what machine, how many copies, and
    what goes wrong: upside-down backs, shrunk pages, order,
    jams? What is the Saturday fallback?
12. The big Sundays and the one-offs (Christmas, Easter,
    baptisms, funerals, weddings): how different is the paper,
    who builds it, and from what: last time's file, a template,
    memory?
13. What is in the bulletin that exists nowhere in writing:
    spellings, courtesies, the thing a new administrator gets
    wrong in week one that nobody thinks to mention?
14. What do members do with the bulletin afterward, and what
    do the homebound and large-print readers get today?
15. What would a program have to do for you to trust a printout
    without reading every line, and what would it have to do
    once for you to never trust it again?

## Spend

Tier-2 for this round: Opus 2,611 in / 14,642 out; GPT 1,712
in / 3,587 out. Roughly $0.34 estimated in total. The two
Wonder lanes ran on internal tokens.
