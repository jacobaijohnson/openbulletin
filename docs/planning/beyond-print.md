# Beyond print: formatting depth, HTML both ways, and what else a bulletin system carries

Jacob, 2026-10-15 late: text formatting (markdown, rich
text), HTML in and out, and "what other aspects of the
bulletin should be incorporated in this type of system."

## 1. Inline formatting: the correction, then the build

Verified state today: a paragraph has ONE style. There is
no way to bold one word or italicize a scripture citation
inside a paragraph (completeness audit gap #12; the model
carries style id + string, no runs). Real bulletins need
both constantly. The build, riding the G1 format migration
so the model changes once:

- Paragraph gains runs: spans of text with their own
  bold/italic (and later color/size deltas), style still
  the paragraph's base.
- The shaped editor edits runs without exposing them as
  markup: select words, Ctrl+B / Ctrl+I (keymap-respecting),
  the chip rules stay for fields.
- PDF, previews, IDML/.sla export, and the import builders
  read and write runs. Imports stop guessing italic from
  font names once the dump schema carries it (G1).
- Rich paste: pasting from Word or a web page brings its
  bold/italic across as runs (plain text paste unchanged).
- Markdown: paste or import of markdown text converts
  **bold** and *italic* to runs; export to markdown is a
  Save-a-copy format for the text (for the church website
  and newsletter tools). Markdown is an interchange
  convenience, never the document model.

## 2. HTML export: the web bulletin

Verdict: yes, and it matters more than import. Churches
post bulletins on their websites; a PDF is a poor web
citizen (fixed pages, weak search, poor screen readers).
The HTML export is the bulletin's CONTENT as one
self-contained web page: real text in reading order,
searchable, screen-reader friendly, pictures embedded,
hymn and reading lines intact, credits lines included
(the rights machinery writes them here too). It is a
reading edition, not the print layout in a browser:
panels and imposition do not translate, and the export
says so in its own words. It rides the one export door as
another format, after G2 gives it pictures to carry.

## 3. HTML import: a rescue kind, like the others

Verdict: yes, later, with the standard honesty. An HTML
page (a Mailchimp newsletter export is the real-world
case: churches already write these) imports through the
one door into a draft bulletin: text blocks, headings,
pictures, bold/italic as runs. Layout fidelity is NOT
promised; CSS positioning does not survive, and the draft
review labels it like every other rescue. A tolerant
parser maps block elements to frames; scripts and forms
are dropped with a count, never silently.

## 4. What else belongs in a bulletin system (ranked)

Adopted into the plan:
- **Large-print edition** as an export purpose beside the
  PDF ones: same content reflowed at 16pt+, for the
  congregants who need it. An aging congregation makes
  this a weekly, not a someday, feature.
- **Participant fields**: readers, ushers, greeters,
  communion assistants as This Sunday fields beside the
  sermon title (the J2 pattern), printed wherever the
  template puts them.
- **A privacy flag on prayer-list content**: "print" vs
  "spoken only", so a name meant for the spoken prayers
  cannot wander into the printed sheet by accident.

## 5. The administrator's eyes (Jacob, 2026-10-15)

The large-print edition serves the congregation. The same
fact applies at the desk: church administrators skew
older, and the application itself must conform to their
eyes (Principle 9 applied to the user, not only to the
reader). This is an objective, not a polish item:

- A text-size setting for the app itself (three steps),
  scaling the whole interface, not only the canvas. The
  audit bar: every step of the weekly flow stays usable
  and unclipped at the largest size.
- A high-contrast option beside the standard palette.
- Nothing essential rides on a small target, a subtle
  color difference, or an unlabeled icon. Words on
  buttons (already the naming rule), generous click
  areas, visible keyboard focus everywhere.
- The canvas editing work (E1) already draws text at real
  sizes; the comfort settings govern the chrome around it.
- Verification is partly human (the standing deferred
  accessibility item): the settings ship with the design
  rules above, and the first older-administrator user is
  the real test, the same gate the GUI itself passed.

## 6. Image formats: wide door, conservative exports

(Jacob, 2026-10-15, extended the same night to JPEG XL.)

The recognition door takes every format a church office
can plausibly be handed: PNG, JPEG, WebP, GIF, SVG, and
JPEG XL. Facts checked 2026-10-15: JPEG XL just crossed
into the mainstream (Chrome 155 enables it by default this
month via a pure-Rust decoder; Firefox 158 follows;
Safari has carried it since 2023; iPhone ProRAW uses it),
it beats JPEG by roughly a third on photos, and it can
shrink an existing JPEG about 20% losslessly and
reversibly. The Rust decoding path is pure-Rust (the
image crate's decoder), so support costs no native
dependencies.

The asymmetry from the optimization ruling holds: the
asset store keeps originals untouched whatever their
format; exports transcode to what the destination
actually speaks. PDF cannot carry WebP or JPEG XL as
native image data today, so print and reading PDFs get
JPEG/Flate at the purpose preset, exactly as ruled. The
HTML export may serve WebP/JPEG XL with a JPEG fallback
now that browsers decode them. Encoding JPEG XL
ourselves is deferred: no destination needs it yet.
HEIC (the iPhone default) is the adjacent hard case:
patent-encumbered and library-poor in Rust; it stays an
honest "convert first" label until a clean path exists.

Recorded, not scheduled:
- **Announcement intake**: a simple submission path
  (form or email-in) landing in the Announcements step.
  Real workflow win; touches online surfaces, so it waits
  for the deployment story.
- **Archive search**: every past .obd searchable (find
  last Easter's readings, when a prayer request started).
  Grows naturally from the file library; medium value.
- **Lectionary lookup**: readings by date from a public
  source. Attractive, but licensing and source stability
  need their own pass before it is promised.
