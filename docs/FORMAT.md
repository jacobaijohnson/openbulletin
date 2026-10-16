# The .obd format

## v1 (current, `format_version: 1`)

A v1 file is a zip containing:

- `document.json`: the whole document (words, styles, stories,
  panels, the church profile, and the asset table below), as in
  v0. Geometry is integer milli-points (1/1000 pt). Text styles
  are referenced by id, never copied into paragraphs.
- `assets/<sha256>`: the bytes of everything the document
  carries (tracing pictures, pictures rescued from old files,
  later music and clips), one entry per asset, named by the
  SHA-256 of its bytes. The same bytes are stored once, however
  many panels use them.

The asset table in `document.json` records, per asset: its
fingerprint, its kind (picture / tracing picture / music /
other), its media type, and a rights record (source, license,
title, creator, the copyright line, who noted it and when), so
credit lines can be composed from the file itself. The bytes
never appear inside `document.json`; they live only in the
`assets/` entries.

A panel's tracing picture names its asset by fingerprint in
v1 documents. Older field names from the first weeks
(`sunday_date`, `sunday_name`) are renamed to the occasion
names (`occasion_date`, `occasion_name`) in memory at load;
the printed tokens `{Sunday.date}` and `{Sunday.name}` in
existing templates keep working, and `{Service.time}` reads
the document's service time.

A paragraph's words are a list of runs: pieces of text with
optional bold/italic switches (absent means the paragraph
style's face speaks). Files written before runs existed
store paragraphs as `{ style, text }`; that shape still
loads (the words become one plain run), and saving always
writes the runs shape. The container version did not need
to change for this: the paragraph reader accepts both.

A story's words are stored once, however many frames name
the story. Since T1 (2026-10-16) several frames may
reference the same story id: the story is threaded through
those boxes in document order (panel order, then frame
order within the panel), and each box draws the part of
the story that flows into it. Frames always named their
story by id, so threading needed no format change:
`format_version` stays 1, and a file written before T1
reads exactly as it did.

A paragraph may carry `spoken_only: true` (Extras,
2026-10-16): it is read aloud in the service and never
printed. The field is absent (read as false) in files
written before it existed, so no version change was
needed. The .obd keeps spoken-only paragraphs; every
export leaves them out and counts them.

A paragraph may carry `web_hidden: true` (R-2b,
2026-10-16): it prints in the bulletin but stays off
the web page. A printed bulletin is a room; the web
page is the world, and some words (children's names
above all) are for the room only. The discipline is
spoken-only's: the field is absent (read as false)
before it existed, and only the HTML export leaves
these paragraphs out, counting them ("N paragraphs
were left out of the web page; they are for the room,
not the web"). Print, IDML, Scribus, and markdown all
keep them. A paragraph carrying both marks counts
once at the web door, as spoken only. The template
learner never learns either kind: a learned template
could not carry the marks, so the words stay out.

An asset's rights record may carry `children_present`
(R-2b): `true`, `false`, or absent (nobody has said
yet). Check asks about each placed picture with no
answer. On the web page, a picture noted as having
children in it rides only when a permission is noted
on the record (a credit line written down, or a
license needing no one's permission: the church's own
work, public domain, CC0); otherwise the page leaves
a note where it belongs, and the door counts it.
Print never drops a picture over this. The record is
the office's own note, not a proof.

The serving lines live in two places (R-2b). The
church profile carries `serving_lines`: the display
names the church prints each week for the people
serving ("Readers", "Acolytes", and the church's
own), in shown order. A profile without the field
serves the six built-ins (Readers, Ushers, Greeters,
Communion assistants, Children's message, Acolytes).
The document carries `participants`: this week's
names, keyed by serving-line slug. A value is stored
under the slug of the line's name AS SHOWN (lowercase,
non-alphanumerics become underscores, trimmed). A
template prints a line's names with the token
`{Sunday.<slug>}`, and resolution honors BOTH the
slug of the current name and the built-in slug from
the alias table in `ob_model::PARTICIPANT_FIELDS`
(display names matched case-insensitively), so
`{Sunday.childrens_message}` and `{Sunday.acolytes}`
keep resolving after the church renames a line.
Before the map existed, the four original lines kept
their names in ordinary document fields under their
slugs (`readers`, `ushers`, `greeters`,
`communion_assistants`); loading folds those into the
map while the map is empty, and resolution still
reads a leftover field as a last resort. Next Sunday
clears every value in the map and names the lines
that had names by their profile display names.

The church profile may carry `children_names` (R-2b):
`FullNames` (the default), `FirstNamesOnly`, or
`NoNames`, the church's rule for naming children in
the bulletin, chosen once. The app does not enforce
it: it cannot tell which names are children's. It
shows the rule beside the Announcements box and
repeats it at Check as a reminder to look.

A document may carry `wanted_fonts` (2026-10-16): one
entry per font the document was made in that the Font
Library did not hold when it was imported, each
`{ name, substituted_by }`, the real font's name as a
person would search for it and the free family that stood
in at import time. The styles keep the real family name
too. The record is information, never a lock: shaping and
printing resolve the family against the library as it
stands at that moment, so `substituted_by` may go stale
on purpose (the real font arrived, and the document now
uses it with no edit). Files written before the field
existed load with an empty list; `format_version` stays 1.

### Loading rules

- Readers reject newer-than-known versions with a clear
  message and accept all older ones.
- A v1 asset whose bytes do not hash to the name it travels
  under is refused in plain words (the file may be damaged);
  it is never silently used.
- Guards: `document.json` may not exceed 64 MB, and the
  total uncompressed size of a v1 file may not exceed
  512 MB; both refusals say so in plain words. The total is
  summed from the zip directory before anything is
  decompressed.
- Saves are atomic (temp file + rename) and always write
  v1. The GUI keeps an autosave journal (the current work,
  rewritten as you edit, in this same format) beside its
  settings; after a crash it offers that copy back. There
  are no rolling backups of older saves.

## v0 (read-only legacy)

A v0 file is a single bare JSON document, `format_version: 0`.
It still opens: the document migrates in memory (occasion
field names; tracing pictures named by a file path beside
the .obd are read in and become assets, and a missing
sidecar file leaves the panel's missing-picture note
exactly as v0 behaved), and the next save writes v1.

Migration is covered today by tests that carry their
example documents inline in the test code. Fixture files
for every released version join CI at the first release.
