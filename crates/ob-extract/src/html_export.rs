//! HTML export: the web bulletin (H1, docs/planning/
//! beyond-print.md section 2).
//!
//! One self-contained `.html` file: the bulletin's
//! CONTENT in reading order, as real text a browser, a
//! search box, and a screen reader can all work with.
//! Pictures ride inside the file (rasters transcoded at
//! the web purpose's sizing; SVG art inlined as vector),
//! and the credits the rights records owe are composed
//! at the end. It is a reading edition, not the print
//! layout in a browser: panels and folding do not
//! translate, and the page says so in its own first line.
//!
//! Two as-built notes against the sketch:
//! - A placed picture shows WHOLE on the web page. The
//!   Fill crop is a paper decision about one printed
//!   size; the reading edition hides none of the picture.
//! - Hymn and music credit lines are already words in
//!   the bulletin (the music draft writes them under the
//!   music), so the Credits section composes only what
//!   the placed pictures owe, and never duplicates a
//!   credit line the document already carries as text.

use base64::Engine as _;
use ob_model::{
    resolve_fields, resolve_runs, Asset, Document, Frame, FrameKind, LicenseKind, Paragraph,
    ParagraphStyle, Run,
};
use std::collections::{HashMap, HashSet};

/// What the export carried, and what it could not, in
/// counts the doors state next to the saved file: nothing
/// is dropped silently.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct HtmlExportReport {
    /// Pictures embedded in the page (raster data URIs
    /// and inlined SVGs alike).
    pub pictures: usize,
    /// Picture boxes whose picture could not be carried
    /// (no picture put in yet, bytes missing from the
    /// file, or a format this export cannot read). Each
    /// one is a plain note in the page where it belongs.
    pub missing_pictures: usize,
    /// Lines in the Credits section (composed picture
    /// credits, plus the missing-credit note when one is
    /// owed and cannot be composed).
    pub credits: usize,
}

/// The reading-edition sentence, the page's first line
/// under its header, in the bulletin's own words.
const READING_NOTE: &str = "This is the reading edition of the bulletin: all the words in order, made to be read and searched on a screen. The printed bulletin folds its pages for paper, so the order here follows the reading, not the folding.";

/// The note a picture box leaves when its picture cannot
/// be carried.
const MISSING_NOTE: &str = "A picture belongs here; it is missing from the bulletin file.";

/// The note the Credits section carries when a placed
/// picture owes a credit nobody recorded enough of to
/// compose.
const MISSING_CREDIT: &str = "A picture in this bulletin is missing its credit.";

/// The page's one stylesheet: system fonts, a comfortable
/// measure, dark when the reader's system is dark, and a
/// print rule that simply lets it paginate. Nothing here
/// is load-bearing: with the style ignored entirely, the
/// semantic order still reads top to bottom.
const PAGE_CSS: &str = "\
:root{color-scheme:light dark}\
body{font-family:system-ui,-apple-system,\"Segoe UI\",Roboto,\"Helvetica Neue\",Arial,sans-serif;\
font-size:18px;line-height:1.5;max-width:42em;margin:0 auto;padding:1em 1em 3em}\
h1{font-size:1.7em;line-height:1.2;margin:.4em 0 .2em}\
h2{font-size:1.35em;line-height:1.25;margin:1.1em 0 .3em}\
h3{font-size:1.15em;line-height:1.3;margin:1.1em 0 .3em}\
p{margin:.45em 0}\
header .occasion,header .time{margin:.1em 0}\
.reading-note{font-style:italic}\
.line{display:flex;justify-content:space-between;gap:1em}\
.line .who{text-align:right;white-space:nowrap}\
figure{margin:1em 0}\
figure img,figure svg{max-width:100%;height:auto}\
.missing{border:1px dashed currentColor;padding:.6em .8em}\
.credits{margin-top:2.5em;border-top:1px solid;padding-top:.5em;font-size:.85em}\
@media (prefers-color-scheme:dark){body{background:#141414;color:#f0f0f0}a{color:#8ecbff}}\
@media print{body{max-width:none}}";

/// Escape text for HTML, in content and in attributes
/// alike: & < > " all become entities, so words from the
/// bulletin (a "<3" in an announcement) can never turn
/// into markup.
fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}

/// Which heading level each style id wears, for the
/// styles that are headings at all: an explicit level in
/// the name wins ("Heading 2", "subheading"), otherwise
/// the document's heading styles rank by size, largest
/// first, h1 down to h3.
fn heading_levels(doc: &Document) -> HashMap<String, u8> {
    let mut map: HashMap<String, u8> = HashMap::new();
    let mut unnumbered: Vec<&ParagraphStyle> = Vec::new();
    for st in &doc.styles {
        let id_l = st.id.to_ascii_lowercase();
        let name_l = st.name.to_ascii_lowercase();
        if !id_l.contains("heading") && !name_l.contains("heading") {
            continue;
        }
        match explicit_level(&id_l).or_else(|| explicit_level(&name_l)) {
            Some(level) => {
                map.insert(st.id.clone(), level);
            }
            None => unnumbered.push(st),
        }
    }
    unnumbered.sort_by_key(|s| std::cmp::Reverse(s.font_size));
    for (i, st) in unnumbered.iter().enumerate() {
        map.insert(st.id.clone(), ((i as u8) + 1).min(3));
    }
    map
}

