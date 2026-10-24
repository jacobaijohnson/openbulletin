//! Token and clause segmentation: how a sender's
//! raw line is cleaned into tokens, and where clause
//! and governor-window breaks fall among them.

use super::composition::*;
use super::dates::*;
use super::grammar::*;
use super::*;

pub(super) fn segment_tokens(raw_tokens: &[&str]) -> Vec<String> {
    let mut tokens: Vec<String> = raw_tokens.iter().map(|t| group_token(t)).collect();
    // The unglued view of the whole run: each token
    // with its glued separators taken off.
    let unglued: Vec<String> = tokens.iter().map(|t| unglued_word(t)).collect();
    // The run's word positions in the unglued view.
    let word_pos: Vec<usize> = (0..tokens.len())
        .filter(|&i| !unglued[i].is_empty())
        .collect();
    let mut unglue = vec![false; tokens.len()];
    // Governor phrases, judged on the unglued
    // vector at every phrase position: every word
    // of a matching phrase is read unglued. The
    // cancellation phrases are judged the same
    // way — the trailing readers read them off
    // these tokens ("*cancel *it" loses its phrase
    // on the relative path exactly as "*never
    // *mind" loses its governor phrase there).
    for s in 0..word_pos.len() {
        for len in 1..=3usize {
            if s + len > word_pos.len() {
                break;
            }
            let phrase: Vec<String> = word_pos[s..s + len]
                .iter()
                .map(|&p| unglued[p].clone())
                .collect();
            if is_governor_phrase(&phrase) || is_cancellation_phrase(&phrase) {
                for &p in &word_pos[s..s + len] {
                    unglue[p] = true;
                }
            }
        }
    }
    // Bare phrase units, judged on the unglued
    // vector like the phrases above: a unit
    // standing in a segment ("back *together") is
    // the unit, so its words are read unglued for
    // the bare test that consumes them.
    {
        let words: Vec<String> = word_pos.iter().map(|&p| unglued[p].clone()).collect();
        for s in 0..words.len() {
            if let Some(span) = bare_phrase_unit_span(&words, s) {
                for &p in &word_pos[s..s + span] {
                    unglue[p] = true;
                }
            }
        }
    }
    // Sunday-after phrases, judged on the unglued
    // vector like the phrases above (round 29,
    // R29-A, F8 = review MINOR-1): "sunday" standing
    // directly before "after" — the next word that
    // is something — is a Sunday-after phrase's
    // head, and the phrase is a phrase family like
    // the others this function judges, so both of
    // its words are read unglued ("the *Sunday
    // *after November 1, 2026" is the phrase, in a
    // segment exactly as on the spaced twin). A
    // glued weekday word standing anywhere else —
    // the standalone "*Monday" garnish before a
    // date, a glued "sunday" with no "after" after
    // it — is not this phrase and keeps its
    // standing cleaned form.
    for s in 0..word_pos.len() {
        if unglued[word_pos[s]] == "sunday"
            && s + 1 < word_pos.len()
            && unglued[word_pos[s + 1]] == "after"
        {
            unglue[word_pos[s]] = true;
            unglue[word_pos[s + 1]] = true;
        }
    }
    // The structural words: an auxiliary, a
    // continuation preposition, "to", or "off" is
    // read unglued wherever it stands.
    for i in 0..tokens.len() {
        let w = unglued[i].as_str();
        if !w.is_empty()
            && (AUXILIARY_WORDS.contains(&w)
                || is_continuation_preposition(w)
                || w == "to"
                || w == "off")
        {
            unglue[i] = true;
        }
    }
    for i in 0..tokens.len() {
        if unglue[i] && unglued[i] != tokens[i] {
            tokens[i] = unglued[i].clone();
        }
    }
    tokens
}

/// Whether a raw group token ends a clause: a token
/// wearing a trailing comma or semicolon closes the
/// words it stands in, so a governor on the far side
/// of it does not reach across it. A dash closes a
/// clause the same way (round 13, R13-2): a
/// standalone "-" / "--" token, or a token wearing a
/// trailing en/em dash or hyphen.
/// The closed abbreviation class of R25-A, by
/// cleaned form — the same list `bare_filler_word`
/// carries ("St." cleans to "st").
pub(super) fn is_abbreviation_class(word: &str) -> bool {
    matches!(
        word,
        "st" | "mt" | "mr" | "mrs" | "ms" | "dr" | "rev" | "fr" | "br" | "sr" | "jr"
    )
}

/// Round 26 (R26-C, F8 = review R26-2): the
/// clause-break judgment with the token's context
/// in view. A token of the abbreviation class (the
/// R25-A list) whose raw form ends in a period does
/// not break a clause when it stands inside a PP —
/// directly after a preposition, with a name word
/// (a noun-like word) standing directly after it,
/// as an abbreviation stands before the name it
/// abbreviates for: "from Mt. Kent", "with Dr.
/// Lee". Every boundary site that fed on the bare
/// period read such a PP as two clauses, and the
/// from-governance of the continuation's target
/// was lost with the clause: "(moved November 1,
/// baked beans from Mt. Kent on Sunday to November
/// 8)" asserted its target where its unperioded
/// twin reads nothing. The general rule is
/// otherwise untouched: a period outside that one
/// shape still breaks the clause.
///
/// Round 29 (R29-B, F5 tail = red team RT29-W2):
/// the same shape inside a DESTINATION name. A
/// destination PP is headed by "to" — a
/// preposition the subject set deliberately
/// excludes, as it heads the target construction —
/// so an abbreviation standing as the object of
/// "to" (or behind the destination's determiner:
/// "to the Rev. Lee's") stood outside the
/// exception and its period ended the destination's
/// noun run in every boundary judgment that feeds
/// on this one: the crossing was lost and the
/// target asserted clean where the unperioded twin
/// ("to St Mary", "to Fort Worth") reads silence.
/// In the destination position the abbreviation
/// shape is judged by its form — a short alphabetic
/// word, the form every destination abbreviation
/// in the witnesses wears ("St.", "Mt.", "Dr.",
/// "Rev.", "Ft.", "Ste.") — with the same
/// name-after requirement the PP arm has always
/// carried. A period anywhere else still breaks
/// the clause exactly as it always has.
pub(super) fn clause_break_after_at(raw_tokens: &[&str], tokens: &[String], j: usize) -> bool {
    let Some(raw) = raw_tokens.get(j) else {
        return false;
    };
    // Round 27 (R27-A, F3): a token that is nothing
    // but format characters (see `cf_only_token`)
    // breaks no clause — its raw form trips the
    // no-letters classification below, but there is
    // no punctuation there to break on; the token
    // is nothing, and nothing breaks nothing.
    if cf_only_token(raw) {
        return false;
    }
    if !clause_break_after(raw) {
        return false;
    }
    if !raw.trim_end_matches([')', ']']).ends_with('.') {
        return true;
    }
    // The destination position: the object of a
    // "to", directly or behind its determiner.
    let in_destination = (j > 0 && tokens[j - 1].as_str() == "to")
        || (j > 1
            && (matches!(tokens[j - 1].as_str(), "the" | "a" | "an")
                || POSSESSIVE_DETERMINERS.contains(&tokens[j - 1].as_str()))
            && tokens[j - 2].as_str() == "to");
    let abbreviation_shape = is_abbreviation_class(&tokens[j])
        || (in_destination
            && tokens[j].len() <= 3
            && tokens[j].chars().all(|c| c.is_ascii_alphabetic()));
    if !abbreviation_shape {
        return true;
    }
    let inside_pp = j > 0 && is_subject_preposition(&tokens[j - 1]);
    let name_after = tokens
        .get(j + 1)
        .is_some_and(|w| noun_like_word(w) && !w.is_empty());
    if (inside_pp || in_destination) && name_after {
        return false;
    }
    true
}

