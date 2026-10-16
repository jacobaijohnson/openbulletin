//! The substitution table and the resolution chain
//! (docs/FONTS.md, wanted fonts).
//!
//! Documents name the font they were made in, even when the
//! Font Library does not hold it: a bulletin made in Gill
//! Sans MT keeps "Gill Sans MT" on its styles forever. What
//! actually shapes and prints is decided HERE, at use time,
//! against the library as it stands at that moment:
//!
//! 1. the real family, when the library holds it (Exact);
//! 2. its metric twin, a free family with the same letter
//!    widths, when the library holds that (MetricTwin);
//! 3. its lookalike, a free family that looks close but
//!    wraps differently, always labeled a lookalike and
//!    never a twin (Lookalike);
//! 4. the house default, Liberation Sans (HouseDefault).
//!
//! Because the chain reads the library at use time, adding
//! the real font to the library upgrades every document
//! that names it, with no edit to any document. Every
//! consumer (the draft builder, the shaper, the PDF
//! writer, the app's painters, the Check step) walks this
//! one chain; the table lives here and nowhere else.

/// How a requested family was answered by the chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubstituteTier {
    /// The library holds the real family itself.
    Exact,
    /// A free family with identical letter widths stands in.
    MetricTwin,
    /// A free family that looks close stands in. Its widths
    /// differ, so lines may wrap in new places: the word
    /// "lookalike" is part of its name everywhere it is
    /// described, so nobody mistakes it for a twin.
    Lookalike,
    /// Nothing closer was available: the house default.
    HouseDefault,
}

/// The family every chain ends at when nothing closer is
/// available: bundled with the app, always present.
pub const HOUSE_DEFAULT_FAMILY: &str = "Liberation Sans";

/// Families bundled with the app (ob-text's bundled set):
/// available even when the library folder is empty, so the
/// chain counts them as present without being told.
pub const BUNDLED_FAMILIES: [&str; 2] = ["Liberation Sans", "Liberation Serif"];

/// What the table knows about one source family: the name
/// a person would search for, the free family that stands
/// in for it, and which tier that stand-in is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Substitution {
    /// The real font's name in display form, e.g.
    /// "Gill Sans MT" for a source name of "GillSansMT".
    pub display_name: String,
    /// The free family that stands in, e.g. "Cabin".
    pub substitute: String,
    /// Exact when the source name IS the free family (the
    /// substitute is the family itself), MetricTwin or
    /// Lookalike otherwise.
    pub tier: SubstituteTier,
}

struct Row {
    /// Normalized keys this row answers to: the spaced
    /// name and the compressed PostScript-style name.
    keys: &'static [&'static str],
    display: &'static str,
    substitute: &'static str,
    tier: SubstituteTier,
}

const fn exact(keys: &'static [&'static str], family: &'static str) -> Row {
    Row {
        keys,
        display: family,
        substitute: family,
        tier: SubstituteTier::Exact,
    }
}

const fn twin(keys: &'static [&'static str], display: &'static str, to: &'static str) -> Row {
    Row {
        keys,
        display,
        substitute: to,
        tier: SubstituteTier::MetricTwin,
    }
}

const ROWS: &[Row] = &[
    // Free families that are their own answer: a document
    // made in one of these wants the family itself.
    exact(&["liberation sans", "liberationsans"], "Liberation Sans"),
    exact(&["liberation serif", "liberationserif"], "Liberation Serif"),
    exact(&["carlito"], "Carlito"),
    exact(&["caladea"], "Caladea"),
    exact(&["gelasio"], "Gelasio"),
    exact(&["lora"], "Lora"),
    exact(&["playfair display", "playfairdisplay"], "Playfair Display"),
    exact(&["cabin"], "Cabin"),
    exact(&["eb garamond", "ebgaramond"], "EB Garamond"),
    exact(&["dejavu sans", "dejavusans"], "DejaVu Sans"),
    exact(&["bookman urw", "bookmanurw"], "Bookman URW"),
    exact(
        &["century schoolbook urw", "centuryschoolbookurw"],
        "Century Schoolbook URW",
    ),
    // Metric twins: same letter widths, so the rebuild is
    // line-for-line faithful (docs/FONTS.md).
    twin(&["arial", "arialmt"], "Arial", "Liberation Sans"),
    twin(&["helvetica"], "Helvetica", "Liberation Sans"),
    twin(
        &["times new roman", "timesnewroman"],
        "Times New Roman",
        "Liberation Serif",
    ),
    twin(&["calibri"], "Calibri", "Carlito"),
    twin(&["cambria"], "Cambria", "Caladea"),
    twin(&["georgia"], "Georgia", "Gelasio"),
    twin(&["book antiqua", "bookantiqua"], "Book Antiqua", "Gelasio"),
    twin(&["garamond"], "Garamond", "EB Garamond"),
    twin(
        &["bookman old style", "bookmanoldstyle"],
        "Bookman Old Style",
        "Bookman URW",
    ),
    twin(
        &["century schoolbook", "centuryschoolbook"],
        "Century Schoolbook",
        "Century Schoolbook URW",
    ),
    // Lookalikes: close in look, not in widths. Gill Sans
    // is the corpus's dominant headline face and has no
    // metric twin; Cabin (SIL Open Font License 1.1, by
    // Pablo Impallari, drawn after Edward Johnston's and
    // Eric Gill's own typefaces) is its free lookalike.
    Row {
        keys: &["gill sans mt", "gillsansmt", "gill sans", "gillsans"],
        display: "Gill Sans MT",
        substitute: "Cabin",
        tier: SubstituteTier::Lookalike,
    },
];