/// A level stated in the style's own name: "subheading"
/// is 2, "heading" alone is 1, and a digit after the
/// heading word ("heading-2", "Heading 3") is itself.
fn explicit_level(name_l: &str) -> Option<u8> {
    if name_l.contains("subheading") || name_l.contains("sub-heading") {
        return Some(2);
    }
    let pos = name_l.find("heading")?;
    let rest = &name_l[pos + "heading".len()..];
    let rest = rest.trim_start_matches(|c: char| !c.is_ascii_digit());
    if let Some(d) = rest.chars().next() {
        return d.to_digit(10).map(|n| (n as u8).clamp(1, 3));
    }
    if rest.is_empty() {
        return Some(1);
    }
    None
}

/// One paragraph's resolved runs as HTML: each run wears
/// its effective face (its own switches over the style's),
/// bold as <strong>, italic as <em>, both nested.
fn runs_html(runs: &[Run], style: Option<&ParagraphStyle>) -> String {
    let mut out = String::new();
    for run in runs {
        if run.text.is_empty() {
            continue;
        }
        let bold = run.bold.unwrap_or(style.map(|s| s.bold).unwrap_or(false));
        let italic = run
            .italic
            .unwrap_or(style.map(|s| s.italic).unwrap_or(false));
        let t = esc(&run.text);
        match (bold, italic) {
            (true, true) => {
                out.push_str("<strong><em>");
                out.push_str(&t);
                out.push_str("</em></strong>");
            }
            (true, false) => {
                out.push_str("<strong>");
                out.push_str(&t);
                out.push_str("</strong>");
            }
            (false, true) => {
                out.push_str("<em>");
                out.push_str(&t);
                out.push_str("</em>");
            }
            (false, false) => out.push_str(&t),
        }
    }
    out
}

/// Split a resolved run list at its first tab: the two
/// halves of a tabbed line (the order-of-service line and
/// its hymn number, the reading and its reference). Tabs
/// past the first become spaces, as they do on paper.
fn split_tab(runs: &[Run]) -> Option<(Vec<Run>, Vec<Run>)> {
    let mut what: Vec<Run> = Vec::new();
    let mut who: Vec<Run> = Vec::new();
    let mut found = false;
    for run in runs {
        if found {
            who.push(Run {
                text: run.text.replace('\t', " "),
                bold: run.bold,
                italic: run.italic,
            });
        } else if let Some(pos) = run.text.find('\t') {
            found = true;
            let (a, b) = run.text.split_at(pos);
            if !a.is_empty() {
                what.push(Run {
                    text: a.to_string(),
                    bold: run.bold,
                    italic: run.italic,
                });
            }
            let rest = b[1..].replace('\t', " ");
            if !rest.is_empty() {
                who.push(Run {
                    text: rest,
                    bold: run.bold,
                    italic: run.italic,
                });
            }
        } else {
            what.push(run.clone());
        }
    }
    found.then_some((what, who))
}

/// One paragraph as one HTML block: a heading when its
/// style is one, a paired line when its words carry a
/// tab, a plain paragraph otherwise. Paragraphs with no
/// words at all leave nothing behind.
fn paragraph_html(
    doc: &Document,
    para: &Paragraph,
    headings: &HashMap<String, u8>,
    out: &mut String,
) {
    let style = doc.style(&para.style);
    let resolved = resolve_runs(&para.runs, doc);
    let text: String = resolved.iter().map(|r| r.text.as_str()).collect();
    if text.trim().is_empty() {
        return;
    }
    if let Some(level) = headings.get(&para.style) {
        out.push_str(&format!("<h{level}>"));
        out.push_str(&runs_html(&resolved, style));
        out.push_str(&format!("</h{level}>\n"));
        return;
    }
    if let Some((what, who)) = split_tab(&resolved) {
        out.push_str("<p class=\"line\"><span class=\"what\">");
        out.push_str(&runs_html(&what, style));
        out.push_str("</span> <span class=\"who\">");
        out.push_str(&runs_html(&who, style));
        out.push_str("</span></p>\n");
        return;
    }
    out.push_str("<p>");
    out.push_str(&runs_html(&resolved, style));
    out.push_str("</p>\n");
}

