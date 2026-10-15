# Deployment shapes (planning)

Jacob, 2026-10-15: run it on the computer itself, or as a Docker image
on a NAS over the local web, easy for others to get into. Login later,
planned-for but not built now.

## The rule that makes this cheap
The engine is headless and never knows which front door served it:
desktop app, CLI, or web. Every shape below runs the same ob-* crates
and the same .obd files. Simple on the surface (ELI5 words, five
steps), powerful underneath (real shaping, imposition, imports).

## Shapes
1. **Desktop app (v0.1, shipping shape).** Native on macOS, Windows,
   Linux. Files live on the user's machine. No account, no network.
2. **Docker image for a NAS / home server (planned, v1.x).** One
   container a church office (or a family) runs on their own network;
   people open it in a browser on the local web. The egui UI compiles
   to WebAssembly (eframe supports wasm), so it is the same interface,
   not a second product. A tiny server process serves the app and the
   .obd files from a mounted folder. `docker compose up` is the whole
   install story.
3. **Hosted (someday, not committed).** Only if churches ask for it.
   Nothing in the format or engine may assume it.

## Accounts and roles (planned-for, not built)
- Not in v1. The desktop app has no login and needs none.
- Design doors kept open from now on: documents carry no owner
  assumptions; the server shape is where accounts live, never the file
  format.
- When the NAS shape gets users, add **local accounts first** (created
  on the box itself), with **Google sign-in as an optional second door**
  for churches that live in Google Workspace.
- Roles, in plain words: **Owner** (can delete, manage people),
  **Editor** (can change the bulletin), **Viewer** (can look and print,
  not change). That is enough for a church office: pastor and admin
  edit, everyone else views.
- Multiple people editing the same bulletin at the same moment is
  still out of scope (see NEVER.md); roles are about who may open and
  change files, one editor at a time, with the file as the truth.
