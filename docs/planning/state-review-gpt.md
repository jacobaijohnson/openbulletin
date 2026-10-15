(A) **Decisions to revisit**  
These are structural choices or ordering calls that are going to hurt you if you keep them as-is.

1. **PDF output that doesn’t use the actual layout fonts**  
   - Today: PDF draws with “core Helvetica” while the engine measures and wraps with Liberation (or other library fonts later).  
   - Problem: you’re explicitly violating Principle 2 (“powerful underneath”) and the Check step promise. The whole point of real shaping is that what you measure is what you print. “Almost the same metrics” is not good enough when you’re warning on overflow and fold hits.  
   - Recommendation: treat “PDFs embed the same font bytes the engine shaped with” as v1-critical, not “later.” If necessary, delay advanced imports and the template analyzer to get embedding done and tested.

2. **File format & packaging pushed too far down the roadmap**  
   - Today: `.obd` v0 = JSON, no packaged fonts or assets; fonts and images live outside the file, with only font hashes recorded. Packaging is listed as “not built yet.”  
   - Problem: for “one real church,” you already have multi-machine stories: secretary on office PC, pastor wanting to open on laptop, future NAS. Principle 6 (“Never lose work”) is not compatible with “you must remember to copy a bunch of fonts and images along with the .obd.”  
   - Recommendation: bring `.obd v1` “zip packaging with fonts and assets” forward into the v1 line, not after. Even if you only support bundling free / installable fonts and local images, you need a “single file is the document” story before you pilot real users.

3. **Inline page editing vs panel-only editing**  
   - Today (README): “Typing words directly on the page” is not built; editing happens in step panels, not on the page picture. Inline editing is in v0.1 roadmap but not implemented.  
   - Problem: this is a behavioral mismatch with what Publisher refugees expect. “You edit pages, the app handles the sheet” is your own tagline; if users can’t click on the page and type, they’ll feel like they’re filling a form, not doing layout. That’s friction every single week.  
   - Recommendation: inline text editing in frames (with cursor, selection, undo) is v1-necessary. Don’t ship a “real church” v1 without it. You can still keep the guided Start/This Sunday/Announcements flow, but frames must be directly editable.

4. **Roadmap emphasis vs actual corpus**  
   - Today: v0.1 acceptance is a letter bi-fold. But your 17-bulletin corpus is 16-page half-letter saddle-stitch booklets with picture covers and commercial fonts.  
   - Problem: you’re optimizing the early milestones around a form (letter bi-fold) that your measured real church doesn’t use, while the harder/imposition-heavy saddle booklet shape is deferred. That’s fine for engine prototyping, but not for “one real church v1.”  
   - Recommendation: explicitly add “half-letter 16-page saddle booklet from letter/tabloid stock” as a first-class preset in the v1 target. The imposition math is largely there; you need presets, wizard text, and some real-world print testing. Don’t hide this as “yeah, saddle-stitch works too” in README; make it the primary runway for Lutheran-style 16pp bulletins.

5. **Imports: flagship PDF analyzer vs minimum viable rescue**  
   - Today: PDF analysis → template + corpus analyzer are framed as flagship, with complex clustering and consensus. Publisher/IDML imports are also in the same track. None actually produce a draft bulletin yet.  
   - Problem: for one real church, you do not need the full analyzer to be useful. You do need *some* way to avoid complete retyping. The current plan risks you over-building the analyzer before you have a simple “drop in last week’s PDF as a locked trace and rebuild over it” workflow.  
   - Recommendation: invert the priority:  
     - First: PDF place-as-graphic + locked trace layer, plus a manual template over it.  
     - Second: simple per-PDF rescue (layout + text, no corpus logic).  
     - Third: corpus-based Template Analyzer.  
   - That gives a secretary something they can actually use earlier, without staking v1 on the hardest import problem.

6. **Acceptance definition for v1**  
   - Roadmap says v1.0 = “Credible for Small Organizations” with newsletters, large-print, hyphenation, `.pub` rescue, tagged PDF, etc. That’s a lot.  
   - For “one real church,” that bar is too high.  
   - Recommendation: define a narrower **“Church v1” acceptance** separate from “Small Organizations v1,” something like:  
     - A secretary can: create, edit, and print weekly bulletins (letter bi-fold or half-letter saddle-booklet), carry styles and recurring blocks, rescue at least one prior Publisher/PDF bulletin into a usable template, and move between office PC and pastor laptop with zero font/image surprises.  
   - Treat that as the decisive line; newsletters and large-print can follow.

