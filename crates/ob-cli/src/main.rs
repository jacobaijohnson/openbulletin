//! OpenBulletin CLI: the headless engine operations (new, export,
//! export-idml, export-sla, export-md, export-html, next-sunday,
//! info, fonts, trace, fingerprint, duplicates). The import doors live in the app.
//! If it cannot be done here, the GUI is hiding engine problems.

use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "openbulletin-cli", about = "OpenBulletin headless engine CLI")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Create a bulletin document from a preset and the church profile.
    New {
        #[arg(long, default_value = "letter-bifold")]
        preset: String,
        #[arg(long, default_value = "Our Church")]
        church: String,
        #[arg(long, default_value = "")]
        pastor: String,
        #[arg(long, default_value = "")]
        address: String,
        #[arg(long, default_value = "Sundays 9:30 AM")]
        times: String,
        #[arg(long, default_value = "")]
        website: String,
        /// Sunday date, YYYY-MM-DD. Defaults to today.
        #[arg(long)]
        date: Option<String>,
        #[arg(long, default_value = "Sunday")]
        sunday_name: String,
        #[arg(long)]
        out: PathBuf,
    },
    /// Export a PDF. Default: reader panels. --imposed: printer
    /// sheets. --large-print: the large-print copy (the same
    /// words at 17pt or bigger, one Letter page at a time).
    Export {
        input: PathBuf,
        #[arg(long)]
        imposed: bool,
        /// The large-print copy instead of the ordinary one.
        #[arg(long)]
        large_print: bool,
        /// What the copy is for: web, print, shop, or full.
        /// Defaults follow the door: print for imposed sheets
        /// and for large print, web for reader panels.
        #[arg(long, value_parser = ["web", "print", "shop", "full"])]
        purpose: Option<String>,
        #[arg(long)]
        out: PathBuf,
    },
    /// Save a copy for InDesign: an IDML package carrying the
    /// words, their boxes, and their fonts. Prints plain-words
    /// notes about anything the copy could not carry.
    ExportIdml {
        input: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    /// Save a copy for Scribus: a .sla file carrying the
    /// words, their boxes, and their fonts. Prints plain-words
    /// notes about anything the copy could not carry.
    ExportSla {
        input: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    /// Save the words as Markdown, for a website or
    /// newsletter: headings from the style names, bold and
    /// italic from the runs, field words filled in.
    ExportMd {
        input: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    /// Save the bulletin as one self-contained web page:
    /// the words in reading order, the pictures carried
    /// inside the file, the credits it owes at the end.
    /// A reading edition, made to be read and searched
    /// on a screen.
    ExportHtml {
        input: PathBuf,
        /// Where to write the page. Defaults to the
        /// bulletin's own name with "-web.html" on it,
        /// beside the bulletin.
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Clone a document for next Sunday (date +7, weekly content cleared).
    NextSunday {
        input: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    /// Print document facts.
    Info { input: PathBuf },
    /// Trace rescue: turn an old PDF bulletin into a draft whose
    /// pages carry the old pages as tracing pictures.
    Trace {
        /// The old bulletin PDF.
        input: PathBuf,
        /// Folder to write the draft and its picture assets into.
        #[arg(long)]
        out: PathBuf,
        /// Path to the ob-pdfium-helper binary (else $OB_PDFIUM_HELPER,
        /// else a sibling of this executable, else PATH).
        #[arg(long)]
        helper: Option<PathBuf>,
    },
    /// Font library: list what is installed, or add a font file.
    Fonts {
        #[command(subcommand)]
        action: FontsCmd,
    },
    /// Fingerprint one file: what kind it is, its version
    /// stamp, and the hashes the duplicates report uses.
    Fingerprint { file: PathBuf },
    /// The duplicates report for a fingerprint index:
    /// add files, then print every proposal, every short
    /// pair that needs its words compared by hand, and
    /// every stale entry waiting on a recompute.
    Duplicates {
        /// The index file (conventional home:
        /// <base>/OpenBulletin/fingerprints.json).
        #[arg(long)]
        index: PathBuf,
        /// Files to add to the index first. Files
        /// already in it are skipped, never duplicated.
        #[arg(long)]
        add: Vec<PathBuf>,
    },
}

#[derive(Subcommand)]
enum FontsCmd {
    /// List the fonts in the library (default folder unless --dir).
    List {
        #[arg(long)]
        dir: Option<PathBuf>,
    },
    /// Copy font files (.ttf/.otf, or a .zip of them) into the library.
    Add {
        #[arg(required = true)]
        files: Vec<PathBuf>,
        #[arg(long)]
        dir: Option<PathBuf>,
    },
    /// Install the Church Starter Pack (free metric twins and
    /// graceful faces) into the library.
    Pack {
        #[arg(long)]
        dir: Option<PathBuf>,
    },
    /// List the fonts installed on this computer (candidates to
    /// copy into the library with `fonts add`).
    System,
    /// Search the free-font catalog by name.
    Search { query: String },
    /// Download a free font family from the catalog into the library.
    Get {
        id: String,
        #[arg(long)]
        dir: Option<PathBuf>,
    },
}

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(cli) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

/// Whether two paths name the same file, the way the
/// filesystem sees them. The input resolves through
/// canonicalize (it exists: every export reads it); the
/// output may not exist yet, so it resolves through its
/// parent folder and its name. A `..` or a linked folder
/// cannot talk its way past this (round 2, R2-6).
fn same_file(a: &Path, b: &Path) -> bool {
    let resolve = |p: &Path| -> Option<PathBuf> {
        if let Ok(c) = std::fs::canonicalize(p) {
            return Some(c);
        }
        std::fs::canonicalize(p.parent()?)
            .ok()
            .zip(p.file_name())
            .map(|(c, name)| c.join(name))
    };
    match (resolve(a), resolve(b)) {
        (Some(x), Some(y)) => x == y,
        _ => false,
    }
}

/// The refusal every export door shares when its output
/// is its own input (round 2, R2-6). Checked before
/// anything is read or written.
fn refuse_replacing_itself(input: &Path, out: &Path) -> Result<(), String> {
    if same_file(input, out) {
        Err("the copy would replace the bulletin itself; pick a different name".to_string())
    } else {
        Ok(())
    }
}

/// Write through a temp file in the same folder, then
/// rename it over the destination, so a failed export
/// never leaves a half file where a good one stood
/// (round 2, R2-6; the same discipline as the document
/// save). A failed write removes the temp file.
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let tmp = match path.extension() {
        Some(e) => path.with_extension(format!("{}.tmp", e.to_string_lossy())),
        None => path.with_extension("tmp"),
    };
    std::fs::write(&tmp, bytes).map_err(|e| e.to_string())?;
    if let Err(e) = std::fs::rename(&tmp, path) {
        let _ = std::fs::remove_file(&tmp);
        return Err(e.to_string());
    }
    Ok(())
}

fn run(cli: Cli) -> Result<(), String> {
    match cli.cmd {
        Cmd::New {
            preset,
            church,
            pastor,
            address,
            times,
            website,
            date,
            sunday_name,
            out,
        } => {
            let preset = ob_layout::Preset::parse(&preset)
                .ok_or_else(|| format!("unknown preset {preset} (try letter-bifold, letter-trifold, letter-portrait, legal-bifold, tabloid-bifold, saddle-booklet)"))?;
            let date = match date {
                Some(d) => chrono::NaiveDate::parse_from_str(&d, "%Y-%m-%d").map_err(|_| {
                    format!(
                        "\"{d}\" does not read as a date; write it as the year, \
                             then the month, then the day, like 2026-10-18"
                    )
                })?,
                None => chrono::Local::now().date_naive(),
            };
            let profile = ob_model::ChurchProfile {
                name: church,
                address,
                service_times: times,
                website,
                giving_url: String::new(),
                pastor,
                expected_parts: Vec::new(),
                children_names: Default::default(),
                serving_lines: ob_model::default_serving_lines(),
                serving_aliases: Default::default(),
            };
            let doc = ob_layout::build_template(preset, profile, date, &sunday_name);
            ob_format::save(&doc, &out)?;
            println!(
                "wrote {} ({} panels, preset {})",
                out.display(),
                doc.panels.len(),
                preset.id()
            );
            Ok(())
        }
        Cmd::Export {
            input,
            imposed,
            large_print,
            purpose,
            out,
        } => {
            refuse_replacing_itself(&input, &out)?;
            let doc = ob_format::load(&input)?;
            let purpose = match purpose.as_deref() {
                Some(slug) => ob_pdf::ExportPurpose::from_slug(slug)
                    .ok_or_else(|| format!("unknown purpose: {slug}"))?,
                None if imposed || large_print => ob_pdf::ExportPurpose::ForPrinting,
                None => ob_pdf::ExportPurpose::ForEmailOrWeb,
            };
            let (bytes, report) = if large_print {
                ob_pdf::export_large_print_pdf_report(&doc, purpose)
            } else if imposed {
                ob_pdf::export_imposed_pdf_report(&doc, purpose)
            } else {
                ob_pdf::export_reader_pdf_report(&doc, purpose)
            };
            write_atomic(&out, &bytes)?;
            println!(
                "wrote {} ({} bytes, {}, {})",
                out.display(),
                bytes.len(),
                if large_print {
                    "large print: the same words, bigger"
                } else if imposed {
                    "imposed sheets"
                } else {
                    "reader panels"
                },
                purpose.words()
            );
            if report.spoken_only_left_out > 0 {
                println!(
                    "note:{}",
                    ob_model::spoken_only_left_out_sentence(report.spoken_only_left_out)
                );
            }
            Ok(())
        }
        Cmd::ExportIdml { input, out } => {
            refuse_replacing_itself(&input, &out)?;
            let doc = ob_format::load(&input)?;
            let (bytes, report) = ob_extract::idml_export::export_idml(&doc)?;
            write_atomic(&out, &bytes)?;
            println!(
                "wrote {} ({} bytes, IDML for InDesign: {} pages, {} text frames)",
                out.display(),
                bytes.len(),
                report.pages,
                report.text_frames
            );
            for n in &report.notes {
                println!("not carried: {n}");
            }
            Ok(())
        }
        Cmd::ExportSla { input, out } => {
            refuse_replacing_itself(&input, &out)?;
            let doc = ob_format::load(&input)?;
            let (bytes, report) = ob_extract::sla_export::export_sla(&doc)?;
            write_atomic(&out, &bytes)?;
            println!(
                "wrote {} ({} bytes, Scribus file: {} pages, {} text frames)",
                out.display(),
                bytes.len(),
                report.pages,
                report.text_frames
            );
            for n in &report.notes {
                println!("not carried: {n}");
            }
            Ok(())
        }
        Cmd::ExportMd { input, out } => {
            refuse_replacing_itself(&input, &out)?;
            let doc = ob_format::load(&input)?;
            let (text, spoken_left_out) = ob_model::markdown::to_markdown_report(&doc);
            write_atomic(&out, text.as_bytes())?;
            println!(
                "wrote {} ({} bytes, Markdown: the words, for a website or newsletter)",
                out.display(),
                text.len()
            );
            if spoken_left_out > 0 {
                println!(
                    "note:{}",
                    ob_model::spoken_only_left_out_sentence(spoken_left_out)
                );
            }
            Ok(())
        }
        Cmd::ExportHtml { input, out } => {
            let out = out.unwrap_or_else(|| {
                let stem = input
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| "bulletin".to_string());
                input.with_file_name(format!("{stem}-web.html"))
            });
            refuse_replacing_itself(&input, &out)?;
            let doc = ob_format::load(&input)?;
            let (bytes, report) = ob_extract::html_export::export_html(&doc)?;
            write_atomic(&out, &bytes)?;
            println!(
                "wrote {} ({} bytes, web page: the reading edition, {} pictures carried, {} credit lines)",
                out.display(),
                bytes.len(),
                report.pictures,
                report.credits
            );
            if report.missing_pictures == 1 {
                println!(
                    "note: 1 picture box had no picture to carry; the page says so where it belongs"
                );
            } else if report.missing_pictures > 1 {
                println!(
                    "note: {} picture boxes had no picture to carry; the page says so where they belong",
                    report.missing_pictures
                );
            }
            if report.spoken_only_left_out > 0 {
                println!(
                    "note:{}",
                    ob_model::spoken_only_left_out_sentence(report.spoken_only_left_out)
                );
            }
            if report.web_hidden_left_out > 0 {
                println!(
                    "note:{}",
                    ob_model::web_hidden_left_out_sentence(report.web_hidden_left_out)
                );
            }
            if report.children_left_out > 0 {
                println!(
                    "note:{}",
                    ob_extract::html_export::children_left_out_sentence(report.children_left_out)
                );
            }
            Ok(())
        }
        Cmd::NextSunday { input, out } => {
            let doc = ob_format::load(&input)?;
            let (next, report) = ob_layout::next_sunday_report(&doc)?;
            if let Some(refusal) = &report.seasonal_refusal {
                // Round 55, batch C, R54-RED-2: a seasonal
                // bulletin has no next week, so there is
                // nothing to write and no "wrote" line to
                // print. The old code saved the input
                // untouched and announced it as the new
                // Sunday. Instead the refusal + guidance
                // print on stdout, no output file is
                // written, and the command fails (exit 1,
                // the CLI's standing refusal exit, as for
                // a bad date or a copy over its own
                // input): a script must not take a
                // duplicate of the same week for next
                // week. The short error on stderr is the
                // script-facing signal; the stdout text
                // is the office-facing guidance.
                println!("{refusal}");
                return Err(
                    "next-sunday refused: the bulletin is a seasonal bulletin, so no new \
                     week was written"
                        .to_string(),
                );
            }
            ob_format::save(&next, &out)?;
            println!(
                "wrote {} for {} ({})",
                out.display(),
                next.fields
                    .get("occasion_date")
                    .cloned()
                    .unwrap_or_default(),
                next.fields
                    .get("occasion_date_long")
                    .cloned()
                    .unwrap_or_default()
            );
            if let Some(note) = &report.occasion_note {
                println!("note: {note}");
            }
            Ok(())
        }
        Cmd::Info { input } => {
            let doc = ob_format::load(&input)?;
            let sides = ob_impose::impose(&doc);
            println!("title: {}", doc.title);
            println!("preset: {}", doc.sheet.preset);
            println!("panels: {}", doc.panels.len());
            println!("stories: {}", doc.stories.len());
            println!("imposed sheet sides: {}", sides.len());
            println!(
                "sunday: {} ({})",
                doc.fields
                    .get("occasion_date_long")
                    .cloned()
                    .unwrap_or_default(),
                doc.fields.get("occasion_date").cloned().unwrap_or_default()
            );
            Ok(())
        }
        Cmd::Trace { input, out, helper } => {
            run_trace(&input, &out, helper.as_deref())?;
            Ok(())
        }
        Cmd::Fonts { action } => {
            match action {
                FontsCmd::List { dir } => {
                    let lib = ob_fonts::FontLibrary::open(
                        dir.unwrap_or_else(ob_fonts::default_library_dir),
                    )?;
                    let fonts = lib.scan();
                    if fonts.is_empty() {
                        println!("the font library is empty ({})", lib.dir().display());
                    }
                    for f in &fonts {
                        println!(
                            "{} | {} | {}",
                            f.family,
                            f.full_name,
                            f.rights.plain_words()
                        );
                    }
                }
                FontsCmd::Search { query } => {
                    let hits = ob_fonts::catalog::search(&query)?;
                    if hits.is_empty() {
                        println!("no free fonts match \"{query}\"");
                    }
                    for h in &hits {
                        println!("{} | {} | {} | {}", h.id, h.family, h.category, h.license);
                    }
                }
                FontsCmd::Get { id, dir } => {
                    let lib = ob_fonts::FontLibrary::open(
                        dir.unwrap_or_else(ob_fonts::default_library_dir),
                    )?;
                    let installed = ob_fonts::catalog::install(&lib, &id)?;
                    for f in &installed {
                        println!("installed {} ({})", f.full_name, f.file_name);
                    }
                    if let Some(first) = installed.first() {
                        println!("{}", first.rights.plain_words());
                    }
                }
                FontsCmd::Pack { dir } => {
                    let lib = ob_fonts::FontLibrary::open(
                        dir.unwrap_or_else(ob_fonts::default_library_dir),
                    )?;
                    let report = ob_fonts::catalog::install_pack(
                        &lib,
                        ob_fonts::catalog::CHURCH_STARTER_PACK,
                    );
                    for f in &report.installed {
                        println!("installed {} ({})", f.full_name, f.file_name);
                    }
                    for (id, why) in &report.failed {
                        println!("could not install {id}: {why}");
                    }
                    println!(
                        "starter pack: {} faces in, {} families failed",
                        report.installed.len(),
                        report.failed.len()
                    );
                }
                FontsCmd::System => {
                    let found = ob_fonts::system_fonts();
                    if found.is_empty() {
                        println!("no system fonts found on this computer");
                    }
                    for f in &found {
                        println!(
                            "{} | {} | {} | {}",
                            f.info.family,
                            f.info.full_name,
                            f.info.rights.plain_words(),
                            f.path.display()
                        );
                    }
                }
                FontsCmd::Add { files, dir } => {
                    let lib = ob_fonts::FontLibrary::open(
                        dir.unwrap_or_else(ob_fonts::default_library_dir),
                    )?;
                    // A bad file in the batch is skipped and
                    // named, never the end of the batch: the
                    // good files still land (round 2, R2-6;
                    // the zip arm has always worked this way).
                    let mut added = 0usize;
                    let mut skipped = 0usize;
                    for file in &files {
                        // The open-door gate (HF-2): a font file
                        // or pack is refused from its stated size
                        // before it is read whole, in the batch's
                        // own skipped-and-named style.
                        let bytes = match ob_extract::read_open_door_file(file) {
                            Ok(b) => b,
                            Err(e) => {
                                println!("skipped {}: {e}", file.display());
                                skipped += 1;
                                continue;
                            }
                        };
                        if file
                            .extension()
                            .map(|e| e.eq_ignore_ascii_case("zip"))
                            .unwrap_or(false)
                        {
                            for result in lib.add_zip_bytes(&bytes) {
                                match result {
                                    Ok(info) => {
                                        added += 1;
                                        println!("added {} ({})", info.full_name, info.file_name)
                                    }
                                    Err(e) => println!("skipped one in {}: {e}", file.display()),
                                }
                            }
                        } else {
                            // One result per font the file
                            // carries (a collection adds each
                            // face); a refusal names the file
                            // and counts as skipped, the same
                            // as any other bad file here.
                            for result in lib.add_path(file) {
                                match result {
                                    Ok(info) => {
                                        added += 1;
                                        println!("added {} ({})", info.full_name, info.file_name);
                                        println!("{}", info.rights.plain_words());
                                    }
                                    Err(e) => {
                                        println!("skipped {}: {e}", file.display());
                                        skipped += 1;
                                    }
                                }
                            }
                        }
                    }
                    if skipped > 0 {
                        println!("{added} added, {skipped} skipped");
                    }
                }
            }
            Ok(())
        }
        Cmd::Fingerprint { file } => {
            let bytes = ob_extract::read_open_door_file(&file)?;
            let summary = fingerprint_summary(&bytes, &file)?;
            println!("{summary}");
            Ok(())
        }
        Cmd::Duplicates { index, add } => {
            let mut idx = ob_fingerprint::matching::FingerprintIndex::load(&index)?;
            for file in &add {
                let bytes = ob_extract::read_open_door_file(file)?;
                if index_add_file(&mut idx, &bytes, file)? {
                    println!("added {}", file.display());
                } else {
                    println!("already in the index: {}", file.display());
                }
            }
            if !add.is_empty() {
                idx.save(&index)?;
            }
            println!("{}", duplicates_report(&idx));
            Ok(())
        }
    }
}

/// One file's fingerprint, routed the way the library
/// index routes it: pictures by magic bytes, music by
/// extension, anything else as readable text. A file
/// that is none of those fails loudly, never as a
/// silent wrong kind.
enum NewEntry {
    Image(ob_fingerprint::matching::ImageEntry),
    Music(ob_fingerprint::matching::MusicEntry),
    Text(ob_fingerprint::matching::TextEntry),
}

fn fingerprint_entry(bytes: &[u8], path: &Path) -> Result<NewEntry, String> {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "unnamed".to_string());
    let sha = ob_fingerprint::sha256_hex(bytes);
    if ob_image::sniff(bytes).decodable() {
        let decoded = ob_image::decode(bytes).map_err(|e| e.to_string())?;
        let fp = ob_fingerprint::image::fingerprint_image(&decoded);
        return Ok(NewEntry::Image(
            ob_fingerprint::matching::ImageEntry::from_engine(&sha, &name, &fp),
        ));
    }
    let is_music = path.extension().map(|e| {
        e.eq_ignore_ascii_case("musicxml")
            || e.eq_ignore_ascii_case("xml")
            || e.eq_ignore_ascii_case("mxl")
    });
    if is_music.unwrap_or(false) {
        let fp = ob_fingerprint::music::fingerprint_music(bytes)?;
        return Ok(NewEntry::Music(
            ob_fingerprint::matching::MusicEntry::from_engine(&sha, &name, &fp),
        ));
    }
    let text = String::from_utf8(bytes.to_vec())
        .map_err(|_| "that file is not a picture, a music file, or readable text".to_string())?;
    let fp = ob_fingerprint::text::fingerprint_text(&text);
    Ok(NewEntry::Text(
        ob_fingerprint::matching::TextEntry::from_engine(&sha, &name, &fp),
    ))
}

/// The `fingerprint` command's one-line summary.
fn fingerprint_summary(bytes: &[u8], path: &Path) -> Result<String, String> {
    match fingerprint_entry(bytes, path)? {
        NewEntry::Image(e) => {
            let low = if e.low_information {
                " (simple drawing on a plain background)"
            } else {
                ""
            };
            Ok(format!(
                "picture | {} | picture hash {:016x} | gradient hash {:016x}{low}",
                e.fp_version, e.phash, e.dhash
            ))
        }
        NewEntry::Music(e) => Ok(format!(
            "music | {} | notes {} | pitches {} | melody {} | Plaine & Easie: {}",
            e.fp_version,
            &e.canon_hash[..8],
            &e.midi_hash[..8],
            &e.melody_hash[..8],
            e.pae
        )),
        NewEntry::Text(e) => Ok(format!(
            "text | {} | words {} | word-groups {}",
            e.fp_version,
            e.token_count,
            e.shingles.len()
        )),
    }
}

/// Add one file to the index. Returns false when the
/// file's bytes are already in it (any kind): the same
/// file is never entered twice.
fn index_add_file(
    index: &mut ob_fingerprint::matching::FingerprintIndex,
    bytes: &[u8],
    path: &Path,
) -> Result<bool, String> {
    let sha = ob_fingerprint::sha256_hex(bytes);
    let known = index.images.iter().any(|e| e.sha256 == sha)
        || index.music.iter().any(|e| e.sha256 == sha)
        || index.texts.iter().any(|e| e.sha256 == sha);
    if known {
        return Ok(false);
    }
    match fingerprint_entry(bytes, path)? {
        NewEntry::Image(e) => index.images.push(e),
        NewEntry::Music(e) => index.music.push(e),
        NewEntry::Text(e) => index.texts.push(e),
    }
    Ok(true)
}

/// The `duplicates` report. A full audit, all pairs,
/// by design: the report's job is to miss nothing.
/// The interactive query path is different — it scores
/// only the music candidate gate's buckets, never
/// all-pairs, because a query must answer now. Pairs
/// the index has already dismissed never appear.
fn duplicates_report(index: &ob_fingerprint::matching::FingerprintIndex) -> String {
    use ob_fingerprint::matching::{match_image, match_music, match_text};
    use ob_fingerprint::text::TextDecision;
    let mut lines: Vec<String> = Vec::new();
    lines.push(format!(
        "{} pictures, {} tunes, {} announcements in the index; {} pairs marked not duplicates",
        index.images.len(),
        index.music.len(),
        index.texts.len(),
        index.dismissals.len()
    ));
    let mut proposals = 0usize;
    for i in 0..index.images.len() {
        for j in (i + 1)..index.images.len() {
            let a = &index.images[i];
            let b = &index.images[j];
            if index.is_dismissed(&a.sha256, &b.sha256) {
                continue;
            }
            let (Some(fa), Some(fb)) = (a.to_engine(), b.to_engine()) else {
                continue;
            };
            if let Some(m) = match_image(&fa, &fb, &a.sha256, &b.sha256) {
                proposals += 1;
                lines.push(format!(
                    "{} <-> {}: {} — {}",
                    a.name, b.name, m.title, m.reason
                ));
            }
        }
    }
    for i in 0..index.music.len() {
        for j in (i + 1)..index.music.len() {
            let a = &index.music[i];
            let b = &index.music[j];
            if index.is_dismissed(&a.sha256, &b.sha256) {
                continue;
            }
            let (Some(fa), Some(fb)) = (a.to_engine(), b.to_engine()) else {
                continue;
            };
            if let Some(m) = match_music(&fa, &fb) {
                proposals += 1;
                lines.push(format!(
                    "{} <-> {}: {} — {}",
                    a.name, b.name, m.title, m.reason
                ));
            }
        }
    }
    let (freq, total) = index.doc_freq();
    let df = Some((&freq, total));
    let mut short_pairs = 0usize;
    for i in 0..index.texts.len() {
        for j in (i + 1)..index.texts.len() {
            let a = &index.texts[i];
            let b = &index.texts[j];
            if index.is_dismissed(&a.sha256, &b.sha256) {
                continue;
            }
            let (Some(fa), Some(fb)) = (a.to_engine(), b.to_engine()) else {
                continue;
            };
            match ob_fingerprint::text::text_band(&fa, &fb, df) {
                TextDecision::NeedsShortTexts => {
                    short_pairs += 1;
                    lines.push(format!(
                        "{} <-> {}: both are short announcements — open both and compare the words",
                        a.name, b.name
                    ));
                }
                TextDecision::NoMatch => {}
                _ => {
                    if let Some(m) = match_text(&fa, &fb, df) {
                        proposals += 1;
                        lines.push(format!(
                            "{} <-> {}: {} — {}",
                            a.name, b.name, m.title, m.reason
                        ));
                    }
                }
            }
        }
    }
    let stale = index.stale_entries();
    if !stale.is_empty() {
        lines.push("waiting on a recompute from the original:".to_string());
        for (kind, _sha, name) in stale {
            lines.push(format!("  {kind}: {name}"));
        }
    }
    if proposals == 0 && short_pairs == 0 && index.stale_entries().is_empty() {
        lines.push("no duplicates found".to_string());
    }
    lines.join("\n")
}

/// The trace rescue: render the old PDF's pages to pictures with
/// the PDFium helper, then build a draft bulletin whose panels
/// carry those pictures as guides (docs/EXTRACT.md). The admin
/// opens the draft and types the new bulletin over the pictures;
/// guides never print.
fn run_trace(input: &Path, out: &Path, helper: Option<&Path>) -> Result<(), String> {
    // The open-door gate (HF-2): the helper gates the PDF
    // in its own process too, but the refusal belongs at
    // this door, before any helper starts.
    ob_extract::check_open_door_size(input)?;
    let helper = resolve_helper(helper)?;
    let assets_dir = out.join("assets");
    std::fs::create_dir_all(&assets_dir)
        .map_err(|e| format!("cannot create {}: {e}", assets_dir.display()))?;
    // The helper runs through the shared runner: a
    // stuck helper once hung trace forever (audit M9,
    // 2026-10-16), and its stderr is quoted trimmed,
    // one line, like everywhere else.
    let run = |args: &[&str]| -> Result<Vec<u8>, String> {
        let mut cmd = std::process::Command::new(&helper);
        cmd.args(args);
        let output = match ob_extract::runner::run_with_timeout(
            &mut cmd,
            None,
            ob_extract::runner::HELPER_TIMEOUT,
        ) {
            Ok(o) => o,
            Err(ob_extract::runner::RunFault::Spawn(e)) => {
                return Err(format!(
                    "cannot run the PDF helper ({}): {e}",
                    helper.display()
                ));
            }
            Err(ob_extract::runner::RunFault::TimedOut) => {
                return Err(
                    "the PDF helper did not answer in two minutes, so the trace was stopped"
                        .to_string(),
                );
            }
            Err(ob_extract::runner::RunFault::TooMuchOutput) => {
                return Err(
                    "the PDF helper answered with more than we can hold, so the trace was stopped"
                        .to_string(),
                );
            }
            Err(ob_extract::runner::RunFault::Wait(e)) => {
                return Err(format!(
                    "something went wrong while the PDF helper was working: {e}"
                ));
            }
        };
        if !output.status.success() {
            let said = ob_extract::runner::stderr_for_user(&output.stderr);
            return Err(if said.is_empty() {
                "the PDF helper failed".to_string()
            } else {
                format!("the PDF helper failed: {said}")
            });
        }
        Ok(output.stdout)
    };
    let input_str = input.to_string_lossy().to_string();
    run(&["render", &input_str, &assets_dir.to_string_lossy()])?;
    let dump_bytes = run(&[&input_str])?;
    let dump_text =
        String::from_utf8(dump_bytes).map_err(|e| format!("the PDF helper answered oddly: {e}"))?;
    // The typed parse checks the dump's schema version
    // (audit M10): a helper answering in a format from
    // the future is refused in words, not read as data.
    // R46-G: run_trace's own signature stays on
    // String for this slice, so the typed from_json
    // error crosses through the From impl (`?`
    // position: the conversion rides the `?`).
    let dump = ob_extract::ExtractionDump::from_json(&dump_text).map_err(String::from)?;
    if dump.pages.is_empty() {
        return Err("that PDF has no pages we can read".into());
    }
    let to_mpt = |v: f64| (v * 1000.0).round() as ob_model::Mpt;
    let page_sizes: Vec<ob_model::Size> = dump
        .pages
        .iter()
        .map(|pg| ob_model::Size {
            w: to_mpt(pg.width_pt as f64),
            h: to_mpt(pg.height_pt as f64),
        })
        .collect();
    // Booklet when every page is the same half-letter size and the
    // count is a multiple of four; otherwise plain pages.
    let first = page_sizes[0];
    let is_booklet = page_sizes.iter().all(|s| *s == first)
        && page_sizes.len() % 4 == 0
        && (first.w - 396_000).abs() < 2_000
        && (first.h - 612_000).abs() < 2_000;
    let (_preset_id, sheet) = if is_booklet {
        (
            "saddle-booklet",
            ob_model::SheetSpec {
                preset: "saddle-booklet".into(),
                size: ob_model::Size::new(792_000, 612_000),
            },
        )
    } else {
        (
            "letter-portrait",
            ob_model::SheetSpec {
                preset: "letter-portrait".into(),
                size: first,
            },
        )
    };
    let shape_name = sheet.preset.clone();
    let mut panels = Vec::new();
    let mut stories = Vec::new();
    for (i, size) in page_sizes.iter().enumerate() {
        let sid = format!("story-{i}");
        stories.push(ob_model::Story {
            id: sid.clone(),
            paragraphs: vec![ob_model::Paragraph::new(
                "body",
                "[Type this page over the picture of the old one.]",
            )],
            keep_through: None,
        });
        let m = ob_model::inch(0.45);
        panels.push(ob_model::Panel {
            id: format!("panel-{i}"),
            name: format!("Page {}", i + 1),
            size: *size,
            frames: vec![ob_model::Frame {
                id: format!("frame-{i}"),
                name: format!("Page {}", i + 1),
                kind: ob_model::FrameKind::Text,
                rect: ob_model::Rect::new(
                    m,
                    ob_model::inch(0.4),
                    size.w - 2 * m,
                    size.h - ob_model::inch(0.8),
                ),
                story_id: Some(sid),
                role: ob_model::ContentRole::Weekly,
                picture: None,
                music: None,
            }],
            background_image: Some(format!("assets/page-{:02}.png", i + 1)),
            guides: Vec::new(),
        });
    }
    let stem = input
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "bulletin".into());
    let doc = ob_model::Document {
        format_version: ob_model::FORMAT_VERSION,
        title: format!("Rebuilt from {stem}"),
        profile: ob_model::ChurchProfile::default(),
        fields: Default::default(),
        styles: ob_model::default_styles(),
        stories,
        panels,
        sheet,
        no_announcements: false,
        assets: Vec::new(),
        asset_bytes: Default::default(),
        occasion_time: None,
        wanted_fonts: Vec::new(),
        participants: Default::default(),
        inserts: Default::default(),
        applied_contributions: Vec::new(),
    };
    let out_file = out.join(format!("{stem}-traced.obd"));
    ob_format::save(&doc, &out_file)?;
    println!(
        "traced {} pages into {} (shape: {})",
        page_sizes.len(),
        out_file.display(),
        shape_name
    );
    println!("open the draft and type each page over its picture; the pictures never print");
    Ok(())
}

