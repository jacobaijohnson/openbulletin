//! The canvas editor's byte map (E1).
//!
//! A typing session edits the RAW story text (field tokens and
//! all), because the commit path stores raw text. The user,
//! however, sees and points at the RESOLVED text: values filled
//! in, tokens shown as chips, tabbed lines expanded into dot
//! leaders. Three coordinate spaces therefore exist for one
//! buffer:
//!
//! - **raw**: the '\n'-joined session buffer, tokens literal.
//! - **resolved**: tokens replaced by their values.
//! - **display**: resolved text with each tabbed paragraph
//!   replaced by its dot-leader expansion (what the canvas
//!   draws, and what caret offsets are measured in).
//!
//! [`BufferMap`] converts offsets and edit ranges between the
//! spaces. Tokens (chips) and dot-leader spans are ATOMIC in
//! display and resolved space: carets never stop inside them,
//! and an edit range that touches part of one grows to cover
//! it whole, so a token can be deleted or replaced but never
//! edited internally or half-deleted.
//!
//! Pure string math throughout: the only outside knowledge it
//! needs is resolution (ob_model) and, for the leader
//! expansion, a closure the caller supplies (it needs the
//! shaper, which lives in the app).

use ob_model::Document;
use std::ops::Range;

/// A field token seen in all three spaces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChipSpan {
    pub raw: Range<usize>,
    pub resolved: Range<usize>,
    pub display: Range<usize>,
    pub token: String,
}

/// One paragraph's three ranges (each excluding the '\n'
/// separator), plus its leader atomics when it is a tabbed
/// line: in resolved space the atomic covers the tab and the
/// whitespace the expansion trims; in display space it covers
/// the " ... " span.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ParaMap {
    raw: Range<usize>,
    resolved: Range<usize>,
    display: Range<usize>,
    leader_atomic_resolved: Option<Range<usize>>,
    leader_display: Option<Range<usize>>,
}

/// The full buffer map for one editing session's text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BufferMap {
    pub raw_text: String,
    pub resolved_text: String,
    pub display_text: String,
    pub chips: Vec<ChipSpan>,
    paras: Vec<ParaMap>,
}

/// Byte ranges of the paragraphs of a '\n'-joined string,
/// separators excluded. Matches `str::split('\n')` semantics:
/// a trailing separator yields a final empty paragraph.
fn para_ranges(text: &str) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    let mut start = 0usize;
    for (i, b) in text.bytes().enumerate() {
        if b == b'\n' {
            out.push(start..i);
            start = i + 1;
        }
    }
    out.push(start..text.len());
    out
}

