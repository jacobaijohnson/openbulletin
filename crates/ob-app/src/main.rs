//! OpenBulletin desktop app.
//!
//! Design law (docs/PRINCIPLES.md): KISS, built for church office
//! administrators, not layout professionals. The home screen is a guided
//! weekly flow with one clear next step at a time:
//!   1 Start  2 This Sunday  3 Announcements  4 Check  5 Print
//! The format, layout, text, and import engines live in the ob-* crates
//! and the CLI. What is GUI-bound lives in this crate, organized into
//! named modules beside this file: settings and the crash journal
//! (settings.rs), the Check step's findings engine (check.rs),
//! preview and editor layout (preview.rs), contribution planning and
//! application (contribution.rs), loose-sheet helpers (inserts.rs),
//! and the step windows (windows.rs). This file holds the App struct,
//! the eframe dispatch, and the helpers every module shares.

use eframe::egui;
use ob_layout::{build_template, Preset};
use ob_model::{merge_rights_records, ChurchProfile, ContentRole, Document};

pub mod check;
pub mod contribution;
mod editmap;
pub mod inserts;
pub mod preview;
mod runsedit;
pub mod settings;
mod starter_clips;
pub mod windows;

use check::*;
use contribution::*;
use inserts::*;
use preview::*;
use settings::*;

const STEPS: [&str; 5] = ["Start", "This Sunday", "Announcements", "Check", "Print"];

/// The app's small, named palette (UI audit round 2, batch 3).
/// One red means "a problem", one amber "look at this", one
/// green "all good", one muted blue "for information", and one
/// blue does every canvas job (guides, selection, drag
/// readouts). No other literal colors for these meanings.
const PROBLEM: egui::Color32 = egui::Color32::from_rgb(170, 40, 30);
const WARN: egui::Color32 = egui::Color32::from_rgb(160, 100, 10);
const GOOD: egui::Color32 = egui::Color32::from_rgb(40, 110, 60);
const INFO: egui::Color32 = egui::Color32::from_rgb(90, 90, 160);
const CANVAS_ACCENT: egui::Color32 = egui::Color32::from_rgb(0, 120, 195);

/// A stable snapshot string for one document state.
impl App {
    /// Start watching for a change worth one Undo step (a drag, a
    /// typing session, a burst of nudges).
    fn begin_gesture(&mut self) {
        if self.gesture_start.is_none() {
            self.gesture_start = Some(snapshot(&self.doc));
        }
        self.gesture_touch = Some(std::time::Instant::now());
    }

    /// Close the gesture: if the document changed, the starting
    /// state becomes one Undo step.
    fn end_gesture(&mut self) {
        if let Some(start) = self.gesture_start.take() {
            self.gesture_touch = None;
            if start != snapshot(&self.doc) {
                self.record_undo(start);
            }
        }
    }

    /// Record the current state as one Undo step right now (for
    /// one-shot changes like a menu apply). Call BEFORE mutating.
    fn push_undo_now(&mut self) {
        let state = snapshot(&self.doc);
        self.record_undo(state);
    }

    /// Put one earlier document state on the Undo stack.
    fn record_undo(&mut self, state: String) {
        self.undo_stack.push(state);
        if self.undo_stack.len() > 60 {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }

    fn undo(&mut self) {
        let Some(prev) = self.undo_stack.pop() else {
            return;
        };
        let Ok(doc) = serde_json::from_str::<Document>(&prev) else {
            return;
        };
        self.redo_stack.push(snapshot(&self.doc));
        self.adopt_document(doc, false);
        self.note("Undone.".into());
    }

    fn redo(&mut self) {
        let Some(next) = self.redo_stack.pop() else {
            return;
        };
        let Ok(doc) = serde_json::from_str::<Document>(&next) else {
            return;
        };
        self.undo_stack.push(snapshot(&self.doc));
        self.adopt_document(doc, false);
        self.note("Redone.".into());
    }

    /// Forget every index that pointed into the old document:
    /// selection, drags, menus, the edit session. Raw indices
    /// into a replaced document are how the audits found a
    /// reachable panic (audit synthesis 2026-10-15, blocker 7).
    fn reset_interaction_state(&mut self) {
        self.gesture_start = None;
        self.gesture_touch = None;
        self.thread_arm = None;
        self.thread_confirm = None;
        self.editing = None;
        self.edit_start = None;
        self.edit_done = false;
        self.drag_guide = None;
        self.drag_frame = None;
        self.drag_resize = false;
        self.drag_pan = None;
        self.pan_origin = None;
        self.selected_frame = None;
        self.canvas_panel = 0;
        self.guide_menu_for = None;
        self.frame_menu_for = None;
        self.ctx_pos = None;
        self.lines_cache = None;
        self.bg_tex_key.clear();
        self.bg_tex = None;
    }

    /// Refresh the text buffers that mirror the document
    /// (announcements, church profile, date) after a swap.
    /// Set a failure status: renders as a banner (F3).
    fn fail(&mut self, msg: String) {
        self.status = msg;
        self.status_error = true;
    }

    /// Set an ordinary status and clear any failure banner.
    fn note(&mut self, msg: String) {
        self.status = msg;
        self.status_error = false;
    }

    fn sync_buffers(&mut self) {
        self.announcements = weekly_body_text(&self.doc);
        self.announcements_dirty = false;
        self.ann_keep_through = weekly_story_id(&self.doc)
            .and_then(|sid| self.doc.story(&sid))
            .and_then(|s| s.keep_through.clone())
            .unwrap_or_default();
        self.church_name = self.doc.profile.name.clone();
        self.profile_address = self.doc.profile.address.clone();
        self.profile_giving_url = self.doc.profile.giving_url.clone();
        self.profile_service_times = self.doc.profile.service_times.clone();
        self.profile_website = self.doc.profile.website.clone();
        self.profile_pastor = self.doc.profile.pastor.clone();
        self.profile_expected_parts = self.doc.profile.expected_parts.join(", ");
        self.profile_serving_lines = self.doc.profile.serving_lines.join(", ");
        self.date_input = self
            .doc
            .fields
            .get("occasion_date")
            .cloned()
            .unwrap_or_default();
        self.service_inputs = service_lines(&self.doc)
            .into_iter()
            .map(|l| l.value)
            .collect();
        self.hymn_inputs = service_lines(&self.doc)
            .iter()
            .map(|l| hymn_field_values(&l.value))
            .collect();
        self.sermon_input = sermon_title(&self.doc);
        self.participant_inputs = participant_values(&self.doc);
        self.participant_line_names = self.doc.profile.serving_lines.clone();
    }

    /// Set the bulletin's date from the Step 2 field, in the
    /// printed long form too. The field accepts any format a
    /// person types (docs/planning/occasions.md): ISO, US
    /// numeric, month names, an optional weekday that must
    /// be right, a missing year meaning the coming one.
    /// Plain failure words when the date does not read
    /// (blocker 1: the date used to be unchangeable).
    fn apply_date(&mut self) {
        match ob_model::parse_flexible_date(self.date_input.trim(), today_simple()) {
            Some(d) => {
                let nd = naive_of(d);
                let before = snapshot(&self.doc);
                self.doc.fields.insert(
                    "occasion_date".to_string(),
                    nd.format("%Y-%m-%d").to_string(),
                );
                self.doc
                    .fields
                    .insert("occasion_date_long".to_string(), long_form(nd));
                if before != snapshot(&self.doc) {
                    self.record_undo(before);
                }
                self.note(format!("Date set to {}.", long_form(nd)));
            }
            None => {
                self.fail("That date does not read. Try year-month-day, like 2026-10-18, or write it out, like October 18, 2026.".into());
            }
        }
    }

    /// Apply the Step 2 date field when it holds a new
    /// date, without being asked: every neighboring field
    /// applies on focus loss, and the date used to be the
    /// one field that silently did not, so a typed date
    /// could be thrown away with no word said (UI audit
    /// round 2, flow blocker). Called when the field loses
    /// focus and when the step is left.
    fn apply_edited_date(&mut self) {
        if date_differs(
            &self.date_input,
            self.doc.fields.get("occasion_date").map(String::as_str),
        ) {
            self.apply_date();
        }
    }

    /// Set the announcements' keep-through date from the
    /// Announcements step field. Blank means "this Sunday only".
    /// A date that does not read is refused in plain words and
    /// the field goes back to what the bulletin holds.
    fn apply_ann_keep_through(&mut self) {
        let Some(sid) = weekly_story_id(&self.doc) else {
            return;
        };
        let text = self.ann_keep_through.trim().to_string();
        let parsed = if text.is_empty() {
            Some(None)
        } else {
            ob_model::parse_flexible_date(&text, today_simple())
                .map(|d| Some(format!("{:04}-{:02}-{:02}", d.year, d.month, d.day)))
        };
        let Some(value) = parsed else {
            self.fail(
                "That keep-through date does not read. Try year-month-day, like 2026-11-08, or write it out, like November 8, 2026, or leave it blank.".into(),
            );
            self.ann_keep_through = self
                .doc
                .story(&sid)
                .and_then(|s| s.keep_through.clone())
                .unwrap_or_default();
            return;
        };
        let before = snapshot(&self.doc);
        if let Some(story) = self.doc.story_mut(&sid) {
            story.keep_through = value.clone();
        }
        if before != snapshot(&self.doc) {
            self.record_undo(before);
            self.note(match &value {
                Some(iso) => {
                    let d =
                        chrono::NaiveDate::parse_from_str(iso, "%Y-%m-%d").expect("just formatted");
                    format!(
                        "These announcements stay in the bulletin through {}.",
                        ob_layout::long_date(d)
                    )
                }
                None => "Announcements are for this Sunday only.".into(),
            });
        }
    }

    /// The one way a whole document replaces the current one
    /// (Open, recovery, Undo, Redo). History is cleared when the
    /// replacement crosses a file boundary, so Undo can never
    /// resurrect another bulletin's content under this file's
    /// name (audit blocker 5). Within one file's editing
    /// session, Undo and Redo pass clear_history = false.
    fn adopt_document(&mut self, doc: Document, clear_history: bool) {
        // A whole new document (Open, an import, a fresh
        // start) retires the loose-sheet canvas state with
        // the old document (R-3): the sheet being edited
        // was part of that document's arrangement, and it
        // goes with it, as the open guard already armed
        // for. Within one document (Undo, Redo) the state
        // stays: those snapshots share the arrangement.
        if clear_history {
            self.insert_edit = None;
            self.insert_sel = None;
            self.insert_sel_loaded = None;
            self.insert_arm_remove = None;
        }
        // Any swap retires a pick the open guard was holding.
        self.pending_path = None;
        // A contribution review and the save guard's state
        // belong to the old document too (R-1).
        self.contribution_review = None;
        // The applied-parts record needs no clearing
        // here: it lives in the document being adopted
        // (round 4, R4-4), so it arrives and leaves with
        // the words it describes, Undo included.
        self.save_conflict = None;
        self.replace_armed = false;
        self.replace_ok = None;
        // The disk stamp deliberately survives a swap: it
        // describes the file at its path, not the screen,
        // and it only ever speaks for that one path.
        // Thrown-away typing belonged to the old document (J3).
        self.discarded_edit = None;
        // A start-fresh arm belongs to the old bulletin.
        self.shape_arm = None;
        if clear_history {
            self.undo_stack.clear();
            self.redo_stack.clear();
        }
        self.reset_interaction_state();
        self.doc = doc;
        self.sync_buffers();
    }
}

fn snapshot(doc: &Document) -> String {
    serde_json::to_string(doc).unwrap_or_default()
}

/// Just the file's name for status messages (UI audit round
/// 2, batch 3): "Print copy written: bulletin-print.pdf, ..."
/// reads better in a narrow sidebar than a full path, and
/// the person just chose the folder in the save dialog.
/// The name Save proposes for a bulletin that has never been
/// saved (J8): dated from the document's own Sunday date, so
/// a folder of bulletins sorts itself by week. Falls back to
/// the plain name when the date field does not parse.
fn suggested_save_name(doc: &Document) -> String {
    match doc
        .fields
        .get("occasion_date")
        .and_then(|iso| chrono::NaiveDate::parse_from_str(iso.trim(), "%Y-%m-%d").ok())
    {
        Some(d) => format!("Bulletin {}.obd", d.format("%Y-%m-%d")),
        None => "bulletin.obd".to_string(),
    }
}

fn file_name_of(path: &std::path::Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

/// The web page's house name: the name picked in the save
/// dialog with "-web" on its stem, beside the pick, so a
/// bulletin and its reading edition sort together and the
/// page never overwrites the file it came from. A pick
/// already carrying the suffix is honored as it stands.
fn web_page_path(picked: &std::path::Path) -> std::path::PathBuf {
    let stem = picked
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "bulletin".to_string());
    if stem.ends_with("-web") {
        return picked.to_path_buf();
    }
    picked.with_file_name(format!("{stem}-web.html"))
}

/// A file size in plain words for status lines: bytes under a
/// kilobyte by count, then KB, then MB with one decimal.
fn human_file_size(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{bytes} bytes")
    } else if bytes < 1024 * 1024 {
        format!("{} KB", (bytes + 512) / 1024)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

struct App {
    doc: Document,
    preset: Preset,
    /// The bulletin shape whose start-fresh click is armed:
    /// picking a shape replaces the whole bulletin, so it
    /// takes the app's usual two clicks, like every other
    /// destructive path (UI audit round 2).
    shape_arm: Option<Preset>,
    church_name: String,
    profile_address: String,
    profile_giving_url: String,
    profile_service_times: String,
    profile_website: String,
    profile_pastor: String,
    /// The Church Profile area's "Parts we usually get"
    /// field, as typed (comma or newline separated). The
    /// document's profile holds the parsed list (R-1).
    profile_expected_parts: String,
    /// The Church Profile area's "Serving lines" field,
    /// as typed (comma or newline separated). The
    /// document's profile holds the parsed list (R-2b).
    profile_serving_lines: String,
    date_input: String,
    /// The This Sunday step's service-line value fields (J2),
    /// parallel to `service_lines(&doc)`, plus the sermon
    /// title field. Synced from the document on every swap.
    service_inputs: Vec<String>,
    /// The This Sunday step's hymn fields (R-2a): for each
    /// service line, its value in three parts (hymnal,
    /// number, title), parallel to `service_lines(&doc)`.
    /// Only hymn-labeled rows show them; the parts compose
    /// back into the line's value when one is edited.
    hymn_inputs: Vec<[String; 3]>,
    sermon_input: String,
    /// The church's hymn memory (R-2a): number and title
    /// pairs learned from this church's saved bulletins,
    /// kept in a file beside the settings. Check holds
    /// this week's hymn lines up against it.
    hymn_memory: ob_model::hymns::HymnMemory,
    /// The This Sunday step's serving-line fields, parallel
    /// to `participant_line_names`: one row per line the
    /// church's profile shows, synced from the document's
    /// participants map on every swap and whenever the
    /// profile's lines change (R-2b).
    participant_inputs: Vec<String>,
    /// The serving-line names `participant_inputs` is
    /// parallel to, so a rename in the Church Profile
    /// re-syncs the rows instead of leaving them stale.
    participant_line_names: Vec<String>,
    announcements_dirty: bool,
    step: usize,
    announcements: String,
    notes: Vec<CheckNote>,
    status: String,
    /// The last "Save a copy for another program..." result,
    /// shown under that button on the Print step: what was
    /// saved and what the copy could not carry.
    export_note: Option<String>,
    /// Stories using characters the print file cannot show,
    /// found by the last Check run (and recomputed on Print).
    char_findings: Vec<CharFinding>,
    /// The "Keep through" date field on the Announcements
    /// step, bound to the announcements story.
    ann_keep_through: String,
    /// The document snapshot a print copy was made from, so
    /// the Print step can say when the bulletin has changed
    /// since. Seeded from Settings per file (J6), so the
    /// memory now survives a restart.
    print_fingerprint: Option<String>,
    /// The document snapshot that last passed Check with
    /// nothing red (J6), seeded and stored beside
    /// print_fingerprint. Any edit changes the snapshot, so
    /// staleness needs no bookkeeping of its own.
    checked_fingerprint: Option<String>,
    path: Option<std::path::PathBuf>,
    settings: Settings,
    /// The palette in force, computed from the settings at
    /// load and whenever the comfort settings change (A1).
    theme: Theme,
    printer_name: String,
    show_pages: bool,
    canvas_panel: usize,
    drag_guide: Option<usize>,
    selected_frame: Option<usize>,
    drag_frame: Option<usize>,
    drag_resize: bool,
    grab_dx: ob_model::Mpt,
    grab_dy: ob_model::Mpt,
    frame_input: [String; 4],
    frame_menu_for: Option<(usize, ob_model::Rect)>,
    ctx_pos: Option<egui::Pos2>,
    guide_input: String,
    guide_menu_for: Option<(usize, ob_model::Mpt)>,
    bg_tex_key: String,
    bg_tex: Option<egui::TextureHandle>,
    /// Decoded picture textures for the canvas, the previews,
    /// and the pickers: (content key, size class in pixels) ->
    /// (texture, decoded width, decoded height). A decode
    /// happens once per key per class; the stored asset bytes
    /// themselves never change (G2).
    pic_tex: std::collections::HashMap<(String, u32), (egui::TextureHandle, u32, u32)>,
    /// Prepared pictures for the PDF exports, shared across
    /// this session's exports (ob_pdf::ImageCache, health
    /// audit Efficiency MINOR-3): the print copy, the reading
    /// copy, and the large-print copy decode and encode each
    /// picture once between them, not once per copy. The
    /// cache keys on the picture's content hash and target
    /// size, like pic_tex above, so a replaced picture can
    /// never serve its predecessor's pixels.
    export_image_cache: ob_pdf::ImageCache,
    /// The picture box whose crop is being panned by a drag
    /// inside it, and where that drag began: (crop_x, crop_y,
    /// pointer x, pointer y) in milli-points.
    drag_pan: Option<usize>,
    pan_origin: Option<(f64, f64, ob_model::Mpt, ob_model::Mpt)>,
    /// The clip art library window and its state.
    show_clips: bool,
    clips_ensured: bool,
    clip_collection: Option<String>,
    clip_query: String,
    clip_status: Option<String>,
    /// The "put a picture in the box" picker (a file door
    /// plus the document's loose pictures).
    show_pic_picker: bool,
    /// The Loose sheets window and its state (R-3).
    show_inserts: bool,
    /// The loose sheet on the canvas right now, by id.
    /// While this is set, the document's panels, stories,
    /// and styles ARE the sheet's (swapped in by
    /// `swap_insert_parts`), and the sheet's own entry in
    /// `doc.inserts` holds the bulletin's: one source of
    /// truth, arranged inside out for the length of the
    /// edit. Every whole-document action (save, print,
    /// check, Next Sunday, a step change) swaps back
    /// first through `leave_insert_editing`, and the
    /// autosave journal writes the swapped-back clone, so
    /// no file on disk ever holds the inside-out shape.
    insert_edit: Option<String>,
    /// The sheet the Loose sheets window's settings are
    /// showing, and the id those buffers were loaded for.
    insert_sel: Option<String>,
    insert_sel_loaded: Option<String>,
    insert_name_buf: String,
    insert_date_buf: String,
    insert_copies_buf: String,
    insert_paper_buf: String,
    insert_add_name: String,
    /// The sheet whose removal is armed (two-click).
    insert_arm_remove: Option<String>,
    /// The sheet library's .obdi files, as (name, path),
    /// rescanned when the window asks.
    insert_lib: Vec<(String, std::path::PathBuf)>,
    insert_lib_dirty: bool,
    show_fonts: bool,
    show_comfort: bool,
    show_keys: bool,
    /// The action whose key is being captured, if any.
    key_capture: Option<String>,
    fonts_loaded: bool,
    font_rows: Vec<FontRow>,
    /// The named font families the definitions ACTIVE this
    /// frame bind, snapshotted at the top of every update.
    /// Painting may name only these (round 2, NOTE 9).
    bound_families: std::collections::HashSet<String>,
    catalog_query: String,
    catalog_hits: Vec<ob_fonts::catalog::CatalogEntry>,
    catalog_rx: Option<std::sync::mpsc::Receiver<CatalogMsg>>,
    catalog_busy: bool,
    system_rows: Vec<ob_fonts::SystemFont>,
    /// The system-font census runs on a worker: reading and
    /// hashing every font on a real PC takes seconds and must
    /// not freeze the window (closing audit).
    sys_scan_rx: Option<std::sync::mpsc::Receiver<Vec<ob_fonts::SystemFont>>>,
    system_shown: bool,
    undo_stack: Vec<String>,
    redo_stack: Vec<String>,
    gesture_start: Option<String>,
    gesture_touch: Option<std::time::Instant>,
    lines_cache: Option<LinesCache>,
    /// The text shaper, rebuilt only when the Font Library
    /// changes (closing audit: it used to be rebuilt, with a
    /// full library scan and hash of every font, per paragraph
    /// and per panel per repaint).
    shaper_cache: Option<(u64, ob_text::shaping::Shaper)>,
    /// Threading by hand (T1): the box whose text is waiting
    /// for its continuation box to be clicked ("Continue
    /// this text in another box..."), as (panel, frame).
    thread_arm: Option<(usize, usize)>,
    /// The continuation target clicked once already, when it
    /// holds words of its own: the second click on the same
    /// box replaces them. Any other click resets this.
    thread_confirm: Option<(usize, usize)>,
    editing: Option<(usize, usize)>,
    edit_text: String,
    edit_styles: Vec<String>,
    /// The session's inline formatting (G1b): one run list
    /// per paragraph of `edit_text`, raw text, kept in step
    /// by runsedit on every buffer mutation. Detached means
    /// the fallback (unshaped) editor changed the words, so
    /// the runs no longer describe them and commit writes
    /// plain runs rather than wrong ones.
    edit_runs: Vec<Vec<ob_model::Run>>,
    edit_runs_detached: bool,
    /// The session's spoken-only flags, one per paragraph of
    /// `edit_text`, kept in step by `sync_edit_spoken`
    /// exactly as `edit_styles` is kept by
    /// `sync_edit_styles`, and written back at commit: a
    /// spoken-only paragraph is read aloud in the service
    /// and never printed (prayer-list privacy).
    edit_spoken: Vec<bool>,
    /// The session's not-for-web flags, one per paragraph
    /// of `edit_text`, kept in step by
    /// `sync_edit_web_hidden` on exactly the spoken-only
    /// discipline, and written back at commit: a
    /// not-for-web paragraph prints in the bulletin but
    /// stays off the web page (children's privacy, R-2b).
    edit_web_hidden: Vec<bool>,
    /// Story id of the current edit session, salting the text
    /// editor's identity so its private state (cursor, its own
    /// undo) never leaks from one box's session into another's
    /// (audit R4).
    edit_salt: String,
    /// Paragraph the cursor is in, kept by the editing toolbar
    /// so style inheritance follows insertions at the cursor
    /// instead of the end of the text (audit R2).
    edit_cursor_para: usize,
    /// True when the status line reports a failure: it renders
    /// as a banner the user cannot miss (finding F3).
    status_error: bool,
    /// Two-click guard on Open while unsaved work is on screen
    /// (finding B3): the first click arms, the second leaves
    /// the work behind.
    /// A guarded file-open waiting for its confirming click,
    /// and the screen state it was armed against. If the work
    /// changes after arming, the guard re-arms (closing audit:
    /// the old arm flag stayed set forever and silently
    /// disarmed the guard for later, different work).
    pending_open: Option<PendingOpen>,
    pending_open_snapshot: String,
    /// A file picked at the old-bulletin door that the
    /// unsaved-work guard held: the confirming click opens
    /// this very file instead of asking for it again
    /// (UI audit round 2).
    pending_path: Option<std::path::PathBuf>,
    /// Two-click guard on throwing recovered work away (F11).
    discard_armed: bool,
    /// Another live OpenBulletin process holds the journal: this
    /// instance never writes or clears it (closing audit).
    peer_alive: bool,
    /// The safety copy has failed to write this session and
    /// the user has already been told once (S-MIN-5): the
    /// journal keeps retrying quietly after that, rather
    /// than raising the banner every two seconds.
    journal_failure_noted: bool,
    /// The journal worker (HF-4): the thread that does the
    /// journal's save and its retirement off the frame
    /// thread. Spawned lazily on the first journal job;
    /// None until then (startup's own journal handling runs
    /// before it exists, on the startup path).
    journal_worker: Option<JournalWorker>,
    /// The journal job currently with the worker, if any.
    /// One flight at a time: journal file mutations never
    /// race.
    journal_flight: Option<JournalFlight>,
    /// The one journal job waiting behind the flight, if
    /// any: for writes only the newest snapshot is kept
    /// (coalesced, never an unbounded backlog), and a
    /// retire replaces a waiting write outright.
    journal_pending: Option<JournalJob>,
    /// A retirement has been requested and has not landed
    /// yet: the frame thread sends no new write jobs until
    /// it does, so no write can land after the retire.
    journal_retiring: bool,
    /// The hymn memory has failed to write this session and
    /// the user has already been told once (S-MIN-2): later
    /// failures stay quiet, like the journal's.
    hymn_memory_failure_noted: bool,
    /// PDF rescue in flight and awaiting review.
    import_rx: Option<std::sync::mpsc::Receiver<Result<ImportReview, String>>>,
    analyzer_rx: Option<std::sync::mpsc::Receiver<Result<AnalyzerReview, String>>>,
    analyzer_review: Option<AnalyzerReview>,
    analyzer_armed: bool,
    import_review: Option<ImportReview>,
    import_armed: bool,
    /// A contribution (from the inbox or the paste box)
    /// waiting for its Accept/Edit/Cancel answer (R-1).
    contribution_review: Option<ContributionReview>,
    /// The paste box: words someone sent, pasted in.
    show_paste: bool,
    paste_text: String,
    /// The Start step's inbox scan, refreshed when Start
    /// is entered and after a part is filed: reading the
    /// folder every repaint would hit the disk constantly.
    inbox_items: Vec<InboxItem>,
    inbox_filed: Vec<InboxItem>,
    inbox_dirty: bool,
    /// The disk stamp of the bulletin file as this app
    /// last saw it (opened it, or saved it), and the save
    /// guard's state when a save was stopped (R-1).
    disk_stamp: Option<(std::path::PathBuf, FileStamp)>,
    save_conflict: Option<std::path::PathBuf>,
    replace_armed: bool,
    replace_ok: Option<std::path::PathBuf>,
    /// Where an unsaved import's page pictures live until the
    /// first Save moves them beside the bulletin file.
    asset_base: Option<std::path::PathBuf>,
    /// Pages window zoom: 1.0 fits the window (finding R9).
    pages_zoom: f32,
    /// The copied box, for Ctrl+C / Ctrl+V on the canvas.
    frame_clipboard: Option<(ob_model::Frame, Option<ob_model::Story>)>,
    /// Sequence for fresh box and story ids.
    box_seq: u32,
    /// One document snapshot per frame, shared by the preview
    /// caches instead of re-serializing the document for every
    /// consumer (audit R6).
    frame_snapshot: String,
    edit_done: bool,
    focus_edit: bool,
    edit_start: Option<String>,
    /// The edit buffer as the current session started (J3):
    /// Escape compares the buffer against it to know whether
    /// real words are being thrown away.
    edit_start_text: String,
    /// Words a cancelled typing session threw away (J3):
    /// (story id, panel, frame, text). While set, the status
    /// area and the Pages window offer to bring them back.
    /// Any new edit session or document swap clears it.
    discarded_edit: Option<(String, usize, usize, String)>,
    /// E1 shaped editor: caret and selection anchor as byte
    /// offsets in DISPLAY space (see editmap::BufferMap). The
    /// session buffer stays the raw '\n'-joined text; every
    /// edit is translated through the map.
    edit_caret: usize,
    edit_anchor: usize,
    /// Threading (T1): this session was opened on a box that
    /// is not its thread's first, so when the layout is
    /// ready the caret starts at the entered box's first
    /// line (the slice the session shows) instead of the
    /// story's start. Consumed by refresh_editor_layout.
    edit_caret_to_slice: bool,
    /// A chip the first Backspace/Delete press selected
    /// whole; a second press deletes it whole.
    edit_chip_armed: Option<std::ops::Range<usize>>,
    /// True while a pointer drag is extending the selection.
    edit_sel_drag: bool,
    /// The x position (frame points) Up/Down movement tries
    /// to keep across lines.
    edit_desired_x: Option<f32>,
    /// Cached shaped geometry for the session's buffer.
    edit_layout: Option<EditorLayout>,
    /// Set when shaped layout construction produced nothing
    /// usable: the session falls back to the plain overlay
    /// editor rather than blocking editing (E1 fallback).
    edit_layout_failed: bool,
    /// IME composition in progress: (display offset, text).
    /// Drawn at the caret, never part of the buffer until
    /// the composition commits.
    ime_preedit: Option<(usize, String)>,
    preview_cache: Option<(String, Vec<PreviewSheet>)>,
    saved_snapshot: String,
    journal_snapshot: String,
    last_journal_check: std::time::Instant,
    startup_done: bool,
    recovery: Option<(Document, Option<std::path::PathBuf>, u64)>,
}

impl Default for App {
    fn default() -> Self {
        let preset = Preset::LetterBifold;
        // A first run is a blank slate, not somebody else's
        // church: the profile starts empty and the Start
        // step asks for it (UI audit round 2). Nothing
        // invented can print on a real cover.
        let doc = build_template(
            preset,
            ChurchProfile::default(),
            // A fresh bulletin is for the coming Sunday, never
            // for whatever weekday today happens to be (J7).
            sunday_on_or_after(chrono::Local::now().date_naive()),
            "Sunday",
        );
        let announcements = weekly_body_text(&doc);
        let date_input = doc.fields.get("occasion_date").cloned().unwrap_or_default();
        let service_inputs: Vec<String> =
            service_lines(&doc).into_iter().map(|l| l.value).collect();
        let hymn_inputs: Vec<[String; 3]> = service_lines(&doc)
            .iter()
            .map(|l| hymn_field_values(&l.value))
            .collect();
        let sermon_input = sermon_title(&doc);
        let participant_inputs = participant_values(&doc);
        let participant_line_names = doc.profile.serving_lines.clone();
        let (settings, settings_read) = load_settings();
        let (hymn_memory, hymn_memory_read) = load_hymn_memory();
        // Reopen on the step the user left (J6), clamped to
        // the steps that exist.
        let step = settings.last_step.min(STEPS.len() - 1);
        let theme = Theme::for_settings(&settings);
        // A settings file that could not be read is said out
        // loud once, on the first status line, never suffered
        // in silence (round 2, R2-4).
        let mut status = match settings_read {
            SettingsRead::Clean => "Welcome to OpenBulletin.".to_string(),
            SettingsRead::SetAside => "Welcome to OpenBulletin. Your settings file \
                 could not be read, so it was set aside as settings-unreadable.json \
                 and fresh settings are in use."
                .to_string(),
            SettingsRead::PartlySalvaged => "Welcome to OpenBulletin. Part of your \
                 settings file could not be read; those parts went back to their \
                 defaults, and the rest were kept."
                .to_string(),
        };
        // The same one-time courtesy for the hymn memory
        // (S-MIN-2): a damaged file was set aside, not
        // silently forgotten.
        if hymn_memory_read == HymnMemoryRead::SetAside {
            status.push_str(
                " Your hymn memory file could not be read, so it was set aside \
                 as hymn-memory-unreadable.json and the memory starts fresh; it \
                 will learn again as bulletins are saved.",
            );
        }
        Self {
            doc,
            preset,
            shape_arm: None,
            church_name: String::new(),
            profile_address: String::new(),
            profile_giving_url: String::new(),
            profile_service_times: String::new(),
            profile_website: String::new(),
            profile_pastor: String::new(),
            profile_expected_parts: String::new(),
            profile_serving_lines: String::new(),
            date_input,
            service_inputs,
            hymn_inputs,
            hymn_memory,
            sermon_input,
            participant_inputs,
            participant_line_names,
            announcements_dirty: false,
            step,
            announcements,
            notes: Vec::new(),
            status,
            export_note: None,
            char_findings: Vec::new(),
            ann_keep_through: String::new(),
            print_fingerprint: None,
            checked_fingerprint: None,
            path: None,
            settings,
            theme,
            printer_name: String::new(),
            show_pages: false,
            canvas_panel: 0,
            drag_guide: None,
            thread_arm: None,
            thread_confirm: None,
            selected_frame: None,
            drag_frame: None,
            drag_resize: false,
            grab_dx: 0,
            grab_dy: 0,
            frame_input: [String::new(), String::new(), String::new(), String::new()],
            frame_menu_for: None,
            ctx_pos: None,
            guide_input: String::new(),
            guide_menu_for: None,
            bg_tex_key: String::new(),
            bg_tex: None,
            pic_tex: std::collections::HashMap::new(),
            export_image_cache: ob_pdf::ImageCache::new(),
            drag_pan: None,
            pan_origin: None,
            show_clips: false,
            clips_ensured: false,
            clip_collection: None,
            clip_query: String::new(),
            clip_status: None,
            show_pic_picker: false,
            show_inserts: false,
            insert_edit: None,
            insert_sel: None,
            insert_sel_loaded: None,
            insert_name_buf: String::new(),
            insert_date_buf: String::new(),
            insert_copies_buf: String::new(),
            insert_paper_buf: String::new(),
            insert_add_name: String::new(),
            insert_arm_remove: None,
            insert_lib: Vec::new(),
            insert_lib_dirty: true,
            show_fonts: false,
            show_comfort: false,
            show_keys: false,
            key_capture: None,
            fonts_loaded: false,
            bound_families: std::collections::HashSet::new(),
            font_rows: Vec::new(),
            catalog_query: String::new(),
            catalog_hits: Vec::new(),
            catalog_rx: None,
            catalog_busy: false,
            system_rows: Vec::new(),
            sys_scan_rx: None,
            system_shown: false,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            gesture_start: None,
            gesture_touch: None,
            lines_cache: None,
            shaper_cache: None,
            editing: None,
            edit_text: String::new(),
            edit_styles: Vec::new(),
            edit_runs: Vec::new(),
            edit_runs_detached: false,
            edit_spoken: Vec::new(),
            edit_web_hidden: Vec::new(),
            edit_salt: String::new(),
            edit_cursor_para: 0,
            frame_snapshot: String::new(),
            status_error: false,
            pending_open: None,
            pending_open_snapshot: String::new(),
            pending_path: None,
            discard_armed: false,
            peer_alive: false,
            journal_failure_noted: false,
            journal_worker: None,
            journal_flight: None,
            journal_pending: None,
            journal_retiring: false,
            hymn_memory_failure_noted: false,
            import_rx: None,
            analyzer_rx: None,
            analyzer_review: None,
            analyzer_armed: false,
            import_review: None,
            import_armed: false,
            contribution_review: None,
            show_paste: false,
            paste_text: String::new(),
            inbox_items: Vec::new(),
            inbox_filed: Vec::new(),
            inbox_dirty: true,
            disk_stamp: None,
            save_conflict: None,
            replace_armed: false,
            replace_ok: None,
            asset_base: None,
            pages_zoom: 1.0,
            frame_clipboard: None,
            box_seq: 0,
            edit_done: false,
            focus_edit: false,
            edit_start: None,
            edit_start_text: String::new(),
            discarded_edit: None,
            edit_caret: 0,
            edit_anchor: 0,
            edit_caret_to_slice: false,
            edit_chip_armed: None,
            edit_sel_drag: false,
            edit_desired_x: None,
            edit_layout: None,
            edit_layout_failed: false,
            ime_preedit: None,
            preview_cache: None,
            saved_snapshot: String::new(),
            journal_snapshot: String::new(),
            last_journal_check: std::time::Instant::now(),
            startup_done: false,
            recovery: None,
        }
    }
}

/// A bulletin file's state on disk, remembered when the
/// app opens or saves it (roles build R-1, the changed-on-
/// disk guard): when it last changed, how long it is, and
/// a hash of its content. The hash is the verdict's real
/// evidence; the time and length ride along because a
/// changed time with the same content (a sync tool touched
/// the file) is not a conflict.
#[derive(Debug, Clone, PartialEq)]
struct FileStamp {
    modified_secs: u64,
    len: u64,
    hash: String,
}

/// The stamp of the file at `path` right now, or None
/// when there is no readable file there.
fn stamp_of(path: &std::path::Path) -> Option<FileStamp> {
    let meta = std::fs::metadata(path).ok()?;
    let bytes = std::fs::read(path).ok()?;
    let modified_secs = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    Some(FileStamp {
        modified_secs,
        len: bytes.len() as u64,
        hash: ob_model::sha256_hex(&bytes),
    })
}

/// What the disk says now, against what the app
/// remembered: the save guard's whole decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StampVerdict {
    /// Same content: saving over it replaces only our own
    /// last save.
    Unchanged,
    /// Different content: someone or something else
    /// changed the file since we opened it.
    Changed,
    /// Gone: not a conflict; a normal save recreates it.
    Deleted,
}

fn stamp_verdict(remembered: &FileStamp, now: Option<&FileStamp>) -> StampVerdict {
    match now {
        None => StampVerdict::Deleted,
        Some(now) => {
            if now.len == remembered.len && now.hash == remembered.hash {
                StampVerdict::Unchanged
            } else {
                StampVerdict::Changed
            }
        }
    }
}

/// The editable text of the weekly story (announcements body paragraphs).
fn weekly_story_id(doc: &Document) -> Option<String> {
    doc.panels
        .iter()
        .flat_map(|p| p.frames.iter())
        .find(|f| f.role == ContentRole::Weekly)
        .and_then(|f| f.story_id.clone())
}

fn weekly_body_text(doc: &Document) -> String {
    let Some(sid) = weekly_story_id(doc) else {
        return String::new();
    };
    let Some(story) = doc.story(&sid) else {
        return String::new();
    };
    story
        .paragraphs
        .iter()
        .filter(|p| p.style == "body")
        .map(|p| p.text())
        .collect::<Vec<_>>()
        .join("\n")
}

/// One tabbed line of the order of service (J2): a label
/// ("Hymn of Praise") and this week's value ("432"), edited
/// as a labeled field on the This Sunday step.
#[derive(Debug, Clone, PartialEq)]
struct ServiceLine {
    /// Index of the paragraph within the service story.
    para_index: usize,
    label: String,
    value: String,
}

/// The story the service lines live in (J2): the one
/// holding the tabbed order lines. The lookup itself is
/// the model's (`ob_model::service_story_id`), shared
/// with Next Sunday's hymn clearing, so every surface
/// means the same story by "the service" (round 3,
/// R3-17).
fn service_story_id(doc: &Document) -> Option<String> {
    ob_model::service_story_id(doc)
}

/// The tabbed lines of the order of service (J2), in page
/// order: the label is the text before the first tab, the
/// value the text after the last tab (dot leaders live
/// between the two in the printed line).
fn service_lines(doc: &Document) -> Vec<ServiceLine> {
    let Some(sid) = service_story_id(doc) else {
        return Vec::new();
    };
    let Some(story) = doc.story(&sid) else {
        return Vec::new();
    };
    story
        .paragraphs
        .iter()
        .enumerate()
        .filter(|(_, p)| p.text().contains('\t'))
        .map(|(i, p)| ServiceLine {
            para_index: i,
            label: p.text().split('\t').next().unwrap_or("").trim().to_string(),
            value: p
                .text()
                .rsplit('\t')
                .next()
                .unwrap_or("")
                .trim()
                .to_string(),
        })
        .collect()
}

/// Rewrite one service line's value (J2), keeping the label
/// exactly as it stands, first tab included.
fn set_service_line(doc: &mut Document, para_index: usize, new_value: &str) {
    let Some(sid) = service_story_id(doc) else {
        return;
    };
    let Some(story) = doc.story_mut(&sid) else {
        return;
    };
    let Some(para) = story.paragraphs.get_mut(para_index) else {
        return;
    };
    let text = para.text();
    if let Some(tab) = text.find('\t') {
        let prefix = text[..=tab].to_string();
        para.set_text(&format!("{prefix}{new_value}"));
    }
}

/// One hymn value in its three field parts (R-2a), the
/// This Sunday step's hymn row: hymnal, number, title.
fn hymn_field_values(value: &str) -> [String; 3] {
    let e = ob_model::hymns::parse_hymn_value(value);
    [
        e.hymnal.unwrap_or_default(),
        e.number.unwrap_or_default(),
        e.title.unwrap_or_default(),
    ]
}

/// Where the church's hymn memory lives (R-2a): beside
/// settings.json, in the same app-data folder. It is the
/// church's own learned pairs, so it travels with the
/// office's settings, never inside a bulletin file.
pub fn hymn_memory_path() -> std::path::PathBuf {
    let dir = settings_path()
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    dir.join("hymn-memory.json")
}

/// How reading the hymn memory file went, so the app can
/// say once, in plain words, when the file could not be
/// trusted — the settings file's courtesy (SettingsRead),
/// extended to its sibling (S-MIN-2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HymnMemoryRead {
    /// The file read clean, or there was no file yet.
    Clean,
    /// The file was there but could not be read as a hymn
    /// memory at all; it was set aside as
    /// hymn-memory-unreadable.json and the memory starts
    /// fresh.
    SetAside,
}

/// Move an unreadable hymn memory file out of the way,
/// under a name that says what happened, so the next launch
/// starts clean and the damaged copy is still on disk for
/// anyone who goes looking (the settings/journal pattern).
/// An older set-aside is replaced: one damaged copy is
/// enough.
fn set_aside_unreadable_hymn_memory(path: &std::path::Path) -> bool {
    if !path.exists() {
        return false;
    }
    let to = path.with_file_name("hymn-memory-unreadable.json");
    std::fs::rename(path, &to).is_ok()
}

/// The hymn memory as it stands on disk. A file that is
/// simply not there starts the memory over in silence, as
/// it always has: it is learned data, rebuilt from the
/// church's bulletins as they are saved, never the only
/// copy of anything. A file that is there but does not
/// parse is set aside with its bytes preserved and the
/// fact reported in the read state (S-MIN-2) — the old
/// code reset it silently, unlike every sibling file.
fn load_hymn_memory_from(path: &std::path::Path) -> (ob_model::hymns::HymnMemory, HymnMemoryRead) {
    let Ok(bytes) = std::fs::read(path) else {
        return (
            ob_model::hymns::HymnMemory::default(),
            HymnMemoryRead::Clean,
        );
    };
    match serde_json::from_slice(&bytes) {
        Ok(memory) => (memory, HymnMemoryRead::Clean),
        Err(_) => {
            set_aside_unreadable_hymn_memory(path);
            (
                ob_model::hymns::HymnMemory::default(),
                HymnMemoryRead::SetAside,
            )
        }
    }
}

fn load_hymn_memory() -> (ob_model::hymns::HymnMemory, HymnMemoryRead) {
    load_hymn_memory_from(&hymn_memory_path())
}

/// Feed one document to the hymn memory (R-2a): the
/// document's own date is the Sunday its pairs were
/// confirmed on. Answers true when the memory learned
/// something new, so the caller knows to write it back.
fn learn_hymns(memory: &mut ob_model::hymns::HymnMemory, doc: &Document) -> bool {
    let Some(date) = doc
        .fields
        .get("occasion_date")
        .and_then(|iso| chrono::NaiveDate::parse_from_str(iso, "%Y-%m-%d").ok())
        .map(simple_of)
    else {
        return false;
    };
    memory.learn_from_document(doc, date)
}

/// The "Sermon:" paragraph, as (story id, paragraph index):
/// in the service story when it is there, else the first
/// story that has one.
fn sermon_para(doc: &Document) -> Option<(String, usize)> {
    let find_in = |story: &ob_model::Story| {
        story
            .paragraphs
            .iter()
            .position(|p| p.text().trim_start().starts_with("Sermon:"))
    };
    if let Some(sid) = service_story_id(doc) {
        if let Some(story) = doc.story(&sid) {
            if let Some(i) = find_in(story) {
                return Some((sid, i));
            }
        }
    }
    doc.stories
        .iter()
        .find_map(|s| find_in(s).map(|i| (s.id.clone(), i)))
}

/// Take the `{...}` field tokens out of a line, so the words
/// a person typed can be told from the words the app fills
/// in by itself.
fn strip_field_tokens(text: &str) -> String {
    let mut out = String::new();
    let mut in_token = false;
    for c in text.chars() {
        match c {
            '{' => in_token = true,
            '}' => in_token = false,
            _ if !in_token => out.push(c),
            _ => {}
        }
    }
    out
}

/// This week's sermon title (J2): what the "Sermon:" line
/// says once the field tokens are taken out. Empty when the
/// line carries only the pastor token, and when the bulletin
/// has no sermon line at all.
/// The participant values (this week's serving names)
/// from the document, one per serving line the church's
/// profile shows, in shown order: the This Sunday rows'
/// starting text (R-2b).
fn participant_values(doc: &Document) -> Vec<String> {
    doc.profile
        .serving_lines
        .iter()
        .map(|name| doc.participant_value_for_name(name))
        .collect()
}

fn sermon_title(doc: &Document) -> String {
    let Some((sid, i)) = sermon_para(doc) else {
        return String::new();
    };
    let Some(story) = doc.story(&sid) else {
        return String::new();
    };
    let sermon_text = story.paragraphs[i].text();
    let after = sermon_text
        .trim_start()
        .strip_prefix("Sermon:")
        .unwrap_or("");
    strip_field_tokens(after).trim().to_string()
}

/// Set the sermon title (J2). The {Church.pastor} token,
/// when the line has it, is preserved and stays after the
/// title: it prints the pastor's name there, as it always
/// has. An empty title restores the bare line.
fn set_sermon_title(doc: &mut Document, title: &str) {
    let Some((sid, i)) = sermon_para(doc) else {
        return;
    };
    let title = title.trim();
    let Some(story) = doc.story_mut(&sid) else {
        return;
    };
    let Some(para) = story.paragraphs.get_mut(i) else {
        return;
    };
    let had_token = para.text().contains("{Church.pastor}");
    let new_text = match (title.is_empty(), had_token) {
        (true, true) => "Sermon: {Church.pastor}".to_string(),
        (true, false) => "Sermon:".to_string(),
        (false, true) => format!("Sermon: {title} {{Church.pastor}}"),
        (false, false) => format!("Sermon: {title}"),
    };
    para.set_text(&new_text);
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.startup_done {
            self.startup();
        }
        // The families painting may name in THIS frame are
        // the ones the active definitions bind. egui takes
        // new definitions at the next frame, so this snapshot
        // (taken before the refresh below) stays the truth
        // for everything painted in this frame; a family
        // registered a moment ago is simply not named yet
        // (round 2, NOTE 9).
        self.bound_families = ctx.fonts(|f| {
            f.families()
                .iter()
                .filter_map(|fam| match fam {
                    egui::FontFamily::Name(name) => Some(name.to_string()),
                    _ => None,
                })
                .collect()
        });
        // Register the library faces once, up front: the
        // previews draw in them (UI audit round 2, batch 3),
        // and they must not wait for a first visit to the
        // Fonts window.
        if !self.fonts_loaded {
            self.refresh_fonts(ctx);
        }
        self.poll_journal(ctx);
        self.poll_import(ctx);
        self.import_review_window(ctx);
        self.contribution_review_window(ctx);
        self.paste_window(ctx);
        self.poll_analyzer(ctx);
        self.analyzer_review_window(ctx);
        self.capture_key(ctx);
        self.keys_window(ctx);
        self.frame_snapshot = self.canonical_snapshot();
        // A gesture must not span a focus change: if the user
        // clicked into a text field mid-gesture, the gesture is
        // over (audit C2: an orphaned snapshot would swallow
        // later edits into one wrong undo step).
        if self.gesture_start.is_some() && ctx.memory(|m| m.focused().is_some()) {
            self.end_gesture();
        }
        // A gesture left idle (nudges finished, mouse up) becomes
        // one Undo step after a short pause.
        if self.gesture_start.is_some()
            && self
                .gesture_touch
                .map(|t| t.elapsed() > std::time::Duration::from_millis(1200))
                .unwrap_or(false)
            && !ctx.input(|i| i.pointer.any_down())
        {
            self.end_gesture();
        }
        // Closing the Pages window mid-typing finishes the
        // session instead of orphaning it (which also blocked
        // global undo, since editing was never over).
        if !self.show_pages && self.editing.is_some() {
            self.commit_edit();
        }
        // Closing the Pages window while a loose sheet is
        // on the canvas puts the sheet back (R-3): the
        // canvas is where a sheet is worked on, and a sheet
        // left swapped in with no canvas open would strand
        // the bulletin inside out.
        if !self.show_pages && self.insert_edit.is_some() {
            self.leave_insert_editing();
        }
        // While the recovery offer waits for its answer, it
        // gates the whole app (J9): draw the journal tick and
        // the recovery window over a plain background, and
        // nothing else. The step panels, the top bar, and the
        // global keys stay unrendered until one of the
        // window's two buttons is chosen; editing behind an
        // unanswered "your work from last time" is how
        // recovered work gets lost.
        if self.recovery.is_some() {
            self.journal_tick();
            self.recovery_window(ctx);
            return;
        }
        // Global keys, when no text box has the keyboard (text
        // boxes keep their own undo while you type in them) and
        // no key is being captured for the Keys window. Every
        // key comes from the keymap: presets set the defaults,
        // and any change makes the set Custom (see Settings).
        if ctx.memory(|m| m.focused()).is_none()
            && self.editing.is_none()
            && self.key_capture.is_none()
        {
            if self.action_pressed(ctx, "undo") {
                self.undo();
            } else if self.action_pressed(ctx, "redo") {
                self.redo();
            } else if self.action_pressed(ctx, "save") {
                self.save_flow();
            } else if self.action_pressed(ctx, "open") {
                self.open_flow();
            } else if self.action_pressed(ctx, "new") {
                self.goto_step(0);
                self.note(
                    "Step 1 is where a fresh bulletin starts: pick a shape and it \
                     rebuilds. Nothing you have open is touched until you pick one."
                        .into(),
                );
            } else if self.action_pressed(ctx, "pages") {
                self.show_pages = !self.show_pages;
            } else if self.action_pressed(ctx, "fonts") {
                self.show_fonts = !self.show_fonts;
            } else if self.action_pressed(ctx, "next_step") && self.step < 4 {
                let next = self.step + 1;
                self.goto_step(next);
            } else if self.action_pressed(ctx, "prev_step") && self.step > 0 {
                let prev = self.step - 1;
                self.goto_step(prev);
            } else if self.show_pages && self.action_pressed(ctx, "next_page") {
                self.page_by(1);
            } else if self.show_pages && self.action_pressed(ctx, "prev_page") {
                self.page_by(-1);
            }
        }
        self.journal_tick();
        self.recovery_window(ctx);
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.heading("OpenBulletin");
                // The version, small and quiet, so "which
                // version are you running?" has an answer on
                // screen during phone help (UI audit round 2).
                ui.label(
                    egui::RichText::new(format!("v{}", env!("CARGO_PKG_VERSION")))
                        .small()
                        .color(egui::Color32::from_gray(110)),
                );
                // The bar is grouped the way an office worker's
                // hands expect (UI audit round 2): the file
                // family together, then undo and redo, then the
                // windows. Same buttons, same words.
                if ui.button("New bulletin").clicked() {
                    self.goto_step(0);
                    self.note(
                        "Step 1 is where a fresh bulletin starts: pick a shape and it \
                         rebuilds. Nothing you have open is touched until you pick one."
                            .into(),
                    );
                }
                let open_label = if self.pending_open == Some(PendingOpen::Plain) {
                    "Leave unsaved work and open?"
                } else {
                    "Open bulletin..."
                };
                if ui.button(open_label).clicked() {
                    self.open_flow();
                }
                if ui.button("Save bulletin").clicked() {
                    self.save_flow();
                }
                ui.separator();
                if ui
                    .add_enabled(
                        self.editing.is_none() && !self.undo_stack.is_empty(),
                        egui::Button::new("Undo"),
                    )
                    .clicked()
                {
                    self.undo();
                }
                if ui
                    .add_enabled(
                        self.editing.is_none() && !self.redo_stack.is_empty(),
                        egui::Button::new("Redo"),
                    )
                    .clicked()
                {
                    self.redo();
                }
                ui.separator();
                if ui.button("Pages").clicked() {
                    self.show_pages = true;
                }
                if ui.button("Fonts").clicked() {
                    self.show_fonts = true;
                }
                if ui.button("Shortcuts").clicked() {
                    self.show_keys = true;
                }
                if ui.button("Text size").clicked() {
                    self.show_comfort = true;
                }
                ui.separator();
                ui.label(format!("Step {} of 5: {}", self.step + 1, STEPS[self.step]));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.step < 4
                        && ui
                            .button(
                                egui::RichText::new(format!("Next: {}", STEPS[self.step + 1]))
                                    .strong(),
                            )
                            .clicked()
                    {
                        let next = self.step + 1;
                        self.goto_step(next);
                    }
                    if self.step > 0 && ui.button("Back").clicked() {
                        let prev = self.step - 1;
                        self.goto_step(prev);
                    }
                });
            });
            // The one sentence that orients the whole step
            // gets emphasis (UI audit round 2, batch 3).
            ui.label(egui::RichText::new(GUIDANCE[self.step]).strong());
        });

        egui::SidePanel::left("steps")
            .resizable(true)
            .default_width(if self.step == 4 { 560.0 } else { 300.0 })
            .show(ctx, |ui| {
                for (i, name) in STEPS.iter().enumerate() {
                    // Only the current step is marked. Steps
                    // above it used to claim "done" for work
                    // that was only walked past; the claim is
                    // gone until a marker can be earned from
                    // the document itself (UI audit round 2).
                    let marker = if i == self.step { ">" } else { "" };
                    if ui
                        .selectable_label(
                            i == self.step,
                            egui::RichText::new(format!("{marker} {}. {name}", i + 1))
                                .strong_if(i == self.step),
                        )
                        .clicked()
                    {
                        self.goto_step(i);
                    }
                }
                ui.separator();
                // The step itself scrolls: the long steps
                // (Start, Print) hold more than a small
                // window shows, and egui panels clip what
                // does not fit, buttons and all (UI audit
                // round 2, blocker B1). The step list above
                // and the status line below stay put.
                egui::ScrollArea::vertical()
                    .id_salt("step-scroll")
                    .show(ui, |ui| {
                        match self.step {
                    0 => {
                        // While a review window waits for its
                        // answer, the doors stay shut: a second
                        // pick would replace the review nobody
                        // answered (UI audit round 2).
                        let doors_enabled =
                            self.import_review.is_none() && self.analyzer_review.is_none();
                        let doc_state = self
                            .path
                            .as_ref()
                            .and_then(|p| self.settings.doc_state(&p.to_string_lossy()))
                            .cloned();
                        let (headline, next_line) = start_status(
                            &self.doc,
                            self.announcements.trim().is_empty(),
                            chrono::Local::now().date_naive(),
                            doc_state.as_ref(),
                        );
                        ui.label(egui::RichText::new(headline).strong());
                        ui.label(next_line);
                        if is_starter_sample(self.path.is_some(), &self.doc.profile.name) {
                            ui.label(
                                "This is a starter sample, not your church's bulletin yet. \
                                 Open an old bulletin, or fill in your church below to \
                                 make it yours.",
                            );
                        }
                        ui.separator();
                        // The first door: the bulletin this
                        // office worked on last, offered back
                        // when it is still on disk. It opens
                        // through the same guard as every
                        // other door (UI audit round 2).
                        let remembered = self.settings.last_bulletin.clone();
                        let still_there = remembered
                            .as_deref()
                            .map(|s| std::path::Path::new(s).exists())
                            .unwrap_or(false);
                        if let Some((last_path, last_name)) =
                            last_bulletin_door(remembered.as_deref(), still_there)
                        {
                            ui.heading("Pick up where you left off");
                            ui.label(format!(
                                "The bulletin this office worked on last: {last_name}."
                            ));
                            let last_label = if self.pending_path.is_some() {
                                "Leave unsaved work and open it?"
                            } else {
                                "Open it"
                            };
                            if ui
                                .add_enabled(doors_enabled, egui::Button::new(last_label))
                                .clicked()
                            {
                                if let Some(path) = self.pending_path.take() {
                                    self.open_old_bulletin(path);
                                } else {
                                    self.open_old_bulletin(last_path);
                                }
                            }
                            ui.separator();
                        }
                        ui.heading("Start from last week");
                        ui.label("The usual week: open last Sunday's bulletin and we move it to this Sunday. Last week's announcements clear; everything standing stays.");
                        let last_week_label =
                            if self.pending_open == Some(PendingOpen::LastWeek) {
                                "Leave unsaved work and use last week's?"
                            } else {
                                "Use last week's bulletin..."
                            };
                        if ui
                            .add_enabled(doors_enabled, egui::Button::new(last_week_label))
                            .clicked()
                            && self.open_guard(PendingOpen::LastWeek)
                        {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter(
                                    "Old bulletins",
                                    &["pdf", "pub", "idml", "sla", "obd"],
                                )
                                .add_filter("All files", &[] as &[&str])
                                .pick_file()
                            {
                                self.open_last_week(path);
                            }
                        }
                        ui.separator();
                        ui.heading("Bring in an old bulletin");
                        ui.label("Open a bulletin made somewhere else: a PDF, a Publisher file, an InDesign file (IDML), or a Scribus file. A web page or newsletter file (HTML, like a Mailchimp export) comes in the same door too: its words arrive in reading order, ready to arrange. A music file (MusicXML, from a music program) comes in the same door: its notes are drawn at a size people can sing from, as boxes you can move. One button for all of them; the app works out which kind it is, and what comes over arrives as a draft you can check. Publisher files ride in through LibreOffice (a free program) when it is installed, and through OpenBulletin's own reader when it is not; for InDesign, export the file there as IDML first.");
                        if self.import_rx.is_some() {
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.label("Reading your old bulletin. Big files can take a minute. Your work on screen is safe while you wait.");
                            });
                        } else {
                            let door_label = if self.pending_path.is_some() {
                                "Leave unsaved work and open?"
                            } else {
                                "Open an old bulletin..."
                            };
                            if ui
                                .add_enabled(doors_enabled, egui::Button::new(door_label))
                                .clicked()
                            {
                                if let Some(path) = self.pending_path.take() {
                                    self.open_old_bulletin(path);
                                } else if let Some(path) = rfd::FileDialog::new()
                                    .add_filter(
                                        "Old bulletins and music",
                                        &[
                                            "pdf", "pub", "idml", "sla", "obd", "html", "htm",
                                            "musicxml", "mxl",
                                        ],
                                    )
                                    .add_filter("All files", &[] as &[&str])
                                    .pick_file()
                                {
                                    self.open_old_bulletin(path);
                                }
                            }
                        }
                        ui.separator();
                        // New parts from the church's people
                        // (R-1): what has arrived in the
                        // inbox, what is still waited for, the
                        // paste door, and the sender's card.
                        if self.inbox_dirty {
                            self.refresh_inbox();
                        }
                        let bulletin_date = self
                            .doc
                            .fields
                            .get("occasion_date")
                            .and_then(|iso| {
                                ob_model::parse_flexible_date(iso, today_simple())
                            });
                        let parts_heading = match self.doc.fields.get("occasion_date_long") {
                            Some(long) if !long.trim().is_empty() => {
                                format!("New parts for {long}")
                            }
                            _ => "New parts for this Sunday".to_string(),
                        };
                        ui.heading(parts_heading);
                        let review_open = self.contribution_review.is_some();
                        let groups = group_inbox(&self.inbox_items);
                        let mut grouped: std::collections::HashSet<usize> =
                            std::collections::HashSet::new();
                        for g in &groups {
                            grouped.extend(g.items.iter().copied());
                        }
                        let mut open_idx: Option<usize> = None;
                        if self.inbox_items.is_empty() {
                            ui.label("Nothing new has arrived.");
                        }
                        for g in &groups {
                            let from_label = if g.from.trim().is_empty() {
                                "No name given".to_string()
                            } else {
                                g.from.clone()
                            };
                            let date_label = g
                                .for_date
                                .map(ob_extract::contribution::format_date_long)
                                .unwrap_or_else(|| "no Sunday named".to_string());
                            if g.items.len() > 1 {
                                ui.label(format!(
                                    "{from_label}, for {date_label}: {} versions; the newest is on top.",
                                    g.items.len()
                                ));
                            } else {
                                ui.label(format!("{from_label}, for {date_label}:"));
                            }
                            for &i in &g.items {
                                let item = &self.inbox_items[i];
                                ui.horizontal(|ui| {
                                    ui.label(format!(
                                        "{} ({})",
                                        item.file_name, item.fingerprint
                                    ));
                                    if ui
                                        .add_enabled(
                                            !review_open,
                                            egui::Button::new("Open"),
                                        )
                                        .clicked()
                                    {
                                        open_idx = Some(i);
                                    }
                                });
                            }
                        }
                        for (i, item) in self.inbox_items.iter().enumerate() {
                            if grouped.contains(&i) {
                                continue;
                            }
                            if let Err(reason) = &item.parsed {
                                ui.label(format!(
                                    "{} could not be read: {}",
                                    item.file_name, reason
                                ));
                            }
                        }
                        if let Some(i) = open_idx {
                            let item = &self.inbox_items[i];
                            if let Ok(c) = item.parsed.clone() {
                                let source = item.path.clone();
                                let text = item.text.clone();
                                self.open_contribution_review(c, Some(source), text);
                            }
                        }
                        if !self.doc.profile.expected_parts.is_empty() {
                            let arrived: Vec<ob_extract::contribution::Contribution> =
                                self.inbox_items
                                    .iter()
                                    .filter_map(|i| i.parsed.clone().ok())
                                    .collect();
                            let filed: Vec<(
                                ob_extract::contribution::Contribution,
                                Option<ob_model::SimpleDate>,
                            )> = self
                                .inbox_filed
                                .iter()
                                .filter_map(|i| {
                                    i.parsed.clone().ok().map(|c| (c, i.filed_for))
                                })
                                .collect();
                            let filed_refs: Vec<(
                                &ob_extract::contribution::Contribution,
                                Option<ob_model::SimpleDate>,
                            )> = filed.iter().map(|(c, f)| (c, *f)).collect();
                            let waiting = still_waiting(
                                &self.doc.profile.expected_parts,
                                &arrived,
                                &filed_refs,
                                bulletin_date,
                            );
                            if waiting.is_empty() {
                                ui.label("Everything expected has arrived.");
                            } else {
                                ui.label(format!(
                                    "Still waiting for: {}",
                                    waiting.join(", ")
                                ));
                                if ui.button("Copy a reminder").clicked() {
                                    ui.ctx()
                                        .copy_text(reminder_text(&waiting, bulletin_date));
                                    self.note(
                                        "Copied. Paste it into your email or message to send it."
                                            .into(),
                                    );
                                }
                            }
                        }
                        ui.horizontal(|ui| {
                            if ui
                                .add_enabled(
                                    !review_open,
                                    egui::Button::new("Paste words someone sent..."),
                                )
                                .clicked()
                            {
                                self.show_paste = true;
                            }
                            if ui.button("Print a sender's card").clicked() {
                                self.print_sender_card();
                            }
                        });
                        ui.small(format!(
                            "Senders do not need this program: the card teaches the shape, and the same words work pasted from an email. Parts arrive as small files in the inbox folder: {}",
                            inbox_dir().display()
                        ));
                        ui.separator();
                        ui.heading("Learn from your old bulletins");
                        ui.label("Or give us old bulletins in any mix (PDFs, Publisher files, InDesign (IDML) files, Scribus files, or OpenBulletin files) and we will learn what stays the same every week and what changes, and suggest a template from them. Learning needs at least three bulletins that count: the other kinds count one-for-one, and PDFs count only in a group of three or more, because one or two PDFs alone cannot show what repeats.");
                        if self.analyzer_rx.is_some() {
                            ui.label("Studying your old bulletins...");
                        } else if ui
                            .add_enabled(
                                doors_enabled,
                                egui::Button::new("Pick old bulletins (3 or more)..."),
                            )
                            .clicked()
                        {
                            if let Some(files) = rfd::FileDialog::new()
                                .add_filter(
                                    "Old bulletins",
                                    &["pdf", "pub", "idml", "sla", "obd"],
                                )
                                .add_filter("All files", &[] as &[&str])
                                .pick_files()
                            {
                                let mut counts = ob_extract::learn::KindCounts::default();
                                for f in &files {
                                    if let Some(kind) = ob_extract::learn::classify_path(f)
                                    {
                                        counts.add(kind);
                                    }
                                }
                                match ob_extract::learn::check_pick(&counts) {
                                    Ok(()) => self.start_analyzer(files),
                                    Err(msg) => self.fail(msg),
                                }
                            }
                        }
                        ui.separator();
                        ui.heading("Start a new bulletin");
                        ui.label("Pick a shape and the bulletin starts over with it. Picking takes two clicks, because it replaces the bulletin on screen (which stays undoable).");
                        for p in Preset::all() {
                            let armed = self.shape_arm == Some(p);
                            let label = if armed {
                                "Click again to start fresh with this shape".to_string()
                            } else if self.preset == p {
                                format!("{} (the shape you have)", p.label())
                            } else {
                                p.label().to_string()
                            };
                            if ui.button(label).clicked() {
                                if armed {
                                    self.shape_arm = None;
                                    self.preset = p;
                                    self.rebuild();
                                } else {
                                    self.shape_arm = Some(p);
                                    self.note(
                                        "Click again to start fresh with this shape. \
                                         What you have open stays undoable."
                                            .into(),
                                    );
                                }
                            }
                        }
                        ui.separator();
                        ui.heading("Your church (prints on the cover)");
                        ui.label("Church name");
                        let resp_church_name = ui.add(
                            egui::TextEdit::singleline(&mut self.church_name)
                                .hint_text("Your church's name"),
                        );
                        if resp_church_name.lost_focus() {
                            let name = self.church_name.clone();
                            if self.doc.profile.name != name {
                                let before = snapshot(&self.doc);
                                self.doc.profile.name = name;
                                self.record_undo(before);
                            }
                        }
                        ui.label("Address");
                        let resp_profile_address = ui.add(
                            egui::TextEdit::singleline(&mut self.profile_address)
                                .hint_text("Street address"),
                        );
                        if resp_profile_address.lost_focus() && self.doc.profile.address != self.profile_address {
                                let before = snapshot(&self.doc);
                                self.doc.profile.address = self.profile_address.clone();
                                self.record_undo(before);
                        }
                        ui.label("Service times");
                        let resp_times = ui.add(
                            egui::TextEdit::singleline(&mut self.profile_service_times)
                                .hint_text("Like Sundays 9:30 AM"),
                        );
                        if resp_times.lost_focus()
                            && self.doc.profile.service_times != self.profile_service_times
                        {
                                let before = snapshot(&self.doc);
                                self.doc.profile.service_times =
                                    self.profile_service_times.clone();
                                self.record_undo(before);
                        }
                        ui.label("Online giving link (shows as “Give online:” on the back page)");
                        let resp_giving = ui.add(
                            egui::TextEdit::singleline(&mut self.profile_giving_url)
                                .hint_text("The web address for online giving"),
                        );
                        if resp_giving.lost_focus()
                            && self.doc.profile.giving_url != self.profile_giving_url
                        {
                                let before = snapshot(&self.doc);
                                self.doc.profile.giving_url =
                                    self.profile_giving_url.clone();
                                self.record_undo(before);
                        }
                        ui.label("Website");
                        let resp_profile_website = ui.add(
                            egui::TextEdit::singleline(&mut self.profile_website)
                                .hint_text("Your church's website"),
                        );
                        if resp_profile_website.lost_focus() && self.doc.profile.website != self.profile_website {
                                let before = snapshot(&self.doc);
                                self.doc.profile.website = self.profile_website.clone();
                                self.record_undo(before);
                        }
                        ui.label("Pastor");
                        let resp_profile_pastor = ui.add(
                            egui::TextEdit::singleline(&mut self.profile_pastor)
                                .hint_text("Your pastor's name"),
                        );
                        if resp_profile_pastor.lost_focus() && self.doc.profile.pastor != self.profile_pastor {
                                let before = snapshot(&self.doc);
                                self.doc.profile.pastor = self.profile_pastor.clone();
                                self.record_undo(before);
                        }
                        ui.label("Parts we usually get (for the still-waiting list on this page)");
                        let resp_expected = ui.add(
                            egui::TextEdit::multiline(&mut self.profile_expected_parts)
                                .hint_text("One per line, or separated by commas: Pastor, Music director, Sunday school")
                                .desired_rows(2),
                        );
                        if resp_expected.lost_focus() {
                            let parsed = parse_expected_parts(&self.profile_expected_parts);
                            if parsed != self.doc.profile.expected_parts {
                                let before = snapshot(&self.doc);
                                self.doc.profile.expected_parts = parsed;
                                self.record_undo(before);
                            }
                        }
                        ui.label("Children's names in the bulletin");
                        ui.horizontal(|ui| {
                            for (rule, words) in [
                                (ob_model::ChildrenNames::FullNames, "Full names"),
                                (ob_model::ChildrenNames::FirstNamesOnly, "First names only"),
                                (ob_model::ChildrenNames::NoNames, "No names"),
                            ] {
                                if ui
                                    .selectable_label(
                                        self.doc.profile.children_names == rule,
                                        words,
                                    )
                                    .clicked()
                                    && self.doc.profile.children_names != rule
                                {
                                    let before = snapshot(&self.doc);
                                    self.doc.profile.children_names = rule;
                                    self.record_undo(before);
                                }
                            }
                        });
                        ui.small("Your church's rule for naming children in print. The app repeats it at Check; it cannot tell which names are children's, so it only reminds.");
                        ui.label("Serving lines (the names your bulletin prints each week for the people serving)");
                        let resp_serving = ui.add(
                            egui::TextEdit::multiline(&mut self.profile_serving_lines)
                                .hint_text("One per line, or separated by commas: Readers, Ushers, Greeters, Communion assistants, Children's message, Acolytes")
                                .desired_rows(2),
                        );
                        if resp_serving.lost_focus() {
                            let parsed = parse_expected_parts(&self.profile_serving_lines);
                            if parsed != self.doc.profile.serving_lines {
                                // Two lines sharing one
                                // slug would share one set
                                // of names, and a line
                                // taking a built-in line's
                                // slug would split its row
                                // from its printed token:
                                // refuse the save before
                                // anything changes, leaving
                                // the typed lines in place
                                // for correction (round 5,
                                // RT5-4).
                                let conflict = ob_model::serving_line_slug_conflict(
                                    &self.doc.profile.serving_lines,
                                    &parsed,
                                    &self.doc.profile.serving_aliases,
                                );
                                if let Some(conflict) = conflict {
                                    let sentence = serving_line_conflict_sentence(&conflict);
                                    self.fail(sentence);
                                } else {
                                    let before = snapshot(&self.doc);
                                    // A renamed line keeps
                                    // its names: the stored
                                    // values follow the
                                    // rename only when the
                                    // pairing is unambiguous
                                    // (round 3, R3-3; round
                                    // 4, R4-2; round 5, F3;
                                    // segment pairing and
                                    // recorded aliases since
                                    // round 6, R6-C). An
                                    // ambiguous pairing
                                    // refuses the lines
                                    // change as a whole:
                                    // nothing is paired,
                                    // moved, or cleared, and
                                    // the saved list stays
                                    // exactly as it was. A
                                    // line removed with
                                    // names still saved
                                    // loses them here, and
                                    // the status says so
                                    // (RT-6c).
                                    let old_lines =
                                        std::mem::take(&mut self.doc.profile.serving_lines);
                                    let migration =
                                        ob_model::migrate_serving_line_renames_detailed(
                                            &old_lines,
                                            &parsed,
                                            &mut self.doc.participants,
                                            &mut self.doc.profile.serving_aliases,
                                        );
                                    if migration.ambiguous {
                                        self.doc.profile.serving_lines = old_lines;
                                        // The refusal's words
                                        // name the kinds of
                                        // change the pairing
                                        // actually saw (round
                                        // 7, R7-10).
                                        let kinds = migration.ambiguous_kinds.unwrap_or(
                                            ob_model::ServingLineEditKinds {
                                                renamed: true,
                                                removed: false,
                                                added: false,
                                                moved: true,
                                            },
                                        );
                                        let sentence = ambiguous_serving_lines_sentence(&kinds);
                                        self.fail(sentence);
                                    } else {
                                        self.doc.profile.serving_lines = parsed;
                                        self.record_undo(before);
                                        let mut notes: Vec<String> = Vec::new();
                                        if !migration.removed.is_empty() {
                                            let sentence = removed_serving_lines_sentence(
                                                &migration.removed,
                                            );
                                            notes.push(sentence);
                                        }
                                        if !migration.unfollowed.is_empty() {
                                            let sentence = unfollowed_serving_lines_sentence(
                                                &migration.unfollowed,
                                            );
                                            notes.push(sentence);
                                        }
                                        if !notes.is_empty() {
                                            self.status = notes.join(" ");
                                        }
                                    }
                                }
                            }
                        }
                        ui.small("Serving lines are the names your bulletin prints each week for the people serving. Add your own, in the words you use. Each line gets its own row on This Sunday.");
                        ui.small("These print on the cover and carry into every week. They fill in the {Church.*} spots in your templates.");
                    }
                    1 => {
                        ui.heading("This Sunday");
                        ui.label(match &self.doc.occasion_time {
                            Some(t) if !t.trim().is_empty() => format!(
                                "Date: {} · {}",
                                self.doc.fields.get("occasion_date_long").cloned().unwrap_or_default(),
                                t.trim()
                            ),
                            _ => format!(
                                "Date: {}",
                                self.doc.fields.get("occasion_date_long").cloned().unwrap_or_default()
                            ),
                        });
                        ui.horizontal(|ui| {
                            ui.label("Change it:");
                            let date_edit = ui.text_edit_singleline(&mut self.date_input);
                            if date_edit.lost_focus() {
                                self.apply_edited_date();
                            }
                            if ui.button("Set date").clicked() {
                                self.apply_date();
                            }
                            if ui.button("Use this Sunday's date").clicked() {
                                let d = sunday_on_or_after(chrono::Local::now().date_naive());
                                self.date_input = d.format("%Y-%m-%d").to_string();
                                self.apply_date();
                            }
                            if ui.button("Use next Sunday's date").clicked() {
                                let d = sunday_on_or_after(chrono::Local::now().date_naive())
                                    + chrono::Duration::days(7);
                                self.date_input = d.format("%Y-%m-%d").to_string();
                                self.apply_date();
                            }
                        });
                        ui.small("Type the date any way you like: 2026-10-18, 10/18/2026, or October 18, 2026. This is the date printed on the bulletin.");
                        if let Some(words) = date_passed_sentence(
                            self.doc.fields.get("occasion_date").map(String::as_str),
                            chrono::Local::now().date_naive(),
                        ) {
                            ui.label(egui::RichText::new(words).color(self.theme.problem));
                        }
                        ui.label("Name of the service");
                        let mut sn = self.doc.fields.get("occasion_name").cloned().unwrap_or_default();
                        let resp_sn = ui.text_edit_singleline(&mut sn);
                        if resp_sn.lost_focus()
                            && self.doc.fields.get("occasion_name").map(|v| v.as_str()) != Some(sn.as_str())
                        {
                                let before = snapshot(&self.doc);
                                self.doc.fields.insert("occasion_name".to_string(), sn);
                                self.record_undo(before);
                        }
                        ui.label("Service time");
                        let mut st = self.doc.occasion_time.clone().unwrap_or_default();
                        let resp_st = ui.add(
                            egui::TextEdit::singleline(&mut st).hint_text("Like 9:00 AM"),
                        );
                        if resp_st.lost_focus()
                            && self.doc.occasion_time.as_deref() != Some(st.as_str())
                        {
                                let before = snapshot(&self.doc);
                                let trimmed = st.trim().to_string();
                                self.doc.occasion_time =
                                    if trimmed.is_empty() { None } else { Some(trimmed) };
                                if before != snapshot(&self.doc) {
                                    self.record_undo(before);
                                }
                        }
                        ui.label("Scripture reading");
                        let mut rd = self.doc.fields.get("reading").cloned().unwrap_or_default();
                        let resp_rd = ui.text_edit_singleline(&mut rd);
                        if resp_rd.lost_focus()
                            && self.doc.fields.get("reading").map(|v| v.as_str()) != Some(rd.as_str())
                        {
                                let before = snapshot(&self.doc);
                                self.doc.fields.insert("reading".to_string(), rd);
                                self.record_undo(before);
                        }
                        ui.small("The cover and the order of service fill themselves from these.");
                        // The service itself (J2): the order of
                        // service's tabbed lines (hymn numbers
                        // and friends) and the sermon title,
                        // edited here as labeled fields instead
                        // of hunted for on the page.
                        let lines = service_lines(&self.doc);
                        let has_sermon = sermon_para(&self.doc).is_some();
                        ui.separator();
                        if lines.is_empty() && !has_sermon {
                            ui.label("This bulletin has no service lines to fill in yet. They appear when the order of service has hymn lines.");
                        } else {
                            ui.label(egui::RichText::new("The service this week").strong());
                            if self.service_inputs.len() != lines.len() {
                                self.service_inputs =
                                    lines.iter().map(|l| l.value.clone()).collect();
                            }
                            if self.hymn_inputs.len() != lines.len() {
                                self.hymn_inputs = lines
                                    .iter()
                                    .map(|l| hymn_field_values(&l.value))
                                    .collect();
                            }
                            for (row, line) in lines.iter().enumerate() {
                                ui.label(&line.label);
                                if ob_model::hymns::is_hymn_label(&line.label) {
                                    // A hymn line is three
                                    // parts (R-2a): the book,
                                    // the number, the title.
                                    // Editing any part writes
                                    // the composed line back.
                                    let mut parts_changed = false;
                                    ui.horizontal(|ui| {
                                        ui.label("Hymnal");
                                        let r0 = ui.add(
                                            egui::TextEdit::singleline(
                                                &mut self.hymn_inputs[row][0],
                                            )
                                            .desired_width(64.0),
                                        );
                                        ui.label("Number");
                                        let r1 = ui.add(
                                            egui::TextEdit::singleline(
                                                &mut self.hymn_inputs[row][1],
                                            )
                                            .desired_width(56.0),
                                        );
                                        ui.label("Title");
                                        let r2 = ui.add(
                                            egui::TextEdit::singleline(
                                                &mut self.hymn_inputs[row][2],
                                            )
                                            .desired_width(220.0),
                                        );
                                        parts_changed =
                                            r0.lost_focus() || r1.lost_focus() || r2.lost_focus();
                                    });
                                    if parts_changed {
                                        let part = |s: &String| {
                                            let t = s.trim();
                                            if t.is_empty() {
                                                None
                                            } else {
                                                Some(t.to_string())
                                            }
                                        };
                                        let composed =
                                            ob_model::hymns::compose_hymn_value(
                                                &ob_model::hymns::HymnEntry {
                                                    hymnal: part(&self.hymn_inputs[row][0]),
                                                    number: part(&self.hymn_inputs[row][1]),
                                                    title: part(&self.hymn_inputs[row][2]),
                                                },
                                            );
                                        if composed != line.value {
                                            let before = snapshot(&self.doc);
                                            set_service_line(
                                                &mut self.doc,
                                                line.para_index,
                                                &composed,
                                            );
                                            if before != snapshot(&self.doc) {
                                                self.record_undo(before);
                                            }
                                            self.service_inputs[row] = composed.clone();
                                            self.hymn_inputs[row] =
                                                hymn_field_values(&composed);
                                        }
                                    }
                                    continue;
                                }
                                let resp =
                                    ui.text_edit_singleline(&mut self.service_inputs[row]);
                                if resp.lost_focus()
                                    && self.service_inputs[row].trim() != line.value
                                {
                                    let before = snapshot(&self.doc);
                                    let value =
                                        self.service_inputs[row].trim().to_string();
                                    set_service_line(&mut self.doc, line.para_index, &value);
                                    if before != snapshot(&self.doc) {
                                        self.record_undo(before);
                                    }
                                    self.service_inputs[row] = value;
                                }
                            }
                            if has_sermon {
                                ui.label("Sermon title");
                                let resp_sermon = ui.add(
                                    egui::TextEdit::singleline(&mut self.sermon_input)
                                        .hint_text("(the pastor's name fills in after it)"),
                                );
                                if resp_sermon.lost_focus()
                                    && self.sermon_input.trim() != sermon_title(&self.doc)
                                {
                                    let before = snapshot(&self.doc);
                                    let title = self.sermon_input.clone();
                                    set_sermon_title(&mut self.doc, &title);
                                    if before != snapshot(&self.doc) {
                                        self.record_undo(before);
                                    }
                                    self.sermon_input = sermon_title(&self.doc);
                                }
                            }
                        }
                        // Who serves this week (the serving
                        // lines): the rows are the church's
                        // own list from the Church Profile. A
                        // template prints the names where it
                        // carries the field words
                        // ({Sunday.readers} and friends). Next
                        // Sunday clears them: the new week has
                        // new people.
                        ui.separator();
                        ui.label(egui::RichText::new("Serving this week").strong());
                        let line_names = self.doc.profile.serving_lines.clone();
                        if self.participant_line_names != line_names
                            || self.participant_inputs.len() != line_names.len()
                        {
                            self.participant_inputs = participant_values(&self.doc);
                            self.participant_line_names = line_names.clone();
                        }
                        for (row, name) in line_names.iter().enumerate() {
                            ui.label(name.as_str());
                            let resp =
                                ui.text_edit_singleline(&mut self.participant_inputs[row]);
                            if resp.lost_focus() {
                                let value = self.participant_inputs[row].trim().to_string();
                                if value != self.doc.participant_value_for_name(name) {
                                    let before = snapshot(&self.doc);
                                    self.doc.set_participant_value(name, &value);
                                    if before != snapshot(&self.doc) {
                                        self.record_undo(before);
                                    }
                                    self.participant_inputs[row] = value;
                                }
                            }
                        }
                        ui.small("These names print where the bulletin carries the words for them. They start empty each week.");
                        ui.separator();
                        if ui.button("Start next Sunday").clicked() {
                            self.next_sunday();
                        }
                    }
                    2 => {
                        ui.heading("Announcements");
                        if ui.button("See the announcements on the page").clicked() {
                            self.show_pages = true;
                        }
                        ui.label("One announcement per line. Paste from email works; formatting is cleaned up for you.");
                        // The children's-names rule, repeated
                        // where names get typed (R-2b).
                        if self.doc.profile.children_names
                            != ob_model::ChildrenNames::FullNames
                        {
                            ui.small(format!(
                                "Your church's rule: {}.",
                                self.doc.profile.children_names.rule_words()
                            ));
                        }
                        let ann_resp = ui.add(
                            egui::TextEdit::multiline(&mut self.announcements)
                                .desired_rows(12)
                                .desired_width(f32::INFINITY),
                        );
                        if ann_resp.changed() {
                            self.announcements_dirty = true;
                        }
                        ui.separator();
                        let mut none_marked = self.doc.no_announcements;
                        if ui
                            .checkbox(&mut none_marked, "No announcements this week")
                            .changed()
                        {
                            let before = snapshot(&self.doc);
                            self.doc.no_announcements = none_marked;
                            if before != snapshot(&self.doc) {
                                self.record_undo(before);
                            }
                        }
                        ui.horizontal(|ui| {
                            ui.label("Keep through:");
                            let kt_resp = ui.text_edit_singleline(&mut self.ann_keep_through);
                            if kt_resp.lost_focus() {
                                self.apply_ann_keep_through();
                            }
                        });
                        ui.small("Leave the date blank for this Sunday only. With a date (year-month-day, like 2026-11-08), these announcements stay in the bulletin through that Sunday when you start next week's.");
                    }
                    3 => {
                        ui.heading("Check");
                        // Say what this step is NOT, before
                        // its OK lines can be read as a proof
                        // pass (J11): spell check is v0.5.
                        ui.small("This step checks that the bulletin will print right. It does not check spelling.");
                        // Problems read as problems: "Check:"
                        // lines in the problem red, "OK:" lines
                        // plain, and a note that knows its box
                        // gets a button that lands on it
                        // (UI audit round 2).
                        let notes = self.notes.clone();
                        for n in &notes {
                            if n.text.starts_with("Check:") {
                                ui.label(
                                    egui::RichText::new(&n.text)
                                        .color(self.theme.problem),
                                );
                            } else {
                                ui.label(&n.text);
                            }
                            if let Some(step) = n.step {
                                if ui.button("Fix this").clicked() {
                                    self.goto_step(step);
                                }
                            }
                            if let Some(key) = &n.forget_hymn {
                                // The office says the remembered
                                // pairing is the wrong one: drop
                                // it from the hymn memory, save
                                // the memory, and look again
                                // (round 3, R3-6). The pairing is
                                // not barred: the next bulletin
                                // saved with it teaches it again.
                                if ui.button("Forget this pairing").clicked() {
                                    if self.hymn_memory.forget_key(key) {
                                        self.persist_hymn_memory();
                                    }
                                    self.run_checks();
                                }
                            }
                            if let Some((pi, fi)) = n.target {
                                if ui.button("Fix this").clicked() {
                                    self.open_pages_at(pi, fi);
                                }
                            }
                            if n.fonts && ui.button("Fix this").clicked() {
                                self.show_fonts = true;
                            }
                            if n.inserts && ui.button("Fix this").clicked() {
                                self.show_inserts = true;
                            }
                        }
                        let findings = self.char_findings.clone();
                        for f in &findings {
                            ui.label(
                                egui::RichText::new(char_note(f))
                                    .color(self.theme.problem),
                            );
                            if weekly_story_id(&self.doc).as_deref()
                                == Some(f.story_id.as_str())
                            {
                                if ui.button("Fix the announcements").clicked() {
                                    self.goto_step(2);
                                }
                            } else if let Some((pi, fi)) = f.target {
                                if ui.button("Fix this in the Pages window").clicked() {
                                    self.open_pages_at(pi, fi);
                                }
                            } else if ui.button("Fix it in the Pages window").clicked() {
                                self.show_pages = true;
                            }
                        }
                        if ui.button("Check again").clicked() {
                            self.save_announcements();
                            self.run_checks();
                        }
                        ui.small("Too much text in a box? Use a finding's Fix this button to land on the box, then make the box bigger or shorten the words.");
                    }
                    _ => {
                        ui.heading("Print");
                        self.save_announcements();
                        self.sheet_preview_ui(ui);
                        ui.separator();
                        ui.label(egui::RichText::new("In the print window, use these settings").strong());
                        for line in self.print_checklist() {
                            ui.label(format!("- {line}"));
                        }
                        // The loose sheets print too (R-3),
                        // each on its own paper in its own
                        // count: one row per sheet, with the
                        // button that makes its own file.
                        if !self.doc.inserts.is_empty() {
                            ui.separator();
                            ui.label(egui::RichText::new("Also print").strong());
                            let sheets = self.doc.inserts.clone();
                            for ins in &sheets {
                                ui.horizontal(|ui| {
                                    ui.label(format!(
                                        "{} — {}, {}",
                                        ins.name,
                                        insert_copies_words(&ins.sheet),
                                        insert_paper_words(&ins.sheet)
                                    ));
                                    if ui.button("Print this sheet").clicked() {
                                        self.print_insert_sheet(ins);
                                    }
                                });
                            }
                            ui.small("The app prints the loose sheets; it cannot tuck them into the bulletins.");
                        }
                        for f in char_findings(&self.doc) {
                            ui.label(
                                egui::RichText::new(char_note(&f))
                                    .strong()
                                    .color(self.theme.problem),
                            );
                        }
                        if print_copy_stale(
                            self.print_fingerprint.as_deref(),
                            &self.frame_snapshot,
                        ) {
                            ui.label(
                                egui::RichText::new("This bulletin changed since the print copy was made. Make a fresh print copy before printing.")
                                    .strong()
                                    .color(self.theme.problem),
                            );
                        }
                        ui.small("Open makes a one-time copy to print right now. The buttons under \"Keep a copy as a file\" keep files you can reuse.");
                        // Bold means the step's one primary action
                        // (UI audit round 2): here, that is opening
                        // the print copy. Everything else is plain.
                        if ui.button(egui::RichText::new("Open the print copy").strong()).clicked() {
                            self.commit_if_editing();
                            self.pin_fonts();
                            let (bytes, report) = ob_pdf::export_imposed_pdf_report_with_cache(
                                &self.doc,
                                ob_pdf::ExportPurpose::ForPrinting,
                                &mut self.export_image_cache,
                            );
                            let path = std::env::temp_dir().join(format!(
                                "openbulletin-print-copy-{}.pdf",
                                std::process::id()
                            ));
                            match write_file_atomically(&path, &bytes).map_err(|e| e.to_string()).and_then(|_| open_with_os(&path)) {
                                Ok(()) => {
                                    self.print_fingerprint = Some(snapshot(&self.doc));
                                    self.store_doc_state();
                                    let guidance = self.current_print_guidance();
                                    self.status = format!(
                                        "Print copy opened in your PDF viewer ({}, {}).{}{} {guidance}",
                                        human_file_size(bytes.len()),
                                        ob_pdf::ExportPurpose::ForPrinting.words(),
                                        missing_pictures_sentence(&report),
                                        spoken_only_sentence(&report)
                                    );
                                }
                                Err(e) => self.status = e,
                            }
                        }
                        // The dialog guidance stands on the step,
                        // not only in the passing status line (J5).
                        ui.small(self.current_print_guidance());
                        ui.separator();
                        ui.label(egui::RichText::new("Keep a copy as a file").strong());
                        ui.label("Print copy: folded sheets, in the right order, ready for the copier.");
                        if ui.button("Save print copy (PDF)").clicked() {
                            self.commit_if_editing();
                            self.save_announcements();
                            if let Some(path) = rfd::FileDialog::new()
                                .set_file_name("bulletin-print.pdf")
                                .add_filter("PDF", &["pdf"])
                                .save_file()
                            {
                                self.pin_fonts();
                                let (bytes, report) = ob_pdf::export_imposed_pdf_report_with_cache(
                                    &self.doc,
                                    ob_pdf::ExportPurpose::ForPrinting,
                                    &mut self.export_image_cache,
                                );
                                match write_file_atomically(&path, &bytes) {
                                    Ok(()) => {
                                        self.print_fingerprint = Some(snapshot(&self.doc));
                                        self.store_doc_state();
                                        let guidance = self.current_print_guidance();
                                        self.status = format!(
                                            "Print copy written: {}, {}, {}.{}{} {guidance}",
                                            file_name_of(&path),
                                            human_file_size(bytes.len()),
                                            ob_pdf::ExportPurpose::ForPrinting.words(),
                                            missing_pictures_sentence(&report),
                                            spoken_only_sentence(&report)
                                        );
                                    }
                                    Err(e) => self.status = format!("Could not save the print copy: {e}"),
                                }
                            }
                        }
                        ui.label("Reading copy: one panel per page, for proofreading, email, and the website.");
                        if ui.button("Save a copy to read through").clicked() {
                            self.commit_if_editing();
                            self.save_announcements();
                            if let Some(path) = rfd::FileDialog::new()
                                .set_file_name("bulletin-reading.pdf")
                                .add_filter("PDF", &["pdf"])
                                .save_file()
                            {
                                self.pin_fonts();
                                let (bytes, report) = ob_pdf::export_reader_pdf_report_with_cache(
                                    &self.doc,
                                    ob_pdf::ExportPurpose::ForEmailOrWeb,
                                    &mut self.export_image_cache,
                                );
                                match write_file_atomically(&path, &bytes) {
                                    Ok(()) => {
                                        // No print fingerprint: a
                                        // reading copy is not the
                                        // print copy, and the
                                        // freshness warning must
                                        // never claim one exists
                                        // (UI audit round 2).
                                        self.status = format!(
                                            "Reading copy written: {}, {}, {}.{}{}",
                                            file_name_of(&path),
                                            human_file_size(bytes.len()),
                                            ob_pdf::ExportPurpose::ForEmailOrWeb.words(),
                                            missing_pictures_sentence(&report),
                                            spoken_only_sentence(&report)
                                        );
                                    }
                                    Err(e) => self.status = format!("Could not save the reading copy: {e}"),
                                }
                            }
                        }
                        ui.label("Large print: the same words, bigger, one page at a time, for readers who need them.");
                        if ui.button("Large-print copy (big words)...").clicked() {
                            self.commit_if_editing();
                            self.save_announcements();
                            if let Some(path) = rfd::FileDialog::new()
                                .set_file_name("bulletin-large-print.pdf")
                                .add_filter("PDF", &["pdf"])
                                .save_file()
                            {
                                self.pin_fonts();
                                self.save_large_print_copy(&path);
                            }
                        }
                        ui.label("Music proof: the hymn lines and the printed music on their own pages, to check or to hand to the music director.");
                        if ui.button("Music proof...").clicked() {
                            self.commit_if_editing();
                            self.save_announcements();
                            if let Some(path) = rfd::FileDialog::new()
                                .set_file_name("music-proof.pdf")
                                .add_filter("PDF", &["pdf"])
                                .save_file()
                            {
                                self.pin_fonts();
                                let (proof, counts) = music_proof_document(&self.doc);
                                let (bytes, _report) = ob_pdf::export_reader_pdf_report_with_cache(
                                    &proof,
                                    ob_pdf::ExportPurpose::ForPrinting,
                                    &mut self.export_image_cache,
                                );
                                self.status = match write_file_atomically(&path, &bytes) {
                                    Ok(()) => format!(
                                        "Music proof saved: {} hymn{}, {} page{} of printed music.{}",
                                        counts.hymns,
                                        if counts.hymns == 1 { "" } else { "s" },
                                        counts.music_pages,
                                        if counts.music_pages == 1 { "" } else { "s" },
                                        ob_model::spoken_only_left_out_sentence(
                                            counts.spoken_only_left_out
                                        )
                                    ),
                                    Err(e) => {
                                        format!("Could not save the music proof: {e}")
                                    }
                                };
                            }
                        }
                        ui.separator();
                        ui.label("A copy for another program: an InDesign (IDML) file or a Scribus file carries the words, their boxes, and their fonts, so someone with one of those programs can take the bulletin from here. A PDF copy carries the finished pages instead.");
                        ui.horizontal_wrapped(|ui| {
                            ui.label("Make a PDF copy:");
                            let current = self.settings.pdf_purpose();
                            for purpose in [
                                ob_pdf::ExportPurpose::ForEmailOrWeb,
                                ob_pdf::ExportPurpose::ForPrinting,
                                ob_pdf::ExportPurpose::ForPrintShop,
                                ob_pdf::ExportPurpose::FullResolution,
                            ] {
                                if ui
                                    .radio(current == purpose, purpose.label())
                                    .clicked()
                                {
                                    self.settings.pdf_purpose = purpose.slug().to_string();
                                    if let Err(e) = save_settings(&self.settings) {
                                        self.status = format!(
                                            "That choice could not be remembered: {e}"
                                        );
                                    }
                                }
                            }
                        });
                        if ui.button("Save a copy for another program...").clicked() {
                            self.commit_if_editing();
                            self.save_announcements();
                            if let Some(path) = rfd::FileDialog::new()
                                .set_file_name("bulletin.idml")
                                .add_filter("InDesign (IDML)", &["idml"])
                                .add_filter("Scribus", &["sla"])
                                .add_filter("PDF", &["pdf"])
                                .add_filter(
                                    "Markdown (the words, for a website or newsletter)",
                                    &["md"],
                                )
                                .add_filter(
                                    "Web page (the words, to read and search on a screen)",
                                    &["html"],
                                )
                                .save_file()
                            {
                                self.save_copy_for_program(&path);
                            }
                        }
                        if let Some(n) = &self.export_note {
                            ui.label(n.clone());
                        }
                        ui.separator();
                        ui.label(egui::RichText::new("Two-sided printing, first time on a printer?").strong());
                        ui.label("Every printer flips the paper its own way. Print one test sheet and tell us how it came out. We remember the answer for that printer.");
                        ui.label("These answers are kept on this computer, under the printer's name.");
                        if !self.settings.printers.is_empty() {
                            ui.horizontal(|ui| {
                                ui.label("Printers we know:");
                                let names: Vec<String> =
                                    self.settings.printers.keys().cloned().collect();
                                for n in names {
                                    if ui.selectable_label(self.printer_name == n, &n).clicked()
                                    {
                                        self.printer_name = n;
                                    }
                                }
                            });
                        }
                        ui.horizontal(|ui| {
                            ui.label("Printer:");
                            ui.add(
                                egui::TextEdit::singleline(&mut self.printer_name)
                                    .hint_text("Name this printer, like Church office printer"),
                            );
                        });
                        let known = self.settings.printers.get(&self.printer_name).cloned();
                        if let Some(setup) = &known {
                            ui.label(egui::RichText::new(&setup.instruction).strong());
                        }
                        if ui.button("Save a test sheet (PDF)").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .set_file_name("two-sided-test-sheet.pdf")
                                .add_filter("PDF", &["pdf"])
                                .save_file()
                            {
                                match write_file_atomically(&path, &ob_pdf::export_duplex_test_pdf()) {
                                    Ok(()) => self.status = "Test sheet saved. Print it with two-sided printing ON, and pick \"flip on long edge\" if the printer asks. Then answer below.".into(),
                                    Err(e) => self.status = format!("Could not save the test sheet: {e}"),
                                }
                            }
                        }
                        ui.label("Hold the FRONT so you can read it. Turn the sheet like a book page. The BACK was:");
                        ui.horizontal(|ui| {
                            for (answer, label) in [
                                ("up", "Right side up"),
                                ("down", "Upside down"),
                                ("manual", "My printer does one side at a time"),
                            ] {
                                if ui.button(label).clicked() {
                                    if self.printer_name.trim().is_empty() {
                                        // An answer learned under no
                                        // name would be quoted later
                                        // for the wrong printer (J4).
                                        self.fail(
                                            "Give the printer a name first, in the box above, so this answer is remembered under the right printer."
                                                .into(),
                                        );
                                        continue;
                                    }
                                    let setup = printer_setup_from_answer(answer, &self.printer_name);
                                    self.settings.printers.insert(self.printer_name.clone(), setup);
                                    match save_settings(&self.settings) {
                                        Ok(()) => self.status = format!("Remembered for the {}. You will see the setting to use right here.", self.printer_name),
                                        Err(e) => self.status = format!("Learned it, but could not save the note: {e}"),
                                    }
                                }
                            }
                        });
                        ui.separator();
                        // Plain, not strong: this step's one
                        // primary is the print copy above, and the
                        // top bar already carries the Next
                        // affordance (UI audit round 2).
                        if ui.button("Start next Sunday").clicked() {
                            self.next_sunday();
                        }
                    }
                        }
                    });
                ui.separator();
                // Thrown-away typing stays recoverable until
                // the next session or document (J3).
                if self.discarded_edit.is_some()
                    && ui.button("Bring the typed words back").clicked()
                {
                    self.restore_discarded_edit();
                }
                // The save guard stopped a save (R-1): its
                // two ways out live right under the status
                // line that explains the stop.
                if self.save_conflict.is_some() {
                    ui.horizontal(|ui| {
                        if ui.button("Save as a new file instead").clicked() {
                            self.save_as_new_file();
                        }
                        let replace_label = if self.replace_armed {
                            "Click again to replace it anyway"
                        } else {
                            "Replace it anyway"
                        };
                        if ui.button(replace_label).clicked() {
                            if self.replace_armed {
                                self.replace_ok = self.save_conflict.take();
                                self.replace_armed = false;
                                self.save_flow();
                            } else {
                                self.replace_armed = true;
                            }
                        }
                    });
                }
                if self.status_error {
                    ui.label(
                        egui::RichText::new(format!("Something needs attention: {}", self.status))
                            .color(self.theme.problem)
                            .strong(),
                    );
                } else {
                    // Body size, not small: this line is the
                    // app's confirmation for saves and copies,
                    // and it should be readable (UI audit
                    // round 2, batch 3).
                    ui.label(&self.status);
                }
            });

        let pages_were_open = self.show_pages;
        self.pages_window(ctx);
        self.picture_picker_window(ctx);
        self.clips_window(ctx);
        self.inserts_window(ctx);
        if pages_were_open && !self.show_pages && self.step == 3 {
            // A fix made on the canvas can clear a finding:
            // never leave the Check step showing a solved
            // problem as if it were still true (UI audit
            // round 2). A sheet on the canvas goes back
            // first: Check reads the bulletin whole (R-3).
            if self.insert_edit.is_some() {
                self.leave_insert_editing();
            }
            self.run_checks();
        }
        self.fonts_window(ctx);
        self.comfort_window(ctx);
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label(egui::RichText::new("Preview: what the congregation will hold").strong());
            ui.small("In your fonts, with lines breaking where they will print. The light outlines are guides; they do not print. The Pages window is where you move boxes.");
            egui::ScrollArea::vertical().show(ui, |ui| {
                let count = self.doc.panels.len();
                for idx in 0..count {
                    let panel = self.doc.panels[idx].clone();
                    ui.label(egui::RichText::new(&panel.name).strong());
                    // Fit the sheet to the window the user
                    // actually has; it grows and shrinks with it
                    // (stub sweep, 2026-10-15).
                    let scale = ((ui.available_width() - 4.0)
                        / (panel.size.w as f32 / 1000.0))
                    .clamp(0.3, 2.0);
                    let size = egui::vec2(
                        panel.size.w as f32 / 1000.0 * scale,
                        panel.size.h as f32 / 1000.0 * scale,
                    );
                    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
                    let painter = ui.painter_at(rect);
                    painter.rect_filled(rect, 2.0, egui::Color32::WHITE);
                    painter.rect_stroke(
                        rect,
                        2.0,
                        egui::Stroke::new(1.0_f32, egui::Color32::DARK_GRAY),
                    );
                    for frame in &panel.frames {
                        let fr = egui::Rect::from_min_size(
                            rect.min
                                + egui::vec2(
                                    frame.rect.x as f32 / 1000.0 * scale,
                                    frame.rect.y as f32 / 1000.0 * scale,
                                ),
                            egui::vec2(
                                frame.rect.w as f32 / 1000.0 * scale,
                                frame.rect.h as f32 / 1000.0 * scale,
                            ),
                        );
                        painter.rect_stroke(
                            fr,
                            1.0,
                            egui::Stroke::new(0.75_f32, egui::Color32::LIGHT_GRAY),
                        );
                    }
                    let lines = self.shaped_panel_lines(idx);
                    for line in &lines {
                        paint_line_segments(
                            &painter,
                            &self.font_rows,
                            &self.bound_families,
                            line,
                            scale,
                            |lx| rect.min + egui::vec2(lx * scale, line.y * scale),
                        );
                    }
                    ui.add_space(14.0);
                }
            });
        });
    }
}

/// Small extension so the step list can bold the current step inline.
trait StrongIf {
    fn strong_if(self, cond: bool) -> egui::RichText;
}
impl StrongIf for egui::RichText {
    fn strong_if(self, cond: bool) -> egui::RichText {
        if cond {
            self.strong()
        } else {
            self
        }
    }
}

/// The app's style: set at launch and again whenever the
/// comfort settings change (A1). The stock egui scale reads
/// small for this audience (office administrators, some
/// covering on unfamiliar machines), so body and button
/// text run a step larger even at Standard, and the
/// text-size steps (UiScale) grow every text style from
/// there, with padding and spacing a step behind.
/// Per-widget sizes in the canvas and previews are document
/// content, not chrome, and are not touched here.
fn app_style(settings: &Settings) -> egui::Style {
    let tf = settings.ui_scale.text_factor();
    let sf = settings.ui_scale.spacing_factor();
    let mut style = egui::Style::default();
    style.text_styles.insert(
        egui::TextStyle::Heading,
        egui::FontId::proportional(20.0 * tf),
    );
    style
        .text_styles
        .insert(egui::TextStyle::Body, egui::FontId::proportional(15.5 * tf));
    style.text_styles.insert(
        egui::TextStyle::Button,
        egui::FontId::proportional(15.5 * tf),
    );
    style.text_styles.insert(
        egui::TextStyle::Small,
        egui::FontId::proportional(12.5 * tf),
    );
    if let Some(mono) = style.text_styles.get_mut(&egui::TextStyle::Monospace) {
        mono.size *= tf;
    }
    style.spacing.button_padding = egui::vec2(
        style.spacing.button_padding.x * sf,
        style.spacing.button_padding.y * sf,
    );
    style.spacing.item_spacing = egui::vec2(
        style.spacing.item_spacing.x * sf,
        style.spacing.item_spacing.y * sf,
    );
    if settings.high_contrast {
        // True black words on white (A1). egui paints a
        // keyboard-focused widget with its "active"
        // visuals, so the focus ring lives there: wide and
        // black. Selection gets the same treatment.
        let mut v = egui::Visuals::light();
        v.override_text_color = Some(egui::Color32::BLACK);
        v.panel_fill = egui::Color32::WHITE;
        v.window_fill = egui::Color32::WHITE;
        v.extreme_bg_color = egui::Color32::WHITE;
        v.faint_bg_color = egui::Color32::from_gray(235);
        v.hyperlink_color = egui::Color32::from_rgb(0, 0, 200);
        v.warn_fg_color = egui::Color32::from_rgb(122, 66, 0);
        v.error_fg_color = egui::Color32::from_rgb(160, 0, 0);
        for w in [
            &mut v.widgets.noninteractive,
            &mut v.widgets.inactive,
            &mut v.widgets.hovered,
            &mut v.widgets.active,
            &mut v.widgets.open,
        ] {
            w.fg_stroke = egui::Stroke::new(1.0_f32, egui::Color32::BLACK);
        }
        v.widgets.inactive.bg_stroke = egui::Stroke::new(1.5_f32, egui::Color32::BLACK);
        v.widgets.hovered.bg_stroke = egui::Stroke::new(2.0_f32, egui::Color32::BLACK);
        v.widgets.active.bg_stroke = egui::Stroke::new(2.5_f32, egui::Color32::BLACK);
        v.selection.bg_fill = egui::Color32::from_rgb(255, 238, 150);
        v.selection.stroke = egui::Stroke::new(2.0_f32, egui::Color32::BLACK);
        style.visuals = v;
    } else {
        // Standard theme: the stock chrome, except that a
        // keyboard-focused widget must be findable. egui
        // paints focus with the "active" visuals, whose
        // outline is a hairline out of the box, so widen
        // and darken it (A1, item 4).
        let edge = if style.visuals.dark_mode {
            egui::Color32::from_gray(230)
        } else {
            egui::Color32::from_gray(40)
        };
        style.visuals.widgets.active.bg_stroke = egui::Stroke::new(1.75_f32, edge);
        style.visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.25_f32, edge);
    }
    style
}

/// The window icon: the app icon the bundle ships as icon.png
/// (the master of the adopted set in assets/icon/ — an open
/// bulletin on sage), decoded with the image crate the app
/// already depends on. A decode failure must never stop the
/// app, so it falls back to no icon.
fn app_icon() -> Option<egui::IconData> {
    let bytes = include_bytes!("../assets/icon.png");
    let img = image::load_from_memory(bytes).ok()?.into_rgba8();
    let (width, height) = img.dimensions();
    Some(egui::IconData {
        rgba: img.into_raw(),
        width,
        height,
    })
}

fn main() -> eframe::Result<()> {
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1180.0, 820.0])
        .with_min_inner_size([940.0, 620.0]);
    if let Some(icon) = app_icon() {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "OpenBulletin",
        options,
        Box::new(|cc| {
            // The saved comfort settings (A1) shape the
            // very first frame, not just later ones.
            let (settings, _) = load_settings();
            cc.egui_ctx.set_style(app_style(&settings));
            Ok(Box::new(App::default()))
        }),
    )
}

/// Shape the whole document the way the PDF path does and record
/// the lines per imposed sheet side.
/// Shape one panel's text frames into lines, in panel-local
/// point coordinates. Shared by the print preview (which offsets
/// them onto sheets) and the Pages canvas (which draws them in
/// place), so both always agree.
/// A paragraph style by id, or the default body style when
/// the document has no style with that id (the fallback the
/// canvas preview has always used).
fn style_or_body(doc: &Document, id: &str) -> ob_model::ParagraphStyle {
    doc.style(id)
        .cloned()
        .unwrap_or_else(|| ob_model::ParagraphStyle {
            id: "body".into(),
            name: "Body".into(),
            font_size: 11_000,
            bold: false,
            italic: false,
            align: ob_model::Align::Left,
            space_after: 4_000,
            color: ob_model::Rgb::BLACK,
            font_family: "Liberation Sans".into(),
            font_sha256: None,
        })
}

/// Every caret stop inside one paragraph layout, in
/// paragraph-local byte offsets: line starts and ends plus
/// every cluster boundary, sorted and deduped.
fn cluster_stops(layout: &ob_text::text_layout::TextLayout) -> Vec<usize> {
    let mut stops: Vec<usize> = Vec::new();
    for line in layout.lines() {
        stops.push(line.range.start);
        stops.push(line.range.end);
        for c in &line.clusters {
            stops.push(c.range.start);
            stops.push(c.range.end);
        }
    }
    stops.sort_unstable();
    stops.dedup();
    stops
}

/// The next word boundary in `text` from `pos`: words are
/// runs of non-whitespace. Moving forward lands on the end
/// of the current or next word; moving back on its start.
/// Byte-wise: ASCII whitespace is never part of a multi-byte
/// character, so stops stay on char boundaries.
fn word_stop(text: &str, pos: usize, forward: bool) -> usize {
    let bytes = text.as_bytes();
    let is_ws = |b: u8| (b as char).is_whitespace();
    if forward {
        let mut i = pos;
        while i < bytes.len() && !is_ws(bytes[i]) {
            i += 1;
        }
        if i == pos {
            while i < bytes.len() && is_ws(bytes[i]) {
                i += 1;
            }
            while i < bytes.len() && !is_ws(bytes[i]) {
                i += 1;
            }
        }
        i
    } else {
        let mut i = pos;
        while i > 0 && is_ws(bytes[i - 1]) {
            i -= 1;
        }
        while i > 0 && !is_ws(bytes[i - 1]) {
            i -= 1;
        }
        i
    }
}

/// One panel's shaped lines, cut from the document's
/// thread layouts (T1): each frame draws its slice of its
/// story, so the canvas shows a threaded story's words
/// exactly once, flowing box to box, the same lines the
/// PDF writer prints.
fn panel_lines_from_threads(
    doc: &Document,
    panel_idx: usize,
    panel: &ob_model::Panel,
    threads: &std::collections::HashMap<String, ob_text::threading::StoryThread>,
) -> Vec<PreviewLine> {
    let mut lines: Vec<PreviewLine> = Vec::new();
    for (fi, frame) in panel.frames.iter().enumerate() {
        if frame.kind == ob_model::FrameKind::Shape {
            continue;
        }
        let Some(sid) = &frame.story_id else { continue };
        let Some(thread) = threads.get(sid.as_str()) else {
            continue;
        };
        let Some(slice) = thread.slice_for(panel_idx, fi) else {
            continue;
        };
        for piece in &slice.paras {
            let style = &piece.style;
            // A spoken-only paragraph is tagged on its first
            // line in this box, and only where the paragraph
            // starts: a continuation piece in a later box
            // gets no second tag (the words there are the
            // middle of a paragraph that began tagged).
            let para_spoken = doc
                .story(sid)
                .and_then(|s| s.paragraphs.get(piece.para))
                .map(|p| p.spoken_only)
                .unwrap_or(false);
            let para_web_hidden = doc
                .story(sid)
                .and_then(|s| s.paragraphs.get(piece.para))
                .map(|p| p.web_hidden)
                .unwrap_or(false);
            for (li, line) in piece.layout.lines().iter().enumerate() {
                lines.push(PreviewLine {
                    text: piece.layout.text()[line.range.clone()].to_string(),
                    size: style.font_size as f32 / 1000.0,
                    x: frame.rect.x as f32 / 1000.0,
                    y: (piece.top + (li as ob_model::Mpt + 1) * piece.line_h) as f32 / 1000.0,
                    w: line.width,
                    align: style.align,
                    font_family: style.font_family.clone(),
                    bold: style.bold,
                    italic: style.italic,
                    segs: line_segments(line, piece.layout.text()),
                    spoken_tag: para_spoken && li == 0 && piece.text_range.start == 0,
                    web_tag: para_web_hidden && li == 0 && piece.text_range.start == 0,
                });
            }
        }
    }
    lines
}

/// Which paragraph of the edit buffer the text cursor is in
/// (count the newlines before it), for the style picker.
/// The Sunday on or after a date (the date itself when it is
/// already a Sunday).
fn sunday_on_or_after(d: chrono::NaiveDate) -> chrono::NaiveDate {
    use chrono::Datelike;
    let wd = d.weekday().num_days_from_sunday() as i64;
    d + chrono::Duration::days((7 - wd) % 7)
}

/// What the autosave journal should do this tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum JournalAction {
    /// Touch nothing (someone is deciding about recovered work,
    /// or another live instance owns the journal).
    Hold,
    /// The screen matches the saved file: the journal copy is
    /// no longer needed.
    Clear,
    /// The screen has unsaved work the journal does not hold yet.
    Write,
}

/// The journal decision, pure so it can be tested: the tick once
/// deleted the only copy of recovered work two seconds after
/// launch, while its own dialog was still asking (closing
/// audit, found by three lanes independently).
fn journal_decision(
    screen_eq_saved: bool,
    screen_eq_journal: bool,
    recovery_pending: bool,
    peer_alive: bool,
) -> JournalAction {
    if recovery_pending || peer_alive {
        return JournalAction::Hold;
    }
    if screen_eq_saved {
        if screen_eq_journal {
            JournalAction::Hold
        } else {
            JournalAction::Clear
        }
    } else if screen_eq_journal {
        JournalAction::Hold
    } else {
        JournalAction::Write
    }
}

/// The native .pub rescue (phases 2b and 2c): our own
/// reader, no LibreOffice needed. The words land in the
/// boxes Publisher put them in when the file's layout can
/// be read; when it cannot, the words still come over in
/// reading order. The review note says which happened,
/// plainly, with the loss counts when anything in the file
/// could not be read or carried.
fn native_pub_review(
    path: &std::path::Path,
    name: &str,
    profile: ChurchProfile,
) -> Result<ImportReview, String> {
    let bytes = ob_extract::read_open_door_file(path)?;
    let source_bytes = bytes.len();
    let cfb = ob_extract::cfb::Cfb::open(bytes)?;
    let x = ob_extract::pub_layout::extract_pub_positioned(&cfb, name, source_bytes)?;
    let (dump, loss) = (x.dump, x.loss);
    if dump.spans.is_empty() {
        return Err("the built-in reader found no words in that Publisher file".into());
    }
    let out =
        ob_extract::draft::document_from_dump(&dump, profile, false, &current_library_families());
    let mut note = if x.positioned {
        "These words came from your Publisher file with OpenBulletin's own \
         reader, so no other program was needed, and they are in the boxes \
         Publisher put them in. Pieces marked Check this need a look: their \
         font could not be matched to a free one, or they are master-page \
         pieces, tables, or styled text art."
            .to_string()
    } else {
        "These words came from your Publisher file with OpenBulletin's own \
         reader, so no other program was needed. Their places on the page \
         could not be read this time, so the words are in reading order. \
         Every piece is marked Check this."
            .to_string()
    };
    if loss.stories_unresolved > 0 {
        note.push_str(&format!(
            " {} of {} pieces of text in the file could not be read and were left out.",
            loss.stories_unresolved, loss.stories_total
        ));
    }
    if loss.stories_unplaced > 0 {
        note.push_str(&format!(
            " The words from {} {} could not be placed in a box and are on \
             the last pages in reading order.",
            loss.stories_unplaced,
            if loss.stories_unplaced == 1 {
                "piece"
            } else {
                "pieces"
            }
        ));
    }
    if loss.stories_overflowed > 0 {
        note.push_str(&format!(
            " {} {} had more words than fit on the page; the words that fit \
             stayed in their box and the rest are on the last pages in \
             reading order.",
            loss.stories_overflowed,
            if loss.stories_overflowed == 1 {
                "piece"
            } else {
                "pieces"
            }
        ));
    }
    if loss.pictures_found > loss.pictures_carried {
        note.push_str(&format!(
            " The file holds {} {}; pictures do not come over yet, and each \
             picture's place is marked on its page.",
            loss.pictures_found,
            if loss.pictures_found == 1 {
                "picture"
            } else {
                "pictures"
            }
        ));
    }
    Ok(ImportReview {
        doc: out.doc,
        items: out.items,
        workdir: None,
        source_name: name.to_string(),
        note: Some(note),
    })
}

#[cfg(test)]
mod door_gate_tests {
    use super::*;

    /// HF-2 pin, one real door end to end: a sparse file
    /// whose stated length is past the open-door limit
    /// (set_len allocates nothing, so the pin is instant)
    /// is refused by the native Publisher door in plain
    /// words, at the gate, before any of it is read.
    #[test]
    fn native_pub_door_refuses_an_oversize_file_at_the_door() {
        let path =
            std::env::temp_dir().join(format!("ob-app-door-gate-{}.pub", std::process::id()));
        let file = std::fs::File::create(&path).expect("create sparse file");
        file.set_len(ob_extract::MAX_OPEN_DOOR_BYTES + 1)
            .expect("state an oversize length");
        drop(file);
        let err = match native_pub_review(&path, "big.pub", ChurchProfile::default()) {
            Ok(_) => panic!("the door refuses an oversize file"),
            Err(e) => e,
        };
        assert!(err.contains("too large"), "{err}");
        assert!(err.contains("512 megabytes"), "{err}");
        let _ = std::fs::remove_file(&path);
    }
}

/// A finished PDF rescue waiting for the admin's verdict:
/// the draft, its review list, and the folder holding the
/// rendered page pictures.
/// A contribution awaiting its Accept/Edit/Cancel answer
/// (roles build R-1). The parsed words are what Accept
/// applies; the text they arrived as is what gets filed,
/// so nothing is ever lost.
struct ContributionReview {
    contribution: ob_extract::contribution::Contribution,
    /// The inbox file the words came from; None for pasted
    /// words (Accept files the pasted text itself).
    source: Option<std::path::PathBuf>,
    /// The words as they stand for filing: the text that
    /// arrived, replaced by the rebuilt text when Edit
    /// changes it.
    text: String,
    /// Wrong-week guard: the first click on Accept only
    /// arms when the words name a different Sunday, or
    /// a Sunday in relative words alone (round 6, R6-8).
    armed: bool,
    editing: bool,
    edit_from: String,
    edit_for: String,
    edit_keep: String,
    /// One raw markdown body per part, parallel to
    /// `contribution.parts` until Apply re-parses.
    edit_parts: Vec<String>,
    edit_kinds: Vec<ob_extract::contribution::PartKind>,
    edit_error: Option<String>,
}

struct ImportReview {
    doc: Document,
    items: Vec<ob_extract::draft::ImportItem>,
    /// Where the page pictures live, for PDF rescues. IDML
    /// imports have none.
    workdir: Option<std::path::PathBuf>,
    source_name: String,
    /// An honesty line shown in the review (IDML pictures).
    note: Option<String>,
}

/// A finished Template Analyzer run awaiting the verdict.
struct AnalyzerReview {
    doc: Document,
    items: Vec<ob_extract::analyzer::TemplateItem>,
    family: usize,
    corpus: usize,
    /// The mix the template was learned from, in plain words
    /// ("Learned from 9 files: 6 old PDFs, ...").
    mix: String,
    workdir: std::path::PathBuf,
    note: Option<String>,
}

/// A file-open that would leave unsaved work behind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PendingOpen {
    Plain,
    LastWeek,
}

/// The instance lock: a file in the settings folder holding
/// the running app's process id and its instance token. A
/// second instance reads it to know the journal already has
/// an owner.
fn instance_lock_path() -> std::path::PathBuf {
    settings_path()
        .parent()
        .map(|d| d.join("openbulletin.lock"))
        .unwrap_or_else(|| std::path::PathBuf::from("openbulletin.lock"))
}

/// This process's instance token (S-MIN-3): the process id
/// plus a nanosecond nonce, minted once per launch. A pid
/// alone is not the token — pids are recycled — but two
/// instances alive at the same time never share a pid, and
/// two launches at different times never share the nonce,
/// so the pair names one launch and no other. The token is
/// what the journal note records as its owner, and what the
/// lock records as its holder: retirement trusts the token
/// match, not the pid.
fn instance_token() -> &'static str {
    static TOKEN: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    TOKEN.get_or_init(|| {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        format!("{}-{nonce}", std::process::id())
    })
}

/// What an instance lock file says: the holder's process id
/// and instance token. Locks written before the token existed
/// carry only the pid; a lock too damaged to read says
/// nothing at all.
#[derive(Debug, Clone, PartialEq, Eq)]
struct InstanceLock {
    pid: Option<u32>,
    token: Option<String>,
}

fn read_instance_lock_from(lock: &std::path::Path) -> Option<InstanceLock> {
    let text = std::fs::read_to_string(lock).ok()?;
    let mut lines = text.lines();
    let pid = lines.next().and_then(|l| l.trim().parse::<u32>().ok());
    let token = lines
        .next()
        .map(|l| l.trim().to_string())
        .filter(|t| !t.is_empty());
    // A one-line legacy lock is just the pid.
    if pid.is_none() {
        let legacy = text.trim().parse::<u32>().ok()?;
        return Some(InstanceLock {
            pid: Some(legacy),
            token: None,
        });
    }
    Some(InstanceLock { pid, token })
}

/// The text a lock file carries for one holder: its pid on
/// the first line, its instance token on the second.
fn instance_lock_text(pid: u32, token: &str) -> String {
    format!("{pid}\n{token}\n")
}

/// A lock whose pid field cannot name a process at all —
/// missing or unparseable, 0 (which names a whole process
/// group, not a process), or above the namable range — is
/// corrupt, not a peer (R31-B, F6). The app's own writer
/// always writes its real pid, so such a lock was never
/// left by a healthy instance: it is a damaged file, and
/// it has no owner whose journal needs protecting. The
/// boundary is provability: a pid that *could* name a live
/// process is never judged by this rule, however unknown
/// its liveness is.
fn lock_pid_cannot_name_a_process(info: &InstanceLock) -> bool {
    match info.pid {
        None => true,
        Some(pid) => pid == 0 || pid > i32::MAX as u32,
    }
}

/// Does the lock at this path name exactly this instance?
/// The post-write re-read rides this: a claim is only as
/// true as the lock file it leaves behind.
fn lock_names_instance(lock: &std::path::Path, own_pid: u32, own_token: &str) -> bool {
    let expected_token = own_token.to_string();
    let expected = InstanceLock {
        pid: Some(own_pid),
        token: Some(expected_token),
    };
    read_instance_lock_from(lock) == Some(expected)
}

/// One step of the atomic claim: create the lock file
/// new, write this instance's text into it, and re-read it
/// before calling the create a win.
enum LockCreate {
    /// The file was created by this call and the re-read
    /// names this instance.
    Created,
    /// A lock file already stood at the path; nothing was
    /// written over it.
    AlreadyExists,
    /// The write itself failed, or the re-read did not
    /// name this instance. Either way the claim must
    /// answer false: it does not hold the lock.
    Failed,
}

fn try_create_instance_lock_from(
    lock: &std::path::Path,
    own_pid: u32,
    own_token: &str,
) -> LockCreate {
    use std::io::Write as _;
    let text = instance_lock_text(own_pid, own_token);
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    let mut file = match options.open(lock) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            return LockCreate::AlreadyExists;
        }
        Err(_) => return LockCreate::Failed,
    };
    if file.write_all(text.as_bytes()).is_err() || file.sync_all().is_err() {
        return LockCreate::Failed;
    }
    drop(file);
    if lock_names_instance(lock, own_pid, own_token) {
        LockCreate::Created
    } else {
        LockCreate::Failed
    }
}

/// Move a lock file that no live peer holds (dead owner,
/// corrupt, or unreadable) aside in one rename, so the
/// claim can take the path through the same create-new
/// step an unheld lock takes. The old file is not
/// deleted: it stays beside the lock as
/// `openbulletin.lock.displaced`, the set-aside shape the
/// journal and settings recovery paths already use.
/// Renaming is the arbiter between two claimants who both
/// judged the old lock takeable: the file can be moved
/// only once, and the loser of the rename finds the path
/// gone or already re-created and re-judges what stands
/// there instead of proceeding.
fn displace_instance_lock_from(lock: &std::path::Path) -> bool {
    // Only a regular file is ever displaced: anything else
    // standing at the lock path (a directory obstructing
    // it, say) is not a lock this claim may move, and the
    // claim answers false instead of clearing its way.
    match std::fs::metadata(lock) {
        Ok(metadata) if metadata.is_file() => {}
        _ => return false,
    }
    let mut displaced_name = lock.as_os_str().to_owned();
    displaced_name.push(".displaced");
    let displaced = std::path::PathBuf::from(displaced_name);
    // Only a previous displaced copy is ever removed, and
    // only so the rename has somewhere to land on
    // platforms whose rename will not replace a file.
    let _ = std::fs::remove_file(&displaced);
    std::fs::rename(lock, &displaced).is_ok()
}

/// The writing half of the claim (R31-B, F3): take the
/// lock atomically and truthfully, answering true only
/// when the lock file now names this instance. The check
/// half is `peer_instance_alive_from`, re-run here at
/// write time rather than trusted from an earlier read,
/// so a claimant that arrives after a peer (or a winning
/// claimant) judges what actually stands at the path:
/// - no file: create-new, so exactly one of two
///   simultaneous claimants creates it;
/// - a file naming a live peer: refused, untouched;
/// - a file naming this instance already: held;
/// - a file no live peer holds (dead owner, corrupt, or
///   unreadable): displaced in one rename, then taken
///   through the same create-new step, and a claimant
///   that loses that create race re-reads and proceeds
///   only if the winner's lock names it (it does not).
///
/// This is the only production writer of the lock file,
/// and no path deletes the lock.
fn acquire_instance_lock_from(lock: &std::path::Path, own_pid: u32, own_token: &str) -> bool {
    if peer_instance_alive_from(lock, own_pid, own_token) {
        return false;
    }
    if lock_names_instance(lock, own_pid, own_token) {
        return true;
    }
    match try_create_instance_lock_from(lock, own_pid, own_token) {
        LockCreate::Created => true,
        LockCreate::Failed => false,
        LockCreate::AlreadyExists => {
            // A lock stood at the path. Judge whoever
            // holds it now, at write time: a live peer (or
            // the winner of the create race) keeps it.
            if peer_instance_alive_from(lock, own_pid, own_token) {
                return false;
            }
            if lock_names_instance(lock, own_pid, own_token) {
                return true;
            }
            // No live peer holds it: take over through
            // the same atomic create-new step.
            if !displace_instance_lock_from(lock) {
                return false;
            }
            match try_create_instance_lock_from(lock, own_pid, own_token) {
                LockCreate::Created => true,
                // The create race was lost or the write
                // failed: proceed only if the lock that
                // stands there now names this instance.
                _ => lock_names_instance(lock, own_pid, own_token),
            }
        }
    }
}

/// What the platform can say about whether a process is
/// alive: the one liveness primitive the journal peer
/// machinery rides. `Unknown` is a real answer, not a
/// failure to be rounded off: where liveness cannot be
/// proven either way, callers fail toward treating the
/// peer as alive.
// On a platform with no primitive below, the answer can
// only ever be `Unknown`, so `Alive` and `Dead` are never
// constructed there; the variants exist on every platform
// because the callers' decision table is shared.
#[cfg_attr(not(unix), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PeerLiveness {
    Alive,
    Dead,
    Unknown,
}

/// Probe a pid's liveness without `/proc`: `/proc` does
/// not exist on macOS, so any rule built on its presence
/// reads every pid as gone there. On Unix, `kill(pid, 0)`
/// sends no signal and only reports whether the pid names
/// a live process: success means alive, ESRCH means no
/// such process, and EPERM means a live process owned by
/// someone else. On Windows the same question is asked
/// through `OpenProcess` + `GetExitCodeProcess` (see
/// below). Pids that cannot name a single foreign
/// process (0 names a whole process group; anything above
/// `i32::MAX` cannot be a pid at all) answer `Unknown`
/// without any syscall, on every platform: the primitive
/// cannot prove such a pid gone, and the corrupt-lock
/// rule that judges lock files naming one lives at the
/// lock layer (`lock_pid_cannot_name_a_process`), not
/// here. On a platform with neither primitive the answer
/// is always `Unknown` and callers keep their
/// conservative posture.
#[cfg(unix)]
#[allow(unsafe_code)]
fn peer_liveness(pid: u32) -> PeerLiveness {
    if pid == 0 || pid > i32::MAX as u32 {
        return PeerLiveness::Unknown;
    }
    // SAFETY: signal 0 sends nothing; it only asks the
    // kernel whether the pid exists and may be signalled.
    // The pid is a plain integer checked above, and no
    // memory is read or written.
    let rc = unsafe { libc::kill(pid as libc::pid_t, 0) };
    if rc == 0 {
        return PeerLiveness::Alive;
    }
    match std::io::Error::last_os_error().raw_os_error() {
        Some(code) if code == libc::ESRCH => PeerLiveness::Dead,
        Some(code) if code == libc::EPERM => PeerLiveness::Alive,
        _ => PeerLiveness::Unknown,
    }
}

#[cfg(windows)]
extern "system" {
    fn OpenProcess(
        desired_access: u32,
        inherit_handle: i32,
        process_id: u32,
    ) -> *mut std::ffi::c_void;
    fn GetExitCodeProcess(process: *mut std::ffi::c_void, exit_code: *mut u32) -> i32;
    fn CloseHandle(object: *mut std::ffi::c_void) -> i32;
}

/// The Windows liveness primitive (R31-B, F2): open the
/// process for querying and read its exit code. A process
/// that does not exist fails the open with
/// `ERROR_INVALID_PARAMETER` and is `Dead`; a process
/// that exists but may not be opened fails with
/// `ERROR_ACCESS_DENIED` and is `Alive`; an opened process
/// still running reports `STILL_ACTIVE`, and one that has
/// exited reports its exit code. Any other failure of the
/// query itself is `Unknown`, which callers read as alive.
#[cfg(windows)]
#[allow(unsafe_code)]
fn peer_liveness(pid: u32) -> PeerLiveness {
    if pid == 0 || pid > i32::MAX as u32 {
        return PeerLiveness::Unknown;
    }
    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
    const STILL_ACTIVE: u32 = 259;
    const ERROR_INVALID_PARAMETER: i32 = 87;
    const ERROR_ACCESS_DENIED: i32 = 5;
    // SAFETY: the calls only ask the kernel about the
    // process named by the plain integer pid checked
    // above. The handle, when one is returned, is owned
    // by this function and closed before it returns; the
    // exit code is written through a pointer to a local
    // this function owns. No other memory is read or
    // written, and no handle is used after it is closed.
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if handle.is_null() {
        return match std::io::Error::last_os_error().raw_os_error() {
            Some(code) if code == ERROR_INVALID_PARAMETER => PeerLiveness::Dead,
            Some(code) if code == ERROR_ACCESS_DENIED => PeerLiveness::Alive,
            _ => PeerLiveness::Unknown,
        };
    }
    let mut exit_code: u32 = 0;
    let queried = unsafe { GetExitCodeProcess(handle, &mut exit_code) };
    let _ = unsafe { CloseHandle(handle) };
    if queried == 0 {
        return PeerLiveness::Unknown;
    }
    if exit_code == STILL_ACTIVE {
        PeerLiveness::Alive
    } else {
        PeerLiveness::Dead
    }
}

#[cfg(not(any(unix, windows)))]
fn peer_liveness(pid: u32) -> PeerLiveness {
    let _ = pid;
    PeerLiveness::Unknown
}

/// Whether the process with this pid is provably not a live
/// OpenBulletin peer: the pid is dead, or the live process
/// there is plainly some other program — a recycled pid.
/// Where the process exists but its identity cannot be
/// established, this answers false: fail toward treating
/// the peer as alive and keeping its journal.
fn peer_provably_gone(pid: u32) -> bool {
    match peer_liveness(pid) {
        PeerLiveness::Dead => true,
        PeerLiveness::Alive => {
            #[cfg(unix)]
            {
                // The /proc identity refinement, applied
                // only where /proc is actually readable
                // (Linux): the exe link names the running
                // program; a name that is not this app means
                // the pid was recycled by an unrelated
                // process. Where /proc is absent or
                // unreadable, identity is unproven and the
                // peer is treated as alive.
                let proc_dir = std::path::PathBuf::from(format!("/proc/{pid}"));
                if let Ok(exe) = std::fs::read_link(proc_dir.join("exe")) {
                    let names_app = exe
                        .file_name()
                        .map(|n| n.to_string_lossy().to_lowercase().contains("openbulletin"))
                        .unwrap_or(false);
                    return !names_app;
                }
                if let Ok(cmdline) = std::fs::read(proc_dir.join("cmdline")) {
                    let text = String::from_utf8_lossy(&cmdline).to_lowercase();
                    if !text.is_empty() {
                        return !text.contains("openbulletin");
                    }
                }
                false
            }
            #[cfg(not(unix))]
            {
                false
            }
        }
        PeerLiveness::Unknown => false,
    }
}

/// Is another OpenBulletin process alive? The answer
/// rides the platform liveness primitive (with the
/// recycled-pid guard above) on every platform, with one
/// decision table (R31-B, F2): a live pid that is not
/// provably a recycled stranger is a live peer, and an
/// `Unknown` answer also reads as alive — muting a lone
/// instance's autosave is recoverable, while retiring a
/// live peer's journal is the loss this machinery exists
/// to prevent. A lock naming a foreign token whose owner
/// cannot be proven gone therefore reads as a live peer
/// everywhere, for the claim, for the `peer_alive` flag
/// startup derives from it, and for the journal hold.
/// The one exception is a corrupt lock (F6): a pid field
/// that cannot name a process at all names no owner, so
/// the lock is no peer however unknown liveness is.
fn peer_instance_alive_from(lock: &std::path::Path, own_pid: u32, own_token: &str) -> bool {
    let Some(info) = read_instance_lock_from(lock) else {
        return false;
    };
    if info.token.as_deref() == Some(own_token) {
        return false;
    }
    if lock_pid_cannot_name_a_process(&info) {
        return false;
    }
    let Some(pid) = info.pid else {
        return false;
    };
    if pid == own_pid {
        return false;
    }
    match peer_liveness(pid) {
        PeerLiveness::Alive => !peer_provably_gone(pid),
        PeerLiveness::Dead => false,
        PeerLiveness::Unknown => true,
    }
}

/// Startup's claim on the instance lock (R30-A, made
/// atomic and truthful by R31-B F3). Answers true only
/// when this instance took the lock and the lock file now
/// names it — the claim is a create-new claim with a
/// verified write and a post-write re-read, never a
/// check-then-plain-write, so two instances launched
/// together cannot both hold it, and a failed write
/// answers false. It answers false when a live peer holds
/// the lock, in which case the lock file is left exactly
/// as it was. Startup must never take the lock from a
/// live peer: the lock naming the peer is what makes the
/// journal token rules refuse this instance's later
/// retirements of the peer's journal, so overwriting it
/// at startup — the round-30 F1 hole — disarmed that
/// protection with the peer still running, and this
/// instance's next ordinary save then deleted the peer's
/// live journal. When no live peer holds the lock
/// (absent, unreadable, corrupt, or naming a dead owner,
/// per `peer_instance_alive_from`'s rules), the lock is
/// taken through `acquire_instance_lock_from`'s atomic
/// path.
fn claim_instance_lock() -> bool {
    claim_instance_lock_from(&instance_lock_path(), instance_token())
}

fn claim_instance_lock_from(lock: &std::path::Path, own_token: &str) -> bool {
    acquire_instance_lock_from(lock, std::process::id(), own_token)
}

/// May this instance retire (clear) a journal? (S-MIN-3.)
/// Only its own journal, or one whose owner is gone:
/// - a note naming this instance's token is its own;
/// - a note naming another token may be retired when that
///   owner is provably gone, or when the lock no longer
///   names that token — its owner exited and a later launch
///   (possibly this one) holds the lock now;
/// - a note naming another token while the lock still names
///   that same token belongs to the instance holding the
///   lock, and is never retired from here;
/// - a tokenless note (written before tokens existed) is
///   retired only when the lock names no other live holder.
///
/// Pure over its inputs, so the rules can be pinned directly.
fn journal_retirement_allowed(
    meta_token: Option<&str>,
    own_token: &str,
    lock_token: Option<&str>,
    owner_provably_gone: bool,
) -> bool {
    match meta_token {
        Some(token) if token == own_token => true,
        Some(token) => {
            if owner_provably_gone {
                return true;
            }
            lock_token != Some(token)
        }
        None => owner_provably_gone || lock_token.is_none() || lock_token == Some(own_token),
    }
}

/// Retire a journal through the token rules, clearing it
/// only when `journal_retirement_allowed` says this instance
/// may. Answers true when the journal was cleared.
fn retire_journal_from(
    obd: &std::path::Path,
    meta: &std::path::Path,
    lock: &std::path::Path,
    own_token: &str,
) -> bool {
    let meta_token = journal_token_from(meta);
    let lock_info = read_instance_lock_from(lock);
    // A corrupt lock (F6) names no owner at all: its token
    // does not hold the journal, and its "owner" counts as
    // gone, so a journal left under one is retirable
    // instead of wedged for as long as the damaged file
    // stands.
    let lock_is_corrupt = lock_info
        .as_ref()
        .map(lock_pid_cannot_name_a_process)
        .unwrap_or(false);
    let lock_token = if lock_is_corrupt {
        None
    } else {
        lock_info.as_ref().and_then(|i| i.token.as_deref())
    };
    // The owner is provably gone when the lock's pid names a
    // process that is dead or is plainly not this app. With
    // no pid to check, nothing is proven.
    let owner_provably_gone = lock_is_corrupt
        || lock_info
            .as_ref()
            .and_then(|i| i.pid)
            .map(peer_provably_gone)
            .unwrap_or(false);
    let allowed = journal_retirement_allowed(
        meta_token.as_deref(),
        own_token,
        lock_token,
        owner_provably_gone,
    );
    if allowed {
        clear_journal_from(obd, meta);
    }
    allowed
}

/// What a right-click on the canvas belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MenuTarget {
    Frame(usize),
    Guide(usize),
    Page,
}

/// A click ON a guide line belongs to the guide, even when a
/// frame covers it (blocker 8); then the frame under the click;
/// then a guide merely near the click; then the page itself.
fn choose_menu_target(
    on_line: Option<usize>,
    frame: Option<usize>,
    near: Option<usize>,
) -> MenuTarget {
    if let Some(gi) = on_line {
        MenuTarget::Guide(gi)
    } else if let Some(fi) = frame {
        MenuTarget::Frame(fi)
    } else if let Some(gi) = near {
        MenuTarget::Guide(gi)
    } else {
        MenuTarget::Page
    }
}

/// A dashed rectangle (an empty or lost picture box's
/// outline): egui has no dashed stroke, so it is drawn as
/// segments along each edge.
fn dashed_rect(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let stroke = egui::Stroke::new(1.0_f32, color);
    let dash = 6.0_f32;
    let gap = 4.0_f32;
    let mut x = rect.min.x;
    while x < rect.max.x {
        let end = (x + dash).min(rect.max.x);
        painter.line_segment(
            [egui::pos2(x, rect.min.y), egui::pos2(end, rect.min.y)],
            stroke,
        );
        painter.line_segment(
            [egui::pos2(x, rect.max.y), egui::pos2(end, rect.max.y)],
            stroke,
        );
        x += dash + gap;
    }
    let mut y = rect.min.y;
    while y < rect.max.y {
        let end = (y + dash).min(rect.max.y);
        painter.line_segment(
            [egui::pos2(rect.min.x, y), egui::pos2(rect.min.x, end)],
            stroke,
        );
        painter.line_segment(
            [egui::pos2(rect.max.x, y), egui::pos2(rect.max.x, end)],
            stroke,
        );
        y += dash + gap;
    }
}

/// The clause a copy's status line adds when some picture
/// boxes printed as "No picture yet" (G2): the copy is
/// still written; the sentence says what the boxes did.
fn missing_pictures_sentence(report: &ob_pdf::ExportReport) -> String {
    match report.missing_pictures {
        0 => String::new(),
        1 => " 1 picture box had no picture and printed as \"No picture yet\".".to_string(),
        n => format!(" {n} picture boxes had no picture and printed as \"No picture yet\"."),
    }
}

/// The spoken-only sentence for a status line, same shape
/// as the missing-pictures one: empty when nothing was left
/// out, so doors can append it unconditionally.
fn spoken_only_sentence(report: &ob_pdf::ExportReport) -> String {
    ob_model::spoken_only_left_out_sentence(report.spoken_only_left_out)
}

/// How many dots per inch a placed picture effectively has
/// in print, at the size it SHOWS (a Fill picture is trimmed
/// to its shown size, so its dots spread over the shown
/// picture, not the box). The lower of the two axes: the
/// worse direction is how blurry it can look. None when the
/// picture shows at no size at all (a box with zero width or
/// height): dividing by that zero used to yield an infinite
/// DPI that the Check step read as perfectly sharp (audit
/// Batch 3).
fn picture_effective_dpi(
    frame: &ob_model::Frame,
    placement: &ob_model::PicturePlacement,
    probe: &ob_image::Probe,
) -> Option<f64> {
    let (_, _, shown_w, shown_h) = ob_model::picture_shown_rect(
        placement.fit,
        probe.width as f64,
        probe.height as f64,
        frame.rect.w as f64,
        frame.rect.h as f64,
        placement.crop_x,
        placement.crop_y,
    );
    if shown_w <= 0.0 || shown_h <= 0.0 {
        return None;
    }
    let dpi_x = probe.width as f64 / (shown_w / 72_000.0);
    let dpi_y = probe.height as f64 / (shown_h / 72_000.0);
    Some(dpi_x.min(dpi_y))
}

/// The pictures a document carries that no box and no panel
/// background uses: the rescue's loose pictures, offered by
/// the picture picker as "Pictures from the old bulletin".
fn unplaced_doc_pictures(doc: &Document) -> Vec<ob_model::Asset> {
    let mut used: std::collections::HashSet<&str> = Default::default();
    for panel in &doc.panels {
        if let Some(bg) = &panel.background_image {
            used.insert(bg.as_str());
        }
        for frame in &panel.frames {
            if let Some(p) = &frame.picture {
                used.insert(p.asset.as_str());
            }
        }
    }
    doc.assets
        .iter()
        .filter(|a| a.kind == ob_model::AssetKind::Picture && !used.contains(a.sha256.as_str()))
        .cloned()
        .collect()
}

/// The sentence an "add a collection" flow reports, in
/// plain words, with a clause for every kind of file that
/// was left out and why (nothing is ever silently dropped).
fn collection_result_sentence(name: &str, report: &ob_clips::AddReport) -> String {
    let landed = report.added + report.converted;
    let mut out = if landed == 0 {
        format!("No clips were added to \"{name}\".")
    } else {
        format!(
            "{} clip{} added to \"{name}\".",
            landed,
            if landed == 1 { " was" } else { "s were" }
        )
    };
    let clause = |n: usize, one: String, many: String| -> Option<String> {
        if n == 0 {
            None
        } else if n == 1 {
            Some(one)
        } else {
            Some(many.replace("{n}", &n.to_string()))
        }
    };
    let clauses: Vec<Option<String>> = vec![
        clause(
            report.converted,
            " 1 old Windows picture (WMF/EMF) was converted so it can be used.".into(),
            " {n} old Windows pictures (WMF/EMF) were converted so they can be used.".into(),
        ),
        clause(
            report.skipped_not_pictures,
            " 1 file was not a picture and was left out.".into(),
            " {n} files were not pictures and were left out.".into(),
        ),
        clause(
            report.skipped_metafiles,
            " 1 old Windows picture (WMF/EMF) was left out: converting it needs LibreOffice on this computer.".into(),
            " {n} old Windows pictures (WMF/EMF) were left out: converting them needs LibreOffice on this computer.".into(),
        ),
        clause(
            report.skipped_convert_first,
            " 1 picture (AVIF or HEIC) was left out: this program cannot use that kind yet; convert it to JPEG or PNG first.".into(),
            " {n} pictures (AVIF or HEIC) were left out: this program cannot use that kind yet; convert them to JPEG or PNG first.".into(),
        ),
        clause(
            report.skipped_unreadable,
            " 1 picture file could not be read and was left out.".into(),
            " {n} picture files could not be read and were left out.".into(),
        ),
    ];
    for c in clauses.into_iter().flatten() {
        out.push_str(&c);
    }
    out
}

/// The WMF/EMF converter for the clip library (G2): when a
/// user-installed LibreOffice is on this computer (found
/// the same way the .pub bridge finds it), each old Windows
/// picture is converted to PNG through it, headless, in a
/// scratch folder. Without LibreOffice there is no
/// converter and the library counts those files and leaves
/// them out; they are never shown as blank thumbnails.
fn wmf_converter() -> Option<Box<ob_clips::MetafileConverter>> {
    let soffice = ob_extract::pub_bridge::find_soffice()?;
    Some(Box::new(move |name: &str, bytes: &[u8]| {
        let dir = std::env::temp_dir().join(format!(
            "openbulletin-wmf-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).ok()?;
        let input = dir.join(name);
        std::fs::write(&input, bytes).ok()?;
        let stem = name
            .rsplit_once('.')
            .map(|(s, _)| s.to_string())
            .unwrap_or_else(|| name.to_string());
        let output = dir.join(format!("{stem}.png"));
        // The 30 s bound the PROCESS, not only the wait
        // (round 2, R2-3): past it the converter is killed
        // and reaped, so the scratch folder below is never
        // deleted under a live LibreOffice.
        let mut cmd = std::process::Command::new(&soffice);
        cmd.arg("--headless")
            .arg("--norestore")
            .arg("--convert-to")
            .arg("png")
            .arg("--outdir")
            .arg(&dir)
            .arg(&input);
        let ok =
            ob_clips::run_converter_with_deadline(&mut cmd, std::time::Duration::from_secs(30));
        let bytes = if ok {
            std::fs::read(&output).ok()
        } else {
            None
        };
        let _ = std::fs::remove_dir_all(&dir);
        bytes
    }))
}

/// The Font Library's family names right now, for the
/// import doors: which source fonts count as "we have
/// it", and what stands in for the rest, is decided
/// against this list at import time (docs/FONTS.md,
/// wanted fonts). An unreadable library answers empty:
/// imports still work, and every non-bundled font is
/// simply wanted.
fn current_library_families() -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    if let Ok(lib) = ob_fonts::FontLibrary::open(ob_fonts::default_library_dir()) {
        for info in lib.scan() {
            if !out.contains(&info.family) {
                out.push(info.family);
            }
        }
    }
    out
}

fn build_preview(doc_in: &Document) -> Vec<PreviewSheet> {
    let padded = ob_impose::padded_document(doc_in);
    let doc = &padded;
    let mut shaper = ob_text::shaping::bundled_shaper();
    if let Ok(lib) = ob_fonts::FontLibrary::open(ob_fonts::default_library_dir()) {
        for info in lib.scan() {
            if let Ok(bytes) = lib.read(&info) {
                shaper.register_font(bytes);
            }
        }
    }
    // Thread layouts for the padded document (T1): the
    // preview cuts each panel's lines from the same slices
    // the canvas and the PDF writer use.
    let threads = ob_text::threading::layout_document_threads(&mut shaper, doc);
    let sides = ob_impose::impose(doc);
    let mut out = Vec::new();
    for side in &sides {
        let mut sheet = PreviewSheet {
            w: side.size.w as f32 / 1000.0,
            h: side.size.h as f32 / 1000.0,
            panels: Vec::new(),
        };
        for pl in &side.placements {
            let Some(panel) = doc.panels.get(pl.panel) else {
                continue;
            };
            let mut pp = PreviewPanel {
                x: pl.x as f32 / 1000.0,
                y: pl.y as f32 / 1000.0,
                w: panel.size.w as f32 / 1000.0,
                h: panel.size.h as f32 / 1000.0,
                name: panel.name.clone(),
                lines: Vec::new(),
                pictures: panel_pictures(panel, doc, pl.x as f32 / 1000.0, pl.y as f32 / 1000.0),
            };
            pp.lines = panel_lines_from_threads(doc, pl.panel, panel, &threads)
                .into_iter()
                .map(|mut l| {
                    l.x += pp.x;
                    l.y += pp.y;
                    l
                })
                .collect();
            sheet.panels.push(pp);
        }
        out.push(sheet);
    }
    out
}

/// Draw one sheet side with the egui painter: white sheet, fold
/// lines, panel outlines, panel names, and the shaped lines in
/// their own faces. The outlines and names are guides (the
/// caption above the preview says they do not print).
/// The placed pictures of one panel, in sheet points, for
/// the previews (G2): each picture frame's shown rect,
/// from the one geometry function every view shares.
fn panel_pictures(
    panel: &ob_model::Panel,
    doc: &Document,
    off_x: f32,
    off_y: f32,
) -> Vec<PreviewPicture> {
    let mut out = Vec::new();
    for frame in &panel.frames {
        if frame.kind != ob_model::FrameKind::Picture {
            continue;
        }
        let Some(placement) = &frame.picture else {
            continue;
        };
        let Some(bytes) = doc.asset_data(&placement.asset) else {
            continue;
        };
        let Ok(probe) = ob_image::probe(bytes) else {
            continue;
        };
        let box_pt = (frame.rect.w as f64 / 1000.0, frame.rect.h as f64 / 1000.0);
        let (sx, sy, sw, sh) = ob_model::picture_shown_rect(
            placement.fit,
            probe.width as f64,
            probe.height as f64,
            box_pt.0,
            box_pt.1,
            placement.crop_x,
            placement.crop_y,
        );
        let fx = frame.rect.x as f32 / 1000.0 + off_x;
        let fy = frame.rect.y as f32 / 1000.0 + off_y;
        out.push(PreviewPicture {
            asset: placement.asset.clone(),
            clip: egui::Rect::from_min_max(
                egui::pos2(fx, fy),
                egui::pos2(
                    fx + frame.rect.w as f32 / 1000.0,
                    fy + frame.rect.h as f32 / 1000.0,
                ),
            ),
            shown: egui::Rect::from_min_max(
                egui::pos2(fx + sx as f32, fy + sy as f32),
                egui::pos2(fx + (sx + sw) as f32, fy + (sy + sh) as f32),
            ),
            fit: placement.fit,
        });
    }
    out
}

fn draw_sheet(
    ui: &mut egui::Ui,
    sheet: &PreviewSheet,
    rows: &[FontRow],
    bound: &std::collections::HashSet<String>,
    textures: &std::collections::HashMap<String, egui::TextureHandle>,
) {
    let avail = ui.available_width().min(560.0);
    let scale = avail / sheet.w;
    let height = sheet.h * scale;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(avail, height), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    let origin = rect.min;
    let to_px = |x: f32, y: f32| egui::pos2(origin.x + x * scale, origin.y + y * scale);
    painter.rect_filled(rect, 2.0, egui::Color32::WHITE);
    painter.rect_stroke(
        rect,
        2.0,
        egui::Stroke::new(1.0_f32, egui::Color32::DARK_GRAY),
    );
    for panel in &sheet.panels {
        let prect = egui::Rect::from_min_max(
            to_px(panel.x, panel.y),
            to_px(panel.x + panel.w, panel.y + panel.h),
        );
        painter.rect_stroke(
            prect,
            0.0,
            egui::Stroke::new(0.75_f32, egui::Color32::LIGHT_GRAY),
        );
        painter.text(
            prect.min + egui::vec2(3.0, 2.0),
            egui::Align2::LEFT_TOP,
            &panel.name,
            egui::FontId::proportional(9.0),
            egui::Color32::from_gray(140),
        );
        // Pictures first, under the words (G2), with the
        // same crop geometry the canvas shows.
        for pic in &panel.pictures {
            let Some(tex) = textures.get(&pic.asset) else {
                continue;
            };
            let clip = egui::Rect::from_min_max(
                to_px(pic.clip.min.x, pic.clip.min.y),
                to_px(pic.clip.max.x, pic.clip.max.y),
            );
            let shown = egui::Rect::from_min_max(
                to_px(pic.shown.min.x, pic.shown.min.y),
                to_px(pic.shown.max.x, pic.shown.max.y),
            );
            let painter2 = painter.with_clip_rect(clip);
            if pic.fit == ob_model::PictureFit::ShowWhole {
                painter2.image(
                    tex.id(),
                    shown,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            } else {
                let fx = (pic.clip.width() / pic.shown.width()).clamp(0.0, 1.0);
                let fy = (pic.clip.height() / pic.shown.height()).clamp(0.0, 1.0);
                let ux = ((pic.clip.min.x - pic.shown.min.x) / pic.shown.width()).clamp(0.0, 1.0);
                let uy = ((pic.clip.min.y - pic.shown.min.y) / pic.shown.height()).clamp(0.0, 1.0);
                painter2.image(
                    tex.id(),
                    clip,
                    egui::Rect::from_min_max(egui::pos2(ux, uy), egui::pos2(ux + fx, uy + fy)),
                    egui::Color32::WHITE,
                );
            }
        }
        let clip = painter.with_clip_rect(prect);
        for line in &panel.lines {
            paint_line_segments(&clip, rows, bound, line, scale, |lx| to_px(lx, line.y));
        }
    }
}

/// A friendly paper name for a sheet size in milli-points.
fn paper_name(w: ob_model::Mpt, h: ob_model::Mpt) -> String {
    let (lw, lh) = (w.max(h), w.min(h));
    let orientation = if w >= h {
        "turned sideways (landscape)"
    } else {
        "upright (portrait)"
    };
    // The paper names are ob-model's one sheet-name
    // table (health audit E-13), keyed by the landscape
    // form this lookup has just normalized to.
    let name = match ob_model::sheet_name(ob_model::Size::new(lw, lh)) {
        Some(name) => name.to_string(),
        None => format!(
            "{:.1} x {:.1} in",
            lw as f32 / 72_000.0,
            lh as f32 / 72_000.0
        ),
    };
    format!("{name}, {orientation}")
}

/// What to choose in the PDF viewer's print window (J5).
/// The app does not print; the viewer does, so these sentences
/// are the bridge between our window and that dialog. The
/// learned branch quotes the checklist's own instruction
/// (`flip_words`), so the two can never disagree. Pure, so it
/// is testable without a window.
fn print_dialog_guidance(printer_known: bool, flip_words: Option<&str>) -> String {
    match (printer_known, flip_words) {
        (true, Some(words)) => format!(
            "In the print window: choose Actual size, never Fit to page. \
             Two-sided printing: {words} Print one sheet first and check \
             it, then print the rest."
        ),
        _ => "In the print window: choose Actual size, never Fit to page. \
              For two-sided printing, run the test sheet below first so \
              the copies flip the right way."
            .to_string(),
    }
}

/// The print-window settings for one document and printer, as
/// plain sentences. Pure, so it is testable without a window.
fn print_checklist_for(doc: &Document, settings: &Settings, printer_name: &str) -> Vec<String> {
    let sides = ob_impose::impose(doc);
    let (w, h) = sides
        .first()
        .map(|s| (s.size.w, s.size.h))
        .unwrap_or((doc.sheet.size.w, doc.sheet.size.h));
    let mut lines = vec![
        format!("Paper: {}", paper_name(w, h)),
        "Size: pick Actual size, or 100%. Never \"Fit to page\": it shrinks \
         the pages and the fold lands in the wrong place."
            .to_string(),
    ];
    match settings.printers.get(printer_name) {
        // One source of words: the checklist quotes the very
        // instruction the wizard learned, so the two can never
        // disagree (finding F6).
        Some(setup) => lines.push(setup.instruction.clone()),
        None => {
            // The name field starts empty now (J4): "this
            // printer" until the office names it.
            let who = if printer_name.trim().is_empty() {
                "this printer".to_string()
            } else {
                format!("the {printer_name}")
            };
            lines.push(format!(
                "Two-sided: we have not learned {who} yet. Print the \
                 test sheet below once and answer, and this line will name the \
                 exact setting."
            ))
        }
    }
    lines.push(
        "Copies: set the number in the print window. This file holds each \
         sheet once, in the right order."
            .to_string(),
    );
    lines
}

/// Open a file with the computer's default program (the PDF
/// viewer, for our print copies).
fn open_with_os(path: &std::path::Path) -> Result<(), String> {
    let mut cmd = if cfg!(target_os = "windows") {
        let mut c = std::process::Command::new("cmd");
        c.args(["/c", "start", ""]);
        c
    } else if cfg!(target_os = "macos") {
        std::process::Command::new("open")
    } else {
        std::process::Command::new("xdg-open")
    };
    cmd.arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("could not open the print copy: {e}"))
}

#[cfg(test)]
mod settings_tests {
    use super::*;

    fn doc_with_sheet() -> Document {
        let mut doc = Document {
            format_version: ob_model::FORMAT_VERSION,
            title: "T".into(),
            profile: ChurchProfile::default(),
            fields: Default::default(),
            styles: ob_model::default_styles(),
            stories: vec![ob_model::Story {
                id: "host-story".into(),
                paragraphs: vec![ob_model::Paragraph::new("body", "The bulletin's own words")],
                keep_through: None,
            }],
            panels: vec![ob_model::Panel {
                background_image: None,
                id: "host-panel".into(),
                name: "Cover".into(),
                size: ob_model::Size::new(396_000, 612_000),
                frames: vec![],
                guides: vec![],
            }],
            sheet: ob_model::SheetSpec {
                preset: "letter-bifold".into(),
                size: ob_model::Size::new(792_000, 612_000),
            },
            no_announcements: false,
            assets: vec![],
            asset_bytes: Default::default(),
            occasion_time: None,
            wanted_fonts: vec![],
            participants: Default::default(),
            inserts: vec![new_insert("sheet-1", "Prayer list")],
            applied_contributions: Vec::new(),
        };
        doc.fields
            .insert("occasion_date".into(), "2026-10-18".into());
        doc
    }

    #[test]
    fn save_time_absorb_never_gathers_outside_the_document_folder() {
        // H-A (HF-1): the Save-time gather is fenced the
        // same way the loader's is. A ".." reference and
        // an absolute reference to a real PNG canary
        // outside the document folder absorb nothing at
        // Save, while a legitimate sidecar in the
        // document folder still absorbs.
        const PNG_BYTES: &[u8] = b"\x89PNG\r\n\x1a\ncanary";
        let root = std::env::temp_dir().join(format!("ob-app-ha-fence-{}", std::process::id()));
        let sub = root.join("sub");
        std::fs::create_dir_all(&sub).expect("dir");
        std::fs::write(root.join("secret.png"), PNG_BYTES).expect("canary");
        std::fs::write(sub.join("page.png"), PNG_BYTES).expect("sidecar");

        let doc_path = sub.join("week.obd");
        let mut app = App {
            doc: doc_with_sheet(),
            path: Some(doc_path),
            ..Default::default()
        };

        // The ".." walk out of the document folder.
        app.doc.panels[0].background_image = Some("../secret.png".into());
        app.absorb_tracing_pictures(None);
        assert_eq!(
            app.doc.panels[0].background_image.as_deref(),
            Some("../secret.png"),
            "the reference stays a path; nothing was gathered"
        );
        assert!(app.doc.asset_bytes.is_empty());

        // The absolute path to the same canary.
        let absolute = root.join("secret.png").to_string_lossy().into_owned();
        app.doc.panels[0].background_image = Some(absolute.clone());
        app.absorb_tracing_pictures(None);
        assert_eq!(
            app.doc.panels[0].background_image.as_deref(),
            Some(absolute.as_str())
        );
        assert!(app.doc.asset_bytes.is_empty());

        // The legitimate sidecar in the document folder.
        app.doc.panels[0].background_image = Some("page.png".into());
        app.absorb_tracing_pictures(None);
        let hash = ob_model::sha256_hex(PNG_BYTES);
        assert_eq!(
            app.doc.panels[0].background_image.as_deref(),
            Some(hash.as_str())
        );
        assert_eq!(app.doc.asset_data(&hash), Some(PNG_BYTES));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_canvas_swap_round_trips_and_keeps_ids() {
        // R-3: swapping a sheet onto the canvas and back
        // restores the document exactly: the same parts in
        // the same places, the sheet's id unchanged, and
        // nothing duplicated in either direction.
        let mut doc = doc_with_sheet();
        let before = doc.clone();
        assert!(swap_insert_parts(&mut doc, "sheet-1"));
        // On the canvas: the document's parts ARE the
        // sheet's, and the sheet's entry holds the
        // bulletin's.
        assert_eq!(doc.panels[0].id, "sheet-1-p1");
        assert_eq!(doc.stories[0].id, "sheet-1-s1");
        assert_eq!(doc.inserts[0].panels[0].id, "host-panel");
        assert_eq!(doc.inserts[0].stories[0].id, "host-story");
        assert_eq!(doc.inserts[0].id, "sheet-1");
        // An edit made on the canvas lands in the sheet.
        doc.stories[0].paragraphs[0].set_text("Pray for the choir");
        assert!(swap_insert_parts(&mut doc, "sheet-1"));
        assert_eq!(
            doc.inserts[0].stories[0].paragraphs[0].text(),
            "Pray for the choir"
        );
        // And the bulletin is otherwise exactly itself.
        let mut restored = doc.clone();
        restored.inserts[0].stories[0].paragraphs[0].set_text("");
        assert_eq!(restored.panels, before.panels);
        assert_eq!(restored.stories, before.stories);
        assert_eq!(restored.inserts[0].panels, before.inserts[0].panels);
        // A sheet that is not there does not swap.
        assert!(!swap_insert_parts(&mut doc, "no-such-sheet"));
    }

    #[test]
    fn r3_opening_a_sheet_is_not_unsaved_work_but_editing_it_is() {
        // Round 3 (R3-7): the dirty comparison runs on
        // the canonical document, so the canvas swap
        // alone is not unsaved work, and a word typed
        // in the sheet is, on both sides of the swap.
        let mut app = App::default();
        app.doc = doc_with_sheet();
        app.saved_snapshot = app.canonical_snapshot();
        app.edit_insert_on_pages("sheet-1");
        assert!(app.insert_edit.is_some(), "the sheet is on the canvas");
        // The refresh update() performs every frame.
        app.frame_snapshot = app.canonical_snapshot();
        assert_eq!(
            app.frame_snapshot, app.saved_snapshot,
            "opening a sheet is not unsaved work"
        );
        app.doc.stories[0].paragraphs[0].set_text("Pray for the choir");
        app.frame_snapshot = app.canonical_snapshot();
        assert_ne!(
            app.frame_snapshot, app.saved_snapshot,
            "a word typed in the sheet is unsaved work"
        );
        app.leave_insert_editing();
        app.frame_snapshot = app.canonical_snapshot();
        assert_ne!(
            app.frame_snapshot, app.saved_snapshot,
            "and it stays unsaved work back in the bulletin"
        );
    }

    #[test]
    fn r4_rights_merge_fills_fields_and_never_overwrites() {
        // Round 4 (R4-5), replacing the round-3 test of
        // the winner-take-all predicate: the merge is
        // field-wise now. An empty record is filled from
        // a fuller one, as R3-11 asked; the new pins are
        // that a split-strength pair keeps both halves
        // and a noted value is never overwritten.
        let weak = ob_model::RightsRecord::default();
        let strong = ob_model::RightsRecord {
            license: ob_model::LicenseKind::CcBy("4.0".into()),
            credit_text: "Photo by Jane Doe, CC BY 4.0".into(),
            ..Default::default()
        };
        let merged = merge_rights_records(&weak, &strong);
        assert_eq!(merged.credit_text, "Photo by Jane Doe, CC BY 4.0");
        assert_eq!(merged.license, ob_model::LicenseKind::CcBy("4.0".into()));
        // Split strength: the bulletin holds the credit
        // and the title, the sheet holds only the
        // license. The merged record carries all three;
        // under the whole-record rule the license half
        // was lost and the picture printed unlicensed.
        let has_credit = ob_model::RightsRecord {
            title: "Harvest supper".into(),
            credit_text: "Photo by Jane Doe".into(),
            ..Default::default()
        };
        let has_license = ob_model::RightsRecord {
            license: ob_model::LicenseKind::CcBy("4.0".into()),
            ..Default::default()
        };
        let merged = merge_rights_records(&has_credit, &has_license);
        assert_eq!(merged.credit_text, "Photo by Jane Doe");
        assert_eq!(merged.title, "Harvest supper");
        assert_eq!(merged.license, ob_model::LicenseKind::CcBy("4.0".into()));
        // A value already noted is never overwritten,
        // and a known license already noted stands.
        let other = ob_model::RightsRecord {
            title: "A different title".into(),
            credit_text: "Someone else".into(),
            ..Default::default()
        };
        let merged = merge_rights_records(&has_credit, &other);
        assert_eq!(merged.title, "Harvest supper");
        assert_eq!(merged.credit_text, "Photo by Jane Doe");
        let cc0 = ob_model::RightsRecord {
            license: ob_model::LicenseKind::Cc0,
            ..Default::default()
        };
        assert_eq!(
            merge_rights_records(&cc0, &has_license).license,
            ob_model::LicenseKind::Cc0
        );
        // The children note merges as OR: a yes from
        // either side stands, a no stands over silence,
        // and silence stays silence.
        let yes = ob_model::RightsRecord {
            children_present: Some(true),
            ..Default::default()
        };
        let no = ob_model::RightsRecord {
            children_present: Some(false),
            ..Default::default()
        };
        assert_eq!(merge_rights_records(&yes, &no).children_present, Some(true));
        assert_eq!(merge_rights_records(&no, &yes).children_present, Some(true));
        assert_eq!(
            merge_rights_records(&yes, &weak).children_present,
            Some(true)
        );
        assert_eq!(
            merge_rights_records(&no, &weak).children_present,
            Some(false)
        );
        assert_eq!(merge_rights_records(&weak, &weak).children_present, None);
        // A merge that adds nothing changes nothing:
        // the attach counts only real fills.
        assert_eq!(merge_rights_records(&strong, &weak), strong);
        assert_eq!(merge_rights_records(&strong, &strong), strong);
    }

    #[test]
    fn r3_sheet_attach_fills_in_a_weak_picture_record() {
        // Round 3 (R3-11), reshaped by round 4 (R4-5):
        // the same picture bytes in both stores, and the
        // two records are strong in different halves. The
        // bulletin holds the credit line, the title, and
        // the children note; the sheet holds only the CC
        // BY license. The field-wise merge carries all of
        // it (the whole-record rule this replaces lost
        // whichever half the losing record held), the
        // composed credits show both halves, and the
        // note says a record was filled in. The bytes are
        // a real drawing, so the web door can carry the
        // picture and its credits.
        let bytes: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><path d="M1 5 L9 5 L5 9 Z"/></svg>"#;
        let hash = ob_model::sha256_hex(bytes);
        let in_bulletin = ob_model::RightsRecord {
            source: "the church camera".into(),
            title: "Harvest supper".into(),
            credit_text: "Photo by Jane Doe".into(),
            children_present: Some(true),
            ..Default::default()
        };
        let in_sheet = ob_model::RightsRecord {
            license: ob_model::LicenseKind::CcBy("4.0".into()),
            ..Default::default()
        };
        let asset = |rights: ob_model::RightsRecord| ob_model::Asset {
            sha256: hash.clone(),
            kind: ob_model::AssetKind::Picture,
            media_type: "image/svg+xml".into(),
            rights,
        };
        let mut ins = new_insert("sheet-9", "Photo sheet");
        let (mut frame, mut placement) =
            picture_frame(216_000, 144_000, ob_model::PictureFit::ShowWhole);
        placement.asset = hash.clone();
        frame.picture = Some(placement);
        ins.panels[0].frames.push(frame);
        let mut host = doc_with_sheet();
        host.assets.push(asset(in_sheet));
        host.asset_bytes.insert(hash.clone(), bytes.to_vec());
        let dir = std::env::temp_dir().join(format!("openbulletin-r3-obdi-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        let file = dir.join("photo-sheet.obdi");
        ob_format::save_insert(&ins, &host, &file).expect("the sheet saves");
        let mut app = App {
            doc: doc_with_sheet(),
            ..Default::default()
        };
        app.doc.assets.push(asset(in_bulletin));
        app.doc.asset_bytes.insert(hash.clone(), bytes.to_vec());
        app.add_insert_from_library(&file);
        let stored = app
            .doc
            .assets
            .iter()
            .find(|a| a.sha256 == hash)
            .expect("the picture stays");
        assert_eq!(stored.rights.credit_text, "Photo by Jane Doe");
        assert_eq!(
            stored.rights.license,
            ob_model::LicenseKind::CcBy("4.0".into())
        );
        assert_eq!(stored.rights.title, "Harvest supper");
        assert_eq!(stored.rights.children_present, Some(true));
        assert!(
            app.status
                .contains("1 picture record was filled in from the sheet."),
            "{}",
            app.status
        );
        // The composed credits carry both halves: the
        // credit the bulletin held and the license the
        // sheet brought, whose address the line shows.
        let (mut placed, mut placed_at) =
            picture_frame(216_000, 144_000, ob_model::PictureFit::ShowWhole);
        placed_at.asset = hash.clone();
        placed.picture = Some(placed_at);
        app.doc.panels[0].frames.push(placed);
        let (html_bytes, _report) =
            ob_extract::html_export::export_html(&app.doc).expect("the web page");
        let html = String::from_utf8(html_bytes).expect("utf8");
        assert!(html.contains("Photo by Jane Doe"), "{html}");
        assert!(
            html.contains("creativecommons.org/licenses/by/4.0"),
            "{html}"
        );
        // Attaching the same sheet again fills nothing:
        // the records already agree, and the note does
        // not claim a fill that did not happen.
        app.add_insert_from_library(&file);
        assert!(!app.status.contains("filled in"), "{}", app.status);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn r5_duplicate_records_in_a_file_fold_and_hold_the_picture_back() {
        // Round 5 (RT5-1): a hand-built bulletin file
        // whose document.json carries two records for
        // one picture: a weak one and the record that
        // notes the children and the CC BY license.
        // Readers take the first record they find, so
        // the weak one used to disarm the children note
        // and drop the license; the loader now folds
        // the pair into one record, in either order.
        // The folded record notes the children and no
        // permission (a CC BY license is not one), so
        // the web page holds the picture back.
        // FLAGGED, round 7 (R7-1): this test used to
        // pin the folded record's CC BY license
        // standing. The noting record carries no credit
        // line at all, so the folded record claimed a
        // CC BY license beside a children note with an
        // empty credit line — the composed state
        // RT7-1 graded BLOCKER. The license assertion
        // below therefore changed from CcBy("4.0") to
        // Unknown in R7-A; every other assertion (one
        // folded record, children note, title, creator,
        // source, the picture held back) stands
        // unchanged.
        let bytes: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><path d="M1 5 L9 5 L5 9 Z"/></svg>"#;
        let hash = ob_model::sha256_hex(bytes);
        let weak = ob_model::RightsRecord {
            source: "a plainer copy".into(),
            ..Default::default()
        };
        let noting = ob_model::RightsRecord {
            title: "Harvest supper".into(),
            creator: "Jane Doe".into(),
            license: ob_model::LicenseKind::CcBy("4.0".into()),
            children_present: Some(true),
            ..Default::default()
        };
        let asset = |rights: ob_model::RightsRecord| ob_model::Asset {
            sha256: hash.clone(),
            kind: ob_model::AssetKind::Picture,
            media_type: "image/svg+xml".into(),
            rights,
        };
        let dir = std::env::temp_dir().join(format!("openbulletin-r5-obd-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        for (tag, records) in [
            (
                "weak-first",
                vec![asset(weak.clone()), asset(noting.clone())],
            ),
            (
                "noting-first",
                vec![asset(noting.clone()), asset(weak.clone())],
            ),
        ] {
            let mut doc = doc_with_sheet();
            doc.assets.extend(records);
            doc.asset_bytes.insert(hash.clone(), bytes.to_vec());
            let (mut frame, mut placement) =
                picture_frame(216_000, 144_000, ob_model::PictureFit::ShowWhole);
            placement.asset = hash.clone();
            frame.picture = Some(placement);
            doc.panels[0].frames.push(frame);
            let file = dir.join(format!("dup-{tag}.obd"));
            ob_format::save(&doc, &file).expect("the file saves");
            let loaded = ob_format::load(&file).expect("the file loads");
            let stored: Vec<&ob_model::Asset> =
                loaded.assets.iter().filter(|a| a.sha256 == hash).collect();
            assert_eq!(stored.len(), 1, "{tag}: the pair folds into one record");
            let rights = &stored[0].rights;
            assert_eq!(rights.children_present, Some(true), "{tag}");
            assert_eq!(rights.license, ob_model::LicenseKind::Unknown, "{tag}");
            assert_eq!(rights.title, "Harvest supper", "{tag}");
            assert_eq!(rights.creator, "Jane Doe", "{tag}");
            assert_eq!(rights.source, "a plainer copy", "{tag}");
            let (_html_bytes, report) =
                ob_extract::html_export::export_html(&loaded).expect("the web page");
            assert_eq!(report.children_left_out, 1, "{tag}");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn r6_credit_first_records_in_a_file_fold_and_hold_the_picture_back() {
        // Round 6 (R6-1 / RT6-1): a hand-built bulletin
        // file whose document.json carries two records
        // for one picture: one holding only a credit line
        // and one noting only the children. Standing the
        // credit record first used to fold into a record
        // noting children WITH a permission, and the web
        // page published the picture. Both orders now
        // fold to a record noting the children and no
        // permission, and the page holds the picture back.
        let bytes: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><path d="M1 5 L9 5 L5 9 Z"/></svg>"#;
        let hash = ob_model::sha256_hex(bytes);
        let credit = ob_model::RightsRecord {
            credit_text: "Photo by Lee".into(),
            ..Default::default()
        };
        let children = ob_model::RightsRecord {
            children_present: Some(true),
            ..Default::default()
        };
        let asset = |rights: ob_model::RightsRecord| ob_model::Asset {
            sha256: hash.clone(),
            kind: ob_model::AssetKind::Picture,
            media_type: "image/svg+xml".into(),
            rights,
        };
        let dir = std::env::temp_dir().join(format!("openbulletin-r6-obd-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        for (tag, records) in [
            (
                "credit-first",
                vec![asset(credit.clone()), asset(children.clone())],
            ),
            (
                "children-first",
                vec![asset(children.clone()), asset(credit.clone())],
            ),
        ] {
            let mut doc = doc_with_sheet();
            doc.assets.extend(records);
            doc.asset_bytes.insert(hash.clone(), bytes.to_vec());
            let (mut frame, mut placement) =
                picture_frame(216_000, 144_000, ob_model::PictureFit::ShowWhole);
            placement.asset = hash.clone();
            frame.picture = Some(placement);
            doc.panels[0].frames.push(frame);
            let file = dir.join(format!("dup-{tag}.obd"));
            ob_format::save(&doc, &file).expect("the file saves");
            let loaded = ob_format::load(&file).expect("the file loads");
            let stored: Vec<&ob_model::Asset> =
                loaded.assets.iter().filter(|a| a.sha256 == hash).collect();
            assert_eq!(stored.len(), 1, "{tag}: the pair folds into one record");
            let rights = &stored[0].rights;
            assert_eq!(rights.children_present, Some(true), "{tag}");
            assert_eq!(rights.credit_text, "", "{tag}");
            let (_html_bytes, report) =
                ob_extract::html_export::export_html(&loaded).expect("the web page");
            assert_eq!(report.children_left_out, 1, "{tag}");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn r6_sheet_attach_with_credit_on_the_bulletin_side_holds_the_picture_back() {
        // Round 6 (R6-1 / RT6-1): no duplicates needed.
        // The bulletin holds a picture with only a credit
        // line; an ordinary sheet noting only children in
        // the same picture attaches, and the composed
        // record used to note a permission neither side
        // claimed for a children picture. The attach now
        // composes no permission, and the web page holds
        // the picture back.
        let bytes: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><path d="M1 5 L9 5 L5 9 Z"/></svg>"#;
        let hash = ob_model::sha256_hex(bytes);
        let in_bulletin = ob_model::RightsRecord {
            source: "the church camera".into(),
            credit_text: "Photo by Jane Doe".into(),
            ..Default::default()
        };
        let in_sheet = ob_model::RightsRecord {
            children_present: Some(true),
            ..Default::default()
        };
        let asset = |rights: ob_model::RightsRecord| ob_model::Asset {
            sha256: hash.clone(),
            kind: ob_model::AssetKind::Picture,
            media_type: "image/svg+xml".into(),
            rights,
        };
        let mut ins = new_insert("sheet-9", "Photo sheet");
        let (mut frame, mut placement) =
            picture_frame(216_000, 144_000, ob_model::PictureFit::ShowWhole);
        placement.asset = hash.clone();
        frame.picture = Some(placement);
        ins.panels[0].frames.push(frame);
        let mut host = doc_with_sheet();
        host.assets.push(asset(in_sheet));
        host.asset_bytes.insert(hash.clone(), bytes.to_vec());
        let dir = std::env::temp_dir().join(format!("openbulletin-r6-obdi-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        let file = dir.join("photo-sheet.obdi");
        ob_format::save_insert(&ins, &host, &file).expect("the sheet saves");
        let mut app = App {
            doc: doc_with_sheet(),
            ..Default::default()
        };
        app.doc.assets.push(asset(in_bulletin));
        app.doc.asset_bytes.insert(hash.clone(), bytes.to_vec());
        app.add_insert_from_library(&file);
        let stored = app
            .doc
            .assets
            .iter()
            .find(|a| a.sha256 == hash)
            .expect("the picture stays");
        assert_eq!(stored.rights.children_present, Some(true));
        assert_eq!(stored.rights.credit_text, "");
        assert_eq!(stored.rights.source, "the church camera");
        let (mut placed, mut placed_at) =
            picture_frame(216_000, 144_000, ob_model::PictureFit::ShowWhole);
        placed_at.asset = hash.clone();
        placed.picture = Some(placed_at);
        app.doc.panels[0].frames.push(placed);
        let (_html_bytes, report) =
            ob_extract::html_export::export_html(&app.doc).expect("the web page");
        assert_eq!(report.children_left_out, 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn r7_sheet_attach_in_the_mirror_order_drops_the_license() {
        // Round 7 (R7-1 / RT7-1): the sheet-attach path
        // in the mirror order. The bulletin holds the
        // picture as children-noted with a CC BY license
        // already standing; the sheet's record carries
        // only the credit line. The fence refuses the
        // credit in the attach merge, and the composed
        // record must not go on claiming the license
        // whose attribution was refused in the same
        // merge: the license falls, the children note
        // and the bulletin's other fields stand.
        let bytes: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><path d="M1 5 L9 5 L5 9 Z"/></svg>"#;
        let hash = ob_model::sha256_hex(bytes);
        let in_bulletin = ob_model::RightsRecord {
            source: "the church camera".into(),
            license: ob_model::LicenseKind::CcBy("4.0".into()),
            children_present: Some(true),
            ..Default::default()
        };
        let in_sheet = ob_model::RightsRecord {
            credit_text: "Photo by Alice, CC BY 4.0".into(),
            ..Default::default()
        };
        let asset = |rights: ob_model::RightsRecord| ob_model::Asset {
            sha256: hash.clone(),
            kind: ob_model::AssetKind::Picture,
            media_type: "image/svg+xml".into(),
            rights,
        };
        let mut ins = new_insert("sheet-10", "Photo sheet");
        let (mut frame, mut placement) =
            picture_frame(216_000, 144_000, ob_model::PictureFit::ShowWhole);
        placement.asset = hash.clone();
        frame.picture = Some(placement);
        ins.panels[0].frames.push(frame);
        let mut host = doc_with_sheet();
        host.assets.push(asset(in_sheet));
        host.asset_bytes.insert(hash.clone(), bytes.to_vec());
        let dir = std::env::temp_dir().join(format!("openbulletin-r7-obdi-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        let file = dir.join("photo-sheet.obdi");
        ob_format::save_insert(&ins, &host, &file).expect("the sheet saves");
        let mut app = App {
            doc: doc_with_sheet(),
            ..Default::default()
        };
        app.doc.assets.push(asset(in_bulletin));
        app.doc.asset_bytes.insert(hash.clone(), bytes.to_vec());
        app.add_insert_from_library(&file);
        let stored = app
            .doc
            .assets
            .iter()
            .find(|a| a.sha256 == hash)
            .expect("the picture stays");
        assert_eq!(stored.rights.children_present, Some(true));
        assert_eq!(stored.rights.credit_text, "");
        assert_eq!(stored.rights.license, ob_model::LicenseKind::Unknown);
        assert_eq!(stored.rights.source, "the church camera");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn insert_findings_name_empty_and_out_of_date_sheets() {
        let mut doc = doc_with_sheet();
        // The fresh sheet has one empty paragraph: no words.
        let notes = insert_findings(&doc);
        assert_eq!(notes.len(), 1);
        assert_eq!(
            notes[0].text,
            "Check: the loose sheet 'Prayer list' has no words yet."
        );
        assert!(notes[0].inserts, "the fix opens the Loose sheets window");
        // Words written, and a keep-through date still to
        // come: silence.
        doc.inserts[0].stories[0].paragraphs[0].set_text("Pray for us");
        doc.inserts[0].life = ob_model::InsertLife::Dated;
        doc.inserts[0].keep_through = Some(ob_model::SimpleDate {
            year: 2026,
            month: 11,
            day: 8,
        });
        assert!(insert_findings(&doc).is_empty());
        // The date passes (the bulletin is for 2026-10-18):
        // the finding names the sheet and the date.
        doc.inserts[0].keep_through = Some(ob_model::SimpleDate {
            year: 2026,
            month: 10,
            day: 11,
        });
        let notes = insert_findings(&doc);
        assert_eq!(notes.len(), 1);
        assert_eq!(
            notes[0].text,
            "Check: the loose sheet 'Prayer list' was kept through October 11, 2026, which has passed."
        );
    }

    #[test]
    fn insert_words_say_the_plain_things() {
        let mut sheet = ob_model::InsertSheet::default();
        assert_eq!(insert_copies_words(&sheet), "as many as the bulletin");
        assert_eq!(insert_paper_words(&sheet), "plain paper");
        assert_eq!(insert_size_words(&sheet), "Half letter");
        sheet.copies = 1;
        assert_eq!(insert_copies_words(&sheet), "1 copy");
        sheet.copies = 60;
        assert_eq!(insert_copies_words(&sheet), "60 copies");
        sheet.paper_note = "  blue paper ".into();
        assert_eq!(insert_paper_words(&sheet), "blue paper");
        let mut ins = new_insert("sheet-9", "Card");
        assert_eq!(insert_life_words(&ins), "This week only");
        ins.life = ob_model::InsertLife::Standing;
        assert_eq!(insert_life_words(&ins), "Standing: rides every week");
        ins.life = ob_model::InsertLife::Dated;
        assert_eq!(insert_life_words(&ins), "Until a date (no date set yet)");
        ins.keep_through = Some(ob_model::SimpleDate {
            year: 2026,
            month: 11,
            day: 8,
        });
        assert_eq!(insert_life_words(&ins), "Until November 8, 2026");
    }

    #[test]
    fn a_wanted_font_warns_on_check_until_the_font_arrives() {
        // The install-later promise, at the Check step
        // (docs/FONTS.md): the finding names the font the
        // bulletin was made in and the stand-in in use
        // TODAY; the day the real font is added to the
        // library, the finding is gone, with no edit to
        // the document.
        // The bulletin's own style still names the font
        // (the shape a real import leaves); the record
        // alone does not raise the finding (R3-16, see
        // the restyle test below).
        let gill_style = ob_model::ParagraphStyle {
            id: "body".into(),
            name: "Body".into(),
            font_size: 11_000,
            bold: false,
            italic: false,
            align: ob_model::Align::Left,
            space_after: 4_000,
            color: ob_model::Rgb::BLACK,
            font_family: "Gill Sans MT".into(),
            font_sha256: None,
        };
        let doc = Document {
            format_version: ob_model::FORMAT_VERSION,
            title: "T".into(),
            profile: ChurchProfile::default(),
            fields: Default::default(),
            styles: vec![gill_style],
            stories: vec![],
            panels: vec![],
            sheet: ob_model::SheetSpec {
                preset: "letter-portrait".into(),
                size: ob_model::Size::new(612_000, 792_000),
            },
            no_announcements: false,
            assets: vec![],
            asset_bytes: Default::default(),
            occasion_time: None,
            wanted_fonts: vec![ob_model::WantedFont {
                name: "Gill Sans MT".into(),
                substituted_by: "Cabin".into(),
            }],
            participants: Default::default(),
            inserts: Default::default(),
            applied_contributions: Vec::new(),
        };
        // Nothing in the library: today's stand-in is the
        // house default, and the words say so (the record
        // still remembers Cabin was the import-time pick).
        let notes = wanted_font_notes(&doc, &[]);
        assert_eq!(notes.len(), 1);
        assert!(
            notes[0].text.contains("made in Gill Sans MT"),
            "{}",
            notes[0].text
        );
        assert!(
            notes[0].text.contains("shown in Liberation Sans"),
            "{}",
            notes[0].text
        );
        assert!(notes[0].fonts, "the fix button opens Fonts");
        // Cabin added: the stand-in named is Cabin now.
        let notes = wanted_font_notes(&doc, &["Cabin".to_string()]);
        assert_eq!(notes.len(), 1);
        assert!(
            notes[0].text.contains("shown in Cabin"),
            "{}",
            notes[0].text
        );
        // The real font added: no finding at all.
        let notes = wanted_font_notes(&doc, &["Gill Sans MT".to_string()]);
        assert!(notes.is_empty(), "{notes:?}");
    }

    #[test]
    fn a_wanted_font_finding_reflects_the_worst_style_that_uses_it() {
        // Round 5 (R5-D, review M3): two source families
        // share the record's display name, "Gill Sans
        // MT": the style set in "GillSansMT" and the one
        // set in "Gill Sans MT". The library holds the
        // first spelling exactly, plus Cabin, the
        // lookalike. Resolving only the FIRST matching
        // style certified the bulletin on the Exact one
        // while the other style printed in Cabin. The
        // oracle now examines every matching style, and
        // the finding is the worse one's, in either
        // style order.
        let style_in = |id: &str, family: &str| ob_model::ParagraphStyle {
            id: id.into(),
            name: "Body".into(),
            font_size: 11_000,
            bold: false,
            italic: false,
            align: ob_model::Align::Left,
            space_after: 4_000,
            color: ob_model::Rgb::BLACK,
            font_family: family.into(),
            font_sha256: None,
        };
        let doc_with_styles = |styles: Vec<ob_model::ParagraphStyle>| Document {
            format_version: ob_model::FORMAT_VERSION,
            title: "T".into(),
            profile: ChurchProfile::default(),
            fields: Default::default(),
            styles,
            stories: vec![],
            panels: vec![],
            sheet: ob_model::SheetSpec {
                preset: "letter-portrait".into(),
                size: ob_model::Size::new(612_000, 792_000),
            },
            no_announcements: false,
            assets: vec![],
            asset_bytes: Default::default(),
            occasion_time: None,
            wanted_fonts: vec![ob_model::WantedFont {
                name: "Gill Sans MT".into(),
                substituted_by: "Cabin".into(),
            }],
            participants: Default::default(),
            inserts: Default::default(),
            applied_contributions: Vec::new(),
        };
        let exact_style = style_in("exact", "GillSansMT");
        let lookalike_style = style_in("lookalike", "Gill Sans MT");
        let available = vec!["GillSansMT".to_string(), "Cabin".to_string()];
        for styles in [
            vec![exact_style.clone(), lookalike_style.clone()],
            vec![lookalike_style, exact_style],
        ] {
            let doc = doc_with_styles(styles);
            let notes = wanted_font_notes(&doc, &available);
            assert_eq!(notes.len(), 1, "{notes:?}");
            assert!(
                notes[0].text.contains("made in Gill Sans MT"),
                "{}",
                notes[0].text
            );
            assert!(
                notes[0].text.contains("shown in Cabin"),
                "the worse style's stand-in is named: {}",
                notes[0].text
            );
        }
    }

    #[test]
    fn a_wanted_font_note_names_the_worst_stand_in_and_scopes_its_claim() {
        // Round 6 (R6-9): two styles share the record's
        // name, "Gill Sans MT". The library holds the
        // first spelling exactly and nothing else: that
        // style prints in the real font, the other falls
        // to the house default. The note names the worst
        // style's stand-in (Liberation Sans), and its
        // claim is scoped — SOME of its words — because
        // the whole-bulletin wording would be false for
        // the style that prints in the real font.
        let style_in = |id: &str, family: &str| ob_model::ParagraphStyle {
            id: id.into(),
            name: "Body".into(),
            font_size: 11_000,
            bold: false,
            italic: false,
            align: ob_model::Align::Left,
            space_after: 4_000,
            color: ob_model::Rgb::BLACK,
            font_family: family.into(),
            font_sha256: None,
        };
        let doc = Document {
            format_version: ob_model::FORMAT_VERSION,
            title: "T".into(),
            profile: ChurchProfile::default(),
            fields: Default::default(),
            styles: vec![
                style_in("exact", "GillSansMT"),
                style_in("fallback", "Gill Sans MT"),
            ],
            stories: vec![],
            panels: vec![],
            sheet: ob_model::SheetSpec {
                preset: "letter-portrait".into(),
                size: ob_model::Size::new(612_000, 792_000),
            },
            no_announcements: false,
            assets: vec![],
            asset_bytes: Default::default(),
            occasion_time: None,
            wanted_fonts: vec![ob_model::WantedFont {
                name: "Gill Sans MT".into(),
                substituted_by: "Cabin".into(),
            }],
            participants: Default::default(),
            inserts: Default::default(),
            applied_contributions: Vec::new(),
        };
        let available = vec!["GillSansMT".to_string()];
        let notes = wanted_font_notes(&doc, &available);
        assert_eq!(notes.len(), 1, "{notes:?}");
        assert!(
            notes[0]
                .text
                .contains("some of its words are shown in Liberation Sans"),
            "the worst style's stand-in, scoped: {}",
            notes[0].text
        );
        // One style alone keeps the unscoped words.
        let mut single = doc.clone();
        single.styles = vec![style_in("fallback", "Gill Sans MT")];
        let notes = wanted_font_notes(&single, &available);
        assert_eq!(notes.len(), 1, "{notes:?}");
        assert!(
            notes[0]
                .text
                .contains("so its words are shown in Liberation Sans for now"),
            "the single-style wording is unchanged: {}",
            notes[0].text
        );
    }

    #[test]
    fn a_wanted_font_goes_silent_once_nothing_uses_it() {
        // R3-16: the record stays on the document, but
        // once every style has been restyled away from
        // the wanted family, nothing prints in it and
        // Check stops asking about it. A loose sheet's
        // style still using it keeps the finding alive.
        let style_in = |family: &str| ob_model::ParagraphStyle {
            id: "body".into(),
            name: "Body".into(),
            font_size: 11_000,
            bold: false,
            italic: false,
            align: ob_model::Align::Left,
            space_after: 4_000,
            color: ob_model::Rgb::BLACK,
            font_family: family.into(),
            font_sha256: None,
        };
        let base_doc = Document {
            format_version: ob_model::FORMAT_VERSION,
            title: "T".into(),
            profile: ChurchProfile::default(),
            fields: Default::default(),
            styles: vec![style_in("Liberation Sans")],
            stories: vec![],
            panels: vec![],
            sheet: ob_model::SheetSpec {
                preset: "letter-portrait".into(),
                size: ob_model::Size::new(612_000, 792_000),
            },
            no_announcements: false,
            assets: vec![],
            asset_bytes: Default::default(),
            occasion_time: None,
            wanted_fonts: vec![ob_model::WantedFont {
                name: "Gill Sans MT".into(),
                substituted_by: "Cabin".into(),
            }],
            participants: Default::default(),
            inserts: Default::default(),
            applied_contributions: Vec::new(),
        };
        // Restyled away in the bulletin itself: silent.
        let notes = wanted_font_notes(&base_doc, &[]);
        assert!(notes.is_empty(), "{notes:?}");
        // A loose sheet still set in it: the finding stays.
        let mut with_sheet = base_doc.clone();
        let mut sheet = new_insert("sheet-1", "Prayer list");
        sheet.styles = vec![style_in("Gill Sans MT")];
        with_sheet.inserts = vec![sheet];
        let notes = wanted_font_notes(&with_sheet, &[]);
        assert_eq!(notes.len(), 1, "{notes:?}");
    }

    #[test]
    fn pdf_purpose_defaults_to_printing_and_remembers() {
        // A settings file from before O1 has no purpose word:
        // the chooser's default is For printing.
        let st: Settings = serde_json::from_str("{}").expect("old settings load");
        assert_eq!(st.pdf_purpose(), ob_pdf::ExportPurpose::ForPrinting);
        // A remembered choice comes back as itself, and the
        // slug round-trips through the settings file.
        let st = Settings {
            pdf_purpose: ob_pdf::ExportPurpose::ForEmailOrWeb.slug().to_string(),
            ..Default::default()
        };
        assert_eq!(st.pdf_purpose(), ob_pdf::ExportPurpose::ForEmailOrWeb);
        let json = serde_json::to_string(&st).expect("settings save");
        let back: Settings = serde_json::from_str(&json).expect("settings reload");
        assert_eq!(back.pdf_purpose(), ob_pdf::ExportPurpose::ForEmailOrWeb);
        // A word we do not know falls back to the default,
        // never to a wrong purpose.
        let st = Settings {
            pdf_purpose: "carrier-pigeon".to_string(),
            ..Default::default()
        };
        assert_eq!(st.pdf_purpose(), ob_pdf::ExportPurpose::ForPrinting);
    }

    #[test]
    fn file_sizes_read_in_plain_words() {
        assert_eq!(human_file_size(512), "512 bytes");
        assert_eq!(human_file_size(1024), "1 KB");
        assert_eq!(human_file_size(212 * 1024 + 300), "212 KB");
        assert_eq!(human_file_size(1024 * 1024), "1.0 MB");
        assert_eq!(human_file_size(7 * 1024 * 1024 + 512 * 1024), "7.5 MB");
    }

    #[test]
    fn keymap_presets_set_defaults_and_any_change_is_custom() {
        // Fresh settings: our defaults, named as such.
        let mut st = Settings::default();
        assert_eq!(keymap_status(&st), "OpenBulletin defaults");
        // Choosing Publisher only sets defaults.
        st.keymap_preset = "publisher".into();
        assert_eq!(keymap_status(&st), "Publisher defaults");
        assert_eq!(effective_keymap(&st)["fonts"], "Ctrl+Shift+F");
        // One key moves away: Custom (Jacob's rule).
        st.keymap.insert("fonts".into(), "Alt+F".into());
        assert_eq!(keymap_status(&st), "Custom");
        // Moving it back restores the preset's name.
        st.keymap.insert("fonts".into(), "Ctrl+Shift+F".into());
        assert_eq!(keymap_status(&st), "Publisher defaults");
        // InDesign's redo is the documented Ctrl+Shift+Z.
        assert_eq!(preset_defaults("indesign")["redo"], "Ctrl+Shift+Z");
        assert_eq!(preset_defaults("publisher")["redo"], "Ctrl+Y");
    }

    #[test]
    fn settings_salvage_keeps_the_good_sections() {
        // One broken section (printers as a string) must not
        // take the rest of the memory down with it (R2-4).
        let text = r#"{
            "printers": "not a map",
            "keymap_preset": "publisher",
            "keymap": {"fonts": "Alt+F"},
            "last_bulletin": "/home/office/week.obd",
            "doc_states": [["/home/office/week.obd", {"print_fingerprint": "abc", "checked_fingerprint": null}]],
            "last_step": 3,
            "pdf_purpose": "web",
            "ui_scale": "Large",
            "high_contrast": true
        }"#;
        let (st, read) = Settings::load_from_str(text);
        assert_eq!(read, SettingsRead::PartlySalvaged);
        assert!(st.printers.is_empty(), "the broken section reset");
        assert_eq!(st.keymap_preset, "publisher");
        assert_eq!(effective_keymap(&st)["fonts"], "Alt+F");
        assert_eq!(st.last_bulletin.as_deref(), Some("/home/office/week.obd"));
        assert_eq!(st.doc_states.len(), 1);
        assert_eq!(
            st.doc_state("/home/office/week.obd")
                .and_then(|d| d.print_fingerprint.as_deref()),
            Some("abc")
        );
        assert_eq!(st.last_step, 3);
        assert_eq!(st.pdf_purpose(), ob_pdf::ExportPurpose::ForEmailOrWeb);
        assert_eq!(st.ui_scale, UiScale::Large);
        assert!(st.high_contrast);
    }

    #[test]
    fn settings_that_are_not_settings_are_set_aside() {
        // Not JSON at all, and JSON that is not an object:
        // both read as defaults plus the set-aside word.
        for text in ["this is not json", "[1, 2, 3]", "\"just a string\""] {
            let (st, read) = Settings::load_from_str(text);
            assert_eq!(read, SettingsRead::SetAside, "{text}");
            assert_eq!(st.last_step, 0);
            assert!(st.printers.is_empty());
        }
        // An empty object is simply a fresh memory.
        let (st, read) = Settings::load_from_str("{}");
        assert_eq!(read, SettingsRead::Clean);
        assert_eq!(st.pdf_purpose(), ob_pdf::ExportPurpose::ForPrinting);
    }

    #[test]
    fn keymap_data_is_validated_on_load() {
        // A press claimed by two actions in the file fires
        // exactly one action after load (R2-5): the later
        // claim (in the canonical action order) takes it, and
        // the earlier holder gets the later action's old
        // press, the capture swap.
        let (st, _) =
            Settings::load_from_str(r#"{"keymap": {"save": "Ctrl+Q", "open": "Ctrl+Q"}}"#);
        let eff = effective_keymap(&st);
        assert_eq!(eff.values().filter(|v| *v == "Ctrl+Q").count(), 1);
        assert_eq!(eff["open"], "Ctrl+Q");
        assert_eq!(eff["save"], "Ctrl+O", "the swap, as capture does it");

        // Garbage entries are dropped: an unknown action
        // cannot poison the set's name, and a press that does
        // not read leaves the action on its preset key.
        let (st, _) = Settings::load_from_str(
            r#"{"keymap_preset": "publisher",
                "keymap": {"nosuch": "Ctrl+Q", "save": "not a key press",
                           "fonts": "Ctrl+Shift+F"}}"#,
        );
        assert_eq!(keymap_status(&st), "Publisher defaults");
        assert_eq!(effective_keymap(&st)["save"], "Ctrl+S");

        // A file equal to a preset loads as that preset,
        // whether it names the preset or restates its keys.
        let (st, _) = Settings::load_from_str(r#"{"keymap_preset": "indesign"}"#);
        assert_eq!(keymap_status(&st), "InDesign defaults");
        let (st, _) =
            Settings::load_from_str(r#"{"keymap": {"redo": "Ctrl+Y", "pages": "Alt+P"}}"#);
        assert_eq!(keymap_status(&st), "OpenBulletin defaults");
        assert!(st.keymap.is_empty(), "restated defaults normalize away");
    }

    #[test]
    fn keymap_bindings_round_trip() {
        for preset in ["openbulletin", "publisher", "indesign"] {
            for (action, text) in preset_defaults(preset) {
                let parsed = parse_binding(&text)
                    .unwrap_or_else(|| panic!("{preset}/{action}: {text} parses"));
                assert_eq!(format_binding(&parsed), text, "{preset}/{action}");
            }
        }
        assert!(parse_binding("Ctrl+Bogus").is_none());
        assert!(parse_binding("Hyper+Z").is_none());
    }

    #[test]
    fn preview_face_key_picks_the_face_a_line_prints_in() {
        let row = |key: &str, family: &str, full_name: &str| FontRow {
            key: key.into(),
            family: family.into(),
            full_name: full_name.into(),
            rights_words: String::new(),
        };
        let rows = vec![
            row("lib-reg", "Liberation Sans", "Liberation Sans"),
            row("lib-bold", "Liberation Sans", "Liberation Sans Bold"),
            row("lib-italic", "Liberation Sans", "Liberation Sans Italic"),
            row("lib-bi", "Liberation Sans", "Liberation Sans Bold Italic"),
            row("lib-serif", "Liberation Serif", "Liberation Serif"),
        ];
        assert_eq!(
            preview_face_key(&rows, "Liberation Sans", false, false).as_deref(),
            Some("lib-reg")
        );
        assert_eq!(
            preview_face_key(&rows, "Liberation Sans", true, false).as_deref(),
            Some("lib-bold")
        );
        assert_eq!(
            preview_face_key(&rows, "Liberation Sans", false, true).as_deref(),
            Some("lib-italic")
        );
        assert_eq!(
            preview_face_key(&rows, "Liberation Sans", true, true).as_deref(),
            Some("lib-bi")
        );
        // A family that ships only a regular face still draws
        // in that family, not in the interface font.
        assert_eq!(
            preview_face_key(&rows, "Liberation Serif", true, false).as_deref(),
            Some("lib-serif")
        );
        // A family the library does not hold paints in the
        // resolution chain's stand-in (docs/FONTS.md): the
        // house default's regular face here, so the preview
        // shows the font the paper will get. A face style
        // the stand-in lacks falls to its regular face, the
        // same rule as any one-face family.
        assert_eq!(
            preview_face_key(&rows, "No Such Family", false, false).as_deref(),
            Some("lib-reg")
        );
        assert_eq!(
            preview_face_key(&rows, "Gill Sans MT", true, false).as_deref(),
            Some("lib-bold"),
            "Gill Sans MT's stand-in here is the house default"
        );
    }

    #[test]
    fn library_font_setup_binds_every_row_it_names() {
        // NOTE 9's registration rule: the definitions and
        // the rows are built together, and every row's key
        // must be a family the definitions bind, or a painter
        // naming that row panics.
        let dir = std::env::temp_dir().join(format!("ob-font-setup-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let lib = ob_fonts::FontLibrary::open(dir.clone()).expect("library");
        for (name, bytes) in ob_text::shaping::bundled_font_files() {
            lib.add_bytes(bytes, name).expect("seed bundled face");
        }
        let (defs, rows) = library_font_setup(&lib);
        assert!(!rows.is_empty(), "the bundled faces are rows");
        let bound = bound_name_families(&defs);
        for row in &rows {
            assert!(
                bound.contains(&row.key),
                "row {} names a family the definitions do not bind",
                row.key
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_unbound_family_falls_back_for_one_frame() {
        // NOTE 9's startup frame, at the level the fix
        // lands: the rows already name library faces, but
        // the definitions active this frame are still egui's
        // defaults, which bind none of them. A bold line
        // must fall back to the interface font, never name
        // the unbound family; once the family is bound, the
        // same line draws in its real face.
        let row = |key: &str, full_name: &str| FontRow {
            key: key.into(),
            family: "Liberation Sans".into(),
            full_name: full_name.into(),
            rights_words: String::new(),
        };
        let rows = vec![
            row("lib-reg", "Liberation Sans"),
            row("lib-bold", "Liberation Sans Bold"),
        ];
        let line = PreviewLine {
            text: "Welcome".into(),
            size: 12.0,
            x: 0.0,
            y: 0.0,
            w: 100.0,
            align: ob_model::Align::Left,
            font_family: "Liberation Sans".into(),
            bold: true,
            italic: false,
            segs: Vec::new(),
            spoken_tag: false,
            web_tag: false,
        };
        let unbound = bound_name_families(&egui::FontDefinitions::default());
        assert!(unbound.is_empty(), "egui defaults bind no lib families");
        let fid = line_font_id(&rows, &unbound, &line, 12.0);
        assert_eq!(fid.family, egui::FontFamily::Proportional);
        let bound: std::collections::HashSet<String> =
            ["lib-reg".to_string(), "lib-bold".to_string()]
                .into_iter()
                .collect();
        let fid = line_font_id(&rows, &bound, &line, 12.0);
        assert_eq!(
            fid.family,
            egui::FontFamily::Name("lib-bold".into()),
            "bound, the line draws in its real face"
        );
    }

    #[test]
    fn checks_flag_missing_fonts_and_edge_boxes() {
        let mut app = App::default();
        app.doc.styles[0].font_family = "NoSuchFont Display".into();
        app.run_checks();
        assert!(
            app.notes
                .iter()
                .any(|n| n.text.contains("NoSuchFont Display")),
            "missing font is named: {:?}",
            app.notes
        );
        // A box hugging the edge is called out on a fold preset.
        let mut app2 = App::default();
        app2.doc.panels[0].frames[0].rect.x = 0;
        app2.run_checks();
        assert!(
            app2.notes
                .iter()
                .any(|n| n.text.contains("close to the page edge")),
            "edge box is named: {:?}",
            app2.notes
        );
        // The untouched template earns neither note.
        let mut app3 = App::default();
        app3.run_checks();
        assert!(!app3
            .notes
            .iter()
            .any(|n| n.text.contains("close to the page edge")));
        assert!(!app3
            .notes
            .iter()
            .any(|n| n.text.contains("not in") && n.text.contains("font")));
    }

    #[test]
    fn last_bulletin_door_needs_a_remembered_file_that_is_still_there() {
        // Nothing remembered: no door.
        assert!(last_bulletin_door(None, true).is_none());
        assert!(last_bulletin_door(Some("  "), true).is_none());
        // Remembered but gone from disk: no door, no error.
        assert!(last_bulletin_door(Some("/office/week.obd"), false).is_none());
        // Remembered and still there: the door, named by the
        // file's own name.
        let (path, name) =
            last_bulletin_door(Some("/office/bulletins/week.obd"), true).expect("door");
        assert_eq!(path, std::path::PathBuf::from("/office/bulletins/week.obd"));
        assert_eq!(name, "week.obd");
    }

    #[test]
    fn settings_carry_the_last_bulletin_and_old_files_load_without_it() {
        // Settings written before the field existed load
        // with no remembered bulletin.
        let old: Settings = serde_json::from_str("{}").expect("old settings load");
        assert_eq!(old.last_bulletin, None);
        // And the field round-trips.
        let st = Settings {
            last_bulletin: Some("/office/week.obd".into()),
            ..Settings::default()
        };
        let text = serde_json::to_string(&st).expect("serialize");
        let back: Settings = serde_json::from_str(&text).expect("reload");
        assert_eq!(back.last_bulletin.as_deref(), Some("/office/week.obd"));
    }

    #[test]
    fn check_notes_point_at_the_box_they_mean() {
        // The frame lookup finds a story's first frame.
        let app = App::default();
        let story_id = app.doc.panels[0].frames[0]
            .story_id
            .clone()
            .expect("template frame has a story");
        assert_eq!(
            frame_target_for_story(&app.doc, &story_id),
            Some((0, Some(0)))
        );
        assert_eq!(frame_target_for_story(&app.doc, "no-such-story"), None);
        // An edge finding carries its box as a target, so
        // its Fix this button can land there.
        let mut app2 = App::default();
        app2.doc.panels[0].frames[0].rect.x = 0;
        app2.run_checks();
        let edge = app2
            .notes
            .iter()
            .find(|n| n.text.contains("close to the page edge"))
            .expect("edge note");
        assert_eq!(edge.target, Some((0, Some(0))));
    }

    #[test]
    fn escape_records_changed_words_and_restore_brings_them_back() {
        // J3: Escape still cancels (the page keeps its old
        // words), but the typed words are recorded, and the
        // restore reopens the same box with them as the buffer.
        let mut app = App::default();
        app.enter_edit(0, 0);
        assert!(
            app.editing.is_some(),
            "template frame opens an edit session"
        );
        app.edit_text = "Words typed and then escaped away.".into();
        app.cancel_edit();
        assert!(app.editing.is_none());
        let rec = app
            .discarded_edit
            .clone()
            .expect("a changed session's words are recorded");
        assert_eq!(rec.1, 0);
        assert_eq!(rec.2, 0);
        assert_eq!(rec.3, "Words typed and then escaped away.");
        app.restore_discarded_edit();
        assert_eq!(app.editing, Some((0, 0)));
        assert_eq!(app.edit_text, "Words typed and then escaped away.");
        assert!(app.discarded_edit.is_none(), "the record is spent");
    }

    #[test]
    fn escape_without_changes_records_nothing() {
        // J3: a clean cancel has nothing to forgive.
        let mut app = App::default();
        app.enter_edit(0, 0);
        app.cancel_edit();
        assert!(app.discarded_edit.is_none());
    }

    #[test]
    fn canvas_toggle_bold_writes_runs_back_on_commit() {
        // G1b: select a word on the canvas, bold it, commit:
        // the story paragraph carries a bold run for exactly
        // that word, and the words themselves are untouched.
        let mut app = App::default();
        app.enter_edit(0, 0);
        app.edit_text = "Sing hymn 432".into();
        app.edit_styles = vec!["body".into()];
        app.edit_runs = vec![vec![ob_model::Run::plain("Sing hymn 432")]];
        app.edit_layout = None;
        app.refresh_editor_layout();
        assert!(app.edit_layout.is_some(), "the shaped layout builds");
        // "hymn" spans display 5..9 (no field words here, so
        // display space is the buffer's own space).
        app.edit_anchor = 5;
        app.edit_caret = 9;
        app.editor_toggle_flag(runsedit::Flag::Bold);
        assert_eq!(app.edit_runs[0].len(), 3);
        assert_eq!(app.edit_runs[0][1].text, "hymn");
        assert_eq!(app.edit_runs[0][1].bold, Some(true));
        assert!(app.editor_flag_states().0, "the toolbar state reads bold");
        // The word-processor rule: toggling again unbolds.
        app.editor_toggle_flag(runsedit::Flag::Bold);
        assert!(!app.editor_flag_states().0);
        app.editor_toggle_flag(runsedit::Flag::Bold);
        app.commit_edit();
        let sid = app.doc.panels[0].frames[0]
            .story_id
            .clone()
            .expect("the frame has a story");
        let story = app.doc.story(&sid).expect("story");
        assert_eq!(story.paragraphs[0].text(), "Sing hymn 432");
        assert!(
            story.paragraphs.iter().any(|p| p
                .runs
                .iter()
                .any(|r| r.text == "hymn" && r.bold == Some(true))),
            "a bold hymn run reached the document: {:?}",
            story.paragraphs[0].runs
        );
    }

    #[test]
    fn md_file_routes_to_a_notes_review() {
        // G1b: a .md file through the one open door becomes
        // a draft waiting in the standard review, labeled
        // as notes or web text.
        let mut app = App::default();
        let dir = std::env::temp_dir().join(format!("ob-md-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("dir");
        let path = dir.join("announcements.md");
        std::fs::write(&path, "# Potluck\n\nBring a **dish** to share.\n").expect("write");
        app.route_old_bulletin(path);
        let review = app.import_review.expect("a review waits");
        assert_eq!(review.items.len(), 1);
        assert!(review.items[0].label.contains("Notes from"));
        assert_eq!(review.items[0].badge, ob_extract::draft::Badge::CheckThis);
        assert!(
            review
                .note
                .as_deref()
                .unwrap_or("")
                .contains("Notes or web text"),
            "the kind label is stated: {:?}",
            review.note
        );
        let doc = &review.doc;
        assert_eq!(doc.stories[0].paragraphs[0].text(), "Potluck");
        assert!(
            doc.stories[0].paragraphs[1]
                .runs
                .iter()
                .any(|r| r.text == "dish" && r.bold == Some(true)),
            "the markdown bold arrived as a run"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn html_file_routes_to_a_web_page_review() {
        // H2: a newsletter export through the one open
        // door becomes a draft waiting in the standard
        // review, labeled as a web page or newsletter.
        let mut app = App::default();
        let dir = std::env::temp_dir().join(format!("ob-html-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("dir");
        let path = dir.join("newsletter.html");
        std::fs::write(
            &path,
            "<!DOCTYPE html><html><body><h2>News</h2>\
             <p>Join us for the <strong>harvest supper</strong> on Saturday.</p>\
             </body></html>",
        )
        .expect("write");
        app.route_old_bulletin(path);
        let review = app.import_review.expect("a review waits");
        assert!(
            review
                .note
                .as_deref()
                .unwrap_or("")
                .contains("Web page or newsletter"),
            "the kind label is stated: {:?}",
            review.note
        );
        let doc = &review.doc;
        assert_eq!(doc.stories[0].paragraphs[0].text(), "News");
        assert!(
            doc.stories[0].paragraphs[1]
                .runs
                .iter()
                .any(|r| r.text == "harvest supper" && r.bold == Some(true)),
            "the page's bold arrived as a run"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn html_renamed_file_routes_by_its_head() {
        // H2: a newsletter export saved under a name that
        // does not say .html still routes by its first
        // bytes, through the sniff.
        let mut app = App::default();
        let dir = std::env::temp_dir().join(format!("ob-html-sniff-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("dir");
        let path = dir.join("newsletter.txt");
        std::fs::write(
            &path,
            "<html><body><p>Words from a renamed export.</p></body></html>",
        )
        .expect("write");
        app.route_old_bulletin(path);
        let review = app.import_review.expect("a review waits");
        assert!(
            review
                .note
                .as_deref()
                .unwrap_or("")
                .contains("Web page or newsletter"),
            "the sniff routed it: {:?}",
            review.note
        );
        assert_eq!(
            review.doc.stories[0].paragraphs[0].text(),
            "Words from a renamed export."
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn paste_with_markdown_markers_lands_as_runs() {
        // G1b: pasting "**hymn** 432" types the words plain
        // and bolds the hymn; a marker-less paste stays the
        // plain insert it always was.
        let mut app = App::default();
        app.enter_edit(0, 0);
        app.edit_text = "Sing ".into();
        app.edit_styles = vec!["body".into()];
        app.edit_runs = vec![vec![ob_model::Run::plain("Sing ")]];
        app.edit_layout = None;
        app.refresh_editor_layout();
        app.edit_anchor = 5;
        app.edit_caret = 5;
        app.editor_paste("**hymn** 432");
        assert_eq!(app.edit_text, "Sing hymn 432");
        assert!(
            app.edit_runs[0]
                .iter()
                .any(|r| r.text == "hymn" && r.bold == Some(true)),
            "the pasted word is a bold run: {:?}",
            app.edit_runs[0]
        );
        // A plain paste changes no faces.
        let end = app.edit_text.len();
        app.edit_anchor = end;
        app.edit_caret = end;
        app.editor_paste(" today");
        assert_eq!(app.edit_text, "Sing hymn 432 today");
        assert_eq!(
            app.edit_runs[0]
                .iter()
                .filter(|r| r.bold == Some(true))
                .count(),
            1,
            "still exactly the one bold run"
        );
    }

    #[test]
    fn typing_after_a_bold_word_stays_plain_and_after_it_bold() {
        // G1b inheritance at the session level: text typed
        // at the end of a bold run is bold; text typed after
        // the following plain run is plain.
        let mut app = App::default();
        app.enter_edit(0, 0);
        app.edit_text = "Sing hymn 432".into();
        app.edit_styles = vec!["body".into()];
        app.edit_runs = vec![vec![
            ob_model::Run::plain("Sing "),
            ob_model::Run {
                text: "hymn".into(),
                bold: Some(true),
                italic: None,
            },
            ob_model::Run::plain(" 432"),
        ]];
        app.edit_layout = None;
        app.refresh_editor_layout();
        // Caret at the end of "hymn" (display 9): type "!".
        app.edit_anchor = 9;
        app.edit_caret = 9;
        app.editor_insert_text("!");
        let bold_text: String = app.edit_runs[0]
            .iter()
            .filter(|r| r.bold == Some(true))
            .map(|r| r.text.as_str())
            .collect();
        assert_eq!(bold_text, "hymn!", "typed text joins the bold run");
        // Caret at the very end: type "." — plain.
        let end = app.edit_text.len();
        app.edit_anchor = end;
        app.edit_caret = end;
        app.editor_insert_text(".");
        let last = app.edit_runs[0].last().expect("a last run");
        assert_eq!(last.text, " 432.");
        assert_eq!(last.bold, None);
    }

    #[test]
    fn journal_never_touches_recovered_work_while_undecided() {
        // The closing audit's most dangerous finding, found by
        // three lanes: the tick must hold while a recovery is
        // pending, whatever the snapshots say.
        assert_eq!(
            journal_decision(false, false, true, false),
            JournalAction::Hold
        );
        assert_eq!(
            journal_decision(true, false, true, false),
            JournalAction::Hold
        );
        // A live peer instance owns the journal.
        assert_eq!(
            journal_decision(false, false, false, true),
            JournalAction::Hold
        );
        // Screen saved and journal stale: clear. Screen saved
        // and journal current: nothing. Unsaved and unjournaled:
        // write. Unsaved but journaled: nothing.
        assert_eq!(
            journal_decision(true, false, false, false),
            JournalAction::Clear
        );
        assert_eq!(
            journal_decision(true, true, false, false),
            JournalAction::Hold
        );
        assert_eq!(
            journal_decision(false, false, false, false),
            JournalAction::Write
        );
        assert_eq!(
            journal_decision(false, true, false, false),
            JournalAction::Hold
        );
    }

    #[test]
    fn open_guard_rearms_when_the_work_changes() {
        let mut app = App {
            frame_snapshot: "work-v1".into(),
            saved_snapshot: "saved".into(),
            ..Default::default()
        };
        assert!(!app.open_guard(PendingOpen::Plain), "first click arms");
        assert!(app.open_guard(PendingOpen::Plain), "second click proceeds");
        // New unsaved work appears: the guard must arm again,
        // not ride the old confirmation.
        app.frame_snapshot = "work-v2".into();
        assert!(!app.open_guard(PendingOpen::Plain), "changed work re-arms");
        // A different button does not inherit the arm either.
        assert!(
            !app.open_guard(PendingOpen::LastWeek),
            "other action re-arms"
        );
        // Clean screen: no guard at all.
        app.saved_snapshot = app.frame_snapshot.clone();
        assert!(app.open_guard(PendingOpen::Plain));
    }

    #[test]
    fn answers_map_to_plain_instructions() {
        let up = printer_setup_from_answer("up", "Brother");
        assert_eq!(up.mode, "long_edge");
        assert!(up.instruction.contains("long edge"));
        let down = printer_setup_from_answer("down", "Brother");
        assert_eq!(down.mode, "short_edge");
        let manual = printer_setup_from_answer("manual", "Brother");
        assert_eq!(manual.mode, "manual");
        assert!(manual.instruction.contains("one side at a time"));
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn announcements_buffer_only_writes_when_dirty() {
        // Regression guard: this gate was once lost in a file
        // splice while every test stayed green. The buffer may
        // only write when it changed (blocker 4).
        let mut app = App::default();
        let sid = weekly_story_id(&app.doc).expect("template has an announcements story");
        let before: Vec<String> = app
            .doc
            .story(&sid)
            .expect("story")
            .paragraphs
            .iter()
            .map(|p| p.text())
            .collect();
        app.announcements = "Potluck next Friday".to_string();
        app.announcements_dirty = false;
        app.save_announcements();
        let after: Vec<String> = app
            .doc
            .story(&sid)
            .expect("story")
            .paragraphs
            .iter()
            .map(|p| p.text())
            .collect();
        assert_eq!(before, after, "a clean buffer must not write");
        app.announcements_dirty = true;
        app.save_announcements();
        let written: Vec<String> = app
            .doc
            .story(&sid)
            .expect("story")
            .paragraphs
            .iter()
            .map(|p| p.text())
            .collect();
        assert!(written.iter().any(|t| t.contains("Potluck next Friday")));
        assert!(!app.announcements_dirty, "the write clears the flag");
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn announcements_preserve_blank_lines() {
        let mut app = App::default();
        app.announcements = "First item\n\nSecond item".to_string();
        app.announcements_dirty = true;
        app.save_announcements();
        let sid = weekly_story_id(&app.doc).expect("story");
        let texts: Vec<String> = app
            .doc
            .story(&sid)
            .expect("story")
            .paragraphs
            .iter()
            .map(|p| p.text())
            .collect();
        let joined = texts.join("|");
        assert!(
            joined.contains("First item||Second item"),
            "the blank line must survive as a paragraph break: {joined}"
        );
    }

    /// Replace the weekly story's paragraphs with a known
    /// set and return the story's id (audit Batch 1 test
    /// scaffolding for the announcements-save tests below).
    fn set_weekly_paragraphs(app: &mut App, paras: Vec<ob_model::Paragraph>) -> String {
        let sid = weekly_story_id(&app.doc).expect("template has an announcements story");
        app.doc.story_mut(&sid).expect("story").paragraphs = paras;
        sid
    }

    fn weekly_body_paras(app: &App, sid: &str) -> Vec<ob_model::Paragraph> {
        app.doc
            .story(sid)
            .expect("story")
            .paragraphs
            .iter()
            .filter(|p| p.style == "body")
            .cloned()
            .collect()
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn announcements_edited_line_keeps_its_spoken_only_flag() {
        // Audit Batch 1: flags used to ride by exact-text
        // match, so adding words to a prayer line silently
        // made it printable. The flag now follows the line.
        let mut app = App::default();
        let mut prayer = ob_model::Paragraph::new("body", "For Wilhelmina Quietname");
        prayer.spoken_only = true;
        let sid = set_weekly_paragraphs(
            &mut app,
            vec![
                ob_model::Paragraph::new("subheading", "Announcements"),
                ob_model::Paragraph::new("body", "Food bank collection this week"),
                prayer,
                ob_model::Paragraph::new("body", "Choir rehearsal Thursday"),
            ],
        );
        app.announcements = "Food bank collection this week\n\
                             For Wilhelmina Quietname and her family\n\
                             Choir rehearsal Thursday"
            .to_string();
        app.announcements_dirty = true;
        app.save_announcements();
        let body = weekly_body_paras(&app, &sid);
        assert_eq!(body.len(), 3);
        assert!(!body[0].spoken_only);
        assert_eq!(body[1].text(), "For Wilhelmina Quietname and her family");
        assert!(
            body[1].spoken_only,
            "an edited prayer line keeps its privacy"
        );
        assert!(!body[2].spoken_only);
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn announcements_identical_lines_keep_their_own_flags() {
        // Audit Batch 1: two identical lines, one private,
        // used to collapse into a single text-matched flag.
        let mut app = App::default();
        let mut spoken = ob_model::Paragraph::new("body", "For our shut-ins");
        spoken.spoken_only = true;
        let sid = set_weekly_paragraphs(
            &mut app,
            vec![
                ob_model::Paragraph::new("subheading", "Announcements"),
                spoken,
                ob_model::Paragraph::new("body", "For our shut-ins"),
            ],
        );
        app.announcements = "For our shut-ins\nFor our shut-ins".to_string();
        app.announcements_dirty = true;
        app.save_announcements();
        let body = weekly_body_paras(&app, &sid);
        assert_eq!(body.len(), 2);
        assert!(body[0].spoken_only, "the spoken-only line keeps its flag");
        assert!(!body[1].spoken_only, "its identical twin was never private");
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn announcements_save_keeps_canvas_bold_on_unchanged_lines() {
        // Audit Batch 1: the rebuild used to hand every
        // line plain runs, undoing bold set on the canvas.
        let mut app = App::default();
        let para = ob_model::Paragraph::from_runs(
            "body",
            vec![
                ob_model::Run::plain("Choir "),
                ob_model::Run {
                    text: "rehearsal".into(),
                    bold: Some(true),
                    italic: None,
                },
            ],
        );
        let sid = set_weekly_paragraphs(
            &mut app,
            vec![
                ob_model::Paragraph::new("subheading", "Announcements"),
                para,
            ],
        );
        app.announcements = "Choir rehearsal".to_string();
        app.announcements_dirty = true;
        app.save_announcements();
        let body = weekly_body_paras(&app, &sid);
        assert_eq!(body.len(), 1);
        assert!(
            body[0]
                .runs
                .iter()
                .any(|r| r.text == "rehearsal" && r.bold == Some(true)),
            "the bold word is still bold: {:?}",
            body[0].runs
        );
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn announcements_edited_line_keeps_its_first_runs_face() {
        // Audit Batch 1: an edited line takes the face of
        // the old paragraph's first run, as one run, and
        // keeps its flag.
        let mut app = App::default();
        let mut para = ob_model::Paragraph::from_runs(
            "body",
            vec![ob_model::Run {
                text: "Choir rehearsal".into(),
                bold: Some(true),
                italic: None,
            }],
        );
        para.spoken_only = true;
        let sid = set_weekly_paragraphs(
            &mut app,
            vec![
                ob_model::Paragraph::new("subheading", "Announcements"),
                para,
            ],
        );
        app.announcements = "Choir rehearsal moves to Friday".to_string();
        app.announcements_dirty = true;
        app.save_announcements();
        let body = weekly_body_paras(&app, &sid);
        assert_eq!(body.len(), 1);
        assert_eq!(
            body[0].runs.len(),
            1,
            "the edited line is one run wearing the old face"
        );
        assert_eq!(body[0].runs[0].text, "Choir rehearsal moves to Friday");
        assert_eq!(body[0].runs[0].bold, Some(true));
        assert!(body[0].spoken_only);
    }

    #[test]
    fn edit_session_commit_then_undo_round_trip() {
        let mut app = App::default();
        let (pi, fi, sid) = {
            let mut found = None;
            for (pi, panel) in app.doc.panels.iter().enumerate() {
                for (fi, f) in panel.frames.iter().enumerate() {
                    if let Some(sid) = &f.story_id {
                        found = Some((pi, fi, sid.clone()));
                        break;
                    }
                }
                if found.is_some() {
                    break;
                }
            }
            found.expect("a frame with a story")
        };
        let original_first = app
            .doc
            .story(&sid)
            .expect("story")
            .paragraphs
            .first()
            .map(|p| p.text())
            .unwrap_or_default();
        app.enter_edit(pi, fi);
        app.edit_text = "Line one\nLine two".to_string();
        app.sync_edit_styles();
        app.commit_edit();
        let texts: Vec<String> = app
            .doc
            .story(&sid)
            .expect("story")
            .paragraphs
            .iter()
            .map(|p| p.text())
            .collect();
        assert_eq!(texts, vec!["Line one", "Line two"]);
        app.undo();
        let restored = app
            .doc
            .story(&sid)
            .expect("story after undo")
            .paragraphs
            .first()
            .map(|p| p.text())
            .unwrap_or_default();
        assert_eq!(restored, original_first);
    }

    #[test]
    fn gesture_becomes_one_undo_step() {
        let mut app = App::default();
        let title = app.doc.title.clone();
        app.begin_gesture();
        app.doc.title = "Changed mid-gesture".to_string();
        app.end_gesture();
        app.undo();
        assert_eq!(app.doc.title, title);
    }

    #[test]
    fn menu_target_priority() {
        assert_eq!(
            choose_menu_target(Some(2), Some(0), Some(1)),
            MenuTarget::Guide(2)
        );
        assert_eq!(
            choose_menu_target(None, Some(0), Some(1)),
            MenuTarget::Frame(0)
        );
        assert_eq!(
            choose_menu_target(None, None, Some(1)),
            MenuTarget::Guide(1)
        );
        assert_eq!(choose_menu_target(None, None, None), MenuTarget::Page);
    }

    #[test]
    fn add_and_paste_boxes_round_trip_with_undo() {
        let mut app = App::default();
        let frames0 = app.doc.panels[0].frames.len();
        let stories0 = app.doc.stories.len();
        app.add_text_box(0);
        assert_eq!(app.doc.panels[0].frames.len(), frames0 + 1);
        assert_eq!(app.doc.stories.len(), stories0 + 1);
        // Copy the new box and paste it: another frame, a distinct story.
        let f = app.doc.panels[0].frames.last().expect("frame").clone();
        let story = f
            .story_id
            .as_ref()
            .and_then(|sid| app.doc.story(sid).cloned());
        let sid_first = f.story_id.clone();
        app.paste_box(0, f, story);
        assert_eq!(app.doc.panels[0].frames.len(), frames0 + 2);
        assert_eq!(app.doc.stories.len(), stories0 + 2);
        let sid_second = app.doc.panels[0]
            .frames
            .last()
            .expect("frame")
            .story_id
            .clone();
        assert_ne!(sid_first, sid_second, "a paste must own its words");
        app.undo();
        assert_eq!(app.doc.panels[0].frames.len(), frames0 + 1);
        app.undo();
        assert_eq!(app.doc.panels[0].frames.len(), frames0);
        assert_eq!(app.doc.stories.len(), stories0);
    }

    #[test]
    fn sunday_math_lands_on_sundays() {
        use chrono::Datelike;
        // 2026-10-15 is a Thursday; its Sunday is 2026-10-18.
        let thu = chrono::NaiveDate::from_ymd_opt(2026, 10, 15).expect("date");
        assert_eq!(
            sunday_on_or_after(thu),
            chrono::NaiveDate::from_ymd_opt(2026, 10, 18).expect("date")
        );
        // A Sunday maps to itself.
        let sun = chrono::NaiveDate::from_ymd_opt(2026, 10, 18).expect("date");
        assert_eq!(sunday_on_or_after(sun), sun);
        assert_eq!(sunday_on_or_after(thu).weekday(), chrono::Weekday::Sun);
    }

    #[test]
    fn opening_another_file_clears_undo_and_selection() {
        let mut app = App::default();
        app.undo_stack.push(snapshot(&app.doc));
        app.selected_frame = Some(2);
        app.canvas_panel = 3;
        let other = ob_layout::build_template(
            ob_layout::Preset::LetterPortrait,
            ob_model::ChurchProfile::default(),
            chrono::NaiveDate::from_ymd_opt(2026, 11, 1).expect("date"),
            "Sunday",
        );
        app.adopt_document(other, true);
        assert!(app.undo_stack.is_empty(), "history must not cross files");
        assert!(app.redo_stack.is_empty());
        assert!(app.selected_frame.is_none());
        assert_eq!(app.canvas_panel, 0);
        // Within one session, adopt keeps the stacks (Undo/Redo use it).
        app.undo_stack.push(snapshot(&app.doc));
        let third = ob_layout::build_template(
            ob_layout::Preset::LetterBifold,
            ob_model::ChurchProfile::default(),
            chrono::NaiveDate::from_ymd_opt(2026, 11, 8).expect("date"),
            "Sunday",
        );
        app.adopt_document(third, false);
        assert_eq!(app.undo_stack.len(), 1);
    }

    #[test]
    fn save_proposes_a_dated_name() {
        // J8: the proposed name carries the bulletin's Sunday;
        // a missing or unreadable date falls back to plain.
        let doc = ob_layout::build_template(
            ob_layout::Preset::LetterBifold,
            ob_model::ChurchProfile::default(),
            chrono::NaiveDate::from_ymd_opt(2026, 10, 18).expect("date"),
            "Sunday",
        );
        assert_eq!(suggested_save_name(&doc), "Bulletin 2026-10-18.obd");
        let mut no_date = doc.clone();
        no_date.fields.remove("occasion_date");
        assert_eq!(suggested_save_name(&no_date), "bulletin.obd");
        let mut bad_date = doc;
        bad_date
            .fields
            .insert("occasion_date".into(), "next Sunday-ish".into());
        assert_eq!(suggested_save_name(&bad_date), "bulletin.obd");
    }

    // J14's leader-line measurement moved into the shared
    // thread layout (ob-text threading::tab_line_cost) with
    // T1: a too-wide hymn line is charged two line heights
    // by the one walk the canvas, Check, and the PDF all
    // read. Its test lives there now.

    #[test]
    fn carried_lines_names_what_is_kept_and_through_when() {
        // J10: a story whose keep-through date covers the
        // bulletin's Sunday is named, as information; a spent
        // date carries nothing.
        let mut doc = ob_layout::build_template(
            ob_layout::Preset::LetterBifold,
            ob_model::ChurchProfile::default(),
            chrono::NaiveDate::from_ymd_opt(2026, 10, 18).expect("date"),
            "Sunday",
        );
        assert!(carried_lines(&doc).is_empty());
        let sid = weekly_story_id(&doc).expect("template has an announcements story");
        doc.stories
            .iter_mut()
            .find(|s| s.id == sid)
            .expect("story")
            .keep_through = Some("2026-11-08".into());
        let lines = carried_lines(&doc);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].starts_with("Carried into this week:"));
        assert!(
            lines[0].contains("kept through November 8, 2026"),
            "{}",
            lines[0]
        );
        assert!(!lines[0].starts_with("Check:"));
        doc.stories
            .iter_mut()
            .find(|s| s.id == sid)
            .expect("story")
            .keep_through = Some("2026-10-11".into());
        assert!(carried_lines(&doc).is_empty());
    }

    #[test]
    fn print_dialog_guidance_speaks_for_both_printer_states() {
        // J5: a learned printer's guidance quotes the learned
        // instruction and ends with the one-sheet check; an
        // unlearned one sends the user to the test sheet.
        let learned = print_dialog_guidance(true, Some("flip on long edge"));
        assert!(learned.contains("Actual size, never Fit to page"));
        assert!(learned.contains("flip on long edge"));
        assert!(learned.contains("Print one sheet first and check it"));
        let fresh = print_dialog_guidance(false, None);
        assert!(fresh.contains("Actual size, never Fit to page"));
        assert!(fresh.contains("run the test sheet below first"));
        // Known flag without words falls back to the test sheet.
        let odd = print_dialog_guidance(true, None);
        assert!(odd.contains("run the test sheet below first"));
    }

    #[test]
    fn print_checklist_names_paper_scale_and_learned_flip() {
        let doc = ob_layout::build_template(
            ob_layout::Preset::LetterBifold,
            ob_model::ChurchProfile::default(),
            chrono::NaiveDate::from_ymd_opt(2026, 10, 18).expect("date"),
            "Sunday",
        );
        let mut settings = Settings::default();
        let lines = print_checklist_for(&doc, &settings, "OfficeJet");
        assert!(lines[0].contains("Letter"), "paper named: {}", lines[0]);
        assert!(lines[0].contains("landscape"));
        assert!(lines.iter().any(|l| l.contains("Actual size")));
        assert!(lines.iter().any(|l| l.contains("not learned")));
        settings.printers.insert(
            "OfficeJet".into(),
            printer_setup_from_answer("up", "OfficeJet"),
        );
        let lines = print_checklist_for(&doc, &settings, "OfficeJet");
        assert!(
            lines.iter().any(|l| l.contains("flip on long edge")),
            "learned flip named: {lines:?}"
        );
    }

    #[test]
    fn journal_round_trip_and_clear() {
        let dir = std::env::temp_dir().join(format!("ob-app-journal-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let obd = dir.join("autosave.obd");
        let meta = dir.join("autosave.txt");
        let doc = ob_layout::build_template(
            ob_layout::Preset::LetterBifold,
            ob_model::ChurchProfile::default(),
            chrono::NaiveDate::from_ymd_opt(2026, 10, 18).expect("date"),
            "Sunday",
        );
        let source = Some(std::path::PathBuf::from("/tmp/church/bulletin.obd"));
        write_journal_to(&obd, &meta, &doc, &source, instance_token()).expect("write journal");
        let (back, back_source, when) = read_journal_from(&obd, &meta).expect("read journal");
        assert_eq!(snapshot(&back), snapshot(&doc));
        assert_eq!(back_source, source);
        assert!(when > 0);
        clear_journal_from(&obd, &meta);
        assert!(read_journal_from(&obd, &meta).is_none());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn unreadable_journal_is_set_aside_not_swallowed() {
        // Audit Batch 3: a journal damaged on disk used to be
        // skipped in silence. The startup path now sets it
        // aside under a name that says what happened.
        let dir = std::env::temp_dir().join(format!("ob-app-journal-bad-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let obd = dir.join("autosave.obd");
        let meta = dir.join("autosave.txt");
        std::fs::write(&obd, b"this is not a bulletin file").expect("write damaged journal");
        std::fs::write(&meta, "1729000000\n\n").expect("write journal note");
        assert!(read_journal_from(&obd, &meta).is_none());
        assert!(set_aside_unreadable_journal(&obd, &meta));
        assert!(!obd.exists() && !meta.exists());
        assert!(dir.join("autosave-unreadable.obd").exists());
        assert!(dir.join("autosave-unreadable.txt").exists());
        // Nothing left in the journal's place: a second call
        // has nothing to set aside.
        assert!(!set_aside_unreadable_journal(&obd, &meta));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn atomic_write_lands_whole_and_leaves_no_temp() {
        // HF-5 / S-MIN-1 pin: the app's durable-write helper
        // lands the new bytes whole, replaces an existing
        // target whole, and leaves no temp file behind. (The
        // sync calls themselves are not observable from a
        // test; they are verified by code inspection.)
        let dir = std::env::temp_dir().join(format!("ob-app-atomic-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("copy.pdf");
        std::fs::write(&path, b"the previous copy, longer than the new one").expect("seed");
        write_file_atomically(&path, b"new").expect("atomic write");
        assert_eq!(std::fs::read(&path).expect("read target"), b"new");
        assert!(!dir.join("copy.pdf.tmp").exists(), "no temp file remains");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn corrupt_hymn_memory_is_set_aside_and_missing_is_not() {
        // S-MIN-2 pin: a hymn-memory.json that exists but
        // does not parse is set aside with its damaged bytes
        // preserved, the memory starts empty, and the read
        // state carries the one-time note. A missing file
        // starts empty with no set-aside and no note.
        let dir = std::env::temp_dir().join(format!("ob-app-hymn-bad-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("hymn-memory.json");
        std::fs::write(&path, b"{ this is not a hymn memory").expect("write damaged memory");
        let (memory, read) = load_hymn_memory_from(&path);
        assert_eq!(read, HymnMemoryRead::SetAside);
        assert!(memory.pairs.is_empty(), "memory starts empty");
        assert!(!path.exists(), "damaged file moved out of the way");
        let aside = dir.join("hymn-memory-unreadable.json");
        assert_eq!(
            std::fs::read(&aside).expect("set-aside bytes"),
            b"{ this is not a hymn memory",
            "the damaged bytes are preserved"
        );
        // Missing file: empty, Clean, nothing set aside.
        let (memory, read) = load_hymn_memory_from(&path);
        assert_eq!(read, HymnMemoryRead::Clean);
        assert!(memory.pairs.is_empty());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn journal_note_carries_the_owner_token() {
        // S-MIN-3/S-MIN-5 pin: the journal written through
        // the atomic pair carries its owning instance's
        // token as the note's third line, round-trips as
        // before, and leaves no temp file behind.
        let dir = std::env::temp_dir().join(format!("ob-app-journal-token-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let obd = dir.join("autosave.obd");
        let meta = dir.join("autosave.txt");
        let doc = ob_layout::build_template(
            ob_layout::Preset::LetterBifold,
            ob_model::ChurchProfile::default(),
            chrono::NaiveDate::from_ymd_opt(2026, 10, 18).expect("date"),
            "Sunday",
        );
        write_journal_to(&obd, &meta, &doc, &None, instance_token()).expect("write journal");
        assert_eq!(
            journal_token_from(&meta).as_deref(),
            Some(instance_token()),
            "the note names this instance as the journal's owner"
        );
        assert!(read_journal_from(&obd, &meta).is_some(), "round-trip holds");
        assert!(
            !dir.join("autosave.txt.tmp").exists(),
            "no temp file remains"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn journal_retirement_respects_the_owner_token() {
        // S-MIN-3 pin: a journal whose note names a foreign
        // token is not retired while the lock still names
        // that same foreign token (a live second instance);
        // the same journal is retired once the lock no longer
        // names its owner, and a journal naming this
        // instance's own token is always retired.
        let dir =
            std::env::temp_dir().join(format!("ob-app-journal-retire-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let obd = dir.join("autosave.obd");
        let meta = dir.join("autosave.txt");
        let lock = dir.join("openbulletin.lock");
        let doc = ob_layout::build_template(
            ob_layout::Preset::LetterBifold,
            ob_model::ChurchProfile::default(),
            chrono::NaiveDate::from_ymd_opt(2026, 10, 18).expect("date"),
            "Sunday",
        );
        // A foreign instance's journal, with the lock still
        // naming that foreign instance.
        write_journal_to(&obd, &meta, &doc, &None, instance_token()).expect("write journal");
        std::fs::write(&meta, "1729000000\n\nforeign-token\n").expect("foreign note");
        std::fs::write(&lock, format!("{}\nforeign-token\n", std::process::id()))
            .expect("foreign lock");
        assert!(
            !retire_journal_from(&obd, &meta, &lock, instance_token()),
            "a live foreign owner's journal is not retired"
        );
        assert!(obd.exists() && meta.exists(), "the journal survives");
        // The lock no longer names that owner: it exited, and
        // the journal may be retired through the usual path.
        // (The lock is taken through the claim itself, the
        // only production writer since R31-B.)
        assert!(claim_instance_lock_from(&lock, instance_token()));
        assert!(
            retire_journal_from(&obd, &meta, &lock, instance_token()),
            "an exited owner's journal may be retired"
        );
        assert!(!obd.exists() && !meta.exists(), "the journal is cleared");
        // Own-token retirement always works.
        write_journal_to(&obd, &meta, &doc, &None, instance_token()).expect("write own journal");
        assert!(retire_journal_from(&obd, &meta, &lock, instance_token()));
        assert!(!obd.exists() && !meta.exists());
        // The pure rule: a provably gone owner may always be
        // retired, even while a stale lock still names it.
        assert!(journal_retirement_allowed(
            Some("foreign-token"),
            "own-token",
            Some("foreign-token"),
            true
        ));
        std::fs::remove_dir_all(&dir).ok();
    }

    /// A live peer process for the lock pins: a copy of
    /// the system sleep binary under a name that reads as
    /// this app, so the /proc identity refinement judges
    /// it the way it judges a real OpenBulletin process
    /// (alive, and not provably a recycled stranger). The
    /// caller kills and reaps it. Modeled this way since
    /// R31-B: a pid-0 lock is a corrupt lock under F6, not
    /// a live peer — a healthy instance always writes its
    /// real pid, so a live peer is modeled with one.
    #[cfg(unix)]
    fn spawn_live_peer(dir: &std::path::Path) -> std::process::Child {
        let peer_exe = dir.join("openbulletin-peer");
        let sleep_source = ["/bin/sleep", "/usr/bin/sleep"]
            .into_iter()
            .map(std::path::Path::new)
            .find(|candidate| candidate.exists())
            .expect("a system sleep binary to model the peer with");
        std::fs::copy(sleep_source, &peer_exe).expect("peer binary copy");
        std::process::Command::new(&peer_exe)
            .arg("30")
            .spawn()
            .expect("spawn live peer")
    }

    #[cfg(unix)]
    #[test]
    fn startup_claim_leaves_a_live_peers_lock_and_journal_alone() {
        // R30-A pin, the round-30 F1 witness: instance A
        // launched and journaled (the journal note and the
        // lock both name A); instance B launches and then
        // saves an ordinary bulletin. Before the fix, B's
        // startup overwrote the lock with B's token, and B's
        // save retired A's journal through the "lock no
        // longer names the owner" rule while A was still
        // running. A is a real live process (see
        // `spawn_live_peer`): alive, and not provably gone.
        let dir = std::env::temp_dir().join(format!("ob-app-startup-claim-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let obd = dir.join("autosave.obd");
        let meta = dir.join("autosave.txt");
        let lock = dir.join("openbulletin.lock");
        let mut peer = spawn_live_peer(&dir);
        let peer_pid = peer.id();
        let doc = ob_layout::build_template(
            ob_layout::Preset::LetterBifold,
            ob_model::ChurchProfile::default(),
            chrono::NaiveDate::from_ymd_opt(2026, 10, 18).expect("date"),
            "Sunday",
        );
        // A writes its journal; the lock names A.
        write_journal_to(&obd, &meta, &doc, &None, "foreign-token").expect("A writes journal");
        let peer_lock = format!("{peer_pid}\nforeign-token\n");
        std::fs::write(&lock, peer_lock).expect("A's lock");
        assert!(
            peer_instance_alive_from(&lock, std::process::id(), instance_token()),
            "the lock reads as a live peer before B starts"
        );
        let lock_before = std::fs::read_to_string(&lock).expect("read A's lock");
        // B's startup: the claim is refused, and the lock
        // file is left byte for byte as A left it.
        assert!(
            !claim_instance_lock_from(&lock, instance_token()),
            "startup does not take a live peer's lock"
        );
        assert_eq!(
            std::fs::read_to_string(&lock).expect("read lock after B starts"),
            lock_before,
            "the lock file is untouched"
        );
        // B saves: the retirement that `mark_clean` sends
        // through the journal worker refuses too, because
        // the lock still names A and A is not provably gone.
        assert!(
            !retire_journal_from(&obd, &meta, &lock, instance_token()),
            "B's save does not retire A's journal"
        );
        assert!(obd.exists() && meta.exists(), "A's journal survives");
        let _ = peer.kill();
        let _ = peer.wait();
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn startup_claim_takes_a_lock_no_live_peer_holds() {
        // R30-A pin, the other side: with no live peer on
        // the lock, startup takes it exactly as it always
        // has — an absent lock, and a lock too damaged to
        // read. (A lock naming a dead owner is the unix-only
        // segment below: deadness is a liveness answer.)
        let dir =
            std::env::temp_dir().join(format!("ob-app-startup-takeover-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let lock = dir.join("openbulletin.lock");
        assert!(
            claim_instance_lock_from(&lock, instance_token()),
            "an absent lock is taken"
        );
        let info = read_instance_lock_from(&lock).expect("the lock was written");
        assert_eq!(info.pid, Some(std::process::id()));
        assert_eq!(info.token.as_deref(), Some(instance_token()));
        std::fs::write(&lock, "this is not a lock\n").expect("damaged lock");
        assert!(
            claim_instance_lock_from(&lock, instance_token()),
            "an unreadable lock is taken"
        );
        let info = read_instance_lock_from(&lock).expect("the lock was rewritten");
        assert_eq!(info.token.as_deref(), Some(instance_token()));
        #[cfg(unix)]
        {
            let mut child = std::process::Command::new("/bin/sh")
                .arg("-c")
                .arg("exit 0")
                .spawn()
                .expect("spawn child");
            let dead_pid = child.id();
            child.wait().expect("reap child");
            let dead_lock = format!("{dead_pid}\nforeign-token\n");
            std::fs::write(&lock, dead_lock).expect("dead owner's lock");
            assert!(
                claim_instance_lock_from(&lock, instance_token()),
                "a dead owner's lock is taken"
            );
            let info = read_instance_lock_from(&lock).expect("the lock was rewritten");
            assert_eq!(info.pid, Some(std::process::id()));
            assert_eq!(info.token.as_deref(), Some(instance_token()));
        }
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn liveness_never_proves_an_unnamable_pid_gone() {
        // CI-FIX pin: pid 0 names a whole process group and
        // a pid above i32::MAX names nothing at all, so
        // neither can be provably gone, on any platform and
        // without any syscall result being consulted.
        for pid in [0, i32::MAX as u32 + 1, u32::MAX] {
            assert_eq!(peer_liveness(pid), PeerLiveness::Unknown, "pid {pid}");
            assert!(!peer_provably_gone(pid), "pid {pid} is never provably gone");
        }
    }

    #[cfg(unix)]
    #[test]
    fn liveness_reads_a_live_pid_as_not_gone() {
        // CI-FIX pin: this test process is alive, so its
        // own pid is Alive and never provably gone — the
        // macOS failure was every pid reading as gone
        // because /proc does not exist there.
        let own = std::process::id();
        assert_eq!(peer_liveness(own), PeerLiveness::Alive);
        assert!(!peer_provably_gone(own), "a live pid is not provably gone");
    }

    #[cfg(unix)]
    #[test]
    fn liveness_reads_a_reaped_child_as_gone() {
        // CI-FIX pin: a child that has exited and been
        // reaped leaves a pid the kernel reports as ESRCH,
        // which is Dead, and provably gone.
        let mut child = std::process::Command::new("/bin/sh")
            .arg("-c")
            .arg("exit 0")
            .spawn()
            .expect("spawn child");
        let pid = child.id();
        child.wait().expect("reap child");
        assert_eq!(peer_liveness(pid), PeerLiveness::Dead, "reaped pid {pid}");
        assert!(peer_provably_gone(pid), "a reaped pid is provably gone");
    }

    #[cfg(unix)]
    #[test]
    fn peer_alive_follows_the_liveness_primitive() {
        // CI-FIX pins for peer_instance_alive_from's unix
        // decision table: a foreign lock naming a reaped
        // pid names no live peer; a foreign lock naming a
        // live process that is not provably a recycled
        // stranger reads as a live peer. (The Unknown side
        // of the table is pinned at the primitive by
        // `liveness_never_proves_an_unnamable_pid_gone`;
        // since R31-B a lock naming such a pid is corrupt
        // under F6, not a peer, and the corrupt pins below
        // pin that shape at this function.)
        let dir = std::env::temp_dir().join(format!("ob-app-peer-alive-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let lock = dir.join("openbulletin.lock");
        let own = std::process::id();
        let mut child = std::process::Command::new("/bin/sh")
            .arg("-c")
            .arg("exit 0")
            .spawn()
            .expect("spawn child");
        let dead_pid = child.id();
        child.wait().expect("reap child");
        std::fs::write(&lock, format!("{dead_pid}\nforeign-token\n")).expect("dead lock");
        assert!(
            !peer_instance_alive_from(&lock, own, instance_token()),
            "a reaped foreign pid is not a live peer"
        );
        let mut peer = spawn_live_peer(&dir);
        let live_pid = peer.id();
        std::fs::write(&lock, format!("{live_pid}\nforeign-token\n")).expect("live lock");
        assert!(
            peer_instance_alive_from(&lock, own, instance_token()),
            "a live foreign pid reads as a live peer"
        );
        let _ = peer.kill();
        let _ = peer.wait();
        std::fs::remove_dir_all(&dir).ok();
    }

    #[cfg(not(unix))]
    #[test]
    fn peer_alive_holds_for_a_foreign_lock_off_unix() {
        // R31-B pin, rewriting the CI-FIX pin that asserted
        // the defective token-only posture: off Unix the
        // invariant holds too — a lock naming a foreign
        // token whose owner cannot be proven gone reads as
        // a live peer, and the claim refuses it and leaves
        // it byte for byte. The owner is modeled with pid
        // 4, the Windows System process: a process that
        // always exists and cannot be proven gone (the
        // query answers Alive, or Unknown where it may not
        // be queried — both read as alive). On a non-Unix
        // platform with no liveness primitive the answer is
        // Unknown, which reads the same way.
        let dir = std::env::temp_dir().join(format!("ob-app-peer-alive-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let lock = dir.join("openbulletin.lock");
        std::fs::write(&lock, "4\nforeign-token\n").expect("foreign lock");
        assert!(
            peer_instance_alive_from(&lock, std::process::id(), instance_token()),
            "a foreign lock of unprovable liveness reads as a live peer off Unix"
        );
        let lock_before = std::fs::read_to_string(&lock).expect("read foreign lock");
        assert!(
            !claim_instance_lock_from(&lock, instance_token()),
            "the claim refuses a foreign lock it cannot prove ownerless off Unix"
        );
        assert_eq!(
            std::fs::read_to_string(&lock).expect("read lock after claim"),
            lock_before,
            "the lock file is untouched"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn claim_interleaving_on_an_unheld_lock_admits_exactly_one_claimant() {
        // R31-B pin for F3, modeled on the red team's
        // lockreplica: force the check-check-write-write
        // interleaving at the `_from` seams — both
        // claimants run the check while the lock is unheld
        // (both see no live peer), then both run the write
        // step. Exactly one holds: the first create-new
        // wins, the loser's write step re-reads, judges the
        // winner a live peer, answers false, and leaves the
        // winner's lock byte for byte. Claimant A is this
        // process, so a lock naming A reads as a live peer
        // to B on every platform; B is a simulated second
        // instance (a distinct pid and token) whose pid is
        // only ever an identity, never a liveness subject.
        let dir = std::env::temp_dir().join(format!("ob-app-claim-race-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let lock = dir.join("openbulletin.lock");
        let a_pid = std::process::id();
        let a_token = "claimant-a-token";
        let b_pid = a_pid + 1000;
        let b_token = "claimant-b-token";
        // Check, check.
        assert!(
            !peer_instance_alive_from(&lock, a_pid, a_token),
            "A's check: no live peer on an unheld lock"
        );
        assert!(
            !peer_instance_alive_from(&lock, b_pid, b_token),
            "B's check: no live peer on an unheld lock"
        );
        // Write, write.
        assert!(
            acquire_instance_lock_from(&lock, a_pid, a_token),
            "A's write step takes the unheld lock"
        );
        let winner_bytes = std::fs::read(&lock).expect("read winner's lock");
        assert!(
            !acquire_instance_lock_from(&lock, b_pid, b_token),
            "B's write step loses the create race and answers false"
        );
        assert_eq!(
            std::fs::read(&lock).expect("read lock after B's write step"),
            winner_bytes,
            "the loser's write step leaves the winner's lock byte for byte"
        );
        let info = read_instance_lock_from(&lock).expect("the lock names the winner");
        assert_eq!(info.pid, Some(a_pid));
        assert_eq!(info.token.as_deref(), Some(a_token));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[cfg(unix)]
    #[test]
    fn claim_interleaving_on_a_stale_lock_admits_exactly_one_claimant() {
        // R31-B pin for F3's takeover half: the same
        // check-check-write-write interleaving, but the
        // lock already stands naming a dead owner. Both
        // claimants judge it takeable; the first takes it
        // through the atomic path (displace, create-new,
        // re-read), and the second then faces a live peer's
        // lock — not the stale one it checked — and refuses
        // it, leaving the winner's lock byte for byte.
        let dir = std::env::temp_dir().join(format!("ob-app-claim-stale-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let lock = dir.join("openbulletin.lock");
        let mut child = std::process::Command::new("/bin/sh")
            .arg("-c")
            .arg("exit 0")
            .spawn()
            .expect("spawn child");
        let dead_pid = child.id();
        child.wait().expect("reap child");
        let stale_lock = format!("{dead_pid}\nforeign-token\n");
        std::fs::write(&lock, stale_lock).expect("stale lock");
        let a_pid = std::process::id();
        let a_token = "claimant-a-token";
        let b_pid = a_pid + 1000;
        let b_token = "claimant-b-token";
        // Check, check.
        assert!(
            !peer_instance_alive_from(&lock, a_pid, a_token),
            "A's check: the stale lock names no live peer"
        );
        assert!(
            !peer_instance_alive_from(&lock, b_pid, b_token),
            "B's check: the stale lock names no live peer"
        );
        // Write, write.
        assert!(
            acquire_instance_lock_from(&lock, a_pid, a_token),
            "A's write step takes the stale lock over"
        );
        let winner_bytes = std::fs::read(&lock).expect("read winner's lock");
        assert!(
            !acquire_instance_lock_from(&lock, b_pid, b_token),
            "B's write step now faces a live peer and answers false"
        );
        assert_eq!(
            std::fs::read(&lock).expect("read lock after B's write step"),
            winner_bytes,
            "the loser's write step leaves the winner's lock byte for byte"
        );
        let info = read_instance_lock_from(&lock).expect("the lock names the winner");
        assert_eq!(info.pid, Some(a_pid));
        assert_eq!(info.token.as_deref(), Some(a_token));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn claim_answers_false_when_the_lock_write_fails() {
        // R31-B pin for F3's truthful half: the claim's
        // contract ("answers true when this instance took
        // the lock and now holds it") fails closed when the
        // write cannot land — here the lock path is
        // obstructed by a directory, and separately the
        // lock's folder does not exist at all.
        let dir = std::env::temp_dir().join(format!("ob-app-claim-fails-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let lock = dir.join("openbulletin.lock");
        std::fs::create_dir(&lock).expect("obstruct the lock path");
        assert!(
            !claim_instance_lock_from(&lock, instance_token()),
            "a claim whose write is obstructed answers false"
        );
        assert!(lock.is_dir(), "the obstruction is left standing");
        let missing = dir.join("no-such-folder").join("openbulletin.lock");
        assert!(
            !claim_instance_lock_from(&missing, instance_token()),
            "a claim into a missing folder answers false"
        );
        assert!(
            !dir.join("no-such-folder").exists(),
            "the failed claim created nothing"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_corrupt_lock_is_no_peer_and_its_journal_is_retirable() {
        // R31-B pins for F6: a lock whose pid field cannot
        // name a process — 0, an over-range pid (just past
        // the boundary and at the u32 ceiling), a garbage
        // pid field, and a missing pid line — is corrupt,
        // not a peer: it does not read as a live peer, the
        // claim takes it through the atomic path, and a
        // journal held under it is retirable instead of
        // wedged for as long as the damaged file stands.
        let dir = std::env::temp_dir().join(format!("ob-app-corrupt-lock-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let doc = ob_layout::build_template(
            ob_layout::Preset::LetterBifold,
            ob_model::ChurchProfile::default(),
            chrono::NaiveDate::from_ymd_opt(2026, 10, 18).expect("date"),
            "Sunday",
        );
        let shapes = [
            ("pid zero", "0\nforeign-token\n"),
            ("pid just over range", "2147483648\nforeign-token\n"),
            ("pid at u32 ceiling", "4294967295\nforeign-token\n"),
            ("garbage pid field", "garbage\nforeign-token\n"),
            ("missing pid line", "\nforeign-token\n"),
        ];
        for (index, (shape, lock_text)) in shapes.into_iter().enumerate() {
            let case = dir.join(format!("case-{index}"));
            std::fs::create_dir_all(&case).expect("case dir");
            let obd = case.join("autosave.obd");
            let meta = case.join("autosave.txt");
            let lock = case.join("openbulletin.lock");
            std::fs::write(&lock, lock_text).expect("corrupt lock");
            assert!(
                !peer_instance_alive_from(&lock, std::process::id(), instance_token()),
                "{shape}: a corrupt lock does not read as a live peer"
            );
            // A journal held under the corrupt lock: the
            // note and the lock both name the foreign
            // token, the shape that wedges when the lock
            // reads as that token's live owner.
            write_journal_to(&obd, &meta, &doc, &None, "foreign-token")
                .expect("write foreign journal");
            assert!(
                retire_journal_from(&obd, &meta, &lock, instance_token()),
                "{shape}: a journal under a corrupt lock is retirable"
            );
            assert!(
                !obd.exists() && !meta.exists(),
                "{shape}: the journal is cleared"
            );
            assert!(
                claim_instance_lock_from(&lock, instance_token()),
                "{shape}: the claim takes a corrupt lock"
            );
            let info = read_instance_lock_from(&lock).expect("the lock was rewritten");
            assert_eq!(info.pid, Some(std::process::id()), "{shape}");
            assert_eq!(info.token.as_deref(), Some(instance_token()), "{shape}");
        }
        std::fs::remove_dir_all(&dir).ok();
    }

    /// Poll the journal worker until no job is in flight,
    /// none is waiting, and no retirement is outstanding.
    fn poll_journal_until_idle(app: &mut App) {
        let ctx = egui::Context::default();
        for _ in 0..500 {
            app.poll_journal(&ctx);
            if app.journal_flight.is_none()
                && app.journal_pending.is_none()
                && !app.journal_retiring
            {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        panic!("the journal worker did not go idle");
    }

    /// An app whose current document reads as saved and
    /// journaled, so the next edit is exactly one Write
    /// decision.
    fn clean_app_with_template() -> App {
        let mut app = App::default();
        app.doc = ob_layout::build_template(
            ob_layout::Preset::LetterBifold,
            ob_model::ChurchProfile::default(),
            chrono::NaiveDate::from_ymd_opt(2026, 10, 18).expect("date"),
            "Sunday",
        );
        app.saved_snapshot = app.canonical_snapshot();
        app.journal_snapshot = app.saved_snapshot.clone();
        app
    }

    fn tick_journal_now(app: &mut App, obd: &std::path::Path, meta: &std::path::Path) {
        app.last_journal_check = std::time::Instant::now() - std::time::Duration::from_secs(3);
        app.journal_tick_to(obd, meta);
    }

    #[test]
    fn journal_write_lands_through_the_worker() {
        // HF-4 pin (a): a dirty document ticked and polled
        // lands a journal bulletin and note that load back
        // through the journal read path, carrying this
        // session's token, and the journal snapshot follows
        // the landed write.
        let dir =
            std::env::temp_dir().join(format!("ob-app-journal-worker-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let obd = dir.join("autosave.obd");
        let meta = dir.join("autosave.txt");
        let mut app = clean_app_with_template();
        app.doc.title = "Work from the worker".into();
        tick_journal_now(&mut app, &obd, &meta);
        poll_journal_until_idle(&mut app);
        let (back, _source, when) = read_journal_from(&obd, &meta).expect("journal read back");
        assert!(when > 0);
        assert_eq!(snapshot(&back), app.canonical_snapshot());
        assert_eq!(
            journal_token_from(&meta).as_deref(),
            Some(instance_token()),
            "the note carries the session token"
        );
        assert_eq!(
            app.journal_snapshot,
            app.canonical_snapshot(),
            "the landed write is the journaled state"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn journal_write_failure_is_noted_once_per_session() {
        // HF-4 pin (b): an unwritable journal destination
        // sets the once-per-session failure note through
        // the worker path (H-C's behavior, off the frame
        // thread), and a second failure does not re-note.
        let dir = std::env::temp_dir().join(format!("ob-app-journal-fails-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        // A folder that does not exist: the save cannot land.
        let obd = dir.join("missing").join("autosave.obd");
        let meta = dir.join("missing").join("autosave.txt");
        let mut app = clean_app_with_template();
        app.doc.title = "Nowhere to land".into();
        tick_journal_now(&mut app, &obd, &meta);
        poll_journal_until_idle(&mut app);
        assert!(app.journal_failure_noted, "the failure is noted");
        assert!(app.status_error, "the note is a failure banner");
        assert!(
            app.status.contains("safety copy could not be written"),
            "spoken in plain words: {}",
            app.status
        );
        // A second failure stays quiet: replace the banner
        // with a marker and fail again.
        app.status = "a later, unrelated note".into();
        app.status_error = false;
        app.doc.title = "Still nowhere to land".into();
        tick_journal_now(&mut app, &obd, &meta);
        poll_journal_until_idle(&mut app);
        assert!(app.journal_failure_noted);
        assert_eq!(
            app.status, "a later, unrelated note",
            "the second failure does not re-note"
        );
        assert!(!app.status_error);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn journal_retirement_after_a_queued_write_leaves_no_journal() {
        // HF-4 pin (c): retirement requested after a queued
        // write leaves no live journal behind. The first
        // write is in flight, a second write is the
        // coalesced pending job, and the retire replaces
        // that pending write and lands after the flight:
        // the serialized-order property, driven at the App
        // level through the worker.
        let dir =
            std::env::temp_dir().join(format!("ob-app-journal-retire-q-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let obd = dir.join("autosave.obd");
        let meta = dir.join("autosave.txt");
        let mut app = clean_app_with_template();
        app.doc.title = "First unsaved state".into();
        tick_journal_now(&mut app, &obd, &meta);
        assert!(app.journal_flight.is_some(), "the first write is in flight");
        app.doc.title = "Second unsaved state".into();
        tick_journal_now(&mut app, &obd, &meta);
        assert!(
            app.journal_pending.is_some(),
            "the second write waits as the one pending job"
        );
        app.request_journal_retire(&obd, &meta);
        poll_journal_until_idle(&mut app);
        assert!(!obd.exists(), "no journal bulletin remains");
        assert!(!meta.exists(), "no journal note remains");
        assert!(read_journal_from(&obd, &meta).is_none());
        // The gate reopens when the retirement lands: new
        // unsaved work journals again.
        app.doc.title = "Work after the retirement".into();
        tick_journal_now(&mut app, &obd, &meta);
        poll_journal_until_idle(&mut app);
        assert!(
            read_journal_from(&obd, &meta).is_some(),
            "journaling resumes after retirement"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn catalog_worker_disconnect_speaks() {
        // S-MIN-4 pin: when the catalog worker's channel
        // disconnects (the worker stopped), poll_catalog
        // clears the busy state AND says so, like poll_import.
        let mut app = App::default();
        let (tx, rx) = std::sync::mpsc::channel::<CatalogMsg>();
        app.catalog_rx = Some(rx);
        app.catalog_busy = true;
        drop(tx);
        app.poll_catalog(&egui::Context::default());
        assert!(app.catalog_rx.is_none());
        assert!(!app.catalog_busy);
        assert!(app.status_error, "the stop is a failure banner");
        assert!(
            app.status.contains("font catalog stopped"),
            "spoken in plain words: {}",
            app.status
        );
    }

    #[test]
    fn char_findings_flag_only_what_print_cannot_show() {
        // Workflow review, adopted item 1: a Latin cross and an
        // emoji must be named (the bundled Liberation faces have
        // no glyph for either, so each would print as the face's
        // missing-glyph box); the dagger and curly quotes print
        // as themselves (the faces have them) and must not be
        // flagged. The test is glyph presence in the printing
        // face, the same test the PDF writer applies (O2).
        let mut doc = ob_layout::build_template(
            ob_layout::Preset::LetterBifold,
            ob_model::ChurchProfile::default(),
            chrono::NaiveDate::from_ymd_opt(2026, 10, 18).expect("date"),
            "Sunday",
        );
        // The fresh template is clean (tabs in the order of
        // service are dot-leader marks, not characters).
        assert!(char_findings(&doc).is_empty());
        let sid = doc.stories[2].id.clone();
        doc.story_mut(&sid)
            .expect("story")
            .paragraphs
            .push(ob_model::Paragraph::new(
                "body",
                "The \u{271D} marks the spot \u{1F600}",
            ));
        doc.story_mut(&sid)
            .expect("story")
            .paragraphs
            .push(ob_model::Paragraph::new(
                "body",
                "A dagger \u{2020} and \u{201C}quotes\u{201D} print fine",
            ));
        let findings = char_findings(&doc);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].story_id, sid);
        assert_eq!(findings[0].chars, vec!['\u{271D}', '\u{1F600}']);
        let note = char_note(&findings[0]);
        assert!(note.contains("\u{271D}"), "the note names the mark: {note}");
        assert!(
            note.contains("empty box"),
            "the note says what happens: {note}"
        );
    }

    #[test]
    fn announcements_check_distinguishes_empty_from_none() {
        // Adopted item 4: filled is fine; empty is fine only
        // when the office marked that there are none; empty and
        // unmarked still needs a person.
        assert!(announcements_check(false, false).starts_with("OK"));
        assert!(announcements_check(true, true).starts_with("OK"));
        let needed = announcements_check(true, false);
        assert!(needed.starts_with("Check"));
        assert!(needed.contains("No announcements this week"));
    }

    #[test]
    fn start_status_names_the_next_thing_needed() {
        // Adopted item 3, derived from the document each time.
        // Today is the Thursday before the bulletin's Sunday,
        // so the date itself raises no flag (J1).
        let today = chrono::NaiveDate::from_ymd_opt(2026, 10, 15).expect("date");
        let mut doc = ob_layout::build_template(
            ob_layout::Preset::LetterBifold,
            ob_model::ChurchProfile::default(),
            chrono::NaiveDate::from_ymd_opt(2026, 10, 18).expect("date"),
            "Sunday",
        );
        // No church name yet: that is the first thing needed.
        doc.profile.name = String::new();
        let (headline, next) = start_status(&doc, true, today, None);
        assert!(headline.starts_with("Working on: Sunday,"));
        assert!(next.contains("church name"), "next: {next}");
        // Name set, date set, announcements empty and unmarked.
        doc.profile.name = "Grace Lutheran Church".into();
        let (_, next) = start_status(&doc, true, today, None);
        assert!(next.contains("Announcements step"), "next: {next}");
        // Announcements present: nothing left but printing.
        let (_, next) = start_status(&doc, false, today, None);
        assert_eq!(next, "Ready to print.");
        // Or marked as none this week: also ready.
        doc.no_announcements = true;
        let (_, next) = start_status(&doc, true, today, None);
        assert_eq!(next, "Ready to print.");
        // No date at all: the This Sunday step is next, and the
        // headline cannot name a Sunday.
        doc.fields.remove("occasion_date");
        doc.fields.remove("occasion_date_long");
        let (headline, next) = start_status(&doc, false, today, None);
        assert_eq!(headline, "Working on: a new bulletin");
        assert!(next.contains("This Sunday step"), "next: {next}");
    }

    #[test]
    fn date_verdict_flags_anything_but_the_coming_sunday() {
        // J1: past, future-but-not-next, and unparseable dates.
        let thursday = chrono::NaiveDate::from_ymd_opt(2026, 10, 15).expect("date");
        // The coming Sunday itself: no finding.
        assert_eq!(date_verdict("2026-10-18", thursday), None);
        // Last Sunday: named, with the coming Sunday beside it.
        let v = date_verdict("2026-10-11", thursday).expect("a past date is a finding");
        assert!(v.contains("Sunday, October 11, 2026"), "verdict: {v}");
        assert!(v.contains("October 18, 2026"), "verdict: {v}");
        // A Sunday, but not the coming one.
        let v = date_verdict("2026-10-25", thursday).expect("a later Sunday is a finding");
        assert!(v.contains("October 25, 2026"), "verdict: {v}");
        // When today IS Sunday, the coming Sunday is today.
        let sunday = chrono::NaiveDate::from_ymd_opt(2026, 10, 18).expect("date");
        assert_eq!(date_verdict("2026-10-18", sunday), None);
        assert!(date_verdict("2026-10-11", sunday).is_some());
        // A date that does not read gets no verdict.
        assert_eq!(date_verdict("next Friday", thursday), None);
        assert_eq!(date_verdict("", thursday), None);
    }

    #[test]
    fn start_status_puts_a_passed_date_first() {
        // J1: a bulletin for a Sunday already gone outranks
        // every other need, church name included.
        let today = chrono::NaiveDate::from_ymd_opt(2026, 10, 15).expect("date");
        let mut doc = ob_layout::build_template(
            ob_layout::Preset::LetterBifold,
            ob_model::ChurchProfile::default(),
            chrono::NaiveDate::from_ymd_opt(2026, 10, 11).expect("date"),
            "Sunday",
        );
        doc.profile.name = String::new();
        let (_, next) = start_status(&doc, true, today, None);
        assert!(next.starts_with("That date has passed."), "next: {next}");
        assert_eq!(
            date_passed_sentence(Some("2026-10-11"), today).as_deref(),
            Some("That date has passed. Set the date for this Sunday on the This Sunday step.")
        );
        assert_eq!(date_passed_sentence(Some("2026-10-18"), today), None);
        assert_eq!(date_passed_sentence(None, today), None);
    }

    #[test]
    fn removed_serving_lines_are_named_in_the_status_words() {
        // Round 4, RT-6c: names cleared with a removed
        // serving line are reported, in words that stay
        // honest for one line or several.
        assert_eq!(
            removed_serving_lines_sentence(&["Acolytes".to_string()]),
            "Names saved for 'Acolytes' were removed with the line."
        );
        assert_eq!(
            removed_serving_lines_sentence(&[
                "Acolytes".to_string(),
                "Nursery workers".to_string(),
            ]),
            "Names saved for 'Acolytes', 'Nursery workers' were removed with their lines."
        );
    }

    #[test]
    fn r12c_unfollowed_serving_pairings_are_named_in_the_status_words() {
        // Round 12 (R12-11, review R12-8): a duplicate
        // pairing, or a removal of an already-paired
        // name, is not followed (round 11, R11-D) —
        // and the save now says so, in the same status
        // the handler builds from `migration.removed`:
        // the sentence names the old name, the pairing
        // that was kept, and the event that was not
        // followed, so the office can correct the
        // names by hand. A migration with no duplicates
        // carries no note.
        let lines =
            |names: &[&str]| -> Vec<String> { names.iter().map(|n| n.to_string()).collect() };
        let old_lines = lines(&["Alpha", "Xray", "Beta", "Xray", "Gamma"]);
        // The review's double-pairing shape: the
        // people land under the first target, the
        // second target's row stands blank, and the
        // status names both pairings.
        let new_lines = lines(&["Alpha", "Yankee", "Beta", "Zulu", "Gamma"]);
        let mut participants: std::collections::BTreeMap<String, String> =
            std::collections::BTreeMap::new();
        participants.insert("xray".to_string(), "Ann".to_string());
        let mut aliases: std::collections::BTreeMap<String, String> =
            std::collections::BTreeMap::new();
        let migration = ob_model::migrate_serving_line_renames_detailed(
            &old_lines,
            &new_lines,
            &mut participants,
            &mut aliases,
        );
        assert!(!migration.ambiguous);
        assert_eq!(participants.get("yankee").map(String::as_str), Some("Ann"));
        assert!(!participants.contains_key("zulu"));
        assert_eq!(
            unfollowed_serving_lines_sentence(&migration.unfollowed),
            "'Xray' was paired with 'Yankee' and with 'Zulu': the names \
             went to 'Yankee', and the pairing with 'Zulu' was not \
             followed. Correct the names by hand if they belong under \
             'Zulu'."
        );
        // The unfollowed removal: paired in one
        // segment, removal-only in another.
        let new_lines = lines(&["Alpha", "Yankee", "Beta", "Gamma"]);
        let mut participants: std::collections::BTreeMap<String, String> =
            std::collections::BTreeMap::new();
        participants.insert("xray".to_string(), "Ann".to_string());
        let mut aliases: std::collections::BTreeMap<String, String> =
            std::collections::BTreeMap::new();
        let migration = ob_model::migrate_serving_line_renames_detailed(
            &old_lines,
            &new_lines,
            &mut participants,
            &mut aliases,
        );
        assert!(!migration.ambiguous);
        assert!(migration.removed.is_empty());
        assert_eq!(
            unfollowed_serving_lines_sentence(&migration.unfollowed),
            "'Xray' was paired with 'Yankee' and was also removed: the \
             names went to 'Yankee', and the removal was not followed. \
             Correct the names by hand if they do not belong under \
             'Yankee'."
        );
        // No duplicates: no note is carried, and the
        // handler — which speaks only from a non-empty
        // `unfollowed` — stays silent.
        let old_lines = lines(&["Alpha", "Xray", "Beta"]);
        let new_lines = lines(&["Alpha", "Yankee", "Beta"]);
        let mut participants: std::collections::BTreeMap<String, String> =
            std::collections::BTreeMap::new();
        participants.insert("xray".to_string(), "Ann".to_string());
        let mut aliases: std::collections::BTreeMap<String, String> =
            std::collections::BTreeMap::new();
        let migration = ob_model::migrate_serving_line_renames_detailed(
            &old_lines,
            &new_lines,
            &mut participants,
            &mut aliases,
        );
        assert!(migration.unfollowed.is_empty());
        assert_eq!(unfollowed_serving_lines_sentence(&migration.unfollowed), "");
    }

    #[test]
    fn r13c_same_target_pairings_speak_no_note_and_events_follow_segment_order() {
        // Round 13 (R13-7, review R13-6 + red team
        // RT13-6): the same-target duplicate pairing
        // spoke a tautological false alarm ("paired
        // with 'Yankee' and with 'Yankee' … Correct
        // the names by hand if they belong under
        // 'Yankee'") about names that are already
        // right, and multiple unfollowed events
        // printed pairings-then-removals rather
        // than in the order the office wrote them.
        let lines =
            |names: &[&str]| -> Vec<String> { names.iter().map(|n| n.to_string()).collect() };
        // Same target twice: no event, no note.
        let old_lines = lines(&["Alpha", "Xray", "Beta", "Xray", "Gamma"]);
        let new_lines = lines(&["Alpha", "Yankee", "Beta", "Yankee", "Gamma"]);
        let mut participants: std::collections::BTreeMap<String, String> =
            std::collections::BTreeMap::new();
        participants.insert("xray".to_string(), "Ann".to_string());
        let mut aliases: std::collections::BTreeMap<String, String> =
            std::collections::BTreeMap::new();
        let migration = ob_model::migrate_serving_line_renames_detailed(
            &old_lines,
            &new_lines,
            &mut participants,
            &mut aliases,
        );
        assert!(!migration.ambiguous);
        assert!(migration.unfollowed.is_empty());
        assert_eq!(unfollowed_serving_lines_sentence(&migration.unfollowed), "");
        assert_eq!(participants.get("yankee").map(String::as_str), Some("Ann"));
        // Pair, removal, pair: the removal was
        // written before the Zulu pairing, and the
        // status words it first.
        let old_lines = lines(&["A0", "Xray", "A1", "Xray", "A2", "Xray", "A3"]);
        let new_lines = lines(&["A0", "Yankee", "A1", "A2", "Zulu", "A3"]);
        let mut participants: std::collections::BTreeMap<String, String> =
            std::collections::BTreeMap::new();
        participants.insert("xray".to_string(), "Ann".to_string());
        let mut aliases: std::collections::BTreeMap<String, String> =
            std::collections::BTreeMap::new();
        let migration = ob_model::migrate_serving_line_renames_detailed(
            &old_lines,
            &new_lines,
            &mut participants,
            &mut aliases,
        );
        assert!(!migration.ambiguous);
        assert_eq!(
            unfollowed_serving_lines_sentence(&migration.unfollowed),
            "'Xray' was paired with 'Yankee' and was also removed: the \
             names went to 'Yankee', and the removal was not followed. \
             Correct the names by hand if they do not belong under \
             'Yankee'. 'Xray' was paired with 'Yankee' and with 'Zulu': \
             the names went to 'Yankee', and the pairing with 'Zulu' was \
             not followed. Correct the names by hand if they belong \
             under 'Zulu'."
        );
    }

    #[test]
    fn ambiguous_and_refused_serving_lines_are_named_in_the_status_words() {
        // Round 5, F3 + RT5-4, words changed in round
        // 6 (R6-C): the profile save's words, pinned
        // exactly as the removed-line words are. An
        // ambiguous rename no longer keeps names under
        // old names (the round-5 "unfollowed" words
        // are gone): the whole lines change is
        // refused, the lines are left as they were,
        // and a slug refusal names both lines in the
        // clash. Round 6 (R6-11) scoped the slug
        // refusal's closing words: "Nothing else was
        // changed" overclaimed in a multi-field edit
        // pass (the profile's other fields save on
        // their own), so both pins now end with what
        // the refusal itself covers.
        // Round 7 (R7-10, FLAGGED): the sentence now
        // takes the edit kinds the pairing saw, so
        // this call passes the crossed-renames kinds
        // explicitly. The asserted words are the
        // round-6 pin, unchanged for that shape.
        let crossed_kinds = ob_model::ServingLineEditKinds {
            renamed: true,
            removed: false,
            added: false,
            moved: true,
        };
        assert_eq!(
            ambiguous_serving_lines_sentence(&crossed_kinds),
            "Serving lines not saved: several lines were renamed and moved \
             at once, and the app cannot tell which names belong to which \
             line. The lines were left as they were. Rename one line at a \
             time, then save again."
        );
        let first = "Prayer Team".to_string();
        let second = "Prayer-Team".to_string();
        let slug = "prayer_team".to_string();
        let twins = ob_model::ServingLineSlugConflict {
            first,
            second,
            slug,
            builtin: false,
            taker_is_new: false,
        };
        assert_eq!(
            serving_line_conflict_sentence(&twins),
            "Serving lines not saved: 'Prayer Team' and 'Prayer-Team' would be \
             stored as the same line, so typing names for one would change \
             the other. Rename one of them, then save again. The serving \
             lines were left as they were."
        );
        let first = "Childrens Message".to_string();
        let second = "Children's message".to_string();
        let slug = "childrens_message".to_string();
        let alias = ob_model::ServingLineSlugConflict {
            first,
            second,
            slug,
            builtin: true,
            taker_is_new: false,
        };
        assert_eq!(
            serving_line_conflict_sentence(&alias),
            "Serving lines not saved: 'Childrens Message' would take the \
             place of the built-in line 'Children's message', so the line's \
             row and the names the bulletin prints could name different \
             people. Rename 'Childrens Message', then save again. The \
             serving lines were left as they were."
        );
    }

    #[test]
    fn the_ambiguous_refusal_sentence_names_only_the_changes_the_pairing_saw() {
        // Round 7 (review m3, R7-10): between two
        // anchors, old [Choir, Bells] becomes [Choir
        // team] — one rename plus one removal, nothing
        // moved anywhere. The refusal is right (the
        // lists cannot say whether "Choir team" is
        // Choir renamed or Bells renamed), but the
        // round-6 sentence claimed the lines "were
        // renamed and moved at once". The sentence now
        // takes its account from the pairing's own
        // kinds, and its remedy covers the removal:
        // make the deletion separately.
        let old_lines: Vec<String> = ["Readers", "Choir", "Bells", "Ushers"]
            .iter()
            .map(|n| n.to_string())
            .collect();
        let new_lines: Vec<String> = ["Readers", "Choir team", "Ushers"]
            .iter()
            .map(|n| n.to_string())
            .collect();
        let mut participants: std::collections::BTreeMap<String, String> =
            std::collections::BTreeMap::new();
        let mut aliases: std::collections::BTreeMap<String, String> =
            std::collections::BTreeMap::new();
        let migration = ob_model::migrate_serving_line_renames_detailed(
            &old_lines,
            &new_lines,
            &mut participants,
            &mut aliases,
        );
        assert!(migration.ambiguous, "the refusal itself stands");
        let kinds = migration.ambiguous_kinds.expect("kinds on a refusal");
        assert_eq!(
            kinds,
            ob_model::ServingLineEditKinds {
                renamed: true,
                removed: true,
                added: false,
                moved: false,
            }
        );
        assert_eq!(
            ambiguous_serving_lines_sentence(&kinds),
            "Serving lines not saved: lines were renamed and removed at once, \
             and the app cannot tell which names belong to which line. The lines \
             were left as they were. Rename or remove one line at a time, then \
             save again."
        );
        // The shape the round-6 sentence was written
        // for — two renames crossing anchors, nothing
        // removed or added — keeps its exact words,
        // through the same pipeline.
        let old_lines = ob_model::default_serving_lines();
        let crossed: Vec<String> = [
            "Candle lighters",
            "Lectors",
            "Ushers",
            "Greeters",
            "Communion assistants",
            "Children's message",
        ]
        .iter()
        .map(|n| n.to_string())
        .collect();
        let migration = ob_model::migrate_serving_line_renames_detailed(
            &old_lines,
            &crossed,
            &mut participants,
            &mut aliases,
        );
        assert!(migration.ambiguous);
        let kinds = migration.ambiguous_kinds.expect("kinds on a refusal");
        assert_eq!(
            ambiguous_serving_lines_sentence(&kinds),
            "Serving lines not saved: several lines were renamed and moved \
             at once, and the app cannot tell which names belong to which \
             line. The lines were left as they were. Rename one line at a \
             time, then save again."
        );
        // And a rename-plus-addition refusal names
        // the addition and not a removal or a move.
        let added_lines: Vec<String> = ["Readers", "Choir team", "Bells team", "Extra", "Ushers"]
            .iter()
            .map(|n| n.to_string())
            .collect();
        let old_pair: Vec<String> = ["Readers", "Choir", "Bells", "Ushers"]
            .iter()
            .map(|n| n.to_string())
            .collect();
        let migration = ob_model::migrate_serving_line_renames_detailed(
            &old_pair,
            &added_lines,
            &mut participants,
            &mut aliases,
        );
        assert!(migration.ambiguous);
        let kinds = migration.ambiguous_kinds.expect("kinds on a refusal");
        assert_eq!(
            kinds,
            ob_model::ServingLineEditKinds {
                renamed: true,
                removed: false,
                added: true,
                moved: false,
            }
        );
        assert_eq!(
            ambiguous_serving_lines_sentence(&kinds),
            "Serving lines not saved: lines were renamed and added at once, \
             and the app cannot tell which names belong to which line. The lines \
             were left as they were. Rename or add one line at a time, then \
             save again."
        );
    }

    #[test]
    fn the_ambiguous_refusal_sentence_for_resolved_segments_names_no_move() {
        // Round 8 (R8-10): the red team's S-C shape
        // (one rename segment, one clean removal, one
        // clean addition; nothing crossed an anchor)
        // used to read "renamed, removed and moved",
        // claiming the kind that was absent and
        // omitting the addition. The verify lane's D8
        // shape (a rename segment plus a removal in
        // its own segment) claimed the same absent
        // move. Both sentences now name only what the
        // pairing saw.
        let mut participants: std::collections::BTreeMap<String, String> =
            std::collections::BTreeMap::new();
        let mut aliases: std::collections::BTreeMap<String, String> =
            std::collections::BTreeMap::new();
        let lines =
            |names: &[&str]| -> Vec<String> { names.iter().map(|n| n.to_string()).collect() };
        let sc_old = lines(&["Alpha", "Old One", "Old Two", "Beta", "Gone Line", "Gamma"]);
        let sc_new = lines(&["Alpha", "New One", "Beta", "Gamma", "Fresh Line"]);
        let migration = ob_model::migrate_serving_line_renames_detailed(
            &sc_old,
            &sc_new,
            &mut participants,
            &mut aliases,
        );
        assert!(migration.ambiguous);
        let kinds = migration.ambiguous_kinds.expect("kinds on a refusal");
        // FLAGGED, round 10 (R10-8): this sentence
        // used to read "renamed, removed and added".
        // The deficit segment's unpaired old name can
        // answer the trailing addition across the
        // anchor — the mirror of the surplus reading
        // the kinds have always given — so the
        // account now names the move and not the
        // addition; the clean removal in its own
        // segment still stands. (The D8 sentence
        // below is unchanged: its shape has no such
        // partner anywhere, and names no move.)
        assert_eq!(
            ambiguous_serving_lines_sentence(&kinds),
            "Serving lines not saved: lines were renamed, removed and moved at once, \
             and the app cannot tell which names belong to which line. The lines \
             were left as they were. Rename or remove one line at a time, then \
             save again."
        );
        let d8_old = lines(&["Alpha", "Choir", "Bells", "Beta", "Solo"]);
        let d8_new = lines(&["Alpha", "Choir team", "Beta"]);
        let migration = ob_model::migrate_serving_line_renames_detailed(
            &d8_old,
            &d8_new,
            &mut participants,
            &mut aliases,
        );
        assert!(migration.ambiguous);
        let kinds = migration.ambiguous_kinds.expect("kinds on a refusal");
        assert_eq!(
            ambiguous_serving_lines_sentence(&kinds),
            "Serving lines not saved: lines were renamed and removed at once, \
             and the app cannot tell which names belong to which line. The lines \
             were left as they were. Rename or remove one line at a time, then \
             save again."
        );
    }

    #[test]
    fn r10_the_mirror_geometries_read_the_same_sentence() {
        // Round 10 (R10-8): the trailing mirrors —
        // surplus plus a trailing removal-only
        // segment, deficit plus a trailing
        // addition-only segment — and the
        // middle-segment witness all carry the
        // crossed kinds, so all three read the
        // canonical crossed-renames sentence, word
        // for word. Before this round the deficit
        // mirror read "renamed, removed and added"
        // for the same geometry.
        let mut participants: std::collections::BTreeMap<String, String> =
            std::collections::BTreeMap::new();
        let mut aliases: std::collections::BTreeMap<String, String> =
            std::collections::BTreeMap::new();
        let lines =
            |names: &[&str]| -> Vec<String> { names.iter().map(|n| n.to_string()).collect() };
        let canonical = "Serving lines not saved: several lines were renamed and moved \
             at once, and the app cannot tell which names belong to which \
             line. The lines were left as they were. Rename one line at a \
             time, then save again.";
        for (old, new) in [
            (
                lines(&["Alpha", "X1", "Beta", "Gone"]),
                lines(&["Alpha", "Y1", "Y2", "Beta"]),
            ),
            (
                lines(&["Alpha", "X1", "X2", "Beta"]),
                lines(&["Alpha", "Y1", "Beta", "Fresh"]),
            ),
            (
                lines(&["Alpha", "X1", "Beta", "Gone", "Gamma"]),
                lines(&["Alpha", "Y1", "Y2", "Beta", "Gamma"]),
            ),
        ] {
            let migration = ob_model::migrate_serving_line_renames_detailed(
                &old,
                &new,
                &mut participants,
                &mut aliases,
            );
            assert!(migration.ambiguous);
            let kinds = migration.ambiguous_kinds.expect("kinds on a refusal");
            assert_eq!(ambiguous_serving_lines_sentence(&kinds), canonical);
        }
    }

    #[test]
    fn the_slug_conflict_sentence_names_the_taker_and_the_holder() {
        // Round 8 (R8-11): the sentence built from a
        // built-in conflict names the taking line
        // distinguishably and the holding side by the
        // name the bulletin currently prints for the
        // claimed line — never the same unqualified
        // name on both sides. "Kids message" claims
        // the children's-message slug here through
        // the recorded alias.
        let mut aliases: std::collections::BTreeMap<String, String> =
            std::collections::BTreeMap::new();
        aliases.insert("childrens_message".to_string(), "kids_message".to_string());
        let lines =
            |names: &[&str]| -> Vec<String> { names.iter().map(|n| n.to_string()).collect() };
        // S-A2: Choir's line takes the name; the
        // holder stands as "Kids message".
        let old_lines = lines(&["Readers", "Kids message", "Choir", "Ushers"]);
        let new_lines = lines(&["Readers", "Story time", "Children's message", "Ushers"]);
        let conflict = ob_model::serving_line_slug_conflict(&old_lines, &new_lines, &aliases)
            .expect("choir taking the claimed name conflicts");
        assert_eq!(
            serving_line_conflict_sentence(&conflict),
            "Serving lines not saved: 'Choir' would take the place of the \
             built-in line 'Kids message', so the line's row and the names \
             the bulletin prints could name different people. Rename \
             'Choir', then save again. The serving lines were left as \
             they were."
        );
        // S-B: a new line takes the name the claimant
        // renames back to; the sentence says a new
        // line, and does not accuse the rename-back
        // line of conflicting with itself.
        let old_lines = lines(&["Readers", "Kids message", "Ushers"]);
        let new_lines = lines(&["Readers", "Children's message", "Kids message", "Ushers"]);
        let conflict = ob_model::serving_line_slug_conflict(&old_lines, &new_lines, &aliases)
            .expect("the new line taking the claimed name conflicts");
        assert_eq!(
            serving_line_conflict_sentence(&conflict),
            "Serving lines not saved: a new line, 'Children's message', \
             would take the place of the built-in line 'Kids message', so \
             the line's row and the names the bulletin prints could name \
             different people. Rename the new line, then save again. The \
             serving lines were left as they were."
        );
    }

    #[test]
    fn a_refused_serving_save_changes_nothing_and_rows_stay_with_tokens() {
        // Round 5, RT5-4: the save handler's order,
        // in its pieces: the slug check runs first and
        // a refusal applies nothing; an accepted save
        // keeps each built-in line's This Sunday row
        // and its printed token naming the same people.
        let mut doc = ob_layout::build_template(
            ob_layout::Preset::LetterBifold,
            ob_model::ChurchProfile::default(),
            chrono::NaiveDate::from_ymd_opt(2026, 10, 18).expect("date"),
            "Sunday",
        );
        doc.set_participant_value("Children's message", "Ms. Patty");
        doc.set_participant_value("Acolytes", "Bob");
        let lines_before = doc.profile.serving_lines.clone();
        let map_before = doc.participants.clone();
        // Renaming Acolytes onto the built-in alias
        // is refused: lines and names stay as they
        // were, and the Children's row and token still
        // agree on Ms. Patty.
        let mut renamed = lines_before.clone();
        renamed[5] = "Childrens Message".to_string();
        let conflict = ob_model::serving_line_slug_conflict(
            &lines_before,
            &renamed,
            &doc.profile.serving_aliases,
        );
        assert!(conflict.is_some_and(|c| c.builtin));
        assert_eq!(doc.profile.serving_lines, lines_before);
        assert_eq!(doc.participants, map_before);
        assert_eq!(
            doc.participant_value_for_name("Children's message"),
            "Ms. Patty"
        );
        assert_eq!(
            ob_model::resolve_fields("{Sunday.childrens_message}", &doc),
            "Ms. Patty"
        );
        // Slug twins are refused the same way.
        let mut twins = lines_before.clone();
        twins.push("Prayer Team".to_string());
        twins.push("Prayer-Team".to_string());
        assert!(ob_model::serving_line_slug_conflict(
            &lines_before,
            &twins,
            &doc.profile.serving_aliases
        )
        .is_some());
        assert_eq!(doc.profile.serving_lines, lines_before);
        assert_eq!(doc.participants, map_before);
        // Lookalike but distinct slugs save fine.
        let mut lookalikes = lines_before.clone();
        lookalikes.push("Prayer Team".to_string());
        lookalikes.push("Prayer Teams".to_string());
        assert_eq!(
            ob_model::serving_line_slug_conflict(
                &lines_before,
                &lookalikes,
                &doc.profile.serving_aliases
            ),
            None
        );
        // An accepted in-place rename of the built-in
        // keeps its row and its token together.
        let mut in_place = lines_before.clone();
        in_place[4] = "Kids' message".to_string();
        assert_eq!(
            ob_model::serving_line_slug_conflict(
                &lines_before,
                &in_place,
                &doc.profile.serving_aliases
            ),
            None
        );
        let migration = ob_model::migrate_serving_line_renames_detailed(
            &lines_before,
            &in_place,
            &mut doc.participants,
            &mut doc.profile.serving_aliases,
        );
        assert!(migration.removed.is_empty());
        assert!(!migration.ambiguous);
        doc.profile.serving_lines = in_place;
        assert_eq!(doc.participant_value_for_name("Kids' message"), "Ms. Patty");
        assert_eq!(
            ob_model::resolve_fields("{Sunday.childrens_message}", &doc),
            doc.participant_value_for_name("Kids' message")
        );
    }

    #[test]
    fn start_status_speaks_the_remembered_state() {
        // J6: the state sentence is the strongest true one,
        // appended to whatever the next step is.
        let today = chrono::NaiveDate::from_ymd_opt(2026, 10, 15).expect("date");
        let mut doc = ob_layout::build_template(
            ob_layout::Preset::LetterBifold,
            ob_model::ChurchProfile::default(),
            chrono::NaiveDate::from_ymd_opt(2026, 10, 18).expect("date"),
            "Sunday",
        );
        doc.profile.name = "Grace Lutheran Church".into();
        let fp = snapshot(&doc);
        // Print copy made from exactly this document.
        let mut state = DocState {
            print_fingerprint: Some(fp.clone()),
            checked_fingerprint: None,
        };
        let (_, next) = start_status(&doc, false, today, Some(&state));
        assert_eq!(next, "Ready to print. Print copy is current.");
        // Checked clean, and untouched since.
        state.print_fingerprint = None;
        state.checked_fingerprint = Some(fp.clone());
        let (_, next) = start_status(&doc, false, today, Some(&state));
        assert_eq!(
            next,
            "Ready to print. Checked, and nothing has changed since."
        );
        // Checked once, then changed.
        state.checked_fingerprint = Some("an older snapshot".into());
        let (_, next) = start_status(&doc, false, today, Some(&state));
        assert_eq!(next, "Ready to print. Not checked since the last change.");
        // Only a stale print copy: Start stays silent about
        // state; the Print step carries that warning.
        state.checked_fingerprint = None;
        state.print_fingerprint = Some("an older snapshot".into());
        let (_, next) = start_status(&doc, false, today, Some(&state));
        assert_eq!(next, "Ready to print.");
        // No state at all: nothing appended.
        let (_, next) = start_status(&doc, false, today, None);
        assert_eq!(next, "Ready to print.");
    }

    #[test]
    fn doc_states_are_capped_and_settings_round_trip() {
        // J6: 20 files remembered, oldest dropped; the new
        // fields survive a settings save/load round trip, and
        // old settings files (without them) still load.
        let mut st = Settings::default();
        for i in 0..21 {
            st.set_doc_state(format!("bulletin-{i}.obd"), DocState::default());
        }
        assert_eq!(st.doc_states.len(), 20);
        assert!(st.doc_state("bulletin-0.obd").is_none(), "oldest dropped");
        assert!(st.doc_state("bulletin-20.obd").is_some());
        // Re-remembering a file refreshes it in place.
        st.set_doc_state(
            "bulletin-20.obd".into(),
            DocState {
                print_fingerprint: Some("fp".into()),
                checked_fingerprint: None,
            },
        );
        assert_eq!(st.doc_states.len(), 20);
        assert_eq!(
            st.doc_state("bulletin-20.obd")
                .and_then(|s| s.print_fingerprint.as_deref()),
            Some("fp")
        );
        st.last_step = 3;
        let text = serde_json::to_string(&st).unwrap();
        let back: Settings = serde_json::from_str(&text).unwrap();
        assert_eq!(back.last_step, 3);
        assert_eq!(back.doc_states.len(), 20);
        let old: Settings = serde_json::from_str("{}").unwrap();
        assert!(old.doc_states.is_empty());
        assert_eq!(old.last_step, 0);
    }

    #[test]
    fn service_lines_read_and_rewrite_the_order_of_service() {
        // J2: the template's order story carries tabbed lines;
        // the hymn numbers are their values.
        let mut doc = ob_layout::build_template(
            ob_layout::Preset::LetterBifold,
            ob_model::ChurchProfile::default(),
            chrono::NaiveDate::from_ymd_opt(2026, 10, 18).expect("date"),
            "Sunday",
        );
        let lines = service_lines(&doc);
        let praise = lines
            .iter()
            .find(|l| l.label == "Hymn of Praise")
            .expect("the praise hymn line");
        assert_eq!(praise.value, "432");
        let response = lines
            .iter()
            .find(|l| l.label == "Hymn of Response")
            .expect("the response hymn line");
        assert_eq!(response.value, "365");
        // A line with nothing after the tab has an empty value.
        let prelude = lines
            .iter()
            .find(|l| l.label == "Prelude")
            .expect("the prelude line");
        assert_eq!(prelude.value, "");

        // Rewriting a value keeps the label exactly as found.
        set_service_line(&mut doc, praise.para_index, "433");
        let lines = service_lines(&doc);
        let praise = lines
            .iter()
            .find(|l| l.label == "Hymn of Praise")
            .expect("still there");
        assert_eq!(praise.value, "433");
        let sid = service_story_id(&doc).expect("a service story");
        let story = doc.story(&sid).expect("the story");
        assert_eq!(
            story.paragraphs[praise.para_index].text(),
            "Hymn of Praise\t433"
        );

        // The value is what follows the LAST tab (leaders
        // between label and value do not confuse the read),
        // and a rewrite puts the value after the first tab.
        let idx = story.paragraphs.len();
        let mut doc2 = doc.clone();
        doc2.story_mut(&sid)
            .expect("story")
            .paragraphs
            .push(ob_model::Paragraph::new(
                "order-line",
                "Hymn of the Day\t\t77",
            ));
        let lines = service_lines(&doc2);
        let day = lines
            .iter()
            .find(|l| l.label == "Hymn of the Day")
            .expect("the pushed line");
        assert_eq!(day.para_index, idx);
        assert_eq!(day.value, "77");
        set_service_line(&mut doc2, idx, "78");
        let story = doc2.story(&sid).expect("the story");
        assert_eq!(story.paragraphs[idx].text(), "Hymn of the Day\t78");
    }

    #[test]
    fn sermon_title_round_trips_around_the_pastor_token() {
        // J2: the template's sermon line carries only the
        // {Church.pastor} token, so it has no title yet.
        let mut doc = ob_layout::build_template(
            ob_layout::Preset::LetterBifold,
            ob_model::ChurchProfile::default(),
            chrono::NaiveDate::from_ymd_opt(2026, 10, 18).expect("date"),
            "Sunday",
        );
        assert_eq!(sermon_title(&doc), "");
        let sermon_text = |doc: &Document| {
            let (sid, i) = sermon_para(doc).expect("a sermon line");
            doc.story(&sid).expect("story").paragraphs[i].text()
        };
        // Setting a title keeps the token after it.
        set_sermon_title(&mut doc, "Grace and Truth");
        assert_eq!(sermon_text(&doc), "Sermon: Grace and Truth {Church.pastor}");
        assert_eq!(sermon_title(&doc), "Grace and Truth");
        // Clearing it restores the bare token line exactly.
        set_sermon_title(&mut doc, "");
        assert_eq!(sermon_text(&doc), "Sermon: {Church.pastor}");
        assert_eq!(sermon_title(&doc), "");

        // A line with no token: the title is the whole rest,
        // and clearing leaves a bare "Sermon:".
        let (sid, i) = sermon_para(&doc).expect("a sermon line");
        doc.story_mut(&sid).expect("story").paragraphs[i].set_text("Sermon: Old Title");
        assert_eq!(sermon_title(&doc), "Old Title");
        set_sermon_title(&mut doc, "New Title");
        assert_eq!(sermon_text(&doc), "Sermon: New Title");
        set_sermon_title(&mut doc, "");
        assert_eq!(sermon_text(&doc), "Sermon:");
    }

    #[test]
    fn a_bulletin_without_a_service_has_no_service_lines() {
        // J2's honest empty state rests on these returning
        // nothing, not on near-misses.
        let mut doc = ob_layout::build_template(
            ob_layout::Preset::LetterBifold,
            ob_model::ChurchProfile::default(),
            chrono::NaiveDate::from_ymd_opt(2026, 10, 18).expect("date"),
            "Sunday",
        );
        let sid = service_story_id(&doc).expect("a service story");
        doc.story_mut(&sid)
            .expect("story")
            .paragraphs
            .retain(|p| !p.text().contains('\t') && !p.text().starts_with("Sermon:"));
        assert!(service_lines(&doc).is_empty());
        assert_eq!(sermon_title(&doc), "");
        assert!(sermon_para(&doc).is_none());
    }

    #[test]
    fn starter_sample_needs_no_file_and_no_church() {
        // J7: the sample is a sample only while nothing has
        // made it the church's own.
        assert!(is_starter_sample(false, ""));
        assert!(is_starter_sample(false, "  "));
        assert!(!is_starter_sample(true, ""));
        assert!(!is_starter_sample(false, "Grace Lutheran Church"));
    }

    #[test]
    fn a_held_pick_opens_on_the_confirming_click() {
        // UI audit round 2: through the old-bulletin door,
        // the unsaved-work guard used to fire after the file
        // pick and eat it, so the user browsed twice. Now the
        // pick is carried through the arm.
        let mut picked = App::default().doc;
        picked.profile.name = "Picked Church".into();
        let dir = std::env::temp_dir().join(format!("ob-door-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("dir");
        let path = dir.join("picked.obd");
        ob_format::save(&picked, &path).expect("save pick");

        let mut app = App {
            frame_snapshot: "unsaved work".into(),
            saved_snapshot: "saved".into(),
            ..Default::default()
        };
        // First call: the guard arms, nothing opens, and the
        // pick is held rather than dropped.
        app.open_old_bulletin(path.clone());
        assert!(
            app.doc.profile.name != "Picked Church",
            "the guard held the open"
        );
        assert_eq!(app.pending_path.as_deref(), Some(path.as_path()));
        // The confirming click opens that very file.
        let held = app.pending_path.take().expect("a held pick");
        app.open_old_bulletin(held);
        assert_eq!(app.doc.profile.name, "Picked Church");
        assert!(app.pending_path.is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_fresh_app_asks_for_its_own_church() {
        // UI audit round 2: the first screen used to present
        // a filled-in fictional church as the user's own,
        // address buffer and all, and Check certified it.
        // Now the identity starts empty everywhere and the
        // Start status asks for it.
        let app = App::default();
        assert!(app.doc.profile.name.is_empty());
        assert!(app.doc.profile.address.is_empty());
        assert!(app.doc.profile.service_times.is_empty());
        assert!(app.church_name.is_empty());
        assert!(app.profile_address.is_empty());
        assert!(app.profile_service_times.is_empty());
        let (_, next) = start_status(&app.doc, true, chrono::Local::now().date_naive(), None);
        assert!(next.contains("church name"), "next: {next}");
        // And the fresh date is the coming Sunday, never a
        // weekday the app happened to launch on (J7).
        let fresh = chrono::NaiveDate::parse_from_str(
            app.doc.fields.get("occasion_date").expect("a date"),
            "%Y-%m-%d",
        )
        .expect("the fresh date reads");
        assert_eq!(fresh, sunday_on_or_after(chrono::Local::now().date_naive()));
    }

    #[test]
    fn an_edited_date_applies_without_being_asked() {
        // UI audit round 2, flow blocker: a typed date used
        // to apply only on Enter or the Set date button, and
        // leaving the step threw it away silently. Now the
        // field applies whenever it holds a new date.
        assert!(date_differs("2026-10-25", Some("2026-10-18")));
        assert!(!date_differs("2026-10-18", Some("2026-10-18")));
        assert!(!date_differs(" 2026-10-18 ", Some("2026-10-18")));
        assert!(!date_differs("", Some("2026-10-18")));
        assert!(date_differs("2026-10-25", None));

        let mut app = App::default();
        let stored = app
            .doc
            .fields
            .get("occasion_date")
            .cloned()
            .expect("the template has a date");
        // The field starts holding the bulletin's own date,
        // so leaving the step untouched changes nothing.
        assert_eq!(app.date_input, stored);
        app.apply_edited_date();
        assert_eq!(app.doc.fields.get("occasion_date"), Some(&stored));
        // A typed date applies through the same path focus
        // loss and step changes use.
        app.date_input = "2030-01-06".into();
        app.apply_edited_date();
        assert_eq!(
            app.doc.fields.get("occasion_date").map(String::as_str),
            Some("2030-01-06")
        );
        assert_eq!(
            app.doc.fields.get("occasion_date_long").map(String::as_str),
            Some("Sunday, January 6, 2030")
        );
        assert!(!app.status_error, "a good date is not a failure");
        // An unreadable date refuses through the banner and
        // the bulletin keeps its date.
        app.date_input = "next Friday".into();
        app.apply_edited_date();
        assert!(app.status_error, "a bad date raises the banner");
        assert_eq!(
            app.doc.fields.get("occasion_date").map(String::as_str),
            Some("2030-01-06")
        );
    }

    #[test]
    fn print_copy_staleness_compares_fingerprints() {
        // Adopted item 5: no copy yet is never stale; the same
        // snapshot is fresh; any later change is stale.
        let mut doc = ob_layout::build_template(
            ob_layout::Preset::LetterBifold,
            ob_model::ChurchProfile::default(),
            chrono::NaiveDate::from_ymd_opt(2026, 10, 18).expect("date"),
            "Sunday",
        );
        let fp = snapshot(&doc);
        assert!(!print_copy_stale(None, &fp));
        assert!(!print_copy_stale(Some(&fp), &snapshot(&doc)));
        doc.profile.name = "Changed Church".into();
        assert!(print_copy_stale(Some(&fp), &snapshot(&doc)));
    }

    #[test]
    fn settings_json_round_trip() {
        let mut st = Settings::default();
        st.printers
            .insert("Brother".into(), printer_setup_from_answer("up", "Brother"));
        let text = serde_json::to_string(&st).unwrap();
        let back: Settings = serde_json::from_str(&text).unwrap();
        assert_eq!(
            back.printers["Brother"].mode, "long_edge",
            "printer memory survives a save/load round trip"
        );
    }

    #[test]
    fn preview_line_segments_carry_the_run_faces() {
        // The canvas preview of a paragraph with a bold word
        // carries a bold segment for that word and plain
        // segments around it (G1b, Item 2).
        let doc = Document {
            format_version: ob_model::FORMAT_VERSION,
            title: "T".into(),
            profile: ChurchProfile::default(),
            fields: Default::default(),
            styles: ob_model::default_styles(),
            stories: vec![ob_model::Story {
                id: "s".into(),
                paragraphs: vec![ob_model::Paragraph::from_runs(
                    "body",
                    vec![
                        ob_model::Run::plain("Sing "),
                        ob_model::Run {
                            text: "hymn".into(),
                            bold: Some(true),
                            italic: None,
                        },
                        ob_model::Run::plain(" 432"),
                    ],
                )],
                keep_through: None,
            }],
            panels: vec![ob_model::Panel {
                background_image: None,
                id: "p".into(),
                name: "P".into(),
                size: ob_model::Size::new(612_000, 792_000),
                frames: vec![ob_model::Frame {
                    id: "f".into(),
                    name: "Box".into(),
                    rect: ob_model::Rect {
                        x: 36_000,
                        y: 36_000,
                        w: 540_000,
                        h: 720_000,
                    },
                    kind: ob_model::FrameKind::Text,
                    story_id: Some("s".into()),
                    role: ob_model::ContentRole::Weekly,
                    picture: None,
                    music: None,
                }],
                guides: vec![],
            }],
            sheet: ob_model::SheetSpec {
                preset: "letter-portrait".into(),
                size: ob_model::Size::new(612_000, 792_000),
            },
            no_announcements: false,
            assets: Vec::new(),
            asset_bytes: Default::default(),
            occasion_time: None,
            wanted_fonts: Vec::new(),
            participants: Default::default(),
            inserts: Default::default(),
            applied_contributions: Vec::new(),
        };
        let mut shaper = ob_text::shaping::bundled_shaper();
        let threads = ob_text::threading::layout_document_threads(&mut shaper, &doc);
        let lines = panel_lines_from_threads(&doc, 0, &doc.panels[0], &threads);
        assert_eq!(lines.len(), 1, "one short paragraph, one line");
        let segs = &lines[0].segs;
        let texts: Vec<(&str, bool)> = segs.iter().map(|s| (s.text.as_str(), s.bold)).collect();
        assert!(
            texts.iter().any(|(t, b)| *t == "hymn" && *b),
            "the bold word is its own bold segment: {texts:?}"
        );
        assert!(
            texts.iter().any(|(t, b)| t.contains("Sing") && !*b),
            "the lead-in stays regular: {texts:?}"
        );
        assert!(
            texts.iter().any(|(t, b)| t.contains("432") && !*b),
            "the number stays regular: {texts:?}"
        );
        // Segments are in reading order with growing offsets.
        for w in segs.windows(2) {
            assert!(w[1].dx >= w[0].dx, "segments advance");
        }
    }

    #[test]
    fn comfort_settings_default_and_round_trip() {
        // A settings file from before A1 has neither comfort
        // field: Standard size, contrast off.
        let st: Settings = serde_json::from_str("{}").expect("old settings load");
        assert_eq!(st.ui_scale, UiScale::Standard);
        assert!(!st.high_contrast);
        // Chosen comforts come back as themselves through
        // the settings file, and the theme follows them.
        let st = Settings {
            ui_scale: UiScale::ExtraLarge,
            high_contrast: true,
            ..Default::default()
        };
        let json = serde_json::to_string(&st).expect("settings save");
        let back: Settings = serde_json::from_str(&json).expect("settings reload");
        assert_eq!(back.ui_scale, UiScale::ExtraLarge);
        assert!(back.high_contrast);
        assert_eq!(Theme::for_settings(&back), Theme::high_contrast());
        let plain: Settings = serde_json::from_str("{}").expect("old settings load");
        assert_eq!(Theme::for_settings(&plain), Theme::standard());
    }

    #[test]
    fn text_size_steps_scale_the_style() {
        let body = |s: &Settings| {
            app_style(s)
                .text_styles
                .get(&egui::TextStyle::Body)
                .map(|f| f.size)
                .unwrap_or(0.0)
        };
        let standard = Settings::default();
        let large = Settings {
            ui_scale: UiScale::Large,
            ..Default::default()
        };
        let extra = Settings {
            ui_scale: UiScale::ExtraLarge,
            ..Default::default()
        };
        assert!((body(&standard) - 15.5).abs() < 0.01);
        assert!((body(&large) - 15.5 * 1.2).abs() < 0.01);
        assert!((body(&extra) - 15.5 * 1.4).abs() < 0.01);
        // Padding grows too, a step behind the words.
        let pad = |s: &Settings| app_style(s).spacing.button_padding.y;
        assert!(pad(&extra) > pad(&large) && pad(&large) > pad(&standard));
        // The labels are the words the window shows.
        assert_eq!(UiScale::Standard.label(), "Standard");
        assert_eq!(UiScale::Large.label(), "Large");
        assert_eq!(UiScale::ExtraLarge.label(), "Extra large");
    }

    #[test]
    fn high_contrast_pairs_meet_their_ratios() {
        let t = Theme::high_contrast();
        // Words on their ground: the text pairs hold 7:1.
        assert!(contrast_ratio(t.text, t.ground) >= 7.0);
        assert!(contrast_ratio(t.problem, t.ground) >= 7.0);
        assert!(contrast_ratio(t.warn, t.ground) >= 7.0);
        assert!(contrast_ratio(t.good, t.ground) >= 7.0);
        assert!(contrast_ratio(t.info, t.ground) >= 7.0);
        // The canvas accent is a working color, held to 4.5:1.
        assert!(contrast_ratio(t.accent, t.ground) >= 4.5);
        // The measuring stick itself: black on white is 21.
        assert!((contrast_ratio(egui::Color32::BLACK, egui::Color32::WHITE) - 21.0).abs() < 0.1);
        // The standard warn tone is why the variant exists:
        // as words on white it does not meet the 7:1 bar.
        assert!(contrast_ratio(WARN, egui::Color32::WHITE) < 7.0);
    }

    // ---- G2: pictures ----

    fn picture_frame(
        w: ob_model::Mpt,
        h: ob_model::Mpt,
        fit: ob_model::PictureFit,
    ) -> (ob_model::Frame, ob_model::PicturePlacement) {
        let placement = ob_model::PicturePlacement {
            asset: "a".into(),
            fit,
            crop_x: 0.5,
            crop_y: 0.5,
        };
        (
            ob_model::Frame {
                id: "f".into(),
                name: "Picture box".into(),
                kind: ob_model::FrameKind::Picture,
                rect: ob_model::Rect { x: 0, y: 0, w, h },
                story_id: None,
                role: ob_model::ContentRole::Weekly,
                picture: Some(placement.clone()),
                music: None,
            },
            placement,
        )
    }

    #[test]
    fn picture_dpi_is_measured_at_the_shown_size() {
        // 600x400 px shown whole in a 3x2 inch box: 200 dpi,
        // no finding (above the 120 floor).
        let (frame, placement) = picture_frame(216_000, 144_000, ob_model::PictureFit::ShowWhole);
        let probe = ob_image::Probe {
            format: ob_image::PictureFormat::Png,
            width: 600,
            height: 400,
        };
        assert!(
            (picture_effective_dpi(&frame, &placement, &probe).expect("sized box") - 200.0).abs()
                < 0.5
        );
        // The same picture in a 6x4 inch box: 100 dpi, the
        // blur finding's territory.
        let (frame, placement) = picture_frame(432_000, 288_000, ob_model::PictureFit::ShowWhole);
        assert!(
            (picture_effective_dpi(&frame, &placement, &probe).expect("sized box") - 100.0).abs()
                < 0.5
        );
        // Fill: a 800x400 picture filling a 2x2 inch box
        // shows at 4x2 inches (sides trimmed): 200 dpi, not
        // the box's naive 400.
        let (frame, placement) = picture_frame(144_000, 144_000, ob_model::PictureFit::Fill);
        let probe = ob_image::Probe {
            format: ob_image::PictureFormat::Jpeg,
            width: 800,
            height: 400,
        };
        assert!(
            (picture_effective_dpi(&frame, &placement, &probe).expect("sized box") - 200.0).abs()
                < 0.5
        );
        // A box with no width shows the picture at no size:
        // no DPI at all, never an infinite one read as sharp
        // (audit Batch 3). Same for no height.
        let (frame, placement) = picture_frame(0, 144_000, ob_model::PictureFit::ShowWhole);
        assert_eq!(picture_effective_dpi(&frame, &placement, &probe), None);
        let (frame, placement) = picture_frame(216_000, 0, ob_model::PictureFit::Fill);
        assert_eq!(picture_effective_dpi(&frame, &placement, &probe), None);
    }

    #[test]
    fn loose_pictures_are_the_unplaced_ones() {
        let mut doc = Document {
            format_version: ob_model::FORMAT_VERSION,
            title: "T".into(),
            profile: ChurchProfile::default(),
            fields: Default::default(),
            styles: ob_model::default_styles(),
            stories: vec![],
            panels: vec![],
            sheet: ob_model::SheetSpec {
                preset: "letter-portrait".into(),
                size: ob_model::Size::new(612_000, 792_000),
            },
            no_announcements: false,
            assets: Vec::new(),
            asset_bytes: Default::default(),
            occasion_time: None,
            wanted_fonts: Vec::new(),
            participants: Default::default(),
            inserts: Default::default(),
            applied_contributions: Vec::new(),
        };
        let placed = doc.add_asset(
            &[1, 2, 3],
            ob_model::AssetKind::Picture,
            "image/png",
            ob_model::RightsRecord::default(),
        );
        let loose = doc.add_asset(
            &[4, 5, 6],
            ob_model::AssetKind::Picture,
            "image/png",
            ob_model::RightsRecord::default(),
        );
        doc.panels.push(ob_model::Panel {
            id: "p".into(),
            name: "Front".into(),
            size: ob_model::Size {
                w: 396_000,
                h: 612_000,
            },
            guides: vec![],
            background_image: None,
            frames: vec![ob_model::Frame {
                id: "f".into(),
                name: "Picture box".into(),
                kind: ob_model::FrameKind::Picture,
                rect: ob_model::Rect {
                    x: 0,
                    y: 0,
                    w: 72_000,
                    h: 72_000,
                },
                story_id: None,
                role: ob_model::ContentRole::Weekly,
                picture: Some(ob_model::PicturePlacement::new(placed.clone())),
                music: None,
            }],
        });
        let found = unplaced_doc_pictures(&doc);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].sha256, loose);
    }

    #[test]
    fn collection_sentence_counts_every_kind() {
        let report = ob_clips::AddReport {
            added: 4,
            converted: 1,
            skipped_not_pictures: 3,
            skipped_metafiles: 2,
            skipped_convert_first: 0,
            skipped_unreadable: 1,
        };
        let s = collection_result_sentence("Easter", &report);
        assert!(s.contains("5 clips were added"), "{s}");
        assert!(
            s.contains("1 old Windows picture (WMF/EMF) was converted"),
            "{s}"
        );
        assert!(
            s.contains("3 files were not pictures and were left out"),
            "{s}"
        );
        assert!(
            s.contains("2 old Windows pictures (WMF/EMF) were left out"),
            "{s}"
        );
        assert!(s.contains("1 picture file could not be read"), "{s}");
        let none = collection_result_sentence("Empty", &ob_clips::AddReport::default());
        assert!(none.starts_with("No clips were added"), "{none}");
    }

    // ---- Roles build R-1: the contribution seam ----

    fn r1_today() -> ob_model::SimpleDate {
        ob_model::SimpleDate {
            year: 2026,
            month: 10,
            day: 16,
        }
    }

    fn r1_template() -> Document {
        ob_layout::build_template(
            ob_layout::Preset::LetterBifold,
            ob_model::ChurchProfile::default(),
            chrono::NaiveDate::from_ymd_opt(2026, 10, 18).expect("date"),
            "Sunday",
        )
    }

    #[test]
    fn stamp_verdicts_unchanged_changed_deleted() {
        let a = FileStamp {
            modified_secs: 100,
            len: 5,
            hash: "abc".to_string(),
        };
        // Same content, later touch: not a conflict.
        let touched = FileStamp {
            modified_secs: 200,
            len: 5,
            hash: "abc".to_string(),
        };
        assert_eq!(stamp_verdict(&a, Some(&touched)), StampVerdict::Unchanged);
        let changed = FileStamp {
            modified_secs: 200,
            len: 6,
            hash: "abd".to_string(),
        };
        assert_eq!(stamp_verdict(&a, Some(&changed)), StampVerdict::Changed);
        assert_eq!(stamp_verdict(&a, None), StampVerdict::Deleted);

        // Against a real file: stamp, save over it
        // ourselves (re-arm), edit it as a stranger would,
        // delete it.
        let dir =
            std::env::temp_dir().join(format!("openbulletin-r1-stamp-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        let path = dir.join("bulletin.obd");
        std::fs::write(&path, b"first").expect("write");
        let s1 = stamp_of(&path).expect("stamp");
        assert_eq!(
            stamp_verdict(&s1, stamp_of(&path).as_ref()),
            StampVerdict::Unchanged
        );
        // Our own save re-arms the stamp.
        std::fs::write(&path, b"second").expect("write");
        let s2 = stamp_of(&path).expect("stamp");
        assert_eq!(
            stamp_verdict(&s2, stamp_of(&path).as_ref()),
            StampVerdict::Unchanged
        );
        // A stranger's edit against the remembered stamp stops.
        assert_eq!(
            stamp_verdict(&s1, stamp_of(&path).as_ref()),
            StampVerdict::Changed
        );
        std::fs::remove_file(&path).expect("remove");
        assert_eq!(
            stamp_verdict(&s2, stamp_of(&path).as_ref()),
            StampVerdict::Deleted
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn inbox_grouping_versions_newest_on_top() {
        let mk = |name: &str, text: &str, mtime: u64| InboxItem {
            path: std::path::PathBuf::from(name),
            file_name: name.to_string(),
            modified_secs: mtime,
            fingerprint: content_fingerprint(text.as_bytes()),
            text: text.to_string(),
            parsed: ob_extract::contribution::parse_contribution(text, r1_today()),
            filed_for: None,
        };
        let sue1 = "From: Sue\nFor Sunday: October 25, 2026\n\n## Announcement\n\nOne.\n";
        let sue2 = "From: Sue\nFor Sunday: October 25, 2026\n\n## Announcement\n\nTwo.\n";
        let sue3 = "From: Sue\nFor Sunday: November 1, 2026\n\n## Announcement\n\nThree.\n";
        let items = vec![
            mk("a.md", sue1, 100),
            mk("b.md", sue2, 300),
            mk("c.md", sue3, 200),
            mk("d.md", "## Announcement\n", 400),
        ];
        // The unreadable file (headings, no words) is not
        // grouped and not hidden: the scan keeps it, Err
        // and all.
        assert!(items[3].parsed.is_err());
        let groups = group_inbox(&items);
        assert_eq!(groups.len(), 2);
        let g = groups
            .iter()
            .find(|g| {
                g.for_date
                    == Some(ob_model::SimpleDate {
                        year: 2026,
                        month: 10,
                        day: 25,
                    })
            })
            .expect("the Oct 25 group");
        assert_eq!(g.items.len(), 2);
        assert_eq!(items[g.items[0]].file_name, "b.md", "newest on top");
        // Same words, same fingerprint; changed words, a
        // different one.
        assert_eq!(
            content_fingerprint(b"same words"),
            content_fingerprint(b"same words")
        );
        assert_ne!(
            content_fingerprint(b"same words"),
            content_fingerprint(b"other words")
        );
        assert_eq!(content_fingerprint(b"x").len(), 8);
    }

    #[test]
    fn inbox_scan_reads_files_and_moves_nothing() {
        let dir =
            std::env::temp_dir().join(format!("openbulletin-r1-inbox-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        std::fs::write(
            dir.join("sue.md"),
            "From: Sue\n\n## Announcement\n\nThe bake sale is Friday.\n",
        )
        .expect("write");
        std::fs::write(dir.join("empty.md"), "## Youth\n").expect("write");
        std::fs::write(dir.join("notes.txt"), "not a part").expect("write");
        let items = scan_contribution_folder(&dir, r1_today(), false);
        assert_eq!(items.len(), 2, "only the .md files are parts");
        assert_eq!(
            items.iter().filter(|i| i.parsed.is_ok()).count(),
            1,
            "the wordless file stays listed as unreadable"
        );
        assert!(dir.join("sue.md").exists(), "reading never moves a file");
        assert!(dir.join("empty.md").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn r14b_inbox_scan_anchors_mixed_relatives_at_the_file_mtime() {
        // Round 14 (R14-6): the scan threads each
        // file's modification date into its parse, so
        // a mixed line in a file dated Friday,
        // October 16, 2026 — scanned a week late,
        // today October 23 — resolves its relative
        // phrase as written: "next Sunday" is
        // October 25, the framed date it restates,
        // and the line reads plainly for October 25.
        let dir =
            std::env::temp_dir().join(format!("openbulletin-r14b-inbox-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        let path = dir.join("sue.md");
        std::fs::write(
            &path,
            "From: Sue\nFor Sunday: next Sunday and October 25, 2026\n\n## Announcement\n\nHi.\n",
        )
        .expect("write");
        let noon = chrono::NaiveDate::from_ymd_opt(2026, 10, 16)
            .expect("date")
            .and_hms_opt(12, 0, 0)
            .expect("noon");
        let stamp = <chrono::Local as chrono::TimeZone>::from_local_datetime(&chrono::Local, &noon)
            .single()
            .expect("local noon is unambiguous")
            .timestamp();
        let file = std::fs::File::options()
            .write(true)
            .open(&path)
            .expect("open written file");
        file.set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(stamp as u64))
            .expect("set the file's mtime to October 16");
        let late = ob_model::SimpleDate {
            year: 2026,
            month: 10,
            day: 23,
        };
        let items = scan_contribution_folder(&dir, late, false);
        assert_eq!(items.len(), 1, "the one part is listed");
        let parsed = items[0].parsed.as_ref().expect("the part parses");
        assert_eq!(
            parsed.for_date,
            Some(ob_model::SimpleDate {
                year: 2026,
                month: 10,
                day: 25,
            }),
            "the file's date anchors the mixed line, not scan day"
        );
        assert_eq!(
            parsed.for_sunday_second_dates,
            Vec::<ob_model::SimpleDate>::new(),
            "the signals agree at the file's date — nothing is noted"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn inbox_scan_lists_an_oversize_part_unreadable_and_reads_the_rest() {
        // Round 3, RT3-E: a part file past 512 KB is
        // listed with the plain reason and never parsed,
        // while an ordinary part beside it reads as
        // usual. Neither file is moved.
        let dir =
            std::env::temp_dir().join(format!("openbulletin-r3-inbox-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        std::fs::write(dir.join("big.md"), "x".repeat(512 * 1024 + 1)).expect("write big");
        std::fs::write(
            dir.join("sue.md"),
            "From: Sue\n\n## Announcement\n\nThe bake sale is Friday.\n",
        )
        .expect("write");
        let items = scan_contribution_folder(&dir, r1_today(), false);
        assert_eq!(items.len(), 2, "both files are listed");
        let big = items
            .iter()
            .find(|i| i.file_name == "big.md")
            .expect("the oversize file is listed");
        match &big.parsed {
            Err(reason) => assert_eq!(reason, "This file is too large to read as a part."),
            Ok(_) => panic!("the oversize file must not be parsed"),
        }
        assert!(big.text.is_empty(), "none of it was read in");
        assert!(
            items
                .iter()
                .find(|i| i.file_name == "sue.md")
                .expect("the small file is listed")
                .parsed
                .is_ok(),
            "the ordinary part still parses"
        );
        assert!(dir.join("big.md").exists(), "reading never moves a file");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn still_waiting_counts_only_this_sundays_arrivals() {
        let parse =
            |t: &str| ob_extract::contribution::parse_contribution(t, r1_today()).expect("parses");
        let arrived = vec![
            parse("From: Pastor Lee\nFor Sunday: October 18, 2026\n\n## For the service\n\nSermon: **X**\n"),
            parse("From: Music director\nFor Sunday: October 25, 2026\n\n## Announcement\n\nLater.\n"),
            parse("From: Sunday school\n\n## Sunday school\n\nClasses at 9.\n"),
        ];
        let bulletin = Some(ob_model::SimpleDate {
            year: 2026,
            month: 10,
            day: 18,
        });
        // Round 3 (R3-10): the expected name is the
        // sender's name as written, in full. This test
        // used to expect "Pastor" to be satisfied by a
        // part From "Pastor Lee" through a substring
        // match; that match is exactly what R3-10
        // removed, so the fixture now names him in full.
        let expected = vec![
            "Pastor Lee".to_string(),
            "Music director".to_string(),
            "Sunday school".to_string(),
        ];
        // The pastor's part names this Sunday: arrived. The
        // music part names a different Sunday: does not
        // count. The undated Sunday school part counts.
        assert_eq!(
            still_waiting(&expected, &arrived, &[], bulletin),
            vec!["Music director".to_string()]
        );
        let text = reminder_text(&still_waiting(&expected, &arrived, &[], bulletin), bulletin);
        assert!(
            text.contains(
                "the bulletin for Sunday, October 18, 2026 is still waiting for Music director."
            ),
            "{text}"
        );
        assert!(
            text.contains("Whenever it is ready, send it over"),
            "{text}"
        );
        // Nothing expected, nothing waited for.
        assert!(still_waiting(&[], &arrived, &[], bulletin).is_empty());
    }

    #[test]
    fn r3_still_waiting_needs_the_same_name_not_a_near_one() {
        // Round 3 (R3-10): the old substring match let
        // Leanne's part satisfy an expected "Lee", so the
        // office stopped waiting for Lee's words while
        // they had never come.
        let parse =
            |t: &str| ob_extract::contribution::parse_contribution(t, r1_today()).expect("parses");
        let bulletin = Some(ob_model::SimpleDate {
            year: 2026,
            month: 10,
            day: 25,
        });
        let expected = vec!["Lee".to_string()];
        let leanne = vec![parse("From: Leanne\n\n## Announcement\n\nWords.\n")];
        assert_eq!(
            still_waiting(&expected, &leanne, &[], bulletin),
            vec!["Lee".to_string()],
            "a near name is not the name"
        );
        // The same name, in another case, counts.
        let lee = vec![parse("From: lee\n\n## Announcement\n\nWords.\n")];
        assert!(still_waiting(&expected, &lee, &[], bulletin).is_empty());
    }

    #[test]
    fn r3_an_undated_filed_part_counts_only_for_its_filing_sunday() {
        // Round 3 (R3-10): an undated part, once filed,
        // belongs to the Sunday it was filed for. Filed
        // last month, it cannot silence this week's
        // reminder; the inbox copy of the same words
        // still counts, because it is waiting to be
        // reviewed for the open Sunday.
        let parse =
            |t: &str| ob_extract::contribution::parse_contribution(t, r1_today()).expect("parses");
        let undated = parse("From: Sue\n\n## Announcement\n\nWords.\n");
        let expected = vec!["Sue".to_string()];
        let day = |month: u32, day: u32| ob_model::SimpleDate {
            year: 2026,
            month,
            day,
        };
        let bulletin = Some(day(10, 25));
        let filed_last_month = vec![(&undated, Some(day(9, 27)))];
        assert_eq!(
            still_waiting(&expected, &[], &filed_last_month, bulletin),
            vec!["Sue".to_string()]
        );
        let filed_this_week = vec![(&undated, Some(day(10, 25)))];
        assert!(still_waiting(&expected, &[], &filed_this_week, bulletin).is_empty());
        assert!(
            still_waiting(&expected, std::slice::from_ref(&undated), &[], bulletin).is_empty(),
            "the undated inbox part still counts"
        );
        // The name round trip: the Sunday
        // file_contribution writes into a filed name is
        // the Sunday the filed scan reads back out.
        let name = filed_file_name(Some("Sue"), Some(day(10, 25)), "abcd1234");
        assert_eq!(filed_sunday_from_name(&name), Some(day(10, 25)));
        let nameless = filed_file_name(Some("Sue"), None, "abcd1234");
        assert_eq!(filed_sunday_from_name(&nameless), None);
    }

    #[test]
    fn r16b_a_filed_part_counts_for_its_filename_sunday_not_its_reparse() {
        // Round 16 (R16-6): for a filed copy the
        // filename's Sunday is the record. A re-parse
        // of the filed words can read a different
        // Sunday than the one the part was filed for
        // (its relatives anchor at the file's date,
        // bounded by scan day), and that re-read may
        // not move the part to another Sunday.
        let parse =
            |t: &str| ob_extract::contribution::parse_contribution(t, r1_today()).expect("parses");
        let day = |month: u32, day: u32| ob_model::SimpleDate {
            year: 2026,
            month,
            day,
        };
        let expected = vec!["Sue".to_string()];
        // The re-parse reads November 22; the name
        // says the part was filed for October 25.
        let flipped =
            parse("From: Sue\nFor Sunday: November 22, 2026\n\n## Announcement\n\nWords.\n");
        assert_eq!(flipped.for_date, Some(day(11, 22)));
        let filed = vec![(&flipped, Some(day(10, 25)))];
        assert!(
            still_waiting(&expected, &[], &filed, Some(day(10, 25))).is_empty(),
            "the filed part counts for its filename Sunday"
        );
        assert_eq!(
            still_waiting(&expected, &[], &filed, Some(day(11, 22))),
            vec!["Sue".to_string()],
            "and not for the Sunday its re-parse reads"
        );
        assert!(
            still_waiting(&expected, &[], &filed, None).is_empty(),
            "with no bulletin open, a filed part counts as before"
        );
        // An ordinary filed copy, whose re-parse agrees
        // with its name, behaves exactly as before.
        let ordinary =
            parse("From: Sue\nFor Sunday: October 25, 2026\n\n## Announcement\n\nWords.\n");
        let filed_ordinary = vec![(&ordinary, Some(day(10, 25)))];
        assert!(still_waiting(&expected, &[], &filed_ordinary, Some(day(10, 25))).is_empty());
        assert_eq!(
            still_waiting(&expected, &[], &filed_ordinary, Some(day(11, 22))),
            vec!["Sue".to_string()]
        );
        // A filed name carrying no date (the older
        // filings) still falls back to the re-parse.
        let filed_nameless = vec![(&ordinary, None)];
        assert!(still_waiting(&expected, &[], &filed_nameless, Some(day(10, 25))).is_empty());
        assert_eq!(
            still_waiting(&expected, &[], &filed_nameless, Some(day(11, 22))),
            vec!["Sue".to_string()]
        );
        // The inbox path is unchanged: the same words,
        // unfiled, count for the Sunday they name.
        assert!(still_waiting(
            &expected,
            std::slice::from_ref(&flipped),
            &[],
            Some(day(11, 22))
        )
        .is_empty());
        assert_eq!(
            still_waiting(
                &expected,
                std::slice::from_ref(&flipped),
                &[],
                Some(day(10, 25))
            ),
            vec!["Sue".to_string()]
        );
    }

    #[test]
    fn r16b_a_filed_copy_counts_for_its_sunday_after_the_bound_crosses() {
        // Round 16 (R16-6), the review lane's
        // construction end to end: a mixed relative
        // line in a file dated Friday, October 16,
        // 2026, filed for October 25. Scanned on
        // November 14 — one day past the 28-day bound
        // (October 16 + 28 days is November 13) — the
        // re-parse falls back to the scan-day anchor
        // and reads November 22, but the filed copy
        // still counts for October 25, the Sunday in
        // its name.
        let dir =
            std::env::temp_dir().join(format!("openbulletin-r16b-filed-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        let day = |month: u32, day: u32| ob_model::SimpleDate {
            year: 2026,
            month,
            day,
        };
        let name = filed_file_name(Some("Sue"), Some(day(10, 25)), "abcd1234");
        let path = dir.join(&name);
        std::fs::write(
            &path,
            "From: Sue\nFor Sunday: next Sunday and October 25, 2026\n\n## Announcement\n\nHi.\n",
        )
        .expect("write");
        let noon = chrono::NaiveDate::from_ymd_opt(2026, 10, 16)
            .expect("date")
            .and_hms_opt(12, 0, 0)
            .expect("noon");
        let stamp = <chrono::Local as chrono::TimeZone>::from_local_datetime(&chrono::Local, &noon)
            .single()
            .expect("local noon is unambiguous")
            .timestamp();
        let file = std::fs::File::options()
            .write(true)
            .open(&path)
            .expect("open written file");
        file.set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(stamp as u64))
            .expect("set the file's mtime to October 16");
        let late = day(11, 14);
        let items = scan_contribution_folder(&dir, late, true);
        assert_eq!(items.len(), 1, "the one filed part is listed");
        assert_eq!(
            items[0].filed_for,
            Some(day(10, 25)),
            "the scan reads the Sunday back out of the filed name"
        );
        let parsed = items[0].parsed.as_ref().expect("the part parses");
        assert_eq!(
            parsed.for_date,
            Some(day(11, 22)),
            "past the bound, the re-parse reads November 22"
        );
        let expected = vec!["Sue".to_string()];
        let filed = vec![(parsed, items[0].filed_for)];
        assert!(
            still_waiting(&expected, &[], &filed, Some(day(10, 25))).is_empty(),
            "the filed copy still counts for October 25"
        );
        assert_eq!(
            still_waiting(&expected, &[], &filed, Some(day(11, 22))),
            vec!["Sue".to_string()],
            "and not for the Sunday its re-parse flipped to"
        );
        // The same folder scanned as the inbox (no
        // filed Sunday) keeps the standing parse
        // rules: the words count for November 22.
        let inbox_items = scan_contribution_folder(&dir, late, false);
        assert_eq!(inbox_items.len(), 1);
        assert_eq!(inbox_items[0].filed_for, None);
        let inbox_parsed = inbox_items[0].parsed.as_ref().expect("the part parses");
        assert!(
            still_waiting(
                &expected,
                std::slice::from_ref(inbox_parsed),
                &[],
                Some(day(11, 22))
            )
            .is_empty(),
            "the unfiled path is unchanged"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn r3_applied_part_fingerprint_guard() {
        // Round 3 (R3-5): the guard is the content
        // fingerprint of the words as reviewed, the same
        // fingerprint the filed name carries. Round 4
        // (R4-4): the record lives in the DOCUMENT, not
        // in a session set, so this test now reads and
        // writes the document's own list.
        let mut doc = r1_template();
        let text = "From: Sue\n\n## Announcement\n\nThe bake sale is Friday.\n";
        assert!(!part_already_applied(&doc, text));
        record_applied_fingerprint(&mut doc, text);
        assert!(part_already_applied(&doc, text));
        assert!(
            !part_already_applied(&doc, "From: Sue\n\n## Announcement\n\nChanged words.\n"),
            "different words are a different part"
        );
        // Recording the same words twice keeps one record.
        record_applied_fingerprint(&mut doc, text);
        assert_eq!(doc.applied_contributions.len(), 1);
        // The review's first line is the settled sentence,
        // pinned word for word.
        assert_eq!(
            APPLIED_PART_FIRST_LINE,
            "These words are already in the bulletin on screen. Saving the bulletin finishes this part; there is nothing more to add."
        );
    }

    #[test]
    fn r4_the_applied_record_travels_with_the_document() {
        // Round 4 (R4-4/R4-6): the record is document
        // state, so every road the document travels
        // carries it, and the roads back remove it.
        let text = "From: Sue\n\n## Announcement\n\nThe bake sale is Friday.\n";
        let c = ob_extract::contribution::parse_contribution(text, r1_today()).expect("parses");
        let mut doc = r1_template();
        // Before the apply: the state Undo would restore.
        let before = snapshot(&doc);
        let summary = apply_contribution(&mut doc, &c);
        assert!(!summary.is_empty());
        record_applied_fingerprint(&mut doc, text);
        assert!(part_already_applied(&doc, text));
        // Undo restores the earlier serialization whole
        // (that is exactly what undo() does with it):
        // the words and their record leave together, and
        // the part can be applied again.
        let restored: Document = serde_json::from_str(&before).expect("snapshot reads back");
        assert!(!part_already_applied(&restored, text));
        // The journal's road: the canonical snapshot is
        // this same serialization, so a recovered
        // document still carries the record and the part
        // cannot be applied twice after a crash.
        let recovered: Document =
            serde_json::from_str(&snapshot(&doc)).expect("snapshot reads back");
        assert!(part_already_applied(&recovered, text));
        // Next Sunday: the new week's document starts
        // unapplied, so the same part sent again applies
        // normally.
        let (next, _report) = ob_layout::next_sunday_report(&doc).expect("next week");
        assert!(next.applied_contributions.is_empty());
        assert!(!part_already_applied(&next, text));
    }

    #[test]
    fn r5_a_part_with_nowhere_to_land_is_not_recorded() {
        // Round 5 (R5-B, F1): an announcement for a
        // bulletin with no announcements box lands
        // nothing. The record must not be written, the
        // apply must name the part and the reason, and
        // the review must offer Accept again: the claim
        // is NotApplied, not "already in".
        let mut doc = r1_template();
        for panel in &mut doc.panels {
            for frame in &mut panel.frames {
                if frame.role == ob_model::ContentRole::Weekly {
                    frame.role = ob_model::ContentRole::Standing;
                }
            }
        }
        assert!(weekly_story_id(&doc).is_none());
        let text = "From: Sue\n\n## Announcement\n\nThe bake sale is Friday.\n";
        let c = ob_extract::contribution::parse_contribution(text, r1_today()).expect("parses");
        let outcome = apply_contribution_recording(&mut doc, &c, text);
        assert!(!outcome.all_landed);
        assert!(
            doc.applied_contributions.is_empty(),
            "a part that landed nothing leaves no record"
        );
        assert!(!part_already_applied(&doc, text));
        assert!(
            outcome.lines.iter().any(|s| {
                s.contains("The Announcement words are not in the bulletin")
                    && s.contains("no announcements box")
            }),
            "{:?}",
            outcome.lines
        );
        assert_eq!(applied_claim(&doc, &c, text), AppliedClaim::NotApplied);
    }

    #[test]
    fn r5_a_partly_landed_contribution_is_not_recorded() {
        // Round 5 (R5-B, F1): one part lands, another
        // has nowhere to go. The landed words stay
        // landed; the contribution is still not
        // recorded, and the naming sentence names the
        // part that did not land, not the one that did.
        let mut doc = r1_template();
        for panel in &mut doc.panels {
            for frame in &mut panel.frames {
                if frame.role == ob_model::ContentRole::Weekly {
                    frame.role = ob_model::ContentRole::Standing;
                }
            }
        }
        let text = "From: Sue\n\n## For the service\n\nThe choir warms up at 9:10 this week.\n\n## Announcement\n\nThe bake sale is Friday.\n";
        let c = ob_extract::contribution::parse_contribution(text, r1_today()).expect("parses");
        let outcome = apply_contribution_recording(&mut doc, &c, text);
        assert!(!outcome.all_landed);
        assert!(doc.applied_contributions.is_empty());
        let sid = service_story_id(&doc).expect("service story");
        assert!(
            doc.story(&sid)
                .expect("story")
                .paragraphs
                .iter()
                .any(|p| p.text() == "The choir warms up at 9:10 this week."),
            "the service words landed and stay landed"
        );
        assert!(
            outcome
                .lines
                .iter()
                .any(|s| s.contains("The Announcement words are not in the bulletin")),
            "{:?}",
            outcome.lines
        );
        assert!(
            !outcome
                .lines
                .iter()
                .any(|s| s.contains("The For the service words are not")),
            "{:?}",
            outcome.lines
        );
        assert_eq!(applied_claim(&doc, &c, text), AppliedClaim::NotApplied);
    }

    #[test]
    fn r5_a_record_whose_words_were_deleted_offers_accept_again() {
        // Round 5 (R5-B, RT5-3): apply, the save fails,
        // the office deletes the words by hand (not
        // Undo, the one road that takes the record
        // away). The record survives, but it no longer
        // swears the words are in: the claim is
        // RecordedButWordsGone, Accept is offered, and
        // applying lands the words again, exactly once.
        let text = "From: Sue\n\n## Announcement\n\nChoir rehearsal moved to Thursday.\n";
        let c = ob_extract::contribution::parse_contribution(text, r1_today()).expect("parses");
        let mut doc = r1_template();
        let outcome = apply_contribution_recording(&mut doc, &c, text);
        assert!(outcome.all_landed);
        assert!(part_already_applied(&doc, text));
        assert_eq!(applied_claim(&doc, &c, text), AppliedClaim::AlreadyIn);
        // The honest sentence, pinned word for word
        // like the first line it stands beside.
        assert_eq!(
            APPLIED_RECORD_MISSING_LINE,
            "Our note says these words were added to this bulletin before, but they are not in it now, so you can add them again."
        );
        let wid = weekly_story_id(&doc).expect("weekly story");
        doc.story_mut(&wid)
            .expect("story")
            .paragraphs
            .retain(|p| !p.text().contains("Choir rehearsal"));
        assert!(
            part_already_applied(&doc, text),
            "the record survives the hand delete"
        );
        assert_eq!(
            applied_claim(&doc, &c, text),
            AppliedClaim::RecordedButWordsGone
        );
        let outcome = apply_contribution_recording(&mut doc, &c, text);
        assert!(outcome.all_landed);
        let count = doc
            .story(&wid)
            .expect("story")
            .paragraphs
            .iter()
            .filter(|p| p.text().contains("Choir rehearsal"))
            .count();
        assert_eq!(count, 1, "the words land again, once");
        assert_eq!(applied_claim(&doc, &c, text), AppliedClaim::AlreadyIn);
    }

    #[test]
    fn r5_words_present_stays_already_in_and_never_lands_twice() {
        // Round 5 (R5-B): the ordinary road is
        // unchanged. Words present and recorded: the
        // claim stands at AlreadyIn, and any second
        // apply adds nothing.
        let text = "From: Sue\n\n## Announcement\n\nThe bake sale is Friday.\n";
        let c = ob_extract::contribution::parse_contribution(text, r1_today()).expect("parses");
        let mut doc = r1_template();
        let outcome = apply_contribution_recording(&mut doc, &c, text);
        assert!(outcome.all_landed);
        let wid = weekly_story_id(&doc).expect("weekly story");
        let count = |doc: &Document| {
            doc.story(&wid)
                .expect("story")
                .paragraphs
                .iter()
                .filter(|p| p.text() == "The bake sale is Friday.")
                .count()
        };
        assert_eq!(count(&doc), 1);
        assert_eq!(applied_claim(&doc, &c, text), AppliedClaim::AlreadyIn);
        let outcome = apply_contribution_recording(&mut doc, &c, text);
        assert!(outcome.all_landed);
        assert_eq!(count(&doc), 1, "the words are never added twice");
        assert!(
            outcome
                .lines
                .iter()
                .any(|s| s.contains("was already in the bulletin")),
            "{:?}",
            outcome.lines
        );
    }

    #[test]
    fn r5_a_resent_part_with_new_spacing_is_caught_by_its_words() {
        // Round 5 (R5-B): the same words re-sent with
        // different spacing are a different fingerprint
        // and carry no record of their own, yet the
        // review still says the words are already in:
        // presence is judged on the words, not the
        // bytes they arrived in.
        let text = "From: Sue\n\n## Announcement\n\nThe bake sale is Friday.\n";
        let c = ob_extract::contribution::parse_contribution(text, r1_today()).expect("parses");
        let mut doc = r1_template();
        apply_contribution_recording(&mut doc, &c, text);
        let resent = "From:  Sue\n\n## Announcement\n\nThe bake sale  is Friday.  \n";
        assert_ne!(
            content_fingerprint(resent.as_bytes()),
            content_fingerprint(text.as_bytes())
        );
        let c2 = ob_extract::contribution::parse_contribution(resent, r1_today()).expect("parses");
        assert!(!part_already_applied(&doc, resent));
        assert_eq!(applied_claim(&doc, &c2, resent), AppliedClaim::AlreadyIn);
    }

    #[test]
    fn r5_one_shared_line_does_not_make_a_part_already_in() {
        // Round 5 (R5-B): presence is the part's whole
        // words, not its first line. A longer part that
        // opens with a line already in the bulletin is
        // not "already in" while the rest of it is
        // missing.
        let mut doc = r1_template();
        let first = "From: Sue\n\n## Announcement\n\nThe bake sale is Friday.\n";
        let c1 = ob_extract::contribution::parse_contribution(first, r1_today()).expect("parses");
        apply_contribution_recording(&mut doc, &c1, first);
        let longer = "From: Sue\n\n## Announcement\n\nThe bake sale is Friday.\n\nBring a chair and a friend.\n";
        let c2 = ob_extract::contribution::parse_contribution(longer, r1_today()).expect("parses");
        assert!(!contribution_words_present(&doc, &c2));
        assert_eq!(applied_claim(&doc, &c2, longer), AppliedClaim::NotApplied);
    }

    #[test]
    fn r5_a_hymn_line_is_present_by_its_composed_form() {
        // Round 5 (R5-B): a music slip's hymn line is
        // composed into the bulletin's own hymn line,
        // not copied word for word, so its sent form
        // never appears in the document. Presence is
        // judged by the composed form, and the applied
        // part is AlreadyIn on reopen, not offered for
        // a second landing.
        let mut doc = r1_template();
        let text = "From: Music director\n\n## For the service\n\nOpening hymn: ELW 500\n";
        let c = ob_extract::contribution::parse_contribution(text, r1_today()).expect("parses");
        let outcome = apply_contribution_recording(&mut doc, &c, text);
        assert!(outcome.all_landed, "{:?}", outcome.lines);
        assert!(contribution_words_present(&doc, &c));
        assert_eq!(applied_claim(&doc, &c, text), AppliedClaim::AlreadyIn);
        // And a second apply adds no second hymn line.
        let sid = service_story_id(&doc).expect("service story");
        let lines_before = doc.story(&sid).expect("story").paragraphs.len();
        apply_contribution_recording(&mut doc, &c, text);
        assert_eq!(
            doc.story(&sid).expect("story").paragraphs.len(),
            lines_before
        );
    }

    #[test]
    fn r6b_a_hymn_mention_in_the_announcements_does_not_make_a_slip_present() {
        // Round 6 (R6-B, R6-3 + RT6-4): hymn presence is
        // a hymn line in the service story. An
        // announcement mentioning the same hymn is not a
        // hymn line, so the slip is not present, and
        // Accept sets the named service line.
        let mut doc = r1_template();
        let wid = weekly_story_id(&doc).expect("weekly story");
        doc.story_mut(&wid)
            .expect("story")
            .paragraphs
            .push(ob_model::Paragraph::plain(
                "body",
                "We will sing ELW 500 at the harvest supper.",
            ));
        let text = "From: Music director\n\n## For the service\n\nHymn of Praise: ELW 500\n";
        let c = ob_extract::contribution::parse_contribution(text, r1_today()).expect("parses");
        assert!(!part_words_present(&doc, &c, &c.parts[0]));
        assert_eq!(applied_claim(&doc, &c, text), AppliedClaim::NotApplied);
        let plans = plan_parts(&doc, &c);
        let service_sid = service_story_id(&doc).expect("service story");
        assert_eq!(plans[0].target, SlotTarget::Story(service_sid.clone()));
        let outcome = apply_contribution_recording(&mut doc, &c, text);
        assert!(outcome.all_landed, "{:?}", outcome.lines);
        let story = doc.story(&service_sid).expect("story");
        assert!(
            story
                .paragraphs
                .iter()
                .any(|p| p.text() == "Hymn of Praise\tELW 500"),
            "the hymn lands on the named line"
        );
        assert_eq!(applied_claim(&doc, &c, text), AppliedClaim::AlreadyIn);
    }

    #[test]
    fn r6b_a_hymn_on_a_differently_named_line_is_not_present() {
        // Round 6 (R6-B, RT6-4 B3b): the template's own
        // Hymn of Praise line stands with 432. A slip
        // naming a DIFFERENT line (Opening hymn) for 432
        // is not present on that line: it plans as an
        // ordinary landing, appends its own hymn line,
        // and only then reads as already in.
        let mut doc = r1_template();
        let text = "From: Music director\n\n## For the service\n\nOpening hymn: 432\n";
        let c = ob_extract::contribution::parse_contribution(text, r1_today()).expect("parses");
        assert!(!part_words_present(&doc, &c, &c.parts[0]));
        assert_eq!(applied_claim(&doc, &c, text), AppliedClaim::NotApplied);
        let plans = plan_parts(&doc, &c);
        assert!(
            matches!(plans[0].target, SlotTarget::Story(_)),
            "{:?}",
            plans[0].target
        );
        let outcome = apply_contribution_recording(&mut doc, &c, text);
        assert!(outcome.all_landed, "{:?}", outcome.lines);
        let sid = service_story_id(&doc).expect("service story");
        let texts: Vec<String> = doc
            .story(&sid)
            .expect("story")
            .paragraphs
            .iter()
            .map(|p| p.text())
            .collect();
        assert!(texts.iter().any(|t| t == "Hymn\t432"), "{texts:?}");
        assert!(
            texts.iter().any(|t| t == "Hymn of Praise\t432"),
            "{texts:?}"
        );
        assert_eq!(applied_claim(&doc, &c, text), AppliedClaim::AlreadyIn);
    }

    #[test]
    fn r6b_fragments_scattered_in_other_stories_do_not_make_a_part_present() {
        // Round 6 (R6-B, RT6-4 second half): every one of
        // this announcement's paragraphs stands somewhere
        // in the document — in the service story, the
        // cover story, and the back story — but none
        // stands in the weekly story the part would land
        // in. Presence is judged there, so the part is
        // NotApplied and Accept lands it as a unit.
        let mut doc = r1_template();
        let text = "From: Sue\n\n## Announcement\n\nOrder of Service\n\nWe are glad you are here.\n\nService Times\n";
        let c = ob_extract::contribution::parse_contribution(text, r1_today()).expect("parses");
        assert!(!part_words_present(&doc, &c, &c.parts[0]));
        assert_eq!(applied_claim(&doc, &c, text), AppliedClaim::NotApplied);
        let wid = weekly_story_id(&doc).expect("weekly story");
        let plans = plan_parts(&doc, &c);
        assert_eq!(plans[0].target, SlotTarget::Story(wid.clone()));
        let before = doc.story(&wid).expect("story").paragraphs.len();
        let outcome = apply_contribution_recording(&mut doc, &c, text);
        assert!(outcome.all_landed, "{:?}", outcome.lines);
        let weekly = doc.story(&wid).expect("story");
        assert_eq!(weekly.paragraphs.len(), before + 3);
        for needle in [
            "Order of Service",
            "We are glad you are here.",
            "Service Times",
        ] {
            assert_eq!(
                weekly
                    .paragraphs
                    .iter()
                    .filter(|p| p.text() == needle)
                    .count(),
                1,
                "{needle}"
            );
        }
        assert_eq!(applied_claim(&doc, &c, text), AppliedClaim::AlreadyIn);
    }

    #[test]
    fn r6b_reaccepting_a_longer_resend_lands_only_the_missing_paragraph() {
        // Round 6 (R6-B, RT6-3 B1): the shipped R5-B
        // shape. The longer re-send's claim is
        // NotApplied and Accept is offered — and the
        // paragraph already in the weekly story is
        // skipped, not printed a second time.
        let mut doc = r1_template();
        let first = "From: Sue\n\n## Announcement\n\nThe bake sale is Friday.\n";
        let c1 = ob_extract::contribution::parse_contribution(first, r1_today()).expect("parses");
        apply_contribution_recording(&mut doc, &c1, first);
        let longer = "From: Sue\n\n## Announcement\n\nThe bake sale is Friday.\n\nBring a chair and a friend.\n";
        let c2 = ob_extract::contribution::parse_contribution(longer, r1_today()).expect("parses");
        assert_eq!(applied_claim(&doc, &c2, longer), AppliedClaim::NotApplied);
        let outcome = apply_contribution_recording(&mut doc, &c2, longer);
        assert!(outcome.all_landed, "{:?}", outcome.lines);
        let wid = weekly_story_id(&doc).expect("weekly story");
        let count = |doc: &Document, needle: &str| {
            doc.story(&wid)
                .expect("story")
                .paragraphs
                .iter()
                .filter(|p| p.text() == needle)
                .count()
        };
        assert_eq!(count(&doc, "The bake sale is Friday."), 1);
        assert_eq!(count(&doc, "Bring a chair and a friend."), 1);
        assert!(
            outcome
                .lines
                .iter()
                .any(|s| s.contains("Added 1 announcement line.")),
            "{:?}",
            outcome.lines
        );
        assert!(
            outcome
                .lines
                .iter()
                .any(|s| s.contains("1 line was already in the bulletin")),
            "{:?}",
            outcome.lines
        );
        assert_eq!(applied_claim(&doc, &c2, longer), AppliedClaim::AlreadyIn);
    }

    #[test]
    fn r6b_a_one_word_changed_resend_lands_only_the_changed_paragraph() {
        // Round 6 (R6-B, RT6-3 B2): a two-paragraph part
        // is applied; the re-send changes one word in
        // paragraph two. Accept lands the changed
        // paragraph and skips the unchanged one, so
        // paragraph one never prints twice.
        let mut doc = r1_template();
        let text = "From: Sue\n\n## Announcement\n\nThe bake sale is Friday.\n\nBring a chair and a friend.\n";
        let c = ob_extract::contribution::parse_contribution(text, r1_today()).expect("parses");
        apply_contribution_recording(&mut doc, &c, text);
        let resent = "From: Sue\n\n## Announcement\n\nThe bake sale is Friday.\n\nBring a chair and a neighbor.\n";
        let c2 = ob_extract::contribution::parse_contribution(resent, r1_today()).expect("parses");
        assert_eq!(applied_claim(&doc, &c2, resent), AppliedClaim::NotApplied);
        let outcome = apply_contribution_recording(&mut doc, &c2, resent);
        assert!(outcome.all_landed, "{:?}", outcome.lines);
        let wid = weekly_story_id(&doc).expect("weekly story");
        let count = |doc: &Document, needle: &str| {
            doc.story(&wid)
                .expect("story")
                .paragraphs
                .iter()
                .filter(|p| p.text() == needle)
                .count()
        };
        assert_eq!(count(&doc, "The bake sale is Friday."), 1);
        assert_eq!(count(&doc, "Bring a chair and a friend."), 1);
        assert_eq!(count(&doc, "Bring a chair and a neighbor."), 1);
        assert!(
            outcome
                .lines
                .iter()
                .any(|s| s.contains("Added 1 announcement line.")),
            "{:?}",
            outcome.lines
        );
        assert!(
            outcome
                .lines
                .iter()
                .any(|s| s.contains("1 line was already in the bulletin")),
            "{:?}",
            outcome.lines
        );
        assert_eq!(applied_claim(&doc, &c2, resent), AppliedClaim::AlreadyIn);
    }

    #[test]
    fn r6b_a_loose_sheet_part_is_judged_by_the_record_not_by_words() {
        // Round 6 (R6-B, R6-2): a loose sheet's words
        // never enter a story, by design — its landing IS
        // its filing. A recorded contribution carrying
        // one therefore reopens AlreadyIn, and the
        // sheet's plan line never claims AlreadyThere.
        // With the story part's words deleted by hand,
        // the same contribution is RecordedButWordsGone.
        let text = "From: Sue\n\n## Announcement\n\nThe bake sale is Friday.\n\n## Insert\n\nThe prayer list.\n";
        let mut doc = r1_template();
        let c = ob_extract::contribution::parse_contribution(text, r1_today()).expect("parses");
        assert_eq!(applied_claim(&doc, &c, text), AppliedClaim::NotApplied);
        let plans = plan_parts(&doc, &c);
        assert_eq!(plans[1].target, SlotTarget::LooseSheet);
        let outcome = apply_contribution_recording(&mut doc, &c, text);
        assert!(outcome.all_landed, "{:?}", outcome.lines);
        assert!(part_already_applied(&doc, text));
        assert_eq!(applied_claim(&doc, &c, text), AppliedClaim::AlreadyIn);
        let plans = plan_parts(&doc, &c);
        assert_eq!(plans[0].target, SlotTarget::AlreadyThere);
        assert_eq!(
            plans[1].target,
            SlotTarget::LooseSheet,
            "a loose part never plans AlreadyThere"
        );
        let wid = weekly_story_id(&doc).expect("weekly story");
        doc.story_mut(&wid)
            .expect("story")
            .paragraphs
            .retain(|p| !p.text().contains("bake sale"));
        assert_eq!(
            applied_claim(&doc, &c, text),
            AppliedClaim::RecordedButWordsGone
        );
    }

    #[test]
    fn r6b_a_loose_only_contribution_is_already_in_exactly_when_recorded() {
        // Round 6 (R6-B, rule 3): with no story parts at
        // all, the record is the whole evidence: before
        // it, NotApplied; once recorded, AlreadyIn.
        let text = "From: Sue\n\n## Insert\n\nThe prayer list.\n";
        let mut doc = r1_template();
        let c = ob_extract::contribution::parse_contribution(text, r1_today()).expect("parses");
        assert_eq!(applied_claim(&doc, &c, text), AppliedClaim::NotApplied);
        let outcome = apply_contribution_recording(&mut doc, &c, text);
        assert!(outcome.all_landed, "{:?}", outcome.lines);
        assert!(part_already_applied(&doc, text));
        assert_eq!(applied_claim(&doc, &c, text), AppliedClaim::AlreadyIn);
    }

    #[test]
    fn r7b_an_under_heading_part_lands_its_words_under_its_own_heading() {
        // FLAGGED, renamed in round 7 (R7-B) from
        // `r6b_an_under_heading_part_lands_only_its_missing_paragraphs`:
        // that test pinned the round-6 whole-story scope
        // — a line standing anywhere in the weekly story
        // was skipped for an under-heading part. Round 7
        // (review M1, verify B3+S1) showed that scope
        // silently loses the part's heading, so presence
        // and landing are now scoped to the part's own
        // heading section. Here the line stands in the
        // weekly story but NOT under a Sunday school
        // heading (there is none), so the heading lands
        // and both paragraphs land under it; the standing
        // copy elsewhere stays where it was.
        let mut doc = r1_template();
        let wid = weekly_story_id(&doc).expect("weekly story");
        doc.story_mut(&wid)
            .expect("story")
            .paragraphs
            .push(ob_model::Paragraph::plain("body", "Classes meet at 9 AM."));
        let text =
            "From: Sue\n\n## Sunday school\n\nClasses meet at 9 AM.\n\nBring a friend to class.\n";
        let c = ob_extract::contribution::parse_contribution(text, r1_today()).expect("parses");
        let plans = plan_parts(&doc, &c);
        assert_eq!(
            plans[0].target,
            SlotTarget::AnnouncementsUnderHeading("Sunday school")
        );
        let outcome = apply_contribution_recording(&mut doc, &c, text);
        assert!(outcome.all_landed, "{:?}", outcome.lines);
        let weekly = doc.story(&wid).expect("story");
        let count = |needle: &str| {
            weekly
                .paragraphs
                .iter()
                .filter(|p| p.text() == needle)
                .count()
        };
        assert_eq!(count("Sunday school"), 1, "the heading lands once");
        assert_eq!(
            count("Classes meet at 9 AM."),
            2,
            "once standing elsewhere, once under its own heading"
        );
        assert_eq!(count("Bring a friend to class."), 1);
        // Both landed copies of the first line: the
        // second stands after the Sunday school heading.
        let texts: Vec<String> = weekly.paragraphs.iter().map(|p| p.text()).collect();
        let head_at = texts
            .iter()
            .position(|t| t == "Sunday school")
            .expect("heading stands");
        assert_eq!(texts[head_at + 1], "Classes meet at 9 AM.");
        assert_eq!(texts[head_at + 2], "Bring a friend to class.");
        assert!(
            outcome
                .lines
                .iter()
                .any(|s| s.contains("Added 2 lines") && s.contains("under their headings")),
            "{:?}",
            outcome.lines
        );
        assert!(
            !outcome
                .lines
                .iter()
                .any(|s| s.contains("already in the bulletin")),
            "no paragraph was already under the heading: {:?}",
            outcome.lines
        );
        assert_eq!(applied_claim(&doc, &c, text), AppliedClaim::AlreadyIn);
    }

    #[test]
    fn r7b_two_announcement_parts_sharing_a_paragraph_settle_already_in() {
        // Round 7 (R7-B, red team RT7-2 verbatim): two
        // ## Announcement parts carrying the same
        // paragraph. Accept lands it once and records
        // the contribution; on re-review the claim must
        // read AlreadyIn and every plan line must agree,
        // because one standing copy placed as the
        // landing places it satisfies each part that
        // asked for exactly that placement. Before the
        // fix the claim's checker spent the copy across
        // both parts while the plan judged each part
        // fresh, so the headline read
        // RecordedButWordsGone forever.
        let text = "From: Sue\n\n## Announcement\n\nThe bake sale is Friday.\n\n## Announcement\n\nThe bake sale is Friday.\n";
        let mut doc = r1_template();
        let c = ob_extract::contribution::parse_contribution(text, r1_today()).expect("parses");
        assert_eq!(c.parts.len(), 2);
        assert_eq!(applied_claim(&doc, &c, text), AppliedClaim::NotApplied);
        let outcome = apply_contribution_recording(&mut doc, &c, text);
        assert!(outcome.all_landed, "{:?}", outcome.lines);
        assert!(part_already_applied(&doc, text));
        let wid = weekly_story_id(&doc).expect("weekly story");
        let count = |doc: &Document| {
            doc.story(&wid)
                .expect("story")
                .paragraphs
                .iter()
                .filter(|p| p.text() == "The bake sale is Friday.")
                .count()
        };
        assert_eq!(count(&doc), 1, "the words stand once");
        assert_eq!(applied_claim(&doc, &c, text), AppliedClaim::AlreadyIn);
        let plans = plan_parts(&doc, &c);
        assert!(
            plans.iter().all(|p| p.target == SlotTarget::AlreadyThere),
            "every plan line agrees with the headline: {plans:?}"
        );
        // Re-accept changes nothing and the claim holds.
        let outcome = apply_contribution_recording(&mut doc, &c, text);
        assert!(outcome.all_landed, "{:?}", outcome.lines);
        assert_eq!(count(&doc), 1);
        assert_eq!(applied_claim(&doc, &c, text), AppliedClaim::AlreadyIn);
    }

    #[test]
    fn r7b_a_shared_line_between_two_parts_of_one_contribution_settles() {
        // Round 7 (R7-B, review M2's probe shape): the
        // first Announcement part carries two paragraphs,
        // the second carries the first one's line alone.
        // One standing copy of the shared line satisfies
        // both parts, and the contribution settles.
        let text = "From: Sue\n\n## Announcement\n\nClasses meet at 9 AM.\n\nBring a chair.\n\n## Announcement\n\nClasses meet at 9 AM.\n";
        let mut doc = r1_template();
        let c = ob_extract::contribution::parse_contribution(text, r1_today()).expect("parses");
        let outcome = apply_contribution_recording(&mut doc, &c, text);
        assert!(outcome.all_landed, "{:?}", outcome.lines);
        let wid = weekly_story_id(&doc).expect("weekly story");
        let weekly = doc.story(&wid).expect("story");
        for needle in ["Classes meet at 9 AM.", "Bring a chair."] {
            assert_eq!(
                weekly
                    .paragraphs
                    .iter()
                    .filter(|p| p.text() == needle)
                    .count(),
                1,
                "{needle} stands once"
            );
        }
        assert_eq!(applied_claim(&doc, &c, text), AppliedClaim::AlreadyIn);
        let plans = plan_parts(&doc, &c);
        assert!(
            plans.iter().all(|p| p.target == SlotTarget::AlreadyThere),
            "{plans:?}"
        );
    }

    #[test]
    fn r7b_the_same_line_under_two_headings_lands_under_both() {
        // Round 7 (R7-B, verify S1 verbatim): the same
        // line under ## Sunday school and ## Youth, both
        // sharing the weekly-story target. Both headings
        // land, each with the line under it, and the
        // contribution settles. Before the fix the plan
        // simulation landed the first part, called the
        // second AlreadyThere, and the Youth heading
        // never appeared at all.
        let text = "From: Sue\n\n## Sunday school\n\nJoint picnic on Saturday.\n\n## Youth\n\nJoint picnic on Saturday.\n";
        let mut doc = r1_template();
        let c = ob_extract::contribution::parse_contribution(text, r1_today()).expect("parses");
        let plans = plan_parts(&doc, &c);
        assert_eq!(
            plans[0].target,
            SlotTarget::AnnouncementsUnderHeading("Sunday school")
        );
        assert_eq!(
            plans[1].target,
            SlotTarget::AnnouncementsUnderHeading("Youth"),
            "the second heading's part plans its own landing: {plans:?}"
        );
        let outcome = apply_contribution_recording(&mut doc, &c, text);
        assert!(outcome.all_landed, "{:?}", outcome.lines);
        let wid = weekly_story_id(&doc).expect("weekly story");
        let weekly = doc.story(&wid).expect("story");
        let texts: Vec<String> = weekly.paragraphs.iter().map(|p| p.text()).collect();
        for heading in ["Sunday school", "Youth"] {
            assert_eq!(
                texts.iter().filter(|t| *t == heading).count(),
                1,
                "the {heading} heading stands once: {texts:?}"
            );
            let at = texts
                .iter()
                .position(|t| t == heading)
                .expect("heading stands");
            assert_eq!(
                texts[at + 1],
                "Joint picnic on Saturday.",
                "the line stands under the {heading} heading: {texts:?}"
            );
        }
        assert_eq!(
            texts
                .iter()
                .filter(|t| *t == "Joint picnic on Saturday.")
                .count(),
            2,
            "once under each heading: {texts:?}"
        );
        assert_eq!(applied_claim(&doc, &c, text), AppliedClaim::AlreadyIn);
        let plans = plan_parts(&doc, &c);
        assert!(
            plans.iter().all(|p| p.target == SlotTarget::AlreadyThere),
            "{plans:?}"
        );
    }

    #[test]
    fn r7b_a_heading_part_sharing_a_line_with_an_announcement_keeps_its_heading() {
        // Round 7 (R7-B, review M1 verbatim): an
        // ## Announcement part and a ## Sunday school
        // part carrying the same line. The announcement
        // lands in the weekly story; the Sunday school
        // heading still lands, with its own copy of the
        // line under it.
        let text = "From: Sue\n\n## Announcement\n\nClasses meet at 9 AM.\n\n## Sunday school\n\nClasses meet at 9 AM.\n";
        let mut doc = r1_template();
        let c = ob_extract::contribution::parse_contribution(text, r1_today()).expect("parses");
        let plans = plan_parts(&doc, &c);
        assert_eq!(
            plans[1].target,
            SlotTarget::AnnouncementsUnderHeading("Sunday school"),
            "{plans:?}"
        );
        let outcome = apply_contribution_recording(&mut doc, &c, text);
        assert!(outcome.all_landed, "{:?}", outcome.lines);
        let wid = weekly_story_id(&doc).expect("weekly story");
        let weekly = doc.story(&wid).expect("story");
        let texts: Vec<String> = weekly.paragraphs.iter().map(|p| p.text()).collect();
        assert_eq!(
            texts.iter().filter(|t| *t == "Sunday school").count(),
            1,
            "the Sunday school heading lands: {texts:?}"
        );
        let at = texts
            .iter()
            .position(|t| t == "Sunday school")
            .expect("heading stands");
        assert_eq!(texts[at + 1], "Classes meet at 9 AM.", "{texts:?}");
        assert_eq!(
            texts
                .iter()
                .filter(|t| *t == "Classes meet at 9 AM.")
                .count(),
            2,
            "once as the announcement, once under the heading: {texts:?}"
        );
        assert_eq!(applied_claim(&doc, &c, text), AppliedClaim::AlreadyIn);
        let plans = plan_parts(&doc, &c);
        assert!(
            plans.iter().all(|p| p.target == SlotTarget::AlreadyThere),
            "{plans:?}"
        );
    }

    #[test]
    fn r7b_a_line_already_under_its_own_heading_is_not_printed_twice() {
        // Round 7 (R7-B): the round-6 no-double-print
        // pin, in the scope it now names. A paragraph
        // already standing under its OWN heading is
        // skipped; only the missing paragraph lands,
        // inside the same section.
        let mut doc = r1_template();
        let wid = weekly_story_id(&doc).expect("weekly story");
        {
            let story = doc.story_mut(&wid).expect("story");
            story
                .paragraphs
                .push(ob_model::Paragraph::plain("subheading", "Sunday school"));
            story
                .paragraphs
                .push(ob_model::Paragraph::plain("body", "Classes meet at 9 AM."));
        }
        let text =
            "From: Sue\n\n## Sunday school\n\nClasses meet at 9 AM.\n\nBring a friend to class.\n";
        let c = ob_extract::contribution::parse_contribution(text, r1_today()).expect("parses");
        let outcome = apply_contribution_recording(&mut doc, &c, text);
        assert!(outcome.all_landed, "{:?}", outcome.lines);
        let weekly = doc.story(&wid).expect("story");
        let texts: Vec<String> = weekly.paragraphs.iter().map(|p| p.text()).collect();
        for needle in [
            "Sunday school",
            "Classes meet at 9 AM.",
            "Bring a friend to class.",
        ] {
            assert_eq!(
                texts.iter().filter(|t| *t == needle).count(),
                1,
                "{needle} stands once: {texts:?}"
            );
        }
        let at = texts
            .iter()
            .position(|t| t == "Sunday school")
            .expect("heading stands");
        assert_eq!(texts[at + 1], "Classes meet at 9 AM.", "{texts:?}");
        assert_eq!(
            texts[at + 2],
            "Bring a friend to class.",
            "the missing paragraph lands inside the section: {texts:?}"
        );
        assert!(
            outcome
                .lines
                .iter()
                .any(|s| s.contains("Added 1 line") && s.contains("under their headings")),
            "{:?}",
            outcome.lines
        );
        assert!(
            outcome
                .lines
                .iter()
                .any(|s| s.contains("1 line was already in the bulletin")),
            "{:?}",
            outcome.lines
        );
        assert_eq!(applied_claim(&doc, &c, text), AppliedClaim::AlreadyIn);
    }

    #[test]
    fn r8b_a_line_in_the_second_section_is_not_landed_again() {
        // Round 8 (R8-2, review R8-R3 verbatim): the
        // weekly story holds two "Sunday school"
        // sections. The part's standing line stands in
        // the SECOND section; the part also carries one
        // new line. Checker, plan, and landing share one
        // section judgment: only the new line lands, into
        // the FIRST section; the standing line stands
        // once; the claim settles.
        let mut doc = r1_template();
        let wid = weekly_story_id(&doc).expect("weekly story");
        {
            let story = doc.story_mut(&wid).expect("story");
            story
                .paragraphs
                .push(ob_model::Paragraph::plain("subheading", "Sunday school"));
            story
                .paragraphs
                .push(ob_model::Paragraph::plain("body", "Other line."));
            story
                .paragraphs
                .push(ob_model::Paragraph::plain("subheading", "Sunday school"));
            story
                .paragraphs
                .push(ob_model::Paragraph::plain("body", "Classes meet at 9 AM."));
            story
                .paragraphs
                .push(ob_model::Paragraph::plain("body", "Bring a friend."));
        }
        let text = "From: Sue\n\n## Sunday school\n\nClasses meet at 9 AM.\n\nBring a chair.\n";
        let c = ob_extract::contribution::parse_contribution(text, r1_today()).expect("parses");
        let plans = plan_parts(&doc, &c);
        assert_eq!(
            plans[0].target,
            SlotTarget::AnnouncementsUnderHeading("Sunday school"),
            "the new line is missing, so the part plans its landing: {plans:?}"
        );
        let outcome = apply_contribution_recording(&mut doc, &c, text);
        assert!(outcome.all_landed, "{:?}", outcome.lines);
        let weekly = doc.story(&wid).expect("story");
        let texts: Vec<String> = weekly.paragraphs.iter().map(|p| p.text()).collect();
        let count = |needle: &str| texts.iter().filter(|t| *t == needle).count();
        assert_eq!(count("Classes meet at 9 AM."), 1, "stands once: {texts:?}");
        assert_eq!(count("Bring a chair."), 1, "lands once: {texts:?}");
        let first_head = texts
            .iter()
            .position(|t| t == "Sunday school")
            .expect("first heading stands");
        let second_head = texts
            .iter()
            .skip(first_head + 1)
            .position(|t| t == "Sunday school")
            .map(|i| i + first_head + 1)
            .expect("second heading stands");
        let chair = texts
            .iter()
            .position(|t| t == "Bring a chair.")
            .expect("the new line landed");
        assert!(
            first_head < chair && chair < second_head,
            "the new line lands in the first section: {texts:?}"
        );
        assert!(
            outcome
                .lines
                .iter()
                .any(|s| s.contains("Added 1 line") && s.contains("under their headings")),
            "{:?}",
            outcome.lines
        );
        assert!(
            outcome
                .lines
                .iter()
                .any(|s| s.contains("1 line was already in the bulletin")),
            "{:?}",
            outcome.lines
        );
        assert_eq!(applied_claim(&doc, &c, text), AppliedClaim::AlreadyIn);
        // Re-apply changes nothing.
        let before = texts.clone();
        let outcome = apply_contribution_recording(&mut doc, &c, text);
        assert!(outcome.all_landed, "{:?}", outcome.lines);
        let texts: Vec<String> = doc
            .story(&wid)
            .expect("story")
            .paragraphs
            .iter()
            .map(|p| p.text())
            .collect();
        assert_eq!(texts, before);
    }

    #[test]
    fn r8b_an_empty_first_section_takes_the_missing_line() {
        // Round 8 (R8-2, red team surface 2, the
        // empty-first-section shape): the heading is
        // immediately repeated, so the first section is
        // empty and the standing line stands under the
        // second heading. The standing line is not
        // landed again; the missing line lands in the
        // first section.
        let mut doc = r1_template();
        let wid = weekly_story_id(&doc).expect("weekly story");
        {
            let story = doc.story_mut(&wid).expect("story");
            story
                .paragraphs
                .push(ob_model::Paragraph::plain("subheading", "Sunday school"));
            story
                .paragraphs
                .push(ob_model::Paragraph::plain("subheading", "Sunday school"));
            story
                .paragraphs
                .push(ob_model::Paragraph::plain("body", "Classes meet at 9 AM."));
        }
        let text = "From: Sue\n\n## Sunday school\n\nClasses meet at 9 AM.\n\nBring a chair.\n";
        let c = ob_extract::contribution::parse_contribution(text, r1_today()).expect("parses");
        let outcome = apply_contribution_recording(&mut doc, &c, text);
        assert!(outcome.all_landed, "{:?}", outcome.lines);
        let weekly = doc.story(&wid).expect("story");
        let texts: Vec<String> = weekly.paragraphs.iter().map(|p| p.text()).collect();
        assert_eq!(
            texts
                .iter()
                .filter(|t| *t == "Classes meet at 9 AM.")
                .count(),
            1,
            "the standing line stands once: {texts:?}"
        );
        let first_head = texts
            .iter()
            .position(|t| t == "Sunday school")
            .expect("first heading stands");
        assert_eq!(
            texts[first_head + 1],
            "Bring a chair.",
            "the missing line lands in the empty first section: {texts:?}"
        );
        assert_eq!(applied_claim(&doc, &c, text), AppliedClaim::AlreadyIn);
    }

    #[test]
    fn r8b_the_whole_part_in_the_second_section_is_already_there() {
        // Round 8 (R8-2, the masked shape): the part's
        // whole words stand in the second of two sections
        // under its heading. It stays AlreadyThere and
        // nothing lands.
        let mut doc = r1_template();
        let wid = weekly_story_id(&doc).expect("weekly story");
        {
            let story = doc.story_mut(&wid).expect("story");
            story
                .paragraphs
                .push(ob_model::Paragraph::plain("subheading", "Sunday school"));
            story
                .paragraphs
                .push(ob_model::Paragraph::plain("body", "Other line."));
            story
                .paragraphs
                .push(ob_model::Paragraph::plain("subheading", "Sunday school"));
            story
                .paragraphs
                .push(ob_model::Paragraph::plain("body", "Classes meet at 9 AM."));
            story
                .paragraphs
                .push(ob_model::Paragraph::plain("body", "Bring a friend."));
        }
        let text = "From: Sue\n\n## Sunday school\n\nClasses meet at 9 AM.\n\nBring a friend.\n";
        let c = ob_extract::contribution::parse_contribution(text, r1_today()).expect("parses");
        assert!(part_words_present(&doc, &c, &c.parts[0]));
        let plans = plan_parts(&doc, &c);
        assert_eq!(plans[0].target, SlotTarget::AlreadyThere, "{plans:?}");
        let before: Vec<String> = doc
            .story(&wid)
            .expect("story")
            .paragraphs
            .iter()
            .map(|p| p.text())
            .collect();
        let outcome = apply_contribution_recording(&mut doc, &c, text);
        assert!(outcome.all_landed, "{:?}", outcome.lines);
        let after: Vec<String> = doc
            .story(&wid)
            .expect("story")
            .paragraphs
            .iter()
            .map(|p| p.text())
            .collect();
        assert_eq!(after, before, "nothing lands");
        assert_eq!(applied_claim(&doc, &c, text), AppliedClaim::AlreadyIn);
    }

    #[test]
    fn r8b_words_split_across_two_sections_of_one_heading_are_present() {
        // Round 8 (R8-2): one section judgment. Each of
        // the part's paragraphs stands under its heading,
        // in a different one of the heading's two
        // sections. The whole part is present — every
        // paragraph stands under the heading, none only
        // outside it — so nothing lands again.
        let mut doc = r1_template();
        let wid = weekly_story_id(&doc).expect("weekly story");
        {
            let story = doc.story_mut(&wid).expect("story");
            story
                .paragraphs
                .push(ob_model::Paragraph::plain("subheading", "Sunday school"));
            story
                .paragraphs
                .push(ob_model::Paragraph::plain("body", "Classes meet at 9 AM."));
            story
                .paragraphs
                .push(ob_model::Paragraph::plain("subheading", "Sunday school"));
            story
                .paragraphs
                .push(ob_model::Paragraph::plain("body", "Bring a friend."));
        }
        let text = "From: Sue\n\n## Sunday school\n\nClasses meet at 9 AM.\n\nBring a friend.\n";
        let c = ob_extract::contribution::parse_contribution(text, r1_today()).expect("parses");
        assert!(part_words_present(&doc, &c, &c.parts[0]));
        let plans = plan_parts(&doc, &c);
        assert_eq!(plans[0].target, SlotTarget::AlreadyThere, "{plans:?}");
        assert_eq!(applied_claim(&doc, &c, text), AppliedClaim::AlreadyIn);
    }

    #[test]
    fn r8b_a_bold_emphasis_line_does_not_end_a_section() {
        // Round 8 (R8-7, review R8-R4 verbatim): a bold
        // emphasis line inside the section is content,
        // not a heading — its words are no heading the
        // app is judging. The part's words standing after
        // it are present; nothing lands, nothing moves.
        let mut doc = r1_template();
        let wid = weekly_story_id(&doc).expect("weekly story");
        {
            let story = doc.story_mut(&wid).expect("story");
            story
                .paragraphs
                .push(ob_model::Paragraph::plain("subheading", "Sunday school"));
            let mut bold = ob_model::Paragraph::plain("body", "Bring money!");
            for r in &mut bold.runs {
                r.bold = Some(true);
            }
            story.paragraphs.push(bold);
            story
                .paragraphs
                .push(ob_model::Paragraph::plain("body", "Classes meet at 9 AM."));
            story
                .paragraphs
                .push(ob_model::Paragraph::plain("body", "Bring a friend."));
        }
        let text = "From: Sue\n\n## Sunday school\n\nClasses meet at 9 AM.\n\nBring a friend.\n";
        let c = ob_extract::contribution::parse_contribution(text, r1_today()).expect("parses");
        assert!(part_words_present(&doc, &c, &c.parts[0]));
        let plans = plan_parts(&doc, &c);
        assert_eq!(plans[0].target, SlotTarget::AlreadyThere, "{plans:?}");
        let before: Vec<String> = doc
            .story(&wid)
            .expect("story")
            .paragraphs
            .iter()
            .map(|p| p.text())
            .collect();
        let outcome = apply_contribution_recording(&mut doc, &c, text);
        assert!(outcome.all_landed, "{:?}", outcome.lines);
        let after: Vec<String> = doc
            .story(&wid)
            .expect("story")
            .paragraphs
            .iter()
            .map(|p| p.text())
            .collect();
        assert_eq!(after, before, "nothing lands, nothing moves");
        assert_eq!(applied_claim(&doc, &c, text), AppliedClaim::AlreadyIn);
    }

    #[test]
    fn r8b_a_bold_paragraph_carrying_another_parts_heading_still_delimits() {
        // Round 8 (R8-7, the converse): a bold-fallback
        // paragraph whose words ARE another part heading
        // of the contribution still starts that part's
        // section. Here both headings stand only as bold
        // body paragraphs: "Bring a friend." stands under
        // the Youth heading, so the Sunday school part
        // carrying it is not present under its own
        // heading and lands its own copy there, while
        // the Youth part reads the standing copy as its
        // own.
        let bold_para = |text: &str| {
            let mut p = ob_model::Paragraph::plain("body", text);
            for r in &mut p.runs {
                r.bold = Some(true);
            }
            p
        };
        let mut doc = r1_template();
        let wid = weekly_story_id(&doc).expect("weekly story");
        {
            let story = doc.story_mut(&wid).expect("story");
            story.paragraphs.push(bold_para("Sunday school"));
            story
                .paragraphs
                .push(ob_model::Paragraph::plain("body", "Classes meet at 9 AM."));
            story.paragraphs.push(bold_para("Youth"));
            story
                .paragraphs
                .push(ob_model::Paragraph::plain("body", "Bring a friend."));
        }
        let text =
            "From: Sue\n\n## Sunday school\n\nBring a friend.\n\n## Youth\n\nBring a friend.\n";
        let c = ob_extract::contribution::parse_contribution(text, r1_today()).expect("parses");
        assert_eq!(c.parts.len(), 2);
        assert!(
            !part_words_present(&doc, &c, &c.parts[0]),
            "the line under the Youth heading is not the Sunday school part's presence"
        );
        assert!(part_words_present(&doc, &c, &c.parts[1]));
        let plans = plan_parts(&doc, &c);
        assert_eq!(
            plans[0].target,
            SlotTarget::AnnouncementsUnderHeading("Sunday school"),
            "{plans:?}"
        );
        assert_eq!(plans[1].target, SlotTarget::AlreadyThere, "{plans:?}");
        let outcome = apply_contribution_recording(&mut doc, &c, text);
        assert!(outcome.all_landed, "{:?}", outcome.lines);
        let weekly = doc.story(&wid).expect("story");
        let texts: Vec<String> = weekly.paragraphs.iter().map(|p| p.text()).collect();
        assert_eq!(
            texts.iter().filter(|t| *t == "Bring a friend.").count(),
            2,
            "once under each heading: {texts:?}"
        );
        let ss = texts
            .iter()
            .position(|t| t == "Sunday school")
            .expect("the Sunday school heading stands");
        let youth = texts
            .iter()
            .position(|t| t == "Youth")
            .expect("the Youth heading stands");
        let first_copy = texts
            .iter()
            .position(|t| t == "Bring a friend.")
            .expect("a copy stands");
        assert!(
            ss < first_copy && first_copy < youth,
            "the landed copy stands under the Sunday school heading: {texts:?}"
        );
        assert_eq!(applied_claim(&doc, &c, text), AppliedClaim::AlreadyIn);
    }

    #[test]
    fn r9b_a_part_targeting_another_story_does_not_redraw_this_storys_sections() {
        // Round 9 (R9-3, red team RT9-3 verbatim A/B): the
        // weekly story holds a Sunday school section with
        // a bold age-group label "Youth" inside it, and
        // the part's second line stands after the label.
        // Control (B1): a contribution carrying ONLY the
        // Sunday school part reads the bold line as
        // content — AlreadyIn, nothing lands. B2: the
        // same story and the same part, plus a ## Youth
        // part whose target is a NAMED "youth" story
        // standing in the document — it never touches
        // the weekly story, so it must change nothing
        // for the Sunday school part. Before the fix the
        // judged set was contribution-global: the Youth
        // part's heading delimited the weekly section at
        // the bold line, the Sunday school part read
        // NotApplied, and apply landed its second line a
        // second time.
        let bold_para = |text: &str| {
            let mut p = ob_model::Paragraph::plain("body", text);
            for r in &mut p.runs {
                r.bold = Some(true);
            }
            p
        };
        let build_doc = |with_youth_story: bool| {
            let mut doc = r1_template();
            let wid = weekly_story_id(&doc).expect("weekly story");
            {
                let story = doc.story_mut(&wid).expect("story");
                story.paragraphs.clear();
                story
                    .paragraphs
                    .push(ob_model::Paragraph::plain("subheading", "Sunday school"));
                story
                    .paragraphs
                    .push(ob_model::Paragraph::plain("body", "Classes meet at 9 AM."));
                story.paragraphs.push(bold_para("Youth"));
                story.paragraphs.push(ob_model::Paragraph::plain(
                    "body",
                    "Bring a friend to class.",
                ));
            }
            if with_youth_story {
                doc.stories.push(ob_model::Story {
                    id: "youth".to_string(),
                    paragraphs: vec![ob_model::Paragraph::plain("body", "Pizza night is Friday.")],
                    keep_through: None,
                });
            }
            doc
        };
        // Control B1: the Sunday school part alone.
        let text_b1 =
            "From: Sue\n\n## Sunday school\n\nClasses meet at 9 AM.\n\nBring a friend to class.\n";
        let doc_b1 = build_doc(false);
        let wid = weekly_story_id(&doc_b1).expect("weekly story");
        let c_b1 =
            ob_extract::contribution::parse_contribution(text_b1, r1_today()).expect("parses");
        assert!(part_words_present(&doc_b1, &c_b1, &c_b1.parts[0]));
        let plans_b1 = plan_parts(&doc_b1, &c_b1);
        assert_eq!(plans_b1[0].target, SlotTarget::AlreadyThere, "{plans_b1:?}");
        assert_eq!(
            applied_claim(&doc_b1, &c_b1, text_b1),
            AppliedClaim::AlreadyIn
        );
        // B2: the same, plus the Youth part aimed at the
        // named youth story. The Sunday school part's
        // judgments are unchanged, in plan and in apply.
        let text_b2 = "From: Sue\n\n## Sunday school\n\nClasses meet at 9 AM.\n\nBring a friend to class.\n\n## Youth\n\nPizza night is Friday.\n";
        let mut doc = build_doc(true);
        let c = ob_extract::contribution::parse_contribution(text_b2, r1_today()).expect("parses");
        assert_eq!(c.parts.len(), 2);
        let judged = judged_headings(&doc, &c);
        assert_eq!(
            heading_sections(&doc, &wid, "Sunday school", &judged),
            vec![(1, 4)],
            "the bold Youth label is content here: the section runs past it"
        );
        assert!(
            part_words_present(&doc, &c, &c.parts[0]),
            "the Sunday school part is present exactly as in B1"
        );
        let plans = plan_parts(&doc, &c);
        assert_eq!(
            plans[0].target,
            SlotTarget::AlreadyThere,
            "the plan agrees with B1: {plans:?}"
        );
        assert_eq!(applied_claim(&doc, &c, text_b2), AppliedClaim::AlreadyIn);
        let outcome = apply_contribution_recording(&mut doc, &c, text_b2);
        assert!(outcome.all_landed, "{:?}", outcome.lines);
        let weekly = doc.story(&wid).expect("story");
        let texts: Vec<String> = weekly.paragraphs.iter().map(|p| p.text()).collect();
        assert_eq!(
            texts
                .iter()
                .filter(|t| *t == "Bring a friend to class.")
                .count(),
            1,
            "no landing, no duplication in the weekly story: {texts:?}"
        );
        assert_eq!(texts.len(), 4, "the weekly story is untouched: {texts:?}");
        let youth = doc.story("youth").expect("youth story");
        assert_eq!(
            youth
                .paragraphs
                .iter()
                .filter(|p| p.text() == "Pizza night is Friday.")
                .count(),
            1,
            "the Youth part's words stand once, in their own story"
        );
        assert_eq!(applied_claim(&doc, &c, text_b2), AppliedClaim::AlreadyIn);
    }

    #[test]
    fn r9b_a_bold_fallback_heading_with_a_room_suffix_counts_as_its_section() {
        // Round 9 (R9-7, red team RT9-7 D4): the suffix
        // rule applies to bold-fallback headings exactly
        // as to styled ones. The weekly story's heading
        // is a bold body paragraph "Sunday school
        // (Room 2)" — the form `heading_paragraph` itself
        // writes when a document has no subheading style.
        // Before the fix the section was not found, and
        // apply landed a new styled heading plus a second
        // copy of the standing line.
        let bold_para = |text: &str| {
            let mut p = ob_model::Paragraph::plain("body", text);
            for r in &mut p.runs {
                r.bold = Some(true);
            }
            p
        };
        let mut doc = r1_template();
        let wid = weekly_story_id(&doc).expect("weekly story");
        {
            let story = doc.story_mut(&wid).expect("story");
            story.paragraphs.push(bold_para("Sunday school (Room 2)"));
            story
                .paragraphs
                .push(ob_model::Paragraph::plain("body", "Classes meet at 9 AM."));
        }
        let text = "From: Sue\n\n## Sunday school\n\nClasses meet at 9 AM.\n\nBring a friend.\n";
        let c = ob_extract::contribution::parse_contribution(text, r1_today()).expect("parses");
        let judged = judged_headings(&doc, &c);
        assert_eq!(
            heading_sections(&doc, &wid, "Sunday school", &judged).len(),
            1,
            "the bold suffixed heading is the heading"
        );
        assert!(
            !part_words_present(&doc, &c, &c.parts[0]),
            "one line is missing"
        );
        let outcome = apply_contribution_recording(&mut doc, &c, text);
        assert!(outcome.all_landed, "{:?}", outcome.lines);
        let weekly = doc.story(&wid).expect("story");
        let texts: Vec<String> = weekly.paragraphs.iter().map(|p| p.text()).collect();
        assert_eq!(
            texts
                .iter()
                .filter(|t| t.starts_with("Sunday school"))
                .count(),
            1,
            "no new heading lands: {texts:?}"
        );
        assert_eq!(
            texts
                .iter()
                .filter(|t| *t == "Classes meet at 9 AM.")
                .count(),
            1,
            "the standing line is not duplicated: {texts:?}"
        );
        let at = texts
            .iter()
            .position(|t| t == "Sunday school (Room 2)")
            .expect("the bold heading stands");
        assert_eq!(texts[at + 1], "Classes meet at 9 AM.", "{texts:?}");
        assert_eq!(
            texts[at + 2],
            "Bring a friend.",
            "the missing line lands under the existing bold heading: {texts:?}"
        );
        assert_eq!(applied_claim(&doc, &c, text), AppliedClaim::AlreadyIn);
    }

    #[test]
    fn r9b_a_suffix_heading_with_a_trailing_period_counts_for_either_kind() {
        // Round 9 (R9-7, red team RT9-7 D6): trailing
        // punctuation on the heading paragraph — a final
        // period — does not defeat the match, for a
        // styled heading or a bold-fallback one.
        let bold_para = |text: &str| {
            let mut p = ob_model::Paragraph::plain("body", text);
            for r in &mut p.runs {
                r.bold = Some(true);
            }
            p
        };
        // Styled: "Sunday school (Room 2)."
        let mut doc = r1_template();
        let wid = weekly_story_id(&doc).expect("weekly story");
        {
            let story = doc.story_mut(&wid).expect("story");
            story.paragraphs.push(ob_model::Paragraph::plain(
                "subheading",
                "Sunday school (Room 2).",
            ));
            story
                .paragraphs
                .push(ob_model::Paragraph::plain("body", "Classes meet at 9 AM."));
            story
                .paragraphs
                .push(ob_model::Paragraph::plain("body", "Bring a friend."));
        }
        let text = "From: Sue\n\n## Sunday school\n\nClasses meet at 9 AM.\n\nBring a friend.\n";
        let c = ob_extract::contribution::parse_contribution(text, r1_today()).expect("parses");
        assert!(
            part_words_present(&doc, &c, &c.parts[0]),
            "the styled suffix heading with its period is the heading"
        );
        let plans = plan_parts(&doc, &c);
        assert_eq!(plans[0].target, SlotTarget::AlreadyThere, "{plans:?}");
        let outcome = apply_contribution_recording(&mut doc, &c, text);
        assert!(outcome.all_landed, "{:?}", outcome.lines);
        let weekly = doc.story(&wid).expect("story");
        let texts: Vec<String> = weekly.paragraphs.iter().map(|p| p.text()).collect();
        assert_eq!(
            texts
                .iter()
                .filter(|t| t.starts_with("Sunday school"))
                .count(),
            1,
            "no second heading lands: {texts:?}"
        );
        assert_eq!(
            texts
                .iter()
                .filter(|t| *t == "Classes meet at 9 AM.")
                .count(),
            1,
            "no second copy lands: {texts:?}"
        );
        assert_eq!(applied_claim(&doc, &c, text), AppliedClaim::AlreadyIn);
        // Bold fallback: "Sunday school." — the exact
        // words with a final period.
        let mut doc = r1_template();
        let wid = weekly_story_id(&doc).expect("weekly story");
        {
            let story = doc.story_mut(&wid).expect("story");
            story.paragraphs.push(bold_para("Sunday school."));
            story
                .paragraphs
                .push(ob_model::Paragraph::plain("body", "Classes meet at 9 AM."));
            story
                .paragraphs
                .push(ob_model::Paragraph::plain("body", "Bring a friend."));
        }
        assert!(
            part_words_present(&doc, &c, &c.parts[0]),
            "the bold heading with its period is the heading"
        );
        let outcome = apply_contribution_recording(&mut doc, &c, text);
        assert!(outcome.all_landed, "{:?}", outcome.lines);
        let weekly = doc.story(&wid).expect("story");
        let texts: Vec<String> = weekly.paragraphs.iter().map(|p| p.text()).collect();
        assert_eq!(
            texts
                .iter()
                .filter(|t| *t == "Classes meet at 9 AM.")
                .count(),
            1,
            "no second copy lands: {texts:?}"
        );
        assert_eq!(applied_claim(&doc, &c, text), AppliedClaim::AlreadyIn);
    }

    #[test]
    fn r9b_a_bold_longer_heading_still_does_not_count_as_the_parts_heading() {
        // Round 9 (R9-7, the other side, bold form): a
        // bold-fallback "Sunday school teachers" is a
        // longer heading, not the part's heading plus a
        // suffix — the styled form of this pin is
        // r8b_a_longer_heading_does_not_count_as_the_parts_heading.
        // The part lands its own heading as before.
        let bold_para = |text: &str| {
            let mut p = ob_model::Paragraph::plain("body", text);
            for r in &mut p.runs {
                r.bold = Some(true);
            }
            p
        };
        let mut doc = r1_template();
        let wid = weekly_story_id(&doc).expect("weekly story");
        {
            let story = doc.story_mut(&wid).expect("story");
            story.paragraphs.push(bold_para("Sunday school teachers"));
            story
                .paragraphs
                .push(ob_model::Paragraph::plain("body", "Classes meet at 9 AM."));
            story
                .paragraphs
                .push(ob_model::Paragraph::plain("body", "Bring a friend."));
        }
        let text = "From: Sue\n\n## Sunday school\n\nClasses meet at 9 AM.\n\nBring a friend.\n";
        let c = ob_extract::contribution::parse_contribution(text, r1_today()).expect("parses");
        assert!(!part_words_present(&doc, &c, &c.parts[0]));
        let outcome = apply_contribution_recording(&mut doc, &c, text);
        assert!(outcome.all_landed, "{:?}", outcome.lines);
        let weekly = doc.story(&wid).expect("story");
        let texts: Vec<String> = weekly.paragraphs.iter().map(|p| p.text()).collect();
        assert_eq!(
            texts.iter().filter(|t| *t == "Sunday school").count(),
            1,
            "the part's own heading lands: {texts:?}"
        );
        assert_eq!(applied_claim(&doc, &c, text), AppliedClaim::AlreadyIn);
    }

    #[test]
    fn r9b_a_suffixed_part_heading_plans_and_lands_under_the_base_heading() {
        // Round 9 (R9-8, red team RT9-8 witness): the
        // sender writes "## Sunday school (Room 2)". The
        // part is the Sunday school part: it plans as an
        // under-heading part for the base heading, its
        // words land under the document's standing
        // "Sunday school" heading as written there, no
        // suffixed heading is written, and the claim
        // settles.
        let mut doc = r1_template();
        let wid = weekly_story_id(&doc).expect("weekly story");
        {
            let story = doc.story_mut(&wid).expect("story");
            story
                .paragraphs
                .push(ob_model::Paragraph::plain("subheading", "Sunday school"));
            story
                .paragraphs
                .push(ob_model::Paragraph::plain("body", "Other line."));
        }
        let text = "From: Sue\n\n## Sunday school (Room 2)\n\nClasses meet at 9 AM.\n\nBring a friend to class.\n";
        let c = ob_extract::contribution::parse_contribution(text, r1_today()).expect("parses");
        assert_eq!(c.parts.len(), 1);
        assert_eq!(
            c.parts[0].kind,
            ob_extract::contribution::PartKind::SundaySchool,
            "the suffixed heading is the Sunday school part, not an announcement"
        );
        let plans = plan_parts(&doc, &c);
        assert_eq!(
            plans[0].target,
            SlotTarget::AnnouncementsUnderHeading("Sunday school"),
            "it plans as an under-heading part for the base heading: {plans:?}"
        );
        let outcome = apply_contribution_recording(&mut doc, &c, text);
        assert!(outcome.all_landed, "{:?}", outcome.lines);
        let weekly = doc.story(&wid).expect("story");
        let texts: Vec<String> = weekly.paragraphs.iter().map(|p| p.text()).collect();
        assert!(
            texts.iter().all(|t| !t.contains("(Room 2)")),
            "no suffixed heading is written to the bulletin: {texts:?}"
        );
        assert_eq!(
            texts.iter().filter(|t| *t == "Sunday school").count(),
            1,
            "the standing heading stands once: {texts:?}"
        );
        let at = texts
            .iter()
            .position(|t| t == "Sunday school")
            .expect("heading stands");
        assert_eq!(texts[at + 1], "Other line.", "{texts:?}");
        assert_eq!(
            texts[at + 2],
            "Classes meet at 9 AM.",
            "the part's words land under the standing heading: {texts:?}"
        );
        assert_eq!(texts[at + 3], "Bring a friend to class.", "{texts:?}");
        assert_eq!(applied_claim(&doc, &c, text), AppliedClaim::AlreadyIn);
    }

    #[test]
    fn r9b_a_suffixed_part_heading_with_no_standing_heading_writes_the_base_heading() {
        // Round 9 (R9-8, the other witness): a suffixed
        // part heading against a document with NO such
        // heading lands writing the BASE heading — the
        // sender's room note is the part's qualifier,
        // not a heading the bulletin prints.
        let mut doc = r1_template();
        let wid = weekly_story_id(&doc).expect("weekly story");
        let text = "From: Sue\n\n## Sunday school (Room 2)\n\nClasses meet at 9 AM.\n\nBring a friend to class.\n";
        let c = ob_extract::contribution::parse_contribution(text, r1_today()).expect("parses");
        let plans = plan_parts(&doc, &c);
        assert_eq!(
            plans[0].target,
            SlotTarget::AnnouncementsUnderHeading("Sunday school"),
            "{plans:?}"
        );
        let outcome = apply_contribution_recording(&mut doc, &c, text);
        assert!(outcome.all_landed, "{:?}", outcome.lines);
        let weekly = doc.story(&wid).expect("story");
        let texts: Vec<String> = weekly.paragraphs.iter().map(|p| p.text()).collect();
        assert!(
            texts.iter().all(|t| !t.contains("(Room 2)")),
            "the suffixed heading is never written: {texts:?}"
        );
        assert_eq!(
            texts.iter().filter(|t| *t == "Sunday school").count(),
            1,
            "the base heading is written once: {texts:?}"
        );
        let at = texts
            .iter()
            .position(|t| t == "Sunday school")
            .expect("base heading stands");
        assert_eq!(texts[at + 1], "Classes meet at 9 AM.", "{texts:?}");
        assert_eq!(texts[at + 2], "Bring a friend to class.", "{texts:?}");
        assert_eq!(applied_claim(&doc, &c, text), AppliedClaim::AlreadyIn);
    }

    #[test]
    fn r8b_a_styled_heading_with_a_room_suffix_counts_as_its_section() {
        // Round 8 (R8-8a): a styled heading "Sunday
        // school (Room 2)" is the Sunday school heading's
        // section. The part's words standing under it are
        // present; no second heading and no second copy
        // land.
        let mut doc = r1_template();
        let wid = weekly_story_id(&doc).expect("weekly story");
        {
            let story = doc.story_mut(&wid).expect("story");
            story.paragraphs.push(ob_model::Paragraph::plain(
                "subheading",
                "Sunday school (Room 2)",
            ));
            story
                .paragraphs
                .push(ob_model::Paragraph::plain("body", "Classes meet at 9 AM."));
            story
                .paragraphs
                .push(ob_model::Paragraph::plain("body", "Bring a friend."));
        }
        let text = "From: Sue\n\n## Sunday school\n\nClasses meet at 9 AM.\n\nBring a friend.\n";
        let c = ob_extract::contribution::parse_contribution(text, r1_today()).expect("parses");
        assert!(part_words_present(&doc, &c, &c.parts[0]));
        let plans = plan_parts(&doc, &c);
        assert_eq!(plans[0].target, SlotTarget::AlreadyThere, "{plans:?}");
        let outcome = apply_contribution_recording(&mut doc, &c, text);
        assert!(outcome.all_landed, "{:?}", outcome.lines);
        let weekly = doc.story(&wid).expect("story");
        let texts: Vec<String> = weekly.paragraphs.iter().map(|p| p.text()).collect();
        assert_eq!(
            texts
                .iter()
                .filter(|t| t.starts_with("Sunday school"))
                .count(),
            1,
            "no second heading lands: {texts:?}"
        );
        assert_eq!(
            texts
                .iter()
                .filter(|t| *t == "Classes meet at 9 AM.")
                .count(),
            1,
            "no second copy lands: {texts:?}"
        );
        assert_eq!(applied_claim(&doc, &c, text), AppliedClaim::AlreadyIn);
    }

    #[test]
    fn r8b_a_room_suffix_section_takes_the_missing_line() {
        // Round 8 (R8-8a): with one paragraph standing
        // under "Sunday school (Room 2)" and one missing,
        // the missing one lands inside that section — no
        // second heading is added for it.
        let mut doc = r1_template();
        let wid = weekly_story_id(&doc).expect("weekly story");
        {
            let story = doc.story_mut(&wid).expect("story");
            story.paragraphs.push(ob_model::Paragraph::plain(
                "subheading",
                "Sunday school (Room 2)",
            ));
            story
                .paragraphs
                .push(ob_model::Paragraph::plain("body", "Classes meet at 9 AM."));
        }
        let text = "From: Sue\n\n## Sunday school\n\nClasses meet at 9 AM.\n\nBring a friend.\n";
        let c = ob_extract::contribution::parse_contribution(text, r1_today()).expect("parses");
        let outcome = apply_contribution_recording(&mut doc, &c, text);
        assert!(outcome.all_landed, "{:?}", outcome.lines);
        let weekly = doc.story(&wid).expect("story");
        let texts: Vec<String> = weekly.paragraphs.iter().map(|p| p.text()).collect();
        assert_eq!(
            texts
                .iter()
                .filter(|t| t.starts_with("Sunday school"))
                .count(),
            1,
            "no second heading lands: {texts:?}"
        );
        let at = texts
            .iter()
            .position(|t| t == "Sunday school (Room 2)")
            .expect("the styled heading stands");
        assert_eq!(texts[at + 1], "Classes meet at 9 AM.", "{texts:?}");
        assert_eq!(
            texts[at + 2],
            "Bring a friend.",
            "the missing line lands inside the Room 2 section: {texts:?}"
        );
        assert_eq!(applied_claim(&doc, &c, text), AppliedClaim::AlreadyIn);
    }

    #[test]
    fn r8b_a_longer_heading_does_not_count_as_the_parts_heading() {
        // Round 8 (R8-8a, the other side): "Sunday school
        // teachers" is a longer heading, not the part's
        // heading plus a parenthesized suffix. The part
        // lands its own heading and its words under it,
        // as before.
        let mut doc = r1_template();
        let wid = weekly_story_id(&doc).expect("weekly story");
        {
            let story = doc.story_mut(&wid).expect("story");
            story.paragraphs.push(ob_model::Paragraph::plain(
                "subheading",
                "Sunday school teachers",
            ));
            story
                .paragraphs
                .push(ob_model::Paragraph::plain("body", "Classes meet at 9 AM."));
            story
                .paragraphs
                .push(ob_model::Paragraph::plain("body", "Bring a friend."));
        }
        let text = "From: Sue\n\n## Sunday school\n\nClasses meet at 9 AM.\n\nBring a friend.\n";
        let c = ob_extract::contribution::parse_contribution(text, r1_today()).expect("parses");
        assert!(!part_words_present(&doc, &c, &c.parts[0]));
        let outcome = apply_contribution_recording(&mut doc, &c, text);
        assert!(outcome.all_landed, "{:?}", outcome.lines);
        let weekly = doc.story(&wid).expect("story");
        let texts: Vec<String> = weekly.paragraphs.iter().map(|p| p.text()).collect();
        assert_eq!(
            texts.iter().filter(|t| *t == "Sunday school").count(),
            1,
            "the part's own heading lands: {texts:?}"
        );
        let at = texts
            .iter()
            .position(|t| t == "Sunday school")
            .expect("heading stands");
        assert_eq!(texts[at + 1], "Classes meet at 9 AM.", "{texts:?}");
        assert_eq!(texts[at + 2], "Bring a friend.", "{texts:?}");
        assert_eq!(
            texts
                .iter()
                .filter(|t| *t == "Classes meet at 9 AM.")
                .count(),
            2,
            "once under the teachers heading, once under its own: {texts:?}"
        );
        assert_eq!(applied_claim(&doc, &c, text), AppliedClaim::AlreadyIn);
    }

    #[test]
    fn r8b_a_subheading_style_with_a_non_literal_id_is_recognized() {
        // Round 8 (R8-8b): the document's subheading
        // style carries a non-literal id ("sub-2"), found
        // by name. The heading writer writes in that id,
        // and the recognizer reads it back — one shared
        // name-or-id recognition — so a part landing
        // under it is counted landed and settles.
        let mut doc = r1_template();
        for style in &mut doc.styles {
            if style.id == "subheading" {
                style.id = "sub-2".to_string();
            }
        }
        let head = heading_paragraph(&doc, "Sunday school");
        assert_eq!(head.style, "sub-2", "the writer finds the style by name");
        let wid = weekly_story_id(&doc).expect("weekly story");
        {
            let story = doc.story_mut(&wid).expect("story");
            story.paragraphs.push(head);
            story
                .paragraphs
                .push(ob_model::Paragraph::plain("body", "Classes meet at 9 AM."));
        }
        let text = "From: Sue\n\n## Sunday school\n\nClasses meet at 9 AM.\n\nBring a friend.\n";
        let c = ob_extract::contribution::parse_contribution(text, r1_today()).expect("parses");
        let judged = judged_headings(&doc, &c);
        assert_eq!(
            heading_sections(&doc, &wid, "Sunday school", &judged).len(),
            1,
            "the recognizer finds the section under the non-literal id"
        );
        let outcome = apply_contribution_recording(&mut doc, &c, text);
        assert!(outcome.all_landed, "{:?}", outcome.lines);
        let weekly = doc.story(&wid).expect("story");
        let texts: Vec<String> = weekly.paragraphs.iter().map(|p| p.text()).collect();
        assert_eq!(
            texts.iter().filter(|t| *t == "Sunday school").count(),
            1,
            "no second heading lands: {texts:?}"
        );
        let at = texts
            .iter()
            .position(|t| t == "Sunday school")
            .expect("heading stands");
        assert_eq!(texts[at + 1], "Classes meet at 9 AM.", "{texts:?}");
        assert_eq!(
            texts[at + 2],
            "Bring a friend.",
            "the missing paragraph lands inside the section: {texts:?}"
        );
        assert_eq!(applied_claim(&doc, &c, text), AppliedClaim::AlreadyIn);
    }

    #[test]
    fn r7b_the_same_paragraph_twice_in_one_part_still_settles() {
        // Round 7 (R7-B, control): the red team's P2
        // control keeps settling. Within ONE part the
        // checker still spends the story's paragraphs
        // one by one, so a paragraph the part carries
        // twice lands twice and reads present.
        let text = "From: Sue\n\n## Announcement\n\nThe bake sale is Friday.\n\nThe bake sale is Friday.\n";
        let mut doc = r1_template();
        let c = ob_extract::contribution::parse_contribution(text, r1_today()).expect("parses");
        assert_eq!(c.parts.len(), 1);
        let outcome = apply_contribution_recording(&mut doc, &c, text);
        assert!(outcome.all_landed, "{:?}", outcome.lines);
        let wid = weekly_story_id(&doc).expect("weekly story");
        assert_eq!(
            doc.story(&wid)
                .expect("story")
                .paragraphs
                .iter()
                .filter(|p| p.text() == "The bake sale is Friday.")
                .count(),
            2
        );
        assert_eq!(applied_claim(&doc, &c, text), AppliedClaim::AlreadyIn);
    }

    #[test]
    fn r7b_the_same_hymn_line_twice_in_one_service_part_still_settles() {
        // Round 7 (R7-B, control): the red team's P9
        // control keeps settling. Hymn presence is
        // untouched: a hymn line in the service story,
        // judged per paragraph within the part.
        let text = "From: Music director\n\n## For the service\n\nHymn of Praise\t433\n\nHymn of Praise\t433\n";
        let mut doc = r1_template();
        let c = ob_extract::contribution::parse_contribution(text, r1_today()).expect("parses");
        let outcome = apply_contribution_recording(&mut doc, &c, text);
        assert!(outcome.all_landed, "{:?}", outcome.lines);
        assert_eq!(applied_claim(&doc, &c, text), AppliedClaim::AlreadyIn);
        let sid = service_story_id(&doc).expect("service story");
        let before = doc.story(&sid).expect("story").paragraphs.len();
        let outcome = apply_contribution_recording(&mut doc, &c, text);
        assert!(outcome.all_landed, "{:?}", outcome.lines);
        assert_eq!(
            doc.story(&sid).expect("story").paragraphs.len(),
            before,
            "a second apply adds no third hymn line"
        );
        assert_eq!(applied_claim(&doc, &c, text), AppliedClaim::AlreadyIn);
    }

    #[test]
    fn r7b_words_removed_by_hand_after_a_shared_apply_still_read_words_gone() {
        // Round 7 (R7-B): the round-5 honest sentence
        // survives the unified notion. The two-part
        // contribution settles, the office deletes the
        // standing paragraph by hand, and the claim
        // reads RecordedButWordsGone again — the record
        // stands, the words do not. Re-applying lands
        // the words once and the contribution settles.
        let text = "From: Sue\n\n## Announcement\n\nThe bake sale is Friday.\n\n## Announcement\n\nThe bake sale is Friday.\n";
        let mut doc = r1_template();
        let c = ob_extract::contribution::parse_contribution(text, r1_today()).expect("parses");
        let outcome = apply_contribution_recording(&mut doc, &c, text);
        assert!(outcome.all_landed, "{:?}", outcome.lines);
        assert_eq!(applied_claim(&doc, &c, text), AppliedClaim::AlreadyIn);
        let wid = weekly_story_id(&doc).expect("weekly story");
        doc.story_mut(&wid)
            .expect("story")
            .paragraphs
            .retain(|p| !p.text().contains("bake sale"));
        assert!(part_already_applied(&doc, text), "the record survives");
        assert_eq!(
            applied_claim(&doc, &c, text),
            AppliedClaim::RecordedButWordsGone
        );
        let outcome = apply_contribution_recording(&mut doc, &c, text);
        assert!(outcome.all_landed, "{:?}", outcome.lines);
        assert_eq!(
            doc.story(&wid)
                .expect("story")
                .paragraphs
                .iter()
                .filter(|p| p.text() == "The bake sale is Friday.")
                .count(),
            1
        );
        assert_eq!(applied_claim(&doc, &c, text), AppliedClaim::AlreadyIn);
    }

    #[test]
    fn r6b_an_appended_hymn_already_on_a_hymn_line_is_not_appended_twice() {
        // Round 6 (R6-B, rule 5): the service story
        // already carries an appended "Hymn" line with
        // ELW 777. A slip carrying that hymn plus one
        // missing plain line is not wholly present, so
        // the hymn path runs — and counts the hymn as
        // already there instead of appending a second
        // Hymn line with the same value.
        let mut doc = r1_template();
        let sid = service_story_id(&doc).expect("service story");
        doc.story_mut(&sid)
            .expect("story")
            .paragraphs
            .push(ob_model::Paragraph::plain("order-line", "Hymn\tELW 777"));
        let text = "From: Music director\n\n## For the service\n\nELW 777\n\nPrelude is at 9:25 this week.\n";
        let c = ob_extract::contribution::parse_contribution(text, r1_today()).expect("parses");
        assert!(!part_words_present(&doc, &c, &c.parts[0]));
        let outcome = apply_contribution_recording(&mut doc, &c, text);
        assert!(outcome.all_landed, "{:?}", outcome.lines);
        let story = doc.story(&sid).expect("story");
        assert_eq!(
            story
                .paragraphs
                .iter()
                .filter(|p| p.text() == "Hymn\tELW 777")
                .count(),
            1
        );
        assert_eq!(
            story
                .paragraphs
                .iter()
                .filter(|p| p.text() == "Prelude is at 9:25 this week.")
                .count(),
            1
        );
        assert!(
            outcome
                .lines
                .iter()
                .any(|s| s.contains("Added 1 line to the service.")),
            "{:?}",
            outcome.lines
        );
        assert!(
            outcome
                .lines
                .iter()
                .any(|s| s.contains("1 line was already in the bulletin")),
            "{:?}",
            outcome.lines
        );
    }

    #[test]
    fn r3_announcement_without_a_box_is_kept_with_the_part() {
        // Round 3 (R3-8a): with no announcements box in
        // the bulletin, the plan says the words stay with
        // the part, and Accept places nothing.
        let mut doc = r1_template();
        for panel in &mut doc.panels {
            for frame in &mut panel.frames {
                if frame.role == ob_model::ContentRole::Weekly {
                    frame.role = ob_model::ContentRole::Standing;
                }
            }
        }
        assert!(weekly_story_id(&doc).is_none());
        let c = ob_extract::contribution::parse_contribution(
            "From: Sue\n\n## Announcement\n\nThe bake sale is Friday.\n",
            r1_today(),
        )
        .expect("parses");
        let plans = plan_parts(&doc, &c);
        assert_eq!(plans[0].target, SlotTarget::KeptWithPart);
        assert!(
            plans[0].line.contains("no announcements box"),
            "{}",
            plans[0].line
        );
        assert!(
            plans[0].line.contains("nothing is added to the pages"),
            "{}",
            plans[0].line
        );
        let words_before: usize = doc.stories.iter().map(|s| s.paragraphs.len()).sum();
        let summary = apply_contribution(&mut doc, &c);
        let words_after: usize = doc.stories.iter().map(|s| s.paragraphs.len()).sum();
        assert_eq!(words_before, words_after, "nothing was placed");
        assert!(
            summary.iter().any(|s| s.contains("had no box to go into")),
            "{summary:?}"
        );
    }

    #[test]
    fn r3_keep_through_lands_on_the_receiving_story_and_the_later_date_wins() {
        // Round 3 (R3-8b/c): the date lands on whichever
        // story received the words, and a story's own
        // later date is never shortened by an earlier
        // contribution date.
        let parse =
            |t: &str| ob_extract::contribution::parse_contribution(t, r1_today()).expect("parses");
        // A named Sunday school box receives the date.
        let mut doc = r1_template();
        doc.stories.push(ob_model::Story {
            id: "sunday-school".into(),
            paragraphs: vec![],
            keep_through: None,
        });
        let c = parse(
            "From: Sue\nKeep-through: November 8, 2026\n\n## Sunday school\n\nClasses at 9.\n",
        );
        let plans = plan_parts(&doc, &c);
        assert_eq!(
            plans[0].target,
            SlotTarget::Story("sunday-school".to_string())
        );
        assert!(
            plans[0].line.contains("Stays in through")
                && plans[0].line.contains("November 8, 2026"),
            "{}",
            plans[0].line
        );
        apply_contribution(&mut doc, &c);
        assert_eq!(
            doc.story("sunday-school")
                .expect("story")
                .keep_through
                .as_deref(),
            Some("2026-11-08")
        );
        // The service fallback heading lands in the weekly
        // story, and the date goes with it.
        let mut doc_fb = r1_template();
        for story in &mut doc_fb.stories {
            for p in &mut story.paragraphs {
                let t = p.text();
                if t.contains('\t') {
                    p.set_text(&t.replace('\t', " "));
                }
            }
        }
        assert!(service_story_id(&doc_fb).is_none());
        let wid_fb = weekly_story_id(&doc_fb).expect("weekly story");
        let c_fb = parse(
            "From: Sue\nKeep-through: November 8, 2026\n\n## For the service\n\nSermon: **Roots**\n",
        );
        let plans_fb = plan_parts(&doc_fb, &c_fb);
        assert_eq!(
            plans_fb[0].target,
            SlotTarget::AnnouncementsUnderHeading("For the service")
        );
        apply_contribution(&mut doc_fb, &c_fb);
        assert_eq!(
            doc_fb
                .story(&wid_fb)
                .expect("story")
                .keep_through
                .as_deref(),
            Some("2026-11-08")
        );
        // Later wins, both directions.
        let announcement =
            parse("From: Sue\nKeep-through: November 8, 2026\n\n## Announcement\n\nSupper.\n");
        let mut doc_late = r1_template();
        let wid_late = weekly_story_id(&doc_late).expect("weekly story");
        doc_late.story_mut(&wid_late).expect("story").keep_through = Some("2026-12-25".into());
        let plans_late = plan_parts(&doc_late, &announcement);
        assert!(
            plans_late[0].line.contains("December 25, 2026"),
            "{}",
            plans_late[0].line
        );
        apply_contribution(&mut doc_late, &announcement);
        assert_eq!(
            doc_late
                .story(&wid_late)
                .expect("story")
                .keep_through
                .as_deref(),
            Some("2026-12-25"),
            "the later date already there is not shortened"
        );
        let mut doc_early = r1_template();
        let wid_early = weekly_story_id(&doc_early).expect("weekly story");
        doc_early.story_mut(&wid_early).expect("story").keep_through = Some("2026-10-20".into());
        apply_contribution(&mut doc_early, &announcement);
        assert_eq!(
            doc_early
                .story(&wid_early)
                .expect("story")
                .keep_through
                .as_deref(),
            Some("2026-11-08"),
            "the contribution's later date extends the stay"
        );
    }

    #[test]
    fn r3_the_plan_is_simulated_so_a_second_service_part_sees_the_first() {
        // Round 3 (R3-8d): two service parts in one file.
        // The first fills the one empty hymn line; the
        // second, planned against the first's work, must
        // promise an added line, not the same fill, and
        // Accept must do what the plan promised.
        let mut doc = r1_template();
        let sid = service_story_id(&doc).expect("service story");
        {
            let story = doc.story_mut(&sid).expect("story");
            for p in story.paragraphs.iter_mut() {
                if p.text().starts_with("Hymn of Response") {
                    p.set_text("Hymn of Response\t");
                }
            }
        }
        let text = "\
From: Music director

## For the service
ELW 111

## For the service
ELW 222
";
        let c = ob_extract::contribution::parse_contribution(text, r1_today()).expect("parses");
        assert_eq!(c.parts.len(), 2, "two service parts, in order");
        let plans = plan_parts(&doc, &c);
        assert!(
            plans[0]
                .line
                .contains("Fills the Hymn of Response line with ELW 111."),
            "{}",
            plans[0].line
        );
        assert!(
            plans[1]
                .line
                .contains("Adds a hymn line at the end: ELW 222."),
            "{}",
            plans[1].line
        );
        apply_contribution(&mut doc, &c);
        let story = doc.story(&sid).expect("story");
        let texts: Vec<String> = story.paragraphs.iter().map(|p| p.text()).collect();
        assert!(
            texts.iter().any(|t| t == "Hymn of Response\tELW 111"),
            "{texts:?}"
        );
        assert!(texts.iter().any(|t| t == "Hymn\tELW 222"), "{texts:?}");
        assert!(
            !texts.iter().any(|t| t == "Hymn of Response\tELW 222"),
            "the second part did not take the filled line: {texts:?}"
        );
    }

    #[test]
    fn r3_part_lines_keep_their_middle_and_bare_numbers_stay_plain() {
        // Round 3 (R3-8e): a value is everything after
        // the FIRST tab, and a line is a hymn entry only
        // with a number and (a hymn label or a hymnal).
        let (label, value) = part_line_label("Hymn\tELW 432\tJoyful, Joyful");
        assert_eq!(label.as_deref(), Some("Hymn"));
        assert_eq!(value, "ELW 432\tJoyful, Joyful");
        let doc = r1_template();
        let part = |lines: &[&str]| ob_extract::contribution::Part {
            kind: ob_extract::contribution::PartKind::Service,
            paragraphs: lines
                .iter()
                .map(|t| ob_model::Paragraph::plain("body", t))
                .collect(),
        };
        let slots = hymn_slots(
            &doc,
            &part(&[
                "Hymn\tELW 432\tJoyful, Joyful",
                "Psalm\t23",
                "Hymn requests: see the director",
            ]),
        );
        match &slots[0] {
            HymnSlot::AppendHymn { value } => {
                assert_eq!(value, "ELW 432 \u{00b7} Joyful, Joyful")
            }
            other => panic!("the two-tab hymn line slots as a hymn entry, got {other:?}"),
        }
        assert_eq!(slots[1], HymnSlot::Plain, "a psalm number is not a hymn");
        assert_eq!(
            slots[2],
            HymnSlot::Plain,
            "prose under a hymn-flavoured label stays prose"
        );
    }

    #[test]
    fn contribution_slots_are_decided_and_said() {
        let mut doc = r1_template();
        let text = "\
From: Sue
For Sunday: October 18, 2026
Keep-through: November 8, 2026

## For the service
Hymn of Praise\t433

## Announcement
Harvest supper is Saturday.

## Sunday school
Classes meet at 9 AM.

## Insert
The prayer list.
";
        let c = ob_extract::contribution::parse_contribution(text, r1_today()).expect("parses");
        let plans = plan_parts(&doc, &c);
        assert_eq!(plans.len(), 4);
        let service_sid = service_story_id(&doc).expect("service story");
        assert_eq!(plans[0].target, SlotTarget::Story(service_sid.clone()));
        assert!(
            plans[1].line.contains("Stays in through"),
            "{}",
            plans[1].line
        );
        // No Sunday school box in the template: the words
        // ride in the announcements under their heading,
        // and the plan says so.
        assert_eq!(
            plans[2].target,
            SlotTarget::AnnouncementsUnderHeading("Sunday school")
        );
        assert_eq!(plans[3].target, SlotTarget::LooseSheet);

        // R-2a: the plan names the hymn slotting out loud.
        assert!(
            plans[0]
                .line
                .contains("The Hymn of Praise line becomes 433."),
            "{}",
            plans[0].line
        );
        let service_before = doc.story(&service_sid).expect("story").paragraphs.len();
        let summary = apply_contribution(&mut doc, &c);
        let service_after = doc.story(&service_sid).expect("story");
        // The part's hymn line names the bulletin's own
        // Hymn of Praise line, so its value is replaced
        // (432 becomes 433); no second line piles up.
        assert_eq!(service_after.paragraphs.len(), service_before);
        assert!(
            service_after
                .paragraphs
                .iter()
                .any(|p| p.text() == "Hymn of Praise\t433"),
            "the named hymn line carries the new number"
        );
        let wid = weekly_story_id(&doc).expect("weekly story");
        let weekly = doc.story(&wid).expect("story");
        assert_eq!(weekly.keep_through.as_deref(), Some("2026-11-08"));
        let texts: Vec<String> = weekly.paragraphs.iter().map(|p| p.text()).collect();
        assert!(texts.iter().any(|t| t == "Sunday school"), "{texts:?}");
        assert!(
            texts.iter().any(|t| t == "Classes meet at 9 AM."),
            "{texts:?}"
        );
        assert!(!doc.no_announcements);
        let joined = summary.join(" ");
        assert!(joined.contains("Added 1 line to the service."), "{joined}");
        assert!(
            joined.contains("The loose sheet is filed with this part and kept safe."),
            "{joined}"
        );
        // No part was an Insert-shaped paragraph in any
        // story: the loose sheet's words were not placed.
        assert!(
            !texts.iter().any(|t| t == "The prayer list."),
            "loose sheet words never land in the pages"
        );
    }

    #[test]
    fn music_slip_fills_replaces_and_appends_hymn_lines() {
        // R-2a: a service part's hymn entries slot by
        // label, fill empty hymn lines in order, and only
        // then append; other lines behave as R-1 left them.
        let mut doc = r1_template();
        let sid = service_story_id(&doc).expect("service story");
        // Empty the Hymn of Response line, so a slip entry
        // has somewhere to fill.
        {
            let story = doc.story_mut(&sid).expect("story");
            for p in story.paragraphs.iter_mut() {
                if p.text().starts_with("Hymn of Response") {
                    p.set_text("Hymn of Response\t");
                }
            }
        }
        let text = "\
From: Music director
For Sunday: October 18, 2026

## For the service
Opening hymn: ELW 500
Hymn of Praise\tELW 432 \u{00b7} O Come, All Ye Faithful
ELW 88 \u{00b7} A Title With No Book
Prelude is at 9:25 this week.
";
        let c = ob_extract::contribution::parse_contribution(text, r1_today()).expect("parses");
        let plans = plan_parts(&doc, &c);
        assert!(
            plans[0]
                .line
                .contains("Fills the Hymn of Response line with ELW 500."),
            "{}",
            plans[0].line
        );
        assert!(
            plans[0].line.contains(
                "The Hymn of Praise line becomes ELW 432 \u{00b7} O Come, All Ye Faithful."
            ),
            "{}",
            plans[0].line
        );
        assert!(
            plans[0]
                .line
                .contains("Adds a hymn line at the end: ELW 88 \u{00b7} A Title With No Book."),
            "{}",
            plans[0].line
        );
        assert!(
            plans[0].line.contains("1 more line is added at the end."),
            "{}",
            plans[0].line
        );
        let before = doc.story(&sid).expect("story").paragraphs.len();
        let summary = apply_contribution(&mut doc, &c);
        let story = doc.story(&sid).expect("story");
        let texts: Vec<String> = story.paragraphs.iter().map(|p| p.text()).collect();
        // Two lines replaced in place, two appended: the
        // leftover hymn (labelled "Hymn") and the plain
        // note, in the part's own order.
        assert_eq!(story.paragraphs.len(), before + 2, "{texts:?}");
        assert!(
            texts
                .iter()
                .any(|t| t == "Hymn of Praise\tELW 432 \u{00b7} O Come, All Ye Faithful"),
            "{texts:?}"
        );
        assert!(
            texts.iter().any(|t| t == "Hymn of Response\tELW 500"),
            "{texts:?}"
        );
        assert!(
            texts
                .iter()
                .any(|t| t == "Hymn\tELW 88 \u{00b7} A Title With No Book"),
            "{texts:?}"
        );
        assert!(
            texts.iter().any(|t| t == "Prelude is at 9:25 this week."),
            "{texts:?}"
        );
        assert!(summary.join(" ").contains("Added 4 lines to the service."));
    }

    #[test]
    fn hymn_findings_name_each_problem_once() {
        use ob_model::hymns::{HymnEntry, HymnMemory};
        let mut doc = r1_template();
        let sid = service_story_id(&doc).expect("service story");
        {
            let story = doc.story_mut(&sid).expect("story");
            for p in story.paragraphs.iter_mut() {
                if p.text().starts_with("Hymn of Praise") {
                    p.set_text("Hymn of Praise\tELW 432");
                }
                if p.text().starts_with("Hymn of Response") {
                    p.set_text("Hymn of Response\tAmazing Grace");
                }
            }
        }
        // Empty memory: incompleteness is named, nothing
        // is invented about what the numbers should be.
        let notes = hymn_findings(&doc, &HymnMemory::default());
        let texts: Vec<&str> = notes.iter().map(|n| n.text.as_str()).collect();
        assert_eq!(notes.len(), 2, "{texts:?}");
        assert!(
            texts
                .iter()
                .any(|t| t.contains("the Hymn of Praise line gives hymn 432 but no title")),
            "{texts:?}"
        );
        assert!(
            texts
                .iter()
                .any(|t| t.contains("the Hymn of Response line gives the title")),
            "{texts:?}"
        );
        assert!(
            notes.iter().all(|n| n.step == Some(1)),
            "findings target This Sunday"
        );

        // Give the praise line its title, teach the memory
        // a different one, and add a duplicate number.
        {
            let story = doc.story_mut(&sid).expect("story");
            for p in story.paragraphs.iter_mut() {
                if p.text().starts_with("Hymn of Praise") {
                    p.set_text("Hymn of Praise\tELW 432 \u{00b7} Joyful, Joyful We Adore Thee");
                }
                if p.text().starts_with("Hymn of Response") {
                    p.set_text("Hymn of Response\tELW 432 \u{00b7} Joyful, Joyful We Adore Thee");
                }
            }
        }
        let mut memory = HymnMemory::default();
        memory.learn(
            &HymnEntry {
                hymnal: Some("ELW".to_string()),
                number: Some("432".to_string()),
                title: Some("O Come, All Ye Faithful".to_string()),
            },
            ob_model::SimpleDate {
                year: 2026,
                month: 10,
                day: 4,
            },
        );
        let notes = hymn_findings(&doc, &memory);
        let texts: Vec<&str> = notes.iter().map(|n| n.text.as_str()).collect();
        // Two disagreements (one per line) and the
        // duplicate named once.
        assert_eq!(notes.len(), 3, "{texts:?}");
        assert_eq!(
            texts
                .iter()
                .filter(|t| t.contains(
                    "was printed as \u{2018}O Come, All Ye Faithful\u{2019} for October 4, 2026"
                ))
                .count(),
            2,
            "{texts:?}"
        );
        // Both disagreement notes carry the pairing the
        // office can tell Check to forget (round 3, R3-6).
        assert_eq!(
            notes
                .iter()
                .filter(|n| n.forget_hymn.as_deref() == Some("elw|432"))
                .count(),
            2,
            "{texts:?}"
        );
        assert_eq!(
            texts
                .iter()
                .filter(|t| t.contains("is named by two hymn lines"))
                .count(),
            1,
            "{texts:?}"
        );
        // When the memory agrees, it is silent.
        let mut agreeing = HymnMemory::default();
        agreeing.learn(
            &HymnEntry {
                hymnal: Some("ELW".to_string()),
                number: Some("432".to_string()),
                title: Some("Joyful, Joyful We Adore Thee".to_string()),
            },
            ob_model::SimpleDate {
                year: 2026,
                month: 10,
                day: 4,
            },
        );
        let notes = hymn_findings(&doc, &agreeing);
        let texts: Vec<&str> = notes.iter().map(|n| n.text.as_str()).collect();
        assert_eq!(notes.len(), 1, "only the duplicate remains: {texts:?}");
        assert!(texts[0].contains("is named by two hymn lines"));
    }

    #[test]
    fn music_proof_carries_hymns_music_and_no_spoken_only_words() {
        let mut doc = r1_template();
        doc.profile.name = "Grace Lutheran Church".to_string();
        let sid = service_story_id(&doc).expect("service story");
        {
            let story = doc.story_mut(&sid).expect("story");
            for p in story.paragraphs.iter_mut() {
                if p.text().starts_with("Hymn of Praise") {
                    p.set_text("Hymn of Praise\tELW 432 \u{00b7} O Come, All Ye Faithful");
                }
            }
            story.paragraphs.push(ob_model::Paragraph::plain(
                "order-line",
                "Anthem\tThe choir sings Morning Has Broken",
            ));
            // A spoken-only name rides in the weekly story;
            // it must never reach the proof.
            story.paragraphs.push({
                let mut p = ob_model::Paragraph::plain("body", "Wilhelmina Quietname");
                p.spoken_only = true;
                p
            });
        }
        // One engraved music page in a picture box.
        let svg = b"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100\" height=\"20\"><path d=\"M0 0h100\"/></svg>";
        let asset = doc.add_asset(
            svg,
            ob_model::AssetKind::Picture,
            "image/svg+xml",
            ob_model::RightsRecord::default(),
        );
        doc.panels[0].frames.push(ob_model::Frame {
            id: "music-1".to_string(),
            name: "Music".to_string(),
            kind: ob_model::FrameKind::Picture,
            rect: ob_model::Rect::new(36_000, 36_000, 468_000, 60_000),
            story_id: None,
            role: ob_model::ContentRole::Standing,
            picture: Some(ob_model::PicturePlacement::new(asset.clone())),
            music: Some(ob_model::MusicFrame {
                system: 0,
                staff_mm: 6.5,
                source: asset,
            }),
        });
        let (proof, counts) = music_proof_document(&doc);
        assert_eq!(counts.hymns, 2);
        assert_eq!(counts.music_pages, 1);
        assert_eq!(
            counts.spoken_only_left_out, 1,
            "the one spoken-only name is counted, so the Print status can say it was left out (R3-15)"
        );
        let all_text: Vec<String> = proof
            .stories
            .iter()
            .flat_map(|s| s.paragraphs.iter().map(|p| p.text()))
            .collect();
        assert!(
            all_text
                .iter()
                .any(|t| t == "Grace Lutheran Church: music for October 18, 2026"),
            "{all_text:?}"
        );
        assert!(
            all_text
                .iter()
                .any(|t| t == "Hymn of Praise: ELW 432 \u{00b7} O Come, All Ye Faithful"),
            "{all_text:?}"
        );
        assert!(
            all_text.iter().any(|t| t == "Hymn of Response: 365"),
            "{all_text:?}"
        );
        assert!(
            all_text
                .iter()
                .any(|t| t == "Anthem: The choir sings Morning Has Broken"),
            "{all_text:?}"
        );
        assert!(
            !all_text.iter().any(|t| t.contains("Quietname")),
            "spoken-only words never reach the proof"
        );
        // Heading page, music page, anthem page.
        assert_eq!(proof.panels.len(), 3);
        let music_frame = &proof.panels[1].frames[0];
        assert!(music_frame.music.is_some());
        assert_eq!(
            music_frame.picture.as_ref().map(|p| p.fit),
            Some(ob_model::PictureFit::ShowWhole)
        );
        assert_eq!(music_frame.rect.w, 468_000, "the bulletin size is kept");
        // And the proof exports through the reader path.
        let bytes = ob_pdf::export_reader_pdf(&proof, ob_pdf::ExportPurpose::ForPrinting);
        assert!(bytes.starts_with(b"%PDF"), "a real PDF comes out");
    }

    #[test]
    fn learn_hymns_reads_the_saved_documents_date() {
        let mut doc = r1_template();
        let sid = service_story_id(&doc).expect("service story");
        {
            let story = doc.story_mut(&sid).expect("story");
            for p in story.paragraphs.iter_mut() {
                if p.text().starts_with("Hymn of Praise") {
                    p.set_text("Hymn of Praise\tELW 432 \u{00b7} O Come, All Ye Faithful");
                }
            }
        }
        let mut memory = ob_model::hymns::HymnMemory::default();
        assert!(learn_hymns(&mut memory, &doc));
        let pair = memory.lookup(Some("ELW"), "432").expect("learned");
        assert_eq!(pair.title, "O Come, All Ye Faithful");
        assert_eq!(
            pair.last_seen,
            ob_model::SimpleDate {
                year: 2026,
                month: 10,
                day: 18
            }
        );
        assert!(
            !learn_hymns(&mut memory, &doc),
            "nothing new the second time"
        );
    }

    #[test]
    fn a_same_sunday_correction_and_forgetting_silence_check() {
        // Round 3, R3-6, end to end: next Sunday's bulletin
        // is saved early with a wrong hymn title; the
        // office spots it, fixes the same file, and saves
        // again. The correction must displace the typo in
        // the memory, or Check flags the correct bulletin
        // forever. And when the office instead says the
        // memory is the wrong one, the finding's forget
        // pairing makes Check drop it.
        fn set_title(doc: &mut Document, title: &str) {
            let sid = service_story_id(doc).expect("service story");
            let story = doc.story_mut(&sid).expect("story");
            for p in story.paragraphs.iter_mut() {
                if p.text().starts_with("Hymn of Praise") {
                    p.set_text(&format!("Hymn of Praise\tELW 432 \u{00b7} {title}"));
                }
            }
        }
        let mut doc = r1_template();
        set_title(&mut doc, "O Come, All Ye Fiathful");
        let mut memory = ob_model::hymns::HymnMemory::default();
        assert!(learn_hymns(&mut memory, &doc), "the typo is learned");
        // The office fixes the file; before the re-save,
        // Check disagrees with the (correct) bulletin and
        // names the pairing the office could forget.
        set_title(&mut doc, "O Come, All Ye Faithful");
        let notes = hymn_findings(&doc, &memory);
        let flagged: Vec<&CheckNote> = notes
            .iter()
            .filter(|n| n.text.contains("was printed as"))
            .collect();
        assert_eq!(
            flagged.len(),
            1,
            "{:?}",
            notes.iter().map(|n| &n.text).collect::<Vec<_>>()
        );
        assert_eq!(flagged[0].forget_hymn.as_deref(), Some("elw|432"));
        // Saving the corrected bulletin heals the memory:
        // the same Sunday's last confirmation stands.
        assert!(learn_hymns(&mut memory, &doc), "the correction lands");
        assert_eq!(
            memory
                .lookup(Some("ELW"), "432")
                .expect("still remembered")
                .title,
            "O Come, All Ye Faithful"
        );
        assert!(
            hymn_findings(&doc, &memory)
                .iter()
                .all(|n| !n.text.contains("was printed as")),
            "Check is silent once the memory matches"
        );
        // The other road: teach the typo once more, see
        // the finding, forget the pairing, and Check has
        // nothing left to disagree with.
        set_title(&mut doc, "O Come, All Ye Fiathful");
        assert!(learn_hymns(&mut memory, &doc));
        set_title(&mut doc, "O Come, All Ye Faithful");
        assert!(hymn_findings(&doc, &memory)
            .iter()
            .any(|n| n.forget_hymn.as_deref() == Some("elw|432")));
        assert!(memory.forget_key("elw|432"));
        assert!(memory.lookup(Some("ELW"), "432").is_none());
        assert!(hymn_findings(&doc, &memory)
            .iter()
            .all(|n| n.forget_hymn.is_none() && !n.text.contains("was printed as")));
    }

    #[test]
    fn hymn_findings_treat_a_bare_number_as_the_same_hymn_listed_twice() {
        // Round 3, R3-17: "ELW 432" on one line and a bare
        // "432" on another used to slip past the duplicate
        // check, which keyed on hymnal AND number together.
        // The finding says which shape each listing took.
        let mut doc = r1_template();
        let sid = service_story_id(&doc).expect("service story");
        {
            let story = doc.story_mut(&sid).expect("story");
            for p in story.paragraphs.iter_mut() {
                if p.text().starts_with("Hymn of Praise") {
                    p.set_text("Hymn of Praise\tELW 432 \u{00b7} O Come, All Ye Faithful");
                }
                if p.text().starts_with("Hymn of Response") {
                    p.set_text("Hymn of Response\t432 \u{00b7} O Come, All Ye Faithful");
                }
            }
        }
        let notes = hymn_findings(&doc, &ob_model::hymns::HymnMemory::default());
        let dup: Vec<&CheckNote> = notes
            .iter()
            .filter(|n| n.text.contains("listed twice"))
            .collect();
        assert_eq!(
            dup.len(),
            1,
            "{:?}",
            notes.iter().map(|n| &n.text).collect::<Vec<_>>()
        );
        assert!(
            dup[0].text.contains(
                "hymn 432 is listed twice, once as ELW 432 and once with no hymnal named"
            ),
            "{}",
            dup[0].text
        );
        // A different hymnal's 432 is a different book:
        // no duplicate is named.
        {
            let story = doc.story_mut(&sid).expect("story");
            for p in story.paragraphs.iter_mut() {
                if p.text().starts_with("Hymn of Response") {
                    p.set_text("Hymn of Response\tLBW 432 \u{00b7} O Come, All Ye Faithful");
                }
            }
        }
        assert!(hymn_findings(&doc, &ob_model::hymns::HymnMemory::default())
            .iter()
            .all(
                |n| !n.text.contains("listed twice") && !n.text.contains("named by two hymn lines")
            ));
    }

    #[test]
    fn music_proof_prints_systems_in_the_pieces_order_once_each() {
        // Round 3, R3-15: the bulletin holds the piece's
        // systems out of order (a box was moved while
        // arranging the page) and one system twice (a box
        // was duplicated). The proof is a reading copy of
        // the piece: systems in the piece's order, each
        // printed once.
        let mut doc = r1_template();
        let svg = b"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100\" height=\"20\"><path d=\"M0 0h100\"/></svg>";
        let asset = doc.add_asset(
            svg,
            ob_model::AssetKind::Picture,
            "image/svg+xml",
            ob_model::RightsRecord::default(),
        );
        let frame = |id: &str, system: u32| ob_model::Frame {
            id: id.to_string(),
            name: "Music".to_string(),
            kind: ob_model::FrameKind::Picture,
            rect: ob_model::Rect::new(36_000, 36_000, 468_000, 60_000),
            story_id: None,
            role: ob_model::ContentRole::Standing,
            picture: Some(ob_model::PicturePlacement::new(asset.clone())),
            music: Some(ob_model::MusicFrame {
                system,
                staff_mm: 6.5,
                source: asset.clone(),
            }),
        };
        doc.panels[0].frames.push(frame("m-a", 2));
        doc.panels[0].frames.push(frame("m-b", 0));
        doc.panels[0].frames.push(frame("m-c", 2));
        doc.panels[0].frames.push(frame("m-d", 1));
        let (proof, counts) = music_proof_document(&doc);
        assert_eq!(counts.music_pages, 3, "the duplicated system prints once");
        let systems: Vec<u32> = proof
            .panels
            .iter()
            .flat_map(|p| p.frames.iter())
            .filter_map(|f| f.music.as_ref().map(|m| m.system))
            .collect();
        assert_eq!(systems, vec![0, 1, 2]);
        assert_eq!(
            proof.panels.len(),
            1 + 3,
            "the words page plus one page per system"
        );
    }

    #[test]
    fn music_proof_prints_every_piece_in_the_service() {
        // Round 4, R4-3: two MusicXML pieces in one
        // service (an opening hymn and a communion
        // hymn). System numbers count from 0 per piece,
        // so keying the proof on the number alone kept
        // only the first piece's system at each number
        // and the second piece vanished from the proof
        // the music director checks. The proof keys on
        // (source, system): pieces in the order their
        // first box appears in the bulletin, systems
        // ascending within a piece, a repeated box
        // printing its system once.
        let mut doc = r1_template();
        let svg_a = b"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100\" height=\"20\"><path d=\"M0 0h100\"/></svg>";
        let svg_b = b"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"120\" height=\"24\"><path d=\"M0 0h120\"/></svg>";
        let asset_a = doc.add_asset(
            svg_a,
            ob_model::AssetKind::Picture,
            "image/svg+xml",
            ob_model::RightsRecord::default(),
        );
        let asset_b = doc.add_asset(
            svg_b,
            ob_model::AssetKind::Picture,
            "image/svg+xml",
            ob_model::RightsRecord::default(),
        );
        assert_ne!(asset_a, asset_b, "two pieces, two sources");
        let frame = |id: &str, asset: &String, system: u32| ob_model::Frame {
            id: id.to_string(),
            name: "Music".to_string(),
            kind: ob_model::FrameKind::Picture,
            rect: ob_model::Rect::new(36_000, 36_000, 468_000, 60_000),
            story_id: None,
            role: ob_model::ContentRole::Standing,
            picture: Some(ob_model::PicturePlacement::new(asset.clone())),
            music: Some(ob_model::MusicFrame {
                system,
                staff_mm: 6.5,
                source: asset.clone(),
            }),
        };
        // Interleaved on the page, out of order, with
        // piece A's first system carried by two boxes.
        doc.panels[0].frames.push(frame("a-1", &asset_a, 1));
        doc.panels[0].frames.push(frame("b-0", &asset_b, 0));
        doc.panels[0].frames.push(frame("a-0", &asset_a, 0));
        doc.panels[0].frames.push(frame("b-1", &asset_b, 1));
        doc.panels[0].frames.push(frame("a-1-dup", &asset_a, 1));
        let (proof, counts) = music_proof_document(&doc);
        assert_eq!(counts.music_pages, 4, "both pieces print in full");
        let systems: Vec<(String, u32)> = proof
            .panels
            .iter()
            .flat_map(|p| p.frames.iter())
            .filter_map(|f| f.music.as_ref().map(|m| (m.source.clone(), m.system)))
            .collect();
        assert_eq!(
            systems,
            vec![
                (asset_a.clone(), 0),
                (asset_a.clone(), 1),
                (asset_b.clone(), 0),
                (asset_b.clone(), 1),
            ]
        );
        assert_eq!(
            proof.panels.len(),
            1 + 4,
            "the words page plus one page per system across both pieces"
        );
    }

    #[test]
    fn a_named_box_takes_its_own_part() {
        let mut doc = r1_template();
        doc.stories.push(ob_model::Story {
            id: "youth".to_string(),
            paragraphs: vec![ob_model::Paragraph::plain("body", "Standing line.")],
            keep_through: None,
        });
        let c = ob_extract::contribution::parse_contribution(
            "## Youth\n\nPizza night is Friday.\n",
            r1_today(),
        )
        .expect("parses");
        let plans = plan_parts(&doc, &c);
        assert_eq!(plans[0].target, SlotTarget::Story("youth".to_string()));
        assert_eq!(plans[0].line, "Goes into the youth box.");
        let summary = apply_contribution(&mut doc, &c);
        let youth = doc.story("youth").expect("story");
        assert_eq!(youth.paragraphs.len(), 2);
        assert!(summary.join(" ").contains("to their boxes."));
    }

    #[test]
    fn filed_names_keep_sender_sunday_and_fingerprint() {
        let name = filed_file_name(
            Some("Pastor Lee"),
            Some(ob_model::SimpleDate {
                year: 2026,
                month: 10,
                day: 25,
            }),
            "abcd1234",
        );
        assert_eq!(name, "Pastor Lee - 2026-10-25 - abcd1234.md");
        let anon = filed_file_name(None, None, "abcd1234");
        assert_eq!(anon, "no name - no Sunday named - abcd1234.md");
        assert_eq!(sanitize_file_part("A/B: C*"), "A-B- C");
    }

    #[test]
    fn sender_card_teaches_the_whole_shape() {
        let card = sender_card_markdown("Grace Lutheran");
        for needle in [
            "Grace Lutheran",
            "From:",
            "For Sunday:",
            "Keep-through:",
            "## For the service",
            "## Announcement",
            "## Insert",
            "## Sunday school",
            "## Youth",
            "Sermon: **The Good Shepherd**",
            "the office fills this in",
        ] {
            assert!(card.contains(needle), "the card is missing {needle}");
        }
        // The card's own example parses as a contribution.
        let example_at = card.find("From: Pastor Lee").expect("example");
        let c = ob_extract::contribution::parse_contribution(&card[example_at..], r1_today())
            .expect("the example on the card parses");
        assert_eq!(c.from.as_deref(), Some("Pastor Lee"));
        assert!(c.for_date.is_some());
    }

    #[test]
    fn expected_parts_parse_commas_lines_and_duplicates() {
        assert_eq!(
            parse_expected_parts("Pastor, Music director\nSunday school\npastor,"),
            vec![
                "Pastor".to_string(),
                "Music director".to_string(),
                "Sunday school".to_string()
            ]
        );
        assert!(parse_expected_parts("  \n, ").is_empty());
    }

    #[test]
    fn children_names_note_repeats_the_rule_and_owns_its_limit() {
        // R-2b: full names is the quiet default; any other
        // rule earns exactly one reminder, worded as a
        // reminder, honest that the app cannot tell which
        // names are children's.
        let mut profile = ob_model::ChurchProfile::default();
        assert!(children_names_note(&profile).is_none());
        profile.children_names = ob_model::ChildrenNames::FirstNamesOnly;
        let note = children_names_note(&profile).expect("a reminder");
        assert_eq!(
            note.text,
            "Your church's rule: children's first names only. This is a reminder \
             to look over the names; the app cannot tell which names are children's."
        );
        profile.children_names = ob_model::ChildrenNames::NoNames;
        let note = children_names_note(&profile).expect("a reminder");
        assert!(note
            .text
            .starts_with("Your church's rule: no children's names."));
    }

    #[test]
    fn picture_children_unnoted_asks_only_for_placed_unnamed_pictures() {
        // R-2b: Check asks about a placed picture whose
        // record has no children answer yet, and only
        // that picture.
        let mut doc = ob_layout::build_template(
            ob_layout::Preset::LetterBifold,
            ob_model::ChurchProfile::default(),
            chrono::NaiveDate::from_ymd_opt(2026, 10, 18).expect("date"),
            "Sunday",
        );
        let png: &[u8] = &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
        let hash = doc.add_asset(
            png,
            ob_model::AssetKind::Picture,
            "image/png",
            ob_model::RightsRecord::default(),
        );
        let mut frame = ob_model::Frame {
            id: "pic".into(),
            name: "Picnic".into(),
            kind: ob_model::FrameKind::Picture,
            rect: ob_model::Rect::new(0, 0, 100_000, 100_000),
            story_id: None,
            role: ob_model::ContentRole::Standing,
            picture: Some(ob_model::PicturePlacement::new(hash.clone())),
            music: None,
        };
        assert!(picture_children_unnoted(&doc, &frame), "no answer yet: ask");
        // Round 3 (R3-4): a music box showing an engraved
        // system of the same unanswered asset is never
        // asked. Sheet music is not a photograph, and the
        // music box's menu has no answer to give.
        let mut music_frame = frame.clone();
        music_frame.music = Some(ob_model::MusicFrame {
            system: 0,
            staff_mm: 6.5,
            source: hash.clone(),
        });
        assert!(
            !picture_children_unnoted(&doc, &music_frame),
            "music is exempt"
        );
        doc.assets
            .iter_mut()
            .find(|a| a.sha256 == hash)
            .expect("asset")
            .rights
            .children_present = Some(false);
        assert!(!picture_children_unnoted(&doc, &frame), "answered: quiet");
        // An empty box and a missing picture are other
        // findings' business.
        frame.picture = None;
        assert!(!picture_children_unnoted(&doc, &frame));
        frame.picture = Some(ob_model::PicturePlacement::new("0".repeat(64)));
        assert!(!picture_children_unnoted(&doc, &frame));
    }

    #[test]
    fn serving_rows_follow_the_profiles_own_lines() {
        // R-2b: This Sunday's rows are the profile's
        // lines, in order, including the church's own;
        // values store under the name as shown.
        let mut doc = ob_layout::build_template(
            ob_layout::Preset::LetterBifold,
            ob_model::ChurchProfile::default(),
            chrono::NaiveDate::from_ymd_opt(2026, 10, 18).expect("date"),
            "Sunday",
        );
        assert_eq!(participant_values(&doc).len(), 6);
        doc.set_participant_value("Children's message", "Ms. Dee");
        doc.set_participant_value("Acolytes", "Sam, Jo");
        let values = participant_values(&doc);
        assert_eq!(values[4], "Ms. Dee");
        assert_eq!(values[5], "Sam, Jo");
        // A renamed list re-rows: same count check the UI
        // leans on (its names snapshot drives the resync).
        doc.profile.serving_lines = vec!["Candle lighters".to_string()];
        doc.set_participant_value("Candle lighters", "Rae");
        assert_eq!(participant_values(&doc), vec!["Rae".to_string()]);
    }
}
