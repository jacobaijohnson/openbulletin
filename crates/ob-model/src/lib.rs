//! OpenBulletin document model.
//!
//! Design rules (see docs/planning/SYNTHESIS.md):
//! - Geometry is integer milli-points (1/1000 pt) so saves are stable and diffs are clean.
//! - The model knows nothing about windows, files, or printers.
//! - A document is a set of *reader panels* (what the congregation reads, in reading
//!   order). Sheets and folds belong to layout/imposition, never to the reader model.

use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::collections::BTreeMap;

pub mod hymns;
pub mod markdown;

/// One milli-point: 1/1000 of a PostScript point.
pub type Mpt = i64;
pub const MPT_PER_INCH: Mpt = 72_000;

/// Convert inches (e.g. 5.5) to milli-points.
pub fn inch(v: f64) -> Mpt {
    (v * MPT_PER_INCH as f64).round() as Mpt
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Size {
    pub w: Mpt,
    pub h: Mpt,
}
impl Size {
    pub fn new(w: Mpt, h: Mpt) -> Self {
        Self { w, h }
    }
}

/// Rectangle in panel space: origin top-left, y grows downward (screen convention).
/// The PDF exporter converts to PDF's bottom-up space in exactly one place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rect {
    pub x: Mpt,
    pub y: Mpt,
    pub w: Mpt,
    pub h: Mpt,
}
impl Rect {
    pub fn new(x: Mpt, y: Mpt, w: Mpt, h: Mpt) -> Self {
        Self { x, y, w, h }
    }
    /// Shrink by a uniform margin.
    pub fn inset(&self, m: Mpt) -> Rect {
        Rect::new(self.x + m, self.y + m, self.w - 2 * m, self.h - 2 * m)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}
impl Rgb {
    pub const BLACK: Rgb = Rgb { r: 0, g: 0, b: 0 };
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Align {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParagraphStyle {
    pub id: String,
    pub name: String,
    pub font_size: Mpt,
    pub bold: bool,
    /// Italic face of the family. Scripture quotes and sung
    /// responses are traditionally italic in bulletins.
    #[serde(default)]
    pub italic: bool,
    pub align: Align,
    pub space_after: Mpt,
    pub color: Rgb,
    /// Font family this style is set in, by name (docs/FONTS.md).
    /// The Font Library resolves the name to actual font bytes; the
    /// layout engine only ever shapes with library fonts, so the same
    /// document lays out identically on every machine.
    #[serde(default = "default_font_family")]
    pub font_family: String,
    /// SHA-256 of the exact font file this style was laid out with,
    /// when known. Pins the metrics even if a same-named font changes.
    #[serde(default)]
    pub font_sha256: Option<String>,
}

fn default_font_family() -> String {
    "Liberation Sans".to_string()
}

/// One span of text inside a paragraph with its own face
/// switches (G1b): a bold word in a body paragraph, an
/// italic scripture citation. `None` means "whatever the
/// paragraph's style says"; the effective face of a run is
/// `run.bold.unwrap_or(style.bold)` and likewise for
/// italic. Runs never cross a paragraph boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Run {
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bold: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub italic: Option<bool>,
}

impl Run {
    /// A run with no switches of its own: the style's face.
    pub fn plain(text: &str) -> Self {
        Self {
            text: text.to_string(),
            bold: None,
            italic: None,
        }
    }
}

/// The normalized form of a run list: adjacent runs with
/// equal switches merge, empty runs drop. A list that
/// normalizes to nothing becomes one empty plain run, so a
/// paragraph's run list is never empty. Idempotent: writers
/// store normalized lists, and loading normalizes again to
/// the same list.
pub fn normalize_runs(runs: Vec<Run>) -> Vec<Run> {
    let mut out: Vec<Run> = Vec::with_capacity(runs.len());
    for run in runs {
        if run.text.is_empty() {
            continue;
        }
        match out.last_mut() {
            Some(last) if last.bold == run.bold && last.italic == run.italic => {
                last.text.push_str(&run.text);
            }
            _ => out.push(run),
        }
    }
    if out.is_empty() {
        out.push(Run::plain(""));
    }
    out
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Paragraph {
    pub style: String,
    #[serde(default)]
    pub runs: Vec<Run>,
    /// Spoken only: this paragraph is read aloud in the
    /// service (a prayer-list name, above all) and must never
    /// reach a publication. Every export except the native
    /// .obd working file leaves spoken-only paragraphs out
    /// (docs/planning/beyond-print.md section 4). The canvas
    /// still shows them, tagged, so the state is never
    /// hidden. Serde-defaulted: files written before the
    /// flag load with every paragraph printable, as they
    /// were. Not written when false, so printable
    /// paragraphs serialize exactly as they always have.
    #[serde(default, skip_serializing_if = "is_false")]
    pub spoken_only: bool,
    /// Not for the web: this paragraph prints in the
    /// bulletin but stays off the web page (roles build
    /// R-2b). A printed bulletin is a room; the web page
    /// is the world, and some words (children's names
    /// above all) are for the room only. The HTML export
    /// is the one door that leaves these paragraphs out,
    /// and counts them; print, IDML, Scribus, and
    /// markdown all keep them. The canvas shows them,
    /// tagged, so the state is never hidden.
    /// Serde-defaulted and not written when false, on
    /// exactly the spoken-only discipline.
    #[serde(default, skip_serializing_if = "is_false")]
    pub web_hidden: bool,
}

fn is_false(b: &bool) -> bool {
    !*b
}

/// Paragraphs are stored as runs (G1b). Every file written
/// before runs stores `{ style, text }` instead; loading
/// accepts BOTH shapes (the old text becomes one plain run)
/// and saving always writes the runs shape.
impl<'de> Deserialize<'de> for Paragraph {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Shadow {
            style: String,
            #[serde(default)]
            text: Option<String>,
            #[serde(default)]
            runs: Option<Vec<Run>>,
            #[serde(default)]
            spoken_only: bool,
            #[serde(default)]
            web_hidden: bool,
        }
        let sh = Shadow::deserialize(deserializer)?;
        let runs = match (sh.runs, sh.text) {
            (Some(runs), _) => normalize_runs(runs),
            (None, Some(text)) => vec![Run::plain(&text)],
            (None, None) => vec![Run::plain("")],
        };
        Ok(Paragraph {
            style: sh.style,
            runs,
            spoken_only: sh.spoken_only,
            web_hidden: sh.web_hidden,
        })
    }
}

impl Paragraph {
    /// A paragraph in one plain run (the style's face
    /// throughout).
    pub fn plain(style: &str, text: &str) -> Self {
        Self {
            style: style.to_string(),
            runs: vec![Run::plain(text)],
            spoken_only: false,
            web_hidden: false,
        }
    }

    /// Kept for the many construction sites that predate
    /// runs: same as `plain`.
    pub fn new(style: &str, text: &str) -> Self {
        Self::plain(style, text)
    }

    /// A paragraph from runs, normalized.
    pub fn from_runs(style: &str, runs: Vec<Run>) -> Self {
        Self {
            style: style.to_string(),
            runs: normalize_runs(runs),
            spoken_only: false,
            web_hidden: false,
        }
    }

    /// The paragraph's whole text: its runs concatenated.
    /// Field tokens live here, unresolved; resolution is a
    /// render step (`resolve_fields`, `resolve_runs`).
    pub fn text(&self) -> String {
        self.runs.iter().map(|r| r.text.as_str()).collect()
    }

    /// Replace the paragraph's words with one plain run
    /// (whole-text writers: Next Sunday clearing a story,
    /// the announcements box).
    pub fn set_text(&mut self, text: &str) {
        self.runs = vec![Run::plain(text)];
    }
}

/// A story is text that belongs together. In later phases one story threads
/// across several frames; in v0.1 each frame holds at most one story.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Story {
    pub id: String,
    pub paragraphs: Vec<Paragraph>,
    /// Optional last Sunday (ISO date, YYYY-MM-DD) a weekly story
    /// stays in the bulletin. Empty (None) means "this Sunday
    /// only": Next Sunday clears the story's body paragraphs.
    /// With a date, the words survive Next Sunday while the date
    /// is on or after the new bulletin's date, and end (cleared,
    /// counted in the report) once it has passed. Serde-defaulted
    /// so bulletin files written before this field load fine.
    #[serde(default)]
    pub keep_through: Option<String>,
}

/// Whether content survives the "Next Sunday" command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContentRole {
    /// Standing content: service times, liturgy skeleton, notes lines.
    Standing,
    /// Weekly content: announcements and other text replaced each week.
    Weekly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FrameKind {
    Text,
    /// A simple rectangle/placeholder (cover art block, QR placeholder).
    Shape,
    /// A picture box: its picture (when it has one) is the
    /// `Frame::picture` placement beside this kind.
    Picture,
}

/// How a picture sits in its box (G2). The office words are
/// "Fill the box (sides may be trimmed)" and "Show the whole
/// picture"; these are the two shapes those words mean.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PictureFit {
    /// Scale to cover the box; the overflow is trimmed away.
    #[default]
    Fill,
    /// Scale to fit inside the box; empty space may show.
    ShowWhole,
}

/// One picture placed in a picture box: which stored asset,
/// how it fits, and where its center sits when Fill trims it.
/// The crop offsets are fractions of the trimmed overflow:
/// 0.0 shows the left/top edge of the picture, 1.0 the
/// right/bottom edge, 0.5 (the default) centers it. They are
/// fractions, not pixels, so the same placement survives the
/// box being resized.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PicturePlacement {
    /// The asset's SHA-256 fingerprint into `Document.assets`
    /// and `Document.asset_bytes`.
    pub asset: String,
    #[serde(default)]
    pub fit: PictureFit,
    #[serde(default = "half")]
    pub crop_x: f64,
    #[serde(default = "half")]
    pub crop_y: f64,
}

fn half() -> f64 {
    0.5
}

impl PicturePlacement {
    pub fn new(asset: String) -> Self {
        PicturePlacement {
            asset,
            fit: PictureFit::Fill,
            crop_x: 0.5,
            crop_y: 0.5,
        }
    }
}

/// The rectangle a picture actually occupies when shown in a
/// box of `box_w` x `box_h` (same units throughout, caller's
/// choice), as (x, y, w, h) relative to the box's top-left.
/// Fill covers the box and may extend past it (negative x/y);
/// the caller clips to the box and uses the crop fractions to
/// choose which part of the overflow shows. ShowWhole fits
/// inside, centered. One function, shared by the canvas, the
/// previews, the PDF writer, and the blur check, so they can
/// never disagree about where a picture sits.
pub fn picture_shown_rect(
    fit: PictureFit,
    pic_w: f64,
    pic_h: f64,
    box_w: f64,
    box_h: f64,
    crop_x: f64,
    crop_y: f64,
) -> (f64, f64, f64, f64) {
    if pic_w <= 0.0 || pic_h <= 0.0 || box_w <= 0.0 || box_h <= 0.0 {
        return (0.0, 0.0, box_w.max(0.0), box_h.max(0.0));
    }
    let scale = match fit {
        PictureFit::Fill => (box_w / pic_w).max(box_h / pic_h),
        PictureFit::ShowWhole => (box_w / pic_w).min(box_h / pic_h),
    };
    let w = pic_w * scale;
    let h = pic_h * scale;
    let (x, y) = match fit {
        PictureFit::Fill => (
            -(w - box_w) * crop_x.clamp(0.0, 1.0),
            -(h - box_h) * crop_y.clamp(0.0, 1.0),
        ),
        PictureFit::ShowWhole => ((box_w - w) / 2.0, (box_h - h) / 2.0),
    };
    (x, y, w, h)
}

/// The mark a music box carries (M1, docs/research/
/// music-landscape.md Phase A). A music box is a picture
/// box whose picture is one engraved line ("system") of a
/// MusicXML file, drawn by the engraver helper at a staff
/// size a layperson can sing from. The mark is what makes
/// the box music rather than a picture: the app lets it be
/// MOVED but never resized, because its size is the
/// legibility contract, and the Check and review words can
/// name where its notes came from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MusicFrame {
    /// Which system of the piece this box carries,
    /// counting from 0 in reading order.
    pub system: u32,
    /// The staff height the engraving measured, in
    /// millimetres. Never below the 5.0 mm floor: the
    /// helper refuses a render that measures under it.
    pub staff_mm: f64,
    /// The MusicXML asset (in `Document.assets`, kind
    /// Music) these notes were engraved from, by SHA-256.
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Frame {
    pub id: String,
    pub name: String,
    pub kind: FrameKind,
    pub rect: Rect,
    pub story_id: Option<String>,
    pub role: ContentRole,
    /// The picture in this box, when the kind is Picture and
    /// a picture has been put in it. An empty picture box has
    /// kind Picture and no placement. Serde-defaulted, so
    /// every older file loads with no picture.
    #[serde(default)]
    pub picture: Option<PicturePlacement>,
    /// The music mark, when this picture box carries one
    /// engraved system of a MusicXML file. Serde-defaulted,
    /// so every older file loads with no music.
    #[serde(default)]
    pub music: Option<MusicFrame>,
}

/// One layout guide on a panel: a straight line the eye (and later,
/// snapping) can line things up against. Positions are in the
/// panel's own milli-point coordinates, from its top-left corner,
/// exactly like frames. On screen they are shown and edited in
/// inches, the office worker's unit (docs/EDITOR.md).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Guide {
    /// true: a vertical line at x = pos. false: horizontal at y.
    pub vertical: bool,
    pub pos: Mpt,
    /// A locked guide cannot be dragged (its exact place can
    /// still be typed, and it can be unlocked from its menu).
    #[serde(default)]
    pub locked: bool,
}

/// Snap a value to the nearest target when it is within the
/// threshold: the quiet magnet that makes dragging feel precise.
/// Returns the target when snapped, else the value unchanged.
pub fn snap_to(value: Mpt, targets: &[Mpt], threshold: Mpt) -> Mpt {
    let mut best: Option<(Mpt, Mpt)> = None; // (target, distance)
    for &t in targets {
        let d = (value - t).abs();
        if d <= threshold && best.map(|(_, bd)| d < bd).unwrap_or(true) {
            best = Some((t, d));
        }
    }
    best.map(|(t, _)| t).unwrap_or(value)
}

/// Inches to milli-points (the editor speaks inches on screen;
/// the file stores milli-points).
pub fn inches_to_mpt(inches: f64) -> Mpt {
    (inches * 72_000.0).round() as Mpt
}

/// Milli-points to inches, for display.
pub fn mpt_to_inches(m: Mpt) -> f64 {
    m as f64 / 72_000.0
}

/// Read a position the way an office worker types it: "1.25",
/// "2", "1/2", or "1 1/4" (inches). None when it is not a number
/// we understand; the caller keeps the old value and says so.
pub fn parse_inches(text: &str) -> Option<f64> {
    let t = text.trim().trim_end_matches(['"', ' ']).trim();
    if t.is_empty() {
        return None;
    }
    let mut total = 0.0f64;
    for part in t.split_whitespace() {
        if let Some((num, den)) = part.split_once('/') {
            let n: f64 = num.parse().ok()?;
            let d: f64 = den.parse().ok()?;
            if d == 0.0 {
                return None;
            }
            total += n / d;
        } else {
            total += part.parse::<f64>().ok()?;
        }
    }
    if total.is_finite() && total >= 0.0 {
        Some(total)
    } else {
        None
    }
}

/// A calendar date as plain numbers. ob-model carries no
/// date library; the little calendar math a bulletin needs
/// (valid days, weekdays) is here and tested.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SimpleDate {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

impl SimpleDate {
    pub fn is_valid(&self) -> bool {
        (1..=12).contains(&self.month)
            && self.day >= 1
            && self.day <= days_in_month(self.year, self.month)
    }
    /// 0 = Sunday .. 6 = Saturday.
    pub fn weekday(&self) -> u32 {
        // Days since 1970-01-01 (a Thursday) in the proleptic
        // Gregorian calendar (Howard Hinnant's algorithm).
        let y = if self.month <= 2 {
            self.year - 1
        } else {
            self.year
        };
        let era = if y >= 0 { y } else { y - 399 } / 400;
        let yoe = (y - era * 400) as i64;
        let mp = ((self.month + 9) % 12) as i64;
        let doy = (153 * mp + 2) / 5 + self.day as i64 - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        let days = era as i64 * 146_097 + doe - 719_468;
        ((days % 7 + 7 + 4) % 7) as u32
    }
}

