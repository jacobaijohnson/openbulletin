# Content fingerprints

OpenBulletin recognises when the library already holds a
picture, a tune, or an announcement — by what it IS, not
by its file name or its dates. A renamed file is still
the same file. A retyped announcement is still the same
words.

## The three tiers, in plain words

Nothing else ever reaches the user. There are no scores
on screen, and never a bare percentage.

- **Same file.** The file bytes are identical
  (SHA-256). This is the app's existing identity.
- **Same content, different file.** A canonical-content
  hash is equal. This tier is exact, so it may group
  silently. It never deletes anything, at any tier.
- **Probably the same — please check.** A similarity
  band was reached. This is always a proposal that asks
  the user, with a concrete reason: how many picture
  details differ, which word-groups are shared, whether
  the tune is in another key.

Music has two extra labels inside the asking tier:

- **Same notes, different spelling — please check.**
  The same notes with different spelling (an enharmonic
  respelling, for example). Only the full
  spelling-sensitive canonical hash earns the silent
  tier; MIDI-pitch equality gets this label, never
  silent grouping.
- **Same melody, different harmony — please check.**
  The same melody with different harmony around it.
  Melody equality only proposes, never groups silently.

## What never happens

- Nothing is ever silently merged or deleted, at any
  tier. A proposal asks; the user decides.
- A rejected pair is never re-proposed. Marking a pair
  "Not duplicates — remember" stores the dismissal in
  the index, and the pair never appears again.
- Fingerprints are an index, never the record. The
  file itself learns nothing about where or how often
  it is used. The index never stores the words of an
  announcement — only hashes and word-group sets — so
  a short announcement's comparison needs its source
  texts in hand (see below).
- Everything is computed offline, at import, and
  persisted. Nothing is recomputed except when the
  fingerprint version changes. Nothing here touches
  the network, and nothing reports anywhere.

## Pictures

Two 64-bit perceptual hashes, computed in-tree
(published test vectors pin them; no image_hasher
dependency):

- **pHash**: the picture reduced to 32×32 grey,
  orthonormal DCT-II, the top-left 8×8 coefficients
  rounded to 6 decimals before median thresholding.
  The rounding matters: without it, solid and gradient
  pictures hash on floating-point dust and two correct
  implementations disagree.
- **dHash**: gradient bits over a 9×8 grid.

A probable match needs BOTH hashes to agree: at most
5 differing details each for "very likely", at most 10
each for "possibly". A coarse 4×4×4 colour histogram
guards the match: a recoloured picture is a different
picture, however close its grey hashes (total-variation
veto at 0.5). Simple drawings on a plain background
get a low-information flag: the flag changes the
wording ("look before you merge"), never suppresses
the proposal.

## Music

A canonical note-event stream from MusicXML (the
`.mxl` zip comes through the same loader as the
music door, so it is never re-implemented):

- Per-part, per-voice streams in document order.
  `<duration>` is the sounding length in divisions
  units, so dots and tuplet scaling are already inside
  it and are never applied again. Ties merge by
  spelled pitch per voice, so chord tones merge
  independently. Grace notes are skipped; rests break
  open ties; a missing duration, divisions, or pitch
  fails loudly.
- Three hashes: the spelling-sensitive canonical hash
  (silent tier), the MIDI-pitch hash (own label), the
  melody hash (proposes only). Melody means the first
  part's first voice, top note per onset; whether the
  top note arrived alone or as a chord tone is not a
  melodic difference.
- Similarity is two separate scores, never one number:
  pitch similarity over melody semitone intervals
  (rests skipped, chain unbroken) and rhythm similarity
  over melody onsets. Very likely needs pitch ≥ 0.95
  and rhythm ≥ 0.90; possibly needs pitch ≥ 0.85 and
  incipit containment ≥ 0.80. The incipit is the first
  15 melody notes as scale degrees, and the Plaine &
  Easie string (first 12 onsets, per the IAML spec) is
  a display and interchange string, never a match key.
- Transposition is invariant by construction for
  intervals, and a transposed tune is detected exactly
  (intervals and rhythm equal, hashes differ) and said
  so: "the same tune in another key".
