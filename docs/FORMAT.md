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

The weekly participant fields are ordinary document
fields (`readers`, `ushers`, `greeters`,
`communion_assistants`), printed by the tokens
`{Sunday.readers}`, `{Sunday.ushers}`, `{Sunday.greeters}`,
and `{Sunday.communion_assistants}`.

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
