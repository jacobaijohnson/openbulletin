# Roles + inserts synthesis — 2026-10-16

Four blind lanes (Sunday school + youth, music + choir,
integration seams, inserts) over Jacob's 2026-10-16 widening:
every church role's part must integrate easily. Lane reports:
~/workspace/openbulletin-audit-wave3/roles-{sundayschool,
music,integration,inserts}.md. Coordinator adjudication
below; this doc is the build authority for the roles track.

## The one seam (lanes converged, coordinator confirms)

Every role is a CONTRIBUTOR, never a second administrator.
A contribution is a small file the one door already eats:
markdown with a short header (From / For Sunday / optional
Keep-through) and named part headings (For the service,
Announcement, Insert, Sunday school, Youth). The same words
work pasted from an email. Senders never need the app; a
sender's card (one printable page) is the whole training.

The administrator's side is an Inbox on the Start step:
parts waiting, each opening into the established
Accept/Edit/Cancel review, PRE-SLOTTED (service lines to
This Sunday fields, announcements to Announcements with
keep-through attached, inserts to the insert list). Two
guards: a wrong-week guard (the review's first line names
the mismatch; a part can be held for its own Sunday, never
lands silently) and a versions guard (same sender + Sunday
grouped, newest on top, duplicates fingerprinted). On
accept, the contribution file moves to Inbox/Filed only
AFTER the bulletin saves; nothing is ever deleted, so "which
title did we print" stays answerable.

Adjudication where lanes differed: the Sunday school lane
also proposed an app-made contribution document kind. That
stands as the app-to-app shape (and the later NAS form's
source), but markdown/paste is the primary seam because
senders do not run the app. Paper stays first-class: the
admin types a paper note into a contribution file once and
it behaves like every other part.

Proofing: a PART PROOF (the reading-copy machinery aimed at
one part) goes back by the channel the part came by.
Approvals stay distinct and DERIVED, never maintained:
contributor approves their words; Check approves the
assembly; later edits clear a part's checked mark by
themselves (the print-freshness pattern). Pastor approval,
if the church wants it, is a recorded mark, never a gate.

Reminders, honestly: the desktop app cannot email or text
and never claims to. Start carries a derived "still waiting
for" list (expected parts live in the Church Profile) and a
copy-a-reminder sentence the admin pastes into her own
thread. Sent reminders are NAS-shape work.

## Per-role builds (joining the loop backlog)

