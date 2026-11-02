# Pre-release checklist — OpenBulletin

No release ships until every item below is checked.
Owner of the human items is Jacob (interactive desktop
run); Envoy prepares and records, but cannot pass a
human item on VM evidence. The H1 sentence stays
UNLANDED until its live check passes — that gate is
not waived by anything in this file.

## H1 — Live check (NOT DONE, gates the H1 sentence)

- What: hand-edit a page, go back a step, start next
  Sunday, check the edit on screen and in print.
- Owner: Jacob, on an interactive desktop (human run).
- Gate: the H1 refined-rule sentence does NOT land
  until this check passes and the pass is recorded
  here with its date.
- Status: NOT DONE. A virtual-display attempt was made
  from the VM in round 51 (R51-F, G-16) — see "VM GUI
  attempt record" below — and it did NOT complete this
  check. The attempt is not a pass and is not cited as
  one.

## R47-B — Keyboard walk (NOT DONE)

- What: a keyboard-only walk of the app.
- Owner: Jacob, on an interactive desktop (human run).
- Gate: must pass before any release; the pass is
  recorded here with its date.
- Status: NOT DONE. Not attempted from the VM: a
  keyboard walk by definition needs a human at a
  keyboard, and the VM attempt below could not even
  drive canvas text editing (see below).

## F11 tripwire check (STANDING, every release)

- What: search the release candidate's hosted CI logs
  for `ETXTBSY` and `ETXTBSY absorbed` lines.
- Owner: Jacob (product owner), executed by Envoy.
- Cadence: at every release-gate CI review — no
  release ships without this check.
- Re-open trigger, verbatim: any future `ETXTBSY
  absorbed` line in hosted CI re-opens F11.
- Standing record: `docs/CARRIED-ITEMS.md` (F11
  tripwire entry, with history).

## Release gates (existing batch gates, restated)

Hosted CI green on all three OS legs for the release
tip; fuzz-as-claims byte-identical to the standing
values; booklet exact 31,717 / 34,977. These are the
per-batch gates; they do not substitute for the human
items above.

## VM GUI attempt record (round 51, R51-F, G-16)

Attempted 2026-11-02 from the Muse VM, because the
charter requires attempting what is actually possible
from here before declaring the walk impossible. What
was attempted, and what was observed, exactly:

- Environment at attempt time: headless Linux VM, no
  physical display or keyboard, no interactive desktop
  session; `Xvfb` and `xdotool` present, `DISPLAY`
  unset; app debug binary built from the R51-F tip
  (`cargo build -p openbulletin --bin openbulletin`,
  exit 0).
- First launch failed: the app panicked at startup
  with `Library libxkbcommon-x11.so could not be
  loaded` (xkbcommon-dl). The VM lacked
  `libxkbcommon-x11-0`; it was installed via apt
  (`libxkbcommon-x11-0 1.6.0-1build1`, plus
  `libxcb-xkb1`), after which the app launched.
- Virtual display run: `Xvfb :99 -screen 0
  1280x900x24`; the app window `OpenBulletin`
  (1180x820) appeared; screenshots captured with
  `xwd` and converted with `ffmpeg`.
- Observed working: the Start step renders fully
  ("Working on: Sunday, November 8, 2026", Front
  Cover preview with the welcome lines); clicking
  the top-bar Pages button opened the Pages window
  ("Front Cover (1 of 4)"); clicking the cover's
  text box selected it ("Selected box: Front
  Cover", blue outline, exact place/size panel).
- Observed BLOCKED: canvas text editing could not be
  driven. Four attempts, each verified by screenshot:
  (1) double-click the Welcome line, then End, then
  type — nothing changed on the canvas; (2) Enter
  with the box selected — screenshot byte-identical,
  no edit mode; (3) single click in the box, then
  type — screenshot byte-identical; (4) Escape, then
  delayed double-click, then type — the typed
  character never reached the canvas (the side
  panel's Height field showed selection highlight
  instead). Keyboard focus never engaged the canvas
  text editor in this blind xdotool-driven session,
  so the hand-edit leg — the first leg of the H1
  check — could not be performed or verified here.
- What this proves and does not prove: the app
  launches and renders under a virtual display, and
  its window navigation responds to driven clicks.
  It does NOT prove the H1 walk, the R47-B walk, or
  any edit/print behavior, and no such claim is made.
- Why the full walk is not completable from the VM:
  the H1 check needs a verified hand-edit, a step
  change, a Next Sunday start, and an on-screen AND
  in-print check of that edit. Without canvas text
  input, the chain cannot start, and driving the
  rest blind (print dialogs, file dialogs) would
  produce unverifiable state, not a check. The
  remaining path is the human run named above.
- Evidence (screenshots + logs):
  `round51-out/coord/r51f-logs/gui/shot1-start-step.png`
  (Start step render),
  `round51-out/coord/r51f-logs/gui/shot2-pages-window.png`
  (Pages window open),
  `round51-out/coord/r51f-logs/gui/shot6-edit-blocked.png`
  (box selected, edit not achieved).
- Side effects cleaned: the runs wrote no bulletin
  file; `settings.json` was untouched (mtime
  2026-11-02 02:53, predating the runs); the runs'
  instance lock files were removed after the app
  was killed, restoring the config dir to its
  pre-run contents.
