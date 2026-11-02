//! Hymns as parts, not as one typed line (roles build
//! R-2a). A hymn line in the order of service names three
//! things: which book, which number, which song. When the
//! three ride in one string, the administrator retypes all
//! of them every week and every error (a wrong number, a
//! dropped title) is born in that retyping. Here the parts
//! are parsed out of the words as written, composed back
//! into one canonical line, and remembered as number and
//! title pairs the church itself confirmed, so Check can
//! hold this week's words up against the church's own
//! memory. There is no bundled hymn database: the memory
//! starts empty and learns only from this church's
//! bulletins.

use crate::{
    compose_permission_line, service_story_id, ContentRole, Document, Frame, FrameKind,
    LicenseKind, Panel, Paragraph, Rect, SimpleDate, Size,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// One hymn, in parts. Any part may be missing: a number
/// alone is how most bulletins print, a title alone is how
/// most people speak.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HymnEntry {
    pub hymnal: Option<String>,
    pub number: Option<String>,
    pub title: Option<String>,
}

/// Is this order-of-service label a hymn line's label? The
/// one test every hymn feature shares: the word "hymn"
/// anywhere in the label, in any case ("Opening Hymn",
/// "Hymn of the Day", "Closing hymn").
pub fn is_hymn_label(label: &str) -> bool {
    label.to_lowercase().contains("hymn")
}

/// The first word of `s`, and what follows it.
fn first_word(s: &str) -> (&str, &str) {
    let s = s.trim_start();
    match s.find(char::is_whitespace) {
        Some(i) => (&s[..i], s[i..].trim_start()),
        None => (s, ""),
    }
}

/// Is this word a hymn number as written? Digits, with an
/// optional leading "#" and a trailing comma or period the
/// typing carried ("432", "#432", "432,").
fn number_word(word: &str) -> Option<String> {
    let w = word
        .trim()
        .trim_start_matches('#')
        .trim_end_matches([',', '.']);
    if !w.is_empty() && w.chars().all(|c| c.is_ascii_digit()) {
        Some(w.to_string())
    } else {
        None
    }
}

/// Take a title as written and return the title itself:
/// the separators a typist puts between a number and its
/// title (a comma, a dash, a dot leader dot) come off, and
/// one pair of surrounding quotes comes off. Words inside
/// quotes are kept exactly as written.
fn clean_title(s: &str) -> Option<String> {
    let t = s.trim().trim_start_matches([
        ' ', ',', '-', '\u{2013}', '\u{2014}', ':', ';', '\u{00b7}', '.',
    ]);
    let t = t.trim();
    if t.is_empty() {
        return None;
    }
    let pairs = [
        ('\'', '\''),
        ('"', '"'),
        ('\u{2018}', '\u{2019}'),
        ('\u{201c}', '\u{201d}'),
        ('\u{2019}', '\u{2019}'),
    ];
    let mut core = t;
    for (open, close) in pairs {
        if t.starts_with(open) {
            if let Some(end) = t[open.len_utf8()..].find(close) {
                core = &t[open.len_utf8()..open.len_utf8() + end];
            } else if t.len() > open.len_utf8() && t.ends_with(close) {
                core = &t[open.len_utf8()..t.len() - close.len_utf8()];
            }
            break;
        }
    }
    let core = core.trim().trim_end_matches([',', ';']).trim();
    if core.is_empty() {
        None
    } else {
        Some(core.to_string())
    }
}

/// Read a hymn line's value as its parts. The shapes a
/// church actually writes: "ELW 432", "LBW 165", "432",
/// "Hymn 432", each with a title after the number when one
/// is written (in quotes, after a dash or a comma, or as
/// bare words), and a title standing alone ("Amazing
/// Grace" is a title with no number, never a hymnal).
pub fn parse_hymn_value(text: &str) -> HymnEntry {
    let t = text.trim();
    let mut entry = HymnEntry::default();
    if t.is_empty() {
        return entry;
    }
    let mut rest = t;
    let (w1, r1) = first_word(rest);
    // Words that introduce a number without naming a book.
    let noise = ["hymn", "hymnal", "no", "number"];
    if noise
        .iter()
        .any(|n| w1.trim_end_matches('.').eq_ignore_ascii_case(n))
    {
        rest = r1;
    } else if w1.chars().all(|c| c.is_alphabetic())
        && w1.chars().count() >= 2
        && w1.chars().count() <= 5
    {
        // A short letters-only word followed by a number is
        // a hymnal's name (ELW, LBW, LSB); a word followed
        // by more words is the start of a title.
        let (w2, _) = first_word(r1);
        if number_word(w2).is_some() {
            entry.hymnal = Some(w1.trim_end_matches([':', ',']).to_string());
            rest = r1;
        }
    }
    let (wn, rn) = first_word(rest);
    if let Some(n) = number_word(wn) {
        entry.number = Some(n);
        rest = rn;
    }
    entry.title = clean_title(rest);
    entry
}

/// The one shape a hymn value takes in the bulletin:
/// "<Hymnal> <number> · <title>", each missing part
/// simply left out. A title alone prints as the title.
pub fn compose_hymn_value(entry: &HymnEntry) -> String {
    let clean = |o: &Option<String>| {
        o.as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    };
    let hymnal = clean(&entry.hymnal);
    let number = clean(&entry.number);
    let title = clean(&entry.title);
    let head = [hymnal, number]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" ");
    match (head.is_empty(), title) {
        (true, None) => String::new(),
        (true, Some(title)) => title,
        (false, None) => head,
        (false, Some(title)) => format!("{head} \u{00b7} {title}"),
    }
}

/// The memory's key for one pair: the hymnal's name
/// without regard to case (or no hymnal at all), and the
/// number as written.
pub fn hymn_key(hymnal: Option<&str>, number: &str) -> String {
    format!(
        "{}|{}",
        hymnal.unwrap_or("").trim().to_lowercase(),
        number.trim()
    )
}

/// One remembered pair: the title the church printed for
/// this number, and the Sunday it was last confirmed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HymnPair {
    pub title: String,
    pub last_seen: SimpleDate,
    /// The license the office recorded for this hymn, when
    /// one is recorded (round 51, G-08d). Absent in older
    /// memory files (read as unlicensed); learning a new
    /// title never clears a recorded license.
    #[serde(default)]
    pub license: HymnLicense,
}

/// A hymn's license record, as the office recorded it
/// (round 51, G-08d): the licensor and number the
/// permission line prints, and the copyright line it
/// prints beside them. The record is the office's own
/// note, not a proof — the same standing as every other
/// rights record in the app.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HymnLicense {
    /// The license kind; only `LicensedReprint` yields a
    /// permission line. Defaults to Unknown.
    #[serde(default)]
    pub kind: LicenseKind,
    /// "CCLI" or "ONE LICENSE" (read case-insensitively);
    /// empty when the office has not named a licensor.
    #[serde(default)]
    pub licensor: String,
    /// The church's license number ("123456", "A-123456");
    /// empty when unrecorded, which is the warn case,
    /// never a block.
    #[serde(default)]
    pub number: String,
    /// The copyright line as it should print
    /// ("© 2019 Jane Doe"); empty when unrecorded.
    #[serde(default)]
    pub copyright_line: String,
}

/// The church's own hymn memory: number and title pairs
/// learned from the church's confirmed bulletins, nothing
/// else. The app keeps it in a small file beside its
/// settings and feeds it every saved bulletin. A learned
/// template teaches nothing: it carries no occasion date
/// of its own (it spans many Sundays), so its pairs
/// arrive when the template is saved as a real week
/// (round 3, R3-12).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HymnMemory {
    pub pairs: BTreeMap<String, HymnPair>,
}

impl HymnMemory {
    /// The pair remembered for this hymnal and number, if
    /// the church has ever confirmed one.
    pub fn lookup(&self, hymnal: Option<&str>, number: &str) -> Option<&HymnPair> {
        self.pairs.get(&hymn_key(hymnal, number))
    }