- Candidate gating is mandatory: scoring runs only
  over incipit-prefix buckets, interval 4-gram buckets,
  and a length filter — never naive all-pairs edit
  distance over the library at query time. (The
  `duplicates` report is the one exception, by design:
  a full audit report is all-pairs because its job is
  to miss nothing. A query must answer now; a report
  must be complete.)
- Audio fingerprinting is deliberately out: it
  identifies recordings, not tunes. This library holds
  notation.

## Text

- Normalisation: Unicode NFC, lowercase, punctuation
  unified to separators. Numbers, dates, and names are
  KEPT: normalising them away would make different
  weeks' announcements match.
- Full 3-word shingle sets with EXACT Jaccard and
  containment. No MinHash approximation at this scale:
  MinHash is a lossy compression of exactly that set,
  designed for web-scale corpora, and on a 30-word
  prayer its estimate is statistically coarse (standard
  error sqrt(J(1-J)/k), about 0.044 at k=128, before
  the small-document degeneracy where a short document
  has fewer shingles than the signature length). If
  MinHash ever appears here, that formula is the error
  bar that must be stated.
- Winnowing (k=5 words, w=4) with positions, so a
  shared passage is locatable on the review screen:
  any shared run of at least 8 tokens yields at least
  one shared fingerprint.
- SimHash (64-bit) is computed and stored but never
  used to exclude a match: at this scale exact set
  intersection is cheaper than a filter miss, and a
  filter that can miss a true duplicate is worse than
  a slower exact pass. Past roughly 100,000 blocks,
  gate on SimHash distance ≤ 6 first, then re-score
  exactly.
- Boilerplate guard: word-groups present in more than
  25% of the library are furniture, not evidence.
  When the library frequency map is available, those
  groups are excluded from both sides before scoring
  (and the reason says so). If exclusion empties both
  sides, there is nothing distinctive shared: no match,
  never a vacuous empty-equals-empty score.
- Short announcements (under 15 tokens) never get
  shingle scores: their sets are too small for the
  bands to mean anything. They fall back to direct
  token comparison with the diff always shown, and
  propose when at least 80% of the shorter text's
  words are shared. A single changed word in a
  five-word prayer line is exactly the pair the user
  needs to see, and the diff lets them judge in one
  glance.
- Bands (literature starting points, not calibrated on
  church data): Jaccard ≥ 0.90 very likely, 0.70–0.90
  possibly, containment ≥ 0.85 at any Jaccard.

## Where fingerprints live

A library-side JSON index, conventionally
`<base>/OpenBulletin/fingerprints.json`, beside the
Font Library. Not in `.obd` documents: no document
schema change was needed, so the post-release
format-version rule (FORMAT.md) was never triggered.
The app owns the path and hands it to the index.

- Every fingerprint is stamped with its `fp_version`
  (algorithm + parameters bundled). Only equal
  versions are ever compared. A version bump
  recomputes from the originals; stale entries never
  match and are listed for recompute.
- A missing index file is a fresh library. A damaged
  index file is a loud error, never a silent empty
  index — silently starting empty would drop
  dismissals and re-propose rejected pairs.

## Using it

- `openbulletin-cli fingerprint <file>`: what one
  file's fingerprint is (picture by magic bytes, music
  by extension, anything else as readable text).
- `openbulletin-cli duplicates --index <path>
  --add <files...>`: add files to the index, then
  print the report — every proposal, every short pair
  that needs its words compared by hand, and every
  stale entry waiting on a recompute. Files already in
  the index are skipped, never entered twice.

## Still open (follow-ups, not findings)

- **App import seam.** The GUI import door has no
  single funnel today: reads land at many call sites
  in the app, and the document asset funnel is the
  wrong layer for library index I/O. The seam needs a
  library-import funnel first, as part of the
  Duplicates view build. The CLI covers the engine
  end to end meanwhile.
- **Duplicates view in the GUI.** The engine, tiers,
  index, and CLI report are built; the in-app proposal
  view (with the shared-passage display and the
  "Not duplicates — remember" button) is the next
  surface, and no human has clicked it yet.
- **Band calibration.** All bands are literature
  starting points. They should be revisited once real
  church libraries exercise them, and any change
  re-stamps the affected `fp_version`.
