# Closing audit — SECURITY lane (blind)

Date: 2026-10-15. Auditor: security lane, Muse subagent, blind to
the other lanes. Scope: threat model + audit of the OpenBulletin
tree at HEAD (`4014b3d`), against the posture stated in
`docs/DEPLOYMENT.md` and `docs/LEGAL.md`. No code was modified.

Verified baseline in this session: `cargo test --workspace` =
**53 passed, 0 failed** (per-crate: ob-extract 2, ob-fonts 8,
ob-format 1, ob-impose 3, ob-layout 4, ob-model 5, ob-pdf 8,
ob-text 10, app 12, helper 0, CLI 0, doc-tests 0).

## Threat model

Assets: the admin's machine and files; the bulletin documents
(church membership-adjacent content: prayer lists, announcements);
the Font Library folder; the integrity of printed output.

Actors, in rough order of realism for a church office:

1. **A hostile or sloppy file handed to the admin** — a font pack
   zip from a font site, a "free font" download, a PDF of an old
   bulletin, a shared `.obd`. This is the app's designed input
   surface ("Add fonts", zip packs, PDF trace rescue, Open).
2. **A compromised or spoofed network endpoint / proxy** on the
   one network path the app has (the font catalog).
3. **Another local account or process** on a shared church PC.
4. **A network attacker** — mostly out of scope today: the app
   opens no listener and makes no inbound surface at all.

Trust boundaries: file bytes → parsers (serde_json, ttf-parser,
zip, lopdf scaffold, PDFium, `image`); remote bytes → Font
Library writes; document fields → filesystem paths and generated
PDF; app → child processes (PDFium helper, OS file opener).

## Findings

### BLOCKER

None.

### MAJOR

**M1. Font ingest writes files under attacker-chosen names with
no path sanitization — traversal out of the Font Library.**

`FontLibrary::add_bytes` writes to `self.dir.join(suggested_name)`
with no check that the name is a plain file name
(`crates/ob-fonts/src/lib.rs:192,199`). `Path::join` honors `..`
segments and absolute paths, so the destination escapes the
library folder. The name is attacker-controlled on two paths:

- **Catalog install:** the file name comes from the remote
  google/fonts `METADATA.pb` (`filename:` lines) and is passed
  straight to `add_bytes` (`crates/ob-fonts/src/catalog.rs:253,256`).
  A hostile catalog response, a compromised upstream, or a MITM
  under a trusted proxy CA (see m4) can name a file
  `../../anything.ttf`; the content must parse as a font, but the
  write lands outside the library.
