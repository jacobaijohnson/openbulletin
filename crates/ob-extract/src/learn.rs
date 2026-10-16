//! The learning door's ground rules (Jacob, 2026-10-15):
//! ONE pick, mixed formats together, and the format
//! minimums stated by the system in plain words BEFORE any
//! work starts.
//!
//! Learning needs at least three bulletins that count. A
//! Publisher, InDesign (IDML), Scribus, or OpenBulletin file
//! counts one-for-one: its structure is stated in the file,
//! so one file already shows real boxes and words. PDFs
//! count only in a group of three or more, because a PDF's
//! structure is rebuilt rather than given: one or two PDFs
//! alone cannot show what repeats week to week.
//!
//! This module holds the rule as pure functions so the app
//! can ask before it starts, the worker can ask again after
//! the unreadable files fall out, and both ask the same
//! question the same way.

use crate::{ExtractionDump, PageInfo, SourceKind, Span, SpanTrust, DUMP_SCHEMA_VERSION};
use ob_model::{Document, FrameKind};
use std::path::Path;

/// How many bulletins that count learning needs at minimum.
pub const MIN_BULLETINS: usize = 3;

/// The kinds of old bulletin the learning door takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LearnKind {
    Pdf,
    Publisher,
    Indesign,
    Scribus,
    OpenBulletin,
}

/// Which kind is this file? The name says first; when the
/// name does not say (or says nothing we know), the file's
/// own first bytes do. Reading the head of a stranger file
/// is all this does; parsing lives in the importers.
pub fn detect_kind(extension: &str, head: &[u8]) -> Option<LearnKind> {
    match extension.to_ascii_lowercase().as_str() {
        "pdf" => return Some(LearnKind::Pdf),
        "pub" => return Some(LearnKind::Publisher),
        "idml" => return Some(LearnKind::Indesign),
        "sla" => return Some(LearnKind::Scribus),
        "obd" => return Some(LearnKind::OpenBulletin),
        _ => {}
    }
    if head.starts_with(b"%PDF") {
        Some(LearnKind::Pdf)
    } else if head.starts_with(&[0xD0, 0xCF, 0x11, 0xE0]) {
        // The compound-file signature every .pub carries.
        Some(LearnKind::Publisher)
    } else if head.starts_with(b"PK") {
        // A zip: of the kinds we take, that is an IDML package.
        Some(LearnKind::Indesign)
    } else if String::from_utf8_lossy(head).contains("SCRIBUSUTF8NEW") {
        // A Scribus file is one XML document under that root.
        Some(LearnKind::Scribus)
    } else if head.iter().find(|b| !b.is_ascii_whitespace()) == Some(&b'{') {
        // A saved bulletin is JSON text; the worker verifies
        // by loading it, and a file that is not one is simply
        // skipped by name there.
        Some(LearnKind::OpenBulletin)
    } else {
        None
    }
}

/// Classify a picked file by name, then by its first bytes.
pub fn classify_path(path: &Path) -> Option<LearnKind> {
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().into_owned())
        .unwrap_or_default();
    if let Some(kind) = detect_kind(&ext, &[]) {
        return Some(kind);
    }
    let bytes = std::fs::read(path).unwrap_or_default();
    let head: Vec<u8> = bytes.into_iter().take(4096).collect();
    detect_kind(&ext, &head)
}

/// How many picked (or read) files of each kind there are.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct KindCounts {
    pub pdf: usize,
    pub publisher: usize,
    pub indesign: usize,
    pub scribus: usize,
    pub openbulletin: usize,
}

impl KindCounts {
    pub fn add(&mut self, kind: LearnKind) {
        match kind {
            LearnKind::Pdf => self.pdf += 1,
            LearnKind::Publisher => self.publisher += 1,
            LearnKind::Indesign => self.indesign += 1,
            LearnKind::Scribus => self.scribus += 1,
            LearnKind::OpenBulletin => self.openbulletin += 1,
        }
    }

    /// Files that count one-for-one (their structure is
    /// stated in the file, not rebuilt).
    pub fn structured(&self) -> usize {
        self.publisher + self.indesign + self.scribus + self.openbulletin
    }

    /// Files that count toward the minimum: structured files
    /// one-for-one, PDFs only as a group of three or more.
    pub fn effective(&self) -> usize {
        self.structured()
            + if self.pdf >= MIN_BULLETINS {
                self.pdf
            } else {
                0
            }
    }

    pub fn total(&self) -> usize {
        self.pdf + self.structured()
    }
}