/// The name a screen reader speaks for one placed
/// picture: a music system's hymn, a clip's title, a
/// photo's title, in that order of knowledge. None when
/// nobody has named the picture at all.
fn picture_name(doc: &Document, frame: &Frame, asset: Option<&Asset>) -> Option<String> {
    if let Some(music) = &frame.music {
        let title = asset.and_then(|a| nonempty(&a.rights.title)).or_else(|| {
            doc.assets
                .iter()
                .find(|a| a.sha256 == music.source)
                .and_then(|a| nonempty(&a.rights.title))
        });
        return Some(match title {
            Some(t) => format!("Music: {t}"),
            None => "Sheet music".to_string(),
        });
    }
    if let Some(a) = asset {
        if let Some(t) = nonempty(&a.rights.title) {
            return Some(t);
        }
        // A picked picture's record names its file as the
        // source; the file's stem is the closest thing to
        // a name it has.
        let src = a.rights.source.trim();
        if src.contains('.') && !src.contains(char::is_whitespace) {
            let stem = src.rsplit(['/', '\\']).next().unwrap_or(src);
            let stem = stem.rsplit_once('.').map(|(s, _)| s).unwrap_or(stem);
            if !stem.is_empty() {
                return Some(stem.to_string());
            }
        }
    }
    None
}

fn nonempty(s: &str) -> Option<String> {
    let t = s.trim();
    if t.is_empty() {
        None
    } else {
        Some(t.to_string())
    }
}

/// Take the active content out of an SVG we are about to
/// inline: script elements, event-handler attributes,
/// and javascript: links. The art itself is untouched;
/// this is a safety net for files the office was handed,
/// not a judgment on them.
fn sanitize_svg(s: &str) -> String {
    let mut s = s.to_string();
    // Script elements, paired or self-closing.
    loop {
        let lower = s.to_ascii_lowercase();
        let Some(start) = lower.find("<script") else {
            break;
        };
        let after = &lower[start..];
        let Some(gt) = after.find('>') else {
            // An unterminated tag: drop it and everything
            // after it; nothing past it can be trusted.
            s.truncate(start);
            break;
        };
        let end = if after[..=gt].ends_with("/>") {
            // Self-closing: the tag is the whole element.
            start + gt + 1
        } else if let Some(close) = after.find("</script>") {
            start + close + "</script>".len()
        } else {
            // An opener with no closer: drop the tag.
            start + gt + 1
        };
        s.replace_range(start..end, "");
    }
    strip_handler_attributes(&s)
}

/// Walk the markup tag by tag, dropping attributes whose
/// name starts with "on" and javascript: href values.
/// Text between tags passes through untouched.
fn strip_handler_attributes(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = String::with_capacity(s.len());
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'<' {
            // Copy the tag, filtering its attributes.
            let mut j = i + 1;
            let mut quote: Option<u8> = None;
            while j < bytes.len() {
                match quote {
                    Some(q) if bytes[j] == q => quote = None,
                    Some(_) => {}
                    None if bytes[j] == b'"' || bytes[j] == b'\'' => quote = Some(bytes[j]),
                    None if bytes[j] == b'>' => break,
                    None => {}
                }
                j += 1;
            }
            let tag = &s[i..(j + 1).min(s.len())];
            out.push_str(&clean_tag(tag));
            i = (j + 1).min(s.len());
        } else {
            let next = s[i..].find('<').map(|p| i + p).unwrap_or(s.len());
            out.push_str(&s[i..next]);
            i = next;
        }
    }
    out
}

/// One tag with its handler attributes removed and any
/// javascript: href pointed nowhere.
fn clean_tag(tag: &str) -> String {
    // Split the tag into whitespace-separated pieces,
    // keeping quoted values whole.
    let mut pieces: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut quote: Option<char> = None;
    for c in tag.chars() {
        match quote {
            Some(q) => {
                cur.push(c);
                if c == q {
                    quote = None;
                }
            }
            None if c == '"' || c == '\'' => {
                quote = Some(c);
                cur.push(c);
            }
            None if c.is_whitespace() => {
                if !cur.is_empty() {
                    pieces.push(std::mem::take(&mut cur));
                }
            }
            None => cur.push(c),
        }
    }
    if !cur.is_empty() {
        pieces.push(cur);
    }
    let kept: Vec<String> = pieces
        .into_iter()
        .filter(|p| {
            let name = p.split('=').next().unwrap_or("").to_ascii_lowercase();
            !(name.starts_with("on")
                && name.len() > 2
                && name[2..].chars().all(|c| c.is_ascii_alphanumeric()))
        })
        .map(|p| {
            let lower = p.to_ascii_lowercase();
            if (lower.starts_with("href=") || lower.starts_with("xlink:href="))
                && lower.contains("javascript:")
            {
                let attr = p.split('=').next().unwrap_or("href");
                format!("{attr}=\"#\"")
            } else {
                p
            }
        })
        .collect();
    kept.join(" ")
}

