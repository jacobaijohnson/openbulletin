//! Picture decoding for OpenBulletin: the recognition door.
//!
//! One small API over every picture format a church office
//! can plausibly be handed (docs/planning/beyond-print.md
//! section 6). Raster formats (PNG, JPEG, WebP, GIF, BMP,
//! TIFF) decode through the `image` crate; SVG renders
//! through resvg at the resolution the caller asks for;
//! JPEG XL decodes through jxl-oxide, pure Rust. EXIF
//! orientation is applied on every raster decode, so a
//! phone photo never lands sideways.
//!
//! Two formats are RECOGNIZED but refused, in plain words,
//! with convert-first guidance: HEIC/HEIF (patent-encumbered,
//! no license-clean Rust decoder) and AVIF. AVIF's story is
//! narrower: the `image` crate's own AVIF support encodes
//! only, and its decode feature (`avif-native`) binds the
//! dav1d C library through a system-library lookup
//! (dav1d-sys + pkg-config, libdav1d >= 1.3.0). Measured
//! 2026-10-16: this machine has no libdav1d and the attempt
//! fails at the pkg-config probe; the one-download Windows
//! bundle builds with a bare `cargo build` on a stock
//! runner, which has no libdav1d either, so the dependency
//! would break the bundle. AVIF therefore starts beside
//! HEIC as a convert-first label until a pure-Rust decoder
//! lands. Both refusals are named, never silent.
//!
//! Decoded pixels are RGBA8. Nothing here ever writes back
//! to a document's asset store: callers cache the decoded
//! form themselves, keyed by the asset's fingerprint.

use std::io::Cursor;

/// A picture format the door recognizes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PictureFormat {
    Png,
    Jpeg,
    WebP,
    Gif,
    Bmp,
    Tiff,
    Svg,
    JpegXl,
    Avif,
    Heic,
    Unknown,
}

impl PictureFormat {
    /// The IANA media type the asset store records.
    pub fn media_type(self) -> &'static str {
        match self {
            PictureFormat::Png => "image/png",
            PictureFormat::Jpeg => "image/jpeg",
            PictureFormat::WebP => "image/webp",
            PictureFormat::Gif => "image/gif",
            PictureFormat::Bmp => "image/bmp",
            PictureFormat::Tiff => "image/tiff",
            PictureFormat::Svg => "image/svg+xml",
            PictureFormat::JpegXl => "image/jxl",
            PictureFormat::Avif => "image/avif",
            PictureFormat::Heic => "image/heic",
            PictureFormat::Unknown => "application/octet-stream",
        }
    }

    /// True when this crate decodes the format today.
    pub fn decodable(self) -> bool {
        matches!(
            self,
            PictureFormat::Png
                | PictureFormat::Jpeg
                | PictureFormat::WebP
                | PictureFormat::Gif
                | PictureFormat::Bmp
                | PictureFormat::Tiff
                | PictureFormat::Svg
                | PictureFormat::JpegXl
        )
    }

    /// True for the still formats (everything but SVG, which
    /// has no fixed pixel size of its own).
    pub fn is_raster(self) -> bool {
        self.decodable() && self != PictureFormat::Svg
    }
}

/// What went wrong, in two families: the file is a picture
/// we know but cannot decode here (the caller words this as
/// convert-first guidance), or the bytes are not a readable
/// picture at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageError {
    /// Recognized, not decodable in this program (yet).
    ConvertFirst(PictureFormat),
    /// Not a picture, or a broken one.
    Unreadable(String),
}

impl std::fmt::Display for ImageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImageError::ConvertFirst(format) => {
                write!(f, "{} pictures need converting first", format.media_type())
            }
            ImageError::Unreadable(why) => write!(f, "not a readable picture: {why}"),
        }
    }
}

impl std::error::Error for ImageError {}

