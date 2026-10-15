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