/// Inline an SVG asset as markup: prolog and doctype
/// off, active content stripped, and a spoken name
/// (role="img", aria-label, and a <title> child) on the
/// root element, so the art is crisp at any zoom and a
/// screen reader can say what it is.
fn inline_svg(bytes: &[u8], name: &str) -> Option<String> {
    let text = std::str::from_utf8(bytes).ok()?;
    let text = text.trim_start_matches('\u{feff}');
    let text = &text[text.find("<svg")?..];
    let text = sanitize_svg(text);
    // The root tag ends at the first '>' outside quotes.
    let b = text.as_bytes();
    let mut quote: Option<u8> = None;
    let mut end = None;
    for (i, &c) in b.iter().enumerate() {
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => {}
            None if c == b'"' || c == b'\'' => quote = Some(c),
            None if c == b'>' => {
                end = Some(i);
                break;
            }
            None => {}
        }
    }
    let end = end?;
    let (open, rest) = text.split_at(end + 1);
    let open = open.trim_end();
    let (open, self_closed) = match open.strip_suffix("/>") {
        Some(stem) => (format!("{stem}>"), true),
        None => (open.to_string(), false),
    };
    let mut out = String::with_capacity(text.len() + name.len() * 2 + 64);
    // role and label go just before the root tag's '>'.
    out.push_str(&open[..open.len() - 1]);
    out.push_str(&format!(" role=\"img\" aria-label=\"{}\"", esc(name)));
    out.push('>');
    if !self_closed {
        if !rest[..rest.len().min(256)].contains("<title") {
            out.push_str(&format!("<title>{}</title>", esc(name)));
        }
        out.push_str(rest);
    }
    Some(out)
}

/// The web purpose's picture sizing, the PDF path's own
/// rule (ob-pdf, purpose ForEmailOrWeb): the picture is
/// right-sized to its shown size at 150 pixels per inch
/// and never upscaled.
const WEB_PPI: f64 = 150.0;

/// One picture box as a figure: the picture embedded, or
/// the missing note in its place (counted either way).
/// Returns the asset that was actually carried, so the
/// credits pass knows what the page holds.
fn figure_html(
    doc: &Document,
    frame: &Frame,
    report: &mut HtmlExportReport,
    out: &mut String,
) -> Option<String> {
    let missing = |report: &mut HtmlExportReport, out: &mut String| {
        report.missing_pictures += 1;
        out.push_str(&format!("<p class=\"missing\">{MISSING_NOTE}</p>\n"));
        None
    };
    let Some(placement) = &frame.picture else {
        return missing(report, out);
    };
    let asset = doc.assets.iter().find(|a| a.sha256 == placement.asset);
    let Some(bytes) = doc.asset_data(&placement.asset) else {
        return missing(report, out);
    };
    let name = picture_name(doc, frame, asset);
    match ob_image::sniff(bytes) {
        ob_image::PictureFormat::Svg => {
            // Vector art inlines: crisp at any zoom, no
            // raster at all.
            if ob_image::probe(bytes).is_err() {
                return missing(report, out);
            }
            let spoken = name.clone().unwrap_or_else(|| "Picture".to_string());
            match inline_svg(bytes, &spoken) {
                Some(svg) => {
                    report.pictures += 1;
                    out.push_str("<figure>");
                    out.push_str(&svg);
                    out.push_str("</figure>\n");
                    Some(placement.asset.clone())
                }
                None => missing(report, out),
            }
        }
        format if format.is_raster() => {
            let Ok(probe) = ob_image::probe(bytes) else {
                return missing(report, out);
            };
            let (_, _, dw, dh) = ob_model::picture_shown_rect(
                placement.fit,
                probe.width as f64,
                probe.height as f64,
                frame.rect.w as f64 / 1000.0,
                frame.rect.h as f64 / 1000.0,
                placement.crop_x,
                placement.crop_y,
            );
            let k = ((dw / 72.0 * WEB_PPI) / probe.width as f64)
                .min((dh / 72.0 * WEB_PPI) / probe.height as f64)
                .min(1.0);
            let tw = ((probe.width as f64 * k).round() as u32).max(1);
            let th = ((probe.height as f64 * k).round() as u32).max(1);
            let Ok(decoded) = ob_image::decode_at(bytes, tw, th) else {
                return missing(report, out);
            };
            let (mime, data) = if decoded.has_alpha {
                match ob_image::encode_png(&decoded) {
                    Ok(d) => ("image/png", d),
                    Err(_) => return missing(report, out),
                }
            } else {
                match ob_image::encode_jpeg(&decoded, 85) {
                    Ok(d) => ("image/jpeg", d),
                    Err(_) => return missing(report, out),
                }
            };
            report.pictures += 1;
            out.push_str(&format!(
                "<figure><img src=\"data:{mime};base64,{}\" alt=\"{}\" width=\"{}\" height=\"{}\"></figure>\n",
                base64::engine::general_purpose::STANDARD.encode(&data),
                esc(name.as_deref().unwrap_or("")),
                decoded.width,
                decoded.height,
            ));
            Some(placement.asset.clone())
        }
        _ => missing(report, out),
    }
}