fn is_leap(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap(year) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

fn month_number(word: &str) -> Option<u32> {
    let w = word.trim_end_matches('.').to_ascii_lowercase();
    Some(match w.as_str() {
        "jan" | "january" => 1,
        "feb" | "february" => 2,
        "mar" | "march" => 3,
        "apr" | "april" => 4,
        "may" => 5,
        "jun" | "june" => 6,
        "jul" | "july" => 7,
        "aug" | "august" => 8,
        "sep" | "sept" | "september" => 9,
        "oct" | "october" => 10,
        "nov" | "november" => 11,
        "dec" | "december" => 12,
        _ => return None,
    })
}

fn weekday_number(word: &str) -> Option<u32> {
    let w = word.trim_end_matches('.').to_ascii_lowercase();
    Some(match w.as_str() {
        "sun" | "sunday" => 0,
        "mon" | "monday" => 1,
        "tue" | "tues" | "tuesday" => 2,
        "wed" | "wednesday" => 3,
        "thu" | "thur" | "thurs" | "thursday" => 4,
        "fri" | "friday" => 5,
        "sat" | "saturday" => 6,
        _ => return None,
    })
}

/// A day-of-month as typed: "18", or with its spoken suffix
/// still on ("18th"). Nothing else reads as a day.
fn day_number(word: &str) -> Option<u32> {
    let w = word.to_ascii_lowercase();
    let w = w
        .strip_suffix("st")
        .or_else(|| w.strip_suffix("nd"))
        .or_else(|| w.strip_suffix("rd"))
        .or_else(|| w.strip_suffix("th"))
        .unwrap_or(&w);
    w.parse::<u32>().ok().filter(|d| (1..=31).contains(d))
}

/// A year as typed: four digits, or two meaning 20xx (the
/// pivot a church office means: "26" is 2026, never 1926).
fn year_number(word: &str) -> Option<i32> {
    if word.len() == 4 {
        word.parse::<i32>().ok().filter(|y| *y >= 1)
    } else if word.len() == 2 {
        word.parse::<i32>().ok().map(|y| 2000 + y)
    } else {
        None
    }
}

/// Read a date the way an office worker writes it
/// (docs/planning/occasions.md): ISO (2026-10-18), US
/// numeric (10/18/2026, 10/18/26, 10-18-2026, also without
/// a year), month names (Oct 18, October 18 2026, 18 October
/// 2026), any of them with a weekday in front (Wed, Feb 18,
/// 2026). A stated weekday is CHECKED against the real one:
/// a date whose weekday disagrees does not read at all,
/// rather than being silently believed. A date with no year
/// means its coming occurrence: today, or the next time it
/// falls. Anything else returns None; callers refuse in
/// plain words with an example, never guess.
pub fn parse_flexible_date(text: &str, today: SimpleDate) -> Option<SimpleDate> {
    let cleaned = text.trim().replace(',', " ");
    if cleaned.is_empty() {
        return None;
    }
    // Split the input into a numeric-slash/dash candidate and
    // word tokens; a bare numeric token with separators is
    // read whole, anything else token by token.
    let mut weekday: Option<u32> = None;
    let mut tokens: Vec<String> = Vec::new();
    for raw in cleaned.split_whitespace() {
        if weekday.is_none() && tokens.is_empty() {
            if let Some(w) = weekday_number(raw) {
                weekday = Some(w);
                continue;
            }
        }
        tokens.push(raw.to_string());
    }
    if tokens.is_empty() {
        return None;
    }
    // Finish the date: check a stated weekday, apply the
    // coming-occurrence rule to a missing year.
    let finish = |month: u32, day: u32, year: Option<i32>| -> Option<SimpleDate> {
        let date = match year {
            Some(y) => {
                let d = SimpleDate {
                    year: y,
                    month,
                    day,
                };
                if !d.is_valid() {
                    return None;
                }
                d
            }
            None => {
                // The coming occurrence: this year when the
                // day is still ahead (or today), else the
                // next year it falls (Feb 29 may skip). The
                // search stops at the year 9999, the top of
                // the supported calendar (audit Batch 3).
                let mut found = None;
                for y in today.year..=(today.year + 4).min(9999) {
                    let d = SimpleDate {
                        year: y,
                        month,
                        day,
                    };
                    if d.is_valid() && d >= today {
                        found = Some(d);
                        break;
                    }
                }
                found?
            }
        };
        if let Some(w) = weekday {
            if date.weekday() != w {
                return None;
            }
        }
        Some(date)
    };
    // One numeric token with separators: ISO, or US order.
    if tokens.len() == 1 {
        let t = &tokens[0];
        let sep = if t.contains('/') {
            '/'
        } else if t.contains('-') {
            '-'
        } else {
            return None;
        };
        let parts: Vec<&str> = t.split(sep).collect();
        let nums: Vec<i64> = parts
            .iter()
            .map(|p| p.parse::<i64>().ok())
            .collect::<Option<Vec<_>>>()?;
        return match nums.len() {
            3 => {
                if parts[0].len() == 4 {
                    // ISO: year first.
                    finish(
                        u32::try_from(nums[1]).ok()?,
                        u32::try_from(nums[2]).ok()?,
                        year_number(parts[0]),
                    )
                } else {
                    finish(
                        u32::try_from(nums[0]).ok()?,
                        u32::try_from(nums[1]).ok()?,
                        year_number(parts[2]),
                    )
                }
            }
            2 => finish(
                u32::try_from(nums[0]).ok()?,
                u32::try_from(nums[1]).ok()?,
                None,
            ),
            _ => None,
        };
    }
    // Words: a month name, a day, maybe a year, in either
    // order (Oct 18 2026 / 18 October 2026).
    let mut month: Option<u32> = None;
    let mut day: Option<u32> = None;
    let mut year: Option<i32> = None;
    for tok in &tokens {
        if month.is_none() {
            if let Some(m) = month_number(tok) {
                month = Some(m);
                continue;
            }
        }
        if year.is_none() && tok.chars().all(|c| c.is_ascii_digit()) {
            // A four-digit number is a year outright; a
            // two-digit one is a year only once the day is
            // spoken for, so "Oct 18" reads 18 as the day
            // and "18 Oct 26" reads 26 as the year.
            if tok.len() == 4 || (tok.len() == 2 && day.is_some()) {
                if let Some(y) = year_number(tok) {
                    year = Some(y);
                    continue;
                }
            }
        }
        if day.is_none() {
            if let Some(d) = day_number(tok) {
                day = Some(d);
                continue;
            }
        }
        return None;
    }
    finish(month?, day?, year)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Panel {
    pub id: String,
    pub name: String,
    pub size: Size,
    pub frames: Vec<Frame>,
    /// Layout guides on this panel (docs/EDITOR.md).
    #[serde(default)]
    pub guides: Vec<Guide>,
    /// Optional tracing-guide picture under this panel (the
    /// page of the old bulletin this panel rebuilds, from the
    /// trace rescue in docs/EXTRACT.md). A guide is shown
    /// under the text while editing and never prints. In a v1
    /// document this names the picture's asset by SHA-256 and
    /// the bytes ride inside the file; a value that is not an
    /// asset hash is an old sidecar path (v0 files, or a draft
    /// whose pictures have not been gathered in yet), resolved
    /// beside the bulletin file. Loading migrates a resolvable
    /// path into the asset store; an unresolvable one stays a
    /// path, and the app says the picture is missing, exactly
    /// as it always has.
    #[serde(default)]
    pub background_image: Option<String>,
}

/// How this church prints children's names in the
/// bulletin, chosen once in the Church Profile (roles
/// build R-2b). The app does not enforce it: it cannot
/// tell which names are children's. It repeats the rule
/// where the words are typed and again at Check, as a
/// reminder to look.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ChildrenNames {
    /// Full names, as the church has always printed them.
    #[default]
    FullNames,
    /// First names only.
    FirstNamesOnly,
    /// No children's names in the bulletin.
    NoNames,
}

impl ChildrenNames {
    /// The rule in the words the Check reminder speaks.
    pub fn rule_words(&self) -> &'static str {
        match self {
            ChildrenNames::FullNames => "children's full names",
            ChildrenNames::FirstNamesOnly => "children's first names only",
            ChildrenNames::NoNames => "no children's names",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChurchProfile {
    pub name: String,
    pub address: String,
    pub service_times: String,
    pub website: String,
    pub giving_url: String,
    pub pastor: String,
    /// The parts this church usually gets for the bulletin
    /// ("Pastor", "Music director", "Sunday school"), set
    /// once in the Church Profile area. Start's "still
    /// waiting" list is these names with no arrived
    /// contribution yet (roles build R-1). Serde-defaulted:
    /// profiles written before the field expect nothing.
    #[serde(default)]
    pub expected_parts: Vec<String>,
    /// How this church prints children's names (roles
    /// build R-2b). Serde-defaulted: profiles written
    /// before the choice print full names, as they did.
    #[serde(default)]
    pub children_names: ChildrenNames,
    /// The serving lines this church prints each week
    /// ("Readers", "Acolytes", and the church's own), in
    /// the order This Sunday shows them (roles build
    /// R-2b). The starting set is the six built-in lines;
    /// the church renames, adds, and removes its own.
    /// Serde-defaulted to that starting set: a profile
    /// written before the list existed serves the six.
    #[serde(default = "default_serving_lines")]
    pub serving_lines: Vec<String>,
    /// The serving-line rename aliases (round 6,
    /// R6-C): when a profile save pairs a rename, the
    /// pairing is recorded here durably, from the OLD
    /// line's slug (its built-in canonical slug when
    /// the old name is a built-in line, else the slug
    /// its name derives) to the NEW line's slug.
    /// Token resolution follows these aliases (chains
    /// included) instead of guessing a line by its
    /// position in the list. Serde-defaulted empty:
    /// profiles written before the record existed
    /// load with no aliases.
    #[serde(default)]
    pub serving_aliases: BTreeMap<String, String>,
}

impl Default for ChurchProfile {
    fn default() -> Self {
        Self {
            name: String::new(),
            address: String::new(),
            service_times: String::new(),
            website: String::new(),
            giving_url: String::new(),
            pastor: String::new(),
            expected_parts: Vec::new(),
            children_names: ChildrenNames::FullNames,
            serving_lines: default_serving_lines(),
            serving_aliases: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SheetSpec {
    /// Preset id, e.g. "letter-bifold". Imposition dispatches on this.
    pub preset: String,
    pub size: Size,
}

pub const FORMAT_VERSION: u32 = 1;

/// The SHA-256 of some bytes, lowercase hex: the name every
/// asset travels under in the v1 package (docs/FORMAT.md).
pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = sha2::Sha256::digest(bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

/// What an asset in the document's store is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetKind {
    /// A picture placed (or waiting to be placed) in the
    /// bulletin: cover art, a photo, clipart.
    Picture,
    /// A tracing-guide picture under a panel, from an import
    /// rescue. Shown while editing, never printed.
    TracingPicture,
    /// A music file carried with the bulletin (MusicXML and
    /// friends; the music build gives it frames).
    Music,
    Other,
}

/// The license an asset's rights record claims, in the
/// vocabulary of docs/research/clipart-landscape.md section 5.
/// The record is a claim the office asserts, never a proof:
/// the app records, composes credit lines, and reminds.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum LicenseKind {
    PublicDomain,
    Cc0,
    /// Creative Commons Attribution, with its version ("4.0").
    CcBy(String),
    /// Creative Commons Attribution-ShareAlike, with version.
    CcBySa(String),
    /// The church's own work, or a work it holds outright.
    ChurchOwned,
    /// Reprinted under a church license (CCLI, One License).
    LicensedReprint,
    #[serde(other)]
    #[default]
    Unknown,
}

/// Where an asset came from and what credit it is owed
/// (clipart-landscape.md section 5). Every field defaults, so
/// a partial record loads and simply says less.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RightsRecord {
    /// Where it came from, in words ("Openclipart", "the
    /// church camera", the imported file it rode in on).
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub source_url: Option<String>,
    #[serde(default)]
    pub license: LicenseKind,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub creator: String,
    /// The copyright line as it should print, when one is
    /// owed ("\u{00a9} 2019 Jane Doe").
    #[serde(default)]
    pub copyright_line: String,
    /// The exact credit text the license requires, when it
    /// requires one.
    #[serde(default)]
    pub credit_text: String,
    /// The licensor's reference: a CCLI or One License
    /// number, a catalog id, a permission letter's date.
    #[serde(default)]
    pub licensor_ref: String,
    /// Who in the office asserted this record.
    #[serde(default)]
    pub asserted_by: String,
    /// When they asserted it (ISO date).
    #[serde(default)]
    pub asserted_at: String,
    /// Are there children in this picture, as far as the
    /// office knows (roles build R-2b)? None means nobody
    /// has said yet, and Check asks. Some(true) puts the
    /// picture under the web page's stricter rule: it
    /// rides the web page only when a permission is
    /// noted on this record. Print never drops a picture
    /// over this. Like the whole record, it is the
    /// office's own note, not a proof.
    #[serde(default)]
    pub children_present: Option<bool>,
}

/// One asset in the document's store. Its bytes live in
/// `Document::asset_bytes` under the same hash and travel as
/// the zip entry `assets/<sha256>` in a v1 file; the metadata
/// (including the rights record) rides in document.json.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Asset {
    pub sha256: String,
    pub kind: AssetKind,
    /// IANA media type, e.g. "image/png".
    pub media_type: String,
    #[serde(default)]
    pub rights: RightsRecord,
}

/// Is a permission noted on this record, for the web
/// page's children rule? This is the same test the HTML
/// export applies before a picture with children noted
/// in it may ride the web page
/// (crates/ob-extract/src/html_export.rs,
/// `permission_noted`, whose copy points back here):
/// a credit line counts, and so does a license that
/// needs no one's permission (the church's own work,
/// public domain, CC0). The two copies are one rule
/// and must stay one rule: the export holds pictures
/// back by it, and the merge below enforces its
/// no-manufactured-permission property by it (round 5,
/// F2; round 6, R6-1).
fn rights_permission_noted(rights: &RightsRecord) -> bool {
    !rights.credit_text.trim().is_empty()
        || matches!(
            rights.license,
            LicenseKind::ChurchOwned | LicenseKind::PublicDomain | LicenseKind::Cc0
        )
}

/// Merge two records for the same picture, field by
/// field (round 4, R4-5): the incoming record fills
/// only what the existing record leaves empty, and a
/// value already noted is never overwritten, so a
/// split-strength pair (the bulletin holds the credit
/// line, the sheet holds the license) ends up carrying
/// both. The children note merges as OR: a "children
/// are in this picture" from either side stands,
/// because the web page's hold-back is only as strong
/// as the strongest note.
///
/// No merge manufactures a permission, in any order
/// (round 5, F2, as a directional fence; round 6,
/// R6-1, as a property of the composed record). After
/// the fills, if the composed record notes children
/// and notes a permission, that permission must be
/// vouched for: a permission-satisfying field (the
/// credit line; a license that itself counts as a
/// noted permission, the church's own work, public
/// domain, CC0) stands only when the record it came
/// from itself notes children, or a children-bearing
/// record already noted a permission on its own
/// fields before this merge. Otherwise the field
/// reverts to the children-bearing record's value for
/// that field. The fill-only merge knows which side
/// supplied each field, so the check is on provenance,
/// not on which record happened to stand first, and
/// a fold of three or more records holds by
/// composition: every single merge leaves the
/// invariant standing. (The one step a fold defers
/// to its own end is the final judgment below —
/// round 8, R8-9, at
/// `fold_duplicate_asset_records`.)
///
/// A license and its credit travel together (round 6,
/// RT6-2): when a merge takes the incoming record's
/// license but the composed record ends up holding
/// that license with an empty credit line, although
/// the incoming record carried a credit line that did
/// not stand (refused above, or otherwise not
/// filled), the license does not fill either. No
/// merge leaves a record claiming a license whose
/// required attribution was dropped in the same merge.
///
/// One final judgment judges the composed record,
/// not the direction (round 7, R7-1, as the
/// togetherness rule; round 9, R9-1, as the only
/// judgment): after all the fills and reversions
/// above, if the composed record notes children and
/// its credit line is empty and it carries a
/// license, that license stands only if vouched —
/// some record in the composition held that same
/// license together with a children note, or
/// together with the credit that stands in the
/// composed record, or a children-bearing record in
/// the composition noted a permission under the
/// fence's bearer rule. This holds for every license
/// class, a reprint license included, and whichever
/// path composed the record: the license filled,
/// already stood, or would stand as a fallback. An
/// attribution license (CC BY, CC BY-SA) is never
/// vouched on an empty credit line. Unvouched, a
/// license falls to another license in the
/// composition that is vouched under this same
/// judgment — chosen order-independently, a license
/// carried by a children-bearing record first, then
/// by a fixed precedence (the church's own work,
/// public domain, CC0, a reprint license) — and to
/// Unknown where nothing vouches a license. A
/// children-bearing record whose own credit line
/// stands keeps its license.
///
/// The assertion fields (asserted_by, asserted_at)
/// fill only when a substantive rights fill from the
/// same incoming record stands in the composed
/// record (round 5; round 7, R7-8): a credit line, or
/// a license that itself stands. An assertion vouches
/// for a rights fill and never rides in alone, on an
/// unrelated fill (a title, a source URL), or on a
/// fill refused above. The caller compares the result
/// with the existing record to know whether anything
/// was filled in.
pub fn merge_rights_records(existing: &RightsRecord, incoming: &RightsRecord) -> RightsRecord {
    merge_rights_records_inner(existing, incoming, false)
}

/// `merge_rights_records`, with the final judgment
/// optionally deferred (round 8, R8-9): a fold defers
/// that one step — the license fall for an empty
/// credit line — so a credit arriving later in the
/// same fold can stand beside the license it belongs
/// to, and judges it once, on the folded record, at
/// the fold's end (see `fold_duplicate_asset_records`).
/// Every other step — the fills, the fence reversions,
/// the incoming license-and-credit pairing, the
/// assertion rule — runs per merge exactly as in a
/// single merge.
fn merge_rights_records_inner(
    existing: &RightsRecord,
    incoming: &RightsRecord,
    defer_composed_rule: bool,
) -> RightsRecord {
    fn fill(existing: &str, incoming: &str) -> String {
        if existing.trim().is_empty() && !incoming.trim().is_empty() {
            incoming.to_string()
        } else {
            existing.to_string()
        }
    }
    // Provenance for the two permission-satisfying
    // fields: which side supplied the composed value.
    let credit_from_incoming =
        existing.credit_text.trim().is_empty() && !incoming.credit_text.trim().is_empty();
    let license_from_incoming =
        existing.license == LicenseKind::Unknown && incoming.license != LicenseKind::Unknown;
    let mut out = existing.clone();
    out.source = fill(&existing.source, &incoming.source);
    if existing.source_url.is_none() {
        out.source_url = incoming.source_url.clone();
    }
    if existing.license == LicenseKind::Unknown {
        out.license = incoming.license.clone();
    }
    out.title = fill(&existing.title, &incoming.title);
    out.creator = fill(&existing.creator, &incoming.creator);
    out.copyright_line = fill(&existing.copyright_line, &incoming.copyright_line);
    out.credit_text = fill(&existing.credit_text, &incoming.credit_text);
    out.licensor_ref = fill(&existing.licensor_ref, &incoming.licensor_ref);
    let children_present = match (existing.children_present, incoming.children_present) {
        (Some(true), _) | (_, Some(true)) => Some(true),
        (Some(false), _) | (_, Some(false)) => Some(false),
        _ => None,
    };
    if children_present == Some(true) {
        // The record (or records) bearing the children
        // note, and whether any of them already noted a
        // permission on its own fields before this merge.
        let bearer = if existing.children_present == Some(true) {
            existing
        } else {
            incoming
        };
        let bearer_noted_permission = [existing, incoming]
            .iter()
            .any(|r| r.children_present == Some(true) && rights_permission_noted(r));
        if !out.credit_text.trim().is_empty() {
            let credit_source = if credit_from_incoming {
                incoming
            } else {
                existing
            };
            if credit_source.children_present != Some(true) && !bearer_noted_permission {
                let reverted_credit = bearer.credit_text.clone();
                out.credit_text = reverted_credit;
            }
        }
        if matches!(
            out.license,
            LicenseKind::ChurchOwned | LicenseKind::PublicDomain | LicenseKind::Cc0
        ) {
            let license_source = if license_from_incoming {
                incoming
            } else {
                existing
            };
            if license_source.children_present != Some(true) && !bearer_noted_permission {
                let reverted_license = bearer.license.clone();
                out.license = reverted_license;
            }
        }
    }
    // A license and its credit travel together: the
    // incoming license does not stand over an empty
    // credit line when the incoming record's own credit
    // line did not stand in this merge.
    if license_from_incoming
        && out.license == incoming.license
        && !incoming.credit_text.trim().is_empty()
        && out.credit_text.trim().is_empty()
    {
        let standing_license = existing.license.clone();
        out.license = standing_license;
    }
    // The final judgment on the composed record
    // (round 9, R9-1): ONE judgment, on every license
    // class, whatever path composed the record (the
    // license filled, already stood, or would stand
    // as a fallback). It subsumes the round-7
    // togetherness rule (an attribution license does
    // not stand beside a children note with an empty
    // credit line) and the round-8 uniform fallback
    // condition. The judgment itself lives at
    // `judged_composed_license`, shared with the
    // fold's end (where it is deferred to, so a credit
    // arriving later in the same fold can stand
    // beside the license it belongs to).
    if !defer_composed_rule {
        let sources: [&RightsRecord; 2] = [existing, incoming];
        let judged =
            judged_composed_license(&out.license, children_present, &out.credit_text, &sources);
        out.license = judged;
    }
    out.children_present = children_present;
    // The assertion stands only with a substantive
    // rights fill from the same incoming record that
    // itself stands in the composed record (round 7,
    // R7-8): its credit line, or its license.
    let credit_stands_from_incoming =
        credit_from_incoming && out.credit_text == incoming.credit_text;
    let license_stands_from_incoming = license_from_incoming && out.license == incoming.license;
    if credit_stands_from_incoming || license_stands_from_incoming {
        out.asserted_by = fill(&existing.asserted_by, &incoming.asserted_by);
        out.asserted_at = fill(&existing.asserted_at, &incoming.asserted_at);
    }
    out
}

/// Is a license vouched, under the final judgment
/// (round 9, R9-1)? A license in a composition that
/// notes children, with the credit line that stands
/// in the composed record, is vouched when some
/// record in the composition held that same license
/// together with a children note, or together with
/// the credit that stands in the composed record,
/// or when a children-bearing record in the
/// composition noted a permission on its own fields
/// (the fence's bearer rule: its credit line, or a
/// license that itself counts as a noted permission).
/// An attribution license (CC BY, CC BY-SA) is never
/// vouched on an empty credit line: its credit is
/// the license's own requirement, so with no credit
/// standing it falls whatever else vouches it.
fn license_vouched(
    license: &LicenseKind,
    standing_credit: &str,
    sources: &[&RightsRecord],
) -> bool {
    if matches!(license, LicenseKind::CcBy(_) | LicenseKind::CcBySa(_))
        && standing_credit.trim().is_empty()
    {
        return false;
    }
    let held_with_children = sources
        .iter()
        .any(|r| r.license == *license && r.children_present == Some(true));
    if held_with_children {
        return true;
    }
    let held_with_standing_credit = !standing_credit.trim().is_empty()
        && sources
            .iter()
            .any(|r| r.license == *license && r.credit_text == standing_credit);
    if held_with_standing_credit {
        return true;
    }
    sources
        .iter()
        .any(|r| r.children_present == Some(true) && rights_permission_noted(r))
}

/// The fixed precedence among fallback licenses
/// (round 9, R9-4), strongest claim first: the
/// church's own work, then public domain, then CC0,
/// then a reprint license (CCLI, One License), which
/// rests on an outside licensor and stands last.
fn fallback_rank(license: &LicenseKind) -> u8 {
    match license {
        LicenseKind::ChurchOwned => 0,
        LicenseKind::PublicDomain => 1,
        LicenseKind::Cc0 => 2,
        LicenseKind::LicensedReprint => 3,
        _ => 4,
    }
}

/// Pick the fallback license for a composed record
/// whose standing license falls (round 9, R9-1 and
/// R9-4): a license one of the composition's records
/// carried, vouched under the same judgment as a
/// standing license. Selection is order-independent:
/// a vouched license carried by a children-bearing
/// source record beats one vouched only through the
/// bearer rule, and among equals the fixed precedence
/// at `fallback_rank` decides, never the order the
/// records arrived in.
fn fallback_license(standing_credit: &str, sources: &[&RightsRecord]) -> Option<LicenseKind> {
    let mut best: Option<(u8, u8, LicenseKind)> = None;
    for source in sources {
        let candidate = &source.license;
        if matches!(
            candidate,
            LicenseKind::Unknown | LicenseKind::CcBy(_) | LicenseKind::CcBySa(_)
        ) {
            continue;
        }
        if best.as_ref().is_some_and(|(_, _, held)| held == candidate) {
            continue;
        }
        if !license_vouched(candidate, standing_credit, sources) {
            continue;
        }
        let carried_by_children_bearer = sources
            .iter()
            .any(|r| r.license == *candidate && r.children_present == Some(true));
        let tier: u8 = if carried_by_children_bearer { 0 } else { 1 };
        let rank = fallback_rank(candidate);
        let is_better = match &best {
            None => true,
            Some((held_tier, held_rank, _)) => (tier, rank) < (*held_tier, *held_rank),
        };
        if is_better {
            let chosen = candidate.clone();
            best = Some((tier, rank, chosen));
        }
    }
    best.map(|(_, _, license)| license)
}

/// The ONE final judgment on a composed record
/// (round 9, R9-1): if the composed record notes
/// children and its credit line is empty and it
/// carries a license, that license stands only if
/// vouched (see `license_vouched`). Unvouched, it
/// falls — to another license in the composition
/// that is vouched under this same judgment (see
/// `fallback_license`), else Unknown. Every other
/// composed record (no children note, a credit line
/// standing, or no license) is left exactly as it
/// stands: the publish fence above has already
/// judged the permission-satisfying fields on their
/// own provenance, and a standing credit vouches
/// through the bearer rule by construction.
fn judged_composed_license(
    license: &LicenseKind,
    children_present: Option<bool>,
    credit_text: &str,
    sources: &[&RightsRecord],
) -> LicenseKind {
    if children_present != Some(true) {
        return license.clone();
    }
    if !credit_text.trim().is_empty() {
        return license.clone();
    }
    if *license == LicenseKind::Unknown {
        return license.clone();
    }
    if license_vouched(license, credit_text, sources) {
        return license.clone();
    }
    fallback_license(credit_text, sources).unwrap_or(LicenseKind::Unknown)
}

/// Re-judge a folded record's assertion against the
/// FINAL folded record (round 9, R9-5), by the R7-8
/// rule a single merge applies: the assertion stands
/// only if a substantive rights fill from its own
/// record stands there — that record's credit line,
/// or that record's license. In a fold the per-merge
/// assertion rule runs while a license still stands
/// under deferral; when the fold-end judgment drops
/// that license, the assertion that rode in on it
/// does not stay behind.
fn rejudge_folded_assertion(folded: &mut RightsRecord, sources: &[RightsRecord]) {
    if folded.asserted_by.trim().is_empty() && folded.asserted_at.trim().is_empty() {
        return;
    }
    let supplier_stands = sources.iter().any(|source| {
        let is_supplier = (!folded.asserted_by.trim().is_empty()
            && source.asserted_by == folded.asserted_by)
            || (!folded.asserted_at.trim().is_empty() && source.asserted_at == folded.asserted_at);
        if !is_supplier {
            return false;
        }
        let credit_stands =
            !source.credit_text.trim().is_empty() && folded.credit_text == source.credit_text;
        let license_stands =
            source.license != LicenseKind::Unknown && folded.license == source.license;
        credit_stands || license_stands
    });
    if !supplier_stands {
        folded.asserted_by = String::new();
        folded.asserted_at = String::new();
    }
}

/// Judge the final judgment once, on a folded
/// record (round 8, R8-9; round 9, R9-1): the same
/// `judged_composed_license` a single merge applies,
/// run over the fold's own source records, and then
/// the assertion re-judged against what finally
/// stands (round 9, R9-5).
fn judge_folded_composed_record(folded: &mut RightsRecord, sources: &[RightsRecord]) {
    let source_refs: Vec<&RightsRecord> = sources.iter().collect();
    let judged = judged_composed_license(
        &folded.license,
        folded.children_present,
        &folded.credit_text,
        &source_refs,
    );
    folded.license = judged;
    rejudge_folded_assertion(folded, sources);
}

/// Fold records that share a hash into one record
/// apiece (round 5, RT5-1): a file may carry two
/// records for the same picture bytes, and every
/// reader looks a picture up by its hash and takes
/// the record it finds, so a weak duplicate standing
/// first silently shadowed the fuller one: a children
/// note disarmed, a license dropped. The loaders fold
/// on load, before any reader sees the records.
/// Duplicates merge field by field with
/// `merge_rights_records`; the first record's kind
/// and media type stand (the same bytes carry the
/// same kind). After the fold, one hash has exactly
/// one record, here and in every copy saved onward.
/// The merges run with the final judgment deferred,
/// and that judgment is run once, on the folded
/// record, at the end (round 8, R8-9; round 9, R9-1),
/// with the assertion then re-judged against what
/// finally stands (round 9, R9-5).
pub fn fold_duplicate_asset_records(assets: &mut Vec<Asset>) {
    let mut folded: Vec<Asset> = Vec::with_capacity(assets.len());
    // The records each folded record was composed
    // from, in fold order: the fold's final judging
    // (below) reads them for its fallback.
    let mut folded_sources: Vec<Vec<RightsRecord>> = Vec::with_capacity(assets.len());
    for asset in assets.drain(..) {
        match folded.iter().position(|a| a.sha256 == asset.sha256) {
            Some(pos) => {
                folded_sources[pos].push(asset.rights.clone());
                let merged = merge_rights_records_inner(&folded[pos].rights, &asset.rights, true);
                folded[pos].rights = merged;
            }
            None => {
                folded_sources.push(vec![asset.rights.clone()]);
                folded.push(asset);
            }
        }
    }
    // The final judgment is judged once, here, on
    // each folded record that a merge actually
    // composed (round 8, R8-9; round 9, R9-1): a
    // record no duplicate folded into is left exactly
    // as it stood, as before.
    for (asset, sources) in folded.iter_mut().zip(folded_sources.iter()) {
        if sources.len() > 1 {
            judge_folded_composed_record(&mut asset.rights, sources);
        }
    }
    *assets = folded;
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Document {
    pub format_version: u32,
    pub title: String,
    pub profile: ChurchProfile,
    /// Field values: occasion_date (ISO), occasion_date_long, occasion_name, reading.
    pub fields: BTreeMap<String, String>,
    pub styles: Vec<ParagraphStyle>,
    pub stories: Vec<Story>,
    pub panels: Vec<Panel>,
    pub sheet: SheetSpec,
    /// The office has said, out loud, that there are no
    /// announcements this week. Without this, an empty
    /// announcements box cannot be told apart from one nobody
    /// filled in yet. Reset by Next Sunday: each week answers
    /// for itself. Serde-defaulted for older files.
    #[serde(default)]
    pub no_announcements: bool,
    /// The asset table: pictures, tracing pictures, and music
    /// the bulletin carries (v1). Bytes live in `asset_bytes`.
    #[serde(default)]
    pub assets: Vec<Asset>,
    /// Asset bytes by SHA-256. Never serialized into
    /// document.json: in a v1 file they are the zip entries
    /// `assets/<sha256>`, and ob-format joins the two halves
    /// on load and splits them on save.
    #[serde(skip)]
    pub asset_bytes: BTreeMap<String, Vec<u8>>,
    /// When the service starts, "9:00 AM" (docs/planning/
    /// occasions.md): one day can carry several services, and
    /// the time is part of which occasion this document is.
    #[serde(default)]
    pub occasion_time: Option<String>,
    /// Fonts this document was made in that the Font Library
    /// did not hold when it was imported (docs/FONTS.md,
    /// wanted fonts): the real name, kept on the document,
    /// and the free font standing in for it at import time.
    /// The styles keep the real family name too, and the
    /// stand-in is chosen fresh at every shaping and print
    /// from the library as it stands then, so adding the
    /// real font later upgrades the document by itself.
    #[serde(default)]
    pub wanted_fonts: Vec<WantedFont>,
    /// This week's serving names, by serving-line slug
    /// (roles build R-2b): who reads, who ushers, who
    /// gives the children's message. Values are stored
    /// under the slug of the line's name AS SHOWN on
    /// This Sunday; token resolution also honors the
    /// built-in slugs, so a template's `{Sunday.acolytes}`
    /// keeps printing after the church renames the line.
    /// Before this map, the first four lines lived in
    /// `fields` under their slugs; loading folds those
    /// in (see `migrate_participants`). Serde-defaulted:
    /// files written before the map hold their names in
    /// `fields` until the fold.
    #[serde(default)]
    pub participants: BTreeMap<String, String>,
    /// The loose sheets riding inside this bulletin
    /// (roles build R-3): each is its own small document
    /// with its own panels, paper, and life, printed on
    /// its own sheets and never imposed into the
    /// bulletin's fold. Serde-defaulted: every file
    /// written before loose sheets existed loads with
    /// none, and `format_version` stays 1.
    #[serde(default)]
    pub inserts: Vec<Insert>,
    /// Content fingerprints of contributions already
    /// applied to THIS document (round 4, R4-4/R4-6):
    /// the record lives in the document, not in the
    /// app's session, so it survives a failed save, a
    /// manual save, and a crash the journal recovers
    /// from, and Undo takes it away with the words it
    /// describes. Next Sunday clears it: the new week
    /// starts unapplied. Bookkeeping only: it is never
    /// printed and no export door reads it. Serde-
    /// defaulted, and `format_version` stays 1.
    #[serde(default)]
    pub applied_contributions: Vec<String>,
}

/// One font an imported document names that the library did
/// not hold (docs/FONTS.md): the name as the source file
/// stated it, in the form a person would search for, and
/// the free family that stood in for it when the record
/// was made. The record is information, never a lock: what
/// actually prints is resolved against the library at use
/// time, so `substituted_by` can go stale on purpose (the
/// real font arrived) without the record lying about where
/// the document came from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WantedFont {
    /// The real font's name, e.g. "Gill Sans MT".
    pub name: String,
    /// The free family standing in for it at import time,
    /// e.g. "Cabin" (a lookalike) or "Liberation Sans".
    pub substituted_by: String,
}

/// How long a loose sheet rides in the bulletin (roles
/// build R-3). The office picks one per sheet; Next
/// Sunday acts on it and says what it did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum InsertLife {
    /// Rides every week until someone takes it out: the
    /// connection card, the giving card.
    Standing,
    /// Rides until its keep-through date passes, then
    /// Next Sunday takes it out and names it, the way an
    /// announcement ends.
    Dated,
    /// This week only: Next Sunday takes it out and
    /// names it. The default for a new sheet.
    #[default]
    ThisWeekOnly,
}

/// A loose sheet's own paper facts: its shape, how many
/// copies the office means to print, and the paper in
/// plain words. This is its own small record rather than
/// the bulletin's `SheetSpec`, because a sheet's shape
/// key names sheet shapes ("half-letter", "letter",
/// "card"), not bulletin folds, and because the copies
/// and paper are print-morning memory the bulletin
/// itself does not carry: the app remembers the
/// intention and says it on the Print step; it does not
/// dial the copier.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InsertSheet {
    /// Sheet shape key: "half-letter", "letter", or
    /// "card" (see [`insert_sheet_size`]).
    pub preset: String,
    /// Copies to print. Zero means "as many as the
    /// bulletin".
    #[serde(default)]
    pub copies: usize,
    /// The paper in plain words ("blue paper", "card
    /// stock"). Empty means plain paper.
    #[serde(default)]
    pub paper_note: String,
}

impl Default for InsertSheet {
    fn default() -> Self {
        InsertSheet {
            preset: "half-letter".to_string(),
            copies: 0,
            paper_note: String::new(),
        }
    }
}

/// The size a sheet shape key names, in milli-points:
/// half letter is the folded bulletin's own size, letter
/// is a full sheet, card is quarter letter (the pew
/// card). None for a key nobody knows; callers fall back
/// to the sheet's first panel.
pub fn insert_sheet_size(preset: &str) -> Option<Size> {
    // Milli-points: 72,000 to the inch.
    Some(match preset {
        "half-letter" => Size::new(396_000, 612_000), // 5.5 x 8.5
        "letter" => Size::new(612_000, 792_000),      // 8.5 x 11
        "card" => Size::new(306_000, 396_000),        // 4.25 x 5.5
        _ => return None,
    })
}

/// The sheet shapes the office can pick, as (key, words)
/// in the order the Loose sheets window lists them.
pub const INSERT_SHEET_SHAPES: [(&str, &str); 3] = [
    ("half-letter", "Half letter (5.5 x 8.5)"),
    ("letter", "Letter (8.5 x 11)"),
    ("card", "Card (4.25 x 5.5)"),
];

/// A loose sheet that travels inside a bulletin but is
/// not one of its pages (roles build R-3): its own
/// panels, its own words and styles, its own paper, its
/// own life. It prints on its own sheets, in the same
/// print job, never imposed into the bulletin's fold.
///
/// Its pictures are the host's: a picture frame in one
/// of its panels names an asset in the HOST document's
/// store by SHA-256, so a photo used by both the
/// bulletin and a sheet is stored once, under the one
/// rights record. Its words resolve their field tokens
/// against the host at print time (see `as_document`):
/// `{Sunday.date}` on an invitation card is the host's
/// occasion date, so a standing sheet never carries a
/// stale date, only the token.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Insert {
    pub id: String,
    /// The name the office calls it: "Prayer list".
    pub name: String,
    #[serde(default)]
    pub life: InsertLife,
    /// For `InsertLife::Dated`: the last day this sheet
    /// rides, with the same meaning as a story's
    /// keep-through date. None on a dated sheet means
    /// its date is not set, and Next Sunday treats it
    /// as passed.
    #[serde(default)]
    pub keep_through: Option<SimpleDate>,
    #[serde(default)]
    pub panels: Vec<Panel>,
    #[serde(default)]
    pub stories: Vec<Story>,
    #[serde(default)]
    pub styles: Vec<ParagraphStyle>,
    #[serde(default)]
    pub sheet: InsertSheet,
}

impl Insert {
    /// This sheet as a document an exporter can print:
    /// the sheet's own panels, stories, and styles on
    /// the host's resolution context (its church
    /// profile, occasion fields, serving names, wanted
    /// fonts, and asset store ride along by the clone),
    /// so field tokens in the sheet's words print the
    /// host's values and its pictures find their bytes.
    /// The synthesized document carries no loose sheets
    /// of its own. Spoken-only words inside the sheet
    /// are dropped by the export funnel itself, exactly
    /// as the bulletin's are: one rule, no door can
    /// forget it.
    pub fn as_document(&self, host: &Document) -> Document {
        let mut doc = host.clone();
        doc.panels = self.panels.clone();
        doc.stories = self.stories.clone();
        doc.styles = self.styles.clone();
        doc.sheet = SheetSpec {
            preset: self.sheet.preset.clone(),
            size: insert_sheet_size(&self.sheet.preset)
                .or_else(|| self.panels.first().map(|p| p.size))
                .unwrap_or_else(|| Size::new(612_000, 792_000)),
        };
        doc.inserts = Vec::new();
        doc.no_announcements = false;
        doc
    }

    /// Whether the sheet's stories carry any words at
    /// all. An empty sheet is the Check step's business:
    /// it would print as a blank sheet of colored paper.
    pub fn has_words(&self) -> bool {
        self.stories
            .iter()
            .flat_map(|s| s.paragraphs.iter())
            .any(|p| !p.text().trim().is_empty())
    }

    /// Clamp this sheet's numbers to the same sane
    /// bounds [`Document::sanitized`] promises: an .obdi
    /// file is a file like any other, and its geometry
    /// reaches the same writer.
    pub fn sanitized(mut self) -> Self {
        sanitize_styles(&mut self.styles);
        sanitize_panels(&mut self.panels);
        self
    }

    /// The asset hashes this sheet's frames use, in
    /// panel order, each once: picture placements, the
    /// music files its music boxes were engraved from,
    /// and panel tracing pictures that name a stored
    /// asset. The .obdi writer packs exactly these; the
    /// library attach merges exactly these.
    pub fn referenced_assets(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let mut push = |hash: &str| {
            if !hash.is_empty() && !out.iter().any(|h| h == hash) {
                out.push(hash.to_string());
            }
        };
        for panel in &self.panels {
            if let Some(bg) = &panel.background_image {
                push(bg);
            }
            for frame in &panel.frames {
                if let Some(pic) = &frame.picture {
                    push(&pic.asset);
                }
                if let Some(music) = &frame.music {
                    push(&music.source);
                }
            }
        }
        out
    }
}

impl Document {
    /// Store asset bytes in the document, recording (or
    /// keeping) their metadata, and return the SHA-256 they
    /// travel under. Adding the same bytes twice is one
    /// asset: the record already held keeps its noted
    /// values, and the incoming record fills what it
    /// leaves empty, field by field like every merge of
    /// records for one picture (round 5, RT5-2: placing
    /// a library picture whose bytes the bulletin already
    /// held used to throw the incoming license and title
    /// away whole).
    pub fn add_asset(
        &mut self,
        bytes: &[u8],
        kind: AssetKind,
        media_type: &str,
        rights: RightsRecord,
    ) -> String {
        let hash = sha256_hex(bytes);
        match self.assets.iter_mut().find(|a| a.sha256 == hash) {
            Some(existing) => {
                existing.rights = merge_rights_records(&existing.rights, &rights);
            }
            None => {
                self.assets.push(Asset {
                    sha256: hash.clone(),
                    kind,
                    media_type: media_type.to_string(),
                    rights,
                });
            }
        }
        self.asset_bytes.insert(hash.clone(), bytes.to_vec());
        hash
    }

    /// The bytes of one stored asset, when the file carried
    /// them (a metadata record whose bytes never arrived
    /// reads as missing, never as empty).
    pub fn asset_data(&self, sha256: &str) -> Option<&[u8]> {
        self.asset_bytes.get(sha256).map(|v| v.as_slice())
    }

    /// The pre-occasions field keys, renamed: v0 files and
    /// early v1 drafts name the service date and name
    /// "sunday_*"; the document calls them "occasion_*" now
    /// (docs/planning/occasions.md). Loading runs this so
    /// every file the app holds speaks the new names; the
    /// {Sunday.*} tokens resolve from the new keys, so old
    /// templates print unchanged.
    pub fn migrate_occasion_keys(&mut self) {
        for (old, new) in [
            ("sunday_date", "occasion_date"),
            ("sunday_date_long", "occasion_date_long"),
            ("sunday_name", "occasion_name"),
        ] {
            if let Some(v) = self.fields.remove(old) {
                self.fields.entry(new.to_string()).or_insert(v);
            }
        }
    }