- **Zip ingest:** the entry name is reduced with
  `name.rsplit('/').next()` (`crates/ob-fonts/src/lib.rs:233`),
  a hand-rolled sanitizer that only strips `/`. On Windows — the
  primary user platform — `\` is the separator, so an entry named
  `..\..\evil.ttf` survives intact and traverses. Worse, the
  license-text branch writes the entry's **raw, unvalidated
  content** to `self.dir.join(&file_name)`
  (`crates/ob-fonts/src/lib.rs:243`): an entry named
  `..\..\ofl.txt` (ends `.txt`, contains "ofl") writes arbitrary
  attacker text anywhere the user can write. The zip crate's safe
  API (`enclosed_name()`) exists and is not used anywhere in the
  tree (`grep enclosed_name` = 0 hits in `crates/`).
- Same family: `write_license_note` builds its file name from the
  catalog `repo_id`, itself derived from server-supplied ids
  (`crates/ob-fonts/src/catalog.rs:281`).

Exploit shape: the admin does exactly what the app invites —
downloads a font pack zip from a font site, clicks "Add fonts" —
and files appear outside the Font Library. Content limits (font
bytes must parse; text branch is `.txt`/`.md` names) keep this
short of code execution, but it is an arbitrary-file-write
primitive on the app's flagship ingest path. Fix direction
(not applied): reject any suggested name that is not a single
path component on both separators, use `enclosed_name()` for
zip entries, and apply the same rule to license notes.

**M2. No size limits anywhere on ingest or download — zip bombs
and unbounded reads exhaust memory/disk.**

- Zip entries are decompressed with `read_to_end` into an
  unbounded `Vec` (`crates/ob-fonts/src/lib.rs:235`); there is no
  per-entry cap, no total cap, and no entry-count cap.
  `ZipArchive` reports each entry's declared size; it is never
  consulted. A classic zip bomb (the exact file type the Fonts
  panel invites from arbitrary websites) grows RAM until the
  process dies, and each accepted font is then also written to
  disk by `add_bytes`.
- The whole zip is first read into memory by the app
  (`std::fs::read`, `crates/ob-app/src/main.rs` ~2259) and copied
  again inside `add_zip_bytes` (`bytes.to_vec()`,
  `crates/ob-fonts/src/lib.rs:224`), tripling peak use.
- Catalog downloads are likewise unbounded: `get_bytes` reads
  until EOF with no cap (`crates/ob-fonts/src/catalog.rs:118`),
  and `get_json`/`get_string` rely on ureq's unbounded body
  reads. A broken or hostile endpoint can stream forever into
  memory (a real font is well under a few MB; nothing enforces
  that scale).

### MINOR

**m1. A document can point its tracing picture outside its own
folder.** `load_bg_texture` does `base.join(rel)` where `rel` is
the `.obd`'s `background_image` string
(`crates/ob-app/src/main.rs:2137`). An absolute path replaces the
base entirely; `../` escapes it. The bytes are decoded with the
`image` crate and displayed as the panel underlay — display-only
(no data leaves the machine), but a crafted `.obd` received by
email can surface an arbitrary image from the victim's disk on
screen, and the decode has no explicit dimension/allocation
limits of our own. The design assumes the picture "lives beside
the bulletin file"; nothing enforces that.

**m2. Predictable temp file for the print copy.**
"Open the print copy" writes the full imposed bulletin to
`std::env::temp_dir()/openbulletin-print-copy.pdf`
(`crates/ob-app/src/main.rs` ~2860) with `std::fs::write`, which
follows symlinks. On a shared machine (Linux `/tmp`), a local
attacker who pre-plants a symlink at that fixed name redirects
the write; at minimum the file (complete bulletin content) is
left behind in a shared temp dir after viewing. Low impact,
standard fix (unique name, create-new semantics).

**m3. Helper resolution trusts environment and PATH; no run
timeout.** `resolve_helper` falls back to a bare
`ob-pdfium-helper` PATH lookup after the `$OB_PDFIUM_HELPER`
override and the exe-sibling check (`crates/ob-cli/src/main.rs:449-463`).
Anyone who can write to a PATH directory or set the env var
chooses the binary the CLI runs. The invocation itself is clean
(argument array, no shell), and the app never invokes the helper
(verified: `Command::new` appears in the app only in
`open_with_os`), so this is a CLI/dev-surface note. Related: the
CLI waits on the helper's `.output()` with no timeout, and the
helper renders bitmaps sized from the PDF's own page geometry
(`crates/ob-pdfium-helper/src/main.rs:160`) — a hostile PDF can
hang or OOM the helper. The out-of-process design contains that
to the helper, as `docs/DEPLOYMENT.md` claims; the CLI's wait is
the only unbounded part.

**m4. Font trust rests on TLS plus the system store, by design,
and is not recorded as a risk decision.** The catalog client
deliberately trusts the OS certificate store (including workplace
proxy CAs) merged over web roots (`crates/ob-fonts/src/catalog.rs:41-58`),
and downloaded fonts are accepted on TLS + a successful
ttf-parser inspection + the license gate — no checksum or
signature from the catalog, so a first install pins whatever
bytes arrived (SHA-256 identity is computed after the fact in
`inspect`, `crates/ob-fonts/src/lib.rs:104-125`). For fonts from
google/fonts over HTTPS this is a reasonable posture, but
combined with M1 it means a network attacker who passes the CA
check also gets a file-name primitive. Worth one recorded line
in `docs/LEGAL.md` or `docs/FONTS.md` that this is accepted.

**m5. `docs/LEGAL.md` dependency numbers have drifted.** The doc
reports a 2026-10-15 audit of **475 packages** and a single
multi-license flag. Recount in this session (`cargo metadata`):
**523 packages**. The LGPL-bearing expression
(`MIT OR Apache-2.0 OR LGPL-2.1-or-later`) now appears on two
entries because r-efi is present at both 5.3.0 and 6.0.0 — same
crate, same OR-choice, still satisfied by MIT/Apache-2.0. One
package the doc does not mention: `epaint_default_fonts 0.29.1`
(egui's embedded fonts) is licensed
`(MIT OR Apache-2.0) AND OFL-1.1 AND LicenseRef-UFL-1.0` — the
app binary therefore redistributes OFL/UFL font data via egui,
which the project's own font-attribution practice (license texts
saved beside fonts) would normally record. The substantive claim
holds (see claim table); the doc's own rule ("re-run whenever
dependencies change materially") has silently lapsed.

**m6. Autosave journal and settings are plaintext in the user
config dir.** The journal writes the complete current bulletin
next to `settings.json` (`write_journal_to`,
`crates/ob-app/src/main.rs:92-108`). Local-first makes this
inherent, and per-user config dirs are the right place; on a
shared church-office PC, any local account that can read the
profile can read the in-progress bulletin, including anything
typed into it. Observation, not a defect: worth one line in
`docs/DEPLOYMENT.md` so the shared-PC case is a known property
rather than a surprise.

**m7. `.obd` load accepts unbounded structure.** `ob_format::load`
(`crates/ob-format/src/lib.rs:20-30`) parses the whole file with
serde_json into the typed model with a newer-version check —
good — but there are no sanity limits on panel/story/paragraph
counts or on the `i64` millipoint geometry the layout, imposition,
and PDF code then compute with. Counts are bounded in practice by
file size, and the model is shallow (no recursive structure, and
serde_json's own nesting limit applies), so the realistic worst
case from a crafted file is a panic or a long hang on a single
user's machine. Recorded as informational; a cheap `validate()`
pass (positive sizes, count ceilings) would close it.

**m8. `open_with_os` goes through `cmd /c start` on Windows**
(`crates/ob-app/src/main.rs:3351-3366`). Today its only caller
passes a fixed, app-generated temp path (m2), so no user input
reaches the command line. Flagging because the helper looks
general-purpose: if it is ever reused with a user-named path,
`cmd` metacharacter handling becomes an injection question.
Informational.

## What checked out (verified, not assumed)

**Network surface is exactly as claimed.** The only network code
in the tree is the ob-fonts catalog client: `grep -rn "ureq"`
hits only `crates/ob-fonts/src/catalog.rs`; the only URLs in code
are `https://api.fontsource.org/v1/fonts` and
`https://raw.githubusercontent.com/google/fonts/main/...`
(`crates/ob-fonts/src/catalog.rs:11-12,192,253,273`). No
`TcpStream`, no other HTTP client, and **no listener anywhere**
(`grep "bind(\|listen"` in `crates/` = 0): the NAS/Docker and
Cloudflare shapes are genuinely unbuilt, as the docs say. Every
catalog call is user-gesture-triggered (app: search/install/pack
buttons at `crates/ob-app/src/main.rs:2338,2372,2405`; CLI:
`fonts search|get|pack` subcommands). No telemetry: greps for
analytics/sentry/telemetry/phone-home return nothing; startup
does no network I/O (the Fonts window seeds bundled Liberation
faces locally on first open).

