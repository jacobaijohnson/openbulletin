//! Image fingerprints, version `img-fp-v1` (see
//! [`crate::IMAGE_FP_VERSION`]).
//!
//! The algorithm, exactly, so that a fingerprint computed
//! today compares only with fingerprints computed the same
//! way, and so that an independent implementation can
//! reproduce these bits:
//!
//! 1. Alpha is composited over white, in integer math:
//!    `c' = (c*a + 255*(255-a) + 127) / 255` per channel.
//!    A transparent pixel is white, the way bulletin art is
//!    viewed on a white page.
//! 2. Grayscale is Rec.601, in integer math:
//!    `y = (299*R + 587*G + 114*B + 500) / 1000`.
//! 3. Downscale is a box average: target cell (tx, ty) of a
//!    Tw x Th grid averages the source box
//!    `[tx*S/Tw, (tx+1)*S/Tw) x [ty*H/Th, (ty+1)*H/Th)`
//!    with integer bounds; an empty box (upscaling a tiny
//!    image) falls back to the nearest source pixel.
//! 4. dHash: downscale to 9x8; bit (row, col) is 1 when the
//!    left pixel is brighter than its right neighbour;
//!    bit index is `row*8 + col`.
//! 5. pHash: downscale to 32x32; orthonormal DCT-II; keep
//!    the top-left 8x8 coefficients; round each coefficient
//!    to 6 decimal places BEFORE thresholding (without the
//!    rounding, near-degenerate images hash on floating
//!    point dust, and two correct implementations disagree);
//!    median is `(sorted[31] + sorted[32]) / 2` over the 64
//!    coefficients including DC; bit is 1 when the
//!    coefficient is strictly above the median; bit index
//!    is `row*8 + col`.
//! 6. Colour histogram: 4x4x4 RGB bins over the composited
//!    (not grayscale) pixels, raw counts. It is a GUARD,
//!    not a match key: two pictures whose histograms are
//!    grossly different are not the same picture, however
//!    close their grayscale hashes land. That is what
//!    stops a gold cross matching a purple Lent cross.
//! 7. Low-information flag: the 32x32 downscale holds 8 or
//!    fewer distinct gray values, or its mean absolute
//!    horizontal gradient is under 2.0. A simple cross or
//!    dove outline on white has very few bits of real
//!    signal, so many different simple icons land within a
//!    few bits of each other; the flag does not suppress
//!    the proposal, it changes the wording to "look
//!    before you merge".
//!
//! What these hashes do NOT survive, stated so no user is
//! misled: cropping, mirroring, rotation, and added
//! borders. A cropped, flipped, or framed copy of a
//! picture will usually NOT be flagged. The Duplicates
//! view is not proof there are no other copies.

use ob_image::Decoded;

/// A picture's fingerprint: two 64-bit perceptual hashes,
/// a colour guard, its dimensions, and the low-information
/// flag. About a hundred bytes; the picture itself is
/// never stored here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageFingerprint {
    pub fp_version: &'static str,
    pub phash: u64,
    pub dhash: u64,
    /// 4x4x4 RGB bins, index `(r/64)*16 + (g/64)*4 + (b/64)`.
    pub hist: [u32; 64],
    pub width: u32,
    pub height: u32,
    pub low_information: bool,
}

/// The image bands (research §4.2, literature starting
/// points, not calibrated on church art): both hashes must
/// agree, and the colour guard must pass, at every band.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageBand {
    /// Both Hamming distances at most 5.
    VeryLikely,
    /// Both Hamming distances at most 10.
    Possibly,
    /// No band: not surfaced as a probable match.
    NoMatch,
}

/// Total-variation distance between two colour
/// histograms, 0.0 (identical) to 1.0 (nothing shared).
/// A gross difference is a veto, not a score: the
/// threshold is [`COLOUR_VETO`].
pub fn hist_distance(a: &[u32; 64], b: &[u32; 64]) -> f64 {
    let mut diff: u64 = 0;
    let mut total: u64 = 0;
    for i in 0..64 {
        diff += a[i].abs_diff(b[i]) as u64;
        total += a[i] as u64;
    }
    if total == 0 {
        return 0.0;
    }
    diff as f64 / (2.0 * total as f64)
}

/// Colour histograms further apart than this veto the
/// match at every band: a recoloured picture is a
/// different picture, however close its grayscale hashes.
pub const COLOUR_VETO: f64 = 0.5;