pub(super) fn clause_break_after(raw: &str) -> bool {
    // Round 23 (R23-3): the ONE boundary classification
    // — a token wearing a trailing colon, semicolon,
    // comma, dash (incl. en/em), slash, or period closes
    // its clause exactly as the comma has since round 12,
    // and a standalone boundary token ("/", ")", a dash)
    // is a boundary standing alone: a closing paren
    // between a governor and its date is a boundary of
    // the same kind. A slash inside a token ("11/1") is
    // token-internal and never reaches this test as an
    // ending.
    // Round 24 (R24-4): a standalone token containing
    // no letters and no digits is a boundary token
    // standing alone, whatever characters build it
    // ("*", "•", "|", "~", "_", an emoji), and a
    // token closing a bracketed span (ending "]" or
    // "}") carries that span's boundary at its tail,
    // as the comma twin of "[November 1, cancelled]
    // [November 8]" requires.
    let raw_trimmed = raw.trim();
    if !raw_trimmed.is_empty() && token_has_no_letters_or_digits(raw_trimmed) {
        return true;
    }
    if raw_trimmed.ends_with(']') || raw_trimmed.ends_with('}') {
        return true;
    }
    if raw.trim() == ")" || raw.trim() == "/" {
        return true;
    }
    let trimmed = raw.trim_end_matches([')', ']', ' ']);
    trimmed.ends_with(',')
        || trimmed.ends_with(';')
        || trimmed.ends_with(':')
        || trimmed.ends_with('.')
        || trimmed.ends_with('/')
        || trimmed.ends_with('\u{2013}')
        || trimmed.ends_with('\u{2014}')
        || trimmed == "-"
        || trimmed == "--"
        || (trimmed.len() > 1 && trimmed.ends_with('-'))
}

/// Whether a clause carries no content of its
/// own (round 14, RT14-4; round 15, RT15-5): every
/// word is date-referring filler — "the", "a",
/// "an", "date", "it", "note", "notes", and now
/// "of" — or the clause is a genitive fragment of
/// the date phrase itself: a clause headed by
/// "date of" ("the date of the service") names
/// the date by naming the occasion the date
/// belongs to; it asserts no event of its own, so
/// it is filler for the reach-back like any pure
/// filler clause. A clause of nothing but break
/// punctuation (a standalone dash) is likewise
/// empty. Any other word is content and stops
/// the reach-back.
pub(super) fn is_filler_clause(clause: &[String]) -> bool {
    // Round 28 (R28-A, F2): judged on the clause's
    // words that are something, as in the clause
    // matchers above — a nothing-token inside a
    // filler clause is nothing in it.
    let words: Vec<String> = clause.iter().filter(|w| !w.is_empty()).cloned().collect();
    let clause: &[String] = &words;
    if clause.iter().all(|w| w.is_empty()) {
        return true;
    }
    if clause.iter().all(|w| {
        matches!(
            w.as_str(),
            "the" | "a" | "an" | "date" | "it" | "note" | "notes" | "of"
        )
    }) {
        return true;
    }
    let body: &[String] = match clause.first().map(String::as_str) {
        Some("the" | "a" | "an") => &clause[1..],
        _ => clause,
    };
    body.len() >= 3
        && body[0] == "date"
        && body[1] == "of"
        && body[2..]
            .iter()
            .all(|w| !direct_single_governor(w) && weekday_word(w).is_none())
}

/// Whether a clause is a cancellation and
/// nothing else (round 15, R15-3): it holds a
/// cancellation phrase, and every word outside
/// that phrase is date-referring filler ("the",
/// "a", "an", "date", "it", "that", "note",
/// "notes", "of") — "postponed", "drop it",
/// "scratch that", "cancelled, the date" read
/// as clauses. Such a clause standing after a
/// date is spent on that date, and a run of
/// them is spent together.
pub(super) fn is_cancellation_only_clause(clause: &[String]) -> bool {
    // Round 28 (R28-A, F2): the clause's words are
    // its words that are something — a nothing-token
    // standing inside the clause ("not \u{200b}",
    // standing as its own clause between dates) is
    // nothing in it, so the clause is judged on its
    // words, as the clean twin's clause is. The
    // positional logic below runs on the filtered
    // words throughout.
    let words: Vec<String> = clause.iter().filter(|w| !w.is_empty()).cloned().collect();
    let clause: &[String] = &words;
    for i in 0..clause.len() {
        for len in 1..=3usize {
            let Some(phrase) = clause.get(i..i + len) else {
                break;
            };
            if !is_cancellation_phrase(phrase) {
                continue;
            }
            // Round 16 (RT16-6): a determiner-headed
            // -s word is a noun, not a cancellation —
            // the clause is not cancellation-only.
            if len == 1 && determiner_headed_s_form(clause, i, &phrase[0]) {
                continue;
            }
            let rest_filler = clause
                .iter()
                .enumerate()
                .filter(|(j, _)| *j < i || *j >= i + len)
                .all(|(_, w)| {
                    matches!(
                        w.as_str(),
                        "the" | "a" | "an" | "date" | "it" | "that" | "note" | "notes" | "of"
                    )
                });
            if rest_filler {
                return true;
            }
        }
    }
    false
}

/// Whether a clause is a negation / prospective
/// phrase and nothing else (round 16, R16-2): it
/// holds a governor phrase that is not a
/// cancellation phrase — "not", "no longer",
/// "ignore", "never", a contraction — and every
/// word outside that phrase is date-referring
/// filler, the same set `is_cancellation_only_clause`
/// allows. Such a clause carries no date of its
/// own, so standing between two dates it belongs to
/// the run that spends on the first date, exactly
/// as a cancellation clause does. (A prospective
/// clause carrying its own date — "use November 8"
/// — is the second date's own clause: the date
/// stands inside it, so it never reaches this test
/// as a between-dates clause.)
pub(super) fn is_prospective_only_clause(clause: &[String]) -> bool {
    // Round 28 (R28-A, F2): judged on the clause's
    // words that are something, as in
    // `is_cancellation_only_clause` above.
    let words: Vec<String> = clause.iter().filter(|w| !w.is_empty()).cloned().collect();
    let clause: &[String] = &words;
    for i in 0..clause.len() {
        for len in 1..=3usize {
            let Some(phrase) = clause.get(i..i + len) else {
                break;
            };
            if !is_governor_phrase(phrase) || is_cancellation_phrase(phrase) {
                continue;
            }
            let rest_filler = clause
                .iter()
                .enumerate()
                .filter(|(j, _)| *j < i || *j >= i + len)
                .all(|(_, w)| {
                    matches!(
                        w.as_str(),
                        "the" | "a" | "an" | "date" | "it" | "that" | "note" | "notes" | "of"
                    )
                });
            if rest_filler {
                return true;
            }
        }
    }
    false
}

/// Whether a clause belongs to one of the governor
/// families or is filler (round 16, R16-2): the
/// clause scope a run between dates is judged by.
pub(super) fn is_governor_family_clause(clause: &[String]) -> bool {
    is_cancellation_only_clause(clause)
        || is_prospective_only_clause(clause)
        || is_filler_clause(clause)
}

