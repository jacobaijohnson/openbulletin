//! The "Keep in the library" step at import review:
//! the pure pieces behind the review window's tick.
//!
//! The window itself (windows.rs `import_review_window`)
//! cannot be driven headless, so everything it does
//! that can be pure lives here, tested: resolving a
//! [`Keepable`] to a [`KeepInput`] ([`resolve_keep_input`]),
//! the per-story keepables the Markdown door uses
//! ([`keepables_for_story`]), and the tick / Ask
//! execution ([`execute_keep`], [`resolve_ask`]), which
//! load the store and the index from paths the caller
//! supplies, so the tests run them against temp dirs.
//! The window is a thin wrapper: it renders the rows,
//! collects clicks as deferred actions, and calls these
//! functions after `show()` returns.

use crate::check::today_simple;
use crate::ImportReview;
use ob_fingerprint::matching::{FingerprintIndex, Match};
use ob_model::draft::{first_line, KeepKind, Keepable};
use ob_model::{
    AssetKind, ContentRole, Document, Frame, FrameKind, LicenseKind, MusicFrame, Panel, Paragraph,
    PictureFit, PicturePlacement, Rect, RightsRecord, Story,
};
use ob_music::draft::{license_from_rights, CREDIT_H, GAP, HEADING_H, MARGIN};
use ob_music::render::{RenderOutcome, TARGET_STAFF_MM};
use ob_snippets::{
    decide_keep, dismiss_proposals, store_snippet, KeepDecision, KeepInput, SnippetRecord,
    SnippetStore,
};
use std::path::Path;

/// One Text keepable per paragraph of the named story
/// in the document, in paragraph order. An unknown
/// story yields no keepables. The Markdown door uses
/// this for its "notes" story; the other builders attach
/// keepables where the content's identity is known.
pub(crate) fn keepables_for_story(doc: &Document, story_id: &str) -> Vec<Keepable> {
    let Some(story) = doc.stories.iter().find(|s| s.id == story_id) else {
        return Vec::new();
    };
    story
        .paragraphs
        .iter()
        .enumerate()
        .map(|(i, para)| Keepable {
            kind: KeepKind::Text {
                story_id: story_id.to_string(),
                paragraph: i,
            },
            default_name: first_line(&para.text()),
        })
        .collect()
}

/// Resolve one keepable to the [`KeepInput`] the keep
/// engine stores. Pure: no store, no index, no egui.
/// Every failure is a loud plain-words error and stores
/// nothing: missing bytes, a missing story or paragraph,
/// a missing workdir file, an asset that is neither a
/// picture nor a song.
pub(crate) fn resolve_keep_input(
    review: &ImportReview,
    keepable: &Keepable,
    name: &str,
) -> Result<KeepInput, String> {
    // Batch C, C-2: an emptied name field is not a name,
    // so the keepable's default applies (brief Q2: named
    // by the admin at keep time, defaulting to its first
    // line). Whitespace-only counts as empty.
    let name = if name.trim().is_empty() {
        keepable.default_name.clone()
    } else {
        name.to_string()
    };
    let today = today_simple();
    let kept_at = format!("{:04}-{:02}-{:02}", today.year, today.month, today.day);
    let source = review.source_name.clone();
    match &keepable.kind {
        KeepKind::Picture { sha } | KeepKind::Music { sha } => {
            let bytes = review
                .doc
                .asset_data(sha)
                .ok_or_else(|| {
                    "The picture or song is missing from this draft, so there is nothing to keep."
                        .to_string()
                })?
                .to_vec();
            let asset = review
                .doc
                .assets
                .iter()
                .find(|a| a.sha256 == *sha)
                .ok_or_else(|| {
                    "The picture or song is missing from this draft, so there is nothing to keep."
                        .to_string()
                })?;
            let kind = match asset.kind {
                AssetKind::Picture => ob_snippets::SnippetKind::Picture,
                AssetKind::Music => ob_snippets::SnippetKind::Music,
                _ => {
                    return Err(
                        "That asset is neither a picture nor a song, so it cannot be kept."
                            .to_string(),
                    );
                }
            };
            Ok(KeepInput {
                content: bytes,
                kind,
                name,
                source,
                kept_at,
                asserted_by: String::new(),
                media_type: asset.media_type.clone(),
                rights: asset.rights.clone(),
                spoken_only: false,
                web_hidden: false,
            })
        }
        KeepKind::Text {
            story_id,
            paragraph,
        } => {
            let story = review
                .doc
                .stories
                .iter()
                .find(|s| s.id == *story_id)
                .ok_or_else(|| {
                    format!("The story {story_id} is missing from this draft, so there is nothing to keep.")
                })?;
            let para = story.paragraphs.get(*paragraph).ok_or_else(|| {
                format!(
                    "Paragraph {} is missing from the story {story_id}, so there is nothing to keep.",
                    paragraph + 1
                )
            })?;
            Ok(KeepInput {
                content: para.text().into_bytes(),
                kind: ob_snippets::SnippetKind::Text,
                name,
                source: source.clone(),
                kept_at,
                asserted_by: String::new(),
                media_type: "text/plain".to_string(),
                rights: RightsRecord {
                    source,
                    ..Default::default()
                },
                spoken_only: para.spoken_only,
                web_hidden: para.web_hidden,
            })
        }
        KeepKind::PagePicture { page } => {
            let workdir = review
                .workdir
                .as_ref()
                .ok_or_else(|| "This import has no page pictures to keep.".to_string())?;
            let path = workdir
                .join("assets")
                .join(format!("page-{:02}.png", page + 1));
            let bytes = std::fs::read(&path)
                .map_err(|e| format!("Could not read the page picture {}: {e}", path.display()))?;
            Ok(KeepInput {
                content: bytes,
                kind: ob_snippets::SnippetKind::Picture,
                name,
                source: source.clone(),
                kept_at,
                asserted_by: String::new(),
                media_type: "image/png".to_string(),
                rights: RightsRecord {
                    source,
                    ..Default::default()
                },
                spoken_only: false,
                web_hidden: false,
            })
        }
    }
}

/// What a tick does, once decided.
#[derive(Debug)]
pub(crate) enum KeepOutcome {
    /// Stored now.
    Kept,
    /// Already in the store: stored nothing.
    AlreadyKept,
    /// Near-duplicates need the admin's eye; nothing
    /// stored yet. Boxed: the Ask carries the record,
    /// the proposals, and the bytes, far larger than
    /// the other variants.
    Ask(Box<PendingKeepAsk>),
    /// The admin answered "Don't keep": stored nothing.
    NotKept,
    /// A loud reason, shown on the row; stored nothing.
    Failed(String),
}

/// A keep whose dedupe decision asked the admin: the
/// record that would be stored, the proposals with
/// their entry SHAs, and the bytes to store on
/// resolution. The row stays pending until the admin
/// answers.
#[derive(Debug)]
pub(crate) struct PendingKeepAsk {
    pub item: usize,
    pub keep: usize,
    pub record: SnippetRecord,
    pub proposals: Vec<(Match, String)>,
    pub entry_shas: Vec<String>,
    pub content: Vec<u8>,
}

/// The admin's answer to an Ask.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AskChoice {
    KeepAnyway,
    NotDuplicates,
    DontKeep,
}

/// A click the review window collected during render,
/// executed after `show()` returns.
#[derive(Debug)]
pub(crate) enum KeepAction {
    Tick {
        item: usize,
        keep: usize,
        name: String,
    },
    AskResolve {
        item: usize,
        keep: usize,
        choice: AskChoice,
    },
}

/// Execute one tick: resolve the keepable, decide
/// against the store and the index, and store at once
/// (or hold the Ask for the admin). The store and the
/// index are loaded fresh from the given paths.
#[allow(clippy::too_many_arguments)]
pub(crate) fn execute_keep(
    review: &ImportReview,
    item: usize,
    keep: usize,
    keepable: &Keepable,
    name: &str,
    store_path: &Path,
    bytes_dir: &Path,
    index_path: &Path,
) -> KeepOutcome {
    let input = match resolve_keep_input(review, keepable, name) {
        Ok(input) => input,
        Err(reason) => return KeepOutcome::Failed(reason),
    };
    let mut store = match SnippetStore::load(store_path) {
        Ok(store) => store,
        Err(reason) => return KeepOutcome::Failed(reason),
    };
    let mut index = match FingerprintIndex::load(index_path) {
        Ok(index) => index,
        Err(reason) => return KeepOutcome::Failed(reason),
    };
    let decision = match decide_keep(&input, &store, &index, bytes_dir) {
        Ok(decision) => decision,
        Err(reason) => return KeepOutcome::Failed(reason),
    };
    match decision {
        KeepDecision::AlreadyKept { .. } => KeepOutcome::AlreadyKept,
        KeepDecision::ReadyToStore { record } => {
            match store_snippet(
                &mut store,
                &mut index,
                record,
                &input.content,
                bytes_dir,
                store_path,
                index_path,
            ) {
                Ok(_) => KeepOutcome::Kept,
                Err(reason) => KeepOutcome::Failed(reason),
            }
        }
        KeepDecision::Ask {
            record,
            proposals,
            entry_shas,
        } => KeepOutcome::Ask(Box::new(PendingKeepAsk {
            item,
            keep,
            record,
            proposals,
            entry_shas,
            content: input.content,
        })),
    }
}

