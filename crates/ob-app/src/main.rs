//! OpenBulletin desktop shell.
//!
//! Design law (docs/PRINCIPLES.md): KISS, built for church office
//! administrators, not layout professionals. The home screen is a guided
//! weekly flow with one clear next step at a time:
//!   1 Start  2 This Sunday  3 Announcements  4 Check  5 Print
//! The engine lives in the ob-* crates and the CLI; this shell stays thin.

use eframe::egui;
use ob_layout::{build_template, Preset};
use ob_model::{merge_rights_records, ChurchProfile, ContentRole, Document};

mod editmap;
mod runsedit;
mod starter_clips;

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

/// How large the app's own words and controls are (A1,
/// docs/planning/beyond-print.md section 5): the app
/// conforms to the administrator's eyes. Three steps, in
/// the words the comfort window offers. The document is
/// never touched by this: a bulletin's words keep the
/// sizes in the document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum UiScale {
    #[default]
    Standard,
    Large,
    ExtraLarge,
}

impl UiScale {
    /// The words the comfort window shows for this step.
    pub fn label(self) -> &'static str {
        match self {
            UiScale::Standard => "Standard",
            UiScale::Large => "Large",
            UiScale::ExtraLarge => "Extra large",
        }
    }

    /// Every text style grows by this much.
    pub fn text_factor(self) -> f32 {
        match self {
            UiScale::Standard => 1.0,
            UiScale::Large => 1.2,
            UiScale::ExtraLarge => 1.4,
        }
    }

    /// Padding and spacing grow a step behind the words.
    pub fn spacing_factor(self) -> f32 {
        match self {
            UiScale::Standard => 1.0,
            UiScale::Large => 1.15,
            UiScale::ExtraLarge => 1.3,
        }
    }
}

/// The palette in force (A1): the standard constants
/// above, or the high-contrast variants when the
/// administrator asks for true black on white. Painters
/// read colors from the App's theme, never from the
/// constants directly, so one switch repaints the app.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Theme {
    pub problem: egui::Color32,
    pub warn: egui::Color32,
    pub good: egui::Color32,
    pub info: egui::Color32,
    pub accent: egui::Color32,
    /// Ordinary words, and the ground they sit on: the
    /// pair the contrast promise is made about.
    pub text: egui::Color32,
    pub ground: egui::Color32,
}

impl Theme {
    pub fn standard() -> Theme {
        Theme {
            problem: PROBLEM,
            warn: WARN,
            good: GOOD,
            info: INFO,
            accent: CANVAS_ACCENT,
            // egui's light chrome, as shipped today.
            text: egui::Color32::from_gray(80),
            ground: egui::Color32::from_gray(248),
        }
    }

    /// True black words on a white ground. Every accent is
    /// darkened until it holds up as words on white (the
    /// ratios are asserted in the tests at the bottom of
    /// this file), and the standard mid-tones stay
    /// untouched for the standard theme.
    pub fn high_contrast() -> Theme {
        Theme {
            problem: egui::Color32::from_rgb(160, 0, 0),
            warn: egui::Color32::from_rgb(122, 66, 0),
            good: egui::Color32::from_rgb(0, 100, 0),
            info: egui::Color32::from_rgb(0, 0, 200),
            accent: egui::Color32::from_rgb(0, 90, 170),
            text: egui::Color32::BLACK,
            ground: egui::Color32::WHITE,
        }
    }

    pub fn for_settings(settings: &Settings) -> Theme {
        if settings.high_contrast {
            Theme::high_contrast()
        } else {
            Theme::standard()
        }
    }
}

/// WCAG relative luminance of an opaque color.
pub fn relative_luminance(c: egui::Color32) -> f32 {
    fn lin(v: u8) -> f32 {
        let v = v as f32 / 255.0;
        if v <= 0.03928 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    }
    0.2126 * lin(c.r()) + 0.7152 * lin(c.g()) + 0.0722 * lin(c.b())
}

/// The contrast ratio between two opaque colors, 1 to 21.
pub fn contrast_ratio(a: egui::Color32, b: egui::Color32) -> f32 {
    let (la, lb) = (relative_luminance(a), relative_luminance(b));
    let (hi, lo) = if la >= lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

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

/// What the app remembers about one bulletin file's state
/// (J6): which document snapshot its print copy was made from,
/// and which snapshot last passed Check with nothing red.
/// Keyed by file path in Settings, so the Start step can say
/// "print copy is current" or "not checked since the last
/// change" across restarts, not just within one sitting.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DocState {
    #[serde(default)]
    pub print_fingerprint: Option<String>,
    #[serde(default)]
    pub checked_fingerprint: Option<String>,
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub printers: std::collections::BTreeMap<String, PrinterSetup>,
    /// The keyboard preset last chosen ("" means ours). The
    /// preset only SETS the defaults: any change to a key
    /// below moves the set to Custom (see keymap_status).
    #[serde(default)]
    pub keymap_preset: String,
    /// Per-action key overrides on top of the preset.
    #[serde(default)]
    pub keymap: std::collections::BTreeMap<String, String>,
    /// The bulletin file this office last opened or saved,
    /// so the Start step can offer it back without a hunt
    /// through the file dialog (UI audit round 2).
    #[serde(default)]
    pub last_bulletin: Option<String>,
    /// Per-file document state (J6), most recent last,
    /// capped at 20 files: enough for an office's rotation,
    /// small enough to stay a memory, not an archive.
    #[serde(default)]
    pub doc_states: Vec<(String, DocState)>,
    /// The step the user left the app on, restored at launch
    /// (J6), clamped to the step list.
    #[serde(default)]
    pub last_step: usize,
    /// The purpose last chosen for a saved PDF copy (an
    /// ob_pdf::ExportPurpose slug). "" means the chooser's
    /// default, For printing (O1).
    #[serde(default)]
    pub pdf_purpose: String,
    /// How large the app's own words and controls are
    /// (A1). Missing from an older settings file means
    /// Standard.
    #[serde(default)]
    pub ui_scale: UiScale,
    /// True when the app paints true black on white with
    /// darkened accents (A1). Missing means off.
    #[serde(default)]
    pub high_contrast: bool,
}

impl Settings {
    /// The remembered state of one bulletin file, if any.
    pub fn doc_state(&self, path: &str) -> Option<&DocState> {
        self.doc_states
            .iter()
            .find(|(p, _)| p == path)
            .map(|(_, s)| s)
    }

    /// The purpose a saved PDF copy is made for: the remembered
    /// choice, or the chooser's default (For printing) when the
    /// saved word is missing or not a purpose we know.
    pub fn pdf_purpose(&self) -> ob_pdf::ExportPurpose {
        ob_pdf::ExportPurpose::from_slug(&self.pdf_purpose)
            .unwrap_or(ob_pdf::ExportPurpose::ForPrinting)
    }

    /// Remember one file's state, most recent last; beyond
    /// 20 files the oldest memory is dropped.
    pub fn set_doc_state(&mut self, path: String, state: DocState) {
        self.doc_states.retain(|(p, _)| p != &path);
        self.doc_states.push((path, state));
        while self.doc_states.len() > 20 {
            self.doc_states.remove(0);
        }
    }
}

/// The keys the app answers to, as (action, plain label).
pub const KEY_ACTIONS: &[(&str, &str)] = &[
    ("undo", "Undo"),
    ("redo", "Redo"),
    ("save", "Save bulletin"),
    ("open", "Open a bulletin"),
    ("new", "New bulletin"),
    ("pages", "Pages window (open or close)"),
    ("fonts", "Fonts window (open or close)"),
    ("next_step", "Go to the next step"),
    ("prev_step", "Go back a step"),
    ("copy_box", "Copy the selected box (Pages window)"),
    ("paste_box", "Paste a box (Pages window)"),
    ("next_page", "Next page (Pages window)"),
    ("prev_page", "Previous page (Pages window)"),
];

/// A preset is only a set of starting defaults: choosing one
/// fills the table, and from there the keys are yours.
pub fn preset_defaults(preset: &str) -> std::collections::BTreeMap<String, String> {
    let mut m: std::collections::BTreeMap<String, String> = [
        ("undo", "Ctrl+Z"),
        ("save", "Ctrl+S"),
        ("open", "Ctrl+O"),
        ("new", "Ctrl+N"),
        ("copy_box", "Ctrl+C"),
        ("paste_box", "Ctrl+V"),
    ]
    .iter()
    .map(|(a, b)| (a.to_string(), b.to_string()))
    .collect();
    match preset {
        "publisher" => {
            m.insert("redo".into(), "Ctrl+Y".into());
            m.insert("pages".into(), "Alt+P".into());
            m.insert("fonts".into(), "Ctrl+Shift+F".into());
            m.insert("next_step".into(), "Ctrl+Tab".into());
            m.insert("prev_step".into(), "Ctrl+Shift+Tab".into());
            m.insert("next_page".into(), "Ctrl+PageDown".into());
            m.insert("prev_page".into(), "Ctrl+PageUp".into());
        }
        "indesign" => {
            m.insert("redo".into(), "Ctrl+Shift+Z".into());
            m.insert("pages".into(), "F12".into());
            m.insert("fonts".into(), "Ctrl+T".into());
            m.insert("next_step".into(), "Ctrl+Right".into());
            m.insert("prev_step".into(), "Ctrl+Left".into());
            m.insert("next_page".into(), "PageDown".into());
            m.insert("prev_page".into(), "PageUp".into());
        }
        _ => {
            m.insert("redo".into(), "Ctrl+Y".into());
            m.insert("pages".into(), "Alt+P".into());
            m.insert("fonts".into(), "Alt+F".into());
            m.insert("next_step".into(), "Ctrl+Right".into());
            m.insert("prev_step".into(), "Ctrl+Left".into());
            m.insert("next_page".into(), "PageDown".into());
            m.insert("prev_page".into(), "PageUp".into());
        }
    }
    m
}

/// The keys actually in force: preset defaults plus overrides.
pub fn effective_keymap(settings: &Settings) -> std::collections::BTreeMap<String, String> {
    let preset = if settings.keymap_preset.is_empty() {
        "openbulletin"
    } else {
        settings.keymap_preset.as_str()
    };
    let mut m = preset_defaults(preset);
    for (action, binding) in &settings.keymap {
        m.insert(action.clone(), binding.clone());
    }
    m
}

/// Jacob's rule, exactly: a preset names the set only while
/// the keys match its defaults. Change any key and the set is
/// Custom; change it back and the name returns.
pub fn keymap_status(settings: &Settings) -> &'static str {
    let eff = effective_keymap(settings);
    if eff == preset_defaults("openbulletin") {
        "OpenBulletin defaults"
    } else if eff == preset_defaults("publisher") {
        "Publisher defaults"
    } else if eff == preset_defaults("indesign") {
        "InDesign defaults"
    } else {
        "Custom"
    }
}

/// The capture discipline, applied to keymap DATA on load
/// (round 2, R2-5). A settings file is data: it may be hand
/// edited, or carried from another machine or version, and
/// it gets the same rules a captured key gets.
/// - An entry naming an action we do not have is dropped:
///   it could never fire, and it must not poison the set's
///   name to "Custom" forever.
/// - An entry whose press does not read as a key press is
///   dropped: the action falls back to the preset's key
///   instead of sitting silently dead.
/// - A press claimed by two actions is settled the way
///   capture settles it (see capture_key): the overrides are
///   applied in the canonical action order, the later claim
///   takes the press, and the earlier holder is given the
///   later action's previous press, a swap, so no press ever
///   fires two actions and no action is silently left
///   without one.
///
/// The stored overrides are rewritten to the smallest set
/// that reproduces the settled keys, so the file says what
/// the keys are from then on.
pub fn validate_keymap(settings: &mut Settings) {
    settings
        .keymap
        .retain(|action, _| KEY_ACTIONS.iter().any(|(a, _)| a == action));
    settings
        .keymap
        .retain(|_, text| parse_binding(text).is_some());
    let preset = if settings.keymap_preset.is_empty() {
        "openbulletin"
    } else {
        settings.keymap_preset.as_str()
    };
    let defaults = preset_defaults(preset);
    let mut working = defaults.clone();
    for (action, _) in KEY_ACTIONS {
        let Some(want) = settings.keymap.get(*action).cloned() else {
            continue;
        };
        if working.get(*action) == Some(&want) {
            continue;
        }
        let previous = working.get(*action).cloned().unwrap_or_default();
        let holder = working
            .iter()
            .find(|(a, p)| a.as_str() != *action && **p == want)
            .map(|(a, _)| a.clone());
        if let Some(holder) = holder {
            working.insert(holder, previous);
        }
        working.insert(action.to_string(), want);
    }
    settings.keymap = working
        .into_iter()
        .filter(|(a, p)| defaults.get(a) != Some(p))
        .collect();
}

/// One key press, as stored: "Ctrl+Shift+Z", "Alt+P", "F12".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyBinding {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub key: egui::Key,
}

pub fn key_name(key: egui::Key) -> Option<&'static str> {
    use egui::Key::*;
    Some(match key {
        A => "A",
        B => "B",
        C => "C",
        D => "D",
        E => "E",
        F => "F",
        G => "G",
        H => "H",
        I => "I",
        J => "J",
        K => "K",
        L => "L",
        M => "M",
        N => "N",
        O => "O",
        P => "P",
        Q => "Q",
        R => "R",
        S => "S",
        T => "T",
        U => "U",
        V => "V",
        W => "W",
        X => "X",
        Y => "Y",
        Z => "Z",
        Num0 => "0",
        Num1 => "1",
        Num2 => "2",
        Num3 => "3",
        Num4 => "4",
        Num5 => "5",
        Num6 => "6",
        Num7 => "7",
        Num8 => "8",
        Num9 => "9",
        F1 => "F1",
        F2 => "F2",
        F3 => "F3",
        F4 => "F4",
        F5 => "F5",
        F6 => "F6",
        F7 => "F7",
        F8 => "F8",
        F9 => "F9",
        F10 => "F10",
        F11 => "F11",
        F12 => "F12",
        F13 => "F13",
        F14 => "F14",
        F15 => "F15",
        F16 => "F16",
        F17 => "F17",
        F18 => "F18",
        F19 => "F19",
        F20 => "F20",
        Enter => "Enter",
        Tab => "Tab",
        PageUp => "PageUp",
        PageDown => "PageDown",
        Space => "Space",
        ArrowLeft => "Left",
        ArrowRight => "Right",
        ArrowUp => "Up",
        ArrowDown => "Down",
        _ => return None,
    })
}

pub fn name_key(name: &str) -> Option<egui::Key> {
    use egui::Key::*;
    Some(match name {
        "A" => A,
        "B" => B,
        "C" => C,
        "D" => D,
        "E" => E,
        "F" => F,
        "G" => G,
        "H" => H,
        "I" => I,
        "J" => J,
        "K" => K,
        "L" => L,
        "M" => M,
        "N" => N,
        "O" => O,
        "P" => P,
        "Q" => Q,
        "R" => R,
        "S" => S,
        "T" => T,
        "U" => U,
        "V" => V,
        "W" => W,
        "X" => X,
        "Y" => Y,
        "Z" => Z,
        "0" => Num0,
        "1" => Num1,
        "2" => Num2,
        "3" => Num3,
        "4" => Num4,
        "5" => Num5,
        "6" => Num6,
        "7" => Num7,
        "8" => Num8,
        "9" => Num9,
        "F1" => F1,
        "F2" => F2,
        "F3" => F3,
        "F4" => F4,
        "F5" => F5,
        "F6" => F6,
        "F7" => F7,
        "F8" => F8,
        "F9" => F9,
        "F10" => F10,
        "F11" => F11,
        "F12" => F12,
        "F13" => F13,
        "F14" => F14,
        "F15" => F15,
        "F16" => F16,
        "F17" => F17,
        "F18" => F18,
        "F19" => F19,
        "F20" => F20,
        "Enter" => Enter,
        "Tab" => Tab,
        "PageUp" => PageUp,
        "PageDown" => PageDown,
        "Space" => Space,
        "Left" => ArrowLeft,
        "Right" => ArrowRight,
        "Up" => ArrowUp,
        "Down" => ArrowDown,
        _ => return None,
    })
}

pub fn parse_binding(text: &str) -> Option<KeyBinding> {
    let mut b = KeyBinding {
        ctrl: false,
        shift: false,
        alt: false,
        key: egui::Key::A,
    };
    let parts: Vec<&str> = text.split('+').collect();
    let (mods, key_part) = parts.split_at(parts.len().checked_sub(1)?);
    for m in mods {
        match m.trim().to_lowercase().as_str() {
            "ctrl" | "control" | "cmd" | "command" => b.ctrl = true,
            "shift" => b.shift = true,
            "alt" | "option" => b.alt = true,
            _ => return None,
        }
    }
    b.key = name_key(key_part[0].trim())?;
    Some(b)
}

pub fn format_binding(b: &KeyBinding) -> String {
    let mut out = String::new();
    if b.ctrl {
        out.push_str("Ctrl+");
    }
    if b.shift {
        out.push_str("Shift+");
    }
    if b.alt {
        out.push_str("Alt+");
    }
    out.push_str(key_name(b.key).unwrap_or("?"));
    out
}

