//! OpenBulletin music, Phase A (docs/research/music-
//! landscape.md): bring a music file in, have it engraved
//! faithfully, lay its lines out across the bulletin's
//! pages at a size a layperson can sing from, and account
//! for every note.
//!
//! The scope law, Jacob's ruling and not negotiable: this
//! is NOT music editing. There is no transposing here, no
//! pitch fixing, no notation authoring, ever. The crate
//! reads a file, counts it, asks an engraver to draw it,
//! measures what came back against a legibility floor,
//! and builds the draft. Nothing in it changes a note.
//!
//! Pieces:
//! - [`parse`]: our own MusicXML reader (.musicxml and
//!   .mxl), with structural counts.
//! - [`svg`]: cutting an engraver page render into one
//!   SVG per system, with the staff height measured.
//! - [`render`]: the app side of the engraver helper
//!   protocol (versioned JSON, helper resolved beside the
//!   app like the PDFium helper).
//! - [`engine`]: the helper side: finding and driving
//!   Verovio (out-of-process, LGPL-3.0; see docs/LEGAL.md).
//! - [`draft`]: the draft bulletin a music import leaves.

pub mod draft;
pub mod engine;
pub mod parse;
pub mod render;
pub mod svg;

pub use draft::{build_music_draft, count_item, music_frame_width_mm, music_frame_width_mpt};
pub use parse::{load_music, parse_musicxml, sniff_music, MusicCounts, MusicInfo};
pub use render::{
    render_music, RenderError, RenderOutcome, RenderedSystem, FLOOR_STAFF_MM, TARGET_STAFF_MM,
};

#[cfg(test)]
mod print_tests {
    //! Item 6 proof: a music draft's lines print through
    //! the picture path, and a music box whose drawing is
    //! missing prints the placeholder, counted, never a
    //! silent gap.
    use crate::draft::build_music_draft;
    use crate::parse::{parse_musicxml, TINY_SCORE};
    use crate::render::{RenderOutcome, RenderedSystem};
    use ob_model::ChurchProfile;

    fn draft_with(render: Option<RenderOutcome>) -> ob_model::Document {
        let info = parse_musicxml(TINY_SCORE).expect("parse");
        build_music_draft(
            &info,
            TINY_SCORE.as_bytes(),
            "tiny.musicxml",
            "application/vnd.recordare.musicxml+xml",
            ChurchProfile::default(),
            render.as_ref(),
        )
        .doc
    }

    const SYSTEM_SVG: &str = "<svg xmlns=\"http://www.w3.org/2000/svg\" \
        xmlns:xlink=\"http://www.w3.org/1999/xlink\" width=\"1437px\" height=\"231px\" \
        viewBox=\"0 0 14370 2310\"><g><path d=\"M0 0 L10 10\" fill=\"black\"/></g></svg>";

    #[test]
    fn music_prints_through_the_picture_path() {
        let render = RenderOutcome {
            backend: "test".into(),
            systems: vec![RenderedSystem {
                svg: SYSTEM_SVG.into(),
                width_units: 1437.0,
                height_units: 231.0,
                staff_mm: 6.5,
                noteheads: 5,
                measures: 2,
                syllables: 3,
                fermatas: 1,
            }],
        };
        let doc = draft_with(Some(render));
        let (_pdf, report) =
            ob_pdf::export_reader_pdf_report(&doc, ob_pdf::ExportPurpose::ForPrinting);
        assert_eq!(report.missing_pictures, 0, "the drawn line prints");
    }

    #[test]
    fn undrawn_music_prints_a_counted_placeholder() {
        let doc = draft_with(None);
        let (_pdf, report) =
            ob_pdf::export_reader_pdf_report(&doc, ob_pdf::ExportPurpose::ForPrinting);
        assert_eq!(
            report.missing_pictures, 1,
            "the empty music box is a counted placeholder, never silent"
        );
    }
}
