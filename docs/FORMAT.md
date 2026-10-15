# The .obd format

- v0: a single JSON file. `format_version` in the document header; readers
  reject newer-than-known versions with a clear message and accept all
  older ones.
- Saves are atomic (temp file + rename). The GUI adds rolling backups and
  crash recovery in v0.1.
- Geometry is integer milli-points (1/1000 pt). Text styles are referenced
  by id, never copied into paragraphs.
- v1 plan: zip container holding this same `document.json` plus
  content-addressed assets (images by SHA-256), so an inherited file opens
  identically on the next volunteer's machine. Migration fixtures for every
  released version live in CI from the first release.