    /// Forget one remembered pair outright (round 3,
    /// R3-6): Check's disagreement finding offers this
    /// when the office says the memory is the wrong one.
    /// The pair is not barred from returning: the next
    /// saved bulletin carrying it teaches it again, with
    /// that bulletin's date. Answers true when a pair
    /// was actually there to forget.
    pub fn forget(&mut self, hymnal: Option<&str>, number: &str) -> bool {
        self.forget_key(&hymn_key(hymnal, number))
    }

    /// Forget by the memory's own key (the shape the
    /// finding carries): see `forget`.
    pub fn forget_key(&mut self, key: &str) -> bool {
        self.pairs.remove(key).is_some()
    }

    /// Learn one entry, when it carries both a number and
    /// a title. Confirmation order is save order, and the
    /// last confirmation for a Sunday wins it: a newer
    /// Sunday wins a disagreement, and on the SAME
    /// Sunday a different title is a correction, because
    /// re-saving the same bulletin with the title fixed
    /// is how a typo gets repaired (round 3, R3-6). An
    /// older Sunday never overwrites. Answers true when
    /// the memory changed, so the caller knows to save
    /// it; confirming the same pair for the same Sunday
    /// again changes nothing.
    pub fn learn(&mut self, entry: &HymnEntry, date: SimpleDate) -> bool {
        let (Some(number), Some(title)) = (entry.number.as_deref(), entry.title.as_deref()) else {
            return false;
        };
        let number = number.trim();
        let title = title.trim();
        if number.is_empty() || title.is_empty() {
            return false;
        }
        let key = hymn_key(entry.hymnal.as_deref(), number);
        match self.pairs.get(&key) {
            Some(old) if old.last_seen > date => false,
            Some(old) if old.last_seen == date && old.title == title => false,
            _ => {
                // A re-learned title never clears a
                // recorded license: the office's license
                // note survives the week's retyping
                // (round 51, G-08d).
                let carried = match self.pairs.get(&key) {
                    Some(old) => old.license.clone(),
                    None => HymnLicense::default(),
                };
                self.pairs.insert(
                    key,
                    HymnPair {
                        title: title.to_string(),
                        last_seen: date,
                        license: carried,
                    },
                );
                true
            }
        }
    }

    /// Learn from one whole document: every hymn line
    /// (a tabbed line whose label names a hymn, the order
    /// of service's own shape) that carries both a number
    /// and a title. Answers true when the memory changed.
    pub fn learn_from_document(&mut self, doc: &Document, date: SimpleDate) -> bool {
        let mut changed = false;
        for story in &doc.stories {
            for para in &story.paragraphs {
                let text = para.text();
                if !text.contains('\t') {
                    continue;
                }
                let label = text.split('\t').next().unwrap_or("").trim();
                if !is_hymn_label(label) {
                    continue;
                }
                let value = text.rsplit('\t').next().unwrap_or("").trim();
                let entry = parse_hymn_value(value);
                changed |= self.learn(&entry, date);
            }
        }
        changed
    }

    /// Record a license for a hymn the church has
    /// confirmed (round 51, G-08d): the pair must already
    /// exist — a license note never invents a hymn the
    /// church has not printed. Answers false when no pair
    /// stands for this hymnal and number, and records
    /// nothing in that case.
    pub fn note_license(
        &mut self,
        hymnal: Option<&str>,
        number: &str,
        license: HymnLicense,
    ) -> bool {
        let key = hymn_key(hymnal, number);
        match self.pairs.get_mut(&key) {
            Some(pair) => {
                pair.license = license;
                true
            }
            None => false,
        }
    }
}

/// One licensed hymn use in a document, for the
/// permission-line path and the usage report alike
/// (round 51, G-08d).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HymnLicenseUse {
    /// The service-line label ("Hymn of Praise").
    pub label: String,
    /// The hymn as the bulletin shows it ("ELW 432").
    pub shown: String,
    /// The title as printed this week, else as remembered.
    pub title: String,
    /// The recorded license, as the office noted it.
    pub license: HymnLicense,
    /// The permission line to print, when a license
    /// number is recorded; None is the warn case.
    pub line: Option<String>,
    /// Licensed, but no license number recorded: warn,
    /// never block, and print no numberless line.
    pub missing_number: bool,
}

/// The permission line for one hymn (round 51, G-08d):
/// the week's printed title when the line carries one,
/// else the remembered title; the office's recorded
/// copyright line and license number. None when the
/// pair is not a licensed reprint, or when no number
/// is recorded.
pub fn permission_line_for_hymn(entry: &HymnEntry, pair: &HymnPair) -> Option<String> {
    if pair.license.kind != LicenseKind::LicensedReprint {
        return None;
    }
    let title = entry
        .title
        .clone()
        .filter(|t| !t.trim().is_empty())
        .unwrap_or_else(|| pair.title.clone());
    compose_permission_line(
        &title,
        "",
        &pair.license.copyright_line,
        &pair.license.licensor,
        &pair.license.number,
    )
}

/// Every licensed hymn use in a document's order of
/// service (round 51, G-08d): the same service story
/// Check reads, each hymn line held up against the
/// church's hymn memory. A song sung twice in one
/// service is one use: the permission line prints once,
/// and the usage report lists it once. Hymns with no
/// remembered pair, and pairs that are not licensed
/// reprints, are not uses and stay quiet.
pub fn hymn_license_uses(doc: &Document, memory: &HymnMemory) -> Vec<HymnLicenseUse> {
    let Some(sid) = service_story_id(doc) else {
        return Vec::new();
    };
    let Some(story) = doc.story(&sid) else {
        return Vec::new();
    };
    let mut out: Vec<HymnLicenseUse> = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    for para in &story.paragraphs {
        let text = para.text();
        if !text.contains('\t') {
            continue;
        }
        let label = text.split('\t').next().unwrap_or("").trim().to_string();
        if !is_hymn_label(&label) {
            continue;
        }
        let value = text.rsplit('\t').next().unwrap_or("").trim();
        let entry = parse_hymn_value(value);
        let Some(number) = entry.number.clone() else {
            continue;
        };
        let key = hymn_key(entry.hymnal.as_deref(), &number);
        if seen.contains(&key) {
            continue;
        }
        seen.push(key);
        let Some(pair) = memory.lookup(entry.hymnal.as_deref(), &number) else {
            continue;
        };
        if pair.license.kind != LicenseKind::LicensedReprint {
            continue;
        }
        let line = permission_line_for_hymn(&entry, pair);
        let missing_number = line.is_none();
        let shown = compose_hymn_value(&entry);
        let title = entry
            .title
            .clone()
            .filter(|t| !t.trim().is_empty())
            .unwrap_or_else(|| pair.title.clone());
        out.push(HymnLicenseUse {
            label,
            shown,
            title,
            license: pair.license.clone(),
            line,
            missing_number,
        });
    }
    out
}

/// What appending the hymn permission lines did
/// (round 51, G-08d).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HymnPermissionOutcome {
    /// The lines now on the Permissions panel (empty
    /// when no licensed hymn carried a number).
    pub lines: Vec<String>,
    /// Licensed hymns with no license number recorded:
    /// warned, never blocked, and given no line.
    pub warnings: Vec<String>,
    /// True when this call appended the panel.
    pub added: bool,
}

/// The story id the hymn permission panel carries, so a
/// second call finds it and appends nothing twice.
pub const HYMN_PERMISSIONS_STORY: &str = "hymn-permissions";

/// The permission lines currently printed on a
/// document's Permissions panel, if it has one: the
/// story's paragraphs after its "License permissions"
/// heading. None when no panel story stands. The panel
/// is a snapshot of the hymns at append time, so these
/// lines are what the page prints — never a substitute
/// for the freshly computed lines.
pub fn permission_panel_lines(doc: &Document) -> Option<Vec<String>> {
    let story = doc.story(HYMN_PERMISSIONS_STORY)?;
    let mut paragraphs = story.paragraphs.iter();
    let first = paragraphs.next();
    let has_heading = first
        .map(|p| p.text() == "License permissions")
        .unwrap_or(false);
    let body: Vec<String> = if has_heading {
        paragraphs.map(|p| p.text().to_string()).collect()
    } else {
        story
            .paragraphs
            .iter()
            .map(|p| p.text().to_string())
            .collect()
    };
    Some(body)
}