impl BufferMap {
    /// Build the map for a session buffer. `expand` receives
    /// a paragraph's index and RESOLVED text and returns its
    /// display text when the paragraph is a tabbed
    /// (dot-leader) line, or None when the paragraph displays
    /// as resolved.
    pub fn build(
        raw_text: &str,
        doc: &Document,
        mut expand: impl FnMut(usize, &str) -> Option<String>,
    ) -> BufferMap {
        let raw_paras = para_ranges(raw_text);
        let mut resolved_text = String::new();
        let mut display_text = String::new();
        let mut chips: Vec<ChipSpan> = Vec::new();
        let mut paras: Vec<ParaMap> = Vec::new();
        for (pi, raw_range) in raw_paras.iter().enumerate() {
            if pi > 0 {
                resolved_text.push('\n');
                display_text.push('\n');
            }
            let raw_para = &raw_text[raw_range.clone()];
            let resolved = ob_model::resolve_fields_mapped(raw_para, doc);
            let resolved_start = resolved_text.len();
            resolved_text.push_str(&resolved.text);
            let resolved_range = resolved_start..resolved_text.len();

            let display_start = display_text.len();
            // The paragraph's display string: its expansion
            // when it is a leader line whose shape we can map,
            // else the resolved text itself.
            let expanded = expand(pi, &resolved.text);
            let (display_para, leader_atomic_resolved, leader_display) = match &expanded {
                Some(e) if *e != resolved.text => {
                    match leader_spans(&resolved.text, e, resolved_start, display_start) {
                        Some((a, d)) => (e.clone(), Some(a), Some(d)),
                        None => (resolved.text.clone(), None, None),
                    }
                }
                _ => (resolved.text.clone(), None, None),
            };
            display_text.push_str(&display_para);
            let display_range = display_start..display_text.len();
            let atomic_local = leader_atomic_resolved
                .as_ref()
                .map(|a| a.start - resolved_start..a.end - resolved_start);
            let leader_local = leader_display
                .as_ref()
                .map(|d| d.start - display_start..d.end - display_start);

            for span in &resolved.spans {
                let r = resolved_start + span.resolved.start..resolved_start + span.resolved.end;
                // The chip's display range: convert its two
                // ends (segment-aligned, never inside a leader).
                let d_start = resolved_point_to_display(
                    atomic_local.clone(),
                    leader_local.clone(),
                    r.start - resolved_start,
                ) + display_start;
                let d_end = resolved_point_to_display(
                    atomic_local.clone(),
                    leader_local.clone(),
                    r.end - resolved_start,
                ) + display_start;
                chips.push(ChipSpan {
                    raw: raw_range.start + span.raw.start..raw_range.start + span.raw.end,
                    resolved: r,
                    display: d_start..d_end,
                    token: span.token.clone(),
                });
            }
            paras.push(ParaMap {
                raw: raw_range.clone(),
                resolved: resolved_range,
                display: display_range,
                leader_atomic_resolved,
                leader_display,
            });
        }
        BufferMap {
            raw_text: raw_text.to_string(),
            resolved_text,
            display_text,
            chips,
            paras,
        }
    }

    /// Number of paragraphs (display, resolved, and raw
    /// paragraph counts always agree).
    pub fn para_count(&self) -> usize {
        self.paras.len()
    }

    /// The paragraph (index) a display offset belongs to: the
    /// first whose display range ends at or after the offset,
    /// so an offset exactly at a paragraph's first byte
    /// belongs to that paragraph, and one at its last byte
    /// position (before the '\n') belongs to it too.
    pub fn para_at_display(&self, d: usize) -> usize {
        self.paras
            .iter()
            .position(|p| d <= p.display.end)
            .unwrap_or(self.paras.len() - 1)
    }

    /// The display byte range of one paragraph's text.
    pub fn para_display_range(&self, para: usize) -> Range<usize> {
        self.paras[para].display.clone()
    }

    /// A paragraph's resolved range.
    pub fn para_resolved_range(&self, para: usize) -> Range<usize> {
        self.paras[para].resolved.clone()
    }

    /// The chip whose display range ends exactly at `d`
    /// (the atomic a Backspace at `d` would meet).
    pub fn atomic_ending_at_display(&self, d: usize) -> Option<Range<usize>> {
        for c in &self.chips {
            if c.display.end == d && !c.display.is_empty() {
                return Some(c.display.clone());
            }
        }
        for p in &self.paras {
            if let Some(l) = &p.leader_display {
                if l.end == d {
                    return Some(l.clone());
                }
            }
        }
        None
    }

    /// The chip whose display range starts exactly at `d`
    /// (the atomic a Delete at `d` would meet).
    pub fn atomic_starting_at_display(&self, d: usize) -> Option<Range<usize>> {
        for c in &self.chips {
            if c.display.start == d && !c.display.is_empty() {
                return Some(c.display.clone());
            }
        }
        for p in &self.paras {
            if let Some(l) = &p.leader_display {
                if l.start == d {
                    return Some(l.clone());
                }
            }
        }
        None
    }

    /// Snap a display offset out of any atomic it falls
    /// strictly inside (chips, leaders): caret stops are only
    /// ever at atomic boundaries or in plain text. Snaps to
    /// the nearer end.
    pub fn snap_display_caret(&self, d: usize) -> usize {
        for a in self.display_atomics() {
            if d > a.start && d < a.end {
                return if d - a.start <= a.end - d {
                    a.start
                } else {
                    a.end
                };
            }
        }
        d
    }

