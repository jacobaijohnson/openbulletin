# Modes: one interface, familiar keys

Three full UI personalities (Publisher mode / InDesign mode / Native mode)
were evaluated and rejected: they triple the testing, documentation, and
tutorial surface, and they break phone-help, where the person helping and
the person at the keyboard must see the same menus.

What ships instead:

- **One canonical UI.** Five guided steps (Start, This Sunday,
  Announcements, Check, Print) carry the weekly work, and three
  windows (Pages, Fonts, Shortcuts) hold the tools. There is no
  Simple/Full workspace switch: the surface stays simple by
  construction, and the power lives one click away in the windows.
- **Keymap presets:** OpenBulletin (default), Publisher, InDesign.
  Customizable per command (see below).

Ideas evaluated and NOT built, recorded so nobody goes looking
for them: a Simple/Full workspace split (the one UI above made it
unnecessary), and vocabulary aliases ("Text Box" / "Text Frame")
in search and tooltips (there is no search and no tooltip system;
every command uses one canonical name everywhere it appears).
Either could return if a real need shows up; neither exists today.

Invariants, never negotiable: the file format and its contents, the menu
tree and labels, command behavior and undo semantics, rendering, and
export output are identical in every mode. A document never records which
keymap created it.

## Keymap, as shipped (2026-10-15)

Jacob's rule: presets only SET the defaults. The Keys window (top
bar) offers three starting sets: OpenBulletin, Publisher, and
InDesign. Choosing one fills the table; from there every key can
be changed by picking the action and pressing the new key. The
moment the effective keys differ from a set's defaults, the set
is named Custom; changing a key back restores the set's name
(the name is derived from the keys, never stored as a claim).
If a pressed key already belongs to another action, the two
actions swap keys: no key does two jobs, and none is lost.
Bindable actions: undo, redo, save, open, new, Pages and Fonts
window toggles, next/previous step, copy/paste box, next/previous
page. Escape cancels a capture. Bindings live in settings.json
(keymap_preset + per-action overrides), never in the .obd file.
