//! Run bookkeeping for the canvas editor (G1b): the pure
//! half of inline formatting. The session carries one
//! `Vec<Run>` per paragraph (RAW text, fields unresolved,
//! exactly the buffer's words), and every buffer mutation
//! funnels through here, so the invariants (runs cover the
//! paragraph text exactly; normalized after every edit)
//! live in one tested place instead of inside the UI code.
//!
//! All offsets are byte offsets into the JOINED raw buffer
//! (paragraph texts joined by '\n'), the same space
//! `edit_text` lives in.

use ob_model::{normalize_runs, Run};
use std::ops::Range;

/// Which face switch a toggle or query is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flag {
    Bold,
    Italic,
}

impl Flag {
    fn run_flag(self, run: &Run) -> Option<bool> {
        match self {
            Flag::Bold => run.bold,
            Flag::Italic => run.italic,
        }
    }

    fn set_run_flag(self, run: &mut Run, value: bool) {
        match self {
            Flag::Bold => run.bold = Some(value),
            Flag::Italic => run.italic = Some(value),
        }
    }

    fn style_flag(self, style: (bool, bool)) -> bool {
        match self {
            Flag::Bold => style.0,
            Flag::Italic => style.1,
        }
    }
}

/// The total text length of one paragraph's runs.
pub fn runs_len(runs: &[Run]) -> usize {
    runs.iter().map(|r| r.text.len()).sum()
}

/// Each paragraph's (start, end) in the joined buffer: the
/// ends exclude the '\n' separators between paragraphs.
pub fn para_offsets(paras: &[Vec<Run>]) -> Vec<Range<usize>> {
    let mut out = Vec::with_capacity(paras.len());
    let mut at = 0usize;
    for (i, runs) in paras.iter().enumerate() {
        if i > 0 {
            at += 1; // the '\n' separator
        }
        let start = at;
        at += runs_len(runs);
        out.push(start..at);
    }
    out
}

/// The paragraph an offset belongs to: the first whose end
/// is at or past it (an offset just after a '\n' separator
/// is the next paragraph's start).
fn para_of(offs: &[Range<usize>], offset: usize) -> usize {
    for (i, r) in offs.iter().enumerate() {
        if offset <= r.end {
            return i;
        }
    }
    offs.len().saturating_sub(1)
}

/// The runs covering `range` of a paragraph, texts sliced,
/// flags kept. The result is NOT normalized (callers
/// normalize after assembly).
fn clip(runs: &[Run], range: Range<usize>) -> Vec<Run> {
    let mut out = Vec::new();
    let mut at = 0usize;
    for r in runs {
        let end = at + r.text.len();
        let s = range.start.max(at);
        let e = range.end.min(end);
        if s < e {
            out.push(Run {
                text: r.text[s - at..e - at].to_string(),
                bold: r.bold,
                italic: r.italic,
            });
        }
        at = end;
    }
    out
}

/// The run holding the character at `local` (a char offset
/// inside one paragraph), if any.
fn run_at(runs: &[Run], local: usize) -> Option<&Run> {
    let mut at = 0usize;
    for r in runs {
        let end = at + r.text.len();
        if local >= at && local < end {
            return Some(r);
        }
        at = end;
    }
    None
}

/// The run flags text typed at `raw_offset` should carry:
/// the flags of the run holding the character before the
/// caret; at a paragraph start, the paragraph's first run;
/// in an empty paragraph, (None, None) so the style speaks.
/// Run flags, not effective flags: typing after a plain run
/// inside a bold-styled paragraph stays a plain run and the
/// style still makes it bold.
pub fn inherit_flags(paras: &[Vec<Run>], raw_offset: usize) -> (Option<bool>, Option<bool>) {
    if paras.is_empty() {
        return (None, None);
    }
    let offs = para_offsets(paras);
    let pi = para_of(&offs, raw_offset);
    let local = raw_offset - offs[pi].start;
    let runs = &paras[pi];
    if local > 0 {
        if let Some(r) = run_at(runs, local - 1) {
            return (r.bold, r.italic);
        }
    }
    match runs.first() {
        Some(r) => (r.bold, r.italic),
        None => (None, None),
    }
}

/// The EFFECTIVE flags at a caret (for the toolbar state):
/// like `inherit_flags`, with the paragraph style folded in.
pub fn effective_flags_at(
    paras: &[Vec<Run>],
    styles: &[(bool, bool)],
    raw_offset: usize,
) -> (bool, bool) {
    if paras.is_empty() {
        return (false, false);
    }
    let offs = para_offsets(paras);
    let pi = para_of(&offs, raw_offset);
    let style = styles.get(pi).copied().unwrap_or((false, false));
    let (b, i) = inherit_flags(paras, raw_offset);
    (b.unwrap_or(style.0), i.unwrap_or(style.1))
}