/// Resolve a pending Ask the way the admin answered.
/// "Keep anyway" stores; "Not duplicates" records the
/// dismissals first, then stores, so the pair is never
/// re-proposed; "Don't keep" stores nothing and writes
/// nothing.
pub(crate) fn resolve_ask(
    ask: &PendingKeepAsk,
    choice: AskChoice,
    store_path: &Path,
    bytes_dir: &Path,
    index_path: &Path,
) -> KeepOutcome {
    if choice == AskChoice::DontKeep {
        return KeepOutcome::NotKept;
    }
    let mut store = match SnippetStore::load(store_path) {
        Ok(store) => store,
        Err(reason) => return KeepOutcome::Failed(reason),
    };
    let mut index = match FingerprintIndex::load(index_path) {
        Ok(index) => index,
        Err(reason) => return KeepOutcome::Failed(reason),
    };
    if choice == AskChoice::NotDuplicates {
        dismiss_proposals(&mut index, &ask.record.id, &ask.entry_shas);
    }
    match store_snippet(
        &mut store,
        &mut index,
        ask.record.clone(),
        &ask.content,
        bytes_dir,
        store_path,
        index_path,
    ) {
        Ok(_) => KeepOutcome::Kept,
        Err(reason) => KeepOutcome::Failed(reason),
    }
}

/// The Cancel note, Batch C C-1: the wording depends on
/// whether a ticked keep survived. The caller reads the
/// kept state BEFORE `reset_keep_state()` clears it.
pub(crate) fn cancel_note(kept_items: bool) -> &'static str {
    if kept_items {
        "Import cancelled. Items you kept stay in the library."
    } else {
        "Import cancelled; nothing changed."
    }
}

/// The credit line the picker shows under a snippet's
/// name: the record's own credit words when it has
/// them, else title and creator, else the file it was
/// kept from.
pub(crate) fn snippet_credit_line(record: &SnippetRecord) -> String {
    let rights = &record.asset.rights;
    if !rights.credit_text.is_empty() {
        rights.credit_text.clone()
    } else if !rights.title.is_empty() && !rights.creator.is_empty() {
        format!("{} — {}", rights.title, rights.creator)
    } else if !rights.title.is_empty() {
        rights.title.clone()
    } else if !rights.creator.is_empty() {
        rights.creator.clone()
    } else {
        format!("From {}", record.source)
    }
}

/// The picker's search: a case-insensitive substring
/// match on the snippet name; an empty query lists
/// the whole library.
pub(crate) fn matching_snippets<'a>(
    store: &'a SnippetStore,
    query: &str,
) -> Vec<&'a SnippetRecord> {
    let needle = query.trim().to_lowercase();
    store
        .snippets
        .iter()
        .filter(|record| needle.is_empty() || record.name.to_lowercase().contains(&needle))
        .collect()
}

/// Place a kept picture into a box, the clips way:
/// the bytes go into the document's asset store
/// (`add_asset` dedupes by SHA-256 and merges rights),
/// then the box takes the picture the way
/// `set_frame_picture` does it (kind Picture, the
/// story let go). A music box refuses, in the same
/// words `place_stored_picture` uses. Returns the
/// asset SHA-256. The caller pushes undo first.
pub(crate) fn place_snippet_picture_in_doc(
    doc: &mut Document,
    panel_idx: usize,
    frame_idx: usize,
    record: &SnippetRecord,
    bytes: &[u8],
) -> Result<String, String> {
    let frame = doc
        .panels
        .get(panel_idx)
        .and_then(|panel| panel.frames.get(frame_idx))
        .ok_or_else(|| "That box is not on this page any more.".to_string())?;
    if frame.music.is_some() {
        return Err("That box holds music; a picture cannot go in it.".to_string());
    }
    let sha = doc.add_asset(
        bytes,
        AssetKind::Picture,
        &record.asset.media_type,
        record.asset.rights.clone(),
    );
    let frame = &mut doc.panels[panel_idx].frames[frame_idx];
    frame.kind = FrameKind::Picture;
    frame.picture = Some(PicturePlacement::new(sha.clone()));
    frame.story_id = None;
    Ok(sha)
}

/// Place kept announcement words as a new text box on a
/// panel, mirroring `add_text_box` (windows.rs:1726)
/// exactly: a fresh story id in the `box-N` family, one
/// `"body"` paragraph, a centred 216_000 x 108_000 box,
/// role Weekly. The snippet's `spoken_only` /
/// `web_hidden` flags ride on the paragraph, so reuse
/// reproduces the same export behaviour. Returns the
/// story id and the new frame's index. The caller
/// pushes undo first and selects the frame.
pub(crate) fn place_snippet_text_in_doc(
    doc: &mut Document,
    panel_idx: usize,
    record: &SnippetRecord,
    bytes: &[u8],
) -> Result<(String, usize), String> {
    let words = String::from_utf8(bytes.to_vec())
        .map_err(|_| "Those words are not text this program can read.".to_string())?;
    if doc.panels.get(panel_idx).is_none() {
        return Err("That page is not in this bulletin any more.".to_string());
    }
    let mut n = 1usize;
    let sid = loop {
        let candidate = format!("box-{n}");
        if doc.story(&candidate).is_none() {
            break candidate;
        }
        n += 1;
    };
    let mut paragraph = Paragraph::new("body", &words);
    paragraph.spoken_only = record.spoken_only;
    paragraph.web_hidden = record.web_hidden;
    doc.stories.push(Story {
        id: sid.clone(),
        paragraphs: vec![paragraph],
        keep_through: None,
    });
    let panel = &mut doc.panels[panel_idx];
    let w = 216_000.min(panel.size.w);
    let h = 108_000.min(panel.size.h);
    panel.frames.push(Frame {
        id: format!("frame-{sid}"),
        name: "Text box".into(),
        kind: FrameKind::Text,
        rect: Rect {
            x: (panel.size.w - w) / 2,
            y: (panel.size.h - h) / 2,
            w,
            h,
        },
        story_id: Some(sid.clone()),
        role: ContentRole::Weekly,
        picture: None,
        music: None,
    });
    Ok((sid, panel.frames.len() - 1))
}

/// What placing a kept song laid down.
#[derive(Debug)]
pub(crate) struct SongPlacement {
    /// Engraved systems placed (1 for the empty-box arm).
    pub systems: usize,
    /// False when no engraver was on this computer and
    /// the empty music box stands instead.
    pub rendered: bool,
    /// A licensed song whose file states no license
    /// number: no permission line prints, and the
    /// window says so plainly.
    pub permission_missing_number: bool,
}

/// A fresh id in a family: the base when it is free,
/// else the base with -2, -3, ... Story ids and frame
/// ids live in separate namespaces, so each family is
/// scanned on its own.
fn fresh_story_id(doc: &Document, base: &str) -> String {
    if doc.story(base).is_none() {
        return base.to_string();
    }
    let mut n = 2usize;
    loop {
        let candidate = format!("{base}-{n}");
        if doc.story(&candidate).is_none() {
            return candidate;
        }
        n += 1;
    }
}

fn fresh_frame_id(doc: &Document, base: &str) -> String {
    let taken = |id: &str| {
        doc.panels
            .iter()
            .flat_map(|panel| &panel.frames)
            .any(|frame| frame.id == id)
    };
    if !taken(base) {
        return base.to_string();
    }
    let mut n = 2usize;
    loop {
        let candidate = format!("{base}-{n}");
        if !taken(&candidate) {
            return candidate;
        }
        n += 1;
    }
}

/// The draft's overflow rule (draft.rs:337-342): a new
/// panel, the target panel's own size, when the next
/// box would run past the bottom margin. Returns the
/// panel to lay on and the cursor on it.
fn overflow_panel(doc: &mut Document, here: usize, cursor: i64, h: i64) -> (usize, i64) {
    let panel_h = doc.panels[here].size.h;
    if cursor + h > panel_h - MARGIN && cursor > MARGIN {
        let size = doc.panels[here].size;
        let number = doc.panels.len();
        doc.panels.push(Panel {
            id: format!("page-{}", number + 1),
            name: format!("Page {}", number + 1),
            size,
            frames: Vec::new(),
            guides: Vec::new(),
            background_image: None,
        });
        (doc.panels.len() - 1, MARGIN)
    } else {
        (here, cursor)
    }
}