    /// Directional snap for caret movement: an offset strictly
    /// inside an atomic lands on its start when moving left,
    /// its end when moving right, so one keypress crosses a
    /// chip in one step.
    pub fn snap_display_caret_dir(&self, d: usize, forward: bool) -> usize {
        for a in self.display_atomics() {
            if d > a.start && d < a.end {
                return if forward { a.end } else { a.start };
            }
        }
        d
    }

    fn display_atomics(&self) -> Vec<Range<usize>> {
        let mut v: Vec<Range<usize>> = self.chips.iter().map(|c| c.display.clone()).collect();
        for p in &self.paras {
            if let Some(l) = &p.leader_display {
                v.push(l.clone());
            }
        }
        v.sort_by_key(|r| r.start);
        v
    }

    fn resolved_atomics(&self) -> Vec<Range<usize>> {
        let mut v: Vec<Range<usize>> = self.chips.iter().map(|c| c.resolved.clone()).collect();
        for p in &self.paras {
            if let Some(l) = &p.leader_atomic_resolved {
                v.push(l.clone());
            }
        }
        v.sort_by_key(|r| r.start);
        v
    }

    /// Grow a range until it covers whole every atomic it
    /// partially overlaps (atomics are disjoint and sorted;
    /// one pass suffices because growth only ever extends to
    /// atomic boundaries).
    fn grow_over(range: Range<usize>, atomics: &[Range<usize>]) -> Range<usize> {
        let mut r = range;
        for a in atomics {
            if r.start < a.end && r.end > a.start && !(r.start <= a.start && r.end >= a.end) {
                r.start = r.start.min(a.start);
                r.end = r.end.max(a.end);
            }
        }
        r
    }

    /// Convert a display offset to resolved space.
    pub fn display_to_resolved(&self, d: usize) -> usize {
        let pi = self.para_at_display(d);
        let p = &self.paras[pi];
        let local = d.saturating_sub(p.display.start);
        let out = match (&p.leader_atomic_resolved, &p.leader_display) {
            (Some(atomic), Some(leader)) => {
                let la = atomic.start - p.resolved.start;
                let lae = atomic.end - p.resolved.start;
                let ld = leader.start - p.display.start;
                let lde = leader.end - p.display.start;
                if local <= ld {
                    p.resolved.start + local
                } else if local >= lde {
                    p.resolved.start + lae + (local - lde)
                } else if local - ld <= lde - local {
                    p.resolved.start + la
                } else {
                    p.resolved.start + lae
                }
            }
            _ => p.resolved.start + local,
        };
        out.min(p.resolved.end)
    }

    /// Convert a resolved offset to display space.
    pub fn resolved_to_display(&self, r: usize) -> usize {
        let pi = self
            .paras
            .iter()
            .position(|p| r <= p.resolved.end)
            .unwrap_or(self.paras.len() - 1);
        let p = &self.paras[pi];
        let local = r.saturating_sub(p.resolved.start);
        let out = match (&p.leader_atomic_resolved, &p.leader_display) {
            (Some(atomic), Some(leader)) => {
                let la = atomic.start - p.resolved.start;
                let lae = atomic.end - p.resolved.start;
                let ld = leader.start - p.display.start;
                let lde = leader.end - p.display.start;
                if local <= la {
                    p.display.start + local
                } else if local >= lae {
                    p.display.start + lde + (local - lae)
                } else if local - la <= lae - local {
                    p.display.start + ld
                } else {
                    p.display.start + lde
                }
            }
            _ => p.display.start + local,
        };
        out.min(p.display.end)
    }

    /// Convert a resolved offset to raw space, walking the
    /// chip segments. Inside a chip, snap to the nearer end.
    pub fn resolved_to_raw(&self, r: usize) -> usize {
        let mut prev_res_end = 0usize;
        let mut prev_raw_end = 0usize;
        for c in &self.chips {
            if r < c.resolved.start {
                return prev_raw_end + (r - prev_res_end);
            }
            if r <= c.resolved.end {
                if r == c.resolved.start {
                    return c.raw.start;
                }
                if r == c.resolved.end {
                    return c.raw.end;
                }
                return if r - c.resolved.start <= c.resolved.end - r {
                    c.raw.start
                } else {
                    c.raw.end
                };
            }
            prev_res_end = c.resolved.end;
            prev_raw_end = c.raw.end;
        }
        prev_raw_end + (r - prev_res_end)
    }

