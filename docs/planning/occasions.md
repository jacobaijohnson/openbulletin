# Any occasion, any date (Jacob, 2026-10-15)

Requirement, in Jacob's terms: allow any date to be added in
any format. Services are not only Sundays: Ash Wednesday,
Passover, Holy Week, several services at different times on
one day, and different churches and denominations hold
services on different days entirely (Saturday, Wednesday,
Friday), plus weddings, funerals, and other events.

## What this changes in the model

The document stops being "a Sunday" and becomes **an
occasion**:

- Fields: a service date (any day of the week), a service
  name (what the day is called: "Twentieth Sunday after
  Pentecost", "Ash Wednesday", "Easter Vigil", "Christmas
  Eve"), and an optional service TIME ("9:00 AM"), so two
  services on one day can be two documents that differ by
  time and whatever else changed.
- Sunday remains the default rhythm, not the definition.
  "Next Sunday" generalizes to advancing to the next service
  day: +7 days by default, and the date guard's notion of
  "the coming Sunday" becomes "the coming service day" for
  the document in hand. The This Sunday step's words adapt
  to the document (the step serves the occasion; for the
  beachhead church it will say Sunday language most weeks
  because most of its occasions are Sundays).
- The date guard, the analyzer's seasonal clustering, and
  the seasonal packs all key off the occasion, not the
  weekday: an Ash Wednesday bulletin is a first-class
  document, and last year's Ash Wednesday is its template
  ancestor.

## Date entry, any format

Everywhere a date is typed (the occasion date, keep-through
dates), the parser accepts the ways office workers write
dates, and shows back the date it understood in long form
before it counts:

- ISO: 2026-10-18
- US numeric: 10/18/2026, 10/18/26, 10-18-2026
- Month names: Oct 18, October 18 2026, 18 October 2026
- With a weekday: Wed, Feb 18, 2026 (the weekday is checked
  against the date; a mismatch is said out loud, never
  silently believed)
- Missing year means the coming occurrence of that date.

Unparseable input is refused in plain words with an example,
never guessed. This is Principle 9 work: the office worker
writes dates their way; the program does the conforming.

## Where it lands

The occasion fields ride with the G1 format work (dump
schema v2 + .obd v1), because both change what a document
carries and one migration beats two. The date parser is a
pure function (ob-model), unit-tested across the formats
above, and lands with the same batch. Until then the
Sunday-shaped fields stay as they are; this note is the
specification the batch builds against.