/// Hamming distance between two 64-bit hashes: the count
/// of differing bits, 0 to 64. Unrelated pictures sit
/// near 32; the interesting action is all under 12.
pub fn hamming(a: u64, b: u64) -> u32 {
    (a ^ b).count_ones()
}

/// The band for two fingerprints of the same version.
/// Different versions never match: they are incomparable,
/// and comparing them would be a silent wrong answer.
pub fn image_band(a: &ImageFingerprint, b: &ImageFingerprint) -> ImageBand {
    if a.fp_version != b.fp_version {
        return ImageBand::NoMatch;
    }
    if hist_distance(&a.hist, &b.hist) > COLOUR_VETO {
        return ImageBand::NoMatch;
    }
    let ph = hamming(a.phash, b.phash);
    let dh = hamming(a.dhash, b.dhash);
    if ph <= 5 && dh <= 5 {
        ImageBand::VeryLikely
    } else if ph <= 10 && dh <= 10 {
        ImageBand::Possibly
    } else {
        ImageBand::NoMatch
    }
}

/// Fingerprint one decoded picture. SVG art rasterizes
/// through ob-image first, so it arrives here as pixels
/// like any other picture; AVIF/HEIC never arrive, because
/// ob-image refuses them before decoding.
pub fn fingerprint_image(decoded: &Decoded) -> ImageFingerprint {
    let gray = grayscale(decoded);
    let small32 = downscale(&gray, decoded.width, decoded.height, 32, 32);
    let distinct = {
        let mut seen = [false; 256];
        for row in &small32 {
            for v in row {
                seen[*v as usize] = true;
            }
        }
        seen.iter().filter(|s| **s).count()
    };
    let mut grad_sum: u64 = 0;
    let mut grad_count: u64 = 0;
    for row in &small32 {
        for pair in row.windows(2) {
            grad_sum += pair[0].abs_diff(pair[1]) as u64;
            grad_count += 1;
        }
    }
    let mean_grad = grad_sum as f64 / grad_count.max(1) as f64;
    let low_information = distinct <= 8 || mean_grad < 2.0;
    ImageFingerprint {
        fp_version: crate::IMAGE_FP_VERSION,
        phash: phash(&gray, decoded.width, decoded.height),
        dhash: dhash(&gray, decoded.width, decoded.height),
        hist: colour_hist(decoded),
        width: decoded.width,
        height: decoded.height,
        low_information,
    }
}

fn composite_channel(c: u8, a: u8) -> u8 {
    let c = c as u32;
    let a = a as u32;
    ((c * a + 255 * (255 - a) + 127) / 255) as u8
}

/// Grayscale rows, one byte per pixel, Rec.601 integer
/// math over alpha-composited RGB.
fn grayscale(decoded: &Decoded) -> Vec<Vec<u8>> {
    let w = decoded.width as usize;
    let h = decoded.height as usize;
    let mut out = Vec::with_capacity(h);
    for y in 0..h {
        let mut row = Vec::with_capacity(w);
        for x in 0..w {
            let i = (y * w + x) * 4;
            let r = composite_channel(decoded.rgba[i], decoded.rgba[i + 3]);
            let g = composite_channel(decoded.rgba[i + 1], decoded.rgba[i + 3]);
            let b = composite_channel(decoded.rgba[i + 2], decoded.rgba[i + 3]);
            let yv = (299 * r as u32 + 587 * g as u32 + 114 * b as u32 + 500) / 1000;
            row.push(yv as u8);
        }
        out.push(row);
    }
    out
}

/// Box-average downscale to Tw x Th, integer boxes, with
/// a nearest-pixel fallback for empty boxes.
fn downscale(gray: &[Vec<u8>], sw: u32, sh: u32, tw: u32, th: u32) -> Vec<Vec<u8>> {
    let sw = sw as usize;
    let sh = sh as usize;
    let tw = tw as usize;
    let th = th as usize;
    let mut out = Vec::with_capacity(th);
    for ty in 0..th {
        let y0 = ty * sh / th;
        let y1 = (ty + 1) * sh / th;
        let mut row = Vec::with_capacity(tw);
        for tx in 0..tw {
            let x0 = tx * sw / tw;
            let x1 = (tx + 1) * sw / tw;
            if x1 <= x0 || y1 <= y0 {
                let px = gray[y0.min(sh - 1)][x0.min(sw - 1)];
                row.push(px);
            } else {
                let mut sum: u32 = 0;
                let mut n: u32 = 0;
                for row_slice in &gray[y0..y1] {
                    for px in &row_slice[x0..x1] {
                        sum += *px as u32;
                        n += 1;
                    }
                }
                row.push((sum / n) as u8);
            }
        }
        out.push(row);
    }
    out
}

