//! Per-song license permission lines and the usage report
//! (round 51, G-08d).
//!
//! A music file's rights line may state the church's
//! reprint license and its number ("Reprinted under CCLI
//! license #123456", "Used with permission under ONE
//! LICENSE #A-123456"). Until this module, that number
//! went nowhere: `rights_from_music` left `licensor_ref`
//! empty and no code composed a permission line carrying
//! it. Now the number is read out of the rights line, the
//! distinct permission line is composed from it, and the
//! usage report lists every licensed song in a bulletin
//! for the office to report from. A licensed song with
//! no number recorded warns (the draft's CheckThis item,
//! Check's license note) and still prints — a missing
//! number is never a block.

use crate::draft::license_from_rights;
use crate::parse::MusicInfo;
use ob_model::compose_permission_line;
use ob_model::hymns::{hymn_license_uses, HymnMemory};
use ob_model::{AssetKind, Document, LicenseKind};

/// Which licensor a rights line names, when it names one:
/// "CCLI", "ONE LICENSE", or "" when neither appears.
/// The check is deliberately name-based, never guessed
/// from the license kind alone.
pub fn licensor_from_rights(rights: &str) -> &'static str {
    let line = rights.to_ascii_lowercase();
    if line.contains("one license") || line.contains("onelicense") {
        "ONE LICENSE"
    } else if line.contains("ccli") {
        "CCLI"
    } else {
        ""
    }
}

/// The license number a rights line states, when it
/// states one: the digits after a CCLI mention, or the
/// "A-..." number after a ONE LICENSE mention (the A-
/// prefix normalized to upper case). The words "license"
/// or "licence" and a "#" between the name and the
/// number are skipped. "" when the line names a licensor
/// but states no number, or names none at all.
pub fn licensor_ref_from_rights(rights: &str) -> String {
    let lower = rights.to_ascii_lowercase();
    let one_license = lower.contains("one license") || lower.contains("onelicense");
    let marker = if one_license {
        lower
            .find("one license")
            .or_else(|| lower.find("onelicense"))
    } else if lower.contains("ccli") {
        lower.find("ccli")
    } else {
        None
    };
    let Some(at) = marker else {
        return String::new();
    };
    let marker_len = if one_license {
        if lower[at..].starts_with("one license") {
            11
        } else {
            10
        }
    } else {
        4
    };
    let bytes = lower.as_bytes();
    let mut i = at + marker_len;
    loop {
        while i < bytes.len() && !bytes[i].is_ascii_alphanumeric() {
            i += 1;
        }
        if lower[i..].starts_with("license") || lower[i..].starts_with("licence") {
            i += 7;
            continue;
        }
        break;
    }
    let mut prefix = "";
    if one_license
        && i + 2 < bytes.len()
        && bytes[i] == b'a'
        && bytes[i + 1] == b'-'
        && bytes[i + 2].is_ascii_digit()
    {
        prefix = "A-";
        i += 2;
    }
    let start = i;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i == start {
        return String::new();
    }
    let digits = &lower[start..i];
    if prefix.is_empty() {
        digits.to_string()
    } else {
        format!("{prefix}{digits}")
    }
}

/// The copyright statement a rights line carries, when it
/// carries one: "© YEAR Owner", normalized from "©",
/// "(c)", or "Copyright" spellings. The owner runs to
/// the licensor mention or to a use clause ("Reprinted
/// under…", "Used with permission…", "All rights
/// reserved"), so the permission sentence never leaks
/// into the copyright. None when the line states no
/// copyright year at all.
pub fn copyright_from_rights(rights: &str) -> Option<String> {
    let lower = rights.to_ascii_lowercase();
    let mut candidates: Vec<usize> = Vec::new();
    if let Some(at) = lower.find('©') {
        candidates.push(at);
    }
    if let Some(at) = lower.find("(c)") {
        candidates.push(at);
    }
    if let Some(at) = lower.find("copyright") {
        candidates.push(at);
    }
    candidates.sort_unstable();
    let at = *candidates.first()?;
    let mut i = at;
    if lower[i..].starts_with('©') {
        i += '©'.len_utf8();
    } else if lower[i..].starts_with("(c)") {
        i += 3;
    } else {
        i += "copyright".len();
    }
    let bytes = lower.as_bytes();
    while i < bytes.len() && !bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i + 4 > bytes.len() || !lower[i..i + 4].bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let year: u32 = lower[i..i + 4].parse().ok()?;
    if !(1900..=2099).contains(&year) {
        return None;
    }
    let mut rest = &rights[i + 4..];
    // Cut at the licensor mention, if one follows.
    let rest_lower = rest.to_ascii_lowercase();
    for marker in ["one license", "onelicense", "ccli"] {
        if let Some(cut) = rest_lower.find(marker) {
            rest = &rest[..cut];
            break;
        }
    }
    // Cut at a use clause ("Reprinted under…", "Used with
    // permission…", "All rights reserved") at a sentence
    // boundary, so "J. R. Smith" keeps its initials.
    let rest_lower = rest.to_ascii_lowercase();
    for (j, _) in rest_lower.match_indices(". ") {
        let after = rest_lower[j + 2..].trim_start();
        if after.starts_with("reprinted")
            || after.starts_with("used")
            || after.starts_with("all rights")
            || after.starts_with("permission")
        {
            rest = &rest[..j];
            break;
        }
    }
    let owner = rest
        .trim()
        .trim_start_matches([' ', ',', ';', ':', '-'])
        .trim_end_matches([' ', '.', ',', ';'])
        .trim();
    if owner.is_empty() {
        Some(format!("© {year}"))
    } else {
        Some(format!("© {year} {owner}"))
    }
}