/// Whether a document carries a Permissions panel at
/// all: its story, a panel frame pointing at that
/// story (a hand-edited panel may have lost its story
/// while keeping its frame), or a bare panel object
/// carrying the Permissions id with no frame
/// displaying the story. The bare-id shape stays
/// present so the office can remove it explicitly
/// rather than have an update append a second panel
/// beside a dead one; it is never printable (see
/// below), so it reports as an orphan.
fn permission_panel_present(doc: &Document) -> bool {
    if doc.story(HYMN_PERMISSIONS_STORY).is_some() {
        return true;
    }
    if doc.panels.iter().any(|p| p.id == "panel-permissions") {
        return true;
    }
    permission_panel_frame_present(doc)
}

/// Whether a panel frame points at the Permissions
/// story: the frame is what puts the story's words on
/// a page, so a story without one prints nowhere.
/// Round 56, batch A, R56-RED-1: the panel id alone
/// does NOT count — a panel object with its frames
/// cleared, or with its frame retargeted away from
/// the story, displays nothing, and the old id branch
/// reported such shapes already current.
fn permission_panel_frame_present(doc: &Document) -> bool {
    doc.panels.iter().any(|p| {
        p.frames
            .iter()
            .any(|f| f.story_id.as_deref() == Some(HYMN_PERMISSIONS_STORY))
    })
}

/// Whether the Permissions panel actually prints: its
/// story AND a frame pointing at that story (round 55,
/// batch C, R54-RED-3). Either half alone is an orphan
/// shape, not a panel on the page.
pub fn permission_panel_printable(doc: &Document) -> bool {
    doc.story(HYMN_PERMISSIONS_STORY).is_some() && permission_panel_frame_present(doc)
}

/// Whether the document carries an orphan Permissions
/// panel shape: its story without a frame displaying
/// it (round 56, batch A, R56-RED-1: frames cleared
/// or retargeted away from the story count here, not
/// just a panel object lost outright), or its frame
/// without its story (round 55, batch C, R54-RED-3).
/// The Check step offers Remove for exactly these
/// shapes.
pub fn permission_panel_orphan(doc: &Document) -> bool {
    permission_panel_present(doc) && !permission_panel_printable(doc)
}

/// What updating the hymn permission panel did
/// (round 53, batch B, R53-RED-3): the panel action
/// never reports the freshly computed lines as "now on
/// the page" unless this very call put them there.
/// `warnings` rides every variant, as in the append
/// outcome: licensed-without-number hymns warn, never
/// block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HymnPermissionUpdate {
    /// No panel stood, and one was appended.
    Appended {
        lines: Vec<String>,
        warnings: Vec<String>,
    },
    /// A printable panel stood and its lines already
    /// equal the freshly computed lines: nothing
    /// changed. Printability is part of the claim
    /// (round 55, batch C, R54-RED-3): an orphan shape
    /// never reports this, even when its story lines
    /// happen to equal the computed lines or when both
    /// sides are empty.
    AlreadyCurrent {
        lines: Vec<String>,
        warnings: Vec<String>,
    },
    /// An orphan panel shape stood (round 55, batch C,
    /// R54-RED-3): its story without a frame, or its
    /// frame without its story, so no permission line
    /// prints. Nothing changed, and nothing is deleted
    /// quietly: the office removes the orphan
    /// explicitly (see `remove_hymn_permission_panel`),
    /// then updates again to append a whole panel if
    /// the lines still belong. `panel_lines` is the
    /// story's lines, or empty for a frame-only shape
    /// with no story to read.
    Orphan {
        panel_lines: Vec<String>,
        warnings: Vec<String>,
    },
    /// A panel stood, its lines differed, and the fresh
    /// lines are non-empty: the panel paragraphs were
    /// rewritten to the fresh lines.
    Refreshed {
        old_lines: Vec<String>,
        new_lines: Vec<String>,
        warnings: Vec<String>,
    },
    /// A panel stood, its lines differed, and the fresh
    /// lines are empty: nothing changed, and the panel
    /// was NOT silently deleted. The office removes it
    /// explicitly (see `remove_hymn_permission_panel`)
    /// — the hymns may have been removed, or the hymn
    /// memory may be absent on this machine, and either
    /// way the printed lines are the office's to keep
    /// or drop, not the action's to drop quietly.
    RefusedStale {
        panel_lines: Vec<String>,
        warnings: Vec<String>,
    },
    /// No panel stood and no fresh line exists: nothing
    /// changed, and no empty panel was appended.
    NoLines { warnings: Vec<String> },
}

/// Update a document's hymn permission panel
/// (round 53, batch B, R53-RED-3): the explicit panel
/// action behind the "Update permission panel" button.
/// Never automatic: no save or print path calls this.
/// The caller snapshots for undo before calling — this
/// mutates in place, as the append does, and undo only
/// works off the caller's snapshot.
pub fn update_hymn_permission_panel(
    doc: &mut Document,
    memory: &HymnMemory,
) -> HymnPermissionUpdate {
    let uses = hymn_license_uses(doc, memory);
    let lines: Vec<String> = uses.iter().filter_map(|u| u.line.clone()).collect();
    let warnings: Vec<String> = uses
        .iter()
        .filter(|u| u.missing_number)
        .map(|u| {
            format!(
                "Hymn {} ({}) is reprinted under a church license but no license number is \
                 recorded, so no permission line with the number prints.",
                u.shown, u.label
            )
        })
        .collect();
    if !permission_panel_present(doc) {
        if lines.is_empty() {
            return HymnPermissionUpdate::NoLines { warnings };
        }
        let out = append_hymn_permission_panel(doc, memory);
        debug_assert!(out.added);
        return HymnPermissionUpdate::Appended {
            lines: out.lines,
            warnings: out.warnings,
        };
    }
    let printable = permission_panel_printable(doc);
    let panel_lines = permission_panel_lines(doc).unwrap_or_default();
    if printable && panel_lines == lines {
        return HymnPermissionUpdate::AlreadyCurrent { lines, warnings };
    }
    if !printable {
        // An orphan shape is never current and never
        // "refreshed" into matching: rewriting a
        // story-only panel's paragraphs still leaves
        // nothing on the page, and a frame-only panel
        // with no fresh lines has nothing to print
        // either. The one exception is the frame-only
        // repair that already held: fresh lines exist,
        // so the refresh below recreates the story
        // under the standing frame and the panel
        // prints again. Every other orphan shape
        // reports Orphan and changes nothing.
        let frame_only_repair = doc.story(HYMN_PERMISSIONS_STORY).is_none() && !lines.is_empty();
        if !frame_only_repair {
            return HymnPermissionUpdate::Orphan {
                panel_lines,
                warnings,
            };
        }
    }
    if lines.is_empty() {
        return HymnPermissionUpdate::RefusedStale {
            panel_lines,
            warnings,
        };
    }
    let mut paragraphs = vec![Paragraph::plain("subheading", "License permissions")];
    for line in &lines {
        paragraphs.push(Paragraph::plain("body", line));
    }
    match doc.story_mut(HYMN_PERMISSIONS_STORY) {
        Some(story) => story.paragraphs = paragraphs,
        None => doc.stories.push(crate::Story {
            id: HYMN_PERMISSIONS_STORY.to_string(),
            paragraphs,
            keep_through: None,
        }),
    }
    HymnPermissionUpdate::Refreshed {
        old_lines: panel_lines,
        new_lines: lines,
        warnings,
    }
}

/// Remove a document's Permissions panel outright
/// (round 53, batch B, R53-RED-3): the office's
/// explicit act behind the "Remove permission panel"
/// button, offered beside a refused-stale warning or
/// an orphan panel note (round 55, batch C, R54-RED-3).
/// The caller snapshots for undo before calling.
/// Answers true when a panel story or panel was
/// actually removed.
pub fn remove_hymn_permission_panel(doc: &mut Document) -> bool {
    let mut removed = false;
    if let Some(pos) = doc
        .stories
        .iter()
        .position(|s| s.id == HYMN_PERMISSIONS_STORY)
    {
        doc.stories.remove(pos);
        removed = true;
    }
    let before = doc.panels.len();
    doc.panels.retain(|p| {
        !(p.id == "panel-permissions"
            || p.frames
                .iter()
                .any(|f| f.story_id.as_deref() == Some(HYMN_PERMISSIONS_STORY)))
    });
    removed || doc.panels.len() != before
}

