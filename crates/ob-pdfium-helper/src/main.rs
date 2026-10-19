//! Out-of-process PDFium extraction helper.
//!
//! The analyzer's real extractor (docs/EXTRACT.md): PDFium reads the
//! PDF and reports every text run WITH its position, size, and font,
//! which is what turns word order from a guess into a fact. This runs
//! as a separate process on purpose: a malformed PDF can crash a
//! native parser, and a crash here costs one file, not the app.
//!
//! Usage: ob-pdfium-helper <file.pdf>            (dump JSON on stdout)
//!        ob-pdfium-helper render <file.pdf> <out-dir>
//! Render mode draws every page to a PNG in the output folder: the
//! tracing guides for the import rescue (docs/EXTRACT.md).
//! The PDFium shared library is found via $OB_PDFIUM_LIB (full path
//! to libpdfium.so / pdfium.dll), or beside this helper program
//! (the shipped layout). There is no other place it will look:
//! if neither resolves, the helper refuses (fails closed) rather
//! than reach for a development tree (health audit S-10).

use ob_extract::{ExtractionDump, PageInfo, SourceKind, Span, SpanTrust};
use pdfium_render::prelude::*;
use std::process::exit;

fn lib_path() -> Option<String> {
    if let Ok(p) = std::env::var("OB_PDFIUM_LIB") {
        return Some(p);
    }
    // The shipped bundle layout: the library sits beside the
    // helper itself (pdfium.dll on Windows, libpdfium.so on
    // Linux, libpdfium.dylib on macOS).
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for name in ["pdfium.dll", "libpdfium.so", "libpdfium.dylib"] {
                let candidate = dir.join(name);
                if candidate.exists() {
                    return Some(candidate.to_string_lossy().into_owned());
                }
            }
        }
    }
    // No development-tree fallback: a shipped helper whose
    // library is not beside it and not named by the env
    // override fails closed (health audit S-10).
    None
}

/// The library path, or the helper's refusal in its
/// existing error shape (a line on stderr, exit 1) when
/// no library resolves.
fn lib_path_or_refuse() -> String {
    lib_path().unwrap_or_else(|| {
        eprintln!(
            "the PDF library that travels with the app could not be found \
             (it should sit beside this reader program)"
        );
        exit(1);
    })
}

