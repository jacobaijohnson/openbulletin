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