(B) **Quality risks in what is built**  
These are things that exist but are fragile or misleading in their current form.

1. **Font metric mismatch in PDFs**  
   - Engine shapes text with Liberation; PDF uses Helvetica. You already warn users that lines might come out “a tiny bit wider or narrower.”  
   - Real risk:  
     - Your Check step can say “OK, fits” and the printed PDF overflows or hits a fold.  
     - Lines with dot leaders, hymn lists, and right-aligned page numbers are exactly where small metric differences become visible.  
     - Print shops wanting embedded fonts will treat your PDF as incomplete.  
   - This will be the first visible paper-cut in real use; fix by embedding the actual library fonts and using them for draw and measure.

2. **Font identity: SHA pinning without packaging**  
   - You correctly pin styles to the SHA-256 of the font used at layout time. Good.  
   - But: without packaging, you have no guarantee that SHA is actually present on the receiving machine. That leads to:  
     - “Missing font” situations that appear only when someone opens the file on another machine or in future NAS.  
     - Silent fallback if you ever decide to auto-substitute (and you really shouldn’t).  
   - There is a quality risk that the fidelity guarantee you describe in FONTS.md (“the same bulletin lays out identically on every machine”) simply isn’t true until packaging exists.  

3. **JSON `.obd` and atomic save vs “Never lose work”**  
   - Model: atomic save is good; you avoid half-written files. But:  
     - No autosave or recovery file is implemented yet, while the roadmap promises it.  
     - Multi-step guided flow means users may have filled in lots of fields without hitting Save. A crash or power cut still loses that work.  
   - This is a principle violation and a quality risk: the current state is closer to “we rarely corrupt files” than “we never lose work.”  
   - You need autosave + crash-recovery journals before calling the engine “safe” for production church offices.

4. **Duplex wizard persistence vs OS-specific printing behaviour**  
   - You’ve built a duplex wizard and persist flip settings per printer. Good direction.  
   - Risk:  
     - Printer drivers and OS updates sometimes change names, capabilities, or flip semantics.  
     - On Windows especially, the same physical printer can show up as multiple logical printers.  
   - If persistence is keyed too naively (e.g., string name only) you’ll end up with “why did my test sheet work last week but not now?” bugs.  
   - Make sure you test the wizard across Windows/macOS/Linux, and consider robust IDs (device GUIDs, not just names).

5. **PDF extraction: lopdf vs PDFium inconsistencies**  
   - You already note lopdf ordering is not deterministic; PDFium extraction is deterministic.  
   - Risk:  
     - Anything upstream of the analyzer that depends on lopdf token order will misbehave for some inputs.  
     - You could end up with apparently random template suggestions across machines or runs.  
   - Given you already have PDFium and treat it as the authoritative extractor, consider treating lopdf as an internal scaffolding only, not as a user-facing or analyzer-facing source of truth.

6. **Network/font catalog robustness**  
   - The font catalog uses Fontsource for search and google/fonts for downloads, handles proxies via env, and trusts OS cert store. That’s solid.  
   - Risk:  
     - Church networks can be sporadically offline or heavily filtered. A “Find a font” operation that hangs or fails without clear, plain-language messaging will frustrate non-technical admins.  
   - You already run fetches on a worker thread; add explicit timeout and offline messaging (“Couldn’t reach the font catalog; check your internet or proxy settings”) and test on a typical locked-down church LAN.

7. **UX promise vs current feature set**  
   - README promises: “You edit pages. The app handles the sheet.” In reality, today:  
     - No inline editing, no font selection, only Liberation Sans, and PDFs don’t use the same fonts.  
   - There’s a mismatch between what the public README sells and what 0.1 actually delivers.  
   - For developers and pilots that’s tolerable; for a first real secretary it will undermine trust. Ensure the README stays brutally aligned with reality during early pilots.

(C) **What is missing for a complete v1 for one real church**  
Focusing strictly on what a real secretary needs to reliably produce weekly bulletins with minimal drama, staying within your scope.

I’ll group by theme and mark items v1-essential vs nice-to-have for later.

1. **Editing & layout (v1-essential)**  
   - **Inline text editing in frames** with:  
     - Cursor movement, selection, basic editing (type, backspace, enter).  
     - Undo/redo across text edits.  
     - Clear visual indication of overflow (e.g., red triangle in frame).  
   - **Threaded frames** for common bulletin patterns (service order spanning panels, announcement lists).  
   - **Dot leaders/tab stops** in live text frames, not just CLI spikes.  
   - **Basic box and frame tools**: create, move, resize text frames within zones/panels, within the guarded template structure you define.  
   - **Key repeat, arrow movement, and possibly Publisher-like keymap** (as planned), at least for text navigation.

