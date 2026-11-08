# Licensing report

At reporting time, instead of digging through old
bulletins in a panic, the office picks the Sundays
it needs and gets one list: every song the church
must report, who licensed it, the license number,
and which Sundays it was sung — with the gaps (a
song with no number recorded) named, never hidden.

## What counts as a use

Only licensed songs count. A song the church has
no license note for, and a song nobody recognises,
are not in the report: there is nothing to report
for them.

A song sung twice in one service counts once. The
question the report answers is which Sundays a
song was sung on, not how many times it sounded
inside one service.

The report uses the same rule, bulletin by
bulletin, as the per-bulletin list on the Check
step ("License usage to report: ..."). Rolling up
a folder that holds exactly one bulletin gives
exactly that bulletin's Check list.

## What the roll-up contains

One row per licensed song, under its licensor's
heading:

- the song's title,
- who licensed it (CCLI, ONE LICENSE, or "church
  license" when no licensor is named),
- the license number,
- how many Sundays it was used on, and the dates
  of those Sundays,
- where it came from in the bulletin — a music file
  or a hymn line.

Rows are grouped by licensor, then by title. Under
each licensor there is a total: how many songs and
how many Sundays of use that licensor accounts for.

A row is one title under one licensor with one
number. If the same song was recorded under two
different numbers, it appears twice, once under
each number — the report never merges different
numbers. If a song had its number recorded on one
Sunday and not on another, it likewise appears
twice: once with the number, and once in the gaps
list below.

## The gaps, listed by name

The report has a section headed "Songs with no
license number recorded". Every licensed song with
no number recorded is listed there by name, with
the Sundays it was used on and their dates — so the
office sees the gap instead of a silently short
report. A missing number never blocks printing, and
it never drops the song from this report either.

When there are no gaps, the section says so:
"None — every licensed song in these bulletins has
a number recorded."

## How the office picks the Sundays

On the Print step, the "Licensing report..."
button asks you to pick a folder of past bulletins
— saved `.obd` files. The app reads each `.obd`
file in that folder, in file order, runs the
per-bulletin license list on each one, and rolls
the lists up into the one report.

The folder you pick is the source. The app's
20-file memory of recent bulletins is not the
source: it is a memory of the last few files, too
small and too incidental to stand for the Sundays
you need. There is no archive to wait for; you
choose the folder each time.

Files that are not `.obd` files are passed over.
A bulletin's Sunday in the report is its occasion
date where one is recorded, otherwise its long
date, otherwise its file name — a dateless file is
still named, never dropped.

## What happens with a file that cannot be read

It is skipped and named, never hidden, and it
never fails the whole roll-up. The report has a
section headed "Files skipped — they could not be
read, so they are not in this report", and each
skipped file is named there: "[file] could not be
read, so it is not in this report." The status line
after saving names the skipped files too.

If the folder holds no `.obd` files at all, no
report is made, and the app says so in plain
words. If the folder holds only files that cannot
be read, a report is still made, naming them.

## The insert-music limit, in plain words

This report counts host-document music only.
Music inside an insert is not counted here: an
insert's music rides the insert's own document at
print, not the host bulletin, so the per-bulletin
list the roll-up is built from never sees it. The
report says so itself, in its last paragraph:
"This report counts host-document music only.
Music inside an insert is not counted here: an
insert's music rides the insert's own document at
print, not the host bulletin."

A church whose music lives in inserts will find
this report short by exactly that music. That is a
known limit of this version, stated in the report
rather than left quiet.

## The report itself

The report is a PDF file, Letter size, saved
through a save dialog — the default name is
`licensing-report.pdf` — made the same way as the
music proof. The label on the Print step says:
"Pick a folder of past bulletins (.obd files) and
save the report as a PDF."

After saving, the Check step names the report in
one plain note, so the office finds it without
hunting: "Licensing report saved as
licensing-report.pdf: 3 licensed songs from 2 past
bulletins, 4 Sundays of use, 1 song with no license
number recorded." That note lasts for the session
only; it names the file that was saved.

There is no CSV copy in this version. A CSV is a
named follow-up, not built here. The report is the
PDF, and the PDF only.

The report never claims the words were checked and
found true. It lists what the past bulletins
record, with the gaps named alongside.