/// Count a set of dumps by the kind of file each came from.
pub fn counts_of_dumps(dumps: &[ExtractionDump]) -> KindCounts {
    let mut counts = KindCounts::default();
    for d in dumps {
        counts.add(match d.source_kind {
            SourceKind::Pdf => LearnKind::Pdf,
            SourceKind::Pub => LearnKind::Publisher,
            SourceKind::Idml => LearnKind::Indesign,
            SourceKind::Sla => LearnKind::Scribus,
            SourceKind::Obd => LearnKind::OpenBulletin,
        });
    }
    counts
}

fn plural(n: usize, one: &str, many: &str) -> String {
    if n == 1 {
        format!("1 {one}")
    } else {
        format!("{n} {many}")
    }
}

/// The picked files in plain words: "2 PDFs and 1 Publisher
/// file", "3 old PDFs" style parts joined the way people
/// list things. `pdf_word` is "PDF" before the work and
/// "old PDF" in the summary of what was learned from.
fn describe(counts: &KindCounts, pdf_word: &str) -> String {
    let mut parts: Vec<String> = Vec::new();
    if counts.pdf > 0 {
        parts.push(plural(counts.pdf, pdf_word, &format!("{pdf_word}s")));
    }
    if counts.publisher > 0 {
        parts.push(plural(
            counts.publisher,
            "Publisher file",
            "Publisher files",
        ));
    }
    if counts.indesign > 0 {
        parts.push(plural(counts.indesign, "InDesign file", "InDesign files"));
    }
    if counts.scribus > 0 {
        parts.push(plural(counts.scribus, "Scribus file", "Scribus files"));
    }
    if counts.openbulletin > 0 {
        parts.push(plural(
            counts.openbulletin,
            "OpenBulletin file",
            "OpenBulletin files",
        ));
    }
    match parts.len() {
        0 => "nothing".to_string(),
        1 => parts[0].clone(),
        2 => format!("{} and {}", parts[0], parts[1]),
        _ => {
            let (last, rest) = parts.split_last().expect("parts");
            format!("{}, and {}", rest.join(", "), last)
        }
    }
}

/// Why a small PDF count does not count yet, when it applies.
fn pdf_group_note(counts: &KindCounts) -> Option<String> {
    match counts.pdf {
        1 => Some(
            "PDFs are learned from in groups of three or more, so that 1 does not count yet."
                .to_string(),
        ),
        2 => Some(
            "PDFs are learned from in groups of three or more, so those 2 do not count yet."
                .to_string(),
        ),
        _ => None,
    }
}

/// Exactly what to add so the pick counts, in plain words.
fn add_advice(counts: &KindCounts) -> String {
    if counts.pdf == 1 || counts.pdf == 2 {
        let more_pdf = MIN_BULLETINS - counts.pdf;
        let more_other = MIN_BULLETINS - counts.structured();
        format!(
            "Add {} to make the PDFs count, or {} of any other kind.",
            plural(more_pdf, "more PDF", "more PDFs"),
            plural(more_other, "more file", "more files"),
        )
    } else {
        let more = MIN_BULLETINS - counts.effective();
        format!(
            "Add {} from Publisher, InDesign, Scribus, or OpenBulletin, or add 3 PDFs.",
            plural(more, "more file", "more files"),
        )
    }
}

/// The pre-flight question the app asks the moment files are
/// picked, before any reading starts: does this pick count?
/// The error says exactly what was picked and what to add.
pub fn check_pick(counts: &KindCounts) -> Result<(), String> {
    if counts.effective() >= MIN_BULLETINS {
        return Ok(());
    }
    if counts.total() == 0 {
        return Err(
            "None of the picked files look like old bulletins we can learn from. \
             Pick at least three that count: Publisher, InDesign (IDML), Scribus, \
             or OpenBulletin files one-for-one, or PDFs in a group of three or more, \
             in any mix that adds up."
                .to_string(),
        );
    }
    let mut msg = format!("You picked {}. ", describe(counts, "PDF"));
    match pdf_group_note(counts) {
        Some(note) => {
            msg.push_str(&note);
            msg.push(' ');
        }
        None => msg.push_str("Learning needs at least three old bulletins that count. "),
    }
    msg.push_str(&add_advice(counts));
    Err(msg)
}