/// Reduce a source font name to the form the table matches
/// against: subset prefix off (a name like "UOETYK+GillSansMT"
/// is a sliced-out piece of GillSansMT), trimmed, lowered.
fn cleaned(source_name: &str) -> String {
    source_name
        .split('+')
        .next_back()
        .unwrap_or(source_name)
        .trim()
        .to_lowercase()
}

/// The table row a source name belongs to, when one is
/// known: the name up to its first dash or comma is its
/// base (the rest is a face suffix like "-Bold"), and the
/// longest key that matches the base, or begins the whole
/// cleaned name, wins.
fn row_for(source_name: &str) -> Option<&'static Row> {
    let clean = cleaned(source_name);
    let base = clean.split(['-', ',']).next().unwrap_or("").trim();
    let mut best: Option<(&Row, usize)> = None;
    for row in ROWS {
        for key in row.keys {
            let hit = base == *key || clean.starts_with(key);
            if hit && best.map(|(_, n)| key.len() > n).unwrap_or(true) {
                best = Some((row, key.len()));
            }
        }
    }
    best.map(|(row, _)| row)
}

/// What the table knows about a source font name: the
/// display name, the free stand-in, and its tier. None
/// when the name is unknown to the table (the caller then
/// keeps the name as written and the chain ends at the
/// house default until the real font arrives).
pub fn substitution_for(source_name: &str) -> Option<Substitution> {
    row_for(source_name).map(|row| Substitution {
        display_name: row.display.to_string(),
        substitute: row.substitute.to_string(),
        tier: row.tier,
    })
}

/// The name a person would search for: the table's
/// display form when the name is known ("Gill Sans MT"
/// for "UOETYK+GillSansMT"), else the source name with
/// its subset prefix off, as written.
pub fn display_name_for(source_name: &str) -> String {
    match row_for(source_name) {
        Some(row) => row.display.to_string(),
        None => source_name
            .split('+')
            .next_back()
            .unwrap_or(source_name)
            .trim()
            .to_string(),
    }
}

/// The chain's answer for one requested family: which
/// family to actually use, and on which tier it answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolution {
    /// The family to shape and print with.
    pub family: String,
    /// On which tier the chain answered.
    pub tier: SubstituteTier,
    /// The requested family in display form.
    pub display_name: String,
}

/// Is `name` among the available families? Names compare
/// the way font people write them: case and spacing do
/// not matter, and the bundled families always count.
/// Returns the available list's own spelling, so callers
/// use exactly the name the library carries.
fn available_spelling(name: &str, available: &[String]) -> Option<String> {
    let want = name.to_lowercase();
    for fam in BUNDLED_FAMILIES {
        if fam.to_lowercase() == want {
            return Some(fam.to_string());
        }
    }
    available
        .iter()
        .find(|f| f.trim().to_lowercase() == want)
        .cloned()
}