**Process execution is disciplined.** All `Command::new` uses
pass argument arrays — helper invocation
(`crates/ob-cli/src/main.rs:334`), PDF opener
(`crates/ob-app/src/main.rs:3351`). No shell string is ever built
from a file name or document field. Filenames reach child
processes only as single argv entries.

**Parsers fail closed.** `.obd` load, settings load
(`crates/ob-app/src/main.rs:296-300`), journal read
(`crates/ob-app/src/main.rs:112-127`), and the undo stack's
re-parse (`crates/ob-app/src/main.rs:177,189`) all use
`Result`/`Option` chains — no `unwrap`/`expect` on untrusted
content anywhere in the load paths (the one `unwrap` found,
`crates/ob-app/src/main.rs:3657`, is inside a test). PDF output
escapes literal strings correctly (`esc()`,
`crates/ob-pdf/src/lib.rs:82-99`), so document text cannot inject
PDF operators into generated files. There is **no `unsafe` block
in any first-party crate** (`grep unsafe crates/` = 0 outside
comments).

**PDFium isolation is real.** The helper is a separate process
(`crates/ob-pdfium-helper`), the PDFium binary is not tracked in
git (`git ls-files | grep -ci pdfium` = 2, both crate source
files), and the desktop app contains no PDF parsing at all. The
in-process lopdf extractor (`ob_extract::extract_pdf`) is used
only by examples and tests (`crates/ob-extract/examples/*`,
lib tests) — dev scaffold, not a shipped path, matching
`docs/EXTRACT.md`.