/// The open-door gate (HF-2): a PDF past the standing
/// 512 MB limit is refused from its filesystem size,
/// before it is read whole or handed to PDFium. The
/// refusal message when the file must be turned away, in
/// this helper's plain error shape; None when it may
/// proceed. A file whose size cannot be read proceeds:
/// the read itself will speak for that case.
fn oversize_refusal(path: &str) -> Option<String> {
    let meta = std::fs::metadata(path).ok()?;
    if meta.len() > ob_extract::MAX_OPEN_DOOR_BYTES {
        Some(format!(
            "{path} is too large to read: OpenBulletin can open files up to \
             512 megabytes, and this one is bigger than that"
        ))
    } else {
        None
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let first = args.next().unwrap_or_else(|| {
        eprintln!("usage: ob-pdfium-helper <file.pdf> | render <file.pdf> <out-dir>");
        exit(2);
    });
    if first == "render" {
        let pdf = args.next().unwrap_or_else(|| {
            eprintln!("usage: ob-pdfium-helper render <file.pdf> <out-dir>");
            exit(2);
        });
        let out_dir = args.next().unwrap_or_else(|| {
            eprintln!("usage: ob-pdfium-helper render <file.pdf> <out-dir>");
            exit(2);
        });
        render_pages(&pdf, &out_dir);
        return;
    }
    let path = first;
    // Dump mode's optional second argument: a folder for the
    // page pictures (schema v2's image channel). Without it,
    // the dump carries no image files, as v1 did.
    let images_dir = args.next();
    if let Some(why) = oversize_refusal(&path) {
        eprintln!("{why}");
        exit(1);
    }
    let bytes = std::fs::read(&path).unwrap_or_else(|e| {
        eprintln!("cannot read {path}: {e}");
        exit(1);
    });
    let bindings = Pdfium::bind_to_library(lib_path_or_refuse()).unwrap_or_else(|e| {
        eprintln!(
            "the PDF library that travels with the app could not be loaded \
             (it should sit beside this reader program). Details: {e:?}"
        );
        exit(1);
    });
    let pdfium = Pdfium::new(bindings);
    let doc = pdfium
        .load_pdf_from_byte_slice(&bytes, None)
        .unwrap_or_else(|e| {
            eprintln!("cannot parse {path}: {e:?}");
            exit(1);
        });

    let mut dump = ExtractionDump {
        schema_version: ob_extract::DUMP_SCHEMA_VERSION,
        source_kind: SourceKind::Pdf,
        source_name: path.clone(),
        source_bytes: bytes.len(),
        extractor: "pdfium-157.0.8100.0".to_string(),
        pages: Vec::new(),
        spans: Vec::new(),
        images: Vec::new(),
        loss_notes: Vec::new(),
    };
    if let Some(dir) = &images_dir {
        std::fs::create_dir_all(dir).unwrap_or_else(|e| {
            eprintln!("cannot create image folder {dir}: {e}");
            exit(1);
        });
    }

    for (index, page) in doc.pages().iter().enumerate() {
        let width = page.width().value;
        let height = page.height().value;
        let text = page.text().unwrap_or_else(|e| {
            eprintln!("page {} text failed: {e:?}", index + 1);
            exit(1);
        });
        let mut page_spans = 0usize;
        for segment in text.segments().iter() {
            let s = segment.text();
            let trimmed = s.trim();
            if trimmed.is_empty() {
                continue;
            }
            let b = segment.bounds();
            // Schema v2: italic from the font flags and the
            // fill color from the character itself, not from
            // guesses about the font's name.
            let (font_size, bold, font, italic, color) = match segment.chars() {
                Ok(chars) => match chars.iter().next() {
                    Some(c) => {
                        let name = c.font_name();
                        let fill = c
                            .fill_color()
                            .map(|col| [col.red(), col.green(), col.blue()])
                            .unwrap_or([0, 0, 0]);
                        (
                            Some(c.scaled_font_size().value),
                            Some(name.to_lowercase().contains("bold")),
                            Some(name),
                            c.font_is_italic(),
                            fill,
                        )
                    }
                    None => (None, None, None, false, [0, 0, 0]),
                },
                Err(_) => (None, None, None, false, [0, 0, 0]),
            };
            dump.spans.push(Span {
                page: index as u32,
                text: trimmed.to_string(),
                trust: SpanTrust::RawText,
                // PDFium reports bottom-up coordinates; the dump
                // schema is top-left origin.
                x: Some(b.left().value),
                y: Some(height - b.top().value),
                w: Some(b.width().value),
                h: Some(b.height().value),
                font_size,
                bold,
                font,
                align: None,
                italic,
                color,
                runs: Vec::new(),
                thread: None,
            });
            page_spans += 1;
        }
        // The image channel: every picture object on the
        // page, decoded to a PNG beside the dump. A picture
        // that will not decode is reported on stderr and
        // skipped; the page's words (and its tracing render)
        // do not depend on it.
        if let Some(dir) = &images_dir {
            let mut n = 0usize;
            for object in page.objects().iter() {
                let PdfPageObject::Image(image) = &object else {
                    continue;
                };
                n += 1;
                let id = format!("p{}-img{}", index + 1, n);
                let file = format!("{id}.png");
                let saved = image.get_raw_image().ok().and_then(|img| {
                    let target = format!("{dir}/{file}");
                    img.save_with_format(&target, image::ImageFormat::Png).ok()
                });
                match (saved, image.bounds()) {
                    (Some(()), Ok(q)) => {
                        dump.images.push(ob_extract::DumpImage {
                            id,
                            page: index as u32,
                            x: q.left().value,
                            y: height - q.top().value,
                            w: q.right().value - q.left().value,
                            h: q.top().value - q.bottom().value,
                            file,
                        });
                    }
                    _ => {
                        eprintln!(
                            "page {} picture {} not carried (decode or bounds failed)",
                            index + 1,
                            n
                        );
                    }
                }
            }
        }
        dump.pages.push(PageInfo {
            index: index as u32,
            width_pt: width,
            height_pt: height,
            picture_only: page_spans == 0,
        });
    }

    // Alignment is not a segment property in PDFium; it is
    // inferred from line geometry against each block, in the
    // one implementation every consumer shares.
    ob_extract::infer_alignments(&mut dump);

    println!("{}", serde_json::to_string(&dump).expect("serialize dump"));
}

/// Draw every page to `page-NN.png` in the output folder, at about
/// 200 dpi: crisp enough to trace over, small enough to keep.
fn render_pages(path: &str, out_dir: &str) {
    if let Some(why) = oversize_refusal(path) {
        eprintln!("{why}");
        exit(1);
    }
    let bytes = std::fs::read(path).unwrap_or_else(|e| {
        eprintln!("cannot read {path}: {e}");
        exit(1);
    });
    let bindings = Pdfium::bind_to_library(lib_path_or_refuse()).unwrap_or_else(|e| {
        eprintln!("cannot load PDFium library: {e:?}");
        exit(1);
    });
    let pdfium = Pdfium::new(bindings);
    let doc = pdfium
        .load_pdf_from_byte_slice(&bytes, None)
        .unwrap_or_else(|e| {
            eprintln!("cannot parse {path}: {e:?}");
            exit(1);
        });
    std::fs::create_dir_all(out_dir).unwrap_or_else(|e| {
        eprintln!("cannot create {out_dir}: {e}");
        exit(1);
    });
    let mut written = 0;
    for (index, page) in doc.pages().iter().enumerate() {
        let target_width = (page.width().value * 200.0 / 72.0).round() as i32;
        let config = PdfRenderConfig::new()
            .set_target_width(target_width.max(200))
            .set_clear_color(PdfColor::new(255, 255, 255, 255));
        match page.render_with_config(&config) {
            Ok(bitmap) => {
                let file = format!("{out_dir}/page-{:02}.png", index + 1);
                match bitmap
                    .as_image()
                    .save_with_format(&file, image::ImageFormat::Png)
                {
                    Ok(()) => written += 1,
                    Err(e) => eprintln!("cannot write {file}: {e}"),
                }
            }
            Err(e) => eprintln!("cannot render page {}: {e:?}", index + 1),
        }
    }
    println!("rendered {written} pages to {out_dir}");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// HF-2 pin: a sparse file whose stated length is past
    /// the limit (set_len allocates nothing, so the pin is
    /// instant) is refused from its metadata alone, in
    /// plain words naming the limit; an ordinary file and
    /// a missing file both pass the gate (the read speaks
    /// for the missing one).
    #[test]
    fn oversize_pdf_is_refused_from_its_stated_size() {
        let path =
            std::env::temp_dir().join(format!("ob-helper-door-gate-{}.pdf", std::process::id()));
        let file = std::fs::File::create(&path).expect("create sparse file");
        file.set_len(ob_extract::MAX_OPEN_DOOR_BYTES + 1)
            .expect("state an oversize length");
        drop(file);
        let why = oversize_refusal(&path.to_string_lossy()).expect("the gate refuses");
        assert!(why.contains("too large"), "{why}");
        assert!(why.contains("512 megabytes"), "{why}");
        let _ = std::fs::remove_file(&path);

        let small =
            std::env::temp_dir().join(format!("ob-helper-door-small-{}.pdf", std::process::id()));
        std::fs::write(&small, b"%PDF-small").expect("write small file");
        assert!(oversize_refusal(&small.to_string_lossy()).is_none());
        let _ = std::fs::remove_file(&small);
        assert!(oversize_refusal("no-such-file-xyz.pdf").is_none());
    }
}