/// Does this input state match the binding right now? "Ctrl"
/// reads as egui's command flag, so it is Cmd on a Mac.
pub fn binding_pressed(i: &egui::InputState, b: &KeyBinding) -> bool {
    i.key_pressed(b.key)
        && i.modifiers.command == b.ctrl
        && i.modifiers.shift == b.shift
        && i.modifiers.alt == b.alt
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

/// A journal that exists but cannot be read (audit Batch 3):
/// move what is there out of the journal's place, under a
/// name that says what happened, so the next autosave starts
/// clean and the damaged copy is still on disk for anyone who
/// goes looking. Returns true when anything was set aside.
/// Renames replace an older set-aside: one damaged copy kept
/// is enough.
fn set_aside_unreadable_journal(obd: &std::path::Path, meta: &std::path::Path) -> bool {
    let mut moved = false;
    for (from, name) in [
        (obd, "autosave-unreadable.obd"),
        (meta, "autosave-unreadable.txt"),
    ] {
        if from.exists() {
            let to = from.with_file_name(name);
            if std::fs::rename(from, &to).is_ok() {
                moved = true;
            }
        }
    }
    moved
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
                self.doc.fields.insert(
                    "occasion_date_long".to_string(),
                    nd.format("%A, %B %-d, %Y").to_string(),
                );
                if before != snapshot(&self.doc) {
                    self.record_undo(before);
                }
                self.note(format!("Date set to {}.", nd.format("%A, %B %-d, %Y")));
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
                        d.format("%B %-d, %Y")
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

/// One Check-step note. A note that knows which panel and
/// frame it is about carries them as a target: the Check
/// step puts a "Fix this" button under it that opens the
/// Pages window on that panel with that frame selected
/// (UI audit round 2).
#[derive(Debug, Clone, PartialEq)]
struct CheckNote {
    text: String,
    /// (panel index, frame index) the note points at.
    target: Option<(usize, Option<usize>)>,
    /// A step the fix button jumps to instead of the Pages
    /// window, when the fix lives on a step (the date
    /// finding's fix lives on This Sunday, J1).
    step: Option<usize>,
    /// The fix lives in the Fonts window (a wanted font
    /// still missing): the button opens it.
    fonts: bool,
    /// The fix lives in the Loose sheets window (a loose
    /// sheet with no words, a dated sheet past its date):
    /// the button opens it.
    inserts: bool,
    /// A hymn pairing the note disagrees with, named by
    /// its hymn-memory key: the Check step puts a "Forget
    /// this pairing" button under the note that removes
    /// the pair from the church's hymn memory, for when
    /// the office says the memory is the wrong one
    /// (round 3, R3-6).
    forget_hymn: Option<String>,
}

impl CheckNote {
    fn plain(text: String) -> Self {
        CheckNote {
            text,
            target: None,
            step: None,
            fonts: false,
            inserts: false,
            forget_hymn: None,
        }
    }

    fn aimed(text: String, panel: usize, frame: Option<usize>) -> Self {
        CheckNote {
            text,
            target: Some((panel, frame)),
            step: None,
            fonts: false,
            inserts: false,
            forget_hymn: None,
        }
    }

    fn to_step(text: String, step: usize) -> Self {
        CheckNote {
            text,
            target: None,
            step: Some(step),
            fonts: false,
            inserts: false,
            forget_hymn: None,
        }
    }

    fn to_fonts(text: String) -> Self {
        CheckNote {
            text,
            target: None,
            step: None,
            fonts: true,
            inserts: false,
            forget_hymn: None,
        }
    }

    fn to_inserts(text: String) -> Self {
        CheckNote {
            text,
            target: None,
            step: None,
            fonts: false,
            inserts: true,
            forget_hymn: None,
        }
    }
}

/// One story whose words use a character the print file cannot
/// show (GPT workflow review, adopted item 1). The character
/// test itself is ob-pdf's own, so this warning and the printed
/// output can never disagree.
#[derive(Debug, Clone, PartialEq)]
struct CharFinding {
    story_id: String,
    /// The story's first words, so the admin can find it.
    label: String,
    /// The unprintable characters, in the order they appear.
    chars: Vec<char>,
    /// The first frame holding this story, when it sits in
    /// one: the Check step's fix button opens the Pages
    /// window there.
    target: Option<(usize, Option<usize>)>,
    /// A catalog face that verifiably carries one of the
    /// missing marks (its downloaded cmap was checked, see
    /// ob_fonts::catalog): the character and the family, so
    /// the note can point at the way out instead of only
    /// naming the problem.
    helper: Option<(char, String)>,
}

/// Every story in the document that uses characters the print
/// copy cannot draw in the face that would print them. Fields
/// are filled in first, because the filled-in words are what
/// actually prints; each run is tested against its own face
/// (the paragraph style's family with the run's bold/italic),
/// resolved exactly the way the PDF writer resolves it, so a
/// character is only named when that face truly has no glyph
/// for it.
/// The Check step's wanted-font findings (docs/FONTS.md):
/// for each font the document was made in that the
/// library still does not hold exactly, one finding in
/// the import review's own words, with the stand-in the
/// chain picks TODAY (a nearer font added since the
/// import is named, not the stale one on the record).
/// Only families a style still names are reported
/// (round 3, R3-16): restyle the whole bulletin away
/// from a wanted font and the record stays on the file
/// but the finding goes silent, because nothing prints
/// in that font anymore. Pure over (document, available
/// families) so the words are testable without a library
/// on disk.
fn wanted_font_notes(doc: &Document, available: &[String]) -> Vec<CheckNote> {
    let mut out = Vec::new();
    let used: Vec<String> = doc
        .styles
        .iter()
        .map(|s| s.font_family.clone())
        .chain(
            doc.inserts
                .iter()
                .flat_map(|ins| ins.styles.iter().map(|s| s.font_family.clone())),
        )
        .collect();
    // How far from Exact a resolution landed, worst
    // last: the finding a style earns is the worst one
    // among the styles that use its family (round 5,
    // R5-D).
    fn tier_rank(tier: ob_fonts::resolve::SubstituteTier) -> u8 {
        match tier {
            ob_fonts::resolve::SubstituteTier::Exact => 0,
            ob_fonts::resolve::SubstituteTier::MetricTwin => 1,
            ob_fonts::resolve::SubstituteTier::Lookalike => 2,
            ob_fonts::resolve::SubstituteTier::HouseDefault => 3,
        }
    }
    for w in &doc.wanted_fonts {
        // The record names the family in display form;
        // the styles that still use it carry the full
        // source name. Resolution keys off a style's full
        // name, not the record (round 4, R4-1): the two
        // can differ ("Verdana-Bold" on the style,
        // "Verdana" on the record), and what prints is
        // decided by the name on the style.
        // EVERY style whose family matches the record is
        // examined (round 5, R5-D): two source families
        // can share one display name, and resolving only
        // the first let an Exact first style certify a
        // second style that prints in a stand-in. The
        // finding reflects the worst resolution among
        // them.
        let mut worst: Option<ob_fonts::resolve::Resolution> = None;
        // Whether the matching styles resolve to more
        // than one stand-in family (round 6, R6-9): the
        // note's claim is scoped when they do.
        let mut first_family: Option<String> = None;
        let mut stand_ins_differ = false;
        for family in used.iter().filter(|family| {
            family.eq_ignore_ascii_case(&w.name)
                || ob_fonts::resolve::display_name_for(family).eq_ignore_ascii_case(&w.name)
        }) {
            let r = ob_fonts::resolve::resolve_family(family, available);
            match &first_family {
                None => first_family = Some(r.family.clone()),
                Some(first) if *first != r.family => stand_ins_differ = true,
                _ => {}
            }
            let is_worse = match &worst {
                None => true,
                Some(worst_so_far) => tier_rank(r.tier) > tier_rank(worst_so_far.tier),
            };
            if is_worse {
                worst = Some(r);
            }
        }
        let Some(r) = worst else {
            continue;
        };
        if r.tier != ob_fonts::resolve::SubstituteTier::Exact {
            // The stand-in named is the worst style's
            // (the one the finding is about); when the
            // styles resolve to different stand-ins, the
            // sentence says SOME of its words, because
            // the whole-bulletin wording would be false
            // for every other style (round 6, R6-9).
            // A single stand-in keeps the round-3 words.
            let words = if stand_ins_differ {
                ob_fonts::resolve::wanted_font_words_some(&w.name, &r.family)
            } else {
                ob_fonts::resolve::wanted_font_words(&w.name, &r.family)
            };
            out.push(CheckNote::to_fonts(format!("Check: {words}")));
        }
    }
    out
}

/// Does this placed picture still need its children
/// note (R-2b)? True when the box holds a picture that
/// is in the bulletin and its rights record carries no
/// answer yet: Check asks, because the web page's rule
/// can only take the care the note describes. A box
/// whose picture is missing in fact is another
/// finding's business. A music box is never asked
/// (round 3, R3-4): its picture is engraved sheet
/// music, not a photograph of anybody, and its menu
/// has no children answer to give, so the question
/// would be unanswerable noise. Pure over (document,
/// frame).
fn picture_children_unnoted(doc: &Document, frame: &ob_model::Frame) -> bool {
    if frame.music.is_some() {
        return false;
    }
    let Some(placement) = &frame.picture else {
        return false;
    };
    if doc.asset_data(&placement.asset).is_none() {
        return false;
    }
    doc.assets
        .iter()
        .find(|a| a.sha256 == placement.asset)
        .and_then(|a| a.rights.children_present)
        .is_none()
}

/// Check's children's-names reminder (R-2b): when the
/// church's rule is not full names, one plain note
/// repeating the rule, honest about its limit. The app
/// cannot tell which names are children's; it can only
/// remind a person to look. Pure over the profile.
fn children_names_note(profile: &ob_model::ChurchProfile) -> Option<CheckNote> {
    match profile.children_names {
        ob_model::ChildrenNames::FullNames => None,
        rule => Some(CheckNote::plain(format!(
            "Your church's rule: {}. This is a reminder to look over the names; \
             the app cannot tell which names are children's.",
            rule.rule_words()
        ))),
    }
}

/// Check's hymn pass (R-2a): each hymn line in the order
/// of service, held up against itself and against the
/// church's hymn memory. A number with no title and a
/// title with no number are both easy to mistype and hard
/// to spot; a pair the church has printed differently
/// before names both versions, and the note carries a
/// "Forget this pairing" button for when the office says
/// the memory is the wrong one (round 3, R3-6). When the
/// memory holds no pair, it says nothing: nothing is
/// invented. Pure over (document, memory); every finding
/// points at the This Sunday step, where the hymn fields
/// live.
fn hymn_findings(doc: &Document, memory: &ob_model::hymns::HymnMemory) -> Vec<CheckNote> {
    use ob_model::hymns::{compose_hymn_value, is_hymn_label, parse_hymn_value};
    let mut out = Vec::new();
    let mut by_number: Vec<(String, Option<String>, String, String)> = Vec::new(); // (number, hymnal, shown, label)
    for line in service_lines(doc) {
        if !is_hymn_label(&line.label) {
            continue;
        }
        if line.value.trim().is_empty() {
            continue;
        }
        let entry = parse_hymn_value(&line.value);
        match (&entry.number, &entry.title) {
            (Some(number), None) => out.push(CheckNote::to_step(
                format!(
                    "Check: the {} line gives hymn {number} but no title. \
                     A number alone is easy to mistype; add the title so it can be checked.",
                    line.label
                ),
                1,
            )),
            (None, Some(title)) => out.push(CheckNote::to_step(
                format!(
                    "Check: the {} line gives the title \u{201c}{title}\u{201d} but no hymn number. \
                     A title alone is easy to mistype; add the number so it can be checked.",
                    line.label
                ),
                1,
            )),
            (Some(number), Some(title)) => {
                if let Some(pair) = memory.lookup(entry.hymnal.as_deref(), number) {
                    let remembered = pair.title.trim();
                    if !remembered.eq_ignore_ascii_case(title.trim()) {
                        let hymn_words = match &entry.hymnal {
                            Some(h) => format!("{h} {number}"),
                            None => number.clone(),
                        };
                        let mut note = CheckNote::to_step(
                            format!(
                                "Check: hymn {hymn_words} was printed as \u{2018}{remembered}\u{2019} \
                                 for {}. This week it says \u{2018}{title}\u{2019}. One of them may be wrong.",
                                ob_layout::long_date(naive_of(pair.last_seen))
                            ),
                            1,
                        );
                        // If the office says the memory is
                        // the wrong one, the note's button
                        // forgets the pairing (R3-6).
                        note.forget_hymn = Some(ob_model::hymns::hymn_key(
                            entry.hymnal.as_deref(),
                            number,
                        ));
                        out.push(note);
                    }
                }
            }
            (None, None) => {}
        }
        if let Some(number) = &entry.number {
            by_number.push((
                number.clone(),
                entry.hymnal.clone(),
                compose_hymn_value(&entry),
                line.label.clone(),
            ));
        }
    }
    // The same number on two hymn lines: named once,
    // however many lines share it. Two lines name the
    // same hymn when their numbers match and their
    // hymnals agree, or when either line names no hymnal
    // at all (round 3, R3-17): "ELW 432" on one line and
    // a bare "432" on another are one song sung twice,
    // not two. Different hymnals with the same number
    // are different books, and stay quiet.
    let mut named: Vec<String> = Vec::new();
    for (i, (number, hymnal, shown, label)) in by_number.iter().enumerate() {
        if named.contains(number) {
            continue;
        }
        let other = by_number.iter().skip(i + 1).find(|(n2, h2, _, _)| {
            n2 == number
                && match (hymnal, h2) {
                    (Some(a), Some(b)) => a.eq_ignore_ascii_case(b),
                    _ => true,
                }
        });
        let Some((_, other_hymnal, _other_shown, other_label)) = other else {
            continue;
        };
        named.push(number.clone());
        let text = match (hymnal.as_deref(), other_hymnal.as_deref()) {
            (Some(h), None) | (None, Some(h)) => format!(
                "Check: hymn {number} is listed twice, once as {h} {number} and once with no hymnal named. \
                 Only one of them can be right."
            ),
            _ => format!(
                "Check: hymn {shown} is named by two hymn lines, {label} and {other_label}. \
                 Only one of them can be right."
            ),
        };
        out.push(CheckNote::to_step(text, 1));
    }
    out
}

/// What a music proof carries, counted for its status
/// line (R-2a). The spoken-only count rides along so the
/// Print status says what was left out, like every other
/// door does (round 3, R3-15).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MusicProofCounts {
    hymns: usize,
    music_pages: usize,
    spoken_only_left_out: usize,
}

/// Compose the music proof document (R-2a): a heading,
/// the hymn lines as words, then one page per engraved
/// music system at its bulletin size, then the anthem
/// and special music lines. Composed from the
/// publication copy, so spoken-only words never reach
/// it. The music pages follow the piece's own order,
/// system by system, however the systems' boxes sit in
/// the bulletin, and a system carried by two boxes
/// prints once: the proof is a reading copy of the
/// piece, not of the layout (round 3, R3-15). The music
/// director checks this sheet, or carries it to the
/// piano; the organist's copy is the same sheet. Pure:
/// the same document in, the same proof out.
fn music_proof_document(doc: &Document) -> (Document, MusicProofCounts) {
    use ob_model::hymns::{compose_hymn_value, is_hymn_label, parse_hymn_value};
    let (base, spoken_only_left_out) = doc.without_spoken_only();
    let page = ob_model::Size::new(612_000, 792_000); // US Letter portrait
    let margin: ob_model::Mpt = 54_000; // 3/4 inch
    let usable_w = page.w - 2 * margin;
    let usable_h = page.h - 2 * margin;
    let heading_style = if base.style("heading").is_some() {
        "heading"
    } else {
        "body"
    };
    let church = base.profile.name.trim();
    let date_long = base
        .fields
        .get("occasion_date_long")
        .map(|s| s.trim())
        .unwrap_or("");
    let heading = match (church.is_empty(), date_long.is_empty()) {
        (false, false) => format!("{church}: music for {date_long}"),
        (false, true) => format!("{church}: music"),
        (true, false) => format!("Music for {date_long}"),
        (true, true) => "Music".to_string(),
    };
    let mut word_paras = vec![ob_model::Paragraph::plain(heading_style, &heading)];
    let mut anthem_paras: Vec<ob_model::Paragraph> = Vec::new();
    let mut hymns = 0usize;
    for line in service_lines(&base) {
        let value = line.value.trim();
        if value.is_empty() {
            continue;
        }
        if is_hymn_label(&line.label) {
            let composed = compose_hymn_value(&parse_hymn_value(value));
            if composed.is_empty() {
                continue;
            }
            word_paras.push(ob_model::Paragraph::plain(
                "body",
                &format!("{}: {}", line.label, composed),
            ));
            hymns += 1;
        } else {
            let l = line.label.to_lowercase();
            if l.contains("anthem")
                || l.contains("special music")
                || l.contains("solo")
                || l.contains("choir")
            {
                anthem_paras.push(ob_model::Paragraph::plain(
                    "body",
                    &format!("{}: {}", line.label, value),
                ));
            }
        }
    }
    let text_frame = |id: &str, name: &str, story: &str| ob_model::Frame {
        id: id.to_string(),
        name: name.to_string(),
        kind: ob_model::FrameKind::Text,
        rect: ob_model::Rect::new(margin, margin, usable_w, usable_h),
        story_id: Some(story.to_string()),
        role: ob_model::ContentRole::Standing,
        picture: None,
        music: None,
    };
    let mut stories = vec![ob_model::Story {
        id: "proof-words".to_string(),
        paragraphs: word_paras,
        keep_through: None,
    }];
    let mut panels = vec![ob_model::Panel {
        id: "proof-page-1".to_string(),
        name: "Music proof".to_string(),
        size: page,
        frames: vec![text_frame(
            "proof-words-frame",
            "Music proof",
            "proof-words",
        )],
        guides: Vec::new(),
        background_image: None,
    }];
    // One page per music system, at its bulletin size
    // (shrunk only when the bulletin box is bigger than
    // this page), the whole system shown: notation is
    // never trimmed. The systems come in the piece's
    // own order (round 3, R3-15): a box moved on the
    // page in the editor reorders the bulletin, not the
    // piece, and two boxes carrying the same system
    // print that system once. The key is (source,
    // system), and the pieces come in the order their
    // first box appears in the bulletin (round 4,
    // R4-3): system numbers count from 0 per piece, so
    // on their own they collide across pieces, and the
    // later piece's systems used to vanish from the
    // proof the music director checks.
    let mut systems: Vec<&ob_model::Frame> = Vec::new();
    for panel in &base.panels {
        for frame in &panel.frames {
            if frame.music.is_some() && frame.picture.is_some() {
                systems.push(frame);
            }
        }
    }
    let mut piece_order: Vec<&str> = Vec::new();
    for f in &systems {
        let src = f.music.as_ref().map(|m| m.source.as_str()).unwrap_or("");
        if !piece_order.contains(&src) {
            piece_order.push(src);
        }
    }
    let piece_rank = |f: &ob_model::Frame| {
        let src = f.music.as_ref().map(|m| m.source.as_str()).unwrap_or("");
        piece_order.iter().position(|s| *s == src).unwrap_or(0)
    };
    systems.sort_by_key(|f| {
        (
            piece_rank(f),
            f.music.as_ref().map(|m| m.system).unwrap_or(0),
        )
    });
    systems.dedup_by_key(|f| {
        f.music
            .as_ref()
            .map(|m| (m.source.clone(), m.system))
            .unwrap_or_default()
    });
    let mut music_pages = 0usize;
    for frame in systems {
        let Some(picture) = &frame.picture else {
            continue;
        };
        music_pages += 1;
        let (mut w, mut h) = (frame.rect.w.max(1), frame.rect.h.max(1));
        if w > usable_w || h > usable_h {
            let scale = (usable_w as f64 / w as f64).min(usable_h as f64 / h as f64);
            w = (w as f64 * scale).round() as ob_model::Mpt;
            h = (h as f64 * scale).round() as ob_model::Mpt;
        }
        let mut nf = frame.clone();
        nf.id = format!("proof-music-{music_pages}");
        nf.name = "Printed music".to_string();
        nf.rect = ob_model::Rect::new(margin, margin, w, h);
        nf.story_id = None;
        nf.picture = Some(ob_model::PicturePlacement {
            fit: ob_model::PictureFit::ShowWhole,
            ..picture.clone()
        });
        panels.push(ob_model::Panel {
            id: format!("proof-music-page-{music_pages}"),
            name: "Printed music".to_string(),
            size: page,
            frames: vec![nf],
            guides: Vec::new(),
            background_image: None,
        });
    }
    if !anthem_paras.is_empty() {
        stories.push(ob_model::Story {
            id: "proof-anthems".to_string(),
            paragraphs: anthem_paras,
            keep_through: None,
        });
        panels.push(ob_model::Panel {
            id: "proof-anthem-page".to_string(),
            name: "Anthems and special music".to_string(),
            size: page,
            frames: vec![text_frame(
                "proof-anthems-frame",
                "Anthems and special music",
                "proof-anthems",
            )],
            guides: Vec::new(),
            background_image: None,
        });
    }
    let mut out = base;
    out.title = "Music proof".to_string();
    out.stories = stories;
    out.panels = panels;
    out.sheet = ob_model::SheetSpec {
        preset: "letter-portrait".to_string(),
        size: page,
    };
    (
        out,
        MusicProofCounts {
            hymns,
            music_pages,
            spoken_only_left_out,
        },
    )
}

fn char_findings(doc: &Document) -> Vec<CharFinding> {
    let oracle = ob_pdf::FaceOracle::new(&ob_pdf::library_font_bytes());
    let mut out = Vec::new();
    for story in &doc.stories {
        let mut chars: Vec<char> = Vec::new();
        for para in &story.paragraphs {
            // Spoken-only paragraphs never print, so their
            // marks can never go missing from a printed
            // copy: not a finding (working as intended).
            if para.spoken_only {
                continue;
            }
            // The same fallback style the PDF writer draws
            // with when a paragraph names a style the document
            // no longer carries.
            let style = doc
                .style(&para.style)
                .cloned()
                .unwrap_or(ob_model::ParagraphStyle {
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
            for run in ob_model::resolve_runs(&para.runs, doc) {
                let bold = run.bold.unwrap_or(style.bold);
                let italic = run.italic.unwrap_or(style.italic);
                for c in oracle.undrawable_chars(&run.text, &style.font_family, bold, italic) {
                    if !chars.contains(&c) {
                        chars.push(c);
                    }
                }
            }
        }
        if chars.is_empty() {
            continue;
        }
        let label = story
            .paragraphs
            .iter()
            .map(|p| p.text())
            .find(|t| !t.trim().is_empty())
            .map(|t| t.trim().chars().take(38).collect::<String>())
            .unwrap_or_else(|| "A text block".to_string());
        let target = frame_target_for_story(doc, &story.id);
        // The way out, when the catalog holds one: a face
        // whose cmap verifiably carries the missing mark.
        let helper = chars.iter().find_map(|c| {
            ob_fonts::catalog::catalog_face_with_glyph(*c).map(|fam| (*c, fam.to_string()))
        });
        out.push(CharFinding {
            story_id: story.id.clone(),
            label,
            chars,
            target,
            helper,
        });
    }
    out
}

/// The first frame holding a story, as (panel, frame)
/// indices, when the story sits in a frame at all. Shared
/// by the character findings and the font findings so a
/// Check note can point at the box it means.
fn frame_target_for_story(doc: &Document, story_id: &str) -> Option<(usize, Option<usize>)> {
    for (pi, panel) in doc.panels.iter().enumerate() {
        for (fi, frame) in panel.frames.iter().enumerate() {
            if frame.story_id.as_deref() == Some(story_id) {
                return Some((pi, Some(fi)));
            }
        }
    }
    None
}

/// The Check-step words for one unprintable-character finding.
fn char_note(f: &CharFinding) -> String {
    let marks = f
        .chars
        .iter()
        .map(|c| format!("\"{c}\""))
        .collect::<Vec<_>>()
        .join(", ");
    let mut note = if f.chars.len() == 1 {
        format!(
            "Check: the words in \"{}\" use a mark the print copy cannot show: {marks}. \
             It would print as an empty box. Type a plain mark instead, or take it out.",
            f.label
        )
    } else {
        format!(
            "Check: the words in \"{}\" use marks the print copy cannot show: {marks}. \
             They would print as empty boxes. Type plain marks instead, or take them out.",
            f.label
        )
    };
    // The way out, when the catalog verifiably holds the
    // mark: name the face that carries it.
    if let Some((_, family)) = &f.helper {
        note.push_str(&format!(
            " {family} has this letter; it is in the Fonts list."
        ));
    }
    note
}

/// The Check-step line about announcements, in plain words.
/// An empty box is only fine when the office said so out loud
/// (adopted item 4).
fn announcements_check(announcements_empty: bool, none_marked: bool) -> String {
    if !announcements_empty {
        "OK: announcements are filled in.".to_string()
    } else if none_marked {
        "OK: no announcements this week, as marked.".to_string()
    } else {
        "Check: announcements are still needed. Add them on the \
         Announcements step, or mark \"No announcements this week\" \
         there if there are none."
            .to_string()
    }
}

/// What is being carried into this week, as information
/// lines for the Check step (J10): every story whose
/// keep-through date covers the bulletin's Sunday, named by
/// its first words and its through date. Automation shows
/// its hand; these are facts, not problems, so the lines
/// never start with "Check:".
fn carried_lines(doc: &Document) -> Vec<String> {
    use chrono::Datelike;
    let Some(bulletin) = doc
        .fields
        .get("occasion_date")
        .and_then(|iso| chrono::NaiveDate::parse_from_str(iso.trim(), "%Y-%m-%d").ok())
    else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for story in &doc.stories {
        let Some(raw) = story.keep_through.as_deref() else {
            continue;
        };
        let Ok(until) = chrono::NaiveDate::parse_from_str(raw.trim(), "%Y-%m-%d") else {
            continue;
        };
        if until < bulletin {
            continue;
        }
        let owned_text = story
            .paragraphs
            .iter()
            .filter(|p| p.style == "body")
            .map(|p| p.text())
            .find(|t| !t.trim().is_empty())
            .or_else(|| {
                story
                    .paragraphs
                    .iter()
                    .map(|p| p.text())
                    .find(|t| !t.trim().is_empty())
            });
        let first_text = owned_text.as_deref().unwrap_or("An announcement");
        let first: String = first_text
            .lines()
            .next()
            .unwrap_or(first_text)
            .trim()
            .chars()
            .take(40)
            .collect();
        let through = format!("{} {}, {}", until.format("%B"), until.day(), until.year());
        out.push(format!(
            "Carried into this week: \"{first}\" (kept through {through})."
        ));
    }
    out
}

/// Whether a date field holds an edit: some text, and not
/// the bulletin's stored date. A blank field is not an
/// edit; the bulletin always has a date.
fn date_differs(input: &str, stored: Option<&str>) -> bool {
    let input = input.trim();
    !input.is_empty() && input != stored.unwrap_or("")
}

/// "Sunday, October 18, 2026": the long form with the
/// weekday, the way dates are spoken on the steps.
fn long_form(d: chrono::NaiveDate) -> String {
    format!("{}, {}", d.format("%A"), ob_layout::long_date(d))
}

/// Today's date in the model's own date type, for the
/// occasions parser (docs/planning/occasions.md).
fn today_simple() -> ob_model::SimpleDate {
    use chrono::Datelike;
    let t = chrono::Local::now().date_naive();
    ob_model::SimpleDate {
        year: t.year(),
        month: t.month(),
        day: t.day(),
    }
}

/// A chrono date in the model's own date type.
fn simple_of(d: chrono::NaiveDate) -> ob_model::SimpleDate {
    use chrono::Datelike;
    ob_model::SimpleDate {
        year: d.year(),
        month: d.month(),
        day: d.day(),
    }
}

/// The model's date type back as a chrono date (for the
/// weekday-inclusive formatting the steps speak in).
fn naive_of(d: ob_model::SimpleDate) -> chrono::NaiveDate {
    chrono::NaiveDate::from_ymd_opt(d.year, d.month, d.day)
        .unwrap_or_else(|| chrono::NaiveDate::from_ymd_opt(1970, 1, 1).expect("epoch"))
}

/// The Check step's date finding (J1, generalized by
/// occasions): a Sunday-dated bulletin whose date is not
/// the coming Sunday keeps the coming-Sunday wording; a
/// bulletin for another day (Ash Wednesday, a wedding)
/// is only flagged when its day has passed, and the words
/// say so without claiming Sunday. A date that does not
/// read gets no verdict here; the missing-date note
/// covers that case.
fn date_verdict(doc_iso: &str, today: chrono::NaiveDate) -> Option<String> {
    let parsed = ob_model::parse_flexible_date(doc_iso.trim(), simple_of(today))?;
    let date = naive_of(parsed);
    if parsed.weekday() == 0 {
        let coming = sunday_on_or_after(today);
        if date == coming {
            return None;
        }
        return Some(format!(
            "Check: this bulletin says {}, but this coming Sunday is {}.",
            long_form(date),
            long_form(coming)
        ));
    }
    if date < today {
        return Some(format!(
            "Check: this bulletin says {}, and that day has passed.",
            long_form(date)
        ));
    }
    None
}

/// The stale-date sentence (J1), shared by the Start status
/// and the This Sunday step: the bulletin's date is behind
/// today, so the bulletin on screen is for a day that has
/// already happened. A Sunday keeps the coming-Sunday
/// wording; another day is named plainly.
fn date_passed_sentence(doc_iso: Option<&str>, today: chrono::NaiveDate) -> Option<String> {
    let parsed = ob_model::parse_flexible_date(doc_iso?.trim(), simple_of(today))?;
    let date = naive_of(parsed);
    if date >= today {
        return None;
    }
    Some(if parsed.weekday() == 0 {
        "That date has passed. Set the date for this Sunday on the This Sunday step.".to_string()
    } else {
        "That date has passed. Set the date on the This Sunday step.".to_string()
    })
}

/// The profile save's words when serving lines were
/// removed with names still saved for them (round 4,
/// RT-6c): the names are cleared with the line, and
/// the status says so plainly, one line or several.
fn removed_serving_lines_sentence(removed: &[String]) -> String {
    let quoted = removed
        .iter()
        .map(|n| format!("'{n}'"))
        .collect::<Vec<_>>()
        .join(", ");
    if removed.len() == 1 {
        format!("Names saved for {quoted} were removed with the line.")
    } else {
        format!("Names saved for {quoted} were removed with their lines.")
    }
}

/// The profile save's words when a serving-lines
/// change is refused as a whole (round 6, R6-C):
/// the app cannot tell which names belong to which
/// line; the lines were left as they were, and the
/// remedy — one line at a time — now works, because
/// nothing half-applied. Round 7 (R7-10): the
/// account names only the kinds of change the
/// pairing actually saw, so a rename-plus-removal
/// refusal no longer claims a move that never
/// happened; the round-6 wording stands exactly as
/// written for the shape it was written for —
/// renames that crossed anchors, with nothing
/// removed or added.
fn ambiguous_serving_lines_sentence(kinds: &ob_model::ServingLineEditKinds) -> String {
    let canonical = ob_model::ServingLineEditKinds {
        renamed: true,
        removed: false,
        added: false,
        moved: true,
    };
    if *kinds == canonical {
        return "Serving lines not saved: several lines were renamed and moved at once, \
             and the app cannot tell which names belong to which line. The lines \
             were left as they were. Rename one line at a time, then save again."
            .to_string();
    }
    // Join a short word list the plain way: "a and
    // b", or "a, b and c" — with `last` before the
    // final word.
    let join_words = |words: &[&str], last: &str| -> String {
        match words.len() {
            0 => String::new(),
            1 => words[0].to_string(),
            2 => format!("{} {last} {}", words[0], words[1]),
            _ => {
                let head = words[..words.len() - 1].join(", ");
                format!("{head} {last} {}", words[words.len() - 1])
            }
        }
    };
    let mut seen: Vec<&str> = Vec::new();
    if kinds.renamed {
        seen.push("renamed");
    }
    if kinds.removed {
        seen.push("removed");
    }
    if kinds.added {
        seen.push("added");
    }
    if kinds.moved {
        seen.push("moved");
    }
    if seen.is_empty() {
        seen.push("changed");
    }
    let account = join_words(&seen, "and");
    // The remedy names the worker's own actions the
    // same way: make each rename, removal, or
    // addition on its own, then save again.
    let mut verbs: Vec<&str> = Vec::new();
    if kinds.renamed {
        verbs.push("rename");
    }
    if kinds.removed {
        verbs.push("remove");
    }
    if kinds.added {
        verbs.push("add");
    }
    if verbs.is_empty() {
        verbs.push("rename");
    }
    let remedy_joined = join_words(&verbs, "or");
    let mut remedy = remedy_joined;
    if let Some(first) = remedy.get_mut(0..1) {
        first.make_ascii_uppercase();
    }
    format!(
        "Serving lines not saved: lines were {account} at once, \
         and the app cannot tell which names belong to which line. The lines \
         were left as they were. {remedy} one line at a time, then save again."
    )
}

/// The profile save's words when serving lines are
/// refused over a shared slug (round 5, RT5-4): the
/// save does not apply, the typed lines stay in the
/// editor for correction, and the sentence names
/// both lines so the office can find the clash.
/// Round 6 (R6-11): the closing words claim only
/// what this refusal covers — the serving lines were
/// left as they were. The profile's other fields
/// save on their own as they are edited, so a wider
/// "nothing else was changed" could be false of the
/// editing pass the person is in the middle of.
fn serving_line_conflict_sentence(conflict: &ob_model::ServingLineSlugConflict) -> String {
    let first = format!("'{}'", conflict.first);
    let second = format!("'{}'", conflict.second);
    if conflict.builtin {
        format!(
            "Serving lines not saved: {first} would take the place of the \
             built-in line {second}, so the line's row and the names the \
             bulletin prints could name different people. Rename {first}, \
             then save again. The serving lines were left as they were."
        )
    } else {
        format!(
            "Serving lines not saved: {first} and {second} would be stored \
             as the same line, so typing names for one would change the \
             other. Rename one of them, then save again. The serving \
             lines were left as they were."
        )
    }
}

/// The Start step's words for a remembered file state (J6):
/// the strongest true statement about the print copy and the
/// last clean check, compared against the document as it
/// stands now. A stale print copy alone says nothing here;
/// the Print step carries that warning.
fn state_sentence(state: Option<&DocState>, current: &str) -> Option<String> {
    let state = state?;
    if state.print_fingerprint.as_deref() == Some(current) {
        return Some("Print copy is current.".to_string());
    }
    if state.checked_fingerprint.as_deref() == Some(current) {
        return Some("Checked, and nothing has changed since.".to_string());
    }
    if state.checked_fingerprint.is_some() {
        return Some("Not checked since the last change.".to_string());
    }
    None
}

/// The Start step's opening lines (adopted item 3): which
/// Sunday this is, and the next thing needed, worked out from
/// the document itself in step order. Nothing here is kept by
/// hand, so it cannot go stale. A date that has passed outranks
/// every other need (J1): the bulletin on screen is for the
/// wrong week until it is fixed. The current file's remembered
/// state (J6) appends its sentence when it has one.
fn start_status(
    doc: &Document,
    announcements_empty: bool,
    today: chrono::NaiveDate,
    state: Option<&DocState>,
) -> (String, String) {
    let long = doc
        .fields
        .get("occasion_date_long")
        .map(|s| s.trim())
        .filter(|s| !s.is_empty());
    // The headline names the day the bulletin is for,
    // weekday included, whatever day of the week it is
    // (occasions). When the stored date does not parse,
    // fall back to the stored long form as before.
    let headline = match doc
        .fields
        .get("occasion_date")
        .and_then(|iso| ob_model::parse_flexible_date(iso.trim(), simple_of(today)))
    {
        Some(d) => format!("Working on: {}", long_form(naive_of(d))),
        None => match long {
            Some(d) => format!("Working on: Sunday, {d}"),
            None => "Working on: a new bulletin".to_string(),
        },
    };
    let date_missing = long.is_none()
        || doc
            .fields
            .get("occasion_date")
            .map(|s| s.trim().is_empty())
            .unwrap_or(true);
    let passed = date_passed_sentence(doc.fields.get("occasion_date").map(String::as_str), today);
    let mut next = if let Some(sentence) = passed {
        sentence
    } else if doc.profile.name.trim().is_empty() {
        "Next: add your church name on this step.".to_string()
    } else if date_missing {
        "Next: set the date on the This Sunday step.".to_string()
    } else if announcements_empty && !doc.no_announcements {
        "Next: add this week's announcements on the Announcements step, \
         or mark there that there are none this week."
            .to_string()
    } else {
        "Ready to print.".to_string()
    };
    if let Some(words) = state_sentence(state, &snapshot(doc)) {
        next = format!("{next} {words}");
    }
    (headline, next)
}

/// True while the bulletin on screen is the starter sample
/// (J7): no file behind it, and no church named yet. Start
/// says so out loud, so a sample is never mistaken for the
/// church's own bulletin.
fn is_starter_sample(has_file: bool, church_name: &str) -> bool {
    !has_file && church_name.trim().is_empty()
}

/// True when a print copy exists but the bulletin has changed
/// since it was made (adopted item 5). The fingerprint is the
/// same snapshot text the undo history keeps.
fn print_copy_stale(fingerprint: Option<&str>, current: &str) -> bool {
    fingerprint.map(|fp| fp != current).unwrap_or(false)
}

/// The Start step's first door: the bulletin this office
/// worked on last, offered back when Settings remember one
/// and the file is still where it was. A gone file shows
/// nothing: no dead button, no error (UI audit round 2).
/// Returns the path and the file's own name.
pub fn last_bulletin_door(
    remembered: Option<&str>,
    file_still_there: bool,
) -> Option<(std::path::PathBuf, String)> {
    if !file_still_there {
        return None;
    }
    let raw = remembered?.trim();
    if raw.is_empty() {
        return None;
    }
    let path = std::path::PathBuf::from(raw);
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .filter(|n| !n.is_empty())?;
    Some((path, name))
}

/// How reading the settings file went, so the app can say
/// once, in plain words, when the file could not be trusted
/// (round 2, R2-4: the courtesy the autosave journal already
/// had, extended to the settings file).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsRead {
    /// The file read clean, or there was no file yet.
    Clean,
    /// Some sections read and some did not: the broken ones
    /// went back to their defaults, and the rest survived.
    PartlySalvaged,
    /// The file was there but could not be read as settings
    /// at all; it was set aside as settings-unreadable.json
    /// and fresh settings are in force.
    SetAside,
}

/// One section of the settings file, read on its own: a
/// missing section is the default and no news; a section
/// that will not read is the default and a mark against the
/// file, but it never takes the other sections down with
/// it (round 2, R2-4).
fn settings_section<T: serde::de::DeserializeOwned>(
    obj: &serde_json::Map<String, serde_json::Value>,
    name: &str,
    salvaged: &mut bool,
) -> Option<T> {
    let value = obj.get(name)?;
    match serde_json::from_value::<T>(value.clone()) {
        Ok(v) => Some(v),
        Err(_) => {
            *salvaged = true;
            None
        }
    }
}

impl Settings {
    /// Read settings from the file's text, section by
    /// section (round 2, R2-4), and validate the keymap as
    /// it lands (R2-5). Text that is not a settings object
    /// at all reads as SetAside: the caller keeps the file
    /// under a name that says what happened and starts
    /// fresh, rather than quietly forgetting everything.
    pub fn load_from_str(text: &str) -> (Settings, SettingsRead) {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
            return (Settings::default(), SettingsRead::SetAside);
        };
        let Some(obj) = value.as_object() else {
            return (Settings::default(), SettingsRead::SetAside);
        };
        let mut salvaged = false;
        let mut s = Settings::default();
        if let Some(v) = settings_section(obj, "printers", &mut salvaged) {
            s.printers = v;
        }
        if let Some(v) = settings_section(obj, "keymap_preset", &mut salvaged) {
            s.keymap_preset = v;
        }
        if let Some(v) = settings_section(obj, "keymap", &mut salvaged) {
            s.keymap = v;
        }
        if let Some(v) = settings_section(obj, "last_bulletin", &mut salvaged) {
            s.last_bulletin = v;
        }
        if let Some(v) = settings_section(obj, "doc_states", &mut salvaged) {
            s.doc_states = v;
        }
        if let Some(v) = settings_section(obj, "last_step", &mut salvaged) {
            s.last_step = v;
        }
        if let Some(v) = settings_section(obj, "pdf_purpose", &mut salvaged) {
            s.pdf_purpose = v;
        }
        if let Some(v) = settings_section(obj, "ui_scale", &mut salvaged) {
            s.ui_scale = v;
        }
        if let Some(v) = settings_section(obj, "high_contrast", &mut salvaged) {
            s.high_contrast = v;
        }
        validate_keymap(&mut s);
        let read = if salvaged {
            SettingsRead::PartlySalvaged
        } else {
            SettingsRead::Clean
        };
        (s, read)
    }
}

/// Move an unreadable settings file out of the way, under
/// a name that says what happened, so the next launch
/// starts clean and the damaged copy is still on disk for
/// anyone who goes looking (the journal's pattern). An
/// older set-aside is replaced: one damaged copy is enough.
fn set_aside_unreadable_settings(path: &std::path::Path) -> bool {
    if !path.exists() {
        return false;
    }
    let to = path.with_file_name("settings-unreadable.json");
    std::fs::rename(path, &to).is_ok()
}

pub fn load_settings() -> (Settings, SettingsRead) {
    let path = settings_path();
    let Ok(text) = std::fs::read_to_string(&path) else {
        return (Settings::default(), SettingsRead::Clean);
    };
    let (settings, read) = Settings::load_from_str(&text);
    if read == SettingsRead::SetAside {
        set_aside_unreadable_settings(&path);
    }
    (settings, read)
}

pub fn save_settings(settings: &Settings) -> Result<(), String> {
    let path = settings_path();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    // Atomic, the document format's discipline (round 2,
    // R2-4): write a sibling temp file and rename it over,
    // so a crash mid-write cannot truncate the one file
    // that remembers the office's printers and keys.
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, text).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())
}