/// Recognize a picture from its first bytes. Magic bytes
/// win; an SVG (which has no magic) is recognized by its
/// text opening. Returns Unknown for anything else.
pub fn sniff(bytes: &[u8]) -> PictureFormat {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        return PictureFormat::Png;
    }
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return PictureFormat::Jpeg;
    }
    if bytes.starts_with(b"GIF8") {
        return PictureFormat::Gif;
    }
    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return PictureFormat::WebP;
    }
    // ISO-BMFF family (AVIF, HEIC): a "ftyp" box whose brand
    // names the format.
    if bytes.len() >= 12 && &bytes[4..8] == b"ftyp" {
        let brand = &bytes[8..12];
        if matches!(brand, b"avif" | b"avis") {
            return PictureFormat::Avif;
        }
        if matches!(
            brand,
            b"heic" | b"heix" | b"hevc" | b"heim" | b"heis" | b"mif1" | b"mif2" | b"msf1"
        ) {
            return PictureFormat::Heic;
        }
    }
    // JPEG XL: the raw codestream signature, or its
    // container box.
    if bytes.starts_with(&[0xFF, 0x0A])
        || (bytes.len() >= 12 && bytes[0..4] == [0x00, 0x00, 0x00, 0x0C] && &bytes[4..8] == b"JXL ")
    {
        return PictureFormat::JpegXl;
    }
    if bytes.starts_with(&[0x49, 0x49, 0x2A, 0x00]) || bytes.starts_with(&[0x4D, 0x4D, 0x00, 0x2A])
    {
        return PictureFormat::Tiff;
    }
    if bytes.starts_with(b"BM") && bytes.len() >= 26 {
        return PictureFormat::Bmp;
    }
    if looks_like_svg(bytes) {
        return PictureFormat::Svg;
    }
    PictureFormat::Unknown
}

fn looks_like_svg(bytes: &[u8]) -> bool {
    let head = &bytes[..bytes.len().min(2048)];
    let Ok(text) = std::str::from_utf8(head) else {
        return false;
    };
    let text = text.trim_start_matches('\u{feff}').trim_start();
    (text.starts_with('<') && text.contains("<svg"))
        || text.starts_with("<?xml") && text.contains("<svg")
}

/// The format a file NAME claims, for the files whose bytes
/// are not at hand (a library listing) and as a cross-check
/// at the doors.
pub fn format_for_name(name: &str) -> Option<PictureFormat> {
    let ext = name.rsplit('.').next()?.to_ascii_lowercase();
    Some(match ext.as_str() {
        "png" => PictureFormat::Png,
        "jpg" | "jpeg" => PictureFormat::Jpeg,
        "webp" => PictureFormat::WebP,
        "gif" => PictureFormat::Gif,
        "bmp" => PictureFormat::Bmp,
        "tif" | "tiff" => PictureFormat::Tiff,
        "svg" => PictureFormat::Svg,
        "jxl" => PictureFormat::JpegXl,
        "avif" => PictureFormat::Avif,
        "heic" | "heif" => PictureFormat::Heic,
        _ => return None,
    })
}

/// A picture's facts without its pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Probe {
    pub format: PictureFormat,
    /// Pixel dimensions (raster) or intrinsic size (SVG,
    /// rounded). For SVG these are a nominal size only; the
    /// art is resolution-independent.
    pub width: u32,
    pub height: u32,
}

/// Decoded pixels: RGBA8, row-major, orientation applied.
#[derive(Debug, Clone, PartialEq)]
pub struct Decoded {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    /// True when any pixel's alpha is below fully opaque.
    pub has_alpha: bool,
}

impl Decoded {
    fn from_image(img: image::DynamicImage) -> Decoded {
        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();
        let has_alpha = rgba.as_raw().chunks_exact(4).any(|px| px[3] < 255);
        Decoded {
            width,
            height,
            rgba: rgba.into_raw(),
            has_alpha,
        }
    }
}

/// Refuse the recognized-but-undecodable formats before any
/// real work, so every entry point words them the same way.
fn gate(format: PictureFormat) -> Result<PictureFormat, ImageError> {
    match format {
        PictureFormat::Avif | PictureFormat::Heic => Err(ImageError::ConvertFirst(format)),
        PictureFormat::Unknown => Err(ImageError::Unreadable(
            "the file does not start like any picture we know".into(),
        )),
        other => Ok(other),
    }
}