MUSIC (music lane): hymn entries as real fields in the
service lines (hymnal + number + title as parts, cleared
weekly like the sermon title); the MUSIC SLIP (the
contribution shape specialized: hymns to service lines,
notes to announcements); Check's HYMN PASS (number without
title, title without number, pair disagreeing with the
church's own remembered pair naming both, duplicates);
HYMN MEMORY learned from the church's confirmed bulletins
(rides the archive-search build; no bundled database —
Sundays and Seasons suggestions are copyrighted editorial
work, but "on this Sunday three years ago you sang…"
needs no one's permission); a one-click MUSIC PROOF; the
CCLI/ONE LICENSE reporting list produced from past
bulletins instead of reconstructed in a panic.

SUNDAY SCHOOL + YOUTH (lane): Children's message and
Acolytes JOIN the participant fields (they pass the family's
own test: serve inside the service, rotate weekly, cleared
by Next Sunday); teachers do NOT (monthly rotations, serve
outside the service) and live in the class listing instead.
Serving lines become a church-named profile list with the
shipped four as the starting set. Named slots land in the
starter templates (Sunday school classes / news / Youth
group). VBS starter wordings; permission-slip and
registration starter inserts (ride the inserts build).

PRIVACY FOR CHILDREN (lane, highest care): three builds on
shipped precedents. (1) A "NOT FOR THE WEB PAGE" paragraph
mark, sibling to spoken-only: the HTML door drops and counts
it ("for the room, not the web"); print keeps it. (2) A
"children in this picture: yes/no" field on the rights
record; Check warns when no permission is noted; the web
door leaves unpermissioned children's pictures out while
print keeps them. (3) A children's-names rule chosen once
in the Church Profile, shown at typing and repeated at
Check. The honest limit stands: the app records, reminds,
and filters; it never prints "verified."

## Inserts (lane design adopted whole)

An Insert is its own small document attached to the
bulletin: own panels, stories, SheetSpec; fields resolved
against the host at print; exports synthesize a Document so
the ob-pdf funnel, spoken-only filtering, and Check reuse
with no new writer or door. Life cycle rides shipped
machinery: Standing rides silently; Dated follows
keep_through; ThisWeekOnly (default) is removed by Next
Sunday and named in a new ended_inserts list. Print:
inserts impose separately, always; one PDF per sheet in the
dated naming style; the checklist gains an "Also print"
block ("Prayer list — 60 copies, half letter, blue paper")
and the honest sentence that the app prints sheets but
cannot tuck them in. Reading copies exclude inserts;
large print and web inclusion are per-sheet opt-in, web
default OFF (the prayer list is the stated reason). The
library is .obdi files (v1 zip discipline, fingerprint-
verified assets) in an OpenBulletin/inserts sibling folder;
attaching copies in. A file-carrying insert kind (a finished
PDF kept whole, the children's-sheet download case) joins
in phase I2. FORMAT_VERSION stays 1 (serde-default
precedent, no deny_unknown_fields); the caveat — an older
build shows no inserts and drops them on save — is open
question 1 for Jacob with the lane's recommendation to
accept. Phases: I1 model + Next Sunday + print + window +
Check + .obdi; I2 two-up, per-sheet large print,
file-carrying, rescue-to-insert, starters; I3 pew-rack
memory, packs.

## Build order inside the loop

After the round-2 fix batch: (1) WANTED FONTS (Jacob's
directive, separate item); (2) the contribution seam N1–N4
(Inbox + slotted review, sender's card + paste box,
still-waiting + copy-a-reminder, changed-on-disk save
guard — the last is also a general data-loss fix);
(3) hymn fields + hymn pass + music slip; (4) children's
privacy trio + participant extensions; (5) inserts I1;
(6) part proofs + licensing report; (7) inserts I2.

## Shipped addendum (2026-10-16)

Item (4), the children's privacy trio + participant
extensions, SHIPPED as Roles R-2b: the not-for-web
paragraph mark (HTML door only; the template learner
excludes marked paragraphs too, since a learned
template cannot carry the mark), the children-in-this-
picture note on rights records with the Check question
and the web door's permission rule, the children's
names rule on Start repeated at Check and
Announcements, the serving lines generalized
(participants map keyed by slug, profile-named lines,
built-in alias table keeps renamed lines' tokens
resolving; Children's message and Acolytes joined),
and the two new lines in the starter template's
Serving section. FORMAT.md carries the data rules;
EDITOR.md and beyond-print.md the surface.

Item (5), inserts I1, SHIPPED as Roles R-3: the
`Insert` model on the document (own panels, stories,
styles; the host's asset store; field tokens resolved
against the host at print through the synthesized
document and the one export funnel), the three lives
with Next Sunday's `ended_inserts` named in its
report and in the app's status line, the Print step's
"Also print" block (one row per sheet with its own
"Print this sheet" file in the dated style), the two
insert-only Check findings (no words yet; a date that
has passed) aimed at the Loose sheets window, the
window itself with the canvas swap under its banner,
and the .obdi library in its sibling folder with
attach-is-copying. Two reconciliations with the lane
sketch, recorded: the sheet's paper facts live in
their own `InsertSheet` record (shape key, copies,
paper note), not the bulletin's `SheetSpec`, because
sheet shapes are not bulletin folds; and a sheet's
copies are a plain count where 0 means "as many as
the bulletin", the same fact the lane's `Option`
carried. I2 remains: two-up printing, per-sheet large
print, file-carrying sheets, rescue-to-insert, the
pre-filled starters.

## Open questions (Jacob / Pastor Mark only)

From the lanes, deduplicated: which channel senders
actually use first (paste vs file); whether pastor approval
exists at his church and as mark or gate (recommendation:
mark, never gate); expected-parts keyed by role or name
(recommendation: role + current name); outside inserts
(funeral home, district) through the same door?; Filed
retention (recommendation: keep everything; the church's
records call); pictures in contributions v1 or later;
children's names in print today and any opt-out history;
whether a photo permission form exists and who keeps it;
the music director's actual tools and whether a proof
would be used; whether "inserts" is the office's word
(lane offers "loose sheets"); the corpus church's prayer
list: booklet or sheet? (joins the Mark interview set).