    /// Convert a raw offset to resolved space (inverse of
    /// `resolved_to_raw`; inside a chip, snap to nearer end).
    pub fn raw_to_resolved(&self, raw: usize) -> usize {
        let mut prev_raw_end = 0usize;
        let mut prev_res_end = 0usize;
        for c in &self.chips {
            if raw < c.raw.start {
                return prev_res_end + (raw - prev_raw_end);
            }
            if raw <= c.raw.end {
                if raw == c.raw.start {
                    return c.resolved.start;
                }
                if raw == c.raw.end {
                    return c.resolved.end;
                }
                return if raw - c.raw.start <= c.raw.end - raw {
                    c.resolved.start
                } else {
                    c.resolved.end
                };
            }
            prev_raw_end = c.raw.end;
            prev_res_end = c.resolved.end;
        }
        prev_res_end + (raw - prev_raw_end)
    }

    /// Convert a raw offset all the way to display space.
    pub fn raw_to_display(&self, raw: usize) -> usize {
        self.resolved_to_display(self.raw_to_resolved(raw))
    }

    /// Grow a display range so it covers every atomic it
    /// overlaps even partially (word selection over a chip
    /// selects the whole chip).
    pub fn expand_display_range(&self, range: Range<usize>) -> Range<usize> {
        Self::grow_over(range, &self.display_atomics())
    }

    /// Translate an edit made in display space (replace
    /// `display_range` with `replacement`) into the raw-space
    /// splice that stores the same change. The range first
    /// grows over any chip or leader it touches, so tokens
    /// are replaced or deleted whole, never in part. The
    /// replacement is user text and is stored verbatim.
    pub fn translate_edit(
        &self,
        display_range: Range<usize>,
        replacement: &str,
    ) -> (Range<usize>, String) {
        let dr = Self::grow_over(display_range, &self.display_atomics());
        let r1 = self.display_to_resolved(dr.start);
        let r2 = self.display_to_resolved(dr.end);
        let rr = Self::grow_over(r1..r2, &self.resolved_atomics());
        let raw = self.resolved_to_raw(rr.start)..self.resolved_to_raw(rr.end);
        (raw, replacement.to_string())
    }
}

/// The leader atomics of one tabbed paragraph, in global
/// coordinates: (resolved atomic, display leader). The
/// expansion's shape (see Shaper::expand_order_line) is
/// `{left trimmed} {dots} {right trimmed}`, so the visible
/// left is a prefix of both texts and the visible right a
/// suffix of both. None when the display string does not have
/// that shape (the caller then displays the paragraph as
/// resolved, unmapped but honest).
fn leader_spans(
    resolved_para: &str,
    display_para: &str,
    resolved_start: usize,
    display_start: usize,
) -> Option<(Range<usize>, Range<usize>)> {
    let tab = resolved_para.find('\t')?;
    let left_res = &resolved_para[..tab];
    let left_vis = left_res.trim_end();
    let right_res = &resolved_para[tab + 1..];
    let right_vis = right_res.trim();
    let right_lead_ws = right_res.len() - right_res.trim_start().len();
    if !display_para.starts_with(left_vis) || !display_para.ends_with(right_vis) {
        return None;
    }
    let atomic_local = left_vis.len()..tab + 1 + right_lead_ws;
    let leader_local = left_vis.len()..display_para.len() - right_vis.len();
    Some((
        resolved_start + atomic_local.start..resolved_start + atomic_local.end,
        display_start + leader_local.start..display_start + leader_local.end,
    ))
}