fn dhash(gray: &[Vec<u8>], sw: u32, sh: u32) -> u64 {
    let small = downscale(gray, sw, sh, 9, 8);
    let mut h: u64 = 0;
    for (r, row) in small.iter().enumerate() {
        for (c, pair) in row.windows(2).enumerate() {
            if pair[0] > pair[1] {
                h |= 1 << (r * 8 + c);
            }
        }
    }
    h
}

fn dct_coeff(small: &[Vec<u8>], u: usize, v: usize) -> f64 {
    let n = 32.0_f64;
    let au = if u == 0 {
        (1.0 / n).sqrt()
    } else {
        (2.0 / n).sqrt()
    };
    let av = if v == 0 {
        (1.0 / n).sqrt()
    } else {
        (2.0 / n).sqrt()
    };
    let mut sum = 0.0_f64;
    for (y, row) in small.iter().enumerate() {
        let cy = ((2 * y + 1) as f64 * v as f64 * std::f64::consts::PI / (2.0 * n)).cos();
        for (x, px) in row.iter().enumerate() {
            let cx = ((2 * x + 1) as f64 * u as f64 * std::f64::consts::PI / (2.0 * n)).cos();
            sum += *px as f64 * cx * cy;
        }
    }
    au * av * sum
}

fn phash(gray: &[Vec<u8>], sw: u32, sh: u32) -> u64 {
    let small = downscale(gray, sw, sh, 32, 32);
    let mut coeffs = Vec::with_capacity(64);
    for v in 0..8 {
        for u in 0..8 {
            let c = dct_coeff(&small, u, v);
            // Round BEFORE thresholding: without this,
            // near-degenerate images hash on floating
            // point dust and two correct implementations
            // disagree. Ties (coeff == median) resolve
            // deterministically by the strict comparison.
            coeffs.push((c * 1_000_000.0).round() / 1_000_000.0);
        }
    }
    let mut sorted = coeffs.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = (sorted[31] + sorted[32]) / 2.0;
    let mut h: u64 = 0;
    for (i, c) in coeffs.iter().enumerate() {
        if *c > median {
            h |= 1 << i;
        }
    }
    h
}