/// Place a kept song into a bulletin, built from the
/// pieces the music import uses — there is no
/// post-document music placement path in production,
/// so this mirrors `build_music_draft` (ob-music
/// draft.rs) instead of calling one. The MusicXML bytes
/// go into the document's asset store (`add_asset`,
/// kind Music, the stored rights); the file is parsed
/// (`load_music`) and the heading + caption stories ride
/// EXACTLY as the draft adds them — title / subtitle /
/// composer / poet lines as `"heading"` / `"subheading"` /
/// `"body"`, the credit line as `"caption"`, and the
/// permission line as its own `"caption"` story under
/// the credit, with the draft's unadvanced cursor
/// between credit and permission. Each engraved system
/// becomes a per-system SVG asset plus a full-width
/// Picture frame carrying `MusicFrame { system,
/// staff_mm, source }`, the height from the engraving's
/// own proportions. With no render (no engraver on this
/// computer) the draft's empty-box arm runs instead: one
/// empty music box, `picture: None`, staff
/// `TARGET_STAFF_MM`, height 42% of the full text width.
///
/// Selected-frame rule: an empty picture frame takes
/// system 0 (further systems as new frames after it); a
/// frame already holding music is replaced the same way;
/// any other filled box refuses in plain words; with no
/// frame selected, new frames are created on the current
/// panel. The song block lays out in draft order from
/// the selected frame's position, or from the top of the
/// panel when no frame is selected, flowing to new
/// panels on overflow. The caller pushes undo first.
pub(crate) fn place_snippet_song_in_doc(
    doc: &mut Document,
    panel_idx: usize,
    frame_idx: Option<usize>,
    record: &SnippetRecord,
    bytes: &[u8],
    render: Option<&RenderOutcome>,
) -> Result<SongPlacement, String> {
    if doc.panels.get(panel_idx).is_none() {
        return Err("That page is not in this bulletin any more.".to_string());
    }
    let anchor = match frame_idx {
        None => None,
        Some(fi) => {
            let frame = doc
                .panels
                .get(panel_idx)
                .and_then(|panel| panel.frames.get(fi))
                .ok_or_else(|| "That box is not on this page any more.".to_string())?;
            if frame.music.is_some()
                || (frame.kind == FrameKind::Picture && frame.picture.is_none())
            {
                Some(fi)
            } else {
                return Err(
                    "That box already holds something. Pick an empty picture box or a music \
                     box, or pick no box and the song gets new boxes."
                        .to_string(),
                );
            }
        }
    };
    let (_xml, info) = ob_music::load_music(bytes)
        .map_err(|why| format!("That song cannot be read as music: {why}"))?;
    let rights = record.asset.rights.clone();
    let source_hash = doc.add_asset(
        bytes,
        AssetKind::Music,
        &record.asset.media_type,
        rights.clone(),
    );
    let panel_w = doc.panels[panel_idx].size.w;
    let width = panel_w - 2 * MARGIN;
    let mut here = panel_idx;
    let mut cursor = anchor
        .map(|fi| doc.panels[panel_idx].frames[fi].rect.y)
        .unwrap_or(MARGIN);

    // The heading, exactly as the draft adds it.
    let mut paras: Vec<(&str, String)> = Vec::new();
    if let Some(title) = &info.title {
        paras.push(("heading", title.clone()));
    }
    if let Some(sub) = &info.subtitle {
        paras.push(("subheading", sub.clone()));
    }
    if let Some(composer) = info.composer() {
        paras.push(("body", composer.to_string()));
    }
    if let Some(poet) = info.creator("lyricist") {
        paras.push(("body", poet.to_string()));
    }
    if paras.is_empty() {
        paras.push(("heading", record.source.clone()));
    }
    let heading_id = fresh_story_id(doc, "music-heading");
    doc.stories.push(Story {
        id: heading_id.clone(),
        paragraphs: paras
            .iter()
            .map(|(style, text)| Paragraph::plain(style, text))
            .collect(),
        keep_through: None,
    });
    let (next_here, next_cursor) = overflow_panel(doc, here, cursor, HEADING_H);
    here = next_here;
    cursor = next_cursor;
    let heading_frame_id = fresh_frame_id(doc, "music-heading");
    doc.panels[here].frames.push(Frame {
        id: heading_frame_id,
        name: "Hymn title and composer".into(),
        kind: FrameKind::Text,
        rect: Rect::new(MARGIN, cursor, width, HEADING_H),
        story_id: Some(heading_id),
        role: ContentRole::Standing,
        picture: None,
        music: None,
    });
    cursor += HEADING_H + GAP;

    // The systems: one frame per engraved line, or the
    // empty box when no engraver drew them.
    let systems: usize;
    match render {
        Some(render) => {
            systems = render.systems.len();
            for (i, system) in render.systems.iter().enumerate() {
                let h = ((width as f64 * system.height_units / system.width_units.max(1.0)).round()
                    as i64)
                    .max(1);
                let (next_here, next_cursor) = overflow_panel(doc, here, cursor, h);
                here = next_here;
                cursor = next_cursor;
                let hash = doc.add_asset(
                    system.svg.as_bytes(),
                    AssetKind::Picture,
                    "image/svg+xml",
                    rights.clone(),
                );
                let picture = Some(PicturePlacement {
                    asset: hash,
                    fit: PictureFit::ShowWhole,
                    crop_x: 0.5,
                    crop_y: 0.5,
                });
                let music = Some(MusicFrame {
                    system: i as u32,
                    staff_mm: system.staff_mm,
                    source: source_hash.clone(),
                });
                if i == 0 {
                    if let Some(fi) = anchor {
                        let frame = &mut doc.panels[panel_idx].frames[fi];
                        frame.kind = FrameKind::Picture;
                        frame.picture = picture;
                        frame.music = music;
                        frame.story_id = None;
                        frame.rect = Rect::new(MARGIN, cursor, width, h);
                        frame.name = "Music line 1".into();
                    } else {
                        let frame_id = fresh_frame_id(doc, "music-line-1");
                        doc.panels[here].frames.push(Frame {
                            id: frame_id,
                            name: "Music line 1".into(),
                            kind: FrameKind::Picture,
                            rect: Rect::new(MARGIN, cursor, width, h),
                            story_id: None,
                            role: ContentRole::Standing,
                            picture,
                            music,
                        });
                    }
                } else {
                    let base = format!("music-line-{}", i + 1);
                    let frame_id = fresh_frame_id(doc, &base);
                    doc.panels[here].frames.push(Frame {
                        id: frame_id,
                        name: format!("Music line {}", i + 1),
                        kind: FrameKind::Picture,
                        rect: Rect::new(MARGIN, cursor, width, h),
                        story_id: None,
                        role: ContentRole::Standing,
                        picture,
                        music,
                    });
                }
                cursor += h + GAP;
            }
        }
        None => {
            systems = 1;
            let h = (width as f64 * 0.42).round() as i64;
            let (next_here, next_cursor) = overflow_panel(doc, here, cursor, h);
            here = next_here;
            cursor = next_cursor;
            let music = Some(MusicFrame {
                system: 0,
                staff_mm: TARGET_STAFF_MM,
                source: source_hash.clone(),
            });
            if let Some(fi) = anchor {
                let frame = &mut doc.panels[panel_idx].frames[fi];
                frame.kind = FrameKind::Picture;
                frame.picture = None;
                frame.music = music;
                frame.story_id = None;
                frame.rect = Rect::new(MARGIN, cursor, width, h);
                frame.name = "Music line 1".into();
            } else {
                let frame_id = fresh_frame_id(doc, "music-line-1");
                doc.panels[here].frames.push(Frame {
                    id: frame_id,
                    name: "Music line 1".into(),
                    kind: FrameKind::Picture,
                    rect: Rect::new(MARGIN, cursor, width, h),
                    story_id: None,
                    role: ContentRole::Standing,
                    picture: None,
                    music,
                });
            }
            cursor += h + GAP;
        }
    }

    // The credit line under the music, as words. The
    // draft leaves the cursor unadvanced after it, so
    // the permission line is what steps past it.
    if let Some(credit) = info.credit_line() {
        let (next_here, next_cursor) = overflow_panel(doc, here, cursor, CREDIT_H);
        here = next_here;
        cursor = next_cursor;
        let credit_id = fresh_story_id(doc, "music-credit");
        doc.stories.push(Story {
            id: credit_id.clone(),
            paragraphs: vec![Paragraph::plain("caption", &credit)],
            keep_through: None,
        });
        let frame_id = fresh_frame_id(doc, "music-credit");
        doc.panels[here].frames.push(Frame {
            id: frame_id,
            name: "Music credit".into(),
            kind: FrameKind::Text,
            rect: Rect::new(MARGIN, cursor, width, CREDIT_H),
            story_id: Some(credit_id),
            role: ContentRole::Standing,
            picture: None,
            music: None,
        });
    }

    // The distinct per-song permission line under the
    // credit; a licensed song with no number stated
    // gets no numberless line, and the window is told.
    let mut permission_missing_number = false;
    if let Some(line) = ob_music::license::permission_line_for_music(&info) {
        cursor += CREDIT_H + GAP;
        let (next_here, next_cursor) = overflow_panel(doc, here, cursor, CREDIT_H);
        here = next_here;
        cursor = next_cursor;
        let permission_id = fresh_story_id(doc, "music-permission");
        doc.stories.push(Story {
            id: permission_id.clone(),
            paragraphs: vec![Paragraph::plain("caption", &line)],
            keep_through: None,
        });
        let frame_id = fresh_frame_id(doc, "music-permission");
        doc.panels[here].frames.push(Frame {
            id: frame_id,
            name: "Permission line".into(),
            kind: FrameKind::Text,
            rect: Rect::new(MARGIN, cursor, width, CREDIT_H),
            story_id: Some(permission_id),
            role: ContentRole::Standing,
            picture: None,
            music: None,
        });
    } else if license_from_rights(info.rights.as_deref()) == LicenseKind::LicensedReprint {
        permission_missing_number = true;
    }

    Ok(SongPlacement {
        systems,
        rendered: render.is_some(),
        permission_missing_number,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ob_model::{AssetKind, ChurchProfile, LicenseKind};
    use std::path::PathBuf;

    fn test_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(name);
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn paths(dir: &Path) -> (PathBuf, PathBuf, PathBuf) {
        (
            dir.join("snippets.json"),
            dir.join("snippets"),
            dir.join("fingerprints.json"),
        )
    }

    fn review(doc: Document, workdir: Option<PathBuf>, source_name: &str) -> ImportReview {
        ImportReview {
            doc,
            items: Vec::new(),
            workdir,
            source_name: source_name.to_string(),
            note: None,
        }
    }

    fn notes_doc() -> Document {
        ob_model::markdown::document_from_markdown(
            "Welcome to worship\n\nLord, in your mercy hear our prayer",
            ChurchProfile::default(),
            "notes.md",
        )
    }

    /// A PNG built from a pixel function, so the tests
    /// need no fixture files.
    fn png_bytes(w: u32, h: u32, pixel: impl Fn(u32, u32) -> (u8, u8, u8)) -> Vec<u8> {
        use image::{ImageBuffer, Rgb};
        let img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_fn(w, h, |x, y| {
            let (r, g, b) = pixel(x, y);
            Rgb([r, g, b])
        });
        let mut bytes = Vec::new();
        image::DynamicImage::ImageRgb8(img)
            .write_to(
                &mut std::io::Cursor::new(&mut bytes),
                image::ImageFormat::Png,
            )
            .expect("PNG encode works");
        bytes
    }

    fn busy_picture() -> Vec<u8> {
        png_bytes(32, 32, |x, y| {
            let v = ((x * 7 + y * 13 + x * y * 3) % 256) as u8;
            (v, v, v)
        })
    }

    fn busy_picture_near() -> Vec<u8> {
        png_bytes(32, 32, |x, y| {
            let mut v = ((x * 7 + y * 13 + x * y * 3) % 256) as u8;
            if (10..16).contains(&x) && (10..16).contains(&y) {
                v = v.saturating_add(40);
            }
            (v, v, v)
        })
    }

    fn scale_c_xml() -> String {
        let mut notes = String::new();
        for (s, o) in [
            ("C", 4),
            ("D", 4),
            ("E", 4),
            ("F", 4),
            ("G", 4),
            ("A", 4),
            ("B", 4),
        ] {
            notes.push_str(&format!(
                "<note><pitch><step>{s}</step><octave>{o}</octave></pitch>\
                 <duration>4</duration><type>quarter</type></note>"
            ));
        }
        format!(
            r#"<?xml version="1.0"?><score-partwise>
            <part-list><score-part id="P1"><part-name>V</part-name></score-part></part-list>
            <part id="P1"><measure><attributes><divisions>4</divisions><key><fifths>0</fifths><mode>major</mode></key></attributes>
            {notes}
            </measure></part></score-partwise>"#
        )
    }

    #[test]
    fn md_helper_yields_one_keepable_per_paragraph() {
        let doc = notes_doc();
        let keepables = keepables_for_story(&doc, "notes");
        assert_eq!(keepables.len(), 2);
        for (i, keepable) in keepables.iter().enumerate() {
            match &keepable.kind {
                KeepKind::Text {
                    story_id,
                    paragraph,
                } => {
                    assert_eq!(story_id, "notes");
                    assert_eq!(*paragraph, i);
                }
                other => panic!("text keepable expected, got {other:?}"),
            }
        }
        assert_eq!(keepables[0].default_name, "Welcome to worship");
        assert_eq!(
            keepables[1].default_name,
            "Lord, in your mercy hear our prayer"
        );
        assert!(keepables_for_story(&doc, "no-such-story").is_empty());
    }

    #[test]
    fn resolver_reads_picture_bytes_and_rights() {
        let mut doc = notes_doc();
        let bytes = busy_picture();
        let rights = RightsRecord {
            source: "old.pdf".to_string(),
            license: LicenseKind::CcBy("4.0".to_string()),
            title: "Cross".to_string(),
            creator: "Jane Doe".to_string(),
            ..RightsRecord::default()
        };
        let sha = doc.add_asset(&bytes, AssetKind::Picture, "image/png", rights.clone());
        let rev = review(doc, None, "old.pdf");
        let keepable = Keepable {
            kind: KeepKind::Picture { sha },
            default_name: "cross.png".to_string(),
        };
        let input = resolve_keep_input(&rev, &keepable, "Cross").expect("picture resolves");
        assert_eq!(input.content, bytes);
        assert_eq!(input.kind, ob_snippets::SnippetKind::Picture);
        assert_eq!(input.media_type, "image/png");
        assert_eq!(input.rights, rights);
        assert_eq!(input.name, "Cross");
        assert_eq!(input.source, "old.pdf");
        assert!(!input.spoken_only && !input.web_hidden);
        assert_eq!(input.kept_at.len(), 10, "kept_at is an ISO date");
        assert_eq!(&input.kept_at[4..5], "-");
        assert_eq!(&input.kept_at[7..8], "-");
    }

    #[test]
    fn resolver_reads_music_bytes_and_rights() {
        let mut doc = notes_doc();
        let xml = scale_c_xml();
        let bytes = xml.into_bytes();
        let rights = RightsRecord {
            source: "tiny.musicxml".to_string(),
            license: LicenseKind::PublicDomain,
            ..RightsRecord::default()
        };
        let sha = doc.add_asset(
            &bytes,
            AssetKind::Music,
            "application/vnd.recordare.musicxml+xml",
            rights.clone(),
        );
        let rev = review(doc, None, "tiny.musicxml");
        let keepable = Keepable {
            kind: KeepKind::Music { sha },
            default_name: "A Tiny Test Hymn".to_string(),
        };
        let input =
            resolve_keep_input(&rev, &keepable, "A Tiny Test Hymn").expect("music resolves");
        assert_eq!(input.content, bytes);
        assert_eq!(input.kind, ob_snippets::SnippetKind::Music);
        assert_eq!(input.media_type, "application/vnd.recordare.musicxml+xml");
        assert_eq!(input.rights, rights);
        assert_eq!(input.source, "tiny.musicxml");
    }

    #[test]
    fn resolver_reads_text_words_and_flags() {
        let mut doc = notes_doc();
        doc.stories[0].paragraphs[0].spoken_only = true;
        doc.stories[0].paragraphs[0].web_hidden = true;
        let rev = review(doc, None, "notes.md");
        let keepable = Keepable {
            kind: KeepKind::Text {
                story_id: "notes".to_string(),
                paragraph: 0,
            },
            default_name: "Welcome to worship".to_string(),
        };
        let input = resolve_keep_input(&rev, &keepable, "Welcome").expect("text resolves");
        assert_eq!(input.content, b"Welcome to worship");
        assert_eq!(input.kind, ob_snippets::SnippetKind::Text);
        assert_eq!(input.media_type, "text/plain");
        assert_eq!(input.rights.source, "notes.md");
        assert_eq!(input.rights.license, LicenseKind::Unknown);
        assert!(input.spoken_only);
        assert!(input.web_hidden);
        assert_eq!(input.asserted_by, "");
    }

    #[test]
    fn resolver_reads_page_picture_from_workdir() {
        let dir = test_dir("ob-app-page-picture-test");
        let workdir = dir.join("workdir");
        let bytes = busy_picture();
        std::fs::create_dir_all(workdir.join("assets")).expect("mkdir");
        std::fs::write(workdir.join("assets").join("page-01.png"), &bytes)
            .expect("write page picture");
        let rev = review(notes_doc(), Some(workdir), "old.pdf");
        let keepable = Keepable {
            kind: KeepKind::PagePicture { page: 0 },
            default_name: "Page 1 picture".to_string(),
        };
        let input =
            resolve_keep_input(&rev, &keepable, "Page 1 picture").expect("page picture resolves");
        assert_eq!(input.content, bytes);
        assert_eq!(input.kind, ob_snippets::SnippetKind::Picture);
        assert_eq!(input.media_type, "image/png");
        assert_eq!(input.rights.source, "old.pdf");
        // A second page's file is missing: loud.
        let missing = Keepable {
            kind: KeepKind::PagePicture { page: 1 },
            default_name: "Page 2 picture".to_string(),
        };
        let err = resolve_keep_input(&rev, &missing, "Page 2 picture")
            .expect_err("missing page picture is loud");
        assert!(err.contains("page-02.png"), "error is {err}");
        // No workdir at all: loud.
        let rev_nowd = review(notes_doc(), None, "old.pdf");
        let err = resolve_keep_input(&rev_nowd, &keepable, "Page 1 picture")
            .expect_err("no workdir is loud");
        assert!(err.contains("no page pictures"), "error is {err}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolver_missing_bytes_story_and_paragraph_are_loud() {
        let rev = review(notes_doc(), None, "old.pdf");
        let missing_pic = Keepable {
            kind: KeepKind::Picture {
                sha: "00".repeat(32),
            },
            default_name: "gone.png".to_string(),
        };
        let err = resolve_keep_input(&rev, &missing_pic, "gone")
            .expect_err("missing asset bytes are loud");
        assert!(err.contains("missing"), "error is {err}");
        let missing_story = Keepable {
            kind: KeepKind::Text {
                story_id: "no-such-story".to_string(),
                paragraph: 0,
            },
            default_name: "x".to_string(),
        };
        let err = resolve_keep_input(&rev, &missing_story, "x").expect_err("missing story is loud");
        assert!(err.contains("no-such-story"), "error is {err}");
        let missing_para = Keepable {
            kind: KeepKind::Text {
                story_id: "notes".to_string(),
                paragraph: 9,
            },
            default_name: "x".to_string(),
        };
        let err =
            resolve_keep_input(&rev, &missing_para, "x").expect_err("missing paragraph is loud");
        assert!(err.contains("Paragraph 10"), "error is {err}");
    }

    #[test]
    fn cancel_safety_keep_survives_workdir_delete() {
        let dir = test_dir("ob-app-cancel-safety-test");
        let (store_path, bytes_dir, index_path) = paths(&dir);
        let workdir = dir.join("workdir");
        let page_bytes = busy_picture();
        std::fs::create_dir_all(workdir.join("assets")).expect("mkdir");
        std::fs::write(workdir.join("assets").join("page-01.png"), &page_bytes)
            .expect("write page picture");
        let rev = review(notes_doc(), Some(workdir.clone()), "old.pdf");
        // Tick the text keepable and the page-picture
        // keepable: both execute immediately.
        let text_keep = keepables_for_story(&rev.doc, "notes")[0].clone();
        let outcome = execute_keep(
            &rev,
            0,
            0,
            &text_keep,
            "Welcome",
            &store_path,
            &bytes_dir,
            &index_path,
        );
        assert!(matches!(outcome, KeepOutcome::Kept), "got {outcome:?}");
        let page_keep = Keepable {
            kind: KeepKind::PagePicture { page: 0 },
            default_name: "Page 1 picture".to_string(),
        };
        let outcome = execute_keep(
            &rev,
            0,
            1,
            &page_keep,
            "Page 1 picture",
            &store_path,
            &bytes_dir,
            &index_path,
        );
        assert!(matches!(outcome, KeepOutcome::Kept), "got {outcome:?}");
        // Cancel: the workdir is deleted, the way the
        // Cancel path deletes it.
        std::fs::remove_dir_all(&workdir).expect("delete workdir");
        assert!(!workdir.exists());
        // The library still holds both snippets and
        // their bytes.
        let store = SnippetStore::load(&store_path).expect("store reloads");
        assert_eq!(store.snippets.len(), 2);
        let text_record = store
            .snippets
            .iter()
            .find(|r| r.name == "Welcome")
            .expect("text snippet kept");
        assert_eq!(
            std::fs::read(bytes_dir.join(&text_record.id)).expect("text bytes kept"),
            b"Welcome to worship"
        );
        let pic_record = store
            .snippets
            .iter()
            .find(|r| r.name == "Page 1 picture")
            .expect("picture snippet kept");
        assert_eq!(
            std::fs::read(bytes_dir.join(&pic_record.id)).expect("picture bytes kept"),
            page_bytes
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn review_with_no_ticks_stores_nothing() {
        let dir = test_dir("ob-app-no-tick-test");
        let (store_path, bytes_dir, index_path) = paths(&dir);
        let _rev = review(notes_doc(), None, "old.pdf");
        // No tick executes: the store file is never
        // created, and a load reads empty.
        assert!(!store_path.exists());
        assert!(!bytes_dir.exists());
        assert!(!index_path.exists());
        let store = SnippetStore::load(&store_path).expect("missing store loads empty");
        assert!(store.snippets.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn second_tick_of_the_same_bytes_is_already_kept() {
        let dir = test_dir("ob-app-second-tick-test");
        let (store_path, bytes_dir, index_path) = paths(&dir);
        let mut doc = notes_doc();
        let bytes = busy_picture();
        let sha = doc.add_asset(
            &bytes,
            AssetKind::Picture,
            "image/png",
            RightsRecord::default(),
        );
        let rev = review(doc, None, "old.pdf");
        let keepable = Keepable {
            kind: KeepKind::Picture { sha },
            default_name: "busy.png".to_string(),
        };
        let first = execute_keep(
            &rev,
            0,
            0,
            &keepable,
            "busy",
            &store_path,
            &bytes_dir,
            &index_path,
        );
        assert!(matches!(first, KeepOutcome::Kept), "got {first:?}");
        let second = execute_keep(
            &rev,
            0,
            0,
            &keepable,
            "busy",
            &store_path,
            &bytes_dir,
            &index_path,
        );
        assert!(matches!(second, KeepOutcome::AlreadyKept), "got {second:?}");
        let store = SnippetStore::load(&store_path).expect("store reloads");
        assert_eq!(store.snippets.len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ask_flow_keep_anyway_dismiss_and_dont_keep() {
        let dir = test_dir("ob-app-ask-flow-test");
        let (store_path, bytes_dir, index_path) = paths(&dir);
        let mut doc = notes_doc();
        let base = busy_picture();
        let near = busy_picture_near();
        let sha_base = doc.add_asset(
            &base,
            AssetKind::Picture,
            "image/png",
            RightsRecord::default(),
        );
        let sha_near = doc.add_asset(
            &near,
            AssetKind::Picture,
            "image/png",
            RightsRecord::default(),
        );
        let rev = review(doc, None, "old.pdf");
        let base_keep = Keepable {
            kind: KeepKind::Picture { sha: sha_base },
            default_name: "busy.png".to_string(),
        };
        let near_keep = Keepable {
            kind: KeepKind::Picture { sha: sha_near },
            default_name: "busy-near.png".to_string(),
        };
        // Keep-anyway: the near-duplicate asks, then stores.
        let outcome = execute_keep(
            &rev,
            0,
            0,
            &base_keep,
            "busy",
            &store_path,
            &bytes_dir,
            &index_path,
        );
        assert!(matches!(outcome, KeepOutcome::Kept), "got {outcome:?}");
        let outcome = execute_keep(
            &rev,
            0,
            1,
            &near_keep,
            "busy-near",
            &store_path,
            &bytes_dir,
            &index_path,
        );
        let ask = match outcome {
            KeepOutcome::Ask(ask) => ask,
            other => panic!("near-duplicate must Ask, got {other:?}"),
        };
        assert_eq!(ask.proposals.len(), 1);
        let kept = resolve_ask(
            &ask,
            AskChoice::KeepAnyway,
            &store_path,
            &bytes_dir,
            &index_path,
        );
        assert!(matches!(kept, KeepOutcome::Kept), "got {kept:?}");
        let store = SnippetStore::load(&store_path).expect("store reloads");
        assert_eq!(store.snippets.len(), 2);
        // Dismiss, fresh store and index: the same pair
        // asks again, "Not duplicates" stores, and a
        // second tick of the same bytes is already kept.
        let dir_b = test_dir("ob-app-ask-dismiss-test");
        let (store_path_b, bytes_dir_b, index_path_b) = paths(&dir_b);
        let outcome = execute_keep(
            &rev,
            0,
            0,
            &base_keep,
            "busy",
            &store_path_b,
            &bytes_dir_b,
            &index_path_b,
        );
        assert!(matches!(outcome, KeepOutcome::Kept), "got {outcome:?}");
        let outcome = execute_keep(
            &rev,
            0,
            1,
            &near_keep,
            "busy-near",
            &store_path_b,
            &bytes_dir_b,
            &index_path_b,
        );
        let ask = match outcome {
            KeepOutcome::Ask(ask) => ask,
            other => panic!("near-duplicate must Ask, got {other:?}"),
        };
        let kept = resolve_ask(
            &ask,
            AskChoice::NotDuplicates,
            &store_path_b,
            &bytes_dir_b,
            &index_path_b,
        );
        assert!(matches!(kept, KeepOutcome::Kept), "got {kept:?}");
        let again = execute_keep(
            &rev,
            0,
            1,
            &near_keep,
            "busy-near",
            &store_path_b,
            &bytes_dir_b,
            &index_path_b,
        );
        assert!(
            matches!(again, KeepOutcome::AlreadyKept),
            "stored bytes decide AlreadyKept, got {again:?}"
        );
        // Don't keep, fresh store and index: nothing is
        // stored for the near-duplicate.
        let dir_c = test_dir("ob-app-ask-dontkeep-test");
        let (store_path_c, bytes_dir_c, index_path_c) = paths(&dir_c);
        let outcome = execute_keep(
            &rev,
            0,
            0,
            &base_keep,
            "busy",
            &store_path_c,
            &bytes_dir_c,
            &index_path_c,
        );
        assert!(matches!(outcome, KeepOutcome::Kept), "got {outcome:?}");
        let outcome = execute_keep(
            &rev,
            0,
            1,
            &near_keep,
            "busy-near",
            &store_path_c,
            &bytes_dir_c,
            &index_path_c,
        );
        let ask = match outcome {
            KeepOutcome::Ask(ask) => ask,
            other => panic!("near-duplicate must Ask, got {other:?}"),
        };
        let not_kept = resolve_ask(
            &ask,
            AskChoice::DontKeep,
            &store_path_c,
            &bytes_dir_c,
            &index_path_c,
        );
        assert!(matches!(not_kept, KeepOutcome::NotKept), "got {not_kept:?}");
        let store = SnippetStore::load(&store_path_c).expect("store reloads");
        assert_eq!(store.snippets.len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&dir_b);
        let _ = std::fs::remove_dir_all(&dir_c);
    }

    // ---- Batch C: picker placement + C-1 + C-2 ----

    fn licensed_scale_xml() -> String {
        scale_c_xml().replace(
            "<part-list>",
            "<identification><rights>© 2019 Jane Doe. Reprinted under CCLI license #123456</rights>\
             <creator type=\"composer\">Test Composer</creator></identification><part-list>",
        )
    }

    fn fake_render() -> ob_music::render::RenderOutcome {
        ob_music::render::RenderOutcome {
            backend: "Verovio 6.3.0 via test".into(),
            systems: vec![
                ob_music::render::RenderedSystem {
                    svg: "<svg xmlns=\"http://www.w3.org/2000/svg\"/>".into(),
                    width_units: 1437.0,
                    height_units: 231.0,
                    staff_mm: 6.5,
                    noteheads: 5,
                    measures: 2,
                    syllables: 3,
                    fermatas: 1,
                },
                ob_music::render::RenderedSystem {
                    svg: "<svg xmlns=\"http://www.w3.org/2000/svg\"/>".into(),
                    width_units: 1437.0,
                    height_units: 200.0,
                    staff_mm: 6.5,
                    noteheads: 4,
                    measures: 1,
                    syllables: 2,
                    fermatas: 0,
                },
            ],
        }
    }

    fn add_empty_picture_frame(doc: &mut Document) -> usize {
        doc.panels[0].frames.push(Frame {
            id: "frame-pic-1".into(),
            name: "Picture box".into(),
            kind: FrameKind::Picture,
            rect: Rect {
                x: 198_000,
                y: 315_000,
                w: 216_000,
                h: 162_000,
            },
            story_id: None,
            role: ContentRole::Weekly,
            picture: None,
            music: None,
        });
        doc.panels[0].frames.len() - 1
    }

    fn add_music_frame(doc: &mut Document, source: &str) -> usize {
        doc.panels[0].frames.push(Frame {
            id: "frame-music-1".into(),
            name: "Music line 1".into(),
            kind: FrameKind::Picture,
            rect: Rect {
                x: 14_400,
                y: 200_000,
                w: 367_200,
                h: 90_000,
            },
            story_id: None,
            role: ContentRole::Standing,
            picture: Some(PicturePlacement::new("00".repeat(32))),
            music: Some(MusicFrame {
                system: 0,
                staff_mm: 6.5,
                source: source.to_string(),
            }),
        });
        doc.panels[0].frames.len() - 1
    }

    /// Keep one keepable from a review and hand back the
    /// temp dir (for cleanup), the stored record, and the
    /// stored bytes.
    fn keep_one(
        dir_name: &str,
        rev: &ImportReview,
        keepable: &Keepable,
        name: &str,
    ) -> (PathBuf, SnippetRecord, Vec<u8>) {
        let dir = test_dir(dir_name);
        let (store_path, bytes_dir, index_path) = paths(&dir);
        let outcome = execute_keep(
            rev,
            0,
            0,
            keepable,
            name,
            &store_path,
            &bytes_dir,
            &index_path,
        );
        assert!(matches!(outcome, KeepOutcome::Kept), "got {outcome:?}");
        let store = SnippetStore::load(&store_path).expect("store reloads");
        assert_eq!(store.snippets.len(), 1);
        let record = store.snippets[0].clone();
        let bytes = std::fs::read(bytes_dir.join(&record.id)).expect("bytes kept");
        (dir, record, bytes)
    }

    #[test]
    fn empty_and_whitespace_names_store_the_default() {
        let mut doc = notes_doc();
        let bytes = busy_picture();
        let sha = doc.add_asset(
            &bytes,
            AssetKind::Picture,
            "image/png",
            RightsRecord::default(),
        );
        let rev = review(doc, None, "old.pdf");
        let keepable = Keepable {
            kind: KeepKind::Picture { sha },
            default_name: "cross.png".to_string(),
        };
        let input = resolve_keep_input(&rev, &keepable, "").expect("resolves");
        assert_eq!(input.name, "cross.png", "an emptied name falls back");
        let input = resolve_keep_input(&rev, &keepable, "   \t ").expect("resolves");
        assert_eq!(input.name, "cross.png", "whitespace-only falls back too");
        let input = resolve_keep_input(&rev, &keepable, "  Cross  ").expect("resolves");
        assert_eq!(input.name, "  Cross  ", "a real name is kept verbatim");
        let dir = test_dir("ob-app-empty-name-test");
        let (store_path, bytes_dir, index_path) = paths(&dir);
        let outcome = execute_keep(
            &rev,
            0,
            0,
            &keepable,
            "",
            &store_path,
            &bytes_dir,
            &index_path,
        );
        assert!(matches!(outcome, KeepOutcome::Kept), "got {outcome:?}");
        let store = SnippetStore::load(&store_path).expect("store reloads");
        assert_eq!(store.snippets[0].name, "cross.png");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn cancel_note_names_kept_items_when_any_survived() {
        assert_eq!(cancel_note(false), "Import cancelled; nothing changed.");
        assert_eq!(
            cancel_note(true),
            "Import cancelled. Items you kept stay in the library."
        );
    }

    #[test]
    fn credit_line_and_search_cover_the_picker_rows() {
        let mut doc = notes_doc();
        let bytes = busy_picture();
        let rights = RightsRecord {
            source: "old.pdf".to_string(),
            license: LicenseKind::CcBy("4.0".to_string()),
            title: "Cross".to_string(),
            creator: "Jane Doe".to_string(),
            credit_text: "Cross by Jane Doe".to_string(),
            licensor_ref: "CCLI #123456".to_string(),
            ..RightsRecord::default()
        };
        let sha = doc.add_asset(&bytes, AssetKind::Picture, "image/png", rights);
        let rev = review(doc, None, "old.pdf");
        let keepable = Keepable {
            kind: KeepKind::Picture { sha },
            default_name: "cross.png".to_string(),
        };
        let (dir, mut record, _kept) =
            keep_one("ob-app-credit-line-test", &rev, &keepable, "Cross");
        assert_eq!(snippet_credit_line(&record), "Cross by Jane Doe");
        record.asset.rights.credit_text = String::new();
        assert_eq!(snippet_credit_line(&record), "Cross — Jane Doe");
        record.asset.rights.creator = String::new();
        assert_eq!(snippet_credit_line(&record), "Cross");
        record.asset.rights.title = String::new();
        assert_eq!(snippet_credit_line(&record), "From old.pdf");
        let mut store = SnippetStore::default();
        let mut second = record.clone();
        second.name = "Dove".to_string();
        second.id = "cc".repeat(32);
        store.snippets.push(record);
        store.snippets.push(second);
        assert_eq!(matching_snippets(&store, "").len(), 2);
        assert_eq!(matching_snippets(&store, "cross").len(), 1);
        assert_eq!(matching_snippets(&store, "DOVE").len(), 1);
        assert!(matching_snippets(&store, "candle").is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn picture_keep_reuses_into_a_fresh_bulletin_with_rights_intact() {
        let mut doc = notes_doc();
        let bytes = busy_picture();
        let rights = RightsRecord {
            source: "old.pdf".to_string(),
            license: LicenseKind::Unknown,
            title: "Cross".to_string(),
            creator: "Jane Doe".to_string(),
            copyright_line: "© 2019 Jane Doe".to_string(),
            credit_text: "Cross by Jane Doe".to_string(),
            licensor_ref: "CCLI #123456".to_string(),
            children_present: Some(true),
            child_web_permission: Some(false),
            ..RightsRecord::default()
        };
        let sha = doc.add_asset(&bytes, AssetKind::Picture, "image/png", rights.clone());
        let rev = review(doc, None, "old.pdf");
        let keepable = Keepable {
            kind: KeepKind::Picture { sha },
            default_name: "cross.png".to_string(),
        };
        let (dir, record, kept_bytes) =
            keep_one("ob-app-picture-reuse-test", &rev, &keepable, "Cross");
        assert_eq!(kept_bytes, bytes, "the store holds the original bytes");
        let mut fresh = notes_doc();
        let fi = add_empty_picture_frame(&mut fresh);
        let placed_sha =
            place_snippet_picture_in_doc(&mut fresh, 0, fi, &record, &kept_bytes).expect("places");
        assert_eq!(
            fresh.asset_data(&placed_sha).expect("asset bytes"),
            bytes.as_slice(),
            "bytes identical after reuse (SHA-256)"
        );
        assert_eq!(ob_model::sha256_hex(&bytes), placed_sha);
        let asset = fresh
            .assets
            .iter()
            .find(|a| a.sha256 == placed_sha)
            .expect("asset record");
        assert_eq!(
            asset.rights, rights,
            "the FULL rights record survives, field for field"
        );
        assert_eq!(asset.rights.license, LicenseKind::Unknown);
        assert_eq!(asset.rights.child_web_permission, Some(false));
        assert_eq!(asset.rights.licensor_ref, "CCLI #123456");
        let frame = &fresh.panels[0].frames[fi];
        assert_eq!(frame.kind, FrameKind::Picture);
        assert_eq!(frame.picture.as_ref().unwrap().asset, placed_sha);
        assert!(frame.story_id.is_none());
        assert_eq!(fresh.format_version, ob_model::FORMAT_VERSION);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn picture_into_a_music_box_is_refused_in_the_same_words() {
        let mut doc = notes_doc();
        let bytes = busy_picture();
        let sha = doc.add_asset(
            &bytes,
            AssetKind::Picture,
            "image/png",
            RightsRecord::default(),
        );
        let rev = review(doc, None, "old.pdf");
        let keepable = Keepable {
            kind: KeepKind::Picture { sha },
            default_name: "cross.png".to_string(),
        };
        let (dir, record, kept_bytes) =
            keep_one("ob-app-picture-music-refuse-test", &rev, &keepable, "Cross");
        let mut fresh = notes_doc();
        let fi = add_music_frame(&mut fresh, &ob_model::sha256_hex(b"old song"));
        let err = place_snippet_picture_in_doc(&mut fresh, 0, fi, &record, &kept_bytes)
            .expect_err("a picture cannot go in a music box");
        assert_eq!(err, "That box holds music; a picture cannot go in it.");
        assert!(fresh.assets.is_empty(), "nothing was stored on refusal");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn children_picture_without_web_permission_stays_off_the_web_page_after_reuse() {
        let mut doc = notes_doc();
        let bytes = busy_picture();
        let rights = RightsRecord {
            source: "old.pdf".to_string(),
            license: LicenseKind::Unknown,
            title: "Cross".to_string(),
            creator: "Jane Doe".to_string(),
            children_present: Some(true),
            child_web_permission: Some(false),
            ..RightsRecord::default()
        };
        let sha = doc.add_asset(&bytes, AssetKind::Picture, "image/png", rights);
        let rev = review(doc, None, "old.pdf");
        let keepable = Keepable {
            kind: KeepKind::Picture { sha },
            default_name: "cross.png".to_string(),
        };
        let (dir, record, kept_bytes) =
            keep_one("ob-app-children-reuse-test", &rev, &keepable, "Cross");
        let mut fresh = notes_doc();
        let fi = add_empty_picture_frame(&mut fresh);
        place_snippet_picture_in_doc(&mut fresh, 0, fi, &record, &kept_bytes).expect("places");
        let (html_bytes, report) = ob_extract::html_export::export_html(&fresh).expect("export");
        let html = String::from_utf8(html_bytes).expect("utf8");
        assert_eq!(report.children_left_out, 1);
        assert!(
            !html.contains("alt=\"Cross\""),
            "an unpermissioned picture of children reached the web page"
        );
        assert!(html.contains(
            "A picture with children in it belongs here; it is in the printed bulletin."
        ));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn song_keep_reuses_with_heading_credit_and_permission_line() {
        let xml = licensed_scale_xml();
        let mut doc = notes_doc();
        let rights = ob_music::draft::rights_from_music(
            &ob_music::load_music(xml.as_bytes()).expect("parses").1,
            "tiny.musicxml",
        );
        let sha = doc.add_asset(
            xml.as_bytes(),
            AssetKind::Music,
            "application/vnd.recordare.musicxml+xml",
            rights.clone(),
        );
        let rev = review(doc, None, "tiny.musicxml");
        let keepable = Keepable {
            kind: KeepKind::Music { sha },
            default_name: "A Tiny Test Hymn".to_string(),
        };
        let (dir, record, kept_bytes) = keep_one(
            "ob-app-song-reuse-test",
            &rev,
            &keepable,
            "A Tiny Test Hymn",
        );
        assert_eq!(kept_bytes, xml.as_bytes());
        let mut fresh = notes_doc();
        let placed = place_snippet_song_in_doc(
            &mut fresh,
            0,
            None,
            &record,
            &kept_bytes,
            Some(&fake_render()),
        )
        .expect("places");
        assert_eq!(placed.systems, 2);
        assert!(placed.rendered);
        assert!(!placed.permission_missing_number);
        // The music asset rides with the stored rights.
        let music_asset = fresh
            .assets
            .iter()
            .find(|a| a.kind == AssetKind::Music)
            .expect("the music asset");
        assert_eq!(music_asset.rights, rights);
        assert_eq!(music_asset.rights.licensor_ref, "123456");
        assert_eq!(
            fresh.asset_data(&music_asset.sha256).expect("music bytes"),
            xml.as_bytes(),
            "song bytes identical after reuse"
        );
        // Two systems, both full-width music frames.
        let music_frames: Vec<&Frame> = fresh
            .panels
            .iter()
            .flat_map(|p| &p.frames)
            .filter(|f| f.music.is_some())
            .collect();
        assert_eq!(music_frames.len(), 2);
        assert_eq!(music_frames[0].music.as_ref().unwrap().system, 0);
        assert_eq!(music_frames[1].music.as_ref().unwrap().system, 1);
        assert_eq!(
            music_frames[0].music.as_ref().unwrap().source,
            music_asset.sha256
        );
        assert_eq!(music_frames[0].rect.w, 612_000 - 2 * 14_400);
        assert_eq!(music_frames[0].music.as_ref().unwrap().staff_mm, 6.5);
        let svg_hash = music_frames[0].picture.as_ref().unwrap().asset.clone();
        assert_eq!(
            fresh.asset_data(&svg_hash).expect("svg bytes"),
            b"<svg xmlns=\"http://www.w3.org/2000/svg\"/>"
        );
        // Heading + caption stories ride exactly as the
        // draft adds them.
        let heading = fresh.story("music-heading").expect("heading story");
        assert!(heading
            .paragraphs
            .iter()
            .any(|p| p.text() == "Test Composer"));
        let credit = fresh.story("music-credit").expect("credit story");
        assert!(credit.paragraphs[0].text().contains("CCLI license #123456"));
        let permission = fresh
            .story("music-permission")
            .expect("the permission story");
        assert!(
            permission
                .paragraphs
                .iter()
                .any(|p| p.text().contains("CCLI license #123456")),
            "{:?}",
            permission.paragraphs
        );
        assert!(
            fresh
                .panels
                .iter()
                .flat_map(|p| p.frames.iter())
                .any(|f| f.story_id.as_deref() == Some("music-permission")),
            "the permission story is framed, so it prints"
        );
        assert_eq!(fresh.format_version, ob_model::FORMAT_VERSION);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn song_placement_without_an_engraver_leaves_the_honest_empty_box() {
        let xml = licensed_scale_xml();
        let mut doc = notes_doc();
        let sha = doc.add_asset(
            xml.as_bytes(),
            AssetKind::Music,
            "application/vnd.recordare.musicxml+xml",
            RightsRecord::default(),
        );
        let rev = review(doc, None, "tiny.musicxml");
        let keepable = Keepable {
            kind: KeepKind::Music { sha },
            default_name: "A Tiny Test Hymn".to_string(),
        };
        let (dir, record, kept_bytes) = keep_one(
            "ob-app-song-noengraver-test",
            &rev,
            &keepable,
            "A Tiny Test Hymn",
        );
        let mut fresh = notes_doc();
        let placed = place_snippet_song_in_doc(&mut fresh, 0, None, &record, &kept_bytes, None)
            .expect("places");
        assert_eq!(placed.systems, 1);
        assert!(!placed.rendered);
        let music: Vec<&Frame> = fresh
            .panels
            .iter()
            .flat_map(|p| &p.frames)
            .filter(|f| f.music.is_some())
            .collect();
        assert_eq!(music.len(), 1);
        assert!(music[0].picture.is_none(), "no drawing is faked");
        assert_eq!(music[0].music.as_ref().unwrap().system, 0);
        assert_eq!(music[0].music.as_ref().unwrap().staff_mm, 6.5);
        assert_eq!(
            music[0].rect.h,
            ((612_000 - 2 * 14_400) as f64 * 0.42).round() as i64
        );
        assert!(fresh.story("music-heading").is_some());
        assert!(fresh.story("music-credit").is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn song_empty_frame_takes_system_zero() {
        let xml = licensed_scale_xml();
        let mut doc = notes_doc();
        let sha = doc.add_asset(
            xml.as_bytes(),
            AssetKind::Music,
            "application/vnd.recordare.musicxml+xml",
            RightsRecord::default(),
        );
        let rev = review(doc, None, "tiny.musicxml");
        let keepable = Keepable {
            kind: KeepKind::Music { sha },
            default_name: "A Tiny Test Hymn".to_string(),
        };
        let (dir, record, kept_bytes) = keep_one(
            "ob-app-song-empty-frame-test",
            &rev,
            &keepable,
            "A Tiny Test Hymn",
        );
        let mut fresh = notes_doc();
        let fi = add_empty_picture_frame(&mut fresh);
        let frames_before = fresh.panels[0].frames.len();
        place_snippet_song_in_doc(
            &mut fresh,
            0,
            Some(fi),
            &record,
            &kept_bytes,
            Some(&fake_render()),
        )
        .expect("places");
        let frame = &fresh.panels[0].frames[fi];
        assert_eq!(frame.music.as_ref().unwrap().system, 0);
        assert!(frame.picture.is_some(), "system 0 is drawn in the box");
        // System 1 is a new frame after it; the box count
        // grew by the second system plus heading, credit,
        // and permission frames only.
        assert_eq!(fresh.panels[0].frames.len(), frames_before + 4);
        let systems: Vec<u32> = fresh
            .panels
            .iter()
            .flat_map(|p| &p.frames)
            .filter_map(|f| f.music.as_ref().map(|m| m.system))
            .collect();
        assert_eq!(systems, vec![0, 1]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn song_music_frame_is_replaced() {
        let xml = licensed_scale_xml();
        let mut doc = notes_doc();
        let sha = doc.add_asset(
            xml.as_bytes(),
            AssetKind::Music,
            "application/vnd.recordare.musicxml+xml",
            RightsRecord::default(),
        );
        let rev = review(doc, None, "tiny.musicxml");
        let keepable = Keepable {
            kind: KeepKind::Music { sha },
            default_name: "A Tiny Test Hymn".to_string(),
        };
        let (dir, record, kept_bytes) = keep_one(
            "ob-app-song-music-frame-test",
            &rev,
            &keepable,
            "A Tiny Test Hymn",
        );
        let mut fresh = notes_doc();
        let old_source = ob_model::sha256_hex(b"an older song");
        let fi = add_music_frame(&mut fresh, &old_source);
        place_snippet_song_in_doc(
            &mut fresh,
            0,
            Some(fi),
            &record,
            &kept_bytes,
            Some(&fake_render()),
        )
        .expect("places");
        let frame = &fresh.panels[0].frames[fi];
        let music = frame.music.as_ref().unwrap();
        assert_eq!(music.system, 0);
        assert_ne!(
            music.source, old_source,
            "the old song is gone from the box"
        );
        assert_eq!(
            music.source,
            ob_model::sha256_hex(xml.as_bytes()),
            "the box now carries the kept song"
        );
        assert!(frame.picture.is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn song_with_no_frame_creates_new_frames() {
        let xml = licensed_scale_xml();
        let mut doc = notes_doc();
        let sha = doc.add_asset(
            xml.as_bytes(),
            AssetKind::Music,
            "application/vnd.recordare.musicxml+xml",
            RightsRecord::default(),
        );
        let rev = review(doc, None, "tiny.musicxml");
        let keepable = Keepable {
            kind: KeepKind::Music { sha },
            default_name: "A Tiny Test Hymn".to_string(),
        };
        let (dir, record, kept_bytes) = keep_one(
            "ob-app-song-no-frame-test",
            &rev,
            &keepable,
            "A Tiny Test Hymn",
        );
        let mut fresh = notes_doc();
        let frames_before = fresh.panels[0].frames.len();
        place_snippet_song_in_doc(
            &mut fresh,
            0,
            None,
            &record,
            &kept_bytes,
            Some(&fake_render()),
        )
        .expect("places");
        // Two system frames plus heading, credit, and
        // permission frames, all new, all on panel 0.
        assert_eq!(fresh.panels[0].frames.len(), frames_before + 5);
        assert_eq!(fresh.panels.len(), 1, "no overflow panel was needed");
        let systems: Vec<u32> = fresh
            .panels
            .iter()
            .flat_map(|p| &p.frames)
            .filter_map(|f| f.music.as_ref().map(|m| m.system))
            .collect();
        assert_eq!(systems, vec![0, 1]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn song_placement_refuses_a_filled_non_music_box() {
        let xml = licensed_scale_xml();
        let mut doc = notes_doc();
        let sha = doc.add_asset(
            xml.as_bytes(),
            AssetKind::Music,
            "application/vnd.recordare.musicxml+xml",
            RightsRecord::default(),
        );
        let rev = review(doc, None, "tiny.musicxml");
        let keepable = Keepable {
            kind: KeepKind::Music { sha },
            default_name: "A Tiny Test Hymn".to_string(),
        };
        let (dir, record, kept_bytes) = keep_one(
            "ob-app-song-refuse-test",
            &rev,
            &keepable,
            "A Tiny Test Hymn",
        );
        let mut fresh = notes_doc();
        // A filled picture box: the text frame at index 0
        // is a text box, also refused.
        let fi = add_empty_picture_frame(&mut fresh);
        fresh.panels[0].frames[fi].picture = Some(PicturePlacement::new("11".repeat(32)));
        let err = place_snippet_song_in_doc(
            &mut fresh,
            0,
            Some(fi),
            &record,
            &kept_bytes,
            Some(&fake_render()),
        )
        .expect_err("a filled box refuses the song");
        assert!(err.contains("already holds something"), "error is {err}");
        assert!(fresh.assets.is_empty(), "nothing was stored on refusal");
        let err = place_snippet_song_in_doc(
            &mut fresh,
            0,
            Some(0),
            &record,
            &kept_bytes,
            Some(&fake_render()),
        )
        .expect_err("a text box refuses the song");
        assert!(err.contains("already holds something"), "error is {err}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn text_keep_reuses_as_a_new_text_box_with_its_flags() {
        let mut doc = notes_doc();
        doc.stories[0].paragraphs[0].spoken_only = true;
        doc.stories[0].paragraphs[0].web_hidden = true;
        let rev = review(doc, None, "notes.md");
        let keepable = Keepable {
            kind: KeepKind::Text {
                story_id: "notes".to_string(),
                paragraph: 0,
            },
            default_name: "Welcome to worship".to_string(),
        };
        let (dir, record, kept_bytes) =
            keep_one("ob-app-text-reuse-test", &rev, &keepable, "Welcome");
        assert_eq!(kept_bytes, b"Welcome to worship");
        assert!(record.spoken_only && record.web_hidden);
        let mut fresh = notes_doc();
        let (sid, fi) =
            place_snippet_text_in_doc(&mut fresh, 0, &record, &kept_bytes).expect("places");
        let story = fresh.story(&sid).expect("the new story");
        assert_eq!(story.paragraphs.len(), 1);
        assert_eq!(story.paragraphs[0].text(), "Welcome to worship");
        assert_eq!(story.paragraphs[0].style, "body");
        assert!(story.paragraphs[0].spoken_only);
        assert!(story.paragraphs[0].web_hidden);
        let frame = &fresh.panels[0].frames[fi];
        assert_eq!(frame.kind, FrameKind::Text);
        assert_eq!(frame.story_id.as_deref(), Some(sid.as_str()));
        assert_eq!(frame.rect.w, 216_000);
        assert_eq!(frame.rect.h, 108_000);
        assert_eq!(frame.rect.x, (612_000 - 216_000) / 2);
        assert_eq!(frame.rect.y, (792_000 - 108_000) / 2);
        assert_eq!(fresh.format_version, ob_model::FORMAT_VERSION);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn reuse_keeps_format_version_at_one() {
        let xml = licensed_scale_xml();
        let mut doc = notes_doc();
        let pic = busy_picture();
        let pic_sha = doc.add_asset(
            &pic,
            AssetKind::Picture,
            "image/png",
            RightsRecord::default(),
        );
        let song_sha = doc.add_asset(
            xml.as_bytes(),
            AssetKind::Music,
            "application/vnd.recordare.musicxml+xml",
            RightsRecord::default(),
        );
        let rev = review(doc, None, "mixed");
        let pic_keep = Keepable {
            kind: KeepKind::Picture { sha: pic_sha },
            default_name: "busy.png".to_string(),
        };
        let song_keep = Keepable {
            kind: KeepKind::Music { sha: song_sha },
            default_name: "Hymn".to_string(),
        };
        let text_keep = Keepable {
            kind: KeepKind::Text {
                story_id: "notes".to_string(),
                paragraph: 0,
            },
            default_name: "Welcome to worship".to_string(),
        };
        let (dir, pic_record, pic_bytes) =
            keep_one("ob-app-version-pic-test", &rev, &pic_keep, "busy");
        let (dir2, song_record, song_bytes) =
            keep_one("ob-app-version-song-test", &rev, &song_keep, "Hymn");
        let (dir3, text_record, text_bytes) =
            keep_one("ob-app-version-text-test", &rev, &text_keep, "Welcome");
        let mut fresh = notes_doc();
        let fi = add_empty_picture_frame(&mut fresh);
        place_snippet_picture_in_doc(&mut fresh, 0, fi, &pic_record, &pic_bytes).expect("pic");
        place_snippet_song_in_doc(&mut fresh, 0, None, &song_record, &song_bytes, None)
            .expect("song");
        place_snippet_text_in_doc(&mut fresh, 0, &text_record, &text_bytes).expect("text");
        assert_eq!(fresh.format_version, 1, "no .obd schema change");
        assert_eq!(fresh.format_version, ob_model::FORMAT_VERSION);
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&dir2);
        let _ = std::fs::remove_dir_all(&dir3);
    }
}
