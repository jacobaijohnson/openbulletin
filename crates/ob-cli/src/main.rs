//! OpenBulletin CLI — every engine operation, headless.
//! If it cannot be done here, the GUI is hiding engine problems.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

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
    /// Export a PDF. Default: reader panels. --imposed: printer sheets.
    Export {
        input: PathBuf,
        #[arg(long)]
        imposed: bool,
        #[arg(long)]
        out: PathBuf,
    },
    /// Clone a document for next Sunday (date +7, weekly content cleared).
    NextSunday {
        input: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    /// Print document facts.
    Info { input: PathBuf },
    /// Font library: list what is installed, or add a font file.
    Fonts {
        #[command(subcommand)]
        action: FontsCmd,
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
                Some(d) => {
                    chrono::NaiveDate::parse_from_str(&d, "%Y-%m-%d").map_err(|e| e.to_string())?
                }
                None => chrono::Local::now().date_naive(),
            };
            let profile = ob_model::ChurchProfile {
                name: church,
                address,
                service_times: times,
                website,
                giving_url: String::new(),
                pastor,
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
            out,
        } => {
            let doc = ob_format::load(&input)?;
            let bytes = if imposed {
                ob_pdf::export_imposed_pdf(&doc)
            } else {
                ob_pdf::export_reader_pdf(&doc)
            };
            std::fs::write(&out, &bytes).map_err(|e| e.to_string())?;
            println!(
                "wrote {} ({} bytes, {})",
                out.display(),
                bytes.len(),
                if imposed {
                    "imposed sheets"
                } else {
                    "reader panels"
                }
            );
            Ok(())
        }
        Cmd::NextSunday { input, out } => {
            let doc = ob_format::load(&input)?;
            let next = ob_layout::next_sunday(&doc)?;
            ob_format::save(&next, &out)?;
            println!(
                "wrote {} for {} ({})",
                out.display(),
                next.fields.get("sunday_date").cloned().unwrap_or_default(),
                next.fields
                    .get("sunday_date_long")
                    .cloned()
                    .unwrap_or_default()
            );
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
                    .get("sunday_date_long")
                    .cloned()
                    .unwrap_or_default(),
                doc.fields.get("sunday_date").cloned().unwrap_or_default()
            );
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
                    for file in &files {
                        let bytes = std::fs::read(file)
                            .map_err(|e| format!("cannot read {}: {e}", file.display()))?;
                        if file
                            .extension()
                            .map(|e| e.eq_ignore_ascii_case("zip"))
                            .unwrap_or(false)
                        {
                            for result in lib.add_zip_bytes(&bytes) {
                                match result {
                                    Ok(info) => {
                                        println!("added {} ({})", info.full_name, info.file_name)
                                    }
                                    Err(e) => println!("skipped one in {}: {e}", file.display()),
                                }
                            }
                        } else {
                            let info = lib.add_path(file)?;
                            println!("added {} ({})", info.full_name, info.file_name);
                            println!("{}", info.rights.plain_words());
                        }
                    }
                }
            }
            Ok(())
        }
    }
}
