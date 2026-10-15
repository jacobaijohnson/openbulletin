# UI/UX Audit — OpenBulletin (Muse Wonder pass)

Basis: a direct read of the implementation in `crates/ob-app/src/main.rs`, judged
against the project's own law in `docs/PRINCIPLES.md`. Every finding quotes the
code or the exact UI string behind it. Function names are given instead of line
numbers because the audited source was supplied without them.

Standing caveat, from the brief itself: no human has ever run this GUI. This
audit can prove what the code says and does; it cannot prove how anything
feels. Several findings below (especially around the canvas) are "the code
permits this failure," not "we watched it happen."

## Completeness verdict

Not yet end-to-end usable by an unaided church office administrator. The
weekly loop is genuinely present — five steps, plain language in most
places, a real shaped print preview, autosave with recovery, a learned
printer setting — and the skeleton honors the principles. But six blockers
stand in the main path: two silent work-loss routes, an undo stack that
crosses file boundaries, a cover date the user cannot set or fix, an
autosave tick that can erase typing done on the page, and a main preview
that misrepresents the bulletin it claims to show. An admin can produce a
bulletin with this build; she cannot yet trust it on a Saturday night,
which is the project's own release bar (Principle 6).

## The journey, stop by stop

**First launch.** The app opens on a ready-made bulletin for a church she
does not belong to, dated today, with the status line
`"Welcome to OpenBulletin."` and the Step 1 guidance. Nothing says "this is
a sample; open last week's file or make it yours." The one action her real
week starts with — opening last week's file — is a top-bar button, while
the guidance text says the opposite of what the step offers:
`"Pick the bulletin shape and confirm your church name. Everything else starts from last week's file."`
Step 1 itself contains no way to open last week's file.

**Start (Step 1).** A shape list and a church-name box. Clicking a shape is
the most destructive action in the app and looks like the safest (see B2).
The church-name box only commits on focus loss.

**This Sunday (Step 2).** The date is shown, not editable (see B1). Sunday
name and scripture reading are text boxes that commit on `lost_focus()`,
with the reassuring note `"The cover and the order of service fill themselves from these."`

**Announcements (Step 3).** One multiline box, one announcement per line,
plus a "Save announcements" button — in an app whose promise is that saving
is automatic. The box silently restructures what she types (see F8) and the
autosave tick can fight the canvas over the same text (see B5).

**Check (Step 4).** Plain-word notes, mostly good:
`"OK: everything fits on the page."`,
`"Check: announcements are empty. If that is correct for this week, you can ignore this."`
But the notes point at steps she cannot jump to (see F1), and the fit check
measures with a different font set than the printer uses (see F9).

**Print (Step 5).** The most consequential step is rendered inside the
300 px left sidebar (see F4), with two competing print-copy buttons (F5)
and a printer wizard whose memory is keyed by a free-typed name (F6).
The good parts are real: the checklist language is excellent
(`"Size: pick Actual size, or 100%."`-style wording, learned flip named
exactly) and the post-Print next step exists: a strong
`"Start next Sunday"` button, as Principle 4 requires.

**Pages window.** A capable canvas — rulers in inches, drag, snap,
right-click exact entry, double-click typing — that the guided flow never
mentions (see F7). Its entire manual is one six-instruction sentence.

**Fonts window.** The plainest writing in the app
(`"These are the fonts this app may use."`), live previews in each face,
rights in plain words. But installing a font visibly changes nothing and
nothing explains why (see F10), and its feedback appears in another
window's corner (see F3).

**Recovery dialog.** Warm and clear:
`"OpenBulletin kept a copy of the bulletin you were editing, saved on {clock}. It was never saved to a file."`
with `"Bring it back"` / `"Throw it away"`. It never says which bulletin
the copy belongs to, and the destructive button is one unconfirmed click
(see F11).

## Findings

### BLOCKER

**B1. The bulletin date cannot be set or corrected anywhere in the app.**
Step 2 renders the date as a label, not a control:

```rust
ui.label(format!(
    "Date: {}",
    self.doc.fields.get("sunday_date_long").cloned().unwrap_or_default()
));
```

The only ways a date enters a document are `build_template(...,
chrono::Local::now().date_naive(), ...)` — today's date, whatever weekday
that is — and Next Sunday's +7 days. An admin who starts fresh on a
Wednesday gets a bulletin dated Wednesday and no in-app way to fix it,
while the guidance tells her to "Confirm the date and the name of the
Sunday." Confirming is all she can do. The Check step treats the date as
central (`"Check: no Sunday date is set (Step 2)."`) yet Step 2 offers no
way to set it.