/// Convert one resolved offset within a single paragraph to
/// its display offset, given the paragraph's leader atomics
/// in paragraph-local coordinates.
fn resolved_point_to_display(
    atomic_local: Option<Range<usize>>,
    leader_local: Option<Range<usize>>,
    local: usize,
) -> usize {
    match (atomic_local, leader_local) {
        (Some(a), Some(l)) => {
            if local <= a.start {
                local
            } else if local >= a.end {
                l.end + (local - a.end)
            } else if local - a.start <= a.end - local {
                l.start
            } else {
                l.end
            }
        }
        _ => local,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn test_doc() -> Document {
        let mut fields = BTreeMap::new();
        fields.insert("occasion_name".to_string(), "Pentecost".to_string());
        Document {
            format_version: ob_model::FORMAT_VERSION,
            title: "t".into(),
            profile: ob_model::ChurchProfile {
                name: "Grace".into(),
                pastor: "Pastor Lee".into(),
                ..Default::default()
            },
            fields,
            styles: vec![],
            stories: vec![],
            panels: vec![],
            sheet: ob_model::SheetSpec {
                preset: "letter-portrait".into(),
                size: ob_model::Size::new(0, 0),
            },
            no_announcements: false,
            assets: Vec::new(),
            asset_bytes: Default::default(),
            occasion_time: None,
            wanted_fonts: Vec::new(),
            participants: Default::default(),
            inserts: Default::default(),
            applied_contributions: Vec::new(),
        }
    }

    fn no_expand(_: usize, _: &str) -> Option<String> {
        None
    }

    fn fake_expand(_: usize, resolved: &str) -> Option<String> {
        resolved
            .split_once('\t')
            .map(|(l, r)| format!("{} ...... {}", l.trim_end(), r.trim()))
    }

    #[test]
    fn plain_text_is_identity_in_all_spaces() {
        let doc = test_doc();
        let m = BufferMap::build("Hello world.\nSecond para.", &doc, no_expand);
        assert_eq!(m.resolved_text, m.raw_text);
        assert_eq!(m.display_text, m.raw_text);
        assert!(m.chips.is_empty());
        assert_eq!(m.raw_to_display(20), 20);
        let (raw, rep) = m.translate_edit(6..11, "there");
        assert_eq!(raw, 6..11);
        assert_eq!(rep, "there");
    }

    #[test]
    fn typing_beside_a_chip_lands_in_the_right_raw_place() {
        let doc = test_doc();
        let raw = "Sermon: {Church.pastor} speaks";
        let m = BufferMap::build(raw, &doc, no_expand);
        assert_eq!(m.resolved_text, "Sermon: Pastor Lee speaks");
        assert_eq!(m.chips.len(), 1);
        let chip = m.chips[0].clone();
        assert_eq!(&raw[chip.raw.clone()], "{Church.pastor}");
        assert_eq!(&m.display_text[chip.display.clone()], "Pastor Lee");
        // Type at the very end: raw insertion goes after
        // "speaks", past the token's raw bytes.
        let end = m.display_text.len();
        let (rr, _) = m.translate_edit(end..end, "!");
        assert_eq!(rr, raw.len()..raw.len());
        // Type just before the chip: raw goes before the token.
        let (rr, _) = m.translate_edit(chip.display.start..chip.display.start, ">");
        assert_eq!(rr, chip.raw.start..chip.raw.start);
        // Type just after the chip: raw goes after the token.
        let (rr, _) = m.translate_edit(chip.display.end..chip.display.end, "<");
        assert_eq!(rr, chip.raw.end..chip.raw.end);
    }

    #[test]
    fn a_chip_is_deleted_or_replaced_whole_never_in_part() {
        let doc = test_doc();
        let raw = "Sermon: {Church.pastor} speaks";
        let m = BufferMap::build(raw, &doc, no_expand);
        let chip = m.chips[0].clone();
        // Select exactly the chip and delete: the whole raw
        // token goes.
        let (rr, rep) = m.translate_edit(chip.display.clone(), "");
        assert_eq!(rr, chip.raw);
        assert_eq!(rep, "");
        // Select HALF the chip plus a trailing word: the range
        // grows over the whole chip.
        let half = chip.display.start + 2..chip.display.end + 3;
        let (rr, _) = m.translate_edit(half, "X");
        assert_eq!(rr.start, chip.raw.start);
        assert!(rr.end > chip.raw.end, "growth covers past the token");
        // Typing over the chip replaces the token with text.
        let (rr, rep) = m.translate_edit(chip.display.clone(), "Grace Notes");
        let mut new_raw = raw.to_string();
        new_raw.replace_range(rr, &rep);
        assert_eq!(new_raw, "Sermon: Grace Notes speaks");
    }

    #[test]
    fn caret_snaps_out_of_chips() {
        let doc = test_doc();
        let m = BufferMap::build("{Church.pastor} speaks", &doc, no_expand);
        let chip = m.chips[0].clone();
        assert_eq!(m.snap_display_caret(chip.display.start), chip.display.start);
        assert_eq!(m.snap_display_caret(chip.display.end), chip.display.end);
        let mid = (chip.display.start + chip.display.end) / 2;
        let snapped = m.snap_display_caret(mid);
        assert!(snapped == chip.display.start || snapped == chip.display.end);
        assert_eq!(
            m.atomic_ending_at_display(chip.display.end),
            Some(chip.display.clone())
        );
        assert_eq!(
            m.atomic_starting_at_display(chip.display.start),
            Some(chip.display)
        );
    }

    #[test]
    fn leader_line_maps_through_the_tab() {
        let doc = test_doc();
        let raw = "Hymn of Praise\t432";
        let m = BufferMap::build(raw, &doc, fake_expand);
        assert_eq!(m.display_text, "Hymn of Praise ...... 432");
        let leader = m.atomic_starting_at_display(14).expect("leader at 14");
        assert_eq!(&m.display_text[leader.clone()], " ...... ");
        // Deleting the leader deletes exactly the tab in raw.
        let (rr, _) = m.translate_edit(leader.clone(), "");
        assert_eq!(&raw[rr.clone()], "\t");
        // Inserting at the leader's end lands after the tab.
        let (rr, _) = m.translate_edit(leader.end..leader.end, "x");
        assert_eq!(rr.start, raw.find('\t').unwrap() + 1);
        // A selection across the whole line covers the tab.
        let (rr, _) = m.translate_edit(0..m.display_text.len(), "New");
        assert_eq!(rr, 0..raw.len());
    }

    #[test]
    fn paragraphs_map_independently() {
        let doc = test_doc();
        let raw = "First para.\nSermon: {Church.pastor}\nThird {Sunday.name} day.";
        let m = BufferMap::build(raw, &doc, no_expand);
        assert_eq!(m.para_count(), 3);
        assert_eq!(m.para_at_display(0), 0);
        let p2 = m.para_display_range(1);
        assert_eq!(&m.display_text[p2.clone()], "Sermon: Pastor Lee");
        assert_eq!(m.para_at_display(p2.start + 2), 1);
        // With no leader expansion, resolved and display
        // ranges line up paragraph for paragraph.
        assert_eq!(m.resolved_text, m.display_text);
        // An edit in paragraph 3 lands in paragraph 3's raw.
        let day = m.display_text.find("day.").unwrap();
        let (rr, _) = m.translate_edit(day..day + 3, "morning");
        assert_eq!(&raw[rr], "day");
        // Every paragraph boundary round-trips raw -> display.
        let raw_paras = para_ranges(raw);
        for (para, rp) in raw_paras.iter().enumerate() {
            let r = m.para_display_range(para);
            assert_eq!(m.raw_to_display(rp.start), r.start);
            assert_eq!(m.raw_to_display(rp.end), r.end);
        }
    }

    #[test]
    fn token_inside_a_leader_line_still_maps() {
        let doc = test_doc();
        let raw = "{Sunday.name} Hymn\t432";
        let m = BufferMap::build(raw, &doc, fake_expand);
        assert_eq!(m.display_text, "Pentecost Hymn ...... 432");
        assert_eq!(m.chips.len(), 1);
        let chip = &m.chips[0];
        assert_eq!(&m.display_text[chip.display.clone()], "Pentecost");
        assert_eq!(&raw[chip.raw.clone()], "{Sunday.name}");
        // Deleting the chip removes just the token, not the tab.
        let (rr, _) = m.translate_edit(chip.display.clone(), "");
        assert_eq!(rr, chip.raw);
    }
}
