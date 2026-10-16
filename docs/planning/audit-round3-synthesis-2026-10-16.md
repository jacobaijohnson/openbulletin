# Convergence loop round 3: synthesis

Three blind Wonder lanes (verify, review, red team) over HEAD
`0d6a77a` (code `3346edd`), covering everything shipped since
round 2: wanted fonts, the contribution seam (R-1), hymns and
hymn memory (R-2a), the privacy trio and serving lines (R-2b),
loose sheets (R-3), and re-attacks on the round-2 fixes.
Reports: `~/workspace/openbulletin-audit-wave3/round3-*.md`.

The round was interrupted by a disk emergency (the VM hit
100 percent; the target tree was deleted from the main chat
mid-round). The review and red team lanes completed on code
reading and finished probes respectively; the verify lane's
claims checks landed but its fresh full-suite run and the
live booklet/music spot-checks did not, and are carried as
UNVERIFIABLE into the fix verification. No lane's findings
rest on a test count this round.

## Verdicts

- VERIFY: claims 1–5 VERIFIED (wanted fonts, the seam, hymns,
  privacy/serving, inserts, on code and shipped-test
  evidence); the known fake-converter flake confirmed as the
  flake (passes solo); bundle staging list verified, run
  PARTIAL at check time. Fresh totals UNVERIFIABLE (above).
- REVIEW: 0 BLOCKER, 7 MAJOR, 11 MINOR.
- RED TEAM: 0 BLOCKER, 2 MAJOR, 7 MINOR, all 20 probes run.
  Strongest HELD: the helper runner's output cap and the IDML
  total-cap accounting, re-attacked at the seams the round-2
  fixes created. The lane's summary: the code that counts
  actual bytes held everywhere; the code that counts declared
  bytes fell.

Exit bar NOT met: 9 MAJOR open. Fix batches R3-A to R3-D
below; round 4 follows.

## MAJOR findings and dispositions

- R3-1 (review) Wanted fonts: a source name carrying a face
  suffix outside the substitution table ("Verdana-Bold") is
  recorded verbatim, so installing the real font later never
  upgrades the document and Check names a family nobody can
  install. The corpus long tail is the common case (217 names,
  ~20 table rows). FIX (R3-A): generalize base-name extraction
  for every name, not just table rows.
- R3-2 (review) Wanted fonts: `row_for` prefix matching
  over-claims tiers: "Arial Narrow"/"Arial Black" earn the
  Liberation metric-twin claim, "Loraine" earns Lora Exact.
  The false "same letter widths" assurance is the harm. FIX
  (R3-A): a row hits on exact base, or base plus a same-design
  face suffix (Bold/Italic/Oblique/Regular) only.
- RT3-A (red team) HTML pre-parse: one stray quote in an
  unquoted attribute value blinds the byte scanner's quote
  tracking; a 150 KB page froze import for 111.6 s (control
  refused in 0.0004 s). The round-2 RT-3 defense is bypassed.
  FIX (R3-B): quote state opens only where HTML allows a
  quoted value; a quote still open at the next `>` is closed
  there (fail-safe depth counting, never blind).
- RT3-B (red team) .obd/.obdi loaders sum DECLARED zip entry
  sizes for the 512 MB cap while asset reads are unbounded:
  a 612 KB archive claiming 5-byte entries expands to 600 MB
  in memory on open; the shape scales to an OOM kill. This
  narrows round 1's HELD for the same loaders. FIX (R3-B):
  count ACTUAL bytes on every read (the IDML `take(cap+1)`
  pattern) in both loaders, and audit every other zip reader
  for declared-vs-actual accounting.
- R3-3 (review) Serving lines: renaming a line strands its
  stored names. The renamed row reads blank, the old names
  keep printing through the built-in token, and names typed
  under the new name never clear the stale value. FIX (R3-C):
  migrate participant keys positionally when the profile's
  serving-lines list is saved; test the token side, not only
  the row side (the gap in the existing regression test).
