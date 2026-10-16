//! Splitting an engraver's page SVG into per-system SVGs,
//! and measuring the staff in what was actually drawn.
//!
//! Verovio draws a whole page into one SVG: a defs block of
//! glyph paths, then one `<g class="system">` subtree per
//! line of music. The bulletin wants one picture per line,
//! so this module cuts the page at system boundaries: each
//! system keeps the shared defs and its own viewBox, cut
//! slightly beyond its own ink and never into the next
//! system. The ink extent is measured from a real
//! rasterization (resvg, through ob-image), because tempo
//! marks and dynamics reach above and below the staff and
//! only ink knows where they end.
//!
//! It also MEASURES the staff height in the render (the
//! grid distance between the outer staff lines) so the
//! legibility floor is checked against the drawing, not
//! against what was asked for (music-spike.md: a legal
//! render can come back smaller than requested).

/// What one engraved line of music carries, counted from
/// the SVG the engraver drew.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SvgCounts {
    pub noteheads: u32,
    pub measures: u32,
    pub syllables: u32,
    pub fermatas: u32,
}

/// One system (one line of music) cut out of a page render,
/// ready to be a picture in a bulletin.
#[derive(Debug, Clone, PartialEq)]
pub struct SystemSvg {
    /// The standalone SVG document for this system.
    pub svg: String,
    /// Its width in the engraver's page units (the unit the
    /// pageWidth request was made in; 72 of them to the
    /// inch at scale 100).
    pub width_units: f64,
    /// Its height in the same units (crop height / 10).
    pub height_units: f64,
    /// The staff height measured in this render, in mm at
    /// the frame width the caller named.
    pub staff_mm: f64,
    pub counts: SvgCounts,
}

/// A start or end tag located in the source string.
struct Tag {
    start: usize,
    end: usize,
    name: String,
    /// The class attribute's tokens, if any.
    classes: Vec<String>,
    closing: bool,
    self_closing: bool,
}

/// Tokenize just enough XML: find the tags, respecting
/// quoted attribute values. Verovio's output is machine
/// XML; nothing here needs a general parser.
fn tags(s: &str) -> Vec<Tag> {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] != b'<' {
            i += 1;
            continue;
        }
        // Comments, declarations, processing instructions:
        // skip to their end.
        if s[i..].starts_with("<!--") {
            match s[i..].find("-->") {
                Some(k) => i += k + 3,
                None => break,
            }
            continue;
        }
        if s[i..].starts_with("<!") || s[i..].starts_with("<?") {
            match s[i..].find('>') {
                Some(k) => i += k + 1,
                None => break,
            }
            continue;
        }
        let start = i;
        let mut j = i + 1;
        let mut quoted = false;
        while j < b.len() {
            match b[j] {
                b'"' => quoted = !quoted,
                b'>' if !quoted => break,
                _ => {}
            }
            j += 1;
        }
        if j >= b.len() {
            break;
        }
        let inner = &s[start + 1..j];
        let closing = inner.starts_with('/');
        let body = inner.trim_start_matches('/').trim();
        let name: String = body
            .chars()
            .take_while(|c| !c.is_whitespace() && *c != '/' && *c != '>')
            .collect();
        let classes = class_tokens(inner);
        out.push(Tag {
            start,
            end: j + 1,
            name,
            classes,
            closing,
            self_closing: inner.trim_end().ends_with('/'),
        });
        i = j + 1;
    }
    out
}

fn class_tokens(tag_inner: &str) -> Vec<String> {
    let Some(at) = tag_inner.find("class=\"") else {
        return Vec::new();
    };
    let rest = &tag_inner[at + 7..];
    let Some(end) = rest.find('"') else {
        return Vec::new();
    };
    rest[..end].split_whitespace().map(str::to_string).collect()
}