**Secrets and personal data: clean, tree and history.** Sweeps
over the working tree and **all git revisions** for the owner's
home address, phone, family email, API-key and token shapes
(`sk-`, `ghp_`, `github_pat`, `AKIA`, private-key headers,
`api_key=` assignments) returned **zero hits**. `.gitignore`
blocks `*.pdf` and `*.obd`, and `git ls-files` shows **0 tracked
PDFs and 0 tracked .obd files**; the only tracked binaries are
the 8 OFL Liberation faces. The only church in code/tests is the
fictional Grace Lutheran Church. Docs about the real research
corpus carry aggregates only (page counts, date ranges, font
names, "Lutheran/ELW") — no congregation name, no location, no
person's name appears anywhere in the tree
(`grep Johnson|Covington` over all tracked text = 0 hits).
GitHub workflows reference **no secrets at all**, use no
`pull_request_target`, and the self-hosted lane
(`.github/workflows/build-desktop.yml`) triggers only on pushes
to main and manual dispatch, so untrusted PR code never runs on
the owner's desktop runner.

**License substance verified.** `cargo metadata` over the
current tree: 523 packages; every license expression is built
from MIT, Apache-2.0, BSD-2/3-Clause, ISC, Zlib, Unicode-3.0,
BSL-1.0, CDLA-Permissive-2.0, CC0-1.0, Unlicense, 0BSD, OFL-1.1
(the egui font package, m5), and the r-efi OR-expression whose
LGPL arm is never selected. **No GPL, AGPL, SSPL, or
LGPL-as-requirement anywhere** — the exclusions `docs/LEGAL.md`
claims (MuPDF, Poppler) are absent from the tree. Project
license files (LICENSE-MIT, LICENSE-APACHE) are present and the
workspace declares `MIT OR Apache-2.0`.

## Verdict

**The security posture claims are true in substance.** Local-first
holds: no telemetry, no uploads, no listener, no network code
outside the font catalog, and that catalog only speaks HTTPS to
two named hosts when the user asks it to. Secrets hygiene is
clean across the full public history, and no real church data is
tracked. The PDFium boundary works as described.

The gaps are concentrated exactly where untrusted bytes enter by
design — **font ingest**: unsanitized file names on library
writes (M1) and the complete absence of size limits (M2). Neither
contradicts a written claim, but both sit on the workflow the app
most encourages (installing font packs from the internet), and
both are cheap to close. Secondary items are documentation
drift (`docs/LEGAL.md` counts, m5) and shared-machine temp/journal
exposure (m2, m6). Recommendation: fix M1 and M2 before any
public release push; refresh `docs/LEGAL.md` numbers in the same
pass; the rest can ride the normal backlog.
