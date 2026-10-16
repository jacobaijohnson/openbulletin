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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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
                // next year it falls (Feb 29 may skip).
                let mut found = None;
                for y in today.year..=today.year + 4 {
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ChurchProfile {
    pub name: String,
    pub address: String,
    pub service_times: String,
    pub website: String,
    pub giving_url: String,
    pub pastor: String,
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
}

impl Document {
    /// Clamp the numbers a hostile or hand-edited file could set
    /// to extremes (a font size of i64::MAX used to overflow the
    /// PDF writer's arithmetic and panic — closing audit). The
    /// bounds are far outside anything a real bulletin uses.
    pub fn sanitized(mut self) -> Self {
        for style in &mut self.styles {
            style.font_size = style.font_size.clamp(500, 288_000);
            style.space_after = style.space_after.clamp(0, 144_000);
        }
        for panel in &mut self.panels {
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
            }
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
fn field_tokens(doc: &Document) -> Vec<(&'static str, String)> {
    let p = &doc.profile;
    let get = |k: &str| doc.fields.get(k).cloned().unwrap_or_default();
    vec![
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
            .find(|(tok, _)| rest.starts_with(tok))
            .map(|(tok, val)| (*tok, val));
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
        };
        assert_eq!(
            resolve_fields("{Church.name} - {Sunday.name}, {Sunday.date}", &doc),
            "Grace Lutheran - Twentieth Sunday after Pentecost, October 18, 2026"
        );
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
}