/// The governor window standing before one date
/// occurrence inside its group (round 12, R12-1 /
/// R12-2): the tokens after the previous date
/// occurrence's end and after the last clause break
/// before this date, up to the date itself. A
/// governor reaches only the one date it governs —
/// the first date standing in its own clause after
/// it. Words in an earlier clause, or spent on an
/// earlier date, govern nothing here: "(notes from
/// the pastor, November 1)" and "(choir was singing,
/// November 1)" name November 1 in a fresh clause,
/// and "(not November 1, November 8)" asserts
/// November 8 on its own terms. Round 13 (R13-2):
/// the break erases only a spent governor — an
/// unspent governor standing as its clause's
/// trailing element reaches the first date of the
/// next clause (see the body).
/// Whether a clause break standing after the
/// nothing-token at `j` bounds a governor window
/// (round 28, R28-A, F2 = review MINOR-3). The
/// standing boundary classification is untouched —
/// a separator-only token is a boundary token at
/// every site the R24-4 pin names — but at THIS
/// site, the governor-scope scan, a break after a
/// nothing bounds the window only when it stands
/// between a spent cancellation and the next
/// date: the word before the nothing ends a
/// cancellation phrase, the governor
/// `governed_after` has already spent on the date
/// before it ("cancelled * November 8" reads as
/// its comma twin, as R24-4 pins). A nothing
/// standing INSIDE a governor phrase ("hold *
/// off", "drop * it") or directly after a
/// prospective governor — a phrase that governs
/// forward, judged by the next date's own window,
/// and is spent backward only as a complete
/// clause of its own, which a nothing cannot make
/// it ("not * November 8") — is nothing there, so
/// the break does not bound the window and the
/// phrase reaches the date as its clean twin's
/// does.
pub(super) fn window_break_after_nothing(tokens: &[String], j: usize) -> bool {
    let Some(&p) = prev_nonempty(tokens, j, 1).first() else {
        return true;
    };
    // A phrase starting at a word before the
    // nothing and completing after it: the nothing
    // stands inside the phrase. Round 29 (R29-A,
    // F1b = red team RT29-C1b): the phrase is judged
    // at EVERY gap position, not only from the word
    // immediately before the nothing — a three-word
    // phrase whose first word stands two positions
    // back ("used to * be") has its nothing in its
    // second gap, and a phrase starting only at the
    // immediately preceding word ("to be") is no
    // phrase at all, so the second gap was invisible
    // here. A phrase is seen when any of its words
    // stands before the nothing with the phrase's
    // remainder after it: the candidate starts are
    // the two nearest words before the nothing (a
    // phrase of at most three words straddles the
    // nothing only from one of those starts).
    for &s in &prev_nonempty(tokens, j, 2) {
        for len in 2..=3usize {
            if let Some((phrase, end)) = word_phrase(tokens, s, len) {
                if end - 1 > j && (is_governor_phrase(&phrase) || is_cancellation_phrase(&phrase)) {
                    return false;
                }
            }
        }
    }
    // The phrases ending at that word: a
    // cancellation ending there bounds the window;
    // a prospective governor ending there does
    // not.
    let prevs = prev_nonempty(tokens, p + 1, 3);
    let mut governor_ends = false;
    for (rank, &s) in prevs.iter().enumerate() {
        let len = prevs.len() - rank;
        if let Some((phrase, end)) = word_phrase(tokens, s, len) {
            if end == p + 1 {
                if is_cancellation_phrase(&phrase) {
                    return true;
                }
                if is_governor_phrase(&phrase) {
                    governor_ends = true;
                }
            }
        }
    }
    !governor_ends
}

/// The clause-break test at the governance sites
/// (round 28, R28-A, F2 = review MINOR-3): the
/// standing classification of
/// `clause_break_after_at`, with the one
/// exception of `window_break_after_nothing` — a
/// break after a nothing-token counts at the
/// governance sites only when it stands between a
/// spent cancellation and the next date. Every
/// other clause-break consumer (the weekday
/// check, the claim reach, the fence tests) keeps
/// the standing classification untouched.
pub(super) fn governance_clause_break_after_at(
    raw_tokens: &[&str],
    tokens: &[String],
    j: usize,
) -> bool {
    clause_break_after_at(raw_tokens, tokens, j)
        && (!nothing_token(raw_tokens[j]) || window_break_after_nothing(tokens, j))
}

pub(super) fn governor_window_start(
    raw_tokens: &[&str],
    tokens: &[String],
    prev_end: usize,
    date_start: usize,
    has_prev_date: bool,
) -> usize {
    let lo = prev_end.min(date_start);
    let break_at = |j: usize| governance_clause_break_after_at(raw_tokens, tokens, j);
    let mut start = lo;
    let mut last_break: Option<usize> = None;
    for j in lo..date_start {
        if break_at(j) {
            start = j + 1;
            last_break = Some(j);
        }
    }
    // Round 13 (R13-2): a clause break erases only a
    // SPENT governor. When the clause standing
    // before the last break ends in an unspent
    // governor — nothing between it and the break
    // but date-referring filler — that governor
    // still governs the first date of the next
    // clause, so the window reaches back to that
    // clause's start. A governor followed by other
    // words in its own clause ("notes from the
    // pastor, …") reaches nothing across the break:
    // the garnish pin stands. Semicolon and dash
    // breaks behave like comma breaks here, as
    // `clause_break_after` already classes them.
    if last_break.is_some() {
        // The clause breaks in the window, in order;
        // the clauses they close are examined from
        // the nearest back (round 14, RT14-4 below).
        let breaks: Vec<usize> = (lo..date_start).filter(|&j| break_at(j)).collect();
        let mut idx = breaks.len();
        while idx > 0 {
            idx -= 1;
            let b = breaks[idx];
            let clause_start = if idx == 0 { lo } else { breaks[idx - 1] + 1 };
            // A standalone dash break token carries
            // no word of the clause it closes.
            let mut clause_end = b + 1;
            if matches!(tokens.get(b).map(String::as_str), Some("" | "-" | "--")) {
                clause_end = b;
            }
            let clause = &tokens[clause_start..clause_end.max(clause_start)];
            // Round 29 (R29-A, F1a = red team
            // RT29-C1a): a clause that cleans to
            // nothing is not the end of the walk.
            // Two boundary tokens in a row after a
            // cancellation phrase ("cancelled * *
            // November 8", "cancelled, * November 1,
            // 2026") leave an empty clause standing
            // between the two breaks; the walk used
            // to stop at that empty clause and never
            // examine the cancellation clause behind
            // it, so the governed date asserted. The
            // reach-back skips a clause that cleans
            // to nothing and examines the clause
            // behind it, exactly as it crosses the
            // filler and spent clauses below. A
            // single break leaves no empty clause
            // behind it, so the standing readings
            // are untouched.
            if clause.iter().all(|w| w.is_empty()) {
                continue;
            }
            let clause_follows_a_date = has_prev_date && clause_start == lo;
            // Round 15 (R15-3): a clause that is a
            // cancellation and nothing else, standing
            // in a window that follows a date, is
            // spent on that date — and so is a whole
            // RUN of them: "(November 1, postponed,
            // drop it, November 8)" spends both
            // phrases on November 1, and November 8
            // asserts. The walk simply never extends
            // the window into such a clause; filler
            // clauses between the run's phrases are
            // crossed by the filler rule below, so a
            // run wearing one ("cancelled, the date,
            // drop it") is spent together too.
            if has_prev_date && is_cancellation_only_clause(clause) {
                continue;
            }
            // Round 16 (R16-2): a negation / prospective
            // clause with no date of its own, standing
            // between dates, spends on the first date
            // together with the cancellation clauses —
            // "(November 1, not, drop it, November 8)"
            // spends "not" and "drop it" on November 1
            // and asserts November 8. Round 15's walk
            // spent only cancellation clauses, so the
            // prospective word read as an unspent
            // governor of the second date and the
            // replacement died unnamed.
            if has_prev_date && is_prospective_only_clause(clause) {
                continue;
            }
            // Round 20 (R20-5): a fronted adverb
            // stack standing in its own clause
            // directly before this one is judged by
            // the same list and the same two-word
            // cap as every other position: an
            // unlisted member, or a stack that with
            // this clause's own crossed adverbs
            // exceeds the cap, leaves this clause's
            // predicates unspent.
            let fronted_invalid = if idx >= 1 {
                let (p_start, p_end) = clause_span(&breaks, lo, idx - 1, tokens);
                match adverb_stack_clause(&tokens[p_start..p_end]) {
                    Some((count, all_listed)) => {
                        let in_clause = clause.iter().filter(|w| is_tolerated_adverb(w)).count();
                        !all_listed || count + in_clause > 2
                    }
                    None => false,
                }
            } else {
                false
            };
            if unspent_trailing_governor_fronted(clause, clause_follows_a_date, fronted_invalid) {
                start = clause_start;
                break;
            }
            // Round 14 (RT14-4): an intervening clause
            // of nothing but date-referring filler
            // ("the", "a", "an", "date", "it", "note",
            // "notes") is transparent to the
            // reach-back — "(ignore, the date,
            // November 1)" and "(ignore, notes,
            // November 1)" disqualify like the held
            // "(ignore the date, November 1)". (Such a
            // clause holds no date token: its words
            // are the filler set itself.) A clause
            // with real content stops the reach-back,
            // exactly as before. Round 15 (RT15-5):
            // "of" joins the filler set, and a
            // genitive fragment of the date phrase
            // ("the date of the service") counts as
            // filler too — see `is_filler_clause`.
            if is_filler_clause(clause) {
                continue;
            }
            // Round 16 (R16-7): a filler clause ending
            // in "of" leaves a genitive open, and a
            // following determiner-headed noun clause
            // completes it — "(ignore, the date of,
            // the service, November 1)" disqualifies
            // like the unsplit shape. Cross the
            // completing clause and judge the "of"
            // clause itself on the next pass.
            if idx >= 1 {
                let (p_start, p_end) = clause_span(&breaks, lo, idx - 1, tokens);
                if p_start < p_end && genitive_completion(clause, &tokens[p_start..p_end]) {
                    continue;
                }
            }
            break;
        }
    }
    start
}