/// One row of the Fonts panel: a library font with everything the
/// admin needs (its own face for the preview, its name, and what its
/// license allows in plain words).
#[derive(Clone)]
struct FontRow {
    key: String,
    family: String,
    full_name: String,
    rights_words: String,
}

/// The shaped-lines cache: per-panel preview lines plus
/// the document's thread layouts (T1), all keyed by the
/// document snapshot, so the canvas, the Check step, and
/// the print preview all read the same layout of a story
/// through its boxes.
struct LinesCache {
    key: String,
    lines: std::collections::HashMap<usize, Vec<PreviewLine>>,
    threads: std::collections::HashMap<String, ob_text::threading::StoryThread>,
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

/// One placed picture in a preview: its content key (the
/// asset fingerprint), the box it sits in, and the rect it
/// actually shows in, sheet points.
#[derive(Clone)]
struct PreviewPicture {
    asset: String,
    clip: egui::Rect,
    shown: egui::Rect,
    fit: ob_model::PictureFit,
}

#[derive(Clone)]
struct PreviewPanel {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    name: String,
    lines: Vec<PreviewLine>,
    /// Placed pictures, painted under the panel's words.
    pictures: Vec<PreviewPicture>,
}

#[derive(Clone)]
struct PreviewLine {
    text: String,
    size: f32,
    x: f32,
    y: f32,
    w: f32,
    align: ob_model::Align,
    /// The face this line prints in, carried so the previews
    /// can draw it in that face too (UI audit round 2,
    /// batch 3): the style's font family name plus its
    /// bold and italic switches.
    font_family: String,
    bold: bool,
    italic: bool,
    /// Same-face segments of the line (G1b): a bold word
    /// inside the line draws in the bold face. `dx` is the
    /// segment's offset from the line's left edge, in points.
    /// Empty means "the whole line in the line's own face"
    /// (the editor's probe lines carry none).
    segs: Vec<PreviewSeg>,
    /// This line starts a spoken-only paragraph: the Pages
    /// canvas tags it "SPOKEN ONLY" above the line, so the
    /// state is visible on the page and never hidden. Only
    /// the canvas paints the tag, never the print previews:
    /// spoken-only words are not in any printed copy.
    spoken_tag: bool,
    /// This line starts a not-for-web paragraph (R-2b):
    /// the Pages canvas tags it "NOT FOR WEB" above the
    /// line, on the same terms as the spoken tag. The
    /// words DO print; only the web page leaves them out,
    /// and no printed copy carries the tag.
    web_tag: bool,
}

/// One same-face piece of a preview line (G1b).
#[derive(Clone)]
struct PreviewSeg {
    text: String,
    dx: f32,
    bold: bool,
    italic: bool,
}

/// Convert one chip-free display piece to its raw range,
/// through resolved space, and push it if it holds words.
fn push_raw_piece(
    map: &editmap::BufferMap,
    start: usize,
    end: usize,
    out: &mut Vec<std::ops::Range<usize>>,
) {
    if start >= end {
        return;
    }
    let rs = map.display_to_resolved(start);
    let re = map.display_to_resolved(end);
    let raws = map.resolved_to_raw(rs);
    let rawe = map.resolved_to_raw(re);
    if raws < rawe {
        out.push(raws..rawe);
    }
}

/// The effective face spans of a paragraph's RESOLVED runs:
/// each run's flags with the paragraph style folded in
/// (`run.bold.unwrap_or(style.bold)`), over the resolved
/// text. Shared by the preview builder and the editor.
fn effective_spans(
    runs: &[ob_model::Run],
    style: &ob_model::ParagraphStyle,
) -> Vec<ob_text::shaping::RunSpan> {
    let mut spans = Vec::new();
    let mut at = 0usize;
    for r in runs {
        spans.push(ob_text::shaping::RunSpan {
            range: at..at + r.text.len(),
            bold: r.bold.unwrap_or(style.bold),
            italic: r.italic.unwrap_or(style.italic),
        });
        at += r.text.len();
    }
    spans
}

/// Group a shaped line's clusters into same-face segments
/// (G1b), in drawing order; each segment starts at its first
/// cluster's position (the line's alignment offset folded
/// in). Clusters in a line are adjacent by construction, so
/// equal-flag neighbors simply extend the open segment.
fn line_segments(line: &ob_text::text_layout::LayoutLine, text: &str) -> Vec<PreviewSeg> {
    let mut segs: Vec<PreviewSeg> = Vec::new();
    for c in &line.clusters {
        let seg_text = &text[c.range.clone()];
        match segs.last_mut() {
            Some(last) if last.bold == c.bold && last.italic == c.italic => {
                last.text.push_str(seg_text);
            }
            _ => segs.push(PreviewSeg {
                text: seg_text.to_string(),
                dx: line.x0 + c.x,
                bold: c.bold,
                italic: c.italic,
            }),
        }
    }
    segs
}

/// One paragraph of the shaped canvas editor (E1): its
/// parley layout in DISPLAY space plus where its slice
/// sits inside the entered frame, stacked with the same
/// arithmetic the thread layout stacks the idle preview.
struct EditorPara {
    layout: ob_text::text_layout::TextLayout,
    /// Top of the first DRAWN line box, frame-local
    /// milli-points.
    top: ob_model::Mpt,
    /// Line height in milli-points (style size x 1.3).
    line_h: ob_model::Mpt,
    /// Lines of this paragraph that sit in EARLIER boxes of
    /// the thread (T1): the session shows the slice starting
    /// at the entered box, so these lines are not drawn
    /// here. Always 0 for a story in one box.
    skip: usize,
    /// Lines drawn in the entered box, starting at `skip`.
    /// Lines before and after exist for caret math but are
    /// not drawn: the same clipping the idle canvas applies,
    /// extended across the thread.
    drawn: usize,
    size: f32,
    font_family: String,
    bold: bool,
    italic: bool,
    align: ob_model::Align,
}

/// The shaped editor's geometry for one buffer state: the
/// raw/resolved/display map plus one layout per paragraph.
struct EditorLayout {
    key: String,
    map: editmap::BufferMap,
    paras: Vec<EditorPara>,
    /// The display offset where the entered box's slice of
    /// the story begins (T1): the first drawn line's start.
    /// None when no line is drawn in the entered box.
    slice_start: Option<usize>,
}

/// The egui family key for the library face a preview line
/// prints in. Every library file registers as its own egui
/// family (keyed by its hash), so the face is chosen among
/// the rows of the line's family by the face's full name:
/// the exact face when the library holds it (Bold, Italic,
/// Bold Italic), else the family's regular face. A family
/// the library does not hold paints in the resolution
/// chain's stand-in (docs/FONTS.md, wanted fonts), the
/// same face shaping and print use, so the screen never
/// shows a different font than the paper gets. None when
/// no face answers at all; the caller then falls back to
/// the interface font.
fn preview_face_key(rows: &[FontRow], family: &str, bold: bool, italic: bool) -> Option<String> {
    if let Some(key) = face_key_in_rows(rows, family, bold, italic) {
        return Some(key);
    }
    let mut available: Vec<String> = Vec::new();
    for row in rows {
        if !available.contains(&row.family) {
            available.push(row.family.clone());
        }
    }
    let resolved = ob_fonts::resolve::resolve_family(family, &available).family;
    if resolved != family {
        return face_key_in_rows(rows, &resolved, bold, italic);
    }
    None
}

/// The face-key lookup itself: among the rows of one
/// family, the exact face by full name, else the regular
/// face, else any face of the family.
fn face_key_in_rows(rows: &[FontRow], family: &str, bold: bool, italic: bool) -> Option<String> {
    let mut regular: Option<&FontRow> = None;
    let mut any: Option<&FontRow> = None;
    for row in rows {
        if !row.family.eq_ignore_ascii_case(family) {
            continue;
        }
        any = any.or(Some(row));
        let name = row.full_name.to_lowercase();
        let has_bold = name.contains("bold");
        let has_italic = name.contains("italic") || name.contains("oblique");
        if has_bold == bold && has_italic == italic {
            return Some(row.key.clone());
        }
        if !has_bold && !has_italic {
            regular = regular.or(Some(row));
        }
    }
    regular.or(any).map(|row| row.key.clone())
}

/// The named families a set of font definitions binds.
/// Painting may only name families in this set: egui
/// panics on a FontId whose family the ACTIVE definitions
/// do not bind ("is not bound to any fonts"), and new
/// definitions take a frame to become active, so a freshly
/// registered library family is unbound for exactly one
/// frame (round 2, NOTE 9: the first frame after
/// refresh_fonts painted preview lines in library faces the
/// still-default definitions had never heard of, and the
/// app died at startup).
pub fn bound_name_families(defs: &egui::FontDefinitions) -> std::collections::HashSet<String> {
    defs.families
        .keys()
        .filter_map(|f| match f {
            egui::FontFamily::Name(name) => Some(name.to_string()),
            _ => None,
        })
        .collect()
}

/// The font a preview line is drawn with: its own library
/// face when the library holds it AND that face's family is
/// bound in the definitions active this frame; otherwise
/// the interface font. The bound check is the NOTE 9 rule:
/// never hand the painter a family name the active
/// definitions do not bind.
fn line_font_id(
    rows: &[FontRow],
    bound: &std::collections::HashSet<String>,
    line: &PreviewLine,
    size: f32,
) -> egui::FontId {
    match preview_face_key(rows, &line.font_family, line.bold, line.italic) {
        Some(key) if bound.contains(&key) => {
            egui::FontId::new(size, egui::FontFamily::Name(key.into()))
        }
        _ => egui::FontId::proportional(size),
    }
}

/// Paint one preview line through `place` (a line-local x in
/// points to the screen point at the line's baseline). With
/// segments (G1b), each same-face piece draws in its own
/// face at its own offset, the offsets already carrying the
/// alignment; without them, the whole line draws in the
/// line's face with the alignment anchor, as before.
fn paint_line_segments(
    painter: &egui::Painter,
    rows: &[FontRow],
    bound: &std::collections::HashSet<String>,
    line: &PreviewLine,
    scale: f32,
    place: impl Fn(f32) -> egui::Pos2,
) {
    let size = (line.size * scale).max(4.0);
    if line.segs.is_empty() {
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
            place(lx),
            anchor,
            &line.text,
            line_font_id(rows, bound, line, size),
            egui::Color32::BLACK,
        );
        return;
    }
    for seg in &line.segs {
        let probe = PreviewLine {
            text: seg.text.clone(),
            size: line.size,
            x: line.x,
            y: line.y,
            w: line.w,
            align: line.align,
            font_family: line.font_family.clone(),
            bold: seg.bold,
            italic: seg.italic,
            segs: Vec::new(),
            spoken_tag: false,
            web_tag: false,
        };
        painter.text(
            place(line.x + seg.dx),
            egui::Align2::LEFT_BOTTOM,
            &seg.text,
            line_font_id(rows, bound, &probe, size),
            egui::Color32::BLACK,
        );
    }
}