    /// Fold the legacy participant fields into the
    /// participants map (roles build R-2b): before the
    /// map, the four original serving lines kept their
    /// names in `fields` under their slugs (readers,
    /// ushers, greeters, communion_assistants). The fold
    /// runs only while the map is empty, the same shape
    /// as the occasion-key migration above: a document
    /// that already speaks in the map is left exactly as
    /// it is, and its leftover field keys (if any) stay
    /// readable through `participant_value`'s fallback.
    pub fn migrate_participants(&mut self) {
        if !self.participants.is_empty() {
            return;
        }
        for (slug, _name) in PARTICIPANT_FIELDS.iter().take(4) {
            if let Some(v) = self.fields.remove(*slug) {
                self.participants.insert(slug.to_string(), v);
            }
        }
    }

    /// The names stored for one serving-line slug, as a
    /// token like `{Sunday.acolytes}` resolves them.
    /// Resolution order, so no honest storage goes
    /// unread:
    /// 1. the exact slug in the participants map (the
    ///    slug of the name as shown, and every built-in
    ///    slug, are both stored this way at times),
    ///    together with its built-in twin key: for a
    ///    built-in slug, the slug its display name
    ///    derives (and the other way round), because
    ///    the two differ for "Children's message";
    /// 2. the profile's recorded rename aliases
    ///    (`ChurchProfile::serving_aliases`, round 6,
    ///    R6-C): the chain from the token's canonical
    ///    slug, followed hop by hop, checking each
    ///    hop's slug (and its built-in twin) in the
    ///    map. A pairing recorded at save time is
    ///    followed; a line is NEVER guessed by its
    ///    position in the list (the round-3 position
    ///    bridge is deleted: after a rename plus a
    ///    front insertion it printed another line's
    ///    names, RT6-5). A token that resolves by
    ///    neither slug nor alias resolves EMPTY,
    ///    never to another line's names. A crafted
    ///    alias cycle cannot hang the walk: the walk
    ///    keeps the slugs it has visited (and stops
    ///    after 64 hops), and a cycle resolves EMPTY
    ///    — the walk stops at the first repeated slug
    ///    and yields no value from the cycle;
    /// 3. the legacy weekly fields, for a document that
    ///    has not been through the load migration
    ///    (importers and tests build documents in
    ///    memory).
    ///
    /// Empty when the line holds no names this week.
    pub fn participant_value(&self, slug: &str) -> String {
        if let Some(v) = self.participant_value_at(slug) {
            return v;
        }
        let start = canonical_serving_slug(slug);
        let mut visited: Vec<String> = vec![start.clone()];
        let mut current = start;
        loop {
            let next = self
                .profile
                .serving_aliases
                .get(&current)
                .or_else(|| {
                    self.profile
                        .serving_aliases
                        .get(&canonical_serving_slug(&current))
                })
                .cloned();
            let Some(next) = next else {
                break;
            };
            if visited.contains(&next) || visited.len() > 64 {
                // A cycle (or an absurdly long chain in
                // a crafted profile): resolves empty.
                break;
            }
            visited.push(next.clone());
            current = next;
            if let Some(v) = self.participant_value_at(&current) {
                return v;
            }
        }
        self.fields.get(slug).cloned().unwrap_or_default()
    }

    /// The value stored at exactly one slug, or at its
    /// built-in twin key (a built-in line's canonical
    /// slug and the slug its display name derives are
    /// the same line's two keys). No alias following,
    /// no positional guessing.
    fn participant_value_at(&self, slug: &str) -> Option<String> {
        if let Some(v) = self.participants.get(slug) {
            return Some(v.clone());
        }
        for (bslug, bname) in PARTICIPANT_FIELDS {
            let derived = serving_slug(bname);
            if slug == bslug {
                if let Some(v) = self.participants.get(&derived) {
                    return Some(v.clone());
                }
            }
            if slug == derived {
                if let Some(v) = self.participants.get(bslug) {
                    return Some(v.clone());
                }
            }
        }
        None
    }

    /// The names shown on one This Sunday row: the value
    /// stored for the line whose display name this is.
    pub fn participant_value_for_name(&self, display_name: &str) -> String {
        self.participant_value(&serving_slug(display_name))
    }

    /// Store one This Sunday row's names under the slug
    /// of the name as shown. A built-in line's permanent
    /// slug key is cleared too, so an older stored value
    /// cannot shadow the fresh one through the alias;
    /// an empty value clears the line outright.
    pub fn set_participant_value(&mut self, display_name: &str, value: &str) {
        let slug = serving_slug(display_name);
        if let Some(bslug) = builtin_serving_slug(display_name) {
            if bslug != slug {
                self.participants.remove(bslug);
            }
        }
        let value = value.trim();
        if value.is_empty() {
            self.participants.remove(&slug);
        } else {
            self.participants.insert(slug, value.to_string());
        }
    }

    /// How many paragraphs in the document are spoken only
    /// (read aloud in the service, never published).
    pub fn spoken_only_count(&self) -> usize {
        self.stories
            .iter()
            .flat_map(|s| s.paragraphs.iter())
            .filter(|p| p.spoken_only)
            .count()
    }

    /// The publication copy of this document: a clone with
    /// every spoken-only paragraph removed, plus how many
    /// were removed. This is the ONE rule every export
    /// follows (docs/planning/beyond-print.md section 4):
    /// the native .obd working file keeps spoken-only
    /// paragraphs; reader and imposed PDFs, the large-print
    /// copy, the web page, markdown, IDML, and .sla never
    /// carry them. Exporters take their document through
    /// here so no door can leak a prayer-list name by
    /// forgetting the rule.
    pub fn without_spoken_only(&self) -> (Document, usize) {
        let mut out = self.clone();
        let mut removed = 0usize;
        for story in &mut out.stories {
            let before = story.paragraphs.len();
            story.paragraphs.retain(|p| !p.spoken_only);
            removed += before - story.paragraphs.len();
        }
        (out, removed)
    }

    /// How many paragraphs in the document are marked
    /// not for the web (they print; they stay off the
    /// web page).
    pub fn web_hidden_count(&self) -> usize {
        self.stories
            .iter()
            .flat_map(|s| s.paragraphs.iter())
            .filter(|p| p.web_hidden)
            .count()
    }

    /// The web page's copy of this document: a clone with
    /// every not-for-web paragraph removed, plus how many
    /// were removed. ONLY the HTML export follows this
    /// rule (roles build R-2b): print, IDML, Scribus, and
    /// markdown keep these paragraphs, because the
    /// bulletin in the room is not the web page. The web
    /// export composes this AFTER `without_spoken_only`,
    /// so a paragraph carrying both marks is counted
    /// once, as spoken only.
    pub fn without_web_hidden(&self) -> (Document, usize) {
        let mut out = self.clone();
        let mut removed = 0usize;
        for story in &mut out.stories {
            let before = story.paragraphs.len();
            story.paragraphs.retain(|p| !p.web_hidden);
            removed += before - story.paragraphs.len();
        }
        (out, removed)
    }
}

/// The sentence an export door adds when spoken-only
/// paragraphs were left out of a copy, in the words the
/// office hears everywhere (the count is paragraphs; on a
/// prayer list each paragraph is one name). Empty when
/// nothing was left out.
pub fn spoken_only_left_out_sentence(count: usize) -> String {
    match count {
        0 => String::new(),
        1 => " 1 name is spoken only and was left out of this copy.".to_string(),
        n => format!(" {n} names are spoken only and were left out of this copy."),
    }
}

/// The sentence the web-page door adds when not-for-web
/// paragraphs were left out of the page, in the words
/// the office hears everywhere. Empty when nothing was
/// left out.
pub fn web_hidden_left_out_sentence(count: usize) -> String {
    match count {
        0 => String::new(),
        1 => " 1 paragraph was left out of the web page; it is for the room, not the web."
            .to_string(),
        n => format!(
            " {n} paragraphs were left out of the web page; they are for the room, not the web."
        ),
    }
}

/// The built-in serving lines (docs/planning/
/// beyond-print.md section 4, grown by roles build
/// R-2b): who is serving this week, beside the sermon
/// title on the This Sunday step (the J2 pattern). Each
/// is (the line's permanent slug, its built-in display
/// name). A template prints a line's names with the
/// token `{Sunday.<slug>}`. Weekly by nature: Next
/// Sunday clears them and names what it cleared.
///
/// The slug is the line's identity; the display name is
/// the church's words and can change. This table is the
/// fixed alias between the two for the built-ins: token
/// resolution tries the slug of the name as shown AND
/// this table, so a line's token keeps working after
/// the church renames the line (see
/// [`Document::participant_value`]).
pub const PARTICIPANT_FIELDS: [(&str, &str); 6] = [
    ("readers", "Readers"),
    ("ushers", "Ushers"),
    ("greeters", "Greeters"),
    ("communion_assistants", "Communion assistants"),
    ("childrens_message", "Children's message"),
    ("acolytes", "Acolytes"),
];

/// The token a template prints one serving line's names
/// with: `{Sunday.<slug>}`.
pub fn participant_token(slug: &str) -> String {
    format!("{{Sunday.{slug}}}")
}

/// The slug a serving line's display name derives:
/// lowercase, every run of non-alphanumeric characters
/// becomes one underscore, no underscore at either end
/// ("Children's message" derives "children_s_message";
/// its permanent slug is "childrens_message", which is
/// why the built-in alias table exists).
pub fn serving_slug(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut pending_underscore = false;
    for c in name.chars() {
        if c.is_alphanumeric() {
            if pending_underscore && !out.is_empty() {
                out.push('_');
            }
            pending_underscore = false;
            out.extend(c.to_lowercase());
        } else {
            pending_underscore = true;
        }
    }
    out
}

/// The built-in slug for a built-in display name, matched
/// case-insensitively ("acolytes" for "Acolytes"), or
/// None when the name is the church's own line.
pub fn builtin_serving_slug(name: &str) -> Option<&'static str> {
    PARTICIPANT_FIELDS
        .iter()
        .find(|(_, n)| n.eq_ignore_ascii_case(name.trim()))
        .map(|(slug, _)| *slug)
}

/// The built-in display name for a built-in slug.
pub fn builtin_serving_name(slug: &str) -> Option<&'static str> {
    PARTICIPANT_FIELDS
        .iter()
        .find(|(s, _)| *s == slug)
        .map(|(_, name)| *name)
}

/// The canonical slug a serving slug resolves under:
/// a built-in line's permanent slug when `slug` is
/// that slug or the slug its display name derives,
/// else the slug itself. Alias records are keyed by
/// canonical slugs, so a token spelled either way
/// enters the same chain.
pub fn canonical_serving_slug(slug: &str) -> String {
    for (bslug, bname) in PARTICIPANT_FIELDS {
        if slug == bslug || slug == serving_slug(bname) {
            return bslug.to_string();
        }
    }
    slug.to_string()
}

/// The slug a display name's line is keyed under in
/// the alias record: its built-in canonical slug when
/// the name is a built-in line, else the slug the
/// name derives.
fn serving_line_key(name: &str) -> String {
    match builtin_serving_slug(name) {
        Some(bslug) => bslug.to_string(),
        None => serving_slug(name),
    }
}

/// Follow an alias chain from `start` to the slug it
/// currently ends at: each hop is looked up by the
/// slug itself and by its canonical form. A cycle
/// ends the walk at the first repeated slug, and the
/// walk stops after 64 hops, so a crafted profile
/// cannot loop it.
fn alias_chain_end(aliases: &BTreeMap<String, String>, start: &str) -> String {
    let mut visited: Vec<String> = vec![start.to_string()];
    let mut current = start.to_string();
    loop {
        let next = aliases
            .get(&current)
            .or_else(|| aliases.get(&canonical_serving_slug(&current)))
            .cloned();
        let Some(next) = next else {
            return current;
        };
        if visited.contains(&next) || visited.len() > 64 {
            return current;
        }
        visited.push(next.clone());
        current = next;
    }
}

/// The serving lines a church starts with: the six
/// built-in display names, in their built-in order.
pub fn default_serving_lines() -> Vec<String> {
    PARTICIPANT_FIELDS
        .iter()
        .map(|(_, name)| name.to_string())
        .collect()
}

/// The names only in one of the two serving-line
/// lists, trimmed, in list order: the first list's
/// names the second list does not carry.
fn serving_names_only_in<'a>(list: &'a [String], other: &[String]) -> Vec<&'a str> {
    list.iter()
        .map(|n| n.trim())
        .filter(|n| !n.is_empty() && !other.iter().any(|m| m.trim() == *n))
        .collect()
}

/// The storage keys one serving line's names can sit
/// under: the slug of the name as shown, plus its
/// built-in canonical slug when the name is a
/// built-in and the two differ.
fn serving_participant_keys(name: &str) -> Vec<String> {
    let mut keys = vec![serving_slug(name)];
    if let Some(bslug) = builtin_serving_slug(name) {
        if !keys.contains(&bslug.to_string()) {
            keys.push(bslug.to_string());
        }
    }
    keys
}

/// How a profile save's two serving-line lists pair
/// up (round 5, R5-C; segment pairing since round 6,
/// R6-C). A pairing is followed only when it is
/// UNAMBIGUOUS from the two lists alone:
///
/// - the names present in BOTH lists are anchors, in
///   order. The runs of old-only names and new-only
///   names between consecutive anchors (and at the
///   ends) are segments. Within a segment, equal
///   counts pair in order (the in-place case, now
///   immune to additions and removals in OTHER
///   segments); a segment with only new names is
///   additions; a segment with only old names is
///   removals;
/// - exactly one old-only name and one new-only name
///   in the whole edit pair with each other wherever
///   either sits (a single rename, with or without a
///   move — even across an anchor).
///
/// After segment pairing, if unpaired old-only AND
/// new-only names both remain — a rename crossed an
/// anchor, so its partner sits in another segment —
/// the pairing is AMBIGUOUS and nothing is followed
/// at all: guessing is how two lines' names crossed in
/// the round-5 F3 probe. The save is refused as a
/// whole (see [`migrate_serving_line_renames_detailed`]).
/// Pure reorder or swap has no old-only or new-only
/// names at all, so nothing pairs and nothing is
/// refused. Recorded residual (round 5, held): a
/// rename-only cycle inside one segment pairs in
/// order like in-place renames; the lists alone
/// cannot tell it apart from them, and its cure is
/// row identity in the editor phase.
///
/// An ambiguous pairing still RECORDS what it
/// computed before refusing — the segment pairs and
/// the unpaired names on both sides — because two
/// readers need the intent even though nothing may
/// be followed: the slug-conflict check excuses a
/// line renaming itself out of those names (round 7,
/// R7-9), and the migration reports the kinds of
/// change the pairing saw (round 7, R7-10).
struct ServingPairing {
    /// (old name, new name) rename pairs to follow.
    pairs: Vec<(String, String)>,
    /// Old-only names with no partner anywhere: the
    /// lines are removed.
    removals: Vec<String>,
    /// True when old-only and new-only names both
    /// remain unpaired: the change is refused whole.
    ambiguous: bool,
    /// Old-only names the segment pairing left
    /// unpaired, kept on an ambiguous pairing as the
    /// rename intent the pairing computed (round 7,
    /// R7-9). Empty on an unambiguous pairing, whose
    /// unpaired old names are folded into `removals`.
    unpaired_old: Vec<String>,
    /// New-only names the segment pairing left
    /// unpaired, the other half of that intent.
    unpaired_new: Vec<String>,
    /// On an ambiguous pairing, the kinds of change
    /// the pairing saw; None otherwise.
    ambiguous_kinds: Option<ServingLineEditKinds>,
    /// Every segment's runs of old-only and new-only
    /// names, in segment order: the positional
    /// evidence the slug-conflict check judges from
    /// (round 8, R8-11) — within a segment, a new name
    /// sits at the position of the old name it is
    /// paired with, or, in a segment the pairing
    /// could not resolve, at the position of the line
    /// the pairing's own in-order reading assigns it.
    segments: Vec<(Vec<String>, Vec<String>)>,
}

/// The kinds of change one serving-line edit
/// contains, as the pairing saw them (round 7,
/// R7-10): the refusal sentence for an ambiguous
/// edit names only the kinds that are present, so a
/// rename-plus-removal refusal no longer claims a
/// move that never happened. `moved` is the pairing's
/// own crossing signal: names the pairing could not
/// resolve spread over more than one segment, so a
/// partner must sit in another segment than its
/// line. Segments the pairing DID resolve — a
/// segment holding only removals, or only additions
/// — contribute their own kinds only and never count
/// toward `moved` (round 8, R8-10); an old-only
/// segment counts toward it only when an unresolved
/// segment holds more new names than old ones, so
/// one of those new names' partners must sit in
/// another segment and an old-only segment is where
/// it can sit. A rename, a removal, or an addition
/// the counts cannot account for the other way is
/// read off the two lists' old-only and new-only
/// totals: more old-only names than new-only means a
/// line was removed, more new-only than old-only
/// means a line was added, and unpaired names on
/// both sides mean at least one rename is among the
/// changes the pairing could not resolve.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ServingLineEditKinds {
    /// At least one line was renamed.
    pub renamed: bool,
    /// At least one line was removed.
    pub removed: bool,
    /// At least one line was added.
    pub added: bool,
    /// At least one line's partner sits in another
    /// segment: the change crossed an anchor.
    pub moved: bool,
}

fn serving_pairing(old_lines: &[String], new_lines: &[String]) -> ServingPairing {
    let old_only = serving_names_only_in(old_lines, new_lines);
    let new_only = serving_names_only_in(new_lines, old_lines);
    if old_only.is_empty() && new_only.is_empty() {
        return ServingPairing {
            pairs: Vec::new(),
            removals: Vec::new(),
            ambiguous: false,
            unpaired_old: Vec::new(),
            unpaired_new: Vec::new(),
            ambiguous_kinds: None,
            segments: Vec::new(),
        };
    }
    if old_only.len() == 1 && new_only.len() == 1 {
        let pair = (old_only[0].to_string(), new_only[0].to_string());
        return ServingPairing {
            pairs: vec![pair],
            removals: Vec::new(),
            ambiguous: false,
            unpaired_old: Vec::new(),
            unpaired_new: Vec::new(),
            ambiguous_kinds: None,
            segments: Vec::new(),
        };
    }
    // Anchors: names present in both lists, matched
    // in order (a kept line that moved ahead of an
    // earlier anchor simply does not anchor; it is
    // in both lists, so it is in no segment's runs).
    let old_trim: Vec<String> = old_lines
        .iter()
        .map(|n| n.trim().to_string())
        .filter(|n| !n.is_empty())
        .collect();
    let new_trim: Vec<String> = new_lines
        .iter()
        .map(|n| n.trim().to_string())
        .filter(|n| !n.is_empty())
        .collect();
    let mut anchors: Vec<(usize, usize)> = Vec::new();
    let mut new_from = 0usize;
    for (oi, name) in old_trim.iter().enumerate() {
        if !new_trim.contains(name) {
            continue;
        }
        if let Some(rel) = new_trim[new_from..].iter().position(|n| n == name) {
            let ni = new_from + rel;
            anchors.push((oi, ni));
            new_from = ni + 1;
        }
    }
    // Segments between consecutive anchors, plus the
    // two ends.
    let mut segments: Vec<((usize, usize), (usize, usize))> = Vec::new();
    let mut o_start = 0usize;
    let mut n_start = 0usize;
    for &(oi, ni) in &anchors {
        segments.push(((o_start, oi), (n_start, ni)));
        o_start = oi + 1;
        n_start = ni + 1;
    }
    segments.push(((o_start, old_trim.len()), (n_start, new_trim.len())));
    let mut pairs: Vec<(String, String)> = Vec::new();
    let mut removals: Vec<String> = Vec::new();
    let mut leftover_old: Vec<String> = Vec::new();
    let mut leftover_new: Vec<String> = Vec::new();
    let mut segment_runs: Vec<(Vec<String>, Vec<String>)> = Vec::new();
    // The segment tallies the ambiguous kinds are
    // read from (round 7, R7-10; round 8, R8-10):
    // segments holding leftover names on both sides
    // are the ones the pairing could not resolve;
    // removal-only and addition-only segments are
    // resolved as what they are. An unresolved
    // segment holding more new names than old ones
    // means one of those new names' partners sits in
    // another segment, and an old-only segment is
    // where such a partner can sit — only then does
    // an old-only segment count toward `moved` too.
    let mut leftover_segments = 0usize;
    let mut removal_only_segments = 0usize;
    let mut addition_only_segments = 0usize;
    let mut leftover_has_surplus_new = false;
    for ((o0, o1), (n0, n1)) in &segments {
        let seg_old: Vec<String> = old_trim[*o0..*o1]
            .iter()
            .filter(|n| !new_trim.contains(n))
            .cloned()
            .collect();
        let seg_new: Vec<String> = new_trim[*n0..*n1]
            .iter()
            .filter(|n| !old_trim.contains(n))
            .cloned()
            .collect();
        segment_runs.push((seg_old.clone(), seg_new.clone()));
        if seg_old.len() == seg_new.len() {
            for (o, n) in seg_old.iter().zip(seg_new.iter()) {
                pairs.push((o.clone(), n.clone()));
            }
        } else if seg_new.is_empty() {
            if !seg_old.is_empty() {
                removal_only_segments += 1;
            }
            removals.extend(seg_old);
        } else if seg_old.is_empty() {
            // Additions only: nothing to pair or clear.
            addition_only_segments += 1;
        } else {
            leftover_segments += 1;
            if seg_new.len() > seg_old.len() {
                leftover_has_surplus_new = true;
            }
            leftover_old.extend(seg_old);
            leftover_new.extend(seg_new);
        }
    }
    if !leftover_old.is_empty() && !leftover_new.is_empty() {
        // Refused whole — but the computed pairs and
        // the unpaired names are kept on the pairing,
        // as the record of the intent the pairing saw
        // (round 7, R7-9, R7-10). Nothing follows
        // them: the migration checks `ambiguous` first.
        let unresolved_segments = leftover_segments
            + if leftover_has_surplus_new {
                removal_only_segments
            } else {
                0
            };
        let kinds = ServingLineEditKinds {
            renamed: true,
            removed: old_only.len() > new_only.len()
                || (removal_only_segments > 0 && !leftover_has_surplus_new),
            added: new_only.len() > old_only.len() || addition_only_segments > 0,
            moved: unresolved_segments >= 2,
        };
        return ServingPairing {
            pairs,
            removals,
            ambiguous: true,
            unpaired_old: leftover_old,
            unpaired_new: leftover_new,
            ambiguous_kinds: Some(kinds),
            segments: segment_runs,
        };
    }
    removals.extend(leftover_old);
    ServingPairing {
        pairs,
        removals,
        ambiguous: false,
        unpaired_old: Vec::new(),
        unpaired_new: Vec::new(),
        ambiguous_kinds: None,
        segments: segment_runs,
    }
}

/// What a profile save's rename migration did, in
/// the names the app must report (round 5, R5-C;
/// refusal since round 6, R6-C).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ServingLineMigration {
    /// Old-only lines whose stored names were
    /// cleared because the line is gone. Only lines
    /// that actually held names are listed.
    pub removed: Vec<String>,
    /// True when the pairing was ambiguous (renames
    /// crossed anchors): NOTHING was moved, cleared,
    /// or aliased, and the app refuses the
    /// serving-lines change as a whole, leaving the
    /// saved list exactly as it was.
    pub ambiguous: bool,
    /// When `ambiguous` is true, the kinds of change
    /// the pairing saw in the refused edit (round 7,
    /// R7-10), so the app's refusal sentence can name
    /// only what actually happened. None on a pairing
    /// that was followed.
    pub ambiguous_kinds: Option<ServingLineEditKinds>,
}

/// Move stored serving names across a profile edit
/// that renamed or removed lines (round 3, R3-3;
/// set-based since round 4, R4-2; unambiguous-only
/// since round 5, R5-C; segment pairing, recorded
/// aliases, and whole-change refusal since round 6,
/// R6-C). The pairing is `serving_pairing`'s: names
/// present in both lists anchor segments, and only
/// unambiguous pairs are followed. Each followed
/// pair's value, stored under the old name's keys
/// (its shown slug, and its built-in canonical slug
/// when the old name is a built-in), moves to the
/// new name's slug, AND the pairing is recorded in
/// `aliases` (the old line's key slug to the new
/// line's slug), so the old line's printed token
/// keeps resolving to the renamed line through
/// [`Document::participant_value`]. When the new
/// name's slug already holds a DIFFERENT value, both
/// values stay where they are: the target's value is
/// the fresher typing, and the source value keeps
/// its old key rather than being discarded. No
/// branch drops a value silently.
///
/// When the pairing is ambiguous, NOTHING is moved,
/// cleared, or aliased: the migration reports
/// `ambiguous` and the app refuses the serving-lines
/// change as a whole, so no name is left stranded
/// under a dead name (the round-5 "unfollowed"
/// disposition, and its status word, are gone).
///
/// An old-only name with no partner anywhere is a
/// REMOVED line (deleted outright, or renamed onto
/// a line that already exists): its stored values
/// are cleared, all its keys, any alias pointing at
/// its slug (or keyed by it) is dropped so its names
/// cannot reattach later, and its name is reported
/// as removed — in the same save as any successful
/// renames. Only names that actually held stored
/// values are reported. Pure over the two lists, the
/// map, and the alias record, so the rule is
/// testable without the app; the app calls the
/// detailed form when the profile's serving lines
/// are saved, and words the reported names in its
/// status line.
pub fn migrate_serving_line_renames_detailed(
    old_lines: &[String],
    new_lines: &[String],
    participants: &mut BTreeMap<String, String>,
    aliases: &mut BTreeMap<String, String>,
) -> ServingLineMigration {
    let pairing = serving_pairing(old_lines, new_lines);
    if pairing.ambiguous {
        return ServingLineMigration {
            removed: Vec::new(),
            ambiguous: true,
            ambiguous_kinds: pairing.ambiguous_kinds,
        };
    }
    for (old_name, new_name) in &pairing.pairs {
        move_participant_value(old_name, new_name, participants);
        let old_key = serving_line_key(old_name);
        let new_slug = serving_slug(new_name);
        if old_key != new_slug {
            aliases.insert(old_key, new_slug);
        }
    }
    let mut migration = ServingLineMigration::default();
    for old_name in &pairing.removals {
        let keys = serving_participant_keys(old_name);
        let mut held = false;
        for k in &keys {
            if participants.remove(k).is_some() {
                held = true;
            }
        }
        // The removed line's slugs, in every spelling
        // an alias could use: drop any alias pointing
        // at the line, and the line's own outgoing
        // alias, so a later line reusing the name
        // starts fresh and inherits nothing.
        let mut slugs = keys.clone();
        let key = serving_line_key(old_name);
        if !slugs.contains(&key) {
            slugs.push(key);
        }
        aliases.retain(|from, to| !slugs.contains(to) && !slugs.contains(from));
        if held {
            migration.removed.push(old_name.to_string());
        }
    }
    migration
}

/// The removed-line half of
/// [`migrate_serving_line_renames_detailed`], kept
/// for the callers and tests that only ask what was
/// cleared.
pub fn migrate_serving_line_renames(
    old_lines: &[String],
    new_lines: &[String],
    participants: &mut BTreeMap<String, String>,
    aliases: &mut BTreeMap<String, String>,
) -> Vec<String> {
    migrate_serving_line_renames_detailed(old_lines, new_lines, participants, aliases).removed
}

/// One serving-line slug conflict in a profile
/// save's new list (round 5, R5-C): the two lines
/// the app's refusal sentence names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServingLineSlugConflict {
    /// For two new-list lines deriving one slug: the
    /// first of them. For a built-in conflict: the
    /// taking line, identified distinguishably
    /// (round 8, R8-11) — the name it had in the old
    /// list, or, when the taking line is a new line
    /// or a line standing under the contested name
    /// itself, the contested name as typed.
    pub first: String,
    /// The other line: the twin in the new list, or,
    /// for a built-in conflict, the holding side —
    /// the name the bulletin currently prints for
    /// the claimed line (the claimant's current
    /// display name), never the built-in's own name
    /// unless the claimant currently stands under it.
    pub second: String,
    /// The slug both sides resolve through.
    pub slug: String,
    /// True when `second` is a built-in line's
    /// claimed place (the alias case); false for two
    /// new-list lines deriving the same slug.
    pub builtin: bool,
    /// True when the taking line is a new line in
    /// the new list (built-in conflicts only), so the
    /// refusal sentence can say "a new line" instead
    /// of naming a line that has no old name.
    pub taker_is_new: bool,
}

