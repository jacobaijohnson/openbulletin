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
use ob_model::{AssetKind, Document, RightsRecord};
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
                name: name.to_string(),
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
                name: name.to_string(),
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
                name: name.to_string(),
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
}
