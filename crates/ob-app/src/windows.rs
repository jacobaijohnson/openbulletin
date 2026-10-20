//! The step windows and the app's operations on
//! them: every `impl App` block that draws a step or
//! a dialog and does the work behind it. The `App`
//! struct itself and the eframe dispatch stay in
//! the crate root (main.rs).

use crate::check::*;
use crate::contribution::*;
use crate::inserts::*;
use crate::preview::*;
use crate::settings::*;
use crate::*;

impl App {
    /// Move to a step the way the Next button does: leaving
    /// Announcements writes the box; arriving at Check runs it.
    /// The step list itself is clickable now (finding F1).
    pub(crate) fn goto_step(&mut self, i: usize) {
        // The steps work on the bulletin whole: a loose
        // sheet on the canvas goes back before any step
        // change (R-3).
        if i != self.step {
            self.leave_insert_editing();
        }
        if self.step == 1 && i != 1 {
            self.apply_edited_date();
        }
        if self.step == 2 && i != 2 {
            // Leaving the step writes the announcements on
            // its own; the old Save button is gone, and the
            // status says what actually happened instead of
            // promising a file save (UI audit round 2).
            let was_dirty = self.announcements_dirty;
            self.save_announcements();
            if was_dirty {
                self.note("Announcements are on the page.".into());
            }
        }
        self.step = i;
        self.remember_step();
        if i == 0 {
            // Arriving at Start re-reads the inbox (R-1).
            self.inbox_dirty = true;
        }
        if i == 3 {
            self.run_checks();
        }
    }

    pub(crate) fn rebuild(&mut self) {
        // A shape change replaces the whole document. It used to
        // be a silent discard with the autosave journal retired:
        // one stray click and the week's work was gone (blocker
        // 3). Now the old bulletin is recorded for Undo first,
        // the journal keeps protecting the new document, and the
        // church profile carries across from the Step 1 fields.
        if self.editing.is_some() {
            self.commit_edit();
        }
        self.save_announcements();
        let before = snapshot(&self.doc);
        let keep_fields = self.doc.fields.clone();
        let doc = build_template(
            self.preset,
            ChurchProfile {
                name: self.church_name.clone(),
                address: self.profile_address.clone(),
                service_times: self.profile_service_times.clone(),
                website: self.profile_website.clone(),
                giving_url: self.doc.profile.giving_url.clone(),
                pastor: self.profile_pastor.clone(),
                expected_parts: self.doc.profile.expected_parts.clone(),
                children_names: self.doc.profile.children_names,
                serving_lines: self.doc.profile.serving_lines.clone(),
                serving_aliases: self.doc.profile.serving_aliases.clone(),
            },
            chrono::Local::now().date_naive(),
            "Sunday",
        );
        let mut doc = doc;
        // Changing only the shape must not reset the week: the
        // date, its long form, the occasion name, and the reading
        // carry across (closing audit).
        for key in [
            "occasion_date",
            "occasion_date_long",
            "occasion_name",
            "reading",
        ] {
            if let Some(v) = keep_fields.get(key) {
                doc.fields.insert(key.to_string(), v.clone());
            }
        }
        self.adopt_document(doc, false);
        // A fresh template is not the old file: Save must ask
        // for a name rather than overwrite it (closing audit).
        self.path = None;
        self.print_fingerprint = None;
        self.checked_fingerprint = None;
        if before != snapshot(&self.doc) {
            self.record_undo(before);
        }
        self.status = format!(
            "Template loaded: {}. Undo brings the old bulletin back.",
            self.preset.label()
        );
    }

    pub(crate) fn save_announcements(&mut self) {
        // The buffer is one of two writers for this story (the
        // Pages canvas is the other). It may only write when it
        // actually changed, or a stale buffer erases canvas
        // typing on the next autosave tick (blocker 4). This
        // gate was once lost in a file splice; the test named
        // announcements_buffer_only_writes_when_dirty guards it.
        if !self.announcements_dirty {
            return;
        }
        // While a loose sheet is on the canvas, the
        // document's stories are the sheet's: the
        // announcements buffer belongs to the bulletin
        // and waits until the sheet goes back (R-3). The
        // dirty flag stays set, so nothing is lost.
        if self.insert_edit.is_some() {
            return;
        }
        let Some(sid) = weekly_story_id(&self.doc) else {
            return;
        };
        self.announcements_dirty = false;
        let before = snapshot(&self.doc);
        // Preserve what was typed: trim only trailing spaces,
        // keep blank lines (they are paragraph breaks). The box
        // used to trim and drop blanks, silently reformatting
        // the admin's text (finding F8). Leading and trailing
        // blank runs still go; they mean nothing on paper.
        let mut lines: Vec<String> = self
            .announcements
            .lines()
            .map(|l| l.trim_end().to_string())
            .collect();
        while lines.first().map(|l| l.is_empty()).unwrap_or(false) {
            lines.remove(0);
        }
        while lines.last().map(|l| l.is_empty()).unwrap_or(false) {
            lines.pop();
        }
        if let Some(story) = self.doc.story_mut(&sid) {
            // Spoken-only and not-for-web flags and run
            // formatting ride with their LINE, by position
            // among the story's body paragraphs (audit
            // Batch 1). They used to
            // ride by exact-text match, and the match failed
            // three ways: editing a prayer line's words
            // dropped its privacy without a word said, two
            // identical lines collapsed into one flag, and
            // every line came back with plain runs, undoing
            // bold set on the canvas. Now a line whose words
            // are unchanged keeps its whole paragraph; an
            // edited line keeps its flag and the face of its
            // first run; a line past the old count starts
            // plain and printable, like a paragraph typed
            // anywhere else.
            let old_body: Vec<ob_model::Paragraph> = story
                .paragraphs
                .iter()
                .filter(|p| p.style == "body")
                .cloned()
                .collect();
            let body_para = |idx: usize, l: &String| -> ob_model::Paragraph {
                match old_body.get(idx) {
                    Some(old) if old.text() == *l => old.clone(),
                    Some(old) => {
                        let (bold, italic) = old
                            .runs
                            .first()
                            .map(|r| (r.bold, r.italic))
                            .unwrap_or((None, None));
                        let mut p = ob_model::Paragraph::from_runs(
                            "body",
                            vec![ob_model::Run {
                                text: l.clone(),
                                bold,
                                italic,
                            }],
                        );
                        p.spoken_only = old.spoken_only;
                        p.web_hidden = old.web_hidden;
                        p
                    }
                    None => ob_model::Paragraph::new("body", l),
                }
            };
            let mut rest: Vec<ob_model::Paragraph> = story
                .paragraphs
                .iter()
                .filter(|p| p.style != "body")
                .cloned()
                .collect();
            // Keep headings/structure first, then the typed announcements.
            let mut new_paras = Vec::new();
            let mut inserted = false;
            for p in rest.drain(..) {
                new_paras.push(p.clone());
                if !inserted && p.style == "subheading" && p.text() == "Announcements" {
                    for (idx, l) in lines.iter().enumerate() {
                        new_paras.push(body_para(idx, l));
                    }
                    inserted = true;
                }
            }
            if !inserted {
                for (idx, l) in lines.iter().enumerate() {
                    new_paras.push(body_para(idx, l));
                }
            }
            story.paragraphs = new_paras;
        }
        if before != snapshot(&self.doc) {
            self.record_undo(before);
        }
    }

    /// A cheap fingerprint of the Font Library's state: file
    /// names, sizes, and modification times from a directory
    /// listing. No font bytes are read to compute it.
    pub(crate) fn library_fingerprint() -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        if let Ok(lib) = ob_fonts::FontLibrary::open(ob_fonts::default_library_dir()) {
            let dir = lib.dir_path();
            if let Ok(rd) = std::fs::read_dir(dir) {
                let mut entries: Vec<(String, u64, u64)> = rd
                    .flatten()
                    .filter_map(|e| {
                        let md = e.metadata().ok()?;
                        let mtime = md
                            .modified()
                            .ok()
                            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                            .map(|d| d.as_secs())
                            .unwrap_or(0);
                        Some((
                            e.file_name().to_string_lossy().into_owned(),
                            md.len(),
                            mtime,
                        ))
                    })
                    .collect();
                entries.sort();
                for e in entries {
                    e.hash(&mut h);
                }
            }
        }
        h.finish()
    }

    /// The one shaper, rebuilt only when the library changes.
    pub(crate) fn shaper(&mut self) -> &mut ob_text::shaping::Shaper {
        let fp = Self::library_fingerprint();
        let stale = self
            .shaper_cache
            .as_ref()
            .map(|(f, _)| *f != fp)
            .unwrap_or(true);
        if stale {
            let mut shaper = ob_text::shaping::bundled_shaper();
            if let Ok(lib) = ob_fonts::FontLibrary::open(ob_fonts::default_library_dir()) {
                for info in lib.scan() {
                    if let Ok(bytes) = lib.read(&info) {
                        shaper.register_font(bytes);
                    }
                }
            }
            self.shaper_cache = Some((fp, shaper));
        }
        &mut self.shaper_cache.as_mut().expect("shaper just built").1
    }

    pub(crate) fn run_checks(&mut self) {
        let mut notes: Vec<CheckNote> = Vec::new();
        if self.doc.profile.name.trim().is_empty() {
            notes.push(CheckNote::plain(
                "Check: the church name is blank (Step 1).".to_string(),
            ));
        } else {
            notes.push(CheckNote::plain("OK: church name is set.".to_string()));
        }
        // The date must be the coming Sunday, not just a date
        // (J1): a stale file passes every other check while
        // printing last month's Sunday. The finding's fix
        // button jumps to the This Sunday step.
        let verdict = self
            .doc
            .fields
            .get("occasion_date")
            .and_then(|iso| date_verdict(iso, chrono::Local::now().date_naive()));
        match verdict {
            Some(words) => notes.push(CheckNote::to_step(words, 1)),
            None => match self.doc.fields.get("occasion_date_long") {
                Some(d) if !d.is_empty() => {
                    notes.push(CheckNote::plain(format!("OK: bulletin date is {d}.")))
                }
                _ => notes.push(CheckNote::plain(
                    "Check: no Sunday date is set (Step 2).".to_string(),
                )),
            },
        }
        notes.push(CheckNote::plain(announcements_check(
            self.announcements.trim().is_empty(),
            self.doc.no_announcements,
        )));
        // Real fit check (T1): the thread layouts say which
        // lines of each story land in which box and how many
        // fit nowhere. A story running through several boxes
        // is judged on the whole thread, and the finding
        // names its LAST box: that is where the words run
        // out, and where a bigger box (or shorter words)
        // helps. The layouts are the cached ones the canvas
        // paints from, so Check and the page can never
        // disagree about what fits.
        let threads = self.doc_threads();
        let mut fit_notes: Vec<CheckNote> = Vec::new();
        for story in &self.doc.stories {
            let frames = self.doc.frames_of_story(&story.id);
            let Some(&(pi, fi)) = frames.last() else {
                continue;
            };
            let Some(thread) = threads.get(&story.id) else {
                continue;
            };
            let over = thread.leftover_lines;
            if over == 0 {
                continue;
            }
            let first = story
                .paragraphs
                .first()
                .map(|p| p.text())
                .unwrap_or_else(|| "A text block".into());
            fit_notes.push(CheckNote::aimed(
                format!(
                    "Check: the words in \"{}\" ({}) do not all fit. About {} line{} too many. Shorten the words, or make the box bigger in the Pages window.",
                    first.chars().take(38).collect::<String>(),
                    self.doc.panels[pi].name,
                    over,
                    if over == 1 { "" } else { "s" }
                ),
                pi,
                Some(fi),
            ));
        }
        if fit_notes.is_empty() {
            notes.push(CheckNote::plain(
                "OK: everything fits on the page.".to_string(),
            ));
        } else {
            notes.extend(fit_notes);
        }
        // Pictures (G2): an empty box, a box whose picture is
        // not in the bulletin, and a picture too small for
        // its box in print. The blur line is 120 dots per
        // inch: the church's own rescued pictures live near
        // 150, so the note waits until a picture is clearly
        // under the line instead of crying wolf weekly.
        {
            let mut probes: std::collections::HashMap<String, Option<ob_image::Probe>> =
                std::collections::HashMap::new();
            for (pi, panel) in self.doc.panels.iter().enumerate() {
                for (fi, frame) in panel.frames.iter().enumerate() {
                    if frame.kind != ob_model::FrameKind::Picture {
                        continue;
                    }
                    let Some(placement) = &frame.picture else {
                        notes.push(CheckNote::aimed(
                            format!(
                                "Check: the picture box on \"{}\" is empty. Put a picture in it in the Pages window, or delete the box.",
                                panel.name
                            ),
                            pi,
                            Some(fi),
                        ));
                        continue;
                    };
                    let Some(bytes) = self.doc.asset_data(&placement.asset) else {
                        notes.push(CheckNote::aimed(
                            format!(
                                "Check: the picture in the box on \"{}\" could not be found in this bulletin. Put the picture in again in the Pages window.",
                                panel.name
                            ),
                            pi,
                            Some(fi),
                        ));
                        continue;
                    };
                    // The children note (R-2b): while the
                    // office has not said whether this
                    // picture has children in it, Check
                    // asks, aimed at the box.
                    if picture_children_unnoted(&self.doc, frame) {
                        notes.push(CheckNote::aimed(
                            format!(
                                "Check: there is no note yet on whether there are children in this picture. Right-click the picture on \"{}\" and say.",
                                panel.name
                            ),
                            pi,
                            Some(fi),
                        ));
                    }
                    let probe = probes
                        .entry(placement.asset.clone())
                        .or_insert_with(|| ob_image::probe(bytes).ok());
                    let Some(probe) = probe else {
                        let words = match ob_image::sniff(bytes) {
                            ob_image::PictureFormat::Avif | ob_image::PictureFormat::Heic => {
                                format!(
                                    "Check: the picture in the box on \"{}\" is a kind this program cannot use yet. Convert it to JPEG or PNG and put it in again.",
                                    panel.name
                                )
                            }
                            _ => format!(
                                "Check: the picture in the box on \"{}\" could not be read. Put a good copy of it in again in the Pages window.",
                                panel.name
                            ),
                        };
                        notes.push(CheckNote::aimed(words, pi, Some(fi)));
                        continue;
                    };
                    if probe.format == ob_image::PictureFormat::Svg {
                        continue; // vector art has no dots to count
                    }
                    let Some(dpi) = picture_effective_dpi(frame, placement, probe) else {
                        notes.push(CheckNote::aimed(
                            format!(
                                "Check: the picture box on \"{}\" has no width or height, so its picture cannot print. Make the box bigger in the Pages window, or delete the box.",
                                panel.name
                            ),
                            pi,
                            Some(fi),
                        ));
                        continue;
                    };
                    if dpi < 120.0 {
                        notes.push(CheckNote::aimed(
                            format!(
                                "Check: the picture in the box on \"{}\" may look blurry in print. At this size it has about {:.0} dots per inch; a clearer copy of the picture would print better.",
                                panel.name, dpi
                            ),
                            pi,
                            Some(fi),
                        ));
                    }
                }
            }
        }
        // Missing fonts: a style naming a family that is neither
        // bundled nor in the Font Library prints in a stand-in.
        {
            let mut have: std::collections::HashSet<String> = Default::default();
            for (name, bytes) in ob_text::shaping::bundled_font_files() {
                if let Ok(info) = ob_fonts::inspect(bytes, name) {
                    have.insert(info.family.to_lowercase());
                }
            }
            if let Ok(lib) = ob_fonts::FontLibrary::open(ob_fonts::default_library_dir()) {
                for info in lib.scan() {
                    have.insert(info.family.to_lowercase());
                }
            }
            let mut missing: Vec<String> = Vec::new();
            for style in &self.doc.styles {
                let fam = style.font_family.trim();
                if !fam.is_empty()
                    && !have.contains(&fam.to_lowercase())
                    && !missing.iter().any(|m| m == fam)
                {
                    missing.push(fam.to_string());
                }
            }
            for fam in missing {
                let text = format!(
                    "Check: this bulletin uses the font \"{fam}\", which is not in \
                     your Font Library, so it will print in a stand-in font. Add \
                     it in the Fonts panel, or pick another font for those styles."
                );
                // Point at the first box whose words use the
                // missing font, when one can be named.
                let target = self
                    .doc
                    .styles
                    .iter()
                    .filter(|st| st.font_family.trim() == fam)
                    .find_map(|st| {
                        self.doc
                            .stories
                            .iter()
                            .filter(|s| s.paragraphs.iter().any(|p| p.style == st.id))
                            .find_map(|s| frame_target_for_story(&self.doc, &s.id))
                    });
                notes.push(match target {
                    Some((pi, fi)) => CheckNote::aimed(text, pi, fi),
                    None => CheckNote::plain(text),
                });
            }
            // Words near a page edge or fold: printers cannot
            // print to the very edge, and folds eat words.
            let fold_preset = matches!(
                self.doc.sheet.preset.as_str(),
                "letter-bifold"
                    | "legal-bifold"
                    | "tabloid-bifold"
                    | "letter-trifold"
                    | "saddle-booklet"
            );
            if fold_preset {
                let edge: ob_model::Mpt = 9_000; // 1/8 inch
                for (pi, panel) in self.doc.panels.iter().enumerate() {
                    for (fi, frame) in panel.frames.iter().enumerate() {
                        if frame.story_id.is_none() {
                            continue;
                        }
                        let near_left = frame.rect.x < edge;
                        let near_right = panel.size.w - (frame.rect.x + frame.rect.w) < edge;
                        let near_top = frame.rect.y < edge;
                        let near_bottom = panel.size.h - (frame.rect.y + frame.rect.h) < edge;
                        if near_left || near_right || near_top || near_bottom {
                            notes.push(CheckNote::aimed(
                                format!(
                                    "Check: a box on \"{}\" runs close to the page edge \
                                     or the fold. Printers cannot print right at the \
                                     edge; give the words a little more room.",
                                    panel.name
                                ),
                                pi,
                                Some(fi),
                            ));
                            break;
                        }
                    }
                }
            }
        }
        let has_order = self.doc.stories.iter().any(|s| {
            s.paragraphs
                .iter()
                .any(|p| p.text().contains("Order of Service"))
        });
        if has_order {
            notes.push(CheckNote::plain(
                "OK: order of service is present and carries over from last week.".to_string(),
            ));
        } else {
            notes.push(CheckNote::plain(
                "Check: no order of service found.".to_string(),
            ));
        }
        // What the automation is carrying into this week,
        // said out loud as information after the problems
        // (J10): plain notes, never "Check:" lines.
        for line in carried_lines(&self.doc) {
            notes.push(CheckNote::plain(line));
        }
        // Wanted fonts the library still does not hold
        // (docs/FONTS.md): one finding per font, in the
        // same words as the import review, its fix button
        // opening the Fonts window. The stand-in named is
        // today's resolution, so if a nearer font was
        // added since the import, the words keep up; when
        // the real font itself arrives, the finding is
        // simply gone.
        notes.extend(wanted_font_notes(&self.doc, &current_library_families()));
        // The hymn pass (R-2a): numbers and titles checked
        // against each other and against the church's own
        // remembered pairs.
        notes.extend(hymn_findings(&self.doc, &self.hymn_memory));
        // The loose sheets (R-3): a sheet with no words,
        // and a dated sheet whose date has passed, named
        // before either prints wrong.
        notes.extend(insert_findings(&self.doc));
        // The children's-names rule (R-2b): repeated here
        // as a reminder when the church chose one, honest
        // that the app cannot tell which names are
        // children's.
        if let Some(note) = children_names_note(&self.doc.profile) {
            notes.push(note);
        }
        // Characters the print copy cannot show get their own
        // findings (not plain notes) so the Check step can put
        // a fix button under each one.
        self.char_findings = char_findings(&self.doc);
        self.notes = notes;
        // A check with nothing red is a state worth keeping
        // (J6): remember which snapshot passed, under this
        // file, so the Start step can say "checked, and
        // nothing has changed since" even next week. Any
        // later edit changes the snapshot, so staleness is
        // automatic.
        let nothing_red = self.notes.iter().all(|n| !n.text.starts_with("Check:"))
            && self.char_findings.is_empty();
        if nothing_red {
            self.checked_fingerprint = Some(snapshot(&self.doc));
            self.store_doc_state();
        }
    }

    pub(crate) fn next_sunday(&mut self) {
        // Next Sunday transforms the bulletin whole: a
        // loose sheet on the canvas goes back first, so
        // its life is judged on the sheet as it stands
        // in the bulletin (R-3).
        self.leave_insert_editing();
        if self.editing.is_some() {
            self.commit_edit();
        }
        self.save_announcements();
        let before = snapshot(&self.doc);
        match ob_layout::next_sunday_report_as_of(&self.doc, chrono::Local::now().date_naive()) {
            Ok((next, report)) => {
                self.adopt_document(next, false);
                if before != snapshot(&self.doc) {
                    self.record_undo(before);
                }
                // The new week is a new file: never let a later
                // Save overwrite last week's bulletin, and never
                // call this unsaved document saved (finding F13).
                self.path = None;
                // A new week's bulletin has no print copy and
                // no clean check yet (J6).
                self.print_fingerprint = None;
                self.checked_fingerprint = None;
                self.step = 1;
                self.remember_step();
                let mut msg = format!(
                    "Started the {} bulletin. Weekly announcements were cleared; standing items carried over. Save it under its own name.",
                    self.doc.fields.get("occasion_date_long").cloned().unwrap_or_default()
                );
                // A one-time occasion says its own news
                // (R2-11): the copy is the coming regular
                // Sunday's, under the regular name. Its
                // sentence replaces the stale-file jump
                // line, which would only repeat it.
                if let Some(note) = &report.occasion_note {
                    msg.push_str(&format!(" {note}"));
                } else if report.weeks_advanced > 1 {
                    msg.push_str(&format!(
                        " The old date had passed, so this jumps ahead {} weeks to the coming Sunday.",
                        report.weeks_advanced
                    ));
                }
                if report.ended_announcements == 1 {
                    msg.push_str(" 1 announcement ended and was cleared.");
                } else if report.ended_announcements > 1 {
                    msg.push_str(&format!(
                        " {} announcements ended and were cleared.",
                        report.ended_announcements
                    ));
                }
                // Name what ended, not only the count (J10):
                // automation shows its hand.
                if !report.ended_names.is_empty() {
                    msg.push_str(&format!(" Ended: {}.", report.ended_names.join(", ")));
                }
                // The participant fields cleared for the new
                // week get named the same way (J10's shape):
                // who served is last week's news, said out
                // loud rather than silently dropped.
                if !report.cleared_participants.is_empty() {
                    msg.push_str(&format!(
                        " Cleared for the new week: {}.",
                        report.cleared_participants.join(", ")
                    ));
                }
                // The hymn lines that were cleared get
                // named the same way (R-2a): their numbers
                // belonged to last week, and the office
                // hears which lines to fill in again.
                if !report.cleared_hymns.is_empty() {
                    msg.push_str(&format!(
                        " Hymn numbers cleared for: {}.",
                        report.cleared_hymns.join(", ")
                    ));
                }
                // The loose sheets that ended get named
                // the same way (R-3): a sheet taken out
                // silently is a sheet nobody misses until
                // print morning.
                if !report.ended_inserts.is_empty() {
                    msg.push_str(&format!(
                        " Loose sheets ended: {}.",
                        report.ended_inserts.join(", ")
                    ));
                }
                // An accidental click lands here too; say
                // the way back out loud (UI audit round 2).
                msg.push_str(" Undo brings this week back.");
                self.note(msg);
            }
            Err(e) => self.fail(format!(
                "Next Sunday could not be made from this bulletin. ({e})"
            )),
        }
    }
}

impl App {
    /// Load the Font Library into the panel, seeding it with the
    /// bundled Liberation faces on first run, and register every
    /// library font with the UI so each preview line is drawn in
    /// its own face.
    pub(crate) fn refresh_fonts(&mut self, ctx: &egui::Context) {
        let Ok(lib) = ob_fonts::FontLibrary::open(ob_fonts::default_library_dir()) else {
            return;
        };
        if lib.scan().is_empty() {
            for (name, bytes) in ob_text::shaping::bundled_font_files() {
                let _ = lib.add_bytes(bytes, name);
            }
        }
        let (defs, rows) = library_font_setup(&lib);
        ctx.set_fonts(defs);
        self.font_rows = rows;
        self.fonts_loaded = true;
    }

    /// The unsaved-work comparison's one shape (round 3,
    /// R3-7): the canonical document, the same arrangement
    /// the journal writes. Opening a loose sheet on the
    /// canvas swaps that sheet into the live document,
    /// which used to read as unsaved work the moment the
    /// sheet opened; in the canonical shape the swap alone
    /// is no change at all, while a word typed in the
    /// sheet still is, because the canonical form carries
    /// the inserts.
    pub(crate) fn canonical_snapshot(&self) -> String {
        snapshot(&self.canonical_document())
    }

    /// Stamp each style with the fingerprint of the library font it
    /// will be laid out with, so saved files remember their fonts.
    /// The document on disk now matches the screen: remember the
    /// state and retire the autosave journal.
    pub(crate) fn mark_clean(&mut self) {
        self.saved_snapshot = self.canonical_snapshot();
        self.journal_snapshot = self.saved_snapshot.clone();
        let (obd, meta) = journal_paths();
        // Retire only a journal this instance may retire
        // (S-MIN-3): another live instance's safety copy is
        // not ours to clear just because this file saved.
        // The retirement goes through the journal worker
        // (HF-4), ordered behind any write already sent, so
        // it behaves exactly as the direct call did, just
        // asynchronously.
        self.request_journal_retire(&obd, &meta);
    }

    /// Ask the journal worker to retire the journal at
    /// these paths, through the token-guarded path
    /// (`retire_journal_from`, run on the worker). Every
    /// in-session retirement goes through here, so it is
    /// serialized with the journal's writes on the one
    /// worker channel: a write enqueued before the retire
    /// lands first, and none can land after it.
    pub(crate) fn request_journal_retire(&mut self, obd: &std::path::Path, meta: &std::path::Path) {
        let lock = instance_lock_path();
        let token = instance_token().to_string();
        self.enqueue_journal_job(JournalJob::Retire(JournalRetireJob {
            obd: obd.to_path_buf(),
            meta: meta.to_path_buf(),
            lock,
            token,
        }));
    }

    /// Hand one job to the journal worker under the
    /// single-flight discipline: at most one job in flight
    /// and at most one waiting behind it. A waiting write
    /// is replaced by the newest write (coalesced — the
    /// backlog never grows), and a retire replaces a
    /// waiting write outright, because that write was
    /// enqueued before the retirement was requested and
    /// must never land after it. Once a retirement is
    /// requested, write jobs are not sent at all until it
    /// lands; a write that comes due in that window is not
    /// lost, because the journal snapshot still differs
    /// and a later tick sends it once retirement completes.
    pub(crate) fn enqueue_journal_job(&mut self, job: JournalJob) {
        if self.journal_retiring && matches!(job, JournalJob::Write(_)) {
            return;
        }
        if matches!(job, JournalJob::Retire(_)) {
            self.journal_retiring = true;
        }
        if self.journal_flight.is_some() {
            self.journal_pending = Some(job);
        } else {
            self.send_journal_job(job);
        }
    }

    /// Send one job to the worker now, spawning the worker
    /// on first use. A send that fails means the worker
    /// thread is gone: a lost write is spoken once per
    /// session, like any other journal write failure; a
    /// lost retire just reopens the gate, leaving the
    /// journal in place (the safe direction).
    pub(crate) fn send_journal_job(&mut self, job: JournalJob) {
        if self.journal_worker.is_none() {
            self.journal_worker = Some(JournalWorker::spawn());
        }
        let flight = match &job {
            JournalJob::Write(_) => JournalFlight::Write,
            JournalJob::Retire(_) => JournalFlight::Retire,
        };
        let outcome = self
            .journal_worker
            .as_ref()
            .expect("worker ensured above")
            .jobs
            .send(job);
        match outcome {
            Ok(()) => {
                self.journal_flight = Some(flight);
            }
            Err(sent) => {
                self.journal_worker = None;
                match sent.0 {
                    JournalJob::Write(_) => self.note_journal_failure(),
                    JournalJob::Retire(_) => {
                        self.journal_retiring = false;
                    }
                }
            }
        }
    }

    /// Send the waiting job, if one is waiting and nothing
    /// is in flight.
    pub(crate) fn dispatch_pending_journal_job(&mut self) {
        if self.journal_flight.is_none() {
            if let Some(job) = self.journal_pending.take() {
                self.send_journal_job(job);
            }
        }
    }

    /// A journal write failure, spoken once per session
    /// (S-MIN-5): dead autosave is never silent, and never
    /// a banner every two seconds either. Later ticks keep
    /// retrying quietly.
    pub(crate) fn note_journal_failure(&mut self) {
        if !self.journal_failure_noted {
            self.journal_failure_noted = true;
            self.fail(
                "The safety copy could not be written, so your unsaved \
                 work is not being kept safe. Save your bulletin to a \
                 file to keep it safe."
                    .into(),
            );
        }
    }

    /// Collect the journal worker's results, if any landed
    /// (HF-4): called from `update` beside `poll_import`.
    /// Never blocks: an empty channel just asks for another
    /// frame soon while a job is in flight.
    pub(crate) fn poll_journal(&mut self, ctx: &egui::Context) {
        let Some(worker) = &self.journal_worker else {
            return;
        };
        match worker.results.try_recv() {
            Ok(JournalResult::Written { snapshot, outcome }) => {
                self.journal_flight = None;
                match outcome {
                    Ok(()) => {
                        // The journal on disk now holds this
                        // snapshot — unless a retirement was
                        // requested after the write was sent:
                        // then the snapshot belongs to the
                        // retirement's callers, who already
                        // recorded it, and must not be
                        // regressed to the written state.
                        if !self.journal_retiring {
                            self.journal_snapshot = snapshot;
                        }
                    }
                    Err(_) => self.note_journal_failure(),
                }
                self.dispatch_pending_journal_job();
            }
            Ok(JournalResult::Retired) => {
                self.journal_flight = None;
                self.journal_retiring = false;
                self.dispatch_pending_journal_job();
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                if self.journal_flight.is_some() {
                    ctx.request_repaint_after(std::time::Duration::from_millis(120));
                }
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                // The worker thread stopped. A write lost
                // with it is a journal write failure, spoken
                // once; a retire lost with it leaves the
                // journal in place and stays quiet, as
                // retirements always have.
                let flight = self.journal_flight;
                self.journal_worker = None;
                self.journal_flight = None;
                self.journal_pending = None;
                self.journal_retiring = false;
                if flight == Some(JournalFlight::Write) {
                    self.note_journal_failure();
                }
            }
        }
    }

    /// Remember the bulletin file this office just opened or
    /// saved, so the Start step can offer it back next time
    /// (UI audit round 2). Best effort: a settings file that
    /// will not write never blocks the work itself.
    pub(crate) fn remember_bulletin(&mut self, path: &std::path::Path) {
        let s = path.to_string_lossy().into_owned();
        if self.settings.last_bulletin.as_deref() != Some(s.as_str()) {
            self.settings.last_bulletin = Some(s);
            let _ = save_settings(&self.settings);
        }
    }

    /// Load the remembered state of the file now open (J6):
    /// its print and checked fingerprints become this
    /// session's, so freshness speaks across restarts.
    pub(crate) fn seed_doc_state(&mut self) {
        let state = self
            .path
            .as_ref()
            .and_then(|p| self.settings.doc_state(&p.to_string_lossy()))
            .cloned()
            .unwrap_or_default();
        self.print_fingerprint = state.print_fingerprint;
        self.checked_fingerprint = state.checked_fingerprint;
    }

    /// Write this session's print and checked fingerprints
    /// back under the current file's path (J6). No file open,
    /// nothing to remember under. Best effort, like every
    /// settings write: a settings file that will not write
    /// never blocks the work itself.
    pub(crate) fn store_doc_state(&mut self) {
        let Some(path) = &self.path else { return };
        let key = path.to_string_lossy().into_owned();
        let state = DocState {
            print_fingerprint: self.print_fingerprint.clone(),
            checked_fingerprint: self.checked_fingerprint.clone(),
        };
        self.settings.set_doc_state(key, state);
        let _ = save_settings(&self.settings);
    }

    /// Remember which step the user is on (J6), so the app
    /// reopens where they left it.
    pub(crate) fn remember_step(&mut self) {
        if self.settings.last_step != self.step {
            self.settings.last_step = self.step;
            let _ = save_settings(&self.settings);
        }
    }

    /// Once per launch: adopt the current document as the saved
    /// state, and pick up any journal a previous session left.
    pub(crate) fn startup(&mut self) {
        self.startup_done = true;
        self.saved_snapshot = self.canonical_snapshot();
        self.journal_snapshot = self.saved_snapshot.clone();
        // Single-instance courtesy: if another live OpenBulletin
        // already holds the journal, this instance leaves the
        // journal entirely alone (closing audit).
        self.peer_alive = peer_instance_alive();
        write_instance_lock();
        if self.peer_alive {
            // The journal belongs to the other window, so
            // this window's work is not being kept safe. Say
            // so on the first frame instead of losing it
            // silently (UI audit round 2).
            self.fail(
                "OpenBulletin is already open in another window. Work in this window \
                 is not being kept safe; close one of them."
                    .into(),
            );
        }
        let (obd, meta) = journal_paths();
        if let Some((doc, source, when)) = read_journal_from(&obd, &meta) {
            if snapshot(&doc) != self.saved_snapshot {
                self.journal_snapshot = snapshot(&doc);
                if !self.peer_alive {
                    self.recovery = Some((doc, source, when));
                }
            } else if !self.peer_alive {
                // Startup runs before the journal worker
                // exists (HF-4): this retirement, and the
                // set-aside below, stay on the startup path.
                retire_journal_from(&obd, &meta, &instance_lock_path(), instance_token());
            }
        } else if !self.peer_alive && (obd.exists() || meta.exists()) {
            // A journal is on disk but did not read: the crash
            // copy was damaged. Say so once, in plain words,
            // and set the remains aside instead of the old
            // silent skip (audit Batch 3). Never block on it:
            // the app opens normally either way.
            if set_aside_unreadable_journal(&obd, &meta) {
                self.fail(
                    "The safety copy from last time could not be read, so it was set \
                     aside (autosave-unreadable) instead of offered back. Your saved \
                     bulletin files were not touched."
                        .into(),
                );
            }
        }
    }

    /// The autosave tick: about every two seconds, if the work on
    /// screen differs from the last save, write the journal.
    pub(crate) fn journal_tick(&mut self) {
        let (obd, meta) = journal_paths();
        self.journal_tick_to(&obd, &meta);
    }

    /// The tick itself, against named journal paths (the
    /// app passes `journal_paths()`; tests pass a scratch
    /// folder's). The frame thread keeps only the throttle
    /// and the changed-snapshot decision (HF-4): the save
    /// itself is a job for the journal worker, and its
    /// outcome lands through `poll_journal`. The snapshot
    /// recorded for a write is set when the write lands,
    /// not here, so a failed write is retried by a later
    /// tick, exactly as the synchronous write was.
    pub(crate) fn journal_tick_to(&mut self, obd: &std::path::Path, meta: &std::path::Path) {
        if self.last_journal_check.elapsed() < std::time::Duration::from_secs(2) {
            return;
        }
        self.last_journal_check = std::time::Instant::now();
        self.save_announcements();
        let snap = self.canonical_snapshot();
        match journal_decision(
            snap == self.saved_snapshot,
            snap == self.journal_snapshot,
            self.recovery.is_some(),
            self.peer_alive,
        ) {
            JournalAction::Hold => {}
            JournalAction::Clear => {
                self.journal_snapshot = snap;
                self.request_journal_retire(obd, meta);
            }
            JournalAction::Write => {
                // The journal is a file like any other: it
                // holds the bulletin whole, never the
                // inside-out shape of a sheet on the canvas
                // (R-3). The snapshot comparisons above are
                // in the canonical shape too (round 3,
                // R3-7), so the bytes written and the state
                // they are compared against agree.
                let doc = self.canonical_document();
                let source = self.path.clone();
                let token = instance_token().to_string();
                self.enqueue_journal_job(JournalJob::Write(Box::new(JournalWriteJob {
                    obd: obd.to_path_buf(),
                    meta: meta.to_path_buf(),
                    doc,
                    source,
                    token,
                    snapshot: snap,
                })));
            }
        }
    }

    /// The two-click unsaved-work guard, shared by every way
    /// of opening a file. Returns true when the open may
    /// proceed. Arming records the screen state; if the work
    /// changes before the confirming click, this re-arms
    /// instead of proceeding (closing audit).
    pub(crate) fn open_guard(&mut self, action: PendingOpen) -> bool {
        let dirty = self.frame_snapshot != self.saved_snapshot;
        if !dirty {
            self.pending_open = None;
            return true;
        }
        if self.pending_open == Some(action) && self.pending_open_snapshot == self.frame_snapshot {
            self.pending_open = None;
            return true;
        }
        self.pending_open = Some(action);
        self.pending_open_snapshot = self.frame_snapshot.clone();
        self.note(
            "You have unsaved work on screen. Click the same button again to leave it \
             behind; it will be gone for good. Or Save first."
                .into(),
        );
        false
    }