/// Walk the chain for one requested family against the
/// families available right now (the Font Library's scan,
/// or a shaper's registered families): exact first, then
/// the twin, then the lookalike, then the house default.
pub fn resolve_family(requested: &str, available: &[String]) -> Resolution {
    let display_name = display_name_for(requested);
    if let Some(family) = available_spelling(&display_name, available) {
        return Resolution {
            family,
            tier: SubstituteTier::Exact,
            display_name,
        };
    }
    // The library may carry the family under the spelling
    // the source file used rather than the display form.
    let raw = requested.split('+').next_back().unwrap_or(requested).trim();
    if !raw.eq_ignore_ascii_case(&display_name) {
        if let Some(family) = available_spelling(raw, available) {
            return Resolution {
                family,
                tier: SubstituteTier::Exact,
                display_name,
            };
        }
    }
    if let Some(row) = row_for(requested) {
        if row.tier != SubstituteTier::Exact {
            if let Some(family) = available_spelling(row.substitute, available) {
                return Resolution {
                    family,
                    tier: row.tier,
                    display_name,
                };
            }
        }
    }
    Resolution {
        family: HOUSE_DEFAULT_FAMILY.to_string(),
        tier: SubstituteTier::HouseDefault,
        display_name,
    }
}

/// The plain words every surface uses for a wanted font
/// (import review and the Check step share this sentence,
/// so the two can never drift apart).
pub fn wanted_font_words(name: &str, substituted_by: &str) -> String {
    format!(
        "This bulletin was made in {name}. We do not have that font, \
         so its words are shown in {substituted_by} for now. If you add \
         {name} in Fonts, this bulletin will use it by itself."
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn avail(names: &[&str]) -> Vec<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn exact_beats_twin_beats_lookalike_beats_default() {
        // Exact: the library holds Arial itself.
        let r = resolve_family("Arial", &avail(&["Arial", "Liberation Sans"]));
        assert_eq!(r.family, "Arial");
        assert_eq!(r.tier, SubstituteTier::Exact);
        // Twin: no Arial, but its twin is there.
        let r = resolve_family("ArialMT", &avail(&["Liberation Sans"]));
        assert_eq!(r.family, "Liberation Sans");
        assert_eq!(r.tier, SubstituteTier::MetricTwin);
        // Lookalike: Gill Sans MT answers with Cabin, and
        // the tier says lookalike, never twin.
        let r = resolve_family("GillSansMT", &avail(&["Cabin"]));
        assert_eq!(r.family, "Cabin");
        assert_eq!(r.tier, SubstituteTier::Lookalike);
        // Default: neither the font nor its lookalike.
        let r = resolve_family("GillSansMT", &avail(&[]));
        assert_eq!(r.family, "Liberation Sans");
        assert_eq!(r.tier, SubstituteTier::HouseDefault);
    }

    #[test]
    fn installing_the_real_font_upgrades_the_answer() {
        // The install-later promise at the chain itself:
        // the same request, before and after the real
        // family joins the available list.
        let before = resolve_family("UOETYK+GillSansMT", &avail(&["Cabin"]));
        assert_eq!(before.family, "Cabin");
        let after = resolve_family("UOETYK+GillSansMT", &avail(&["Cabin", "Gill Sans MT"]));
        assert_eq!(after.family, "Gill Sans MT");
        assert_eq!(after.tier, SubstituteTier::Exact);
    }

    #[test]
    fn names_normalize_the_way_sources_write_them() {
        assert_eq!(display_name_for("UOETYK+GillSansMT"), "Gill Sans MT");
        assert_eq!(display_name_for("GillSansMT-Bold"), "Gill Sans MT");
        assert_eq!(display_name_for("TimesNewRomanPSMT"), "Times New Roman");
        assert_eq!(display_name_for("ABCDEF+ArialMT"), "Arial");
        assert_eq!(display_name_for("Calibri"), "Calibri");
        // Unknown names keep their own spelling.
        assert_eq!(display_name_for("FooBarMT"), "FooBarMT");
        let sub = substitution_for("GillSansMT").expect("gill is in the table");
        assert_eq!(sub.substitute, "Cabin");
        assert_eq!(sub.tier, SubstituteTier::Lookalike);
        assert!(substitution_for("NoSuchFont").is_none());
    }

    #[test]
    fn bundled_families_count_as_available() {
        let r = resolve_family("Liberation Serif", &avail(&[]));
        assert_eq!(r.family, "Liberation Serif");
        assert_eq!(r.tier, SubstituteTier::Exact);
        // An exact-tier family that is neither bundled nor
        // in the library falls to the house default: the
        // chain never invents a stand-in for it.
        let r = resolve_family("Carlito", &avail(&[]));
        assert_eq!(r.family, "Liberation Sans");
        assert_eq!(r.tier, SubstituteTier::HouseDefault);
    }

    #[test]
    fn the_wanted_words_name_both_fonts() {
        let words = wanted_font_words("Gill Sans MT", "Cabin");
        assert!(words.contains("made in Gill Sans MT"));
        assert!(words.contains("shown in Cabin for now"));
        assert!(words.contains("will use it by itself"));
    }
}