/// The same question asked again inside the worker, after
/// the files that could not be read fell out: the guidance
/// when what survived no longer counts. Names the losses.
pub fn shortfall_after_reading(counts: &KindCounts, skipped: &[String]) -> String {
    let mut msg = String::new();
    if !skipped.is_empty() {
        msg.push_str(&format!(
            "{} could not be read: {}. ",
            plural(skipped.len(), "picked file", "picked files"),
            skipped.join(", ")
        ));
    }
    msg.push_str(&format!(
        "What could be read is {}, and learning needs at least three old bulletins that count. ",
        describe(counts, "PDF")
    ));
    if let Some(note) = pdf_group_note(counts) {
        msg.push_str(&note);
        msg.push(' ');
    }
    msg.push_str(&add_advice(counts));
    msg
}

/// "Learned from 9 files: 6 old PDFs, 2 Publisher files,
/// and 1 Scribus file." The review summary's mix line.
pub fn mix_summary(counts: &KindCounts) -> String {
    format!(
        "Learned from {}: {}.",
        plural(counts.total(), "file", "files"),
        describe(counts, "old PDF")
    )
}

/// The mix line's second sentence, for when spoken-only
/// paragraphs were left out of learning (audit Batch 1):
/// they are private by the church's own marking, never
/// template content, and the leaving-out is said out loud
/// rather than done silently.
pub fn spoken_only_left_out_line(n: usize) -> String {
    if n == 1 {
        "1 spoken-only paragraph was left out; spoken-only words are private and never \
         become part of a template."
            .to_string()
    } else {
        format!(
            "{n} spoken-only paragraphs were left out; spoken-only words are private \
             and never become part of a template."
        )
    }
}

