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
`{Sunday.<slug>}`. The profile also carries
`serving_aliases` (round 6, R6-C): a map from a
renamed line's OLD slug (its built-in canonical
slug when the old name is a built-in line, else the
slug its name derives) to its NEW slug, recorded
durably at the save that pairs the rename. A
profile written before the record existed loads
with no aliases. When the church renames a line in
the Church Profile and saves, the stored names move
with the rename: saving pairs the old and new lists
by anchored segments
(`ob_model::migrate_serving_line_renames`). The
names present in both lists are anchors, in order;
the runs of old-only and new-only names between
consecutive anchors (and at the ends) are segments.
Within a segment, equal counts pair in order (the
in-place case, immune to additions and removals in
other segments); a segment with only new names is
additions; a segment with only old names is
removals. Exactly one old-only and one new-only
name in the whole edit also pair wherever either
sits (a single rename, with or without a move). A
name present in both lists keeps its names exactly
where they are, so reordering or swapping lines in
the editor moves nothing and crosses nothing. A
followed rename moves the value stored under the
old name's slug (or its built-in canonical slug,
when the value sits under it) to the new name's
slug, clearing the old keys, AND records the
pairing in `serving_aliases`, so the renamed row
shows the names in the editor and the old name's
token keeps printing them through the alias (round
3, R3-3; set-based since round 4, R4-2;
unambiguous-only since round 5, R5-C; segment
pairing and recorded aliases since round 6, R6-C).
When the new name's slug already holds a different
value, both values stay where they are and neither
is discarded. When unpaired old-only AND new-only
names both remain after segment pairing — a rename
crossed an anchor, so its partner sits in another
segment — the pairing is ambiguous and the serving
-lines change is REFUSED AS A WHOLE: the saved list
keeps the old names, nothing is paired, moved, or
cleared, and the status names the kinds of change
the pairing actually saw. For the shape the refusal
was first written for — renames that crossed
anchors, with nothing removed or added — it says
several lines were renamed and moved at once, that
the app cannot tell which names belong to which
line, that the lines were left as they were, and to
rename one line at a time. Every other ambiguous
shape is worded by its own kinds (round 7, R7-10;
round 8, R8-10): the status says the lines were
renamed, removed, added, or moved at once — only
the kinds present, in that order — that the app
cannot tell which names belong to which line, that
the lines were left as they were, and names the
remedy in the same kinds: rename, remove, or add
one line at a time, whichever kinds the edit
contained. A kind is claimed only when the pairing
saw it: `moved` only when names the pairing could
not resolve spread over more than one segment, so
a clean removal or addition standing in its own
segment names its own kind and never a move.
(Round 5 instead kept the new names and
reported the lines "unfollowed" with their names
stranded under the old slugs; that disposition and
its status word are gone.) A name only in the old
list with no partner anywhere is a removed line
(deleted, or renamed onto a line that already
exists): its stored names are cleared at the save,
all its keys, any alias pointing at its slug (or
keyed by it) is dropped so the names cannot
reattach later, and the app names the line as
removed in its status line — in the same save as
any successful renames — so names never vanish
silently (round 4, RT-6b/c). A new line later
taking a removed line's exact name starts fresh and
inherits no alias. The slug is many-to-one, so the
save is also refused, leaving the typed lines in
place and the serving lines as they were, when two
names in the new list derive the same slug or a
name derives a built-in line's canonical (or
shown-name) slug without being the line that
CLAIMS that slug. (The refusal covers the serving
lines only: the profile's other fields save on
their own during the same edit pass, the scope the
status sentence itself states — round 7, R7-11.)
Claimed-ness follows resolution, not spelling: a
built-in's canonical slug is claimed by the
old-list line its token currently resolves to (the
line whose own key is the canonical slug, or the
line the alias record maps the canonical slug to,
through its chain); only the claiming line may take
a name deriving the slug, and the claiming line
itself may always rename to such a name, including
back to the built-in's own name; the refusal
sentence names both lines (round 5, RT5-4; round 6,
R6-4).
Resolution never guesses a line by position. It
honors the slug of the current name, the built-in
slug from the alias table in
`ob_model::PARTICIPANT_FIELDS` (display names matched
case-insensitively), and then the recorded alias
chain from the token's canonical slug, followed hop
by hop (a chain across successive renames works; a
crafted cycle stops at the first repeated slug and
resolves empty, and the walk is depth-capped). A
token that resolves by neither slug nor alias
resolves EMPTY, never to another line's names, so
`{Sunday.childrens_message}` and `{Sunday.acolytes}`
keep resolving for values saved before the rename —
through the recorded alias, not their old position
in the list.
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