/// All the file's authors in credit-line order and labels
/// ("Words: … Music: …"), the way hymnals print them.
/// The CCLI permission line carries every author, not
/// just the composer the rights record kept.
fn authors_line(info: &MusicInfo) -> String {
    let mut parts: Vec<String> = Vec::new();
    for (kind, label) in [
        ("lyricist", "Words"),
        ("composer", "Music"),
        ("arranger", "Arranged"),
        ("poet", "Words"),
        ("translator", "Translated"),
    ] {
        if let Some(name) = info.creator(kind) {
            parts.push(format!("{label}: {name}"));
        }
    }
    if parts.is_empty() {
        String::new()
    } else {
        let joined = parts.join(". ");
        format!("{joined}.")
    }
}

/// The distinct per-song permission line for a music file
/// (round 51, G-08d): CCLI carries the full title, all
/// authors, the copyright year and owner, and the
/// permission sentence with the church's license number;
/// ONE LICENSE carries the copyright line plus "Used with
/// permission under ONE LICENSE #A-…". None when the
/// file is not a licensed reprint, or when it states no
/// license number — the caller warns in that case.
pub fn permission_line_for_music(info: &MusicInfo) -> Option<String> {
    let rights = info.rights.as_deref().unwrap_or("");
    if license_from_rights(Some(rights)) != LicenseKind::LicensedReprint {
        return None;
    }
    let licensor = licensor_from_rights(rights);
    let number = licensor_ref_from_rights(rights);
    if number.is_empty() {
        return None;
    }
    let title = info.title.clone().unwrap_or_default();
    let authors = authors_line(info);
    let copyright = copyright_from_rights(rights).unwrap_or_default();
    compose_permission_line(&title, &authors, &copyright, licensor, &number)
}

/// One entry of the license usage report (round 51,
/// G-08d): a licensed song the bulletin uses, with the
/// number to report or the gap to fill.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsageEntry {
    /// The song's title, else the file it came in on.
    pub title: String,
    /// Where it was used: "Music file: <name>" or
    /// "Hymn: <shown>".
    pub source: String,
    /// "CCLI", "ONE LICENSE", or "church license" when
    /// the record names no licensor.
    pub licensor: String,
    /// The license number to report; None when the office
    /// has not recorded one.
    pub number: Option<String>,
    /// The copyright line as recorded, when one is.
    pub copyright_line: String,
    /// Licensed, but no number recorded: the entry still
    /// surfaces, flagged, so the gap is visible.
    pub missing_number: bool,
}