/// The font definitions and the Fonts panel rows for the
/// whole library, built together so the two can never
/// disagree: every row's key is a family the definitions
/// bind (round 2, NOTE 9's registration rule).
fn library_font_setup(lib: &ob_fonts::FontLibrary) -> (egui::FontDefinitions, Vec<FontRow>) {
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
    (defs, rows)
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
        // Reopen on the step the user left (J6), clamped to
        // the steps that exist.
        let step = settings.last_step.min(STEPS.len() - 1);
        let theme = Theme::for_settings(&settings);
        // A settings file that could not be read is said out
        // loud once, on the first status line, never suffered
        // in silence (round 2, R2-4).
        let status = match settings_read {
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
            hymn_memory: load_hymn_memory(),
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

/// Where one part of a contribution will go, decided
/// before Accept so the review can say it in words.
#[derive(Debug, Clone, PartialEq)]
enum SlotTarget {
    /// Append the part's paragraphs to this story.
    Story(String),
    /// Append to the announcements story under a heading
    /// paragraph with these words (a Sunday school or youth
    /// part with no box of its own in this bulletin).
    AnnouncementsUnderHeading(&'static str),
    /// A loose sheet: not applied. It is filed with the
    /// contribution and kept safe (inserts get their own
    /// build later; the words are never dropped and never
    /// fake-applied).
    LooseSheet,
    /// Nowhere to land: the bulletin has no box for these
    /// words yet (round 3, R3-8a). They stay with the part
    /// when it is filed, and the plan says exactly that
    /// instead of promising a box that is not there.
    KeptWithPart,
    /// The part's words are already in the bulletin
    /// (round 5, R5-B): the presence check found every
    /// one of its paragraphs in the story the part would
    /// land in (round 6, R6-B), so applying it again
    /// would print the words twice. The apply step
    /// leaves the document untouched and counts the part
    /// as landed: it is where it belongs.
    AlreadyThere,
}

/// One part's slot, with the sentence the review shows.
#[derive(Debug, Clone, PartialEq)]
struct PartPlan {
    target: SlotTarget,
    line: String,
}

/// A story named for a set of words ("sunday school",
/// "youth"): the story's own id, or the name of a box
/// that shows it, compared without case and with dashes
/// and underscores read as spaces.
fn named_story_id(doc: &Document, words: &str) -> Option<String> {
    let norm = |s: &str| {
        s.chars()
            .map(|c| match c {
                '-' | '_' => ' ',
                c => c,
            })
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_ascii_lowercase()
    };
    let want = norm(words);
    if let Some(s) = doc.stories.iter().find(|s| norm(&s.id) == want) {
        return Some(s.id.clone());
    }
    doc.panels
        .iter()
        .flat_map(|p| p.frames.iter())
        .find(|f| norm(&f.name) == want)
        .and_then(|f| f.story_id.clone())
}

/// Where one line of a service part lands (R-2a, the
/// music slip): hymn lines slot into the bulletin's hymn
/// lines instead of piling up at the end of the service.
#[derive(Debug, Clone, PartialEq)]
enum HymnSlot {
    /// Set the existing service line at this paragraph
    /// index to the composed value: a named line replaced,
    /// or an empty line filled.
    Replace {
        para_index: usize,
        label: String,
        was_empty: bool,
        value: String,
    },
    /// A new service line at the end, labelled "Hymn".
    AppendHymn { value: String },
    /// The paragraph goes in as written, at the end.
    Plain,
}

/// A contribution line's own label and value, the way
/// senders write them: "Hymn of Praise\t433" (the tab
/// mirrors the bulletin's own lines) or "Opening hymn:
/// ELW 432". Anything else is all value, no label. The
/// value is everything after the FIRST tab (round 3,
/// R3-8e): a line typed as "Hymn\tELW 432\tJoyful,
/// Joyful" keeps its middle, so the hymnal and number
/// survive to be composed with the title.
fn part_line_label(text: &str) -> (Option<String>, String) {
    let t = text.trim();
    if let Some(tab) = t.find('\t') {
        let label = t[..tab].trim();
        let value = t[tab + 1..].trim();
        return (
            if label.is_empty() {
                None
            } else {
                Some(label.to_string())
            },
            value.to_string(),
        );
    }
    if let Some((head, tail)) = t.split_once(':') {
        let head = head.trim();
        if !head.is_empty()
            && head.split_whitespace().count() <= 6
            && !head.chars().any(|c| c.is_ascii_digit())
        {
            return (Some(head.to_string()), tail.trim().to_string());
        }
    }
    (None, t.to_string())
}

/// Decide where every line of a service part lands
/// against this document's service lines. A line is a
/// hymn entry only when it parses to a hymn value WITH
/// a number, and either its label is a hymn label or
/// the value itself names a hymnal (round 3, R3-8e):
/// a bare number can be a psalm, a page, or a time
/// ("Psalm\t23" stays a plain service line), and prose
/// under a hymn-flavoured label ("Hymn requests: see
/// the director") has no number and stays prose.
/// Named entries replace the hymn line they name; the
/// rest fill empty hymn lines in order; what is left
/// becomes a new line at the end. Pure, so the review's
/// plan and the apply step can both run it and always
/// agree.
fn hymn_slots(doc: &Document, part: &ob_extract::contribution::Part) -> Vec<HymnSlot> {
    use ob_model::hymns::{compose_hymn_value, is_hymn_label, parse_hymn_value};
    let lines = service_lines(doc);
    let mut used: Vec<usize> = Vec::new();
    let mut parsed: Vec<(Option<String>, ob_model::hymns::HymnEntry, bool)> = Vec::new();
    for p in &part.paragraphs {
        let text = p.text();
        let (label, value) = part_line_label(&text);
        let entry = parse_hymn_value(&value);
        let hymn_labelled = label.as_deref().map(is_hymn_label).unwrap_or(false);
        let valued = !text.trim().is_empty()
            && entry.number.is_some()
            && (hymn_labelled || entry.hymnal.is_some());
        parsed.push((label, entry, valued));
    }
    let mut slots: Vec<Option<HymnSlot>> = Vec::new();
    slots.resize_with(part.paragraphs.len(), || None);
    // Named first: a label that names an existing hymn
    // line replaces that line's value.
    for (i, (label, entry, valued)) in parsed.iter().enumerate() {
        if !valued {
            continue;
        }
        let Some(label) = label else { continue };
        let Some(line) = lines.iter().find(|l| {
            is_hymn_label(&l.label)
                && l.label.eq_ignore_ascii_case(label)
                && !used.contains(&l.para_index)
        }) else {
            continue;
        };
        used.push(line.para_index);
        slots[i] = Some(HymnSlot::Replace {
            para_index: line.para_index,
            label: line.label.clone(),
            was_empty: line.value.trim().is_empty(),
            value: compose_hymn_value(entry),
        });
    }
    // Then in order: what is left fills the empty hymn
    // lines.
    for (i, (_, entry, valued)) in parsed.iter().enumerate() {
        if !valued || slots[i].is_some() {
            continue;
        }
        let Some(line) = lines.iter().find(|l| {
            is_hymn_label(&l.label) && l.value.trim().is_empty() && !used.contains(&l.para_index)
        }) else {
            continue;
        };
        used.push(line.para_index);
        slots[i] = Some(HymnSlot::Replace {
            para_index: line.para_index,
            label: line.label.clone(),
            was_empty: true,
            value: compose_hymn_value(entry),
        });
    }
    // What is still standing: a hymn entry becomes a new
    // line; anything else goes in as written.
    for (i, (_, entry, valued)) in parsed.iter().enumerate() {
        if slots[i].is_none() {
            slots[i] = Some(if *valued {
                HymnSlot::AppendHymn {
                    value: compose_hymn_value(entry),
                }
            } else {
                HymnSlot::Plain
            });
        }
    }
    slots
        .into_iter()
        .map(|s| s.expect("every paragraph slotted"))
        .collect()
}

/// The review's words for a service part (R-2a): when the
/// part carries hymn entries, the plan names the hymn
/// lines they fill or replace; when it carries none, the
/// R-1 words stand.
fn service_plan_line(doc: &Document, part: &ob_extract::contribution::Part) -> String {
    let slots = hymn_slots(doc, part);
    if !slots
        .iter()
        .any(|s| matches!(s, HymnSlot::Replace { .. } | HymnSlot::AppendHymn { .. }))
    {
        return "Goes into the service lines, added at the end.".to_string();
    }
    let mut sentences: Vec<String> = Vec::new();
    let mut plain = 0usize;
    for slot in &slots {
        match slot {
            HymnSlot::Replace {
                label,
                was_empty,
                value,
                ..
            } => {
                if *was_empty {
                    sentences.push(format!("Fills the {label} line with {value}."));
                } else {
                    sentences.push(format!("The {label} line becomes {value}."));
                }
            }
            HymnSlot::AppendHymn { value } => {
                sentences.push(format!("Adds a hymn line at the end: {value}."));
            }
            HymnSlot::Plain => plain += 1,
        }
    }
    if plain == 1 {
        sentences.push("1 more line is added at the end.".to_string());
    } else if plain > 1 {
        sentences.push(format!("{plain} more lines are added at the end."));
    }
    sentences.join(" ")
}

/// The style the service lines wear, for a line the app
/// adds itself: the first tabbed line's style in the
/// service story, or body when there is none.
fn service_line_style(doc: &Document) -> String {
    let Some(sid) = service_story_id(doc) else {
        return "body".to_string();
    };
    let Some(story) = doc.story(&sid) else {
        return "body".to_string();
    };
    story
        .paragraphs
        .iter()
        .find(|p| p.text().contains('\t'))
        .map(|p| p.style.clone())
        .unwrap_or_else(|| "body".to_string())
}

/// Push one paragraph onto a story. Answers false when
/// the story is not there.
fn push_story_paragraph(doc: &mut Document, sid: &str, p: ob_model::Paragraph) -> bool {
    match doc.story_mut(sid) {
        Some(story) => {
            story.paragraphs.push(p);
            true
        }
        None => false,
    }
}

/// The date a story keeps through once a contribution's
/// date lands in it: the later of the story's own date
/// and the contribution's (round 3, R3-8c). An existing
/// date that does not read is simply replaced; the new
/// date can extend a stay, never silently shorten one.
fn effective_keep_through(
    doc: &Document,
    sid: &str,
    kt: ob_model::SimpleDate,
) -> ob_model::SimpleDate {
    if let Some(existing) = doc
        .story(sid)
        .and_then(|s| s.keep_through.as_deref())
        .and_then(|iso| ob_model::parse_flexible_date(iso, kt))
    {
        if existing > kt {
            return existing;
        }
    }
    kt
}

/// The keep-through sentence for a part landing in the
/// story `sid`, worded with the date that will actually
/// be in force there. Nothing when the contribution
/// names no date: then no date is in play.
fn stays_in_sentence(
    doc: &Document,
    sid: &str,
    c: &ob_extract::contribution::Contribution,
) -> String {
    let Some(kt) = c.keep_through else {
        return String::new();
    };
    format!(
        " Stays in through {}.",
        ob_extract::contribution::format_date_long(effective_keep_through(doc, sid, kt))
    )
}

/// Land a contribution's keep-through date in the story
/// that received its words (round 3, R3-8b): every arm
/// that places words, not only the announcement arm.
/// The later of the two dates wins (R3-8c).
fn apply_keep_through(doc: &mut Document, sid: &str, kt: ob_model::SimpleDate) {
    let effective = effective_keep_through(doc, sid, kt);
    if let Some(story) = doc.story_mut(sid) {
        story.keep_through = Some(format!(
            "{:04}-{:02}-{:02}",
            effective.year, effective.month, effective.day
        ));
    }
}

/// Decide one part's slot against this document.
fn plan_part(
    doc: &Document,
    c: &ob_extract::contribution::Contribution,
    part: &ob_extract::contribution::Part,
) -> PartPlan {
    // Round 5 (R5-B): a part whose words are already in
    // the bulletin is planned as already there, whatever
    // box it would otherwise have gone into, so the
    // review's sentence and the apply step agree that
    // nothing is added. Round 6 (R6-B): presence is
    // judged against the part's own planned target (see
    // `part_words_present`), never against the whole
    // document, so the plan target is decided first and
    // the presence judgment follows it.
    let plan = planned_part_plan(doc, c, part);
    if part_words_present(doc, c, part) {
        let line =
            "These words are already in the bulletin, so they are not added again.".to_string();
        return PartPlan {
            target: SlotTarget::AlreadyThere,
            line,
        };
    }
    plan
}

/// The slot a part would land in, decided without the
/// presence judgment: `plan_part`'s match, shared with
/// the presence machinery, which must know a part's
/// target before it can say whether the part's words
/// are already in the story that target names.
fn planned_part_plan(
    doc: &Document,
    c: &ob_extract::contribution::Contribution,
    part: &ob_extract::contribution::Part,
) -> PartPlan {
    use ob_extract::contribution::PartKind;
    let service = service_story_id(doc);
    let weekly = weekly_story_id(doc);
    match part.kind {
        PartKind::Service => match &service {
            Some(sid) => PartPlan {
                target: SlotTarget::Story(sid.clone()),
                line: format!(
                    "{}{}",
                    service_plan_line(doc, part),
                    stays_in_sentence(doc, sid, c)
                ),
            },
            None => {
                let mut line = "This bulletin has no service box, so these words go into \
                       the announcements, under a \u{201c}For the service\u{201d} heading."
                    .to_string();
                if let Some(wid) = &weekly {
                    line.push_str(&stays_in_sentence(doc, wid, c));
                }
                PartPlan {
                    target: SlotTarget::AnnouncementsUnderHeading("For the service"),
                    line,
                }
            }
        },
        PartKind::Announcement => match &weekly {
            Some(wid) => PartPlan {
                target: SlotTarget::Story(wid.clone()),
                line: format!(
                    "Goes into the announcements.{}",
                    stays_in_sentence(doc, wid, c)
                ),
            },
            // Round 3 (R3-8a): with no announcements box,
            // the plan says the words stay with the part.
            // Promising a box that is not there sent the
            // office looking for words Accept never placed.
            None => PartPlan {
                target: SlotTarget::KeptWithPart,
                line: "There is no announcements box in this bulletin yet, so these \
                       words are kept with the part when it is filed; nothing is \
                       added to the pages."
                    .to_string(),
            },
        },
        PartKind::SundaySchool | PartKind::Youth => {
            let words = if part.kind == PartKind::SundaySchool {
                "sunday school"
            } else {
                "youth"
            };
            match named_story_id(doc, words) {
                Some(sid) => PartPlan {
                    line: format!(
                        "Goes into the {words} box.{}",
                        stays_in_sentence(doc, &sid, c)
                    ),
                    target: SlotTarget::Story(sid),
                },
                None => {
                    let mut line = format!(
                        "Goes into the announcements, under a \u{201c}{}\u{201d} heading.",
                        part.kind.heading()
                    );
                    if let Some(wid) = &weekly {
                        line.push_str(&stays_in_sentence(doc, wid, c));
                    }
                    PartPlan {
                        target: SlotTarget::AnnouncementsUnderHeading(part.kind.heading()),
                        line,
                    }
                }
            }
        }
        PartKind::Insert => PartPlan {
            target: SlotTarget::LooseSheet,
            line: "A loose sheet. It is filed with this part and kept safe; \
                   it does not go into the bulletin pages."
                .to_string(),
        },
    }
}

/// Decide every part's slot, in file order. The plan is
/// a simulation (round 3, R3-8d): each part is planned
/// against a clone the earlier parts have already been
/// applied to, with the very function Accept will run,
/// so the sentences the review shows are the outcomes
/// Accept produces, including a second service part
/// slotting its hymns against the first part's work.
fn plan_parts(doc: &Document, c: &ob_extract::contribution::Contribution) -> Vec<PartPlan> {
    let mut sim = doc.clone();
    let mut plans = Vec::with_capacity(c.parts.len());
    for part in &c.parts {
        plans.push(plan_part(&sim, c, part));
        let single = ob_extract::contribution::Contribution {
            from: c.from.clone(),
            for_date: c.for_date,
            keep_through: c.keep_through,
            parts: vec![part.clone()],
            named_weekday_mismatch: c.named_weekday_mismatch.clone(),
            keep_through_weekday_mismatch: c.keep_through_weekday_mismatch.clone(),
            for_sunday_in_words: c.for_sunday_in_words.clone(),
            for_sunday_after_in_words: c.for_sunday_after_in_words.clone(),
        };
        apply_contribution(&mut sim, &single);
    }
    plans
}

/// A heading paragraph in the document's own styles: the
/// subheading style when the document has one, else body
/// words made bold (the markdown parser's own fallback).
fn heading_paragraph(doc: &Document, text: &str) -> ob_model::Paragraph {
    if let Some(s) = doc
        .styles
        .iter()
        .find(|s| s.id == "subheading" || s.name.eq_ignore_ascii_case("subheading"))
    {
        return ob_model::Paragraph::plain(&s.id, text);
    }
    let mut p = ob_model::Paragraph::plain("body", text);
    for r in &mut p.runs {
        if r.bold.is_none() {
            r.bold = Some(true);
        }
    }
    p
}

/// The running counts one apply builds toward its
/// summary sentences.
#[derive(Default)]
struct ApplyCounts {
    service_lines: usize,
    announcements: usize,
    named: usize,
    under_headings: usize,
    loose: usize,
    unplaced: usize,
    already: usize,
}

/// Split a part's paragraphs against one target story
/// (round 6, R6-B): the paragraphs still missing from
/// that story, in the part's own order, and how many
/// were already there. Presence spends the story's
/// paragraphs one by one, so a paragraph the part
/// carries twice needs the story to carry it twice
/// before both are skipped. An empty paragraph has no
/// words that could be missing; it lands as before.
fn split_present_paragraphs(
    doc: &Document,
    sid: &str,
    part: &ob_extract::contribution::Part,
) -> (Vec<ob_model::Paragraph>, usize) {
    let mut spent: Vec<usize> = Vec::new();
    let mut to_land: Vec<ob_model::Paragraph> = Vec::new();
    let mut skipped = 0usize;
    for p in &part.paragraphs {
        if normalized_words(&p.text()).is_empty() {
            to_land.push(p.clone());
            continue;
        }
        match story_paragraph_index(doc, sid, &p.text(), &spent) {
            Some(i) => {
                spent.push(i);
                skipped += 1;
            }
            None => to_land.push(p.clone()),
        }
    }
    (to_land, skipped)
}

/// Split an under-heading part's paragraphs against its
/// own heading's section of the weekly story (round 7,
/// R7-B): the paragraphs still missing FROM THAT
/// SECTION, and the section they are missing from.
/// Words standing elsewhere in the story do not skip a
/// paragraph here — the part's landing is its placement
/// under its heading, so a paragraph that stands only
/// elsewhere lands again, under the heading. When the
/// heading stands nowhere in the story there is no
/// section: every paragraph is to land, under a heading
/// the apply adds.
fn split_present_paragraphs_under_heading(
    doc: &Document,
    wid: &str,
    heading: &str,
    part: &ob_extract::contribution::Part,
) -> (Vec<ob_model::Paragraph>, usize, Option<(usize, usize)>) {
    let sections = heading_sections(doc, wid, heading);
    let Some(section) = sections.first().copied() else {
        let all = part.paragraphs.clone();
        return (all, 0, None);
    };
    let mut spent: Vec<usize> = Vec::new();
    let mut to_land: Vec<ob_model::Paragraph> = Vec::new();
    let mut skipped = 0usize;
    for p in &part.paragraphs {
        if normalized_words(&p.text()).is_empty() {
            to_land.push(p.clone());
            continue;
        }
        match story_paragraph_index_in_range(doc, wid, &p.text(), &spent, section) {
            Some(i) => {
                spent.push(i);
                skipped += 1;
            }
            None => to_land.push(p.clone()),
        }
    }
    (to_land, skipped, Some(section))
}

/// Put one part into the document, per its plan. The
/// plan is decided against the document as it stands
/// when the part lands, so an apply walks the same
/// ground the review's simulation walked. Answers
/// whether the part landed COMPLETELY (round 5, R5-B):
/// every one of its words placed, or already there, or
/// kept as a loose sheet is meant to be kept. The
/// applied record is written only for contributions
/// whose every part answers true, so a part that
/// landed partly or not at all is never remembered as
/// applied. Round 6 (R6-B): landing is per paragraph —
/// a paragraph already in the part's target story is
/// skipped and counted as already there, and only the
/// missing ones land, so a partly present part never
/// prints its present paragraphs twice.
fn apply_part(
    doc: &mut Document,
    c: &ob_extract::contribution::Contribution,
    part: &ob_extract::contribution::Part,
    plan: &PartPlan,
    counts: &mut ApplyCounts,
) -> bool {
    use ob_extract::contribution::PartKind;
    // The music slip (R-2a): a service part aimed at
    // the service story slots its hymn lines into the
    // bulletin's hymn lines, the way the review's plan
    // said, instead of appending the whole part.
    if part.kind == PartKind::Service {
        if let SlotTarget::Story(sid) = &plan.target {
            if service_story_id(doc).as_deref() == Some(sid.as_str()) {
                let slots = hymn_slots(doc, part);
                let mut spent: Vec<usize> = Vec::new();
                let mut landed = 0usize;
                let mut complete = 0usize;
                for (p, slot) in part.paragraphs.iter().zip(slots.iter()) {
                    match slot {
                        HymnSlot::Replace {
                            para_index, value, ..
                        } => {
                            // Replace stays as it is: setting
                            // the line to the same value is
                            // its own no-op.
                            set_service_line(doc, *para_index, value);
                            spent.push(*para_index);
                            landed += 1;
                            complete += 1;
                        }
                        HymnSlot::AppendHymn { value } => {
                            // Round 6 (R6-B): a hymn whose
                            // composed value already sits on
                            // a hymn line in the service story
                            // (the rule-2 judgment for this
                            // paragraph) counts as already
                            // there instead of appending a
                            // duplicate line.
                            if let Some(i) = find_hymn_line(doc, &p.text(), &spent) {
                                spent.push(i);
                                counts.already += 1;
                                complete += 1;
                            } else {
                                let style = service_line_style(doc);
                                let text = format!("Hymn\t{value}");
                                let para = ob_model::Paragraph::plain(&style, &text);
                                let new_index = doc.story(sid).map(|s| s.paragraphs.len());
                                if push_story_paragraph(doc, sid, para) {
                                    if let Some(i) = new_index {
                                        spent.push(i);
                                    }
                                    landed += 1;
                                    complete += 1;
                                }
                            }
                        }
                        HymnSlot::Plain => {
                            // A plain line already in the
                            // service story is skipped, not
                            // printed twice.
                            if let Some(i) = story_paragraph_index(doc, sid, &p.text(), &spent) {
                                spent.push(i);
                                counts.already += 1;
                                complete += 1;
                            } else {
                                let new_index = doc.story(sid).map(|s| s.paragraphs.len());
                                if push_story_paragraph(doc, sid, p.clone()) {
                                    if let Some(i) = new_index {
                                        spent.push(i);
                                    }
                                    landed += 1;
                                    complete += 1;
                                }
                            }
                        }
                    }
                }
                counts.service_lines += landed;
                counts.unplaced += part.paragraphs.len() - complete;
                if landed > 0 {
                    if let Some(kt) = c.keep_through {
                        apply_keep_through(doc, sid, kt);
                    }
                }
                return complete == part.paragraphs.len();
            }
        }
    }
    match &plan.target {
        SlotTarget::Story(sid) => {
            if doc.story(sid).is_none() {
                counts.unplaced += part.paragraphs.len();
                return false;
            }
            // Per-paragraph landing: the paragraphs
            // already in this story are skipped, only the
            // missing ones land. The part is complete when
            // every paragraph ended up in its target,
            // landed or already there, and the target
            // story exists.
            let (to_land, skipped) = split_present_paragraphs(doc, sid, part);
            let landed = to_land.len();
            if landed > 0 {
                if let Some(story) = doc.story_mut(sid) {
                    story.paragraphs.extend(to_land);
                }
            }
            match part.kind {
                PartKind::Service => counts.service_lines += landed,
                PartKind::Announcement => counts.announcements += landed,
                _ => counts.named += landed,
            }
            counts.already += skipped;
            if landed > 0 {
                if let Some(kt) = c.keep_through {
                    apply_keep_through(doc, sid, kt);
                }
            }
            true
        }
        SlotTarget::AnnouncementsUnderHeading(heading) => {
            let wid = weekly_story_id(doc);
            let Some(wid) = wid.as_ref() else {
                counts.unplaced += part.paragraphs.len();
                return false;
            };
            if doc.story(wid).is_none() {
                counts.unplaced += part.paragraphs.len();
                return false;
            }
            // Round 7 (R7-B): presence and landing are
            // scoped to the part's own heading section of
            // the weekly story. Paragraphs already under
            // the heading are skipped; the missing ones
            // land under the heading even when the same
            // words stand elsewhere in the story. When
            // the heading already stands, the missing
            // paragraphs go in at the end of its section;
            // when it stands nowhere, the heading is
            // added (only when a paragraph actually lands
            // under it, or the part carries no words at
            // all) with the paragraphs after it.
            let (to_land, skipped, section) =
                split_present_paragraphs_under_heading(doc, wid, heading, part);
            let landed = to_land.len();
            match section {
                Some(section) => {
                    if landed > 0 {
                        if let Some(story) = doc.story_mut(wid) {
                            let at = section.1.min(story.paragraphs.len());
                            for (n, p) in to_land.into_iter().enumerate() {
                                story.paragraphs.insert(at + n, p);
                            }
                        }
                        counts.under_headings += landed;
                        if let Some(kt) = c.keep_through {
                            apply_keep_through(doc, wid, kt);
                        }
                    }
                }
                None => {
                    if landed > 0 || part.paragraphs.is_empty() {
                        let head = heading_paragraph(doc, heading);
                        if let Some(story) = doc.story_mut(wid) {
                            story.paragraphs.push(head);
                            story.paragraphs.extend(to_land);
                        }
                        counts.under_headings += landed;
                        if landed > 0 {
                            if let Some(kt) = c.keep_through {
                                apply_keep_through(doc, wid, kt);
                            }
                        }
                    }
                }
            }
            counts.already += skipped;
            // The part counts as landed only when its
            // heading and its words both stand, judged
            // with the one presence notion.
            match target_checker(doc, part, &plan.target) {
                Some(mut checker) => checker.part_present(part),
                None => false,
            }
        }
        // A loose sheet lands by being kept with the
        // part, exactly as its plan said: complete.
        SlotTarget::LooseSheet => {
            counts.loose += 1;
            true
        }
        SlotTarget::KeptWithPart => {
            counts.unplaced += part.paragraphs.len();
            // Nothing landed, unless the part carried no
            // words at all: then there was nothing to lose.
            part.paragraphs.is_empty()
        }
        SlotTarget::AlreadyThere => {
            counts.already += part.paragraphs.len();
            true
        }
    }
}

/// What one apply did, in full: the summary lines
/// Accept reports, and whether every part landed
/// completely (round 5, R5-B). The applied record is
/// written only when `all_landed` is true.
struct ApplyOutcome {
    lines: Vec<String>,
    all_landed: bool,
}

/// The sentence that names a part whose words did not
/// all land, and says why in the office's words. The
/// words themselves stay with the part when it is
/// filed, as the plan already promised.
fn unlanded_sentence(part: &ob_extract::contribution::Part, plan: &PartPlan) -> String {
    let kind = part.kind.heading();
    match &plan.target {
        SlotTarget::KeptWithPart => format!(
            "The {kind} words are not in the bulletin: there is no \
             announcements box in it yet, so the words stay with \
             the part when it is filed."
        ),
        _ => format!(
            "The {kind} words are not all in the bulletin: the box \
             they belong in could not be found in it, so the words \
             stay with the part when it is filed."
        ),
    }
}

/// Put a reviewed contribution's parts into the document,
/// each planned against the document the parts before
/// it left behind, exactly as the review's plan was
/// simulated. Returns the summary lines Accept reports:
/// counts per part, the loose sheet's fate said out
/// loud, and a naming sentence for every part whose
/// words did not all land.
fn apply_contribution_outcome(
    doc: &mut Document,
    c: &ob_extract::contribution::Contribution,
) -> ApplyOutcome {
    let mut counts = ApplyCounts::default();
    let mut all_landed = true;
    let mut unlanded: Vec<String> = Vec::new();
    for part in &c.parts {
        let plan = plan_part(doc, c, part);
        if !apply_part(doc, c, part, &plan, &mut counts) {
            all_landed = false;
            unlanded.push(unlanded_sentence(part, &plan));
        }
    }
    let ApplyCounts {
        service_lines,
        announcements,
        named,
        under_headings,
        loose,
        unplaced,
        already,
    } = counts;
    if announcements + under_headings > 0 {
        // Words arrived: the office is no longer saying
        // there are no announcements this week.
        doc.no_announcements = false;
    }
    let mut out: Vec<String> = Vec::new();
    if service_lines > 0 {
        out.push(format!(
            "Added {service_lines} line{} to the service.",
            if service_lines == 1 { "" } else { "s" }
        ));
    }
    if announcements > 0 {
        out.push(format!(
            "Added {announcements} announcement line{}.",
            if announcements == 1 { "" } else { "s" }
        ));
    }
    if named > 0 {
        out.push(format!(
            "Added {named} line{} to their boxes.",
            if named == 1 { "" } else { "s" }
        ));
    }
    if under_headings > 0 {
        out.push(format!(
            "Added {under_headings} line{} to the announcements, under their headings.",
            if under_headings == 1 { "" } else { "s" }
        ));
    }
    if loose > 0 {
        out.push("The loose sheet is filed with this part and kept safe.".to_string());
    }
    if unplaced > 0 {
        out.push(format!(
            "{unplaced} line{} had no box to go into; they are filed with this part.",
            if unplaced == 1 { "" } else { "s" }
        ));
    }
    if already > 0 {
        out.push(if already == 1 {
            "1 line was already in the bulletin, so it was not added again.".to_string()
        } else {
            format!("{already} lines were already in the bulletin, so they were not added again.")
        });
    }
    out.extend(unlanded);
    ApplyOutcome {
        lines: out,
        all_landed,
    }
}

/// Apply a contribution and report only the summary
/// lines: the shape the review's plan simulation and
/// the older tests use. Accept itself goes through
/// `apply_contribution_recording`, which also knows
/// whether the words may be remembered as applied.
fn apply_contribution(
    doc: &mut Document,
    c: &ob_extract::contribution::Contribution,
) -> Vec<String> {
    apply_contribution_outcome(doc, c).lines
}

/// Apply a reviewed contribution the way Accept does,
/// and record it in the document when, and only when,
/// every part landed completely (round 5, R5-B). A
/// part that landed partly or not at all leaves no
/// record behind, so the review can offer its words
/// again honestly instead of swearing they are in the
/// bulletin when they are not.
fn apply_contribution_recording(
    doc: &mut Document,
    c: &ob_extract::contribution::Contribution,
    text: &str,
) -> ApplyOutcome {
    let outcome = apply_contribution_outcome(doc, c);
    if outcome.all_landed {
        record_applied_fingerprint(doc, text);
    }
    outcome
}

/// One file-name piece made safe for any folder: letters,
/// digits, spaces, and dashes stay; everything else
/// becomes a dash.
fn sanitize_file_part(s: &str) -> String {
    let cleaned: String = s
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == ' ' || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let cleaned = cleaned.trim().trim_matches('-').trim().to_string();
    if cleaned.is_empty() {
        "no name".to_string()
    } else {
        cleaned
    }
}

/// The name a contribution is filed under: sender, Sunday,
/// and content fingerprint, so which words were used stays
/// answerable later.
fn filed_file_name(
    from: Option<&str>,
    for_date: Option<ob_model::SimpleDate>,
    fingerprint: &str,
) -> String {
    let sender = sanitize_file_part(from.unwrap_or(""));
    let sunday = for_date
        .map(|d| format!("{:04}-{:02}-{:02}", d.year, d.month, d.day))
        .unwrap_or_else(|| "no Sunday named".to_string());
    format!("{sender} - {sunday} - {fingerprint}.md")
}

/// Rebuild a contribution's text from its pieces (the
/// review's Edit mode): the header lines as written, then
/// each part under its heading. The rebuilt text is what
/// gets re-parsed, and what gets filed after an edit.
fn rebuild_contribution_text(
    from: &str,
    for_sunday: &str,
    keep_through: &str,
    parts: &[(ob_extract::contribution::PartKind, String)],
) -> String {
    let mut out = String::new();
    if !from.trim().is_empty() {
        out.push_str(&format!("From: {}\n", from.trim()));
    }
    if !for_sunday.trim().is_empty() {
        out.push_str(&format!("For Sunday: {}\n", for_sunday.trim()));
    }
    if !keep_through.trim().is_empty() {
        out.push_str(&format!("Keep-through: {}\n", keep_through.trim()));
    }
    for (kind, body) in parts {
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&format!("## {}\n\n{}\n", kind.heading(), body.trim()));
    }
    out
}

/// The sender's card as markdown: one page that teaches
/// the whole contribution shape, example included. It is
/// the entire training program, so the words are written
/// for the sender, not about them.
fn sender_card_markdown(church_name: &str) -> String {
    let church = if church_name.trim().is_empty() {
        "your church office".to_string()
    } else {
        church_name.trim().to_string()
    };
    format!(
        "# Sending words for the bulletin\n\
         \n\
         From {church}\n\
         \n\
         Anyone can send words for the bulletin: the pastor, the music director, \
         Sunday school, the youth group, or you. You do not need any program. \
         Type your words in an email or a notes app, in the shape below, and send \
         them to the church office.\n\
         \n\
         ## The shape\n\
         \n\
         Start with two short lines:\n\
         \n\
         From: your name or your job (Pastor, Music director)\n\
         \n\
         For Sunday: the date, written any way you like (October 25, 2026)\n\
         \n\
         Then give each part its own heading, spelled like one of these:\n\
         \n\
         ## For the service\n\
         \n\
         ## Announcement\n\
         \n\
         ## Insert\n\
         \n\
         ## Sunday school\n\
         \n\
         ## Youth\n\
         \n\
         Under each heading, type the words just as they should print. \
         **Bold** and *italic* work the way you would expect.\n\
         \n\
         An announcement can stay in for a few weeks. Add a third line at the \
         top: Keep-through: the last Sunday it should print.\n\
         \n\
         ## An example\n\
         \n\
         From: Pastor Lee\n\
         \n\
         For Sunday: October 25, 2026\n\
         \n\
         ## For the service\n\
         \n\
         Sermon: **The Good Shepherd**\n\
         \n\
         ## Announcement\n\
         \n\
         Harvest supper is Saturday at 5 PM. Bring a dish to share.\n\
         \n\
         Please send your words by ______________ (the office fills this in).\n"
    )
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

/// Where contributions wait: an "inbox" folder beside the
/// Font Library folder (the clips discipline), with a
/// "filed" subfolder for parts already used.
fn inbox_dir() -> std::path::PathBuf {
    let fonts = ob_fonts::default_library_dir();
    match fonts.parent() {
        Some(base) => base.join("inbox"),
        None => std::path::PathBuf::from("inbox"),
    }
}

fn filed_dir() -> std::path::PathBuf {
    inbox_dir().join("filed")
}

/// Where the loose sheet library lives: an "inserts"
/// folder beside the Font Library folder (the clips and
/// inbox discipline), holding one .obdi file per sheet.
fn insert_library_dir() -> std::path::PathBuf {
    let fonts = ob_fonts::default_library_dir();
    match fonts.parent() {
        Some(base) => base.join("inserts"),
        None => std::path::PathBuf::from("inserts"),
    }
}

/// Exchange a document's panels, stories, and styles
/// with one of its loose sheets' (R-3): the canvas swap.
/// The swap is its own inverse, so entering and leaving
/// a sheet's edit are the same act. Nothing is copied
/// and nothing is duplicated: the parts live in exactly
/// one of the two places at any moment. False when the
/// document holds no sheet under that id.
fn swap_insert_parts(doc: &mut Document, id: &str) -> bool {
    let Some(pos) = doc.inserts.iter().position(|i| i.id == id) else {
        return false;
    };
    std::mem::swap(&mut doc.panels, &mut doc.inserts[pos].panels);
    std::mem::swap(&mut doc.stories, &mut doc.inserts[pos].stories);
    std::mem::swap(&mut doc.styles, &mut doc.inserts[pos].styles);
    true
}

/// An id no sheet in this document already uses.
fn fresh_insert_id(doc: &Document) -> String {
    let mut n = doc.inserts.len() + 1;
    loop {
        let id = format!("sheet-{n}");
        if !doc.inserts.iter().any(|i| i.id == id) {
            return id;
        }
        n += 1;
    }
}

/// A new loose sheet: one panel of the half-letter
/// shape, one text box, one empty story, this week only
/// until the office says otherwise (R-3).
fn new_insert(id: &str, name: &str) -> ob_model::Insert {
    let size = ob_model::insert_sheet_size("half-letter").expect("half-letter is a sheet shape");
    let m = 36_000; // a half-inch margin, in milli-points
    ob_model::Insert {
        id: id.to_string(),
        name: name.to_string(),
        life: ob_model::InsertLife::ThisWeekOnly,
        keep_through: None,
        panels: vec![ob_model::Panel {
            id: format!("{id}-p1"),
            name: name.to_string(),
            size,
            frames: vec![ob_model::Frame {
                id: format!("{id}-f1"),
                name: "Words".into(),
                kind: ob_model::FrameKind::Text,
                rect: ob_model::Rect::new(m, m, size.w - 2 * m, size.h - 2 * m),
                story_id: Some(format!("{id}-s1")),
                role: ob_model::ContentRole::Standing,
                picture: None,
                music: None,
            }],
            guides: vec![],
            background_image: None,
        }],
        stories: vec![ob_model::Story {
            id: format!("{id}-s1"),
            paragraphs: vec![ob_model::Paragraph::new("body", "")],
            keep_through: None,
        }],
        styles: ob_model::default_styles(),
        sheet: ob_model::InsertSheet::default(),
    }
}

/// A sheet's name as a file name's middle: letters,
/// numbers, spaces, and dashes stay; anything a file
/// system would trip on becomes a dash.
fn insert_file_stem(name: &str) -> String {
    let stem: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == ' ' || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let stem = stem.trim();
    if stem.is_empty() {
        "Loose sheet".to_string()
    } else {
        stem.to_string()
    }
}

/// A sheet's life in the words the office hears.
fn insert_life_words(ins: &ob_model::Insert) -> String {
    match ins.life {
        ob_model::InsertLife::Standing => "Standing: rides every week".to_string(),
        ob_model::InsertLife::ThisWeekOnly => "This week only".to_string(),
        ob_model::InsertLife::Dated => match ins.keep_through {
            Some(d) => format!("Until {}", ob_layout::long_date(naive_of(d))),
            None => "Until a date (no date set yet)".to_string(),
        },
    }
}

/// A sheet's size in words: "Half letter".
fn insert_size_words(sheet: &ob_model::InsertSheet) -> String {
    for (key, words) in ob_model::INSERT_SHEET_SHAPES {
        if key == sheet.preset {
            return words.split(" (").next().unwrap_or(words).to_string();
        }
    }
    sheet.preset.clone()
}

/// How many copies, in words: zero is the usual case
/// and means the sheet prints with the bulletin run.
fn insert_copies_words(sheet: &ob_model::InsertSheet) -> String {
    match sheet.copies {
        0 => "as many as the bulletin".to_string(),
        1 => "1 copy".to_string(),
        n => format!("{n} copies"),
    }
}

/// The paper, in words: no note means plain paper.
fn insert_paper_words(sheet: &ob_model::InsertSheet) -> String {
    let note = sheet.paper_note.trim();
    if note.is_empty() {
        "plain paper".to_string()
    } else {
        note.to_string()
    }
}

/// The Check findings only loose sheets can have (R-3):
/// a sheet with no words would print as a blank sheet of
/// colored paper, and a dated sheet whose date has
/// passed is still riding until Next Sunday takes it
/// out, so Check names it before it prints stale. Both
/// aim at the Loose sheets window, where the fix lives.
fn insert_findings(doc: &Document) -> Vec<CheckNote> {
    let mut out = Vec::new();
    let doc_date = doc
        .fields
        .get("occasion_date")
        .and_then(|iso| chrono::NaiveDate::parse_from_str(iso.trim(), "%Y-%m-%d").ok())
        .map(simple_of);
    for ins in &doc.inserts {
        if !ins.has_words() {
            out.push(CheckNote::to_inserts(format!(
                "Check: the loose sheet '{}' has no words yet.",
                ins.name
            )));
        }
        if ins.life == ob_model::InsertLife::Dated {
            if let (Some(until), Some(date)) = (ins.keep_through, doc_date) {
                if until.is_valid() && until < date {
                    out.push(CheckNote::to_inserts(format!(
                        "Check: the loose sheet '{}' was kept through {}, which has passed.",
                        ins.name,
                        ob_layout::long_date(naive_of(until))
                    )));
                }
            }
        }
    }
    out
}

/// Words compared as words (round 5, R5-B): case
/// folded and every run of whitespace, tabs included,
/// read as one space. Presence is a question about the
/// words, not about the bytes they arrived in, so a
/// part re-sent with different spacing is still the
/// same words.
fn normalized_words(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Whether `needle` appears in `entry` as whole words:
/// the whole entry, or fenced by spaces on the side it
/// does not run off. Both sides are normalized first,
/// so a short line cannot claim presence by hiding
/// inside a longer word.
fn contains_words(entry: &str, needle: &str) -> bool {
    !needle.is_empty()
        && (entry == needle
            || entry.starts_with(&format!("{needle} "))
            || entry.ends_with(&format!(" {needle}"))
            || entry.contains(&format!(" {needle} ")))
}

/// The composed value a service part's hymn line lands
/// as, when the line is a hymn entry: the same test
/// `hymn_slots` slots by (a number, and a hymn label
/// or a named hymnal). A hymn line is not copied into
/// the bulletin word for word; it is composed into
/// the bulletin's own hymn line, so its presence is
/// judged by the composed form.
fn hymn_composed_form(text: &str) -> Option<String> {
    use ob_model::hymns::{compose_hymn_value, is_hymn_label, parse_hymn_value};
    let (label, value) = part_line_label(text);
    let entry = parse_hymn_value(&value);
    let hymn_labelled = label.as_deref().map(is_hymn_label).unwrap_or(false);
    if !text.trim().is_empty()
        && entry.number.is_some()
        && (hymn_labelled || entry.hymnal.is_some())
    {
        Some(compose_hymn_value(&entry))
    } else {
        None
    }
}

/// The index of the paragraph in story `sid` whose
/// words contain `text`'s words, skipping the indexes
/// in `spent`. Presence is scoped to one story (round
/// 6, R6-B): words standing in some other story never
/// count for a part that would land here. A match
/// spends its paragraph, so a line the part carries
/// twice needs the story to carry it twice.
fn story_paragraph_index(doc: &Document, sid: &str, text: &str, spent: &[usize]) -> Option<usize> {
    let needle = normalized_words(text);
    if needle.is_empty() {
        return None;
    }
    let story = doc.story(sid)?;
    story
        .paragraphs
        .iter()
        .enumerate()
        .find(|(i, p)| !spent.contains(i) && contains_words(&normalized_words(&p.text()), &needle))
        .map(|(i, _)| i)
}

/// Whether a paragraph reads as a heading in a story
/// (round 7, R7-B): it wears a heading style, or it is
/// the bold body paragraph `heading_paragraph` falls
/// back to when the document has no heading style.
fn is_heading_paragraph(p: &ob_model::Paragraph) -> bool {
    if p.style == "heading" || p.style == "subheading" {
        return true;
    }
    !p.runs.is_empty()
        && !normalized_words(&p.text()).is_empty()
        && p.runs.iter().all(|r| r.bold == Some(true))
}

/// The sections of story `sid` that stand under a
/// heading with these words (round 7, R7-B): each
/// section is the paragraph range (start, end) after
/// one heading paragraph and before the next heading
/// paragraph of any words. An under-heading part's
/// words count as present only inside one of its own
/// heading's sections: the same words standing
/// elsewhere in the story — before the heading, or
/// under another heading — are not its presence.
fn heading_sections(doc: &Document, sid: &str, heading: &str) -> Vec<(usize, usize)> {
    let Some(story) = doc.story(sid) else {
        return Vec::new();
    };
    let want = normalized_words(heading);
    let paras = &story.paragraphs;
    let mut sections = Vec::new();
    let mut i = 0usize;
    while i < paras.len() {
        if is_heading_paragraph(&paras[i]) && normalized_words(&paras[i].text()) == want {
            let start = i + 1;
            let mut end = start;
            while end < paras.len() && !is_heading_paragraph(&paras[end]) {
                end += 1;
            }
            sections.push((start, end));
            i = end;
        } else {
            i += 1;
        }
    }
    sections
}

/// `story_paragraph_index` restricted to one paragraph
/// range of the story (round 7, R7-B): the search an
/// under-heading part's presence and landing share, so
/// both judge the same section the same way.
fn story_paragraph_index_in_range(
    doc: &Document,
    sid: &str,
    text: &str,
    spent: &[usize],
    range: (usize, usize),
) -> Option<usize> {
    let needle = normalized_words(text);
    if needle.is_empty() {
        return None;
    }
    let story = doc.story(sid)?;
    story
        .paragraphs
        .iter()
        .enumerate()
        .skip(range.0)
        .take(range.1.saturating_sub(range.0))
        .find(|(i, p)| !spent.contains(i) && contains_words(&normalized_words(&p.text()), &needle))
        .map(|(i, _)| i)
}

/// The paragraph index of the service-story hymn line
/// carrying a hymn paragraph's composed value, when
/// the paragraph counts as present (round 6, R6-B,
/// rule 2). A hymn-valued paragraph is present only on
/// a hymn line in the service story: when the paragraph
/// names a line (its label), the value must sit on a
/// service line with that label, or on an appended line
/// labelled "Hymn"; when it names none, any
/// hymn-labelled service line carrying the value
/// counts. Text anywhere else — announcements, other
/// stories, non-hymn service lines — never counts.
/// Indexes in `spent` are skipped, so each line can
/// satisfy one paragraph.
fn find_hymn_line(doc: &Document, text: &str, spent: &[usize]) -> Option<usize> {
    let composed = normalized_words(&hymn_composed_form(text)?);
    if composed.is_empty() {
        return None;
    }
    let (label, _) = part_line_label(text);
    service_lines(doc)
        .into_iter()
        .find(|line| {
            if spent.contains(&line.para_index) {
                return false;
            }
            let label_ok = match &label {
                Some(named) => {
                    line.label.eq_ignore_ascii_case(named)
                        || line.label.eq_ignore_ascii_case("Hymn")
                }
                None => ob_model::hymns::is_hymn_label(&line.label),
            };
            label_ok && contains_words(&normalized_words(&line.value), &composed)
        })
        .map(|line| line.para_index)
}

/// Presence judged against one target story (round 6,
/// R6-B): the paragraphs of that story only, each
/// spendable once. In hymn mode (a service part whose
/// target is the service story itself), hymn-valued
/// paragraphs are judged by the hymn-line rule and the
/// rest by their words in the service story.
struct TargetPresence<'a> {
    doc: &'a Document,
    sid: Option<String>,
    hymn_mode: bool,
    spent: Vec<usize>,
    /// Round 7 (R7-B): for an under-heading part, the
    /// one section of the target story the part's words
    /// must stand in (see `heading_sections`). None
    /// searches the whole story, as a plain story part
    /// always has.
    range: Option<(usize, usize)>,
    /// Round 7 (R7-B): the placement itself does not
    /// stand — an under-heading part whose heading is
    /// nowhere in the target story. Nothing is present
    /// for such a part, however its words stand
    /// elsewhere in the story.
    absent: bool,
}

impl TargetPresence<'_> {
    fn paragraph_present(&mut self, text: &str) -> bool {
        if self.absent {
            return false;
        }
        if normalized_words(text).is_empty() {
            return true;
        }
        if self.hymn_mode && hymn_composed_form(text).is_some() {
            match find_hymn_line(self.doc, text, &self.spent) {
                Some(i) => {
                    self.spent.push(i);
                    return true;
                }
                None => return false,
            }
        }
        let Some(sid) = self.sid.clone() else {
            return false;
        };
        let found = match self.range {
            Some(range) => story_paragraph_index_in_range(self.doc, &sid, text, &self.spent, range),
            None => story_paragraph_index(self.doc, &sid, text, &self.spent),
        };
        match found {
            Some(i) => {
                self.spent.push(i);
                true
            }
            None => false,
        }
    }

    fn part_present(&mut self, part: &ob_extract::contribution::Part) -> bool {
        if self.absent {
            return false;
        }
        part.paragraphs
            .iter()
            .all(|p| self.paragraph_present(&p.text()))
    }
}

/// The presence checker for a part's planned target:
/// the story it would land in (the weekly story for an
/// under-heading part). None when the target names no
/// story a checker can search.
fn target_checker<'a>(
    doc: &'a Document,
    part: &ob_extract::contribution::Part,
    target: &SlotTarget,
) -> Option<TargetPresence<'a>> {
    use ob_extract::contribution::PartKind;
    match target {
        SlotTarget::Story(sid) => {
            let hymn_mode = part.kind == PartKind::Service
                && service_story_id(doc).as_deref() == Some(sid.as_str());
            Some(TargetPresence {
                doc,
                sid: Some(sid.clone()),
                hymn_mode,
                spent: Vec::new(),
                range: None,
                absent: false,
            })
        }
        SlotTarget::AnnouncementsUnderHeading(heading) => {
            // Round 7 (R7-B): an under-heading part is
            // present only under its own heading. The
            // checker searches one section of the weekly
            // story — the first section that holds the
            // part's whole words, or the first section
            // when none does (so the judgment fails
            // there, exactly as the landing will read
            // it). No section at all: the placement is
            // absent, and the part is not present
            // however its words stand elsewhere.
            let wid = weekly_story_id(doc)?;
            let sections = heading_sections(doc, &wid, heading);
            let (range, absent) = match sections.first() {
                None => (Some((0, 0)), true),
                Some(first) => {
                    let chosen = sections
                        .iter()
                        .copied()
                        .find(|range| {
                            let mut spent: Vec<usize> = Vec::new();
                            part.paragraphs.iter().all(|p| {
                                let text = p.text();
                                if normalized_words(&text).is_empty() {
                                    return true;
                                }
                                match story_paragraph_index_in_range(
                                    doc, &wid, &text, &spent, *range,
                                ) {
                                    Some(i) => {
                                        spent.push(i);
                                        true
                                    }
                                    None => false,
                                }
                            })
                        })
                        .unwrap_or(*first);
                    (Some(chosen), false)
                }
            };
            Some(TargetPresence {
                doc,
                sid: Some(wid),
                hymn_mode: false,
                spent: Vec::new(),
                range,
                absent,
            })
        }
        _ => None,
    }
}