/// A saved bulletin (.obd) as an extraction dump, so a
/// church's own OpenBulletin files can teach the analyzer
/// alongside everyone else's. Spans carry the frames' real
/// places; the paragraphs of one story stack down its frame
/// at their own sizes, the way the words sit in the box.
///
/// Spoken-only paragraphs never become spans at all
/// (audit Batch 1): the dump has no field to carry their
/// flag, so before this rule a spoken-only prayer list
/// (the realistic case: the same shut-ins, week after
/// week) was learned as standing content and would print
/// every week. They are excluded here, and the count of
/// them rides back with the dump so the mix line can say
/// they were left out.
pub fn dump_from_document(doc: &Document, source_name: &str) -> (ExtractionDump, usize) {
    let mut pages: Vec<PageInfo> = Vec::new();
    let mut spans: Vec<Span> = Vec::new();
    let mut spoken_left_out = 0usize;
    for (pi, panel) in doc.panels.iter().enumerate() {
        let mut panel_spans: Vec<Span> = Vec::new();
        for frame in &panel.frames {
            if frame.kind != FrameKind::Text {
                continue;
            }
            let Some(story) = frame.story_id.as_ref().and_then(|id| doc.story(id)) else {
                continue;
            };
            let fx = frame.rect.x as f32 / 1000.0;
            let fw = frame.rect.w as f32 / 1000.0;
            let mut y = frame.rect.y as f32 / 1000.0;
            for para in &story.paragraphs {
                if para.spoken_only {
                    if !para.text().trim().is_empty() {
                        spoken_left_out += 1;
                    }
                    continue;
                }
                if para.text().trim().is_empty() {
                    continue;
                }
                let style = doc.style(&para.style);
                let size_pt = style
                    .map(|s| s.font_size as f32 / 1000.0)
                    .unwrap_or(11.0)
                    .max(1.0);
                panel_spans.push(Span {
                    page: pi as u32,
                    text: para.text(),
                    trust: SpanTrust::RawText,
                    x: Some(fx),
                    y: Some(y),
                    w: Some(fw),
                    h: Some(size_pt * 1.25),
                    font_size: Some(size_pt),
                    bold: Some(style.map(|s| s.bold).unwrap_or(false)),
                    font: style.map(|s| s.font_family.clone()),
                    // A document knows its own styles
                    // exactly: alignment, italic and color
                    // come from the paragraph's style.
                    align: style.map(|s| s.align),
                    italic: style.map(|s| s.italic).unwrap_or(false),
                    color: style
                        .map(|s| [s.color.r, s.color.g, s.color.b])
                        .unwrap_or([0, 0, 0]),
                    runs: Vec::new(),
                    thread: None,
                });
                y += size_pt * 1.25 + style.map(|s| s.space_after as f32 / 1000.0).unwrap_or(0.0);
            }
        }
        pages.push(PageInfo {
            index: pi as u32,
            width_pt: panel.size.w as f32 / 1000.0,
            height_pt: panel.size.h as f32 / 1000.0,
            picture_only: panel_spans.is_empty(),
        });
        spans.extend(panel_spans);
    }
    let dump = ExtractionDump {
        schema_version: DUMP_SCHEMA_VERSION,
        source_kind: SourceKind::Obd,
        source_name: source_name.to_string(),
        source_bytes: 0,
        extractor: "openbulletin (from a saved bulletin)".to_string(),
        pages,
        spans,
        images: Vec::new(),
    };
    (dump, spoken_left_out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ob_model::*;

    fn counts(
        pdf: usize,
        publisher: usize,
        indesign: usize,
        scribus: usize,
        obd: usize,
    ) -> KindCounts {
        KindCounts {
            pdf,
            publisher,
            indesign,
            scribus,
            openbulletin: obd,
        }
    }

    #[test]
    fn minimums_rule_counts() {
        // No PDFs: structured files count one-for-one.
        assert!(check_pick(&counts(0, 0, 0, 0, 0)).is_err());
        assert!(check_pick(&counts(0, 2, 0, 0, 0)).is_err());
        assert!(check_pick(&counts(0, 3, 0, 0, 0)).is_ok());
        assert!(check_pick(&counts(0, 1, 1, 1, 0)).is_ok());
        assert!(check_pick(&counts(0, 0, 0, 0, 2)).is_err());
        // PDFs alone: only a group of three or more counts.
        assert!(check_pick(&counts(1, 0, 0, 0, 0)).is_err());
        assert!(check_pick(&counts(2, 0, 0, 0, 0)).is_err());
        assert!(check_pick(&counts(3, 0, 0, 0, 0)).is_ok());
        assert!(check_pick(&counts(7, 0, 0, 0, 0)).is_ok());
        // Mixed: one or two PDFs contribute nothing.
        assert!(check_pick(&counts(1, 2, 0, 0, 0)).is_err());
        assert!(check_pick(&counts(2, 1, 0, 0, 0)).is_err());
        assert!(check_pick(&counts(1, 3, 0, 0, 0)).is_ok());
        assert!(check_pick(&counts(2, 3, 0, 0, 0)).is_ok());
        assert!(check_pick(&counts(3, 0, 0, 0, 0)).is_ok());
        // Effective counts, directly.
        assert_eq!(counts(2, 1, 0, 0, 0).effective(), 1);
        assert_eq!(counts(3, 1, 0, 0, 0).effective(), 4);
        assert_eq!(counts(0, 1, 1, 1, 1).effective(), 4);
    }

    #[test]
    fn refusal_says_what_was_picked_and_what_to_add() {
        // The brief's own example shape.
        let err = check_pick(&counts(2, 1, 0, 0, 0)).expect_err("refused");
        assert_eq!(
            err,
            "You picked 2 PDFs and 1 Publisher file. PDFs are learned from in \
             groups of three or more, so those 2 do not count yet. Add 1 more \
             PDF to make the PDFs count, or 2 more files of any other kind."
        );
        // One PDF, nothing else.
        let err = check_pick(&counts(1, 0, 0, 0, 0)).expect_err("refused");
        assert_eq!(
            err,
            "You picked 1 PDF. PDFs are learned from in groups of three or \
             more, so that 1 does not count yet. Add 2 more PDFs to make the \
             PDFs count, or 3 more files of any other kind."
        );
        // No PDFs at all: the plain shortfall wording.
        let err = check_pick(&counts(0, 1, 0, 1, 0)).expect_err("refused");
        assert_eq!(
            err,
            "You picked 1 Publisher file and 1 Scribus file. Learning needs \
             at least three old bulletins that count. Add 1 more file from \
             Publisher, InDesign, Scribus, or OpenBulletin, or add 3 PDFs."
        );
        // Nothing recognizable.
        let err = check_pick(&counts(0, 0, 0, 0, 0)).expect_err("refused");
        assert!(err.starts_with("None of the picked files"), "{err}");
    }

    #[test]
    fn shortfall_after_reading_names_the_losses() {
        let msg = shortfall_after_reading(&counts(1, 0, 0, 0, 0), &["old.pub".to_string()]);
        assert!(
            msg.contains("1 picked file could not be read: old.pub."),
            "{msg}"
        );
        assert!(msg.contains("that 1 does not count yet"), "{msg}");
        let msg = shortfall_after_reading(&counts(0, 2, 0, 0, 0), &[]);
        assert!(
            msg.contains("What could be read is 2 Publisher files"),
            "{msg}"
        );
        assert!(msg.contains("Add 1 more file"), "{msg}");
    }

    #[test]
    fn mix_summary_lists_the_mix() {
        assert_eq!(
            mix_summary(&counts(6, 2, 0, 1, 0)),
            "Learned from 9 files: 6 old PDFs, 2 Publisher files, and 1 Scribus file."
        );
        assert_eq!(
            mix_summary(&counts(0, 0, 3, 0, 0)),
            "Learned from 3 files: 3 InDesign files."
        );
    }

    #[test]
    fn detect_kind_by_name_then_by_bytes() {
        assert_eq!(detect_kind("pdf", &[]), Some(LearnKind::Pdf));
        assert_eq!(detect_kind("PUB", &[]), Some(LearnKind::Publisher));
        assert_eq!(detect_kind("idml", &[]), Some(LearnKind::Indesign));
        assert_eq!(detect_kind("sla", &[]), Some(LearnKind::Scribus));
        assert_eq!(detect_kind("obd", &[]), Some(LearnKind::OpenBulletin));
        // No name worth trusting: the bytes decide.
        assert_eq!(detect_kind("", b"%PDF-1.7 ..."), Some(LearnKind::Pdf));
        assert_eq!(
            detect_kind("bin", &[0xD0, 0xCF, 0x11, 0xE0, 1, 2]),
            Some(LearnKind::Publisher)
        );
        assert_eq!(
            detect_kind("bin", b"PK\x03\x04rest"),
            Some(LearnKind::Indesign)
        );
        assert_eq!(
            detect_kind(
                "",
                b"<?xml version=\"1.0\"?><SCRIBUSUTF8NEW Version=\"1.6\">"
            ),
            Some(LearnKind::Scribus)
        );
        assert_eq!(
            detect_kind("txt", b"  \n {\"format_version\": 0}"),
            Some(LearnKind::OpenBulletin)
        );
        assert_eq!(detect_kind("docx", b"hello"), None);
        assert_eq!(detect_kind("", b""), None);
    }

    fn small_doc() -> Document {
        Document {
            format_version: FORMAT_VERSION,
            title: "Grace Lutheran bulletin".into(),
            profile: ChurchProfile::default(),
            fields: Default::default(),
            styles: vec![ParagraphStyle {
                id: "body".into(),
                name: "Body".into(),
                font_size: 11_000,
                bold: false,
                italic: false,
                align: Align::Left,
                space_after: 4_000,
                color: Rgb::BLACK,
                font_family: "Liberation Serif".into(),
                font_sha256: None,
            }],
            stories: vec![Story {
                keep_through: None,
                id: "story-1".into(),
                paragraphs: vec![
                    Paragraph::new("body", "Welcome to Grace Lutheran Church"),
                    Paragraph::new("body", "The Lord is my shepherd, I shall not want"),
                ],
            }],
            no_announcements: false,
            panels: vec![Panel {
                id: "page-1".into(),
                name: "Page 1".into(),
                size: Size::new(612_000, 792_000),
                frames: vec![Frame {
                    id: "frame-1".into(),
                    name: "Words".into(),
                    kind: FrameKind::Text,
                    rect: Rect::new(72_000, 72_000, 468_000, 600_000),
                    story_id: Some("story-1".into()),
                    role: ContentRole::Weekly,
                    picture: None,
                    music: None,
                }],
                guides: Vec::new(),
                background_image: None,
            }],
            sheet: SheetSpec {
                preset: "letter-portrait".into(),
                size: Size::new(612_000, 792_000),
            },
            assets: Vec::new(),
            asset_bytes: Default::default(),
            occasion_time: None,
        }
    }

    #[test]
    fn document_becomes_a_dump_with_real_geometry() {
        let (dump, spoken_left_out) = dump_from_document(&small_doc(), "week.obd");
        assert_eq!(spoken_left_out, 0, "nothing private in this document");
        assert_eq!(dump.source_kind, SourceKind::Obd);
        assert_eq!(dump.pages.len(), 1);
        assert!((dump.pages[0].width_pt - 612.0).abs() < 0.01);
        assert!(!dump.pages[0].picture_only);
        assert_eq!(dump.spans.len(), 2, "one span per paragraph");
        let first = &dump.spans[0];
        assert_eq!(first.text, "Welcome to Grace Lutheran Church");
        assert_eq!(first.x, Some(72.0));
        assert_eq!(first.w, Some(468.0));
        assert_eq!(first.font_size, Some(11.0));
        assert_eq!(first.font.as_deref(), Some("Liberation Serif"));
        // The second paragraph stacks below the first.
        assert!(dump.spans[1].y.unwrap() > first.y.unwrap());
        // The shared JSON contract still holds.
        let back = ExtractionDump::from_json(&dump.to_json().expect("json")).expect("parse");
        assert_eq!(back.spans.len(), 2);
    }

    #[test]
    fn spoken_only_paragraphs_never_enter_learning() {
        use crate::analyzer::analyze;
        // Three saved bulletins, each carrying the same
        // standing words and the same spoken-only prayer
        // (the shut-ins list: the realistic spoken-only
        // content, repeated week after week). Before the
        // fix, the prayer was learned as standing content
        // and would have printed every week (audit Batch 1).
        let mut total_left_out = 0usize;
        let mut dumps = Vec::new();
        for name in ["a.obd", "b.obd", "c.obd"] {
            let mut doc = small_doc();
            let mut prayer = Paragraph::new("body", "For Wilhelmina Quietname, spoken at the rail");
            prayer.spoken_only = true;
            doc.stories[0].paragraphs.push(prayer);
            let (dump, left_out) = dump_from_document(&doc, name);
            total_left_out += left_out;
            dumps.push(dump);
        }
        assert_eq!(total_left_out, 3, "one prayer per bulletin, all left out");
        // No dump carries the words at all.
        assert!(dumps
            .iter()
            .all(|d| d.spans.iter().all(|s| !s.text.contains("Wilhelmina"))));
        let out = analyze(&dumps, ChurchProfile::default()).expect("analyze");
        let template_text: Vec<String> = out
            .doc
            .stories
            .iter()
            .flat_map(|s| s.paragraphs.iter().map(|p| p.text()))
            .collect();
        let template_text = template_text.join(" ");
        assert!(
            !template_text.contains("Wilhelmina"),
            "the learned template holds no spoken-only words: {template_text}"
        );
        assert!(
            template_text.contains("shepherd"),
            "the standing words still learned: {template_text}"
        );
        // And the sentence that reports the leaving-out is
        // the plain-words one the mix line carries.
        assert_eq!(
            spoken_only_left_out_line(3),
            "3 spoken-only paragraphs were left out; spoken-only words are private \
             and never become part of a template."
        );
        assert!(spoken_only_left_out_line(1).starts_with("1 spoken-only paragraph was"));
    }

    #[test]
    fn analyzer_learns_from_mixed_source_kinds() {
        use crate::analyzer::analyze;
        fn dump(kind: SourceKind, name: &str, weekly: &str) -> ExtractionDump {
            let standing =
                "the lord is my shepherd i shall not want he makes me lie down in green pastures";
            let mk = |text: &str, y: f32| Span {
                page: 0,
                text: text.into(),
                trust: SpanTrust::RawText,
                x: Some(40.0),
                y: Some(y),
                w: Some(300.0),
                h: Some(14.0),
                font_size: Some(11.0),
                bold: Some(false),
                font: Some("Calibri".into()),
                align: None,
                italic: false,
                color: [0, 0, 0],
                runs: Vec::new(),
                thread: None,
            };
            ExtractionDump {
                schema_version: 1,
                source_kind: kind,
                source_name: name.into(),
                source_bytes: 10,
                extractor: "test".into(),
                pages: vec![PageInfo {
                    index: 0,
                    width_pt: 612.0,
                    height_pt: 792.0,
                    picture_only: false,
                }],
                spans: vec![mk(standing, 40.0), mk(weekly, 120.0)],
                images: Vec::new(),
            }
        }
        let dumps = vec![
            dump(
                SourceKind::Pdf,
                "a.pdf",
                "potluck this friday at six bring a dish",
            ),
            dump(
                SourceKind::Sla,
                "b.sla",
                "choir rehearsal moved to thursday evening",
            ),
            dump(
                SourceKind::Obd,
                "c.obd",
                "youth group car wash on saturday morning",
            ),
        ];
        let out = analyze(&dumps, ChurchProfile::default()).expect("analyze mixed");
        assert_eq!(out.family, 3);
        assert_eq!(out.corpus, 3);
        let mix = mix_summary(&counts_of_dumps(&dumps));
        assert_eq!(
            mix,
            "Learned from 3 files: 1 old PDF, 1 Scribus file, and 1 OpenBulletin file."
        );
    }
}