/// The (start, end) token span of the clause closed
/// by break `k` in a governor window, in the walk's
/// own terms (round 16, R16-7): a standalone dash
/// break token carries no word of its clause.
pub(super) fn clause_span(
    breaks: &[usize],
    lo: usize,
    k: usize,
    tokens: &[String],
) -> (usize, usize) {
    let b = breaks[k];
    let start = if k == 0 { lo } else { breaks[k - 1] + 1 };
    let mut end = b + 1;
    if matches!(tokens.get(b).map(String::as_str), Some("" | "-" | "--")) {
        end = b;
    }
    (start, end)
}

/// Whether `clause` completes a genitive the clause
/// before it left open (round 16, R16-7): the prior
/// clause is filler ending in "of" ("the date of"),
/// and this clause is a determiner-headed noun
/// fragment — a determiner plus nouns only, no date
/// token, no verb-form, no governor — that completes
/// it ("the service"). The reach-back crosses such a
/// clause as it crosses the filler itself. A clause
/// with real content (a verb, a date, or more than
/// the fragment) still stops the reach-back.
pub(super) fn genitive_completion(clause: &[String], prev: &[String]) -> bool {
    if clause.len() < 2 || !matches!(clause[0].as_str(), "the" | "a" | "an") {
        return false;
    }
    // Round 17 (R17-6): the "no verb-form" test is
    // the determiner-aware form test, the same one
    // the rest of the batch's determiner rule uses —
    // an -s stem word headed by this clause's own
    // determiner is a noun phrase's head ("the
    // drops", "the shifts"), not a verb-form, so it
    // does not stop the crossing. A word that is a
    // verb-form without that heading (a bare -ed
    // form, "the moved service") still stops it.
    let nouns_only = clause[1..].iter().enumerate().all(|(off, w)| {
        let headed_noun = determiner_headed_s_form(clause, off + 1, w);
        (!is_cancellation_verb(w) || headed_noun)
            && (!direct_single_governor(w) || headed_noun)
            && weekday_word(w).is_none()
    });
    if !nouns_only {
        return false;
    }
    prev.last().map(String::as_str) == Some("of") && is_filler_clause(prev)
}

/// Whether the words in a date occurrence's governor
/// window disqualify that date (round 11, R11-4;
/// families completed round 12, R12-1): a negation
/// or cancellation ("not", "ignore", "never",
/// "skip", "except", "rather than", "instead of",
/// "cancelled/canceled", bare "no", "isn't" and its
/// family, "never mind", "no longer",
/// "don't/didn't use") or an origin/past marker
/// ("from", "originally", "previously", "formerly",
/// "used to be", "was", "old date") standing in the
/// window means the group does not assert that
/// date — it denies it, cancels it, or names it as
/// the superseded one. Round 13 (R13-1) completes
/// the classes structurally: any "n't" contraction
/// (plus wont / cannot) negates, the cancellation
/// verbs cancel / scratch / drop / postpone /
/// remove and the pair "hold off" govern, and the
/// label "wrong:" (the token "wrong", its colon
/// stripped by `group_token`) disowns its date.
/// "Was" and "no" are common
/// words: they govern only here, in the governing
/// clause the window defines, never anywhere in
/// prose at large. "Moved to" and "correction:"
/// carry no marker and keep contributing.
/// The start of a passive "for" construction
/// standing at the end of a governor window, if one
/// does (round 17, R17-4): the window ends in "for",
/// the word before it is a participle — a governor
/// participle in any form, or any -ed participle
/// ("scheduled", the canonical occasion verb) — and
/// scanning back from the participle crosses at
/// least one auxiliary, up to two tolerated
/// adverbs allowed (round 18, R18-10), exactly as
/// the spent-predicate scan crosses them. In "(it was moved for November 1)"
/// the predicate is spent on its subject and "for"
/// names the date as the occasion, the way the
/// comma twin "(it was moved, November 1)" always
/// has. The bare same-clause parity stands apart:
/// with no "for" there is no construction here, and
/// "(it was moved November 1)" governs as pinned.
/// Common finite verb stems beyond the governor
/// families (round 18, R18-8): the verbs an office
/// clause is built from — "choir sang", "the ushers
/// walked" — whose forms mark a new finite clause.
/// The governor families (cancel / move / …) are
/// tested by their own stem rules wherever this
/// test is used; this list covers the rest of the
/// ordinary clause verbs, regular and irregular
/// alike. It is a closed list, like the adverb
/// list: a verb outside it is not seen as one.
pub(super) const FINITE_VERB_STEMS: [&str; 50] = [
    "sing", "walk", "talk", "speak", "write", "read", "play", "work", "stand", "sit", "run",
    "meet", "hold", "take", "give", "make", "say", "tell", "come", "leave", "keep", "begin",
    "break", "choose", "fall", "feel", "find", "grow", "hear", "know", "lose", "pay", "see",
    "send", "spend", "teach", "think", "wear", "win", "build", "catch", "draw", "fight", "lead",
    "mean", "ring",
    // Round 19 (R19-5): the go family and the
    // rehearses / gathers / plans stems — "go"
    // stood in neither list ("went" alone was in
    // the irregulars), and "rehearse" / "gather" /
    // "plan" were absent entirely, so the reach
    // crossed those second clauses and named the
    // from-date.
    "go", "plan", "rehearse", "gather",
];