/// The license usage report for a bulletin (round 51,
/// G-08d): every licensed song it uses — each music-file
/// asset under a reprint license, then each licensed
/// hymn in the order of service — in document order.
/// A song sung twice is one entry (the permission line
/// prints once); a licensed song with no number recorded
/// still surfaces, flagged `missing_number`, so the
/// office sees the gap instead of a silently short
/// report. Unlicensed and unknown songs are not entries:
/// there is nothing to report for them.
pub fn usage_report(doc: &Document, memory: &HymnMemory) -> Vec<UsageEntry> {
    let mut out: Vec<UsageEntry> = Vec::new();
    for asset in &doc.assets {
        if asset.kind != AssetKind::Music {
            continue;
        }
        if asset.rights.license != LicenseKind::LicensedReprint {
            continue;
        }
        let rights = &asset.rights;
        let licensor = licensor_from_rights(&rights.copyright_line);
        let licensor = if licensor.is_empty() {
            "church license".to_string()
        } else {
            licensor.to_string()
        };
        let number = if rights.licensor_ref.trim().is_empty() {
            None
        } else {
            Some(rights.licensor_ref.clone())
        };
        let missing_number = number.is_none();
        let title = if rights.title.trim().is_empty() {
            rights.source.clone()
        } else {
            rights.title.clone()
        };
        let source = format!("Music file: {}", rights.source);
        out.push(UsageEntry {
            title,
            source,
            licensor,
            number,
            copyright_line: rights.copyright_line.clone(),
            missing_number,
        });
    }
    for hymn_use in hymn_license_uses(doc, memory) {
        let licensor = if hymn_use.license.licensor.trim().is_empty() {
            "church license".to_string()
        } else {
            hymn_use.license.licensor.clone()
        };
        let number = if hymn_use.license.number.trim().is_empty() {
            None
        } else {
            Some(hymn_use.license.number.clone())
        };
        let source = format!("Hymn: {}", hymn_use.shown);
        out.push(UsageEntry {
            title: hymn_use.title,
            source,
            licensor,
            number,
            copyright_line: hymn_use.license.copyright_line.clone(),
            missing_number: hymn_use.missing_number,
        });
    }
    out
}