fn colour_hist(decoded: &Decoded) -> [u32; 64] {
    let mut bins = [0u32; 64];
    let w = decoded.width as usize;
    let h = decoded.height as usize;
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) * 4;
            let r = composite_channel(decoded.rgba[i], decoded.rgba[i + 3]);
            let g = composite_channel(decoded.rgba[i + 1], decoded.rgba[i + 3]);
            let b = composite_channel(decoded.rgba[i + 2], decoded.rgba[i + 3]);
            bins[(r / 64) as usize * 16 + (g / 64) as usize * 4 + (b / 64) as usize] += 1;
        }
    }
    bins
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A Decoded picture built from a pixel function, so
    /// the vectors below need no fixture files.
    fn build<F: Fn(usize, usize) -> (u8, u8, u8, u8)>(w: usize, h: usize, f: F) -> Decoded {
        let mut rgba = Vec::with_capacity(w * h * 4);
        for y in 0..h {
            for x in 0..w {
                let (r, g, b, a) = f(x, y);
                rgba.extend_from_slice(&[r, g, b, a]);
            }
        }
        Decoded {
            width: w as u32,
            height: h as u32,
            rgba,
            has_alpha: false,
        }
    }

    fn solid(px: (u8, u8, u8, u8)) -> impl Fn(usize, usize) -> (u8, u8, u8, u8) {
        move |_, _| px
    }

    /// Vectors cross-checked against an independent Python
    /// implementation of the same documented algorithm
    /// (/tmp/fp_ref2.py, run 2026-11-03). The Python side
    /// and this side agree bit-for-bit on all ten images;
    /// the formula images carry the non-degenerate DCT
    /// cases, where the smallest coefficient-to-median
    /// margin is 0.044, far above float noise.
    #[test]
    fn vectors_match_independent_implementation() {
        let black = (0u8, 0u8, 0u8, 255u8);
        let white = (255u8, 255u8, 255u8, 255u8);
        let cases: Vec<(Decoded, u64, u64, bool)> = vec![
            (
                build(16, 16, solid(black)),
                0x0000000000000000,
                0x0000000000000000,
                true,
            ),
            (
                build(16, 16, solid(white)),
                0x0000000000000001,
                0x0000000000000000,
                true,
            ),
            (
                build(16, 16, |x, _| {
                    let v = (x * 255 / 15) as u8;
                    (v, v, v, 255)
                }),
                0x0000000000000001,
                0x0000000000000000,
                false,
            ),
            (
                build(16, 16, |x, _| if x < 8 { black } else { white }),
                0x0000000000000089,
                0x0000000000000000,
                true,
            ),
            (
                build(16, 16, |x, y| {
                    if (x / 2 + y / 2) % 2 == 0 {
                        black
                    } else {
                        white
                    }
                }),
                0x0000000000000001,
                0xa950a950a950a950,
                true,
            ),
            (
                build(32, 32, |x, y| {
                    let v = ((x * 7 + y * 13 + x * y * 3) % 256) as u8;
                    (v, v, v, 255)
                }),
                0xf9f9e8f8806070e9,
                0xc69da6acabedd6d8,
                false,
            ),
            (
                build(48, 32, |x, y| {
                    let v = ((x * 7 + y * 13 + x * y * 3) % 256) as u8;
                    (v, v, v, 255)
                }),
                0xfb7bc8fd48406813,
                0x13b4dabb9cf95254,
                false,
            ),
            (
                build(16, 16, |x, _| {
                    if x < 8 {
                        (255, 0, 0, 255)
                    } else {
                        (0, 0, 255, 255)
                    }
                }),
                0x0000000000000023,
                0x1010101010101010,
                true,
            ),
            (
                build(16, 16, solid((0, 0, 0, 0))),
                0x0000000000000001,
                0x0000000000000000,
                true,
            ),
            (
                build(16, 16, solid((255, 0, 0, 128))),
                0x0000000000000001,
                0x0000000000000000,
                true,
            ),
        ];
        for (d, ph, dh, low) in cases {
            let fp = fingerprint_image(&d);
            assert_eq!(fp.fp_version, crate::IMAGE_FP_VERSION);
            assert_eq!(fp.phash, ph, "phash for {}x{}", d.width, d.height);
            assert_eq!(fp.dhash, dh, "dhash for {}x{}", d.width, d.height);
            assert_eq!(
                fp.low_information, low,
                "low flag for {}x{}",
                d.width, d.height
            );
        }
    }

    #[test]
    fn colour_guard_vetoes_recolouring() {
        let red = build(16, 16, solid((255, 0, 0, 255)));
        let blue = build(16, 16, solid((0, 0, 255, 255)));
        let fr = fingerprint_image(&red);
        let fb = fingerprint_image(&blue);
        assert_eq!(hist_distance(&fr.hist, &fb.hist), 1.0);
        assert_eq!(image_band(&fr, &fb), ImageBand::NoMatch);
        assert_eq!(hist_distance(&fr.hist, &fr.hist), 0.0);
        assert_eq!(image_band(&fr, &fr), ImageBand::VeryLikely);
    }

    #[test]
    fn bands_need_both_hashes_and_same_version() {
        let base = ImageFingerprint {
            fp_version: crate::IMAGE_FP_VERSION,
            phash: 0,
            dhash: 0,
            hist: [0; 64],
            width: 16,
            height: 16,
            low_information: false,
        };
        let mut close = base.clone();
        close.phash = 0b11111;
        close.dhash = 0b11111;
        assert_eq!(image_band(&base, &close), ImageBand::VeryLikely);
        let mut far_one = base.clone();
        far_one.phash = 0b11111;
        far_one.dhash = 0b11111111111;
        assert_eq!(image_band(&base, &far_one), ImageBand::NoMatch);
        let mut other_version = base.clone();
        other_version.fp_version = "img-fp-v0";
        assert_eq!(image_band(&base, &other_version), ImageBand::NoMatch);
    }

    #[test]
    fn hamming_counts_bits() {
        assert_eq!(hamming(0, 0), 0);
        assert_eq!(hamming(0, u64::MAX), 64);
        assert_eq!(hamming(0b1010, 0b1001), 2);
    }
}
