//! OpenBulletin desktop shell.
//!
//! Design law (docs/PRINCIPLES.md): KISS, built for church office
//! administrators, not layout professionals. The home screen is a guided
//! weekly flow with one clear next step at a time:
//!   1 Start  2 This Sunday  3 Announcements  4 Check  5 Print
//! The engine lives in the ob-* crates and the CLI; this shell stays thin.

use eframe::egui;
use ob_layout::{build_template, Preset};
use ob_model::{ChurchProfile, ContentRole, Document};

const STEPS: [&str; 5] = ["Start", "This Sunday", "Announcements", "Check", "Print"];

const GUIDANCE: [&str; 5] = [
    "Pick the bulletin shape and confirm your church name. Everything else starts from last week's file.",
    "Confirm the date and the name of the Sunday. These fill the cover automatically.",
    "Type or paste this week's announcements. Standing items (service times, the order of service) carry over on their own.",
    "We check the things that usually go wrong: missing date, empty announcements, missing church name.",
    "Print the folded bulletin, or save a reading copy as PDF. Then start next Sunday in one click.",
];

/// How one printer flips paper, learned from the test sheet and
/// remembered in a small settings file next to the app's other data.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PrinterSetup {
    /// "long_edge", "short_edge", or "manual".
    pub mode: String,
    /// The plain-words instruction shown in the Print step.
    pub instruction: String,
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub printers: std::collections::BTreeMap<String, PrinterSetup>,
}

pub fn printer_setup_from_answer(answer: &str, printer: &str) -> PrinterSetup {
    match answer {
        "up" => PrinterSetup {
            mode: "long_edge".into(),
            instruction: format!(
                "For the {printer}: in the print window, turn two-sided printing on and pick \"flip on long edge\" (long-edge binding). That is what your test sheet proved works."
            ),
        },
        "down" => PrinterSetup {
            mode: "short_edge".into(),
            instruction: format!(
                "For the {printer}: in the print window, turn two-sided printing on and pick \"flip on short edge\" (short-edge binding). Your test sheet showed long edge comes out upside down on this printer."
            ),
        },
        _ => PrinterSetup {
            mode: "manual".into(),
            instruction: format!(
                "For the {printer}: it prints one side at a time. Print the front sides, put the stack back in the tray printed-side up with the top edge going in first, then print the back sides. Try one sheet first until it comes out right."
            ),
        },
    }
}

pub fn settings_path() -> std::path::PathBuf {
    let base = if let Ok(appdata) = std::env::var("APPDATA") {
        std::path::PathBuf::from(appdata)
    } else if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        std::path::PathBuf::from(xdg)
    } else if let Ok(home) = std::env::var("HOME") {
        let mac = std::path::PathBuf::from(&home).join("Library/Application Support");
        if mac.exists() {
            mac
        } else {
            std::path::PathBuf::from(home).join(".config")
        }
    } else {
        std::path::PathBuf::from(".")
    };
    base.join("OpenBulletin").join("settings.json")
}

/// Where the autosave journal lives: beside settings.json.
/// Two files: the bulletin itself and a two-line note (when it was
/// written, and which file it belongs to, if any).
pub fn journal_paths() -> (std::path::PathBuf, std::path::PathBuf) {
    let dir = settings_path()
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    (dir.join("autosave.obd"), dir.join("autosave.txt"))
}

fn write_journal_to(
    obd: &std::path::Path,
    meta: &std::path::Path,
    doc: &Document,
    source: &Option<std::path::PathBuf>,
) -> Result<(), String> {
    ob_format::save(doc, obd)?;
    let when = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let source_line = source
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    std::fs::write(meta, format!("{when}\n{source_line}\n"))
        .map_err(|e| format!("cannot write the autosave note: {e}"))
}

fn read_journal_from(
    obd: &std::path::Path,
    meta: &std::path::Path,
) -> Option<(Document, Option<std::path::PathBuf>, u64)> {
    let doc = ob_format::load(obd).ok()?;
    let note = std::fs::read_to_string(meta).ok()?;
    let mut lines = note.lines();
    let when: u64 = lines.next()?.trim().parse().ok()?;
    let source = lines.next().unwrap_or("").trim().to_string();
    // The metadata is a plain text file anyone (or any program)
    // can edit: only accept it as a save target when it names a
    // bulletin file, never an arbitrary path (closing audit).
    let source_path = if source.is_empty() {
        None
    } else {
        let pb = std::path::PathBuf::from(&source);
        let is_obd = pb
            .extension()
            .map(|e| e.eq_ignore_ascii_case("obd"))
            .unwrap_or(false);
        if is_obd {
            Some(pb)
        } else {
            None
        }
    };
    Some((doc, source_path, when))
}