/// The slug conflict a profile save must refuse, if
/// any (round 5, R5-C, RT5-4; claimed-ness by
/// resolution since round 6, R6-C, R6-4).
/// `serving_slug` is many-to-one, so distinct
/// display names can share one storage slot and
/// silently share one set of names. The save
/// refuses when:
///
/// - two names in the new list derive the same
///   slug ("Prayer Team" and "Prayer-Team"), or
/// - a name in the new list derives the canonical
///   slug (or the shown-name slug) of a built-in
///   serving line while that line is not the line
///   that CLAIMS the slug, so the name would hijack
///   the built-in line's This Sunday row and its
///   printed `{Sunday.<slug>}` token.
///
/// Claimed-ness follows resolution, not spelling:
/// a built-in's canonical slug is claimed by the
/// OLD-list line that the built-in's token currently
/// resolves to — the line whose own key IS the
/// canonical slug (the built-in standing under its
/// own name, in either spelling), or the line whose
/// slug the profile's alias record maps the
/// canonical slug to, through its chain. Only the
/// claiming line may take a name deriving the
/// canonical slug, and the claiming line itself may
/// always rename to such a name — including back to
/// the built-in's own name. When no old-list line
/// claims the slug (the built-in line is gone and
/// no alias record survives), the name is free.
/// The conflict the check returns names the taking
/// line by the pairing's best evidence — the name it
/// had, or a new line — and the holding side by the
/// name the bulletin currently prints for the claimed
/// line, so a sentence built from it never names one
/// unqualified name on both sides (round 8, R8-11).
/// Pure over the two lists and the alias record;
/// the app calls it at the profile save and refuses
/// before anything changes.
pub fn serving_line_slug_conflict(
    old_lines: &[String],
    new_lines: &[String],
    aliases: &BTreeMap<String, String>,
) -> Option<ServingLineSlugConflict> {
    let names = serving_names_only_in(new_lines, &[]);
    // Two names the editor kept as distinct lines
    // deriving one slug share one storage slot.
    for (i, first) in names.iter().enumerate() {
        let first_slug = serving_slug(first);
        for second in names.iter().skip(i + 1) {
            if serving_slug(second) == first_slug {
                let conflict = ServingLineSlugConflict {
                    first: first.to_string(),
                    second: second.to_string(),
                    slug: first_slug,
                    builtin: false,
                    taker_is_new: false,
                };
                return Some(conflict);
            }
        }
    }
    let pairing = serving_pairing(old_lines, new_lines);
    for name in &names {
        let slug = serving_slug(name);
        if slug.is_empty() {
            continue;
        }
        for (bslug, bname) in PARTICIPANT_FIELDS {
            let derived = serving_slug(bname);
            if slug != bslug && slug != derived {
                continue;
            }
            let Some(claimant) = claiming_line(old_lines, aliases, bslug) else {
                // No old-list line holds this built-in's
                // token: the name takes no one's place.
                continue;
            };
            // The line itself: the claimant standing
            // under its current name, unchanged...
            if name.trim() == claimant {
                continue;
            }
            // Who takes this name, on the pairing's
            // best evidence (round 8, R8-11)? The
            // round-7 excuse tested set membership —
            // the claimant among the unpaired old
            // names, the contested name among the
            // unpaired new ones — which excused a
            // genuine hijack sharing those sets and
            // missed a claimant the pairing guessed
            // across inside an equal-count segment.
            // The judgment now works from who the
            // pairing assigns the name to: a followed
            // pair, or the line standing at the
            // name's own position inside its segment.
            // The excuse is reserved for the shapes
            // where that evidence assigns the name to
            // the claimant itself, or where no
            // positional evidence assigns it to a
            // different line. The excuse changes only
            // WHICH refusal the save meets — the
            // migration still refuses an ambiguous
            // edit whole — so the claimant is never
            // reported as conflicting with itself.
            let contested = name.trim();
            let mut taker: Option<(String, bool)> = None;
            let mut excused = false;
            if pairing
                .pairs
                .iter()
                .any(|(o, n)| o == claimant && n == contested)
            {
                // The claimant renaming to this name
                // in a pairing that would be followed.
                excused = true;
            } else if let Some((o, _)) = pairing.pairs.iter().find(|(_, n)| n == contested) {
                // A followed pair assigns the name to
                // a different line: that line takes it.
                taker = Some((o.clone(), false));
            } else {
                // Positional evidence inside the
                // segments: the contested name at
                // position i of a segment's new run
                // sits on the line at position i of
                // its old run, resolved segment or
                // not; past the old run's end, it
                // arrives on a new line.
                for (seg_old, seg_new) in &pairing.segments {
                    let Some(idx) = seg_new.iter().position(|n| n == contested) else {
                        continue;
                    };
                    if idx < seg_old.len() {
                        if seg_old[idx] == claimant {
                            excused = true;
                        } else {
                            taker = Some((seg_old[idx].clone(), false));
                        }
                    } else {
                        taker = Some((contested.to_string(), true));
                    }
                    break;
                }
            }
            if !excused && taker.is_none() {
                if old_lines.iter().any(|l| l.trim() == contested) {
                    // A line standing under this very
                    // name in the old list, unchanged:
                    // it is the taker, under its own
                    // name.
                    taker = Some((contested.to_string(), false));
                } else if pairing.ambiguous
                    && pairing.unpaired_old.iter().any(|o| o == claimant)
                    && pairing.unpaired_new.iter().any(|n| n == contested)
                {
                    // The membership excuse, kept for
                    // the shapes it was pinned on
                    // (round 7, R7-9) and only where no
                    // positional evidence above assigned
                    // the name to a different line.
                    excused = true;
                } else {
                    // No evidence assigns the name to
                    // any old line: it arrives on a
                    // new line.
                    taker = Some((contested.to_string(), true));
                }
            }
            if excused {
                continue;
            }
            let (taker_label, taker_is_new) =
                taker.unwrap_or_else(|| (contested.to_string(), false));
            let holder = claimant.to_string();
            let conflict = ServingLineSlugConflict {
                first: taker_label,
                second: holder,
                slug,
                builtin: true,
                taker_is_new,
            };
            return Some(conflict);
        }
    }
    None
}

/// The OLD-list line that currently claims a
/// built-in's canonical slug (round 6, R6-C): the
/// line whose own key is the canonical slug, or the
/// line whose slug the alias chain from the
/// canonical slug ends at. None when no old-list
/// line holds the built-in's token.
fn claiming_line<'a>(
    old_lines: &'a [String],
    aliases: &BTreeMap<String, String>,
    bslug: &str,
) -> Option<&'a str> {
    for line in old_lines {
        let shown = line.trim();
        if !shown.is_empty() && serving_line_key(shown) == bslug {
            return Some(shown);
        }
    }
    let end = alias_chain_end(aliases, bslug);
    if end != bslug {
        for line in old_lines {
            let shown = line.trim();
            if !shown.is_empty() && (serving_slug(shown) == end || serving_line_key(shown) == end) {
                return Some(shown);
            }
        }
    }
    None
}

/// Move one renamed line's stored value to its new
/// name's slug, under the keeping rules
/// `migrate_serving_line_renames` states: the value
/// comes from the first of the old name's keys that
/// holds one (shown slug first, canonical slug
/// second); a target already holding a different
/// value keeps it, and the source value then stays
/// under its old key; source keys holding the moved
/// value are cleared, while a source key holding a
/// different value keeps it, so no stored name is
/// ever discarded by a rename.
fn move_participant_value(
    old_name: &str,
    new_name: &str,
    participants: &mut BTreeMap<String, String>,
) {
    let target = serving_slug(new_name);
    let mut sources: Vec<String> = vec![serving_slug(old_name)];
    if let Some(bslug) = builtin_serving_slug(old_name) {
        if !sources.contains(&bslug.to_string()) {
            sources.push(bslug.to_string());
        }
    }
    let found = sources
        .iter()
        .filter(|k| k.as_str() != target)
        .find_map(|k| participants.get(k).cloned());
    let Some(value) = found else {
        return;
    };
    if let Some(existing) = participants.get(&target) {
        if *existing != value {
            // Both values stay where they are.
            return;
        }
    }
    participants.insert(target.clone(), value.clone());
    for k in &sources {
        if *k != target && participants.get(k).map(String::as_str) == Some(value.as_str()) {
            participants.remove(k);
        }
    }
}

/// Clamp one style table's numbers to the sane bounds
/// [`Document::sanitized`] promises.
fn sanitize_styles(styles: &mut [ParagraphStyle]) {
    for style in styles {
        style.font_size = style.font_size.clamp(500, 288_000);
        style.space_after = style.space_after.clamp(0, 144_000);
    }
}

/// Clamp one panel list's geometry to the sane bounds
/// [`Document::sanitized`] promises.
fn sanitize_panels(panels: &mut [Panel]) {
    for panel in panels {
        panel.size.w = panel.size.w.clamp(0, 10_000_000);
        panel.size.h = panel.size.h.clamp(0, 10_000_000);
        for frame in &mut panel.frames {
            frame.rect.x = frame.rect.x.clamp(-10_000_000, 10_000_000);
            frame.rect.y = frame.rect.y.clamp(-10_000_000, 10_000_000);
            frame.rect.w = frame.rect.w.clamp(0, 10_000_000);
            frame.rect.h = frame.rect.h.clamp(0, 10_000_000);
            if let Some(pic) = &mut frame.picture {
                if !pic.crop_x.is_finite() {
                    pic.crop_x = 0.5;
                }
                if !pic.crop_y.is_finite() {
                    pic.crop_y = 0.5;
                }
                pic.crop_x = pic.crop_x.clamp(0.0, 1.0);
                pic.crop_y = pic.crop_y.clamp(0.0, 1.0);
            }
            if let Some(music) = &mut frame.music {
                if !music.staff_mm.is_finite() || music.staff_mm <= 0.0 {
                    music.staff_mm = 6.5;
                }
                music.staff_mm = music.staff_mm.clamp(1.0, 50.0);
            }
        }
    }
}

impl Document {
    /// Clamp the numbers a hostile or hand-edited file could set
    /// to extremes (a font size of i64::MAX used to overflow the
    /// PDF writer's arithmetic and panic — closing audit). The
    /// bounds are far outside anything a real bulletin uses.
    pub fn sanitized(mut self) -> Self {
        sanitize_styles(&mut self.styles);
        sanitize_panels(&mut self.panels);
        // A loose sheet's geometry gets the same clamp
        // (roles build R-3): its numbers come from the
        // same kind of file, and its panels print through
        // the same writer.
        for insert in &mut self.inserts {
            sanitize_styles(&mut insert.styles);
            sanitize_panels(&mut insert.panels);
        }
        self
    }
}

impl Document {
    pub fn style(&self, id: &str) -> Option<&ParagraphStyle> {
        self.styles.iter().find(|s| s.id == id)
    }
    pub fn story(&self, id: &str) -> Option<&Story> {
        self.stories.iter().find(|s| s.id == id)
    }
    pub fn story_mut(&mut self, id: &str) -> Option<&mut Story> {
        self.stories.iter_mut().find(|s| s.id == id)
    }

    /// Every frame that draws a story, as (panel index,
    /// frame index) pairs in document order: panel order,
    /// then frame order within the panel. A story in more
    /// than one frame is a thread (T1): its words flow
    /// through the frames in exactly this order, and every
    /// surface (canvas, PDF, Check) reads the order here so
    /// they can never disagree about which box is next.
    pub fn frames_of_story(&self, story_id: &str) -> Vec<(usize, usize)> {
        let mut out = Vec::new();
        for (pi, panel) in self.panels.iter().enumerate() {
            for (fi, frame) in panel.frames.iter().enumerate() {
                if frame.story_id.as_deref() == Some(story_id) {
                    out.push((pi, fi));
                }
            }
        }
        out
    }
}

/// The story the service lines live in (J2): the one
/// holding the tabbed order lines. Tabbed lines with a
/// "Sermon:" line alongside confirm it (the fallback for
/// imported bulletins, whose frames carry no roles worth
/// trusting); otherwise a story a Standing frame holds;
/// otherwise the first story with tabbed lines at all.
/// This is the ONE lookup every surface shares: the This
/// Sunday editor, the hymn slotting, Check's hymn pass,
/// and Next Sunday's hymn clearing all mean the same
/// story when they say "the service" (round 3, R3-17).
pub fn service_story_id(doc: &Document) -> Option<String> {
    let has_tabs = |s: &Story| s.paragraphs.iter().any(|p| p.text().contains('\t'));
    let has_sermon = |s: &Story| {
        s.paragraphs
            .iter()
            .any(|p| p.text().trim_start().starts_with("Sermon:"))
    };
    if let Some(s) = doc.stories.iter().find(|s| has_tabs(s) && has_sermon(s)) {
        return Some(s.id.clone());
    }
    let standing: Vec<&str> = doc
        .panels
        .iter()
        .flat_map(|p| p.frames.iter())
        .filter(|f| f.role == ContentRole::Standing)
        .filter_map(|f| f.story_id.as_deref())
        .collect();
    if let Some(s) = doc
        .stories
        .iter()
        .find(|s| standing.contains(&s.id.as_str()) && has_tabs(s))
    {
        return Some(s.id.clone());
    }
    doc.stories
        .iter()
        .find(|s| has_tabs(s))
        .map(|s| s.id.clone())
}

/// One field token found in raw text: where it sits in the raw
/// text, where its value sits in the resolved text, and the
/// token as written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldSpan {
    pub raw: std::ops::Range<usize>,
    pub resolved: std::ops::Range<usize>,
    pub token: String,
}

/// Text with its field tokens resolved, plus the span map the
/// canvas editor needs to treat each token as one indivisible
/// chip (E1): the resolved text is what prints; the spans say
/// which raw bytes produced which resolved bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedText {
    pub text: String,
    pub spans: Vec<FieldSpan>,
}

/// The field tokens and where their values come from, in one
/// table so the plain resolver and the mapped resolver can
/// never disagree about the set.
fn field_tokens(doc: &Document) -> Vec<(String, String)> {
    let p = &doc.profile;
    let get = |k: &str| doc.fields.get(k).cloned().unwrap_or_default();
    let mut out: Vec<(String, String)> = [
        ("{Church.name}", p.name.clone()),
        ("{Church.address}", p.address.clone()),
        ("{Church.times}", p.service_times.clone()),
        ("{Church.website}", p.website.clone()),
        ("{Church.giving}", p.giving_url.clone()),
        ("{Church.pastor}", p.pastor.clone()),
        ("{Sunday.date}", get("occasion_date_long")),
        ("{Sunday.name}", get("occasion_name")),
        ("{Sunday.reading}", get("reading")),
        (
            "{Service.time}",
            doc.occasion_time.clone().unwrap_or_default(),
        ),
    ]
    .into_iter()
    .map(|(tok, val)| (tok.to_string(), val))
    .collect();
    // The serving lines (roles build R-2b): one token per
    // line, minted from the slug. The set is the union of
    // the built-in slugs, the slugs of the profile's
    // current line names, and every slug holding a stored
    // value, so a token resolves the same week it is
    // typed, the week after a rename, and for a line the
    // profile no longer shows but the week still holds.
    // Values resolve through `participant_value`, whose
    // alias rules these tokens inherit. A serving slug
    // that collided with a fixed token above loses: the
    // fixed words were there first.
    let mut slugs: Vec<String> = Vec::new();
    let mut add_slug = |slug: String| {
        if !slug.is_empty() && !slugs.contains(&slug) {
            slugs.push(slug);
        }
    };
    for (slug, _name) in PARTICIPANT_FIELDS {
        add_slug(slug.to_string());
    }
    for name in &p.serving_lines {
        add_slug(serving_slug(name));
    }
    // The recorded rename aliases (round 6, R6-C):
    // an old line's token stays minted, and resolves
    // through the alias, after its line is renamed.
    for (old_slug, new_slug) in &p.serving_aliases {
        add_slug(old_slug.clone());
        add_slug(new_slug.clone());
    }
    for key in doc.participants.keys() {
        add_slug(key.clone());
    }
    for slug in slugs {
        let tok = participant_token(&slug);
        if out.iter().any(|(t, _)| *t == tok) {
            continue;
        }
        let val = doc.participant_value(&slug);
        out.push((tok, val));
    }
    out
}

/// Resolve `{...}` field tokens in text against the profile and
/// field map, remembering where each token went.
///
/// Resolution is a single left-to-right pass: a value that
/// itself contains a token-looking string is data, not a
/// template, and is never re-resolved. (The previous chained
/// `.replace` implementation could re-resolve inside values;
/// every display surface now shares this one pass, so the
/// canvas, the previews, and the PDF always agree.)
pub fn resolve_fields_mapped(text: &str, doc: &Document) -> ResolvedText {
    let tokens = field_tokens(doc);
    let mut out = String::with_capacity(text.len());
    let mut spans = Vec::new();
    let mut i = 0usize;
    while i < text.len() {
        let rest = &text[i..];
        let hit = tokens
            .iter()
            .find(|(tok, _)| rest.starts_with(tok.as_str()))
            .map(|(tok, val)| (tok.as_str(), val));
        match hit {
            Some((tok, val)) => {
                let raw = i..i + tok.len();
                let resolved = out.len()..out.len() + val.len();
                out.push_str(val);
                spans.push(FieldSpan {
                    raw,
                    resolved,
                    token: tok.to_string(),
                });
                i += tok.len();
            }
            None => {
                // Copy one character (never split a UTF-8 boundary).
                let ch_len = rest.chars().next().map(|c| c.len_utf8()).unwrap_or(1);
                out.push_str(&rest[..ch_len]);
                i += ch_len;
            }
        }
    }
    ResolvedText { text: out, spans }
}

/// Resolve `{...}` field tokens in text against the profile and field map.
pub fn resolve_fields(text: &str, doc: &Document) -> String {
    resolve_fields_mapped(text, doc).text
}

/// Resolve a paragraph's runs: the same single resolution
/// pass as `resolve_fields`, carried over the run list so
/// the shaped renderers can keep each span's face switches.
/// A token's resolved value takes the switches of the run
/// that holds the token's first byte (a token split across
/// runs is still one chip; it cannot wear two faces).
/// Literal text keeps its runs, split only at token edges.
pub fn resolve_runs(runs: &[Run], doc: &Document) -> Vec<Run> {
    // Run boundaries over the concatenated raw text.
    let mut bounds: Vec<(usize, usize, Option<bool>, Option<bool>)> = Vec::new();
    let mut raw = String::new();
    for run in runs {
        let start = raw.len();
        raw.push_str(&run.text);
        bounds.push((start, raw.len(), run.bold, run.italic));
    }
    if raw.is_empty() {
        return vec![Run::plain("")];
    }
    let mapped = resolve_fields_mapped(&raw, doc);
    let flags_at = |offset: usize| -> (Option<bool>, Option<bool>) {
        for &(s, e, b, i) in &bounds {
            if offset >= s && offset < e {
                return (b, i);
            }
        }
        bounds
            .last()
            .map(|&(_, _, b, i)| (b, i))
            .unwrap_or((None, None))
    };
    // Pieces of the raw text, in order: literal stretches
    // (split at run boundaries) and whole tokens.
    let mut pieces: Vec<(String, Option<bool>, Option<bool>)> = Vec::new();
    let mut cursor = 0usize;
    let push_literal =
        |from: usize, to: usize, pieces: &mut Vec<(String, Option<bool>, Option<bool>)>| {
            let mut pos = from;
            while pos < to {
                let (b, i) = flags_at(pos);
                let run_end = bounds
                    .iter()
                    .find(|&&(s, e, _, _)| pos >= s && pos < e)
                    .map(|&(_, e, _, _)| e)
                    .unwrap_or(to);
                let next = run_end.min(to);
                pieces.push((raw[pos..next].to_string(), b, i));
                pos = next;
            }
        };
    for span in &mapped.spans {
        if cursor < span.raw.start {
            push_literal(cursor, span.raw.start, &mut pieces);
        }
        let (b, i) = flags_at(span.raw.start);
        pieces.push((mapped.text[span.resolved.clone()].to_string(), b, i));
        cursor = span.raw.end;
    }
    if cursor < raw.len() {
        push_literal(cursor, raw.len(), &mut pieces);
    }
    normalize_runs(
        pieces
            .into_iter()
            .map(|(text, bold, italic)| Run { text, bold, italic })
            .collect(),
    )
}

/// Split one paragraph's runs at a byte offset in the
/// paragraph's text: (the runs before, the runs from the
/// offset on). The run the offset lands in divides, keeping
/// its face switches on both sides. A side with no words
/// is one empty plain run, the model's standing shape for
/// an empty paragraph.
fn split_runs_at(runs: &[Run], byte: usize) -> (Vec<Run>, Vec<Run>) {
    let mut head: Vec<Run> = Vec::new();
    let mut tail: Vec<Run> = Vec::new();
    let mut at = 0usize;
    for run in runs {
        let end = at + run.text.len();
        if end <= byte {
            head.push(run.clone());
        } else if at >= byte {
            tail.push(run.clone());
        } else {
            let cut = byte - at;
            head.push(Run {
                text: run.text[..cut].to_string(),
                bold: run.bold,
                italic: run.italic,
            });
            tail.push(Run {
                text: run.text[cut..].to_string(),
                bold: run.bold,
                italic: run.italic,
            });
        }
        at = end;
    }
    (normalize_runs(head), normalize_runs(tail))
}

/// Cut a story's paragraphs in two at (paragraph index,
/// byte offset in that paragraph's text): everything
/// before the cut, then everything from the cut on. This
/// is the surgery "Make this box start its own text" (T1)
/// performs when a thread divides: the earlier boxes keep
/// the head, the later boxes get the tail as a story of
/// their own. A cut at the very start gives an empty head
/// (one empty paragraph in the first paragraph's style);
/// a cut at the very end gives an empty tail the same way,
/// so both sides always hold at least one paragraph.
pub fn split_story_paragraphs(
    paras: &[Paragraph],
    para: usize,
    byte: usize,
) -> (Vec<Paragraph>, Vec<Paragraph>) {
    if paras.is_empty() {
        let empty = vec![Paragraph::plain("body", "")];
        return (empty.clone(), empty);
    }
    let para = para.min(paras.len() - 1);
    let target = &paras[para];
    let byte = byte.min(target.text().len());
    // Never split inside a character.
    let byte = (0..=byte)
        .rev()
        .find(|&b| target.text().is_char_boundary(b))
        .unwrap_or(0);
    let (head_runs, tail_runs) = split_runs_at(&target.runs, byte);
    let mut head: Vec<Paragraph> = paras[..para].to_vec();
    let mut tail: Vec<Paragraph> = Vec::new();
    // The two pieces of the cut paragraph keep its
    // spoken-only flag: splitting a prayer name in two
    // must never make either half printable. The
    // not-for-web flag rides the same way, for the
    // same reason.
    if byte > 0 || para == 0 {
        let mut p = Paragraph::from_runs(&target.style, head_runs);
        p.spoken_only = target.spoken_only;
        p.web_hidden = target.web_hidden;
        head.push(p);
    }
    if byte < target.text().len() || para == paras.len() - 1 {
        let mut p = Paragraph::from_runs(&target.style, tail_runs);
        p.spoken_only = target.spoken_only;
        p.web_hidden = target.web_hidden;
        tail.push(p);
    }
    tail.extend_from_slice(&paras[para + 1..]);
    if head.is_empty() {
        head.push(Paragraph::plain(&target.style, ""));
    }
    if tail.is_empty() {
        tail.push(Paragraph::plain(&target.style, ""));
    }
    (head, tail)
}

/// The raw-text byte offset that a byte offset in the
/// RESOLVED text (field tokens filled in) came from, for
/// one paragraph's raw `text`. Threading computes its cut
/// points in resolved space, because that is what prints;
/// story surgery needs them in raw space, because that is
/// what the file stores. A position inside a token's
/// filled-in value maps to the token's raw START: the whole
/// token travels with the tail. (A value long enough to
/// wrap across a box boundary mid-value is the one corner
/// this can repeat a few words in; losing words silently
/// would be worse, and the corner needs a value that wraps
/// at exactly a box edge.)
pub fn raw_offset_for_resolved(text: &str, doc: &Document, resolved_off: usize) -> usize {
    let mapped = resolve_fields_mapped(text, doc);
    if resolved_off >= mapped.text.len() {
        return text.len();
    }
    let mut raw = resolved_off;
    for span in &mapped.spans {
        if resolved_off < span.resolved.start {
            break;
        }
        if resolved_off < span.resolved.end {
            // Inside (or at the start of) this token's value.
            return span.raw.start;
        }
        // The whole token sits before the position: trade
        // its resolved length for its raw length.
        raw = raw - (span.resolved.end - span.resolved.start) + (span.raw.end - span.raw.start);
    }
    raw.min(text.len())
}

