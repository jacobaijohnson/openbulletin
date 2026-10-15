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
    show_fonts: bool,
    fonts_loaded: bool,
    font_rows: Vec<FontRow>,
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
            show_fonts: false,
            fonts_loaded: false,
            font_rows: Vec::new(),
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
                        italic: false,
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

    fn fonts_window(&mut self, ctx: &egui::Context) {
        if !self.show_fonts {
            return;
        }
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
                if ui.button("Add a font...").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Font files", &["ttf", "otf"])
                        .pick_file()
                    {
                        match std::fs::read(&path) {
                            Ok(bytes) => {
                                let name = path
                                    .file_name()
                                    .map(|n| n.to_string_lossy().to_string())
                                    .unwrap_or_else(|| "font.ttf".into());
                                match ob_fonts::FontLibrary::open(
                                    ob_fonts::default_library_dir(),
                                )
                                .and_then(|lib| lib.add_bytes(&bytes, &name))
                                {
                                    Ok(info) => {
                                        self.status =
                                            format!("Font added: {}", info.full_name);
                                        self.refresh_fonts(ctx);
                                    }
                                    Err(e) => {
                                        self.status = format!("Could not add that font: {e}")
                                    }
                                }
                            }
                            Err(e) => {
                                self.status = format!("Could not read that file: {e}")
                            }
                        }
                    }
                }
                ui.label(
                    egui::RichText::new(
                        "Free fonts can also be fetched from the catalog with the                          command line for now: openbulletin-cli fonts search / get.",
                    )
                    .small(),
                );
            });
        self.show_fonts = open;
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("OpenBulletin");
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
                            }
                            Err(e) => self.status = format!("Could not open that file: {e}"),
                        }
                    }
                }
                if ui.button("Save bulletin").clicked() {
                    self.save_announcements();
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
                        ui.label("Print copy: folded sheets, in the right order, ready for the copier.");
                        if ui.button(egui::RichText::new("Save print copy (PDF)").strong()).clicked() {
                            self.save_announcements();
                            if let Some(path) = rfd::FileDialog::new()
                                .set_file_name("bulletin-print.pdf")
                                .add_filter("PDF", &["pdf"])
                                .save_file()
                            {
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
