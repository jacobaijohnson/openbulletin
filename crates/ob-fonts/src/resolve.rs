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

const fn lookalike(keys: &'static [&'static str], display: &'static str, to: &'static str) -> Row {
    Row {
        keys,
        display,
        substitute: to,
        tier: SubstituteTier::Lookalike,
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
        &["century schoolbook", "centuryschoolbook"],
        "Century Schoolbook",
        "Century Schoolbook URW",
    ),
    // Lookalikes: close in look, not in widths. Gill Sans
    // is the corpus's dominant headline face and has no
    // metric twin; Cabin (SIL Open Font License 1.1, by
    // Pablo Impallari, drawn after Edward Johnston's and
    // Eric Gill's own typefaces) is its free lookalike.
    lookalike(
        &["gill sans mt", "gillsansmt", "gill sans", "gillsans"],
        "Gill Sans MT",
        "Cabin",
    ),
    // Bookman URW is a close free relative of Bookman Old
    // Style, but nobody here has verified the letter
    // widths match, so it is a lookalike, never a twin
    // (round 3, R3-18: an unverified metric claim is a
    // promise the import review would repeat as fact).
    lookalike(
        &["bookman old style", "bookmanoldstyle"],
        "Bookman Old Style",
        "Bookman URW",
    ),
];

/// Face words: the part of a source name that names one
/// face of a family rather than the family itself. Source
/// files write them after a dash or comma
/// ("Verdana-Bold"), spaced ("DejaVu Sans Bold"), or
/// both at once ("Verdana-Bold Italic"). After a dash or
/// comma the full set below comes off, Roman and Regular
/// included: in the PostScript-style names our sources
/// write that way, Roman is a face word, so
/// "Acme-Roman" bases to "Acme". After a SPACE only the
/// narrow set (SPACED_FACE_SUFFIXES) comes off: a spaced
/// Roman or Regular is as likely to be part of the
/// family itself ("Times New Roman", "Latin Modern
/// Roman"), and stripping it named a family that does
/// not exist, one the office could never install and
/// whose table row could never answer (round 4, R4-1).
/// The family is the name a person can go and install:
/// that is the semantics the install-later promise
/// needs.
const FACE_SUFFIXES: &[&str] = &[
    "bolditalic",
    "boldoblique",
    "bold italic",
    "bold",
    "italic",
    "oblique",
    "regular",
    "roman",
];

/// The face words that come off after a space: only the
/// weight and slant words, which never end a real
/// family's own name the way Roman and Regular can.
const SPACED_FACE_SUFFIXES: &[&str] = &[
    "bolditalic",
    "boldoblique",
    "bold italic",
    "bold",
    "italic",
    "oblique",
];

/// PostScript name tails: markers glued onto compressed
/// names by the tools that write them ("TimesNewRomanPSMT"
/// is Times New Roman). They say nothing about the design,
/// so a table row may answer past them. Design words
/// (Narrow, Black, Light) are not tails: skipping those
/// is what would let a different typeface claim a row
/// (round 3, R3-2).
const PS_TAILS: &[&str] = &["psmt", "ps", "mt"];

/// Is this trailing word (spaces removed, lowered) one
/// of `suffixes`, or one of them with a PostScript tail
/// glued on ("BoldMT", from "TimesNewRomanPS-BoldMT")?
fn is_suffix_tail(tail: &str, suffixes: &[&str]) -> bool {
    let squashed: String = tail.to_lowercase().chars().filter(|c| *c != ' ').collect();
    suffixes.iter().any(|s| {
        let s: String = s.chars().filter(|c| *c != ' ').collect();
        squashed == s
            || squashed
                .strip_prefix(&s)
                .map(|rest| PS_TAILS.contains(&rest))
                .unwrap_or(false)
    })
}

/// A face tail after a dash or comma: the full set.
fn is_face_tail(tail: &str) -> bool {
    is_suffix_tail(tail, FACE_SUFFIXES)
}

/// A face tail after a space: the narrow set only, so a
/// spaced Roman or Regular stays part of the name.
fn is_spaced_face_tail(tail: &str) -> bool {
    is_suffix_tail(tail, SPACED_FACE_SUFFIXES)
}