/// Whether a part carries no words at all: every
/// paragraph empty. There is nothing that could be
/// missing, so such a part is present trivially
/// wherever it would land — the one case a part with
/// no landing story (KeptWithPart) counts as present.
fn part_has_no_words(part: &ob_extract::contribution::Part) -> bool {
    part.paragraphs
        .iter()
        .all(|p| normalized_words(&p.text()).is_empty())
}

/// Whether every one of a part's paragraphs is present
/// in the story the part would land in, the way a
/// complete landing would leave them (round 6, R6-B;
/// round 7, R7-B: for an under-heading part, present
/// means standing under its own heading — the same
/// words elsewhere in the story are not its presence,
/// and a missing heading means the part is not
/// present). A part is more than its first line: one
/// shared boilerplate line cannot make a part "already
/// in" while the rest of its words are missing, and
/// words standing in another story cannot make it
/// present here. A loose-sheet part is never present
/// by words: its landing is its filing, judged by the
/// record in `applied_claim`, so the plan never calls
/// it AlreadyThere.
fn part_words_present(
    doc: &Document,
    c: &ob_extract::contribution::Contribution,
    part: &ob_extract::contribution::Part,
) -> bool {
    let plan = planned_part_plan(doc, c, part);
    match &plan.target {
        SlotTarget::LooseSheet => false,
        SlotTarget::KeptWithPart => part_has_no_words(part),
        SlotTarget::AlreadyThere => true,
        target => match target_checker(doc, part, target) {
            Some(mut checker) => checker.part_present(part),
            None => part_has_no_words(part),
        },
    }
}