**B2. Clicking a shape on Step 1 silently throws away the whole document.**
Preset rows are plain selectable labels whose click calls `rebuild()`:

```rust
if ui.selectable_label(self.preset == p, p.label()).clicked() {
    self.preset = p;
    self.rebuild();
}
```

and `rebuild()` replaces the document wholesale and then calls
`mark_clean()`, which sets the destroyed state as the "saved" state and
deletes the autosave journal:

```rust
fn mark_clean(&mut self) {
    self.saved_snapshot = snapshot(&self.doc);
    ...
    clear_journal_from(&obd, &meta);
}
```

No confirmation, no undo entry (`rebuild` never touches the undo stack),
no recovery copy. This also fires innocently: after opening a real file,
Step 1 still highlights the *previous* preset (Open never syncs
`self.preset` to the opened document), so returning to Step 1 and clicking
the shape that merely *looks* current rebuilds a blank template over her
file. Principle 6 — "Never lose work" — is violated by the first control
on the first screen.

**B3. Opening a file silently discards unsaved work *and* its autosave.**
The Open handler replaces the document and immediately marks it clean:

```rust
self.doc = doc;
self.path = Some(path);
self.step = 1;
self.status = "Bulletin opened.".into();
self.mark_clean();
```

`mark_clean()` clears the journal files, so the unsaved work the autosave
had been protecting is destroyed by the same click — no "you have unsaved
work" prompt exists anywhere in the app. Combined with B2, the two most
ordinary file actions are the two ways to lose a bulletin.

**B4. Undo history crosses document boundaries and can overwrite the wrong file.**
Nothing clears `undo_stack`/`redo_stack` on Open or on `rebuild()`. The
`undo()` implementation restores whatever snapshot is on the stack into
whatever document is current:

```rust
fn undo(&mut self) {
    if let Some(prev) = self.undo_stack.pop() {
        if let Ok(doc) = serde_json::from_str::<Document>(&prev) {
            ...
            self.doc = doc;
```

Sequence an admin will actually perform: edit bulletin A, open bulletin B,
press Ctrl+Z out of habit — the screen now shows A while `self.path`
points at B. Press "Save bulletin" and A is written over B's file. The
top-bar Undo button is enabled by the same stack, so no keyboard is
required to reach this.

**B5. The autosave tick rewrites the announcements story from a stale buffer and can erase canvas typing.**
Every two seconds, `journal_tick()` runs:

```rust
self.save_announcements();
let snap = snapshot(&self.doc);
```

`save_announcements()` rebuilds the weekly story's body paragraphs from
the `self.announcements` text buffer, discarding whatever is in the story.
But the canvas editor writes the other way: `commit_edit()` puts typed
text into the story and never updates `self.announcements`. So: double-
click the announcements box on the page, type this week's items, click
Done — and within two seconds the tick rewrites the story from the old
buffer. The user's newest work is deleted by the feature whose entire
purpose is never losing work. (The Print step calls
`self.save_announcements()` on every frame, making the same clobber
immediate there.) There are two sources of truth for the same text, and
the wrong one wins on a timer.

**B6. The always-visible main preview misrepresents the bulletin.**
The central panel is headed `"Preview: what the congregation will hold"`
and then renders each paragraph like this:

```rust
let text = text.replace('\t', "  ");
painter.text(
    ...
    text.chars().take(64).collect::<String>(),
    egui::FontId::proportional(fs * scale + 3.0),
    ...
);
...
if y > fr.max.y - 10.0 {
    break;
}
```

That is: every paragraph cut off at 64 characters with no ellipsis or any
other sign of cutting, no word wrapping (one straight line per paragraph
regardless of width), dot leaders replaced by two spaces, font sizes
approximated, and any paragraph that would pass the frame's bottom
silently dropped. An admin proofreading here — the natural place to
proofread, since it is always on screen — cannot see most of her own
announcements and gets no signal that she is not seeing them. The true
shaped rendering exists (`panel_lines`, used by the canvas and the Print
preview), which makes this a choice, not a limitation. Principle 6 again:
"print output that matches the preview" cannot hold while the default
preview is a sketch that hides text.

### FRICTION

