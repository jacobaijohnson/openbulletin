# Closing audit synthesis, 2026-10-15

Six blind Muse Wonder lanes audited the whole project cold:
code correctness, security, efficiency, effectiveness, process,
and a red team that ran real attacks. Every lane verified the
baseline itself (53 tests, clean clippy at the time). Reports:
closing-code.md, closing-security.md, closing-efficiency.md,
closing-effectiveness.md, closing-process.md,
closing-redteam.md.

## Lane verdicts

| Lane | BLOCKER | MAJOR | MINOR | Verdict |
|---|---|---|---|---|
| Code | 2 | 5 | 11 | Not yet: two reachable panics in the flagship canvas |
| Security | 0 | 2 | 6+2 info | Posture claims true in substance |
| Efficiency | 2 | 3 | 5 | Booklet and Check feel broken on an old office PC |
| Effectiveness | 2 | 6 | 10 | Do not hand to an admin today |
| Process | 1 | 7 | 7 | Honest habits, not yet self-checking |
| Red team | 3 | 6 | 6 | Most dangerous: the journal deletes recovered work |

## What three lanes found independently

The autosave journal deleted the on-disk recovery copy about
two seconds after launch, while the recovery dialog was still
asking what to do. No attacker, no unusual file: just an
interrupted admin who closes the app without choosing. The
"never lose work" mechanism destroyed the only copy of the
work. This is fixed (the journal decision is now a pure,
tested function that holds while a recovery is undecided, and
also when another live instance owns the journal).

## Convergent and notable findings

- Imposition panicked on any shared .obd whose panel count
  disagreed with its preset (proven with crafted files through
  the CLI; the Print step is the same path). Now: blank-page
  padding, renderers pair with the padded document, regression
  tests for all six mismatch shapes.
- The Step 1 "Use last week's bulletin..." button bypassed
  the unsaved-work guard the top-bar Open had. Both now share
  one guard, and it re-arms when the work changes (the old arm
  flag also never reset). Both behaviors are tested.
- Deleting a box while typing in another left a stale editing
  index: the original blocker family, re-entered through the
  new Batch C feature. Menu actions now commit typing first;
  the Pages pager clears selection (a stale selection across
  panels was the code lane's other blocker).
- Save and the Print exports did not commit an open typing
  session, so the newest words silently missed the file while
  the app said "Saved". All those paths commit first now.
- Performance was one architectural flaw: the text shaper was
  rebuilt, with a full Font Library scan and SHA-256 of every
  font byte, per paragraph in Check (1.25 s measured on the
  booklet) and per panel per repaint (about 0.46 s per repaint,
  seconds with a real font library). Now one cached shaper
  keyed by a cheap directory fingerprint, a per-panel lines
  cache, and one library read per PDF export. The system-font
  census moved to a worker thread.
- Font ingest was the designed untrusted-input path and had
  real holes: Windows backslash path traversal in zip entries,
  unbounded decompression, silent overwrite of a namesake
  font, and quadratic import cost. All fixed with caps,
  sanitization, hash-suffixed collision names, and one scan
  per batch.
- Extreme values in a valid .obd (font size i64::MAX)
  overflowed the PDF writer. Loads now clamp extremes, and
  .obd size is capped.
- Process: CI tested only 7 of 11 crates. The app crate's
  tests, including the regression written for the earlier
  splice-loss incident, never ran in CI. CI is now the whole
  workspace. README, ROADMAP, FORMAT, FONTS, DEPLOYMENT, and
  LEGAL had drifted behind the code in both directions; all
  synced, and the LEGAL recount verified at 523 packages, no
  copyleft-as-requirement.

## After the fix round

62 workspace tests pass (was 53), clippy clean across the
workspace with all targets. Every BLOCKER and MAJOR from all
six lanes is fixed and, where the behavior is testable,
covered by a regression test written for the finding.

MINORs were dispositioned: the cheap ones landed in the same
round (print-copy temp name, load size cap, the flaky system
font test, print-step failure wording). The rest stand as
known and accepted for now: whole-font embedding makes
1.25 MB PDFs (subsetting is planned), the tracing picture
path is display-only, the PDFium helper inherits PATH
resolution (it is the operator's own machine), journal and
settings are plaintext in the user's own config folder (no
accounts exist yet; see docs/DEPLOYMENT.md), and two app
instances are handled by courtesy lock, not enforcement.

## What no lane could check

The GUI has still never been run by a human. Every verdict is
from code reading, tests, and crafted inputs. The remaining
honest gate is a person opening the app and making a bulletin.

## Tier-2 milestone judgment (Opus + GPT, 2026-10-15)

Split verdict on "ready for one real admin, supervised":
Opus yes, GPT no. The split is about evidence, not the tree.
GPT's no rests on the effectiveness lane's pre-fix findings
(E-B1, E-M4) treated as current; the packet it received held
the pre-fix lane reports beside the post-fix synthesis, and
it weighted the reports over the synthesis. The coordinator
re-checked the territory: both findings are fixed in commit
d828571, each with a regression test (open_guard_rearms_when_
the_work_changes; journal_never_touches_recovered_work_
while_undecided; 62 tests green). Lesson recorded: external
auditors judging fixes need the fixed code or a diff, not
reports about the old code.

Both families independently rank the same remaining gate
first: a human running the GUI. Opus also named the standing
fault line precisely: the stale-index fixes are procedural
(commit funnels), not structural. One more path of that
family was found and closed the same day (paging to another
panel mid-typing now commits first). The structural answer,
frame identity instead of raw indices, is recorded debt for
the editor's next phase.
