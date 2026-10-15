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

struct App {
    doc: Document,
    preset: Preset,
    church_name: String,
    step: usize,
    announcements: String,
    notes: Vec<String>,
    status: String,
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

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("OpenBulletin");
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
                        ui.label("Print copy: folded sheets, in the right order, ready for the copier (Export in the app lands next; today the same PDF comes from the openbulletin-cli export --imposed command).");
                        ui.label("Reading copy: one panel per page, for email and the website (export without --imposed).");
                        ui.separator();
                        if ui.button(egui::RichText::new("Start next Sunday").strong()).clicked() {
                            self.next_sunday();
                        }
                    }
                }
                ui.separator();
                ui.small(&self.status);
            });

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