**F1. The step list looks like navigation and is not; Check's directions point at steps the user cannot jump to.**
The sidebar renders steps as inert labels:

```rust
ui.label(egui::RichText::new(format!("{marker} {}. {name}", i + 1)).strong_if(i == self.step));
```

Navigation exists only as Back / `Next: {step}` buttons in the top bar.
Meanwhile Check notes give directions like
`"Check: the church name is blank (Step 1)."` — with no way to act on the
reference except pressing Back repeatedly. Users will click the list,
because every Office app trained them to; here nothing happens.

**F2. Standing church information the app prints cannot be edited in the app.**
`rebuild()` and `App::default()` hardcode the profile:

```rust
ChurchProfile {
    name: church_name.clone(),
    service_times: "Sundays 9:30 AM".into(),
    ..Default::default()
}
```

Only the church name has a field anywhere. Service times, website, giving
link, and pastor — all fields the README advertises
("Church name and address, service times, website, giving link, pastor…
these are fields") — have no editing surface in the UI. A church whose
service is at 10:30 gets "Sundays 9:30 AM" from every template, forever,
with no remedy short of editing files outside the app.

**F3. All feedback, including failures, is one small line of text in the wrong place.**
The only status channel is rendered once, at the bottom of the left
sidebar, in small type:

```rust
ui.small(&self.status);
```

Save and open failures land there with raw system error text:
`format!("Could not save: {e}")`, `format!("Could not open that file: {e}")`,
`format!("Font catalog: {e}")`. Actions taken inside the Fonts window
report to this same line, behind or beside the window the user is looking
at. A failed Save looks exactly like a successful one unless the user
happens to read small gray text in a corner — in an app whose sixth
principle is "never lose work," the *perception* of saving is part of the
feature.

**F4. The Print step — preview, checklist, and printer wizard — is squeezed into the 300 px sidebar.**
The steps panel is fixed:

```rust
egui::SidePanel::left("steps")
    .resizable(false)
    .default_width(300.0)
```

and the Print arm renders `sheet_preview_ui(ui)` inside it, where
`draw_sheet` gets `ui.available_width().min(560.0)` — roughly 268 px of
sheet. Text is drawn at `(line.size * scale).max(4.0)`, i.e. at the 4 px
floor: present, but unreadable. So at the exact moment the app says
`"This is exactly what comes out of the printer"`, it shows the sheet at
postage-stamp size in the narrowest column, while the wide central area
keeps displaying the B6 sketch of reader pages. The layout contradicts
the step's purpose.

**F5. Two print-copy buttons compete; neither is clearly *the* action.**
The Print step offers `"Open the print copy"` (plain button) and, a
separator later, a strong `"Save print copy (PDF)"`, plus a third,
`"Save reading copy (PDF)"`. Principle 4 promises "One primary action
per step." A first-time admin must guess which button her printer
workflow starts with; the one that opens the PDF in her viewer — the
actual start of printing — is the visually weaker one.

**F6. Printer memory is keyed by a free-typed name, and the two manual-print instructions disagree.**
The learned setting is stored under whatever string is in the box:

```rust
ui.label("Printer:");
ui.text_edit_singleline(&mut self.printer_name);
...
self.settings.printers.get(&self.printer_name)
```

Retype the name slightly differently and the checklist concludes
`"Two-sided: we have not learned the {printer_name} yet."` and the whole
test-sheet ritual restarts. There is no list of already-known printers to
pick from. Separately, the manual-feed guidance contradicts itself
between surfaces. The wizard's stored instruction says: "Print the front
sides, put the stack back in the tray printed-side up with the top edge
going in first, then print the back sides." The checklist for the same
mode says: "Print all the front sides, put the stack back in the tray
the same way it came out, then print the back sides." At the scariest
moment of the workflow, the app gives two different physical
instructions for the same act.

**F7. The Pages canvas is undiscoverable from the guided flow, and its manual is one breathless sentence.**
No step content, guidance string, or Check note ever mentions the Pages
window; entry is solely a top-bar `"Pages"` button among file actions.
An admin who follows the (well-built) guided path will never learn that
she can see pages, move boxes, or type on the page — the features the
README roadmap calls the future of editing are behind an unlabeled door.
Once inside, the whole manual is a single label:

> "Click a box to select it. Drag to move it, drag its corner square to
> size it, arrow keys nudge it. Double-click a box to type in it. Boxes
> and guides both snap to the blue lines. Right-click anything for its
> exact place in inches. Guides never print."

Each clause is plain; six instructions in one unpunk'd block fails the
ELI5 test the project sets itself, and none of it is repeated at the
moment of need (e.g., a hint when a box is first selected).

**F8. The Announcements box silently rewrites what the user typed.**
`weekly_body_text()` shows only paragraphs with `p.style == "body"`, and
`save_announcements()` stores text after:

```rust
.map(|l| l.trim().to_string())
.filter(|l| !l.is_empty())
```

Blank lines she used to group announcements vanish; leading/trailing
spaces are trimmed; anything the template held in a non-"body" style is
invisible in the box but still prints. The instruction says
`"One announcement per line. Paste from email works; formatting is cleaned up for you."`
— "cleaned up" is doing heavy lifting: the box, the stored story, and
the printed page can all show different spacing for the same text, with
no signal. (This is the visible half of B5's two-sources-of-truth
problem.)

**F9. The Check step measures fit with fonts the printer will not use.**
`App` holds one shaper, created once from bundled fonts:

```rust
shaper: ob_text::shaping::bundled_shaper(),
```

and `run_checks()` measures with `self.shaper.wrap(...)`. The print path
and the shaped previews build a fresh shaper and register every library
font into it (`build_preview`). So after an admin installs fonts and a
style names one, Check keeps measuring with the bundled set and can say
`"OK: everything fits on the page."` about text that overflows in print —
or warn about text that fits. A preflight that can disagree with the
printer undermines the one step whose job is trust.

**F10. Installing a font changes nothing, and the Fonts window never says why.**
The README is honest that "You cannot pick a font for a style inside the
app yet," but the Fonts window itself never tells the user this. She
searches, clicks "Get this font," sees
`format!("Font installed: {}", names.join(", "))` in the far status line,
and her bulletin looks exactly the same. Nothing in the window says a
font only takes effect when a style uses it, or where styles are chosen.
Expectation set by a successful install, silently unmet. Relatedly,
catalog rows print raw license codes to the user:
`format!("{} ({}) - {}", hit.family, hit.category, hit.license)` — the
license string is exactly the kind of jargon Principle 3 bans from the
main path, in a window whose other text is carefully plain.

**F11. The recovery dialog hides which file the work belongs to, and its destructive button is unguarded.**
`recovery` carries the source path — `bring_back` uses it to restore
`self.path` — but the window text tells the user only the time:

```rust
"OpenBulletin kept a copy of the bulletin you were editing, \
 saved on {clock}. It was never saved to a file."
```

An admin who edits two bulletins in a week cannot tell which one this
copy is before choosing. And `"Throw it away"` executes on a single click
with no confirmation, permanently deleting the only copy of unsaved
work — placed at equal visual weight beside the safe choice.

**F12. Escape *saves* a canvas typing session; there is no way to abandon typed changes.**
In the canvas, Escape is wired to commit:

```rust
if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
    commit = true;
}
```

Every Office habit says Escape means "get me out of this, unchanged."
Here it writes the text into the story (the only retreat is Undo, after
the fact, and only if she realizes). The editing bar offers
`"Done typing"` but no Cancel. A user who mangles a paragraph and hits
Escape to escape has instead published her mistake into the bulletin.

**F13. "Start next Sunday" marks an unsaved document as saved and leaves it pointing at last week's file.**
`next_sunday()` calls `mark_clean()` on the brand-new document — telling
the journal system the screen matches the last save and clearing the
autosave — while `self.path` still names last week's file. Two sharp
edges follow: close the app without touching anything and the new
Sunday's document is gone with no recovery offer (it was "clean"); or
press "Save bulletin" and last week's file is silently overwritten by
next week's, under the old name. The status text —
`"Started {} bulletin. Weekly announcements were cleared; standing items carried over."` —
mentions neither risk.

### POLISH

**P1. Step and pager affordances are cryptic glyphs.**
Completed steps are marked with an ASCII prefix (`"done -"`) in
`format!("{marker} {}. {name}", i + 1)`, and the Pages window pager is
two bare buttons labeled `"<"` and `">"` — no words, no tooltips.

**P2. A Check message makes a roadmap promise to the end user.**
The overflow note ends: `"Shorten the words, or we can shrink them in a later version."`
Release-note language inside a working dialog tells the admin her
problem is scheduled, not solved, and invites her to wait for a feature
instead of shortening the text — the opposite of the step's job.

**P3. No tooltips anywhere; MODES.md's promised keymaps and workspaces do not exist in the code.**
`docs/MODES.md` states as shipping fact: "**Keymap presets:**
OpenBulletin (default), Publisher, InDesign" and a Full workspace with
docked panels. The application contains no keymap system and no
workspace switch; the README, to its credit, says shortcuts are
"Not built yet." The internal spec overclaims what the user will find,
and the complete absence of tooltips also removes the one place the
vocabulary aliases ("Text Box" / "Text Frame") were supposed to live.

**P4. Fields commit on focus loss with no hint that they do.**
Church name, Sunday name, and reading all apply only via
`lost_focus()`. Nothing on screen says "press Tab or click away to
apply." Most users will be fine by accident; the one who types the
church name and immediately clicks a preset (B2) or closes the app meets
the edge without ever being told the rule.

**P5. Dead ends on the canvas give no feedback.**
Double-clicking a frame with no story (a shape, a picture area) does
nothing at all — `if ... .story_id.is_some() { self.enter_edit(...) }`
with no else. The frame context menu announces the internal frame name
(`format!("Box: {}", f.name)`), which the template, not the user,
chose. Small silences and small jargon, on the surface most likely to be
explored by clicking around.

**P6. The font catalog's internet requirement is invisible until it fails.**
The Fonts window invites the user to "Search the free font catalog" with
no note that this needs an internet connection; the church office PC
that is offline learns it only from `format!("Font catalog: {e}")` in
the status line (F3). The Starter Pack button has the same silent
dependency.

## Ordered fix list

1. **Guard every document replacement.** Preset clicks and Open must
   prompt when the current document differs from its last save; never let
   `mark_clean()` run as a side effect of discarding work. (B2, B3)
2. **Scope undo to the document.** Clear both stacks on Open and on any
   template rebuild, so history can never resurrect one bulletin inside
   another's file. (B4)
3. **Give Step 2 a real date control** ("Which Sunday?" with a date
   picker or typed date), and default new templates to the coming
   Sunday, not today. (B1)
4. **Make one source of truth for announcements text.** The journal tick
   should snapshot the document, never re-derive stories; sync the
   Announcements buffer whenever the story changes (canvas commit, undo,
   open). (B5, F8)
5. **Replace the central preview with the shaped rendering** already
   used by the canvas and Print (`panel_lines`), including visible marks
   wherever any preview must clip text. Retire the 64-character sketch.
   (B6)
6. **Move the Print step into the wide area** (or widen the panel for
   that step) so sheets render at readable size, and crown one primary
   print action. (F4, F5)
7. **Make the step list clickable** and let each Check note jump to the
   step it names. (F1)
8. **Promote feedback.** Save/open failures get dialogs in plain words;
   status text moves to normal size near the work; no raw `{e}` reaches
   the user. Fonts-window actions report inside the Fonts window. (F3)
9. **Add the rest of the church profile to the UI** — service times,
   website, giving link, pastor — and stop hardcoding "Sundays 9:30 AM".
   (F2)
10. **Reconcile the Check shaper with the print shaper** (register
    library fonts for checks, as `build_preview` does) so "fits" means
    fits in print. (F9)
11. **Point the flow at the Pages canvas** — a guidance line and a
    "See the pages" action from Check/Print — and break its one-sentence
    manual into short lines shown where each gesture is used. (F7)
12. **Fix the printer memory UX:** pick from remembered printers
    instead of retyping the key, and make the wizard and checklist use
    one identical manual-feed instruction. (F6)
13. **Recovery dialog:** name the file the rescued copy belongs to, and
    ask once before "Throw it away" destroys it. (F11)
14. **Escape cancels canvas typing** (discard the buffer edits), with
    "Done typing" as the only commit-by-keyboard path besides clicking
    away — or, if commit-on-Escape stays, say so on the editing bar.
    (F12)
15. **Next Sunday should set a new unsaved state honestly:** leave the
    document dirty (journal armed, status "not saved yet") and suggest
    the new file name instead of silently reusing last week's path.
    (F13)
16. **Close the small gaps:** say in the Fonts window that a style must
    use a font before it changes the bulletin and that search needs the
    internet; spell license names in plain words; label the Pages pager
    buttons ("Previous page" / "Next page"); drop the "in a later
    version" promise from the Check message; correct MODES.md or build
    what it claims. (F10, P1, P2, P3, P6)