/// Find the PDFium helper binary: explicit flag, environment,
/// beside this executable, then PATH.
fn resolve_helper(explicit: Option<&Path>) -> Result<PathBuf, String> {
    Ok(ob_extract::helper::resolve_helper(explicit))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_routes_text_by_content() {
        let entry = fingerprint_entry(b"lord hear our prayer", Path::new("notes.txt"))
            .expect("text routes");
        assert!(matches!(entry, NewEntry::Text(_)));
        let summary =
            fingerprint_summary(b"lord hear our prayer", Path::new("notes.txt")).expect("summary");
        assert!(summary.starts_with("text | text-fp-v1 | words 4"));
    }

    #[test]
    fn fingerprint_routes_music_and_fails_loud_on_garbage() {
        let score = "<score-partwise><part-list><score-part id=\"P1\">\
            <part-name>Piano</part-name></score-part></part-list><part id=\"P1\">\
            <measure><attributes><divisions>1</divisions></attributes>\
            <note><pitch><step>C</step><octave>4</octave></pitch>\
            <duration>1</duration><voice>1</voice><type>quarter</type></note>\
            </measure></part></score-partwise>";
        let entry =
            fingerprint_entry(score.as_bytes(), Path::new("tune.musicxml")).expect("music routes");
        assert!(matches!(entry, NewEntry::Music(_)));
        let bad = fingerprint_entry(b"not xml at all", Path::new("bad.musicxml"));
        assert!(bad.is_err());
        let unknown = fingerprint_entry(&[0xff, 0xfe, 0x00], Path::new("blob.bin"));
        assert!(unknown.is_err());
    }

    #[test]
    fn duplicates_report_proposes_and_honours_dismissals() {
        use ob_fingerprint::matching::{FingerprintIndex, ImageEntry};
        use ob_fingerprint::{image::ImageFingerprint, IMAGE_FP_VERSION};
        let mut index = FingerprintIndex::default();
        let fp = ImageFingerprint {
            fp_version: IMAGE_FP_VERSION,
            phash: 0,
            dhash: 0,
            hist: [0u32; 64],
            width: 0,
            height: 0,
            low_information: false,
        };
        index
            .images
            .push(ImageEntry::from_engine("sha1", "cross.png", &fp));
        index
            .images
            .push(ImageEntry::from_engine("sha2", "cross-copy.png", &fp));
        let report = duplicates_report(&index);
        assert!(report.contains("cross.png <-> cross-copy.png"));
        assert!(report.contains("Probably the same"));
        index.dismiss("sha1", "sha2");
        let report = duplicates_report(&index);
        assert!(!report.contains("cross-copy.png"));
        assert!(report.contains("1 pairs marked not duplicates"));
    }
}