/// Apply one raw-space edit: replace `range` with
/// `replacement` (which may contain '\n's, splitting
/// paragraphs). Text typed takes the flags
/// `inherit_flags` reports for the range's start, computed
/// before anything moves; a non-empty range starting at a
/// paragraph start inherits from the first replaced
/// character instead, as word processors do.
pub fn apply_edit(paras: &mut Vec<Vec<Run>>, range: Range<usize>, replacement: &str) {
    if paras.is_empty() {
        paras.push(vec![Run::plain("")]);
    }
    let offs = para_offsets(paras);
    let ps = para_of(&offs, range.start);
    let pe = para_of(&offs, range.end);
    let inherit = if range.start == offs[ps].start && !range.is_empty() {
        match run_at(&paras[ps], 0) {
            Some(r) => (r.bold, r.italic),
            None => inherit_flags(paras, range.start),
        }
    } else {
        inherit_flags(paras, range.start)
    };
    let local_start = range.start - offs[ps].start;
    let local_end = range.end - offs[pe].start;
    let left = clip(&paras[ps], 0..local_start);
    let right = clip(&paras[pe], local_end..runs_len(&paras[pe]));
    let lines: Vec<&str> = replacement.split('\n').collect();
    let mut new_paras: Vec<Vec<Run>> = Vec::with_capacity(lines.len());
    for (i, line) in lines.iter().enumerate() {
        let mut runs: Vec<Run> = Vec::new();
        if i == 0 {
            runs.extend(left.iter().cloned());
        }
        if !line.is_empty() {
            runs.push(Run {
                text: line.to_string(),
                bold: inherit.0,
                italic: inherit.1,
            });
        }
        if i == lines.len() - 1 {
            runs.extend(right.iter().cloned());
        }
        new_paras.push(normalize_runs(runs));
    }
    paras.splice(ps..=pe, new_paras);
}

/// Whether every character in a raw range carries `flag`
/// effectively (run flag, else the paragraph's style flag).
/// Separator newlines are not characters. An empty range is
/// false: there is nothing to unbold.
pub fn range_all_effective(
    paras: &[Vec<Run>],
    styles: &[(bool, bool)],
    range: Range<usize>,
    flag: Flag,
) -> bool {
    let offs = para_offsets(paras);
    let mut any = false;
    for (pi, runs) in paras.iter().enumerate() {
        let s = range.start.max(offs[pi].start);
        let e = range.end.min(offs[pi].end);
        if s >= e {
            continue;
        }
        let style = styles.get(pi).copied().unwrap_or((false, false));
        for r in clip(runs, s - offs[pi].start..e - offs[pi].start) {
            any = true;
            let eff = flag.run_flag(&r).unwrap_or_else(|| flag.style_flag(style));
            if !eff {
                return false;
            }
        }
    }
    any
}