/// The house style set every template starts from. Large-print leaning:
/// body text is 11pt minimum, order-of-service lines get their own style
/// because they carry tab-leader hymn numbers.
pub fn default_styles() -> Vec<ParagraphStyle> {
    let st =
        |id: &str, name: &str, size: Mpt, bold: bool, italic: bool, align: Align, after: Mpt| {
            ParagraphStyle {
                id: id.to_string(),
                name: name.to_string(),
                font_size: size,
                bold,
                italic,
                align,
                space_after: after,
                color: Rgb::BLACK,
                font_family: default_font_family(),
                font_sha256: None,
            }
        };
    vec![
        st(
            "church-name",
            "Church Name",
            26_000,
            true,
            false,
            Align::Center,
            6_000,
        ),
        st(
            "heading",
            "Heading",
            18_000,
            true,
            false,
            Align::Left,
            6_000,
        ),
        st(
            "subheading",
            "Subheading",
            13_000,
            true,
            false,
            Align::Left,
            4_000,
        ),
        st("body", "Body", 11_000, false, false, Align::Left, 4_000),
        st(
            "body-center",
            "Body Centered",
            11_000,
            false,
            false,
            Align::Center,
            4_000,
        ),
        st(
            "order-line",
            "Order of Service Line",
            11_000,
            false,
            false,
            Align::Left,
            3_000,
        ),
        st(
            "notes-line",
            "Sermon Notes Line",
            11_000,
            false,
            false,
            Align::Left,
            10_000,
        ),
        st(
            "caption",
            "Caption",
            9_500,
            false,
            false,
            Align::Center,
            3_000,
        ),
        st(
            "scripture",
            "Scripture",
            11_000,
            false,
            true,
            Align::Left,
            4_000,
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inch_math() {
        assert_eq!(inch(1.0), 72_000);
        assert_eq!(inch(5.5), 396_000);
        assert_eq!(inch(8.5), 612_000);
    }

    #[test]
    fn fields_resolve() {
        let mut fields = BTreeMap::new();
        fields.insert(
            "occasion_name".to_string(),
            "Twentieth Sunday after Pentecost".to_string(),
        );
        fields.insert(
            "occasion_date_long".to_string(),
            "October 18, 2026".to_string(),
        );
        let doc = Document {
            format_version: FORMAT_VERSION,
            title: "t".into(),
            profile: ChurchProfile {
                name: "Grace Lutheran".into(),
                ..Default::default()
            },
            fields,
            styles: vec![],
            stories: vec![],
            panels: vec![],
            sheet: SheetSpec {
                preset: "letter-portrait".into(),
                size: Size::new(0, 0),
            },
            no_announcements: false,
            assets: Vec::new(),
            asset_bytes: Default::default(),
            occasion_time: None,
            wanted_fonts: Vec::new(),
            participants: BTreeMap::new(),
            inserts: Vec::new(),
            applied_contributions: Vec::new(),
        };
        assert_eq!(
            resolve_fields("{Church.name} - {Sunday.name}, {Sunday.date}", &doc),
            "Grace Lutheran - Twentieth Sunday after Pentecost, October 18, 2026"
        );
    }

    fn bare_doc() -> Document {
        Document {
            format_version: FORMAT_VERSION,
            title: "t".into(),
            profile: ChurchProfile::default(),
            fields: BTreeMap::new(),
            styles: vec![],
            stories: vec![],
            panels: vec![],
            sheet: SheetSpec {
                preset: "letter-portrait".into(),
                size: Size::new(0, 0),
            },
            no_announcements: false,
            assets: Vec::new(),
            asset_bytes: Default::default(),
            occasion_time: None,
            wanted_fonts: Vec::new(),
            participants: BTreeMap::new(),
            inserts: Vec::new(),
            applied_contributions: Vec::new(),
        }
    }

    #[test]
    fn participant_tokens_resolve_with_and_without_values() {
        let mut doc = bare_doc();
        // Unset: every participant token resolves to nothing,
        // exactly like the other weekly fields.
        assert_eq!(
            resolve_fields(
                "Readers: {Sunday.readers}|Ushers: {Sunday.ushers}|Greeters: {Sunday.greeters}|Communion: {Sunday.communion_assistants}",
                &doc
            ),
            "Readers: |Ushers: |Greeters: |Communion: "
        );
        doc.fields
            .insert("readers".to_string(), "Ana Ruiz".to_string());
        doc.fields
            .insert("ushers".to_string(), "Tom and Bea".to_string());
        doc.fields.insert("greeters".to_string(), "Sam".to_string());
        doc.fields
            .insert("communion_assistants".to_string(), "Pat, Jo".to_string());
        assert_eq!(
            resolve_fields(
                "Readers: {Sunday.readers}|Ushers: {Sunday.ushers}|Greeters: {Sunday.greeters}|Communion: {Sunday.communion_assistants}",
                &doc
            ),
            "Readers: Ana Ruiz|Ushers: Tom and Bea|Greeters: Sam|Communion: Pat, Jo"
        );
        // The spec table's tokens and the resolver's agree.
        for (key, _label) in PARTICIPANT_FIELDS.iter().take(4) {
            let key = *key;
            let token = participant_token(key);
            assert!(doc.fields.contains_key(key), "test setup covers {key}");
            assert_eq!(
                resolve_fields(&token, &doc),
                doc.fields[key],
                "{token} resolves from {key}"
            );
        }
        // The two lines R-2b added resolve from the map.
        doc.set_participant_value("Children's message", "Ms. Dee");
        doc.set_participant_value("Acolytes", "Sam, Jo");
        assert_eq!(
            resolve_fields("{Sunday.childrens_message}", &doc),
            "Ms. Dee"
        );
        assert_eq!(resolve_fields("{Sunday.acolytes}", &doc), "Sam, Jo");
    }

    #[test]
    fn legacy_participant_fields_fold_into_the_map_on_load() {
        // A file written before the participants map: the
        // four names live in `fields` under their slugs.
        let mut doc = bare_doc();
        doc.fields
            .insert("readers".to_string(), "Ana Ruiz".to_string());
        doc.fields
            .insert("ushers".to_string(), "Tom and Bea".to_string());
        doc.migrate_participants();
        assert!(!doc.fields.contains_key("readers"));
        assert_eq!(doc.participant_value("readers"), "Ana Ruiz");
        assert_eq!(
            resolve_fields("Readers: {Sunday.readers}", &doc),
            "Readers: Ana Ruiz"
        );
        // The fold is a fold, not a copy: running it again
        // changes nothing, and a document already speaking
        // in the map keeps its fields untouched.
        doc.fields
            .insert("greeters".to_string(), "Late".to_string());
        doc.migrate_participants();
        assert_eq!(doc.fields.get("greeters").map(String::as_str), Some("Late"));
        // ...and the leftover still resolves, through the
        // fallback, so no honest storage goes unread.
        assert_eq!(resolve_fields("{Sunday.greeters}", &doc), "Late");
    }

    #[test]
    fn a_renamed_serving_line_keeps_its_token() {
        // R-2b: the church renames "Acolytes" to "Candle
        // lighters". Names stored while the line showed
        // its built-in name sit under the built-in slug,
        // and the template's token must keep resolving.
        let mut doc = bare_doc();
        doc.set_participant_value("Acolytes", "Sam, Jo");
        doc.profile.serving_lines = vec!["Candle lighters".to_string()];
        assert_eq!(resolve_fields("{Sunday.acolytes}", &doc), "Sam, Jo");
        // Names stored under the new name as shown resolve
        // under the new name's own token too.
        doc.set_participant_value("Candle lighters", "Rae");
        assert_eq!(doc.participant_value_for_name("Candle lighters"), "Rae");
        assert_eq!(resolve_fields("{Sunday.candle_lighters}", &doc), "Rae");
    }

    #[test]
    fn a_profile_rename_moves_the_stored_names() {
        // Round 3, R3-3: renaming "Acolytes" to "Candle
        // lighters" in the profile moves the stored names
        // to the renamed line: the row shows them, typing
        // replaces them, and the built-in token prints
        // the NEW names afterwards (the audit's gap was a
        // regression test that asserted only the row).
        let mut doc = bare_doc();
        doc.profile.serving_lines = default_serving_lines();
        doc.set_participant_value("Acolytes", "Sam, Jo");
        let old_lines = doc.profile.serving_lines.clone();
        let mut new_lines = old_lines.clone();
        new_lines[5] = "Candle lighters".to_string();
        migrate_serving_line_renames(
            &old_lines,
            &new_lines,
            &mut doc.participants,
            &mut doc.profile.serving_aliases,
        );
        doc.profile.serving_lines = new_lines;
        // The value moved; the built-in key is cleared.
        assert_eq!(
            doc.participants.get("candle_lighters").map(String::as_str),
            Some("Sam, Jo")
        );
        assert!(!doc.participants.contains_key("acolytes"));
        // The renamed row shows the names.
        assert_eq!(doc.participant_value_for_name("Candle lighters"), "Sam, Jo");
        // Typing new names in the renamed row replaces
        // them...
        doc.set_participant_value("Candle lighters", "Rae, Kim");
        assert_eq!(
            doc.participant_value_for_name("Candle lighters"),
            "Rae, Kim"
        );
        // ...and the template's built-in token prints the
        // NEW names, not the stranded ones.
        assert_eq!(resolve_fields("{Sunday.acolytes}", &doc), "Rae, Kim");
        assert_eq!(resolve_fields("{Sunday.candle_lighters}", &doc), "Rae, Kim");
    }

    #[test]
    fn a_rename_moves_names_from_either_stored_key() {
        // "Children's message" is stored under its shown
        // slug (children_s_message) by the row, but a
        // value can also sit under the permanent slug
        // (childrens_message); a rename moves either.
        let mut doc = bare_doc();
        doc.profile.serving_lines = default_serving_lines();
        doc.participants
            .insert("childrens_message".to_string(), "Ms. Dee".to_string());
        let old_lines = doc.profile.serving_lines.clone();
        let mut new_lines = old_lines.clone();
        new_lines[4] = "Kids' message".to_string();
        migrate_serving_line_renames(
            &old_lines,
            &new_lines,
            &mut doc.participants,
            &mut doc.profile.serving_aliases,
        );
        doc.profile.serving_lines = new_lines;
        assert_eq!(doc.participant_value_for_name("Kids' message"), "Ms. Dee");
        assert_eq!(
            resolve_fields("{Sunday.childrens_message}", &doc),
            "Ms. Dee"
        );
        assert!(!doc.participants.contains_key("childrens_message"));
        // A save with no renames moves nothing.
        doc.set_participant_value("Readers", "Ana Ruiz");
        let before = doc.participants.clone();
        let lines_now = doc.profile.serving_lines.clone();
        migrate_serving_line_renames(
            &lines_now,
            &lines_now,
            &mut doc.participants,
            &mut doc.profile.serving_aliases,
        );
        assert_eq!(doc.participants, before);
    }

    #[test]
    fn a_swap_or_reorder_moves_and_crosses_nothing() {
        // Round 4, R4-2: the migration compares the
        // lists as sets of names. A name in both lists
        // is the same line wherever it sits, so tidying
        // the list's order loses nothing and crosses
        // nothing (the positional rule lost one line's
        // names and printed another's against the
        // wrong duty).
        let mut doc = bare_doc();
        doc.profile.serving_lines = default_serving_lines();
        doc.set_participant_value("Readers", "Ann");
        doc.set_participant_value("Acolytes", "Bob");
        let before = doc.participants.clone();
        let old_lines = doc.profile.serving_lines.clone();
        let mut swapped = old_lines.clone();
        swapped.swap(0, 5);
        let removed = migrate_serving_line_renames(
            &old_lines,
            &swapped,
            &mut doc.participants,
            &mut doc.profile.serving_aliases,
        );
        assert!(removed.is_empty());
        assert_eq!(doc.participants, before);
        doc.profile.serving_lines = swapped;
        assert_eq!(doc.participant_value_for_name("Readers"), "Ann");
        assert_eq!(doc.participant_value_for_name("Acolytes"), "Bob");
        assert_eq!(resolve_fields("{Sunday.readers}", &doc), "Ann");
        assert_eq!(resolve_fields("{Sunday.acolytes}", &doc), "Bob");

        // A three-line reorder, the review lane's
        // traced shape, is the same story.
        let mut doc = bare_doc();
        doc.profile.serving_lines = default_serving_lines();
        doc.set_participant_value("Readers", "Rae");
        doc.set_participant_value("Ushers", "Uma");
        doc.set_participant_value("Greeters", "Gus");
        let before = doc.participants.clone();
        let old_lines = doc.profile.serving_lines.clone();
        let mut reordered = old_lines.clone();
        reordered.swap(1, 2);
        let removed = migrate_serving_line_renames(
            &old_lines,
            &reordered,
            &mut doc.participants,
            &mut doc.profile.serving_aliases,
        );
        assert!(removed.is_empty());
        assert_eq!(doc.participants, before);
        doc.profile.serving_lines = reordered;
        assert_eq!(resolve_fields("{Sunday.ushers}", &doc), "Uma");
        assert_eq!(resolve_fields("{Sunday.greeters}", &doc), "Gus");
    }

    #[test]
    fn a_rename_with_a_reorder_lands_on_the_renamed_line() {
        // Round 4, R4-2: one edit that renames
        // "Acolytes" to "Candle lighters" AND moves
        // the line to the top of the list. The acolyte's
        // names land on the renamed line; the readers'
        // names stay on Readers, which also moved.
        let mut doc = bare_doc();
        doc.profile.serving_lines = default_serving_lines();
        doc.set_participant_value("Readers", "Ann");
        doc.set_participant_value("Acolytes", "Bob");
        let old_lines = doc.profile.serving_lines.clone();
        let mut new_lines = old_lines.clone();
        new_lines.remove(5);
        new_lines.insert(0, "Candle lighters".to_string());
        let removed = migrate_serving_line_renames(
            &old_lines,
            &new_lines,
            &mut doc.participants,
            &mut doc.profile.serving_aliases,
        );
        assert!(removed.is_empty());
        assert_eq!(
            doc.participants.get("candle_lighters").map(String::as_str),
            Some("Bob")
        );
        assert_eq!(
            doc.participants.get("readers").map(String::as_str),
            Some("Ann")
        );
        assert!(!doc.participants.contains_key("acolytes"));
        doc.profile.serving_lines = new_lines;
        assert_eq!(doc.participant_value_for_name("Candle lighters"), "Bob");
        assert_eq!(resolve_fields("{Sunday.candle_lighters}", &doc), "Bob");
        assert_eq!(resolve_fields("{Sunday.readers}", &doc), "Ann");
    }

    #[test]
    fn a_rename_onto_a_filled_slug_keeps_both_values() {
        // Round 4, R4-2: when the renamed line's slug
        // already holds a different value (names typed
        // for that slug in an earlier week), both values
        // stay where they are: the target's value is
        // the fresher typing, and the renamed line's
        // value keeps its old key instead of being
        // discarded.
        let mut doc = bare_doc();
        doc.profile.serving_lines = default_serving_lines();
        doc.set_participant_value("Acolytes", "Bob");
        doc.participants
            .insert("candle_lighters".to_string(), "Zed".to_string());
        let old_lines = doc.profile.serving_lines.clone();
        let mut new_lines = old_lines.clone();
        new_lines[5] = "Candle lighters".to_string();
        let removed = migrate_serving_line_renames(
            &old_lines,
            &new_lines,
            &mut doc.participants,
            &mut doc.profile.serving_aliases,
        );
        assert!(removed.is_empty());
        assert_eq!(
            doc.participants.get("candle_lighters").map(String::as_str),
            Some("Zed")
        );
        assert_eq!(
            doc.participants.get("acolytes").map(String::as_str),
            Some("Bob")
        );
    }

    #[test]
    fn two_renames_with_a_reorder_cross_nothing_and_are_named() {
        // Round 5, F3, disposition changed in round 6
        // (R6-C, R6-5): Acolytes ("Bob") and Readers
        // ("Ann") are both renamed in one save, and
        // the renamed lines move across the list:
        // "Candle lighters" (the acolytes' new name)
        // lands at the top and "Lectors" (the readers'
        // new name) just under it, ahead of lines that
        // did not change. The renames cross anchors —
        // each partner sits in another segment — so
        // the pairing is ambiguous. Round 5 kept the
        // new names and reported the lines "unfollowed"
        // with their names stranded under the old
        // slugs; round 6 refuses the change as a
        // whole instead: nothing is paired, moved,
        // cleared, or aliased, the saved list keeps
        // the old names (the app does not apply the
        // edit), and the no-crossing, no-loss
        // properties this test was written for are
        // re-pinned under that disposition.
        let mut doc = bare_doc();
        doc.profile.serving_lines = default_serving_lines();
        doc.set_participant_value("Readers", "Ann");
        doc.set_participant_value("Acolytes", "Bob");
        let before = doc.participants.clone();
        let old_lines = doc.profile.serving_lines.clone();
        let new_lines: Vec<String> = [
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
        let migration = migrate_serving_line_renames_detailed(
            &old_lines,
            &new_lines,
            &mut doc.participants,
            &mut doc.profile.serving_aliases,
        );
        assert!(migration.ambiguous);
        assert!(migration.removed.is_empty());
        // Byte-identical afterwards: the map and the
        // alias record are untouched, and the saved
        // list still shows the old names, so the
        // names stay reachable by their lines and
        // their built-in tokens.
        assert_eq!(doc.participants, before);
        assert!(doc.profile.serving_aliases.is_empty());
        assert_eq!(doc.profile.serving_lines, old_lines);
        assert_eq!(doc.participant_value_for_name("Readers"), "Ann");
        assert_eq!(doc.participant_value_for_name("Acolytes"), "Bob");
        assert_eq!(resolve_fields("{Sunday.readers}", &doc), "Ann");
        assert_eq!(resolve_fields("{Sunday.acolytes}", &doc), "Bob");
        // The wrapper agrees: nothing was cleared,
        // and the map it was handed is untouched.
        let mut map = before.clone();
        let mut aliases: BTreeMap<String, String> = BTreeMap::new();
        let removed = migrate_serving_line_renames(&old_lines, &new_lines, &mut map, &mut aliases);
        assert!(removed.is_empty());
        assert_eq!(map, before);
        assert!(aliases.is_empty());
    }

    #[test]
    fn two_renames_in_place_pair_by_position() {
        // Round 5, R5-C: two renames in one save where
        // nothing moved (same length, every position
        // unchanged or the pair at that position) pair
        // by position and land on the right lines.
        let mut doc = bare_doc();
        doc.profile.serving_lines = default_serving_lines();
        doc.set_participant_value("Readers", "Ann");
        doc.set_participant_value("Acolytes", "Bob");
        let old_lines = doc.profile.serving_lines.clone();
        let mut new_lines = old_lines.clone();
        new_lines[0] = "Lectors".to_string();
        new_lines[5] = "Candle lighters".to_string();
        let migration = migrate_serving_line_renames_detailed(
            &old_lines,
            &new_lines,
            &mut doc.participants,
            &mut doc.profile.serving_aliases,
        );
        assert!(migration.removed.is_empty());
        assert!(!migration.ambiguous);
        doc.profile.serving_lines = new_lines;
        assert_eq!(doc.participant_value_for_name("Lectors"), "Ann");
        assert_eq!(doc.participant_value_for_name("Candle lighters"), "Bob");
        assert_eq!(resolve_fields("{Sunday.readers}", &doc), "Ann");
        assert_eq!(resolve_fields("{Sunday.acolytes}", &doc), "Bob");
        // The token bridge stays honest: each renamed
        // line's row and its built-in token agree.
        assert_eq!(
            doc.participant_value_for_name("Candle lighters"),
            resolve_fields("{Sunday.acolytes}", &doc)
        );
    }

    #[test]
    fn a_rename_then_a_front_insertion_never_crosses_tokens() {
        // Round 6, RT6-5: rename Readers to Lectors
        // (one save), then insert a custom line "Band"
        // at the FRONT of the list (a second save, a
        // pure addition). The old position bridge then
        // paired {Sunday.readers} with the line now at
        // the readers' position and printed the band's
        // names for it, and the Lectors' names under
        // Ushers. With the bridge deleted, the recorded
        // alias carries the token: readers resolves to
        // the renamed line, ushers to the Ushers, and a
        // token for a line that does not exist resolves
        // empty, never to another line's names.
        let mut doc = bare_doc();
        doc.profile.serving_lines = default_serving_lines();
        doc.set_participant_value("Readers", "Laura Lector");
        doc.set_participant_value("Ushers", "Uma Usher");
        // Save one: Readers renamed to Lectors, in
        // place. The pairing is recorded as an alias.
        let old_lines = doc.profile.serving_lines.clone();
        let mut renamed = old_lines.clone();
        renamed[0] = "Lectors".to_string();
        let migration = migrate_serving_line_renames_detailed(
            &old_lines,
            &renamed,
            &mut doc.participants,
            &mut doc.profile.serving_aliases,
        );
        assert!(!migration.ambiguous);
        doc.profile.serving_lines = renamed;
        assert_eq!(
            doc.profile
                .serving_aliases
                .get("readers")
                .map(String::as_str),
            Some("lectors")
        );
        // Save two: "Band" inserted at the front.
        let old_lines = doc.profile.serving_lines.clone();
        let mut with_band = old_lines.clone();
        with_band.insert(0, "Band".to_string());
        let migration = migrate_serving_line_renames_detailed(
            &old_lines,
            &with_band,
            &mut doc.participants,
            &mut doc.profile.serving_aliases,
        );
        assert!(!migration.ambiguous);
        assert!(migration.removed.is_empty());
        doc.profile.serving_lines = with_band;
        doc.set_participant_value("Band", "The Band Kids");
        assert_eq!(doc.participant_value("readers"), "Laura Lector");
        assert_eq!(doc.participant_value("ushers"), "Uma Usher");
        assert_eq!(resolve_fields("{Sunday.readers}", &doc), "Laura Lector");
        assert_eq!(resolve_fields("{Sunday.ushers}", &doc), "Uma Usher");
        assert_eq!(resolve_fields("{Sunday.band}", &doc), "The Band Kids");
        assert_eq!(doc.participant_value("choir"), "");
    }

    #[test]
    fn an_alias_chain_across_two_renames_resolves() {
        // Round 6, R6-C: Readers to Lectors in one
        // save, Lectors to "Readers' leaders" in the
        // next. The alias record holds the chain
        // (readers to lectors, lectors to
        // readers_leaders), and the built-in token
        // follows it to the names.
        let mut doc = bare_doc();
        doc.profile.serving_lines = default_serving_lines();
        doc.set_participant_value("Readers", "Ann");
        let old_lines = doc.profile.serving_lines.clone();
        let mut step_one = old_lines.clone();
        step_one[0] = "Lectors".to_string();
        migrate_serving_line_renames(
            &old_lines,
            &step_one,
            &mut doc.participants,
            &mut doc.profile.serving_aliases,
        );
        doc.profile.serving_lines = step_one;
        let old_lines = doc.profile.serving_lines.clone();
        let mut step_two = old_lines.clone();
        step_two[0] = "Readers' leaders".to_string();
        migrate_serving_line_renames(
            &old_lines,
            &step_two,
            &mut doc.participants,
            &mut doc.profile.serving_aliases,
        );
        doc.profile.serving_lines = step_two;
        assert_eq!(
            doc.profile
                .serving_aliases
                .get("readers")
                .map(String::as_str),
            Some("lectors")
        );
        assert_eq!(
            doc.profile
                .serving_aliases
                .get("lectors")
                .map(String::as_str),
            Some("readers_leaders")
        );
        assert_eq!(doc.participant_value("readers"), "Ann");
        assert_eq!(resolve_fields("{Sunday.readers}", &doc), "Ann");
        assert_eq!(doc.participant_value_for_name("Readers' leaders"), "Ann");
    }

    #[test]
    fn a_crafted_alias_cycle_resolves_empty_and_cannot_hang() {
        // Round 6, R6-C: a hand-edited profile whose
        // alias record loops (alpha to beta, beta to
        // alpha) must not hang resolution and must not
        // cross into another line's names. The pinned
        // rule: the walk stops at the first repeated
        // slug and the cycle resolves EMPTY.
        let mut doc = bare_doc();
        doc.profile
            .serving_aliases
            .insert("alpha".to_string(), "beta".to_string());
        doc.profile
            .serving_aliases
            .insert("beta".to_string(), "alpha".to_string());
        doc.set_participant_value("Greeters", "Gus");
        assert_eq!(doc.participant_value("alpha"), "");
        assert_eq!(doc.participant_value("beta"), "");
        // The honest line beside the cycle is unmoved.
        assert_eq!(doc.participant_value("greeters"), "Gus");
    }

    #[test]
    fn claimedness_follows_the_alias_record_both_ways() {
        // Round 6, R6-4: the built-in "Children's
        // message" was renamed to "Kids message" in an
        // earlier save, so the alias record claims its
        // slug for the Kids message line.
        let mut doc = bare_doc();
        doc.profile.serving_lines = default_serving_lines();
        doc.set_participant_value("Children's message", "Ms Dee");
        let old_lines = doc.profile.serving_lines.clone();
        let mut renamed = old_lines.clone();
        renamed[4] = "Kids message".to_string();
        migrate_serving_line_renames(
            &old_lines,
            &renamed,
            &mut doc.participants,
            &mut doc.profile.serving_aliases,
        );
        doc.profile.serving_lines = renamed;
        let lines_now = doc.profile.serving_lines.clone();
        // Another line taking the built-in's own name
        // is refused while Kids message holds the
        // claim: the token would print that line's
        // names as the children's message.
        let mut hijack = lines_now.clone();
        hijack[1] = "Children's message".to_string();
        let conflict =
            serving_line_slug_conflict(&lines_now, &hijack, &doc.profile.serving_aliases)
                .expect("ushers taking the claimed name conflicts");
        assert!(conflict.builtin);
        // FLAGGED, round 8 (R8-11): `first` used to be
        // the contested name itself and `second` the
        // built-in's display name, so this assertion
        // pair read "Children's message" on both
        // sides. The conflict now names the taking
        // line by the name it had and the holding
        // side by the name the bulletin currently
        // prints for the claimed line.
        assert_eq!(conflict.first, "Ushers");
        assert_eq!(conflict.second, "Kids message");
        // The claiming line itself may take the name
        // back, in either spelling.
        let mut back = lines_now.clone();
        back[4] = "Children's message".to_string();
        assert_eq!(
            serving_line_slug_conflict(&lines_now, &back, &doc.profile.serving_aliases),
            None
        );
        let mut back_plain = lines_now.clone();
        back_plain[4] = "Childrens Message".to_string();
        assert_eq!(
            serving_line_slug_conflict(&lines_now, &back_plain, &doc.profile.serving_aliases),
            None
        );
    }

    #[test]
    fn two_in_place_renames_and_an_added_line_all_take_effect() {
        // Round 6, R6-5: two lines renamed in place
        // and one line added, all in one save. The
        // round-5 pairing compared raw lists, read
        // the counts as mismatched, and followed
        // nothing; segment pairing anchors on Ushers,
        // so the renames pair in their segment and the
        // addition lands in its own.
        let mut doc = bare_doc();
        let old_lines: Vec<String> = ["Choir", "Bells", "Ushers"]
            .iter()
            .map(|n| n.to_string())
            .collect();
        doc.profile.serving_lines = old_lines.clone();
        doc.set_participant_value("Choir", "Ann");
        doc.set_participant_value("Bells", "Bob");
        let new_lines: Vec<String> = ["Choir team", "Bells team", "Ushers", "Drivers"]
            .iter()
            .map(|n| n.to_string())
            .collect();
        let migration = migrate_serving_line_renames_detailed(
            &old_lines,
            &new_lines,
            &mut doc.participants,
            &mut doc.profile.serving_aliases,
        );
        assert!(!migration.ambiguous);
        assert!(migration.removed.is_empty());
        doc.profile.serving_lines = new_lines;
        assert_eq!(doc.participant_value_for_name("Choir team"), "Ann");
        assert_eq!(doc.participant_value_for_name("Bells team"), "Bob");
        assert_eq!(doc.participant_value("choir"), "Ann");
        assert_eq!(doc.participant_value("bells"), "Bob");
    }

    #[test]
    fn a_delete_and_a_rename_in_one_save_report_the_delete_as_removed() {
        // Round 6, RT6-6: Readers renamed to Lectors
        // and Acolytes deleted outright, in one save.
        // The deleted line is reported REMOVED with
        // its names cleared — never "unfollowed" —
        // and the rename pairs in the same save.
        let mut doc = bare_doc();
        let old_lines: Vec<String> = ["Readers", "Ushers", "Acolytes"]
            .iter()
            .map(|n| n.to_string())
            .collect();
        doc.profile.serving_lines = old_lines.clone();
        doc.set_participant_value("Readers", "Ann");
        doc.set_participant_value("Acolytes", "Kid K");
        let new_lines: Vec<String> = ["Lectors", "Ushers"]
            .iter()
            .map(|n| n.to_string())
            .collect();
        let migration = migrate_serving_line_renames_detailed(
            &old_lines,
            &new_lines,
            &mut doc.participants,
            &mut doc.profile.serving_aliases,
        );
        assert!(!migration.ambiguous);
        assert_eq!(migration.removed, vec!["Acolytes".to_string()]);
        assert!(!doc.participants.contains_key("acolytes"));
        doc.profile.serving_lines = new_lines;
        assert_eq!(doc.participant_value_for_name("Lectors"), "Ann");
        assert_eq!(doc.participant_value("readers"), "Ann");
        assert_eq!(doc.participant_value("acolytes"), "");
    }

    #[test]
    fn a_removal_drops_its_aliases_and_a_reused_name_starts_fresh() {
        // Round 6, R6-C: Readers renamed to Lectors
        // records readers to lectors. Removing the
        // Lectors line drops that alias, so the
        // readers token stops resolving; and a NEW
        // line later taking the exact name "Lectors"
        // starts fresh — it inherits neither the
        // alias nor the removed line's names.
        let mut doc = bare_doc();
        doc.profile.serving_lines = default_serving_lines();
        doc.set_participant_value("Readers", "Ann");
        let old_lines = doc.profile.serving_lines.clone();
        let mut renamed = old_lines.clone();
        renamed[0] = "Lectors".to_string();
        migrate_serving_line_renames(
            &old_lines,
            &renamed,
            &mut doc.participants,
            &mut doc.profile.serving_aliases,
        );
        doc.profile.serving_lines = renamed;
        assert_eq!(doc.participant_value("readers"), "Ann");
        // Remove the renamed line outright.
        let old_lines = doc.profile.serving_lines.clone();
        let mut without = old_lines.clone();
        without.remove(0);
        let migration = migrate_serving_line_renames_detailed(
            &old_lines,
            &without,
            &mut doc.participants,
            &mut doc.profile.serving_aliases,
        );
        assert_eq!(migration.removed, vec!["Lectors".to_string()]);
        assert!(!doc.profile.serving_aliases.contains_key("readers"));
        doc.profile.serving_lines = without;
        assert_eq!(doc.participant_value("readers"), "");
        assert_eq!(doc.participant_value("lectors"), "");
        // A new line reusing the exact name starts
        // fresh: no names, and the old token does not
        // resurrect them onto it.
        let old_lines = doc.profile.serving_lines.clone();
        let mut reused = old_lines.clone();
        reused.insert(0, "Lectors".to_string());
        migrate_serving_line_renames(
            &old_lines,
            &reused,
            &mut doc.participants,
            &mut doc.profile.serving_aliases,
        );
        doc.profile.serving_lines = reused;
        assert_eq!(doc.participant_value_for_name("Lectors"), "");
        assert_eq!(doc.participant_value("readers"), "");
    }

    #[test]
    fn a_profile_without_the_alias_record_loads_with_none() {
        // Round 6, R6-C: profiles written before the
        // alias record existed carry no
        // `serving_aliases` key and load with an empty
        // record.
        let json = r#"{"name":"Grace","address":"","service_times":"",
            "website":"","giving_url":"","pastor":""}"#;
        let profile: ChurchProfile = serde_json::from_str(json).expect("old profile shape");
        assert!(profile.serving_aliases.is_empty());
        assert_eq!(profile.serving_lines, default_serving_lines());
    }

    #[test]
    fn slug_twins_in_one_list_are_a_conflict() {
        // Round 5, RT5-4 (1): "Prayer Team" and
        // "Prayer-Team" pass exact-name dedupe but
        // derive one slug, so they would share one
        // storage slot.
        let old_lines = default_serving_lines();
        let mut new_lines = old_lines.clone();
        new_lines.push("Prayer Team".to_string());
        new_lines.push("Prayer-Team".to_string());
        let conflict = serving_line_slug_conflict(&old_lines, &new_lines, &BTreeMap::new())
            .expect("slug twins conflict");
        assert_eq!(conflict.first, "Prayer Team");
        assert_eq!(conflict.second, "Prayer-Team");
        assert_eq!(conflict.slug, "prayer_team");
        assert!(!conflict.builtin);
        // Lookalike but distinct slugs are no
        // conflict: "Prayer Team" and "Prayer Teams"
        // save normally.
        let mut fine_lines = old_lines.clone();
        fine_lines.push("Prayer Team".to_string());
        fine_lines.push("Prayer Teams".to_string());
        assert_eq!(
            serving_line_slug_conflict(&old_lines, &fine_lines, &BTreeMap::new()),
            None
        );
        // A plain reorder of the built-ins is no
        // conflict either.
        let mut swapped = old_lines.clone();
        swapped.swap(0, 5);
        assert_eq!(
            serving_line_slug_conflict(&old_lines, &swapped, &BTreeMap::new()),
            None
        );
    }

    #[test]
    fn a_name_taking_a_builtin_slug_is_a_conflict() {
        // Round 5, RT5-4 (2): an added "Childrens
        // Message" derives the built-in Children's
        // message canonical slug and would hijack its
        // row and its printed token.
        let old_lines = default_serving_lines();
        let mut added = old_lines.clone();
        added.push("Childrens Message".to_string());
        let conflict = serving_line_slug_conflict(&old_lines, &added, &BTreeMap::new())
            .expect("alias addition conflicts");
        assert_eq!(conflict.first, "Childrens Message");
        assert_eq!(conflict.second, "Children's message");
        assert_eq!(conflict.slug, "childrens_message");
        assert!(conflict.builtin);
        // Round 8 (R8-11): the taking line here is a
        // new line, and the conflict says so.
        assert!(conflict.taker_is_new);
        // Round 5, RT5-4 (3): renaming Acolytes onto
        // that name is the same taking, by rename.
        let mut renamed = old_lines.clone();
        renamed[5] = "Childrens Message".to_string();
        let conflict = serving_line_slug_conflict(&old_lines, &renamed, &BTreeMap::new())
            .expect("alias rename conflicts");
        // FLAGGED, round 8 (R8-11): `first` used to be
        // the contested name as typed; the conflict
        // now names the taking line by the name it
        // had — Acolytes — while the holding side
        // keeps the name the bulletin prints for the
        // claimed line.
        assert_eq!(conflict.first, "Acolytes");
        assert_eq!(conflict.second, "Children's message");
        assert!(conflict.builtin);
        assert!(!conflict.taker_is_new);
        // The built-in renaming ITSELF to the same
        // words without the apostrophe is the line
        // itself: no conflict, and after the save its
        // row and its token read the same names.
        let mut itself = old_lines.clone();
        itself[4] = "Childrens Message".to_string();
        assert_eq!(
            serving_line_slug_conflict(&old_lines, &itself, &BTreeMap::new()),
            None
        );
        let mut doc = bare_doc();
        doc.profile.serving_lines = old_lines.clone();
        doc.set_participant_value("Children's message", "Ms. Patty");
        migrate_serving_line_renames(
            &old_lines,
            &itself,
            &mut doc.participants,
            &mut doc.profile.serving_aliases,
        );
        doc.profile.serving_lines = itself;
        assert_eq!(
            doc.participant_value_for_name("Childrens Message"),
            "Ms. Patty"
        );
        assert_eq!(
            resolve_fields("{Sunday.childrens_message}", &doc),
            "Ms. Patty"
        );
        // The built-in under its own name is itself.
        assert_eq!(
            serving_line_slug_conflict(&old_lines, &default_serving_lines(), &BTreeMap::new()),
            None
        );
    }

    #[test]
    fn a_claimant_renaming_back_inside_an_ambiguous_edit_is_not_its_own_conflict() {
        // Round 7 (review m1, R7-9): the old list's
        // "Kids message" claims childrens_message
        // through the recorded alias. The new list
        // renames it back to "Children's message" and
        // also renames and moves Readers and Acolytes
        // across anchors, so the pairing is ambiguous
        // and its pairs are discarded. The conflict
        // check used to lose the claimant's self-excuse
        // with those pairs and report the line as
        // conflicting with itself ("'Children's
        // message' would take the place of the built-in
        // line 'Children's message'..."). The verdict
        // for this edit is now NOT a conflict at all:
        // the save is still refused, by the ambiguous
        // path, whose kinds name the renames and the
        // crossing that actually block it.
        let old_lines: Vec<String> = [
            "Readers",
            "Ushers",
            "Greeters",
            "Communion assistants",
            "Kids message",
            "Acolytes",
        ]
        .iter()
        .map(|n| n.to_string())
        .collect();
        let mut aliases: BTreeMap<String, String> = BTreeMap::new();
        aliases.insert("childrens_message".to_string(), "kids_message".to_string());
        let new_lines: Vec<String> = [
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
        assert_eq!(
            serving_line_slug_conflict(&old_lines, &new_lines, &aliases),
            None,
            "the claimant renaming back to its built-in name is not a self-conflict"
        );
        let mut participants: BTreeMap<String, String> = BTreeMap::new();
        let mut alias_record = aliases.clone();
        let migration = migrate_serving_line_renames_detailed(
            &old_lines,
            &new_lines,
            &mut participants,
            &mut alias_record,
        );
        assert!(
            migration.ambiguous,
            "the save is still refused, as ambiguous"
        );
        assert_eq!(
            migration.ambiguous_kinds,
            Some(ServingLineEditKinds {
                renamed: true,
                removed: false,
                added: false,
                moved: true,
            })
        );
        // The standing pin beside it: a DIFFERENT line
        // taking the built-in's name in the same kind
        // of edit is still a hijack and still fires.
        let hijack: Vec<String> = [
            "Lectors",
            "Ushers",
            "Greeters",
            "Communion assistants",
            "Kids message",
            "Children's message",
        ]
        .iter()
        .map(|n| n.to_string())
        .collect();
        let conflict = serving_line_slug_conflict(&old_lines, &hijack, &aliases)
            .expect("a different line taking the name still conflicts");
        assert!(conflict.builtin);
        // FLAGGED, round 8 (R8-11): `first` used to be
        // the contested name itself; the conflict now
        // names the taking line by the name it had
        // (Acolytes, paired with the name inside its
        // segment) and the holding side by the name
        // the bulletin currently prints for the
        // claimed line ("Kids message").
        assert_eq!(conflict.first, "Acolytes");
        assert_eq!(conflict.second, "Kids message");
        assert_eq!(conflict.slug, "children_s_message");
    }

    #[test]
    fn ambiguous_kinds_count_only_segments_the_pairing_could_not_resolve() {
        // Round 8 (R8-10, all three lanes): the
        // `moved` tally used to increment for
        // removal-only and addition-only segments
        // too, so an ambiguous rename segment plus a
        // clean removal elsewhere claimed a move that
        // never happened, and the addition in the
        // first shape below was omitted while the
        // absent move was claimed. Resolved segments
        // now contribute their own kinds only.
        let kinds_of = |old: &[&str], new: &[&str]| {
            let old_lines: Vec<String> = old.iter().map(|n| n.to_string()).collect();
            let new_lines: Vec<String> = new.iter().map(|n| n.to_string()).collect();
            let mut participants: BTreeMap<String, String> = BTreeMap::new();
            let mut aliases: BTreeMap<String, String> = BTreeMap::new();
            let migration = migrate_serving_line_renames_detailed(
                &old_lines,
                &new_lines,
                &mut participants,
                &mut aliases,
            );
            assert!(migration.ambiguous, "the refusal itself stands");
            migration.ambiguous_kinds.expect("kinds on a refusal")
        };
        // The red team's S-C: one rename segment, one
        // clean removal, one clean addition; nothing
        // crossed an anchor.
        assert_eq!(
            kinds_of(
                &["Alpha", "Old One", "Old Two", "Beta", "Gone Line", "Gamma"],
                &["Alpha", "New One", "Beta", "Gamma", "Fresh Line"],
            ),
            ServingLineEditKinds {
                renamed: true,
                removed: true,
                added: true,
                moved: false,
            }
        );
        // The red team's S-D: rename segment plus a
        // removal only — no move, and no addition.
        assert_eq!(
            kinds_of(
                &["Alpha", "Old One", "Old Two", "Beta", "Gone", "Gamma"],
                &["Alpha", "New One", "Beta", "Gamma"],
            ),
            ServingLineEditKinds {
                renamed: true,
                removed: true,
                added: false,
                moved: false,
            }
        );
        // The verify lane's D8 shape: an ambiguous
        // rename segment between two anchors, plus a
        // line removed in its own segment after the
        // last anchor.
        assert_eq!(
            kinds_of(
                &["Alpha", "Choir", "Bells", "Beta", "Solo"],
                &["Alpha", "Choir team", "Beta"],
            ),
            ServingLineEditKinds {
                renamed: true,
                removed: true,
                added: false,
                moved: false,
            }
        );
        // The canonical crossed-renames shape keeps
        // its kinds exactly: two renames crossing
        // anchors, nothing removed or added. (The
        // Acolytes line's old-only segment counts
        // toward `moved` here because the first
        // segment holds more new names than old ones:
        // one of those names' partners — the Acolytes
        // line itself — must sit in another segment.)
        assert_eq!(
            kinds_of(
                &[
                    "Readers",
                    "Ushers",
                    "Greeters",
                    "Communion assistants",
                    "Children's message",
                    "Acolytes",
                ],
                &[
                    "Candle lighters",
                    "Lectors",
                    "Ushers",
                    "Greeters",
                    "Communion assistants",
                    "Children's message",
                ],
            ),
            ServingLineEditKinds {
                renamed: true,
                removed: false,
                added: false,
                moved: true,
            }
        );
    }

    #[test]
    fn a_slug_conflict_names_the_taking_line_and_the_current_holder() {
        // Round 8 (R8-11, review R8-R8 + red team
        // RT8-7/RT8-8): around the built-in's own
        // name, the conflict used to name the built-in
        // on both sides, excuse a hijack that merely
        // shared the unpaired sets, and miss a
        // claimant the pairing guessed across inside
        // an equal-count segment. The judgment now
        // works from the pairing's best evidence. In
        // every shape here "Kids message" claims
        // childrens_message through the recorded
        // alias, so the holding side is "Kids
        // message" — the name the bulletin currently
        // prints for that line.
        let mut aliases: BTreeMap<String, String> = BTreeMap::new();
        aliases.insert("childrens_message".to_string(), "kids_message".to_string());
        let lines =
            |names: &[&str]| -> Vec<String> { names.iter().map(|n| n.to_string()).collect() };
        // S-A2: the segment pairs in order — Kids
        // message to "Story time", Choir to
        // "Children's message". Choir's line is the
        // taker, named by the name it had.
        let old_lines = lines(&["Readers", "Kids message", "Choir", "Ushers"]);
        let new_lines = lines(&["Readers", "Story time", "Children's message", "Ushers"]);
        let conflict = serving_line_slug_conflict(&old_lines, &new_lines, &aliases)
            .expect("choir taking the claimed name conflicts");
        assert!(conflict.builtin);
        assert_eq!(conflict.first, "Choir");
        assert_eq!(conflict.second, "Kids message");
        assert!(!conflict.taker_is_new);
        // S-B: the claimant renames back while another
        // (new) line takes its freed name in the same
        // save. The rename-back line is not accused of
        // conflicting with itself: the taker is a new
        // line, and the holder is "Kids message", so
        // no side names the contested name
        // unqualified.
        let old_lines = lines(&["Readers", "Kids message", "Ushers"]);
        let new_lines = lines(&["Readers", "Children's message", "Kids message", "Ushers"]);
        let conflict = serving_line_slug_conflict(&old_lines, &new_lines, &aliases)
            .expect("the new line taking the claimed name conflicts");
        assert!(conflict.builtin);
        assert_eq!(conflict.first, "Children's message");
        assert_eq!(conflict.second, "Kids message");
        assert!(conflict.taker_is_new);
        // S-A: a genuine hijack sharing the unpaired
        // sets. The segment holds old [Kids message,
        // Choir] against new [Story time, Children's
        // message, Bonus line]: the claimant's own
        // in-segment position pairs it with "Story
        // time", and Choir's line sits at the
        // contested name's position. The membership
        // excuse used to excuse this; the conflict
        // now names the hijack, and the save is still
        // refused (by the migration, as ambiguous).
        let old_lines = lines(&["Readers", "Kids message", "Choir", "Ushers"]);
        let new_lines = lines(&[
            "Readers",
            "Story time",
            "Children's message",
            "Bonus line",
            "Ushers",
        ]);
        let conflict = serving_line_slug_conflict(&old_lines, &new_lines, &aliases)
            .expect("the hijack is named, not excused");
        assert!(conflict.builtin);
        assert_eq!(conflict.first, "Choir");
        assert_eq!(conflict.second, "Kids message");
        let mut participants: BTreeMap<String, String> = BTreeMap::new();
        let mut alias_record = aliases.clone();
        let migration = migrate_serving_line_renames_detailed(
            &old_lines,
            &new_lines,
            &mut participants,
            &mut alias_record,
        );
        assert!(migration.ambiguous, "the save is still refused");
        // The review's crossed-segment shape: inside
        // an equal-count segment the pairing pairs
        // Kids message with "Other name" and Other
        // line with "Children's message", while a
        // different segment is ambiguous. Membership
        // found the claimant in neither a pair to the
        // name nor the leftover sets and printed the
        // self-conflict sentence; the positional
        // evidence reports the conflict against Other
        // line instead.
        let old_lines = lines(&[
            "Alpha",
            "Kids message",
            "Other line",
            "Beta",
            "Old One",
            "Old Two",
            "Gamma",
        ]);
        let new_lines = lines(&[
            "Alpha",
            "Other name",
            "Children's message",
            "Beta",
            "New One",
            "Gamma",
        ]);
        let conflict = serving_line_slug_conflict(&old_lines, &new_lines, &aliases)
            .expect("the crossed segment's taker is named");
        assert!(conflict.builtin);
        assert_eq!(conflict.first, "Other line");
        assert_eq!(conflict.second, "Kids message");
    }

    #[test]
    fn a_removed_line_loses_its_names_and_is_reported() {
        // Round 4, RT-6c: deleting a line from the
        // profile list used to leave its names in the
        // map, invisible in every list yet still
        // printing through the line's token. The save
        // now clears them, and the migration returns
        // the line's name so the app can say so.
        let mut doc = bare_doc();
        doc.profile.serving_lines = default_serving_lines();
        doc.set_participant_value("Readers", "Ana Ruiz");
        doc.set_participant_value("Acolytes", "Sam, Jo");
        let old_lines = doc.profile.serving_lines.clone();
        let mut new_lines = old_lines.clone();
        new_lines.remove(5);
        let removed = migrate_serving_line_renames(
            &old_lines,
            &new_lines,
            &mut doc.participants,
            &mut doc.profile.serving_aliases,
        );
        assert_eq!(removed, vec!["Acolytes".to_string()]);
        assert!(!doc.participants.contains_key("acolytes"));
        doc.profile.serving_lines = new_lines;
        assert_eq!(doc.participant_value("acolytes"), "");
        assert_eq!(resolve_fields("{Sunday.acolytes}", &doc), "");
        assert_eq!(
            doc.participants.get("readers").map(String::as_str),
            Some("Ana Ruiz")
        );
        // A line removed while it holds no names is
        // not reported: nothing was removed.
        let old_lines = doc.profile.serving_lines.clone();
        let mut new_lines = old_lines.clone();
        new_lines.retain(|n| n != "Greeters");
        let removed = migrate_serving_line_renames(
            &old_lines,
            &new_lines,
            &mut doc.participants,
            &mut doc.profile.serving_aliases,
        );
        assert!(removed.is_empty());
    }

    #[test]
    fn a_rename_onto_an_existing_line_clears_the_merged_names() {
        // Round 4, RT-6b: renaming "Readers" onto the
        // already-present "Ushers" leaves Ushers' own
        // names in place, and the merged-away line's
        // names are cleared and reported instead of
        // vanishing silently.
        let mut doc = bare_doc();
        doc.profile.serving_lines = default_serving_lines();
        doc.set_participant_value("Readers", "Ann");
        doc.set_participant_value("Ushers", "Uma");
        let old_lines = doc.profile.serving_lines.clone();
        let mut new_lines = old_lines.clone();
        new_lines[0] = "Ushers".to_string();
        let removed = migrate_serving_line_renames(
            &old_lines,
            &new_lines,
            &mut doc.participants,
            &mut doc.profile.serving_aliases,
        );
        assert_eq!(removed, vec!["Readers".to_string()]);
        assert!(!doc.participants.contains_key("readers"));
        assert_eq!(
            doc.participants.get("ushers").map(String::as_str),
            Some("Uma")
        );
        doc.profile.serving_lines = new_lines;
        assert_eq!(doc.participant_value_for_name("Ushers"), "Uma");
    }

    #[test]
    fn childrens_message_stores_under_its_shown_slug_and_resolves_by_its_token() {
        // The shown name derives "children_s_message"; the
        // permanent token is {Sunday.childrens_message}.
        // The alias table bridges them.
        let mut doc = bare_doc();
        doc.set_participant_value("Children's message", "Ms. Dee");
        assert_eq!(
            doc.participants
                .get("children_s_message")
                .map(String::as_str),
            Some("Ms. Dee")
        );
        assert_eq!(
            resolve_fields("{Sunday.childrens_message}", &doc),
            "Ms. Dee"
        );
        assert_eq!(
            doc.participant_value_for_name("Children's message"),
            "Ms. Dee"
        );
    }

    #[test]
    fn web_hidden_paragraphs_leave_only_the_web_copy() {
        let mut doc = bare_doc();
        let mut hidden = Paragraph::plain("body", "Timmy and June, front row");
        hidden.web_hidden = true;
        doc.stories.push(Story {
            id: "news".into(),
            paragraphs: vec![Paragraph::plain("body", "Welcome."), hidden],
            keep_through: None,
        });
        assert_eq!(doc.web_hidden_count(), 1);
        let (web_doc, removed) = doc.without_web_hidden();
        assert_eq!(removed, 1);
        assert_eq!(web_doc.stories[0].paragraphs.len(), 1);
        // The working document itself is untouched.
        assert_eq!(doc.stories[0].paragraphs.len(), 2);
        assert_eq!(web_hidden_left_out_sentence(0), "");
        assert_eq!(
            web_hidden_left_out_sentence(1),
            " 1 paragraph was left out of the web page; it is for the room, not the web."
        );
        assert_eq!(
            web_hidden_left_out_sentence(2),
            " 2 paragraphs were left out of the web page; they are for the room, not the web."
        );
    }

    #[test]
    fn web_hidden_serializes_only_when_set() {
        // Old paragraphs carry no web_hidden key and load
        // as web-visible; a flagged one round-trips; an
        // unflagged one writes no key, so old files keep
        // their exact byte shape.
        let old: Paragraph =
            serde_json::from_str(r#"{"style":"body","text":"Hello"}"#).expect("old shape");
        assert!(!old.web_hidden);
        let plain_json =
            serde_json::to_string(&Paragraph::plain("body", "Hello")).expect("serialize");
        assert!(!plain_json.contains("web_hidden"), "{plain_json}");
        let mut flagged = Paragraph::plain("body", "Hello");
        flagged.web_hidden = true;
        let json = serde_json::to_string(&flagged).expect("serialize");
        assert!(json.contains("\"web_hidden\":true"), "{json}");
        let back: Paragraph = serde_json::from_str(&json).expect("round trip");
        assert!(back.web_hidden);
    }

    #[test]
    fn spoken_only_paragraphs_leave_the_publication_copy_only() {
        let mut doc = bare_doc();
        let mut spoken = Paragraph::plain("body", "For Mary, quietly");
        spoken.spoken_only = true;
        doc.stories.push(Story {
            id: "prayers".into(),
            paragraphs: vec![
                Paragraph::plain("subheading", "Prayers"),
                spoken,
                Paragraph::plain("body", "For the food bank"),
            ],
            keep_through: None,
        });
        assert_eq!(doc.spoken_only_count(), 1);
        let (pub_doc, removed) = doc.without_spoken_only();
        assert_eq!(removed, 1);
        let texts: Vec<String> = pub_doc.stories[0]
            .paragraphs
            .iter()
            .map(|p| p.text())
            .collect();
        assert_eq!(texts, vec!["Prayers", "For the food bank"]);
        // The working document itself is untouched.
        assert_eq!(doc.stories[0].paragraphs.len(), 3);
        assert_eq!(spoken_only_left_out_sentence(0), "");
        assert_eq!(
            spoken_only_left_out_sentence(1),
            " 1 name is spoken only and was left out of this copy."
        );
        assert_eq!(
            spoken_only_left_out_sentence(2),
            " 2 names are spoken only and were left out of this copy."
        );
    }

    #[test]
    fn paragraphs_written_before_the_flag_load_as_printable() {
        // Both paragraph shapes as old files carry them: the
        // text shape and the runs shape, neither with a
        // spoken_only key.
        let old_text: Paragraph =
            serde_json::from_str(r#"{"style":"body","text":"Hello"}"#).expect("old text shape");
        assert!(!old_text.spoken_only);
        assert_eq!(old_text.text(), "Hello");
        let old_runs: Paragraph =
            serde_json::from_str(r#"{"style":"body","runs":[{"text":"Hello"}]}"#)
                .expect("old runs shape");
        assert!(!old_runs.spoken_only);
        let flagged: Paragraph =
            serde_json::from_str(r#"{"style":"body","text":"Quiet","spoken_only":true}"#)
                .expect("flagged shape");
        assert!(flagged.spoken_only);
    }

    fn mapped_doc() -> Document {
        let mut fields = BTreeMap::new();
        fields.insert("occasion_name".to_string(), "Pentecost".to_string());
        Document {
            format_version: FORMAT_VERSION,
            title: "t".into(),
            profile: ChurchProfile {
                name: "Grace".into(),
                pastor: "Pastor Lee".into(),
                ..Default::default()
            },
            fields,
            styles: vec![],
            stories: vec![],
            panels: vec![],
            sheet: SheetSpec {
                preset: "letter-portrait".into(),
                size: Size::new(0, 0),
            },
            no_announcements: false,
            assets: Vec::new(),
            asset_bytes: Default::default(),
            occasion_time: None,
            wanted_fonts: Vec::new(),
            participants: BTreeMap::new(),
            inserts: Vec::new(),
            applied_contributions: Vec::new(),
        }
    }

    #[test]
    fn mapped_resolution_records_exact_spans() {
        let doc = mapped_doc();
        let raw = "Sermon: {Church.pastor} on {Sunday.name}";
        let r = resolve_fields_mapped(raw, &doc);
        assert_eq!(r.text, "Sermon: Pastor Lee on Pentecost");
        assert_eq!(r.spans.len(), 2);
        let s0 = &r.spans[0];
        assert_eq!(&raw[s0.raw.clone()], "{Church.pastor}");
        assert_eq!(&r.text[s0.resolved.clone()], "Pastor Lee");
        let s1 = &r.spans[1];
        assert_eq!(&raw[s1.raw.clone()], "{Sunday.name}");
        assert_eq!(&r.text[s1.resolved.clone()], "Pentecost");
        // The plain resolver and the mapped one agree.
        assert_eq!(resolve_fields(raw, &doc), r.text);
    }

    #[test]
    fn mapped_resolution_handles_empty_values_and_no_tokens() {
        let doc = mapped_doc();
        let r = resolve_fields_mapped("At {Church.website} now", &doc);
        assert_eq!(r.text, "At  now");
        assert_eq!(r.spans.len(), 1);
        assert_eq!(r.spans[0].resolved.len(), 0);
        let plain = resolve_fields_mapped("No tokens here.", &doc);
        assert_eq!(plain.text, "No tokens here.");
        assert!(plain.spans.is_empty());
    }

    #[test]
    fn mapped_resolution_never_re_resolves_inside_a_value() {
        // A church literally named with braces is data: the
        // single pass must not treat the value as a template.
        let mut doc = mapped_doc();
        doc.profile.name = "{Sunday.name} Church".into();
        let r = resolve_fields_mapped("{Church.name}", &doc);
        assert_eq!(r.text, "{Sunday.name} Church");
        assert_eq!(r.spans.len(), 1);
    }
}

#[cfg(test)]
mod guide_tests {
    use super::*;

    #[test]
    fn snapping_grabs_nearby_targets_only() {
        let targets = [0, 90_000, 396_000];
        assert_eq!(snap_to(91_000, &targets, 3_000), 90_000);
        assert_eq!(snap_to(95_000, &targets, 3_000), 95_000);
        assert_eq!(snap_to(-2_000, &targets, 3_000), 0);
    }

    #[test]
    fn inches_round_trip() {
        assert_eq!(inches_to_mpt(1.25), 90_000);
        assert_eq!(mpt_to_inches(90_000), 1.25);
        assert_eq!(inches_to_mpt(0.0), 0);
    }

    #[test]
    fn parses_office_worker_numbers() {
        assert_eq!(parse_inches("1.25"), Some(1.25));
        assert_eq!(parse_inches("2"), Some(2.0));
        assert_eq!(parse_inches("1/2"), Some(0.5));
        assert_eq!(parse_inches("1 1/4"), Some(1.25));
        assert_eq!(parse_inches("  3/8 \""), Some(0.375));
        assert_eq!(parse_inches("abc"), None);
        assert_eq!(parse_inches(""), None);
        assert_eq!(parse_inches("-1"), None);
        assert_eq!(parse_inches("1/0"), None);
    }
}

#[cfg(test)]
mod date_tests {
    use super::*;

    fn d(year: i32, month: u32, day: u32) -> SimpleDate {
        SimpleDate { year, month, day }
    }

    #[test]
    fn weekday_math_matches_known_days() {
        // 1970-01-01 was a Thursday; 2026-10-18 is a Sunday;
        // 2026-02-18 (Ash Wednesday) is a Wednesday;
        // 2000-01-01 was a Saturday.
        assert_eq!(d(1970, 1, 1).weekday(), 4);
        assert_eq!(d(2026, 10, 18).weekday(), 0);
        assert_eq!(d(2026, 2, 18).weekday(), 3);
        assert_eq!(d(2000, 1, 1).weekday(), 6);
        assert!(d(2028, 2, 29).is_valid());
        assert!(!d(2026, 2, 29).is_valid());
    }

    #[test]
    fn flexible_dates_read_every_office_format() {
        // Today: Thursday 2026-10-15.
        let today = d(2026, 10, 15);
        let want = Some(d(2026, 10, 18));
        // ISO.
        assert_eq!(parse_flexible_date("2026-10-18", today), want);
        // US numeric, four- and two-digit years, both separators.
        assert_eq!(parse_flexible_date("10/18/2026", today), want);
        assert_eq!(parse_flexible_date("10/18/26", today), want);
        assert_eq!(parse_flexible_date("10-18-2026", today), want);
        // Month names, both orders, with and without the year.
        assert_eq!(parse_flexible_date("Oct 18", today), want);
        assert_eq!(parse_flexible_date("October 18, 2026", today), want);
        assert_eq!(parse_flexible_date("18 October 2026", today), want);
        assert_eq!(parse_flexible_date("Oct. 18th, 2026", today), want);
        // A correct weekday in front is fine in any of them.
        assert_eq!(parse_flexible_date("Sun, Oct 18, 2026", today), want);
        assert_eq!(parse_flexible_date("Sunday, October 18, 2026", today), want);
        assert_eq!(
            parse_flexible_date("Wed, Feb 18, 2026", today),
            Some(d(2026, 2, 18))
        );
    }

    #[test]
    fn flexible_dates_reject_rather_than_guess() {
        let today = d(2026, 10, 15);
        // 2026-10-18 is a Sunday, not a Wednesday: a wrong
        // stated weekday never reads.
        assert_eq!(parse_flexible_date("Wed, Oct 18, 2026", today), None);
        assert_eq!(
            parse_flexible_date("Wednesday, October 18, 2026", today),
            None
        );
        // Impossible and unparseable dates do not read.
        assert_eq!(parse_flexible_date("2026-02-30", today), None);
        assert_eq!(parse_flexible_date("13/1/2026", today), None);
        assert_eq!(parse_flexible_date("next Friday", today), None);
        assert_eq!(parse_flexible_date("18", today), None);
        assert_eq!(parse_flexible_date("", today), None);
    }

    #[test]
    fn flexible_dates_without_a_year_mean_the_coming_occurrence() {
        let today = d(2026, 10, 15);
        // Later this year stays this year.
        assert_eq!(parse_flexible_date("Dec 25", today), Some(d(2026, 12, 25)));
        // Today itself counts.
        assert_eq!(parse_flexible_date("10/15", today), Some(d(2026, 10, 15)));
        // Already gone by: next year.
        assert_eq!(parse_flexible_date("Oct 11", today), Some(d(2027, 10, 11)));
        assert_eq!(parse_flexible_date("1/1", today), Some(d(2027, 1, 1)));
        // Leap day skips to the next year it exists.
        assert_eq!(parse_flexible_date("Feb 29", today), Some(d(2028, 2, 29)));
    }
}

#[cfg(test)]
mod occasion_tests {
    use super::*;
    use std::collections::BTreeMap;

    fn bare_doc() -> Document {
        Document {
            format_version: FORMAT_VERSION,
            title: "t".into(),
            profile: ChurchProfile::default(),
            fields: BTreeMap::new(),
            styles: vec![],
            stories: vec![],
            panels: vec![],
            sheet: SheetSpec {
                preset: "letter-portrait".into(),
                size: Size::new(0, 0),
            },
            no_announcements: false,
            assets: Vec::new(),
            asset_bytes: Default::default(),
            occasion_time: None,
            wanted_fonts: Vec::new(),
            participants: BTreeMap::new(),
            inserts: Vec::new(),
            applied_contributions: Vec::new(),
        }
    }

    #[test]
    fn old_field_keys_rename_on_migration() {
        let mut doc = bare_doc();
        doc.fields.insert("sunday_date".into(), "2026-10-18".into());
        doc.fields
            .insert("sunday_date_long".into(), "October 18, 2026".into());
        doc.fields.insert("sunday_name".into(), "Pentecost".into());
        doc.migrate_occasion_keys();
        assert_eq!(doc.fields["occasion_date"], "2026-10-18");
        assert_eq!(doc.fields["occasion_date_long"], "October 18, 2026");
        assert_eq!(doc.fields["occasion_name"], "Pentecost");
        assert!(!doc.fields.contains_key("sunday_date"));
        // A document already speaking the new names is left
        // exactly as it is (the new value wins over a stale
        // old one riding along).
        let mut doc = bare_doc();
        doc.fields
            .insert("occasion_date".into(), "2026-12-25".into());
        doc.fields.insert("sunday_date".into(), "2026-10-18".into());
        doc.migrate_occasion_keys();
        assert_eq!(doc.fields["occasion_date"], "2026-12-25");
        assert!(!doc.fields.contains_key("sunday_date"));
    }

    #[test]
    fn sunday_tokens_and_service_time_resolve_from_occasion_fields() {
        let mut doc = bare_doc();
        doc.fields
            .insert("occasion_date_long".into(), "October 18, 2026".into());
        doc.fields
            .insert("occasion_name".into(), "Pentecost".into());
        doc.occasion_time = Some("9:00 AM".into());
        assert_eq!(
            resolve_fields("{Sunday.name}, {Sunday.date} at {Service.time}", &doc),
            "Pentecost, October 18, 2026 at 9:00 AM"
        );
        // No time set: the token resolves to nothing, not a
        // leftover brace.
        doc.occasion_time = None;
        assert_eq!(resolve_fields("at {Service.time}!", &doc), "at !");
    }

    #[test]
    fn assets_store_bytes_by_hash_with_rights() {
        let mut doc = bare_doc();
        let rights = RightsRecord {
            source: "the church camera".into(),
            license: LicenseKind::ChurchOwned,
            ..Default::default()
        };
        let h1 = doc.add_asset(b"PNGDATA", AssetKind::Picture, "image/png", rights);
        assert_eq!(h1, sha256_hex(b"PNGDATA"));
        assert_eq!(doc.asset_data(&h1), Some(b"PNGDATA".as_slice()));
        assert_eq!(doc.assets.len(), 1);
        assert_eq!(doc.assets[0].kind, AssetKind::Picture);
        // The same bytes again are the same asset.
        let h2 = doc.add_asset(
            b"PNGDATA",
            AssetKind::Picture,
            "image/png",
            RightsRecord::default(),
        );
        assert_eq!(h1, h2);
        assert_eq!(doc.assets.len(), 1);
        // Rights and license kinds survive JSON (they ride in
        // document.json; the bytes never do).
        let json = serde_json::to_string(&doc).expect("json");
        assert!(!json.contains("PNGDATA"), "bytes stay out of the JSON");
        let back: Document = serde_json::from_str(&json).expect("parse");
        assert_eq!(back.assets[0].rights.license, LicenseKind::ChurchOwned);
        assert_eq!(back.assets[0].rights.source, "the church camera");
        assert!(back.asset_bytes.is_empty());
    }

    #[test]
    fn r5_add_asset_merges_the_rights_of_bytes_already_held() {
        // Round 5 (RT5-2): the same bytes arrive twice,
        // strong in different halves. The second record
        // used to be thrown away whole; now it fills
        // what the first leaves empty, and the first
        // record's noted values stand.
        let mut doc = bare_doc();
        let h1 = doc.add_asset(
            b"PNGDATA",
            AssetKind::Picture,
            "image/png",
            RightsRecord {
                source: "the church camera".into(),
                ..Default::default()
            },
        );
        let h2 = doc.add_asset(
            b"PNGDATA",
            AssetKind::Picture,
            "image/png",
            RightsRecord {
                title: "Harvest supper".into(),
                license: LicenseKind::CcBy("4.0".into()),
                ..Default::default()
            },
        );
        assert_eq!(h1, h2);
        assert_eq!(doc.assets.len(), 1);
        let rights = &doc.assets[0].rights;
        assert_eq!(rights.source, "the church camera");
        assert_eq!(rights.title, "Harvest supper");
        assert_eq!(rights.license, LicenseKind::CcBy("4.0".into()));
    }

    #[test]
    fn r5_merge_never_manufactures_a_web_permission() {
        // Round 5 (F2): the bulletin record notes
        // children in the picture and no permission;
        // the sheet record holds a credit line. The
        // merge must not complete the pair into a
        // noted permission.
        let bulletin = RightsRecord {
            children_present: Some(true),
            ..Default::default()
        };
        let sheet = RightsRecord {
            credit_text: "Photo by Lee".into(),
            ..Default::default()
        };
        let merged = merge_rights_records(&bulletin, &sheet);
        assert_eq!(merged.children_present, Some(true));
        assert_eq!(merged.credit_text, "");
        assert_eq!(merged.license, LicenseKind::Unknown);
        // A license that itself counts as a noted
        // permission is fenced the same way.
        let sheet_cc0 = RightsRecord {
            license: LicenseKind::Cc0,
            ..Default::default()
        };
        assert_eq!(
            merge_rights_records(&bulletin, &sheet_cc0).license,
            LicenseKind::Unknown
        );
        // Fills that say nothing about permission
        // still land in a fenced record.
        // FLAGGED, round 7 (R7-1): this pin used to
        // assert the CC BY license standing here. The
        // composed record notes children and holds no
        // credit line at all, so the standing license
        // claimed an attribution nothing in the record
        // carries — the composed state RT7-1 graded
        // BLOCKER. Under the composed togetherness
        // rule the title still lands and the license
        // now falls to Unknown; the round-7 mirror
        // test pins the same state from the other
        // direction.
        let sheet_fuller = RightsRecord {
            title: "Harvest supper".into(),
            license: LicenseKind::CcBy("4.0".into()),
            ..Default::default()
        };
        let merged = merge_rights_records(&bulletin, &sheet_fuller);
        assert_eq!(merged.title, "Harvest supper");
        assert_eq!(merged.license, LicenseKind::Unknown);
        // With no children note, nothing is fenced:
        // the ordinary fill (round 4, R4-5) carries
        // the credit line.
        let plain = RightsRecord::default();
        let merged = merge_rights_records(&plain, &sheet);
        assert_eq!(merged.credit_text, "Photo by Lee");
        // And a record whose permission is already
        // noted is not fenced at all.
        let noted = RightsRecord {
            children_present: Some(true),
            credit_text: "Photo by Lee, used with parent permission.".into(),
            ..Default::default()
        };
        let merged = merge_rights_records(&noted, &sheet_cc0);
        assert_eq!(merged.license, LicenseKind::Cc0);
    }

    #[test]
    fn r5_merge_assertion_fields_ride_only_with_a_real_fill() {
        // Round 5: asserted_by / asserted_at vouch for
        // a fill. An incoming record carrying only an
        // assertion changes nothing at all.
        let existing = RightsRecord {
            source: "the church camera".into(),
            ..Default::default()
        };
        let assertion_only = RightsRecord {
            asserted_by: "Sam".into(),
            asserted_at: "2026-10-16".into(),
            ..Default::default()
        };
        assert_eq!(merge_rights_records(&existing, &assertion_only), existing);
        // With a substantive fill from the same record,
        // the assertion rides along with it.
        // FLAGGED, round 7 (R7-8): the fill here used
        // to be a title. "Substantive" now means
        // rights-bearing — a credit line or a license
        // that itself stands — so the fill is a credit
        // line; a title alone no longer carries an
        // assertion, and the round-7 assertion test
        // pins that shape by name.
        let with_fill = RightsRecord {
            credit_text: "Photo by Jane Doe".into(),
            asserted_by: "Sam".into(),
            asserted_at: "2026-10-16".into(),
            ..Default::default()
        };
        let merged = merge_rights_records(&existing, &with_fill);
        assert_eq!(merged.credit_text, "Photo by Jane Doe");
        assert_eq!(merged.asserted_by, "Sam");
        assert_eq!(merged.asserted_at, "2026-10-16");
    }

    #[test]
    fn r6_merge_never_manufactures_a_permission_in_either_order() {
        // Round 6 (R6-1 / RT6-1): the round-5 fence faced
        // one way. A credit-bearing record standing first
        // and a children-noted record second composed a
        // permission no single record ever claimed. The
        // property is on the composed record now, so both
        // orders end the same way.
        let credit = RightsRecord {
            credit_text: "Photo by Lee".into(),
            ..Default::default()
        };
        let children = RightsRecord {
            children_present: Some(true),
            ..Default::default()
        };
        for merged in [
            merge_rights_records(&credit, &children),
            merge_rights_records(&children, &credit),
        ] {
            assert_eq!(merged.children_present, Some(true));
            assert_eq!(merged.credit_text, "");
            assert!(!rights_permission_noted(&merged));
        }
        // A permission license standing first is fenced
        // the same way, in either order.
        let cc0 = RightsRecord {
            license: LicenseKind::Cc0,
            ..Default::default()
        };
        let church_owned = RightsRecord {
            license: LicenseKind::ChurchOwned,
            ..Default::default()
        };
        for merged in [
            merge_rights_records(&cc0, &children),
            merge_rights_records(&children, &cc0),
            merge_rights_records(&church_owned, &children),
            merge_rights_records(&children, &church_owned),
        ] {
            assert_eq!(merged.children_present, Some(true));
            assert_eq!(merged.license, LicenseKind::Unknown);
            assert!(!rights_permission_noted(&merged));
        }
        // A credit arriving on a record that itself notes
        // children may stand: that record carried both.
        let children_with_credit = RightsRecord {
            credit_text: "Photo by Lee, used with parent permission.".into(),
            children_present: Some(true),
            ..Default::default()
        };
        let merged = merge_rights_records(&children, &children_with_credit);
        assert_eq!(
            merged.credit_text,
            "Photo by Lee, used with parent permission."
        );
        assert!(rights_permission_noted(&merged));
        // A children record that noted its own permission
        // keeps it and accepts other fills.
        let fuller = RightsRecord {
            title: "Harvest supper".into(),
            license: LicenseKind::Cc0,
            ..Default::default()
        };
        let merged = merge_rights_records(&children_with_credit, &fuller);
        assert_eq!(merged.title, "Harvest supper");
        assert_eq!(merged.license, LicenseKind::Cc0);
        assert_eq!(
            merged.credit_text,
            "Photo by Lee, used with parent permission."
        );
        // With no children note anywhere, the ordinary
        // split-strength merge (round 4, R4-5) still
        // carries both halves.
        let license_only = RightsRecord {
            license: LicenseKind::CcBy("4.0".into()),
            ..Default::default()
        };
        let merged = merge_rights_records(&credit, &license_only);
        assert_eq!(merged.credit_text, "Photo by Lee");
        assert_eq!(merged.license, LicenseKind::CcBy("4.0".into()));
        assert_eq!(merged.children_present, None);
    }

    #[test]
    fn r6_three_record_fold_never_composes_a_permission() {
        // Round 6 (R6-1): a fold of three records (a
        // credit record, a children record, a neutral
        // record) in mixed orders never composes a
        // permission, and the neutral fills still land.
        let hash = sha256_hex(b"R6FOLD");
        let credit = RightsRecord {
            credit_text: "Photo by Lee".into(),
            ..Default::default()
        };
        let children = RightsRecord {
            children_present: Some(true),
            ..Default::default()
        };
        let neutral = RightsRecord {
            title: "Harvest supper".into(),
            ..Default::default()
        };
        let asset = |rights: RightsRecord| Asset {
            sha256: hash.clone(),
            kind: AssetKind::Picture,
            media_type: "image/png".into(),
            rights,
        };
        for records in [
            vec![credit.clone(), children.clone(), neutral.clone()],
            vec![credit.clone(), neutral.clone(), children.clone()],
            vec![children.clone(), neutral.clone(), credit.clone()],
            vec![neutral.clone(), credit.clone(), children.clone()],
        ] {
            let mut assets: Vec<Asset> = records.into_iter().map(asset).collect();
            fold_duplicate_asset_records(&mut assets);
            assert_eq!(assets.len(), 1);
            let rights = &assets[0].rights;
            assert_eq!(rights.children_present, Some(true));
            assert_eq!(rights.title, "Harvest supper");
            assert!(!rights_permission_noted(rights), "{rights:?}");
        }
    }

    #[test]
    fn r6_merge_license_and_its_credit_travel_together() {
        // Round 6 (RT6-2): into a children-noted record,
        // the old fence blocked the incoming credit line
        // but let its CC BY license through, claiming a
        // license whose required attribution was dropped.
        // The pair travels together or not at all, while
        // fills that say nothing about the pair still land.
        let existing = RightsRecord {
            children_present: Some(true),
            ..Default::default()
        };
        let incoming = RightsRecord {
            title: "Church picnic".into(),
            creator: "Alice".into(),
            license: LicenseKind::CcBy("4.0".into()),
            credit_text: "Photo by Alice, CC BY 4.0".into(),
            ..Default::default()
        };
        let merged = merge_rights_records(&existing, &incoming);
        assert_eq!(merged.children_present, Some(true));
        assert_eq!(merged.credit_text, "");
        assert_eq!(merged.license, LicenseKind::Unknown);
        assert_eq!(merged.title, "Church picnic");
        assert_eq!(merged.creator, "Alice");
        // The refused pair counts as no fill at all: with
        // nothing else in the incoming record, the merge
        // changes nothing and no assertion rides in.
        let pair_only = RightsRecord {
            license: LicenseKind::CcBy("4.0".into()),
            credit_text: "Photo by Alice, CC BY 4.0".into(),
            asserted_by: "Sam".into(),
            asserted_at: "2026-10-16".into(),
            ..Default::default()
        };
        assert_eq!(merge_rights_records(&existing, &pair_only), existing);
        // When a credit line already stands on the
        // existing record, the license may fill: a credit
        // stands, which is not the harm above.
        let credited = RightsRecord {
            credit_text: "Photo by Alice, used with permission.".into(),
            ..Default::default()
        };
        let merged = merge_rights_records(&credited, &incoming);
        assert_eq!(merged.license, LicenseKind::CcBy("4.0".into()));
        assert_eq!(merged.credit_text, "Photo by Alice, used with permission.");
    }

    #[test]
    fn r6_add_asset_never_manufactures_a_permission() {
        // Round 6 (R6-1): add_asset composes through the
        // same merge. Bytes first held with a credit line
        // and later given a children note (and the other
        // order) never end up noting a permission.
        for first_credit in [true, false] {
            let mut doc = bare_doc();
            let credit = RightsRecord {
                credit_text: "Photo by Lee".into(),
                ..Default::default()
            };
            let children = RightsRecord {
                children_present: Some(true),
                ..Default::default()
            };
            let (first, second) = if first_credit {
                (credit, children)
            } else {
                (children, credit)
            };
            doc.add_asset(b"R6PIC", AssetKind::Picture, "image/png", first);
            doc.add_asset(b"R6PIC", AssetKind::Picture, "image/png", second);
            assert_eq!(doc.assets.len(), 1);
            let rights = &doc.assets[0].rights;
            assert_eq!(rights.children_present, Some(true));
            assert!(!rights_permission_noted(rights), "{rights:?}");
        }
    }

    #[test]
    fn r7_a_standing_license_loses_its_credit_and_falls() {
        // Round 7 (R7-1 / RT7-1, the RT7-1 probe
        // verbatim): the license already stands on the
        // children-bearing record and the only credit
        // arrives from a second record, where the fence
        // refuses it. The composed record used to keep
        // the license with an empty credit line — the
        // state RT6-2 was graded BLOCKER for, composed
        // in the mirror direction. The togetherness
        // rule judges the composed record now: the
        // license falls, the children note stands, for
        // both attribution licenses.
        let credit = RightsRecord {
            credit_text: "Photo by Alice, CC BY 4.0".into(),
            ..Default::default()
        };
        for license in [
            LicenseKind::CcBy("4.0".into()),
            LicenseKind::CcBySa("4.0".into()),
        ] {
            let existing = RightsRecord {
                license: license.clone(),
                children_present: Some(true),
                ..Default::default()
            };
            let merged = merge_rights_records(&existing, &credit);
            assert_eq!(merged.children_present, Some(true), "{license:?}");
            assert_eq!(merged.credit_text, "", "{license:?}");
            assert_eq!(merged.license, LicenseKind::Unknown, "{license:?}");
        }
        // Control (the R5b shape): a children-bearing
        // record whose OWN credit stands keeps its
        // license when an unrelated record merges in.
        let own_credit = RightsRecord {
            license: LicenseKind::CcBy("4.0".into()),
            credit_text: "Photo by Alice, CC BY 4.0".into(),
            children_present: Some(true),
            ..Default::default()
        };
        let unrelated = RightsRecord {
            title: "Picnic".into(),
            ..Default::default()
        };
        let merged = merge_rights_records(&own_credit, &unrelated);
        assert_eq!(merged.license, LicenseKind::CcBy("4.0".into()));
        assert_eq!(merged.credit_text, "Photo by Alice, CC BY 4.0");
        assert_eq!(merged.title, "Picnic");
        // Licenses that require no attribution are
        // untouched by the rule: a children-bearing
        // record's CC0 stands with an empty credit
        // line after an unrelated merge. (A CC0 record
        // is itself a noted permission, so a credit
        // arriving beside it stands under the round-6
        // bearer rule and cannot show this.)
        let cc0_bearer = RightsRecord {
            license: LicenseKind::Cc0,
            children_present: Some(true),
            ..Default::default()
        };
        let merged = merge_rights_records(&cc0_bearer, &unrelated);
        assert_eq!(merged.license, LicenseKind::Cc0);
        assert_eq!(merged.credit_text, "");
    }

    #[test]
    fn r7_a_dropped_license_leaves_a_vouched_license_standing() {
        // Round 7 (R7-1): the dropped license reverts
        // to what the merge holds without it. Here the
        // other record is itself children-bearing and
        // vouches a CC0 license, so when the CC BY
        // falls that license is what the merge holds.
        let ccby = RightsRecord {
            license: LicenseKind::CcBy("4.0".into()),
            children_present: Some(true),
            ..Default::default()
        };
        let cc0 = RightsRecord {
            license: LicenseKind::Cc0,
            children_present: Some(true),
            ..Default::default()
        };
        for merged in [
            merge_rights_records(&ccby, &cc0),
            merge_rights_records(&cc0, &ccby),
        ] {
            assert_eq!(merged.children_present, Some(true));
            assert_eq!(merged.credit_text, "");
            assert_eq!(merged.license, LicenseKind::Cc0);
        }
    }

    #[test]
    fn r7_three_record_fold_drops_the_orphaned_license() {
        // Round 7 (R7-1): the three-record fold
        // [children+CC BY, title, credit] through the
        // load fold's own funnel. The credit is refused
        // at the last merge, so the license that stood
        // from the first record falls with it; the
        // neutral title fill still lands.
        let hash = sha256_hex(b"R7FOLD");
        let children_ccby = RightsRecord {
            license: LicenseKind::CcBy("4.0".into()),
            children_present: Some(true),
            ..Default::default()
        };
        let titled = RightsRecord {
            title: "Picnic".into(),
            ..Default::default()
        };
        let credit = RightsRecord {
            credit_text: "Photo by Alice, CC BY 4.0".into(),
            ..Default::default()
        };
        let asset = |rights: RightsRecord| Asset {
            sha256: hash.clone(),
            kind: AssetKind::Picture,
            media_type: "image/png".into(),
            rights,
        };
        let mut assets: Vec<Asset> = vec![children_ccby, titled, credit]
            .into_iter()
            .map(asset)
            .collect();
        fold_duplicate_asset_records(&mut assets);
        assert_eq!(assets.len(), 1);
        let rights = &assets[0].rights;
        assert_eq!(rights.children_present, Some(true));
        assert_eq!(rights.license, LicenseKind::Unknown);
        assert_eq!(rights.credit_text, "");
        assert_eq!(rights.title, "Picnic");
    }

    #[test]
    fn r8_unvouched_licensed_reprint_fallback_falls() {
        // Round 8 (R8-3 / R8-R2, the review probe
        // verbatim): the existing record notes
        // children and a CC BY license with no credit;
        // the incoming record carries LicensedReprint
        // only — no children note, no credit, no
        // permission of any kind. The composed rule
        // drops the CC BY, and the fallback used to
        // install the LicensedReprint unconditionally,
        // displacing the standing license with a claim
        // no record vouched. The fallback arms are
        // uniform now: an unvouched LicensedReprint
        // falls to Unknown like any other.
        let existing = RightsRecord {
            children_present: Some(true),
            license: LicenseKind::CcBy("4.0".into()),
            credit_text: String::new(),
            ..Default::default()
        };
        let incoming = RightsRecord {
            license: LicenseKind::LicensedReprint,
            ..Default::default()
        };
        let merged = merge_rights_records(&existing, &incoming);
        assert_eq!(merged.children_present, Some(true));
        assert_eq!(merged.credit_text, "");
        assert_eq!(merged.license, LicenseKind::Unknown);
        // The same pair through a fold composes the
        // same way.
        let hash = sha256_hex(b"R8REPRINT");
        let asset = |rights: RightsRecord| Asset {
            sha256: hash.clone(),
            kind: AssetKind::Picture,
            media_type: "image/png".into(),
            rights,
        };
        let mut assets: Vec<Asset> = vec![existing.clone(), incoming]
            .into_iter()
            .map(asset)
            .collect();
        fold_duplicate_asset_records(&mut assets);
        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].rights.children_present, Some(true));
        assert_eq!(assets[0].rights.license, LicenseKind::Unknown);
        // Control: a LicensedReprint whose own source
        // record notes children IS vouched, and stands
        // where the dropped CC BY stood — the uniform
        // condition, not a blanket fall.
        let vouched = RightsRecord {
            children_present: Some(true),
            license: LicenseKind::LicensedReprint,
            ..Default::default()
        };
        let merged = merge_rights_records(&existing, &vouched);
        assert_eq!(merged.license, LicenseKind::LicensedReprint);
    }

    #[test]
    fn r8_fallback_controls_from_the_cleared_shapes() {
        // Round 8 (R8-3): the review lane's cleared
        // controls, pinned where they were not already
        // pinned. (The vouched-CC0 fallback and the
        // own-credit control are already pinned by
        // `r7_a_dropped_license_leaves_a_vouched_license_standing`
        // and `r7_a_standing_license_loses_its_credit_and_falls`.)
        let existing = RightsRecord {
            children_present: Some(true),
            license: LicenseKind::CcBy("4.0".into()),
            ..Default::default()
        };
        // A ChurchOwned license from a source that
        // notes no children falls to Unknown.
        let church_owned = RightsRecord {
            license: LicenseKind::ChurchOwned,
            ..Default::default()
        };
        let merged = merge_rights_records(&existing, &church_owned);
        assert_eq!(merged.license, LicenseKind::Unknown);
        // A CC BY-SA "other" license falls too:
        // attribution licenses never stand as the
        // fallback.
        let cc_by_sa = RightsRecord {
            license: LicenseKind::CcBySa("4.0".into()),
            ..Default::default()
        };
        let merged = merge_rights_records(&existing, &cc_by_sa);
        assert_eq!(merged.license, LicenseKind::Unknown);
        // The mirror direction, exactly as cleared: a
        // standing LicensedReprint, an incoming CC BY
        // whose credit the fence refuses — the
        // standing license stays and the credit is
        // refused.
        let credit_text = "Photo by Alice, CC BY 4.0".to_string();
        let standing = RightsRecord {
            children_present: Some(true),
            license: LicenseKind::LicensedReprint,
            ..Default::default()
        };
        let incoming = RightsRecord {
            license: LicenseKind::CcBy("4.0".into()),
            credit_text: credit_text.clone(),
            ..Default::default()
        };
        let merged = merge_rights_records(&standing, &incoming);
        assert_eq!(merged.children_present, Some(true));
        assert_eq!(merged.license, LicenseKind::LicensedReprint);
        assert_eq!(merged.credit_text, "");
    }

    #[test]
    fn r8_fold_judges_the_composed_rule_once_at_the_end() {
        // Round 8 (R8-9 / R8-R6): fold [A, T, C] —
        // A {children, CC BY, no credit}, T {title
        // only}, C {children, credit}. Pairwise judging
        // dropped the license at A+T, before C's credit
        // arrived to stand beside it. The fold now
        // judges the composed rule once, on the final
        // folded record, so both orders compose
        // identically: CC BY + credit + title stand.
        let credit_text = "Photo by Alice".to_string();
        let title_text = "Harvest supper".to_string();
        let a = RightsRecord {
            children_present: Some(true),
            license: LicenseKind::CcBy("4.0".into()),
            ..Default::default()
        };
        let t = RightsRecord {
            title: title_text.clone(),
            ..Default::default()
        };
        let c = RightsRecord {
            children_present: Some(true),
            credit_text: credit_text.clone(),
            ..Default::default()
        };
        let hash = sha256_hex(b"R8FOLD");
        let asset = |rights: RightsRecord| Asset {
            sha256: hash.clone(),
            kind: AssetKind::Picture,
            media_type: "image/png".into(),
            rights,
        };
        for records in [
            vec![a.clone(), t.clone(), c.clone()],
            vec![a.clone(), c.clone(), t.clone()],
        ] {
            let mut assets: Vec<Asset> = records.into_iter().map(asset).collect();
            fold_duplicate_asset_records(&mut assets);
            assert_eq!(assets.len(), 1);
            let rights = &assets[0].rights;
            assert_eq!(rights.children_present, Some(true));
            assert_eq!(rights.license, LicenseKind::CcBy("4.0".into()));
            assert_eq!(rights.credit_text, credit_text);
            assert_eq!(rights.title, title_text);
        }
        // The same sequence through single merges
        // (add_asset) keeps the R7-A pairwise outcome
        // exactly: A+T drops the license before C's
        // credit arrives, and the credit then stands
        // beside no license.
        let mut doc = bare_doc();
        doc.add_asset(b"R8PIC", AssetKind::Picture, "image/png", a.clone());
        doc.add_asset(b"R8PIC", AssetKind::Picture, "image/png", t.clone());
        doc.add_asset(b"R8PIC", AssetKind::Picture, "image/png", c.clone());
        assert_eq!(doc.assets.len(), 1);
        let rights = &doc.assets[0].rights;
        assert_eq!(rights.children_present, Some(true));
        assert_eq!(rights.license, LicenseKind::Unknown);
        assert_eq!(rights.credit_text, credit_text);
        assert_eq!(rights.title, title_text);
    }

    #[test]
    fn r9_unvouched_licensed_reprint_falls_in_both_paths() {
        // Round 9 (R9-1 / RT9-1): a LicensedReprint
        // that IS the standing license, or that fills
        // the empty slot beside a children note, was
        // judged by nothing. One final judgment now
        // runs on every composed record: unvouched, it
        // falls to Unknown in both paths, in every
        // witness order.
        let lr = RightsRecord {
            license: LicenseKind::LicensedReprint,
            ..Default::default()
        };
        let kids = RightsRecord {
            children_present: Some(true),
            ..Default::default()
        };
        let kids_ccby = RightsRecord {
            children_present: Some(true),
            license: LicenseKind::CcBy("4.0".into()),
            ..Default::default()
        };
        let title_text = "Harvest supper".to_string();
        let title_only = RightsRecord {
            title: title_text.clone(),
            ..Default::default()
        };
        let single = |seq: &[RightsRecord]| -> RightsRecord {
            let mut acc = seq[0].clone();
            for next in &seq[1..] {
                acc = merge_rights_records(&acc, next);
            }
            acc
        };
        let folded = |seq: &[RightsRecord]| -> RightsRecord {
            let hash = sha256_hex(b"R9FOLD");
            let mut assets: Vec<Asset> = seq
                .iter()
                .cloned()
                .map(|rights| Asset {
                    sha256: hash.clone(),
                    kind: AssetKind::Picture,
                    media_type: "image/png".into(),
                    rights,
                })
                .collect();
            fold_duplicate_asset_records(&mut assets);
            assert_eq!(assets.len(), 1);
            assets[0].rights.clone()
        };
        let witnesses: Vec<Vec<RightsRecord>> = vec![
            vec![lr.clone(), kids_ccby.clone()],
            vec![kids_ccby.clone(), lr.clone()],
            vec![kids.clone(), lr.clone()],
            vec![lr.clone(), kids.clone()],
            vec![kids_ccby.clone(), title_only.clone(), lr.clone()],
            vec![lr.clone(), kids_ccby.clone(), title_only.clone()],
        ];
        for seq in &witnesses {
            for composed in [single(seq), folded(seq)] {
                assert_eq!(composed.children_present, Some(true), "{seq:?}");
                assert_eq!(composed.credit_text, "", "{seq:?}");
                assert_eq!(composed.license, LicenseKind::Unknown, "{seq:?}");
            }
            // The two paths compose identically.
            let via_single = single(seq);
            let via_fold = folded(seq);
            assert_eq!(
                (
                    via_single.children_present,
                    via_single.license.clone(),
                    via_single.credit_text.clone()
                ),
                (
                    via_fold.children_present,
                    via_fold.license.clone(),
                    via_fold.credit_text.clone()
                ),
                "{seq:?}"
            );
        }
    }

    #[test]
    fn r9_vouched_licensed_reprint_controls_stand_in_every_order() {
        // Round 9 (R9-1): the vouched controls stand —
        // a record holding LicensedReprint together
        // with a children note, and a record holding
        // LicensedReprint together with its own credit
        // (that credit standing in the composed
        // record) — in every order, in both paths.
        let kids_ccby = RightsRecord {
            children_present: Some(true),
            license: LicenseKind::CcBy("4.0".into()),
            ..Default::default()
        };
        let kids_lr = RightsRecord {
            children_present: Some(true),
            license: LicenseKind::LicensedReprint,
            ..Default::default()
        };
        let credit_text = "Photo by H".to_string();
        let kids_lr_credit = RightsRecord {
            children_present: Some(true),
            license: LicenseKind::LicensedReprint,
            credit_text: credit_text.clone(),
            ..Default::default()
        };
        let lr_credit = RightsRecord {
            license: LicenseKind::LicensedReprint,
            credit_text: credit_text.clone(),
            ..Default::default()
        };
        let kids_credit = RightsRecord {
            children_present: Some(true),
            credit_text: credit_text.clone(),
            ..Default::default()
        };
        let title_text = "Harvest supper".to_string();
        let title_only = RightsRecord {
            title: title_text.clone(),
            ..Default::default()
        };
        let single = |seq: &[RightsRecord]| -> RightsRecord {
            let mut acc = seq[0].clone();
            for next in &seq[1..] {
                acc = merge_rights_records(&acc, next);
            }
            acc
        };
        let folded = |seq: &[RightsRecord]| -> RightsRecord {
            let hash = sha256_hex(b"R9CTRL");
            let mut assets: Vec<Asset> = seq
                .iter()
                .cloned()
                .map(|rights| Asset {
                    sha256: hash.clone(),
                    kind: AssetKind::Picture,
                    media_type: "image/png".into(),
                    rights,
                })
                .collect();
            fold_duplicate_asset_records(&mut assets);
            assert_eq!(assets.len(), 1);
            assets[0].rights.clone()
        };
        // LicensedReprint + children, against the
        // attribution record, in both orders.
        for seq in [
            vec![kids_ccby.clone(), kids_lr.clone()],
            vec![kids_lr.clone(), kids_ccby.clone()],
        ] {
            for composed in [single(&seq), folded(&seq)] {
                assert_eq!(composed.license, LicenseKind::LicensedReprint, "{seq:?}");
                assert_eq!(composed.children_present, Some(true), "{seq:?}");
            }
        }
        // LicensedReprint + children + its own credit:
        // the credit stands, the license stands.
        for seq in [
            vec![kids_lr_credit.clone(), title_only.clone()],
            vec![title_only.clone(), kids_lr_credit.clone()],
        ] {
            for composed in [single(&seq), folded(&seq)] {
                assert_eq!(composed.license, LicenseKind::LicensedReprint, "{seq:?}");
                assert_eq!(composed.credit_text, credit_text, "{seq:?}");
            }
        }
        // LicensedReprint + its own credit, where the
        // same credit stands on a children-bearing
        // record in the composition: the credit that
        // stands is the credit its record held.
        for seq in [
            vec![lr_credit.clone(), kids_credit.clone()],
            vec![kids_credit.clone(), lr_credit.clone()],
        ] {
            for composed in [single(&seq), folded(&seq)] {
                assert_eq!(composed.license, LicenseKind::LicensedReprint, "{seq:?}");
                assert_eq!(composed.credit_text, credit_text, "{seq:?}");
                assert_eq!(composed.children_present, Some(true), "{seq:?}");
            }
        }
    }

    #[test]
    fn r9_fold_fallback_winner_is_order_independent() {
        // Round 9 (R9-4 / RT9-4): the fold's fallback
        // winner was the first vouched match in fold
        // order. Fallback selection is order-independent
        // now: a vouched license carried by a
        // children-bearing source record beats one
        // vouched only through the bearer rule, and
        // among equals the fixed precedence decides —
        // the church's own work, then public domain,
        // then CC0, then a reprint license.
        let kids_ccby = RightsRecord {
            children_present: Some(true),
            license: LicenseKind::CcBy("4.0".into()),
            ..Default::default()
        };
        let kids_cc0 = RightsRecord {
            children_present: Some(true),
            license: LicenseKind::Cc0,
            ..Default::default()
        };
        let church = RightsRecord {
            license: LicenseKind::ChurchOwned,
            ..Default::default()
        };
        let kids_pd = RightsRecord {
            children_present: Some(true),
            license: LicenseKind::PublicDomain,
            ..Default::default()
        };
        let kids_church = RightsRecord {
            children_present: Some(true),
            license: LicenseKind::ChurchOwned,
            ..Default::default()
        };
        let kids_lr = RightsRecord {
            children_present: Some(true),
            license: LicenseKind::LicensedReprint,
            ..Default::default()
        };
        let folded_license = |seq: &[RightsRecord]| -> LicenseKind {
            let hash = sha256_hex(b"R9FALL");
            let mut assets: Vec<Asset> = seq
                .iter()
                .cloned()
                .map(|rights| Asset {
                    sha256: hash.clone(),
                    kind: AssetKind::Picture,
                    media_type: "image/png".into(),
                    rights,
                })
                .collect();
            fold_duplicate_asset_records(&mut assets);
            assert_eq!(assets.len(), 1);
            assets[0].rights.license.clone()
        };
        // The RT9-4 witness pair: Cc0 is carried by a
        // children-bearing record; ChurchOwned is
        // vouched only through the bearer rule (the
        // Cc0 record). Cc0 wins in both orders.
        assert_eq!(
            folded_license(&[kids_ccby.clone(), kids_cc0.clone(), church.clone()]),
            LicenseKind::Cc0
        );
        assert_eq!(
            folded_license(&[kids_ccby.clone(), church.clone(), kids_cc0.clone()]),
            LicenseKind::Cc0
        );
        // Both carriers children-bearing: the fixed
        // precedence decides — ChurchOwned over
        // PublicDomain, in both orders.
        assert_eq!(
            folded_license(&[kids_ccby.clone(), kids_pd.clone(), kids_church.clone()]),
            LicenseKind::ChurchOwned
        );
        assert_eq!(
            folded_license(&[kids_ccby.clone(), kids_church.clone(), kids_pd.clone()]),
            LicenseKind::ChurchOwned
        );
        // Both carriers children-bearing: CC0 over a
        // reprint license, in both orders.
        assert_eq!(
            folded_license(&[kids_ccby.clone(), kids_lr.clone(), kids_cc0.clone()]),
            LicenseKind::Cc0
        );
        assert_eq!(
            folded_license(&[kids_ccby.clone(), kids_cc0.clone(), kids_lr.clone()]),
            LicenseKind::Cc0
        );
    }

    #[test]
    fn r9_fold_rejudges_the_assertion_against_the_final_record() {
        // Round 9 (R9-5 / RT9-5): in a fold, an
        // assertion rode in on a license the fold-end
        // judge then dropped, and the assertion stayed.
        // At fold end the assertion is re-judged by
        // the R7-8 rule against the FINAL folded
        // record: it stands only if a substantive
        // rights fill from its own record stands there.
        let empty = RightsRecord::default();
        let asserter_name = "Mallory".to_string();
        let asserter_date = "2026-01-01".to_string();
        let kids_ccby_asserted = RightsRecord {
            children_present: Some(true),
            license: LicenseKind::CcBy("4.0".into()),
            asserted_by: asserter_name.clone(),
            asserted_at: asserter_date.clone(),
            ..Default::default()
        };
        let title_text = "Harvest supper".to_string();
        let title_only = RightsRecord {
            title: title_text.clone(),
            ..Default::default()
        };
        let folded = |seq: &[RightsRecord]| -> RightsRecord {
            let hash = sha256_hex(b"R9ASSERT");
            let mut assets: Vec<Asset> = seq
                .iter()
                .cloned()
                .map(|rights| Asset {
                    sha256: hash.clone(),
                    kind: AssetKind::Picture,
                    media_type: "image/png".into(),
                    rights,
                })
                .collect();
            fold_duplicate_asset_records(&mut assets);
            assert_eq!(assets.len(), 1);
            assets[0].rights.clone()
        };
        for seq in [
            vec![empty.clone(), kids_ccby_asserted.clone()],
            vec![
                title_only.clone(),
                kids_ccby_asserted.clone(),
                title_only.clone(),
            ],
        ] {
            let composed = folded(&seq);
            assert_eq!(composed.license, LicenseKind::Unknown, "{seq:?}");
            assert_eq!(composed.asserted_by, "", "{seq:?}");
            assert_eq!(composed.asserted_at, "", "{seq:?}");
        }
        // The single-merge control is unchanged: the
        // same pair through one merge leaves the
        // assertion empty too.
        let merged = merge_rights_records(&empty, &kids_ccby_asserted);
        assert_eq!(merged.license, LicenseKind::Unknown);
        assert_eq!(merged.asserted_by, "");
        assert_eq!(merged.asserted_at, "");
        // Control: when the asserting record's own
        // credit stands in the final folded record,
        // the assertion stands with it.
        let credit_text = "Photo by Alice".to_string();
        let kids_credit_asserted = RightsRecord {
            children_present: Some(true),
            credit_text: credit_text.clone(),
            asserted_by: asserter_name.clone(),
            asserted_at: asserter_date.clone(),
            ..Default::default()
        };
        let composed = folded(&[empty.clone(), kids_credit_asserted]);
        assert_eq!(composed.credit_text, credit_text);
        assert_eq!(composed.asserted_by, asserter_name);
        assert_eq!(composed.asserted_at, asserter_date);
    }

    #[test]
    fn r9_fold_triple_is_permutation_invariant_over_the_pool() {
        // Round 9 (R9-1): a brute-force-style
        // permutation check, in-tree over a small
        // archetype pool: for each multiset tested,
        // every permutation folds to the same
        // (children, license, credit) triple.
        fn all_perms(idxs: &[usize]) -> Vec<Vec<usize>> {
            if idxs.len() <= 1 {
                return vec![idxs.to_vec()];
            }
            let mut out = Vec::new();
            for i in 0..idxs.len() {
                let mut rest = idxs.to_vec();
                let first = rest.remove(i);
                for tail in all_perms(&rest) {
                    let mut perm = vec![first];
                    perm.extend(tail);
                    out.push(perm);
                }
            }
            out
        }
        let lr = RightsRecord {
            license: LicenseKind::LicensedReprint,
            ..Default::default()
        };
        let kids = RightsRecord {
            children_present: Some(true),
            ..Default::default()
        };
        let kids_ccby = RightsRecord {
            children_present: Some(true),
            license: LicenseKind::CcBy("4.0".into()),
            ..Default::default()
        };
        let kids_cc0 = RightsRecord {
            children_present: Some(true),
            license: LicenseKind::Cc0,
            ..Default::default()
        };
        let kids_lr = RightsRecord {
            children_present: Some(true),
            license: LicenseKind::LicensedReprint,
            ..Default::default()
        };
        let title_text = "Harvest supper".to_string();
        let title_only = RightsRecord {
            title: title_text.clone(),
            ..Default::default()
        };
        let credit_text = "Photo by H".to_string();
        let kids_lr_credit = RightsRecord {
            children_present: Some(true),
            license: LicenseKind::LicensedReprint,
            credit_text: credit_text.clone(),
            ..Default::default()
        };
        // (multiset, the triple every permutation
        // must fold to). Each multiset tested here
        // carries at most one vouched non-attribution
        // license, so the final judgment fully
        // determines the triple: the witness multisets
        // (no vouched license at all), the vouched
        // controls (one), and single-candidate
        // fallback multisets (one). Multisets with
        // two vouched licenses are pinned by their
        // RT9-4 witness orders in
        // `r9_fold_fallback_winner_is_order_independent`
        // — there the standing license, when it is
        // itself vouched, stands under the fill law,
        // and precedence governs only the fallback a
        // fallen license drops to.
        type Triple = (Option<bool>, LicenseKind, String);
        let cases: Vec<(Vec<RightsRecord>, Triple)> = vec![
            (
                vec![lr.clone(), kids_ccby.clone()],
                (Some(true), LicenseKind::Unknown, String::new()),
            ),
            (
                vec![kids.clone(), lr.clone()],
                (Some(true), LicenseKind::Unknown, String::new()),
            ),
            (
                vec![kids_ccby.clone(), title_only.clone(), lr.clone()],
                (Some(true), LicenseKind::Unknown, String::new()),
            ),
            (
                vec![kids_ccby.clone(), kids_lr.clone()],
                (Some(true), LicenseKind::LicensedReprint, String::new()),
            ),
            (
                vec![kids_ccby.clone(), kids_cc0.clone(), title_only.clone()],
                (Some(true), LicenseKind::Cc0, String::new()),
            ),
            (
                vec![kids_ccby.clone(), kids_lr.clone(), title_only.clone()],
                (Some(true), LicenseKind::LicensedReprint, String::new()),
            ),
            (
                vec![kids_lr_credit.clone(), title_only.clone()],
                (
                    Some(true),
                    LicenseKind::LicensedReprint,
                    credit_text.clone(),
                ),
            ),
        ];
        for (multiset, expected) in &cases {
            let indices: Vec<usize> = (0..multiset.len()).collect();
            for perm in all_perms(&indices) {
                let seq: Vec<RightsRecord> = perm.iter().map(|&i| multiset[i].clone()).collect();
                let hash = sha256_hex(b"R9PERM");
                let mut assets: Vec<Asset> = seq
                    .iter()
                    .cloned()
                    .map(|rights| Asset {
                        sha256: hash.clone(),
                        kind: AssetKind::Picture,
                        media_type: "image/png".into(),
                        rights,
                    })
                    .collect();
                fold_duplicate_asset_records(&mut assets);
                assert_eq!(assets.len(), 1);
                let rights = &assets[0].rights;
                let triple = (
                    rights.children_present,
                    rights.license.clone(),
                    rights.credit_text.clone(),
                );
                assert_eq!(&triple, expected, "permutation {perm:?} of {multiset:?}");
            }
        }
    }

    #[test]
    fn r7_add_asset_in_the_mirror_order_drops_the_license() {
        // Round 7 (R7-1): add_asset in the mirror
        // order — the bulletin first holds the picture
        // as children-noted with a CC BY license, then
        // the same bytes arrive again carrying only
        // the credit the fence refuses.
        let mut doc = bare_doc();
        let first = RightsRecord {
            license: LicenseKind::CcBy("4.0".into()),
            children_present: Some(true),
            ..Default::default()
        };
        let second = RightsRecord {
            credit_text: "Photo by Alice, CC BY 4.0".into(),
            ..Default::default()
        };
        doc.add_asset(b"R7PIC", AssetKind::Picture, "image/png", first);
        doc.add_asset(b"R7PIC", AssetKind::Picture, "image/png", second);
        assert_eq!(doc.assets.len(), 1);
        let rights = &doc.assets[0].rights;
        assert_eq!(rights.children_present, Some(true));
        assert_eq!(rights.license, LicenseKind::Unknown);
        assert_eq!(rights.credit_text, "");
    }

    #[test]
    fn r7_assertion_rides_only_a_standing_rights_fill() {
        // Round 7 (R7-8 / RT7-8, the probe verbatim):
        // the incoming record's credit is refused by
        // the fence, its title fill lands — and its
        // assertion used to ride in on that unrelated
        // fill, reading as vouching for a composed
        // record its author never saw.
        let existing = RightsRecord {
            children_present: Some(true),
            ..Default::default()
        };
        let title_fill = RightsRecord {
            title: "Harvest supper".into(),
            credit_text: "Photo by Alice".into(),
            asserted_by: "Mallory".into(),
            asserted_at: "2026-01-01".into(),
            ..Default::default()
        };
        let merged = merge_rights_records(&existing, &title_fill);
        assert_eq!(merged.title, "Harvest supper");
        assert_eq!(merged.credit_text, "");
        assert_eq!(merged.asserted_by, "");
        assert_eq!(merged.asserted_at, "");
        // The same ride on a source_url-only fill.
        let url_fill = RightsRecord {
            source_url: Some("https://example.org/picnic".into()),
            credit_text: "Photo by Alice".into(),
            asserted_by: "Mallory".into(),
            asserted_at: "2026-01-01".into(),
            ..Default::default()
        };
        let merged = merge_rights_records(&existing, &url_fill);
        assert_eq!(
            merged.source_url.as_deref(),
            Some("https://example.org/picnic")
        );
        assert_eq!(merged.credit_text, "");
        assert_eq!(merged.asserted_by, "");
        assert_eq!(merged.asserted_at, "");
        // The R5-A control stays held: an incoming
        // record carrying only the refused credit plus
        // the assertion changes nothing at all.
        let pair_only = RightsRecord {
            credit_text: "Photo by Alice".into(),
            asserted_by: "Mallory".into(),
            asserted_at: "2026-01-01".into(),
            ..Default::default()
        };
        assert_eq!(merge_rights_records(&existing, &pair_only), existing);
        // When the same record's credit genuinely
        // stands, the assertion stands with it.
        let plain = RightsRecord::default();
        let credited = RightsRecord {
            credit_text: "Photo by Alice".into(),
            asserted_by: "Mallory".into(),
            asserted_at: "2026-01-01".into(),
            ..Default::default()
        };
        let merged = merge_rights_records(&plain, &credited);
        assert_eq!(merged.credit_text, "Photo by Alice");
        assert_eq!(merged.asserted_by, "Mallory");
        assert_eq!(merged.asserted_at, "2026-01-01");
        // A license that itself stands is substantive
        // the same way.
        let licensed = RightsRecord {
            license: LicenseKind::CcBy("4.0".into()),
            asserted_by: "Mallory".into(),
            asserted_at: "2026-01-01".into(),
            ..Default::default()
        };
        let merged = merge_rights_records(&plain, &licensed);
        assert_eq!(merged.license, LicenseKind::CcBy("4.0".into()));
        assert_eq!(merged.asserted_by, "Mallory");
        assert_eq!(merged.asserted_at, "2026-01-01");
    }

    #[test]
    fn paragraph_loads_the_pre_runs_text_shape() {
        // Every file written before G1b stores paragraphs
        // as { style, text }: the words become one plain run.
        let p: Paragraph =
            serde_json::from_str(r#"{"style":"body","text":"Sing hymn 432"}"#).expect("parse");
        assert_eq!(p.style, "body");
        assert_eq!(p.text(), "Sing hymn 432");
        assert_eq!(p.runs, vec![Run::plain("Sing hymn 432")]);
    }

    #[test]
    fn paragraph_runs_shape_round_trips_and_writes_no_text_key() {
        let p = Paragraph::from_runs(
            "body",
            vec![
                Run::plain("Sing "),
                Run {
                    text: "hymn".into(),
                    bold: Some(true),
                    italic: None,
                },
                Run::plain(" 432"),
            ],
        );
        let json = serde_json::to_string(&p).expect("json");
        assert!(json.contains("\"runs\""), "runs shape written: {json}");
        assert!(
            !json.contains("\"style\":\"body\",\"text\""),
            "no legacy paragraph-level text key: {json}"
        );
        let back: Paragraph = serde_json::from_str(&json).expect("parse");
        assert_eq!(back, p);
        assert_eq!(back.text(), "Sing hymn 432");
    }

    #[test]
    fn normalize_runs_merges_and_drops_empties() {
        let runs = normalize_runs(vec![
            Run::plain("a"),
            Run::plain(""),
            Run::plain("b"),
            Run {
                text: "c".into(),
                bold: Some(true),
                italic: None,
            },
            Run {
                text: "d".into(),
                bold: Some(true),
                italic: None,
            },
        ]);
        assert_eq!(
            runs,
            vec![
                Run::plain("ab"),
                Run {
                    text: "cd".into(),
                    bold: Some(true),
                    italic: None,
                },
            ]
        );
        // An all-empty list still yields one (empty) run.
        assert_eq!(normalize_runs(vec![Run::plain("")]), vec![Run::plain("")]);
        assert_eq!(normalize_runs(vec![]), vec![Run::plain("")]);
    }

    #[test]
    fn resolve_runs_keeps_faces_and_resolves_tokens() {
        let mut doc = bare_doc();
        doc.profile.pastor = "Pastor Lee".into();
        let runs = vec![
            Run::plain("Sermon: "),
            Run {
                text: "{Church.pastor}".into(),
                bold: Some(true),
                italic: None,
            },
            Run::plain(" speaks on "),
            Run {
                text: "grace".into(),
                bold: None,
                italic: Some(true),
            },
        ];
        let out = resolve_runs(&runs, &doc);
        assert_eq!(
            out,
            vec![
                Run::plain("Sermon: "),
                Run {
                    text: "Pastor Lee".into(),
                    bold: Some(true),
                    italic: None,
                },
                Run::plain(" speaks on "),
                Run {
                    text: "grace".into(),
                    bold: None,
                    italic: Some(true),
                },
            ]
        );
        // A token split across two runs resolves whole and
        // wears the face of the run holding its first byte.
        let split = vec![
            Run {
                text: "{Church.pa".into(),
                bold: Some(true),
                italic: None,
            },
            Run::plain("stor}"),
        ];
        let out = resolve_runs(&split, &doc);
        assert_eq!(
            out,
            vec![Run {
                text: "Pastor Lee".into(),
                bold: Some(true),
                italic: None,
            }]
        );
    }

    #[test]
    fn v0_era_document_json_loads_with_words_intact() {
        // A whole v0-era document: paragraphs in the text
        // shape, pre-occasions keys absent. Deserialization
        // alone must carry the words.
        let json = r#"{
            "format_version": 0,
            "title": "Old",
            "profile": {"name": "Grace", "address": "", "service_times": "",
                        "website": "", "giving_url": "", "pastor": ""},
            "fields": {},
            "styles": [],
            "stories": [{"id": "s", "paragraphs": [
                {"style": "body", "text": "Welcome to worship"}
            ], "keep_through": null}],
            "panels": [],
            "sheet": {"preset": "letter-portrait", "size": {"w": 612000, "h": 792000}}
        }"#;
        let doc: Document = serde_json::from_str(json).expect("parse");
        assert_eq!(doc.stories[0].paragraphs[0].text(), "Welcome to worship");
        assert_eq!(
            doc.stories[0].paragraphs[0].runs,
            vec![Run::plain("Welcome to worship")]
        );
        // Fields added since (occasion time, wanted fonts,
        // assets) all arrive at their defaults: a file from
        // before wanted fonts simply wants none.
        assert!(doc.wanted_fonts.is_empty());
        assert!(doc.occasion_time.is_none());
    }
}

#[cfg(test)]
mod picture_tests {
    use super::*;

    #[test]
    fn frames_written_before_pictures_load_without_one() {
        // A frame JSON from before picture boxes existed: no
        // `picture` key at all, and the kind names only the
        // old two.
        let json = r#"{"id":"f","name":"Box","kind":"Text",
            "rect":{"x":0,"y":0,"w":1000,"h":1000},
            "story_id":"s","role":"Weekly"}"#;
        let frame: Frame = serde_json::from_str(json).expect("parse");
        assert_eq!(frame.kind, FrameKind::Text);
        assert_eq!(frame.picture, None);
    }

    #[test]
    fn a_picture_placement_defaults_to_fill_centered() {
        let json = r#"{"asset":"abc123"}"#;
        let p: PicturePlacement = serde_json::from_str(json).expect("parse");
        assert_eq!(p.fit, PictureFit::Fill);
        assert_eq!(p.crop_x, 0.5);
        assert_eq!(p.crop_y, 0.5);
    }

    #[test]
    fn shown_rect_fill_covers_and_crop_chooses_the_part() {
        // A wide picture (200x100) in a square box (100x100):
        // Fill scales to 200x100 shown, half hidden sideways.
        let (x, y, w, h) =
            picture_shown_rect(PictureFit::Fill, 200.0, 100.0, 100.0, 100.0, 0.5, 0.5);
        assert_eq!((x, y, w, h), (-50.0, 0.0, 200.0, 100.0));
        // Crop hard left shows the picture's left edge.
        let (x, ..) = picture_shown_rect(PictureFit::Fill, 200.0, 100.0, 100.0, 100.0, 0.0, 0.5);
        assert_eq!(x, 0.0);
        // Crop hard right shows the right edge (box ends at
        // the picture's right edge).
        let (x, ..) = picture_shown_rect(PictureFit::Fill, 200.0, 100.0, 100.0, 100.0, 1.0, 0.5);
        assert_eq!(x, -100.0);
    }

    #[test]
    fn shown_rect_show_whole_fits_inside_centered() {
        let (x, y, w, h) =
            picture_shown_rect(PictureFit::ShowWhole, 200.0, 100.0, 100.0, 100.0, 0.5, 0.5);
        assert_eq!((x, y, w, h), (0.0, 25.0, 100.0, 50.0));
    }

    // ---- Threading surgery (T1) ---------------------------

    fn frame(id: &str, story: Option<&str>) -> Frame {
        Frame {
            id: id.into(),
            name: id.into(),
            kind: FrameKind::Text,
            rect: Rect::new(0, 0, 100_000, 100_000),
            story_id: story.map(|s| s.to_string()),
            role: ContentRole::Weekly,
            picture: None,
            music: None,
        }
    }

    fn doc_with_panels(panels: Vec<Panel>) -> Document {
        Document {
            format_version: FORMAT_VERSION,
            title: "t".into(),
            profile: ChurchProfile::default(),
            fields: Default::default(),
            styles: default_styles(),
            stories: Vec::new(),
            panels,
            sheet: SheetSpec {
                preset: "letter-portrait".into(),
                size: Size::new(612_000, 792_000),
            },
            no_announcements: false,
            assets: Vec::new(),
            asset_bytes: Default::default(),
            occasion_time: None,
            wanted_fonts: Vec::new(),
            participants: BTreeMap::new(),
            inserts: Vec::new(),
            applied_contributions: Vec::new(),
        }
    }

    #[test]
    fn frames_of_story_come_in_document_order() {
        let panel = |id: &str, frames: Vec<Frame>| Panel {
            id: id.into(),
            name: id.into(),
            size: Size::new(612_000, 792_000),
            frames,
            guides: Vec::new(),
            background_image: None,
        };
        let doc = doc_with_panels(vec![
            panel("p1", vec![frame("a", Some("s1")), frame("b", Some("s2"))]),
            panel("p2", vec![frame("c", Some("s1"))]),
        ]);
        assert_eq!(doc.frames_of_story("s1"), vec![(0, 0), (1, 0)]);
        assert_eq!(doc.frames_of_story("s2"), vec![(0, 1)]);
        assert!(doc.frames_of_story("nope").is_empty());
    }

    #[test]
    fn split_story_paragraphs_cuts_words_and_runs_cleanly() {
        let paras = vec![
            Paragraph::plain("body", "First para"),
            Paragraph::from_runs(
                "body",
                vec![
                    Run::plain("Sing "),
                    Run {
                        text: "hymn".into(),
                        bold: Some(true),
                        italic: None,
                    },
                    Run::plain(" 432 today"),
                ],
            ),
            Paragraph::plain("body", "Last para"),
        ];
        // Cut inside the second paragraph, inside "hymn".
        let cut = "Sing hy".len();
        let (head, tail) = split_story_paragraphs(&paras, 1, cut);
        let head_text: Vec<String> = head.iter().map(|p| p.text()).collect();
        let tail_text: Vec<String> = tail.iter().map(|p| p.text()).collect();
        assert_eq!(head_text, vec!["First para", "Sing hy"]);
        assert_eq!(tail_text, vec!["mn 432 today", "Last para"]);
        // The bold run divides with its face on both sides.
        let tail_first = &tail[0];
        assert!(
            tail_first
                .runs
                .iter()
                .any(|r| r.text == "mn" && r.bold == Some(true)),
            "the tail keeps the cut run's face: {:?}",
            tail_first.runs
        );
        // No words are lost or repeated across the cut.
        let joined = format!("{}{}", head_text.join(""), tail_text.join(""));
        assert_eq!(
            joined, "First paraSing hymn 432 todayLast para",
            "the two sides tile the story"
        );
    }

    #[test]
    fn split_story_paragraphs_at_the_edges_stays_valid() {
        let paras = vec![Paragraph::plain("body", "Only words")];
        let (head, tail) = split_story_paragraphs(&paras, 0, 0);
        assert_eq!(head.len(), 1);
        assert_eq!(head[0].text(), "");
        assert_eq!(tail[0].text(), "Only words");
        let (head, tail) = split_story_paragraphs(&paras, 0, "Only words".len());
        assert_eq!(head[0].text(), "Only words");
        assert_eq!(tail.len(), 1);
        assert_eq!(tail[0].text(), "");
        // A paragraph boundary cut: head takes whole
        // paragraphs, the tail starts fresh.
        let two = vec![
            Paragraph::plain("body", "One"),
            Paragraph::plain("body", "Two"),
        ];
        let (head, tail) = split_story_paragraphs(&two, 1, 0);
        assert_eq!(head.len(), 1);
        assert_eq!(head[0].text(), "One");
        assert_eq!(tail[0].text(), "Two");
    }

    fn sample_insert() -> Insert {
        Insert {
            id: "sheet-1".into(),
            name: "Prayer list".into(),
            life: InsertLife::Dated,
            keep_through: Some(SimpleDate {
                year: 2026,
                month: 11,
                day: 8,
            }),
            panels: vec![Panel {
                id: "sheet-1-p1".into(),
                name: "Prayer list".into(),
                size: insert_sheet_size("half-letter").expect("shape"),
                frames: vec![Frame {
                    id: "sheet-1-f1".into(),
                    name: "Words".into(),
                    kind: FrameKind::Text,
                    rect: Rect::new(1000, 1000, 300_000, 500_000),
                    story_id: Some("sheet-1-s1".into()),
                    role: ContentRole::Standing,
                    picture: None,
                    music: None,
                }],
                guides: vec![],
                background_image: None,
            }],
            stories: vec![Story {
                id: "sheet-1-s1".into(),
                paragraphs: vec![Paragraph::new(
                    "body",
                    "Prayers for {Sunday.date} at {Church.name}",
                )],
                keep_through: None,
            }],
            styles: default_styles(),
            sheet: InsertSheet {
                preset: "half-letter".into(),
                copies: 60,
                paper_note: "blue paper".into(),
            },
        }
    }

    #[test]
    fn inserts_round_trip_through_json() {
        let mut doc = doc_with_panels(Vec::new());
        doc.inserts = vec![sample_insert()];
        let json = serde_json::to_string(&doc).expect("json");
        let back: Document = serde_json::from_str(&json).expect("back");
        assert_eq!(back.inserts.len(), 1);
        let ins = &back.inserts[0];
        assert_eq!(ins.name, "Prayer list");
        assert_eq!(ins.life, InsertLife::Dated);
        assert_eq!(
            ins.keep_through,
            Some(SimpleDate {
                year: 2026,
                month: 11,
                day: 8
            })
        );
        assert_eq!(ins.sheet.copies, 60);
        assert_eq!(ins.sheet.paper_note, "blue paper");
        assert!(ins.has_words());
    }

    #[test]
    fn a_document_written_before_loose_sheets_loads_with_none() {
        let doc = doc_with_panels(Vec::new());
        let mut value: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&doc).expect("json")).expect("value");
        value.as_object_mut().expect("object").remove("inserts");
        let back: Document = serde_json::from_value(value).expect("loads");
        assert!(back.inserts.is_empty());
    }

    #[test]
    fn an_insert_prints_as_a_document_on_the_hosts_fields() {
        let mut host = doc_with_panels(Vec::new());
        host.profile.name = "Grace Lutheran Church".into();
        host.fields.insert(
            "occasion_date_long".into(),
            "Sunday, October 18, 2026".into(),
        );
        let ins = sample_insert();
        let doc = ins.as_document(&host);
        // The sheet's own parts, the host's context.
        assert_eq!(doc.panels.len(), 1);
        assert_eq!(doc.stories.len(), 1);
        assert!(doc.inserts.is_empty(), "a sheet has no sheets of its own");
        assert_eq!(doc.sheet.preset, "half-letter");
        assert_eq!(doc.sheet.size, Size::new(396_000, 612_000));
        // Its words resolve against the host: the date and
        // the church name are the host's, not copies.
        let story = &doc.stories[0];
        assert_eq!(
            resolve_fields(&story.paragraphs[0].text(), &doc),
            "Prayers for Sunday, October 18, 2026 at Grace Lutheran Church"
        );
    }

    #[test]
    fn an_insert_with_no_words_says_so() {
        let mut ins = sample_insert();
        assert!(ins.has_words());
        ins.stories[0].paragraphs[0].set_text("   ");
        assert!(!ins.has_words());
        ins.stories.clear();
        assert!(!ins.has_words());
    }

    #[test]
    fn raw_offset_for_resolved_walks_past_tokens() {
        let mut doc = doc_with_panels(Vec::new());
        doc.profile.name = "Grace Lutheran Church".into(); // 21 chars
        let raw = "Welcome to {Church.name} today";
        // Resolved: "Welcome to Grace Lutheran Church today".
        // A position in plain text before the token maps 1:1.
        assert_eq!(raw_offset_for_resolved(raw, &doc, 5), 5);
        // The token's raw start.
        assert_eq!(raw_offset_for_resolved(raw, &doc, 11), 11);
        // Inside the filled-in value: the whole token
        // travels with the tail, so back to its raw start.
        assert_eq!(raw_offset_for_resolved(raw, &doc, 20), 11);
        // Just past the value: the token's raw end (24).
        assert_eq!(raw_offset_for_resolved(raw, &doc, 32), 24);
        // After the token, positions shift by the difference
        // (resolved " today" starts at 32, raw at 24).
        assert_eq!(raw_offset_for_resolved(raw, &doc, 33), 25);
        // The end maps to the end.
        assert_eq!(
            raw_offset_for_resolved(raw, &doc, 999),
            raw.len(),
            "past the end clamps to the raw end"
        );
    }
}