fn attr_value(tag_inner: &str, key: &str) -> Option<String> {
    // Match ` key="` with a space before the name, so a key
    // like "d" cannot match the tail of "id".
    let pat = format!(" {key}=\"");
    let at = tag_inner.find(&pat)?;
    let rest = &tag_inner[at + pat.len()..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// The (x, y) of a translate() in a transform attribute.
fn translate_of(tag_inner: &str) -> (f64, f64) {
    let Some(t) = attr_value(tag_inner, "transform") else {
        return (0.0, 0.0);
    };
    let Some(inner) = t
        .strip_prefix("translate(")
        .and_then(|t| t.split(')').next())
    else {
        return (0.0, 0.0);
    };
    let nums: Vec<f64> = inner
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|p| !p.is_empty())
        .filter_map(|p| p.parse().ok())
        .collect();
    (
        nums.first().copied().unwrap_or(0.0),
        nums.get(1).copied().unwrap_or(0.0),
    )
}

/// (min_y, max_y) of the horizontal staff lines inside one
/// staff group, from its `M x y L x y` path data.
fn staff_line_span(d_attrs: &[String]) -> Option<(f64, f64)> {
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    for d in d_attrs {
        let nums: Vec<f64> = d
            .split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-'))
            .filter(|p| !p.is_empty() && *p != "-")
            .filter_map(|p| p.parse().ok())
            .collect();
        if nums.len() == 4 {
            let (x1, y1, x2, y2) = (nums[0], nums[1], nums[2], nums[3]);
            if (y1 - y2).abs() < 0.5 && (x2 - x1).abs() > 100.0 {
                lo = lo.min(y1);
                hi = hi.max(y1);
            }
        }
    }
    if lo <= hi {
        Some((lo, hi))
    } else {
        None
    }
}

/// Count the drawn things in one system's subtree.
/// Verovio classes: notehead groups, measure groups,
/// lyric syllables are `syl` groups, fermata groups.
fn count_system(subtree: &str) -> SvgCounts {
    let mut counts = SvgCounts::default();
    for tag in tags(subtree) {
        if tag.closing {
            continue;
        }
        for class in &tag.classes {
            match class.as_str() {
                "notehead" => counts.noteheads += 1,
                "measure" => counts.measures += 1,
                "syl" => counts.syllables += 1,
                "fermata" => counts.fermatas += 1,
                _ => {}
            }
        }
    }
    counts
}

/// Cut one engraved page SVG into one SVG per system, each
/// measured. `frame_width_mm` is the width the systems
/// will be placed at; the measured staff height is in mm
/// at that width.
pub fn split_systems(page_svg: &str, frame_width_mm: f64) -> Result<Vec<SystemSvg>, String> {
    let all = tags(page_svg);
    // The page frame: the nested definition-scale svg holds
    // the viewBox in grid units (10 per page unit).
    let defscale = all
        .iter()
        .find(|t| t.name == "svg" && t.classes.iter().any(|c| c == "definition-scale"))
        .ok_or("the engraver's page has no drawing area (no definition-scale)")?;
    let defscale_text = &page_svg[defscale.start..defscale.end];
    let viewbox =
        attr_value(defscale_text, "viewBox").ok_or("the engraver's page names no viewBox")?;
    let vb: Vec<f64> = viewbox
        .split_whitespace()
        .filter_map(|p| p.parse().ok())
        .collect();
    if vb.len() != 4 || vb[2] <= 0.0 || vb[3] <= 0.0 {
        return Err("the engraver's page has a viewBox we cannot read".to_string());
    }
    let (vx, vy, vw, vh) = (vb[0], vb[1], vb[2], vb[3]);
    // The defs block rides into every system verbatim.
    let defs_open = all
        .iter()
        .find(|t| t.name == "defs" && !t.closing)
        .ok_or("the engraver's page has no glyph definitions")?;
    let defs_close = all
        .iter()
        .find(|t| t.name == "defs" && t.closing && t.start > defs_open.start)
        .ok_or("the engraver's glyph definitions do not close")?;
    let defs = &page_svg[defs_open.start..defs_close.end];
    // The optional page description.
    let desc = all
        .iter()
        .find(|t| t.name == "desc" && !t.closing)
        .and_then(|open| {
            all.iter()
                .find(|t| t.name == "desc" && t.closing && t.start > open.start)
                .map(|close| &page_svg[open.start..close.end])
        });
    // The page-margin group wraps the systems; its
    // transform shifts everything inside it.
    let margin = all
        .iter()
        .find(|t| t.name == "g" && t.classes.iter().any(|c| c == "page-margin"))
        .ok_or("the engraver's page has no page margin group")?;
    let margin_text = &page_svg[margin.start..margin.end];
    let (_tx, ty) = translate_of(margin_text);

    // System subtrees and their staff spans, in order.
    struct Sys {
        start: usize,
        end: usize,
        staff_top: f64,    // root grid coords
        staff_bottom: f64, // root grid coords
    }
    let mut systems: Vec<Sys> = Vec::new();
    let mut staff_heights: Vec<f64> = Vec::new();
    let mut i = 0;
    while i < all.len() {
        let t = &all[i];
        if !t.closing && t.name == "g" && t.classes.iter().any(|c| c == "system") {
            // Walk to the matching close, collecting staff
            // groups and their line paths on the way.
            let mut depth = 0usize;
            let mut j = i;
            let (mut top, mut bottom) = (f64::INFINITY, f64::NEG_INFINITY);
            let mut in_staff = false;
            let mut staff_paths: Vec<String> = Vec::new();
            loop {
                let u = &all[j];
                if !u.closing && !u.self_closing {
                    depth += 1;
                    if u.name == "g" && u.classes.iter().any(|c| c == "staff") {
                        in_staff = true;
                        staff_paths.clear();
                    }
                }
                if in_staff && u.name == "path" {
                    if let Some(d) = attr_value(&page_svg[u.start..u.end], "d") {
                        staff_paths.push(d);
                    }
                }
                if u.closing {
                    if u.name == "g" && in_staff {
                        if let Some((lo, hi)) = staff_line_span(&staff_paths) {
                            staff_heights.push(hi - lo);
                            top = top.min(lo);
                            bottom = bottom.max(hi);
                        }
                        in_staff = false;
                    }
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        break;
                    }
                }
                j += 1;
                if j >= all.len() {
                    break;
                }
            }
            if top <= bottom {
                systems.push(Sys {
                    start: t.start,
                    end: all[j.min(all.len() - 1)].end,
                    staff_top: top + ty,
                    staff_bottom: bottom + ty,
                });
            }
            i = j + 1;
        } else {
            i += 1;
        }
    }
    if systems.is_empty() {
        return Err("the engraver drew no lines of music on that page".to_string());
    }
    staff_heights.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let staff_grid = staff_heights
        .get(staff_heights.len() / 2)
        .copied()
        .unwrap_or(720.0);
    let staff_mm = staff_grid / vw * frame_width_mm;

    // Ink rows from a real rasterization, to cut each
    // system just beyond its own ink.
    let ink = ink_rows(page_svg, vw, vh);

    let mut out = Vec::new();
    for (idx, sys) in systems.iter().enumerate() {
        let lower = if idx == 0 {
            vy
        } else {
            (systems[idx - 1].staff_bottom + sys.staff_top) / 2.0
        };
        let upper = if idx + 1 == systems.len() {
            vy + vh
        } else {
            (sys.staff_bottom + systems[idx + 1].staff_top) / 2.0
        };
        // Ink band for this system: rows whose grid center
        // falls inside the system's half of the page.
        let mut ink_span: Option<(f64, f64)> = None;
        if let Some((rows, grid_per_row, row0_grid)) = &ink {
            for (r, has) in rows.iter().enumerate() {
                if !has {
                    continue;
                }
                let center = row0_grid + (r as f64 + 0.5) * grid_per_row;
                if center > lower && center < upper {
                    let top_r = row0_grid + r as f64 * grid_per_row;
                    let bot_r = top_r + grid_per_row;
                    ink_span = Some(match ink_span {
                        None => (top_r, bot_r),
                        Some((t, b)) => (t.min(top_r), b.max(bot_r)),
                    });
                }
            }
        }
        let (mut crop_top, mut crop_bottom) = match ink_span {
            Some((t, b)) => (t - 0.30 * staff_grid, b + 0.30 * staff_grid),
            None => (
                sys.staff_top - 1.2 * staff_grid,
                sys.staff_bottom + 2.2 * staff_grid,
            ),
        };
        // Never reach into a neighbor's half, and never cut
        // the staff itself.
        crop_top = crop_top.max(lower).min(sys.staff_top - 0.15 * staff_grid);
        crop_bottom = crop_bottom
            .min(upper)
            .max(sys.staff_bottom + 0.15 * staff_grid);
        let crop_h = (crop_bottom - crop_top).max(1.0);
        let subtree = &page_svg[sys.start..sys.end];
        let counts = count_system(subtree);
        let w_px = vw / 10.0;
        let h_px = crop_h / 10.0;
        let svg = format!(
            "<svg width=\"{w_px:.1}px\" height=\"{h_px:.1}px\" version=\"1.1\" \
             xmlns=\"http://www.w3.org/2000/svg\" \
             xmlns:xlink=\"http://www.w3.org/1999/xlink\" overflow=\"visible\">\
             {desc}{defs}\
             <svg class=\"definition-scale\" color=\"black\" font-family=\"Times, serif\" \
             viewBox=\"{vx:.1} {crop_top:.1} {vw:.1} {crop_h:.1}\">\
             {margin_text}{subtree}</g></svg></svg>",
            desc = desc.unwrap_or(""),
        );
        out.push(SystemSvg {
            svg,
            width_units: vw / 10.0,
            height_units: crop_h / 10.0,
            staff_mm,
            counts,
        });
    }
    Ok(out)
}