/// Whether every story-landing part of a contribution
/// has all its words present in its own target, judged
/// part by part with the very judgment the plan uses
/// (`part_words_present`), so the claim and the plan
/// lines can never disagree (round 7, R7-B). Each part
/// gets a fresh checker: one standing copy placed as a
/// landing places it satisfies every part that asked
/// for exactly that placement, so a paragraph repeated
/// across two parts settles instead of demanding the
/// words stand twice. Loose-sheet parts are not judged
/// here at all: their words never enter a story, so
/// their presence is the applied record's business
/// (see `applied_claim`).
fn contribution_words_present(doc: &Document, c: &ob_extract::contribution::Contribution) -> bool {
    for part in &c.parts {
        let plan = planned_part_plan(doc, c, part);
        match &plan.target {
            SlotTarget::LooseSheet => continue,
            SlotTarget::AlreadyThere => continue,
            _ => {
                if !part_words_present(doc, c, part) {
                    return false;
                }
            }
        }
    }
    true
}

/// Whether any part of a contribution lands as a loose
/// sheet: filed with the part, never in a story.
fn contribution_has_loose_part(doc: &Document, c: &ob_extract::contribution::Contribution) -> bool {
    c.parts.iter().any(|part| {
        matches!(
            planned_part_plan(doc, c, part).target,
            SlotTarget::LooseSheet
        )
    })
}