/// The base family a source name names: subset prefix
/// off, then trailing face words off. Which words come
/// off depends on the separator in front of them (see
/// FACE_SUFFIXES): a dash or comma takes the full set,
/// a space only the weight and slant words, so "Times
/// New Roman" bases to itself while "Verdana-Bold" and
/// "DejaVu Sans Bold" base to "Verdana" and "DejaVu
/// Sans". Case and spacing are preserved, so the result
/// is a name a person would search for. This holds
/// whether or not the family is in the table (round 3,
/// R3-1: the long tail of names no table row knows is
/// the common case, and its wanted-font record must
/// name an installable family, not a face).
pub fn base_name(source_name: &str) -> String {
    let mut name = source_name
        .split('+')
        .next_back()
        .unwrap_or(source_name)
        .trim()
        .to_string();
    // One name can carry two face words ("Bold Italic");
    // strip while a separator precedes a face word and a
    // real head remains. The separator in front of the
    // tail decides which set judges it: the byte at idx
    // is one of the three ASCII separators.
    while let Some(idx) = name.rfind(['-', ',', ' ']) {
        let tail = &name[idx + 1..];
        let strip = if name.as_bytes()[idx] == b' ' {
            is_spaced_face_tail(tail)
        } else {
            is_face_tail(tail)
        };
        if !strip {
            break;
        }
        let head = name[..idx].trim_end_matches(['-', ',', ' ']).trim();
        if head.is_empty() {
            break;
        }
        name = head.to_string();
    }
    name
}