2. **Fonts (v1-essential)**  
   - **Font selection per style**: choose family and face (regular/bold/italic) from the Font Library.  
   - **Italics**: at least for body text and scripture quotes. Right now “no italic” is a hard UX-limiter; many churches use italics in consistent ways.  
   - **Font embedding in PDFs**:  
     - Use the exact library font bytes used for shaping.  
     - Honor fsType and license rules as designed (installable/editable/OFL/Apache → embed; preview-and-print/restricted → no embed and clear warning).  
   - **Font packs**: at least one “church starter pack” curated from OFL/Apache fonts (a humanist sans, a classic serif, a display script, a slab) installed in one tap.  
   - **Clear missing-font handling**: if a document references a font SHA not in the library, show a plain-language “You don’t have this font; here’s how to add it” instead of falling back silently.

3. **Document packaging & transport (v1-essential)**  
   - **`.obd v1` as a zip package** containing:  
     - The JSON model.  
     - Embedded fonts that are allowed to travel.  
     - Embedded image assets (or a clear, robust asset reference scheme if you keep them outside).  
   - **Open/save semantics**: opening on another machine must either:  
     - Work identically (all fonts/assets present in package and load), or  
     - Fail gracefully with explicit instructions, not quiet substitution.  
   - This is key for a secretary who occasionally hands a file to the pastor or a tech volunteer.

4. **Bulletin shapes and presets (v1-essential)**  
   - **Letter bi-fold** (already there) tested end-to-end with duplex wizard.  
   - **Half-letter saddle booklet** matching your corpus:  
     - Preset for 12–16 pages, folded and optionally stapled.  
     - Imposition tested so page numbers and folds are correct.  
   - **Basic cover handling**:  
     - Support a “picture-only cover” with one large image zone and minimal text.  
     - Ability to drop in a new cover image weekly with consistent frame behavior.

5. **Guided flow & checks (v1-essential)**  
   - Current 5-step app is good; for v1 you need:  
     - **Check step** that covers:  
       - Required fields (date, Sunday name, church name/address).  
       - Text-overflow checks using true font metrics and actual PDF layout fonts.  
       - Simple fold-position check for key content (warnings for headings landing on folds).  
     - **Persistent duplex wizard** that works across OSes and survives restarts.  
   - Plus: ensure that “exactly one visible next step” holds even when something fails (e.g., font missing, image missing) with clear guidance.

6. **Imports / rescue (minimum viable, v1-essential)**  
   You don’t need the full analyzer; you do need one path that isn’t “retype everything.”

   - **PDF place-as-graphic/trace layer**:  
     - User drops last week’s PDF; app places it as a locked background.  
     - User creates frames on top and types or pastes text into them.  
   - **Single-PDF template rescue** (non-corpus):  
     - Import: extract page sizes, text blocks, and images from one PDF.  
     - Draft: create an OB template with zones/frames roughly aligned; user reviews in Accept/Edit/Cancel.  
   - `.pub` and IDML rescue are nice but not strictly v1-essential for “one real church,” especially if they are willing to export PDFs from Publisher/InDesign as a stepping stone.

7. **Reliability & safety (v1-essential)**  
   - **Autosave**:  
     - Periodic autosave to a journal file while editing.  
     - On crash/restart, detect a journal and offer recovery (“We found unsaved work from [time]; restore?”).  
   - **Settings and printer config robustness**:  
     - Ensure settings persistence handles upgrades and multiple versions of the app cleanly.  
   - **Error messaging**:  
     - All fatal and non-fatal errors expressed in plain language, not Rust or technical jargon (“We couldn’t reach the font catalog” vs “TLS handshake failed”).

8. **Deployment & installers (v1-essential for one real church)**  
   - **Installers**:  
     - Signed Windows installer (.msi or .exe).  
     - Notarized macOS app bundle.  
     - Linux AppImage or Flatpak.  
   - Real churches will not build from source. v1 has to be “download, install, run” with no Rust knowledge.

9. **Nice-to-have but not strictly required for first v1**  
   - Hyphenation and H&J controls (nice for typography; can be 1.x later).  
   - Newsletters, facing pages, jump lines.  
   - Large-print companion exports.  
   - Tagged PDF and accessibility metadata.  
   - Full `.pub` rescue import beyond PDF-based workflows.  
   - Template packs beyond a small starter set.  
   - Docker/NAS deployment and account roles (these can be “phase 2” once a few desktops are happy).

