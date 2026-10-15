# Modes: one interface, familiar keys

Three full UI personalities (Publisher mode / InDesign mode / Native mode)
were evaluated and rejected: they triple the testing, documentation, and
tutorial surface, and they break phone-help, where the person helping and
the person at the keyboard must see the same menus.

What ships instead:

- **One canonical UI.** Simple workspace by default (template gallery,
  properties, fields); a Full workspace adds docked panels for pages,
  styles, and preflight.
- **Keymap presets:** OpenBulletin (default), Publisher, InDesign.
  Customizable per command.
- **Vocabulary aliases in search and tooltips only.** "Text Box"
  (Publisher) and "Text Frame" (InDesign) both find the same command;
  menus always use the canonical name.

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
