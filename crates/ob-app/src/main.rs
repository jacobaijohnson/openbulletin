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
    Some((
        doc,
        if source.is_empty() {
            None
        } else {
            Some(std::path::PathBuf::from(source))
        },
        when,
    ))
}

fn clear_journal_from(obd: &std::path::Path, meta: &std::path::Path) {
    let _ = std::fs::remove_file(obd);
    let _ = std::fs::remove_file(meta);
}

/// A stable snapshot string for one document state.
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
    step: usize,
    announcements: String,
    notes: Vec<String>,
    status: String,
    shaper: ob_text::shaping::Shaper,
    path: Option<std::path::PathBuf>,
    settings: Settings,
    printer_name: String,
    show_pages: bool,
    canvas_panel: usize,
    drag_guide: Option<usize>,
    ctx_pos: Option<egui::Pos2>,
    guide_input: String,
    guide_menu_for: Option<usize>,
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
    system_shown: bool,
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
            step: 0,
            announcements,
            notes: Vec::new(),
            status: "Welcome to OpenBulletin.".into(),
            shaper: ob_text::shaping::bundled_shaper(),
            path: None,
            settings: load_settings(),
            printer_name: "Church office printer".into(),
            show_pages: false,
            canvas_panel: 0,
            drag_guide: None,
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
            system_shown: false,
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
    fn rebuild(&mut self) {
        self.doc = build_template(
            self.preset,
            ChurchProfile {
                name: self.church_name.clone(),
                service_times: "Sundays 9:30 AM".into(),
                ..Default::default()
            },
            chrono::Local::now().date_naive(),
            "Sunday",
        );
        self.announcements = weekly_body_text(&self.doc);
        self.status = format!("Template loaded: {}", self.preset.label());
        self.mark_clean();
    }

    fn save_announcements(&mut self) {
        let Some(sid) = weekly_story_id(&self.doc) else {
            return;
        };
        let lines: Vec<String> = self
            .announcements
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect();
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
        // in plain words about any that do not fit.
        let mut fit_notes = Vec::new();
        for panel in &self.doc.panels {
            for frame in &panel.frames {
                let Some(sid) = &frame.story_id else { continue };
                let Some(story) = self.doc.story(sid) else {
                    continue;
                };
                let mut needed: ob_model::Mpt = 0;
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
                    let lines = if text.contains('\t') {
                        1
                    } else {
                        self.shaper
                            .wrap(&text, frame.rect.w, &spec, style.font_size, 1.3)
                            .len()
                    };
                    needed +=
                        lines as ob_model::Mpt * (style.font_size * 13 / 10) + style.space_after;
                }
                if needed > frame.rect.h {
                    let over_lines = ((needed - frame.rect.h) / 14_000).max(1);
                    let area = story
                        .paragraphs
                        .first()
                        .map(|p| p.text.clone())
                        .unwrap_or_else(|| "A text block".into());
                    fit_notes.push(format!(
                        "Check: the words in \"{}\" ({}) do not all fit. About {} line{} too many. Shorten the words, or we can shrink them in a later version.",
                        area.chars().take(38).collect::<String>(),
                        panel.name,
                        over_lines,
                        if over_lines == 1 { "" } else { "s" }
                    ));
                }
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
        self.save_announcements();
        match ob_layout::next_sunday(&self.doc) {
            Ok(next) => {
                self.doc = next;
                self.announcements = weekly_body_text(&self.doc);
                self.mark_clean();
                self.step = 1;
                self.status = format!(
                    "Started {} bulletin. Weekly announcements were cleared; standing items carried over.",
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
        let (obd, meta) = journal_paths();
        if let Some((doc, source, when)) = read_journal_from(&obd, &meta) {
            if snapshot(&doc) != self.saved_snapshot {
                self.journal_snapshot = snapshot(&doc);
                self.recovery = Some((doc, source, when));
            } else {
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
        if snap == self.saved_snapshot {
            if snap != self.journal_snapshot {
                self.journal_snapshot = snap;
                let (obd, meta) = journal_paths();
                clear_journal_from(&obd, &meta);
            }
            return;
        }
        if snap != self.journal_snapshot {
            let (obd, meta) = journal_paths();
            if write_journal_to(&obd, &meta, &self.doc, &self.path).is_ok() {
                self.journal_snapshot = snap;
            }
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
                ui.horizontal(|ui| {
                    if ui.button("Bring it back").clicked() {
                        bring_back = true;
                    }
                    if ui.button("Throw it away").clicked() {
                        throw_away = true;
                    }
                });
            });
        if bring_back {
            if let Some((doc, source, _)) = self.recovery.take() {
                self.doc = doc;
                self.path = source;
                self.announcements = weekly_body_text(&self.doc);
                self.church_name = self.doc.profile.name.clone();
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
        let key = snapshot(&self.doc);
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
                        self.canvas_panel -= 1;
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
                        self.canvas_panel += 1;
                    }
                });
                ui.label("Drag a blue line to move it. Right-click a line to type its exact place, or right-click the page to add a line there. Guides never print.");
                ui.separator();
                self.panel_canvas(ui, idx, ctx);
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
        let scale = (avail_w / pw).min(760.0 / ph).min(2.0);
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
                    .unwrap_or_default(),
                bg
            );
            if self.bg_tex_key != key {
                self.bg_tex_key = key.clone();
                self.bg_tex = self.load_bg_texture(ctx, bg);
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
        let guides_now = self.doc.panels[idx].guides.clone();
        let near_guide = |pos: egui::Pos2| -> Option<usize> {
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
                if d <= 5.0 && best.map(|(_, bd)| d < bd).unwrap_or(true) {
                    best = Some((gi, d));
                }
            }
            best.map(|(gi, _)| gi)
        };
        if response.drag_started() {
            if let Some(pos) = pointer {
                self.drag_guide = near_guide(pos);
            }
        }
        if response.dragged() {
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
        }
        if response.drag_stopped() {
            self.drag_guide = None;
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
        for (gi, g) in self.doc.panels[idx].guides.iter().enumerate() {
            let dragging = self.drag_guide == Some(gi);
            let stroke = egui::Stroke::new(if dragging { 2.0_f32 } else { 1.25_f32 }, guide_color);
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

        // Right-click: exact position for a guide, or add one here.
        let hit = self.ctx_pos.and_then(near_guide);
        response.context_menu(|ui| {
            if let Some(gi) = hit {
                let g = self.doc.panels[idx].guides[gi];
                if self.guide_menu_for != Some(gi) {
                    self.guide_menu_for = Some(gi);
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
                if ui.button("Delete this guide").clicked() {
                    self.doc.panels[idx].guides.remove(gi);
                    self.status = "Guide deleted.".into();
                    ui.close_menu();
                }
            } else {
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
                    self.doc.panels[idx].guides.push(ob_model::Guide {
                        vertical: true,
                        pos: x_mpt.clamp(0, panel.size.w),
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
                    self.doc.panels[idx].guides.push(ob_model::Guide {
                        vertical: false,
                        pos: y_mpt.clamp(0, panel.size.h),
                    });
                    ui.close_menu();
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
            .unwrap_or_default();
        let bytes = std::fs::read(base.join(rel)).ok()?;
        let img = image::load_from_memory(&bytes).ok()?.to_rgba8();
        let (w, h) = img.dimensions();
        let color =
            egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], img.as_raw());
        Some(ctx.load_texture("trace-bg", color, egui::TextureOptions::LINEAR))
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
                ui.label(
                    "These are the fonts this app may use. Bulletins are laid out                      only with fonts from this library, so they look the same on                      every computer.",
                );
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
                    self.system_rows = ob_fonts::system_fonts()
                        .into_iter()
                        .filter(|f| !in_library.contains(&f.info.sha256))
                        .collect();
                    self.system_shown = true;
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
        self.journal_tick();
        self.recovery_window(ctx);
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("OpenBulletin");
                if ui.button("Pages").clicked() {
                    self.show_pages = true;
                }
                if ui.button("Fonts").clicked() {
                    self.show_fonts = true;
                }
                if ui.button("Open bulletin...").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("OpenBulletin bulletin", &["obd"])
                        .pick_file()
                    {
                        match ob_format::load(&path) {
                            Ok(doc) => {
                                self.announcements = weekly_body_text(&doc);
                                self.church_name = doc.profile.name.clone();
                                self.doc = doc;
                                self.path = Some(path);
                                self.step = 1;
                                self.status = "Bulletin opened.".into();
                                self.mark_clean();
                            }
                            Err(e) => self.status = format!("Could not open that file: {e}"),
                        }
                    }
                }
                if ui.button("Save bulletin").clicked() {
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
                                self.status = format!("Saved to {}", path.display());
                                self.path = Some(path);
                                self.mark_clean();
                            }
                            Err(e) => self.status = format!("Could not save: {e}"),
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
                        if self.step == 2 {
                            self.save_announcements();
                        }
                        self.step += 1;
                        if self.step == 3 {
                            self.run_checks();
                        }
                    }
                    if self.step > 0 && ui.button("Back").clicked() {
                        self.step -= 1;
                    }
                });
            });
            ui.label(GUIDANCE[self.step]);
        });

        egui::SidePanel::left("steps")
            .resizable(false)
            .default_width(300.0)
            .show(ctx, |ui| {
                for (i, name) in STEPS.iter().enumerate() {
                    let marker = if i == self.step {
                        ">"
                    } else if i < self.step {
                        "done -"
                    } else {
                        ""
                    };
                    ui.label(egui::RichText::new(format!("{marker} {}. {name}", i + 1)).strong_if(i == self.step));
                }
                ui.separator();
                match self.step {
                    0 => {
                        ui.heading("Bulletin shape");
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
                            self.doc.profile.name = name;
                        }
                    }
                    1 => {
                        ui.heading("This Sunday");
                        ui.label(format!(
                            "Date: {}",
                            self.doc.fields.get("sunday_date_long").cloned().unwrap_or_default()
                        ));
                        ui.label("Name of the Sunday");
                        let mut sn = self.doc.fields.get("sunday_name").cloned().unwrap_or_default();
                        if ui.text_edit_singleline(&mut sn).lost_focus() {
                            self.doc.fields.insert("sunday_name".to_string(), sn);
                        }
                        ui.label("Scripture reading");
                        let mut rd = self.doc.fields.get("reading").cloned().unwrap_or_default();
                        if ui.text_edit_singleline(&mut rd).lost_focus() {
                            self.doc.fields.insert("reading".to_string(), rd);
                        }
                        ui.small("The cover and the order of service fill themselves from these.");
                    }
                    2 => {
                        ui.heading("Announcements");
                        ui.label("One announcement per line. Paste from email works; formatting is cleaned up for you.");
                        ui.add(
                            egui::TextEdit::multiline(&mut self.announcements)
                                .desired_rows(12)
                                .desired_width(f32::INFINITY),
                        );
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
                        if ui.button("Open the print copy").clicked() {
                            self.pin_fonts();
                            let bytes = ob_pdf::export_imposed_pdf(&self.doc);
                            let path = std::env::temp_dir().join("openbulletin-print-copy.pdf");
                            match std::fs::write(&path, &bytes).map_err(|e| e.to_string()).and_then(|_| open_with_os(&path)) {
                                Ok(()) => self.status = "Print copy opened in your PDF viewer. Use the settings listed above.".into(),
                                Err(e) => self.status = e,
                            }
                        }
                        ui.separator();
                        ui.label("Print copy: folded sheets, in the right order, ready for the copier.");
                        if ui.button(egui::RichText::new("Save print copy (PDF)").strong()).clicked() {
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
                ui.small(&self.status);
            });

        self.pages_window(ctx);
        self.fonts_window(ctx);
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label(egui::RichText::new("Preview: what the congregation will hold").strong());
            egui::ScrollArea::vertical().show(ui, |ui| {
                for panel in &self.doc.panels {
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
                        if let Some(sid) = &frame.story_id {
                            if let Some(story) = self.doc.story(sid) {
                                let mut y = fr.min.y + 4.0;
                                for para in &story.paragraphs {
                                    let style = self.doc.style(&para.style);
                                    let fs =
                                        style.map(|s| s.font_size as f32 / 1000.0).unwrap_or(11.0);
                                    let text = ob_model::resolve_fields(&para.text, &self.doc);
                                    let text = text.replace('\t', "  ");
                                    painter.text(
                                        egui::pos2(fr.min.x + 6.0, y),
                                        egui::Align2::LEFT_TOP,
                                        text.chars().take(64).collect::<String>(),
                                        egui::FontId::proportional(fs * scale + 3.0),
                                        egui::Color32::BLACK,
                                    );
                                    y += (fs * scale + 3.0) * 1.35;
                                    if y > fr.max.y - 10.0 {
                                        break;
                                    }
                                }
                            }
                        }
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
fn build_preview(doc: &Document) -> Vec<PreviewSheet> {
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
                    let style = doc.style(&para.style).cloned().unwrap_or_else(|| {
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
                    let resolved = ob_model::resolve_fields(&para.text, doc);
                    let spec = ob_text::shaping::FontSpec {
                        family: style.font_family.clone(),
                        bold: style.bold,
                        italic: style.italic,
                    };
                    let line_h = style.font_size * 13 / 10;
                    let shaped: Vec<(String, ob_model::Mpt)> = if resolved.contains('\t') {
                        let line = shaper.expand_order_line(
                            &resolved,
                            frame.rect.w,
                            &spec,
                            style.font_size,
                        );
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
                        pp.lines.push(PreviewLine {
                            text: line,
                            size: style.font_size as f32 / 1000.0,
                            x: pp.x + lx as f32 / 1000.0,
                            y: pp.y + y as f32 / 1000.0,
                            w: tw as f32 / 1000.0,
                            align: style.align,
                        });
                    }
                    y += style.space_after;
                }
            }
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
        Some(setup) if setup.mode == "long_edge" => lines.push(
            "Two-sided: On, and pick \"flip on long edge\". Your test sheet \
             proved this is right for this printer."
                .to_string(),
        ),
        Some(setup) if setup.mode == "short_edge" => lines.push(
            "Two-sided: On, and pick \"flip on short edge\". Your test sheet \
             proved long edge comes out upside down on this printer."
                .to_string(),
        ),
        Some(_) => lines.push(
            "Two-sided: this printer does one side at a time. Print all the \
             front sides, put the stack back in the tray the same way it came \
             out, then print the back sides. Try one sheet first."
                .to_string(),
        ),
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