/// A license's canonical deed address, when the record
/// names a version: the address prints as visible text,
/// the printed-page rule carried to the web.
fn license_url(license: &LicenseKind) -> Option<String> {
    match license {
        LicenseKind::CcBy(v) if !v.trim().is_empty() => Some(format!(
            "https://creativecommons.org/licenses/by/{}/",
            v.trim()
        )),
        LicenseKind::CcBySa(v) if !v.trim().is_empty() => Some(format!(
            "https://creativecommons.org/licenses/by-sa/{}/",
            v.trim()
        )),
        _ => None,
    }
}

fn license_name(license: &LicenseKind) -> String {
    match license {
        LicenseKind::CcBy(v) => {
            let v = v.trim();
            if v.is_empty() {
                "CC BY".to_string()
            } else {
                format!("CC BY {v}")
            }
        }
        LicenseKind::CcBySa(v) => {
            let v = v.trim();
            if v.is_empty() {
                "CC BY-SA".to_string()
            } else {
                format!("CC BY-SA {v}")
            }
        }
        _ => String::new(),
    }
}

fn requires_credit(license: &LicenseKind) -> bool {
    matches!(license, LicenseKind::CcBy(_) | LicenseKind::CcBySa(_))
}

/// Compose the Credits lines for the assets the page
/// actually carries, per clipart-landscape section 5:
/// a recorded credit text is used verbatim (and skipped
/// when the bulletin already carries it as words, so a
/// hymn's credit is never printed twice); without one, a
/// line is composed from title, creator, the copyright
/// line, and the license, whose address shows as text.
/// An asset that owes a credit nobody recorded enough
/// of to compose yields the missing-credit note, once.
fn credit_lines(doc: &Document, carried: &[String], doc_text: &str) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut missing = false;
    let mut seen: HashSet<&str> = HashSet::new();
    for sha in carried {
        if !seen.insert(sha.as_str()) {
            continue;
        }
        let Some(asset) = doc.assets.iter().find(|a| &a.sha256 == sha) else {
            continue;
        };
        let rights = &asset.rights;
        if !requires_credit(&rights.license) {
            continue;
        }
        let url = license_url(&rights.license);
        if let Some(credit) = nonempty(&rights.credit_text) {
            if doc_text.contains(credit.trim()) {
                continue; // already words in the bulletin
            }
            let mut line = esc(&credit);
            if let Some(u) = &url {
                if !credit.contains("http") {
                    line.push_str(&format!(" <a href=\"{u}\">{u}</a>"));
                }
            }
            if !lines.contains(&line) {
                lines.push(line);
            }
            continue;
        }
        let mut parts: Vec<String> = Vec::new();
        if let Some(t) = nonempty(&rights.title) {
            parts.push(format!("&quot;{}&quot;", esc(&t)));
        }
        if let Some(c) = nonempty(&rights.creator) {
            parts.push(format!("by {}", esc(&c)));
        }
        if let Some(cp) = nonempty(&rights.copyright_line) {
            parts.push(esc(&cp));
        }
        if parts.is_empty() {
            missing = true;
            continue;
        }
        let name = license_name(&rights.license);
        match &url {
            Some(u) => parts.push(format!("{name} (<a href=\"{u}\">{u}</a>)")),
            None => parts.push(name),
        }
        if let Some(src) = &rights.source_url {
            if let Some(src) = nonempty(src) {
                parts.push(format!("Source: {}", esc(&src)));
            }
        }
        let line = format!("{}.", parts.join(", "));
        if !lines.contains(&line) {
            lines.push(line);
        }
    }
    if missing {
        lines.push(esc(MISSING_CREDIT));
    }
    lines
}