/// Set `flag` to `Some(value)` on every character of a raw
/// range, splitting runs at the range's edges. Characters
/// outside the range keep their runs exactly.
pub fn set_flag_range(paras: &mut [Vec<Run>], range: Range<usize>, flag: Flag, value: bool) {
    let offs = para_offsets(paras);
    for (pi, runs) in paras.iter_mut().enumerate() {
        let s = range.start.max(offs[pi].start);
        let e = range.end.min(offs[pi].end);
        if s >= e {
            continue;
        }
        let len = runs_len(runs);
        let ls = s - offs[pi].start;
        let le = e - offs[pi].start;
        let mut out = clip(runs, 0..ls);
        for mut r in clip(runs, ls..le) {
            flag.set_run_flag(&mut r, value);
            out.push(r);
        }
        out.extend(clip(runs, le..len));
        *runs = normalize_runs(out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bold(text: &str) -> Run {
        Run {
            text: text.into(),
            bold: Some(true),
            italic: None,
        }
    }

    fn texts(paras: &[Vec<Run>]) -> Vec<String> {
        paras
            .iter()
            .map(|p| p.iter().map(|r| r.text.as_str()).collect())
            .collect()
    }

    #[test]
    fn typing_inside_a_run_keeps_one_run() {
        let mut paras = vec![vec![Run::plain("Sing hymn")]];
        apply_edit(&mut paras, 4..4, " a");
        assert_eq!(texts(&paras), vec!["Sing a hymn"]);
        assert_eq!(paras[0].len(), 1, "still one plain run");
    }

    #[test]
    fn typing_at_a_boundary_inherits_the_left_run() {
        let mut paras = vec![vec![Run::plain("Sing "), bold("hymn")]];
        // Caret between the plain run and the bold run.
        let (b, _) = inherit_flags(&paras, 5);
        assert_eq!(b, None, "the left (plain) run wins");
        apply_edit(&mut paras, 5..5, "the ");
        assert_eq!(texts(&paras), vec!["Sing the hymn"]);
        assert_eq!(paras[0][0], Run::plain("Sing the "));
        assert_eq!(paras[0][1], bold("hymn"));
        // Caret just inside the bold run inherits bold.
        let (b, _) = inherit_flags(&paras, 11);
        assert_eq!(b, Some(true));
    }

    #[test]
    fn paragraph_start_inherits_the_first_run() {
        let paras = vec![vec![bold("Hymn"), Run::plain(" 432")]];
        let (b, _) = inherit_flags(&paras, 0);
        assert_eq!(b, Some(true));
    }

    #[test]
    fn enter_splits_the_runs_at_the_caret() {
        let mut paras = vec![vec![Run::plain("Sing "), bold("hymn 432")]];
        apply_edit(&mut paras, 7..7, "\n");
        assert_eq!(texts(&paras), vec!["Sing hy", "mn 432"]);
        assert_eq!(paras[0][1], bold("hy"));
        assert_eq!(paras[1][0], bold("mn 432"));
    }

    #[test]
    fn deleting_a_newline_merges_the_runs() {
        let mut paras = vec![vec![Run::plain("One")], vec![bold("Two")]];
        // The '\n' sits at offset 3; deleting 3..4 merges.
        apply_edit(&mut paras, 3..4, "");
        assert_eq!(paras.len(), 1);
        assert_eq!(texts(&paras), vec!["OneTwo"]);
        assert_eq!(paras[0][1], bold("Two"));
    }

    #[test]
    fn replacing_a_span_keeps_the_runs_outside_it() {
        let mut paras = vec![vec![Run::plain("Sing "), bold("hymn"), Run::plain(" 432")]];
        apply_edit(&mut paras, 5..9, "psalm");
        assert_eq!(texts(&paras), vec!["Sing psalm 432"]);
        // The replacement inherits the face in force at its
        // start (the char before is plain, so plain it is),
        // and the equal-flag neighbors merge back into one.
        assert_eq!(paras[0], vec![Run::plain("Sing psalm 432")]);
    }

    #[test]
    fn set_flag_range_splits_and_restores() {
        let mut paras = vec![vec![Run::plain("Sing hymn 432")]];
        set_flag_range(&mut paras, 5..9, Flag::Bold, true);
        assert_eq!(
            paras[0],
            vec![Run::plain("Sing "), bold("hymn"), Run::plain(" 432")]
        );
        let styles = [(false, false)];
        assert!(range_all_effective(&paras, &styles, 5..9, Flag::Bold));
        assert!(!range_all_effective(&paras, &styles, 0..9, Flag::Bold));
        set_flag_range(&mut paras, 5..9, Flag::Bold, false);
        assert_eq!(
            paras[0][1],
            Run {
                text: "hymn".into(),
                bold: Some(false),
                italic: None,
            },
            "unbolding writes an explicit Some(false) on the word"
        );
        assert!(!range_all_effective(&paras, &styles, 5..9, Flag::Bold));
    }

    #[test]
    fn effective_checks_fold_the_style_in() {
        let paras = vec![vec![Run::plain("Title")]];
        let styles = [(true, false)];
        assert!(
            range_all_effective(&paras, &styles, 0..5, Flag::Bold),
            "a bold style makes plain runs effectively bold"
        );
        assert!(!range_all_effective(&paras, &styles, 0..5, Flag::Italic));
        let (b, i) = effective_flags_at(&paras, &styles, 3);
        assert!(b && !i);
    }

    #[test]
    fn multi_paragraph_edits_land_in_the_right_paragraph() {
        let mut paras = vec![vec![Run::plain("One")], vec![Run::plain("Two")]];
        // Type at the start of the second paragraph (offset 4).
        apply_edit(&mut paras, 4..4, "Say ");
        assert_eq!(texts(&paras), vec!["One", "Say Two"]);
        // A paste with a newline over a cross-paragraph span.
        apply_edit(&mut paras, 2..5, "x\ny");
        assert_eq!(texts(&paras), vec!["Onx", "yay Two"]);
    }
}