- R3-4 (review) Check asks the children question about every
  engraved music system, and the directed fix (right-click
  the picture) does not exist on move-only music boxes: one
  hymn import yields 4–6 unfixable findings. FIX (R3-D):
  music frames are exempt from the children finding (sheet
  music is not a photograph).
- R3-5 (review) Contribution seam: when the save after Accept
  fails, the app's status text tells the user to open the
  part again; re-accepting applies it a second time and every
  appended word lands twice. A copy-then-remove fallback in
  filing reaches the same duplication. FIX (R3-D): the app
  remembers applied parts by content fingerprint for the
  session; a re-opened applied part offers filing, not
  re-applying.
- R3-6 (review) + RT3-H (red team) Hymn memory: the learn
  rule (strictly-newer date wins, same date keeps the first)
  makes a wrong pair unfixable: a typo saved in next week's
  draft seals the pair, and a future-dated bulletin seals it
  against every real correction; Check then flags correct
  bulletins weekly, and no surface can inspect or forget the
  memory. FIX (R3-C): same-date learns overwrite (a re-save
  is a correction); the disagreement finding gains a
  forget-this-pairing action; the finding names the
  remembered date so the office can judge. Residual: a
  future-dated wrong pair still outranks older corrections
  until forgotten; the forget action is the answer, stated
  here rather than hidden.
- R3-7 (review) Inserts: a sheet merely open on the canvas
  makes the app falsely dirty (the snapshot compares the
  swapped arrangement against the canonical save), arming
  the unsaved-work guard with a false warning and leaving a
  spurious recovery offer after quit. No data is lost; the
  harm is the guard crying wolf on a normal path. FIX
  (R3-D): dirty comparison uses the canonical document
  serialization; undo snapshots keep the live arrangement.

## MINOR findings and dispositions

- R3-8 (review) Contribution plan/apply divergences, five:
  (a) plan says "Goes into the announcements" while apply
  places nothing when no Weekly frame exists; (b) a
  keep-through date is silently dropped on non-announcement
  arms; (c) an accepted part's keep-through overwrites the
  weekly story's existing date, silently shortening older
  content; (d) the review plan is computed once but apply
  recomputes hymn slots per part, so part 2 can land
  differently than shown; (e) part-line parsing: "Psalm\t23"
  counts as a hymn entry and multi-tab lines lose their
  middle segment. FIX (R3-D): plan by simulating the apply
  on a clone; keep-through composes as the later of the two
  dates and applies on every arm that places words; hymn
  entries require a parsed number AND (a hymn label or a
  hymnal); a line's value is everything after the first tab.
- R3-9 (review) Contribution Accept is the one whole-document
  mutation that does not leave insert editing first. FIX
  (R3-D): same one-line discipline as the other paths.
- R3-10 (review) still_waiting over-credits arrivals:
  substring sender match ("Lee" satisfied by "Leanne"), and
  one undated filed part satisfies its expected part every
  week forever. FIX (R3-D): sender match is exact
  (case-insensitive, trimmed); undated filed parts count
  only for the Sunday they were filed for.
- R3-11 (review) .obdi attach silently drops a sheet's rights
  record when the same picture bytes already exist under a
  weaker record. FIX (R3-D): merge toward the stronger record
  (a credit line, then a known license, beats an empty one)
  and say so in the attach summary.
- R3-12 (review) (a) The analyzer-accept hymn learn call is
  dead (learned documents carry no occasion date); the claim
  in the R-2a build report was wrong. FIX (R3-C): remove the
  call; memory learns from saved weekly bulletins, which is
  the design. (b) hymn-memory.json writes are non-atomic.
  FIX (R3-C): tmp + rename, the settings pattern.
