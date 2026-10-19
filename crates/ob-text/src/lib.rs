//! Text: real shaping, layout, and threading for OpenBulletin.
//!
//! The engine is [`shaping::Shaper`], built on parley: real font
//! advances, real line breaks, real overflow, on registered fonts
//! only. [`text_layout`] turns shaped paragraphs into the display
//! lines the canvas and the PDF writer work from, and
//! [`threading`] flows stories through their frames and reports
//! what does not fit. Anything that decides whether text fits a
//! frame comes through here, never through an estimate.

pub mod shaping;
pub mod text_layout;
pub mod threading;
