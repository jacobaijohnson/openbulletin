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

Migration fixtures for every released version live in CI
from the first release.