(D) **The font decision: copy-in vs live system fonts**  

Your current leaning:  
> Font packs YES, system fonts only by COPYING them into the library through a picker, never by live reference.

You asked: is copy-in right, or is there a better practice?

**Short ruling: your leaning is correct. Copy-in to an app-managed Font Library is the right practice for OpenBulletin’s goals. Do not use live system fonts for layout, except as a source to copy from.**

Here’s the breakdown, grounded in industry and web practice:

1. **Reproducibility is your north star**  
   - Print-world practice (InDesign, Quark, etc.):  
     - They *do* use system fonts while editing, but for reliable output they push you to **Package** the document with a copy of every font used. That package is what travels to another machine or the print shop.  
   - Web-world practice for self-hosted fonts:  
     - Serious sites don’t rely on client system fonts; they **self-host font files** (WOFF/TTF) so every viewer sees the same metrics.  
   - NAS/server scenario:  
     - A Docker container has no meaningful “system fonts” in the way a desktop does. Relying on system fonts breaks immediately in your own planned deployment shape.  

   Given your stated requirement that a bulletin “lays out identically on the office PC, the pastor’s laptop, and future NAS,” **you must control the font bytes**. That aligns with the packaging concept: fonts are part of the document’s environment, not a random OS dependency.

2. **Copy-in from system fonts is the right compromise**  
   - Church admins “already have” fonts like Calibri, Arial, Times, etc. on their PCs; you want to use them legally and conveniently.  
   - Copy-in via picker:  
     - Reads the font file from the system’s font directory.  
     - Makes a copy into the Font Library.  
     - Leaves the original untouched.  
   - This is exactly analogous to “Package” in DTP tools, except you’re doing it at font-add time rather than at export time. It gives you:  
     - Deterministic layout across machines and deployments.  
     - Clear embedding and license handling via fsType and known licenses.  
     - The ability to include fonts in `.obd` packages where allowed.  

   The extra step (“Pick Calibri from your system, we’ll copy it”) is worth it because it enforces a clean mental model: **“OpenBulletin uses fonts in its Library. System fonts are just a source for that Library.”** That is far easier to explain than “sometimes we use system fonts but they won’t work on the server.”

3. **Font managers & webfont practice agree indirectly**  
   - Font managers (Suitcase, FontBase, etc.) work by managing font files and activating them into the OS. Their value proposition is central control of font files, not reliance on whatever the OS happens to have.  
   - When designers send InDesign jobs to print, the expected professional behavior is “Package with document fonts,” not “assume the printer has Calibri installed.”  
   - Webfont best practice is “self-host or use a CDN with explicit versions,” not “default to system-ui for critical branding text.”  

   You are essentially baking this discipline into the app: **fonts live in one controlled library, versioned and hashed.** That’s in line with how pros work, just with a simpler UI and rule set.

4. **How to reduce friction while keeping copy-in**  
   The real concern is usability: admins don’t want to fumble through font folders.

   I recommend:

   - **System font scanner + one-tap copy**:  
     - Show a list of common system families (Arial, Times, Calibri, Tahoma, etc.) with a “Add to Library” button next to each.  
     - When clicked, copy the actual font files into the Library, read fsType, and expose rights in plain words.  
   - **Explain once, clearly** in the Fonts panel:  
     - “OpenBulletin uses fonts in its Font Library so your bulletins look the same on every computer. When you add a system font, we make a copy into this Library. That way, we can safely package and share it with your bulletin.”  
   - **Curated free packs**:  
     - Make the first-run experience pleasant by providing a starter pack of good, church-appropriate OFL/Apache fonts so many churches never need system fonts at all.

5. **Why not live system fonts?**  
   - You’d have to explain to non-technical users:  
     - “This font works on this PC but not on the NAS.”  
     - “This bulletin will reflow if you open it on a Mac because that system font is different.”  
   - That violates Principles 2 and 6 (powerful, never lose work) and undermines trust.  
   - Technically, you’d also complicate your file format: you could no longer promise reproducible layout without recording and shipping OS-level information you don’t control.

**Conclusion:**  
Stay with the Font Library as the single source of truth, with fonts arriving via:

- Bundled free families.  
- Curated font packs.  
- Copy-from-system via a friendly picker.  
- Catalog download from OFL/Apache sources.

Do not introduce live system-font usage for layout or PDF export. The extra copy-in step is justified and aligned with best practices in both desktop publishing and self-hosted webfonts, especially given your NAS/server ambitions.