/// The document as one self-contained web page, with
/// the counts of what it carries and what it had to
/// leave as a note.
pub fn export_html(doc: &Document) -> Result<(Vec<u8>, HtmlExportReport), String> {
    let mut report = HtmlExportReport::default();
    let headings = heading_levels(doc);
    // Every story's resolved words, for the credits
    // pass's "already in the bulletin" check.
    let doc_text: String = doc
        .stories
        .iter()
        .flat_map(|s| &s.paragraphs)
        .map(|p| resolve_fields(&p.text(), doc))
        .collect::<Vec<_>>()
        .join("\n");

    let church = nonempty(&doc.profile.name).unwrap_or_else(|| doc.title.clone());
    let occasion_name = doc
        .fields
        .get("occasion_name")
        .and_then(|v| nonempty(v))
        .unwrap_or_default();
    let occasion_date = doc
        .fields
        .get("occasion_date_long")
        .and_then(|v| nonempty(v))
        .unwrap_or_default();
    let occasion_line = [occasion_name.clone(), occasion_date.clone()]
        .into_iter()
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join(", ");
    let page_title = if occasion_line.is_empty() {
        church.clone()
    } else {
        format!("{church} — {occasion_line}")
    };

    let mut body = String::new();
    let mut carried: Vec<String> = Vec::new();
    let mut seen_stories: HashSet<&str> = HashSet::new();
    for panel in &doc.panels {
        for frame in &panel.frames {
            match frame.kind {
                FrameKind::Picture => {
                    if let Some(sha) = figure_html(doc, frame, &mut report, &mut body) {
                        carried.push(sha);
                    }
                }
                FrameKind::Text => {
                    let Some(sid) = &frame.story_id else {
                        continue;
                    };
                    if !seen_stories.insert(sid.as_str()) {
                        continue; // a thread prints once, at its first box
                    }
                    let Some(story) = doc.story(sid) else {
                        continue;
                    };
                    for para in &story.paragraphs {
                        paragraph_html(doc, para, &headings, &mut body);
                    }
                }
                FrameKind::Shape => {
                    // A drawn rectangle is layout, not
                    // content; the reading edition has
                    // nothing to say for it.
                }
            }
        }
    }

    let credits = credit_lines(doc, &carried, &doc_text);
    report.credits = credits.len();

    let mut html = String::with_capacity(body.len() + 4096);
    html.push_str("<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n");
    html.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n");
    html.push_str(&format!("<title>{}</title>\n", esc(&page_title)));
    html.push_str(&format!("<style>{PAGE_CSS}</style>\n</head>\n<body>\n"));
    html.push_str("<header>\n");
    html.push_str(&format!("<h1 class=\"church\">{}</h1>\n", esc(&church)));
    if !occasion_line.is_empty() {
        html.push_str(&format!(
            "<p class=\"occasion\">{}</p>\n",
            esc(&occasion_line)
        ));
    }
    if let Some(time) = doc.occasion_time.as_deref().and_then(nonempty) {
        html.push_str(&format!("<p class=\"time\">{}</p>\n", esc(&time)));
    }
    html.push_str("</header>\n");
    html.push_str(&format!("<p class=\"reading-note\">{READING_NOTE}</p>\n"));
    html.push_str("<main>\n");
    html.push_str(&body);
    html.push_str("</main>\n");
    if !credits.is_empty() {
        html.push_str("<section class=\"credits\">\n<h2>Credits</h2>\n<ul>\n");
        for line in &credits {
            html.push_str(&format!("<li>{line}</li>\n"));
        }
        html.push_str("</ul>\n</section>\n");
    }
    html.push_str("</body>\n</html>\n");
    Ok((html.into_bytes(), report))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ob_model::{
        AssetKind, ContentRole, Frame, LicenseKind, MusicFrame, Paragraph, PicturePlacement,
        RightsRecord, Run,
    };

    fn png_bytes(w: u32, h: u32, rgba: [u8; 4]) -> Vec<u8> {
        let mut px = Vec::with_capacity((w * h * 4) as usize);
        for _ in 0..(w * h) {
            px.extend_from_slice(&rgba);
        }
        let decoded = ob_image::Decoded {
            width: w,
            height: h,
            rgba: px,
            has_alpha: rgba[3] < 255,
        };
        ob_image::encode_png(&decoded).expect("encode png")
    }

    fn picture_frame(id: &str, asset: &str, music: Option<MusicFrame>) -> Frame {
        Frame {
            id: id.to_string(),
            name: id.to_string(),
            kind: FrameKind::Picture,
            rect: ob_model::Rect::new(36_000, 400_000, 200_000, 150_000),
            story_id: None,
            role: ContentRole::Standing,
            picture: Some(PicturePlacement::new(asset.to_string())),
            music,
        }
    }

    const DOVE_SVG: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><path d="M1 5 L9 5 L5 9 Z"/></svg>"#;
    const MUSIC_SVG: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" width="120" height="30" viewBox="0 0 120 30"><g class="system"><path d="M0 15h120"/></g></svg>"#;

    /// The standard sample: the LetterBifold template for
    /// Grace Lutheran, dressed with a bold run, a <3, a
    /// photo, a CC BY clip, a CC BY-SA photo whose credit
    /// already rides as words, one music system, and one
    /// picture box whose picture is gone.
    fn sample_doc() -> Document {
        let start = chrono::NaiveDate::from_ymd_opt(2026, 10, 18).unwrap();
        let mut doc = ob_layout::build_template(
            ob_layout::Preset::LetterBifold,
            ob_model::ChurchProfile {
                name: "Grace Lutheran Church".into(),
                pastor: "Pastor Lee".into(),
                address: "123 Main Street".into(),
                service_times: "Sundays 9:00 AM".into(),
                website: "gracelutheran.example".into(),
                giving_url: String::new(),
            },
            start,
            "Twentieth Sunday after Pentecost",
        );
        doc.occasion_time = Some("9:00 AM".to_string());

        // The announcements: a bold run and a "<3" that
        // must survive as text, never as markup. Plus the
        // picnic credit, carried as words on purpose.
        for story in &mut doc.stories {
            for para in &mut story.paragraphs {
                if para
                    .text()
                    .contains("Replace with this week's announcements")
                {
                    *para = Paragraph::from_runs(
                        "body",
                        vec![
                            Run::plain("Potluck after the service. "),
                            Run {
                                text: "Bring a dish".into(),
                                bold: Some(true),
                                italic: None,
                            },
                            Run::plain(" to share. We love you <3"),
                        ],
                    );
                }
            }
        }
        doc.stories[2]
            .paragraphs
            .push(Paragraph::plain("body", "Picnic photo by Lee."));

        let photo = png_bytes(60, 40, [200, 40, 40, 255]);
        let photo_hash = doc.add_asset(
            &photo,
            AssetKind::Picture,
            "image/jpeg",
            RightsRecord {
                title: "Church picnic".into(),
                license: LicenseKind::ChurchOwned,
                ..Default::default()
            },
        );
        // Semi-transparent, so it must ride as a PNG; its
        // credit already appears as words in the bulletin.
        let picnic = png_bytes(30, 20, [40, 120, 200, 128]);
        let picnic_hash = doc.add_asset(
            &picnic,
            AssetKind::Picture,
            "image/png",
            RightsRecord {
                title: "Picnic".into(),
                credit_text: "Picnic photo by Lee.".into(),
                license: LicenseKind::CcBy("4.0".into()),
                ..Default::default()
            },
        );
        let dove_hash = doc.add_asset(
            DOVE_SVG,
            AssetKind::Picture,
            "image/svg+xml",
            RightsRecord {
                title: "Dove".into(),
                creator: "Jane Artist".into(),
                license: LicenseKind::CcBy("4.0".into()),
                ..Default::default()
            },
        );
        let chancel = png_bytes(20, 20, [90, 90, 30, 255]);
        let chancel_hash = doc.add_asset(
            &chancel,
            AssetKind::Picture,
            "image/png",
            RightsRecord {
                title: "Chancel".into(),
                credit_text: "Photo by Sam Church, used under CC BY-SA 4.0.".into(),
                license: LicenseKind::CcBySa("4.0".into()),
                ..Default::default()
            },
        );
        let musicxml: &[u8] = b"<score-partwise></score-partwise>";
        let music_hash = doc.add_asset(
            musicxml,
            AssetKind::Music,
            "application/vnd.recordare.musicxml+xml",
            RightsRecord {
                title: "A Mighty Fortress Is Our God".into(),
                license: LicenseKind::LicensedReprint,
                ..Default::default()
            },
        );
        let system_hash = doc.add_asset(
            MUSIC_SVG,
            AssetKind::Picture,
            "image/svg+xml",
            RightsRecord {
                title: "A Mighty Fortress Is Our God".into(),
                license: LicenseKind::LicensedReprint,
                ..Default::default()
            },
        );

        doc.panels[0]
            .frames
            .push(picture_frame("photo", &photo_hash, None));
        doc.panels[0]
            .frames
            .push(picture_frame("picnic", &picnic_hash, None));
        doc.panels[1].frames.push(picture_frame(
            "music-1",
            &system_hash,
            Some(MusicFrame {
                system: 0,
                staff_mm: 6.5,
                source: music_hash,
            }),
        ));
        doc.panels[2]
            .frames
            .push(picture_frame("gone", &"0".repeat(64), None));
        doc.panels[3]
            .frames
            .push(picture_frame("dove", &dove_hash, None));
        doc.panels[3]
            .frames
            .push(picture_frame("chancel", &chancel_hash, None));
        doc
    }

    #[test]
    fn the_web_bulletin_carries_words_pictures_and_credits() {
        let doc = sample_doc();
        let (bytes, report) = export_html(&doc).expect("export");
        let html = String::from_utf8(bytes).expect("utf8");

        // The page frame: one file, no scripts, in English.
        assert!(html.starts_with("<!doctype html>"), "{html:.200}");
        assert!(html.contains("<html lang=\"en\">"));
        assert!(!html.contains("<script"), "no scripts anywhere");
        assert!(html.contains("<style>"), "one embedded stylesheet");
        assert!(
            html.contains(
                "<title>Grace Lutheran Church — Twentieth Sunday after Pentecost, October 18, 2026</title>"
            ),
            "title from church + occasion: {html:.400}"
        );
        assert!(html.contains("<p class=\"time\">9:00 AM</p>"));
        assert!(
            html.contains("This is the reading edition of the bulletin: all the words in order, made to be read and searched on a screen."),
            "the reading-edition sentence leads the page"
        );

        // Words: fields resolved, runs honored, tabs paired.
        assert!(html.contains("Sermon: Pastor Lee"), "resolved sermon words");
        assert!(html.contains("<strong>Bring a dish</strong>"));
        assert!(
            html.contains(
                "<span class=\"what\">Hymn of Praise</span> <span class=\"who\">432</span>"
            ),
            "the tabbed line keeps its two halves paired"
        );
        assert!(
            html.contains("<h2><strong>Order of Service</strong></h2>"),
            "a heading style's own bold folds into its words"
        );
        assert!(
            html.contains("We love you &lt;3"),
            "the heart survives as text"
        );
        assert!(!html.contains("<3"), "never as markup");

        // Pictures: rasters as data URIs (JPEG opaque, PNG
        // with real transparency), SVGs inlined with names.
        assert!(
            html.contains("src=\"data:image/jpeg;base64,"),
            "photo embedded"
        );
        assert!(
            html.contains("src=\"data:image/png;base64,"),
            "alpha picture embedded"
        );
        assert!(html.contains("alt=\"Church picnic\""));
        assert!(html.contains("aria-label=\"Dove\""));
        assert!(html.contains("<title>Dove</title>"));
        assert!(html.contains("aria-label=\"Music: A Mighty Fortress Is Our God\""));

        // The missing picture is a note in place, counted.
        assert!(html.contains("A picture belongs here; it is missing from the bulletin file."));
        assert_eq!(report.missing_pictures, 1);
        assert_eq!(report.pictures, 5);

        // Credits: composed for the dove (title, creator,
        // license address as visible text), verbatim for
        // the chancel photo (its recorded line, plus the
        // license address it did not carry), and the
        // picnic credit NOT repeated: it already rides as
        // words in the bulletin, so it appears exactly once.
        assert!(html.contains("<h2>Credits</h2>"));
        assert!(html.contains("Jane Artist"));
        assert!(html.contains(
            "CC BY 4.0 (<a href=\"https://creativecommons.org/licenses/by/4.0/\">https://creativecommons.org/licenses/by/4.0/</a>)"
        ));
        assert!(html.contains("Photo by Sam Church, used under CC BY-SA 4.0."));
        assert!(html.contains(
            "<a href=\"https://creativecommons.org/licenses/by-sa/4.0/\">https://creativecommons.org/licenses/by-sa/4.0/</a>"
        ));
        assert_eq!(html.matches("Picnic photo by Lee.").count(), 1);
        assert_eq!(report.credits, 2);

        // The only http strings on the page: the two SVGs'
        // xmlns (one each) and the two license addresses
        // (href + visible text each).
        assert_eq!(html.matches("http").count(), 6, "{html}");
        for (at, _) in html.match_indices("href=\"") {
            let rest = &html[at + 6..];
            assert!(
                rest.starts_with("https://creativecommons.org/licenses/"),
                "every link is a license address: {rest:.60}"
            );
        }
        assert!(!html.contains("src=\"http"), "no external src");
        assert!(!html.contains("<link"), "no external link tags");
    }

    #[test]
    fn an_unrecorded_credit_says_so() {
        let mut doc = sample_doc();
        let art = png_bytes(10, 10, [10, 10, 10, 255]);
        let hash = doc.add_asset(
            &art,
            AssetKind::Picture,
            "image/png",
            RightsRecord {
                license: LicenseKind::CcBy("4.0".into()),
                ..Default::default()
            },
        );
        doc.panels[3]
            .frames
            .push(picture_frame("mystery", &hash, None));
        let (bytes, report) = export_html(&doc).expect("export");
        let html = String::from_utf8(bytes).expect("utf8");
        assert!(html.contains("A picture in this bulletin is missing its credit."));
        assert_eq!(report.credits, 3);
    }

    #[test]
    fn inlined_svg_loses_its_scripts_and_handlers() {
        let mut doc = sample_doc();
        let evil: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" onload="evil()" viewBox="0 0 10 10"><script>alert("x")</script><rect width="10" height="10" onclick="evil()"/></svg>"#;
        let hash = doc.add_asset(
            evil,
            AssetKind::Picture,
            "image/svg+xml",
            RightsRecord {
                title: "Seal".into(),
                license: LicenseKind::PublicDomain,
                ..Default::default()
            },
        );
        doc.panels[3]
            .frames
            .push(picture_frame("seal", &hash, None));
        let (bytes, _) = export_html(&doc).expect("export");
        let html = String::from_utf8(bytes).expect("utf8");
        assert!(html.contains("aria-label=\"Seal\""), "the art itself stays");
        assert!(!html.contains("<script"));
        assert!(!html.contains("alert"));
        assert!(!html.contains("onload"));
        assert!(!html.contains("onclick"));
        assert!(!html.contains("evil"));
    }

    #[test]
    fn heading_levels_follow_names_then_sizes() {
        let doc = sample_doc();
        let levels = heading_levels(&doc);
        assert_eq!(levels.get("heading"), Some(&1));
        assert_eq!(levels.get("subheading"), Some(&2));
        assert!(!levels.contains_key("body"));
        assert!(!levels.contains_key("church-name"));
    }
}