A document may carry `inserts` (roles build R-3): the
loose sheets that ride inside the bulletin without being
pages of it. Each insert is its own small document:
`{ id, name, life, keep_through, panels, stories,
styles, sheet }`, where `sheet` is `{ preset, copies,
paper_note }` (the shape key names sheet shapes:
"half-letter", "letter", "card"; `copies` 0 means "as
many as the bulletin"; `paper_note` is plain words like
"blue paper", empty meaning plain paper). An insert's
picture frames name assets in the HOST document's store
by SHA-256, so a photo used by both is stored once. Its
words resolve their field tokens against the host at
print time: at export the insert is synthesized into a
document (its parts on the host's profile, fields, and
asset store) and printed through the same funnel as the
bulletin, spoken-only filtering included. Inserts never
enter the bulletin's imposition; each prints as its own
file. Files written before the field existed load with
no sheets; `format_version` stays 1. (The known cost of
staying at 1: a build from before loose sheets opens
such a file, shows no sheets, and drops them on save.
That exposure was accepted, the same one the
spoken-only flags already carry.)

A document may carry `applied_contributions` (round 4,
R4-4): the content fingerprints of contributions
already applied to this document, recorded at the
moment the words land, and only when every part of
the contribution landed completely (round 5, R5-B):
a part that landed partly or not at all leaves no
record, so it can be offered again. The record is a
note, not the words: before the review says a part
is already in the bulletin, it checks the words
themselves are still in the document, and a record
whose words were deleted by hand offers the part
again instead of blocking it. The record lives in the
document, not in the app's memory, so it survives a
save that failed, a save done by hand, and a crash
the recovery journal restores from; Undo removes a
fingerprint with the words it describes, and Next
Sunday clears the list for the new week. It is
bookkeeping only: never printed, and no export door
reads it. It lives only on the bulletin document; a
sheet's `.obdi` file has no such list (a sheet is not
a bulletin and applies nothing). Files written before
the field existed load with an empty list;
`format_version` stays 1. (The
same accepted cost as the fields above: a build from
before the field existed drops the list on save, and
a part could then be offered for applying twice.)

An insert's `life` is `standing`, `dated`, or
`this_week_only` (the default). Next Sunday follows it:
standing sheets ride untouched and unmentioned; a dated
sheet rides while its `keep_through` date (a
`{ year, month, day }`, with a story keep-through's
meaning) is on or after the new Sunday, and is removed
and named in the report's `ended_inserts` once it has
passed (a dated sheet with no readable date is removed
too); a this-week-only sheet is removed and named.

### The .obdi sheet file

A loose sheet saved to the library is one file,
extension `.obdi`: the same v1 zip discipline under
another root. It holds `insert.json` (the insert, plus
the asset records of the pictures it uses) and one
`assets/<sha256>` entry per picture the sheet's frames
use, fingerprint-verified on load with the same caps
and the same refusal words as a bulletin. Library files
live in an `inserts` folder beside the Font Library
folder. Attaching a library sheet copies it into the
document under a fresh id and merges its pictures into
the document's store; from then on the bulletin travels
complete, and editing the attached copy never writes
back to the library. When the bulletin already holds a
picture the sheet carries, the two rights records merge
field by field (round 4, R4-5): the sheet's record fills
only what the bulletin's leaves empty, a value already
noted is never overwritten, and `children_present`
merges as OR, so a "children are in this picture" from
either side stands. The attach says how many records it
actually filled in, and says nothing when the merge
changed nothing.

Duplicate records for one picture fold on load (round
5, RT5-1): a bulletin file or sheet file whose JSON
carries two records for the same asset hash loads with
one composed record, merged field by field by the same
rule, the children note included. No reader ever sees
the shadow copy, and a copy saved onward carries the
one folded record. One property rides with the merge,
in any order (round 5, F2; round 6, R6-1): no merge
manufactures a permission. If the composed record
notes children in the picture and notes a permission
(a credit line, or a license that itself counts as
one), that permission must be vouched for: the field
that satisfies it stands only when the record it came
from itself notes the children, or a children-bearing
record already noted a permission on its own fields
before the merge. Otherwise the field reverts to the
children-bearing record's value for it. A license and
its credit also travel together (round 6, RT6-2;
round 7, R7-1): when a merge takes a record's license
but that same record's credit line does not stand in
the composed record, the license does not fill
either — and the rule judges the composed record,
not the direction, so a license that requires
attribution (CC BY, CC BY-SA) already standing on
the record likewise does not stand when the
composed record notes children and its credit line
is empty — the mirror case being a merge whose
only credit was refused by the fence above. Either
way, no composed record that notes children claims
an attribution license with an empty credit line,
and a record whose own credit line stands keeps
its license. When an attribution license falls
this way in a merge, the other record's license
stands in its place only when that license itself
stands under the same rules — its own record notes
the children, or a children-bearing record already
noted a permission — and that condition is the
same for every fallback license, a reprint license
(CCLI, One License) included (round 8, R8-3): an
unvouched fallback license falls to unknown like
any other. In a fold of duplicate records the
togetherness judgment runs once, on the final
folded record, so a credit line arriving later in
the same fold stands beside the license it belongs
to (round 8, R8-9); single merges outside a fold
judge each pair as before.

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
