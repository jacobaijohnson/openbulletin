//! Settings, keymap, printer setup, and the crash
//! journal: what the office chose, what survives a
//! restart, and the autosave journal's worker.

use crate::*;

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
    pub(crate) fn lin(v: u8) -> f32 {
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

pub(crate) const GUIDANCE: [&str; 5] = [
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

/// The app's one durable-write shape (HF-5, S-MIN-1): write
/// the bytes to a temp file in the target's own folder (so
/// the rename stays on one volume), sync the temp file,
/// rename it over the target, then sync the containing
/// directory where the platform allows it. The temp-file
/// sync is load-bearing and its failure is the caller's
/// error to report; the directory sync is best-effort —
/// where a directory cannot be opened and synced that way,
/// there is nothing to do and the write must not fail over
/// it. A crash mid-write leaves the previous file whole.
///
/// The body is ob-model's one durable write,
/// `ob_model::sync_and_rename` (round 31, F9) — the same
/// body the .obd/.obdi saves land through, directory sync
/// included. This adapter keeps only the app's own parts
/// of the shape: its temp naming (the target's own name
/// plus `.tmp`) and its `std::io::Error` surface.
pub(crate) fn write_file_atomically(
    path: &std::path::Path,
    bytes: &[u8],
) -> Result<(), std::io::Error> {
    let mut tmp_name = path.as_os_str().to_owned();
    tmp_name.push(".tmp");
    let tmp = std::path::PathBuf::from(tmp_name);
    ob_model::sync_and_rename(&tmp, path, bytes)
}

/// Where the autosave journal lives: beside settings.json.
/// Two files: the bulletin itself and a note (when it was
/// written, which file it belongs to if any, and the token
/// of the instance that wrote it).
pub fn journal_paths() -> (std::path::PathBuf, std::path::PathBuf) {
    let dir = settings_path()
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    (dir.join("autosave.obd"), dir.join("autosave.txt"))
}

pub(crate) fn write_journal_to(
    obd: &std::path::Path,
    meta: &std::path::Path,
    doc: &Document,
    source: &Option<std::path::PathBuf>,
    token: &str,
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
    // The note lands with the same atomic, durable discipline
    // as the bulletin (S-MIN-5): a torn or missing note used to
    // demote an intact journal bulletin to unreadable at the
    // next startup. Third line: the token of the instance that
    // wrote this journal (S-MIN-3), so retirement can tell its
    // own journal from another live instance's. Older notes
    // have no third line and read as tokenless.
    let note_text = format!("{when}\n{source_line}\n{token}\n");
    write_file_atomically(meta, note_text.as_bytes())
        .map_err(|e| format!("cannot write the autosave note: {e}"))
}

/// The instance token a journal's note names as its owner,
/// if the note carries one (notes written before the token
/// existed have only two lines).
pub(crate) fn journal_token_from(meta: &std::path::Path) -> Option<String> {
    let note = std::fs::read_to_string(meta).ok()?;
    let token = note.lines().nth(2)?.trim();
    if token.is_empty() {
        None
    } else {
        Some(token.to_string())
    }
}

pub(crate) fn read_journal_from(
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

pub(crate) fn clear_journal_from(obd: &std::path::Path, meta: &std::path::Path) {
    let _ = std::fs::remove_file(obd);
    let _ = std::fs::remove_file(meta);
}

/// One job for the journal worker (HF-4). A job carries
/// everything the work needs — the journal paths, the
/// document clone and the instance token for a write; the
/// journal paths, the lock path and the token for a
/// retire — so the worker thread never touches App state.
pub(crate) enum JournalJob {
    // Boxed: a write job carries a whole document clone,
    // and the retire job must not grow to match it.
    Write(Box<JournalWriteJob>),
    Retire(JournalRetireJob),
}

pub(crate) struct JournalWriteJob {
    pub(crate) obd: std::path::PathBuf,
    pub(crate) meta: std::path::PathBuf,
    pub(crate) doc: Document,
    pub(crate) source: Option<std::path::PathBuf>,
    pub(crate) token: String,
    /// The canonical snapshot this write carries: when
    /// the write lands, the journal holds exactly this
    /// state, and `poll_journal` records it.
    pub(crate) snapshot: String,
}

pub(crate) struct JournalRetireJob {
    pub(crate) obd: std::path::PathBuf,
    pub(crate) meta: std::path::PathBuf,
    pub(crate) lock: std::path::PathBuf,
    pub(crate) token: String,
}

/// What the journal worker reports back, one per job, in
/// the order the jobs ran.
pub(crate) enum JournalResult {
    Written {
        snapshot: String,
        outcome: Result<(), String>,
    },
    Retired,
}

/// The journal worker: one dedicated thread doing the
/// journal's file work (the full `ob_format::save` zip and
/// the guarded retirement) off the egui frame thread, on
/// the same job/result channel shape as the import and
/// catalog workers. Jobs run one at a time, in the order
/// they were sent, so a retire sent after a write lands
/// after it, and no write sent before a retire can land
/// behind it.
pub(crate) struct JournalWorker {
    pub(crate) jobs: std::sync::mpsc::Sender<JournalJob>,
    pub(crate) results: std::sync::mpsc::Receiver<JournalResult>,
}

impl JournalWorker {
    pub(crate) fn spawn() -> JournalWorker {
        let (job_tx, job_rx) = std::sync::mpsc::channel::<JournalJob>();
        let (result_tx, result_rx) = std::sync::mpsc::channel::<JournalResult>();
        std::thread::spawn(move || {
            while let Ok(job) = job_rx.recv() {
                let result = match job {
                    JournalJob::Write(job) => {
                        let outcome = write_journal_to(
                            &job.obd,
                            &job.meta,
                            &job.doc,
                            &job.source,
                            &job.token,
                        );
                        JournalResult::Written {
                            snapshot: job.snapshot,
                            outcome,
                        }
                    }
                    JournalJob::Retire(job) => {
                        retire_journal_from(&job.obd, &job.meta, &job.lock, &job.token);
                        JournalResult::Retired
                    }
                };
                if result_tx.send(result).is_err() {
                    break;
                }
            }
        });
        JournalWorker {
            jobs: job_tx,
            results: result_rx,
        }
    }
}

/// Which kind of journal job is in flight, if any: the
/// single-flight gate that keeps journal file mutations
/// from ever racing each other.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum JournalFlight {
    Write,
    Retire,
}

/// A journal that exists but cannot be read (audit Batch 3):
/// move what is there out of the journal's place, under a
/// name that says what happened, so the next autosave starts
/// clean and the damaged copy is still on disk for anyone who
/// goes looking. Returns true when anything was set aside.
/// Renames replace an older set-aside: one damaged copy kept
/// is enough.
pub(crate) fn set_aside_unreadable_journal(obd: &std::path::Path, meta: &std::path::Path) -> bool {
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
pub(crate) fn settings_section<T: serde::de::DeserializeOwned>(
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
pub(crate) fn set_aside_unreadable_settings(path: &std::path::Path) -> bool {
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
    // Atomic and durable, the document format's discipline
    // (round 2, R2-4; HF-5): a synced sibling temp file
    // renamed over, so a crash or power cut mid-write cannot
    // truncate the one file that remembers the office's
    // printers and keys.
    write_file_atomically(&path, text.as_bytes()).map_err(|e| e.to_string())
}
