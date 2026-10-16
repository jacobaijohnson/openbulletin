# Adjudication: GPT workflow review (2026-10-15)

GPT reviewed documents, not code. The coordinator checked every
material claim against the shipped app (HEAD at review time) and
rules as follows.

## Already shipped (the docs GPT read lag the code)

- Check warnings carry go-fix buttons; recovery shows the saved
  time and offers continue/discard; the print checklist quotes
  the learned printer instruction verbatim; a duplex test sheet
  exists; Next Sunday advances a copy and never overwrites the
  source; fields already resolve the date and church facts.

## Adopted to build (in this order)

1. **Unsupported-character detection.** Real silent-wrong-output
   hazard: ob-pdf prints a space for characters outside WinAnsi
   (a typed cross can vanish). Check will name the character and
   the story it sits in; Print repeats it in the checklist.
   Warned loudly, not hard-blocked: the user stays sovereign,
   but it cannot be missed.
2. **Announcement expiry.** "This Sunday only" by default, "Keep
   through [date]" when a notice runs several weeks; Next
   Sunday clears expired ones and reports what it cleared,
   with undo already provided by the snapshot history.
3. **Start status in plain words.** "Working on: Sunday,
   [date]" plus the next thing needed, derived from the
   document and step state, not maintained by hand.
4. **Explicit "No announcements this week."** Ends the blank
   versus missing ambiguity; the Check step accepts it as an
   answer and otherwise says announcements are still needed.
5. **Print copy freshness.** The app remembers the revision it
   exported; editing afterward makes Print say "Changed since
   the print copy was made" until a fresh copy exists.

## Shipped (2026-10-15, the workflow streamlining batch)

All five landed, each with tests:

1. Unsupported characters: ob-pdf now publishes its own
   WinAnsi test (`winansi_supported` / `unsupported_chars`), so
   the warning and the encoder share one definition. Check
   names the character and the story, with a fix button to the
   step that edits it; Print repeats the warning. One
   correction to the review's premise: the dagger and the
   curly quotes DO print (they are real WinAnsi characters);
   the characters that vanish are ones like the Latin cross,
   check marks, emoji, and Greek letters, and the tests pin
   both halves of that. Tab is never flagged: order lines
   spend it as the dot-leader mark before encoding.
2. Announcement expiry: a story may carry a keep-through date
   (serde-defaulted; older files load). Next Sunday keeps the
   words while the date covers the new Sunday, ends and counts
   the story once the date has passed, and the app says how
   many ended. The Announcements step has one optional
   "Keep through" date field for the announcements it edits.
3. Start status: a pure function over the document derives
   "Working on: Sunday, [date]" and the next thing needed
   (church name, then date, then announcements), or "Ready
   to print."
4. "No announcements this week": a document flag
   (serde-defaulted), a checkbox on the Announcements step,
   and Check accepts the empty box only when it is marked.
   Next Sunday resets the mark.
5. Print-copy freshness: the Print step remembers the
   snapshot a copy was made from; any later change brings up
   "changed since the print copy was made" until a fresh copy
   exists. In-memory only for now: closing the app forgets it.

## Considered and deferred

- Per-field Confirmed states: real value for teams, but they
  ask people to maintain state by hand, which GPT's own scope
  section warns against. Revisit after the five above land and
  a real handoff has been observed.
- Liturgical suggestion packs and spelling: already on the
  roadmap for v0.5; unchanged.

## Rejected (agreeing with GPT's scope section)

Project boards, simultaneous editing, desktop logins, expert
modes, recurrence rule languages, automatic shrink-to-fit,
unreviewed generated content, NAS as a handoff prerequisite.

## Docs debt GPT caught

README's "Not built yet" and "Run it" now tell two stories
since the Windows bundle landed; ROADMAP still lists shipped
imports as future work and calls native .pub reading excluded
(it shipped in phases 2a/2b). A docs sync joins the next
integration step.