/// Rasterize the page small and report which rows hold ink:
/// (rows top to bottom, grid units per row, grid y of the
/// first row's top). None when the rasterizer cannot draw
/// it; callers fall back to staff arithmetic.
fn ink_rows(page_svg: &str, vw: f64, vh: f64) -> Option<(Vec<bool>, f64, f64)> {
    let target_w = 640u32;
    let target_h = ((target_w as f64 * vh / vw).round() as u32).clamp(64, 12_000);
    let img = ob_image::render_svg(page_svg.as_bytes(), target_w, target_h).ok()?;
    let (w, h) = (img.width as usize, img.height as usize);
    let mut rows = vec![false; h];
    for (y, row) in img.rgba.chunks_exact(w * 4).enumerate() {
        rows[y] = row.chunks_exact(4).any(|px| px[3] > 12);
    }
    Some((rows, vh / h as f64, 0.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal Verovio-shaped page: one staff, five lines,
    /// drawn by hand in the spike render's structure.
    fn page_with(systems: &str) -> String {
        format!(
            "<svg width=\"1000px\" height=\"2000px\" version=\"1.1\" \
             xmlns=\"http://www.w3.org/2000/svg\" \
             xmlns:xlink=\"http://www.w3.org/1999/xlink\" overflow=\"visible\">\
             <desc>test</desc><defs><path id=\"E050\" d=\"M0 0\"/></defs>\
             <svg class=\"definition-scale\" viewBox=\"0 0 10000 20000\">\
             <g class=\"page-margin\" transform=\"translate(0, 0)\">{systems}</g>\
             </svg></svg>"
        )
    }

    fn system(y0: i32, heads: usize) -> String {
        let mut lines = String::new();
        for k in 0..5 {
            let y = y0 + k * 180;
            lines.push_str(&format!("<path d=\"M0 {y} L9000 {y}\" stroke=\"black\"/>"));
        }
        let mut heads_s = String::new();
        for _ in 0..heads {
            heads_s.push_str("<g class=\"notehead\"><path d=\"M1 1\"/></g>");
        }
        format!(
            "<g class=\"system\"><g class=\"measure\" id=\"m1\">\
             <g class=\"staff\">{lines}</g>\
             <g class=\"layer\">{heads_s}</g></g></g>"
        )
    }

    #[test]
    fn one_system_page_splits_and_measures() {
        let page = page_with(&system(1000, 3));
        let out = split_systems(&page, 129.7).expect("split");
        assert_eq!(out.len(), 1);
        // Staff grid height is 4 x 180 = 720 of 10000 wide:
        // 720/10000 x 129.7mm = 9.34mm.
        assert!((out[0].staff_mm - 9.34).abs() < 0.05, "{}", out[0].staff_mm);
        assert_eq!(out[0].counts.noteheads, 3);
        assert_eq!(out[0].counts.measures, 1);
        assert!(out[0].svg.contains("class=\"system\""));
        // The viewBox is cut around the staff, not the page.
        assert!(out[0].height_units < 2000.0);
        assert!(out[0].height_units > 50.0);
    }

    #[test]
    fn garbage_page_refuses() {
        assert!(split_systems("<svg></svg>", 100.0).is_err());
        assert!(split_systems("not svg at all", 100.0).is_err());
    }
}
