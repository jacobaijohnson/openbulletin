# The convergence loop — Jacob's directive, 2026-10-16

Jacob's words, condensed: "re-loop over and over via Wonder
until everything checks out and everything is correct"; the
north star is a usable product with "the ideas built and
integrated and helpful and user-friendly," first class,
natively implemented, working end to end. When the loop
converges, Opus and GPT run a similar audit as the final gate
(Tier 2, his instruction, sequenced last on purpose).

## The north star

A real church secretary, unaided, produces a real Sunday
bulletin end to end (the Sunday Test), and every role that
feeds the bulletin (pastor, music director, choir director,
Sunday school director, youth leader) gets their part in
without the administrator retyping it. One app, one document,
one door for everything that arrives.

## The loop

Each round:
1. AUDIT: three blind Wonder lanes (verify / review /
   red team) over current HEAD, briefs carrying the prior
   rounds' dispositions so lanes regression-hunt the fixes
   and hunt new ground.
2. FIX: batches in severity order, one writer at a time,
   coordinator re-verifies every batch (own test run + spot
   checks) before the next round.
3. IDEAS: in parallel, role lanes (below) feed a build-out
   backlog; buildable items are built between audit rounds
   in the same one-writer discipline and join the next
   round's audit surface.

Exit bar: one full audit round with zero BLOCKER and zero
MAJOR findings, every MINOR fixed or dispositioned on the
record, and the build-out backlog below burned down to
items blocked on a human or fixture. Then Tier 2 (Opus +
GPT, same three-aspect shape) runs as the final gate;
whatever it finds re-enters the loop.

## Build-cache policy (Jacob, 2026-10-20)

Full text as relayed and adopted:
`~/workspace/openbulletin-audit-wave3/build-cache-policy-2026-10-20.md`
(the audit tree copy is the source of record; this
section is the charter summary). Both projects share
the 100 GB VM disk, so cache churn here is disk
pressure there.

- A round is not complete until backed up and
  confirmed on git (tip pushed, verified against the
  remote). No cache action is owed before that
  confirmed completion; a cold rebuild after a clear
  runs from that confirmed tree.
- At each confirmed completion, `df -h /home/hatch`
  is checked against the window threshold: overnight
  23:00-07:00, at or above 87% used clears; daytime
  07:01-22:59, at or above 94% used clears. On a
  clear: delete the cargo target directory and
  rebuild cold before the next round, verifying
  clean first. Below the threshold, the cache
  carries into the next round.
- Trigger latching (Jacob, 2026-10-20 23:10): an
  over-limit reading triggers the completion clear
  even if a later reading is lower; the trigger does
  not re-test at completion. The 88% reading at
  adoption (2026-10-20 23:08, overnight line 87%)
  latched the clear at round 34's confirmed
  completion.
- Never clear mid-round. Exceptions: a Jacob-approved
  clear at a commit boundary — the standing per-batch
  clean + cold verification (Jacob, 2026-10-20
  14:53) is exactly that and continues unchanged —
  or the emergency stop: below 1 GB free, stop
  writing, push what exists, and alert.
- The close's own cold verification pass remains
  part of completing the round, before the
  completion is confirmed; this policy governs only
  the post-completion disposition of the target
  directory.

## Roles track (Jacob, 2026-10-16)

The identities widen beyond administrator / pastor /
substitute: Sunday school director and workers, youth
leaders, music director, choir director. Standing law holds:
diverge on state, never on identity; no per-role modes.
Roles appear as CONTRIBUTIONS into the one document through
defined slots (their hymns, their announcements, their
schedules), never as separate programs. Research lanes:
roles-sundayschool, roles-music, roles-integration,
roles-inserts (reports under ~/workspace/openbulletin-audit-wave3/).

## Inserts (Jacob, 2026-10-16: "a big thing")

Inserts are first-class citizens of the model, not pages of
the bulletin: secondary sheets that travel inside it. An
insert is its own small document (own size, own panels,
own life: standing or dated), attached to the bulletin,
printed in the same job on its own paper, counted for the
print run, and reusable from a library week to week. The
roles-inserts lane designs the details against real practice.

I1 shipped (Roles R-3, 2026-10-16): the model, the three
lives in Next Sunday with ended sheets named, the Print
step's Also print block with one file per sheet, the two
insert-only Check findings, the Loose sheets window with
the canvas swap, and the .obdi library. I2 remains:
two-up printing, per-sheet large print, file-carrying
sheets, rescue-to-insert, pre-filled starters.

## Build-out backlog (from recorded debts + ideas)

Buildable now, no input needed: the print-time overflow
guard (nothing clips silently even when Check is skipped);
font pin (font_sha256) enforcement with named print
fallbacks; IDML import fidelity (fabricated positions,
rotation, threaded-story duplication); .pub remainder
threading; interaction fuzz pack over the parser doors;
clip starter set growth (the manifest's follow-up subjects);
seasonal packs for the confirmed occasions; archive search
across past .obd files; Verovio engraver staging for the
bundle (spike first); AVIF decode inside the bundle build
(spike first: CI can provision what this sandbox cannot);
editor frame identity, then multi-select / align /
distribute / snapping / rotation / z-order.

Blocked on humans or fixtures, honestly parked: the Sunday
Test itself; INDD probe fixtures; a real .pub and .idml at
the church PC; Pastor Mark's workflow notes; funeral /
memorial pack confirmation; repo public/private ruling;
signed installers (needs a certificate).

Out by Jacob's standing rulings, never in the loop: music
editing of any kind; writing .pub or .indd; OCR in v1;
cloud hosting before churches ask.