/// One usage entry as a single line for Check's note:
/// "Title — CCLI #123456 (Music file: x.musicxml)", or
/// with the gap named when no number is recorded.
pub fn format_usage_entry(entry: &UsageEntry) -> String {
    match &entry.number {
        Some(number) => format!(
            "{} — {} #{} ({})",
            entry.title, entry.licensor, number, entry.source
        ),
        None => format!(
            "{} — {} license number not recorded ({})",
            entry.title, entry.licensor, entry.source
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::{parse_musicxml, TINY_SCORE};
    use ob_model::hymns::{HymnEntry, HymnLicense};
    use ob_model::{ChurchProfile, Paragraph, RightsRecord, Story};

    fn info_with(rights: &str, title: &str) -> MusicInfo {
        let mut info = parse_musicxml(TINY_SCORE).expect("tiny score parses");
        info.rights = Some(rights.to_string());
        info.title = Some(title.to_string());
        info.creators = vec![
            ("lyricist".to_string(), "Jane Doe".to_string()),
            ("composer".to_string(), "John Smith".to_string()),
        ];
        info
    }

    #[test]
    fn license_numbers_read_out_of_rights_lines() {
        assert_eq!(
            licensor_ref_from_rights("Reprinted under CCLI license #123456"),
            "123456"
        );
        assert_eq!(licensor_ref_from_rights("CCLI #987"), "987");
        assert_eq!(licensor_ref_from_rights("ccli 5555"), "5555");
        assert_eq!(
            licensor_ref_from_rights("Used with permission under ONE LICENSE #A-123456"),
            "A-123456"
        );
        assert_eq!(
            licensor_ref_from_rights("© 2020 John Smith, One License #a-42"),
            "A-42"
        );
        assert_eq!(
            licensor_ref_from_rights("Public domain in the file's own words"),
            ""
        );
        assert_eq!(licensor_ref_from_rights("Reprinted under CCLI license"), "");
        assert_eq!(licensor_ref_from_rights(""), "");
    }

    #[test]
    fn licensors_named_by_name_never_guessed() {
        assert_eq!(
            licensor_from_rights("Reprinted under CCLI license #1"),
            "CCLI"
        );
        assert_eq!(
            licensor_from_rights("Used with permission under ONE LICENSE #A-1"),
            "ONE LICENSE"
        );
        assert_eq!(licensor_from_rights("Public domain"), "");
    }

    #[test]
    fn copyright_lines_parse_year_and_owner() {
        assert_eq!(
            copyright_from_rights("© 2019 Jane Doe. Reprinted under CCLI license #123456"),
            Some("© 2019 Jane Doe".to_string())
        );
        assert_eq!(
            copyright_from_rights("(c) 2018 A. B. Carter, One License #A-9"),
            Some("© 2018 A. B. Carter".to_string())
        );
        assert_eq!(
            copyright_from_rights("Copyright 2020 John Smith. All rights reserved."),
            Some("© 2020 John Smith".to_string())
        );
        assert_eq!(
            copyright_from_rights("Reprinted under CCLI license #123456"),
            None
        );
        assert_eq!(copyright_from_rights("© 1800 Too Old"), None);
    }

    #[test]
    fn ccli_permission_line_carries_title_authors_copyright_and_number() {
        let info = info_with(
            "© 2019 Jane Doe. Reprinted under CCLI license #123456",
            "Test Hymn",
        );
        let line = permission_line_for_music(&info).expect("a line prints");
        assert!(line.contains("Test Hymn"), "{line}");
        assert!(line.contains("Words: Jane Doe"), "{line}");
        assert!(line.contains("Music: John Smith"), "{line}");
        assert!(line.contains("© 2019 Jane Doe"), "{line}");
        assert!(
            line.contains("Reprinted under CCLI license #123456"),
            "{line}"
        );
    }

    #[test]
    fn one_license_permission_line_carries_copyright_and_number() {
        let info = info_with(
            "© 2020 John Smith. Used with permission under ONE LICENSE #A-987654",
            "Test Hymn",
        );
        let line = permission_line_for_music(&info).expect("a line prints");
        assert!(line.contains("© 2020 John Smith"), "{line}");
        assert!(
            line.contains("Used with permission under ONE LICENSE #A-987654"),
            "{line}"
        );
    }

    #[test]
    fn no_number_or_no_license_means_no_line() {
        let info = info_with("Reprinted under CCLI license", "Test Hymn");
        assert!(permission_line_for_music(&info).is_none());
        let info = info_with("Public domain in the file's own words", "Test Hymn");
        assert!(permission_line_for_music(&info).is_none());
    }

    fn empty_doc() -> Document {
        use ob_model::{default_styles, SheetSpec, Size};
        use std::collections::BTreeMap;
        Document {
            format_version: ob_model::FORMAT_VERSION,
            title: "t".to_string(),
            profile: ChurchProfile::default(),
            fields: BTreeMap::new(),
            styles: default_styles(),
            stories: Vec::new(),
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
        }
    }

    #[test]
    fn usage_report_surfaces_every_licensed_use_and_flags_the_gap() {
        let mut doc = empty_doc();
        let licensed = RightsRecord {
            source: "song.musicxml".to_string(),
            source_url: None,
            license: LicenseKind::LicensedReprint,
            title: "Licensed Song".to_string(),
            creator: "Jane Doe".to_string(),
            copyright_line: "© 2019 Jane Doe. Reprinted under CCLI license #123456".to_string(),
            credit_text: String::new(),
            licensor_ref: "123456".to_string(),
            asserted_by: String::new(),
            asserted_at: String::new(),
            children_present: None,
            child_web_permission: None,
        };
        doc.add_asset(
            b"fake music bytes",
            AssetKind::Music,
            "application/xml",
            licensed,
        );
        let unlicensed = RightsRecord {
            license: LicenseKind::PublicDomain,
            title: "Free Song".to_string(),
            ..RightsRecord::default()
        };
        doc.add_asset(
            b"other bytes",
            AssetKind::Music,
            "application/xml",
            unlicensed,
        );
        // A licensed hymn with no number recorded.
        doc.stories.push(Story {
            id: "order".to_string(),
            keep_through: None,
            paragraphs: vec![
                Paragraph::plain("body", "Sermon:\tPastor"),
                Paragraph::plain(
                    "order-line",
                    "Hymn of Praise\tELW 432 · O Come, All Ye Faithful",
                ),
            ],
        });
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
                day: 18,
            },
        );
        assert!(memory.note_license(
            Some("ELW"),
            "432",
            HymnLicense {
                kind: LicenseKind::LicensedReprint,
                licensor: "ONE LICENSE".to_string(),
                number: String::new(),
                copyright_line: "© 2020 John Smith".to_string(),
            },
        ));
        let report = usage_report(&doc, &memory);
        assert_eq!(report.len(), 2, "one music use, one hymn use");
        assert_eq!(report[0].title, "Licensed Song");
        assert_eq!(report[0].number.as_deref(), Some("123456"));
        assert!(!report[0].missing_number);
        assert!(report[0].source.contains("song.musicxml"));
        assert_eq!(report[1].title, "O Come, All Ye Faithful");
        assert!(report[1].number.is_none());
        assert!(report[1].missing_number);
        assert!(report[1].source.contains("ELW 432"));
        assert!(
            format_usage_entry(&report[1]).contains("license number not recorded"),
            "{}",
            format_usage_entry(&report[1])
        );
    }
}