/// What the review may honestly say about a part and
/// this bulletin (round 5, R5-B). The applied record
/// alone never swears the words are in: the claim of
/// "already in" is made only when the words themselves
/// are found in the document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AppliedClaim {
    /// The words are in the bulletin (with or without
    /// a record: a re-sent part whose bytes differ
    /// still lands on its words). File it; applying
    /// again would print the words twice.
    AlreadyIn,
    /// The document's record says the words were
    /// applied, but they are not in it now (deleted by
    /// hand after a failed save, the road round 5's
    /// RT5-3 walked). Accept is offered again, under a
    /// sentence that says exactly this.
    RecordedButWordsGone,
    /// Neither a record nor the words: an ordinary
    /// review, with an ordinary Accept.
    NotApplied,
}

/// The claim for one reviewed contribution against
/// this document (round 6, R6-B, rule 3). Story-landing
/// parts are judged by target-scoped presence; a loose
/// part's landing IS its filing, so loose parts are
/// judged by the record. AlreadyIn: every story part is
/// present in its target, and the contribution either
/// carries no loose parts or is recorded. A loose-only
/// contribution is therefore AlreadyIn exactly when
/// recorded, and NotApplied when not.
fn applied_claim(
    doc: &Document,
    c: &ob_extract::contribution::Contribution,
    text: &str,
) -> AppliedClaim {
    let recorded = part_already_applied(doc, text);
    let story_present = contribution_words_present(doc, c);
    let has_loose = contribution_has_loose_part(doc, c);
    if story_present && (!has_loose || recorded) {
        AppliedClaim::AlreadyIn
    } else if recorded {
        AppliedClaim::RecordedButWordsGone
    } else {
        AppliedClaim::NotApplied
    }
}

/// A short content fingerprint: the first eight letters
/// of the content hash. Two versions of a part with the
/// same fingerprint are the same words.
fn content_fingerprint(bytes: &[u8]) -> String {
    ob_model::sha256_hex(bytes).chars().take(8).collect()
}

/// The review's first line when the part under review
/// was already applied to the bulletin on screen in
/// this session (round 3, R3-5): said once, in these
/// words, and pinned by a test. Round 5 (R5-B): the
/// line is earned by the claim in `applied_claim`,
/// which finds the words themselves in the bulletin;
/// the record alone no longer swears it.
const APPLIED_PART_FIRST_LINE: &str = "These words are already in the bulletin on screen. \
    Saving the bulletin finishes this part; there is nothing more to add.";

/// The review's first line when the document's record
/// says a part was applied but its words are not in
/// the bulletin now (round 5, R5-B): the record is a
/// note, not the words, and the office is told the
/// words can simply be added again.
const APPLIED_RECORD_MISSING_LINE: &str = "Our note says these words were added to this \
    bulletin before, but they are not in it now, so you can add them again.";

/// Whether the document's record says these exact
/// words were applied to it. The record lives in the
/// document itself (round 4, R4-4/R4-6), so it survives
/// a save that failed, a save done by hand, and a
/// crash the journal recovers from; Undo takes it away
/// with the words, and Next Sunday starts the new week
/// without it. The fingerprint is the same one the
/// filed name carries, so the file a failed save left
/// in the inbox and the words on screen compare equal.
/// Round 5 (R5-B): this is the record half of the
/// story only. What the review may claim is decided
/// by `applied_claim`, which also looks for the words
/// themselves: a record whose words were deleted by
/// hand no longer blocks Accept, and a re-sent part
/// whose bytes changed is still caught by its words.
fn part_already_applied(doc: &Document, text: &str) -> bool {
    doc.applied_contributions
        .iter()
        .any(|fp| fp == &content_fingerprint(text.as_bytes()))
}

/// Record a contribution's words as applied to the
/// document, at the moment they land (round 4, R4-4):
/// before any save is attempted, so no path that
/// follows (a failed save, a manual save, a recovery)
/// can offer to apply them again.
fn record_applied_fingerprint(doc: &mut Document, text: &str) {
    let fp = content_fingerprint(text.as_bytes());
    if !doc.applied_contributions.contains(&fp) {
        doc.applied_contributions.push(fp);
    }
}

/// One markdown file found in the inbox, read and parsed.
/// A file that does not parse stays on the list as
/// unreadable, with the reason: it is never hidden.
struct InboxItem {
    path: std::path::PathBuf,
    file_name: String,
    /// Seconds since the epoch, for newest-first order.
    modified_secs: u64,
    fingerprint: String,
    text: String,
    parsed: Result<ob_extract::contribution::Contribution, String>,
    /// Filed items only: the Sunday the filed name
    /// carries (round 3, R3-10), read back out of the
    /// name file_contribution wrote. For an undated part
    /// that is the Sunday it was filed for; the
    /// still-waiting count trusts it and nothing else.
    filed_for: Option<ob_model::SimpleDate>,
}

/// The Sunday a filed part's name carries: the date
/// segment between the sender and the fingerprint,
/// split from the right so a sender whose name holds
/// dashes survives. None when the name carries no date
/// (the "no Sunday named" names older filings used).
fn filed_sunday_from_name(file_name: &str) -> Option<ob_model::SimpleDate> {
    let stem = file_name
        .strip_suffix(".md")
        .or_else(|| file_name.strip_suffix(".MD"))?;
    let (rest, _fingerprint) = stem.rsplit_once(" - ")?;
    let (_sender, date) = rest.rsplit_once(" - ")?;
    ob_model::parse_flexible_date(
        date.trim(),
        ob_model::SimpleDate {
            year: 2000,
            month: 1,
            day: 1,
        },
    )
}

/// The largest contribution file the inbox will read
/// (round 3, RT3-E). A part is a page or two of
/// markdown, so 512 KB is already generous. Past it
/// the file is listed as unreadable with the reason
/// below and is never parsed: the scan used to read
/// every file whole and parse whatever came back, so
/// one 5 MB file cost the Start step seconds before
/// anything could be shown.
const MAX_PART_BYTES: u64 = 512 * 1024;

/// Read one inbox file through a cap, counting actual
/// bytes as they arrive, so no file, however the
/// filesystem describes it, can make the scan hold
/// more than the cap plus one byte. Read failures
/// keep the scan's long-standing words.
fn read_part_file(path: &std::path::Path) -> Result<Vec<u8>, String> {
    use std::io::Read as _;
    let mut file =
        std::fs::File::open(path).map_err(|e| format!("This file could not be read. ({e})"))?;
    let mut bytes = Vec::new();
    file.by_ref()
        .take(MAX_PART_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("This file could not be read. ({e})"))?;
    if bytes.len() as u64 > MAX_PART_BYTES {
        return Err("This file is too large to read as a part.".to_string());
    }
    Ok(bytes)
}

/// Read every `.md` file in a folder (the inbox, or the
/// filed folder when arrivals are counted) and parse each
/// as a contribution. Reading never moves a file. Filed
/// items also carry the Sunday their name was filed
/// under (round 3, R3-10).
fn scan_contribution_folder(
    dir: &std::path::Path,
    today: ob_model::SimpleDate,
    filed: bool,
) -> Vec<InboxItem> {
    let mut items: Vec<InboxItem> = Vec::new();
    let Ok(rd) = std::fs::read_dir(dir) else {
        return items;
    };
    for entry in rd.flatten() {
        let path = entry.path();
        let is_md = path
            .extension()
            .map(|e| e.eq_ignore_ascii_case("md"))
            .unwrap_or(false);
        if !is_md || !path.is_file() {
            continue;
        }
        let file_name = entry.file_name().to_string_lossy().into_owned();
        let modified_secs = entry
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let (fingerprint, text, parsed) = match read_part_file(&path) {
            Ok(bytes) => {
                let fp = content_fingerprint(&bytes);
                let text = String::from_utf8_lossy(&bytes).into_owned();
                let parsed = ob_extract::contribution::parse_contribution(&text, today);
                (fp, text, parsed)
            }
            Err(reason) => (String::new(), String::new(), Err(reason)),
        };
        let filed_for = if filed {
            filed_sunday_from_name(&file_name)
        } else {
            None
        };
        items.push(InboxItem {
            path,
            file_name,
            modified_secs,
            fingerprint,
            text,
            parsed,
            filed_for,
        });
    }
    items.sort_by(|a, b| a.file_name.cmp(&b.file_name));
    items
}

/// One sender-and-Sunday family of inbox items: versions
/// of (probably) the same part, newest on top.
struct InboxGroup {
    /// The sender as written, or "" when the words name
    /// nobody.
    from: String,
    for_date: Option<ob_model::SimpleDate>,
    /// Indexes into the scanned items, newest first.
    items: Vec<usize>,
}

/// Group parsed inbox items by (sender, Sunday). The
/// grouping is a plain name match, nothing smarter: the
/// sender words as written, compared without case.
fn group_inbox(items: &[InboxItem]) -> Vec<InboxGroup> {
    let mut groups: Vec<InboxGroup> = Vec::new();
    for (i, item) in items.iter().enumerate() {
        let Ok(c) = &item.parsed else { continue };
        let from = c.from.clone().unwrap_or_default();
        let key = from.trim().to_ascii_lowercase();
        let at = groups.iter().position(|g: &InboxGroup| {
            g.from.trim().to_ascii_lowercase() == key && g.for_date == c.for_date
        });
        match at {
            Some(at) => groups[at].items.push(i),
            None => groups.push(InboxGroup {
                from,
                for_date: c.for_date,
                items: vec![i],
            }),
        }
    }
    for g in &mut groups {
        g.items
            .sort_by_key(|&i| std::cmp::Reverse(items[i].modified_secs));
    }
    groups
}

/// The expected parts with no arrived contribution yet:
/// the Start step's "still waiting for" list. A
/// contribution counts as arrived when its From IS the
/// expected name, compared without case and ignoring
/// spaces at the ends (round 3, R3-10): "Lee" is not
/// satisfied by Leanne's part, however close the names
/// sit; the old substring match let exactly that
/// happen. Dating: a part that names a Sunday counts
/// only for that Sunday, in either folder. An undated
/// part still sitting in the inbox counts (it is
/// waiting to be reviewed, for the open Sunday by
/// default); an undated part that was FILED counts
/// only for the Sunday it was filed for, which the
/// filed scan read back out of its name.
fn still_waiting(
    expected: &[String],
    inbox: &[ob_extract::contribution::Contribution],
    filed: &[(
        &ob_extract::contribution::Contribution,
        Option<ob_model::SimpleDate>,
    )],
    bulletin: Option<ob_model::SimpleDate>,
) -> Vec<String> {
    let same_sender = |from: &str, want: &str| {
        !want.trim().is_empty() && from.trim().eq_ignore_ascii_case(want.trim())
    };
    expected
        .iter()
        .filter(|want| {
            let in_inbox = inbox.iter().any(|c| {
                same_sender(c.from.as_deref().unwrap_or(""), want)
                    && match (c.for_date, bulletin) {
                        (Some(d), Some(b)) => d == b,
                        _ => true,
                    }
            });
            let in_filed = filed.iter().any(|(c, filed_for)| {
                same_sender(c.from.as_deref().unwrap_or(""), want)
                    && match (c.for_date, bulletin) {
                        (Some(d), Some(b)) => d == b,
                        (Some(_), None) => true,
                        (None, Some(b)) => *filed_for == Some(b),
                        (None, None) => true,
                    }
            });
            !(in_inbox || in_filed)
        })
        .cloned()
        .collect()
}

/// The "Parts we usually get" field as a list: split on
/// commas and new lines, trimmed, empties dropped, and a
/// name kept once (compared without case).
fn parse_expected_parts(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for piece in text.split([',', '\n']) {
        let p = piece.trim();
        if !p.is_empty() && !out.iter().any(|e| e.eq_ignore_ascii_case(p)) {
            out.push(p.to_string());
        }
    }
    out
}

