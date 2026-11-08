//! OpenBulletin CLI: the headless engine operations (new, export,
//! export-idml, export-sla, export-md, export-html, next-sunday,
//! info, fonts, trace, fingerprint, duplicates, snippets). The import doors live in the app.
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
    /// Reusable snippets: list what is kept, keep a
    /// file into the library, or check the dedupe tier
    /// a file would hit without storing anything.
    Snippets {
        #[command(subcommand)]
        action: SnippetsCmd,
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

#[derive(Subcommand)]
enum SnippetsCmd {
    /// List the kept snippets: name + kind + credit
    /// line (the default store unless --store).
    List {
        #[arg(long)]
        store: Option<PathBuf>,
    },
    /// Keep a file into the snippet store, through
    /// the same decide + store path as the review
    /// window's tick. A near-duplicate is reported
    /// and nothing is stored.
    Keep {
        file: PathBuf,
        /// What the file is: picture, song, or text.
        #[arg(long, value_parser = ["picture", "song", "text"])]
        kind: String,
        /// The plain name the office recognises.
        /// Defaults to the file name.
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        store: Option<PathBuf>,
    },
    /// Dedupe check only: report the tier the file
    /// would hit (Same file / Same content /
    /// Probably the same — please check / no match)
    /// with reasons, and store nothing.
    Check {
        file: PathBuf,
        #[arg(long)]
        store: Option<PathBuf>,
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
        Cmd::Snippets { action } => {
            match action {
                SnippetsCmd::List { store } => {
                    let (store_path, _, _) = snippet_paths(store);
                    let store = ob_snippets::SnippetStore::load(&store_path)?;
                    for line in snippets_list_lines(&store, &store_path) {
                        println!("{line}");
                    }
                }
                SnippetsCmd::Keep {
                    file,
                    kind,
                    name,
                    store,
                } => {
                    let (store_path, bytes_dir, index_path) = snippet_paths(store);
                    match snippets_keep(
                        &store_path,
                        &bytes_dir,
                        &index_path,
                        &file,
                        &kind,
                        name.as_deref(),
                    )? {
                        SnippetKeepOutcome::Kept(line) => println!("{line}"),
                        SnippetKeepOutcome::AlreadyKept(line) => println!("{line}"),
                        SnippetKeepOutcome::Ask(lines) => {
                            for line in &lines {
                                println!("{line}");
                            }
                            return Err(format!(
                                "not stored: {} looks like a snippet already in the library — \
                                 please check before keeping it",
                                file.display()
                            ));
                        }
                    }
                }
                SnippetsCmd::Check { file, store } => {
                    let (store_path, bytes_dir, index_path) = snippet_paths(store);
                    for line in snippets_check(&store_path, &bytes_dir, &index_path, &file)? {
                        println!("{line}");
                    }
                }
            }
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

/// The snippet store's three paths: the store file,
/// the content-bytes folder, and the fingerprint index
/// beside it. An explicit --store puts the bytes folder
/// and the index beside that file; otherwise all three
/// are the computed library defaults, the fonts
/// pattern (there is no default path for
/// fingerprints.json — the index default is computed
/// here from the same library home).
fn snippet_paths(store: Option<PathBuf>) -> (PathBuf, PathBuf, PathBuf) {
    match store {
        Some(path) => {
            let bytes_dir = path
                .parent()
                .map(|parent| parent.join("snippets"))
                .unwrap_or_else(|| PathBuf::from("snippets"));
            let index_path = path
                .parent()
                .map(|parent| parent.join("fingerprints.json"))
                .unwrap_or_else(|| PathBuf::from("fingerprints.json"));
            (path, bytes_dir, index_path)
        }
        None => (
            ob_snippets::default_store_path(),
            ob_snippets::default_bytes_dir(),
            ob_fingerprint::matching::default_index_path(),
        ),
    }
}

/// The kind word the office uses, for the list line.
fn snippet_kind_word(kind: ob_snippets::SnippetKind) -> &'static str {
    match kind {
        ob_snippets::SnippetKind::Picture => "picture",
        ob_snippets::SnippetKind::Music => "song",
        ob_snippets::SnippetKind::Text => "text",
    }
}

/// The --kind word back to the store kind.
fn snippet_kind_from_word(word: &str) -> ob_snippets::SnippetKind {
    match word {
        "picture" => ob_snippets::SnippetKind::Picture,
        "song" => ob_snippets::SnippetKind::Music,
        _ => ob_snippets::SnippetKind::Text,
    }
}

/// The credit line the list shows under a snippet's
/// name: the record's own credit words when it has
/// them, else title and creator, else the file it was
/// kept from. Mirrors the picker's line in the app.
fn snippet_credit_line(record: &ob_snippets::SnippetRecord) -> String {
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

/// The list lines for a loaded store: one per snippet,
/// name + kind + credit line; one line saying so when
/// the library is empty.
fn snippets_list_lines(store: &ob_snippets::SnippetStore, store_path: &Path) -> Vec<String> {
    if store.snippets.is_empty() {
        return vec![format!(
            "the snippet library is empty ({})",
            store_path.display()
        )];
    }
    store
        .snippets
        .iter()
        .map(|record| {
            format!(
                "{} | {} | {}",
                record.name,
                snippet_kind_word(record.kind),
                snippet_credit_line(record)
            )
        })
        .collect()
}

/// Build the keep input for a file the CLI keeps or
/// checks: the file name is the source and the default
/// name, the kept date is today, asserted_by is empty,
/// and the rights start from the source file alone
/// (a bare file carries no credit of its own).
fn snippet_keep_input(
    bytes: Vec<u8>,
    kind: ob_snippets::SnippetKind,
    name: &str,
    source: &str,
) -> ob_snippets::KeepInput {
    let media_type = match kind {
        ob_snippets::SnippetKind::Picture => ob_image::sniff(&bytes).media_type().to_string(),
        ob_snippets::SnippetKind::Music => "application/vnd.recordare.musicxml+xml".to_string(),
        ob_snippets::SnippetKind::Text => "text/plain".to_string(),
    };
    let today = chrono::Local::now().date_naive();
    use chrono::Datelike;
    ob_snippets::KeepInput {
        content: bytes,
        kind,
        name: name.to_string(),
        source: source.to_string(),
        kept_at: format!(
            "{:04}-{:02}-{:02}",
            today.year(),
            today.month(),
            today.day()
        ),
        asserted_by: String::new(),
        media_type,
        rights: ob_model::RightsRecord {
            source: source.to_string(),
            ..Default::default()
        },
        spoken_only: false,
        web_hidden: false,
    }
}

/// What a CLI keep decided. The Ask carries its
/// proposal lines so the caller can print them before
/// refusing: a NeedsCheck proposal is reported, never
/// silently stored or skipped.
#[derive(Debug)]
enum SnippetKeepOutcome {
    Kept(String),
    AlreadyKept(String),
    Ask(Vec<String>),
}

/// Keep one file into the store, through the same
/// `decide_keep` + `store_snippet` path as the review
/// window's tick (same dedupe, same Ask behaviour as
/// the CLI can express). A stored dismissal is
/// honoured by the decision itself. Loud errors, never
/// silent: a missing file, a damaged store, an
/// undecodable picture, an unparsable song, non-UTF-8
/// text.
fn snippets_keep(
    store_path: &Path,
    bytes_dir: &Path,
    index_path: &Path,
    file: &Path,
    kind_word: &str,
    name: Option<&str>,
) -> Result<SnippetKeepOutcome, String> {
    let bytes = ob_extract::read_open_door_file(file)?;
    let file_name = file
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "unnamed".to_string());
    let name = match name {
        Some(n) if !n.trim().is_empty() => n.to_string(),
        _ => file_name.clone(),
    };
    let kind = snippet_kind_from_word(kind_word);
    let input = snippet_keep_input(bytes, kind, &name, &file_name);
    let mut store = ob_snippets::SnippetStore::load(store_path)?;
    let mut index = ob_fingerprint::matching::FingerprintIndex::load(index_path)?;
    let content = input.content.clone();
    match ob_snippets::decide_keep(&input, &store, &index, bytes_dir)? {
        ob_snippets::KeepDecision::AlreadyKept { id } => {
            let kept_name = store
                .get(&id)
                .map(|record| record.name.clone())
                .unwrap_or_else(|| id.clone());
            Ok(SnippetKeepOutcome::AlreadyKept(format!(
                "already kept as {kept_name} ({})",
                snippet_kind_word(kind)
            )))
        }
        ob_snippets::KeepDecision::ReadyToStore { record } => {
            let kept_name = record.name.clone();
            ob_snippets::store_snippet(
                &mut store, &mut index, record, &content, bytes_dir, store_path, index_path,
            )?;
            Ok(SnippetKeepOutcome::Kept(format!(
                "kept {kept_name} ({})",
                snippet_kind_word(kind)
            )))
        }
        ob_snippets::KeepDecision::Ask { proposals, .. } => {
            let mut lines = vec!["Probably the same — please check:".to_string()];
            for (proposal, name) in &proposals {
                lines.push(format!(
                    "  {name}: {} — {}",
                    proposal.title, proposal.reason
                ));
            }
            Ok(SnippetKeepOutcome::Ask(lines))
        }
    }
}

/// Route a check file by content, the way
/// `fingerprint_entry` routes it: pictures by magic
/// bytes, music by extension, anything else as
/// readable text. Garbage fails loudly, never as a
/// silent wrong kind.
fn route_snippet_kind(bytes: &[u8], path: &Path) -> Result<ob_snippets::SnippetKind, String> {
    if ob_image::sniff(bytes).decodable() {
        return Ok(ob_snippets::SnippetKind::Picture);
    }
    let is_music = path.extension().map(|e| {
        e.eq_ignore_ascii_case("musicxml")
            || e.eq_ignore_ascii_case("xml")
            || e.eq_ignore_ascii_case("mxl")
    });
    if is_music.unwrap_or(false) {
        return Ok(ob_snippets::SnippetKind::Music);
    }
    if std::str::from_utf8(bytes).is_ok() {
        return Ok(ob_snippets::SnippetKind::Text);
    }
    Err("that file is not a picture, a music file, or readable text".to_string())
}

/// Dedupe check only: report the tier the file would
/// hit, with reasons, and store nothing. Same file
/// (identical bytes already kept) and Same content
/// (the same content under another name) are told
/// apart by the stored bytes; a NeedsCheck proposal
/// prints its reason; no match says so plainly.
fn snippets_check(
    store_path: &Path,
    bytes_dir: &Path,
    index_path: &Path,
    file: &Path,
) -> Result<Vec<String>, String> {
    let bytes = ob_extract::read_open_door_file(file)?;
    let file_name = file
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "unnamed".to_string());
    let kind = route_snippet_kind(&bytes, file)?;
    let sha = ob_fingerprint::sha256_hex(&bytes);
    let input = snippet_keep_input(bytes, kind, &file_name, &file_name);
    let store = ob_snippets::SnippetStore::load(store_path)?;
    let index = ob_fingerprint::matching::FingerprintIndex::load(index_path)?;
    match ob_snippets::decide_keep(&input, &store, &index, bytes_dir)? {
        ob_snippets::KeepDecision::AlreadyKept { id } => {
            let kept_name = store
                .get(&id)
                .map(|record| record.name.clone())
                .unwrap_or_else(|| id.clone());
            if store.contains(&sha) {
                Ok(vec![format!("Same file: already kept as {kept_name}")])
            } else {
                Ok(vec![format!(
                    "Same content: matches the kept snippet {kept_name}"
                )])
            }
        }
        ob_snippets::KeepDecision::ReadyToStore { .. } => Ok(vec![
            "no match: nothing in the library matches this file".to_string(),
        ]),
        ob_snippets::KeepDecision::Ask { proposals, .. } => {
            let mut lines = vec!["Probably the same — please check:".to_string()];
            for (proposal, name) in &proposals {
                lines.push(format!(
                    "  {name}: {} — {}",
                    proposal.title, proposal.reason
                ));
            }
            Ok(lines)
        }
    }
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

    fn snippets_test_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(name);
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn snippets_paths(dir: &Path) -> (PathBuf, PathBuf, PathBuf) {
        (
            dir.join("snippets.json"),
            dir.join("snippets"),
            dir.join("fingerprints.json"),
        )
    }

    /// A PNG built from a pixel function, so the tests
    /// need no fixture files.
    fn snippets_png_bytes(w: u32, h: u32, pixel: impl Fn(u32, u32) -> (u8, u8, u8)) -> Vec<u8> {
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

    /// A busy 32x32 grey picture: plenty of hash signal,
    /// so a small change stays inside the bands.
    fn snippets_busy_picture() -> Vec<u8> {
        snippets_png_bytes(32, 32, |x, y| {
            let v = ((x * 7 + y * 13 + x * y * 3) % 256) as u8;
            (v, v, v)
        })
    }

    /// The same picture with a 6x6 block brightened: a
    /// near-duplicate, not the same bytes.
    fn snippets_busy_picture_near() -> Vec<u8> {
        snippets_png_bytes(32, 32, |x, y| {
            let mut v = ((x * 7 + y * 13 + x * y * 3) % 256) as u8;
            if (10..16).contains(&x) && (10..16).contains(&y) {
                v = v.saturating_add(40);
            }
            (v, v, v)
        })
    }

    fn snippets_music_note(step: &str, alter: &str, octave: u8, dur: u32, ntype: &str) -> String {
        let alter_xml = if alter.is_empty() {
            String::new()
        } else {
            format!("<alter>{alter}</alter>")
        };
        format!(
            "<note><pitch><step>{step}</step>{alter_xml}<octave>{octave}</octave></pitch>\
             <duration>{dur}</duration><type>{ntype}</type></note>"
        )
    }

    fn snippets_scale_c_xml() -> String {
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
            notes.push_str(&snippets_music_note(s, "", o, 4, "quarter"));
        }
        format!(
            r#"<?xml version="1.0"?><score-partwise>
            <part-list><score-part id="P1"><part-name>V</part-name></score-part></part-list>
            <part id="P1"><measure><attributes><divisions>4</divisions>
            <key><fifths>0</fifths><mode>major</mode></key></attributes>
            {notes}
            </measure></part></score-partwise>"#
        )
    }

    /// The same scale with divisions rescaled 4 -> 480:
    /// different bytes, same canonical content.
    fn snippets_scale_c_rescaled_xml() -> String {
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
                 <duration>480</duration><type>quarter</type></note>"
            ));
        }
        format!(
            r#"<?xml version="1.0"?><score-partwise>
            <part-list><score-part id="P1"><part-name>V</part-name></score-part></part-list>
            <part id="P1"><measure><attributes><divisions>480</divisions></attributes>
            {notes}
            </measure></part></score-partwise>"#
        )
    }

    #[test]
    fn snippets_keep_stores_and_lists() {
        let dir = snippets_test_dir("ob-cli-snippets-list-test");
        let (store_path, bytes_dir, index_path) = snippets_paths(&dir);
        std::fs::create_dir_all(&dir).expect("mkdir");
        let file = dir.join("welcome.txt");
        let content = b"Lord, in your mercy hear our prayer";
        std::fs::write(&file, content).expect("write text");
        let kept = snippets_keep(
            &store_path,
            &bytes_dir,
            &index_path,
            &file,
            "text",
            Some("Welcome"),
        )
        .expect("keep works");
        assert!(matches!(kept, SnippetKeepOutcome::Kept(_)));
        let store = ob_snippets::SnippetStore::load(&store_path).expect("store reloads");
        assert_eq!(store.snippets.len(), 1);
        assert_eq!(store.snippets[0].name, "Welcome");
        assert_eq!(store.snippets[0].source, "welcome.txt");
        let bytes_file = bytes_dir.join(&store.snippets[0].id);
        assert_eq!(std::fs::read(&bytes_file).expect("bytes written"), content);
        let lines = snippets_list_lines(&store, &store_path);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("Welcome"), "line: {}", lines[0]);
        assert!(lines[0].contains("text"), "line: {}", lines[0]);
        assert!(lines[0].contains("From welcome.txt"), "line: {}", lines[0]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn snippets_keep_same_bytes_twice_stores_one() {
        let dir = snippets_test_dir("ob-cli-snippets-same-bytes-test");
        let (store_path, bytes_dir, index_path) = snippets_paths(&dir);
        std::fs::create_dir_all(&dir).expect("mkdir");
        let file = dir.join("busy.png");
        std::fs::write(&file, snippets_busy_picture()).expect("write picture");
        let first = snippets_keep(&store_path, &bytes_dir, &index_path, &file, "picture", None)
            .expect("first keep works");
        assert!(matches!(first, SnippetKeepOutcome::Kept(_)));
        let second = snippets_keep(
            &store_path,
            &bytes_dir,
            &index_path,
            &file,
            "picture",
            Some("busy-again"),
        )
        .expect("second keep works");
        assert!(
            matches!(second, SnippetKeepOutcome::AlreadyKept(_)),
            "second keep of the same bytes must report already kept"
        );
        let store = ob_snippets::SnippetStore::load(&store_path).expect("store reloads");
        assert_eq!(store.snippets.len(), 1);
        assert_eq!(store.snippets[0].name, "busy.png");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn snippets_keep_retyped_text_and_rescaled_music_dedupe() {
        let dir = snippets_test_dir("ob-cli-snippets-same-content-test");
        let (store_path, bytes_dir, index_path) = snippets_paths(&dir);
        std::fs::create_dir_all(&dir).expect("mkdir");
        let text = dir.join("prayer.txt");
        std::fs::write(&text, b"Lord, in your mercy hear our prayer").expect("write text");
        let first = snippets_keep(&store_path, &bytes_dir, &index_path, &text, "text", None)
            .expect("keep works");
        assert!(matches!(first, SnippetKeepOutcome::Kept(_)));
        let retyped = dir.join("prayer-copy.txt");
        std::fs::write(
            &retyped,
            "LORD,  in Your Mercy — hear our prayer!".as_bytes(),
        )
        .expect("write retype");
        let second = snippets_keep(&store_path, &bytes_dir, &index_path, &retyped, "text", None)
            .expect("retyped keep works");
        assert!(
            matches!(second, SnippetKeepOutcome::AlreadyKept(_)),
            "retyped text must group on norm_hash"
        );
        let tune = dir.join("scale.musicxml");
        std::fs::write(&tune, snippets_scale_c_xml()).expect("write tune");
        let kept = snippets_keep(&store_path, &bytes_dir, &index_path, &tune, "song", None)
            .expect("song keep works");
        assert!(matches!(kept, SnippetKeepOutcome::Kept(_)));
        let rescaled = dir.join("scale-copy.musicxml");
        std::fs::write(&rescaled, snippets_scale_c_rescaled_xml()).expect("write rescaled");
        let again = snippets_keep(
            &store_path,
            &bytes_dir,
            &index_path,
            &rescaled,
            "song",
            None,
        )
        .expect("rescaled keep works");
        assert!(
            matches!(again, SnippetKeepOutcome::AlreadyKept(_)),
            "rescaled tune must group on canon_hash"
        );
        let store = ob_snippets::SnippetStore::load(&store_path).expect("store reloads");
        assert_eq!(store.snippets.len(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn snippets_check_reports_each_tier_without_storing() {
        let dir = snippets_test_dir("ob-cli-snippets-check-test");
        let (store_path, bytes_dir, index_path) = snippets_paths(&dir);
        std::fs::create_dir_all(&dir).expect("mkdir");
        let long = dir.join("prayer-long.txt");
        let long_text =
            "Lord, in your mercy hear our prayer for all who are sick and in need today";
        std::fs::write(&long, long_text).expect("write long text");
        snippets_keep(
            &store_path,
            &bytes_dir,
            &index_path,
            &long,
            "text",
            Some("prayer-long"),
        )
        .expect("keep works");
        let short = dir.join("prayers.txt");
        std::fs::write(&short, b"we pray for the sick").expect("write short text");
        snippets_keep(
            &store_path,
            &bytes_dir,
            &index_path,
            &short,
            "text",
            Some("prayers"),
        )
        .expect("keep works");
        // Same bytes: Same file.
        let same =
            snippets_check(&store_path, &bytes_dir, &index_path, &long).expect("check works");
        assert_eq!(same.len(), 1);
        assert!(same[0].starts_with("Same file"), "report: {}", same[0]);
        assert!(same[0].contains("prayer-long"), "report: {}", same[0]);
        // Retyped: Same content.
        let retyped = dir.join("prayer-long-copy.txt");
        std::fs::write(
            &retyped,
            "LORD, in Your Mercy — hear our prayer for all who are sick and in need today!",
        )
        .expect("write retype");
        let content =
            snippets_check(&store_path, &bytes_dir, &index_path, &retyped).expect("check works");
        assert_eq!(content.len(), 1);
        assert!(
            content[0].starts_with("Same content"),
            "report: {}",
            content[0]
        );
        // Unrelated: no match.
        let other = dir.join("other.txt");
        std::fs::write(
            &other,
            b"The coffee morning is on Tuesday in the parish hall with cake and tea",
        )
        .expect("write other");
        let none =
            snippets_check(&store_path, &bytes_dir, &index_path, &other).expect("check works");
        assert_eq!(none.len(), 1);
        assert!(none[0].starts_with("no match"), "report: {}", none[0]);
        // Short near-duplicate: Probably the same, with the diff.
        let near = dir.join("prayers-ill.txt");
        std::fs::write(&near, b"we pray for the ill").expect("write near text");
        let ask = snippets_check(&store_path, &bytes_dir, &index_path, &near).expect("check works");
        assert!(
            ask[0].starts_with("Probably the same"),
            "report: {}",
            ask[0]
        );
        assert!(
            ask.iter().any(|line| line.contains("prayers")),
            "report: {ask:?}"
        );
        assert!(
            ask.iter().any(|line| line.contains("sick")),
            "report: {ask:?}"
        );
        // Checks stored nothing.
        let store = ob_snippets::SnippetStore::load(&store_path).expect("store reloads");
        assert_eq!(store.snippets.len(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn snippets_keep_reports_a_needs_check_proposal_without_storing() {
        let dir = snippets_test_dir("ob-cli-snippets-keep-ask-test");
        let (store_path, bytes_dir, index_path) = snippets_paths(&dir);
        std::fs::create_dir_all(&dir).expect("mkdir");
        let base = dir.join("busy.png");
        let base_bytes = snippets_busy_picture();
        std::fs::write(&base, &base_bytes).expect("write base");
        snippets_keep(&store_path, &bytes_dir, &index_path, &base, "picture", None)
            .expect("keep works");
        let near = dir.join("busy-near.png");
        let near_bytes = snippets_busy_picture_near();
        assert_ne!(near_bytes, base_bytes);
        std::fs::write(&near, &near_bytes).expect("write near");
        let outcome = snippets_keep(&store_path, &bytes_dir, &index_path, &near, "picture", None)
            .expect("near keep decides");
        let lines = match outcome {
            SnippetKeepOutcome::Ask(lines) => lines,
            other => panic!("near-duplicate keep must Ask, got {other:?}"),
        };
        assert!(
            lines[0].starts_with("Probably the same"),
            "lines: {lines:?}"
        );
        assert!(
            lines.iter().any(|line| line.contains("busy.png")),
            "lines: {lines:?}"
        );
        let store = ob_snippets::SnippetStore::load(&store_path).expect("store reloads");
        assert_eq!(store.snippets.len(), 1);
        assert!(
            !bytes_dir
                .join(ob_fingerprint::sha256_hex(&near_bytes))
                .exists(),
            "the near-duplicate bytes must not be stored"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn snippets_loud_errors() {
        let dir = snippets_test_dir("ob-cli-snippets-loud-errors-test");
        let (store_path, bytes_dir, index_path) = snippets_paths(&dir);
        std::fs::create_dir_all(&dir).expect("mkdir");
        // Missing file.
        let missing = dir.join("no-such-file.txt");
        let err = snippets_keep(&store_path, &bytes_dir, &index_path, &missing, "text", None)
            .expect_err("missing file must be loud");
        assert!(!err.is_empty(), "error is empty");
        // Damaged store.
        let file = dir.join("note.txt");
        std::fs::write(&file, b"Lord, hear our prayer").expect("write text");
        std::fs::write(&store_path, "{not json").expect("damage the store");
        let err = snippets_keep(&store_path, &bytes_dir, &index_path, &file, "text", None)
            .expect_err("damaged store must be loud");
        assert!(err.contains("damaged"), "error is {err}");
        let err = snippets_check(&store_path, &bytes_dir, &index_path, &file)
            .expect_err("damaged store must be loud on check");
        assert!(err.contains("damaged"), "error is {err}");
        std::fs::remove_file(&store_path).expect("clear the damage");
        // Undecodable picture.
        let bad_pic = dir.join("bad.png");
        std::fs::write(&bad_pic, b"not a picture").expect("write bad picture");
        let err = snippets_keep(
            &store_path,
            &bytes_dir,
            &index_path,
            &bad_pic,
            "picture",
            None,
        )
        .expect_err("undecodable picture must be loud");
        assert!(err.contains("decode"), "error is {err}");
        // Unparsable song.
        let bad_tune = dir.join("bad.musicxml");
        std::fs::write(&bad_tune, b"not musicxml at all").expect("write bad tune");
        let err = snippets_keep(
            &store_path,
            &bytes_dir,
            &index_path,
            &bad_tune,
            "song",
            None,
        )
        .expect_err("unparsable song must be loud");
        assert!(!err.is_empty(), "error is empty");
        // Non-UTF-8 text.
        let bad_text = dir.join("bad.txt");
        std::fs::write(&bad_text, [0xff, 0xfe, 0xfd]).expect("write bad text");
        let err = snippets_keep(
            &store_path,
            &bytes_dir,
            &index_path,
            &bad_text,
            "text",
            None,
        )
        .expect_err("non-UTF-8 text must be loud");
        assert!(err.contains("UTF-8"), "error is {err}");
        // Garbage check: none of the three kinds.
        let blob = dir.join("blob.bin");
        std::fs::write(&blob, [0xff, 0xfe, 0x00]).expect("write blob");
        let err = snippets_check(&store_path, &bytes_dir, &index_path, &blob)
            .expect_err("garbage check must be loud");
        assert!(err.contains("not a picture"), "error is {err}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn snippets_run_wiring() {
        // Drive the real dispatch arms: list on an empty
        // store, keep, check, and a keep that Asks (an
        // error, with nothing stored).
        let dir = snippets_test_dir("ob-cli-snippets-run-test");
        let (store_path, _, _) = snippets_paths(&dir);
        std::fs::create_dir_all(&dir).expect("mkdir");
        let list = run(Cli {
            cmd: Cmd::Snippets {
                action: SnippetsCmd::List {
                    store: Some(store_path.clone()),
                },
            },
        });
        assert!(list.is_ok(), "list on an empty store works");
        let file = dir.join("note.txt");
        std::fs::write(&file, b"Lord, hear our prayer").expect("write text");
        let keep = run(Cli {
            cmd: Cmd::Snippets {
                action: SnippetsCmd::Keep {
                    file: file.clone(),
                    kind: "text".to_string(),
                    name: Some("note".to_string()),
                    store: Some(store_path.clone()),
                },
            },
        });
        assert!(keep.is_ok(), "keep works");
        let check = run(Cli {
            cmd: Cmd::Snippets {
                action: SnippetsCmd::Check {
                    file: file.clone(),
                    store: Some(store_path.clone()),
                },
            },
        });
        assert!(check.is_ok(), "check works");
        let base = dir.join("busy.png");
        std::fs::write(&base, snippets_busy_picture()).expect("write base");
        let keep_pic = run(Cli {
            cmd: Cmd::Snippets {
                action: SnippetsCmd::Keep {
                    file: base.clone(),
                    kind: "picture".to_string(),
                    name: None,
                    store: Some(store_path.clone()),
                },
            },
        });
        assert!(keep_pic.is_ok(), "picture keep works");
        let near = dir.join("busy-near.png");
        std::fs::write(&near, snippets_busy_picture_near()).expect("write near");
        let ask = run(Cli {
            cmd: Cmd::Snippets {
                action: SnippetsCmd::Keep {
                    file: near.clone(),
                    kind: "picture".to_string(),
                    name: None,
                    store: Some(store_path.clone()),
                },
            },
        });
        assert!(ask.is_err(), "a keep that Asks must fail the command");
        let store = ob_snippets::SnippetStore::load(&store_path).expect("store reloads");
        assert_eq!(store.snippets.len(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