/// Dimensions without decoding the whole picture.
pub fn probe(bytes: &[u8]) -> Result<Probe, ImageError> {
    let format = gate(sniff(bytes))?;
    let (width, height) = match format {
        PictureFormat::Svg => {
            let tree = parse_svg(bytes)?;
            let size = tree.size();
            (
                size.width().round().max(1.0) as u32,
                size.height().round().max(1.0) as u32,
            )
        }
        PictureFormat::JpegXl => {
            use image::ImageDecoder as _;
            let decoder = jxl_decoder(bytes)?;
            decoder.dimensions()
        }
        _ => {
            let mut reader = image::ImageReader::new(Cursor::new(bytes));
            reader = reader
                .with_guessed_format()
                .map_err(|e| ImageError::Unreadable(e.to_string()))?;
            reader
                .into_dimensions()
                .map_err(|e| ImageError::Unreadable(e.to_string()))?
        }
    };
    Ok(Probe {
        format,
        width,
        height,
    })
}

fn jxl_decoder(
    bytes: &[u8],
) -> Result<jxl_oxide::integration::JxlDecoder<Cursor<&[u8]>>, ImageError> {
    jxl_oxide::integration::JxlDecoder::new(Cursor::new(bytes))
        .map_err(|e| ImageError::Unreadable(format!("JPEG XL: {e}")))
}

/// Decode a raster picture (PNG/JPEG/WebP/GIF/BMP/TIFF/JXL)
/// to RGBA8 with its EXIF orientation applied. SVG goes
/// through [`render_svg`] or [`decode_fitted`], which know
/// the size the caller wants.
pub fn decode(bytes: &[u8]) -> Result<Decoded, ImageError> {
    let format = gate(sniff(bytes))?;
    if format == PictureFormat::Svg {
        // No target size was named: render at the art's
        // intrinsic size, the only honest default.
        let p = probe(bytes)?;
        return render_svg(bytes, p.width, p.height);
    }
    let mut img = decode_raster(bytes, format)?;
    // Big camera files are fine to hold, but a hostile or
    // broken file must not talk us into a giant allocation:
    // the image crate's own limits already ran inside the
    // decoder; this is the belt over those braces.
    if (img.width() as u64) * (img.height() as u64) > 100_000_000 {
        img = img.thumbnail(10_000, 10_000);
    }
    Ok(Decoded::from_image(img))
}

fn decode_raster(bytes: &[u8], format: PictureFormat) -> Result<image::DynamicImage, ImageError> {
    use image::ImageDecoder as _;
    let mut reader = image::ImageReader::new(Cursor::new(bytes));
    reader = reader
        .with_guessed_format()
        .map_err(|e| ImageError::Unreadable(e.to_string()))?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(20_000);
    limits.max_image_height = Some(20_000);
    limits.max_alloc = Some(256 * 1024 * 1024);
    reader.limits(limits);
    if format == PictureFormat::JpegXl {
        let decoder = jxl_decoder(bytes)?;
        let mut img = image::DynamicImage::from_decoder(decoder)
            .map_err(|e| ImageError::Unreadable(e.to_string()))?;
        // jxl-oxide applies the file's orientation while it
        // renders, so there is nothing to fold in here.
        return Ok(std::mem::take(&mut img));
    }
    let mut decoder = reader
        .into_decoder()
        .map_err(|e| ImageError::Unreadable(e.to_string()))?;
    let orientation = decoder
        .orientation()
        .unwrap_or(image::metadata::Orientation::NoTransforms);
    let mut img = image::DynamicImage::from_decoder(decoder)
        .map_err(|e| ImageError::Unreadable(e.to_string()))?;
    img.apply_orientation(orientation);
    Ok(img)
}

/// Decode any picture, sized to fit WITHIN `max_w` x `max_h`
/// pixels (aspect kept, never upscaled): the canvas, the
/// previews, and the thumbnails all want exactly this. SVG
/// renders straight at the fitted size.
pub fn decode_fitted(bytes: &[u8], max_w: u32, max_h: u32) -> Result<Decoded, ImageError> {
    let format = gate(sniff(bytes))?;
    if format == PictureFormat::Svg {
        let p = probe(bytes)?;
        let (w, h) = fit_dims(p.width, p.height, max_w.max(1), max_h.max(1));
        return render_svg(bytes, w, h);
    }
    let img = decode_raster(bytes, format)?;
    let fitted = fit_within(img, max_w.max(1), max_h.max(1));
    Ok(Decoded::from_image(fitted))
}