fn clear_journal_from(obd: &std::path::Path, meta: &std::path::Path) {
    let _ = std::fs::remove_file(obd);
    let _ = std::fs::remove_file(meta);
}

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
        self.editing = None;
        self.edit_start = None;
        self.edit_done = false;
        self.drag_guide = None;
        self.drag_frame = None;
        self.drag_resize = false;
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
        self.church_name = self.doc.profile.name.clone();
        self.profile_address = self.doc.profile.address.clone();
        self.profile_giving_url = self.doc.profile.giving_url.clone();
        self.profile_service_times = self.doc.profile.service_times.clone();
        self.profile_website = self.doc.profile.website.clone();
        self.profile_pastor = self.doc.profile.pastor.clone();
        self.date_input = self
            .doc
            .fields
            .get("sunday_date")
            .cloned()
            .unwrap_or_default();
    }

    /// Set the bulletin's date from the Step 2 field, in the
    /// printed long form too. Plain failure words when the date
    /// does not read (blocker 1: the date used to be unchangeable).
    fn apply_date(&mut self) {
        match chrono::NaiveDate::parse_from_str(self.date_input.trim(), "%Y-%m-%d") {
            Ok(d) => {
                let before = snapshot(&self.doc);
                self.doc
                    .fields
                    .insert("sunday_date".to_string(), d.format("%Y-%m-%d").to_string());
                self.doc.fields.insert(
                    "sunday_date_long".to_string(),
                    d.format("%A, %B %-d, %Y").to_string(),
                );
                if before != snapshot(&self.doc) {
                    self.record_undo(before);
                }
                self.status = format!("Date set to {}.", d.format("%A, %B %-d, %Y"));
            }
            Err(_) => {
                self.status =
                    "That date does not read. Use year-month-day, like 2026-10-18.".into();
            }
        }
    }

    /// The one way a whole document replaces the current one
    /// (Open, recovery, Undo, Redo). History is cleared when the
    /// replacement crosses a file boundary, so Undo can never
    /// resurrect another bulletin's content under this file's
    /// name (audit blocker 5). Within one file's editing
    /// session, Undo and Redo pass clear_history = false.
    fn adopt_document(&mut self, doc: Document, clear_history: bool) {
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

pub fn load_settings() -> Settings {
    std::fs::read_to_string(settings_path())
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

pub fn save_settings(settings: &Settings) -> Result<(), String> {
    let path = settings_path();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    std::fs::write(&path, text).map_err(|e| e.to_string())
}

/// One row of the Fonts panel: a library font with everything the
/// admin needs (its own face for the preview, its name, and what its
/// license allows in plain words).
struct FontRow {
    key: String,
    family: String,
    full_name: String,
    rights_words: String,
}

struct App {
    doc: Document,
    preset: Preset,
    church_name: String,
    profile_address: String,
    profile_giving_url: String,
    profile_service_times: String,
    profile_website: String,
    profile_pastor: String,
    date_input: String,
    announcements_dirty: bool,
    step: usize,
    announcements: String,
    notes: Vec<String>,
    status: String,
    path: Option<std::path::PathBuf>,
    settings: Settings,
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
    show_fonts: bool,
    fonts_loaded: bool,
    font_rows: Vec<FontRow>,
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
    lines_cache: Option<(String, std::collections::HashMap<usize, Vec<PreviewLine>>)>,
    /// The text shaper, rebuilt only when the Font Library
    /// changes (closing audit: it used to be rebuilt, with a
    /// full library scan and hash of every font, per paragraph
    /// and per panel per repaint).
    shaper_cache: Option<(u64, ob_text::shaping::Shaper)>,
    editing: Option<(usize, usize)>,
    edit_text: String,
    edit_styles: Vec<String>,
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
    /// Two-click guard on throwing recovered work away (F11).
    discard_armed: bool,
    /// Another live OpenBulletin process holds the journal: this
    /// instance never writes or clears it (closing audit).
    peer_alive: bool,
    /// PDF rescue in flight and awaiting review.
    import_rx: Option<std::sync::mpsc::Receiver<Result<ImportReview, String>>>,
    import_review: Option<ImportReview>,
    import_armed: bool,
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
    preview_cache: Option<(String, Vec<PreviewSheet>)>,
    saved_snapshot: String,
    journal_snapshot: String,
    last_journal_check: std::time::Instant,
    startup_done: bool,
    recovery: Option<(Document, Option<std::path::PathBuf>, u64)>,
}

/// One sheet side as the printer will produce it: panel rectangles
/// in points, with the text lines shaped exactly as the PDF shapes
/// them (same shaper, same fonts), so the breaks shown are the
/// breaks printed.
#[derive(Clone)]
struct PreviewSheet {
    w: f32,
    h: f32,
    panels: Vec<PreviewPanel>,
}

#[derive(Clone)]
struct PreviewPanel {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    name: String,
    lines: Vec<PreviewLine>,
}

#[derive(Clone)]
struct PreviewLine {
    text: String,
    size: f32,
    x: f32,
    y: f32,
    w: f32,
    align: ob_model::Align,
}

/// A message back from a catalog worker thread (search or install).
enum CatalogMsg {
    Hits(Vec<ob_fonts::catalog::CatalogEntry>),
    Installed(Vec<String>),
    PackDone {
        installed: Vec<String>,
        failed: Vec<(String, String)>,
    },
    Failed(String),
}

impl Default for App {
    fn default() -> Self {
        let church_name = "Grace Lutheran Church".to_string();
        let preset = Preset::LetterBifold;
        let doc = build_template(
            preset,
            ChurchProfile {
                name: church_name.clone(),
                service_times: "Sundays 9:30 AM".into(),
                ..Default::default()
            },
            chrono::Local::now().date_naive(),
            "Sunday",
        );
        let announcements = weekly_body_text(&doc);
        Self {
            doc,
            preset,
            church_name,
            profile_address: "123 Main Street".into(),
            profile_giving_url: String::new(),
            profile_service_times: "Sundays 9:30 AM".into(),
            profile_website: String::new(),
            profile_pastor: String::new(),
            date_input: String::new(),
            announcements_dirty: false,
            step: 0,
            announcements,
            notes: Vec::new(),
            status: "Welcome to OpenBulletin.".into(),
            path: None,
            settings: load_settings(),
            printer_name: "Church office printer".into(),
            show_pages: false,
            canvas_panel: 0,
            drag_guide: None,
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
            show_fonts: false,
            fonts_loaded: false,
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
            edit_salt: String::new(),
            edit_cursor_para: 0,
            frame_snapshot: String::new(),
            status_error: false,
            pending_open: None,
            pending_open_snapshot: String::new(),
            discard_armed: false,
            peer_alive: false,
            import_rx: None,
            import_review: None,
            import_armed: false,
            asset_base: None,
            pages_zoom: 1.0,
            frame_clipboard: None,
            box_seq: 0,
            edit_done: false,
            focus_edit: false,
            edit_start: None,
            preview_cache: None,
            saved_snapshot: String::new(),
            journal_snapshot: String::new(),
            last_journal_check: std::time::Instant::now(),
            startup_done: false,
            recovery: None,
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
        .map(|p| p.text.clone())
        .collect::<Vec<_>>()
        .join("\n")
}

impl App {
    /// Move to a step the way the Next button does: leaving
    /// Announcements writes the box; arriving at Check runs it.
    /// The step list itself is clickable now (finding F1).
    fn goto_step(&mut self, i: usize) {
        if self.step == 2 && i != 2 {
            self.save_announcements();
        }
        self.step = i;
        if i == 3 {
            self.run_checks();
        }
    }

    fn rebuild(&mut self) {
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
            },
            chrono::Local::now().date_naive(),
            "Sunday",
        );
        let mut doc = doc;
        // Changing only the shape must not reset the week: the
        // date, its long form, the Sunday name, and the reading
        // carry across (closing audit).
        for key in ["sunday_date", "sunday_date_long", "sunday_name", "reading"] {
            if let Some(v) = keep_fields.get(key) {
                doc.fields.insert(key.to_string(), v.clone());
            }
        }
        self.adopt_document(doc, false);
        // A fresh template is not the old file: Save must ask
        // for a name rather than overwrite it (closing audit).
        self.path = None;
        if before != snapshot(&self.doc) {
            self.record_undo(before);
        }
        self.status = format!(
            "Template loaded: {}. Undo brings the old bulletin back.",
            self.preset.label()
        );
    }

    fn save_announcements(&mut self) {
        // The buffer is one of two writers for this story (the
        // Pages canvas is the other). It may only write when it
        // actually changed, or a stale buffer erases canvas
        // typing on the next autosave tick (blocker 4). This
        // gate was once lost in a file splice; the test named
        // announcements_buffer_only_writes_when_dirty guards it.
        if !self.announcements_dirty {
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
                if !inserted && p.style == "subheading" && p.text == "Announcements" {
                    for l in &lines {
                        new_paras.push(ob_model::Paragraph::new("body", l));
                    }
                    inserted = true;
                }
            }
            if !inserted {
                for l in &lines {
                    new_paras.push(ob_model::Paragraph::new("body", l));
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
    fn library_fingerprint() -> u64 {
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
    fn shaper(&mut self) -> &mut ob_text::shaping::Shaper {
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

    fn run_checks(&mut self) {
        let mut notes = Vec::new();
        if self.doc.profile.name.trim().is_empty() {
            notes.push("Check: the church name is blank (Step 1).".to_string());
        } else {
            notes.push("OK: church name is set.".to_string());
        }
        match self.doc.fields.get("sunday_date_long") {
            Some(d) if !d.is_empty() => notes.push(format!("OK: bulletin date is {d}.")),
            _ => notes.push("Check: no Sunday date is set (Step 2).".to_string()),
        }
        if self.announcements.trim().is_empty() {
            notes.push("Check: announcements are empty. If that is correct for this week, you can ignore this.".to_string());
        } else {
            notes.push("OK: announcements are filled in.".to_string());
        }
        // Real fit check: shape every story into its frame and complain
        // in plain words about any that do not fit. Two passes: first
        // collect the words (borrowing the document), then shape them
        // with the one cached shaper. This used to rebuild the whole
        // font engine, with a full library scan, per paragraph
        // (closing audit: 1.25 s on the booklet, worse with more fonts).
        type FitWork = (
            String,
            ob_model::Rect,
            String,
            Vec<(
                String,
                ob_text::shaping::FontSpec,
                ob_model::Mpt,
                ob_model::Mpt,
            )>,
        );
        let mut work: Vec<FitWork> = Vec::new();
        for panel in &self.doc.panels {
            for frame in &panel.frames {
                let Some(sid) = &frame.story_id else { continue };
                let Some(story) = self.doc.story(sid) else {
                    continue;
                };
                let mut paras = Vec::new();
                for para in &story.paragraphs {
                    let style = self.doc.style(&para.style).cloned().unwrap_or_else(|| {
                        ob_model::ParagraphStyle {
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
                        }
                    });
                    let spec = ob_text::shaping::FontSpec {
                        family: style.font_family.clone(),
                        bold: style.bold,
                        italic: style.italic,
                    };
                    let text = ob_model::resolve_fields(&para.text, &self.doc);
                    paras.push((text, spec, style.font_size, style.space_after));
                }
                let first = story
                    .paragraphs
                    .first()
                    .map(|p| p.text.clone())
                    .unwrap_or_else(|| "A text block".into());
                work.push((panel.name.clone(), frame.rect, first, paras));
            }
        }
        let mut fit_notes = Vec::new();
        for (panel_name, rect, first, paras) in &work {
            let mut needed: ob_model::Mpt = 0;
            for (text, spec, font_size, space_after) in paras {
                let lines = if text.contains('\t') {
                    1
                } else {
                    self.shaper()
                        .wrap(text, rect.w, spec, *font_size, 1.3)
                        .len()
                };
                needed += lines as ob_model::Mpt * (font_size * 13 / 10) + space_after;
            }
            if needed > rect.h {
                let over_lines = ((needed - rect.h) / 14_000).max(1);
                fit_notes.push(format!(
                    "Check: the words in \"{}\" ({}) do not all fit. About {} line{} too many. Shorten the words, or we can shrink them in a later version.",
                    first.chars().take(38).collect::<String>(),
                    panel_name,
                    over_lines,
                    if over_lines == 1 { "" } else { "s" }
                ));
            }
        }
        if fit_notes.is_empty() {
            notes.push("OK: everything fits on the page.".to_string());
        } else {
            notes.extend(fit_notes);
        }
        let has_order = self.doc.stories.iter().any(|s| {
            s.paragraphs
                .iter()
                .any(|p| p.text.contains("Order of Service"))
        });
        if has_order {
            notes.push(
                "OK: order of service is present and carries over from last week.".to_string(),
            );
        } else {
            notes.push("Check: no order of service found.".to_string());
        }
        self.notes = notes;
    }

    fn next_sunday(&mut self) {
        if self.editing.is_some() {
            self.commit_edit();
        }
        self.save_announcements();
        let before = snapshot(&self.doc);
        match ob_layout::next_sunday(&self.doc) {
            Ok(next) => {
                self.adopt_document(next, false);
                if before != snapshot(&self.doc) {
                    self.record_undo(before);
                }
                // The new week is a new file: never let a later
                // Save overwrite last week's bulletin, and never
                // call this unsaved document saved (finding F13).
                self.path = None;
                self.step = 1;
                self.status = format!(
                    "Started the {} bulletin. Weekly announcements were cleared; standing items carried over. Save it under its own name.",
                    self.doc.fields.get("sunday_date_long").cloned().unwrap_or_default()
                );
            }
            Err(e) => self.status = format!("Next Sunday failed: {e}"),
        }
    }
}

impl App {
    /// Load the Font Library into the panel, seeding it with the
    /// bundled Liberation faces on first run, and register every
    /// library font with the UI so each preview line is drawn in
    /// its own face.
    fn refresh_fonts(&mut self, ctx: &egui::Context) {
        let Ok(lib) = ob_fonts::FontLibrary::open(ob_fonts::default_library_dir()) else {
            return;
        };
        if lib.scan().is_empty() {
            for (name, bytes) in ob_text::shaping::bundled_font_files() {
                let _ = lib.add_bytes(bytes, name);
            }
        }
        let mut defs = egui::FontDefinitions::default();
        let mut rows = Vec::new();
        for info in lib.scan() {
            let Ok(bytes) = lib.read(&info) else { continue };
            let key = format!("lib-{}", &info.sha256[..12]);
            defs.font_data
                .insert(key.clone(), egui::FontData::from_owned(bytes));
            defs.families.insert(
                egui::FontFamily::Name(key.clone().into()),
                vec![key.clone()],
            );
            rows.push(FontRow {
                key,
                family: info.family.clone(),
                full_name: info.full_name.clone(),
                rights_words: info.rights.plain_words().to_string(),
            });
        }
        ctx.set_fonts(defs);
        self.font_rows = rows;
        self.fonts_loaded = true;
    }

    /// Stamp each style with the fingerprint of the library font it
    /// will be laid out with, so saved files remember their fonts.
    /// The document on disk now matches the screen: remember the
    /// state and retire the autosave journal.
    fn mark_clean(&mut self) {
        self.saved_snapshot = snapshot(&self.doc);
        self.journal_snapshot = self.saved_snapshot.clone();
        let (obd, meta) = journal_paths();
        clear_journal_from(&obd, &meta);
    }

    /// Once per launch: adopt the current document as the saved
    /// state, and pick up any journal a previous session left.
    fn startup(&mut self) {
        self.startup_done = true;
        self.saved_snapshot = snapshot(&self.doc);
        self.journal_snapshot = self.saved_snapshot.clone();
        // Single-instance courtesy: if another live OpenBulletin
        // already holds the journal, this instance leaves the
        // journal entirely alone (closing audit).
        self.peer_alive = peer_instance_alive();
        write_instance_lock();
        let (obd, meta) = journal_paths();
        if let Some((doc, source, when)) = read_journal_from(&obd, &meta) {
            if snapshot(&doc) != self.saved_snapshot {
                self.journal_snapshot = snapshot(&doc);
                if !self.peer_alive {
                    self.recovery = Some((doc, source, when));
                }
            } else if !self.peer_alive {
                clear_journal_from(&obd, &meta);
            }
        }
    }

    /// The autosave tick: about every two seconds, if the work on
    /// screen differs from the last save, write the journal.
    fn journal_tick(&mut self) {
        if self.last_journal_check.elapsed() < std::time::Duration::from_secs(2) {
            return;
        }
        self.last_journal_check = std::time::Instant::now();
        self.save_announcements();
        let snap = snapshot(&self.doc);
        match journal_decision(
            snap == self.saved_snapshot,
            snap == self.journal_snapshot,
            self.recovery.is_some(),
            self.peer_alive,
        ) {
            JournalAction::Hold => {}
            JournalAction::Clear => {
                self.journal_snapshot = snap;
                let (obd, meta) = journal_paths();
                clear_journal_from(&obd, &meta);
            }
            JournalAction::Write => {
                let (obd, meta) = journal_paths();
                if write_journal_to(&obd, &meta, &self.doc, &self.path).is_ok() {
                    self.journal_snapshot = snap;
                }
            }
        }
    }

    /// The two-click unsaved-work guard, shared by every way
    /// of opening a file. Returns true when the open may
    /// proceed. Arming records the screen state; if the work
    /// changes before the confirming click, this re-arms
    /// instead of proceeding (closing audit).
    fn open_guard(&mut self, action: PendingOpen) -> bool {
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
    fn commit_if_editing(&mut self) {
        if self.editing.is_some() {
            self.commit_edit();
        }
    }

    fn pin_fonts(&mut self) {
        if let Ok(lib) = ob_fonts::FontLibrary::open(ob_fonts::default_library_dir()) {
            ob_fonts::pin_document_fonts(&mut self.doc, &lib);
        }
    }

    /// Collect a finished catalog search or install, if one landed.
    fn poll_catalog(&mut self, ctx: &egui::Context) {
        let Some(rx) = &self.catalog_rx else { return };
        match rx.try_recv() {
            Ok(CatalogMsg::Hits(hits)) => {
                self.catalog_hits = hits;
                self.catalog_rx = None;
                self.catalog_busy = false;
            }
            Ok(CatalogMsg::Installed(names)) => {
                self.status = format!("Font installed: {}", names.join(", "));
                self.catalog_rx = None;
                self.catalog_busy = false;
                self.refresh_fonts(ctx);
            }
            Ok(CatalogMsg::PackDone { installed, failed }) => {
                self.status = if failed.is_empty() {
                    format!("Church Starter Pack installed: {} fonts", installed.len())
                } else {
                    format!(
                        "Starter Pack mostly installed: {} fonts in; could not get {}",
                        installed.len(),
                        failed
                            .iter()
                            .map(|(id, _)| id.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                };
                self.catalog_rx = None;
                self.catalog_busy = false;
                self.refresh_fonts(ctx);
            }
            Ok(CatalogMsg::Failed(e)) => {
                self.status = format!("Font catalog: {e}");
                self.catalog_rx = None;
                self.catalog_busy = false;
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                ctx.request_repaint_after(std::time::Duration::from_millis(120));
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.catalog_rx = None;
                self.catalog_busy = false;
            }
        }
    }

    /// Offer back work a previous session left unsaved.
    fn recovery_window(&mut self, ctx: &egui::Context) {
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
                self.path = source;
                // The journal's content is now the screen's content:
                // the saved state stays the older file (or nothing).
                self.journal_snapshot = snapshot(&self.doc);
                self.status = "Your unsaved work is back. Save when ready.".into();
            }
        }
        if throw_away {
            self.recovery = None;
            let (obd, meta) = journal_paths();
            clear_journal_from(&obd, &meta);
            self.journal_snapshot = self.saved_snapshot.clone();
            self.status = "Threw away the unsaved copy.".into();
        }
    }

    /// Build (or reuse) the sheet preview for the Print step.
    fn preview_sheets(&mut self) -> &Vec<PreviewSheet> {
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
    fn sheet_preview_ui(&mut self, ui: &mut egui::Ui) {
        // Build first (borrows self), then draw from the cache.
        let count = self.preview_sheets().len();
        ui.label(format!(
            "This is exactly what comes out of the printer: {count} sheet side{}.
             Lines break where they will print.",
            if count == 1 { "" } else { "s" }
        ));
        let sheets = self.preview_sheets().clone();
        for (i, sheet) in sheets.iter().enumerate() {
            let pair = i / 2 + 1;
            let face = if i % 2 == 0 { "front" } else { "back" };
            ui.label(egui::RichText::new(format!("Sheet {pair}, {face}")).strong());
            draw_sheet(ui, sheet);
        }
    }

    /// The settings to use in the computer's print window, in plain
    /// words, computed from this document and the learned printer.
    fn print_checklist(&self) -> Vec<String> {
        print_checklist_for(&self.doc, &self.settings, &self.printer_name)
    }

    /// Paragraph count sync for the edit buffer: new paragraphs
    /// inherit the previous paragraph's style (docs/EDITOR.md).
    fn sync_edit_styles(&mut self) {
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
    fn paste_box(
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
    fn add_text_box(&mut self, idx: usize) {
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
        });
        self.selected_frame = Some(panel.frames.len() - 1);
        self.note("Text box added in the middle of the page. Drag it where it belongs.".into());
    }

    fn cancel_edit(&mut self) {
        self.editing = None;
        self.edit_start = None;
        self.edit_text.clear();
        self.edit_styles.clear();
        self.edit_done = false;
        self.status = "Typing thrown away. The page keeps the words it had.".into();
    }

    fn enter_edit(&mut self, pi: usize, fi: usize) {
        if self.editing.is_some() {
            self.commit_edit();
        }
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
        self.edit_text = story
            .paragraphs
            .iter()
            .map(|p| p.text.clone())
            .collect::<Vec<_>>()
            .join("\n");
        self.edit_styles = story.paragraphs.iter().map(|p| p.style.clone()).collect();
        self.editing = Some((pi, fi));
        self.selected_frame = Some(fi);
        self.edit_done = false;
        self.focus_edit = true;
    }

    /// Write the edit buffer back into the story and leave
    /// editing. The gesture closes, so the whole typing session
    /// is one Undo step.
    fn commit_edit(&mut self) {
        let Some((pi, fi)) = self.editing.take() else {
            return;
        };
        self.sync_edit_styles();
        let Some(frame) = self.doc.panels[pi].frames.get(fi) else {
            self.edit_start = None;
            return;
        };
        let Some(sid) = frame.story_id.clone() else {
            self.edit_start = None;
            return;
        };
        let texts: Vec<String> = self.edit_text.split('\n').map(|t| t.to_string()).collect();
        let paragraphs: Vec<ob_model::Paragraph> = texts
            .iter()
            .enumerate()
            .map(|(i, t)| {
                let style = self
                    .edit_styles
                    .get(i)
                    .cloned()
                    .unwrap_or_else(|| "body".to_string());
                ob_model::Paragraph {
                    style,
                    text: t.clone(),
                }
            })
            .collect();
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

    /// Shaped lines for one panel, cached per document state:
    /// the canvas and the print preview can never disagree.
    fn shaped_panel_lines(&mut self, idx: usize) -> Vec<PreviewLine> {
        let key = self.frame_snapshot.clone();
        let stale = self
            .lines_cache
            .as_ref()
            .map(|(k, _)| k != &key)
            .unwrap_or(true);
        if stale {
            self.lines_cache = Some((key.clone(), std::collections::HashMap::new()));
        }
        if let Some((_, map)) = &self.lines_cache {
            if let Some(lines) = map.get(&idx) {
                return lines.clone();
            }
        }
        let lines = {
            // Clone on a cache miss only: the shaper borrows the
            // app mutably while the lines are computed from the doc.
            let doc = self.doc.clone();
            let shaper = self.shaper();
            panel_lines(&doc, &doc.panels[idx], shaper)
        };
        if let Some((_, map)) = &mut self.lines_cache {
            map.insert(idx, lines.clone());
        }
        lines
    }

    /// The Pages window: one panel at a time, with inch rulers,
    /// the tracing picture, frame outlines, and the panel's guides.
    /// Guides follow office habits: drag to move, right-click to
    /// type the exact place (docs/EDITOR.md).
    fn pages_window(&mut self, ctx: &egui::Context) {
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
            .default_width(720.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("<").clicked() && self.canvas_panel > 0 {
                        // Changing pages mid-typing commits the
                        // session first, while its panel index is
                        // still the page it belongs to.
                        self.commit_if_editing();
                        self.canvas_panel -= 1;
                        self.selected_frame = None;
                        self.guide_menu_for = None;
                        self.frame_menu_for = None;
                    }
                    let panel = &self.doc.panels[self.canvas_panel];
                    ui.label(egui::RichText::new(format!(
                        "{}  ({} of {})",
                        panel.name,
                        self.canvas_panel + 1,
                        self.doc.panels.len()
                    )).strong());
                    if ui.button(">").clicked()
                        && self.canvas_panel + 1 < self.doc.panels.len()
                    {
                        self.commit_if_editing();
                        self.canvas_panel += 1;
                        self.selected_frame = None;
                        self.guide_menu_for = None;
                        self.frame_menu_for = None;
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
                ui.label("Right-click anything for its exact place in inches. The blue guides never print.");
                ui.small("Keys: Tab moves to the next box, Enter types in it, arrows move a box (Shift for bigger steps), Ctrl+C and Ctrl+V copy a box, Ctrl+Z undoes.");
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
                    let para = cursor_paragraph(ctx, &self.edit_text, &self.edit_salt);
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
                        if ui.button("Done typing").clicked() {
                            self.edit_done = true;
                        }
                    });
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
    fn panel_canvas(&mut self, ui: &mut egui::Ui, idx: usize, ctx: &egui::Context) {
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
                    .or_else(|| {
                        self.asset_base
                            .as_ref()
                            .map(|d| d.display().to_string())
                    })
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
                    egui::Color32::from_rgb(150, 60, 50),
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
        if let Some(fi) = self.selected_frame {
            if let Some(f) = panel.frames.get(fi) {
                let r = f.rect;
                let frect = egui::Rect::from_min_max(
                    to_px(r.x as f32 / 1000.0, r.y as f32 / 1000.0),
                    to_px((r.x + r.w) as f32 / 1000.0, (r.y + r.h) as f32 / 1000.0),
                );
                let blue = egui::Color32::from_rgb(0, 110, 190);
                painter.rect_stroke(frect, 0.0, egui::Stroke::new(1.75_f32, blue));
                let handle = egui::Rect::from_min_size(
                    egui::pos2(frect.max.x - 9.0, frect.max.y - 9.0),
                    egui::vec2(9.0, 9.0),
                );
                painter.rect_filled(handle, 0.0, egui::Color32::WHITE);
                painter.rect_stroke(handle, 0.0, egui::Stroke::new(1.25_f32, blue));
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
                let anchor = match line.align {
                    ob_model::Align::Left => egui::Align2::LEFT_BOTTOM,
                    ob_model::Align::Center => egui::Align2::CENTER_BOTTOM,
                    ob_model::Align::Right => egui::Align2::RIGHT_BOTTOM,
                };
                let lx = match line.align {
                    ob_model::Align::Left => line.x,
                    ob_model::Align::Center => line.x + line.w / 2.0,
                    ob_model::Align::Right => line.x + line.w,
                };
                clip.text(
                    to_px(lx, line.y),
                    anchor,
                    &line.text,
                    egui::FontId::proportional((line.size * scale).max(4.0)),
                    egui::Color32::BLACK,
                );
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
        let near_guide = |pos: egui::Pos2| near_guide_tol(pos, 5.0);
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
                    if self.doc.panels[idx].frames[fi].story_id.is_some() {
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
                            let handle = self.selected_frame == Some(fi)
                                && pm.0 >= f.rect.x + f.rect.w - 14_000
                                && pm.1 >= f.rect.y + f.rect.h - 14_000;
                            self.selected_frame = Some(fi);
                            self.drag_frame = Some(fi);
                            self.drag_resize = handle;
                            self.grab_dx = pm.0 - f.rect.x;
                            self.grab_dy = pm.1 - f.rect.y;
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
            self.end_gesture();
        }
        if response.clicked() {
            if let Some(pos) = pointer {
                if near_guide(pos).is_none() {
                    let pm = to_mpt(pos);
                    self.selected_frame = frame_hit(pm);
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
                if ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::C)) {
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
                if ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::V)) {
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
        let guide_color = egui::Color32::from_rgb(0, 150, 200);
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

        // The inline editor, standing exactly where the frame is.
        if self.editing.map(|(pi, _)| pi) == Some(idx) {
            let (_, efi) = self.editing.expect("editing checked");
            self.sync_edit_styles();
            let f = panel.frames[efi].clone();
            let frect = egui::Rect::from_min_max(
                to_px(f.rect.x as f32 / 1000.0, f.rect.y as f32 / 1000.0),
                to_px(
                    (f.rect.x + f.rect.w) as f32 / 1000.0,
                    (f.rect.y + f.rect.h) as f32 / 1000.0,
                ),
            );
            let first_size = self
                .edit_styles
                .first()
                .and_then(|sid| self.doc.style(sid))
                .map(|st| st.font_size as f32 / 1000.0 * scale)
                .unwrap_or(12.0)
                .clamp(10.0, 20.0);
            let edit_id = egui::Id::new(("canvas-edit", self.edit_salt.as_str()));
            if self.focus_edit {
                self.focus_edit = false;
                ctx.memory_mut(|m| m.request_focus(edit_id));
            }
            let editor = egui::TextEdit::multiline(&mut self.edit_text)
                .id(edit_id)
                .font(egui::FontId::proportional(first_size))
                .desired_width(frect.width());
            ui.put(frect, editor);
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
                                .map(|o| ob_model::inches_to_mpt(o.unwrap()))
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
                                    ob_model::inches_to_mpt(inches).clamp(0, extent);
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

    /// Load a panel's tracing picture (PNG beside the document)
    /// as an egui texture, or None when it cannot be read.
    fn load_bg_texture(&self, ctx: &egui::Context, rel: &str) -> Option<egui::TextureHandle> {
        let base = self
            .path
            .as_ref()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()))
            .or_else(|| self.asset_base.clone())
            .unwrap_or_default();
        let bytes = std::fs::read(base.join(rel)).ok()?;
        let img = image::load_from_memory(&bytes).ok()?.to_rgba8();
        let (w, h) = img.dimensions();
        let color =
            egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], img.as_raw());
        Some(ctx.load_texture("trace-bg", color, egui::TextureOptions::LINEAR))
    }

    /// Begin a PDF rescue on a worker thread: read the old
    /// bulletin with the helper, render its pages as tracing
    /// pictures, and build the draft.
    fn start_import(&mut self, pdf: std::path::PathBuf) {
        let profile = self.doc.profile.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        self.import_rx = Some(rx);
        self.note("Reading your old bulletin...".into());
        std::thread::spawn(move || {
            let result = (|| -> Result<ImportReview, String> {
                let helper = ob_extract::helper::resolve_helper(None);
                let dump = ob_extract::helper::dump_pdf(&helper, &pdf)?;
                let workdir = std::env::temp_dir()
                    .join(format!("openbulletin-import-{}", std::process::id()));
                let _ = std::fs::remove_dir_all(&workdir);
                ob_extract::helper::render_pdf(&helper, &pdf, &workdir.join("assets"))?;
                let out =
                    ob_extract::draft::document_from_dump(&dump, profile, true);
                Ok(ImportReview {
                    doc: out.doc,
                    items: out.items,
                    workdir,
                    source_name: pdf
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| "old bulletin".into()),
                })
            })();
            let _ = tx.send(result);
        });
    }

    fn poll_import(&mut self, ctx: &egui::Context) {
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
                self.fail(format!("That PDF could not be brought in. {e}"));
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.import_rx = None;
                self.fail("That PDF could not be brought in; the reader stopped.".into());
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
        }
    }

    /// Adopt a reviewed draft. The pictures stay in the work
    /// folder as the asset base until the first Save moves
    /// them beside the bulletin file.
    fn finalize_import(&mut self, edit: bool) {
        let Some(review) = self.import_review.take() else {
            return;
        };
        self.import_armed = false;
        self.asset_base = Some(review.workdir.clone());
        self.adopt_document(review.doc, true);
        self.path = None;
        if edit {
            self.show_pages = true;
            self.note(
                "Here is your old bulletin as a draft. The old pages sit under each \
                 page as pictures to check against."
                    .into(),
            );
        } else {
            self.step = 1;
            self.note(
                "Here is your old bulletin as a draft. Check the date, then work \
                 through the steps. Save it under a new name when ready."
                    .into(),
            );
        }
    }

    fn import_review_window(&mut self, ctx: &egui::Context) {
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
                ui.separator();
                egui::ScrollArea::vertical()
                    .max_height(300.0)
                    .show(ui, |ui| {
                        for item in &review.items {
                            let (tag, color) = match item.badge {
                                ob_extract::draft::Badge::Matched => {
                                    ("Good", egui::Color32::from_rgb(40, 110, 60))
                                }
                                ob_extract::draft::Badge::CheckThis => {
                                    ("Check this", egui::Color32::from_rgb(170, 110, 20))
                                }
                                ob_extract::draft::Badge::PictureOnly => {
                                    ("Picture only", egui::Color32::from_rgb(90, 90, 160))
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
                    if ui.button("Use it and arrange the pages").clicked() {
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
                let _ = std::fs::remove_dir_all(&review.workdir);
            }
            self.import_armed = false;
            self.note("Import cancelled; nothing changed.".into());
        }
    }

    fn fonts_window(&mut self, ctx: &egui::Context) {
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
            .resizable(true)
            .default_width(520.0)
            .show(ctx, |ui| {
                ui.label("These are the fonts this app may use.");
                ui.label("Bulletins are laid out only with fonts from this library, so they look the same on every computer.");
                ui.add_space(6.0);
                egui::ScrollArea::vertical()
                    .max_height(320.0)
                    .show(ui, |ui| {
                        for row in &self.font_rows {
                            ui.label(
                                egui::RichText::new(
                                    "Welcome! The Lord be with you. 1234567890",
                                )
                                .font(egui::FontId::new(
                                    17.0,
                                    egui::FontFamily::Name(row.key.clone().into()),
                                )),
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
                ui.label(egui::RichText::new("Put a font to work").strong());
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
                        .add_filter("Font files and packs", &["ttf", "otf", "zip"])
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
                                        match lib.add_path(path) {
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
                                self.status = if added.is_empty() {
                                    format!(
                                        "No fonts added. {}",
                                        problems.into_iter().next().unwrap_or_default()
                                    )
                                } else {
                                    format!(
                                        "Fonts added: {}{}",
                                        added.join(", "),
                                        caution
                                    )
                                };
                                self.refresh_fonts(ctx);
                            }
                            Err(e) => {
                                self.status = format!("Could not open the font library: {e}")
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
                    match ob_fonts::FontLibrary::open(ob_fonts::default_library_dir())
                        .and_then(|lib| lib.add_path(&path))
                    {
                        Ok(info) => {
                            self.status = if info.rights.may_package() {
                                format!("Font copied in: {}", info.full_name)
                            } else {
                                format!(
                                    "Font copied in: {}. Note: it did not come from \
                                     the free catalog; only share bulletins using \
                                     it if you have the right to share that font.",
                                    info.full_name
                                )
                            };
                            self.system_rows
                                .retain(|f| f.info.sha256 != info.sha256);
                            self.refresh_fonts(ctx);
                        }
                        Err(e) => {
                            self.status = format!("Could not add that font: {e}")
                        }
                    }
                }
            });
        self.show_fonts = open;
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.startup_done {
            self.startup();
        }
        self.poll_import(ctx);
        self.import_review_window(ctx);
        self.frame_snapshot = snapshot(&self.doc);
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
        // Global undo/redo, when no text box has the keyboard (text
        // boxes keep their own undo while you type in them).
        if ctx.memory(|m| m.focused()).is_none() && self.editing.is_none() {
            let (undo, redo) = ctx.input(|i| {
                (
                    i.modifiers.ctrl && i.key_pressed(egui::Key::Z) && !i.modifiers.shift,
                    i.modifiers.ctrl
                        && (i.key_pressed(egui::Key::Y)
                            || (i.key_pressed(egui::Key::Z) && i.modifiers.shift)),
                )
            });
            if undo {
                self.undo();
            } else if redo {
                self.redo();
            }
        }
        self.journal_tick();
        self.recovery_window(ctx);
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("OpenBulletin");
                if ui.button("New bulletin").clicked() {
                    self.goto_step(0);
                    self.note(
                        "Step 1 is where a fresh bulletin starts: pick a shape and it \
                         rebuilds. Nothing you have open is touched until you pick one."
                            .into(),
                    );
                }
                if ui.button("Pages").clicked() {
                    self.show_pages = true;
                }
                if ui.button("Fonts").clicked() {
                    self.show_fonts = true;
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
                let open_label = if self.pending_open == Some(PendingOpen::Plain) {
                    "Leave unsaved work and open?"
                } else {
                    "Open bulletin..."
                };
                if ui.button(open_label).clicked() && self.open_guard(PendingOpen::Plain) {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("OpenBulletin bulletin", &["obd"])
                        .pick_file()
                    {
                        match ob_format::load(&path) {
                            Ok(doc) => {
                                self.adopt_document(doc, true);
                                self.path = Some(path);
                                self.step = 1;
                                self.note("Bulletin opened.".into());
                                self.mark_clean();
                            }
                            Err(e) => self.fail(format!(
                                "Could not open that file. Pick a bulletin file ending in .obd. ({e})"
                            )),
                        }
                    }
                }
                if ui.button("Save bulletin").clicked() {
                    self.commit_if_editing();
                    self.save_announcements();
                    self.pin_fonts();
                    let target = self.path.clone().or_else(|| {
                        rfd::FileDialog::new()
                            .set_file_name("bulletin.obd")
                            .add_filter("OpenBulletin bulletin", &["obd"])
                            .save_file()
                    });
                    if let Some(path) = target {
                        match ob_format::save(&self.doc, &path) {
                            Ok(()) => {
                                self.note(format!("Saved to {}", path.display()));
                                // An import's page pictures move
                                // beside the file on first Save.
                                if let Some(base) = self.asset_base.take() {
                                    if let Some(parent) = path.parent() {
                                        let from = base.join("assets");
                                        let to = parent.join("assets");
                                        if from.exists() && from != to {
                                            let _ = std::fs::create_dir_all(&to);
                                            if let Ok(rd) = std::fs::read_dir(&from) {
                                                for e in rd.flatten() {
                                                    let _ = std::fs::copy(
                                                        e.path(),
                                                        to.join(e.file_name()),
                                                    );
                                                }
                                            }
                                        }
                                    }
                                }
                                self.path = Some(path);
                                self.mark_clean();
                            }
                            Err(e) => self.fail(format!(
                                "Could not save the file. Try your Documents folder. ({e})"
                            )),
                        }
                    }
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
            ui.label(GUIDANCE[self.step]);
        });

        egui::SidePanel::left("steps")
            .resizable(true)
            .default_width(if self.step == 4 { 560.0 } else { 300.0 })
            .show(ctx, |ui| {
                for (i, name) in STEPS.iter().enumerate() {
                    let marker = if i == self.step {
                        ">"
                    } else if i < self.step {
                        "done -"
                    } else {
                        ""
                    };
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
                match self.step {
                    0 => {
                        ui.heading("Start from last week");
                        ui.label("The usual week: open last Sunday's bulletin and we move it to this Sunday. Last week's announcements clear; everything standing stays.");
                        let last_week_label =
                            if self.pending_open == Some(PendingOpen::LastWeek) {
                                "Leave unsaved work and use last week's?"
                            } else {
                                "Use last week's bulletin..."
                            };
                        if ui.button(last_week_label).clicked()
                            && self.open_guard(PendingOpen::LastWeek)
                        {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("OpenBulletin bulletin", &["obd"])
                                .pick_file()
                            {
                                match ob_format::load(&path) {
                                    Ok(doc) => {
                                        self.adopt_document(doc, true);
                                        self.path = Some(path);
                                        self.mark_clean();
                                        self.next_sunday();
                                    }
                                    Err(e) => self.fail(format!(
                                        "Could not open that file. Pick a bulletin file ending in .obd. ({e})"
                                    )),
                                }
                            }
                        }
                        ui.separator();
                        ui.heading("Bring in an old bulletin");
                        ui.label("Have last year's bulletin as a PDF? We can read it and rebuild it as a draft you can edit: the words come over, and each old page sits underneath as a picture to check against.");
                        if self.import_rx.is_some() {
                            ui.label("Reading your old bulletin...");
                        } else if ui.button("Open a PDF bulletin...").clicked() {
                            if let Some(pdf) = rfd::FileDialog::new()
                                .add_filter("PDF bulletin", &["pdf"])
                                .pick_file()
                            {
                                self.start_import(pdf);
                            }
                        }
                        ui.separator();
                        ui.heading("Or start fresh: bulletin shape");
                        for p in Preset::all() {
                            if ui.selectable_label(self.preset == p, p.label()).clicked() {
                                self.preset = p;
                                self.rebuild();
                            }
                        }
                        ui.separator();
                        ui.label("Church name");
                        if ui.text_edit_singleline(&mut self.church_name).lost_focus() {
                            let name = self.church_name.clone();
                            if self.doc.profile.name != name {
                                let before = snapshot(&self.doc);
                                self.doc.profile.name = name;
                                self.record_undo(before);
                            }
                        }
                        ui.label("Address");
                        let resp_profile_address = ui.text_edit_singleline(&mut self.profile_address);
                        if resp_profile_address.lost_focus() && self.doc.profile.address != self.profile_address {
                                let before = snapshot(&self.doc);
                                self.doc.profile.address = self.profile_address.clone();
                                self.record_undo(before);
                        }
                        ui.label("Service times");
                        let resp_times = ui.text_edit_singleline(&mut self.profile_service_times);
                        if resp_times.lost_focus()
                            && self.doc.profile.service_times != self.profile_service_times
                        {
                                let before = snapshot(&self.doc);
                                self.doc.profile.service_times =
                                    self.profile_service_times.clone();
                                self.record_undo(before);
                        }
                        ui.label("Online giving link (shows as “Give online:” on the back page)");
                        let resp_giving = ui.text_edit_singleline(&mut self.profile_giving_url);
                        if resp_giving.lost_focus()
                            && self.doc.profile.giving_url != self.profile_giving_url
                        {
                                let before = snapshot(&self.doc);
                                self.doc.profile.giving_url =
                                    self.profile_giving_url.clone();
                                self.record_undo(before);
                        }
                        ui.label("Website");
                        let resp_profile_website = ui.text_edit_singleline(&mut self.profile_website);
                        if resp_profile_website.lost_focus() && self.doc.profile.website != self.profile_website {
                                let before = snapshot(&self.doc);
                                self.doc.profile.website = self.profile_website.clone();
                                self.record_undo(before);
                        }
                        ui.label("Pastor");
                        let resp_profile_pastor = ui.text_edit_singleline(&mut self.profile_pastor);
                        if resp_profile_pastor.lost_focus() && self.doc.profile.pastor != self.profile_pastor {
                                let before = snapshot(&self.doc);
                                self.doc.profile.pastor = self.profile_pastor.clone();
                                self.record_undo(before);
                        }
                        ui.small("These print on the cover and carry into every week. They fill in the {Church.*} spots in your templates.");
                    }
                    1 => {
                        ui.heading("This Sunday");
                        ui.label(format!(
                            "Date: {}",
                            self.doc.fields.get("sunday_date_long").cloned().unwrap_or_default()
                        ));
                        ui.horizontal(|ui| {
                            ui.label("Change it:");
                            let date_edit = ui.text_edit_singleline(&mut self.date_input);
                            if date_edit.lost_focus()
                                && ui.input(|i| i.key_pressed(egui::Key::Enter))
                            {
                                self.apply_date();
                            }
                            if ui.button("Set date").clicked() {
                                self.apply_date();
                            }
                            if ui.button("This Sunday").clicked() {
                                let d = sunday_on_or_after(chrono::Local::now().date_naive());
                                self.date_input = d.format("%Y-%m-%d").to_string();
                                self.apply_date();
                            }
                            if ui.button("Next Sunday").clicked() {
                                let d = sunday_on_or_after(chrono::Local::now().date_naive())
                                    + chrono::Duration::days(7);
                                self.date_input = d.format("%Y-%m-%d").to_string();
                                self.apply_date();
                            }
                        });
                        ui.small("Year-month-day, like 2026-10-18. This is the date printed on the bulletin.");
                        ui.label("Name of the Sunday");
                        let mut sn = self.doc.fields.get("sunday_name").cloned().unwrap_or_default();
                        let resp_sn = ui.text_edit_singleline(&mut sn);
                        if resp_sn.lost_focus()
                            && self.doc.fields.get("sunday_name").map(|v| v.as_str()) != Some(sn.as_str())
                        {
                                let before = snapshot(&self.doc);
                                self.doc.fields.insert("sunday_name".to_string(), sn);
                                self.record_undo(before);
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
                        ui.separator();
                        if ui.button("Make next week's bulletin from this one").clicked() {
                            self.next_sunday();
                        }
                    }
                    2 => {
                        ui.heading("Announcements");
                        if ui.button("See the announcements on the page").clicked() {
                            self.show_pages = true;
                        }
                        ui.label("One announcement per line. Paste from email works; formatting is cleaned up for you.");
                        let ann_resp = ui.add(
                            egui::TextEdit::multiline(&mut self.announcements)
                                .desired_rows(12)
                                .desired_width(f32::INFINITY),
                        );
                        if ann_resp.changed() {
                            self.announcements_dirty = true;
                        }
                        if ui.button("Save announcements").clicked() {
                            self.save_announcements();
                            self.status = "Announcements saved.".into();
                        }
                    }
                    3 => {
                        ui.heading("Before you print");
                        for n in &self.notes {
                            ui.label(n);
                        }
                        if ui.button("Check again").clicked() {
                            self.save_announcements();
                            self.run_checks();
                        }
                        ui.horizontal(|ui| {
                            if ui.button("Fix the date or readings").clicked() {
                                self.goto_step(1);
                            }
                            if ui.button("Fix the words").clicked() {
                                self.goto_step(2);
                            }
                            if ui.button("Open the Pages window").clicked() {
                                self.show_pages = true;
                            }
                        });
                        ui.small("Too much text in a box? Open the Pages window and make the box bigger, or shorten the words.");
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
                        ui.small("Open makes a one-time copy to print right now. The Save buttons below keep files you can reuse.");
                        if ui.button("Open the print copy").clicked() {
                            self.commit_if_editing();
                            self.pin_fonts();
                            let bytes = ob_pdf::export_imposed_pdf(&self.doc);
                            let path = std::env::temp_dir().join(format!(
                                "openbulletin-print-copy-{}.pdf",
                                std::process::id()
                            ));
                            match std::fs::write(&path, &bytes).map_err(|e| e.to_string()).and_then(|_| open_with_os(&path)) {
                                Ok(()) => self.status = "Print copy opened in your PDF viewer. Use the settings listed above.".into(),
                                Err(e) => self.status = e,
                            }
                        }
                        ui.separator();
                        ui.label("Print copy: folded sheets, in the right order, ready for the copier.");
                        if ui.button(egui::RichText::new("Save print copy (PDF)").strong()).clicked() {
                            self.commit_if_editing();
                            self.save_announcements();
                            if let Some(path) = rfd::FileDialog::new()
                                .set_file_name("bulletin-print.pdf")
                                .add_filter("PDF", &["pdf"])
                                .save_file()
                            {
                                self.pin_fonts();
                                let bytes = ob_pdf::export_imposed_pdf(&self.doc);
                                match std::fs::write(&path, &bytes) {
                                    Ok(()) => self.status = format!("Saved print copy to {}", path.display()),
                                    Err(e) => self.status = format!("Could not save the print copy: {e}"),
                                }
                            }
                        }
                        ui.label("Reading copy: one panel per page, for email and the website.");
                        if ui.button("Save reading copy (PDF)").clicked() {
                            self.commit_if_editing();
                            self.save_announcements();
                            if let Some(path) = rfd::FileDialog::new()
                                .set_file_name("bulletin-reading.pdf")
                                .add_filter("PDF", &["pdf"])
                                .save_file()
                            {
                                self.pin_fonts();
                                let bytes = ob_pdf::export_reader_pdf(&self.doc);
                                match std::fs::write(&path, &bytes) {
                                    Ok(()) => self.status = format!("Saved reading copy to {}", path.display()),
                                    Err(e) => self.status = format!("Could not save the reading copy: {e}"),
                                }
                            }
                        }
                        ui.separator();
                        ui.label(egui::RichText::new("Two-sided printing, first time on a printer?").strong());
                        ui.label("Every printer flips the paper its own way. Print one test sheet and tell us how it came out. We remember the answer for that printer.");
                        ui.horizontal(|ui| {
                            ui.label("Printer:");
                            ui.text_edit_singleline(&mut self.printer_name);
                        });
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
                                match std::fs::write(&path, ob_pdf::export_duplex_test_pdf()) {
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
                        if ui.button(egui::RichText::new("Start next Sunday").strong()).clicked() {
                            self.next_sunday();
                        }
                    }
                }
                ui.separator();
                if self.status_error {
                    ui.label(
                        egui::RichText::new(format!("Something needs attention: {}", self.status))
                            .color(egui::Color32::from_rgb(170, 40, 30))
                            .strong(),
                    );
                } else {
                    ui.small(&self.status);
                }
            });

        self.pages_window(ctx);
        self.fonts_window(ctx);
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label(egui::RichText::new("Preview: what the congregation will hold").strong());
            ui.small("Laid out with your real fonts, the same lines the printer gets. The Pages window is where you move boxes.");
            egui::ScrollArea::vertical().show(ui, |ui| {
                let count = self.doc.panels.len();
                for idx in 0..count {
                    let panel = self.doc.panels[idx].clone();
                    ui.label(egui::RichText::new(&panel.name).strong());
                    let scale = 0.85f32;
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
                        let anchor = match line.align {
                            ob_model::Align::Left => egui::Align2::LEFT_BOTTOM,
                            ob_model::Align::Center => egui::Align2::CENTER_BOTTOM,
                            ob_model::Align::Right => egui::Align2::RIGHT_BOTTOM,
                        };
                        let lx = match line.align {
                            ob_model::Align::Left => line.x,
                            ob_model::Align::Center => line.x + line.w / 2.0,
                            ob_model::Align::Right => line.x + line.w,
                        };
                        painter.text(
                            rect.min + egui::vec2(lx * scale, line.y * scale),
                            anchor,
                            &line.text,
                            egui::FontId::proportional((line.size * scale).max(4.0)),
                            egui::Color32::BLACK,
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

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1180.0, 820.0]),
        ..Default::default()
    };
    eframe::run_native(
        "OpenBulletin",
        options,
        Box::new(|_cc| Ok(Box::new(App::default()))),
    )
}

/// Shape the whole document the way the PDF path does and record
/// the lines per imposed sheet side.
/// Shape one panel's text frames into lines, in panel-local
/// point coordinates. Shared by the print preview (which offsets
/// them onto sheets) and the Pages canvas (which draws them in
/// place), so both always agree.
fn panel_lines(
    doc: &Document,
    panel: &ob_model::Panel,
    shaper: &mut ob_text::shaping::Shaper,
) -> Vec<PreviewLine> {
    let mut lines: Vec<PreviewLine> = Vec::new();
    for frame in &panel.frames {
        if frame.kind == ob_model::FrameKind::Shape {
            continue;
        }
        let Some(sid) = &frame.story_id else { continue };
        let Some(story) = doc.story(sid) else {
            continue;
        };
        let mut y = frame.rect.y;
        for para in &story.paragraphs {
            let style =
                doc.style(&para.style)
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
                    });
            let resolved = ob_model::resolve_fields(&para.text, doc);
            let spec = ob_text::shaping::FontSpec {
                family: style.font_family.clone(),
                bold: style.bold,
                italic: style.italic,
            };
            let line_h = style.font_size * 13 / 10;
            let shaped: Vec<(String, ob_model::Mpt)> = if resolved.contains('\t') {
                let line =
                    shaper.expand_order_line(&resolved, frame.rect.w, &spec, style.font_size);
                let w = shaper.measure(&line, &spec, style.font_size);
                vec![(line, w)]
            } else {
                shaper
                    .wrap(&resolved, frame.rect.w, &spec, style.font_size, 1.3)
                    .into_iter()
                    .map(|l| (l.text, l.width))
                    .collect()
            };
            for (line, tw) in shaped {
                if y + line_h > frame.rect.y + frame.rect.h {
                    break;
                }
                y += line_h;
                let lx = match style.align {
                    ob_model::Align::Left => frame.rect.x,
                    ob_model::Align::Center => frame.rect.x + (frame.rect.w - tw) / 2,
                    ob_model::Align::Right => frame.rect.x + frame.rect.w - tw,
                };
                lines.push(PreviewLine {
                    text: line,
                    size: style.font_size as f32 / 1000.0,
                    x: lx as f32 / 1000.0,
                    y: y as f32 / 1000.0,
                    w: tw as f32 / 1000.0,
                    align: style.align,
                });
            }
            y += style.space_after;
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

/// A finished PDF rescue waiting for the admin's verdict:
/// the draft, its review list, and the folder holding the
/// rendered page pictures.
struct ImportReview {
    doc: Document,
    items: Vec<ob_extract::draft::ImportItem>,
    workdir: std::path::PathBuf,
    source_name: String,
}

/// A file-open that would leave unsaved work behind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PendingOpen {
    Plain,
    LastWeek,
}

/// The instance lock: a file in the settings folder holding
/// the running app's process id. A second instance reads it to
/// know the journal already has an owner.
fn instance_lock_path() -> std::path::PathBuf {
    settings_path()
        .parent()
        .map(|d| d.join("openbulletin.lock"))
        .unwrap_or_else(|| std::path::PathBuf::from("openbulletin.lock"))
}

fn write_instance_lock() {
    let _ = std::fs::write(instance_lock_path(), std::process::id().to_string());
}

/// Is another OpenBulletin process alive? Proven on Unix via
/// /proc; elsewhere we cannot tell cheaply, so we answer no
/// rather than mute a lone instance's autosave.
fn peer_instance_alive() -> bool {
    let Ok(text) = std::fs::read_to_string(instance_lock_path()) else {
        return false;
    };
    let Ok(pid) = text.trim().parse::<u32>() else {
        return false;
    };
    if pid == std::process::id() {
        return false;
    }
    std::path::Path::new(&format!("/proc/{pid}")).exists()
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

fn cursor_paragraph(ctx: &egui::Context, text: &str, salt: &str) -> usize {
    let id = egui::Id::new(("canvas-edit", salt));
    if let Some(state) = egui::widgets::text_edit::TextEditState::load(ctx, id) {
        if let Some(range) = state.cursor.char_range() {
            let idx = range.primary.index;
            return text.chars().take(idx).filter(|c| *c == '\n').count();
        }
    }
    0
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
            };
            pp.lines = panel_lines(doc, panel, &mut shaper)
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
/// lines, panel outlines, panel names, and the shaped lines.
fn draw_sheet(ui: &mut egui::Ui, sheet: &PreviewSheet) {
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
        let clip = painter.with_clip_rect(prect);
        for line in &panel.lines {
            let anchor = match line.align {
                ob_model::Align::Left => egui::Align2::LEFT_BOTTOM,
                ob_model::Align::Center => egui::Align2::CENTER_BOTTOM,
                ob_model::Align::Right => egui::Align2::RIGHT_BOTTOM,
            };
            let lx = match line.align {
                ob_model::Align::Left => line.x,
                ob_model::Align::Center => line.x + line.w / 2.0,
                ob_model::Align::Right => line.x + line.w,
            };
            clip.text(
                to_px(lx, line.y),
                anchor,
                &line.text,
                egui::FontId::proportional((line.size * scale).max(4.0)),
                egui::Color32::BLACK,
            );
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
    let name = match (lw, lh) {
        (792_000, 612_000) => "Letter (11 x 8.5 in)".to_string(),
        (1_008_000, 612_000) => "Legal (14 x 8.5 in)".to_string(),
        (1_224_000, 792_000) => "Tabloid (17 x 11 in)".to_string(),
        _ => format!(
            "{:.1} x {:.1} in",
            lw as f32 / 72_000.0,
            lh as f32 / 72_000.0
        ),
    };
    format!("{name}, {orientation}")
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
        None => lines.push(format!(
            "Two-sided: we have not learned the {printer_name} yet. Print the \
             test sheet below once and answer, and this line will name the \
             exact setting."
        )),
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
            .map(|p| p.text.clone())
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
            .map(|p| p.text.clone())
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
            .map(|p| p.text.clone())
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
        let texts: Vec<&str> = app
            .doc
            .story(&sid)
            .expect("story")
            .paragraphs
            .iter()
            .map(|p| p.text.as_str())
            .collect();
        let joined = texts.join("|");
        assert!(
            joined.contains("First item||Second item"),
            "the blank line must survive as a paragraph break: {joined}"
        );
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
            .map(|p| p.text.clone())
            .unwrap_or_default();
        app.enter_edit(pi, fi);
        app.edit_text = "Line one\nLine two".to_string();
        app.sync_edit_styles();
        app.commit_edit();
        let texts: Vec<&str> = app
            .doc
            .story(&sid)
            .expect("story")
            .paragraphs
            .iter()
            .map(|p| p.text.as_str())
            .collect();
        assert_eq!(texts, vec!["Line one", "Line two"]);
        app.undo();
        let restored = app
            .doc
            .story(&sid)
            .expect("story after undo")
            .paragraphs
            .first()
            .map(|p| p.text.clone())
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
        write_journal_to(&obd, &meta, &doc, &source).expect("write journal");
        let (back, back_source, when) = read_journal_from(&obd, &meta).expect("read journal");
        assert_eq!(snapshot(&back), snapshot(&doc));
        assert_eq!(back_source, source);
        assert!(when > 0);
        clear_journal_from(&obd, &meta);
        assert!(read_journal_from(&obd, &meta).is_none());
        std::fs::remove_dir_all(&dir).ok();
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
}
