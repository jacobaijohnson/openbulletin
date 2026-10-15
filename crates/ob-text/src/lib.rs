//! Text helpers for the v0 scaffold.
//!
//! HONEST PLACEHOLDER: measurement here is an average-glyph-width estimate for
//! Helvetica-like faces. The project-killing risk is real text layout
//! (threaded frames, print-identical breaks); the planned spike replaces this
//! module's internals with a real shaping engine (parley/cosmic-text class)
//! behind this same API: `measure`, `wrap`, `leader_line`.

use ob_model::Mpt;

/// Estimated width of a string in milli-points.
/// Average advance ~0.50 em (regular) / ~0.53 em (bold) for Helvetica-like fonts.
pub fn measure(text: &str, font_size: Mpt, bold: bool) -> Mpt {
    let factor = if bold { 530 } else { 500 };
    text.chars().count() as Mpt * font_size * factor / 1000
}

/// Greedy word wrap to a maximum line width. Words longer than the width are
/// hard-split. Returns at least one (possibly empty) line.
pub fn wrap(text: &str, max_width: Mpt, font_size: Mpt, bold: bool) -> Vec<String> {
    if text.trim().is_empty() {
        return vec![String::new()];
    }
    // First pass: split any single word wider than the line into pieces that fit.
    let mut tokens: Vec<String> = Vec::new();
    for word in text.split_whitespace() {
        let mut rest = word;
        while measure(rest, font_size, bold) > max_width {
            let mut end = 0usize;
            for (i, ch) in rest.char_indices() {
                let nxt = i + ch.len_utf8();
                if measure(&rest[..nxt], font_size, bold) <= max_width {
                    end = nxt;
                } else {
                    break;
                }
            }
            if end == 0 {
                end = rest
                    .chars()
                    .next()
                    .map(|c| c.len_utf8())
                    .unwrap_or(rest.len());
            }
            tokens.push(rest[..end].to_string());
            rest = &rest[end..];
        }
        if !rest.is_empty() {
            tokens.push(rest.to_string());
        }
    }
    // Greedy flow over the tokens.
    let mut lines = Vec::new();
    let mut cur = String::new();
    for tok in tokens {
        let trial = if cur.is_empty() {
            tok.clone()
        } else {
            format!("{cur} {tok}")
        };
        if measure(&trial, font_size, bold) <= max_width || cur.is_empty() {
            cur = trial;
        } else {
            lines.push(cur);
            cur = tok;
        }
    }
    if !cur.is_empty() || lines.is_empty() {
        lines.push(cur);
    }
    lines
}

/// An order-of-service line with a dot leader:
/// `Hymn of Praise .......... 432`. If `right` is empty, returns `left`.
/// This is the single most-used line shape in a church bulletin; faking it
/// with spaces is how layouts break, so it is a first-class function.
pub fn leader_line(left: &str, right: &str, width: Mpt, font_size: Mpt, bold: bool) -> String {
    if right.is_empty() {
        return left.to_string();
    }
    let dot_w = measure(".", font_size, bold).max(1);
    let used = measure(left, font_size, bold) + measure(right, font_size, bold) + 2 * dot_w;
    let dots = ((width - used) / dot_w).max(2) as usize;
    format!("{left} {} {right}", ".".repeat(dots))
}

/// Expand the tab in an order-line paragraph into a dot-leader line.
pub fn expand_order_line(text: &str, width: Mpt, font_size: Mpt, bold: bool) -> String {
    match text.split_once('\t') {
        Some((l, r)) => leader_line(l.trim_end(), r.trim(), width, font_size, bold),
        None => text.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_words() {
        let lines = wrap("the quick brown fox jumps", 20_000, 11_000, false);
        assert!(lines.len() >= 2);
        for l in &lines {
            assert!(
                measure(l, 11_000, false) <= 20_000 + 6_000,
                "line too wide: {l}"
            );
        }
    }

    #[test]
    fn leader_fills_and_ends_with_number() {
        let s = leader_line("Hymn of Praise", "432", 300_000, 11_000, false);
        assert!(s.starts_with("Hymn of Praise"));
        assert!(s.ends_with("432"));
        assert!(s.contains("..."));
        assert!(measure(&s, 11_000, false) <= 300_000);
    }

    #[test]
    fn leader_without_right_is_plain() {
        assert_eq!(
            leader_line("Prelude", "", 300_000, 11_000, false),
            "Prelude"
        );
    }

    #[test]
    fn tab_expands() {
        let s = expand_order_line("Hymn of Response\t365", 300_000, 11_000, false);
        assert!(s.ends_with("365") && s.contains("..."));
    }
}

pub mod shaping;