/// The reminder sentences for the missing parts, one per
/// part, ready to paste into the admin's own email or
/// message thread. The app sends nothing itself and never
/// claims to.
fn reminder_text(missing: &[String], bulletin: Option<ob_model::SimpleDate>) -> String {
    let date = bulletin
        .map(ob_extract::contribution::format_date_short)
        .unwrap_or_else(|| "this week".to_string());
    missing
        .iter()
        .map(|part| {
            format!(
                "A friendly reminder: the bulletin for Sunday, {date} is still \
                 waiting for {part}. Whenever it is ready, send it over and it \
                 will be added."
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
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

/// The hymn memory as it stands on disk. A file that
/// cannot be read starts the memory over: it is learned
/// data, rebuilt from the church's bulletins as they are
/// saved, never the only copy of anything.
fn load_hymn_memory() -> ob_model::hymns::HymnMemory {
    std::fs::read_to_string(hymn_memory_path())
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
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

impl App {
    /// Move to a step the way the Next button does: leaving
    /// Announcements writes the box; arriving at Check runs it.
    /// The step list itself is clickable now (finding F1).
    fn goto_step(&mut self, i: usize) {
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

    fn next_sunday(&mut self) {
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
    fn refresh_fonts(&mut self, ctx: &egui::Context) {
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
    fn canonical_snapshot(&self) -> String {
        snapshot(&self.canonical_document())
    }

    /// Stamp each style with the fingerprint of the library font it
    /// will be laid out with, so saved files remember their fonts.
    /// The document on disk now matches the screen: remember the
    /// state and retire the autosave journal.
    fn mark_clean(&mut self) {
        self.saved_snapshot = self.canonical_snapshot();
        self.journal_snapshot = self.saved_snapshot.clone();
        let (obd, meta) = journal_paths();
        clear_journal_from(&obd, &meta);
    }

    /// Remember the bulletin file this office just opened or
    /// saved, so the Start step can offer it back next time
    /// (UI audit round 2). Best effort: a settings file that
    /// will not write never blocks the work itself.
    fn remember_bulletin(&mut self, path: &std::path::Path) {
        let s = path.to_string_lossy().into_owned();
        if self.settings.last_bulletin.as_deref() != Some(s.as_str()) {
            self.settings.last_bulletin = Some(s);
            let _ = save_settings(&self.settings);
        }
    }

    /// Load the remembered state of the file now open (J6):
    /// its print and checked fingerprints become this
    /// session's, so freshness speaks across restarts.
    fn seed_doc_state(&mut self) {
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
    fn store_doc_state(&mut self) {
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
    fn remember_step(&mut self) {
        if self.settings.last_step != self.step {
            self.settings.last_step = self.step;
            let _ = save_settings(&self.settings);
        }
    }

    /// Once per launch: adopt the current document as the saved
    /// state, and pick up any journal a previous session left.
    fn startup(&mut self) {
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
                clear_journal_from(&obd, &meta);
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
    fn journal_tick(&mut self) {
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
                let (obd, meta) = journal_paths();
                clear_journal_from(&obd, &meta);
            }
            JournalAction::Write => {
                let (obd, meta) = journal_paths();
                // The journal is a file like any other: it
                // holds the bulletin whole, never the
                // inside-out shape of a sheet on the canvas
                // (R-3). The snapshot comparisons above are
                // in the canonical shape too (round 3,
                // R3-7), so the bytes written and the state
                // they are compared against agree.
                let doc = self.canonical_document();
                if write_journal_to(&obd, &meta, &doc, &self.path).is_ok() {
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
            let bundled = ob_text::shaping::bundled_font_infos();
            ob_fonts::pin_document_fonts(&mut self.doc, &lib, &bundled);
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
    fn print_checklist(&self) -> Vec<String> {
        print_checklist_for(&self.doc, &self.settings, &self.printer_name)
    }

    /// The print-window guidance for the printer currently
    /// named on the Print step (J5).
    fn current_print_guidance(&self) -> String {
        let setup = self.settings.printers.get(&self.printer_name);
        print_dialog_guidance(setup.is_some(), setup.map(|s| s.instruction.as_str()))
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

    /// Keep `edit_spoken` the same length as the edit
    /// buffer's paragraph list, with the same cursor-side
    /// growth and shrink as [`App::sync_edit_styles`]: a
    /// paragraph split in two keeps the flag on both halves
    /// (a split prayer list stays private), a deleted
    /// paragraph takes its flag with it.
    fn sync_edit_spoken(&mut self) {
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
    fn sync_edit_web_hidden(&mut self) {
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
    fn arm_thread_from(&mut self, pi: usize, fi: usize) {
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
    fn thread_pick_clicked(&mut self, pi: usize, hit: Option<usize>) {
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
    fn split_thread_at(&mut self, pi: usize, fi: usize) {
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

    fn add_picture_box(&mut self, idx: usize) {
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

    fn cancel_edit(&mut self) {
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
    fn reset_editor_transient(&mut self) {
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
    fn restore_discarded_edit(&mut self) {
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

    fn enter_edit(&mut self, pi: usize, fi: usize) {
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
    fn commit_edit(&mut self) {
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
    fn refresh_editor_layout(&mut self) {
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
                    shaper.layout_display_line_runs(
                        dtext,
                        &disp,
                        &spec,
                        st.font_size,
                        1.3,
                        frame.w,
                        st.align,
                    )
                } else {
                    shaper.layout_paragraph_runs(
                        dtext,
                        &eff_spans,
                        &spec,
                        st.font_size,
                        1.3,
                        frame.w,
                        st.align,
                    )
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
    fn editor_selection(&self) -> std::ops::Range<usize> {
        self.edit_anchor.min(self.edit_caret)..self.edit_anchor.max(self.edit_caret)
    }

    /// Apply one display-space edit to the session buffer
    /// through the byte map, park the caret after the
    /// inserted text, and resync the paragraph style list
    /// (the same bookkeeping commit_edit relies on).
    fn apply_display_edit(&mut self, range: std::ops::Range<usize>, replacement: &str) {
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
    fn editor_insert_text(&mut self, s: &str) {
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
    fn editor_paste(&mut self, t: &str) {
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
    fn editor_set_caret(&mut self, target: usize, extend: bool, forward: bool) {
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
    fn editor_hit(
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
    fn editor_click_caret(&mut self, d: usize, extend: bool) {
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
    fn editor_select_word(&mut self, d: usize) {
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
    fn editor_caret_local(&self) -> Option<(usize, usize)> {
        let layout = self.edit_layout.as_ref()?;
        let para = layout.map.para_at_display(self.edit_caret);
        let start = layout.map.para_display_range(para).start;
        Some((para, self.edit_caret - start))
    }

    /// Step one cluster stop left or right, crossing into
    /// the next paragraph at its edge. A plain arrow with a
    /// selection collapses it, as editors do.
    fn editor_step(&mut self, forward: bool, extend: bool) {
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
    fn editor_step_line(&mut self, down: bool, extend: bool) {
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
    fn editor_line_edge(&mut self, end: bool, extend: bool) {
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
    fn editor_step_word(&mut self, forward: bool, extend: bool) {
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
    fn editor_delete(&mut self, backward: bool) {
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
    fn editor_event(&mut self, ctx: &egui::Context, event: &egui::Event) {
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
    fn editor_styles_vec(&self) -> Vec<(bool, bool)> {
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
    fn editor_selection_raw_ranges(&self) -> Vec<std::ops::Range<usize>> {
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
    fn editor_toggle_flag(&mut self, flag: runsedit::Flag) {
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
    fn editor_flag_states(&self) -> (bool, bool) {
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
    fn editor_key(&mut self, key: egui::Key, modifiers: egui::Modifiers) {
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
    fn refresh_lines_cache(&mut self) {
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
    fn doc_threads(
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
    fn shaped_panel_lines(&mut self, idx: usize) -> Vec<PreviewLine> {
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
    fn picture_picker_window(&mut self, ctx: &egui::Context) {
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
    fn clips_window(&mut self, ctx: &egui::Context) {
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
                        let license = lib2
                            .as_ref()
                            .and_then(|l| l.collection(&collection))
                            .and_then(|c| c.notes)
                            .map(|n| n.license_kind())
                            .unwrap_or(ob_model::LicenseKind::Unknown);
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
                                license,
                                title,
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
    fn add_clips_from_folder(&mut self) {
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
    fn add_clips_from_zip(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Zip files", &["zip"])
            .pick_file()
        else {
            return;
        };
        let name = file_name_of(&path);
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
    fn edit_insert_on_pages(&mut self, id: &str) {
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
    fn leave_insert_editing(&mut self) {
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
    fn canonical_document(&self) -> Document {
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
    fn print_insert_sheet(&mut self, insert: &ob_model::Insert) {
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
            let (bytes, report) =
                ob_pdf::export_reader_pdf_report(&doc, ob_pdf::ExportPurpose::ForPrinting);
            match std::fs::write(&path, &bytes) {
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
    fn refresh_insert_library(&mut self) {
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
    fn save_insert_to_library(&mut self, id: &str) {
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
    fn add_insert_from_library(&mut self, path: &std::path::Path) {
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
    fn add_insert(&mut self) {
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
    fn remove_insert(&mut self, id: &str) {
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
    fn load_insert_buffers(&mut self, id: &str) {
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
    fn selected_insert_mut(&mut self) -> Option<&mut ob_model::Insert> {
        let id = self.insert_sel.clone()?;
        self.doc.inserts.iter_mut().find(|i| i.id == id)
    }

    fn apply_insert_name(&mut self) {
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

    fn apply_insert_date(&mut self) {
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

    fn apply_insert_copies(&mut self) {
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

    fn apply_insert_paper(&mut self) {
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

    fn set_insert_life(&mut self, id: &str, life: ob_model::InsertLife) {
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
    fn set_insert_preset(&mut self, id: &str, preset: &str) {
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
    fn inserts_window(&mut self, ctx: &egui::Context) {
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
                                fr.rect.x = ob_model::inches_to_mpt(l)
                                    .clamp(0, panel.size.w - fr.rect.w);
                                fr.rect.y = ob_model::inches_to_mpt(t)
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

    /// Load a panel's tracing picture as an egui texture, or
    /// None when it cannot be read. A v1 document carries
    /// the picture's bytes in its asset store and the panel
    /// names the asset's fingerprint; older documents name
    /// a file beside the .obd, read as before.
    fn load_bg_texture(&self, ctx: &egui::Context, rel: &str) -> Option<egui::TextureHandle> {
        if let Some(stored) = self.doc.asset_data(rel) {
            let img = image::load_from_memory(stored).ok()?.to_rgba8();
            let (w, h) = img.dimensions();
            let color =
                egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], img.as_raw());
            return Some(ctx.load_texture("trace-bg", color, egui::TextureOptions::LINEAR));
        }
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

    /// The decoded texture for one picture, at a size class
    /// (the longest pixel edge the caller needs), decoded
    /// once and cached: (texture, pixel width, pixel height)
    /// of the decoded picture. `key` names the CONTENT (an
    /// asset fingerprint, a library file path), never the
    /// document, so the cache survives document swaps. None
    /// when the bytes do not decode (missing formats and
    /// broken files alike; callers that need the reason ask
    /// ob_image themselves).
    fn picture_texture(
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
    fn set_frame_picture(
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
    fn set_picture_children(&mut self, idx: usize, fi: usize, present: bool) {
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
    fn pick_picture_file(&mut self, idx: usize, fi: usize) {
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
    fn frame_is_music(&self, idx: usize, fi: usize) -> bool {
        self.doc
            .panels
            .get(idx)
            .and_then(|p| p.frames.get(fi))
            .map(|f| f.music.is_some())
            .unwrap_or(false)
    }

    fn place_stored_picture(&mut self, idx: usize, fi: usize, sha: &str) {
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
    /// own collection, installed from the files and manifest
    /// shipped with the app.
    fn ensure_starter_clips(&mut self) {
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
        let files: Vec<(String, Vec<u8>)> = starter_clips::STARTER_FILES
            .iter()
            .map(|(name, text)| (name.to_string(), text.as_bytes().to_vec()))
            .collect();
        let notes = ob_clips::CollectionNotes {
            source: "OpenBulletin starter set (bundled with the program)".into(),
            license: "CC0".into(),
            added: String::new(),
        };
        let _ = lib.add_collection("Starter set", &files, &notes, None);
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
    fn start_idml_import(&mut self, path: std::path::PathBuf) {
        let profile = self.doc.profile.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        self.import_rx = Some(rx);
        self.note("Reading your InDesign file...".into());
        std::thread::spawn(move || {
            let result = (|| -> Result<ImportReview, String> {
                let bytes =
                    std::fs::read(&path).map_err(|e| format!("cannot read that file: {e}"))?;
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
    fn start_sla_import(&mut self, path: std::path::PathBuf) {
        let profile = self.doc.profile.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        self.import_rx = Some(rx);
        self.note("Reading your Scribus file...".into());
        std::thread::spawn(move || {
            let result = (|| -> Result<ImportReview, String> {
                let bytes =
                    std::fs::read(&path).map_err(|e| format!("cannot read that file: {e}"))?;
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
    fn start_pub_import(&mut self, path: std::path::PathBuf) {
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
    fn open_last_week(&mut self, path: std::path::PathBuf) {
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

    fn open_old_bulletin(&mut self, path: std::path::PathBuf) {
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
    fn route_old_bulletin(&mut self, path: std::path::PathBuf) {
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
                let head = std::fs::read(&path)
                    .unwrap_or_default()
                    .into_iter()
                    .take(4096)
                    .collect::<Vec<u8>>();
                if head.starts_with(b"%PDF") {
                    self.start_import(path);
                } else if String::from_utf8_lossy(&head).contains("<score-partwise") {
                    // A music file whose name did not say so
                    // (an extension-less or .xml MusicXML).
                    self.start_music_import(path);
                } else if head.starts_with(b"PK") {
                    // A zip: an IDML package, or a .mxl music
                    // file. The music sniff reads the whole
                    // file; IDML remains the default.
                    let bytes = std::fs::read(&path).unwrap_or_default();
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
    fn start_md_import(&mut self, path: std::path::PathBuf) {
        let name = file_name_of(&path);
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) => {
                self.fail(format!("Could not read that file. ({e})"));
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
    fn start_html_import(&mut self, path: std::path::PathBuf) {
        let name = file_name_of(&path);
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(e) => {
                self.fail(format!("Could not read that file. ({e})"));
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
    fn start_music_import(&mut self, path: std::path::PathBuf) {
        let profile = self.doc.profile.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        self.import_rx = Some(rx);
        self.note("Reading your music file...".into());
        std::thread::spawn(move || {
            let result = (|| -> Result<ImportReview, String> {
                let bytes =
                    std::fs::read(&path).map_err(|e| format!("cannot read that file: {e}"))?;
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
    fn finalize_import(&mut self, edit: bool) {
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
    fn refresh_inbox(&mut self) {
        let today = today_simple();
        self.inbox_items = scan_contribution_folder(&inbox_dir(), today, false);
        self.inbox_filed = scan_contribution_folder(&filed_dir(), today, true);
        self.inbox_dirty = false;
    }

    /// Open words someone sent into the contribution
    /// review: the parsed words, where they came from (an
    /// inbox file, or None for pasted words), and the text
    /// as it stands.
    fn open_contribution_review(
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
    fn begin_contribution_edit(&mut self) {
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
    fn apply_contribution_edit(&mut self) {
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
    fn accept_contribution(&mut self) {
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
    fn file_applied_contribution(&mut self) {
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
    fn file_contribution(&mut self, review: &ContributionReview) -> String {
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
    fn contribution_review_window(&mut self, ctx: &egui::Context) {
        if self.contribution_review.is_none() {
            return;
        }
        use ob_extract::contribution::{format_date_long, week_verdict, WeekVerdict};
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
                            ui.label(
                                egui::RichText::new(format!(
                                    "These words say they are for {}. The bulletin open is for {}.",
                                    format_date_long(said),
                                    format_date_long(open)
                                ))
                                .color(self.theme.warn)
                                .strong(),
                            );
                        }
                        WeekVerdict::SameSunday => {
                            let d = review.contribution.for_date.expect("verdict");
                            ui.label(format!(
                                "These words say they are for {}.",
                                format_date_long(d)
                            ));
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
    fn paste_window(&mut self, ctx: &egui::Context) {
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
    fn print_sender_card(&mut self) {
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
            match std::fs::write(&path, &bytes) {
                Ok(()) => self.note(format!(
                    "Sender's card written: {}. Print it and hand it to anyone who sends words.",
                    file_name_of(&path)
                )),
                Err(e) => self.fail(format!("Could not write the sender's card: {e}")),
            }
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
    fn start_analyzer(&mut self, files: Vec<std::path::PathBuf>) {
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
                                let bytes = std::fs::read(path)
                                    .map_err(|e| format!("cannot read that file: {e}"))?;
                                let size = bytes.len();
                                let cfb = ob_extract::cfb::Cfb::open(bytes)?;
                                let x = ob_extract::pub_layout::extract_pub_positioned(
                                    &cfb, &name, size,
                                )?;
                                Ok((x.dump, None))
                            })(),
                        },
                        Some(LearnKind::Indesign) => std::fs::read(path)
                            .map_err(|e| format!("cannot read that file: {e}"))
                            .and_then(|bytes| ob_extract::idml::dump_from_idml(&bytes, &name))
                            .map(|d| (d, None)),
                        Some(LearnKind::Scribus) => std::fs::read(path)
                            .map_err(|e| format!("cannot read that file: {e}"))
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

    fn poll_analyzer(&mut self, ctx: &egui::Context) {
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

    fn finalize_analyzer(&mut self, edit: bool) {
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

    fn analyzer_review_window(&mut self, ctx: &egui::Context) {
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
    fn open_pages_at(&mut self, panel: usize, frame: Option<usize>) {
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
    fn page_by(&mut self, delta: i32) {
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
    fn open_flow(&mut self) {
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
    fn save_large_print_copy(&mut self, path: &std::path::Path) {
        let (bytes, report) =
            ob_pdf::export_large_print_pdf_report(&self.doc, ob_pdf::ExportPurpose::ForPrinting);
        match std::fs::write(path, &bytes) {
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

    fn save_copy_for_program(&mut self, path: &std::path::Path) {
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        match ext.as_str() {
            "idml" => match ob_extract::idml_export::export_idml(&self.doc) {
                Ok((bytes, report)) => match std::fs::write(path, &bytes) {
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
                Ok((bytes, report)) => match std::fs::write(path, &bytes) {
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
                match std::fs::write(path, &bytes) {
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
                match std::fs::write(path, text) {
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
                        match std::fs::write(&target, &bytes) {
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
    fn absorb_tracing_pictures(&mut self, target_dir: Option<&std::path::Path>) {
        let mut jobs: Vec<(usize, String, Vec<u8>, String)> = Vec::new();
        for (pi, panel) in self.doc.panels.iter().enumerate() {
            let Some(r) = panel.background_image.clone() else {
                continue;
            };
            if self.doc.asset_data(&r).is_some() {
                continue; // already carried, by fingerprint
            }
            let mut candidates: Vec<std::path::PathBuf> = Vec::new();
            if let Some(dir) = self.path.as_ref().and_then(|p| p.parent()) {
                candidates.push(dir.join(&r));
            }
            if let Some(dir) = target_dir {
                candidates.push(dir.join(&r));
            }
            if let Some(base) = &self.asset_base {
                candidates.push(base.join(&r));
            }
            for c in candidates {
                if let Ok(bytes) = std::fs::read(&c) {
                    let media = match std::path::Path::new(&r)
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("")
                        .to_ascii_lowercase()
                        .as_str()
                    {
                        "jpg" | "jpeg" => "image/jpeg",
                        "gif" => "image/gif",
                        "webp" => "image/webp",
                        _ => "image/png",
                    }
                    .to_string();
                    jobs.push((pi, r.clone(), bytes, media));
                    break;
                }
            }
        }
        for (pi, old, bytes, media) in jobs {
            let hash = self.doc.add_asset(
                &bytes,
                ob_model::AssetKind::TracingPicture,
                &media,
                ob_model::RightsRecord {
                    source: old.clone(),
                    ..Default::default()
                },
            );
            if let Some(panel) = self.doc.panels.get_mut(pi) {
                if panel.background_image.as_deref() == Some(old.as_str()) {
                    panel.background_image = Some(hash);
                }
            }
        }
    }

    /// The Save button's work, shared with its key.
    /// Answers whether the bulletin was actually saved:
    /// the contribution seam files a part only after the
    /// bulletin carrying it is safely on disk.
    fn save_flow(&mut self) -> bool {
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
    fn learn_hymns_from_current(&mut self) {
        if learn_hymns(&mut self.hymn_memory, &self.doc) {
            self.persist_hymn_memory();
        }
    }

    /// Write the hymn memory beside the settings. Best
    /// effort, like the settings themselves: a memory that
    /// will not write never blocks the work it came from.
    /// The write is atomic, the settings pattern (round
    /// 3, R3-12): a crash mid-write leaves the old memory
    /// whole rather than a truncated file that parses as
    /// nothing and silently starts the memory over.
    fn persist_hymn_memory(&self) {
        if let Ok(text) = serde_json::to_string_pretty(&self.hymn_memory) {
            let path = hymn_memory_path();
            let tmp = path.with_extension("json.tmp");
            if std::fs::write(&tmp, text).is_ok() {
                let _ = std::fs::rename(&tmp, &path);
            }
        }
    }

    /// Remember the on-disk state of the file now open, as
    /// this app last saw it (opened it, or saved it): the
    /// save guard compares against this stamp. With no file
    /// open, there is nothing to remember.
    fn note_disk_stamp(&mut self) {
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
    fn save_as_new_file(&mut self) {
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
    fn binding_for(&self, action: &str) -> Option<KeyBinding> {
        effective_keymap(&self.settings)
            .get(action)
            .and_then(|t| parse_binding(t))
    }

    /// Was this action's key just pressed?
    fn action_pressed(&self, ctx: &egui::Context, action: &str) -> bool {
        match self.binding_for(action) {
            Some(b) => ctx.input(|i| binding_pressed(i, &b)),
            None => false,
        }
    }

    /// While a key is being captured for the Keys window, the
    /// next key press becomes that action's key. If another
    /// action held that key, they swap: no key ever does two
    /// jobs, and none is silently lost.
    fn capture_key(&mut self, ctx: &egui::Context) {
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

    fn keys_window(&mut self, ctx: &egui::Context) {
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
    fn apply_comfort(&mut self, ctx: &egui::Context) {
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
    fn comfort_window(&mut self, ctx: &egui::Context) {
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
                            let (bytes, report) = ob_pdf::export_imposed_pdf_report(
                                &self.doc,
                                ob_pdf::ExportPurpose::ForPrinting,
                            );
                            let path = std::env::temp_dir().join(format!(
                                "openbulletin-print-copy-{}.pdf",
                                std::process::id()
                            ));
                            match std::fs::write(&path, &bytes).map_err(|e| e.to_string()).and_then(|_| open_with_os(&path)) {
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
                                let (bytes, report) = ob_pdf::export_imposed_pdf_report(
                                    &self.doc,
                                    ob_pdf::ExportPurpose::ForPrinting,
                                );
                                match std::fs::write(&path, &bytes) {
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
                        if ui.button("Open a copy to read through").clicked() {
                            self.commit_if_editing();
                            self.save_announcements();
                            if let Some(path) = rfd::FileDialog::new()
                                .set_file_name("bulletin-reading.pdf")
                                .add_filter("PDF", &["pdf"])
                                .save_file()
                            {
                                self.pin_fonts();
                                let (bytes, report) = ob_pdf::export_reader_pdf_report(
                                    &self.doc,
                                    ob_pdf::ExportPurpose::ForEmailOrWeb,
                                );
                                match std::fs::write(&path, &bytes) {
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
                                let (bytes, _report) = ob_pdf::export_reader_pdf_report(
                                    &proof,
                                    ob_pdf::ExportPurpose::ForPrinting,
                                );
                                self.status = match std::fs::write(&path, &bytes) {
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

/// The window icon: the same folded-bulletin glyph the bundle
/// ships as icon.png, decoded with the image crate the app
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
    let bytes = std::fs::read(path).map_err(|e| format!("cannot read that file: {e}"))?;
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