/// Append the hymn permission lines to a document as
/// printable words (round 51, G-08d): one Permissions
/// panel at the end, carrying one story and frame with
/// the lines, so they print through the normal print
/// path exactly as the music draft's credit line does.
/// Explicit, never automatic: no save or print path
/// calls this on its own, so a bulletin never gains a
/// page behind the office's back; the office runs it
/// when the permission lines belong on the page.
/// Idempotent: a document already carrying the story
/// gains no second panel. A document with no licensed
/// hymn carrying a number gains no panel at all, and
/// the licensed-without-number warnings still come back.
pub fn append_hymn_permission_panel(
    doc: &mut Document,
    memory: &HymnMemory,
) -> HymnPermissionOutcome {
    let uses = hymn_license_uses(doc, memory);
    let lines: Vec<String> = uses.iter().filter_map(|u| u.line.clone()).collect();
    let warnings: Vec<String> = uses
        .iter()
        .filter(|u| u.missing_number)
        .map(|u| {
            format!(
                "Hymn {} ({}) is reprinted under a church license but no license number is \
                 recorded, so no permission line with the number prints.",
                u.shown, u.label
            )
        })
        .collect();
    if doc.story(HYMN_PERMISSIONS_STORY).is_some() || lines.is_empty() {
        return HymnPermissionOutcome {
            lines,
            warnings,
            added: false,
        };
    }
    let size = doc
        .panels
        .last()
        .map(|p| p.size)
        .unwrap_or_else(|| Size::new(crate::inch(5.5), crate::inch(8.5)));
    let mut paragraphs = vec![Paragraph::plain("subheading", "License permissions")];
    for line in &lines {
        paragraphs.push(Paragraph::plain("body", line));
    }
    let margin = crate::inch(0.45);
    let frame = Frame {
        id: "frame-hymn-permissions".to_string(),
        name: "License permissions".to_string(),
        kind: FrameKind::Text,
        rect: Rect::new(
            margin,
            crate::inch(0.4),
            size.w - 2 * margin,
            size.h - crate::inch(0.8),
        ),
        story_id: Some(HYMN_PERMISSIONS_STORY.to_string()),
        role: ContentRole::Standing,
        picture: None,
        music: None,
    };
    doc.stories.push(crate::Story {
        id: HYMN_PERMISSIONS_STORY.to_string(),
        paragraphs,
        keep_through: None,
    });
    doc.panels.push(Panel {
        id: "panel-permissions".to_string(),
        name: "Permissions".to_string(),
        size,
        frames: vec![frame],
        background_image: None,
        guides: Vec::new(),
    });
    HymnPermissionOutcome {
        lines,
        warnings,
        added: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(hymnal: Option<&str>, number: Option<&str>, title: Option<&str>) -> HymnEntry {
        HymnEntry {
            hymnal: hymnal.map(str::to_string),
            number: number.map(str::to_string),
            title: title.map(str::to_string),
        }
    }

    #[test]
    fn parses_the_shapes_churches_write() {
        assert_eq!(
            parse_hymn_value("ELW 432"),
            entry(Some("ELW"), Some("432"), None)
        );
        assert_eq!(
            parse_hymn_value("LBW 165"),
            entry(Some("LBW"), Some("165"), None)
        );
        assert_eq!(parse_hymn_value("432"), entry(None, Some("432"), None));
        assert_eq!(parse_hymn_value("Hymn 432"), entry(None, Some("432"), None));
        assert_eq!(parse_hymn_value(""), entry(None, None, None));
        assert_eq!(
            parse_hymn_value("  ELW   432  "),
            entry(Some("ELW"), Some("432"), None)
        );
    }

    #[test]
    fn parses_titles_in_every_separator_dress() {
        let want = entry(Some("ELW"), Some("432"), Some("O Come, All Ye Faithful"));
        assert_eq!(parse_hymn_value("ELW 432 'O Come, All Ye Faithful'"), want);
        assert_eq!(
            parse_hymn_value("ELW 432 \"O Come, All Ye Faithful\""),
            want
        );
        assert_eq!(
            parse_hymn_value("ELW 432 \u{2013} O Come, All Ye Faithful"),
            want
        );
        assert_eq!(parse_hymn_value("ELW 432, O Come, All Ye Faithful"), want);
        assert_eq!(parse_hymn_value("ELW 432 O Come, All Ye Faithful"), want);
        assert_eq!(
            parse_hymn_value("432 \u{00b7} Amazing Grace"),
            entry(None, Some("432"), Some("Amazing Grace"))
        );
        assert_eq!(
            parse_hymn_value("432 - Amazing Grace"),
            entry(None, Some("432"), Some("Amazing Grace"))
        );
    }

    #[test]
    fn a_title_alone_is_a_title() {
        assert_eq!(
            parse_hymn_value("Amazing Grace"),
            entry(None, None, Some("Amazing Grace"))
        );
        assert_eq!(
            parse_hymn_value("'Amazing Grace'"),
            entry(None, None, Some("Amazing Grace"))
        );
        // A long first word is not a hymnal, even followed
        // by words that end in digits.
        assert_eq!(
            parse_hymn_value("Praise to the Lord, the Almighty"),
            entry(None, None, Some("Praise to the Lord, the Almighty"))
        );
    }

    #[test]
    fn compose_omits_missing_parts_gracefully() {
        assert_eq!(
            compose_hymn_value(&entry(Some("ELW"), Some("432"), Some("Amazing Grace"))),
            "ELW 432 \u{00b7} Amazing Grace"
        );
        assert_eq!(
            compose_hymn_value(&entry(Some("ELW"), Some("432"), None)),
            "ELW 432"
        );
        assert_eq!(compose_hymn_value(&entry(None, Some("432"), None)), "432");
        assert_eq!(
            compose_hymn_value(&entry(None, None, Some("Amazing Grace"))),
            "Amazing Grace"
        );
        assert_eq!(compose_hymn_value(&entry(None, None, None)), "");
    }

    #[test]
    fn compose_and_parse_round_trip() {
        for e in [
            entry(Some("ELW"), Some("432"), Some("O Come, All Ye Faithful")),
            entry(Some("LBW"), Some("165"), None),
            entry(None, Some("432"), Some("Amazing Grace")),
            entry(None, Some("432"), None),
            entry(None, None, Some("Amazing Grace")),
        ] {
            assert_eq!(parse_hymn_value(&compose_hymn_value(&e)), e, "{e:?}");
        }
        // A messier written form composes to the canonical
        // shape, and the canonical shape parses back.
        let parsed = parse_hymn_value("ELW 432, 'Amazing Grace'");
        assert_eq!(
            compose_hymn_value(&parsed),
            "ELW 432 \u{00b7} Amazing Grace"
        );
    }

    #[test]
    fn hymn_labels_are_named_by_the_word() {
        assert!(is_hymn_label("Opening Hymn"));
        assert!(is_hymn_label("Hymn of the Day"));
        assert!(is_hymn_label("closing hymn"));
        assert!(!is_hymn_label("Anthem"));
        assert!(!is_hymn_label("Prelude"));
        assert!(!is_hymn_label(""));
    }

    fn d(year: i32, month: u32, day: u32) -> SimpleDate {
        SimpleDate { year, month, day }
    }

    #[test]
    fn memory_learns_pairs_and_the_newer_sunday_wins() {
        let mut mem = HymnMemory::default();
        let e = entry(Some("ELW"), Some("432"), Some("O Come, All Ye Faithful"));
        assert!(mem.learn(&e, d(2026, 10, 4)));
        let pair = mem.lookup(Some("ELW"), "432").expect("remembered");
        assert_eq!(pair.title, "O Come, All Ye Faithful");
        assert_eq!(pair.last_seen, d(2026, 10, 4));
        // The hymnal's case does not matter.
        assert!(mem.lookup(Some("elw"), "432").is_some());
        // An older Sunday never overwrites.
        let older = entry(Some("ELW"), Some("432"), Some("A Different Song"));
        assert!(!mem.learn(&older, d(2026, 9, 27)));
        assert_eq!(
            mem.lookup(Some("ELW"), "432").unwrap().title,
            "O Come, All Ye Faithful"
        );
        // The same Sunday: the last confirmation wins.
        // Re-saving the same bulletin with the title
        // corrected is how a typo gets repaired; the
        // first confirmation keeping the seat made a
        // wrong pair unfixable (round 3, R3-6).
        assert!(mem.learn(&older, d(2026, 10, 4)));
        assert_eq!(
            mem.lookup(Some("ELW"), "432").unwrap().title,
            "A Different Song"
        );
        assert_eq!(
            mem.lookup(Some("ELW"), "432").unwrap().last_seen,
            d(2026, 10, 4)
        );
        // Confirming the same pair again changes nothing.
        assert!(!mem.learn(&older, d(2026, 10, 4)));
        // An older Sunday still never overwrites.
        assert!(!mem.learn(&e, d(2026, 9, 27)));
        // A newer Sunday wins.
        assert!(mem.learn(&older, d(2026, 10, 11)));
        assert_eq!(
            mem.lookup(Some("ELW"), "432").unwrap().title,
            "A Different Song"
        );
        assert_eq!(
            mem.lookup(Some("ELW"), "432").unwrap().last_seen,
            d(2026, 10, 11)
        );
        // Number-only and title-only entries teach nothing.
        assert!(!mem.learn(&entry(Some("ELW"), Some("500"), None), d(2026, 10, 11)));
        assert!(!mem.learn(&entry(None, None, Some("Orphan Title")), d(2026, 10, 11)));
        assert!(mem.lookup(Some("ELW"), "500").is_none());
    }

    #[test]
    fn memory_learns_from_a_documents_hymn_lines() {
        use crate::{ChurchProfile, Paragraph, SheetSpec, Size, Story, FORMAT_VERSION};
        use std::collections::BTreeMap;
        let doc = Document {
            format_version: FORMAT_VERSION,
            title: "t".to_string(),
            profile: ChurchProfile::default(),
            fields: BTreeMap::new(),
            styles: Vec::new(),
            stories: vec![Story {
                id: "order".to_string(),
                keep_through: None,
                paragraphs: vec![
                    Paragraph::plain(
                        "order-line",
                        "Hymn of Praise\tELW 432 \u{00b7} O Come, All Ye Faithful",
                    ),
                    Paragraph::plain("order-line", "Hymn of Response\t365"),
                    Paragraph::plain("order-line", "Prelude\t"),
                    Paragraph::plain("body", "An announcement with a\ttab and no hymn."),
                ],
            }],
            panels: Vec::new(),
            sheet: SheetSpec {
                preset: "letter-portrait".to_string(),
                size: Size::new(0, 0),
            },
            no_announcements: false,
            assets: Vec::new(),
            asset_bytes: BTreeMap::new(),
            occasion_time: None,
            wanted_fonts: Vec::new(),
            participants: BTreeMap::new(),
            inserts: Vec::new(),
            applied_contributions: Vec::new(),
        };
        let mut mem = HymnMemory::default();
        assert!(mem.learn_from_document(&doc, d(2026, 10, 18)));
        // Only the line with both a number and a title taught.
        let pair = mem.lookup(Some("ELW"), "432").expect("remembered");
        assert_eq!(pair.title, "O Come, All Ye Faithful");
        assert_eq!(mem.pairs.len(), 1);
        // The same document again changes nothing.
        assert!(!mem.learn_from_document(&doc, d(2026, 10, 18)));
    }

    #[test]
    fn a_typo_saved_then_corrected_on_the_same_sunday_heals() {
        // Round 3, R3-6: next Sunday's bulletin is drafted
        // early with a wrong title and saved; the office
        // spots it, fixes the same file, and saves again.
        // The correction must displace the typo, or Check
        // flags the correct bulletin forever.
        let mut mem = HymnMemory::default();
        let typo = entry(Some("ELW"), Some("432"), Some("O Come, All Ye Faithful"));
        let fixed = entry(
            Some("ELW"),
            Some("432"),
            Some("Joyful, Joyful We Adore Thee"),
        );
        assert!(mem.learn(&typo, d(2026, 10, 25)));
        assert!(mem.learn(&fixed, d(2026, 10, 25)));
        let pair = mem.lookup(Some("ELW"), "432").expect("remembered");
        assert_eq!(pair.title, "Joyful, Joyful We Adore Thee");
        assert_eq!(pair.last_seen, d(2026, 10, 25));
    }

    #[test]
    fn forgetting_a_pair_removes_it_and_it_can_be_learned_again() {
        // Round 3, R3-6: the office can tell Check to
        // forget a pairing outright; the removal is what
        // the persisted file carries, and nothing bars
        // the pair from being taught again by a later
        // save.
        let mut mem = HymnMemory::default();
        let e = entry(Some("ELW"), Some("432"), Some("O Come, All Ye Faithful"));
        assert!(mem.learn(&e, d(2026, 10, 4)));
        assert!(
            mem.forget(Some("elw"), "432"),
            "the hymnal's case does not matter"
        );
        assert!(mem.lookup(Some("ELW"), "432").is_none());
        assert!(!mem.forget(Some("ELW"), "432"), "nothing left to forget");
        assert!(!mem.forget_key("elw|999"));
        // The forgotten state is what a round trip keeps.
        let text = serde_json::to_string(&mem).expect("serializes");
        let back: HymnMemory = serde_json::from_str(&text).expect("parses");
        assert_eq!(back, mem);
        // And the pair can be learned again, at its new
        // confirmation date.
        assert!(mem.learn(&e, d(2026, 10, 11)));
        assert_eq!(
            mem.lookup(Some("ELW"), "432").unwrap().last_seen,
            d(2026, 10, 11)
        );
    }

    #[test]
    fn memory_survives_a_json_round_trip() {
        let mut mem = HymnMemory::default();
        mem.learn(
            &entry(Some("ELW"), Some("432"), Some("O Come, All Ye Faithful")),
            d(2026, 10, 18),
        );
        mem.learn(
            &entry(None, Some("88"), Some("A Title With No Book")),
            d(2026, 10, 18),
        );
        let text = serde_json::to_string(&mem).expect("serializes");
        let back: HymnMemory = serde_json::from_str(&text).expect("parses");
        assert_eq!(back, mem);
        assert_eq!(hymn_key(None, "88"), "|88");
    }

    fn licensed_pair(licensor: &str, number: &str, copyright: &str) -> HymnLicense {
        HymnLicense {
            kind: LicenseKind::LicensedReprint,
            licensor: licensor.to_string(),
            number: number.to_string(),
            copyright_line: copyright.to_string(),
        }
    }

    fn order_doc(lines: Vec<&str>) -> Document {
        use crate::{ChurchProfile, Paragraph, SheetSpec, Size, Story, FORMAT_VERSION};
        use std::collections::BTreeMap;
        let mut paragraphs = vec![Paragraph::plain("body", "Sermon:\tPastor")];
        for line in lines {
            paragraphs.push(Paragraph::plain("order-line", line));
        }
        Document {
            format_version: FORMAT_VERSION,
            title: "t".to_string(),
            profile: ChurchProfile::default(),
            fields: BTreeMap::new(),
            styles: Vec::new(),
            stories: vec![Story {
                id: "order".to_string(),
                keep_through: None,
                paragraphs,
            }],
            panels: vec![Panel {
                id: "panel-0".to_string(),
                name: "Order".to_string(),
                size: Size::new(396_000, 612_000),
                frames: vec![Frame {
                    id: "frame-0".to_string(),
                    name: "Order".to_string(),
                    kind: FrameKind::Text,
                    rect: Rect::new(0, 0, 396_000, 612_000),
                    story_id: Some("order".to_string()),
                    role: ContentRole::Standing,
                    picture: None,
                    music: None,
                }],
                background_image: None,
                guides: Vec::new(),
            }],
            sheet: SheetSpec {
                preset: "letter-portrait".to_string(),
                size: Size::new(0, 0),
            },
            no_announcements: false,
            assets: Vec::new(),
            asset_bytes: BTreeMap::new(),
            occasion_time: None,
            wanted_fonts: Vec::new(),
            participants: BTreeMap::new(),
            inserts: Vec::new(),
            applied_contributions: Vec::new(),
        }
    }

    fn memory_with(
        hymnal: Option<&str>,
        number: &str,
        title: &str,
        license: HymnLicense,
    ) -> HymnMemory {
        let mut mem = HymnMemory::default();
        mem.learn(&entry(hymnal, Some(number), Some(title)), d(2026, 10, 18));
        assert!(mem.note_license(hymnal, number, license));
        mem
    }

    #[test]
    fn hymn_permission_lines_carry_the_license_number() {
        let mem = memory_with(
            Some("ELW"),
            "432",
            "O Come, All Ye Faithful",
            licensed_pair("CCLI", "123456", "© 2019 Jane Doe"),
        );
        let doc = order_doc(vec!["Hymn of Praise\tELW 432 · O Come, All Ye Faithful"]);
        let uses = hymn_license_uses(&doc, &mem);
        assert_eq!(uses.len(), 1);
        let line = uses[0].line.as_ref().expect("a line prints");
        assert!(line.contains("CCLI license #123456"), "{line}");
        assert!(line.contains("O Come, All Ye Faithful"), "{line}");
        assert!(line.contains("© 2019 Jane Doe"), "{line}");
        assert!(!uses[0].missing_number);
    }

    #[test]
    fn one_license_hymn_line_uses_the_one_license_words() {
        let mem = memory_with(
            None,
            "88",
            "A Title With No Book",
            licensed_pair("ONE LICENSE", "A-987654", "© 2020 John Smith"),
        );
        let doc = order_doc(vec!["Hymn of Response\t88 · A Title With No Book"]);
        let uses = hymn_license_uses(&doc, &mem);
        assert_eq!(uses.len(), 1);
        let line = uses[0].line.as_ref().expect("a line prints");
        assert!(line.contains("ONE LICENSE #A-987654"), "{line}");
        assert!(line.contains("© 2020 John Smith"), "{line}");
    }

    #[test]
    fn a_licensed_hymn_without_a_number_warns_and_gets_no_line() {
        let mem = memory_with(
            Some("ELW"),
            "432",
            "O Come, All Ye Faithful",
            licensed_pair("CCLI", "", "© 2019 Jane Doe"),
        );
        let doc = order_doc(vec!["Hymn of Praise\tELW 432 · O Come, All Ye Faithful"]);
        let uses = hymn_license_uses(&doc, &mem);
        assert_eq!(uses.len(), 1);
        assert!(uses[0].line.is_none());
        assert!(uses[0].missing_number);
    }

    #[test]
    fn license_notes_do_not_invent_hymns_and_survive_relearning() {
        let mut mem = HymnMemory::default();
        let license = licensed_pair("CCLI", "123456", "© 2019 Jane Doe");
        assert!(!mem.note_license(Some("ELW"), "432", license.clone()));
        assert!(mem.pairs.is_empty(), "no pair was invented");
        mem.learn(
            &entry(Some("ELW"), Some("432"), Some("O Come, All Ye Faithful")),
            d(2026, 10, 18),
        );
        assert!(mem.note_license(Some("ELW"), "432", license));
        // A corrected title re-learned later keeps the license.
        mem.learn(
            &entry(Some("ELW"), Some("432"), Some("O Come, All Ye Faithful!")),
            d(2026, 10, 25),
        );
        let pair = mem.lookup(Some("ELW"), "432").expect("remembered");
        assert_eq!(pair.license.number, "123456");
        assert_eq!(pair.title, "O Come, All Ye Faithful!");
    }

    #[test]
    fn permission_panel_appends_once_and_warns_without_blocking() {
        let mut mem = memory_with(
            Some("ELW"),
            "432",
            "O Come, All Ye Faithful",
            licensed_pair("CCLI", "123456", "© 2019 Jane Doe"),
        );
        mem.learn(
            &entry(None, Some("77"), Some("Numberless Song")),
            d(2026, 10, 18),
        );
        assert!(mem.note_license(
            None,
            "77",
            licensed_pair("ONE LICENSE", "", "© 2020 John Smith")
        ));
        let mut doc = order_doc(vec![
            "Hymn of Praise\tELW 432 · O Come, All Ye Faithful",
            "Hymn of Response\t77 · Numberless Song",
            "Hymn of Praise\tELW 432 · O Come, All Ye Faithful",
        ]);
        let out = append_hymn_permission_panel(&mut doc, &mem);
        assert!(out.added, "the panel appends");
        assert_eq!(out.lines.len(), 1, "the twice-sung hymn prints once");
        assert!(
            out.lines[0].contains("CCLI license #123456"),
            "{}",
            out.lines[0]
        );
        assert_eq!(out.warnings.len(), 1, "the numberless hymn warns");
        assert!(out.warnings[0].contains("77"), "{}", out.warnings[0]);
        // The panel is printable words: a story and a frame on it.
        let story = doc.story(HYMN_PERMISSIONS_STORY).expect("the story");
        assert!(story
            .paragraphs
            .iter()
            .any(|p| p.text().contains("CCLI license #123456")));
        assert_eq!(doc.panels.len(), 2);
        assert_eq!(
            doc.panels[1].frames[0].story_id.as_deref(),
            Some(HYMN_PERMISSIONS_STORY)
        );
        // A second call appends nothing twice.
        let again = append_hymn_permission_panel(&mut doc, &mem);
        assert!(!again.added);
        assert_eq!(doc.panels.len(), 2);
    }

    #[test]
    fn permission_panel_update_replays_the_p2_stale_panel() {
        // Round 53, R53-RED-3, probe P2 replayed through
        // the update action: first append prints CCLI
        // #123456 and ONE LICENSE #A-777; after the hymn
        // edits (one hymn removed, the other renumbered
        // to #A-999) the refresh replaces the page lines
        // with #A-999, and the report and page agree.
        let mut mem = memory_with(
            Some("ELW"),
            "432",
            "Amazing Grace",
            licensed_pair("CCLI", "123456", "© 2019 Jane Doe"),
        );
        mem.learn(
            &entry(Some("ELW"), Some("365"), Some("Holy Holy")),
            d(2026, 10, 18),
        );
        assert!(mem.note_license(
            Some("ELW"),
            "365",
            licensed_pair("ONE LICENSE", "A-777", "© 2020 John Smith")
        ));
        let mut doc = order_doc(vec![
            "Hymn of Praise\tELW 432 · Amazing Grace",
            "Hymn of Response\tELW 365 · Holy Holy",
        ]);
        let out = append_hymn_permission_panel(&mut doc, &mem);
        assert!(out.added);
        assert_eq!(doc.panels.len(), 2);
        // The office edits the service: the praise hymn
        // goes, the response hymn is renumbered.
        assert!(mem.note_license(
            Some("ELW"),
            "365",
            licensed_pair("ONE LICENSE", "A-999", "© 2020 John Smith")
        ));
        let mut edited = order_doc(vec!["Hymn of Response\tELW 365 · Holy Holy"]);
        edited.stories.push(
            doc.story(HYMN_PERMISSIONS_STORY)
                .expect("the appended story")
                .clone(),
        );
        edited.panels.push(doc.panels[1].clone());
        let update = update_hymn_permission_panel(&mut edited, &mem);
        match update {
            HymnPermissionUpdate::Refreshed {
                old_lines,
                new_lines,
                ..
            } => {
                assert_eq!(old_lines.len(), 2, "{old_lines:?}");
                assert_eq!(new_lines.len(), 1, "{new_lines:?}");
                assert!(
                    new_lines[0].contains("ONE LICENSE #A-999"),
                    "{}",
                    new_lines[0]
                );
            }
            other => panic!("expected a refresh, got {other:?}"),
        }
        let page = permission_panel_lines(&edited).expect("the panel stands");
        assert_eq!(page.len(), 1, "{page:?}");
        assert!(page[0].contains("ONE LICENSE #A-999"), "{}", page[0]);
        assert!(
            !page
                .iter()
                .any(|l| l.contains("123456") || l.contains("A-777")),
            "no stale line survives the refresh: {page:?}"
        );
        // A second update now reports already current and
        // changes nothing.
        let again = update_hymn_permission_panel(&mut edited, &mem);
        assert!(
            matches!(again, HymnPermissionUpdate::AlreadyCurrent { .. }),
            "{again:?}"
        );
        assert_eq!(edited.panels.len(), 2);
    }

    #[test]
    fn permission_panel_update_refuses_a_stale_panel_instead_of_deleting_it() {
        // Round 53, R53-RED-3, cross-machine shape: the
        // panel prints lines, but on this machine the
        // current hymns yield none (memory absent). The
        // update refuses, the panel stands untouched, and
        // only the explicit removal drops it.
        let mem = memory_with(
            Some("ELW"),
            "432",
            "Amazing Grace",
            licensed_pair("CCLI", "123456", "© 2019 Jane Doe"),
        );
        let mut doc = order_doc(vec!["Hymn of Praise\tELW 432 · Amazing Grace"]);
        assert!(append_hymn_permission_panel(&mut doc, &mem).added);
        let mut moved = order_doc(vec!["Hymn of Praise\tELW 432 · Amazing Grace"]);
        moved.stories.push(
            doc.story(HYMN_PERMISSIONS_STORY)
                .expect("the appended story")
                .clone(),
        );
        moved.panels.push(doc.panels[1].clone());
        let empty = HymnMemory::default();
        let update = update_hymn_permission_panel(&mut moved, &empty);
        match update {
            HymnPermissionUpdate::RefusedStale { panel_lines, .. } => {
                assert_eq!(panel_lines.len(), 1, "{panel_lines:?}");
                assert!(panel_lines[0].contains("123456"), "{}", panel_lines[0]);
            }
            other => panic!("expected a refusal, got {other:?}"),
        }
        assert_eq!(moved.panels.len(), 2, "the refused panel stands");
        assert!(
            permission_panel_lines(&moved).expect("the panel stands")[0].contains("123456"),
            "the refused panel is unchanged"
        );
        assert!(remove_hymn_permission_panel(&mut moved));
        assert_eq!(moved.panels.len(), 1);
        assert!(permission_panel_lines(&moved).is_none());
        assert!(!remove_hymn_permission_panel(&mut moved));
    }

    #[test]
    fn r55c_orphan_story_only_is_not_already_current_and_removes_cleanly() {
        // Round 55, batch C, R54-RED-3, shape A
        // (round-54 probe H2): the panel frame is lost
        // but its story stands, so no permission line
        // prints — even though the story lines still
        // equal the computed lines. The update must
        // not report AlreadyCurrent, must change
        // nothing, and the explicit removal must drop
        // the orphan story, after which an update
        // appends a whole panel again.
        let mut mem = memory_with(
            Some("ELW"),
            "432",
            "Amazing Grace",
            licensed_pair("CCLI", "123456", "© 2019 Jane Doe"),
        );
        mem.learn(
            &entry(Some("ELW"), Some("365"), Some("Holy Holy")),
            d(2026, 10, 18),
        );
        assert!(mem.note_license(
            Some("ELW"),
            "365",
            licensed_pair("ONE LICENSE", "A-777", "© 2020 John Smith")
        ));
        let mut doc = order_doc(vec![
            "Hymn of Praise\tELW 432 · Amazing Grace",
            "Hymn of Response\tELW 365 · Holy Holy",
        ]);
        assert!(append_hymn_permission_panel(&mut doc, &mem).added);
        assert_eq!(doc.panels.len(), 2);
        // Healthy first: the whole panel reports
        // already current, unchanged.
        let healthy = update_hymn_permission_panel(&mut doc, &mem);
        assert!(
            matches!(healthy, HymnPermissionUpdate::AlreadyCurrent { .. }),
            "{healthy:?}"
        );
        // Lose the frame, keep the story.
        doc.panels.retain(|p| p.id != "panel-permissions");
        assert_eq!(doc.panels.len(), 1);
        assert!(permission_panel_present(&doc));
        assert!(!permission_panel_printable(&doc));
        assert!(permission_panel_orphan(&doc));
        let story_lines = permission_panel_lines(&doc).expect("the story stands");
        assert_eq!(story_lines.len(), 2, "{story_lines:?}");
        let update = update_hymn_permission_panel(&mut doc, &mem);
        match update {
            HymnPermissionUpdate::Orphan { panel_lines, .. } => {
                assert_eq!(panel_lines.len(), 2, "{panel_lines:?}");
            }
            other => panic!("a story-only panel is not current, got {other:?}"),
        }
        assert_eq!(doc.panels.len(), 1, "the orphan update changes nothing");
        assert_eq!(
            permission_panel_lines(&doc).expect("the story stands"),
            story_lines,
            "the orphan update changes nothing"
        );
        // With an empty memory the shape is still an
        // orphan, not a refused stale panel and never
        // already current with zero lines.
        let empty = HymnMemory::default();
        let update = update_hymn_permission_panel(&mut doc, &empty);
        assert!(
            matches!(update, HymnPermissionUpdate::Orphan { .. }),
            "{update:?}"
        );
        assert!(remove_hymn_permission_panel(&mut doc));
        assert!(!permission_panel_present(&doc));
        assert!(!permission_panel_orphan(&doc));
        assert!(permission_panel_lines(&doc).is_none());
        let again = update_hymn_permission_panel(&mut doc, &mem);
        assert!(
            matches!(again, HymnPermissionUpdate::Appended { .. }),
            "removal plus update restores a whole panel: {again:?}"
        );
        assert!(permission_panel_printable(&doc));
    }

    #[test]
    fn r55c_orphan_frame_only_with_empty_memory_is_not_already_current() {
        // Round 55, batch C, R54-RED-3, shape B
        // (round-54 probe H3): the story is lost but
        // its frame stands, and on this machine the
        // hymns yield nothing. The old comparison read
        // no story as [] and [] == [] as current; the
        // update must report Orphan instead, the dead
        // frame must stand until the office removes it,
        // and removal must drop it.
        let mem = memory_with(
            Some("ELW"),
            "432",
            "Amazing Grace",
            licensed_pair("CCLI", "123456", "© 2019 Jane Doe"),
        );
        let mut doc = order_doc(vec!["Hymn of Praise\tELW 432 · Amazing Grace"]);
        assert!(append_hymn_permission_panel(&mut doc, &mem).added);
        assert_eq!(doc.panels.len(), 2);
        doc.stories.retain(|s| s.id != HYMN_PERMISSIONS_STORY);
        assert!(permission_panel_present(&doc));
        assert!(!permission_panel_printable(&doc));
        assert!(permission_panel_orphan(&doc));
        assert!(permission_panel_lines(&doc).is_none());
        let empty = HymnMemory::default();
        let update = update_hymn_permission_panel(&mut doc, &empty);
        match update {
            HymnPermissionUpdate::Orphan { panel_lines, .. } => {
                assert!(panel_lines.is_empty(), "{panel_lines:?}");
            }
            other => panic!("a frame-only panel is not current, got {other:?}"),
        }
        assert_eq!(doc.panels.len(), 2, "the orphan frame stands");
        assert!(remove_hymn_permission_panel(&mut doc));
        assert_eq!(doc.panels.len(), 1);
        assert!(!permission_panel_present(&doc));
        assert!(!permission_panel_orphan(&doc));
        assert!(!remove_hymn_permission_panel(&mut doc));
    }

    #[test]
    fn r55c_orphan_frame_only_with_lines_still_refreshes_into_a_panel() {
        // Round 55, batch C, R54-RED-3, the repair
        // direction that already held: a frame-only
        // shape with fresh lines recreates its story
        // under the standing frame, so the panel prints
        // again — and the next update reports the
        // healthy panel already current.
        let mem = memory_with(
            Some("ELW"),
            "432",
            "Amazing Grace",
            licensed_pair("CCLI", "123456", "© 2019 Jane Doe"),
        );
        let mut doc = order_doc(vec!["Hymn of Praise\tELW 432 · Amazing Grace"]);
        assert!(append_hymn_permission_panel(&mut doc, &mem).added);
        doc.stories.retain(|s| s.id != HYMN_PERMISSIONS_STORY);
        assert!(permission_panel_orphan(&doc));
        let update = update_hymn_permission_panel(&mut doc, &mem);
        match update {
            HymnPermissionUpdate::Refreshed {
                old_lines,
                new_lines,
                ..
            } => {
                assert!(old_lines.is_empty(), "{old_lines:?}");
                assert_eq!(new_lines.len(), 1, "{new_lines:?}");
            }
            other => panic!("expected the frame-only repair, got {other:?}"),
        }
        assert!(permission_panel_printable(&doc));
        assert_eq!(
            permission_panel_lines(&doc)
                .expect("the story is back")
                .len(),
            1
        );
        let again = update_hymn_permission_panel(&mut doc, &mem);
        assert!(
            matches!(again, HymnPermissionUpdate::AlreadyCurrent { .. }),
            "{again:?}"
        );
    }

    #[test]
    fn r56a_orphan_frameless_panel_with_matching_memory_is_not_already_current() {
        // Round 56, batch A, R56-RED-1, probe V1: a
        // panel-permissions panel with all its frames
        // cleared — the panel object and the story
        // stand, but no frame displays the story, so
        // no permission line prints. With memory
        // matching the story lines, the old panel-id
        // branch reported AlreadyCurrent; the update
        // must report Orphan instead, change nothing,
        // and the explicit removal must drop the
        // orphan, after which an update appends a
        // whole panel again.
        let mut mem = memory_with(
            Some("ELW"),
            "432",
            "Amazing Grace",
            licensed_pair("CCLI", "123456", "© 2019 Jane Doe"),
        );
        mem.learn(
            &entry(Some("ELW"), Some("365"), Some("Holy Holy")),
            d(2026, 10, 18),
        );
        assert!(mem.note_license(
            Some("ELW"),
            "365",
            licensed_pair("ONE LICENSE", "A-777", "© 2020 John Smith")
        ));
        let mut doc = order_doc(vec![
            "Hymn of Praise\tELW 432 · Amazing Grace",
            "Hymn of Response\tELW 365 · Holy Holy",
        ]);
        assert!(append_hymn_permission_panel(&mut doc, &mem).added);
        assert_eq!(doc.panels.len(), 2);
        let healthy = update_hymn_permission_panel(&mut doc, &mem);
        assert!(
            matches!(healthy, HymnPermissionUpdate::AlreadyCurrent { .. }),
            "{healthy:?}"
        );
        // Clear every frame from the Permissions
        // panel, keeping the panel object and story.
        let panel = doc
            .panels
            .iter_mut()
            .find(|p| p.id == "panel-permissions")
            .expect("the appended panel stands");
        panel.frames.clear();
        assert_eq!(doc.panels.len(), 2, "the panel object stands");
        assert!(permission_panel_present(&doc));
        assert!(!permission_panel_printable(&doc));
        assert!(permission_panel_orphan(&doc));
        let story_lines = permission_panel_lines(&doc).expect("the story stands");
        assert_eq!(story_lines.len(), 2, "{story_lines:?}");
        let update = update_hymn_permission_panel(&mut doc, &mem);
        match update {
            HymnPermissionUpdate::Orphan { panel_lines, .. } => {
                assert_eq!(panel_lines.len(), 2, "{panel_lines:?}");
            }
            other => panic!("a frameless panel is not current, got {other:?}"),
        }
        assert_eq!(doc.panels.len(), 2, "the orphan update changes nothing");
        assert_eq!(
            permission_panel_lines(&doc).expect("the story stands"),
            story_lines,
            "the orphan update changes nothing"
        );
        assert!(remove_hymn_permission_panel(&mut doc));
        assert!(!permission_panel_present(&doc));
        assert!(!permission_panel_orphan(&doc));
        assert!(permission_panel_lines(&doc).is_none());
        let again = update_hymn_permission_panel(&mut doc, &mem);
        assert!(
            matches!(again, HymnPermissionUpdate::Appended { .. }),
            "removal plus update restores a whole panel: {again:?}"
        );
        assert!(permission_panel_printable(&doc));
    }

    #[test]
    fn r56a_orphan_retargeted_frame_with_matching_memory_is_not_already_current() {
        // Round 56, batch A, R56-RED-1, probe V2: a
        // panel-permissions panel whose frame was
        // retargeted away from the Permissions story
        // — the panel object, frame, and story stand,
        // but no frame displays the story, so no
        // permission line prints. With memory matching
        // the story lines, the old panel-id branch
        // reported AlreadyCurrent; the update must
        // report Orphan instead, change nothing, and
        // the explicit removal must drop the orphan,
        // after which an update appends a whole panel
        // again.
        let mut mem = memory_with(
            Some("ELW"),
            "432",
            "Amazing Grace",
            licensed_pair("CCLI", "123456", "© 2019 Jane Doe"),
        );
        mem.learn(
            &entry(Some("ELW"), Some("365"), Some("Holy Holy")),
            d(2026, 10, 18),
        );
        assert!(mem.note_license(
            Some("ELW"),
            "365",
            licensed_pair("ONE LICENSE", "A-777", "© 2020 John Smith")
        ));
        let mut doc = order_doc(vec![
            "Hymn of Praise\tELW 432 · Amazing Grace",
            "Hymn of Response\tELW 365 · Holy Holy",
        ]);
        assert!(append_hymn_permission_panel(&mut doc, &mem).added);
        assert_eq!(doc.panels.len(), 2);
        let healthy = update_hymn_permission_panel(&mut doc, &mem);
        assert!(
            matches!(healthy, HymnPermissionUpdate::AlreadyCurrent { .. }),
            "{healthy:?}"
        );
        // Retarget the Permissions frame away from
        // the story, keeping panel, frame, and story.
        let panel = doc
            .panels
            .iter_mut()
            .find(|p| p.id == "panel-permissions")
            .expect("the appended panel stands");
        assert_eq!(panel.frames.len(), 1);
        panel.frames[0].story_id = None;
        assert_eq!(doc.panels.len(), 2, "the panel object stands");
        assert!(permission_panel_present(&doc));
        assert!(!permission_panel_printable(&doc));
        assert!(permission_panel_orphan(&doc));
        let story_lines = permission_panel_lines(&doc).expect("the story stands");
        assert_eq!(story_lines.len(), 2, "{story_lines:?}");
        let update = update_hymn_permission_panel(&mut doc, &mem);
        match update {
            HymnPermissionUpdate::Orphan { panel_lines, .. } => {
                assert_eq!(panel_lines.len(), 2, "{panel_lines:?}");
            }
            other => panic!("a retargeted panel is not current, got {other:?}"),
        }
        assert_eq!(doc.panels.len(), 2, "the orphan update changes nothing");
        assert_eq!(
            permission_panel_lines(&doc).expect("the story stands"),
            story_lines,
            "the orphan update changes nothing"
        );
        assert!(remove_hymn_permission_panel(&mut doc));
        assert!(!permission_panel_present(&doc));
        assert!(!permission_panel_orphan(&doc));
        assert!(permission_panel_lines(&doc).is_none());
        let again = update_hymn_permission_panel(&mut doc, &mem);
        assert!(
            matches!(again, HymnPermissionUpdate::Appended { .. }),
            "removal plus update restores a whole panel: {again:?}"
        );
        assert!(permission_panel_printable(&doc));
    }

    #[test]
    fn permission_panel_stays_absent_when_nothing_carries_a_number() {
        let mem = memory_with(
            Some("ELW"),
            "432",
            "O Come, All Ye Faithful",
            licensed_pair("CCLI", "", "© 2019 Jane Doe"),
        );
        let mut doc = order_doc(vec!["Hymn of Praise\tELW 432 · O Come, All Ye Faithful"]);
        let out = append_hymn_permission_panel(&mut doc, &mem);
        assert!(!out.added, "no number, no panel");
        assert_eq!(out.warnings.len(), 1);
        assert_eq!(doc.panels.len(), 1);
    }
}