/// Irregular finite forms the stem rules cannot
/// build (round 18, R18-8): "sang" is a form of
/// "sing" the way "forgot" is a form of "forget",
/// and it is the witness word of the second-clause
/// rule — "(moved November 1, choir sang to
/// November 8)".
pub(super) const IRREGULAR_FINITE_FORMS: [&str; 59] = [
    // Round 19 (R19-5): "gone" joins "went" — the
    // go family's participle, which the stem rules
    // cannot build from "go".
    // Round 22 (R22-4): "taken" joins on the same
    // ground — the take family's participle, which
    // the suffix rules cannot build from "take";
    // without it "then was taken" had no complement
    // the auxiliary scan could see.
    "sang", "sung", "went", "gone", "came", "took", "taken", "gave", "made", "said", "told", "met",
    "held", "ran", "sat", "stood", "spoke", "wrote", "ate", "drank", "brought", "bought", "sold",
    "left", "kept", "began", "broke", "chose", "fell", "felt", "found", "got", "grew", "heard",
    "knew", "lost", "paid", "saw", "sent", "spent", "taught", "thought", "wore", "won", "built",
    "caught", "drew", "fought", "led", "meant", "rang", "rose", "shook", "slept", "struck", "swam",
    "tore", "threw", "woke",
];

/// The past-tense-only irregular forms (round 22,
/// R22-6): the members of `IRREGULAR_FINITE_FORMS`
/// whose participle is a different word (sing /
/// sang / sung, ring / rang / rung, take / took /
/// taken, begin / began / begun, and their kin —
/// "went" and "came" included, their participles
/// being "gone" and "come"). A past-only form has
/// no adjective use in any position, so the
/// determiner shield that spans a noun phrase for
/// a participle form (see `verb_form_is_finite`)
/// never reaches it: "sang hymns" is a clause
/// standing alone and a clause behind a subject
/// ("the choir sang hymns"), never a noun phrase.
/// The remaining irregulars are participles or
/// past/participle-shared forms (sung, gone, left,
/// held, made, met, …), which do carry the
/// adjective use "the big sung hymns" wears.
pub(super) const IRREGULAR_PAST_ONLY_FORMS: [&str; 25] = [
    "sang", "went", "came", "took", "gave", "spoke", "wrote", "ate", "drank", "ran", "began",
    "broke", "chose", "fell", "grew", "knew", "saw", "drew", "rang", "rose", "shook", "swam",
    "tore", "threw", "woke",
];

/// Whether `word` is a finite verb form beyond the
/// governor families (round 18, R18-8): a form of
/// one of the common clause-verb stems, an irregular
/// finite form, or a regular -ed form. The -ed test
/// is the blunt edge: a word the office uses as a
/// noun or adjective can end in "ed" ("indeed" is
/// the standing exception), and callers that scan a
/// noun phrase temper it with the determiner rule
/// in `segment_has_finite_verb`.
pub(super) fn is_finite_verb_form(word: &str) -> bool {
    if word == "indeed" {
        return false;
    }
    if IRREGULAR_FINITE_FORMS.contains(&word) {
        return true;
    }
    if FINITE_VERB_STEMS
        .iter()
        .any(|stem| is_verb_form(word, stem))
    {
        return true;
    }
    word.len() > 3 && word.ends_with("ed")
}

/// The generic -ed ambiguity (round 19, R19-4):
/// a word whose only claim to being a finite
/// verb is the generic -ed suffix — not an
/// irregular form and not a form of any listed
/// stem — is exactly the ambiguous case: the
/// same spelling
/// is an adjective in "baked beans" and a noun in
/// "the printed bulletins" office prose. Round 20
/// (R20-4): the tempering that distinction fed —
/// generic -ed forms only — is replaced by the
/// one verb test for every verb form (see
/// `verb_form_is_finite` below).
/// Whether a noun-like subject stands before the
/// verb-form word at `off` in its segment (round
/// 20, R20-4): the subject is a noun-like word
/// that is not itself a verb form, and nothing
/// but tolerated adverbs and auxiliaries stand
/// between it and the verb form — "choir folded",
/// "team played", "their team played", "the choir
/// sang" all qualify. A segment opening with a
/// conjunction continues the target phrase's
/// object list (the R18-4 reading: "…, and the
/// dinner to …"), so its words are that list's
/// own material and no subject stands in it: "and
/// planned dinner", "and fresh baked beans" name
/// things, they do not do anything.
///
/// Round 21 (R21-6): the subject is a RUN of
/// determiner / modifier / noun-like words ending
/// at the verb form, not exactly one noun-like
/// word — the one-word test crossed every real
/// clause whose subject wore a modifier or a
/// title ("the big choir sang", "choir members
/// sang", "the youth choir rang", "Pastor Lee
/// sang") and named a false from-date. The walk
/// back from the verb form crosses the words that
/// can stand in a subject phrase or between it
/// and its verb — determiners and possessives,
/// noun-like words (the head and its modifiers,
/// verb-form modifiers among them: "visiting
/// choir", "planning committee"), tolerated
/// adverbs, and auxiliaries — and the subject
/// stands when the walk crosses the phrase's
/// head: a noun-like word that is not itself a
/// verb form. Two noun-like words before the verb
/// form are no longer by themselves a noun phrase
/// the verb form postposes; the postposed reading
/// is judged in `verb_form_is_finite` itself.
/// The prepositions that can head a phrase inside
/// a subject run (round 22, R22-5): the locative /
/// origin / temporal family a subject wears — "the
/// team from Kent", "the choir in the hall", "the
/// choir on Sunday". "to" and "for" are not here:
/// they head the target and occasion constructions,
/// never a subject's own phrase.
pub(super) fn is_subject_preposition(word: &str) -> bool {
    matches!(
        word,
        "from"
            | "of"
            | "on"
            | "in"
            | "at"
            | "by"
            | "with"
            | "before"
            | "after"
            | "since"
            | "during"
            | "between"
            | "among"
            | "within"
            | "upon"
            | "into"
            | "over"
            | "under"
            | "against"
            | "across"
            | "behind"
            | "beyond"
            | "beside"
            | "near"
            | "through"
            | "without"
            | "toward"
            | "towards"
            | "onto"
            | "per"
            | "via"
            | "until"
            | "around"
            | "about"
    )
}

/// Round 26 (R26-C, F2 mechanism 4 = review
/// R26-4): the OBJECT-CONTINUATION walk's own
/// crossing set — the subject prepositions plus
/// "for". A "for" PP standing inside a target
/// claim's object continuation ("baked beans for
/// them on Sunday to November 8") is crossed by
/// this walk exactly as the other prepositions
/// are, when a target construction follows (the
/// walk's own two arms judge that). This set is
/// the continuation walk's alone: `is_subject_
/// preposition` — the subject-PP set, from whose
/// exclusion of "for" the passive-for naming rule
/// stands — is untouched, and "(it was moved for
/// November 1)" and its family keep their
/// readings exactly.
pub(super) fn is_continuation_preposition(word: &str) -> bool {
    // Round 27 (R27-A, F1 mechanism 4): "past" joins
    // the continuation set — a destination
    // preposition of the same family as "through" /
    // "toward" ("… from Kent past the hall to
    // November 8") — in THIS set only: it is not a
    // subject preposition, and `is_subject_preposition`
    // is untouched.
    is_subject_preposition(word) || word == "for" || word == "past"
}

