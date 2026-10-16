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
    /// travel under. Adding the same bytes twice is one asset.
    pub fn add_asset(
        &mut self,
        bytes: &[u8],
        kind: AssetKind,
        media_type: &str,
        rights: RightsRecord,
    ) -> String {
        let hash = sha256_hex(bytes);
        if !self.assets.iter().any(|a| a.sha256 == hash) {
            self.assets.push(Asset {
                sha256: hash.clone(),
                kind,
                media_type: media_type.to_string(),
                rights,
            });
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
    ///    slug, are both stored this way at times);
    /// 2. the built-in alias: for a built-in slug, the
    ///    slug its display name derives (and the other
    ///    way round), because the two differ for
    ///    "Children's message";
    /// 3. the profile position bridge: a built-in line
    ///    the church renamed in its profile is the same
    ///    line, at the same position in the serving
    ///    lines, under its new name (round 3, R3-3).
    ///    A renamed line's token reads the value stored
    ///    under the new name, and the renamed line's
    ///    row reads a value still stored under the
    ///    built-in slug, so neither side can strand the
    ///    names the other side wrote. The profile save
    ///    also moves the stored value itself
    ///    (`migrate_serving_line_renames`); the bridge
    ///    covers documents renamed before that move
    ///    existed, and templates, which print by token;
    /// 4. the legacy weekly fields, for a document that
    ///    has not been through the load migration
    ///    (importers and tests build documents in
    ///    memory).
    ///
    /// Empty when the line holds no names this week.
    pub fn participant_value(&self, slug: &str) -> String {
        if let Some(v) = self.participants.get(slug) {
            return v.clone();
        }
        for (bslug, bname) in PARTICIPANT_FIELDS {
            let derived = serving_slug(bname);
            if bslug == slug {
                if let Some(v) = self.participants.get(&derived) {
                    return v.clone();
                }
            }
            if derived == slug {
                if let Some(v) = self.participants.get(bslug) {
                    return v.clone();
                }
            }
        }
        for (i, (bslug, bname)) in PARTICIPANT_FIELDS.iter().enumerate() {
            let Some(shown) = self.profile.serving_lines.get(i) else {
                continue;
            };
            if shown.eq_ignore_ascii_case(bname) || builtin_serving_slug(shown).is_some() {
                // The position holds a built-in name (this
                // line's own, or another line's after a
                // reorder): no rename to bridge.
                continue;
            }
            let shown_slug = serving_slug(shown);
            if slug == *bslug || slug == serving_slug(bname) {
                if let Some(v) = self.participants.get(&shown_slug) {
                    return v.clone();
                }
            }
            if slug == shown_slug {
                if let Some(v) = self.participants.get(*bslug) {
                    return v.clone();
                }
                if let Some(v) = self.participants.get(&serving_slug(bname)) {
                    return v.clone();
                }
            }
        }
        self.fields.get(slug).cloned().unwrap_or_default()
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

/// The serving lines a church starts with: the six
/// built-in display names, in their built-in order.
pub fn default_serving_lines() -> Vec<String> {
    PARTICIPANT_FIELDS
        .iter()
        .map(|(_, name)| name.to_string())
        .collect()
}

/// Move stored serving names across a profile edit
/// that renamed lines (round 3, R3-3). The profile
/// editor works on the list of names, so a rename
/// presents as the same POSITION carrying a new name:
/// for each position where the name changed, a value
/// stored under the old name's keys moves to the new
/// name's slug, and the old keys are cleared. The old
/// keys are the old name's shown slug, its built-in
/// slug when the old name is a built-in, and the
/// built-in slug of the line at that position (a
/// rename of a rename can leave the value under the
/// position's permanent slug). A value already stored
/// under the new name's slug is the fresher typing
/// and is never overwritten. Positions past the
/// shorter list (lines added or removed around the
/// edit) are left alone, and values under keys no
/// rename points at are untouched. Pure over the two
/// lists and the map, so the rule is testable without
/// the app; the app calls it when the profile's
/// serving lines are saved.
pub fn migrate_serving_line_renames(
    old_lines: &[String],
    new_lines: &[String],
    participants: &mut BTreeMap<String, String>,
) {
    let common = old_lines.len().min(new_lines.len());
    for i in 0..common {
        let old_name = old_lines[i].trim();
        let new_name = new_lines[i].trim();
        if old_name == new_name {
            continue;
        }
        let target = serving_slug(new_name);
        let mut sources: Vec<String> = vec![serving_slug(old_name)];
        if let Some(bslug) = builtin_serving_slug(old_name) {
            if !sources.contains(&bslug.to_string()) {
                sources.push(bslug.to_string());
            }
        }
        if let Some((bslug, bname)) = PARTICIPANT_FIELDS.get(i) {
            if !sources.contains(&bslug.to_string()) {
                sources.push(bslug.to_string());
            }
            let derived = serving_slug(bname);
            if !sources.contains(&derived) {
                sources.push(derived);
            }
        }
        let found = sources
            .iter()
            .filter(|k| k.as_str() != target)
            .find_map(|k| participants.get(k).cloned());
        for k in &sources {
            if *k != target {
                participants.remove(k);
            }
        }
        if let Some(value) = found {
            participants.entry(target).or_insert(value);
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
        migrate_serving_line_renames(&old_lines, &new_lines, &mut doc.participants);
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
        migrate_serving_line_renames(&old_lines, &new_lines, &mut doc.participants);
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
        migrate_serving_line_renames(&lines_now, &lines_now, &mut doc.participants);
        assert_eq!(doc.participants, before);
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