/// The table row a source name belongs to, when one is
/// known. A row answers when the name's base IS one of
/// its keys, or begins with a key and continues only
/// with a face word or a PostScript tail: the rest of
/// the name must say "same design, another face" or
/// nothing at all. Anything more ("Arial Narrow",
/// "Calibri Light", "Loraine") is a different typeface
/// and earns no row, least of all a metric twin's
/// promise of identical letter widths (round 3, R3-2).
/// The longest matching key wins.
fn row_for(source_name: &str) -> Option<&'static Row> {
    let base = base_name(source_name).to_lowercase();
    let mut best: Option<(&Row, usize)> = None;
    for row in ROWS {
        for key in row.keys {
            let hit = base == *key
                || base
                    .strip_prefix(key)
                    .map(|rest| {
                        let rest: String = rest.trim().chars().filter(|c| *c != ' ').collect();
                        is_face_tail(&rest) || PS_TAILS.contains(&rest.as_str())
                    })
                    .unwrap_or(false);
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
/// for "UOETYK+GillSansMT"), else the name's base family
/// (see base_name): "Verdana-Bold" records "Verdana",
/// the family a person can actually install, never a
/// face of it (round 3, R3-1).
pub fn display_name_for(source_name: &str) -> String {
    match row_for(source_name) {
        Some(row) => row.display.to_string(),
        None => base_name(source_name),
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
/// Exact tries the FULL name the source file stated
/// before any stripped form (round 4, R4-1): a family
/// really named "Acme Bold" or "Latin Modern Roman"
/// keeps its identity on the document's styles, and when
/// the library holds that family it must win over the
/// base the name strips to, or the wrong typeface
/// prints and installing the real font never upgrades
/// anything.
pub fn resolve_family(requested: &str, available: &[String]) -> Resolution {
    let display_name = display_name_for(requested);
    // The full name first: the spelling the source file
    // used, subset tag off, nothing stripped.
    let raw = requested.split('+').next_back().unwrap_or(requested).trim();
    if let Some(family) = available_spelling(raw, available) {
        return Resolution {
            family,
            tier: SubstituteTier::Exact,
            display_name,
        };
    }
    // Then the display form: the name a person would
    // search for ("Gill Sans MT" for "GillSansMT").
    if !display_name.eq_ignore_ascii_case(raw) {
        if let Some(family) = available_spelling(&display_name, available) {
            return Resolution {
                family,
                tier: SubstituteTier::Exact,
                display_name,
            };
        }
    }
    // Or under the name's base: the source file wrote
    // a face ("Verdana-Bold") and the library's family is
    // the base ("Verdana"). This attempt is what makes
    // installing the real font upgrade a document whose
    // recorded name still wears its face (round 3, R3-1).
    let base = base_name(requested);
    if !base.eq_ignore_ascii_case(&display_name) && !base.eq_ignore_ascii_case(raw) {
        if let Some(family) = available_spelling(&base, available) {
            return Resolution {
                family,
                tier: SubstituteTier::Exact,
                display_name,
            };
        }
    }
    // The table's stand-in. row_for reads the full name
    // (it bases the name itself), so a row is consulted
    // for "Times New Roman" whole, never for a stripped
    // form that lost the family's own last word.
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

/// The Check step's words when a wanted font's styles
/// do not all resolve to the same stand-in (round 6,
/// R6-9): the whole-bulletin claim would be false for
/// every style but one, so the claim is scoped to SOME
/// of the words, and the stand-in named is the worst
/// style's — the one the finding is about.
pub fn wanted_font_words_some(name: &str, substituted_by: &str) -> String {
    format!(
        "This bulletin was made in {name}. We do not have that font, \
         so some of its words are shown in {substituted_by} for now. If you add \
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
    fn every_name_bases_to_its_family() {
        // R3-1: the base extraction is not a table privilege.
        // Glued, comma, and spaced face words all come off,
        // for names the table has never heard of.
        assert_eq!(base_name("Verdana-Bold"), "Verdana");
        assert_eq!(base_name("Verdana,Bold"), "Verdana");
        assert_eq!(base_name("Verdana-BoldItalic"), "Verdana");
        assert_eq!(base_name("DejaVu Sans Bold"), "DejaVu Sans");
        assert_eq!(base_name("DejaVu Sans Bold Italic"), "DejaVu Sans");
        assert_eq!(base_name("UOETYK+Verdana-Bold"), "Verdana");
        assert_eq!(display_name_for("Verdana-Bold"), "Verdana");
        assert_eq!(display_name_for("DejaVu Sans Bold"), "DejaVu Sans");
        // After a dash or comma, Roman and Regular are
        // face words (round 3, R3-1, kept). After a space
        // they stay: they can be the family's own last
        // word (round 4, R4-1).
        assert_eq!(base_name("Acme-Roman"), "Acme");
        assert_eq!(base_name("Acme,Regular"), "Acme");
        assert_eq!(base_name("Acme Roman"), "Acme Roman");
        assert_eq!(base_name("Foo Regular"), "Foo Regular");
        assert_eq!(base_name("Times New Roman"), "Times New Roman");
        // Design words are not face words: they stay.
        assert_eq!(base_name("Arial Narrow"), "Arial Narrow");
        assert_eq!(base_name("FooBarMT"), "FooBarMT");
    }

    #[test]
    fn installing_a_long_tail_font_upgrades_the_document() {
        // R3-1 end to end: a bulletin made in Verdana,
        // recorded from its bold face's name, picks the
        // real family up when the office installs it.
        let before = resolve_family("Verdana-Bold", &avail(&[]));
        assert_eq!(before.tier, SubstituteTier::HouseDefault);
        let after = resolve_family("Verdana-Bold", &avail(&["Verdana"]));
        assert_eq!(after.family, "Verdana");
        assert_eq!(after.tier, SubstituteTier::Exact);
        assert_eq!(after.display_name, "Verdana");
        // The spaced shape Scribus writes behaves the same.
        let after = resolve_family("DejaVu Sans Bold", &avail(&["DejaVu Sans"]));
        assert_eq!(after.family, "DejaVu Sans");
        assert_eq!(after.tier, SubstituteTier::Exact);
    }

    #[test]
    fn rows_answer_only_the_family_and_its_faces() {
        // R3-2: a different typeface that merely begins
        // with a known name earns no row at all.
        for name in [
            "Arial Narrow",
            "Arial Black",
            "Calibri Light",
            "Loraine",
            "Cabinet Grotesk",
        ] {
            assert!(substitution_for(name).is_none(), "{name} must earn no row");
        }
        // So it cannot claim a twin's widths: installed or
        // house default are its only answers.
        let r = resolve_family("Arial Narrow", &avail(&["Liberation Sans"]));
        assert_eq!(r.tier, SubstituteTier::HouseDefault);
        let r = resolve_family("Arial Narrow", &avail(&["Arial Narrow"]));
        assert_eq!(r.tier, SubstituteTier::Exact);
        // The family plus a face word still answers.
        assert_eq!(
            substitution_for("Arial-Bold").map(|s| s.tier),
            Some(SubstituteTier::MetricTwin)
        );
        assert_eq!(
            substitution_for("Arial Bold").map(|s| s.tier),
            Some(SubstituteTier::MetricTwin)
        );
        // And the PostScript tails still answer.
        assert_eq!(
            substitution_for("TimesNewRomanPSMT").map(|s| s.display_name),
            Some("Times New Roman".to_string())
        );
        assert_eq!(
            substitution_for("TimesNewRomanPS-BoldMT").map(|s| s.display_name),
            Some("Times New Roman".to_string())
        );
    }

    #[test]
    fn bookman_old_style_answers_with_a_lookalike() {
        // R3-18: Bookman URW was never verified as a
        // metric twin, so the row says lookalike.
        let sub = substitution_for("Bookman Old Style").expect("bookman row");
        assert_eq!(sub.substitute, "Bookman URW");
        assert_eq!(sub.tier, SubstituteTier::Lookalike);
        // Bookman URW itself is still its own answer.
        let r = resolve_family("Bookman URW", &avail(&["Bookman URW"]));
        assert_eq!(r.tier, SubstituteTier::Exact);
    }

    #[test]
    fn times_new_roman_reaches_its_twin_and_itself() {
        // R4-1 (a): the spaced name the .pub fixtures
        // carry bases to itself, so its Liberation Serif
        // row answers, and the real font wins when the
        // library holds it. Before the fix the name based
        // to "Times New" and neither could ever happen:
        // a serif bulletin laid out in a sans.
        assert_eq!(display_name_for("Times New Roman"), "Times New Roman");
        let r = resolve_family("Times New Roman", &avail(&["Liberation Serif"]));
        assert_eq!(r.family, "Liberation Serif");
        assert_eq!(r.tier, SubstituteTier::MetricTwin);
        let r = resolve_family(
            "Times New Roman",
            &avail(&["Liberation Serif", "Times New Roman"]),
        );
        assert_eq!(r.family, "Times New Roman");
        assert_eq!(r.tier, SubstituteTier::Exact);
        // Its faces still strip, by dash and by space.
        assert_eq!(base_name("Times New Roman-Regular"), "Times New Roman");
        assert_eq!(base_name("Times New Roman Bold"), "Times New Roman");
        assert_eq!(
            substitution_for("Times New Roman Bold").map(|s| s.tier),
            Some(SubstituteTier::MetricTwin)
        );
    }

    #[test]
    fn the_full_name_wins_over_the_base_it_strips_to() {
        // R4-1 (b): a family really named "Acme Bold",
        // installed beside "Acme". A document set in it
        // resolves to it, not to the base the name strips
        // to (before the fix the display attempt answered
        // "Acme" first and the wrong typeface printed).
        let both = avail(&["Acme", "Acme Bold"]);
        let r = resolve_family("Acme Bold", &both);
        assert_eq!(r.family, "Acme Bold");
        assert_eq!(r.tier, SubstituteTier::Exact);
        // With only the base installed, the base answers,
        // as it always has.
        let r = resolve_family("Acme Bold", &avail(&["Acme"]));
        assert_eq!(r.family, "Acme");
        assert_eq!(r.tier, SubstituteTier::Exact);
    }

    #[test]
    fn a_family_ending_in_roman_upgrades_when_installed() {
        // R4-1 (c): "Latin Modern Roman" names itself, so
        // the wanted record is installable and the real
        // font upgrades the document when it arrives.
        // Before the fix the record said "Latin Modern",
        // a family that does not exist under that name.
        assert_eq!(display_name_for("Latin Modern Roman"), "Latin Modern Roman");
        let before = resolve_family("Latin Modern Roman", &avail(&[]));
        assert_eq!(before.tier, SubstituteTier::HouseDefault);
        assert_eq!(before.display_name, "Latin Modern Roman");
        let after = resolve_family("Latin Modern Roman", &avail(&["Latin Modern Roman"]));
        assert_eq!(after.family, "Latin Modern Roman");
        assert_eq!(after.tier, SubstituteTier::Exact);
    }

    #[test]
    fn the_wanted_words_name_both_fonts() {
        let words = wanted_font_words("Gill Sans MT", "Cabin");
        assert!(words.contains("made in Gill Sans MT"));
        assert!(words.contains("shown in Cabin for now"));
        assert!(words.contains("will use it by itself"));
    }
}