/// Round 27 (R27-A, F1 mechanism 1): ONE word
/// class for the auxiliary stack's slot — the
/// tolerated adverbs, the adverbial (-ly) class,
/// "never" / "ever", and the negation "not". Every
/// site that crosses an auxiliary stack judges its
/// slot words by this one predicate: the passive
/// test below, the object-continuation walk's own
/// adverb branch and post-PP slot, and the forward
/// arm's conjunct auxiliary crossing.
pub(super) fn aux_slot_word(word: &str) -> bool {
    is_tolerated_adverb(word) || adverbial_word(word) || matches!(word, "never" | "ever" | "not")
}

/// Round 26 (R26-C, F2 mechanism 1): whether the
/// finite verb form at `verb` in `tokens` heads a
/// PASSIVE construction — an auxiliary standing
/// before it across its slot ("were served", "was
/// still sung"): the auxiliary stack the backward
/// walk's own auxiliary branch already crosses.
/// Round 27 (R27-A, F1 mechanism 1): the slot is
/// the FULL stack pattern the pinned families
/// imply — any number of slot words (`aux_slot_
/// word`: listed adverbs and the negation, in any
/// order), not at most one: "were still just
/// served" and "were not served" head passive
/// constructions exactly as "were still served"
/// does. A token that cleans to nothing is nothing
/// in the stack, as at every site the walk reads.
/// The walk and this test share the rule through
/// `aux_slot_word`; there are not two copies of it.
pub(super) fn passive_aux_before(tokens: &[String], verb: usize) -> bool {
    let mut j = verb;
    while j > 0 {
        let w = tokens[j - 1].as_str();
        if AUXILIARY_WORDS.contains(&w) {
            return true;
        }
        if w.is_empty() || aux_slot_word(w) {
            j -= 1;
            continue;
        }
        return false;
    }
    false
}

/// Whether `word` can stand in the destination
/// NP's noun run (round 29, R29-B, F5 = review
/// MAJOR-2): a noun-like word, or a finite verb
/// form whose spelling is a noun in this
/// position ("work", "rose", "reading") — the
/// run is judged as an NP, so verb-form
/// membership alone does not remove a word from
/// it. The word classes that are no noun in any
/// position stay out: auxiliaries, governor
/// words, possessive determiners, weekday words,
/// and tolerated adverbs are the exclusions
/// `noun_like_word` itself carries, applied to
/// the verb-form arm alike ("rose" reaches the
/// run only through this arm, `noun_like_word`
/// excluding the irregular forms by name).
pub(super) fn destination_noun_word(word: &str) -> bool {
    if noun_like_word(word) {
        return true;
    }
    is_finite_verb_form(word)
        && !AUXILIARY_WORDS.contains(&word)
        && !POSSESSIVE_DETERMINERS.contains(&word)
        && !direct_single_governor(word)
        && !is_tolerated_adverb(word)
        && weekday_word(word).is_none()
}

/// Round 28 (R28-B, F5 = red team RT28-W3): the
/// end of the DESTINATION PP the "to" at `k`
/// heads, if it heads one — the index just past
/// its object's noun run. One rule for every site
/// that crosses a destination "to" (the forward
/// arm, its intervening-PP twin, and the backward
/// walk): a "to" is crossed as a destination when
/// its object is a PLACE-SHAPED NP and a further
/// target construction follows it (another "to" /
/// "until", a conjunction, or a further PP of the
/// stack). Place-shaped is a shape, not a
/// determiner: the NP may be determiner-headed
/// ("to the hall", the R27-A rule), possessive-
/// headed ("to our hall"), or bare ("to church",
/// "to Seattle") — the other destination
/// prepositions (into / past / toward / towards /
/// through) have always crossed the same bare
/// objects through their generic noun run, and
/// "to" is now judged by the same shape. A bare
/// common noun standing in this position — between
/// an origin PP and the target construction that
/// must follow for the crossing to fire — is read
/// as the itinerary object: the position is what
/// makes the NP place-shaped, exactly as a
/// determiner-headed NP of any noun already was,
/// and no itinerary vocabulary list is kept. Two
/// shapes are never destinations: a "to" whose
/// object is a framed date (the claim's target
/// itself — "to November 8"), and a "to" whose
/// object is a relative phrase (likewise the
/// target's own shape, judged by its own arm).
pub(super) fn destination_to_end(
    raw_tokens: &[&str],
    tokens: &[String],
    k: usize,
    today: SimpleDate,
) -> Option<usize> {
    if tokens.get(k).map(String::as_str) != Some("to") {
        return None;
    }
    // The NP's head: determiners and possessive
    // determiners the object wears, crossed with
    // any nothings among them (R28-A, F2).
    let mut m = k + 1;
    while tokens.get(m).is_some_and(|w| {
        w.is_empty()
            || matches!(w.as_str(), "the" | "a" | "an")
            || POSSESSIVE_DETERMINERS.contains(&w.as_str())
    }) {
        m += 1;
    }
    // A date or a relative phrase standing as the
    // object is the claim's target, not a
    // destination.
    if raw_tokens
        .get(m..)
        .is_some_and(|rest| parse_date_prefix(rest, today).is_some())
    {
        return None;
    }
    if matches!(
        tokens.get(m).map(String::as_str),
        Some("next") | Some("this")
    ) && tokens.get(m + 1).is_some_and(|w| weekday_word(w).is_some())
    {
        return None;
    }
    // Round 29 (R29-B, F5 = review MAJOR-2): the
    // run is judged as an NP — a word's
    // finite-verb-form membership does not remove
    // it from the destination noun run when it
    // stands as the object of "to" in the
    // destination position, in the bare arm and
    // the determiner-headed arm alike (both read
    // this one run): "to work", "to Rose", "to
    // Reading" are itinerary objects whose
    // spelling is also a verb form, and the
    // position is what makes the NP place-shaped,
    // exactly as the shipped choice states for
    // every other noun. The admission is the
    // noun run's own shape with only the
    // verb-form exclusion lifted: a governor or
    // auxiliary word is no noun in any position,
    // and the run still ends at "until", at a
    // continuation preposition, and at every word
    // that is neither noun-like nor a verb form.
    // The further-target-construction check below
    // is unchanged — a verb form standing as the
    // target's own verb, with no target
    // construction after its run, heads no
    // destination here as it heads none anywhere.
    let mut n = m;
    while tokens.get(n).is_some_and(|w| {
        w.is_empty()
            || (destination_noun_word(w)
                && w.as_str() != "until"
                && !is_continuation_preposition(w))
    }) {
        n += 1;
    }
    if n == m {
        return None;
    }
    let next = tokens.get(n).map(String::as_str);
    if matches!(next, Some("to") | Some("until") | Some("and") | Some("or"))
        || next.is_some_and(is_continuation_preposition)
    {
        Some(n)
    } else {
        None
    }
}

