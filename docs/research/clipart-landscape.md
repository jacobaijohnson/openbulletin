# Clipart landscape, 2026-10-15

Jacob's ask: users should be able to import their own clipart
collections, and are there open collections we can bundle or
offer so art works out of the box? Researched against the
collections' own sites and license pages (URLs cited inline,
checked 2026-10-15), plus the licensors' own pages for the
attribution and music-reprint section Jacob added the same
evening.

Short answers: yes to both halves, with one bright line.
Bundling is clean only for public domain / CC0 art, verified
item by item. The big general collections (Openclipart and
its successor FreeSVG) are CC0 by policy and strong on
church subjects. Everything else is per-item, per-set, or
attribution-carrying, which is exactly what user imports and
downloadable packs are for. Section 5 designs the rights
record that makes the attribution-carrying half honest:
every asset in a document remembers where it came from and
what credit it owes, and the app composes the credit lines
itself.

## 1. The open collections

### Openclipart (openclipart.org)

- License: every upload is dedicated under CC0 1.0; the
  site's FAQ (https://openclipart.org/share) says clipart may
  be used even for unlimited commercial purposes and that no
  citation is required.
- Size and shape: the FAQ claims 180,000+ files. SVG is the
  canonical form; PNG derivatives are offered per item.
- Health: mixed. The site went dark for a long stretch and
  its community re-formed at FreeSVG (below); the site is
  live again and old detail pages resolve, but treat it as a
  source to draw from, not infrastructure to depend on.
  Historical bulk exists: numbered Open Clip Art Library
  release packages and distro packages (e.g. the
  `openclipart` package in Linux distributions), so a
  one-time bulk harvest for curation is practical without
  scraping.
- The standing caveat is provenance, not license text: CC0
  is applied on the uploader's say-so. Volunteer librarians
  tag doubtful files `pd_issue` when origin or rights are in
  question. Anything flagged, anything that looks like a
  traced photo or a trademark, is out.
- Church coverage: the strongest of any collection here.
  Two decades of volunteer uploads include crosses in many
  forms, doves, candles, Advent wreaths, nativity scenes,
  Easter lilies and empty tombs, wheat and harvest art,
  baptism and wedding art. Quality varies item by item;
  that is what curation is for.

### FreeSVG (freesvg.org)

