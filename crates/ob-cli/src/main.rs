//! OpenBulletin CLI — every engine operation, headless.
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

/// The trace rescue: render the old PDF's pages to pictures with
/// the PDFium helper, then build a draft bulletin whose panels
/// carry those pictures as guides (docs/EXTRACT.md). The admin
/// opens the draft and types the new bulletin over the pictures;
/// guides never print.
fn run_trace(input: &Path, out: &Path, helper: Option<&Path>) -> Result<(), String> {
    let helper = resolve_helper(helper)?;
    let assets_dir = out.join("assets");
    std::fs::create_dir_all(&assets_dir)
        .map_err(|e| format!("cannot create {}: {e}", assets_dir.display()))?;
    let run = |args: &[&str]| -> Result<Vec<u8>, String> {
        let output = std::process::Command::new(&helper)
            .args(args)
            .output()
            .map_err(|e| format!("cannot run the PDF helper ({}): {e}", helper.display()))?;
        if !output.status.success() {
            return Err(format!(
                "the PDF helper failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        Ok(output.stdout)
    };
    let input_str = input.to_string_lossy().to_string();
    run(&["render", &input_str, &assets_dir.to_string_lossy()])?;
    let dump_bytes = run(&[&input_str])?;
    let dump: serde_json::Value = serde_json::from_slice(&dump_bytes)
        .map_err(|e| format!("the PDF helper answered oddly: {e}"))?;
    let pages = dump["pages"].as_array().cloned().unwrap_or_default();
    if pages.is_empty() {
        return Err("that PDF has no pages we can read".into());
    }
    let to_mpt = |v: f64| (v * 1000.0).round() as ob_model::Mpt;
    let page_sizes: Vec<ob_model::Size> = pages
        .iter()
        .map(|pg| ob_model::Size {
            w: to_mpt(pg["width_pt"].as_f64().unwrap_or(612.0)),
            h: to_mpt(pg["height_pt"].as_f64().unwrap_or(792.0)),
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
    if let Some(p) = explicit {
        return Ok(p.to_path_buf());
    }
    if let Ok(p) = std::env::var("OB_PDFIUM_HELPER") {
        return Ok(PathBuf::from(p));
    }
    if let Ok(exe) = std::env::current_exe() {
        let sibling = exe.with_file_name("ob-pdfium-helper");
        if sibling.exists() {
            return Ok(sibling);
        }
    }
    Ok(PathBuf::from("ob-pdfium-helper"))
}