/// Round 26 (R26-C, F2 mechanism 1): whether a
/// further prepositional phrase stands between
/// the weekday at `i` in `tokens` and the claim's
/// "to" / "until" — the forward stack the main
/// forward arm would cross, judged here only for
/// whether at least one PP intervenes before the
/// target preposition is reached.
pub(super) fn forward_has_intervening_pp(
    raw_tokens: &[&str],
    tokens: &[String],
    i: usize,
    today: SimpleDate,
) -> bool {
    let mut k = i + 1;
    let mut intervening = false;
    loop {
        // Round 27 (R27-A, F1 mechanisms 2 and 4):
        // the post-PP slot words cross as in the
        // main forward arm, and the noun crossing
        // stops at a PP's preposition, as there.
        while tokens.get(k).is_some_and(|w| aux_slot_word(w)) {
            k += 1;
        }
        while tokens.get(k).is_some_and(|w| {
            w.is_empty()
                || (noun_like_word(w)
                    && !is_finite_verb_form(w)
                    && w.as_str() != "until"
                    && !is_continuation_preposition(w))
        }) {
            k += 1;
        }
        match tokens.get(k).map(String::as_str) {
            Some("to") | Some("until") => {
                // Round 27 (R27-A, F1 mechanism 4): a
                // destination "to" — a place-shaped
                // place NP, a further target
                // construction after it — is an
                // intervening PP, not the claim's
                // target, exactly as the main forward
                // arm crosses it. Round 28 (R28-B,
                // F5): the place-shaped judgment is
                // the shared `destination_to_end`
                // rule (determiner-headed,
                // possessive-headed, and bare objects
                // alike), not this arm's own
                // determiner check.
                if tokens[k] == "to" {
                    if let Some(n) = destination_to_end(raw_tokens, tokens, k, today) {
                        intervening = true;
                        k = n;
                        continue;
                    }
                }
                return intervening;
            }
            Some(w) if is_continuation_preposition(w) => {
                intervening = true;
                let mut m = k + 1;
                while tokens.get(m).is_some_and(|w| {
                    w.is_empty()
                        || matches!(w.as_str(), "the" | "a" | "an")
                        || POSSESSIVE_DETERMINERS.contains(&w.as_str())
                }) {
                    m += 1;
                }
                if matches!(
                    tokens.get(m).map(String::as_str),
                    Some("next") | Some("this")
                ) && tokens.get(m + 1).is_some_and(|w| weekday_word(w).is_some())
                {
                    k = m + 2;
                    continue;
                }
                if tokens.get(m).is_some_and(|w| weekday_word(w).is_some()) {
                    k = m + 1;
                    continue;
                }
                k = m;
            }
            _ => return false,
        }
    }
}

pub(super) fn segment_subject_before(segment: &[String], off: usize) -> bool {
    if matches!(
        segment.first().map(String::as_str),
        Some("and" | "or" | "but")
    ) {
        return false;
    }
    let mut head = false;
    let mut i = off;
    // Round 22 (R22-5): the subject run crosses a
    // prepositional phrase standing inside it —
    // determiner / noun head + preposition + its
    // own noun-like run counts as the subject, and
    // the phrase's words are consumed wholly by
    // it: "the team from Kent sang", "the choir on
    // Sunday played". The walk crosses a
    // preposition only after crossing the noun-like
    // run that is its object (a weekday object
    // included — "on Sunday" is the phrase's own
    // material, spawning nothing of its own), and
    // a preposition with no object run before it in
    // the walk still ends the run, as every
    // preposition did before this round. Subject
    // runs without PPs behave exactly as R21-B
    // pinned.
    let mut pp_object_crossed = false;
    while i > 0 {
        // Round 27 (R27-A, F3): a token that cleans
        // to nothing inside the run is nothing —
        // crossed without judging it, as in the
        // claim walk.
        if segment[i - 1].is_empty() {
            i -= 1;
            continue;
        }
        let w = segment[i - 1].as_str();
        if matches!(w, "the" | "a" | "an") || POSSESSIVE_DETERMINERS.contains(&w) {
            i -= 1;
        } else if noun_like_word(w) {
            if !is_finite_verb_form(w) {
                head = true;
            }
            pp_object_crossed = true;
            i -= 1;
        } else if weekday_word(w).is_some() && weekday_is_subject_pp_object(segment, i - 1) {
            pp_object_crossed = true;
            i -= 1;
        } else if is_subject_preposition(w) && pp_object_crossed {
            pp_object_crossed = false;
            i -= 1;
        } else if is_tolerated_adverb(w) || AUXILIARY_WORDS.contains(&w) {
            i -= 1;
        } else {
            break;
        }
    }
    head
}

/// Whether the weekday word at `i` in `tokens`
/// heads a weekday-modifier phrase (round 20,
/// R20-6; round 21, R21-7; round 23, R23-6): a
/// weekday word followed by a noun-like word is
/// that noun phrase's modifier ("Sunday
/// bulletins", "Sunday school"), and — R23-6 — a
/// weekday word followed by a VERB-FORM word and
/// then a noun-like word is the same phrase with
/// the verb-form word read as its adjective
/// ("Sunday left luggage", "Sunday sung hymns",
/// "Sunday taken seats"). Before this round the
/// second shape spawned the phantom October 18:
/// `noun_like_word` excludes the irregular verb
/// forms, so "left" / "sung" / "taken" could not
/// be the adjective the scans needed, and the
/// weekday fell through to the bare-Sunday
/// relative signal. One reading on every surface
/// that judges a weekday phrase: the group and
/// plain signal scans, the claim reach's
/// between-words test, and the lone-framed fence.
pub(super) fn weekday_modifier_at(tokens: &[String], i: usize) -> bool {
    // Round 39 (R39-G, F9/F11): the weekday test
    // stands first — the judgment is a conjunction
    // whose first conjunct is the weekday test, so
    // a token that names no weekday answers false
    // without the forward collection below. Before
    // this the collection ran first for EVERY token
    // the plain scans asked about, and on a core
    // carrying thousands of nothing-cleaning tokens
    // (the de-grouped core of a semicolon-group line
    // is mostly ";" markers) each token's collection
    // walked the whole nothing-run ahead of it —
    // quadratic time in the run's length, for a
    // conjunct that was false at every one of them.
    // The collection itself, and every judgment it
    // feeds, is unchanged.
    if weekday_word(&tokens[i]).is_none() {
        return false;
    }
    // Round 27 (R27-A, F3): the words the weekday
    // would modify are the next words that are
    // something — a token that cleans to nothing
    // between the weekday and its noun ("Sunday
    // \u{200b} bulletins") leaves the modifier
    // phrase exactly as it stands without it.
    let mut next: Vec<usize> = Vec::new();
    let mut j = i + 1;
    while j < tokens.len() && next.len() < 2 {
        if !tokens[j].is_empty() {
            next.push(j);
        }
        j += 1;
    }
    next.first().is_some_and(|&n| noun_like_word(&tokens[n]))
        || (next
            .first()
            .is_some_and(|&n| is_finite_verb_form(&tokens[n]))
            && next.get(1).is_some_and(|&n| noun_like_word(&tokens[n])))
}

/// Whether the weekday word at `i` in `tokens` is
/// consumed wholly as the object of a prepositional
/// phrase inside a subject run (round 22, R22-5):
/// a subject head stands before the preposition
/// ("the choir on Sunday …") and the clause's own
/// verb stands after the weekday. Such a weekday
/// spawns no relative signal, asserts no phantom
/// date, and is named nowhere — before this round
/// "the choir on Sunday was singing" minted an
/// October 18 signal inside the subject phrase,
/// because the modifier rule reaches only a
/// following noun-like word and an auxiliary or an
/// irregular verb is not one. A weekday phrase
/// with no subject head before it, or no clause
/// verb after it, keeps its standing readings.
pub(super) fn weekday_in_subject_pp(tokens: &[String], i: usize) -> bool {
    // Round 23 (R23-5): the weekday object may
    // wear its own determiner — "the choir on the
    // Sunday sang" — so the preposition may stand
    // one word further back, behind that
    // determiner. A determiner inside the PP does
    // not block consumption.
    let Some(prep) = subject_prep_before(tokens, i) else {
        return false;
    };
    if !subject_head_before(tokens, prep) {
        return false;
    }
    // The phrase's object may wear its own nouns
    // after the weekday ("on Sunday morning sang"),
    // and a subject may wear a STACK of PPs, so the
    // clause's verb may stand past a further PP
    // ("on Sunday from Kent sang"); the shared
    // forward walk crosses them all.
    verb_after_pp_objects(tokens, i + 1)
}