- License: CC0 site-wide, stated on its about page
  (https://freesvg.org/about): copy, modify, distribute,
  even commercially, no permission, attribution appreciated
  but not required.
- Shape: SVG only, actively updated (new uploads daily),
  per-item download pages. No official whole-library archive
  was found; it is a picking source, not a bulk source.
- Relationship: founded by Openclipart contributors after
  the Openclipart outage, and much of the Openclipart corpus
  lives there now. For curation purposes the two are one
  well: CC0 policy, uploader-asserted, same provenance
  discipline (skip doubtful items, prefer named long-standing
  contributors).

### SVG Repo (svgrepo.com)

- Shape: a large aggregator (hundreds of thousands of
  vectors), icon-weighted rather than scene clipart.
- License: per item, and this is the whole story. Its
  licensing page (https://www.svgrepo.com/page/licensing/)
  describes several families side by side: a public-domain /
  CC0 tier (the page's default for simple icons unless the
  item page says otherwise), open-source licenses carried
  from source packs (MIT, GPL named on the page), and
  attribution-carrying items. Each item page shows its own
  license label. Third parties have also reported items
  whose labels do not match their original pack's terms, so
  the label is a lead, not proof; follow the source.
- Site terms add a second layer: the same licensing page
  bars redistributing the material as a like-for-like
  collection and reselling it as-is. Placing individual
  PD/CC0 items into bulletins is ordinary use; mirroring
  SVG Repo as a browsable in-app library would collide with
  the site's own terms even where items are public domain.
- Verdict: a fine place for a user to fetch one missing
  icon; not a bundle source and not a pack source.

### Wikimedia Commons (commons.wikimedia.org)

- Not one collection and not one license. Every file page
  carries its own license: public domain, CC0, CC BY,
  CC BY-SA, and legacy GFDL all coexist. Commons' licensing
  policy (linked from every file page as Commons:Licensing)
  admits only freely licensed media, but "free" includes
  attribution and share-alike terms that must travel with
  the file.
- Mechanics: a real API, category trees, and full data
  dumps exist, so bulk work is possible; the work is
  per-item license filtering, which the API exposes.
- Church coverage: unmatched for heritage art. Classic
  religious painting (much of it public domain), church
  architecture photography, and one set worth naming: the
  Sweet Publishing Bible illustrations (Jim Padgett's
  picture-Bible art), released to Commons under
  CC BY-SA 3.0 with the rights-holder's permission on file
  (see any file page, e.g.
  https://commons.wikimedia.org/wiki/File:Book_of_Genesis_Chapter_25-8_(Bible_Illustrations_by_Sweet_Media).jpg).
  Hundreds of Bible-story scenes, genuinely useful for
  bulletins and children's pages, and share-alike, which is
  why they are pack/user territory and not bundle material
  (section 2).
- Verdict: the deep well for downloadable packs and user
  imports, harvested per item with its license recorded.

### The Noun Project (thenounproject.com)

- Icons only: single-glyph pictograms, no scenes, no
  borders. Useful for a missing pictogram, not a clipart
  library.
- Terms (https://help.thenounproject.com/hc/en-us/articles/200509798-What-licenses-do-you-offer-for-icons
  and the legal terms at thenounproject.com/legal): free
  downloads are CC BY 3.0 with attribution, or marked
  public domain where the creator chose that. The paid
  royalty-free option does not change the license; it buys
  out the attribution condition for the buyer. That waiver
  is a contract with the buyer and does not transfer to
  anyone the buyer ships files to, so a subscriber's
  downloads cannot become our bundle.
- Verdict: not a bundle or pack source. Individual
  public-domain-marked icons are fine for a user to import
  like anything else.

### Museum open access (the heritage route)

- The Met's Open Access program marks public-domain works
  CC0 and offers a keyless API with a per-object
  public-domain flag (https://www.metmuseum.org/policies/frequently-asked-questions-image-and-data-resources;
  API at collectionapi.metmuseum.org). Cleveland Museum of
  Art (CC0 filter in its open-access API) and the Art
  Institute of Chicago (CC0 images via its API;
  https://api.artic.edu/docs/) work the same way; the
  National Gallery of Art publishes an open-access dataset.
- These are photographs of classic art, JPEG at print
  resolutions, with deep religious holdings. Redistribution
  is the point of the programs.
- Verdict: the clean source for a small set of classic
  cover images (a nativity painting for Christmas Eve, an
  Easter image), offered as a downloadable pack with the
  museum and title in each record. Not line clipart; do not
  sell it as such.

### FreeBibleimages (freebibleimages.org)

- Bible-story image sets, free to download, but rights are
  per set under each set's Terms of Download, and the site
  says so plainly (https://www.freebibleimages.org). The
  large Sweet Publishing sets are CC BY-SA 3.0; other
  contributors' sets carry their own terms, and some sets
  add faith-use conditions (the images are for retelling
  Bible stories faithfully and not for messages undermining
  the Christian faith) that go beyond any CC license.
- Verdict: a good door to point users at, and its sets
  import cleanly as user collections with the set's terms
  recorded. Not bundle material: share-alike plus
  per-set conditions plus content restrictions is three
  reasons.

### The trap shelf (named so nobody walks into it)

- "Free clipart" search-result sites routinely label
  downloads "personal use" in the fine print while the page
  headline says free. Personal use is not redistribution
  and not church printing at scale.
- Flaticon's free tier requires a credit line and link and
  does not permit redistributing the art as a collection.
- Stock-house "free" sections (Shutterstock, iStock and
  friends advertise beside the open sites) are ordinary
  commercial licensing.
- Rule of thumb for the docs and the app both: if the
  license is not stated on the item's own page in words we
  can record, the item is not open, whatever the site name
  says.

## 2. The license line

**Bundled in the shipped app: public domain and CC0 only,**
each item individually verified (section 3), with the
source page and its license recorded in a manifest that
ships with the pack. This matches the fonts rule in
docs/FONTS.md (bundle only what the license plainly allows,
license text stored alongside).

What the other licenses would cost, and where they are
allowed instead:

- **CC BY** is welcome in downloadable packs and user
  imports, never silent. Its cost is machinery, and
  section 5 is that machinery: a per-item rights record,
  credit lines composed onto the bulletin automatically, and
  a Check pass that catches a placed item whose credit is
  missing. Without the machinery, CC BY art in an office
  worker's hands becomes unattributed art by Friday. With
  it, CC BY packs (much of Commons' best church photography
  lives here) become practical.
- **CC BY-SA stays out of the bundle.** Unmodified
  redistribution with attribution and license text is
  lawful, but bulletin use is rarely unmodified: a crop, a
  resize-to-column, a duotone for a one-color copier are
  the daily bread, and share-alike then speaks to the
  adapted image. Keeping the bundle PD/CC0 keeps the
  promise simple: art in the box is art you cannot get
  wrong. SA sets (the Sweet Publishing illustrations above
  all) are first-class user imports with their terms on the
  record.
- **NonCommercial (any -NC license) is out everywhere we
  choose.** Churches mostly publish free bulletins, but
  they also sell cookbooks, charge for the harvest supper,
  and hand the same art to the preschool fundraiser.
  "Noncommercial" has no bright edge there, and an app
  shipped to thousands of churches should not pretend it
  does. NC items are the user's own call in their own
  library, labeled as what they are.
- **NoDerivatives** items cannot be cropped or recolored,
  which bulletin layout does constantly; treat like NC:
  user territory, labeled.
- **Trademark is a separate fence CC0 does not cross.** A
  CC0 drawing of a denominational emblem (the United
  Methodist cross-and-flame is a registered mark, for
  example) is still the denomination's mark. The starter
  pack carries no denominational emblems and no brand
  logos, whatever their copyright status.
- **Site terms can fence what copyright leaves open**
  (SVG Repo, section 1). A pack is built from sources
  whose terms allow the pack to exist.

## 3. The starter pack proposal

A small set in the box, not a library: **40 to 60 items,
one style family, at or under 1 MB total** (hard cap 2 MB).
Single-color line and silhouette SVGs, so a cross from the
pack sits comfortably next to a dove from the pack and
both survive a black-and-white copier, which is the real
press most bulletins meet.

Coverage list (the church year plus weekly furniture):

- Crosses: plain, budded, holding/anchor cross; an empty
  tomb; Easter lily; palm branch; crown of thorns kept
  tasteful or omitted (curator's call, leaning omit).
- Advent and Christmas: wreath, candle (one, and a row),
  star, angel, nativity silhouette, three kings silhouette.
- Lent and Holy Week: ash cross mark, chalice and host,
  wheat sheaf and grapes (communion), fish (ichthys),
  anchor, lamb.
- Pentecost and ordinary time: dove (two poses), flame,
  open Bible, church building, praying hands, music notes
  and a treble clef, bell.
- Life events: wedding rings, baptism shell with drops,
  candle for memorials, baby/ cradle kept generic.
- Seasons and thanks: harvest sheaf, cornucopia, leaf,
  snowflake, flower sprig, sun.
- Furniture: two corner ornaments and one divider rule
  that match the line weight of the rest.

How items get chosen: one curator pass over Openclipart /
FreeSVG with a written checklist per item: (1) open the
source page and record URL, author handle, and the CC0
statement; (2) check for `pd_issue` or any provenance
doubt and reject on doubt; (3) render the SVG and look at
it at bulletin size and at thumbnail size; (4) confirm it
is drawn, not traced from a photo or a stock preview;
(5) normalize (strip editor cruft, set a viewBox, single
ink color) and record that we did, in the manifest. The
manifest (item, source URL, author, retrieved date,
license, our edits) ships inside the pack and doubles as
the seed data for the rights records in section 5.
Rejected candidates are listed in the manifest's working
notes with the reason, so next year's curator does not
re-litigate them.

An endorsement note, plainly: whatever ships in the box,
the project is vouching for. Small, plain, and safe beats
large and lively. No humor clips, no mascots, no
denominational marks, nothing a funeral bulletin would
regret.

## 4. The library design (user imports)

The fonts precedent (docs/FONTS.md) carries over almost
unchanged: one app-managed **Clipart Library** folder,
desktop and Docker shapes identical, everything copied in
and never live-linked, so a bulletin made on the office PC
and one made on the NAS see the same library rules.

- **Collections are folders.** The library holds named
  collections ("Church basement scans", "VBS 2024",
  "Sweet Publishing: Genesis"). A collection is a folder
  of image files plus one small notes file recording where
  the collection came from and the rights note the user
  asserts for it (section 5's collection-level default).
- **Add a collection** offers three doors, one flow: pick
  a folder; pick a .zip (unzipped under the same guards as
  font-pack zips, including path-escape refusal); or pick
  loose files, which land in a collection named for the
  day ("Added 2026-10-18") until renamed. Files are copied
  into the library; originals are never touched. Accepted
  on sight: SVG, PNG, JPG/JPEG, GIF, WebP, BMP, TIFF (the
  image crate's decode set plus the G2 SVG path). Anything
  else is counted and named in the result sentence, never
  silently dropped.
- **Publisher-era reality check, stated in the import
  result:** old clipart packs (Microsoft Clip Organizer
  galleries and the CD collections churches still own) are
  heavily WMF, with EMF second. Section 4.1 is the honest
  answer for those files.
- **Indexing without AI.** The index is the collection
  name, the folder path inside the collection, and the
  words in each filename ("advent-wreath.svg" finds
  "advent" and "wreath"). No tagging engine in v1; a
  rename in the picker renames the file, which is the
  whole tagging story, and it is enough for office-scale
  libraries. The index is a small file rebuilt
  incrementally, not a scan on every open.
- **The picker** shows collections on one side, a
  thumbnail grid on the other, and a name search across all
  collections. Thumbnails render lazily as the grid
  scrolls and are cached on disk beside the library (a
  rendered thumb is derived data; deleting the cache is
  always safe). SVG thumbs come from the same resvg path
  as placement, so what the picker shows is what the page
  gets.
- **Placing copies, always.** Placing a clip copies its
  bytes into the document's asset store (the G1
  content-addressed store) together with its rights record
  (section 5). The .obd then travels complete; the library
  can be reorganized or lost without breaking old
  bulletins. Library items are never edited in place by
  the app; crop and fit live on the placed frame (G2), and
  the stored asset is the original bytes, per the
  optimization boundary (full quality in the file,
  derivatives only in exports).
- **Removing a collection** removes the library copy only,
  and says so; documents that already placed its art keep
  their copies.

### 4.1 WMF and EMF: the honest fallback

Can anything license-clean read the old metafiles? The
formats are documented ([MS-WMF] and [MS-EMF] are
published Microsoft specifications), so the door is not
locked the way .sib is. But the reader shelf today:

- **libwmf** is the established C renderer (the one behind
  ImageMagick's WMF delegate). Its licensing paperwork is
  genuinely untidy: the project's own tracker carries a
  long-open report that the shipped COPYING file in some
  releases is the GPL text while the library is understood
  to be LGPL (https://sourceforge.net/p/wvware/bugs/161/).
  Untidy paperwork is exactly what docs/LEGAL.md exists to
  keep out of the bundle; libwmf does not ship with us.
- **LibreOffice** renders both formats well, because it
  has to. That makes it the same shape as the .pub
  stopgap: a user-installed bridge, never a bundled
  dependency.
- **Rust-native:** young converters exist (the emf-core /
  wmf-core crates convert EMF, with WMF fallback, to SVG;
  self-described work in progress, small
...[truncated 16856 chars]