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
//! to libpdfium.so / pdfium.dll), defaulting to the workspace
//! toolchain copy used in development.

use ob_extract::{ExtractionDump, PageInfo, SourceKind, Span, SpanTrust};
use pdfium_render::prelude::*;
use std::process::exit;

fn lib_path() -> String {
    if let Ok(p) = std::env::var("OB_PDFIUM_LIB") {
        return p;
    }
    let home = std::env::var("HOME").unwrap_or_default();
    format!("{home}/workspace/.toolchain/pdfium/lib/libpdfium.so")
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
    let bytes = std::fs::read(&path).unwrap_or_else(|e| {
        eprintln!("cannot read {path}: {e}");
        exit(1);
    });
    let bindings = Pdfium::bind_to_library(lib_path()).unwrap_or_else(|e| {
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

    let mut dump = ExtractionDump {
        schema_version: ob_extract::DUMP_SCHEMA_VERSION,
        source_kind: SourceKind::Pdf,
        source_name: path.clone(),
        source_bytes: bytes.len(),
        extractor: "pdfium-157.0.8100.0".to_string(),
        pages: Vec::new(),
        spans: Vec::new(),
    };

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
            let (font_size, bold, font) = match segment.chars() {
                Ok(chars) => match chars.iter().next() {
                    Some(c) => {
                        let name = c.font_name();
                        (
                            Some(c.scaled_font_size().value),
                            Some(name.to_lowercase().contains("bold")),
                            Some(name),
                        )
                    }
                    None => (None, None, None),
                },
                Err(_) => (None, None, None),
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
            });
            page_spans += 1;
        }
        dump.pages.push(PageInfo {
            index: index as u32,
            width_pt: width,
            height_pt: height,
            picture_only: page_spans == 0,
        });
    }

    println!("{}", serde_json::to_string(&dump).expect("serialize dump"));
}

/// Draw every page to `page-NN.png` in the output folder, at about
/// 200 dpi: crisp enough to trace over, small enough to keep.
fn render_pages(path: &str, out_dir: &str) {
    let bytes = std::fs::read(path).unwrap_or_else(|e| {
        eprintln!("cannot read {path}: {e}");
        exit(1);
    });
    let bindings = Pdfium::bind_to_library(lib_path()).unwrap_or_else(|e| {
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