/// Decode a picture AT a target pixel size: the print path's
/// right-sizing in one call. A raster picture is fitted
/// within the target (aspect kept, never upscaled, the same
/// rule as everywhere else). An SVG renders AT the target:
/// vector art scales without loss, so the target is the
/// resolution the caller asked for, not a ceiling.
pub fn decode_at(bytes: &[u8], target_w: u32, target_h: u32) -> Result<Decoded, ImageError> {
    let format = gate(sniff(bytes))?;
    if format == PictureFormat::Svg {
        let p = probe(bytes)?;
        let scale =
            (target_w.max(1) as f64 / p.width as f64).min(target_h.max(1) as f64 / p.height as f64);
        let w = ((p.width as f64 * scale).round() as u32).max(1);
        let h = ((p.height as f64 * scale).round() as u32).max(1);
        return render_svg(bytes, w, h);
    }
    decode_fitted(bytes, target_w, target_h)
}

/// Downscale to fit within the box, aspect kept; a picture
/// already inside the box is returned untouched (nothing in
/// this crate ever upscales).
pub fn fit_within(img: image::DynamicImage, max_w: u32, max_h: u32) -> image::DynamicImage {
    if img.width() <= max_w && img.height() <= max_h {
        return img;
    }
    let (w, h) = fit_dims(img.width(), img.height(), max_w, max_h);
    img.resize(w, h, image::imageops::FilterType::Triangle)
}

fn fit_dims(w: u32, h: u32, max_w: u32, max_h: u32) -> (u32, u32) {
    if w == 0 || h == 0 {
        return (1, 1);
    }
    let scale = (max_w as f64 / w as f64)
        .min(max_h as f64 / h as f64)
        .min(1.0);
    (
        ((w as f64 * scale).round() as u32).max(1),
        ((h as f64 * scale).round() as u32).max(1),
    )
}

fn svg_options() -> resvg::usvg::Options<'static> {
    // System fonts, loaded once per process, so an SVG with
    // words in it renders its words the way the office's
    // other programs do. Path-only art (the clipart case)
    // never touches this.
    static FONTDB: std::sync::OnceLock<std::sync::Arc<resvg::usvg::fontdb::Database>> =
        std::sync::OnceLock::new();
    let db = FONTDB.get_or_init(|| {
        let mut db = resvg::usvg::fontdb::Database::new();
        db.load_system_fonts();
        std::sync::Arc::new(db)
    });
    let mut opt = resvg::usvg::Options::default();
    *opt.fontdb_mut() = (**db).clone();
    opt
}

fn parse_svg(bytes: &[u8]) -> Result<resvg::usvg::Tree, ImageError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| ImageError::Unreadable("the SVG is not text we can read".into()))?;
    resvg::usvg::Tree::from_str(text, &svg_options())
        .map_err(|e| ImageError::Unreadable(format!("SVG: {e}")))
}

/// Render an SVG at exactly `w` x `h` pixels.
pub fn render_svg(bytes: &[u8], w: u32, h: u32) -> Result<Decoded, ImageError> {
    let tree = parse_svg(bytes)?;
    let w = w.clamp(1, 16_384);
    let h = h.clamp(1, 16_384);
    if (w as u64) * (h as u64) > 100_000_000 {
        return Err(ImageError::Unreadable(
            "that SVG asks for a picture over 100 megapixels".into(),
        ));
    }
    let size = tree.size();
    let sx = w as f32 / size.width().max(1.0);
    let sy = h as f32 / size.height().max(1.0);
    let mut pixmap = resvg::tiny_skia::Pixmap::new(w, h)
        .ok_or_else(|| ImageError::Unreadable("cannot make room to draw the SVG".into()))?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(sx, sy),
        &mut pixmap.as_mut(),
    );
    let rgba = pixmap.take();
    let has_alpha = rgba.chunks_exact(4).any(|px| px[3] < 255);
    Ok(Decoded {
        width: w,
        height: h,
        rgba,
        has_alpha,
    })
}