/// Whether a subject head stands before the
/// preposition at `pos` in `tokens` (round 23,
/// R23-5): the backward walk crosses determiners,
/// noun-like subject words, and — where a stack
/// stands before this preposition — an earlier
/// PP's object run and its preposition, exactly
/// as `segment_subject_before`'s forward-facing
/// twin crosses them walking back from a verb.
/// The head is a noun-like word that is not
/// itself a verb form.
pub(super) fn subject_head_before(tokens: &[String], pos: usize) -> bool {
    let mut head = false;
    let mut object_crossed = false;
    let mut j = pos;
    while j > 0 {
        if tokens[j - 1].is_empty() {
            j -= 1;
            continue;
        }
        let w = tokens[j - 1].as_str();
        if matches!(w, "the" | "a" | "an") || POSSESSIVE_DETERMINERS.contains(&w) {
            j -= 1;
        } else if noun_like_word(w) {
            if !is_finite_verb_form(w) {
                head = true;
            }
            object_crossed = true;
            j -= 1;
        } else if weekday_word(w).is_some() && weekday_is_subject_pp_object(tokens, j - 1) {
            object_crossed = true;
            j -= 1;
        } else if is_subject_preposition(w) && object_crossed {
            object_crossed = false;
            j -= 1;
        } else if is_tolerated_adverb(w) || AUXILIARY_WORDS.contains(&w) {
            j -= 1;
        } else {
            break;
        }
    }
    head
}

/// Whether the clause's verb stands at or after
/// `start` in `tokens`, crossing the object
/// material of a subject's PP STACK (round 23,
/// R23-5): from `start`, cross noun-like object
/// continuations; where a subject preposition
/// stands, cross it and its own object — a
/// determiner-wearing noun run, a weekday object
/// (its own determiner included), or a framed
/// date's word run, which is noun-like word by
/// word — and repeat, any number of PPs in any
/// preposition order. The walk ends at the first
/// word that is none of that material: the verb
/// stands there, or nowhere.
pub(super) fn verb_after_pp_objects(tokens: &[String], start: usize) -> bool {
    let mut k = start;
    loop {
        // Round 27 (R27-A, F3): tokens that clean
        // to nothing in the object material are
        // crossed as nothing.
        while tokens.get(k).is_some_and(|w| w.is_empty()) {
            k += 1;
        }
        while tokens
            .get(k)
            .is_some_and(|w| noun_like_word(w) && !is_finite_verb_form(w))
        {
            k += 1;
        }
        if !tokens.get(k).is_some_and(|w| is_subject_preposition(w)) {
            break;
        }
        // Cross this PP: the preposition, any
        // determiners its object wears, and the
        // object itself.
        let mut m = k + 1;
        while tokens.get(m).is_some_and(|w| {
            w.is_empty()
                || matches!(w.as_str(), "the" | "a" | "an")
                || POSSESSIVE_DETERMINERS.contains(&w.as_str())
        }) {
            m += 1;
        }
        // Round 24 (R24-7): the PP's object may be
        // a relative phrase ("on next Sunday") —
        // cross both of its words as one object.
        if matches!(
            tokens.get(m).map(String::as_str),
            Some("next") | Some("this")
        ) && tokens.get(m + 1).is_some_and(|w| weekday_word(w).is_some())
        {
            k = m + 2;
            continue;
        }
        if tokens.get(m).is_some_and(|w| weekday_word(w).is_some()) {
            k = m + 1;
            continue;
        }
        let object_start = m;
        while tokens
            .get(m)
            .is_some_and(|w| noun_like_word(w) && !is_finite_verb_form(w))
        {
            m += 1;
        }
        if m == object_start {
            return false;
        }
        k = m;
    }
    match tokens.get(k) {
        Some(w) => {
            (is_finite_verb_form(w) && verb_form_is_finite(tokens, k))
                || auxiliary_heads_clause(tokens, k)
        }
        None => false,
    }
}

/// Whether the framed date standing at
/// `start..start + used` in `tokens` is consumed
/// wholly as the object of a prepositional phrase
/// inside a subject run (round 23, R23-5, review
/// F9 + red team RT23-10): a subject preposition
/// stands directly before the date, a subject head
/// stands before the preposition, and the clause's
/// own verb stands after the date — "the team
/// after November 1 sang", "the choir on November
/// 15 sang", after ANY preposition, exactly as
/// after "from". Such a date is the subject
/// phrase's own material: it asserts nothing of
/// its own and competes with no other date; where
/// a composition names the dates beside its
/// assertion, it is named like the "from" twin's
/// date is. A date whose PP is not a subject PP —
/// no head before it, or no clause verb after it —
/// keeps its standing readings.
pub(super) fn date_in_subject_pp(tokens: &[String], start: usize, used: usize) -> bool {
    // Round 27 (R27-A, F3): the preposition is the
    // nearest word that is something before the
    // date — an empty token standing between it
    // and the date ("from \u{200b} November 1")
    // is nothing, as at every positional site.
    let prev = prev_nonempty(tokens, start, 1);
    let Some(&prep) = prev.first() else {
        return false;
    };
    if !is_subject_preposition(tokens[prep].as_str()) {
        return false;
    }
    if !subject_head_before(tokens, prep) {
        return false;
    }
    verb_after_pp_objects(tokens, start + used)
}

/// Whether the relative phrase standing at `start`
/// in `tokens` ("next Sunday" / "this Sunday") is
/// consumed wholly as the object of a prepositional
/// phrase inside a subject run (round 24, R24-7):
/// the relative-phrase twin of `date_in_subject_pp`
/// — a subject preposition stands directly before
/// the phrase (a determiner may stand between the
/// preposition and the phrase), a subject head
/// stands before the preposition, and the clause's
/// own verb stands after the phrase: "the choir
/// from next Sunday sang", "the choir before this
/// Sunday was singing". Such a phrase is the
/// subject phrase's own material: it asserts
/// nothing of its own and takes no primacy over the
/// clause's date; where a composition names the
/// dates beside its assertion, its resolution is
/// named exactly as the framed twin's PP object is
/// named. Before this round the phrase kept its
/// signal unconsumed — it was named beside a clean
/// target in the finite forms, took first-signal
/// primacy over the framed date after "on" /
/// "after", and demoted the target to alternatives
/// behind an auxiliary. A relative phrase standing
/// anywhere else — no subject head before its
/// preposition, or no clause verb after it — keeps
/// its standing readings.
pub(super) fn relative_in_subject_pp(tokens: &[String], start: usize) -> bool {
    if !matches!(
        tokens.get(start).map(String::as_str),
        Some("next") | Some("this")
    ) || !tokens
        .get(start + 1)
        .is_some_and(|w| weekday_word(w).is_some())
    {
        return false;
    }
    let Some(prep) = subject_prep_before(tokens, start) else {
        return false;
    };
    if !subject_head_before(tokens, prep) {
        return false;
    }
    verb_after_pp_objects(tokens, start + 2)
}