    /// Finish any canvas typing session so Save and Print
    /// never miss the newest words (closing audit).
    pub(crate) fn commit_if_editing(&mut self) {
        if self.editing.is_some() {
            self.commit_edit();
        }
    }

    pub(crate) fn pin_fonts(&mut self) {
        if let Ok(lib) = ob_fonts::FontLibrary::open(ob_fonts::default_library_dir()) {
            let bundled = ob_text::shaping::bundled_font_infos();
            ob_fonts::pin_document_fonts(&mut self.doc, &lib, &bundled);
        }
    }

    /// Collect a finished catalog search or install, if one landed.
    pub(crate) fn poll_catalog(&mut self, ctx: &egui::Context) {
        let Some(rx) = &self.catalog_rx else { return };
        match rx.try_recv() {
            Ok(CatalogMsg::Hits(hits)) => {
                self.catalog_hits = hits;
                self.catalog_rx = None;
                self.catalog_busy = false;
            }
            Ok(CatalogMsg::Installed(names)) => {
                self.note(format!("Font installed: {}", names.join(", ")));
                self.catalog_rx = None;
                self.catalog_busy = false;
                self.refresh_fonts(ctx);
            }
            Ok(CatalogMsg::PackDone { installed, failed }) => {
                // Font results go through note()/fail() like
                // every other outcome, so a failure gets the
                // banner instead of whispering (UI audit
                // round 2, batch 3).
                if failed.is_empty() {
                    self.note(format!(
                        "Church Starter Pack installed: {} fonts",
                        installed.len()
                    ));
                } else {
                    self.fail(format!(
                        "Starter Pack mostly installed: {} fonts in; could not get {}",
                        installed.len(),
                        failed
                            .iter()
                            .map(|(id, _)| id.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                }
                self.catalog_rx = None;
                self.catalog_busy = false;
                self.refresh_fonts(ctx);
            }
            Ok(CatalogMsg::Failed(e)) => {
                self.fail(format!("Font catalog: {e}"));
                self.catalog_rx = None;
                self.catalog_busy = false;
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                ctx.request_repaint_after(std::time::Duration::from_millis(120));
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.catalog_rx = None;
                self.catalog_busy = false;
                // A stopped catalog worker speaks, like a
                // stopped import reader (S-MIN-4): the Fonts
                // search used to just stop spinning in
                // silence.
                self.fail(
                    "The font catalog stopped, so fonts cannot be searched or \
                     fetched until you restart OpenBulletin."
                        .into(),
                );
            }
        }
    }

    /// Offer back work a previous session left unsaved.
    pub(crate) fn recovery_window(&mut self, ctx: &egui::Context) {
        if self.recovery.is_none() {
            return;
        }
        let mut bring_back = false;
        let mut throw_away = false;
        egui::Window::new("Your work from last time")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                let when = self.recovery.as_ref().map(|(_, _, w)| *w).unwrap_or(0);
                let clock = chrono::DateTime::from_timestamp(when as i64, 0)
                    .map(|utc| {
                        utc.with_timezone(&chrono::Local)
                            .format("%A at %I:%M %p")
                            .to_string()
                    })
                    .unwrap_or_else(|| "earlier".to_string());
                ui.label(format!(
                    "OpenBulletin kept a copy of the bulletin you were editing, \
                     saved on {clock}. It was never saved to a file."
                ));
                if let Some(source) = self.recovery.as_ref().and_then(|(_, src, _)| src.as_ref()) {
                    ui.label(format!("It belongs to the file: {}", source.display()));
                } else {
                    ui.label("It was a brand-new bulletin, never saved anywhere.");
                }
                ui.horizontal(|ui| {
                    if ui.button("Bring it back").clicked() {
                        bring_back = true;
                        self.discard_armed = false;
                    }
                    let discard_label = if self.discard_armed {
                        "Click again to throw it away for good"
                    } else {
                        "Throw it away"
                    };
                    if ui.button(discard_label).clicked() {
                        if self.discard_armed {
                            throw_away = true;
                            self.discard_armed = false;
                        } else {
                            self.discard_armed = true;
                        }
                    }
                });
            });
        if bring_back {
            if let Some((doc, source, _)) = self.recovery.take() {
                self.adopt_document(doc, true);
                if let Some(src) = &source {
                    self.remember_bulletin(src);
                }
                self.path = source;
                self.seed_doc_state();
                // The file on disk still holds its older
                // saved state: the save guard's stamp is of
                // the disk, not of the screen (R-1).
                self.note_disk_stamp();
                // The journal's content is now the screen's content:
                // the saved state stays the older file (or nothing).
                self.journal_snapshot = self.canonical_snapshot();
                self.status = "Your unsaved work is back. Save when ready.".into();
            }
        }
        if throw_away {
            self.recovery = None;
            let (obd, meta) = journal_paths();
            self.request_journal_retire(&obd, &meta);
            self.journal_snapshot = self.saved_snapshot.clone();
            self.status = "Threw away the unsaved copy.".into();
        }
    }

    /// Build (or reuse) the sheet preview for the Print step.
    pub(crate) fn preview_sheets(&mut self) -> &Vec<PreviewSheet> {
        let key = self.frame_snapshot.clone();
        let stale = self
            .preview_cache
            .as_ref()
            .map(|(k, _)| k != &key)
            .unwrap_or(true);
        if stale {
            let sheets = build_preview(&self.doc);
            self.preview_cache = Some((key, sheets));
        }
        &self.preview_cache.as_ref().expect("cache filled").1
    }

    /// The Print step's preview: every sheet side, drawn to scale,
    /// with fold lines and the real shaped text.
    pub(crate) fn sheet_preview_ui(&mut self, ui: &mut egui::Ui) {
        // Build first (borrows self), then draw from the cache.
        let count = self.preview_sheets().len();
        // The caption says only what the picture can back up
        // (UI audit round 2, batch 3): the words are drawn in
        // the bulletin's own faces and the breaks are the
        // printed breaks; the gray names and outlines are
        // guides, and guides never print.
        ui.label(format!(
            "The printed sheets, one side at a time: {count} sheet side{}. \
             Lines break where they will print, in your fonts. \
             The gray panel names and outlines are guides; they do not print.",
            if count == 1 { "" } else { "s" }
        ));
        let sheets = self.preview_sheets().clone();
        let rows = self.font_rows.clone();
        // The sheets' pictures, decoded through the same
        // door as the canvas (G2), keyed by fingerprint.
        let mut textures: std::collections::HashMap<String, egui::TextureHandle> =
            std::collections::HashMap::new();
        for sheet in &sheets {
            for panel in &sheet.panels {
                for pic in &panel.pictures {
                    if textures.contains_key(&pic.asset) {
                        continue;
                    }
                    if let Some(bytes) = self.doc.asset_data(&pic.asset).map(|b| b.to_vec()) {
                        let key = format!("asset:{}", pic.asset);
                        if let Some((tex, _, _)) =
                            self.picture_texture(ui.ctx(), &key, &bytes, 1024)
                        {
                            textures.insert(pic.asset.clone(), tex);
                        }
                    }
                }
            }
        }
        // The preview is a picture, not the point of the
        // step: cap the room it takes (its own little scroll
        // box) so the copy buttons stay within reach on
        // smaller windows (UI audit round 2, blocker B1).
        egui::ScrollArea::vertical()
            .id_salt("sheet-preview")
            .max_height(300.0)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (i, sheet) in sheets.iter().enumerate() {
                    let pair = i / 2 + 1;
                    let face = if i % 2 == 0 { "front" } else { "back" };
                    ui.label(egui::RichText::new(format!("Sheet {pair}, {face}")).strong());
                    draw_sheet(ui, sheet, &rows, &self.bound_families, &textures);
                }
            });
    }

    /// The settings to use in the computer's print window, in plain
    /// words, computed from this document and the learned printer.
    pub(crate) fn print_checklist(&self) -> Vec<String> {
        print_checklist_for(&self.doc, &self.settings, &self.printer_name)
    }

    /// The print-window guidance for the printer currently
    /// named on the Print step (J5).
    pub(crate) fn current_print_guidance(&self) -> String {
        let setup = self.settings.printers.get(&self.printer_name);
        print_dialog_guidance(setup.is_some(), setup.map(|s| s.instruction.as_str()))
    }

    /// Paragraph count sync for the edit buffer: new paragraphs
    /// inherit the previous paragraph's style (docs/EDITOR.md).
    pub(crate) fn sync_edit_styles(&mut self) {
        // Changes happen at the cursor (audit R2): styles used to
        // grow and shrink only at the end, so a paragraph inserted
        // in the middle silently shifted every style below it.
        let count = self.edit_text.split('\n').count();
        let cur = self
            .edit_cursor_para
            .min(self.edit_styles.len().saturating_sub(1));
        if self.edit_styles.len() < count {
            let inherit = self
                .edit_styles
                .get(cur)
                .cloned()
                .unwrap_or_else(|| "body".to_string());
            while self.edit_styles.len() < count {
                let at = (cur + 1).min(self.edit_styles.len());
                self.edit_styles.insert(at, inherit.clone());
            }
        } else if self.edit_styles.len() > count {
            let excess = self.edit_styles.len() - count;
            let start = (cur + 1).min(self.edit_styles.len() - excess);
            self.edit_styles.drain(start..start + excess);
        }
    }

    /// Keep `edit_spoken` the same length as the edit
    /// buffer's paragraph list, with the same cursor-side
    /// growth and shrink as [`App::sync_edit_styles`]: a
    /// paragraph split in two keeps the flag on both halves
    /// (a split prayer list stays private), a deleted
    /// paragraph takes its flag with it.
    pub(crate) fn sync_edit_spoken(&mut self) {
        let count = self.edit_text.split('\n').count();
        let cur = self
            .edit_cursor_para
            .min(self.edit_spoken.len().saturating_sub(1));
        if self.edit_spoken.len() < count {
            let inherit = self.edit_spoken.get(cur).copied().unwrap_or(false);
            while self.edit_spoken.len() < count {
                let at = (cur + 1).min(self.edit_spoken.len());
                self.edit_spoken.insert(at, inherit);
            }
        } else if self.edit_spoken.len() > count {
            let excess = self.edit_spoken.len() - count;
            let start = (cur + 1).min(self.edit_spoken.len() - excess);
            self.edit_spoken.drain(start..start + excess);
        }
    }

    /// Keep `edit_web_hidden` the same length as the edit
    /// buffer's paragraph list, with the same cursor-side
    /// growth and shrink as [`App::sync_edit_spoken`]: a
    /// paragraph split in two keeps the flag on both
    /// halves (a split line of children's names stays off
    /// the web page), a deleted paragraph takes its flag
    /// with it.
    pub(crate) fn sync_edit_web_hidden(&mut self) {
        let count = self.edit_text.split('\n').count();
        let cur = self
            .edit_cursor_para
            .min(self.edit_web_hidden.len().saturating_sub(1));
        if self.edit_web_hidden.len() < count {
            let inherit = self.edit_web_hidden.get(cur).copied().unwrap_or(false);
            while self.edit_web_hidden.len() < count {
                let at = (cur + 1).min(self.edit_web_hidden.len());
                self.edit_web_hidden.insert(at, inherit);
            }
        } else if self.edit_web_hidden.len() > count {
            let excess = self.edit_web_hidden.len() - count;
            let start = (cur + 1).min(self.edit_web_hidden.len() - excess);
            self.edit_web_hidden.drain(start..start + excess);
        }
    }

    /// Enter inline editing for one frame's story. Any session
    /// already open commits first: starting a new one must never
    /// silently drop the words in the old one (blocker 6).
    /// Throw the typing away: Escape in a text box means cancel
    /// in the office tools our users know (audit finding C1,
    /// resolved for the office convention). Done typing, or
    /// clicking another box, is what saves.
    /// Paste a copied box onto a panel: fresh ids, the copied
    /// words come along, dropped a little down and right of the
    /// original like every office app does.
    pub(crate) fn paste_box(
        &mut self,
        idx: usize,
        mut frame: ob_model::Frame,
        story: Option<ob_model::Story>,
    ) {
        self.push_undo_now();
        if let Some(mut st) = story {
            self.box_seq += 1;
            let mut n = self.box_seq;
            let mut sid = format!("box-{n}");
            while self.doc.story(&sid).is_some() {
                n += 1;
                sid = format!("box-{n}");
            }
            self.box_seq = n;
            st.id = sid.clone();
            self.doc.stories.push(st);
            frame.story_id = Some(sid);
        }
        frame.id = format!(
            "frame-{}",
            frame.story_id.clone().unwrap_or_else(|| "shape".into())
        );
        let panel = &mut self.doc.panels[idx];
        frame.rect.x = (frame.rect.x + 18_000).min(panel.size.w - frame.rect.w);
        frame.rect.y = (frame.rect.y + 18_000).min(panel.size.h - frame.rect.h);
        frame.name = format!("{} copy", frame.name);
        panel.frames.push(frame);
        self.selected_frame = Some(panel.frames.len() - 1);
        self.note("Box pasted. Drag it where it belongs.".into());
    }

    /// Add a fresh text box to a panel, with its own new story
    /// (finding D2: boxes could not be born before this).
    pub(crate) fn add_text_box(&mut self, idx: usize) {
        if self.editing.is_some() {
            self.commit_edit();
        }
        self.push_undo_now();
        self.box_seq += 1;
        let mut n = self.box_seq;
        let mut sid = format!("box-{n}");
        while self.doc.story(&sid).is_some() {
            n += 1;
            sid = format!("box-{n}");
        }
        self.box_seq = n;
        self.doc.stories.push(ob_model::Story {
            id: sid.clone(),
            paragraphs: vec![ob_model::Paragraph::new(
                "body",
                "New box. Double-click and type.",
            )],
            keep_through: None,
        });
        let panel = &mut self.doc.panels[idx];
        let w = 216_000.min(panel.size.w);
        let h = 108_000.min(panel.size.h);
        panel.frames.push(ob_model::Frame {
            id: format!("frame-{sid}"),
            name: "Text box".into(),
            kind: ob_model::FrameKind::Text,
            rect: ob_model::Rect {
                x: (panel.size.w - w) / 2,
                y: (panel.size.h - h) / 2,
                w,
                h,
            },
            story_id: Some(sid),
            role: ob_model::ContentRole::Weekly,
            picture: None,
            music: None,
        });
        self.selected_frame = Some(panel.frames.len() - 1);
        self.note("Text box added in the middle of the page. Drag it where it belongs.".into());
    }

    /// Add a fresh picture box to a panel: empty, in the
    /// middle, waiting for its picture (G2).
    /// Arm the continuation pick (T1): the next text box
    /// clicked becomes the continuation of this box's text.
    pub(crate) fn arm_thread_from(&mut self, pi: usize, fi: usize) {
        self.thread_arm = Some((pi, fi));
        self.thread_confirm = None;
        self.note(
            "Click the box this text should continue in. It can be on another page. Click an empty part of the page to stop."
                .into(),
        );
    }

    /// A canvas click while a continuation pick is armed
    /// (T1). `hit` is the frame clicked, or None for empty
    /// page (which cancels). Every refusal says so in plain
    /// words and leaves the document untouched.
    pub(crate) fn thread_pick_clicked(&mut self, pi: usize, hit: Option<usize>) {
        let Some((spi, sfi)) = self.thread_arm else {
            return;
        };
        let Some(tfi) = hit else {
            self.thread_arm = None;
            self.thread_confirm = None;
            self.note("No box chosen. The text stays as it is.".into());
            return;
        };
        if (pi, tfi) == (spi, sfi) {
            self.note(
                "That is the same box. Click a different box for the text to continue in.".into(),
            );
            return;
        }
        let target = self.doc.panels[pi].frames[tfi].clone();
        if target.kind != ob_model::FrameKind::Text || target.story_id.is_none() {
            self.note(
                "That box holds no text. Click a text box for the text to continue in.".into(),
            );
            return;
        }
        let source_sid = self.doc.panels[spi].frames[sfi]
            .story_id
            .clone()
            .unwrap_or_default();
        if target.story_id.as_deref() == Some(source_sid.as_str()) {
            self.thread_arm = None;
            self.thread_confirm = None;
            self.note("That box already continues this text.".into());
            return;
        }
        // The continuation must come after the source box in
        // reading order, or the words would flow backwards.
        let source_pos = (spi, sfi);
        if (pi, tfi) <= source_pos {
            self.note(
                "That box comes before this one. Text continues into a box that comes later: pick a box further down, or on a later page."
                    .into(),
            );
            return;
        }
        // A target with words of its own loses them: its
        // story is replaced by the continuation. That takes
        // a second click on the same box, said out loud.
        let target_has_words = target
            .story_id
            .as_ref()
            .and_then(|sid| self.doc.story(sid))
            .map(|s| s.paragraphs.iter().any(|p| !p.text().trim().is_empty()))
            .unwrap_or(false);
        if target_has_words && self.thread_confirm != Some((pi, tfi)) {
            self.thread_confirm = Some((pi, tfi));
            self.note(
                "That box has its own words. Click it once more to replace them with this text's continuation; its words will be removed."
                    .into(),
            );
            return;
        }
        self.push_undo_now();
        // The target's old story goes away if no box uses
        // it anymore (checked after the switch).
        let old = target.story_id.clone().unwrap_or_default();
        self.doc.panels[pi].frames[tfi].story_id = Some(source_sid);
        let still_used = self
            .doc
            .panels
            .iter()
            .flat_map(|p| &p.frames)
            .any(|fr| fr.story_id.as_deref() == Some(old.as_str()));
        if !still_used {
            self.doc.stories.retain(|st| st.id != old);
        }
        self.thread_arm = None;
        self.thread_confirm = None;
        self.selected_frame = Some(tfi);
        self.note(
            "The text now continues in that box. Each line prints once, flowing from box to box."
                .into(),
        );
    }

    /// "Make this box start its own text" (T1): the thread
    /// divides at this box. This box and every box after it
    /// in the thread get a new story holding the words from
    /// this box's first line onward, cut at the continuation
    /// offsets the thread layout computed.
    pub(crate) fn split_thread_at(&mut self, pi: usize, fi: usize) {
        if self.editing.is_some() {
            self.commit_edit();
        }
        let Some(sid) = self.doc.panels[pi].frames[fi].story_id.clone() else {
            return;
        };
        let frames = self.doc.frames_of_story(&sid);
        let Some(pos) = frames.iter().position(|&f| f == (pi, fi)) else {
            return;
        };
        if pos == 0 || frames.len() < 2 {
            return;
        }
        let threads = self.doc_threads();
        let Some(thread) = threads.get(&sid) else {
            return;
        };
        let (para_idx, resolved_off) = thread.slices[pos].starts_at;
        let Some(story) = self.doc.story(&sid).cloned() else {
            return;
        };
        if para_idx >= story.paragraphs.len() {
            self.note("This box holds the end of the text; there is nothing after it to start a new text with.".into());
            return;
        }
        let raw_off = ob_model::raw_offset_for_resolved(
            &story.paragraphs[para_idx].text(),
            &self.doc,
            resolved_off,
        );
        let (head, tail) = ob_model::split_story_paragraphs(&story.paragraphs, para_idx, raw_off);
        self.push_undo_now();
        // A fresh story id in the box-N family, skipping
        // ids already taken.
        let mut n = self.box_seq;
        let new_id = loop {
            n += 1;
            let cand = format!("box-{n}");
            if self.doc.story(&cand).is_none() {
                break cand;
            }
        };
        self.box_seq = n;
        if let Some(s) = self.doc.story_mut(&sid) {
            s.paragraphs = head;
        }
        self.doc.stories.push(ob_model::Story {
            id: new_id.clone(),
            paragraphs: tail,
            keep_through: None,
        });
        for &(p2, f2) in &frames[pos..] {
            self.doc.panels[p2].frames[f2].story_id = Some(new_id.clone());
        }
        self.note(
            "This box now starts its own text, from the words that were in it. The boxes before it keep the rest."
                .into(),
        );
    }

    pub(crate) fn add_picture_box(&mut self, idx: usize) {
        if self.editing.is_some() {
            self.commit_edit();
        }
        self.push_undo_now();
        self.box_seq += 1;
        let seq = self.box_seq;
        let panel = &mut self.doc.panels[idx];
        let w = 216_000.min(panel.size.w);
        let h = 162_000.min(panel.size.h);
        panel.frames.push(ob_model::Frame {
            id: format!("frame-pic-{seq}"),
            name: "Picture box".into(),
            kind: ob_model::FrameKind::Picture,
            rect: ob_model::Rect {
                x: (panel.size.w - w) / 2,
                y: (panel.size.h - h) / 2,
                w,
                h,
            },
            story_id: None,
            role: ob_model::ContentRole::Weekly,
            picture: None,
            music: None,
        });
        self.selected_frame = Some(panel.frames.len() - 1);
        self.note(
            "Picture box added in the middle of the page. With it selected, use Picture... to fill it."
                .into(),
        );
    }

    pub(crate) fn cancel_edit(&mut self) {
        // Escape still cancels, and the page keeps its old
        // words (Batch A law). But a session whose words
        // changed is not gone without a trace (J3): record
        // what was typed, so the "Bring the typed words
        // back" button can reopen the box with them.
        if let Some((pi, fi)) = self.editing {
            if self.edit_text != self.edit_start_text {
                self.discarded_edit =
                    Some((self.edit_salt.clone(), pi, fi, self.edit_text.clone()));
            }
        }
        self.editing = None;
        self.edit_start = None;
        self.edit_start_text.clear();
        self.edit_text.clear();
        self.edit_styles.clear();
        self.edit_runs.clear();
        self.edit_runs_detached = false;
        self.edit_spoken.clear();
        self.edit_web_hidden.clear();
        self.edit_done = false;
        self.reset_editor_transient();
        self.status = "Typing thrown away. The page keeps the words it had.".into();
    }

    /// Clear the shaped editor's per-session state (E1):
    /// caret, selection, chip arm, cached layout, and any
    /// IME composition. Called when a session opens and when
    /// it closes, so no stale geometry survives a buffer swap.
    pub(crate) fn reset_editor_transient(&mut self) {
        self.edit_caret = 0;
        self.edit_anchor = 0;
        self.edit_caret_to_slice = false;
        self.edit_chip_armed = None;
        self.edit_sel_drag = false;
        self.edit_desired_x = None;
        self.edit_layout = None;
        self.edit_layout_failed = false;
        self.ime_preedit = None;
    }

    /// Bring back the words a cancelled typing session threw
    /// away (J3): reopen the session on the same box with the
    /// recorded words as the buffer. If the box is not there
    /// anymore, say so and drop the record.
    pub(crate) fn restore_discarded_edit(&mut self) {
        let Some((sid, pi, fi, text)) = self.discarded_edit.take() else {
            return;
        };
        let still_there = self
            .doc
            .panels
            .get(pi)
            .and_then(|p| p.frames.get(fi))
            .map(|f| f.story_id.as_deref() == Some(sid.as_str()))
            .unwrap_or(false);
        if !still_there {
            self.fail(
                "Those typed words belonged to a box that is not there anymore, \
                 so they cannot be brought back."
                    .into(),
            );
            return;
        }
        self.show_pages = true;
        self.canvas_panel = pi;
        self.enter_edit(pi, fi);
        self.edit_text = text;
        // The brought-back words are plain text: whatever
        // formatting the thrown-away session had is not part
        // of the record (J3 recorded words only), so the runs
        // restart plain rather than describe other words.
        self.edit_runs = self
            .edit_text
            .split('\n')
            .map(|t| vec![ob_model::Run::plain(t)])
            .collect();
        self.edit_runs_detached = false;
        self.note(
            "Your typed words are back in the box. Click away, or use Done \
             typing, to keep them."
                .into(),
        );
    }

    pub(crate) fn enter_edit(&mut self, pi: usize, fi: usize) {
        if self.editing.is_some() {
            self.commit_edit();
        }
        // Typing supersedes an armed continuation pick (T1).
        self.thread_arm = None;
        self.thread_confirm = None;
        let Some(frame) = self.doc.panels[pi].frames.get(fi) else {
            return;
        };
        let Some(sid) = frame.story_id.clone() else {
            return;
        };
        let Some(story) = self.doc.story(&sid).cloned() else {
            return;
        };
        self.edit_start = Some(snapshot(&self.doc));
        self.edit_salt = story.id.clone();
        self.edit_cursor_para = 0;
        // A new session supersedes any thrown-away words
        // still on offer (J3).
        self.discarded_edit = None;
        self.edit_text = story
            .paragraphs
            .iter()
            .map(|p| p.text())
            .collect::<Vec<_>>()
            .join("\n");
        self.edit_start_text = self.edit_text.clone();
        self.edit_styles = story.paragraphs.iter().map(|p| p.style.clone()).collect();
        self.edit_runs = story.paragraphs.iter().map(|p| p.runs.clone()).collect();
        self.edit_runs_detached = false;
        self.edit_spoken = story.paragraphs.iter().map(|p| p.spoken_only).collect();
        self.edit_web_hidden = story.paragraphs.iter().map(|p| p.web_hidden).collect();
        self.editing = Some((pi, fi));
        self.selected_frame = Some(fi);
        self.edit_done = false;
        self.reset_editor_transient();
        // T1: entering a thread at a later box starts the
        // caret at that box's first line, the slice the
        // session shows (see refresh_editor_layout).
        self.edit_caret_to_slice = self.doc.frames_of_story(&sid).first() != Some(&(pi, fi));
        self.focus_edit = true;
    }

    /// Write the edit buffer back into the story and leave
    /// editing. The gesture closes, so the whole typing session
    /// is one Undo step.
    pub(crate) fn commit_edit(&mut self) {
        let Some((pi, fi)) = self.editing.take() else {
            return;
        };
        self.reset_editor_transient();
        self.edit_start_text.clear();
        self.sync_edit_styles();
        self.sync_edit_spoken();
        self.sync_edit_web_hidden();
        let Some(frame) = self.doc.panels[pi].frames.get(fi) else {
            self.edit_start = None;
            return;
        };
        let Some(sid) = frame.story_id.clone() else {
            self.edit_start = None;
            return;
        };
        let texts: Vec<String> = self.edit_text.split('\n').map(|t| t.to_string()).collect();
        // The session's runs go back with the words (G1b).
        // Detached runs (the fallback editor changed the
        // words out from under them) or a count that no
        // longer matches the buffer fall back to plain runs
        // for the affected paragraphs: plain words, never
        // wrong faces on the wrong words.
        let runs_trustworthy = !self.edit_runs_detached && self.edit_runs.len() == texts.len();
        let paragraphs: Vec<ob_model::Paragraph> = texts
            .iter()
            .enumerate()
            .map(|(i, t)| {
                let style = self
                    .edit_styles
                    .get(i)
                    .cloned()
                    .unwrap_or_else(|| "body".to_string());
                let runs_match = runs_trustworthy
                    && runsedit::runs_len(&self.edit_runs[i]) == t.len()
                    && self.edit_runs[i]
                        .iter()
                        .map(|r| r.text.as_str())
                        .collect::<String>()
                        == *t;
                let mut para = if runs_match {
                    ob_model::Paragraph::from_runs(&style, self.edit_runs[i].clone())
                } else {
                    ob_model::Paragraph::plain(&style, t)
                };
                // The spoken-only flag rides with the
                // paragraph, like its style (both
                // constructors start it printable).
                para.spoken_only = self.edit_spoken.get(i).copied().unwrap_or(false);
                // The not-for-web flag rides the same way.
                para.web_hidden = self.edit_web_hidden.get(i).copied().unwrap_or(false);
                para
            })
            .collect();
        self.edit_runs.clear();
        self.edit_runs_detached = false;
        self.edit_spoken.clear();
        self.edit_web_hidden.clear();
        if let Some(story) = self.doc.stories.iter_mut().find(|st| st.id == sid) {
            story.paragraphs = paragraphs;
        }
        self.status = "Text updated.".into();
        // If the edited story is the announcements story, the
        // step box must show the same words (the box is the
        // other writer; it refreshes from here and stays clean).
        if weekly_story_id(&self.doc).as_deref() == Some(sid.as_str()) {
            self.announcements = weekly_body_text(&self.doc);
            self.announcements_dirty = false;
        }
        if let Some(start) = self.edit_start.take() {
            if start != snapshot(&self.doc) {
                self.record_undo(start);
            }
        }
    }

    /// Build (or reuse) the shaped editor geometry for the
    /// current session buffer (E1). The cache key covers the
    /// document snapshot, the buffer, the per-paragraph
    /// styles, and the thread's frame sizes, so any change
    /// rebuilds. Paragraphs are dealt through the story's
    /// boxes with the thread walk's arithmetic (T1), so
    /// editing shows the entered box's slice of exactly
    /// what the idle canvas and the PDF show.
    pub(crate) fn refresh_editor_layout(&mut self) {
        let Some((pi, fi)) = self.editing else {
            return;
        };
        let Some(frame) = self
            .doc
            .panels
            .get(pi)
            .and_then(|p| p.frames.get(fi))
            .map(|f| f.rect)
        else {
            return;
        };
        // The entered box's thread (T1): every box the
        // edited story flows through, in order, and the
        // entered box's place among them. A story in one
        // box threads through that box alone.
        let story_frames: Vec<(usize, usize)> = self
            .doc
            .panels
            .get(pi)
            .and_then(|p| p.frames.get(fi))
            .and_then(|f| f.story_id.as_ref())
            .map(|sid| self.doc.frames_of_story(sid))
            .unwrap_or_default();
        let (thread_rects, entered_pos): (Vec<ob_model::Rect>, usize) = if story_frames.is_empty() {
            (vec![frame], 0)
        } else {
            let pos = story_frames
                .iter()
                .position(|&x| x == (pi, fi))
                .unwrap_or(0);
            let rects = story_frames
                .iter()
                .map(|&(p2, f2)| self.doc.panels[p2].frames[f2].rect)
                .collect();
            (rects, pos)
        };
        // The runs sign the cache key too (G1b): bolding a
        // word changes no text and no style, but the layout
        // (faces, widths) must rebuild.
        let runs_sig: String = self
            .edit_runs
            .iter()
            .flat_map(|para| {
                para.iter().map(|r| {
                    format!(
                        "{}:{}:{};",
                        r.text,
                        r.bold.map(|b| b as u8).unwrap_or(2),
                        r.italic.map(|b| b as u8).unwrap_or(2)
                    )
                })
            })
            .collect();
        // The thread's boxes sign the key too (T1): moving
        // or sizing any box of the thread changes where the
        // entered box's slice begins.
        let rects_sig: String = thread_rects
            .iter()
            .map(|r| format!("{},{},{},{};", r.x, r.y, r.w, r.h))
            .collect();
        let key = format!(
            "{}|{}|{}|{}|{}|{}x{}|{}",
            self.frame_snapshot,
            self.edit_salt,
            self.edit_text,
            self.edit_styles.join("\u{1}"),
            runs_sig,
            frame.w,
            frame.h,
            rects_sig
        );
        if self
            .edit_layout
            .as_ref()
            .map(|l| l.key == key)
            .unwrap_or(false)
        {
            return;
        }
        let doc = self.doc.clone();
        let text = self.edit_text.clone();
        let edit_runs = self.edit_runs.clone();
        let styles: Vec<ob_model::ParagraphStyle> = (0..text.split('\n').count())
            .map(|i| {
                style_or_body(
                    &doc,
                    self.edit_styles
                        .get(i)
                        .map(|s| s.as_str())
                        .unwrap_or("body"),
                )
            })
            .collect();
        let layout_parts = {
            let shaper = self.shaper();
            let mut flags: Vec<bool> = Vec::new();
            let map = editmap::BufferMap::build(&text, &doc, |pi, resolved| {
                if resolved.contains('\t') {
                    let st = &styles[pi.min(styles.len() - 1)];
                    let spec = ob_text::shaping::FontSpec {
                        family: st.font_family.clone(),
                        bold: st.bold,
                        italic: st.italic,
                    };
                    flags.push(true);
                    Some(shaper.expand_order_line(resolved, frame.w, &spec, st.font_size))
                } else {
                    flags.push(false);
                    None
                }
            });
            let mut paras: Vec<EditorPara> = Vec::new();
            // The thread walk (T1): the story's lines are
            // dealt through its boxes in order, every layout
            // at the ENTERED box's width (the session's
            // documented simplification: the idle canvas and
            // print use each box's own width, so a thread of
            // mixed-width boxes can differ by a line here).
            // Each paragraph records how many of its lines
            // sit in earlier boxes (skip), how many the
            // entered box shows (drawn), and where the first
            // shown line sits (top). A story in one box walks
            // exactly as the session always stacked.
            let mut wf = 0usize;
            let mut wy = thread_rects[0].y;
            let mut after_y = frame.y + frame.h;
            let mut any_text = false;
            let mut any_clusters = false;
            for i in 0..map.para_count() {
                let st = styles[i.min(styles.len() - 1)].clone();
                let spec = ob_text::shaping::FontSpec {
                    family: st.font_family.clone(),
                    bold: st.bold,
                    italic: st.italic,
                };
                let dr = map.para_display_range(i);
                let dtext = &map.display_text[dr];
                if !dtext.trim().is_empty() {
                    any_text = true;
                }
                // The paragraph's face spans (G1b): resolve
                // the session runs with the fields, fold the
                // style in. If the runs and the map disagree
                // about the words (they cannot, by the session
                // invariants), fall back to the style's face
                // for the whole paragraph rather than paint
                // faces on the wrong words.
                let rr = map.para_resolved_range(i);
                let resolved_para = &map.resolved_text[rr];
                let res_runs = edit_runs.get(i).cloned().unwrap_or_default();
                let res_text: String = res_runs.iter().map(|r| r.text.as_str()).collect();
                let eff_spans: Vec<ob_text::shaping::RunSpan> = if res_text == resolved_para {
                    effective_spans(&res_runs, &st)
                } else {
                    vec![ob_text::shaping::RunSpan {
                        range: 0..resolved_para.len(),
                        bold: st.bold,
                        italic: st.italic,
                    }]
                };
                let layout = if flags.get(i) == Some(&true) {
                    let disp =
                        ob_text::text_layout::leader_run_spans(resolved_para, &eff_spans, dtext);
                    shaper.layout_display_line_runs(ob_text::text_layout::SpannedLayoutRequest {
                        text: dtext,
                        spans: &disp,
                        spec: &spec,
                        size: st.font_size,
                        leading_rel: 1.3,
                        width: frame.w,
                        align: st.align,
                    })
                } else {
                    shaper.layout_paragraph_runs(ob_text::text_layout::SpannedLayoutRequest {
                        text: dtext,
                        spans: &eff_spans,
                        spec: &spec,
                        size: st.font_size,
                        leading_rel: 1.3,
                        width: frame.w,
                        align: st.align,
                    })
                };
                if layout.lines().iter().any(|l| !l.clusters.is_empty()) {
                    any_clusters = true;
                }
                let line_h = st.font_size * 13 / 10;
                let total = layout.lines().len();
                let mut skip = 0usize;
                let mut drawn = 0usize;
                let mut top: Option<ob_model::Mpt> = None;
                let mut touched_later = wf >= entered_pos;
                let mut remaining = total;
                while remaining > 0 && wf < thread_rects.len() {
                    let fr = thread_rects[wf];
                    if wf >= entered_pos {
                        touched_later = true;
                    }
                    let start_y = wy;
                    let mut fit = 0usize;
                    while fit < remaining && wy + line_h <= fr.y + fr.h {
                        wy += line_h;
                        fit += 1;
                    }
                    if wf < entered_pos {
                        skip += fit;
                    } else if wf == entered_pos {
                        drawn = fit;
                        if fit > 0 {
                            top = Some(start_y);
                        }
                    }
                    remaining -= fit;
                    if remaining == 0 {
                        wy += st.space_after;
                    } else {
                        wf += 1;
                        if wf < thread_rects.len() {
                            wy = thread_rects[wf].y;
                        }
                    }
                }
                let top = match top {
                    Some(t) => t,
                    None if touched_later => {
                        // Nothing of this paragraph shows in
                        // the entered box: it starts later in
                        // the thread (or fits nowhere). Its
                        // top stacks virtually below the box,
                        // as overflow paragraphs always have.
                        let t = after_y;
                        after_y += total as ob_model::Mpt * line_h + st.space_after;
                        t
                    }
                    None => frame.y,
                };
                paras.push(EditorPara {
                    layout,
                    top,
                    line_h,
                    skip,
                    drawn,
                    size: st.font_size as f32 / 1000.0,
                    font_family: st.font_family.clone(),
                    bold: st.bold,
                    italic: st.italic,
                    align: st.align,
                });
            }
            // Where the entered box's slice begins, in
            // display space: the first drawn line's start.
            let slice_start = paras
                .iter()
                .enumerate()
                .find(|(_, p)| p.drawn > 0)
                .map(|(i, p)| {
                    map.para_display_range(i).start
                        + p.layout
                            .lines()
                            .get(p.skip)
                            .map(|l| l.range.start)
                            .unwrap_or(0)
                });
            (map, paras, slice_start, any_text && !any_clusters)
        };
        let (map, paras, slice_start, failed) = layout_parts;
        self.edit_layout = Some(EditorLayout {
            key,
            map,
            paras,
            slice_start,
        });
        self.edit_layout_failed = failed;
        // T1: a session opened on a later box of a thread
        // starts its caret at that box's first line.
        if self.edit_caret_to_slice {
            self.edit_caret_to_slice = false;
            if let Some(start) = self.edit_layout.as_ref().and_then(|l| l.slice_start) {
                self.edit_caret = start;
                self.edit_anchor = start;
                self.edit_cursor_para = self
                    .edit_layout
                    .as_ref()
                    .map(|l| l.map.para_at_display(start))
                    .unwrap_or(0);
            }
        }
    }

    /// The current selection in display space.
    pub(crate) fn editor_selection(&self) -> std::ops::Range<usize> {
        self.edit_anchor.min(self.edit_caret)..self.edit_anchor.max(self.edit_caret)
    }

    /// Apply one display-space edit to the session buffer
    /// through the byte map, park the caret after the
    /// inserted text, and resync the paragraph style list
    /// (the same bookkeeping commit_edit relies on).
    pub(crate) fn apply_display_edit(&mut self, range: std::ops::Range<usize>, replacement: &str) {
        let Some((raw_range, rep)) = self
            .edit_layout
            .as_ref()
            .map(|l| l.map.translate_edit(range, replacement))
        else {
            return;
        };
        // The same edit lands on the runs (G1b): typed text
        // inherits the face in force at its start, runs
        // outside the edited range keep their flags exactly.
        if !self.edit_runs_detached {
            runsedit::apply_edit(&mut self.edit_runs, raw_range.clone(), &rep);
        }
        let caret_raw = raw_range.start + rep.len();
        let mut text = std::mem::take(&mut self.edit_text);
        text.replace_range(raw_range, &rep);
        self.edit_text = text;
        self.edit_chip_armed = None;
        self.edit_desired_x = None;
        self.edit_layout = None;
        self.refresh_editor_layout();
        let d = self
            .edit_layout
            .as_ref()
            .map(|l| l.map.raw_to_display(caret_raw))
            .unwrap_or(caret_raw);
        self.edit_caret = d;
        self.edit_anchor = d;
        if let Some(l) = &self.edit_layout {
            self.edit_cursor_para = l.map.para_at_display(d);
        }
        self.sync_edit_styles();
        self.sync_edit_spoken();
        self.sync_edit_web_hidden();
    }

    /// Type or paste text at the selection, replacing it.
    pub(crate) fn editor_insert_text(&mut self, s: &str) {
        if s.is_empty() {
            return;
        }
        let sel = self.editor_selection();
        self.apply_display_edit(sel, s);
    }

    /// Paste into the canvas editor (G1b). When the pasted
    /// text carries markdown bold/italic markers (the shape
    /// notes and web text arrive in, and the shape a chat or
    /// a notes app copies), the markers become runs on the
    /// way in: the stripped words insert through the normal
    /// funnel, then the marks land as run flags over the
    /// inserted range. Unmarked pastes are the plain insert
    /// they always were. The whole paste is one session
    /// edit, so still one undo step.
    pub(crate) fn editor_paste(&mut self, t: &str) {
        if self.edit_runs_detached || self.edit_layout.is_none() {
            self.editor_insert_text(t);
            return;
        }
        let parsed: Vec<Vec<ob_model::Run>> =
            t.split('\n').map(ob_model::markdown::inline_runs).collect();
        let stripped: Vec<String> = parsed
            .iter()
            .map(|runs| runs.iter().map(|r| r.text.as_str()).collect())
            .collect();
        let any_marks = stripped
            .iter()
            .zip(t.split('\n'))
            .any(|(s, orig)| s.as_str() != orig);
        if !any_marks {
            self.editor_insert_text(t);
            return;
        }
        // The insertion point in raw space, before the edit
        // moves anything (selections are built from snapped
        // carets, so the start is a true boundary).
        let sel = self.editor_selection();
        let raw_start = self
            .edit_layout
            .as_ref()
            .map(|l| l.map.resolved_to_raw(l.map.display_to_resolved(sel.start)));
        let joined = stripped.join("\n");
        self.editor_insert_text(&joined);
        let Some(raw_start) = raw_start else {
            return;
        };
        let mut line_start = raw_start;
        for runs in &parsed {
            let mut at = line_start;
            for r in runs {
                let end = at + r.text.len();
                if r.bold == Some(true) {
                    runsedit::set_flag_range(
                        &mut self.edit_runs,
                        at..end,
                        runsedit::Flag::Bold,
                        true,
                    );
                }
                if r.italic == Some(true) {
                    runsedit::set_flag_range(
                        &mut self.edit_runs,
                        at..end,
                        runsedit::Flag::Italic,
                        true,
                    );
                }
                at = end;
            }
            line_start = at + 1; // the '\n' between pasted lines
        }
        self.edit_layout = None;
        self.refresh_editor_layout();
    }

    /// Set the caret to a display offset, optionally
    /// extending the selection. `forward` picks which end of
    /// an atomic (a chip or a dot leader) a stop inside it
    /// snaps to, so one keypress crosses a chip in one step.
    pub(crate) fn editor_set_caret(&mut self, target: usize, extend: bool, forward: bool) {
        let Some(layout) = &self.edit_layout else {
            return;
        };
        let target = layout.map.snap_display_caret_dir(target, forward);
        self.edit_caret = target;
        if !extend {
            self.edit_anchor = target;
        }
        self.edit_cursor_para = layout.map.para_at_display(target);
        self.edit_chip_armed = None;
    }

    /// Hit-test a screen point against the editor geometry:
    /// the display byte offset under the pointer. x and y are
    /// converted into the paragraph's layout space (points,
    /// paragraph top-left origin) exactly as the layouts were
    /// built.
    pub(crate) fn editor_hit(
        &self,
        pos: egui::Pos2,
        origin: egui::Pos2,
        scale: f32,
        frame: &ob_model::Rect,
    ) -> Option<usize> {
        let layout = self.edit_layout.as_ref()?;
        if layout.paras.is_empty() {
            return None;
        }
        let px = (pos.x - origin.x) / scale;
        let py = (pos.y - origin.y) / scale;
        let lx = px - frame.x as f32 / 1000.0;
        // Only paragraphs with lines in this box can be hit
        // (T1): a threaded session shows a slice, and clicks
        // land in the slice. The fallback is the last
        // paragraph that shows lines here.
        let mut chosen = layout
            .paras
            .iter()
            .rposition(|p| p.drawn > 0)
            .unwrap_or(layout.paras.len() - 1);
        for (i, para) in layout.paras.iter().enumerate() {
            if para.drawn == 0 {
                continue;
            }
            let top = para.top as f32 / 1000.0;
            let bottom = top + para.drawn as f32 * para.layout.line_height();
            if py <= bottom {
                chosen = i;
                break;
            }
        }
        let para = &layout.paras[chosen];
        let top = para.top as f32 / 1000.0;
        // The layout's y counts all its lines; the slice
        // starts `skip` lines in.
        let local = para
            .layout
            .byte_offset_at(lx, py - top + para.skip as f32 * para.layout.line_height());
        Some(layout.map.para_display_range(chosen).start + local)
    }

    /// Place the caret from a pointer position, snapped to
    /// the nearest atomic boundary (a click inside a chip
    /// lands on its nearer end). With `extend`, the anchor
    /// stays and only the caret moves (drag selection).
    pub(crate) fn editor_click_caret(&mut self, d: usize, extend: bool) {
        let Some(layout) = &self.edit_layout else {
            return;
        };
        let d = layout.map.snap_display_caret(d);
        self.edit_caret = d;
        if !extend {
            self.edit_anchor = d;
        }
        self.edit_cursor_para = layout.map.para_at_display(d);
        self.edit_chip_armed = None;
        self.edit_desired_x = None;
    }

    /// Select the word under a display offset (double-click),
    /// grown over any chip it touches.
    pub(crate) fn editor_select_word(&mut self, d: usize) {
        let (range, para) = {
            let Some(layout) = &self.edit_layout else {
                return;
            };
            let para = layout.map.para_at_display(d);
            let start = layout.map.para_display_range(para).start;
            let wr = layout.paras[para].layout.word_range_at(d - start);
            (
                layout
                    .map
                    .expand_display_range(start + wr.start..start + wr.end),
                para,
            )
        };
        self.edit_anchor = range.start;
        self.edit_caret = range.end;
        self.edit_cursor_para = para;
        self.edit_chip_armed = None;
        self.edit_desired_x = None;
    }

    /// The caret's paragraph index and its byte offset
    /// within that paragraph's display text.
    pub(crate) fn editor_caret_local(&self) -> Option<(usize, usize)> {
        let layout = self.edit_layout.as_ref()?;
        let para = layout.map.para_at_display(self.edit_caret);
        let start = layout.map.para_display_range(para).start;
        Some((para, self.edit_caret - start))
    }

    /// Step one cluster stop left or right, crossing into
    /// the next paragraph at its edge. A plain arrow with a
    /// selection collapses it, as editors do.
    pub(crate) fn editor_step(&mut self, forward: bool, extend: bool) {
        if !extend {
            let sel = self.editor_selection();
            if sel.start != sel.end {
                let t = if forward { sel.end } else { sel.start };
                self.editor_set_caret(t, false, forward);
                return;
            }
        }
        let Some((para, local)) = self.editor_caret_local() else {
            return;
        };
        let target = {
            let layout = self.edit_layout.as_ref().unwrap();
            let stops = cluster_stops(&layout.paras[para].layout);
            let range = layout.map.para_display_range(para);
            if forward {
                match stops.iter().find(|&&s| s > local) {
                    Some(&s) => range.start + s,
                    None => {
                        if para + 1 < layout.map.para_count() {
                            range.end + 1
                        } else {
                            range.end
                        }
                    }
                }
            } else {
                match stops.iter().rev().find(|&&s| s < local) {
                    Some(&s) => range.start + s,
                    None => {
                        if para > 0 {
                            range.start - 1
                        } else {
                            range.start
                        }
                    }
                }
            }
        };
        self.edit_desired_x = None;
        self.editor_set_caret(target, extend, forward);
    }

    /// Move the caret one visual line up or down, keeping
    /// the x it started from until a horizontal move resets
    /// it. Lines flow across paragraphs in order.
    pub(crate) fn editor_step_line(&mut self, down: bool, extend: bool) {
        let (target, keep_x) = {
            let Some(layout) = self.edit_layout.as_ref() else {
                return;
            };
            let para = layout.map.para_at_display(self.edit_caret);
            let start = layout.map.para_display_range(para).start;
            let local = self.edit_caret - start;
            let pl = &layout.paras[para];
            let (x0, y0, _) = pl.layout.caret_rect(local);
            let line_h = pl.layout.line_height();
            let cur_line = if line_h > 0.0 {
                (y0 / line_h).round() as usize
            } else {
                0
            };
            let x = self.edit_desired_x.unwrap_or(x0);
            let spot = if down {
                if cur_line + 1 < pl.layout.lines().len() {
                    Some((para, cur_line + 1))
                } else if para + 1 < layout.paras.len() {
                    Some((para + 1, 0))
                } else {
                    None
                }
            } else if cur_line > 0 {
                Some((para, cur_line - 1))
            } else if para > 0 {
                Some((
                    para - 1,
                    layout.paras[para - 1]
                        .layout
                        .lines()
                        .len()
                        .saturating_sub(1),
                ))
            } else {
                None
            };
            match spot {
                Some((tp, tl)) => {
                    let tpl = &layout.paras[tp];
                    let mid_y = (tl as f32 + 0.5) * tpl.layout.line_height();
                    let tlocal = tpl.layout.byte_offset_at(x, mid_y);
                    (layout.map.para_display_range(tp).start + tlocal, Some(x))
                }
                None => (
                    if down {
                        layout.map.display_text.len()
                    } else {
                        0
                    },
                    None,
                ),
            }
        };
        self.editor_set_caret(target, extend, down);
        self.edit_desired_x = keep_x;
    }

    /// Jump to the start or end of the caret's visual line.
    pub(crate) fn editor_line_edge(&mut self, end: bool, extend: bool) {
        let Some((para, local)) = self.editor_caret_local() else {
            return;
        };
        let target = {
            let layout = self.edit_layout.as_ref().unwrap();
            let pl = &layout.paras[para].layout;
            let (_, y0, _) = pl.caret_rect(local);
            let line_h = pl.line_height();
            let cur = if line_h > 0.0 {
                (y0 / line_h).round() as usize
            } else {
                0
            };
            let Some(line) = pl.lines().get(cur) else {
                return;
            };
            let t = if end {
                line.range.end
            } else {
                line.range.start
            };
            layout.map.para_display_range(para).start + t
        };
        self.edit_desired_x = None;
        self.editor_set_caret(target, extend, end);
    }

    /// Jump by whole words over the display text.
    pub(crate) fn editor_step_word(&mut self, forward: bool, extend: bool) {
        let target = {
            let Some(layout) = self.edit_layout.as_ref() else {
                return;
            };
            word_stop(&layout.map.display_text, self.edit_caret, forward)
        };
        self.edit_desired_x = None;
        self.editor_set_caret(target, extend, forward);
    }

    /// Backspace (backward) or Delete, with chip semantics:
    /// the first press next to a chip or leader selects it
    /// whole; the second press (or any press with a
    /// selection) deletes the whole selection. A plain press
    /// removes one cluster stop, or the paragraph break at a
    /// paragraph edge.
    pub(crate) fn editor_delete(&mut self, backward: bool) {
        let sel = self.editor_selection();
        if sel.start != sel.end {
            self.apply_display_edit(sel, "");
            return;
        }
        let caret = self.edit_caret;
        let arm = {
            let Some(layout) = self.edit_layout.as_ref() else {
                return;
            };
            if backward {
                layout.map.atomic_ending_at_display(caret)
            } else {
                layout.map.atomic_starting_at_display(caret)
            }
        };
        if let Some(a) = arm {
            let para = self
                .edit_layout
                .as_ref()
                .map(|l| l.map.para_at_display(a.start));
            self.edit_anchor = a.start;
            self.edit_caret = a.end;
            self.edit_chip_armed = Some(a);
            self.edit_desired_x = None;
            if let Some(p) = para {
                self.edit_cursor_para = p;
            }
            return;
        }
        let range = {
            let Some((para, local)) = self.editor_caret_local() else {
                return;
            };
            let layout = self.edit_layout.as_ref().unwrap();
            let range = layout.map.para_display_range(para);
            let stops = cluster_stops(&layout.paras[para].layout);
            if backward {
                if caret == 0 {
                    return;
                }
                let from = if local == 0 {
                    caret - 1
                } else {
                    let prev = stops
                        .iter()
                        .rev()
                        .find(|&&s| s < local)
                        .copied()
                        .unwrap_or(0);
                    range.start + prev
                };
                from..caret
            } else {
                if caret >= layout.map.display_text.len() {
                    return;
                }
                let to = if local >= range.end - range.start {
                    caret + 1
                } else {
                    let next = stops
                        .iter()
                        .find(|&&s| s > local)
                        .copied()
                        .unwrap_or(range.end - range.start);
                    range.start + next
                };
                caret..to
            }
        };
        self.apply_display_edit(range, "");
    }

    /// Feed one input event to the shaped editor (E1). Runs
    /// only while the editor's focus sink holds focus. Text
    /// and IME commits go through the same display-space
    /// edit path as the mouse, so chips stay atomic.
    pub(crate) fn editor_event(&mut self, ctx: &egui::Context, event: &egui::Event) {
        match event {
            egui::Event::Text(t) => {
                if self.ime_preedit.is_none() {
                    let clean: String = t.chars().filter(|c| !c.is_control()).collect();
                    self.editor_insert_text(&clean);
                }
            }
            egui::Event::Paste(t) => {
                let t = t.replace("\r\n", "\n").replace('\r', "\n");
                self.editor_paste(&t);
            }
            egui::Event::Copy => {
                let sel = self.editor_selection();
                if sel.start != sel.end {
                    if let Some(l) = &self.edit_layout {
                        ctx.copy_text(l.map.display_text[sel.clone()].to_string());
                    }
                }
            }
            egui::Event::Cut => {
                let sel = self.editor_selection();
                if sel.start != sel.end {
                    if let Some(l) = &self.edit_layout {
                        ctx.copy_text(l.map.display_text[sel.clone()].to_string());
                    }
                    self.apply_display_edit(sel, "");
                }
            }
            egui::Event::Ime(ime) => match ime {
                egui::ImeEvent::Enabled | egui::ImeEvent::Disabled => {
                    self.ime_preedit = None;
                }
                egui::ImeEvent::Preedit(s) => {
                    if s.is_empty() {
                        self.ime_preedit = None;
                    } else {
                        let at = self.editor_selection().start;
                        self.ime_preedit = Some((at, s.clone()));
                    }
                }
                egui::ImeEvent::Commit(s) => {
                    self.ime_preedit = None;
                    let s = s.clone();
                    self.editor_insert_text(&s);
                }
            },
            egui::Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } => {
                self.editor_key(*key, *modifiers);
            }
            _ => {}
        }
    }

    /// The session paragraphs' style faces, in runsedit's
    /// (bold, italic) pairs.
    pub(crate) fn editor_styles_vec(&self) -> Vec<(bool, bool)> {
        (0..self.edit_runs.len())
            .map(|i| {
                let st = style_or_body(
                    &self.doc,
                    self.edit_styles
                        .get(i)
                        .map(|s| s.as_str())
                        .unwrap_or("body"),
                );
                (st.bold, st.italic)
            })
            .collect()
    }

    /// The current selection as RAW ranges with the chips cut
    /// out (G1b): a toggle lands around a shaded field word,
    /// never inside it, exactly as typing deletes around it.
    /// Empty when there is no selection or no layout.
    pub(crate) fn editor_selection_raw_ranges(&self) -> Vec<std::ops::Range<usize>> {
        let Some(layout) = self.edit_layout.as_ref() else {
            return Vec::new();
        };
        let map = &layout.map;
        let sel = self.editor_selection();
        if sel.is_empty() {
            return Vec::new();
        }
        let mut out: Vec<std::ops::Range<usize>> = Vec::new();
        let first = map.para_at_display(sel.start);
        let last = map.para_at_display(sel.end.min(map.display_text.len()));
        for p in first..=last {
            let dr = map.para_display_range(p);
            let mut s = sel.start.max(dr.start);
            let e = sel.end.min(dr.end);
            if s >= e {
                continue;
            }
            // Cut the chips out of [s, e), left to right.
            let mut chips: Vec<std::ops::Range<usize>> = map
                .chips
                .iter()
                .map(|c| c.display.clone())
                .filter(|c| c.start < e && c.end > s)
                .collect();
            chips.sort_by_key(|c| c.start);
            for chip in chips {
                if chip.start > s {
                    push_raw_piece(map, s, chip.start.min(e), &mut out);
                }
                s = s.max(chip.end);
                if s >= e {
                    break;
                }
            }
            if s < e {
                push_raw_piece(map, s, e, &mut out);
            }
        }
        out
    }

    /// Bold or italic over the selection (G1b): if every
    /// selected character already carries the face, take it
    /// off; otherwise put it on (the word-processor rule).
    /// Runs only: the words do not change, so the caret and
    /// selection stay where they are; the layout rebuilds
    /// because faces change widths.
    pub(crate) fn editor_toggle_flag(&mut self, flag: runsedit::Flag) {
        if self.edit_runs_detached || self.editing.is_none() {
            return;
        }
        let pieces = self.editor_selection_raw_ranges();
        if pieces.is_empty() {
            return;
        }
        let styles = self.editor_styles_vec();
        let all_on = pieces
            .iter()
            .all(|r| runsedit::range_all_effective(&self.edit_runs, &styles, r.clone(), flag));
        for r in pieces {
            runsedit::set_flag_range(&mut self.edit_runs, r, flag, !all_on);
        }
        self.edit_layout = None;
        self.refresh_editor_layout();
    }

    /// (bold, italic) state for the toolbar buttons: over a
    /// selection, whether every selected character carries
    /// the face; at a bare caret, the face typing would take.
    pub(crate) fn editor_flag_states(&self) -> (bool, bool) {
        if self.edit_runs_detached || self.editing.is_none() {
            return (false, false);
        }
        let styles = self.editor_styles_vec();
        let pieces = self.editor_selection_raw_ranges();
        if !pieces.is_empty() {
            let b = pieces.iter().all(|r| {
                runsedit::range_all_effective(
                    &self.edit_runs,
                    &styles,
                    r.clone(),
                    runsedit::Flag::Bold,
                )
            });
            let i = pieces.iter().all(|r| {
                runsedit::range_all_effective(
                    &self.edit_runs,
                    &styles,
                    r.clone(),
                    runsedit::Flag::Italic,
                )
            });
            return (b, i);
        }
        let Some(layout) = self.edit_layout.as_ref() else {
            return (false, false);
        };
        let caret_raw = layout
            .map
            .resolved_to_raw(layout.map.display_to_resolved(self.edit_caret));
        runsedit::effective_flags_at(&self.edit_runs, &styles, caret_raw)
    }

    /// One keypress inside the shaped editor. Escape is not
    /// handled here: the canvas still cancels the session on
    /// Escape, exactly as before. Ctrl/Cmd+Z does nothing in
    /// a session on purpose: the session is one undo step
    /// from its begin snapshot.
    pub(crate) fn editor_key(&mut self, key: egui::Key, modifiers: egui::Modifiers) {
        use egui::Key;
        if self.ime_preedit.is_some() {
            return; // the composition owns the keyboard
        }
        // Bold / Italic (G1b). The keymap has no bold/italic
        // bindings (its presets carry editing and file keys
        // only), so these read the modifiers directly, like
        // the session's other direct keys: Ctrl or Cmd + B/I.
        if modifiers.command {
            match key {
                Key::B => {
                    self.editor_toggle_flag(runsedit::Flag::Bold);
                    return;
                }
                Key::I => {
                    self.editor_toggle_flag(runsedit::Flag::Italic);
                    return;
                }
                _ => {}
            }
        }
        let extend = modifiers.shift;
        let word = modifiers.ctrl || modifiers.alt;
        match key {
            Key::Enter => self.editor_insert_text("\n"),
            Key::Backspace => self.editor_delete(true),
            Key::Delete => self.editor_delete(false),
            Key::ArrowLeft => {
                if word {
                    self.editor_step_word(false, extend);
                } else if modifiers.mac_cmd {
                    self.editor_line_edge(false, extend);
                } else {
                    self.editor_step(false, extend);
                }
            }
            Key::ArrowRight => {
                if word {
                    self.editor_step_word(true, extend);
                } else if modifiers.mac_cmd {
                    self.editor_line_edge(true, extend);
                } else {
                    self.editor_step(true, extend);
                }
            }
            Key::ArrowUp => self.editor_step_line(false, extend),
            Key::ArrowDown => self.editor_step_line(true, extend),
            Key::Home => {
                if modifiers.command {
                    self.editor_set_caret(0, extend, false);
                } else {
                    self.editor_line_edge(false, extend);
                }
            }
            Key::End => {
                if modifiers.command {
                    let len = self
                        .edit_layout
                        .as_ref()
                        .map(|l| l.map.display_text.len())
                        .unwrap_or(0);
                    self.editor_set_caret(len, extend, true);
                } else {
                    self.editor_line_edge(true, extend);
                }
            }
            Key::A if modifiers.command => {
                let len = self
                    .edit_layout
                    .as_ref()
                    .map(|l| l.map.display_text.len())
                    .unwrap_or(0);
                self.edit_anchor = 0;
                self.edit_caret = len;
                self.edit_chip_armed = None;
            }
            _ => {}
        }
    }

    /// Refresh the lines cache when the document changed:
    /// the thread layouts are computed once per document
    /// state, and every panel's lines are cut from them.
    pub(crate) fn refresh_lines_cache(&mut self) {
        let key = self.frame_snapshot.clone();
        let stale = self
            .lines_cache
            .as_ref()
            .map(|c| c.key != key)
            .unwrap_or(true);
        if stale {
            // Clone on a rebuild only: the shaper borrows the
            // app mutably while the threads are computed from
            // the doc.
            let doc = self.doc.clone();
            let shaper = self.shaper();
            let threads = ob_text::threading::layout_document_threads(shaper, &doc);
            self.lines_cache = Some(LinesCache {
                key,
                lines: std::collections::HashMap::new(),
                threads,
            });
        }
    }

    /// The document's thread layouts (T1): which lines of
    /// each story land in which box, and how many fit
    /// nowhere. Cached with the shaped lines.
    pub(crate) fn doc_threads(
        &mut self,
    ) -> std::collections::HashMap<String, ob_text::threading::StoryThread> {
        self.refresh_lines_cache();
        self.lines_cache
            .as_ref()
            .map(|c| c.threads.clone())
            .unwrap_or_default()
    }

    /// Shaped lines for one panel, cached per document state:
    /// the canvas and the print preview can never disagree.
    pub(crate) fn shaped_panel_lines(&mut self, idx: usize) -> Vec<PreviewLine> {
        self.refresh_lines_cache();
        if let Some(cache) = &self.lines_cache {
            if let Some(lines) = cache.lines.get(&idx) {
                return lines.clone();
            }
        }
        let lines = {
            let cache = self.lines_cache.as_ref().expect("cache refreshed");
            panel_lines_from_threads(&self.doc, idx, &self.doc.panels[idx], &cache.threads)
        };
        if let Some(cache) = &mut self.lines_cache {
            cache.lines.insert(idx, lines.clone());
        }
        lines
    }

    /// The Pages window: one panel at a time, with inch rulers,
    /// the tracing picture, frame outlines, and the panel's guides.
    /// Guides follow office habits: drag to move, right-click to
    /// type the exact place (docs/EDITOR.md).
    /// The "put a picture in the box" window (G2): one file
    /// door, plus the pictures this bulletin already carries
    /// that no box uses yet ("Pictures from the old
    /// bulletin", from a PDF rescue).
    pub(crate) fn picture_picker_window(&mut self, ctx: &egui::Context) {
        if !self.show_pic_picker {
            return;
        }
        let target = match self.selected_frame {
            Some(fi) if self.doc.panels.get(self.canvas_panel).is_some() => {
                Some((self.canvas_panel, fi))
            }
            _ => None,
        };
        let loose = unplaced_doc_pictures(&self.doc);
        let mut open = self.show_pic_picker;
        let mut close = false;
        let mut place: Option<String> = None;
        let mut pick_file = false;
        egui::Window::new("Put a picture in the box")
            .open(&mut open)
            .resizable(true)
            .default_width(460.0)
            .show(ctx, |ui| {
                match target {
                    None => {
                        ui.label(
                            "First click a box on the page in the Pages window, \
                             then choose its picture here.",
                        );
                    }
                    Some(_) => {
                        if ui.button("Pick a picture file...").clicked() {
                            pick_file = true;
                        }
                        ui.separator();
                        ui.label(egui::RichText::new("Pictures from the old bulletin").strong());
                        ui.label(
                            "Pictures this bulletin already carries that no box uses yet. \
                             Click one to put it in the box.",
                        );
                        if loose.is_empty() {
                            ui.label("No loose pictures from an old bulletin are waiting.");
                        }
                        egui::ScrollArea::vertical()
                            .id_salt("loose-pictures")
                            .max_height(280.0)
                            .show(ui, |ui| {
                                ui.horizontal_wrapped(|ui| {
                                    for asset in &loose {
                                        ui.vertical(|ui| {
                                            let bytes = self
                                                .doc
                                                .asset_data(&asset.sha256)
                                                .map(|b| b.to_vec());
                                            let mut clicked = false;
                                            match bytes {
                                                Some(bytes) => {
                                                    let key = format!("asset:{}", asset.sha256);
                                                    match self.picture_texture(
                                                        ui.ctx(),
                                                        &key,
                                                        &bytes,
                                                        160,
                                                    ) {
                                                        Some((tex, _, _)) => {
                                                            if ui
                                                                .add(egui::ImageButton::new(
                                                                    egui::load::SizedTexture::new(
                                                                        tex.id(),
                                                                        egui::vec2(72.0, 72.0),
                                                                    ),
                                                                ))
                                                                .clicked()
                                                            {
                                                                clicked = true;
                                                            }
                                                        }
                                                        None => {
                                                            ui.label("(could not be shown)");
                                                        }
                                                    }
                                                }
                                                None => {
                                                    ui.label("(missing)");
                                                }
                                            }
                                            let label = if asset.rights.source.is_empty() {
                                                "From the old bulletin".to_string()
                                            } else {
                                                asset.rights.source.clone()
                                            };
                                            if ui.small_button(label).clicked() || clicked {
                                                place = Some(asset.sha256.clone());
                                            }
                                        });
                                    }
                                });
                            });
                    }
                }
                ui.separator();
                if ui.button("Close").clicked() {
                    close = true;
                }
            });
        self.show_pic_picker = open && !close;
        if let (Some((idx, fi)), true) = (target, pick_file) {
            self.pick_picture_file(idx, fi);
            self.show_pic_picker = false;
        }
        if let (Some((idx, fi)), Some(sha)) = (target, place) {
            self.place_stored_picture(idx, fi, &sha);
            self.show_pic_picker = false;
        }
    }

    /// The clip art library window (G2, per
    /// docs/research/clipart-landscape.md section 4):
    /// collections on the left, a thumbnail grid with name
    /// search on the right, and the add-a-collection doors.
    pub(crate) fn clips_window(&mut self, ctx: &egui::Context) {
        if !self.show_clips {
            return;
        }
        self.ensure_starter_clips();
        let lib = ob_clips::ClipLibrary::open(ob_clips::default_library_dir()).ok();
        let collections: Vec<ob_clips::Collection> =
            lib.as_ref().map(|l| l.collections()).unwrap_or_default();
        if self.clip_collection.is_none() {
            self.clip_collection = collections.first().map(|c| c.name.clone());
        }
        let selected = collections
            .iter()
            .find(|c| Some(&c.name) == self.clip_collection.as_ref());
        let target = match self.selected_frame {
            Some(fi) if self.doc.panels.get(self.canvas_panel).is_some() => {
                Some((self.canvas_panel, fi))
            }
            _ => None,
        };
        let mut open = self.show_clips;
        let mut place_item: Option<(String, String, Option<ob_image::PictureFormat>, String)> =
            None;
        let mut add_folder = false;
        let mut add_zip = false;
        egui::Window::new("Clip art")
            .open(&mut open)
            .resizable(true)
            .default_width(640.0)
            .default_height(440.0)
            .show(ctx, |ui| {
                ui.horizontal_top(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width(170.0);
                        ui.label(egui::RichText::new("Collections").strong());
                        for c in &collections {
                            let label = format!("{} ({})", c.name, c.items.len());
                            if ui
                                .selectable_label(
                                    self.clip_collection.as_deref() == Some(c.name.as_str()),
                                    label,
                                )
                                .clicked()
                            {
                                self.clip_collection = Some(c.name.clone());
                            }
                        }
                        if collections.is_empty() {
                            ui.label("No collections yet. Add one below.");
                        }
                        ui.separator();
                        if ui.button("Add a collection from a folder...").clicked() {
                            add_folder = true;
                        }
                        if ui.button("Add a collection from a zip file...").clicked() {
                            add_zip = true;
                        }
                    });
                    ui.separator();
                    ui.vertical(|ui| {
                        ui.label("Find a clip by words in its name:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.clip_query)
                                .hint_text("try: dove, candle, cross"),
                        );
                        match selected {
                            None => {
                                ui.label("Pick a collection on the left.");
                            }
                            Some(coll) => {
                                if target.is_none() {
                                    ui.label(
                                        "Tip: click a box on the page in the Pages window first; \
                                         then a clip here goes straight into that box.",
                                    );
                                }
                                let hits = coll.matching(&self.clip_query);
                                egui::ScrollArea::vertical()
                                    .id_salt("clip-grid")
                                    .max_height(300.0)
                                    .show(ui, |ui| {
                                        ui.horizontal_wrapped(|ui| {
                                            for item in &hits {
                                                ui.vertical(|ui| {
                                                    let bytes =
                                                        std::fs::read(&item.path).ok();
                                                    let mut clicked = false;
                                                    match (bytes, item.format) {
                                                        (Some(bytes), Some(_)) => {
                                                            let key = format!(
                                                                "file:{}",
                                                                item.path.display()
                                                            );
                                                            match self.picture_texture(
                                                                ui.ctx(),
                                                                &key,
                                                                &bytes,
                                                                160,
                                                            ) {
                                                                Some((tex, _, _)) => {
                                                                    if ui
                                                                        .add(egui::ImageButton::new(
                                                                            egui::load::SizedTexture::new(
                                                                                tex.id(),
                                                                                egui::vec2(72.0, 72.0),
                                                                            ),
                                                                        ))
                                                                        .clicked()
                                                                    {
                                                                        clicked = true;
                                                                    }
                                                                    // The library's own
                                                                    // thumbnail cache,
                                                                    // written the first
                                                                    // time a clip shows.
                                                                    if let Some(l) = &lib {
                                                                        if let Some(tp) =
                                                                            l.thumb_path(&item.path)
                                                                        {
                                                                            if !tp.exists() {
                                                                                if let Some(dir) =
                                                                                    tp.parent()
                                                                                {
                                                                                    let _ = std::fs::create_dir_all(dir);
                                                                                }
                                                                                if let Ok(d) = ob_image::decode_fitted(&bytes, 192, 192) {
                                                                                    if let Ok(png) = ob_image::encode_png(&d) {
                                                                                        let _ = std::fs::write(&tp, png);
                                                                                    }
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                                None => {
                                                                    ui.label("(could not be shown)");
                                                                }
                                                            }
                                                        }
                                                        _ => {
                                                            ui.label("(could not be read)");
                                                        }
                                                    }
                                                    if ui.small_button(&item.file_name).clicked()
                                                        || clicked
                                                    {
                                                        place_item = Some((
                                                            item.path.display().to_string(),
                                                            item.file_name.clone(),
                                                            item.format,
                                                            coll.name.clone(),
                                                        ));
                                                    }
                                                });
                                            }
                                        });
                                        if hits.is_empty() {
                                            ui.label(
                                                "No clips in this collection match those words.",
                                            );
                                        }
                                    });
                            }
                        }
                    });
                });
                ui.separator();
                if let Some(status) = &self.clip_status {
                    ui.label(status.clone());
                }
                ui.label(
                    "Clips you add are copied into the library, and a copy goes into the bulletin, so the bulletin always travels complete.",
                );
            });
        self.show_clips = open;
        if add_folder {
            self.add_clips_from_folder();
        }
        if add_zip {
            self.add_clips_from_zip();
        }
        if let Some((path, file_name, format, collection)) = place_item {
            match target {
                None => {
                    self.clip_status =
                        Some("Click a box on the page first, then pick the clip.".into());
                }
                Some((idx, fi)) => match std::fs::read(&path) {
                    Ok(bytes) => {
                        let lib2 =
                            ob_clips::ClipLibrary::open(ob_clips::default_library_dir()).ok();
                        let coll = lib2.as_ref().and_then(|l| l.collection(&collection));
                        // A piece with a record of its own
                        // (the starter set's pieces, from the
                        // manifest) places with that record:
                        // its own license, author, and source
                        // page ride into the bulletin's rights
                        // record. Other pieces place with
                        // their collection's notes, as before.
                        let own = coll
                            .as_ref()
                            .and_then(|c| c.pieces.get(&file_name).cloned());
                        let license = own
                            .as_ref()
                            .map(|n| n.license_kind())
                            .or_else(|| {
                                coll.as_ref()
                                    .and_then(|c| c.notes.as_ref())
                                    .map(|n| n.license_kind())
                            })
                            .unwrap_or(ob_model::LicenseKind::Unknown);
                        let piece_creator =
                            own.as_ref().map(|n| n.creator.clone()).unwrap_or_default();
                        let piece_source_url = own.as_ref().map(|n| n.source.clone());
                        let media = format
                            .map(|f| f.media_type().to_string())
                            .unwrap_or_else(|| "application/octet-stream".into());
                        let title = file_name
                            .rsplit_once('.')
                            .map(|(stem, _)| stem.to_string())
                            .unwrap_or(file_name.clone());
                        let sha = self.doc.add_asset(
                            &bytes,
                            ob_model::AssetKind::Picture,
                            &media,
                            ob_model::RightsRecord {
                                source: collection.clone(),
                                source_url: piece_source_url,
                                license,
                                title,
                                creator: piece_creator,
                                ..Default::default()
                            },
                        );
                        self.place_stored_picture(idx, fi, &sha);
                        self.clip_status =
                            Some(format!("\"{file_name}\" is in the box on the page."));
                    }
                    Err(e) => {
                        self.clip_status = Some(format!("Could not read that clip: {e}"));
                    }
                },
            }
        }
    }

    /// "Add a collection" from a folder of pictures.
    pub(crate) fn add_clips_from_folder(&mut self) {
        let Some(folder) = rfd::FileDialog::new().pick_folder() else {
            return;
        };
        let Ok(lib) = ob_clips::ClipLibrary::open(ob_clips::default_library_dir()) else {
            self.fail("The clip art library folder could not be opened.".into());
            return;
        };
        let converter = wmf_converter();
        let name = file_name_of(&folder);
        match lib.add_collection_from_folder(
            &folder,
            &ob_clips::CollectionNotes::default(),
            converter.as_deref(),
        ) {
            Ok(report) => {
                self.clip_collection = Some(name.clone());
                self.clip_status = Some(collection_result_sentence(&name, &report));
            }
            Err(e) => self.fail(e),
        }
    }

    /// "Add a collection" from a .zip of pictures.
    pub(crate) fn add_clips_from_zip(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Zip files", &["zip"])
            .pick_file()
        else {
            return;
        };
        let name = file_name_of(&path);
        // Size before read (health audit S-8): a zip past
        // the clip intake's batch cap is refused from its
        // filesystem size, before its bytes are read whole.
        if let Ok(meta) = std::fs::metadata(&path) {
            if meta.len() > ob_clips::MAX_CLIP_BATCH_BYTES {
                self.fail(format!(
                    "\"{name}\" is too large to bring in as clip art (over 512 megabytes)."
                ));
                return;
            }
        }
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(e) => {
                self.fail(format!("Could not read {name}: {e}"));
                return;
            }
        };
        let Ok(lib) = ob_clips::ClipLibrary::open(ob_clips::default_library_dir()) else {
            self.fail("The clip art library folder could not be opened.".into());
            return;
        };
        let converter = wmf_converter();
        let coll_name = name
            .rsplit_once('.')
            .map(|(stem, _)| stem.to_string())
            .unwrap_or(name.clone());
        match lib.add_collection_from_zip(
            &name,
            &bytes,
            &ob_clips::CollectionNotes::default(),
            converter.as_deref(),
        ) {
            Ok(report) => {
                self.clip_collection = Some(coll_name.clone());
                self.clip_status = Some(collection_result_sentence(&coll_name, &report));
            }
            Err(e) => self.fail(e),
        }
    }

    /// Put one loose sheet on the canvas (R-3): its
    /// panels, stories, and styles swap with the
    /// bulletin's, and the office works on it with every
    /// canvas habit they already have. One sheet at a
    /// time: a sheet already on the canvas goes back
    /// first, and any typing session is finished before
    /// the swap.
    pub(crate) fn edit_insert_on_pages(&mut self, id: &str) {
        if self.insert_edit.as_deref() == Some(id) {
            self.show_pages = true;
            return;
        }
        self.leave_insert_editing();
        self.commit_if_editing();
        if swap_insert_parts(&mut self.doc, id) {
            self.insert_edit = Some(id.to_string());
            // Undo history does not cross the swap: a
            // snapshot spans one document shape, and an
            // undo restored mid-swap would put the other
            // shape's panels under this one's name.
            self.undo_stack.clear();
            self.redo_stack.clear();
            self.reset_interaction_state();
            // The preview and edit-layout caches key on
            // the frame snapshot, which is canonical now
            // (round 3, R3-7): a swap no longer changes
            // the key, so drop both caches by hand or the
            // sheet would be painted from the bulletin's
            // cached layout.
            self.preview_cache = None;
            self.edit_layout = None;
            self.show_pages = true;
            let name = self
                .doc
                .inserts
                .iter()
                .find(|i| i.id == id)
                .map(|i| i.name.clone())
                .unwrap_or_default();
            self.note(format!(
                "Working on the loose sheet '{name}'. It prints on its own sheet."
            ));
        }
    }

    /// Take the loose sheet off the canvas (R-3): the
    /// swap runs back, history clears (same reason as
    /// going in), and the buffers that mirror the
    /// bulletin re-read the restored document.
    pub(crate) fn leave_insert_editing(&mut self) {
        let Some(id) = self.insert_edit.take() else {
            return;
        };
        self.commit_if_editing();
        swap_insert_parts(&mut self.doc, &id);
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.reset_interaction_state();
        // Same cache drop as going in (round 3, R3-7):
        // the canonical key does not change at the swap.
        self.preview_cache = None;
        self.edit_layout = None;
        self.sync_buffers();
    }

    /// The document as every file on disk must see it:
    /// with the sheet's parts back in their places, even
    /// while a sheet is on the canvas.
    pub(crate) fn canonical_document(&self) -> Document {
        let mut doc = self.doc.clone();
        if let Some(id) = &self.insert_edit {
            swap_insert_parts(&mut doc, id);
        }
        doc
    }

    /// Print one loose sheet on its own (R-3): the sheet
    /// synthesized as a document, through the reader path
    /// at the print purpose, to a file of its own whose
    /// name says which bulletin, which sheet, and which
    /// Sunday, so a folder of print copies reads like a
    /// sentence.
    pub(crate) fn print_insert_sheet(&mut self, insert: &ob_model::Insert) {
        self.commit_if_editing();
        let mut doc = insert.as_document(&self.doc);
        if let Ok(lib) = ob_fonts::FontLibrary::open(ob_fonts::default_library_dir()) {
            let bundled = ob_text::shaping::bundled_font_infos();
            ob_fonts::pin_document_fonts(&mut doc, &lib, &bundled);
        }
        let stem = self
            .path
            .as_ref()
            .and_then(|p| p.file_stem())
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Bulletin".to_string());
        let date = self
            .doc
            .fields
            .get("occasion_date")
            .cloned()
            .unwrap_or_default();
        let suggested = format!("{stem} - {} - {date}.pdf", insert_file_stem(&insert.name));
        if let Some(path) = rfd::FileDialog::new()
            .set_file_name(suggested)
            .add_filter("PDF", &["pdf"])
            .save_file()
        {
            let (bytes, report) = ob_pdf::export_reader_pdf_report_with_cache(
                &doc,
                ob_pdf::ExportPurpose::ForPrinting,
                &mut self.export_image_cache,
            );
            match write_file_atomically(&path, &bytes) {
                Ok(()) => {
                    self.note(format!(
                        "Loose sheet written: {}, {}.{}{}",
                        file_name_of(&path),
                        human_file_size(bytes.len()),
                        missing_pictures_sentence(&report),
                        spoken_only_sentence(&report)
                    ));
                }
                Err(e) => self.fail(format!("Could not save the loose sheet: {e}")),
            }
        }
    }

    /// Rescan the sheet library folder: the .obdi files,
    /// named by their file names.
    pub(crate) fn refresh_insert_library(&mut self) {
        let mut out: Vec<(String, std::path::PathBuf)> = Vec::new();
        if let Ok(rd) = std::fs::read_dir(insert_library_dir()) {
            for e in rd.flatten() {
                let p = e.path();
                let is_obdi = p
                    .extension()
                    .map(|x| x.eq_ignore_ascii_case("obdi"))
                    .unwrap_or(false);
                if is_obdi && p.is_file() {
                    let name = p
                        .file_stem()
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    out.push((name, p));
                }
            }
        }
        out.sort_by_key(|(name, _)| name.to_lowercase());
        self.insert_lib = out;
        self.insert_lib_dirty = false;
    }

    /// Save one of this bulletin's sheets to the library,
    /// so any bulletin after this one can bring it in.
    pub(crate) fn save_insert_to_library(&mut self, id: &str) {
        // The canonical document: the sheet's own parts
        // in their places, even when this sheet is the
        // one on the canvas right now.
        let doc = self.canonical_document();
        let Some(ins) = doc.inserts.iter().find(|i| i.id == id).cloned() else {
            return;
        };
        let dir = insert_library_dir();
        if let Err(e) = std::fs::create_dir_all(&dir) {
            self.fail(format!("The sheet library folder could not be made: {e}"));
            return;
        }
        let file = dir.join(format!("{}.obdi", insert_file_stem(&ins.name)));
        match ob_format::save_insert(&ins, &doc, &file) {
            Ok(()) => {
                self.insert_lib_dirty = true;
                self.note(format!("Saved '{}' to the sheet library.", ins.name));
            }
            Err(e) => self.fail(format!("Could not save the sheet to the library: {e}")),
        }
    }

    /// Add a library sheet to this bulletin: the sheet is
    /// copied in under a fresh id and its pictures join
    /// the bulletin's store, so from here the bulletin
    /// travels complete whatever happens to the library.
    pub(crate) fn add_insert_from_library(&mut self, path: &std::path::Path) {
        let (mut ins, assets, bytes) = match ob_format::load_insert(path) {
            Ok(v) => v,
            Err(e) => {
                self.fail(format!("That sheet could not be read: {e}"));
                return;
            }
        };
        self.leave_insert_editing();
        self.push_undo_now();
        ins.id = fresh_insert_id(&self.doc);
        // A picture the bulletin already holds keeps its
        // bytes; its rights record merges field by field
        // (round 3, R3-11; round 4, R4-5): a sheet saved
        // from a fuller bulletin used to lose its credit
        // lines silently when the plainer copy was
        // already here, and a whole-record win used to
        // lose the other side's half just as silently.
        let mut filled = 0usize;
        for rec in assets {
            match self.doc.assets.iter_mut().find(|a| a.sha256 == rec.sha256) {
                Some(existing) => {
                    let merged = merge_rights_records(&existing.rights, &rec.rights);
                    if merged != existing.rights {
                        existing.rights = merged;
                        filled += 1;
                    }
                }
                None => self.doc.assets.push(rec),
            }
        }
        for (hash, data) in bytes {
            self.doc.asset_bytes.insert(hash, data);
        }
        let name = ins.name.clone();
        let new_id = ins.id.clone();
        self.doc.inserts.push(ins);
        self.insert_sel = Some(new_id);
        self.insert_sel_loaded = None;
        let mut msg = format!("Added the loose sheet '{name}' to this bulletin.");
        if filled > 0 {
            msg.push_str(&format!(
                " {} picture record{} {} filled in from the sheet.",
                filled,
                if filled == 1 { "" } else { "s" },
                if filled == 1 { "was" } else { "were" }
            ));
        }
        self.note(msg);
    }

    /// Add a fresh loose sheet to this bulletin: one
    /// half-letter panel, one empty text box, this week
    /// only until the office says otherwise.
    pub(crate) fn add_insert(&mut self) {
        let typed = self.insert_add_name.trim().to_string();
        let name = if typed.is_empty() {
            "Loose sheet".to_string()
        } else {
            typed
        };
        self.leave_insert_editing();
        self.push_undo_now();
        let id = fresh_insert_id(&self.doc);
        self.doc.inserts.push(new_insert(&id, &name));
        self.insert_add_name.clear();
        self.insert_sel = Some(id);
        self.insert_sel_loaded = None;
        self.note(format!(
            "Added the loose sheet '{name}'. Work on this sheet to give it words."
        ));
    }

    /// Take a sheet out of this bulletin. It survives in
    /// last week's file, and in the library if it was
    /// saved there; the note says so.
    pub(crate) fn remove_insert(&mut self, id: &str) {
        if self.insert_edit.as_deref() == Some(id) {
            self.leave_insert_editing();
        }
        let Some(name) = self
            .doc
            .inserts
            .iter()
            .find(|i| i.id == id)
            .map(|i| i.name.clone())
        else {
            return;
        };
        self.push_undo_now();
        self.doc.inserts.retain(|i| i.id != id);
        if self.insert_sel.as_deref() == Some(id) {
            self.insert_sel = None;
            self.insert_sel_loaded = None;
        }
        self.insert_arm_remove = None;
        self.note(format!(
            "Took the loose sheet '{name}' out of this bulletin. It is still in \
             last week's file, and in the sheet library if you saved it there."
        ));
    }

    /// Load the settings buffers from the sheet the Loose
    /// sheets window is now showing.
    pub(crate) fn load_insert_buffers(&mut self, id: &str) {
        let Some(ins) = self.doc.inserts.iter().find(|i| i.id == id) else {
            return;
        };
        self.insert_name_buf = ins.name.clone();
        self.insert_date_buf = ins
            .keep_through
            .map(|d| format!("{:04}-{:02}-{:02}", d.year, d.month, d.day))
            .unwrap_or_default();
        self.insert_copies_buf = if ins.sheet.copies == 0 {
            String::new()
        } else {
            ins.sheet.copies.to_string()
        };
        self.insert_paper_buf = ins.sheet.paper_note.clone();
        self.insert_sel_loaded = Some(id.to_string());
    }

    /// The selected sheet, mutably, when the window's
    /// settings belong to one.
    pub(crate) fn selected_insert_mut(&mut self) -> Option<&mut ob_model::Insert> {
        let id = self.insert_sel.clone()?;
        self.doc.inserts.iter_mut().find(|i| i.id == id)
    }

    pub(crate) fn apply_insert_name(&mut self) {
        let name = self.insert_name_buf.trim().to_string();
        if name.is_empty() {
            self.fail("A loose sheet needs a name: the Print step and Next Sunday name it when they speak of it.".into());
            if let Some(id) = self.insert_sel.clone() {
                self.load_insert_buffers(&id);
            }
            return;
        }
        let changed = self
            .selected_insert_mut()
            .map(|ins| ins.name != name)
            .unwrap_or(false);
        if changed {
            self.push_undo_now();
            if let Some(ins) = self.selected_insert_mut() {
                ins.name = name.clone();
                // The first panel wears the sheet's name,
                // the way a bulletin's pages wear theirs.
                if let Some(panel) = ins.panels.first_mut() {
                    panel.name = name.clone();
                }
            }
            self.note(format!("The sheet is called '{name}' now."));
        }
    }

    pub(crate) fn apply_insert_date(&mut self) {
        let text = self.insert_date_buf.trim().to_string();
        if text.is_empty() {
            self.fail(
                "A sheet that rides until a date needs the date. Write it like \
                 2026-11-08, or pick Standing or This week only."
                    .into(),
            );
            if let Some(id) = self.insert_sel.clone() {
                self.load_insert_buffers(&id);
            }
            return;
        }
        match ob_model::parse_flexible_date(&text, today_simple()) {
            Some(d) => {
                let changed = self
                    .selected_insert_mut()
                    .map(|ins| ins.keep_through != Some(d))
                    .unwrap_or(false);
                if changed {
                    self.push_undo_now();
                    if let Some(ins) = self.selected_insert_mut() {
                        ins.keep_through = Some(d);
                    }
                    self.note(format!(
                        "The sheet rides until {}.",
                        ob_layout::long_date(naive_of(d))
                    ));
                }
            }
            None => {
                self.fail(
                    "That date does not read. Try year-month-day, like 2026-11-08, \
                     or write it out, like November 8, 2026."
                        .into(),
                );
                if let Some(id) = self.insert_sel.clone() {
                    self.load_insert_buffers(&id);
                }
            }
        }
    }

    pub(crate) fn apply_insert_copies(&mut self) {
        let text = self.insert_copies_buf.trim().to_string();
        let value: Option<usize> = if text.is_empty() {
            Some(0)
        } else {
            text.parse().ok()
        };
        let Some(value) = value else {
            self.fail("Copies is a number, or blank for as many as the bulletin.".into());
            if let Some(id) = self.insert_sel.clone() {
                self.load_insert_buffers(&id);
            }
            return;
        };
        let changed = self
            .selected_insert_mut()
            .map(|ins| ins.sheet.copies != value)
            .unwrap_or(false);
        if changed {
            self.push_undo_now();
            if let Some(ins) = self.selected_insert_mut() {
                ins.sheet.copies = value;
            }
        }
    }

    pub(crate) fn apply_insert_paper(&mut self) {
        let note = self.insert_paper_buf.trim().to_string();
        let changed = self
            .selected_insert_mut()
            .map(|ins| ins.sheet.paper_note != note)
            .unwrap_or(false);
        if changed {
            self.push_undo_now();
            if let Some(ins) = self.selected_insert_mut() {
                ins.sheet.paper_note = note;
            }
        }
    }

    pub(crate) fn set_insert_life(&mut self, id: &str, life: ob_model::InsertLife) {
        self.push_undo_now();
        if let Some(ins) = self.doc.inserts.iter_mut().find(|i| i.id == id) {
            ins.life = life;
        }
        // The date buffer belongs to the new life now.
        self.insert_sel_loaded = None;
    }

    /// A sheet's shape pick: its panels take the shape's
    /// size (the boxes keep their places; a smaller sheet
    /// may leave a box hanging past the edge, and the
    /// canvas shows that plainly), and its sheet record
    /// takes the shape's key.
    pub(crate) fn set_insert_preset(&mut self, id: &str, preset: &str) {
        let Some(size) = ob_model::insert_sheet_size(preset) else {
            return;
        };
        self.push_undo_now();
        if let Some(ins) = self.doc.inserts.iter_mut().find(|i| i.id == id) {
            ins.sheet.preset = preset.to_string();
            for panel in &mut ins.panels {
                panel.size = size;
            }
        }
    }

    /// The Loose sheets window (R-3): this bulletin's
    /// sheets with their lives and papers, the settings
    /// for one chosen sheet, and the library the sheets
    /// outlive the week in. Modeless, like the clip
    /// library: it sits beside the five steps and changes
    /// nothing until a named button is clicked.
    pub(crate) fn inserts_window(&mut self, ctx: &egui::Context) {
        if !self.show_inserts {
            return;
        }
        if self.insert_lib_dirty {
            self.refresh_insert_library();
        }
        let mut open = self.show_inserts;
        egui::Window::new("Loose sheets")
            .open(&mut open)
            .resizable(true)
            .default_size([640.0, 580.0])
            .show(ctx, |ui| {
                if let Some(id) = self.insert_edit.clone() {
                    let name = self
                        .doc
                        .inserts
                        .iter()
                        .find(|i| i.id == id)
                        .map(|i| i.name.clone())
                        .unwrap_or_default();
                    ui.label(
                        egui::RichText::new(format!(
                            "You are editing the loose sheet '{name}'. It prints on its own sheet."
                        ))
                        .strong(),
                    );
                    if ui.button("Back to the bulletin").clicked() {
                        self.leave_insert_editing();
                    }
                    ui.separator();
                }
                ui.label(egui::RichText::new("In this bulletin").strong());
                ui.label("Loose sheets ride inside the bulletin but are not its pages. Each one prints on its own paper, in its own count.");
                if self.doc.inserts.is_empty() {
                    ui.label("No loose sheets in this bulletin yet. Add one below, or bring one in from the library.");
                }
                let rows: Vec<(String, String, String)> = self
                    .doc
                    .inserts
                    .iter()
                    .map(|i| {
                        (
                            i.id.clone(),
                            i.name.clone(),
                            format!(
                                "{} · {} · {} · {}",
                                insert_life_words(i),
                                insert_size_words(&i.sheet),
                                insert_copies_words(&i.sheet),
                                insert_paper_words(&i.sheet)
                            ),
                        )
                    })
                    .collect();
                for (id, name, words) in rows {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(&name).strong());
                        if self.insert_edit.as_deref() == Some(id.as_str()) {
                            if ui.button("Back to the bulletin").clicked() {
                                self.leave_insert_editing();
                            }
                        } else if ui.button("Work on this sheet").clicked() {
                            self.edit_insert_on_pages(&id);
                        }
                        if ui.button("Choose").clicked() {
                            self.insert_sel = Some(id.clone());
                        }
                    });
                    ui.small(words);
                }
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Name for a new sheet:");
                    ui.text_edit_singleline(&mut self.insert_add_name);
                    if ui.button("Add a loose sheet").clicked() {
                        self.add_insert();
                    }
                });
                ui.separator();
                if let Some(sel) = self.insert_sel.clone() {
                    if self.insert_sel_loaded.as_deref() != Some(sel.as_str()) {
                        self.load_insert_buffers(&sel);
                    }
                    let chosen = self.doc.inserts.iter().find(|i| i.id == sel).cloned();
                    if let Some(ins) = chosen {
                        ui.label(egui::RichText::new(format!("About '{}'", ins.name)).strong());
                        ui.horizontal(|ui| {
                            ui.label("Name:");
                            let r = ui.text_edit_singleline(&mut self.insert_name_buf);
                            if r.lost_focus() {
                                self.apply_insert_name();
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label("How long it rides:");
                            for (life, words) in [
                                (ob_model::InsertLife::Standing, "Standing"),
                                (ob_model::InsertLife::Dated, "Until a date"),
                                (ob_model::InsertLife::ThisWeekOnly, "This week only"),
                            ] {
                                if ui.selectable_label(ins.life == life, words).clicked()
                                    && ins.life != life
                                {
                                    self.set_insert_life(&sel, life);
                                }
                            }
                        });
                        if ins.life == ob_model::InsertLife::Dated {
                            ui.horizontal(|ui| {
                                ui.label("Last day it rides:");
                                let r = ui.add(
                                    egui::TextEdit::singleline(&mut self.insert_date_buf)
                                        .hint_text("like 2026-11-08"),
                                );
                                if r.lost_focus() {
                                    self.apply_insert_date();
                                }
                            });
                        }
                        ui.horizontal(|ui| {
                            ui.label("Sheet shape:");
                            egui::ComboBox::from_id_salt("insert-shape")
                                .selected_text(
                                    ob_model::INSERT_SHEET_SHAPES
                                        .iter()
                                        .find(|(k, _)| *k == ins.sheet.preset)
                                        .map(|(_, w)| *w)
                                        .unwrap_or(ins.sheet.preset.as_str()),
                                )
                                .show_ui(ui, |ui| {
                                    for (key, words) in ob_model::INSERT_SHEET_SHAPES {
                                        if ui
                                            .selectable_label(ins.sheet.preset == key, words)
                                            .clicked()
                                        {
                                            self.set_insert_preset(&sel, key);
                                        }
                                    }
                                });
                        });
                        ui.horizontal(|ui| {
                            ui.label("Copies:");
                            let r = ui.add(
                                egui::TextEdit::singleline(&mut self.insert_copies_buf)
                                    .desired_width(56.0),
                            );
                            if r.lost_focus() {
                                self.apply_insert_copies();
                            }
                            ui.label("(blank means as many as the bulletin)");
                        });
                        ui.horizontal(|ui| {
                            ui.label("Paper:");
                            let r = ui.add(
                                egui::TextEdit::singleline(&mut self.insert_paper_buf)
                                    .hint_text("like blue paper, or card stock"),
                            );
                            if r.lost_focus() {
                                self.apply_insert_paper();
                            }
                        });
                        ui.horizontal(|ui| {
                            if ui
                                .button(format!("Save '{}' to the library", ins.name))
                                .clicked()
                            {
                                self.save_insert_to_library(&sel);
                            }
                            let remove_label = if self.insert_arm_remove.as_deref()
                                == Some(sel.as_str())
                            {
                                "Click again to take it out"
                            } else {
                                "Take it out of this bulletin"
                            };
                            if ui.button(remove_label).clicked() {
                                if self.insert_arm_remove.as_deref() == Some(sel.as_str()) {
                                    self.remove_insert(&sel);
                                } else {
                                    self.insert_arm_remove = Some(sel.clone());
                                }
                            }
                        });
                    } else {
                        ui.label("That sheet is not in this bulletin anymore.");
                    }
                } else if !self.doc.inserts.is_empty() {
                    ui.label("Choose a sheet above to change its name, its date, and its paper.");
                }
                ui.separator();
                ui.label(egui::RichText::new("From the library").strong());
                ui.label("A sheet saved to the library is here for every bulletin after. Adding one copies it in: the bulletin keeps its own copy from then on.");
                if self.insert_lib.is_empty() {
                    ui.label("The sheet library is empty.");
                }
                for (name, path) in self.insert_lib.clone() {
                    ui.horizontal(|ui| {
                        ui.label(&name);
                        if ui.button("Add to this bulletin").clicked() {
                            self.add_insert_from_library(&path);
                        }
                    });
                }
            });
        self.show_inserts = open;
    }

    pub(crate) fn pages_window(&mut self, ctx: &egui::Context) {
        if !self.show_pages {
            return;
        }
        if self.doc.panels.is_empty() {
            self.show_pages = false;
            return;
        }
        self.canvas_panel = self.canvas_panel.min(self.doc.panels.len() - 1);
        let idx = self.canvas_panel;
        let mut open = self.show_pages;
        egui::Window::new("Pages")
            .open(&mut open)
            .resizable(true)
            .default_size([900.0, 700.0])
            .min_size([520.0, 420.0])
            .show(ctx, |ui| {
                // While a loose sheet is on the canvas, the
                // banner says what state the work is in and
                // carries the way back (R-3).
                if let Some(id) = self.insert_edit.clone() {
                    let name = self
                        .doc
                        .inserts
                        .iter()
                        .find(|i| i.id == id)
                        .map(|i| i.name.clone())
                        .unwrap_or_default();
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(format!(
                                "You are editing the loose sheet '{name}'. It prints on its own sheet."
                            ))
                            .strong(),
                        );
                        if ui.button("Back to the bulletin").clicked() {
                            self.leave_insert_editing();
                        }
                    });
                    ui.separator();
                }
                ui.horizontal(|ui| {
                    if ui.button("Previous page").clicked() {
                        self.page_by(-1);
                    }
                    let panel = &self.doc.panels[self.canvas_panel];
                    ui.label(egui::RichText::new(format!(
                        "{}  ({} of {})",
                        panel.name,
                        self.canvas_panel + 1,
                        self.doc.panels.len()
                    )).strong());
                    if ui.button("Next page").clicked() {
                        self.page_by(1);
                    }
                    ui.separator();
                    if ui.button("Smaller").clicked() {
                        self.pages_zoom = (self.pages_zoom / 1.25).clamp(0.4, 3.0);
                    }
                    if ui.button("Fit").clicked() {
                        self.pages_zoom = 1.0;
                    }
                    if ui.button("Bigger").clicked() {
                        self.pages_zoom = (self.pages_zoom * 1.25).clamp(0.4, 3.0);
                    }
                    ui.separator();
                    if ui.button("Add a text box").clicked() {
                        self.add_text_box(idx);
                    }
                    if ui.button("Add a picture box").clicked() {
                        self.add_picture_box(idx);
                    }
                    if ui
                        .add_enabled(
                            self.selected_frame.is_some(),
                            egui::Button::new("Picture..."),
                        )
                        .clicked()
                    {
                        self.show_pic_picker = true;
                    }
                    if ui.button("Clip art...").clicked() {
                        self.ensure_starter_clips();
                        self.show_clips = true;
                    }
                    if ui.button("Loose sheets...").clicked() {
                        self.insert_lib_dirty = true;
                        self.show_inserts = true;
                    }
                    if self.doc.panels[idx].background_image.is_some() {
                        ui.separator();
                        if ui.button("Remove the old page picture").clicked() {
                            self.push_undo_now();
                            self.doc.panels[idx].background_image = None;
                            self.bg_tex_key.clear();
                            self.bg_tex = None;
                            self.note(
                                "Old page picture removed for this page.".into(),
                            );
                        }
                    }
                });
                ui.label("Click a box to select it, then drag to move it. Drag the corner square to size it.");
                ui.label("Double-click a box to type in it. Arrow keys nudge a selected box.");
                ui.label("A filled picture box that is already selected: drag inside it to move the picture within its box; double-click to center the picture again.");
                ui.label("Right-click anything for its exact place in inches. The blue guides never print.");
                ui.small("Keys: Tab moves to the next box, Enter types in it, arrows move a box (Shift for bigger steps), Ctrl+C and Ctrl+V copy a box, Ctrl+Z undoes.");
                // Threading (T1): the selected text box can
                // continue into another box, and a box in a
                // thread can start its own text. The info line
                // names the thread in words.
                {
                    let sel = self.selected_frame.and_then(|fi| {
                        self.doc.panels[idx].frames.get(fi).map(|f| {
                            (
                                fi,
                                f.name.clone(),
                                f.kind == ob_model::FrameKind::Text,
                                f.story_id.clone(),
                            )
                        })
                    });
                    if let Some((fi, name, is_text, Some(sid))) = sel {
                        if is_text {
                            let mates = self.doc.frames_of_story(&sid);
                            ui.horizontal(|ui| {
                                if self.thread_arm == Some((idx, fi)) {
                                    if ui.button("Stop choosing").clicked() {
                                        self.thread_arm = None;
                                        self.thread_confirm = None;
                                        self.note(
                                            "No box chosen. The text stays as it is.".into(),
                                        );
                                    }
                                } else if ui
                                    .button("Continue this text in another box...")
                                    .clicked()
                                {
                                    self.arm_thread_from(idx, fi);
                                }
                                if mates.len() > 1
                                    && mates.first() != Some(&(idx, fi))
                                    && ui.button("Make this box start its own text").clicked()
                                {
                                    self.split_thread_at(idx, fi);
                                }
                            });
                            if mates.len() > 1 {
                                ui.label(format!(
                                    "Selected: {name}. This text runs through {} boxes.",
                                    mates.len()
                                ));
                            } else {
                                ui.label(format!("Selected: {name}."));
                            }
                            if self.thread_arm == Some((idx, fi)) {
                                ui.label(
                                    "Now click the box this text should continue in.",
                                );
                            }
                        }
                    }
                }
                // Music boxes (M1): the selection line says
                // the move-only rule in one plain sentence.
                if let Some(f) = self
                    .selected_frame
                    .and_then(|fi| self.doc.panels[idx].frames.get(fi))
                {
                    if f.music.is_some() {
                        ui.label(format!(
                            "Selected: {}. This is music: its size keeps the notes \
                             readable, so the box can be moved but not resized.",
                            f.name
                        ));
                    }
                }
                if self.discarded_edit.is_some()
                    && ui.button("Bring the typed words back").clicked()
                {
                    self.restore_discarded_edit();
                }
                if self.editing.map(|(pi, _)| pi) != Some(idx) && self.editing.is_some() {
                    self.commit_edit();
                }
                if let Some((_, fi)) = self.editing {
                    let frame_name = self.doc.panels[idx]
                        .frames
                        .get(fi)
                        .map(|f| f.name.clone())
                        .unwrap_or_default();
                    self.sync_edit_styles();
                    self.sync_edit_spoken();
        self.sync_edit_web_hidden();
                    // The caret's paragraph comes from the
                    // shaped editor's own position (E1).
                    self.refresh_editor_layout();
                    let para = self
                        .edit_layout
                        .as_ref()
                        .map(|l| l.map.para_at_display(self.edit_caret))
                        .unwrap_or(0);
                    self.edit_cursor_para = para;
                    let cur_style = self
                        .edit_styles
                        .get(para)
                        .cloned()
                        .unwrap_or_else(|| "body".to_string());
                    let cur_name = self
                        .doc
                        .style(&cur_style)
                        .map(|st| st.name.clone())
                        .unwrap_or_else(|| cur_style.clone());
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(format!("Typing in: {frame_name}")).strong());
                        ui.label("This paragraph:");
                        egui::ComboBox::from_id_salt("para-style")
                            .selected_text(cur_name)
                            .show_ui(ui, |ui| {
                                for st in self.doc.styles.clone() {
                                    if ui
                                        .selectable_label(cur_style == st.id, st.name.clone())
                                        .clicked()
                                        && para < self.edit_styles.len()
                                    {
                                        self.edit_styles[para] = st.id.clone();
                                    }
                                }
                            });
                        // Bold / Italic for the selected words
                        // (G1b); the buttons show the state at
                        // the caret or over the selection, and
                        // Ctrl+B / Ctrl+I do the same thing.
                        let (bold_on, italic_on) = self.editor_flag_states();
                        let runs_live = !self.edit_runs_detached && !self.edit_layout_failed;
                        if ui
                            .add_enabled(runs_live, egui::SelectableLabel::new(bold_on, "Bold"))
                            .clicked()
                        {
                            self.editor_toggle_flag(runsedit::Flag::Bold);
                        }
                        if ui
                            .add_enabled(
                                runs_live,
                                egui::SelectableLabel::new(italic_on, "Italic"),
                            )
                            .clicked()
                        {
                            self.editor_toggle_flag(runsedit::Flag::Italic);
                        }
                        // Spoken only: this paragraph is read
                        // aloud in the service and never
                        // printed (prayer-list privacy). A
                        // paragraph flag, not a run flag, so
                        // it works even when the runs are
                        // detached.
                        let spoken_on =
                            self.edit_spoken.get(para).copied().unwrap_or(false);
                        if ui
                            .add(egui::SelectableLabel::new(spoken_on, "Spoken only"))
                            .on_hover_text(
                                "Spoken only: this paragraph is read aloud in the \
                                 service but never printed. Prayer-list names \
                                 belong here.",
                            )
                            .clicked()
                            && para < self.edit_spoken.len()
                        {
                            self.edit_spoken[para] = !spoken_on;
                        }
                        // Not for the web: this paragraph
                        // prints in the bulletin but stays off
                        // the web page (R-2b). Words meant for
                        // the room, like children's names,
                        // belong here. A paragraph flag, like
                        // spoken only, so it works even when
                        // the runs are detached.
                        let web_off =
                            self.edit_web_hidden.get(para).copied().unwrap_or(false);
                        if ui
                            .add(egui::SelectableLabel::new(web_off, "Not for the web"))
                            .on_hover_text(
                                "Not for the web: this paragraph prints in the \
                                 bulletin but stays off the web page. The \
                                 bulletin is a room; the web page is the world.",
                            )
                            .clicked()
                            && para < self.edit_web_hidden.len()
                        {
                            self.edit_web_hidden[para] = !web_off;
                        }
                        if ui.button("Done typing").clicked() {
                            self.edit_done = true;
                        }
                    });
                    // Field words show filled in, shaded, and
                    // cannot be typed over (E1); say where to
                    // change them before someone tries (J12).
                    ui.small("Shaded words fill themselves in from your church and this Sunday. They cannot be typed over; change them on the Start and This Sunday steps.");
                }
                ui.separator();
                egui::ScrollArea::both()
                    .max_height(820.0)
                    .show(ui, |ui| {
                        self.panel_canvas(ui, idx, ctx);
                    });
                ui.separator();
                let n = self.doc.panels[idx].guides.len();
                ui.small(format!(
                    "{n} guide{} on this page. They save with the bulletin.",
                    if n == 1 { "" } else { "s" }
                ));
            });
        self.show_pages = open;
    }

    /// Draw and interact with one panel: rulers, tracing picture,
    /// frame outlines, guides with drag + right-click editing.
    pub(crate) fn panel_canvas(&mut self, ui: &mut egui::Ui, idx: usize, ctx: &egui::Context) {
        let panel = self.doc.panels[idx].clone();
        let pw = panel.size.w as f32 / 1000.0; // points
        let ph = panel.size.h as f32 / 1000.0;
        let ruler = 20.0_f32;
        let avail_w = (ui.available_width() - ruler - 4.0).max(120.0);
        let scale = ((avail_w / pw).min(760.0 / ph) * self.pages_zoom).clamp(0.15, 6.0);
        let disp_w = pw * scale;
        let disp_h = ph * scale;
        let (full, _) = ui.allocate_exact_size(
            egui::vec2(disp_w + ruler + 2.0, disp_h + ruler + 2.0),
            egui::Sense::hover(),
        );
        let origin = egui::pos2(full.min.x + ruler, full.min.y + ruler);
        let to_px =
            |x_pt: f32, y_pt: f32| egui::pos2(origin.x + x_pt * scale, origin.y + y_pt * scale);
        let panel_rect = egui::Rect::from_min_size(origin, egui::vec2(disp_w, disp_h));
        let painter = ui.painter_at(full.shrink(0.0));

        // Rulers: quarter-inch ticks, a number each inch.
        let in_w = pw / 72.0;
        let in_h = ph / 72.0;
        let mut t = 0.0_f32;
        while t <= in_w + 0.001 {
            let x = origin.x + t * 72.0 * scale;
            let len = if (t * 4.0).fract() < 0.01 {
                if t.fract() < 0.01 {
                    11.0
                } else {
                    6.0
                }
            } else {
                6.0
            };
            painter.line_segment(
                [
                    egui::pos2(x, origin.y - 2.0),
                    egui::pos2(x, origin.y - 2.0 - len),
                ],
                egui::Stroke::new(0.75_f32, egui::Color32::GRAY),
            );
            if t.fract() < 0.01 && t > 0.0 {
                painter.text(
                    egui::pos2(x - 4.0, origin.y - 20.0),
                    egui::Align2::LEFT_TOP,
                    format!("{}", t as i32),
                    egui::FontId::proportional(8.0),
                    egui::Color32::GRAY,
                );
            }
            t += 0.25;
        }
        let mut t = 0.0_f32;
        while t <= in_h + 0.001 {
            let y = origin.y + t * 72.0 * scale;
            let len = if t.fract() < 0.01 { 11.0 } else { 6.0 };
            painter.line_segment(
                [
                    egui::pos2(origin.x - 2.0, y),
                    egui::pos2(origin.x - 2.0 - len, y),
                ],
                egui::Stroke::new(0.75_f32, egui::Color32::GRAY),
            );
            if t.fract() < 0.01 && t > 0.0 {
                painter.text(
                    egui::pos2(origin.x - 18.0, y - 4.0),
                    egui::Align2::LEFT_TOP,
                    format!("{}", t as i32),
                    egui::FontId::proportional(8.0),
                    egui::Color32::GRAY,
                );
            }
            t += 0.25;
        }

        // The panel itself, its tracing picture, and its frames.
        painter.rect_filled(panel_rect, 0.0, egui::Color32::WHITE);
        if let Some(bg) = &panel.background_image {
            let key = format!(
                "{}|{}",
                self.path
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .or_else(|| { self.asset_base.as_ref().map(|d| d.display().to_string()) })
                    .unwrap_or_default(),
                bg
            );
            if self.bg_tex_key != key {
                self.bg_tex_key = key.clone();
                self.bg_tex = self.load_bg_texture(ctx, bg);
            }
            if self.bg_tex.is_none() {
                // A missing tracing picture used to fail silently
                // (audit R7): say so on the page itself.
                painter.text(
                    panel_rect.min + egui::vec2(10.0, 10.0),
                    egui::Align2::LEFT_TOP,
                    "The old page picture for this panel could not be found.\nIt lives beside the bulletin file.",
                    egui::FontId::proportional(12.0),
                    self.theme.problem,
                );
            }
            if let Some(tex) = &self.bg_tex {
                painter.image(
                    tex.id(),
                    panel_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }
        }
        for frame in &panel.frames {
            let r = frame.rect;
            let frect = egui::Rect::from_min_max(
                to_px(r.x as f32 / 1000.0, r.y as f32 / 1000.0),
                to_px((r.x + r.w) as f32 / 1000.0, (r.y + r.h) as f32 / 1000.0),
            );
            painter.rect_stroke(
                frect,
                0.0,
                egui::Stroke::new(0.75_f32, egui::Color32::LIGHT_GRAY),
            );
        }
        // Picture boxes show their pictures (G2), decoded
        // through the same door the print path uses, so the
        // page and the sheet cannot disagree about the art.
        for frame in &panel.frames {
            if frame.kind != ob_model::FrameKind::Picture {
                continue;
            }
            let r = frame.rect;
            let frect = egui::Rect::from_min_max(
                to_px(r.x as f32 / 1000.0, r.y as f32 / 1000.0),
                to_px((r.x + r.w) as f32 / 1000.0, (r.y + r.h) as f32 / 1000.0),
            );
            let box_pt = (r.w as f32 / 1000.0, r.h as f32 / 1000.0);
            match &frame.picture {
                None => {
                    dashed_rect(&painter, frect, egui::Color32::from_gray(150));
                    painter.text(
                        frect.center(),
                        egui::Align2::CENTER_CENTER,
                        "No picture yet",
                        egui::FontId::proportional(12.0),
                        egui::Color32::from_gray(130),
                    );
                }
                Some(placement) => {
                    let bytes = self.doc.asset_data(&placement.asset).map(|b| b.to_vec());
                    match bytes {
                        None => {
                            dashed_rect(&painter, frect, self.theme.problem);
                            painter.text(
                                frect.center(),
                                egui::Align2::CENTER_CENTER,
                                "The picture for this box\ncould not be found.",
                                egui::FontId::proportional(11.0),
                                self.theme.problem,
                            );
                        }
                        Some(bytes) => {
                            let key = format!("asset:{}", placement.asset);
                            match self.picture_texture(ctx, &key, &bytes, 1024) {
                                Some((tex, _, _)) => {
                                    let probe = ob_image::probe(&bytes);
                                    let (pw, ph) = probe
                                        .map(|p| (p.width as f32, p.height as f32))
                                        .unwrap_or((box_pt.0, box_pt.1));
                                    let (sx, sy, sw, sh) = ob_model::picture_shown_rect(
                                        placement.fit,
                                        pw as f64,
                                        ph as f64,
                                        box_pt.0 as f64,
                                        box_pt.1 as f64,
                                        placement.crop_x,
                                        placement.crop_y,
                                    );
                                    let shown = egui::Rect::from_min_max(
                                        to_px(
                                            r.x as f32 / 1000.0 + sx as f32,
                                            r.y as f32 / 1000.0 + sy as f32,
                                        ),
                                        to_px(
                                            r.x as f32 / 1000.0 + (sx as f32 + sw as f32),
                                            r.y as f32 / 1000.0 + (sy as f32 + sh as f32),
                                        ),
                                    );
                                    let clip = painter.with_clip_rect(frect);
                                    if placement.fit == ob_model::PictureFit::ShowWhole {
                                        clip.image(
                                            tex.id(),
                                            shown,
                                            egui::Rect::from_min_max(
                                                egui::pos2(0.0, 0.0),
                                                egui::pos2(1.0, 1.0),
                                            ),
                                            egui::Color32::WHITE,
                                        );
                                    } else {
                                        // Fill: the frame shows
                                        // the crop fractions'
                                        // slice of the picture.
                                        let fx = (box_pt.0 as f64 / sw).clamp(0.0, 1.0) as f32;
                                        let fy = (box_pt.1 as f64 / sh).clamp(0.0, 1.0) as f32;
                                        let ux = (placement.crop_x as f32) * (1.0 - fx);
                                        let uy = (placement.crop_y as f32) * (1.0 - fy);
                                        clip.image(
                                            tex.id(),
                                            frect,
                                            egui::Rect::from_min_max(
                                                egui::pos2(ux, uy),
                                                egui::pos2(ux + fx, uy + fy),
                                            ),
                                            egui::Color32::WHITE,
                                        );
                                    }
                                }
                                None => {
                                    let words = match ob_image::sniff(&bytes) {
                                        ob_image::PictureFormat::Avif => {
                                            "This is an AVIF picture.\nConvert it to JPEG or PNG\nand put it in again."
                                        }
                                        ob_image::PictureFormat::Heic => {
                                            "This is a HEIC picture.\nConvert it to JPEG or PNG\nand put it in again."
                                        }
                                        _ => "This picture could not be read.",
                                    };
                                    dashed_rect(&painter, frect, self.theme.problem);
                                    painter.text(
                                        frect.center(),
                                        egui::Align2::CENTER_CENTER,
                                        words,
                                        egui::FontId::proportional(11.0),
                                        self.theme.problem,
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
        if let Some(fi) = self.selected_frame {
            if let Some(f) = panel.frames.get(fi) {
                let r = f.rect;
                let frect = egui::Rect::from_min_max(
                    to_px(r.x as f32 / 1000.0, r.y as f32 / 1000.0),
                    to_px((r.x + r.w) as f32 / 1000.0, (r.y + r.h) as f32 / 1000.0),
                );
                let blue = self.theme.accent;
                painter.rect_stroke(frect, 0.0, egui::Stroke::new(1.75_f32, blue));
                // No corner square on a music box: there is
                // no resize to offer (M1).
                if f.music.is_none() {
                    let handle = egui::Rect::from_min_size(
                        egui::pos2(frect.max.x - 9.0, frect.max.y - 9.0),
                        egui::vec2(9.0, 9.0),
                    );
                    painter.rect_filled(handle, 0.0, egui::Color32::WHITE);
                    painter.rect_stroke(handle, 0.0, egui::Stroke::new(1.25_f32, blue));
                }
                if self.drag_frame == Some(fi) {
                    painter.text(
                        frect.min + egui::vec2(4.0, -16.0),
                        egui::Align2::LEFT_TOP,
                        format!(
                            "Left {:.2} in, Top {:.2} in, {:.2} x {:.2} in",
                            ob_model::mpt_to_inches(r.x),
                            ob_model::mpt_to_inches(r.y),
                            ob_model::mpt_to_inches(r.w),
                            ob_model::mpt_to_inches(r.h)
                        ),
                        egui::FontId::proportional(11.0),
                        blue,
                    );
                }
            }
        }

        // The story text, shaped exactly as it will print. The
        // frame being typed in hides its lines; the editor widget
        // (below) stands in for them.
        {
            let lines = self.shaped_panel_lines(idx);
            let clip = painter.with_clip_rect(panel_rect);
            let editing_rect = self
                .editing
                .filter(|(pi, _)| *pi == idx)
                .and_then(|(_, fi)| panel.frames.get(fi).map(|f| f.rect));
            for line in &lines {
                if let Some(er) = editing_rect {
                    let lx = (line.x * 1000.0) as ob_model::Mpt;
                    let ly = (line.y * 1000.0) as ob_model::Mpt;
                    if lx >= er.x && lx <= er.x + er.w && ly >= er.y && ly <= er.y + er.h {
                        continue;
                    }
                }
                paint_line_segments(
                    &clip,
                    &self.font_rows,
                    &self.bound_families,
                    line,
                    scale,
                    |lx| to_px(lx, line.y),
                );
                // The privacy tags (spoken only, not for
                // the web): small caps above the paragraph's
                // first line, in the info color, so the state
                // is seen on the page and never hidden.
                // Canvas only; no printed copy ever carries
                // these words. A paragraph with both marks
                // wears both words on the one tag line.
                let tag = match (line.spoken_tag, line.web_tag) {
                    (true, true) => Some("SPOKEN ONLY + NOT FOR WEB"),
                    (true, false) => Some("SPOKEN ONLY"),
                    (false, true) => Some("NOT FOR WEB"),
                    (false, false) => None,
                };
                if let Some(tag) = tag {
                    clip.text(
                        to_px(line.x, line.y - line.size * 1.45),
                        egui::Align2::LEFT_BOTTOM,
                        tag,
                        egui::FontId::proportional((7.5 * scale).clamp(6.0, 11.0)),
                        self.theme.info,
                    );
                }
            }
        }

        // Threading marks (T1). The red overflow mark (a
        // small red square with a white plus, the Publisher
        // convention) sits at the bottom-right corner of a
        // thread's LAST box whenever words remain that no box
        // could hold. The continuation badges appear only
        // while a threaded box is selected, and only on that
        // box and its thread-mates on this page: calm, never
        // a permanent spiderweb.
        {
            let threads = self.doc_threads();
            let frame_px = |r: ob_model::Rect| {
                egui::Rect::from_min_max(
                    to_px(r.x as f32 / 1000.0, r.y as f32 / 1000.0),
                    to_px((r.x + r.w) as f32 / 1000.0, (r.y + r.h) as f32 / 1000.0),
                )
            };
            for thread in threads.values() {
                if thread.leftover_lines == 0 {
                    continue;
                }
                if let Some(&(lpi, lfi)) = thread.frames.last() {
                    if lpi == idx {
                        if let Some(f) = panel.frames.get(lfi) {
                            let frect = frame_px(f.rect);
                            let side = 11.0_f32;
                            let sq = egui::Rect::from_min_size(
                                egui::pos2(frect.max.x - side - 1.5, frect.max.y - side - 1.5),
                                egui::vec2(side, side),
                            );
                            painter.rect_filled(sq, 1.0, self.theme.problem);
                            let c = sq.center();
                            let arm = 3.0_f32;
                            let plus = egui::Stroke::new(1.5_f32, egui::Color32::WHITE);
                            painter.line_segment(
                                [egui::pos2(c.x - arm, c.y), egui::pos2(c.x + arm, c.y)],
                                plus,
                            );
                            painter.line_segment(
                                [egui::pos2(c.x, c.y - arm), egui::pos2(c.x, c.y + arm)],
                                plus,
                            );
                        }
                    }
                }
            }
            if let Some(sfi) = self.selected_frame {
                if let Some(sid) = panel.frames.get(sfi).and_then(|f| f.story_id.clone()) {
                    if let Some(thread) = threads.get(&sid) {
                        if thread.frames.len() > 1 {
                            let badge = |painter: &egui::Painter,
                                         frect: egui::Rect,
                                         top: bool,
                                         text: &str| {
                                let font = egui::FontId::proportional(10.0);
                                let galley = painter.layout_no_wrap(
                                    text.to_string(),
                                    font.clone(),
                                    egui::Color32::WHITE,
                                );
                                let size = galley.size() + egui::vec2(9.0, 4.0);
                                let min = if top {
                                    egui::pos2(frect.min.x + 4.0, frect.min.y + 4.0)
                                } else {
                                    egui::pos2(frect.min.x + 4.0, frect.max.y - 4.0 - size.y)
                                };
                                let rect = egui::Rect::from_min_size(min, size);
                                painter.rect_filled(rect, 3.0, self.theme.accent);
                                painter.text(
                                    rect.min + egui::vec2(4.5, 2.0),
                                    egui::Align2::LEFT_TOP,
                                    text,
                                    font,
                                    egui::Color32::WHITE,
                                );
                            };
                            for (pos, &(mpi, mfi)) in thread.frames.iter().enumerate() {
                                if mpi != idx {
                                    continue;
                                }
                                let Some(f) = panel.frames.get(mfi) else {
                                    continue;
                                };
                                let frect = frame_px(f.rect);
                                if pos > 0 {
                                    let prev_panel = thread.frames[pos - 1].0;
                                    let text = if prev_panel == idx {
                                        "Continued from another box on this page".to_string()
                                    } else {
                                        format!(
                                            "Continued from the box on {}",
                                            self.doc.panels[prev_panel].name
                                        )
                                    };
                                    badge(&painter, frect, true, &text);
                                }
                                if pos + 1 < thread.frames.len() {
                                    let next_panel = thread.frames[pos + 1].0;
                                    let text = if next_panel == idx {
                                        "Continues in another box on this page".to_string()
                                    } else {
                                        format!(
                                            "Continues in the box on {}",
                                            self.doc.panels[next_panel].name
                                        )
                                    };
                                    badge(&painter, frect, false, &text);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Guides, hit-testing, dragging.
        let response = ui.interact(
            panel_rect,
            egui::Id::new("panel-canvas"),
            egui::Sense::click_and_drag(),
        );
        let pointer = response.hover_pos().or(self.ctx_pos);
        if response.hovered() {
            if let Some(pos) = response.hover_pos() {
                self.ctx_pos = Some(pos);
            }
        }
        // The menu must open where the right-click landed, not
        // where the pointer last hovered (audit C3/R3).
        if ui.input(|i| i.pointer.secondary_pressed()) {
            if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                if panel_rect.contains(pos) {
                    self.ctx_pos = Some(pos);
                }
            }
        }
        let guides_now = self.doc.panels[idx].guides.clone();
        let near_guide_tol = |pos: egui::Pos2, tol: f32| -> Option<usize> {
            let mut best: Option<(usize, f32)> = None;
            for (gi, g) in guides_now.iter().enumerate() {
                let line_px = if g.vertical {
                    origin.x + (g.pos as f32 / 1000.0) * scale
                } else {
                    origin.y + (g.pos as f32 / 1000.0) * scale
                };
                let d = if g.vertical {
                    (pos.x - line_px).abs()
                } else {
                    (pos.y - line_px).abs()
                };
                if d <= tol && best.map(|(_, bd)| d < bd).unwrap_or(true) {
                    best = Some((gi, d));
                }
            }
            best.map(|(gi, _)| gi)
        };
        // At the larger text sizes the guides get a wider
        // grab, like the corner square below (A1).
        let guide_tol = 5.0 * self.settings.ui_scale.text_factor();
        let near_guide = |pos: egui::Pos2| near_guide_tol(pos, guide_tol);
        let to_mpt = |pos: egui::Pos2| -> (ob_model::Mpt, ob_model::Mpt) {
            (
                (((pos.x - origin.x) / scale) * 1000.0) as ob_model::Mpt,
                (((pos.y - origin.y) / scale) * 1000.0) as ob_model::Mpt,
            )
        };
        let frame_hit = |pm: (ob_model::Mpt, ob_model::Mpt)| -> Option<usize> {
            panel.frames.iter().rposition(|f| {
                pm.0 >= f.rect.x
                    && pm.0 <= f.rect.x + f.rect.w
                    && pm.1 >= f.rect.y
                    && pm.1 <= f.rect.y + f.rect.h
            })
        };
        if response.double_clicked() {
            if let Some(pos) = pointer {
                let pm = to_mpt(pos);
                if let Some(fi) = frame_hit(pm) {
                    // A double-click on a picture box centers
                    // its picture again (G2); it never starts
                    // typing, a picture box holds no words.
                    let pic = self.doc.panels[idx].frames[fi].picture.clone();
                    if self.doc.panels[idx].frames[fi].kind == ob_model::FrameKind::Picture {
                        if let Some(mut p) = pic {
                            if p.crop_x != 0.5 || p.crop_y != 0.5 {
                                self.push_undo_now();
                                p.crop_x = 0.5;
                                p.crop_y = 0.5;
                                self.doc.panels[idx].frames[fi].picture = Some(p);
                                self.note("Picture centered in its box.".into());
                            }
                        }
                    } else if self.editing != Some((idx, fi))
                        && self.doc.panels[idx].frames[fi].story_id.is_some()
                    {
                        // A double-click on the box already being
                        // edited selects a word in the shaped
                        // editor (handled below), not a re-entry.
                        self.enter_edit(idx, fi);
                    }
                }
            }
        }
        if self.editing.map(|(pi, _)| pi) == Some(idx) {
            let mut commit = self.edit_done;
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.cancel_edit();
                return;
            }
            if response.drag_started() {
                if let Some(pos) = pointer {
                    let pm = to_mpt(pos);
                    let (_, efi) = self.editing.expect("editing checked");
                    let f = &self.doc.panels[idx].frames[efi];
                    let inside = pm.0 >= f.rect.x
                        && pm.0 <= f.rect.x + f.rect.w
                        && pm.1 >= f.rect.y
                        && pm.1 <= f.rect.y + f.rect.h;
                    if !inside {
                        commit = true;
                    }
                }
            }
            if commit {
                self.edit_done = false;
                self.commit_edit();
            }
        }
        if response.drag_started() {
            if let Some(pos) = pointer {
                self.begin_gesture();
                self.drag_guide = near_guide(pos).filter(|&gi| !panel.guides[gi].locked);
                if self.drag_guide.is_none() {
                    let pm = to_mpt(pos);
                    match frame_hit(pm) {
                        Some(fi) => {
                            let f = &self.doc.panels[idx].frames[fi];
                            // The corner square's grab zone
                            // grows with the text size (A1);
                            // the square drawn stays 9 px.
                            let grip =
                                (14_000.0 * self.settings.ui_scale.text_factor()) as ob_model::Mpt;
                            // A music box is never resized (M1):
                            // its size is the legibility
                            // contract, so its corner drags it
                            // like any other part of the box.
                            let handle = self.selected_frame == Some(fi)
                                && f.music.is_none()
                                && pm.0 >= f.rect.x + f.rect.w - grip
                                && pm.1 >= f.rect.y + f.rect.h - grip;
                            // A drag that starts INSIDE an
                            // already-selected filled picture
                            // box pans the picture within its
                            // box instead of moving the box
                            // (G2): grab the thing itself. The
                            // box still moves by its edges,
                            // its corner, the arrow keys, and
                            // the right-click numbers.
                            let edge = (8.0 / scale * 1000.0) as ob_model::Mpt;
                            let interior = pm.0 >= f.rect.x + edge
                                && pm.0 <= f.rect.x + f.rect.w - edge
                                && pm.1 >= f.rect.y + edge
                                && pm.1 <= f.rect.y + f.rect.h - edge;
                            let pan = self.selected_frame == Some(fi)
                                && !handle
                                && interior
                                && f.kind == ob_model::FrameKind::Picture
                                && f.picture
                                    .as_ref()
                                    .map(|p| p.fit == ob_model::PictureFit::Fill)
                                    .unwrap_or(false);
                            self.selected_frame = Some(fi);
                            if pan {
                                let p = f.picture.as_ref().expect("pan checked a picture");
                                self.drag_pan = Some(fi);
                                self.pan_origin = Some((p.crop_x, p.crop_y, pm.0, pm.1));
                                self.drag_frame = None;
                                self.drag_resize = false;
                            } else {
                                self.drag_frame = Some(fi);
                                self.drag_resize = handle;
                                self.grab_dx = pm.0 - f.rect.x;
                                self.grab_dy = pm.1 - f.rect.y;
                            }
                        }
                        None => self.selected_frame = None,
                    }
                }
            }
        }
        if response.dragged() && self.editing.map(|(pi, _)| pi) != Some(idx) {
            if let (Some(gi), Some(pos)) = (self.drag_guide, pointer) {
                let g = &mut self.doc.panels[idx].guides[gi];
                let extent = if g.vertical {
                    panel.size.w
                } else {
                    panel.size.h
                };
                let new_pos = if g.vertical {
                    (((pos.x - origin.x) / scale) * 1000.0) as ob_model::Mpt
                } else {
                    (((pos.y - origin.y) / scale) * 1000.0) as ob_model::Mpt
                };
                g.pos = new_pos.clamp(0, extent);
            }
            if let (Some(fi), Some(pos)) = (self.drag_pan, pointer) {
                // Panning a picture inside its box: the
                // picture follows the pointer, expressed in
                // the model's crop fractions (the share of
                // the trimmed overflow showing on each side).
                if let Some((crop_x, crop_y, start_x, start_y)) = self.pan_origin {
                    let pm = to_mpt(pos);
                    let (asset, fit, box_rect) = {
                        let f = &self.doc.panels[idx].frames[fi];
                        (
                            f.picture.as_ref().map(|p| p.asset.clone()),
                            f.picture.as_ref().map(|p| p.fit),
                            f.rect,
                        )
                    };
                    if let (Some(asset), Some(fit)) = (asset, fit) {
                        if let Some(bytes) = self.doc.asset_data(&asset).map(|b| b.to_vec()) {
                            if let Ok(probe) = ob_image::probe(&bytes) {
                                let (_, _, shown_w, shown_h) = ob_model::picture_shown_rect(
                                    fit,
                                    probe.width as f64,
                                    probe.height as f64,
                                    box_rect.w as f64,
                                    box_rect.h as f64,
                                    crop_x,
                                    crop_y,
                                );
                                let over_x = shown_w - box_rect.w as f64;
                                let over_y = shown_h - box_rect.h as f64;
                                let f = &mut self.doc.panels[idx].frames[fi];
                                if let Some(p) = &mut f.picture {
                                    if over_x > 0.0 {
                                        p.crop_x = (crop_x - (pm.0 - start_x) as f64 / over_x)
                                            .clamp(0.0, 1.0);
                                    }
                                    if over_y > 0.0 {
                                        p.crop_y = (crop_y - (pm.1 - start_y) as f64 / over_y)
                                            .clamp(0.0, 1.0);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if let (Some(fi), Some(pos)) = (self.drag_frame, pointer) {
                let pm = to_mpt(pos);
                let threshold = (6.0 / scale * 1000.0) as ob_model::Mpt;
                let xt: Vec<ob_model::Mpt> = self.doc.panels[idx]
                    .guides
                    .iter()
                    .filter(|g| g.vertical)
                    .map(|g| g.pos)
                    .chain([0, panel.size.w / 2, panel.size.w])
                    .collect();
                let yt: Vec<ob_model::Mpt> = self.doc.panels[idx]
                    .guides
                    .iter()
                    .filter(|g| !g.vertical)
                    .map(|g| g.pos)
                    .chain([0, panel.size.h / 2, panel.size.h])
                    .collect();
                let f = &mut self.doc.panels[idx].frames[fi];
                if self.drag_resize {
                    let nw = ob_model::snap_to(pm.0, &xt, threshold) - f.rect.x;
                    let nh = ob_model::snap_to(pm.1, &yt, threshold) - f.rect.y;
                    f.rect.w = nw.clamp(18_000, panel.size.w - f.rect.x);
                    f.rect.h = nh.clamp(18_000, panel.size.h - f.rect.y);
                } else {
                    let mut nx = pm.0 - self.grab_dx;
                    let mut ny = pm.1 - self.grab_dy;
                    for anchor in [0, f.rect.w / 2, f.rect.w] {
                        let snapped = ob_model::snap_to(nx + anchor, &xt, threshold);
                        if snapped != nx + anchor {
                            nx = snapped - anchor;
                            break;
                        }
                    }
                    for anchor in [0, f.rect.h / 2, f.rect.h] {
                        let snapped = ob_model::snap_to(ny + anchor, &yt, threshold);
                        if snapped != ny + anchor {
                            ny = snapped - anchor;
                            break;
                        }
                    }
                    f.rect.x = nx.clamp(0, panel.size.w - f.rect.w);
                    f.rect.y = ny.clamp(0, panel.size.h - f.rect.h);
                }
            }
        }
        if response.drag_stopped() {
            self.drag_guide = None;
            self.drag_frame = None;
            self.drag_resize = false;
            self.drag_pan = None;
            self.pan_origin = None;
            self.end_gesture();
        }
        if response.clicked() {
            if let Some(pos) = pointer {
                if near_guide(pos).is_none() {
                    let pm = to_mpt(pos);
                    if self.thread_arm.is_some() {
                        // A continuation pick is armed (T1):
                        // this click chooses the box, it does
                        // not just select.
                        self.thread_pick_clicked(idx, frame_hit(pm));
                    } else {
                        self.selected_frame = frame_hit(pm);
                    }
                }
            }
        }
        // Canvas keys when no text box is being typed in: Tab
        // cycles boxes, Enter types in the selected box, Ctrl+C
        // and Ctrl+V copy and paste a box, arrows nudge.
        let canvas_keys_active = ctx.memory(|m| m.focused()).is_none() && self.editing.is_none();
        if canvas_keys_active {
            let n_frames = self.doc.panels[idx].frames.len();
            if n_frames > 0 {
                let backwards = ui.input(|i| i.modifiers.shift);
                if ui.input(|i| i.key_pressed(egui::Key::Tab)) {
                    let cur = self.selected_frame.unwrap_or(0);
                    self.selected_frame = Some(if backwards {
                        (cur + n_frames - 1) % n_frames
                    } else if self.selected_frame.is_none() {
                        0
                    } else {
                        (cur + 1) % n_frames
                    });
                }
                if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    if let Some(fi) = self.selected_frame {
                        if self.doc.panels[idx].frames[fi].story_id.is_some() {
                            self.enter_edit(idx, fi);
                            return;
                        }
                    }
                }
                if self.action_pressed(ui.ctx(), "copy_box") {
                    if let Some(fi) = self.selected_frame {
                        let f = self.doc.panels[idx].frames[fi].clone();
                        let story = f
                            .story_id
                            .as_ref()
                            .and_then(|sid| self.doc.story(sid).cloned());
                        self.frame_clipboard = Some((f, story));
                        self.status = "Box copied.".into();
                    }
                }
                if self.action_pressed(ui.ctx(), "paste_box") {
                    if let Some((f, story)) = self.frame_clipboard.clone() {
                        self.paste_box(idx, f, story);
                    }
                }
            }
        }
        // Arrow keys nudge the selected frame (Shift: bigger steps),
        // when no text box is being typed in.
        if ctx.memory(|m| m.focused()).is_none() {
            if let Some(fi) = self.selected_frame {
                let step: ob_model::Mpt = if ui.input(|i| i.modifiers.shift) {
                    7_200
                } else {
                    720
                };
                let (mut dx, mut dy): (ob_model::Mpt, ob_model::Mpt) = (0, 0);
                if ui.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
                    dx = -step;
                }
                if ui.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
                    dx = step;
                }
                if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                    dy = -step;
                }
                if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                    dy = step;
                }
                if dx != 0 || dy != 0 {
                    self.begin_gesture();
                    let f = &mut self.doc.panels[idx].frames[fi];
                    f.rect.x = (f.rect.x + dx).clamp(0, panel.size.w - f.rect.w);
                    f.rect.y = (f.rect.y + dy).clamp(0, panel.size.h - f.rect.h);
                }
            }
        }
        if response.hovered() {
            if let Some(pos) = pointer {
                if let Some(gi) = near_guide(pos) {
                    let g = self.doc.panels[idx].guides[gi];
                    ctx.set_cursor_icon(if g.vertical {
                        egui::CursorIcon::ResizeHorizontal
                    } else {
                        egui::CursorIcon::ResizeVertical
                    });
                }
            }
        }
        let guide_color = self.theme.accent;
        let locked_color = egui::Color32::from_rgb(120, 120, 120);
        for (gi, g) in self.doc.panels[idx].guides.iter().enumerate() {
            let dragging = self.drag_guide == Some(gi);
            let stroke = egui::Stroke::new(
                if dragging { 2.0_f32 } else { 1.25_f32 },
                if g.locked { locked_color } else { guide_color },
            );
            if g.vertical {
                let x = origin.x + (g.pos as f32 / 1000.0) * scale;
                painter.line_segment(
                    [egui::pos2(x, origin.y), egui::pos2(x, origin.y + disp_h)],
                    stroke,
                );
                if dragging {
                    painter.text(
                        egui::pos2(x + 6.0, origin.y + 4.0),
                        egui::Align2::LEFT_TOP,
                        format!("{:.2} in from the left", ob_model::mpt_to_inches(g.pos)),
                        egui::FontId::proportional(11.0),
                        guide_color,
                    );
                }
            } else {
                let y = origin.y + (g.pos as f32 / 1000.0) * scale;
                painter.line_segment(
                    [egui::pos2(origin.x, y), egui::pos2(origin.x + disp_w, y)],
                    stroke,
                );
                if dragging {
                    painter.text(
                        egui::pos2(origin.x + 6.0, y + 4.0),
                        egui::Align2::LEFT_TOP,
                        format!("{:.2} in from the top", ob_model::mpt_to_inches(g.pos)),
                        egui::FontId::proportional(11.0),
                        guide_color,
                    );
                }
            }
        }
        painter.rect_stroke(
            panel_rect,
            0.0,
            egui::Stroke::new(1.0_f32, egui::Color32::DARK_GRAY),
        );

        // The inline editor, standing exactly where the frame
        // is (E1): the shaped editor. The frame's text is drawn
        // from the session buffer with the real faces, sizes,
        // and dot leaders, field words show as shaded chips
        // with their filled-in text, and the caret and
        // selection come from the same parley geometry as the
        // print path. Keyboard input lands on a focus sink:
        // the edit id holds keyboard focus in egui's memory
        // (no widget needs to exist for that), and this code
        // reads the raw input events while it does.
        if self.editing.map(|(pi, _)| pi) == Some(idx) {
            let (_, efi) = self.editing.expect("editing checked");
            self.sync_edit_styles();
            self.sync_edit_spoken();
            self.sync_edit_web_hidden();
            let f = panel.frames[efi].clone();
            let fx = f.rect.x as f32 / 1000.0;
            let frect = egui::Rect::from_min_max(
                to_px(fx, f.rect.y as f32 / 1000.0),
                to_px(
                    (f.rect.x + f.rect.w) as f32 / 1000.0,
                    (f.rect.y + f.rect.h) as f32 / 1000.0,
                ),
            );
            let edit_id = egui::Id::new(("canvas-edit", self.edit_salt.as_str()));
            self.refresh_editor_layout();
            if self.edit_layout_failed || self.edit_layout.is_none() {
                // FALLBACK (E1): shaped layout construction
                // produced nothing usable (the shaping stack
                // found no clusters for non-empty text), so
                // this session uses the plain overlay editor
                // rather than blocking editing. Field words
                // show as raw {braces} here, as they did
                // before the shaped editor.
                let first_size = self
                    .edit_styles
                    .first()
                    .and_then(|sid| self.doc.style(sid))
                    .map(|st| st.font_size as f32 / 1000.0 * scale)
                    .unwrap_or(12.0)
                    .clamp(10.0, 20.0);
                if self.focus_edit {
                    self.focus_edit = false;
                    ctx.memory_mut(|m| m.request_focus(edit_id));
                }
                // Words changed here bypass the runs funnel
                // (this overlay edits the buffer directly), so
                // the session's runs detach: commit will write
                // these words as plain runs rather than lay
                // stale faces over changed words (G1b).
                let before = self.edit_text.clone();
                let editor = egui::TextEdit::multiline(&mut self.edit_text)
                    .id(edit_id)
                    .font(egui::FontId::proportional(first_size))
                    .desired_width(frect.width());
                ui.put(frect, editor);
                if self.edit_text != before {
                    self.edit_runs_detached = true;
                }
            } else {
                if self.focus_edit {
                    self.focus_edit = false;
                    ctx.memory_mut(|m| m.request_focus(edit_id));
                }
                // Mouse: press places the caret, drag extends
                // the selection, double-click selects a word.
                let pointer_pos = ui.input(|i| i.pointer.interact_pos());
                if ui.input(|i| i.pointer.primary_pressed()) {
                    if let Some(pos) = pointer_pos {
                        if frect.contains(pos) {
                            ctx.memory_mut(|m| m.request_focus(edit_id));
                            if let Some(d) = self.editor_hit(pos, origin, scale, &f.rect) {
                                self.editor_click_caret(d, false);
                                self.edit_sel_drag = true;
                            }
                        }
                    }
                }
                if self.edit_sel_drag && response.dragged() {
                    if let Some(pos) = response.interact_pointer_pos() {
                        if let Some(d) = self.editor_hit(pos, origin, scale, &f.rect) {
                            self.editor_click_caret(d, true);
                        }
                    }
                }
                if response.drag_stopped() {
                    self.edit_sel_drag = false;
                }
                if response.double_clicked() {
                    if let Some(pos) = pointer_pos {
                        if frect.contains(pos) {
                            if let Some(d) = self.editor_hit(pos, origin, scale, &f.rect) {
                                self.editor_select_word(d);
                            }
                        }
                    }
                }
                // Keyboard, clipboard, and IME events.
                if ctx.memory(|m| m.has_focus(edit_id)) {
                    let events = ctx.input(|i| i.events.clone());
                    for ev in &events {
                        self.editor_event(ctx, ev);
                    }
                }
                if let Some(pos) = pointer_pos {
                    if frect.contains(pos) {
                        ctx.output_mut(|o| o.cursor_icon = egui::CursorIcon::Text);
                    }
                }
                // Draw: chips and selection under the text,
                // then the shaped lines, caret, and any IME
                // composition preview.
                let clip = painter.with_clip_rect(panel_rect);
                let sel = self.editor_selection();
                if let Some(layout) = &self.edit_layout {
                    for (pno, para) in layout.paras.iter().enumerate() {
                        let prange = layout.map.para_display_range(pno);
                        let ptop = para.top as f32 / 1000.0;
                        // The slice's line window (T1): rects
                        // on lines outside [skip, skip+drawn)
                        // belong to other boxes and are not
                        // painted here; rects inside shift up
                        // by the skipped lines.
                        let skip_h = para.skip as f32 * para.layout.line_height();
                        let visible = |ry: f32| {
                            let lh = para.layout.line_height();
                            if lh <= 0.0 {
                                return true;
                            }
                            let li = (ry / lh).round() as usize;
                            (para.skip..para.skip + para.drawn).contains(&li)
                        };
                        let lo = sel.start.max(prange.start);
                        let hi = sel.end.min(prange.end);
                        if sel.start != sel.end && lo < hi {
                            let rects = para
                                .layout
                                .selection_rects(lo - prange.start..hi - prange.start);
                            for (rx, ry, rw, rh) in rects {
                                if !visible(ry) {
                                    continue;
                                }
                                let r = egui::Rect::from_min_size(
                                    to_px(fx + rx, ptop + ry - skip_h),
                                    egui::vec2(rw * scale, rh * scale),
                                );
                                clip.rect_filled(r, 2.0, ctx.style().visuals.selection.bg_fill);
                            }
                        }
                        for chip in &layout.map.chips {
                            if chip.display.start >= prange.start && chip.display.end <= prange.end
                            {
                                let armed = self.edit_chip_armed.as_ref() == Some(&chip.display);
                                let rects = para.layout.selection_rects(
                                    chip.display.start - prange.start
                                        ..chip.display.end - prange.start,
                                );
                                for (rx, ry, rw, rh) in rects {
                                    if !visible(ry) {
                                        continue;
                                    }
                                    let r = egui::Rect::from_min_size(
                                        to_px(fx + rx, ptop + ry - skip_h),
                                        egui::vec2(rw * scale, rh * scale),
                                    );
                                    clip.rect_filled(
                                        r,
                                        2.0,
                                        egui::Color32::from_rgba_unmultiplied(0, 120, 195, 42),
                                    );
                                    if armed {
                                        clip.rect_stroke(
                                            r,
                                            2.0,
                                            egui::Stroke::new(1.5_f32, self.theme.problem),
                                        );
                                    }
                                }
                            }
                        }
                    }
                    for para in &layout.paras {
                        for (li, line) in para
                            .layout
                            .lines()
                            .iter()
                            .enumerate()
                            .skip(para.skip)
                            .take(para.drawn)
                        {
                            let line_text = &para.layout.text()[line.range.clone()];
                            if line_text.is_empty() {
                                continue;
                            }
                            let bottom = (para.top
                                + ((li - para.skip + 1) as ob_model::Mpt) * para.line_h)
                                as f32
                                / 1000.0;
                            // Same-face segments (G1b): a bold
                            // word paints in the bold face even
                            // while it is being typed.
                            for seg in line_segments(line, para.layout.text()) {
                                let probe = PreviewLine {
                                    text: seg.text.clone(),
                                    size: para.size,
                                    x: 0.0,
                                    y: 0.0,
                                    w: line.width,
                                    align: para.align,
                                    font_family: para.font_family.clone(),
                                    bold: seg.bold,
                                    italic: seg.italic,
                                    segs: Vec::new(),
                                    spoken_tag: false,
                                    web_tag: false,
                                };
                                let fid = line_font_id(
                                    &self.font_rows,
                                    &self.bound_families,
                                    &probe,
                                    (para.size * scale).max(4.0),
                                );
                                clip.text(
                                    to_px(fx + seg.dx, bottom),
                                    egui::Align2::LEFT_BOTTOM,
                                    &seg.text,
                                    fid,
                                    egui::Color32::BLACK,
                                );
                            }
                        }
                    }
                    if let Some((cpara, clocal)) = self.editor_caret_local() {
                        let para = &layout.paras[cpara];
                        let (cx, cy, ch) = para.layout.caret_rect(clocal);
                        let line_h = para.layout.line_height();
                        let line_idx = if line_h > 0.0 {
                            (cy / line_h).round() as usize
                        } else {
                            0
                        };
                        // The caret shows only when it sits
                        // on a line this box draws (T1): a
                        // caret in an earlier or later box's
                        // slice is real, but it is not on this
                        // box's page.
                        if (para.skip..para.skip + para.drawn).contains(&line_idx) {
                            let ptop = para.top as f32 / 1000.0;
                            let caret_rect = egui::Rect::from_min_size(
                                to_px(fx + cx, ptop + cy - para.skip as f32 * line_h),
                                egui::vec2(1.5, ch * scale),
                            );
                            let t = ctx.input(|i| i.time);
                            if (t * 1.2) % 1.2 < 0.75 {
                                clip.rect_filled(caret_rect, 0.0, egui::Color32::BLACK);
                            }
                            ctx.request_repaint_after(std::time::Duration::from_millis(450));
                            ctx.output_mut(|o| {
                                o.ime = Some(egui::output::IMEOutput {
                                    rect: frect,
                                    cursor_rect: caret_rect,
                                });
                            });
                            if let Some((_, pre)) = &self.ime_preedit {
                                let probe = PreviewLine {
                                    text: pre.clone(),
                                    size: para.size,
                                    x: 0.0,
                                    y: 0.0,
                                    w: 0.0,
                                    align: para.align,
                                    font_family: para.font_family.clone(),
                                    bold: para.bold,
                                    italic: para.italic,
                                    segs: Vec::new(),
                                    spoken_tag: false,
                                    web_tag: false,
                                };
                                let fid = line_font_id(
                                    &self.font_rows,
                                    &self.bound_families,
                                    &probe,
                                    (para.size * scale).max(4.0),
                                );
                                let galley =
                                    clip.layout_no_wrap(pre.clone(), fid, egui::Color32::BLACK);
                                let pos = to_px(fx + cx, ptop + cy - para.skip as f32 * line_h);
                                let gw = galley.size().x;
                                let gh = galley.size().y;
                                clip.galley(pos, galley, egui::Color32::BLACK);
                                clip.line_segment(
                                    [
                                        egui::pos2(pos.x, pos.y + gh),
                                        egui::pos2(pos.x + gw, pos.y + gh),
                                    ],
                                    egui::Stroke::new(1.0_f32, egui::Color32::BLACK),
                                );
                            }
                        }
                    }
                }
            }
        }

        // Right-click: exact position for a guide, or add one here.
        let hit = self.ctx_pos.and_then(near_guide);
        let on_the_line = self.ctx_pos.and_then(|pos| near_guide_tol(pos, 3.0));
        let frame_at_menu = self
            .ctx_pos
            .map(|pos| {
                let pm = (
                    (((pos.x - origin.x) / scale) * 1000.0) as ob_model::Mpt,
                    (((pos.y - origin.y) / scale) * 1000.0) as ob_model::Mpt,
                );
                panel.frames.iter().rposition(|f| {
                    pm.0 >= f.rect.x
                        && pm.0 <= f.rect.x + f.rect.w
                        && pm.1 >= f.rect.y
                        && pm.1 <= f.rect.y + f.rect.h
                })
            })
            .unwrap_or(None);
        let target = choose_menu_target(on_the_line, frame_at_menu, hit);
        response.context_menu(|ui| {
            // A menu action can remove or shift boxes: finish any
            // typing session first, while its indices are still
            // true (closing audit: deleting a box mid-typing in
            // another could corrupt text or panic).
            if self.editing.is_some() {
                self.commit_edit();
            }
            match target {
                MenuTarget::Frame(fi) => {
                    self.selected_frame = Some(fi);
                    let f = self.doc.panels[idx].frames[fi].clone();
                    if self.frame_menu_for != Some((fi, f.rect)) {
                        self.frame_menu_for = Some((fi, f.rect));
                        self.frame_input = [
                            format!("{:.3}", ob_model::mpt_to_inches(f.rect.x)),
                            format!("{:.3}", ob_model::mpt_to_inches(f.rect.y)),
                            format!("{:.3}", ob_model::mpt_to_inches(f.rect.w)),
                            format!("{:.3}", ob_model::mpt_to_inches(f.rect.h)),
                        ];
                    }
                    ui.label(egui::RichText::new(format!("Box: {}", f.name)).strong());
                    if f.music.is_some() {
                        // Music boxes move but never resize
                        // (M1): their size keeps the notes
                        // readable, so only the place is
                        // offered, in the same words as the
                        // selection line.
                        ui.label("Exact place, in inches:");
                        for (i, label) in ["Left", "Top"].iter().enumerate() {
                            ui.horizontal(|ui| {
                                ui.label(format!("{label}:"));
                                ui.text_edit_singleline(&mut self.frame_input[i]);
                            });
                        }
                        ui.label(
                            "This is music. Its size keeps the notes readable; \
                             it can be moved, not resized.",
                        );
                        if ui.button("Set the place").clicked() {
                            let left = ob_model::parse_inches(&self.frame_input[0]);
                            let top = ob_model::parse_inches(&self.frame_input[1]);
                            if let (Some(l), Some(t)) = (left, top) {
                                self.push_undo_now();
                                let fr = &mut self.doc.panels[idx].frames[fi];
                                fr.rect.x = ob_model::inch(l)
                                    .clamp(0, panel.size.w - fr.rect.w);
                                fr.rect.y = ob_model::inch(t)
                                    .clamp(0, panel.size.h - fr.rect.h);
                                self.status = "Box moved.".into();
                                ui.close_menu();
                            } else {
                                self.status =
                                    "One of those is not a measurement I can read. Try 1.25 or 1 1/4."
                                        .into();
                            }
                        }
                    } else {
                    ui.label("Exact place and size, in inches:");
                    for (i, label) in ["Left", "Top", "Width", "Height"].iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(format!("{label}:"));
                            ui.text_edit_singleline(&mut self.frame_input[i]);
                        });
                    }
                    if ui.button("Set place and size").clicked() {
                        let parsed: Vec<Option<f64>> = self
                            .frame_input
                            .iter()
                            .map(|t| ob_model::parse_inches(t))
                            .collect();
                        if parsed.iter().all(|v| v.is_some()) {
                            let v: Vec<ob_model::Mpt> = parsed
                                .iter()
                                .map(|o| ob_model::inch(o.unwrap()))
                                .collect();
                            self.push_undo_now();
                            let fr = &mut self.doc.panels[idx].frames[fi];
                            fr.rect.w = v[2].clamp(18_000, panel.size.w);
                            fr.rect.h = v[3].clamp(18_000, panel.size.h);
                            fr.rect.x = v[0].clamp(0, panel.size.w - fr.rect.w);
                            fr.rect.y = v[1].clamp(0, panel.size.h - fr.rect.h);
                            self.status = "Box moved and sized.".into();
                            ui.close_menu();
                        } else {
                            self.status =
                                "One of those is not a measurement I can read. Try 1.25 or 1 1/4."
                                    .into();
                        }
                    }
                    }
                    if f.kind == ob_model::FrameKind::Picture && f.music.is_none() {
                        ui.separator();
                        match &f.picture {
                            None => {
                                ui.label("This picture box is empty.");
                                if ui.button("Put a picture in this box...").clicked() {
                                    self.show_pic_picker = true;
                                    ui.close_menu();
                                }
                            }
                            Some(p) => {
                                ui.label(match p.fit {
                                    ob_model::PictureFit::Fill => {
                                        "The picture fills the box; its sides may be trimmed."
                                    }
                                    ob_model::PictureFit::ShowWhole => {
                                        "The whole picture shows; empty space may show."
                                    }
                                });
                                let other = match p.fit {
                                    ob_model::PictureFit::Fill => {
                                        (ob_model::PictureFit::ShowWhole, "Show the whole picture")
                                    }
                                    ob_model::PictureFit::ShowWhole => (
                                        ob_model::PictureFit::Fill,
                                        "Fill the box (sides may be trimmed)",
                                    ),
                                };
                                if ui.button(other.1).clicked() {
                                    self.push_undo_now();
                                    if let Some(slot) = &mut self.doc.panels[idx].frames[fi].picture
                                    {
                                        slot.fit = other.0;
                                    }
                                    self.status = "Picture fit changed.".into();
                                    ui.close_menu();
                                }
                                if p.fit == ob_model::PictureFit::Fill
                                    && (p.crop_x != 0.5 || p.crop_y != 0.5)
                                    && ui.button("Center the picture").clicked()
                                {
                                    self.push_undo_now();
                                    if let Some(slot) = &mut self.doc.panels[idx].frames[fi].picture
                                    {
                                        slot.crop_x = 0.5;
                                        slot.crop_y = 0.5;
                                    }
                                    self.status = "Picture centered.".into();
                                    ui.close_menu();
                                }
                                if ui.button("Take the picture out").clicked() {
                                    self.set_frame_picture(idx, fi, None);
                                    ui.close_menu();
                                }
                                // Children in the picture
                                // (R-2b): the office's own
                                // note, shown as it stands.
                                // Check asks while it is
                                // missing; the web page
                                // takes extra care when the
                                // answer is yes.
                                ui.separator();
                                let noted = self
                                    .doc
                                    .assets
                                    .iter()
                                    .find(|a| a.sha256 == p.asset)
                                    .and_then(|a| a.rights.children_present);
                                ui.label(match noted {
                                    Some(true) => {
                                        "Children in this picture: yes, noted."
                                    }
                                    Some(false) => {
                                        "Children in this picture: no, noted."
                                    }
                                    None => {
                                        "Children in this picture: not noted yet."
                                    }
                                });
                                if ui.button("This picture has children in it").clicked() {
                                    self.set_picture_children(idx, fi, true);
                                    ui.close_menu();
                                }
                                if ui.button("No children in this picture").clicked() {
                                    self.set_picture_children(idx, fi, false);
                                    ui.close_menu();
                                }
                            }
                        }
                    }
                    if f.kind == ob_model::FrameKind::Text && f.story_id.is_some() {
                        // Threading by hand (T1): the same two
                        // actions the Pages toolbar offers, in
                        // the same words.
                        ui.separator();
                        if ui.button("Continue this text in another box...").clicked() {
                            self.arm_thread_from(idx, fi);
                            ui.close_menu();
                        }
                        let mates = self
                            .doc
                            .frames_of_story(f.story_id.as_deref().unwrap_or_default());
                        if mates.len() > 1
                            && mates.first() != Some(&(idx, fi))
                            && ui.button("Make this box start its own text").clicked()
                        {
                            self.split_thread_at(idx, fi);
                            ui.close_menu();
                        }
                    }
                    ui.separator();
                    if ui.button("Copy this box (Ctrl+C)").clicked() {
                        let story = f
                            .story_id
                            .as_ref()
                            .and_then(|sid| self.doc.story(sid).cloned());
                        self.frame_clipboard = Some((f.clone(), story));
                        self.status = "Box copied. Ctrl+V puts a copy on the page.".into();
                        ui.close_menu();
                    }
                    if ui.button("Delete this box").clicked() {
                        self.push_undo_now();
                        let gone = self.doc.panels[idx].frames.remove(fi);
                        if let Some(sid) = &gone.story_id {
                            let used_elsewhere = self
                                .doc
                                .panels
                                .iter()
                                .flat_map(|p| &p.frames)
                                .any(|fr| fr.story_id.as_deref() == Some(sid.as_str()));
                            if !used_elsewhere {
                                self.doc.stories.retain(|st| &st.id != sid);
                            }
                        }
                        self.selected_frame = None;
                        self.frame_menu_for = None;
                        self.status = "Box deleted. Undo brings it back.".into();
                        ui.close_menu();
                    }
                }
                MenuTarget::Guide(gi) => {
                    let g = self.doc.panels[idx].guides[gi];
                    if self.guide_menu_for != Some((gi, g.pos)) {
                        self.guide_menu_for = Some((gi, g.pos));
                        self.guide_input = format!("{:.3}", ob_model::mpt_to_inches(g.pos));
                    }
                    let (what, from) = if g.vertical {
                        ("Vertical guide", "from the left edge")
                    } else {
                        ("Horizontal guide", "from the top edge")
                    };
                    ui.label(egui::RichText::new(format!("{what}, {from}")).strong());
                    ui.label("Exact place, in inches (1.25 or 1 1/4 works):");
                    let edit = ui.text_edit_singleline(&mut self.guide_input);
                    let apply = ui.button("Set the position").clicked()
                        || (edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));
                    if apply {
                        match ob_model::parse_inches(&self.guide_input) {
                            Some(inches) => {
                                let extent = if g.vertical {
                                    panel.size.w
                                } else {
                                    panel.size.h
                                };
                                self.push_undo_now();
                                self.doc.panels[idx].guides[gi].pos =
                                    ob_model::inch(inches).clamp(0, extent);
                                self.status = "Guide moved.".into();
                                ui.close_menu();
                            }
                            None => {
                                self.status =
                                    "That is not a place I can read. Try 1.25 or 1 1/4.".into();
                            }
                        }
                    }
                    if ui
                        .button(if g.locked {
                            "Unlock this guide"
                        } else {
                            "Lock this guide"
                        })
                        .clicked()
                    {
                        self.push_undo_now();
                        self.doc.panels[idx].guides[gi].locked = !g.locked;
                        self.status = if g.locked {
                            "Guide unlocked: it can be dragged again.".into()
                        } else {
                            "Guide locked: it stays put until you unlock it.".into()
                        };
                        ui.close_menu();
                    }
                    if ui.button("Delete this guide").clicked() {
                        self.push_undo_now();
                        self.doc.panels[idx].guides.remove(gi);
                        self.status = "Guide deleted.".into();
                        ui.close_menu();
                    }
                }
                MenuTarget::Page => {
                    self.guide_menu_for = None;
                    let pos = self.ctx_pos.unwrap_or(origin);
                    let x_mpt = (((pos.x - origin.x) / scale) * 1000.0) as ob_model::Mpt;
                    let y_mpt = (((pos.y - origin.y) / scale) * 1000.0) as ob_model::Mpt;
                    if ui
                        .button(format!(
                            "Add vertical guide here ({:.2} in)",
                            ob_model::mpt_to_inches(x_mpt.clamp(0, panel.size.w))
                        ))
                        .clicked()
                    {
                        self.push_undo_now();
                        self.doc.panels[idx].guides.push(ob_model::Guide {
                            vertical: true,
                            pos: x_mpt.clamp(0, panel.size.w),
                            locked: false,
                        });
                        ui.close_menu();
                    }
                    if ui
                        .button(format!(
                            "Add horizontal guide here ({:.2} in)",
                            ob_model::mpt_to_inches(y_mpt.clamp(0, panel.size.h))
                        ))
                        .clicked()
                    {
                        self.push_undo_now();
                        self.doc.panels[idx].guides.push(ob_model::Guide {
                            vertical: false,
                            pos: y_mpt.clamp(0, panel.size.h),
                            locked: false,
                        });
                        ui.close_menu();
                    }
                }
            }
        });
    }

    /// Load a panel's tracing picture as an egui texture, or
    /// None when it cannot be read. A v1 document carries
    /// the picture's bytes in its asset store and the panel
    /// names the asset's fingerprint; older documents name
    /// a file beside the .obd, read as before.
    pub(crate) fn load_bg_texture(
        &self,
        ctx: &egui::Context,
        rel: &str,
    ) -> Option<egui::TextureHandle> {
        if let Some(stored) = self.doc.asset_data(rel) {
            let decoded = ob_image::decode(stored).ok()?;
            let color = egui::ColorImage::from_rgba_unmultiplied(
                [decoded.width as usize, decoded.height as usize],
                &decoded.rgba,
            );
            return Some(ctx.load_texture("trace-bg", color, egui::TextureOptions::LINEAR));
        }
        let base = self
            .path
            .as_ref()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()))
            .or_else(|| self.asset_base.clone())
            .unwrap_or_default();
        let bytes = std::fs::read(base.join(rel)).ok()?;
        let decoded = ob_image::decode(&bytes).ok()?;
        let color = egui::ColorImage::from_rgba_unmultiplied(
            [decoded.width as usize, decoded.height as usize],
            &decoded.rgba,
        );
        Some(ctx.load_texture("trace-bg", color, egui::TextureOptions::LINEAR))
    }

    /// The decoded texture for one picture, at a size class
    /// (the longest pixel edge the caller needs), decoded
    /// once and cached: (texture, pixel width, pixel height)
    /// of the decoded picture. `key` names the CONTENT (an
    /// asset fingerprint, a library file path), never the
    /// document, so the cache survives document swaps. None
    /// when the bytes do not decode (missing formats and
    /// broken files alike; callers that need the reason ask
    /// ob_image themselves).
    pub(crate) fn picture_texture(
        &mut self,
        ctx: &egui::Context,
        key: &str,
        bytes: &[u8],
        max_px: u32,
    ) -> Option<(egui::TextureHandle, u32, u32)> {
        let cache_key = (key.to_string(), max_px);
        if let Some(hit) = self.pic_tex.get(&cache_key) {
            return Some(hit.clone());
        }
        let decoded = ob_image::decode_fitted(bytes, max_px, max_px).ok()?;
        let color = egui::ColorImage::from_rgba_unmultiplied(
            [decoded.width as usize, decoded.height as usize],
            &decoded.rgba,
        );
        let tex = ctx.load_texture(
            format!("pic:{key}#{max_px}"),
            color,
            egui::TextureOptions::LINEAR,
        );
        let entry = (tex, decoded.width, decoded.height);
        self.pic_tex.insert(cache_key, entry.clone());
        Some(entry)
    }

    /// Put a picture (already in the document's asset store)
    /// into a box, or take the picture out when `placement`
    /// is None (the box stays). One undo step either way.
    pub(crate) fn set_frame_picture(
        &mut self,
        idx: usize,
        fi: usize,
        placement: Option<ob_model::PicturePlacement>,
    ) {
        if self.editing.is_some() {
            self.commit_edit();
        }
        self.push_undo_now();
        let has_picture = placement.is_some();
        let frame = &mut self.doc.panels[idx].frames[fi];
        frame.kind = ob_model::FrameKind::Picture;
        frame.picture = placement;
        if has_picture {
            // A box holds a picture OR words, never both at
            // once; the words' story stays in the document
            // (undo and the file keep it), the box lets go.
            frame.story_id = None;
        }
        self.note(if has_picture {
            "Picture put in the box.".into()
        } else {
            "Picture taken out. The box stays.".into()
        });
    }

    /// Note, on a placed picture's rights record, whether
    /// it has children in it (R-2b): the answer drives
    /// Check's question and the web page's stricter rule.
    /// A placement whose record is missing gets one, so
    /// the note always has somewhere to live. One undo
    /// step.
    pub(crate) fn set_picture_children(&mut self, idx: usize, fi: usize, present: bool) {
        let Some(asset_hash) = self.doc.panels[idx]
            .frames
            .get(fi)
            .and_then(|f| f.picture.as_ref())
            .map(|p| p.asset.clone())
        else {
            return;
        };
        self.push_undo_now();
        if let Some(asset) = self.doc.assets.iter_mut().find(|a| a.sha256 == asset_hash) {
            asset.rights.children_present = Some(present);
        } else if let Some(bytes) = self.doc.asset_data(&asset_hash).map(|b| b.to_vec()) {
            let media = match ob_image::sniff(&bytes) {
                ob_image::PictureFormat::Jpeg => "image/jpeg",
                ob_image::PictureFormat::Svg => "image/svg+xml",
                _ => "image/png",
            };
            let rights = ob_model::RightsRecord {
                children_present: Some(present),
                ..Default::default()
            };
            self.doc
                .add_asset(&bytes, ob_model::AssetKind::Picture, media, rights);
        }
        self.note(if present {
            "Noted: this picture has children in it.".into()
        } else {
            "Noted: no children in this picture.".into()
        });
    }

    /// The "Picture..." file door: pick ONE picture file and
    /// put it in the box. The file's bytes copy into the
    /// bulletin's asset store with an honest rights record
    /// (source: the file's name; license: not stated).
    pub(crate) fn pick_picture_file(&mut self, idx: usize, fi: usize) {
        if self.frame_is_music(idx, fi) {
            self.note("That box holds music; a picture cannot go in it.".into());
            return;
        }
        let Some(path) = rfd::FileDialog::new()
            .add_filter(
                "Pictures",
                &[
                    "png", "jpg", "jpeg", "webp", "gif", "bmp", "tif", "tiff", "svg", "jxl",
                    "avif", "heic", "heif",
                ],
            )
            .add_filter("All files", &[] as &[&str])
            .pick_file()
        else {
            return;
        };
        let name = file_name_of(&path);
        // Size before read (health audit S-8): a zip past
        // the clip intake's batch cap is refused from its
        // filesystem size, before its bytes are read whole.
        if let Ok(meta) = std::fs::metadata(&path) {
            if meta.len() > ob_clips::MAX_CLIP_BATCH_BYTES {
                self.fail(format!(
                    "\"{name}\" is too large to bring in as clip art: a clip art                      pack is not bigger than 512 megabytes."
                ));
                return;
            }
        }
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(e) => {
                self.fail(format!("Could not read {name}: {e}"));
                return;
            }
        };
        // The door recognizes by the bytes first, the name
        // second, and says in plain words what it cannot take.
        let format = match ob_image::sniff(&bytes) {
            ob_image::PictureFormat::Unknown => {
                ob_image::format_for_name(&name).unwrap_or(ob_image::PictureFormat::Unknown)
            }
            f => f,
        };
        match format {
            ob_image::PictureFormat::Avif | ob_image::PictureFormat::Heic => {
                self.fail(format!(
                    "{name} is a {} picture. This program cannot use that kind yet. \
                     Convert it to JPEG or PNG first (your photo program can save a copy that way), then put it in.",
                    if format == ob_image::PictureFormat::Avif { "AVIF" } else { "HEIC" }
                ));
            }
            f if f.decodable() => {
                if ob_image::probe(&bytes).is_err() {
                    self.fail(format!(
                        "{name} could not be read as a picture. It may be broken, or not a picture at all."
                    ));
                    return;
                }
                let sha = self.doc.add_asset(
                    &bytes,
                    ob_model::AssetKind::Picture,
                    f.media_type(),
                    ob_model::RightsRecord {
                        source: name.clone(),
                        ..Default::default()
                    },
                );
                self.set_frame_picture(idx, fi, Some(ob_model::PicturePlacement::new(sha)));
            }
            _ => {
                self.fail(format!(
                    "{name} is not a picture this program knows. Pictures it can use: PNG, JPEG, WebP, GIF, BMP, TIFF, SVG, and JPEG XL."
                ));
            }
        }
    }

    /// Place one of the document's own stored pictures into
    /// a box (the picker and the clip library both land here).
    /// True when the named box carries music (M1): such a
    /// box takes no photograph and offers no resize.
    pub(crate) fn frame_is_music(&self, idx: usize, fi: usize) -> bool {
        self.doc
            .panels
            .get(idx)
            .and_then(|p| p.frames.get(fi))
            .map(|f| f.music.is_some())
            .unwrap_or(false)
    }

    pub(crate) fn place_stored_picture(&mut self, idx: usize, fi: usize, sha: &str) {
        if self.frame_is_music(idx, fi) {
            self.note("That box holds music; a picture cannot go in it.".into());
            return;
        }
        self.set_frame_picture(
            idx,
            fi,
            Some(ob_model::PicturePlacement::new(sha.to_string())),
        );
    }

    /// Make sure the bundled starter set is in the Clipart
    /// Library (first open of the window only): the set's
    /// own collection, installed from the files shipped with
    /// the app, each piece recorded with its own row of the
    /// manifest shipped beside them (source page, author,
    /// license, retrieval date).
    pub(crate) fn ensure_starter_clips(&mut self) {
        if self.clips_ensured {
            return;
        }
        self.clips_ensured = true;
        if starter_clips::STARTER_FILES.is_empty() {
            return;
        }
        let Ok(lib) = ob_clips::ClipLibrary::open(ob_clips::default_library_dir()) else {
            return;
        };
        if lib.collection("Starter set").is_some() {
            return;
        }
        let _ = starter_clips::install_into(&lib);
    }

    /// Begin a PDF rescue on a worker thread: read the old
    /// bulletin with the helper, render its pages as tracing
    /// pictures, and build the draft.
    pub(crate) fn start_import(&mut self, pdf: std::path::PathBuf) {
        let profile = self.doc.profile.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        self.import_rx = Some(rx);
        self.note("Reading your old bulletin...".into());
        std::thread::spawn(move || {
            let result = (|| -> Result<ImportReview, String> {
                // The PDF door: the helper reads the file in
                // its own process and gates it there too, but
                // the refusal belongs at this door, before any
                // helper starts.
                ob_extract::check_open_door_size(&pdf)?;
                let helper = ob_extract::helper::resolve_helper(None);
                let workdir = std::env::temp_dir()
                    .join(format!("openbulletin-import-{}", std::process::id()));
                let _ = std::fs::remove_dir_all(&workdir);
                // Schema v2: the pictures inside the PDF ride
                // along into the draft's asset store.
                let pictures_dir = workdir.join("pictures");
                let dump = ob_extract::helper::dump_pdf_with_images(&helper, &pdf, &pictures_dir)?;
                let pictured =
                    ob_extract::helper::render_pdf(&helper, &pdf, &workdir.join("assets")).is_ok();
                let mut image_bytes = std::collections::HashMap::new();
                for img in &dump.images {
                    if let Ok(b) = std::fs::read(pictures_dir.join(&img.file)) {
                        image_bytes.insert(img.file.clone(), b);
                    }
                }
                let families = current_library_families();
                let out = ob_extract::draft::document_from_dump_with_assets(
                    &dump,
                    profile,
                    true,
                    &image_bytes,
                    &families,
                );
                Ok(ImportReview {
                    doc: out.doc,
                    items: out.items,
                    workdir: Some(workdir),
                    note: if pictured {
                        None
                    } else {
                        Some(
                            "The words came over, but the old pages could not be \
                             pictured underneath the draft this time."
                                .into(),
                        )
                    },
                    source_name: pdf
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| "old bulletin".into()),
                })
            })();
            let _ = tx.send(result);
        });
    }

    /// IDML import on a worker: parse the package in-process
    /// (memory-safe zip and XML), build the draft.
    pub(crate) fn start_idml_import(&mut self, path: std::path::PathBuf) {
        let profile = self.doc.profile.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        self.import_rx = Some(rx);
        self.note("Reading your InDesign file...".into());
        std::thread::spawn(move || {
            let result = (|| -> Result<ImportReview, String> {
                let bytes = ob_extract::read_open_door_file(&path)?;
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "InDesign file".into());
                let dump = ob_extract::idml::dump_from_idml(&bytes, &name)?;
                if dump.pages.is_empty() {
                    return Err("that IDML file has no pages we can read".into());
                }
                let out = ob_extract::draft::document_from_dump(
                    &dump,
                    profile,
                    false,
                    &current_library_families(),
                );
                Ok(ImportReview {
                    doc: out.doc,
                    items: out.items,
                    workdir: None,
                    source_name: name,
                    note: Some(
                        "Pictures placed in InDesign do not come over in this \
                         first version. The words, their boxes, and their fonts do."
                            .into(),
                    ),
                })
            })();
            let _ = tx.send(result);
        });
    }

    /// Scribus import on a worker: a .sla file is one XML
    /// document, read in-process into the shared dump, and the
    /// draft and its review are the same as every other
    /// rescue's.
    pub(crate) fn start_sla_import(&mut self, path: std::path::PathBuf) {
        let profile = self.doc.profile.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        self.import_rx = Some(rx);
        self.note("Reading your Scribus file...".into());
        std::thread::spawn(move || {
            let result = (|| -> Result<ImportReview, String> {
                let bytes = ob_extract::read_open_door_file(&path)?;
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "Scribus file".into());
                let (dump, loss_notes) = ob_extract::sla::import_sla(&bytes, &name)?;
                if dump.pages.is_empty() {
                    return Err("that Scribus file has no pages we can read".into());
                }
                let out = ob_extract::draft::document_from_dump(
                    &dump,
                    profile,
                    false,
                    &current_library_families(),
                );
                let mut note = "This came from a Scribus file: the words, their                                 boxes, and their fonts came over as a draft to check."
                    .to_string();
                for n in &loss_notes {
                    note.push(' ');
                    note.push_str(n);
                }
                Ok(ImportReview {
                    doc: out.doc,
                    items: out.items,
                    workdir: None,
                    source_name: name,
                    note: Some(note),
                })
            })();
            let _ = tx.send(result);
        });
    }

    /// Publisher import on a worker: LibreOffice (if the
    /// office has it) turns the .pub into a PDF, and from
    /// there the ordinary PDF rescue takes over.
    pub(crate) fn start_pub_import(&mut self, path: std::path::PathBuf) {
        let profile = self.doc.profile.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        self.import_rx = Some(rx);
        self.note("Reading your Publisher file...".into());
        std::thread::spawn(move || {
            let result = (|| -> Result<ImportReview, String> {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "Publisher file".into());
                let Some(soffice) = ob_extract::pub_bridge::find_soffice() else {
                    // No LibreOffice: our own reader takes the
                    // Publisher 2002-and-later family. Anything
                    // it cannot rescue (the older family, a
                    // damaged file) keeps the standing message.
                    if let Ok(review) = native_pub_review(&path, &name, profile.clone()) {
                        return Ok(review);
                    }
                    return Err("That Publisher file could not be read here. Publisher \
                         files ride through LibreOffice, a free program, and it is not \
                         installed on this computer. Two ways forward: install \
                         LibreOffice from libreoffice.org and try again, or open the \
                         file in Publisher, choose File, then Save As, and save it as \
                         a PDF; then open that PDF here with \"Open an old bulletin...\"."
                        .into());
                };
                let workdir = std::env::temp_dir()
                    .join(format!("openbulletin-import-{}", std::process::id()));
                let _ = std::fs::remove_dir_all(&workdir);
                let pdf = ob_extract::pub_bridge::convert_pub_to_pdf(&soffice, &path, &workdir)?;
                let helper = ob_extract::helper::resolve_helper(None);
                let pictures_dir = workdir.join("pictures");
                let dump = ob_extract::helper::dump_pdf_with_images(&helper, &pdf, &pictures_dir)?;
                let pictured =
                    ob_extract::helper::render_pdf(&helper, &pdf, &workdir.join("assets")).is_ok();
                let mut image_bytes = std::collections::HashMap::new();
                for img in &dump.images {
                    if let Ok(b) = std::fs::read(pictures_dir.join(&img.file)) {
                        image_bytes.insert(img.file.clone(), b);
                    }
                }
                let families = current_library_families();
                let out = ob_extract::draft::document_from_dump_with_assets(
                    &dump,
                    profile,
                    true,
                    &image_bytes,
                    &families,
                );
                let note = if pictured {
                    "This came from your Publisher file by way of LibreOffice, \
                     so small layout differences from the original are normal. \
                     Check each page against its picture underneath."
                        .to_string()
                } else {
                    "This came from your Publisher file by way of LibreOffice, \
                     so small layout differences from the original are normal. \
                     The old pages could not be pictured underneath the \
                     draft this time."
                        .to_string()
                };
                Ok(ImportReview {
                    doc: out.doc,
                    items: out.items,
                    workdir: Some(workdir),
                    source_name: name,
                    note: Some(note),
                })
            })();
            let _ = tx.send(result);
        });
    }

    /// One door for every old bulletin (Jacob's rule: the app
    /// conforms to the office worker, who has a file, not a
    /// format opinion). A pick that loads at once (an
    /// OpenBulletin file) answers to the unsaved-work guard
    /// first; the pick is carried through the arm in
    /// `pending_path`, so the confirming click opens the very
    /// file chosen instead of asking for it again (UI audit
    /// round 2). Imports answer to their review windows,
    /// whose own confirm guards the discard.
    /// "Start from last week" (J13): the pick goes through the
    /// same door as everything else, filters and all. One of
    /// ours (.obd) opens and moves to this Sunday, exactly as
    /// it always did. Anything else (the predecessor's .pub,
    /// most often) comes in through the one open door as a
    /// draft to review, because that is what those files are.
    pub(crate) fn open_last_week(&mut self, path: std::path::PathBuf) {
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        if ext != "obd" {
            self.route_old_bulletin(path);
            return;
        }
        match ob_format::load(&path) {
            Ok(doc) => {
                self.adopt_document(doc, true);
                self.remember_bulletin(&path);
                self.path = Some(path);
                self.seed_doc_state();
                self.mark_clean();
                self.note_disk_stamp();
                self.next_sunday();
            }
            Err(e) => self.fail(format!("Could not open that file. ({e})")),
        }
    }

    pub(crate) fn open_old_bulletin(&mut self, path: std::path::PathBuf) {
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        if ext == "obd" && !self.open_guard(PendingOpen::Plain) {
            self.pending_path = Some(path);
            return;
        }
        self.route_old_bulletin(path);
    }

    /// Route a picked old bulletin to its reader, by
    /// extension, and when the name does not say, by the
    /// file's own first bytes. Callers have settled the
    /// unsaved-work guard (the door above, or Open).
    pub(crate) fn route_old_bulletin(&mut self, path: std::path::PathBuf) {
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        match ext.as_str() {
            "pdf" => self.start_import(path),
            "idml" => self.start_idml_import(path),
            "pub" => self.start_pub_import(path),
            "sla" => self.start_sla_import(path),
            "md" => self.start_md_import(path),
            "html" | "htm" => self.start_html_import(path),
            "musicxml" | "mxl" => self.start_music_import(path),
            "obd" => match ob_format::load(&path) {
                Ok(doc) => {
                    self.adopt_document(doc, true);
                    self.remember_bulletin(&path);
                    self.path = Some(path);
                    self.seed_doc_state();
                    self.step = 1;
                    self.note("Bulletin opened.".into());
                    self.mark_clean();
                    self.note_disk_stamp();
                }
                Err(e) => self.fail(format!("Could not open that file. ({e})")),
            },
            _ => {
                // Sniff the first bytes: PDF says %PDF, an IDML
                // is a zip (PK), a .pub is a compound file, a
                // Scribus file is XML under a SCRIBUSUTF8NEW root.
                // Only the head is read (counted, never the whole
                // file): the sniff routes, the door it routes to
                // applies the open-door size gate.
                let head = {
                    use std::io::Read as _;
                    std::fs::File::open(&path)
                        .map(|mut f| {
                            let mut buf = Vec::new();
                            let _ = f.by_ref().take(4096).read_to_end(&mut buf);
                            buf
                        })
                        .unwrap_or_default()
                };
                if head.starts_with(b"%PDF") {
                    self.start_import(path);
                } else if String::from_utf8_lossy(&head).contains("<score-partwise") {
                    // A music file whose name did not say so
                    // (an extension-less or .xml MusicXML).
                    self.start_music_import(path);
                } else if head.starts_with(b"PK") {
                    // A zip: an IDML package, or a .mxl music
                    // file. The music sniff reads the whole
                    // file; IDML remains the default. The sniff's
                    // read goes through the same open-door gate;
                    // an oversize file sniffs as nothing here and
                    // the IDML door it lands on refuses it in
                    // plain words.
                    let bytes = ob_extract::read_open_door_file(&path).unwrap_or_default();
                    if ob_music::sniff_music(&bytes) {
                        self.start_music_import(path);
                    } else {
                        self.start_idml_import(path);
                    }
                } else if head.starts_with(&[0xD0, 0xCF, 0x11, 0xE0]) {
                    self.start_pub_import(path);
                } else if String::from_utf8_lossy(&head).contains("SCRIBUSUTF8NEW") {
                    self.start_sla_import(path);
                } else if ob_extract::html_import::sniff_html(&head) {
                    // A web page whose name did not say so
                    // (a newsletter export saved as .txt, say).
                    self.start_html_import(path);
                } else {
                    self.fail(
                        "That file does not look like a PDF, a Publisher file, an \
                         InDesign (IDML) file, a Scribus file, a music file \
                         (MusicXML), or a web page, so it cannot be brought in."
                            .into(),
                    );
                }
            }
        }
    }

    /// A Markdown file through the one open door (G1b):
    /// the words become a one-page draft and wait for the
    /// standard review, labeled for what they are: notes or
    /// web text, not a designed bulletin.
    pub(crate) fn start_md_import(&mut self, path: std::path::PathBuf) {
        let name = file_name_of(&path);
        let text = match ob_extract::read_open_door_text(&path) {
            Ok(t) => t,
            Err(e) => {
                self.fail(e);
                return;
            }
        };
        let doc =
            ob_model::markdown::document_from_markdown(&text, self.doc.profile.clone(), &name);
        let count = doc.stories.first().map(|s| s.paragraphs.len()).unwrap_or(0);
        self.import_review = Some(ImportReview {
            doc,
            items: vec![ob_extract::draft::ImportItem {
                panel: 0,
                label: format!("Notes from {name}"),
                badge: ob_extract::draft::Badge::CheckThis,
                detail: format!(
                    "{count} paragraph{} of notes, on one page. Read them over, \
                     then use the draft and arrange the words where they belong.",
                    if count == 1 { "" } else { "s" }
                ),
            }],
            workdir: None,
            source_name: name,
            note: Some(
                "Notes or web text: bold and italic marks in the text were honored; \
                 the layout is a plain page, ready to arrange."
                    .into(),
            ),
        });
    }

    /// A web page or newsletter export through the one open
    /// door (H2): the page's words become a draft and wait
    /// for the standard review, labeled for what they are.
    /// The parse is in-process (html5ever, no network: a
    /// picture is carried only when its bytes are inside
    /// the page itself), so like the markdown door this one
    /// answers at once.
    pub(crate) fn start_html_import(&mut self, path: std::path::PathBuf) {
        let name = file_name_of(&path);
        let bytes = match ob_extract::read_open_door_file(&path) {
            Ok(b) => b,
            Err(e) => {
                self.fail(e);
                return;
            }
        };
        match ob_extract::html_import::document_from_html(&bytes, &name, self.doc.profile.clone()) {
            Ok(out) => {
                self.import_review = Some(ImportReview {
                    doc: out.doc,
                    items: out.items,
                    workdir: None,
                    source_name: name,
                    note: Some(
                        "Web page or newsletter: the words came over in reading \
                         order with their bold and italic. The page's own layout \
                         and styling did not come over, and pictures that live \
                         on the web stayed there."
                            .into(),
                    ),
                });
            }
            Err(e) => self.fail(e),
        }
    }

    /// A music file through the one open door (M1): our own
    /// MusicXML reader counts the file, the engraver helper
    /// draws its lines at a singing size, and the draft goes
    /// through the standard review. No engraver on this
    /// computer is not a failure: the draft forms with the
    /// notes standing undrawn and a plain Check-this saying
    /// so. OpenBulletin never edits the music itself.
    pub(crate) fn start_music_import(&mut self, path: std::path::PathBuf) {
        let profile = self.doc.profile.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        self.import_rx = Some(rx);
        self.note("Reading your music file...".into());
        std::thread::spawn(move || {
            let result = (|| -> Result<ImportReview, String> {
                let bytes = ob_extract::read_open_door_file(&path)?;
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "music file".into());
                let (xml, info) = ob_music::load_music(&bytes)?;
                let media = if bytes.starts_with(b"PK") {
                    "application/vnd.recordare.musicxml"
                } else {
                    "application/vnd.recordare.musicxml+xml"
                };
                let (render, note) = match ob_music::render_music(
                    &xml,
                    ob_music::music_frame_width_mm(),
                    ob_music::TARGET_STAFF_MM,
                ) {
                    Ok(r) => (Some(r), None),
                    Err(ob_music::RenderError::NotAvailable(_)) => (
                        None,
                        Some(
                            "The notes are not drawn yet: the music engraver is not on \
                         this computer. The draft holds the music file itself, and \
                         bringing it in again where the engraver is installed draws \
                         every line."
                                .to_string(),
                        ),
                    ),
                    Err(ob_music::RenderError::Failed(why)) => return Err(why),
                };
                let out = ob_music::build_music_draft(
                    &info,
                    &bytes,
                    &name,
                    media,
                    profile,
                    render.as_ref(),
                );
                Ok(ImportReview {
                    doc: out.doc,
                    items: out.items,
                    workdir: None,
                    source_name: name,
                    note,
                })
            })();
            let _ = tx.send(result);
        });
    }

    pub(crate) fn poll_import(&mut self, ctx: &egui::Context) {
        let Some(rx) = &self.import_rx else { return };
        match rx.try_recv() {
            Ok(Ok(review)) => {
                self.import_rx = None;
                self.import_armed = false;
                self.import_review = Some(review);
                ctx.request_repaint();
            }
            Ok(Err(e)) => {
                self.import_rx = None;
                self.fail(format!("That file could not be brought in. {e}"));
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.import_rx = None;
                self.fail("That file could not be brought in; the reader stopped.".into());
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
        }
    }

    /// Adopt a reviewed draft. The pictures stay in the work
    /// folder as the asset base until the first Save moves
    /// them beside the bulletin file.
    pub(crate) fn finalize_import(&mut self, edit: bool) {
        let Some(review) = self.import_review.take() else {
            return;
        };
        self.import_armed = false;
        self.asset_base = review.workdir.clone();
        self.adopt_document(review.doc, true);
        self.path = None;
        // A fresh draft has no print copy and no clean check.
        self.print_fingerprint = None;
        self.checked_fingerprint = None;
        if edit {
            self.show_pages = true;
            self.note("Here is your old bulletin as a draft. Check it page by page.".into());
        } else {
            self.step = 1;
            self.note(
                "Here is your old bulletin as a draft. Check the date, then work \
                 through the steps. Save it under a new name when ready."
                    .into(),
            );
        }
    }

    /// Refresh the Start step's inbox scan (the inbox,
    /// and the filed folder, which the still-waiting count
    /// reads: an accepted part has arrived, even though it
    /// left the inbox). Reading never moves a file.
    pub(crate) fn refresh_inbox(&mut self) {
        let today = today_simple();
        self.inbox_items = scan_contribution_folder(&inbox_dir(), today, false);
        self.inbox_filed = scan_contribution_folder(&filed_dir(), today, true);
        self.inbox_dirty = false;
    }

    /// Open words someone sent into the contribution
    /// review: the parsed words, where they came from (an
    /// inbox file, or None for pasted words), and the text
    /// as it stands.
    pub(crate) fn open_contribution_review(
        &mut self,
        contribution: ob_extract::contribution::Contribution,
        source: Option<std::path::PathBuf>,
        text: String,
    ) {
        self.contribution_review = Some(ContributionReview {
            contribution,
            source,
            text,
            armed: false,
            editing: false,
            edit_from: String::new(),
            edit_for: String::new(),
            edit_keep: String::new(),
            edit_parts: Vec::new(),
            edit_kinds: Vec::new(),
            edit_error: None,
        });
    }

    /// Begin the review's Edit mode: the sender and date
    /// fields as written, and one raw markdown box per
    /// part, from the words as they stand.
    pub(crate) fn begin_contribution_edit(&mut self) {
        let Some(review) = &mut self.contribution_review else {
            return;
        };
        let raw = ob_extract::contribution::split_contribution(&review.text);
        review.edit_from = raw.from.clone().unwrap_or_default();
        review.edit_for = raw.for_sunday.clone().unwrap_or_default();
        review.edit_keep = raw.keep_through.clone().unwrap_or_default();
        let mut kinds = Vec::new();
        let mut bodies = Vec::new();
        for (kind, body) in raw.parts {
            if body.trim().is_empty() {
                continue;
            }
            kinds.push(kind);
            bodies.push(body);
        }
        review.edit_kinds = kinds;
        review.edit_parts = bodies;
        review.edit_error = None;
        review.editing = true;
    }

    /// Rebuild the contribution from the Edit fields and
    /// re-parse it: the one parser judges the edited words
    /// exactly as it judged the arrived ones.
    pub(crate) fn apply_contribution_edit(&mut self) {
        let Some(review) = &mut self.contribution_review else {
            return;
        };
        let parts: Vec<(ob_extract::contribution::PartKind, String)> = review
            .edit_kinds
            .iter()
            .cloned()
            .zip(review.edit_parts.iter().cloned())
            .collect();
        let from = review.edit_from.clone();
        let for_sunday = review.edit_for.clone();
        let keep = review.edit_keep.clone();
        let text = rebuild_contribution_text(&from, &for_sunday, &keep, &parts);
        match ob_extract::contribution::parse_contribution(&text, today_simple()) {
            Ok(c) => {
                review.contribution = c;
                review.text = text;
                review.editing = false;
                review.armed = false;
                review.edit_error = None;
            }
            Err(e) => {
                review.edit_error = Some(e);
            }
        }
    }

    /// Accept a reviewed contribution: its parts go into
    /// the bulletin, the bulletin is saved, and only then
    /// is the part filed. If the save does not happen, the
    /// file stays in the inbox and the status says so.
    /// Accept leaves any loose-sheet editing first, like
    /// every other door out of the sheet canvas (round 3,
    /// R3-9): the words belong in the bulletin the office
    /// sees, not inside one sheet's private arrangement.
    pub(crate) fn accept_contribution(&mut self) {
        self.leave_insert_editing();
        let Some(review) = self.contribution_review.take() else {
            return;
        };
        // A part whose words are in this document is
        // filed, never applied twice (round 3, R3-5; the
        // record moved into the document in round 4,
        // R4-4). The claim is presence-verified (round
        // 5, R5-B): the words themselves are looked for,
        // so a record whose words were deleted by hand
        // does not block this path, and a re-sent part
        // with changed bytes is still caught. The review
        // normally routes these to "File it"; this keeps
        // every other path to Accept honest too.
        if applied_claim(&self.doc, &review.contribution, &review.text) == AppliedClaim::AlreadyIn {
            let filed = self.file_contribution(&review);
            self.note(filed);
            self.inbox_dirty = true;
            return;
        }
        let before = snapshot(&self.doc);
        // The words land, and the document itself
        // remembers them from this moment, before any
        // save is attempted (round 4, R4-4) — but only
        // when every part landed completely (round 5,
        // R5-B). A save that fails, a save done by hand,
        // and a crash the journal recovers from all meet
        // the same record; a part that landed partly or
        // not at all leaves no record, so the copy still
        // sitting in the inbox can be offered its words
        // again instead of being sworn in.
        let outcome =
            apply_contribution_recording(&mut self.doc, &review.contribution, &review.text);
        let summary = outcome.lines;
        if before != snapshot(&self.doc) {
            self.record_undo(before);
        }
        self.sync_buffers();
        let saved = self.save_flow();
        if !saved {
            if self.save_conflict.is_none() {
                self.fail(
                    "The words are in the bulletin on screen, but the bulletin \
                     was not saved, so this part stays in the inbox. Save the \
                     bulletin, and the part can then be filed from the inbox."
                        .into(),
                );
            }
            // With a save conflict, the guard's own words
            // and its two buttons are already on screen;
            // the part likewise stays in the inbox.
            return;
        }
        let filed = self.file_contribution(&review);
        let mut msg = summary.join(" ");
        if !msg.is_empty() {
            msg.push(' ');
        }
        msg.push_str(&filed);
        self.note(msg);
        self.inbox_dirty = true;
    }

    /// File a part whose words are already in the
    /// bulletin (round 3, R3-5): the review's "File it"
    /// answer. No apply, no save; the filing is the
    /// whole job.
    pub(crate) fn file_applied_contribution(&mut self) {
        self.leave_insert_editing();
        let Some(review) = self.contribution_review.take() else {
            return;
        };
        let filed = self.file_contribution(&review);
        self.note(filed);
        self.inbox_dirty = true;
    }

    /// Move (or, for pasted words, write) an accepted
    /// contribution into the filed folder, under the name
    /// that keeps its sender, its Sunday, and its content
    /// fingerprint. Nothing is ever deleted: the filed copy
    /// is how "which words did we print" stays answerable.
    pub(crate) fn file_contribution(&mut self, review: &ContributionReview) -> String {
        let dir = filed_dir();
        if std::fs::create_dir_all(&dir).is_err() {
            return "The part could not be filed; it is still in the inbox.".to_string();
        }
        let fingerprint = content_fingerprint(review.text.as_bytes());
        // The Sunday the name carries: the part's own when
        // it names one; for an undated part, the Sunday it
        // was filed for, which is the open bulletin's
        // (round 3, R3-10). The still-waiting count reads
        // the date back out of the name, so an undated
        // part filed last month cannot pose as this
        // week's arrival.
        let filing_sunday = self
            .doc
            .fields
            .get("occasion_date")
            .and_then(|iso| ob_model::parse_flexible_date(iso, today_simple()));
        let name = filed_file_name(
            review.contribution.from.as_deref(),
            review.contribution.for_date.or(filing_sunday),
            &fingerprint,
        );
        let target = dir.join(&name);
        match &review.source {
            Some(path) => match std::fs::rename(path, &target) {
                Ok(()) => format!("Filed as {name}."),
                // The rename failed (another drive, most
                // often): copy, then remove. The two steps
                // report separately (round 3, R3-5): a copy
                // that landed IS a filing, even when the
                // inbox copy could not be removed, and the
                // office is told to delete it by hand.
                Err(_) => match std::fs::copy(path, &target) {
                    Ok(_) => match std::fs::remove_file(path) {
                        Ok(()) => format!("Filed as {name}."),
                        Err(_) => format!(
                            "Filed as {name}. The copy in the inbox could not be \
                             removed; delete it by hand so the part is not \
                             counted twice."
                        ),
                    },
                    Err(_) => "The words are saved in the bulletin, but the part \
                               file could not be moved to the filed folder; it \
                               is still in the inbox."
                        .to_string(),
                },
            },
            None => match std::fs::write(&target, &review.text) {
                Ok(()) => format!("Filed as {name}."),
                Err(_) => "The words are saved in the bulletin, but the pasted words \
                     could not be filed."
                    .to_string(),
            },
        }
    }

    /// The contribution review window (R-1): the same
    /// Accept/Edit/Cancel shape as every other review in
    /// the app. The first line always says which Sunday
    /// the words name; when that is not the open
    /// bulletin's Sunday, Accept arms instead of landing
    /// the words in the wrong week silently.
    pub(crate) fn contribution_review_window(&mut self, ctx: &egui::Context) {
        if self.contribution_review.is_none() {
            return;
        }
        use ob_extract::contribution::{
            format_date_long, keep_through_second_date_sentence,
            keep_through_second_dates_sentence, second_date_sentence, second_dates_sentence,
            week_verdict, WeekVerdict,
        };
        let bulletin_date = self
            .doc
            .fields
            .get("occasion_date")
            .and_then(|iso| ob_model::parse_flexible_date(iso, today_simple()));
        let mut review = self.contribution_review.take().expect("checked");
        let mut accept = false;
        let mut file_only = false;
        let mut cancel = false;
        let mut start_edit = false;
        let mut apply_edit = false;
        let mut back = false;
        egui::Window::new("Words for the bulletin")
            .collapsible(false)
            .resizable(true)
            .default_size([560.0, 520.0])
            .show(ctx, |ui| {
                if review.editing {
                    ui.label("Change the words here. They are read again when you use them, the same as when they arrived.");
                    ui.label("From");
                    ui.add(
                        egui::TextEdit::singleline(&mut review.edit_from)
                            .hint_text("A name or a job, like Pastor Lee"),
                    );
                    ui.label("For Sunday");
                    ui.add(
                        egui::TextEdit::singleline(&mut review.edit_for)
                            .hint_text("October 25, 2026"),
                    );
                    ui.label("Keep-through (announcements only; blank means one Sunday)");
                    ui.add(
                        egui::TextEdit::singleline(&mut review.edit_keep)
                            .hint_text("November 8, 2026"),
                    );
                    ui.separator();
                    egui::ScrollArea::vertical()
                        .max_height(260.0)
                        .show(ui, |ui| {
                            for i in 0..review.edit_parts.len() {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "## {}",
                                        review.edit_kinds[i].heading()
                                    ))
                                    .strong(),
                                );
                                ui.add(
                                    egui::TextEdit::multiline(&mut review.edit_parts[i])
                                        .desired_rows(4)
                                        .desired_width(f32::INFINITY),
                                );
                            }
                        });
                    if let Some(e) = &review.edit_error {
                        ui.label(egui::RichText::new(e.clone()).color(self.theme.problem));
                    }
                    ui.horizontal(|ui| {
                        if ui.button("Use the edited words").clicked() {
                            apply_edit = true;
                        }
                        if ui.button("Back").clicked() {
                            back = true;
                        }
                    });
                    return;
                }
                let claim = applied_claim(&self.doc, &review.contribution, &review.text);
                if claim == AppliedClaim::AlreadyIn {
                    // Round 3 (R3-5): these words are
                    // already in this bulletin. The first
                    // line says so, and the only answer
                    // left is to file the part: applying
                    // it again would print the words twice.
                    // Round 5 (R5-B): the claim is earned
                    // by the words being found in the
                    // bulletin, not by the record alone.
                    ui.label(egui::RichText::new(APPLIED_PART_FIRST_LINE).strong());
                    let sender = match (&review.contribution.from, &review.source) {
                        (Some(f), _) => format!("From: {f}"),
                        (None, Some(path)) => {
                            format!("No name given (the file is {}).", file_name_of(path))
                        }
                        (None, None) => "Pasted words; no name given.".to_string(),
                    };
                    ui.label(sender);
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("File it").clicked() {
                            file_only = true;
                        }
                        if ui.button("Edit").clicked() {
                            start_edit = true;
                        }
                        if ui.button("Cancel").clicked() {
                            cancel = true;
                        }
                    });
                    return;
                }
                if claim == AppliedClaim::RecordedButWordsGone {
                    // Round 5 (R5-B): the document's note
                    // says these words were added before,
                    // but they are not in the bulletin now.
                    // The first line says so plainly, and
                    // the ordinary review below offers
                    // Accept: the words can be added again.
                    ui.label(
                        egui::RichText::new(APPLIED_RECORD_MISSING_LINE)
                            .color(self.theme.warn)
                            .strong(),
                    );
                }
                let verdict = week_verdict(&review.contribution, bulletin_date);
                if let Some(mismatch) = &review.contribution.named_weekday_mismatch {
                    // Round 3 (RT3-D): the line named a day
                    // that does not exist. That is the first
                    // line, in place of the "do not say"
                    // words, which would misdescribe the
                    // file. There is no Sunday to arm
                    // against: Accept stays as it is, with
                    // this sentence in view.
                    ui.label(
                        egui::RichText::new(mismatch.sentence())
                            .color(self.theme.warn)
                            .strong(),
                    );
                } else {
                    match verdict {
                        WeekVerdict::DifferentSunday(said) => {
                            let open = bulletin_date.expect("verdict names both");
                            // Round 9 (R9-11/R9-10): a
                            // Sunday-after phrase whose
                            // parenthesized date disagrees
                            // with the relative resolution
                            // names the resolution and the
                            // parenthesized date, never one
                            // date on both sides.
                            let sentence = review
                                .contribution
                                .parenthesized_different_sentence(open)
                                .unwrap_or_else(|| {
                                    format!(
                                        "These words say they are for {}. The bulletin open is for {}.",
                                        format_date_long(said),
                                        format_date_long(open)
                                    )
                                });
                            ui.label(
                                egui::RichText::new(sentence)
                                    .color(self.theme.warn)
                                    .strong(),
                            );
                        }
                        WeekVerdict::SameSunday => {
                            // Round 9 (R9-11): a
                            // Sunday-after phrase whose
                            // parenthesized date is the
                            // bulletin's own date reads
                            // SameSunday on that explicit
                            // date, with the relative words
                            // noted beside it; no date was
                            // read from the words alone, so
                            // `for_date` may be None here.
                            let sentence = match review.contribution.for_date {
                                Some(d) => format!(
                                    "These words say they are for {}.",
                                    format_date_long(d)
                                ),
                                None => {
                                    let open = bulletin_date.expect("verdict names both");
                                    review
                                        .contribution
                                        .parenthesized_same_sunday_sentence(open)
                                        .unwrap_or_else(|| {
                                            format!(
                                                "These words say they are for {}.",
                                                format_date_long(open)
                                            )
                                        })
                                }
                            };
                            ui.label(sentence);
                        }
                        WeekVerdict::SundayInWords(open) => {
                            // Round 5 (RT5-6): the file
                            // named its Sunday in relative
                            // words only. The sentence says
                            // the words were read against
                            // this bulletin's date, in the
                            // sender's own phrasing. Round 6
                            // (R6-8): the day is named by
                            // the weekday the bulletin's
                            // date actually falls on, never
                            // the word Sunday for a date
                            // that is not one.
                            let phrase = review
                                .contribution
                                .for_sunday_in_words
                                .as_deref()
                                .unwrap_or("a Sunday");
                            let read_as = format_date_long(open);
                            ui.label(format!(
                                "These words name {phrase} in words; \
                                 read as this bulletin's date, {read_as}."
                            ));
                        }
                        WeekVerdict::NoSundayNamed => {
                            if let Some(d) = review.contribution.for_date {
                                ui.label(format!(
                                    "These words say they are for {}.",
                                    format_date_long(d)
                                ));
                            } else if let Some(phrase) =
                                &review.contribution.for_sunday_in_words
                            {
                                // Round 5 (RT5-6): with no
                                // bulletin Sunday to read the
                                // words against, the sentence
                                // still admits the file named
                                // a Sunday in words.
                                ui.label(format!(
                                    "These words name {phrase} in words, but this \
                                     bulletin does not say which Sunday it is for."
                                ));
                            } else if let Some(phrase) =
                                &review.contribution.for_sunday_after_in_words
                            {
                                // Round 6 (RT6-9): the file
                                // named a Sunday in words
                                // after another Sunday; with
                                // no bulletin date to read
                                // the words against, the
                                // sentence admits that too,
                                // rather than claiming no
                                // Sunday was named.
                                ui.label(format!(
                                    "These words name {phrase} in words, but this \
                                     bulletin does not say which Sunday it is for."
                                ));
                            } else {
                                ui.label("These words do not say which Sunday they are for.");
                            }
                        }
                    }
                }
                if let Some(mismatch) = &review.contribution.keep_through_weekday_mismatch {
                    // Round 4 (R4-7): the Keep-through line
                    // named an impossible day too. The date
                    // did not apply (the announcement runs
                    // one Sunday), and the office hears why
                    // instead of the date vanishing quietly.
                    ui.label(
                        egui::RichText::new(mismatch.keep_through_sentence())
                            .color(self.theme.warn),
                    );
                }
                if let Some(mismatch) = &review.contribution.for_sunday_parenthesized_mismatch {
                    // Round 9 (R9-12): a trailing
                    // parenthesized date on the For Sunday
                    // line disagreed with the date the line
                    // read. The line's date governs, and
                    // the office hears about the
                    // disagreement.
                    ui.label(
                        egui::RichText::new(mismatch.sentence()).color(self.theme.warn),
                    );
                }
                if let Some(mismatch) = &review.contribution.keep_through_parenthesized_mismatch {
                    // Round 9 (R9-9/R9-12): the same
                    // disagreement on the Keep-through
                    // line, including stacked parenthesized
                    // groups that disagree with each other.
                    ui.label(
                        egui::RichText::new(mismatch.keep_through_sentence())
                            .color(self.theme.warn),
                    );
                }
                if !review.contribution.for_sunday_second_dates.is_empty() {
                    // Round 9 (R9-13) / round 10 (R10-4,
                    // R10-6): the For Sunday line named
                    // other absolute dates, before the read
                    // date or after it. The read date still
                    // governs; the note names every distinct
                    // other date in the file.
                    let sentence =
                        second_dates_sentence(&review.contribution.for_sunday_second_dates);
                    ui.label(egui::RichText::new(sentence).color(self.theme.warn));
                } else if let Some(second) = review.contribution.for_sunday_second_date {
                    let sentence = second_date_sentence(second);
                    ui.label(egui::RichText::new(sentence).color(self.theme.warn));
                }
                if !review.contribution.keep_through_second_dates.is_empty() {
                    // Round 10 (R10-4, R10-6): the same
                    // note for the Keep-through line,
                    // naming every distinct second date.
                    let sentence = keep_through_second_dates_sentence(
                        &review.contribution.keep_through_second_dates,
                    );
                    ui.label(egui::RichText::new(sentence).color(self.theme.warn));
                } else if let Some(second) = review.contribution.keep_through_second_date {
                    let sentence = keep_through_second_date_sentence(second);
                    ui.label(egui::RichText::new(sentence).color(self.theme.warn));
                }
                let sender = match (&review.contribution.from, &review.source) {
                    (Some(f), _) => format!("From: {f}"),
                    (None, Some(path)) => {
                        format!("No name given (the file is {}).", file_name_of(path))
                    }
                    (None, None) => "Pasted words; no name given.".to_string(),
                };
                ui.label(sender);
                ui.separator();
                let plans = plan_parts(&self.doc, &review.contribution);
                egui::ScrollArea::vertical()
                    .max_height(280.0)
                    .show(ui, |ui| {
                        for (part, plan) in review.contribution.parts.iter().zip(plans.iter()) {
                            ui.label(egui::RichText::new(&plan.line).strong());
                            for p in &part.paragraphs {
                                ui.small(p.text());
                            }
                            ui.add_space(6.0);
                        }
                    });
                ui.separator();
                let different = matches!(verdict, WeekVerdict::DifferentSunday(_));
                // Round 6 (R6-8): a Sunday named only in
                // relative words arms too — its reading
                // depends entirely on which bulletin is
                // open, so the first click shows the
                // reading (the first line above) and the
                // second confirms, like DifferentSunday.
                let in_words = matches!(verdict, WeekVerdict::SundayInWords(_));
                let guarded = different || in_words;
                if different && review.armed {
                    ui.label("One more click, and the words go into this Sunday's bulletin, not the Sunday they name.");
                }
                if in_words && review.armed {
                    ui.label("One more click, and the words go into this bulletin, read as the date above.");
                }
                ui.horizontal(|ui| {
                    let accept_label = if different && !review.armed {
                        "Use it for this Sunday anyway"
                    } else if different {
                        "Click again to use it for this Sunday"
                    } else if in_words && review.armed {
                        "Click again to use these words"
                    } else {
                        "Use these words"
                    };
                    if ui.button(accept_label).clicked() {
                        if guarded && !review.armed {
                            review.armed = true;
                        } else {
                            accept = true;
                        }
                    }
                    if ui.button("Edit").clicked() {
                        start_edit = true;
                    }
                    if ui.button("Cancel").clicked() {
                        cancel = true;
                    }
                });
            });
        if cancel {
            self.note("Nothing was added.".into());
            return;
        }
        if accept {
            self.contribution_review = Some(review);
            self.accept_contribution();
            return;
        }
        if file_only {
            self.contribution_review = Some(review);
            self.file_applied_contribution();
            return;
        }
        if back {
            review.editing = false;
            review.edit_error = None;
        }
        self.contribution_review = Some(review);
        if start_edit {
            self.begin_contribution_edit();
        }
        if apply_edit {
            self.apply_contribution_edit();
        }
    }

    /// The paste box (R-1): words someone sent, pasted in
    /// as they came, read by the same parser and reviewed
    /// in the same window as an inbox file.
    pub(crate) fn paste_window(&mut self, ctx: &egui::Context) {
        if !self.show_paste {
            return;
        }
        let mut read = false;
        let mut close = false;
        egui::Window::new("Paste words someone sent")
            .collapsible(false)
            .resizable(true)
            .default_size([520.0, 420.0])
            .show(ctx, |ui| {
                ui.label("Paste the words here, just as they came: an email, a message, a note typed on a phone. Words in the sender's card shape slot into their places; plain words come in as an announcement.");
                ui.add(
                    egui::TextEdit::multiline(&mut self.paste_text)
                        .desired_rows(12)
                        .desired_width(f32::INFINITY),
                );
                ui.horizontal(|ui| {
                    if ui.button("Read these words").clicked() {
                        read = true;
                    }
                    if ui.button("Cancel").clicked() {
                        close = true;
                    }
                });
            });
        if close {
            self.show_paste = false;
        }
        if read {
            if self.contribution_review.is_some() {
                self.note("Finish the part you are reviewing first; these words stay here.".into());
                return;
            }
            match ob_extract::contribution::parse_contribution(&self.paste_text, today_simple()) {
                Ok(c) => {
                    let text = self.paste_text.clone();
                    self.paste_text.clear();
                    self.show_paste = false;
                    self.open_contribution_review(c, None, text);
                }
                Err(_) => {
                    self.fail(
                        "Those words have no readable words in them. Paste the \
                         whole message and try again."
                            .into(),
                    );
                }
            }
        }
    }

    /// Print the sender's card: one page that teaches the
    /// whole contribution shape, written as a PDF where
    /// the office picks. The card is the training program.
    pub(crate) fn print_sender_card(&mut self) {
        let md = sender_card_markdown(&self.doc.profile.name);
        let doc = ob_model::markdown::document_from_markdown(
            &md,
            self.doc.profile.clone(),
            "Sender's card",
        );
        if let Some(path) = rfd::FileDialog::new()
            .set_file_name("Sender's card.pdf")
            .add_filter("PDF", &["pdf"])
            .save_file()
        {
            let bytes = ob_pdf::export_reader_pdf(&doc, ob_pdf::ExportPurpose::ForPrinting);
            match write_file_atomically(&path, &bytes) {
                Ok(()) => self.note(format!(
                    "Sender's card written: {}. Print it and hand it to anyone who sends words.",
                    file_name_of(&path)
                )),
                Err(e) => self.fail(format!("Could not write the sender's card: {e}")),
            }
        }
    }

    pub(crate) fn import_review_window(&mut self, ctx: &egui::Context) {
        if self.import_review.is_none() {
            return;
        }
        let mut choice: Option<bool> = None; // Some(edit) = use it
        let mut cancel = false;
        let dirty = self.frame_snapshot != self.saved_snapshot;
        egui::Window::new("Your old bulletin, as a draft")
            .collapsible(false)
            .resizable(true)
            .default_size([560.0, 520.0])
            .show(ctx, |ui| {
                let review = self.import_review.as_ref().expect("review");
                let (mut good, mut check, mut pics) = (0, 0, 0);
                for item in &review.items {
                    match item.badge {
                        ob_extract::draft::Badge::Matched => good += 1,
                        ob_extract::draft::Badge::CheckThis => check += 1,
                        ob_extract::draft::Badge::PictureOnly => pics += 1,
                    }
                }
                ui.label(format!(
                    "We read {} and rebuilt it as a draft bulletin:                      {good} pieces came over cleanly, {check} need your eye,                      and {pics} pages are pictures only. Nothing is final until                      you say so, and the old pages sit under the new ones as                      pictures to check against.",
                    review.source_name
                ));
                if let Some(note) = &review.note {
                    ui.label(note);
                }
                ui.separator();
                egui::ScrollArea::vertical()
                    .max_height(300.0)
                    .show(ui, |ui| {
                        for item in &review.items {
                            let (tag, color) = match item.badge {
                                ob_extract::draft::Badge::Matched => {
                                    ("Good", self.theme.good)
                                }
                                ob_extract::draft::Badge::CheckThis => {
                                    ("Check this", self.theme.warn)
                                }
                                ob_extract::draft::Badge::PictureOnly => {
                                    ("Picture only", self.theme.info)
                                }
                            };
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(tag).color(color).strong(),
                                );
                                ui.label(format!("Page {}: {}", item.panel + 1, item.label));
                            });
                            ui.small(&item.detail);
                        }
                    });
                ui.separator();
                if dirty && !self.import_armed {
                    ui.label(
                        "You have unsaved work on screen. Using this draft leaves it behind.",
                    );
                }
                ui.horizontal(|ui| {
                    let use_label = if dirty && !self.import_armed {
                        "Leave unsaved work and use this draft"
                    } else if dirty {
                        "Yes, leave it and use the draft"
                    } else {
                        "Use this draft"
                    };
                    if ui.button(use_label).clicked() {
                        if dirty && !self.import_armed {
                            self.import_armed = true;
                        } else {
                            choice = Some(false);
                        }
                    }
                    let arrange_label = if dirty && self.import_armed {
                        "Yes, leave it and arrange the pages"
                    } else {
                        "Use it and arrange the pages"
                    };
                    if ui.button(arrange_label).clicked() {
                        if dirty && !self.import_armed {
                            self.import_armed = true;
                        } else {
                            choice = Some(true);
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        cancel = true;
                    }
                });
            });
        if let Some(edit) = choice {
            self.finalize_import(edit);
        } else if cancel {
            if let Some(review) = self.import_review.take() {
                if let Some(wd) = &review.workdir {
                    let _ = std::fs::remove_dir_all(wd);
                }
            }
            self.import_armed = false;
            self.note("Import cancelled; nothing changed.".into());
        }
    }

    /// The Template Analyzer on a worker: turn every picked
    /// bulletin into a dump through its own reader (the pick
    /// may mix PDFs, Publisher files, InDesign files, Scribus
    /// files, and OpenBulletin files), learn the family and
    /// its standing text, and render the base bulletin's pages
    /// as tracing pictures when the base can be rendered.
    /// Files that cannot be read are skipped and named; if
    /// what survives no longer meets the learning minimums
    /// (ob_extract::learn), the run stops and says what to add.
    pub(crate) fn start_analyzer(&mut self, files: Vec<std::path::PathBuf>) {
        let profile = self.doc.profile.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        self.analyzer_rx = Some(rx);
        self.note("Studying your old bulletins...".into());
        std::thread::spawn(move || {
            let result = (|| -> Result<AnalyzerReview, String> {
                use ob_extract::learn::LearnKind;
                let helper = ob_extract::helper::resolve_helper(None);
                let workdir = std::env::temp_dir()
                    .join(format!("openbulletin-template-{}", std::process::id()));
                let _ = std::fs::remove_dir_all(&workdir);
                let soffice = ob_extract::pub_bridge::find_soffice();
                // Each read bulletin: its dump, plus the PDF it
                // can be pictured from, when it has one.
                let mut entries: Vec<(ob_extract::ExtractionDump, Option<std::path::PathBuf>)> =
                    Vec::new();
                let mut skipped: Vec<String> = Vec::new();
                let mut spoken_left_out = 0usize;
                let mut web_left_out = 0usize;
                for path in &files {
                    let name = path
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| path.display().to_string());
                    let read: Result<
                        (ob_extract::ExtractionDump, Option<std::path::PathBuf>),
                        String,
                    > = match ob_extract::learn::classify_path(path) {
                        Some(LearnKind::Pdf) => ob_extract::helper::dump_pdf(&helper, path)
                            .map(|d| (d, Some(path.clone()))),
                        Some(LearnKind::Publisher) => match &soffice {
                            Some(so) => ob_extract::pub_bridge::convert_pub_to_pdf(
                                so,
                                path,
                                &workdir.join("pub-pdf"),
                            )
                            .and_then(|pdf| {
                                ob_extract::helper::dump_pdf(&helper, &pdf).map(|d| (d, Some(pdf)))
                            }),
                            None => (|| {
                                let bytes = ob_extract::read_open_door_file(path)?;
                                let size = bytes.len();
                                let cfb = ob_extract::cfb::Cfb::open(bytes)?;
                                let x = ob_extract::pub_layout::extract_pub_positioned(
                                    &cfb, &name, size,
                                )?;
                                Ok((x.dump, None))
                            })(),
                        },
                        Some(LearnKind::Indesign) => ob_extract::read_open_door_file(path)
                            .and_then(|bytes| ob_extract::idml::dump_from_idml(&bytes, &name))
                            .map(|d| (d, None)),
                        Some(LearnKind::Scribus) => ob_extract::read_open_door_file(path)
                            .and_then(|bytes| ob_extract::sla::dump_from_sla(&bytes, &name))
                            .map(|d| (d, None)),
                        Some(LearnKind::OpenBulletin) => ob_format::load(path).map(|doc| {
                            let (dump, left_out) =
                                ob_extract::learn::dump_from_document(&doc, &name);
                            spoken_left_out += left_out.spoken_only;
                            web_left_out += left_out.web_hidden;
                            (dump, None)
                        }),
                        None => Err("not a kind of file we can learn from".into()),
                    };
                    match read {
                        Ok(entry) => entries.push(entry),
                        Err(_) => skipped.push(name),
                    }
                }
                let dumps: Vec<ob_extract::ExtractionDump> =
                    entries.iter().map(|(d, _)| d.clone()).collect();
                let counts = ob_extract::learn::counts_of_dumps(&dumps);
                if counts.effective() < ob_extract::learn::MIN_BULLETINS {
                    return Err(ob_extract::learn::shortfall_after_reading(
                        &counts, &skipped,
                    ));
                }
                let out =
                    ob_extract::analyzer::analyze(&dumps, profile, &current_library_families())?;
                if let Some((_, Some(pdf))) = entries
                    .iter()
                    .find(|(d, _)| d.source_name == out.base_source)
                {
                    let _ = ob_extract::helper::render_pdf(&helper, pdf, &workdir.join("assets"));
                }
                // Spoken-only words left out of learning are
                // said on the mix line, not left silent
                // (audit Batch 1).
                let mut mix_line = ob_extract::learn::mix_summary(&counts);
                if spoken_left_out > 0 {
                    mix_line.push(' ');
                    mix_line.push_str(&ob_extract::learn::spoken_only_left_out_line(
                        spoken_left_out,
                    ));
                }
                if web_left_out > 0 {
                    mix_line.push(' ');
                    mix_line.push_str(&ob_extract::learn::web_hidden_left_out_line(web_left_out));
                }
                // The review's note line carries the family's
                // own news first (too few alike to learn what
                // stands still, round 2 R2-1), then any files
                // that could not be read.
                let mut notes: Vec<String> = Vec::new();
                if let Some(n) = &out.family_note {
                    notes.push(n.clone());
                }
                if !skipped.is_empty() {
                    notes.push(format!(
                        "{} file(s) could not be read and were left out: {}",
                        skipped.len(),
                        skipped.join(", ")
                    ));
                }
                Ok(AnalyzerReview {
                    doc: out.doc,
                    items: out.items,
                    family: out.family,
                    corpus: out.corpus,
                    mix: mix_line,
                    workdir,
                    note: if notes.is_empty() {
                        None
                    } else {
                        Some(notes.join(" "))
                    },
                })
            })();
            let _ = tx.send(result);
        });
    }

    pub(crate) fn poll_analyzer(&mut self, ctx: &egui::Context) {
        let Some(rx) = &self.analyzer_rx else { return };
        match rx.try_recv() {
            Ok(Ok(review)) => {
                self.analyzer_rx = None;
                self.analyzer_armed = false;
                self.analyzer_review = Some(review);
                ctx.request_repaint();
            }
            Ok(Err(e)) => {
                self.analyzer_rx = None;
                self.fail(format!("The old bulletins could not be studied. {e}"));
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.analyzer_rx = None;
                self.fail("The old bulletins could not be studied; the reader stopped.".into());
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
        }
    }

    pub(crate) fn finalize_analyzer(&mut self, edit: bool) {
        let Some(review) = self.analyzer_review.take() else {
            return;
        };
        self.analyzer_armed = false;
        self.asset_base = Some(review.workdir.clone());
        self.adopt_document(review.doc, true);
        self.path = None;
        self.print_fingerprint = None;
        self.checked_fingerprint = None;
        if edit {
            self.show_pages = true;
            self.note(
                "Here is the suggested template. Standing words stay each week; \
                 weekly words clear on Next Sunday."
                    .into(),
            );
        } else {
            self.step = 1;
            self.note(
                "Template suggested from your old bulletins. Set the date, and \
                 Save it as your church template."
                    .into(),
            );
        }
    }

    pub(crate) fn analyzer_review_window(&mut self, ctx: &egui::Context) {
        if self.analyzer_review.is_none() {
            return;
        }
        let mut choice: Option<bool> = None;
        let mut cancel = false;
        let dirty = self.frame_snapshot != self.saved_snapshot;
        egui::Window::new("A template from your old bulletins")
            .collapsible(false)
            .resizable(true)
            .default_size([560.0, 520.0])
            .show(ctx, |ui| {
                let review = self.analyzer_review.as_ref().expect("review");
                let standing = review.items.iter().filter(|i| i.standing).count();
                ui.label(format!(
                    "We studied {} bulletins. {} of them look like one family,                      and this suggestion is built from the fullest of those.                      {standing} pieces stand still every week; the rest change                      and become the slots Next Sunday clears.",
                    review.corpus,
                    review.family,
                ));
                ui.label(&review.mix);
                if let Some(note) = &review.note {
                    ui.label(
                        egui::RichText::new(note)
                            .color(self.theme.problem),
                    );
                }
                ui.separator();
                egui::ScrollArea::vertical()
                    .max_height(300.0)
                    .show(ui, |ui| {
                        for item in &review.items {
                            let (tag, color) = if item.standing {
                                ("Stays", self.theme.good)
                            } else {
                                ("Changes weekly", self.theme.warn)
                            };
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(tag).color(color).strong());
                                ui.label(&item.label);
                            });
                            ui.small(&item.detail);
                        }
                    });
                ui.separator();
                if dirty && !self.analyzer_armed {
                    ui.label(
                        "You have unsaved work on screen. Using this template leaves it behind.",
                    );
                }
                ui.horizontal(|ui| {
                    let use_label = if dirty && !self.analyzer_armed {
                        "Leave unsaved work and use this template"
                    } else if dirty {
                        "Yes, leave it and use the template"
                    } else {
                        "Use this template"
                    };
                    if ui.button(use_label).clicked() {
                        if dirty && !self.analyzer_armed {
                            self.analyzer_armed = true;
                        } else {
                            choice = Some(false);
                        }
                    }
                    let arrange_label = if dirty && self.analyzer_armed {
                        "Yes, leave it and arrange the pages"
                    } else {
                        "Use it and arrange the pages"
                    };
                    if ui.button(arrange_label).clicked() {
                        if dirty && !self.analyzer_armed {
                            self.analyzer_armed = true;
                        } else {
                            choice = Some(true);
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        cancel = true;
                    }
                });
            });
        if let Some(edit) = choice {
            self.finalize_analyzer(edit);
        } else if cancel {
            if let Some(review) = self.analyzer_review.take() {
                let _ = std::fs::remove_dir_all(&review.workdir);
            }
            self.analyzer_armed = false;
            self.note("Template cancelled; nothing changed.".into());
        }
    }

    /// Open the Pages window on a named panel with a named
    /// frame selected: where the Check step's "Fix this"
    /// buttons land the user (UI audit round 2). Indices
    /// are clamped to the document as it stands now.
    pub(crate) fn open_pages_at(&mut self, panel: usize, frame: Option<usize>) {
        self.commit_if_editing();
        if self.doc.panels.is_empty() {
            self.show_pages = true;
            return;
        }
        self.canvas_panel = panel.min(self.doc.panels.len() - 1);
        self.selected_frame =
            frame.filter(|fi| *fi < self.doc.panels[self.canvas_panel].frames.len());
        self.guide_menu_for = None;
        self.frame_menu_for = None;
        self.show_pages = true;
    }

    /// Move the Pages window by delta pages, with the same
    /// care the pager buttons take (commit typing first, drop
    /// indices into the old page).
    pub(crate) fn page_by(&mut self, delta: i32) {
        let target = self.canvas_panel as i32 + delta;
        if target < 0 || target as usize >= self.doc.panels.len() {
            return;
        }
        self.commit_if_editing();
        self.canvas_panel = target as usize;
        self.selected_frame = None;
        self.guide_menu_for = None;
        self.frame_menu_for = None;
    }

    /// The Open button's work, shared with its key. One
    /// door, two entrances: Open offers every kind the
    /// Start step's old-bulletin door opens, routed the
    /// same way (UI audit round 2). The guard runs before
    /// the dialog here, as it always has; a pick the door
    /// below is holding opens straight away.
    pub(crate) fn open_flow(&mut self) {
        if let Some(path) = self.pending_path.take() {
            self.open_old_bulletin(path);
            return;
        }
        if !self.open_guard(PendingOpen::Plain) {
            return;
        }
        if let Some(path) = rfd::FileDialog::new()
            .add_filter(
                "Old bulletins",
                &[
                    "pdf", "pub", "idml", "sla", "obd", "md", "html", "htm", "musicxml", "mxl",
                ],
            )
            .add_filter("All files", &[] as &[&str])
            .pick_file()
        {
            self.route_old_bulletin(path);
        }
    }

    /// The one export door for other programs (Jacob's
    /// rule: one button, the format chosen in the save
    /// dialog, never a button per format). Dispatches by the
    /// ending the user chose, so a new format joins by
    /// adding one arm here. The confirmation keeps the
    /// export report's plain-words notes: what was saved
    /// and what the copy could not carry.
    /// The large-print copy (beyond-print.md section 4):
    /// the same words at 17pt or bigger, one Letter page at
    /// a time. Not the print copy: it never touches the
    /// print fingerprint, and its status line says what it
    /// is, with the usual counts beside it.
    pub(crate) fn save_large_print_copy(&mut self, path: &std::path::Path) {
        let (bytes, report) = ob_pdf::export_large_print_pdf_report_with_cache(
            &self.doc,
            ob_pdf::ExportPurpose::ForPrinting,
            &mut self.export_image_cache,
        );
        match write_file_atomically(path, &bytes) {
            Ok(()) => {
                self.status = format!(
                    "Large-print copy written: {}, {}, for printing.{}{}",
                    file_name_of(path),
                    human_file_size(bytes.len()),
                    missing_pictures_sentence(&report),
                    spoken_only_sentence(&report)
                );
            }
            Err(e) => self.fail(format!("Could not save the large-print copy: {e}")),
        }
    }

    pub(crate) fn save_copy_for_program(&mut self, path: &std::path::Path) {
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        match ext.as_str() {
            "idml" => match ob_extract::idml_export::export_idml(&self.doc) {
                Ok((bytes, report)) => match write_file_atomically(path, &bytes) {
                    Ok(()) => {
                        let mut msg =
                            format!("Saved a copy for InDesign as {}.", file_name_of(path));
                        if report.notes.is_empty() {
                            msg.push_str(" Everything in the bulletin came over.");
                        } else {
                            for n in &report.notes {
                                msg.push(' ');
                                msg.push_str(n);
                            }
                        }
                        self.export_note = Some(msg.clone());
                        self.note(msg);
                    }
                    Err(e) => self.fail(format!("Could not save that copy: {e}")),
                },
                Err(e) => self.fail(format!("Could not make that copy: {e}")),
            },
            "sla" => match ob_extract::sla_export::export_sla(&self.doc) {
                Ok((bytes, report)) => match write_file_atomically(path, &bytes) {
                    Ok(()) => {
                        let mut msg =
                            format!("Saved a copy for Scribus as {}.", file_name_of(path));
                        if report.notes.is_empty() {
                            msg.push_str(" Everything in the bulletin came over.");
                        } else {
                            for n in &report.notes {
                                msg.push(' ');
                                msg.push_str(n);
                            }
                        }
                        self.export_note = Some(msg.clone());
                        self.note(msg);
                    }
                    Err(e) => self.fail(format!("Could not save that copy: {e}")),
                },
                Err(e) => self.fail(format!("Could not make that copy: {e}")),
            },
            "pdf" => {
                // The finished pages, made for the purpose the
                // radio above the button remembers (O1).
                let purpose = self.settings.pdf_purpose();
                let (bytes, report) = ob_pdf::export_reader_pdf_report(&self.doc, purpose);
                match write_file_atomically(path, &bytes) {
                    Ok(()) => {
                        let msg = format!(
                            "Saved a copy as {}, {}, {}.{}{}",
                            file_name_of(path),
                            human_file_size(bytes.len()),
                            purpose.words(),
                            missing_pictures_sentence(&report),
                            spoken_only_sentence(&report)
                        );
                        self.export_note = Some(msg.clone());
                        self.note(msg);
                    }
                    Err(e) => self.fail(format!("Could not save that copy: {e}")),
                }
            }
            "md" => {
                // The words as markdown (G1b): headings from
                // the style names, bold and italic from the
                // runs, field words filled in. Spoken-only
                // words never publish, markdown included.
                let (text, spoken_left_out) = ob_model::markdown::to_markdown_report(&self.doc);
                match write_file_atomically(path, text.as_bytes()) {
                    Ok(()) => {
                        let msg = format!(
                            "Saved the words as {}, for a website or newsletter.{}",
                            file_name_of(path),
                            ob_model::spoken_only_left_out_sentence(spoken_left_out)
                        );
                        self.export_note = Some(msg.clone());
                        self.note(msg);
                    }
                    Err(e) => self.fail(format!("Could not save that copy: {e}")),
                }
            }
            "html" | "htm" => {
                // The web bulletin (H1): one self-contained
                // page, the words in reading order, the
                // pictures carried inside it, the credits it
                // owes at the end. It takes the house name:
                // the picked name with "-web" on it.
                match ob_extract::html_export::export_html(&self.doc) {
                    Ok((bytes, report)) => {
                        let target = web_page_path(path);
                        match write_file_atomically(&target, &bytes) {
                            Ok(()) => {
                                let mut msg = format!(
                                    "Saved the web page as {}, {}.",
                                    file_name_of(&target),
                                    human_file_size(bytes.len())
                                );
                                if report.missing_pictures == 1 {
                                    msg.push_str(
                                        " 1 picture box had no picture to carry; the page says so where it belongs.",
                                    );
                                } else if report.missing_pictures > 1 {
                                    msg.push_str(&format!(
                                        " {} picture boxes had no picture to carry; the page says so where they belong.",
                                        report.missing_pictures
                                    ));
                                }
                                msg.push_str(&ob_model::spoken_only_left_out_sentence(
                                    report.spoken_only_left_out,
                                ));
                                msg.push_str(&ob_model::web_hidden_left_out_sentence(
                                    report.web_hidden_left_out,
                                ));
                                msg.push_str(&ob_extract::html_export::children_left_out_sentence(
                                    report.children_left_out,
                                ));
                                self.export_note = Some(msg.clone());
                                self.note(msg);
                            }
                            Err(e) => self.fail(format!("Could not save that copy: {e}")),
                        }
                    }
                    Err(e) => self.fail(format!("Could not make that copy: {e}")),
                }
            }
            _ => self.fail(
                "That kind of copy is not one we can make yet. The save dialog offers InDesign (IDML), Scribus, PDF, Markdown, and a Web page."
                    .into(),
            ),
        }
    }

    /// Gather tracing pictures that panels still reference
    /// by file path into the document's asset store, so a
    /// v1 save carries their bytes. A reference that cannot
    /// be read anywhere it might live stays a path, and the
    /// panel keeps its missing-picture note, as before.
    pub(crate) fn absorb_tracing_pictures(&mut self, target_dir: Option<&std::path::Path>) {
        // Delegate to the fenced gather in ob-format
        // (HF-1): a received document's reference must not
        // pull arbitrary local files into a bulletin that
        // travels to other churches. One call per candidate
        // base, in the existing priority order; a panel
        // absorbed by an earlier call names an asset
        // afterwards, so later calls skip it.
        if let Some(dir) = self
            .path
            .as_ref()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        {
            ob_format::absorb_sidecar_pictures_in(&mut self.doc, &dir);
        }
        if let Some(dir) = target_dir {
            ob_format::absorb_sidecar_pictures_in(&mut self.doc, dir);
        }
        if let Some(base) = self.asset_base.clone() {
            ob_format::absorb_sidecar_pictures_in(&mut self.doc, &base);
        }
    }

    /// The Save button's work, shared with its key.
    /// Answers whether the bulletin was actually saved:
    /// the contribution seam files a part only after the
    /// bulletin carrying it is safely on disk.
    pub(crate) fn save_flow(&mut self) -> bool {
        // A save writes the bulletin whole: a loose sheet
        // on the canvas goes back first, so no file ever
        // holds the inside-out shape (R-3).
        self.leave_insert_editing();
        self.commit_if_editing();
        self.save_announcements();
        self.pin_fonts();
        // A "replace it anyway" answer arms exactly one
        // save attempt, whichever file it lands on.
        let replace_ok = self.replace_ok.take();
        let target = self.path.clone().or_else(|| {
            rfd::FileDialog::new()
                .set_file_name(suggested_save_name(&self.doc))
                .add_filter("OpenBulletin bulletin", &["obd"])
                .save_file()
        });
        if let Some(path) = target {
            // The changed-on-disk guard (R-1): if the file
            // at the save target is not the file we opened
            // or last saved (another program, a sync tool, a
            // second copy of this app changed it), stop and
            // say so, with the two ways out under the status
            // line. A deleted file is not a conflict.
            let stamped = self
                .disk_stamp
                .as_ref()
                .filter(|(p, _)| p == &path)
                .map(|(_, s)| s.clone());
            if let Some(remembered) = stamped {
                if replace_ok.as_deref() != Some(path.as_path()) {
                    let now = stamp_of(&path);
                    if stamp_verdict(&remembered, now.as_ref()) == StampVerdict::Changed {
                        self.save_conflict = Some(path);
                        self.replace_armed = false;
                        self.fail(
                            "This file changed on disk since you opened it. \
                             Saving would replace those changes."
                                .into(),
                        );
                        return false;
                    }
                }
            }
            // v1 packaging: tracing pictures still named by
            // a file path move INTO the document before the
            // save, so the .obd carries them (docs/FORMAT.md).
            self.absorb_tracing_pictures(path.parent());
            match ob_format::save(&self.doc, &path) {
                Ok(()) => {
                    self.note(format!("Saved as {}", file_name_of(&path)));
                    // An import's page pictures move beside the
                    // file on first Save.
                    if let Some(base) = self.asset_base.take() {
                        if let Some(parent) = path.parent() {
                            let from = base.join("assets");
                            let to = parent.join("assets");
                            if from.exists() && from != to {
                                let _ = std::fs::create_dir_all(&to);
                                if let Ok(rd) = std::fs::read_dir(&from) {
                                    for e in rd.flatten() {
                                        let _ = std::fs::copy(e.path(), to.join(e.file_name()));
                                    }
                                }
                            }
                        }
                    }
                    self.remember_bulletin(&path);
                    self.path = Some(path);
                    // The state memory follows the document to
                    // its (possibly new) name (J6).
                    self.store_doc_state();
                    self.mark_clean();
                    // The disk now holds what we wrote: the
                    // save guard's stamp follows (R-1).
                    self.note_disk_stamp();
                    // A saved bulletin is a confirmed one:
                    // its hymn pairs join the church's hymn
                    // memory (R-2a).
                    self.learn_hymns_from_current();
                    return true;
                }
                Err(e) => self.fail(format!(
                    "Could not save the file. Try your Documents folder. ({e})"
                )),
            }
        }
        false
    }

    /// Feed the open document's hymn pairs to the church's
    /// hymn memory (R-2a), and write the memory back when
    /// it learned something. Its one call site is a
    /// successful save of a weekly bulletin: that is the
    /// only moment a document is confirmed with a date
    /// the memory can learn pairs for. Accepting a learned
    /// template teaches nothing, by design (round 3,
    /// R3-12): a template spans many Sundays and carries
    /// no occasion date of its own. (An earlier version
    /// called this on template accept too, where it could
    /// never learn a thing, and said it could.)
    pub(crate) fn learn_hymns_from_current(&mut self) {
        if learn_hymns(&mut self.hymn_memory, &self.doc) {
            self.persist_hymn_memory();
        }
    }

    /// Write the hymn memory beside the settings. Best
    /// effort, like the settings themselves: a memory that
    /// will not write never blocks the work it came from.
    /// The write is atomic and durable, the settings pattern
    /// (round 3, R3-12; HF-5): a crash mid-write leaves the
    /// old memory whole rather than a truncated file that
    /// parses as nothing and silently starts the memory over.
    /// A failure is spoken once per session (S-MIN-2) — the
    /// old code swallowed even a failed rename in silence —
    /// and then stays quiet, like the journal's failures.
    pub(crate) fn persist_hymn_memory(&mut self) {
        let outcome = serde_json::to_string_pretty(&self.hymn_memory)
            .map_err(|e| e.to_string())
            .and_then(|text| {
                write_file_atomically(&hymn_memory_path(), text.as_bytes())
                    .map_err(|e| e.to_string())
            });
        if outcome.is_err() && !self.hymn_memory_failure_noted {
            self.hymn_memory_failure_noted = true;
            self.fail(
                "The hymn memory could not be written, so hymn pairings learned \
                 this time will not be remembered. Your bulletin itself is saved \
                 as usual."
                    .into(),
            );
        }
    }

    /// Remember the on-disk state of the file now open, as
    /// this app last saw it (opened it, or saved it): the
    /// save guard compares against this stamp. With no file
    /// open, there is nothing to remember.
    pub(crate) fn note_disk_stamp(&mut self) {
        self.disk_stamp = self
            .path
            .as_ref()
            .and_then(|p| stamp_of(p).map(|s| (p.clone(), s)));
        self.save_conflict = None;
    }

    /// The save guard's "save as a new file instead" way
    /// out: the same save flow, but the dated-name dialog
    /// chooses the target, so the changed file on disk is
    /// left exactly as it is.
    pub(crate) fn save_as_new_file(&mut self) {
        self.save_conflict = None;
        self.replace_armed = false;
        let shelved = self.path.take();
        let saved = self.save_flow();
        if !saved && self.path.is_none() {
            // The dialog was cancelled or the save failed:
            // the bulletin still belongs to its old file.
            self.path = shelved;
        }
    }

    /// The binding currently in force for an action.
    pub(crate) fn binding_for(&self, action: &str) -> Option<KeyBinding> {
        effective_keymap(&self.settings)
            .get(action)
            .and_then(|t| parse_binding(t))
    }

    /// Was this action's key just pressed?
    pub(crate) fn action_pressed(&self, ctx: &egui::Context, action: &str) -> bool {
        match self.binding_for(action) {
            Some(b) => ctx.input(|i| binding_pressed(i, &b)),
            None => false,
        }
    }

    /// While a key is being captured for the Keys window, the
    /// next key press becomes that action's key. If another
    /// action held that key, they swap: no key ever does two
    /// jobs, and none is silently lost.
    pub(crate) fn capture_key(&mut self, ctx: &egui::Context) {
        let Some(action) = self.key_capture.clone() else {
            return;
        };
        let found = ctx.input(|i| {
            i.events.iter().find_map(|e| match e {
                egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } => Some((*key, *modifiers)),
                _ => None,
            })
        });
        let Some((key, modifiers)) = found else {
            return;
        };
        if key == egui::Key::Escape {
            self.key_capture = None;
            return;
        }
        if key_name(key).is_none() {
            return; // not a key we can bind; keep waiting
        }
        let new_binding = KeyBinding {
            ctrl: modifiers.command,
            shift: modifiers.shift,
            alt: modifiers.alt,
            key,
        };
        let new_text = format_binding(&new_binding);
        let eff = effective_keymap(&self.settings);
        let old_text = eff.get(&action).cloned().unwrap_or_default();
        if let Some((other, _)) = eff.iter().find(|(a, b)| *a != &action && *b == &new_text) {
            let other = other.clone();
            self.settings.keymap.insert(other, old_text.clone());
        }
        self.settings.keymap.insert(action, new_text);
        self.key_capture = None;
        if let Err(e) = save_settings(&self.settings) {
            self.fail(format!(
                "Your key was set, but the settings could not be saved. ({e})"
            ));
        }
    }

    pub(crate) fn keys_window(&mut self, ctx: &egui::Context) {
        if !self.show_keys {
            return;
        }
        let mut open = self.show_keys;
        let eff = effective_keymap(&self.settings);
        let status = keymap_status(&self.settings);
        egui::Window::new("Keyboard shortcuts")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_size([520.0, 560.0])
            .show(ctx, |ui| {
                ui.label(
                    "Keyboard shortcuts. Start from a set you know, then make any \
                     key your own: the moment a key moves away from the set's \
                     defaults, the set is called Custom, which is fine and normal.",
                );
                ui.label(format!("You are using: {status}"));
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Start from:");
                    for (preset, label) in [
                        ("openbulletin", "OpenBulletin"),
                        ("publisher", "Publisher"),
                        ("indesign", "InDesign"),
                    ] {
                        if ui.button(label).clicked() {
                            self.settings.keymap_preset = preset.to_string();
                            self.settings.keymap.clear();
                            self.key_capture = None;
                            if let Err(e) = save_settings(&self.settings) {
                                self.fail(format!("Settings could not be saved. ({e})"));
                            }
                        }
                    }
                });
                ui.separator();
                egui::ScrollArea::vertical()
                    .max_height(340.0)
                    .show(ui, |ui| {
                        for (action, label) in KEY_ACTIONS {
                            ui.horizontal(|ui| {
                                ui.label(*label);
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        let capturing = self.key_capture.as_deref() == Some(action);
                                        let text = if capturing {
                                            "Press a key... (Escape cancels)".to_string()
                                        } else {
                                            eff.get(*action).cloned().unwrap_or_default()
                                        };
                                        if ui.button(text).clicked() {
                                            self.key_capture = Some(action.to_string());
                                        }
                                    },
                                );
                            });
                        }
                    });
                ui.separator();
                if ui
                    .button("Put every key back to the set's defaults")
                    .clicked()
                {
                    self.settings.keymap.clear();
                    self.key_capture = None;
                    if let Err(e) = save_settings(&self.settings) {
                        self.fail(format!("Settings could not be saved. ({e})"));
                    }
                }
            });
        self.show_keys = open;
    }

    /// Comfort settings changed (A1): repaint the whole
    /// app in the new size and palette at once, and
    /// remember the choice like every other setting.
    pub(crate) fn apply_comfort(&mut self, ctx: &egui::Context) {
        self.theme = Theme::for_settings(&self.settings);
        ctx.set_style(app_style(&self.settings));
        if let Err(e) = save_settings(&self.settings) {
            self.fail(format!("Settings could not be saved. ({e})"));
        }
        ctx.request_repaint();
    }

    /// The "Text size and contrast" window (A1): three
    /// sizes in plain words, one contrast switch, a live
    /// sample line so the size can be judged before the
    /// window is closed, and the one line of truth about
    /// what these settings do not touch.
    pub(crate) fn comfort_window(&mut self, ctx: &egui::Context) {
        if !self.show_comfort {
            return;
        }
        let mut open = self.show_comfort;
        egui::Window::new("Text size and contrast")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(460.0)
            .show(ctx, |ui| {
                ui.label("How big the words in this app are, and how hard they stand out.");
                ui.add_space(4.0);
                ui.label(egui::RichText::new("Text size").strong());
                for scale in [UiScale::Standard, UiScale::Large, UiScale::ExtraLarge] {
                    if ui
                        .radio_value(&mut self.settings.ui_scale, scale, scale.label())
                        .changed()
                    {
                        self.apply_comfort(ctx);
                    }
                }
                ui.add_space(4.0);
                ui.label("Welcome to worship. The Lord be with you.");
                ui.add_space(6.0);
                ui.separator();
                let mut hc = self.settings.high_contrast;
                if ui
                    .checkbox(&mut hc, "High contrast (black text on white)")
                    .changed()
                {
                    self.settings.high_contrast = hc;
                    self.apply_comfort(ctx);
                }
                ui.add_space(6.0);
                ui.separator();
                ui.label(
                    "These settings change this app only. The bulletin's own \
                     words keep the sizes in the document.",
                );
            });
        self.show_comfort = open;
    }

    pub(crate) fn fonts_window(&mut self, ctx: &egui::Context) {
        if !self.show_fonts {
            return;
        }
        self.poll_catalog(ctx);
        if !self.fonts_loaded {
            self.refresh_fonts(ctx);
        }
        let mut open = self.show_fonts;
        egui::Window::new("Fonts")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(520.0)
            .show(ctx, |ui| {
              egui::ScrollArea::vertical()
                .id_salt("fonts-scroll")
                .max_height(600.0)
                .show(ui, |ui| {
                ui.label("These are the fonts this app may use.");
                ui.label("Bulletins are laid out only with fonts from this library, so they look the same on every computer.");
                ui.add_space(6.0);
                egui::ScrollArea::vertical()
                    .max_height(320.0)
                    .show(ui, |ui| {
                        for row in &self.font_rows {
                            // The sample line names the row's
                            // face only when this frame's
                            // definitions bind it (NOTE 9);
                            // a just-installed font shows in
                            // the interface face for one frame
                            // instead of panicking the painter.
                            let sample_face = if self.bound_families.contains(&row.key) {
                                egui::FontId::new(
                                    17.0,
                                    egui::FontFamily::Name(row.key.clone().into()),
                                )
                            } else {
                                egui::FontId::proportional(17.0)
                            };
                            ui.label(
                                egui::RichText::new(
                                    "Welcome! The Lord be with you. 1234567890",
                                )
                                .font(sample_face),
                            );
                            ui.label(
                                egui::RichText::new(format!(
                                    "{} ({})",
                                    row.full_name, row.family
                                ))
                                .strong(),
                            );
                            ui.label(egui::RichText::new(&row.rights_words).small());
                            ui.separator();
                        }
                    });
                ui.add_space(4.0);
                ui.separator();
                ui.label(egui::RichText::new("Which font each kind of text uses").strong());
                ui.label("Installing a font only makes it available. Pick one here to use it in this bulletin:");
                {
                    let mut families: Vec<String> = vec![
                        "Liberation Sans".to_string(),
                        "Liberation Serif".to_string(),
                    ];
                    for row in &self.font_rows {
                        if !families.contains(&row.family) {
                            families.push(row.family.clone());
                        }
                    }
                    families.sort();
                    families.dedup();
                    let style_names: Vec<String> =
                        self.doc.styles.iter().map(|st| st.name.clone()).collect();
                    for name in style_names {
                        let current = self
                            .doc
                            .styles
                            .iter()
                            .find(|st| st.name == name)
                            .map(|st| st.font_family.clone())
                            .unwrap_or_default();
                        ui.horizontal(|ui| {
                            ui.label(format!("{name}:"));
                            let mut picked = current.clone();
                            egui::ComboBox::from_id_salt(("style-font", &name))
                                .selected_text(&current)
                                .show_ui(ui, |ui| {
                                    for fam in &families {
                                        ui.selectable_value(&mut picked, fam.clone(), fam);
                                    }
                                });
                            if picked != current {
                                let before = snapshot(&self.doc);
                                if let Some(st) =
                                    self.doc.styles.iter_mut().find(|st| st.name == name)
                                {
                                    st.font_family = picked.clone();
                                }
                                self.pin_fonts();
                                self.record_undo(before);
                                self.note(format!("{name} now uses {picked}."));
                            }
                        });
                    }
                }
                ui.separator();
                if ui.button("Add fonts...").clicked() {
                    if let Some(paths) = rfd::FileDialog::new()
                        .add_filter(
                            "Font files and packs",
                            &["ttf", "otf", "ttc", "zip", "woff", "woff2", "pfb", "afm"],
                        )
                        .pick_files()
                    {
                        match ob_fonts::FontLibrary::open(ob_fonts::default_library_dir()) {
                            Ok(lib) => {
                                let mut added: Vec<String> = Vec::new();
                                let mut caution = String::new();
                                let mut problems: Vec<String> = Vec::new();
                                for path in &paths {
                                    let is_zip = path
                                        .extension()
                                        .map(|e| e.eq_ignore_ascii_case("zip"))
                                        .unwrap_or(false);
                                    if is_zip {
                                        match std::fs::read(path) {
                                            Ok(bytes) => {
                                                for result in lib.add_zip_bytes(&bytes) {
                                                    match result {
                                                        Ok(info) => {
                                                            if !info.rights.may_package() {
                                                                caution = format!(
                                                                    " Note: {} did not come from \
                                                                     the free catalog; only share \
                                                                     bulletins using it if you have \
                                                                     the right to share that font.",
                                                                    info.full_name
                                                                );
                                                            }
                                                            added.push(info.full_name);
                                                        }
                                                        Err(e) => problems.push(e),
                                                    }
                                                }
                                            }
                                            Err(e) => problems
                                                .push(format!("could not read a zip: {e}")),
                                        }
                                    } else {
                                        // One result per font
                                        // the file carries: a
                                        // collection adds each
                                        // of its faces, and a
                                        // format we cannot use
                                        // answers with its
                                        // refusal, counted and
                                        // named like any other.
                                        for result in lib.add_path(path) {
                                            match result {
                                                Ok(info) => {
                                                    if !info.rights.may_package() {
                                                        caution = format!(
                                                            " Note: {} did not come from the \
                                                             free catalog; only share bulletins \
                                                             using it if you have the right to \
                                                             share that font.",
                                                            info.full_name
                                                        );
                                                    }
                                                    added.push(info.full_name);
                                                }
                                                Err(e) => problems.push(e),
                                            }
                                        }
                                    }
                                }
                                if added.is_empty() {
                                    self.fail(format!(
                                        "No fonts added. {}",
                                        problems.into_iter().next().unwrap_or_default()
                                    ));
                                } else {
                                    self.note(format!(
                                        "Fonts added: {}{}",
                                        added.join(", "),
                                        caution
                                    ));
                                }
                                self.refresh_fonts(ctx);
                            }
                            Err(e) => {
                                self.fail(format!("Could not open the font library: {e}"))
                            }
                        }
                    }
                }
                ui.separator();
                ui.label(egui::RichText::new("Get a free font").strong());
                ui.label(
                    "Search the free font catalog (nearly all of it is free to \
                     use and share). Getting a font adds its regular, bold, \
                     and italic faces to this library.",
                );
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut self.catalog_query);
                    if ui
                        .add_enabled(!self.catalog_busy, egui::Button::new("Search"))
                        .clicked()
                    {
                        let (tx, rx) = std::sync::mpsc::channel();
                        let query = self.catalog_query.clone();
                        self.catalog_rx = Some(rx);
                        self.catalog_busy = true;
                        std::thread::spawn(move || {
                            let msg = match ob_fonts::catalog::search(&query) {
                                Ok(hits) => CatalogMsg::Hits(hits),
                                Err(e) => CatalogMsg::Failed(e),
                            };
                            let _ = tx.send(msg);
                        });
                    }
                    if self.catalog_busy {
                        ui.spinner();
                    }
                });
                let mut get_id: Option<String> = None;
                for hit in &self.catalog_hits {
                    ui.horizontal(|ui| {
                        ui.label(format!(
                            "{} ({}) - {}",
                            hit.family, hit.category, hit.license
                        ));
                        if ui
                            .add_enabled(!self.catalog_busy, egui::Button::new("Get this font"))
                            .clicked()
                        {
                            get_id = Some(hit.id.clone());
                        }
                    });
                }
                if let Some(id) = get_id {
                    let (tx, rx) = std::sync::mpsc::channel();
                    self.catalog_rx = Some(rx);
                    self.catalog_busy = true;
                    std::thread::spawn(move || {
                        let msg = match ob_fonts::FontLibrary::open(
                            ob_fonts::default_library_dir(),
                        )
                        .and_then(|lib| ob_fonts::catalog::install(&lib, &id))
                        {
                            Ok(installed) => CatalogMsg::Installed(
                                installed.iter().map(|f| f.full_name.clone()).collect(),
                            ),
                            Err(e) => CatalogMsg::Failed(e),
                        };
                        let _ = tx.send(msg);
                    });
                }
                ui.separator();
                ui.label(egui::RichText::new("The Church Starter Pack").strong());
                ui.label(
                    "One tap adds the free stand-ins for the fonts church \
                     computers already have: Carlito (for Calibri), Caladea \
                     (Cambria), Gelasio (Georgia), plus Lora for readings \
                     and Playfair Display for titles and holidays.",
                );
                if ui
                    .add_enabled(
                        !self.catalog_busy,
                        egui::Button::new("Install the Starter Pack"),
                    )
                    .clicked()
                {
                    let (tx, rx) = std::sync::mpsc::channel();
                    self.catalog_rx = Some(rx);
                    self.catalog_busy = true;
                    std::thread::spawn(move || {
                        let msg = match ob_fonts::FontLibrary::open(
                            ob_fonts::default_library_dir(),
                        ) {
                            Ok(lib) => {
                                let report = ob_fonts::catalog::install_pack(
                                    &lib,
                                    ob_fonts::catalog::CHURCH_STARTER_PACK,
                                );
                                CatalogMsg::PackDone {
                                    installed: report
                                        .installed
                                        .iter()
                                        .map(|f| f.full_name.clone())
                                        .collect(),
                                    failed: report.failed,
                                }
                            }
                            Err(e) => CatalogMsg::Failed(e),
                        };
                        let _ = tx.send(msg);
                    });
                }
                ui.separator();
                ui.label(egui::RichText::new("Fonts on this computer").strong());
                ui.label(
                    "OpenBulletin keeps its fonts in its own library, so a \
                     bulletin looks the same on every computer. To use a \
                     font your computer already has, add a copy of it here.",
                );
                if ui.button("Show fonts on this computer").clicked() {
                    let in_library: std::collections::HashSet<String> =
                        ob_fonts::FontLibrary::open(ob_fonts::default_library_dir())
                            .map(|lib| {
                                lib.scan()
                                    .iter()
                                    .map(|f| f.sha256.clone())
                                    .collect()
                            })
                            .unwrap_or_default();
                    let (tx, rx) = std::sync::mpsc::channel();
                    self.sys_scan_rx = Some(rx);
                    std::thread::spawn(move || {
                        let rows = ob_fonts::system_fonts()
                            .into_iter()
                            .filter(|f| !in_library.contains(&f.info.sha256))
                            .collect();
                        let _ = tx.send(rows);
                    });
                }
                if let Some(rx) = &self.sys_scan_rx {
                    ui.label("Looking through this computer's fonts...");
                    match rx.try_recv() {
                        Ok(rows) => {
                            self.system_rows = rows;
                            self.system_shown = true;
                            self.sys_scan_rx = None;
                        }
                        Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                            self.sys_scan_rx = None;
                        }
                        Err(std::sync::mpsc::TryRecvError::Empty) => {}
                    }
                }
                if self.system_shown && self.system_rows.is_empty() {
                    ui.label("Every font on this computer is already in the library.");
                }
                let mut add_copy: Option<std::path::PathBuf> = None;
                for found in &self.system_rows {
                    ui.horizontal(|ui| {
                        ui.label(format!(
                            "{} ({})",
                            found.info.full_name, found.info.family
                        ));
                        if ui.button("Add a copy").clicked() {
                            add_copy = Some(found.path.clone());
                        }
                    });
                }
                if let Some(path) = add_copy {
                    // The list only offers single font files,
                    // so the copy answers with one result.
                    let copied = ob_fonts::FontLibrary::open(ob_fonts::default_library_dir())
                        .and_then(|lib| {
                            lib.add_path(&path)
                                .into_iter()
                                .next()
                                .unwrap_or_else(|| Err("that font file held no font".into()))
                        });
                    match copied {
                        Ok(info) => {
                            if info.rights.may_package() {
                                self.note(format!("Font copied in: {}", info.full_name));
                            } else {
                                self.note(format!(
                                    "Font copied in: {}. Note: it did not come from \
                                     the free catalog; only share bulletins using \
                                     it if you have the right to share that font.",
                                    info.full_name
                                ));
                            }
                            self.system_rows
                                .retain(|f| f.info.sha256 != info.sha256);
                            self.refresh_fonts(ctx);
                        }
                        Err(e) => {
                            self.fail(format!("Could not add that font: {e}"))
                        }
                    }
                }
                });
            });
        self.show_fonts = open;
    }
}