/// Encode decoded pixels as a JPEG at the given quality
/// (1-100): the print path's photo encoding. JPEG has no
/// transparency, so the alpha channel is dropped here;
/// callers who must keep it use the pixels directly.
pub fn encode_jpeg(decoded: &Decoded, quality: u8) -> Result<Vec<u8>, ImageError> {
    let rgb: Vec<u8> = decoded
        .rgba
        .chunks_exact(4)
        .flat_map(|px| [px[0], px[1], px[2]])
        .collect();
    let mut out = Vec::new();
    let mut encoder =
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, quality.clamp(1, 100));
    encoder
        .encode(
            &rgb,
            decoded.width,
            decoded.height,
            image::ExtendedColorType::Rgb8,
        )
        .map_err(|e| ImageError::Unreadable(format!("JPEG encode: {e}")))?;
    Ok(out)
}

/// Encode decoded pixels as a PNG (the clip library's
/// thumbnail cache and the WMF conversion landing spot).
pub fn encode_png(decoded: &Decoded) -> Result<Vec<u8>, ImageError> {
    let img = image::RgbaImage::from_raw(decoded.width, decoded.height, decoded.rgba.clone())
        .ok_or_else(|| ImageError::Unreadable("pixel buffer does not match its size".into()))?;
    let mut out = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut out);
    use image::ImageEncoder as _;
    encoder
        .write_image(
            img.as_raw(),
            decoded.width,
            decoded.height,
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|e| ImageError::Unreadable(format!("PNG encode: {e}")))?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid_png(w: u32, h: u32, rgba: [u8; 4]) -> Vec<u8> {
        let img = image::RgbaImage::from_pixel(w, h, image::Rgba(rgba));
        let d = Decoded {
            width: w,
            height: h,
            rgba: img.into_raw(),
            has_alpha: rgba[3] < 255,
        };
        encode_png(&d).expect("encode png")
    }

    fn solid_jpeg(w: u32, h: u32) -> Vec<u8> {
        let img = image::RgbImage::from_pixel(w, h, image::Rgb([200, 40, 40]));
        let mut out = Vec::new();
        let enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 90);
        use image::ImageEncoder as _;
        enc.write_image(img.as_raw(), w, h, image::ExtendedColorType::Rgb8)
            .expect("encode jpeg");
        out
    }

    #[test]
    fn sniff_names_the_common_formats() {
        assert_eq!(sniff(&solid_png(4, 4, [1, 2, 3, 255])), PictureFormat::Png);
        assert_eq!(sniff(&solid_jpeg(4, 4)), PictureFormat::Jpeg);
        assert_eq!(sniff(b"GIF89a...."), PictureFormat::Gif);
        assert_eq!(sniff(b"RIFF\x00\x00\x00\x00WEBPVP8 "), PictureFormat::WebP);
        assert_eq!(
            sniff(b"<svg xmlns=\"http://www.w3.org/2000/svg\"></svg>"),
            PictureFormat::Svg
        );
        assert_eq!(
            sniff(b"<?xml version=\"1.0\"?><svg xmlns=\"http://www.w3.org/2000/svg\"/>"),
            PictureFormat::Svg
        );
        assert_eq!(sniff(&[0xFF, 0x0A, 0x12, 0x34]), PictureFormat::JpegXl);
        // An AVIF ftyp box.
        let mut avif = b"\x00\x00\x00\x18".to_vec();
        avif.extend_from_slice(b"ftypavif");
        assert_eq!(sniff(&avif), PictureFormat::Avif);
        let mut heic = b"\x00\x00\x00\x18".to_vec();
        heic.extend_from_slice(b"ftypheic");
        assert_eq!(sniff(&heic), PictureFormat::Heic);
        assert_eq!(sniff(b"plain text, honestly"), PictureFormat::Unknown);
    }

    #[test]
    fn avif_and_heic_are_refused_as_convert_first() {
        let mut avif = b"\x00\x00\x00\x18".to_vec();
        avif.extend_from_slice(b"ftypavif");
        avif.extend_from_slice(&[0; 64]);
        assert_eq!(
            decode(&avif),
            Err(ImageError::ConvertFirst(PictureFormat::Avif))
        );
        assert_eq!(
            probe(&avif),
            Err(ImageError::ConvertFirst(PictureFormat::Avif))
        );
        let mut heic = b"\x00\x00\x00\x18".to_vec();
        heic.extend_from_slice(b"ftypheic");
        heic.extend_from_slice(&[0; 64]);
        assert_eq!(
            decode(&heic),
            Err(ImageError::ConvertFirst(PictureFormat::Heic))
        );
    }

    #[test]
    fn png_and_jpeg_round_trip_through_decode() {
        let png = solid_png(30, 12, [10, 20, 30, 255]);
        let d = decode(&png).expect("decode png");
        assert_eq!((d.width, d.height), (30, 12));
        assert!(!d.has_alpha);
        let p = probe(&png).expect("probe png");
        assert_eq!((p.width, p.height, p.format), (30, 12, PictureFormat::Png));

        let jpg = solid_jpeg(17, 9);
        let d = decode(&jpg).expect("decode jpeg");
        assert_eq!((d.width, d.height), (17, 9));
    }

    #[test]
    fn fitted_decode_never_upscales() {
        let png = solid_png(40, 20, [0, 0, 0, 255]);
        let d = decode_fitted(&png, 10, 10).expect("fitted");
        assert_eq!((d.width, d.height), (10, 5));
        let d = decode_fitted(&png, 400, 400).expect("fitted big");
        assert_eq!((d.width, d.height), (40, 20));
    }

    #[test]
    fn exif_orientation_turns_a_phone_photo_upright() {
        // Build a 6x4 JPEG whose EXIF says "rotate 90 CW"
        // (orientation 6): the decoded picture must come
        // back 4x6, standing up.
        let mut jpg = solid_jpeg(6, 4);
        // Insert an APP1 EXIF segment right after the SOI.
        let tiff: &[u8] = b"II\x2A\x00\x08\x00\x00\x00\x01\x00\x12\x01\x03\x00\x01\x00\x00\x00\x06\x00\x00\x00\x00\x00\x00\x00";
        let mut seg = vec![0xFF, 0xE1, 0x00, 0x00];
        let len = (tiff.len() + 8) as u16; // "Exif\0\0" + tiff + the 2 length bytes
        seg[2] = (len >> 8) as u8;
        seg[3] = (len & 0xFF) as u8;
        seg.extend_from_slice(b"Exif\x00\x00");
        seg.extend_from_slice(tiff);
        jpg.splice(2..2, seg);
        let d = decode(&jpg).expect("decode oriented jpeg");
        assert_eq!((d.width, d.height), (4, 6), "orientation 6 swaps the sides");
    }

    const SQUARE_SVG: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="50" viewBox="0 0 100 50"><rect width="100" height="50" fill="#204060"/></svg>"##;

    #[test]
    fn svg_probes_and_renders_at_the_asked_size() {
        let p = probe(SQUARE_SVG).expect("probe svg");
        assert_eq!((p.width, p.height, p.format), (100, 50, PictureFormat::Svg));
        let d = render_svg(SQUARE_SVG, 200, 100).expect("render svg");
        assert_eq!((d.width, d.height), (200, 100));
        assert!(!d.has_alpha);
        // The middle pixel is the rect's color.
        let mid = ((50 * 200 + 100) * 4) as usize;
        assert_eq!(&d.rgba[mid..mid + 3], &[0x20, 0x40, 0x60]);
        let fitted = decode_fitted(SQUARE_SVG, 50, 50).expect("fitted svg");
        assert_eq!((fitted.width, fitted.height), (50, 25));
    }

    #[test]
    fn a_jxl_file_decodes() {
        // A real JPEG XL codestream: pq_gradient.jxl from
        // the libjxl testdata repository (MIT/BSD-style
        // test data), 137 bytes.
        let bytes = include_bytes!("../tests/fixtures/pq_gradient.jxl");
        let p = probe(bytes).expect("probe jxl");
        assert_eq!(p.format, PictureFormat::JpegXl);
        assert!(p.width > 0 && p.height > 0);
        let d = decode(bytes).expect("decode jxl");
        assert_eq!((d.width, d.height), (p.width, p.height));
        assert_eq!(d.rgba.len(), (p.width * p.height * 4) as usize);
    }
}