- R3-13 (review) pin_document_fonts can name a different
  face than the PDF writer embeds (first vs last same-key;
  italic styles pin a non-italic face), and font_sha256 is
  read nowhere. FIX (R3-A): one face-pick rule shared by pin
  and writer (exact bold/italic, last same-key wins), living
  where both can use it.
- R3-14 (review) PDF source_families iterates a HashMap, so
  family spelling choice can differ between runs when the
  library holds two spellings. FIX (R3-A): deterministic
  order (sorted; first case-insensitive match wins).
- R3-15 (review) Music proof prints systems in frame order,
  not system order (music boxes are movable), duplicates a
  duplicated frame's system, and drops the spoken-only
  count every other door states. FIX (R3-C): sort by system,
  dedupe by system, thread the count into the status.
- R3-16 (review) Check keeps naming a wanted font after the
  bulletin stops using it. FIX (R3-A): only families still
  named by a style in the document are reported.
- R3-17 (review) Hymn duplicate detection misses "ELW 432"
  vs bare "432"; Next Sunday's hymn clearing wipes
  hymn-labelled lines in EVERY story (an announcement
  "Hymn Sing\t7 PM" would be cleared and reported as a
  hymn). FIX (R3-C): duplicates when numbers match and
  hymnals agree or either is absent; clearing scoped to the
  service story.
- R3-18 (review) FONTS.md says Bookman has no metric twin;
  the table maps Bookman Old Style to Bookman URW at
  MetricTwin with no metric evidence in the repo. FIX
  (R3-A): demote the row to Lookalike; correct FONTS.md.
  An unverified metric claim is the R3-2 harm in miniature.
- RT3-C (red team) IDML close_off_cut landing inside CDATA
  collapses the rescue to zero spans while the loss note
  claims a tail trim, with a 23.4 s stall. FIX (R3-B):
  CDATA-aware cut (back off to before the CDATA opens) and
  one bounded rescue parse.
- RT3-D (red team) Contribution "For Sunday: Monday,
  October 25, 2026" parses as no Sunday named (the weekday
  is wrong for that date), so the review misdescribes the
  file and wrong-week arming never engages. FIX (R3-D): the
  parser distinguishes "a date was written with the wrong
  weekday" and the review says so.
- RT3-E (red team) Inbox scan reads and parses every .md
  with no size cap (5 MB file: 10.7 s on entering Start).
  FIX (R3-B): per-file cap in the scan; oversize files list
  with their reason, unparsed (the counted-never-silent
  shape).
- RT3-F (red team) Font zip entries past 512 are dropped
  with no refusal line. FIX (R3-A): count and report them,
  the fonts-add closing line names the skips.
- RT3-G (red team) Prose under a hymn-containing label
  ("Hymn requests: see the director") fills an empty hymn
  row and is taught to memory on save. Merged into the
  R3-8(e) fix (a parsed number is required).
- RT3-I (red team) Each helper-runner timeout leaves 2
  detached reader threads + fds until the grandchild exits
  (measured 1 to 7 threads over 3 runs). ACCEPTED: the
  caller's deadline holds (the round-2 requirement); the
  residue dies with the process or the pipe. Recorded here
  so round 4 does not re-derive it.

## Verified clean, worth restating

- The runner's output cap (stderr-only flood trips
  TooMuchOutput in 3.79 s) and the IDML total-cap accounting
  (actual bytes via take()) held under re-attack.
- Round-2 fixes hold under hostile re-reading (analyzer
  seeding, bound_families snapshot, salvaged settings).
- The resolution chain is genuinely shared by canvas, Check,
  and the PDF writer; fsType gates embedding; the inbox read
  never moves a file; without_web_hidden composes only in
  the HTML door; .obdi keeps the .obd loader discipline
  apart from the declared-size accounting fixed above.

## Carried into the fix verification

The verify lane's UNVERIFIABLE items (fresh suite totals;
live booklet byte sizes; the music 237/237 helper run) are
re-run by the coordinator on the clean build after the fix
batches, before round 4 is dispatched.
